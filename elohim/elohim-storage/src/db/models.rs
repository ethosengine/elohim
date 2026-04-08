//! Diesel model definitions for database tables
//!
//! All models include `h_app_id` for multi-tenant app scoping.
//! - Queryable structs: for SELECT queries (reading data)
//! - Insertable structs: for INSERT queries (writing data)
//!
//! TypeScript types are auto-generated via ts-rs. Run:
//!   cargo test export_bindings
//! Generated files go to: holochain/sdk/storage-client-ts/src/generated/

use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::diesel_schema::{
    access_grants, agreements, appeals, apps, challenges, collective_participations, collectives,
    comments, content, content_attestations, content_mastery, content_tags, contributor_dashboards,
    contributor_presences, custodian_metrics, device_policies, discussions, economic_events,
    enum_registry, governance_dispositions, governance_signals, governance_states, hazards,
    human_relationships, humans, imagodei_observations, knowledge_maps, local_sessions,
    node_stewardship, observation_entries, observation_sessions, places, precedents, premium_gates,
    proposal_options, proposals, ranked_votes, rea_commitments, relationships,
    responsibility_demand_configs, risk_alerts, schedules, shard_locations, shard_manifests,
    spatial_contexts, statement_votes, statements, steward_credentials, stewarded_nodes,
    stewardship_allocations, token_balances, token_decay_events, token_mint_events,
    token_transfers, votes,
};

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
    pub h_app_id: String,
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
    /// Source of truth: DHT (Content entry in lamad DNA). Classification: A (Notarized).
    pub dht_anchor_hash: Option<String>,
    /// Timestamp when this content was successfully published to the libp2p Kademlia DHT by the seeder drain loop.
    /// NULL = not yet published. Classification: C (Operational).
    pub p2p_published_at: Option<String>,
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
    pub h_app_id: &'a str,
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
    pub content_body: Option<&'a str>,
}

/// Content tag row
#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize, TS)]
#[diesel(table_name = content_tags)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentTag {
    pub h_app_id: String,
    pub content_id: String,
    pub tag: String,
}

/// New content tag for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = content_tags)]
pub struct NewContentTag<'a> {
    pub h_app_id: &'a str,
    pub content_id: &'a str,
    pub tag: &'a str,
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
    pub h_app_id: String,
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
    /// Source of truth: DHT (Relationship entry in lamad DNA). Classification: A (Notarized).
    pub dht_anchor_hash: Option<String>,
}

/// New relationship for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = relationships)]
pub struct NewRelationship<'a> {
    pub id: &'a str,
    pub h_app_id: &'a str,
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
    pub h_app_id: String,
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
    /// Source of truth: DHT (HumanRelationship entry in imagodei DNA). NULL for pre-coherence rows.
    pub dht_anchor_hash: Option<String>,
}

/// New human relationship for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = human_relationships)]
pub struct NewHumanRelationship<'a> {
    pub id: &'a str,
    pub h_app_id: &'a str,
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
    pub h_app_id: String,
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
    /// Source of truth: DHT (ContributorPresence entry in imagodei DNA). NULL for pre-coherence rows.
    pub dht_anchor_hash: Option<String>,
}

/// New contributor presence for INSERT (minimal fields - DB defaults handle the rest)
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = contributor_presences)]
pub struct NewContributorPresence<'a> {
    pub id: &'a str,
    pub h_app_id: &'a str,
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
    pub h_app_id: String,
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
    pub dht_anchor_hash: Option<String>,
    pub created_at: String,
    /// Place ID where this event occurred (spatial grounding for carrying capacity)
    pub at_location: Option<String>,
}

/// New economic event for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = economic_events)]
pub struct NewEconomicEvent<'a> {
    pub id: &'a str,
    pub h_app_id: &'a str,
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
    pub dht_anchor_hash: Option<&'a str>,
    pub at_location: Option<&'a str>,
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
    pub h_app_id: String,
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
    /// Source of truth: private source chain (agent-scoped). Classification: B2 (Agent-Scoped + Attestation).
    /// Populated only when mastery crosses threshold and an Attestation is issued to the DHT.
    pub dht_anchor_hash: Option<String>,
}

