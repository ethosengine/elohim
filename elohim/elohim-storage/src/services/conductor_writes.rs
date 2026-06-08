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
}
