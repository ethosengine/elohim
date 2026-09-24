//! Bounded background worker for Node Registry shard-assignment registration.
//!
//! ## Why this exists
//!
//! `PUT /blob/{hash}` used to store each shard locally, then — for EACH shard,
//! synchronously, before answering — await `NodeRegistryApi::create_shard_assignment`,
//! a conductor zome call. The registration is purely advisory (a failure only
//! `warn!`s; nothing downstream depends on it succeeding), but the PUT could
//! not respond until every one of those calls returned. Under conductor CPU
//! saturation (alpha, 2026-09-23) those calls queued or timed out, and doorway
//! app-bundle PUTs (3-12 MB) blew the doorway's ~33s client timeout and were
//! retried ~100x over 3 hours — bulk delivery held hostage by an advisory
//! side-effect on a lane it was never supposed to share with control traffic.
//!
//! This module moves the registration OFF the request path: `put_blob_bytes`
//! now verifies, stores, and projects the manifest locally exactly as before,
//! then hands each shard's registration to a bounded queue and returns. One
//! worker drains the queue, running each conductor call through the
//! Background admission lane (`AdmissionClass::Background` — see
//! [`crate::conductor_admission`]) so it yields to interactive HTTP traffic
//! under contention instead of competing with it for the same permit pool.
//!
//! ## Bounded, not buffered
//!
//! The queue is a fixed-capacity `mpsc` channel. Past capacity, a new
//! registration is DROPPED and counted rather than queued — the registration
//! is advisory, so silently dropping is correct, and an unbounded queue would
//! only relocate the same storm into process memory (the failure mode we're
//! removing, one layer up). A per-item timeout keeps one stuck registration
//! from parking the worker: it moves on to the next item.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::error::StorageError;
use crate::node_registry_api::{NodeRegistryApi, ShardAssignment};

/// Bound on the pending-registration queue. Sized generously above one PUT's
/// worst-case shard count (an `rs-4-7` manifest chunks large blobs into many
/// 1MB segments x 7 shards each) while staying a fixed, small allocation.
pub const QUEUE_CAPACITY: usize = 512;

/// How long ONE registration call is allowed to run (admission wait + the
/// zome call itself) before the worker gives up on it and moves to the next
/// item. Bounded so a single unresponsive conductor cannot park the worker
/// indefinitely behind one item.
pub const REGISTRATION_TIMEOUT: Duration = Duration::from_secs(10);

/// Seam over the Node Registry zome call. Production wires
/// [`NodeRegistryApi`]; tests can substitute a fake that hangs, errors, or
/// succeeds on demand — the worker's pacing and drop-on-full behavior is what
/// gets exercised, not a live conductor.
#[async_trait::async_trait]
pub trait ShardRegistrar: Send + Sync + 'static {
    async fn register(&self, assignment: ShardAssignment) -> Result<Vec<u8>, StorageError>;
}

#[async_trait::async_trait]
impl ShardRegistrar for NodeRegistryApi {
    /// Runs on the Background admission lane — see
    /// [`NodeRegistryApi::create_shard_assignment`]'s doc comment for why this
    /// call, uniquely among this struct's callers, is no longer Interactive.
    async fn register(&self, assignment: ShardAssignment) -> Result<Vec<u8>, StorageError> {
        self.create_shard_assignment(assignment).await
    }
}

/// The bounded queue + its single drain worker.
pub struct ShardRegistrationQueue {
    tx: mpsc::Sender<ShardAssignment>,
    enqueued: AtomicU64,
    completed: AtomicU64,
    failed: AtomicU64,
    dropped: AtomicU64,
}

impl ShardRegistrationQueue {
    /// Spawn the drain worker and return the handle callers enqueue through.
    /// One worker draining one channel: pacing comes entirely from the
    /// Background admission gate each call runs through, so there is no
    /// second throttle to keep in sync with that one.
    pub fn spawn(registrar: Arc<dyn ShardRegistrar>) -> Arc<Self> {
        let (tx, rx) = mpsc::channel(QUEUE_CAPACITY);
        let queue = Arc::new(Self {
            tx,
            enqueued: AtomicU64::new(0),
            completed: AtomicU64::new(0),
            failed: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
        });
        tokio::spawn(Self::drain(rx, registrar, Arc::clone(&queue)));
        queue
    }

