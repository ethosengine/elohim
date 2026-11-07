//! Local chunked custody for derivation artifacts.
//!
//! Chunks are cut by the byte profile's content-defined chunker (`fastcdc-2020`, epoch 0) and
//! each is addressed under the byte profile (`CIDv1(raw 0x55, BLAKE3-256)`); the manifest is a
//! canonical DAG-CBOR record (`CIDv1(dag-cbor 0x71, sha2-256)`) that records the profile it was
//! cut under, so a reader uses the recorded parameters, never the current defaults. This module
//! writes only to the caller's [`EprfsStorage`]; it creates no network head, publication, or
//! daemon dependency.
//!
//! Epoch-1 manifests (fixed 1 MiB chunks, raw sha2 chunk CIDs, no recorded profile) stay in
//! custody: the reader reads the epoch before it deserializes the epoch-2 schema and answers
//! them [`TierBlobUnavailable::UnsupportedEpoch`], never a hit and never a parse error
//! (re-genesis 1 design gate, dna-blast 4).

use std::fmt;

use bytes::{Bytes, BytesMut};
use elohim_epr::address::{
    BYTE_CODEC, BYTE_DIGEST_LEN, BYTE_MULTIHASH, CITE_CODEC, RECORD_CODEC, SHA2_256,
};
use elohim_epr::profile::{try_chunk_bytes, ByteProfile, Chunker, Erasure, BYTE_PROFILE_0};
use eprfs_core::{BlobCid, DerivationRecord, EprfsError, EprfsStorage, FetchPolicy, Result};
use ipld_core::ipld::Ipld;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The local manifest epoch. An unknown epoch is retained in custody but never read as a hit.
/// Epoch 2 is the byte-profile layout: content-defined chunks under BLAKE3 and a recorded
/// [`ByteProfile`].
pub const TIER_BLOB_MANIFEST_EPOCH: u32 = 2;

/// The profile every new tier blob is cut and addressed under: the epoch-0 byte profile's address
/// triple and chunker. The local tier store holds plain chunks, so the recorded erasure is
/// `none` — a manifest records the representation it actually is (representation is not
/// identity; the payload CID never depends on it).
pub const TIER_BLOB_PROFILE: ByteProfile = ByteProfile {
    erasure: Erasure::None,
    ..BYTE_PROFILE_0
};

/// One raw chunk named by a local tier blob manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TierBlobChunk {
    pub cid: BlobCid,
    pub byte_len: u32,
}

/// The canonical DAG-CBOR record for one chunked derivation output (epoch 2).
///
/// `record.output` is the CID of the complete artifact under its own declared profile (a byte
/// CID, or a sha2 record/citation CID). A reader verifies every chunk and then that complete CID
/// before returning bytes. `profile` is the byte profile the chunks were cut and addressed under;
/// readers bound chunk lengths by it. The record names the closure and signer claim; accepting
/// its witness or widening its reach remains the caller's decision.
///
/// D7: a known-epoch manifest refuses unknown fields outside `ext`; content under `ext` carries no
/// decision meaning and is retained, so a decode followed by an encode reproduces it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TierBlobManifest {
    pub epoch: u32,
    pub profile: ByteProfile,
    pub record: DerivationRecord,
    pub byte_len: u64,
    pub chunks: Vec<TierBlobChunk>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ext: Option<ManifestExt>,
}

/// Opaque content of a manifest's top-level `ext` namespace (D7), held as its DAG-CBOR bytes so
/// the manifest stays `Eq` and the content is never interpreted. Decoding keeps the value and
/// encoding writes it back; for the canonical DAG-CBOR a writer emits that is byte-exact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestExt(Vec<u8>);

