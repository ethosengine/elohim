//! lamad view types — migrated from elohim-storage/src/views.rs (VIEWS.T2).

use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use serde_json::Value;
use ts_rs::TS;
use crate::shared::*;
#[allow(unused_imports)]
use crate::infrastructure::*;
#[allow(unused_imports)]
use crate::shefa::*;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentView {
    pub id: String,
    pub h_app_id: String,
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
    pub dht_anchor_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentWithTagsView {
    #[serde(flatten)]
    pub content: ContentView,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentMasteryView {
    pub id: String,
    pub h_app_id: String,
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
    pub dht_anchor_hash: Option<String>,
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

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CommentView {
    pub id: String,
    pub content_id: String,
    pub human_id: String,
    pub body: String,
    pub reach: String,
    pub governance_state: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateCommentInputView {
    pub content_id: String,
    pub body: String,
}

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

/// Input for partially updating a content item — PATCH /db/content/{id}
///
/// All fields are optional — only provided fields are applied.
/// `metadata` is shallow-merged into the existing metadata object (key-by-key overwrite).
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct UpdateContentInputView {
    #[serde(default)]
    pub title: Option<String>,
    /// Pass `null` explicitly to clear the description field.
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub content_body: Option<String>,
    #[serde(default)]
    pub content_format: Option<String>,
    /// Shallow-merged into existing metadata: only keys present in this object are updated.
    #[serde(default)]
    pub metadata: Option<JsonVal>,
    /// If provided, replaces all existing tags.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub reach: Option<String>,
}

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

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentAttestationView {
    pub id: String,
    pub content_id: String,
    pub attestor_presence_id: String,
    pub scope: String,
    pub attestation_type: String,
    pub evidence: Option<JsonVal>,
    pub grantor: Option<JsonVal>,
    pub is_revoked: bool,
    pub revocation: Option<JsonVal>,
    pub created_at: String,
    pub updated_at: String,
    pub dht_anchor_hash: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateAttestationInputView {
    pub content_id: String,
    pub attestor_presence_id: String,
    pub scope: String,
    pub attestation_type: String,
    pub evidence: Option<JsonVal>,
    pub grantor: Option<JsonVal>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RevokeAttestationInputView {
    pub revocation: Option<JsonVal>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContributorDashboardView {
    pub presence_id: String,
    pub total_contributions: i32,
    pub total_recognitions: i32,
    pub impact_score: f32,
    pub last_contribution_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContributorImpactView {
    pub presence_id: String,
    pub total_events: i64,
    pub unique_content_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContributorRecognitionView {
    pub presence_id: String,
    pub events: Vec<EconomicEventView>,
}

/// Input for recording a curation activity
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CurationEventInputView {
    pub steward_id: String,
    pub content_id: String,
    pub activity_type: String,
}

/// Wire view for a notarized attestation Content entry.
///
/// Source of truth: Holochain DHT (elohim DNA, Content entry,
/// `content_type LIKE 'attestation:%'`, Category A per p2p-design-gate).
/// This record is a read-optimised projection populated by the
/// `AttestationProjector` post-commit signal. DHT is authoritative.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AttestationView {
    /// CID of this attestation (content-derived identity).
    pub id: String,
    /// ActionHash (hex) of the DHT entry — provenance anchor.
    pub dht_anchor_hash: String,
    /// Discriminator matching `attestation:<subtype>`.
    pub attestation_kind: String,
    /// CID of the entity being attested.
    pub subject_cid: String,
    /// Kind of the subject: "agent" | "content" | "device" | "hub" | "computation" | "governance-action".
    pub subject_kind: String,
    /// CID of the issuing agent.
    pub issuer_cid: String,
    /// CID of the parent governance-action, if this attestation is a vote.
    pub parent_governance_action_cid: Option<String>,
    /// Vote value: "approve" | "reject" | "abstain" (null for non-votes).
    pub vote_value: Option<String>,
    /// Optional vote weight as a decimal string (null when unweighted).
    pub vote_weight: Option<String>,
    /// Proof class: "witness" | "self-attest" | "audit-signature" | "computational".
    pub proof_class: String,
    /// Serialised proof evidence JSON (opaque string — parse only when needed).
    pub proof_evidence_json: String,
    /// Serialised full evidence JSON (opaque string).
    pub evidence_json: String,
    /// ISO 8601 expiry timestamp, if any.
    pub expires_at: Option<String>,
    /// CID of the attestation this one supersedes, if any.
    pub supersedes_cid: Option<String>,
    /// Reason this attestation was revoked, if revoked.
    pub revocation_reason: Option<String>,
    /// ISO 8601 revocation timestamp, if revoked.
    pub revoked_at: Option<String>,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
    /// Manifest reference (e.g. "mishpat", "lamad").
    pub manifest_ref: String,
    /// Human-readable title from the Content entry.
    pub title: String,
    /// Optional description from the Content entry.
    pub description: Option<String>,
}

