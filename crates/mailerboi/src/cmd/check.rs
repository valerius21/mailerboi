use std::path::PathBuf;

use anyhow::{Context, Result};
use mailerboi_core::config::{
    config_path, credentials_path, load_config, load_credentials, resolve_account,
};
use mailerboi_core::imap::connect;
use mailerboi_core::output::{format_check, OutputFormat};

pub async fn run(
    config_path_override: Option<PathBuf>,
    account_name: Option<&str>,
    output: &OutputFormat,
    insecure: bool,
    mailbox: &str,
) -> Result<()> {
    let path = config_path_override.unwrap_or_else(config_path);
    let config = load_config(&path)
        .with_context(|| format!("Failed to load config from {}", path.display()))?;
    let creds = load_credentials(&credentials_path()).context("Failed to load credentials")?;

    let mut results = Vec::new();

    if let Some(name) = account_name {
        let (acc_name, account) = resolve_account(&config, Some(name))?;
        let password = creds
            .passwords
            .get(acc_name)
            .ok_or_else(|| anyhow::anyhow!("No password for '{}'", acc_name))?;
        let mut account = account.clone();
        if insecure {
            account.insecure = true;
        }
        let mut session = connect(&account, password)
            .await
            .context("IMAP connection failed")?;
        let mut status = session
            .check_mailbox_status(mailbox)
            .await
            .context("STATUS failed")?;
        status.account = acc_name.to_string();
        session.logout().await.ok();
        results.push(status);
    } else {
        for (acc_name, account) in &config.accounts {
            if let Some(password) = creds.passwords.get(acc_name) {
                let mut account = account.clone();
                if insecure {
                    account.insecure = true;
                }
                if let Ok(mut session) = connect(&account, password).await {
                    if let Ok(mut status) = session.check_mailbox_status(mailbox).await {
                        status.account = acc_name.clone();
                        results.push(status);
                    }
                    session.logout().await.ok();
                }
            }
        }
    }

    println!("{}", format_check(&results, output));
    Ok(())
}
