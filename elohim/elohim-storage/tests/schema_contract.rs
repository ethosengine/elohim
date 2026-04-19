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

// ── Gate Decision Attestation ───────────────────────────────────

#[test]
fn gate_decision_attestation_view_matches_schema() {
    use elohim_storage::GateDecisionAttestationView;

    let view = GateDecisionAttestationView {
        decision_id: "bafyreib2vq7fztfnmgzrmo7q5jnfkdvxkfxpvsjmesxrqjzqxkzqzqzqa".to_string(),
        phase: "elohim-active".to_string(),
        elohim_id: "uhCAkABCDEFGHIJKLMNOPQRSTUVWXYZ012345678901234567890123456789012".to_string(),
        elohim_substance_cid: "bafyreib2vq7substance01234567890123456789012345678901234567"
            .to_string(),
        gate_name: "discernment-gate-v1-mechanical".to_string(),
        gate_process_cid: "bafyreib2vq7process01234567890123456789012345678901234567890"
            .to_string(),
        request_ref_json: r#"{"eventId":"ev-001","agentId":"uhCAkABC"}"#.to_string(),
        decision: "allow".to_string(),
        reasoning_json: r#"{"steps":[],"verdict":"allow","constitutional_basis":[]}"#.to_string(),
        context_summary_cid: "bafyreib2vq7context01234567890123456789012345678901234567890"
            .to_string(),
        decided_at: "2026-04-18T12:00:00Z".to_string(),
        universal_band_cid: "bafyreib2vq7univband01234567890123456789012345678901234567"
            .to_string(),
        dht_anchor_hash: "uhCkkDEFGHIJKLMNOPQRSTUVWXYZ0123456789012345678901234567890123"
            .to_string(),
        created_at: "2026-04-18T12:00:01Z".to_string(),
    };

    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/gate-decision-attestation-view.schema.json", &json);
}

#[test]
fn gate_decision_attestation_view_dev_context_phase() {
    use elohim_storage::GateDecisionAttestationView;

    let view = GateDecisionAttestationView {
        decision_id: "bafyreib2vq7decision-dev-context-01234567890123456789012345".to_string(),
        phase: "dev-context".to_string(),
        elohim_id: "uhCAkDEV01234567890123456789012345678901234567890123456789012".to_string(),
        elohim_substance_cid: "bafyreib2vq7substance-dev-0123456789012345678901234567890"
            .to_string(),
        gate_name: "discernment-gate-v1-mechanical".to_string(),
        gate_process_cid: "bafyreib2vq7process-dev-01234567890123456789012345678901234".to_string(),
        request_ref_json: r#"{"eventId":"ev-dev-001"}"#.to_string(),
        decision: "decline".to_string(),
        reasoning_json: r#"{"steps":[],"verdict":"decline"}"#.to_string(),
        context_summary_cid: "bafyreib2vq7ctx-dev-01234567890123456789012345678901234567"
            .to_string(),
        decided_at: "2026-04-18T08:30:00Z".to_string(),
        universal_band_cid: "bafyreib2vq7band-dev-0123456789012345678901234567890123456"
            .to_string(),
        dht_anchor_hash: "uhCkkDEV01234567890123456789012345678901234567890123456789012"
            .to_string(),
        created_at: "2026-04-18T08:30:01Z".to_string(),
    };

    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/gate-decision-attestation-view.schema.json", &json);
}

// ── Gate Decision Challenge ─────────────────────────────────────

#[test]
fn gate_decision_challenge_view_matches_schema() {
    use elohim_storage::GateDecisionChallengeView;

    let view = GateDecisionChallengeView {
        challenge_id: "bafyreib2vq7challenge01234567890123456789012345678901234567".to_string(),
        challenged_decision_cid: "bafyreib2vq7decision0123456789012345678901234567890123456"
            .to_string(),
        challenger_id: "uhCAkCHALLENGERABCDEFGHIJKLMNOPQRSTUVWXYZ01234567890123456789012"
            .to_string(),
        grounds: "constitutional".to_string(),
        summary: "Decision appears to violate the P4 principle of graduated capability".to_string(),
        evidence_refs: "bafyreib2vq7evidence1,bafyreib2vq7evidence2".to_string(),
        filed_at: "2026-04-19T10:00:00Z".to_string(),
        reach: "community".to_string(),
        dht_anchor_hash: "uhCkkCHALANCHOR01234567890123456789012345678901234567890123456"
            .to_string(),
        created_at: "2026-04-19T10:00:01Z".to_string(),
    };

    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/gate-decision-challenge-view.schema.json", &json);
}

