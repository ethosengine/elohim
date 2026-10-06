//! Storage-facing adapter scaffolding for `eprfs`.
//!
//! This crate is where `elohim-storage` integration belongs. The current
//! implementation provides a memory-backed adapter used by tests and early
//! consumers while the storage HTTP/DHT contract is selected.

// `async_trait` marks its generated futures `#[must_use]`; clippy 1.99 reads
// that as doubled on methods returning `Result`. The attribute is the macro's.
#![allow(clippy::double_must_use)]

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use bytes::Bytes;
use eprfs_core::{
    AttestationDraft, BlobCid, BlobHandle, BlobPresence, EprRecord, EprRef, EprfsError,
    EprfsStorage, FetchPolicy, Result,
};
use tokio::sync::RwLock;

/// Adapter configuration for a future elohim-storage HTTP-backed client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageEndpoint {
    pub base_url: String,
}

/// In-memory storage adapter for tests and contract prototyping.
#[derive(Default)]
pub struct MemoryStorage {
    blobs: RwLock<HashMap<BlobCid, Bytes>>,
    records: RwLock<HashMap<EprRef, EprRecord>>,
    attestations: RwLock<Vec<AttestationDraft>>,
}

impl MemoryStorage {
    pub async fn insert_blob(&self, cid: BlobCid, bytes: Bytes) {
        self.blobs.write().await.insert(cid, bytes);
    }

    pub async fn insert_record(&self, record: EprRecord) {
        self.records
            .write()
            .await
            .insert(record.reference.clone(), record);
    }

    pub async fn attestations(&self) -> Vec<AttestationDraft> {
        self.attestations.read().await.clone()
    }
}

#[async_trait]
impl EprfsStorage for MemoryStorage {
    async fn resolve_epr(&self, reference: &EprRef) -> Result<EprRecord> {
        self.records
            .read()
            .await
            .get(reference)
            .cloned()
            .ok_or_else(|| EprfsError::EprNotFound(reference.clone()))
    }

    async fn has_blob(&self, cid: &BlobCid) -> Result<BlobPresence> {
        if self.blobs.read().await.contains_key(cid) {
            Ok(BlobPresence::Local)
        } else {
            Ok(BlobPresence::Missing)
        }
    }

    async fn fetch_blob(&self, cid: &BlobCid, _policy: FetchPolicy) -> Result<BlobHandle> {
        let bytes = self
            .blobs
            .read()
            .await
            .get(cid)
            .cloned()
            .ok_or_else(|| EprfsError::BlobNotFound(cid.clone()))?;

        Ok(BlobHandle {
            cid: cid.clone(),
            bytes,
        })
    }

    async fn put_blob(&self, bytes: Bytes) -> Result<BlobCid> {
        let cid = BlobCid::compute(&bytes);
        self.blobs.write().await.insert(cid.clone(), bytes);
        Ok(cid)
    }

    async fn put_blob_verified(&self, cid: &BlobCid, bytes: Bytes) -> Result<()> {
        if !cid.verifies(&bytes) {
            return Err(EprfsError::Storage(format!(
                "blob integrity verification failed for {cid}"
            )));
        }
        let mut blobs = self.blobs.write().await;
        if let Some(existing) = blobs.get(cid) {
            if !cid.verifies(existing) {
                return Err(EprfsError::Storage(format!(
                    "existing blob is corrupt for {cid}"
                )));
            }
        }
        blobs.insert(cid.clone(), bytes);
        Ok(())
    }

    async fn publish_attestation(&self, draft: AttestationDraft) -> Result<EprRef> {
        let reference = EprRef::new(format!(
            "epr:attestation:{}",
            self.attestations.read().await.len() + 1
        ));
        self.attestations.write().await.push(draft);
        Ok(reference)
    }
}

/// Flat local content store for explicit caller-selected private bytes. This
/// does not resolve EPRs, publish attestations, or establish release standing.
/// Its directory is caller-owned and trusted against concurrent path replacement.
#[derive(Debug, Clone)]
pub struct DirectoryStorage {
    root: PathBuf,
    max_blob_bytes: usize,
}

const DEFAULT_MAX_BLOB_BYTES: usize = 64 * 1024 * 1024;

