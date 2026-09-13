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
use std::time::{Duration, Instant};

use dashmap::DashMap;
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::blob_store::BlobStore;
use crate::db::DbPool;
use crate::p2p::blob_fetch::{finalize_fetch_success, race_fetch, FetchOutcome};
use crate::p2p::{P2PCommand, ReconciliationMetrics};
use crate::reconcile::custody::FetchKicker;

/// Default cooldown when `custody_sweep_seconds` is unset or disabled (0).
/// Mirrors the `run()` sweep-timer default so the gate and the timer agree on
/// what "the declared cadence" means.
pub const DEFAULT_KICK_COOLDOWN_SECONDS: u64 = 120;

/// State of one blob hash inside the [`KickGate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KickState {
    /// A race is running for this hash; no second race until it resolves.
    InFlight,
    /// The last race resolved; the next kick is admitted at this instant.
    CoolingUntil(Instant),
}

/// Cross-pass idempotency gate for fetch kicks: **one in-flight race per blob
/// hash, then a cooldown before the same hash may be raced again.**
///
/// [`super::custody::reconcile_pass`] is deliberately stateless: every pass
/// re-derives the full set of own-provider commitments whose bytes are missing
/// locally and asks the [`FetchKicker`] to heal each one. That is a correct
/// *decision*, but it is not a *schedule*. [`RaceFetchKicker::kick`] spawns a
/// DETACHED task per kick, so a pass returns long before its races resolve,
/// and `kick_semaphore` is rebuilt fresh on every pass in
/// `P2PNode::run_custody_reconcile` — so nothing carried the knowledge that a
/// race for this exact hash was already running.
///
/// Passes are event-triggered (`ConnectionEstablished` →
/// `P2PCommand::TriggerCustodyReconcile`) as well as timed, so the declared
/// `custody_sweep_seconds` cadence is NOT the real pass rate. On the
/// 2026-09-13 household mesh that produced a self-sustaining loop: three peers
/// each re-kicked the same 4–5 unfetchable 58 MB shards on every pass, every
/// kick made the holder serve 58 MB, the serving wedged the HTTP listener, the
/// resulting timeouts churned connections, and the churn triggered the next
/// pass. Serving rate went 4/min → 12/min and the holder sat at ~208% CPU with
/// `/health` timing out.
///
/// The gate supplies the missing schedule. Healing is unchanged in the limit:
/// a genuinely missing blob is still retried once per cadence, forever — the
/// gate removes repetition, never persistence.
pub struct KickGate {
    entries: DashMap<String, KickState>,
    cooldown: Duration,
}

impl KickGate {
    /// Build a gate whose post-race cooldown is `cooldown` (the declared
    /// sweep cadence).
    pub fn new(cooldown: Duration) -> Self {
        Self {
            entries: DashMap::new(),
            cooldown,
        }
    }

    /// The configured post-race cooldown.
    pub fn cooldown(&self) -> Duration {
        self.cooldown
    }

    /// Admit a kick for `blob_hash` at `now`, marking it in-flight, or refuse
    /// because a race is already running or its cooldown has not elapsed.
    ///
    /// Atomic per hash via the `DashMap` entry API: two passes racing on the
    /// same hash cannot both be admitted.
    pub fn try_begin(&self, blob_hash: &str, now: Instant) -> bool {
        use dashmap::mapref::entry::Entry;
        match self.entries.entry(blob_hash.to_string()) {
            Entry::Occupied(mut occupied) => match *occupied.get() {
                KickState::InFlight => false,
                KickState::CoolingUntil(until) => {
                    if now >= until {
                        occupied.insert(KickState::InFlight);
                        true
                    } else {
                        false
                    }
                }
            },
            Entry::Vacant(vacant) => {
                vacant.insert(KickState::InFlight);
                true
            }
        }
    }

    /// Release an in-flight kick and open its cooldown window.
    pub fn release(&self, blob_hash: &str, now: Instant) {
        self.entries.insert(
            blob_hash.to_string(),
            KickState::CoolingUntil(now + self.cooldown),
        );
    }

