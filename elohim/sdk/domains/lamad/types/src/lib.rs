//! Wire types for lamad (learning) domain coordinator functions.
//!
//! These types define the MessagePack-serialized inputs and outputs for
//! content_store zome calls related to learning content. They are consumed by:
//! - The content_store coordinator zome (WASM target)
//! - Doorway gateway service (native target)
//! - elohim-storage (native target)
//!
//! This crate is an IoC artifact in `sdk/domains/lamad/`, alongside
//! the domain's schemas and manifest. It must NOT depend on HDK, HDI,
//! or any WASM-specific crates.

use holo_hash::{ActionHash, EntryHash};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Content CRUD Types
// =============================================================================

/// Input for content_store::create_content coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateContentInput {
    pub id: String,
    pub content_type: String,
    pub title: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub content: String,
    pub content_format: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(default)]
    pub related_node_ids: Vec<String>,
    pub reach: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_minutes: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    pub metadata_json: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blob_cid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_size_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
}

/// Content wire type. Mirrors the integrity zome's Content entry type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Content {
    pub id: String,
    pub content_type: String,
    pub title: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub content: String,
    pub content_format: String,
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    pub related_node_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_id: Option<String>,
    pub reach: String,
    pub trust_score: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_minutes: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub validation_status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blob_cid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_size_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
}

/// Output from content_store::get_content coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ContentOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
    pub content: Content,
}

/// Input for bulk content creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BulkCreateContentInput {
    pub import_id: String,
    pub contents: Vec<CreateContentInput>,
}

/// Output from bulk content creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BulkCreateContentOutput {
    pub import_id: String,
    pub created_count: u32,
    pub action_hashes: Vec<ActionHash>,
    pub errors: Vec<String>,
}

/// Input for querying content by ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QueryByIdInput {
    pub id: String,
}

/// Input for querying content by type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QueryByTypeInput {
    pub content_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// Input for batch ID existence check.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CheckIdsExistInput {
    pub ids: Vec<String>,
}

/// Output for batch ID existence check.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CheckIdsExistOutput {
    pub existing_ids: Vec<String>,
}

/// Input for batch content retrieval by IDs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BatchGetContentInput {
    pub ids: Vec<String>,
}

/// Output for batch content retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BatchGetContentOutput {
    pub found: Vec<ContentOutput>,
    pub not_found: Vec<String>,
}

/// Input for paginated content query by type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PaginatedByTypeInput {
    pub content_type: String,
    pub page_size: u32,
    pub offset: u32,
}

/// Input for paginated content query by tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PaginatedByTagInput {
    pub tag: String,
    pub page_size: u32,
    pub offset: u32,
}

/// Output for paginated content query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PaginatedContentOutput {
    pub items: Vec<ContentOutput>,
    pub total_count: u32,
    pub offset: u32,
    pub has_more: bool,
}

/// Content statistics output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ContentStats {
    pub total_count: u32,
    pub by_type: HashMap<String, u32>,
}

// =============================================================================
// Content Relationship Types
// =============================================================================

/// Input for creating a content relationship.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateRelationshipInput {
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: String,
    pub confidence: f64,
    pub inference_source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_json: Option<String>,
}

/// Relationship wire type. Mirrors the integrity zome's Relationship entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Relationship {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: String,
    pub confidence: f64,
    pub inference_source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_json: Option<String>,
    pub created_at: String,
}

/// Output for relationship operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RelationshipOutput {
    pub action_hash: ActionHash,
    pub relationship: Relationship,
}

/// Input for querying relationships by content ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetRelationshipsInput {
    pub content_id: String,
    pub direction: String,
}

/// Input for querying related content with traversal.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QueryRelatedContentInput {
    pub content_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relationship_types: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<u32>,
}

/// Content graph node for tree traversal.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ContentGraphNode {
    pub content: ContentOutput,
    pub relationship_type: String,
    pub confidence: f64,
    pub children: Vec<ContentGraphNode>,
}

/// Content graph output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ContentGraph {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<ContentOutput>,
    pub related: Vec<ContentGraphNode>,
    pub total_nodes: u32,
}

// =============================================================================
// Learning Path Types
// =============================================================================

