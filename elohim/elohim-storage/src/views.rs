//! View types for HTTP API boundary
//!
//! These types use camelCase serialization for TypeScript clients.
//! Wire types in models.rs use snake_case for database compatibility.
//!
//! Pattern:
//! - Service layer returns Wire types (Content, Relationship, etc.)
//! - HTTP layer converts to View types (ContentView, RelationshipView, etc.)
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

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
// ts_rs is used transitively via elohim-views; keep for any remaining #[derive(TS)] in this file
#[allow(unused_imports)]
use ts_rs::TS;

// CustodianMetrics, ObservationEntry, Schedule → views_convert/infrastructure.rs

// App / Content / Relationship / HumanRelationship → views_convert/lamad.rs
pub use crate::views_convert::lamad::content_view_from_epr_head;

// ContributorPresence, EconomicEvent, ReaCommitment, StewardshipAllocation,
// ContentStewardship, TokenMintEvent, TokenBalance, TokenTransfer,
// ResponsibilityDemandConfig, TokenDecayEvent, StewardCredential, PremiumGate,
// AccessGrant, RevenueSummary→StewardRevenueSummaryView, ContributorDashboard,
// ContributorImpactView, AgreementRow, StewardedNode, StewardAffinity,
// node_stewardship_view_from_with_name → views_convert/shefa.rs
pub use crate::views_convert::shefa::node_stewardship_view_from_with_name;

// Collective, CollectiveParticipation, GovernanceState, Challenge, Appeal,
// Proposal, Precedent, Discussion, ProposalOption, GovernanceSignal,
// GovernanceDisposition, Statement, StatementVote, GateDecisionAttestation,
// GateDecisionChallenge, ChallengeOutcome, GovernanceAction/Tally,
// AttestationRow, vote_view_from_vote, ranked_vote_view_from_ranked_vote → views_convert/qahal.rs
// ContentAttestation → views_convert/imagodei.rs (A.6)
pub use crate::views_convert::qahal::vote_view_from_vote;
pub use crate::views_convert::qahal::ranked_vote_view_from_ranked_vote;

// Content Mastery → views_convert/lamad.rs

// Comment → views_convert/lamad.rs

// DevicePolicy, LocalSession, Human, RecoveryWitnessRow, KeyRevocationRow,
// RevocationVoteRow, upsert_policy_to_db_input → views_convert/imagodei.rs
pub use crate::views_convert::imagodei::upsert_policy_to_db_input;

// ============================================================================
// Input View Types (API boundary for writes)
// ============================================================================
//
// These types accept camelCase JSON from TypeScript clients with parsed Value
// objects. They convert to internal DB Input types which use snake_case with
// String fields. This encapsulates JSON serialization at the API boundary.


// ============================================================================
// Content Input Views
// ============================================================================

use crate::db::content_diesel::CreateContentInput;


impl From<CreateContentInputView> for CreateContentInput {
    fn from(v: CreateContentInputView) -> Self {
        Self {
            id: v.id,
            title: v.title,
            description: v.description,
            content_type: v.content_type.unwrap_or_else(|| "concept".to_string()),
            content_format: v.content_format.unwrap_or_else(|| "markdown".to_string()),
            blob_hash: v.blob_hash,
            blob_cid: v.blob_cid,
            content_size_bytes: v.content_size_bytes.map(|s| s as i32),
            metadata_json: serialize_json_opt(&v.metadata),
            reach: v.reach.unwrap_or_else(|| "public".to_string()),
            created_by: v.created_by,
            tags: v.tags,
            content_body: v.content_body,
        }
    }
}


// ============================================================================
// Relationship Input Views
// ============================================================================

use crate::db::relationships_diesel::CreateRelationshipInput;


impl From<CreateRelationshipInputView> for CreateRelationshipInput {
    fn from(v: CreateRelationshipInputView) -> Self {
        Self {
            id: v.id,
            source_id: v.source_id,
            target_id: v.target_id,
            relationship_type: v.relationship_type,
            confidence: v.confidence.unwrap_or(1.0) as f32,
            inference_source: v.inference_source.unwrap_or_else(|| "explicit".to_string()),
            is_bidirectional: false,
            provenance_chain_json: None,
            governance_layer: None,
            reach: "commons".to_string(),
            metadata_json: serialize_json_opt(&v.metadata),
        }
    }
}