#[test]
fn gate_decision_challenge_view_empty_evidence_refs() {
    use elohim_storage::GateDecisionChallengeView;

    let view = GateDecisionChallengeView {
        challenge_id: "bafyreib2vq7challenge-safety-01234567890123456789012345678".to_string(),
        challenged_decision_cid: "bafyreib2vq7decision-safety-0123456789012345678901234567"
            .to_string(),
        challenger_id: "uhCAkCHALLENGER2ABCDEFGHIJKLMNOPQRSTUVWXYZ01234567890123456789"
            .to_string(),
        grounds: "safety".to_string(),
        summary: "Content safety concern — no external evidence available at filing time".to_string(),
        evidence_refs: String::new(),
        filed_at: "2026-04-19T11:00:00Z".to_string(),
        reach: "intimate".to_string(),
        dht_anchor_hash: "uhCkkSAFETYANCHOR0123456789012345678901234567890123456789012"
            .to_string(),
        created_at: "2026-04-19T11:00:01Z".to_string(),
    };

    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/gate-decision-challenge-view.schema.json", &json);
}

// ── Challenge Outcome ───────────────────────────────────────────

#[test]
fn challenge_outcome_view_upheld_matches_schema() {
    use elohim_storage::ChallengeOutcomeView;

    let view = ChallengeOutcomeView {
        outcome_id: "bafyreib2vq7outcome01234567890123456789012345678901234567890"
            .to_string(),
        challenge_cid: "bafyreib2vq7challenge01234567890123456789012345678901234567"
            .to_string(),
        verdict: "upheld".to_string(),
        reviewer_consensus:
            "uhCAkREVIEWER1ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789,uhCAkREVIEWER2ABCDEFGHIJK"
                .to_string(),
        reasoning_json: r#"{"summary":"Evidence confirms challenger's grounds","steps":[],"constitutional_basis":["P4"]}"#
            .to_string(),
        decided_at: "2026-04-20T10:00:00Z".to_string(),
        indemnification_actions_json: r#"[{"kind":"reputation-degrade","dimensions":["appeals-sustained"],"magnitude":0.15}]"#
            .to_string(),
        dht_anchor_hash: "uhCkkOUTCOMEANCHOR01234567890123456789012345678901234567890"
            .to_string(),
        created_at: "2026-04-20T10:00:01Z".to_string(),
    };

    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/challenge-outcome-view.schema.json", &json);
}

#[test]
fn challenge_outcome_view_dismissed_no_indemnification() {
    use elohim_storage::ChallengeOutcomeView;

    let view = ChallengeOutcomeView {
        outcome_id: "bafyreib2vq7outcome-dismissed-0123456789012345678901234567890"
            .to_string(),
        challenge_cid: "bafyreib2vq7challenge-dismissed-01234567890123456789012345678"
            .to_string(),
        verdict: "dismissed".to_string(),
        reviewer_consensus:
            "uhCAkREVIEWERSINGLE01234567890123456789012345678901234567890123456".to_string(),
        reasoning_json: r#"{"summary":"Insufficient evidence to sustain challenge","steps":[]}"#
            .to_string(),
        decided_at: "2026-04-20T14:00:00Z".to_string(),
        indemnification_actions_json: "[]".to_string(),
        dht_anchor_hash: "uhCkkDISMISSEDANCHOR0123456789012345678901234567890123456789"
            .to_string(),
        created_at: "2026-04-20T14:00:01Z".to_string(),
    };

    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/challenge-outcome-view.schema.json", &json);
}

