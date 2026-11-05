//! # `elohim-address` — the one Rust addressing kernel
//!
//! Every Elohim address is a self-describing CIDv1: the codec and the multihash code inside
//! the address say how it was made, so a reader never guesses. Three profiles exist at epoch 0,
//! one per plane, and this crate is the only Rust code that mints or checks them:
//!
//! | Plane | Codec | Multihash | Renders as | Constructor |
//! |---|---|---|---|---|
//! | **bytes** (content, bundles, artifacts, chunks, shards) | raw `0x55` | BLAKE3-256 `0x1e` | `bafkr4i…` (59 chars) | [`compute_bytes_cid`] |
//! | **records** (canonical dag-cbor atoms) | dag-cbor `0x71` | sha2-256 `0x12` | `bafyrei…` | [`compute_record_cid`] |
//! | **citations** (cite fingerprints over canonical bodies) | raw `0x55` | sha2-256 `0x12` | `bafkrei…` | [`compute_raw_sha2_cid`] |
//!
//! Holochain entry hashes are a fourth plane and are not touched here.
//!
//! ## The byte surface admits by full decode, never by prefix
//!
//! A byte address arriving as text is admitted only by [`ByteCid::parse`], which decodes the
//! whole 36-byte canonical form and requires the header `01 55 1e 20`, no non-zero padding
//! bits, and canonical re-encoding equal to the input. A prefix-and-length check would admit
//! `01 55 1e 21` (a declared 33-byte digest carrying 32) and non-canonical padding; both are in
//! the negative vector set (`vectors/address-vectors.json`). [`AddressError::check`] names the
//! check that failed.
//!
//! ## Verification fails closed
//!
//! [`verify_cid_any`] recomputes a digest only for the three `(codec, multihash, length)` pairs
//! above. Anything else is [`Verify::UnsupportedProfile`] — an address this kernel cannot check
//! is never a passing one. Verification proves the bytes match; it never grants authorization.
//!
//! ## Boundary
//!
//! The crate links into the DNA zomes and browser builds (`wasm32-unknown-unknown`). Its
//! dependencies are `cid`, `multihash-codetable` (sha2 + blake3), `data-encoding` and
//! `thiserror`, with `serde` behind a feature; `tests/boundary.rs` reads the lockfile and fails
//! if an async runtime, a TypeScript generator or a JSON stack arrives.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::{fmt, str::FromStr, sync::OnceLock};

use cid::{Cid, Version};
use data_encoding::{Encoding, Specification};
use multihash_codetable::{Code, MultihashDigest};

/// Codec of the byte profile: raw (`0x55`).
pub const BYTE_CODEC: u64 = 0x55;
/// Multihash code of the byte profile: BLAKE3-256 (`0x1e`), unkeyed.
pub const BYTE_MULTIHASH: u64 = 0x1e;
/// Digest length of the byte profile, in bytes.
pub const BYTE_DIGEST_LEN: usize = 32;
/// Every byte CID's text starts with this; the header alone determines it.
pub const BYTE_CID_PREFIX: &str = "bafkr4i";
/// Every byte CID's text is exactly this long (`b` + 58 base32 characters for 36 bytes).
pub const BYTE_CID_LEN: usize = 59;
/// Byte length of a decoded byte CID: four one-byte header varints and the digest.
pub const BYTE_CID_BYTES: usize = 36;

/// Codec of the record profile: dag-cbor (`0x71`).
pub const RECORD_CODEC: u64 = 0x71;
/// Multihash code of the record and citation profiles: sha2-256 (`0x12`).
pub const SHA2_256: u64 = 0x12;
/// Text prefix of a record CID (dag-cbor + sha2-256).
pub const RECORD_CID_PREFIX: &str = "bafyrei";
/// Codec of the citation profile: raw (`0x55`), shared with the byte profile; the multihash
/// tells them apart.
pub const CITE_CODEC: u64 = 0x55;
/// Text prefix of a citation CID (raw + sha2-256). Never admitted on the byte surface.
pub const CITE_CID_PREFIX: &str = "bafkrei";

/// The header every byte CID decodes to: version 1, raw, BLAKE3-256, 32-byte digest.
const BYTE_HEADER: [u8; 4] = [0x01, 0x55, 0x1e, 0x20];

