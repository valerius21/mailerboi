use comfy_table::{presets::UTF8_FULL_CONDENSED, Table};
use serde::{Deserialize, Serialize};

use crate::config::AccountConfig;
use crate::domain::{Envelope, Folder, Message};
use crate::imap::DoctorReport;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
    Toon,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "table" => Ok(Self::Table),
            "json" => Ok(Self::Json),
            "toon" => Ok(Self::Toon),
            other => Err(format!(
                "Unknown output format: '{}'. Use: table, json, toon",
                other
            )),
        }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Table => write!(f, "table"),
            Self::Json => write!(f, "json"),
            Self::Toon => write!(f, "toon"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MailboxStatus {
    pub account: String,
    pub mailbox: String,
    pub total: u32,
    pub unseen: u32,
    pub recent: u32,
}

pub fn format_folders(folders: &[Folder], format: &OutputFormat) -> String {
    match format {
        OutputFormat::Table => {
            if folders.is_empty() {
                return "No folders found.".to_string();
            }

            let mut table = new_table();
            table.set_header(["Name", "Delimiter", "Attributes"]);

            for folder in folders {
                table.add_row(vec![
                    folder.name.clone(),
                    folder.delimiter.clone().unwrap_or_else(|| "-".to_string()),
                    joined_or_dash(&folder.attributes),
                ]);
            }

            table.to_string()
        }
        OutputFormat::Json => serde_json_string(&folders.to_vec()),
        OutputFormat::Toon => toon_string(&folders.to_vec()),
    }
}

pub fn format_envelopes(envelopes: &[Envelope], format: &OutputFormat) -> String {
    match format {
        OutputFormat::Table => {
            if envelopes.is_empty() {
                return "No messages found.".to_string();
            }

            let mut table = new_table();
            table.set_header(["UID", "From", "Subject", "Date", "Flags"]);

            for envelope in envelopes {
                let from = envelope
                    .from
                    .first()
                    .map(|address| address.email.clone())
                    .unwrap_or_else(|| "-".to_string());
                let subject = truncate(envelope.subject.as_deref().unwrap_or("(no subject)"), 50);
                let date = envelope.date.clone().unwrap_or_else(|| "-".to_string());
                let flags = envelope
                    .flags
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();

                table.add_row(vec![
                    envelope.uid.to_string(),
                    from,
                    subject,
                    date,
                    joined_or_dash(&flags),
                ]);
            }

            table.to_string()
        }
        OutputFormat::Json => serde_json_string(&envelopes.to_vec()),
        OutputFormat::Toon => toon_string(&envelopes.to_vec()),
    }
}

pub fn format_message(message: &Message, format: &OutputFormat) -> String {
    match format {
        OutputFormat::Table => {
            let from = message
                .envelope
                .from
                .first()
                .map(ToString::to_string)
                .unwrap_or_default();
            let subject = message
                .envelope
                .subject
                .as_deref()
                .unwrap_or("(no subject)");
            let date = message.envelope.date.as_deref().unwrap_or("-");
            let body = message
                .text_body
                .as_deref()
                .or(message.html_body.as_deref())
                .unwrap_or("(no body)");

            format!(
                "From: {}\nSubject: {}\nDate: {}\n\n{}",
                from, subject, date, body
            )
        }
        OutputFormat::Json => serde_json_string(message),
        OutputFormat::Toon => toon_string(message),
    }
}

pub fn format_check(checks: &[MailboxStatus], format: &OutputFormat) -> String {
    match format {
        OutputFormat::Table => {
            if checks.is_empty() {
                return "No accounts checked.".to_string();
            }

            let mut table = new_table();
            table.set_header(["Account", "Mailbox", "Total", "Unseen", "Recent"]);

            for check in checks {
                table.add_row(vec![
                    check.account.clone(),
                    check.mailbox.clone(),
                    check.total.to_string(),
                    check.unseen.to_string(),
                    check.recent.to_string(),
                ]);
            }

            table.to_string()
        }
        OutputFormat::Json => serde_json_string(&checks.to_vec()),
        OutputFormat::Toon => toon_string(&checks.to_vec()),
    }
}

pub fn format_accounts(accounts: &[(&str, &AccountConfig)], format: &OutputFormat) -> String {
    #[derive(Serialize)]
    struct AccountRow<'a> {
        name: &'a str,
        email: &'a str,
        host: &'a str,
        port: u16,
        tls: bool,
        default: bool,
    }

    let rows = accounts
        .iter()
        .map(|(name, account)| AccountRow {
            name,
            email: &account.email,
            host: &account.host,
            port: account.port,
            tls: account.tls,
            default: account.default,
        })
        .collect::<Vec<_>>();

    match format {
        OutputFormat::Table => {
            if rows.is_empty() {
                return "No accounts configured.".to_string();
            }

            let mut table = new_table();
            table.set_header(["Name", "Email", "Host", "Port", "TLS", "Default"]);

            for row in &rows {
                table.add_row(vec![
                    row.name.to_string(),
                    row.email.to_string(),
                    row.host.to_string(),
                    row.port.to_string(),
                    row.tls.to_string(),
                    row.default.to_string(),
                ]);
            }

            table.to_string()
        }
        OutputFormat::Json => serde_json_string(&rows),
        OutputFormat::Toon => toon_string(&rows),
    }
}

