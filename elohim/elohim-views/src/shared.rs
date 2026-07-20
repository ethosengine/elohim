//! shared view types — migrated from elohim-storage/src/views.rs (VIEWS.T2).
//!
//! This module also owns `JsonVal` (the ts-rs-safe wrapper for serde_json::Value),
//! helper functions used by multiple domain modules, and schema-version constants.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;
// MessagePack for canonical bytes (ViewFederationRequest, ViewSlice signing)
#[allow(unused_imports)]
use rmp_serde;

// ============================================================================
// JsonVal — ts-rs-safe wrapper for serde_json::Value
// ============================================================================

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

// ============================================================================
// Helper functions used by domain modules
// ============================================================================

/// Parse a JSON string to JsonVal, returning None on parse failure.
/// This encapsulates the storage format (TEXT) from the API contract.
pub fn parse_json_opt(json_str: &Option<String>) -> Option<JsonVal> {
    json_str
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .map(JsonVal)
}

/// Parse a required JSON string to JsonVal, returning empty object on failure.
pub fn parse_json(json_str: &str) -> JsonVal {
    JsonVal(serde_json::from_str(json_str).unwrap_or(Value::Object(serde_json::Map::new())))
}

/// Serialize an Option<JsonVal> back to an Option<String>.
pub fn serialize_json_opt(value: &Option<JsonVal>) -> Option<String> {
    value
        .as_ref()
        .map(|v| serde_json::to_string(&v.0).unwrap_or_else(|_| "null".to_string()))
}

/// Default schema version for InputView types.
/// Clients that omit schemaVersion are implicitly version 1.
pub fn default_schema_version() -> u32 {
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

pub fn default_true() -> bool {
    true
}

pub fn default_30i32() -> i32 {
    30
}

pub fn default_governance_layer() -> String {
    "community".to_string()
}

pub fn default_profile_reach() -> String {
    "collective".to_string()
}

pub fn default_voting_mechanism() -> String {
    "consensus".to_string()
}

pub fn default_claim_status() -> String {
    "pending".to_string()
}

pub fn default_steward_tier() -> String {
    "observer".to_string()
}

pub fn default_relationship() -> String {
    "member".to_string()
}

pub fn default_hazard_source() -> String {
    "community".to_string()
}

pub fn default_obs_ttl() -> i32 {
    3600
}

pub fn default_obs_severity() -> String {
    "info".to_string()
}

/// Freshness state bucket for cluster + topology + slice views.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "snake_case")]
pub enum FreshnessState {
    Live,
    Stale,
    Offline,
    CachedOfflineUntilReconnect,
    Unverifiable,
    AllOffline,
}

/// Liveness/staleness indicator. `staleSinceMs` is populated when state ≠ live.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct Freshness {
    pub state: FreshnessState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale_since_ms: Option<u64>,
}

/// Which kind of view a federation slice represents.
///
/// `ProjectionInventory` (P1 reconciliation stream) is NOT a per-device view
/// like the others — it carries a `table` discriminator naming which projection
/// table the requester wants the responder's `(id, dhtAnchorHash)` inventory
/// for. v1 supports only `"rea_commitments"`; the discriminator is the seam for
/// `agreements` / `economic_events` later. The inventory itself rides in the
/// `ViewSlice.payload` as a [`ProjectionInventoryPayload`].
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq, Hash)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "snake_case")]
pub enum ViewKind {
    Cluster,
    PeerTopology,
    /// Projection-inventory discovery for the reconciliation stream. `table`
    /// names the projection (v1: `"rea_commitments"` only).
    ProjectionInventory {
        table: String,
    },
}

/// One discovered projection row in a [`ProjectionInventoryPayload`]: the
/// logical id and the DHT anchor hash the responder's projection holds for it.
/// The reconciler diffs these against its own projection: missing id OR a
/// different `dhtAnchorHash` ⇒ a convergence gap to heal from its OWN conductor.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ProjectionInventoryEntry {
    pub id: String,
    /// May be empty when the responder's projection row carries no anchor yet
    /// (un-anchored bulk-seed row). An empty anchor still counts as "present"
    /// for discovery — the reconciler heals it from its own conductor.
    pub dht_anchor_hash: String,
}

