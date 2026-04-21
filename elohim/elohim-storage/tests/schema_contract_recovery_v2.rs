//! Schema contract tests for Recovery Protocol Phase 2 view types.
//!
//! Verifies that Rust view types serialize to JSON matching the published
//! wire-contract schemas in elohim/sdk/schemas/v1/views/.
//!
//! Uses the same harness as schema_contract.rs — load_schema inlines $ref
//! before compiling because the jsonschema crate does not resolve file-based
//! references automatically.

use elohim_storage::views::{KeyRotationView, RecoveryQuorumRequestView, RecoverySeedCommitmentView};
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
fn recovery_seed_commitment_view_matches_schema() {
    let view = RecoverySeedCommitmentView {
        dht_anchor_hash: "abcd1234".to_string(),
        human_agent_pubkey: "uhCAk...".to_string(),
        seed_public_half: vec![0u8; 32],
        threshold_n: 3,
        total_m: 5,
        commitment_nonce: vec![0u8; 16],
        superseded_by: None,
        created_at: "2026-04-21T00:00:00Z".to_string(),
    };
    let json = serde_json::to_value(&view).expect("RecoverySeedCommitmentView serializes");
    validate_against_schema("views/recovery-seed-commitment.schema.json", &json);
}

#[test]
fn recovery_quorum_request_view_matches_schema() {
    let view = RecoveryQuorumRequestView {
        dht_anchor_hash: "req0001".to_string(),
        human_agent_pubkey: "uhCAk...".to_string(),
        seed_commitment_hash: "commit001".to_string(),
        new_agent_pubkey: "uhCAk_new".to_string(),
        hosting_doorway_pubkey: "uhCAk_door".to_string(),
        recovery_mode: "normal".to_string(),
        stewarded_grant_hash: None,
        request_nonce: vec![0u8; 16],
        created_at: "2026-04-21T00:00:00Z".to_string(),
    };
    let json = serde_json::to_value(&view).expect("RecoveryQuorumRequestView serializes");
    validate_against_schema("views/recovery-quorum-request.schema.json", &json);
}

#[test]
fn key_rotation_view_matches_schema() {
    let view = KeyRotationView {
        dht_anchor_hash: "rot0001".to_string(),
        human_agent_pubkey: "uhCAk...".to_string(),
        new_agent_pubkey: "uhCAk_new".to_string(),
        superseded_agent_pubkey: "uhCAk_old".to_string(),
        seed_commitment_hash: "commit001".to_string(),
        recovery_request_hash: "req0001".to_string(),
        quorum_signature: vec![0u8; 64],
        rotated_at: "2026-04-21T00:00:00Z".to_string(),
    };
    let json = serde_json::to_value(&view).expect("KeyRotationView serializes");
    validate_against_schema("views/key-rotation.schema.json", &json);
}
