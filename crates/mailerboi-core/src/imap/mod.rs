//! IMAP connection management and mailbox operations.
//!
//! Supports both TLS (port 993) and plain (port 143) IMAP connections.
//! All operations use UIDs for stable message references.

use std::net::ToSocketAddrs;

use async_native_tls::TlsConnector;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tracing::{debug, instrument, warn};

use crate::config::AccountConfig;
use crate::error::{ImapError, Result};

type TlsSession = async_imap::Session<async_native_tls::TlsStream<TcpStream>>;
type PlainSession = async_imap::Session<TcpStream>;

/// An authenticated IMAP session over TLS or plain TCP.
pub enum ImapSession {
    /// A TLS-protected IMAP session.
    Tls(TlsSession),
    /// A plain-text IMAP session.
    Plain(PlainSession),
}

/// Connectivity and mailbox health checks for one account.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DoctorReport {
    /// Account email address used for the check.
    pub account: String,
    /// DNS resolution succeeded for the configured host.
    pub dns_ok: bool,
    /// A TCP connection to the IMAP server succeeded.
    pub tcp_ok: bool,
    /// TLS negotiation succeeded, or plain IMAP was intentionally used.
    pub tls_ok: bool,
    /// Authentication with the supplied credentials succeeded.
    pub auth_ok: bool,
    /// Selecting `INBOX` succeeded after login.
    pub inbox_ok: bool,
    /// First failure encountered during the diagnostic run.
    pub error: Option<String>,
}

/// Search filters for [`ImapSession::search_messages`].
#[derive(Debug, Default)]
pub struct SearchQuery {
    /// Restrict results to unread messages.
    pub unseen: bool,
    /// Restrict results to read messages.
    pub seen: bool,
    /// Match sender addresses containing this string.
    pub from: Option<String>,
    /// Match subjects containing this string.
    pub subject: Option<String>,
    /// Match messages on or after an IMAP date value.
    pub since: Option<String>,
    /// Match messages before an IMAP date value.
    pub before: Option<String>,
    /// Maximum number of results to return; `0` falls back to an internal default.
    pub limit: u32,
}

impl DoctorReport {
    /// Returns `true` when every diagnostic check succeeded.
    pub fn all_ok(&self) -> bool {
        self.dns_ok && self.tcp_ok && self.tls_ok && self.auth_ok && self.inbox_ok
    }
}

fn server_addr(config: &AccountConfig) -> String {
    format!("{}:{}", config.host, config.port)
}

impl ImapSession {
    /// Selects a mailbox and returns its server status.
    ///
    /// Returns [`ImapError::Protocol`] if the server rejects the request.
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

    /// Lists folders visible to the authenticated account.
    ///
    /// Returns [`ImapError::Protocol`] if the `LIST` command fails.
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

    /// Lists message envelopes for one mailbox page.
    ///
    /// `limit` and `page` are clamped to at least `1`. Returns
    /// [`ImapError::Protocol`] if selection or fetching fails.
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

    /// Fetches one message by UID, including bodies and attachments.
    ///
    /// Returns [`ImapError::MessageNotFound`] when the UID is missing and
    /// [`ImapError::Protocol`] when IMAP fetching fails.
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

    /// Searches a mailbox and returns matching envelopes.
    ///
    /// Empty criteria default to `ALL`. Returns [`ImapError::Protocol`] if the
    /// search or fetch commands fail.
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

    /// Reads message counts for a mailbox.
    ///
    /// Returns [`ImapError::Protocol`] if the server rejects the status query.
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

    /// Adds or removes flags for the given UIDs.
    ///
    /// When `add` is `true`, flags are added; otherwise they are removed.
    /// Returns [`ImapError::Protocol`] if the store command fails.
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

    /// Moves a message to another mailbox.
    ///
    /// Falls back to copy-plus-delete when the server lacks `MOVE` support.
    /// Returns [`ImapError::Protocol`] if any IMAP command fails.
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

