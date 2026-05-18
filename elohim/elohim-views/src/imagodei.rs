//! imagodei view types — migrated from elohim-storage/src/views.rs (VIEWS.T2).

use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use serde_json::Value;
use ts_rs::TS;
use crate::shared::*;
#[allow(unused_imports)]
use crate::infrastructure::*;
#[allow(unused_imports)]
use crate::lamad::*;
#[allow(unused_imports)]
use crate::shefa::*;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RelationshipView {
    pub id: String,
    pub h_app_id: String,
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
    pub dht_anchor_hash: Option<String>,
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

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct HumanRelationshipView {
    pub id: String,
    pub h_app_id: String,
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
    /// DHT provenance: ActionHash of the HumanRelationship entry in imagodei DNA. None for pre-coherence rows.
    pub dht_anchor_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContributorPresenceView {
    pub id: String,
    pub h_app_id: String,
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
    /// DHT provenance: ActionHash of the ContributorPresence entry in imagodei DNA. None for pre-coherence rows.
    pub dht_anchor_hash: Option<String>,
}

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
    pub relationships_skipped: usize,
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
    pub profile_photo_url: Option<String>,
    pub h_app_id: String,
    pub created_at: String,
    pub updated_at: String,
    /// DHT provenance: ActionHash of the Human entry in imagodei DNA. None for pre-coherence rows.
    pub dht_anchor_hash: Option<String>,
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
    #[serde(default)]
    pub profile_photo_url: Option<String>,
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
    #[serde(default)]
    pub profile_photo_url: Option<String>,
}

/// Imagodei observation output view.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ImagodeiObservationView {
    pub id: String,
    pub human_id: String,
    pub observed_at: String,
    pub observation_type: String,
    pub content: String,
    pub structured_signals: Option<JsonVal>,
    pub trust_delta: f64,
    pub visibility_layer: String,
    pub relevance_decay: f64,
    pub created_at: String,
    pub dht_anchor_hash: Option<String>,
}

/// Session intent input view.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct SetSessionIntentInputView {
    pub intent: String,
}

/// Source of truth: DHT (imagodei RecoveryRequest entry).
/// RecoveryRequestView — projection of the modernized RecoveryRequest DHT entry.
/// Replaces the M1 RecoveryQuorumRequestView. See recovery phase 2 revised spec §5.3.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RecoveryRequestView {
    pub dht_anchor_hash: String,
    pub human_agent_pubkey: String,
    pub new_agent_pubkey: String,
    pub hosting_doorway_pubkey: String,
    /// Discriminator for proposed_authority — "intimateQuorum" | "communityConsensus" | "governanceAct" | "networkWitness" | "cryptographicQuorum".
    pub proposed_authority_kind: String,
    /// JSON-encoded variant-specific fields (grant_hash, purpose, stewardship_hash, etc.).
    /// Empty string `""` for variants with no extra fields.
    pub proposed_authority_json: String,
    pub request_nonce: Vec<u8>,
    /// Coordinator-populated resolution of human_agent_pubkey to legacy String human_id.
    /// None at request-commit time is accepted (back-compat); required for IntimateQuorum
    /// KeyRotation and freeze-floor checks per M2 validator semantics.
    pub human_id: Option<String>,
    /// Threshold for IntimateQuorum — distinct witness-author count must be >= this value.
    /// Coordinator computes ceil(emergency_contact_count / 2) + 1, floored at 2.
    pub required_witness_count: u32,
    pub created_at: String,
}

