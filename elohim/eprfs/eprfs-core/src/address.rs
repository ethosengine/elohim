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
/// Codec byte for raw bytes per the IPLD multicodec table — for file/body/arbitrary bytes that
/// are NOT already canonical dag-cbor atoms.
const RAW_CODEC: u64 = 0x55;

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
    /// Content-address **already-canonical dag-cbor** bytes: `CIDv1(dag-cbor 0x71, sha2-256)`
    /// (`bafyrei…`). Matches `elohim_epr::cid::compute_cid`. This labels the input as dag-cbor —
    /// only use it for CBOR-encoded atoms. Arbitrary file/body/blob bytes are NOT dag-cbor and must
    /// use [`BlobCid::compute_raw`] (codec 0x55) so the codec tag tells the truth about the bytes.
    pub fn compute(bytes: &[u8]) -> Self {
        Self(Cid::new_v1(DAG_CBOR_CODEC, Code::Sha2_256.digest(bytes)))
    }

    /// Content-address **raw** bytes: `CIDv1(raw 0x55, sha2-256)` (`bafkrei…`). The codec for
    /// arbitrary file bytes, blob contents, and canonical document bodies — anything that is not a
    /// canonical dag-cbor atom. Byte-identical to brit's `BritCid::compute_raw` (same golden
    /// vectors); the `raw_codec_matches_brit_vector` test pins the shared format.
    ///
    /// The cite fingerprint `sha256:hex16` is, mathematically, the [`short_fingerprint`] of the
    /// `compute_raw` CID over the same canonical-body bytes — one sha2-256 digest, two renderings
    /// (see `2026-07-12-cite-fingerprint-cid-convergence-design.md`).
    ///
    /// [`short_fingerprint`]: BlobCid::short_fingerprint
    pub fn compute_raw(bytes: &[u8]) -> Self {
        Self(Cid::new_v1(RAW_CODEC, Code::Sha2_256.digest(bytes)))
    }

    /// The declared cite **short-form**: `"sha256:" + hex(multihash digest)[:16]`. This is the same
    /// first-16-hex-of-sha2-256 that the Python cite oracle (`cite_graph.fingerprint_text`) and
    /// brit's `drift_fingerprint` emit — a short projection of THIS CID's digest. For a
    /// `compute_raw` CID over a canonical body, it equals the doc's cite fingerprint exactly.
    pub fn short_fingerprint(&self) -> String {
        // First 8 digest bytes render to the 16 hex chars of the short form (no `hex` dep needed).
        let mut s = String::with_capacity("sha256:".len() + 16);
        s.push_str("sha256:");
        for b in self.0.hash().digest().iter().take(8) {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }

    /// Parse a canonical CID string (base32 `bafy…` / `bafk…`). Errors on a non-CID string — the
    /// boundary guard against a stringly-typed fingerprint re-entering.
    pub fn parse(s: &str) -> Result<Self, cid::Error> {
        Ok(Self(s.parse::<Cid>()?))
    }

    /// True iff this fingerprint is the content hash of `bytes` (tamper detection) — the property a
    /// String newtype could never provide.
    ///
    /// Recomputation dispatches on the codec THIS CID declares, because the codec tag is a claim
    /// about what the bytes are: a `compute_raw` fingerprint (0x55, the codec of every document
    /// body and file blob) and a `compute` fingerprint (0x71, canonical dag-cbor atoms) over the
    /// same bytes are different CIDs, and each must verify only against its own construction.
    /// An unrecognized codec fails closed — a fingerprint we cannot recompute is an unverified
    /// claim, never a passing one.
    pub fn verifies(&self, bytes: &[u8]) -> bool {
        match self.0.codec() {
            RAW_CODEC => Self::compute_raw(bytes).0 == self.0,
            DAG_CBOR_CODEC => Self::compute(bytes).0 == self.0,
            _ => false,
        }
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

    // Raw-codec golden vector: byte-identical to brit's `BritCid::compute_raw`. "hello world" is
    // the canonical IPFS raw-CID fixture; if this drifts, eprfs raw fingerprints stop matching brit
    // + the protocol content-addressing.
    #[test]
    fn raw_codec_matches_brit_vector() {
        let cid = BlobCid::compute_raw(b"hello world");
        assert_eq!(
            cid.to_string(),
            "bafkreifzjut3te2nhyekklss27nh3k72ysco7y32koao5eei66wof36n5e",
            "got {cid}"
        );
        assert!(
            cid.to_string().starts_with("bafkrei"),
            "raw codec must render bafkrei…"
        );
    }

    #[test]
    fn compute_raw_and_compute_differ_by_codec() {
        // Same bytes, different codec tag → different CID (dag-cbor bafyrei… vs raw bafkrei…).
        let raw = BlobCid::compute_raw(b"hello world");
        let cbor = BlobCid::compute(b"hello world");
        assert_ne!(raw, cbor);
        assert!(raw.to_string().starts_with("bafkrei"));
        assert!(cbor.to_string().starts_with("bafyrei"));
        // But the underlying sha2-256 digest — hence the short form — is identical.
        assert_eq!(raw.short_fingerprint(), cbor.short_fingerprint());
    }

    // The convergence pin: the cite fingerprint `sha256:hex16` IS the short-form of the raw-codec
    // CID over the same canonical-body bytes. Expected value hard-coded from the Python oracle
    // (`cite_graph.fingerprint_text("ok target body")`).
    #[test]
    fn short_fingerprint_equals_python_cite_fingerprint() {
        let body = b"ok target body";
        let cid = BlobCid::compute_raw(body);
        assert_eq!(cid.short_fingerprint(), "sha256:4e72cded3d6affd3");
    }

    #[test]
    fn short_fingerprint_shape() {
        let f = BlobCid::compute_raw(b"anything").short_fingerprint();
        assert!(f.starts_with("sha256:"));
        assert_eq!(f.len(), "sha256:".len() + 16);
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

    // `verifies` must check against the codec THIS CID declares, not a hardcoded one. Every
    // production caller content-addresses document bodies and file bytes with `compute_raw`
    // (0x55) — `eprfs-cli/src/main.rs`, `epr-cli/src/flow/{mod,edges}.rs` — so a `verifies` that
    // always recomputed dag-cbor (0x71) returned false for every real fingerprint against its own
    // bytes. `verifies_detects_tampering` cannot catch that: it computes both sides with
    // `compute`, so it passes either way.
    #[test]
    fn verifies_dispatches_on_declared_codec() {
        let raw = BlobCid::compute_raw(b"hello world");
        assert!(
            raw.verifies(b"hello world"),
            "raw CID must verify raw bytes"
        );
        assert!(!raw.verifies(b"hello w0rld"));

        let cbor = BlobCid::compute(b"hello world");
        assert!(cbor.verifies(b"hello world"), "dag-cbor CID must verify");
        assert!(!cbor.verifies(b"hello w0rld"));

        // Same bytes, different codec tag — neither may verify as the other.
        assert_ne!(raw, cbor);
    }

    // An unrecognized codec must fail closed. Verification is the tamper-detection primitive; a
    // codec we cannot recompute is an unverified claim, never a passing one.
    #[test]
    fn verifies_fails_closed_on_unknown_codec() {
        // 0x70 = dag-pb, a valid multicodec this type never mints.
        let dag_pb = BlobCid(Cid::new_v1(0x70, Code::Sha2_256.digest(b"hello world")));
        assert!(!dag_pb.verifies(b"hello world"));
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
