//! View types for HTTP API boundary
//!
//! These types use camelCase serialization for TypeScript clients.
//! Wire types in models.rs use snake_case for database compatibility.
//!
//! Pattern:
//! - Service layer returns Wire types (Path, Content, etc.)
//! - HTTP layer converts to View types (PathView, ContentView, etc.)
//! - ts-rs generates camelCase TypeScript from View types
//!
//! Design principles:
//! - Boolean coercion: SQLite stores bools as i32. Views expose proper bools.
//! - JSON parsing: Internal *_json strings are parsed to serde_json::Value.
//!   This encapsulates storage format and provides typed objects to clients.
//!
//! InputView types (suffix InputView):
//! - Accept camelCase JSON from TypeScript with parsed Value objects
//! - Convert to internal DB Input types (snake_case with String fields)
//! - Encapsulate JSON serialization at the API boundary

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

/// Parse a JSON string to Value, returning null on parse failure.
/// This encapsulates the storage format (TEXT) from the API contract.
fn parse_json_opt(json_str: &Option<String>) -> Option<Value> {
    json_str.as_ref().and_then(|s| serde_json::from_str(s).ok())
}

/// Parse a required JSON string to Value, returning empty object on failure.
fn parse_json(json_str: &str) -> Value {
    serde_json::from_str(json_str).unwrap_or(Value::Object(serde_json::Map::new()))
}

/// Default schema version for InputView types.
/// Clients that omit schemaVersion are implicitly version 1.
fn default_schema_version() -> u32 { 1 }

/// Supported schema versions. Reject anything not in this set.
/// Extend this array when introducing a new schema version.
pub const SUPPORTED_SCHEMA_VERSIONS: &[u32] = &[1];

/// Validate that all schema versions in a batch are supported.
pub fn validate_schema_versions(versions: &[u32]) -> Result<(), String> {
    if let Some(&bad) = versions.iter().find(|v| !SUPPORTED_SCHEMA_VERSIONS.contains(v)) {
        return Err(format!(
            "Unsupported schema version: {}. Supported: {:?}",
            bad, SUPPORTED_SCHEMA_VERSIONS
        ));
    }
    Ok(())
}

use crate::db::models::{
    App, Chapter, ChapterWithSteps, Content, ContentMastery, ContentStewardship, ContentWithTags,
    ContributorPresence, EconomicEvent, HumanRelationship, LocalSession, Path, PathAttestation,
    PathWithDetails, PathWithSteps, Relationship, RelationshipWithContent, Step,
    StewardshipAllocation, StewardshipAllocationWithPresence,
};

// Legacy rusqlite types (used by services until migration complete)
use crate::db::paths::{PathRow, StepRow, ChapterRow, PathWithSteps as LegacyPathWithSteps};
use crate::db::relationships::RelationshipRow;
use crate::db::content::ContentRow;

// ============================================================================
// App View
// ============================================================================

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AppView {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub enabled: bool,
}

impl From<App> for AppView {
    fn from(a: App) -> Self {
        Self {
            id: a.id,
            name: a.name,
            description: a.description,
            created_at: a.created_at,
            enabled: a.enabled == 1,
        }
    }
}

// ============================================================================
// Content Views
// ============================================================================

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentView {
    pub id: String,
    pub app_id: String,
    pub title: String,
    pub description: Option<String>,
    pub content_type: String,
    pub content_format: String,
    pub blob_hash: Option<String>,
    pub blob_cid: Option<String>,
    pub content_size_bytes: Option<i32>,
    /// Parsed metadata object (was metadata_json string in storage)
    pub metadata: Option<Value>,
    pub reach: String,
    pub validation_status: String,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub content_body: Option<String>,
}

impl From<Content> for ContentView {
    fn from(c: Content) -> Self {
        Self {
            id: c.id,
            app_id: c.app_id,
            title: c.title,
            description: c.description,
            content_type: c.content_type,
            content_format: c.content_format,
            blob_hash: c.blob_hash,
            blob_cid: c.blob_cid,
            content_size_bytes: c.content_size_bytes,
            metadata: parse_json_opt(&c.metadata_json),
            reach: c.reach,
            validation_status: c.validation_status,
            created_by: c.created_by,
            created_at: c.created_at,
            updated_at: c.updated_at,
            content_body: c.content_body,
        }
    }
}

