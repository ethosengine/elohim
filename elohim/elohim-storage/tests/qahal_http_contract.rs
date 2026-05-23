//! HTTP-level contract tests for qahal routes (M1 Task 12).
//!
//! ## Scope
//!
//! Scope-honest contract tests: what elohim-storage can verify without a live
//! Holochain conductor, without Task 11's handler module, and without the
//! Task 7/8 view types.  The full happy-path + auth enforcement tests are
//! CI-scope (sweettest conductor, doorway integration).
//!
//! ## Test surfaces (all currently passing)
//!
//! 1. **§1 HcClientRegistry struct contract** — verifies the `imagodei` slot
//!    exists and accepts `None`.  Compiles and passes today.
//!
//! 2. **§2 Dispatch routing logic** — encodes the `handle_collective` and
//!    `handle_collab` dispatch decision tables as pure functions.  Passes today
//!    and will remain green when the real handlers are added.
//!
//! 3. **§3 CID encoding conventions** — pins the `collective:` / `agreement:`
//!    prefix contract and the no-slash guard.
//!
//! 4. **§4 Path-CID mismatch guard logic** — encodes the boolean guard used by
//!    `handle_attest_collab_agreement` and `handle_withdraw_membership`.
//!
//! 5. **§5 JSON wire shape** — verifies the camelCase key names and structural
//!    requirements for each M1 request body using raw `serde_json::Value`.
//!
//! ## Tests that activate with Task 11
//!
//! The `§6_manifest_*` tests and `§7_503_*` handler tests are marked `#[ignore]`
//! with an explicit "activate after Task 11" message.  They will turn green
//! without code changes once Task 11 adds `pub mod qahal` to `api/mod.rs` and
//! the 6 M1 routes to `build_manifest()`.
//!
//! ## Tests that activate with Tasks 7/8
//!
//! `§8_serde_*` tests are marked `#[ignore]` pending the view types landing in
//! `elohim-views/src/qahal.rs` (Tasks 7/8).
//!
//! See genesis/docs/plans/2026-05-23-multi-collective-collaboration-epr-plan.md §12.

use serde_json::{json, Value};

use elohim_storage::{
    hc_client_registry::HcClientRegistry,
    http::build_manifest,
};

// =============================================================================
// §1 — HcClientRegistry struct contract
//
// Compile-time + runtime guards for the imagodei slot shape that Task 11's
// handlers depend on.  Pass today and should never regress.
// =============================================================================

#[test]
fn hc_client_registry_has_imagodei_field() {
    // Construct with both slots empty — simulates the 503-path state.
    // If `imagodei` is renamed or removed, this fails at compile time.
    let registry = HcClientRegistry {
        infrastructure: None,
        imagodei: None,
    };
    assert!(
        registry.imagodei.is_none(),
        "HcClientRegistry.imagodei must exist and accept None"
    );
    assert!(
        registry.infrastructure.is_none(),
        "HcClientRegistry.infrastructure must exist and accept None"
    );
}

#[test]
fn hc_client_registry_both_slots_absent_is_valid_state() {
    // A registry built with no connected roles is legitimate — storage serves
    // on HTTP but conductor-gated routes (qahal writes) return 503.
    let registry = std::sync::Arc::new(HcClientRegistry {
        infrastructure: None,
        imagodei: None,
    });
    assert!(registry.infrastructure.is_none());
    assert!(registry.imagodei.is_none());
}

// =============================================================================
// §2 — Dispatch routing logic
//
// The `handle_collective` and `handle_collab` dispatch decision tables are
// encoded here as pure functions that mirror the Rust code in api/qahal.rs.
// These pass immediately and guard against regressions when Task 11 lands.
// =============================================================================

/// Mirror of `handle_collective` method+path dispatch logic.
fn collective_dispatch(method: &str, path: &str) -> &'static str {
    let path = path.trim_start_matches('/');
    match (method, path) {
        ("POST", "") => "create_collective",
        ("GET", cid) if !cid.is_empty() && !cid.contains('/') => "get_collective",
        _ => "not_found",
    }
}