/// Source of truth: DHT (imagodei KeyRotation entry).
/// KeyRotationView — projection of the modernized KeyRotation DHT entry with RecoveryAuthority enum.
/// Replaces the M1 KeyRotationView (which had seed_commitment_hash + quorum_signature fields).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct KeyRotationView {
    pub dht_anchor_hash: String,
    pub human_agent_pubkey: String,
    pub new_agent_pubkey: String,
    pub superseded_agent_pubkey: String,
    pub recovery_request_hash: String,
    /// Discriminator for authority — "intimateQuorum" | "communityConsensus" | "governanceAct" | "networkWitness" | "cryptographicQuorum".
    pub authority_kind: String,
    /// JSON-encoded variant-specific fields (witness_hashes, challenge_hash, etc.).
    pub authority_json: String,
    pub rotated_at: String,
}

/// Source of truth: DHT (imagodei HumanityWitness entry linked via
/// RecoveryRequestToHumanityWitness from a RecoveryRequest). Projection
/// populated from `RecoveryV2Signal::IntimateWitnessSubmitted`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RecoveryWitnessView {
    pub dht_anchor_hash: String,
    pub recovery_request_hash: String,
    pub witness_agent_id: String,
    pub human_id: String,
    pub note: Option<String>,
    pub submitted_at: String,
}

/// Projection of an elohim-DNA Content key-revocation entry
/// (content_type = 'governance-action:key-revocation').
/// Source of truth: DHT. Rebuildable via signal replay on the
/// ElohimContentSignal dispatcher.
///
/// Field naming follows the Content-routed producer contract:
/// `subject_human_id` / `initiated_by_cid` (replacing the legacy
/// `human_id` / `initiated_by`). EPR W2D adds `derived_compromise_at`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct KeyRevocationView {
    /// Holochain ActionHash of the source Content entry (hex-encoded).
    pub dht_anchor_hash: String,
    pub id: String,
    pub subject_human_id: String,
    pub revoked_key: String,
    pub reason: String,
    pub trigger_type: String,
    pub initiated_by_cid: String,
    pub required_votes: u32,
    pub current_votes: u32,
    pub threshold_reached: bool,
    pub effective_at: Option<String>,
    pub derived_compromise_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Projection of an imagodei RevocationVote DHT entry.
/// Source of truth: DHT. Rebuildable via signal replay on
/// RecoveryV2Signal::RevocationVoteSubmitted.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RevocationVoteView {
    pub dht_anchor_hash: String,
    pub id: String,
    pub revocation_dht_anchor_hash: String,
    pub revocation_id: String,
    pub steward_id: String,
    pub approved: bool,
    pub attestation: String,
    pub voted_at: String,
}

/// Projection of an imagodei PortalHost DHT entry.
/// Source of truth: DHT (imagodei PortalHost entry).
/// This table is a read-optimized projection rebuildable via signal replay.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PortalHostView {
    pub human_id: String,
    pub host_url: String,
    pub label: Option<String>,
    pub added_at: String,
    /// Last time this host was successfully reached.
    /// Operational enrichment — NOT stored in the notarized DHT entry.
    /// Populated by the elohim-storage reconciliation controller from
    /// periodic health probes; may be None if never probed or unreachable.
    pub last_reachable_at: Option<String>,
    pub reach: String,
    pub dht_anchor_hash: String,
}

/// Projection of an EPR 2B AgentPeerBinding DHT entry.
/// Source of truth: DHT (imagodei AgentPeerBinding entry).
/// Links an agent CID to a libp2p PeerId with an Ed25519 signature proof.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AgentPeerBindingView {
    /// CIDv1 base32 of the agent's EPR atom.
    pub agent_cid: String,
    /// libp2p PeerId of the bound peer (base58btc multihash).
    pub peer_id: String,
    /// RFC3339 date-time when the binding becomes valid.
    pub valid_from: String,
    /// RFC3339 date-time when the binding expires; None for non-expiring bindings.
    pub valid_until: Option<String>,
    /// Hex-encoded Ed25519 signature over canonical binding bytes.
    pub signature: String,
    pub dht_anchor_hash: String,
}