/// New content mastery for INSERT (minimal - uses DB defaults for counters/scores)
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = content_mastery)]
pub struct NewContentMastery<'a> {
    pub id: &'a str,
    pub h_app_id: &'a str,
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
    use crate::generated_enums::ALL_MASTERY_LEVELS;

    pub const NOT_STARTED: &str = "not_started";
    pub const SEEN: &str = "seen";
    /// Legacy alias — schema canonical value is "seen"
    pub const AWARE: &str = "aware";
    pub const REMEMBER: &str = "remember";
    pub const UNDERSTAND: &str = "understand";
    pub const APPLY: &str = "apply";
    pub const ANALYZE: &str = "analyze";
    pub const EVALUATE: &str = "evaluate";
    pub const CREATE: &str = "create";

    /// Core mastery levels in order (schema-canonical)
    pub const ALL: [&str; 8] = [
        NOT_STARTED,
        SEEN,
        REMEMBER,
        UNDERSTAND,
        APPLY,
        ANALYZE,
        EVALUATE,
        CREATE,
    ];

    /// Convert mastery level to index (0-7). Accepts legacy "aware" as alias for "seen".
    pub fn to_index(level: &str) -> i32 {
        match level {
            NOT_STARTED => 0,
            SEEN | AWARE => 1,
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

    /// Check if a mastery level is valid (uses schema-generated vocabulary)
    pub fn is_valid(level: &str) -> bool {
        ALL_MASTERY_LEVELS.contains(&level) || level == AWARE
    }

    /// Check if mastery level indicates mastered (APPLY or above)
    pub fn is_mastered(level: &str) -> bool {
        to_index(level) >= ATTESTATION_GATE_LEVEL
    }

    /// Convert index to mastery level
    pub fn from_index(index: i32) -> &'static str {
        match index {
            0 => NOT_STARTED,
            1 => SEEN,
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
// Human Identity Models (imagodei pillar)
// ============================================================================

/// Human identity row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = humans)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Human {
    pub id: String,
    pub agent_pub_key: Option<String>,
    pub display_name: String,
    pub bio: Option<String>,
    /// JSON array of affinity strings stored as TEXT
    pub affinities: String,
    pub profile_reach: String,
    pub location: Option<String>,
    pub profile_photo_url: Option<String>,
    pub h_app_id: String,
    pub created_at: String,
    pub updated_at: String,
    /// Source of truth: DHT (Human entry in imagodei DNA). NULL for pre-coherence rows.
    pub dht_anchor_hash: Option<String>,
}

/// New human for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = humans)]
pub struct NewHuman {
    pub id: String,
    pub agent_pub_key: Option<String>,
    pub display_name: String,
    pub bio: Option<String>,
    pub affinities: String,
    pub profile_reach: String,
    pub location: Option<String>,
    pub profile_photo_url: Option<String>,
    pub h_app_id: String,
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

    // Social actions
    pub const APPRECIATE: &str = "appreciate";

    /// All supported hREA actions
    pub const ALL: [&str; 17] = [
        USE,
        CONSUME,
        CITE,
        PRODUCE,
        RAISE,
        LOWER,
        TRANSFER,
        TRANSFER_CUSTODY,
        TRANSFER_ALL_RIGHTS,
        MOVE,
        MODIFY,
        COMBINE,
        SEPARATE,
        WORK,
        DELIVER_SERVICE,
        ACCEPT,
        APPRECIATE,
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
    pub const CONTENT_DELIVERY: &str = "content-delivery";

    /// All supported lamad event types
    pub const ALL: [&str; 12] = [
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
        CONTENT_DELIVERY,
    ];

    /// Check if an event type is valid
    pub fn is_valid(event_type: &str) -> bool {
        ALL.contains(&event_type)
    }
}

// ============================================================================
// Steward Affinity
// ============================================================================

/// Steward affinity source types
pub mod affinity_sources {
    pub const GENESIS_SEED: &str = "genesis_seed";
    pub const MASTERY_GATE: &str = "mastery_gate";
    pub const CURATION_EDIT: &str = "curation_edit";
    pub const CURATION_REVIEW: &str = "curation_review";
    pub const DISPUTE_RESOLUTION: &str = "dispute_resolution";

    pub fn is_valid(source: &str) -> bool {
        matches!(
            source,
            GENESIS_SEED | MASTERY_GATE | CURATION_EDIT | CURATION_REVIEW | DISPUTE_RESOLUTION
        )
    }
}

/// Steward affinity record — tracks a steward's earned relationship to content
#[derive(Debug, Clone, Queryable, Serialize)]
#[diesel(table_name = crate::db::diesel_schema::steward_affinity)]
pub struct StewardAffinity {
    pub id: String,
    pub h_app_id: String,
    pub steward_id: String,
    pub content_id: String,
    pub affinity_score: f32,
    pub source: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Insertable steward affinity record
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::db::diesel_schema::steward_affinity)]
pub struct NewStewardAffinity<'a> {
    pub id: &'a str,
    pub h_app_id: &'a str,
    pub steward_id: &'a str,
    pub content_id: &'a str,
    pub affinity_score: f32,
    pub source: &'a str,
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
    pub h_app_id: String,
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
    pub dht_anchor_hash: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// New stewardship allocation for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = stewardship_allocations)]
pub struct NewStewardshipAllocation<'a> {
    pub id: &'a str,
    pub h_app_id: &'a str,
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
// Device Policy Models (Stewardship v5)
// ============================================================================

/// Device policy row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = device_policies)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct DevicePolicy {
    pub id: String,
    pub subject_id: String,
    pub device_id: Option<String>,
    pub author_id: String,
    pub author_tier: String,
    pub inherits_from: Option<String>,
    pub blocked_categories_json: String,
    pub blocked_hashes_json: String,
    pub age_rating_max: Option<String>,
    pub reach_level_max: Option<i32>,
    pub session_max_minutes: Option<i32>,
    pub daily_max_minutes: Option<i32>,
    pub time_windows_json: String,
    pub cooldown_minutes: Option<i32>,
    pub disabled_features_json: String,
    pub disabled_routes_json: String,
    pub require_approval_json: String,
    pub log_sessions: i32,
    pub log_categories: i32,
    pub log_policy_events: i32,
    pub retention_days: i32,
    pub subject_can_view: i32,
    pub effective_from: String,
    pub effective_until: Option<String>,
    pub version: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// New device policy for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = device_policies)]
pub struct NewDevicePolicy {
    pub id: String,
    pub subject_id: String,
    pub device_id: Option<String>,
    pub author_id: String,
    pub author_tier: String,
    pub inherits_from: Option<String>,
    pub blocked_categories_json: String,
    pub blocked_hashes_json: String,
    pub age_rating_max: Option<String>,
    pub reach_level_max: Option<i32>,
    pub session_max_minutes: Option<i32>,
    pub daily_max_minutes: Option<i32>,
    pub time_windows_json: String,
    pub cooldown_minutes: Option<i32>,
    pub disabled_features_json: String,
    pub disabled_routes_json: String,
    pub require_approval_json: String,
    pub log_sessions: i32,
    pub log_categories: i32,
    pub log_policy_events: i32,
    pub retention_days: i32,
    pub subject_can_view: i32,
    pub effective_from: String,
    pub effective_until: Option<String>,
    pub version: i32,
    pub created_at: String,
    pub updated_at: String,
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
        ORIGINAL_CREATOR,
        EDITOR,
        TRANSLATOR,
        CURATOR,
        MAINTAINER,
        INHERITED,
    ];

    pub fn is_valid(ctype: &str) -> bool {
        ALL.contains(&ctype)
    }
}

/// Governance state constants for stewardship allocations
pub mod allocation_governance_states {
    pub const ACTIVE: &str = "active";
    pub const DISPUTED: &str = "disputed";
    pub const PENDING_REVIEW: &str = "pending_review";
    pub const SUPERSEDED: &str = "superseded";

    pub const ALL: [&str; 4] = [ACTIVE, DISPUTED, PENDING_REVIEW, SUPERSEDED];

    pub fn is_valid(state: &str) -> bool {
        ALL.contains(&state)
    }
}

// ============================================================================
// Local Session Models (Tauri Native Handoff)
// ============================================================================

/// Local session row from SELECT query
/// Stores identity info from doorway after OAuth authentication for offline access
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize, TS)]
#[diesel(table_name = local_sessions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct LocalSession {
    pub id: String,
    pub human_id: String,
    pub agent_pub_key: String,
    pub doorway_url: String,
    pub doorway_id: Option<String>,
    pub identifier: String,
    pub display_name: Option<String>,
    pub profile_image_hash: Option<String>,
    pub is_active: i32,
    pub created_at: String,
    pub updated_at: String,
    pub last_synced_at: Option<String>,
    pub bootstrap_url: Option<String>,
    pub session_intent_json: Option<String>,
    pub intent_set_at: Option<String>,
}

/// New local session for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = local_sessions)]
pub struct NewLocalSession<'a> {
    pub id: &'a str,
    pub human_id: &'a str,
    pub agent_pub_key: &'a str,
    pub doorway_url: &'a str,
    pub doorway_id: Option<&'a str>,
    pub identifier: &'a str,
    pub display_name: Option<&'a str>,
    pub profile_image_hash: Option<&'a str>,
    pub bootstrap_url: Option<&'a str>,
}

// ============================================================================
// Collective Models (Qahal - Governance Contexts)
// ============================================================================

/// Collective row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize, TS)]
#[diesel(table_name = collectives)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct Collective {
    pub id: String,
    pub h_app_id: String,
    pub name: String,
    pub description: Option<String>,
    pub governance_layer: String,
    pub constitutional_parent_id: Option<String>,
    pub reach: String,
    pub metadata_json: Option<String>,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub dissolved_at: Option<String>,
}

/// New collective for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = collectives)]
pub struct NewCollective<'a> {
    pub id: &'a str,
    pub h_app_id: &'a str,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub governance_layer: &'a str,
    pub constitutional_parent_id: Option<&'a str>,
    pub reach: &'a str,
    pub metadata_json: Option<&'a str>,
    pub created_by: Option<&'a str>,
}

// ============================================================================
// Collective Participation Models
// ============================================================================

/// Collective participation row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize, TS)]
#[diesel(table_name = collective_participations)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CollectiveParticipation {
    pub id: String,
    pub h_app_id: String,
    pub collective_id: String,
    pub human_id: String,
    pub intimacy_level: String,
    pub role_context: Option<String>,
    pub governance_weight: f32,
    pub consent_state: String,
    pub metadata_json: Option<String>,
    pub joined_at: String,
    pub updated_at: String,
    pub departed_at: Option<String>,
}

