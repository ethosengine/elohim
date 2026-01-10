//! Database schemas for Doorway
//!
//! Defines MongoDB document structures for users, API keys, and hosts.

mod api_key;
mod host;
mod metadata;
mod user;

pub use api_key::{ApiKeyDoc, API_KEY_COLLECTION};
pub use host::{HostDoc, HostStatus, HOST_COLLECTION};
pub use metadata::Metadata;
pub use user::{UserDoc, USER_COLLECTION};
