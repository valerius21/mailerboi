//! Message envelope and address types.

use super::flag::Flag;
use serde::{Deserialize, Serialize};
use std::fmt;

/// A named email address.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Address {
    /// Display name, when present in the message headers.
    pub name: Option<String>,
    pub email: String,
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = &self.name {
            write!(f, "{} <{}>", name, self.email)
        } else {
            write!(f, "{}", self.email)
        }
    }
}

/// Summary metadata for one message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Envelope {
    /// Stable IMAP UID for the message.
    pub uid: u32,
    /// Decoded `Subject` header.
    pub subject: Option<String>,
    /// Parsed sender addresses.
    pub from: Vec<Address>,
    /// Parsed recipient addresses when available.
    pub to: Vec<Address>,
    /// Message date as a display-ready string.
    pub date: Option<String>,
    /// IMAP flags currently set on the message.
    pub flags: Vec<Flag>,
    /// Indicates whether attachment metadata is known to exist.
    pub has_attachments: bool,
}

impl fmt::Display for Envelope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let from = self
            .from
            .first()
            .map(|address| address.to_string())
            .unwrap_or_default();
        let subject = self.subject.as_deref().unwrap_or("(no subject)");
        let date = self.date.as_deref().unwrap_or("unknown");

        write!(f, "[{}] {} | {} | {}", self.uid, from, subject, date)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_display_with_name() {
        let address = Address {
            name: Some("Alice".to_string()),
            email: "alice@example.com".to_string(),
        };

        assert_eq!(format!("{}", address), "Alice <alice@example.com>");
    }

    #[test]
    fn address_display_without_name() {
        let address = Address {
            name: None,
            email: "bob@example.com".to_string(),
        };

        assert_eq!(format!("{}", address), "bob@example.com");
    }

    #[test]
    fn envelope_display() {
        let envelope = Envelope {
            uid: 42,
            subject: Some("Hello".to_string()),
            from: vec![Address {
                name: Some("Alice".to_string()),
                email: "alice@example.com".to_string(),
            }],
            to: vec![],
            date: Some("2026-01-01".to_string()),
            flags: vec![Flag::Seen],
            has_attachments: false,
        };

        assert_eq!(
            format!("{}", envelope),
            "[42] Alice <alice@example.com> | Hello | 2026-01-01"
        );
    }

    #[test]
    fn envelope_serialize_roundtrip() {
        let envelope = Envelope {
            uid: 42,
            subject: Some("Hello".to_string()),
            from: vec![Address {
                name: None,
                email: "a@b.com".to_string(),
            }],
            to: vec![],
            date: Some("2026-01-01".to_string()),
            flags: vec![Flag::Seen],
            has_attachments: false,
        };

        let json = serde_json::to_string(&envelope).unwrap();
        let back: Envelope = serde_json::from_str(&json).unwrap();

        assert_eq!(envelope, back);
    }
}