/// New collective participation for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = collective_participations)]
pub struct NewCollectiveParticipation<'a> {
    pub id: &'a str,
    pub h_app_id: &'a str,
    pub collective_id: &'a str,
    pub human_id: &'a str,
    pub intimacy_level: &'a str,
    pub role_context: Option<&'a str>,
    pub governance_weight: f32,
    pub consent_state: &'a str,
    pub metadata_json: Option<&'a str>,
}

// ============================================================================
// Governance Layer Constants
// ============================================================================

/// Governance layer types for collectives
pub mod governance_layers {
    pub const FAMILY: &str = "family";
    pub const NEIGHBORHOOD: &str = "neighborhood";
    pub const FAITH: &str = "faith";
    pub const EDUCATION: &str = "education";
    pub const INTEREST: &str = "interest";
    pub const GEOGRAPHIC: &str = "geographic";
    pub const WORKPLACE: &str = "workplace";
    pub const ECONOMIC: &str = "economic";
    pub const COMMUNITY: &str = "community";

    /// All governance layers
    pub const ALL: [&str; 9] = [
        FAMILY,
        NEIGHBORHOOD,
        FAITH,
        EDUCATION,
        INTEREST,
        GEOGRAPHIC,
        WORKPLACE,
        ECONOMIC,
        COMMUNITY,
    ];

    /// Check if a governance layer is valid
    pub fn is_valid(layer: &str) -> bool {
        ALL.contains(&layer)
    }
}

/// Consent states for participation
pub mod consent_states {
    pub const PENDING: &str = "pending";
    pub const CONSENTED: &str = "consented";
    pub const WITHDRAWN: &str = "withdrawn";

    pub const ALL: [&str; 3] = [PENDING, CONSENTED, WITHDRAWN];

    pub fn is_valid(state: &str) -> bool {
        ALL.contains(&state)
    }
}

// ============================================================================
// Custodian Metrics Models
// ============================================================================

/// Persisted custodian metrics snapshot — reported by custodian nodes via
/// the storage API and queried by the shefa dashboard and operator tooling.
///
/// The primary key is `custodian_id` (agent pub key); each node upserts its
/// own row so the table holds the most-recent snapshot per custodian.
///
/// Metric groups are stored as JSON TEXT blobs to stay within Diesel's
/// 32-column limit. The service layer deserialises them before building
/// `CustodianMetricsView`.
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = custodian_metrics)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct CustodianMetrics {
    pub custodian_id: String,
    pub h_app_id: String,
    pub tier: i32,
    /// JSON: CustodianHealthView fields
    pub health_json: String,
    /// JSON: CustodianStorageMetricsView fields
    pub storage_json: String,
    /// JSON: CustodianBandwidthView fields
    pub bandwidth_json: String,
    /// JSON: CustodianComputationView fields
    pub computation_json: String,
    /// JSON: CustodianReputationView fields
    pub reputation_json: String,
    /// JSON: CustodianEconomicView fields
    pub economic_json: String,
    /// Unix timestamp (milliseconds) when metrics were collected
    pub collected_at: i64,
    /// Unix timestamp (milliseconds) of last update
    pub last_updated_at: i64,
}

/// New or updated custodian metrics for INSERT OR REPLACE.
#[derive(Debug, Clone, Insertable, Deserialize)]
#[diesel(table_name = custodian_metrics)]
pub struct UpsertCustodianMetrics {
    pub custodian_id: String,
    pub h_app_id: String,
    pub tier: i32,
    pub health_json: String,
    pub storage_json: String,
    pub bandwidth_json: String,
    pub computation_json: String,
    pub reputation_json: String,
    pub economic_json: String,
    pub collected_at: i64,
    pub last_updated_at: i64,
}

// ============================================================================
// Governance Models (v7)
// ============================================================================

/// Governance state for an entity
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = governance_states)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct GovernanceState {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub reach: String,
    pub labels: String,
    pub voting_state: String,
    pub signal_count: i32,
    pub created_at: String,
    pub updated_at: String,
    /// Source of truth: DHT (derived from proposal lifecycle). Classification: A2 (Derived).
    /// dht_anchor_hash links to the parent proposal ActionHash.
    pub dht_anchor_hash: Option<String>,
}

/// New governance state for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = governance_states)]
pub struct NewGovernanceState<'a> {
    pub id: &'a str,
    pub entity_type: &'a str,
    pub entity_id: &'a str,
    pub reach: &'a str,
    pub labels: &'a str,
    pub voting_state: &'a str,
}

/// Challenge — governance immune system challenge with full lifecycle
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = challenges)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Challenge {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub challenger_id: String,
    pub standing_basis: String,
    pub grounds_primary: String,
    pub grounds_secondary: Option<String>,
    pub evidence: String,
    pub requested_outcome: Option<String>,
    pub state: String,
    pub response_outcome: Option<String>,
    pub response_reasoning: Option<String>,
    pub response_actions: Option<String>,
    pub response_by: Option<String>,
    pub sets_precedent: i32,
    pub filed_at: String,
    pub acknowledged_at: Option<String>,
    pub response_deadline: String,
    pub responded_at: Option<String>,
    pub resolved_at: Option<String>,
    pub created_at: String,
    /// Source of truth: DHT (governance entry). Classification: A (Notarized).
    /// Challenges are public acts that must be witnessed by the community.
    pub dht_anchor_hash: Option<String>,
}

/// New challenge for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = challenges)]
pub struct NewChallenge<'a> {
    pub id: &'a str,
    pub entity_type: &'a str,
    pub entity_id: &'a str,
    pub challenger_id: &'a str,
    pub standing_basis: &'a str,
    pub grounds_primary: &'a str,
    pub grounds_secondary: Option<&'a str>,
    pub evidence: &'a str,
    pub requested_outcome: Option<&'a str>,
    pub state: &'a str,
    pub filed_at: &'a str,
    pub response_deadline: &'a str,
    pub created_at: &'a str,
}

/// Appeal — escalation of a challenge decision
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = appeals)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Appeal {
    pub id: String,
    pub challenge_id: String,
    pub appellant_id: String,
    pub grounds: String,
    pub additional_evidence: Option<String>,
    pub state: String,
    pub escalation_level: Option<String>,
    pub decision: Option<String>,
    pub decision_reasoning: Option<String>,
    pub decided_by: Option<String>,
    pub filed_at: String,
    pub decided_at: Option<String>,
    pub created_at: String,
    /// Source of truth: DHT (governance entry). Classification: A (Notarized).
    /// Appeals are public acts that must be witnessed by the community.
    pub dht_anchor_hash: Option<String>,
}

/// New appeal for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = appeals)]
pub struct NewAppeal<'a> {
    pub id: &'a str,
    pub challenge_id: &'a str,
    pub appellant_id: &'a str,
    pub grounds: &'a str,
    pub additional_evidence: Option<&'a str>,
    pub state: &'a str,
    pub filed_at: &'a str,
    pub created_at: &'a str,
}

