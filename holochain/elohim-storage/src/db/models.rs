//! Diesel model definitions for database tables
//!
//! All models include `app_id` for multi-tenant app scoping.
//! - Queryable structs: for SELECT queries (reading data)
//! - Insertable structs: for INSERT queries (writing data)
//!
//! TypeScript types are auto-generated via ts-rs. Run:
//!   cargo test export_bindings
//! Generated files go to: holochain/sdk/storage-client-ts/src/generated/

use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::diesel_schema::*;

// ============================================================================
// Timestamp Helpers (SQLite stores timestamps as TEXT)
// ============================================================================

/// Get current UTC timestamp as ISO 8601 string for SQLite TEXT columns
pub fn current_timestamp() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

// ============================================================================
// App Registry Models
// ============================================================================

/// Registered app from the apps table
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize, TS)]
#[diesel(table_name = apps)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct App {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub enabled: i32,
}

/// New app for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = apps)]
pub struct NewApp<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub description: Option<&'a str>,
}

// ============================================================================
// Content Models
// ============================================================================

/// Content row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize, TS)]
#[diesel(table_name = content)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct Content {
    pub id: String,
    pub app_id: String,
    pub title: String,
    pub description: Option<String>,
    pub content_type: String,
    pub content_format: String,
    pub blob_hash: Option<String>,
    pub blob_cid: Option<String>,
    pub content_size_bytes: Option<i32>,
    pub metadata_json: Option<String>,
    pub reach: String,
    pub validation_status: String,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub content_body: Option<String>,
}

/// Content with tags attached (API response)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentWithTags {
    #[serde(flatten)]
    pub content: Content,
    pub tags: Vec<String>,
}

/// New content for INSERT
#[derive(Debug, Clone, Insertable, Deserialize)]
#[diesel(table_name = content)]
pub struct NewContent<'a> {
    pub id: &'a str,
    pub app_id: &'a str,
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub content_type: &'a str,
    pub content_format: &'a str,
    pub blob_hash: Option<&'a str>,
    pub blob_cid: Option<&'a str>,
    pub content_size_bytes: Option<i32>,
    pub metadata_json: Option<&'a str>,
    pub reach: &'a str,
    pub created_by: Option<&'a str>,
}

/// Content tag row
#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize, TS)]
#[diesel(table_name = content_tags)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentTag {
    pub app_id: String,
    pub content_id: String,
    pub tag: String,
}

/// New content tag for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = content_tags)]
pub struct NewContentTag<'a> {
    pub app_id: &'a str,
    pub content_id: &'a str,
    pub tag: &'a str,
}

// ============================================================================
// Path Models
// ============================================================================

/// Path row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize, TS)]
#[diesel(table_name = paths)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct Path {
    pub id: String,
    pub app_id: String,
    pub title: String,
    pub description: Option<String>,
    pub path_type: String,
    pub difficulty: Option<String>,
    pub estimated_duration: Option<String>,
    pub thumbnail_url: Option<String>,
    pub thumbnail_alt: Option<String>,
    pub metadata_json: Option<String>,
    pub visibility: String,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// New path for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = paths)]
pub struct NewPath<'a> {
    pub id: &'a str,
    pub app_id: &'a str,
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub path_type: &'a str,
    pub difficulty: Option<&'a str>,
    pub estimated_duration: Option<&'a str>,
    pub thumbnail_url: Option<&'a str>,
    pub thumbnail_alt: Option<&'a str>,
    pub metadata_json: Option<&'a str>,
    pub visibility: &'a str,
    pub created_by: Option<&'a str>,
}

/// Path tag row
#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize, TS)]
#[diesel(table_name = path_tags)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PathTag {
    pub app_id: String,
    pub path_id: String,
    pub tag: String,
}

/// New path tag for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = path_tags)]
pub struct NewPathTag<'a> {
    pub app_id: &'a str,
    pub path_id: &'a str,
    pub tag: &'a str,
}

// ============================================================================
// Chapter Models
// ============================================================================