    /// Drop entries whose cooldown has elapsed, so the map tracks only live
    /// custody work rather than every hash ever kicked. Called once per pass.
    pub fn sweep_expired(&self, now: Instant) {
        self.entries.retain(|_, state| match *state {
            KickState::InFlight => true,
            KickState::CoolingUntil(until) => now < until,
        });
    }

    /// Number of hashes currently tracked (in-flight + cooling). Tests + tracing.
    pub fn tracked(&self) -> usize {
        self.entries.len()
    }

    /// Admit a kick and hand back an RAII release. `None` when refused.
    pub fn admit(self: &Arc<Self>, blob_hash: &str, now: Instant) -> Option<KickPermit> {
        if self.try_begin(blob_hash, now) {
            Some(KickPermit {
                gate: self.clone(),
                blob_hash: blob_hash.to_string(),
            })
        } else {
            None
        }
    }
}

/// RAII release for an admitted kick. Dropping it opens the cooldown window,
/// so EVERY exit path of the spawned race task — hit, miss, invalid address,
/// an early `return` on pool exhaustion, or a panic — reopens the hash for a
/// later cadence. A forgotten release would wedge a hash as permanently
/// in-flight and stop healing it, which is the one failure worse than the
/// drumbeat this gate exists to stop.
pub struct KickPermit {
    gate: Arc<KickGate>,
    blob_hash: String,
}