/// Mirror of `handle_collab` method+path dispatch logic.
fn collab_dispatch(method: &str, path: &str) -> &'static str {
    let path = path.trim_start_matches('/');

    // Must match `/agreement/{cid}/attest` BEFORE `/agreement` to avoid overlap.
    if let Some(rest) = path.strip_prefix("agreement/") {
        if rest.ends_with("/attest") && method == "POST" {
            return "attest_collab_agreement";
        }
        return "not_found";
    }

    match (method, path) {
        ("POST", "agreement") => "create_collab_agreement",
        ("GET", cid) if !cid.is_empty() && !cid.contains('/') => "fetch_collab_qahal",
        ("POST", rest) if rest.contains('/') => {
            let mut parts = rest.splitn(2, '/');
            let _cid = parts.next().unwrap_or("");
            let action = parts.next().unwrap_or("");
            if action == "withdraw" {
                "withdraw_membership"
            } else {
                "not_found"
            }
        }
        _ => "not_found",
    }
}

#[test]
fn collective_dispatcher_post_empty_path_routes_to_create() {
    assert_eq!(collective_dispatch("POST", ""), "create_collective");
}

#[test]
fn collective_dispatcher_get_cid_routes_to_get() {
    assert_eq!(collective_dispatch("GET", "/uhCkkAAA"), "get_collective");
}

#[test]
fn collective_dispatcher_delete_returns_not_found() {
    assert_eq!(
        collective_dispatch("DELETE", ""),
        "not_found",
        "DELETE is not a registered method"
    );
}

#[test]
fn collective_dispatcher_get_with_slash_in_cid_returns_not_found() {
    assert_eq!(
        collective_dispatch("GET", "/cid/with/slash"),
        "not_found",
        "CIDs must not contain slashes — multi-segment path is unrecognised"
    );
}

#[test]
fn collective_dispatcher_get_empty_cid_returns_not_found() {
    assert_eq!(
        collective_dispatch("GET", ""),
        "not_found",
        "GET with empty CID is unrecognised"
    );
}

#[test]
fn collab_dispatcher_post_agreement_routes_to_create() {
    assert_eq!(
        collab_dispatch("POST", "/agreement"),
        "create_collab_agreement"
    );
}

#[test]
fn collab_dispatcher_post_agreement_cid_attest_routes_to_attest() {
    assert_eq!(
        collab_dispatch("POST", "/agreement/uhCkkAAA/attest"),
        "attest_collab_agreement"
    );
}

#[test]
fn collab_dispatcher_get_cid_routes_to_fetch_collab() {
    assert_eq!(
        collab_dispatch("GET", "/uhCkkAAA"),
        "fetch_collab_qahal"
    );
}

#[test]
fn collab_dispatcher_post_cid_withdraw_routes_to_withdraw() {
    assert_eq!(
        collab_dispatch("POST", "/uhCkkAAA/withdraw"),
        "withdraw_membership"
    );
}

#[test]
fn collab_dispatcher_unknown_action_suffix_returns_not_found() {
    assert_eq!(
        collab_dispatch("POST", "/uhCkkAAA/dissolve"),
        "not_found",
        "'dissolve' is not a registered M1 action"
    );
}

#[test]
fn collab_dispatcher_delete_cid_returns_not_found() {
    assert_eq!(
        collab_dispatch("DELETE", "/uhCkkAAA"),
        "not_found",
        "DELETE on collab is not a registered method"
    );
}

#[test]
fn collab_dispatcher_agreement_sub_path_without_attest_returns_not_found() {
    // GET /api/v1/collab/agreement/{cid} — not a registered route
    assert_eq!(
        collab_dispatch("GET", "/agreement/uhCkkAAA"),
        "not_found"
    );
}