/// Response payload (carried in `ViewSlice.payload`) for a `ProjectionInventory`
/// federation request. `entries` is capped at the most-recent-N rows; `total`
/// reports the full row count so the requester can tell the inventory was
/// truncated (v1 cap documented in `p2p::projection_reconcile`).
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ProjectionInventoryPayload {
    /// The projection table this inventory is for (echoes the request's `table`).
    pub table: String,
    /// Total rows in the responder's projection for `table` (may exceed
    /// `entries.len()` when truncated by the cap).
    #[ts(type = "number")]
    pub total: usize,
    /// Most-recent-N `(id, dhtAnchorHash)` pairs, newest first.
    pub entries: Vec<ProjectionInventoryEntry>,
}

/// Per-device slice returned over the view-federation/1.0.0 libp2p protocol;
/// signed by the responding peer's agent key. The meta-shape that federates
/// cluster + topology views across household peers.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ViewSlice {
    pub peer_id: String,
    pub view_kind: ViewKind,
    pub freshness: Freshness,
    pub payload: JsonVal,
    pub signature: String,
}

/// Request envelope for `/elohim/view-federation/1.0.0` — peer A asks peer B
/// for a signed `ViewSlice` of `view_kind` on behalf of `agent_cid`.
///
/// F-T16: wire envelope only. The codec lands in F-T17 and the responder
/// handler in F-T20.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ViewFederationRequest {
    pub view_kind: ViewKind,
    pub agent_cid: String,
    pub request_id: String,
    /// Rotating window offset for `ProjectionInventory` requests: the responder
    /// serves its inventory starting at this offset (0 / absent = the hot set, as
    /// before), so successive reconcile sweeps advance a window across the whole
    /// corpus rather than re-advertising only the capped hot set forever. Ignored
    /// for non-inventory view kinds.
    ///
    /// ## Wire compatibility (MANDATORY — mixed-version peers during rolling
    /// deploys)
    ///
    /// Additive and optional. MessagePack via `to_vec_named` is map-keyed and this
    /// struct is NOT `deny_unknown_fields`, so:
    /// - a NEW peer sending this key to an OLD responder: the old struct lacks the
    ///   field and ignores the unknown key → serves offset 0 (yesterday's behavior);
    /// - an OLD peer sending no key to a NEW responder: `#[serde(default)]` yields
    ///   `None` → offset 0 (yesterday's behavior).
    ///
    /// `skip_serializing_if` keeps the `None` wire bytes byte-identical to the
    /// pre-field encoding, so the `canonical_bytes` dedup key is unchanged for
    /// every existing caller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory_offset: Option<u32>,
}

/// Response envelope for `/elohim/view-federation/1.0.0` — peer B returns the
/// signed slice. Echoes `view_kind` + `agent_cid` + `request_id` so the caller
/// can dedup replies in the F-T21 aggregator.
///
/// PartialEq is intentionally NOT derived: `ViewSlice.payload` is `JsonVal`
/// (`serde_json::Value`), which does not implement `PartialEq` cleanly.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ViewFederationResponse {
    pub view_kind: ViewKind,
    pub agent_cid: String,
    pub request_id: String,
    pub slice: ViewSlice,
}

impl ViewFederationRequest {
    /// Canonical bytes for signing/dedup keys. MessagePack named-fields encoding
    /// — same shape as the wire codec uses, so request bytes used by the codec
    /// and request bytes used as a dedup-key are byte-identical.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        rmp_serde::to_vec_named(self).expect("canonical request msgpack should not fail")
    }
}