/// Chapter row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize, TS)]
#[diesel(table_name = chapters)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct Chapter {
    pub id: String,
    pub app_id: String,
    pub path_id: String,
    pub title: String,
    pub description: Option<String>,
    pub order_index: i32,
    pub estimated_duration: Option<String>,
}

/// New chapter for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = chapters)]
pub struct NewChapter<'a> {
    pub id: &'a str,
    pub app_id: &'a str,
    pub path_id: &'a str,
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub order_index: i32,
    pub estimated_duration: Option<&'a str>,
}

// ============================================================================
// Step Models
// ============================================================================

/// Step row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize, TS)]
#[diesel(table_name = steps)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct Step {
    pub id: String,
    pub app_id: String,
    pub path_id: String,
    pub chapter_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub step_type: String,
    pub resource_id: Option<String>,
    pub resource_type: Option<String>,
    pub order_index: i32,
    pub estimated_duration: Option<String>,
    pub metadata_json: Option<String>,
}

/// New step for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = steps)]
pub struct NewStep<'a> {
    pub id: &'a str,
    pub app_id: &'a str,
    pub path_id: &'a str,
    pub chapter_id: Option<&'a str>,
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub step_type: &'a str,
    pub resource_id: Option<&'a str>,
    pub resource_type: Option<&'a str>,
    pub order_index: i32,
    pub estimated_duration: Option<&'a str>,
    pub metadata_json: Option<&'a str>,
}

// ============================================================================
// Path Attestation Models
// ============================================================================

/// Path attestation row
#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize, TS)]
#[diesel(table_name = path_attestations)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PathAttestation {
    pub app_id: String,
    pub path_id: String,
    pub attestation_type: String,
    pub attestation_name: String,
}

/// New path attestation for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = path_attestations)]
pub struct NewPathAttestation<'a> {
    pub app_id: &'a str,
    pub path_id: &'a str,
    pub attestation_type: &'a str,
    pub attestation_name: &'a str,
}

// ============================================================================
// Composite Types (API responses)
// ============================================================================

/// Chapter with its steps
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ChapterWithSteps {
    #[serde(flatten)]
    pub chapter: Chapter,
    pub steps: Vec<Step>,
}

/// Path with all nested data (chapters, steps, tags, attestations)
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PathWithDetails {
    #[serde(flatten)]
    pub path: Path,
    pub tags: Vec<String>,
    pub chapters: Vec<ChapterWithSteps>,
    pub ungrouped_steps: Vec<Step>,
    pub attestations: Vec<PathAttestation>,
}

/// Path with just steps (no chapter grouping)
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PathWithSteps {
    #[serde(flatten)]
    pub path: Path,
    pub steps: Vec<Step>,
}

// ============================================================================
// Relationship Models (Content Graph)
// ============================================================================

/// Content relationship row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize, TS)]
#[diesel(table_name = relationships)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct Relationship {
    pub id: String,
    pub app_id: String,
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: String,
    pub confidence: f32,
    pub inference_source: String,
    pub is_bidirectional: i32,
    pub inverse_relationship_id: Option<String>,
    pub provenance_chain_json: Option<String>,
    pub governance_layer: Option<String>,
    pub reach: String,
    pub metadata_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// New relationship for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = relationships)]
pub struct NewRelationship<'a> {
    pub id: &'a str,
    pub app_id: &'a str,
    pub source_id: &'a str,
    pub target_id: &'a str,
    pub relationship_type: &'a str,
    pub confidence: f32,
    pub inference_source: &'a str,
    pub is_bidirectional: i32,
    pub inverse_relationship_id: Option<&'a str>,
    pub provenance_chain_json: Option<&'a str>,
    pub governance_layer: Option<&'a str>,
    pub reach: &'a str,
    pub metadata_json: Option<&'a str>,
}

/// Relationship with both endpoints populated (API response)
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RelationshipWithContent {
    #[serde(flatten)]
    pub relationship: Relationship,
    pub source: Option<Content>,
    pub target: Option<Content>,
}

