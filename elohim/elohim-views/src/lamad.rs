//! lamad view types — migrated from elohim-storage/src/views.rs (VIEWS.T2).

#[allow(unused_imports)]
use crate::infrastructure::*;
use crate::shared::*;
#[allow(unused_imports)]
use crate::shefa::*;
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use serde_json::Value;
use ts_rs::TS;

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
    /// Content-addressed hash of the Angular SSR *server* bundle (wire:
    /// `serverBlobHash`). Mirrors `blob_hash` (the *browser* bundle) so the one
    /// EPR node carries its full SSR nature. `None` ⇒ SSR not materialized (CSR
    /// fallback). Deploy-PATCH populated; absent is the normal pre-deploy state.
    pub server_blob_hash: Option<String>,
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
    /// REQ-F10 trust legibility label — the "HTTP vs HTTPS" address-bar signal
    /// for this content row. Computed from the row's provenance markers:
    /// `"notarized"` when `dht_anchor_hash` is set (green padlock — DHT-notarized),
    /// else `"published"` when `p2p_published_at` is set (peer-attested), else
    /// `"unconfirmed"` (amber — CRDT-converged-only or all-null; served like HTTP
    /// "Not secure"). NEVER derive authority/attribution from this field — an
    /// "unconfirmed" row is functional-but-not-notarized.
    ///
    /// LIVENESS: `dht_anchor_hash` being SET is not sufficient for
    /// `"notarized"` — the anchor must also not be known-dead. See
    /// [`Self::dht_anchor_state`].
    pub trust: String,
    /// Liveness of [`Self::dht_anchor_hash`] against the serving node's CURRENT
    /// conductor incarnation: `"live"` | `"dead"` | `"unverified"`.
    ///
    /// `dht_anchor_hash` answers "was this ever authored?"; this answers "can
    /// anybody still produce the action behind it?". After a conductor re-key
    /// (chain and keystore replaced, the storage projection kept) the hash
    /// survives but the chain that signed it does not — and before this field
    /// existed the row went on claiming `trust: "notarized"`, the strongest
    /// provenance claim this system makes, about a signature no living chain
    /// could present (2026-08-21 RCA).
    ///
    /// `"dead"` means the serving node's own conductor answered ABSENT for this
    /// id; such a row is queued for re-authoring under the live key and its
    /// `trust` falls back to whatever non-anchor evidence it still has.
    /// `"unverified"` is the honest default — not yet asked, or the ask could
    /// not be put. Absence of an answer is never evidence of death, so an
    /// unverified row is treated exactly like a live one for `trust`.
    ///
    /// OPTIONAL on the wire: absent means a serving node that predates the
    /// liveness projection. Consumers must treat `None` as "unverified", never
    /// as "dead".
    #[ts(optional)]
    pub dht_anchor_state: Option<String>,
}

/// The notary-declared HEAD of a content id's version DAG (HEAD-election, Plan
/// C3 / notary-authority Leg 3 — the HTTP surface over `content_store`'s
/// resolve/declare coordinators).
///
/// Source of truth: DHT (Notarized HEAD election, Category A). Projected from
/// the `content` row's notary markers; the surface exists only for a row that
/// carries a notary answer (a declared head OR a DHT anchor) — a row with
/// neither has no HEAD and the handler 404s rather than returning this view.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentHeadView {
    /// The content id whose HEAD this is (wire: `contentId`).
    pub content_id: String,
    /// The action hash the notary holds as this id's current HEAD. Prefers the
    /// explicitly-declared HEAD; falls back to the DHT anchor when no explicit
    /// declaration has been stamped (wire: `headActionHash`).
    pub head_action_hash: String,
    /// `true` iff an explicit `declared_head_action_hash` was set (an author
    /// moved the HEAD via the declare authority); `false` when the answer is the
    /// DHT-anchor fallback (the single-author implicit head).
    pub declared: bool,
    /// The DHT anchor for the resolved row, when notarized. `None` when the HEAD
    /// answer rests only on a declared head with no anchor yet.
    pub dht_anchor_hash: Option<String>,
    /// REQ-F10 trust legibility label — same vocabulary as `ContentView.trust`
    /// (`notarized` | `published` | `unconfirmed`). Never an authority source.
    pub trust: String,
    /// The serving blob hash of the resolved row, if any (browser bundle).
    pub blob_hash: Option<String>,
    /// When the resolved row was last written (mirrors `ContentView.updatedAt`).
    pub updated_at: Option<String>,
}

