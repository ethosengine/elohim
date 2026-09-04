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

use crate::conductor_admission::AdmissionClass;
use crate::error::StorageError;
use crate::hc_client::HcClient;
use crate::rea_projection::ContentEntry;
use crate::signals::HoloHashB64;

/// The admission class the un-classed declare wrappers delegate with.
///
/// Interactive is the correct default because the callers that kept the short
/// signatures are the HTTP declare routes — a deliberate operator act with a
/// person waiting on the answer. Every caller with a cheap deferral path (the
/// reconcile sweeps) names [`AdmissionClass::Background`] explicitly through the
/// `_classed` variants, so the lane a person stands in is never occupied by
/// work that could just as well ride the next sweep.
pub const DECLARE_DEFAULT_CLASS: AdmissionClass = AdmissionClass::Interactive;

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

/// Wire mirror of `imagodei_integrity::qahal::Membership` — the DHT entry body
/// as it arrives inside a `Record`'s `Entry::App` msgpack bytes.
///
/// A LOCAL mirror for the same reason [`CollectiveWire`] is one: elohim-storage
/// does not link the DNA crates. The two enum fields reuse the storage-side
/// signal mirrors ([`crate::signals::MemberKindPayload`] /
/// [`crate::signals::MembershipRolePayload`]) rather than re-declaring them, so
/// the conductor-read path and the post-commit signal path can never disagree
/// about how a role maps to a `role_context` string.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MembershipWire {
    pub member_cid: String,
    pub member_kind: crate::signals::MemberKindPayload,
    pub collective_cid: String,
    pub role: crate::signals::MembershipRolePayload,
    #[serde(default)]
    pub sponsor_cid: Option<String>,
    #[serde(default)]
    pub joined_at_block_height: u64,
    #[serde(default)]
    pub withdrawn_at_block_height: Option<u64>,
}

/// Resolve a `Membership` ActionHash against the OWN conductor's imagodei cell
/// (`get_membership_by_action`).
///
/// The participations arm's analogue of [`get_collective_by_cid`]: the
/// reconcile leg learns WHICH Membership anchors exist from peers (discovery),
/// then reads the entry BODY exclusively from its own conductor. No peer bytes
/// ever enter the projection.
///
/// `Ok(None)` means the own conductor cannot see the entry (not yet gossiped
/// in, or a foreign-DHT anchor) — retried on the NEXT sweep, never immediately.
/// A malformed anchor is an `InvalidInput` error, not a miss: it can never
/// resolve, so re-attempting it every sweep would burn a round-trip forever.
///
/// NOTE the coordinator returns `ExternResult<Record>`, NOT `Option<Record>` —
/// a missing Membership arrives as a wasm error carrying `"Membership not
/// found"`. That string match is the only available not-found signal, so it is
/// classified here (and ONLY here) rather than leaking a "the conductor is
/// broken" error into the heal loop; anything else propagates as `Err`.
pub async fn get_membership_by_anchor(
    hc: &Arc<HcClient>,
    action_hash_b64: &str,
) -> Result<Option<MembershipWire>, StorageError> {
    let action_hash = decode_action_hash(action_hash_b64)?;
    let payload = rmp_serde::to_vec_named(&action_hash).map_err(|e| {
        StorageError::Internal(format!(
            "conductor_writes: encode ActionHash for get_membership_by_action: {e}"
        ))
    })?;
    let bytes = match hc
        .call_zome_imagodei(IMAGODEI_ZOME, "get_membership_by_action", payload)
        .await
    {
        Ok(b) => b,
        Err(e) if is_membership_not_found(&e) => return Ok(None),
        Err(e) => return Err(e),
    };
    // TYPED `Record` decode — a `serde_json::Value` pre-pass or a hand-rolled
    // mirror drops the holo_hash fields inside `SignedActionHashed` (the
    // msgpack raw-bytes decode class).
    let record: holochain_types::prelude::Record = rmp_serde::from_slice(&bytes).map_err(|e| {
        StorageError::Serialization(format!(
            "conductor_writes: decode Record from get_membership_by_action: {e}"
        ))
    })?;
    // A record whose entry is absent (a delete/tombstone) is not a Membership.
    let Some(holochain_types::prelude::Entry::App(eb)) = record.entry.as_option() else {
        return Ok(None);
    };
    let membership: MembershipWire = rmp_serde::from_slice(eb.bytes()).map_err(|e| {
        StorageError::Serialization(format!(
            "conductor_writes: decode Membership entry for {action_hash_b64}: {e}"
        ))
    })?;
    Ok(Some(membership))
}

/// Whether a `get_membership_by_action` error is the coordinator's own
/// not-found, rather than a transport/conductor fault. Both strings come from
/// `qahal_coordinator::get_membership_by_action`'s two `wasm_error!` arms.
fn is_membership_not_found(err: &StorageError) -> bool {
    let lowered = err.to_string().to_ascii_lowercase();
    lowered.contains("membership not found") || lowered.contains("membership update record missing")
}

