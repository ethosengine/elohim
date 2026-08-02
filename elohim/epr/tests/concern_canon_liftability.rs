//! Brit-liftability gate for the CONCERN CANON (seam-concern-contract plan, task P0.4).
//!
//! WHAT THIS PROVES — and, just as importantly, what it does not.
//!
//! The canon rows live today in `.claude/epr-meta/concerns.yaml` +
//! `.claude/epr-meta/policies.yaml`, pinned by a canonical-JSON `sha256` written by
//! `.claude/scripts/epr-meta-pin.py` (the SAME canonicalization `_lib.epr_meta.policy_content_hash`
//! uses at resolve time). **That live pin mechanism is untouched by this test.** P5 will lift the
//! rows onto the eprfs/brit layer as CIDv1 EPR atoms under declared heads; this test is the gate
//! that guarantees the lift is a MOVE, not a REWRITE:
//!
//!   the exact bytes the sha256 pin already covers, wrapped in CIDv1(dag-cbor, sha2-256)
//!   via `elohim_epr::cid::compute_cid`, carry a multihash digest BYTE-IDENTICAL to that pin.
//!
//! If that holds, the lift is literally a codec wrap of bytes that already exist. If it ever stops
//! holding, someone changed the canonicalization on one side and the P5 lift becomes a rewrite —
//! which is the thing this gate exists to make impossible to do quietly.
//!
//! FIXTURE, NOT MIRROR. The fixture (`tests/vectors/concern_canon_row_c0.json`) is the frozen
//! canonical-JSON body of `c0-plane-location@1` — sorted keys, compact separators, ASCII-escaped,
//! minus the lifecycle fields (`contentHash`, `status`, `superseded_by`) the pin deliberately
//! excludes. The two sides are computed INDEPENDENTLY on purpose (the anti-mirror discipline: a
//! passing contract test can be measuring the mirror):
//!   • this Rust test derives sha256 + the CID from the fixture bytes, through the real
//!     `compute_cid` path, and compares against `PINNED_CONTENT_HASH` transcribed from the registry;
//!   • `.claude/scripts/_lib/__tests__/concern_canon_liftability_test.py` regenerates the body from
//!     the live YAML and asserts byte-equality with this fixture — so a canon edit that leaves the
//!     fixture behind fails LOUD there instead of passing green here.
//!
//! STANDING (brit frame). The canon rows are `attestation`s; this fixture is a `pin` — a sealed
//! fingerprint re-blessed by judgment (`--write-fixture` on the Python sibling, run deliberately
//! after re-reading the row), never auto-blessed by recency. Identity here is the CONTENT address,
//! never a provenance/anchor hash: the entry-hash discipline.
//!
//! FAIL-CLOSED. An unrecognized codec (or multihash) is an UNVERIFIED CLAIM, never a pass — the
//! `9e9c9d023` (2026-07-24) lesson from the eprfs/cite-seal family, where `BlobCid::verifies`
//! hardcoded the wrong codec and self-verification was inert for every real fingerprint while the
//! test stayed green because it computed both sides the same wrong way.

use elohim_epr::cid::compute_cid;
use multihash_codetable::{Code, MultihashDigest};

/// The frozen canonical-JSON body of `.claude/epr-meta/concerns.yaml#c0-plane-location@1`.
/// Regenerate with the Python sibling test's `--write-fixture`, never by hand.
const CANON_ROW_C0: &[u8] = include_bytes!("vectors/concern_canon_row_c0.json");

/// Transcribed from that row's `contentHash:` — the LIVE pin, written by `epr-meta-pin.py --write`.
const PINNED_CONTENT_HASH: &str =
    "sha256:df38bb561769ff552e76d63391996e41093a70621a47fae47ac25fcfa3500505";

/// IPLD multicodec for dag-cbor, and the multihash code for sha2-256 — the ONLY pair this gate
/// recognizes. Anything else classifies `Unverified` (see `classify`).
const DAG_CBOR_CODEC: u64 = 0x71;
const SHA2_256_MH: u64 = 0x12;

/// Fail-closed liftability verdict. There is deliberately no `Unknown => pass` arm: an
/// unrecognized codec is an unverified CLAIM about liftability, never evidence of it.
#[derive(Debug, PartialEq, Eq)]
enum Liftability {
    /// CIDv1 + dag-cbor + sha2-256, and the multihash digest equals the pinned sha256 body hash.
    VerifiedMove,
    /// Right shape, but the digest does not reproduce the pin — the lift would be a rewrite.
    DigestMismatch,
    /// Unrecognized codec/multihash/version. Confers nothing, in either direction (C5).
    Unverified,
}