/// Why a text or CID is not an admissible byte address, or why a fingerprint cannot be taken.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AddressError {
    /// The text is not exactly [`BYTE_CID_LEN`] bytes long.
    #[error("byte CID must be {BYTE_CID_LEN} characters, got {len}")]
    Length {
        /// The length received.
        len: usize,
    },
    /// The multibase prefix is not `b` (base32 lower, no padding).
    #[error("byte CID must use multibase 'b' (base32 lower)")]
    Multibase,
    /// A character outside `a-z2-7` (upper case included).
    #[error("byte CID holds a character outside base32 lower")]
    Alphabet,
    /// The final character carries non-zero padding bits: a second spelling of the same bytes.
    #[error("byte CID has non-zero padding bits")]
    Padding,
    /// The base32 body did not decode.
    #[error("byte CID body is not base32")]
    Base32,
    /// The body decoded to a length other than [`BYTE_CID_BYTES`].
    #[error("byte CID must decode to {BYTE_CID_BYTES} bytes, got {len}")]
    DecodedLength {
        /// The decoded length.
        len: usize,
    },
    /// The CID version is not 1.
    #[error("byte CID version must be 1, got {version}")]
    Version {
        /// The version declared.
        version: u64,
    },
    /// The codec is not raw (`0x55`).
    #[error("byte CID codec must be 0x55, got {codec:#x}")]
    Codec {
        /// The codec declared.
        codec: u64,
    },
    /// The multihash is not BLAKE3-256 (`0x1e`).
    #[error("byte CID multihash must be 0x1e, got {multihash:#x}")]
    Multihash {
        /// The multihash code declared.
        multihash: u64,
    },
    /// A header varint carries a redundant trailing zero group (e.g. version `81 00`): a second
    /// spelling of the same field, refused before any decoder sees it.
    #[error("byte CID header varint is not minimal")]
    NonMinimalVarint,
    /// The declared digest length is not 32.
    #[error("byte CID digest must be 32 bytes, header declares {len}")]
    DigestLength {
        /// The digest length declared.
        len: u64,
    },
    /// The bytes do not re-encode to the input text.
    #[error("byte CID is not in canonical form")]
    NonCanonical,
    /// A short fingerprint was asked of a CID whose digest is not sha2-256.
    #[error("short fingerprints are sha2-256 only, got multihash {multihash:#x}")]
    NotSha2 {
        /// The multihash code of the CID.
        multihash: u64,
    },
}

impl AddressError {
    /// The stable name of the check that failed, as the conformance vectors spell it.
    pub fn check(&self) -> &'static str {
        match self {
            Self::Length { .. } => "length",
            Self::Multibase => "multibase",
            Self::Alphabet => "alphabet",
            Self::Padding => "padding",
            Self::Base32 => "base32",
            Self::DecodedLength { .. } => "decoded-length",
            Self::Version { .. } => "version",
            Self::Codec { .. } => "codec",
            Self::Multihash { .. } => "multihash",
            Self::NonMinimalVarint => "non-minimal-varint",
            Self::DigestLength { .. } => "digest-length",
            Self::NonCanonical => "non-canonical",
            Self::NotSha2 { .. } => "not-sha2",
        }
    }
}

/// RFC 4648 base32, lower-case alphabet, no padding, trailing bits required to be zero.
fn base32_lower() -> &'static Encoding {
    static ENCODING: OnceLock<Encoding> = OnceLock::new();
    ENCODING.get_or_init(|| {
        let mut spec = Specification::new();
        spec.symbols.push_str("abcdefghijklmnopqrstuvwxyz234567");
        spec.check_trailing_bits = true;
        spec.encoding()
            .expect("the base32-lower specification is a valid 32-symbol alphabet")
    })
}

/// One unsigned LEB128 varint from the front of `bytes`, requiring the minimal encoding:
/// (value, bytes read). A varint that runs past nine bytes or past the input is
/// [`AddressError::NonCanonical`].
fn minimal_varint(bytes: &[u8]) -> Result<(u64, usize), AddressError> {
    let mut value = 0u64;
    for (i, b) in bytes.iter().enumerate().take(9) {
        value |= u64::from(b & 0x7f) << (7 * i);
        if b & 0x80 == 0 {
            if i > 0 && *b == 0 {
                return Err(AddressError::NonMinimalVarint);
            }
            return Ok((value, i + 1));
        }
    }
    Err(AddressError::NonCanonical)
}