// ── Peer Status + Elohim Capability ────────────────────────────

#[test]
fn peer_status_view_no_elohim_capability_matches_schema() {
    use elohim_storage::PeerStatusView;

    let view = PeerStatusView {
        peer_id: "uhCAkSMOKEPEER001".to_string(),
        status: "online".to_string(),
        general_pool_member: true,
        accepting_stewardship_reserves: true,
        archetype_class: Some("home-nuc".to_string()),
        timestamp: "1700000000000000".to_string(),
        dht_anchor_hash: "uhCkkSMOKEHASH001".to_string(),
        updated_at: "1700000000100000".to_string(),
        elohim_capability: None,
    };

    let json = serde_json::to_value(&view).unwrap();
    // elohim_capability must be absent (skip_serializing_if) when None
    assert!(
        !json.as_object().unwrap().contains_key("elohimCapability"),
        "elohimCapability must be absent when None (skip_serializing_if)"
    );
    validate_against_schema("views/peer-status-view.schema.json", &json);
}

#[test]
fn peer_status_view_full_elohim_capability_matches_schema() {
    use elohim_storage::{ElohimCapabilityProfile, PeerStatusView};

    let view = PeerStatusView {
        peer_id: "uhCAkFULLPEER001".to_string(),
        status: "online".to_string(),
        general_pool_member: true,
        accepting_stewardship_reserves: false,
        archetype_class: Some("blade".to_string()),
        timestamp: "1700000000000000".to_string(),
        dht_anchor_hash: "uhCkkFULLHASH001".to_string(),
        updated_at: "1700000000100000".to_string(),
        elohim_capability: Some(ElohimCapabilityProfile {
            model_name: "claude-opus-4-7".to_string(),
            model_family: "claude".to_string(),
            context_window_tokens: 1_000_000,
            constitution_cid: Some(
                "bafyreib2vq7constitutionABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".to_string(),
            ),
            quantization_spec: None,
            deployment_context: Some("elohim-node-linux-x86_64".to_string()),
            specialties: vec!["child-safety".to_string(), "curriculum-design".to_string()],
            skills: vec![
                "content-safety-review".to_string(),
                "discernment-evaluation".to_string(),
            ],
            strengths: vec!["consistent-constitutional-reasoning".to_string()],
            active_since: "2026-04-18T00:00:00Z".to_string(),
            reach_level: Some("community".to_string()),
        }),
    };

    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/peer-status-view.schema.json", &json);
}

#[test]
fn peer_status_view_minimal_elohim_capability_matches_schema() {
    use elohim_storage::{ElohimCapabilityProfile, PeerStatusView};

    // Only required fields on ElohimCapabilityProfile; all optional fields absent/None.
    let view = PeerStatusView {
        peer_id: "uhCAkMINIMALPEER01".to_string(),
        status: "degraded".to_string(),
        general_pool_member: false,
        accepting_stewardship_reserves: false,
        archetype_class: None,
        timestamp: "1700000000000000".to_string(),
        dht_anchor_hash: "uhCkkMINIMALHASH01".to_string(),
        updated_at: "1700000000100000".to_string(),
        elohim_capability: Some(ElohimCapabilityProfile {
            model_name: "llama-3.1-8b".to_string(),
            model_family: "llama".to_string(),
            context_window_tokens: 128_000,
            constitution_cid: None,
            quantization_spec: None,
            deployment_context: None,
            specialties: vec![],
            skills: vec![],
            strengths: vec![],
            active_since: "2026-01-01T00:00:00Z".to_string(),
            reach_level: None,
        }),
    };

    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/peer-status-view.schema.json", &json);
}

