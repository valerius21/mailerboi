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
    subject: &str,
    body: Option<String>,
    body_file: Option<PathBuf>,
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

    let body_text = if let Some(b) = body {
        b
    } else if let Some(f) = body_file {
        std::fs::read_to_string(&f)
            .with_context(|| format!("Failed to read body file {}", f.display()))?
    } else {
        anyhow::bail!("Provide --body or --body-file");
    };

    let mut session = connect(&account, password)
        .await
        .context("IMAP connection failed")?;
    session
        .create_draft(&account.email, subject, &body_text, mailbox)
        .await
        .context("Failed to create draft")?;
    session.logout().await.ok();

    println!("Draft created in {}", mailbox);
    Ok(())
}
