use std::collections::BTreeSet;

use ark_core::candidate::*;
use chrono::{TimeZone, Utc};
use cid::Cid;
use elohim_epr::{cid::compute_cid, AgentKeypair, Epr, EprKind, Reach};

fn set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|s| (*s).to_owned()).collect()
}

fn fixture() -> (CandidateContent, CandidatePin, CandidateObservation) {
    // Raw CID parsed from an external, fixed empty-blob fixture, not the verifier.
    let raw: Cid = "bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku"
        .parse()
        .unwrap();
    let runtime = CandidateRuntime {
        manifest: compute_cid(b"incumbent manifest"),
        executable: raw,
        version: "0.7.0".into(),
        platform: "x86_64-unknown-linux-gnu".into(),
        writes_format: "hc-sqlite-0.7".into(),
        reads_formats: set(&["hc-sqlite-0.7"]),
        capabilities: set(&["iroh", "admin-v0.7"]),
    };
    let now = Utc.timestamp_opt(1_800_000_000, 0).unwrap();
    let content = CandidateContent {
        channel: "runtime:conductor-binary:elohim:household".into(),
        release_action: format!("uhCkk{}", "A".repeat(48)),
        grant_entry: format!("uhCEk{}", "B".repeat(48)),
        grant_action: format!("uhCkk{}", "E".repeat(48)),
        berth: compute_cid(b"berth"),
        incarnation: 3,
        child: "conductor".into(),
        predecessor: runtime.clone(),
        candidate: CandidateRuntime {
            manifest: compute_cid(b"candidate manifest"),
            version: "label-does-not-prove-compatibility".into(),
            ..runtime.clone()
        },
        required_capabilities: set(&["iroh"]),
        valid_from: now - chrono::Duration::seconds(30),
        valid_until: now + chrono::Duration::seconds(30),
    };
    let key = AgentKeypair::from_secret(&[42; 32]).unwrap();
    let pin = CandidatePin {
        signer: compute_cid(b"issuer"),
        public_key: key.public_key_bytes(),
        schema: compute_cid(b"independently provisioned test schema"),
    };
    let observed = CandidateObservation {
        channel: content.channel.clone(),
        release_action: content.release_action.clone(),
        grant_entry: content.grant_entry.clone(),
        grant_action: content.grant_action.clone(),
        berth: content.berth,
        incarnation: 3,
        child: "conductor".into(),
        installed: runtime,
        database_format: "hc-sqlite-0.7".into(),
        required_capabilities: set(&["admin-v0.7"]),
        observed_executable: raw,
        candidate_manifest: content.candidate.manifest,
        now,
    };
    (content, pin, observed)
}

fn signed(content: &CandidateContent, pin: &CandidatePin) -> Epr {
    Epr::builder()
        .kind(EprKind::Content)
        .schema_ref(pin.schema)
        .schema_key(CANDIDATE_SCHEMA_KEY)
        .reach(Reach::Private)
        .issued_at(content.valid_from)
        .payload(serde_json::to_vec(content).unwrap())
        .sign(&AgentKeypair::from_secret(&[42; 32]).unwrap(), pin.signer)
        .unwrap()
}

pub fn authenticated() -> ark_core::candidate::AuthenticatedCandidateContent {
    let (content, pin, observed) = fixture();
    authenticate_candidate(&signed(&content, &pin), &pin, &observed).unwrap()
}
