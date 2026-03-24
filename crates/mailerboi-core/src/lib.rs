//! # mailerboi-core
//!
//! Core library for the `mailerboi` CLI email client.
//!
//! This crate provides the building blocks for managing multiple IMAP email accounts:
//!
//! - [`config`] parses TOON config files and TOML credentials.
//! - [`domain`] defines serializable email data types.
//! - [`error`] exposes structured error types and the crate [`Result`] alias.
//! - [`imap`] manages IMAP connections, mailbox operations, and message retrieval.
//! - [`output`] renders CLI output as tables, JSON, or TOON.
//!
//! # Config Format
//!
//! `mailerboi` uses [TOON format](https://github.com/toon-format/toon) for configuration:
//!
//! ```text
//! accounts.personal:
//!   email: alice@example.com
//!   host: imap.example.com
//!   port: 993
//!   tls: true
//!   default: true
//! ```
//!
//! Credentials are stored separately in `credentials.toml`:
//!
//! ```text
//! personal = "app-password-here"
//! ```
//!
//! # Usage
//!
//! ```no_run
//! use mailerboi_core::config::{load_config_default, resolve_account};
//!
//! let config = load_config_default()?;
//! let (_name, account) = resolve_account(&config, None)?;
//! println!("connecting to {}", account.host);
//! # Ok::<(), mailerboi_core::MailerboiError>(())
//! ```

/// Configuration loading and account resolution.
pub mod config;
/// Email domain types used across the crate.
pub mod domain;
/// Structured error types and the crate [`Result`] alias.
pub mod error;
/// IMAP connection management and mailbox operations.
pub mod imap;
/// Output formatting for CLI commands.
pub mod output;

mod spike_imap;
mod spike_toon;

pub use error::{ConfigError, ImapError, MailerboiError, Result};
pub use output::OutputFormat;
