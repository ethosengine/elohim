//! Local chunked custody for derivation artifacts.
//!
//! The manifest and every chunk use the existing [`BlobCid`] codec. This module writes only to
//! the caller's [`EprfsStorage`]; it creates no network head, publication, or daemon dependency.

use std::fmt;

use bytes::{Bytes, BytesMut};
use eprfs_core::{BlobCid, DerivationRecord, EprfsError, EprfsStorage, FetchPolicy, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The largest raw chunk named by a tier blob manifest.
pub const TIER_BLOB_CHUNK_BYTES: usize = 1024 * 1024;

/// The local manifest epoch. An unknown epoch is retained in custody but never read as a hit.
pub const TIER_BLOB_MANIFEST_EPOCH: u32 = 1;

/// One raw chunk named by a local tier blob manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TierBlobChunk {
    pub cid: BlobCid,
    pub byte_len: u32,
}

/// The canonical DAG-CBOR record for one chunked derivation output.
///
/// `record.output` is the raw CID of the complete artifact. A reader verifies every chunk and
/// then that complete CID before returning bytes. The record names the closure and signer claim;
/// accepting its witness or widening its reach remains the caller's decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TierBlobManifest {
    pub epoch: u32,
    pub record: DerivationRecord,
    pub byte_len: u64,
    pub chunks: Vec<TierBlobChunk>,
}

/// A completed local put. The manifest itself is stored under `manifest_cid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TierBlobPut {
    pub manifest_cid: BlobCid,
    pub manifest: TierBlobManifest,
}

/// Why a local draw is a cold miss rather than a hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TierBlobMiss {
    ManifestAbsent,
}

impl fmt::Display for TierBlobMiss {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ManifestAbsent => f.write_str("manifest is absent"),
        }
    }
}

/// Why a manifest could not produce verified bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TierBlobUnavailable {
    ManifestCorrupt,
    ManifestInvalid { reason: String },
    UnsupportedEpoch { epoch: u32 },
    ChunkMissing { cid: BlobCid },
    ChunkCorrupt { cid: BlobCid },
    BlobCorrupt { cid: BlobCid },
    Sink { reason: String },
    Store { reason: String },
}

impl fmt::Display for TierBlobUnavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ManifestCorrupt => f.write_str("manifest is corrupt"),
            Self::ManifestInvalid { reason } => write!(f, "manifest is invalid: {reason}"),
            Self::UnsupportedEpoch { epoch } => {
                write!(f, "manifest epoch {epoch} is unsupported")
            }
            Self::ChunkMissing { cid } => write!(f, "chunk is missing: {cid}"),
            Self::ChunkCorrupt { cid } => write!(f, "chunk is corrupt: {cid}"),
            Self::BlobCorrupt { cid } => write!(f, "complete blob is corrupt: {cid}"),
            Self::Sink { reason } => write!(f, "stream sink stopped: {reason}"),
            Self::Store { reason } => write!(f, "local store is unavailable: {reason}"),
        }
    }
}

/// A caller-owned destination for one verified chunk at a time.
///
/// A sink may have received earlier chunks when a later chunk is unavailable. It must treat those
/// bytes as a completed artifact only when [`TierBlobStreamDraw::Completed`] is returned.
pub trait TierBlobSink {
    /// Accept one chunk after its CID and declared length have both verified.
    fn chunk(&mut self, chunk: &TierBlobChunk, bytes: Bytes) -> std::io::Result<()>;
}

impl<F> TierBlobSink for F
where
    F: FnMut(&TierBlobChunk, Bytes) -> std::io::Result<()>,
{
    fn chunk(&mut self, chunk: &TierBlobChunk, bytes: Bytes) -> std::io::Result<()> {
        self(chunk, bytes)
    }
}

/// The exhaustive outcome of a streaming draw.
///
/// Only `Completed` means every chunk arrived in manifest order and the incremental complete-blob
/// digest matched the derivation output. A sink can therefore distinguish a complete stream from
/// one stopped after a verified prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TierBlobStreamDraw {
    Completed {
        manifest: TierBlobManifest,
    },
    Miss {
        manifest_cid: BlobCid,
        why: TierBlobMiss,
    },
    Unavailable {
        manifest_cid: BlobCid,
        why: TierBlobUnavailable,
    },
}

impl TierBlobStreamDraw {
    /// A printable reason for every stream that did not complete.
    pub fn reason(&self) -> Option<String> {
        match self {
            Self::Completed { .. } => None,
            Self::Miss { why, .. } => Some(why.to_string()),
            Self::Unavailable { why, .. } => Some(why.to_string()),
        }
    }
}

/// The exhaustive local draw outcome. Only [`TierBlobDraw::Hit`] carries bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TierBlobDraw {
    Hit {
        manifest: TierBlobManifest,
        bytes: Bytes,
    },
    Miss {
        manifest_cid: BlobCid,
        why: TierBlobMiss,
    },
    Unavailable {
        manifest_cid: BlobCid,
        why: TierBlobUnavailable,
    },
}

