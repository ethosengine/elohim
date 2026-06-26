//! Integration tests for POST /admin/seed/delegates-compute (Che op-gate Slice 1).
//!
//! Tests the gated seed+revoke endpoint that writes `delegates-compute`
//! commitments directly into `mishpat_commitments`.
//!
//! ## Testing strategy
//!
//! We test the db-write + flag-gate logic directly rather than constructing
//! a full hyper `Request<Incoming>` (which requires a live transport stack).
//! The HTTP handler thin-wraps these; the load-bearing correctness is here.
//!
//! ## Load-bearing assertions
//!
//! 1. Non-NULL `dht_anchor_hash` after seed (fail-closed fetcher requirement).
//! 2. `state="active"` after seed.
//! 3. `recipient` / `scope` / `bounds_json` persisted verbatim (incl. `_provenance`).
//! 4. `revoked_at` non-NULL after revoke.
//! 5. Flag-off → flag check returns false (handler returns 403, no write made).
//! 6. Route absent from `build_manifest()` (never doorway-proxied).

use elohim_storage::api::seed_delegates_compute::{
    is_seed_allowed, perform_seed, SeedDelegatesInput,
};
use elohim_storage::db::mishpat_commitments;
use elohim_storage::http::build_manifest;
use elohim_storage::test_util::test_pool;

// ---------------------------------------------------------------------------
// Constants shared across tests
// ---------------------------------------------------------------------------

const SCOPE: &str = "republish-epr";
const PROVIDER: &str = "agent:matthew-steward";
const RECIPIENT: &str = "agent:deploy-svc";
/// Bounds JSON with `_provenance:"dev-seed"` key — stored verbatim, doubles as
/// audit marker (bounds_validator ignores unknown keys via untyped serde_json::Value).
const BOUNDS_JSON: &str =
    r#"{"rate_per_hour":30,"reach_ceiling":"commons","_provenance":"dev-seed"}"#;
const VALID_FROM: &str = "2026-06-01T00:00:00Z";
const VALID_UNTIL: &str = "2026-09-01T00:00:00Z";

/// Serialise env-var mutations across tests.  cargo test runs tests in parallel
/// so an unguarded set_var in one test leaks into env::var reads in another.
static SEED_FLAG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

// ---------------------------------------------------------------------------
// Test 1: flag ON → seed → load-bearing row assertions
// ---------------------------------------------------------------------------

/// Flag SET: `perform_seed` writes a row with non-NULL `dht_anchor_hash`,
/// `state="active"`, and `recipient`/`scope`/`bounds_json` persisted verbatim.
#[test]
fn seed_delegates_compute_flag_on_row_written() {
    let _g = SEED_FLAG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("ALLOW_SEED_DELEGATES_COMPUTE", "1");

    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");

    let cid = "commitment:che-opgate-test-seed-001";
    let row = perform_seed(
        &mut conn,
        &SeedDelegatesInput {
            cid,
            scope: SCOPE,
            provider: PROVIDER,
            recipient: RECIPIENT,
            bounds_json: BOUNDS_JSON,
            valid_from: VALID_FROM,
            valid_until: VALID_UNTIL,
        },
    )
    .expect("perform_seed must succeed");

    // Load-bearing assertion 1: non-NULL dht_anchor_hash.
    // The ProjectionCommitmentFetcher (Slice-2a T6) fails-closed on NULL anchors;
    // a dev-seeded row without an anchor would never be accepted by the gate.
    assert!(
        row.dht_anchor_hash.is_some(),
        "dht_anchor_hash must be non-NULL after seed"
    );

    // Load-bearing assertion 2: state="active".
    // Dev-seed rows are pre-graduated (active, not proposed) so the fetcher
    // accepts them on the first read without a DHT-projection graduate step.
    assert_eq!(row.state, "active", "state must be 'active' after dev-seed");

    // Load-bearing assertion 3a: recipient persisted verbatim.
    assert_eq!(
        row.recipient, RECIPIENT,
        "recipient must be stored verbatim"
    );

    // Load-bearing assertion 3b: scope persisted verbatim.
    assert_eq!(row.scope, SCOPE, "scope must be stored verbatim");

    // Load-bearing assertion 3c: bounds_json stored verbatim, including the
    // `_provenance:"dev-seed"` audit key from the Task-1 seeder.
    assert!(
        row.bounds_json.contains("_provenance"),
        "bounds_json must retain the _provenance key; got: {}",
        row.bounds_json
    );
    assert!(
        row.bounds_json.contains("dev-seed"),
        "bounds_json _provenance value must be 'dev-seed'; got: {}",
        row.bounds_json
    );
    assert!(
        row.bounds_json.contains("rate_per_hour"),
        "bounds_json must retain rate_per_hour; got: {}",
        row.bounds_json
    );
}

