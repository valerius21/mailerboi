# Learnings — mailerboi

## Tech Stack (Confirmed)
- IMAP: `async-imap` v0.11 + `async-native-tls` (NOT tokio-rustls)
- Config: `toon-format` v0.4.4 (spike needed to validate custom struct serde)
- Credentials: separate `credentials.toml` (TOML format)
- CLI: `clap` v4 derive
- Logging: `tracing` + `tracing-subscriber`
- Error: `thiserror` (lib) + `anyhow` (bin)
- Output: `comfy-table` + `serde_json` + `toon-format`
- Workspace: `crates/mailerboi-core` (lib) + `crates/mailerboi` (bin)
- Testing: TDD, GreenMail Docker for integration

## Critical Guardrails
- NO `unwrap()` in library code
- NO `println!()` in library code — tracing only
- NO backend trait abstraction — direct IMAP only
- NO async trait objects
- Use UIDs (not sequence numbers) for all message ops
- Exit codes: 0=success, 1=error, 2=config error

## Spike Risks
- toon-format custom struct serde UNVALIDATED — Task 2 must confirm
- async-imap + TLS combination UNVALIDATED — Task 3 must confirm
