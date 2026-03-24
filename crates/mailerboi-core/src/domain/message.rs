//! Full message payloads and attachment metadata.

use super::envelope::Envelope;
use serde::{Deserialize, Serialize};
use std::fmt;

/// One attachment extracted from a message body.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Attachment {
    /// Attachment filename from MIME metadata.
    pub filename: String,
    /// Attachment MIME type.
    pub content_type: String,
    /// Attachment size in bytes.
    pub size: usize,
    #[serde(skip)]
    /// Raw attachment bytes, skipped during serialization.
    pub data: Vec<u8>,
}

impl fmt::Display for Attachment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}, {} bytes)",
            self.filename, self.content_type, self.size
        )
    }
}

/// A fully fetched message with decoded bodies and attachments.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    /// Envelope metadata for the message.
    pub envelope: Envelope,
    /// Plain-text body, if available.
    pub text_body: Option<String>,
    /// HTML body, if available.
    pub html_body: Option<String>,
    /// Attachments extracted from the MIME structure.
    pub attachments: Vec<Attachment>,
    #[serde(skip)]
    /// Raw RFC822 message bytes, skipped during serialization.
    pub raw: Vec<u8>,
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.envelope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Address, Flag};

    fn sample_envelope() -> Envelope {
        Envelope {
            uid: 1,
            subject: Some("Test".to_string()),
            from: vec![Address {
                name: None,
                email: "a@b.com".to_string(),
            }],
            to: vec![],
            date: None,
            flags: vec![Flag::Seen],
            has_attachments: false,
        }
    }

    #[test]
    fn attachment_display() {
        let attachment = Attachment {
            filename: "report.pdf".to_string(),
            content_type: "application/pdf".to_string(),
            size: 1024,
            data: vec![1, 2, 3],
        };

        assert_eq!(
            format!("{}", attachment),
            "report.pdf (application/pdf, 1024 bytes)"
        );
    }

    #[test]
    fn message_display() {
        let message = Message {
            envelope: sample_envelope(),
            text_body: Some("Hello world".to_string()),
            html_body: None,
            attachments: vec![],
            raw: vec![],
        };

        assert_eq!(format!("{}", message), "[1] a@b.com | Test | unknown");
    }

    #[test]
    fn message_serialize() {
        let message = Message {
            envelope: sample_envelope(),
            text_body: Some("Hello world".to_string()),
            html_body: None,
            attachments: vec![],
            raw: vec![],
        };

        let json = serde_json::to_string(&message).unwrap();

        assert!(json.contains("Hello world"));
        assert!(json.contains("Test"));
    }

    #[test]
    fn attachment_fields() {
        let attachment = Attachment {
            filename: "report.pdf".to_string(),
            content_type: "application/pdf".to_string(),
            size: 1024,
            data: vec![1, 2, 3],
        };

        assert_eq!(attachment.size, 1024);
        assert_eq!(attachment.filename, "report.pdf");
    }
}
