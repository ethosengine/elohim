//! Cutover gate #3 e2e: a seeded blob is fetchable by BOTH its SHA256 and
//! BLAKE3 addresses, returning identical bytes.
//!
//! Memory anchor: feedback_head_vs_get_blob_asymmetry — this test uses GET,
//! never HEAD, on /blob/{hash}.
//!
//! Note: `blake3Hash` in the PUT response carries the canonical "blake3-{hex}"
//! prefix (consistent with `blob_hash` carrying "sha256-{hex}"). The GET
//! handler accepts both prefixed forms on the same `/blob/{hash}` route per
//! the http-blob-graduation plan.
//!
//! Gated on `p2p-iroh`.

#![cfg(feature = "p2p-iroh")]

use elohim_storage::blob_store::BlobStore;
use elohim_storage::http::HttpServer;
use elohim_storage::p2p_iroh::IrohBlobStore;
use std::sync::Arc;
use tempfile::tempdir;

/// After a PUT /blob/{sha256} with iroh wired:
/// - GET on the SHA256 address returns the original bytes
/// - GET on the BLAKE3 address returns the same bytes
/// - Both responses are byte-identical
#[tokio::test]
async fn seeded_blob_is_fetchable_by_both_addresses() {
    let tmp = tempdir().unwrap();
    let blob_store = Arc::new(BlobStore::new(tmp.path().join("legacy")).await.unwrap());
    let iroh_store = Arc::new(IrohBlobStore::load(&tmp.path().join("iroh")).await.unwrap());

    let bind: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = HttpServer::new(blob_store.clone(), bind)
        .with_iroh_blob_store(iroh_store.clone())
        .with_self_peer_id("peer_self_e2e".into());

    let payload: Vec<u8> = (0..1024).map(|i| (i % 251) as u8).collect();
    let sha = BlobStore::compute_hash(&payload);

    // 1. PUT bytes once — dual-write to both stores.
    let put_resp = server
        .test_put_blob(&sha, &payload, "application/octet-stream")
        .await;
    assert_eq!(put_resp.status, 201, "PUT must return 201");
    let view: serde_json::Value = serde_json::from_slice(&put_resp.body).unwrap();

    // blake3Hash is "blake3-{hex}" (prefixed, consistent with blob_hash = "sha256-{hex}").
    let blake3_addr = view["blake3Hash"]
        .as_str()
        .expect("blake3Hash must be present when iroh is wired")
        .to_string();
    assert!(
        blake3_addr.starts_with("blake3-"),
        "blake3Hash must have 'blake3-' prefix, got: {blake3_addr}"
    );

    // 2. GET by SHA256 returns the bytes.
    let sha_resp = server.test_get_blob(&sha).await;
    assert_eq!(sha_resp.status, 200, "SHA256 GET must return 200");
    assert_eq!(
        sha_resp.body.as_ref(),
        payload.as_slice(),
        "SHA256 GET body mismatch"
    );

    // 3. GET by BLAKE3 returns the same bytes.
    // blake3_addr already has the "blake3-" prefix — pass it directly.
    let blake3_resp = server.test_get_blob(&blake3_addr).await;
    assert_eq!(blake3_resp.status, 200, "BLAKE3 GET must return 200");
    assert_eq!(
        blake3_resp.body.as_ref(),
        payload.as_slice(),
        "BLAKE3 GET body mismatch"
    );

    // 4. Both addresses return byte-identical payloads.
    assert_eq!(
        sha_resp.body, blake3_resp.body,
        "SHA256 and BLAKE3 responses must be byte-identical"
    );
}