#[test]
fn elohim_capability_profile_rejects_additional_properties() {
    // additionalProperties: false enforcement: inject an unknown field
    // and verify schema validation catches it.
    let mut profile = serde_json::json!({
        "modelName": "claude-opus-4-7",
        "modelFamily": "claude",
        "contextWindowTokens": 1000000,
        "activeSince": "2026-04-18T00:00:00Z"
    });
    profile["unknownField"] = serde_json::json!("should-not-be-here");

    let schema = load_schema("views/elohim-capability-profile.schema.json");
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors: Vec<_> = validator.iter_errors(&profile).collect();
    assert!(
        !errors.is_empty(),
        "Schema must reject additionalProperties on ElohimCapabilityProfile"
    );
}

#[test]
fn peer_status_view_rejects_additional_properties() {
    let mut status = serde_json::json!({
        "peerId": "uhCAkSMOKE",
        "status": "online",
        "generalPoolMember": true,
        "acceptingStewardshipReserves": true,
        "timestamp": "1700000000000000",
        "dhtAnchorHash": "uhCkkSMOKE",
        "updatedAt": "1700000000000000"
    });
    status["injectedField"] = serde_json::json!("should-fail");

    let schema = load_schema("views/peer-status-view.schema.json");
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors: Vec<_> = validator.iter_errors(&status).collect();
    assert!(
        !errors.is_empty(),
        "Schema must reject additionalProperties on PeerStatusView"
    );
}

#[test]
fn elohim_capability_profile_standalone_matches_schema() {
    use elohim_storage::ElohimCapabilityProfile;

    let profile = ElohimCapabilityProfile {
        model_name: "gpt-4o".to_string(),
        model_family: "gpt".to_string(),
        context_window_tokens: 128_000,
        constitution_cid: None,
        quantization_spec: None,
        deployment_context: Some("tauri-desktop-mac-arm64".to_string()),
        specialties: vec![],
        skills: vec!["content-safety-review".to_string()],
        strengths: vec![],
        active_since: "2026-04-18T10:00:00Z".to_string(),
        reach_level: None,
    };

    let json = serde_json::to_value(&profile).unwrap();
    validate_against_schema("views/elohim-capability-profile.schema.json", &json);
}

// ── ElohimCapabilityProfile vocabulary validation ───────────────

#[test]
fn elohim_capability_profile_core_specialties_valid() {
    // Core-tier specialties must pass schema validation.
    let profile = serde_json::json!({
        "modelName": "claude-opus-4-7",
        "modelFamily": "claude",
        "contextWindowTokens": 200000,
        "specialties": ["child-safety", "family-dynamics", "content-safety", "discernment",
                        "reach-evaluation", "medical", "legal", "crisis"],
        "skills": [],
        "strengths": [],
        "activeSince": "2026-04-18T00:00:00Z"
    });
    validate_against_schema("views/elohim-capability-profile.schema.json", &profile);
}

#[test]
fn elohim_capability_profile_extensible_specialties_valid() {
    // Extensible-tier specialties must also pass schema validation.
    let profile = serde_json::json!({
        "modelName": "llama-3.1-70b",
        "modelFamily": "llama",
        "contextWindowTokens": 128000,
        "specialties": ["education", "code-review", "governance", "curriculum-design"],
        "skills": [],
        "strengths": [],
        "activeSince": "2026-04-18T00:00:00Z"
    });
    validate_against_schema("views/elohim-capability-profile.schema.json", &profile);
}

#[test]
fn elohim_capability_profile_unknown_specialty_rejected() {
    // Unknown specialty values must be rejected by the schema (strict vocabulary).
    let profile = serde_json::json!({
        "modelName": "test-model",
        "modelFamily": "test",
        "contextWindowTokens": 4096,
        "specialties": ["pineapple"],
        "skills": [],
        "strengths": [],
        "activeSince": "2026-04-18T00:00:00Z"
    });
    let schema = load_schema("views/elohim-capability-profile.schema.json");
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors: Vec<_> = validator.iter_errors(&profile).collect();
    assert!(
        !errors.is_empty(),
        "Schema must reject unknown specialty 'pineapple' — ElohimSpecialty vocabulary is strict"
    );
}

