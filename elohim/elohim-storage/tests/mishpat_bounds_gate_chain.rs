//! Storage-level composition integration test: REA-rails chain end-to-end.
//!
//! # Scope
//!
//! This test drives the storage-side chain through REAL components (no mocks):
//!
//! ```text
//! project (signal) → fetch (ProjectionCommitmentFetcher) → bounds-gate
//!   (bounds_validator::validate) → graduate (graduate_to_active) → revoke → refuse
//! ```
//!
//! ## What this covers
//!
//! - The full data-transaction gate: a notarized, in-window, in-scope commitment
//!   clears the bounds-gate; a revoked commitment is refused; a null-anchor
//!   (un-notarized) row is refused (fail-closed).
//! - `handle_mishpat_signal` → `mishpat_commitments::upsert_with_anchor` (T5
//!   projection) operating against a real in-memory SQLite pool with migrations.
//! - `ProjectionCommitmentFetcher::fetch` reading from that pool (T6 fetcher).
//! - `bounds_validator::validate` with the REAL fetcher (not MockCommitmentFetcher)
//!   and a permissive MockRateHistory so the rate-limit check doesn't block.
//! - `graduate_to_active` (T7) flipping the state column.
//! - `set_revoked_at` + re-validation producing `CommitmentRevoked` violation (T8).
//!
//! ## What this does NOT cover (honest boundary)
//!
//! The conductor→storage signal wire (Mishpat conductor → app WebSocket →
//! `HolochainAppSignalStream` → `handle_mishpat_signal`) is a running-stack
//! concern, not unit-testable. The serde wire shape is covered by the
//! `commitment_committed_*` tests in `src/mishpat_projection.rs`. The full
//! live loop is a shakeout-stack concern (sweettest or running alpha cluster).
//!
//! Spec: 2026-06-07-epr-acquisition-pull-queue-design.md §6.5 (Slice-2a T8).

use elohim_storage::db::mishpat_commitments;
use elohim_storage::mishpat_projection::CommitmentPayload;
use elohim_storage::services::bounds_validator::{validate, BoundsViolation, EventForValidation};
use elohim_storage::services::commitment_fetcher::ProjectionCommitmentFetcher;
use elohim_storage::services::rate_history::{DieselRateHistory, MockRateHistory};
use elohim_storage::signals::{handle_mishpat_signal, MishpatSignal};
use elohim_storage::test_util::test_pool;
use elohim_views::bounds::ViolationKind;

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

/// Well-formed `delegates-compute` payload JSON (in-window, in-scope).
///
/// `signed_at` on `EventForValidation` must fall within [valid_from, valid_until].
/// We use a wide 2026-06 → 2026-09 window so tests are not calendar-sensitive.
fn delegates_compute_payload_json() -> String {
    serde_json::json!({
        "action": "delegates-compute",
        "scope": "republish-epr",
        "provider": "agent:matthew-steward",
        "recipient": "agent:deploy-svc-matthew",
        "bounds": {
            "epr_scope": ["epr:lamad-spa"],
            "reach_ceiling": "commons",
            "rate_per_hour": 30,
            "rotation_ttl_days": 90
        },
        "valid_from": "2026-06-01T00:00:00Z",
        "valid_until": "2026-09-01T00:00:00Z"
    })
    .to_string()
}

/// Build an `EventForValidation` that should PASS all seven checks against
/// the commitment inserted by `delegates_compute_payload_json`.
fn passing_event(bounded_by_cid: &str) -> EventForValidation {
    EventForValidation {
        action: "republish-epr".into(),
        performer: "agent:deploy-svc-matthew".into(),
        bounded_by: bounded_by_cid.to_string(),
        target_epr_id: "epr:lamad-spa".into(),
        reach: "commons".into(),
        // Must be within [valid_from, valid_until] and < rotation_ttl_days (90d) after valid_from.
        signed_at: "2026-07-01T12:00:00Z".into(),
    }
}

// ---------------------------------------------------------------------------
// Step 1 helper: project a CommitmentCommitted signal via the real handler
// ---------------------------------------------------------------------------

