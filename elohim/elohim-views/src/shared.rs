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
    /// P2P twin of `GET /db/content/{id}/head-record`: ask the responder for
    /// the serialized DHT `Record` behind ITS declared head for `content_id`,
    /// so a peer with a full-arc gossip gap can declare a head it cannot
    /// retrieve itself (declare-carries-Record). Not agent-scoped — like
    /// `ProjectionInventory`, the answer is "what THIS peer holds".
    ///
    /// ## Wire-compatibility (load-bearing)
    ///
    /// This variant is ADDITIVE but NOT backward-decodable: an externally
    /// tagged enum has no `#[serde(other)]` escape, so a pre-cure peer's
    /// `ViewKind` decode FAILS on this request and the transport surfaces a
    /// codec/inbound error. Requesters MUST treat that error as an explicit
    /// "this peer is too old to serve head records", log it once, and fall
    /// through to the author path — never retry-loop, never panic. See
    /// `p2p::head_record_client`.
    ContentHeadRecord {
        content_id: String,
    },
}

/// Response payload (carried in `ViewSlice.payload`) for a
/// [`ViewKind::ContentHeadRecord`] request — the P2P twin of the
/// `GET /db/content/{id}/head-record` response body.
///
/// All fields are optional so an HONEST ABSENCE ("I have no head for that id",
/// or "I have a head but my conductor cannot retrieve its Record") is a
/// well-formed answer rather than a transport error: the requester then
/// declares without a carried record, or falls through to the author path.
///
/// `record` is standard base64 of the serialized `Record` — an OPAQUE token to
/// every layer between the two conductors, re-verified in wasm by the
/// receiving conductor before the declaration is honored.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ContentHeadRecordPayload {
    /// Echoes the requested content id.
    pub content_id: String,
    /// The responder's declared head action, when it has one.
    #[serde(default)]
    pub head_action_hash: Option<String>,
    /// Holochain `Timestamp` (i64 microseconds) of the responder's declaration,
    /// when its projection carries the ordering.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub declared_at: Option<i64>,
    /// Standard-base64 serialized `Record` for `head_action_hash`, when this
    /// peer's conductor could retrieve it.
    #[serde(default)]
    pub record: Option<String>,
    /// ADDITIVE (2026-08-03, adopt-before-author evidence supply): WHY `record`
    /// is absent, when it is. `None` whenever bytes ARE served — and also
    /// whenever the responder has nothing honest to say (no conductor bridge at
    /// all), because "I never asked my conductor" establishes nothing about
    /// whether the bytes exist.
    ///
    /// Vocabulary, matching the responder's three collapse sites:
    /// `no_record` (the conductor answered cleanly and holds none — STRUCTURAL,
    /// no capacity relief changes it) | `conductor_error` | `budget_elapsed`.
    ///
    /// ## Why a `String` and not an enum
    ///
    /// A unit-variant serde enum has no `#[serde(other)]` escape, so a peer
    /// running a FUTURE version that adds a fourth reason would make this whole
    /// payload undecodable here — and `head_record_client` maps an undecodable
    /// payload to `Answer::Unreachable`, i.e. one added vocabulary word would
    /// silently stop adoption against newer peers. A tolerant string decoded
    /// into a typed reason with an `Unknown` fallback
    /// (`services::head_adoption::RecordAbsentReason::from_wire`) keeps the
    /// typing where it can be total and the tolerance where the fleet is mixed.
    ///
    /// ## Wire compatibility (MANDATORY — mixed-version peers during rolling
    /// deploys)
    ///
    /// Additive and optional, same discipline as
    /// [`ProjectionInventoryEntry::declared_head_action_hash`] and
    /// [`ViewFederationRequest::inventory_offset`]:
    /// - a NEW responder answering an OLD requester: this payload rides the
    ///   frame as an opaque `JsonVal` map, and the old requester's
    ///   `ContentHeadRecordPayload` is NOT `deny_unknown_fields`, so the extra
    ///   key is ignored → yesterday's behaviour;
    /// - an OLD responder answering a NEW requester: the key is missing and
    ///   `#[serde(default)]` yields `None` → classified `Unknown`, which takes
    ///   the ordinary backoff, i.e. yesterday's behaviour.
    ///
    /// `skip_serializing_if` keeps the `None` encoding byte-identical to the
    /// pre-field one, so a served-bytes answer is unchanged on the wire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_absent_reason: Option<String>,
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
    /// ADDITIVE (adopt-before-author): the responder's NOTARY-DECLARED head for
    /// this id, when it carries one. Distinct from `dht_anchor_hash` — an anchor
    /// is "the action my projection last saw", a declaration is "the action a
    /// canonical channel elected". A peer advertising this is what lets a
    /// restarting peer ADOPT rather than re-author its own root.
    ///
    /// `#[serde(default)]` (and no `deny_unknown_fields` on this struct) makes
    /// this bidirectionally wire-compatible: a pre-cure peer omits the key and
    /// it decodes to `None`; a pre-cure peer receiving it ignores the key. No
    /// protocol-version bump.
    ///
    /// **HINT, never truth.** This value is NEVER stamped into a local row. It
    /// only triggers a verified fetch (own-conductor resolve, then a
    /// `ContentHeadRecord` fetch → declare-with-carried-record through the
    /// conductor). See `services::head_adoption`.
    #[serde(default)]
    pub declared_head_action_hash: Option<String>,
    /// Holochain `Timestamp` (i64 microseconds) the responder's declaration
    /// carries, when its projection has the ordering. Additive, same
    /// compatibility rules as `declared_head_action_hash`.
    ///
    /// **Not globally comparable.** The zome substitutes the RECEIVING
    /// conductor's `sys_time` on the carried-record branch, so this must never
    /// be used to order declarations ACROSS peers. It is carried for
    /// observability and for the conductor to arbitrate, never for a
    /// Rust-side "newest wins" election.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub declared_head_at: Option<i64>,
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
    /// ADDITIVE (T4, head-plane trust-gradient program plan §3 L2): set by the
    /// responder ONLY when the request carried
    /// [`ViewFederationRequest::head_corpus_digest`] for the `content` table —
    /// `Some(true)` when the responder's OWN ready corpus digest equals the
    /// requester's digest (the requester is already in sync with this peer's
    /// head plane and the responder omits inventory enumeration), `Some(false)`
    /// on a ready-corpus mismatch, `None` when either side is inside its
    /// bulk-seed amber window, the requester sent no digest, or the request is
    /// for a different table. Amber abstention is intentionally distinct from
    /// mismatch: the anchored subset is still changing while witness work
    /// remains.
    ///
    /// T5's requester remains dormant by default behind
    /// `ELOHIM_HEAD_CORPUS_DIGEST` and additionally suppresses the field while
    /// its own distribution-safe corpus is amber. The operator flips it only
    /// after the T4 responder is fleet-wide.
    ///
    /// `skip_serializing_if` keeps the `None` encoding byte-identical to the
    /// pre-field shape, same discipline as `declared_head_action_hash` /
    /// `inventory_offset`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_sync: Option<bool>,
    /// ADDITIVE (T8, head-plane trust-gradient program plan §3 L5): a
    /// `trust::snapshot::CarriedSnapshot` — a `HeadSetSnapshot` plus its
    /// optional detached signer proof — encoded as base64(dag-cbor canonical
    /// bytes).
    ///
    /// **Bytes, not a structured field, on purpose.** The snapshot's CID is
    /// minted over exactly these canonical bytes (`epr_codec::encode_epr_head`
    /// pattern, dag-cbor 0x71), so carrying the bytes lets the receiver
    /// re-derive the CID and re-verify the signature over the SAME preimage
    /// the signer signed. A structured field would be re-encoded by this
    /// crate's serializer and could differ byte-for-byte from what was signed
    /// — evidence-not-authority (C5) requires the receiver to check the
    /// original bytes, not a re-serialization of them. It also keeps this
    /// wire crate free of a `cid`/dag-cbor dependency.
    ///
    /// **NO responder in this release constructs this field.** Same rollout
    /// rule as `in_sync` / `head_corpus_digest` (the `ListDocumentsSince`
    /// precedent): the DECODE + verify path ships fleet-wide first, and a
    /// later task flips a sender behind a config flag. Receiving one changes
    /// no behavior today — `trust::snapshot::verify_snapshot` returns a
    /// verdict that is logged, never adopted.
    ///
    /// `skip_serializing_if` keeps the `None` encoding byte-identical to the
    /// pre-field shape, same discipline as `in_sync` / `inventory_offset`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_set_snapshot: Option<String>,
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
    /// ADDITIVE (T4, head-plane trust-gradient program plan §3 L2): the
    /// requester's own `db::content_diesel::head_corpus_digest` for the
    /// `content` `ProjectionInventory` table, carried so the responder can
    /// answer `ProjectionInventoryPayload::in_sync` without the requester
    /// paying a full inventory round-trip when the two peers already agree.
    /// Ignored for non-`content`, non-`ProjectionInventory` view kinds.
    ///
    /// **T5 requester rollout.** `projection_reconcile` sets this field only
    /// when `ELOHIM_HEAD_CORPUS_DIGEST` is enabled AND every
    /// distribution-safe local content row has a DHT anchor. The flag defaults
    /// off until the T4 responder is fleet-wide; the readiness gate suppresses
    /// digest flap during the hours-long bulk-seed witness window.
    ///
    /// ## Wire compatibility (MANDATORY — mixed-version peers during rolling
    /// deploys)
    ///
    /// Additive and optional, same discipline as `inventory_offset`:
    /// - a NEW peer sending this key to an OLD responder: the old struct lacks
    ///   the field and ignores the unknown key → no `in_sync` answer, so the
    ///   requester follows the ordinary inventory path;
    /// - an OLD peer sending no key to a NEW responder: `#[serde(default)]`
    ///   yields `None` → the responder skips the digest comparison and omits
    ///   `in_sync` (yesterday's behavior).
    ///
    /// `skip_serializing_if` keeps the `None` wire bytes byte-identical to the
    /// pre-field encoding, so the `canonical_bytes` dedup key is unchanged for
    /// every existing caller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_corpus_digest: Option<String>,
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
            head_corpus_digest: None,
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
            head_corpus_digest: None,
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
            head_corpus_digest: None,
        };
        let bytes = rmp_serde::to_vec_named(&req).unwrap();
        let back: ViewFederationRequest = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(back, req);
    }

    /// Yesterday's `ProjectionInventoryEntry` — the two declared-head fields
    /// did not exist. Kept as a literal so the compat assertions below test the
    /// REAL old shape rather than a `skip_serializing_if` trick on the new one.
    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    struct OldProjectionInventoryEntry {
        id: String,
        dht_anchor_hash: String,
    }

    #[test]
    fn new_peer_decodes_old_inventory_entry_defaulting_declared_head_to_none() {
        // OLD peer emits {id, dhtAnchorHash}; the NEW struct must decode it with
        // both additive fields defaulting to None — the adopt path then simply
        // has no hint from that peer and falls through to today's behavior.
        let old = OldProjectionInventoryEntry {
            id: "elohim-host-landing".into(),
            dht_anchor_hash: "uhCkkOLD".into(),
        };

        let msgpack = rmp_serde::to_vec_named(&old).unwrap();
        let via_msgpack: ProjectionInventoryEntry =
            rmp_serde::from_slice(&msgpack).expect("new struct defaults the missing declared head");
        assert_eq!(via_msgpack.id, "elohim-host-landing");
        assert_eq!(via_msgpack.dht_anchor_hash, "uhCkkOLD");
        assert_eq!(via_msgpack.declared_head_action_hash, None);
        assert_eq!(via_msgpack.declared_head_at, None);

        // The inventory rides the ViewSlice payload as JSON before the frame is
        // msgpack'd, so the JSON leg must default identically.
        let json = serde_json::to_string(&old).unwrap();
        let via_json: ProjectionInventoryEntry =
            serde_json::from_str(&json).expect("JSON leg defaults the missing declared head too");
        assert_eq!(via_json.declared_head_action_hash, None);
        assert_eq!(via_json.declared_head_at, None);
    }

    #[test]
    fn old_peer_decodes_new_inventory_entry_ignoring_declared_head_keys() {
        // NEW peer advertises a declaration; an OLD peer must still decode the
        // entry at yesterday's behavior rather than erroring the WHOLE payload
        // (one undecodable entry drops the peer for the sweep — see
        // `projection_reconcile`'s "peer inventory payload undecodable" arm).
        let new = ProjectionInventoryEntry {
            id: "elohim-host-landing".into(),
            dht_anchor_hash: "uhCkkANCHOR".into(),
            declared_head_action_hash: Some("uhCkkDECLARED".into()),
            declared_head_at: Some(1_753_000_000_000_000),
        };

        let msgpack = rmp_serde::to_vec_named(&new).unwrap();
        let old: OldProjectionInventoryEntry =
            rmp_serde::from_slice(&msgpack).expect("old struct tolerates the unknown keys");
        assert_eq!(old.id, "elohim-host-landing");
        assert_eq!(old.dht_anchor_hash, "uhCkkANCHOR");

        let json = serde_json::to_string(&new).unwrap();
        let old_json: OldProjectionInventoryEntry =
            serde_json::from_str(&json).expect("old struct tolerates the unknown keys over JSON");
        assert_eq!(old_json.dht_anchor_hash, "uhCkkANCHOR");
    }

    #[test]
    fn declared_head_round_trips_when_present() {
        let entry = ProjectionInventoryEntry {
            id: "elohim-host-landing".into(),
            dht_anchor_hash: "uhCkkANCHOR".into(),
            declared_head_action_hash: Some("uhCkkDECLARED".into()),
            declared_head_at: Some(-42),
        };
        let bytes = rmp_serde::to_vec_named(&entry).unwrap();
        let back: ProjectionInventoryEntry = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(back, entry);
    }

    #[test]
    fn content_head_record_payload_round_trips_and_tolerates_absence() {
        // Honest absence is a well-formed answer, not a transport error.
        let absent = ContentHeadRecordPayload {
            content_id: "elohim-host-landing".into(),
            head_action_hash: None,
            declared_at: None,
            record: None,
            record_absent_reason: None,
        };
        let bytes = rmp_serde::to_vec_named(&absent).unwrap();
        let back: ContentHeadRecordPayload = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(back, absent);

        let present = ContentHeadRecordPayload {
            content_id: "elohim-host-landing".into(),
            head_action_hash: Some("uhCkkDECLARED".into()),
            declared_at: Some(1_753_000_000_000_000),
            record: Some("YmFzZTY0".into()),
            record_absent_reason: None,
        };
        let json = serde_json::to_value(&present).unwrap();
        assert!(
            json.get("headActionHash").is_some(),
            "payload must be camelCase on the wire"
        );
        let back: ContentHeadRecordPayload = serde_json::from_value(json).unwrap();
        assert_eq!(back, present);
    }

    /// The pre-field shape of the head-record payload — a stand-in for an OLD
    /// (not-yet-updated) peer's struct during a rolling deploy. No
    /// `record_absent_reason`, and NOT `deny_unknown_fields`, exactly like the
    /// real struct was yesterday.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    struct OldContentHeadRecordPayload {
        content_id: String,
        #[serde(default)]
        head_action_hash: Option<String>,
        #[serde(default)]
        declared_at: Option<i64>,
        #[serde(default)]
        record: Option<String>,
    }

    /// C10 WIRE COMPATIBILITY, both directions, over BOTH encodings the payload
    /// actually travels through: MessagePack `to_vec_named` (the view-federation
    /// codec's frame) and `serde_json::Value` (what `head_record_client` decodes
    /// from `ViewSlice.payload`).
    ///
    /// This is the test that licenses shipping the field without a protocol
    /// version bump. A rolling deploy puts old and new responders/requesters in
    /// the same fleet for hours, and the failure mode of getting it wrong is not
    /// a bad reason label — it is `head_record_client` seeing an undecodable
    /// payload, answering `Unreachable`, and stopping adoption against every
    /// peer on the other side of the version line.
    #[test]
    fn record_absent_reason_is_additive_in_both_directions() {
        // NEW responder → OLD requester: the unknown key must be IGNORED, and
        // every field the old struct does know must survive intact.
        let new = ContentHeadRecordPayload {
            content_id: "elohim-host-landing".into(),
            head_action_hash: Some("uhCkkDECLARED".into()),
            declared_at: Some(1_753_000_000_000_000),
            record: None,
            record_absent_reason: Some("no_record".into()),
        };
        let msgpack = rmp_serde::to_vec_named(&new).unwrap();
        let old: OldContentHeadRecordPayload = rmp_serde::from_slice(&msgpack)
            .expect("an old peer must still decode a new responder's msgpack frame");
        assert_eq!(old.head_action_hash.as_deref(), Some("uhCkkDECLARED"));
        assert_eq!(old.record, None);

        let json = serde_json::to_value(&new).unwrap();
        let old_json: OldContentHeadRecordPayload = serde_json::from_value(json.clone())
            .expect("an old peer must still decode a new responder's JSON payload");
        assert_eq!(old_json.head_action_hash.as_deref(), Some("uhCkkDECLARED"));
        assert_eq!(
            json.get("recordAbsentReason").and_then(|v| v.as_str()),
            Some("no_record"),
            "the reason must ride the wire camelCase, like every other key"
        );

        // OLD responder → NEW requester: the missing key defaults to None, which
        // downstream classifies as `unknown` and takes the ordinary backoff.
        let old_wire = OldContentHeadRecordPayload {
            content_id: "elohim-host-landing".into(),
            head_action_hash: Some("uhCkkDECLARED".into()),
            declared_at: None,
            record: None,
        };
        let back: ContentHeadRecordPayload =
            rmp_serde::from_slice(&rmp_serde::to_vec_named(&old_wire).unwrap())
                .expect("a new peer must decode an old responder's msgpack frame");
        assert_eq!(back.record_absent_reason, None);
        let back_json: ContentHeadRecordPayload =
            serde_json::from_value(serde_json::to_value(&old_wire).unwrap())
                .expect("a new peer must decode an old responder's JSON payload");
        assert_eq!(back_json.record_absent_reason, None);
    }

    /// A served-bytes answer must encode BYTE-IDENTICALLY to the pre-field
    /// shape: `skip_serializing_if` means the happy path's frame is unchanged,
    /// so nothing keyed on those bytes (dedup, size budgets, a peer's parser)
    /// sees any difference at all.
    #[test]
    fn a_carried_answer_encodes_byte_identically_to_the_pre_field_shape() {
        let carried = ContentHeadRecordPayload {
            content_id: "elohim-host-landing".into(),
            head_action_hash: Some("uhCkkDECLARED".into()),
            declared_at: Some(42),
            record: Some("YmFzZTY0".into()),
            record_absent_reason: None,
        };
        let old = OldContentHeadRecordPayload {
            content_id: "elohim-host-landing".into(),
            head_action_hash: Some("uhCkkDECLARED".into()),
            declared_at: Some(42),
            record: Some("YmFzZTY0".into()),
        };
        assert_eq!(
            rmp_serde::to_vec_named(&carried).unwrap(),
            rmp_serde::to_vec_named(&old).unwrap(),
            "a None reason must not change the wire bytes of a carried answer"
        );
    }

    #[test]
    fn content_head_record_view_kind_is_snake_case_tagged() {
        let kind = ViewKind::ContentHeadRecord {
            content_id: "elohim-host-landing".into(),
        };
        let json = serde_json::to_value(&kind).unwrap();
        assert!(
            json.get("content_head_record").is_some(),
            "ViewKind stays snake_case-tagged: {json}"
        );
        let back: ViewKind = serde_json::from_value(json).unwrap();
        assert_eq!(back, kind);
    }
}

