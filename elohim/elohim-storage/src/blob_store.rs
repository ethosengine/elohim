//! The `/blob` route's content-addressed byte store: a shim over `elohim-blob-store`.
//!
//! The store moved to `crates/elohim-blob-store` (artifact-store E4) so one payload store per
//! device can be opened by every process on it. This module keeps every name and signature the
//! crate's callers use: [`BlobStore`] is a newtype whose methods delegate to
//! [`elohim_blob_store::BlobStore`] and return [`StorageError`], mapped from
//! [`BlobStoreError`] variant for variant (`Internal`, `BlobNotFound`, `HashMismatch`,
//! `InvalidContentAddress`, `Io`; the owner/shared refusals, which the unlocked `/blob` store
//! never produces, become `Internal`). The value types re-export unchanged.
//!
//! [`BlobStore::new`] opens the crate's **unlocked** store (no lock files: this process is the
//! only one on `blobs_dir`), then schedules what the store itself must not know about: the
//! node-local compute-payload cleanup (`crate::compute_payload_store`), when
//! `ELOHIM_COMPUTE_LOCAL_API=1`.
//!
//! One behaviour differs from the store as it was on dev before the move, by an accepted
//! correctness fix (artifact-store E4, review round 3 ruling 3): deleting a payload no longer
//! unlists a unit whose verified bytes remain — an inline object one of the payload's chunks
//! duplicated stays in `list_servable`. The on-disk representation is unchanged.
//!
//! See the crate's module docs for the layout, write protocol and read verification.

use std::path::Path;

use bytes::Bytes;
use cid::Cid;
use elohim_epr::address::ByteCid;
use tokio::io::AsyncRead;
use tracing::warn;

pub use elohim_blob_store::{
    byte_cid_serde, BlobStoreError, ChunkRef, Exists, LayoutRecord, RecordStoreResult,
    StorageStats, StoreResult, MARKER_MAGIC, STORE_PROFILE,
};

use crate::error::StorageError;

/// `BlobStoreError` → `StorageError`, variant for variant.
pub fn storage_error(error: BlobStoreError) -> StorageError {
    match error {
        BlobStoreError::BlobNotFound(cid) => StorageError::BlobNotFound(cid),
        BlobStoreError::HashMismatch { expected, actual } => {
            StorageError::HashMismatch { expected, actual }
        }
        BlobStoreError::InvalidContentAddress(text) => StorageError::InvalidContentAddress(text),
        BlobStoreError::Io(error) => StorageError::Io(error),
        BlobStoreError::Internal(text) => StorageError::Internal(text),
        other @ (BlobStoreError::OwnerHeld { .. }
        | BlobStoreError::SharedInstance { .. }
        | BlobStoreError::HeldCopyUnreadable { .. }
        | BlobStoreError::LegacyLayout { .. }
        | BlobStoreError::UnsupportedFilesystem { .. }
        | BlobStoreError::PublicationLost { .. }
        | BlobStoreError::UnsafeRoot { .. }) => StorageError::Internal(other.to_string()),
    }
}

/// Blob storage manager. Cheap to share: the state lives behind one `Arc`.
pub struct BlobStore {
    core: elohim_blob_store::BlobStore,
}

impl BlobStore {
    /// Open (or create) the store at `root_dir` (see the module docs).
    pub async fn new<P: AsRef<Path>>(root_dir: P) -> Result<Self, StorageError> {
        let core = elohim_blob_store::BlobStore::new(root_dir)
            .await
            .map_err(storage_error)?;
        if std::env::var("ELOHIM_COMPUTE_LOCAL_API").as_deref() == Ok("1") {
            let payload_root = core.root_dir().to_path_buf();
            tokio::spawn(async move {
                let mut tick = tokio::time::interval(std::time::Duration::from_secs(60));
                loop {
                    tick.tick().await;
                    if let Err(error) = crate::compute_payload_store::cleanup(&payload_root).await {
                        warn!(%error, "compute payload cleanup failed");
                    }
                }
            });
        }
        Ok(Self { core })
    }

