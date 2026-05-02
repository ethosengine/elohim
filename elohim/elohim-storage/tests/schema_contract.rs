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
/// Also indexes by `$id` so that URI-style `$ref` values (e.g. `epr:schema:view:human`)
/// resolve correctly for schemas like `account-view.schema.json`.
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
                            // Same-dir ref with leading ./: "./drain-status-view.schema.json"
                            refs.insert(format!("./{}", filename), schema.clone());
                            // Cross-dir ref: "../enums/nat-status.schema.json"
                            refs.insert(format!("../{}/{}", subdir, filename), schema.clone());
                            // From views/ to views/: "replication-status-view.schema.json"
                            refs.insert(format!("{}/{}", subdir, filename), schema.clone());
                            // URI-style $id ref: "epr:schema:view:human" / "epr:schema:enum:reach"
                            // Allows account-view.schema.json $refs to resolve during inline_refs.
                            if let Some(Value::String(id)) = schema.get("$id") {
                                refs.insert(id.clone(), schema);
                            }
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
        dedup_unique_len: 42,
        dedup_total_seen: 55,
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
        dedup_unique_len: 0,
        dedup_total_seen: 0,
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
        challenger_id: "uhCAkCHALLENGER2ABCDEFGHIJKLMNOPQRSTUVWXYZ01234567890123456789".to_string(),
        grounds: "safety".to_string(),
        summary: "Content safety concern — no external evidence available at filing time"
            .to_string(),
        evidence_refs: String::new(),
        filed_at: "2026-04-19T11:00:00Z".to_string(),
        reach: "intimate".to_string(),
        dht_anchor_hash: "uhCkkSAFETYANCHOR0123456789012345678901234567890123456789012".to_string(),
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
        outcome_id: "bafyreib2vq7outcome-dismissed-0123456789012345678901234567890".to_string(),
        challenge_cid: "bafyreib2vq7challenge-dismissed-01234567890123456789012345678".to_string(),
        verdict: "dismissed".to_string(),
        reviewer_consensus: "uhCAkREVIEWERSINGLE01234567890123456789012345678901234567890123456"
            .to_string(),
        reasoning_json: r#"{"summary":"Insufficient evidence to sustain challenge","steps":[]}"#
            .to_string(),
        decided_at: "2026-04-20T14:00:00Z".to_string(),
        indemnification_actions_json: "[]".to_string(),
        dht_anchor_hash: "uhCkkDISMISSEDANCHOR0123456789012345678901234567890123456789".to_string(),
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

// ── Elohim Reputation Profile ───────────────────────────────────

#[test]
fn elohim_reputation_profile_view_matches_schema() {
    use elohim_storage::views::JsonVal;
    use elohim_storage::ElohimReputationProfileView;

    let view = ElohimReputationProfileView {
        elohim_id: "uhCAkABCDEFGHIJKLMNOPQRSTUVWXYZ012345678901234567890123456789012".to_string(),
        window_start: "2026-01-19T00:00:00Z".to_string(),
        window_end: "2026-04-19T00:00:00Z".to_string(),
        current_substance_cid: Some(
            "bafyreib2vq7substance01234567890123456789012345678901234567".to_string(),
        ),
        total_decisions: 42,
        challenged_count: 3,
        upheld_count: 1,
        dismissed_count: 1,
        superseded_count: 0,
        pending_count: 1,
        challenges_by_grounds: JsonVal(serde_json::json!({
            "safety": 2,
            "policy": 1
        })),
        outcomes_by_verdict: JsonVal(serde_json::json!({
            "upheld": 1,
            "dismissed": 1
        })),
    };

    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/elohim-reputation-profile-view.schema.json", &json);
}

#[test]
fn elohim_reputation_profile_view_empty_window_matches_schema() {
    use elohim_storage::views::JsonVal;
    use elohim_storage::ElohimReputationProfileView;

    // An elohim with no decisions in the window is a valid response (never a 404).
    let view = ElohimReputationProfileView {
        elohim_id: "uhCAkFRESH01234567890123456789012345678901234567890123456789012".to_string(),
        window_start: "2026-03-20T00:00:00Z".to_string(),
        window_end: "2026-04-19T00:00:00Z".to_string(),
        current_substance_cid: None,
        total_decisions: 0,
        challenged_count: 0,
        upheld_count: 0,
        dismissed_count: 0,
        superseded_count: 0,
        pending_count: 0,
        challenges_by_grounds: JsonVal(serde_json::json!({})),
        outcomes_by_verdict: JsonVal(serde_json::json!({})),
    };

    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/elohim-reputation-profile-view.schema.json", &json);
}

// ── Node-shape / household-devices views ────────────────────────

#[test]
fn node_shape_view_matches_schema() {
    use elohim_storage::{CommittedResources, NodeShapeView};

    let view = NodeShapeView {
        node_id: "uhCAkTESTNODE001".to_string(),
        hostname: "matthew-home".to_string(),
        device_archetype_id: "home-nuc".to_string(),
        household_id: "household-matthew".to_string(),
        role: "edge".to_string(),
        capability_level: 3,
        committed: CommittedResources {
            cpu_cores: 4,
            memory_gb: 16,
            storage_tb: 2.0,
            bandwidth_mbps: Some(1000),
            max_custody_gb: Some(500.0),
            can_steward: true,
            can_infer: false,
            can_doorway: false,
        },
        steward_tier: Some("guardian".to_string()),
        custodian_opt_in: true,
        region: Some("us-central".to_string()),
        signature: "uhCkkSIGNATURE001".to_string(),
        signed_at: "2026-04-19T00:00:00Z".to_string(),
        dht_anchor_hash: None,
    };

    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/node-shape-view.schema.json", &json);
}

