//! Generates shared test vectors for Rust ↔ TS interop verification.
//!
//! Run: cargo run -p elohim-epr --example gen_vectors
//! Writes JSON files to elohim/epr/tests/vectors/.

use chrono::{TimeZone, Utc};
use cid::Cid;
use elohim_epr::{cid::compute_cid, proof::AgentKeypair, Coupling, Epr, EprKind, Reach};
use serde::Serialize;
use std::{fs, path::PathBuf};

fn cid(b: u8) -> Cid {
    compute_cid(&[b])
}

#[derive(Serialize)]
struct KeypairVector {
    seed_hex: String,
    secret_hex: String,
    public_hex: String,
}

#[derive(Serialize)]
struct EprVector {
    label: String,
    envelope: serde_json::Value,
    payload_hex: String,
    canonical_bytes_hex: String,
    cid: String,
    signature_hex: String,
    public_key_hex: String,
}

fn main() {
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/vectors");
    fs::create_dir_all(&out_dir).expect("create vectors dir");

    // Deterministic seed for reproducibility (RFC 8032 Test 1 secret)
    let seed = [
        0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c,
        0xc4, 0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae,
        0x7f, 0x60,
    ];
    let kp = AgentKeypair::from_secret(&seed).unwrap();
    let keypair_vec = KeypairVector {
        seed_hex: hex::encode(seed),
        secret_hex: hex::encode(kp.secret_bytes()),
        public_hex: hex::encode(kp.public_key_bytes()),
    };
    fs::write(
        out_dir.join("keypairs.json"),
        serde_json::to_string_pretty(&keypair_vec).unwrap(),
    )
    .unwrap();

    // Representative vectors across kinds that make sense in isolation
    let agent_cid = cid(100);
    let vectors: Vec<(&str, EprKind, Coupling, Vec<u8>)> = vec![
        (
            "content_all_legs",
            EprKind::Content,
            Coupling {
                knowledge: Some(cid(2)),
                value: Some(cid(3)),
                governance: Some(cid(4)),
            },
            b"hello world".to_vec(),
        ),
        (
            "manifest_gov_only",
            EprKind::Manifest,
            Coupling {
                knowledge: None,
                value: None,
                governance: Some(cid(4)),
            },
            b"{}".to_vec(),
        ),
        (
            "economic_event_value",
            EprKind::EconomicEvent,
            Coupling {
                knowledge: None,
                value: Some(cid(3)),
                governance: None,
            },
            br#"{"action":"use"}"#.to_vec(),
        ),
    ];

    let issued_at = Utc.with_ymd_and_hms(2026, 4, 21, 12, 0, 0).unwrap();
    let epr_vectors: Vec<EprVector> = vectors
        .into_iter()
        .map(|(label, kind, coupling, payload)| {
            let epr = Epr::builder()
                .kind(kind)
                .schema_ref(cid(1))
                .schema_key("test")
                .reach(Reach::Commons)
                .coupling(coupling)
                .claim(cid(5))
                .issued_at(issued_at)
                .payload(payload.clone())
                .sign(&kp, agent_cid)
                .unwrap();

            let canonical = epr.envelope.canonical_bytes(&payload).unwrap();

            EprVector {
                label: label.into(),
                envelope: serde_json::to_value(&epr.envelope).unwrap(),
                payload_hex: hex::encode(&payload),
                canonical_bytes_hex: hex::encode(&canonical),
                cid: epr.envelope.cid.to_string(),
                signature_hex: hex::encode(&epr.envelope.proof.signature),
                public_key_hex: hex::encode(kp.public_key_bytes()),
            }
        })
        .collect();

    fs::write(
        out_dir.join("signed_eprs.json"),
        serde_json::to_string_pretty(&epr_vectors).unwrap(),
    )
    .unwrap();

    println!("wrote vectors to {}", out_dir.display());
}
