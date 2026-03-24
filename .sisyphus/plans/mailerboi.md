# Mailerboi — Rust CLI Multi-Account IMAP Email Client

## TL;DR

> **Quick Summary**: Build a Rust CLI tool for managing multiple IMAP email accounts — list, read, search, move, delete, flag, download attachments, create drafts — using TOON format config, credentials.toml for secrets, and async-imap with TDD.
> 
> **Deliverables**:
> - Cargo workspace: `mailerboi-core` (library) + `mailerboi` (binary)
> - TOON config file support (`~/.config/mailerboi/config.toon`)
> - credentials.toml for secrets (`~/.config/mailerboi/credentials.toml`)
> - 12 CLI subcommands: `list-accounts`, `doctor`, `check`, `folders`, `list`, `read`, `search`, `move`, `delete`, `flag`, `download`, `draft`
> - Table + TOON + JSON output modes
> - TDD with cargo test + GreenMail Docker integration tests
> 
> **Estimated Effort**: Large
> **Parallel Execution**: YES — 5 waves
> **Critical Path**: Task 1 → Task 2 → Task 3/4 → Task 5 → Tasks 6-8 → Tasks 9-15 → Tasks 16-19 → Task 20 → F1-F4

---

## Context

### Original Request
Build a Rust CLI tool for managing multiple email accounts via IMAP. Config in TOON format with credentials.toml. Support listing mailboxes, reading emails, searching, moving, deleting, flagging, downloading attachments, creating drafts. Rein CLI with subcommands, no TUI, no sending. Output as table, TOON, or JSON. Running on NixOS with devenv.

### Interview Summary
**Key Discussions**:
- **Config format**: TOON (`toon-format` crate v0.4.4) — user explicitly chose over YAML/TOML
- **Credentials**: Separate `credentials.toml` file — user chose over .env and inline config
- **Providers**: Generic IMAP only, no OAuth2 for now (app passwords)
- **Sending**: Explicitly excluded — no SMTP. But draft creation via IMAP APPEND is IN scope
- **Interface**: Pure CLI with subcommands, no TUI
- **Output**: Table + TOON + JSON (--output flag)
- **Logging**: `tracing` (user chose over env_logger)
- **Structure**: Workspace with lib + bin (user chose over single crate)
- **Testing**: TDD — tests first (user chose over tests-after and no-tests)
- **Additional features from imap-smtp-email review**: Search filters, mark read/unread, attachments, list accounts, connection doctor, delete, quick check — ALL confirmed

**Research Findings**:
- `async-imap` v0.11 is stable, async, Tokio-compatible. All examples use `async-native-tls`.
- `mail-parser` v0.11 is best-in-class (fuzzed, MIRI-tested, 41 charsets, serde support)
- `toon-format` v0.4.4 claims generic serde support for custom structs — NEEDS SPIKE VALIDATION
- Himalaya architecture: backend trait abstraction is overkill for single-backend (IMAP-only)
- Himalaya domain types (Folder, Envelope, Flag, Message) are excellent patterns to follow
- IMAP MOVE (RFC 6851) not universally supported — needs COPY+DELETE fallback
- `async-imap` + `tokio-rustls` combination is untested — `async-native-tls` is proven

### Metis Review
**Identified Gaps** (addressed):
- **toon-format spike risk**: v0.4.4 custom struct serde unvalidated → Added spike task (Task 2)
- **async-imap + TLS spike risk**: tokio-rustls untested with async-imap → Added spike task (Task 3), defaulted to async-native-tls
- **Git not initialized**: No .git in project → Added to Task 1
- **IMAP MOVE fallback missing**: Not all servers support RFC 6851 → Added fallback logic to move task
- **UID vs sequence numbers**: Not discussed → Defaulted to UIDs (stable across sessions)
- **Exit code scheme**: Not discussed → Defaulted to 0=success, 1=error, 2=config error
- **STARTTLS support**: Not discussed → Support both 993 (implicit TLS) and 143 (STARTTLS)
- **Self-signed certs**: Not discussed → Added --insecure flag
- **Default account**: Not discussed → First account or one marked `default = true`
- **Architecture over-engineering**: Himalaya patterns too complex → Simplified to direct IMAP, no traits

---

## Work Objectives

### Core Objective
Build a production-quality Rust CLI for managing multiple IMAP email accounts with TOON config, comprehensive mailbox operations, and scriptable output.

### Concrete Deliverables
- `mailerboi` binary installable via `cargo install`
- `mailerboi-core` library with all domain logic
- 12 CLI subcommands fully functional
- Example config files (TOON + credentials.toml)
- Integration tests against GreenMail Docker
- Unit tests for all domain logic

### Definition of Done
- [ ] `cargo test --workspace` passes (all unit + integration tests)
- [ ] `cargo clippy --workspace -- -D warnings` clean
- [ ] `cargo fmt --check` clean
- [ ] All 12 subcommands functional against GreenMail
- [ ] Example config renders correct output

### Must Have
- Multi-account IMAP support (username/password auth)
- TOON config file parsing + credentials.toml
- Subcommands: list-accounts, doctor, check, folders, list, read, search, move, delete, flag, download, draft
- Table, TOON, and JSON output modes
- Structured logging with tracing
- TLS support (implicit TLS on 993)
- Proper error handling with thiserror/anyhow
- TDD for all modules

### Must NOT Have (Guardrails)
- **NO backend trait abstraction** — Direct IMAP implementation only. We have ONE backend.
- **NO feature gates for backends** — No `#[cfg(feature = "imap")]`. Everything is IMAP.
- **NO plugin system** — No dynamic loading, no extension points.
- **NO async trait objects** — Use concrete types, not `Box<dyn Backend>`.
- **NO email sending** — No SMTP, no sendmail, no message composition beyond drafts.
- **NO OAuth2** — Username/password only. No token refresh, no browser auth flow.
- **NO TUI/interactive mode** — No ratatui, no crossterm, no interactive prompts.
- **NO shell completions** — Can be added later, not in v1.
- **NO caching** — No local message cache, no offline mode. Every operation hits IMAP.
- **NO address book** — No contact management.
- **NO `unwrap()` in library code** — All errors propagated with `?` or `thiserror`.
- **NO `println!()` in library code** — Use `tracing` macros only. CLI binary handles output.
- **NO blocking I/O in async code** — No `std::fs` in async contexts; use `tokio::fs`.
- **NO over-abstraction** — No `Box<dyn Backend>`, no `Arc<dyn Fn>` patterns.

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: NO (greenfield — cargo test is built-in, no extra setup needed)
- **Automated tests**: TDD — RED (failing test) → GREEN (minimal impl) → REFACTOR
- **Framework**: `cargo test` (Rust built-in)
- **Integration tests**: GreenMail Docker container for real IMAP operations

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Library modules**: Use Bash (`cargo test`) — Run tests, assert pass counts
- **CLI commands**: Use Bash (`cargo run -p mailerboi --`) — Run subcommands, assert stdout/stderr
- **IMAP integration**: Use Bash (GreenMail Docker + cargo test) — Real IMAP operations
- **Config parsing**: Use Bash (inline TOON strings in tests) — Parse, validate, roundtrip

### GreenMail Docker Setup (for integration tests)
```bash
docker run -d --name greenmail \
  -e GREENMAIL_OPTS='-Dgreenmail.setup.test.all -Dgreenmail.hostname=0.0.0.0 -Dgreenmail.auth.disabled -Dgreenmail.verbose' \
  -p 3025:3025 -p 3110:3110 -p 3143:3143 -p 3465:3465 -p 3993:3993 -p 3995:3995 \
  greenmail/standalone:2.1.2
```

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Foundation — start immediately):
├── Task 1: Project scaffold (git init, workspace, devenv) [quick]
└── (sequential gate: Task 1 must complete before Wave 2)

Wave 2 (Spikes + Core Types — validate risky assumptions):
├── Task 2: Spike: toon-format custom struct serde (depends: 1) [deep]
├── Task 3: Spike: async-imap + TLS connection (depends: 1) [deep]
├── Task 4: Domain types: Folder, Envelope, Flag, Message (depends: 1) [unspecified-high]
└── Task 5: Error types with thiserror (depends: 1) [quick]

Wave 3 (Config + Connection — core infrastructure):
├── Task 6: TOON config parsing + credentials.toml (depends: 2, 4, 5) [deep]
├── Task 7: IMAP connection manager (depends: 3, 5) [deep]
├── Task 8: Output formatting: Table + TOON + JSON (depends: 2, 4) [unspecified-high]
└── Task 9: CLI skeleton with clap (depends: 4) [unspecified-high]

Wave 4 (IMAP Operations — core features, MAX PARALLEL):
├── Task 10: list-accounts + doctor commands (depends: 6, 7, 9) [unspecified-high]
├── Task 11: folders command — list mailboxes (depends: 7, 8, 9) [unspecified-high]
├── Task 12: list command — list envelopes (depends: 7, 8, 9) [unspecified-high]
├── Task 13: read command — fetch + parse email (depends: 7, 8, 9) [deep]
├── Task 14: search command — IMAP SEARCH with filters (depends: 7, 8, 9) [unspecified-high]
├── Task 15: check command — unread count (depends: 7, 8, 9) [quick]
├── Task 16: flag command — mark read/unread/flagged (depends: 7, 9) [unspecified-high]
├── Task 17: move command — with MOVE/COPY fallback (depends: 7, 9) [deep]
├── Task 18: delete command (depends: 7, 9) [unspecified-high]
├── Task 19: download command — attachments (depends: 13) [unspecified-high]
└── Task 20: draft command — IMAP APPEND (depends: 7, 9) [unspecified-high]

Wave 5 (Polish + Integration):
├── Task 21: CLI integration tests (depends: 10-20) [deep]
├── Task 22: Example configs + devenv polish (depends: 6) [quick]
└── Task 23: Final clippy/fmt/test cleanup (depends: 21, 22) [quick]

