//! Thin wrapper over [`iroh_blobs::store::fs::FsStore`].
//!
//! Phase 2 surface for content-addressed blob storage on the iroh path.
//! BLAKE3-keyed (iroh-blobs native); disjoint on disk from the legacy
//! SHA256-keyed [`crate::blob_store::BlobStore`].
//!
//! Cutover (Phase 11) will replace legacy `BlobStore` call sites with this
//! type uniformly. For now both stores coexist, runtime config selects one.
//!
//! API drift chokepoint: any breakage when bumping iroh-blobs versions
//! lands here, not in callers.

use std::path::Path;

use anyhow::Result;
use bytes::Bytes;
use iroh_blobs::{store::fs::FsStore, Hash};

/// Phase 2 wrapper around the iroh-blobs filesystem store.
#[derive(Debug, Clone)]
pub struct IrohBlobStore {
    inner: FsStore,
}

impl IrohBlobStore {
    /// Open or create the iroh-blobs store at `blobs_dir`. Creates the
    /// directory if missing. The store internally uses redb for metadata
    /// + filesystem for blob data — `blobs_dir` is the redb root.
    pub async fn load(blobs_dir: &Path) -> Result<Self> {
        if !blobs_dir.exists() {
            tokio::fs::create_dir_all(blobs_dir).await?;
        }
        let inner = FsStore::load(blobs_dir).await?;
        Ok(Self { inner })
    }

    /// Add bytes to the store; returns the BLAKE3 hash. iroh-blobs creates
    /// a persistent tag automatically, so the blob is not subject to GC.
    pub async fn add_bytes(&self, data: Vec<u8>) -> Result<Hash> {
        let tag_info = self.inner.blobs().add_bytes(data).await?;
        Ok(tag_info.hash)
    }

    /// Read bytes for a known hash.
    pub async fn get_bytes(&self, hash: Hash) -> Result<Bytes> {
        let bytes = self.inner.blobs().get_bytes(hash).await?;
        Ok(bytes)
    }

    /// Whether the store currently holds the given hash. Errors propagate
    /// (rather than being collapsed to `false`) so transport-level issues
    /// don't masquerade as "not found".
    pub async fn has(&self, hash: Hash) -> Result<bool> {
        let present = self.inner.blobs().has(hash).await?;
        Ok(present)
    }

    /// Borrow the underlying iroh-blobs store. Phase 2 Task 2.2 mounts this
    /// on an iroh `Endpoint` via `BlobsProtocol`.
    pub fn inner(&self) -> &FsStore {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn add_then_get_round_trips() {
        let dir = tempdir().unwrap();
        let store = IrohBlobStore::load(&dir.path().join("blobs_iroh"))
            .await
            .unwrap();

        let payload = b"hello iroh blobs".to_vec();
        let hash = store.add_bytes(payload.clone()).await.unwrap();

        let got = store.get_bytes(hash).await.unwrap();
        assert_eq!(&got[..], &payload[..]);
    }

    #[tokio::test]
    async fn has_returns_false_for_unknown() {
        let dir = tempdir().unwrap();
        let store = IrohBlobStore::load(&dir.path().join("blobs_iroh"))
            .await
            .unwrap();

        // A hash with no corresponding blob.
        let unknown = Hash::new(b"nothing-stored-here");
        assert!(!store.has(unknown).await.unwrap());
    }

    #[tokio::test]
    async fn same_bytes_dedupe_to_same_hash() {
        let dir = tempdir().unwrap();
        let store = IrohBlobStore::load(&dir.path().join("blobs_iroh"))
            .await
            .unwrap();

        let payload = b"identical content".to_vec();
        let h1 = store.add_bytes(payload.clone()).await.unwrap();
        let h2 = store.add_bytes(payload.clone()).await.unwrap();
        assert_eq!(h1, h2, "BLAKE3 is content-addressed; same bytes => same hash");
    }

    #[tokio::test]
    async fn creates_blobs_dir_if_missing() {
        let dir = tempdir().unwrap();
        let blobs_dir = dir.path().join("not-yet-created").join("blobs_iroh");
        assert!(!blobs_dir.exists());
        let _store = IrohBlobStore::load(&blobs_dir).await.unwrap();
        assert!(blobs_dir.exists());
    }

    #[tokio::test]
    async fn has_returns_true_after_add() {
        let dir = tempdir().unwrap();
        let store = IrohBlobStore::load(&dir.path().join("blobs_iroh"))
            .await
            .unwrap();

        let hash = store.add_bytes(b"present".to_vec()).await.unwrap();
        assert!(store.has(hash).await.unwrap());
    }
}
