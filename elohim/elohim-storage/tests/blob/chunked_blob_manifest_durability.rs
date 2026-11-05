//! A multi-chunk payload must stay servable after the in-process manifest cache
//! is gone: the payload store holds it whole under its byte CID, so a read never
//! depends on the cache (and the durable manifest projection names its chunks).
//!
//! Dropping the in-memory map is the in-test stand-in for a restart.

use elohim_epr::address::compute_bytes_cid;
use elohim_storage::blob_store::BlobStore;
use elohim_storage::http::HttpServer;
use elohim_storage::test_util::test_pool;
use std::sync::Arc;
use tempfile::tempdir;

/// ~9 MiB — several chunks at the epoch-0 chunker's 4 MiB maximum, below the RS
/// threshold (unstriped). Deterministic, non-uniform bytes so a byte-identity
/// assertion is meaningful (all-zeros would pass under a truncation bug).
fn chunked_payload() -> Vec<u8> {
    let len = 9 * 1024 * 1024 + 4321;
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
    let hash = compute_bytes_cid(&payload).to_string();

    let put = server
        .test_put_blob(&hash, &payload, "application/zip")
        .await;
    assert_eq!(put.status, 201, "chunked blob ingest must succeed");

    // Warm path still works.
    let warm = server.test_get_blob(&hash).await;
    assert_eq!(warm.status, 200, "chunked blob must serve from warm cache");

    // Simulated restart: nothing the read needs lives in the lost cache.
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
    let hash = compute_bytes_cid(&payload).to_string();

    let put = server
        .test_put_blob(&hash, &payload, "application/zip")
        .await;
    assert_eq!(put.status, 201, "chunked blob ingest must succeed");

    server.clear_manifest_cache_for_test().await;

    // This is the `/apps/{slug}` resolution path: it reads the payload by its
    // byte CID directly.
    let bytes = server
        .test_get_blob_or_heal(&hash)
        .await
        .expect("get_blob_or_heal must serve a chunked blob after the cache is lost");
    assert_eq!(
        bytes.as_slice(),
        payload.as_slice(),
        "get_blob_or_heal must return the original bytes"
    );
}

#[tokio::test]
async fn unsharded_blob_path_is_unchanged_by_the_manifest_fallback() {
    let (server, _tmp) = server_with_pool().await;
    let payload = b"a one-chunk payload stays a single stored blob".to_vec();
    let hash = compute_bytes_cid(&payload).to_string();

    let put = server.test_put_blob(&hash, &payload, "text/plain").await;
    assert_eq!(put.status, 201);

    server.clear_manifest_cache_for_test().await;

    let cold = server.test_get_blob(&hash).await;
    assert_eq!(cold.status, 200, "small blob must still serve");
    assert_eq!(cold.body.as_ref(), payload.as_slice());

    // A one-chunk payload's single unit IS the payload, so the direct read
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
    let hash = compute_bytes_cid(b"never ingested").to_string();

    let resp = server.test_get_blob(&hash).await;
    assert_eq!(
        resp.status, 404,
        "a blob with no bytes and no manifest must still 404"
    );
    assert!(server.test_get_blob_or_heal(&hash).await.is_none());
}
