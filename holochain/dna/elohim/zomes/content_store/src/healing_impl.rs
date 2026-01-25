//! Self-Healing DNA Implementation for Lamad
//!
//! Provides transformation functions and helper utilities for the self-healing pattern.
//! The actual SelfHealingEntry trait implementations are defined in the integrity crate
//! to avoid Rust's orphan rule violations.
//!
//! This module handles:
//! - V1 → V2 data transformation
//! - Healing orchestrator setup
//! - Healing initialization and signals

use hdk::prelude::*;
use content_store_integrity::*;
use hc_rna::{HealingOrchestrator, HealingSignal};

// ============================================================================
// V1 → V2 Transformation Functions
// ============================================================================

/// V1 Content export format
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

pub fn transform_content_v1_to_v2(v1: ContentV1Export) -> Content {
    Content {
        id: v1.id,
        content_type: v1.content_type,
        title: v1.title,
        description: v1.description,
        summary: v1.summary,
        content: v1.content,
        content_format: v1.content_format,
        tags: v1.tags,
        source_path: v1.source_path,
        related_node_ids: v1.related_node_ids,
        author_id: v1.author_id,
        reach: v1.reach,
        trust_score: v1.trust_score,
        estimated_minutes: v1.estimated_minutes,
        thumbnail_url: v1.thumbnail_url,
        metadata_json: v1.metadata_json,
        created_at: v1.created_at,
        updated_at: v1.updated_at,
        schema_version: 2,  // Current version
        validation_status: "Migrated".to_string(),
        // V1 content has body in content field, no blob storage
        blob_cid: None,
        content_size_bytes: None,
        content_hash: None,
    }
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

pub fn transform_learning_path_v1_to_v2(v1: LearningPathV1Export) -> LearningPath {
    LearningPath {
        id: v1.id,
        version: v1.version,
        title: v1.title,
        description: v1.description,
        purpose: v1.purpose,
        created_by: v1.created_by,
        difficulty: v1.difficulty,
        estimated_duration: v1.estimated_duration,
        visibility: v1.visibility,
        path_type: v1.path_type,
        tags: v1.tags,
        metadata_json: v1.metadata_json,
        created_at: v1.created_at,
        updated_at: v1.updated_at,
        schema_version: 2,
        validation_status: "Migrated".to_string(),
    }
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

pub fn transform_path_step_v1_to_v2(v1: PathStepV1Export) -> PathStep {
    PathStep {
        id: v1.id,
        path_id: v1.path_id,
        chapter_id: v1.chapter_id,
        module_id: None,  // New field in V2 - not present in V1
        section_id: None, // New field in V2 - not present in V1
        order_index: v1.order_index,
        step_type: v1.step_type,
        resource_id: v1.resource_id,
        step_title: v1.step_title,
        step_narrative: v1.step_narrative,
        is_optional: v1.is_optional,
        learning_objectives_json: v1.learning_objectives_json,
        reflection_prompts_json: v1.reflection_prompts_json,
        practice_exercises_json: v1.practice_exercises_json,
        estimated_minutes: v1.estimated_minutes,
        completion_criteria: v1.completion_criteria,
        attestation_required: v1.attestation_required,
        attestation_granted: v1.attestation_granted,
        mastery_threshold: v1.mastery_threshold,
        metadata_json: v1.metadata_json,
        created_at: v1.created_at,
        updated_at: v1.updated_at,
        schema_version: 2,
        validation_status: "Migrated".to_string(),
    }
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

pub fn transform_content_mastery_v1_to_v2(v1: ContentMasteryV1Export) -> ContentMastery {
    ContentMastery {
        id: v1.id,
        human_id: v1.human_id,
        content_id: v1.content_id,
        mastery_level: v1.mastery_level,
        mastery_level_index: v1.mastery_level_index,
        freshness_score: v1.freshness_score,
        needs_refresh: v1.needs_refresh,
        engagement_count: v1.engagement_count,
        last_engagement_type: v1.last_engagement_type,
        last_engagement_at: v1.last_engagement_at,
        level_achieved_at: v1.level_achieved_at,
        content_version_at_mastery: v1.content_version_at_mastery,
        assessment_evidence_json: v1.assessment_evidence_json,
        privileges_json: v1.privileges_json,
        created_at: v1.created_at,
        updated_at: v1.updated_at,
        schema_version: 2,
        validation_status: "Migrated".to_string(),
    }
}

// ============================================================================
// Healing Orchestrator Setup
// ============================================================================

pub fn create_healing_orchestrator() -> HealingOrchestrator {
    HealingOrchestrator::new(
        "lamad-v1",  // Previous DNA role name
        "lamad-v2",  // Current DNA role name
    )
}

// ============================================================================
// Initialization
// ============================================================================

pub fn init_healing() -> ExternResult<()> {
    let orchestrator = create_healing_orchestrator();

    // Check if v1 has data
    match orchestrator.check_v1_on_startup()? {
        Some(has_data) => {
            if has_data {
                debug_log("Init: v1 DNA available with data, will heal on demand")?;
                // Healing will happen lazily when entries are queried
            } else {
                debug_log("Init: v1 DNA available but empty")?;
            }
        }
        None => {
            debug_log("Init: no v1 bridge, fresh start")?;
        }
    }

    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Debug logging
pub fn debug_log(msg: &str) -> ExternResult<()> {
    // In production, use proper logging
    Ok(())
}

/// Emit a healing signal
pub fn emit_healing_signal(signal: HealingSignal) -> ExternResult<()> {
    hc_rna::emit_healing_signal(signal)
}

/// Internal helper to get content by ID (simplified for example)
/// In real implementation, would use DHT queries
fn get_content_by_id_internal(id: &str) -> ExternResult<Option<Content>> {
    // This would use actual DHT queries in production
    // For now, return Ok(Some(_)) to avoid blocking healing
    Ok(None)
}

/// Internal helper to get path by ID
fn get_path_by_id_internal(id: &str) -> ExternResult<Option<LearningPath>> {
    // This would use actual DHT queries in production
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_v1_transformation() {
        let v1 = ContentV1Export {
            id: "test".to_string(),
            content_type: "lesson".to_string(),
            title: "V1 Content".to_string(),
            description: "Old version".to_string(),
            summary: None,
            content: "Body".to_string(),
            content_format: "markdown".to_string(),
            tags: vec![],
            source_path: None,
            related_node_ids: vec![],
            author_id: None,
            reach: "commons".to_string(),
            trust_score: 0.9,
            estimated_minutes: None,
            thumbnail_url: None,
            metadata_json: "{}".to_string(),
            created_at: "2025-01-01".to_string(),
            updated_at: "2025-01-01".to_string(),
        };

        let v2 = transform_content_v1_to_v2(v1);
        assert_eq!(v2.schema_version, 2);
        assert_eq!(v2.validation_status, "Migrated");
    }
}
