//! Content Store Integrity Zome
//!
//! Defines the entry types and validation rules for the Lamad spike.
//! This is a minimal implementation for testing browser-to-conductor connectivity.

use hdi::prelude::*;

/// Content entry - minimal version for spike
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Content {
    /// Unique identifier (matches ContentNode.id in the app)
    pub id: String,
    /// Content title
    pub title: String,
    /// Content body (markdown)
    pub body: String,
    /// Creation timestamp (ISO 8601)
    pub created_at: String,
    /// Author agent public key (base64)
    pub author: String,
}

/// All entry types in this DNA
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    Content(Content),
}

/// Link types for relationships
#[hdk_link_types]
pub enum LinkTypes {
    /// Link from author to their content
    AuthorToContent,
    /// Link from content ID hash to content entry
    IdToContent,
}

/// Validation callback - runs when entries are created/updated
#[hdk_extern]
pub fn validate(_op: Op) -> ExternResult<ValidateCallbackResult> {
    // Minimal validation for spike - accept all valid entries
    // In production: validate content structure, author permissions, etc.
    Ok(ValidateCallbackResult::Valid)
}
