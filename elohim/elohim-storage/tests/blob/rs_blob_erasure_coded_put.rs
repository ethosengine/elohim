//! An artifact over the erasure-coding threshold must be accepted whole and
//! served whole.
//!
//! `RS_THRESHOLD` is 64 MiB. Above it a manifest declares `rs-gf8-0x11d-v1` 4+3:
//! the payload is cut into stripes of at most 16 MiB, each coded into 4 data and 3
//! parity shards. The PUT stores the payload itself (the one authoritative payload
//! store) and answers the manifest the producer minted; stripe shards are derived
//! for distribution.
//!
//! These tests pin:
//!
//! 1. **Write** — the payload above the threshold is accepted whole and served
//!    byte-identical, with a striped manifest.
//! 2. **Read** — a peer that holds only the stripe shards (custody it received,
//!    not the payload) reconstructs through parity: with any 3 shards of every
//!    stripe absent locally the blob still serves. That is the entire reason
//!    parity is carried.
//! 3. Below the threshold the manifest is unstriped and a payload absent locally
//!    is an honest 404 (no parity to fall back on).

use elohim_epr::address::compute_bytes_cid;
use elohim_storage::blob_store::BlobStore;
use elohim_storage::db::init_pool_from_dir;
use elohim_storage::http::HttpServer;
use elohim_storage::sharding::{ShardConfig, ShardEncoder, ShardManifest, RS_THRESHOLD};
use std::sync::Arc;
use tempfile::tempdir;

/// Deterministic, non-uniform bytes. All-zeros would pass under a truncation or
/// padding bug, so byte-identity has to be asserted against real entropy.
fn payload(len: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(len);
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    while data.len() < len {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        data.extend_from_slice(&state.to_le_bytes());
    }
    data.truncate(len);
    data
}

fn manifest_for(data: &[u8], mime: &str) -> ShardManifest {
    ShardEncoder::new(ShardConfig::default())
        .create_manifest(data, mime, "commons")
        .expect("manifest generation must succeed")
}

fn server_over(dir: &std::path::Path, blob_store: Arc<BlobStore>) -> HttpServer {
    let pool = init_pool_from_dir(dir).expect("pool over the storage dir");
    HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap()).with_db_pool(pool)
}

/// PUT then GET an erasure-coded blob and assert byte-identity.
///
/// Shared by the two length shapes below so only one oversized payload is ever
/// alive per test.
async fn put_and_get_round_trip(len: usize) {
    let dir = tempdir().unwrap();
    let data = payload(len);
    let hash = compute_bytes_cid(&data).to_string();

    let manifest = manifest_for(&data, "application/zip");
    assert_eq!(
        (
            manifest.erasure.id.as_str(),
            manifest.erasure.data,
            manifest.erasure.parity
        ),
        ("rs-gf8-0x11d-v1", 4, 3),
        "test premise: {len} bytes must land in the erasure-coded band"
    );
    assert!(manifest.stripes.iter().all(|s| s.shards.len() == 7));

    let store = Arc::new(BlobStore::new(dir.path()).await.unwrap());
    let server = server_over(dir.path(), store);

    let put = server.test_put_blob(&hash, &data, "application/zip").await;
    assert_eq!(
        put.status,
        201,
        "an erasure-coded blob must be accepted whole — got body: {}",
        String::from_utf8_lossy(&put.body[..put.body.len().min(200)])
    );

    let view: serde_json::Value = serde_json::from_slice(&put.body).unwrap();
    assert_eq!(view["blobCid"], hash);
    assert_eq!(
        view["stripes"].as_array().unwrap().len(),
        manifest.stripes.len(),
        "the PUT answers the striped manifest the producer minted"
    );

    let got = server.test_get_blob(&hash).await;
    assert_eq!(got.status, 200, "an accepted RS blob must serve");
    assert_eq!(
        got.body.as_ref(),
        data.as_slice(),
        "an RS blob must come back byte-identical to what was PUT"
    );
}