impl DirectoryStorage {
    /// Open an existing directory without creating or modifying it.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let root = path.as_ref();
        let metadata = std::fs::symlink_metadata(root).map_err(|source| EprfsError::Io {
            path: root.to_path_buf(),
            source,
        })?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(EprfsError::Storage(format!(
                "content store is not a real directory: {}",
                root.display()
            )));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err(EprfsError::Storage(format!(
                    "content store directory permits group/other access: {}",
                    root.display()
                )));
            }
        }
        Ok(Self {
            root: root.to_path_buf(),
            max_blob_bytes: DEFAULT_MAX_BLOB_BYTES,
        })
    }

    /// Restrict each object read and write before allocating or persisting it.
    pub fn with_max_blob_bytes(mut self, max_blob_bytes: usize) -> Self {
        self.max_blob_bytes = max_blob_bytes;
        self
    }

    /// Explicitly create a new store, then open it. Existing stores are opened
    /// without changing their contents.
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        let root = path.as_ref();
        if !root.exists() {
            let mut builder = std::fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            builder.create(root).map_err(|source| EprfsError::Io {
                path: root.to_path_buf(),
                source,
            })?;
        }
        Self::open(root)
    }

    fn object_path(&self, cid: &BlobCid) -> PathBuf {
        self.root.join(cid.to_string())
    }

    fn read_object(&self, cid: &BlobCid) -> Result<Bytes> {
        let path = self.object_path(cid);
        let metadata = std::fs::symlink_metadata(&path).map_err(|source| {
            if source.kind() == std::io::ErrorKind::NotFound {
                EprfsError::BlobNotFound(cid.clone())
            } else {
                EprfsError::Io {
                    path: path.clone(),
                    source,
                }
            }
        })?;
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            return Err(EprfsError::Storage(format!(
                "content object is not a regular file: {cid}"
            )));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err(EprfsError::Storage(format!(
                    "content object permits group/other access: {cid}"
                )));
            }
        }
        if metadata.len() > self.max_blob_bytes as u64 {
            return Err(EprfsError::Storage(format!(
                "content object exceeds byte limit: {cid}"
            )));
        }
        let file = std::fs::File::open(&path).map_err(|source| EprfsError::Io {
            path: path.clone(),
            source,
        })?;
        let mut bytes = Vec::new();
        file.take((self.max_blob_bytes as u64).saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|source| EprfsError::Io { path, source })?;
        if bytes.len() > self.max_blob_bytes {
            return Err(EprfsError::Storage(format!(
                "content object exceeds byte limit: {cid}"
            )));
        }
        if !cid.verifies(&bytes) {
            return Err(EprfsError::Storage(format!(
                "existing content object is corrupt: {cid}"
            )));
        }
        Ok(Bytes::from(bytes))
    }
}

#[async_trait]
impl EprfsStorage for DirectoryStorage {
    async fn resolve_epr(&self, _reference: &EprRef) -> Result<EprRecord> {
        Err(EprfsError::Storage(
            "directory content store cannot resolve EPRs".into(),
        ))
    }