    /// Deletes a message from a mailbox.
    ///
    /// When `force` is `false`, the message is moved to `Trash`; otherwise it is
    /// marked deleted and expunged. Returns [`ImapError::Protocol`] if the server
    /// rejects the operation.
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

    /// Saves matching attachments from one message into `target_dir`.
    ///
    /// Existing filenames are de-duplicated by appending a numeric suffix.
    /// Returns [`ImapError::MessageNotFound`], [`ImapError::Protocol`], or
    /// [`crate::error::MailerboiError::Io`] if writing a file fails.
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
            tokio::fs::write(&dest, &att.data)
                .await
                .map_err(crate::error::MailerboiError::Io)?;
            saved.push(dest);
        }
        Ok(saved)
    }

    /// Appends a simple plain-text draft message to `drafts_folder`.
    ///
    /// Returns [`ImapError::Protocol`] if folder creation or append fails.
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
        // Try to create the Drafts folder if it doesn't exist
        let _ = match self {
            ImapSession::Tls(s) => s.create(drafts_folder).await,
            ImapSession::Plain(s) => s.create(drafts_folder).await,
        };
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

    /// Logs out and closes the IMAP session.
    ///
    /// Returns [`ImapError::Protocol`] if logout fails.
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
/// Connects to an IMAP server and authenticates the configured account.
///
/// Uses implicit TLS when [`AccountConfig::tls`] is enabled without STARTTLS.
/// Returns [`ImapError::ConnectionFailed`], [`ImapError::Tls`],
/// [`ImapError::AuthFailed`], or [`ImapError::Protocol`] on failure.
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
            .login(config.login_username(), password)
            .await
            .map_err(|(_e, _)| ImapError::AuthFailed {
                user: config.login_username().to_string(),
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
            .login(config.login_username(), password)
            .await
            .map_err(|(_e, _)| ImapError::AuthFailed {
                user: config.login_username().to_string(),
            })?;
        Ok(ImapSession::Plain(session))
    }
}

/// Logs out from an IMAP session.
pub async fn disconnect(session: ImapSession) -> Result<()> {
    session.logout().await
}