Wave FINAL (After ALL tasks — 4 parallel reviews, then user okay):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high)
└── Task F4: Scope fidelity check (deep)
-> Present results -> Get explicit user okay
```

### Critical Path
Task 1 → Task 2/3 → Task 6/7 → Task 13 → Task 19 → Task 21 → Task 23 → F1-F4 → user okay

### Dependency Matrix

| Task | Depends On | Blocks | Wave |
|------|-----------|--------|------|
| 1 | — | 2,3,4,5 | 1 |
| 2 | 1 | 6,8 | 2 |
| 3 | 1 | 7 | 2 |
| 4 | 1 | 6,8,9 | 2 |
| 5 | 1 | 6,7 | 2 |
| 6 | 2,4,5 | 10,22 | 3 |
| 7 | 3,5 | 10-20 | 3 |
| 8 | 2,4 | 11-15 | 3 |
| 9 | 4 | 10-20 | 3 |
| 10 | 6,7,9 | 21 | 4 |
| 11 | 7,8,9 | 21 | 4 |
| 12 | 7,8,9 | 21 | 4 |
| 13 | 7,8,9 | 19,21 | 4 |
| 14 | 7,8,9 | 21 | 4 |
| 15 | 7,8,9 | 21 | 4 |
| 16 | 7,9 | 21 | 4 |
| 17 | 7,9 | 21 | 4 |
| 18 | 7,9 | 21 | 4 |
| 19 | 13 | 21 | 4 |
| 20 | 7,9 | 21 | 4 |
| 21 | 10-20 | 23 | 5 |
| 22 | 6 | 23 | 5 |
| 23 | 21,22 | F1-F4 | 5 |

### Agent Dispatch Summary

- **Wave 1**: **1 task** — T1 → `quick`
- **Wave 2**: **4 tasks** — T2 → `deep`, T3 → `deep`, T4 → `unspecified-high`, T5 → `quick`
- **Wave 3**: **4 tasks** — T6 → `deep`, T7 → `deep`, T8 → `unspecified-high`, T9 → `unspecified-high`
- **Wave 4**: **11 tasks** — T10-T12,T14-T16,T18-T20 → `unspecified-high`, T13,T17 → `deep`, T15 → `quick`
- **Wave 5**: **3 tasks** — T21 → `deep`, T22 → `quick`, T23 → `quick`
- **FINAL**: **4 tasks** — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

- [x] 1. Project Scaffold — git init, workspace, devenv, tooling

  **What to do**:
  - Initialize git: `git init`
  - Update `.gitignore` with Rust patterns: `/target`, `Cargo.lock` (debate: include for binaries), `*.swp`, `*.swo`, `.sisyphus/evidence/`
  - Create workspace `Cargo.toml` at root with `resolver = "2"`, workspace members: `crates/mailerboi-core`, `crates/mailerboi`
  - Create `crates/mailerboi-core/Cargo.toml` with workspace version/edition inheritance
  - Create `crates/mailerboi-core/src/lib.rs` with module declarations (empty stubs)
  - Create `crates/mailerboi/Cargo.toml` with `[[bin]]` target, dep on `mailerboi-core`
  - Create `crates/mailerboi/src/main.rs` with `#[tokio::main]` stub returning `Ok(())`
  - Update `devenv.nix`: add `pkgs.pkg-config`, `pkgs.openssl` to packages (needed for native-tls), add `RUST_LOG = "debug"` env
  - Create `rust-toolchain.toml` with stable channel + components (rustfmt, clippy, rust-src)
  - Verify: `cargo check --workspace` passes

  **Must NOT do**:
  - Don't add actual dependencies yet (beyond tokio and anyhow for main.rs)
  - Don't create any domain logic
  - Don't set up test infrastructure beyond cargo's built-in

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []
  - **Reason**: Pure scaffolding, no complex logic

  **Parallelization**:
  - **Can Run In Parallel**: NO (first task)
  - **Parallel Group**: Wave 1 (solo)
  - **Blocks**: Tasks 2, 3, 4, 5
  - **Blocked By**: None

  **References**:
  - `/home/valerius/code/mailerboi/devenv.nix` — Existing devenv config to update with pkgs
  - `/home/valerius/code/mailerboi/devenv.yaml` — Don't modify this
  - `/home/valerius/code/mailerboi/.gitignore` — Existing gitignore to extend with Rust patterns

  **Acceptance Criteria**:
  - [ ] `git status` shows initialized repo
  - [ ] `cargo check --workspace` exits 0
  - [ ] `crates/mailerboi-core/src/lib.rs` exists
  - [ ] `crates/mailerboi/src/main.rs` compiles with tokio main

  **QA Scenarios**:
  ```
  Scenario: Workspace compiles clean
    Tool: Bash
    Preconditions: devenv shell active
    Steps:
      1. Run `cargo check --workspace`
      2. Assert exit code 0
      3. Run `cargo test --workspace`
      4. Assert exit code 0 (no tests yet, but should not fail)
    Expected Result: Both commands exit 0 with no errors
    Evidence: .sisyphus/evidence/task-1-workspace-check.txt

  Scenario: Git initialized correctly
    Tool: Bash
    Preconditions: None
    Steps:
      1. Run `git status` in project root
      2. Assert output contains "On branch" (not "fatal: not a git repository")
      3. Run `git log` — should show initial commit or be empty
    Expected Result: Git repo initialized, .gitignore tracks Rust patterns
    Evidence: .sisyphus/evidence/task-1-git-status.txt
  ```

  **Commit**: YES
  - Message: `chore: initialize workspace with cargo, git, devenv`
  - Files: `Cargo.toml`, `crates/*/Cargo.toml`, `crates/*/src/*`, `.gitignore`, `devenv.nix`, `rust-toolchain.toml`
  - Pre-commit: `cargo check --workspace`

---

- [x] 2. Spike: Validate toon-format Custom Struct Serde Roundtrip

  **What to do**:
  - Add `toon-format = "0.4"` and `serde = { version = "1", features = ["derive"] }` to `mailerboi-core` Cargo.toml
  - Write a test module in `mailerboi-core/src/lib.rs` (or separate test file) that:
    1. Defines a representative config struct with `#[derive(Serialize, Deserialize, Debug, PartialEq)]`:
       ```rust
       struct TestConfig {
           display_name: String,
           default: bool,
           accounts: HashMap<String, TestAccount>,
       }
       struct TestAccount {
           email: String,
           host: String,
           port: u16,
           tls: bool,
       }
       ```
    2. Creates an instance with test data
    3. Calls `toon_format::encode_default(&config)` — assert Ok
    4. Calls `toon_format::decode_default::<TestConfig>(&encoded)` — assert Ok
    5. Asserts roundtrip equality: `decoded == original`
    6. Tests edge cases: empty strings, special chars in passwords, nested structs, Option fields, Vec fields
  - **IF SPIKE FAILS**: Document the failure mode. Evaluate alternatives:
    - Use `serde_json::Value` as intermediary: struct → json → toon
    - Switch to `serde_toon` crate (has native serde `to_string`/`from_str`)
    - Fall back to TOML for config (user's second choice)
  - Record results and decision

  **Must NOT do**:
  - Don't build the actual config system — this is just validation
  - Don't commit broken code — if spike fails, document and pivot

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []
  - **Reason**: Needs careful exploration of an unvalidated API, potential pivoting

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 3, 4, 5)
  - **Blocks**: Tasks 6, 8
  - **Blocked By**: Task 1

  **References**:
  - `toon-format` docs: https://docs.rs/toon-format/0.4.4 — API reference for encode/decode
  - `toon-format` GitHub: https://github.com/toon-format/toon-rust — README with examples
  - If spike fails, alternatives: `serde_toon` (crates.io), `serde_toon2` (crates.io)

  **Acceptance Criteria**:
  - [ ] Test `spike_toon_roundtrip` passes: custom struct → TOON → custom struct
  - [ ] Test covers: nested structs, HashMap, Option, Vec, bool, u16, String
  - [ ] If fails: documented fallback plan with alternative approach implemented

  **QA Scenarios**:
  ```
  Scenario: toon-format roundtrip with custom structs
    Tool: Bash
    Preconditions: toon-format added to Cargo.toml
    Steps:
      1. Run `cargo test -p mailerboi-core spike_toon`
      2. If pass: assert exit 0, test output shows "test ... ok"
      3. If fail: check error message, document in .sisyphus/evidence/
    Expected Result: All spike tests pass OR clear failure documentation
    Evidence: .sisyphus/evidence/task-2-toon-spike.txt

  Scenario: Edge case — special characters in strings
    Tool: Bash
    Preconditions: Same as above
    Steps:
      1. Run test with password containing: commas, colons, newlines, unicode
      2. Assert roundtrip preserves exact string values
    Expected Result: Special characters survive encode/decode roundtrip
    Evidence: .sisyphus/evidence/task-2-toon-special-chars.txt
  ```

  **Commit**: YES
  - Message: `spike: validate toon-format custom struct serde roundtrip`
  - Pre-commit: `cargo test --workspace`

---

