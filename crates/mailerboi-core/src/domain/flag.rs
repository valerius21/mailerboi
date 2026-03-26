//! IMAP message flags.

use serde::{Deserialize, Serialize};
use std::fmt;

/// An IMAP system flag or provider-specific custom flag.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Flag {
    /// The message has been read.
    Seen,
    /// The message has been answered.
    Answered,
    /// The message is marked for follow-up.
    Flagged,
    /// The message is marked for deletion.
    Deleted,
    /// The message is saved as a draft.
    Draft,
    /// A server-defined flag not covered by the standard variants.
    Custom(String),
}

impl Flag {
    /// Parses an IMAP flag name into a [`Flag`].
    pub fn from_imap_str(s: &str) -> Self {
        match s {
            "\\Seen" => Flag::Seen,
            "\\Answered" => Flag::Answered,
            "\\Flagged" => Flag::Flagged,
            "\\Deleted" => Flag::Deleted,
            "\\Draft" => Flag::Draft,
            other => Flag::Custom(other.to_string()),
        }
    }

    /// Returns the IMAP wire representation for this flag.
    pub fn to_imap_str(&self) -> &str {
        match self {
            Flag::Seen => "\\Seen",
            Flag::Answered => "\\Answered",
            Flag::Flagged => "\\Flagged",
            Flag::Deleted => "\\Deleted",
            Flag::Draft => "\\Draft",
            Flag::Custom(value) => value,
        }
    }
}

impl fmt::Display for Flag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Flag::Seen => write!(f, "Seen"),
            Flag::Answered => write!(f, "Answered"),
            Flag::Flagged => write!(f, "Flagged"),
            Flag::Deleted => write!(f, "Deleted"),
            Flag::Draft => write!(f, "Draft"),
            Flag::Custom(value) => write!(f, "{}", value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_from_imap_str() {
        assert_eq!(Flag::from_imap_str("\\Seen"), Flag::Seen);
        assert_eq!(Flag::from_imap_str("\\Flagged"), Flag::Flagged);
        assert_eq!(Flag::from_imap_str("\\Draft"), Flag::Draft);
        assert_eq!(
            Flag::from_imap_str("custom-label"),
            Flag::Custom("custom-label".to_string())
        );
    }

    #[test]
    fn flag_to_imap_str() {
        assert_eq!(Flag::Seen.to_imap_str(), "\\Seen");
        assert_eq!(Flag::Draft.to_imap_str(), "\\Draft");
        assert_eq!(Flag::Custom("myLabel".to_string()).to_imap_str(), "myLabel");
    }

    #[test]
    fn flag_display() {
        assert_eq!(format!("{}", Flag::Seen), "Seen");
        assert_eq!(format!("{}", Flag::Custom("foo".to_string())), "foo");
    }

    #[test]
    fn flag_to_imap_str_all_variants() {
        assert_eq!(Flag::Answered.to_imap_str(), "\\Answered");
        assert_eq!(Flag::Flagged.to_imap_str(), "\\Flagged");
        assert_eq!(Flag::Deleted.to_imap_str(), "\\Deleted");
    }

    #[test]
    fn flag_display_all_variants() {
        assert_eq!(format!("{}", Flag::Answered), "Answered");
        assert_eq!(format!("{}", Flag::Flagged), "Flagged");
        assert_eq!(format!("{}", Flag::Deleted), "Deleted");
        assert_eq!(format!("{}", Flag::Draft), "Draft");
    }

    #[test]
    fn flag_from_imap_str_answered_deleted() {
        assert_eq!(Flag::from_imap_str("\\Answered"), Flag::Answered);
        assert_eq!(Flag::from_imap_str("\\Deleted"), Flag::Deleted);
    }
}
