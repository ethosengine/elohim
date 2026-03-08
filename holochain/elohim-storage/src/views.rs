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

/// Wrapper for `serde_json::Value` that controls ts-rs export location.
///
/// This replaces the `serde-json-impl` feature of ts-rs, which exports
/// `JsonValue.ts` to `bindings/serde_json/` — a different directory than
/// our View types. When other generated files import `JsonValue`, ts-rs
/// calculates a cross-directory relative path that breaks at build time.
///
/// By owning the type locally, we set `export_to` to the same directory
/// as all View types, so all imports resolve as `"./JsonValue"`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(transparent)]
#[ts(
    export,
    export_to = "../../sdk/storage-client-ts/src/generated/",
    rename = "JsonValue"
)]
pub struct JsonVal(
    #[ts(
        type = "number | string | boolean | Array<JsonValue> | { [key in string]?: JsonValue } | null"
    )]
    pub Value,
);

/// Parse a JSON string to JsonVal, returning None on parse failure.
/// This encapsulates the storage format (TEXT) from the API contract.
fn parse_json_opt(json_str: &Option<String>) -> Option<JsonVal> {
    json_str
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .map(JsonVal)
}

/// Parse a required JSON string to JsonVal, returning empty object on failure.
fn parse_json(json_str: &str) -> JsonVal {
    JsonVal(serde_json::from_str(json_str).unwrap_or(Value::Object(serde_json::Map::new())))
}

/// Default schema version for InputView types.
/// Clients that omit schemaVersion are implicitly version 1.
fn default_schema_version() -> u32 {
    1
}

/// Supported schema versions. Reject anything not in this set.
/// Extend this array when introducing a new schema version.
pub const SUPPORTED_SCHEMA_VERSIONS: &[u32] = &[1];

/// Validate that all schema versions in a batch are supported.
pub fn validate_schema_versions(versions: &[u32]) -> Result<(), String> {
    if let Some(&bad) = versions
        .iter()
        .find(|v| !SUPPORTED_SCHEMA_VERSIONS.contains(v))
    {
        return Err(format!(
            "Unsupported schema version: {}. Supported: {:?}",
            bad, SUPPORTED_SCHEMA_VERSIONS
        ));
    }
    Ok(())
}

use crate::db::models::{
    App, Chapter, ChapterWithSteps, Content, ContentMastery, ContentStewardship, ContentWithTags,
    ContributorPresence, CustodianMetrics, EconomicEvent, Human, HumanRelationship, LocalSession,
    Path, PathAttestation, PathWithDetails, PathWithSteps, Relationship, RelationshipWithContent,
    Step, StewardshipAllocation, StewardshipAllocationWithPresence,
};

// Legacy rusqlite types (used by services until migration complete)
use crate::db::content::ContentRow;
use crate::db::paths::{ChapterRow, PathRow, PathWithSteps as LegacyPathWithSteps, StepRow};
use crate::db::relationships::RelationshipRow;

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
    pub metadata: Option<JsonVal>,
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
    pub metadata: Option<JsonVal>,
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
    pub metadata: Option<JsonVal>,
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
            chapters: p
                .chapters
                .into_iter()
                .map(|c| ChapterWithStepsView {
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
                })
                .collect(),
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
    pub provenance_chain: Option<JsonVal>,
    pub governance_layer: Option<String>,
    pub reach: String,
    /// Parsed metadata object (was metadata_json string in storage)
    pub metadata: Option<JsonVal>,
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
    pub context: Option<JsonVal>,
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
    pub external_identifiers: Option<JsonVal>,
    /// Parsed establishing content IDs (was establishing_content_ids_json string in storage)
    pub establishing_content_ids: JsonVal,
    pub affinity_total: f32,
    pub unique_engagers: i32,
    pub citation_count: i32,
    pub recognition_score: f32,
    /// Parsed recognition by content (was recognition_by_content_json string in storage)
    pub recognition_by_content: Option<JsonVal>,
    pub last_recognition_at: Option<String>,
    pub steward_id: Option<String>,
    pub stewardship_started_at: Option<String>,
    pub stewardship_commitment_id: Option<String>,
    pub stewardship_quality_score: Option<f32>,
    pub claim_initiated_at: Option<String>,
    pub claim_verified_at: Option<String>,
    pub claim_verification_method: Option<String>,
    /// Parsed claim evidence (was claim_evidence_json string in storage)
    pub claim_evidence: Option<JsonVal>,
    pub claimed_agent_id: Option<String>,
    pub claim_recognition_transferred_value: Option<f32>,
    pub claim_facilitated_by: Option<String>,
    pub image: Option<String>,
    pub note: Option<String>,
    /// Parsed metadata object (was metadata_json string in storage)
    pub metadata: Option<JsonVal>,
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
    pub resource_classified_as: Option<JsonVal>,
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
    pub metadata: Option<JsonVal>,
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
    pub assessment_evidence: Option<JsonVal>,
    /// Parsed privileges (was privileges_json string in storage)
    pub privileges: Option<JsonVal>,
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
    pub contribution_evidence: Option<JsonVal>,
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
    pub metadata: Option<JsonVal>,
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
// Device Policy Views (Stewardship v5)
// ============================================================================

use crate::db::models::DevicePolicy;