- [x] 3. Spike: Validate async-imap + TLS Connection to GreenMail

  **What to do**:
  - Add to `mailerboi-core` Cargo.toml: `async-imap = { version = "0.11", features = ["runtime-tokio"] }`, `async-native-tls = "0.5"`, `tokio = { version = "1", features = ["macros", "rt-multi-thread"] }`
  - Start GreenMail Docker: `docker run -d --name greenmail -e GREENMAIL_OPTS='-Dgreenmail.setup.test.all -Dgreenmail.hostname=0.0.0.0 -Dgreenmail.auth.disabled -Dgreenmail.verbose' -p 3025:3025 -p 3110:3110 -p 3143:3143 -p 3465:3465 -p 3993:3993 -p 3995:3995 greenmail/standalone:2.1.2`
  - Write an integration test (marked `#[ignore]` by default) that:
    1. Connects to GreenMail IMAP on `localhost:3143` (plain) or `localhost:3993` (TLS)
    2. For TLS: uses `async-native-tls` with `TlsConnector::new().danger_accept_invalid_certs(true)` (GreenMail self-signed)
    3. Logs in with test credentials (GreenMail auto-creates accounts on first login)
    4. Selects INBOX
    5. Lists mailboxes
    6. Logs out cleanly
  - Test STARTTLS on port 3143 as well (IMAP STARTTLS upgrade)
  - **IF SPIKE FAILS**: Document failure mode. Try `tokio-rustls` instead. If both fail, fall back to sync `imap` crate with `spawn_blocking`.
  - Record results and connection patterns

  **Must NOT do**:
  - Don't build the full connection manager — this is just validation
  - Don't hardcode any real credentials

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []
  - **Reason**: Docker setup + async networking + TLS — needs careful debugging

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 2, 4, 5)
  - **Blocks**: Task 7
  - **Blocked By**: Task 1

  **References**:
  - `async-imap` docs: https://docs.rs/async-imap — Client API, login, select, list
  - `async-native-tls` docs: https://docs.rs/async-native-tls — TlsConnector API
  - GreenMail docs: https://greenmail-mail-test.github.io/greenmail/ — Test mail server, ports, auto-account creation
  - GreenMail Docker: `greenmail/standalone:2.1.2` — latest stable image

  **Acceptance Criteria**:
  - [ ] Integration test connects to GreenMail on port 3993 (TLS) successfully
  - [ ] Integration test can login, SELECT INBOX, LIST mailboxes, LOGOUT
  - [ ] STARTTLS on port 3143 also validated
  - [ ] If fails: documented fallback with alternative approach

  **QA Scenarios**:
  ```
  Scenario: TLS connection to GreenMail
    Tool: Bash
    Preconditions: GreenMail Docker running on localhost:3993
    Steps:
      1. Run `docker ps | grep greenmail` — assert container running
      2. Run `cargo test -p mailerboi-core spike_imap -- --ignored` (run ignored integration tests)
      3. Assert exit 0
      4. Check test output for "SELECT INBOX" success
    Expected Result: Connection established, login succeeds, mailbox listed
    Evidence: .sisyphus/evidence/task-3-imap-spike.txt

  Scenario: Connection failure handling
    Tool: Bash
    Preconditions: GreenMail NOT running
    Steps:
      1. Stop GreenMail: `docker stop greenmail`
      2. Run connection test
      3. Assert graceful error (not panic)
    Expected Result: Error type returned, no panic, meaningful error message
    Evidence: .sisyphus/evidence/task-3-imap-error.txt
  ```

  **Commit**: YES
  - Message: `spike: validate async-imap + TLS connection to GreenMail`
  - Pre-commit: `cargo test --workspace`

---

- [x] 4. Domain Types — Folder, Envelope, Flag, Message

  **What to do**:
  - Create `crates/mailerboi-core/src/domain/mod.rs` with submodules
  - Create `crates/mailerboi-core/src/domain/folder.rs`:
    ```rust
    pub struct Folder { pub name: String, pub delimiter: Option<String>, pub attributes: Vec<String> }
    ```
    - Implement `Display` for human-readable output
    - Implement `Serialize` for JSON/TOON output
  - Create `crates/mailerboi-core/src/domain/envelope.rs`:
    ```rust
    pub struct Envelope {
        pub uid: u32,
        pub subject: Option<String>,
        pub from: Vec<Address>,
        pub to: Vec<Address>,
        pub date: Option<String>,
        pub flags: Vec<Flag>,
        pub has_attachments: bool,
    }
    pub struct Address { pub name: Option<String>, pub email: String }
    ```
  - Create `crates/mailerboi-core/src/domain/flag.rs`:
    ```rust
    pub enum Flag { Seen, Answered, Flagged, Deleted, Draft, Custom(String) }
    ```
    - Implement `From<&imap_types::Flag>` for conversion from IMAP flags
  - Create `crates/mailerboi-core/src/domain/message.rs`:
    ```rust
    pub struct Message {
        pub envelope: Envelope,
        pub text_body: Option<String>,
        pub html_body: Option<String>,
        pub attachments: Vec<Attachment>,
        pub raw: Vec<u8>,
    }
    pub struct Attachment { pub filename: String, pub content_type: String, pub size: usize, pub data: Vec<u8> }
    ```
  - TDD: Write tests FIRST for Display impls, serialization, flag conversions
  - All types derive: `Debug, Clone, Serialize, Deserialize` (where applicable)

  **Must NOT do**:
  - No IMAP parsing logic here — just data structures
  - No backend trait abstraction — just plain structs

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []
  - **Reason**: Well-defined struct creation with TDD, medium complexity

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 2, 3, 5)
  - **Blocks**: Tasks 6, 8, 9
  - **Blocked By**: Task 1

  **References**:
  - Himalaya domain types pattern: Folder, Envelope, Flag structs — see Metis analysis for field patterns
  - `serde` derive docs: https://serde.rs/derive.html
  - `mail-parser` types: https://docs.rs/mail-parser — MessageParser output types (for later conversion in Task 13)

  **Acceptance Criteria**:
  - [ ] `cargo test -p mailerboi-core domain` — all tests pass
  - [ ] Folder, Envelope, Flag, Message, Address, Attachment types exist
  - [ ] All types implement Debug, Clone, Serialize
  - [ ] Flag enum covers: Seen, Answered, Flagged, Deleted, Draft, Custom
  - [ ] Display impls produce readable output

  **QA Scenarios**:
  ```
  Scenario: Domain type serialization roundtrip
    Tool: Bash
    Steps:
      1. Run `cargo test -p mailerboi-core domain`
      2. Assert all tests pass
      3. Check test coverage: Folder, Envelope, Flag, Message, Address, Attachment
    Expected Result: All domain types serialize/deserialize correctly
    Evidence: .sisyphus/evidence/task-4-domain-types.txt

  Scenario: Flag conversion from string
    Tool: Bash
    Steps:
      1. Run test that converts "\\Seen" → Flag::Seen, "\\Flagged" → Flag::Flagged
      2. Run test that converts "custom-label" → Flag::Custom("custom-label")
    Expected Result: All IMAP flag strings map correctly to Flag enum variants
    Evidence: .sisyphus/evidence/task-4-flag-conversion.txt
  ```

  **Commit**: YES
  - Message: `feat: add domain types — Folder, Envelope, Flag, Message`
  - Pre-commit: `cargo test --workspace`

---

- [x] 5. Error Types with thiserror

  **What to do**:
  - Create `crates/mailerboi-core/src/error.rs`
  - Define error hierarchy:
    ```rust
    #[derive(Error, Debug)]
    pub enum MailerboiError {
        #[error("Config error: {0}")]
        Config(#[from] ConfigError),
        #[error("IMAP error: {0}")]
        Imap(#[from] ImapError),
        #[error("IO error: {0}")]
        Io(#[from] std::io::Error),
    }

    #[derive(Error, Debug)]
    pub enum ConfigError {
        #[error("Config file not found: {path}")]
        NotFound { path: PathBuf },
        #[error("Failed to parse config: {0}")]
        Parse(String),
        #[error("Credentials file not found: {path}")]
        CredentialsNotFound { path: PathBuf },
        #[error("Account '{name}' not found in config")]
        AccountNotFound { name: String },
    }

    #[derive(Error, Debug)]
    pub enum ImapError {
        #[error("Connection failed to {host}:{port}: {reason}")]
        ConnectionFailed { host: String, port: u16, reason: String },
        #[error("Authentication failed for {user}")]
        AuthFailed { user: String },
        #[error("Mailbox '{name}' not found")]
        MailboxNotFound { name: String },
        #[error("Message UID {uid} not found")]
        MessageNotFound { uid: u32 },
        #[error("IMAP protocol error: {0}")]
        Protocol(String),
        #[error("TLS error: {0}")]
        Tls(String),
    }
    ```
  - Add `thiserror = "2"` to mailerboi-core Cargo.toml
  - TDD: Write tests for error Display output and From conversions
  - Export as `pub type Result<T> = std::result::Result<T, MailerboiError>;`

  **Must NOT do**:
  - No anyhow in the library — thiserror only
  - Don't implement error recovery logic — just type definitions

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []
  - **Reason**: Simple error type definitions with derives

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 2, 3, 4)
  - **Blocks**: Tasks 6, 7
  - **Blocked By**: Task 1

  **References**:
  - `thiserror` docs: https://docs.rs/thiserror — derive Error macro
  - Himalaya error pattern: nested error enums with #[from] for automatic conversion

  **Acceptance Criteria**:
  - [ ] `cargo test -p mailerboi-core error` — all tests pass
  - [ ] Error hierarchy: MailerboiError → ConfigError, ImapError
  - [ ] All errors implement Display with meaningful messages
  - [ ] `Result<T>` type alias exported

  **QA Scenarios**:
  ```
  Scenario: Error display messages
    Tool: Bash
    Steps:
      1. Run `cargo test -p mailerboi-core error`
      2. Assert tests verify Display output for each error variant
    Expected Result: Error messages are human-readable and include context
    Evidence: .sisyphus/evidence/task-5-error-types.txt
  ```

  **Commit**: YES
  - Message: `feat: add error types with thiserror`
  - Pre-commit: `cargo test --workspace`

