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
// Test 2b: re-seed AFTER revoke must be refused (revoke is terminal for dev-seed)
// ---------------------------------------------------------------------------

/// Idempotency footgun guard: the dev seeder is advertised idempotent and its CID
/// is deterministic, so a re-run of `seed:delegates` AFTER `{revoke:true}` would
/// silently un-revoke the grant via the shared `upsert_with_anchor`. `perform_seed`
/// must refuse: re-seeding a revoked row returns `Err` and the row stays revoked.
#[test]
fn seed_delegates_compute_reseed_after_revoke_refused() {
    let _g = SEED_FLAG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("ALLOW_SEED_DELEGATES_COMPUTE", "1");

    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");

    let cid = "commitment:che-opgate-reseed-after-revoke-001";
    let input = SeedDelegatesInput {
        cid,
        scope: SCOPE,
        provider: PROVIDER,
        recipient: RECIPIENT,
        bounds_json: BOUNDS_JSON,
        valid_from: VALID_FROM,
        valid_until: VALID_UNTIL,
    };

    // 1. Seed (active).
    perform_seed(&mut conn, &input).expect("initial seed must succeed");

    // 2. Revoke via the real revoke path.
    let ts = "2026-07-01T12:00:00Z";
    mishpat_commitments::set_revoked_at(&mut conn, cid, ts).expect("set_revoked_at must succeed");

    // 3. Re-seed: must be REFUSED (revoke is terminal for dev-seed).
    let err = perform_seed(&mut conn, &input)
        .expect_err("re-seeding a revoked row must return Err, not silently reactivate");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("revoked"),
        "error must explain the revocation refusal; got: {msg}"
    );

    // 4. The row stays revoked and is NOT reactivated.
    let post = mishpat_commitments::get_by_cid(&mut conn, cid)
        .expect("get_by_cid post-reseed")
        .expect("row must still exist");
    assert!(
        post.revoked_at.is_some(),
        "revoked_at must remain non-NULL after the refused re-seed"
    );
    assert_eq!(
        post.revoked_at.as_deref(),
        Some(ts),
        "revoked_at must equal the original revoke timestamp (not reset by re-seed)"
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

// ---------------------------------------------------------------------------
// Test 5: dev-seed supersession — a new dev-seed grant revokes prior ACTIVE
// dev-seeded grants for the same (recipient, scope)
// ---------------------------------------------------------------------------

/// Dev-seed hygiene: scenario sequences (grant → use → grant → revoke → use)
/// require that "the holder's grant" is singular per (recipient, scope) at any
/// moment. Without supersession, a revoked fresh grant is shadowed by an
/// earlier still-active dev-seeded grant (find_active_delegates_compute picks
/// newest-first among ACTIVE rows), so "a revoked commitment stops working
/// immediately" can never be observed on an accumulated dev DB. Supersession
/// is scoped HARD to rows whose bounds carry `_provenance:"dev-seed"` — a
/// DHT-projected (notarized) grant must never be touched by the dev lever.
#[test]
fn seed_delegates_compute_supersedes_prior_active_dev_seeded_grant() {
    let _g = SEED_FLAG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("ALLOW_SEED_DELEGATES_COMPUTE", "1");

    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");

    let seed = |conn: &mut _, cid: &str, recipient: &str, scope: &str, bounds: &str| {
        perform_seed(
            conn,
            &SeedDelegatesInput {
                cid,
                scope,
                provider: PROVIDER,
                recipient,
                bounds_json: bounds,
                valid_from: VALID_FROM,
                valid_until: VALID_UNTIL,
            },
        )
        .expect("perform_seed must succeed")
    };

    // Grant A then grant B for the SAME (recipient, scope).
    seed(
        &mut conn,
        "commitment:supersede-a",
        RECIPIENT,
        SCOPE,
        BOUNDS_JSON,
    );
    // Unrelated rows that must survive B's seeding untouched:
    seed(
        &mut conn,
        "commitment:supersede-other-scope",
        RECIPIENT,
        "operator-reconcile",
        BOUNDS_JSON,
    );
    seed(
        &mut conn,
        "commitment:supersede-other-recipient",
        "agent:someone-else",
        SCOPE,
        BOUNDS_JSON,
    );
    let b = seed(
        &mut conn,
        "commitment:supersede-b",
        RECIPIENT,
        SCOPE,
        BOUNDS_JSON,
    );

    // B is active; A got superseded (revoked_at set).
    assert!(b.revoked_at.is_none(), "the new grant must be active");
    let a = mishpat_commitments::get_by_cid(&mut conn, "commitment:supersede-a")
        .expect("get_by_cid")
        .expect("row a exists");
    assert!(
        a.revoked_at.is_some(),
        "prior active dev-seeded grant for the same (recipient, scope) must be superseded"
    );

    // Same recipient, different scope — untouched.
    let other_scope =
        mishpat_commitments::get_by_cid(&mut conn, "commitment:supersede-other-scope")
            .expect("get_by_cid")
            .expect("row exists");
    assert!(
        other_scope.revoked_at.is_none(),
        "different scope must not be superseded"
    );

    // Same scope, different recipient — untouched.
    let other_recipient =
        mishpat_commitments::get_by_cid(&mut conn, "commitment:supersede-other-recipient")
            .expect("get_by_cid")
            .expect("row exists");
    assert!(
        other_recipient.revoked_at.is_none(),
        "different recipient must not be superseded"
    );
}

/// A grant whose bounds do NOT carry `_provenance:"dev-seed"` (i.e. a
/// DHT-projected/notarized row) must never be revoked by dev-seed supersession.
#[test]
fn seed_delegates_compute_supersession_never_touches_non_dev_seeded_rows() {
    let _g = SEED_FLAG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("ALLOW_SEED_DELEGATES_COMPUTE", "1");

    let pool = test_pool();
    let mut conn = pool.get().expect("pool connection");

    // A notarized-shaped row (no dev-seed marker) for the same (recipient, scope),
    // written through the shared projection upsert like the DHT signal path does.
    mishpat_commitments::upsert_with_anchor(
        &mut conn,
        elohim_storage::db::models::NewMishpatCommitment {
            cid: "commitment:notarized-grant".into(),
            action: "delegates-compute".into(),
            scope: SCOPE.into(),
            provider: PROVIDER.into(),
            recipient: RECIPIENT.into(),
            bounds_json: r#"{"rate_per_hour":30,"reach_ceiling":"commons"}"#.into(),
            valid_from: VALID_FROM.into(),
            valid_until: VALID_UNTIL.into(),
            revoked_at: None,
            dht_anchor_hash: Some("uhCkk-test-anchor".into()),
            state: "active".into(),
        },
    )
    .expect("upsert notarized row");

    perform_seed(
        &mut conn,
        &SeedDelegatesInput {
            cid: "commitment:dev-seeded-after-notarized",
            scope: SCOPE,
            provider: PROVIDER,
            recipient: RECIPIENT,
            bounds_json: BOUNDS_JSON,
            valid_from: VALID_FROM,
            valid_until: VALID_UNTIL,
        },
    )
    .expect("perform_seed must succeed");

    let notarized = mishpat_commitments::get_by_cid(&mut conn, "commitment:notarized-grant")
        .expect("get_by_cid")
        .expect("row exists");
    assert!(
        notarized.revoked_at.is_none(),
        "a non-dev-seeded (notarized) grant must NEVER be revoked by the dev lever"
    );
}
