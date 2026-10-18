//! CID derivation for EPR canonical bytes.
//!
//! CIDv1, codec = 0x71 (dag-cbor), multihash = 0x12 (sha2-256) for atoms; codec = 0x55 (raw)
//! for arbitrary bytes and for checksums published outside the protocol (lockfiles, spec sheets).

use cid::Cid;
use multihash_codetable::{Code, MultihashDigest};

/// Codec byte for dag-cbor per the IPLD multicodec table.
///
/// The crate's single definition — [`crate::head`] re-exports it rather than
/// declaring a second one.
pub const DAG_CBOR_CODEC: u64 = 0x71;

/// Compute CIDv1(dag-cbor, sha2-256) over the given canonical bytes.
pub fn compute_cid(canonical_bytes: &[u8]) -> Cid {
    let mh = Code::Sha2_256.digest(canonical_bytes);
    Cid::new_v1(DAG_CBOR_CODEC, mh)
}

/// Verify that a given CID matches the hash of the given canonical bytes.
pub fn verify_cid(claimed: &Cid, canonical_bytes: &[u8]) -> bool {
    compute_cid(canonical_bytes) == *claimed
}

// ---------------------------------------------------------------------------
// External resource identity — the CID that wraps a checksum someone else published
// ---------------------------------------------------------------------------
//
// A thing made outside the protocol (a library version, a supplier's spec sheet, a seed-lot
// record) is often already pinned by a checksum its own ecosystem publishes: Cargo.lock's
// `checksum` is the sha256 of the `.crate` file, npm/pnpm's `integrity` is a Subresource
// Integrity string over the tarball. Those digests ARE content addresses; wrapping one in a
// CIDv1 with the raw codec names the same bytes in the protocol's one address format, with no
// re-download and no second format. Recompute from the bytes and you get the same CID — that is
// the probe (see `tests::external_checksum_cid_equals_cid_of_the_bytes`).

/// Codec byte for raw bytes per the IPLD multicodec table — for file bytes and anything that is
/// not a canonical dag-cbor atom. A raw sha2-256 CID renders as `bafkrei…`.
pub const RAW_CODEC: u64 = 0x55;

/// Multihash codes for the digests an external checksum can carry.
const SHA2_256: u64 = 0x12;
const SHA2_384: u64 = 0x20;
const SHA2_512: u64 = 0x13;

/// Why an external checksum could not be wrapped as a CID.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ChecksumError {
    #[error("checksum is not hex: {0}")]
    NotHex(String),
    #[error("{algorithm} digest must be {expected} bytes, got {actual}")]
    WrongLength {
        algorithm: &'static str,
        expected: usize,
        actual: usize,
    },
    #[error("integrity string has no `<algorithm>-<base64>` entry: {0}")]
    MalformedIntegrity(String),
    #[error("integrity digest is not base64: {0}")]
    NotBase64(String),
    /// sha1 (old npm packages) and anything else outside sha2-256/384/512. A sha1 digest is a
    /// collision-broken address; it is refused rather than wrapped, so a weak pin can never pass
    /// for a strong one.
    #[error("unsupported integrity algorithm `{0}` (sha256, sha384, sha512 only)")]
    UnsupportedAlgorithm(String),
    /// Two different digests of the same (strongest) algorithm: which one names the bytes would
    /// depend on their order in the string, so neither is chosen.
    #[error("integrity string carries two different {0} digests")]
    ConflictingDigests(&'static str),
}

/// Compute CIDv1(raw, sha2-256) over arbitrary bytes (`bafkrei…`). Byte-identical to
/// `eprfs_core::BlobCid::compute_raw`.
pub fn compute_raw_cid(bytes: &[u8]) -> Cid {
    Cid::new_v1(RAW_CODEC, Code::Sha2_256.digest(bytes))
}

fn wrap_raw(code: u64, algorithm: &'static str, digest: &[u8]) -> Result<Cid, ChecksumError> {
    let expected = match code {
        SHA2_256 => 32,
        SHA2_384 => 48,
        _ => 64,
    };
    if digest.len() != expected {
        return Err(ChecksumError::WrongLength {
            algorithm,
            expected,
            actual: digest.len(),
        });
    }
    let mh = cid::multihash::Multihash::<64>::wrap(code, digest).map_err(|_| {
        ChecksumError::WrongLength {
            algorithm,
            expected,
            actual: digest.len(),
        }
    })?;
    Ok(Cid::new_v1(RAW_CODEC, mh))
}

/// Wrap a published sha256 hex digest (e.g. a `Cargo.lock` `checksum`, the sha256 of the
/// `.crate` file) as CIDv1(raw, sha2-256). Equal to [`compute_raw_cid`] over the same bytes.
pub fn raw_cid_from_sha256_hex(hex_digest: &str) -> Result<Cid, ChecksumError> {
    let digest =
        hex::decode(hex_digest.trim()).map_err(|_| ChecksumError::NotHex(hex_digest.into()))?;
    wrap_raw(SHA2_256, "sha256", &digest)
}