/// Aggregate view for the authenticated account portal.
/// Composes identity, recovery state, and hosting configuration into a single
/// response so the Angular account pillar can render the full account surface
/// without multiple sequential calls.
///
/// Source of truth: composite — individual sub-views each carry their own
/// `dhtAnchorHash` for provenance. See sub-view docs for per-entity truth layer.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AccountView {
    pub human: HumanView,
    pub active_key_rotation: Option<KeyRotationView>,
    pub recent_revocations: Vec<KeyRevocationView>,
    pub pending_recovery_requests: Vec<RecoveryRequestView>,
    pub emergency_contacts: Vec<HumanRelationshipView>,
    pub portal_hosts: Vec<PortalHostView>,
    /// True when this account holds a stewardship allocation on the local conductor.
    pub is_steward: bool,
    /// True when elohim-storage is connected to a local Holochain conductor
    /// (as opposed to operating through a hosted doorway-only path).
    pub has_local_conductor: bool,
}

/// Input for registering a new portal host URL for the authenticated human.
/// reach defaults to "trusted" when omitted.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AddPortalHostInputView {
    pub host_url: String,
    pub label: Option<String>,
    /// Reach classification — "public" | "trusted" | "private".
    /// Defaults to "trusted" when omitted.
    pub reach: Option<String>,
}

/// Input for an elohim defender submitting a specialist revocation.
/// Used when the defender detects anomalous behaviour and escalates a
/// revocation without waiting for a community quorum vote.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct SubmitSpecialistRevocationInputView {
    /// ActionHash (base64) of the human's source-chain entry being acted upon.
    pub human_action_hash: String,
    /// The agent pubkey being revoked (base64 or hex — must match DHT record).
    pub revoked_pub_key: String,
    /// Structured anomaly attestation produced by the elohim defender.
    /// Schema is defined by the specialist revocation protocol; validated
    /// by the imagodei coordinator zome before committing to DHT.
    pub anomaly_attestation: JsonVal,
}

/// Input for self-revocation (M4 fast-path: a human voluntarily revokes
/// one of their own keys).
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateSelfRevocationInputView {
    /// Base64-encoded AgentPubKey (e.g. "uhCAk...") of the key being revoked.
    /// Must belong to the same human as the caller.
    pub revoked_key: String,
    /// Reason — one of REVOCATION_REASONS recognised by the imagodei zome.
    pub reason: String,
}

/// Output of `create_self_revocation` — projected from the zome's
/// `KeyRevocationOutput` for HTTP camelCase responses.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateSelfRevocationOutputView {
    pub revocation_id: String,
    /// Base64-encoded ActionHash of the committed KeyRevocation entry.
    pub action_hash: String,
}

/// Input for an emergency-contact vote on a pending KeyRevocation.
/// The `revocation_id` arrives via the URL path `/recovery/:id/vote`,
/// so the body holds only the steward's vote payload.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct SubmitRevocationVoteInputView {
    /// `true` to approve the revocation, `false` to reject. The M4 zome
    /// only counts approvals towards the threshold; rejections are recorded
    /// for transparency.
    pub approved: bool,
    /// Free-text steward attestation — must be non-empty.
    pub attestation: String,
}

/// Output of `submit_revocation_vote` — projected from the zome's
/// `RevocationVoteOutput` for HTTP camelCase responses.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct SubmitRevocationVoteOutputView {
    pub vote_id: String,
    pub current_votes: u32,
    pub required_votes: u32,
    pub threshold_now_reached: bool,
}

/// Output of `add_portal_host` — projected from the zome's `ActionHash`
/// return for a uniform HTTP shape.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AddPortalHostOutputView {
    /// Base64-encoded ActionHash of the committed PortalHost entry.
    pub action_hash: String,
}

/// Output of `remove_portal_host`. Empty body on success — included for
/// uniform contract shape (clients may still expect a JSON body).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RemovePortalHostOutputView {
    /// Always `true` — the zome returns `()` on success; clients can use
    /// this as a presence check.
    pub deleted: bool,
}