/// Decode a bare `uhCkk…` HoloHash-display string back to its `ActionHash`.
///
/// Pure + total: every failure mode is a legible `InvalidInput`. Carries the
/// same PANIC GUARD [`decode_collective_cid`] documents — `holo_hash_decode`
/// opens with `&s[..1]` and PANICS on an empty string, and this value arrives
/// from a PEER's advertised inventory, so an empty anchor would abort the whole
/// reconcile task (the one-poisoned-row class).
pub fn decode_action_hash(
    action_hash_b64: &str,
) -> Result<holochain_types::prelude::ActionHash, StorageError> {
    let raw = action_hash_b64.trim();
    if !raw.starts_with('u') {
        return Err(StorageError::InvalidInput(format!(
            "invalid ActionHash '{action_hash_b64}': must be a 'u'-prefixed HoloHash"
        )));
    }
    holochain_types::prelude::ActionHash::try_from(raw).map_err(|e| {
        StorageError::InvalidInput(format!("invalid ActionHash '{action_hash_b64}': {e}"))
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
    /// The STAGING declaration standing BENEATH an earned winner — the next
    /// release on this channel awaiting promotion
    /// (`content_store::select_staging_candidate`, a pure function of the same
    /// link set, so every peer names the same candidate).
    ///
    /// `Some` only when [`Self::canonical_earned`] is `Some(true)` AND a staging
    /// declaration postdates the winning earned one. `None` otherwise, and
    /// `None` from any pre-candidate coordinator via `serde(default)` — which
    /// reads as "no candidate stands beneath this head", the safe default: a
    /// peer that cannot see a candidate simply keeps following the winner.
    ///
    /// This never competes with [`Self::head_action_hash`]. The elected head is
    /// still the single winner; the candidate is a SUBORDINATE declaration a
    /// canary may verify and attest ahead of promotion.
    #[serde(default)]
    pub staging_candidate: Option<HoloHashB64>,
    /// The candidate declaration LINK's notarized DHT timestamp — the same clock
    /// class as [`Self::canonical_declared_at`] and orderable against it.
    /// `Some` exactly when [`Self::staging_candidate`] is `Some`.
    #[serde(default)]
    pub staging_candidate_declared_at: Option<i64>,
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

/// Mirror of `content_store::CanonicalElectionOutput` — what the DHT elected for
/// an id, WITHOUT the winner's Content bytes.
///
/// This is the answer `resolve_content_head` structurally cannot give: it
/// collapses "no election" and "election visible but target unretrievable" into
/// the same `None`, which is why a conductor-missing peer could never act on a
/// head the DHT had already decided.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CanonicalElectionWire {
    pub winner_target: HoloHashB64,
    /// The winning declaration LINK's notarized timestamp — feeds
    /// `content_diesel::CanonicalOrdering` directly.
    pub canonical_declared_at: i64,
    pub canonical_earned: bool,
    /// The STAGING declaration standing beneath an earned winner — see
    /// `ContentHeadWire::staging_candidate` for the full contract. Additive and
    /// `serde(default)`: a pre-candidate coordinator omits the key and it reads
    /// as "no candidate".
    #[serde(default)]
    pub staging_candidate: Option<HoloHashB64>,
    /// The candidate declaration LINK's notarized timestamp. `Some` exactly when
    /// [`Self::staging_candidate`] is `Some`.
    #[serde(default)]
    pub staging_candidate_declared_at: Option<i64>,
}

impl CanonicalElectionWire {
    /// The election in the shape the stamp guard takes.
    pub fn ordering(&self) -> crate::db::content_diesel::CanonicalOrdering {
        (self.canonical_declared_at, self.canonical_earned)
    }
}

/// Ask the conductor what the DHT elected for `id`, without needing it to serve
/// the winner's bytes (`content_store::resolve_canonical_election`).
///
/// `Ok(None)` = no election in this conductor's LOCAL view yet — never
/// authoritative absence.
pub async fn call_resolve_canonical_election(
    hc: &Arc<HcClient>,
    id: &str,
) -> Result<Option<CanonicalElectionWire>, StorageError> {
    let payload = rmp_serde::to_vec_named(&id.to_string()).map_err(|e| {
        StorageError::Internal(format!(
            "conductor_writes: encode resolve_canonical_election id: {e}"
        ))
    })?;
    let bytes = hc
        .call_zome(ZOME_NAME, "resolve_canonical_election", payload)
        .await?;
    let out: Option<CanonicalElectionWire> = rmp_serde::from_slice(&bytes).map_err(|e| {
        StorageError::Serialization(format!(
            "conductor_writes: decode CanonicalElectionWire: {e}"
        ))
    })?;
    Ok(out)
}

/// Wire shape of `content_store::get_canonical_election_evidence`'s answer —
/// the election PLUS the winning declaration LINK's own serialized `Record`
/// (the evidence a requester's conductor re-derives in wasm).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CanonicalElectionEvidenceWire {
    pub election: CanonicalElectionWire,
    /// `holochain_serialized_bytes`-encoded `Record` of the winning
    /// canonical-head CreateLink action — opaque to this layer.
    #[serde(with = "serde_bytes")]
    pub link_record: Vec<u8>,
}

/// Ask the conductor for its canonical election for `id` WITH the winning
/// declaration link's `Record` (`content_store::get_canonical_election_evidence`)
/// — the SOURCE side of carry-the-election. `Ok(None)` = no election in this
/// conductor's local view, or the link's Record is not locally retrievable —
/// never authoritative absence.
pub async fn call_get_canonical_election_evidence(
    hc: &Arc<HcClient>,
    id: &str,
) -> Result<Option<CanonicalElectionEvidenceWire>, StorageError> {
    let payload = rmp_serde::to_vec_named(&id.to_string()).map_err(|e| {
        StorageError::Internal(format!(
            "conductor_writes: encode get_canonical_election_evidence id: {e}"
        ))
    })?;
    let bytes = hc
        .call_zome(ZOME_NAME, "get_canonical_election_evidence", payload)
        .await?;
    let out: Option<CanonicalElectionEvidenceWire> =
        rmp_serde::from_slice(&bytes).map_err(|e| {
            StorageError::Serialization(format!(
                "conductor_writes: decode CanonicalElectionEvidenceWire: {e}"
            ))
        })?;
    Ok(out)
}

/// Caller-input wire shape for `content_store::verify_carried_election`.
#[derive(Debug, serde::Serialize)]
struct VerifyCarriedElectionInput {
    id: String,
    #[serde(with = "serde_bytes")]
    link_record: Vec<u8>,
}

/// Hand a peer-carried canonical-head declaration LINK `Record` to the OWN
/// conductor for full wasm re-derivation (`content_store::verify_carried_election`):
/// self-binding, author signature, anchor binding to `id`, link type + tier —
/// then a merge against every locally-visible candidate under the one shared
/// ordering. `Ok(Some(_))` is the election the merged set yields (which may be
/// a LOCAL candidate outranking the carried one); `Ok(None)` = no candidate
/// survived; `Err` includes the zome's refusals (forged or misbound evidence —
/// worth reading the text).
pub async fn call_verify_carried_election(
    hc: &Arc<HcClient>,
    id: &str,
    link_record: Vec<u8>,
) -> Result<Option<CanonicalElectionWire>, StorageError> {
    let input = VerifyCarriedElectionInput {
        id: id.to_string(),
        link_record,
    };
    let payload = rmp_serde::to_vec_named(&input).map_err(|e| {
        StorageError::Internal(format!(
            "conductor_writes: encode verify_carried_election: {e}"
        ))
    })?;
    let bytes = hc
        .call_zome(ZOME_NAME, "verify_carried_election", payload)
        .await?;
    let out: Option<CanonicalElectionWire> = rmp_serde::from_slice(&bytes).map_err(|e| {
        StorageError::Serialization(format!(
            "conductor_writes: decode CanonicalElectionWire (verify carried): {e}"
        ))
    })?;
    Ok(out)
}

/// Caller-input wire shape for `content_store::validate_carried_head_record`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidateCarriedHeadRecordInput {
    pub id: String,
    pub expected_action_hash: String,
    #[serde(with = "serde_bytes")]
    pub record: Option<Vec<u8>>,
}