/// Device policy view — camelCase API boundary
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct DevicePolicyView {
    pub id: String,
    pub subject_id: String,
    pub device_id: Option<String>,
    pub author_id: String,
    pub author_tier: String,
    pub inherits_from: Option<String>,
    pub blocked_categories: JsonVal,
    pub blocked_hashes: JsonVal,
    pub age_rating_max: Option<String>,
    pub reach_level_max: Option<i32>,
    pub session_max_minutes: Option<i32>,
    pub daily_max_minutes: Option<i32>,
    pub time_windows: JsonVal,
    pub cooldown_minutes: Option<i32>,
    pub disabled_features: JsonVal,
    pub disabled_routes: JsonVal,
    pub require_approval: JsonVal,
    pub log_sessions: bool,
    pub log_categories: bool,
    pub log_policy_events: bool,
    pub retention_days: i32,
    pub subject_can_view: bool,
    pub effective_from: String,
    pub effective_until: Option<String>,
    pub version: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl From<DevicePolicy> for DevicePolicyView {
    fn from(p: DevicePolicy) -> Self {
        Self {
            id: p.id,
            subject_id: p.subject_id,
            device_id: p.device_id,
            author_id: p.author_id,
            author_tier: p.author_tier,
            inherits_from: p.inherits_from,
            blocked_categories: parse_json(&p.blocked_categories_json),
            blocked_hashes: parse_json(&p.blocked_hashes_json),
            age_rating_max: p.age_rating_max,
            reach_level_max: p.reach_level_max,
            session_max_minutes: p.session_max_minutes,
            daily_max_minutes: p.daily_max_minutes,
            time_windows: parse_json(&p.time_windows_json),
            cooldown_minutes: p.cooldown_minutes,
            disabled_features: parse_json(&p.disabled_features_json),
            disabled_routes: parse_json(&p.disabled_routes_json),
            require_approval: parse_json(&p.require_approval_json),
            log_sessions: p.log_sessions != 0,
            log_categories: p.log_categories != 0,
            log_policy_events: p.log_policy_events != 0,
            retention_days: p.retention_days,
            subject_can_view: p.subject_can_view != 0,
            effective_from: p.effective_from,
            effective_until: p.effective_until,
            version: p.version,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

/// Policy chain link — one layer in the policy inheritance chain
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PolicyChainLinkView {
    pub policy_id: String,
    pub author_tier: String,
    pub layer_order: i32,
}

/// Time access decision — result of time-based policy check
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "status")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum TimeAccessView {
    #[serde(rename = "allowed")]
    Allowed {
        remaining_session: Option<u32>,
        remaining_daily: Option<u32>,
    },
    #[serde(rename = "outside_window")]
    OutsideWindow,
    #[serde(rename = "session_limit")]
    SessionLimit,
    #[serde(rename = "daily_limit")]
    DailyLimit,
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

/// Serialize a JsonVal to JSON string for DB storage, or None if null/absent.
fn serialize_json_opt(value: &Option<JsonVal>) -> Option<String> {
    value.as_ref().map(|v| v.0.to_string())
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
    pub metadata: Option<JsonVal>,
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

use crate::db::paths::{CreateChapterInput, CreatePathInput, CreateStepInput};

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
    pub metadata: Option<JsonVal>,
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
    pub metadata: Option<JsonVal>,
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
    pub metadata: Option<JsonVal>,
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
    pub context: Option<JsonVal>,
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
            intimacy_level: v
                .intimacy_level
                .unwrap_or_else(|| "recognition".to_string()),
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
    pub external_identifiers: Option<JsonVal>,
    pub establishing_content_ids: Vec<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    /// Parsed metadata object (serialized to JSON string for DB)
    #[serde(default)]
    pub metadata: Option<JsonVal>,
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
    pub evidence: Option<JsonVal>,
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
    pub metadata: Option<JsonVal>,
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
    pub contribution_evidence: Option<JsonVal>,
    #[serde(default)]
    pub note: Option<String>,
    /// Parsed metadata object (serialized to JSON string for DB)
    #[serde(default)]
    pub metadata: Option<JsonVal>,
}

impl From<CreateAllocationInputView> for CreateAllocationInput {
    fn from(v: CreateAllocationInputView) -> Self {
        Self {
            content_id: v.content_id,
            steward_presence_id: v.steward_presence_id,
            allocation_ratio: v.allocation_ratio.unwrap_or(1.0),
            allocation_method: v.allocation_method.unwrap_or_else(|| "manual".to_string()),
            contribution_type: v
                .contribution_type
                .unwrap_or_else(|| "inherited".to_string()),
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
    pub contribution_evidence: Option<JsonVal>,
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
// Device Policy Input Views
// ============================================================================

/// Input for upserting a device policy — camelCase API boundary type
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct UpsertPolicyInputView {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub subject_id: Option<String>,
    pub device_id: Option<String>,
    pub content_rules: ContentRulesInput,
    pub time_rules: TimeRulesInput,
    pub feature_rules: FeatureRulesInput,
    #[serde(default)]
    pub monitoring_rules: Option<MonitoringRulesInput>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentRulesInput {
    #[serde(default)]
    pub blocked_categories: Vec<String>,
    #[serde(default)]
    pub blocked_hashes: Vec<String>,
    pub age_rating_max: Option<String>,
    pub reach_level_max: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct TimeRulesInput {
    pub session_max_minutes: Option<i32>,
    pub daily_max_minutes: Option<i32>,
    #[serde(default)]
    pub time_windows: Vec<JsonVal>,
    pub cooldown_minutes: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct FeatureRulesInput {
    #[serde(default)]
    pub disabled_features: Vec<String>,
    #[serde(default)]
    pub disabled_routes: Vec<String>,
    #[serde(default)]
    pub require_approval: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct MonitoringRulesInput {
    #[serde(default)]
    pub log_sessions: bool,
    #[serde(default)]
    pub log_categories: bool,
    #[serde(default = "default_true")]
    pub log_policy_events: bool,
    #[serde(default = "default_30i32")]
    pub retention_days: i32,
    #[serde(default = "default_true")]
    pub subject_can_view: bool,
}

fn default_true() -> bool {
    true
}
fn default_30i32() -> i32 {
    30
}

impl UpsertPolicyInputView {
    /// Convert to DB input with author context
    pub fn to_db_input(
        self,
        author_id: &str,
        author_tier: &str,
    ) -> crate::db::device_policies::CreateDevicePolicyInput {
        let monitoring = self.monitoring_rules.unwrap_or(MonitoringRulesInput {
            log_sessions: false,
            log_categories: false,
            log_policy_events: true,
            retention_days: 30,
            subject_can_view: true,
        });
        crate::db::device_policies::CreateDevicePolicyInput {
            subject_id: self.subject_id.unwrap_or_default(),
            device_id: self.device_id,
            author_id: author_id.to_string(),
            author_tier: author_tier.to_string(),
            inherits_from: None,
            blocked_categories_json: serde_json::to_string(&self.content_rules.blocked_categories)
                .unwrap_or_else(|_| "[]".into()),
            blocked_hashes_json: serde_json::to_string(&self.content_rules.blocked_hashes)
                .unwrap_or_else(|_| "[]".into()),
            age_rating_max: self.content_rules.age_rating_max,
            reach_level_max: self.content_rules.reach_level_max,
            session_max_minutes: self.time_rules.session_max_minutes,
            daily_max_minutes: self.time_rules.daily_max_minutes,
            time_windows_json: serde_json::to_string(
                &self
                    .time_rules
                    .time_windows
                    .iter()
                    .map(|v| &v.0)
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_else(|_| "[]".into()),
            cooldown_minutes: self.time_rules.cooldown_minutes,
            disabled_features_json: serde_json::to_string(&self.feature_rules.disabled_features)
                .unwrap_or_else(|_| "[]".into()),
            disabled_routes_json: serde_json::to_string(&self.feature_rules.disabled_routes)
                .unwrap_or_else(|_| "[]".into()),
            require_approval_json: serde_json::to_string(&self.feature_rules.require_approval)
                .unwrap_or_else(|_| "[]".into()),
            log_sessions: monitoring.log_sessions,
            log_categories: monitoring.log_categories,
            log_policy_events: monitoring.log_policy_events,
            retention_days: monitoring.retention_days,
            subject_can_view: monitoring.subject_can_view,
        }
    }
}

// ============================================================================
// Content Mastery Input View
// ============================================================================

use crate::db::content_mastery::CreateMasteryInput;

/// Input for creating/updating content mastery - camelCase API boundary type
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateMasteryInputView {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub human_id: String,
    pub content_id: String,
    #[serde(default)]
    pub mastery_level: Option<String>,
    #[serde(default)]
    pub content_version_at_mastery: Option<String>,
}

impl From<CreateMasteryInputView> for CreateMasteryInput {
    fn from(v: CreateMasteryInputView) -> Self {
        Self {
            id: v.id,
            human_id: v.human_id,
            content_id: v.content_id,
            mastery_level: v.mastery_level.unwrap_or_else(|| "not_started".to_string()),
            content_version_at_mastery: v.content_version_at_mastery,
        }
    }
}

// ============================================================================
// Collective Views (Qahal - Governance Contexts)
// ============================================================================

use crate::db::models::{Collective, CollectiveParticipation};

/// Collective response view
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CollectiveView {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub governance_layer: String,
    pub constitutional_parent_id: Option<String>,
    pub reach: String,
    pub metadata: Option<JsonVal>,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub dissolved_at: Option<String>,
}

impl From<Collective> for CollectiveView {
    fn from(c: Collective) -> Self {
        Self {
            id: c.id,
            name: c.name,
            description: c.description,
            governance_layer: c.governance_layer,
            constitutional_parent_id: c.constitutional_parent_id,
            reach: c.reach,
            metadata: parse_json_opt(&c.metadata_json),
            created_by: c.created_by,
            created_at: c.created_at,
            updated_at: c.updated_at,
            dissolved_at: c.dissolved_at,
        }
    }
}

/// Create collective input view
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateCollectiveInputView {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub governance_layer: String,
    #[serde(default)]
    pub constitutional_parent_id: Option<String>,
    #[serde(default)]
    pub reach: Option<String>,
    #[serde(default)]
    pub metadata: Option<JsonVal>,
    #[serde(default)]
    pub created_by: Option<String>,
}

impl From<CreateCollectiveInputView> for crate::db::collectives::CreateCollectiveInput {
    fn from(v: CreateCollectiveInputView) -> Self {
        Self {
            id: v.id,
            name: v.name,
            description: v.description,
            governance_layer: v.governance_layer,
            constitutional_parent_id: v.constitutional_parent_id,
            reach: v.reach.unwrap_or_else(|| "community".to_string()),
            metadata_json: serialize_json_opt(&v.metadata),
            created_by: v.created_by,
        }
    }
}

/// Collective participation response view
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CollectiveParticipationView {
    pub id: String,
    pub collective_id: String,
    pub human_id: String,
    pub intimacy_level: String,
    pub role_context: Option<String>,
    pub governance_weight: f32,
    pub consent_state: String,
    pub metadata: Option<JsonVal>,
    pub joined_at: String,
    pub updated_at: String,
    pub departed_at: Option<String>,
}

impl From<CollectiveParticipation> for CollectiveParticipationView {
    fn from(p: CollectiveParticipation) -> Self {
        Self {
            id: p.id,
            collective_id: p.collective_id,
            human_id: p.human_id,
            intimacy_level: p.intimacy_level,
            role_context: p.role_context,
            governance_weight: p.governance_weight,
            consent_state: p.consent_state,
            metadata: parse_json_opt(&p.metadata_json),
            joined_at: p.joined_at,
            updated_at: p.updated_at,
            departed_at: p.departed_at,
        }
    }
}

// ============================================================================
// Account Package Views (Import/Export)
// ============================================================================

/// Content assignment within an account package
#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentAssignmentView {
    pub content_id: String,
    pub reach: String,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub steward_ratio: Option<f32>,
}

/// Relationship seed within an account package
#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RelationshipSeedView {
    pub target_id: String,
    pub relationship_type: String,
    pub intimacy_level: String,
    #[serde(default)]
    pub is_bidirectional: bool,
    #[serde(default)]
    pub reach: Option<String>,
}

/// Stewardship seed within an account package
#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StewardshipSeedView {
    pub content_category: String,
    pub allocation_ratio: f32,
    #[serde(default)]
    pub contribution_type: Option<String>,
}

/// Collective seed within an account package
#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CollectiveSeedView {
    pub collective_id: String,
    #[serde(default)]
    pub role_context: Option<String>,
    #[serde(default)]
    pub intimacy_level: Option<String>,
}

/// Organization context within account identity (display-only, from humans.json)
#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct OrganizationContextView {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub role: Option<String>,
}

/// Identity section of an account package
#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AccountIdentityView {
    pub human_id: String,
    pub display_name: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub profile_reach: Option<String>,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub affinities: Vec<String>,
    #[serde(default)]
    pub organizations: Vec<OrganizationContextView>,
}

/// Package manifest metadata
#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PackageManifestView {
    pub version: String,
    pub generated_at: String,
    #[serde(default)]
    pub source_story: Option<String>,
    #[serde(default)]
    pub content_hash: Option<String>,
}

/// Account package input — accepts a full account package for import
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AccountPackageInputView {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub identity: AccountIdentityView,
    #[serde(default)]
    pub content: Vec<ContentAssignmentView>,
    #[serde(default)]
    pub relationships: Vec<RelationshipSeedView>,
    #[serde(default)]
    pub stewardship: Vec<StewardshipSeedView>,
    #[serde(default)]
    pub collectives: Vec<CollectiveSeedView>,
    #[serde(default)]
    pub conductor_group: Option<u32>,
    #[serde(default)]
    pub manifest: Option<PackageManifestView>,
}

/// Result of an account package import
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AccountImportResultView {
    pub human_id: String,
    pub content_updated: usize,
    pub relationships_created: usize,
    pub stewardship_created: usize,
    pub collectives_joined: usize,
    pub errors: Vec<String>,
}

/// Exported account package (full response)
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AccountPackageView {
    pub identity: AccountIdentityView,
    pub content: Vec<ContentAssignmentView>,
    pub relationships: Vec<RelationshipSeedView>,
    pub stewardship: Vec<StewardshipSeedView>,
    pub collectives: Vec<CollectiveSeedView>,
    pub manifest: PackageManifestView,
}

// ============================================================================
// EPR Head Views
// ============================================================================

use crate::epr_codec::{
    EprHead, EprLamadContext, EprQahalContext, EprRelationship, EprShefaContext,
};

/// EPR Head input — accepts camelCase JSON from TypeScript clients.
/// Converts to `EprHead` for DAG-CBOR encoding.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EprHeadInputView {
    pub version: Option<u32>,
    pub id: String,
    pub content: String,
    pub lamad: EprLamadContextInputView,
    pub shefa: Option<EprShefaContextInputView>,
    pub qahal: Option<EprQahalContextInputView>,
    #[serde(default)]
    pub relationships: Vec<EprRelationshipInputView>,
    pub author: Option<String>,
    pub updated: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EprLamadContextInputView {
    pub title: String,
    pub content_type: String,
    pub description: Option<String>,
    pub content_format: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EprShefaContextInputView {
    #[serde(default)]
    pub stewards: Vec<String>,
    #[serde(default)]
    pub allocations: Vec<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EprQahalContextInputView {
    pub reach: Option<String>,
    pub layer: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EprRelationshipInputView {
    #[serde(rename = "type")]
    pub rel_type: String,
    pub target: String,
    pub target_cid: Option<String>,
}

impl From<EprHeadInputView> for EprHead {
    fn from(v: EprHeadInputView) -> Self {
        Self {
            version: v.version.unwrap_or(1),
            id: v.id,
            content: v.content,
            lamad: EprLamadContext {
                title: v.lamad.title,
                content_type: v.lamad.content_type,
                description: v.lamad.description,
                content_format: v.lamad.content_format,
                tags: v.lamad.tags,
            },
            shefa: v.shefa.map_or_else(
                || EprShefaContext {
                    stewards: vec![],
                    allocations: vec![],
                },
                |s| EprShefaContext {
                    stewards: s.stewards,
                    allocations: s.allocations,
                },
            ),
            qahal: v.qahal.map_or_else(
                || EprQahalContext {
                    reach: None,
                    layer: None,
                },
                |q| EprQahalContext {
                    reach: q.reach,
                    layer: q.layer,
                },
            ),
            relationships: v
                .relationships
                .into_iter()
                .map(|r| EprRelationship {
                    rel_type: r.rel_type,
                    target: r.target,
                    target_cid: r.target_cid,
                })
                .collect(),
            author: v.author,
            updated: v.updated,
        }
    }
}

/// EPR Head response — camelCase output for TypeScript clients.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EprHeadView {
    pub version: u32,
    pub id: String,
    pub content: String,
    pub lamad: EprLamadContext,
    pub shefa: EprShefaContext,
    pub qahal: EprQahalContext,
    pub relationships: Vec<EprRelationship>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    /// CID of the DAG-CBOR encoded head (set after encoding)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cid: Option<String>,
}

impl From<EprHead> for EprHeadView {
    fn from(h: EprHead) -> Self {
        Self {
            version: h.version,
            id: h.id,
            content: h.content,
            lamad: h.lamad,
            shefa: h.shefa,
            qahal: h.qahal,
            relationships: h.relationships,
            author: h.author,
            updated: h.updated,
            cid: None,
        }
    }
}

// ============================================================================
// Human Identity Views (imagodei pillar)
// ============================================================================

/// Output view for a human identity record — camelCase for TypeScript clients.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct HumanView {
    pub id: String,
    pub agent_pub_key: Option<String>,
    pub display_name: String,
    pub bio: Option<String>,
    /// Parsed affinities array (stored as JSON text in DB)
    pub affinities: Vec<String>,
    pub profile_reach: String,
    pub location: Option<String>,
    pub app_id: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Human> for HumanView {
    fn from(h: Human) -> Self {
        let affinities: Vec<String> = serde_json::from_str(&h.affinities).unwrap_or_default();
        Self {
            id: h.id,
            agent_pub_key: h.agent_pub_key,
            display_name: h.display_name,
            bio: h.bio,
            affinities,
            profile_reach: h.profile_reach,
            location: h.location,
            app_id: h.app_id,
            created_at: h.created_at,
            updated_at: h.updated_at,
        }
    }
}

/// Input for registering a new human — camelCase API boundary type.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateHumanInputView {
    /// Caller-supplied stable ID (e.g. UUID derived from agent key)
    pub id: String,
    #[serde(default)]
    pub agent_pub_key: Option<String>,
    pub display_name: String,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub affinities: Vec<String>,
    #[serde(default = "default_profile_reach")]
    pub profile_reach: String,
    #[serde(default)]
    pub location: Option<String>,
}

fn default_profile_reach() -> String {
    "community".to_string()
}

/// Input for updating a human's mutable profile fields — camelCase API boundary type.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct UpdateHumanInputView {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub affinities: Option<Vec<String>>,
    #[serde(default)]
    pub profile_reach: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
}

// ============================================================================
// Custodian Metrics Views
// ============================================================================

/// Health metrics for a custodian node
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CustodianHealthView {
    pub uptime_percent: f64,
    pub availability: bool,
    pub response_time_p50_ms: f64,
    pub response_time_p95_ms: f64,
    pub response_time_p99_ms: f64,
    pub error_rate: f64,
    pub sla_compliance: bool,
}

/// Storage metrics for a custodian node
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CustodianStorageMetricsView {
    pub total_capacity_bytes: i64,
    pub used_bytes: i64,
    pub free_bytes: i64,
    pub utilization_percent: f64,
    /// Parsed by-domain map (was JSON string in storage)
    pub by_domain: Option<JsonVal>,
    pub full_replica_bytes: i64,
    pub threshold_bytes: i64,
    pub erasure_coded_bytes: i64,
}

/// Bandwidth metrics for a custodian node
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CustodianBandwidthView {
    pub declared_mbps: f64,
    pub current_usage_mbps: f64,
    pub peak_usage_mbps: f64,
    pub average_usage_mbps: f64,
    pub utilization_percent: f64,
    pub inbound_mbps: f64,
    pub outbound_mbps: f64,
    /// Parsed by-domain map (was JSON string in storage)
    pub by_domain: Option<JsonVal>,
}

/// Computation metrics for a custodian node
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CustodianComputationView {
    pub cpu_cores: u32,
    pub cpu_usage_percent: f64,
    pub memory_gb: f64,
    pub memory_usage_percent: f64,
    pub zome_ops_per_second: f64,
    pub reconstruction_workload_percent: f64,
}

/// Reputation metrics for a custodian node
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CustodianReputationView {
    pub reliability_rating: f64,
    pub speed_rating: f64,
    pub reputation_score: f64,
    pub specialization_bonus: f64,
    pub commitment_fulfillment: f64,
}

/// Economic metrics for a custodian node
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CustodianEconomicView {
    pub steward_tier: u32,
    pub price_per_gb: f64,
    pub monthly_earnings: f64,
    pub lifetime_earnings: f64,
    pub active_commitments: u32,
    pub total_committed_bytes: i64,
}

/// Complete metrics snapshot for a single custodian node.
///
/// Assembled from the custodian_metrics table (reported by the node) and
/// used by the shefa dashboard and operator tooling.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CustodianMetricsView {
    pub custodian_id: String,
    pub tier: u32,
    pub health: CustodianHealthView,
    pub storage: CustodianStorageMetricsView,
    pub bandwidth: CustodianBandwidthView,
    pub computation: CustodianComputationView,
    pub reputation: CustodianReputationView,
    pub economic: CustodianEconomicView,
    /// Unix timestamp (milliseconds) when metrics were collected
    pub collected_at: i64,
    /// Unix timestamp (milliseconds) of last update
    pub last_updated_at: i64,
}

/// Alert for custodian operators.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CustodianAlertView {
    pub custodian_id: String,
    /// "warning" | "critical"
    pub severity: String,
    pub category: String,
    pub message: String,
    pub suggestion: Option<String>,
}

/// Recommendation for custodian operators.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CustodianRecommendationView {
    pub custodian_id: String,
    pub category: String,
    pub opportunity: String,
    pub potential_revenue: Option<f64>,
}

impl From<CustodianMetrics> for CustodianMetricsView {
    fn from(m: CustodianMetrics) -> Self {
        // Metric groups are stored as JSON blobs; parse or fall back to defaults.
        let health: CustodianHealthView = serde_json::from_str(&m.health_json)
            .unwrap_or_else(|_| CustodianHealthView {
                uptime_percent: 0.0,
                availability: false,
                response_time_p50_ms: 0.0,
                response_time_p95_ms: 0.0,
                response_time_p99_ms: 0.0,
                error_rate: 0.0,
                sla_compliance: false,
            });
        let storage: CustodianStorageMetricsView = serde_json::from_str(&m.storage_json)
            .unwrap_or_else(|_| CustodianStorageMetricsView {
                total_capacity_bytes: 0,
                used_bytes: 0,
                free_bytes: 0,
                utilization_percent: 0.0,
                by_domain: None,
                full_replica_bytes: 0,
                threshold_bytes: 0,
                erasure_coded_bytes: 0,
            });
        let bandwidth: CustodianBandwidthView = serde_json::from_str(&m.bandwidth_json)
            .unwrap_or_else(|_| CustodianBandwidthView {
                declared_mbps: 0.0,
                current_usage_mbps: 0.0,
                peak_usage_mbps: 0.0,
                average_usage_mbps: 0.0,
                utilization_percent: 0.0,
                inbound_mbps: 0.0,
                outbound_mbps: 0.0,
                by_domain: None,
            });
        let computation: CustodianComputationView = serde_json::from_str(&m.computation_json)
            .unwrap_or_else(|_| CustodianComputationView {
                cpu_cores: 0,
                cpu_usage_percent: 0.0,
                memory_gb: 0.0,
                memory_usage_percent: 0.0,
                zome_ops_per_second: 0.0,
                reconstruction_workload_percent: 0.0,
            });
        let reputation: CustodianReputationView = serde_json::from_str(&m.reputation_json)
            .unwrap_or_else(|_| CustodianReputationView {
                reliability_rating: 0.0,
                speed_rating: 0.0,
                reputation_score: 0.0,
                specialization_bonus: 0.0,
                commitment_fulfillment: 0.0,
            });
        let economic: CustodianEconomicView = serde_json::from_str(&m.economic_json)
            .unwrap_or_else(|_| CustodianEconomicView {
                steward_tier: 0,
                price_per_gb: 0.0,
                monthly_earnings: 0.0,
                lifetime_earnings: 0.0,
                active_commitments: 0,
                total_committed_bytes: 0,
            });
        Self {
            custodian_id: m.custodian_id,
            tier: m.tier as u32,
            health,
            storage,
            bandwidth,
            computation,
            reputation,
            economic,
            collected_at: m.collected_at,
            last_updated_at: m.last_updated_at,
        }
    }
}

/// Input view for a node reporting its own metrics snapshot.
///
/// The sub-metric groups are accepted as raw JSON values and stored verbatim
/// (serialised back to String for the `UpsertCustodianMetrics` insertable).
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ReportCustodianMetricsInputView {
    pub custodian_id: String,
    pub tier: u32,
    pub health: CustodianHealthView,
    pub storage: CustodianStorageMetricsView,
    pub bandwidth: CustodianBandwidthView,
    pub computation: CustodianComputationView,
    pub reputation: CustodianReputationView,
    pub economic: CustodianEconomicView,
    /// Unix timestamp (ms) when metrics were collected — defaults to now if absent
    pub collected_at: Option<i64>,
}

impl ReportCustodianMetricsInputView {
    /// Convert to the insertable DB type, stamping `app_id` and `last_updated_at`.
    pub fn into_upsert(
        self,
        app_id: impl Into<String>,
        now_ms: i64,
    ) -> crate::db::models::UpsertCustodianMetrics {
        crate::db::models::UpsertCustodianMetrics {
            custodian_id: self.custodian_id,
            app_id: app_id.into(),
            tier: self.tier as i32,
            health_json: serde_json::to_string(&self.health).unwrap_or_default(),
            storage_json: serde_json::to_string(&self.storage).unwrap_or_default(),
            bandwidth_json: serde_json::to_string(&self.bandwidth).unwrap_or_default(),
            computation_json: serde_json::to_string(&self.computation).unwrap_or_default(),
            reputation_json: serde_json::to_string(&self.reputation).unwrap_or_default(),
            economic_json: serde_json::to_string(&self.economic).unwrap_or_default(),
            collected_at: self.collected_at.unwrap_or(now_ms),
            last_updated_at: now_ms,
        }
    }
}

// ============================================================================
// Data Protection Views
//
// Read-only aggregation views — assembled from custodian commitment data
// and DHT queries. No dedicated DB tables required.
// ============================================================================

/// A node protecting the operator's data.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CustodianNodeView {
    pub id: String,
    pub name: String,
    /// "family" | "friend" | "community" | "professional" | "institution"
    pub custodian_type: String,
    pub location_region: Option<String>,
    pub location_country: Option<String>,
    // dataStored
    pub data_stored_total_gb: f64,
    pub data_stored_shard_count: u32,
    pub data_stored_redundancy_level: u32,
    // health
    pub health_up_percent: f64,
    pub health_last_heartbeat: String,
    pub health_response_time_ms: f64,
    // commitment
    pub commitment_id: String,
    /// "active" | "pending" | "breached" | "expired"
    pub commitment_status: String,
    pub commitment_start_date: String,
    pub commitment_expiry_date: String,
    /// "auto-renew" | "manual" | "expired"
    pub commitment_renewal_status: String,
    pub trust_level: f64,
    pub relationship: String,
}

/// Geographic distribution of custodians in a region.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RegionalPresenceView {
    pub region: String,
    pub custodian_count: u32,
    pub data_shards: u32,
    pub redundancy: u32,
    pub risk_factors: Vec<String>,
}

