//! Thin facade for HTTP write handlers to call local conductor zome functions.
//!
//! Per `elohim/holochain/dna/CLAUDE.md` gospel-tier guidance: "Never write to
//! storage directly for notarized types (legacy code may still do this —
//! migrate toward conductor-first)." This module centralizes the in-process
//! HcClient zome call so the migration happens once. Future Category-A
//! migrations (relationships, agreements, economic_events) follow the same
//! pattern.
//!
//! ## Flow
//!
//! ```text
//! HTTP POST /api/v1/commitments
//!   → ReaCommitmentService::create
//!   → conductor_writes::call_create_rea_commitment   (← you are here)
//!   → HcClient on `lamad` role → content_store::create_rea_commitment zome fn
//!   → DHT entry created
//!   → post_commit emits ProjectionSignal::ReaCommitmentCommitted (lib.rs:10768)
//!   → rea_projection::project_signal (this process)
//!   → rea_commitments::upsert_with_anchor (with dht_anchor_hash from ActionHash)
//!   → SQL row populated; future GETs return the projected View
//!   (in parallel: Holochain DHT gossip propagates entry to peer B/C/...,
//!    each peer's signal subscriber projects to its own SQL with the same
//!    dht_anchor_hash — solving the cross-peer replication gap that produced
//!    /lamad → 404 and the PLACEHOLDER_REPLACED_BY_SEED_SCRIPT content header
//!    on alpha as of 2026-05-26)
//! ```
//!
//! ## Wire shape
//!
//! The facade takes `shefa_types::CreateReaCommitmentInput` — the canonical
//! coordinator-input wire shape (re-exported by content_store). The
//! storage-layer's diesel-side `db::rea_commitments::CreateReaCommitmentInput`
//! is a *different* shape (Option<String> id, Option<String> in_scope_of,
//! includes medium_of_exchange_id) and conversion happens in the service
//! layer (see ReaCommitmentService::create in the next task).
//!
//! ## Plan reference
//!
//! `genesis/docs/superpowers/plans/2026-05-26-substrate-rea-replication-fix.md`
//! (Addendum 1 carries substrate-name corrections grounding this module.)

use std::sync::Arc;

use crate::error::StorageError;
use crate::hc_client::HcClient;
use crate::rea_projection::ContentEntry;
use crate::signals::HoloHashB64;

/// Zome name hosting REA commitment + content coordinator functions.
/// Lives in the `lamad` role of the elohim hApp (see
/// `elohim/holochain/dna/elohim/workdir/happ.yaml:24`).
const ZOME_NAME: &str = "content_store";

/// Zome name hosting the qahal Collective/Membership coordinator functions.
/// Lives in the `imagodei` role of the elohim hApp — a DIFFERENT cell from
/// [`ZOME_NAME`], so these calls MUST route through
/// [`HcClient::call_zome_imagodei`] (see that method's doc for the
/// ZomeNotFound/signing hazard).
const IMAGODEI_ZOME: &str = "imagodei";

/// Prefix the imagodei coordinator stamps onto a Collective's canonical CID
/// (`qahal_coordinator::action_hash_to_cid` — `format!("collective:{hash}")`).
pub const COLLECTIVE_CID_PREFIX: &str = "collective:";

/// Wire mirror of `imagodei_integrity::qahal::Collective` — the DHT entry body
/// as it arrives inside a `Record`'s `Entry::App` msgpack bytes.
///
/// Deliberately a LOCAL mirror rather than a dependency on the integrity crate:
/// elohim-storage does not link the DNA crates, and the mirror makes the wire
/// contract explicit at the seam (the same convention
/// `holochain_humans_replayer::MembershipWire` uses).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CollectiveWire {
    pub founder_agent_cid: String,
    pub charter: String,
    pub display_name: String,
    #[serde(default)]
    pub created_at_block_height: u64,
    #[serde(default)]
    pub salt: String,
    #[serde(default)]
    pub anchor_agreement_cid: Option<String>,
}

/// Resolve a `collective:{action_hash}` CID against the OWN conductor's
/// imagodei cell (`get_collective_by_action`).
///
/// This is the collectives arm's analogue of
/// [`call_resolve_content_head`]: the reconcile leg learns WHICH cids exist
/// from peers (discovery), then reads the entry BODY exclusively from its own
/// conductor's DHT view. On a full-arc fleet every conductor holds the
/// authoring agent's `Collective` entry, so a non-authoring peer can answer for
/// itself — no peer bytes are ever written into the projection.
///
/// `Ok(None)` means the own conductor cannot see the entry (not yet gossiped in,
/// or a foreign-DHT cid) — retried on the NEXT sweep, never immediately.
/// A malformed cid is an `InvalidInput` error, not a miss: it can never resolve,
/// so re-attempting it every sweep would burn a conductor round-trip forever.
pub async fn get_collective_by_cid(
    hc: &Arc<HcClient>,
    collective_cid: &str,
) -> Result<Option<CollectiveWire>, StorageError> {
    let action_hash = decode_collective_cid(collective_cid)?;
    // HoloHash serializes as msgpack `bin` (holo_hash::ser — `serialize_bytes`
    // over the raw 39 bytes), which is exactly what the coordinator's
    // `ActionHash` parameter deserializes.
    let payload = rmp_serde::to_vec_named(&action_hash).map_err(|e| {
        StorageError::Internal(format!(
            "conductor_writes: encode ActionHash for get_collective_by_action: {e}"
        ))
    })?;
    let bytes = hc
        .call_zome_imagodei(IMAGODEI_ZOME, "get_collective_by_action", payload)
        .await?;
    // Decode through the TYPED `Record` deserializer — a `serde_json::Value`
    // pre-pass or a hand-rolled mirror would drop the holo_hash fields inside
    // `SignedActionHashed` (the msgpack raw-bytes decode class).
    let record: Option<holochain_types::prelude::Record> =
        rmp_serde::from_slice(&bytes).map_err(|e| {
            StorageError::Serialization(format!(
                "conductor_writes: decode Option<Record> from get_collective_by_action: {e}"
            ))
        })?;
    let Some(record) = record else {
        return Ok(None);
    };
    // A record whose entry is absent (a delete/tombstone) is not a Collective.
    let Some(holochain_types::prelude::Entry::App(eb)) = record.entry.as_option() else {
        return Ok(None);
    };
    let collective: CollectiveWire = rmp_serde::from_slice(eb.bytes()).map_err(|e| {
        StorageError::Serialization(format!(
            "conductor_writes: decode Collective entry for {collective_cid}: {e}"
        ))
    })?;
    Ok(Some(collective))
}