impl ManifestExt {
    /// The canonical DAG-CBOR bytes of the retained `ext` value.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Serialize for ManifestExt {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let value: Ipld =
            serde_ipld_dagcbor::from_slice(&self.0).map_err(serde::ser::Error::custom)?;
        value.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ManifestExt {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let value = Ipld::deserialize(deserializer)?;
        serde_ipld_dagcbor::to_vec(&value)
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}

/// The one field every manifest epoch shares. Read before the epoch-2 schema so an older (or
/// newer) object is answered by its epoch, never by the first field its schema lacks.
#[derive(Deserialize)]
struct ManifestEpoch {
    epoch: u32,
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
    UnsupportedProfile { cid: BlobCid },
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
            Self::UnsupportedProfile { cid } => {
                write!(f, "complete blob address profile is unsupported: {cid}")
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
        manifest: Box<TierBlobManifest>,
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
        manifest: Box<TierBlobManifest>,
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

    // The whole profile is validated before any byte is cut (the kernel refuses, never chunks,
    // a profile outside what it admits).
    let layout = try_chunk_bytes(&TIER_BLOB_PROFILE, &bytes)
        .map_err(|error| EprfsError::Storage(format!("tier blob profile: {error}")))?;
    let mut chunks = Vec::with_capacity(layout.len());
    for (offset, len) in layout {
        let chunk = bytes.slice(offset..offset + len);
        let cid = BlobCid::compute_bytes(&chunk);
        let byte_len = u32::try_from(len).map_err(|_| {
            EprfsError::Storage(format!("tier blob chunk of {len} bytes exceeds u32"))
        })?;
        store.put_blob_verified(&cid, chunk).await?;
        chunks.push(TierBlobChunk { cid, byte_len });
    }
    let manifest = TierBlobManifest {
        epoch: TIER_BLOB_MANIFEST_EPOCH,
        profile: TIER_BLOB_PROFILE,
        record,
        byte_len: bytes.len() as u64,
        chunks,
        ext: None,
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
    let manifest = match decode_manifest(&manifest_handle.bytes) {
        Ok(manifest) => manifest,
        Err(why) => {
            return TierBlobStreamDraw::Unavailable {
                manifest_cid: manifest_cid.clone(),
                why,
            };
        }
    };
    let Some(mut complete_digest) = CompleteDigest::for_output(&manifest.record.output) else {
        return TierBlobStreamDraw::Unavailable {
            manifest_cid: manifest_cid.clone(),
            why: TierBlobUnavailable::UnsupportedProfile {
                cid: manifest.record.output.clone(),
            },
        };
    };
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
    if !complete_digest.matches(&manifest.record.output) {
        return TierBlobStreamDraw::Unavailable {
            manifest_cid: manifest_cid.clone(),
            why: TierBlobUnavailable::BlobCorrupt {
                cid: manifest.record.output.clone(),
            },
        };
    }
    TierBlobStreamDraw::Completed {
        manifest: Box::new(manifest),
    }
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

/// The complete-artifact digest, computed incrementally under the output CID's declared profile
/// so a reader proves the whole artifact without retaining it. Byte-profile outputs hash with
/// BLAKE3; record and citation outputs stay sha2-256. Any other profile has no digest here and
/// the draw is `Unavailable`, never a passing match.
enum CompleteDigest {
    Blake3(Box<blake3::Hasher>),
    Sha2(Sha256),
}

impl CompleteDigest {
    fn for_output(output: &BlobCid) -> Option<Self> {
        let cid = output.as_cid();
        let hash = cid.hash();
        match (cid.codec(), hash.code(), usize::from(hash.size())) {
            (BYTE_CODEC, BYTE_MULTIHASH, BYTE_DIGEST_LEN) => {
                Some(Self::Blake3(Box::new(blake3::Hasher::new())))
            }
            (CITE_CODEC | RECORD_CODEC, SHA2_256, 32) => Some(Self::Sha2(Sha256::new())),
            _ => None,
        }
    }

    fn update(&mut self, bytes: &[u8]) {
        match self {
            Self::Blake3(hasher) => {
                hasher.update(bytes);
            }
            Self::Sha2(hasher) => hasher.update(bytes),
        }
    }

    fn matches(self, output: &BlobCid) -> bool {
        let digest = output.as_cid().hash().digest();
        match self {
            Self::Blake3(hasher) => hasher.finalize().as_bytes().as_slice() == digest,
            Self::Sha2(hasher) => hasher.finalize().as_slice() == digest,
        }
    }
}

/// Read the epoch, then — only for the epoch this reader knows — the epoch-2 schema.
fn decode_manifest(bytes: &[u8]) -> std::result::Result<TierBlobManifest, TierBlobUnavailable> {
    let invalid = |error: serde_ipld_dagcbor::DecodeError<std::convert::Infallible>| {
        TierBlobUnavailable::ManifestInvalid {
            reason: error.to_string(),
        }
    };
    let ManifestEpoch { epoch } = serde_ipld_dagcbor::from_slice(bytes).map_err(invalid)?;
    if epoch != TIER_BLOB_MANIFEST_EPOCH {
        return Err(TierBlobUnavailable::UnsupportedEpoch { epoch });
    }
    let manifest: TierBlobManifest = serde_ipld_dagcbor::from_slice(bytes).map_err(invalid)?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

fn validate_manifest(manifest: &TierBlobManifest) -> std::result::Result<(), TierBlobUnavailable> {
    if manifest.epoch != TIER_BLOB_MANIFEST_EPOCH {
        return Err(TierBlobUnavailable::UnsupportedEpoch {
            epoch: manifest.epoch,
        });
    }
    let profile = &manifest.profile;
    profile
        .validate()
        .map_err(|error| TierBlobUnavailable::ManifestInvalid {
            reason: format!("recorded profile is unusable: {error}"),
        })?;
    let Chunker::Fastcdc2020 { max, .. } = profile.chunker;
    let mut total = 0u64;
    for chunk in &manifest.chunks {
        let cid = chunk.cid.as_cid();
        if (cid.codec(), cid.hash().code()) != (profile.codec, profile.multihash) {
            return Err(TierBlobUnavailable::ManifestInvalid {
                reason: format!(
                    "chunk {} is not addressed under the recorded profile",
                    chunk.cid
                ),
            });
        }
        if chunk.byte_len == 0 || chunk.byte_len > max {
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

/// Read and validate one tier manifest from `store`, local only, without reading its chunks: the
/// manifest bytes must hash to `manifest_cid` and decode under this reader's epoch.
///
/// A serving index uses this to learn which chunks an artifact names before it serves any of
/// them; a draw still verifies every chunk and the complete output ([`stream_tier_blob`]).
///
/// # Errors
///
/// [`TierBlobUnavailable::ManifestCorrupt`] when the stored bytes fail their address,
/// [`TierBlobUnavailable::Store`] when the store cannot answer (an absent manifest included), and
/// the decode refusals [`stream_tier_blob`] gives for the same bytes.
pub async fn read_tier_manifest<S: EprfsStorage + ?Sized>(
    store: &S,
    manifest_cid: &BlobCid,
) -> std::result::Result<TierBlobManifest, TierBlobUnavailable> {
    let handle = store
        .fetch_blob(manifest_cid, FetchPolicy::LocalOnly)
        .await
        .map_err(|error| match error {
            EprfsError::Corrupt(_) => TierBlobUnavailable::ManifestCorrupt,
            other => TierBlobUnavailable::Store {
                reason: other.to_string(),
            },
        })?;
    if handle.cid != *manifest_cid || !manifest_cid.verifies(&handle.bytes) {
        return Err(TierBlobUnavailable::ManifestCorrupt);
    }
    decode_manifest(&handle.bytes)
}
