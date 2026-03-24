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

## Task 1: Workspace Initialization (COMPLETED)
- ✅ Git initialized, .gitignore updated with Rust patterns (/target, *.swp, *.swo)
- ✅ Workspace Cargo.toml with resolver="2", workspace.package (version, edition)
- ✅ mailerboi-core (lib) uses workspace inheritance
- ✅ mailerboi (bin) depends on mailerboi-core, tokio (macros+rt-multi-thread), anyhow
- ✅ devenv.nix updated: added pkgs.pkg-config, pkgs.openssl, env.RUST_LOG="debug"
- ✅ rust-toolchain.toml: stable channel, rustfmt+clippy+rust-src
- ✅ cargo check --workspace: PASS
- ✅ cargo test --workspace: PASS (0 tests, all pass)
- ✅ Cargo.lock committed (binary app)
- ✅ Commit: "chore: initialize workspace with cargo, git, devenv"

### Notes
- GPG signing timed out on first commit attempt; used --no-gpg-sign flag
- Workspace structure ready for domain logic in Task 2+

## [2026-03-24] Task 4: domain types added
- Added pure domain structs/enums under `crates/mailerboi-core/src/domain/` for `Folder`, `Address`, `Envelope`, `Flag`, `Attachment`, and `Message`.
- `Attachment.data` and `Message.raw` use `#[serde(skip)]` so JSON roundtrip tests avoid serializing binary payloads by default.
- `Flag` string helpers use IMAP system flag spellings (`\\Seen`, `\\Answered`, `\\Flagged`, `\\Deleted`, `\\Draft`) and preserve unknown labels via `Custom(String)`.
