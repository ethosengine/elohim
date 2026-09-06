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

#[test]
fn authentic_content_is_repeatable_but_has_no_activation_authority() {
    let (content, pin, observed) = fixture();
    let epr = signed(&content, &pin);
    let result = authenticate_candidate(&epr, &pin, &observed).unwrap();
    assert_eq!(result.content(), &content);
    assert_eq!(result.cid(), epr.envelope.cid);
    let mut relabeled = observed.clone();
    relabeled.installed.version = "informational rename".into();
    assert!(authenticate_candidate(&epr, &pin, &relabeled).is_ok());
    assert_eq!(authenticate_candidate(&epr, &pin, &observed), Ok(result));
}

#[test]
fn tampering_and_independent_wrong_key_are_refused() {
    let (content, mut pin, observed) = fixture();
    let epr = signed(&content, &pin);
    let mut tampered = epr.clone();
    tampered.payload[0] ^= 1;
    assert_eq!(
        authenticate_candidate(&tampered, &pin, &observed),
        Err(CandidateRefusal::InvalidProof)
    );
    // Recomputing the digest after substitution cannot repair the signature.
    tampered.envelope.cid = compute_cid(
        &tampered
            .envelope
            .canonical_bytes(&tampered.payload)
            .unwrap(),
    );
    assert_eq!(
        authenticate_candidate(&tampered, &pin, &observed),
        Err(CandidateRefusal::InvalidProof)
    );
    pin.public_key = AgentKeypair::from_secret(&[24; 32])
        .unwrap()
        .public_key_bytes();
    assert_eq!(
        authenticate_candidate(&epr, &pin, &observed),
        Err(CandidateRefusal::InvalidProof)
    );
    pin.signer = compute_cid(b"other agent");
    assert_eq!(
        authenticate_candidate(&epr, &pin, &observed),
        Err(CandidateRefusal::WrongIssuer)
    );
}

#[test]
fn independent_target_and_installed_changes_refuse_stale_content() {
    let (content, pin, observed) = fixture();
    let epr = signed(&content, &pin);
    for changed in [
        CandidateObservation {
            candidate_manifest: compute_cid(b"other elected manifest"),
            ..observed.clone()
        },
        CandidateObservation {
            grant_action: format!("uhCkk{}", "F".repeat(48)),
            ..observed.clone()
        },
        CandidateObservation {
            incarnation: 4,
            ..observed.clone()
        },
        CandidateObservation {
            child: "storage".into(),
            ..observed.clone()
        },
        CandidateObservation {
            release_action: format!("uhCkk{}", "C".repeat(48)),
            ..observed.clone()
        },
        CandidateObservation {
            grant_entry: format!("uhCEk{}", "D".repeat(48)),
            ..observed.clone()
        },
        CandidateObservation {
            berth: compute_cid(b"other berth"),
            ..observed.clone()
        },
    ] {
        assert_eq!(
            authenticate_candidate(&epr, &pin, &changed),
            Err(CandidateRefusal::WrongTarget)
        );
    }
    let mut changed = observed.clone();
    changed.installed.manifest = compute_cid(b"a different incumbent");
    assert_eq!(
        authenticate_candidate(&epr, &pin, &changed),
        Err(CandidateRefusal::StalePredecessor)
    );
    changed = observed.clone();
    changed.required_capabilities.insert("not-provided".into());
    assert_eq!(
        authenticate_candidate(&epr, &pin, &changed),
        Err(CandidateRefusal::MissingCapability)
    );
    changed = observed.clone();
    changed.database_format = "hc-sqlite-next".into();
    assert_eq!(
        authenticate_candidate(&epr, &pin, &changed),
        Err(CandidateRefusal::DatabaseIncompatible)
    );
    changed = observed;
    changed.observed_executable = compute_cid(b"wrong independently observed bytes");
    assert_eq!(
        authenticate_candidate(&epr, &pin, &changed),
        Err(CandidateRefusal::ArtifactMismatch)
    );
}

