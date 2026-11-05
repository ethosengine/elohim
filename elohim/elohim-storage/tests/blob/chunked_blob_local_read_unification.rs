//! `/blob` and the `/apps` resolver must give the SAME answer about a multi-chunk
//! payload, and that answer must survive the minting process going away.
//!
//! The payload store holds a payload whole under its byte CID (content-defined
//! chunks behind one layout record); the durable manifest projection names the
//! chunks the producer cut. These tests pin: a true restart (fresh `BlobStore` +
//! fresh pool over the same dir), the heal response's use of the manifest's
//! declared mime type, and an honest 404 when no local bytes remain.

use elohim_epr::address::{compute_bytes_cid, ByteCid};
use elohim_storage::blob_store::BlobStore;
use elohim_storage::db::init_pool_from_dir;
use elohim_storage::http::HttpServer;
use elohim_storage::sharding::{ShardConfig, ShardEncoder};
use std::sync::Arc;
use tempfile::tempdir;

/// ~9 MiB — more than two chunks at the epoch-0 chunker's 4 MiB maximum, below
/// the RS threshold (unstriped). Deterministic, non-uniform bytes so a
/// byte-identity assertion is meaningful (all-zeros would pass under a
/// truncation bug).
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

fn server_over(dir: &std::path::Path, blob_store: Arc<BlobStore>) -> HttpServer {
    let pool = init_pool_from_dir(dir).expect("pool over the storage dir");
    HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap()).with_db_pool(pool)
}

/// The fixture really is multi-chunk: the producer cuts it into several chunks.
#[test]
fn the_fixture_payload_is_multi_chunk_and_unstriped() {
    let payload = chunked_payload();
    let manifest = ShardEncoder::new(ShardConfig::default())
        .create_manifest(&payload, "application/zip", "commons")
        .unwrap();
    assert!(manifest.chunks.len() >= 3, "{}", manifest.chunks.len());
    assert!(manifest.stripes.is_empty());
}

/// A multi-chunk payload must still serve after the process that minted it is gone.
///
/// A FRESH `BlobStore` and a FRESH connection pool over the same storage dir —
/// no in-process cache carries over, exactly as on a restart.
#[tokio::test]
async fn chunked_blob_survives_a_full_process_restart() {
    let dir = tempdir().unwrap();
    let payload = chunked_payload();
    let cid = compute_bytes_cid(&payload).to_string();

    {
        let store = Arc::new(BlobStore::new(dir.path()).await.unwrap());
        let server = server_over(dir.path(), store);
        let put = server
            .test_put_blob(&cid, &payload, "application/zip")
            .await;
        assert_eq!(put.status, 201, "chunked blob ingest must succeed");
        assert_eq!(
            server.test_get_blob(&cid).await.status,
            200,
            "chunked blob must serve from the minting process"
        );
    } // server, pool and blob store all dropped — the minting process is gone.

    let store = Arc::new(BlobStore::new(dir.path()).await.unwrap());
    let server = server_over(dir.path(), store);

    let cold = server.test_get_blob(&cid).await;
    assert_eq!(
        cold.status, 200,
        "chunked blob must serve after a restart (fresh BlobStore + fresh pool over the same dir)"
    );
    assert_eq!(
        cold.body.as_ref(),
        payload.as_slice(),
        "restart-read blob must be byte-identical to the ingested bytes"
    );

    // The `/apps/{slug}` ZIP resolver rides `get_blob_or_heal` — same answer.
    let via_apps = server
        .test_get_blob_or_heal(&cid)
        .await
        .expect("the /apps resolution path must serve a chunked blob after a restart");
    assert_eq!(via_apps.as_slice(), payload.as_slice());
}

/// Bytes healed from a peer must serve with the manifest's Content-Type and an
/// ETag — not the fallback `application/octet-stream`: the durable manifest
/// recorded the mime type at ingest, and a heal must not lose it.
#[tokio::test]
async fn healed_composite_carries_the_manifest_mime_and_etag() {
    let dir = tempdir().unwrap();
    let payload = chunked_payload();
    let cid = compute_bytes_cid(&payload);
    let cid_text = cid.to_string();

    let store = Arc::new(BlobStore::new(dir.path()).await.unwrap());
    let server = server_over(dir.path(), store.clone());

    // Ingest to mint the durable manifest (mime = application/zip), then drop
    // the local payload: the local read now misses, which precedes a heal.
    assert_eq!(
        server
            .test_put_blob(&cid_text, &payload, "application/zip")
            .await
            .status,
        201
    );
    store.delete(&cid).await.expect("drop the local payload");

    let (status, content_type, etag) = server
        .test_blob_response_for_healed_bytes(&cid_text, payload.clone())
        .await;

    assert_eq!(status, 200, "healed bytes must serve");
    assert_eq!(
        content_type.as_deref(),
        Some("application/zip"),
        "a healed payload must carry the manifest's mime type, not opaque octets"
    );
    assert!(
        etag.is_some(),
        "a healed payload must carry an ETag like a locally-read one — got {etag:?}"
    );
}

/// A manifest with no local bytes still 404s — the fallback must not invent
/// availability.
#[tokio::test]
async fn composite_with_no_local_bytes_still_404s() {
    let dir = tempdir().unwrap();
    let payload = chunked_payload();
    let cid = compute_bytes_cid(&payload);
    let cid_text = cid.to_string();

    let store = Arc::new(BlobStore::new(dir.path()).await.unwrap());
    let server = server_over(dir.path(), store.clone());

    assert_eq!(
        server
            .test_put_blob(&cid_text, &payload, "application/zip")
            .await
            .status,
        201
    );
    store.delete(&cid).await.expect("drop the local payload");
    assert!(ByteCid::parse(&cid_text).is_ok());

    let resp = server.test_get_blob(&cid_text).await;
    assert_eq!(
        resp.status, 404,
        "a manifest with no local bytes anywhere must 404"
    );
    assert!(server.test_get_blob_or_heal(&cid_text).await.is_none());
}