/// Governance proposal
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = proposals)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Proposal {
    pub id: String,
    pub content_id: String,
    pub proposer_presence_id: String,
    pub proposal_type: String,
    pub title: String,
    pub body: String,
    pub status: String,
    pub votes_for: i32,
    pub votes_against: i32,
    pub voting_anonymous: i32,
    pub created_at: String,
    pub updated_at: String,
    pub voting_mechanism: String,
    pub score_min: Option<i32>,
    pub score_max: Option<i32>,
    pub dots_per_voter: Option<i32>,
    pub quorum_percentage: Option<f64>,
    pub passage_threshold: Option<f64>,
    /// Source of truth: DHT (governance entry in lamad DNA). Classification: A (Notarized).
    /// dht_anchor_hash links to the proposal ActionHash on the DHT.
    pub dht_anchor_hash: Option<String>,
}

/// New proposal for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = proposals)]
pub struct NewProposal<'a> {
    pub id: &'a str,
    pub content_id: &'a str,
    pub proposer_presence_id: &'a str,
    pub proposal_type: &'a str,
    pub title: &'a str,
    pub body: &'a str,
    pub voting_mechanism: &'a str,
    pub score_min: Option<i32>,
    pub score_max: Option<i32>,
    pub dots_per_voter: Option<i32>,
    pub quorum_percentage: Option<f64>,
    pub passage_threshold: Option<f64>,
}

/// Governance precedent
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = precedents)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Precedent {
    pub id: String,
    pub content_id: String,
    pub principle: String,
    pub interpretation: String,
    pub established_by: String,
    pub created_at: String,
    /// Source of truth: DHT (governance memory). Classification: A (Notarized).
    /// Precedents are immutable governance records — once set, the community relies on them.
    pub dht_anchor_hash: Option<String>,
}

/// New precedent for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = precedents)]
pub struct NewPrecedent<'a> {
    pub id: &'a str,
    pub content_id: &'a str,
    pub principle: &'a str,
    pub interpretation: &'a str,
    pub established_by: &'a str,
}

/// Discussion entry on content
/// Operational (Category C): reconstructable from thread/parent references. No dht_anchor_hash needed.
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = discussions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Discussion {
    pub id: String,
    pub content_id: String,
    pub author_presence_id: String,
    pub body: String,
    pub parent_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// New discussion for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = discussions)]
pub struct NewDiscussion<'a> {
    pub id: &'a str,
    pub content_id: &'a str,
    pub author_presence_id: &'a str,
    pub body: &'a str,
    pub parent_id: Option<&'a str>,
}

/// Governance vote on a proposal
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = votes)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Vote {
    pub id: String,
    pub proposal_id: String,
    pub human_id: String,
    pub position: String,
    pub reason: Option<String>,
    pub anonymous: i32,
    pub created_at: String,
    pub updated_at: String,
    /// Source of truth: private source chain (agent-scoped ballot). Classification: B2.
    /// Raw vote is private; dht_anchor_hash populated when tally Attestation is issued.
    pub dht_anchor_hash: Option<String>,
}

/// New vote for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = votes)]
pub struct NewVote<'a> {
    pub id: &'a str,
    pub proposal_id: &'a str,
    pub human_id: &'a str,
    pub position: &'a str,
    pub reason: Option<&'a str>,
    pub anonymous: i32,
    pub created_at: &'a str,
    pub updated_at: &'a str,
}

// ============================================================================
// Proposal Option Models (multi-mechanism voting)
// ============================================================================

/// Option within a multi-mechanism proposal
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = proposal_options)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ProposalOption {
    pub id: String,
    pub proposal_id: String,
    pub label: String,
    pub description: String,
    pub position: i32,
    pub source: Option<String>,
    pub source_justification: Option<String>,
    pub created_at: String,
    /// Source of truth: DHT (derived from proposal via Link). Classification: A2 (Derived).
    /// dht_anchor_hash links to the parent proposal ActionHash.
    pub dht_anchor_hash: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = proposal_options)]
pub struct NewProposalOption<'a> {
    pub id: &'a str,
    pub proposal_id: &'a str,
    pub label: &'a str,
    pub description: &'a str,
    pub position: i32,
    pub source: Option<&'a str>,
    pub source_justification: Option<&'a str>,
    pub created_at: &'a str,
}

// ============================================================================
// Ranked Vote Models (multi-mechanism voting)
// ============================================================================

/// Vote in a multi-mechanism proposal (ranked-choice, score, dot, approval)
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = ranked_votes)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct RankedVote {
    pub id: String,
    pub proposal_id: String,
    pub human_id: String,
    pub option_id: String,
    pub rank: Option<i32>,
    pub score: Option<i32>,
    pub dots: Option<i32>,
    pub approved: Option<i32>,
    pub reasoning: Option<String>,
    pub proxy_elohim_id: Option<String>,
    pub proxy_justification: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// Source of truth: private source chain (agent-scoped ballot). Classification: B2.
    /// Raw vote is private; dht_anchor_hash populated when tally Attestation is issued.
    pub dht_anchor_hash: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = ranked_votes)]
pub struct NewRankedVote<'a> {
    pub id: &'a str,
    pub proposal_id: &'a str,
    pub human_id: &'a str,
    pub option_id: &'a str,
    pub rank: Option<i32>,
    pub score: Option<i32>,
    pub dots: Option<i32>,
    pub approved: Option<i32>,
    pub reasoning: Option<&'a str>,
    pub proxy_elohim_id: Option<&'a str>,
    pub proxy_justification: Option<&'a str>,
    pub created_at: &'a str,
    pub updated_at: &'a str,
}

// ============================================================================
// Governance Signal Models
// ============================================================================

/// Normalized governance signal from any feedback mechanism
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = governance_signals)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct GovernanceSignal {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub human_id: String,
    pub signal_type: String,
    pub signal_value: String,
    pub mechanism_level: i32,
    pub proxy_elohim_id: Option<String>,
    pub created_at: String,
    /// Source of truth: private source chain (agent-scoped reaction). Classification: B2.
    /// Raw signal is private; dht_anchor_hash populated when aggregate Attestation is issued.
    pub dht_anchor_hash: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = governance_signals)]
pub struct NewGovernanceSignal<'a> {
    pub id: &'a str,
    pub entity_type: &'a str,
    pub entity_id: &'a str,
    pub human_id: &'a str,
    pub signal_type: &'a str,
    pub signal_value: &'a str,
    pub mechanism_level: i32,
    pub proxy_elohim_id: Option<&'a str>,
    pub created_at: &'a str,
}

// ============================================================================
// Governance Disposition Models
// ============================================================================

/// Classification: B (Agent-Scoped). Private governance profile.
/// Source of truth: agent source chain. NOT published to DHT.
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = governance_dispositions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct GovernanceDisposition {
    pub id: String,
    pub human_id: String,
    pub risk_tolerance: f32,
    pub change_openness: f32,
    pub consensus_preference: f32,
    pub priority_values: String,
    pub voting_pattern_summary: String,
    pub total_votes_cast: i32,
    pub total_challenges_filed: i32,
    pub total_signals_recorded: i32,
    /// Classification B: NOT published to DHT. Agent-scoped only.
    pub dht_anchor_hash: Option<String>,
    pub last_computed_at: String,
    pub created_at: String,
    pub updated_at: String,
}

