//! V1 DNA Export Functions for Self-Healing Bridge Calls
//!
//! This module provides functions that v2 DNA can call via bridge to retrieve v1 data.
//! These exports enable v2 to heal from v1 when schema changes occur.
//!
//! Add these functions to the v1 DNA coordinator zome. They're called by v2 via:
//!   hc_rna::bridge_call(
//!       "lamad-v1",
//!       "coordinator",
//!       "export_content_by_id",
//!       serde_json::json!({ "id": "some-id" })
//!   )

use hdk::prelude::*;
use content_store_integrity::*;

// ============================================================================
// BRIDGE-CALLABLE EXPORTS (use #[hdk_extern] for these)
// ============================================================================

/// Check if v1 DNA has any data
/// Returns true if there are content, paths, or mastery entries
#[hdk_extern]
pub fn is_data_present(_: ()) -> ExternResult<bool> {
    // Try to find any content entry
    match get_all_content() {
        Ok(entries) if !entries.is_empty() => return Ok(true),
        _ => {}
    }

    // Try to find any path entry
    match get_all_paths() {
        Ok(entries) if !entries.is_empty() => return Ok(true),
        _ => {}
    }

    // Try to find any mastery entry
    match get_all_mastery() {
        Ok(entries) if !entries.is_empty() => return Ok(true),
        _ => {}
    }

    Ok(false)
}

/// Export a content entry in v1 format (to be transformed by v2)
#[hdk_extern]
pub fn export_content_by_id(input: serde_json::Value) -> ExternResult<ContentV1Export> {
    let id: String = input
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Missing or invalid 'id' parameter".to_string()
        )))?
        .to_string();

    let content = get_content_by_id_internal(&id)?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            format!("Content {} not found", id)
        )))?;

    Ok(ContentV1Export {
        id: content.id,
        content_type: content.content_type,
        title: content.title,
        description: content.description,
        summary: content.summary,
        content: content.content,
        content_format: content.content_format,
        tags: content.tags,
        source_path: content.source_path,
        related_node_ids: content.related_node_ids,
        author_id: content.author_id,
        reach: content.reach,
        trust_score: content.trust_score,
        estimated_minutes: content.estimated_minutes,
        thumbnail_url: content.thumbnail_url,
        metadata_json: content.metadata_json,
        created_at: content.created_at,
        updated_at: content.updated_at,
    })
}

/// Export a learning path in v1 format
#[hdk_extern]
pub fn export_path_by_id(input: serde_json::Value) -> ExternResult<LearningPathV1Export> {
    let id: String = input
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Missing or invalid 'id' parameter".to_string()
        )))?
        .to_string();

    let path = get_path_by_id_internal(&id)?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            format!("Path {} not found", id)
        )))?;

    Ok(LearningPathV1Export {
        id: path.id,
        version: path.version,
        title: path.title,
        description: path.description,
        purpose: path.purpose,
        created_by: path.created_by,
        difficulty: path.difficulty,
        estimated_duration: path.estimated_duration,
        visibility: path.visibility,
        path_type: path.path_type,
        tags: path.tags,
        metadata_json: path.metadata_json,
        created_at: path.created_at,
        updated_at: path.updated_at,
    })
}

/// Export a path step in v1 format
#[hdk_extern]
pub fn export_step_by_id(input: serde_json::Value) -> ExternResult<PathStepV1Export> {
    let id: String = input
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Missing or invalid 'id' parameter".to_string()
        )))?
        .to_string();

    let step = get_step_by_id_internal(&id)?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            format!("Step {} not found", id)
        )))?;

    Ok(PathStepV1Export {
        id: step.id,
        path_id: step.path_id,
        chapter_id: step.chapter_id,
        order_index: step.order_index,
        step_type: step.step_type,
        resource_id: step.resource_id,
        step_title: step.step_title,
        step_narrative: step.step_narrative,
        is_optional: step.is_optional,
        learning_objectives_json: step.learning_objectives_json,
        reflection_prompts_json: step.reflection_prompts_json,
        practice_exercises_json: step.practice_exercises_json,
        estimated_minutes: step.estimated_minutes,
        completion_criteria: step.completion_criteria,
        attestation_required: step.attestation_required,
        attestation_granted: step.attestation_granted,
        mastery_threshold: step.mastery_threshold,
        metadata_json: step.metadata_json,
        created_at: step.created_at,
        updated_at: step.updated_at,
    })
}