    /// A store in a fresh temporary directory (tests).
    pub fn new_memory() -> Self {
        Self {
            core: elohim_blob_store::BlobStore::new_memory(),
        }
    }

    /// The relocated store behind this shim.
    pub fn core(&self) -> &elohim_blob_store::BlobStore {
        &self.core
    }

    /// The store's root directory (free-space probes measure its filesystem).
    pub fn root_dir(&self) -> &Path {
        self.core.root_dir()
    }

    /// Store a payload, returning its byte CID, layout and chunks.
    pub async fn store(&self, data: &[u8]) -> Result<StoreResult, StorageError> {
        self.core.store(data).await.map_err(storage_error)
    }

    /// Store a payload read from `reader`, holding at most one maximum chunk (plus the
    /// first chunk, kept until a second arrives) in memory.
    pub async fn store_stream<R>(&self, reader: R) -> Result<StoreResult, StorageError>
    where
        R: AsyncRead + Send + Unpin + 'static,
    {
        self.core.store_stream(reader).await.map_err(storage_error)
    }

    /// Read a whole payload, recomputing its BLAKE3 before returning a byte.
    pub async fn get(&self, cid: &ByteCid) -> Result<Bytes, StorageError> {
        self.core.get(cid).await.map_err(storage_error)
    }

    /// Read one chunk by its own byte CID, verified against that CID.
    pub async fn get_chunk(&self, cid: &ByteCid) -> Result<Bytes, StorageError> {
        self.core.get_chunk(cid).await.map_err(storage_error)
    }

    /// `[start, end)` of a payload: a full verified read, then a slice.
    pub async fn get_range(
        &self,
        cid: &ByteCid,
        start: u64,
        end: u64,
    ) -> Result<Bytes, StorageError> {
        self.core
            .get_range(cid, start, end)
            .await
            .map_err(storage_error)
    }

    /// A payload's length, from the inline file or its layout record, without reading it.
    pub async fn size(&self, cid: &ByteCid) -> Result<u64, StorageError> {
        self.core.size(cid).await.map_err(storage_error)
    }

    /// Verified presence; see [`Exists`].
    pub async fn exists(&self, cid: &ByteCid) -> Exists {
        self.core.exists(cid).await
    }

    /// Structural presence with no payload hashing and no side effects.
    pub async fn present(&self, cid: &ByteCid) -> bool {
        self.core.present(cid).await
    }

    /// Remove a payload and the chunks no remaining layout record names.
    pub async fn delete(&self, cid: &ByteCid) -> Result<(), StorageError> {
        self.core.delete(cid).await.map_err(storage_error)
    }

    /// Move a payload (marker or inline file, and its layout record) to quarantine.
    pub async fn quarantine(&self, cid: &ByteCid, reason: &str) -> Result<(), StorageError> {
        self.core
            .quarantine(cid, reason)
            .await
            .map_err(storage_error)
    }

    /// Every payload CID filed under `blobs/`. Synchronous walk.
    pub fn list_cids(&self) -> Result<Vec<ByteCid>, StorageError> {
        self.core.list_cids().map_err(storage_error)
    }

    /// Every custody unit held and readable verified. Synchronous, hashes every unit.
    pub fn list_units(&self) -> Result<Vec<ByteCid>, StorageError> {
        self.core.list_units().map_err(storage_error)
    }

    /// Whether `cid` is held AND reads back verified. Synchronous, hashes.
    pub fn holds_verified(&self, cid: &ByteCid) -> bool {
        self.core.holds_verified(cid)
    }

    /// Every CID servable verified, from the servable index (no hashing).
    pub fn list_servable(&self) -> Result<Vec<ByteCid>, StorageError> {
        self.core.list_servable().map_err(storage_error)
    }

    /// Rebuild the servable index by hashing every unit and payload. Synchronous.
    pub fn scrub(&self) -> Result<(), StorageError> {
        self.core.scrub().map_err(storage_error)
    }

