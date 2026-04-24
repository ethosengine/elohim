//! Schema contract tests for Recovery Protocol Phase 2 view types.
//!
//! Verifies that Rust view types serialize to JSON matching the published
//! wire-contract schemas in elohim/sdk/schemas/v1/views/.
//!
//! Uses the same harness as schema_contract.rs — load_schema inlines $ref
//! before compiling because the jsonschema crate does not resolve file-based
//! references automatically.

use elohim_storage::views::{KeyRotationView, RecoveryRequestView, RecoveryWitnessView};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Resolve a schema file relative to the repo root.
fn schema_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("../sdk/schemas/v1")
}

/// Load all schemas from enums/ and views/ into a ref map keyed by relative path.
fn load_ref_map() -> HashMap<String, Value> {
    let base = schema_dir();
    let mut refs = HashMap::new();

    for subdir in &["enums", "views"] {
        let dir = base.join(subdir);
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(schema) = serde_json::from_str::<Value>(&content) {
                            let filename = path.file_name().unwrap().to_str().unwrap().to_string();
                            refs.insert(filename.clone(), schema.clone());
                            refs.insert(format!("../{}/{}", subdir, filename), schema.clone());
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

/// Load a schema, inline all $ref, and return the resolved value.
fn load_schema(relative: &str) -> Value {
    let path = schema_dir().join(relative);
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read schema {}: {}", path.display(), e));
    let raw: Value = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse schema {}: {}", path.display(), e));

    let refs = load_ref_map();
    inline_refs(&raw, &refs)
}

/// Validate a serialized Rust struct against a schema.
fn validate_against_schema(schema_path_str: &str, instance: &Value) {
    let schema = load_schema(schema_path_str);
    let validator = jsonschema::validator_for(&schema)
        .unwrap_or_else(|e| panic!("Failed to compile schema {}: {}", schema_path_str, e));
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| format!("  - {} (at {})", e, e.instance_path))
        .collect();
    if !errors.is_empty() {
        panic!(
            "Schema validation failed for {}:\n{}\n\nInstance:\n{}",
            schema_path_str,
            errors.join("\n"),
            serde_json::to_string_pretty(instance).unwrap()
        );
    }
}

// ── Recovery Phase 2 Contract Tests ─────────────────────────────────────────

#[test]
fn recovery_request_view_matches_schema() {
    let view = RecoveryRequestView {
        dht_anchor_hash: "req001".to_string(),
        human_agent_pubkey: "uhCAk_human".to_string(),
        new_agent_pubkey: "uhCAk_new".to_string(),
        hosting_doorway_pubkey: "uhCAk_doorway".to_string(),
        proposed_authority_kind: "intimateQuorum".to_string(),
        proposed_authority_json: "{}".to_string(),
        request_nonce: vec![0u8; 16],
        human_id: Some("human-123".to_string()),
        required_witness_count: 2,
        created_at: "2026-04-22T00:00:00Z".to_string(),
    };
    let json = serde_json::to_value(&view).expect("serializes");
    validate_against_schema("views/recovery-request.schema.json", &json);
}

#[test]
fn recovery_request_view_matches_schema_null_human_id() {
    // human_id is explicitly optional at request-commit time (back-compat).
    // requiredWitnessCount is required in the schema (must be present even when human_id is None).
    let view = RecoveryRequestView {
        dht_anchor_hash: "req002".to_string(),
        human_agent_pubkey: "uhCAk_human2".to_string(),
        new_agent_pubkey: "uhCAk_new2".to_string(),
        hosting_doorway_pubkey: "uhCAk_doorway2".to_string(),
        proposed_authority_kind: "cryptographicQuorum".to_string(),
        proposed_authority_json: "{\"stewardshipHash\":\"abc\"}".to_string(),
        request_nonce: vec![0u8; 16],
        human_id: None,
        required_witness_count: 2,
        created_at: "2026-04-22T00:00:00Z".to_string(),
    };
    let json = serde_json::to_value(&view).expect("serializes");
    validate_against_schema("views/recovery-request.schema.json", &json);
}

#[test]
fn key_rotation_view_matches_schema() {
    let view = KeyRotationView {
        dht_anchor_hash: "rot001".to_string(),
        human_agent_pubkey: "uhCAk_human".to_string(),
        new_agent_pubkey: "uhCAk_new".to_string(),
        superseded_agent_pubkey: "uhCAk_old".to_string(),
        recovery_request_hash: "req001".to_string(),
        authority_kind: "intimateQuorum".to_string(),
        authority_json: "{\"witnessHashes\":[]}".to_string(),
        rotated_at: "2026-04-22T00:00:00Z".to_string(),
    };
    let json = serde_json::to_value(&view).expect("serializes");
    validate_against_schema("views/key-rotation.schema.json", &json);
}

#[test]
fn recovery_witness_view_matches_schema() {
    let view = RecoveryWitnessView {
        dht_anchor_hash: "wit001".to_string(),
        recovery_request_hash: "req001".to_string(),
        witness_agent_id: "uhCAk_witness".to_string(),
        human_id: "human-123".to_string(),
        note: Some("recognized the voice".to_string()),
        submitted_at: "2026-04-24T00:00:00Z".to_string(),
    };
    let json = serde_json::to_value(&view).expect("serializes");
    validate_against_schema("views/recovery-witness.schema.json", &json);
}

#[test]
fn recovery_witness_view_matches_schema_null_note() {
    // note is optional — verify null variant validates.
    let view = RecoveryWitnessView {
        dht_anchor_hash: "wit002".to_string(),
        recovery_request_hash: "req002".to_string(),
        witness_agent_id: "uhCAk_witness2".to_string(),
        human_id: "human-456".to_string(),
        note: None,
        submitted_at: "2026-04-24T12:00:00Z".to_string(),
    };
    let json = serde_json::to_value(&view).expect("serializes");
    validate_against_schema("views/recovery-witness.schema.json", &json);
}
