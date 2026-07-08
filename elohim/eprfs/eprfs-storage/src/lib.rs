//! Storage-facing adapter scaffolding for `eprfs`.
//!
//! This crate is where `elohim-storage` integration belongs. The current
//! implementation provides a memory-backed adapter used by tests and early
//! consumers while the storage HTTP/DHT contract is selected.

use std::collections::HashMap;

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

    async fn publish_attestation(&self, draft: AttestationDraft) -> Result<EprRef> {
        let reference = EprRef::new(format!(
            "epr:attestation:{}",
            self.attestations.read().await.len() + 1
        ));
        self.attestations.write().await.push(draft);
        Ok(reference)
    }
}
