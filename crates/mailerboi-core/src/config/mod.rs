//! Configuration loading and account management.
//!
//! Supports TOON format config files and TOML credentials files.
//! Default paths follow XDG conventions (`~/.config/mailerboi/`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer, Serialize};
use tracing::warn;

use crate::error::{ConfigError, Result};

/// Parsed application configuration.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AppConfig {
    /// Accounts keyed by their configured account name.
    pub accounts: HashMap<String, AccountConfig>,
}

#[derive(Debug, Deserialize)]
struct AppConfigRepr {
    #[serde(default)]
    accounts: HashMap<String, AccountConfig>,
    #[serde(flatten)]
    dotted_accounts: HashMap<String, AccountConfig>,
}

impl<'de> Deserialize<'de> for AppConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let repr = AppConfigRepr::deserialize(deserializer)?;
        let mut accounts = repr.accounts;

        for (key, value) in repr.dotted_accounts {
            if let Some(name) = key.strip_prefix("accounts.") {
                accounts.insert(name.to_string(), value);
            }
        }

        Ok(Self { accounts })
    }
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

/// Loads an [`AppConfig`] from a TOON file.
///
/// Returns [`crate::error::ConfigError::NotFound`] when `path` does not exist and
/// [`crate::error::ConfigError::Parse`] when the file cannot be read or decoded.
pub fn load_config(path: &Path) -> Result<AppConfig> {
    if !path.exists() {
        return Err(ConfigError::NotFound {
            path: path.to_path_buf(),
        }
        .into());
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| ConfigError::Parse(format!("Failed to read {}: {}", path.display(), e)))?;

    toon_format::decode_default(&content)
        .map_err(|e| ConfigError::Parse(format!("TOON parse error: {}", e)).into())
}

/// Loads the default config file from [`config_path`].
pub fn load_config_default() -> Result<AppConfig> {
    let path = config_path();
    load_config(&path)
}

/// Returns the config file path, honoring `MAILERBOI_CONFIG` first.
///
/// ```text
/// ~/.config/mailerboi/config.toon
/// ```
pub fn config_path() -> PathBuf {
    if let Ok(p) = std::env::var("MAILERBOI_CONFIG") {
        return PathBuf::from(p);
    }

    dirs::config_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
        .unwrap_or_else(|| PathBuf::from("/etc"))
        .join("mailerboi")
        .join("config.toon")
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

    use tempfile::NamedTempFile;

    use super::*;

    fn sample_toon_config() -> &'static str {
        r#"accounts.personal:
  email: alice@example.com
  host: imap.example.com
  port: 993
  tls: true
  default: true
accounts.work:
  email: alice@company.com
  host: imap.company.com
  port: 993
  tls: true
"#
    }

    fn sample_credentials_toml() -> &'static str {
        r#"personal = "secret123"
work = "workpass456"
"#
    }

    #[test]
    fn parse_multi_account_config() {
        let config: AppConfig = toon_format::decode_default(sample_toon_config()).unwrap();
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
        let config: AppConfig = toon_format::decode_default(sample_toon_config()).unwrap();
        let (name, acc) = resolve_account(&config, Some("work")).unwrap();
        assert_eq!(name, "work");
        assert_eq!(acc.email, "alice@company.com");
    }

    #[test]
    fn resolve_account_default() {
        let config: AppConfig = toon_format::decode_default(sample_toon_config()).unwrap();
        let (name, acc) = resolve_account(&config, None).unwrap();
        assert_eq!(name, "personal");
        assert_eq!(acc.email, "alice@example.com");
    }

    #[test]
    fn resolve_account_not_found() {
        let config: AppConfig = toon_format::decode_default(sample_toon_config()).unwrap();
        let result = resolve_account(&config, Some("nonexistent"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("nonexistent"));
    }

    #[test]
    fn load_config_missing_file() {
        let result = load_config(Path::new("/nonexistent/path/config.toon"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found") || err.to_string().contains("Config file"));
    }

    #[test]
    fn load_config_from_file() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", sample_toon_config()).unwrap();
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
        let minimal = r#"accounts.test:
  email: test@example.com
  host: imap.example.com
"#;
        let config: AppConfig = toon_format::decode_default(minimal).unwrap();
        let acc = config.accounts.get("test").unwrap();
        assert_eq!(acc.port, 993);
        assert!(acc.tls);
        assert_eq!(acc.default_mailbox, "INBOX");
    }
}