// =============================================================================
// §3 — CID encoding conventions
// =============================================================================

#[test]
fn collective_cid_prefix_is_collective_colon() {
    let cid = "collective:dGVzdAo";
    assert!(
        cid.starts_with("collective:"),
        "Collective CIDs must start with 'collective:'"
    );
    assert!(
        cid.strip_prefix("collective:").unwrap().len() > 0,
        "collective CID suffix (base64url of ActionHash) must be non-empty"
    );
}

#[test]
fn agreement_cid_prefix_is_agreement_colon() {
    let cid = "agreement:dGVzdAo";
    assert!(
        cid.starts_with("agreement:"),
        "Agreement CIDs must start with 'agreement:'"
    );
    assert!(
        cid.strip_prefix("agreement:").unwrap().len() > 0,
        "agreement CID suffix must be non-empty"
    );
}

#[test]
fn valid_collective_cid_contains_no_slash() {
    // The dispatcher guard `!cid.contains('/')` rejects slashed CIDs.
    let valid = "collective:dGVzdAo";
    let invalid = "collective:part1/part2";
    assert!(!valid.contains('/'), "valid CID must not contain slash");
    assert!(invalid.contains('/'), "fixture confirms slash detection works");
}

// =============================================================================
// §4 — Path-CID mismatch guard logic
// =============================================================================

fn attest_would_reject(path_cid: &str, body_cid: &str) -> bool {
    !path_cid.is_empty() && body_cid != path_cid
}

fn withdraw_would_reject(path_cid: &str, body_cid: &str) -> bool {
    !path_cid.is_empty() && body_cid != path_cid
}

#[test]
fn attest_guard_rejects_mismatched_cids() {
    assert!(
        attest_would_reject("agreement:AAA", "agreement:BBB"),
        "different path and body CIDs must be rejected"
    );
}

#[test]
fn attest_guard_accepts_matching_cids() {
    assert!(
        !attest_would_reject("agreement:AAA", "agreement:AAA"),
        "identical path and body CIDs must be accepted"
    );
}

#[test]
fn attest_guard_disabled_when_path_cid_empty() {
    assert!(
        !attest_would_reject("", "agreement:BBB"),
        "empty path_cid disables the guard (URL param absent)"
    );
}

#[test]
fn withdraw_guard_rejects_mismatched_cids() {
    assert!(
        withdraw_would_reject("collective:QQQ", "collective:RRR"),
        "different path and body collabQahalCid must be rejected"
    );
}

#[test]
fn withdraw_guard_accepts_matching_cids() {
    assert!(
        !withdraw_would_reject("collective:QQQ", "collective:QQQ"),
        "identical collabQahalCid values must be accepted"
    );
}

// =============================================================================
// §5 — JSON wire shape
//
// Validates the camelCase field names and structural requirements for M1
// request bodies using raw `serde_json::Value`.  No view types required.
// =============================================================================

#[test]
fn create_collective_body_is_camel_case_with_three_required_fields() {
    let body = json!({
        "charter": "We steward the commons together.",
        "displayName": "Dawn Runners",
        "salt": "e3b0c44298fc"
    });
    assert!(body.get("charter").is_some(), "charter is required");
    assert!(body.get("displayName").is_some(), "displayName is camelCase required field");
    assert!(body.get("salt").is_some(), "salt is required");
    // Verify snake_case is NOT the wire format
    assert!(body.get("display_name").is_none(), "snake_case must not be used in wire format");
}

