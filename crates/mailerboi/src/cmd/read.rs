use std::path::PathBuf;

use anyhow::{Context, Result};
use mailerboi_core::config::{
    config_path, credentials_path, load_config, load_credentials, resolve_account,
};
use mailerboi_core::imap::connect;
use mailerboi_core::output::OutputFormat;

use crate::cli::ReadFormat;

pub async fn run(
    config_path_override: Option<PathBuf>,
    account_name: Option<&str>,
    output: &OutputFormat,
    insecure: bool,
    uid: u32,
    mailbox: &str,
    format: &ReadFormat,
) -> Result<()> {
    let path = config_path_override.unwrap_or_else(config_path);
    let config = load_config(&path)
        .with_context(|| format!("Failed to load config from {}", path.display()))?;
    let creds = load_credentials(&credentials_path()).context("Failed to load credentials")?;
    let (name, account) = resolve_account(&config, account_name)?;
    let password = creds
        .passwords
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("No password for account '{}'", name))?;
    let mut account = account.clone();
    if insecure {
        account.insecure = true;
    }

    let mut session = connect(&account, password)
        .await
        .context("IMAP connection failed")?;
    let message = session
        .fetch_message(uid, mailbox)
        .await
        .context("Failed to fetch message")?;
    session.logout().await.ok();

    match output {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&message).unwrap_or_default()
            );
        }
        OutputFormat::Toon => {
            println!(
                "{}",
                toon_format::encode_default(&message).unwrap_or_default()
            );
        }
        OutputFormat::Table => match format {
            ReadFormat::Text => {
                let from = message
                    .envelope
                    .from
                    .first()
                    .map(|a| a.to_string())
                    .unwrap_or_default();
                let subject = message
                    .envelope
                    .subject
                    .as_deref()
                    .unwrap_or("(no subject)");
                let date = message.envelope.date.as_deref().unwrap_or("-");
                println!("From: {}\nSubject: {}\nDate: {}\n", from, subject, date);
                let body = message
                    .text_body
                    .as_deref()
                    .or(message.html_body.as_deref())
                    .unwrap_or("(no body)");
                println!("{}", body);
            }
            ReadFormat::Html => {
                println!(
                    "{}",
                    message.html_body.as_deref().unwrap_or("(no HTML body)")
                );
            }
            ReadFormat::Raw => {
                println!("{}", String::from_utf8_lossy(&message.raw));
            }
            ReadFormat::Headers => {
                let from = message
                    .envelope
                    .from
                    .first()
                    .map(|a| a.to_string())
                    .unwrap_or_default();
                let subject = message
                    .envelope
                    .subject
                    .as_deref()
                    .unwrap_or("(no subject)");
                let date = message.envelope.date.as_deref().unwrap_or("-");
                println!("From: {}\nSubject: {}\nDate: {}", from, subject, date);
            }
        },
    }
    Ok(())
}
