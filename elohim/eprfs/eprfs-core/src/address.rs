use std::fmt;

use cid::Cid;
pub use elohim_address::{AddressError, ByteCid};
use elohim_address::{
    Verify, BYTE_CODEC, BYTE_DIGEST_LEN, BYTE_MULTIHASH, CITE_CODEC, RECORD_CODEC, SHA2_256,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

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

/// Content-addressed blob identifier: a parsed CIDv1 under one of the protocol's declared address
/// profiles, the canonical **fingerprint**. eprfs mints no address of its own: every constructor
/// below delegates to the one Rust addressing kernel ([`elohim_address`], re-genesis 1 amended R1),
/// so a profile is declared once and every consumer agrees by construction.
///
/// | Constructor | Profile | Renders | Use |
/// |---|---|---|---|
/// | [`BlobCid::compute_bytes`] | raw `0x55` + BLAKE3-256 `0x1e` (the byte profile, epoch 0) | `bafkr4i…` | newly produced payload bytes and chunks |
/// | [`BlobCid::compute`] | dag-cbor `0x71` + sha2-256 (the record profile) | `bafyrei…` | canonical DAG-CBOR records |
/// | [`BlobCid::compute_raw`] | raw `0x55` + sha2-256 (the citation profile) | `bafkrei…` | citation fingerprints; its bytes never change |
///
/// A `BlobCid` can ONLY be constructed by hashing bytes or parsing a valid CID string
/// ([`BlobCid::parse`]). There is deliberately no `From<&str>`/`new` that would let an arbitrary
/// non-CID string masquerade as a fingerprint. Wire form is the canonical base32 CID string,
/// validated on deserialize. A CID names its own multihash, so objects addressed under an older
/// profile stay verifiable under it ([`BlobCid::verifies`] dispatches on the declared profile).
///
/// `PartialOrd`/`Ord` follow `cid::Cid`'s own order (interfaces spec §0 finding 3): keys such as
/// `DerivationKey` and the `BTreeMap`s keyed by CID need a total order, and the order carries no
/// meaning beyond being stable.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct BlobCid(Cid);

impl BlobCid {
    /// Content-address **payload bytes** under the byte profile: `CIDv1(raw 0x55, BLAKE3-256)`
    /// (`bafkr4i…`, [`elohim_address::compute_bytes_cid`]). Every newly produced native payload
    /// (artifact, chunk, shard) is addressed this way; payload size never selects a hash.
    pub fn compute_bytes(bytes: &[u8]) -> Self {
        Self(elohim_address::compute_bytes_cid(bytes).into())
    }

    /// Content-address **already-canonical dag-cbor** bytes: `CIDv1(dag-cbor 0x71, sha2-256)`
    /// (`bafyrei…`, [`elohim_address::compute_record_cid`]; equal to `elohim_epr::cid::compute_cid`).
    /// This labels the input as dag-cbor — only use it for CBOR-encoded records.
    pub fn compute(bytes: &[u8]) -> Self {
        Self(elohim_address::compute_record_cid(bytes))
    }

    /// Content-address bytes under the **citation profile**: `CIDv1(raw 0x55, sha2-256)`
    /// (`bafkrei…`, [`elohim_address::compute_raw_sha2_cid`]). Its output bytes are frozen: the
    /// cite fingerprint `sha256:hex16` is, mathematically, the [`short_fingerprint`] of this CID
    /// over a document's canonical body (`2026-07-12-cite-fingerprint-cid-convergence-design.md`),
    /// and brit's `BritCid::compute_raw` pins the same golden vector. New payload bytes use
    /// [`BlobCid::compute_bytes`]; this constructor stays for citations and for objects already
    /// addressed under it.
    ///
    /// [`short_fingerprint`]: BlobCid::short_fingerprint
    pub fn compute_raw(bytes: &[u8]) -> Self {
        Self(elohim_address::compute_raw_sha2_cid(bytes))
    }

    /// The declared cite **short-form**: `"sha256:" + hex(multihash digest)[:16]`, the same
    /// first-16-hex-of-sha2-256 the Python cite oracle (`cite_graph.fingerprint_text`) and brit's
    /// `drift_fingerprint` emit. For a [`BlobCid::compute_raw`] CID over a canonical body it equals
    /// the doc's cite fingerprint exactly.
    ///
    /// # Errors
    ///
    /// [`AddressError::NotSha2`] for a CID whose multihash is not sha2-256: a `sha256:` label
    /// over a BLAKE3 digest would be a false fingerprint.
    pub fn short_fingerprint(&self) -> Result<String, AddressError> {
        elohim_address::short_fingerprint(&self.0)
    }

    /// Encode a serializable manifest as canonical DAG-CBOR and address those exact bytes.
    pub fn encode_manifest<T: Serialize>(
        value: &T,
    ) -> Result<(Self, Vec<u8>), serde_ipld_dagcbor::EncodeError<std::collections::TryReserveError>>
    {
        let bytes = serde_ipld_dagcbor::to_vec(value)?;
        Ok((Self::compute(&bytes), bytes))
    }

    /// Parse a canonical CID string (base32 `bafy…` / `bafk…`). Errors on a non-CID string — the
    /// boundary guard against a stringly-typed fingerprint re-entering. This admits any CID; a
    /// byte-payload surface narrows it with [`BlobCid::byte_cid`].
    pub fn parse(s: &str) -> Result<Self, cid::Error> {
        Ok(Self(s.parse::<Cid>()?))
    }

    /// This CID as a byte-profile address, refused unless it is exactly `CIDv1(raw 0x55,
    /// BLAKE3-256, 32-byte digest)`. This checks the profile of an already-parsed CID; TEXT
    /// arriving at a byte surface is admitted by [`ByteCid::parse`] (full decode and canonical
    /// re-encoding), because [`BlobCid::parse`] accepts non-canonical spellings such as an
    /// uppercase multibase.
    ///
    /// # Errors
    ///
    /// The kernel's [`AddressError`], naming the first check that failed (codec, multihash, ...).
    pub fn byte_cid(&self) -> Result<ByteCid, AddressError> {
        ByteCid::try_from(self.0)
    }

    /// True iff this fingerprint is the content hash of `bytes` (tamper detection) — the property a
    /// String newtype could never provide.
    ///
    /// Recomputation dispatches on the profile THIS CID declares — codec, multihash and digest
    /// length together ([`elohim_address::verify_cid_any`]) — because the CID is a claim about how
    /// the bytes were addressed: a sha2 CID never verifies BLAKE3-addressed bytes and the reverse.
    /// An unsupported profile fails closed — a fingerprint we cannot recompute is an unverified
    /// claim, never a passing one.
    pub fn verifies(&self, bytes: &[u8]) -> bool {
        matches!(
            elohim_address::verify_cid_any(&self.0, bytes),
            Verify::Verified
        )
    }

    /// Recompute the fingerprint of bytes under this CID's declared profile. Returns `None` for an
    /// unsupported profile so a caller cannot turn an unknown format into a passing projection.
    pub fn recompute_for_profile(&self, bytes: &[u8]) -> Option<Self> {
        let hash = self.0.hash();
        match (self.0.codec(), hash.code(), usize::from(hash.size())) {
            (BYTE_CODEC, BYTE_MULTIHASH, BYTE_DIGEST_LEN) => Some(Self::compute_bytes(bytes)),
            (CITE_CODEC, SHA2_256, 32) => Some(Self::compute_raw(bytes)),
            (RECORD_CODEC, SHA2_256, 32) => Some(Self::compute(bytes)),
            _ => None,
        }
    }

    /// The underlying canonical CID.
    pub fn as_cid(&self) -> &Cid {
        &self.0
    }
}

impl From<ByteCid> for BlobCid {
    fn from(value: ByteCid) -> Self {
        Self(value.into())
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

/// A CID in a canonical DAG-CBOR link position. This is the same address as
/// [`BlobCid`], not a second identity or a claim that the referenced bytes were
/// fetched or verified. Its wire representation is deliberately different:
/// DAG-CBOR uses IPLD tag 42, while human-readable formats such as JSON use the
/// existing validated CID string. Callers choose this type only for link fields;
/// changing an existing `BlobCid` field to `BlobLink` changes its canonical bytes.
///
/// This codec does not bound an enclosing document or record a decode refusal.
/// Callers must bound input size and witness failures at their own boundary.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlobLink(BlobCid);

impl BlobLink {
    pub fn as_blob_cid(&self) -> &BlobCid {
        &self.0
    }
}

impl From<BlobCid> for BlobLink {
    fn from(cid: BlobCid) -> Self {
        Self(cid)
    }
}

impl From<BlobLink> for BlobCid {
    fn from(link: BlobLink) -> Self {
        link.0
    }
}

impl Serialize for BlobLink {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            self.0.serialize(serializer)
        } else {
            self.0.as_cid().serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for BlobLink {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            BlobCid::deserialize(deserializer).map(Self)
        } else {
            Cid::deserialize(deserializer).map(|cid| Self(BlobCid(cid)))
        }
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

    // Independently frozen from CIDv1 bytes (01 55 12 20 + SHA-256("hello world"))
    // wrapped as DAG-CBOR tag 42 / byte-string(37) / 00 + CID. This expected
    // value must not be generated through BlobLink or the serializer under test.
    const RAW_HELLO_LINK_CBOR: [u8; 41] = [
        0xd8, 0x2a, 0x58, 0x25, 0x00, 0x01, 0x55, 0x12, 0x20, 0xb9, 0x4d, 0x27, 0xb9, 0x93, 0x4d,
        0x3e, 0x08, 0xa5, 0x2e, 0x52, 0xd7, 0xda, 0x7d, 0xab, 0xfa, 0xc4, 0x84, 0xef, 0xe3, 0x7a,
        0x53, 0x80, 0xee, 0x90, 0x88, 0xf7, 0xac, 0xe2, 0xef, 0xcd, 0xe9,
    ];
    const RAW_HELLO_CID: &str = "bafkreifzjut3te2nhyekklss27nh3k72ysco7y32koao5eei66wof36n5e";
    // The kernel's byte-profile vector for "hello world" (`crates/elohim-address/vectors/
    // address-vectors.json`, derived there with @noble/hashes/blake3 and a hand-written CIDv1
    // encoder): raw 0x55 + BLAKE3-256. Hard-coded so this pin never runs through the code under
    // test.
    const HELLO_BYTE_CID: &str = "bafkr4igxjga67jykbseaxdmmdgc5a5o3zp3htom2l6mrjznk7fvyggu6eq";

    /// A dag-pb (0x70) CID over sha2-256 of `bytes`: a valid multicodec no profile admits.
    fn dag_pb(bytes: &[u8]) -> BlobCid {
        BlobCid(Cid::new_v1(
            0x70,
            *BlobCid::compute_raw(bytes).as_cid().hash(),
        ))
    }

    /// A CID that declares `(codec, multihash)` over a digest it did not compute.
    fn forged(codec: u64, multihash: u64, digest: &[u8]) -> BlobCid {
        BlobCid(Cid::new_v1(
            codec,
            cid::multihash::Multihash::<64>::wrap(multihash, digest).unwrap(),
        ))
    }

    #[test]
    fn blob_link_has_explicit_human_and_canonical_forms() {
        let blob_cid = BlobCid::parse(RAW_HELLO_CID).unwrap();
        let link = BlobLink::from(blob_cid.clone());

        assert_eq!(
            serde_json::to_string(&link).unwrap(),
            format!("\"{RAW_HELLO_CID}\"")
        );
        assert_eq!(
            serde_json::from_str::<BlobLink>(&format!("\"{RAW_HELLO_CID}\"")).unwrap(),
            link
        );
        assert_eq!(
            serde_ipld_dagcbor::to_vec(&link).unwrap(),
            RAW_HELLO_LINK_CBOR
        );
        assert_eq!(
            serde_ipld_dagcbor::from_slice::<BlobLink>(&RAW_HELLO_LINK_CBOR).unwrap(),
            link
        );

        // The pre-existing BlobCid CBOR and JSON string contract remains intact.
        let mut old_cbor = vec![0x78, 0x3b];
        old_cbor.extend_from_slice(RAW_HELLO_CID.as_bytes());
        assert_eq!(serde_ipld_dagcbor::to_vec(&blob_cid).unwrap(), old_cbor);
        assert_eq!(
            serde_json::to_string(&blob_cid).unwrap(),
            format!("\"{RAW_HELLO_CID}\"")
        );
        assert_eq!(BlobCid::from(link), blob_cid);
    }

    #[test]
    fn blob_link_rejects_wrong_cbor_representation_and_invalid_json() {
        let mut old_cbor = vec![0x78, 0x3b];
        old_cbor.extend_from_slice(RAW_HELLO_CID.as_bytes());
        assert!(serde_ipld_dagcbor::from_slice::<BlobLink>(&old_cbor).is_err());

        let mut wrong_sentinel = RAW_HELLO_LINK_CBOR;
        wrong_sentinel[4] = 0x01;
        assert!(serde_ipld_dagcbor::from_slice::<BlobLink>(&wrong_sentinel).is_err());

        let mut wrong_tag = RAW_HELLO_LINK_CBOR;
        wrong_tag[1] = 0x2b;
        assert!(serde_ipld_dagcbor::from_slice::<BlobLink>(&wrong_tag).is_err());

        let truncated = &RAW_HELLO_LINK_CBOR[..RAW_HELLO_LINK_CBOR.len() - 1];
        assert!(serde_ipld_dagcbor::from_slice::<BlobLink>(truncated).is_err());
        assert!(serde_json::from_str::<BlobLink>("\"not-a-cid\"").is_err());
    }

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
        assert_eq!(
            raw.short_fingerprint().unwrap(),
            cbor.short_fingerprint().unwrap()
        );
    }

    // The convergence pin: the cite fingerprint `sha256:hex16` IS the short-form of the raw-codec
    // CID over the same canonical-body bytes. Expected value hard-coded from the Python oracle
    // (`cite_graph.fingerprint_text("ok target body")`).
    #[test]
    fn short_fingerprint_equals_python_cite_fingerprint() {
        let body = b"ok target body";
        let cid = BlobCid::compute_raw(body);
        assert_eq!(cid.short_fingerprint().unwrap(), "sha256:4e72cded3d6affd3");
    }

    #[test]
    fn short_fingerprint_shape() {
        let f = BlobCid::compute_raw(b"anything")
            .short_fingerprint()
            .unwrap();
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

    #[test]
    fn recompute_preserves_declared_profile_and_external_vectors() {
        let raw =
            BlobCid::parse("bafkreifzjut3te2nhyekklss27nh3k72ysco7y32koao5eei66wof36n5e").unwrap();
        assert_eq!(raw.recompute_for_profile(b"hello world"), Some(raw));

        let cbor =
            BlobCid::parse("bafyreifzjut3te2nhyekklss27nh3k72ysco7y32koao5eei66wof36n5e").unwrap();
        assert_eq!(cbor.recompute_for_profile(b"hello world"), Some(cbor));

        let byte = BlobCid::parse(HELLO_BYTE_CID).unwrap();
        assert_eq!(byte.recompute_for_profile(b"hello world"), Some(byte));

        let unsupported = dag_pb(b"hello world");
        assert_eq!(unsupported.recompute_for_profile(b"hello world"), None);
    }

    // An unrecognized codec must fail closed. Verification is the tamper-detection primitive; a
    // codec we cannot recompute is an unverified claim, never a passing one.
    #[test]
    fn verifies_fails_closed_on_unknown_codec() {
        // 0x70 = dag-pb, a valid multicodec this type never mints.
        assert!(!dag_pb(b"hello world").verifies(b"hello world"));
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

    #[test]
    fn compute_bytes_matches_kernel_vector() {
        let cid = BlobCid::compute_bytes(b"hello world");
        assert_eq!(cid.to_string(), HELLO_BYTE_CID);
        assert!(cid.verifies(b"hello world"));
        assert!(!cid.verifies(b"hello w0rld"));
        // The byte profile and the citation profile never share an address over the same bytes.
        assert_ne!(cid, BlobCid::compute_raw(b"hello world"));
        assert_eq!(
            cid.byte_cid().unwrap().to_string(),
            HELLO_BYTE_CID,
            "a minted byte CID is admitted by the full-decode byte surface"
        );
    }

    #[test]
    fn verifies_rejects_sha2_cid_over_blake3_bytes() {
        // A citation-profile (sha2) CID carrying the BLAKE3 digest of the bytes, and a byte-profile
        // (BLAKE3) CID carrying their sha2 digest: each claims bytes it does not address under its
        // own declared multihash, so neither verifies.
        let bytes = b"hello world";
        let blake3 = *BlobCid::compute_bytes(bytes).as_cid().hash();
        let sha2 = *BlobCid::compute_raw(bytes).as_cid().hash();
        assert!(!forged(CITE_CODEC, SHA2_256, blake3.digest()).verifies(bytes));
        assert!(!forged(BYTE_CODEC, BYTE_MULTIHASH, sha2.digest()).verifies(bytes));
        // And the honest constructors each verify only under their own profile.
        assert!(BlobCid::compute_raw(bytes).verifies(bytes));
        assert!(BlobCid::compute_bytes(bytes).verifies(bytes));
    }

    #[test]
    fn short_fingerprint_refuses_blake3() {
        let err = BlobCid::compute_bytes(b"ok target body")
            .short_fingerprint()
            .unwrap_err();
        assert_eq!(err.check(), "not-sha2");
        // The citation profile still renders the frozen fingerprint.
        assert_eq!(
            BlobCid::compute_raw(b"ok target body")
                .short_fingerprint()
                .unwrap(),
            "sha256:4e72cded3d6affd3"
        );
    }

    // The kernel's malformed-header negatives: `BlobCid::parse` admits any well-formed CID, so the
    // byte surface (`byte_cid`) is where each is refused, with the check the kernel names.
    #[test]
    fn byte_surface_refuses_the_negative_header_vectors() {
        for (input, check) in [
            // a raw+sha2 citation CID offered as payload bytes
            (RAW_HELLO_CID, "multihash"),
            // a dag-cbor record CID offered as payload bytes
            (
                "bafyreifzjut3te2nhyekklss27nh3k72ysco7y32koao5eei66wof36n5e",
                "codec",
            ),
            // dag-pb codec over the BLAKE3 digest
            (
                "bafyb4igxjga67jykbseaxdmmdgc5a5o3zp3htom2l6mrjznk7fvyggu6eq",
                "codec",
            ),
        ] {
            let parsed = BlobCid::parse(input).unwrap();
            assert_eq!(parsed.byte_cid().unwrap_err().check(), check, "{input}");
            assert_eq!(ByteCid::parse(input).unwrap_err().check(), check, "{input}");
        }
        // A header declaring a 33-byte digest over 32 supplied bytes, and non-zero padding bits:
        // refused by the full decode before any CID exists.
        assert_eq!(
            ByteCid::parse("bafkr4ioxjga67jykbseaxdmmdgc5a5o3zp3htom2l6mrjznk7fvyggu6eq")
                .unwrap_err()
                .check(),
            "digest-length"
        );
        assert_eq!(
            ByteCid::parse("bafkr4igxjga67jykbseaxdmmdgc5a5o3zp3htom2l6mrjznk7fvyggu6er")
                .unwrap_err()
                .check(),
            "padding"
        );
    }
}