// Legacy ContentRow → ContentView (rusqlite)
impl From<ContentRow> for ContentView {
    fn from(c: ContentRow) -> Self {
        Self {
            id: c.id,
            app_id: String::new(), // Legacy doesn't have app_id
            title: c.title,
            description: c.description,
            content_type: c.content_type,
            content_format: c.content_format,
            blob_hash: c.blob_hash,
            blob_cid: c.blob_cid,
            content_size_bytes: c.content_size_bytes.map(|v| v as i32),
            metadata: parse_json_opt(&c.metadata_json),
            reach: c.reach,
            validation_status: c.validation_status,
            created_by: c.created_by,
            created_at: c.created_at,
            updated_at: c.updated_at,
            content_body: c.content_body,
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentWithTagsView {
    #[serde(flatten)]
    pub content: ContentView,
    pub tags: Vec<String>,
}

impl From<ContentWithTags> for ContentWithTagsView {
    fn from(c: ContentWithTags) -> Self {
        Self {
            content: c.content.into(),
            tags: c.tags,
        }
    }
}

// ============================================================================
// Path Views
// ============================================================================

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PathView {
    pub id: String,
    pub app_id: String,
    pub title: String,
    pub description: Option<String>,
    pub path_type: String,
    pub difficulty: Option<String>,
    pub estimated_duration: Option<String>,
    pub thumbnail_url: Option<String>,
    pub thumbnail_alt: Option<String>,
    /// Parsed metadata object (was metadata_json string in storage)
    pub metadata: Option<Value>,
    pub visibility: String,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Path> for PathView {
    fn from(p: Path) -> Self {
        Self {
            id: p.id,
            app_id: p.app_id,
            title: p.title,
            description: p.description,
            path_type: p.path_type,
            difficulty: p.difficulty,
            estimated_duration: p.estimated_duration,
            thumbnail_url: p.thumbnail_url,
            thumbnail_alt: p.thumbnail_alt,
            metadata: parse_json_opt(&p.metadata_json),
            visibility: p.visibility,
            created_by: p.created_by,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

// Legacy PathRow → PathView (rusqlite, missing app_id)
impl From<PathRow> for PathView {
    fn from(p: PathRow) -> Self {
        Self {
            id: p.id,
            app_id: String::new(), // Legacy doesn't have app_id
            title: p.title,
            description: p.description,
            path_type: p.path_type,
            difficulty: p.difficulty,
            estimated_duration: p.estimated_duration,
            thumbnail_url: p.thumbnail_url,
            thumbnail_alt: p.thumbnail_alt,
            metadata: parse_json_opt(&p.metadata_json),
            visibility: p.visibility,
            created_by: p.created_by,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

// ============================================================================
// Chapter Views
// ============================================================================

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ChapterView {
    pub id: String,
    pub app_id: String,
    pub path_id: String,
    pub title: String,
    pub description: Option<String>,
    pub order_index: i32,
    pub estimated_duration: Option<String>,
}

impl From<Chapter> for ChapterView {
    fn from(c: Chapter) -> Self {
        Self {
            id: c.id,
            app_id: c.app_id,
            path_id: c.path_id,
            title: c.title,
            description: c.description,
            order_index: c.order_index,
            estimated_duration: c.estimated_duration,
        }
    }
}

// Legacy ChapterRow → ChapterView (rusqlite)
impl From<ChapterRow> for ChapterView {
    fn from(c: ChapterRow) -> Self {
        Self {
            id: c.id,
            app_id: String::new(), // Legacy doesn't have app_id
            path_id: c.path_id,
            title: c.title,
            description: c.description,
            order_index: c.order_index,
            estimated_duration: c.estimated_duration,
        }
    }
}

// ============================================================================
// Step Views
// ============================================================================

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StepView {
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
    /// Parsed metadata object (was metadata_json string in storage)
    pub metadata: Option<Value>,
}

impl From<Step> for StepView {
    fn from(s: Step) -> Self {
        Self {
            id: s.id,
            app_id: s.app_id,
            path_id: s.path_id,
            chapter_id: s.chapter_id,
            title: s.title,
            description: s.description,
            step_type: s.step_type,
            resource_id: s.resource_id,
            resource_type: s.resource_type,
            order_index: s.order_index,
            estimated_duration: s.estimated_duration,
            metadata: parse_json_opt(&s.metadata_json),
        }
    }
}

// Legacy StepRow → StepView (rusqlite)
impl From<StepRow> for StepView {
    fn from(s: StepRow) -> Self {
        Self {
            id: s.id,
            app_id: String::new(), // Legacy doesn't have app_id
            path_id: s.path_id,
            chapter_id: s.chapter_id,
            title: s.title,
            description: s.description,
            step_type: s.step_type,
            resource_id: s.resource_id,
            resource_type: s.resource_type,
            order_index: s.order_index,
            estimated_duration: s.estimated_duration,
            metadata: parse_json_opt(&s.metadata_json),
        }
    }
}

// ============================================================================
// Path Attestation Views
// ============================================================================

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PathAttestationView {
    pub app_id: String,
    pub path_id: String,
    pub attestation_type: String,
    pub attestation_name: String,
}

impl From<PathAttestation> for PathAttestationView {
    fn from(a: PathAttestation) -> Self {
        Self {
            app_id: a.app_id,
            path_id: a.path_id,
            attestation_type: a.attestation_type,
            attestation_name: a.attestation_name,
        }
    }
}

// ============================================================================
// Composite Path Views
// ============================================================================

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ChapterWithStepsView {
    #[serde(flatten)]
    pub chapter: ChapterView,
    pub steps: Vec<StepView>,
}

impl From<ChapterWithSteps> for ChapterWithStepsView {
    fn from(c: ChapterWithSteps) -> Self {
        Self {
            chapter: c.chapter.into(),
            steps: c.steps.into_iter().map(|s| s.into()).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PathWithDetailsView {
    #[serde(flatten)]
    pub path: PathView,
    pub tags: Vec<String>,
    pub chapters: Vec<ChapterWithStepsView>,
    pub ungrouped_steps: Vec<StepView>,
    pub attestations: Vec<PathAttestationView>,
}

impl From<PathWithDetails> for PathWithDetailsView {
    fn from(p: PathWithDetails) -> Self {
        Self {
            path: p.path.into(),
            tags: p.tags,
            chapters: p.chapters.into_iter().map(|c| c.into()).collect(),
            ungrouped_steps: p.ungrouped_steps.into_iter().map(|s| s.into()).collect(),
            attestations: p.attestations.into_iter().map(|a| a.into()).collect(),
        }
    }
}

// Legacy PathWithSteps → PathWithDetailsView (rusqlite)
// Note: Legacy type doesn't have tags or attestations
impl From<LegacyPathWithSteps> for PathWithDetailsView {
    fn from(p: LegacyPathWithSteps) -> Self {
        Self {
            path: p.path.into(),
            tags: vec![], // Legacy doesn't have tags at this level
            chapters: p.chapters.into_iter().map(|c| {
                ChapterWithStepsView {
                    chapter: ChapterView {
                        id: c.id,
                        app_id: String::new(),
                        path_id: c.path_id,
                        title: c.title,
                        description: c.description,
                        order_index: c.order_index,
                        estimated_duration: c.estimated_duration,
                    },
                    steps: c.steps.into_iter().map(|s| s.into()).collect(),
                }
            }).collect(),
            ungrouped_steps: p.ungrouped_steps.into_iter().map(|s| s.into()).collect(),
            attestations: vec![], // Legacy doesn't have attestations
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PathWithStepsView {
    #[serde(flatten)]
    pub path: PathView,
    pub steps: Vec<StepView>,
}

impl From<PathWithSteps> for PathWithStepsView {
    fn from(p: PathWithSteps) -> Self {
        Self {
            path: p.path.into(),
            steps: p.steps.into_iter().map(|s| s.into()).collect(),
        }
    }
}

// ============================================================================
// Relationship Views
// ============================================================================

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RelationshipView {
    pub id: String,
    pub app_id: String,
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: String,
    pub confidence: f32,
    pub inference_source: String,
    pub is_bidirectional: bool,
    pub inverse_relationship_id: Option<String>,
    /// Parsed provenance chain (was provenance_chain_json string in storage)
    pub provenance_chain: Option<Value>,
    pub governance_layer: Option<String>,
    pub reach: String,
    /// Parsed metadata object (was metadata_json string in storage)
    pub metadata: Option<Value>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Relationship> for RelationshipView {
    fn from(r: Relationship) -> Self {
        Self {
            id: r.id,
            app_id: r.app_id,
            source_id: r.source_id,
            target_id: r.target_id,
            relationship_type: r.relationship_type,
            confidence: r.confidence,
            inference_source: r.inference_source,
            is_bidirectional: r.is_bidirectional == 1,
            inverse_relationship_id: r.inverse_relationship_id,
            provenance_chain: parse_json_opt(&r.provenance_chain_json),
            governance_layer: r.governance_layer,
            reach: r.reach,
            metadata: parse_json_opt(&r.metadata_json),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// Legacy RelationshipRow → RelationshipView (rusqlite)
// Note: Legacy type has fewer fields
impl From<RelationshipRow> for RelationshipView {
    fn from(r: RelationshipRow) -> Self {
        Self {
            id: r.id,
            app_id: String::new(), // Legacy doesn't have app_id
            source_id: r.source_id,
            target_id: r.target_id,
            relationship_type: r.relationship_type,
            confidence: r.confidence as f32, // Legacy uses f64
            inference_source: r.inference_source,
            is_bidirectional: false, // Legacy doesn't have this field
            inverse_relationship_id: None,
            provenance_chain: None,
            governance_layer: None,
            reach: "public".to_string(), // Default reach
            metadata: parse_json_opt(&r.metadata_json),
            created_at: r.created_at.clone(),
            updated_at: r.created_at, // Legacy doesn't have updated_at
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RelationshipWithContentView {
    #[serde(flatten)]
    pub relationship: RelationshipView,
    pub source: Option<ContentView>,
    pub target: Option<ContentView>,
}

impl From<RelationshipWithContent> for RelationshipWithContentView {
    fn from(r: RelationshipWithContent) -> Self {
        Self {
            relationship: r.relationship.into(),
            source: r.source.map(|c| c.into()),
            target: r.target.map(|c| c.into()),
        }
    }
}

// ============================================================================
// Human Relationship Views
// ============================================================================

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct HumanRelationshipView {
    pub id: String,
    pub app_id: String,
    pub party_a_id: String,
    pub party_b_id: String,
    pub relationship_type: String,
    pub intimacy_level: String,
    pub is_bidirectional: bool,
    pub consent_given_by_a: bool,
    pub consent_given_by_b: bool,
    pub custody_enabled_by_a: bool,
    pub custody_enabled_by_b: bool,
    pub auto_custody_enabled: bool,
    pub emergency_access_enabled: bool,
    pub initiated_by: String,
    pub verified_at: Option<String>,
    pub governance_layer: Option<String>,
    pub reach: String,
    /// Parsed context object (was context_json string in storage)
    pub context: Option<Value>,
    pub created_at: String,
    pub updated_at: String,
    pub expires_at: Option<String>,
}

impl From<HumanRelationship> for HumanRelationshipView {
    fn from(h: HumanRelationship) -> Self {
        Self {
            id: h.id,
            app_id: h.app_id,
            party_a_id: h.party_a_id,
            party_b_id: h.party_b_id,
            relationship_type: h.relationship_type,
            intimacy_level: h.intimacy_level,
            is_bidirectional: h.is_bidirectional == 1,
            consent_given_by_a: h.consent_given_by_a == 1,
            consent_given_by_b: h.consent_given_by_b == 1,
            custody_enabled_by_a: h.custody_enabled_by_a == 1,
            custody_enabled_by_b: h.custody_enabled_by_b == 1,
            auto_custody_enabled: h.auto_custody_enabled == 1,
            emergency_access_enabled: h.emergency_access_enabled == 1,
            initiated_by: h.initiated_by,
            verified_at: h.verified_at,
            governance_layer: h.governance_layer,
            reach: h.reach,
            context: parse_json_opt(&h.context_json),
            created_at: h.created_at,
            updated_at: h.updated_at,
            expires_at: h.expires_at,
        }
    }
}

// ============================================================================
// Contributor Presence Views
// ============================================================================

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContributorPresenceView {
    pub id: String,
    pub app_id: String,
    pub display_name: String,
    pub presence_state: String,
    /// Parsed external identifiers (was external_identifiers_json string in storage)
    pub external_identifiers: Option<Value>,
    /// Parsed establishing content IDs (was establishing_content_ids_json string in storage)
    pub establishing_content_ids: Value,
    pub affinity_total: f32,
    pub unique_engagers: i32,
    pub citation_count: i32,
    pub recognition_score: f32,
    /// Parsed recognition by content (was recognition_by_content_json string in storage)
    pub recognition_by_content: Option<Value>,
    pub last_recognition_at: Option<String>,
    pub steward_id: Option<String>,
    pub stewardship_started_at: Option<String>,
    pub stewardship_commitment_id: Option<String>,
    pub stewardship_quality_score: Option<f32>,
    pub claim_initiated_at: Option<String>,
    pub claim_verified_at: Option<String>,
    pub claim_verification_method: Option<String>,
    /// Parsed claim evidence (was claim_evidence_json string in storage)
    pub claim_evidence: Option<Value>,
    pub claimed_agent_id: Option<String>,
    pub claim_recognition_transferred_value: Option<f32>,
    pub claim_facilitated_by: Option<String>,
    pub image: Option<String>,
    pub note: Option<String>,
    /// Parsed metadata object (was metadata_json string in storage)
    pub metadata: Option<Value>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ContributorPresence> for ContributorPresenceView {
    fn from(c: ContributorPresence) -> Self {
        Self {
            id: c.id,
            app_id: c.app_id,
            display_name: c.display_name,
            presence_state: c.presence_state,
            external_identifiers: parse_json_opt(&c.external_identifiers_json),
            establishing_content_ids: parse_json(&c.establishing_content_ids_json),
            affinity_total: c.affinity_total,
            unique_engagers: c.unique_engagers,
            citation_count: c.citation_count,
            recognition_score: c.recognition_score,
            recognition_by_content: parse_json_opt(&c.recognition_by_content_json),
            last_recognition_at: c.last_recognition_at,
            steward_id: c.steward_id,
            stewardship_started_at: c.stewardship_started_at,
            stewardship_commitment_id: c.stewardship_commitment_id,
            stewardship_quality_score: c.stewardship_quality_score,
            claim_initiated_at: c.claim_initiated_at,
            claim_verified_at: c.claim_verified_at,
            claim_verification_method: c.claim_verification_method,
            claim_evidence: parse_json_opt(&c.claim_evidence_json),
            claimed_agent_id: c.claimed_agent_id,
            claim_recognition_transferred_value: c.claim_recognition_transferred_value,
            claim_facilitated_by: c.claim_facilitated_by,
            image: c.image,
            note: c.note,
            metadata: parse_json_opt(&c.metadata_json),
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

// ============================================================================
// Economic Event Views
// ============================================================================

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct EconomicEventView {
    pub id: String,
    pub app_id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    pub resource_conforms_to: Option<String>,
    pub resource_inventoried_as: Option<String>,
    /// Parsed resource classification (was resource_classified_as_json string in storage)
    pub resource_classified_as: Option<Value>,
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
    /// Parsed metadata object (was metadata_json string in storage)
    pub metadata: Option<Value>,
    pub created_at: String,
}

impl From<EconomicEvent> for EconomicEventView {
    fn from(e: EconomicEvent) -> Self {
        Self {
            id: e.id,
            app_id: e.app_id,
            action: e.action,
            provider: e.provider,
            receiver: e.receiver,
            resource_conforms_to: e.resource_conforms_to,
            resource_inventoried_as: e.resource_inventoried_as,
            resource_classified_as: parse_json_opt(&e.resource_classified_as_json),
            resource_quantity_value: e.resource_quantity_value,
            resource_quantity_unit: e.resource_quantity_unit,
            effort_quantity_value: e.effort_quantity_value,
            effort_quantity_unit: e.effort_quantity_unit,
            has_point_in_time: e.has_point_in_time,
            has_duration: e.has_duration,
            input_of: e.input_of,
            output_of: e.output_of,
            lamad_event_type: e.lamad_event_type,
            content_id: e.content_id,
            contributor_presence_id: e.contributor_presence_id,
            path_id: e.path_id,
            triggered_by: e.triggered_by,
            state: e.state,
            note: e.note,
            metadata: parse_json_opt(&e.metadata_json),
            created_at: e.created_at,
        }
    }
}

// ============================================================================
// Content Mastery Views
// ============================================================================

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentMasteryView {
    pub id: String,
    pub app_id: String,
    pub human_id: String,
    pub content_id: String,
    pub mastery_level: String,
    pub mastery_level_index: i32,
    pub freshness_score: f32,
    pub needs_refresh: bool,
    pub engagement_count: i32,
    pub last_engagement_type: Option<String>,
    pub last_engagement_at: Option<String>,
    pub level_achieved_at: Option<String>,
    pub content_version_at_mastery: Option<String>,
    /// Parsed assessment evidence (was assessment_evidence_json string in storage)
    pub assessment_evidence: Option<Value>,
    /// Parsed privileges (was privileges_json string in storage)
    pub privileges: Option<Value>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ContentMastery> for ContentMasteryView {
    fn from(m: ContentMastery) -> Self {
        Self {
            id: m.id,
            app_id: m.app_id,
            human_id: m.human_id,
            content_id: m.content_id,
            mastery_level: m.mastery_level,
            mastery_level_index: m.mastery_level_index,
            freshness_score: m.freshness_score,
            needs_refresh: m.needs_refresh == 1,
            engagement_count: m.engagement_count,
            last_engagement_type: m.last_engagement_type,
            last_engagement_at: m.last_engagement_at,
            level_achieved_at: m.level_achieved_at,
            content_version_at_mastery: m.content_version_at_mastery,
            assessment_evidence: parse_json_opt(&m.assessment_evidence_json),
            privileges: parse_json_opt(&m.privileges_json),
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

// ============================================================================
// Stewardship Allocation Views
// ============================================================================

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StewardshipAllocationView {
    pub id: String,
    pub app_id: String,
    pub content_id: String,
    pub steward_presence_id: String,
    pub allocation_ratio: f32,
    pub allocation_method: String,
    pub contribution_type: String,
    /// Parsed contribution evidence (was contribution_evidence_json string in storage)
    pub contribution_evidence: Option<Value>,
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
    /// Parsed metadata object (was metadata_json string in storage)
    pub metadata: Option<Value>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<StewardshipAllocation> for StewardshipAllocationView {
    fn from(a: StewardshipAllocation) -> Self {
        Self {
            id: a.id,
            app_id: a.app_id,
            content_id: a.content_id,
            steward_presence_id: a.steward_presence_id,
            allocation_ratio: a.allocation_ratio,
            allocation_method: a.allocation_method,
            contribution_type: a.contribution_type,
            contribution_evidence: parse_json_opt(&a.contribution_evidence_json),
            governance_state: a.governance_state,
            dispute_id: a.dispute_id,
            dispute_reason: a.dispute_reason,
            disputed_at: a.disputed_at,
            disputed_by: a.disputed_by,
            negotiation_session_id: a.negotiation_session_id,
            elohim_ratified_at: a.elohim_ratified_at,
            elohim_ratifier_id: a.elohim_ratifier_id,
            effective_from: a.effective_from,
            effective_until: a.effective_until,
            superseded_by: a.superseded_by,
            recognition_accumulated: a.recognition_accumulated,
            last_recognition_at: a.last_recognition_at,
            note: a.note,
            metadata: parse_json_opt(&a.metadata_json),
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StewardshipAllocationWithPresenceView {
    #[serde(flatten)]
    pub allocation: StewardshipAllocationView,
    pub steward: Option<ContributorPresenceView>,
}

impl From<StewardshipAllocationWithPresence> for StewardshipAllocationWithPresenceView {
    fn from(a: StewardshipAllocationWithPresence) -> Self {
        Self {
            allocation: a.allocation.into(),
            steward: a.steward.map(|s| s.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentStewardshipView {
    pub content_id: String,
    pub allocations: Vec<StewardshipAllocationWithPresenceView>,
    pub total_allocation: f32,
    pub has_disputes: bool,
    pub primary_steward: Option<StewardshipAllocationView>,
}

impl From<ContentStewardship> for ContentStewardshipView {
    fn from(s: ContentStewardship) -> Self {
        Self {
            content_id: s.content_id,
            allocations: s.allocations.into_iter().map(|a| a.into()).collect(),
            total_allocation: s.total_allocation,
            has_disputes: s.has_disputes,
            primary_steward: s.primary_steward.map(|a| a.into()),
        }
    }
}

// ============================================================================
// Local Session Views
// ============================================================================

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct LocalSessionView {
    pub id: String,
    pub human_id: String,
    pub agent_pub_key: String,
    pub doorway_url: String,
    pub doorway_id: Option<String>,
    pub identifier: String,
    pub display_name: Option<String>,
    pub profile_image_hash: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub last_synced_at: Option<String>,
    pub bootstrap_url: Option<String>,
}

impl From<LocalSession> for LocalSessionView {
    fn from(s: LocalSession) -> Self {
        Self {
            id: s.id,
            human_id: s.human_id,
            agent_pub_key: s.agent_pub_key,
            doorway_url: s.doorway_url,
            doorway_id: s.doorway_id,
            identifier: s.identifier,
            display_name: s.display_name,
            profile_image_hash: s.profile_image_hash,
            is_active: s.is_active == 1,
            created_at: s.created_at,
            updated_at: s.updated_at,
            last_synced_at: s.last_synced_at,
            bootstrap_url: s.bootstrap_url,
        }
    }
}

// ============================================================================
// Input View Types (API boundary for writes)
// ============================================================================
//
// These types accept camelCase JSON from TypeScript clients with parsed Value
// objects. They convert to internal DB Input types which use snake_case with
// String fields. This encapsulates JSON serialization at the API boundary.

/// Serialize a Value to JSON string for DB storage, or None if null/absent.
fn serialize_json_opt(value: &Option<Value>) -> Option<String> {
    value.as_ref().map(|v| v.to_string())
}

// ============================================================================
// Content Input Views
// ============================================================================

use crate::db::content::CreateContentInput;

/// Input for creating content - camelCase API boundary type
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateContentInputView {
    pub id: String,
    pub title: String,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub content_format: Option<String>,
    #[serde(default)]
    pub content_body: Option<String>,
    #[serde(default)]
    pub blob_hash: Option<String>,
    #[serde(default)]
    pub blob_cid: Option<String>,
    #[serde(default)]
    pub content_size_bytes: Option<i64>,
    /// Parsed metadata object (serialized to JSON string for DB)
    #[serde(default)]
    pub metadata: Option<Value>,
    #[serde(default)]
    pub reach: Option<String>,
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl From<CreateContentInputView> for CreateContentInput {
    fn from(v: CreateContentInputView) -> Self {
        Self {
            id: v.id,
            title: v.title,
            description: v.description,
            content_type: v.content_type.unwrap_or_else(|| "concept".to_string()),
            content_format: v.content_format.unwrap_or_else(|| "markdown".to_string()),
            content_body: v.content_body,
            blob_hash: v.blob_hash,
            blob_cid: v.blob_cid,
            content_size_bytes: v.content_size_bytes,
            metadata_json: serialize_json_opt(&v.metadata),
            reach: v.reach.unwrap_or_else(|| "public".to_string()),
            created_by: v.created_by,
            tags: v.tags,
        }
    }
}

// ============================================================================
// Path Input Views
// ============================================================================

use crate::db::paths::{CreatePathInput, CreateChapterInput, CreateStepInput};

/// Input for creating a step - camelCase API boundary type
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateStepInputView {
    pub id: String,
    pub path_id: String,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub chapter_id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub step_type: Option<String>,
    #[serde(default)]
    pub resource_id: Option<String>,
    #[serde(default)]
    pub resource_type: Option<String>,
    #[serde(default)]
    pub order_index: i32,
    #[serde(default)]
    pub estimated_duration: Option<String>,
    /// Parsed metadata object (serialized to JSON string for DB)
    #[serde(default)]
    pub metadata: Option<Value>,
}

impl From<CreateStepInputView> for CreateStepInput {
    fn from(v: CreateStepInputView) -> Self {
        Self {
            id: v.id,
            path_id: v.path_id,
            chapter_id: v.chapter_id,
            title: v.title,
            description: v.description,
            step_type: v.step_type.unwrap_or_else(|| "learn".to_string()),
            resource_id: v.resource_id,
            resource_type: v.resource_type,
            order_index: v.order_index,
            estimated_duration: v.estimated_duration,
            metadata_json: serialize_json_opt(&v.metadata),
        }
    }
}

/// Input for creating a chapter - camelCase API boundary type
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateChapterInputView {
    pub id: String,
    pub title: String,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub order_index: i32,
    #[serde(default)]
    pub estimated_duration: Option<String>,
    #[serde(default)]
    pub steps: Vec<CreateStepInputView>,
}

impl From<CreateChapterInputView> for CreateChapterInput {
    fn from(v: CreateChapterInputView) -> Self {
        Self {
            id: v.id,
            title: v.title,
            description: v.description,
            order_index: v.order_index,
            estimated_duration: v.estimated_duration,
            steps: v.steps.into_iter().map(|s| s.into()).collect(),
        }
    }
}

/// Input for creating a path - camelCase API boundary type
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreatePathInputView {
    pub id: String,
    pub title: String,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub path_type: Option<String>,
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub estimated_duration: Option<String>,
    #[serde(default)]
    pub thumbnail_url: Option<String>,
    #[serde(default)]
    pub thumbnail_alt: Option<String>,
    /// Parsed metadata object (serialized to JSON string for DB)
    #[serde(default)]
    pub metadata: Option<Value>,
    #[serde(default)]
    pub visibility: Option<String>,
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub chapters: Vec<CreateChapterInputView>,
}

impl From<CreatePathInputView> for CreatePathInput {
    fn from(v: CreatePathInputView) -> Self {
        Self {
            id: v.id,
            title: v.title,
            description: v.description,
            path_type: v.path_type.unwrap_or_else(|| "guided".to_string()),
            difficulty: v.difficulty,
            estimated_duration: v.estimated_duration,
            thumbnail_url: v.thumbnail_url,
            thumbnail_alt: v.thumbnail_alt,
            metadata_json: serialize_json_opt(&v.metadata),
            visibility: v.visibility.unwrap_or_else(|| "public".to_string()),
            created_by: v.created_by,
            tags: v.tags,
            chapters: v.chapters.into_iter().map(|c| c.into()).collect(),
        }
    }
}

// ============================================================================
// Relationship Input Views
// ============================================================================

use crate::db::relationships::CreateRelationshipInput;

/// Input for creating a relationship - camelCase API boundary type
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateRelationshipInputView {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: String,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub inference_source: Option<String>,
    /// Parsed metadata object (serialized to JSON string for DB)
    #[serde(default)]
    pub metadata: Option<Value>,
}

impl From<CreateRelationshipInputView> for CreateRelationshipInput {
    fn from(v: CreateRelationshipInputView) -> Self {
        Self {
            id: v.id,
            source_id: v.source_id,
            target_id: v.target_id,
            relationship_type: v.relationship_type,
            confidence: v.confidence.unwrap_or(1.0),
            inference_source: v.inference_source.unwrap_or_else(|| "explicit".to_string()),
            metadata_json: serialize_json_opt(&v.metadata),
        }
    }
}

// ============================================================================
// Human Relationship Input Views
// ============================================================================

use crate::db::human_relationships::CreateHumanRelationshipInput;

/// Input for creating a human relationship - camelCase API boundary type
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateHumanRelationshipInputView {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub party_a_id: String,
    pub party_b_id: String,
    pub relationship_type: String,
    #[serde(default)]
    pub intimacy_level: Option<String>,
    #[serde(default)]
    pub is_bidirectional: bool,
    #[serde(default)]
    pub consent_given_by_a: bool,
    #[serde(default)]
    pub consent_given_by_b: bool,
    pub initiated_by: String,
    #[serde(default)]
    pub governance_layer: Option<String>,
    #[serde(default)]
    pub reach: Option<String>,
    /// Parsed context object (serialized to JSON string for DB)
    #[serde(default)]
    pub context: Option<Value>,
    #[serde(default)]
    pub expires_at: Option<String>,
}

impl From<CreateHumanRelationshipInputView> for CreateHumanRelationshipInput {
    fn from(v: CreateHumanRelationshipInputView) -> Self {
        Self {
            id: v.id,
            party_a_id: v.party_a_id,
            party_b_id: v.party_b_id,
            relationship_type: v.relationship_type,
            intimacy_level: v.intimacy_level.unwrap_or_else(|| "recognition".to_string()),
            is_bidirectional: v.is_bidirectional,
            consent_given_by_a: v.consent_given_by_a,
            consent_given_by_b: v.consent_given_by_b,
            initiated_by: v.initiated_by,
            governance_layer: v.governance_layer,
            reach: v.reach.unwrap_or_else(|| "private".to_string()),
            context_json: serialize_json_opt(&v.context),
            expires_at: v.expires_at,
        }
    }
}

// ============================================================================
// Contributor Presence Input Views
// ============================================================================

use crate::db::contributor_presences::{CreateContributorPresenceInput, InitiateClaimInput};

/// Input for creating a contributor presence - camelCase API boundary type
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateContributorPresenceInputView {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub display_name: String,
    /// Parsed external identifiers (serialized to JSON string for DB)
    #[serde(default)]
    pub external_identifiers: Option<Value>,
    pub establishing_content_ids: Vec<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    /// Parsed metadata object (serialized to JSON string for DB)
    #[serde(default)]
    pub metadata: Option<Value>,
}

impl From<CreateContributorPresenceInputView> for CreateContributorPresenceInput {
    fn from(v: CreateContributorPresenceInputView) -> Self {
        Self {
            id: v.id,
            display_name: v.display_name,
            external_identifiers_json: serialize_json_opt(&v.external_identifiers),
            establishing_content_ids: v.establishing_content_ids,
            image: v.image,
            note: v.note,
            metadata_json: serialize_json_opt(&v.metadata),
        }
    }
}

/// Input for initiating a claim - camelCase API boundary type
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct InitiateClaimInputView {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub claiming_agent_id: String,
    pub verification_method: String,
    /// Parsed evidence object (serialized to JSON string for DB)
    #[serde(default)]
    pub evidence: Option<Value>,
    #[serde(default)]
    pub facilitated_by: Option<String>,
}

impl From<InitiateClaimInputView> for InitiateClaimInput {
    fn from(v: InitiateClaimInputView) -> Self {
        Self {
            claiming_agent_id: v.claiming_agent_id,
            verification_method: v.verification_method,
            evidence_json: serialize_json_opt(&v.evidence),
            facilitated_by: v.facilitated_by,
        }
    }
}

// ============================================================================
// Economic Event Input Views
// ============================================================================

use crate::db::economic_events::CreateEconomicEventInput;

/// Input for creating an economic event - camelCase API boundary type
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateEconomicEventInputView {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    #[serde(default)]
    pub resource_conforms_to: Option<String>,
    #[serde(default)]
    pub resource_inventoried_as: Option<String>,
    #[serde(default)]
    pub resource_classified_as: Vec<String>,
    #[serde(default)]
    pub resource_quantity_value: Option<f32>,
    #[serde(default)]
    pub resource_quantity_unit: Option<String>,
    #[serde(default)]
    pub effort_quantity_value: Option<f32>,
    #[serde(default)]
    pub effort_quantity_unit: Option<String>,
    #[serde(default)]
    pub has_point_in_time: Option<String>,
    #[serde(default)]
    pub has_duration: Option<String>,
    #[serde(default)]
    pub input_of: Option<String>,
    #[serde(default)]
    pub output_of: Option<String>,
    #[serde(default)]
    pub lamad_event_type: Option<String>,
    #[serde(default)]
    pub content_id: Option<String>,
    #[serde(default)]
    pub contributor_presence_id: Option<String>,
    #[serde(default)]
    pub path_id: Option<String>,
    #[serde(default)]
    pub triggered_by: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    /// Parsed metadata object (serialized to JSON string for DB)
    #[serde(default)]
    pub metadata: Option<Value>,
}

impl From<CreateEconomicEventInputView> for CreateEconomicEventInput {
    fn from(v: CreateEconomicEventInputView) -> Self {
        Self {
            id: v.id,
            action: v.action,
            provider: v.provider,
            receiver: v.receiver,
            resource_conforms_to: v.resource_conforms_to,
            resource_inventoried_as: v.resource_inventoried_as,
            resource_classified_as: v.resource_classified_as,
            resource_quantity_value: v.resource_quantity_value,
            resource_quantity_unit: v.resource_quantity_unit,
            effort_quantity_value: v.effort_quantity_value,
            effort_quantity_unit: v.effort_quantity_unit,
            has_point_in_time: v.has_point_in_time,
            has_duration: v.has_duration,
            input_of: v.input_of,
            output_of: v.output_of,
            lamad_event_type: v.lamad_event_type,
            content_id: v.content_id,
            contributor_presence_id: v.contributor_presence_id,
            path_id: v.path_id,
            triggered_by: v.triggered_by,
            note: v.note,
            metadata_json: serialize_json_opt(&v.metadata),
        }
    }
}

// ============================================================================
// Stewardship Allocation Input Views
// ============================================================================

use crate::db::stewardship_allocations::{CreateAllocationInput, UpdateAllocationInput};

/// Input for creating a stewardship allocation - camelCase API boundary type
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateAllocationInputView {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub content_id: String,
    pub steward_presence_id: String,
    #[serde(default)]
    pub allocation_ratio: Option<f32>,
    #[serde(default)]
    pub allocation_method: Option<String>,
    #[serde(default)]
    pub contribution_type: Option<String>,
    /// Parsed contribution evidence (serialized to JSON string for DB)
    #[serde(default)]
    pub contribution_evidence: Option<Value>,
    #[serde(default)]
    pub note: Option<String>,
    /// Parsed metadata object (serialized to JSON string for DB)
    #[serde(default)]
    pub metadata: Option<Value>,
}

impl From<CreateAllocationInputView> for CreateAllocationInput {
    fn from(v: CreateAllocationInputView) -> Self {
        Self {
            content_id: v.content_id,
            steward_presence_id: v.steward_presence_id,
            allocation_ratio: v.allocation_ratio.unwrap_or(1.0),
            allocation_method: v.allocation_method.unwrap_or_else(|| "manual".to_string()),
            contribution_type: v.contribution_type.unwrap_or_else(|| "inherited".to_string()),
            contribution_evidence_json: serialize_json_opt(&v.contribution_evidence),
            note: v.note,
            metadata_json: serialize_json_opt(&v.metadata),
        }
    }
}

/// Input for updating a stewardship allocation - camelCase API boundary type
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct UpdateAllocationInputView {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub allocation_ratio: Option<f32>,
    #[serde(default)]
    pub allocation_method: Option<String>,
    #[serde(default)]
    pub contribution_type: Option<String>,
    /// Parsed contribution evidence (serialized to JSON string for DB)
    #[serde(default)]
    pub contribution_evidence: Option<Value>,
    #[serde(default)]
    pub governance_state: Option<String>,
    #[serde(default)]
    pub dispute_id: Option<String>,
    #[serde(default)]
    pub dispute_reason: Option<String>,
    #[serde(default)]
    pub elohim_ratified_at: Option<String>,
    #[serde(default)]
    pub elohim_ratifier_id: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

impl From<UpdateAllocationInputView> for UpdateAllocationInput {
    fn from(v: UpdateAllocationInputView) -> Self {
        Self {
            allocation_ratio: v.allocation_ratio,
            allocation_method: v.allocation_method,
            contribution_type: v.contribution_type,
            contribution_evidence_json: serialize_json_opt(&v.contribution_evidence),
            governance_state: v.governance_state,
            dispute_id: v.dispute_id,
            dispute_reason: v.dispute_reason,
            elohim_ratified_at: v.elohim_ratified_at,
            elohim_ratifier_id: v.elohim_ratifier_id,
            note: v.note,
        }
    }
}

// ============================================================================
// Schema Version Tests
// ============================================================================

#[cfg(test)]
mod schema_version_tests {
    use super::*;

    #[test]
    fn default_schema_version_is_one() {
        // Missing schemaVersion field defaults to 1
        let json = r#"{"id":"test","title":"Test"}"#;
        let view: CreateContentInputView = serde_json::from_str(json).unwrap();
        assert_eq!(view.schema_version, 1);
    }

    #[test]
    fn explicit_schema_version_is_preserved() {
        let json = r#"{"id":"test","title":"Test","schemaVersion":2}"#;
        let view: CreateContentInputView = serde_json::from_str(json).unwrap();
        assert_eq!(view.schema_version, 2);
    }

    #[test]
    fn unknown_fields_are_silently_ignored() {
        // Tolerant reader: future fields don't break deserialization
        let json = r#"{"id":"test","title":"Test","futureField":"ignored","anotherNew":42}"#;
        let view: CreateContentInputView = serde_json::from_str(json).unwrap();
        assert_eq!(view.id, "test");
        assert_eq!(view.schema_version, 1);
    }

    #[test]
    fn all_input_views_accept_schema_version() {
        // Verify schema_version works across representative InputView types
        let content: CreateContentInputView = serde_json::from_str(
            r#"{"id":"c","title":"T","schemaVersion":3}"#
        ).unwrap();
        assert_eq!(content.schema_version, 3);

        let rel: CreateRelationshipInputView = serde_json::from_str(
            r#"{"sourceId":"a","targetId":"b","relationshipType":"relates","schemaVersion":2}"#
        ).unwrap();
        assert_eq!(rel.schema_version, 2);

        let event: CreateEconomicEventInputView = serde_json::from_str(
            r#"{"action":"use","provider":"p","receiver":"r","schemaVersion":5}"#
        ).unwrap();
        assert_eq!(event.schema_version, 5);
    }

    /// Compile-time lint: every InputView MUST have schema_version.
    /// If you add a new InputView struct without schema_version, this test
    /// will fail to compile. Add the field following the existing pattern:
    ///   #[serde(default = "default_schema_version")]
    ///   pub schema_version: u32,
    #[test]
    fn all_input_views_have_schema_version_field() {
        // Every InputView type must appear here. If you add a new one, add it below.
        let content: CreateContentInputView = serde_json::from_value(
            serde_json::json!({"id":"x","title":"x"})
        ).unwrap();
        let step: CreateStepInputView = serde_json::from_value(
            serde_json::json!({"id":"x","pathId":"p","title":"x"})
        ).unwrap();
        let chapter: CreateChapterInputView = serde_json::from_value(
            serde_json::json!({"id":"x","title":"x"})
        ).unwrap();
        let path: CreatePathInputView = serde_json::from_value(
            serde_json::json!({"id":"x","title":"x"})
        ).unwrap();
        let rel: CreateRelationshipInputView = serde_json::from_value(
            serde_json::json!({"sourceId":"a","targetId":"b","relationshipType":"r"})
        ).unwrap();
        let human_rel: CreateHumanRelationshipInputView = serde_json::from_value(
            serde_json::json!({"partyAId":"a","partyBId":"b","relationshipType":"r","initiatedBy":"a"})
        ).unwrap();
        let presence: CreateContributorPresenceInputView = serde_json::from_value(
            serde_json::json!({"displayName":"x","establishingContentIds":[]})
        ).unwrap();
        let claim: InitiateClaimInputView = serde_json::from_value(
            serde_json::json!({"claimingAgentId":"a","verificationMethod":"m"})
        ).unwrap();
        let event: CreateEconomicEventInputView = serde_json::from_value(
            serde_json::json!({"action":"use","provider":"p","receiver":"r"})
        ).unwrap();
        let alloc: CreateAllocationInputView = serde_json::from_value(
            serde_json::json!({"contentId":"c","stewardPresenceId":"s"})
        ).unwrap();
        let update_alloc: UpdateAllocationInputView = serde_json::from_value(
            serde_json::json!({})
        ).unwrap();

        // The lint: accessing .schema_version on each. Fails to compile if missing.
        assert_eq!(content.schema_version, 1);
        assert_eq!(step.schema_version, 1);
        assert_eq!(chapter.schema_version, 1);
        assert_eq!(path.schema_version, 1);
        assert_eq!(rel.schema_version, 1);
        assert_eq!(human_rel.schema_version, 1);
        assert_eq!(presence.schema_version, 1);
        assert_eq!(claim.schema_version, 1);
        assert_eq!(event.schema_version, 1);
        assert_eq!(alloc.schema_version, 1);
        assert_eq!(update_alloc.schema_version, 1);
    }

    #[test]
    fn validate_supported_version_accepted() {
        assert!(super::validate_schema_versions(&[1]).is_ok());
    }

    #[test]
    fn validate_unsupported_version_rejected() {
        let err = super::validate_schema_versions(&[99]).unwrap_err();
        assert!(err.contains("Unsupported schema version: 99"));
        assert!(err.contains("Supported:"));
    }

    #[test]
    fn validate_empty_batch_ok() {
        assert!(super::validate_schema_versions(&[]).is_ok());
    }

    #[test]
    fn supported_versions_includes_default() {
        assert!(super::SUPPORTED_SCHEMA_VERSIONS.contains(&super::default_schema_version()));
    }
}
