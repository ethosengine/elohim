//! Byte and record profiles — every primitive that can age, declared by id + parameters.
//!
//! An address names its hash (CIDv1's multihash); a stored representation names its chunker
//! and its erasure code the same way. This module declares the epoch-0 profiles and owns the
//! byte profile's chunker. The addresses themselves are minted and checked by
//! [`elohim_address`] (re-exported as [`crate::address`]); nothing here re-derives them.
//!
//! | Profile | Epoch 0 |
//! |---|---|
//! | [`BYTE_PROFILE_0`] | raw `0x55` + BLAKE3-256 `0x1e`, chunker `fastcdc-2020 {262144, 1048576, 4194304, level 1}`, erasure `rs-gf8-0x11d-v1 {data 4, parity 3}` |
//! | [`RECORD_PROFILE_0`] | dag-cbor `0x71` + sha2-256 `0x12` |
//!
//! Readers use the parameters a manifest *records*, never the current defaults: two producers
//! may hold different representations of one payload CID, and the payload CID never depends on
//! the representation.
//!
//! # The `fastcdc-2020` chunker (immutable specification, epoch 0)
//!
//! This text is the specification; the `fastcdc` crate at exactly `=5.0.0`, module `v2020`,
//! `FastCDC::with_level(bytes, min, avg, max, Normalization::Level{level})` with seed 0, is its
//! reference implementation, and `tests/chunker_vectors.rs` re-implements the text below
//! independently and requires both to agree with the pinned vectors.
//!
//! **Tables.** `GEAR[256]` and `GEAR_LS[256]` are the crate's published, *unseeded* v2020 gear
//! tables (`GEAR_LS[i] = GEAR[i] << 1`), and `MASKS[26]` its mask table. They are identified by
//! [`FASTCDC_2020_TABLE_DIGEST`]: the BLAKE3-256 of `GEAR ‖ GEAR_LS ‖ MASKS`, each entry as
//! 8 little-endian bytes. A dependency that changes any entry fails
//! `chunker_tables_match_the_declared_digest`. There is no seed field and never will be one in
//! this profile id: a seed is a different chunker.
//!
//! **Masks.** `bits = round(log2(avg))` (round half away from zero), `mask_s = MASKS[bits +
//! level]`, `mask_l = MASKS[bits - level]`, `mask_s_ls = mask_s << 1`, `mask_l_ls = mask_l << 1`.
//!
//! **Arithmetic.** All hash arithmetic is unsigned 64-bit: shifts discard the bits shifted out,
//! additions wrap modulo 2^64. (A TypeScript twin uses `BigInt` and masks to 64 bits.)
//!
//! **Cut.** Given the window `w` of the `n` bytes not yet chunked:
//! 1. if `n <= min`, the chunk is all `n` bytes (when `n = 0` there is no chunk);
//! 2. otherwise `limit = min(n, max)`, `center = avg` if `limit >= avg` else `limit`;
//!    `hash = 0`; for `i` from `min / 2` while `i < center / 2`: with `a = 2i`,
//!    `hash = (hash << 2) + GEAR_LS[w[a]]` and if `hash & mask_s_ls == 0` the chunk is
//!    `w[0..a]`; then `hash = hash + GEAR[w[a + 1]]` and if `hash & mask_s == 0` the chunk is
//!    `w[0..a + 1]`;
//! 3. continue the same loop while `i < limit / 2` with `mask_l_ls` and `mask_l`;
//! 4. if no test passed, the chunk is `w[0..limit]`.
//!
//! The next window starts where the chunk ends. **End of input:** the final chunk is whatever
//! remains and may be shorter than `min`; empty input yields zero chunks. `min`, `avg` and
//! `max` are even, `64 <= min <= 1 MiB`, `256 <= avg <= 4 MiB`, `1 KiB <= max <= 16 MiB`,
//! `min <= avg <= max`, `level <= 3`. Streaming input yields the same chunks as one-shot input,
//! whatever the read sizes.
//!
//! # The `rs-gf8-0x11d-v1` erasure code
//!
//! Reed–Solomon over GF(2^8) with the field polynomial `0x11d`, systematic Vandermonde
//! construction as `reed-solomon-erasure` 6.0 builds it, shards in index order (data first),
//! zero padding to the shard length and truncation to the recorded original length. This module
//! declares the id and parameters; the stripe layout and parity vectors belong to the store
//! that encodes (plan §2.5 amendment 7).

use std::io::Read;

use fastcdc::v2020::{FastCDC, Normalization, StreamCDC};
use multihash_codetable::{Code, MultihashDigest};
use serde::{Deserialize, Serialize};

