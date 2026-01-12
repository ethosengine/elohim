//! User document schema
//!
//! Stores user credentials, Holochain identity mappings, usage tracking, and quotas.

use bson::{doc, oid::ObjectId, DateTime, Document};
use mongodb::options::IndexOptions;
use serde::{Deserialize, Serialize};

use crate::auth::PermissionLevel;
use crate::db::mongo::{IntoIndexes, MutMetadata};
use crate::db::schemas::Metadata;

/// Collection name for users
pub const USER_COLLECTION: &str = "users";

// =============================================================================
// Usage and Quota Types
// =============================================================================

/// Usage tracking for hosted users
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserUsage {
    /// Storage consumed in bytes (blobs, content)
    #[serde(default)]
    pub storage_bytes: u64,

    /// Projection queries this billing period
    #[serde(default)]
    pub projection_queries: u64,

    /// Bandwidth consumed in bytes this period
    #[serde(default)]
    pub bandwidth_bytes: u64,

    /// Period start timestamp (for daily/monthly resets)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period_start: Option<DateTime>,

    /// Last usage update timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<DateTime>,
}

/// Quota limits for hosted users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserQuota {
    /// Storage limit in bytes (0 = unlimited)
    #[serde(default)]
    pub storage_limit: u64,

    /// Projection queries per day (0 = unlimited)
    #[serde(default)]
    pub daily_query_limit: u64,

    /// Bandwidth limit per day in bytes (0 = unlimited)
    #[serde(default)]
    pub daily_bandwidth_limit: u64,

    /// Whether to hard-block or soft-warn on quota exceeded
    #[serde(default)]
    pub enforce_hard_limit: bool,
}

impl Default for UserQuota {
    fn default() -> Self {
        Self {
            storage_limit: 100 * 1024 * 1024,         // 100 MB default
            daily_query_limit: 1000,                  // 1000 queries/day
            daily_bandwidth_limit: 500 * 1024 * 1024, // 500 MB/day
            enforce_hard_limit: false,                // Warn only by default
        }
    }
}

// =============================================================================
// User Document
// =============================================================================

/// User document stored in MongoDB
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct UserDoc {
    /// MongoDB document ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _id: Option<ObjectId>,

    /// Common metadata (created_at, updated_at, is_deleted)
    #[serde(default)]
    pub metadata: Metadata,

    /// User identifier (email or username)
    pub identifier: String,

    /// Type of identifier (email, username, etc.)
    #[serde(default = "default_identifier_type")]
    pub identifier_type: String,

    /// Argon2 password hash
    pub password_hash: String,

    /// Holochain human ID (from register_human zome call)
    pub human_id: String,

    /// Holochain agent public key (hex encoded)
    pub agent_pub_key: String,

    /// Token version for invalidation (increment to invalidate all tokens)
    #[serde(default)]
    pub token_version: i32,

    /// Whether the user account is active
    #[serde(default = "default_true")]
    pub is_active: bool,

    /// Permission level for this user
    #[serde(default = "default_permission_level")]
    pub permission_level: PermissionLevel,

    /// Last login timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login_at: Option<DateTime>,

    /// Usage tracking (storage, queries, bandwidth)
    #[serde(default)]
    pub usage: UserUsage,

    /// Quota limits
    #[serde(default)]
    pub quota: UserQuota,
}

fn default_identifier_type() -> String {
    "email".to_string()
}

fn default_true() -> bool {
    true
}

fn default_permission_level() -> PermissionLevel {
    PermissionLevel::Authenticated
}

impl UserDoc {
    /// Create a new user document with default Authenticated permission
    pub fn new(
        identifier: String,
        identifier_type: String,
        password_hash: String,
        human_id: String,
        agent_pub_key: String,
    ) -> Self {
        Self {
            _id: None,
            metadata: Metadata::new(),
            identifier,
            identifier_type,
            password_hash,
            human_id,
            agent_pub_key,
            token_version: 1,
            is_active: true,
            permission_level: PermissionLevel::Authenticated,
            last_login_at: None,
            usage: UserUsage::default(),
            quota: UserQuota::default(),
        }
    }

    /// Create a new admin user
    pub fn new_admin(
        identifier: String,
        identifier_type: String,
        password_hash: String,
        human_id: String,
        agent_pub_key: String,
    ) -> Self {
        let mut user = Self::new(identifier, identifier_type, password_hash, human_id, agent_pub_key);
        user.permission_level = PermissionLevel::Admin;
        user
    }

    /// Check if user is over any quota limit
    pub fn is_over_quota(&self) -> bool {
        let q = &self.quota;
        let u = &self.usage;

        (q.storage_limit > 0 && u.storage_bytes > q.storage_limit)
            || (q.daily_query_limit > 0 && u.projection_queries > q.daily_query_limit)
            || (q.daily_bandwidth_limit > 0 && u.bandwidth_bytes > q.daily_bandwidth_limit)
    }

    /// Get storage usage as percentage (0-100)
    pub fn storage_percent(&self) -> f64 {
        if self.quota.storage_limit == 0 {
            0.0
        } else {
            (self.usage.storage_bytes as f64 / self.quota.storage_limit as f64) * 100.0
        }
    }

    /// Get query usage as percentage (0-100)
    pub fn queries_percent(&self) -> f64 {
        if self.quota.daily_query_limit == 0 {
            0.0
        } else {
            (self.usage.projection_queries as f64 / self.quota.daily_query_limit as f64) * 100.0
        }
    }

    /// Get bandwidth usage as percentage (0-100)
    pub fn bandwidth_percent(&self) -> f64 {
        if self.quota.daily_bandwidth_limit == 0 {
            0.0
        } else {
            (self.usage.bandwidth_bytes as f64 / self.quota.daily_bandwidth_limit as f64) * 100.0
        }
    }
}

impl IntoIndexes for UserDoc {
    fn into_indices() -> Vec<(Document, Option<IndexOptions>)> {
        vec![
            // Unique index on identifier
            (
                doc! { "identifier": 1 },
                Some(
                    IndexOptions::builder()
                        .unique(true)
                        .name("identifier_unique".to_string())
                        .build(),
                ),
            ),
            // Index on human_id for lookups
            (
                doc! { "human_id": 1 },
                Some(
                    IndexOptions::builder()
                        .name("human_id_index".to_string())
                        .build(),
                ),
            ),
            // Index on agent_pub_key
            (
                doc! { "agent_pub_key": 1 },
                Some(
                    IndexOptions::builder()
                        .name("agent_pub_key_index".to_string())
                        .build(),
                ),
            ),
            // Index on permission_level for admin queries
            (
                doc! { "permission_level": 1 },
                Some(
                    IndexOptions::builder()
                        .name("permission_level_index".to_string())
                        .build(),
                ),
            ),
            // Index on is_active for filtering
            (
                doc! { "is_active": 1 },
                Some(
                    IndexOptions::builder()
                        .name("is_active_index".to_string())
                        .build(),
                ),
            ),
        ]
    }
}

impl MutMetadata for UserDoc {
    fn mut_metadata(&mut self) -> &mut Metadata {
        &mut self.metadata
    }
}