/// New governance disposition for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = governance_dispositions)]
pub struct NewGovernanceDisposition<'a> {
    pub id: &'a str,
    pub human_id: &'a str,
    pub risk_tolerance: f32,
    pub change_openness: f32,
    pub consensus_preference: f32,
    pub priority_values: &'a str,
    pub voting_pattern_summary: &'a str,
    pub total_votes_cast: i32,
    pub total_challenges_filed: i32,
    pub total_signals_recorded: i32,
    pub last_computed_at: &'a str,
    pub created_at: &'a str,
    pub updated_at: &'a str,
}

// ============================================================================
// Content Attestation Models
// ============================================================================

/// Content attestation row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = content_attestations)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ContentAttestation {
    pub id: String,
    pub content_id: String,
    pub attestor_presence_id: String,
    pub scope: String,
    pub attestation_type: String,
    pub evidence: Option<String>,
    pub grantor: Option<String>,
    pub is_revoked: i32,
    pub revocation: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// Source of truth: DHT (Attestation entry in imagodei DNA). Classification: A (Notarized).
    pub dht_anchor_hash: Option<String>,
}

/// New content attestation for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = content_attestations)]
pub struct NewContentAttestation<'a> {
    pub id: &'a str,
    pub content_id: &'a str,
    pub attestor_presence_id: &'a str,
    pub scope: &'a str,
    pub attestation_type: &'a str,
    pub evidence: Option<&'a str>,
    pub grantor: Option<&'a str>,
}

// ============================================================================
// Steward Credential Models
// ============================================================================

/// Steward credential row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = steward_credentials)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct StewardCredential {
    pub id: String,
    pub presence_id: String,
    pub content_id: String,
    pub affinity_coefficient: f32,
    pub credential_type: String,
    pub status: String,
    pub dht_anchor_hash: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// New steward credential for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = steward_credentials)]
pub struct NewStewardCredential<'a> {
    pub id: &'a str,
    pub presence_id: &'a str,
    pub content_id: &'a str,
    pub affinity_coefficient: f32,
    pub credential_type: &'a str,
    pub status: &'a str,
}

// ============================================================================
// Premium Gate Models
// ============================================================================

/// Premium gate row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = premium_gates)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PremiumGate {
    pub id: String,
    pub steward_credential_id: String,
    pub steward_presence_id: String,
    pub gated_resource_type: String,
    pub gated_resource_ids: String,
    pub gate_title: String,
    pub gate_description: Option<String>,
    pub dht_anchor_hash: Option<String>,
    pub created_at: String,
}

/// New premium gate for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = premium_gates)]
pub struct NewPremiumGate<'a> {
    pub id: &'a str,
    pub steward_credential_id: &'a str,
    pub steward_presence_id: &'a str,
    pub gated_resource_type: &'a str,
    pub gated_resource_ids: &'a str,
    pub gate_title: &'a str,
    pub gate_description: Option<&'a str>,
}

// ============================================================================
// Access Grant Models
// ============================================================================

/// Access grant row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = access_grants)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct AccessGrant {
    pub id: String,
    pub gate_id: String,
    pub grantee_presence_id: String,
    pub contributor_presence_id: Option<String>,
    pub granted_at: String,
    pub expires_at: Option<String>,
    pub status: String,
    pub dht_anchor_hash: Option<String>,
}

/// New access grant for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = access_grants)]
pub struct NewAccessGrant<'a> {
    pub id: &'a str,
    pub gate_id: &'a str,
    pub grantee_presence_id: &'a str,
    pub contributor_presence_id: Option<&'a str>,
    pub granted_at: &'a str,
    pub expires_at: Option<&'a str>,
    pub status: &'a str,
}

// ============================================================================
// Contributor Dashboard Models
// ============================================================================

/// Contributor dashboard row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = contributor_dashboards)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ContributorDashboard {
    pub presence_id: String,
    pub total_contributions: i32,
    pub total_recognitions: i32,
    pub impact_score: f32,
    pub last_contribution_at: Option<String>,
    pub updated_at: String,
}

/// New contributor dashboard for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = contributor_dashboards)]
pub struct NewContributorDashboard<'a> {
    pub presence_id: &'a str,
    pub total_contributions: i32,
    pub total_recognitions: i32,
    pub impact_score: f32,
    pub last_contribution_at: Option<&'a str>,
}

// ============================================================================
// REA Commitment
// ============================================================================

/// REA Commitment — a binding promise of future economic activity
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = rea_commitments)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ReaCommitment {
    pub id: String,
    pub h_app_id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    pub resource_conforms_to: Option<String>,
    pub resource_classified_as: Option<String>,
    pub resource_quantity_value: Option<f32>,
    pub resource_quantity_unit: Option<String>,
    pub effort_quantity_value: Option<f32>,
    pub effort_quantity_unit: Option<String>,
    pub has_beginning: Option<String>,
    pub has_end: Option<String>,
    pub due: Option<String>,
    pub clause_of: Option<String>,
    pub in_scope_of: Option<String>,
    pub medium_of_exchange_id: Option<String>,
    pub state: String,
    pub finished: i32,
    pub note: Option<String>,
    pub metadata_json: Option<String>,
    pub dht_anchor_hash: Option<String>,
    pub created_at: String,
}

/// New REA commitment for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = rea_commitments)]
pub struct NewReaCommitment<'a> {
    pub id: &'a str,
    pub h_app_id: &'a str,
    pub action: &'a str,
    pub provider: &'a str,
    pub receiver: &'a str,
    pub resource_conforms_to: Option<&'a str>,
    pub resource_classified_as: Option<&'a str>,
    pub resource_quantity_value: Option<f32>,
    pub resource_quantity_unit: Option<&'a str>,
    pub effort_quantity_value: Option<f32>,
    pub effort_quantity_unit: Option<&'a str>,
    pub has_beginning: Option<&'a str>,
    pub has_end: Option<&'a str>,
    pub due: Option<&'a str>,
    pub clause_of: Option<&'a str>,
    pub in_scope_of: Option<&'a str>,
    pub medium_of_exchange_id: Option<&'a str>,
    pub state: &'a str,
    pub finished: i32,
    pub note: Option<&'a str>,
    pub metadata_json: Option<&'a str>,
    pub dht_anchor_hash: Option<&'a str>,
}

// ============================================================================
// Agreement
// ============================================================================

/// REA Agreement — a mutually understood arrangement between agents.
/// Commitments reference agreements via `clause_of`.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = agreements)]
pub struct AgreementRow {
    pub id: String,
    pub h_app_id: String,
    pub name: Option<String>,
    pub note: Option<String>,
    pub dht_anchor_hash: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = agreements)]
pub struct NewAgreement<'a> {
    pub id: &'a str,
    pub h_app_id: &'a str,
    pub name: Option<&'a str>,
    pub note: Option<&'a str>,
    pub dht_anchor_hash: Option<&'a str>,
    pub metadata_json: Option<&'a str>,
}

// ============================================================================
// Stewarded Node Models
// ============================================================================

/// A physical node registered on the DHT and projected to storage.
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = stewarded_nodes)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct StewardedNode {
    pub id: String,
    pub display_name: String,
    pub claim_status: String,
    pub cpu_cores: i32,
    pub memory_gb: i32,
    pub storage_tb: f64,
    pub bandwidth_mbps: i32,
    pub steward_tier: String,
    pub custodian_opt_in: i32,
    pub region: Option<String>,
    pub context_epr_id: Option<String>,
    pub dht_anchor_hash: Option<String>,
    pub h_app_id: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = stewarded_nodes)]
