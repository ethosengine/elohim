//! epr view types — migrated from elohim-storage/src/views.rs (VIEWS.T2).

use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use serde_json::Value;
use ts_rs::TS;
use crate::shared::*;
#[allow(unused_imports)]
use crate::imagodei::*;
#[allow(unused_imports)]
use crate::infrastructure::*;
#[allow(unused_imports)]
use crate::lamad::*;

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