/// Call `handle_mishpat_signal` with a `CommitmentCommitted` signal carrying
/// a well-formed `delegates-compute` payload, then assert the row was written
/// into the pool with state="proposed" and dht_anchor_hash=Some(action_hash).
///
/// Returns the `entry_hash` used as the row's `cid`.
fn project_commitment(pool: &elohim_storage::db::DbPool) -> String {
    let entry_hash = "uhCEk-test-entry-chain-1".to_string();
    let action_hash = "uhCkk-test-action-chain-1".to_string();

    let signal = MishpatSignal::CommitmentCommitted {
        action_hash: action_hash.clone(),
        entry_hash: entry_hash.clone(),
        author: "agent:matthew-steward".into(),
        commitment: CommitmentPayload {
            action: "delegates-compute".into(),
            payload_json: delegates_compute_payload_json(),
            signed_at: "1748390400".into(), // epoch-seconds (metadata only)
        },
    };

    let mut conn = pool.get().expect("pool connection for projection");
    handle_mishpat_signal(&mut conn, "app", signal)
        .expect("handle_mishpat_signal must succeed for a well-formed CommitmentCommitted signal");

    entry_hash
}

// ---------------------------------------------------------------------------
// Step 1 + 2: project → fetch+bounds-pass
// ---------------------------------------------------------------------------

/// Step 1: project a CommitmentCommitted signal and assert the row is present
/// with state="proposed" and a non-null dht_anchor_hash.
#[tokio::test]
async fn step1_project_commitment_creates_proposed_notarized_row() {
    let pool = test_pool();
    let cid = project_commitment(&pool);

    let mut conn = pool.get().expect("pool conn");
    let row = mishpat_commitments::get_by_cid(&mut conn, &cid)
        .expect("get_by_cid must not error")
        .expect("row must be present after projection");

    assert_eq!(row.cid, cid, "cid must match entry_hash");
    assert_eq!(row.state, "proposed", "initial state must be 'proposed'");
    assert_eq!(
        row.dht_anchor_hash.as_deref(),
        Some("uhCkk-test-action-chain-1"),
        "dht_anchor_hash must be set to action_hash (notarized provenance)"
    );
    assert_eq!(row.action, "delegates-compute");
    assert_eq!(row.scope, "republish-epr");
}

/// Step 2: the REAL ProjectionCommitmentFetcher reads the projected row and
/// bounds_validator::validate clears the gate for a notarized, in-window,
/// in-scope event.
///
/// This is the core proof: bounds_validator operating through the REAL fetcher
/// against a REAL projected row (not MockCommitmentFetcher).
#[tokio::test]
async fn step2_real_fetcher_and_validator_clears_gate_for_notarized_commitment() {
    let pool = test_pool();
    let cid = project_commitment(&pool);

    // Build the REAL fetcher backed by the pool that has the projected row.
    let fetcher = ProjectionCommitmentFetcher::new(pool);
    // Permissive rate history: count=0 for any commitment → rate-limit check passes.
    let rate = MockRateHistory::new();

    let event = passing_event(&cid);
    let result = validate(&event, &fetcher, &rate).await;

    assert!(
        result.is_ok(),
        "bounds_validator must CLEAR the gate for a notarized, in-window, \
         in-scope commitment; got: {:?}",
        result
    );

    let checks = result.unwrap();
    assert!(checks.commitment_found, "commitment_found must be true");
    assert!(checks.not_revoked, "not_revoked must be true");
    assert!(checks.active, "active (window) must be true");
    assert!(
        checks.scope_includes_event,
        "scope_includes_event must be true"
    );
    assert!(
        checks.reach_ceiling_ok,
        "reach_ceiling_ok must be true (commons <= commons)"
    );
    assert!(checks.rate_within_limit, "rate_within_limit must be true");
    assert!(
        checks.key_rotation_current,
        "key_rotation_current must be true (within 90d rotation TTL)"
    );
}

// ---------------------------------------------------------------------------
// Step 3: graduate proposed → active
// ---------------------------------------------------------------------------