pub struct NewStewardedNode {
    pub id: String,
    pub display_name: String,
    pub claim_status: String,
    pub cpu_cores: i32,
    pub memory_gb: i32,
    pub storage_tb: f64,
    pub bandwidth_mbps: i32,
    pub steward_tier: String,
    pub custodian_opt_in: i32,
    pub region: Option<String>,
    pub context_epr_id: Option<String>,
    pub dht_anchor_hash: Option<String>,
    pub h_app_id: String,
}

/// Stewardship relationship between a node and a human.
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = node_stewardship)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NodeStewardship {
    pub node_id: String,
    pub human_id: String,
    pub affinity_score: f64,
    pub relationship: String,
    pub context_epr_id: Option<String>,
    pub granted_at: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = node_stewardship)]
pub struct NewNodeStewardship {
    pub node_id: String,
    pub human_id: String,
    pub affinity_score: f64,
    pub relationship: String,
    pub context_epr_id: Option<String>,
}

// ============================================================================
// Knowledge Map Models
// ============================================================================

/// Knowledge map from the database
///
/// Operational (Category C): personal sensemaking, reconstructable from content relationships.
/// No dht_anchor_hash needed.
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = knowledge_maps)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct KnowledgeMap {
    pub id: String,
    pub h_app_id: String,
    pub map_type: String,
    pub owner_id: String,
    pub title: String,
    pub description: Option<String>,
    pub subject_type: String,
    pub subject_id: String,
    pub subject_name: String,
    pub visibility: String,
    pub shared_with_json: Option<String>,
    pub nodes_json: String,
    pub path_ids_json: Option<String>,
    pub overall_affinity: f32,
    pub content_graph_id: Option<String>,
    pub mastery_levels_json: Option<String>,
    pub goals_json: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Insertable knowledge map
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = knowledge_maps)]
pub struct NewKnowledgeMap<'a> {
    pub id: &'a str,
    pub h_app_id: &'a str,
    pub map_type: &'a str,
    pub owner_id: &'a str,
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub subject_type: &'a str,
    pub subject_id: &'a str,
    pub subject_name: &'a str,
    pub visibility: &'a str,
    pub shared_with_json: Option<&'a str>,
    pub nodes_json: &'a str,
    pub path_ids_json: Option<&'a str>,
    pub overall_affinity: f32,
    pub content_graph_id: Option<&'a str>,
    pub mastery_levels_json: Option<&'a str>,
    pub goals_json: Option<&'a str>,
    pub metadata_json: Option<&'a str>,
}

// ============================================================================
// ImagodeiObservation
// ============================================================================

/// Elohim observation about human behavior — constitutional memory layer.
/// Access-controlled by constitutional layer. Feeds behavioral_trust in TrustContext.
#[derive(Debug, Clone, Queryable, Selectable, Serialize)]
#[diesel(table_name = imagodei_observations)]
pub struct ImagodeiObservation {
    pub id: String,
    pub h_app_id: String,
    pub human_id: String,
    pub observed_at: String,
    pub observation_type: String,
    pub content: String,
    pub structured_signals_json: Option<String>,
    pub trust_delta: f32,
    pub visibility_layer: String,
    pub originating_elohim: String,
    pub relevance_decay: f32,
    pub superseded_by: Option<String>,
    pub created_at: String,
    /// B2: populated when trust attestation is issued from aggregate observations
    pub dht_anchor_hash: Option<String>,
}

/// New observation for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = imagodei_observations)]
pub struct NewImagodeiObservation<'a> {
    pub id: &'a str,
    pub h_app_id: &'a str,
    pub human_id: &'a str,
    pub observed_at: &'a str,
    pub observation_type: &'a str,
    pub content: &'a str,
    pub structured_signals_json: Option<&'a str>,
    pub trust_delta: f32,
    pub visibility_layer: &'a str,
    pub originating_elohim: &'a str,
    pub relevance_decay: f32,
    pub superseded_by: Option<&'a str>,
    /// B2: NULL on creation, populated when trust attestation is issued
    pub dht_anchor_hash: Option<&'a str>,
}

// ============================================================================
// Comment Models
// ============================================================================

/// Comment from the comments table (Queryable)
/// Operational (Category C): reconstructable from parent references. No dht_anchor_hash needed.
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = comments)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Comment {
    pub id: String,
    pub h_app_id: String,
    pub content_id: String,
    pub human_id: String,
    pub body: String,
    pub reach: String,
    pub governance_state: String,
    pub created_at: String,
    pub updated_at: String,
}

/// New comment for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = comments)]
pub struct NewComment {
    pub id: String,
    pub h_app_id: String,
    pub content_id: String,
    pub human_id: String,
    pub body: String,
    pub reach: String,
    pub governance_state: String,
    pub created_at: String,
    pub updated_at: String,
}

// ============================================================================
// Sensemaking Statement Models
// ============================================================================

/// Statement row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = statements)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Statement {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub human_id: String,
    pub text: String,
    pub agree_count: i32,
    pub disagree_count: i32,
    pub pass_count: i32,
    pub group_id: Option<String>,
    pub is_bridging: i32,
    pub created_at: String,
    /// Source of truth: DHT (Polis sensemaking entry). Classification: A (Notarized).
    /// Community-visible statements must be witnessed to establish shared deliberative record.
    pub dht_anchor_hash: Option<String>,
}

/// New statement for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = statements)]
pub struct NewStatement<'a> {
    pub id: &'a str,
    pub entity_type: &'a str,
    pub entity_id: &'a str,
    pub human_id: &'a str,
    pub text: &'a str,
    pub created_at: &'a str,
}

/// Statement vote row from SELECT query
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = statement_votes)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct StatementVote {
    pub id: String,
    pub statement_id: String,
    pub human_id: String,
    pub vote: String,
    pub created_at: String,
    /// Source of truth: private source chain (agent-scoped stance). Classification: B2.
    /// Private stance; dht_anchor_hash populated when clustered aggregate Attestation is issued.
    pub dht_anchor_hash: Option<String>,
}

/// New statement vote for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = statement_votes)]
pub struct NewStatementVote<'a> {
    pub id: &'a str,
    pub statement_id: &'a str,
    pub human_id: &'a str,
    pub vote: &'a str,
    pub created_at: &'a str,
}

// ============================================================================
// Schedule Models (Kairos temporal dimension)
// ============================================================================

