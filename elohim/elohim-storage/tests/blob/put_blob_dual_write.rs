//! PUT /blob/{cid}: the payload lands once, in the one authoritative payload
//! store, under its byte CID (raw + BLAKE3); the answer declares the byte profile;
//! the self-custody inventory row is stamped once and a re-seed is idempotent.
//!
//! (Formerly the iroh dual-write gate: there is no second, iroh-keyed copy any more —
//! the iroh hash of a payload IS its byte CID's digest, so nothing is written twice.)

use elohim_epr::address::{compute_bytes_cid, ByteCid};
use elohim_storage::blob_store::{BlobStore, Exists};
use elohim_storage::http::HttpServer;
use elohim_storage::test_util::test_pool;
use std::sync::Arc;
use tempfile::tempdir;

async fn server_with_pool(tmp: &tempfile::TempDir) -> (HttpServer, Arc<BlobStore>) {
    let blob_store = Arc::new(BlobStore::new(tmp.path().join("blobs")).await.unwrap());
    let bind: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = HttpServer::new(blob_store.clone(), bind)
        .with_self_peer_id("peer_self_test".into())
        .with_db_pool(test_pool());
    (server, blob_store)
}

/// The PUT answer names the byte CID and the profile it was cut under, the store
/// holds the bytes under that CID, and the inventory row is stamped under it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn put_blob_stores_once_under_the_byte_cid_and_stamps_self_custody() {
    let tmp = tempdir().unwrap();
    let (server, blob_store) = server_with_pool(&tmp).await;

    let payload = b"single-store test bytes".to_vec();
    let cid = compute_bytes_cid(&payload);
    let cid_text = cid.to_string();

    let resp = server
        .test_put_blob(&cid_text, &payload, "application/octet-stream")
        .await;
    assert_eq!(resp.status, 201, "expected 201, body: {:?}", resp.body);

    let view: serde_json::Value = serde_json::from_slice(&resp.body).unwrap();
    assert_eq!(view["blobCid"].as_str().unwrap(), cid_text);
    assert_eq!(view["byteLen"].as_u64().unwrap(), payload.len() as u64);
    assert_eq!(view["profile"]["epoch"].as_u64().unwrap(), 0);
    assert_eq!(view["profile"]["chunker"]["id"], "fastcdc-2020");
    assert!(
        view.get("blake3Hash").is_none(),
        "no second address field: the CID is the BLAKE3 address"
    );
    assert_eq!(
        ByteCid::parse(&cid_text).unwrap().digest(),
        blake3::hash(&payload).as_bytes(),
        "the iroh hash of the payload is the byte CID's digest"
    );

    assert_eq!(blob_store.exists(&cid).await, Exists::Verified);
    assert_eq!(
        blob_store.get(&cid).await.unwrap().as_ref(),
        payload.as_slice()
    );

    // The self-custody write is fire-and-forget on the blocking pool.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let pool = server.db_pool().expect("test db pool");
    let mut conn = pool.get().unwrap();
    let rows = elohim_storage::db::peer_blob_inventory::lookup_hosts(
        &mut conn,
        &cid_text,
        "2026-01-01T00:00:00Z",
    )
    .unwrap();
    assert_eq!(rows.len(), 1, "expected exactly one inventory row");
    assert_eq!(rows[0].peer_id, "peer_self_test");
    assert_eq!(rows[0].source, "self-custody");
}

/// Three consecutive PUT requests with identical bytes must not produce
/// duplicate inventory rows (replace_into guarantees idempotency).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn put_blob_re_seed_is_idempotent() {
    let tmp = tempdir().unwrap();
    let (server, _) = server_with_pool(&tmp).await;

    let payload = b"idempotent test bytes".to_vec();
    let cid_text = compute_bytes_cid(&payload).to_string();

    for i in 0..3u32 {
        let resp = server
            .test_put_blob(&cid_text, &payload, "application/octet-stream")
            .await;
        assert_eq!(resp.status, 201, "iteration {i}: expected 201");
    }

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let pool = server.db_pool().expect("test db pool");
    let mut conn = pool.get().unwrap();
    let rows = elohim_storage::db::peer_blob_inventory::lookup_hosts(
        &mut conn,
        &cid_text,
        "2026-01-01T00:00:00Z",
    )
    .unwrap();
    assert_eq!(rows.len(), 1, "re-seed must not duplicate inventory rows");
}

/// A path that names bytes other than the body is refused (`cid-mismatch`) and
/// nothing is stored; a legacy `sha256-` path is refused as `invalid-byte-cid`.
#[tokio::test]
async fn put_blob_refuses_a_mismatched_or_legacy_path_and_stores_nothing() {
    let tmp = tempdir().unwrap();
    let blob_store = Arc::new(BlobStore::new(tmp.path().join("blobs")).await.unwrap());
    let bind: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = HttpServer::new(blob_store.clone(), bind);

    let payload = b"the body".to_vec();
    let claimed = compute_bytes_cid(b"some other body").to_string();
    let resp = server
        .test_put_blob(&claimed, &payload, "application/octet-stream")
        .await;
    assert_eq!(resp.status, 400);
    let body: serde_json::Value = serde_json::from_slice(&resp.body).unwrap();
    assert_eq!(body["error"], "cid-mismatch");
    assert_ne!(
        blob_store.exists(&compute_bytes_cid(&payload)).await,
        Exists::Verified
    );

    let legacy = format!("sha256-{}", "a".repeat(64));
    let resp = server
        .test_put_blob(&legacy, &payload, "application/octet-stream")
        .await;
    assert_eq!(resp.status, 400);
    let body: serde_json::Value = serde_json::from_slice(&resp.body).unwrap();
    assert_eq!(body["error"], "invalid-byte-cid");
}