/// Have the ZOME cryptographically validate carried head bytes against an
/// expected action hash (`content_store::validate_carried_head_record`).
///
/// The bytes are NEVER trusted by storage: the wasm side re-derives the
/// action hash, verifies the author signature, checks the entry↔action binding,
/// and enforces the target-id gate. Storage only decides what to do with a
/// PROVEN head.
///
/// `Ok(None)` = nothing was carried. `Err` = the bytes failed a proof, which is
/// a refusal to act, never a reason to stamp anyway.
pub async fn call_validate_carried_head_record(
    hc: &Arc<HcClient>,
    id: &str,
    expected_action_hash: &str,
    record: Option<Vec<u8>>,
) -> Result<Option<ContentHeadWire>, StorageError> {
    let input = ValidateCarriedHeadRecordInput {
        id: id.to_string(),
        expected_action_hash: expected_action_hash.to_string(),
        record,
    };
    let payload = rmp_serde::to_vec_named(&input).map_err(|e| {
        StorageError::Internal(format!(
            "conductor_writes: encode validate_carried_head_record: {e}"
        ))
    })?;
    let bytes = hc
        .call_zome(ZOME_NAME, "validate_carried_head_record", payload)
        .await?;
    let out: Option<ContentHeadWire> = rmp_serde::from_slice(&bytes).map_err(|e| {
        StorageError::Serialization(format!(
            "conductor_writes: decode ContentHeadWire (validate carried): {e}"
        ))
    })?;
    Ok(out)
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
    /// ADOPT-BEFORE-AUTHOR (operator-reserved, default OFF): ask the coordinator
    /// to accept this declaration even when it holds NO local chain for `id`,
    /// PROVIDED [`Self::carried_record`] proves itself in wasm against the
    /// declared target. Mirrors
    /// `content_store::DeclareCanonicalHeadInput.adopt_before_author`.
    ///
    /// Additive in BOTH directions, which is what makes a rolling upgrade safe:
    /// a `false` here is a msgpack `false` an OLD coordinator simply ignores
    /// (serde skips unknown map keys — the struct is not `deny_unknown_fields`),
    /// and a NEW coordinator receiving a payload with the key absent decodes it
    /// to `false` via its own `#[serde(default)]`. Old-happ/new-storage and
    /// new-happ/old-storage therefore both behave exactly as before.
    ///
    /// Never set speculatively: `head_adoption::adopt_before_author_param` sets
    /// it only when the node-level flag is on AND validated-shape carried bytes
    /// are actually in hand, so `true` always travels WITH its evidence.
    #[serde(default)]
    pub adopt_before_author: bool,
    /// Root-author-signed delegation letting a NON-author device declare the
    /// EARNED head (station 3). Mirrors `content_store::HeadDelegation` field
    /// for field — the holochain types are the SAME crate lineage the zome
    /// serializes with, so the MessagePack bytes match without a hand-rolled
    /// mirror. Absent → omitted from the wire (old coordinators ignore it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delegation: Option<HeadDelegationWire>,
}

/// Mirror of `content_store::HeadDelegationPayload` — the bytes the root author
/// signed. Field ORDER matters (it is what `sign`/`verify_signature` serialize),
/// so this must stay identical to the zome struct.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HeadDelegationPayloadWire {
    pub grantor: holochain_types::prelude::AgentPubKey,
    pub delegate: holochain_types::prelude::AgentPubKey,
    pub scope: String,
    pub valid_until: holochain_types::prelude::Timestamp,
}

/// Mirror of `content_store::HeadDelegation`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HeadDelegationWire {
    pub payload: HeadDelegationPayloadWire,
    pub signature: holochain_types::prelude::Signature,
}

/// The HTTP/JSON form of a delegation as a device carries it: agent keys and
/// the signature as base64 strings (the form `grant_head_delegation` answers
/// over the app websocket once encoded for humans), `validUntil` as
/// microseconds. Converted to the MessagePack wire form by
/// [`HeadDelegationJson::into_wire`] — the ONLY place the two meet.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadDelegationJson {
    pub grantor: String,
    pub delegate: String,
    #[serde(default = "HeadDelegationJson::default_scope")]
    pub scope: String,
    pub valid_until: i64,
    /// Standard base64 of the 64 signature bytes.
    pub signature: String,
}

impl HeadDelegationJson {
    fn default_scope() -> String {
        "*".to_string()
    }

    pub fn into_wire(self) -> Result<HeadDelegationWire, StorageError> {
        use base64::Engine as _;
        let grantor = holochain_types::prelude::AgentPubKey::try_from(self.grantor.as_str())
            .map_err(|e| StorageError::InvalidInput(format!("delegation.grantor: {e:?}")))?;
        let delegate = holochain_types::prelude::AgentPubKey::try_from(self.delegate.as_str())
            .map_err(|e| StorageError::InvalidInput(format!("delegation.delegate: {e:?}")))?;
        let sig_bytes = base64::engine::general_purpose::STANDARD
            .decode(self.signature.as_bytes())
            .or_else(|_| {
                base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(self.signature.as_bytes())
            })
            .map_err(|e| StorageError::InvalidInput(format!("delegation.signature: {e}")))?;
        let sig: [u8; 64] = sig_bytes.as_slice().try_into().map_err(|_| {
            StorageError::InvalidInput(format!(
                "delegation.signature: expected 64 bytes, got {}",
                sig_bytes.len()
            ))
        })?;
        Ok(HeadDelegationWire {
            payload: HeadDelegationPayloadWire {
                grantor,
                delegate,
                scope: self.scope,
                valid_until: holochain_types::prelude::Timestamp::from_micros(self.valid_until),
            },
            signature: holochain_types::prelude::Signature(sig),
        })
    }
}

