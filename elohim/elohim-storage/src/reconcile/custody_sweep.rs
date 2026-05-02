//! T23: Custody reconcile sweep — tokio orchestration around
//! [`crate::reconcile::custody::reconcile_pass`].
//!
//! The reconcile pass itself is sync and trait-driven (`LocalBlobStore`,
//! `FetchKicker`) so it stays unit-testable. Production wiring happens here:
//!
//! - [`RaceFetchKicker`] adapts the sync `FetchKicker` trait onto the async
//!   `blob_fetch::race_fetch` helper. Each `kick()` call spawns a detached
//!   tokio task that races the candidate set, persists bytes on `Hit` via
//!   `finalize_fetch_success`, and otherwise drops the result.
//! - The driving timer arm + `ConnectionEstablished` trigger live on
//!   [`crate::p2p::P2PNode::run_custody_reconcile`], which builds a
//!   [`super::custody::BlobStoreSnapshot`] + a `RaceFetchKicker`, calls
//!   `reconcile_pass`, and folds the [`super::custody::ReconcileOutcome`]
//!   into [`crate::p2p::ReconciliationMetrics`].
//!
//! The `kicks_fired_total` atomic is incremented synchronously _before_
//! `tokio::spawn` is called. This counts work _scheduled_ at kick time,
//! including kicks that race-fetch later finds have no connected
//! candidates (NoCandidates outcome). The metric serves as a backpressure
//! signal independent of tokio scheduling. Note that semaphore-dropped
//! kicks are NOT counted (they didn't reach the metric increment site).

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::blob_store::BlobStore;
use crate::db::DbPool;
use crate::p2p::blob_fetch::{finalize_fetch_success, race_fetch, FetchOutcome};
use crate::p2p::{P2PCommand, ReconciliationMetrics};
use crate::reconcile::custody::FetchKicker;

/// Production [`FetchKicker`] adapter: races candidates over the
/// `/elohim/blob/1.0.0` request-response protocol via
/// [`crate::p2p::blob_fetch::race_fetch`], persists bytes on hit, and
/// records every kick into the shared `kicks_fired_total` counter.
///
/// `connected_peers` is the same [`DashMap`] tracked by
/// [`crate::p2p::P2PNode`] so the `is_connected` filter passed to
/// `race_fetch` always reflects the current swarm state at kick time, not
/// at sweep-pass-construction time.
pub struct RaceFetchKicker {
    /// Sender into the swarm command channel; clones are cheap.
    pub command_tx: mpsc::Sender<P2PCommand>,
    /// Shared peer-metrics map; entries are inserted on
    /// `SwarmEvent::ConnectionEstablished` and removed on close.
    pub connected_peers: Arc<DashMap<String, crate::p2p::PeerMetrics>>,
    /// Pool used to acquire a connection for `finalize_fetch_success` on hit.
    pub db_pool: DbPool,
    /// Local blob store; bytes-on-hit are persisted here (filesystem first,
    /// then SQL — see `finalize_fetch_success` ordering contract).
    pub blob_store: Arc<BlobStore>,
    /// CID of this peer's steward; written into the `serve-blob` REA event.
    pub self_cid: String,
    /// Per-batch parallelism bound for `race_fetch`.
    pub fetch_blob_parallelism: usize,
    /// Per-peer timeout for `race_fetch` (seconds).
    pub fetch_blob_timeout_seconds: u64,
    /// Shared metrics — `kicks_fired_total` is incremented once per
    /// scheduled kick. The whole `Arc` is cloned (cheap) into the spawned
    /// task so the atomic survives even if `RaceFetchKicker` is dropped
    /// between `kick()` and resolution.
    pub metrics: Arc<ReconciliationMetrics>,
    /// Bounds the number of concurrently in-flight spawned kicks.
    /// When saturated, additional kicks are dropped with a debug log; the
    /// next sweep tick will retry. Sized at `fetch_blob_parallelism * 4`
    /// by default (12 with default parallelism=3).
    ///
    /// **Scope:** the semaphore is constructed per-reconcile-pass in
    /// [`crate::p2p::P2PNode::run_custody_reconcile`] (not stored on
    /// `P2PNode`). This means the bound applies within a single pass's
    /// spawned tasks; overlapping passes (timer + ConnectionEstablished
    /// trigger collision) each get their own fresh full semaphore. With
    /// the T23 review-fix #2 change that routes ConnectionEstablished
    /// through `P2PCommand::TriggerCustodyReconcile`, reconciliation is
    /// serialized on a single execution lane, so in-pass-only bounding
    /// is the correct shape.
    pub kick_semaphore: Arc<tokio::sync::Semaphore>,
}

