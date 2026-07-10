//! Database schemas for Doorway
//!
//! Defines MongoDB document structures for users, API keys, hosts, OAuth, and caches.

mod api_key;
mod app_file_cache;
pub mod bootstrap_entry;
mod host;
mod metadata;
mod oauth_session;
pub mod signal_bus;
mod user;

pub use api_key::{ApiKeyDoc, API_KEY_COLLECTION};
pub use app_file_cache::{AppFileCacheDoc, APP_FILE_CACHE_COLLECTION};
pub use host::{HostDoc, HostStatus, HOST_COLLECTION};
pub use metadata::Metadata;
pub use oauth_session::{
    get_registered_clients, validate_redirect_uri, OAuthClient, OAuthSessionDoc,
    OAUTH_SESSION_COLLECTION,
};
pub use user::{CustodialKeyMaterial, UserDoc, UserQuota, UserUsage, USER_COLLECTION};