/// LearningPath wire type. Mirrors the integrity zome's LearningPath entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LearningPath {
    pub id: String,
    pub version: String,
    pub title: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    pub created_by: String,
    pub difficulty: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_duration: Option<String>,
    pub visibility: String,
    pub path_type: String,
    pub tags: Vec<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub validation_status: String,
}

/// PathStep wire type. Mirrors the integrity zome's PathStep entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PathStep {
    pub id: String,
    pub path_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapter_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_id: Option<String>,
    pub order_index: u32,
    pub step_type: String,
    pub resource_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_narrative: Option<String>,
    pub is_optional: bool,
    pub learning_objectives_json: String,
    pub reflection_prompts_json: String,
    pub practice_exercises_json: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_minutes: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_criteria: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attestation_required: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attestation_granted: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mastery_threshold: Option<u32>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub validation_status: String,
}

/// Input for creating a learning path.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreatePathInput {
    pub id: String,
    pub version: String,
    pub title: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    pub difficulty: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_duration: Option<String>,
    pub visibility: String,
    pub path_type: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_json: Option<String>,
}

/// Input for bulk path import (from seeder). Includes embedded steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PathImportInput {
    pub id: String,
    pub version: String,
    pub title: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    pub difficulty: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_duration: Option<String>,
    pub visibility: String,
    pub path_type: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_json: Option<String>,
    #[serde(default)]
    pub steps: Vec<PathImportStepInput>,
}

/// Step input embedded in PathImportInput.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PathImportStepInput {
    pub step_type: String,
    pub resource_id: String,
    pub order_index: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_narrative: Option<String>,
    #[serde(default)]
    pub is_optional: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapter_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_id: Option<String>,
}

/// Input for adding a step to a path.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AddPathStepInput {
    pub path_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapter_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_id: Option<String>,
    pub order_index: u32,
    pub step_type: String,
    pub resource_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_narrative: Option<String>,
    pub is_optional: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub learning_objectives: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reflection_prompts: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub practice_exercises: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_minutes: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_criteria: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attestation_required: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attestation_granted: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mastery_threshold: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_json: Option<String>,
}

/// Input for batch adding steps to a path.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BatchAddPathStepsInput {
    pub steps: Vec<AddPathStepInput>,
}

/// Output for a path step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PathStepOutput {
    pub action_hash: ActionHash,
    pub step: PathStep,
}

/// Input for updating a path.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UpdatePathInput {
    pub path_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_duration: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

/// Output for learning path with steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PathWithStepsOutput {
    pub action_hash: ActionHash,
    pub path: LearningPath,
    pub steps: Vec<PathStepOutput>,
}

/// Lightweight path overview (no step content, just counts).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PathOverviewOutput {
    pub action_hash: ActionHash,
    pub path: LearningPath,
    pub step_count: usize,
}

// =============================================================================
// Chapter Types
// =============================================================================

/// Chapter wire type. Mirrors the integrity zome's PathChapter entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Chapter {
    pub id: String,
    pub path_id: String,
    pub order_index: u32,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub learning_objectives_json: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_minutes: Option<u32>,
    pub is_optional: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attestation_granted: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mastery_threshold: Option<u32>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for creating a chapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateChapterInput {
    pub path_id: String,
    pub order_index: u32,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub learning_objectives: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_minutes: Option<u32>,
    pub is_optional: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attestation_granted: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mastery_threshold: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_json: Option<String>,
}

/// Output for a chapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ChapterOutput {
    pub action_hash: ActionHash,
    pub chapter: Chapter,
}

/// Input for updating a chapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UpdateChapterInput {
    pub chapter_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub learning_objectives: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_minutes: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_optional: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_index: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attestation_granted: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mastery_threshold: Option<u32>,
}

// =============================================================================
// Progress & Mastery Types
// =============================================================================

/// Input for starting path progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StartPathProgressInput {
    pub path_id: String,
}

/// Input for completing a step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CompleteStepInput {
    pub path_id: String,
    pub step_index: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affinity_score: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reflection_responses: Option<Vec<String>>,
}

/// Input for querying progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QueryProgressInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// Input for granting attestation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GrantAttestationInput {
    pub path_id: String,
    pub attestation_id: String,
    pub reason: String,
    pub source_type: String,
    pub source_id: String,
}

