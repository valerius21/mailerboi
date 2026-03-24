use std::path::PathBuf;

use anyhow::{Context, Result};
use mailerboi_core::config::{
    config_path, credentials_path, load_config, load_credentials, resolve_account,
};
use mailerboi_core::imap::{connect, SearchQuery};
use mailerboi_core::output::{format_envelopes, OutputFormat};

pub async fn run(
    config_path_override: Option<PathBuf>,
    account_name: Option<&str>,
    output: &OutputFormat,
    insecure: bool,
    unseen: bool,
    seen: bool,
    from: Option<String>,
    subject: Option<String>,
    since: Option<String>,
    before: Option<String>,
    limit: u32,
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
        .search_messages(mailbox, &query)
        .await
        .context("Search failed")?;
    session.logout().await.ok();

    println!("{}", format_envelopes(&envelopes, output));
    Ok(())
}
