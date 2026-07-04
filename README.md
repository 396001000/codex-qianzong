# codex-qianzong

Cross-platform desktop dashboard for local OpenAI Codex / ChatGPT Codex quota, token usage, value progress, and daily task status.

## Stack

- Tauri 2 desktop shell
- React 19 + TypeScript + Vite frontend
- Rust native data services
- SQLite access through `rusqlite`
- JSONL session parsing in Rust
- Tauri tray and global shortcut integration

## Development

```powershell
npm install
npm run dev
```

Frontend-only preview:

```powershell
npm run dev:frontend
```

Build frontend:

```powershell
npm run build
```

Rust checks:

```powershell
npm run rust:check
npm run rust:test
```

Desktop build:

```powershell
npm run tauri build
```

## Data Sources

- `codex app-server` JSON-RPC:
  - `account/read`
  - `account/rateLimits/read`
  - `account/usage/read`
- Local state DB:
  - Windows: `%USERPROFILE%\.codex\state_5.sqlite`
  - macOS: `~/.codex/state_5.sqlite`
  - fallback: `.codex/sqlite/state_5.sqlite`
- Session logs:
  - `.codex/sessions/**/rollout-*.jsonl`
  - paths are read from the local `threads.rollout_path` column
- Automations:
  - `.codex/automations/**/automation.toml`

## Desktop Controls

- Windows shortcut: `Ctrl+Alt+U`
- macOS shortcut: `Command+U`
- Tray menu:
  - Show / Hide
  - Toggle Always On Top
  - Quit

## Security Boundary

The frontend does not read arbitrary local files. Privileged access stays in Rust commands, which validate settings, detect Codex paths, read SQLite in read-only mode, and return typed data to the UI.

## Documentation

- `docs/architecture.md`
- `docs/ui-design.md`
- `docs/data-contract.md`
- `docs/windows-packaging.md`
- `docs/macos-packaging.md`
