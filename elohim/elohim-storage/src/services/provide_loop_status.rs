//! Observability for the Slice-2b provide-loop and the re-anchor backfill.
//!
//! ## Why this exists (the dark-card incident)
//!
//! The EPR resilience card (`GET /api/v1/resilience/{id}/household`) read all
//! zeros on alpha. Two app-layer gates, both invisible without log-scraping:
//!
//! 1. **The provide-loop was dormant.** It authors the `replicates-content`
//!    Commitment whose side-projection writes the `content:<reach>` provide rows
//!    the snapshot counts. It spawns only when `config.self_cid` is non-empty
//!    (main.rs), and `self_cid` was sourced solely from the `SELF_CID` env which
//!    is set in no manifest → permanently off, fleet-wide.
//! 2. **The reach circuit latched provenance-only.** The seed/import hit the
//!    in-pod conductor while its cells were still `CellDisabled`, so content rows
//!    landed with `dht_anchor_hash IS NULL` (never DHT-authored) → reach never
//!    re-notarized → no provide rows.
//!
//! This holder surfaces BOTH on `/p2p/status` so "card dark because loop off /
//! circuit latched" is one HTTP read, not a Loki query.
//!
//! Category C (operational): a per-process status snapshot, reconstructed in
//! memory, never persisted, never notarized. Mirrors
//! `p2p::projection_reconcile::ProjectionReconcileState`.

use std::sync::Arc;

use serde::Serialize;
use tokio::sync::RwLock;
use ts_rs::TS;

/// Where `self_cid` came from at boot — the load-bearing gate on whether the
/// provide-loop can spawn at all.
///
/// Serialized as the `selfCidSource` string on `/p2p/status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfCidSource {
    /// `SELF_CID` env was set and non-empty.
    Env,
    /// Derived at startup from the libp2p `NodeIdentity` peer id (the join key
    /// the seeder resolves from `/p2p/status .peerId`).
    DerivedLibp2pPeerId,
    /// Neither — the provide-loop stays dormant (the dark-card cause).
    Unset,
}

impl SelfCidSource {
    pub fn as_str(self) -> &'static str {
        match self {
            SelfCidSource::Env => "env",
            SelfCidSource::DerivedLibp2pPeerId => "derived-libp2p-peer-id",
            SelfCidSource::Unset => "unset",
        }
    }
}

/// Provide-loop + re-anchor-backfill status, exposed via `/p2p/status`.
///
/// Wire format: the `provideLoop` property of
/// `elohim/sdk/schemas/v1/views/p2p-status-view.schema.json` (an inline object,
/// mirroring the `projectionReconcile`/`pull` precedents). Schema contract test:
/// the `p2p_status_view_*` cases in `tests/schema_contract.rs`.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ProvideLoopStatus {
    /// Where `self_cid` came from: `env` | `derived-libp2p-peer-id` | `unset`.
    pub self_cid_source: String,
    /// True once the Slice-2b provide-loop authoring tick has spawned. False
    /// means dormant — the card cannot light its `content:<reach>` counts.
    pub active: bool,
    /// Re-anchor backfill: content rows still lacking `dht_anchor_hash` at the
    /// last sweep (candidates that could not yet be re-authored).
    #[ts(type = "number")]
    pub reanchor_pending: usize,
    /// Re-anchor backfill: rows successfully re-authored this process lifetime.
    #[ts(type = "number")]
    pub reanchor_completed: usize,
    /// Re-anchor backfill: rows that errored re-authoring (non-fatal, retried on
    /// a future boot's sweep).
    #[ts(type = "number")]
    pub reanchor_failed: usize,
    /// True when the re-anchor backfill has run AND found no NULL-anchor rows
    /// left to re-author. False before the first run or while candidates remain.
    pub reanchor_caught_up: bool,
}

impl Default for ProvideLoopStatus {
    fn default() -> Self {
        Self {
            self_cid_source: SelfCidSource::Unset.as_str().to_string(),
            active: false,
            reanchor_pending: 0,
            reanchor_completed: 0,
            reanchor_failed: 0,
            reanchor_caught_up: false,
        }
    }
}

/// Thread-safe holder for the provide-loop status snapshot. Created in the
/// composition root (main.rs), written by the boot path (self_cid derive +
/// loop spawn) and the re-anchor backfill, read by `/p2p/status`.
#[derive(Debug, Clone, Default)]
pub struct ProvideLoopState {
    inner: Arc<RwLock<ProvideLoopStatus>>,
}

impl ProvideLoopState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot for `/p2p/status`.
    pub async fn status(&self) -> ProvideLoopStatus {
        self.inner.read().await.clone()
    }

    /// Record where `self_cid` resolved from (boot path).
    pub async fn set_self_cid_source(&self, source: SelfCidSource) {
        self.inner.write().await.self_cid_source = source.as_str().to_string();
    }

    /// Mark the provide-loop spawned (or confirm it stayed dormant).
    pub async fn set_active(&self, active: bool) {
        self.inner.write().await.active = active;
    }

    /// Publish the result of one re-anchor backfill sweep. `pending` is the
    /// count of NULL-anchor rows that remain after the sweep; `caught_up` is
    /// `pending == 0`. `completed`/`failed` advance the cumulative counters.
    pub async fn publish_reanchor_sweep(&self, completed: usize, failed: usize, pending: usize) {
        let mut s = self.inner.write().await;
        s.reanchor_completed = s.reanchor_completed.saturating_add(completed);
        s.reanchor_failed = s.reanchor_failed.saturating_add(failed);
        s.reanchor_pending = pending;
        s.reanchor_caught_up = pending == 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_cid_source_strings_are_stable_wire_values() {
        assert_eq!(SelfCidSource::Env.as_str(), "env");
        assert_eq!(
            SelfCidSource::DerivedLibp2pPeerId.as_str(),
            "derived-libp2p-peer-id"
        );
        assert_eq!(SelfCidSource::Unset.as_str(), "unset");
    }

    #[test]
    fn default_status_reads_dormant_unset() {
        let s = ProvideLoopStatus::default();
        assert_eq!(s.self_cid_source, "unset");
        assert!(!s.active);
        assert!(!s.reanchor_caught_up);
        assert_eq!(s.reanchor_completed, 0);
    }

    #[tokio::test]
    async fn set_and_snapshot_roundtrip() {
        let state = ProvideLoopState::new();
        state
            .set_self_cid_source(SelfCidSource::DerivedLibp2pPeerId)
            .await;
        state.set_active(true).await;
        state.publish_reanchor_sweep(3, 1, 0).await;

        let snap = state.status().await;
        assert_eq!(snap.self_cid_source, "derived-libp2p-peer-id");
        assert!(snap.active);
        assert_eq!(snap.reanchor_completed, 3);
        assert_eq!(snap.reanchor_failed, 1);
        assert_eq!(snap.reanchor_pending, 0);
        assert!(snap.reanchor_caught_up);
    }

    #[tokio::test]
    async fn reanchor_sweeps_accumulate_completed_and_failed() {
        let state = ProvideLoopState::new();
        state.publish_reanchor_sweep(2, 0, 5).await;
        state.publish_reanchor_sweep(5, 1, 0).await;

        let snap = state.status().await;
        // Cumulative across sweeps.
        assert_eq!(snap.reanchor_completed, 7);
        assert_eq!(snap.reanchor_failed, 1);
        // pending + caught_up reflect the LATEST sweep.
        assert_eq!(snap.reanchor_pending, 0);
        assert!(snap.reanchor_caught_up);
    }
}