/// One node in a content relationship graph.
///
/// Wire shape for `content-graph.schema.json` `$defs/ContentGraphNode`. Carries
/// the resolver's `inferenceSource` (explicit | path | tag | semantic | system)
/// and `depth` so the boundary is provenance-honest. `children` is recursive
/// (depth + 1). Built from `ResolvedEdge` via a `From` shim in
/// `elohim-storage/src/views.rs`.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentGraphNodeView {
    pub content_id: String,
    pub relationship_type: String,
    pub confidence: f64,
    pub inference_source: String,
    pub depth: u32,
    pub children: Vec<ContentGraphNodeView>,
}

/// Content relationship graph rooted at a given content id.
///
/// Wire shape for `content-graph.schema.json`. A multi-provenance composite
/// read projection (never persisted). Built from `ResolvedNeighborhood` via a
/// `From` shim in `elohim-storage/src/views.rs`.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentGraphView {
    pub root_id: String,
    pub related: Vec<ContentGraphNodeView>,
    pub total_nodes: usize,
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
    /// Content-derived provenance anchor, set at INGEST (seed/import) so the
    /// row satisfies the `require_provenance` read gate (`dht_anchor_hash IS
    /// NOT NULL OR p2p_published_at IS NOT NULL`) on hub-optional / peer-starved
    /// stacks where the libp2p publish drain never runs. This is the HONEST
    /// alternative to stamping `p2pPublishedAt` (which would assert a DHT
    /// publication that never happened): the value is the content's
    /// content-address, superseded by the real ActionHash when a
    /// `ContentCommitted` notarization later runs. Optional — omit on the
    /// peered path where the drain stamps `p2pPublishedAt`.
    #[serde(default)]
    #[ts(optional)]
    pub dht_anchor_hash: Option<String>,
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
    /// Content-addressed SHA256 of the bundle/asset this row projects.
    /// Set at deploy-time by Jenkinsfile:stageSpaBlob — see
    /// genesis/docs/superpowers/plans/2026-05-23-spa-blob-deploy-drift.md.
    /// Deliberately optional: PATCH callers MAY set this without touching
    /// any other field.
    #[serde(default)]
    pub blob_hash: Option<String>,
    /// Content-addressed SHA256 of the Angular SSR *server* bundle this row
    /// projects (wire: `serverBlobHash`). Set at deploy-time by the Jenkins SSR
    /// PATCH (mirrors `blob_hash`, the browser bundle). Deliberately optional:
    /// PATCH callers MAY set this without touching any other field — and a
    /// `serverBlobHash`-only PATCH must NOT clobber `blob_hash` or other fields.
    /// Unlike `blob_hash`/`reach`, this is a deploy-projection artifact, not a
    /// DNA-notarized content-entry field, so it takes the diesel-direct PATCH
    /// path (see `patch_needs_conductor`), like `p2p_published_at`.
    #[serde(default)]
    pub server_blob_hash: Option<String>,
    /// RFC-3339 timestamp marking when this row was published to the libp2p
    /// Kad DHT. Stamping this satisfies the `require_provenance` read gate
    /// (content_diesel: `dht_anchor_hash IS NOT NULL OR p2p_published_at IS
    /// NOT NULL`). The drain loop is the canonical writer in a peered stack;
    /// the genesis seeder stamps it directly so household/local stacks with
    /// no DHT peers still pass the gate. Optional: PATCH callers MAY set this
    /// without touching any other field.
    #[serde(default)]
    pub p2p_published_at: Option<String>,
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

