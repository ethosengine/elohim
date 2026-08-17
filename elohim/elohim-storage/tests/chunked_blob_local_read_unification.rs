//! `/blob` and the `/apps` resolver must give the SAME answer about a `>16MB`
//! blob, and that answer must survive the minting process going away.
//!
//! Two facts make a sharded composite easy to lose:
//!
//! 1. `put_blob_bytes` stores each 1MB shard under ITS own hash and never
//!    stores the composite under its own name, so a raw `BlobStore` probe on
//!    the composite always answers "absent" for bytes we fully hold.
//! 2. A composite that arrived by peer-heal takes the OTHER shape — the whole
//!    blob IS on disk under its own name (`BlobStore::store` → `CHUNKED`
//!    marker + `.d/` chunk dir) while the shards named by the manifest were
//!    never fetched.
//!
//! Every local-read caller must handle both shapes identically. These tests pin
//! that: a true restart (fresh `BlobStore` + fresh pool over the same dir), and
//! the healed-composite shape where the manifest names shards we do not hold.

use elohim_storage::blob_store::BlobStore;
use elohim_storage::db::init_pool_from_dir;
use elohim_storage::http::HttpServer;
use elohim_storage::sharding::{ShardConfig, ShardEncoder, SINGLE_SHARD_MAX};
use std::sync::Arc;
use tempfile::tempdir;

/// ~17MB — above `SINGLE_SHARD_MAX` (16MB) and below `RS_THRESHOLD` (64MB), so
/// it lands in the "chunked" band. Deterministic, non-uniform bytes so a
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

/// The shard hashes `put_blob_bytes` would mint for `data` — the same encoder
/// and config the ingest path uses, so the set is byte-for-byte the one on disk.
fn shard_hashes_for(data: &[u8], mime: &str) -> Vec<String> {
    ShardEncoder::new(ShardConfig::default())
        .create_manifest(data, mime, "commons")
        .expect("manifest generation must succeed")
        .shard_hashes
}

fn server_over(dir: &std::path::Path, blob_store: Arc<BlobStore>) -> HttpServer {
    let pool = init_pool_from_dir(dir).expect("pool over the storage dir");
    HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap()).with_db_pool(pool)
}

/// A `>16MB` blob must still serve after the process that minted it is gone.
///
/// This is the restart-durability acceptance test the mesh repro asked for: a
/// FRESH `BlobStore` and a FRESH connection pool over the same storage dir —
/// no in-process cache carries over, exactly as on a restart.
#[tokio::test]
async fn chunked_blob_survives_a_full_process_restart() {
    let dir = tempdir().unwrap();
    let payload = chunked_payload();
    let hash = BlobStore::compute_hash(&payload);

    {
        let store = Arc::new(BlobStore::new(dir.path()).await.unwrap());
        let server = server_over(dir.path(), store);
        let put = server
            .test_put_blob(&hash, &payload, "application/zip")
            .await;
        assert_eq!(put.status, 201, "chunked blob ingest must succeed");
        assert_eq!(
            server.test_get_blob(&hash).await.status,
            200,
            "chunked blob must serve from the minting process"
        );
    } // server, pool and blob store all dropped — the minting process is gone.

    let store = Arc::new(BlobStore::new(dir.path()).await.unwrap());
    let server = server_over(dir.path(), store);

    let cold = server.test_get_blob(&hash).await;
    assert_eq!(
        cold.status, 200,
        "chunked blob must serve after a restart (fresh BlobStore + fresh pool over the same dir)"
    );
    assert_eq!(
        cold.body.as_ref(),
        payload.as_slice(),
        "restart-reassembled blob must be byte-identical to the ingested bytes"
    );

    // The `/apps/{slug}` ZIP resolver rides `get_blob_or_heal` — same answer.
    let via_apps = server
        .test_get_blob_or_heal(&hash)
        .await
        .expect("the /apps resolution path must serve a chunked blob after a restart");
    assert_eq!(via_apps.as_slice(), payload.as_slice());
}

