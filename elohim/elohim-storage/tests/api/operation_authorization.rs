//! Integration tests for POST /api/v1/authorize-operation (Che op-gate Slice 1).
//!
//! Tests the core op-gate logic:
//!   - Finds the active `delegates-compute` grant for (performer, capability)
//!   - Reuses `bounds_validator` via the `ProjectionCommitmentFetcher`
//!   - Enforces `performer == recipient` structurally (lookup) + explicitly (guard)
//!   - Returns `allowed:false` on revocation
//!
//! ## Testing strategy
//!
//! Uses the REAL `ProjectionCommitmentFetcher` (not a mock) against a seeded
//! in-memory SQLite pool.  This is the honest integration test that proves the
//! full fetch→validate chain works end-to-end.
//!
//! Seeding uses `perform_seed` from Task 2 (which synthesises a non-NULL
//! `dht_anchor_hash` so `ProjectionCommitmentFetcher` accepts the row).

use elohim_storage::api::seed_delegates_compute::{perform_seed, SeedDelegatesInput};
use elohim_storage::db::mishpat_commitments;
use elohim_storage::http::build_manifest;
use elohim_storage::services::operation_authorization::{
    authorize_operation, AuthorizeOperationRequest,
};
use elohim_storage::test_util::test_pool;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MATTHEW: &str = "uhCAk-matthew";
const SCOPE: &str = "orchestrate-node";
const PROVIDER: &str = "uhCAk-matthew"; // self-contract: provider == recipient
/// bounds: wildcard epr_scope, commons ceiling, 60/hr limit, 30-day TTL.
/// Rotation check: signed_at(2026-06-26) - valid_from(2026-06-01) = 25 days < 30. Passes.
const BOUNDS_JSON: &str = r#"{"epr_scope":["*"],"reach_ceiling":"commons","rate_per_hour":60,"rotation_ttl_days":30,"_provenance":"dev-seed"}"#;
const VALID_FROM: &str = "2026-06-01T00:00:00Z";
const VALID_UNTIL: &str = "2026-09-01T00:00:00Z";
const NOW: &str = "2026-06-26T12:00:00Z";

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn req(performer: &str, capability: &str) -> AuthorizeOperationRequest {
    AuthorizeOperationRequest {
        performer: performer.to_string(),
        capability: capability.to_string(),
        target_epr_id: None, // service maps None → "*"; wildcard epr_scope passes
        reach: "commons".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Test: all five cases from the brief spec §14
// ---------------------------------------------------------------------------

/// End-to-end integration: seed a real row, exercise all five cases.
#[tokio::test]
async fn authorize_operation_gate_all_cases() {
    let pool = test_pool();
    let mut conn = pool.get().expect("pool conn");

    // Seed the primary grant: matthew grants himself orchestrate-node.
    let cid = "commitment:op-gate-test-001";
    let row = perform_seed(
        &mut conn,
        &SeedDelegatesInput {
            cid,
            scope: SCOPE,
            provider: PROVIDER,
            recipient: MATTHEW,
            bounds_json: BOUNDS_JSON,
            valid_from: VALID_FROM,
            valid_until: VALID_UNTIL,
        },
    )
    .expect("perform_seed must succeed");

    // Guard: row is notarized (ProjectionCommitmentFetcher fails-closed on NULL anchor).
    assert!(
        row.dht_anchor_hash.is_some(),
        "seeded row must have a non-NULL dht_anchor_hash"
    );
    let seeded_cid = row.cid.clone();

    // (a) Active bounded grant: recipient=provider=matthew, scope=orchestrate-node → allowed.
    let result_a = authorize_operation(&pool, req(MATTHEW, SCOPE), NOW.to_string()).await;
    assert!(
        result_a.allowed,
        "case (a): active grant must be allowed; reason: {}",
        result_a.reason
    );
    assert_eq!(
        result_a.commitment_cid.as_deref(),
        Some(seeded_cid.as_str()),
        "case (a): commitment_cid must equal the seeded cid"
    );

    // (b) Wrong capability: no row with scope="node:wipe" for matthew → denied.
    let result_b = authorize_operation(&pool, req(MATTHEW, "node:wipe"), NOW.to_string()).await;
    assert!(
        !result_b.allowed,
        "case (b): wrong capability (scope-in-SQL filter) must deny; reason: {}",
        result_b.reason
    );

    // (c) Unknown performer: "uhCAk-stranger" has no grant → fail-closed deny.
    let result_c = authorize_operation(&pool, req("uhCAk-stranger", SCOPE), NOW.to_string()).await;
    assert!(
        !result_c.allowed,
        "case (c): performer with no grant must be denied; reason: {}",
        result_c.reason
    );

    // (d) Cross-recipient scope isolation: seed a SECOND grant for alice with
    //     a DISTINCT scope "alice-only-op".  Ask as matthew with "alice-only-op"
    //     → lookup filters recipient=matthew, so alice's grant is invisible → deny.
    let alice_cid = "commitment:op-gate-alice-001";
    perform_seed(
        &mut conn,
        &SeedDelegatesInput {
            cid: alice_cid,
            scope: "alice-only-op",
            provider: "uhCAk-alice",
            recipient: "uhCAk-alice",
            bounds_json: BOUNDS_JSON,
            valid_from: VALID_FROM,
            valid_until: VALID_UNTIL,
        },
    )
    .expect("alice seed must succeed");

    let result_d = authorize_operation(&pool, req(MATTHEW, "alice-only-op"), NOW.to_string()).await;
    assert!(
        !result_d.allowed,
        "case (d): cross-recipient scope must deny (no row for matthew with alice-only-op); reason: {}",
        result_d.reason
    );

    // (e) Revoke then re-call: set_revoked_at on matthew's grant, then re-run (a)'s request.
    mishpat_commitments::set_revoked_at(&mut conn, &seeded_cid, NOW)
        .expect("set_revoked_at must succeed");

    let result_e = authorize_operation(&pool, req(MATTHEW, SCOPE), NOW.to_string()).await;
    assert!(
        !result_e.allowed,
        "case (e): revoked grant must deny on next call; reason: {}",
        result_e.reason
    );
}

// ---------------------------------------------------------------------------
// Test: build_manifest() must NOT include the authorize-operation route
// ---------------------------------------------------------------------------

/// Route guard: `build_manifest()` must not register `/api/v1/authorize-operation`.
///
/// Any route in `build_manifest()` is auto-promoted to a public doorway proxy.
/// The authorize-operation endpoint must NEVER be doorway-proxied — it is a
/// node-internal verdict oracle; doorway exposes it to callers via its OWN
/// logic (calling storage directly, not as a proxied route).
#[test]
fn authorize_operation_absent_from_build_manifest() {
    let manifest = build_manifest();
    let routes_json =
        serde_json::to_string(&manifest).expect("build_manifest must be JSON-serialisable");
    assert!(
        !routes_json.contains("authorize-operation"),
        "POST /api/v1/authorize-operation must NOT appear in build_manifest() — \
         it must never be doorway-proxied (verdict-oracle DoS vector); got manifest"
    );
}
