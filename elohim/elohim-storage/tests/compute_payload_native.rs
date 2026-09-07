//! Native compute-payload leases served through the real iroh shard plane.
//!
//! This keeps compute payload custody on the native payload store while
//! exercising the same `SHARD_ALPN` path used by peer fetches. No HTTP route or
//! permanent blob-store copy is involved.

#![cfg(feature = "p2p-iroh")]

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use elohim_storage::blob_store::BlobStore;
use elohim_storage::compute_payload_store as payloads;
use elohim_storage::p2p_iroh::{
    parity_harness::TwoNodeFixture, AlpnRegistration, IrohBlobFetchError, IrohShardProtocol,
    ShardServiceBackend, SHARD_ALPN,
};
use elohim_storage::shard_service::ShardService;
use tempfile::tempdir;

#[tokio::test]
async fn compute_payload_lease_lifecycle_survives_real_iroh_fetches() -> Result<()> {
    let provider_dir = tempdir()?;
    let fetcher_dir = tempdir()?;
    let payload_dir = tempdir()?;
    let data = b"native compute payload bytes";
    let cid = BlobStore::compute_cid(data).to_string();

    payloads::put(payload_dir.path(), &cid, "task-a", 3600, data).await?;

    let blob_store = Arc::new(BlobStore::new(payload_dir.path()).await?);
    let service = Arc::new(ShardService::new(blob_store, None));
    let backend = Arc::new(ShardServiceBackend::new(service));
    let handler = IrohShardProtocol::new(backend);
    let provider_protocols: Vec<AlpnRegistration> = vec![(SHARD_ALPN.to_vec(), Box::new(handler))];
    let fixture = TwoNodeFixture::new_asymmetric(
        provider_dir.path(),
        provider_protocols,
        fetcher_dir.path(),
        Vec::new(),
    )
    .await?;

    // Keep all temporary directories alive while both iroh nodes are serving.
    let result = tokio::time::timeout(Duration::from_secs(15), async {
        let fetched = fixture
            .fetcher
            .fetch_blob_by_content_address(fixture.provider_addr.clone(), &cid)
            .await
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        assert_eq!(fetched, data);

        payloads::release(payload_dir.path(), &cid, "task-a").await?;
        let missing = fixture
            .fetcher
            .fetch_blob_by_content_address(fixture.provider_addr.clone(), &cid)
            .await;
        assert!(matches!(
            missing,
            Err(IrohBlobFetchError::NotFound(address)) if address == cid
        ));

        payloads::put(payload_dir.path(), &cid, "task-b", 3600, data).await?;
        assert!(payloads::owner_expiry(payload_dir.path(), &cid, "task-a")
            .await
            .is_some_and(|expiry| expiry <= payloads::now()));
        assert!(payloads::owner_expiry(payload_dir.path(), &cid, "task-b")
            .await
            .is_some_and(|expiry| expiry > payloads::now()));

        let refetched = fixture
            .fetcher
            .fetch_blob_by_content_address(fixture.provider_addr.clone(), &cid)
            .await
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        assert_eq!(refetched, data);
        Ok::<(), anyhow::Error>(())
    })
    .await;

    // Shutdown happens before the tempdir guards drop, even when the bounded
    // assertion times out, so endpoint tasks never outlive their directories.
    fixture.shutdown().await?;
    result.map_err(|_| anyhow::anyhow!("compute payload iroh test timed out"))??;
    Ok(())
}
