use std::path::PathBuf;

use anyhow::{Context, Result};
use mailerboi_core::config::{
    config_path, credentials_path, load_config, load_credentials, resolve_account,
};
use mailerboi_core::imap::connect;
use mailerboi_core::output::OutputFormat;

fn flag_name_to_imap(name: &str) -> String {
    match name.to_lowercase().as_str() {
        "seen" => "\\Seen".to_string(),
        "flagged" => "\\Flagged".to_string(),
        "answered" => "\\Answered".to_string(),
        "draft" => "\\Draft".to_string(),
        "deleted" => "\\Deleted".to_string(),
        other => other.to_string(),
    }
}

pub struct FlagParams {
    pub config_path_override: Option<PathBuf>,
    pub account_name: Option<String>,
    pub _output: OutputFormat,
    pub insecure: bool,
    pub uids: Vec<u32>,
    pub set: Option<String>,
    pub unset: Option<String>,
    pub read: bool,
    pub unread: bool,
    pub mailbox: String,
}

pub async fn run(params: FlagParams) -> Result<()> {
    let FlagParams {
        config_path_override,
        account_name,
        _output,
        insecure,
        uids,
        set,
        unset,
        read,
        unread,
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

    if read || set.as_deref() == Some("seen") {
        session
            .set_flags(&uids, &mailbox, &["\\Seen".to_string()], true)
            .await
            .context("Failed to set flags")?;
        println!("Marked {} message(s) as read", uids.len());
    } else if unread || unset.as_deref() == Some("seen") {
        session
            .set_flags(&uids, &mailbox, &["\\Seen".to_string()], false)
            .await
            .context("Failed to unset flags")?;
        println!("Marked {} message(s) as unread", uids.len());
    } else if let Some(flag) = set {
        let imap_flag = flag_name_to_imap(&flag);
        session
            .set_flags(&uids, &mailbox, &[imap_flag], true)
            .await
            .context("Failed to set flags")?;
        println!("Set flag '{}' on {} message(s)", flag, uids.len());
    } else if let Some(flag) = unset {
        let imap_flag = flag_name_to_imap(&flag);
        session
            .set_flags(&uids, &mailbox, &[imap_flag], false)
            .await
            .context("Failed to unset flags")?;
        println!("Unset flag '{}' on {} message(s)", flag, uids.len());
    } else {
        anyhow::bail!("Specify --set, --unset, --read, or --unread");
    }

    session.logout().await.ok();
    Ok(())
}