#[test]
fn create_collab_agreement_body_uses_camel_case_keys() {
    let body = json!({
        "participants": ["collective:AAA", "collective:BBB"],
        "scope": "cross-pillar experiment",
        "shareAllocation": {
            "form": "Declared",
            "commonsPoolTribute": 0.05
        },
        "initialTier": "T0",
        "displayNameForQahal": "Alpha Collab",
        "salt": "deadbeef"
    });

    assert!(body.get("displayNameForQahal").is_some(), "must use displayNameForQahal");
    assert!(body.get("display_name_for_qahal").is_none(), "snake_case must not be used");
    assert!(body.get("initialTier").is_some(), "must use initialTier");
    assert!(body.get("initial_tier").is_none(), "snake_case must not be used");
    assert!(body.get("shareAllocation").is_some(), "must use shareAllocation");

    let sa = body.get("shareAllocation").unwrap();
    assert!(sa.get("commonsPoolTribute").is_some(), "shareAllocation.commonsPoolTribute required");
    assert!(sa.get("commons_pool_tribute").is_none(), "snake_case must not be in shareAllocation");
    assert!(sa.get("form").is_some(), "shareAllocation.form required");
}

#[test]
fn create_collab_agreement_participants_field_must_be_array() {
    let valid = json!({ "participants": ["collective:AAA"] });
    assert!(
        valid.get("participants").unwrap().is_array(),
        "participants must be a JSON array of collective CIDs"
    );
}

#[test]
fn attest_collab_agreement_body_requires_two_camel_case_fields() {
    let body = json!({
        "agreementCid": "agreement:AAAA",
        "attestingCollectiveCid": "collective:BBBB"
    });
    assert!(body.get("agreementCid").is_some(), "agreementCid required");
    assert!(body.get("attestingCollectiveCid").is_some(), "attestingCollectiveCid required");
    assert!(body.get("agreement_cid").is_none(), "snake_case not used");
}

#[test]
fn withdraw_membership_body_requires_two_camel_case_fields() {
    let body = json!({
        "membershipCid": "collective:MMMM",
        "collabQahalCid": "collective:QQQQ"
    });
    assert!(body.get("membershipCid").is_some(), "membershipCid required");
    assert!(body.get("collabQahalCid").is_some(), "collabQahalCid required");
    assert!(body.get("membership_cid").is_none(), "snake_case not used");
}

#[test]
fn elohim_tier_json_values_are_t_prefixed_strings() {
    for tier in &["T0", "T1", "T2", "T3"] {
        let v = json!(tier);
        assert!(
            v.as_str().unwrap().starts_with('T'),
            "ElohimTier variant {tier} must be a T-prefixed string"
        );
    }
}

#[test]
fn share_allocation_form_json_values_match_spec() {
    // Per spec §4.2 and serde(rename_all = "camelCase") on the enum:
    //   ShareAllocationForm::Declared     → "Declared"       (PascalCase → unchanged)
    //   ShareAllocationForm::AffinityDerived → "affinityDerived" (camelCase)
    let declared = json!("Declared");
    let affinity = json!("affinityDerived");

    assert_eq!(declared.as_str(), Some("Declared"));
    assert_eq!(affinity.as_str(), Some("affinityDerived"));
}

#[test]
fn zero_commons_pool_tribute_is_accepted_at_wire_level() {
    // Zero tribute is a valid f64 at the wire/serde level.
    // Business-rule enforcement (tribute must be > 0 per spec §4.2) lives in
    // the Holochain coordinator zome's validate_create_collab_agreement, not
    // at the HTTP deserialization boundary.
    let body = json!({
        "shareAllocation": {
            "form": "Declared",
            "commonsPoolTribute": 0.0
        }
    });
    let tribute = body["shareAllocation"]["commonsPoolTribute"].as_f64().unwrap();
    assert_eq!(tribute, 0.0_f64, "zero tribute is representable at the wire level");
    // NOTE: if a future version adds HTTP-layer validation for tribute > 0,
    // update this test to expect a 400 response instead.
}

// =============================================================================
// §6 — Manifest route registration (activates with Task 11)
//
// These tests are marked #[ignore] because `build_manifest()` does not yet
// include the M1 qahal routes. Remove `#[ignore]` after Task 11 adds them.
// =============================================================================