- [x] 6. TOON Config Parsing + credentials.toml Loading

  **What to do**:
  - Add deps: `toon-format`, `toml`, `serde`, `dirs` (for XDG paths), `tracing`
  - Create `crates/mailerboi-core/src/config/mod.rs` with:
    - `AppConfig` struct (deserialized from TOON): accounts map, default_account, global settings
    - `AccountConfig` struct: email, display_name, imap host/port/tls/starttls/insecure, default_mailbox
    - `Credentials` struct (deserialized from TOML): account_name → password mapping
  - Config loading logic:
    1. Default path: `~/.config/mailerboi/config.toon` (XDG_CONFIG_HOME)
    2. Override via `--config` CLI flag or `MAILERBOI_CONFIG` env var
    3. Parse with `toon_format::decode_default()` (or fallback from spike)
  - Credentials loading:
    1. Default path: `~/.config/mailerboi/credentials.toml`
    2. Override via `MAILERBOI_CREDENTIALS` env var
    3. Parse with `toml::from_str()`
    4. Warn if file permissions are world-readable (mode check on Unix)
  - Account resolution: `resolve_account(name: Option<&str>) -> Result<(&AccountConfig, &str)>`
    - If name given: look up in accounts map
    - If not given: use account marked `default = true`, or first account
  - TDD: test config parsing with inline TOON strings, test credentials loading, test account resolution

  **Must NOT do**:
  - Don't implement IMAP connection here — just config loading
  - Don't use keyring or shell command for secrets — plaintext TOML only
  - Don't create actual config files on disk — tests use inline strings

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []
  - **Reason**: Complex serde with TOON (potentially needs spike fallback), file system interaction, XDG paths

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 7, 8, 9)
  - **Blocks**: Tasks 10, 22
  - **Blocked By**: Tasks 2 (toon-format spike result), 4 (domain types), 5 (error types)

  **References**:
  - Task 2 spike results — determines whether to use `toon_format::decode_default()` directly or via Value intermediary
  - `dirs` crate: https://docs.rs/dirs — `config_dir()` for XDG_CONFIG_HOME
  - `toml` crate: https://docs.rs/toml — `from_str()` for credentials parsing
  - imap-smtp-email .env pattern (inspiration): prefix-based multi-account — we use TOON sections instead
  - Himalaya config.sample.toml: `[accounts.name]` nested sections — translate to TOON equivalent

  **Acceptance Criteria**:
  - [ ] `cargo test -p mailerboi-core config` — all tests pass
  - [ ] TOON config with 2+ accounts parses correctly
  - [ ] credentials.toml with matching accounts loads passwords
  - [ ] Account resolution: by name, by default flag, first-account fallback
  - [ ] Missing config file → ConfigError::NotFound
  - [ ] Malformed TOON → ConfigError::Parse with meaningful message

  **QA Scenarios**:
  ```
  Scenario: Parse multi-account TOON config
    Tool: Bash
    Steps:
      1. Run `cargo test -p mailerboi-core config::tests::parse_multi_account`
      2. Assert test creates 2-account config, verifies all fields
    Expected Result: Both accounts parsed with correct host, port, email
    Evidence: .sisyphus/evidence/task-6-config-parse.txt

  Scenario: Missing credentials file
    Tool: Bash
    Steps:
      1. Run `cargo test -p mailerboi-core config::tests::missing_credentials`
      2. Assert ConfigError::CredentialsNotFound returned
    Expected Result: Meaningful error, not panic
    Evidence: .sisyphus/evidence/task-6-config-error.txt
  ```

  **Commit**: YES
  - Message: `feat: add TOON config parsing + credentials.toml`
  - Pre-commit: `cargo test --workspace`

---

- [x] 7. IMAP Connection Manager with TLS

  **What to do**:
  - Create `crates/mailerboi-core/src/imap/mod.rs` with:
    - `ImapSession` struct wrapping `async_imap::Session<TlsStream<TcpStream>>` (or plain stream)
    - `connect(config: &AccountConfig, password: &str) -> Result<ImapSession>`:
      1. Resolve host + port from config
      2. If TLS (port 993): connect with `async-native-tls` TlsConnector
      3. If STARTTLS (port 143): connect plain, then upgrade with STARTTLS command
      4. If insecure flag: `danger_accept_invalid_certs(true)`
      5. Login with username + password
      6. Return wrapped session
    - `disconnect(session: ImapSession) -> Result<()>`: LOGOUT cleanly
    - `doctor(config: &AccountConfig, password: &str) -> Result<DoctorReport>`:
      1. Test DNS resolution
      2. Test TCP connection
      3. Test TLS handshake
      4. Test authentication
      5. Test SELECT INBOX
      6. Return report with pass/fail per step
  - Use `tracing::instrument` for all public functions
  - TDD: unit tests for config → connection params, integration tests against GreenMail

  **Must NOT do**:
  - No connection pooling — one connection per operation
  - No reconnection logic — fail and let CLI retry
  - No IDLE support

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []
  - **Reason**: Async networking, TLS, IMAP protocol — needs careful implementation

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 6, 8, 9)
  - **Blocks**: Tasks 10-20
  - **Blocked By**: Tasks 3 (imap spike result), 5 (error types)

  **References**:
  - Task 3 spike results — proven connection pattern from spike
  - `async-imap` API: `Client::secure_connect()`, `Client::connect()`, `.login()`, `.select()`, `.logout()`
  - `async-native-tls`: `TlsConnector::new()`, `.danger_accept_invalid_certs()`, `.connect()`
  - GreenMail ports: 3143 (IMAP), 3993 (IMAPS) — for integration tests

  **Acceptance Criteria**:
  - [ ] `cargo test -p mailerboi-core imap` — all unit tests pass
  - [ ] Integration test connects to GreenMail on 3993 (TLS)
  - [ ] Integration test connects to GreenMail on 3143 (STARTTLS)
  - [ ] Doctor command returns structured report with 5 checkpoints
  - [ ] Connection failure returns ImapError::ConnectionFailed (not panic)

  **QA Scenarios**:
  ```
  Scenario: TLS connection + login + SELECT INBOX
    Tool: Bash
    Preconditions: GreenMail Docker running
    Steps:
      1. Run `cargo test -p mailerboi-core imap::tests::connect_tls -- --ignored`
      2. Assert test connects, logs in, selects INBOX, logs out
    Expected Result: Full connection lifecycle completes without error
    Evidence: .sisyphus/evidence/task-7-imap-connect.txt

  Scenario: Auth failure with wrong password
    Tool: Bash
    Preconditions: GreenMail Docker running
    Steps:
      1. Run test with intentionally wrong password
      2. Assert ImapError::AuthFailed returned
    Expected Result: Meaningful auth error, clean connection teardown
    Evidence: .sisyphus/evidence/task-7-auth-failure.txt
  ```

  **Commit**: YES
  - Message: `feat: add IMAP connection manager with TLS`
  - Pre-commit: `cargo test --workspace`

---

- [x] 8. Output Formatting — Table, TOON, JSON

  **What to do**:
  - Add deps: `comfy-table` (or `tabled`), `serde_json`, `toon-format`
  - Create `crates/mailerboi-core/src/output.rs` with:
    - `OutputFormat` enum: `Table`, `Toon`, `Json`
    - `fn format_folders(folders: &[Folder], format: OutputFormat) -> String`
    - `fn format_envelopes(envelopes: &[Envelope], format: OutputFormat) -> String`
    - `fn format_message(message: &Message, format: OutputFormat) -> String`
    - `fn format_check(checks: &[AccountCheck], format: OutputFormat) -> String`
    - Table output: human-readable with column headers, alignment, truncation
    - JSON output: `serde_json::to_string_pretty()`
    - TOON output: `toon_format::encode_default()`
  - Table layout for envelopes: `UID | From | Subject | Date | Flags`
  - Table layout for folders: `Name | Messages | Unseen`
  - TDD: test all 3 formats for each domain type

  **Must NOT do**:
  - No colored output (can add later)
  - No pagination in output module — CLI handles pagination

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []
  - **Reason**: Multiple output format implementations, table layout design

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 6, 7, 9)
  - **Blocks**: Tasks 11-15
  - **Blocked By**: Tasks 2 (toon spike), 4 (domain types)

  **References**:
  - `comfy-table` docs: https://docs.rs/comfy-table — Table creation, column alignment
  - `serde_json::to_string_pretty` — JSON output
  - `toon_format::encode_default` — TOON output
  - Domain types from Task 4 — Folder, Envelope, Message structs

  **Acceptance Criteria**:
  - [ ] `cargo test -p mailerboi-core output` — all tests pass
  - [ ] Table output has aligned columns with headers
  - [ ] JSON output is valid, pretty-printed JSON
  - [ ] TOON output encodes correctly
  - [ ] Each domain type has all 3 format tests

  **QA Scenarios**:
  ```
  Scenario: Envelope list in all 3 formats
    Tool: Bash
    Steps:
      1. Run `cargo test -p mailerboi-core output::tests::envelope_table`
      2. Run `cargo test -p mailerboi-core output::tests::envelope_json`
      3. Run `cargo test -p mailerboi-core output::tests::envelope_toon`
      4. Assert all pass
    Expected Result: Each format produces correct, parseable output
    Evidence: .sisyphus/evidence/task-8-output-formats.txt
  ```

  **Commit**: YES
  - Message: `feat: add output formatting — table, TOON, JSON`
  - Pre-commit: `cargo test --workspace`

---