/// Input for checking attestation access.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CheckAttestationInput {
    pub path_id: String,
    pub required_attestation: String,
}

/// Output from attestation operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AttestationOutput {
    pub action_hash: ActionHash,
    pub attestation: AttestationRecord,
}

/// Attestation record fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AttestationRecord {
    pub agent_id: String,
    pub category: String,
    pub attestation_type: String,
    pub display_name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,
    pub earned_via_json: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// ContentAttestation wire type. Trust claims about content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ContentAttestation {
    pub id: String,
    pub content_id: String,
    pub attestation_type: String,
    pub reach_granted: String,
    pub granted_by_json: String,
    pub granted_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revocation_json: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_json: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_json: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub validation_status: String,
}

// =============================================================================
// Knowledge Map Types
// =============================================================================

/// KnowledgeMap wire type. Mirrors the integrity zome's KnowledgeMap entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct KnowledgeMap {
    pub id: String,
    pub map_type: String,
    pub owner_id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub subject_type: String,
    pub subject_id: String,
    pub subject_name: String,
    pub visibility: String,
    pub shared_with_json: String,
    pub nodes_json: String,
    pub path_ids_json: String,
    pub overall_affinity: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_graph_id: Option<String>,
    pub mastery_levels_json: String,
    pub goals_json: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Input for creating a knowledge map.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateKnowledgeMapInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub map_type: String,
    pub owner_id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub subject_type: String,
    pub subject_id: String,
    pub subject_name: String,
    pub visibility: String,
    pub shared_with_json: String,
    pub nodes_json: String,
    pub path_ids_json: String,
    pub overall_affinity: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_graph_id: Option<String>,
    pub mastery_levels_json: String,
    pub goals_json: String,
    pub metadata_json: String,
}

/// Output for knowledge map operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct KnowledgeMapOutput {
    pub action_hash: ActionHash,
    pub knowledge_map: KnowledgeMap,
}

/// Input for querying knowledge maps.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QueryKnowledgeMapInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

// =============================================================================
// Path Extension Types
// =============================================================================

/// PathExtension wire type. Mirrors the integrity zome's PathExtension entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PathExtension {
    pub id: String,
    pub base_path_id: String,
    pub base_path_version: String,
    pub extended_by: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub insertions_json: String,
    pub annotations_json: String,
    pub reorderings_json: String,
    pub exclusions_json: String,
    pub visibility: String,
    pub shared_with_json: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forked_from: Option<String>,
    pub forks_json: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_proposal_json: Option<String>,
    pub stats_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for creating a path extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreatePathExtensionInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub base_path_id: String,
    pub base_path_version: String,
    pub extended_by: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub insertions_json: String,
    pub annotations_json: String,
    pub reorderings_json: String,
    pub exclusions_json: String,
    pub visibility: String,
    pub shared_with_json: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forked_from: Option<String>,
    pub forks_json: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_proposal_json: Option<String>,
    pub stats_json: String,
}

/// Output for path extension operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PathExtensionOutput {
    pub action_hash: ActionHash,
    pub path_extension: PathExtension,
}

/// Input for querying path extensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QueryPathExtensionInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_path_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extended_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

// =============================================================================
// Import Types
// =============================================================================

/// Input for queuing an import batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QueueImportInput {
    pub id: String,
    pub batch_type: String,
    pub blob_hash: String,
    pub total_items: u32,
    pub schema_version: u32,
}

/// Input for processing a chunk of import data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ImportChunkInput {
    pub batch_id: String,
    pub chunk_index: u32,
    pub is_final: bool,
    pub items_json: String,
}

