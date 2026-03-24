// mailerboi-core library

pub mod domain;
pub mod error;
pub mod config;
pub mod imap;
pub mod output;

mod spike_toon;
mod spike_imap;

pub use error::{MailerboiError, ConfigError, ImapError, Result};
pub use output::OutputFormat;