/// Check a decoded header field by field, naming the first that differs from the byte profile.
/// Every field must be a minimal varint, so a header that passes is exactly [`BYTE_HEADER`].
fn check_header(bytes: &[u8]) -> Result<(), AddressError> {
    let mut at = 0;
    let mut next = || -> Result<u64, AddressError> {
        let (v, n) = minimal_varint(bytes.get(at..).unwrap_or_default())?;
        at += n;
        Ok(v)
    };
    let version = next()?;
    if version != 1 {
        return Err(AddressError::Version { version });
    }
    let codec = next()?;
    if codec != BYTE_CODEC {
        return Err(AddressError::Codec { codec });
    }
    let multihash = next()?;
    if multihash != BYTE_MULTIHASH {
        return Err(AddressError::Multihash { multihash });
    }
    let len = next()?;
    if len != BYTE_DIGEST_LEN as u64 {
        return Err(AddressError::DigestLength { len });
    }
    if bytes.get(..BYTE_HEADER.len()) != Some(BYTE_HEADER.as_slice()) {
        return Err(AddressError::NonCanonical);
    }
    Ok(())
}

/// An address on the byte surface: CIDv1 raw (`0x55`) + BLAKE3-256 (`0x1e`), 32-byte digest.
///
/// The only ways to hold one are hashing bytes ([`compute_bytes_cid`]), full-decode parsing
/// ([`ByteCid::parse`]), checking a [`Cid`] (`TryFrom<Cid>`), or wrapping a BLAKE3 digest
/// ([`ByteCid::from_digest`]). There is no constructor from unchecked text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ByteCid(Cid);

impl ByteCid {
    /// Admit a byte address from text by full decode: length 59, multibase `b`, base32-lower
    /// alphabet, zero padding bits, exactly 36 decoded bytes, header `01 55 1e 20` in minimal
    /// varints, and canonical re-encoding equal to the input. The error names the first failed
    /// check. Never panics, in any build mode, on any input.
    pub fn parse(text: &str) -> Result<ByteCid, AddressError> {
        if text.len() != BYTE_CID_LEN {
            return Err(AddressError::Length { len: text.len() });
        }
        let body = text.strip_prefix('b').ok_or(AddressError::Multibase)?;
        if !body.bytes().all(|b| matches!(b, b'a'..=b'z' | b'2'..=b'7')) {
            return Err(AddressError::Alphabet);
        }
        let bytes = base32_lower()
            .decode(body.as_bytes())
            .map_err(|e| match e.kind {
                data_encoding::DecodeKind::Trailing => AddressError::Padding,
                _ => AddressError::Base32,
            })?;
        if bytes.len() != BYTE_CID_BYTES {
            return Err(AddressError::DecodedLength { len: bytes.len() });
        }
        check_header(&bytes)?;
        let cid = Cid::try_from(bytes.as_slice()).map_err(|_| AddressError::NonCanonical)?;
        if cid.to_string() != text {
            return Err(AddressError::NonCanonical);
        }
        Ok(ByteCid(cid))
    }

    /// Wrap a BLAKE3-256 digest the caller computed (for instance incrementally over a stream).
    pub fn from_digest(digest: [u8; 32]) -> ByteCid {
        let mh = cid::multihash::Multihash::<64>::wrap(BYTE_MULTIHASH, &digest)
            .expect("a 32-byte digest fits a 64-byte multihash");
        ByteCid(Cid::new_v1(BYTE_CODEC, mh))
    }

    /// The underlying CID.
    pub fn as_cid(&self) -> &Cid {
        &self.0
    }

    /// The 32-byte BLAKE3 digest (also the iroh-blobs key for this payload).
    pub fn digest(&self) -> &[u8; 32] {
        self.0
            .hash()
            .digest()
            .try_into()
            .expect("a ByteCid always carries a 32-byte digest")
    }

    /// The canonical 36-byte binary form.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }
}

impl fmt::Display for ByteCid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl FromStr for ByteCid {
    type Err = AddressError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        ByteCid::parse(text)
    }
}

impl TryFrom<Cid> for ByteCid {
    type Error = AddressError;

    /// Admit a CID that already decoded, checking it carries the byte profile.
    fn try_from(cid: Cid) -> Result<Self, Self::Error> {
        if cid.version() != Version::V1 {
            return Err(AddressError::Version { version: 0 });
        }
        if cid.codec() != BYTE_CODEC {
            return Err(AddressError::Codec { codec: cid.codec() });
        }
        if cid.hash().code() != BYTE_MULTIHASH {
            return Err(AddressError::Multihash {
                multihash: cid.hash().code(),
            });
        }
        if usize::from(cid.hash().size()) != BYTE_DIGEST_LEN {
            return Err(AddressError::DigestLength {
                len: u64::from(cid.hash().size()),
            });
        }
        Ok(ByteCid(cid))
    }
}

