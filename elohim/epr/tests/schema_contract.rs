//! Schema contract tests — validates that Rust serialization matches
//! the authoritative JSON schemas in elohim/sdk/schemas/v1/.
//!
//! These tests catch drift between Rust struct changes and the schema
//! contract (IoC chain layer 3). If a field is renamed, added, or removed
//! in Rust without updating the schema (or vice versa), these tests fail.
//!
//! Per the integrator compatibility contract §1, EPR atoms are self-notarized
//! via content-address (CIDv1 dag-cbor sha256) + Ed25519 signature.
//!
//! The harness inlines `$ref` before compiling because the jsonschema
//! crate doesn't resolve file-based references automatically.

use chrono::{TimeZone, Utc};
use elohim_epr::{
    cid::compute_cid,
    proof::AgentKeypair,
    Coupling, CouplingLeg, Epr, EprKind, Reach, Signature,
};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// ── Harness ──────────────────────────────────────────────────────

fn schema_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR = elohim/epr/
    // parent             = elohim/
    // join               = elohim/sdk/schemas/v1/
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("parent of epr/ must be elohim/")
        .join("sdk/schemas/v1")
}

/// Load all schemas from enums/ and objects/ into a ref map keyed by relative path.
fn load_ref_map() -> HashMap<String, Value> {
    let base = schema_dir();
    let mut refs = HashMap::new();

    for subdir in &["enums", "objects"] {
        let dir = base.join(subdir);
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(schema) = serde_json::from_str::<Value>(&content) {
                            let filename =
                                path.file_name().unwrap().to_str().unwrap().to_string();
                            // Same-dir ref: "signature.schema.json"
                            refs.insert(filename.clone(), schema.clone());
                            // Same-dir ref with ./ prefix: "./signature.schema.json"
                            refs.insert(format!("./{}", filename), schema.clone());
                            // Cross-dir from objects/ to enums/: "../enums/epr-kind.schema.json"
                            refs.insert(format!("../{}/{}", subdir, filename), schema.clone());
                            // Qualified: "enums/epr-kind.schema.json"
                            refs.insert(format!("{}/{}", subdir, filename), schema);
                        }
                    }
                }
            }
        }
    }

    refs
}

/// Recursively inline `$ref` in a schema value using the ref map.
fn inline_refs(schema: &Value, refs: &HashMap<String, Value>) -> Value {
    match schema {
        Value::Object(map) => {
            if let Some(Value::String(ref_path)) = map.get("$ref") {
                if !ref_path.starts_with('#') {
                    if let Some(referenced) = refs.get(ref_path) {
                        let mut inlined = referenced.clone();
                        if let Value::Object(ref mut obj) = inlined {
                            obj.remove("$id");
                            obj.remove("$schema");
                        }
                        return inline_refs(&inlined, refs);
                    }
                }
            }
            let mut result = serde_json::Map::new();
            for (key, value) in map {
                result.insert(key.clone(), inline_refs(value, refs));
            }
            Value::Object(result)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(|v| inline_refs(v, refs)).collect()),
        other => other.clone(),
    }
}

/// Load a schema relative to the sdk/schemas/v1 root, inline all $ref.
fn load_schema(relative: &str) -> Value {
    let path = schema_dir().join(relative);
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read schema {}: {}", path.display(), e));
    let raw: Value = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse schema {}: {}", path.display(), e));
    let refs = load_ref_map();
    inline_refs(&raw, &refs)
}

/// Validate a JSON instance against a schema. Panic with a clear message on failure.
fn validate(schema_path: &str, instance: &Value) {
    let schema = load_schema(schema_path);
    let validator = jsonschema::validator_for(&schema)
        .unwrap_or_else(|e| panic!("Failed to compile schema {}: {}", schema_path, e));
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| format!("  - {} (at {})", e, e.instance_path))
        .collect();
    if !errors.is_empty() {
        panic!(
            "Schema validation failed for {}:\n{}\n\nInstance:\n{}",
            schema_path,
            errors.join("\n"),
            serde_json::to_string_pretty(instance).unwrap()
        );
    }
}