#[test]
fn household_devices_view_matches_schema() {
    use elohim_storage::HouseholdDevicesView;

    let view = HouseholdDevicesView {
        household_id: "household-matthew".to_string(),
        devices: vec![],
    };

    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/household-devices-view.schema.json", &json);
}

// ── Placement Gap + Resilience Snapshot ────────────────────────

#[test]
fn placement_gap_view_matches_schema() {
    use elohim_storage::PlacementGapView;

    let view = PlacementGapView {
        id: "gap-uuid-001".to_string(),
        content_id: "content-abc-001".to_string(),
        shard_hash: "bafyreib2vq7shardABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789012345".to_string(),
        requested_steward_count: 4,
        achieved_steward_count: 2,
        contract_coverage: 0.5,
        gap_kind: "under-committed".to_string(),
        first_seen_at: "2026-04-19T10:00:00Z".to_string(),
        last_seen_at: "2026-04-20T10:00:00Z".to_string(),
    };

    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/placement-gap-view.schema.json", &json);
}

#[test]
fn resilience_snapshot_view_matches_schema() {
    use elohim_storage::{
        PlacementGapView, RegionalDistributionView, ResilienceSnapshotDetailsView,
        ResilienceSnapshotView, StewardingCollectiveEntry,
    };

    let view = ResilienceSnapshotView {
        content_id: "content-abc-001".to_string(),
        stewarding_collectives: 3,
        commitment_backed_collectives: 2,
        diversity_score: 0.75,
        regional_distribution: RegionalDistributionView {
            local: 2,
            regional: 1,
            global: 0,
            unknown: 0,
        },
        placement_gaps: vec![PlacementGapView {
            id: "gap-uuid-002".to_string(),
            content_id: "content-abc-001".to_string(),
            shard_hash: "bafyreib2vq7shardXYZ0123456789012345678901234567890123456789".to_string(),
            requested_steward_count: 4,
            achieved_steward_count: 3,
            contract_coverage: 0.75,
            gap_kind: "peers-unavailable".to_string(),
            first_seen_at: "2026-04-19T08:00:00Z".to_string(),
            last_seen_at: "2026-04-20T08:00:00Z".to_string(),
        }],
        protection_status: "partial".to_string(),
        reciprocating_collectives: Some(1),
        details: Some(ResilienceSnapshotDetailsView {
            stewarding_collectives: vec![
                StewardingCollectiveEntry {
                    id: "household-matthew".to_string(),
                    kind: "household".to_string(),
                    label: Some("Matthew's Home".to_string()),
                },
                StewardingCollectiveEntry {
                    id: "church-bethel".to_string(),
                    kind: "church".to_string(),
                    label: None,
                },
            ],
            online_peer_count: 5,
            health_score: 0.8,
        }),
    };

    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/resilience-snapshot-view.schema.json", &json);
}

#[test]
fn resilience_snapshot_view_minimal_matches_schema() {
    use elohim_storage::{RegionalDistributionView, ResilienceSnapshotView};

    // Minimal: no reciprocating_collectives, no details, empty gaps
    let view = ResilienceSnapshotView {
        content_id: "content-minimal-001".to_string(),
        stewarding_collectives: 1,
        commitment_backed_collectives: 1,
        diversity_score: 1.0,
        regional_distribution: RegionalDistributionView {
            local: 1,
            regional: 0,
            global: 0,
            unknown: 0,
        },
        placement_gaps: vec![],
        protection_status: "protected".to_string(),
        reciprocating_collectives: None,
        details: None,
    };

    let json = serde_json::to_value(&view).unwrap();
    // Optional fields must be absent when None (skip_serializing_if)
    assert!(
        !json
            .as_object()
            .unwrap()
            .contains_key("reciprocatingCollectives"),
        "reciprocatingCollectives must be absent when None"
    );
    assert!(
        !json.as_object().unwrap().contains_key("details"),
        "details must be absent when None"
    );
    validate_against_schema("views/resilience-snapshot-view.schema.json", &json);
}

// ============================================================================
// Phase 2a — EPR view schema conformance (Task 11)
// ============================================================================

fn sample_envelope_view() -> elohim_storage::EprEnvelopeView {
    use elohim_storage::{EprCouplingView, EprEnvelopeView, EprSignatureView};
    EprEnvelopeView {
        cid: "bafyreib2vq7fztfnmgzrmo7q5jnfkdvxkfxpvsjmesxrqjzqxkzqzqzqa".into(),
        kind: "Manifest".into(),
        schema_ref: "bafyreib2vq7schemaref01234567890123456789012345678901234567890".into(),
        schema_key: "concept".into(),
        reach: "commons".into(),
        coupling: EprCouplingView {
            knowledge: None,
            value: None,
            governance: Some(
                "bafyreib2vq7governance01234567890123456789012345678901234567890".into(),
            ),
        },
        claims: vec![],
        supersedes: None,
        superseded_by: None,
        issued_at: "2026-04-22T00:00:00Z".into(),
        proof: EprSignatureView {
            signer: "bafyreib2vq7agentcid01234567890123456789012345678901234567890".into(),
            algorithm: "ed25519".into(),
            // 128 lowercase hex chars = 64 bytes
            signature: "a".repeat(128),
        },
    }
}

#[test]
fn epr_envelope_view_conforms() {
    let v = sample_envelope_view();
    let json = serde_json::to_value(&v).unwrap();
    validate_against_schema("views/epr-envelope-view.schema.json", &json);
}

