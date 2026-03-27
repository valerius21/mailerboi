# mailerboi

Multi-account IMAP email client for the command line.

## Features

- 📧 Manage multiple IMAP accounts from a single CLI
- 🔍 List, search, read, and organize emails
- 📎 Download attachments
- 📝 Create draft messages
- 🔐 Support for TLS and plain IMAP connections
- 📊 Multiple output formats: table, JSON, and TOON (token-efficient)

## Installation

```bash
cargo install --path crates/mailerboi
```

## Configuration

Create `~/.config/mailerboi/config.toml`:

```toml
[accounts.personal]
email = "alice@example.com"
host = "imap.example.com"
port = 993
tls = true
default = true
```

Create `~/.config/mailerboi/credentials.toml`:

```toml
personal = "your-app-password"
```

## Usage

```bash
# List configured accounts
mailerboi list-accounts

# Check connectivity
mailerboi --account personal doctor

# List recent emails
mailerboi --account personal list --mailbox INBOX --limit 20

# Search emails
mailerboi --account personal search --from "alice@example.com" --limit 20

# Read a message
mailerboi --account personal read 1234 --mailbox INBOX

# Create a draft
mailerboi --account personal draft --subject "Hello" --body "Message text" --mailbox Drafts
```

## License

MIT
