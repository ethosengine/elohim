//! A `>16MB` blob must stay servable after the minting process is gone.
//!
//! `put_blob_bytes` shards blobs above `SINGLE_SHARD_MAX` at 1MB and stores
//! each shard under its own hash — the composite is never stored under its own
//! name. Reassembly therefore depends entirely on the manifest, which used to
//! live only in the process-local cache: a restart (or any peer that received
//! the shards by replication) served 404 for bytes it demonstrably held.
//!
//! Dropping the in-memory map is the in-test stand-in for that restart.

use elohim_storage::blob_store::BlobStore;
use elohim_storage::http::HttpServer;
use elohim_storage::sharding::SINGLE_SHARD_MAX;
use elohim_storage::test_util::test_pool;
use std::sync::Arc;
use tempfile::tempdir;

/// ~17MB — above `SINGLE_SHARD_MAX` (16MB) and below `RS_THRESHOLD` (64MB),
/// so it lands in the "chunked" band. Deterministic, non-uniform bytes so a
/// byte-identity assertion is meaningful (all-zeros would pass under a
/// truncation bug).
fn chunked_payload() -> Vec<u8> {
    let len = SINGLE_SHARD_MAX + 1024 * 1024;
    let mut data = Vec::with_capacity(len);
    let mut state: u32 = 0x9E37_79B9;
    for _ in 0..len {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        data.push((state >> 24) as u8);
    }
    data
}

async fn server_with_pool() -> (HttpServer, tempfile::TempDir) {
    let tmp = tempdir().unwrap();
    let blob_store = Arc::new(BlobStore::new(tmp.path()).await.unwrap());
    let server =
        HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap()).with_db_pool(test_pool());
    (server, tmp)
}

#[tokio::test]
async fn chunked_blob_serves_after_manifest_cache_is_lost() {
    let (server, _tmp) = server_with_pool().await;
    let payload = chunked_payload();
    let hash = BlobStore::compute_hash(&payload);

    let put = server
        .test_put_blob(&hash, &payload, "application/zip")
        .await;
    assert_eq!(put.status, 201, "chunked blob ingest must succeed");

    // Warm path still works.
    let warm = server.test_get_blob(&hash).await;
    assert_eq!(warm.status, 200, "chunked blob must serve from warm cache");

    // Simulated restart: the composite was never stored under its own name, so
    // only the durable manifest projection can rescue this read.
    server.clear_manifest_cache_for_test().await;

    let cold = server.test_get_blob(&hash).await;
    assert_eq!(
        cold.status, 200,
        "chunked blob must serve after the in-process manifest cache is lost"
    );
    assert_eq!(
        cold.body.len(),
        payload.len(),
        "reassembled blob must be the original length"
    );
    assert_eq!(
        cold.body.as_ref(),
        payload.as_slice(),
        "reassembled blob must be byte-identical to the ingested bytes"
    );
}

#[tokio::test]
async fn get_blob_or_heal_serves_chunked_blob_after_manifest_cache_is_lost() {
    let (server, _tmp) = server_with_pool().await;
    let payload = chunked_payload();
    let hash = BlobStore::compute_hash(&payload);

    let put = server
        .test_put_blob(&hash, &payload, "application/zip")
        .await;
    assert_eq!(put.status, 201, "chunked blob ingest must succeed");

    server.clear_manifest_cache_for_test().await;

    // This is the `/apps/{slug}` resolution path: it reads the composite hash
    // directly, which for a sharded blob is a guaranteed local miss.
    let bytes = server
        .test_get_blob_or_heal(&hash)
        .await
        .expect("get_blob_or_heal must reassemble a chunked blob from its manifest");
    assert_eq!(
        bytes.as_slice(),
        payload.as_slice(),
        "get_blob_or_heal must return the original bytes"
    );
}

#[tokio::test]
async fn unsharded_blob_path_is_unchanged_by_the_manifest_fallback() {
    let (server, _tmp) = server_with_pool().await;
    let payload = b"encoding=none stays a single stored blob".to_vec();
    let hash = BlobStore::compute_hash(&payload);

    let put = server.test_put_blob(&hash, &payload, "text/plain").await;
    assert_eq!(put.status, 201);

    server.clear_manifest_cache_for_test().await;

    let cold = server.test_get_blob(&hash).await;
    assert_eq!(cold.status, 200, "small blob must still serve");
    assert_eq!(cold.body.as_ref(), payload.as_slice());

    // The composite IS the shard for `encoding = "none"`, so the direct read
    // path serves it without ever needing the manifest.
    let healed = server
        .test_get_blob_or_heal(&hash)
        .await
        .expect("small blob must serve from the direct local read");
    assert_eq!(healed.as_slice(), payload.as_slice());
}

#[tokio::test]
async fn unknown_blob_still_404s_after_the_fallback() {
    let (server, _tmp) = server_with_pool().await;
    let hash = BlobStore::compute_hash(b"never ingested");

    let resp = server.test_get_blob(&hash).await;
    assert_eq!(
        resp.status, 404,
        "a blob with no bytes and no manifest must still 404"
    );
    assert!(server.test_get_blob_or_heal(&hash).await.is_none());
}
