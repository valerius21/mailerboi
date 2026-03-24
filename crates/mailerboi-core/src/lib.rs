// mailerboi-core library

pub mod config;
pub mod domain;
pub mod error;
pub mod imap;
pub mod output;

mod spike_imap;
mod spike_toon;

pub use error::{ConfigError, ImapError, MailerboiError, Result};
pub use output::OutputFormat;
