//! epr view types — migrated from elohim-storage/src/views.rs (VIEWS.T2).

#[allow(unused_imports)]
use crate::imagodei::*;
#[allow(unused_imports)]
use crate::infrastructure::*;
#[allow(unused_imports)]
use crate::lamad::*;
use crate::shared::*;
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use serde_json::Value;
use ts_rs::TS;

/// Coupling legs for an EPR envelope. All legs are optional at the wire level;
/// kind-specific requirements are enforced by the validator (EprService::ingest).
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct EprCouplingView {
    pub knowledge: Option<String>,
    pub value: Option<String>,
    pub governance: Option<String>,
}

/// Detached Ed25519 proof on the EPR canonical bytes.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct EprSignatureView {
    /// CIDv1 base32 of the signer's Agent EPR.
    pub signer: String,
    pub algorithm: String,
    /// Hex-encoded 64 bytes (128 hex chars).
    pub signature: String,
}

/// Wire-string projection of an EPR Envelope for HTTP consumers.
/// CIDs are CIDv1 base32 strings. `issuedAt` is an RFC3339 date-time string.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct EprEnvelopeView {
    pub cid: String,
    pub kind: String,
    pub schema_ref: String,
    pub schema_key: String,
    pub reach: String,
    pub coupling: EprCouplingView,
    pub claims: Vec<String>,
    pub supersedes: Option<String>,
    pub superseded_by: Option<String>,
    /// RFC3339 date-time string — passed through from storage as-is.
    pub issued_at: String,
    pub proof: EprSignatureView,
}

/// Full HTTP view for GET /api/v1/epr/:cid. Payload is hex-encoded.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct EprView {
    pub envelope: EprEnvelopeView,
    /// Hex-encoded payload bytes.
    pub payload: String,
    /// Optional hex-encoded canonical bytes (included when ?includeCanonical=true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_bytes: Option<String>,
}

/// Error detail for a failed verify stage.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct EprVerifyErrorView {
    pub stage: String,
    pub message: String,
}

/// Verify response for GET /api/v1/epr/:cid/verify.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct EprVerifyView {
    pub cid: String,
    pub verified: bool,
    pub stages_run: Vec<String>,
    pub stages_skipped: Vec<String>,
    pub error: Option<EprVerifyErrorView>,
}

/// Paged list response for GET /api/v1/epr. Items are envelope-only (no payload).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct EprListView {
    pub items: Vec<EprEnvelopeView>,
    /// Opaque cursor for next page; null when exhausted.
    pub next_cursor: Option<String>,
}

/// Input body for POST /api/v1/epr.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct EprPublishInput {
    pub envelope: EprEnvelopeView,
    /// Hex-encoded payload bytes.
    pub payload: String,
    /// Optional republish-epr EconomicEvent. Required when the publish is a
    /// substrate-correct re-deploy; absent for direct envelope writes.
    #[serde(default)]
    pub event: Option<RepublishEprEventView>,
}

/// Optional EconomicEvent payload accompanying a substrate-correct republish.
///
/// Per Z.D spec §2 (genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md):
/// when CI re-deploys an EPR bundle, it emits a `republish-epr` EconomicEvent
/// bounded by a `delegates-compute` Commitment. The substrate validator walks
/// `bounded_by` → Commitment and rejects out-of-bounds events.
///
/// Field shape matches `elohim/sdk/schemas/v1/economic-events/republish-epr.schema.json`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RepublishEprEventView {
    pub action: String,
    pub performer: String,
    pub bounded_by: String,
    pub target: String,
    pub supersedes: Option<String>,
    /// Inline payload object with {blob_cid, epr_kind, reach, bundle_path?}.
    pub payload: JsonVal,
    pub signed_at: String,
}

/// Wire view for the providers endpoint.
///
/// Source of truth: peer providers advertising that they hold the atom at the
/// given cid. Phase 2a returns ["local"] when held locally, [] otherwise.
/// Phase 2c extends with Kad DHT provider records.
/// Category C — operational (DHT query, reconstructed per request).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct EprProvidersView {
    /// CIDv1 base32 of the queried atom.
    pub cid: String,
    /// Peer identifiers — "local" for this node, or libp2p PeerId for remote peers.
    pub providers: Vec<String>,
}

