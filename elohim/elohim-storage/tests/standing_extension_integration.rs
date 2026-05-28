//! Integration: projecting the 3 Z.D-related extension signal_kinds into
//! standing_view moves StandingScore in the expected direction.
//!
//! Weights come from the elohim domain manifest (`sdk/domains/elohim/manifest.json`),
//! loaded at runtime by `signal_weight_registry::weight_for`:
//!
//! | signal_kind               | debit_weight |
//! |---------------------------|-------------|
//! | rate-limit-exceeded       | 5           |
//! | bad-custody               | 20          |
//! | reach-escalation-pending  | 3           |
//!
//! Standing thresholds (from `score_for_debit_sum`):
//!   sum ≤ -3 → Trusted, -2..=-1 → High, 0..=2 → Neutral, 3..=7 → Low, 8+ → Floor
//!
//! Tests are isolated: each uses a fresh in-memory DB (via `test_util::test_pool`)
//! with all migrations applied, including the `standing_view` migration
//! (`2026-05-01-040000_standing_view/up.sql`).

use elohim_storage::services::standing::StandingScore;
use elohim_storage::services::standing_projector::project_extension_signal;
use elohim_storage::test_util::test_pool;

// ---------------------------------------------------------------------------
// Shared test helpers
// ---------------------------------------------------------------------------

fn eval() -> Vec<u8> {
    b"evaluator-pubkey-extension".to_vec()
}

fn subj(tag: &str) -> Vec<u8> {
    // Each test uses a distinct subject so DB rows don't bleed across tests
    // when tests happen to reuse the same pool (they don't here, but defensive).
    format!("subject-pubkey-{tag}").into_bytes()
}

// ---------------------------------------------------------------------------
// Test 1: rate-limit-exceeded accumulates across signals and crosses Floor
// ---------------------------------------------------------------------------

/// Two `rate-limit-exceeded` signals (weight 5 each → debit_sum 10) push the
/// subject to Floor.  A third signal applied after already at Floor stays Floor.
///
/// Demonstrates that cumulative application of the same extension signal_kind
/// works correctly: the projector reads the existing `debit_weight_sum` from the
/// DB and adds the new weight rather than resetting.
#[test]
fn rate_limit_exceeded_signal_debits_target_standing() {
    let pool = test_pool();
    let mut conn = pool.get().expect("conn");
    let evaluator = eval();
    let target = subj("rate-limit");

    // First signal: sum = 5 → Low (3..=7).
    let first = project_extension_signal(
        &mut conn,
        &evaluator,
        &target,
        "rate-limit-exceeded",
        "manifest-cid-test",
    )
    .expect("first signal");
    assert_eq!(first, StandingScore::Low, "sum=5 should be Low");

    // Second signal: sum = 10 → Floor (≥8).
    let second = project_extension_signal(
        &mut conn,
        &evaluator,
        &target,
        "rate-limit-exceeded",
        "manifest-cid-test",
    )
    .expect("second signal");
    assert_eq!(second, StandingScore::Floor, "sum=10 should be Floor");

    // Third signal: sum = 15 → still Floor.
    let third = project_extension_signal(
        &mut conn,
        &evaluator,
        &target,
        "rate-limit-exceeded",
        "manifest-cid-test",
    )
    .expect("third signal");
    assert!(
        matches!(third, StandingScore::Floor),
        "sum=15 should stay Floor"
    );
}

// ---------------------------------------------------------------------------
// Test 2: bad-custody single signal crosses directly to Floor
// ---------------------------------------------------------------------------

/// A single `bad-custody` signal carries weight 20, well above the Floor
/// threshold (≥8), so one signal alone drives the subject to Floor.
///
/// This is the highest-weight signal in the bounds-validator set — it represents
/// revoked-or-rotation-stale Commitment usage, a hard custody violation.
#[test]
fn bad_custody_signal_higher_weight() {
    let pool = test_pool();
    let mut conn = pool.get().expect("conn");
    let evaluator = eval();
    let target = subj("bad-custody");

    // ONE bad-custody (weight 20) → Floor (since 20 ≥ 8 threshold).
    let score = project_extension_signal(
        &mut conn,
        &evaluator,
        &target,
        "bad-custody",
        "mfst",
    )
    .expect("bad-custody signal");
    assert_eq!(score, StandingScore::Floor, "bad-custody weight=20 should reach Floor");
}

// ---------------------------------------------------------------------------
// Test 3: reach-escalation-pending is a soft-warn that lands at Low
// ---------------------------------------------------------------------------

/// A single `reach-escalation-pending` signal carries weight 3, landing exactly
/// at the Low threshold boundary (3..=7).  This is by design — reach escalations
/// are advisory/soft-warn per §3 of the Z.D substrate-correct deploy spec.
#[test]
fn reach_escalation_pending_signal_debits_lightly() {
    let pool = test_pool();
    let mut conn = pool.get().expect("conn");
    let evaluator = eval();
    let target = subj("reach-escalation");

    // ONE reach-escalation-pending (weight 3) → Low (3..=7 boundary).
    let score = project_extension_signal(
        &mut conn,
        &evaluator,
        &target,
        "reach-escalation-pending",
        "mfst",
    )
    .expect("reach-escalation-pending signal");
    assert_eq!(score, StandingScore::Low, "reach-escalation-pending weight=3 should be Low");
}

// ---------------------------------------------------------------------------
// Test 4: unknown signal_kind applies zero weight → Neutral (graceful degradation)
// ---------------------------------------------------------------------------

/// When `signal_weight_registry::weight_for` returns None (signal_kind not
/// declared in the elohim manifest), `project_extension_signal` must NOT error
/// or panic.  It applies weight 0, writes the standing_view row with
/// `debit_weight_sum = 0`, and returns `StandingScore::Neutral`.
///
/// The `tracing::warn!` is emitted but not asserted here (would require a
/// tracing subscriber harness) — the invariant tested is that no error is
/// returned and the score is Neutral.
#[test]
fn unknown_signal_kind_logs_warn_and_applies_zero() {
    let pool = test_pool();
    let mut conn = pool.get().expect("conn");
    let evaluator = eval();
    let target = subj("unknown-kind");

    // Unregistered signal_kind → weight 0 → Neutral (0..=2 threshold, sum=0).
    let score = project_extension_signal(
        &mut conn,
        &evaluator,
        &target,
        "totally-unknown-kind",
        "mfst",
    )
    .expect("unknown kind must not error");
    assert_eq!(
        score,
        StandingScore::Neutral,
        "unknown signal_kind with weight 0 should produce Neutral"
    );
}
