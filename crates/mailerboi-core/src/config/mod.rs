//! Configuration loading and account management.
//!
//! Supports TOML config files and TOML credentials files.
//! Default paths follow XDG conventions (`~/.config/mailerboi/`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::error::{ConfigError, Result};

/// Parsed application configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    /// Accounts keyed by their configured account name.
    pub accounts: HashMap<String, AccountConfig>,
}

/// Connection settings for one configured IMAP account.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccountConfig {
    /// Login email address used for IMAP authentication.
    pub email: String,
    #[serde(default)]
    /// Optional display name used when composing messages.
    pub display_name: Option<String>,
    /// IMAP server hostname.
    pub host: String,
    #[serde(default = "default_port")]
    /// IMAP server port.
    pub port: u16,
    #[serde(default = "default_tls")]
    /// Enables implicit TLS, typically on port `993`.
    pub tls: bool,
    #[serde(default)]
    /// Requests STARTTLS when supported by the server.
    pub starttls: bool,
    #[serde(default)]
    /// Skips certificate and hostname validation for TLS connections.
    pub insecure: bool,
    #[serde(default = "default_mailbox")]
    /// Mailbox used when a command does not specify one.
    pub default_mailbox: String,
    #[serde(default)]
    /// Marks this account as the preferred default.
    pub default: bool,
}

fn default_port() -> u16 {
    993
}

fn default_tls() -> bool {
    true
}

fn default_mailbox() -> String {
    "INBOX".to_string()
}

/// Passwords loaded from `credentials.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct Credentials {
    #[serde(flatten)]
    /// Account passwords keyed by account name.
    pub passwords: HashMap<String, String>,
}

/// Loads an [`AppConfig`] from a TOML file.
///
/// Returns [`crate::error::ConfigError::NotFound`] when `path` does not exist and
/// [`crate::error::ConfigError::Parse`] when the file cannot be read or parsed.
pub fn load_config(path: &Path) -> Result<AppConfig> {
    if !path.exists() {
        return Err(ConfigError::NotFound {
            path: path.to_path_buf(),
        }
        .into());
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| ConfigError::Parse(format!("Failed to read {}: {}", path.display(), e)))?;

    toml::from_str(&content)
        .map_err(|e| ConfigError::Parse(format!("TOML parse error: {}", e)).into())
}

/// Loads the default config file from [`config_path`].
pub fn load_config_default() -> Result<AppConfig> {
    let path = config_path();
    load_config(&path)
}

/// Returns the config file path, honoring `MAILERBOI_CONFIG` first.
///
/// ```text
/// ~/.config/mailerboi/config.toml
/// ```
pub fn config_path() -> PathBuf {
    if let Ok(p) = std::env::var("MAILERBOI_CONFIG") {
        return PathBuf::from(p);
    }

    dirs::config_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
        .unwrap_or_else(|| PathBuf::from("/etc"))
        .join("mailerboi")
        .join("config.toml")
}

/// Loads account passwords from a TOML credentials file.
///
/// Returns [`crate::error::ConfigError::CredentialsNotFound`] when `path` does
/// not exist and [`crate::error::ConfigError::Parse`] when the file cannot be
/// read or parsed.
pub fn load_credentials(path: &Path) -> Result<Credentials> {
    if !path.exists() {
        return Err(ConfigError::CredentialsNotFound {
            path: path.to_path_buf(),
        }
        .into());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        if let Ok(meta) = std::fs::metadata(path) {
            let mode = meta.permissions().mode();
            if mode & 0o004 != 0 {
                warn!(
                    "Credentials file {} is world-readable (mode {:o}). Consider `chmod 600`.",
                    path.display(),
                    mode
                );
            }
        }
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| ConfigError::Parse(format!("Failed to read credentials: {}", e)))?;

    toml::from_str(&content)
        .map_err(|e| ConfigError::Parse(format!("TOML parse error: {}", e)).into())
}

/// Returns the credentials file path, honoring `MAILERBOI_CREDENTIALS` first.
///
/// ```text
/// ~/.config/mailerboi/credentials.toml
/// ```
pub fn credentials_path() -> PathBuf {
    if let Ok(p) = std::env::var("MAILERBOI_CREDENTIALS") {
        return PathBuf::from(p);
    }

    dirs::config_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
        .unwrap_or_else(|| PathBuf::from("/etc"))
        .join("mailerboi")
        .join("credentials.toml")
}

