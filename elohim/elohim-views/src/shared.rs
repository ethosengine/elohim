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
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq, Hash)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "snake_case")]
pub enum ViewKind {
    Cluster,
    PeerTopology,
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
