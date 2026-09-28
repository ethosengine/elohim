use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::address::{BlobCid, EprRef};
use crate::attestation::AttestationDraft;
use crate::error::EprfsError;
use crate::error::Result;

/// Minimal EPR record shape needed by projection code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EprRecord {
    pub reference: EprRef,
    pub content_type: String,
    pub payload: Value,
}

/// Local/remote availability of a blob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BlobPresence {
    Local,
    Remote,
    Missing,
    Unknown,
}

/// Fetch behavior requested by a projection/materialization operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FetchPolicy {
    LocalOnly,
    FetchIfMissing,
}

/// Bytes returned by the storage layer.
#[derive(Debug, Clone, PartialEq)]
pub struct BlobHandle {
    pub cid: BlobCid,
    pub bytes: Bytes,
}

#[async_trait]
pub trait EprfsStorage: Send + Sync {
    async fn resolve_epr(&self, reference: &EprRef) -> Result<EprRecord>;

    async fn has_blob(&self, cid: &BlobCid) -> Result<BlobPresence>;

    async fn fetch_blob(&self, cid: &BlobCid, policy: FetchPolicy) -> Result<BlobHandle>;

    async fn put_blob(&self, bytes: Bytes) -> Result<BlobCid>;

    /// Store bytes under an explicitly selected raw or DAG-CBOR CID. Adapters
    /// must verify the CID before persisting. This is byte custody only.
    async fn put_blob_verified(&self, _cid: &BlobCid, _bytes: Bytes) -> Result<()> {
        Err(EprfsError::Storage(
            "explicit-CID put is unsupported by this adapter".into(),
        ))
    }

    async fn publish_attestation(&self, draft: AttestationDraft) -> Result<EprRef>;
}
