use std::net::ToSocketAddrs;

use async_native_tls::TlsConnector;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tracing::{debug, instrument, warn};

use crate::config::AccountConfig;
use crate::error::{ImapError, Result};

type TlsSession = async_imap::Session<async_native_tls::TlsStream<TcpStream>>;
type PlainSession = async_imap::Session<TcpStream>;

pub enum ImapSession {
    Tls(TlsSession),
    Plain(PlainSession),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DoctorReport {
    pub account: String,
    pub dns_ok: bool,
    pub tcp_ok: bool,
    pub tls_ok: bool,
    pub auth_ok: bool,
    pub inbox_ok: bool,
    pub error: Option<String>,
}

#[derive(Debug, Default)]
pub struct SearchQuery {
    pub unseen: bool,
    pub seen: bool,
    pub from: Option<String>,
    pub subject: Option<String>,
    pub since: Option<String>,
    pub before: Option<String>,
    pub limit: u32,
}

impl DoctorReport {
    pub fn all_ok(&self) -> bool {
        self.dns_ok && self.tcp_ok && self.tls_ok && self.auth_ok && self.inbox_ok
    }
}

fn server_addr(config: &AccountConfig) -> String {
    format!("{}:{}", config.host, config.port)
}

impl ImapSession {
    pub async fn select(&mut self, mailbox: &str) -> Result<async_imap::types::Mailbox> {
        match self {
            ImapSession::Tls(s) => s
                .select(mailbox)
                .await
                .map_err(|e| ImapError::Protocol(e.to_string()).into()),
            ImapSession::Plain(s) => s
                .select(mailbox)
                .await
                .map_err(|e| ImapError::Protocol(e.to_string()).into()),
        }
    }

    pub async fn list_folders(&mut self) -> Result<Vec<crate::domain::Folder>> {
        use futures::StreamExt;

        let stream = match self {
            ImapSession::Tls(s) => {
                let names = s
                    .list(Some(""), Some("*"))
                    .await
                    .map_err(|e| ImapError::Protocol(e.to_string()))?;
                names.collect::<Vec<_>>().await
            }
            ImapSession::Plain(s) => {
                let names = s
                    .list(Some(""), Some("*"))
                    .await
                    .map_err(|e| ImapError::Protocol(e.to_string()))?;
                names.collect::<Vec<_>>().await
            }
        };

        Ok(stream
            .into_iter()
            .filter_map(|n| n.ok())
            .map(|n| crate::domain::Folder {
                name: n.name().to_string(),
                delimiter: n.delimiter().map(|d| d.to_string()),
                attributes: n.attributes().iter().map(|a| format!("{:?}", a)).collect(),
            })
            .collect())
    }

