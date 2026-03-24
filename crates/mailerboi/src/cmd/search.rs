use std::path::PathBuf;

use anyhow::{Context, Result};
use mailerboi_core::config::{
    config_path, credentials_path, load_config, load_credentials, resolve_account,
};
use mailerboi_core::imap::{connect, SearchQuery};
use mailerboi_core::output::{format_envelopes, OutputFormat};

pub struct SearchParams {
    pub config_path_override: Option<PathBuf>,
    pub account_name: Option<String>,
    pub output: OutputFormat,
    pub insecure: bool,
    pub unseen: bool,
    pub seen: bool,
    pub from: Option<String>,
    pub subject: Option<String>,
    pub since: Option<String>,
    pub before: Option<String>,
    pub limit: u32,
    pub mailbox: String,
}

pub async fn run(params: SearchParams) -> Result<()> {
    let SearchParams {
        config_path_override,
        account_name,
        output,
        insecure,
        unseen,
        seen,
        from,
        subject,
        since,
        before,
        limit,
        mailbox,
    } = params;
    let path = config_path_override.unwrap_or_else(config_path);
    let config = load_config(&path)
        .with_context(|| format!("Failed to load config from {}", path.display()))?;
    let creds = load_credentials(&credentials_path()).context("Failed to load credentials")?;
    let (name, account) = resolve_account(&config, account_name.as_deref())?;
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
    let query = SearchQuery {
        unseen,
        seen,
        from,
        subject,
        since,
        before,
        limit,
    };
    let envelopes = session
        .search_messages(&mailbox, &query)
        .await
        .context("Search failed")?;
    session.logout().await.ok();

    println!("{}", format_envelopes(&envelopes, &output));
    Ok(())
}