/// Decode a `collective:{HoloHash-display}` CID back to its `ActionHash`.
///
/// Pure + total: every failure mode is a legible `InvalidInput`. Exposed for the
/// reconcile arm's up-front validation (a cid it cannot decode is never enqueued
/// as a gap).
pub fn decode_collective_cid(
    collective_cid: &str,
) -> Result<holochain_types::prelude::ActionHash, StorageError> {
    let raw = collective_cid
        .strip_prefix(COLLECTIVE_CID_PREFIX)
        .ok_or_else(|| {
            StorageError::InvalidInput(format!(
                "collective CID must start with '{COLLECTIVE_CID_PREFIX}'; got: {collective_cid}"
            ))
        })?;
    // PANIC GUARD — load-bearing, not defensive noise. `holo_hash::encode::
    // holo_hash_decode` opens with `&s[..1]`, which PANICS on an empty string
    // (every other malformation is length-checked and returns Err). This value
    // arrives from a PEER's advertised inventory, so an empty suffix
    // (`"collective:"`) would abort the whole reconcile task — the
    // one-poisoned-row class. Reject it here, before the decoder sees it.
    if !raw.starts_with('u') {
        return Err(StorageError::InvalidInput(format!(
            "invalid collective CID '{collective_cid}': hash must be a 'u'-prefixed HoloHash"
        )));
    }
    holochain_types::prelude::ActionHash::try_from(raw).map_err(|e| {
        StorageError::InvalidInput(format!("invalid collective CID '{collective_cid}': {e}"))
    })
}

/// Round-trip `create_rea_commitment` through the local conductor.
///
/// Returns raw MessagePack bytes encoding `shefa_types::ReaCommitmentOutput`.
/// Caller decodes if it needs the ActionHash directly; the projection to local
/// SQL happens asynchronously via the post-commit handler at
/// `rea_projection.rs:148`, so callers that need to read the projected row
/// poll `rea_commitments::get_commitment(id)` after this call returns.
pub async fn call_create_rea_commitment(
    hc: &Arc<HcClient>,
    input: &shefa_types::CreateReaCommitmentInput,
) -> Result<Vec<u8>, StorageError> {
    let payload = rmp_serde::to_vec_named(input).map_err(|e| {
        StorageError::Internal(format!(
            "conductor_writes: encode CreateReaCommitmentInput: {e}"
        ))
    })?;
    hc.call_zome(ZOME_NAME, "create_rea_commitment", payload)
        .await
}

/// Emit a REA EconomicEvent through the conductor (vs the diesel-direct
/// `economic_events::record_event`, which writes no DHT anchor). The DNA
/// coordinator `create_rea_economic_event` (content_store/src/lib.rs:12124)
/// resolves `input.fulfills` (commitment IDs) into EventFulfillsCommitment DHT
/// links and emits ProjectionSignal::ReaEconomicEventCommitted (~:10892) →
/// the storage projection upserts with dht_anchor_hash (rea_projection.rs).
pub async fn call_create_rea_economic_event(
    hc: &Arc<HcClient>,
    input: &shefa_types::CreateReaEconomicEventInput,
) -> Result<Vec<u8>, StorageError> {
    let payload = rmp_serde::to_vec_named(input).map_err(|e| {
        StorageError::Internal(format!(
            "conductor_writes: encode CreateReaEconomicEventInput: {e}"
        ))
    })?;
    hc.call_zome(ZOME_NAME, "create_rea_economic_event", payload)
        .await
}

/// Round-trip `update_rea_commitment_state` through the local conductor.
///
/// The zome's update_entry produces a new ActionHash for the same logical
/// id; the post-commit handler emits ProjectionSignal::ReaCommitmentCommitted
/// (dispatches on entry type — CREATE and UPDATE both fire the same signal
/// per content_store/src/lib.rs:10768). Receivers project the new state.
pub async fn call_update_rea_commitment_state(
    hc: &Arc<HcClient>,
    input: &shefa_types::UpdateReaCommitmentStateInput,
) -> Result<Vec<u8>, StorageError> {
    let payload = rmp_serde::to_vec_named(input).map_err(|e| {
        StorageError::Internal(format!(
            "conductor_writes: encode UpdateReaCommitmentStateInput: {e}"
        ))
    })?;
    hc.call_zome(ZOME_NAME, "update_rea_commitment_state", payload)
        .await
}

/// Round-trip `create_content` through the local conductor (lamad role).
///
/// Used by the lazy-migration bootstrap path in
/// `ContentService::update_via_conductor`: when a content row exists in
/// local SQL (from `bulk_create_content` during seeding) but has no
/// `dht_anchor_hash`, the first PATCH must publish the entry to the DHT
/// *before* update_content has a prev entry to mutate. The service layer
/// constructs `CreateContentInput` from the existing SQL row + patch and
/// calls this helper.
///
/// Post-commit fires `ProjectionSignal::ContentCommitted`, projected by
/// the receiver added in Task 8a.
pub async fn call_create_content(
    hc: &Arc<HcClient>,
    input: &lamad_types::CreateContentInput,
) -> Result<Vec<u8>, StorageError> {
    let payload = rmp_serde::to_vec_named(input).map_err(|e| {
        StorageError::Internal(format!("conductor_writes: encode CreateContentInput: {e}"))
    })?;
    hc.call_zome(ZOME_NAME, "create_content", payload).await
}