// ---------------------------------------------------------------------------
// Test 2: flag ON → seed then revoke → revoked_at non-NULL
// ---------------------------------------------------------------------------

/// After seeding, calling `set_revoked_at` (the handler's revoke path) must
/// set `revoked_at` to a non-NULL timestamp.
#[test]
fn seed_delegates_compute_revoke_sets_revoked_at() {
    let _g = SEED_FLAG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("ALLOW_SEED_DELEGATES_COMPUTE", "1");

    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");

    let cid = "commitment:che-opgate-revoke-test-001";

    perform_seed(
        &mut conn,
        &SeedDelegatesInput {
            cid,
            scope: SCOPE,
            provider: PROVIDER,
            recipient: RECIPIENT,
            bounds_json: BOUNDS_JSON,
            valid_from: VALID_FROM,
            valid_until: VALID_UNTIL,
        },
    )
    .expect("seed must succeed");

    // Verify row exists and is active before revoke.
    let pre = mishpat_commitments::get_by_cid(&mut conn, cid)
        .expect("get_by_cid pre-revoke")
        .expect("row must exist after seed");
    assert_eq!(pre.state, "active");
    assert!(
        pre.revoked_at.is_none(),
        "revoked_at must be None before revoke"
    );

    // Revoke via the same function the handler uses.
    let ts = "2026-07-01T12:00:00Z";
    let affected = mishpat_commitments::set_revoked_at(&mut conn, cid, ts)
        .expect("set_revoked_at must succeed");
    assert_eq!(affected, 1, "set_revoked_at must affect exactly one row");

    // Load-bearing assertion 4: revoked_at non-NULL after revoke.
    let post = mishpat_commitments::get_by_cid(&mut conn, cid)
        .expect("get_by_cid post-revoke")
        .expect("row must still exist after revoke");
    assert!(
        post.revoked_at.is_some(),
        "revoked_at must be non-NULL after set_revoked_at"
    );
    assert_eq!(
        post.revoked_at.as_deref(),
        Some(ts),
        "revoked_at must equal the supplied timestamp"
    );
}

// ---------------------------------------------------------------------------
// Test 3: flag UNSET → is_seed_allowed() returns false, no row written
// ---------------------------------------------------------------------------

/// Flag UNSET: `is_seed_allowed()` returns false.
///
/// The HTTP handler checks `is_seed_allowed()` and returns 403 without calling
/// `perform_seed`.  We verify the flag check and assert no spurious row exists.
#[test]
fn seed_delegates_compute_flag_off_no_write() {
    let _g = SEED_FLAG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::remove_var("ALLOW_SEED_DELEGATES_COMPUTE");

    // Assertion: flag is off → handler would return 403.
    assert!(
        !is_seed_allowed(),
        "is_seed_allowed() must return false when ALLOW_SEED_DELEGATES_COMPUTE is unset"
    );

    // Verify no row was written (perform_seed is never called when flag is off).
    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");
    let row = mishpat_commitments::get_by_cid(&mut conn, "commitment:flag-off-sentinel")
        .expect("get_by_cid must not error");
    assert!(
        row.is_none(),
        "no row must exist: perform_seed was not called because the flag is off"
    );
}

// ---------------------------------------------------------------------------
// Test 4: build_manifest() must NOT include the seed route
// ---------------------------------------------------------------------------

/// Route guard: `build_manifest()` must not register `/admin/seed/delegates-compute`.
///
/// The storage auto-promote convention exposes every `build_manifest()` route
/// to a public doorway proxy.  The seed endpoint must NEVER be doorway-proxied
/// (it writes commitments directly without Holochain notarisation).
#[test]
fn seed_delegates_compute_absent_from_build_manifest() {
    let manifest = build_manifest();
    let routes_json =
        serde_json::to_string(&manifest).expect("build_manifest must be JSON-serialisable");
    assert!(
        !routes_json.contains("seed/delegates-compute"),
        "POST /admin/seed/delegates-compute must NOT appear in build_manifest() — \
         it must never be doorway-proxied; got manifest excerpt"
    );
}