    /// The unit names [`BlobStore::list_units`] would verify, with no byte read.
    pub fn unit_names(&self) -> Vec<ByteCid> {
        self.core.unit_names()
    }

    /// Store a dag-cbor record (`0x71 + sha2-256`) on the record lane.
    pub async fn store_record(&self, canonical: &[u8]) -> Result<RecordStoreResult, StorageError> {
        self.core
            .store_record(canonical)
            .await
            .map_err(storage_error)
    }

    /// Read a dag-cbor record, verified against its CID.
    pub async fn get_record(&self, cid: &Cid) -> Result<Bytes, StorageError> {
        self.core.get_record(cid).await.map_err(storage_error)
    }

    /// Constant-time statistics from the running tally (seeded once at open).
    pub fn stats(&self) -> StorageStats {
        self.core.stats()
    }
}

/// Test support the crate's sibling modules use (sharding, signals, http, custody).
#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The kernel's xorshift64* stream (seed `0x454c4f48494d`); every xorshift fixture in
    /// `tree-vectors.json` is a prefix of it.
    pub(crate) fn xorshift_bytes(len: usize) -> Vec<u8> {
        let mut state: u64 = 0x454c_4f48_494d;
        let mut out = Vec::with_capacity(len + 8);
        while out.len() < len {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            out.extend_from_slice(&state.wrapping_mul(0x2545_F491_4F6C_DD1D).to_le_bytes());
        }
        out.truncate(len);
        out
    }

    /// Flip a byte of an inline payload's file in place (same length): present and
    /// well-shaped, no longer hashing to its CID. For probe tests.
    pub(crate) fn spoil_inline_payload(store: &BlobStore, cid: &ByteCid) {
        let path =
            elohim_blob_store::fanout_path(&store.root_dir().join("blobs"), &cid.to_string());
        let mut bytes = std::fs::read(&path).unwrap();
        bytes[0] ^= 0x01;
        std::fs::write(path, bytes).unwrap();
    }

    /// Nothing under `quarantine/`: no probe moved a file.
    pub(crate) fn nothing_quarantined(store: &BlobStore) -> bool {
        std::fs::read_dir(store.root_dir().join("quarantine"))
            .map(|mut d| d.next().is_none())
            .unwrap_or(true)
    }

    #[test]
    fn every_store_error_maps_to_its_storage_error() {
        assert!(matches!(
            storage_error(BlobStoreError::BlobNotFound("x".into())),
            StorageError::BlobNotFound(_)
        ));
        assert!(matches!(
            storage_error(BlobStoreError::HashMismatch {
                expected: "a".into(),
                actual: "b".into()
            }),
            StorageError::HashMismatch { .. }
        ));
        assert!(matches!(
            storage_error(BlobStoreError::InvalidContentAddress("x".into())),
            StorageError::InvalidContentAddress(_)
        ));
        assert!(matches!(
            storage_error(BlobStoreError::Io(std::io::Error::other("x"))),
            StorageError::Io(_)
        ));
        assert!(matches!(
            storage_error(BlobStoreError::Internal("x".into())),
            StorageError::Internal(_)
        ));
        assert!(matches!(
            storage_error(BlobStoreError::SharedInstance { op: "delete" }),
            StorageError::Internal(_)
        ));
    }

    #[tokio::test]
    async fn the_shim_opens_the_unlocked_store_with_no_lock_files() {
        let dir = tempfile::tempdir().unwrap();
        let store = BlobStore::new(dir.path()).await.unwrap();
        assert_eq!(store.core().open_mode(), None);
        let stored = store.store(&xorshift_bytes(6 * 1024 * 1024)).await.unwrap();
        assert!(stored.chunks.len() > 1);
        assert_eq!(store.exists(&stored.cid).await, Exists::Verified);
        for lock in [elohim_blob_store::OWNER_LOCK, elohim_blob_store::OPS_LOCK] {
            assert!(!dir.path().join(lock).exists(), "{lock}");
        }
    }
}
