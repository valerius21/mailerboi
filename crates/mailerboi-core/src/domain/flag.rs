use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Flag {
    Seen,
    Answered,
    Flagged,
    Deleted,
    Draft,
    Custom(String),
}

impl Flag {
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
}