// ============================================================================
// Human Relationship Models (Imagodei)
// ============================================================================

/// Human relationship row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize, TS)]
#[diesel(table_name = human_relationships)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct HumanRelationship {
    pub id: String,
    pub app_id: String,
    pub party_a_id: String,
    pub party_b_id: String,
    pub relationship_type: String,
    pub intimacy_level: String,
    pub is_bidirectional: i32,
    pub consent_given_by_a: i32,
    pub consent_given_by_b: i32,
    pub custody_enabled_by_a: i32,
    pub custody_enabled_by_b: i32,
    pub auto_custody_enabled: i32,
    pub emergency_access_enabled: i32,
    pub initiated_by: String,
    pub verified_at: Option<String>,
    pub governance_layer: Option<String>,
    pub reach: String,
    pub context_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub expires_at: Option<String>,
}

/// New human relationship for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = human_relationships)]
pub struct NewHumanRelationship<'a> {
    pub id: &'a str,
    pub app_id: &'a str,
    pub party_a_id: &'a str,
    pub party_b_id: &'a str,
    pub relationship_type: &'a str,
    pub intimacy_level: &'a str,
    pub is_bidirectional: i32,
    pub consent_given_by_a: i32,
    pub consent_given_by_b: i32,
    pub custody_enabled_by_a: i32,
    pub custody_enabled_by_b: i32,
    pub auto_custody_enabled: i32,
    pub emergency_access_enabled: i32,
    pub initiated_by: &'a str,
    pub verified_at: Option<&'a str>,
    pub governance_layer: Option<&'a str>,
    pub reach: &'a str,
    pub context_json: Option<&'a str>,
    pub expires_at: Option<&'a str>,
}

// ============================================================================
// Contributor Presence Models (Stewardship)
// ============================================================================

/// Contributor presence row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize, TS)]
#[diesel(table_name = contributor_presences)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContributorPresence {
    pub id: String,
    pub app_id: String,
    pub display_name: String,
    pub presence_state: String,
    pub external_identifiers_json: Option<String>,
    pub establishing_content_ids_json: String,
    pub affinity_total: f32,
    pub unique_engagers: i32,
    pub citation_count: i32,
    pub recognition_score: f32,
    pub recognition_by_content_json: Option<String>,
    pub last_recognition_at: Option<String>,
    pub steward_id: Option<String>,
    pub stewardship_started_at: Option<String>,
    pub stewardship_commitment_id: Option<String>,
    pub stewardship_quality_score: Option<f32>,
    pub claim_initiated_at: Option<String>,
    pub claim_verified_at: Option<String>,
    pub claim_verification_method: Option<String>,
    pub claim_evidence_json: Option<String>,
    pub claimed_agent_id: Option<String>,
    pub claim_recognition_transferred_value: Option<f32>,
    pub claim_facilitated_by: Option<String>,
    pub image: Option<String>,
    pub note: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// New contributor presence for INSERT (minimal fields - DB defaults handle the rest)
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = contributor_presences)]
pub struct NewContributorPresence<'a> {
    pub id: &'a str,
    pub app_id: &'a str,
    pub display_name: &'a str,
    pub presence_state: &'a str,
    pub external_identifiers_json: Option<&'a str>,
    pub establishing_content_ids_json: &'a str,
    // Numeric fields with defaults - let DB handle defaults via DEFAULT constraint
    // affinity_total, unique_engagers, citation_count, recognition_score
    pub image: Option<&'a str>,
    pub note: Option<&'a str>,
    pub metadata_json: Option<&'a str>,
}

// ============================================================================
// Economic Event Models (hREA/ValueFlows)
// ============================================================================