impl From<ByteCid> for Cid {
    fn from(value: ByteCid) -> Self {
        value.0
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for ByteCid {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for ByteCid {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Text;
        impl serde::de::Visitor<'_> for Text {
            type Value = ByteCid;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a byte CID (bafkr4i…, 59 characters)")
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<ByteCid, E> {
                ByteCid::parse(v).map_err(E::custom)
            }
        }
        deserializer.deserialize_str(Text)
    }
}

/// Address bytes under the epoch-0 byte profile: CIDv1 raw + BLAKE3-256 (`bafkr4i…`).
pub fn compute_bytes_cid(bytes: &[u8]) -> ByteCid {
    ByteCid(Cid::new_v1(BYTE_CODEC, Code::Blake3_256.digest(bytes)))
}

/// Address a canonical body under the citation profile: CIDv1 raw + sha2-256 (`bafkrei…`).
///
/// Citations only: the cite fingerprint `sha256:<16 hex>` is the [`short_fingerprint`] of this
/// CID over the canonical body, and it is unchanged by the byte-profile crossing. Never use it
/// to address payload bytes.
pub fn compute_raw_sha2_cid(bytes: &[u8]) -> Cid {
    Cid::new_v1(CITE_CODEC, Code::Sha2_256.digest(bytes))
}

/// Address canonical dag-cbor bytes under the record profile: CIDv1 dag-cbor + sha2-256
/// (`bafyrei…`). The bytes must already be canonical dag-cbor; this does not encode.
pub fn compute_record_cid(canonical_dag_cbor: &[u8]) -> Cid {
    Cid::new_v1(RECORD_CODEC, Code::Sha2_256.digest(canonical_dag_cbor))
}

/// The outcome of checking a claimed CID against bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verify {
    /// The CID's own profile recomputes to the claimed digest.
    Verified,
    /// The profile is known and the digest differs.
    Mismatch,
    /// The `(codec, multihash, digest length)` triple is not a profile this kernel checks;
    /// the claim is neither verified nor refuted, and must not be served as verified.
    UnsupportedProfile {
        /// The codec the CID declares.
        codec: u64,
        /// The multihash code the CID declares.
        multihash: u64,
        /// The digest length the CID carries.
        len: usize,
    },
}

/// Check `claimed` against `bytes` under the profile `claimed` itself declares, dispatching on
/// `(codec, multihash code, digest length)`: `(0x55, 0x1e, 32)` bytes, `(0x55, 0x12, 32)`
/// citations, `(0x71, 0x12, 32)` records. Every other triple, and every CIDv0, fails closed as
/// [`Verify::UnsupportedProfile`].
pub fn verify_cid_any(claimed: &Cid, bytes: &[u8]) -> Verify {
    let codec = claimed.codec();
    let multihash = claimed.hash().code();
    let len = usize::from(claimed.hash().size());
    let unsupported = Verify::UnsupportedProfile {
        codec,
        multihash,
        len,
    };
    if claimed.version() != Version::V1 {
        return unsupported;
    }
    let recomputed = match (codec, multihash, len) {
        (BYTE_CODEC, BYTE_MULTIHASH, BYTE_DIGEST_LEN) => Code::Blake3_256.digest(bytes),
        (CITE_CODEC, SHA2_256, 32) | (RECORD_CODEC, SHA2_256, 32) => Code::Sha2_256.digest(bytes),
        _ => return unsupported,
    };
    if recomputed.digest() == claimed.hash().digest() {
        Verify::Verified
    } else {
        Verify::Mismatch
    }
}

/// The cite short form `"sha256:" + first 16 hex of the digest`, for sha2-256 CIDs only.
///
/// The same first-16-hex the cite oracle and brit's `drift_fingerprint` emit. A BLAKE3 (or any
/// other) digest is refused: a `sha256:` label over a different algorithm would be a lie.
pub fn short_fingerprint(cid: &Cid) -> Result<String, AddressError> {
    let mh = cid.hash();
    if mh.code() != SHA2_256 || usize::from(mh.size()) != 32 {
        return Err(AddressError::NotSha2 {
            multihash: mh.code(),
        });
    }
    let mut out = String::with_capacity("sha256:".len() + 16);
    out.push_str("sha256:");
    for b in &mh.digest()[..8] {
        out.push_str(&format!("{b:02x}"));
    }
    Ok(out)
}
