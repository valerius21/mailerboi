use std::path::PathBuf;

use anyhow::{Context, Result};
use mailerboi_core::config::{
    config_path, credentials_path, load_config, load_credentials, resolve_account,
};
use mailerboi_core::imap::doctor;
use mailerboi_core::output::{format_doctor, OutputFormat};

pub async fn run(
    config_path_override: Option<PathBuf>,
    account_name: Option<&str>,
    output: &OutputFormat,
    insecure: bool,
) -> Result<()> {
    let path = config_path_override.unwrap_or_else(config_path);
    let config =
        load_config(&path).with_context(|| format!("Failed to load config from {}", path.display()))?;

    let creds_path = credentials_path();
    let creds = load_credentials(&creds_path)
        .with_context(|| format!("Failed to load credentials from {}", creds_path.display()))?;

    let (name, account) = resolve_account(&config, account_name)?;
    let password = creds
        .passwords
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("No password found for account '{}'", name))?;

    let mut account = account.clone();
    if insecure {
        account.insecure = true;
    }

    let report = doctor(&account, password).await;
    println!("{}", format_doctor(&report, output));

    if !report.all_ok() {
        std::process::exit(1);
    }

    Ok(())
}
