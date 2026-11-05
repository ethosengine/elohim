//! Shard manifests and erasure coding under the DNA `ShardManifest` shape.
//!
//! A payload is described by its byte CID, its content-defined chunks (the profile's
//! chunker, the same cut [`crate::blob_store::BlobStore`] files it under) and, when the
//! store elects erasure coding, its **stripes**: contiguous runs of at most
//! [`STRIPE_MAX`] original bytes, each cut into `data` contiguous data shards of
//! `ceil(stripe_len / data)` bytes (the last zero-padded) plus `parity` parity shards over
//! `rs-gf8-0x11d-v1` (GF(2^8), polynomial 0x11d, systematic Vandermonde as
//! `reed-solomon-erasure` 6 computes it, shards in index order). Every shard is addressed
//! by its own byte CID over its padded bytes; reconstruction truncates each stripe to its
//! declared length.
//!
//! `stripes` is empty iff the erasure is `none`; then the chunks are the custody unit.
//! Whether a payload is erasure-coded is storage configuration ([`ShardConfig`]); what a
//! manifest declares is what every reader uses ([`ShardManifest::validate`]), so a
//! manifest minted under other thresholds reads the same.

use elohim_epr::address::{compute_bytes_cid, ByteCid};
use elohim_epr::profile::{
    chunk_bytes, Chunker, Erasure, ValidatedProfile, BYTE_PROFILE_0, CHUNKER_FASTCDC_2020,
    ERASURE_NONE, ERASURE_RS_GF8_0X11D_V1,
};
use reed_solomon_erasure::galois_8::ReedSolomon;
use serde::{Deserialize, Serialize};

use crate::blob_store::{byte_cid_serde, BlobStore, ChunkRef, Exists};

/// The most original bytes one stripe covers (16 MiB).
pub const STRIPE_MAX: usize = 16 * 1024 * 1024;

/// Above this many bytes the store elects erasure coding (storage configuration).
pub const RS_THRESHOLD: usize = 64 * 1024 * 1024;

/// Why a manifest cannot be minted, read or reconstructed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ShardError {
    #[error("unsupported chunker {0}")]
    Chunker(String),
    #[error("unsupported erasure {0}")]
    Erasure(String),
    #[error("manifest inconsistent: {0}")]
    Inconsistent(String),
    #[error("stripe {stripe}: need {need} shards, have {have}")]
    TooFewShards {
        stripe: u32,
        need: usize,
        have: usize,
    },
    #[error("shard {stripe}/{index} does not hash to its declared cid")]
    ShardMismatch { stripe: u32, index: u8 },
    #[error("reassembled payload hashes to {actual}, manifest declares {expected}")]
    PayloadMismatch { expected: String, actual: String },
    #[error("reed-solomon: {0}")]
    Codec(String),
}

/// The chunker a manifest declares (DNA `ChunkerDecl`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkerDecl {
    pub id: ChunkerId,
    pub min: u32,
    pub avg: u32,
    pub max: u32,
    pub level: u8,
}

/// The chunker ids this store reads (`fastcdc-2020`). The DNA carries the id as a
/// `String`; the wire form is the same, and an unknown id fails to decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkerId {
    #[serde(rename = "fastcdc-2020")]
    Fastcdc2020,
}

impl ChunkerDecl {
    /// The declaration of a kernel chunker.
    pub fn of(chunker: Chunker) -> Self {
        let Chunker::Fastcdc2020 {
            min,
            avg,
            max,
            level,
        } = chunker;
        Self {
            id: ChunkerId::Fastcdc2020,
            min,
            avg,
            max,
            level,
        }
    }

    /// The kernel chunker this declares, refusing parameters outside the admitted ranges.
    pub fn chunker(&self) -> Result<Chunker, ShardError> {
        let ChunkerId::Fastcdc2020 = self.id;
        let chunker = Chunker::Fastcdc2020 {
            min: self.min,
            avg: self.avg,
            max: self.max,
            level: self.level,
        };
        chunker
            .validate()
            .map_err(|error| ShardError::Chunker(format!("{CHUNKER_FASTCDC_2020}: {error}")))?;
        Ok(chunker)
    }
}

/// The erasure code a manifest declares (DNA `ErasureDecl`); `none` declares 0/0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErasureDecl {
    pub id: String,
    pub data: u8,
    pub parity: u8,
}

