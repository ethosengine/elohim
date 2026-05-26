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
    hc.call_zome(ZOME_NAME, "create_rea_commitment", payload).await
}