/// Wrap a Subresource Integrity string (`<algorithm>-<base64 digest>`, e.g. npm/pnpm's
/// `integrity: sha512-…`) as CIDv1(raw, sha2-*). When the string lists several space-separated
/// entries, the strongest supported algorithm is used, as the SRI spec directs. Every entry of
/// that strength is decoded: identical digests collapse to one, and two DIFFERENT digests of the
/// same strength are refused ([`ChecksumError::ConflictingDigests`]) — otherwise the entries'
/// order would choose the address.
pub fn raw_cid_from_sri(integrity: &str) -> Result<Cid, ChecksumError> {
    use base64::Engine as _;

    let strength = |c: u64| match c {
        SHA2_256 => 1,
        SHA2_384 => 2,
        _ => 3,
    };
    let mut entries: Vec<(u64, &'static str, &str)> = Vec::new();
    let mut unsupported: Option<String> = None;
    for entry in integrity.split_whitespace() {
        // An SRI entry may carry `?options` after the digest; they never change the digest.
        let entry = entry.split('?').next().unwrap_or(entry);
        let Some((algorithm, digest)) = entry.split_once('-') else {
            continue;
        };
        let (code, name) = match algorithm {
            "sha256" => (SHA2_256, "sha256"),
            "sha384" => (SHA2_384, "sha384"),
            "sha512" => (SHA2_512, "sha512"),
            other => {
                unsupported.get_or_insert_with(|| other.to_string());
                continue;
            }
        };
        entries.push((code, name, digest));
    }
    let Some(top) = entries.iter().map(|(c, _, _)| strength(*c)).max() else {
        return Err(match unsupported {
            Some(algorithm) => ChecksumError::UnsupportedAlgorithm(algorithm),
            None => ChecksumError::MalformedIntegrity(integrity.into()),
        });
    };
    let mut chosen: Option<(u64, &'static str, Vec<u8>)> = None;
    for (code, name, digest_b64) in entries.into_iter().filter(|(c, _, _)| strength(*c) == top) {
        let digest = base64::engine::general_purpose::STANDARD
            .decode(digest_b64)
            .map_err(|_| ChecksumError::NotBase64(digest_b64.into()))?;
        match &chosen {
            None => chosen = Some((code, name, digest)),
            Some((_, _, seen)) if *seen == digest => {}
            Some(_) => return Err(ChecksumError::ConflictingDigests(name)),
        }
    }
    let (code, name, digest) = chosen.expect("at least one entry has the top strength");
    wrap_raw(code, name, &digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Vectors computed independently (Python hashlib + a hand-rolled CIDv1 encoder), never by
    // this crate, so a test cannot pass by agreeing with itself.
    const FIXTURE_BYTES: &[u8] = b"elohim fixture crate bytes\n";
    /// What a `Cargo.lock` entry for FIXTURE_BYTES would carry: `checksum = "<this>"`.
    const FIXTURE_SHA256_HEX: &str =
        "d82a8b4f1f75b6422de6545986df3943d10fe75690461d441e98bc4d3fcb9419";
    const FIXTURE_RAW_SHA256_CID: &str =
        "bafkreigyfkfu6h3vwzbc3zsulgdn6okd2eh6ovuqiyouihuyxrgt7s4ude";
    /// What an npm/pnpm lockfile entry for FIXTURE_BYTES would carry: `integrity: <this>`.
    const FIXTURE_SRI_SHA512: &str =
        "sha512-8KcksMl1RIuwfwHrjTZxB3FD8ek50NkwhjGg+fw1GyVH1shnSPSiq+BNz6yzT6HkQhJsy9kBOQLEehkdYFw/zg==";
    const FIXTURE_RAW_SHA512_CID: &str = "bafkrgqhqu4slbslvisf3a7yb5ogtm4ihofb7d2jz2dmtbbrrud47yni3evd5nsdhjd2kfk7ajxh2zm2puhseeetmzpmqcoicyr5bshlalq744";
    const FIXTURE_SRI_SHA384: &str =
        "sha384-2oOpnXic7WTEM/Gz/dO6A0k5LOQUkmmrD1xVksKUEkgOyP+1Y4AJ3QI33zmE4o98";

    /// Node 2's probe: a package file's bytes, hashed, give the same CID as the one wrapped
    /// from its lockfile checksum. The fixture is a small file written here, not a download.
    #[test]
    fn external_checksum_cid_equals_cid_of_the_bytes() {
        let dir = std::env::temp_dir().join(format!("epr-cid-fixture-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("fixture-0.1.0.crate");
        std::fs::write(&path, FIXTURE_BYTES).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        std::fs::remove_dir_all(&dir).ok();

        let from_bytes = compute_raw_cid(&bytes);
        let from_lockfile = raw_cid_from_sha256_hex(FIXTURE_SHA256_HEX).unwrap();
        assert_eq!(from_bytes, from_lockfile);
        assert_eq!(from_lockfile.to_string(), FIXTURE_RAW_SHA256_CID);
        assert!(from_lockfile.to_string().starts_with("bafkrei"));
        assert_eq!(from_lockfile.codec(), RAW_CODEC);
    }

    #[test]
    fn sri_integrity_wraps_a_sha2_512_multihash() {
        let cid = raw_cid_from_sri(FIXTURE_SRI_SHA512).unwrap();
        assert_eq!(cid.to_string(), FIXTURE_RAW_SHA512_CID);
        assert_eq!(cid.codec(), RAW_CODEC);
        assert_eq!(cid.hash().code(), SHA2_512);
        assert_eq!(
            cid,
            Cid::new_v1(RAW_CODEC, Code::Sha2_512.digest(FIXTURE_BYTES))
        );
    }

    #[test]
    fn sri_takes_the_strongest_listed_digest() {
        let both = format!("{FIXTURE_SRI_SHA384} {FIXTURE_SRI_SHA512}");
        assert_eq!(
            raw_cid_from_sri(&both).unwrap().to_string(),
            FIXTURE_RAW_SHA512_CID
        );
        let only_384 = raw_cid_from_sri(FIXTURE_SRI_SHA384).unwrap();
        assert_eq!(only_384.hash().code(), SHA2_384);
    }

    #[test]
    fn sri_entries_of_equal_strength_never_let_order_choose_the_address() {
        // The same digest twice collapses to one CID, in either order.
        let twice = format!("{FIXTURE_SRI_SHA512} {FIXTURE_SRI_SHA512}");
        assert_eq!(
            raw_cid_from_sri(&twice).unwrap().to_string(),
            FIXTURE_RAW_SHA512_CID
        );
        // Two DIFFERENT sha512 digests refuse, whichever comes first.
        let other = "sha512-3a81oZNherrMQXNJriBBMRLm+k6JqX6iCp7u5ktV05ohkpkqJ0/BqDa6PCOj/uu9RU1EI2Q86A4qmslPpUyknw==";
        for s in [
            format!("{FIXTURE_SRI_SHA512} {other}"),
            format!("{other} {FIXTURE_SRI_SHA512}"),
        ] {
            assert_eq!(
                raw_cid_from_sri(&s),
                Err(ChecksumError::ConflictingDigests("sha512"))
            );
        }
        // A weaker entry that disagrees is ignored: only the strongest algorithm is read.
        let mixed = format!("sha256-YWJj {FIXTURE_SRI_SHA512}");
        assert_eq!(
            raw_cid_from_sri(&mixed).unwrap().to_string(),
            FIXTURE_RAW_SHA512_CID
        );
    }

    #[test]
    fn known_answer_abc_matches_nist_sha256() {
        // NIST FIPS 180-2 sha256("abc").
        let cid = raw_cid_from_sha256_hex(
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        )
        .unwrap();
        assert_eq!(
            cid.to_string(),
            "bafkreif2pall7dybz7vecqka3zo24irdwabwdi4wc55jznaq75q7eaavvu"
        );
        assert_eq!(cid, compute_raw_cid(b"abc"));
    }

    /// The raw-codec format is shared, not re-derived: the same golden vector
    /// `eprfs_core::BlobCid::compute_raw` and brit's `BritCid::compute_raw` pin
    /// (`raw_codec_matches_brit_vector` in elohim/eprfs/eprfs-core/src/address.rs).
    #[test]
    fn compute_raw_cid_matches_the_shared_brit_vector() {
        assert_eq!(
            compute_raw_cid(b"hello world").to_string(),
            "bafkreifzjut3te2nhyekklss27nh3k72ysco7y32koao5eei66wof36n5e"
        );
    }

    #[test]
    fn malformed_checksums_are_refused_not_guessed() {
        assert!(matches!(
            raw_cid_from_sha256_hex("not-hex"),
            Err(ChecksumError::NotHex(_))
        ));
        assert!(matches!(
            raw_cid_from_sha256_hex("abcd"),
            Err(ChecksumError::WrongLength { expected: 32, .. })
        ));
        assert!(matches!(
            raw_cid_from_sri("sha1-qUqP5cyxm6YcTAhz05Hph5gvu9M="),
            Err(ChecksumError::UnsupportedAlgorithm(a)) if a == "sha1"
        ));
        assert!(matches!(
            raw_cid_from_sri("no dash here"),
            Err(ChecksumError::MalformedIntegrity(_))
        ));
        assert!(matches!(
            raw_cid_from_sri("sha512-!!!"),
            Err(ChecksumError::NotBase64(_))
        ));
        assert!(matches!(
            raw_cid_from_sri("sha512-YWJj"),
            Err(ChecksumError::WrongLength { expected: 64, .. })
        ));
    }

    #[test]
    fn raw_and_dag_cbor_cids_over_the_same_bytes_differ() {
        // The codec tag is a claim about what the bytes are; a package file is not an atom.
        assert_ne!(compute_raw_cid(FIXTURE_BYTES), compute_cid(FIXTURE_BYTES));
    }
}