/// Economic event row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize, TS)]
#[diesel(table_name = economic_events)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct EconomicEvent {
    pub id: String,
    pub app_id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    pub resource_conforms_to: Option<String>,
    pub resource_inventoried_as: Option<String>,
    pub resource_classified_as_json: Option<String>,
    pub resource_quantity_value: Option<f32>,
    pub resource_quantity_unit: Option<String>,
    pub effort_quantity_value: Option<f32>,
    pub effort_quantity_unit: Option<String>,
    pub has_point_in_time: String,
    pub has_duration: Option<String>,
    pub input_of: Option<String>,
    pub output_of: Option<String>,
    pub lamad_event_type: Option<String>,
    pub content_id: Option<String>,
    pub contributor_presence_id: Option<String>,
    pub path_id: Option<String>,
    pub triggered_by: Option<String>,
    pub state: String,
    pub note: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: String,
}

/// New economic event for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = economic_events)]
pub struct NewEconomicEvent<'a> {
    pub id: &'a str,
    pub app_id: &'a str,
    pub action: &'a str,
    pub provider: &'a str,
    pub receiver: &'a str,
    pub resource_conforms_to: Option<&'a str>,
    pub resource_inventoried_as: Option<&'a str>,
    pub resource_classified_as_json: Option<&'a str>,
    pub resource_quantity_value: Option<f32>,
    pub resource_quantity_unit: Option<&'a str>,
    pub effort_quantity_value: Option<f32>,
    pub effort_quantity_unit: Option<&'a str>,
    pub has_point_in_time: &'a str,
    pub has_duration: Option<&'a str>,
    pub input_of: Option<&'a str>,
    pub output_of: Option<&'a str>,
    pub lamad_event_type: Option<&'a str>,
    pub content_id: Option<&'a str>,
    pub contributor_presence_id: Option<&'a str>,
    pub path_id: Option<&'a str>,
    pub triggered_by: Option<&'a str>,
    pub state: &'a str,
    pub note: Option<&'a str>,
    pub metadata_json: Option<&'a str>,
}

// ============================================================================
// Content Mastery Models (Bloom's Taxonomy)
// ============================================================================

/// Content mastery row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize, TS)]
#[diesel(table_name = content_mastery)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentMastery {
    pub id: String,
    pub app_id: String,
    pub human_id: String,
    pub content_id: String,
    pub mastery_level: String,
    pub mastery_level_index: i32,
    pub freshness_score: f32,
    pub needs_refresh: i32,
    pub engagement_count: i32,
    pub last_engagement_type: Option<String>,
    pub last_engagement_at: Option<String>,
    pub level_achieved_at: Option<String>,
    pub content_version_at_mastery: Option<String>,
    pub assessment_evidence_json: Option<String>,
    pub privileges_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// New content mastery for INSERT (minimal - uses DB defaults for counters/scores)
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = content_mastery)]
pub struct NewContentMastery<'a> {
    pub id: &'a str,
    pub app_id: &'a str,
    pub human_id: &'a str,
    pub content_id: &'a str,
    pub mastery_level: &'a str,
    pub mastery_level_index: i32,
    // DB defaults: freshness_score=1.0, needs_refresh=0, engagement_count=0
    pub content_version_at_mastery: Option<&'a str>,
}

// ============================================================================
// Mastery Level Constants (Bloom's Taxonomy)
// ============================================================================

/// Bloom's taxonomy mastery levels
pub mod mastery_levels {
    pub const NOT_STARTED: &str = "not_started";
    pub const AWARE: &str = "aware";
    pub const REMEMBER: &str = "remember";
    pub const UNDERSTAND: &str = "understand";
    pub const APPLY: &str = "apply";
    pub const ANALYZE: &str = "analyze";
    pub const EVALUATE: &str = "evaluate";
    pub const CREATE: &str = "create";

    /// All mastery levels in order
    pub const ALL: [&str; 8] = [
        NOT_STARTED, AWARE, REMEMBER, UNDERSTAND, APPLY, ANALYZE, EVALUATE, CREATE
    ];

    /// Convert mastery level to index (0-7)
    pub fn to_index(level: &str) -> i32 {
        match level {
            NOT_STARTED => 0,
            AWARE => 1,
            REMEMBER => 2,
            UNDERSTAND => 3,
            APPLY => 4,
            ANALYZE => 5,
            EVALUATE => 6,
            CREATE => 7,
            _ => 0,
        }
    }