impl TierBlobDraw {
    /// Verified bytes, only for a hit.
    pub fn bytes(&self) -> Option<&Bytes> {
        match self {
            Self::Hit { bytes, .. } => Some(bytes),
            Self::Miss { .. } | Self::Unavailable { .. } => None,
        }
    }

    /// A printable reason for every non-hit.
    pub fn reason(&self) -> Option<String> {
        match self {
            Self::Hit { .. } => None,
            Self::Miss { why, .. } => Some(why.to_string()),
            Self::Unavailable { why, .. } => Some(why.to_string()),
        }
    }
}

/// Chunk and persist one derivation output directly through the supplied local store.
///
/// # Errors
///
/// Returns a storage error when the record's output does not address `bytes`, a chunk or the
/// manifest cannot be persisted, or the canonical manifest cannot be encoded. Already-written
/// chunks remain valid content-addressed custody and may be reused by an explicit retry.
pub async fn put_tier_blob<S: EprfsStorage + ?Sized>(
    store: &S,
    record: DerivationRecord,
    bytes: Bytes,
) -> Result<TierBlobPut> {
    if !record.output.verifies(&bytes) {
        return Err(EprfsError::Storage(format!(
            "derivation output does not address supplied bytes: {}",
            record.output
        )));
    }

    let mut chunks = Vec::with_capacity(bytes.len().div_ceil(TIER_BLOB_CHUNK_BYTES));
    for chunk in bytes.chunks(TIER_BLOB_CHUNK_BYTES) {
        let cid = BlobCid::compute_raw(chunk);
        store
            .put_blob_verified(&cid, Bytes::copy_from_slice(chunk))
            .await?;
        chunks.push(TierBlobChunk {
            cid,
            byte_len: chunk.len() as u32,
        });
    }
    let manifest = TierBlobManifest {
        epoch: TIER_BLOB_MANIFEST_EPOCH,
        record,
        byte_len: bytes.len() as u64,
        chunks,
    };
    let (manifest_cid, manifest_bytes) = BlobCid::encode_manifest(&manifest)
        .map_err(|error| EprfsError::Storage(format!("tier blob manifest encoding: {error}")))?;
    store
        .put_blob_verified(&manifest_cid, Bytes::from(manifest_bytes))
        .await?;
    Ok(TierBlobPut {
        manifest_cid,
        manifest,
    })
}

/// Stream a local chunked artifact through a caller-owned sink.
///
/// At most one artifact chunk is held by this function at a time. Each chunk is checked against
/// its manifest CID and length before the sink sees it. The complete artifact is hashed
/// incrementally, so `Completed` also proves the derivation output CID without reassembling the
/// artifact. On any failure the sink may retain only the verified prefix and receives a non-
/// completed typed outcome.
pub async fn stream_tier_blob<S, T>(
    store: &S,
    manifest_cid: &BlobCid,
    sink: &mut T,
) -> TierBlobStreamDraw
where
    S: EprfsStorage + ?Sized,
    T: TierBlobSink + ?Sized,
{
    let manifest_handle = match store.fetch_blob(manifest_cid, FetchPolicy::LocalOnly).await {
        Ok(handle) if handle.cid == *manifest_cid && manifest_cid.verifies(&handle.bytes) => handle,
        Ok(_) | Err(EprfsError::Corrupt(_)) => {
            return TierBlobStreamDraw::Unavailable {
                manifest_cid: manifest_cid.clone(),
                why: TierBlobUnavailable::ManifestCorrupt,
            };
        }
        Err(EprfsError::BlobNotFound(_)) | Err(EprfsError::BlobNotLocal(_)) => {
            return TierBlobStreamDraw::Miss {
                manifest_cid: manifest_cid.clone(),
                why: TierBlobMiss::ManifestAbsent,
            };
        }
        Err(error) => {
            return TierBlobStreamDraw::Unavailable {
                manifest_cid: manifest_cid.clone(),
                why: TierBlobUnavailable::Store {
                    reason: error.to_string(),
                },
            };
        }
    };
    let manifest: TierBlobManifest = match serde_ipld_dagcbor::from_slice(&manifest_handle.bytes) {
        Ok(manifest) => manifest,
        Err(error) => {
            return TierBlobStreamDraw::Unavailable {
                manifest_cid: manifest_cid.clone(),
                why: TierBlobUnavailable::ManifestInvalid {
                    reason: error.to_string(),
                },
            };
        }
    };
    if let Err(why) = validate_manifest(&manifest) {
        return TierBlobStreamDraw::Unavailable {
            manifest_cid: manifest_cid.clone(),
            why,
        };
    }

    let mut complete_digest = Sha256::new();
    for chunk in &manifest.chunks {
        match store.fetch_blob(&chunk.cid, FetchPolicy::LocalOnly).await {
            Ok(handle)
                if handle.cid == chunk.cid
                    && chunk.cid.verifies(&handle.bytes)
                    && handle.bytes.len() == chunk.byte_len as usize =>
            {
                complete_digest.update(&handle.bytes);
                if let Err(error) = sink.chunk(chunk, handle.bytes) {
                    return TierBlobStreamDraw::Unavailable {
                        manifest_cid: manifest_cid.clone(),
                        why: TierBlobUnavailable::Sink {
                            reason: error.to_string(),
                        },
                    };
                }
            }
            Ok(_) | Err(EprfsError::Corrupt(_)) => {
                return TierBlobStreamDraw::Unavailable {
                    manifest_cid: manifest_cid.clone(),
                    why: TierBlobUnavailable::ChunkCorrupt {
                        cid: chunk.cid.clone(),
                    },
                };
            }
            Err(EprfsError::BlobNotFound(_)) | Err(EprfsError::BlobNotLocal(_)) => {
                return TierBlobStreamDraw::Unavailable {
                    manifest_cid: manifest_cid.clone(),
                    why: TierBlobUnavailable::ChunkMissing {
                        cid: chunk.cid.clone(),
                    },
                };
            }
            Err(error) => {
                return TierBlobStreamDraw::Unavailable {
                    manifest_cid: manifest_cid.clone(),
                    why: TierBlobUnavailable::Store {
                        reason: error.to_string(),
                    },
                };
            }
        }
    }
    let digest = complete_digest.finalize();
    if !matches_complete_digest(&manifest.record.output, &digest) {
        return TierBlobStreamDraw::Unavailable {
            manifest_cid: manifest_cid.clone(),
            why: TierBlobUnavailable::BlobCorrupt {
                cid: manifest.record.output.clone(),
            },
        };
    }
    TierBlobStreamDraw::Completed { manifest }
}