/// Round-trip `update_content` through the local conductor (lamad role).
///
/// Used by `ContentService::update_via_conductor` when the SQL row already
/// has a `dht_anchor_hash` (i.e. the entry has been published to the DHT
/// previously and we're patching specific fields like `blob_cid`).
///
/// The zome assumes the entry exists; failure mode = clean 5xx with a
/// "no Content entry found for id" message. Service layer ensures the
/// bootstrap branch handles the no-anchor case.
pub async fn call_update_content(
    hc: &Arc<HcClient>,
    input: &lamad_types::UpdateContentInput,
) -> Result<Vec<u8>, StorageError> {
    let payload = rmp_serde::to_vec_named(input).map_err(|e| {
        StorageError::Internal(format!("conductor_writes: encode UpdateContentInput: {e}"))
    })?;
    hc.call_zome(ZOME_NAME, "update_content", payload).await
}

/// Read content back from THIS conductor's DHT view by logical `id` via the
/// `content_store::get_content_by_id` coordinator (lamad role). `Ok(None)` when
/// the entry is not on this conductor's DHT view.
///
/// The read half of the reanchor backfill's already-exists recovery:
/// `create_content` refuses a duplicate id with a Guest error, so we read the
/// committed entry back to recover its `ActionHash` (the real
/// `dht_anchor_hash`) and project it. Same own-conductor-DHT-only discipline as
/// [`get_rea_commitment`] — peer bytes are never written into the projection.
/// The returned `ContentOutput` is the SAME wire shape (`content_to_wire`) that
/// `create_content`/`update_content` return, so it feeds the shared
/// `upsert_with_anchor` projection unchanged.
pub async fn get_content_by_id(
    hc: &Arc<HcClient>,
    id: &str,
) -> Result<Option<lamad_types::ContentOutput>, StorageError> {
    let input = lamad_types::QueryByIdInput { id: id.to_string() };
    let payload = rmp_serde::to_vec_named(&input).map_err(|e| {
        StorageError::Internal(format!("conductor_writes: encode QueryByIdInput: {e}"))
    })?;
    let bytes = hc
        .call_zome(ZOME_NAME, "get_content_by_id", payload)
        .await?;
    let out: Option<lamad_types::ContentOutput> = rmp_serde::from_slice(&bytes).map_err(|e| {
        StorageError::Serialization(format!(
            "conductor_writes: decode Option<ContentOutput>: {e}"
        ))
    })?;
    Ok(out)
}

/// Read a notarized REA Commitment back from THIS conductor's DHT view by its
/// logical `id`, via the `content_store::get_rea_commitment` coordinator
/// (lamad role). `Ok(None)` when the entry is not on this conductor's DHT view.
///
/// This is the read half the P1 projection reconciler uses: peers supply
/// discovery (which ids exist), but the row content comes EXCLUSIVELY from this
/// — the own-conductor DHT notary view. Peer bytes are never written into the
/// projection. The returned `ReaCommitmentOutput` carries the same wire shape
/// the post-commit `ReaCommitmentCommitted` signal carries (`action_hash` +
/// `commitment`), so both feed the shared `project_commitment_from_wire`
/// mapping.
pub async fn get_rea_commitment(
    hc: &Arc<HcClient>,
    id: &str,
) -> Result<Option<shefa_types::ReaCommitmentOutput>, StorageError> {
    let payload = rmp_serde::to_vec_named(&id.to_string()).map_err(|e| {
        StorageError::Internal(format!(
            "conductor_writes: encode get_rea_commitment id: {e}"
        ))
    })?;
    let bytes = hc
        .call_zome(ZOME_NAME, "get_rea_commitment", payload)
        .await?;
    let out: Option<shefa_types::ReaCommitmentOutput> =
        rmp_serde::from_slice(&bytes).map_err(|e| {
            StorageError::Serialization(format!(
                "conductor_writes: decode ReaCommitmentOutput: {e}"
            ))
        })?;
    Ok(out)
}

/// Wire mirror of the `content_store::{resolve_content_head, declare_content_head}`
/// coordinator output — the notary-declared HEAD of a content id's version DAG plus
/// the resolved head `Content` entry (HEAD-election, Plan C3 / notary-authority Leg 2).
///
/// Hash fields are typed [`HoloHashB64`] (accepts BOTH the raw-39-byte msgpack form the
/// conductor emits AND a base64 string), matching the signal-mirror convention — a
/// plain `String` mirror would silently drop the raw-byte wire form. `content` reuses
/// the [`ContentEntry`] mirror the signal path already carries.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ContentHeadWire {
    pub content_id: String,
    pub head_action_hash: HoloHashB64,
    #[serde(default)]
    pub entry_hash: Option<HoloHashB64>,
    #[serde(default)]
    pub author: Option<HoloHashB64>,
    /// Holochain `Timestamp` — i64 microseconds since the Unix epoch.
    pub declared_at: i64,
    /// The prior HEAD this declaration supersedes, if any.
    #[serde(default)]
    pub supersedes: Option<HoloHashB64>,
    pub content: ContentEntry,
    /// TRUE when the zome's answer was authoritative (canonical-head record or
    /// an explicit declaration act); FALSE for the root-author-newest FALLBACK
    /// election a cold conductor gives while the canonical link has not
    /// gossiped in. Drives stamp semantics in the heal path (fallback may FILL
    /// an undeclared row, never MOVE a declared one). `default` keeps
    /// old-coordinator wire output (no field) reading as false — safe.
    #[serde(default)]
    pub canonical: bool,
    /// The winning canonical-head declaration LINK's notarized DHT timestamp —
    /// the exact ordering `content_store::select_canonical_winner` arbitrated on.
    ///
    /// `Some` only when [`Self::canonical`] is true AND the coordinator is
    /// post-cure; `None` from a pre-cure coordinator (via `serde(default)`), on
    /// the root-author fallback, and on the declare paths. `None` reads as "no
    /// election stands behind this answer" everywhere — the safe default.
    ///
    /// NOT [`Self::declared_at`], which is the head ACTION's timestamp (and is
    /// replaced by the receiving conductor's `sys_time()` on the carried-record
    /// declare branch). Three clocks share that field; this is the one that can
    /// order two DECLARATIONS, and it is what
    /// `content_diesel::canonical_move_verdict` compares.
    #[serde(default)]
    pub canonical_declared_at: Option<i64>,
    /// Whether the winning declaration carried the EARNED provenance marker.
    /// `None` exactly when [`Self::canonical_declared_at`] is `None`. Lets the
    /// projection replay the selector's tier precedence (earned beats staging
    /// regardless of recency) without re-reading the DHT.
    #[serde(default)]
    pub canonical_earned: Option<bool>,
}

