use crate::{
    error::{AppError, AppResult},
    models::{ApiSpeedMode, AppSettings, CodexAccessMode, ReasoningEffort},
    paths,
};
use chrono::Local;
use serde_json::{Map, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};
use toml_edit::{value, DocumentMut, Item, Table};

const RELAY_PROVIDER_ID: &str = "qianzong_relay";
const OFFICIAL_MODEL: &str = "gpt-5.5";

pub fn sync_codex_config(settings: &AppSettings) -> AppResult<()> {
    let config_path = codex_config_path()?;
    let auth_path = codex_auth_path()?;
    let restore_path = restore_snapshot_path()?;
    sync_codex_config_for_paths(settings, &config_path, &auth_path, &restore_path)
}

fn codex_config_path() -> AppResult<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| AppError::Config("无法定位用户主目录".into()))?;
    Ok(home.join(".codex").join("config.toml"))
}

fn codex_auth_path() -> AppResult<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| AppError::Config("无法定位用户主目录".into()))?;
    Ok(home.join(".codex").join("auth.json"))
}

fn restore_snapshot_path() -> AppResult<PathBuf> {
    let app_dir = paths::app_log_dir()?
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    Ok(app_dir.join("codex-config-restore.toml"))
}

fn sync_codex_config_for_paths(
    settings: &AppSettings,
    config_path: &Path,
    auth_path: &Path,
    restore_path: &Path,
) -> AppResult<()> {
    let original = read_optional_text(config_path)?;

    let next_text = match settings.access_mode {
        CodexAccessMode::Relay => {
            ensure_restore_snapshot(config_path, restore_path, &original)?;
            let mut doc = parse_config(&original)?;
            apply_relay_config(&mut doc, settings)?;
            doc.to_string()
        }
        CodexAccessMode::Official => {
            let mut doc = parse_config(&original)?;
            apply_official_config(&mut doc);
            doc.to_string()
        }
    };

    if next_text != original {
        backup_existing_file(config_path, "config.toml")?;
        write_config(config_path, &next_text)?;
    }

    sync_codex_auth_for_path(settings, auth_path)?;

    Ok(())
}

fn read_optional_text(path: &Path) -> AppResult<String> {
    if path.exists() {
        Ok(fs::read_to_string(path)?)
    } else {
        Ok(String::new())
    }
}

fn parse_config(text: &str) -> AppResult<DocumentMut> {
    if text.trim().is_empty() {
        Ok(DocumentMut::new())
    } else {
        text.parse::<DocumentMut>()
            .map_err(|err| AppError::Config(format!("Codex 配置解析失败: {err}")))
    }
}

fn ensure_restore_snapshot(
    config_path: &Path,
    restore_path: &Path,
    current_text: &str,
) -> AppResult<()> {
    if restore_path.exists() || !config_path.exists() || is_qianzong_managed(current_text) {
        return Ok(());
    }
    if let Some(parent) = restore_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(restore_path, current_text)?;
    Ok(())
}

fn sync_codex_auth_for_path(settings: &AppSettings, auth_path: &Path) -> AppResult<()> {
    let original = read_optional_text(auth_path)?;
    let mut auth = parse_auth_json(&original)?;

    match settings.access_mode {
        CodexAccessMode::Relay => apply_relay_auth(&mut auth, settings)?,
        CodexAccessMode::Official => apply_official_auth(&mut auth),
    }

    let next_text = serde_json::to_string_pretty(&Value::Object(auth))?;
    if next_text != original.trim() {
        backup_existing_file(auth_path, "auth.json")?;
        write_config(auth_path, &format!("{next_text}\n"))?;
    }

    Ok(())
}

fn parse_auth_json(text: &str) -> AppResult<Map<String, Value>> {
    if text.trim().is_empty() {
        return Ok(Map::new());
    }
    let value: Value = serde_json::from_str(text)
        .map_err(|err| AppError::Config(format!("Codex 认证文件解析失败: {err}")))?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| AppError::Config("Codex 认证文件必须是 JSON 对象".into()))
}

