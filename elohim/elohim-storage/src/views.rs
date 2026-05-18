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

// All InputView → DbInput From impls → views_convert/inputs.rs
// (CreateContentInputView, CreateRelationshipInputView, CreateHumanRelationshipInputView,
//  CreateContributorPresenceInputView, InitiateClaimInputView, CreateEconomicEventInputView,
//  CreateAllocationInputView, UpdateAllocationInputView, CreateMasteryInputView,
//  CreateCollectiveInputView, CreateStewardedNodeInputView)

// upsert_policy_to_db_input → views_convert/imagodei.rs (re-exported above)
// TokenMintEvent, TokenBalance, TokenTransfer, ResponsibilityDemandConfig,
// TokenDecayEvent → views_convert/shefa.rs

// Collective, CollectiveParticipation → views_convert/qahal.rs
// CreateCollectiveInputView → views_convert/inputs.rs

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

// schema_version_tests → views_convert/inputs.rs

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