/// Trust relationship in the trust graph.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct TrustRelationshipView {
    pub from: String,
    pub to: String,
    /// "family-member" | "friend" | "community-peer" | "professional" | "institution"
    pub relationship_type: String,
    pub trust_score: f64,
    pub depth: u32,
    /// "weak" | "moderate" | "strong"
    pub strength: String,
}

/// Complete family-community protection status.
///
/// Assembled from CustodianCommitment entries and DHT health data.
/// No dedicated DB table — assembled by the data protection service.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct FamilyCommunityProtectionStatusView {
    // Redundancy model
    /// "full_replica" | "threshold_split" | "erasure_coded"
    pub redundancy_strategy: String,
    pub redundancy_factor: f64,
    pub recovery_threshold: u32,
    // Custodian network
    pub custodians: Vec<CustodianNodeView>,
    pub total_custodians: u32,
    // Geographic distribution
    pub geographic_regions: Vec<RegionalPresenceView>,
    /// "centralized" | "distributed" | "geo-redundant"
    pub geographic_risk_profile: String,
    // Trust graph
    pub trust_graph: Vec<TrustRelationshipView>,
    // Overall protection status
    /// "vulnerable" | "protected" | "highly-protected"
    pub protection_level: String,
    pub estimated_recovery_time: String,
    pub last_verification: String,
    /// "verified" | "pending" | "failed"
    pub verification_status: String,
}