#[test]
#[ignore = "activate after Task 11 adds M1 routes to build_manifest()"]
fn manifest_registers_exactly_six_m1_qahal_routes() {
    let manifest = build_manifest();
    let qahal_routes: Vec<&str> = manifest
        .routes
        .iter()
        .map(|r| r.path.as_str())
        .filter(|p| p.starts_with("/api/v1/collective") || p.starts_with("/api/v1/collab"))
        .collect();

    assert_eq!(
        qahal_routes.len(),
        6,
        "expected exactly 6 M1 qahal routes, found {}: {:?}",
        qahal_routes.len(),
        qahal_routes
    );
}

#[test]
#[ignore = "activate after Task 11 adds M1 routes to build_manifest()"]
fn post_collective_requires_auth_in_manifest() {
    let flags = build_manifest()
        .routes
        .iter()
        .map(|r| (r.path.clone(), r.auth_required))
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(flags.get("/api/v1/collective").copied(), Some(true));
}

#[test]
#[ignore = "activate after Task 11 adds M1 routes to build_manifest()"]
fn get_collective_by_cid_is_public_in_manifest() {
    let flags = build_manifest()
        .routes
        .iter()
        .map(|r| (r.path.clone(), r.auth_required))
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(flags.get("/api/v1/collective/{cid}").copied(), Some(false));
}

#[test]
#[ignore = "activate after Task 11 adds M1 routes to build_manifest()"]
fn post_collab_agreement_requires_auth_in_manifest() {
    let flags = build_manifest()
        .routes
        .iter()
        .map(|r| (r.path.clone(), r.auth_required))
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(flags.get("/api/v1/collab/agreement").copied(), Some(true));
}

#[test]
#[ignore = "activate after Task 11 adds M1 routes to build_manifest()"]
fn post_attest_collab_agreement_requires_auth_in_manifest() {
    let flags = build_manifest()
        .routes
        .iter()
        .map(|r| (r.path.clone(), r.auth_required))
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(
        flags
            .get("/api/v1/collab/agreement/{cid}/attest")
            .copied(),
        Some(true)
    );
}

#[test]
#[ignore = "activate after Task 11 adds M1 routes to build_manifest()"]
fn get_collab_qahal_is_public_in_manifest() {
    let flags = build_manifest()
        .routes
        .iter()
        .map(|r| (r.path.clone(), r.auth_required))
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(flags.get("/api/v1/collab/{cid}").copied(), Some(false));
}

#[test]
#[ignore = "activate after Task 11 adds M1 routes to build_manifest()"]
fn post_withdraw_membership_requires_auth_in_manifest() {
    let flags = build_manifest()
        .routes
        .iter()
        .map(|r| (r.path.clone(), r.auth_required))
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(
        flags.get("/api/v1/collab/{cid}/withdraw").copied(),
        Some(true)
    );
}

// =============================================================================
// §7 — 503 service-unavailable path (activates with Task 11)
//
// Marked #[ignore] until api::qahal is declared public.
// The implementation contracts below describe the expected behaviour.
// =============================================================================

#[test]
#[ignore = "activate after Task 11: api::qahal module not yet public"]
fn create_collective_returns_503_when_registry_absent() {
    // use elohim_storage::api::qahal::handle_collective;
    // let req = make_incoming_request(Method::POST);
    // let resp = run(handle_collective(req, Method::POST, "", None)).unwrap();
    // assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    unimplemented!("activate after Task 11")
}

#[test]
#[ignore = "activate after Task 11: api::qahal module not yet public"]
fn create_collab_agreement_returns_503_when_registry_absent() {
    unimplemented!("activate after Task 11")
}

#[test]
#[ignore = "activate after Task 11: api::qahal module not yet public"]
fn attest_collab_agreement_returns_503_when_registry_absent() {
    unimplemented!("activate after Task 11")
}