pub use elohim_address::{
    BYTE_CID_LEN, BYTE_CID_PREFIX, BYTE_CODEC, BYTE_DIGEST_LEN, BYTE_MULTIHASH, CITE_CID_PREFIX,
    RECORD_CID_PREFIX, RECORD_CODEC, SHA2_256,
};

/// The byte profile epoch this crate mints under.
pub const BYTE_PROFILE_EPOCH: u16 = 0;
/// The record profile epoch this crate mints under.
pub const RECORD_PROFILE_EPOCH: u16 = 0;

/// Wire id of the FastCDC 2020 chunker.
pub const CHUNKER_FASTCDC_2020: &str = "fastcdc-2020";
/// Wire id of "no erasure coding": the chunks are the custody unit.
pub const ERASURE_NONE: &str = "none";
/// Wire id of Reed–Solomon over GF(2^8), polynomial 0x11d, version 1.
pub const ERASURE_RS_GF8_0X11D_V1: &str = "rs-gf8-0x11d-v1";

/// BLAKE3-256 (hex) of the fastcdc v2020 tables `GEAR ‖ GEAR_LS ‖ MASKS`, each entry as 8
/// little-endian bytes. Identifies the tables the `fastcdc-2020` chunker is defined over.
pub const FASTCDC_2020_TABLE_DIGEST: &str =
    "3bd9b87949825cd9b2a592d199dc15d485812848535c9b1d2ff6979baf7a8001";

/// A content-defined chunker, declared by id and parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "id")]
pub enum Chunker {
    /// FastCDC 2020 over the unseeded v2020 tables (see the module documentation).
    #[serde(rename = "fastcdc-2020")]
    Fastcdc2020 {
        /// Minimum chunk length in bytes (the final chunk may be shorter).
        min: u32,
        /// Target average chunk length in bytes.
        avg: u32,
        /// Maximum chunk length in bytes.
        max: u32,
        /// Normalization level, 0..=3.
        level: u8,
    },
}

impl Chunker {
    /// The chunker's wire id.
    pub fn id(&self) -> &'static str {
        match self {
            Chunker::Fastcdc2020 { .. } => CHUNKER_FASTCDC_2020,
        }
    }

    /// Whether the parameters are inside the ranges the specification admits.
    pub fn validate(&self) -> Result<(), ProfileError> {
        let Chunker::Fastcdc2020 {
            min,
            avg,
            max,
            level,
        } = *self;
        let (min, avg, max) = (min as usize, avg as usize, max as usize);
        let in_range = (fastcdc::v2020::MINIMUM_MIN..=fastcdc::v2020::MINIMUM_MAX).contains(&min)
            && (fastcdc::v2020::AVERAGE_MIN..=fastcdc::v2020::AVERAGE_MAX).contains(&avg)
            && (fastcdc::v2020::MAXIMUM_MIN..=fastcdc::v2020::MAXIMUM_MAX).contains(&max)
            && min <= avg
            && avg <= max
            && min.is_multiple_of(2)
            && avg.is_multiple_of(2)
            && max.is_multiple_of(2)
            && level <= 3;
        if in_range {
            Ok(())
        } else {
            Err(ProfileError::ChunkerParameters(*self))
        }
    }

    fn normalization(level: u8) -> Normalization {
        match level {
            0 => Normalization::Level0,
            1 => Normalization::Level1,
            2 => Normalization::Level2,
            _ => Normalization::Level3,
        }
    }
}

/// An erasure code, declared by id and parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "id")]
pub enum Erasure {
    /// No erasure coding.
    #[serde(rename = "none")]
    None,
    /// Reed–Solomon over GF(2^8), polynomial 0x11d, systematic Vandermonde, version 1.
    #[serde(rename = "rs-gf8-0x11d-v1")]
    RsGf8x11dV1 {
        /// Data shards per stripe.
        data: u8,
        /// Parity shards per stripe.
        parity: u8,
    },
}

impl Erasure {
    /// The erasure code's wire id.
    pub fn id(&self) -> &'static str {
        match self {
            Erasure::None => ERASURE_NONE,
            Erasure::RsGf8x11dV1 { .. } => ERASURE_RS_GF8_0X11D_V1,
        }
    }

    /// Whether the parameters describe a constructible code (at least one data and one parity
    /// shard, at most 255 shards in all over GF(2^8)).
    pub fn validate(&self) -> Result<(), ProfileError> {
        match *self {
            Erasure::None => Ok(()),
            Erasure::RsGf8x11dV1 { data, parity } => {
                if data >= 1 && parity >= 1 && u16::from(data) + u16::from(parity) <= 255 {
                    Ok(())
                } else {
                    Err(ProfileError::ErasureParameters(*self))
                }
            }
        }
    }
}