// ============================================================================
// Shefa Dashboard Views
//
// Read-only aggregation views assembled from multiple sources by the
// compute handler. No dedicated DB tables.
// ============================================================================

/// Time-series data point for metric history charts.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct MetricHistoryView {
    pub timestamp: String,
    pub value: f64,
}

/// Node availability tracking.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct UpTimeMetricsView {
    pub up_percent: f64,
    pub downtime_hours_24: f64,
    pub downtime_hours_7d: f64,
    pub downtime_hours_30d: f64,
    pub last_failure: Option<String>,
    pub consecutive_uptime: String,
    /// "excellent" | "good" | "fair" | "poor"
    pub reliability: String,
}

/// Real-time node performance and capacity.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ComputeMetricsView {
    // CPU
    pub cpu_total_cores: u32,
    pub cpu_available: u32,
    pub cpu_usage_percent: f64,
    pub cpu_usage_history: Vec<MetricHistoryView>,
    pub cpu_temperature: Option<f64>,
    // Memory
    pub memory_total_gb: f64,
    pub memory_used_gb: f64,
    pub memory_available_gb: f64,
    pub memory_usage_percent: f64,
    pub memory_usage_history: Vec<MetricHistoryView>,
    // Storage
    pub storage_total_gb: f64,
    pub storage_used_gb: f64,
    pub storage_available_gb: f64,
    pub storage_usage_percent: f64,
    pub storage_usage_history: Vec<MetricHistoryView>,
    pub storage_breakdown_holochain_gb: f64,
    pub storage_breakdown_cache_gb: f64,
    pub storage_breakdown_custodian_data_gb: f64,
    pub storage_breakdown_user_applications_gb: f64,
    // Network
    pub network_upstream_mbps: f64,
    pub network_downstream_mbps: f64,
    pub network_used_upstream_mbps: f64,
    pub network_used_downstream_mbps: f64,
    pub network_latency_p50: f64,
    pub network_latency_p95: f64,
    pub network_latency_p99: f64,
    pub network_connections_total: u32,
    pub network_connections_holochain: u32,
    pub network_connections_cache: u32,
    pub network_connections_custodian: u32,
    // Load
    pub load_average_one_minute: f64,
    pub load_average_five_minutes: f64,
    pub load_average_fifteen_minutes: f64,
    // Power (optional)
    pub power_consumption_watts: Option<f64>,
    pub power_thermal_output: Option<f64>,
}