/// A composite whose bytes are held WHOLE on disk must serve from `/blob`,
/// even when a manifest row names shards this peer never fetched.
///
/// This is the peer-heal shape: `finalize_fetch_success` stores the reassembled
/// composite under its own name, while the manifest projection (written at
/// ingest, or propagated) still names the 1MB shards. The `/apps` resolver
/// reads the composite directly and serves it; `/blob` must not disagree.
#[tokio::test]
async fn blob_route_serves_a_composite_held_whole_on_disk() {
    let dir = tempdir().unwrap();
    let payload = chunked_payload();
    let hash = BlobStore::compute_hash(&payload);

    let store = Arc::new(BlobStore::new(dir.path()).await.unwrap());
    let server = server_over(dir.path(), store.clone());

    // Ingest normally: 18 shards on disk + a durable manifest row.
    let put = server
        .test_put_blob(&hash, &payload, "application/zip")
        .await;
    assert_eq!(put.status, 201);

    // Now take the OTHER shape: drop every shard and hold the composite whole.
    for shard_hash in shard_hashes_for(&payload, "application/zip") {
        store.delete(&shard_hash).await.expect("drop shard");
    }
    store.store(&payload).await.expect("hold composite whole");

    let via_apps = server.test_get_blob_or_heal(&hash).await;
    assert_eq!(
        via_apps.as_deref(),
        Some(payload.as_slice()),
        "test-setup invariant: the /apps resolver reads the whole composite"
    );

    let resp = server.test_get_blob(&hash).await;
    assert_eq!(
        resp.status,
        200,
        "/blob must serve a composite held whole on disk — got body: {}",
        String::from_utf8_lossy(&resp.body[..resp.body.len().min(160)])
    );
    assert_eq!(resp.body.as_ref(), payload.as_slice());
}

/// A composite healed from a peer must serve with the manifest's Content-Type
/// and an ETag — not the fallback `application/octet-stream`.
///
/// This is the arm the shard-gap→peer-heal fall-through newly reaches: the
/// local read resolved the manifest (so the mime type is known and paid for)
/// but a shard was missing, and a peer supplied the composite. Serving those
/// bytes as opaque octets would make a `>16MB` bundle undisplayable in a
/// browser purely because it arrived by heal rather than from disk.
#[tokio::test]
async fn healed_composite_carries_the_manifest_mime_and_etag() {
    let dir = tempdir().unwrap();
    let payload = chunked_payload();
    let hash = BlobStore::compute_hash(&payload);

    let store = Arc::new(BlobStore::new(dir.path()).await.unwrap());
    let server = server_over(dir.path(), store.clone());

    // Ingest to mint the durable manifest (mime = application/zip), then drop
    // the shards: the local read now yields the shard-gap that precedes a heal.
    assert_eq!(
        server
            .test_put_blob(&hash, &payload, "application/zip")
            .await
            .status,
        201
    );
    for shard_hash in shard_hashes_for(&payload, "application/zip") {
        store.delete(&shard_hash).await.expect("drop shard");
    }

    let (status, content_type, etag) = server
        .test_blob_response_for_healed_bytes(&hash, payload.clone())
        .await;

    assert_eq!(status, 200, "healed bytes must serve");
    assert_eq!(
        content_type.as_deref(),
        Some("application/zip"),
        "a healed composite must carry the manifest's mime type, not opaque octets"
    );
    assert!(
        etag.is_some(),
        "a healed composite must carry an ETag like a locally-read one — got {etag:?}"
    );
}

/// A composite with neither shards nor whole bytes still 404s — the fallback
/// must not invent availability.
#[tokio::test]
async fn composite_with_no_local_bytes_still_404s() {
    let dir = tempdir().unwrap();
    let payload = chunked_payload();
    let hash = BlobStore::compute_hash(&payload);

    let store = Arc::new(BlobStore::new(dir.path()).await.unwrap());
    let server = server_over(dir.path(), store.clone());

    assert_eq!(
        server
            .test_put_blob(&hash, &payload, "application/zip")
            .await
            .status,
        201
    );
    for shard_hash in shard_hashes_for(&payload, "application/zip") {
        store.delete(&shard_hash).await.expect("drop shard");
    }

    let resp = server.test_get_blob(&hash).await;
    assert_eq!(
        resp.status, 404,
        "a manifest with no local bytes anywhere must 404"
    );
    let body = String::from_utf8_lossy(&resp.body).to_string();
    assert!(
        body.contains("shard"),
        "the 404 must name the missing shard rather than report the composite as \
         simply absent: got {body}"
    );
    assert!(server.test_get_blob_or_heal(&hash).await.is_none());
}
