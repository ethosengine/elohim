//! Schema contract tests — validates that Rust serialization matches
//! the JSON Schema source of truth in elohim/sdk/schemas/v1/views/.
//!
//! These tests catch drift between Rust struct changes and the schema
//! contract. If a field is renamed, added, or removed in Rust without
//! updating the schema (or vice versa), these tests fail.
//!
//! The harness inlines `$ref` before compiling because the jsonschema
//! crate doesn't resolve file-based references automatically.

use elohim_storage::p2p::replication::ReplicationStatus;
use elohim_storage::p2p::DrainStatusInfo;
use elohim_storage::P2PStatusInfo;
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
                            // Same-dir ref: "drain-status-view.schema.json"
                            refs.insert(filename.clone(), schema.clone());
                            // Cross-dir ref: "../enums/nat-status.schema.json"
                            refs.insert(format!("../{}/{}", subdir, filename), schema.clone());
                            // From views/ to views/: "replication-status-view.schema.json"
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
            // If this object has a $ref, replace with the referenced schema
            if let Some(Value::String(ref_path)) = map.get("$ref") {
                if !ref_path.starts_with('#') {
                    if let Some(referenced) = refs.get(ref_path) {
                        // Inline: take the referenced schema, strip meta fields, inline its refs too
                        let mut inlined = referenced.clone();
                        if let Value::Object(ref mut obj) = inlined {
                            obj.remove("$id");
                            obj.remove("$schema");
                        }
                        return inline_refs(&inlined, refs);
                    }
                }
            }

            // Recurse into all values
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

/// Convention: every view schema must declare source of truth in description.
fn assert_source_of_truth_declared(schema_value: &Value, schema_name: &str) {
    let desc = schema_value["description"]
        .as_str()
        .unwrap_or_else(|| panic!("Schema {} missing description", schema_name));
    assert!(
        desc.contains("Source of truth:"),
        "Schema {} description must contain 'Source of truth:' — got: {}",
        schema_name,
        desc
    );
}

// ── P2P Status ──────────────────────────────────────────────────

#[test]
fn p2p_status_view_matches_schema() {
    let status = P2PStatusInfo {
        peer_id: "12D3KooWTest".to_string(),
        listen_addresses: vec!["/ip4/127.0.0.1/tcp/4001".to_string()],
        connected_peers: 3,
        bootstrap_nodes: vec!["/dnsaddr/bootstrap.example.com".to_string()],
        sync_documents: 5,
        nat_status: "public".to_string(),
        relay_reservations: 1,
        announce_addresses: vec!["/ip4/1.2.3.4/tcp/4001".to_string()],
        relay_mode: "client".to_string(),
        replication: ReplicationStatus {
            pending: 2,
            completed: 10,
            failed: 0,
            caught_up: false,
        },
        drain: Some(DrainStatusInfo {
            total: 100,
            published: 95,
            pending: 5,
        }),
        sync_paused: false,
    };

    let json = serde_json::to_value(&status).unwrap();
    validate_against_schema("views/p2p-status-view.schema.json", &json);
}

#[test]
fn p2p_status_view_with_null_drain() {
    let status = P2PStatusInfo {
        peer_id: "12D3KooWTest".to_string(),
        listen_addresses: vec![],
        connected_peers: 0,
        bootstrap_nodes: vec![],
        sync_documents: 0,
        nat_status: "unknown".to_string(),
        relay_reservations: 0,
        announce_addresses: vec![],
        relay_mode: "disabled".to_string(),
        replication: ReplicationStatus::default(),
        drain: None,
        sync_paused: true,
    };

    let json = serde_json::to_value(&status).unwrap();
    validate_against_schema("views/p2p-status-view.schema.json", &json);
}

// ── Sub-views ───────────────────────────────────────────────────

#[test]
fn drain_status_view_matches_schema() {
    let drain = DrainStatusInfo {
        total: 100,
        published: 95,
        pending: 5,
    };

    let json = serde_json::to_value(&drain).unwrap();
    validate_against_schema("views/drain-status-view.schema.json", &json);
}

#[test]
fn replication_status_view_matches_schema() {
    let replication = ReplicationStatus {
        pending: 10,
        completed: 50,
        failed: 2,
        caught_up: false,
    };

    let json = serde_json::to_value(&replication).unwrap();
    validate_against_schema("views/replication-status-view.schema.json", &json);
}

// ── Peer views ──────────────────────────────────────────────────

#[cfg(feature = "p2p")]
#[test]
fn peer_info_view_matches_schema() {
    use elohim_storage::PeerInfoView;

    let peer = PeerInfoView {
        peer_id: "12D3KooWPeer1".to_string(),
        multiaddrs: vec!["/ip4/192.168.1.1/tcp/4001".to_string()],
        protocols: vec!["/elohim/shard/1.0.0".to_string()],
        agent_version: "elohim-storage/0.1.0".to_string(),
        direction: "outbound".to_string(),
        rtt_ms: None,
        last_seen_ms: None,
        remote_nat_status: None,
        bandwidth_in: None,
        bandwidth_out: None,
    };

    let json = serde_json::to_value(&peer).unwrap();
    validate_against_schema("views/peer-info-view.schema.json", &json);
}

#[cfg(feature = "p2p")]
#[test]
fn peer_list_view_matches_schema() {
    use elohim_storage::{PeerInfoView, PeerListView};

    let list = PeerListView {
        peers: vec![PeerInfoView {
            peer_id: "12D3KooWPeer1".to_string(),
            multiaddrs: vec![],
            protocols: vec![],
            agent_version: "test".to_string(),
            direction: "inbound".to_string(),
            rtt_ms: Some(42.5),
            last_seen_ms: Some(1712678400000),
            remote_nat_status: Some("public".to_string()),
            bandwidth_in: Some(1024),
            bandwidth_out: Some(2048),
        }],
        total: 1,
    };

    let json = serde_json::to_value(&list).unwrap();
    validate_against_schema("views/peer-list-view.schema.json", &json);
}

// ── Convention enforcement ──────────────────────────────────────

#[test]
fn view_schemas_declare_source_of_truth() {
    let view_schemas = [
        "views/p2p-status-view.schema.json",
        "views/drain-status-view.schema.json",
        "views/replication-status-view.schema.json",
        "views/peer-info-view.schema.json",
        "views/peer-list-view.schema.json",
        "views/content-view.schema.json",
        "views/economic-event-view.schema.json",
    ];

    for schema_name in &view_schemas {
        let path = schema_dir().join(schema_name);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", schema_name, e));
        let schema_value: Value = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse {}: {}", schema_name, e));
        assert_source_of_truth_declared(&schema_value, schema_name);
    }
}