impl ContentHeadWire {
    /// The DHT election behind this answer, in the shape the stamp guard takes.
    /// `None` when the answer carries no election — which is the only thing a
    /// pre-cure coordinator, a fallback resolve, or a declare path can honestly
    /// report.
    pub fn canonical_ordering(&self) -> Option<crate::db::content_diesel::CanonicalOrdering> {
        self.canonical_declared_at
            .map(|ts| (ts, self.canonical_earned.unwrap_or(false)))
    }

    /// Election tier label for `elohim_content_canonical_answers_total`.
    pub fn canonical_tier_label(&self) -> &'static str {
        match self.canonical_earned {
            Some(true) => "earned",
            Some(false) => "staging",
            // No election resolved: the root-author FALLBACK. Also what a
            // pre-cure coordinator reports for a canonical answer, which is why
            // this reads `canonical_earned` rather than `canonical` — the meter
            // must not claim an election it has no evidence for.
            None => "none",
        }
    }
}

/// Caller-input wire shape for the `content_store::declare_content_head` coordinator:
/// declare (or advance) the notary HEAD for a content `id`. `head_action_hash = None`
/// asks the coordinator to resolve the author's latest committed action as the HEAD
/// (single-author auto-declare); `Some(_)` pins an explicit action.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeclareContentHeadInput {
    pub id: String,
    pub head_action_hash: Option<String>,
}

/// Caller-input wire shape for the `content_store::declare_canonical_content_head`
/// coordinator: the CROSS-ROOT canonical-head selector (notary-authority
/// convergence, Model B / Tier-1 STAGING tier). Unlike [`DeclareContentHeadInput`],
/// `head_action_hash` is REQUIRED — a cross-root declaration always names its
/// explicit target action (any retrievable Content action for the id, including
/// one authored under a different root by a different agent).
///
/// `carried_record` is the DECLARE-CARRIES-RECORD extension (sprint-3): the
/// `holochain_serialized_bytes`-encoded `Record` of the target, served by the
/// AUTHORING peer via [`call_get_record_for_action`]. On a full-arc fleet
/// (`target_arc_factor = 1`) every conductor is an authority for every hash, so
/// the coordinator's `get` cascade short-circuits WITHOUT a network fetch — a
/// gossip gap reads as absence and no retry ladder can clear it. Carrying the
/// record lets the declaring conductor verify the target in wasm instead of
/// waiting for gossip that will never arrive.
///
/// Additive: `None` (the pre-sprint-3 shape) is serialized as a msgpack nil the
/// coordinator's `#[serde(default)] Option<Vec<u8>>` decodes to `None`, so an
/// old coordinator and a new one both accept this payload.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeclareCanonicalHeadInput {
    pub id: String,
    pub head_action_hash: String,
    /// `serde_bytes` keeps the multi-KB carried record a MessagePack `bin`
    /// (compact) rather than an array-of-ints (~2x bloat). MUST match the
    /// coordinator's `content_store::DeclareCanonicalHeadInput.carried_record`,
    /// which is also `#[serde(default, with = "serde_bytes")]`.
    #[serde(default, with = "serde_bytes")]
    pub carried_record: Option<Vec<u8>>,
}

/// Caller-input wire shape for the `content_store::get_record_for_action`
/// coordinator — the SOURCE half of declare-carries-Record.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GetRecordForActionInput {
    pub action_hash: String,
}

/// Typed decode of the `content_store::get_record_for_action` coordinator
/// return (`CarriedRecordOutput`). `record` is the opaque HSB/MessagePack
/// encoding of the `Record` — storage never interprets it, it only relays it
/// (base64 on the HTTP hop, raw bytes on the zome hop).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CarriedRecordWire {
    pub action_hash: String,
    /// `serde_bytes` keeps the opaque `Record` encoding a MessagePack `bin`
    /// (compact) rather than an array-of-ints (~2x bloat). MUST match the
    /// coordinator's `content_store::CarriedRecordOutput.record`, which is also
    /// `#[serde(with = "serde_bytes")]`.
    #[serde(with = "serde_bytes")]
    pub record: Vec<u8>,
}