/// How payload bytes are addressed and represented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ByteProfile {
    /// Profile epoch.
    pub epoch: u16,
    /// CID codec of payload addresses.
    pub codec: u64,
    /// Multihash code of payload addresses.
    pub multihash: u64,
    /// The chunker that cuts payloads into chunks.
    pub chunker: Chunker,
    /// The erasure code applied to stripes, when a store elects one.
    pub erasure: Erasure,
}

/// The epoch-0 byte profile.
pub const BYTE_PROFILE_0: ByteProfile = ByteProfile {
    epoch: 0,
    codec: BYTE_CODEC,
    multihash: BYTE_MULTIHASH,
    chunker: Chunker::Fastcdc2020 {
        min: 262_144,
        avg: 1_048_576,
        max: 4_194_304,
        level: 1,
    },
    erasure: Erasure::RsGf8x11dV1 { data: 4, parity: 3 },
};

impl ByteProfile {
    /// Whether this profile is one this crate can act on: a known epoch whose address triple
    /// matches, and chunker and erasure parameters inside their admitted ranges.
    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.epoch != BYTE_PROFILE_EPOCH {
            return Err(ProfileError::UnknownEpoch(self.epoch));
        }
        if (self.codec, self.multihash) != (BYTE_CODEC, BYTE_MULTIHASH) {
            return Err(ProfileError::AddressTriple {
                codec: self.codec,
                multihash: self.multihash,
            });
        }
        self.chunker.validate()?;
        self.erasure.validate()
    }
}

/// How structured records are addressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecordProfile {
    /// Profile epoch.
    pub epoch: u16,
    /// CID codec of record addresses.
    pub codec: u64,
    /// Multihash code of record addresses.
    pub multihash: u64,
}

/// The epoch-0 record profile.
pub const RECORD_PROFILE_0: RecordProfile = RecordProfile {
    epoch: 0,
    codec: RECORD_CODEC,
    multihash: SHA2_256,
};

/// Why a profile cannot be acted on.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProfileError {
    /// An epoch this crate does not know: hold the object, do not reinterpret it.
    #[error("unknown byte profile epoch {0}")]
    UnknownEpoch(u16),
    /// The address codec/multihash do not match the declared epoch.
    #[error("profile epoch declares codec {codec:#x} / multihash {multihash:#x}")]
    AddressTriple {
        /// Declared codec.
        codec: u64,
        /// Declared multihash.
        multihash: u64,
    },
    /// Chunker parameters outside the specification's ranges.
    #[error("chunker parameters out of range: {0:?}")]
    ChunkerParameters(Chunker),
    /// Erasure parameters that build no code.
    #[error("erasure parameters out of range: {0:?}")]
    ErasureParameters(Erasure),
}

/// The fastcdc v2020 tables the `fastcdc-2020` chunker is defined over, as bytes:
/// `GEAR ‖ GEAR_LS ‖ MASKS`, each entry 8 little-endian bytes.
pub fn fastcdc_2020_table_bytes() -> Vec<u8> {
    let (gear, gear_ls) = fastcdc::v2020::get_gear_with_seed(0);
    gear.iter()
        .chain(gear_ls.iter())
        .chain(fastcdc::v2020::MASKS.iter())
        .flat_map(|v| v.to_le_bytes())
        .collect()
}

/// BLAKE3-256 (hex) of [`fastcdc_2020_table_bytes`]; must equal [`FASTCDC_2020_TABLE_DIGEST`].
pub fn fastcdc_2020_table_digest() -> String {
    Code::Blake3_256
        .digest(&fastcdc_2020_table_bytes())
        .digest()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Cut `bytes` into chunks under `profile`'s chunker: `(offset, length)` pairs in order,
/// covering `bytes` exactly. Empty input yields no chunks.
///
/// # Panics
///
/// When the profile's chunker parameters are outside the admitted ranges. A profile read from
/// the wire goes through [`try_chunk_bytes`] (or [`ByteProfile::validate`]) first.
pub fn chunk_bytes(profile: &ByteProfile, bytes: &[u8]) -> Vec<(usize, usize)> {
    try_chunk_bytes(profile, bytes).expect("chunk_bytes requires admitted chunker parameters")
}

/// [`chunk_bytes`], refusing chunker parameters outside the admitted ranges.
pub fn try_chunk_bytes(
    profile: &ByteProfile,
    bytes: &[u8],
) -> Result<Vec<(usize, usize)>, ProfileError> {
    profile.chunker.validate()?;
    let Chunker::Fastcdc2020 {
        min,
        avg,
        max,
        level,
    } = profile.chunker;
    Ok(FastCDC::with_level(
        bytes,
        min as usize,
        avg as usize,
        max as usize,
        Chunker::normalization(level),
    )
    .map(|c| (c.offset, c.length))
    .collect())
}

/// One chunk cut from a stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamChunk {
    /// Offset of the chunk in the stream.
    pub offset: u64,
    /// The chunk's bytes.
    pub data: Vec<u8>,
}