impl ErasureDecl {
    /// The declaration of a kernel erasure code.
    pub fn of(erasure: Erasure) -> Self {
        match erasure {
            Erasure::None => Self {
                id: ERASURE_NONE.to_string(),
                data: 0,
                parity: 0,
            },
            Erasure::RsGf8x11dV1 { data, parity } => Self {
                id: ERASURE_RS_GF8_0X11D_V1.to_string(),
                data,
                parity,
            },
        }
    }

    /// The kernel erasure code this declares. Unknown ids are refused (fail closed).
    pub fn erasure(&self) -> Result<Erasure, ShardError> {
        let erasure = match self.id.as_str() {
            ERASURE_NONE if self.data == 0 && self.parity == 0 => Erasure::None,
            ERASURE_RS_GF8_0X11D_V1 => Erasure::RsGf8x11dV1 {
                data: self.data,
                parity: self.parity,
            },
            _ => {
                return Err(ShardError::Erasure(format!(
                    "{}/{}+{}",
                    self.id, self.data, self.parity
                )))
            }
        };
        erasure
            .validate()
            .map_err(|error| ShardError::Erasure(error.to_string()))?;
        Ok(erasure)
    }
}

/// One erasure-coded shard of a stripe (DNA `ShardRef`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShardRef {
    #[serde(with = "byte_cid_serde")]
    pub cid: ByteCid,
    /// The shard's stored (padded) length.
    pub len: u32,
    /// Position in the stripe: `0..data` data, `data..data+parity` parity.
    pub index: u8,
}

/// One stripe: at most [`STRIPE_MAX`] original bytes and its `data + parity` shards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stripe {
    pub index: u32,
    /// Original bytes this stripe covers (before padding).
    pub byte_len: u32,
    pub shards: Vec<ShardRef>,
}

/// The shard manifest (DNA `ShardManifest`, `elohim` DNA `content_store` integrity).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShardManifest {
    #[serde(with = "byte_cid_serde")]
    pub blob_cid: ByteCid,
    pub byte_len: u64,
    pub mime_type: String,
    pub chunker: ChunkerDecl,
    pub chunks: Vec<ChunkRef>,
    pub erasure: ErasureDecl,
    /// Empty iff `erasure.id == "none"`.
    pub stripes: Vec<Stripe>,
    pub reach: String,
    pub author_id: Option<String>,
    pub created_at: String,
    pub verified_at: Option<String>,
}

impl ShardManifest {
    /// Check a manifest read from the wire or the DHT before acting on it: declared
    /// chunker and erasure are supported, chunks cover `byte_len`, stripes are contiguous,
    /// at most [`STRIPE_MAX`] each, cover `byte_len`, and carry exactly `data + parity`
    /// shards of `ceil(stripe_len / data)` bytes in index order.
    pub fn validate(&self) -> Result<(), ShardError> {
        self.chunker.chunker()?;
        let erasure = self.erasure.erasure()?;
        let chunk_total: u64 = self.chunks.iter().map(|c| u64::from(c.len)).sum();
        if chunk_total != self.byte_len {
            return Err(ShardError::Inconsistent(format!(
                "chunks cover {chunk_total} of {} bytes",
                self.byte_len
            )));
        }
        let Erasure::RsGf8x11dV1 { data, parity } = erasure else {
            return if self.stripes.is_empty() {
                Ok(())
            } else {
                Err(ShardError::Inconsistent(
                    "stripes under erasure none".into(),
                ))
            };
        };
        let mut covered = 0u64;
        for (position, stripe) in self.stripes.iter().enumerate() {
            if stripe.index as usize != position {
                return Err(ShardError::Inconsistent(format!(
                    "stripe {} at position {position}",
                    stripe.index
                )));
            }
            let len = stripe.byte_len as usize;
            if len == 0 || len > STRIPE_MAX {
                return Err(ShardError::Inconsistent(format!(
                    "stripe {} covers {len} bytes",
                    stripe.index
                )));
            }
            if stripe.shards.len() != usize::from(data) + usize::from(parity) {
                return Err(ShardError::Inconsistent(format!(
                    "stripe {} has {} shards, the code needs {}",
                    stripe.index,
                    stripe.shards.len(),
                    u16::from(data) + u16::from(parity)
                )));
            }
            let shard_len = len.div_ceil(usize::from(data));
            for (i, shard) in stripe.shards.iter().enumerate() {
                if usize::from(shard.index) != i || shard.len as usize != shard_len {
                    return Err(ShardError::Inconsistent(format!(
                        "stripe {} shard {i}: index {} len {}, expected len {shard_len}",
                        stripe.index, shard.index, shard.len
                    )));
                }
            }
            covered += u64::from(stripe.byte_len);
        }
        if covered != self.byte_len {
            return Err(ShardError::Inconsistent(format!(
                "stripes cover {covered} of {} bytes",
                self.byte_len
            )));
        }
        Ok(())
    }