fn apply_relay_auth(auth: &mut Map<String, Value>, settings: &AppSettings) -> AppResult<()> {
    let existing_key = auth
        .get("OPENAI_API_KEY")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let next_key = settings
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or(existing_key)
        .ok_or_else(|| AppError::Config("API 中转模式需要填写 API Key".into()))?;

    auth.insert("auth_mode".to_string(), Value::String("apikey".to_string()));
    auth.insert("OPENAI_API_KEY".to_string(), Value::String(next_key));
    Ok(())
}

fn apply_official_auth(auth: &mut Map<String, Value>) {
    auth.insert(
        "auth_mode".to_string(),
        Value::String("chatgpt".to_string()),
    );
    auth.insert("OPENAI_API_KEY".to_string(), Value::Null);
}

fn backup_existing_file(path: &Path, base_name: &str) -> AppResult<()> {
    if !path.exists() {
        return Ok(());
    }
    let timestamp = Local::now().format("%Y%m%d%H%M%S%3f");
    let backup_name = format!("{base_name}.qianzong-backup-{timestamp}");
    let backup_path = path.with_file_name(backup_name);
    fs::copy(path, backup_path)?;
    Ok(())
}

fn write_config(config_path: &Path, text: &str) -> AppResult<()> {
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(config_path, text)?;
    Ok(())
}

fn apply_relay_config(doc: &mut DocumentMut, settings: &AppSettings) -> AppResult<()> {
    let endpoint = settings
        .api_endpoint
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Config("API 中转模式需要填写 API 地址".into()))?;
    let model = settings.api_model.trim();

    doc["model"] = value(if model.is_empty() { "gpt-5" } else { model });
    doc["model_provider"] = value(RELAY_PROVIDER_ID);
    doc["preferred_auth_method"] = value("apikey");
    doc["model_reasoning_effort"] = value(reasoning_effort_value(&settings.reasoning_effort));

    match settings.speed_mode {
        ApiSpeedMode::Fast => {
            doc["service_tier"] = value("priority");
        }
        ApiSpeedMode::Stable | ApiSpeedMode::Balanced => {
            doc.as_table_mut().remove("service_tier");
        }
    }

    let relay = ensure_relay_provider_table(doc)?;
    relay.insert("name", value(RELAY_PROVIDER_ID));
    relay.insert("base_url", value(endpoint));
    relay.insert("wire_api", value("responses"));
    Ok(())
}

fn apply_official_config(doc: &mut DocumentMut) {
    let root = doc.as_table_mut();
    root.remove("model_provider");
    root.remove("openai_base_url");
    root.remove("service_tier");
    root.insert("model", value(OFFICIAL_MODEL));
    root.insert("model_reasoning_effort", value("medium"));
    root.insert("preferred_auth_method", value("chatgpt"));

    if let Some(Item::Table(providers)) = root.get_mut("model_providers") {
        providers.remove(RELAY_PROVIDER_ID);
        if providers.is_empty() {
            root.remove("model_providers");
        }
    }
}

fn ensure_relay_provider_table(doc: &mut DocumentMut) -> AppResult<&mut Table> {
    let root = doc.as_table_mut();
    if !matches!(root.get("model_providers"), Some(Item::Table(_))) {
        root.insert("model_providers", Item::Table(Table::new()));
    }
    let providers = root
        .get_mut("model_providers")
        .and_then(Item::as_table_mut)
        .ok_or_else(|| AppError::Config("无法写入 Codex provider 配置".into()))?;
    if !matches!(providers.get(RELAY_PROVIDER_ID), Some(Item::Table(_))) {
        providers.insert(RELAY_PROVIDER_ID, Item::Table(Table::new()));
    }
    providers
        .get_mut(RELAY_PROVIDER_ID)
        .and_then(Item::as_table_mut)
        .ok_or_else(|| AppError::Config("无法写入 API 中转 provider 配置".into()))
}

fn reasoning_effort_value(effort: &ReasoningEffort) -> &'static str {
    match effort {
        ReasoningEffort::Minimal | ReasoningEffort::Low => "low",
        ReasoningEffort::Medium => "medium",
        ReasoningEffort::High => "high",
        ReasoningEffort::Extreme => "xhigh",
    }
}

