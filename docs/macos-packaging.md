# macOS Packaging

## Read When

- Before changing macOS bundle, signing, notarization, shortcut behavior, or Codex path detection.

## Owner

- Desktop / Release

## Update Trigger

- Bundle metadata, signing, notarization, app permissions, or macOS runtime behavior changes.

## Validation

- `npm run tauri build` succeeds on macOS and the generated `.app` launches.

## Requirements

- macOS 13+
- Xcode Command Line Tools
- Node.js 24+
- Rust 1.92+

## Commands

```sh
npm install
npm run build
npm run rust:check
npm run tauri build
```

## Runtime Behavior

- Global shortcut: `Command+U`
- Codex CLI detection checks:
  - `/Applications/Codex.app/Contents/Resources/codex`
  - `/opt/homebrew/bin/codex`
  - `/usr/local/bin/codex`
  - `/usr/bin/codex`
  - `PATH`
- `.codex` data detection defaults to `~/.codex`.

## Release Notes

- `.icns` is generated from `Resources/codexU-icon.png`.
- Developer ID signing and notarization are intentionally not hardcoded.
- Add signing and notarization through release environment variables or a dedicated release workflow.