    /// Every shard in stripe order: `(stripe index, shard)`.
    pub fn shards(&self) -> impl Iterator<Item = (u32, &ShardRef)> {
        self.stripes
            .iter()
            .flat_map(|stripe| stripe.shards.iter().map(move |s| (stripe.index, s)))
    }

    /// The units custody holds: shard CIDs when erasure-coded, else chunk CIDs.
    pub fn custody_units(&self) -> Vec<ByteCid> {
        if self.stripes.is_empty() {
            self.chunks.iter().map(|c| c.cid).collect()
        } else {
            self.shards().map(|(_, s)| s.cid).collect()
        }
    }

    /// Data shards per stripe (0 under `none`).
    pub fn data_shards(&self) -> u8 {
        self.erasure.data
    }
}

/// Whether a shard index is a data or a parity shard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShardRole {
    Data,
    Parity,
}

/// The role of shard `index` within a stripe of `manifest`. Without erasure every unit is
/// required data.
pub fn shard_role(manifest: &ShardManifest, index: usize) -> ShardRole {
    if !manifest.stripes.is_empty() && index >= usize::from(manifest.erasure.data) {
        ShardRole::Parity
    } else {
        ShardRole::Data
    }
}

/// One deterministic round-robin placement slot over a diversity-ranked holder list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShardPlacementSlot {
    /// Stripe the unit belongs to (0 for every chunk of an unstriped payload).
    pub stripe_index: u32,
    /// Position within the stripe (or the chunk's position).
    pub shard_index: usize,
    pub holder_index: usize,
    pub role: ShardRole,
}

/// Place every custody unit round-robin by its position within its stripe, so shard `i`
/// of every stripe lands on the same holder and the data shards take the strongest,
/// household-diverse prefix of `holder_count` before parity does.
pub fn plan_shard_placement_slots(
    manifest: &ShardManifest,
    holder_count: usize,
) -> Vec<ShardPlacementSlot> {
    if holder_count == 0 {
        return Vec::new();
    }
    if manifest.stripes.is_empty() {
        return (0..manifest.chunks.len())
            .map(|shard_index| ShardPlacementSlot {
                stripe_index: 0,
                shard_index,
                holder_index: shard_index % holder_count,
                role: ShardRole::Data,
            })
            .collect();
    }
    manifest
        .shards()
        .map(|(stripe_index, shard)| {
            let shard_index = usize::from(shard.index);
            ShardPlacementSlot {
                stripe_index,
                shard_index,
                holder_index: shard_index % holder_count,
                role: shard_role(manifest, shard_index),
            }
        })
        .collect()
}

/// Storage configuration for electing erasure coding.
#[derive(Debug, Clone)]
pub struct ShardConfig {
    /// Payloads above this many bytes are erasure-coded.
    pub rs_threshold: usize,
    pub rs_data_shards: u8,
    pub rs_parity_shards: u8,
}

impl Default for ShardConfig {
    fn default() -> Self {
        let Erasure::RsGf8x11dV1 { data, parity } = BYTE_PROFILE_0.erasure else {
            unreachable!("the epoch-0 profile declares rs-gf8-0x11d-v1")
        };
        Self {
            rs_threshold: RS_THRESHOLD,
            rs_data_shards: data,
            rs_parity_shards: parity,
        }
    }
}

/// The shards of one stripe, in index order.
pub type StripeShards = Vec<Vec<u8>>;

/// Mints manifests and their shards, and reconstructs payloads from them.
pub struct ShardEncoder {
    config: ShardConfig,
    stripe_max: usize,
}

impl ShardEncoder {
    pub fn new(config: ShardConfig) -> Self {
        Self {
            config,
            stripe_max: STRIPE_MAX,
        }
    }

