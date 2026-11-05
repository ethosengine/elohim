//! Integration test for `GET /blob/{cid}` with an iroh-capable caller. There is
//! one payload store: the bytes serve from it under their byte CID whatever the
//! caller's transport; a copy held only in the iroh blob store (the retired
//! second, BLAKE3-keyed store) is not a second source and does not serve.
//!
//! Wire shape (`Content-Type`, `X-Content-Address`) is the payload store's.

use elohim_epr::address::compute_bytes_cid;
use elohim_storage::blob_store::BlobStore;
use elohim_storage::http::HttpServer;
use elohim_storage::p2p_iroh::peer_map::{
    record_iroh_observation, record_libp2p_observation, PeerTransportManifest, Plane,
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
async fn a_copy_only_in_the_iroh_store_does_not_serve() {
    let iroh_dir = tempdir().unwrap();
    let iroh = Arc::new(
        IrohBlobStore::load(&iroh_dir.path().join("blobs_iroh"))
            .await
            .unwrap(),
    );
    let payload = b"bytes held only by the iroh store".to_vec();
    iroh.add_bytes(payload.clone()).await.unwrap();

    let store_dir = tempdir().unwrap();
    let store = Arc::new(
        BlobStore::new(store_dir.path().to_path_buf())
            .await
            .unwrap(),
    );

    let caller_cid = "did:elohim:test-iroh-caller";
    let pool = test_pool();
    seed_iroh_capable_manifest(&pool, caller_cid);

    let server = HttpServer::new(store, "127.0.0.1:0".parse().unwrap())
        .with_db_pool(pool)
        .with_iroh_blob_store(iroh.clone())
        .with_self_transport_manifest(Arc::new(PeerTransportManifest::iroh_capable_for_test()));

    let cid = compute_bytes_cid(&payload).to_string();
    let resp = server
        .handle_blob_get_for_test(&cid, Some(caller_cid))
        .await
        .unwrap();
    assert_eq!(resp.status(), 404, "the iroh store is no second source");
    assert_eq!(server.blob_iroh_served_count_snapshot(), 0);
}

#[tokio::test]
async fn the_payload_store_serves_an_iroh_capable_caller_under_the_byte_cid() {
    let iroh_dir = tempdir().unwrap();
    let iroh = Arc::new(
        IrohBlobStore::load(&iroh_dir.path().join("blobs_iroh"))
            .await
            .unwrap(),
    );

    let store_dir = tempdir().unwrap();
    let store = Arc::new(
        BlobStore::new(store_dir.path().to_path_buf())
            .await
            .unwrap(),
    );
    let payload = b"payload store blob".to_vec();
    let stored = store.store(&payload).await.unwrap();
    assert_eq!(stored.cid, compute_bytes_cid(&payload));

    let caller_cid = "did:elohim:test-iroh-caller-2";
    let pool = test_pool();
    seed_iroh_capable_manifest(&pool, caller_cid);

    let server = HttpServer::new(store.clone(), "127.0.0.1:0".parse().unwrap())
        .with_db_pool(pool)
        .with_iroh_blob_store(iroh.clone())
        .with_self_transport_manifest(Arc::new(PeerTransportManifest::iroh_capable_for_test()));

    let cid = stored.cid.to_string();
    let resp = server
        .handle_blob_get_for_test(&cid, Some(caller_cid))
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get(hyper::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("application/octet-stream")
    );
    assert_eq!(
        resp.headers()
            .get("X-Content-Address")
            .and_then(|v| v.to_str().ok()),
        Some(cid.as_str())
    );
    assert_eq!(server.blob_iroh_served_count_snapshot(), 0);
    assert_eq!(server.blob_libp2p_served_count_snapshot(), 1);
}
