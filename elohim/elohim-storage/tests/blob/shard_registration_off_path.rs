//! `PUT /blob/{hash}` must never wait on Node Registry shard-assignment
//! registration. Before this fix `put_blob_bytes` synchronously awaited one
//! `create_shard_assignment` conductor call PER SHARD before answering — an
//! advisory side-effect (failure only `warn!`s) held the whole HTTP response
//! hostage. Under conductor CPU saturation (alpha, 2026-09-23) that blew the
//! doorway's ~33s client timeout on every app-bundle PUT.
//!
//! These tests drive `put_blob_bytes` (via `HttpServer::test_put_blob`)
//! against a fake [`elohim_storage::shard_registration::ShardRegistrar`]
//! injected through `with_shard_registration_queue` — there is no other
//! seam into `NodeRegistryApi`, which requires a live conductor connection.

use elohim_storage::blob_store::BlobStore;
use elohim_storage::error::StorageError;
use elohim_storage::http::HttpServer;
use elohim_storage::node_registry_api::ShardAssignment;
use elohim_storage::shard_registration::{ShardRegistrar, ShardRegistrationQueue};
use elohim_storage::test_util::test_pool;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use tokio::sync::Notify;

/// A registrar whose `register` call never resolves until released — the
/// conductor-CPU-saturation shape from the incident.
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

async fn server_with_registrar(
    registrar: Arc<dyn ShardRegistrar>,
) -> (HttpServer, Arc<ShardRegistrationQueue>) {
    let tmp = tempdir().unwrap();
    let blob_store = Arc::new(BlobStore::new(tmp.path().join("blobs")).await.unwrap());
    let pool = test_pool();
    let queue = ShardRegistrationQueue::spawn(registrar);
    let server = HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap())
        .with_db_pool(pool)
        .with_shard_registration_queue(Arc::clone(&queue));
    (server, queue)
}

/// PUT returns success (and quickly) without ever waiting on a
/// slow/never-answering Node Registry — the enqueue is the observable stand-in
/// for "the response is not held hostage on this call."
#[tokio::test]
async fn put_completes_without_waiting_on_a_never_answering_registrar() {
    let release = Arc::new(Notify::new());
    let calls = Arc::new(AtomicUsize::new(0));
    let registrar: Arc<dyn ShardRegistrar> = Arc::new(NeverRegistrar {
        release: Arc::clone(&release),
        calls: Arc::clone(&calls),
    });
    let (server, queue) = server_with_registrar(registrar).await;

    let payload = b"put-off-path regression payload".to_vec();
    let sha = BlobStore::compute_hash(&payload);

    let start = Instant::now();
    let resp = server
        .test_put_blob(&sha, &payload, "application/octet-stream")
        .await;
    let elapsed = start.elapsed();

    assert_eq!(resp.status, 201, "PUT must still succeed: {:?}", resp.body);
    assert!(
        elapsed < Duration::from_secs(5),
        "PUT must not wait on the (never-answering) Node Registry call; took {elapsed:?}"
    );

    // The registration really was handed to the background worker — not
    // silently skipped — which is the other half of the guarantee: this
    // fix does not just delete the registration, it defers it.
    assert!(
        queue.enqueued_count() >= 1,
        "shard registration must have been enqueued"
    );

    release.notify_one();
}

/// A second PUT still succeeds while the worker is permanently stuck
/// draining the first one's registration — the request path and the
/// background queue are fully decoupled, not just fast-on-the-first-call.
/// (The queue-full drop-and-count behavior itself is unit-tested directly:
/// `shard_registration::tests::queue_full_drops_and_counts_instead_of_blocking`,
/// where the channel capacity can be made small enough to observe without a
/// 512-shard payload.)
#[tokio::test]
async fn second_put_still_succeeds_while_worker_is_stuck_on_the_first() {
    let release = Arc::new(Notify::new());
    let calls = Arc::new(AtomicUsize::new(0));
    let registrar: Arc<dyn ShardRegistrar> = Arc::new(NeverRegistrar {
        release: Arc::clone(&release),
        calls: Arc::clone(&calls),
    });
    let (server, queue) = server_with_registrar(registrar).await;

    for (i, payload) in [
        b"first put, worker will hang on its registration".to_vec(),
        b"second put while worker is stuck on the first's".to_vec(),
    ]
    .into_iter()
    .enumerate()
    {
        let sha = BlobStore::compute_hash(&payload);
        let start = Instant::now();
        let resp = server
            .test_put_blob(&sha, &payload, "application/octet-stream")
            .await;
        assert_eq!(
            resp.status, 201,
            "PUT #{i} must still succeed: {:?}",
            resp.body
        );
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "PUT #{i} must not wait on the stuck worker"
        );
    }

    assert!(queue.enqueued_count() >= 2);
    release.notify_one();
}