- [x] 9. CLI Skeleton with Clap Subcommands

  **What to do**:
  - Add deps to `mailerboi` Cargo.toml: `clap = { version = "4", features = ["derive", "env"] }`, `anyhow`, `tracing-subscriber`, `mailerboi-core`
  - Create `crates/mailerboi/src/cli.rs` with clap derive:
    ```rust
    #[derive(Parser)]
    #[command(name = "mailerboi", about = "Multi-account IMAP email CLI")]
    pub struct Cli {
        #[arg(short, long, global = true, env = "MAILERBOI_CONFIG")]
        pub config: Option<PathBuf>,
        #[arg(short, long, global = true)]
        pub account: Option<String>,
        #[arg(short, long, global = true, default_value = "table")]
        pub output: OutputFormat,
        #[arg(long, global = true)]
        pub insecure: bool,
        #[command(subcommand)]
        pub command: Commands,
    }
    ```
  - Define all subcommand stubs in `Commands` enum:
    - `ListAccounts` — no args
    - `Doctor` — no extra args
    - `Check` — optional `--mailbox`
    - `Folders` — no extra args
    - `List` — `--mailbox`, `--limit`, `--page`
    - `Read` — `uid: u32`, `--format` (text/html/raw/headers)
    - `Search` — `--unseen`, `--seen`, `--from`, `--subject`, `--since`, `--before`, `--recent`, `--limit`, `--mailbox`
    - `Move` — `uid: u32`, `target: String` (target folder)
    - `Delete` — `uid: u32`, `--force` (skip trash)
    - `Flag` — `uid: u32`, `--set`/`--unset` with flag name
    - `Download` — `uid: u32`, `--dir`, `--file`
    - `Draft` — `--subject`, `--body`, `--body-file`
  - Update `main.rs`: parse CLI, init tracing-subscriber, stub command dispatch (just print "not implemented yet")
  - TDD: test CLI parsing for each subcommand
  - Exit codes: 0=success, 1=runtime error, 2=config error

  **Must NOT do**:
  - Don't implement actual command logic — just CLI parsing + dispatch stubs
  - No shell completions

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []
  - **Reason**: Extensive clap derive setup with many subcommands

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 6, 7, 8)
  - **Blocks**: Tasks 10-20
  - **Blocked By**: Task 4 (domain types for OutputFormat enum)

  **References**:
  - `clap` v4 derive: https://docs.rs/clap/4/clap/_derive/index.html — Parser, Subcommand, Args derives
  - Himalaya CLI structure: nested subcommand enums from Metis analysis
  - imap-smtp-email command patterns: `check`, `fetch`, `search`, `mark-read`, `list-mailboxes`

  **Acceptance Criteria**:
  - [ ] `cargo test -p mailerboi` — CLI parsing tests pass
  - [ ] `cargo run -p mailerboi -- --help` shows all 12 subcommands
  - [ ] Each subcommand --help shows correct args
  - [ ] `--account`, `--output`, `--config` global flags work
  - [ ] Unknown subcommand exits with code 2

  **QA Scenarios**:
  ```
  Scenario: CLI help output
    Tool: Bash
    Steps:
      1. Run `cargo run -p mailerboi -- --help`
      2. Assert output contains all 12 subcommand names
      3. Assert output contains --account, --output, --config flags
    Expected Result: Help text lists all commands and global flags
    Evidence: .sisyphus/evidence/task-9-cli-help.txt

  Scenario: Subcommand arg parsing
    Tool: Bash
    Steps:
      1. Run `cargo run -p mailerboi -- read --help`
      2. Assert shows uid argument and --format flag
      3. Run `cargo run -p mailerboi -- search --help`
      4. Assert shows all filter flags
    Expected Result: Each subcommand has correct argument definitions
    Evidence: .sisyphus/evidence/task-9-cli-subcommands.txt
  ```

  **Commit**: YES
  - Message: `feat: add CLI skeleton with clap subcommands`
  - Pre-commit: `cargo test --workspace`

- [x] 10. list-accounts + doctor Commands

  **What to do**:
  - Implement `list-accounts` command in `crates/mailerboi/src/cmd/accounts.rs`:
    - Load config, iterate accounts, display: name, email, host, port, default status
    - Output via format_accounts() (add to output module if needed)
  - Implement `doctor` command in `crates/mailerboi/src/cmd/doctor.rs`:
    - Load config + credentials for specified account (or default)
    - Call `ImapSession::doctor()` from Task 7
    - Display step-by-step results: DNS ✓, TCP ✓, TLS ✓, AUTH ✓, INBOX ✓
    - If any step fails, show error and stop
  - Wire both commands into main.rs dispatch
  - TDD: test list-accounts output, test doctor against GreenMail

  **Must NOT do**:
  - Don't test against real email servers — GreenMail only

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 11-20)
  - **Blocks**: Task 21
  - **Blocked By**: Tasks 6, 7, 9

  **References**:
  - Task 6: config loading functions
  - Task 7: `ImapSession::doctor()` function
  - Task 8: output formatting functions
  - Task 9: CLI dispatch pattern
  - imap-smtp-email `list-accounts` pattern: shows name, email, server, status

  **Acceptance Criteria**:
  - [ ] `cargo run -p mailerboi -- list-accounts` shows configured accounts in table format
  - [ ] `cargo run -p mailerboi -- doctor --account test` tests connectivity step-by-step
  - [ ] Doctor shows clear pass/fail per step with error details on failure
  - [ ] All 3 output formats work for both commands

  **QA Scenarios**:
  ```
  Scenario: List accounts from config
    Tool: Bash
    Preconditions: Test config file with 2 accounts
    Steps:
      1. Run `cargo run -p mailerboi -- --config test-config.toon list-accounts`
      2. Assert output table has 2 rows with correct email addresses
    Expected Result: Both accounts listed with name, email, host
    Evidence: .sisyphus/evidence/task-10-list-accounts.txt

  Scenario: Doctor against GreenMail
    Tool: Bash
    Preconditions: GreenMail running
    Steps:
      1. Run doctor command against GreenMail account
      2. Assert all 5 steps show ✓ (pass)
    Expected Result: Full diagnostic passes
    Evidence: .sisyphus/evidence/task-10-doctor.txt
  ```

  **Commit**: YES
  - Message: `feat: add list-accounts + doctor commands`
  - Pre-commit: `cargo test --workspace`

---

- [x] 11. folders Command — List Mailboxes

  **What to do**:
  - Add `list_folders()` to `crates/mailerboi-core/src/imap/mod.rs`:
    - Send IMAP LIST "" "*" command
    - Parse response into `Vec<Folder>` domain types
    - Handle modified UTF-7 mailbox names (IMAP encoding)
  - Implement `folders` command in `crates/mailerboi/src/cmd/folders.rs`:
    - Connect, list folders, disconnect, format output
  - TDD: test folder listing against GreenMail, test UTF-7 name handling

  **Must NOT do**:
  - Don't show message counts per folder here (that's `check` command)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: Task 21
  - **Blocked By**: Tasks 7, 8, 9

  **References**:
  - `async-imap` LIST command: `session.list(Some(""), Some("*"))`
  - IMAP modified UTF-7: RFC 3501 §5.1.3 — mailbox names encoding
  - Task 4: Folder domain type

  **Acceptance Criteria**:
  - [ ] `cargo run -p mailerboi -- folders` lists INBOX and any other default mailboxes
  - [ ] Table output shows: Name, Delimiter, Attributes
  - [ ] GreenMail integration test verifies folder listing

  **QA Scenarios**:
  ```
  Scenario: List folders from GreenMail
    Tool: Bash
    Preconditions: GreenMail running
    Steps:
      1. Run folders command against GreenMail
      2. Assert INBOX appears in output
    Expected Result: At least INBOX listed with correct attributes
    Evidence: .sisyphus/evidence/task-11-folders.txt
  ```

  **Commit**: YES
  - Message: `feat: add folders command`
  - Pre-commit: `cargo test --workspace`

---

- [x] 12. list Command — Envelope Listing

  **What to do**:
  - Add `list_envelopes(mailbox: &str, limit: u32, page: u32) -> Result<Vec<Envelope>>` to imap module:
    - SELECT mailbox
    - FETCH range with ENVELOPE and FLAGS
    - Parse into `Vec<Envelope>` using async-imap's Fetch type
    - Support pagination via UID ranges (calculate from total count)
    - Use UIDs (not sequence numbers) for stable references
  - Implement `list` command in `crates/mailerboi/src/cmd/list.rs`:
    - `--mailbox` (default INBOX), `--limit` (default 20), `--page` (default 1)
    - Format output with envelopes table
  - TDD: test envelope parsing, test pagination logic, integration test with GreenMail

  **Must NOT do**:
  - Don't fetch message bodies — just envelopes (headers + flags)
  - Don't implement search here — separate command

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: Task 21
  - **Blocked By**: Tasks 7, 8, 9

  **References**:
  - `async-imap` FETCH: `session.uid_fetch("1:*", "ENVELOPE FLAGS")` — returns Vec<Fetch>
  - `async-imap` Fetch type: `.envelope()`, `.flags()`, `.uid` fields
  - Task 4: Envelope domain type — field mapping from IMAP Fetch

  **Acceptance Criteria**:
  - [ ] `cargo run -p mailerboi -- list` shows envelopes from INBOX
  - [ ] Table output: UID, From, Subject, Date, Flags columns
  - [ ] `--limit 5` shows max 5 results
  - [ ] `--mailbox Sent` lists sent folder
  - [ ] Empty mailbox shows "No messages" (not error)

  **QA Scenarios**:
  ```
  Scenario: List emails from INBOX
    Tool: Bash
    Preconditions: GreenMail running, test email sent to test account
    Steps:
      1. Send test email via GreenMail SMTP
      2. Run `cargo run -p mailerboi -- list --account test --limit 10`
      3. Assert output contains test email subject
    Expected Result: Email envelope shown with correct From, Subject, Date
    Evidence: .sisyphus/evidence/task-12-list.txt

  Scenario: Empty mailbox
    Tool: Bash
    Steps:
      1. Run list on empty mailbox
      2. Assert no error, shows "No messages" or empty table
    Expected Result: Graceful handling of zero messages
    Evidence: .sisyphus/evidence/task-12-empty.txt
  ```

  **Commit**: YES
  - Message: `feat: add list command — envelope listing`
  - Pre-commit: `cargo test --workspace`

---