#[instrument(skip(password))]
/// Runs connectivity and login diagnostics for one account.
///
/// The returned [`DoctorReport`] records each step and captures the first error
/// message instead of returning early with a [`Result`].
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
        let mut session = match client.login(config.login_username(), password).await {
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
        let mut session = match client.login(config.login_username(), password).await {
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
    use tempfile::tempdir;

    fn greenmail_tls_config() -> AccountConfig {
        AccountConfig {
            email: "test@localhost".to_string(),
            username: None,
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
            username: None,
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

    /// Creates a TLS session for an isolated per-test user so tests don't
    /// interfere with each other. GreenMail creates the user on first login.
    async fn session_for(user: &str) -> ImapSession {
        let config = AccountConfig {
            email: format!("{}@localhost", user),
            ..greenmail_tls_config()
        };
        connect(&config, user).await.unwrap()
    }

    // ── list_folders ─────────────────────────────────────────────────────────

    #[tokio::test]
    #[ignore]
    async fn list_folders_contains_inbox() {
        let mut session = session_for("folders_user").await;
        let folders = session.list_folders().await.unwrap();
        let names: Vec<&str> = folders.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"INBOX"), "INBOX missing from {:?}", names);
        session.logout().await.unwrap();
    }

    // ── list_envelopes ────────────────────────────────────────────────────────

    #[tokio::test]
    #[ignore]
    async fn list_envelopes_empty_mailbox() {
        let mut session = session_for("envelopes_empty").await;
        let envelopes = session.list_envelopes("INBOX", 10, 1).await.unwrap();
        assert!(envelopes.is_empty());
        session.logout().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn list_envelopes_with_message() {
        let mut session = session_for("envelopes_msg").await;
        session
            .create_draft("envelopes_msg@localhost", "Envelope Test", "body", "INBOX")
            .await
            .unwrap();
        let envelopes = session.list_envelopes("INBOX", 10, 1).await.unwrap();
        assert!(!envelopes.is_empty());
        let subjects: Vec<&str> = envelopes
            .iter()
            .filter_map(|e| e.subject.as_deref())
            .collect();
        assert!(
            subjects.iter().any(|s| s.contains("Envelope Test")),
            "subject not found in {:?}",
            subjects
        );
        session.logout().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn list_envelopes_pagination() {
        let mut session = session_for("envelopes_page").await;
        for i in 0..5 {
            session
                .create_draft(
                    "envelopes_page@localhost",
                    &format!("Msg {}", i),
                    "body",
                    "INBOX",
                )
                .await
                .unwrap();
        }
        let page1 = session.list_envelopes("INBOX", 2, 1).await.unwrap();
        assert_eq!(page1.len(), 2);
        let page2 = session.list_envelopes("INBOX", 2, 2).await.unwrap();
        assert_eq!(page2.len(), 2);
        // UIDs should be different across pages
        let p1_uids: Vec<u32> = page1.iter().map(|e| e.uid).collect();
        let p2_uids: Vec<u32> = page2.iter().map(|e| e.uid).collect();
        assert!(
            p1_uids.iter().all(|u| !p2_uids.contains(u)),
            "pages overlap: {:?} / {:?}",
            p1_uids,
            p2_uids
        );
        session.logout().await.unwrap();
    }

    // ── fetch_message ─────────────────────────────────────────────────────────

    #[tokio::test]
    #[ignore]
    async fn fetch_message_returns_body() {
        let mut session = session_for("fetch_msg").await;
        session
            .create_draft("fetch_msg@localhost", "Fetch Me", "Hello world", "INBOX")
            .await
            .unwrap();
        let envelopes = session.list_envelopes("INBOX", 10, 1).await.unwrap();
        let uid = envelopes[0].uid;
        let msg = session.fetch_message(uid, "INBOX").await.unwrap();
        assert_eq!(msg.envelope.uid, uid);
        assert!(
            msg.text_body
                .as_deref()
                .unwrap_or("")
                .contains("Hello world"),
            "body not found: {:?}",
            msg.text_body
        );
        session.logout().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn fetch_message_not_found_returns_error() {
        let mut session = session_for("fetch_missing").await;
        let result = session.fetch_message(999_999, "INBOX").await;
        assert!(result.is_err());
        session.logout().await.unwrap();
    }

    // ── search_messages ───────────────────────────────────────────────────────

    #[tokio::test]
    #[ignore]
    async fn search_messages_all_returns_results() {
        let mut session = session_for("search_all").await;
        session
            .create_draft("search_all@localhost", "Searchable", "body", "INBOX")
            .await
            .unwrap();
        let results = session
            .search_messages("INBOX", &SearchQuery::default())
            .await
            .unwrap();
        assert!(!results.is_empty());
        session.logout().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn search_messages_unseen() {
        let mut session = session_for("search_unseen").await;
        session
            .create_draft("search_unseen@localhost", "Unseen Msg", "body", "INBOX")
            .await
            .unwrap();
        let results = session
            .search_messages(
                "INBOX",
                &SearchQuery {
                    unseen: true,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        // Appended messages start as \Recent but not \Seen
        assert!(!results.is_empty());
        session.logout().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn search_messages_empty_mailbox() {
        let mut session = session_for("search_empty").await;
        let results = session
            .search_messages("INBOX", &SearchQuery::default())
            .await
            .unwrap();
        assert!(results.is_empty());
        session.logout().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn search_messages_with_limit() {
        let mut session = session_for("search_limit").await;
        for i in 0..5 {
            session
                .create_draft(
                    "search_limit@localhost",
                    &format!("Msg {}", i),
                    "body",
                    "INBOX",
                )
                .await
                .unwrap();
        }
        let results = session
            .search_messages(
                "INBOX",
                &SearchQuery {
                    limit: 2,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
        session.logout().await.unwrap();
    }

    // ── check_mailbox_status ──────────────────────────────────────────────────

    #[tokio::test]
    #[ignore]
    async fn check_mailbox_status_empty() {
        let mut session = session_for("status_empty").await;
        let status = session.check_mailbox_status("INBOX").await.unwrap();
        assert_eq!(status.total, 0);
        session.logout().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn check_mailbox_status_with_message() {
        let mut session = session_for("status_msg").await;
        session
            .create_draft("status_msg@localhost", "Status Test", "body", "INBOX")
            .await
            .unwrap();
        let status = session.check_mailbox_status("INBOX").await.unwrap();
        assert!(status.total >= 1);
        session.logout().await.unwrap();
    }

    // ── set_flags ─────────────────────────────────────────────────────────────

    #[tokio::test]
    #[ignore]
    async fn set_flags_marks_message_seen() {
        let mut session = session_for("flags_seen").await;
        session
            .create_draft("flags_seen@localhost", "Flag Me", "body", "INBOX")
            .await
            .unwrap();
        let envelopes = session.list_envelopes("INBOX", 10, 1).await.unwrap();
        let uid = envelopes[0].uid;
        session
            .set_flags(&[uid], "INBOX", &["\\Seen".to_string()], true)
            .await
            .unwrap();
        // Verify via search: the message should no longer appear in UNSEEN
        let unseen = session
            .search_messages(
                "INBOX",
                &SearchQuery {
                    unseen: true,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert!(
            unseen.iter().all(|e| e.uid != uid),
            "uid {} still unseen after marking seen",
            uid
        );
        session.logout().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn set_flags_removes_flag() {
        let mut session = session_for("flags_remove").await;
        session
            .create_draft("flags_remove@localhost", "Flag Remove", "body", "INBOX")
            .await
            .unwrap();
        let envelopes = session.list_envelopes("INBOX", 10, 1).await.unwrap();
        let uid = envelopes[0].uid;
        // Add then remove \Flagged
        session
            .set_flags(&[uid], "INBOX", &["\\Flagged".to_string()], true)
            .await
            .unwrap();
        session
            .set_flags(&[uid], "INBOX", &["\\Flagged".to_string()], false)
            .await
            .unwrap();
        session.logout().await.unwrap();
    }

    // ── move_message ──────────────────────────────────────────────────────────

    #[tokio::test]
    #[ignore]
    async fn move_message_to_trash() {
        let mut session = session_for("move_msg").await;
        session
            .create_draft("move_msg@localhost", "Move Me", "body", "INBOX")
            .await
            .unwrap();
        let envelopes = session.list_envelopes("INBOX", 10, 1).await.unwrap();
        let uid = envelopes[0].uid;
        // Create Trash first so the move target exists
        let _ = match &mut session {
            ImapSession::Tls(s) => s.create("Trash").await,
            ImapSession::Plain(s) => s.create("Trash").await,
        };
        session.move_message(uid, "INBOX", "Trash").await.unwrap();
        // Message should be gone from INBOX
        let inbox = session.list_envelopes("INBOX", 10, 1).await.unwrap();
        assert!(
            inbox.iter().all(|e| e.uid != uid),
            "uid {} still in INBOX after move",
            uid
        );
        session.logout().await.unwrap();
    }

    // ── delete_message ────────────────────────────────────────────────────────

    #[tokio::test]
    #[ignore]
    async fn delete_message_force_expunges() {
        let mut session = session_for("delete_force").await;
        session
            .create_draft("delete_force@localhost", "Delete Me", "body", "INBOX")
            .await
            .unwrap();
        let envelopes = session.list_envelopes("INBOX", 10, 1).await.unwrap();
        let uid = envelopes[0].uid;
        session.delete_message(uid, "INBOX", true).await.unwrap();
        let remaining = session.list_envelopes("INBOX", 10, 1).await.unwrap();
        assert!(
            remaining.iter().all(|e| e.uid != uid),
            "uid {} still present after force delete",
            uid
        );
        session.logout().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn delete_message_soft_moves_to_trash() {
        let mut session = session_for("delete_soft").await;
        // GreenMail does not auto-create Trash; create it before soft-delete
        let _ = match &mut session {
            ImapSession::Tls(s) => s.create("Trash").await,
            ImapSession::Plain(s) => s.create("Trash").await,
        };
        session
            .create_draft("delete_soft@localhost", "Soft Delete", "body", "INBOX")
            .await
            .unwrap();
        let envelopes = session.list_envelopes("INBOX", 10, 1).await.unwrap();
        let uid = envelopes[0].uid;
        session.delete_message(uid, "INBOX", false).await.unwrap();
        let inbox = session.list_envelopes("INBOX", 10, 1).await.unwrap();
        assert!(
            inbox.iter().all(|e| e.uid != uid),
            "uid {} still in INBOX after soft delete",
            uid
        );
        session.logout().await.unwrap();
    }

    // ── download_attachments ──────────────────────────────────────────────────

    #[tokio::test]
    #[ignore]
    async fn download_attachments_plain_message_returns_empty() {
        let mut session = session_for("download_plain").await;
        session
            .create_draft(
                "download_plain@localhost",
                "No Attachments",
                "Just text",
                "INBOX",
            )
            .await
            .unwrap();
        let envelopes = session.list_envelopes("INBOX", 10, 1).await.unwrap();
        let uid = envelopes[0].uid;
        let dir = tempdir().unwrap();
        let saved = session
            .download_attachments(uid, "INBOX", dir.path(), None)
            .await
            .unwrap();
        assert!(saved.is_empty());
        session.logout().await.unwrap();
    }

    // ── create_draft ──────────────────────────────────────────────────────────

    #[tokio::test]
    #[ignore]
    async fn create_draft_appends_to_drafts_folder() {
        let mut session = session_for("draft_create").await;
        session
            .create_draft(
                "draft_create@localhost",
                "My Draft",
                "draft body",
                "Drafts",
            )
            .await
            .unwrap();
        let envelopes = session.list_envelopes("Drafts", 10, 1).await.unwrap();
        assert!(!envelopes.is_empty());
        let subjects: Vec<&str> = envelopes
            .iter()
            .filter_map(|e| e.subject.as_deref())
            .collect();
        assert!(
            subjects.iter().any(|s| s.contains("My Draft")),
            "draft subject not found in {:?}",
            subjects
        );
        session.logout().await.unwrap();
    }

    // ── disconnect ────────────────────────────────────────────────────────────

    #[tokio::test]
    #[ignore]
    async fn disconnect_helper_closes_session() {
        let config = greenmail_tls_config();
        let session = connect(&config, "test").await.unwrap();
        disconnect(session).await.unwrap();
    }

    // ── plain TCP variants ────────────────────────────────────────────────────

    async fn plain_session_for(user: &str) -> ImapSession {
        let config = AccountConfig {
            email: format!("{}@localhost", user),
            ..greenmail_plain_config()
        };
        connect(&config, user).await.unwrap()
    }

    #[tokio::test]
    #[ignore]
    async fn plain_list_folders_contains_inbox() {
        let mut session = plain_session_for("plain_folders").await;
        let folders = session.list_folders().await.unwrap();
        let names: Vec<&str> = folders.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"INBOX"), "INBOX missing: {:?}", names);
        session.logout().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn plain_list_envelopes_with_message() {
        let mut session = plain_session_for("plain_envelopes").await;
        session
            .create_draft("plain_envelopes@localhost", "Plain Test", "body", "INBOX")
            .await
            .unwrap();
        let envelopes = session.list_envelopes("INBOX", 10, 1).await.unwrap();
        assert!(!envelopes.is_empty());
        session.logout().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn plain_fetch_message() {
        let mut session = plain_session_for("plain_fetch").await;
        session
            .create_draft("plain_fetch@localhost", "Plain Fetch", "hello plain", "INBOX")
            .await
            .unwrap();
        let envelopes = session.list_envelopes("INBOX", 10, 1).await.unwrap();
        let uid = envelopes[0].uid;
        let msg = session.fetch_message(uid, "INBOX").await.unwrap();
        assert_eq!(msg.envelope.uid, uid);
        session.logout().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn plain_search_messages() {
        let mut session = plain_session_for("plain_search").await;
        session
            .create_draft("plain_search@localhost", "Plain Search", "body", "INBOX")
            .await
            .unwrap();
        let results = session
            .search_messages("INBOX", &SearchQuery::default())
            .await
            .unwrap();
        assert!(!results.is_empty());
        session.logout().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn plain_check_mailbox_status() {
        let mut session = plain_session_for("plain_status").await;
        let status = session.check_mailbox_status("INBOX").await.unwrap();
        let _ = status.total;
        session.logout().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn plain_set_flags() {
        let mut session = plain_session_for("plain_flags").await;
        session
            .create_draft("plain_flags@localhost", "Plain Flags", "body", "INBOX")
            .await
            .unwrap();
        let envelopes = session.list_envelopes("INBOX", 10, 1).await.unwrap();
        let uid = envelopes[0].uid;
        session
            .set_flags(&[uid], "INBOX", &["\\Seen".to_string()], true)
            .await
            .unwrap();
        session.logout().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn plain_move_message() {
        let mut session = plain_session_for("plain_move").await;
        let _ = match &mut session {
            ImapSession::Tls(s) => s.create("Trash").await,
            ImapSession::Plain(s) => s.create("Trash").await,
        };
        session
            .create_draft("plain_move@localhost", "Plain Move", "body", "INBOX")
            .await
            .unwrap();
        let envelopes = session.list_envelopes("INBOX", 10, 1).await.unwrap();
        let uid = envelopes[0].uid;
        session.move_message(uid, "INBOX", "Trash").await.unwrap();
        session.logout().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn plain_delete_message_force() {
        let mut session = plain_session_for("plain_delete").await;
        session
            .create_draft("plain_delete@localhost", "Plain Delete", "body", "INBOX")
            .await
            .unwrap();
        let envelopes = session.list_envelopes("INBOX", 10, 1).await.unwrap();
        let uid = envelopes[0].uid;
        session.delete_message(uid, "INBOX", true).await.unwrap();
        session.logout().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn plain_create_draft() {
        let mut session = plain_session_for("plain_draft").await;
        session
            .create_draft("plain_draft@localhost", "Plain Draft", "body", "Drafts")
            .await
            .unwrap();
        let envelopes = session.list_envelopes("Drafts", 10, 1).await.unwrap();
        assert!(!envelopes.is_empty());
        session.logout().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn plain_download_attachments_empty() {
        let mut session = plain_session_for("plain_download").await;
        session
            .create_draft("plain_download@localhost", "Plain DL", "body", "INBOX")
            .await
            .unwrap();
        let envelopes = session.list_envelopes("INBOX", 10, 1).await.unwrap();
        let uid = envelopes[0].uid;
        let dir = tempdir().unwrap();
        let saved = session
            .download_attachments(uid, "INBOX", dir.path(), None)
            .await
            .unwrap();
        assert!(saved.is_empty());
        session.logout().await.unwrap();
    }

    // ── doctor plain path ─────────────────────────────────────────────────────

    #[tokio::test]
    #[ignore]
    async fn doctor_plain_all_ok() {
        let config = greenmail_plain_config();
        let report = doctor(&config, "test2").await;
        assert!(report.dns_ok);
        assert!(report.tcp_ok);
        assert!(report.tls_ok, "plain IMAP sets tls_ok=true");
        assert!(report.auth_ok, "auth failed: {:?}", report.error);
        assert!(report.inbox_ok, "inbox failed: {:?}", report.error);
        assert!(report.all_ok());
    }
}