/// Chunks of a stream, cut under a byte profile; yields the same boundaries as
/// [`chunk_bytes`] over the whole input, whatever the reader's read sizes.
pub struct ChunkStream<R: Read> {
    inner: StreamCDC<R>,
}

impl<R: Read> Iterator for ChunkStream<R> {
    type Item = std::io::Result<StreamChunk>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|r| {
            r.map(|c| StreamChunk {
                offset: c.offset,
                data: c.data,
            })
            .map_err(std::io::Error::other)
        })
    }
}

/// Cut a reader's bytes into chunks under `profile`'s chunker, holding at most one maximum
/// chunk in memory.
pub fn chunk_stream<R: Read>(
    profile: &ByteProfile,
    reader: R,
) -> Result<ChunkStream<R>, ProfileError> {
    profile.chunker.validate()?;
    let Chunker::Fastcdc2020 {
        min,
        avg,
        max,
        level,
    } = profile.chunker;
    Ok(ChunkStream {
        inner: StreamCDC::with_level(
            reader,
            min as usize,
            avg as usize,
            max as usize,
            Chunker::normalization(level),
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunker_tables_match_the_declared_digest() {
        assert_eq!(fastcdc_2020_table_digest(), FASTCDC_2020_TABLE_DIGEST);
        assert_eq!(fastcdc_2020_table_bytes().len(), (256 + 256 + 26) * 8);
    }

    #[test]
    fn epoch_zero_profiles_are_admitted() {
        BYTE_PROFILE_0.validate().unwrap();
        assert_eq!(BYTE_PROFILE_0.chunker.id(), "fastcdc-2020");
        assert_eq!(BYTE_PROFILE_0.erasure.id(), "rs-gf8-0x11d-v1");
        assert_eq!(Erasure::None.id(), "none");
        assert_eq!(RECORD_PROFILE_0.codec, 0x71);
        assert_eq!(RECORD_PROFILE_0.multihash, 0x12);
    }

    #[test]
    fn profile_wire_ids_are_the_declared_strings() {
        let json = serde_json::to_value(BYTE_PROFILE_0).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "epoch": 0, "codec": 0x55, "multihash": 0x1e,
                "chunker": {"id": "fastcdc-2020", "min": 262144, "avg": 1048576, "max": 4194304, "level": 1},
                "erasure": {"id": "rs-gf8-0x11d-v1", "data": 4, "parity": 3}
            })
        );
        let none: Erasure = serde_json::from_value(serde_json::json!({"id": "none"})).unwrap();
        assert_eq!(none, Erasure::None);
        // An unknown id is refused, never defaulted.
        assert!(serde_json::from_value::<Chunker>(
            serde_json::json!({"id": "fastcdc-2016", "min": 1, "avg": 2, "max": 3, "level": 1})
        )
        .is_err());
        assert!(serde_json::from_value::<Erasure>(serde_json::json!({"id": "rs-gf8"})).is_err());
    }

    #[test]
    fn out_of_range_parameters_are_refused_not_chunked() {
        let mut odd = BYTE_PROFILE_0;
        odd.chunker = Chunker::Fastcdc2020 {
            min: 262_145,
            avg: 1_048_576,
            max: 4_194_304,
            level: 1,
        };
        assert!(try_chunk_bytes(&odd, b"x").is_err());
        let mut level = BYTE_PROFILE_0;
        level.chunker = Chunker::Fastcdc2020 {
            min: 262_144,
            avg: 1_048_576,
            max: 4_194_304,
            level: 4,
        };
        assert!(level.validate().is_err());
        let mut epoch = BYTE_PROFILE_0;
        epoch.epoch = 1;
        assert_eq!(epoch.validate(), Err(ProfileError::UnknownEpoch(1)));
        let mut rs = BYTE_PROFILE_0;
        rs.erasure = Erasure::RsGf8x11dV1 {
            data: 200,
            parity: 100,
        };
        assert!(rs.validate().is_err());
    }

    #[test]
    fn empty_input_is_zero_chunks() {
        assert!(chunk_bytes(&BYTE_PROFILE_0, b"").is_empty());
        assert_eq!(chunk_stream(&BYTE_PROFILE_0, &b""[..]).unwrap().count(), 0);
    }
}