#[test]
fn elohim_capability_profile_core_skills_valid() {
    // Core-tier skills (gate-shaped) must pass schema validation.
    let profile = serde_json::json!({
        "modelName": "claude-opus-4-7",
        "modelFamily": "claude",
        "contextWindowTokens": 200000,
        "specialties": [],
        "skills": [
            "content-safety-review",
            "discernment-evaluation",
            "reach-negotiation",
            "attestation-recommendation",
            "spiral-detection",
            "care-connection",
            "graduated-intervention",
            "constitutional-verification"
        ],
        "strengths": [],
        "activeSince": "2026-04-18T00:00:00Z"
    });
    validate_against_schema("views/elohim-capability-profile.schema.json", &profile);
}

#[test]
fn elohim_capability_profile_extensible_skills_valid() {
    // Extensible skills (full ElohimCapability variant set) must pass schema validation.
    let profile = serde_json::json!({
        "modelName": "gpt-4o",
        "modelFamily": "gpt",
        "contextWindowTokens": 128000,
        "specialties": [],
        "skills": [
            "accuracy-verification",
            "knowledge-map-synthesis",
            "mastery-assessment-design",
            "feedback-profile-negotiation",
            "bioregional-enforcement"
        ],
        "strengths": [],
        "activeSince": "2026-04-18T00:00:00Z"
    });
    validate_against_schema("views/elohim-capability-profile.schema.json", &profile);
}

#[test]
fn elohim_capability_profile_unknown_skill_rejected() {
    // Unknown skill values must be rejected by the schema (strict vocabulary).
    let profile = serde_json::json!({
        "modelName": "test-model",
        "modelFamily": "test",
        "contextWindowTokens": 4096,
        "specialties": [],
        "skills": ["psychic-prediction"],
        "strengths": [],
        "activeSince": "2026-04-18T00:00:00Z"
    });
    let schema = load_schema("views/elohim-capability-profile.schema.json");
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors: Vec<_> = validator.iter_errors(&profile).collect();
    assert!(
        !errors.is_empty(),
        "Schema must reject unknown skill 'psychic-prediction' — ElohimSkill vocabulary is strict"
    );
}

#[test]
fn elohim_capability_profile_core_strengths_valid() {
    // Core-tier strengths must pass schema validation.
    let profile = serde_json::json!({
        "modelName": "claude-opus-4-7",
        "modelFamily": "claude",
        "contextWindowTokens": 200000,
        "specialties": [],
        "skills": [],
        "strengths": [
            "high-confidence-judgments",
            "consensus-alignment",
            "steady-baseline"
        ],
        "activeSince": "2026-04-18T00:00:00Z"
    });
    validate_against_schema("views/elohim-capability-profile.schema.json", &profile);
}

#[test]
fn elohim_capability_profile_extensible_strengths_valid() {
    // Extensible-tier strengths must also pass schema validation.
    let profile = serde_json::json!({
        "modelName": "claude-opus-4-7",
        "modelFamily": "claude",
        "contextWindowTokens": 200000,
        "specialties": [],
        "skills": [],
        "strengths": ["consistent-constitutional-reasoning", "low-false-positive-rate"],
        "activeSince": "2026-04-18T00:00:00Z"
    });
    validate_against_schema("views/elohim-capability-profile.schema.json", &profile);
}

#[test]
fn elohim_capability_profile_unknown_strength_rejected() {
    // Unknown strength values must be rejected by the schema (strict vocabulary).
    let profile = serde_json::json!({
        "modelName": "test-model",
        "modelFamily": "test",
        "contextWindowTokens": 4096,
        "specialties": [],
        "skills": [],
        "strengths": ["made-up-accolade"],
        "activeSince": "2026-04-18T00:00:00Z"
    });
    let schema = load_schema("views/elohim-capability-profile.schema.json");
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors: Vec<_> = validator.iter_errors(&profile).collect();
    assert!(
        !errors.is_empty(),
        "Schema must reject unknown strength 'made-up-accolade' — ElohimStrength vocabulary is strict"
    );
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
        "views/gate-decision-attestation-view.schema.json",
        "views/peer-status-view.schema.json",
        "views/elohim-capability-profile.schema.json",
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
