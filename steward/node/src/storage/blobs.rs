//! Blob reference management — delegates to elohim-storage BlobStore.
//!
//! Provides `BlobManager` which wraps an `Arc<BlobStore>` and implements
//! `elohim_bitswap::BitswapStore` for Bitswap block exchange.

use std::sync::Arc;

use cid::Cid;
use elohim_epr::address::{ByteCid, RECORD_CODEC};
use elohim_storage::blob_store::Exists;
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

impl BlobManager {
    /// Read a block under whichever lane its CID names: a dag-cbor record (`0x71`) or a byte
    /// CID. A CID in neither profile is simply not held.
    async fn read_block(&self, cid: &Cid) -> Option<Vec<u8>> {
        if cid.codec() == RECORD_CODEC {
            return self.store.get_record(cid).await.ok().map(|b| b.to_vec());
        }
        let byte_cid = ByteCid::try_from(*cid).ok()?;
        self.store.get(&byte_cid).await.ok().map(|b| b.to_vec())
    }
}

#[async_trait::async_trait]
impl elohim_bitswap::BitswapStore for BlobManager {
    async fn contains(&self, cid: &Cid) -> anyhow::Result<bool> {
        if cid.codec() == RECORD_CODEC {
            return Ok(self.store.get_record(cid).await.is_ok());
        }
        Ok(match ByteCid::try_from(*cid) {
            Ok(byte_cid) => self.store.exists(&byte_cid).await == Exists::Verified,
            Err(_) => false,
        })
    }

    async fn get(&self, cid: &Cid) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(self.read_block(cid).await)
    }

    async fn insert(&self, cid: Cid, data: Vec<u8>) -> anyhow::Result<()> {
        // The CID is recomputed from the bytes by the store; a block whose bytes hash
        // elsewhere than its claimed CID is refused, never stored under another name.
        let stored = if cid.codec() == RECORD_CODEC {
            self.store.store_record(&data).await?.cid
        } else {
            *self.store.store(&data).await?.cid.as_cid()
        };
        anyhow::ensure!(
            stored == cid,
            "block bytes hash to {stored}, not the claimed {cid}"
        );
        Ok(())
    }

    async fn missing_blocks(&self, cid: &Cid) -> anyhow::Result<Vec<Cid>> {
        let Some(data) = self.read_block(cid).await else {
            return Ok(vec![*cid]); // Block itself is missing
        };

        // Extract linked CIDs from IPLD-encoded data
        let links = elohim_storage::dag_store::extract_links(&data, cid.codec());
        let mut missing = Vec::new();
        for link_cid in links {
            if !self.contains(&link_cid).await.unwrap_or(false) {
                missing.push(link_cid);
            }
        }
        Ok(missing)
    }
}