/// Resource allocation percentages for a single governance level.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct GovernanceLevelAllocationView {
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub storage_percent: f64,
    pub bandwidth_percent: f64,
}

/// A specific allocation block for a purpose (e.g., "10% CPU for Lamad family learning").
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AllocationBlockView {
    pub id: String,
    pub label: String,
    /// "individual" | "household" | "community" | "network"
    pub governance_level: String,
    pub priority: u32,
    pub cpu_cores: f64,
    pub cpu_percent: f64,
    pub memory_gb: f64,
    pub memory_percent: f64,
    pub storage_gb: f64,
    pub storage_percent: f64,
    pub bandwidth_mbps: f64,
    pub bandwidth_percent: f64,
    pub utilized_cpu_percent: f64,
    pub utilized_memory_percent: f64,
    pub utilized_storage_percent: f64,
    pub utilized_bandwidth_percent: f64,
    pub commitment_id: Option<String>,
    pub related_agents: Vec<String>,
}

/// How much compute is allocated to family-community.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AllocationSnapshotView {
    pub by_governance_individual: GovernanceLevelAllocationView,
    pub by_governance_household: GovernanceLevelAllocationView,
    pub by_governance_community: GovernanceLevelAllocationView,
    pub by_governance_network: GovernanceLevelAllocationView,
    pub total_allocated_cpu_percent: f64,
    pub total_allocated_memory_percent: f64,
    pub total_allocated_storage_percent: f64,
    pub total_allocated_bandwidth_percent: f64,
    pub allocation_blocks: Vec<AllocationBlockView>,
}

