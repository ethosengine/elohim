//! Conformance against `vectors/address-vectors.json` — the positive rows were derived
//! outside this crate (coreutils sha256sum, @noble/hashes blake3, a hand-written Python CID
//! encoder), the negative rows are inputs the byte surface must refuse, each naming the first
//! check that fails.

mod json;

use elohim_address::{
    compute_bytes_cid, compute_raw_sha2_cid, compute_record_cid, short_fingerprint, verify_cid_any,
    ByteCid, Verify, BYTE_CID_LEN, BYTE_CID_PREFIX, CITE_CID_PREFIX, RECORD_CID_PREFIX,
};
use json::Json;

fn vectors() -> Json {
    Json::parse(include_str!("../vectors/address-vectors.json")).expect("vector file parses")
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn positive_rows_match_the_independent_oracle() {
    let v = vectors();
    let rows = v.get("positive").arr();
    assert_eq!(rows.len(), 3, "hello world, abc and the empty input");
    for row in rows {
        let input = row.get("input").str().as_bytes();

        // The digests themselves, recomputed with the hash crates directly.
        assert_eq!(
            blake3::hash(input).as_bytes().to_vec(),
            unhex(row.get("blake3").str())
        );
        {
            use sha2::Digest;
            assert_eq!(
                sha2::Sha256::digest(input).to_vec(),
                unhex(row.get("sha256").str())
            );
        }

        let bytes = compute_bytes_cid(input);
        assert_eq!(bytes.to_string(), row.get("byteCid").str());
        assert_eq!(bytes.digest().to_vec(), unhex(row.get("blake3").str()));
        assert_eq!(ByteCid::parse(row.get("byteCid").str()).unwrap(), bytes);
        assert!(bytes.to_string().starts_with(BYTE_CID_PREFIX));
        assert_eq!(bytes.to_string().len(), BYTE_CID_LEN);

        let cite = compute_raw_sha2_cid(input);
        assert_eq!(cite.to_string(), row.get("citeCid").str());
        assert!(cite.to_string().starts_with(CITE_CID_PREFIX));

        let record = compute_record_cid(input);
        assert_eq!(record.to_string(), row.get("recordCid").str());
        assert!(record.to_string().starts_with(RECORD_CID_PREFIX));

        assert_eq!(
            short_fingerprint(&cite).unwrap(),
            row.get("shortFingerprint").str()
        );
        assert_eq!(
            short_fingerprint(&record).unwrap(),
            row.get("shortFingerprint").str()
        );

        for cid in [*bytes.as_cid(), cite, record] {
            assert_eq!(verify_cid_any(&cid, input), Verify::Verified);
            assert_eq!(verify_cid_any(&cid, b"tampered"), Verify::Mismatch);
        }
    }
}

#[test]
fn negative_rows_are_refused_naming_the_failed_check() {
    let v = vectors();
    let rows = v.get("negative").arr();
    assert!(rows.len() >= 10);
    for row in rows {
        let name = row.get("name").str();
        let input = row.get("input").str();
        let err = ByteCid::parse(input)
            .err()
            .unwrap_or_else(|| panic!("{name}: {input:?} was admitted"));
        assert_eq!(err.check(), row.get("check").str(), "{name}: {err}");
        // `FromStr` is the same parser, never a looser one.
        assert!(input.parse::<ByteCid>().is_err(), "{name}");
    }
}

#[test]
fn byte_cid_from_cid_refuses_every_other_profile() {
    let v = vectors();
    let row = &v.get("positive").arr()[0];
    let input = row.get("input").str().as_bytes();
    assert!(ByteCid::try_from(compute_raw_sha2_cid(input)).is_err());
    assert!(ByteCid::try_from(compute_record_cid(input)).is_err());
    let bytes = compute_bytes_cid(input);
    assert_eq!(ByteCid::try_from(*bytes.as_cid()).unwrap(), bytes);
}