    /// Get index of mastery level, returning None for invalid levels
    pub fn index_of(level: &str) -> Option<usize> {
        ALL.iter().position(|&l| l == level)
    }

    /// Check if a mastery level is valid
    pub fn is_valid(level: &str) -> bool {
        ALL.contains(&level)
    }

    /// Check if mastery level indicates mastered (APPLY or above)
    pub fn is_mastered(level: &str) -> bool {
        to_index(level) >= ATTESTATION_GATE_LEVEL
    }

    /// Convert index to mastery level
    pub fn from_index(index: i32) -> &'static str {
        match index {
            0 => NOT_STARTED,
            1 => AWARE,
            2 => REMEMBER,
            3 => UNDERSTAND,
            4 => APPLY,
            5 => ANALYZE,
            6 => EVALUATE,
            7 => CREATE,
            _ => NOT_STARTED,
        }
    }

    /// Attestation gate level (apply = level 4)
    pub const ATTESTATION_GATE_LEVEL: i32 = 4;
}

// ============================================================================
// Presence State Constants
// ============================================================================

/// Contributor presence states
pub mod presence_states {
    pub const UNCLAIMED: &str = "unclaimed";
    pub const STEWARDED: &str = "stewarded";
    pub const CLAIMING: &str = "claiming";
    pub const CLAIMED: &str = "claimed";

    /// All presence states in lifecycle order
    pub const ALL: [&str; 4] = [UNCLAIMED, STEWARDED, CLAIMING, CLAIMED];

    /// Check if a presence state is valid
    pub fn is_valid(state: &str) -> bool {
        ALL.contains(&state)
    }
}

// ============================================================================
// Intimacy Level Constants
// ============================================================================

/// Human relationship intimacy levels
pub mod intimacy_levels {
    pub const RECOGNITION: &str = "recognition";
    pub const CONNECTION: &str = "connection";
    pub const TRUSTED: &str = "trusted";
    pub const INTIMATE: &str = "intimate";

    /// All intimacy levels in order (lowest to highest)
    pub const ALL: [&str; 4] = [RECOGNITION, CONNECTION, TRUSTED, INTIMATE];

    /// Check if an intimacy level is valid
    pub fn is_valid(level: &str) -> bool {
        ALL.contains(&level)
    }

    /// Get index of intimacy level, returning None for invalid levels
    pub fn index_of(level: &str) -> Option<usize> {
        ALL.iter().position(|&l| l == level)
    }

    /// Returns true if this intimacy level enables auto-custody
    pub fn auto_custody_enabled(level: &str) -> bool {
        matches!(level, INTIMATE)
    }

    /// Returns true if this intimacy level triggers auto-custody when both parties enable
    pub fn triggers_auto_custody(level: &str) -> bool {
        matches!(level, INTIMATE)
    }
}

// ============================================================================
// Relationship Type Constants
// ============================================================================

/// Content relationship types
pub mod relationship_types {
    pub const RELATES_TO: &str = "RELATES_TO";
    pub const CONTAINS: &str = "CONTAINS";
    pub const DEPENDS_ON: &str = "DEPENDS_ON";
    pub const IMPLEMENTS: &str = "IMPLEMENTS";
    pub const REFERENCES: &str = "REFERENCES";
    pub const DERIVED_FROM: &str = "DERIVED_FROM";
    pub const PREREQUISITE: &str = "PREREQUISITE";
    pub const FOLLOWUP: &str = "FOLLOWUP";
    pub const SIBLING: &str = "SIBLING";
    pub const PARENT: &str = "PARENT";
    pub const CHILD: &str = "CHILD";
    pub const SIMILAR_TO: &str = "SIMILAR_TO";
    pub const CONTRASTS_WITH: &str = "CONTRASTS_WITH";
    pub const ELABORATES: &str = "ELABORATES";
    pub const SUMMARIZES: &str = "SUMMARIZES";
    pub const EXAMPLE_OF: &str = "EXAMPLE_OF";
    pub const DEFINITION_OF: &str = "DEFINITION_OF";