/// Resolve the notary-declared HEAD for a content `id` from THIS conductor's DHT view
/// via the `content_store::resolve_content_head` coordinator (lamad role). `Ok(None)`
/// when no HEAD is declared on this conductor's view.
///
/// This is the read half the HEAD-election reconcile leg uses before stamping the
/// local projection: peers supply discovery (which ids exist), but the head content
/// comes EXCLUSIVELY from this own-conductor DHT notary view — peer bytes are never
/// written into the projection (the same P1 discipline as [`get_rea_commitment`]).
pub async fn call_resolve_content_head(
    hc: &Arc<HcClient>,
    id: &str,
) -> Result<Option<ContentHeadWire>, StorageError> {
    let payload = rmp_serde::to_vec_named(&id.to_string()).map_err(|e| {
        StorageError::Internal(format!(
            "conductor_writes: encode resolve_content_head id: {e}"
        ))
    })?;
    let bytes = hc
        .call_zome(ZOME_NAME, "resolve_content_head", payload)
        .await?;
    let out: Option<ContentHeadWire> = rmp_serde::from_slice(&bytes).map_err(|e| {
        StorageError::Serialization(format!(
            "conductor_writes: decode ContentHeadWire (resolve): {e}"
        ))
    })?;
    Ok(out)
}

/// [`call_resolve_content_head`] restricted to the conductor's LOCAL databases,
/// via the `content_store::resolve_content_head_local` coordinator.
///
/// FOR THE HEAL LOOP ONLY. A `Network` resolve cannot complete on a node whose
/// storage arc has not reconverged since its last restart: authority follows the
/// agent's CURRENT arc, kitsune2 resets that arc to `Empty` on every conductor
/// start, and until a gossip round promotes it back to FULL every `get_links`
/// leaves the box and dies on the conductor's request timeout. The heal traffic
/// is itself what keeps the fetch queue from draining, so the stall is
/// self-sustaining; reading local breaks it.
///
/// CONTRACT — `Ok(None)` here means "not in this conductor's local view YET",
/// **not** "does not exist". Never use it to gate authorship, deny a declare, or
/// answer a 404. The HTTP author gate and the adoption pre-flight deliberately
/// keep calling [`call_resolve_content_head`] (Network) for exactly that reason.
pub async fn call_resolve_content_head_local(
    hc: &Arc<HcClient>,
    id: &str,
) -> Result<Option<ContentHeadWire>, StorageError> {
    let payload = rmp_serde::to_vec_named(&id.to_string()).map_err(|e| {
        StorageError::Internal(format!(
            "conductor_writes: encode resolve_content_head_local id: {e}"
        ))
    })?;
    let bytes = hc
        .call_zome(ZOME_NAME, "resolve_content_head_local", payload)
        .await?;
    let out: Option<ContentHeadWire> = rmp_serde::from_slice(&bytes).map_err(|e| {
        StorageError::Serialization(format!(
            "conductor_writes: decode ContentHeadWire (resolve local): {e}"
        ))
    })?;
    Ok(out)
}

/// Declare (or advance) the notary HEAD for a content `id` via the
/// `content_store::declare_content_head` coordinator (lamad role). `head_action_hash =
/// None` lets the coordinator resolve the author's latest committed action as the HEAD
/// (single-author auto-declare); `Some(_)` pins an explicit action. Returns the
/// resulting [`ContentHeadWire`].
///
/// Error strings are preserved verbatim (the underlying `call_zome` error propagates
/// unmapped via `?`) so callers can match on the coordinator's "not the author"
/// substring — only the authoring agent may declare its own content's HEAD.
pub async fn call_declare_content_head(
    hc: &Arc<HcClient>,
    id: &str,
    head_action_hash: Option<String>,
) -> Result<ContentHeadWire, StorageError> {
    let input = DeclareContentHeadInput {
        id: id.to_string(),
        head_action_hash,
    };
    let payload = rmp_serde::to_vec_named(&input).map_err(|e| {
        StorageError::Internal(format!(
            "conductor_writes: encode DeclareContentHeadInput: {e}"
        ))
    })?;
    let bytes = hc
        .call_zome(ZOME_NAME, "declare_content_head", payload)
        .await?;
    let out: ContentHeadWire = rmp_serde::from_slice(&bytes).map_err(|e| {
        StorageError::Serialization(format!(
            "conductor_writes: decode ContentHeadWire (declare): {e}"
        ))
    })?;
    Ok(out)
}

/// Declare the CROSS-ROOT canonical HEAD for a content `id` via the
/// `content_store::declare_canonical_content_head` coordinator (lamad role, STAGING
/// tier). Unlike [`call_declare_content_head`], `head_action_hash` is REQUIRED — the
/// zome's `DeclareCanonicalHeadInput.head_action_hash` is a `holo_hash::ActionHashB64`
/// (serde-transparent), which accepts a canonical base64 `uhCkk…` String over the wire.
/// Returns the resulting [`ContentHeadWire`].
///
/// Error strings are preserved verbatim (the underlying `call_zome` error propagates
/// unmapped via `?`) so callers can match on the coordinator's guard substrings — e.g.
/// "earned head is protected" (staging cannot override an earned canonical) or
/// unauthorized-declarer.
///
/// `carried_record` (declare-carries-Record) is the optional HSB-encoded
/// `Record` of the target. Pass `None` for the classic behaviour; pass
/// `Some(bytes)` — obtained from the authoring peer via
/// [`call_get_record_for_action`] — to let a conductor that cannot retrieve the
/// target declare it anyway. The coordinator consults it ONLY on a local miss,
/// and only after re-deriving the action hash, author signature, and
/// entry↔action binding in wasm.
pub async fn call_declare_canonical_content_head(
    hc: &Arc<HcClient>,
    id: &str,
    head_action_hash: String,
    carried_record: Option<Vec<u8>>,
) -> Result<ContentHeadWire, StorageError> {
    let input = DeclareCanonicalHeadInput {
        id: id.to_string(),
        head_action_hash,
        carried_record,
    };
    let payload = rmp_serde::to_vec_named(&input).map_err(|e| {
        StorageError::Internal(format!(
            "conductor_writes: encode DeclareCanonicalHeadInput: {e}"
        ))
    })?;
    let bytes = hc
        .call_zome(ZOME_NAME, "declare_canonical_content_head", payload)
        .await?;
    let out: ContentHeadWire = rmp_serde::from_slice(&bytes).map_err(|e| {
        StorageError::Serialization(format!(
            "conductor_writes: decode ContentHeadWire (declare_canonical): {e}"
        ))
    })?;
    Ok(out)
}