impl FetchKicker for RaceFetchKicker {
    fn kick(&self, blob_hash: &str, candidates: Vec<String>) {
        // T23 review fix #1: bound concurrent in-flight kicks via a
        // semaphore. Under reconnect bursts (cluster restart) an unbounded
        // spawn-per-(missing × candidate) explosion could exhaust the r2d2
        // pool and overload the scheduler. When saturated we drop the kick
        // and log; the next sweep tick will retry. Dropped kicks are NOT
        // counted in `kicks_fired_total` (the metric increments below this
        // gate), so the metric reflects work actually scheduled.
        let permit = match self.kick_semaphore.clone().try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                debug!(
                    target: "elohim_storage::reconcile",
                    hash = %blob_hash,
                    "T23: kick backpressure — semaphore saturated, dropping; next sweep will retry"
                );
                return;
            }
        };

        let cmd_tx = self.command_tx.clone();
        let connected = self.connected_peers.clone();
        let pool = self.db_pool.clone();
        let blob_store = self.blob_store.clone();
        let self_cid = self.self_cid.clone();
        let parallelism = self.fetch_blob_parallelism.max(1);
        let timeout = Duration::from_secs(self.fetch_blob_timeout_seconds.max(1));
        let metrics = self.metrics.clone();
        let hash_owned = blob_hash.to_string();

        // The kick atomic is incremented before scheduling so a saturated
        // tokio runtime cannot drop the count silently.
        metrics.kicks_fired_total.fetch_add(1, Ordering::Relaxed);

        tokio::spawn(async move {
            // Hold the semaphore permit for the lifetime of the spawned
            // task; it drops (releasing one slot) when this future
            // resolves, making the bound a true in-flight cap.
            let _permit = permit;
            // Snapshot connected peers at task start (post-spawn).
            // Disconnects racing with this snapshot cause benign fetch
            // misses; reconnects are picked up on the next sweep tick.
            // DashMap iter is atomic at entry granularity. `race_fetch`
            // filters its candidate batch through this `is_connected`
            // closure so disconnects mid-batch are observed but
            // reconnects mid-batch are not (acceptable; the next sweep
            // tick picks up new peers).
            let connected_set: std::collections::HashSet<String> = connected
                .iter()
                .filter_map(|e| {
                    if e.value().is_connected {
                        Some(e.key().clone())
                    } else {
                        None
                    }
                })
                .collect();
            let is_connected = move |peer: &str| connected_set.contains(peer);

            let outcome = race_fetch(
                &hash_owned,
                candidates,
                &cmd_tx,
                is_connected,
                parallelism,
                timeout,
            )
            .await;

            match outcome {
                FetchOutcome::Hit { bytes, source_peer } => {
                    let mut conn = match pool.get() {
                        Ok(c) => c,
                        Err(e) => {
                            warn!(
                                target: "elohim_storage::reconcile",
                                hash = %hash_owned,
                                error = %e,
                                "T23: pool exhausted; cannot finalize kicked fetch"
                            );
                            return;
                        }
                    };
                    if let Err(e) = finalize_fetch_success(
                        &mut conn,
                        &hash_owned,
                        &source_peer,
                        &bytes,
                        &self_cid,
                        &blob_store,
                    )
                    .await
                    {
                        warn!(
                            target: "elohim_storage::reconcile",
                            hash = %hash_owned,
                            error = %e,
                            "T23: finalize_fetch_success failed for kicked fetch"
                        );
                    } else {
                        debug!(
                            target: "elohim_storage::reconcile",
                            hash = %hash_owned,
                            source_peer = %source_peer,
                            size = bytes.len(),
                            "T23: race-fetch hit, blob persisted"
                        );
                    }
                }
                FetchOutcome::Miss => {
                    debug!(
                        target: "elohim_storage::reconcile",
                        hash = %hash_owned,
                        "T23: race-fetch miss after exhausting candidates"
                    );
                }
                FetchOutcome::NoCandidates => {
                    debug!(
                        target: "elohim_storage::reconcile",
                        hash = %hash_owned,
                        "T23: race-fetch found no connected candidates"
                    );
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::SqliteConnection;

    fn test_pool() -> DbPool {
        let url = format!(
            "file:t23_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    /// `kick()` must increment the atomic synchronously and return without
    /// blocking on the spawned task. The spawned task itself races against
    /// an empty connected-peer set, which yields `NoCandidates` quickly —
    /// but that resolution is irrelevant to this assertion.
    #[tokio::test]
    async fn kick_increments_counter_synchronously_and_returns() {
        let (cmd_tx, _cmd_rx) = mpsc::channel::<P2PCommand>(8);
        let metrics = Arc::new(ReconciliationMetrics::default());
        let kicker = RaceFetchKicker {
            command_tx: cmd_tx,
            connected_peers: Arc::new(DashMap::new()),
            db_pool: test_pool(),
            blob_store: Arc::new(BlobStore::new_memory()),
            self_cid: "self-cid-fixture".into(),
            fetch_blob_parallelism: 2,
            fetch_blob_timeout_seconds: 1,
            metrics: metrics.clone(),
            kick_semaphore: Arc::new(tokio::sync::Semaphore::new(8)),
        };

        kicker.kick(
            "sha256-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            vec!["peer-A".into(), "peer-B".into()],
        );
        // Atomic increments before tokio::spawn returns.
        assert_eq!(metrics.kicks_fired_total.load(Ordering::Relaxed), 1);

        kicker.kick(
            "sha256-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            vec![],
        );
        assert_eq!(metrics.kicks_fired_total.load(Ordering::Relaxed), 2);

        // Yield so the spawned tasks can resolve to NoCandidates without
        // leaking into other tests.
        tokio::task::yield_now().await;
    }

    /// T23 review fix #1: when the kick semaphore is saturated (here: zero
    /// permits ever issued), `kick()` must drop the kick — no spawn, no
    /// metric increment. The next sweep tick is the recovery path.
    #[tokio::test]
    async fn kick_dropped_when_semaphore_saturated() {
        let (cmd_tx, _cmd_rx) = mpsc::channel::<P2PCommand>(8);
        let metrics = Arc::new(ReconciliationMetrics::default());
        let kicker = RaceFetchKicker {
            command_tx: cmd_tx,
            connected_peers: Arc::new(DashMap::new()),
            db_pool: test_pool(),
            blob_store: Arc::new(BlobStore::new_memory()),
            self_cid: "self-cid-fixture".into(),
            fetch_blob_parallelism: 2,
            fetch_blob_timeout_seconds: 1,
            metrics: metrics.clone(),
            // Zero permits: every try_acquire_owned() fails immediately.
            kick_semaphore: Arc::new(tokio::sync::Semaphore::new(0)),
        };

        kicker.kick(
            "sha256-cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            vec!["peer-A".into()],
        );

        // Yield so any (unexpected) spawned task could run; semaphore drop
        // is synchronous so this assertion would already hold without the
        // yield, but we want to be defensive about scheduling races.
        tokio::task::yield_now().await;

        assert_eq!(
            metrics.kicks_fired_total.load(Ordering::Relaxed),
            0,
            "saturated-semaphore kicks must NOT increment kicks_fired_total"
        );
    }
}