/// A lightweight reference to a related EPR atom, used in the nav-context
/// projection. Label and resilience tier are best-effort — populated when the
/// referenced atom is locally held; absent otherwise.
///
/// Category C — wire view only, no DHT entry type, no new table.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct EprNavRef {
    /// CIDv1 base32 of the referenced atom.
    pub cid: String,
    /// Short human-readable label, populated when the atom is locally held.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Resilience tier string derived from the atom's `reach` field.
    /// One of "high" | "medium" | "low" | "unknown". Absent when the atom is
    /// not locally held.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resilience_tier: Option<String>,
}

/// Nav-context projection for GET /api/v1/epr/{cid}/nav-context.
///
/// Source of truth: existing `epr_atoms`, `epr_coupling`, `epr_supersedence`,
/// and `epr_claims` tables (read-only projection). Zero new DHT entry types,
/// zero new tables, zero migrations -- Category C per p2p-design-gate.
///
/// - `prev`: atom this one supersedes (the atom it replaces).
/// - `next`: atom that supersedes this one (what replaced it).
/// - `partOf`: coupling legs that point inward -- atoms that couple to this one.
/// - `related`: outbound coupling legs from this atom (knowledge / value /
///   governance targets) plus claim CIDs.
/// - `derivedFrom`: list of relationship kinds that contributed entries
///   (e.g. ["supersedence", "coupling", "claims"]) -- for client-side
///   disclosure of which relationship types were consulted.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct EprNavContextView {
    /// CIDv1 base32 of the focal atom.
    pub cid: String,
    /// Predecessor: atom this one supersedes (the one it replaced).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev: Option<EprNavRef>,
    /// Successor: atom that superseded this one (what replaced it).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<EprNavRef>,
    /// Atoms that include this atom as a coupling target (reverse coupling).
    #[serde(default)]
    pub part_of: Vec<EprNavRef>,
    /// Outbound coupling targets plus claim CIDs from this atom.
    #[serde(default)]
    pub related: Vec<EprNavRef>,
    /// Relationship kinds that contributed entries to this view.
    #[serde(default)]
    pub derived_from: Vec<String>,
}

/// Raw inspector projection for GET /api/v1/epr/{cid}/raw — the EPR-content
/// facing's first-person fold over its materialized leg-neighborhood. Where
/// `/nav-context` is the navigational projection (labelled refs, prev/next),
/// `/raw` is the inspector: the bare folds with no enrichment.
///
/// Source of truth: existing `epr_coupling` table (read-only projection of
/// notarized EPR atoms). Zero new DHT entry types, zero new tables, zero
/// migrations — Category C per p2p-design-gate.
///
/// Folds (pure, DB-free): `elohim_facings::folds::epr_content::{fold_coupling_legs,
/// neighborhood_degree, reverse_part_of}`. Charter:
/// `genesis/docs/superpowers/specs/2026-06-19-epr-content-perspective-facing-lens-design.md` §6 Slice 2.
///
/// Honesty note: `coupling` carries the three modelled legs (knowledge / value /
/// governance). The unmodelled `process` leg is dropped by the fold — `/raw` v1
/// does NOT advertise 4-of-4 leg coverage (charter Slice 1 owns `process`).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct EprRawView {
    /// CIDv1 base32 of the focal atom.
    pub cid: String,
    /// Forward coupling legs (knowledge / value / governance) pivoted from
    /// the focal atom's `epr_coupling` rows.
    pub coupling: EprCouplingView,
    /// Count of DISTINCT forward coupling targets across all legs (the content's
    /// immediate dataplane reach — a multi-leg coupling to one target counts once).
    pub neighborhood_degree: i32,
    /// DISTINCT source atoms that couple TO this atom (inbound "part-of" set),
    /// sorted for deterministic wire order. Each entry is a CIDv1 base32 string.
    pub reverse_part_of: Vec<String>,
}
