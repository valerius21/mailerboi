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

## [2026-03-24] Domain model
- Keep mail domain types in `mailerboi-core::domain` as pure serde-friendly data structures with `Display` impls; no IMAP parsing or backend behavior lives in these types.
- Represent mailbox flags with a small typed enum plus `Custom(String)` to preserve unknown server labels without losing roundtrip fidelity.

## [2026-03-24] Task 6 config module
- Keep `AppConfig` public shape as `accounts: HashMap<String, AccountConfig>` and implement a custom `Deserialize` bridge so TOON dotted keys (`accounts.<name>`) are accepted without changing caller-facing types.
- Keep credentials as plaintext `credentials.toml` with `#[serde(flatten)]` map to support account-name keyed passwords without fixed schema.

## [2026-03-24] Task 7 IMAP connectivity
- Model live IMAP connections with an enum wrapper (`ImapSession::Tls` | `ImapSession::Plain`) instead of trait abstraction/pooling so each operation owns exactly one session lifecycle.
- Expose a dedicated `disconnect(session)` helper that delegates to protocol `logout()` for explicit teardown semantics in callers/tests.
- Keep implicit TLS (`tls = true`, `starttls = false`) fully supported now; defer STARTTLS handshake implementation and explicitly warn when falling back to plain path.
