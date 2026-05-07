//! Two-node iroh blob round-trip — Phase 2 acceptance test.
//!
//! Provider node adds a blob; fetcher node pulls it by hash + provider's
//! `NodeAddr`; bytes match. Loopback only (relays disabled). If this test
//! flakes in CI with `use_n0_relays: false`, the Phase 2 README documents
//! the n0-relay fallback.
//!
//! Gated on `p2p-iroh`.

#![cfg(feature = "p2p-iroh")]

use elohim_storage::p2p_iroh::{IrohConfig, IrohNode};
use tempfile::tempdir;

fn loopback_config(dir: &std::path::Path) -> IrohConfig {
    IrohConfig {
        blobs_dir: dir.join("blobs_iroh"),
        secret_key_path: dir.join("iroh.key"),
        use_n0_relays: false,
    }
}

/// Provider adds a blob; fetcher pulls by hash + provider's NodeAddr.
/// Bytes round-trip identically.
#[tokio::test]
async fn two_node_blob_round_trip() {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let provider = IrohNode::start(loopback_config(provider_dir.path()))
        .await
        .expect("provider starts");
    let fetcher = IrohNode::start(loopback_config(fetcher_dir.path()))
        .await
        .expect("fetcher starts");

    let payload = b"phase 2 round-trip payload".to_vec();
    let hash = provider
        .add_bytes(payload.clone())
        .await
        .expect("add succeeds");
    assert!(provider.has(hash).await.unwrap());
    assert!(!fetcher.has(hash).await.unwrap(), "fetcher starts empty");

    let provider_addr = provider.node_addr().await.expect("addr initializes");

    let received = fetcher
        .fetch_blob_from(provider_addr, hash)
        .await
        .expect("fetch succeeds");
    assert_eq!(&received[..], &payload[..]);
    assert!(
        fetcher.has(hash).await.unwrap(),
        "fetcher now has the blob locally"
    );

    provider.shutdown().await.unwrap();
    fetcher.shutdown().await.unwrap();
}

/// Distinct payloads round-trip with distinct BLAKE3 hashes.
#[tokio::test]
async fn two_node_distinct_payloads() {
    let provider_dir = tempdir().unwrap();
    let fetcher_dir = tempdir().unwrap();

    let provider = IrohNode::start(loopback_config(provider_dir.path()))
        .await
        .unwrap();
    let fetcher = IrohNode::start(loopback_config(fetcher_dir.path()))
        .await
        .unwrap();

    let p1 = b"alpha".to_vec();
    let p2 = b"beta".to_vec();
    let h1 = provider.add_bytes(p1.clone()).await.unwrap();
    let h2 = provider.add_bytes(p2.clone()).await.unwrap();
    assert_ne!(h1, h2);

    let provider_addr = provider.node_addr().await.unwrap();
    let r1 = fetcher
        .fetch_blob_from(provider_addr.clone(), h1)
        .await
        .unwrap();
    let r2 = fetcher.fetch_blob_from(provider_addr, h2).await.unwrap();
    assert_eq!(&r1[..], &p1[..]);
    assert_eq!(&r2[..], &p2[..]);

    provider.shutdown().await.unwrap();
    fetcher.shutdown().await.unwrap();
}