/// Schedule from the schedules table (Queryable)
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = schedules)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Schedule {
    pub id: String,
    pub h_app_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub scheduled_at: Option<String>,
    pub expires_at: Option<String>,
    pub rrule: Option<String>,
    pub last_occurred_at: Option<String>,
    pub next_occurrence_at: Option<String>,
    pub occurrence_count: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// New schedule for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = schedules)]
pub struct NewSchedule {
    pub id: String,
    pub h_app_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub scheduled_at: Option<String>,
    pub expires_at: Option<String>,
    pub rrule: Option<String>,
    pub last_occurred_at: Option<String>,
    pub next_occurrence_at: Option<String>,
    pub occurrence_count: i32,
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// Spatial Context — polymorphic geospatial attachment
// Source of truth: SQLite (operational, Category C)
// =============================================================================

/// Spatial context record (SELECT)
#[derive(Debug, Clone, Queryable, Serialize, Deserialize)]
#[diesel(table_name = spatial_contexts)]
pub struct SpatialContext {
    pub id: String,
    pub h_app_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f64>,
    pub accuracy: Option<f64>,
    pub h3_res5: Option<String>,
    pub h3_res7: Option<String>,
    pub h3_res9: Option<String>,
    pub place_id: Option<String>,
    pub osm_type: Option<String>,
    pub osm_id: Option<i32>,
    pub label: Option<String>,
    pub context_type: String,
    pub geometry_json: Option<String>,
    pub metadata_json: Option<String>,
    pub observed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub is_current: i32,
}

/// New spatial context for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = spatial_contexts)]
pub struct NewSpatialContext {
    pub id: String,
    pub h_app_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f64>,
    pub accuracy: Option<f64>,
    pub h3_res5: Option<String>,
    pub h3_res7: Option<String>,
    pub h3_res9: Option<String>,
    pub place_id: Option<String>,
    pub osm_type: Option<String>,
    pub osm_id: Option<i32>,
    pub label: Option<String>,
    pub context_type: String,
    pub geometry_json: Option<String>,
    pub metadata_json: Option<String>,
    pub observed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub is_current: i32,
}

// =============================================================================
// Place — notarized governed spatial entity
// Source of truth: DHT (Mishpat DNA, Category A)
// =============================================================================

/// Place record (SELECT) — storage projection of DHT Place entry
#[derive(Debug, Clone, Queryable, Serialize, Deserialize)]
#[diesel(table_name = places)]
pub struct Place {
    pub id: String,
    pub h_app_id: String,
    pub dht_anchor_hash: String,
    pub name: String,
    pub place_type: String,
    pub constitutional_layer: String,
    pub h3_index: String,
    pub h3_resolution: i32,
    pub geometry_json: String,
    pub centroid_lat: f64,
    pub centroid_lng: f64,
    pub parent_place_id: Option<String>,
    pub osm_reference_json: Option<String>,
    pub carrying_capacity_json: String,
    pub governing_collective_id: Option<String>,
    pub status: String,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// New place for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = places)]
pub struct NewPlace {
    pub id: String,
    pub h_app_id: String,
    pub dht_anchor_hash: String,
    pub name: String,
    pub place_type: String,
    pub constitutional_layer: String,
    pub h3_index: String,
    pub h3_resolution: i32,
    pub geometry_json: String,
    pub centroid_lat: f64,
    pub centroid_lng: f64,
    pub parent_place_id: Option<String>,
    pub osm_reference_json: Option<String>,
    pub carrying_capacity_json: String,
    pub governing_collective_id: Option<String>,
    pub status: String,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

// =============================================================================
// Hazard — geospatial risk event
// Source of truth: SQLite (operational, Category C)
// Reconstructable from external feeds / manual reports on cache miss.
// =============================================================================

/// Hazard record (SELECT)
#[derive(Debug, Clone, Queryable, Selectable, Serialize)]
#[diesel(table_name = hazards)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Hazard {
    pub id: String,
    pub h_app_id: String,
    pub place_id: String,
    pub hazard_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub reported_at: String,
    pub projected_onset: Option<String>,
    pub projected_end: Option<String>,
    pub actual_onset: Option<String>,
    pub resolved_at: Option<String>,
    pub affected_h3_cells: String,
    pub radius_km: Option<f64>,
    pub source: String,
    pub source_reference: Option<String>,
    pub metadata_json: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// New hazard for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = hazards)]
pub struct NewHazard {
    pub id: String,
    pub h_app_id: String,
    pub place_id: String,
    pub hazard_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub reported_at: String,
    pub projected_onset: Option<String>,
    pub projected_end: Option<String>,
    pub affected_h3_cells: String,
    pub radius_km: Option<f64>,
    pub source: String,
    pub source_reference: Option<String>,
    pub metadata_json: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// RiskAlert — generated alert from vulnerability assessment
// Source of truth: SQLite (operational, Category C)
// Reconstructable by re-running evaluate_and_generate_alerts on each Place.
// =============================================================================

/// Risk alert record (SELECT)
#[derive(Debug, Clone, Queryable, Selectable, Serialize)]
#[diesel(table_name = risk_alerts)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct RiskAlert {
    pub id: String,
    pub h_app_id: String,
    pub place_id: String,
    pub alert_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub trigger_hazard_id: Option<String>,
    pub trigger_data_json: String,
    pub triggered_at: String,
    pub lead_time_hours: Option<f64>,
    pub expires_at: Option<String>,
    pub status: String,
    pub acknowledged_by: Option<String>,
    pub acknowledged_at: Option<String>,
    pub resolved_at: Option<String>,
    pub escalated_to: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// New risk alert for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = risk_alerts)]
pub struct NewRiskAlert {
    pub id: String,
    pub h_app_id: String,
    pub place_id: String,
    pub alert_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub trigger_hazard_id: Option<String>,
    pub trigger_data_json: String,
    pub triggered_at: String,
    pub lead_time_hours: Option<f64>,
    pub expires_at: Option<String>,
    pub status: String,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// EnumRegistry — extensible vocabulary (Category C operational)
// Source of truth: SQLite. Seeded from JSON Schema on startup.
// =============================================================================

/// Enum registry entry (SELECT)
#[derive(Debug, Clone, Queryable, Selectable, Serialize)]
#[diesel(table_name = enum_registry)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct EnumRegistryEntry {
    pub id: i32,
    pub h_app_id: String,
    pub enum_name: String,
    pub enum_value: String,
    pub tier: String,
    pub added_by: Option<String>,
    pub created_at: String,
}

/// Enum registry entry (INSERT)
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = enum_registry)]
pub struct NewEnumRegistryEntry {
    pub h_app_id: String,
    pub enum_name: String,
    pub enum_value: String,
    pub tier: String,
    pub added_by: Option<String>,
    pub created_at: String,
}

// ============================================================================
// Token Models (Shefa economy — elohim-token sprint 1)
// ============================================================================

/// Token mint event row from SELECT query.
/// Category A (notarized): every mint is coupled to a witnessed REA event.
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = token_mint_events)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TokenMintEvent {
    pub id: String,
    pub h_app_id: String,
    pub amount: f32,
    pub provenance_event_id: String,
    pub mint_tier: String,
    pub source_epr_id: String,
    pub agent_id: String,
    pub constitutional_context: Option<String>,
    pub elohim_attestation: Option<String>,
    pub reasoning_trace: Option<String>,
    pub dht_anchor_hash: Option<String>,
    pub created_at: String,
}

/// New token mint event for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = token_mint_events)]
pub struct NewTokenMintEvent<'a> {
    pub id: &'a str,
    pub h_app_id: &'a str,
    pub amount: f32,
    pub provenance_event_id: &'a str,
    pub mint_tier: &'a str,
    pub source_epr_id: &'a str,
    pub agent_id: &'a str,
    pub constitutional_context: Option<&'a str>,
    pub elohim_attestation: Option<&'a str>,
    pub reasoning_trace: Option<&'a str>,
    pub dht_anchor_hash: Option<&'a str>,
}

/// Token balance row from SELECT query.
/// Category B (agent-scoped): per-agent, per-governance-layer balance ledger.
/// Composite PK: (agent_id, h_app_id, governance_layer).
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = token_balances)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TokenBalance {
    pub agent_id: String,
    pub h_app_id: String,
    pub governance_layer: String,
    pub balance: f32,
    pub total_minted: f32,
    pub total_transferred_in: f32,
    pub total_transferred_out: f32,
    pub last_activity_at: String,
    pub created_at: String,
}