/// Declare the EARNED canonical head of `id` at `head_action_hash` from THIS
/// conductor, as the root author OR as a device carrying the root author's
/// signed [`HeadDelegationWire`] (station 3). Calls the coordinator's
/// `declare_earned_canonical_head`; its Guest errors ("restricted to the root
/// author, a device it delegated, or the bootstrap steward", "head delegation:
/// …") surface verbatim so the HTTP layer can classify them.
pub async fn call_declare_earned_canonical_head(
    hc: &Arc<HcClient>,
    id: &str,
    head_action_hash: String,
    delegation: Option<HeadDelegationWire>,
) -> Result<ContentHeadWire, StorageError> {
    let input = DeclareCanonicalHeadInput {
        id: id.to_string(),
        head_action_hash,
        carried_record: None,
        adopt_before_author: false,
        delegation,
    };
    let payload = rmp_serde::to_vec_named(&input).map_err(|e| {
        StorageError::Internal(format!(
            "conductor_writes: encode DeclareCanonicalHeadInput (earned): {e}"
        ))
    })?;
    let (bytes, _timing) = hc
        .call_zome_timed(
            ZOME_NAME,
            "declare_earned_canonical_head",
            payload,
            DECLARE_DEFAULT_CLASS,
        )
        .await?;
    let out: ContentHeadWire = rmp_serde::from_slice(&bytes).map_err(|e| {
        StorageError::Serialization(format!(
            "conductor_writes: decode ContentHeadWire (declare earned): {e}"
        ))
    })?;
    Ok(out)
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

// ═══════════════════════════════════════════════════════════════════════════
// BATCHED head-plane reads (T1b contract, head-plane trust-gradient program L1)
// ═══════════════════════════════════════════════════════════════════════════
//
// These are the DECODE MIRRORS of `content_store`'s batch externs
// (`resolve_content_heads_local`, `resolve_canonical_elections`). Field names,
// variant names and serde casing MUST match the coordinator EXACTLY — the zome
// source is the authority and
// `tests::batch_wire_mirrors_decode_the_coordinator_shapes` pins it against
// independently-declared replicas of the coordinator structs.
//
// SHAPE (revised by operator review 2026-08-08, superseding T1's first landing):
//   schema_version: u16      — read FIRST; a mismatch is a fallback trigger
//   attempted:      Vec<{ id, outcome: Resolved(Option<T>) | Failed{reason,phase} }>
//   unattempted:    Vec<String>   — NEVER STARTED. Not failures. Ask again.
//   stop_reason:    Option<BudgetExhausted|IdCeiling|SharedFailure(reason)>
//   elapsed_ms:     u32           — IN-WASM wall time (not the round-trip)
//
// The one non-obvious consumer rule: `unattempted` is not evidence about an id
// in ANY direction, and neither is a call-level `Err`. Both mean "ask again",
// never "mark it failed" — see `services::head_batch_resolver`.

/// Schema version this build's mirrors decode. A response carrying anything
/// else is undecodable BY CONTRACT even if serde happens to accept it, and
/// triggers the single-id fallback exactly once (see
/// [`crate::services::head_batch_resolver`]).
pub const BATCH_RESOLVE_SCHEMA_VERSION: u16 = 1;

/// Caller-input wire shape shared by BOTH batch externs
/// (`content_store::BatchResolveHeadsInput` / `BatchResolveElectionsInput` are
/// two distinct coordinator types with one identical shape; one mirror is
/// enough on the encode side and keeps the two call sites honest about being
/// the same request).
///
/// `budget_ms` is the extern's IN-WASM deadline. `None` takes the
/// coordinator's `BATCH_BUDGET_DEFAULT_MS`; any value is clamped coordinator-side
/// to `BATCH_BUDGET_CEILING_MS`. `skip_serializing_if` keeps a `None` off the
/// wire so the coordinator's `#[serde(default)]` sees an absent key.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchResolveInput {
    pub ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_ms: Option<u32>,
}

/// Mirror of `content_store::BatchFailureReason` — the typed per-item failure
/// vocabulary (C14: never free-form text).
///
/// The CONTINUE/STOP disposition lives coordinator-side
/// (`batch_failure_disposition`); what storage needs from the same vocabulary is
/// a DIFFERENT question — does this failure count against the id, or is it
/// backpressure that must return the id to pending? — answered by
/// [`BatchFailureReason::is_backpressure`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchFailureReason {
    /// A link resolved to a valid Content record whose own `id` was NOT the one
    /// requested (poisoned/misrouted link). DETERMINISTIC.
    WrongContentId,
    /// The resolved action was not a Content entry at all. DETERMINISTIC.
    RecordShape,
    /// Conductor DB read-permit acquisition timed out — a SATURATED read pool.
    /// Retryable backpressure, never evidence about the id.
    PermitTimeout,
    /// Network/transport-class failure from a delegated call. Backpressure.
    Transport,
    /// The conductor's own `sys_time()` host call failed. Backpressure.
    ClockUnavailable,
    /// Unrecognised error surface. CONSERVATIVE: treated as backpressure.
    Unknown,
}

