//! API Key document schema
//!
//! Stores API keys for programmatic access to Doorway.

use bson::{doc, oid::ObjectId, DateTime, Document};
use mongodb::options::IndexOptions;
use serde::{Deserialize, Serialize};

use crate::auth::PermissionLevel;
use crate::db::mongo::{IntoIndexes, MutMetadata};
use crate::db::schemas::Metadata;

/// Collection name for API keys
pub const API_KEY_COLLECTION: &str = "api_keys";

/// API Key document stored in MongoDB
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ApiKeyDoc {
    /// MongoDB document ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _id: Option<ObjectId>,

    /// Common metadata
    #[serde(default)]
    pub metadata: Metadata,

    /// The API key value (hashed for storage)
    pub key_hash: String,

    /// Human-readable name for the key
    pub name: String,

    /// User ID that owns this key
    pub owner_id: ObjectId,

    /// Permission level granted by this key
    pub permission_level: PermissionLevel,

    /// Optional expiration time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime>,

    /// Last time the key was used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<DateTime>,

    /// Whether the key is active
    #[serde(default = "default_true")]
    pub is_active: bool,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn default_true() -> bool {
    true
}

impl ApiKeyDoc {
    /// Create a new API key document
    pub fn new(
        key_hash: String,
        name: String,
        owner_id: ObjectId,
        permission_level: PermissionLevel,
    ) -> Self {
        Self {
            _id: None,
            metadata: Metadata::new(),
            key_hash,
            name,
            owner_id,
            permission_level,
            expires_at: None,
            last_used_at: None,
            is_active: true,
            description: None,
        }
    }

    /// Check if the key has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at < DateTime::now()
        } else {
            false
        }
    }
}

impl IntoIndexes for ApiKeyDoc {
    fn into_indices() -> Vec<(Document, Option<IndexOptions>)> {
        vec![
            // Unique index on key_hash
            (
                doc! { "key_hash": 1 },
                Some(
                    IndexOptions::builder()
                        .unique(true)
                        .name("key_hash_unique".to_string())
                        .build(),
                ),
            ),
            // Index on owner_id for listing user's keys
            (
                doc! { "owner_id": 1 },
                Some(
                    IndexOptions::builder()
                        .name("owner_id_index".to_string())
                        .build(),
                ),
            ),
        ]
    }
}

impl MutMetadata for ApiKeyDoc {
    fn mut_metadata(&mut self) -> &mut Metadata {
        &mut self.metadata
    }
}