/// Convention: every schema must declare source of truth in description.
fn assert_source_of_truth_declared(schema_path: &str) {
    let path = schema_dir().join(schema_path);
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", schema_path, e));
    let schema_value: Value = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", schema_path, e));
    let desc = schema_value["description"]
        .as_str()
        .unwrap_or_else(|| panic!("Schema {} missing description", schema_path));
    assert!(
        desc.contains("Source of truth:"),
        "Schema {} description must contain 'Source of truth:' — got: {}",
        schema_path,
        desc
    );
}

// ── Convention enforcement ────────────────────────────────────────

#[test]
fn epr_schemas_declare_source_of_truth() {
    let schemas = [
        "enums/reach.schema.json",
        "enums/epr-kind.schema.json",
        "enums/coupling-leg.schema.json",
        "objects/signature.schema.json",
        "objects/coupling.schema.json",
        "objects/envelope.schema.json",
        "objects/epr.schema.json",
    ];
    for schema_path in &schemas {
        assert_source_of_truth_declared(schema_path);
    }
}

// ── Enum tests ────────────────────────────────────────────────────

#[test]
fn reach_values_conform() {
    for r in [
        Reach::Private,
        Reach::SelfScope,
        Reach::Intimate,
        Reach::Trusted,
        Reach::Familiar,
        Reach::Community,
        Reach::Public,
        Reach::Commons,
    ] {
        validate(
            "enums/reach.schema.json",
            &serde_json::to_value(r).unwrap(),
        );
    }
}

#[test]
fn epr_kind_values_conform() {
    for k in [
        EprKind::Content,
        EprKind::Agent,
        EprKind::Manifest,
        EprKind::Claim,
        EprKind::Observation,
        EprKind::EconomicEvent,
        EprKind::Commitment,
        EprKind::Attestation,
        EprKind::Delegation,
    ] {
        validate(
            "enums/epr-kind.schema.json",
            &serde_json::to_value(k).unwrap(),
        );
    }
}

#[test]
fn coupling_leg_values_conform() {
    for leg in [
        CouplingLeg::Knowledge,
        CouplingLeg::Value,
        CouplingLeg::Governance,
    ] {
        validate(
            "enums/coupling-leg.schema.json",
            &serde_json::to_value(leg).unwrap(),
        );
    }
}

// ── Object tests ──────────────────────────────────────────────────

#[test]
fn signature_conforms() {
    let signer_cid = compute_cid(&[1u8]);
    let sig = Signature::ed25519(signer_cid, vec![0u8; 64]);
    validate(
        "objects/signature.schema.json",
        &serde_json::to_value(&sig).unwrap(),
    );
}

#[test]
fn coupling_empty_conforms() {
    let c = Coupling::default();
    validate(
        "objects/coupling.schema.json",
        &serde_json::to_value(&c).unwrap(),
    );
}

#[test]
fn coupling_knowledge_only_conforms() {
    let c = Coupling {
        knowledge: Some(compute_cid(&[1u8])),
        ..Default::default()
    };
    validate(
        "objects/coupling.schema.json",
        &serde_json::to_value(&c).unwrap(),
    );
}

#[test]
fn coupling_all_legs_conforms() {
    let c = Coupling {
        knowledge: Some(compute_cid(&[1u8])),
        value: Some(compute_cid(&[2u8])),
        governance: Some(compute_cid(&[3u8])),
    };
    validate(
        "objects/coupling.schema.json",
        &serde_json::to_value(&c).unwrap(),
    );
}

#[test]
fn envelope_conforms() {
    let kp = AgentKeypair::from_secret(&[7u8; 32]).unwrap();
    let agent_cid = compute_cid(&[100u8]);

    let epr = Epr::builder()
        .kind(EprKind::Content)
        .schema_ref(compute_cid(&[1u8]))
        .schema_key("concept")
        .reach(Reach::Commons)
        .coupling(Coupling {
            knowledge: Some(compute_cid(&[2u8])),
            value: Some(compute_cid(&[3u8])),
            governance: Some(compute_cid(&[4u8])),
        })
        .claim(compute_cid(&[5u8]))
        .issued_at(Utc.with_ymd_and_hms(2026, 4, 21, 12, 0, 0).unwrap())
        .payload(b"hello".to_vec())
        .sign(&kp, agent_cid)
        .expect("sign");

    validate(
        "objects/envelope.schema.json",
        &serde_json::to_value(&epr.envelope).unwrap(),
    );
}