- [ ] 13. read Command — Fetch + Parse Email

  **What to do**:
  - Add `mail-parser = "0.11"` to mailerboi-core deps
  - Add `fetch_message(uid: u32, mailbox: &str) -> Result<Message>` to imap module:
    - SELECT mailbox
    - FETCH uid with `RFC822` (full message) + `FLAGS`
    - Parse raw bytes with `mail_parser::MessageParser::default().parse()`
    - Map to Message domain type: envelope fields, text_body, html_body, attachments list
  - Implement `read` command in `crates/mailerboi/src/cmd/read.rs`:
    - `uid` positional arg
    - `--format` flag: text (default), html, raw, headers
    - text: show text_body, fallback to stripped html_body
    - html: show raw HTML
    - raw: show full RFC822
    - headers: show key headers only (From, To, Subject, Date, CC)
  - Output formatting: plain text for `read` (not table), JSON/TOON for --output json/toon
  - TDD: test email parsing with sample .eml content, test format flag behavior

  **Must NOT do**:
  - Don't auto-mark as read (user controls flags explicitly)
  - Don't open in browser/pager

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []
  - **Reason**: Email parsing is complex (MIME, multipart, charset handling)

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: Tasks 19, 21
  - **Blocked By**: Tasks 7, 8, 9

  **References**:
  - `mail-parser` API: `MessageParser::default().parse(raw)` → `Message` with `.body_text()`, `.body_html()`, `.attachment()`
  - `mail-parser` docs: https://docs.rs/mail-parser/0.11 — full API reference
  - Task 4: Message, Attachment domain types

  **Acceptance Criteria**:
  - [ ] `cargo run -p mailerboi -- read 1` displays email text body
  - [ ] `--format html` shows HTML body
  - [ ] `--format headers` shows From, To, Subject, Date
  - [ ] `--format raw` shows full RFC822
  - [ ] Multipart email correctly extracts text and HTML parts
  - [ ] Non-ASCII subjects (UTF-8, encoded-words) display correctly

  **QA Scenarios**:
  ```
  Scenario: Read plain text email
    Tool: Bash
    Preconditions: GreenMail with test email
    Steps:
      1. Run `cargo run -p mailerboi -- read 1 --account test`
      2. Assert output contains email body text
      3. Assert output shows From, Subject header info
    Expected Result: Email body displayed with headers
    Evidence: .sisyphus/evidence/task-13-read.txt

  Scenario: Read with JSON output
    Tool: Bash
    Steps:
      1. Run `cargo run -p mailerboi -- read 1 --account test --output json`
      2. Parse stdout as JSON
      3. Assert JSON has "text_body", "envelope.subject" fields
    Expected Result: Valid JSON with all message fields
    Evidence: .sisyphus/evidence/task-13-read-json.txt
  ```

  **Commit**: YES
  - Message: `feat: add read command — email parsing + display`
  - Pre-commit: `cargo test --workspace`

---

- [ ] 14. search Command — IMAP SEARCH with Filters

  **What to do**:
  - Add `search_messages(mailbox: &str, query: SearchQuery) -> Result<Vec<Envelope>>` to imap module:
    - Build IMAP SEARCH string from SearchQuery fields
    - Supported filters: UNSEEN, SEEN, FROM "x", SUBJECT "x", SINCE date, BEFORE date
    - `--recent` flag: convert "2h"/"7d" to SINCE date
    - Combine filters with AND (IMAP SEARCH default)
    - FETCH envelopes for matching UIDs
  - Define `SearchQuery` struct in domain types:
    ```rust
    pub struct SearchQuery {
        pub unseen: bool, pub seen: bool,
        pub from: Option<String>, pub subject: Option<String>,
        pub since: Option<NaiveDate>, pub before: Option<NaiveDate>,
        pub limit: u32,
    }
    ```
  - Implement `search` command in `crates/mailerboi/src/cmd/search.rs`
  - TDD: test IMAP SEARCH string building, test with GreenMail

  **Must NOT do**:
  - No full-text body search (IMAP TEXT search is very slow)
  - No OR logic — AND only (matches imap-smtp-email behavior)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: Task 21
  - **Blocked By**: Tasks 7, 8, 9

  **References**:
  - `async-imap` SEARCH: `session.uid_search("UNSEEN FROM \"alice\"")` — returns Vec<u32> UIDs
  - IMAP SEARCH spec: RFC 3501 §6.4.4 — search keys
  - imap-smtp-email search pattern: --unseen, --from, --subject, --since, --before, --recent

  **Acceptance Criteria**:
  - [ ] `cargo run -p mailerboi -- search --unseen` shows only unread emails
  - [ ] `cargo run -p mailerboi -- search --from alice` filters by sender
  - [ ] Filters combine: `--unseen --from alice` → AND logic
  - [ ] `--recent 2h` converts to correct SINCE date
  - [ ] `--limit 5` caps results

  **QA Scenarios**:
  ```
  Scenario: Search unseen emails
    Tool: Bash
    Preconditions: GreenMail with mix of read/unread emails
    Steps:
      1. Run `cargo run -p mailerboi -- search --unseen --account test`
      2. Assert only unread emails appear
    Expected Result: Filtered results show only unseen messages
    Evidence: .sisyphus/evidence/task-14-search.txt
  ```

  **Commit**: YES
  - Message: `feat: add search command with IMAP filters`
  - Pre-commit: `cargo test --workspace`

---

- [ ] 15. check Command — Unread Count

  **What to do**:
  - Add `check_mailbox(mailbox: &str) -> Result<MailboxStatus>` to imap module:
    - Use IMAP STATUS command: `STATUS "INBOX" (MESSAGES UNSEEN RECENT)`
    - Return struct: `{ total: u32, unseen: u32, recent: u32 }`
  - Implement `check` command in `crates/mailerboi/src/cmd/check.rs`:
    - If no `--mailbox`: check INBOX for each configured account
    - If `--mailbox` specified: check that specific mailbox
    - Table output: Account | Mailbox | Total | Unseen | Recent
  - TDD: test STATUS parsing, test multi-account check

  **Must NOT do**:
  - No IDLE — just a one-shot status check
  - No notification/watch mode

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []
  - **Reason**: Simple STATUS command wrapper

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: Task 21
  - **Blocked By**: Tasks 7, 8, 9

  **References**:
  - `async-imap` STATUS: check if `status()` method exists, or use raw `run_command_and_read_response`
  - IMAP STATUS spec: RFC 3501 §6.3.10

  **Acceptance Criteria**:
  - [ ] `cargo run -p mailerboi -- check` shows unread count per account
  - [ ] Multi-account: checks all configured accounts
  - [ ] Table output: Account, Mailbox, Total, Unseen, Recent columns

  **QA Scenarios**:
  ```
  Scenario: Check unread count
    Tool: Bash
    Preconditions: GreenMail with some unread emails
    Steps:
      1. Run `cargo run -p mailerboi -- check --account test`
      2. Assert output shows INBOX with correct unseen count
    Expected Result: Accurate unread count displayed
    Evidence: .sisyphus/evidence/task-15-check.txt
  ```

  **Commit**: YES
  - Message: `feat: add check command — unread count`
  - Pre-commit: `cargo test --workspace`

- [ ] 16. flag Command — Mark Read/Unread/Flagged

  **What to do**:
  - Add `set_flags(uid: u32, mailbox: &str, flags: &[Flag], action: FlagAction) -> Result<()>` to imap module:
    - `FlagAction::Set` → IMAP STORE +FLAGS
    - `FlagAction::Unset` → IMAP STORE -FLAGS
    - Map Flag enum to IMAP flag strings: Seen→\Seen, Flagged→\Flagged, etc.
    - Use UID STORE (not sequence numbers)
  - Implement `flag` command in `crates/mailerboi/src/cmd/flag.rs`:
    - `uid: u32` positional
    - `--set <flag>` or `--unset <flag>` (flag names: seen, flagged, answered, draft)
    - Shorthand: `--read` = `--set seen`, `--unread` = `--unset seen`
    - Support multiple UIDs: `flag 1 2 3 --set seen`
  - TDD: test flag operations against GreenMail

  **Must NOT do**:
  - No custom flags beyond IMAP standard set + Custom(String)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: Task 21
  - **Blocked By**: Tasks 7, 9

  **References**:
  - `async-imap` STORE: `session.uid_store("1", "+FLAGS (\\Seen)")` — flag manipulation
  - Task 4: Flag enum — mapping to IMAP flag strings
  - imap-smtp-email mark-read/mark-unread pattern

  **Acceptance Criteria**:
  - [ ] `cargo run -p mailerboi -- flag 1 --set seen` marks email as read
  - [ ] `cargo run -p mailerboi -- flag 1 --unset seen` marks as unread
  - [ ] `--read` and `--unread` shorthands work
  - [ ] Multiple UIDs supported: `flag 1 2 3 --read`
  - [ ] GreenMail integration test verifies flag persistence

  **QA Scenarios**:
  ```
  Scenario: Mark email as read
    Tool: Bash
    Preconditions: GreenMail with unread email UID 1
    Steps:
      1. Run `cargo run -p mailerboi -- flag 1 --read --account test`
      2. Run `cargo run -p mailerboi -- list --account test`
      3. Assert UID 1 now shows Seen flag
    Expected Result: Flag persisted, visible in subsequent list
    Evidence: .sisyphus/evidence/task-16-flag.txt
  ```

  **Commit**: YES
  - Message: `feat: add flag command — mark read/unread`
  - Pre-commit: `cargo test --workspace`

---