/// Fetch the full serialized `Record` for an action from THIS conductor's local
/// DHT view via `content_store::get_record_for_action` — the SOURCE half of
/// declare-carries-Record.
///
/// The authoring peer answers `Some(..)`; a peer that cannot retrieve the
/// action answers `Ok(None)` (honest absence, not an error — the caller then
/// declares without a carried record and gets the classic behaviour).
///
/// A `fn-not-found`-class conductor error propagates verbatim: it is the
/// hot-swap probe signal telling the caller this peer still runs a pre-sprint-3
/// coordinator.
pub async fn call_get_record_for_action(
    hc: &Arc<HcClient>,
    action_hash: &str,
) -> Result<Option<CarriedRecordWire>, StorageError> {
    let input = GetRecordForActionInput {
        action_hash: action_hash.to_string(),
    };
    let payload = rmp_serde::to_vec_named(&input).map_err(|e| {
        StorageError::Internal(format!(
            "conductor_writes: encode GetRecordForActionInput: {e}"
        ))
    })?;
    let bytes = hc
        .call_zome(ZOME_NAME, "get_record_for_action", payload)
        .await?;
    let out: Option<CarriedRecordWire> = rmp_serde::from_slice(&bytes).map_err(|e| {
        StorageError::Serialization(format!(
            "conductor_writes: decode CarriedRecordWire (get_record_for_action): {e}"
        ))
    })?;
    Ok(out)
}

/// Coordinator zome name (the mishpat zome, hosted in the mishpat role cell —
/// role selection happens at HcClient construction). Hosts the Mishpat commitment
/// coordinator functions (`create_commitment`, `get_commitment`); `call_zome`
/// dispatches by this zome name.
const MISHPAT_ZOME: &str = "mishpat";

/// Caller-input wire shape for the Mishpat `create_commitment` coordinator.
///
/// Mirrors `mishpat::commitments::CreateCommitmentInput` field-for-field. The
/// `payload_json` stays a String across the zome boundary (never a
/// `serde_json::Value` — see dna/CLAUDE.md MessagePack-at-boundary rule).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateMishpatCommitmentInput {
    pub action: String,
    pub payload_json: String,
    pub signed_at: String,
}

/// Typed decode of the `mishpat::create_commitment` coordinator return
/// (`CommitmentOutput { action_hash, entry_hash }`).
///
/// The coordinator returns native `ActionHash`/`EntryHash` HoloHash types (not
/// base64 strings — that is only the *signal* wire shape). Decoding into these
/// typed fields lets callers derive the canonical base64 `uhCkk…`/`uhCEk…` CIDs
/// via the HoloHash `Display` impl. The **`entry_hash`** is the canonical
/// commitment CID (what the projection stores as `cid` and every consumer keys
/// off — see [`create_commitment_returning_cid`]); the `action_hash` is the
/// provenance anchor the projection stores separately as `dht_anchor_hash`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateCommitmentOutput {
    pub action_hash: holochain_types::prelude::ActionHash,
    pub entry_hash: holochain_types::prelude::EntryHash,
}

/// Round-trip the Mishpat `create_commitment` coordinator and decode the new
/// commitment's `entry_hash` to its canonical base64 CID string.
///
/// This is the typed convenience over [`call_create_commitment`] for callers
/// (e.g. the production `CommitmentAuthor`) that need the new commitment CID as
/// the pin back-reference (`pin.commitment_cid`), the `bounded_by` annotation on
/// emitted economic events, and the `target_cid` for a later revocation.
///
/// We return the **`entry_hash`**, NOT the `action_hash`. The content-addressed
/// `entry_hash` is the canonical commitment CID: the post-commit projection
/// writes the `mishpat_commitments` row with `cid = entry_hash` (and stores the
/// `action_hash` separately as `dht_anchor_hash`), and EVERY downstream consumer
/// keys off `cid`:
/// - [`crate::services::commitment_fetcher::ProjectionCommitmentFetcher::fetch`]
///   / `get_by_cid` → `mc::cid.eq(cid)`
/// - rea graduation → `graduate_to_active(conn, bounded_by)` → `cid.eq(bounded_by)`
/// - T10 revocation → `set_revoked_at(conn, target_cid)` → `cid.eq(target_cid)`
///
/// Returning the `action_hash` here would mint a `bounded_by`/`commitment_cid`
/// that resolves to no row (the projection is keyed by `entry_hash`), breaking
/// the bounds check, graduation, and revocation.
pub async fn create_commitment_returning_cid(
    hc: &Arc<HcClient>,
    input: CreateMishpatCommitmentInput,
) -> Result<String, StorageError> {
    let bytes = call_create_commitment(hc, input).await?;
    let out: CreateCommitmentOutput = rmp_serde::from_slice(&bytes).map_err(|e| {
        StorageError::Serialization(format!(
            "conductor_writes: decode CreateCommitmentOutput: {e}"
        ))
    })?;
    Ok(format!("{}", out.entry_hash))
}