// ============================================================================
// Human Relationship Input Views
// ============================================================================

use crate::db::human_relationships::CreateHumanRelationshipInput;


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
            at_location: v.at_location,
        }
    }
}

// ============================================================================
// Stewardship Allocation Input Views
// ============================================================================

use crate::db::stewardship_allocations::{CreateAllocationInput, UpdateAllocationInput};


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







// upsert_policy_to_db_input → views_convert/imagodei.rs (re-exported above)


// ============================================================================
// Content Mastery Input View
// ============================================================================

use crate::db::content_mastery::CreateMasteryInput;


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

// TokenMintEvent, TokenBalance, TokenTransfer, ResponsibilityDemandConfig,
// TokenDecayEvent → views_convert/shefa.rs

// Collective, CollectiveParticipation → views_convert/qahal.rs
// CreateCollectiveInputView → views_convert/inputs.rs (A.9 pending — impl retained here until then)

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

// ============================================================================
// Account Package Views (Import/Export)
// ============================================================================











// EprHeadInputView, EprLamadContextInputView, EprShefaContextInputView,
// EprQahalContextInputView, EprRelationshipInputView, From<EprHeadInputView> for EprHead,
// EprHeadView, From<EprHead> for EprHeadView → views_convert/epr.rs
pub use crate::views_convert::epr::{
    EprHeadInputView, EprHeadView, EprLamadContextInputView, EprQahalContextInputView,
    EprRelationshipInputView, EprShefaContextInputView,
};

// Human → views_convert/imagodei.rs




// ============================================================================
// Custodian Metrics Views
// ============================================================================










// CustodianMetrics, report_custodian_metrics_into_upsert → views_convert/infrastructure.rs
pub use crate::views_convert::infrastructure::report_custodian_metrics_into_upsert;


// ============================================================================
// Data Protection Views
//
// Read-only aggregation views — assembled from custodian commitment data
// and DHT queries. No dedicated DB tables required.
// ============================================================================





// ============================================================================
// Shefa Dashboard Views
//
// Read-only aggregation views assembled from multiple sources by the
// compute handler. No dedicated DB tables.
// ============================================================================





























// GovernanceState, Challenge, Appeal → views_convert/qahal.rs
// Proposal, Precedent, Discussion → views_convert/qahal.rs





// ProposalOption, GovernanceSignal, GovernanceDisposition → views_convert/qahal.rs


// ContentAttestation → views_convert/imagodei.rs (A.6)



// StewardCredential, PremiumGate, AccessGrant, RevenueSummary,
// ContributorDashboard, ContributorImpactView, AgreementRow, StewardedNode
// → views_convert/shefa.rs

// REA Commitment Input Views → views_convert/inputs.rs

// Agreement Input Views → views_convert/inputs.rs

// Stewarded Node Input Views → views_convert/inputs.rs





impl From<CreateStewardedNodeInputView> for crate::db::stewarded_nodes::CreateStewardedNodeInput {
    fn from(v: CreateStewardedNodeInputView) -> Self {
        Self {
            id: v.id,
            display_name: v.display_name,
            claim_status: v.claim_status,
            cpu_cores: v.cpu_cores,
            memory_gb: v.memory_gb,
            storage_tb: v.storage_tb,
            bandwidth_mbps: v.bandwidth_mbps,
            steward_tier: v.steward_tier,
            custodian_opt_in: if v.custodian_opt_in { 1 } else { 0 },
            region: v.region,
            context_epr_id: v.context_epr_id,
            dht_anchor_hash: None,
            h_app_id: String::new(), // set by handler from AppContext
        }
    }
}


// CreateNodeStewardshipInputView → views_convert/inputs.rs

// Recognition Pipeline Views → views_convert/imagodei.rs
// (StageTrace + RecognitionDistributionResult live in imagodei per A.6)

// StewardAffinity, CreateStewardAffinityInputView → views_convert/shefa.rs + inputs.rs