/// Export a content mastery entry in v1 format
#[hdk_extern]
pub fn export_mastery_by_id(input: serde_json::Value) -> ExternResult<ContentMasteryV1Export> {
    let id: String = input
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Missing or invalid 'id' parameter".to_string()
        )))?
        .to_string();

    let mastery = get_mastery_by_id_internal(&id)?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            format!("Mastery {} not found", id)
        )))?;

    Ok(ContentMasteryV1Export {
        id: mastery.id,
        human_id: mastery.human_id,
        content_id: mastery.content_id,
        mastery_level: mastery.mastery_level,
        mastery_level_index: mastery.mastery_level_index,
        freshness_score: mastery.freshness_score,
        needs_refresh: mastery.needs_refresh,
        engagement_count: mastery.engagement_count,
        last_engagement_type: mastery.last_engagement_type,
        last_engagement_at: mastery.last_engagement_at,
        level_achieved_at: mastery.level_achieved_at,
        content_version_at_mastery: mastery.content_version_at_mastery,
        assessment_evidence_json: mastery.assessment_evidence_json,
        privileges_json: mastery.privileges_json,
        created_at: mastery.created_at,
        updated_at: mastery.updated_at,
    })
}

// ============================================================================
// EXPORT TYPE STRUCTURES (used in healing_impl.rs in v2)
// ============================================================================

/// V1 Content export format (same as in healing_impl.rs)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContentV1Export {
    pub id: String,
    pub content_type: String,
    pub title: String,
    pub description: String,
    pub summary: Option<String>,
    pub content: String,
    pub content_format: String,
    pub tags: Vec<String>,
    pub source_path: Option<String>,
    pub related_node_ids: Vec<String>,
    pub author_id: Option<String>,
    pub reach: String,
    pub trust_score: f64,
    pub estimated_minutes: Option<u32>,
    pub thumbnail_url: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// V1 LearningPath export format
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LearningPathV1Export {
    pub id: String,
    pub version: String,
    pub title: String,
    pub description: String,
    pub purpose: Option<String>,
    pub created_by: String,
    pub difficulty: String,
    pub estimated_duration: Option<String>,
    pub visibility: String,
    pub path_type: String,
    pub tags: Vec<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// V1 PathStep export format
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PathStepV1Export {
    pub id: String,
    pub path_id: String,
    pub chapter_id: Option<String>,
    pub order_index: u32,
    pub step_type: String,
    pub resource_id: String,
    pub step_title: Option<String>,
    pub step_narrative: Option<String>,
    pub is_optional: bool,
    pub learning_objectives_json: String,
    pub reflection_prompts_json: String,
    pub practice_exercises_json: String,
    pub estimated_minutes: Option<u32>,
    pub completion_criteria: Option<String>,
    pub attestation_required: Option<String>,
    pub attestation_granted: Option<String>,
    pub mastery_threshold: Option<u32>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// V1 ContentMastery export format
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContentMasteryV1Export {
    pub id: String,
    pub human_id: String,
    pub content_id: String,
    pub mastery_level: String,
    pub mastery_level_index: u32,
    pub freshness_score: f64,
    pub needs_refresh: bool,
    pub engagement_count: u32,
    pub last_engagement_type: String,
    pub last_engagement_at: String,
    pub level_achieved_at: String,
    pub content_version_at_mastery: Option<String>,
    pub assessment_evidence_json: String,
    pub privileges_json: String,
    pub created_at: String,
    pub updated_at: String,
}

// ============================================================================
// INTERNAL HELPER FUNCTIONS (use existing query functions from coordinator)
// ============================================================================

/// Get content by ID using DHT anchor/link pattern
fn get_content_by_id_internal(id: &str) -> ExternResult<Option<Content>> {
    // Query the DHT for content by ID using the standard anchor/link pattern
    let anchor = StringAnchor::new("content_id", id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToContent)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash".to_string())))?;

        let record = get(action_hash, GetOptions::default())?;
        if let Some(record) = record {
            let content: Content = record
                .entry()
                .to_app_option()
                .map_err(|e| wasm_error!(e))?
                .ok_or(wasm_error!(WasmErrorInner::Guest("Could not deserialize content".to_string())))?;

            return Ok(Some(content));
        }
    }

    Ok(None)
}

/// Get learning path by ID
fn get_path_by_id_internal(id: &str) -> ExternResult<Option<LearningPath>> {
    // Same pattern as content, but for LearningPath
    let anchor = StringAnchor::new("path_id", id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToPath)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash".to_string())))?;

        let record = get(action_hash, GetOptions::default())?;
        if let Some(record) = record {
            let path: LearningPath = record
                .entry()
                .to_app_option()
                .map_err(|e| wasm_error!(e))?
                .ok_or(wasm_error!(WasmErrorInner::Guest("Could not deserialize path".to_string())))?;

            return Ok(Some(path));
        }
    }

    Ok(None)
}

