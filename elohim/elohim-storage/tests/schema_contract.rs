//! Schema contract tests — validates that Rust serialization matches
//! the JSON Schema source of truth in elohim/sdk/schemas/v1/views/.
//!
//! These tests catch drift between Rust struct changes and the schema
//! contract. If a field is renamed, added, or removed in Rust without
//! updating the schema (or vice versa), these tests fail.
//!
//! The harness inlines `$ref` before compiling because the jsonschema
//! crate doesn't resolve file-based references automatically.

// `DOORWAY_CAP_ENV_LOCK` (defined later) serializes env-var mutations across
// `#[tokio::test]` functions, each of which runs on its own current-thread
// runtime. The lock contention is therefore across OS threads, not across
// tasks within a single runtime — the deadlock pattern `await_holding_lock`
// guards against doesn't apply. Switching to `tokio::sync::Mutex` would be a
// task-level mutex (wrong granularity for cross-runtime env-var contention).
#![allow(clippy::await_holding_lock)]

use elohim_storage::p2p::acquisition::PullStatusInfo;
use elohim_storage::p2p::projection_reconcile::ProjectionReconcileStatus;
use elohim_storage::p2p::replication::ReplicationStatus;
use elohim_storage::p2p::DrainStatusInfo;
use elohim_storage::services::provide_loop_status::ProvideLoopStatus;
use elohim_storage::P2PStatusInfo;
use elohim_views::{
    CollabAgreementStatus, CollabAgreementView, CollabCollectiveView, CollabMembershipRole,
    CollabMembershipView, CollabQahalView, DeclaredShare, ElohimTier, GovernanceTerms,
    LensBindingView, LensMarketView, MemberKind, ShareAllocation, ShareAllocationForm,
};
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

    // NB: both "manifests" (plural — bootstrap payloads) and "manifest" (singular —
    // floor/policy schemas like tending-policy-floor, standing-policy-floor) must be
    // scanned. The singular dir holds the `epr:schema:manifest:*` $id targets that
    // manifest-payloads schemas $ref; omitting it leaves those refs unresolved and the
    // jsonschema compile fails with "Unknown scheme epr".
    for subdir in &[
        "enums",
        "views",
        "manifests",
        "manifest",
        "objects",
        "manifest-payloads",
    ] {
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
                    // Split file portion from JSON Pointer fragment (e.g. "file.json#/definitions/X")
                    let (file_part, fragment) = match ref_path.split_once('#') {
                        Some((file, frag)) => (file, Some(frag)),
                        None => (ref_path.as_str(), None),
                    };
                    if let Some(referenced) = refs.get(file_part) {
                        // Inline: take the referenced schema, strip meta fields, inline its refs too
                        let mut inlined = referenced.clone();
                        if let Value::Object(ref mut obj) = inlined {
                            obj.remove("$id");
                            obj.remove("$schema");
                        }
                        // If a JSON Pointer fragment is present, walk into it
                        if let Some(frag) = fragment {
                            let pointer = if frag.starts_with('/') {
                                frag.to_string()
                            } else {
                                format!("/{frag}")
                            };
                            if let Some(sub) = inlined.pointer(&pointer) {
                                return inline_refs(&sub.clone(), refs);
                            }
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

// ── Content HEAD (notary-authority, HEAD-election Plan C3 / Leg 3) ──

#[test]
fn content_head_view_matches_schema() {
    use elohim_views::ContentHeadView;

    // Full variant: explicitly-declared head with an anchor + serving blob.
    let declared = ContentHeadView {
        content_id: "epr:lamad-concept-001".to_string(),
        head_action_hash: "uhCkkDECLAREDHEAD0123456789012345678901234567890123456789012"
            .to_string(),
        declared: true,
        dht_anchor_hash: Some(
            "uhCkkANCHOR0123456789012345678901234567890123456789012345678".to_string(),
        ),
        trust: "notarized".to_string(),
        blob_hash: Some("sha256-abc123".to_string()),
        updated_at: Some("2026-07-04T00:00:00Z".to_string()),
    };
    let json = serde_json::to_value(&declared).unwrap();
    validate_against_schema("views/content-head.schema.json", &json);
    assert_source_of_truth_declared(
        &load_schema("views/content-head.schema.json"),
        "content-head.schema.json",
    );

    // Anchor-only fallback: declared=false, optional serving fields null.
    let anchor_only = ContentHeadView {
        content_id: "epr:lamad-concept-002".to_string(),
        head_action_hash: "uhCkkANCHOR0123456789012345678901234567890123456789012345678"
            .to_string(),
        declared: false,
        dht_anchor_hash: Some(
            "uhCkkANCHOR0123456789012345678901234567890123456789012345678".to_string(),
        ),
        trust: "notarized".to_string(),
        blob_hash: None,
        updated_at: None,
    };
    let json = serde_json::to_value(&anchor_only).unwrap();
    validate_against_schema("views/content-head.schema.json", &json);
}

// ── Lens Market (plural Mishpat lenses over an EPR) ──────────────

#[test]
fn lens_market_view_matches_schema() {
    let view = LensMarketView {
        epr_scope: "epr:lamad-spa".to_string(),
        lenses: vec![
            LensBindingView {
                lens_cid: "uhCEkGeorgist".to_string(),
                school: "georgist".to_string(),
                role: "lens".to_string(),
                telos_summary: "tax unearned land rent".to_string(),
                affinity_in_context: 3,
                current_verdict: Some("rent-bearing".to_string()),
                valid: true,
            },
            LensBindingView {
                lens_cid: "uhCEkBeerian".to_string(),
                school: "beerian".to_string(),
                role: "lens".to_string(),
                telos_summary: "keep the loop regulable".to_string(),
                affinity_in_context: 0,
                current_verdict: None, // warm, not yet fired in this context
                valid: true,
            },
        ],
        contention_index: 1.0,
        regime_status: "breached".to_string(),
        open_bounty_cid: Some("uhCEkBounty".to_string()),
        computed_at: "2026-06-27T00:00:00Z".to_string(),
    };
    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/lens-market-view.schema.json", &json);
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
        pull: Some(PullStatusInfo {
            total: 3,
            fetched: 2,
            pending: 1,
            failed: 0,
            caught_up: false,
        }),
        projection_reconcile: Some(ProjectionReconcileStatus {
            pending: 1,
            completed: 4,
            failed: 0,
            caught_up: false,
            peers_asked: 2,
            divergent_anchor: 1,
            healed_total: 9,
            sweeps: 3,
        }),
        sync_paused: false,
        dedup_unique_len: 42,
        dedup_total_seen: 55,
        reconcile_passes_total: 7,
        kicks_fired_total: 3,
        placement_gaps_emitted_total: 12,
        provide_loop: Some(ProvideLoopStatus {
            self_cid_source: "derived-libp2p-peer-id".to_string(),
            active: true,
            reanchor_pending: 0,
            reanchor_completed: 8,
            reanchor_failed: 1,
            reanchor_caught_up: true,
        }),
        // Dual-stack: exercise the additive irohNodeId serialization against the
        // schema (64-char hex NodeId).
        iroh_node_id: Some(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        ),
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
        pull: None,
        projection_reconcile: None,
        sync_paused: true,
        dedup_unique_len: 0,
        dedup_total_seen: 0,
        reconcile_passes_total: 0,
        kicks_fired_total: 0,
        placement_gaps_emitted_total: 0,
        provide_loop: None,
        // libp2p-only node: irohNodeId omitted from the wire (skip_serializing_if).
        iroh_node_id: None,
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

// ── EPR pull status (Slice 2b T13) ──────────────────────────────
#[test]
fn epr_pull_status_view_matches_schema() {
    use elohim_views::acquisition::EprPullStatusView;
    let view = EprPullStatusView {
        epr_id: "epr:bafyHead".to_string(),
        total: Some(5),
        fetched: 3,
        pending: 2,
        failed: 0,
        caught_up: Some(false),
    };
    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/epr-pull-status.schema.json", &json);
}

#[test]
fn epr_pull_status_view_null_total_serializes() {
    // total/caughtUp are Option (None = "cannot compute" tri-state, spec §4.3).
    use elohim_views::acquisition::EprPullStatusView;
    let view = EprPullStatusView {
        epr_id: "epr:bafyHead".to_string(),
        total: None,
        fetched: 0,
        pending: 0,
        failed: 0,
        caught_up: None,
    };
    let json = serde_json::to_value(&view).unwrap();
    // total and caughtUp present as null — schema allows ["integer","null"] / ["boolean","null"].
    assert!(json.get("total").unwrap().is_null());
    assert!(json.get("caughtUp").unwrap().is_null());
    validate_against_schema("views/epr-pull-status.schema.json", &json);
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
        render_capability: None,
        extensions: None,
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
        render_capability: None,
        extensions: None,
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
        render_capability: None,
        extensions: None,
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

// ── Weave View ─────────────────────────────────────────────────

#[test]
fn weave_view_matches_schema() {
    use elohim_storage::{ComputeTriptych, RegionalDistributionView, WeaveView};

    // Minimal required-only variant (cluster_capacity absent → rsCoverage absent).
    let minimal = WeaveView {
        placement_gap_count: 0,
        rs_coverage: None,
        cluster_capacity: None,
        tier_occupancy: None,
        region_occupancy: None,
        measured_at: "2026-06-20T12:00:00Z".to_string(),
    };
    let json = serde_json::to_value(&minimal).unwrap();
    // Optional fields must be absent when None (skip_serializing_if).
    assert!(
        !json.as_object().unwrap().contains_key("rsCoverage"),
        "rsCoverage must be absent when None"
    );
    validate_against_schema("views/weave-view.schema.json", &json);

    // Full variant: all SELECTED lenses populated (tier_occupancy stays None — the
    // deliberate scope deferral, never fabricated). region_occupancy is populated so
    // this contract actually exercises the field `build_weave_view` now ships on the
    // wire (it emits `Some(regions)`); validating only its absence would leave a
    // malformed regionOccupancy sub-schema uncaught.
    // Use all-Some ComputeTriptych fields to avoid null serialization in clusterCapacity.
    let full = WeaveView {
        placement_gap_count: 3,
        rs_coverage: Some(0.75),
        cluster_capacity: Some(ComputeTriptych {
            free: Some(1_000_000_000),
            used: Some(500_000_000),
            stewarded: Some(200_000_000),
        }),
        tier_occupancy: None,
        region_occupancy: Some(RegionalDistributionView {
            local: 0,
            regional: 0,
            global: 1,
            unknown: 1,
        }),
        measured_at: "2026-06-20T12:00:00Z".to_string(),
    };
    let json = serde_json::to_value(&full).unwrap();
    assert!(
        json.as_object().unwrap().contains_key("regionOccupancy"),
        "regionOccupancy must be present when Some (the lens build_weave_view selects)"
    );
    validate_against_schema("views/weave-view.schema.json", &json);
}

#[test]
fn mishpat_commitment_view_matches_schema() {
    use elohim_storage::views::JsonVal;
    use elohim_storage::MishpatCommitmentView;

    // Minimal required-only variant: bounds + dhtAnchorHash absent (an un-notarized
    // row that carried no parseable bounds — both optional fields must be OMITTED,
    // never serialized as null).
    let minimal = MishpatCommitmentView {
        cid: "uhCEk-commitment-1".to_string(),
        action: "delegates-compute".to_string(),
        scope: "republish-epr".to_string(),
        provider: "uhCAk-provider".to_string(),
        recipient: "uhCAk-recipient".to_string(),
        bounds: None,
        valid_from: "2026-05-28T00:00:00Z".to_string(),
        valid_until: "2026-08-26T00:00:00Z".to_string(),
        state: "proposed".to_string(),
        dht_anchor_hash: None,
        created_at: "2026-06-22T00:00:00Z".to_string(),
    };
    let json = serde_json::to_value(&minimal).unwrap();
    assert!(
        !json.as_object().unwrap().contains_key("bounds"),
        "bounds must be absent when None (not null)"
    );
    assert!(
        !json.as_object().unwrap().contains_key("dhtAnchorHash"),
        "dhtAnchorHash must be absent when None (the un-notarized row contract)"
    );
    validate_against_schema("views/mishpat-commitment-view.schema.json", &json);

    // Full variant: bounds (opaque action-specific envelope) + dhtAnchorHash present.
    let full = MishpatCommitmentView {
        cid: "uhCEk-commitment-2".to_string(),
        action: "delegates-compute".to_string(),
        scope: "republish-epr".to_string(),
        provider: "uhCAk-provider".to_string(),
        recipient: "uhCAk-recipient".to_string(),
        bounds: Some(JsonVal(serde_json::json!({
            "maxCallsPerHour": 100,
            "reachCeiling": "commons"
        }))),
        valid_from: "2026-05-28T00:00:00Z".to_string(),
        valid_until: "2026-08-26T00:00:00Z".to_string(),
        state: "active".to_string(),
        dht_anchor_hash: Some("uhCkk-anchor".to_string()),
        created_at: "2026-06-22T00:00:00Z".to_string(),
    };
    let json = serde_json::to_value(&full).unwrap();
    assert!(
        json.as_object().unwrap().contains_key("bounds"),
        "bounds must be present when Some"
    );
    assert!(
        json.as_object().unwrap().contains_key("dhtAnchorHash"),
        "dhtAnchorHash must be present when Some (notarized row)"
    );
    validate_against_schema("views/mishpat-commitment-view.schema.json", &json);
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
        FeltFloorView, FeltStatusView, OnlinePeersView, PlacementGapView, RegionalDistributionView,
        ResilienceSnapshotDetailsView, ResilienceSnapshotView, StewardingCollectiveEntry,
    };

    let view = ResilienceSnapshotView {
        content_id: "content-abc-001".to_string(),
        distribution_state: "measured".to_string(),
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
                    intra_hub_peers: Some(2),
                },
                StewardingCollectiveEntry {
                    id: "church-bethel".to_string(),
                    kind: "church".to_string(),
                    label: None,
                    intra_hub_peers: Some(1),
                },
            ],
            online_peers: OnlinePeersView { live: 5, known: 7 },
            health_score: 0.8,
        }),
        felt_status: Some(FeltStatusView {
            headline: "Held by 2 households: Matthew's Home, Bethel Church".to_string(),
            reassurance: "watching".to_string(),
            held_by: vec![
                StewardingCollectiveEntry {
                    id: "household-matthew".to_string(),
                    kind: "household".to_string(),
                    label: Some("Matthew's Home".to_string()),
                    intra_hub_peers: Some(2),
                },
                StewardingCollectiveEntry {
                    id: "church-bethel".to_string(),
                    kind: "church".to_string(),
                    label: None,
                    intra_hub_peers: Some(1),
                },
            ],
            floor: FeltFloorView {
                tier: "standard".to_string(),
                tier_declared: false,
                wants_households: 3,
                has_households: 2,
            },
            suggested_action: None,
        }),
        coverage_shortfall: None,
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
        distribution_state: "unmeasured".to_string(),
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
        felt_status: None,
        coverage_shortfall: None,
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
    assert!(
        !json.as_object().unwrap().contains_key("feltStatus"),
        "feltStatus must be absent when None"
    );
    assert!(
        !json.as_object().unwrap().contains_key("coverageShortfall"),
        "coverageShortfall must be absent when None (missing ≡ not-selected)"
    );
    validate_against_schema("views/resilience-snapshot-view.schema.json", &json);
}

#[test]
fn resilience_snapshot_view_with_coverage_shortfall_validates() {
    use elohim_storage::{RegionalDistributionView, ResilienceSnapshotView};

    // Graph branch: coverage_shortfall populated when short of the floor.
    let view = ResilienceSnapshotView {
        content_id: "content-shortfall-001".to_string(),
        distribution_state: "measured".to_string(),
        stewarding_collectives: 2,
        commitment_backed_collectives: 2,
        diversity_score: 0.67,
        regional_distribution: RegionalDistributionView {
            local: 0,
            regional: 0,
            global: 0,
            unknown: 2,
        },
        placement_gaps: vec![],
        protection_status: "partial".to_string(),
        reciprocating_collectives: None,
        details: None,
        felt_status: None,
        coverage_shortfall: Some(1), // 1 slot short of floor=3
    };

    let json = serde_json::to_value(&view).unwrap();
    // coverageShortfall must be present and equal 1
    assert_eq!(
        json["coverageShortfall"],
        serde_json::Value::Number(1.into()),
        "coverageShortfall must serialize when Some"
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
fn epr_raw_view_conforms() {
    use elohim_storage::{EprCouplingView, EprRawView};
    let v = EprRawView {
        cid: "bafyreib2vq7fztfnmgzrmo7q5jnfkdvxkfxpvsjmesxrqjzqxkzqzqzqa".into(),
        coupling: EprCouplingView {
            knowledge: Some(
                "bafyreib2vq7knowledge01234567890123456789012345678901234567890".into(),
            ),
            value: None,
            governance: Some(
                "bafyreib2vq7governance01234567890123456789012345678901234567890".into(),
            ),
        },
        neighborhood_degree: 2,
        reverse_part_of: vec![
            "bafyreib2vq7citingatom01234567890123456789012345678901234567890".into(),
        ],
    };
    let json = serde_json::to_value(&v).unwrap();
    validate_against_schema("views/epr-raw-view.schema.json", &json);
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
        event: None,
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

/// Build a `ContentView` with every required field populated. `server_blob_hash`
/// is the field under test (SSR row-collapse T1) — varied by the caller.
fn sample_content_view(server_blob_hash: Option<String>) -> elohim_views::ContentView {
    elohim_views::ContentView {
        id: "elohim-host-landing".into(),
        h_app_id: "lamad".into(),
        title: "Elohim Host Landing".into(),
        description: None,
        content_type: "concept".into(),
        content_format: "spa-bundle".into(),
        blob_hash: Some(format!("sha256-{}", "ab".repeat(32))),
        server_blob_hash,
        blob_cid: None,
        content_size_bytes: None,
        metadata: None,
        reach: "commons".into(),
        validation_status: "valid".into(),
        created_by: None,
        created_at: "2026-06-26T00:00:00Z".into(),
        updated_at: "2026-06-26T00:00:00Z".into(),
        content_body: None,
        dht_anchor_hash: None,
        // No dht_anchor_hash and no p2p_published marker on this fixture → the
        // REQ-F10 trust label is "unconfirmed" (amber). Schema requires `trust`.
        trust: "unconfirmed".into(),
    }
}

/// Round-trip: a `ContentView` carrying `serverBlobHash` conforms to the schema,
/// and the wire key is exactly `serverBlobHash` (camelCase), mirroring `blobHash`.
/// `content-view.schema.json` is `additionalProperties: false`, so this also
/// proves the schema declares the new optional field.
#[test]
fn content_view_with_server_blob_hash_conforms() {
    let v = sample_content_view(Some(format!("sha256-{}", "cd".repeat(32))));
    let json = serde_json::to_value(&v).unwrap();
    assert!(
        json.get("serverBlobHash")
            .and_then(|x| x.as_str())
            .is_some(),
        "serverBlobHash must surface on the wire as a camelCase string"
    );
    assert!(
        json.get("server_blob_hash").is_none(),
        "snake_case server_blob_hash must NOT leak to the wire"
    );
    validate_against_schema("views/content-view.schema.json", &json);
}

/// Absent `serverBlobHash` is the normal pre-deploy state — it serializes to
/// `null` and must still conform (the field is optional/nullable, not required).
#[test]
fn content_view_without_server_blob_hash_conforms() {
    let v = sample_content_view(None);
    let json = serde_json::to_value(&v).unwrap();
    assert_eq!(
        json.get("serverBlobHash"),
        Some(&serde_json::Value::Null),
        "absent serverBlobHash serializes to null"
    );
    validate_against_schema("views/content-view.schema.json", &json);
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
        // Phase 1: light-up-topology view schemas
        "views/distribution-summary.schema.json",
        "views/distribution-details.schema.json",
        "views/replica-peer.schema.json",
        "views/projector-identity.schema.json",
        "views/diversity-hint.schema.json",
        "views/my-cluster-view.schema.json",
        "views/freshness.schema.json",
        "views/peer-topology-view.schema.json",
        "views/peer-household-edge.schema.json",
        "views/reciprocity-view.schema.json",
        "views/reciprocity-row.schema.json",
        "views/doorway-dashboard-view.schema.json",
        "views/dashboard-steward.schema.json",
        "views/dashboard-federation-peer.schema.json",
        "views/projection-coverage.schema.json",
        "views/public-surface-state.schema.json",
        "views/view-slice.schema.json",
        // Intent schemas — same convention: description must declare source of truth
        "intents/lamad-event-intent.schema.json",
        // M-REA-3: RecognitionParticipationIntent wire shape + extended standing-policy payload
        "intents/recognition-participation-intent.schema.json",
        // M-POLICY-1: AccumulationStatus view + standing-policy payload schemas
        "views/accumulation-status.schema.json",
        "manifest-payloads/standing-policy.schema.json",
        // M-POLICY-2: MechanismSelection view + pillar-projection payload schemas
        "views/mechanism-selection.schema.json",
        "manifest-payloads/pillar-projection.schema.json",
        // M-AGGR-3: ContentEngagementStats projection (Category C operational — EconomicEvent stream)
        "views/content-engagement-stats-view.schema.json",
        // Sprint 2 Task 1: BoundsValidationResult + StandingScore views
        "views/bounds-validation-result-view.schema.json",
        "views/standing-score-view.schema.json",
        // Sprint 3: per-peer and hub storage capacity projections
        "views/peer-capacity-view.schema.json",
        "views/hub-capacity-view.schema.json",
        // Native content-graph seam (Task A9): multi-provenance composite read
        "views/content-graph.schema.json",
        // Slice 2b T13: per-EPR pull progress (own-node only)
        "views/epr-pull-status.schema.json",
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
        subject_human_id: "human-matthew".into(),
        revoked_key: "uhCAkABCD1234567890".into(),
        reason: "compromised".into(),
        trigger_type: "voluntary".into(),
        initiated_by_cid: "human-matthew".into(),
        required_votes: 1,
        current_votes: 1,
        threshold_reached: true,
        effective_at: Some("2026-04-24T00:00:00Z".into()),
        derived_compromise_at: None,
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
        subject_human_id: "human-jessica".into(),
        revoked_key: "uhCAkXYZ987654321".into(),
        reason: "stolen".into(),
        trigger_type: "steward_vote".into(),
        initiated_by_cid: "human-timothy".into(),
        required_votes: 3,
        current_votes: 0,
        threshold_reached: false,
        effective_at: None,
        derived_compromise_at: Some("2026-04-23T22:00:00Z".into()),
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
        household_id: Some("household-dowell".into()),
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
        household_id: None,
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
        household_id: Some("household-dowell".into()),
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
        DistributionSummary, DiversityHint, FetchSource, MyRole, ProjectionTier, ReachClass,
        ReplicaHealth,
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
        projection_tier: ProjectionTier::Local, // T15: computed
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
        DeviceArchetype, DistributionDetails, DistributionSummary, DiversityHint,
        FaultDomainDiversity, FetchSource, ProjectionTier, ProjectorIdentity, ReachClass,
        ReplicaHealth, ReplicaPeer,
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
        projection_tier: ProjectionTier::Local, // T15: computed
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
            shards_held: None,        // T15: computed
            shards_by_encoding: None, // T15: computed
        }],
        projector_identities: vec![ProjectorIdentity {
            doorway_hostname: "matthew.elohim.host".into(),
            last_ack_seconds: 5,
            region_tier: Some("us-central".into()),
        }],
        placement_gaps: vec![],
        recent_projection_events: vec![],
        commitment_references: Some(vec!["bafy-commit-1".into()]),
        replication_commitments: Vec::new(), // T15: computed
        fault_domain_diversity: FaultDomainDiversity::default(), // T15: computed
    };

    let json = serde_json::to_value(&sample).unwrap();
    validate_against_schema("views/distribution-details.schema.json", &json);

    let raw_schema: Value = serde_json::from_str(
        &fs::read_to_string(schema_dir().join("views/distribution-details.schema.json")).unwrap(),
    )
    .unwrap();
    assert_source_of_truth_declared(&raw_schema, "distribution-details.schema.json");
}

// ── PlacementGapRow (hub-abstract) ─────────────────────────────

#[test]
fn placement_gap_row_schema_matches_struct() {
    use elohim_storage::views::{PlacementGapKind, PlacementGapRow, PlacementGapShortfall};

    // Full row with remediation hint
    let row = PlacementGapRow {
        kind: PlacementGapKind::HubDiversity,
        content_id: "content-x".to_string(),
        shortfall: PlacementGapShortfall {
            target: 3,
            observed: 1,
        },
        remediation: Some("Recruit a replica in another hub".to_string()),
    };
    let json = serde_json::to_value(&row).unwrap();
    validate_against_schema("views/placement-gap-row.schema.json", &json);
}

#[test]
fn placement_gap_row_all_kinds_validate() {
    use elohim_storage::views::{PlacementGapKind, PlacementGapRow, PlacementGapShortfall};

    for kind in [
        PlacementGapKind::HubDiversity,
        PlacementGapKind::ReplicaCount,
        PlacementGapKind::ReachClass,
    ] {
        let row = PlacementGapRow {
            kind,
            content_id: "content-y".to_string(),
            shortfall: PlacementGapShortfall {
                target: 5,
                observed: 2,
            },
            remediation: None,
        };
        let json = serde_json::to_value(&row).unwrap();
        validate_against_schema("views/placement-gap-row.schema.json", &json);
    }
}

#[test]
fn placement_gap_row_schema_source_of_truth_declared() {
    let raw_schema: Value = serde_json::from_str(
        &fs::read_to_string(schema_dir().join("views/placement-gap-row.schema.json")).unwrap(),
    )
    .unwrap();
    assert_source_of_truth_declared(&raw_schema, "placement-gap-row.schema.json");
}

#[test]
fn distribution_details_placement_gaps_typed() {
    // Verify that DistributionDetails serializes with typed PlacementGapRow,
    // not open-shape JsonVal — the schema now refs placement-gap-row.schema.json.
    use elohim_storage::views::{
        DeviceArchetype, DistributionDetails, DistributionSummary, DiversityHint,
        FaultDomainDiversity, FetchSource, PlacementGapKind, PlacementGapRow,
        PlacementGapShortfall, ProjectionTier, ProjectorIdentity, ReachClass, ReplicaHealth,
        ReplicaPeer,
    };

    let summary = DistributionSummary {
        replica_count: 4,
        replica_target: 5,
        replica_health: ReplicaHealth::AtRisk,
        projector_count: 1,
        reach_class: ReachClass::Trusted,
        diversity_hint: DiversityHint::RegionMetro(vec!["us-central".into()]),
        this_fetch_source: FetchSource::ProjectedViaDoorway,
        last_verified_seconds: 60,
        my_role: None,
        reciprocity_hint: None,
        projection_tier: ProjectionTier::Local, // T15: computed
    };

    let details = DistributionDetails {
        summary,
        replica_peers: vec![ReplicaPeer {
            peer_id: "12D3KooWReplica2".into(),
            device_archetype: DeviceArchetype::Desktop,
            last_seen_seconds: 10,
            hop_hint: None,
            household_id: None,
            region_tier: None,
            shards_held: None,        // T15: computed
            shards_by_encoding: None, // T15: computed
        }],
        projector_identities: vec![ProjectorIdentity {
            doorway_hostname: "test.elohim.host".into(),
            last_ack_seconds: 2,
            region_tier: None,
        }],
        placement_gaps: vec![PlacementGapRow {
            kind: PlacementGapKind::ReplicaCount,
            content_id: "content-z".to_string(),
            shortfall: PlacementGapShortfall {
                target: 5,
                observed: 4,
            },
            remediation: None,
        }],
        recent_projection_events: vec![],
        commitment_references: None,
        replication_commitments: Vec::new(), // T15: computed
        fault_domain_diversity: FaultDomainDiversity::default(), // T15: computed
    };

    let json = serde_json::to_value(&details).unwrap();
    validate_against_schema("views/distribution-details.schema.json", &json);
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
        shards_held: None,        // T15: computed
        shards_by_encoding: None, // T15: computed
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
            compute: None,
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
fn compute_triptych_in_device_summary_matches_schema() {
    use elohim_storage::views::{
        ComputeTriptych, DeviceArchetype, DeviceSummary, DeviceTotals, Freshness, FreshnessState,
        MyClusterView,
    };

    let sample = MyClusterView {
        agent_cid: "agent_triptych_test".into(),
        devices: vec![DeviceSummary {
            peer_id: "12D3KooWTriptychPeer".into(),
            archetype: DeviceArchetype::Node,
            display_name: None,
            online: true,
            freshness: Freshness {
                state: FreshnessState::Live,
                stale_since_ms: None,
            },
            storage_used_bytes: Some(5_000_000_000),
            storage_total_bytes: Some(500_000_000_000),
            memory_used_bytes: None,
            memory_total_bytes: None,
            hosting_count: Some(300),
            projecting_count: Some(150),
            beacon_age_ms: Some(250),
            compute: Some(ComputeTriptych {
                free: Some(495_000_000_000),
                used: Some(5_000_000_000),
                stewarded: Some(2_000_000_000),
            }),
        }],
        totals: DeviceTotals {
            storage_used_bytes: 5_000_000_000,
            storage_total_bytes: 500_000_000_000,
            external_committed_bytes: 2_000_000_000,
            reciprocity_net_bytes: 1_000_000_000,
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
fn hub_compute_aggregate_view_matches_schema() {
    use elohim_storage::views::{ComputeTriptych, HubComputeAggregateView, HubKind};

    // Household hub — 3 devices, full triptych populated.
    let populated = HubComputeAggregateView {
        hub_id: "hub:household:matthew-jessica-james".into(),
        hub_kind: HubKind::Dwelling,
        display_label: Some("Matthew's household".into()),
        compute: Some(ComputeTriptych {
            free: Some(5_368_709_120),
            used: Some(10_737_418_240),
            stewarded: Some(12_884_901_888),
        }),
        member_device_count: 3,
    };
    validate_against_schema(
        "views/hub-compute-aggregate.schema.json",
        &serde_json::to_value(&populated).unwrap(),
    );

    // Fresh node — no samples yet, no commitments. The empty-state shape.
    let empty = HubComputeAggregateView {
        hub_id: "hub:household:fresh-node".into(),
        hub_kind: HubKind::Computed,
        display_label: None,
        compute: None,
        member_device_count: 1,
    };
    validate_against_schema(
        "views/hub-compute-aggregate.schema.json",
        &serde_json::to_value(&empty).unwrap(),
    );

    // Collective hub — wire-shape sanity for the qahal-scoped variant.
    let collective = HubComputeAggregateView {
        hub_id: "hub:collective:shem-research-cluster".into(),
        hub_kind: HubKind::Collective,
        display_label: Some("Shem research cluster".into()),
        compute: Some(ComputeTriptych {
            free: Some(214_748_364_800),
            used: Some(107_374_182_400),
            stewarded: Some(53_687_091_200),
        }),
        member_device_count: 6,
    };
    validate_against_schema(
        "views/hub-compute-aggregate.schema.json",
        &serde_json::to_value(&collective).unwrap(),
    );
}

#[test]
fn peer_topology_view_matches_schema() {
    use elohim_storage::views::{
        Freshness, FreshnessState, PeerHouseholdEdge, PeerTopologyView, ResilienceCliff,
    };

    let sample = PeerTopologyView {
        agent_cid: "agent_abc123".into(),
        edges: vec![PeerHouseholdEdge {
            household_id: "household-jessica".into(),
            display_name: Some("Jessica".into()),
            online: true,
            last_sync_sec: Some(45),
            my_cids_hosted_by_them: 412,
            their_cids_hosted_by_me: 380,
            net_diff: Some(-32),
            is_critical_for_me: Some(false),
            i_am_critical_for_them: Some(true),
        }],
        reciprocation_count: 3,
        resilience_cliffs: vec![ResilienceCliff {
            household_id: "household-shem".into(),
            sole_replica_cid_count: 7,
        }],
        freshness: Freshness {
            state: FreshnessState::Live,
            stale_since_ms: None,
        },
    };

    let json = serde_json::to_value(&sample).unwrap();
    validate_against_schema("views/peer-topology-view.schema.json", &json);
}

#[test]
fn reciprocity_view_matches_schema() {
    use elohim_storage::views::{ReciprocityRow, ReciprocityView};

    let sample = ReciprocityView {
        agent_cid: "agent_abc123".into(),
        inflow: vec![ReciprocityRow {
            counterparty_household_id: "household-jessica".into(),
            display_name: Some("Jessica".into()),
            committed_bytes: 5_000_000_000,
            delivered_bytes: 4_750_000_000,
            honored_percent: 0.95,
            online: Some(true),
        }],
        outflow: vec![ReciprocityRow {
            counterparty_household_id: "household-shem".into(),
            display_name: None,
            committed_bytes: 8_000_000_000,
            delivered_bytes: 8_400_000_000,
            honored_percent: 1.05,
            online: None,
        }],
        net_hosted_bytes: -3_650_000_000,
        capacity_available_bytes: 12_500_000_000,
    };

    let json = serde_json::to_value(&sample).unwrap();
    validate_against_schema("views/reciprocity-view.schema.json", &json);
}

#[test]
fn view_slice_cluster_matches_schema() {
    use elohim_storage::views::{Freshness, FreshnessState, JsonVal, ViewKind, ViewSlice};

    let sample = ViewSlice {
        peer_id: "12D3KooWMatthewLaptop".into(),
        view_kind: ViewKind::Cluster,
        freshness: Freshness {
            state: FreshnessState::Live,
            stale_since_ms: None,
        },
        payload: JsonVal(serde_json::json!({
            "agentCid": "agent_abc123",
            "devices": [],
            "totals": {
                "storageUsedBytes": 0,
                "storageTotalBytes": 0,
                "externalCommittedBytes": 0,
                "reciprocityNetBytes": 0
            },
            "freshness": {"state": "live"}
        })),
        signature:
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
                .into(),
    };

    let json = serde_json::to_value(&sample).unwrap();
    validate_against_schema("views/view-slice.schema.json", &json);
}

#[test]
fn view_slice_peer_topology_matches_schema() {
    use elohim_storage::views::{Freshness, FreshnessState, JsonVal, ViewKind, ViewSlice};

    let sample = ViewSlice {
        peer_id: "12D3KooWJessicaPhone".into(),
        view_kind: ViewKind::PeerTopology,
        freshness: Freshness {
            state: FreshnessState::Stale,
            stale_since_ms: Some(60_000),
        },
        payload: JsonVal(serde_json::json!({
            "agentCid": "agent_jess",
            "edges": [],
            "reciprocationCount": 0,
            "resilienceCliffs": [],
            "freshness": {"state": "stale", "staleSinceMs": 60000}
        })),
        signature:
            "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB"
                .into(),
    };

    let json = serde_json::to_value(&sample).unwrap();
    validate_against_schema("views/view-slice.schema.json", &json);
}

#[test]
fn doorway_dashboard_view_matches_schema() {
    use elohim_storage::views::{
        DashboardFederationPeer, DashboardSteward, DeviceArchetype, DoorwayDashboardView,
        FederationDirection, ProjectionCoverage, PublicSurfaceState,
    };

    let sample = DoorwayDashboardView {
        doorway_hostname: "matthew.elohim.host".into(),
        storage_stewards: vec![DashboardSteward {
            peer_id: "12D3KooWMatthew".into(),
            archetype: DeviceArchetype::Node,
            display_name: Some("matthew-blade-01".into()),
            online: true,
            hosting_count: 2_134,
            hop_hint: Some(1),
        }],
        federation_peers: vec![DashboardFederationPeer {
            doorway_hostname: "shem.elohim.host".into(),
            online: true,
            direction: FederationDirection::Bidirectional,
            shared_cid_count: 412,
        }],
        projection_coverage: ProjectionCoverage {
            projected_cid_count: 4_318,
            known_cid_count: 5_672,
            cache_hit_rate_24h: 0.87,
            projection_lag_ms_avg: 340,
        },
        public_surface: PublicSurfaceState {
            dns_resolves: true,
            dns_target: Some("203.0.113.42".into()),
            tls_valid: true,
            tls_expires_in_days: Some(64),
            public_reachable: true,
        },
    };

    let json = serde_json::to_value(&sample).unwrap();
    validate_against_schema("views/doorway-dashboard-view.schema.json", &json);
}

#[test]
fn reciprocity_row_matches_schema() {
    use elohim_storage::views::ReciprocityRow;

    let sample = ReciprocityRow {
        counterparty_household_id: "household-x".into(),
        display_name: None,
        committed_bytes: 0,
        delivered_bytes: 0,
        honored_percent: 0.0,
        online: None,
    };

    let json = serde_json::to_value(&sample).unwrap();
    validate_against_schema("views/reciprocity-row.schema.json", &json);
}

#[test]
fn peer_household_edge_minimal_matches_schema() {
    use elohim_storage::views::PeerHouseholdEdge;

    let sample = PeerHouseholdEdge {
        household_id: "household-x".into(),
        display_name: None,
        online: false,
        last_sync_sec: None,
        my_cids_hosted_by_them: 0,
        their_cids_hosted_by_me: 0,
        net_diff: None,
        is_critical_for_me: None,
        i_am_critical_for_them: None,
    };

    let json = serde_json::to_value(&sample).unwrap();
    validate_against_schema("views/peer-household-edge.schema.json", &json);
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
        DistributionSummary, DiversityHint, FetchSource, ProjectionTier, ReachClass, ReplicaHealth,
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
        projection_tier: ProjectionTier::Local, // T15: computed
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

#[test]
fn render_capability_profile_round_trips_against_schema() {
    use elohim_storage::{BundleEntry, RenderCapabilityProfile, RendererKind};
    let profile = RenderCapabilityProfile {
        bundles: vec![BundleEntry {
            name: "lamad-app".into(),
            version: "1.0.3".into(),
            renderer: RendererKind::AngularSsr,
            digest: Some("sha256:abc123".into()),
        }],
        renderers: vec![RendererKind::AngularSsr],
        auth_modes: vec!["anonymous".into(), "doorway-hosted".into()],
        max_concurrent_renders: 8,
        memory_budget_mib: Some(1024),
    };
    let json = serde_json::to_string(&profile).expect("serialize");
    let back: RenderCapabilityProfile = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.bundles[0].name, "lamad-app");
    assert_eq!(back.bundles[0].renderer, RendererKind::AngularSsr);
    assert!(back.auth_modes.contains(&"anonymous".to_string()));
}

#[test]
fn capability_extensions_round_trips() {
    use elohim_storage::CapabilityExtensions;
    use serde_json::json;
    let ext_json = json!({
        "transcode": {
            "schemaRef": "epr:schema:view:transcode-capability-profile",
            "profile": { "codecs": ["h264", "av1"] }
        }
    });
    let ext: CapabilityExtensions = serde_json::from_value(ext_json.clone()).expect("deserialize");
    let back = serde_json::to_value(&ext).expect("serialize");
    assert_eq!(back, ext_json);
}

#[test]
fn render_capability_serializes_with_camel_case() {
    use elohim_storage::{BundleEntry, RenderCapabilityProfile, RendererKind};
    let profile = RenderCapabilityProfile {
        bundles: vec![BundleEntry {
            name: "lamad-app".into(),
            version: "1.0.3".into(),
            renderer: RendererKind::AngularSsr,
            digest: None,
        }],
        renderers: vec![RendererKind::AngularSsr],
        auth_modes: vec!["anonymous".into()],
        max_concurrent_renders: 4,
        memory_budget_mib: None,
    };
    let json_value: serde_json::Value = serde_json::to_value(&profile).unwrap();
    // Camel case keys, kebab-case enum values
    assert!(json_value.get("authModes").is_some());
    assert!(json_value.get("maxConcurrentRenders").is_some());
    assert!(json_value["bundles"][0].get("renderer").unwrap() == "angular-ssr");
    // Optional fields with None should be skipped (per #[serde(skip_serializing_if = "Option::is_none")])
    assert!(json_value.get("memoryBudgetMib").is_none());
    assert!(json_value["bundles"][0].get("digest").is_none());
}

#[test]
fn peer_status_view_carries_render_capability_when_layered() {
    use elohim_storage::{
        build_peer_status_view, BundleEntry, RenderCapabilityProfile, RendererKind,
    };
    let row = elohim_storage::db::peer_statuses::PeerStatusRow {
        peer_id: "peer-x".into(),
        status: "online".into(),
        general_pool_member: 1,
        accepting_stewardship_reserves: 0,
        archetype_class: Some("home-nuc".into()),
        timestamp: 1_700_000_000_000_000,
        dht_anchor_hash: "anchor-1".into(),
        updated_at: 1_700_000_000_000_000,
    };
    let render_cap = RenderCapabilityProfile {
        bundles: vec![BundleEntry {
            name: "lamad-app".into(),
            version: "1.0.3".into(),
            renderer: RendererKind::AngularSsr,
            digest: None,
        }],
        renderers: vec![RendererKind::AngularSsr],
        auth_modes: vec!["anonymous".into()],
        max_concurrent_renders: 4,
        memory_budget_mib: None,
    };
    let view = build_peer_status_view(row, None, Some(&render_cap), None);
    assert!(view.render_capability.is_some());
    assert_eq!(view.render_capability.unwrap().bundles[0].name, "lamad-app");
    assert!(view.extensions.is_none());
}

#[test]
fn peer_status_view_renders_null_capability_when_unlayered() {
    use elohim_storage::build_peer_status_view;
    let row = elohim_storage::db::peer_statuses::PeerStatusRow {
        peer_id: "peer-y".into(),
        status: "online".into(),
        general_pool_member: 1,
        accepting_stewardship_reserves: 0,
        archetype_class: None,
        timestamp: 1_700_000_000_000_000,
        dht_anchor_hash: "anchor-2".into(),
        updated_at: 1_700_000_000_000_000,
    };
    let view = build_peer_status_view(row, None, None, None);
    assert!(view.render_capability.is_none());
    assert!(view.extensions.is_none());
}

// ---------------------------------------------------------------------------
// Task 7: load_render_capability_from_url loader tests
// ---------------------------------------------------------------------------
//
// These tests mutate `DOORWAY_CAPABILITY_URL`. cargo test runs tests in
// parallel, so an unguarded set_var in one test can leak into env::var reads
// in another (memory: feedback_env_var_test_flakiness). Serialize via a
// single static Mutex around the env-var section of each test.

static DOORWAY_CAP_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn load_render_capability_from_url_returns_none_when_unset() {
    let _g = DOORWAY_CAP_ENV_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    std::env::remove_var("DOORWAY_CAPABILITY_URL");
    let result = elohim_storage::load_render_capability_from_url_blocking();
    assert!(result.is_none());
}

#[tokio::test]
async fn load_render_capability_from_url_parses_valid_response() {
    use wiremock::{matchers, Mock, MockServer, ResponseTemplate};
    let server = MockServer::start().await;
    let body = serde_json::json!({
        "bundles": [{ "name": "lamad-app", "version": "1.0.3", "renderer": "angular-ssr" }],
        "renderers": ["angular-ssr"],
        "authModes": ["anonymous"],
        "maxConcurrentRenders": 4
    });
    Mock::given(matchers::method("GET"))
        .and(matchers::path("/admin/capability"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;
    let _g = DOORWAY_CAP_ENV_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    std::env::set_var(
        "DOORWAY_CAPABILITY_URL",
        format!("{}/admin/capability", server.uri()),
    );
    let result = elohim_storage::load_render_capability_from_url().await;
    std::env::remove_var("DOORWAY_CAPABILITY_URL");
    drop(_g);
    assert!(result.is_some());
    let profile = result.unwrap();
    assert_eq!(profile.bundles.len(), 1);
    assert_eq!(profile.bundles[0].name, "lamad-app");
    assert!(profile.auth_modes.contains(&"anonymous".to_string()));
}

#[tokio::test]
async fn load_render_capability_from_url_returns_none_on_5xx() {
    use wiremock::{matchers, Mock, MockServer, ResponseTemplate};
    let server = MockServer::start().await;
    Mock::given(matchers::method("GET"))
        .and(matchers::path("/admin/capability"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    let _g = DOORWAY_CAP_ENV_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    std::env::set_var(
        "DOORWAY_CAPABILITY_URL",
        format!("{}/admin/capability", server.uri()),
    );
    let result = elohim_storage::load_render_capability_from_url().await;
    std::env::remove_var("DOORWAY_CAPABILITY_URL");
    drop(_g);
    assert!(result.is_none());
}

#[tokio::test]
async fn load_render_capability_from_url_returns_none_on_unparseable_body() {
    use wiremock::{matchers, Mock, MockServer, ResponseTemplate};
    let server = MockServer::start().await;
    Mock::given(matchers::method("GET"))
        .and(matchers::path("/admin/capability"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json at all"))
        .mount(&server)
        .await;
    let _g = DOORWAY_CAP_ENV_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    std::env::set_var(
        "DOORWAY_CAPABILITY_URL",
        format!("{}/admin/capability", server.uri()),
    );
    let result = elohim_storage::load_render_capability_from_url().await;
    std::env::remove_var("DOORWAY_CAPABILITY_URL");
    drop(_g);
    assert!(result.is_none());
}

// ── Peer Transport Manifest (Phase 12) ──────────────────────────

#[test]
fn peer_transport_manifest_view_matches_schema() {
    use elohim_storage::{
        IrohTransportProfileView, Libp2pTransportProfileView, PeerTransportManifestView,
    };

    let manifest = PeerTransportManifestView {
        agent_cid: "bafyrei...agent-1".to_string(),
        libp2p: Some(Libp2pTransportProfileView {
            peer_id: "12D3KooWPeer1".to_string(),
            addrs: vec!["/ip4/127.0.0.1/tcp/4001".to_string()],
            supports: vec!["blob".to_string(), "gossip".to_string()],
        }),
        iroh: Some(IrohTransportProfileView {
            node_id: "abcd...node-1".to_string(),
            relays: vec!["https://relay.iroh.network".to_string()],
            supports: vec!["blob".to_string(), "epr".to_string()],
        }),
        discovery: vec!["pkarr".to_string(), "kademlia".to_string()],
        capability_level: 5,
        last_observed: 1746878400,
    };

    let json = serde_json::to_value(&manifest).unwrap();
    validate_against_schema("views/peer-transport-manifest.schema.json", &json);
}

#[test]
fn peer_transport_manifest_view_libp2p_only() {
    use elohim_storage::{Libp2pTransportProfileView, PeerTransportManifestView};

    let manifest = PeerTransportManifestView {
        agent_cid: "bafyrei...agent-2".to_string(),
        libp2p: Some(Libp2pTransportProfileView {
            peer_id: "12D3KooWPeer2".to_string(),
            addrs: vec![],
            supports: vec!["blob".to_string()],
        }),
        iroh: None,
        discovery: vec!["kademlia".to_string()],
        capability_level: 3,
        last_observed: 1746878500,
    };

    let json = serde_json::to_value(&manifest).unwrap();
    validate_against_schema("views/peer-transport-manifest.schema.json", &json);
}

#[test]
fn peer_transport_manifest_view_iroh_only() {
    use elohim_storage::{IrohTransportProfileView, PeerTransportManifestView};

    let manifest = PeerTransportManifestView {
        agent_cid: "bafyrei...agent-3".to_string(),
        libp2p: None,
        iroh: Some(IrohTransportProfileView {
            node_id: "wxyz...node-3".to_string(),
            relays: vec![],
            supports: vec!["sync".to_string(), "view-fed".to_string()],
        }),
        discovery: vec!["pkarr".to_string()],
        capability_level: 5,
        last_observed: 1746878600,
    };

    let json = serde_json::to_value(&manifest).unwrap();
    validate_against_schema("views/peer-transport-manifest.schema.json", &json);
}

// ── PUT /blob/{hash} response — Cutover gate #3 ─────────────────────────────

#[test]
fn put_blob_response_view_matches_schema_with_blake3() {
    use elohim_storage::PutBlobResponseView;

    let sample = PutBlobResponseView {
        blob_hash: format!("sha256-{}", "aa".repeat(32)),
        total_size: 42,
        mime_type: "application/octet-stream".to_string(),
        encoding: "none".to_string(),
        data_shards: 1,
        total_shards: 1,
        shard_size: 42,
        shard_hashes: vec![format!("sha256-{}", "bb".repeat(32))],
        reach: "commons".to_string(),
        author_id: None,
        created_at: "2026-05-10T00:00:00Z".to_string(),
        verified_at: None,
        blake3_hash: Some("a".repeat(64)),
    };
    let json = serde_json::to_value(&sample).unwrap();
    validate_against_schema("views/put-blob-response.schema.json", &json);
}

#[test]
fn put_blob_response_view_matches_schema_libp2p_only() {
    use elohim_storage::PutBlobResponseView;

    // libp2p-only deployment: blake3_hash is None → serialises as null
    let sample = PutBlobResponseView {
        blob_hash: format!("sha256-{}", "cc".repeat(32)),
        total_size: 1024,
        mime_type: "application/zip".to_string(),
        encoding: "none".to_string(),
        data_shards: 1,
        total_shards: 1,
        shard_size: 1024,
        shard_hashes: vec![format!("sha256-{}", "dd".repeat(32))],
        reach: "commons".to_string(),
        author_id: Some("did:elohim:test".to_string()),
        created_at: "2026-05-10T01:00:00Z".to_string(),
        verified_at: Some("2026-05-10T01:00:01Z".to_string()),
        blake3_hash: None,
    };
    let json = serde_json::to_value(&sample).unwrap();
    validate_against_schema("views/put-blob-response.schema.json", &json);
}

// ── Unified AttestationView (Category A — DHT source of truth) ──────────────

#[test]
fn attestation_view_matches_schema_full() {
    use elohim_storage::AttestationView;

    let view = AttestationView {
        id: "bafyreiattestation001234567890123456789012345678901234567890123".to_string(),
        dht_anchor_hash: "aabbccdd0011223344556677889900aabbccdd001122334455667788990011"
            .to_string(),
        attestation_kind: "attestation:peer-endorsement".to_string(),
        subject_cid: "bafyreisubject001234567890123456789012345678901234567890123456".to_string(),
        subject_kind: "agent".to_string(),
        issuer_cid: "bafyreiissuer0012345678901234567890123456789012345678901234567".to_string(),
        parent_governance_action_cid: None,
        vote_value: None,
        vote_weight: None,
        proof_class: "witness".to_string(),
        proof_evidence_json: r#"{"witnessCount":3}"#.to_string(),
        evidence_json: r#"{"observationPeriodStart":"2026-01-01T00:00:00Z"}"#.to_string(),
        expires_at: None,
        supersedes_cid: None,
        revocation_reason: None,
        revoked_at: None,
        created_at: "2026-05-12T10:00:00Z".to_string(),
        manifest_ref: "lamad".to_string(),
        title: "Peer endorsement for agent-001".to_string(),
        description: None,
    };

    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/attestation-view.schema.json", &json);
}

#[test]
fn attestation_view_matches_schema_vote() {
    use elohim_storage::AttestationView;

    // Vote attestation — child of a governance-action.
    let view = AttestationView {
        id: "bafyreivote00001234567890123456789012345678901234567890123456789".to_string(),
        dht_anchor_hash: "11223344556677889900aabbccdd00112233445566778899001122334455".to_string(),
        attestation_kind: "attestation:governance-vote".to_string(),
        subject_cid: "bafyreisubject001234567890123456789012345678901234567890123456".to_string(),
        subject_kind: "governance-action".to_string(),
        issuer_cid: "bafyreiissuer0012345678901234567890123456789012345678901234567".to_string(),
        parent_governance_action_cid: Some(
            "bafyreigovaction01234567890123456789012345678901234567890123".to_string(),
        ),
        vote_value: Some("approve".to_string()),
        vote_weight: Some("1.0".to_string()),
        proof_class: "self-attest".to_string(),
        proof_evidence_json: r#"{}"#.to_string(),
        evidence_json: r#"{}"#.to_string(),
        expires_at: Some("2099-12-31T23:59:59Z".to_string()),
        supersedes_cid: None,
        revocation_reason: None,
        revoked_at: None,
        created_at: "2026-05-12T10:05:00Z".to_string(),
        manifest_ref: "mishpat".to_string(),
        title: "Vote: approve recovery-001".to_string(),
        description: Some("Approving this recovery request".to_string()),
    };

    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/attestation-view.schema.json", &json);
}

#[test]
fn attestation_view_matches_schema_revoked() {
    use elohim_storage::AttestationView;

    // Revoked attestation — revocation_reason and revoked_at present.
    let view = AttestationView {
        id: "bafyreirevoked001234567890123456789012345678901234567890123456".to_string(),
        dht_anchor_hash: "99887766554433221100ffeeddccbbaa99887766554433221100ffeeddccbb"
            .to_string(),
        attestation_kind: "attestation:mastery-endorsement".to_string(),
        subject_cid: "bafyreisubject001234567890123456789012345678901234567890123456".to_string(),
        subject_kind: "content".to_string(),
        issuer_cid: "bafyreiissuer0012345678901234567890123456789012345678901234567".to_string(),
        parent_governance_action_cid: None,
        vote_value: None,
        vote_weight: None,
        proof_class: "audit-signature".to_string(),
        proof_evidence_json: r#"{"auditRef":"audit-2026-001"}"#.to_string(),
        evidence_json: r#"{"summary":"mastery demonstrated"}"#.to_string(),
        expires_at: None,
        supersedes_cid: Some(
            "bafyreioldattestation01234567890123456789012345678901234567890".to_string(),
        ),
        revocation_reason: Some("superseded by updated assessment".to_string()),
        revoked_at: Some("2026-05-13T08:00:00Z".to_string()),
        created_at: "2026-05-12T09:00:00Z".to_string(),
        manifest_ref: "lamad".to_string(),
        title: "Mastery endorsement — deprecated".to_string(),
        description: None,
    };

    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/attestation-view.schema.json", &json);
}

// ── GovernanceActionView (Category A — DHT source of truth) ─────────────────

#[test]
fn governance_action_view_matches_schema_full() {
    use elohim_storage::GovernanceActionView;

    let view = GovernanceActionView {
        id: "bafyreigovaction01234567890123456789012345678901234567890123456".to_string(),
        dht_anchor_hash: "aabbccdd0011223344556677889900aabbccdd001122334455667788990011"
            .to_string(),
        governance_kind: "governance-action:recovery".to_string(),
        subject_cid: "bafyreiagent001234567890123456789012345678901234567890123456789".to_string(),
        proposer_cid: "bafyreiproposer01234567890123456789012345678901234567890123456".to_string(),
        threshold_json: r#"{"m":3}"#.to_string(),
        eligibility_predicate_json: None,
        ballot_format: "approve-reject".to_string(),
        closes_at: "2099-01-01T00:00:00Z".to_string(),
        parameters_json: None,
        title: "Recovery request for agent-001".to_string(),
        description: Some("Agent has lost access to their device".to_string()),
        created_at: "2026-05-12T10:00:00Z".to_string(),
    };

    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/governance-action-view.schema.json", &json);
}

#[test]
fn governance_action_view_matches_schema_with_eligibility() {
    use elohim_storage::GovernanceActionView;

    let view = GovernanceActionView {
        id: "bafyreigovaction-eligibility-01234567890123456789012345678901".to_string(),
        dht_anchor_hash: "11223344556677889900aabbccdd0011223344556677889900aabbccdd0011"
            .to_string(),
        governance_kind: "governance-action:content-moderation".to_string(),
        subject_cid: "bafyreicontent01234567890123456789012345678901234567890123456".to_string(),
        proposer_cid: "bafyreiproposer01234567890123456789012345678901234567890123456".to_string(),
        threshold_json: r#"{"m":5,"n":7}"#.to_string(),
        eligibility_predicate_json: Some(r#"{"role":"moderator"}"#.to_string()),
        ballot_format: "approve-reject".to_string(),
        closes_at: "2026-12-31T23:59:59Z".to_string(),
        parameters_json: Some(r#"{"appealWindow":"7d"}"#.to_string()),
        title: "Content moderation vote".to_string(),
        description: None,
        created_at: "2026-05-12T10:30:00Z".to_string(),
    };

    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/governance-action-view.schema.json", &json);
}

// ── GovernanceActionTallyView (Category C — local operational) ───────────────

#[test]
fn governance_action_tally_view_matches_schema_pending() {
    use elohim_storage::GovernanceActionTallyView;

    let view = GovernanceActionTallyView {
        parent_cid: "bafyreigovaction01234567890123456789012345678901234567890123456".to_string(),
        governance_kind: "governance-action:recovery".to_string(),
        subject_cid: "bafyreiagent001234567890123456789012345678901234567890123456789".to_string(),
        threshold_m: 3,
        threshold_n: None,
        threshold_percentage: None,
        closes_at: "2099-01-01T00:00:00Z".to_string(),
        current_approve_count: 1,
        current_reject_count: 0,
        current_abstain_count: 0,
        computed_status: "pending".to_string(),
        last_child_at: Some("2026-05-12T10:05:00Z".to_string()),
        rebuilt_at: "2026-05-12T10:05:01Z".to_string(),
    };

    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/governance-action-tally-view.schema.json", &json);
}

#[test]
fn governance_action_tally_view_matches_schema_quorum_reached() {
    use elohim_storage::GovernanceActionTallyView;

    let view = GovernanceActionTallyView {
        parent_cid: "bafyreigovaction01234567890123456789012345678901234567890123456".to_string(),
        governance_kind: "governance-action:recovery".to_string(),
        subject_cid: "bafyreiagent001234567890123456789012345678901234567890123456789".to_string(),
        threshold_m: 3,
        threshold_n: Some(5),
        threshold_percentage: None,
        closes_at: "2099-01-01T00:00:00Z".to_string(),
        current_approve_count: 3,
        current_reject_count: 0,
        current_abstain_count: 1,
        computed_status: "reached-quorum".to_string(),
        last_child_at: Some("2026-05-12T11:00:00Z".to_string()),
        rebuilt_at: "2026-05-12T11:00:01Z".to_string(),
    };

    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/governance-action-tally-view.schema.json", &json);
}

#[test]
fn governance_action_tally_view_matches_schema_no_votes_yet() {
    use elohim_storage::GovernanceActionTallyView;

    // Edge case: tally row exists but no votes cast yet.
    let view = GovernanceActionTallyView {
        parent_cid: "bafyreigovaction-empty-01234567890123456789012345678901234567".to_string(),
        governance_kind: "governance-action:recovery".to_string(),
        subject_cid: "bafyreiagent001234567890123456789012345678901234567890123456789".to_string(),
        threshold_m: 2,
        threshold_n: None,
        threshold_percentage: None,
        closes_at: "2099-06-01T00:00:00Z".to_string(),
        current_approve_count: 0,
        current_reject_count: 0,
        current_abstain_count: 0,
        computed_status: "pending".to_string(),
        last_child_at: None,
        rebuilt_at: "2026-05-12T09:00:00Z".to_string(),
    };

    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/governance-action-tally-view.schema.json", &json);
}

// ── ResilienceHubView + HubSummary (C2) ───────────────────────

#[test]
fn resilience_hub_view_empty_hubs_matches_schema() {
    use elohim_storage::views::{HubKind, HubSummary, ResilienceHubView};
    let _ = (HubKind::Computed, HubSummary::default()); // ensure types are in scope

    let view = ResilienceHubView {
        content_id: "sha256-empty-test".to_string(),
        hubs: vec![],
    };
    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/resilience-hub-view.schema.json", &json);
}

#[test]
fn resilience_hub_view_computed_hub_matches_schema() {
    use elohim_storage::views::{HubKind, HubSummary, ResilienceHubView};

    let view = ResilienceHubView {
        content_id: "sha256-computed-test".to_string(),
        hubs: vec![HubSummary {
            hub_id: "12D3KooWSinglePeer".to_string(),
            kind: HubKind::Computed,
            replica_count: 1,
            last_verified_seconds: Some(120),
            display_label: None,
        }],
    };
    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/resilience-hub-view.schema.json", &json);
    // Verify hub entry validates individually against hub-summary schema
    validate_against_schema("views/hub-summary.schema.json", &json["hubs"][0]);
}

#[test]
fn resilience_hub_view_all_kinds_validate() {
    use elohim_storage::views::{HubKind, HubSummary, ResilienceHubView};

    for (kind, hub_id) in [
        (HubKind::Dwelling, "household-alpha"),
        (HubKind::Collective, "collective-dawn-runners"),
        (HubKind::Computed, "12D3KooWPeerSingle"),
    ] {
        let view = ResilienceHubView {
            content_id: "sha256-all-kinds-test".to_string(),
            hubs: vec![HubSummary {
                hub_id: hub_id.to_string(),
                kind,
                replica_count: 2,
                last_verified_seconds: None,
                display_label: Some(format!("Test {hub_id}")),
            }],
        };
        let json = serde_json::to_value(&view).unwrap();
        validate_against_schema("views/resilience-hub-view.schema.json", &json);
        validate_against_schema("views/hub-summary.schema.json", &json["hubs"][0]);
    }
}

#[test]
fn hub_summary_no_optional_fields_matches_schema() {
    use elohim_storage::views::{HubKind, HubSummary};

    // Minimal HubSummary — only required fields; optional fields omitted.
    let summary = HubSummary {
        hub_id: "household-minimal".to_string(),
        kind: HubKind::Dwelling,
        replica_count: 3,
        last_verified_seconds: None,
        display_label: None,
    };
    let json = serde_json::to_value(&summary).unwrap();
    validate_against_schema("views/hub-summary.schema.json", &json);
    // Ensure optional fields are absent in wire format
    assert!(
        json.get("lastVerifiedSeconds").is_none(),
        "lastVerifiedSeconds must be absent when None"
    );
    assert!(
        json.get("displayLabel").is_none(),
        "displayLabel must be absent when None"
    );
}

// ── Multi-collective collab views (Task 9) ───────────────────────────────────

#[test]
fn collective_view_round_trips_through_schema() {
    let v = CollabCollectiveView {
        cid: "collective:abc123".into(),
        founder_agent_cid: "agent:xyz".into(),
        charter: "Test charter".into(),
        display_name: "Test Collective".into(),
        created_at_block_height: 12345,
        anchor_agreement_cid: None,
        elohim_tier: ElohimTier::T0,
    };
    let json = serde_json::to_value(&v).unwrap();
    validate_against_schema("views/collective-view.schema.json", &json);
    let round: CollabCollectiveView = serde_json::from_value(json).unwrap();
    assert_eq!(v, round);
}

#[test]
fn collab_qahal_view_round_trips() {
    let v = CollabQahalView {
        cid: "collective:qahal-1".into(),
        anchor_agreement_cid: "agreement:abc".into(),
        display_name: "Test Collab".into(),
        created_at_block_height: 22222,
        elohim_tier: ElohimTier::T0,
        member_collectives: vec![],
        member_persons: vec!["agent:p1".into()],
        commons_pool_balance: None,
    };
    let json = serde_json::to_value(&v).unwrap();
    validate_against_schema("views/collab-qahal-view.schema.json", &json);
    let round: CollabQahalView = serde_json::from_value(json).unwrap();
    assert_eq!(v, round);
}

#[test]
fn membership_view_round_trips() {
    let v = CollabMembershipView {
        cid: "membership:m1".into(),
        member_cid: "collective:a".into(),
        member_kind: MemberKind::Collective,
        collective_cid: "collective:qahal-1".into(),
        role: CollabMembershipRole::Steward,
        sponsor_cid: Some("agreement:abc".into()),
        joined_at_block_height: 33333,
        withdrawn_at_block_height: None,
    };
    let json = serde_json::to_value(&v).unwrap();
    validate_against_schema("views/membership-view.schema.json", &json);
    let round: CollabMembershipView = serde_json::from_value(json).unwrap();
    assert_eq!(v, round);
}

#[test]
fn collab_agreement_view_round_trips() {
    let v = CollabAgreementView {
        cid: "agreement:abc".into(),
        authored_by_agent_cid: "agent:author".into(),
        participants: vec!["collective:a".into(), "collective:b".into()],
        scope: "test scope".into(),
        share_allocation: ShareAllocation {
            form: ShareAllocationForm::Declared,
            shares: Some(vec![
                DeclaredShare {
                    collective_cid: "collective:a".into(),
                    share: 0.5,
                },
                DeclaredShare {
                    collective_cid: "collective:b".into(),
                    share: 0.45,
                },
            ]),
            affinity_window_blocks: None,
            rebalance_cadence_blocks: None,
            commons_pool_tribute: 0.05,
        },
        commons_pool_tribute: 0.05,
        governance_terms: Some(GovernanceTerms {
            exit_terms: "clean".into(),
        }),
        initial_tier: ElohimTier::T0,
        created_at_block_height: 44444,
        status: CollabAgreementStatus::PendingAttestations,
        attested_by: vec![],
        collab_qahal_cid: None,
    };
    let json = serde_json::to_value(&v).unwrap();
    validate_against_schema("views/collab-agreement-view.schema.json", &json);
    let round: CollabAgreementView = serde_json::from_value(json).unwrap();
    assert_eq!(v, round);
}

#[test]
#[should_panic(expected = "Schema validation failed")]
fn share_allocation_refuses_zero_tribute_via_schema() {
    // JSON Schema must refuse zero tribute via exclusiveMinimum on commonsPoolTribute.
    // validate_against_schema panics on validation failure — that panic is the assertion.
    let bad = serde_json::json!({
        "form": "declared",
        "shares": [
            {"collectiveCid": "collective:a", "share": 0.5},
            {"collectiveCid": "collective:b", "share": 0.5}
        ],
        "commonsPoolTribute": 0.0
    });
    validate_against_schema("objects/share-allocation.schema.json", &bad);
}

// ── ElementRegistryView (Task A9 — pillar EPR decomposition) ─────────────────
//
// Source of truth: content table with contentFormat='element-registry-manifest'
// (Category C, operational projection — no new DHT entry type).
// Schema: elohim/sdk/schemas/v1/views/element-registry-view.schema.json
// Rust struct: elohim-views/src/element_registry.rs

#[test]
fn element_registry_view_matches_schema() {
    use elohim_views::element_registry::{ElementEntry, ElementRegistryView};

    // Exercises:
    //  - minimal valid registry (one element, no view_deps)
    //  - tagName satisfies ^[a-z][a-z0-9]*-[a-z0-9-]+$ pattern
    //  - cid satisfies ^sha256- pattern
    //  - version satisfies ^\d+\.\d+\.\d+ pattern
    let view = ElementRegistryView {
        epr_id: "elohim-core-elements".into(),
        pillar: "elohim-core".into(),
        elements: vec![ElementEntry {
            tag_name: "elohim-button".into(),
            cid: "sha256-0000000000000000000000000000000000000000000000000000000000000000".into(),
            version: "1.0.0".into(),
            view_deps: vec![],
        }],
    };

    let json = serde_json::to_value(&view).unwrap();

    // Confirm camelCase wire shape before handing to schema validator.
    assert_eq!(json["eprId"], serde_json::json!("elohim-core-elements"));
    assert_eq!(
        json["elements"][0]["tagName"],
        serde_json::json!("elohim-button")
    );

    validate_against_schema("views/element-registry-view.schema.json", &json);
}

#[test]
fn element_registry_view_with_view_deps_matches_schema() {
    use elohim_views::element_registry::{ElementEntry, ElementRegistryView};

    // Exercises:
    //  - multiple elements, one with view_deps populated
    //  - pillar = "shefa"
    let view = ElementRegistryView {
        epr_id: "shefa-elements".into(),
        pillar: "shefa".into(),
        elements: vec![
            ElementEntry {
                tag_name: "shefa-balance-card".into(),
                cid: "sha256-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .into(),
                version: "2.1.3".into(),
                view_deps: vec!["ShefaBalanceView".into(), "AccountView".into()],
            },
            ElementEntry {
                tag_name: "shefa-commitment-chip".into(),
                cid: "sha256-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    .into(),
                version: "1.0.0".into(),
                view_deps: vec![],
            },
        ],
    };

    let json = serde_json::to_value(&view).unwrap();

    // Confirm nested camelCase (viewDeps).
    assert_eq!(
        json["elements"][0]["viewDeps"],
        serde_json::json!(["ShefaBalanceView", "AccountView"])
    );

    validate_against_schema("views/element-registry-view.schema.json", &json);
}

// ── EprProjectionView (Task A5 — pillar EPR decomposition) ──────────────────
//
// Source of truth: rea_commitments where action='project-epr' (Notarized,
// Category A — backed by REA Commitment entry type on the DHT).
// Schema: elohim/sdk/schemas/v1/views/epr-projection-view.schema.json
// Rust struct: elohim-views/src/projection.rs

#[test]
fn epr_projection_view_cached_mode_matches_schema() {
    use elohim_views::projection::{
        EprProjectionView, GateHintRef, GateHintRelation, ProjectionMode,
    };

    // Exercises:
    //  - mode = "cached" (stewardDirectEndpoint must be null / absent)
    //  - previewEprRef = null (nullable optional)
    //  - gateHints with one entry (including label = null)
    //  - redirectsFrom non-empty
    let view = EprProjectionView {
        commitment_id: "sha256-abc01234def56789abc01234def56789abc01234def56789abc01234def56789"
            .into(),
        epr_id: "bafyreib2vq7lamadEPRcid0123456789012345678901234567890123456".into(),
        doorway_id: "doorway:alpha-elohim-host".into(),
        url_path: "/lamad".into(),
        mode: ProjectionMode::Cached,
        reach: "commons".into(),
        base_href: "/lamad/".into(),
        entry_file: "index.html".into(),
        spa_fallback: true,
        redirects_from: vec!["/learn".into(), "/study".into()],
        redirect_templates: vec![],
        route_claims: None,
        preview_epr_ref: None,
        gate_hints: vec![GateHintRef {
            epr_ref: "bafyreib2vq7gateEPRcid0123456789012345678901234567890123456".into(),
            label: None,
            relation: GateHintRelation::MembershipPrerequisite,
        }],
        dead_end: false,
        steward_direct_endpoint: None,
        seeded_at: "2026-05-25T00:00:00Z".into(),
        seeded_by: "12D3KooWMatthewBlade01AlphaNode".into(),
    };

    let json = serde_json::to_value(&view).unwrap();

    // stewardDirectEndpoint must be null on the wire (not absent) because the
    // schema lists it in `required` indirectly via oneOf[null | $ref].
    // Confirm mode serializes correctly.
    assert_eq!(json["mode"], serde_json::json!("cached"));
    // spaFallback rides the wire as a required boolean (§12.2 SPA deep-link
    // fallback); confirm it serializes for the default-true bundle case.
    assert_eq!(json["spaFallback"], serde_json::json!(true));
    // Confirm doorwayId satisfies the "^doorway:" pattern constraint.
    assert!(
        json["doorwayId"].as_str().unwrap().starts_with("doorway:"),
        "doorwayId must begin with 'doorway:'"
    );

    validate_against_schema("views/epr-projection-view.schema.json", &json);
}

#[test]
fn epr_projection_view_with_route_claims_matches_schema() {
    use elohim_views::projection::{
        EprProjectionView, ProjectionMode, RedirectTemplate, RouteClaimGrant, RouteClaimTemplate,
    };

    let view = EprProjectionView {
        commitment_id: "sha256-claims0123".into(),
        epr_id: "lamad-spa".into(),
        doorway_id: "doorway:alpha-elohim-host".into(),
        url_path: "/lamad".into(),
        mode: ProjectionMode::Cached,
        reach: "commons".into(),
        base_href: "/lamad/".into(),
        entry_file: "index.html".into(),
        spa_fallback: true,
        redirects_from: vec![],
        redirect_templates: vec![RedirectTemplate {
            from: "/lamad/resource/{id}".into(),
            to: "/epr/{id}".into(),
        }],
        route_claims: Some(RouteClaimGrant {
            schema_version: 1,
            claims_manifest_cid: None,
            claims: vec![RouteClaimTemplate {
                content_type: "path".into(),
                template: "path/{id}".into(),
                fragments: std::collections::BTreeMap::from([(
                    "step".to_string(),
                    "path/{id}/step/{n}".to_string(),
                )]),
            }],
        }),
        preview_epr_ref: None,
        gate_hints: vec![],
        dead_end: false,
        steward_direct_endpoint: None,
        seeded_at: "2026-06-06T00:00:00Z".into(),
        seeded_by: "12D3KooWTest".into(),
    };

    let json = serde_json::to_value(&view).unwrap();
    assert_eq!(
        json["routeClaims"]["claims"][0]["contentType"],
        serde_json::json!("path")
    );
    assert_eq!(
        json["redirectTemplates"][0]["from"],
        serde_json::json!("/lamad/resource/{id}")
    );
    validate_against_schema("views/epr-projection-view.schema.json", &json);
}

#[test]
fn epr_projection_view_steward_direct_populated_matches_schema() {
    use elohim_views::projection::{
        EprProjectionView, GateHintRef, GateHintRelation, ProjectionMode, StewardDirectEndpoint,
    };

    // Exercises:
    //  - mode = "stewardDirect" — hits the $ref branch of the oneOf
    //  - stewardDirectEndpoint fully populated with alt_host = Some(...)
    //  - previewEprRef = Some("bafyrei...")
    //  - gate_hints with multiple relations
    //  - dead_end = true
    let view = EprProjectionView {
        commitment_id: "sha256-feed0123feed0123feed0123feed0123feed0123feed0123feed0123feed0123"
            .into(),
        epr_id: "bafyreib2vq7elohimAppEPR012345678901234567890123456789012345".into(),
        doorway_id: "doorway:shem-elohim-host".into(),
        url_path: "/elohim-app".into(),
        mode: ProjectionMode::StewardDirect,
        reach: "qahal:dawn-runners".into(),
        base_href: "/elohim-app/".into(),
        entry_file: "index.html".into(),
        // Non-default false — the explicit opt-out of SPA fallback (§12.2). This
        // is wired end-to-end: doorway is contract-authoritative and propagates
        // the opt-out to storage's convention-level safety net via
        // `?spaFallback=0` (see dispatch_to_projected_epr + handle_app_request's
        // app_request_route_miss_respects_spa_fallback_opt_out test).
        spa_fallback: false,
        redirects_from: vec![],
        redirect_templates: vec![],
        route_claims: None,
        preview_epr_ref: Some(
            "bafyreib2vq7previewEPR01234567890123456789012345678901234567".into(),
        ),
        gate_hints: vec![
            GateHintRef {
                epr_ref: "bafyreib2vq7grantorEPR01234567890123456789012345678901234".into(),
                label: Some("Ask Matthew for access".into()),
                relation: GateHintRelation::PersonWhoCanGrant,
            },
            GateHintRef {
                epr_ref: "bafyreib2vq7capabilityEPR0123456789012345678901234567890".into(),
                label: None,
                relation: GateHintRelation::CapabilityToEarn,
            },
        ],
        dead_end: true,
        steward_direct_endpoint: Some(StewardDirectEndpoint {
            peer_id: "12D3KooWShemBlade02".into(),
            alt_host: Some("192.168.1.42".into()),
            tls_cert_san: "shem-blade-02.local".into(),
            accepts_projection_for: vec![
                "bafyreib2vq7elohimAppEPR012345678901234567890123456789012345".into(),
            ],
        }),
        seeded_at: "2026-05-25T10:30:00Z".into(),
        seeded_by: "12D3KooWShemBlade02".into(),
    };

    let json = serde_json::to_value(&view).unwrap();

    // Confirm mode serializes as "stewardDirect" (camelCase).
    assert_eq!(json["mode"], serde_json::json!("stewardDirect"));
    // Confirm stewardDirectEndpoint is present and not null.
    assert!(
        !json["stewardDirectEndpoint"].is_null(),
        "stewardDirectEndpoint must be an object when mode is stewardDirect"
    );
    // Confirm nested peerId is present.
    assert_eq!(
        json["stewardDirectEndpoint"]["peerId"],
        serde_json::json!("12D3KooWShemBlade02")
    );
    // Confirm the non-default spaFallback = false serializes on the wire.
    assert_eq!(json["spaFallback"], serde_json::json!(false));

    validate_against_schema("views/epr-projection-view.schema.json", &json);
}

// ── M-REA-1: LamadEventIntent intent schema contract ────────────────────────
//
// Validates that the LamadEventIntentView Rust struct serializes compatibly
// with the `intents/lamad-event-intent.schema.json` contract.
// Source of truth: request body wire shape (not an entity — Category A existing
// EconomicEvent is the notarized source of truth on the DHT).

#[test]
fn lamad_event_intent_minimal_validates() {
    // Minimum required fields only: agentId + lamadEventType
    let minimal = serde_json::json!({
        "agentId": "agent-abc123",
        "lamadEventType": "content-view"
    });
    validate_against_schema("intents/lamad-event-intent.schema.json", &minimal);
}

#[test]
fn lamad_event_intent_full_validates() {
    // All optional fields populated
    let full = serde_json::json!({
        "agentId": "agent-abc123",
        "lamadEventType": "recognition-given",
        "contentId": "content-xyz",
        "pathId": "path-456",
        "contributorPresenceId": "presence-789",
        "resourceQuantityValue": 1.0,
        "resourceQuantityUnit": "recognition",
        "metadata": { "source": "lamad-harness" },
        "note": "Genuine recognition for quality contribution"
    });
    validate_against_schema("intents/lamad-event-intent.schema.json", &full);
}

#[test]
fn lamad_event_intent_all_enum_values_are_valid() {
    // Every lamadEventType value in the schema enum must validate.
    // This catches drift if a new value is added to the Rust match arm
    // without being added to the schema enum, or vice versa.
    let schema_event_types = [
        "content-view",
        "content-complete",
        "path-step-complete",
        "path-complete",
        "session-start",
        "session-end",
        "assessment-start",
        "assessment-complete",
        "practice-attempt",
        "quiz-submit",
        "affinity-mark",
        "endorsement",
        "citation",
        "recognition-given",
        "recognition-received",
        "attestation-grant",
        "capability-earn",
        "content-create",
        "path-create",
        "extension-create",
        "map-synthesis",
        "analysis-complete",
        "stewardship-begin",
        "invitation-send",
        "presence-claim",
        "recognition-transfer",
        "attestation-revoke",
        "content-flag",
        "governance-vote",
    ];
    for event_type in &schema_event_types {
        let instance = serde_json::json!({
            "agentId": "agent-test",
            "lamadEventType": event_type
        });
        validate_against_schema("intents/lamad-event-intent.schema.json", &instance);
    }
}

#[test]
fn lamad_event_intent_rejects_unknown_event_type() {
    let invalid = serde_json::json!({
        "agentId": "agent-abc123",
        "lamadEventType": "totally-made-up"
    });
    let schema = load_schema("intents/lamad-event-intent.schema.json");
    let validator =
        jsonschema::validator_for(&schema).expect("lamad-event-intent schema should compile");
    let errors: Vec<_> = validator.iter_errors(&invalid).collect();
    assert!(
        !errors.is_empty(),
        "Schema should reject unknown lamadEventType values"
    );
}

#[test]
fn lamad_event_intent_rejects_missing_required_fields() {
    let schema = load_schema("intents/lamad-event-intent.schema.json");
    let validator =
        jsonschema::validator_for(&schema).expect("lamad-event-intent schema should compile");

    // Missing agentId
    let no_agent = serde_json::json!({ "lamadEventType": "content-view" });
    assert!(
        validator.iter_errors(&no_agent).next().is_some(),
        "Schema should require agentId"
    );

    // Missing lamadEventType
    let no_type = serde_json::json!({ "agentId": "agent-abc123" });
    assert!(
        validator.iter_errors(&no_type).next().is_some(),
        "Schema should require lamadEventType"
    );
}

#[test]
fn lamad_event_intent_schema_declares_source_of_truth() {
    let path = schema_dir().join("intents/lamad-event-intent.schema.json");
    let content =
        fs::read_to_string(&path).expect("intents/lamad-event-intent.schema.json must exist");
    let schema_value: Value = serde_json::from_str(&content)
        .expect("intents/lamad-event-intent.schema.json must be valid JSON");
    assert_source_of_truth_declared(&schema_value, "intents/lamad-event-intent.schema.json");
}

// =============================================================================
// M-POLICY-1: AccumulationStatus + standing-policy payload schema contract tests
//
// Ticket: M-POLICY-1 (Server-side AccumulationStatus) — Phase E
// Source: genesis/docs/superpowers/plans/2026-05-28-thin-client-backend-migration.md §3
// =============================================================================

/// Full minimal AccumulationStatusView that satisfies all required fields.
fn minimal_accumulation_status() -> Value {
    serde_json::json!({
        "entityType": "content",
        "entityId": "concept-abc123",
        "totalSignals": 25,
        "uniqueParticipants": 18,
        "consensusStrength": 0.62,
        "status": "ready_for_sensemaking",
        "readyForSensemaking": true,
        "controversyDetected": false,
        "settled": false,
        "policyManifestCid": null,
        "computedAt": "2026-05-28T12:00:00Z"
    })
}

#[test]
fn accumulation_status_view_minimal_validates() {
    validate_against_schema(
        "views/accumulation-status.schema.json",
        &minimal_accumulation_status(),
    );
}

#[test]
fn accumulation_status_view_full_validates() {
    let full = serde_json::json!({
        "entityType": "path",
        "entityId": "path-resilience-1",
        "totalSignals": 35,
        "uniqueParticipants": 28,
        "consensusStrength": 0.88,
        "status": "settled",
        "readyForSensemaking": false,
        "controversyDetected": false,
        "settled": true,
        "policyManifestCid": "bafyreia1234567890abcdef",
        "computedAt": "2026-05-28T15:30:00Z"
    });
    validate_against_schema("views/accumulation-status.schema.json", &full);
}

#[test]
fn accumulation_status_view_controversy_detected_validates() {
    let controversy = serde_json::json!({
        "entityType": "content",
        "entityId": "lesson-xyz",
        "totalSignals": 14,
        "uniqueParticipants": 11,
        "consensusStrength": 0.22,
        "status": "controversy_detected",
        "readyForSensemaking": false,
        "controversyDetected": true,
        "settled": false,
        "policyManifestCid": null,
        "computedAt": "2026-05-28T09:00:00Z"
    });
    validate_against_schema("views/accumulation-status.schema.json", &controversy);
}

#[test]
fn accumulation_status_view_pending_validates() {
    let pending = serde_json::json!({
        "entityType": "content",
        "entityId": "new-content-1",
        "totalSignals": 3,
        "uniqueParticipants": 3,
        "consensusStrength": 1.0,
        "status": "pending",
        "readyForSensemaking": false,
        "controversyDetected": false,
        "settled": false,
        "policyManifestCid": null,
        "computedAt": "2026-05-28T08:00:00Z"
    });
    validate_against_schema("views/accumulation-status.schema.json", &pending);
}

#[test]
fn accumulation_status_view_rejects_missing_required_fields() {
    let schema = load_schema("views/accumulation-status.schema.json");
    let validator =
        jsonschema::validator_for(&schema).expect("accumulation-status schema should compile");

    // Missing status
    let no_status = {
        let mut v = minimal_accumulation_status();
        v.as_object_mut().unwrap().remove("status");
        v
    };
    assert!(
        validator.iter_errors(&no_status).next().is_some(),
        "Schema should require 'status'"
    );

    // Missing consensusStrength
    let no_consensus = {
        let mut v = minimal_accumulation_status();
        v.as_object_mut().unwrap().remove("consensusStrength");
        v
    };
    assert!(
        validator.iter_errors(&no_consensus).next().is_some(),
        "Schema should require 'consensusStrength'"
    );

    // Missing computedAt
    let no_computed = {
        let mut v = minimal_accumulation_status();
        v.as_object_mut().unwrap().remove("computedAt");
        v
    };
    assert!(
        validator.iter_errors(&no_computed).next().is_some(),
        "Schema should require 'computedAt'"
    );
}

#[test]
fn accumulation_status_view_rejects_unknown_status_value() {
    let schema = load_schema("views/accumulation-status.schema.json");
    let validator =
        jsonschema::validator_for(&schema).expect("accumulation-status schema should compile");

    let mut bad = minimal_accumulation_status();
    *bad.get_mut("status").unwrap() = serde_json::json!("stalemate");
    assert!(
        validator.iter_errors(&bad).next().is_some(),
        "Schema should reject unknown status values"
    );
}

#[test]
fn accumulation_status_schema_declares_source_of_truth() {
    let path = schema_dir().join("views/accumulation-status.schema.json");
    let content =
        fs::read_to_string(&path).expect("views/accumulation-status.schema.json must exist");
    let schema_value: Value = serde_json::from_str(&content)
        .expect("views/accumulation-status.schema.json must be valid JSON");
    assert_source_of_truth_declared(&schema_value, "views/accumulation-status.schema.json");
}

#[test]
fn standing_policy_payload_with_accumulation_thresholds_validates() {
    // A complete standing-policy payload with the accumulation_thresholds extension.
    // The floor object is required by the zome integrity validator.
    let payload = serde_json::json!({
        "floor": {
            "classes": [
                { "class": "local-relationship-reach",        "protection": "unconditional family/household propagation" },
                { "class": "cid-targeted-lookup",             "protection": "anyone with a CID can fetch" },
                { "class": "constitutional-floor-signatures", "protection": "mishpat decisions are always verified" },
                { "class": "new-voice-baseline",              "protection": "first-time author has nonzero floor" },
                { "class": "vulnerable-class-elevation",      "protection": "children and displaced persons get elevated floor" }
            ]
        },
        "accumulation_thresholds": {
            "readyForSensemaking": { "minTotalSignals": 20, "maxConsensusStrength": 0.7 },
            "controversyDetected": { "minTotalSignals": 10, "maxConsensusStrength": 0.3 },
            "settled":             { "minTotalSignals": 30, "maxConsensusStrength": 0.85 }
        }
    });
    validate_against_schema("manifest-payloads/standing-policy.schema.json", &payload);
}

#[test]
fn standing_policy_payload_without_thresholds_validates() {
    // accumulation_thresholds is optional — payload with only floor is valid.
    let payload = serde_json::json!({
        "floor": {
            "classes": [
                { "class": "local-relationship-reach",        "protection": "p1" },
                { "class": "cid-targeted-lookup",             "protection": "p2" },
                { "class": "constitutional-floor-signatures", "protection": "p3" },
                { "class": "new-voice-baseline",              "protection": "p4" },
                { "class": "vulnerable-class-elevation",      "protection": "p5" }
            ]
        }
    });
    validate_against_schema("manifest-payloads/standing-policy.schema.json", &payload);
}

#[test]
fn standing_policy_payload_rejects_missing_floor() {
    let schema = load_schema("manifest-payloads/standing-policy.schema.json");
    let validator =
        jsonschema::validator_for(&schema).expect("standing-policy payload schema should compile");

    let no_floor = serde_json::json!({
        "accumulation_thresholds": {
            "readyForSensemaking": { "minTotalSignals": 5, "maxConsensusStrength": 0.5 },
            "controversyDetected": { "minTotalSignals": 3, "maxConsensusStrength": 0.2 },
            "settled":             { "minTotalSignals": 10, "maxConsensusStrength": 0.9 }
        }
    });
    assert!(
        validator.iter_errors(&no_floor).next().is_some(),
        "standing-policy payload must require 'floor'"
    );
}

#[test]
fn standing_policy_payload_schema_declares_source_of_truth() {
    let path = schema_dir().join("manifest-payloads/standing-policy.schema.json");
    let content = fs::read_to_string(&path)
        .expect("manifest-payloads/standing-policy.schema.json must exist");
    let schema_value: Value = serde_json::from_str(&content)
        .expect("manifest-payloads/standing-policy.schema.json must be valid JSON");
    assert_source_of_truth_declared(
        &schema_value,
        "manifest-payloads/standing-policy.schema.json",
    );
}

// =============================================================================
// M-POLICY-2: MechanismSelection + pillar-projection payload schema contract tests
//
// Ticket: M-POLICY-2 (Server-side MechanismSelection) — Phase E
// Source: genesis/docs/superpowers/plans/2026-05-28-thin-client-backend-migration.md §3
// =============================================================================

/// Full minimal MechanismSelectionView that satisfies all required fields.
fn minimal_mechanism_selection() -> Value {
    serde_json::json!({
        "entityType": "content",
        "entityId": "concept-abc123",
        "level": 1,
        "mechanism": "reactions",
        "renderTarget": "angular",
        "contextMenuOnly": false,
        "allowReactions": true,
        "allowGraduatedFeedback": false,
        "activeProposalId": null,
        "activeProposalMechanism": null,
        "policyManifestCid": null,
        "computedAt": "2026-05-28T12:00:00Z"
    })
}

#[test]
fn mechanism_selection_view_minimal_validates() {
    validate_against_schema(
        "views/mechanism-selection.schema.json",
        &minimal_mechanism_selection(),
    );
}

#[test]
fn mechanism_selection_view_full_validates() {
    // Level 2: graduated feedback, policy manifest applied
    let full = serde_json::json!({
        "entityType": "content",
        "entityId": "discussion-xyz",
        "level": 2,
        "mechanism": "graduated-feedback",
        "renderTarget": "angular",
        "contextMenuOnly": false,
        "allowReactions": true,
        "allowGraduatedFeedback": true,
        "activeProposalId": null,
        "activeProposalMechanism": null,
        "policyManifestCid": "bafyreia1234567890abcdef",
        "computedAt": "2026-05-28T15:30:00Z"
    });
    validate_against_schema("views/mechanism-selection.schema.json", &full);
}

#[test]
fn mechanism_selection_view_psephos_level_validates() {
    // Level 4: active proposal — ranked-choice, psephos render target
    let psephos = serde_json::json!({
        "entityType": "content",
        "entityId": "proposal-resilience-1",
        "level": 4,
        "mechanism": "ranked-choice",
        "renderTarget": "psephos",
        "contextMenuOnly": false,
        "allowReactions": false,
        "allowGraduatedFeedback": false,
        "activeProposalId": "prop-abc-123",
        "activeProposalMechanism": "ranked-choice",
        "policyManifestCid": null,
        "computedAt": "2026-05-28T10:00:00Z"
    });
    validate_against_schema("views/mechanism-selection.schema.json", &psephos);
}

#[test]
fn mechanism_selection_view_level_zero_validates() {
    // Level 0: constitutional/settled state — context menu only
    let level_zero = serde_json::json!({
        "entityType": "path",
        "entityId": "path-constitutional",
        "level": 0,
        "mechanism": "none",
        "renderTarget": "angular",
        "contextMenuOnly": true,
        "allowReactions": false,
        "allowGraduatedFeedback": false,
        "activeProposalId": null,
        "activeProposalMechanism": null,
        "policyManifestCid": null,
        "computedAt": "2026-05-28T08:00:00Z"
    });
    validate_against_schema("views/mechanism-selection.schema.json", &level_zero);
}

#[test]
fn mechanism_selection_view_rejects_missing_required_fields() {
    let schema = load_schema("views/mechanism-selection.schema.json");
    let validator =
        jsonschema::validator_for(&schema).expect("mechanism-selection schema should compile");

    // Missing level
    let no_level = {
        let mut v = minimal_mechanism_selection();
        v.as_object_mut().unwrap().remove("level");
        v
    };
    assert!(
        validator.iter_errors(&no_level).next().is_some(),
        "Schema should require 'level'"
    );

    // Missing renderTarget
    let no_render_target = {
        let mut v = minimal_mechanism_selection();
        v.as_object_mut().unwrap().remove("renderTarget");
        v
    };
    assert!(
        validator.iter_errors(&no_render_target).next().is_some(),
        "Schema should require 'renderTarget'"
    );

    // Missing computedAt
    let no_computed = {
        let mut v = minimal_mechanism_selection();
        v.as_object_mut().unwrap().remove("computedAt");
        v
    };
    assert!(
        validator.iter_errors(&no_computed).next().is_some(),
        "Schema should require 'computedAt'"
    );
}

#[test]
fn mechanism_selection_view_rejects_invalid_render_target() {
    let schema = load_schema("views/mechanism-selection.schema.json");
    let validator =
        jsonschema::validator_for(&schema).expect("mechanism-selection schema should compile");

    let mut bad = minimal_mechanism_selection();
    *bad.get_mut("renderTarget").unwrap() = serde_json::json!("holochain");
    assert!(
        validator.iter_errors(&bad).next().is_some(),
        "Schema should reject renderTarget values other than 'angular' or 'psephos'"
    );
}

#[test]
fn mechanism_selection_view_schema_declares_source_of_truth() {
    let path = schema_dir().join("views/mechanism-selection.schema.json");
    let content =
        fs::read_to_string(&path).expect("views/mechanism-selection.schema.json must exist");
    let schema_value: Value = serde_json::from_str(&content)
        .expect("views/mechanism-selection.schema.json must be valid JSON");
    assert_source_of_truth_declared(&schema_value, "views/mechanism-selection.schema.json");
}

#[test]
fn pillar_projection_payload_with_mechanism_ladder_validates() {
    // A complete pillar-projection payload with full mechanism_ladder configuration.
    // No floor sub-object required (unlike standing-policy).
    let payload = serde_json::json!({
        "mechanism_ladder": {
            "settled_states": ["constitutional", "settled"],
            "mechanism_level_map": {
                "approval": 3,
                "dot-vote": 3,
                "ranked-choice": 4,
                "score-vote": 5,
                "conviction": 5,
                "consent": 6
            },
            "feedback_inviting_content_types": [
                "discussion",
                "proposal-draft",
                "request-for-comment",
                "reflection"
            ],
            "default_level_no_proposal": 1
        }
    });
    validate_against_schema("manifest-payloads/pillar-projection.schema.json", &payload);
}

#[test]
fn pillar_projection_payload_without_ladder_validates() {
    // mechanism_ladder is optional — an empty payload is valid (protocol defaults apply).
    let payload = serde_json::json!({});
    validate_against_schema("manifest-payloads/pillar-projection.schema.json", &payload);
}

#[test]
fn pillar_projection_payload_schema_declares_source_of_truth() {
    let path = schema_dir().join("manifest-payloads/pillar-projection.schema.json");
    let content = fs::read_to_string(&path)
        .expect("manifest-payloads/pillar-projection.schema.json must exist");
    let schema_value: Value = serde_json::from_str(&content)
        .expect("manifest-payloads/pillar-projection.schema.json must be valid JSON");
    assert_source_of_truth_declared(
        &schema_value,
        "manifest-payloads/pillar-projection.schema.json",
    );
}

// ── M-REA-2: tending-policy manifest-payload schema contract ────────────────
//
// Validates the tending-policy.schema.json payload shape and its alignment
// with what the Manifest integrity validator (content_store_integrity/src/manifest.rs)
// enforces at zome-create time. The floor sub-object must contain exactly
// the five TENDING_IMMUNE_CLASSES listed in the integrity zome.

fn tending_floor_json() -> Value {
    serde_json::json!({
        "floor": {
            "classes": [
                { "class": "accountability-information",        "protection": "corrections bypass all filters" },
                { "class": "community-facts",                   "protection": "civic emergencies never filterable" },
                { "class": "custodial-communications",          "protection": "ward-targeting messages bypass steward filter" },
                { "class": "constitutional-updates",            "protection": "mishpat rule changes always reach recipients" },
                { "class": "elohim-as-counsel-notifications",   "protection": "elohim may override filters for counsel" }
            ]
        }
    })
}

#[test]
fn tending_policy_payload_minimal_validates() {
    // floor is the only required field; all other fields are optional.
    validate_against_schema(
        "manifest-payloads/tending-policy.schema.json",
        &tending_floor_json(),
    );
}

#[test]
fn tending_policy_payload_with_dwell_qualification_validates() {
    // A complete tending-policy payload with all optional fields populated.
    let mut payload = tending_floor_json();
    payload.as_object_mut().unwrap().insert(
        "dwell_qualification_ms".to_string(),
        serde_json::json!(5000),
    );
    payload
        .as_object_mut()
        .unwrap()
        .insert("min_ttl_seconds".to_string(), serde_json::json!(7200));
    payload
        .as_object_mut()
        .unwrap()
        .insert("collective_k_threshold".to_string(), serde_json::json!(10));
    payload.as_object_mut().unwrap().insert(
        "allowed_classifications".to_string(),
        serde_json::json!(["values-forward", "fatigue"]),
    );
    validate_against_schema("manifest-payloads/tending-policy.schema.json", &payload);
}

#[test]
fn tending_policy_payload_rejects_missing_floor() {
    // floor sub-object is required — the integrity zome enforces this at create time.
    let no_floor = serde_json::json!({ "dwell_qualification_ms": 3000 });
    let schema = load_schema("manifest-payloads/tending-policy.schema.json");
    let validator =
        jsonschema::validator_for(&schema).expect("tending-policy schema should compile");
    assert!(
        validator.iter_errors(&no_floor).next().is_some(),
        "tending-policy payload must require the floor sub-object"
    );
}

#[test]
fn tending_policy_payload_schema_declares_source_of_truth() {
    let path = schema_dir().join("manifest-payloads/tending-policy.schema.json");
    let content =
        fs::read_to_string(&path).expect("manifest-payloads/tending-policy.schema.json must exist");
    let schema_value: Value = serde_json::from_str(&content)
        .expect("manifest-payloads/tending-policy.schema.json must be valid JSON");
    assert_source_of_truth_declared(
        &schema_value,
        "manifest-payloads/tending-policy.schema.json",
    );
}

// ── M-REA-2: attention-tending-intent schema contract ───────────────────────
//
// Validates that the intent wire shape for POST /api/v1/attention/tending
// aligns with CreateAttentionTendingInput in content_store/src/attention_tending.rs.
// Source of truth: request body wire shape (not an entity — Category B agent-scoped
// private source chain entry). The struct is canonical for an EXISTING substrate type.

#[test]
fn attention_tending_intent_minimal_validates() {
    // Minimum required fields: all five are required per the schema.
    let minimal = serde_json::json!({
        "filterSubjectJson": r#"{"contentId":"abc123"}"#,
        "classification": "values-forward",
        "ttlSeconds": 3600,
        "contextJson": r#"{"pillar":"lamad"}"#,
        "elapsedMs": 4500
    });
    validate_against_schema("intents/attention-tending-intent.schema.json", &minimal);
}

#[test]
fn attention_tending_intent_with_reason_validates() {
    // Optional reason field populated.
    let full = serde_json::json!({
        "filterSubjectJson": r#"{"contentId":"xyz789"}"#,
        "classification": "fatigue",
        "reason": "Saw this topic repeatedly this week",
        "ttlSeconds": 7200,
        "contextJson": r#"{"pillar":"lamad","surface":"content-viewer"}"#,
        "elapsedMs": 1200
    });
    validate_against_schema("intents/attention-tending-intent.schema.json", &full);
}

#[test]
fn attention_tending_intent_rejects_missing_required_fields() {
    let schema = load_schema("intents/attention-tending-intent.schema.json");
    let validator =
        jsonschema::validator_for(&schema).expect("attention-tending-intent schema should compile");

    // Missing filterSubjectJson
    let no_filter = serde_json::json!({
        "classification": "values-forward",
        "ttlSeconds": 3600,
        "contextJson": r#"{}"#,
        "elapsedMs": 4000
    });
    assert!(
        validator.iter_errors(&no_filter).next().is_some(),
        "Schema should require filterSubjectJson"
    );

    // Missing elapsedMs (new field replacing client-side DWELL_THRESHOLD_MS)
    let no_elapsed = serde_json::json!({
        "filterSubjectJson": r#"{"contentId":"abc"}"#,
        "classification": "values-forward",
        "ttlSeconds": 3600,
        "contextJson": r#"{}"#
    });
    assert!(
        validator.iter_errors(&no_elapsed).next().is_some(),
        "Schema should require elapsedMs"
    );

    // ttlSeconds below minimum (3600)
    let short_ttl = serde_json::json!({
        "filterSubjectJson": r#"{"contentId":"abc"}"#,
        "classification": "values-forward",
        "ttlSeconds": 3599,
        "contextJson": r#"{}"#,
        "elapsedMs": 4000
    });
    assert!(
        validator.iter_errors(&short_ttl).next().is_some(),
        "Schema should reject ttlSeconds below 3600"
    );
}

#[test]
fn attention_tending_intent_rejects_invalid_classification() {
    let schema = load_schema("intents/attention-tending-intent.schema.json");
    let validator =
        jsonschema::validator_for(&schema).expect("attention-tending-intent schema should compile");

    let bad_class = serde_json::json!({
        "filterSubjectJson": r#"{"contentId":"abc"}"#,
        "classification": "invalid-class",
        "ttlSeconds": 3600,
        "contextJson": r#"{}"#,
        "elapsedMs": 4000
    });
    assert!(
        validator.iter_errors(&bad_class).next().is_some(),
        "Schema should reject invalid classification values"
    );
}

#[test]
fn attention_tending_intent_schema_declares_source_of_truth() {
    let path = schema_dir().join("intents/attention-tending-intent.schema.json");
    let content =
        fs::read_to_string(&path).expect("intents/attention-tending-intent.schema.json must exist");
    let schema_value: Value = serde_json::from_str(&content)
        .expect("intents/attention-tending-intent.schema.json must be valid JSON");
    assert_source_of_truth_declared(
        &schema_value,
        "intents/attention-tending-intent.schema.json",
    );
}

// =============================================================================
// M-REA-3: RecognitionParticipationIntent schema contract tests
//
// Ticket: M-REA-3 (Substrate-side governance recognition) — Phase B
// Source: genesis/docs/superpowers/plans/2026-05-28-thin-client-backend-migration.md §3
//
// These tests validate the recognition-participation-intent.schema.json shape
// against realistic instances and confirm the source-of-truth declaration is
// present. The intent wire shape drives POST /api/v1/mishpat/recognition/participation;
// the mishpat coordinator composes the EconomicEvent using the standing-policy
// Manifest's recognition_weight_by_level. No new DHT entry type.
// =============================================================================

#[test]
fn recognition_participation_intent_minimal_validates() {
    // Minimal valid intent — all required fields, no optional ones.
    let minimal = serde_json::json!({
        "entityType": "content",
        "entityId": "sha256-abc123",
        "humanId": "uhCAkSomeAgentPubKey",
        "mechanismLevel": 1,
        "participationType": "reaction"
    });
    validate_against_schema(
        "intents/recognition-participation-intent.schema.json",
        &minimal,
    );
}

#[test]
fn recognition_participation_intent_level_7_deliberation_validates() {
    // Level 7 (deliberation — highest weight) with block participation type.
    let high_friction = serde_json::json!({
        "entityType": "proposal",
        "entityId": "proposal-xyz-456",
        "humanId": "uhCAkAnotherAgent",
        "mechanismLevel": 7,
        "participationType": "graduated-feedback"
    });
    validate_against_schema(
        "intents/recognition-participation-intent.schema.json",
        &high_friction,
    );
}

#[test]
fn recognition_participation_intent_all_participation_types_validate() {
    // Every participationType enum value must be valid.
    for pt in &["reaction", "graduated-feedback", "vote", "block"] {
        let instance = serde_json::json!({
            "entityType": "content",
            "entityId": "content-123",
            "humanId": "agent-abc",
            "mechanismLevel": 2,
            "participationType": pt
        });
        validate_against_schema(
            "intents/recognition-participation-intent.schema.json",
            &instance,
        );
    }
}

#[test]
fn recognition_participation_intent_rejects_missing_required_fields() {
    let schema = load_schema("intents/recognition-participation-intent.schema.json");
    let validator = jsonschema::validator_for(&schema)
        .expect("recognition-participation-intent schema should compile");

    // Missing humanId
    let no_human = serde_json::json!({
        "entityType": "content",
        "entityId": "entity-abc",
        "mechanismLevel": 1,
        "participationType": "reaction"
    });
    assert!(
        validator.iter_errors(&no_human).next().is_some(),
        "Schema should require humanId"
    );

    // Missing mechanismLevel
    let no_level = serde_json::json!({
        "entityType": "content",
        "entityId": "entity-abc",
        "humanId": "agent-abc",
        "participationType": "reaction"
    });
    assert!(
        validator.iter_errors(&no_level).next().is_some(),
        "Schema should require mechanismLevel"
    );

    // Missing participationType
    let no_type = serde_json::json!({
        "entityType": "content",
        "entityId": "entity-abc",
        "humanId": "agent-abc",
        "mechanismLevel": 1
    });
    assert!(
        validator.iter_errors(&no_type).next().is_some(),
        "Schema should require participationType"
    );
}

#[test]
fn recognition_participation_intent_rejects_invalid_level() {
    let schema = load_schema("intents/recognition-participation-intent.schema.json");
    let validator = jsonschema::validator_for(&schema)
        .expect("recognition-participation-intent schema should compile");

    // Level 8 exceeds maximum (0–7)
    let over_max = serde_json::json!({
        "entityType": "content",
        "entityId": "entity-abc",
        "humanId": "agent-abc",
        "mechanismLevel": 8,
        "participationType": "reaction"
    });
    assert!(
        validator.iter_errors(&over_max).next().is_some(),
        "Schema should reject mechanismLevel > 7"
    );

    // Negative level
    let negative = serde_json::json!({
        "entityType": "content",
        "entityId": "entity-abc",
        "humanId": "agent-abc",
        "mechanismLevel": -1,
        "participationType": "reaction"
    });
    assert!(
        validator.iter_errors(&negative).next().is_some(),
        "Schema should reject negative mechanismLevel"
    );
}

#[test]
fn recognition_participation_intent_rejects_invalid_participation_type() {
    let schema = load_schema("intents/recognition-participation-intent.schema.json");
    let validator = jsonschema::validator_for(&schema)
        .expect("recognition-participation-intent schema should compile");

    let bad_type = serde_json::json!({
        "entityType": "content",
        "entityId": "entity-abc",
        "humanId": "agent-abc",
        "mechanismLevel": 1,
        "participationType": "thumbs-up"
    });
    assert!(
        validator.iter_errors(&bad_type).next().is_some(),
        "Schema should reject unknown participationType"
    );
}

#[test]
fn recognition_participation_intent_schema_declares_source_of_truth() {
    let path = schema_dir().join("intents/recognition-participation-intent.schema.json");
    let content = fs::read_to_string(&path)
        .expect("intents/recognition-participation-intent.schema.json must exist");
    let schema_value: Value = serde_json::from_str(&content)
        .expect("intents/recognition-participation-intent.schema.json must be valid JSON");
    assert_source_of_truth_declared(
        &schema_value,
        "intents/recognition-participation-intent.schema.json",
    );
}

#[test]
fn standing_policy_payload_with_recognition_weight_by_level_validates() {
    // Extended standing-policy payload with recognition_weight_by_level (M-REA-3 addition)
    // alongside the existing accumulation_thresholds (M-POLICY-1). Both are optional,
    // either can be omitted, and both can coexist.
    let payload = serde_json::json!({
        "floor": {
            "classes": [
                { "class": "local-relationship-reach",        "protection": "unconditional family/household propagation" },
                { "class": "cid-targeted-lookup",             "protection": "anyone with a CID can fetch" },
                { "class": "constitutional-floor-signatures", "protection": "mishpat decisions are always verified" },
                { "class": "new-voice-baseline",              "protection": "first-time author has nonzero floor" },
                { "class": "vulnerable-class-elevation",      "protection": "children and displaced persons get elevated floor" }
            ]
        },
        "recognition_weight_by_level": {
            "L0": 0.1,
            "L1": 0.1,
            "L2": 0.3,
            "L3": 0.5,
            "L4": 0.7,
            "L5": 0.7,
            "L6": 0.8,
            "L7": 1.0
        },
        "accumulation_thresholds": {
            "readyForSensemaking": { "minTotalSignals": 20, "maxConsensusStrength": 0.7 },
            "controversyDetected": { "minTotalSignals": 10, "maxConsensusStrength": 0.3 },
            "settled":             { "minTotalSignals": 30, "maxConsensusStrength": 0.85 }
        }
    });
    validate_against_schema("manifest-payloads/standing-policy.schema.json", &payload);
}

#[test]
fn standing_policy_with_weight_levels_only_validates() {
    // recognition_weight_by_level alone (no accumulation_thresholds) is valid.
    // Partial level maps are also valid (additionalProperties: false but all level
    // keys are optional).
    let payload = serde_json::json!({
        "floor": {
            "classes": [
                { "class": "local-relationship-reach",        "protection": "p1" },
                { "class": "cid-targeted-lookup",             "protection": "p2" },
                { "class": "constitutional-floor-signatures", "protection": "p3" },
                { "class": "new-voice-baseline",              "protection": "p4" },
                { "class": "vulnerable-class-elevation",      "protection": "p5" }
            ]
        },
        "recognition_weight_by_level": {
            "L1": 0.1,
            "L7": 1.0
        }
    });
    validate_against_schema("manifest-payloads/standing-policy.schema.json", &payload);
}

// ─────────────────────────────────────────────────────────────────────────────
// M-AGGR-3: ContentEngagementStatsView schema tests
// Source of truth: derived projection of EconomicEvent stream filtered by
// (content_id, lamadEventType IN ['content-view', 'content-complete']).
// ─────────────────────────────────────────────────────────────────────────────

fn minimal_content_engagement_stats() -> serde_json::Value {
    serde_json::json!({
        "contentId": "concept-foundations-1",
        "views": 42,
        "completions": 17,
        "uniqueViewers": 38,
        "completionRate": 0.405,
        "computedAt": "2026-05-28T12:00:00Z"
    })
}

#[test]
fn content_engagement_stats_view_minimal_validates() {
    validate_against_schema(
        "views/content-engagement-stats-view.schema.json",
        &minimal_content_engagement_stats(),
    );
}

#[test]
fn content_engagement_stats_view_zero_counts_validates() {
    let zero = serde_json::json!({
        "contentId": "new-content-no-views",
        "views": 0,
        "completions": 0,
        "uniqueViewers": 0,
        "completionRate": 0.0,
        "computedAt": "2026-05-28T10:00:00Z"
    });
    validate_against_schema("views/content-engagement-stats-view.schema.json", &zero);
}

#[test]
fn content_engagement_stats_view_full_completion_validates() {
    // Every viewer also completed — 100% rate
    let full = serde_json::json!({
        "contentId": "lesson-mastery-complete",
        "views": 10,
        "completions": 10,
        "uniqueViewers": 10,
        "completionRate": 1.0,
        "computedAt": "2026-05-28T14:00:00Z"
    });
    validate_against_schema("views/content-engagement-stats-view.schema.json", &full);
}

#[test]
fn content_engagement_stats_view_rejects_missing_required_fields() {
    let schema = load_schema("views/content-engagement-stats-view.schema.json");
    let validator =
        jsonschema::validator_for(&schema).expect("content-engagement-stats schema should compile");

    // Missing views
    let no_views = {
        let mut v = minimal_content_engagement_stats();
        v.as_object_mut().unwrap().remove("views");
        v
    };
    assert!(
        validator.iter_errors(&no_views).next().is_some(),
        "Schema should require 'views'"
    );

    // Missing contentId
    let no_content_id = {
        let mut v = minimal_content_engagement_stats();
        v.as_object_mut().unwrap().remove("contentId");
        v
    };
    assert!(
        validator.iter_errors(&no_content_id).next().is_some(),
        "Schema should require 'contentId'"
    );

    // Missing completionRate
    let no_rate = {
        let mut v = minimal_content_engagement_stats();
        v.as_object_mut().unwrap().remove("completionRate");
        v
    };
    assert!(
        validator.iter_errors(&no_rate).next().is_some(),
        "Schema should require 'completionRate'"
    );
}

#[test]
fn content_engagement_stats_schema_loads() {
    let _ = load_schema("views/content-engagement-stats-view.schema.json");
}

// ── M-AGGR-2: SourceChainEntryView ──────────────────────────────

/// Build a minimal valid SourceChainEntryView JSON.
fn minimal_source_chain_entry() -> serde_json::Value {
    serde_json::json!({
        "actionHash": "uhCkkABCDEFGHIJKLMNOP",
        "entryHash": "uhCEkABCDEFGHIJKLMNOP",
        "authorAgent": "uhCAkABCDEFGHIJKLMNOP",
        "entryType": "Create:App(0)",
        "contentJson": null,
        "prevActionHash": null,
        "sequence": 0,
        "timestamp": "1748000000000000"
    })
}

#[test]
fn source_chain_entry_view_schema_loads() {
    let schema = load_schema("views/source-chain-entry-view.schema.json");
    assert_source_of_truth_declared(&schema, "source-chain-entry-view");
}

#[test]
fn source_chain_entry_view_minimal_valid() {
    let instance = minimal_source_chain_entry();
    validate_against_schema("views/source-chain-entry-view.schema.json", &instance);
}

#[test]
fn source_chain_entry_view_with_content_json() {
    let mut instance = minimal_source_chain_entry();
    instance["contentJson"] = serde_json::json!(r#"{"id":"human-1","displayName":"Alice"}"#);
    instance["prevActionHash"] = serde_json::json!("uhCkkPREVACTIONHASH");
    instance["sequence"] = serde_json::json!(3u32);
    instance["entryType"] = serde_json::json!("Create:App(1)");
    validate_against_schema("views/source-chain-entry-view.schema.json", &instance);
}

#[test]
fn source_chain_entry_view_requires_action_hash() {
    let schema = load_schema("views/source-chain-entry-view.schema.json");
    let validator = jsonschema::validator_for(&schema).unwrap();
    let mut instance = minimal_source_chain_entry();
    instance.as_object_mut().unwrap().remove("actionHash");
    assert!(
        validator.iter_errors(&instance).next().is_some(),
        "Schema should require 'actionHash'"
    );
}

#[test]
fn source_chain_entry_view_requires_sequence() {
    let schema = load_schema("views/source-chain-entry-view.schema.json");
    let validator = jsonschema::validator_for(&schema).unwrap();
    let mut instance = minimal_source_chain_entry();
    instance.as_object_mut().unwrap().remove("sequence");
    assert!(
        validator.iter_errors(&instance).next().is_some(),
        "Schema should require 'sequence'"
    );
}

// ── M-AGGR-2: EntryLinkView ────────────────────────────────────

/// Build a minimal valid EntryLinkView JSON.
fn minimal_entry_link() -> serde_json::Value {
    serde_json::json!({
        "linkHash": "uhCkkLINKHASH",
        "baseHash": "uhCkkBASEHASH",
        "targetHash": "uhCkkTARGETHASH",
        "linkType": "0",
        "tag": null,
        "authorAgent": "uhCAkABCDEFGHIJKLMNOP",
        "timestamp": "1748000000000000",
        "deleted": false,
        "deletedAt": null
    })
}

#[test]
fn entry_link_view_schema_loads() {
    let schema = load_schema("views/entry-link-view.schema.json");
    assert_source_of_truth_declared(&schema, "entry-link-view");
}

#[test]
fn entry_link_view_minimal_valid() {
    let instance = minimal_entry_link();
    validate_against_schema("views/entry-link-view.schema.json", &instance);
}

#[test]
fn entry_link_view_with_tag_and_deletion() {
    let mut instance = minimal_entry_link();
    instance["tag"] = serde_json::json!("dGFnYnl0ZXM="); // base64("tagbytes")
    instance["deleted"] = serde_json::json!(true);
    instance["deletedAt"] = serde_json::json!("1748001000000000");
    instance["linkType"] = serde_json::json!("5");
    validate_against_schema("views/entry-link-view.schema.json", &instance);
}

#[test]
fn entry_link_view_requires_link_hash() {
    let schema = load_schema("views/entry-link-view.schema.json");
    let validator = jsonschema::validator_for(&schema).unwrap();
    let mut instance = minimal_entry_link();
    instance.as_object_mut().unwrap().remove("linkHash");
    assert!(
        validator.iter_errors(&instance).next().is_some(),
        "Schema should require 'linkHash'"
    );
}

#[test]
fn entry_link_view_requires_deleted() {
    let schema = load_schema("views/entry-link-view.schema.json");
    let validator = jsonschema::validator_for(&schema).unwrap();
    let mut instance = minimal_entry_link();
    instance.as_object_mut().unwrap().remove("deleted");
    assert!(
        validator.iter_errors(&instance).next().is_some(),
        "Schema should require 'deleted'"
    );
}

// ── M-AGGR-1: SessionHumanView ──────────────────────────────────

/// Build a minimal valid SessionHumanView JSON instance.
fn minimal_session_human() -> serde_json::Value {
    serde_json::json!({
        "agentId": "uhCAkABCDEFGHIJKLMNOP1234567890",
        "hAppId": "lamad",
        "nodesViewed": 0,
        "nodesWithAffinity": 0,
        "pathsStarted": 0,
        "pathsCompleted": 0,
        "stepsCompleted": 0,
        "computedAt": "2026-05-28T12:00:00Z"
    })
}

#[test]
fn session_human_view_minimal_validates() {
    validate_against_schema(
        "views/session-human-view.schema.json",
        &minimal_session_human(),
    );
}

#[test]
fn session_human_view_with_journey_validates() {
    let mut v = minimal_session_human();
    v["journeyStartedAt"] = serde_json::json!("2026-01-15T08:30:00Z");
    v["nodesViewed"] = serde_json::json!(12_i64);
    v["pathsStarted"] = serde_json::json!(3_i64);
    v["stepsCompleted"] = serde_json::json!(47_i64);
    validate_against_schema("views/session-human-view.schema.json", &v);
}

#[test]
fn session_human_view_null_journey_validates() {
    let mut v = minimal_session_human();
    v["journeyStartedAt"] = serde_json::json!(null);
    validate_against_schema("views/session-human-view.schema.json", &v);
}

#[test]
fn session_human_view_rejects_missing_required_fields() {
    let schema = load_schema("views/session-human-view.schema.json");
    let validator =
        jsonschema::validator_for(&schema).expect("session-human-view schema should compile");

    for field in &[
        "agentId",
        "hAppId",
        "nodesViewed",
        "nodesWithAffinity",
        "pathsStarted",
        "pathsCompleted",
        "stepsCompleted",
        "computedAt",
    ] {
        let mut v = minimal_session_human();
        v.as_object_mut().unwrap().remove(*field);
        assert!(
            validator.iter_errors(&v).next().is_some(),
            "Schema should require '{field}'"
        );
    }
}

#[test]
fn session_human_view_schema_declares_source_of_truth() {
    let schema = load_schema("views/session-human-view.schema.json");
    assert_source_of_truth_declared(&schema, "session-human-view");
}

// ── M-AGGR-1: UpgradePromptView ─────────────────────────────────

/// Build a minimal valid UpgradePromptView JSON instance (empty active prompts).
fn minimal_upgrade_prompt() -> serde_json::Value {
    serde_json::json!({
        "agentId": "uhCAkABCDEFGHIJKLMNOP1234567890",
        "hAppId": "lamad",
        "activePrompts": [],
        "computedAt": "2026-05-28T12:00:00Z"
    })
}

#[test]
fn upgrade_prompt_view_minimal_validates() {
    validate_against_schema(
        "views/upgrade-prompt-view.schema.json",
        &minimal_upgrade_prompt(),
    );
}

#[test]
fn upgrade_prompt_view_with_prompts_validates() {
    let v = serde_json::json!({
        "agentId": "uhCAkABCDEFGHIJKLMNOP1234567890",
        "hAppId": "lamad",
        "activePrompts": [
            {
                "promptId": "first-affinity",
                "trigger": "contributor-presence",
                "title": "Save Your Progress",
                "message": "You're building a personal knowledge map!",
                "benefits": [
                    "Your progress syncs across devices",
                    "Join a network of learners"
                ]
            },
            {
                "promptId": "path-started",
                "trigger": "path-started",
                "title": "You've Started a Journey",
                "message": "Your learning path is stored in your browser.",
                "benefits": ["Resume from any device"]
            }
        ],
        "computedAt": "2026-05-28T14:30:00Z"
    });
    validate_against_schema("views/upgrade-prompt-view.schema.json", &v);
}

#[test]
fn upgrade_prompt_view_rejects_missing_required_fields() {
    let schema = load_schema("views/upgrade-prompt-view.schema.json");
    let validator =
        jsonschema::validator_for(&schema).expect("upgrade-prompt-view schema should compile");

    for field in &["agentId", "hAppId", "activePrompts", "computedAt"] {
        let mut v = minimal_upgrade_prompt();
        v.as_object_mut().unwrap().remove(*field);
        assert!(
            validator.iter_errors(&v).next().is_some(),
            "Schema should require '{field}'"
        );
    }
}

#[test]
fn upgrade_prompt_view_schema_declares_source_of_truth() {
    let schema = load_schema("views/upgrade-prompt-view.schema.json");
    assert_source_of_truth_declared(&schema, "upgrade-prompt-view");
}

// ── M-AGGR-1: OnboardingManifestPayload ─────────────────────────

/// Build a minimal valid onboarding manifest payload instance.
fn minimal_onboarding_payload() -> serde_json::Value {
    serde_json::json!({
        "prompts": []
    })
}

#[test]
fn onboarding_manifest_payload_minimal_validates() {
    validate_against_schema(
        "manifest-payloads/onboarding.schema.json",
        &minimal_onboarding_payload(),
    );
}

#[test]
fn onboarding_manifest_payload_with_prompts_validates() {
    let v = serde_json::json!({
        "prompts": [
            {
                "promptId": "first-affinity",
                "trigger": "contributor-presence",
                "triggerCondition": {
                    "field": "nodesWithAffinity",
                    "operator": "gte",
                    "threshold": 1
                },
                "title": "Save Your Progress",
                "message": "You're building a personal knowledge map! Install the Elohim app to save it permanently.",
                "benefits": [
                    "Your progress syncs across devices",
                    "Join a network of learners",
                    "Never lose your journey"
                ]
            },
            {
                "promptId": "path-started",
                "trigger": "path-started",
                "triggerCondition": {
                    "field": "pathsStarted",
                    "operator": "gte",
                    "threshold": 1
                },
                "title": "You've Started a Journey",
                "message": "Your learning path is stored in your browser.",
                "benefits": ["Resume from any device", "Get updates to your paths"]
            },
            {
                "promptId": "return-visit",
                "trigger": "return-visit",
                "triggerCondition": {
                    "field": "nodesViewed",
                    "operator": "always",
                    "threshold": 0
                },
                "title": "Welcome Back!",
                "message": "Good to see you again.",
                "benefits": ["Automatic progress backup"]
            }
        ]
    });
    validate_against_schema("manifest-payloads/onboarding.schema.json", &v);
}

#[test]
fn onboarding_manifest_payload_rejects_missing_prompts() {
    let schema = load_schema("manifest-payloads/onboarding.schema.json");
    let validator = jsonschema::validator_for(&schema)
        .expect("onboarding manifest-payload schema should compile");

    // Missing prompts array entirely
    let no_prompts = serde_json::json!({});
    assert!(
        validator.iter_errors(&no_prompts).next().is_some(),
        "Schema should require 'prompts'"
    );
}

// =============================================================================
// Sprint 2 Task 1: BoundsValidationResultView schema contract tests
//
// Ticket: Sprint 2 Task 1 — View schemas + ts-rs Rust structs
// Source: genesis/docs/superpowers/plans/2026-05-28-sprint2-bounds-validator-standing-aggregator.md §1
//
// Validates bounds-validation-result-view.schema.json against realistic
// instances covering the pass and fail (violation) branches. This view is
// the wire shape for POST /api/v1/diagnostics/validate-bounds; source of
// truth is pure function output — no persisted entity.
// =============================================================================

#[test]
fn bounds_validation_result_passing_validates() {
    // All checks pass — violation is absent (null).
    let passing = serde_json::json!({
        "pass": true,
        "commitment_cid": "uhCEkSomeCommitmentCid",
        "violation": null,
        "checks": {
            "commitment_found": true,
            "active": true,
            "scope_includes_event": true,
            "reach_ceiling_ok": true,
            "rate_within_limit": true,
            "key_rotation_current": true,
            "not_revoked": true
        }
    });
    validate_against_schema("views/bounds-validation-result-view.schema.json", &passing);
}

#[test]
fn bounds_validation_result_failing_with_violation_validates() {
    // Validation fails — violation present, checks short-circuit.
    let failing = serde_json::json!({
        "pass": false,
        "commitment_cid": "uhCEkAnotherCommitmentCid",
        "violation": {
            "kind": "reach_ceiling_exceeded",
            "summary": "EconomicEvent reach (Trusted) exceeds Commitment ceiling (High)."
        },
        "checks": {
            "commitment_found": true,
            "active": true,
            "scope_includes_event": true,
            "reach_ceiling_ok": false,
            "rate_within_limit": false,
            "key_rotation_current": false,
            "not_revoked": false
        }
    });
    validate_against_schema("views/bounds-validation-result-view.schema.json", &failing);
}

#[test]
fn bounds_validation_result_all_violation_kinds_valid() {
    // Every ViolationKind enum value must be accepted by the schema.
    for kind in &[
        "commitment_inactive",
        "scope_not_included",
        "reach_ceiling_exceeded",
        "rate_limit_exceeded",
        "key_rotation_stale",
        "commitment_revoked",
        "commitment_not_found",
    ] {
        let instance = serde_json::json!({
            "pass": false,
            "commitment_cid": "uhCEkCid",
            "violation": {
                "kind": kind,
                "summary": format!("Violated: {}", kind)
            },
            "checks": {
                "commitment_found": true,
                "active": false,
                "scope_includes_event": false,
                "reach_ceiling_ok": false,
                "rate_within_limit": false,
                "key_rotation_current": false,
                "not_revoked": false
            }
        });
        validate_against_schema("views/bounds-validation-result-view.schema.json", &instance);
    }
}

#[test]
fn bounds_validation_result_rejects_missing_required_fields() {
    let schema = load_schema("views/bounds-validation-result-view.schema.json");
    let validator = jsonschema::validator_for(&schema)
        .expect("bounds-validation-result-view schema should compile");

    // Missing checks
    let no_checks = serde_json::json!({
        "pass": true,
        "commitment_cid": "uhCEkCid"
    });
    assert!(
        validator.iter_errors(&no_checks).next().is_some(),
        "Schema should require 'checks'"
    );

    // Missing commitment_cid
    let no_cid = serde_json::json!({
        "pass": true,
        "checks": {
            "commitment_found": true,
            "active": true,
            "scope_includes_event": true,
            "reach_ceiling_ok": true,
            "rate_within_limit": true,
            "key_rotation_current": true,
            "not_revoked": true
        }
    });
    assert!(
        validator.iter_errors(&no_cid).next().is_some(),
        "Schema should require 'commitment_cid'"
    );
}

// =============================================================================
// Sprint 2 Task 1: StandingScoreView schema contract tests
//
// Ticket: Sprint 2 Task 1 — View schemas + ts-rs Rust structs
// Source: genesis/docs/superpowers/plans/2026-05-28-sprint2-bounds-validator-standing-aggregator.md §1
//
// Validates standing-score-view.schema.json against realistic instances.
// This view is the wire shape for GET /api/v1/standing/{agent_cid}; source
// of truth is the local SQLite operational projection (standing_view table),
// recomputable from the FeedbackSignal subgraph (Holochain DHT).
// =============================================================================

#[test]
fn standing_score_view_neutral_no_breaches_validates() {
    // Cold-start or clean slate — Neutral with empty breach list.
    let neutral = serde_json::json!({
        "evaluator_cid": "uhCAkEvaluatorAgent",
        "subject_cid": "uhCAkSubjectAgent",
        "score": "Neutral",
        "recent_breaches": [],
        "computed_at": "2026-05-28T10:00:00Z"
    });
    validate_against_schema("views/standing-score-view.schema.json", &neutral);
}

#[test]
fn standing_score_view_floor_with_breaches_validates() {
    // Subject is at Floor standing with recent breach evidence.
    let floor = serde_json::json!({
        "evaluator_cid": "uhCAkEvaluatorAgent",
        "subject_cid": "uhCAkFloorSubject",
        "score": "Floor",
        "debit_weight_sum": 42,
        "recent_breaches": [
            {
                "signal_kind": "compute:capacity-breach",
                "emitted_at": "2026-05-27T08:30:00Z",
                "weight": 20,
                "evidence_summary": "Node reported capacity at 3% for 6h window."
            },
            {
                "signal_kind": "compute:replication-shortfall",
                "emitted_at": "2026-05-26T14:00:00Z",
                "weight": 22
            }
        ],
        "computed_at": "2026-05-28T10:05:00Z"
    });
    validate_against_schema("views/standing-score-view.schema.json", &floor);
}

#[test]
fn standing_score_view_all_tiers_valid() {
    // Every StandingScoreTier value must be accepted.
    for tier in &["Unknown", "Floor", "Low", "Neutral", "High", "Trusted"] {
        let instance = serde_json::json!({
            "evaluator_cid": "uhCAkEvaluatorAgent",
            "subject_cid": "uhCAkSubjectAgent",
            "score": tier,
            "recent_breaches": [],
            "computed_at": "2026-05-28T10:00:00Z"
        });
        validate_against_schema("views/standing-score-view.schema.json", &instance);
    }
}

#[test]
fn standing_score_view_rejects_missing_required_fields() {
    let schema = load_schema("views/standing-score-view.schema.json");
    let validator =
        jsonschema::validator_for(&schema).expect("standing-score-view schema should compile");

    // Missing score
    let no_score = serde_json::json!({
        "evaluator_cid": "uhCAkEval",
        "subject_cid": "uhCAkSubject",
        "recent_breaches": [],
        "computed_at": "2026-05-28T10:00:00Z"
    });
    assert!(
        validator.iter_errors(&no_score).next().is_some(),
        "Schema should require 'score'"
    );

    // Missing computed_at
    let no_timestamp = serde_json::json!({
        "evaluator_cid": "uhCAkEval",
        "subject_cid": "uhCAkSubject",
        "score": "Neutral",
        "recent_breaches": []
    });
    assert!(
        validator.iter_errors(&no_timestamp).next().is_some(),
        "Schema should require 'computed_at'"
    );
}

// =============================================================================
// Sprint 3: replicates-dwelling Commitment schema
// =============================================================================

#[test]
fn replicates_dwelling_minimal_steward_mutual_validates() {
    let payload = serde_json::json!({
        "action": "replicates-dwelling",
        "provider_dwelling_hub_id": "hub:A",
        "recipient_dwelling_hub_id": "hub:B",
        "provider_role": "steward_mutual",
        "capacity_bytes": 50_000_000_000u64,
        "scope_filter": {"epr_kinds": ["Content"], "bytes_per_blob_max": 1_000_000_000u64},
        "valid_from": "2026-05-28T00:00:00Z",
        "valid_until": "2026-08-26T00:00:00Z",
        "grace_period_days": 14,
        "rotation_ttl_days": 90,
        "ratio_attestation": {
            "commons_pct": 20,
            "dwelling_pct": 40,
            "collective_pct": 25,
            "free_pct": 15,
            "effective_ratio_cid": "bafkrei-manifest-abc"
        }
    });
    validate_against_schema("commitments/replicates-dwelling.schema.json", &payload);
}

#[test]
fn replicates_dwelling_collective_steward_validates() {
    let payload = serde_json::json!({
        "action": "replicates-dwelling",
        "provider_dwelling_hub_id": "hub:church-server",
        "recipient_dwelling_hub_id": "hub:member-family",
        "provider_role": "collective_steward",
        "via_collective_hub_id": "collective:saint-marys",
        "capacity_bytes": 100_000_000_000u64,
        "scope_filter": {"epr_kinds": ["Content"]},
        "valid_from": "2026-05-28T00:00:00Z",
        "valid_until": "2026-08-26T00:00:00Z",
        "grace_period_days": 14,
        "rotation_ttl_days": 90,
        "ratio_attestation": {
            "commons_pct": 20,
            "dwelling_pct": 40,
            "collective_pct": 25,
            "free_pct": 15,
            "effective_ratio_cid": "bafkrei-manifest-abc"
        }
    });
    validate_against_schema("commitments/replicates-dwelling.schema.json", &payload);
}

#[test]
fn replicates_dwelling_rejects_unknown_provider_role() {
    let payload = serde_json::json!({
        "action": "replicates-dwelling",
        "provider_dwelling_hub_id": "hub:A",
        "recipient_dwelling_hub_id": "hub:B",
        "provider_role": "totally-bogus",
        "capacity_bytes": 1,
        "scope_filter": {},
        "valid_from": "2026-05-28T00:00:00Z",
        "valid_until": "2026-08-26T00:00:00Z",
        "grace_period_days": 14,
        "rotation_ttl_days": 90,
        "ratio_attestation": {
            "commons_pct": 20, "dwelling_pct": 40, "collective_pct": 25, "free_pct": 15,
            "effective_ratio_cid": "bafkrei-manifest-abc"
        }
    });
    let schema = load_schema("commitments/replicates-dwelling.schema.json");
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors: Vec<_> = validator.iter_errors(&payload).collect();
    assert!(
        !errors.is_empty(),
        "Schema must reject unknown provider_role 'totally-bogus'"
    );
}

// =============================================================================
// Sprint 3: PeerCapacityView + HubCapacityView schema contract tests
//
// Source of truth: computed projection from infrastructure:system-sample
// (raw capacity), Mishpat::Commitment entries (pledges), peer_blob_inventory
// (actually held). Operational Category C — no persisted entity; recomputable.
// =============================================================================

#[test]
fn peer_capacity_view_minimal_validates() {
    let payload = serde_json::json!({
        "peerCid": "peer:abc",
        "computedAt": "2026-05-28T12:00:00Z",
        "totalRawBytes": 100_000_000_000u64,
        "pledges": {"dwellingBytes": 0, "collectiveBytes": 0, "commonsBytes": 0, "totalPledgedBytes": 0},
        "actuallyHeld": {"uniqueShardBytes": 0, "freeBytesRemaining": 100_000_000_000u64, "fragmentationEstimate": 0.0},
        "ratioCompliance": {
            "effectiveRatios": {"commonsPct": 20, "dwellingPct": 40, "collectivePct": 25, "freePct": 15, "manifestCid": "bafkrei-x"},
            "currentRatios": {"commonsPct": 0, "dwellingPct": 0, "collectivePct": 0, "freePct": 100},
            "compliantWithDonut": false,
            "violations": []
        }
    });
    validate_against_schema("views/peer-capacity-view.schema.json", &payload);
}

#[test]
fn hub_capacity_view_dwelling_kind_validates() {
    let payload = serde_json::json!({
        "hubId": "hub:family-smiths",
        "hubKind": "dwelling",
        "memberDeviceCount": 2,
        "capacity": null
    });
    validate_against_schema("views/hub-capacity-view.schema.json", &payload);
}

// ── Sprint 3: Storage-replication fields in topology view schemas ─────────────

#[test]
fn distribution_summary_with_projection_tier_validates() {
    let payload = serde_json::json!({
        "replicaCount": 11,
        "replicaTarget": 11,
        "replicaHealth": "over_replicated",
        "projectorCount": 3,
        "reachClass": "trusted",
        "diversityHint": {"kind": "collective_member_count", "value": 2},
        "thisFetchSource": "peer_direct",
        "lastVerifiedSeconds": 30,
        "projectionTier": "regional"
    });
    validate_against_schema("views/distribution-summary.schema.json", &payload);
}

#[test]
fn household_resilience_with_commitment_backed_replication_validates() {
    let payload = serde_json::json!({
        "contentId": "bafkrei-content-x",
        "householdsStewarding": 3,
        "householdsReciprocated": 2,
        "protectionStatus": "protected",
        "details": {"stewardHouseholds": ["hub:A","hub:B","hub:C"], "onlinePeerCount": 5, "healthScore": 0.95},
        "commitmentBackedReplication": {
            "dwellingCommitments": 3, "collectiveCommitments": 1, "commonsCommitments": 0, "totalPledgedBytes": 150_000_000_000u64
        }
    });
    validate_against_schema("views/household-resilience-view.schema.json", &payload);
}

// ── Content-graph view (native content-graph seam, Task A9) ───────────────────
//
// Source of truth: multi-provenance composite read (explicit edges are Category A
// DHT Relationship entries; computed edges are Category C derived-on-read). The
// view is a read projection, never persisted. `inferenceSource` is validated
// against the cross-file `../enums/inference-source.schema.json` $ref, which the
// harness resolves via inline_refs (enums/ is in the ref map). `totalNodes` is
// the neighbour count (root excluded — `related.len()` flat-read semantics).

#[test]
fn content_graph_view_matches_schema() {
    use elohim_views::lamad::{ContentGraphNodeView, ContentGraphView};

    let view = ContentGraphView {
        root_id: "confession".to_string(),
        related: vec![
            // Explicit, author-declared edge (Category A DHT Relationship).
            ContentGraphNodeView {
                content_id: "theology".to_string(),
                relationship_type: "RELATES_TO".to_string(),
                confidence: 1.0,
                inference_source: "explicit".to_string(),
                depth: 1,
                children: vec![],
            },
            // Computed shared-tag proximity edge (Category C derived-on-read).
            ContentGraphNodeView {
                content_id: "repentance".to_string(),
                relationship_type: "RELATES_TO".to_string(),
                confidence: 0.42,
                inference_source: "tag".to_string(),
                depth: 1,
                children: vec![],
            },
        ],
        total_nodes: 2,
    };

    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/content-graph.schema.json", &json);
}

// ── Verdict spine (verdict-view, Category C — the gradient answer projection) ──

#[test]
fn verdict_view_matches_schema() {
    use elohim_epr::verdict::{
        CheckOutcome, CheckWitness, Decision, ReferQuestion, ReferReason, Verdict, Witness,
    };

    // Refer variant (the ceiling marker) with a failed + a passed witness check —
    // exercises the richest wire shape (decision payload + observed values).
    let refer = Verdict {
        axis: "epistemic".to_string(),
        subject: Some("bafyreisubjectcid0000000000000000000000000000000000000000000".to_string()),
        decision: Decision::Refer(ReferQuestion {
            layer: "community".to_string(),
            reason: ReferReason::ContestedEvidence,
            note: Some("dissent share over the contest threshold".to_string()),
        }),
        witness: Witness {
            checks: vec![
                CheckWitness {
                    check_id: "contest-ratio".to_string(),
                    outcome: CheckOutcome::Failed,
                    summary: "dismiss weight over contest threshold".to_string(),
                    observed: Some(serde_json::json!({ "dissentRatio": 0.4 })),
                },
                CheckWitness {
                    check_id: "review-tally".to_string(),
                    outcome: CheckOutcome::Passed,
                    summary: "4 reviews recorded".to_string(),
                    observed: None,
                },
            ],
        },
        policy_ref: Some("epr:policy:epistemic:v1".to_string()),
    };
    validate_against_schema(
        "views/verdict-view.schema.json",
        &serde_json::to_value(&refer).unwrap(),
    );

    // Permit variant: the decision carries only the discriminator, no subject/policyRef.
    let permit = Verdict {
        axis: "reach".to_string(),
        subject: None,
        decision: Decision::Permit,
        witness: Witness { checks: vec![] },
        policy_ref: None,
    };
    validate_against_schema(
        "views/verdict-view.schema.json",
        &serde_json::to_value(&permit).unwrap(),
    );

    assert_source_of_truth_declared(
        &load_schema("views/verdict-view.schema.json"),
        "verdict-view.schema.json",
    );
}