impl Drop for KickPermit {
    fn drop(&mut self) {
        self.gate.release(&self.blob_hash, Instant::now());
    }
}

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
    /// This peer's TRANSPORT id (`Config::self_cid`, libp2p/iroh). Legacy
    /// fallback for the `serve-blob` receiver when the agent_cid is unresolvable.
    pub self_cid: String,
    /// This peer's resolved holochain `agent_cid` (`uhCAk…`), when available.
    /// PREFERRED as the `serve-blob` REA event receiver so the delivery-attribution
    /// row keys on the canonical identity namespace (`humans.agent_pub_key`),
    /// never a transport id. `None` at boot when the conductor cell key was not
    /// resolvable — the write then falls back to `self_cid` (the pre-existing
    /// behavior; a transition, not a regression). See `identity_namespace`.
    pub self_agent_cid: Option<String>,
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
    /// Cross-pass idempotency gate — see [`KickGate`]. Owned by `P2PNode` and
    /// cloned in per pass, UNLIKE `kick_semaphore`, which is rebuilt per pass.
    /// That difference is the whole point: the semaphore bounds concurrency
    /// within one pass; the gate is what stops pass N+1 from re-racing a hash
    /// pass N is still racing.
    pub kick_gate: Arc<KickGate>,
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

        // Cross-pass idempotency: a hash already being raced (or still inside
        // its cooldown) must not be raced again, no matter how many passes the
        // event triggers fire. Checked AFTER the semaphore so a
        // backpressure-dropped kick keeps its existing "next sweep will retry"
        // semantics instead of burning a cooldown window it never used.
        let Some(kick_permit) = self.kick_gate.admit(blob_hash, Instant::now()) else {
            debug!(
                target: "elohim_storage::reconcile",
                hash = %blob_hash,
                cooldown_secs = self.kick_gate.cooldown().as_secs(),
                "T23: kick suppressed — a race for this blob is already in flight \
                 or still within its cooldown; the declared cadence, not the pass \
                 rate, governs retries"
            );
            return;
        };

        let cmd_tx = self.command_tx.clone();
        let connected = self.connected_peers.clone();
        let pool = self.db_pool.clone();
        let blob_store = self.blob_store.clone();
        // Prefer the resolved agent_cid for the serve-blob receiver; fall back to
        // the transport self_cid only when the agent_cid is unresolvable. Never
        // NULL — a receiver is always written.
        let self_cid = self
            .self_agent_cid
            .clone()
            .unwrap_or_else(|| self.self_cid.clone());
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
            // Held for the lifetime of the race. Drop (on ANY exit path,
            // including the early `return`s below) opens the cooldown window.
            let _kick_permit = kick_permit;
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
                // Q3: no bytes, but a peer named a durable manifest for this
                // (composite) hash. Persist it — same write path a direct
                // ingest uses — so a LATER kick (or Q2's local reassembly,
                // once the remaining shards land via ordinary replication)
                // can serve it without repeating this manifest round-trip.
                // Deliberately NOT chaining into a full Q4 swarm shard-fetch
                // here: custody-sweep kicks are a low-priority background
                // nudge already retried every sweep tick, and the
                // interactive callers (`get_blob_or_heal`,
                // `p2p::mod::run_custody_reconcile`'s acquisition pull) are
                // where the user-facing latency actually lives.
                FetchOutcome::Manifest { manifest, .. } => {
                    let mut conn = match pool.get() {
                        Ok(c) => c,
                        Err(e) => {
                            warn!(
                                target: "elohim_storage::reconcile",
                                hash = %hash_owned,
                                error = %e,
                                "T23/Q3: pool exhausted; cannot persist kicked manifest"
                            );
                            return;
                        }
                    };
                    let content_id = format!("blob:{}", manifest.blob_cid);
                    if let Err(e) = crate::db::shard_manifests::record_generated_manifest(
                        &mut conn,
                        &content_id,
                        "lamad",
                        &manifest,
                    ) {
                        warn!(
                            target: "elohim_storage::reconcile",
                            hash = %hash_owned,
                            error = %e,
                            "T23/Q3: failed to persist manifest from kicked fetch"
                        );
                    } else {
                        debug!(
                            target: "elohim_storage::reconcile",
                            hash = %hash_owned,
                            shards = manifest.shard_hashes.len(),
                            "T23/Q3: race-fetch returned a manifest; persisted for later reassembly"
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
                // Terminal for this address: race_fetch refused to put a
                // malformed content address on the wire (and already logged
                // it at WARN). reconcile_pass normalizes markers before
                // kicking, so reaching this arm means a caller bypassed that
                // hygiene — do not retry; the row carrying the address needs
                // healing.
                FetchOutcome::InvalidAddress => {
                    warn!(
                        target: "elohim_storage::reconcile",
                        hash = %hash_owned,
                        "T23: kicked address is not a valid content address; \
                         giving up (no retry can succeed)"
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
            self_agent_cid: None,
            fetch_blob_parallelism: 2,
            fetch_blob_timeout_seconds: 1,
            metrics: metrics.clone(),
            kick_semaphore: Arc::new(tokio::sync::Semaphore::new(8)),
            kick_gate: Arc::new(KickGate::new(Duration::from_secs(120))),
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
            self_agent_cid: None,
            fetch_blob_parallelism: 2,
            fetch_blob_timeout_seconds: 1,
            metrics: metrics.clone(),
            // Zero permits: every try_acquire_owned() fails immediately.
            kick_semaphore: Arc::new(tokio::sync::Semaphore::new(0)),
            kick_gate: Arc::new(KickGate::new(Duration::from_secs(120))),
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

    // ---------------------------------------------------------------
    // KickGate — cross-pass idempotency (2026-09-13 household-mesh loop)
    // ---------------------------------------------------------------

    const H_A: &str = "sha256-1111111111111111111111111111111111111111111111111111111111111111";
    const H_B: &str = "sha256-2222222222222222222222222222222222222222222222222222222222222222";

    /// A second pass must NOT re-race a hash the first pass is still racing.
    /// This is the defect the mesh exposed: `reconcile_pass` re-decides the
    /// same missing blobs every pass and `kick()` spawns a DETACHED task, so
    /// without the gate every pass stacked another 58 MB race on the wire.
    #[test]
    fn kick_gate_refuses_a_second_kick_while_the_first_is_in_flight() {
        let gate = KickGate::new(Duration::from_secs(120));
        let now = Instant::now();

        assert!(gate.try_begin(H_A, now), "first kick must be admitted");
        assert!(
            !gate.try_begin(H_A, now),
            "a race is already in flight for this hash — the second kick must be refused"
        );
        assert!(
            !gate.try_begin(H_A, now + Duration::from_secs(3600)),
            "in-flight is not a timeout: only release() reopens the hash"
        );
        assert!(
            gate.try_begin(H_B, now),
            "the gate is per-hash; an unrelated blob must still be healable"
        );
    }

    /// Once a race resolves, the hash stays closed for the DECLARED cadence —
    /// so the retry rate is the cadence, not the pass rate.
    #[test]
    fn kick_gate_holds_a_resolved_hash_for_the_declared_cadence() {
        let cooldown = Duration::from_secs(120);
        let gate = KickGate::new(cooldown);
        let t0 = Instant::now();

        assert!(gate.try_begin(H_A, t0));
        gate.release(H_A, t0);

        assert!(
            !gate.try_begin(H_A, t0),
            "a just-resolved hash must not be re-raced immediately"
        );
        assert!(
            !gate.try_begin(H_A, t0 + cooldown - Duration::from_secs(1)),
            "still inside the cooldown window"
        );
        assert!(
            gate.try_begin(H_A, t0 + cooldown),
            "healing must resume at the cadence — the gate removes repetition, never persistence"
        );
    }

    /// `sweep_expired` retires cooled entries so the map tracks live custody
    /// work, but must never evict an in-flight entry (that would readmit a
    /// hash whose 58 MB race is still on the wire).
    #[test]
    fn kick_gate_sweep_retires_cooled_entries_but_keeps_in_flight() {
        let gate = KickGate::new(Duration::from_secs(60));
        let t0 = Instant::now();

        assert!(gate.try_begin(H_A, t0));
        assert!(gate.try_begin(H_B, t0));
        gate.release(H_B, t0);
        assert_eq!(gate.tracked(), 2);

        gate.sweep_expired(t0 + Duration::from_secs(120));
        assert_eq!(gate.tracked(), 1, "the cooled entry is retired");
        assert!(
            !gate.try_begin(H_A, t0 + Duration::from_secs(120)),
            "the in-flight entry survived the sweep"
        );
    }

    /// The permit is RAII: dropping it on ANY exit path of the spawned race
    /// (hit, miss, early `return` on pool exhaustion, panic) opens the
    /// cooldown. A leaked permit would wedge the hash as permanently
    /// in-flight and stop healing it entirely.
    #[test]
    fn kick_permit_release_is_raii() {
        let gate = Arc::new(KickGate::new(Duration::ZERO));
        let now = Instant::now();

        {
            let permit = gate.admit(H_A, now).expect("first admit");
            assert!(
                gate.admit(H_A, now).is_none(),
                "held permit blocks a concurrent admit"
            );
            drop(permit);
        }

        assert!(
            gate.admit(H_A, Instant::now()).is_some(),
            "dropping the permit reopened the hash (zero cooldown)"
        );
    }

    /// End-to-end regression for the loop itself: five reconcile passes in
    /// quick succession (what `ConnectionEstablished` churn produced on the
    /// household mesh) must put ONE race on the wire for a given blob, not
    /// five. Before the gate, `kicks_fired_total` was 5 here — five 58 MB
    /// serves the holder had to answer, whose cost caused the churn that
    /// triggered the next pass.
    #[tokio::test]
    async fn repeated_passes_race_a_hash_once_per_cadence() {
        let (cmd_tx, _cmd_rx) = mpsc::channel::<P2PCommand>(8);
        let metrics = Arc::new(ReconciliationMetrics::default());
        let kicker = RaceFetchKicker {
            command_tx: cmd_tx,
            connected_peers: Arc::new(DashMap::new()),
            db_pool: test_pool(),
            blob_store: Arc::new(BlobStore::new_memory()),
            self_cid: "self-cid-fixture".into(),
            self_agent_cid: None,
            fetch_blob_parallelism: 2,
            fetch_blob_timeout_seconds: 1,
            metrics: metrics.clone(),
            kick_semaphore: Arc::new(tokio::sync::Semaphore::new(64)),
            kick_gate: Arc::new(KickGate::new(Duration::from_secs(120))),
        };

        for _ in 0..5 {
            kicker.kick(H_A, vec!["peer-A".into(), "peer-B".into()]);
        }
        assert_eq!(
            metrics.kicks_fired_total.load(Ordering::Relaxed),
            1,
            "five passes over the same missing blob must schedule ONE race"
        );

        // The gate is per-hash: a different missing blob is still healed.
        kicker.kick(H_B, vec!["peer-A".into()]);
        assert_eq!(metrics.kicks_fired_total.load(Ordering::Relaxed), 2);

        tokio::task::yield_now().await;
    }
}
