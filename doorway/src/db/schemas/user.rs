//! User document schema
//!
//! Stores user credentials and Holochain identity mappings.

use bson::{doc, oid::ObjectId, Document};
use mongodb::options::IndexOptions;
use serde::{Deserialize, Serialize};

use crate::db::mongo::{IntoIndexes, MutMetadata};
use crate::db::schemas::Metadata;

/// Collection name for users
pub const USER_COLLECTION: &str = "users";

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
}

fn default_identifier_type() -> String {
    "email".to_string()
}

fn default_true() -> bool {
    true
}

impl UserDoc {
    /// Create a new user document
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
        ]
    }
}

impl MutMetadata for UserDoc {
    fn mut_metadata(&mut self) -> &mut Metadata {
        &mut self.metadata
    }
}