#[test]
fn signed_incompatible_candidates_do_not_gain_compatibility_from_signature() {
    let (content, pin, observed) = fixture();
    let cases: Vec<(CandidateContent, CandidateRefusal)> = vec![
        (
            CandidateContent {
                candidate: CandidateRuntime {
                    platform: "aarch64-unknown-linux-gnu".into(),
                    ..content.candidate.clone()
                },
                ..content.clone()
            },
            CandidateRefusal::WrongPlatform,
        ),
        (
            CandidateContent {
                candidate: CandidateRuntime {
                    writes_format: "irreversible-new-format".into(),
                    ..content.candidate.clone()
                },
                ..content.clone()
            },
            CandidateRefusal::DatabaseIncompatible,
        ),
        (
            CandidateContent {
                candidate: CandidateRuntime {
                    reads_formats: set(&[]),
                    ..content.candidate.clone()
                },
                ..content.clone()
            },
            CandidateRefusal::DatabaseIncompatible,
        ),
        (
            CandidateContent {
                valid_until: observed.now,
                ..content.clone()
            },
            CandidateRefusal::InvalidTime,
        ),
        (
            CandidateContent {
                valid_from: observed.now + chrono::Duration::seconds(1),
                ..content.clone()
            },
            CandidateRefusal::InvalidTime,
        ),
        (
            CandidateContent {
                grant_entry: "caller-says-authorized".into(),
                ..content.clone()
            },
            CandidateRefusal::InvalidContent,
        ),
        (
            CandidateContent {
                required_capabilities: set(&["missing"]),
                ..content.clone()
            },
            CandidateRefusal::MissingCapability,
        ),
    ];
    for (candidate, reason) in cases {
        assert_eq!(
            authenticate_candidate(&signed(&candidate, &pin), &pin, &observed),
            Err(reason)
        );
    }
}

#[test]
fn unknown_domain_schema_fields_and_unbounded_inputs_are_refused() {
    let (content, pin, observed) = fixture();
    let epr = signed(&content, &pin);
    let mut changed = epr.clone();
    changed.envelope.schema_key = "ark-activation-authorized".into();
    assert_eq!(
        authenticate_candidate(&changed, &pin, &observed),
        Err(CandidateRefusal::InvalidContent)
    );
    changed = epr.clone();
    changed.envelope.schema_ref = compute_cid(b"other schema");
    assert_eq!(
        authenticate_candidate(&changed, &pin, &observed),
        Err(CandidateRefusal::InvalidContent)
    );
    changed = epr;
    changed.payload = vec![0; MAX_CANDIDATE_BYTES + 1];
    assert_eq!(
        authenticate_candidate(&changed, &pin, &observed),
        Err(CandidateRefusal::TooLarge)
    );
    let mut json = serde_json::to_value(&content).unwrap();
    json["verified"] = true.into();
    let unknown = Epr::builder()
        .kind(EprKind::Content)
        .schema_ref(pin.schema)
        .schema_key(CANDIDATE_SCHEMA_KEY)
        .reach(Reach::Private)
        .issued_at(content.valid_from)
        .payload(serde_json::to_vec(&json).unwrap())
        .sign(&AgentKeypair::from_secret(&[42; 32]).unwrap(), pin.signer)
        .unwrap();
    assert_eq!(
        authenticate_candidate(&unknown, &pin, &observed),
        Err(CandidateRefusal::InvalidContent)
    );
}

#[test]
fn refusal_labels_are_stable_and_distinct() {
    seam_contracts::assert_reason_labels_conformant::<CandidateRefusal>();
    seam_contracts::assert_reason_labels_stable::<CandidateRefusal>(&[
        "candidate_too_large",
        "candidate_invalid_content",
        "candidate_wrong_issuer",
        "candidate_invalid_proof",
        "candidate_invalid_time",
        "candidate_wrong_target",
        "candidate_stale_predecessor",
        "candidate_wrong_platform",
        "candidate_artifact_mismatch",
        "candidate_missing_capability",
        "candidate_database_incompatible",
    ]);
}