impl BatchFailureReason {
    /// Is this failure a shared/conductor-wide backpressure condition rather
    /// than a deterministic fact about the id?
    ///
    /// This is THE routing question for the reconcile arms. A deterministic
    /// reason (the answer will not change if we ask again) may follow the arm's
    /// existing failure accounting; a backpressure reason must return the id to
    /// `pending` — marking it failed would burn `MAX_RETRIES` and poison the
    /// `MissLedger` for a condition that says nothing at all about the id.
    ///
    /// Mirrors the coordinator's `batch_failure_disposition` split exactly
    /// (Continue ⇔ deterministic, Stop ⇔ backpressure); pinned by
    /// `tests::the_backpressure_split_matches_the_coordinator_disposition`.
    pub fn is_backpressure(self) -> bool {
        match self {
            BatchFailureReason::WrongContentId | BatchFailureReason::RecordShape => false,
            BatchFailureReason::PermitTimeout
            | BatchFailureReason::Transport
            | BatchFailureReason::ClockUnavailable
            | BatchFailureReason::Unknown => true,
        }
    }

    /// Countable label (C8). Stable — a dashboard contract.
    pub fn label(self) -> &'static str {
        match self {
            BatchFailureReason::WrongContentId => "wrong_content_id",
            BatchFailureReason::RecordShape => "record_shape",
            BatchFailureReason::PermitTimeout => "permit_timeout",
            BatchFailureReason::Transport => "transport",
            BatchFailureReason::ClockUnavailable => "clock_unavailable",
            BatchFailureReason::Unknown => "unknown",
        }
    }
}

/// Mirror of `content_store::BatchResolvePhase`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchResolvePhase {
    HeadResolve,
    Election,
    Clock,
}

/// Mirror of `content_store::BatchResolveFailure`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BatchResolveFailure {
    pub reason: BatchFailureReason,
    pub phase: BatchResolvePhase,
}

/// Mirror of `content_store::BatchOutcome<T>` — externally tagged, variant names
/// verbatim (`Resolved` / `Failed`), NO `rename_all` on the coordinator side.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum BatchOutcome<T> {
    /// `Some` = a genuine answer; `None` = the SAME honest local absence the
    /// single-id twin reports. Never a failure.
    Resolved(Option<T>),
    Failed(BatchResolveFailure),
}

/// Mirror of `content_store::BatchAttempt<T>`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchAttempt<T> {
    pub id: String,
    pub outcome: BatchOutcome<T>,
}

/// Mirror of `content_store::BatchStopReason` — externally tagged, variant names
/// verbatim.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BatchStopReason {
    /// The in-wasm deadline was exhausted. NOT a failure.
    BudgetExhausted,
    /// More ids than the coordinator's `BATCH_ID_CEILING`. NOT a failure.
    IdCeiling,
    /// A shared/conductor-wide failure stopped admission; the untouched tail is
    /// in `unattempted`.
    SharedFailure(BatchFailureReason),
}

impl BatchStopReason {
    /// Countable label (C8).
    pub fn label(&self) -> &'static str {
        match self {
            BatchStopReason::BudgetExhausted => "budget_exhausted",
            BatchStopReason::IdCeiling => "id_ceiling",
            BatchStopReason::SharedFailure(_) => "shared_failure",
        }
    }
}

/// Mirror of `content_store::BatchResolveHeadsOutput` /
/// `BatchResolveElectionsOutput` — one generic mirror for two coordinator types
/// with an identical shape.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchResolveOutput<T> {
    pub schema_version: u16,
    pub attempted: Vec<BatchAttempt<T>>,
    pub unattempted: Vec<String>,
    pub stop_reason: Option<BatchStopReason>,
    pub elapsed_ms: u32,
}

/// One batch call's decoded output PLUS what the admission gate observed while
/// making it.
///
/// The timing rides along because the pacing controller downstream needs a
/// signal that is monotonic in the thing it controls. Batch size ↑ ⇒ permits
/// held longer ⇒ occupancy ↑ ⇒ `admission_wait` ↑. The older
/// `RTT − in-wasm elapsed_ms` has no such property: it subtracts precisely the
/// interval an extern spends blocked on a conductor read permit, so a conductor
/// stalling *inside* the extern reports ZERO queue-wait and the controller reads
/// that as headroom.
#[derive(Debug, Clone)]
pub struct BatchCall<T> {
    pub out: BatchResolveOutput<T>,
    pub timing: crate::hc_client::ZomeCallTiming,
}

/// Does this conductor error mean "the running coordinator has no such
/// function" — i.e. the batch externs have not been hot-swapped onto this
/// conductor yet (T2 pending)?
///
/// The DNA hash is BLIND to coordinator zomes, so a shipped-but-not-swapped
/// coordinator is indistinguishable from a landed one except by this error. It
/// is the ONE error class that licenses the single-id fallback; everything else
/// (notably `"deadline has elapsed"`, which is DB-pool saturation) is
/// backpressure and must NOT fall back — falling back would multiply the load on
/// an already-saturated conductor by the batch size.
///
/// **Contract test:** `tests::only_unknown_function_licenses_the_fallback`.
pub fn is_unknown_function_error(err: &StorageError) -> bool {
    let text = err.to_string();
    let lowered = text.to_ascii_lowercase();
    lowered.contains("unknown function")
        || lowered.contains("no such function")
        || lowered.contains("function not found")
        || lowered.contains("zomefnnotexists")
        || lowered.contains("zome function not found")
        // holochain's `RibosomeError::ZomeFnNotExists` renders with the pair of
        // words below in its Display text.
        || (lowered.contains("not exist") && lowered.contains("function"))
}

/// BATCHED [`call_resolve_content_head_local`]: many ids, ONE conductor
/// round-trip, bounded by an IN-WASM deadline the coordinator enforces itself.
///
/// The round-trip collapse is the whole point — the per-tick cap and the
/// concurrency do NOT rise, only the yield per round-trip.
///
/// Returns the raw wire output. Interpreting it (per-item outcomes → C4
/// `Answer`s, `unattempted` → back-to-pending, `schema_version` → fallback) is
/// [`crate::services::head_batch_resolver`]'s job, deliberately not this
/// function's: this layer owns ENCODING and DECODING only.
pub async fn call_resolve_content_heads_local(
    hc: &Arc<HcClient>,
    ids: &[String],
    budget_ms: Option<u32>,
) -> Result<BatchCall<ContentHeadWire>, StorageError> {
    let input = BatchResolveInput {
        ids: ids.to_vec(),
        budget_ms,
    };
    let payload = rmp_serde::to_vec_named(&input).map_err(|e| {
        StorageError::Internal(format!(
            "conductor_writes: encode resolve_content_heads_local: {e}"
        ))
    })?;
    let (bytes, timing) = hc
        .call_zome_timed(
            ZOME_NAME,
            "resolve_content_heads_local",
            payload,
            AdmissionClass::Background,
        )
        .await?;
    let out = rmp_serde::from_slice(&bytes).map_err(|e| {
        StorageError::Serialization(format!(
            "conductor_writes: decode BatchResolveOutput<ContentHeadWire>: {e}"
        ))
    })?;
    Ok(BatchCall { out, timing })
}

