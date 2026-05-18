//! View types for HTTP API boundary
//!
//! These types use camelCase serialization for TypeScript clients. Wire types
//! in `db/models.rs` use snake_case for database compatibility; conversion
//! happens here (or — for the bulk of the converters — in `views_convert/`).
//!
//! ## Layout after Plan 3.A decomposition
//!
//! - **Wire-shape View structs** live in the `elohim-views` crate
//!   (`elohim_views::{shared,lamad,shefa,qahal,imagodei,infrastructure,epr,inputs}`)
//!   and are re-exported at the bottom of this file so `crate::views::TypeName`
//!   continues to resolve for in-tree callers. New code should import directly
//!   from `elohim_views::*`.
//! - **Wire→View `From` impls** that touch DB types live in
//!   `crate::views_convert::<domain>` (one sibling-module per pillar).
//! - **Observation Session response shapes** remain here as a transitional home
//!   until they migrate to `elohim_views::infrastructure` — they're locally
//!   composed and don't share the InputView contract with TypeScript clients.
//!
//! Design principles preserved:
//! - Boolean coercion: SQLite stores bools as i32; Views expose proper bools.
//! - JSON parsing: internal `*_json` strings are parsed to `serde_json::Value`.
//! - InputView types are camelCase-in, snake_case-out; conversion is encapsulated
//!   at the API boundary.

use serde::{Deserialize, Serialize};

// Re-exports: free functions that callers reference via `crate::views::<name>` —
// preserved here so the public surface from before A.3–A.9 still resolves.
pub use crate::views_convert::epr::{
    EprHeadInputView, EprHeadView, EprLamadContextInputView, EprQahalContextInputView,
    EprRelationshipInputView, EprShefaContextInputView,
};
pub use crate::views_convert::imagodei::upsert_policy_to_db_input;
pub use crate::views_convert::infrastructure::{
    build_peer_status_view, elohim_reputation_profile_view_from_result,
    load_elohim_capability_from_env, load_render_capability_from_url,
    load_render_capability_from_url_blocking, put_blob_response_view_from_manifest,
    report_custodian_metrics_into_upsert,
};
pub use crate::views_convert::lamad::content_view_from_epr_head;
pub use crate::views_convert::qahal::{ranked_vote_view_from_ranked_vote, vote_view_from_vote};
pub use crate::views_convert::shefa::node_stewardship_view_from_with_name;

// ============================================================================
// Observation Sessions — Views
// ============================================================================
//
// These shapes drive the `/observation` HTTP endpoint family. They are composed
// per-request (no dedicated DB tables on the response side), so they live here
// rather than in `elohim_views`. The corresponding `ObservationEntry` DB→View
// conversion is the single `From` impl in this file.

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
// Wire-type re-exports
// ============================================================================
//
// `crate::views::TypeName` resolves to `elohim_views::*::TypeName` so existing
// in-tree imports continue to work without churn. New code should import
// directly from `elohim_views::*`.

pub use elohim_views::epr::*;
pub use elohim_views::imagodei::*;
pub use elohim_views::infrastructure::*;
// InputView types: re-exported for callers, but unused inside this crate under
// the WASM rustflags build. allow keeps clippy -D warnings happy.
#[allow(unused_imports)]
pub use elohim_views::inputs::*;
pub use elohim_views::lamad::*;
pub use elohim_views::qahal::*;
pub use elohim_views::shared::*;
pub use elohim_views::shefa::*;
