# Decisions — mailerboi

## Architecture
- Single backend (IMAP only): NO trait abstraction needed
- Workspace layout: `crates/mailerboi-core/` + `crates/mailerboi/`
- All message operations use UIDs (stable across sessions)
- TLS: async-native-tls (proven with async-imap), not tokio-rustls

## Config
- Main config: TOON format at ~/.config/mailerboi/config.toon
- Credentials: credentials.toml at ~/.config/mailerboi/credentials.toml
- Default account: first account OR one marked `default = true`

## CLI
- 12 subcommands: list-accounts, doctor, check, folders, list, read, search, move, delete, flag, download, draft
- Global flags: --config, --account, --output, --insecure
- Output: table (default), json, toon

## Scope
- IN: IMAP read/manage/search, draft creation (IMAP APPEND)
- OUT: SMTP/send, OAuth2, TUI, shell completions, caching