    /// Returns the inverse relationship type, if applicable
    pub fn inverse(rel_type: &str) -> Option<&'static str> {
        match rel_type {
            CONTAINS => Some(CHILD),
            PARENT => Some(CHILD),
            CHILD => Some(PARENT),
            DEPENDS_ON => Some(PREREQUISITE),
            PREREQUISITE => Some(DEPENDS_ON),
            IMPLEMENTS => Some(DEFINITION_OF),
            DEFINITION_OF => Some(IMPLEMENTS),
            EXAMPLE_OF => Some(DEFINITION_OF),
            ELABORATES => Some(SUMMARIZES),
            SUMMARIZES => Some(ELABORATES),
            _ => None, // RELATES_TO, SIMILAR_TO, etc. are symmetric
        }
    }

    /// Returns true if this relationship type is hierarchical (can form cycles)
    pub fn is_hierarchical(rel_type: &str) -> bool {
        matches!(
            rel_type,
            CONTAINS | PARENT | CHILD | DEPENDS_ON | PREREQUISITE
        )
    }
}

// ============================================================================
// hREA Action Constants
// ============================================================================

/// Economic event actions (hREA/ValueFlows)
pub mod rea_actions {
    // Input actions
    pub const USE: &str = "use";
    pub const CONSUME: &str = "consume";
    pub const CITE: &str = "cite";

    // Output actions
    pub const PRODUCE: &str = "produce";
    pub const RAISE: &str = "raise";
    pub const LOWER: &str = "lower";

    // Transfer actions
    pub const TRANSFER: &str = "transfer";
    pub const TRANSFER_CUSTODY: &str = "transfer-custody";
    pub const TRANSFER_ALL_RIGHTS: &str = "transfer-all-rights";
    pub const MOVE: &str = "move";

    // Modify actions
    pub const MODIFY: &str = "modify";
    pub const COMBINE: &str = "combine";
    pub const SEPARATE: &str = "separate";

    // Work actions
    pub const WORK: &str = "work";
    pub const DELIVER_SERVICE: &str = "deliver-service";

    // Exchange actions
    pub const GIVE: &str = "give";
    pub const TAKE: &str = "take";
    pub const ACCEPT: &str = "accept";

    /// All supported hREA actions
    pub const ALL: [&str; 16] = [
        USE, CONSUME, CITE,
        PRODUCE, RAISE, LOWER,
        TRANSFER, TRANSFER_CUSTODY, TRANSFER_ALL_RIGHTS, MOVE,
        MODIFY, COMBINE, SEPARATE,
        WORK, DELIVER_SERVICE,
        ACCEPT,
    ];

    /// Check if an action is valid
    pub fn is_valid(action: &str) -> bool {
        ALL.contains(&action)
    }
}

/// Lamad-specific event types
pub mod lamad_event_types {
    pub const CONTENT_VIEW: &str = "content-view";
    pub const PATH_STEP_COMPLETE: &str = "path-step-complete";
    pub const AFFINITY_MARK: &str = "affinity-mark";
    pub const PATH_COMPLETE: &str = "path-complete";
    pub const PATH_COMPLETION: &str = "path-complete"; // Alias for PATH_COMPLETE
    pub const ATTESTATION_GRANT: &str = "attestation-grant";
    pub const STEWARDSHIP_BEGIN: &str = "stewardship-begin";
    pub const PRESENCE_CLAIM: &str = "presence-claim";
    pub const RECOGNITION_TRANSFER: &str = "recognition-transfer";
    pub const AFFINITY_TRANSFER: &str = "affinity-transfer";
    pub const CITATION: &str = "citation";
    pub const MASTERY_ADVANCE: &str = "mastery-advance";

    /// All supported lamad event types
    pub const ALL: [&str; 11] = [
        CONTENT_VIEW,
        PATH_STEP_COMPLETE,
        AFFINITY_MARK,
        PATH_COMPLETE,
        ATTESTATION_GRANT,
        STEWARDSHIP_BEGIN,
        PRESENCE_CLAIM,
        RECOGNITION_TRANSFER,
        AFFINITY_TRANSFER,
        CITATION,
        MASTERY_ADVANCE,
    ];

