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

## [2026-03-24] Task 6: config parsing behavior
- TOON input using dotted keys (`accounts.personal:`) does not deserialize into `AppConfig { accounts: ... }` directly; custom deserialization is needed to map `accounts.<name>` keys into the `accounts` map.
- `load_credentials` now emits a Unix warning when credentials mode is world-readable (`mode & 0o004 != 0`) without failing the load.
- Account resolution order is stable: explicit name > first `default = true` account > first configured account.

## [2026-03-24] Task 7: IMAP connection manager
- Added `mailerboi_core::imap` with `ImapSession` enum (`Tls`/`Plain`) and top-level `connect`, `disconnect`, and `doctor` async functions.
- `doctor` now evaluates five checkpoints (`dns_ok`, `tcp_ok`, `tls_ok`, `auth_ok`, `inbox_ok`) and preserves first failure detail in `error` while short-circuiting on hard connectivity failures.
- STARTTLS is intentionally not implemented yet; when `starttls = true`, current behavior logs a warning and uses plain IMAP path.

## [2026-03-24] Task 8: output formatting
- Added `mailerboi_core::output` with a shared `OutputFormat` enum and formatter helpers for folders, envelopes, single messages, account checks, account lists, and doctor reports.
- Table rendering uses `comfy-table` with condensed UTF-8 borders; JSON uses `serde_json`; TOON serialization uses `toon_format::encode_default()` for list/report payloads.
- `DoctorReport` now derives serde traits so doctor results can be emitted in machine-readable formats without adapter structs.

## [2026-03-24] Task 9: CLI skeleton
- `clap` derive on `Commands` automatically exposes kebab-case subcommands, so `ListAccounts` becomes `list-accounts` without extra rename attributes.
- `mailerboi_core::output::OutputFormat` can stay in the core crate unchanged; the CLI uses `value_parser = clap::value_parser!(OutputFormat)` to parse its existing `FromStr` implementation.
- Keeping parse tests in `crates/mailerboi/src/main.rs` works for a binary-only crate and verifies command wiring without introducing a library target.

## [2026-03-24] Task 10 command wiring
- Added thin async command modules under `crates/mailerboi/src/cmd/` so CLI wiring stays separate from clap parsing and delegates config loading / output rendering to `mailerboi-core`.
- `doctor` reuses `resolve_account()` plus `credentials.toml` lookup, then exits with status 1 when any doctor checkpoint fails while still printing the formatted report first.
- A minimal TOON fixture under `crates/mailerboi/tests/fixtures/test-config.toon` is enough to verify `list-accounts` end-to-end without touching real mail servers.

## [2026-03-24] Task 11 folders command
- Added `ImapSession::list_folders()` in `crates/mailerboi-core/src/imap/mod.rs` and collected the async-imap `LIST` stream with `futures::StreamExt` to convert server names directly into domain `Folder` values.
- New CLI command module `crates/mailerboi/src/cmd/folders.rs` follows the same config/account/credentials flow as `doctor`, so folder listing stays a thin wrapper around core IMAP and shared output formatting.
- End-to-end `folders` verification depends on `credentials_path()` resolving a real file; for local fixture runs, placing `test-credentials.toml` at `~/.config/mailerboi/credentials.toml` satisfies the existing lookup path without changing config code.

## [2026-03-24] Task 12 list command
- Added `ImapSession::list_envelopes()` in `crates/mailerboi-core/src/imap/mod.rs`; because `async-imap::Session::fetch()` returns different opaque stream types for TLS vs plain sessions, each match arm must collect its stream before the results can be unified as a `Vec`.
- Pagination uses IMAP sequence numbers only to choose the fetch window (`start:end`) and then returns domain `Envelope` values with UIDs preserved for downstream message operations.
- New CLI wrapper `crates/mailerboi/src/cmd/list.rs` mirrors the existing folder/doctor command flow: load config, resolve account + password, connect, call core IMAP, print shared formatted output, then best-effort logout.