/// Step 3: after the bounds-gate clears, `graduate_to_active` flips the state
/// from "proposed" to "active" (simulating a notarized provide event projecting).
#[tokio::test]
async fn step3_graduate_to_active_flips_state() {
    let pool = test_pool();
    let cid = project_commitment(&pool);

    let mut conn = pool.get().expect("pool conn");

    // Pre-condition: state is "proposed".
    let pre = mishpat_commitments::get_by_cid(&mut conn, &cid)
        .expect("get_by_cid pre-graduation")
        .expect("row must exist");
    assert_eq!(pre.state, "proposed");

    // Graduate.
    let affected = mishpat_commitments::graduate_to_active(&mut conn, &cid)
        .expect("graduate_to_active must succeed");
    assert_eq!(
        affected, 1,
        "graduate_to_active must flip exactly one row from proposed → active"
    );

    // Post-condition: state is "active".
    let post = mishpat_commitments::get_by_cid(&mut conn, &cid)
        .expect("get_by_cid post-graduation")
        .expect("row must still exist after graduation");
    assert_eq!(
        post.state, "active",
        "state must be 'active' after graduation"
    );
}

// ---------------------------------------------------------------------------
// Step 4: revoke → refuse
// ---------------------------------------------------------------------------

/// Step 4: after revocation, bounds_validator must REFUSE the event with
/// `ViolationKind::CommitmentRevoked`. This proves the un-pin revocation property:
/// a revoked commitment's subsequent provide is refused.
#[tokio::test]
async fn step4_revoked_commitment_is_refused_by_bounds_gate() {
    let pool = test_pool();
    let cid = project_commitment(&pool);

    // Revoke the commitment.
    {
        let mut conn = pool.get().expect("pool conn for revocation");
        let affected = mishpat_commitments::set_revoked_at(&mut conn, &cid, "2026-06-15T00:00:00Z")
            .expect("set_revoked_at must succeed");
        assert_eq!(affected, 1, "set_revoked_at must touch exactly one row");
    }

    // Re-validate: the REAL fetcher now returns a row with revoked_at=Some(_).
    let fetcher = ProjectionCommitmentFetcher::new(pool);
    let rate = MockRateHistory::new();
    let event = passing_event(&cid);

    let result = validate(&event, &fetcher, &rate).await;

    assert!(
        matches!(
            result,
            Err(BoundsViolation {
                kind: ViolationKind::CommitmentRevoked,
                ..
            })
        ),
        "bounds_validator must REFUSE with CommitmentRevoked after revocation; got: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Step 5: null-anchor → refuse (defense-in-depth)
// ---------------------------------------------------------------------------

/// Step 5: a row with dht_anchor_hash=None (storage-only, un-notarized) must
/// never clear the bounds-gate. The ProjectionCommitmentFetcher returns
/// FetchError::NotarizedRequired and bounds_validator maps it to CommitmentNotFound.
///
/// Defense-in-depth: this path can arise if storage receives a row before the
/// DHT notarization signal arrives (un-anchored pre-projection). The gate must
/// be fail-closed regardless of how the un-notarized row got there.
#[tokio::test]
async fn step5_null_anchor_commitment_is_refused_by_bounds_gate() {
    let pool = test_pool();

    // Insert a row directly with dht_anchor_hash=None (un-notarized storage-only).
    // We bypass handle_mishpat_signal because the signal handler always sets
    // dht_anchor_hash = action_hash; this path simulates a hypothetical race
    // where a row lands without notarized provenance.
    let unanchored_cid = "uhCEk-unanchored-chain-test-1";
    {
        let mut conn = pool.get().expect("pool conn for unanchored insert");
        mishpat_commitments::upsert_with_anchor(
            &mut conn,
            elohim_storage::db::models::NewMishpatCommitment {
                cid: unanchored_cid.to_string(),
                action: "delegates-compute".to_string(),
                scope: "republish-epr".to_string(),
                provider: "agent:matthew-steward".to_string(),
                recipient: "agent:deploy-svc-matthew".to_string(),
                bounds_json: serde_json::json!({
                    "epr_scope": ["epr:lamad-spa"],
                    "reach_ceiling": "commons",
                    "rate_per_hour": 30,
                    "rotation_ttl_days": 90
                })
                .to_string(),
                valid_from: "2026-06-01T00:00:00Z".to_string(),
                valid_until: "2026-09-01T00:00:00Z".to_string(),
                revoked_at: None,
                state: "proposed".to_string(),
                dht_anchor_hash: None, // ← un-notarized: no DHT anchor
            },
        )
        .expect("upsert un-notarized row must succeed");
    }

    // Validate: the REAL fetcher must return NotarizedRequired → CommitmentNotFound.
    let fetcher = ProjectionCommitmentFetcher::new(pool);
    let rate = MockRateHistory::new();
    let event = passing_event(unanchored_cid);

    let result = validate(&event, &fetcher, &rate).await;

    assert!(
        matches!(
            result,
            Err(BoundsViolation {
                kind: ViolationKind::CommitmentNotFound,
                ..
            })
        ),
        "bounds_validator must REFUSE (CommitmentNotFound) for a null-anchor \
         (un-notarized) row — fail-closed per spec §6.5; got: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Full chain in a single test (canonical integration proof)
// ---------------------------------------------------------------------------

/// Canonical end-to-end chain test: project → fetch+bounds-pass → graduate →
/// revoke → refuse → null-anchor refuse, all in one sequential flow.
///
/// This is the "receipt" test that proves the entire storage-side REA-rails
/// chain works through the REAL components. The individual step tests above
/// provide isolation for debugging; this test is the single authoritative proof.
///
/// ## Conductor→storage wire boundary
///
/// The conductor→storage signal wire (Mishpat conductor → app WebSocket →
/// `HolochainAppSignalStream`) is a live-stack concern; the serde shape is
/// covered by `src/mishpat_projection.rs:decode_commitment_payload_from_dna_wire_shape`.
/// This test exercises the storage layer from `handle_mishpat_signal` inward.
#[tokio::test]
async fn mishpat_bounds_gate_chain_end_to_end() {
    let pool = test_pool();

    // ── 1. Project ────────────────────────────────────────────────────────────
    let cid = project_commitment(&pool);

    {
        let mut conn = pool.get().expect("pool conn for step-1 check");
        let row = mishpat_commitments::get_by_cid(&mut conn, &cid)
            .expect("get_by_cid step 1")
            .expect("row must exist after projection");
        assert_eq!(row.state, "proposed", "step 1: state must be proposed");
        assert!(
            row.dht_anchor_hash.is_some(),
            "step 1: dht_anchor_hash must be set (notarized)"
        );
    }

    // ── 2. Fetch + bounds-pass ────────────────────────────────────────────────
    {
        let fetcher = ProjectionCommitmentFetcher::new(pool.clone());
        let rate = MockRateHistory::new();
        let event = passing_event(&cid);
        let result = validate(&event, &fetcher, &rate).await;
        assert!(
            result.is_ok(),
            "step 2: bounds MUST clear for a notarized in-window commitment; got: {:?}",
            result
        );
    }

    // ── 3. Graduate proposed → active ────────────────────────────────────────
    {
        let mut conn = pool.get().expect("pool conn for step-3");
        let affected = mishpat_commitments::graduate_to_active(&mut conn, &cid)
            .expect("graduate_to_active step 3");
        assert_eq!(affected, 1, "step 3: exactly one row must be graduated");

        let row = mishpat_commitments::get_by_cid(&mut conn, &cid)
            .expect("get_by_cid step 3")
            .expect("row must exist");
        assert_eq!(row.state, "active", "step 3: state must be active");
    }

    // ── 4. Revoke → refuse ───────────────────────────────────────────────────
    {
        let mut conn = pool.get().expect("pool conn for step-4 revoke");
        mishpat_commitments::set_revoked_at(&mut conn, &cid, "2026-06-15T00:00:00Z")
            .expect("set_revoked_at step 4");
    }
    {
        let fetcher = ProjectionCommitmentFetcher::new(pool.clone());
        let rate = MockRateHistory::new();
        let event = passing_event(&cid);
        let result = validate(&event, &fetcher, &rate).await;
        assert!(
            matches!(
                result,
                Err(BoundsViolation {
                    kind: ViolationKind::CommitmentRevoked,
                    ..
                })
            ),
            "step 4: bounds MUST refuse with CommitmentRevoked after revocation; got: {:?}",
            result
        );
    }

    // ── 5. Null-anchor → refuse ───────────────────────────────────────────────
    let unanchored_cid = "uhCEk-unanchored-chain-e2e-1";
    {
        let mut conn = pool.get().expect("pool conn for step-5 insert");
        mishpat_commitments::upsert_with_anchor(
            &mut conn,
            elohim_storage::db::models::NewMishpatCommitment {
                cid: unanchored_cid.to_string(),
                action: "delegates-compute".to_string(),
                scope: "republish-epr".to_string(),
                provider: "agent:matthew-steward".to_string(),
                recipient: "agent:deploy-svc-matthew".to_string(),
                bounds_json: serde_json::json!({
                    "epr_scope": ["epr:lamad-spa"],
                    "reach_ceiling": "commons",
                    "rate_per_hour": 30,
                    "rotation_ttl_days": 90
                })
                .to_string(),
                valid_from: "2026-06-01T00:00:00Z".to_string(),
                valid_until: "2026-09-01T00:00:00Z".to_string(),
                revoked_at: None,
                state: "proposed".to_string(),
                dht_anchor_hash: None, // un-notarized
            },
        )
        .expect("upsert un-notarized row step 5");
    }
    {
        let fetcher = ProjectionCommitmentFetcher::new(pool.clone());
        let rate = MockRateHistory::new();
        let event = passing_event(unanchored_cid);
        let result = validate(&event, &fetcher, &rate).await;
        assert!(
            matches!(
                result,
                Err(BoundsViolation {
                    kind: ViolationKind::CommitmentNotFound,
                    ..
                })
            ),
            "step 5: bounds MUST refuse (CommitmentNotFound) for null-anchor row; got: {:?}",
            result
        );
    }
}

// ---------------------------------------------------------------------------
// N2: put_epr republish-epr gate is wired to the production ProjectionCommitmentFetcher
//
// Regression guard for the N2 fix. `put_epr` (src/api/epr.rs) previously held a
// hardcoded `let hc_lamad: Option<Arc<HcClient>> = None;` whose `None` arm
// unconditionally returned 503 service_unavailable — the republish-epr bounds
// gate never executed. The fix swaps to `ProjectionCommitmentFetcher::new(pool)`
// + `DieselRateHistory { pool }` and calls `validate_republish_epr` directly
// (the pool is already a parameter; P1: storage as reconciliation controller).
//
// These tests exercise the EXACT function-and-fetcher pairing the handler now
// uses — `validate_republish_epr` over a real ProjectionCommitmentFetcher
// reading a projected notarized row — proving the gate actually runs (pass on
// a clean commitment, refuse on a revoked one), not bypassed/503.
// ---------------------------------------------------------------------------

use elohim_storage::services::republish_epr_validator::{validate_republish_epr, ValidationError};

/// A well-formed `republish-epr` EconomicEvent payload bounded by the projected
/// commitment. Mirrors the JSON the put_epr handler hands to
/// `validate_republish_epr` (the `input.event` serialized to a serde Value).
fn republish_event_json(bounded_by_cid: &str) -> serde_json::Value {
    serde_json::json!({
        "action": "republish-epr",
        "performer": "agent:deploy-svc-matthew",
        "bounded_by": bounded_by_cid,
        "target": "epr-head-new",
        "payload": {
            "blob_cid": "bafkrei-blob-cid",
            "epr_kind": "Content",
            "reach": "commons",
            "bundle_path": "lamad-spa"
        },
        // Within [valid_from, valid_until] (2026-06 → 2026-09) and inside the
        // 90-day rotation TTL measured from valid_from.
        "signed_at": "2026-07-01T12:00:00Z"
    })
}

/// The put_epr gate CLEARS for a notarized, in-window, in-scope commitment when
/// driven through the production fetcher pairing the N2 fix installed.
#[tokio::test]
async fn n2_put_epr_republish_gate_clears_via_projection_fetcher() {
    let pool = test_pool();
    let cid = project_commitment(&pool);

    // Exact production wiring from the N2 fix in put_epr.
    let fetcher = ProjectionCommitmentFetcher::new(pool.clone());
    let rate = DieselRateHistory { pool: pool.clone() };

    let event = republish_event_json(&cid);
    let result = validate_republish_epr(&event, &fetcher, &rate, "epr:lamad-spa").await;

    assert!(
        result.is_ok(),
        "N2: republish-epr gate MUST clear for a notarized in-window commitment \
         through the production ProjectionCommitmentFetcher + DieselRateHistory \
         (previously the hardcoded None returned 503 and never ran); got: {:?}",
        result
    );
}

/// The put_epr gate REFUSES when the bounding commitment is revoked — proving
/// the gate genuinely executes (not the prior unconditional 503 bypass).
#[tokio::test]
async fn n2_put_epr_republish_gate_refuses_revoked_commitment() {
    let pool = test_pool();
    let cid = project_commitment(&pool);

    {
        let mut conn = pool.get().expect("pool conn for revoke");
        mishpat_commitments::set_revoked_at(&mut conn, &cid, "2026-06-15T00:00:00Z")
            .expect("set_revoked_at");
    }

    let fetcher = ProjectionCommitmentFetcher::new(pool.clone());
    let rate = DieselRateHistory { pool: pool.clone() };

    let event = republish_event_json(&cid);
    let result = validate_republish_epr(&event, &fetcher, &rate, "epr:lamad-spa").await;

    assert!(
        matches!(result, Err(ValidationError::Bounds(_))),
        "N2: republish-epr gate MUST refuse a revoked commitment (Bounds \
         violation) — proving the gate executes rather than 503-bypassing; got: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// N2-infra: an INFRA failure reaching the commitment store is a 503-class
// `ValidationError::Unavailable`, NOT a 400-class `Bounds` rejection.
//
// Regression guard for the error-class fix: `put_epr` previously mapped EVERY
// validation `Err` to `bad_request(400)`, so a transient db pool/query outage
// (the `FetchError::ConductorUnreachable` class, which `bounds_validator`
// fail-closed-collapses into `CommitmentNotFound`) was mis-reported as a caller
// fault. The fix re-introduces the infra distinction at the republish layer
// (`ValidationError::Unavailable`) and maps it to 503 — restoring the
// pre-change behaviour of the formerly-hardcoded `None` arm for the infra case.
// ---------------------------------------------------------------------------

/// Build a deliberately-poisoned pool whose every `get()` fails: it points at
/// an unopenable SQLite URL (`mode=rw` against a non-existent directory, so the
/// file is never created) and is built with `build_unchecked` + a short
/// `connection_timeout` so construction succeeds but checkout fails fast. This
/// drives `ProjectionCommitmentFetcher::fetch` down the
/// `FetchError::ConductorUnreachable` (infra) arm.
fn poisoned_pool() -> elohim_storage::db::DbPool {
    use diesel::r2d2::{ConnectionManager, Pool};
    use elohim_storage::db::SqliteConnection;
    use std::time::Duration;

    let unopenable = "file:/nonexistent_dir_n2_infra/blocked.db?mode=rw";
    let manager = ConnectionManager::<SqliteConnection>::new(unopenable);
    Pool::builder()
        .max_size(1)
        .min_idle(Some(0))
        .connection_timeout(Duration::from_millis(200))
        .build_unchecked(manager)
}

/// A pool/query infra failure path returns the 503-class `Unavailable`, not a
/// 400-class `Bounds` violation.
#[tokio::test]
async fn n2_put_epr_republish_gate_infra_failure_is_unavailable_503_class() {
    let pool = poisoned_pool();

    // Sanity: this pool genuinely cannot hand out a connection.
    assert!(
        pool.get().is_err(),
        "poisoned pool must fail to check out a connection (test precondition)"
    );

    let fetcher = ProjectionCommitmentFetcher::new(pool.clone());
    let rate = DieselRateHistory { pool };

    // A schema-valid event so we reach the bounds/fetch path (not a Schema error).
    let event = republish_event_json("uhCEk-some-commitment-cid");
    let result = validate_republish_epr(&event, &fetcher, &rate, "epr:lamad-spa").await;

    assert!(
        matches!(result, Err(ValidationError::Unavailable(_))),
        "N2-infra: a pool/query infra failure MUST surface as the 503-class \
         ValidationError::Unavailable (NOT a 400-class Bounds rejection); got: {:?}",
        result
    );
}