#[test]
fn envelope_with_supersedes_conforms() {
    let kp = AgentKeypair::from_secret(&[8u8; 32]).unwrap();
    let agent_cid = compute_cid(&[100u8]);

    let epr = Epr::builder()
        .kind(EprKind::Content)
        .schema_ref(compute_cid(&[1u8]))
        .schema_key("concept")
        .reach(Reach::Trusted)
        .coupling(Coupling {
            knowledge: Some(compute_cid(&[2u8])),
            value: Some(compute_cid(&[3u8])),
            governance: Some(compute_cid(&[4u8])),
        })
        .claim(compute_cid(&[5u8]))
        .supersedes(compute_cid(&[99u8]))
        .issued_at(Utc.with_ymd_and_hms(2026, 4, 21, 13, 0, 0).unwrap())
        .payload(b"revised".to_vec())
        .sign(&kp, agent_cid)
        .expect("sign with supersedes");

    validate(
        "objects/envelope.schema.json",
        &serde_json::to_value(&epr.envelope).unwrap(),
    );
}

#[test]
fn epr_conforms() {
    let kp = AgentKeypair::from_secret(&[9u8; 32]).unwrap();
    let agent_cid = compute_cid(&[100u8]);

    let epr = Epr::builder()
        .kind(EprKind::Manifest)
        .schema_ref(compute_cid(&[1u8]))
        .schema_key("app-manifest")
        .reach(Reach::Commons)
        .coupling(Coupling {
            governance: Some(compute_cid(&[4u8])),
            ..Default::default()
        })
        .claim(compute_cid(&[5u8]))
        .issued_at(Utc.with_ymd_and_hms(2026, 4, 21, 12, 0, 0).unwrap())
        .payload(b"{}".to_vec())
        .sign(&kp, agent_cid)
        .expect("sign");

    validate(
        "objects/epr.schema.json",
        &serde_json::to_value(&epr).unwrap(),
    );
}

#[test]
fn epr_all_kinds_envelope_conform() {
    // Ensure every EprKind serializes a valid envelope.
    let kp = AgentKeypair::from_secret(&[42u8; 32]).unwrap();
    let agent_cid = compute_cid(&[100u8]);
    let issued_at = Utc.with_ymd_and_hms(2026, 4, 21, 12, 0, 0).unwrap();

    let cases: &[(EprKind, Coupling)] = &[
        (
            EprKind::Content,
            Coupling {
                knowledge: Some(compute_cid(&[2u8])),
                value: Some(compute_cid(&[3u8])),
                governance: Some(compute_cid(&[4u8])),
            },
        ),
        (
            EprKind::Agent,
            Coupling {
                governance: Some(compute_cid(&[4u8])),
                ..Default::default()
            },
        ),
        (
            EprKind::Manifest,
            Coupling {
                governance: Some(compute_cid(&[4u8])),
                ..Default::default()
            },
        ),
        (
            EprKind::Claim,
            Coupling {
                knowledge: Some(compute_cid(&[2u8])),
                ..Default::default()
            },
        ),
        (
            EprKind::Observation,
            Coupling {
                knowledge: Some(compute_cid(&[2u8])),
                ..Default::default()
            },
        ),
        (
            EprKind::EconomicEvent,
            Coupling {
                value: Some(compute_cid(&[3u8])),
                ..Default::default()
            },
        ),
        (
            EprKind::Commitment,
            Coupling {
                value: Some(compute_cid(&[3u8])),
                governance: Some(compute_cid(&[4u8])),
                ..Default::default()
            },
        ),
        (
            EprKind::Attestation,
            Coupling {
                governance: Some(compute_cid(&[4u8])),
                ..Default::default()
            },
        ),
        (
            EprKind::Delegation,
            Coupling {
                governance: Some(compute_cid(&[4u8])),
                ..Default::default()
            },
        ),
    ];

    for (kind, coupling) in cases {
        let epr = Epr::builder()
            .kind(*kind)
            .schema_ref(compute_cid(&[1u8]))
            .schema_key("test")
            .reach(Reach::Commons)
            .coupling(coupling.clone())
            .claim(compute_cid(&[5u8]))
            .issued_at(issued_at)
            .payload(b"test".to_vec())
            .sign(&kp, agent_cid)
            .unwrap_or_else(|e| panic!("sign {:?}: {}", kind, e));

        validate(
            "objects/envelope.schema.json",
            &serde_json::to_value(&epr.envelope).unwrap(),
        );
    }
}
