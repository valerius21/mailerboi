use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, MailerboiError>;

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
    ConnectionFailed {
        host: String,
        port: u16,
        reason: String,
    },
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
