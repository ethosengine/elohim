//! Database schemas for Doorway
//!
//! Defines MongoDB document structures for users, API keys, hosts, and OAuth.

mod api_key;
mod host;
mod metadata;
mod oauth_session;
mod user;

pub use api_key::{ApiKeyDoc, API_KEY_COLLECTION};
pub use host::{HostDoc, HostStatus, HOST_COLLECTION};
pub use metadata::Metadata;
pub use oauth_session::{
    get_registered_clients, validate_redirect_uri, OAuthClient, OAuthSessionDoc,
    OAUTH_SESSION_COLLECTION,
};
pub use user::{UserDoc, UserQuota, UserUsage, USER_COLLECTION};