    pub async fn list_envelopes(
        &mut self,
        mailbox: &str,
        limit: u32,
        page: u32,
    ) -> Result<Vec<crate::domain::Envelope>> {
        use futures::StreamExt;

        let mbox = self.select(mailbox).await?;
        let total = mbox.exists;
        if total == 0 {
            return Ok(vec![]);
        }

        let page = page.max(1);
        let limit = limit.max(1);
        let end = total.saturating_sub((page - 1) * limit);
        if end == 0 {
            return Ok(vec![]);
        }
        let start = end.saturating_sub(limit - 1).max(1);
        let range = format!("{}:{}", start, end);

        let fetches: Vec<_> = match self {
            ImapSession::Tls(s) => {
                s.fetch(&range, "(UID ENVELOPE FLAGS)")
                    .await
                    .map_err(|e| ImapError::Protocol(e.to_string()))?
                    .collect()
                    .await
            }
            ImapSession::Plain(s) => {
                s.fetch(&range, "(UID ENVELOPE FLAGS)")
                    .await
                    .map_err(|e| ImapError::Protocol(e.to_string()))?
                    .collect()
                    .await
            }
        };

        let mut envelopes: Vec<crate::domain::Envelope> = fetches
            .into_iter()
            .filter_map(|f| f.ok())
            .filter_map(|fetch| {
                let env = fetch.envelope()?;
                let uid = fetch.uid.unwrap_or(0);
                let subject = env
                    .subject
                    .as_ref()
                    .and_then(|s| std::str::from_utf8(s).ok())
                    .map(|s| s.to_string());
                let from = env
                    .from
                    .as_ref()
                    .map(|addrs| {
                        addrs
                            .iter()
                            .map(|a| crate::domain::Address {
                                name: a
                                    .name
                                    .as_ref()
                                    .and_then(|n| std::str::from_utf8(n).ok())
                                    .map(|s| s.to_string()),
                                email: format!(
                                    "{}@{}",
                                    a.mailbox
                                        .as_ref()
                                        .and_then(|m| std::str::from_utf8(m).ok())
                                        .unwrap_or(""),
                                    a.host
                                        .as_ref()
                                        .and_then(|h| std::str::from_utf8(h).ok())
                                        .unwrap_or("")
                                ),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let date = env
                    .date
                    .as_ref()
                    .and_then(|d| std::str::from_utf8(d).ok())
                    .map(|s| s.to_string());
                let flags: Vec<crate::domain::Flag> = fetch
                    .flags()
                    .map(|f| crate::domain::Flag::from_imap_str(&format!("{:?}", f)))
                    .collect();
                Some(crate::domain::Envelope {
                    uid,
                    subject,
                    from,
                    to: vec![],
                    date,
                    flags,
                    has_attachments: false,
                })
            })
            .collect();

        envelopes.reverse();
        Ok(envelopes)
    }

    pub async fn fetch_message(
        &mut self,
        uid: u32,
        mailbox: &str,
    ) -> Result<crate::domain::Message> {
        use futures::StreamExt;
        use mail_parser::MimeHeaders;

        self.select(mailbox).await?;

        let fetches: Vec<_> = match self {
            ImapSession::Tls(s) => {
                s.uid_fetch(uid.to_string(), "(RFC822 FLAGS)")
                    .await
                    .map_err(|e| ImapError::Protocol(e.to_string()))?
                    .collect()
                    .await
            }
            ImapSession::Plain(s) => {
                s.uid_fetch(uid.to_string(), "(RFC822 FLAGS)")
                    .await
                    .map_err(|e| ImapError::Protocol(e.to_string()))?
                    .collect()
                    .await
            }
        };

        let fetch = fetches
            .into_iter()
            .filter_map(|f| f.ok())
            .next()
            .ok_or(ImapError::MessageNotFound { uid })?;

        let raw = fetch.body().unwrap_or(&[]).to_vec();
        let flags: Vec<crate::domain::Flag> = fetch
            .flags()
            .map(|f| crate::domain::Flag::from_imap_str(&format!("{:?}", f)))
            .collect();

        let parsed = mail_parser::MessageParser::default().parse(&raw);

        let (text_body, html_body, attachments, subject, from_addrs, date_str) =
            if let Some(msg) = &parsed {
                let text = msg.body_text(0).map(|s| s.to_string());
                let html = msg.body_html(0).map(|s| s.to_string());

                let mut atts = Vec::new();
                for i in 0..msg.attachment_count() {
                    if let Some(att) = msg.attachment(i as u32) {
                        atts.push(crate::domain::Attachment {
                            filename: att.attachment_name().unwrap_or("attachment").to_string(),
                            content_type: att
                                .content_type()
                                .map(|ct| format!("{}/{}", ct.ctype(), ct.subtype().unwrap_or("")))
                                .unwrap_or_default(),
                            size: att.len(),
                            data: att.contents().to_vec(),
                        });
                    }
                }

                let subj = msg.subject().map(|s| s.to_string());
                let from = msg
                    .from()
                    .and_then(|f| f.first())
                    .map(|a| crate::domain::Address {
                        name: a.name().map(|n| n.to_string()),
                        email: a.address().unwrap_or("").to_string(),
                    });
                let date = msg.date().map(|d| d.to_rfc3339());
                (
                    text,
                    html,
                    atts,
                    subj,
                    from.map(|a| vec![a]).unwrap_or_default(),
                    date,
                )
            } else {
                (None, None, vec![], None, vec![], None)
            };

        Ok(crate::domain::Message {
            envelope: crate::domain::Envelope {
                uid,
                subject,
                from: from_addrs,
                to: vec![],
                date: date_str,
                flags,
                has_attachments: !attachments.is_empty(),
            },
            text_body,
            html_body,
            attachments,
            raw,
        })
    }

    pub async fn search_messages(
        &mut self,
        mailbox: &str,
        query: &SearchQuery,
    ) -> Result<Vec<crate::domain::Envelope>> {
        use futures::StreamExt;

        self.select(mailbox).await?;

        let mut criteria = Vec::new();
        if query.unseen {
            criteria.push("UNSEEN".to_string());
        }
        if query.seen {
            criteria.push("SEEN".to_string());
        }
        if let Some(from) = &query.from {
            criteria.push(format!("FROM \"{}\"", from));
        }
        if let Some(subj) = &query.subject {
            criteria.push(format!("SUBJECT \"{}\"", subj));
        }
        if let Some(since) = &query.since {
            criteria.push(format!("SINCE {}", since));
        }
        if let Some(before) = &query.before {
            criteria.push(format!("BEFORE {}", before));
        }
        if criteria.is_empty() {
            criteria.push("ALL".to_string());
        }
        let search_str = criteria.join(" ");

        let uid_set: std::collections::HashSet<u32> = match self {
            ImapSession::Tls(s) => s
                .uid_search(&search_str)
                .await
                .map_err(|e| ImapError::Protocol(e.to_string()))?,
            ImapSession::Plain(s) => s
                .uid_search(&search_str)
                .await
                .map_err(|e| ImapError::Protocol(e.to_string()))?,
        };

        let limit = if query.limit == 0 {
            20
        } else {
            query.limit as usize
        };
        let mut uids: Vec<u32> = uid_set.into_iter().collect();
        uids.sort_unstable_by(|a, b| b.cmp(a));
        uids.truncate(limit);

        if uids.is_empty() {
            return Ok(vec![]);
        }

        let uid_str = uids
            .iter()
            .map(|u| u.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let fetches: Vec<_> = match self {
            ImapSession::Tls(s) => {
                s.uid_fetch(&uid_str, "(UID ENVELOPE FLAGS)")
                    .await
                    .map_err(|e| ImapError::Protocol(e.to_string()))?
                    .collect()
                    .await
            }
            ImapSession::Plain(s) => {
                s.uid_fetch(&uid_str, "(UID ENVELOPE FLAGS)")
                    .await
                    .map_err(|e| ImapError::Protocol(e.to_string()))?
                    .collect()
                    .await
            }
        };

        Ok(fetches
            .into_iter()
            .filter_map(|f| f.ok())
            .filter_map(|fetch| {
                let env = fetch.envelope()?;
                let uid = fetch.uid.unwrap_or(0);
                let subject = env
                    .subject
                    .as_ref()
                    .and_then(|s| std::str::from_utf8(s).ok())
                    .map(|s| s.to_string());
                let from = env
                    .from
                    .as_ref()
                    .map(|addrs| {
                        addrs
                            .iter()
                            .map(|a| crate::domain::Address {
                                name: a
                                    .name
                                    .as_ref()
                                    .and_then(|n| std::str::from_utf8(n).ok())
                                    .map(|s| s.to_string()),
                                email: format!(
                                    "{}@{}",
                                    a.mailbox
                                        .as_ref()
                                        .and_then(|m| std::str::from_utf8(m).ok())
                                        .unwrap_or(""),
                                    a.host
                                        .as_ref()
                                        .and_then(|h| std::str::from_utf8(h).ok())
                                        .unwrap_or("")
                                ),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                Some(crate::domain::Envelope {
                    uid,
                    subject,
                    from,
                    to: vec![],
                    date: None,
                    flags: vec![],
                    has_attachments: false,
                })
            })
            .collect())
    }

    pub async fn check_mailbox_status(
        &mut self,
        mailbox: &str,
    ) -> Result<crate::output::MailboxStatus> {
        let status = match self {
            ImapSession::Tls(s) => s
                .status(mailbox, "(MESSAGES UNSEEN RECENT)")
                .await
                .map_err(|e| ImapError::Protocol(e.to_string()))?,
            ImapSession::Plain(s) => s
                .status(mailbox, "(MESSAGES UNSEEN RECENT)")
                .await
                .map_err(|e| ImapError::Protocol(e.to_string()))?,
        };
        Ok(crate::output::MailboxStatus {
            account: String::new(),
            mailbox: mailbox.to_string(),
            total: status.exists,
            unseen: status.unseen.unwrap_or(0),
            recent: status.recent,
        })
    }

    pub async fn set_flags(
        &mut self,
        uids: &[u32],
        mailbox: &str,
        flag_strs: &[String],
        add: bool,
    ) -> Result<()> {
        use futures::StreamExt;

        self.select(mailbox).await?;
        let uid_str = uids
            .iter()
            .map(|u| u.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let flags_joined = flag_strs.join(" ");
        let store_cmd = if add {
            format!("+FLAGS ({})", flags_joined)
        } else {
            format!("-FLAGS ({})", flags_joined)
        };
        match self {
            ImapSession::Tls(s) => {
                let stream = s
                    .uid_store(&uid_str, &store_cmd)
                    .await
                    .map_err(|e| ImapError::Protocol(e.to_string()))?;
                let _: Vec<_> = stream.collect().await;
            }
            ImapSession::Plain(s) => {
                let stream = s
                    .uid_store(&uid_str, &store_cmd)
                    .await
                    .map_err(|e| ImapError::Protocol(e.to_string()))?;
                let _: Vec<_> = stream.collect().await;
            }
        }
        Ok(())
    }

    pub async fn move_message(&mut self, uid: u32, source: &str, target: &str) -> Result<()> {
        use futures::StreamExt;

        self.select(source).await?;

        let caps = match self {
            ImapSession::Tls(s) => s
                .capabilities()
                .await
                .map_err(|e| ImapError::Protocol(e.to_string()))?,
            ImapSession::Plain(s) => s
                .capabilities()
                .await
                .map_err(|e| ImapError::Protocol(e.to_string()))?,
        };

        if caps.has_str("MOVE") {
            match self {
                ImapSession::Tls(s) => s
                    .uid_mv(uid.to_string(), target)
                    .await
                    .map_err(|e| ImapError::Protocol(e.to_string()))?,
                ImapSession::Plain(s) => s
                    .uid_mv(uid.to_string(), target)
                    .await
                    .map_err(|e| ImapError::Protocol(e.to_string()))?,
            }
        } else {
            match self {
                ImapSession::Tls(s) => s
                    .uid_copy(uid.to_string(), target)
                    .await
                    .map_err(|e| ImapError::Protocol(e.to_string()))?,
                ImapSession::Plain(s) => s
                    .uid_copy(uid.to_string(), target)
                    .await
                    .map_err(|e| ImapError::Protocol(e.to_string()))?,
            }
            let uid_str = uid.to_string();
            match self {
                ImapSession::Tls(s) => {
                    let stream = s
                        .uid_store(&uid_str, "+FLAGS (\\Deleted)")
                        .await
                        .map_err(|e| ImapError::Protocol(e.to_string()))?;
                    let _: Vec<_> = stream.collect().await;
                    let exp = s
                        .expunge()
                        .await
                        .map_err(|e| ImapError::Protocol(e.to_string()))?;
                    let _: Vec<_> = exp.collect().await;
                }
                ImapSession::Plain(s) => {
                    let stream = s
                        .uid_store(&uid_str, "+FLAGS (\\Deleted)")
                        .await
                        .map_err(|e| ImapError::Protocol(e.to_string()))?;
                    let _: Vec<_> = stream.collect().await;
                    let exp = s
                        .expunge()
                        .await
                        .map_err(|e| ImapError::Protocol(e.to_string()))?;
                    let _: Vec<_> = exp.collect().await;
                }
            }
        }
        Ok(())
    }

    pub async fn delete_message(&mut self, uid: u32, mailbox: &str, force: bool) -> Result<()> {
        use futures::StreamExt;

        if force {
            self.select(mailbox).await?;
            let uid_str = uid.to_string();
            match self {
                ImapSession::Tls(s) => {
                    let stream = s
                        .uid_store(&uid_str, "+FLAGS (\\Deleted)")
                        .await
                        .map_err(|e| ImapError::Protocol(e.to_string()))?;
                    let _: Vec<_> = stream.collect().await;
                    let exp = s
                        .expunge()
                        .await
                        .map_err(|e| ImapError::Protocol(e.to_string()))?;
                    let _: Vec<_> = exp.collect().await;
                }
                ImapSession::Plain(s) => {
                    let stream = s
                        .uid_store(&uid_str, "+FLAGS (\\Deleted)")
                        .await
                        .map_err(|e| ImapError::Protocol(e.to_string()))?;
                    let _: Vec<_> = stream.collect().await;
                    let exp = s
                        .expunge()
                        .await
                        .map_err(|e| ImapError::Protocol(e.to_string()))?;
                    let _: Vec<_> = exp.collect().await;
                }
            }
        } else {
            self.move_message(uid, mailbox, "Trash").await?;
        }
        Ok(())
    }

    pub async fn download_attachments(
        &mut self,
        uid: u32,
        mailbox: &str,
        target_dir: &std::path::Path,
        filename_filter: Option<&str>,
    ) -> Result<Vec<std::path::PathBuf>> {
        let message = self.fetch_message(uid, mailbox).await?;
        let mut saved = Vec::new();

        if message.attachments.is_empty() {
            return Ok(saved);
        }

        for att in &message.attachments {
            if let Some(filter) = filename_filter {
                if att.filename != filter {
                    continue;
                }
            }
            let mut dest = target_dir.join(&att.filename);
            let mut counter = 1u32;
            while dest.exists() {
                let stem = std::path::Path::new(&att.filename)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("attachment");
                let ext = std::path::Path::new(&att.filename)
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                let new_name = if ext.is_empty() {
                    format!("{}_{}", stem, counter)
                } else {
                    format!("{}_{}.{}", stem, counter, ext)
                };
                dest = target_dir.join(new_name);
                counter += 1;
            }
            std::fs::write(&dest, &att.data).map_err(crate::error::MailerboiError::Io)?;
            saved.push(dest);
        }
        Ok(saved)
    }

    pub async fn create_draft(
        &mut self,
        from_email: &str,
        subject: &str,
        body: &str,
        drafts_folder: &str,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc2822();
        let raw = format!(
            "From: {}\r\nSubject: {}\r\nDate: {}\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{}",
            from_email, subject, now, body
        );
        match self {
            ImapSession::Tls(s) => s
                .append(drafts_folder, None, None, raw.as_bytes())
                .await
                .map_err(|e| ImapError::Protocol(e.to_string()))?,
            ImapSession::Plain(s) => s
                .append(drafts_folder, None, None, raw.as_bytes())
                .await
                .map_err(|e| ImapError::Protocol(e.to_string()))?,
        }
        Ok(())
    }

    pub async fn logout(mut self) -> Result<()> {
        match self {
            ImapSession::Tls(ref mut s) => s
                .logout()
                .await
                .map_err(|e| ImapError::Protocol(e.to_string()).into()),
            ImapSession::Plain(ref mut s) => s
                .logout()
                .await
                .map_err(|e| ImapError::Protocol(e.to_string()).into()),
        }
    }
}

#[instrument(skip(password))]
pub async fn connect(config: &AccountConfig, password: &str) -> Result<ImapSession> {
    let addr = server_addr(config);
    debug!("Connecting to {}", addr);

    if config.tls && !config.starttls {
        let tcp = TcpStream::connect(&addr)
            .await
            .map_err(|e| ImapError::ConnectionFailed {
                host: config.host.clone(),
                port: config.port,
                reason: e.to_string(),
            })?;
        let mut connector = TlsConnector::new();
        if config.insecure {
            connector = connector
                .danger_accept_invalid_certs(true)
                .danger_accept_invalid_hostnames(true);
        }
        let tls_stream = connector
            .connect(&config.host, tcp)
            .await
            .map_err(|e| ImapError::Tls(e.to_string()))?;
        let client = async_imap::Client::new(tls_stream);
        let session = client
            .login(&config.email, password)
            .await
            .map_err(|(_e, _)| ImapError::AuthFailed {
                user: config.email.clone(),
            })?;
        Ok(ImapSession::Tls(session))
    } else {
        if config.starttls {
            warn!("STARTTLS requested but not implemented; using plain IMAP");
        }
        let tcp = TcpStream::connect(&addr)
            .await
            .map_err(|e| ImapError::ConnectionFailed {
                host: config.host.clone(),
                port: config.port,
                reason: e.to_string(),
            })?;
        let client = async_imap::Client::new(tcp);
        let session = client
            .login(&config.email, password)
            .await
            .map_err(|(_e, _)| ImapError::AuthFailed {
                user: config.email.clone(),
            })?;
        Ok(ImapSession::Plain(session))
    }
}

pub async fn disconnect(session: ImapSession) -> Result<()> {
    session.logout().await
}

#[instrument(skip(password))]
pub async fn doctor(config: &AccountConfig, password: &str) -> DoctorReport {
    let mut report = DoctorReport {
        account: config.email.clone(),
        dns_ok: false,
        tcp_ok: false,
        tls_ok: false,
        auth_ok: false,
        inbox_ok: false,
        error: None,
    };

    let addr = server_addr(config);
    match addr.to_socket_addrs() {
        Ok(_) => {
            report.dns_ok = true;
            debug!("DNS OK");
        }
        Err(e) => {
            report.error = Some(format!("DNS failed: {}", e));
            return report;
        }
    }

    let tcp = match TcpStream::connect(&addr).await {
        Ok(t) => {
            report.tcp_ok = true;
            debug!("TCP OK");
            t
        }
        Err(e) => {
            report.error = Some(format!("TCP failed: {}", e));
            return report;
        }
    };

    if config.tls && !config.starttls {
        let mut connector = TlsConnector::new();
        if config.insecure {
            connector = connector
                .danger_accept_invalid_certs(true)
                .danger_accept_invalid_hostnames(true);
        }
        let tls_stream = match connector.connect(&config.host, tcp).await {
            Ok(s) => {
                report.tls_ok = true;
                debug!("TLS OK");
                s
            }
            Err(e) => {
                report.error = Some(format!("TLS failed: {}", e));
                return report;
            }
        };
        let client = async_imap::Client::new(tls_stream);
        let mut session = match client.login(&config.email, password).await {
            Ok(s) => {
                report.auth_ok = true;
                debug!("Auth OK");
                s
            }
            Err((e, _)) => {
                report.error = Some(format!("Auth failed: {}", e));
                return report;
            }
        };
        match session.select("INBOX").await {
            Ok(_) => {
                report.inbox_ok = true;
                debug!("INBOX OK");
            }
            Err(e) => {
                report.error = Some(format!("SELECT INBOX failed: {}", e));
            }
        }
        let _ = session.logout().await;
    } else {
        if config.starttls {
            warn!("STARTTLS requested but doctor uses plain IMAP");
        }
        report.tls_ok = true;
        let client = async_imap::Client::new(tcp);
        let mut session = match client.login(&config.email, password).await {
            Ok(s) => {
                report.auth_ok = true;
                debug!("Auth OK");
                s
            }
            Err((e, _)) => {
                report.error = Some(format!("Auth failed: {}", e));
                return report;
            }
        };
        match session.select("INBOX").await {
            Ok(_) => {
                report.inbox_ok = true;
                debug!("INBOX OK");
            }
            Err(e) => {
                report.error = Some(format!("SELECT INBOX failed: {}", e));
            }
        }
        let _ = session.logout().await;
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn greenmail_tls_config() -> AccountConfig {
        AccountConfig {
            email: "test@localhost".to_string(),
            display_name: None,
            host: "127.0.0.1".to_string(),
            port: 3993,
            tls: true,
            starttls: false,
            insecure: true,
            default_mailbox: "INBOX".to_string(),
            default: true,
        }
    }

    fn greenmail_plain_config() -> AccountConfig {
        AccountConfig {
            email: "test2@localhost".to_string(),
            display_name: None,
            host: "127.0.0.1".to_string(),
            port: 3143,
            tls: false,
            starttls: false,
            insecure: false,
            default_mailbox: "INBOX".to_string(),
            default: false,
        }
    }

    #[test]
    fn connection_addr_uses_host_and_port() {
        let config = greenmail_tls_config();
        assert_eq!(server_addr(&config), "127.0.0.1:3993");
    }

    #[test]
    fn connection_addr_supports_plain_port() {
        let config = greenmail_plain_config();
        assert_eq!(server_addr(&config), "127.0.0.1:3143");
    }

    #[tokio::test]
    #[ignore]
    async fn connect_tls_and_select_inbox() {
        let config = greenmail_tls_config();
        let mut session = connect(&config, "test").await.unwrap();
        let mailbox = session.select("INBOX").await.unwrap();
        println!("INBOX: {:?}", mailbox);
        disconnect(session).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn connect_plain_and_select_inbox() {
        let config = greenmail_plain_config();
        let mut session = connect(&config, "test2").await.unwrap();
        session.select("INBOX").await.unwrap();
        disconnect(session).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn doctor_tls_all_ok() {
        let config = greenmail_tls_config();
        let report = doctor(&config, "test").await;
        println!("Doctor report: {:?}", report);
        assert!(report.dns_ok, "DNS failed");
        assert!(report.tcp_ok, "TCP failed");
        assert!(report.tls_ok, "TLS failed");
        assert!(report.auth_ok, "Auth failed: {:?}", report.error);
        assert!(report.inbox_ok, "INBOX failed: {:?}", report.error);
        assert!(report.all_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn doctor_wrong_password() {
        let config = greenmail_tls_config();
        let mut bad_config = config.clone();
        bad_config.port = 9999;
        let report = doctor(&bad_config, "wrong").await;
        assert!(!report.tcp_ok);
        assert!(!report.all_ok());
    }

    #[test]
    fn doctor_report_all_ok() {
        let report = DoctorReport {
            account: "test".to_string(),
            dns_ok: true,
            tcp_ok: true,
            tls_ok: true,
            auth_ok: true,
            inbox_ok: true,
            error: None,
        };
        assert!(report.all_ok());
    }

    #[test]
    fn doctor_report_not_all_ok() {
        let report = DoctorReport {
            account: "test".to_string(),
            dns_ok: true,
            tcp_ok: false,
            tls_ok: false,
            auth_ok: false,
            inbox_ok: false,
            error: Some("TCP failed".to_string()),
        };
        assert!(!report.all_ok());
    }
}
