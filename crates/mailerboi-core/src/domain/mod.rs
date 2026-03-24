pub mod envelope;
pub mod flag;
pub mod folder;
pub mod message;

pub use envelope::{Address, Envelope};
pub use flag::Flag;
pub use folder::Folder;
pub use message::{Attachment, Message};