/// Infrastructure-token balance and earnings.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct InfrastructureTokenBalanceView {
    pub balance_tokens: f64,
    pub balance_estimated_value: f64,
    pub balance_currency: String,
    pub earning_rate_tokens_per_hour: f64,
    pub earning_rate_cpu_allocation: f64,
    pub earning_rate_storage_allocation: f64,
    pub earning_rate_bandwidth_allocation: f64,
    pub earning_rate_estimated_monthly: f64,
    pub decay_demurrage_rate: f64,
    pub decay_last_calculated: String,
    pub decay_projected_next_month_tokens: f64,
    pub decay_projected_next_month_value_usd: f64,
    pub token_history_last_24h: f64,
    pub token_history_last_7d: f64,
    pub token_history_last_30d: f64,
    pub token_history_all_time: f64,
    pub transactions: Vec<TokenTransactionView>,
    pub exchange_rates: Vec<ExchangeRateView>,
}

/// A token earning or spending event.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct TokenTransactionView {
    pub id: String,
    pub timestamp: String,
    /// "earned" | "transferred" | "exchanged" | "decayed" | "claimed"
    pub transaction_type: String,
    pub amount: f64,
    pub related_agent: Option<String>,
    pub description: String,
    pub economic_event_id: Option<String>,
}

/// Cross-swimlane token exchange rate.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ExchangeRateView {
    pub from: String,
    pub to: String,
    pub rate: f64,
    /// "market" | "consensus" | "algorithm"
    pub source: String,
    pub last_updated: String,
}