impl ViewSlice {
    /// Bytes-to-sign for the slice's `signature` field:
    /// `view_kind || peer_id || freshness_state || payload` in MessagePack
    /// named-fields canonical form.
    pub fn canonical_bytes_for_signing(&self) -> Vec<u8> {
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Canonical<'a> {
            view_kind: &'a ViewKind,
            peer_id: &'a str,
            freshness_state: &'a FreshnessState,
            payload: &'a JsonVal,
        }
        rmp_serde::to_vec_named(&Canonical {
            view_kind: &self.view_kind,
            peer_id: &self.peer_id,
            freshness_state: &self.freshness.state,
            payload: &self.payload,
        })
        .expect("canonical slice msgpack should not fail")
    }
}

#[cfg(test)]
mod inventory_offset_wire_compat_tests {
    use super::*;

    /// The pre-field shape of the request — a stand-in for an OLD (not-yet-updated)
    /// peer's struct during a rolling deploy. It has NO `inventory_offset` and is
    /// NOT `deny_unknown_fields`, exactly like the real struct was yesterday.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    struct OldViewFederationRequest {
        view_kind: ViewKind,
        agent_cid: String,
        request_id: String,
    }

    #[test]
    fn none_offset_is_byte_identical_to_pre_field_encoding() {
        // `skip_serializing_if` means a `None` offset adds no wire bytes, so every
        // existing caller's `canonical_bytes` (dedup key) is unchanged.
        let new = ViewFederationRequest {
            view_kind: ViewKind::Cluster,
            agent_cid: "agent".into(),
            request_id: "r".into(),
            inventory_offset: None,
        };
        let old = OldViewFederationRequest {
            view_kind: ViewKind::Cluster,
            agent_cid: "agent".into(),
            request_id: "r".into(),
        };
        assert_eq!(
            rmp_serde::to_vec_named(&new).unwrap(),
            rmp_serde::to_vec_named(&old).unwrap(),
            "None offset must serialize byte-identically to the pre-field struct"
        );
    }

    #[test]
    fn old_peer_decodes_new_bytes_ignoring_the_unknown_key() {
        // NEW peer emits a request carrying inventory_offset; an OLD peer must
        // decode it at yesterday's behavior (offset 0), ignoring the extra key.
        let new = ViewFederationRequest {
            view_kind: ViewKind::ProjectionInventory {
                table: "content".into(),
            },
            agent_cid: "agent".into(),
            request_id: "r".into(),
            inventory_offset: Some(1000),
        };
        let bytes = rmp_serde::to_vec_named(&new).unwrap();
        let old: OldViewFederationRequest =
            rmp_serde::from_slice(&bytes).expect("old struct tolerates the unknown offset key");
        assert_eq!(old.agent_cid, "agent");
        assert_eq!(old.request_id, "r");
    }

    #[test]
    fn new_peer_decodes_old_bytes_defaulting_offset_to_none() {
        // OLD peer emits a request without inventory_offset; a NEW peer must
        // decode it with the field defaulting to None (offset 0).
        let old = OldViewFederationRequest {
            view_kind: ViewKind::ProjectionInventory {
                table: "content".into(),
            },
            agent_cid: "agent".into(),
            request_id: "r".into(),
        };
        let bytes = rmp_serde::to_vec_named(&old).unwrap();
        let new: ViewFederationRequest =
            rmp_serde::from_slice(&bytes).expect("new struct defaults the missing offset");
        assert_eq!(new.inventory_offset, None, "missing key defaults to None");
        assert_eq!(new.agent_cid, "agent");
    }

    #[test]
    fn offset_round_trips_when_present() {
        let req = ViewFederationRequest {
            view_kind: ViewKind::ProjectionInventory {
                table: "content".into(),
            },
            agent_cid: "agent".into(),
            request_id: "r".into(),
            inventory_offset: Some(2000),
        };
        let bytes = rmp_serde::to_vec_named(&req).unwrap();
        let back: ViewFederationRequest = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(back, req);
    }
}