/// `RS_THRESHOLD + 2` — length % 4 == 2, the exact arithmetic of the live case
/// (71,763,974 bytes, `shard_size` 17,940,994, index 4 starting past the end).
#[tokio::test]
async fn erasure_coded_blob_just_over_the_threshold_round_trips() {
    put_and_get_round_trip(RS_THRESHOLD + 2).await;
}

/// A second non-multiple length, comfortably clear of the boundary, so the fix
/// is not an off-by-one that happens to work only at the threshold.
#[tokio::test]
async fn erasure_coded_blob_well_over_the_threshold_round_trips() {
    put_and_get_round_trip(RS_THRESHOLD + 4 * 1024 * 1024 + 1).await;
}

/// The point of parity: a peer holding only the stripe shards — and three of
/// every seven of those gone (one data shard, two parity shards) — reconstructs
/// the payload rather than report a shard gap.
#[tokio::test]
async fn erasure_coded_blob_serves_through_parity_with_shards_missing() {
    let dir = tempdir().unwrap();
    let data = payload(RS_THRESHOLD + 2);
    let cid = compute_bytes_cid(&data);
    let hash = cid.to_string();
    let (manifest, stripe_shards) = ShardEncoder::new(ShardConfig::default())
        .encode(&data, "application/zip", "commons")
        .expect("encode");

    let store = Arc::new(BlobStore::new(dir.path()).await.unwrap());
    let server = server_over(dir.path(), store.clone());

    // Ingest (mints the durable manifest), then become a peer that holds only
    // custody shards: the payload goes, the surviving shards are stored under
    // their own byte CIDs as received custody would be.
    assert_eq!(
        server
            .test_put_blob(&hash, &data, "application/zip")
            .await
            .status,
        201
    );
    store.delete(&cid).await.expect("drop the payload");
    for (stripe, shards) in manifest.stripes.iter().zip(&stripe_shards) {
        for (shard_ref, bytes) in stripe.shards.iter().zip(shards) {
            if [0u8, 5, 6].contains(&shard_ref.index) {
                continue;
            }
            let stored = store.store(bytes).await.expect("store custody shard");
            assert_eq!(stored.cid, shard_ref.cid);
        }
    }

    let got = server.test_get_blob(&hash).await;
    assert_eq!(
        got.status, 200,
        "4 of 7 shards per stripe is what rs 4+3 needs — it must reconstruct, not 404"
    );
    assert_eq!(
        got.body.as_ref(),
        data.as_slice(),
        "a parity-reconstructed blob must be byte-identical to the ingested bytes"
    );
}

/// Below the threshold: an unstriped manifest (chunks, no stripes). Such a payload
/// carries no parity, so with no peers to heal from a local miss is a 404.
#[tokio::test]
async fn unstriped_band_has_no_parity_to_fall_back_on() {
    let dir = tempdir().unwrap();
    let data = payload(17 * 1024 * 1024 + 1);
    let cid = compute_bytes_cid(&data);
    let hash = cid.to_string();
    let manifest = manifest_for(&data, "application/zip");
    assert!(manifest.stripes.is_empty(), "test premise");
    assert_eq!(manifest.erasure.id, "none");

    let store = Arc::new(BlobStore::new(dir.path()).await.unwrap());
    let server = server_over(dir.path(), store.clone());

    assert_eq!(
        server
            .test_put_blob(&hash, &data, "application/zip")
            .await
            .status,
        201
    );

    let got = server.test_get_blob(&hash).await;
    assert_eq!(got.status, 200);
    assert_eq!(got.body.as_ref(), data.as_slice());

    store.delete(&cid).await.expect("drop the payload");
    assert_eq!(
        server.test_get_blob(&hash).await.status,
        404,
        "an unstriped payload absent locally has no parity to fall back on"
    );
}
