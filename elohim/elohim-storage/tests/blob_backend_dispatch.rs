//! Integration test for cutover gate #2: HTTP `GET /blob/{hash}`
//! dual-format dispatch. Two fixture blobs:
//!   * `blob_blake3_only` — present in IrohBlobStore but absent from
//!     legacy BlobStore. Iroh-capable caller must receive 200; the
//!     iroh counter must increment by 1.
//!   * `blob_sha256_only` — present in legacy BlobStore but absent
//!     from IrohBlobStore. Iroh-capable caller must still receive 200
//!     (fall-through); the libp2p counter must increment by 1.
//!
//! Wire shape (`Content-Type`, `Content-Length`, `Cache-Control`)
//! must be identical regardless of which backend served.

#![cfg(feature = "p2p-iroh")]

use elohim_storage::blob_store::BlobStore;
use elohim_storage::http::HttpServer;
use elohim_storage::p2p_iroh::peer_map::{
    PeerTransportManifest, Plane, record_iroh_observation, record_libp2p_observation,
};
use elohim_storage::p2p_iroh::IrohBlobStore;
use elohim_storage::test_util::test_pool;
use std::sync::Arc;
use tempfile::tempdir;

/// Seed the test DB with an iroh-capable manifest for the given agent CID.
/// This makes `lookup_by_agent_cid` find a manifest with Blob on both
/// transports, so `select_transport` returns `Iroh` for `Plane::Blob`.
fn seed_iroh_capable_manifest(pool: &elohim_storage::db::DbPool, agent_cid: &str) {
    let mut conn = pool.get().unwrap();
    record_iroh_observation(
        &mut conn,
        agent_cid,
        &format!("iroh-node-{agent_cid}"),
        &[],
        &[Plane::Blob],
        1746878400,
    )
    .unwrap();
    record_libp2p_observation(
        &mut conn,
        agent_cid,
        &format!("libp2p-peer-{agent_cid}"),
        &[],
        &[Plane::Blob],
        1746878400,
    )
    .unwrap();
}

#[tokio::test]
async fn blake3_only_blob_served_from_iroh_for_iroh_capable_caller() {
    // Arrange: IrohBlobStore with one blob; legacy BlobStore empty.
    let iroh_dir = tempdir().unwrap();
    let iroh = Arc::new(
        IrohBlobStore::load(&iroh_dir.path().join("blobs_iroh"))
            .await
            .unwrap(),
    );
    let payload = b"blake3 only blob".to_vec();
    let blake3 = iroh.add_bytes(payload.clone()).await.unwrap();

    let legacy_dir = tempdir().unwrap();
    let legacy = Arc::new(BlobStore::new(legacy_dir.path().to_path_buf()).await.unwrap());

    // Seed a transport manifest for the caller so lookup_by_agent_cid finds it.
    let caller_cid = "did:elohim:test-iroh-caller";
    let pool = test_pool();
    seed_iroh_capable_manifest(&pool, caller_cid);

    // Build HttpServer with both backends + an iroh-capable self manifest.
    let server = HttpServer::new(legacy, "127.0.0.1:0".parse().unwrap())
        .with_db_pool(pool)
        .with_iroh_blob_store(iroh.clone())
        .with_self_transport_manifest(Arc::new(PeerTransportManifest::iroh_capable_for_test()));

    // Caller supplies a blake3-prefixed hash and their agent CID.
    let hash_str = format!("blake3-{}", blake3);
    let resp = server
        .handle_blob_get_for_test(&hash_str, Some(caller_cid))
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get(hyper::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("application/octet-stream")
    );
    // iroh counter must have incremented; libp2p counter must stay at 0.
    assert_eq!(server.blob_iroh_served_count_snapshot(), 1);
    assert_eq!(server.blob_libp2p_served_count_snapshot(), 0);
}

#[tokio::test]
async fn sha256_only_blob_served_from_libp2p_even_for_iroh_capable_caller() {
    // Arrange: legacy BlobStore with one blob; IrohBlobStore empty.
    let iroh_dir = tempdir().unwrap();
    let iroh = Arc::new(
        IrohBlobStore::load(&iroh_dir.path().join("blobs_iroh"))
            .await
            .unwrap(),
    );

    let legacy_dir = tempdir().unwrap();
    let legacy = Arc::new(BlobStore::new(legacy_dir.path().to_path_buf()).await.unwrap());
    let payload = b"sha256 only blob".to_vec();
    let stored = legacy.store(&payload).await.unwrap();

    // Seed a transport manifest for the caller.
    let caller_cid = "did:elohim:test-iroh-caller-2";
    let pool = test_pool();
    seed_iroh_capable_manifest(&pool, caller_cid);

    let server = HttpServer::new(legacy.clone(), "127.0.0.1:0".parse().unwrap())
        .with_db_pool(pool)
        .with_iroh_blob_store(iroh.clone())
        .with_self_transport_manifest(Arc::new(PeerTransportManifest::iroh_capable_for_test()));

    // Caller supplies a sha256-prefixed hash (no blake3 alias in DB).
    let resp = server
        .handle_blob_get_for_test(&stored.hash, Some(caller_cid))
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get(hyper::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("application/octet-stream")
    );
    // libp2p counter must have incremented; iroh counter must stay at 0.
    assert_eq!(server.blob_iroh_served_count_snapshot(), 0);
    assert_eq!(server.blob_libp2p_served_count_snapshot(), 1);
}