/// Wire mirror of the `mishpat::get_commitment` output. Used by
/// [`crate::services::commitment_fetcher::ConductorCommitmentFetcher::fetch`] to
/// read a notarized commitment back.
///
/// Like [`CreateCommitmentOutput`], the coordinator returns native
/// `ActionHash`/`EntryHash` HoloHash types over the zome-call wire (msgpack
/// byte arrays) — `String` fields here fail decode with "invalid value: byte
/// array, expected a string" (the 2026-06-13 signal-decode class, resurfaced
/// live 2026-07-26 when the read-back bounds check first executed against a
/// real conductor). Callers derive the canonical base64 `uhCkk…`/`uhCEk…`
/// strings via the HoloHash `Display` impl.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GetCommitmentOutput {
    /// Action hash — the value the projection stores (base64) as `dht_anchor_hash`.
    pub action_hash: holochain_types::prelude::ActionHash,
    /// Entry hash — the storage `cid` (base64 via `Display`).
    pub entry_hash: holochain_types::prelude::EntryHash,
    pub action: String,
    pub payload_json: String,
    pub signed_at: String,
}

/// Round-trip the Mishpat `create_commitment` coordinator. Returns the raw
/// `CommitmentOutput` MessagePack bytes (action_hash + entry_hash); callers
/// that need the anchor decode with `rmp_serde::from_slice`. The post-commit
/// signal projects the commitment into `mishpat_commitments` with
/// `dht_anchor_hash = action_hash` (Slice 2b T1).
pub async fn call_create_commitment(
    hc: &Arc<HcClient>,
    input: CreateMishpatCommitmentInput,
) -> Result<Vec<u8>, StorageError> {
    let payload = rmp_serde::to_vec_named(&input).map_err(|e| {
        StorageError::Internal(format!(
            "conductor_writes: encode CreateMishpatCommitmentInput: {e}"
        ))
    })?;
    hc.call_zome_mishpat(MISHPAT_ZOME, "create_commitment", payload)
        .await
}

/// Input for `create_commitment_state_link` — records a Commitment's lifecycle
/// transition as a notarized `CommitmentByState` link in the Mishpat DNA.
///
/// The link is the source of truth for lifecycle; the SQL `state` column is a
/// write-through cache (`graduate_to_active` writes the cache, this writes the
/// truth). `event_hash` is the graduating EconomicEvent's action_hash — the
/// link's tag carries `<state>|<signed_at>` so a verifier can replay the proof.
/// Mirrors `mishpat::commitments::CreateCommitmentStateLinkInput` field-for-field.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateCommitmentStateLinkInput {
    /// Base64 `entry_hash` of the Commitment being transitioned (the live anchor;
    /// the same value the projection stores as `mishpat_commitments.cid`).
    pub commitment_cid: String,
    /// New lifecycle state, e.g. "active".
    pub state: String,
    /// Base64 `action_hash` of the event that justifies the transition.
    pub event_hash: String,
    /// ISO-8601 signing time (Category-A determinism — never sys_time in-zome).
    pub signed_at: String,
}

/// Author a `CommitmentByState` link recording a commitment's state transition.
///
/// Called from the graduation projection right after `graduate_to_active` flips
/// the SQL cache, so the DHT link and the cache agree. Returns raw bytes
/// (`CommitmentStateLinkOutput { link_action_hash }`); the caller rarely needs
/// the result — the link is fire-and-confirm.
pub async fn call_create_commitment_state_link(
    hc: &Arc<HcClient>,
    input: CreateCommitmentStateLinkInput,
) -> Result<Vec<u8>, StorageError> {
    let payload = rmp_serde::to_vec_named(&input).map_err(|e| {
        StorageError::Internal(format!(
            "conductor_writes: encode CreateCommitmentStateLinkInput: {e}"
        ))
    })?;
    hc.call_zome_mishpat(MISHPAT_ZOME, "create_commitment_state_link", payload)
        .await
}

/// Read a notarized Mishpat commitment back by its base64 `cid` (entry_hash)
/// via the `mishpat::get_commitment` coordinator. `Ok(None)` when the entry is
/// not on this conductor's DHT view. This is the conductor-backed read path for
/// [`crate::services::commitment_fetcher::ConductorCommitmentFetcher`].
pub async fn get_commitment(
    hc: &Arc<HcClient>,
    cid: &str,
) -> Result<Option<GetCommitmentOutput>, StorageError> {
    let payload = rmp_serde::to_vec_named(&cid.to_string()).map_err(|e| {
        StorageError::Internal(format!("conductor_writes: encode get_commitment cid: {e}"))
    })?;
    let bytes = hc
        .call_zome_mishpat(MISHPAT_ZOME, "get_commitment", payload)
        .await?;
    let out: Option<GetCommitmentOutput> = rmp_serde::from_slice(&bytes).map_err(|e| {
        StorageError::Serialization(format!("conductor_writes: decode GetCommitmentOutput: {e}"))
    })?;
    Ok(out)
}

#[cfg(test)]
mod collective_cid_tests {
    use super::*;

    /// The reconcile arm drops an undecodable peer cid at DISCOVERY rather than
    /// enqueuing it — an id that can never resolve would otherwise burn a
    /// conductor round-trip every sweep forever. This pins the classifier those
    /// drops are made on, and the round-trip through the coordinator's own
    /// `format!("collective:{hash}")` encoding.
    #[test]
    fn decode_collective_cid_round_trips_and_rejects_junk() {
        // `from_raw_32` COMPUTES the 4 DHT-location bytes; `from_raw_36` would
        // leave them as supplied and the base64 form then fails `try_from`'s
        // checksum — i.e. the decoder really does validate, which is why an
        // undecodable peer cid is safe to drop at discovery.
        let hash = holochain_types::prelude::ActionHash::from_raw_32(vec![0x5A; 32]);
        let cid = format!("{COLLECTIVE_CID_PREFIX}{hash}");
        assert_eq!(
            decode_collective_cid(&cid).expect("round-trips the coordinator encoding"),
            hash
        );

        for bad in [
            "",
            "uhCkkNoPrefix",
            "agreement:uhCkkWrongPrefix",
            // `collective:` (empty suffix) PANICKED inside holo_hash_decode's
            // `&s[..1]` before the guard in `decode_collective_cid` — and this
            // value comes off a peer's wire, so it would have aborted the whole
            // reconcile task. This case is the regression pin.
            "collective:",
            "collective:not-a-hash",
            "collective:uhCAkWrongHashType0000000000000000000000000",
            // Right prefix, right shape, WRONG checksum — the decoder really
            // validates, which is what makes dropping an undecodable peer cid at
            // discovery safe rather than lossy.
            &format!(
                "{COLLECTIVE_CID_PREFIX}{}",
                holochain_types::prelude::ActionHash::from_raw_36(vec![0x5A; 36])
            ),
        ] {
            assert!(
                decode_collective_cid(bad).is_err(),
                "'{bad}' must be rejected up front, never enqueued as a gap"
            );
        }
    }

