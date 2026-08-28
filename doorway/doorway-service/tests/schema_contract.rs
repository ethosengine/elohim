//! Schema contract tests — validates that the doorway /auth/* response
//! structs' Rust serialization matches the JSON Schema source of truth in
//! elohim/sdk/schemas/v1/views/.
//!
//! Mirrors the harness in elohim/elohim-storage/tests/schema_contract.rs
//! (per-crate harness, shared schemas directory). These tests catch drift
//! between the auth wire shapes (auth_routes.rs serialize structs —
//! Category C operational session state, HTTP wire only) and the schema
//! contract. If a field is renamed, added, or removed in Rust without
//! updating the schema (or vice versa), these tests fail.
//!
//! The harness inlines `$ref` before compiling because the jsonschema
//! crate doesn't resolve file-based references automatically.

use doorway::routes::auth_routes::{
    AccountResponse, AuthResponse, AuthorityRef, ExchangeSessionResponse, HumanProfileResponse,
    MeResponse, SessionTokenResponse,
};
use doorway::routes::auth_discovery::{AuthDiscovery, AuthEndpoints};
use doorway::routes::health::P2PHealth;
use doorway::routes::self_healing::{
    AdmissionView, ConductorView, PeerView, ProjectorView, RenderView, SelfHealingView,
    UpstreamPolicyView, UpstreamView, WarmupView,
};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Resolve the shared protocol schema directory relative to this crate.
fn schema_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("../../elohim/sdk/schemas/v1")
}