pub fn format_doctor(report: &DoctorReport, format: &OutputFormat) -> String {
    match format {
        OutputFormat::Table => {
            let mut table = new_table();
            table.set_header(["Check", "Status"]);
            table.add_row(["Account", report.account.as_str()]);
            table.add_row(["DNS", status_label(report.dns_ok)]);
            table.add_row(["TCP", status_label(report.tcp_ok)]);
            table.add_row(["TLS", status_label(report.tls_ok)]);
            table.add_row(["Auth", status_label(report.auth_ok)]);
            table.add_row(["INBOX", status_label(report.inbox_ok)]);

            if let Some(error) = &report.error {
                table.add_row(["Error", error.as_str()]);
            }

            table.to_string()
        }
        OutputFormat::Json => serde_json_string(report),
        OutputFormat::Toon => toon_string(report),
    }
}

fn new_table() -> Table {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table
}

fn serde_json_string<T: Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default()
}

fn toon_string<T: Serialize>(value: &T) -> String {
    toon_format::encode_default(value).unwrap_or_default()
}

fn joined_or_dash(values: &[String]) -> String {
    if values.is_empty() {
        "-".to_string()
    } else {
        values.join(", ")
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    let count = value.chars().count();
    if count <= max_chars {
        return value.to_string();
    }

    let keep = max_chars.saturating_sub(3);
    let prefix = value.chars().take(keep).collect::<String>();
    format!("{}...", prefix)
}

fn status_label(ok: bool) -> &'static str {
    if ok {
        "ok"
    } else {
        "fail"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Address, Flag};

    fn sample_folders() -> Vec<Folder> {
        vec![
            Folder {
                name: "INBOX".to_string(),
                delimiter: Some("/".to_string()),
                attributes: vec![],
            },
            Folder {
                name: "Sent".to_string(),
                delimiter: Some("/".to_string()),
                attributes: vec!["\\Sent".to_string()],
            },
        ]
    }

    fn sample_envelopes() -> Vec<Envelope> {
        vec![
            Envelope {
                uid: 1,
                subject: Some("Hello World".to_string()),
                from: vec![Address {
                    name: Some("Alice".to_string()),
                    email: "alice@example.com".to_string(),
                }],
                to: vec![],
                date: Some("2026-01-01".to_string()),
                flags: vec![Flag::Seen],
                has_attachments: false,
            },
            Envelope {
                uid: 2,
                subject: Some("Test".to_string()),
                from: vec![Address {
                    name: None,
                    email: "bob@example.com".to_string(),
                }],
                to: vec![],
                date: Some("2026-01-02".to_string()),
                flags: vec![],
                has_attachments: true,
            },
        ]
    }

    fn sample_message() -> Message {
        Message {
            envelope: sample_envelopes()[0].clone(),
            text_body: Some("Hello body".to_string()),
            html_body: None,
            attachments: vec![],
            raw: vec![],
        }
    }

    fn sample_account() -> AccountConfig {
        AccountConfig {
            email: "alice@example.com".to_string(),
            display_name: Some("Alice".to_string()),
            host: "imap.example.com".to_string(),
            port: 993,
            tls: true,
            starttls: false,
            insecure: false,
            default_mailbox: "INBOX".to_string(),
            default: true,
        }
    }

    fn sample_doctor() -> DoctorReport {
        DoctorReport {
            account: "personal".to_string(),
            dns_ok: true,
            tcp_ok: true,
            tls_ok: true,
            auth_ok: false,
            inbox_ok: false,
            error: Some("auth failed".to_string()),
        }
    }

    #[test]
    fn format_folders_table() {
        let out = format_folders(&sample_folders(), &OutputFormat::Table);
        assert!(out.contains("INBOX"));
        assert!(out.contains("Sent"));
    }

    #[test]
    fn format_folders_json() {
        let out = format_folders(&sample_folders(), &OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed.as_array().unwrap().len(), 2);
    }

    #[test]
    fn format_folders_toon() {
        let out = format_folders(&sample_folders(), &OutputFormat::Toon);
        assert!(!out.is_empty());
        let back: Vec<Folder> = toon_format::decode_default(&out).unwrap();
        assert_eq!(back.len(), 2);
    }

    #[test]
    fn format_envelopes_table() {
        let out = format_envelopes(&sample_envelopes(), &OutputFormat::Table);
        assert!(out.contains("Hello World"));
        assert!(out.contains("alice@example.com"));
    }

    #[test]
    fn format_envelopes_json() {
        let out = format_envelopes(&sample_envelopes(), &OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed.as_array().unwrap().len(), 2);
        assert_eq!(parsed[0]["uid"], 1);
    }

    #[test]
    fn format_envelopes_empty() {
        let out = format_envelopes(&[], &OutputFormat::Table);
        assert!(out.contains("No messages"));
    }

    #[test]
    fn format_message_json() {
        let out = format_message(&sample_message(), &OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["envelope"]["uid"], 1);
    }

    #[test]
    fn format_message_toon() {
        let out = format_message(&sample_message(), &OutputFormat::Toon);
        let parsed: Message = toon_format::decode_default(&out).unwrap();
        assert_eq!(parsed.envelope.uid, 1);
    }

    #[test]
    fn output_format_from_str() {
        assert_eq!(
            "table".parse::<OutputFormat>().unwrap(),
            OutputFormat::Table
        );
        assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!("toon".parse::<OutputFormat>().unwrap(), OutputFormat::Toon);
        assert!("invalid".parse::<OutputFormat>().is_err());
    }

    #[test]
    fn format_check_table() {
        let checks = vec![MailboxStatus {
            account: "personal".to_string(),
            mailbox: "INBOX".to_string(),
            total: 100,
            unseen: 5,
            recent: 2,
        }];
        let out = format_check(&checks, &OutputFormat::Table);
        assert!(out.contains("personal"));
        assert!(out.contains("5"));
    }

    #[test]
    fn format_accounts_json() {
        let account = sample_account();
        let out = format_accounts(&[("personal", &account)], &OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed[0]["name"], "personal");
        assert_eq!(parsed[0]["email"], "alice@example.com");
    }

    #[test]
    fn format_doctor_json() {
        let out = format_doctor(&sample_doctor(), &OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["account"], "personal");
        assert_eq!(parsed["auth_ok"], false);
    }
}
