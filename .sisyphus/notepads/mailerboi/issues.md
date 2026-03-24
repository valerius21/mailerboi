# Issues — mailerboi

## Spike Risks (Pending Validation)
1. toon-format v0.4.4 custom struct serde — needs Task 2 validation
   - If fails: fallback to serde_json::Value intermediary OR serde_toon crate OR TOML
2. async-imap + async-native-tls + GreenMail — needs Task 3 validation
   - If fails: try tokio-rustls, or fall back to sync imap crate with spawn_blocking

## Known IMAP Quirks
- MOVE (RFC 6851) not universally supported — need COPY+DELETE fallback
- Mailbox names may use modified UTF-7 encoding
- GreenMail uses self-signed certs → danger_accept_invalid_certs(true) for tests
- GreenMail ports: 3143 (plain/STARTTLS), 3993 (TLS)

## Rust Specifics
- `move` is a Rust keyword — use `move_cmd.rs` for the move command file
- async-imap returns `async_imap::error::Result` not std Result

## [2026-03-24] Spike Task 2: toon-format result
PASS: `toon-format` v0.4.4 roundtrips custom Rust structs with `Serialize`/`Deserialize` (not only `serde_json::Value`).

Validated in `mailerboi-core` tests:
- Simple struct (`TestConfig`) encode/decode equality.
- Nested map struct (`TestRoot` + `HashMap<String, TestAccount>`) encode/decode equality.
- Special character payload (comma, colon, Unicode `h\u{00E9}llo`) preserved.
- Optional fields (`Option<String>`) for both `Some` and `None` preserved.

Actionable decision for Task 6: use `toon_format::encode_default()` / `toon_format::decode_default()` directly for config parsing.
