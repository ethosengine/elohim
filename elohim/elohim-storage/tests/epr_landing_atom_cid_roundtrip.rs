//! Round-trip CID parity test for the `elohim-host-landing` EPR atom.
//!
//! Verifies that the Rust `Envelope::canonical_bytes` + `compute_cid` path
//! produces the EXACT CID that the TypeScript seeder (`seed-epr-atom.ts`)
//! asserts: `bafyreigadm5oiuvm7t7izjunqyqqjiqitujolwxqo64tezkkfgg35d45fi`.
//!
//! If this test fails, the seeder's dag-cbor encoding diverges from Rust and
//! the `PUT /api/v1/epr/:cid` Stage-1 canonicalization check will reject the atom.
//!
//! ## Seed values (must be byte-identical to genesis/seeder/src/seed-epr-atom.ts)
//!
//! SEEDER_ED25519_SEED  = "elohim-seeder-landing-atom-001\0\0" (32 bytes)
//! SCHEMA_REF_SEED      = "schema-ref-landing-page-v1" (26 bytes)
//! GOVERNANCE_SEED      = "elohim-host-governance-v1" (25 bytes)
//! kind       = "Manifest"
//! reach      = "commons"
//! schemaKey  = "elohim-host-landing"
//! issuedAt   = 2026-01-01T00:00:00Z
//! payload    = JSON.stringify({ title, description, schemaKey })
//!              (JS insertion-order: title first, then description, then schemaKey)
//! claims = [], coupling = { governance only }, supersedes = None
//!
//! CID derivation in both Rust and TS:
//!   - sha2-256 hash seed bytes  →  CIDv1(dag-cbor codec 0x71, sha2-256 multihash)
//!     This is exactly `elohim_epr::cid::compute_cid(seed_bytes)` on the Rust side
//!     and `CID.create(1, 0x71, await sha256.digest(seedBytes))` on the TS side.

use chrono::TimeZone;
use elohim_epr::{cid::compute_cid, proof::AgentKeypair, Coupling, Epr, EprKind, Reach};

#[test]
fn landing_atom_cid_matches_ts_seeder() {
    // ---- Signing keypair (mirror SEEDER_ED25519_SEED in seed-epr-atom.ts) ----
    let seeder_seed: [u8; 32] = [
        0x65, 0x6c, 0x6f, 0x68, 0x69, 0x6d, 0x2d, 0x73, // "elohim-s"
        0x65, 0x65, 0x64, 0x65, 0x72, 0x2d, 0x6c, 0x61, // "eeder-la"
        0x6e, 0x64, 0x69, 0x6e, 0x67, 0x2d, 0x61, 0x74, // "nding-at"
        0x6f, 0x6d, 0x2d, 0x30, 0x30, 0x31, 0x00, 0x00, // "om-001\0\0"
    ];
    let kp = AgentKeypair::from_secret(&seeder_seed).expect("keypair from fixed seed");

    // ---- schema_ref CID: compute_cid(SCHEMA_REF_SEED_BYTES) ----
    // Matches TS: CID.create(1, 0x71, await sha256.digest(SCHEMA_REF_SEED_BYTES))
    let schema_ref_cid = compute_cid(b"schema-ref-landing-page-v1");

    // ---- signer CID: compute_cid(pubkey_bytes) ----
    // Matches TS: CID.create(1, 0x71, await sha256.digest(publicKeyBytes))
    let pubkey_bytes = kp.public_key_bytes();
    let signer_cid = compute_cid(&pubkey_bytes);

    // ---- governance CID: compute_cid(GOVERNANCE_SEED_BYTES) ----
    // Matches TS: CID.create(1, 0x71, await sha256.digest(governanceBytes))
    let governance_cid = compute_cid(b"elohim-host-governance-v1");

    // ---- payload bytes ----
    // JS: JSON.stringify({ title: '...', description: '...', schemaKey: '...' })
    // JSON.stringify preserves insertion order; no key sorting. Must be identical.
    let payload_str = r#"{"title":"Elohim Host Landing","description":"The hosted landing page for elohim.host / alpha.elohim.host.","schemaKey":"elohim-host-landing"}"#;
    let payload = payload_str.as_bytes().to_vec();

    // ---- issued_at ----
    let issued_at = chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();

    // ---- Build the Epr ----
    let epr = Epr::builder()
        .kind(EprKind::Manifest)
        .schema_ref(schema_ref_cid)
        .schema_key("elohim-host-landing")
        .reach(Reach::Commons)
        .coupling(Coupling {
            knowledge: None,
            value: None,
            governance: Some(governance_cid),
        })
        .issued_at(issued_at)
        .payload(payload.clone())
        .sign(&kp, signer_cid)
        .expect("sign landing atom");

    let canonical = epr
        .envelope
        .canonical_bytes(&payload)
        .expect("canonical_bytes");
    let derived_cid = compute_cid(&canonical);
    let cid_str = derived_cid.to_string();

    println!("Rust-derived CID:   {}", cid_str);
    println!("Expected (TS) CID:  bafyreigadm5oiuvm7t7izjunqyqqjiqitujolwxqo64tezkkfgg35d45fi");
    println!("schema_ref CID:     {}", schema_ref_cid);
    println!("signer CID:         {}", signer_cid);
    println!("governance CID:     {}", governance_cid);
    println!("canonical bytes len: {}", canonical.len());
    println!("canonical bytes hex: {}", hex::encode(&canonical));
    println!("payload: {}", payload_str);

    assert_eq!(
        cid_str, "bafyreigadm5oiuvm7t7izjunqyqqjiqitujolwxqo64tezkkfgg35d45fi",
        "Rust-computed CID does not match the TypeScript seeder's CID — \
         the dag-cbor encoding diverges. Check field order, CID-as-Link encoding, \
         issuedAt format, or payload bytes."
    );
}
