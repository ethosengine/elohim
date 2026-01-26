//! Common metadata for all documents
//!
//! Tracks creation, update, and soft deletion timestamps.

use bson::DateTime;
use serde::{Deserialize, Serialize};

/// Common metadata for all documents
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Metadata {
    /// Whether this document has been soft-deleted
    #[serde(default)]
    pub is_deleted: bool,

    /// When the document was soft-deleted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime>,

    /// When the document was last updated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime>,

    /// When the document was created
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime>,
}

impl Metadata {
    /// Create new metadata with current timestamp
    pub fn new() -> Self {
        Self {
            is_deleted: false,
            deleted_at: None,
            updated_at: Some(DateTime::now()),
            created_at: Some(DateTime::now()),
        }
    }
}
