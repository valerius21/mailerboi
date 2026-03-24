use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Folder {
    pub name: String,
    pub delimiter: Option<String>,
    pub attributes: Vec<String>,
}

impl fmt::Display for Folder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_display() {
        let folder = Folder {
            name: "INBOX".to_string(),
            delimiter: Some("/".to_string()),
            attributes: vec![],
        };

        assert_eq!(format!("{}", folder), "INBOX");
    }

    #[test]
    fn folder_serialize() {
        let folder = Folder {
            name: "Sent".to_string(),
            delimiter: None,
            attributes: vec!["\\Sent".to_string()],
        };

        let json = serde_json::to_string(&folder).unwrap();
        let back: Folder = serde_json::from_str(&json).unwrap();

        assert_eq!(folder, back);
    }
}