// ============================================================================
// ElohimGate Views
// ============================================================================





// Statement, StatementVote → views_convert/qahal.rs







// Schedule, SpatialContext, Place, Hazard, RiskAlert → views_convert/infrastructure.rs


// ============================================================================
// Schema Version Tests
// ============================================================================

// ============================================================================
// Spatial Dashboard Views (Sprint 8 — Planet-Scale Governance Dashboard)
// ============================================================================











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

    #[test]
    fn recognition_trigger_input_deserializes_camel_case() {
        let json = r#"{"contentId":"c-1","eventType":"mastery_completion","rawAmount":10.0}"#;
        let view: super::RecognitionTriggerInputView = serde_json::from_str(json).unwrap();
        assert_eq!(view.content_id, "c-1");
        assert_eq!(view.event_type, "mastery_completion");
        assert!((view.raw_amount - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_update_content_input_view_deserializes_partial() {
        let json = r#"{"metadata": {"status": "done"}}"#;
        let view: super::UpdateContentInputView = serde_json::from_str(json).unwrap();
        assert!(view.title.is_none());
        assert!(view.tags.is_none());
        let meta = view.metadata.unwrap();
        assert_eq!(meta.0["status"], "done");
    }

    #[test]
    fn test_update_content_input_view_empty_patch_deserializes() {
        let json = r#"{}"#;
        let view: super::UpdateContentInputView = serde_json::from_str(json).unwrap();
        assert!(view.title.is_none());
        assert!(view.metadata.is_none());
    }
}

// ============================================================================
// Observation Sessions — Views
// ============================================================================

/// Input for beginning a new observation session.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeginObservationInputView {
    pub source: String,
    #[serde(default = "default_obs_ttl")]
    pub ttl_seconds: i32,
    #[serde(default)]
    pub metadata: Option<JsonVal>,
}


/// Response returned after beginning an observation session.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BeginObservationResponseView {
    pub session_id: String,
    pub expires_at: String,
}

/// Input for appending a single entry to an observation session.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationEntryInputView {
    pub origin: String,
    pub category: String,
    #[serde(default = "default_obs_severity")]
    pub severity: String,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub status_code: Option<i32>,
    pub message: String,
    #[serde(default)]
    pub context: Option<JsonVal>,
}


/// A single observation entry as returned in a report.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationEntryView {
    pub timestamp: String,
    pub origin: String,
    pub category: String,
    pub severity: String,
    pub method: Option<String>,
    pub path: Option<String>,
    pub status_code: Option<i32>,
    pub message: String,
    pub context: Option<JsonVal>,
}

impl From<crate::db::models::ObservationEntry> for ObservationEntryView {
    fn from(e: crate::db::models::ObservationEntry) -> Self {
        Self {
            timestamp: e.timestamp,
            origin: e.origin,
            category: e.category,
            severity: e.severity,
            method: e.method,
            path: e.path,
            status_code: e.status_code,
            message: e.message,
            context: parse_json_opt(&e.context_json),
        }
    }
}

/// A detected issue surfaced from observation entries.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationIssueView {
    pub id: String,
    pub category: String,
    pub severity: String,
    pub title: String,
    pub entry_count: usize,
    pub related_content_ids: Vec<String>,
    pub suggested_cause: String,
}

/// Duration metadata for an observation report.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationDurationView {
    pub started_at: String,
    pub ended_at: String,
    pub duration_ms: i64,
}

/// Aggregate counts across entries in a session.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationSummaryView {
    pub total_entries: usize,
    pub by_origin: std::collections::HashMap<String, usize>,
    pub by_severity: std::collections::HashMap<String, usize>,
    pub by_category: std::collections::HashMap<String, usize>,
}

/// Snapshot of relevant system health at report time.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationSystemStateView {
    pub storage_healthy: bool,
    pub conductor_connected: bool,
    pub p2p_peer_count: usize,
}

/// Full observation report returned when a session is closed.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationReportView {
    pub content_id: String,
    pub session_id: String,
    pub source: String,
    pub metadata: Option<JsonVal>,
    pub duration: ObservationDurationView,
    pub summary: ObservationSummaryView,
    pub issues: Vec<ObservationIssueView>,
    pub system_state: ObservationSystemStateView,
}