    async fn has_blob(&self, cid: &BlobCid) -> Result<BlobPresence> {
        let path = self.object_path(cid);
        match std::fs::symlink_metadata(&path) {
            Ok(_) => {
                self.read_object(cid)?;
                Ok(BlobPresence::Local)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(BlobPresence::Missing),
            Err(source) => Err(EprfsError::Io { path, source }),
        }
    }

    async fn fetch_blob(&self, cid: &BlobCid, _policy: FetchPolicy) -> Result<BlobHandle> {
        Ok(BlobHandle {
            cid: cid.clone(),
            bytes: self.read_object(cid)?,
        })
    }

    async fn put_blob(&self, bytes: Bytes) -> Result<BlobCid> {
        let cid = BlobCid::compute(&bytes);
        self.put_blob_verified(&cid, bytes).await?;
        Ok(cid)
    }

    async fn put_blob_verified(&self, cid: &BlobCid, bytes: Bytes) -> Result<()> {
        if bytes.len() > self.max_blob_bytes {
            return Err(EprfsError::Storage(format!(
                "content object exceeds byte limit: {cid}"
            )));
        }
        if !cid.verifies(&bytes) {
            return Err(EprfsError::Storage(format!(
                "blob integrity verification failed for {cid}"
            )));
        }
        let path = self.object_path(cid);
        match std::fs::symlink_metadata(&path) {
            Ok(_) => {
                self.read_object(cid)?;
                return Ok(());
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => return Err(EprfsError::Io { path, source }),
        }
        let mut temporary =
            tempfile::NamedTempFile::new_in(&self.root).map_err(|source| EprfsError::Io {
                path: self.root.clone(),
                source,
            })?;
        temporary
            .write_all(&bytes)
            .map_err(|source| EprfsError::Io {
                path: path.clone(),
                source,
            })?;
        temporary
            .as_file()
            .sync_all()
            .map_err(|source| EprfsError::Io {
                path: path.clone(),
                source,
            })?;
        match temporary.persist_noclobber(&path) {
            Ok(_) => Ok(()),
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                self.read_object(cid)?;
                Ok(())
            }
            Err(error) => Err(EprfsError::Io {
                path,
                source: error.error,
            }),
        }
    }

    async fn publish_attestation(&self, _draft: AttestationDraft) -> Result<EprRef> {
        Err(EprfsError::Storage(
            "directory content store cannot publish attestations".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn directory_store_verifies_raw_and_dag_cbor_and_never_clobbers_corruption() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("objects");
        assert!(DirectoryStorage::open(&root).is_err());
        assert!(!root.exists());
        let store = DirectoryStorage::create(&root).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&root).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        let capped = store.clone().with_max_blob_bytes(2);
        let too_large = BlobCid::compute_raw(b"raw");
        assert!(capped
            .put_blob_verified(&too_large, Bytes::from_static(b"raw"))
            .await
            .is_err());
        let raw = Bytes::from_static(b"raw");
        let raw_cid = BlobCid::compute_raw(&raw);
        store
            .put_blob_verified(&raw_cid, raw.clone())
            .await
            .unwrap();
        assert_eq!(
            store
                .fetch_blob(&raw_cid, FetchPolicy::LocalOnly)
                .await
                .unwrap()
                .bytes,
            raw
        );
        let cbor = Bytes::from_static(&[0xa0]);
        let cbor_cid = BlobCid::compute(&cbor);
        store
            .put_blob_verified(&cbor_cid, cbor.clone())
            .await
            .unwrap();
        assert_eq!(
            store
                .fetch_blob(&cbor_cid, FetchPolicy::LocalOnly)
                .await
                .unwrap()
                .bytes,
            cbor
        );
        assert!(store
            .put_blob_verified(&raw_cid, Bytes::from_static(b"wrong"))
            .await
            .is_err());
        std::fs::write(root.join(raw_cid.to_string()), b"corrupt").unwrap();
        assert!(store.put_blob_verified(&raw_cid, raw).await.is_err());
        assert_eq!(
            std::fs::read(root.join(raw_cid.to_string())).unwrap(),
            b"corrupt"
        );
    }

    #[tokio::test]
    async fn directory_store_refuses_symlink_object_and_root() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("objects");
            DirectoryStorage::create(&root).unwrap();
            symlink(&root, dir.path().join("linked")).unwrap();
            assert!(DirectoryStorage::open(dir.path().join("linked")).is_err());
            let cid = BlobCid::compute_raw(b"contents");
            symlink(dir.path().join("outside"), root.join(cid.to_string())).unwrap();
            let store = DirectoryStorage::open(&root).unwrap();
            assert!(store
                .put_blob_verified(&cid, Bytes::from_static(b"contents"))
                .await
                .is_err());
        }
    }

    #[cfg(unix)]
    #[test]
    fn directory_store_refuses_public_existing_root_and_object() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("objects");
        std::fs::create_dir(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(DirectoryStorage::open(&root).is_err());
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        let store = DirectoryStorage::open(&root).unwrap();
        let cid = BlobCid::compute_raw(b"hello");
        let object = root.join(cid.to_string());
        std::fs::write(&object, b"hello").unwrap();
        std::fs::set_permissions(&object, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(store.read_object(&cid).is_err());
    }
}