fn classify(cid: &cid::Cid, expected_sha256: &[u8]) -> Liftability {
    if cid.version() != cid::Version::V1 || cid.codec() != DAG_CBOR_CODEC {
        return Liftability::Unverified;
    }
    let mh = cid.hash();
    if mh.code() != SHA2_256_MH {
        return Liftability::Unverified;
    }
    if mh.digest() != expected_sha256 {
        return Liftability::DigestMismatch;
    }
    Liftability::VerifiedMove
}

fn sha256(bytes: &[u8]) -> Vec<u8> {
    Code::Sha2_256.digest(bytes).digest().to_vec()
}

/// The fixture IS the pinned bytes — proven against the registry's own pin, not asserted.
/// This is the "without rewriting" half: if the fixture were a re-serialization (different key
/// order, different separators, non-ASCII escapes) this fails before the CID is ever computed.
#[test]
fn fixture_bytes_reproduce_the_live_sha256_pin() {
    let hex = format!("sha256:{}", hex::encode(sha256(CANON_ROW_C0)));
    assert_eq!(
        hex, PINNED_CONTENT_HASH,
        "fixture bytes no longer hash to the registry pin — the canon row moved and the fixture \
         did not (or the canonicalization changed). Re-generate the fixture and re-read the row."
    );
}

/// The lift itself: the SAME bytes, addressed as CIDv1(dag-cbor, sha2-256).
#[test]
fn canon_row_re_addresses_as_cidv1_dag_cbor_without_rewriting() {
    let cid = compute_cid(CANON_ROW_C0);
    let expected = sha256(CANON_ROW_C0);
    assert_eq!(
        classify(&cid, &expected),
        Liftability::VerifiedMove,
        "canon row did not re-address as CIDv1(dag-cbor, sha2-256) over the pinned bytes: {cid}"
    );
    // The load-bearing identity claim: the CID's digest and the sha256 pin are the same 32 bytes.
    // The P5 lift therefore wraps a codec around a hash that already exists — a move, not a rewrite.
    assert_eq!(
        format!("sha256:{}", hex::encode(cid.hash().digest())),
        PINNED_CONTENT_HASH
    );
    // CIDv1 + dag-cbor + sha2-256 renders in base32 with the `bafyrei` prefix.
    assert!(cid.to_string().starts_with("bafyrei"), "got {cid}");
}

/// Identity is the CONTENT address — never a provenance/anchor hash. Two rows with different
/// bodies must never share a CID, and the same body must address identically on every node.
#[test]
fn content_address_is_deterministic_and_body_sensitive() {
    assert_eq!(compute_cid(CANON_ROW_C0), compute_cid(CANON_ROW_C0));
    let mut mutated = CANON_ROW_C0.to_vec();
    let last = mutated.len() - 1;
    mutated[last] ^= 0x01; // one bit of the body
    assert_ne!(
        compute_cid(CANON_ROW_C0),
        compute_cid(&mutated),
        "a one-bit body change did not move the address — identity is not content-derived"
    );
}

/// FAIL-CLOSED: an unrecognized codec is an unverified claim, never a pass. Built by hand rather
/// than through `compute_cid` precisely so the assertion cannot be satisfied by the happy path.
#[test]
fn unrecognized_codec_is_unverified_never_a_pass() {
    let digest = sha256(CANON_ROW_C0);
    let mh = Code::Sha2_256.digest(CANON_ROW_C0);

    // raw (0x55) instead of dag-cbor (0x71): correct bytes, correct hash, WRONG codec.
    let raw_cid = cid::Cid::new_v1(0x55, mh);
    assert_eq!(
        classify(&raw_cid, &digest),
        Liftability::Unverified,
        "a raw-codec CID over the right bytes must NOT verify as liftable"
    );

    // A different multihash over the same bytes: shape recognized, digest is not the pin.
    let sha512_mh = Code::Sha2_512.digest(CANON_ROW_C0);
    let sha512_cid = cid::Cid::new_v1(DAG_CBOR_CODEC, sha512_mh);
    assert_eq!(
        classify(&sha512_cid, &digest),
        Liftability::Unverified,
        "a non-sha2-256 multihash must NOT verify as liftable"
    );

    // Right shape, wrong bytes → DigestMismatch, which is also not a pass.
    let other = compute_cid(b"not the canon row");
    assert_eq!(classify(&other, &digest), Liftability::DigestMismatch);
    assert_ne!(classify(&other, &digest), Liftability::VerifiedMove);
}