#[test]
fn epr_view_conforms() {
    use elohim_storage::EprView;
    let v = EprView {
        envelope: sample_envelope_view(),
        payload: "deadbeef".into(),
        canonical_bytes: None,
    };
    let json = serde_json::to_value(&v).unwrap();
    validate_against_schema("views/epr-view.schema.json", &json);
}

#[test]
fn epr_verify_view_conforms() {
    use elohim_storage::EprVerifyView;
    let v = EprVerifyView {
        cid: "bafyreib2vq7fztfnmgzrmo7q5jnfkdvxkfxpvsjmesxrqjzqxkzqzqzqa".into(),
        verified: true,
        stages_run: vec![
            "canonicalization".into(),
            "signature".into(),
            "coupling".into(),
        ],
        stages_skipped: vec!["payloadSchema".into()],
        error: None,
    };
    let json = serde_json::to_value(&v).unwrap();
    validate_against_schema("views/epr-verify-view.schema.json", &json);
}

#[test]
fn epr_list_view_conforms() {
    use elohim_storage::EprListView;
    let v = EprListView {
        items: vec![sample_envelope_view()],
        next_cursor: Some("bafyreib2vq7fztfnmgzrmo7q5jnfkdvxkfxpvsjmesxrqjzqxkzqzqzqa".into()),
    };
    let json = serde_json::to_value(&v).unwrap();
    validate_against_schema("views/epr-list-view.schema.json", &json);
}

#[test]
fn epr_publish_input_conforms() {
    use elohim_storage::EprPublishInput;
    let v = EprPublishInput {
        envelope: sample_envelope_view(),
        payload: "cafebabe".into(),
    };
    let json = serde_json::to_value(&v).unwrap();
    validate_against_schema("inputs/epr-publish-input.schema.json", &json);
}

#[test]
fn epr_providers_view_conforms() {
    use elohim_storage::EprProvidersView;
    let v = EprProvidersView {
        cid: "bafyreib2vq7fztfnmgzrmo7q5jnfkdvxkfxpvsjmesxrqjzqxkzqzqzqa".into(),
        providers: vec!["local".into()],
    };
    let json = serde_json::to_value(&v).unwrap();
    validate_against_schema("views/epr-providers-view.schema.json", &json);
}

