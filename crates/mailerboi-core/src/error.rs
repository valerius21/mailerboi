//! Error types for the `mailerboi-core` library.
//!
//! Uses [`thiserror`] for structured, displayable errors.
//! The [`Result`] type alias uses [`MailerboiError`] as the error type.

use std::path::PathBuf;
use thiserror::Error;

/// Convenient result type for fallible crate operations.
pub type Result<T> = std::result::Result<T, MailerboiError>;

/// Top-level error type for crate consumers.
#[derive(Error, Debug)]
pub enum MailerboiError {
    /// Configuration loading or resolution failed.
    #[error("Config error: {0}")]
    Config(#[from] ConfigError),
    /// IMAP communication or mailbox operations failed.
    #[error("IMAP error: {0}")]
    Imap(#[from] ImapError),
    /// Local filesystem I/O failed.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors related to config and credential handling.
#[derive(Error, Debug)]
pub enum ConfigError {
    /// The main config file could not be found.
    #[error("Config file not found: {path}")]
    NotFound { path: PathBuf },
    /// A config or credentials file could not be parsed.
    #[error("Failed to parse config: {0}")]
    Parse(String),
    /// The credentials file could not be found.
    #[error("Credentials file not found: {path}")]
    CredentialsNotFound { path: PathBuf },
    /// The requested account name does not exist in the config.
    #[error("Account '{name}' not found in config")]
    AccountNotFound { name: String },
}

/// Errors returned by IMAP connection and mailbox operations.
#[derive(Error, Debug)]
pub enum ImapError {
    /// The server could not be reached.
    #[error("Connection failed to {host}:{port}: {reason}")]
    ConnectionFailed {
        host: String,
        port: u16,
        reason: String,
    },
    /// Server login failed for the supplied user.
    #[error("Authentication failed for {user}")]
    AuthFailed { user: String },
    /// The requested mailbox does not exist.
    #[error("Mailbox '{name}' not found")]
    MailboxNotFound { name: String },
    /// The requested message UID was not found.
    #[error("Message UID {uid} not found")]
    MessageNotFound { uid: u32 },
    /// The IMAP library reported a protocol-level failure.
    #[error("IMAP protocol error: {0}")]
    Protocol(String),
    /// TLS negotiation or certificate validation failed.
    #[error("TLS error: {0}")]
    Tls(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_error_not_found_display() {
        let e = ConfigError::NotFound {
            path: PathBuf::from("/etc/mailerboi/config.toon"),
        };
        assert_eq!(
            format!("{}", e),
            "Config file not found: /etc/mailerboi/config.toon"
        );
    }

    #[test]
    fn config_error_account_not_found_display() {
        let e = ConfigError::AccountNotFound {
            name: "work".to_string(),
        };
        assert_eq!(format!("{}", e), "Account 'work' not found in config");
    }

    #[test]
    fn imap_error_connection_failed_display() {
        let e = ImapError::ConnectionFailed {
            host: "imap.example.com".to_string(),
            port: 993,
            reason: "connection refused".to_string(),
        };
        assert_eq!(
            format!("{}", e),
            "Connection failed to imap.example.com:993: connection refused"
        );
    }

    #[test]
    fn imap_error_auth_failed_display() {
        let e = ImapError::AuthFailed {
            user: "bob@example.com".to_string(),
        };
        assert_eq!(
            format!("{}", e),
            "Authentication failed for bob@example.com"
        );
    }

    #[test]
    fn mailerboierror_wraps_config() {
        let config_err = ConfigError::Parse("bad syntax at line 3".to_string());
        let e: MailerboiError = config_err.into();
        assert!(format!("{}", e).contains("bad syntax at line 3"));
    }

    #[test]
    fn result_type_alias() {
        fn returns_result() -> Result<i32> {
            Ok(42)
        }
        assert_eq!(returns_result().unwrap(), 42);
    }
}
