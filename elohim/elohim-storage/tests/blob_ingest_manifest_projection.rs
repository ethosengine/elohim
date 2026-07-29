//! Blob ingest must persist the exact shard manifest it generated.
//!
//! The custody-facing fold resolves `shard_locations.shard_hash` back to a
//! blob through this operational projection. Keeping the manifest only in the
//! process-local cache makes a legitimate custody commitment unsearchable
//! after ingest or restart.

use elohim_storage::blob_store::BlobStore;
use elohim_storage::db::shard_manifests;
use elohim_storage::http::HttpServer;
use elohim_storage::sharding::{ShardConfig, ShardEncoder};
use elohim_storage::test_util::test_pool;
use std::sync::Arc;
use tempfile::tempdir;

#[tokio::test]
async fn blob_ingest_projects_its_generated_manifest_for_custody_resolution() {
    let tmp = tempdir().unwrap();
    let blob_store = Arc::new(BlobStore::new(tmp.path()).await.unwrap());
    let pool = test_pool();
    let server = HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap()).with_db_pool(pool);

    let payload = b"Matthew's witnessed shard mapping";
    let blob_hash = BlobStore::compute_hash(payload);
    let expected = ShardEncoder::new(ShardConfig::default())
        .create_manifest(payload, "text/plain", "commons")
        .unwrap();

    let response = server
        .test_put_blob(&blob_hash, payload, "text/plain")
        .await;
    assert_eq!(response.status, 201, "blob ingest must succeed");

    let pool = server.db_pool().expect("manifest projection DB pool");
    let mut conn = pool.get().unwrap();
    let manifest =
        shard_manifests::get_manifest(&mut conn, "lamad", &format!("blob:{}", expected.blob_cid))
            .unwrap()
            .expect("ingested blob must have a persisted shard manifest");

    assert_eq!(manifest.blob_hash, expected.blob_hash);
    assert_eq!(
        manifest.blob_cid.as_deref(),
        Some(expected.blob_cid.as_str())
    );
    assert_eq!(manifest.encoding, expected.encoding);
    assert_eq!(
        serde_json::from_str::<Vec<String>>(&manifest.shard_hashes_json).unwrap(),
        expected.shard_hashes,
        "the persisted mapping must be exactly the mapping used at ingest"
    );
}