// ContentAttestationView + CreateAttestationInputView retired (attestation-consolidation
// Phase-2a frontend cleanup): the legacy per-type content-attestation read/write surface
// was removed; the unified AttestationView (elohim/sdk/schemas/v1/views/attestation-view.schema.json)
// is the canonical content-quality read. These ts-rs structs had no Rust handler and no live
// TS consumer after the trust-badge repoint onto the unified surface.
//
// NOTE (rust-architect follow-up, not in this frontend slice's scope): the schema-codegen
// input `elohim/sdk/schemas/v1/inputs/create-attestation-input.schema.json` (a SEPARATE
// pipeline from these ts-rs structs) is now orphaned of a matching Rust struct. It lives in
// the protocol-truth schema layer (operator/rust-architect territory) — flagged here for a
// follow-up decision (retire vs. realign to the unified attestation input), not deleted here.

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

/// One (action, summed-value) entry in a contributor-reflexive recognition rollup.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RecognitionByAction {
    pub action: String,
    pub value: f64,
}

/// The contributor-reflexive facing — "how the network sees a contributor". Assembles the
/// engagement-accrued scalars (off the presence row) with the network-routed recognition folded
/// from `economic_events` and the steward-role rollup from active `stewardship_allocations`.
/// Operational Category C (read projection; no DHT entry). Spec: Wave 2 of
/// 2026-06-21-contributor-presence-bootstrap-whoswho-design.md.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContributorReflexiveView {
    pub presence_id: String,
    pub display_name: String,
    pub presence_state: String,
    // Engagement-accrued scalars — assembled straight off the presence row (NOT re-folded).
    pub recognition_score: f64,
    pub citation_count: i32,
    pub affinity_total: f64,
    pub unique_engagers: i32,
    // Network-routed recognition — folded from this presence's economic_events.
    pub total_recognition_value: f64,
    pub recognition_by_action: Vec<RecognitionByAction>,
    pub distinct_content_recognized: i32,
    /// PARTIAL: only commons flows attributed to this presence's events (settlements carry
    /// no contributor_presence_id). Full commons accounting is a separate query (deferred).
    pub commons_flow_value: f64,
    // Steward role — folded from this presence's active stewardship_allocations.
    pub steward_allocation_count: i32,
    pub steward_recognition_accumulated: f64,
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

// ---------------------------------------------------------------------------
// M-AGGR-3: ContentEngagementStats projection (Category C operational)
// Source of truth: derived projection of EconomicEvent stream filtered by
// content_id AND lamadEventType IN ('content-view', 'content-complete').
// Computed on Signal::EconomicEventCreated; reconstructable from the
// underlying EconomicEvent entries in the elohim DNA content_store zome.
// Schema: epr:schema:view:content-engagement-stats
// ---------------------------------------------------------------------------

/// Materialized engagement statistics for a single content item.
///
/// Derived by grouping and counting EconomicEvent entries whose
/// `content_id` matches and `lamadEventType` is one of the two
/// engagement event kinds ('content-view', 'content-complete').
/// When no events exist, all counters are zero and `completionRate`
/// is 0.0.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentEngagementStatsView {
    /// Identifier of the content item this projection covers.
    pub content_id: String,
    /// Total count of EconomicEvents with lamadEventType='content-view'.
    pub views: i64,
    /// Total count of EconomicEvents with lamadEventType='content-complete'.
    pub completions: i64,
    /// Count of distinct provider (agent) values across content-view events.
    pub unique_viewers: i64,
    /// Ratio of completions to views in [0.0, 1.0]. Zero when views == 0.
    pub completion_rate: f64,
    /// ISO-8601 timestamp when this projection was last computed.
    pub computed_at: String,
}