/// A single compute-related hREA economic event.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RecentEconomicEventView {
    pub id: String,
    pub timestamp: String,
    pub event_type: String,
    pub provider: Option<String>,
    pub receiver: Option<String>,
    pub quantity_has_unit: String,
    pub quantity_has_numerical_value: f64,
    pub tokens_minted: Option<f64>,
    pub note: String,
}

/// Dignity floor enforcement status.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct DignityFloorView {
    pub compute_min_cores: f64,
    pub compute_min_memory_gb: f64,
    pub compute_min_storage_gb: f64,
    pub compute_min_bandwidth_mbps: f64,
    /// "met" | "warning" | "breached"
    pub status: String,
    pub percent_of_floor: f64,
    /// "voluntary" | "progressive" | "hard"
    pub enforcement: String,
}

/// Ceiling limit enforcement status.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CeilingLimitView {
    pub compute_max_cores: f64,
    pub compute_max_memory_gb: f64,
    pub compute_max_storage_gb: f64,
    pub compute_max_bandwidth_mbps: f64,
    pub token_accumulation_ceiling: f64,
    pub current_accumulation: f64,
    pub percent_of_ceiling: f64,
    /// "safe" | "warning" | "breached"
    pub status: String,
    /// "voluntary" | "progressive" | "hard"
    pub enforcement: String,
}

/// A constitutional limit violation alert.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ConstitutionalAlertView {
    pub id: String,
    /// "info" | "warning" | "critical"
    pub severity: String,
    /// "floor-breach" | "ceiling-breach" | "redistribution-required"
    pub alert_type: String,
    pub message: String,
    pub affected_resource: String,
    pub current_value: f64,
    pub threshold: f64,
    pub recommended_action: String,
    pub timestamp: String,
}

/// Dignity floor and ceiling enforcement status.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ConstitutionalLimitsStatusView {
    pub dignity_floor: DignityFloorView,
    pub ceiling_limit: CeilingLimitView,
    pub safe_zone_cpu: f64,
    pub safe_zone_memory: f64,
    pub safe_zone_storage: f64,
    pub safe_zone_bandwidth: f64,
    pub safe_zone_tokens: f64,
    pub alerts: Vec<ConstitutionalAlertView>,
}

/// A node in the user's cluster.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct OwnedNodeView {
    pub node_id: String,
    pub display_name: String,
    /// "holoport" | "holoport-plus" | "holoport-nano" | "self-hosted" | "cloud"
    pub node_type: String,
    /// "online" | "offline" | "degraded" | "maintenance" | "provisioning" | "unknown"
    pub status: String,
    pub last_heartbeat: String,
    pub consecutive_uptime: String,
    pub location_label: Option<String>,
    pub location_region: Option<String>,
    pub location_country: Option<String>,
    pub roles: Vec<NodeRoleView>,
    pub resources_cpu_percent: f64,
    pub resources_memory_percent: f64,
    pub resources_storage_used_gb: f64,
    pub resources_storage_total_gb: f64,
    pub resources_bandwidth_mbps: f64,
    pub custodian_activity_items_custodied: u32,
    pub custodian_activity_items_being_custodied: u32,
    pub custodian_activity_total_custodied_gb: f64,
    pub is_primary: bool,
}

/// A role a node is playing in the cluster.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct NodeRoleView {
    /// "storage" | "compute" | "gateway" | "custodian" | "archive"
    pub role: String,
    pub description: String,
    pub utilization_percent: f64,
}

/// Node topology overview for an operator.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct NodeTopologyStateView {
    pub nodes: Vec<OwnedNodeView>,
    pub total_nodes: u32,
    pub online_nodes: u32,
    pub offline_nodes: u32,
    pub degraded_nodes: u32,
    pub primary_node_id: Option<String>,
    pub primary_node_status: Option<String>,
    pub primary_node_is_online: Option<bool>,
    /// "healthy" | "degraded" | "critical" | "offline"
    pub cluster_health: String,
    pub alerts: Vec<OfflineNodeAlertView>,
    pub last_updated: String,
}

/// Alert when a node goes offline.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct OfflineNodeAlertView {
    pub id: String,
    /// "info" | "warning" | "critical"
    pub severity: String,
    pub node_id: String,
    pub node_name: String,
    pub is_primary_node: bool,
    /// "went-offline" | "degraded" | "heartbeat-missed" | "recovery-needed"
    pub event_type: String,
    pub message: String,
    pub detected_at: String,
    pub last_seen_online: String,
    pub offline_duration: String,
    pub impact_affected_content: u32,
    pub impact_affected_custodians: u32,
    pub impact_compute_gap_percent: f64,
    pub impact_storage_gap_percent: f64,
    pub recommended_actions: Vec<String>,
    pub help_flow_url: Option<String>,
    pub dismissed_at: Option<String>,
}

/// A single custodian relationship (helping or being helped).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CustodianRelationshipView {
    pub agent_id: String,
    pub display_name: String,
    /// "family" | "friend" | "community" | "professional"
    pub relationship_type: String,
    pub trust_score: f64,
    /// "i-help-them" | "they-help-me"
    pub direction: String,
    pub content_summary_total_items: u32,
    pub content_summary_total_gb: f64,
    /// Parsed content types breakdown (was JSON string in storage)
    pub content_summary_content_types: Vec<JsonVal>,
    /// "active" | "pending" | "at-risk" | "expired"
    pub status: String,
    pub last_activity: String,
    pub reliability: f64,
}

/// Bidirectional custodian view (who I help vs who helps me).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct BidirectionalCustodianView {
    pub helping: Vec<CustodianRelationshipView>,
    pub helping_count: u32,
    pub helping_total_gb: f64,
    pub being_helped_by: Vec<CustodianRelationshipView>,
    pub being_helped_by_count: u32,
    pub being_helped_by_total_gb: f64,
    pub mutual_aid_balance_ratio: f64,
    /// "giving-more" | "balanced" | "receiving-more"
    pub mutual_aid_balance_status: String,
    pub mutual_aid_balance_message: String,
    /// "strong" | "moderate" | "weak"
    pub community_strength: String,
}

/// Storage breakdown by content type.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentTypeStorageView {
    /// "video" | "audio" | "image" | "document" | "application" | "learning" | "other"
    pub content_type: String,
    pub display_label: String,
    pub icon: Option<String>,
    pub item_count: u32,
    pub size_gb: f64,
    pub percent_of_total: f64,
    pub fully_replicated: u32,
    pub under_replicated: u32,
    pub average_replicas: f64,
}

/// Storage breakdown by reach level (0–7).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ReachLevelStorageView {
    pub reach_level: u32,
    pub reach_label: String,
    pub item_count: u32,
    pub size_gb: f64,
    pub target_replicas: u32,
    pub current_replicas: f64,
    /// "met" | "under" | "over"
    pub replication_status: String,
}

