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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeclareCanonicalHeadInput {
    pub id: String,
    pub head_action_hash: String,
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
pub async fn call_declare_canonical_content_head(
    hc: &Arc<HcClient>,
    id: &str,
    head_action_hash: String,
) -> Result<ContentHeadWire, StorageError> {
    let input = DeclareCanonicalHeadInput {
        id: id.to_string(),
        head_action_hash,
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GetCommitmentOutput {
    /// Base64 action hash — the value the projection stores as `dht_anchor_hash`.
    pub action_hash: String,
    /// Base64 entry hash — the storage `cid`.
    pub entry_hash: String,
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
    hc.call_zome(MISHPAT_ZOME, "create_commitment", payload)
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
    hc.call_zome(MISHPAT_ZOME, "create_commitment_state_link", payload)
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
        .call_zome(MISHPAT_ZOME, "get_commitment", payload)
        .await?;
    let out: Option<GetCommitmentOutput> = rmp_serde::from_slice(&bytes).map_err(|e| {
        StorageError::Serialization(format!("conductor_writes: decode GetCommitmentOutput: {e}"))
    })?;
    Ok(out)
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