// =========================================================================
// Resilience Views
// =========================================================================









// GateDecisionAttestation, GateDecisionChallenge, ChallengeOutcome → views_convert/qahal.rs

// ============================================================================
// Elohim Reputation Profile View
// ============================================================================
//
// Source of truth: computed aggregation over the mishpat DNA DHT outcome graph
// (GateDecisionAttestation + GateDecisionChallenge + ChallengeOutcome entries).
// This is NOT a direct table projection — it is computed by
// `crate::db::elohim_reputation::compute` at query time from the SQLite
// projections of those DHT entries. If the projections and the DHT disagree,
// the DHT wins.
//
// No scalar score is returned. Dimensions are raw counts; time-decay and
// severity weighting are deferred to Phase 11+ and applied by consumers.
// Wire format governed by:
//   `elohim/sdk/schemas/v1/views/elohim-reputation-profile-view.schema.json`

// PeerStatusRow, build_peer_status_view, load_elohim_capability_from_env,
// load_render_capability_from_url, load_render_capability_from_url_blocking,
// PlacementGapRow, CapabilityExtensions, ReputationResult usage,
// elohim_reputation_profile_view_from_result, put_blob_response_view_from_manifest,
// ObservationRow, ObservationDiversitySummaryRow → views_convert/infrastructure.rs
pub use crate::views_convert::infrastructure::{
    build_peer_status_view, load_elohim_capability_from_env, load_render_capability_from_url,
    load_render_capability_from_url_blocking, elohim_reputation_profile_view_from_result,
    put_blob_response_view_from_manifest,
};

// ============================================================================
// Peer Status View
// ============================================================================
//
// Source of truth: Holochain infrastructure DNA DHT (Notarized, Category A).
// This view is a read-optimised SQLite projection populated by
// `InfrastructureSignal::PeerStatusRecorded` post-commit projections.
// If the projection and the DHT disagree, the DHT wins.
//
// Wire format governed by:
//   `elohim/sdk/schemas/v1/views/peer-status-view.schema.json`
//   `elohim/sdk/schemas/v1/views/elohim-capability-profile.schema.json`
//
// `elohimCapability` is operator-configured at startup (Phase 9). The field is
// optional: a peer that does not run an elohim-agent omits it. Phase 10+ may
// auto-detect capabilities from a live elohim-agent-service.



// CapabilityExtensions → views_convert/infrastructure.rs (re-exported via elohim_views::infrastructure)



// From<PeerStatusRow> → views_convert/infrastructure.rs

// build_peer_status_view → views_convert/infrastructure.rs (re-exported above)
// load_elohim_capability_from_env → views_convert/infrastructure.rs (re-exported above)
// load_render_capability_from_url → views_convert/infrastructure.rs (re-exported above)
// load_render_capability_from_url_blocking → views_convert/infrastructure.rs (re-exported above)








// Placement Gap + Resilience Snapshot Views → views_convert/infrastructure.rs





// RecoveryWitnessRow, KeyRevocationRow, RevocationVoteRow → views_convert/imagodei.rs

// ============================================================================
// EPR views (Phase 2a)
// Source of truth: EPR atom (self-notarized via content-address + Ed25519).
// ============================================================================










// =============================================================================
// Recovery Protocol Phase 2 — M5 Views
// Auth Portal Convergence + Revocation UX + Stub Defender
// =============================================================================












// ───────────────────────────────────────────────────────────────────────────
// Light-Up-Topology Phase 1 — operational distribution + cluster + reciprocity
// ───────────────────────────────────────────────────────────────────────────
//
// Wire shapes for the light-up-topology epic. These are Operational (Category
// C) projections — composed per request from rea_commitments + economic_events
// + peer_identity_bindings + libp2p swarm state. None of them introduce a new
// DHT entry type; none of them are persisted on the elohim-storage side.

































#[cfg(test)]
mod federation_canonical_tests {
    use super::*;

