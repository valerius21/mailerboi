use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use mailerboi_core::output::OutputFormat;

#[derive(Parser, Debug)]
#[command(name = "mailerboi", about = "Multi-account IMAP email CLI", version)]
pub struct Cli {
    /// Path to config file (default: ~/.config/mailerboi/config.toml)
    #[arg(short, long, global = true, env = "MAILERBOI_CONFIG")]
    pub config: Option<PathBuf>,

    /// Account name to use (default: first or marked default)
    #[arg(short, long, global = true)]
    pub account: Option<String>,

    /// Output format
    #[arg(
        short,
        long,
        global = true,
        default_value = "table",
        value_parser = clap::value_parser!(OutputFormat)
    )]
    pub output: OutputFormat,

    /// Accept invalid TLS certificates (for self-signed certs)
    #[arg(long, global = true)]
    pub insecure: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List all configured accounts
    ListAccounts,

    /// Test IMAP connectivity for an account
    Doctor,

    /// Show unread message count per account/mailbox
    Check {
        /// Mailbox to check (default: INBOX)
        #[arg(short, long, default_value = "INBOX")]
        mailbox: String,
    },

    /// List all mailboxes/folders
    Folders,

    /// List emails in a mailbox
    List {
        /// Mailbox to list (default: INBOX)
        #[arg(short, long, default_value = "INBOX")]
        mailbox: String,
        /// Maximum number of results
        #[arg(short, long, default_value = "20")]
        limit: u32,
        /// Page number (1-indexed)
        #[arg(short, long, default_value = "1")]
        page: u32,
    },

    /// Read a specific email by UID
    Read {
        /// Message UID
        uid: u32,
        /// Mailbox containing the message
        #[arg(short, long, default_value = "INBOX")]
        mailbox: String,
        /// Display format
        #[arg(short, long, default_value = "text", value_enum)]
        format: ReadFormat,
    },

    /// Search emails with filters
    Search {
        /// Only unread messages
        #[arg(long)]
        unseen: bool,
        /// Only read messages
        #[arg(long)]
        seen: bool,
        /// Filter by sender (contains)
        #[arg(long)]
        from: Option<String>,
        /// Filter by subject (contains)
        #[arg(long)]
        subject: Option<String>,
        /// Messages since date (YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,
        /// Messages before date (YYYY-MM-DD)
        #[arg(long)]
        before: Option<String>,
        /// Messages from last N time (e.g. 2h, 7d)
        #[arg(long)]
        recent: Option<String>,
        /// Maximum results
        #[arg(short, long, default_value = "20")]
        limit: u32,
        /// Mailbox to search
        #[arg(short, long, default_value = "INBOX")]
        mailbox: String,
    },

    /// Move a message to another folder
    Move {
        /// Message UID
        uid: u32,
        /// Target folder name
        target: String,
        /// Source mailbox
        #[arg(short, long, default_value = "INBOX")]
        mailbox: String,
    },

    /// Delete a message (moves to Trash by default)
    Delete {
        /// Message UID
        uid: u32,
        /// Permanently delete (skip Trash)
        #[arg(long)]
        force: bool,
        /// Source mailbox
        #[arg(short, long, default_value = "INBOX")]
        mailbox: String,
    },

    /// Set or unset flags on a message
    Flag {
        /// Message UID(s)
        #[arg(required = true)]
        uids: Vec<u32>,
        /// Flag to set (seen, flagged, answered, draft)
        #[arg(long)]
        set: Option<String>,
        /// Flag to unset
        #[arg(long)]
        unset: Option<String>,
        /// Mark as read (shorthand for --set seen)
        #[arg(long)]
        read: bool,
        /// Mark as unread (shorthand for --unset seen)
        #[arg(long)]
        unread: bool,
        /// Source mailbox
        #[arg(short, long, default_value = "INBOX")]
        mailbox: String,
    },

    /// Download attachments from a message
    Download {
        /// Message UID
        uid: u32,
        /// Output directory (default: current directory)
        #[arg(short, long)]
        dir: Option<PathBuf>,
        /// Download only this attachment filename
        #[arg(short, long)]
        file: Option<String>,
        /// Source mailbox
        #[arg(short, long, default_value = "INBOX")]
        mailbox: String,
    },

    /// Create a draft message
    Draft {
        /// Email subject
        #[arg(short, long)]
        subject: String,
        /// Email body text
        #[arg(short, long)]
        body: Option<String>,
        /// Read body from file
        #[arg(long)]
        body_file: Option<PathBuf>,
        /// Drafts folder name (default: Drafts)
        #[arg(short, long, default_value = "Drafts")]
        mailbox: String,
    },
}

#[derive(ValueEnum, Debug, Clone, Default)]
pub enum ReadFormat {
    #[default]
    Text,
    Html,
    Raw,
    Headers,
}
