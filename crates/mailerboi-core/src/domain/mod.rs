//! Email domain types.
//!
//! Pure data structures with no IMAP logic - serializable, displayable, and cloneable.

/// Message envelope types and email addresses.
pub mod envelope;
/// IMAP system and custom flags.
pub mod flag;
/// Mailbox and folder metadata.
pub mod folder;
/// Full message payloads and attachments.
pub mod message;

pub use envelope::{Address, Envelope};
pub use flag::Flag;
pub use folder::Folder;
pub use message::{Attachment, Message};
