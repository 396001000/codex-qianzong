# Development Log

## Read When

- When recovering recent implementation decisions or preparing a follow-up change.

## Owner

- Project Assistant

## Update Trigger

- Reusable product behavior, validation evidence, or implementation constraints change.

## Validation

- Entries include concrete changed behavior and avoid transient command logs.

## 2026-07-03

- Settings drawer no longer shows the old explanatory subtitle `接入方式、路径、刷新频率、主题与任务看板行为`.
- Official native mode now keeps relay-only controls hidden. It uses Codex default official state and does not require an API endpoint.
- API relay mode shows endpoint/model/reasoning/speed presets. Endpoint normalization accepts a base URL and stores exactly one trailing `/v1`, avoiding duplicate `/v1/v1`.
- Refresh now always parses local session JSONL detailed usage for token-value cards, while official usage remains responsible for account-level trend data.
- Member value progress now uses the current billing cycle anchored to the configured membership open date. Example: `2026-04-10` with local date `2026-07-03` starts at `2026-06-10`.
- Settings drawer save now waits for persistence, closes on success, keeps inline errors on failure, and no longer shows the log button.
- Dashboard data source now follows access mode: official native mode uses official app-server account data and `account/usage/read` daily buckets; API relay mode uses local SQLite/JSONL statistics and hides official quota windows.
- 环境诊断卡片改为固定图标列 + 可截断文本列，长本地路径不会再压住相邻卡片。
- 保存设置现在会自动同步 Codex `config.toml`：官方原生模式恢复官方 ChatGPT 配置形态，API 中转模式写入 `qianzong_relay` provider、模型、推理强度和速度服务层；同步前会创建恢复快照和时间戳备份。
- 无边框窗口标题栏新增最小化和关闭按钮；默认开发窗口宽度从 `920` 增加到 `930`。

## 2026-07-04

- 今日任务看板的可见入口替换为独立 Skills 技能看板；旧任务看板代码和 `UsageSnapshot.task_board` 聚合暂时保留，降低对统计功能的影响。
- Skills 看板前端集中在 `src/features/skills-board/`，后端集中在 `src-tauri/src/skills_board/`；前端只传 `skillId`，后端重新扫描解析路径。
- 技能删除采用安全归档到 `skills-trash`，禁用采用移动到 `skills-disabled`；系统技能、插件技能和 `yonghu-preferences` 强制只读。
- Skills 看板新增 `全部` / `已启动` / `已禁用` 筛选，已禁用技能来自 `.codex/skills-disabled` 并显示在对应列表。
- Skills 看板新增 Google 翻译切换按钮，翻译只影响当前显示的技能描述，`取消翻译` 会恢复原文，不写回 `SKILL.md`。
- Skills 看板新增启用已禁用技能能力：后端 `enable_skill` 将 `.codex/skills-disabled` 条目移回 `.codex/skills`，前端成功启用后切回 `已启动` 列表。
- 发布版本号统一更新到 `1.2.0`，并生成 Windows MSI/NSIS 安装包。子项目 `.gitignore` 排除了 `.devlogs` 和 `.dev-logs`，避免本地验证截图和日志进入独立发布仓库。