/// Storage breakdown for a single node.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct NodeStorageBreakdownView {
    pub node_id: String,
    pub node_name: String,
    pub node_status: String,
    pub total_gb: f64,
    pub used_gb: f64,
    pub available_gb: f64,
    pub my_content_gb: f64,
    pub custodied_content_gb: f64,
    pub cache_content_gb: f64,
    /// Parsed content-type list (was JSON string in storage)
    pub content_types: Vec<JsonVal>,
}

/// What types of content are stored where.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StorageContentDistributionView {
    pub by_content_type: Vec<ContentTypeStorageView>,
    pub by_reach_level: Vec<ReachLevelStorageView>,
    pub by_node: Vec<NodeStorageBreakdownView>,
    pub total_items: u32,
    pub total_size_gb: f64,
    pub total_replica_count: u32,
}

/// A specific compute deficiency.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ComputeGapView {
    /// "cpu" | "memory" | "storage" | "bandwidth" | "redundancy"
    pub resource: String,
    pub current_value: f64,
    pub target_value: f64,
    pub gap_percent: f64,
    /// "minor" | "moderate" | "critical"
    pub severity: String,
    pub description: String,
    pub impact: String,
}

/// Suggested node to address compute gaps.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct NodeRecommendationView {
    /// "holoport" | "holoport-plus" | "holoport-nano" | "self-hosted" | "cloud"
    pub node_type: String,
    pub display_name: String,
    pub description: String,
    pub addresses_gaps: Vec<String>,
    pub improvement_percent: f64,
    pub estimated_cost_value: Option<f64>,
    pub estimated_cost_currency: Option<String>,
    pub estimated_cost_period: Option<String>,
    pub order_url: Option<String>,
    /// "recommended" | "optional" | "future"
    pub priority: String,
}

/// Compute needs assessment — gaps and recommendations for the help-flow.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ComputeNeedsAssessmentView {
    pub current_capacity_cpu_cores: u32,
    pub current_capacity_memory_gb: f64,
    pub current_capacity_storage_gb: f64,
    pub current_capacity_bandwidth_mbps: f64,
    pub gaps: Vec<ComputeGapView>,
    pub has_gaps: bool,
    /// "none" | "minor" | "moderate" | "critical"
    pub overall_gap_severity: String,
    pub recommendations: Vec<NodeRecommendationView>,
    pub help_flow_url: String,
    pub help_flow_cta: String,
}

/// Complete state for the operator's Shefa compute dashboard.
///
/// Assembled server-side from compute metrics, allocations, protection
/// status, token economics, and constitutional limits. Angular is a thin
/// display client — no aggregation happens in TypeScript.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct SheafaDashboardStateView {
    // Identity
    pub operator_id: String,
    pub operator_name: String,
    pub stewarded_resource_id: String,
    pub node_id: String,
    pub node_location_region: Option<String>,
    pub node_location_country: Option<String>,
    pub node_location_latitude: Option<f64>,
    pub node_location_longitude: Option<f64>,
    // Status
    /// "online" | "offline" | "degraded" | "maintenance"
    pub status: String,
    pub last_heartbeat: String,
    pub uptime: UpTimeMetricsView,
    // Compute
    pub compute_metrics: ComputeMetricsView,
    pub allocations: AllocationSnapshotView,
    // Protection
    pub family_community_protection: FamilyCommunityProtectionStatusView,
    // Economics
    pub infrastructure_tokens: InfrastructureTokenBalanceView,
    pub economic_events: Vec<RecentEconomicEventView>,
    // Constitutional
    pub constitutional_limits: ConstitutionalLimitsStatusView,
    // Timestamps
    pub last_updated: String,
    pub update_frequency_ms: u32,
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
        let content: CreateContentInputView =
            serde_json::from_str(r#"{"id":"c","title":"T","schemaVersion":3}"#).unwrap();
        assert_eq!(content.schema_version, 3);

        let rel: CreateRelationshipInputView = serde_json::from_str(
            r#"{"sourceId":"a","targetId":"b","relationshipType":"relates","schemaVersion":2}"#,
        )
        .unwrap();
        assert_eq!(rel.schema_version, 2);

        let event: CreateEconomicEventInputView = serde_json::from_str(
            r#"{"action":"use","provider":"p","receiver":"r","schemaVersion":5}"#,
        )
        .unwrap();
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
        let content: CreateContentInputView =
            serde_json::from_value(serde_json::json!({"id":"x","title":"x"})).unwrap();
        let step: CreateStepInputView =
            serde_json::from_value(serde_json::json!({"id":"x","pathId":"p","title":"x"})).unwrap();
        let chapter: CreateChapterInputView =
            serde_json::from_value(serde_json::json!({"id":"x","title":"x"})).unwrap();
        let path: CreatePathInputView =
            serde_json::from_value(serde_json::json!({"id":"x","title":"x"})).unwrap();
        let rel: CreateRelationshipInputView = serde_json::from_value(
            serde_json::json!({"sourceId":"a","targetId":"b","relationshipType":"r"}),
        )
        .unwrap();
        let human_rel: CreateHumanRelationshipInputView = serde_json::from_value(
            serde_json::json!({"partyAId":"a","partyBId":"b","relationshipType":"r","initiatedBy":"a"})
        ).unwrap();
        let presence: CreateContributorPresenceInputView = serde_json::from_value(
            serde_json::json!({"displayName":"x","establishingContentIds":[]}),
        )
        .unwrap();
        let claim: InitiateClaimInputView = serde_json::from_value(
            serde_json::json!({"claimingAgentId":"a","verificationMethod":"m"}),
        )
        .unwrap();
        let event: CreateEconomicEventInputView = serde_json::from_value(
            serde_json::json!({"action":"use","provider":"p","receiver":"r"}),
        )
        .unwrap();
        let alloc: CreateAllocationInputView =
            serde_json::from_value(serde_json::json!({"contentId":"c","stewardPresenceId":"s"}))
                .unwrap();
        let update_alloc: UpdateAllocationInputView =
            serde_json::from_value(serde_json::json!({})).unwrap();
        let mastery: CreateMasteryInputView =
            serde_json::from_value(serde_json::json!({"humanId":"h","contentId":"c"})).unwrap();
        let account_pkg: AccountPackageInputView = serde_json::from_value(
            serde_json::json!({"identity":{"humanId":"h","displayName":"Test"}}),
        )
        .unwrap();
        let upsert_policy: UpsertPolicyInputView = serde_json::from_value(
            serde_json::json!({"contentRules":{"blockedCategories":[],"blockedHashes":[]},"timeRules":{},"featureRules":{}}),
        )
        .unwrap();

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
        assert_eq!(mastery.schema_version, 1);
        assert_eq!(account_pkg.schema_version, 1);
        assert_eq!(upsert_policy.schema_version, 1);
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