/// BATCHED [`call_resolve_canonical_election`] — same contract as
/// [`call_resolve_content_heads_local`], answering "what did the DHT elect?"
/// without needing the winner's bytes.
pub async fn call_resolve_canonical_elections(
    hc: &Arc<HcClient>,
    ids: &[String],
    budget_ms: Option<u32>,
) -> Result<BatchCall<CanonicalElectionWire>, StorageError> {
    let input = BatchResolveInput {
        ids: ids.to_vec(),
        budget_ms,
    };
    let payload = rmp_serde::to_vec_named(&input).map_err(|e| {
        StorageError::Internal(format!(
            "conductor_writes: encode resolve_canonical_elections: {e}"
        ))
    })?;
    let (bytes, timing) = hc
        .call_zome_timed(
            ZOME_NAME,
            "resolve_canonical_elections",
            payload,
            AdmissionClass::Background,
        )
        .await?;
    let out = rmp_serde::from_slice(&bytes).map_err(|e| {
        StorageError::Serialization(format!(
            "conductor_writes: decode BatchResolveOutput<CanonicalElectionWire>: {e}"
        ))
    })?;
    Ok(BatchCall { out, timing })
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
    call_declare_content_head_classed(hc, id, head_action_hash, DECLARE_DEFAULT_CLASS).await
}