/// Token transfer row from SELECT query.
/// Category A (notarized): witnessed exchange between two agents.
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = token_transfers)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TokenTransfer {
    pub id: String,
    pub h_app_id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub amount: f32,
    pub governance_layer: String,
    pub note: Option<String>,
    pub dht_anchor_hash: Option<String>,
    pub created_at: String,
}

/// New token transfer for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = token_transfers)]
pub struct NewTokenTransfer<'a> {
    pub id: &'a str,
    pub h_app_id: &'a str,
    pub from_agent: &'a str,
    pub to_agent: &'a str,
    pub amount: f32,
    pub governance_layer: &'a str,
    pub note: Option<&'a str>,
    pub dht_anchor_hash: Option<&'a str>,
}

// ============================================================================
// Responsibility Demand Config Models (Shefa — elohim-token sprint 2)
// ============================================================================

/// Responsibility demand curve config row from SELECT query.
/// Category C (operational): one config per governance layer per app.
/// The curve parameters encode the community's social contract for coupling
/// token accumulation with obligation (Robeyns capability insight).
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = responsibility_demand_configs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ResponsibilityDemandConfig {
    pub id: String,
    pub h_app_id: String,
    pub governance_layer: String,
    pub dignity_floor: f32,
    pub median_estimate: f32,
    pub soft_ceiling_multiplier: f32,
    pub hard_ceiling_multiplier: f32,
    pub social_contract_health: f32,
    pub enforcement_active: i32,
    pub ratified_by: Option<String>,
    pub ratified_at: Option<String>,
    pub dht_anchor_hash: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// New responsibility demand config for INSERT.
/// `created_at` and `updated_at` are omitted — SQLite provides defaults.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = responsibility_demand_configs)]
pub struct NewResponsibilityDemandConfig<'a> {
    pub id: &'a str,
    pub h_app_id: &'a str,
    pub governance_layer: &'a str,
    pub dignity_floor: f32,
    pub median_estimate: f32,
    pub soft_ceiling_multiplier: f32,
    pub hard_ceiling_multiplier: f32,
    pub social_contract_health: f32,
    pub enforcement_active: i32,
    pub ratified_by: Option<&'a str>,
    pub ratified_at: Option<&'a str>,
    pub dht_anchor_hash: Option<&'a str>,
}

// ============================================================================
// Token Decay Event Models (Shefa — elohim-token sprint 3)
// ============================================================================

/// Token decay event row from SELECT query.
/// Category C (operational): records each periodic decay application against
/// an agent's balance, coupling balance reductions to obligation levels and
/// dignity floor enforcement.
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = token_decay_events)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TokenDecayEvent {
    pub id: String,
    pub h_app_id: String,
    pub agent_id: String,
    pub governance_layer: String,
    pub balance_before: f32,
    pub balance_after: f32,
    pub decay_amount: f32,
    pub obligation_level: String,
    pub dignity_floor: f32,
    pub created_at: String,
}

/// New token decay event for INSERT.
/// `created_at` is omitted — SQLite provides the default via `datetime('now')`.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = token_decay_events)]
pub struct NewTokenDecayEvent<'a> {
    pub id: &'a str,
    pub h_app_id: &'a str,
    pub agent_id: &'a str,
    pub governance_layer: &'a str,
    pub balance_before: f32,
    pub balance_after: f32,
    pub decay_amount: f32,
    pub obligation_level: &'a str,
    pub dignity_floor: f32,
}

// ============================================================================
// Shard Manifest Models (P2P Resilience — Sprint B)
// ============================================================================

/// Shard manifest row from SELECT query.
/// Category C (operational): local projection of encoding state per content item.
/// No dht_anchor_hash — rebuilt from local blob store, not shared via DHT.
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = shard_manifests)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ShardManifestRow {
    pub content_id: String,
    pub h_app_id: String,
    pub blob_hash: String,
    pub blob_cid: Option<String>,
    pub encoding: String,
    pub data_shard_count: i32,
    pub parity_shard_count: i32,
    pub shard_hashes_json: String,
    pub total_size_bytes: i64,
    pub shard_size_bytes: i64,
    pub mime_type: String,
    pub reach: String,
    pub created_at: String,
}

/// New shard manifest for INSERT/REPLACE.
/// `created_at` is omitted — SQLite provides the default via `datetime('now')`.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = shard_manifests)]
pub struct NewShardManifest<'a> {
    pub content_id: &'a str,
    pub h_app_id: &'a str,
    pub blob_hash: &'a str,
    pub blob_cid: Option<&'a str>,
    pub encoding: &'a str,
    pub data_shard_count: i32,
    pub parity_shard_count: i32,
    pub shard_hashes_json: &'a str,
    pub total_size_bytes: i64,
    pub shard_size_bytes: i64,
    pub mime_type: &'a str,
    pub reach: &'a str,
}

// ============================================================================
// Shard Location Models (P2P Resilience — Sprint B)
// ============================================================================

/// Shard location row from SELECT query.
/// Category C (operational): tracks which peers hold which shards.
/// No dht_anchor_hash — rebuilt from shard protocol ack events, not shared via DHT.
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = shard_locations)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ShardLocationRow {
    pub shard_hash: String,
    pub peer_id: String,
    pub h_app_id: String,
    pub status: String,
    pub first_seen: String,
    pub last_verified: Option<String>,
}

/// New shard location for INSERT/REPLACE.
/// `first_seen` is omitted — SQLite provides the default via `datetime('now')`.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = shard_locations)]
pub struct NewShardLocation<'a> {
    pub shard_hash: &'a str,
    pub peer_id: &'a str,
    pub h_app_id: &'a str,
    pub status: &'a str,
}

// ============================================================================
// Observation Sessions (Operational — Category C)
// ============================================================================

/// Active or closed diagnostic observation session.
/// Category C (operational): ephemeral per-node session, not shared via DHT.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = observation_sessions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ObservationSession {
    pub id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub ttl_seconds: i32,
    pub source: String,
    pub metadata_json: Option<String>,
    pub report_content_id: Option<String>,
}

/// New observation session for INSERT.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = observation_sessions)]
pub struct NewObservationSession<'a> {
    pub id: &'a str,
    pub ttl_seconds: i32,
    pub source: &'a str,
    pub metadata_json: Option<&'a str>,
}

/// Single entry appended to an observation session.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = observation_entries)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ObservationEntry {
    pub id: i32,
    pub session_id: String,
    pub timestamp: String,
    pub origin: String,
    pub category: String,
    pub severity: String,
    pub method: Option<String>,
    pub path: Option<String>,
    pub status_code: Option<i32>,
    pub message: String,
    pub context_json: Option<String>,
}

/// New observation entry for INSERT.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = observation_entries)]
pub struct NewObservationEntry<'a> {
    pub session_id: &'a str,
    pub origin: &'a str,
    pub category: &'a str,
    pub severity: &'a str,
    pub method: Option<&'a str>,
    pub path: Option<&'a str>,
    pub status_code: Option<i32>,
    pub message: &'a str,
    pub context_json: Option<&'a str>,
}
