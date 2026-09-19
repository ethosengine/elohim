//! An artifact over the erasure-coding threshold must be accepted whole and
//! served whole.
//!
//! `RS_THRESHOLD` is 64MB. Above it a manifest is `"rs-4-7"`: 4 data + 3 parity
//! shards computed over data padded to `4 * ceil(len/4)`. The ingest path used
//! to hand-slice the RAW body as if every non-`"none"` encoding were sequential
//! chunks, which for this band mis-hashes the unpadded tail data shard and then
//! indexes past the end of the body at the first parity shard — panicking the
//! HTTP task mid-PUT and dropping the connection after `100 Continue`. The blob
//! then 404s on every peer. Nothing in the suite PUT a blob above the threshold,
//! so the RS band had never worked through PUT at all.
//!
//! These tests pin both halves of the fix:
//!
//! 1. **Write** — shards come from `ShardEncoder::create_shards`, the same
//!    producer `create_manifest` hashed and the p2p push path already uses.
//! 2. **Read** — an RS manifest reconstructs through parity, so the blob still
//!    serves with up to `total_shards - data_shards` shards absent locally.
//!    That is the entire reason parity is carried; concatenating all seven
//!    shards only ever worked when none was missing.

use elohim_storage::blob_store::BlobStore;
use elohim_storage::db::init_pool_from_dir;
use elohim_storage::http::HttpServer;
use elohim_storage::sharding::{
    ShardConfig, ShardEncoder, ShardManifest, RS_THRESHOLD, SINGLE_SHARD_MAX,
};
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
    let hash = BlobStore::compute_hash(&data);

    let manifest = manifest_for(&data, "application/zip");
    assert_eq!(
        manifest.encoding, "rs-4-7",
        "test premise: {len} bytes must land in the erasure-coded band"
    );
    assert_eq!(manifest.shard_hashes.len(), 7);

    let store = Arc::new(BlobStore::new(dir.path()).await.unwrap());
    let server = server_over(dir.path(), store);

    let put = server.test_put_blob(&hash, &data, "application/zip").await;
    assert_eq!(
        put.status,
        201,
        "an erasure-coded blob must be accepted whole — got body: {}",
        String::from_utf8_lossy(&put.body[..put.body.len().min(200)])
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

/// The point of parity: an RS blob still serves when shards are gone.
///
/// One data shard and one parity shard are deleted — 5 of 7 remain, above the
/// 4 needed — and `/blob` must reconstruct rather than report a shard gap.
#[tokio::test]
async fn erasure_coded_blob_serves_through_parity_with_shards_missing() {
    let dir = tempdir().unwrap();
    let data = payload(RS_THRESHOLD + 2);
    let hash = BlobStore::compute_hash(&data);
    let manifest = manifest_for(&data, "application/zip");

    let store = Arc::new(BlobStore::new(dir.path()).await.unwrap());
    let server = server_over(dir.path(), store.clone());

    assert_eq!(
        server
            .test_put_blob(&hash, &data, "application/zip")
            .await
            .status,
        201
    );

    // Drop one data shard (index 0) and one parity shard (index 6).
    for index in [0usize, 6usize] {
        store
            .delete(&manifest.shard_hashes[index])
            .await
            .unwrap_or_else(|e| panic!("drop shard {index}: {e}"));
    }

    let got = server.test_get_blob(&hash).await;
    assert_eq!(
        got.status, 200,
        "5 of 7 shards is above the 4 an rs-4-7 blob needs — it must reconstruct, not 404"
    );
    assert_eq!(
        got.body.as_ref(),
        data.as_slice(),
        "a parity-reconstructed blob must be byte-identical to the ingested bytes"
    );
}

/// Regression guard for the band below the threshold: `"chunked"` blobs keep
/// their sequential, every-shard-required semantics unchanged. The RS fix must
/// not have leaked reconstruction (or its tolerance for gaps) into this band.
#[tokio::test]
async fn chunked_band_is_unchanged_by_the_erasure_coding_fix() {
    let dir = tempdir().unwrap();
    let data = payload(SINGLE_SHARD_MAX + 1);
    let hash = BlobStore::compute_hash(&data);
    let manifest = manifest_for(&data, "application/zip");
    assert_eq!(manifest.encoding, "chunked", "test premise");

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

    // A chunked blob carries no parity: one missing shard is terminal, and with
    // no peers to heal from the route 404s. This is the `MissingShard` heal
    // contract, preserved verbatim.
    store
        .delete(&manifest.shard_hashes[0])
        .await
        .expect("drop the first chunk");
    assert_eq!(
        server.test_get_blob(&hash).await.status,
        404,
        "a chunked blob with a missing chunk has no parity to fall back on"
    );
}