fn is_qianzong_managed(text: &str) -> bool {
    if let Ok(doc) = text.parse::<DocumentMut>() {
        let root = doc.as_table();
        if root
            .get("model_provider")
            .and_then(Item::as_value)
            .and_then(|item| item.as_str())
            == Some(RELAY_PROVIDER_ID)
        {
            return true;
        }
        if root
            .get("model_providers")
            .and_then(Item::as_table)
            .and_then(|providers| providers.get(RELAY_PROVIDER_ID))
            .is_some()
        {
            return true;
        }
    }
    text.contains("model_provider = \"qianzong_relay\"")
        || text.contains("[model_providers.qianzong_relay]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_sync_writes_custom_provider_and_restore_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("config.toml");
        let auth_path = temp.path().join("auth.json");
        let restore_path = temp.path().join("codex-config-restore.toml");
        fs::write(
            &config_path,
            r#"model = "gpt-5.5"
preferred_auth_method = "chatgpt"
"#,
        )
        .unwrap();

        let mut settings = AppSettings::default();
        settings.access_mode = CodexAccessMode::Relay;
        settings.api_endpoint = Some("https://api.example.com/v1".into());
        settings.api_key = Some("sk-test".into());
        settings.api_model = "gpt-5.4".into();
        settings.reasoning_effort = ReasoningEffort::Extreme;
        settings.speed_mode = ApiSpeedMode::Fast;

        sync_codex_config_for_paths(&settings, &config_path, &auth_path, &restore_path).unwrap();

        let text = fs::read_to_string(&config_path).unwrap();
        assert!(text.contains(r#"model = "gpt-5.4""#));
        assert!(text.contains(r#"model_provider = "qianzong_relay""#));
        assert!(text.contains(r#"preferred_auth_method = "apikey""#));
        assert!(text.contains(r#"model_reasoning_effort = "xhigh""#));
        assert!(text.contains(r#"service_tier = "priority""#));
        assert!(text.contains(r#"[model_providers.qianzong_relay]"#));
        assert!(text.contains(r#"base_url = "https://api.example.com/v1""#));
        assert!(text.contains(r#"wire_api = "responses""#));

        let restore = fs::read_to_string(&restore_path).unwrap();
        assert!(restore.contains(r#"preferred_auth_method = "chatgpt""#));
        let auth = fs::read_to_string(&auth_path).unwrap();
        assert!(auth.contains(r#""auth_mode": "apikey""#));
        assert!(auth.contains(r#""OPENAI_API_KEY": "sk-test""#));
        assert!(temp.path().read_dir().unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains("qianzong-backup")));
    }

    #[test]
    fn official_sync_restores_official_provider_shape() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("config.toml");
        let auth_path = temp.path().join("auth.json");
        let restore_path = temp.path().join("codex-config-restore.toml");
        fs::write(
            &config_path,
            r#"model = "relay-model"
model_provider = "qianzong_relay"
preferred_auth_method = "apikey"
model_reasoning_effort = "xhigh"
service_tier = "priority"

[model_providers.qianzong_relay]
name = "qianzong_relay"
base_url = "https://api.example.com/v1"
wire_api = "responses"

[mcp_servers.current]
command = "node"

[projects."/Users/mac/project-a"]
trust_level = "trusted"
"#,
        )
        .unwrap();
        fs::write(
            &restore_path,
            r#"model = "gpt-5.4"
preferred_auth_method = "chatgpt"
service_tier = "priority"

[mcp_servers.stale_restore]
command = "stale"
"#,
        )
        .unwrap();
        fs::write(
            &auth_path,
            r#"{
  "auth_mode": "apikey",
  "OPENAI_API_KEY": "sk-test"
}
"#,
        )
        .unwrap();

        let settings = AppSettings::default();
        sync_codex_config_for_paths(&settings, &config_path, &auth_path, &restore_path).unwrap();

        let text = fs::read_to_string(&config_path).unwrap();
        assert!(text.contains(r#"model = "gpt-5.5""#));
        assert!(text.contains(r#"preferred_auth_method = "chatgpt""#));
        assert!(text.contains(r#"model_reasoning_effort = "medium""#));
        assert!(text.contains(r#"[mcp_servers.current]"#));
        assert!(text.contains(r#"[projects."/Users/mac/project-a"]"#));
        assert!(text.contains(r#"trust_level = "trusted""#));
        assert!(!text.contains("stale_restore"));
        assert!(!text.contains("service_tier"));
        assert!(!text.contains("qianzong_relay"));
        assert!(!text.contains("model_provider"));
        let auth = fs::read_to_string(&auth_path).unwrap();
        assert!(auth.contains(r#""auth_mode": "chatgpt""#));
        assert!(auth.contains(r#""OPENAI_API_KEY": null"#));
    }

    #[test]
    fn official_sync_repairs_apikey_auth_residue_without_restore_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("config.toml");
        let auth_path = temp.path().join("auth.json");
        let restore_path = temp.path().join("codex-config-restore.toml");
        fs::write(
            &config_path,
            r#"model = "gpt-5.5"
preferred_auth_method = "apikey"
service_tier = "priority"
"#,
        )
        .unwrap();
        fs::write(
            &auth_path,
            r#"{
  "auth_mode": "chatgpt",
  "OPENAI_API_KEY": null,
  "tokens": {}
}
"#,
        )
        .unwrap();

        let settings = AppSettings::default();
        sync_codex_config_for_paths(&settings, &config_path, &auth_path, &restore_path).unwrap();

        let text = fs::read_to_string(&config_path).unwrap();
        assert!(text.contains(r#"preferred_auth_method = "chatgpt""#));
        assert!(!text.contains(r#"preferred_auth_method = "apikey""#));
        assert!(!text.contains("service_tier"));
        let auth = fs::read_to_string(&auth_path).unwrap();
        assert!(auth.contains(r#""auth_mode": "chatgpt""#));
    }

    #[test]
    fn relay_sync_requires_endpoint() {
        let temp = tempfile::tempdir().unwrap();
        let mut settings = AppSettings::default();
        settings.access_mode = CodexAccessMode::Relay;
        settings.api_endpoint = None;

        let err = sync_codex_config_for_paths(
            &settings,
            &temp.path().join("config.toml"),
            &temp.path().join("auth.json"),
            &temp.path().join("restore.toml"),
        )
        .unwrap_err();

        assert!(err.to_string().contains("API 中转模式需要填写 API 地址"));
    }

    #[test]
    fn relay_sync_requires_api_key_when_auth_has_no_existing_key() {
        let temp = tempfile::tempdir().unwrap();
        let mut settings = AppSettings::default();
        settings.access_mode = CodexAccessMode::Relay;
        settings.api_endpoint = Some("https://api.example.com/v1".into());

        let err = sync_codex_config_for_paths(
            &settings,
            &temp.path().join("config.toml"),
            &temp.path().join("auth.json"),
            &temp.path().join("restore.toml"),
        )
        .unwrap_err();

        assert!(err.to_string().contains("API 中转模式需要填写 API Key"));
    }

    #[test]
    fn relay_sync_preserves_existing_api_key_when_input_is_empty() {
        let temp = tempfile::tempdir().unwrap();
        let auth_path = temp.path().join("auth.json");
        fs::write(
            &auth_path,
            r#"{
  "auth_mode": "apikey",
  "OPENAI_API_KEY": "sk-existing"
}
"#,
        )
        .unwrap();
        let mut settings = AppSettings::default();
        settings.access_mode = CodexAccessMode::Relay;
        settings.api_endpoint = Some("https://api.example.com/v1".into());

        sync_codex_config_for_paths(
            &settings,
            &temp.path().join("config.toml"),
            &auth_path,
            &temp.path().join("restore.toml"),
        )
        .unwrap();

        let auth = fs::read_to_string(&auth_path).unwrap();
        assert!(auth.contains(r#""OPENAI_API_KEY": "sk-existing""#));
    }

    #[test]
    fn qianzong_detection_handles_toml_quoting_variants() {
        assert!(is_qianzong_managed("model_provider = 'qianzong_relay'"));
        assert!(is_qianzong_managed(
            "[model_providers.qianzong_relay]\nbase_url = 'https://api.example.com/v1'"
        ));
    }
}