/// [`call_declare_content_head`] with the caller's admission class.
///
/// A SWEEP declare is not the same caller as an operator's declare route: nobody
/// is waiting on it, it has a cheap deferral path (the next sweep), and a fleet
/// of them queueing on the interactive bound is exactly how a background arm
/// starves the lane a person is standing in. Sweep callers pass
/// [`AdmissionClass::Background`]; the wrapper above keeps
/// [`DECLARE_DEFAULT_CLASS`] for the HTTP path.
///
/// The gate timing is discarded here (as in [`call_resolve_content_heads_local`]
/// the declare is a single write, not a batch a controller paces).
pub async fn call_declare_content_head_classed(
    hc: &Arc<HcClient>,
    id: &str,
    head_action_hash: Option<String>,
    class: AdmissionClass,
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
    let (bytes, _timing) = hc
        .call_zome_timed(ZOME_NAME, "declare_content_head", payload, class)
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
/// `adopt_before_author` is the operator-reserved bypass of the coordinator's
/// target-independent no-chain gate; pass `false` for the classic behaviour.
/// Callers MUST NOT set it without carried bytes in hand — see
/// [`DeclareCanonicalHeadInput::adopt_before_author`] and
/// `head_adoption::adopt_before_author_param`.
pub async fn call_declare_canonical_content_head(
    hc: &Arc<HcClient>,
    id: &str,
    head_action_hash: String,
    carried_record: Option<Vec<u8>>,
    adopt_before_author: bool,
) -> Result<ContentHeadWire, StorageError> {
    call_declare_canonical_content_head_classed(
        hc,
        id,
        head_action_hash,
        carried_record,
        adopt_before_author,
        DECLARE_DEFAULT_CLASS,
    )
    .await
}

/// [`call_declare_canonical_content_head`] with the caller's admission class.
///
/// The sweep-side contest/adopt arms in [`crate::services::head_adoption`] pass
/// [`AdmissionClass::Background`]: they are reconciler work with a cheap
/// deferral path (the next sweep), so they take the short wait bound and shed
/// rather than occupy the lane an operator's declare route is standing in. A
/// shed is NOT a verdict — nothing was dispatched — and callers must route it
/// like backpressure (see [`crate::conductor_admission::is_admission_shed`]).
///
/// Everything else is byte-identical to the un-classed wrapper: same input, same
/// verbatim error strings (callers still match the coordinator's guard
/// substrings), same decode.
pub async fn call_declare_canonical_content_head_classed(
    hc: &Arc<HcClient>,
    id: &str,
    head_action_hash: String,
    carried_record: Option<Vec<u8>>,
    adopt_before_author: bool,
    class: AdmissionClass,
) -> Result<ContentHeadWire, StorageError> {
    let input = DeclareCanonicalHeadInput {
        id: id.to_string(),
        head_action_hash,
        carried_record,
        adopt_before_author,
        // The STAGING/adopt path never carries a device delegation — that is
        // the EARNED path's concern (`call_declare_earned_canonical_head`).
        delegation: None,
    };
    let payload = rmp_serde::to_vec_named(&input).map_err(|e| {
        StorageError::Internal(format!(
            "conductor_writes: encode DeclareCanonicalHeadInput: {e}"
        ))
    })?;
    let (bytes, _timing) = hc
        .call_zome_timed(ZOME_NAME, "declare_canonical_content_head", payload, class)
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

/// Wire mirror of one `mishpat::commitments::CommitmentStateLink` — a
/// `CommitmentByState` link projected to its wire shape by the coordinator.
///
/// Every field is a `String` here (unlike [`GetCommitmentOutput`]) because the
/// coordinator renders them so: `state`/`signed_at` are parsed out of the
/// LinkTag (`"<state>|<signed_at>"`) and `event_hash` is the link target
/// already base64-encoded via HoloHash `Display`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommitmentStateLink {
    /// The lifecycle state this transition records, e.g. `"active"` / `"revoked"`.
    pub state: String,
    /// The deterministic, caller-supplied signing time of the transition.
    pub signed_at: String,
    /// Base64 `ActionHash` of the action that justifies the transition.
    pub event_hash: String,
}

/// Read every `CommitmentByState` link off a commitment's anchor through THIS
/// peer's own conductor (`mishpat::get_commitment_state_links`).
///
/// The sibling of [`get_commitment`], and the C5 rail for LIFECYCLE: the
/// commitment body says what the commitment is, these links say what has since
/// been done to it. An empty `Vec` is an ANSWER ("this conductor's DHT view
/// carries no transition for that anchor"), never an outage — an outage is
/// `Err`, and callers must keep the two apart (C4).
pub async fn get_commitment_state_links(
    hc: &Arc<HcClient>,
    cid: &str,
) -> Result<Vec<CommitmentStateLink>, StorageError> {
    let payload = rmp_serde::to_vec_named(&cid.to_string()).map_err(|e| {
        StorageError::Internal(format!(
            "conductor_writes: encode get_commitment_state_links cid: {e}"
        ))
    })?;
    let bytes = hc
        .call_zome_mishpat(MISHPAT_ZOME, "get_commitment_state_links", payload)
        .await?;
    let out: Vec<CommitmentStateLink> = rmp_serde::from_slice(&bytes).map_err(|e| {
        StorageError::Serialization(format!(
            "conductor_writes: decode Vec<CommitmentStateLink>: {e}"
        ))
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

    /// The short declare signatures are the HTTP routes' calling convention, and
    /// those are deliberate operator acts with a person waiting on the answer —
    /// so the class they delegate with must stay INTERACTIVE. A background
    /// caller that wants the short wait bound names it explicitly through the
    /// `_classed` variants (see `head_adoption::SWEEP_DECLARE_CLASS`); flipping
    /// this constant instead would move the HTTP declare routes onto the 1s
    /// bound without touching a single call site.
    #[test]
    fn the_unclassed_declare_wrappers_stay_interactive() {
        assert_eq!(
            super::DECLARE_DEFAULT_CLASS,
            crate::conductor_admission::AdmissionClass::Interactive
        );
    }
}

#[cfg(test)]
mod batch_wire_contract_tests {
    //! DECODE MIRRORS pinned against INDEPENDENTLY-DECLARED replicas of the
    //! coordinator's own structs.
    //!
    //! The replicas below are transcribed from
    //! `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` (the T1b
    //! contract revision, 2026-08-08). They are deliberately a SECOND
    //! declaration rather than an import: the zome crate is a WASM-target
    //! workspace this crate cannot depend on, so the only way to catch a
    //! field-name or serde-casing drift at `cargo test` time is to encode from
    //! a replica of the coordinator shape and decode into the production
    //! mirror. A rename on either side fails these tests.

    use super::{
        is_unknown_function_error, BatchFailureReason, BatchOutcome, BatchResolveOutput,
        BatchResolvePhase, BatchStopReason, BATCH_RESOLVE_SCHEMA_VERSION,
    };
    use crate::error::StorageError;
    use serde::{Deserialize, Serialize};

    // ── Coordinator-side replicas (transcribed from content_store) ───────────

    #[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
    #[serde(rename_all = "snake_case")]
    enum ZomeBatchFailureReason {
        WrongContentId,
        RecordShape,
        PermitTimeout,
        Transport,
        ClockUnavailable,
        Unknown,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
    #[serde(rename_all = "snake_case")]
    enum ZomeBatchResolvePhase {
        HeadResolve,
        Election,
        Clock,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    struct ZomeBatchResolveFailure {
        reason: ZomeBatchFailureReason,
        phase: ZomeBatchResolvePhase,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    enum ZomeBatchOutcome<T> {
        Resolved(Option<T>),
        Failed(ZomeBatchResolveFailure),
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct ZomeBatchAttempt<T> {
        id: String,
        outcome: ZomeBatchOutcome<T>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    enum ZomeBatchStopReason {
        BudgetExhausted,
        IdCeiling,
        SharedFailure(ZomeBatchFailureReason),
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct ZomeBatchResolveOutput<T> {
        schema_version: u16,
        attempted: Vec<ZomeBatchAttempt<T>>,
        unattempted: Vec<String>,
        stop_reason: Option<ZomeBatchStopReason>,
        elapsed_ms: u32,
    }

    /// A stand-in payload type: the per-item answer shape is exercised
    /// separately by `ContentHeadWire`'s own round-trip tests, and using a
    /// tiny type here keeps THIS test about the ENVELOPE (attempted /
    /// unattempted / stop_reason / schema_version), which is the part the
    /// contract revision changed.
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    struct Payload {
        marker: String,
    }

    /// The envelope decodes field-for-field, INCLUDING a `stop_reason` present
    /// case (the field a caller must read to tell backpressure apart from an
    /// on-schedule budget yield).
    #[test]
    fn batch_wire_mirrors_decode_the_coordinator_shapes() {
        let zome = ZomeBatchResolveOutput {
            schema_version: BATCH_RESOLVE_SCHEMA_VERSION,
            attempted: vec![
                ZomeBatchAttempt {
                    id: "id-present".to_string(),
                    outcome: ZomeBatchOutcome::Resolved(Some(Payload {
                        marker: "head".to_string(),
                    })),
                },
                ZomeBatchAttempt {
                    id: "id-absent".to_string(),
                    outcome: ZomeBatchOutcome::Resolved(None),
                },
                ZomeBatchAttempt {
                    id: "id-failed".to_string(),
                    outcome: ZomeBatchOutcome::Failed(ZomeBatchResolveFailure {
                        reason: ZomeBatchFailureReason::WrongContentId,
                        phase: ZomeBatchResolvePhase::HeadResolve,
                    }),
                },
            ],
            unattempted: vec!["id-never-started".to_string()],
            stop_reason: Some(ZomeBatchStopReason::SharedFailure(
                ZomeBatchFailureReason::PermitTimeout,
            )),
            elapsed_ms: 3_912,
        };

        let bytes = rmp_serde::to_vec_named(&zome).expect("encode coordinator shape");
        let decoded: BatchResolveOutput<Payload> = rmp_serde::from_slice(&bytes)
            .expect("the storage mirror must decode coordinator bytes");

        assert_eq!(decoded.schema_version, BATCH_RESOLVE_SCHEMA_VERSION);
        assert_eq!(decoded.elapsed_ms, 3_912);
        assert_eq!(decoded.unattempted, vec!["id-never-started".to_string()]);
        assert_eq!(
            decoded.stop_reason,
            Some(BatchStopReason::SharedFailure(
                BatchFailureReason::PermitTimeout
            )),
            "stop_reason must survive the trip — it is how the caller tells conductor \
             backpressure apart from an on-schedule budget yield"
        );
        assert_eq!(decoded.attempted.len(), 3);
        assert_eq!(decoded.attempted[0].id, "id-present");
        assert!(matches!(
            &decoded.attempted[0].outcome,
            BatchOutcome::Resolved(Some(p)) if p.marker == "head"
        ));
        assert!(matches!(
            decoded.attempted[1].outcome,
            BatchOutcome::Resolved(None)
        ));
        match &decoded.attempted[2].outcome {
            BatchOutcome::Failed(f) => {
                assert_eq!(f.reason, BatchFailureReason::WrongContentId);
                assert_eq!(f.phase, BatchResolvePhase::HeadResolve);
            }
            other => panic!("expected a typed per-item failure, got {other:?}"),
        }
    }

    /// The other two `stop_reason` arms are unit variants — a different
    /// MessagePack encoding from the newtype `SharedFailure`, so they need
    /// their own pin.
    #[test]
    fn unit_stop_reasons_decode_too() {
        for (zome, expected) in [
            (
                ZomeBatchStopReason::BudgetExhausted,
                BatchStopReason::BudgetExhausted,
            ),
            (ZomeBatchStopReason::IdCeiling, BatchStopReason::IdCeiling),
        ] {
            let out = ZomeBatchResolveOutput::<Payload> {
                schema_version: BATCH_RESOLVE_SCHEMA_VERSION,
                attempted: Vec::new(),
                unattempted: Vec::new(),
                stop_reason: Some(zome),
                elapsed_ms: 0,
            };
            let bytes = rmp_serde::to_vec_named(&out).expect("encode");
            let decoded: BatchResolveOutput<Payload> =
                rmp_serde::from_slice(&bytes).expect("decode");
            assert_eq!(decoded.stop_reason, Some(expected));
        }
    }

    /// A `None` stop_reason (every id attempted) round-trips as `None` —
    /// the quiescent case, and the one a `skip_serializing_if` mistake would
    /// break silently.
    #[test]
    fn an_absent_stop_reason_stays_absent() {
        let out = ZomeBatchResolveOutput::<Payload> {
            schema_version: BATCH_RESOLVE_SCHEMA_VERSION,
            attempted: Vec::new(),
            unattempted: Vec::new(),
            stop_reason: None,
            elapsed_ms: 7,
        };
        let bytes = rmp_serde::to_vec_named(&out).expect("encode");
        let decoded: BatchResolveOutput<Payload> = rmp_serde::from_slice(&bytes).expect("decode");
        assert!(decoded.stop_reason.is_none());
    }

    /// The typed reason vocabulary must survive by NAME, not by ordinal — a
    /// reordered coordinator enum would otherwise silently re-label every
    /// failure.
    #[test]
    fn every_failure_reason_decodes_by_name() {
        let pairs = [
            (
                ZomeBatchFailureReason::WrongContentId,
                BatchFailureReason::WrongContentId,
            ),
            (
                ZomeBatchFailureReason::RecordShape,
                BatchFailureReason::RecordShape,
            ),
            (
                ZomeBatchFailureReason::PermitTimeout,
                BatchFailureReason::PermitTimeout,
            ),
            (
                ZomeBatchFailureReason::Transport,
                BatchFailureReason::Transport,
            ),
            (
                ZomeBatchFailureReason::ClockUnavailable,
                BatchFailureReason::ClockUnavailable,
            ),
            (ZomeBatchFailureReason::Unknown, BatchFailureReason::Unknown),
        ];
        for (zome, expected) in pairs {
            let bytes = rmp_serde::to_vec_named(&zome).expect("encode");
            let decoded: BatchFailureReason = rmp_serde::from_slice(&bytes).expect("decode");
            assert_eq!(decoded, expected, "reason must decode by NAME");
        }
    }

    /// The storage-side routing split must mirror the coordinator's
    /// CONTINUE/STOP disposition exactly. Two vocabularies for one fact is how
    /// a backpressure failure gets counted as an id's fault.
    #[test]
    fn the_backpressure_split_matches_the_coordinator_disposition() {
        // Coordinator: Continue for the deterministic pair, Stop for the rest.
        assert!(!BatchFailureReason::WrongContentId.is_backpressure());
        assert!(!BatchFailureReason::RecordShape.is_backpressure());
        assert!(BatchFailureReason::PermitTimeout.is_backpressure());
        assert!(BatchFailureReason::Transport.is_backpressure());
        assert!(BatchFailureReason::ClockUnavailable.is_backpressure());
        assert!(
            BatchFailureReason::Unknown.is_backpressure(),
            "an unrecognised reason must fail toward backpressure (return the id to pending), \
             never toward blaming the id"
        );
    }

    /// ONLY an unknown-function error licenses the single-id fallback.
    /// A DB-pool saturation error MUST NOT — falling back there would multiply
    /// the load on an already-saturated conductor by the batch size, which is
    /// the exact failure this whole program exists to avoid.
    #[test]
    fn only_unknown_function_licenses_the_fallback() {
        let unknown = StorageError::Internal(
            "Zome call failed: RibosomeError: Wasm error ... Zome function \
             resolve_content_heads_local does not exist"
                .to_string(),
        );
        assert!(is_unknown_function_error(&unknown));

        let saturated = StorageError::Internal(
            "Zome call failed: WasmError { error: Host(\"deadline has elapsed\") }".to_string(),
        );
        assert!(
            !is_unknown_function_error(&saturated),
            "DB read-permit saturation is BACKPRESSURE — falling back to single-id would \
             multiply the load on a conductor that is already refusing work"
        );

        let ws = StorageError::Internal("Zome call failed: Websocket error: Timeout".to_string());
        assert!(!is_unknown_function_error(&ws));
    }
}