- [ ] 17. move Command — with MOVE/COPY Fallback

  **What to do**:
  - Add `move_message(uid: u32, source: &str, target: &str) -> Result<()>` to imap module:
    1. Check server CAPABILITY for MOVE (RFC 6851)
    2. If MOVE supported: `session.uid_mv(uid, target)`
    3. If MOVE NOT supported: fallback:
       a. `session.uid_copy(uid, target)` — copy to target
       b. `session.uid_store(uid, "+FLAGS (\\Deleted)")` — mark deleted in source
       c. `session.expunge()` — permanently remove from source
    4. Verify target mailbox exists before move (return MailboxNotFound if not)
  - Implement `move` command in `crates/mailerboi/src/cmd/move_cmd.rs` (avoid Rust keyword):
    - `uid: u32` positional, `target: String` positional
    - `--mailbox` for source (default INBOX)
  - TDD: test both MOVE and COPY fallback paths, test non-existent target

  **Must NOT do**:
  - Don't create target mailbox if it doesn't exist
  - Don't batch move (single UID per invocation for safety)

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []
  - **Reason**: Two code paths (MOVE vs fallback), capability detection

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: Task 21
  - **Blocked By**: Tasks 7, 9

  **References**:
  - `async-imap` MOVE: check if `uid_mv()` exists (may be `uid_move()`)
  - `async-imap` COPY: `session.uid_copy(uid, mailbox_name)`
  - `async-imap` STORE: `session.uid_store(uid, "+FLAGS (\\Deleted)")`
  - `async-imap` CAPABILITY: `session.capabilities()` → check for "MOVE"
  - RFC 6851: IMAP MOVE extension

  **Acceptance Criteria**:
  - [ ] `cargo run -p mailerboi -- move 1 Trash` moves email from INBOX to Trash
  - [ ] Works with MOVE command if server supports it
  - [ ] Falls back to COPY+DELETE if MOVE not supported
  - [ ] Non-existent target returns MailboxNotFound error

  **QA Scenarios**:
  ```
  Scenario: Move email to Trash
    Tool: Bash
    Preconditions: GreenMail with email UID 1 in INBOX
    Steps:
      1. Run `cargo run -p mailerboi -- move 1 Trash --account test`
      2. Run `cargo run -p mailerboi -- list --account test`
      3. Assert UID 1 no longer in INBOX
    Expected Result: Email moved, no longer visible in source
    Evidence: .sisyphus/evidence/task-17-move.txt

  Scenario: Move to non-existent folder
    Tool: Bash
    Steps:
      1. Run `cargo run -p mailerboi -- move 1 NonExistentFolder --account test`
      2. Assert error message mentions "not found"
    Expected Result: Meaningful error, email NOT deleted from source
    Evidence: .sisyphus/evidence/task-17-move-error.txt
  ```

  **Commit**: YES
  - Message: `feat: add move command with MOVE/COPY fallback`
  - Pre-commit: `cargo test --workspace`

---

- [ ] 18. delete Command

  **What to do**:
  - Add `delete_message(uid: u32, mailbox: &str, force: bool) -> Result<()>` to imap module:
    - If `force = false`: move to Trash folder (use move_message logic)
    - If `force = true`: STORE +FLAGS \\Deleted then EXPUNGE — permanent delete
    - Log warning on permanent delete via tracing
  - Implement `delete` command in `crates/mailerboi/src/cmd/delete.rs`:
    - `uid: u32` positional
    - `--force` flag for permanent delete (default: move to Trash)
    - `--mailbox` for source (default INBOX)
  - TDD: test soft delete (move to trash), test hard delete (force), test already-deleted

  **Must NOT do**:
  - No batch delete
  - No confirmation prompt (pure CLI, no interactivity)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: Task 21
  - **Blocked By**: Tasks 7, 9

  **References**:
  - Task 17: move_message logic — reuse for soft delete
  - `async-imap` EXPUNGE: `session.expunge()` — permanent removal
  - IMAP \Deleted flag + EXPUNGE flow

  **Acceptance Criteria**:
  - [ ] `cargo run -p mailerboi -- delete 1` moves to Trash (soft delete)
  - [ ] `cargo run -p mailerboi -- delete 1 --force` permanently removes
  - [ ] Deleting non-existent UID returns MessageNotFound error

  **QA Scenarios**:
  ```
  Scenario: Soft delete (move to Trash)
    Tool: Bash
    Steps:
      1. Run `cargo run -p mailerboi -- delete 1 --account test`
      2. Assert email moved to Trash, not in INBOX
    Expected Result: Email in Trash, not permanently deleted
    Evidence: .sisyphus/evidence/task-18-delete.txt
  ```

  **Commit**: YES
  - Message: `feat: add delete command`
  - Pre-commit: `cargo test --workspace`

---

