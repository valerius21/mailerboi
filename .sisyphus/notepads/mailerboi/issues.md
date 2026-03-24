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

## [2026-03-24] Spike Task 3: async-imap + GreenMail result
PASS: `async-imap` + `async-native-tls` works against GreenMail for TLS (3993) and plain IMAP (3143), including login, `SELECT INBOX`, and `LOGOUT`.

Working connection pattern used:
- Dependencies:
  - `async-imap = { version = "0.11.2", default-features = false, features = ["runtime-tokio"] }`
  - `async-native-tls = { version = "0.5", default-features = false, features = ["runtime-tokio", "vendored"] }`
  - `tokio = { version = "1", features = ["macros", "rt-multi-thread", "net"] }`
- TLS flow:
  1. `let tcp = tokio::net::TcpStream::connect("127.0.0.1:3993").await?;`
  2. `let tls = async_native_tls::TlsConnector::new().danger_accept_invalid_certs(true).danger_accept_invalid_hostnames(true);`
  3. `let stream = tls.connect("localhost", tcp).await?;`
  4. `let mut session = async_imap::Client::new(stream).login(user, pass).await.map_err(|(e, _)| e)?;`
  5. `session.select("INBOX").await?; session.logout().await?;`
- Plain flow:
  1. `let tcp = tokio::net::TcpStream::connect("127.0.0.1:3143").await?;`
  2. `let mut session = async_imap::Client::new(tcp).login(user, pass).await.map_err(|(e, _)| e)?;`
  3. `session.select("INBOX").await?; session.logout().await?;`

Gotchas discovered:
- `async-imap` must disable default features (`default-features = false`) when using `runtime-tokio`; otherwise both async-std and tokio runtimes are enabled and compile fails.
- `async-native-tls` also needs `default-features = false` + `runtime-tokio` to accept `tokio::net::TcpStream` directly.
- On this environment OpenSSL system headers are missing; `async-native-tls` needs `vendored` feature to compile successfully.