    /// Enqueue a registration. Never blocks and never fails the caller: a
    /// full queue drops the item and counts it rather than pushing
    /// backpressure onto the HTTP response this call was split off from.
    pub fn enqueue(&self, assignment: ShardAssignment) {
        let shard_index = assignment.shard_index;
        match self.tx.try_send(assignment) {
            Ok(()) => {
                self.enqueued.fetch_add(1, Ordering::Relaxed);
                crate::metrics::inc_shard_registration("enqueued");
            }
            Err(_) => {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                crate::metrics::inc_shard_registration("dropped");
                warn!(
                    shard_index,
                    capacity = QUEUE_CAPACITY,
                    "shard registration queue full — dropping advisory Node Registry registration"
                );
            }
        }
    }

    pub fn enqueued_count(&self) -> u64 {
        self.enqueued.load(Ordering::Relaxed)
    }

    pub fn completed_count(&self) -> u64 {
        self.completed.load(Ordering::Relaxed)
    }

    pub fn failed_count(&self) -> u64 {
        self.failed.load(Ordering::Relaxed)
    }

    pub fn dropped_count(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    async fn drain(
        mut rx: mpsc::Receiver<ShardAssignment>,
        registrar: Arc<dyn ShardRegistrar>,
        queue: Arc<Self>,
    ) {
        while let Some(assignment) = rx.recv().await {
            Self::process_one(&registrar, assignment, &queue).await;
        }
    }

    /// Handle exactly one queued item: run it through the registrar with a
    /// hard timeout, then record the outcome on both the process-global
    /// metric and this queue's own counters (so a test can assert without
    /// scraping Prometheus).
    async fn process_one(
        registrar: &Arc<dyn ShardRegistrar>,
        assignment: ShardAssignment,
        queue: &Arc<Self>,
    ) {
        let shard_index = assignment.shard_index;
        let content_hash = assignment.content_hash.clone();
        match tokio::time::timeout(REGISTRATION_TIMEOUT, registrar.register(assignment)).await {
            Ok(Ok(_)) => {
                crate::metrics::inc_shard_registration("completed");
                queue.completed.fetch_add(1, Ordering::Relaxed);
                info!(
                    shard_index,
                    content_hash = %content_hash,
                    "Registered shard assignment with Node Registry (background)"
                );
            }
            Ok(Err(e)) => {
                crate::metrics::inc_shard_registration("failed");
                queue.failed.fetch_add(1, Ordering::Relaxed);
                warn!(
                    error = %e,
                    shard_index,
                    content_hash = %content_hash,
                    "Background shard-assignment registration failed"
                );
            }
            Err(_elapsed) => {
                crate::metrics::inc_shard_registration("failed");
                queue.failed.fetch_add(1, Ordering::Relaxed);
                warn!(
                    shard_index,
                    content_hash = %content_hash,
                    timeout_secs = REGISTRATION_TIMEOUT.as_secs(),
                    "Background shard-assignment registration timed out"
                );
            }
        }
    }

    /// Test-only: build the queue's channel at an arbitrary capacity with NO
    /// drain worker spawned. Returns the receiver too — the caller must hold
    /// it (never `.recv()`ing) to keep the channel from closing, so sends
    /// past `capacity` deterministically hit `Full` instead of `Closed`.
    #[cfg(test)]
    fn channel_only(capacity: usize) -> (Arc<Self>, mpsc::Receiver<ShardAssignment>) {
        let (tx, rx) = mpsc::channel(capacity);
        let queue = Arc::new(Self {
            tx,
            enqueued: AtomicU64::new(0),
            completed: AtomicU64::new(0),
            failed: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
        });
        (queue, rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::time::Instant;
    use tokio::sync::Notify;

    fn test_assignment(idx: u32) -> ShardAssignment {
        ShardAssignment {
            assignment_hash: None,
            content_hash: format!("hash-{idx}"),
            custodian_did: "did:elohim:test".to_string(),
            shard_index: idx,
            strategy: crate::node_registry_api::ShardingStrategy::Geographic,
            status: crate::node_registry_api::ShardStatus::Active,
            verified_at: None,
            created_at: "2026-09-23T00:00:00Z".to_string(),
            updated_at: "2026-09-23T00:00:00Z".to_string(),
        }
    }

    /// Stands in for a conductor call nobody ever answers — the exact
    /// incident shape this module fixes. `register` never resolves until
    /// the test explicitly notifies it.
    struct NeverRegistrar {
        release: Arc<Notify>,
        calls: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl ShardRegistrar for NeverRegistrar {
        async fn register(&self, _assignment: ShardAssignment) -> Result<Vec<u8>, StorageError> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            self.release.notified().await;
            Ok(vec![])
        }
    }

    struct AlwaysOkRegistrar;

    #[async_trait::async_trait]
    impl ShardRegistrar for AlwaysOkRegistrar {
        async fn register(&self, _assignment: ShardAssignment) -> Result<Vec<u8>, StorageError> {
            Ok(vec![])
        }
    }

    struct AlwaysFailRegistrar;

    #[async_trait::async_trait]
    impl ShardRegistrar for AlwaysFailRegistrar {
        async fn register(&self, _assignment: ShardAssignment) -> Result<Vec<u8>, StorageError> {
            Err(StorageError::Conductor("boom".into()))
        }
    }

    /// The observable this module exists to guarantee: enqueue returns
    /// immediately even when the registrar it feeds never answers. A caller
    /// (`put_blob_bytes`) that only waits on `enqueue`, never on the
    /// registration itself, cannot be held hostage by a stuck conductor.
    #[tokio::test]
    async fn enqueue_returns_immediately_even_when_registrar_never_answers() {
        let release = Arc::new(Notify::new());
        let calls = Arc::new(AtomicUsize::new(0));
        let registrar: Arc<dyn ShardRegistrar> = Arc::new(NeverRegistrar {
            release: Arc::clone(&release),
            calls: Arc::clone(&calls),
        });
        let queue = ShardRegistrationQueue::spawn(registrar);

        let start = Instant::now();
        queue.enqueue(test_assignment(0));
        assert!(
            start.elapsed() < Duration::from_millis(100),
            "enqueue must not block on the registrar"
        );
        assert_eq!(queue.enqueued_count(), 1);

        // Give the background worker a chance to pick the item up and park
        // inside `register` — proves the job really was queued and picked
        // up, not merely accepted and dropped.
        for _ in 0..50 {
            if calls.load(Ordering::Relaxed) == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(
            calls.load(Ordering::Relaxed),
            1,
            "worker must have picked up the queued item"
        );
        assert_eq!(queue.completed_count(), 0);
        assert_eq!(queue.failed_count(), 0);

        // Release the hung call so the test doesn't leak a parked task.
        release.notify_one();
    }

    /// Past capacity, `enqueue` drops and counts rather than blocking or
    /// growing memory — the bounded-queue half of the fix.
    #[tokio::test]
    async fn queue_full_drops_and_counts_instead_of_blocking() {
        let (queue, _rx) = ShardRegistrationQueue::channel_only(2);
        queue.enqueue(test_assignment(0));
        queue.enqueue(test_assignment(1));

        let start = Instant::now();
        queue.enqueue(test_assignment(2));
        assert!(
            start.elapsed() < Duration::from_millis(100),
            "a full queue must drop, never block"
        );

        assert_eq!(queue.enqueued_count(), 2);
        assert_eq!(queue.dropped_count(), 1);
    }

    /// A successful registration is counted as completed.
    #[tokio::test]
    async fn successful_registration_is_counted_completed() {
        let queue = ShardRegistrationQueue::spawn(Arc::new(AlwaysOkRegistrar));
        queue.enqueue(test_assignment(0));

        for _ in 0..50 {
            if queue.completed_count() == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(queue.completed_count(), 1);
        assert_eq!(queue.failed_count(), 0);
    }

    /// A failing registration is counted as failed, not silently swallowed —
    /// and never surfaces as an error to whoever enqueued it (the call is
    /// advisory; `enqueue` has no `Result` to fail).
    #[tokio::test]
    async fn failing_registration_is_counted_failed() {
        let queue = ShardRegistrationQueue::spawn(Arc::new(AlwaysFailRegistrar));
        queue.enqueue(test_assignment(0));

        for _ in 0..50 {
            if queue.failed_count() == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(queue.failed_count(), 1);
        assert_eq!(queue.completed_count(), 0);
    }
}
