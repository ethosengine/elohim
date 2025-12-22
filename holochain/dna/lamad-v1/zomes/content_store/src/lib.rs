//! Content Store Coordinator Zome (V1 - Previous DNA Version)
//!
//! This is the v1 DNA coordinator zome. It provides the same functionality as v2,
//! but exports data in v1 format for healing by v2.
//!
//! When v2 schema changes occur, v2 queries v1 via bridge calls to retrieve data
//! that hasn't been migrated yet. These export functions enable that process.

use hdk::prelude::*;
use content_store_integrity::*;

// Export functions for v2 to call via bridge
pub mod healing_exports;

// =============================================================================
// Bridge-Callable Export Functions (v2 calls these via bridge_call)
// =============================================================================

/// Check if v1 DNA has any data
/// Called by v2 to determine if healing from v1 is needed
#[hdk_extern]
pub fn is_data_present(_: ()) -> ExternResult<bool> {
    healing_exports::is_data_present(())
}

/// Export a content entry in v1 format
/// Called by v2 when content is not found in v2 DHT
#[hdk_extern]
pub fn export_content_by_id(input: serde_json::Value) -> ExternResult<serde_json::Value> {
    let export = healing_exports::export_content_by_id(input)?;
    Ok(serde_json::to_value(export)?)
}

/// Export a learning path entry in v1 format
#[hdk_extern]
pub fn export_path_by_id(input: serde_json::Value) -> ExternResult<serde_json::Value> {
    let export = healing_exports::export_path_by_id(input)?;
    Ok(serde_json::to_value(export)?)
}

/// Export a path step entry in v1 format
#[hdk_extern]
pub fn export_step_by_id(input: serde_json::Value) -> ExternResult<serde_json::Value> {
    let export = healing_exports::export_step_by_id(input)?;
    Ok(serde_json::to_value(export)?)
}

/// Export a content mastery entry in v1 format
#[hdk_extern]
pub fn export_mastery_by_id(input: serde_json::Value) -> ExternResult<serde_json::Value> {
    let export = healing_exports::export_mastery_by_id(input)?;
    Ok(serde_json::to_value(export)?)
}

// =============================================================================
// V1 Coordinator Functions (stub - implement with actual v1 queries)
// =============================================================================
//
// For a real v1 DNA, these would be the actual coordinator functions
// that query the DHT and manage state. The healing_exports module
// uses these internally to retrieve v1 data.
//
// For now, they're stubs that return errors, indicating the helper
// functions in healing_exports need to be updated with actual queries.
//
// Example of what a real implementation would look like:
//
// pub fn get_content_by_id(id: &str) -> ExternResult<Option<Content>> {
//     let anchor = StringAnchor::new("content_id", id);
//     let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
//     let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToContent)?;
//     let links = get_links(query, GetStrategy::default())?;
//
//     if let Some(link) = links.first() {
//         let action_hash = ActionHash::try_from(link.target.clone())?;
//         // ... get entry from DHT ...
//     }
//     Ok(None)
// }
//
// Update the helper functions in healing_exports.rs to call these.