    /// The erasure code this store elects for a payload of `byte_len` bytes.
    pub fn erasure_for(&self, byte_len: u64) -> Erasure {
        if byte_len > self.config.rs_threshold as u64 {
            Erasure::RsGf8x11dV1 {
                data: self.config.rs_data_shards,
                parity: self.config.rs_parity_shards,
            }
        } else {
            Erasure::None
        }
    }

    /// Mint a manifest for `data`, returning the shards of every stripe beside it.
    pub fn encode(
        &self,
        data: &[u8],
        mime_type: &str,
        reach: &str,
    ) -> Result<(ShardManifest, Vec<StripeShards>), ShardError> {
        let erasure = self.erasure_for(data.len() as u64);
        let chunks = chunk_bytes(&ValidatedProfile::EPOCH_0, data)
            .into_iter()
            .map(|(offset, len)| ChunkRef {
                cid: compute_bytes_cid(&data[offset..offset + len]),
                // A chunk is at most the chunker's max (4 MiB).
                len: len as u32,
            })
            .collect();
        let (stripes, shards) = match erasure {
            Erasure::None => (Vec::new(), Vec::new()),
            Erasure::RsGf8x11dV1 { data: d, parity: p } => {
                encode_stripes(data, d, p, self.stripe_max)?
            }
        };
        let now = chrono::Utc::now().to_rfc3339();
        let manifest = ShardManifest {
            blob_cid: compute_bytes_cid(data),
            byte_len: data.len() as u64,
            mime_type: mime_type.to_string(),
            chunker: ChunkerDecl::of(BYTE_PROFILE_0.chunker),
            chunks,
            erasure: ErasureDecl::of(erasure),
            stripes,
            reach: reach.to_string(),
            author_id: None,
            created_at: now.clone(),
            verified_at: Some(now),
        };
        Ok((manifest, shards))
    }

    /// Mint a manifest for `data` (shards discarded).
    pub fn create_manifest(
        &self,
        data: &[u8],
        mime_type: &str,
        reach: &str,
    ) -> Result<ShardManifest, ShardError> {
        self.encode(data, mime_type, reach)
            .map(|(manifest, _)| manifest)
    }

    /// Reassemble a payload from the shards a manifest declares (`stripes[s][i]` is shard
    /// `i` of stripe `s`, `None` when missing). Each present shard is checked against its
    /// CID before use; a stripe needs `data` good shards; the result is checked against
    /// `blob_cid`. The manifest is validated first; its recorded parameters are used.
    pub fn reconstruct(
        manifest: &ShardManifest,
        stripes: &[Vec<Option<Vec<u8>>>],
    ) -> Result<Vec<u8>, ShardError> {
        manifest.validate()?;
        let Erasure::RsGf8x11dV1 { data, parity } = manifest.erasure.erasure()? else {
            return Err(ShardError::Inconsistent(
                "an unstriped payload is reassembled from its chunks".into(),
            ));
        };
        if stripes.len() != manifest.stripes.len() {
            return Err(ShardError::Inconsistent(format!(
                "{} stripes supplied, manifest declares {}",
                stripes.len(),
                manifest.stripes.len()
            )));
        }
        let rs = ReedSolomon::new(usize::from(data), usize::from(parity))
            .map_err(|error| ShardError::Codec(error.to_string()))?;
        let mut out = Vec::with_capacity(usize::try_from(manifest.byte_len).unwrap_or(0));
        for (declared, supplied) in manifest.stripes.iter().zip(stripes) {
            out.extend_from_slice(&reconstruct_stripe(&rs, declared, supplied, data)?);
        }
        let actual = compute_bytes_cid(&out);
        if actual != manifest.blob_cid {
            return Err(ShardError::PayloadMismatch {
                expected: manifest.blob_cid.to_string(),
                actual: actual.to_string(),
            });
        }
        Ok(out)
    }
}