/// Get path step by ID
fn get_step_by_id_internal(id: &str) -> ExternResult<Option<PathStep>> {
    // Same pattern as others, for PathStep
    let anchor = StringAnchor::new("step_id", id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToPathStep)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash".to_string())))?;

        let record = get(action_hash, GetOptions::default())?;
        if let Some(record) = record {
            let step: PathStep = record
                .entry()
                .to_app_option()
                .map_err(|e| wasm_error!(e))?
                .ok_or(wasm_error!(WasmErrorInner::Guest("Could not deserialize step".to_string())))?;

            return Ok(Some(step));
        }
    }

    Ok(None)
}

/// Get content mastery by ID
fn get_mastery_by_id_internal(id: &str) -> ExternResult<Option<ContentMastery>> {
    // Same pattern as others, for ContentMastery
    let anchor = StringAnchor::new("mastery_id", id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToContentMastery)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash".to_string())))?;

        let record = get(action_hash, GetOptions::default())?;
        if let Some(record) = record {
            let mastery: ContentMastery = record
                .entry()
                .to_app_option()
                .map_err(|e| wasm_error!(e))?
                .ok_or(wasm_error!(WasmErrorInner::Guest("Could not deserialize mastery".to_string())))?;

            return Ok(Some(mastery));
        }
    }

    Ok(None)
}

/// Get all content entries (for is_data_present check)
fn get_all_content() -> ExternResult<Vec<Content>> {
    // Query for any content entries
    // Replace with actual DHT query implementation
    Ok(Vec::new())
}

/// Get all learning paths
fn get_all_paths() -> ExternResult<Vec<LearningPath>> {
    // Replace with actual DHT query implementation
    Ok(Vec::new())
}

/// Get all content mastery entries
fn get_all_mastery() -> ExternResult<Vec<ContentMastery>> {
    // Replace with actual DHT query implementation
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_types_serialize() {
        let content_export = ContentV1Export {
            id: "test".to_string(),
            content_type: "concept".to_string(),
            title: "Test".to_string(),
            description: "Test content".to_string(),
            summary: None,
            content: "Body".to_string(),
            content_format: "markdown".to_string(),
            tags: vec![],
            source_path: None,
            related_node_ids: vec![],
            author_id: None,
            reach: "public".to_string(),
            trust_score: 0.8,
            estimated_minutes: Some(10),
            thumbnail_url: None,
            metadata_json: "{}".to_string(),
            created_at: "2025-01-01".to_string(),
            updated_at: "2025-01-01".to_string(),
        };

        // Should serialize without error
        let json = serde_json::to_string(&content_export).expect("Should serialize");
        assert!(!json.is_empty());

        // Should deserialize back
        let _deserialized: ContentV1Export =
            serde_json::from_str(&json).expect("Should deserialize");
    }
}