    #[test]
    fn request_canonical_round_trips() {
        let req = ViewFederationRequest {
            view_kind: ViewKind::Cluster,
            agent_cid: "agent_test".to_string(),
            request_id: "req_001".to_string(),
        };
        let bytes = req.canonical_bytes();
        let back: ViewFederationRequest = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(back, req);
    }

    #[test]
    fn slice_canonical_excludes_stale_since_ms() {
        // Two slices identical except for stale_since_ms must produce the SAME
        // canonical signing bytes — that's the contract that makes signatures
        // verifiable across receivers with drifting clocks.
        let base = ViewSlice {
            peer_id: "12D3KooWAAA".to_string(),
            view_kind: ViewKind::Cluster,
            freshness: Freshness {
                state: FreshnessState::Live,
                stale_since_ms: Some(12_345),
            },
            payload: JsonVal(serde_json::Value::Null),
            signature: String::new(),
        };
        let mut other = base.clone();
        other.freshness.stale_since_ms = Some(99_999);
        assert_eq!(
            base.canonical_bytes_for_signing(),
            other.canonical_bytes_for_signing(),
            "stale_since_ms must NOT influence the signing canonical"
        );

        // And signature must NOT influence the canonical either — that would
        // be circular (you can't sign something whose bytes depend on the
        // signature you're producing).
        let mut signed = base.clone();
        signed.signature = "fake-base64-signature".to_string();
        assert_eq!(
            base.canonical_bytes_for_signing(),
            signed.canonical_bytes_for_signing(),
            "signature field must NOT be self-referential in the canonical"
        );
    }

    #[test]
    fn slice_canonical_serializes_freshness_state_as_camel_case_field() {
        // Sanity check: confirm the canonical bytes parse back as a msgpack
        // map whose `freshnessState` value is "live" (snake_case enum
        // serialization, camelCase field name to match the wire format).
        let slice = ViewSlice {
            peer_id: "12D3KooWAAA".to_string(),
            view_kind: ViewKind::Cluster,
            freshness: Freshness {
                state: FreshnessState::Live,
                stale_since_ms: Some(12_345),
            },
            payload: JsonVal(serde_json::json!({"hello": "world"})),
            signature: String::new(),
        };
        let bytes = slice.canonical_bytes_for_signing();
        let parsed: serde_json::Value = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(parsed["freshnessState"], serde_json::json!("live"));
        assert_eq!(parsed["viewKind"], serde_json::json!("cluster"));
        assert_eq!(parsed["peerId"], serde_json::json!("12D3KooWAAA"));
        assert_eq!(parsed["payload"], serde_json::json!({"hello": "world"}));
        // staleSinceMs must NOT appear in the canonical.
        assert!(
            parsed.get("staleSinceMs").is_none(),
            "staleSinceMs must not appear in signing canonical"
        );
    }
}

// ============================================================================
// Peer Transport Manifest View (Phase 12)
// ============================================================================
//
// Source-of-truth pairing: peer_transport_manifest SQLite table,
// populated by p2p_iroh::peer_map record_* fns.





// ============================================================================

// Observation Views
// ============================================================================


// From<ObservationRow> → views_convert/infrastructure.rs
// From<ObservationDiversitySummaryRow> → views_convert/infrastructure.rs

// AttestationRow, GovernanceActionRow, GovernanceActionTallyRow → views_convert/qahal.rs
// vote_view_from_vote, ranked_vote_view_from_ranked_vote → views_convert/qahal.rs (re-exported above)
// node_stewardship_view_from_with_name → views_convert/shefa.rs (re-exported above)

// Re-export wire types from elohim-views so in-tree consumers' `crate::views::TypeName`
// continues to resolve. New code should import directly from `elohim_views::*`.
pub use elohim_views::shared::*;
pub use elohim_views::lamad::*;
pub use elohim_views::shefa::*;
pub use elohim_views::qahal::*;
pub use elohim_views::imagodei::*;
pub use elohim_views::infrastructure::*;
pub use elohim_views::epr::*;
pub use elohim_views::inputs::*;

// elohim_reputation_profile_view_from_result → views_convert/infrastructure.rs (re-exported above)
// put_blob_response_view_from_manifest → views_convert/infrastructure.rs (re-exported above)
