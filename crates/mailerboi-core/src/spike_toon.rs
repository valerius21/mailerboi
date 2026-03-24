#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde::{Deserialize, Serialize};
    use toon_format::{decode_default, encode_default};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestConfig {
        display_name: String,
        default_account: bool,
        host: String,
        port: u16,
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestAccount {
        email: String,
        host: String,
        port: u16,
        tls: bool,
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestRoot {
        accounts: HashMap<String, TestAccount>,
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestOptionConfig {
        name: String,
        description: Option<String>,
    }

    #[test]
    fn spike_toon_simple_roundtrip() {
        let input = TestConfig {
            display_name: "Primary Account".to_string(),
            default_account: true,
            host: "imap.example.com".to_string(),
            port: 993,
        };

        let toon = encode_default(&input).expect("encode_default should encode TestConfig");
        let decoded: TestConfig =
            decode_default(&toon).expect("decode_default should decode TestConfig");

        assert_eq!(decoded, input);
    }

    #[test]
    fn spike_toon_nested_roundtrip() {
        let mut accounts = HashMap::new();
        accounts.insert(
            "primary".to_string(),
            TestAccount {
                email: "primary@example.com".to_string(),
                host: "imap.primary.example.com".to_string(),
                port: 993,
                tls: true,
            },
        );
        accounts.insert(
            "backup".to_string(),
            TestAccount {
                email: "backup@example.com".to_string(),
                host: "imap.backup.example.com".to_string(),
                port: 143,
                tls: false,
            },
        );

        let input = TestRoot { accounts };

        let toon = encode_default(&input).expect("encode_default should encode nested structs");
        let decoded: TestRoot =
            decode_default(&toon).expect("decode_default should decode nested structs");

        assert_eq!(decoded, input);
    }

    #[test]
    fn spike_toon_special_chars() {
        let input = TestConfig {
            display_name: "Hello, World! h\u{00E9}llo: value,with,comma".to_string(),
            default_account: false,
            host: "imap.example.com".to_string(),
            port: 993,
        };

        let toon = encode_default(&input).expect("encode_default should preserve special chars");
        let decoded: TestConfig =
            decode_default(&toon).expect("decode_default should parse special chars");

        assert_eq!(decoded, input);
    }

    #[test]
    fn spike_toon_option_fields() {
        let some_value = TestOptionConfig {
            name: "some-case".to_string(),
            description: Some("optional description".to_string()),
        };
        let none_value = TestOptionConfig {
            name: "none-case".to_string(),
            description: None,
        };

        let some_toon =
            encode_default(&some_value).expect("encode_default should encode Option::Some");
        let decoded_some: TestOptionConfig =
            decode_default(&some_toon).expect("decode_default should decode Option::Some");
        assert_eq!(decoded_some, some_value);

        let none_toon =
            encode_default(&none_value).expect("encode_default should encode Option::None");
        let decoded_none: TestOptionConfig =
            decode_default(&none_toon).expect("decode_default should decode Option::None");
        assert_eq!(decoded_none, none_value);
    }
}