/// Load schemas from the subdirectories the auth views can reference into a
/// ref map keyed by every relative-path form plus `$id` (same key forms as
/// the elohim-storage harness).
fn load_ref_map() -> HashMap<String, Value> {
    let base = schema_dir();
    let mut refs = HashMap::new();

    for subdir in &["enums", "views", "objects"] {
        let dir = base.join(subdir);
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(schema) = serde_json::from_str::<Value>(&content) {
                            let filename = path.file_name().unwrap().to_str().unwrap().to_string();
                            // Same-dir ref: "authority-ref.schema.json"
                            refs.insert(filename.clone(), schema.clone());
                            // Same-dir ref with leading ./
                            refs.insert(format!("./{}", filename), schema.clone());
                            // Cross-dir ref: "../enums/reach.schema.json"
                            refs.insert(format!("../{}/{}", subdir, filename), schema.clone());
                            // From repo-relative form: "views/authority-ref.schema.json"
                            refs.insert(format!("{}/{}", subdir, filename), schema.clone());
                            // URI-style $id ref: "epr:schema:view:authority-ref"
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
                    // Split file portion from JSON Pointer fragment
                    let (file_part, fragment) = match ref_path.split_once('#') {
                        Some((file, frag)) => (file, Some(frag)),
                        None => (ref_path.as_str(), None),
                    };
                    if let Some(referenced) = refs.get(file_part) {
                        let mut inlined = referenced.clone();
                        if let Value::Object(ref mut obj) = inlined {
                            obj.remove("$id");
                            obj.remove("$schema");
                        }
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

fn sample_profile() -> HumanProfileResponse {
    HumanProfileResponse {
        id: "human-matthew".to_string(),
        display_name: "Matthew".to_string(),
        bio: Some("Learning steward".to_string()),
        affinities: vec!["systems".to_string(), "gardening".to_string()],
        profile_reach: "public".to_string(),
        location: Some("us-central".to_string()),
        created_at: "2026-06-10T00:00:00Z".to_string(),
        updated_at: "2026-06-10T00:00:00Z".to_string(),
    }
}

// ── AuthResponse (POST /auth/register | /auth/login | /auth/refresh) ────

#[test]
fn auth_response_full_matches_schema() {
    let response = AuthResponse {
        token: "eyJhbGciOiJIUzI1NiJ9.payload.sig".to_string(),
        human_id: "human-matthew".to_string(),
        agent_pub_key: "uhCAkABCDEFGHIJKLMNOPQRSTUVWXYZ01234567890123456789012".to_string(),
        identifier: "matthew@example.com".to_string(),
        expires_at: 1_780_000_000,
        doorway_id: Some("alpha".to_string()),
        doorway_url: Some("https://doorway-alpha.elohim.host".to_string()),
        installed_app_id: Some("elohim-human-matthew".to_string()),
        profile: Some(sample_profile()),
        is_steward: true,
        portal_host_url: Some("https://portal.matthew.example".to_string()),
    };

    let json = serde_json::to_value(&response).unwrap();
    validate_against_schema("views/auth-response.schema.json", &json);
}

#[test]
fn auth_response_minimal_matches_schema() {
    // Login path without federation config, profile, or stewardship: every
    // skip_serializing_if field must be ABSENT (not null) on the wire.
    let response = AuthResponse {
        token: "eyJhbGciOiJIUzI1NiJ9.payload.sig".to_string(),
        human_id: "human-visitor".to_string(),
        agent_pub_key: "uhCAkVISITOR0123456789012345678901234567890123456789012".to_string(),
        identifier: "visitor@example.com".to_string(),
        expires_at: 1_780_000_000,
        doorway_id: None,
        doorway_url: None,
        installed_app_id: None,
        profile: None,
        is_steward: false,
        portal_host_url: None,
    };

    let json = serde_json::to_value(&response).unwrap();
    let obj = json.as_object().unwrap();
    // `isSteward` is deliberately NOT in the absent set: it always serializes
    // (e6b74e684) — a hosted visitor's client must read an explicit
    // `isSteward: false` to select the visitor surface, since an omitted field
    // is indistinguishable from an old doorway that never emitted the claim.
    for absent in [
        "doorwayId",
        "doorwayUrl",
        "installedAppId",
        "profile",
        "portalHostUrl",
    ] {
        assert!(
            !obj.contains_key(absent),
            "{} must be absent when None (serde skip_serializing_if)",
            absent
        );
    }
    assert_eq!(
        obj.get("isSteward"),
        Some(&serde_json::Value::Bool(false)),
        "isSteward must be PRESENT and false for a hosted visitor (always-serialize)"
    );
    validate_against_schema("views/auth-response.schema.json", &json);
}

#[test]
fn auth_response_rejects_additional_properties() {
    let mut instance = serde_json::json!({
        "token": "jwt",
        "humanId": "human-x",
        "agentPubKey": "uhCAkX",
        "identifier": "x@example.com",
        "expiresAt": 1780000000u64
    });
    instance["snake_case_leak"] = serde_json::json!("should-not-be-here");

    let schema = load_schema("views/auth-response.schema.json");
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors: Vec<_> = validator.iter_errors(&instance).collect();
    assert!(
        !errors.is_empty(),
        "Schema must reject additionalProperties on AuthResponse"
    );
}

// ── HumanProfileResponse (nested in AuthResponse) ───────────────────────

#[test]
fn human_profile_response_matches_schema() {
    let json = serde_json::to_value(sample_profile()).unwrap();
    validate_against_schema("views/human-profile-response.schema.json", &json);
}

#[test]
fn doorway_health_p2p_matches_schema() {
    let health = P2PHealth {
        enabled: true,
        peer_count: 2,
        peer_id: Some("12D3KooWexample".to_string()),
        caught_up: Some(true),
        converged: Some(false),
        divergent_anchor: Some(1_860),
        sync_paused: Some(true),
        sync_reasons: Some(vec!["operator-paused".to_string()]),
        observed_age_ms: Some(4_000),
        stale: false,
    };

    let json = serde_json::to_value(&health).unwrap();
    validate_against_schema("views/doorway-health-p2p.schema.json", &json);
}

#[test]
fn human_profile_response_minimal_matches_schema() {
    let profile = HumanProfileResponse {
        id: "human-minimal".to_string(),
        display_name: "Minimal".to_string(),
        bio: None,
        affinities: vec![],
        profile_reach: "private".to_string(),
        location: None,
        created_at: "2026-06-10T00:00:00Z".to_string(),
        updated_at: "2026-06-10T00:00:00Z".to_string(),
    };

    let json = serde_json::to_value(&profile).unwrap();
    let obj = json.as_object().unwrap();
    assert!(!obj.contains_key("bio"), "bio must be absent when None");
    assert!(
        !obj.contains_key("location"),
        "location must be absent when None"
    );
    validate_against_schema("views/human-profile-response.schema.json", &json);
}

// ── MeResponse (GET /auth/me) ───────────────────────────────────────────

#[test]
fn me_response_full_matches_schema() {
    let response = MeResponse {
        human_id: "human-matthew".to_string(),
        agent_pub_key: "uhCAkABCDEFGHIJKLMNOPQRSTUVWXYZ01234567890123456789012".to_string(),
        identifier: "matthew@example.com".to_string(),
        permission_level: "ADMIN".to_string(),
        doorway_id: Some("alpha".to_string()),
        doorway_url: Some("https://doorway-alpha.elohim.host".to_string()),
        authenticated: true,
        trust_mode: "doorway-host".to_string(),
        authority: AuthorityRef {
            label: "doorway-alpha.elohim.host".to_string(),
            id: Some("alpha".to_string()),
        },
        conductor_endpoint: Some("ws://conductor-0.headless:8445".to_string()),
    };

    let json = serde_json::to_value(&response).unwrap();
    validate_against_schema("views/me-response.schema.json", &json);
}

#[test]
fn me_response_minimal_matches_schema() {
    // MVP shape: no federation config, doorway-host mode, no conductor endpoint.
    let response = MeResponse {
        human_id: "human-visitor".to_string(),
        agent_pub_key: "uhCAkVISITOR0123456789012345678901234567890123456789012".to_string(),
        identifier: "visitor@example.com".to_string(),
        permission_level: "AUTHENTICATED".to_string(),
        doorway_id: None,
        doorway_url: None,
        authenticated: true,
        trust_mode: "doorway-host".to_string(),
        authority: AuthorityRef {
            label: "localhost:8888".to_string(),
            id: None,
        },
        conductor_endpoint: None,
    };

    let json = serde_json::to_value(&response).unwrap();
    let obj = json.as_object().unwrap();
    for absent in ["doorwayId", "doorwayUrl", "conductorEndpoint"] {
        assert!(
            !obj.contains_key(absent),
            "{} must be absent when None (serde skip_serializing_if)",
            absent
        );
    }
    assert!(
        !json["authority"].as_object().unwrap().contains_key("id"),
        "authority.id must be absent when None (serde skip_serializing_if)"
    );
    validate_against_schema("views/me-response.schema.json", &json);
}

// ── AuthorityRef (nested in MeResponse) ─────────────────────────────────

#[test]
fn authority_ref_matches_schema() {
    let authority = AuthorityRef {
        label: "alpha.elohim.host".to_string(),
        id: Some("alpha".to_string()),
    };
    let json = serde_json::to_value(&authority).unwrap();
    validate_against_schema("views/authority-ref.schema.json", &json);
}

// ── SessionTokenResponse (GET /auth/session-token) ──────────────────────

#[test]
fn session_token_response_matches_schema() {
    let response = SessionTokenResponse {
        session_token: "f3a9c2e1d4b5a6978085a4b3c2d1e0f9".to_string(),
        expires_at: 1_780_000_060,
    };

    let json = serde_json::to_value(&response).unwrap();
    validate_against_schema("views/session-token-response.schema.json", &json);
}

// ── ExchangeSessionResponse (GET /auth/exchange-session) ────────────────

#[test]
fn exchange_session_response_full_matches_schema() {
    let response = ExchangeSessionResponse {
        token: "eyJhbGciOiJIUzI1NiJ9.payload.sig".to_string(),
        human_id: "human-matthew".to_string(),
        agent_pub_key: "uhCAkABCDEFGHIJKLMNOPQRSTUVWXYZ01234567890123456789012".to_string(),
        identifier: "matthew@example.com".to_string(),
        expires_at: 1_780_000_000,
        doorway_id: Some("alpha".to_string()),
        doorway_url: Some("https://doorway-alpha.elohim.host".to_string()),
        portal_host_url: Some("https://portal.matthew.example".to_string()),
    };

    let json = serde_json::to_value(&response).unwrap();
    validate_against_schema("views/exchange-session-response.schema.json", &json);
}

#[test]
fn exchange_session_response_minimal_matches_schema() {
    let response = ExchangeSessionResponse {
        token: "eyJhbGciOiJIUzI1NiJ9.payload.sig".to_string(),
        human_id: "human-visitor".to_string(),
        agent_pub_key: "uhCAkVISITOR0123456789012345678901234567890123456789012".to_string(),
        identifier: "visitor@example.com".to_string(),
        expires_at: 1_780_000_000,
        doorway_id: None,
        doorway_url: None,
        portal_host_url: None,
    };

    let json = serde_json::to_value(&response).unwrap();
    let obj = json.as_object().unwrap();
    for absent in ["doorwayId", "doorwayUrl", "portalHostUrl"] {
        assert!(
            !obj.contains_key(absent),
            "{} must be absent when None (serde skip_serializing_if)",
            absent
        );
    }
    validate_against_schema("views/exchange-session-response.schema.json", &json);
}

// ── AccountResponse (GET /auth/account) ─────────────────────────────────

#[test]
fn account_response_full_matches_schema() {
    let response = AccountResponse {
        human_id: "human-matthew".to_string(),
        identifier: "matthew@example.com".to_string(),
        permission_level: "AUTHENTICATED".to_string(),
        storage_bytes: 52_428_800,
        storage_limit: 1_073_741_824,
        storage_percent: 4.88,
        projection_queries: 142,
        daily_query_limit: 10_000,
        queries_percent: 1.42,
        bandwidth_bytes: 10_485_760,
        daily_bandwidth_limit: 5_368_709_120,
        bandwidth_percent: 0.2,
        conductor_id: Some("conductor-0".to_string()),
        is_steward: true,
        stewardship_at: Some("2026-05-01T12:00:00Z".to_string()),
        key_exported: true,
        created_at: Some("2026-01-15T08:30:00Z".to_string()),
        last_login_at: Some("2026-06-10T07:45:00Z".to_string()),
    };

    let json = serde_json::to_value(&response).unwrap();
    validate_against_schema("views/account-response.schema.json", &json);
}

#[test]
fn account_response_minimal_matches_schema() {
    // Fresh hosted account: no conductor assignment, no stewardship, no
    // recorded timestamps. is_steward has NO skip attribute — it must be
    // PRESENT even when false.
    let response = AccountResponse {
        human_id: "human-fresh".to_string(),
        identifier: "fresh@example.com".to_string(),
        permission_level: "AUTHENTICATED".to_string(),
        storage_bytes: 0,
        storage_limit: 1_073_741_824,
        storage_percent: 0.0,
        projection_queries: 0,
        daily_query_limit: 10_000,
        queries_percent: 0.0,
        bandwidth_bytes: 0,
        daily_bandwidth_limit: 5_368_709_120,
        bandwidth_percent: 0.0,
        conductor_id: None,
        is_steward: false,
        stewardship_at: None,
        key_exported: false,
        created_at: None,
        last_login_at: None,
    };

    let json = serde_json::to_value(&response).unwrap();
    let obj = json.as_object().unwrap();
    for absent in ["conductorId", "stewardshipAt", "createdAt", "lastLoginAt"] {
        assert!(
            !obj.contains_key(absent),
            "{} must be absent when None (serde skip_serializing_if)",
            absent
        );
    }
    assert!(
        obj.contains_key("isSteward"),
        "isSteward must be present even when false (no skip attribute on AccountResponse)"
    );
    validate_against_schema("views/account-response.schema.json", &json);
}

// ── Convention enforcement ──────────────────────────────────────────────

#[test]
fn auth_view_schemas_declare_source_of_truth() {
    let view_schemas = [
        "views/auth-response.schema.json",
        "views/me-response.schema.json",
        "views/exchange-session-response.schema.json",
        "views/session-token-response.schema.json",
        "views/account-response.schema.json",
        "views/authority-ref.schema.json",
        "views/human-profile-response.schema.json",
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

// ---------------------------------------------------------------------------
// stability-status-view (self-healing read model — GET /admin/self-healing)
// ---------------------------------------------------------------------------

#[test]
fn stability_status_view_pending_matches_schema() {
    // PENDING wire-up state: autoPreset/admission null, upstreams empty —
    // locks the forward-compat seam (sibling wire-ups not yet landed).
    let view = SelfHealingView {
        auto_preset: None,
        admission: None,
        upstreams: Vec::new(),
        // The open condition is NEVER pending: a doorway always knows what it
        // would take to open a circuit, even before any upstream is observed.
        upstream_policy: UpstreamPolicyView {
            fail_threshold: 3,
            fail_window_seconds: 20,
            cooldown_seconds: 30,
        },
        projector: ProjectorView {
            lag_seconds: None,
            caught_up: None,
            divergent_anchor: None,
        },
        peers: Vec::new(),
        render: RenderView {
            total: 0,
            degenerate_rate: 0.0,
        },
        warmup: WarmupView {
            in_progress: false,
            attempts: 0,
            completed: false,
            last_error: None,
        },
        conductor: ConductorView {
            connected: false,
            connected_workers: 0,
            total_workers: 0,
        },
    };
    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/stability-status-view.schema.json", &json);

    let schema = load_schema("views/stability-status-view.schema.json");
    assert_source_of_truth_declared(&schema, "stability-status-view");
}

#[test]
fn stability_status_view_populated_matches_schema() {
    // Forward-compat: the PENDING fields populated once their siblings wire up.
    let view = SelfHealingView {
        auto_preset: Some(serde_json::json!({ "maxInflight": 64 })),
        admission: Some(AdmissionView {
            max_inflight: 64,
            available: 60,
            shed_total: 3,
        }),
        upstreams: vec![UpstreamView {
            endpoint: "https://upstream.example".to_string(),
            circuit: "open".to_string(),
            error_streak: 5,
            recent_failures: 3,
            last_good: Some("2026-06-13T00:00:00Z".to_string()),
            skipped: true,
        }],
        upstream_policy: UpstreamPolicyView {
            fail_threshold: 3,
            fail_window_seconds: 20,
            cooldown_seconds: 30,
        },
        projector: ProjectorView {
            lag_seconds: Some(7),
            caught_up: Some(false),
            divergent_anchor: Some(2),
        },
        peers: vec![PeerView {
            peer: "uhCAk...".to_string(),
            status: "Degraded".to_string(),
            last_seen: Some("2026-06-13T00:00:00Z".to_string()),
        }],
        render: RenderView {
            total: 42,
            degenerate_rate: 0.1,
        },
        warmup: WarmupView {
            in_progress: true,
            attempts: 2,
            completed: false,
            last_error: Some("timeout".to_string()),
        },
        conductor: ConductorView {
            connected: true,
            connected_workers: 3,
            total_workers: 4,
        },
    };
    let json = serde_json::to_value(&view).unwrap();
    validate_against_schema("views/stability-status-view.schema.json", &json);
}

// ── AuthDiscovery (GET /.well-known/elohim-auth) ────────────────────────

/// The shape an app reads instead of carrying auth configuration.
#[test]
fn auth_discovery_matches_schema() {
    let doc = AuthDiscovery {
        version: 1,
        doorway_id: Some("alpha-elohim-host".to_string()),
        portal: "/threshold/login",
        endpoints: AuthEndpoints::current(),
    };

    let json = serde_json::to_value(&doc).unwrap();
    validate_against_schema("views/auth-discovery.schema.json", &json);
}

/// A doorway with no configured id omits the field rather than sending null,
/// so a client can branch on presence.
#[test]
fn auth_discovery_without_doorway_id_matches_schema() {
    let doc = AuthDiscovery {
        version: 1,
        doorway_id: None,
        portal: "/threshold/login",
        endpoints: AuthEndpoints::current(),
    };

    let json = serde_json::to_value(&doc).unwrap();
    assert!(json.get("doorwayId").is_none(), "absent id must be omitted, not null");
    validate_against_schema("views/auth-discovery.schema.json", &json);
}

/// THE security property, pinned at the SCHEMA boundary rather than only in the
/// crate: the `relativePath` pattern must refuse a document that escapes its own
/// origin. A discovery document that could name another origin would let whoever
/// answers it aim a Login button at an attacker's portal.
#[test]
fn auth_discovery_schema_refuses_a_foreign_origin() {
    for hostile in ["https://evil.tld/login", "//evil.tld/login"] {
        let doc = AuthDiscovery {
            version: 1,
            doorway_id: Some("alpha-elohim-host".to_string()),
            portal: hostile,
            endpoints: AuthEndpoints::current(),
        };
        let json = serde_json::to_value(&doc).unwrap();
        let schema = load_schema("views/auth-discovery.schema.json");
        let compiled = jsonschema::validator_for(&schema).expect("schema compiles");
        assert!(
            !compiled.is_valid(&json),
            "the schema accepted a discovery document naming {hostile:?} — the relativePath \
             pattern is the wire-level guard against an open-redirect primitive"
        );
    }
}