/// Cut `data` into stripes of at most `stripe_max` bytes and erasure-code each.
fn encode_stripes(
    data: &[u8],
    d: u8,
    p: u8,
    stripe_max: usize,
) -> Result<(Vec<Stripe>, Vec<StripeShards>), ShardError> {
    let rs = ReedSolomon::new(usize::from(d), usize::from(p))
        .map_err(|error| ShardError::Codec(error.to_string()))?;
    let mut stripes = Vec::new();
    let mut all = Vec::new();
    for (index, original) in data.chunks(stripe_max).enumerate() {
        let shard_len = original.len().div_ceil(usize::from(d));
        let mut shards: Vec<Vec<u8>> = (0..usize::from(d))
            .map(|i| {
                let start = (i * shard_len).min(original.len());
                let end = ((i + 1) * shard_len).min(original.len());
                let mut shard = original[start..end].to_vec();
                shard.resize(shard_len, 0);
                shard
            })
            .collect();
        shards.extend((0..p).map(|_| vec![0u8; shard_len]));
        rs.encode(&mut shards)
            .map_err(|error| ShardError::Codec(error.to_string()))?;
        stripes.push(Stripe {
            // At most 2^32 stripes of 16 MiB: far beyond any payload this store accepts.
            index: index as u32,
            byte_len: original.len() as u32,
            shards: shards
                .iter()
                .enumerate()
                .map(|(i, shard)| ShardRef {
                    cid: compute_bytes_cid(shard),
                    len: shard_len as u32,
                    index: i as u8,
                })
                .collect(),
        });
        all.push(shards);
    }
    Ok((stripes, all))
}

fn reconstruct_stripe(
    rs: &ReedSolomon,
    declared: &Stripe,
    supplied: &[Option<Vec<u8>>],
    data: u8,
) -> Result<Vec<u8>, ShardError> {
    let mut shards: Vec<Option<Vec<u8>>> = Vec::with_capacity(declared.shards.len());
    for (i, shard_ref) in declared.shards.iter().enumerate() {
        let shard = supplied.get(i).cloned().flatten();
        match shard {
            Some(bytes) if compute_bytes_cid(&bytes) != shard_ref.cid => {
                return Err(ShardError::ShardMismatch {
                    stripe: declared.index,
                    index: shard_ref.index,
                })
            }
            other => shards.push(other),
        }
    }
    let have = shards.iter().filter(|s| s.is_some()).count();
    if have < usize::from(data) {
        return Err(ShardError::TooFewShards {
            stripe: declared.index,
            need: usize::from(data),
            have,
        });
    }
    rs.reconstruct_data(&mut shards)
        .map_err(|error| ShardError::Codec(error.to_string()))?;
    let mut stripe = Vec::with_capacity(declared.byte_len as usize);
    for shard in shards.into_iter().take(usize::from(data)) {
        stripe.extend_from_slice(&shard.unwrap_or_default());
    }
    stripe.truncate(declared.byte_len as usize);
    Ok(stripe)
}

