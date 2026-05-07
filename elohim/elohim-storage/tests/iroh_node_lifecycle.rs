//! Single-node iroh smoke test — gated on `p2p-iroh` feature.
//!
//! Verifies the boot path: build endpoint, mount iroh-blobs on the Router,
//! shut down cleanly. Does NOT exercise peer connectivity (see
//! `iroh_blob_roundtrip.rs` for that).

#![cfg(feature = "p2p-iroh")]

use elohim_storage::p2p_iroh::{IrohConfig, IrohNode};
use tempfile::tempdir;

#[tokio::test]
async fn single_node_starts_and_shuts_down() {
    let dir = tempdir().unwrap();
    let cfg = IrohConfig {
        blobs_dir: dir.path().join("blobs_iroh"),
        secret_key_path: dir.path().join("iroh.key"),
        use_n0_relays: false,
    };

    let node = IrohNode::start(cfg).await.expect("node starts");
    let _id = node.node_id();
    node.shutdown().await.expect("shutdown clean");
}

#[tokio::test]
async fn iroh_key_persisted_to_disk() {
    let dir = tempdir().unwrap();
    let cfg = IrohConfig {
        blobs_dir: dir.path().join("blobs_iroh"),
        secret_key_path: dir.path().join("iroh.key"),
        use_n0_relays: false,
    };

    {
        let node = IrohNode::start(cfg.clone()).await.unwrap();
        node.shutdown().await.unwrap();
    }

    assert!(cfg.secret_key_path.exists(), "iroh.key should persist");
    assert!(cfg.blobs_dir.exists(), "blobs_iroh dir should persist");
}
