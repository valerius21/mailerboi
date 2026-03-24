use std::path::PathBuf;

use anyhow::{Context, Result};
use mailerboi_core::config::{
    config_path, credentials_path, load_config, load_credentials, resolve_account,
};
use mailerboi_core::imap::connect;
use mailerboi_core::output::OutputFormat;

pub async fn run(
    config_path_override: Option<PathBuf>,
    account_name: Option<&str>,
    _output: &OutputFormat,
    insecure: bool,
    uid: u32,
    dir: Option<PathBuf>,
    file: Option<String>,
    mailbox: &str,
) -> Result<()> {
    let path = config_path_override.unwrap_or_else(config_path);
    let config =
        load_config(&path).with_context(|| format!("Failed to load config from {}", path.display()))?;
    let creds = load_credentials(&credentials_path()).context("Failed to load credentials")?;
    let (name, account) = resolve_account(&config, account_name)?;
    let password = creds
        .passwords
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("No password for '{}'", name))?;

    let mut account = account.clone();
    if insecure {
        account.insecure = true;
    }
    let mut session = connect(&account, password)
        .await
        .context("IMAP connection failed")?;

    let target_dir = dir.unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&target_dir).context("Failed to create output directory")?;

    let saved = session
        .download_attachments(uid, mailbox, &target_dir, file.as_deref())
        .await
        .context("Failed to download attachments")?;
    session.logout().await.ok();

    if saved.is_empty() {
        println!("No attachments found.");
    } else {
        for path in &saved {
            println!("Saved: {}", path.display());
        }
    }
    Ok(())
}