    /// The `Collective` entry mirror decodes the coordinator's msgpack body,
    /// and tolerates an older sender that omits the additive tail fields.
    #[test]
    fn collective_wire_decodes_entry_body() {
        #[derive(serde::Serialize)]
        struct ZomeCollective<'a> {
            founder_agent_cid: &'a str,
            charter: &'a str,
            display_name: &'a str,
            created_at_block_height: u64,
            salt: &'a str,
            anchor_agreement_cid: Option<&'a str>,
        }
        let bytes = rmp_serde::to_vec_named(&ZomeCollective {
            founder_agent_cid: "uhCAkFounder0001",
            charter: r#"{"kind":"household","slugAlias":"household-dowell"}"#,
            display_name: "Dowell Household",
            created_at_block_height: 42,
            salt: "s",
            anchor_agreement_cid: None,
        })
        .expect("encode");
        let wire: CollectiveWire = rmp_serde::from_slice(&bytes).expect("decode");
        assert_eq!(wire.display_name, "Dowell Household");
        assert_eq!(wire.founder_agent_cid, "uhCAkFounder0001");
        assert_eq!(wire.created_at_block_height, 42);
        assert!(wire.anchor_agreement_cid.is_none());
        // Household-ness rides the charter string through to the shared mapping.
        assert!(crate::db::collectives::CharterHints::parse(Some(&wire.charter)).is_household());

        // Additive-tail tolerance: a sender without the optional fields decodes.
        #[derive(serde::Serialize)]
        struct Minimal<'a> {
            founder_agent_cid: &'a str,
            charter: &'a str,
            display_name: &'a str,
        }
        let bytes = rmp_serde::to_vec_named(&Minimal {
            founder_agent_cid: "uhCAkFounder0002",
            charter: "{}",
            display_name: "Bare",
        })
        .expect("encode");
        let wire: CollectiveWire = rmp_serde::from_slice(&bytes).expect("decode minimal");
        assert_eq!(wire.display_name, "Bare");
        assert_eq!(wire.created_at_block_height, 0);
    }
}

#[cfg(test)]
mod tests {
    /// Asserts that `shefa_types::CreateReaCommitmentInput` survives a
    /// MessagePack named-fields round-trip via `rmp_serde::to_vec_named` →
    /// `rmp_serde::from_slice`. This is the wire-shape contract between
    /// elohim-storage's HTTP write path and the DNA's content_store zome:
    /// if encoding drops a field or the receiving side can't decode, the
    /// conductor call would 500 at runtime.
    ///
    /// Real end-to-end execution (the actual zome call landing as a DHT
    /// entry on a sweettest conductor) is covered by Task 9's sweettest at
    /// `elohim/holochain/tests/sweettest/tests/rea_commitment_replication.rs`.
    #[test]
    fn create_rea_commitment_input_serde_roundtrip() {
        let original = shefa_types::CreateReaCommitmentInput {
            id: "test-projection-001".to_string(),
            action: "project-epr".to_string(),
            provider: "doorway:alpha-elohim-host".to_string(),
            receiver: "epr:lamad-spa".to_string(),
            resource_classified_as: vec![],
            resource_quantity_value: None,
            resource_quantity_unit: None,
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_beginning: None,
            has_end: None,
            due: None,
            clause_of: None,
            in_scope_of: vec!["doorway:alpha-elohim-host|epr:lamad-spa".to_string()],
            note: None,
            metadata_json: None,
        };

        let bytes = rmp_serde::to_vec_named(&original).expect("encode");
        let decoded: shefa_types::CreateReaCommitmentInput =
            rmp_serde::from_slice(&bytes).expect("decode");

        assert_eq!(decoded.id, original.id);
        assert_eq!(decoded.action, original.action);
        assert_eq!(decoded.provider, original.provider);
        assert_eq!(decoded.receiver, original.receiver);
        assert_eq!(decoded.in_scope_of, original.in_scope_of);
        assert_eq!(
            decoded.resource_classified_as,
            original.resource_classified_as
        );
    }

    /// The state-link author input must survive a MessagePack named-fields
    /// round-trip — the wire contract with the Mishpat coordinator's
    /// create_commitment_state_link extern. A dropped field would 500 at runtime.
    #[test]
    fn create_commitment_state_link_input_serde_roundtrip() {
        let original = super::CreateCommitmentStateLinkInput {
            commitment_cid: "anchor:commitment-1".to_string(),
            state: "active".to_string(),
            event_hash: "uhCkk-graduating-event".to_string(),
            signed_at: "2026-06-11T10:00:00Z".to_string(),
        };
        let bytes = rmp_serde::to_vec_named(&original).expect("encode");
        let decoded: super::CreateCommitmentStateLinkInput =
            rmp_serde::from_slice(&bytes).expect("decode");
        assert_eq!(decoded.commitment_cid, original.commitment_cid);
        assert_eq!(decoded.state, original.state);
        assert_eq!(decoded.event_hash, original.event_hash);
        assert_eq!(decoded.signed_at, original.signed_at);
    }
}