/// Resolves an account by name or falls back to the default account.
///
/// When `name` is `None`, the first account marked as default is preferred,
/// then the first configured account. Returns
/// [`crate::error::ConfigError::AccountNotFound`] if no matching account exists.
pub fn resolve_account<'a>(
    config: &'a AppConfig,
    name: Option<&str>,
) -> Result<(&'a str, &'a AccountConfig)> {
    if let Some(n) = name {
        config
            .accounts
            .get_key_value(n)
            .map(|(k, v)| (k.as_str(), v))
            .ok_or_else(|| {
                ConfigError::AccountNotFound {
                    name: n.to_string(),
                }
                .into()
            })
    } else {
        config
            .accounts
            .iter()
            .find(|(_, v)| v.default)
            .or_else(|| config.accounts.iter().next())
            .map(|(k, v)| (k.as_str(), v))
            .ok_or_else(|| {
                ConfigError::AccountNotFound {
                    name: "(default)".to_string(),
                }
                .into()
            })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::sync::Mutex;

    use tempfile::NamedTempFile;

    use super::*;

    // Serialize tests that mutate environment variables to prevent flakiness
    // when tests run concurrently.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    fn sample_config_toml() -> &'static str {
        r#"[accounts.personal]
email = "alice@example.com"
host = "imap.example.com"
port = 993
tls = true
default = true

[accounts.work]
email = "alice@company.com"
host = "imap.company.com"
port = 993
tls = true
"#
    }

    fn sample_credentials_toml() -> &'static str {
        r#"personal = "secret123"
work = "workpass456"
"#
    }

    #[test]
    fn parse_multi_account_config() {
        let config: AppConfig = toml::from_str(sample_config_toml()).unwrap();
        assert_eq!(config.accounts.len(), 2);
        let personal = config.accounts.get("personal").unwrap();
        assert_eq!(personal.email, "alice@example.com");
        assert_eq!(personal.host, "imap.example.com");
        assert_eq!(personal.port, 993);
        assert!(personal.tls);
        assert!(personal.default);
        let work = config.accounts.get("work").unwrap();
        assert_eq!(work.email, "alice@company.com");
    }

    #[test]
    fn parse_credentials_toml() {
        let creds: Credentials = toml::from_str(sample_credentials_toml()).unwrap();
        assert_eq!(creds.passwords.get("personal").unwrap(), "secret123");
        assert_eq!(creds.passwords.get("work").unwrap(), "workpass456");
    }

    #[test]
    fn resolve_account_by_name() {
        let config: AppConfig = toml::from_str(sample_config_toml()).unwrap();
        let (name, acc) = resolve_account(&config, Some("work")).unwrap();
        assert_eq!(name, "work");
        assert_eq!(acc.email, "alice@company.com");
    }

    #[test]
    fn resolve_account_default() {
        let config: AppConfig = toml::from_str(sample_config_toml()).unwrap();
        let (name, acc) = resolve_account(&config, None).unwrap();
        assert_eq!(name, "personal");
        assert_eq!(acc.email, "alice@example.com");
    }

    #[test]
    fn resolve_account_not_found() {
        let config: AppConfig = toml::from_str(sample_config_toml()).unwrap();
        let result = resolve_account(&config, Some("nonexistent"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("nonexistent"));
    }

    #[test]
    fn load_config_missing_file() {
        let result = load_config(Path::new("/nonexistent/path/config.toml"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found") || err.to_string().contains("Config file"));
    }

    #[test]
    fn load_config_from_file() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", sample_config_toml()).unwrap();
        let config = load_config(f.path()).unwrap();
        assert_eq!(config.accounts.len(), 2);
    }

    #[test]
    fn load_credentials_missing_file() {
        let result = load_credentials(Path::new("/nonexistent/credentials.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn load_credentials_from_file() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", sample_credentials_toml()).unwrap();
        let creds = load_credentials(f.path()).unwrap();
        assert_eq!(creds.passwords.get("personal").unwrap(), "secret123");
    }

    #[test]
    fn default_values_applied() {
        let minimal = r#"[accounts.test]
email = "test@example.com"
host = "imap.example.com"
"#;
        let config: AppConfig = toml::from_str(minimal).unwrap();
        let acc = config.accounts.get("test").unwrap();
        assert_eq!(acc.port, 993);
        assert!(acc.tls);
        assert_eq!(acc.default_mailbox, "INBOX");
    }

    #[test]
    fn config_path_uses_env_var() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("MAILERBOI_CONFIG", "/tmp/test_config.toml");
        let path = config_path();
        std::env::remove_var("MAILERBOI_CONFIG");
        assert_eq!(path, std::path::PathBuf::from("/tmp/test_config.toml"));
    }

    #[test]
    fn config_path_fallback_is_reasonable() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("MAILERBOI_CONFIG");
        let path = config_path();
        assert!(path.to_string_lossy().contains("mailerboi"));
        assert!(path.to_string_lossy().ends_with("config.toml"));
    }

    #[test]
    fn credentials_path_uses_env_var() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("MAILERBOI_CREDENTIALS", "/tmp/test_creds.toml");
        let path = credentials_path();
        std::env::remove_var("MAILERBOI_CREDENTIALS");
        assert_eq!(path, std::path::PathBuf::from("/tmp/test_creds.toml"));
    }

    #[test]
    fn credentials_path_fallback_is_reasonable() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("MAILERBOI_CREDENTIALS");
        let path = credentials_path();
        assert!(path.to_string_lossy().contains("mailerboi"));
        assert!(path.to_string_lossy().ends_with("credentials.toml"));
    }

    #[test]
    fn load_config_default_missing_returns_error() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("MAILERBOI_CONFIG", "/nonexistent/mailerboi/config.toml");
        let result = load_config_default();
        std::env::remove_var("MAILERBOI_CONFIG");
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn load_credentials_world_readable_succeeds() {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "personal = \"secret\"\n").unwrap();
        // Make it world-readable to trigger the warning
        std::fs::set_permissions(f.path(), std::fs::Permissions::from_mode(0o644)).unwrap();
        let creds = load_credentials(f.path()).unwrap();
        assert_eq!(creds.passwords.get("personal").unwrap(), "secret");
    }

    #[test]
    fn resolve_account_no_default_returns_first() {
        // No account has default = true; should return first account found
        let toml = r#"[accounts.alpha]
email = "alpha@example.com"
host = "imap.example.com"
"#;
        let config: AppConfig = toml::from_str(toml).unwrap();
        let (name, acc) = resolve_account(&config, None).unwrap();
        assert_eq!(name, "alpha");
        assert_eq!(acc.email, "alpha@example.com");
    }

    #[test]
    fn resolve_account_empty_config_returns_error() {
        let config = AppConfig {
            accounts: std::collections::HashMap::new(),
        };
        let result = resolve_account(&config, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("default"));
    }
}