/// Do we hold a payload as its custody units? Under `none`, the payload itself must
/// verify; erasure-coded, every stripe needs `data` verified shards.
pub async fn holds_as_shards(store: &BlobStore, manifest: &ShardManifest) -> bool {
    if manifest.stripes.is_empty() {
        return store.exists(&manifest.blob_cid).await == Exists::Verified;
    }
    for stripe in &manifest.stripes {
        let mut present = 0usize;
        for shard in &stripe.shards {
            if store.exists(&shard.cid).await == Exists::Verified {
                present += 1;
            }
        }
        if present < usize::from(manifest.erasure.data) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blob_store::tests::xorshift_bytes;

    fn small_stripes() -> ShardEncoder {
        ShardEncoder {
            config: ShardConfig {
                rs_threshold: 0,
                ..ShardConfig::default()
            },
            stripe_max: 1001,
        }
    }

    fn all_present(shards: &[StripeShards]) -> Vec<Vec<Option<Vec<u8>>>> {
        shards
            .iter()
            .map(|stripe| stripe.iter().cloned().map(Some).collect())
            .collect()
    }

    #[test]
    fn erasure_for_follows_the_configured_threshold() {
        let enc = ShardEncoder::new(ShardConfig::default());
        assert_eq!(enc.erasure_for(RS_THRESHOLD as u64), Erasure::None);
        assert_eq!(
            enc.erasure_for(RS_THRESHOLD as u64 + 1),
            Erasure::RsGf8x11dV1 { data: 4, parity: 3 }
        );
    }

    #[test]
    fn unstriped_manifest_declares_chunks_and_no_stripes() {
        let enc = ShardEncoder::new(ShardConfig::default());
        let data = xorshift_bytes(3 * 1024 * 1024);
        let manifest = enc.create_manifest(&data, "text/plain", "commons").unwrap();
        manifest.validate().unwrap();
        assert_eq!(manifest.erasure.id, "none");
        assert!(manifest.stripes.is_empty());
        assert_eq!(manifest.blob_cid, compute_bytes_cid(&data));
        assert!(manifest.blob_cid.to_string().starts_with("bafkr4i"));
        assert!(!manifest.chunks.is_empty());
        assert_eq!(manifest.custody_units().len(), manifest.chunks.len());
    }

    #[test]
    fn rs_4_3_reconstructs_from_every_4_of_7_per_stripe() {
        // 2,503 bytes in 1,001-byte stripes: three stripes, every one padded
        // (1001 = 4*251 - 3, and the 501-byte tail stripe = 4*126 - 3).
        let data = xorshift_bytes(2_503);
        let (manifest, shards) = small_stripes()
            .encode(&data, "application/octet-stream", "commons")
            .unwrap();
        manifest.validate().unwrap();
        assert_eq!(manifest.stripes.len(), 3);
        assert_eq!(manifest.stripes[0].shards[0].len, 251);
        assert_eq!(manifest.stripes[2].byte_len, 501);
        assert_eq!(manifest.stripes[2].shards[0].len, 126);
        for stripe in 0..manifest.stripes.len() {
            for mask in 0u8..128 {
                if mask.count_ones() != 4 {
                    continue;
                }
                let mut supplied = all_present(&shards);
                for (i, slot) in supplied[stripe].iter_mut().enumerate() {
                    if mask & (1 << i) == 0 {
                        *slot = None;
                    }
                }
                let out = ShardEncoder::reconstruct(&manifest, &supplied).unwrap();
                assert_eq!(out, data, "stripe {stripe}, kept shards {mask:07b}");
            }
        }
        // Three of seven is not enough.
        let mut supplied = all_present(&shards);
        for slot in supplied[1].iter_mut().take(4) {
            *slot = None;
        }
        assert!(matches!(
            ShardEncoder::reconstruct(&manifest, &supplied),
            Err(ShardError::TooFewShards {
                stripe: 1,
                need: 4,
                have: 3
            })
        ));
    }

    #[test]
    fn a_substituted_shard_is_refused_not_decoded() {
        let data = xorshift_bytes(2_503);
        let (manifest, shards) = small_stripes().encode(&data, "x", "commons").unwrap();
        let mut supplied = all_present(&shards);
        if let Some(Some(shard)) = supplied[0].get_mut(5) {
            shard[0] ^= 1;
        }
        assert_eq!(
            ShardEncoder::reconstruct(&manifest, &supplied),
            Err(ShardError::ShardMismatch {
                stripe: 0,
                index: 5
            })
        );
    }

    #[test]
    fn stripes_never_exceed_16mib() {
        let enc = ShardEncoder::new(ShardConfig {
            rs_threshold: 0,
            ..ShardConfig::default()
        });
        let data = xorshift_bytes(STRIPE_MAX * 2 + 3);
        let manifest = enc.create_manifest(&data, "x", "commons").unwrap();
        manifest.validate().unwrap();
        let lens: Vec<u32> = manifest.stripes.iter().map(|s| s.byte_len).collect();
        assert_eq!(lens, vec![STRIPE_MAX as u32, STRIPE_MAX as u32, 3]);
        assert!(manifest
            .stripes
            .iter()
            .all(|s| s.byte_len as usize <= STRIPE_MAX));

        // A manifest declaring a larger stripe is refused on read.
        let mut oversized = manifest.clone();
        oversized.stripes[0].byte_len = STRIPE_MAX as u32 + 1;
        assert!(matches!(
            oversized.validate(),
            Err(ShardError::Inconsistent(_))
        ));
    }

    /// The `rs-gf8-0x11d-v1` parity of a fixed input, pinned. A change here is a change
    /// of erasure code and needs a new id.
    #[test]
    fn parity_vectors_are_pinned() {
        let data = xorshift_bytes(1_000);
        let (manifest, shards) = small_stripes().encode(&data, "x", "commons").unwrap();
        let cids: Vec<String> = manifest.stripes[0]
            .shards
            .iter()
            .map(|s| s.cid.to_string())
            .collect();
        assert_eq!(shards[0][0], data[..250]);
        assert_eq!(
            cids[4..],
            PINNED_PARITY
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()[..],
            "parity shards of xorshift64*[..1000] under rs-gf8-0x11d-v1 (4+3)"
        );
        // Independently of the library: GF(2^8) over 0x11d, systematic matrix
        // V * inverse(V[..4]) with V[r][c] = r^c, parity rows applied byte-wise.
        let data_shards: Vec<&[u8]> = shards[0][..4].iter().map(Vec::as_slice).collect();
        for (row, parity) in spec_parity(&data_shards).iter().enumerate() {
            assert_eq!(parity, &shards[0][4 + row], "parity row {row}");
        }
    }

    const PINNED_PARITY: [&str; 3] = [
        "bafkr4ihx33s2azb2d5cc46nvcj3dbmngtf2c4l2fca54tj63fbjespl33q",
        "bafkr4if3m45dy22cfll6o7ztdhwjvdk4uy5k6im6ikzreepfy6kftkgnjm",
        "bafkr4ia6pyu2kgicxyfb2wurhl63fqoivu55d4pn653z2ap74pb3j7fsta",
    ];

    fn gf_mul(mut a: u8, mut b: u8) -> u8 {
        let mut product = 0u8;
        while b != 0 {
            if b & 1 != 0 {
                product ^= a;
            }
            let carry = a & 0x80 != 0;
            a <<= 1;
            if carry {
                a ^= 0x1d; // x^8 = x^4 + x^3 + x^2 + 1 (0x11d)
            }
            b >>= 1;
        }
        product
    }

    fn gf_pow(base: u8, exp: usize) -> u8 {
        (0..exp).fold(1, |acc, _| gf_mul(acc, base))
    }

    fn gf_inv(a: u8) -> u8 {
        (1..=255u8).find(|b| gf_mul(a, *b) == 1).unwrap()
    }

    /// The three parity shards of `rs-gf8-0x11d-v1` (4+3), from the spec text alone.
    fn spec_parity(data: &[&[u8]]) -> Vec<Vec<u8>> {
        let (d, total) = (4usize, 7usize);
        let v: Vec<Vec<u8>> = (0..total)
            .map(|r| (0..d).map(|c| gf_pow(r as u8, c)).collect())
            .collect();
        // Invert the top d x d block by Gauss-Jordan elimination.
        let mut m: Vec<Vec<u8>> = (0..d)
            .map(|r| {
                let mut row = v[r].clone();
                row.extend((0..d).map(|c| u8::from(c == r)));
                row
            })
            .collect();
        for col in 0..d {
            let pivot = (col..d).find(|r| m[*r][col] != 0).unwrap();
            m.swap(col, pivot);
            let inv = gf_inv(m[col][col]);
            for x in m[col].iter_mut() {
                *x = gf_mul(*x, inv);
            }
            for r in 0..d {
                if r != col && m[r][col] != 0 {
                    let factor = m[r][col];
                    let pivot_row = m[col].clone();
                    for (x, p) in m[r].iter_mut().zip(pivot_row) {
                        *x ^= gf_mul(factor, p);
                    }
                }
            }
        }
        let top_inv: Vec<Vec<u8>> = m.iter().map(|row| row[d..].to_vec()).collect();
        (d..total)
            .map(|r| {
                let coeffs: Vec<u8> = (0..d)
                    .map(|c| (0..d).fold(0, |acc, k| acc ^ gf_mul(v[r][k], top_inv[k][c])))
                    .collect();
                (0..data[0].len())
                    .map(|i| (0..d).fold(0, |acc, c| acc ^ gf_mul(coeffs[c], data[c][i])))
                    .collect()
            })
            .collect()
    }

    #[test]
    fn manifest_wire_shape_matches_the_dna() {
        let data = xorshift_bytes(2_503);
        let manifest = small_stripes()
            .create_manifest(&data, "x", "commons")
            .unwrap();
        let json = serde_json::to_value(&manifest).unwrap();
        let keys: Vec<&str> = json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        for key in [
            "blob_cid",
            "byte_len",
            "mime_type",
            "chunker",
            "chunks",
            "erasure",
            "stripes",
            "reach",
            "author_id",
            "created_at",
            "verified_at",
        ] {
            assert!(keys.contains(&key), "{key} missing from {keys:?}");
        }
        assert_eq!(json["chunker"]["id"], "fastcdc-2020");
        assert_eq!(json["erasure"]["id"], "rs-gf8-0x11d-v1");
        assert!(json["stripes"][0]["shards"][0]["cid"]
            .as_str()
            .unwrap()
            .starts_with("bafkr4i"));
        let back: ShardManifest = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(back, manifest);

        // A sha256 spelling in a shard cid is refused at decode.
        let mut tampered = json;
        tampered["stripes"][0]["shards"][0]["cid"] =
            serde_json::Value::String(format!("sha256-{}", "a".repeat(64)));
        assert!(serde_json::from_value::<ShardManifest>(tampered).is_err());
    }

    #[test]
    fn unknown_erasure_and_chunker_ids_are_refused() {
        let data = xorshift_bytes(2_503);
        let manifest = small_stripes()
            .create_manifest(&data, "x", "commons")
            .unwrap();
        let mut rs_4_7 = manifest.clone();
        rs_4_7.erasure.id = "rs-4-7".to_string();
        assert!(matches!(rs_4_7.validate(), Err(ShardError::Erasure(_))));
        let json = serde_json::to_value(&manifest).unwrap();
        let mut other_chunker = json;
        other_chunker["chunker"]["id"] = "fastcdc-2016".into();
        assert!(serde_json::from_value::<ShardManifest>(other_chunker).is_err());
    }

    #[test]
    fn rs_roles_and_placement_follow_the_stripe_position() {
        let data = xorshift_bytes(2_503);
        let manifest = small_stripes()
            .create_manifest(&data, "x", "commons")
            .unwrap();
        assert!((0..4).all(|i| shard_role(&manifest, i) == ShardRole::Data));
        assert!((4..7).all(|i| shard_role(&manifest, i) == ShardRole::Parity));
        // Diversity-ranked holders A,B,C,A,B,C: data shards take the first three
        // households before parity, in every stripe.
        let plan = plan_shard_placement_slots(&manifest, 6);
        assert_eq!(plan.len(), 21);
        for slot in &plan {
            assert_eq!(slot.holder_index, slot.shard_index % 6);
        }
        let households = ["A", "B", "C", "A", "B", "C"];
        let data_households: std::collections::BTreeSet<_> = plan
            .iter()
            .filter(|s| s.role == ShardRole::Data)
            .map(|s| households[s.holder_index])
            .collect();
        assert_eq!(data_households.len(), 3);

        let unstriped = ShardEncoder::new(ShardConfig::default())
            .create_manifest(&xorshift_bytes(3 * 1024 * 1024), "x", "commons")
            .unwrap();
        let plan = plan_shard_placement_slots(&unstriped, 2);
        assert_eq!(plan.len(), unstriped.chunks.len());
        assert!(plan.iter().all(|s| s.role == ShardRole::Data));
    }

    /// The 454 MiB acceptance: stripe count, every shard far under the 64 MiB transport
    /// frame, and reconstruction with three shards lost in every stripe.
    #[test]
    fn acceptance_454mib_payload_stripes_and_reconstructs() {
        const LEN: usize = 454 * 1024 * 1024;
        const FRAME: usize = 64 * 1024 * 1024;
        let data = xorshift_bytes(LEN);
        let (manifest, shards) = ShardEncoder::new(ShardConfig::default())
            .encode(&data, "video/mp4", "commons")
            .unwrap();
        manifest.validate().unwrap();
        assert_eq!(manifest.stripes.len(), LEN.div_ceil(STRIPE_MAX));
        assert_eq!(manifest.stripes.len(), 29);
        assert_eq!(
            manifest.stripes[28].byte_len as usize,
            LEN - 28 * STRIPE_MAX
        );
        let largest = manifest
            .shards()
            .map(|(_, s)| s.len as usize)
            .max()
            .unwrap();
        assert_eq!(largest, STRIPE_MAX / 4, "a full stripe's shard is 4 MiB");
        assert!(largest < FRAME, "every shard fits one transport frame");
        assert_eq!(manifest.shards().count(), 29 * 7);

        // Lose a different three shards in every stripe.
        let supplied: Vec<Vec<Option<Vec<u8>>>> = shards
            .into_iter()
            .enumerate()
            .map(|(s, stripe)| {
                stripe
                    .into_iter()
                    .enumerate()
                    .map(|(i, shard)| ((i + s) % 7 >= 3).then_some(shard))
                    .collect()
            })
            .collect();
        let out = ShardEncoder::reconstruct(&manifest, &supplied).unwrap();
        assert_eq!(out.len(), LEN);
        assert!(out == data);
    }
}
