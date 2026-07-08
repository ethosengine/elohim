use std::fmt;

use cid::Cid;
use multihash_codetable::{Code, MultihashDigest};
use serde::{Deserialize, Serialize};

/// Protocol-level reference to an EPR record — the permanent, human-readable **identity** (a slug,
/// e.g. `epr:content-name`). Identity is the slug; the **fingerprint** is a [`BlobCid`].
///
/// Deliberately NOT content-addressed: an `EprRef` survives edits and moves, exactly the
/// identity/fingerprint split the substrate makes
/// (`genesis/docs/superpowers/specs/2026-06-02-semantic-computable-links-design.md`). CID-ifying
/// this would break the slug-is-identity model.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EprRef(String);

impl EprRef {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for EprRef {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for EprRef {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Codec byte for dag-cbor per the IPLD multicodec table — the canonical EPR CID codec.
const DAG_CBOR_CODEC: u64 = 0x71;

/// Content-addressed blob identifier — a real `CIDv1(dag-cbor, sha2-256)`: the protocol's canonical
/// **fingerprint**. Byte-identical to the canonical codec `elohim-epr` (`elohim/epr/src/cid.rs`),
/// which is the single source of the format; this consumes the same base crates (`cid`,
/// `multihash-codetable`) rather than the heavy atom codec, keeping `eprfs-core` lean. The
/// `cid_matches_canonical_format` vector test pins the shared format.
///
/// A `BlobCid` can ONLY be constructed by hashing bytes ([`BlobCid::compute`]) or parsing a valid
/// CID string ([`BlobCid::parse`]). There is deliberately no `From<&str>`/`new` that would let an
/// arbitrary non-CID string masquerade as a fingerprint — that stringly-typed placeholder is the
/// drift this type replaces. Wire form is the canonical base32 CID string, validated on deserialize.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct BlobCid(Cid);

impl BlobCid {
    /// Content-address bytes: `CIDv1(dag-cbor, sha2-256)`. Matches `elohim_epr::cid::compute_cid`.
    pub fn compute(bytes: &[u8]) -> Self {
        Self(Cid::new_v1(DAG_CBOR_CODEC, Code::Sha2_256.digest(bytes)))
    }

    /// Parse a canonical CID string (base32 `bafy…` / `bafk…`). Errors on a non-CID string — the
    /// boundary guard against a stringly-typed fingerprint re-entering.
    pub fn parse(s: &str) -> Result<Self, cid::Error> {
        Ok(Self(s.parse::<Cid>()?))
    }

    /// True iff this fingerprint is the content hash of `bytes` (tamper detection) — the property a
    /// String newtype could never provide.
    pub fn verifies(&self, bytes: &[u8]) -> bool {
        Self::compute(bytes).0 == self.0
    }

    /// The underlying canonical CID.
    pub fn as_cid(&self) -> &Cid {
        &self.0
    }
}

impl fmt::Display for BlobCid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<BlobCid> for String {
    fn from(value: BlobCid) -> Self {
        value.0.to_string()
    }
}

impl TryFrom<String> for BlobCid {
    type Error = cid::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

/// Stable identifier for a local projection of an EPR-backed tree — a local **name** (slug), not a
/// content hash. Stays a string for the same reason [`EprRef`] does.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectionId(String);

impl ProjectionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The canonical-format pin (mirrors elohim/epr/tests/cid_vectors.rs). If this drifts, eprfs
    // fingerprints stop matching the protocol's content-addressing — the whole reason to use a CID.
    #[test]
    fn cid_matches_canonical_format() {
        // Empty CBOR map (0xa0): CIDv1 base32 + dag-cbor + sha2-256 begins with "bafyrei".
        let cid = BlobCid::compute(&[0xa0]);
        assert!(cid.to_string().starts_with("bafyrei"), "got {cid}");
    }

    #[test]
    fn compute_is_stable_and_content_sensitive() {
        assert_eq!(BlobCid::compute(b"abc"), BlobCid::compute(b"abc"));
        assert_ne!(BlobCid::compute(b"abc"), BlobCid::compute(b"abd"));
    }

    #[test]
    fn verifies_detects_tampering() {
        let cid = BlobCid::compute(b"hello");
        assert!(cid.verifies(b"hello"));
        assert!(!cid.verifies(b"hell0"));
    }

    #[test]
    fn parse_round_trips_and_rejects_non_cid() {
        let cid = BlobCid::compute(b"round");
        assert_eq!(BlobCid::parse(&cid.to_string()).unwrap(), cid);
        assert!(BlobCid::parse("not-a-cid").is_err());
    }

    #[test]
    fn serde_wire_is_the_canonical_cid_string() {
        let cid = BlobCid::compute(b"wire");
        let json = serde_json::to_string(&cid).unwrap();
        assert_eq!(json, format!("\"{cid}\""));
        assert_eq!(serde_json::from_str::<BlobCid>(&json).unwrap(), cid);
        // a non-CID string is rejected at the wire boundary
        assert!(serde_json::from_str::<BlobCid>("\"bafk-test\"").is_err());
    }
}