/// Wire-compat tests for T4 (head-plane trust-gradient program plan §3 L2):
/// `ViewFederationRequest::head_corpus_digest` (requester → responder) and
/// `ProjectionInventoryPayload::in_sync` (responder → requester). Same
/// discipline as [`inventory_offset_wire_compat_tests`] above — additive,
/// optional, `skip_serializing_if`-backed, so a rolling deploy mixes old and
/// new peers for hours without either side losing a field it understands.
#[cfg(test)]
mod head_corpus_digest_wire_compat_tests {
    use super::*;

    /// Yesterday's `ViewFederationRequest` shape — no `head_corpus_digest`,
    /// but (unlike [`inventory_offset_wire_compat_tests::OldViewFederationRequest`])
    /// it DOES already carry `inventory_offset`, since that field landed first.
    /// This is the REAL immediately-pre-T4 shape, not the pre-`inventory_offset`
    /// shape two features back.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    struct OldViewFederationRequest {
        view_kind: ViewKind,
        agent_cid: String,
        request_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        inventory_offset: Option<u32>,
    }

    /// Yesterday's `ProjectionInventoryPayload` shape — no `in_sync`.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    struct OldProjectionInventoryPayload {
        table: String,
        #[allow(dead_code)]
        total: usize,
        entries: Vec<ProjectionInventoryEntry>,
    }

    // ------------------------------------------------------------------
    // (a) Round-trip with the field(s) present.
    // ------------------------------------------------------------------

    #[test]
    fn head_corpus_digest_round_trips_when_present() {
        let req = ViewFederationRequest {
            view_kind: ViewKind::ProjectionInventory {
                table: "content".into(),
            },
            agent_cid: "agent".into(),
            request_id: "r".into(),
            inventory_offset: None,
            head_corpus_digest: Some("sha256:deadbeef".into()),
        };
        let bytes = rmp_serde::to_vec_named(&req).unwrap();
        let back: ViewFederationRequest = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(back, req);
    }

    #[test]
    fn in_sync_round_trips_when_present() {
        let payload = ProjectionInventoryPayload {
            table: "content".into(),
            total: 0,
            entries: Vec::new(),
            in_sync: Some(true),
            head_set_snapshot: None,
        };
        let bytes = rmp_serde::to_vec_named(&payload).unwrap();
        let back: ProjectionInventoryPayload = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(back, payload);

        // The payload also rides `ViewSlice.payload` as JSON before the frame
        // is msgpack'd, so the JSON leg must round-trip identically.
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json.get("inSync").and_then(|v| v.as_bool()), Some(true));
        let via_json: ProjectionInventoryPayload = serde_json::from_value(json).unwrap();
        assert_eq!(via_json, payload);
    }

    // ------------------------------------------------------------------
    // (b) Old bytes (without the field) decode on the new struct.
    // ------------------------------------------------------------------

    #[test]
    fn new_peer_decodes_old_request_bytes_defaulting_digest_to_none() {
        // An OLD (post-inventory_offset, pre-T4) peer emits a request with no
        // head_corpus_digest key at all.
        let old = OldViewFederationRequest {
            view_kind: ViewKind::ProjectionInventory {
                table: "content".into(),
            },
            agent_cid: "agent".into(),
            request_id: "r".into(),
            inventory_offset: Some(500),
        };
        let bytes = rmp_serde::to_vec_named(&old).unwrap();
        let new: ViewFederationRequest = rmp_serde::from_slice(&bytes)
            .expect("new struct must decode a request missing head_corpus_digest");
        assert_eq!(
            new.head_corpus_digest, None,
            "missing key defaults to None — the responder skips the digest comparison"
        );
        assert_eq!(new.inventory_offset, Some(500), "sibling field unaffected");
        assert_eq!(new.agent_cid, "agent");
    }

    #[test]
    fn new_peer_decodes_old_inventory_payload_defaulting_in_sync_to_none() {
        // An OLD responder emits a ProjectionInventoryPayload with no in_sync
        // key (it never saw a head_corpus_digest to answer).
        let old = OldProjectionInventoryPayload {
            table: "content".into(),
            total: 0,
            entries: Vec::new(),
        };
        let bytes = rmp_serde::to_vec_named(&old).unwrap();
        let new: ProjectionInventoryPayload = rmp_serde::from_slice(&bytes)
            .expect("new struct must decode a payload missing in_sync");
        assert_eq!(
            new.in_sync, None,
            "missing key defaults to None — no answer to a question never asked"
        );

        let json = serde_json::to_string(&old).unwrap();
        let via_json: ProjectionInventoryPayload =
            serde_json::from_str(&json).expect("JSON leg defaults the missing in_sync too");
        assert_eq!(via_json.in_sync, None);
    }

    // ------------------------------------------------------------------
    // (c) New bytes with `None` decode on the OLD struct shape — proven via
    // byte-identity: a `None` field must add NO wire bytes, so the frame an
    // old peer receives is indistinguishable from yesterday's.
    // ------------------------------------------------------------------

    #[test]
    fn none_head_corpus_digest_is_byte_identical_to_the_pre_field_encoding() {
        let new = ViewFederationRequest {
            view_kind: ViewKind::ProjectionInventory {
                table: "content".into(),
            },
            agent_cid: "agent".into(),
            request_id: "r".into(),
            inventory_offset: Some(0),
            head_corpus_digest: None,
        };
        let old = OldViewFederationRequest {
            view_kind: ViewKind::ProjectionInventory {
                table: "content".into(),
            },
            agent_cid: "agent".into(),
            request_id: "r".into(),
            inventory_offset: Some(0),
        };
        let new_bytes = rmp_serde::to_vec_named(&new).unwrap();
        assert_eq!(
            new_bytes,
            rmp_serde::to_vec_named(&old).unwrap(),
            "None head_corpus_digest must serialize byte-identically to the pre-field \
             struct — the canonical_bytes dedup key must not change for callers that \
             never set the field"
        );
        // And an OLD peer's struct must still decode the new (unchanged) bytes.
        let decoded: OldViewFederationRequest = rmp_serde::from_slice(&new_bytes)
            .expect("an old peer must still decode a None-digest request");
        assert_eq!(decoded, old);
    }

    #[test]
    fn none_in_sync_is_byte_identical_to_the_pre_field_encoding() {
        let new = ProjectionInventoryPayload {
            table: "content".into(),
            total: 2,
            entries: vec![ProjectionInventoryEntry {
                id: "epr:a".into(),
                dht_anchor_hash: "anc-a".into(),
                declared_head_action_hash: None,
                declared_head_at: None,
            }],
            in_sync: None,
            head_set_snapshot: None,
        };
        let old = OldProjectionInventoryPayload {
            table: "content".into(),
            total: 2,
            entries: vec![ProjectionInventoryEntry {
                id: "epr:a".into(),
                dht_anchor_hash: "anc-a".into(),
                declared_head_action_hash: None,
                declared_head_at: None,
            }],
        };
        let new_bytes = rmp_serde::to_vec_named(&new).unwrap();
        assert_eq!(
            new_bytes,
            rmp_serde::to_vec_named(&old).unwrap(),
            "None in_sync must serialize byte-identically to the pre-field struct — a \
             request that never carried a digest must produce the exact frame it did \
             before this field existed"
        );
        let decoded: OldProjectionInventoryPayload = rmp_serde::from_slice(&new_bytes)
            .expect("an old peer must still decode a None-in_sync payload");
        assert_eq!(decoded, old);
    }
}

