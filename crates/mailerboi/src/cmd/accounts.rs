use std::path::PathBuf;

use anyhow::{Context, Result};
use mailerboi_core::config::{config_path, load_config};
use mailerboi_core::output::{format_accounts, OutputFormat};

pub async fn run(config_path_override: Option<PathBuf>, output: &OutputFormat) -> Result<()> {
    let path = config_path_override.unwrap_or_else(config_path);
    let config = load_config(&path)
        .with_context(|| format!("Failed to load config from {}", path.display()))?;

    let accounts: Vec<(&str, &mailerboi_core::config::AccountConfig)> = config
        .accounts
        .iter()
        .map(|(key, value)| (key.as_str(), value))
        .collect();

    println!("{}", format_accounts(&accounts, output));
    Ok(())
}