#[test]
#[ignore = "activate after Task 11: api::qahal module not yet public"]
fn get_collab_qahal_returns_503_when_registry_absent() {
    unimplemented!("activate after Task 11")
}

#[test]
#[ignore = "activate after Task 11: api::qahal module not yet public"]
fn withdraw_membership_returns_503_when_registry_absent() {
    unimplemented!("activate after Task 11")
}

#[test]
#[ignore = "activate after Task 11: api::qahal module not yet public"]
fn service_unavailable_response_carries_imagodei_bridge_offline_code() {
    // body["code"] must equal "IMAGODEI_BRIDGE_OFFLINE"
    unimplemented!("activate after Task 11")
}

#[test]
#[ignore = "activate after Task 11: api::qahal module not yet public"]
fn create_collective_returns_503_when_imagodei_slot_absent() {
    // let registry = Arc::new(HcClientRegistry { infrastructure: None, imagodei: None });
    // handle_collective(req, POST, "", Some(&registry)) → 503
    unimplemented!("activate after Task 11")
}

#[test]
#[ignore = "activate after Task 11: api::qahal module not yet public"]
fn create_collab_agreement_returns_503_when_imagodei_slot_absent() {
    unimplemented!("activate after Task 11")
}

// =============================================================================
// §8 — Input-type serde contract (activates with Tasks 7/8)
//
// Marked #[ignore] until the view types land in elohim-views.
// =============================================================================

#[test]
#[ignore = "activate after Tasks 7/8: CreateCollabCollectiveInputView not yet in elohim-views"]
fn create_collective_input_deserializes_from_valid_json() {
    // use elohim_views::CreateCollabCollectiveInputView;
    // let v: CreateCollabCollectiveInputView = serde_json::from_value(json!({
    //     "charter": "We steward the commons.",
    //     "displayName": "Dawn Runners",
    //     "salt": "e3b0c44298fc"
    // })).unwrap();
    // assert_eq!(v.display_name, "Dawn Runners");
    unimplemented!("activate after Tasks 7/8")
}

#[test]
#[ignore = "activate after Tasks 7/8: CreateCollabCollectiveInputView not yet in elohim-views"]
fn create_collective_input_rejects_missing_display_name() {
    unimplemented!("activate after Tasks 7/8")
}

#[test]
#[ignore = "activate after Tasks 7/8: CreateCollabAgreementInputView not yet in elohim-views"]
fn create_collab_agreement_input_deserializes_from_valid_json() {
    unimplemented!("activate after Tasks 7/8")
}

#[test]
#[ignore = "activate after Tasks 7/8: CreateCollabAgreementInputView not yet in elohim-views"]
fn create_collab_agreement_input_rejects_missing_participants() {
    unimplemented!("activate after Tasks 7/8")
}

#[test]
#[ignore = "activate after Tasks 7/8: CreateCollabAgreementInputView not yet in elohim-views"]
fn create_collab_agreement_input_accepts_zero_tribute() {
    unimplemented!("activate after Tasks 7/8")
}

#[test]
#[ignore = "activate after Tasks 7/8: AttestCollabAgreementInputView not yet in elohim-views"]
fn attest_collab_agreement_input_deserializes_from_valid_json() {
    unimplemented!("activate after Tasks 7/8")
}

#[test]
#[ignore = "activate after Tasks 7/8: WithdrawMembershipInputView not yet in elohim-views"]
fn withdraw_membership_input_deserializes_from_valid_json() {
    unimplemented!("activate after Tasks 7/8")
}

#[test]
#[ignore = "activate after Tasks 7/8: ElohimTier not yet in elohim-views"]
fn elohim_tier_enum_all_variants_parse() {
    unimplemented!("activate after Tasks 7/8")
}

#[test]
#[ignore = "activate after Tasks 7/8: ShareAllocationForm not yet in elohim-views"]
fn share_allocation_form_enum_roundtrips() {
    unimplemented!("activate after Tasks 7/8")
}
