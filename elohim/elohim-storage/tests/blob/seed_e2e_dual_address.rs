//! A seeded blob is fetchable by its one address, the byte CID, with the bytes it
//! was seeded with; the spellings of the retired dual-address era (`sha256-<hex>`,
//! `blake3-<hex>`, a sha2-256 raw CID) are refused with the typed 400.
//!
//! Memory anchor: feedback_head_vs_get_blob_asymmetry — this test uses GET,
//! never HEAD, on /blob/{cid}.

use elohim_epr::address::{compute_bytes_cid, compute_raw_sha2_cid};
use elohim_storage::blob_store::BlobStore;
use elohim_storage::http::HttpServer;
use std::sync::Arc;
use tempfile::tempdir;

#[tokio::test]
async fn seeded_blob_is_fetchable_by_its_byte_cid_and_no_other_spelling() {
    let tmp = tempdir().unwrap();
    let blob_store = Arc::new(BlobStore::new(tmp.path().join("blobs")).await.unwrap());
    let bind: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server =
        HttpServer::new(blob_store.clone(), bind).with_self_peer_id("peer_self_e2e".into());

    let payload: Vec<u8> = (0..1024).map(|i| (i % 251) as u8).collect();
    let cid = compute_bytes_cid(&payload).to_string();

    let put_resp = server
        .test_put_blob(&cid, &payload, "application/octet-stream")
        .await;
    assert_eq!(put_resp.status, 201, "PUT must return 201");

    let resp = server.test_get_blob(&cid).await;
    assert_eq!(resp.status, 200, "byte-CID GET must return 200");
    assert_eq!(resp.body.as_ref(), payload.as_slice(), "GET body mismatch");

    let blake3_hex = blake3::hash(&payload).to_hex().to_string();
    for retired in [
        format!("sha256-{}", "a".repeat(64)),
        format!("blake3-{blake3_hex}"),
        compute_raw_sha2_cid(&payload).to_string(),
    ] {
        let resp = server.test_get_blob(&retired).await;
        assert_eq!(resp.status, 400, "{retired} must be refused");
        let body: serde_json::Value = serde_json::from_slice(&resp.body).unwrap();
        assert_eq!(body["error"], "invalid-byte-cid", "{retired}");
    }
}