#[test]
fn epr_providers_view_schema_parses() {
    // Ensures the schema file is present and valid JSON Schema — will panic if not.
    let _ = load_schema("views/epr-providers-view.schema.json");
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
        "views/elohim-reputation-profile-view.schema.json",
        "views/epr-view.schema.json",
        "views/epr-envelope-view.schema.json",
        "views/epr-verify-view.schema.json",
        "views/epr-list-view.schema.json",
        "views/epr-providers-view.schema.json",
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

// -----------------------------------------------------------------------
// /elohim/epr-atom/1.0.0 wire contract validation (Phase 2c)
//
// Validates the transient wire contract — no projection, no persistent
// source of truth. The notarized source of truth is the ed25519-signed
// CBOR Envelope (content-addressed by CID); this test only verifies the
// logical request/response shapes described in the wire contract.
//
// Note: CBOR encodes `envelope_bytes` as a native byte string. For this
// JSON-Schema validation we use base64-encoded strings, which is the
// shape a non-CBOR peer would produce. On-wire CBOR encoding is
// separately exercised by tests in `tests/epr_atom_protocol_unit.rs`.
// -----------------------------------------------------------------------

#[test]
fn epr_atom_wire_contract_validates_request_shapes() {
    let schema = load_schema("p2p/epr-atom-message.schema.json");
    let validator =
        jsonschema::validator_for(&schema).expect("epr-atom wire contract should compile");

    let examples = [
        serde_json::json!({ "tag": "fetch", "cid": "bafkreiabc" }),
        serde_json::json!({ "tag": "announce", "envelope_bytes": "AQID" }),
        serde_json::json!({
            "tag": "fetch_batch",
            "cids": ["bafkreiabc", "bafkreidef"],
        }),
    ];

    for instance in &examples {
        let errors: Vec<String> = validator
            .iter_errors(instance)
            .map(|e| format!("  - {} (at {})", e, e.instance_path))
            .collect();
        assert!(
            errors.is_empty(),
            "request shape failed wire contract:\n{}\n\ninstance:\n{}",
            errors.join("\n"),
            serde_json::to_string_pretty(instance).unwrap()
        );
    }
}

#[test]
fn epr_atom_wire_contract_validates_response_shapes() {
    let schema = load_schema("p2p/epr-atom-message.schema.json");
    let validator =
        jsonschema::validator_for(&schema).expect("epr-atom wire contract should compile");

    let examples = [
        serde_json::json!({ "tag": "atom", "envelope_bytes": "AQID" }),
        serde_json::json!({
            "tag": "atom_batch",
            "atoms": ["AQ==", null, "AwQ="],
        }),
        serde_json::json!({ "tag": "announced", "accepted": true, "reason": null }),
        serde_json::json!({
            "tag": "announced",
            "accepted": false,
            "reason": "signature verification failed",
        }),
        serde_json::json!({ "tag": "not_found" }),
        serde_json::json!({ "tag": "error", "message": "batch too large" }),
    ];

    for instance in &examples {
        let errors: Vec<String> = validator
            .iter_errors(instance)
            .map(|e| format!("  - {} (at {})", e, e.instance_path))
            .collect();
        assert!(
            errors.is_empty(),
            "response shape failed wire contract:\n{}\n\ninstance:\n{}",
            errors.join("\n"),
            serde_json::to_string_pretty(instance).unwrap()
        );
    }
}

#[test]
fn epr_atom_wire_contract_rejects_oversized_batch() {
    let schema = load_schema("p2p/epr-atom-message.schema.json");
    let validator =
        jsonschema::validator_for(&schema).expect("epr-atom wire contract should compile");

    // 129 cids — schema cap is 128
    let cids: Vec<String> = (0..129).map(|i| format!("bafkrei_{}", i)).collect();
    let oversize = serde_json::json!({ "tag": "fetch_batch", "cids": cids });

    let has_errors = validator.iter_errors(&oversize).next().is_some();
    assert!(
        has_errors,
        "wire contract should reject batch > MAX_BATCH_CIDS"
    );
}

#[test]
fn epr_atom_wire_contract_rejects_missing_tag() {
    let schema = load_schema("p2p/epr-atom-message.schema.json");
    let validator =
        jsonschema::validator_for(&schema).expect("epr-atom wire contract should compile");

    let missing_tag = serde_json::json!({ "cid": "bafkreiabc" });

    let has_errors = validator.iter_errors(&missing_tag).next().is_some();
    assert!(has_errors, "wire contract should require tag discriminator");
}

// =============================================================================
// Recovery Protocol Phase 2 — M4 View Schema Contract Tests
//
// Source of truth: DHT (imagodei KeyRevocation + RevocationVote entries).
// These tests verify that the Rust projection view structs match their
// JSON Schema contracts in elohim/sdk/schemas/v1/views/.
// =============================================================================

#[test]
fn key_revocation_view_matches_schema() {
    use elohim_storage::views::KeyRevocationView;

    let sample = KeyRevocationView {
        dht_anchor_hash: "hCAkTESTHASH123".into(),
        id: "rev-human-matthew-2026-04-24T00:00:00Z".into(),
        human_id: "human-matthew".into(),
        revoked_key: "uhCAkABCD1234567890".into(),
        reason: "compromised".into(),
        trigger_type: "voluntary".into(),
        initiated_by: "human-matthew".into(),
        required_votes: 1,
        current_votes: 1,
        threshold_reached: true,
        effective_at: Some("2026-04-24T00:00:00Z".into()),
        created_at: "2026-04-24T00:00:00Z".into(),
        updated_at: "2026-04-24T00:00:00Z".into(),
    };

    validate_against_schema(
        "views/key-revocation.schema.json",
        &serde_json::to_value(&sample).unwrap(),
    );
}

#[test]
fn key_revocation_view_null_effective_at_matches_schema() {
    use elohim_storage::views::KeyRevocationView;

    let sample = KeyRevocationView {
        dht_anchor_hash: "hCAkTESTHASH456".into(),
        id: "rev-human-jessica-2026-04-24T00:01:00Z".into(),
        human_id: "human-jessica".into(),
        revoked_key: "uhCAkXYZ987654321".into(),
        reason: "stolen".into(),
        trigger_type: "steward_vote".into(),
        initiated_by: "human-timothy".into(),
        required_votes: 3,
        current_votes: 0,
        threshold_reached: false,
        effective_at: None,
        created_at: "2026-04-24T00:01:00Z".into(),
        updated_at: "2026-04-24T00:01:00Z".into(),
    };

    validate_against_schema(
        "views/key-revocation.schema.json",
        &serde_json::to_value(&sample).unwrap(),
    );
}

#[test]
fn revocation_vote_view_matches_schema() {
    use elohim_storage::views::RevocationVoteView;

    let sample = RevocationVoteView {
        dht_anchor_hash: "hCAkVOTEHASH789".into(),
        id: "vote-human-jessica-2026-04-24T00:02:00Z".into(),
        revocation_dht_anchor_hash: "hCAkREVHASH123".into(),
        revocation_id: "rev-human-jessica-2026-04-24T00:01:00Z".into(),
        steward_id: "human-timothy".into(),
        approved: true,
        attestation: "Key was captured by attacker; I verified identity via video call.".into(),
        voted_at: "2026-04-24T00:02:00Z".into(),
    };

    validate_against_schema(
        "views/revocation-vote.schema.json",
        &serde_json::to_value(&sample).unwrap(),
    );
}

#[test]
fn revocation_vote_view_rejected_matches_schema() {
    use elohim_storage::views::RevocationVoteView;

    let sample = RevocationVoteView {
        dht_anchor_hash: "hCAkVOTEHASHabc".into(),
        id: "vote-human-sarah-2026-04-24T00:03:00Z".into(),
        revocation_dht_anchor_hash: "hCAkREVHASH123".into(),
        revocation_id: "rev-human-jessica-2026-04-24T00:01:00Z".into(),
        steward_id: "human-sarah".into(),
        approved: false,
        attestation: "I spoke to Jessica directly; she still has her phone.".into(),
        voted_at: "2026-04-24T00:03:00Z".into(),
    };

    validate_against_schema(
        "views/revocation-vote.schema.json",
        &serde_json::to_value(&sample).unwrap(),
    );
}

// =============================================================================
// Recovery Protocol Phase 2 — M5 View Schema Contract Tests
//
// Auth Portal Convergence + Revocation UX + Stub Defender.
// Source of truth: DHT (imagodei PortalHost + AgentPeerBinding) for sub-views;
// composite DHT projection for AccountView.
// =============================================================================

#[test]
fn human_view_matches_schema() {
    use elohim_storage::views::HumanView;

    let sample = HumanView {
        id: "human-matthew".into(),
        agent_pub_key: Some("uhCAkABCD1234567890".into()),
        display_name: "Matthew".into(),
        bio: Some("Protocol architect and household operator.".into()),
        affinities: vec!["education".into(), "distributed-systems".into()],
        profile_reach: "community".into(),
        location: Some("Pacific Northwest".into()),
        profile_photo_url: None,
        h_app_id: "elohim-app-test-001".into(),
        created_at: "2026-04-25T00:00:00Z".into(),
        updated_at: "2026-04-25T00:00:00Z".into(),
        dht_anchor_hash: Some("uhCkkHUMAN001".into()),
    };

    validate_against_schema(
        "views/human.schema.json",
        &serde_json::to_value(&sample).unwrap(),
    );
}

#[test]
fn human_view_pre_coherence_matches_schema() {
    use elohim_storage::views::HumanView;

    // Pre-coherence rows have no agentPubKey and no dhtAnchorHash.
    let sample = HumanView {
        id: "human-timothy".into(),
        agent_pub_key: None,
        display_name: "Timothy".into(),
        bio: None,
        affinities: vec![],
        profile_reach: "intimate".into(),
        location: None,
        profile_photo_url: None,
        h_app_id: "elohim-app-test-001".into(),
        created_at: "2026-04-25T00:01:00Z".into(),
        updated_at: "2026-04-25T00:01:00Z".into(),
        dht_anchor_hash: None,
    };

    validate_against_schema(
        "views/human.schema.json",
        &serde_json::to_value(&sample).unwrap(),
    );
}

#[test]
fn human_relationship_view_matches_schema() {
    use elohim_storage::views::HumanRelationshipView;
    use elohim_storage::views::JsonVal;

    let sample = HumanRelationshipView {
        id: "rel-matthew-jessica-family".into(),
        h_app_id: "elohim-app-test-001".into(),
        party_a_id: "human-matthew".into(),
        party_b_id: "human-jessica".into(),
        relationship_type: "family".into(),
        intimacy_level: "intimate".into(),
        is_bidirectional: true,
        consent_given_by_a: true,
        consent_given_by_b: true,
        custody_enabled_by_a: true,
        custody_enabled_by_b: true,
        auto_custody_enabled: true,
        emergency_access_enabled: true,
        initiated_by: "human-matthew".into(),
        verified_at: Some("2026-04-25T00:00:00Z".into()),
        governance_layer: Some("household".into()),
        reach: "intimate".into(),
        context: Some(JsonVal(serde_json::json!({ "notes": "spouse" }))),
        created_at: "2026-04-25T00:00:00Z".into(),
        updated_at: "2026-04-25T00:00:00Z".into(),
        expires_at: None,
        dht_anchor_hash: Some("uhCkkREL001".into()),
    };

    validate_against_schema(
        "views/human-relationship.schema.json",
        &serde_json::to_value(&sample).unwrap(),
    );
}

#[test]
fn human_relationship_view_emergency_contact_matches_schema() {
    use elohim_storage::views::HumanRelationshipView;

    // Minimal emergency-contact row — no optional fields.
    let sample = HumanRelationshipView {
        id: "rel-timothy-matthew-stewardship".into(),
        h_app_id: "elohim-app-test-001".into(),
        party_a_id: "human-timothy".into(),
        party_b_id: "human-matthew".into(),
        relationship_type: "stewardship".into(),
        intimacy_level: "trusted".into(),
        is_bidirectional: false,
        consent_given_by_a: true,
        consent_given_by_b: false,
        custody_enabled_by_a: false,
        custody_enabled_by_b: true,
        auto_custody_enabled: false,
        emergency_access_enabled: true,
        initiated_by: "human-matthew".into(),
        verified_at: None,
        governance_layer: None,
        reach: "trusted".into(),
        context: None,
        created_at: "2026-04-25T00:02:00Z".into(),
        updated_at: "2026-04-25T00:02:00Z".into(),
        expires_at: None,
        dht_anchor_hash: None,
    };

    validate_against_schema(
        "views/human-relationship.schema.json",
        &serde_json::to_value(&sample).unwrap(),
    );
}

#[test]
fn portal_host_view_matches_schema() {
    use elohim_storage::views::PortalHostView;

    let sample = PortalHostView {
        human_id: "uhCEkAbc123".into(),
        host_url: "https://m.example.com/account".into(),
        label: Some("main".into()),
        added_at: "2026-04-25T12:00:00Z".into(),
        last_reachable_at: Some("2026-04-25T12:30:00Z".into()),
        reach: "trusted".into(),
        dht_anchor_hash: "uhCkBcd456".into(),
    };

    validate_against_schema(
        "views/portal-host-view.schema.json",
        &serde_json::to_value(&sample).unwrap(),
    );
}

#[test]
fn portal_host_view_minimal_matches_schema() {
    use elohim_storage::views::PortalHostView;

    // Minimal: no label, no last_reachable_at.
    let sample = PortalHostView {
        human_id: "uhCEkDef789".into(),
        host_url: "https://doorway.elohim.host/account".into(),
        label: None,
        added_at: "2026-04-25T12:00:00Z".into(),
        last_reachable_at: None,
        reach: "trusted".into(),
        dht_anchor_hash: "uhCkGhi012".into(),
    };

    validate_against_schema(
        "views/portal-host-view.schema.json",
        &serde_json::to_value(&sample).unwrap(),
    );
}

#[test]
fn agent_peer_binding_view_matches_schema() {
    use elohim_storage::views::AgentPeerBindingView;

    let sample = AgentPeerBindingView {
        agent_cid: "bafyreib2vq7agentcid01234567890123456789012345678901234567890".into(),
        peer_id: "12D3KooWTestPeer001".into(),
        valid_from: "2026-04-25T00:00:00Z".into(),
        valid_until: Some("2027-04-25T00:00:00Z".into()),
        signature: "a".repeat(128),
        dht_anchor_hash: "uhCkBIND001".into(),
    };

    validate_against_schema(
        "views/agent-peer-binding-view.schema.json",
        &serde_json::to_value(&sample).unwrap(),
    );
}

#[test]
fn agent_peer_binding_view_non_expiring_matches_schema() {
    use elohim_storage::views::AgentPeerBindingView;

    // Non-expiring binding: valid_until is None.
    let sample = AgentPeerBindingView {
        agent_cid: "bafyreib2vq7agentcid-permanent-01234567890123456789012345678".into(),
        peer_id: "12D3KooWPermanentPeer".into(),
        valid_from: "2026-01-01T00:00:00Z".into(),
        valid_until: None,
        signature: "b".repeat(128),
        dht_anchor_hash: "uhCkBINDPERM01".into(),
    };

    validate_against_schema(
        "views/agent-peer-binding-view.schema.json",
        &serde_json::to_value(&sample).unwrap(),
    );
}

#[test]
fn account_view_matches_schema() {
    use elohim_storage::views::{AccountView, HumanRelationshipView, HumanView, PortalHostView};

    let human = HumanView {
        id: "human-matthew".into(),
        agent_pub_key: Some("uhCAkABCD1234567890".into()),
        display_name: "Matthew".into(),
        bio: None,
        affinities: vec!["education".into()],
        profile_reach: "community".into(),
        location: None,
        profile_photo_url: None,
        h_app_id: "elohim-app-test-001".into(),
        created_at: "2026-04-25T00:00:00Z".into(),
        updated_at: "2026-04-25T00:00:00Z".into(),
        dht_anchor_hash: Some("uhCkkHUMAN001".into()),
    };

    let emergency_contact = HumanRelationshipView {
        id: "rel-timothy-matthew-stewardship".into(),
        h_app_id: "elohim-app-test-001".into(),
        party_a_id: "human-timothy".into(),
        party_b_id: "human-matthew".into(),
        relationship_type: "stewardship".into(),
        intimacy_level: "trusted".into(),
        is_bidirectional: false,
        consent_given_by_a: true,
        consent_given_by_b: true,
        custody_enabled_by_a: false,
        custody_enabled_by_b: true,
        auto_custody_enabled: false,
        emergency_access_enabled: true,
        initiated_by: "human-matthew".into(),
        verified_at: None,
        governance_layer: None,
        reach: "trusted".into(),
        context: None,
        created_at: "2026-04-25T00:00:00Z".into(),
        updated_at: "2026-04-25T00:00:00Z".into(),
        expires_at: None,
        dht_anchor_hash: None,
    };

    let portal = PortalHostView {
        human_id: "uhCEkAbc123".into(),
        host_url: "https://doorway.elohim.host/account".into(),
        label: None,
        added_at: "2026-04-25T12:00:00Z".into(),
        last_reachable_at: None,
        reach: "trusted".into(),
        dht_anchor_hash: "uhCkBcd456".into(),
    };

    let sample = AccountView {
        human,
        active_key_rotation: None,
        recent_revocations: vec![],
        pending_recovery_requests: vec![],
        emergency_contacts: vec![emergency_contact],
        portal_hosts: vec![portal],
        is_steward: true,
        has_local_conductor: true,
    };

    validate_against_schema(
        "views/account-view.schema.json",
        &serde_json::to_value(&sample).unwrap(),
    );
}

// =============================================================================
// FeedbackSignal — p2p/feedback-signal.schema.json contract tests
//
// Mirrors the epr-atom-message.schema.json pattern above.  Three tests:
//   1. squelch (evidenceCid absent) passes the schema.
//   2. correction with evidenceCid passes the schema.
//   3. correction WITHOUT evidenceCid is rejected by the schema's if/then clause.
//      Uses a hand-crafted serde_json::Value — NOT the Rust struct's validate() —
//      so the test proves the JSON Schema constraint fires at runtime even if a
//      misbehaving producer bypasses Rust type safety.
// =============================================================================

#[test]
fn feedback_signal_squelch_validates_against_schema() {
    let instance = serde_json::json!({
        "targetCid": "bafyreiabcdef1234567890",
        "signalKind": "squelch",
        "standingImpact": "advisory",
        "signedBy": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
        "signature": "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB="
    });
    validate_against_schema("p2p/feedback-signal.schema.json", &instance);
}

#[test]
fn feedback_signal_correction_with_evidence_validates_against_schema() {
    use elohim_storage::p2p::feedback_signal::{FeedbackSignal, SignalKind, StandingImpact};

    let signal = FeedbackSignal::new_correction(
        "bafyreiabcdef1234567890".to_string(),
        "bafyreicorrection_evidence_cid_abc".to_string(),
        StandingImpact::DebitSoft,
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_string(),
        "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB="
            .to_string(),
    );
    assert_eq!(signal.signal_kind, SignalKind::Correction);
    assert!(signal.evidence_cid.is_some());
    validate_against_schema(
        "p2p/feedback-signal.schema.json",
        &serde_json::to_value(&signal).unwrap(),
    );
}

#[test]
fn feedback_signal_correction_without_evidence_rejected_by_schema() {
    // Hand-crafted Value — NOT constructed through the Rust struct — so this
    // proves the JSON Schema if/then clause fires independently of Rust's
    // validate() guard.
    let bad_instance = serde_json::json!({
        "targetCid": "bafyreiabcdef1234567890",
        "signalKind": "correction",
        "standingImpact": "debit-soft",
        "signedBy": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
        "signature": "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB="
        // evidenceCid intentionally absent — schema if/then must reject this
    });

    let schema = load_schema("p2p/feedback-signal.schema.json");
    let validator =
        jsonschema::validator_for(&schema).expect("feedback-signal schema should compile");

    let has_errors = validator.iter_errors(&bad_instance).next().is_some();
    assert!(
        has_errors,
        "schema should reject a 'correction' signal missing evidenceCid (if/then clause)"
    );
}

// =============================================================================
// AttentionTending — p2p/attention-tending.schema.json contract tests
//
// Three tests:
//   1. A well-formed AttentionTending JSON passes the schema.
//   2. A ttlSeconds value below the 3600 minimum is rejected.
//   3. An empty tendedAt array is rejected (minItems: 1 constraint).
// =============================================================================

#[test]
fn attention_tending_validates_against_schema() {
    use elohim_storage::p2p::attention_tending::{AttentionTending, Classification};

    let tending = AttentionTending {
        filter_subject: serde_json::json!({"contentKind": "concept"}),
        classification: Classification::ValuesForward,
        reason: None,
        ttl_seconds: 2_592_000, // 30 days
        tended_at: vec![1_746_000_000],
        context: serde_json::json!({"collective": "household"}),
        signed_by: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_string(),
        signature:
            "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB="
                .to_string(),
    };
    assert!(tending.validate().is_ok());
    validate_against_schema(
        "p2p/attention-tending.schema.json",
        &serde_json::to_value(&tending).unwrap(),
    );
}

#[test]
fn attention_tending_with_short_ttl_rejected_by_schema() {
    // Hand-crafted Value — NOT constructed through the Rust struct — so this
    // proves the JSON Schema minimum constraint fires independently of Rust's
    // validate() guard.
    let bad_instance = serde_json::json!({
        "filterSubject": {},
        "classification": "fatigue",
        "ttlSeconds": 100,
        "tendedAt": [1746000000],
        "context": {},
        "signedBy": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
        "signature": "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB="
        // ttlSeconds: 100 — below the 3600 minimum; schema must reject
    });

    let schema = load_schema("p2p/attention-tending.schema.json");
    let validator =
        jsonschema::validator_for(&schema).expect("attention-tending schema should compile");

    let has_errors = validator.iter_errors(&bad_instance).next().is_some();
    assert!(
        has_errors,
        "schema should reject ttlSeconds: 100 (below minimum 3600)"
    );
}

#[test]
fn attention_tending_with_empty_tended_at_rejected_by_schema() {
    // Hand-crafted Value — NOT constructed through the Rust struct — so this
    // proves the JSON Schema minItems: 1 constraint fires independently of
    // Rust's validate() guard.
    let bad_instance = serde_json::json!({
        "filterSubject": {},
        "classification": "safety",
        "ttlSeconds": 3600,
        "tendedAt": [],
        "context": {},
        "signedBy": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
        "signature": "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB="
        // tendedAt: [] — empty array; schema minItems: 1 must reject
    });

    let schema = load_schema("p2p/attention-tending.schema.json");
    let validator =
        jsonschema::validator_for(&schema).expect("attention-tending schema should compile");

    let has_errors = validator.iter_errors(&bad_instance).next().is_some();
    assert!(
        has_errors,
        "schema should reject tendedAt: [] (minItems: 1 violated)"
    );
}

// ── Light-Up-Topology Phase 1 — Distribution / Cluster / Reciprocity ────

#[test]
fn distribution_summary_matches_schema() {
    use elohim_storage::views::{
        DiversityHint, DistributionSummary, FetchSource, MyRole, ReachClass, ReplicaHealth,
    };

    let sample = DistributionSummary {
        replica_count: 12,
        replica_target: 14,
        replica_health: ReplicaHealth::Healthy,
        projector_count: 2,
        reach_class: ReachClass::Public,
        diversity_hint: DiversityHint::RegionMetro(vec!["us-central".into(), "eu-west".into()]),
        this_fetch_source: FetchSource::ProjectedViaDoorway,
        last_verified_seconds: 420,
        my_role: Some(MyRole::Replica),
        reciprocity_hint: Some(0),
    };

    let json = serde_json::to_value(&sample).unwrap();
    validate_against_schema("views/distribution-summary.schema.json", &json);

    let raw_schema: Value = serde_json::from_str(
        &fs::read_to_string(schema_dir().join("views/distribution-summary.schema.json")).unwrap(),
    )
    .unwrap();
    assert_source_of_truth_declared(&raw_schema, "distribution-summary.schema.json");
}

#[test]
fn distribution_details_matches_schema() {
    use elohim_storage::views::{
        DeviceArchetype, DistributionDetails, DistributionSummary, DiversityHint, FetchSource,
        ProjectorIdentity, ReachClass, ReplicaHealth, ReplicaPeer,
    };

    let summary = DistributionSummary {
        replica_count: 12,
        replica_target: 14,
        replica_health: ReplicaHealth::Healthy,
        projector_count: 2,
        reach_class: ReachClass::Public,
        diversity_hint: DiversityHint::RegionMetro(vec!["us-central".into(), "eu-west".into()]),
        this_fetch_source: FetchSource::ProjectedViaDoorway,
        last_verified_seconds: 420,
        my_role: None,
        reciprocity_hint: None,
    };

    let sample = DistributionDetails {
        summary,
        replica_peers: vec![ReplicaPeer {
            peer_id: "12D3KooWReplica1".into(),
            device_archetype: DeviceArchetype::Desktop,
            last_seen_seconds: 30,
            hop_hint: Some(2),
            household_id: Some("household-mathew".into()),
            region_tier: Some("us-central".into()),
        }],
        projector_identities: vec![ProjectorIdentity {
            doorway_hostname: "matthew.elohim.host".into(),
            last_ack_seconds: 5,
            region_tier: Some("us-central".into()),
        }],
        placement_gaps: vec![],
        recent_projection_events: vec![],
        commitment_references: Some(vec!["bafy-commit-1".into()]),
    };

    let json = serde_json::to_value(&sample).unwrap();
    validate_against_schema("views/distribution-details.schema.json", &json);

    let raw_schema: Value = serde_json::from_str(
        &fs::read_to_string(schema_dir().join("views/distribution-details.schema.json")).unwrap(),
    )
    .unwrap();
    assert_source_of_truth_declared(&raw_schema, "distribution-details.schema.json");
}

#[test]
fn replica_peer_matches_schema() {
    use elohim_storage::views::{DeviceArchetype, ReplicaPeer};

    let sample = ReplicaPeer {
        peer_id: "12D3KooWReplica1".into(),
        device_archetype: DeviceArchetype::Mobile,
        last_seen_seconds: 90,
        hop_hint: None,
        household_id: None,
        region_tier: None,
    };

    let json = serde_json::to_value(&sample).unwrap();
    validate_against_schema("views/replica-peer.schema.json", &json);
}

#[test]
fn projector_identity_matches_schema() {
    use elohim_storage::views::ProjectorIdentity;

    let sample = ProjectorIdentity {
        doorway_hostname: "shem.elohim.host".into(),
        last_ack_seconds: 12,
        region_tier: None,
    };

    let json = serde_json::to_value(&sample).unwrap();
    validate_against_schema("views/projector-identity.schema.json", &json);
}

#[test]
fn my_cluster_view_matches_schema() {
    use elohim_storage::views::{
        DeviceArchetype, DeviceSummary, DeviceTotals, Freshness, FreshnessState, MyClusterView,
    };

    let sample = MyClusterView {
        agent_cid: "agent_abc123".into(),
        devices: vec![DeviceSummary {
            peer_id: "12D3KooWMatthewLaptop".into(),
            archetype: DeviceArchetype::Desktop,
            display_name: Some("Matthew's laptop".into()),
            online: true,
            freshness: Freshness {
                state: FreshnessState::Live,
                stale_since_ms: None,
            },
            storage_used_bytes: Some(18_400_000_000),
            storage_total_bytes: Some(250_000_000_000),
            memory_used_bytes: None,
            memory_total_bytes: None,
            hosting_count: Some(1247),
            projecting_count: Some(802),
            beacon_age_ms: Some(0),
        }],
        totals: DeviceTotals {
            storage_used_bytes: 25_200_000_000,
            storage_total_bytes: 298_000_000_000,
            external_committed_bytes: 14_800_000_000,
            reciprocity_net_bytes: 5_200_000_000,
        },
        freshness: Freshness {
            state: FreshnessState::Live,
            stale_since_ms: None,
        },
    };

    let json = serde_json::to_value(&sample).unwrap();
    validate_against_schema("views/my-cluster-view.schema.json", &json);
}

#[test]
fn freshness_offline_matches_schema() {
    use elohim_storage::views::{Freshness, FreshnessState};

    let sample = Freshness {
        state: FreshnessState::Offline,
        stale_since_ms: Some(120_000),
    };

    let json = serde_json::to_value(&sample).unwrap();
    validate_against_schema("views/freshness.schema.json", &json);
}

#[test]
fn distribution_summary_with_diversity_none_matches_schema() {
    use elohim_storage::views::{
        DiversityHint, DistributionSummary, FetchSource, ReachClass, ReplicaHealth,
    };

    let sample = DistributionSummary {
        replica_count: 1,
        replica_target: 3,
        replica_health: ReplicaHealth::AtRisk,
        projector_count: 0,
        reach_class: ReachClass::Private,
        diversity_hint: DiversityHint::None,
        this_fetch_source: FetchSource::LocalPantry,
        last_verified_seconds: 0,
        my_role: None,
        reciprocity_hint: None,
    };

    let json = serde_json::to_value(&sample).unwrap();
    validate_against_schema("views/distribution-summary.schema.json", &json);
}

#[test]
fn attention_tending_with_negative_tended_at_rejected_by_schema() {
    // Hand-crafted Value — NOT constructed through the Rust struct — so this
    // proves the JSON Schema minimum: 0 constraint on tendedAt items fires
    // independently of Rust's type system (Vec<u64> already blocks negatives
    // at compile time; this test guards the schema layer end-to-end).
    let bad_instance = serde_json::json!({
        "filterSubject": {},
        "classification": "values-forward",
        "ttlSeconds": 3600,
        "tendedAt": [-1],
        "context": {},
        "signedBy": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
        "signature": "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB="
        // tendedAt: [-1] — negative timestamp; schema minimum: 0 must reject
    });

    let schema = load_schema("p2p/attention-tending.schema.json");
    let validator =
        jsonschema::validator_for(&schema).expect("attention-tending schema should compile");

    let has_errors = validator.iter_errors(&bad_instance).next().is_some();
    assert!(
        has_errors,
        "schema should reject tendedAt: [-1] (items minimum: 0 violated)"
    );
}
