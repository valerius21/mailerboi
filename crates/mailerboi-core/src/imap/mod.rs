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
            ImapSession::Tls(s) => s
                .fetch(&range, "(UID ENVELOPE FLAGS)")
                .await
                .map_err(|e| ImapError::Protocol(e.to_string()))?
                .collect()
                .await,
            ImapSession::Plain(s) => s
                .fetch(&range, "(UID ENVELOPE FLAGS)")
                .await
                .map_err(|e| ImapError::Protocol(e.to_string()))?
                .collect()
                .await,
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