/// Read a complete local chunked artifact into memory.
///
/// This convenience wrapper is intended for small artifacts. Large-artifact callers should use
/// [`stream_tier_blob`] so they retain at most one chunk at a time.
pub async fn read_tier_blob<S: EprfsStorage + ?Sized>(
    store: &S,
    manifest_cid: &BlobCid,
) -> TierBlobDraw {
    let mut bytes = BytesMut::new();
    let mut collect = |_: &TierBlobChunk, chunk: Bytes| {
        bytes.extend_from_slice(&chunk);
        Ok(())
    };
    match stream_tier_blob(store, manifest_cid, &mut collect).await {
        TierBlobStreamDraw::Completed { manifest } => TierBlobDraw::Hit {
            manifest,
            bytes: bytes.freeze(),
        },
        TierBlobStreamDraw::Miss { manifest_cid, why } => TierBlobDraw::Miss { manifest_cid, why },
        TierBlobStreamDraw::Unavailable { manifest_cid, why } => {
            TierBlobDraw::Unavailable { manifest_cid, why }
        }
    }
}

fn matches_complete_digest(cid: &BlobCid, digest: &[u8]) -> bool {
    // BlobCid verifies only these codecs with SHA2-256. Comparing the incremental digest keeps
    // the same fail-closed contract without retaining the artifact merely to call `verifies`.
    const RAW_CODEC: u64 = 0x55;
    const DAG_CBOR_CODEC: u64 = 0x71;
    const SHA2_256: u64 = 0x12;

    let cid = cid.as_cid();
    matches!(cid.codec(), RAW_CODEC | DAG_CBOR_CODEC)
        && cid.hash().code() == SHA2_256
        && cid.hash().digest() == digest
}

fn validate_manifest(manifest: &TierBlobManifest) -> std::result::Result<(), TierBlobUnavailable> {
    if manifest.epoch != TIER_BLOB_MANIFEST_EPOCH {
        return Err(TierBlobUnavailable::UnsupportedEpoch {
            epoch: manifest.epoch,
        });
    }
    let mut total = 0u64;
    for chunk in &manifest.chunks {
        if chunk.byte_len == 0 || chunk.byte_len as usize > TIER_BLOB_CHUNK_BYTES {
            return Err(TierBlobUnavailable::ManifestInvalid {
                reason: format!(
                    "chunk {} has invalid byte length {}",
                    chunk.cid, chunk.byte_len
                ),
            });
        }
        total = total.checked_add(chunk.byte_len as u64).ok_or_else(|| {
            TierBlobUnavailable::ManifestInvalid {
                reason: "chunk lengths overflow".into(),
            }
        })?;
    }
    if total != manifest.byte_len {
        return Err(TierBlobUnavailable::ManifestInvalid {
            reason: format!(
                "chunk lengths total {total} bytes, manifest declares {}",
                manifest.byte_len
            ),
        });
    }
    if manifest.byte_len == 0 && !manifest.chunks.is_empty() {
        return Err(TierBlobUnavailable::ManifestInvalid {
            reason: "empty blob declares chunks".into(),
        });
    }
    Ok(())
}
