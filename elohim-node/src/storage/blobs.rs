//! Blob reference management — delegates to elohim-storage BlobStore.
//!
//! Provides `BlobManager` which wraps an `Arc<BlobStore>` and implements
//! `elohim_bitswap::BitswapStore` for Bitswap block exchange.

use std::sync::Arc;

use cid::Cid;
use elohim_storage::BlobStore;

/// Manages blob storage for the unified node.
///
/// Wraps elohim-storage's `BlobStore` and implements the `BitswapStore` trait
/// so Bitswap can directly read/write blocks via the shared blob store.
pub struct BlobManager {
    store: Arc<BlobStore>,
}

impl BlobManager {
    /// Create a new BlobManager wrapping the given BlobStore.
    pub fn new(store: Arc<BlobStore>) -> Self {
        Self { store }
    }

    /// Get a reference to the underlying BlobStore.
    #[allow(dead_code)]
    pub fn store(&self) -> &Arc<BlobStore> {
        &self.store
    }
}

#[async_trait::async_trait]
impl elohim_bitswap::BitswapStore for BlobManager {
    async fn contains(&self, cid: &Cid) -> anyhow::Result<bool> {
        let cid_str = cid.to_string();
        Ok(self.store.exists_by_address(&cid_str).await.unwrap_or(false))
    }

    async fn get(&self, cid: &Cid) -> anyhow::Result<Option<Vec<u8>>> {
        let cid_str = cid.to_string();
        match self.store.get_by_address(&cid_str).await {
            Ok(data) => Ok(Some(data)),
            Err(_) => Ok(None),
        }
    }

    async fn insert(&self, cid: Cid, data: Vec<u8>) -> anyhow::Result<()> {
        // Store using the BlobStore; CID is recomputed from data internally.
        // For DAG-CBOR (codec 0x71), use store_dag_cbor to get the right codec in the CID.
        if cid.codec() == 0x71 {
            self.store.store_dag_cbor(&data).await?;
        } else {
            self.store.store(&data).await?;
        }
        Ok(())
    }

    async fn missing_blocks(&self, cid: &Cid) -> anyhow::Result<Vec<Cid>> {
        let cid_str = cid.to_string();
        let data = match self.store.get_by_address(&cid_str).await {
            Ok(data) => data,
            Err(_) => return Ok(vec![*cid]), // Block itself is missing
        };

        // Extract linked CIDs from IPLD-encoded data
        let links = elohim_storage::dag_store::extract_links(&data, cid.codec());
        let mut missing = Vec::new();
        for link_cid in links {
            let link_str = link_cid.to_string();
            if !self
                .store
                .exists_by_address(&link_str)
                .await
                .unwrap_or(false)
            {
                missing.push(link_cid);
            }
        }
        Ok(missing)
    }
}