/// Wire-compat tests for T8 (head-plane trust-gradient program plan §3 L5):
/// `ProjectionInventoryPayload::head_set_snapshot`. Same three-legged
/// discipline as the two additive fields above — round-trip present, old bytes
/// on the new struct, and byte-identity when `None` (which is what proves the
/// new bytes decode on an old peer).
#[cfg(test)]
mod head_set_snapshot_wire_compat_tests {
    use super::*;

    /// Yesterday's `ProjectionInventoryPayload` — post-`in_sync` (T4), which
    /// is the REAL immediately-pre-T8 shape, not the pre-`in_sync` one a
    /// feature back.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    struct OldProjectionInventoryPayload {
        table: String,
        total: usize,
        entries: Vec<ProjectionInventoryEntry>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        in_sync: Option<bool>,
    }

    fn entry() -> ProjectionInventoryEntry {
        ProjectionInventoryEntry {
            id: "epr:a".into(),
            dht_anchor_hash: "anc-a".into(),
            declared_head_action_hash: None,
            declared_head_at: None,
        }
    }

    #[test]
    fn head_set_snapshot_round_trips_when_present() {
        let payload = ProjectionInventoryPayload {
            table: "content".into(),
            total: 1,
            entries: vec![entry()],
            in_sync: Some(true),
            // Opaque to this crate by design: base64(dag-cbor canonical bytes).
            head_set_snapshot: Some("o2Vwcm9vZg".into()),
        };
        let bytes = rmp_serde::to_vec_named(&payload).unwrap();
        let back: ProjectionInventoryPayload = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(back, payload);

        // The payload also rides `ViewSlice.payload` as JSON before the frame
        // is msgpack'd, so the JSON leg must round-trip identically.
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(
            json.get("headSetSnapshot").and_then(|v| v.as_str()),
            Some("o2Vwcm9vZg")
        );
        let via_json: ProjectionInventoryPayload = serde_json::from_value(json).unwrap();
        assert_eq!(via_json, payload);
    }

    #[test]
    fn new_peer_decodes_old_payload_bytes_defaulting_the_snapshot_to_none() {
        let old = OldProjectionInventoryPayload {
            table: "content".into(),
            total: 1,
            entries: vec![entry()],
            in_sync: Some(false),
        };
        let bytes = rmp_serde::to_vec_named(&old).unwrap();
        let new: ProjectionInventoryPayload = rmp_serde::from_slice(&bytes)
            .expect("new struct must decode a payload missing head_set_snapshot");
        assert_eq!(
            new.head_set_snapshot, None,
            "missing key defaults to None — nothing to verify, so no verdict"
        );
        assert_eq!(new.in_sync, Some(false), "sibling field unaffected");

        let json = serde_json::to_string(&old).unwrap();
        let via_json: ProjectionInventoryPayload = serde_json::from_str(&json)
            .expect("JSON leg defaults the missing head_set_snapshot too");
        assert_eq!(via_json.head_set_snapshot, None);
    }

    #[test]
    fn none_head_set_snapshot_is_byte_identical_to_the_pre_field_encoding() {
        let new = ProjectionInventoryPayload {
            table: "content".into(),
            total: 1,
            entries: vec![entry()],
            in_sync: Some(true),
            head_set_snapshot: None,
        };
        let old = OldProjectionInventoryPayload {
            table: "content".into(),
            total: 1,
            entries: vec![entry()],
            in_sync: Some(true),
        };
        let new_bytes = rmp_serde::to_vec_named(&new).unwrap();
        assert_eq!(
            new_bytes,
            rmp_serde::to_vec_named(&old).unwrap(),
            "None head_set_snapshot must serialize byte-identically to the pre-field \
             struct — every responder that never attaches one emits yesterday's frame, \
             so a rolling deploy is invisible on the wire"
        );
        let decoded: OldProjectionInventoryPayload = rmp_serde::from_slice(&new_bytes)
            .expect("an old peer must still decode a None-snapshot payload");
        assert_eq!(decoded, old);
    }

    #[test]
    fn old_peer_ignores_a_carried_snapshot_it_does_not_understand() {
        // A NEW responder that HAS flipped the sender emits the extra key; a
        // pre-T8 peer must degrade to yesterday's behavior, not fail decode.
        let new = ProjectionInventoryPayload {
            table: "content".into(),
            total: 1,
            entries: vec![entry()],
            in_sync: None,
            head_set_snapshot: Some("o2Vwcm9vZg".into()),
        };
        let bytes = rmp_serde::to_vec_named(&new).unwrap();
        let old: OldProjectionInventoryPayload = rmp_serde::from_slice(&bytes)
            .expect("old struct tolerates the unknown head_set_snapshot key");
        assert_eq!(old.entries, vec![entry()]);
        assert_eq!(old.total, 1);
    }
}