    /// Check if an event type is valid
    pub fn is_valid(event_type: &str) -> bool {
        ALL.contains(&event_type)
    }
}

// ============================================================================
// Stewardship Allocation Models
// ============================================================================

/// Stewardship allocation row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize, TS)]
#[diesel(table_name = stewardship_allocations)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StewardshipAllocation {
    pub id: String,
    pub app_id: String,
    pub content_id: String,
    pub steward_presence_id: String,
    pub allocation_ratio: f32,
    pub allocation_method: String,
    pub contribution_type: String,
    pub contribution_evidence_json: Option<String>,
    pub governance_state: String,
    pub dispute_id: Option<String>,
    pub dispute_reason: Option<String>,
    pub disputed_at: Option<String>,
    pub disputed_by: Option<String>,
    pub negotiation_session_id: Option<String>,
    pub elohim_ratified_at: Option<String>,
    pub elohim_ratifier_id: Option<String>,
    pub effective_from: String,
    pub effective_until: Option<String>,
    pub superseded_by: Option<String>,
    pub recognition_accumulated: f32,
    pub last_recognition_at: Option<String>,
    pub note: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// New stewardship allocation for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = stewardship_allocations)]
pub struct NewStewardshipAllocation<'a> {
    pub id: &'a str,
    pub app_id: &'a str,
    pub content_id: &'a str,
    pub steward_presence_id: &'a str,
    pub allocation_ratio: f32,
    pub allocation_method: &'a str,
    pub contribution_type: &'a str,
    pub contribution_evidence_json: Option<&'a str>,
    pub governance_state: &'a str,
    pub note: Option<&'a str>,
    pub metadata_json: Option<&'a str>,
}

/// Allocation with steward presence attached (API response)
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StewardshipAllocationWithPresence {
    #[serde(flatten)]
    pub allocation: StewardshipAllocation,
    pub steward: Option<ContributorPresence>,
}

/// Content stewardship aggregate (all allocations for content)
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentStewardship {
    pub content_id: String,
    pub allocations: Vec<StewardshipAllocationWithPresence>,
    pub total_allocation: f32,
    pub has_disputes: bool,
    pub primary_steward: Option<StewardshipAllocation>,
}

// ============================================================================
// Stewardship Allocation Constants
// ============================================================================

/// Allocation method constants
pub mod allocation_methods {
    pub const MANUAL: &str = "manual";
    pub const COMPUTED: &str = "computed";
    pub const NEGOTIATED: &str = "negotiated";

    pub const ALL: [&str; 3] = [MANUAL, COMPUTED, NEGOTIATED];

    pub fn is_valid(method: &str) -> bool {
        ALL.contains(&method)
    }
}

/// Contribution type constants
pub mod contribution_types {
    pub const ORIGINAL_CREATOR: &str = "original_creator";
    pub const EDITOR: &str = "editor";
    pub const TRANSLATOR: &str = "translator";
    pub const CURATOR: &str = "curator";
    pub const MAINTAINER: &str = "maintainer";
    pub const INHERITED: &str = "inherited";

    pub const ALL: [&str; 6] = [
        ORIGINAL_CREATOR, EDITOR, TRANSLATOR, CURATOR, MAINTAINER, INHERITED
    ];

    pub fn is_valid(ctype: &str) -> bool {
        ALL.contains(&ctype)
    }
}

/// Governance state constants
pub mod governance_states {
    pub const ACTIVE: &str = "active";
    pub const DISPUTED: &str = "disputed";
    pub const PENDING_REVIEW: &str = "pending_review";
    pub const SUPERSEDED: &str = "superseded";

    pub const ALL: [&str; 4] = [ACTIVE, DISPUTED, PENDING_REVIEW, SUPERSEDED];

    pub fn is_valid(state: &str) -> bool {
        ALL.contains(&state)
    }
}