- [ ] 19. download Command — Attachments

  **What to do**:
  - Add `download_attachments(uid: u32, mailbox: &str, target_dir: &Path, filename: Option<&str>) -> Result<Vec<PathBuf>>` to imap module:
    - Fetch full message (reuse Task 13 fetch_message)
    - Extract attachments from parsed Message
    - If `filename` specified: download only that attachment
    - Save to target_dir with original filename
    - Handle naming conflicts: append `_1`, `_2` etc.
    - Return list of saved file paths
  - Implement `download` command in `crates/mailerboi/src/cmd/download.rs`:
    - `uid: u32` positional
    - `--dir` output directory (default: current directory)
    - `--file` specific attachment filename (default: all)
    - `--mailbox` (default INBOX)
    - Print saved file paths to stdout
  - TDD: test attachment extraction, test naming conflicts, test no-attachment email

  **Must NOT do**:
  - No progress bar (keep it simple)
  - No size limits — download everything

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (but depends on Task 13's fetch logic)
  - **Parallel Group**: Wave 4 (starts after Task 13 if it lands first)
  - **Blocks**: Task 21
  - **Blocked By**: Task 13 (fetch_message function)

  **References**:
  - Task 13: fetch_message + mail-parser parsing — reuse for attachment extraction
  - `mail-parser` attachments: `message.attachment(n)` → Attachment with `.contents()`, `.content_type()`, `.attachment_name()`
  - imap-smtp-email download pattern: --dir, --file options

  **Acceptance Criteria**:
  - [ ] `cargo run -p mailerboi -- download 1 --account test` saves all attachments to cwd
  - [ ] `--dir /tmp/mail` saves to specified directory
  - [ ] `--file report.pdf` downloads only that attachment
  - [ ] Email without attachments shows "No attachments"
  - [ ] Naming conflict handled (doesn't overwrite)

  **QA Scenarios**:
  ```
  Scenario: Download attachment
    Tool: Bash
    Preconditions: GreenMail with email containing attachment
    Steps:
      1. Run `cargo run -p mailerboi -- download 1 --dir /tmp/test-dl --account test`
      2. Assert file exists in /tmp/test-dl/
      3. Assert file size > 0
    Expected Result: Attachment saved to disk with correct filename
    Evidence: .sisyphus/evidence/task-19-download.txt
  ```

  **Commit**: YES
  - Message: `feat: add download command — attachments`
  - Pre-commit: `cargo test --workspace`

---

- [ ] 20. draft Command — IMAP APPEND

  **What to do**:
  - Add `create_draft(account: &AccountConfig, subject: &str, body: &str) -> Result<u32>` to imap module:
    - Build minimal RFC822 message:
      ```
      From: <account.email>
      Subject: <subject>
      Date: <now>
      Content-Type: text/plain; charset=utf-8
      
      <body>
      ```
    - Use IMAP APPEND to Drafts folder with \Draft flag
    - Determine Drafts folder name: try "Drafts", "INBOX.Drafts", or from config
    - Return UID of created draft
  - Implement `draft` command in `crates/mailerboi/src/cmd/draft.rs`:
    - `--subject` required
    - `--body` or `--body-file` (one required)
    - `--mailbox` override drafts folder name
  - TDD: test RFC822 message building, test APPEND against GreenMail

  **Must NOT do**:
  - No HTML body support — plain text only for drafts
  - No attachments on drafts
  - No MIME multipart — single text/plain part

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: Task 21
  - **Blocked By**: Tasks 7, 9

  **References**:
  - `async-imap` APPEND: `session.append(mailbox, content)` with flags
  - RFC 822 message format: headers + blank line + body
  - IMAP APPEND spec: RFC 3501 §6.3.11

  **Acceptance Criteria**:
  - [ ] `cargo run -p mailerboi -- draft --subject "Test" --body "WIP" --account test` creates draft
  - [ ] Draft appears in Drafts folder with \Draft flag
  - [ ] `--body-file` reads body from file
  - [ ] Subject and body support UTF-8

  **QA Scenarios**:
  ```
  Scenario: Create draft
    Tool: Bash
    Preconditions: GreenMail running
    Steps:
      1. Run `cargo run -p mailerboi -- draft --subject "My Draft" --body "Work in progress" --account test`
      2. Run `cargo run -p mailerboi -- list --mailbox Drafts --account test`
      3. Assert draft appears with subject "My Draft"
    Expected Result: Draft created and visible in Drafts folder
    Evidence: .sisyphus/evidence/task-20-draft.txt
  ```

  **Commit**: YES
  - Message: `feat: add draft command — IMAP APPEND`
  - Pre-commit: `cargo test --workspace`

- [ ] 21. CLI Integration Tests

  **What to do**:
  - Create `crates/mailerboi/tests/integration.rs`:
    - Use `assert_cmd` crate to test CLI binary
    - Test each subcommand against GreenMail with real IMAP operations
    - Test multi-account scenarios
    - Test all 3 output formats (--output table/json/toon)
    - Test error cases: wrong password, unreachable server, non-existent mailbox
    - Test exit codes: 0 for success, 1 for runtime error, 2 for config error
  - Add `assert_cmd` and `predicates` to dev-dependencies
  - Mark all integration tests with `#[ignore]` (require GreenMail running)
  - Create test config files in `tests/fixtures/`
  - End-to-end workflow test:
    1. doctor → check → list → read → flag → move → delete → draft

  **Must NOT do**:
  - Don't test against real email servers
  - Don't test internals — only CLI interface

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []
  - **Reason**: Complex integration test setup, GreenMail coordination

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 5 (with Tasks 22, 23)
  - **Blocks**: Task 23
  - **Blocked By**: Tasks 10-20 (all commands must be implemented)

  **References**:
  - `assert_cmd` docs: https://docs.rs/assert_cmd — CLI binary testing
  - `predicates` docs: https://docs.rs/predicates — assertion helpers
  - All Task 10-20 command implementations

  **Acceptance Criteria**:
  - [ ] `cargo test -p mailerboi -- --ignored` runs all integration tests
  - [ ] Each subcommand has at least 1 happy path + 1 error path test
  - [ ] All 3 output formats tested
  - [ ] Exit code tests for success/error/config-error
  - [ ] End-to-end workflow test passes

  **QA Scenarios**:
  ```
  Scenario: Full integration test suite
    Tool: Bash
    Preconditions: GreenMail running
    Steps:
      1. Run `cargo test -p mailerboi -- --ignored`
      2. Assert all tests pass
      3. Check test count: minimum 24 tests (12 commands × 2 scenarios each)
    Expected Result: All integration tests pass
    Evidence: .sisyphus/evidence/task-21-integration.txt
  ```

  **Commit**: YES
  - Message: `test: add CLI integration tests`
  - Pre-commit: `cargo test --workspace`

---

- [ ] 22. Example Configs + devenv Polish

  **What to do**:
  - Create `examples/config.toon` — example TOON config with 2 accounts (personal + work), commented explanations
  - Create `examples/credentials.toml` — example credentials file with placeholder passwords
  - Update `devenv.nix`:
    - Add `pkgs.docker` or `pkgs.docker-compose` to packages (for GreenMail)
    - Add script: `scripts.greenmail.exec = "docker run -d --name greenmail ..."` for easy test server startup
    - Add script: `scripts.test.exec = "cargo test --workspace"`, `scripts.test-all.exec = "cargo test --workspace -- --include-ignored"`
  - Update `.gitignore`: ensure `credentials.toml` patterns for user configs are excluded, but `examples/` are included

  **Must NOT do**:
  - Don't include real credentials in examples
  - Don't create a README (not requested)

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 5 (with Tasks 21, 23)
  - **Blocks**: Task 23
  - **Blocked By**: Task 6 (config module — to match actual config structure)

  **References**:
  - Task 6: AppConfig, AccountConfig struct definitions — examples must match
  - Himalaya config.sample.toml — inspiration for example layout
  - `/home/valerius/code/mailerboi/devenv.nix` — current devenv to extend

  **Acceptance Criteria**:
  - [ ] `examples/config.toon` is valid TOON parseable by mailerboi
  - [ ] `examples/credentials.toml` is valid TOML
  - [ ] devenv scripts: `greenmail` starts Docker container, `test` runs cargo test
  - [ ] No real credentials in any committed file

  **QA Scenarios**:
  ```
  Scenario: Example config parses
    Tool: Bash
    Steps:
      1. Run `cargo test -p mailerboi-core config::tests::parse_example_config`
      2. Assert example config parses without error
    Expected Result: Example config matches AppConfig struct
    Evidence: .sisyphus/evidence/task-22-example-config.txt
  ```

  **Commit**: YES
  - Message: `chore: add example configs + polish devenv`
  - Pre-commit: `cargo check --workspace`

---

- [ ] 23. Final clippy/fmt/test Cleanup

  **What to do**:
  - Run `cargo clippy --workspace -- -D warnings` and fix ALL warnings
  - Run `cargo fmt --all` and verify formatting
  - Run `cargo test --workspace` and fix any failures
  - Run `cargo test --workspace -- --ignored` (with GreenMail) and fix integration failures
  - Review all `#[allow(...)]` attributes — remove if unnecessary
  - Verify no `unwrap()` in library code (grep for it)
  - Verify no `println!()` in library code
  - Verify all public items have at least a one-line doc comment
  - Update module re-exports in lib.rs for clean public API

  **Must NOT do**:
  - Don't add new features
  - Don't refactor working code for style only

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO (must be last before final verification)
  - **Parallel Group**: Sequential after Tasks 21, 22
  - **Blocks**: F1-F4
  - **Blocked By**: Tasks 21, 22

  **References**:
  - All source files in workspace
  - Guardrails from "Must NOT Have" section

  **Acceptance Criteria**:
  - [ ] `cargo clippy --workspace -- -D warnings` exits 0
  - [ ] `cargo fmt --check` exits 0
  - [ ] `cargo test --workspace` all pass
  - [ ] `grep -r "unwrap()" crates/mailerboi-core/src/` returns 0 results (except tests)
  - [ ] `grep -r "println!" crates/mailerboi-core/src/` returns 0 results (except tests)

  **QA Scenarios**:
  ```
  Scenario: Full quality gate
    Tool: Bash
    Steps:
      1. Run `cargo clippy --workspace -- -D warnings` — assert exit 0
      2. Run `cargo fmt --check` — assert exit 0
      3. Run `cargo test --workspace` — assert exit 0
      4. Run `grep -rn "unwrap()" crates/mailerboi-core/src/ --include="*.rs" | grep -v "#\[cfg(test)\]" | grep -v "mod tests"` — assert empty
      5. Run `grep -rn "println!" crates/mailerboi-core/src/ --include="*.rs" | grep -v "#\[cfg(test)\]"` — assert empty
    Expected Result: All quality checks pass, no forbidden patterns
    Evidence: .sisyphus/evidence/task-23-quality-gate.txt
  ```

  **Commit**: YES
  - Message: `chore: final clippy/fmt/test cleanup`
  - Pre-commit: `cargo test --workspace && cargo clippy --workspace -- -D warnings`

---

## Final Verification Wave

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.

- [ ] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, run `cargo run -p mailerboi -- <cmd>`). For each "Must NOT Have": search codebase for forbidden patterns (unwrap in lib, println in lib, trait objects, OAuth2 code) — reject with file:line if found. Check evidence files exist in .sisyphus/evidence/. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo clippy --workspace -- -D warnings` + `cargo fmt --check` + `cargo test --workspace`. Review all changed files for: `as any`-equivalent patterns, empty error handling, leftover debug prints, commented-out code, unused imports. Check AI slop: excessive comments, over-abstraction, generic names.
  Output: `Build [PASS/FAIL] | Clippy [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [ ] F3. **Real Manual QA** — `unspecified-high`
  Start GreenMail Docker. Execute EVERY subcommand against it: list-accounts, doctor, check, folders, list, read, search, move, delete, flag, download, draft. Test all 3 output formats. Test multi-account. Test error cases (wrong password, unreachable server). Save evidence to `.sisyphus/evidence/final-qa/`.
  Output: `Commands [12/12 pass] | Output Formats [3/3] | Error Handling [N/N] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff (git log/diff). Verify 1:1 — everything in spec was built, nothing beyond spec was built. Check "Must NOT do" compliance. Detect cross-task contamination. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

| Task | Commit Message | Files | Pre-commit |
|------|---------------|-------|------------|
| 1 | `chore: initialize workspace with cargo, git, devenv` | Cargo.toml, crates/*/Cargo.toml, .gitignore, devenv.nix | `cargo check --workspace` |
| 2 | `spike: validate toon-format custom struct serde roundtrip` | spike test files | `cargo test --workspace` |
| 3 | `spike: validate async-imap + TLS connection to GreenMail` | spike test files | `cargo test --workspace` |
| 4 | `feat: add domain types — Folder, Envelope, Flag, Message` | mailerboi-core/src/domain/ | `cargo test --workspace` |
| 5 | `feat: add error types with thiserror` | mailerboi-core/src/error.rs | `cargo test --workspace` |
| 6 | `feat: add TOON config parsing + credentials.toml` | mailerboi-core/src/config/ | `cargo test --workspace` |
| 7 | `feat: add IMAP connection manager with TLS` | mailerboi-core/src/imap/ | `cargo test --workspace` |
| 8 | `feat: add output formatting — table, TOON, JSON` | mailerboi-core/src/output.rs | `cargo test --workspace` |
| 9 | `feat: add CLI skeleton with clap subcommands` | mailerboi/src/ | `cargo test --workspace` |
| 10 | `feat: add list-accounts + doctor commands` | mailerboi/src/cmd/ | `cargo test --workspace` |
| 11 | `feat: add folders command` | mailerboi/src/cmd/ | `cargo test --workspace` |
| 12 | `feat: add list command — envelope listing` | mailerboi/src/cmd/ | `cargo test --workspace` |
| 13 | `feat: add read command — email parsing + display` | mailerboi/src/cmd/ | `cargo test --workspace` |
| 14 | `feat: add search command with IMAP filters` | mailerboi/src/cmd/ | `cargo test --workspace` |
| 15 | `feat: add check command — unread count` | mailerboi/src/cmd/ | `cargo test --workspace` |
| 16 | `feat: add flag command — mark read/unread` | mailerboi/src/cmd/ | `cargo test --workspace` |
| 17 | `feat: add move command with MOVE/COPY fallback` | mailerboi/src/cmd/ | `cargo test --workspace` |
| 18 | `feat: add delete command` | mailerboi/src/cmd/ | `cargo test --workspace` |
| 19 | `feat: add download command — attachments` | mailerboi/src/cmd/ | `cargo test --workspace` |
| 20 | `feat: add draft command — IMAP APPEND` | mailerboi/src/cmd/ | `cargo test --workspace` |
| 21 | `test: add CLI integration tests` | mailerboi/tests/ | `cargo test --workspace` |
| 22 | `chore: add example configs + polish devenv` | examples/, devenv.nix | `cargo check --workspace` |
| 23 | `chore: final clippy/fmt/test cleanup` | various | `cargo test --workspace && cargo clippy --workspace -- -D warnings` |

---

## Success Criteria

### Verification Commands
```bash
cargo test --workspace                              # All tests pass
cargo clippy --workspace -- -D warnings              # No warnings
cargo fmt --check                                    # Formatted
cargo run -p mailerboi -- --help                     # Shows all subcommands
cargo run -p mailerboi -- list-accounts              # Lists configured accounts
cargo run -p mailerboi -- doctor --account test      # Tests IMAP connectivity
cargo run -p mailerboi -- check                      # Shows unread counts
cargo run -p mailerboi -- folders --account test     # Lists mailboxes
cargo run -p mailerboi -- list --account test        # Lists emails in INBOX
cargo run -p mailerboi -- read --account test 1      # Reads email UID 1
cargo run -p mailerboi -- search --account test --from alice  # Searches
cargo run -p mailerboi -- flag --account test 1 --set seen    # Marks read
cargo run -p mailerboi -- move --account test 1 Trash         # Moves to Trash
cargo run -p mailerboi -- delete --account test 1             # Deletes
cargo run -p mailerboi -- download --account test 1           # Downloads attachments
cargo run -p mailerboi -- draft --account test --subject "Draft" --body "WIP"  # Creates draft
```

### Final Checklist
- [ ] All "Must Have" present
- [ ] All "Must NOT Have" absent
- [ ] All 12 subcommands functional
- [ ] All tests pass
- [ ] 3 output formats working (table, toon, json)
- [ ] Multi-account config working
- [ ] GreenMail integration tests pass