/// Output for import status.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ImportStatusOutput {
    pub id: String,
    pub batch_type: String,
    pub blob_hash: String,
    pub total_items: u32,
    pub status: String,
    pub processed_count: u32,
    pub error_count: u32,
    pub errors_json: String,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_id: Option<String>,
    pub schema_version: u32,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_content_input_msgpack_roundtrip() {
        let input = CreateContentInput {
            id: "concept-001".to_string(),
            content_type: "concept".to_string(),
            title: "Test Concept".to_string(),
            description: "A test concept".to_string(),
            summary: Some("Short summary".to_string()),
            content: "# Content body".to_string(),
            content_format: "markdown".to_string(),
            tags: vec!["test".to_string(), "concept".to_string()],
            source_path: None,
            related_node_ids: vec![],
            reach: "commons".to_string(),
            estimated_minutes: Some(10),
            thumbnail_url: None,
            metadata_json: "{}".to_string(),
            blob_cid: None,
            content_size_bytes: None,
            content_hash: None,
        };

        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateContentInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "concept-001");
        assert_eq!(decoded.content_type, "concept");
        assert_eq!(decoded.tags.len(), 2);
    }

    #[test]
    fn content_wire_type_msgpack_roundtrip() {
        let content = Content {
            id: "test-001".to_string(),
            content_type: "concept".to_string(),
            title: "Test".to_string(),
            description: "Desc".to_string(),
            summary: None,
            content: "body".to_string(),
            content_format: "markdown".to_string(),
            tags: vec![],
            source_path: None,
            related_node_ids: vec![],
            author_id: None,
            reach: "commons".to_string(),
            trust_score: 1.0,
            estimated_minutes: None,
            thumbnail_url: None,
            metadata_json: "{}".to_string(),
            created_at: "2024-01-01".to_string(),
            updated_at: "2024-01-01".to_string(),
            schema_version: 0,
            validation_status: String::new(),
            blob_cid: None,
            content_size_bytes: None,
            content_hash: None,
        };

        let bytes = rmp_serde::to_vec_named(&content).unwrap();
        let decoded: Content = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "test-001");
        assert_eq!(decoded.trust_score, 1.0);
    }

    #[test]
    fn create_relationship_input_msgpack_roundtrip() {
        let input = CreateRelationshipInput {
            source_id: "concept-001".to_string(),
            target_id: "concept-002".to_string(),
            relationship_type: "RELATES_TO".to_string(),
            confidence: 0.9,
            inference_source: "explicit".to_string(),
            metadata_json: None,
        };

        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateRelationshipInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.source_id, "concept-001");
        assert_eq!(decoded.confidence, 0.9);
    }

    #[test]
    fn create_path_input_msgpack_roundtrip() {
        let input = CreatePathInput {
            id: "path-001".to_string(),
            version: "1.0".to_string(),
            title: "Learning Rust".to_string(),
            description: "A path for learning Rust".to_string(),
            purpose: Some("Mastery".to_string()),
            difficulty: "intermediate".to_string(),
            estimated_duration: Some("4 weeks".to_string()),
            visibility: "public".to_string(),
            path_type: "linear".to_string(),
            tags: vec!["rust".to_string()],
            metadata_json: None,
        };

        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreatePathInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "path-001");
        assert_eq!(decoded.title, "Learning Rust");
    }

    #[test]
    fn add_path_step_input_msgpack_roundtrip() {
        let input = AddPathStepInput {
            path_id: "path-001".to_string(),
            chapter_id: Some("ch-01".to_string()),
            module_id: None,
            section_id: None,
            order_index: 0,
            step_type: "content".to_string(),
            resource_id: "concept-001".to_string(),
            step_title: Some("Introduction".to_string()),
            step_narrative: None,
            is_optional: false,
            learning_objectives: Some(vec!["Understand basics".to_string()]),
            reflection_prompts: None,
            practice_exercises: None,
            estimated_minutes: Some(15),
            completion_criteria: Some("view".to_string()),
            attestation_required: None,
            attestation_granted: None,
            mastery_threshold: None,
            metadata_json: None,
        };

        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: AddPathStepInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.path_id, "path-001");
        assert_eq!(decoded.order_index, 0);
    }

    #[test]
    fn create_chapter_input_msgpack_roundtrip() {
        let input = CreateChapterInput {
            path_id: "path-001".to_string(),
            order_index: 0,
            title: "Chapter 1".to_string(),
            description: Some("First chapter".to_string()),
            learning_objectives: vec!["Understand basics".to_string()],
            estimated_minutes: Some(60),
            is_optional: false,
            attestation_granted: None,
            mastery_threshold: None,
            metadata_json: None,
        };

        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateChapterInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.path_id, "path-001");
        assert_eq!(decoded.title, "Chapter 1");
    }

    #[test]
    fn complete_step_input_msgpack_roundtrip() {
        let input = CompleteStepInput {
            path_id: "path-001".to_string(),
            step_index: 2,
            content_id: Some("concept-003".to_string()),
            affinity_score: Some(8),
            notes: Some("Great content!".to_string()),
            reflection_responses: None,
        };

        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CompleteStepInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.path_id, "path-001");
        assert_eq!(decoded.step_index, 2);
    }

    #[test]
    fn create_knowledge_map_input_msgpack_roundtrip() {
        let input = CreateKnowledgeMapInput {
            id: Some("km-001".to_string()),
            map_type: "domain".to_string(),
            owner_id: "user-001".to_string(),
            title: "Rust Knowledge".to_string(),
            description: Some("Map of Rust knowledge".to_string()),
            subject_type: "content-graph".to_string(),
            subject_id: "graph-001".to_string(),
            subject_name: "Rust".to_string(),
            visibility: "private".to_string(),
            shared_with_json: "[]".to_string(),
            nodes_json: "[]".to_string(),
            path_ids_json: "[]".to_string(),
            overall_affinity: 0.7,
            content_graph_id: None,
            mastery_levels_json: "{}".to_string(),
            goals_json: "[]".to_string(),
            metadata_json: "{}".to_string(),
        };

        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateKnowledgeMapInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.map_type, "domain");
        assert_eq!(decoded.overall_affinity, 0.7);
    }

    #[test]
    fn create_path_extension_input_msgpack_roundtrip() {
        let input = CreatePathExtensionInput {
            id: None,
            base_path_id: "path-001".to_string(),
            base_path_version: "1.0".to_string(),
            extended_by: "user-001".to_string(),
            title: "My Extensions".to_string(),
            description: None,
            insertions_json: "[]".to_string(),
            annotations_json: "[]".to_string(),
            reorderings_json: "[]".to_string(),
            exclusions_json: "[]".to_string(),
            visibility: "private".to_string(),
            shared_with_json: "[]".to_string(),
            forked_from: None,
            forks_json: "[]".to_string(),
            upstream_proposal_json: None,
            stats_json: "{}".to_string(),
        };

        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreatePathExtensionInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.base_path_id, "path-001");
    }

    #[test]
    fn queue_import_input_msgpack_roundtrip() {
        let input = QueueImportInput {
            id: "import-001".to_string(),
            batch_type: "content".to_string(),
            blob_hash: "sha256-abc123".to_string(),
            total_items: 50,
            schema_version: 1,
        };

        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: QueueImportInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "import-001");
        assert_eq!(decoded.total_items, 50);
    }

    #[test]
    fn content_stats_msgpack_roundtrip() {
        let stats = ContentStats {
            total_count: 100,
            by_type: {
                let mut m = HashMap::new();
                m.insert("concept".to_string(), 50);
                m.insert("lesson".to_string(), 30);
                m
            },
        };

        let bytes = rmp_serde::to_vec_named(&stats).unwrap();
        let decoded: ContentStats = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.total_count, 100);
        assert_eq!(decoded.by_type.get("concept"), Some(&50));
    }

    #[test]
    fn path_import_input_msgpack_roundtrip() {
        let input = PathImportInput {
            id: "path-001".to_string(),
            version: "1.0".to_string(),
            title: "Test Path".to_string(),
            description: "Test".to_string(),
            purpose: None,
            difficulty: "beginner".to_string(),
            estimated_duration: None,
            visibility: "public".to_string(),
            path_type: "linear".to_string(),
            tags: vec![],
            metadata_json: None,
            steps: vec![PathImportStepInput {
                step_type: "content".to_string(),
                resource_id: "concept-001".to_string(),
                order_index: 0,
                step_title: None,
                step_narrative: None,
                is_optional: false,
                module_id: None,
                chapter_id: None,
                section_id: None,
            }],
        };

        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: PathImportInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "path-001");
        assert_eq!(decoded.steps.len(), 1);
    }

    #[test]
    fn import_chunk_input_msgpack_roundtrip() {
        let input = ImportChunkInput {
            batch_id: "import-001".to_string(),
            chunk_index: 0,
            is_final: false,
            items_json: "[]".to_string(),
        };

        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: ImportChunkInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.batch_id, "import-001");
        assert_eq!(decoded.chunk_index, 0);
    }
}
