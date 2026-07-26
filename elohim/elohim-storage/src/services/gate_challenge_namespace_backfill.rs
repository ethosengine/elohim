//! Level-triggered heal for the gate-challenge projections' `h_app_id` namespace.
//!
//! ## The gap it closes (the `h_app_id` pillar-namespace mismatch)
//!
//! Sibling of [`crate::services::mishpat_mirror_backfill`], same defect class,
//! two more tables. `signals::handle_mishpat_signal` projects
//! `GateDecisionChallengeCreated` → `gate_decision_challenges` and
//! `ChallengeOutcomeCreated` → `challenge_outcomes`, stamping each row's
//! `app_id` from the value the subscriber was handed. On a deployed node that
//! argument was the conductor's **installed-hApp id** (`HOLOCHAIN_APP_ID`,
//! `"elohim"`), while EVERY reader of both tables scopes to the **projection
//! namespace** `"lamad"` — `api::extract_app_context` defaults to `"lamad"` and
//! nothing in the tree sends `X-App-Id`, so `ctx.h_app_id` is `"lamad"` at
//! `GET /db/gate-decision-challenges`, `GET /db/challenge-outcomes`, and
//! `GET /db/challenge-outcomes/{id}`. The rows landed correctly shaped and
//! correctly anchored, and were invisible to all three.
//!
//! The wiring fix (pass `AppContext::default_lamad().h_app_id` to the
//! subscriber) is **edge-triggered**: it only helps challenges notarized after
//! it ships. A challenge already notarized never re-signals, so its mis-filed
//! projection would stay dark forever.
//!
//! ## Why re-file ONLY — no synthetic inserts
//!
//! This is where the two tables DIVERGE from `rea_commitments`, and the
//! divergence is load-bearing:
//!
//! 1. **The PK is composite** — `(app_id, challenge_id)` / `(app_id,
//!    outcome_id)`. `app_id` is *half the key*, not a bare scoping column. So a
//!    mis-filed row is invisible but **not** insert-blocking, and the re-file
//!    must be keyed by BOTH halves. It also introduces a case
//!    `rea_commitments` cannot have: a mis-filed row whose id ALSO exists at
//!    the canonical namespace, where a naive `UPDATE app_id` collides with the
//!    PK.
//! 2. **There is no local desired-state ledger to insert FROM.** The
//!    `mishpat_commitments` ledger let the mirror sweep synthesize absent rows;
//!    here the projection rows ARE the only local record of the DHT truth. An
//!    absent challenge is not detectable locally and not healable locally — it
//!    means the signal never landed, which only a DHT re-read or a re-delivered
//!    signal can fix. Inventing a row would be fabricating notarized content.
//!
//! So the three-way reconcile per candidate is: **mis-filed with no canonical
//! twin → re-file**, **mis-filed but shadowed by a canonical twin → skip**
//! (non-destructive: the twin already serves readers, and deleting a row in a
//! heal sweep is data loss for no gain), **already canonical → not a candidate
//! at all**.
//!
//! ## Bounded
//!
//! Each table contributes at most [`MAX_ROWS_PER_SWEEP`] candidates per run. A
//! larger dark set drains across successive runs rather than stalling one tick —
//! level-triggered, so the next run simply re-observes what is still foreign.
//!
//! Category C (operational): idempotent. Once a row is canonical it stops being
//! a candidate, so a second run is a pure no-op and it is safe to run on every
//! boot.

use diesel::sqlite::SqliteConnection;

use crate::db::{challenge_outcomes, gate_decision_challenges};
use crate::error::StorageError;

/// Per-table candidate cap for one sweep. The dark set on any real node is tiny
/// (governance challenges are rare); the cap exists so a pathological table
/// cannot monopolize a boot tick.
pub const MAX_ROWS_PER_SWEEP: i64 = 500;

/// Outcome counters for one sweep (returned for logging/tests).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GateChallengeNamespaceReport {
    /// `gate_decision_challenges` rows observed under a foreign namespace.
    pub challenges_examined: usize,
    /// `gate_decision_challenges` rows re-filed into the canonical namespace.
    pub challenges_refiled: usize,
    /// `gate_decision_challenges` rows left alone because a correctly-filed twin
    /// already occupies `(canonical, challenge_id)`.
    pub challenges_shadowed: usize,
    /// `challenge_outcomes` rows observed under a foreign namespace.
    pub outcomes_examined: usize,
    /// `challenge_outcomes` rows re-filed into the canonical namespace.
    pub outcomes_refiled: usize,
    /// `challenge_outcomes` rows left alone because a correctly-filed twin
    /// already occupies `(canonical, outcome_id)`.
    pub outcomes_shadowed: usize,
}

impl GateChallengeNamespaceReport {
    /// Whether the sweep changed anything (drives log level at the call site).
    pub fn healed_anything(&self) -> bool {
        self.challenges_refiled > 0 || self.outcomes_refiled > 0
    }

    /// Whether the sweep saw anything mis-filed at all (healed or shadowed).
    pub fn saw_anything(&self) -> bool {
        self.challenges_examined > 0 || self.outcomes_examined > 0
    }
}

/// What the reconcile decided for one mis-filed row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reconciled {
    /// Moved into the canonical namespace.
    Refiled,
    /// A correctly-filed twin already holds the canonical key — left in place.
    Shadowed,
    /// The row vanished between the candidate read and the update (concurrent
    /// signal re-delivery). Nothing to do.
    Vanished,
}

/// Decide and apply the namespace repair for ONE mis-filed row.
///
/// Both table-specific operations arrive as closures so the three-way decision
/// lives in exactly ONE place, the way `mishpat_mirror_backfill::reconcile_one`
/// keeps its own.
fn reconcile_one(
    conn: &mut SqliteConnection,
    canonical_occupied: impl FnOnce(&mut SqliteConnection) -> Result<bool, StorageError>,
    refile: impl FnOnce(&mut SqliteConnection) -> Result<usize, StorageError>,
) -> Result<Reconciled, StorageError> {
    // `app_id` is half the PK here, so the canonical slot can already be taken.
    // Check BEFORE the update — an UPDATE into an occupied key is a constraint
    // violation, not a silent no-op.
    if canonical_occupied(conn)? {
        return Ok(Reconciled::Shadowed);
    }
    if refile(conn)? > 0 {
        Ok(Reconciled::Refiled)
    } else {
        Ok(Reconciled::Vanished)
    }
}

/// Run one namespace-reconcile sweep over both gate-challenge projections.
///
/// `canonical_h_app_id` is the PROJECTION namespace every reader of these tables
/// scopes to (`AppContext::default_lamad().h_app_id`, which is also
/// `api::extract_app_context`'s default) — never the conductor's installed-hApp
/// id.
///
/// A per-row failure is logged and skipped, never fatal: one malformed row must
/// not deny the heal to every other challenge (the same per-row degradation
/// discipline a resolver feeding a live surface owes its callers).
pub fn run_once(
    conn: &mut SqliteConnection,
    canonical_h_app_id: &str,
) -> Result<GateChallengeNamespaceReport, StorageError> {
    let mut report = GateChallengeNamespaceReport::default();

    // --- gate_decision_challenges -----------------------------------------
    let challenge_candidates = gate_decision_challenges::foreign_namespace_rows(
        conn,
        canonical_h_app_id,
        MAX_ROWS_PER_SWEEP,
    )
    .map_err(|e| StorageError::Database(e.to_string()))?;
    report.challenges_examined = challenge_candidates.len();

    for (foreign_ns, challenge_id) in challenge_candidates {
        let outcome = reconcile_one(
            conn,
            |c| {
                gate_decision_challenges::find_by_id(c, canonical_h_app_id, &challenge_id)
                    .map(|row| row.is_some())
                    .map_err(|e| StorageError::Database(e.to_string()))
            },
            |c| {
                gate_decision_challenges::refile_namespace(
                    c,
                    &foreign_ns,
                    &challenge_id,
                    canonical_h_app_id,
                )
                .map_err(|e| StorageError::Database(e.to_string()))
            },
        );

        match outcome {
            Ok(Reconciled::Refiled) => {
                report.challenges_refiled += 1;
                tracing::info!(
                    challenge_id = %challenge_id,
                    from_app_id = %foreign_ns,
                    to_app_id = %canonical_h_app_id,
                    "gate_challenge_namespace_backfill: gate decision challenge re-filed"
                );
            }
            Ok(Reconciled::Shadowed) => {
                report.challenges_shadowed += 1;
                tracing::info!(
                    challenge_id = %challenge_id,
                    from_app_id = %foreign_ns,
                    "gate_challenge_namespace_backfill: challenge left in place (canonical twin already filed)"
                );
            }
            Ok(Reconciled::Vanished) => {}
            Err(e) => tracing::warn!(
                error = %e,
                challenge_id = %challenge_id,
                "gate_challenge_namespace_backfill: challenge reconcile failed (skipped; retries next boot)"
            ),
        }
    }

    // --- challenge_outcomes ------------------------------------------------
    let outcome_candidates =
        challenge_outcomes::foreign_namespace_rows(conn, canonical_h_app_id, MAX_ROWS_PER_SWEEP)
            .map_err(|e| StorageError::Database(e.to_string()))?;
    report.outcomes_examined = outcome_candidates.len();

    for (foreign_ns, outcome_id) in outcome_candidates {
        let outcome = reconcile_one(
            conn,
            |c| {
                challenge_outcomes::find_by_id(c, canonical_h_app_id, &outcome_id)
                    .map(|row| row.is_some())
                    .map_err(|e| StorageError::Database(e.to_string()))
            },
            |c| {
                challenge_outcomes::refile_namespace(
                    c,
                    &foreign_ns,
                    &outcome_id,
                    canonical_h_app_id,
                )
                .map_err(|e| StorageError::Database(e.to_string()))
            },
        );

        match outcome {
            Ok(Reconciled::Refiled) => {
                report.outcomes_refiled += 1;
                tracing::info!(
                    outcome_id = %outcome_id,
                    from_app_id = %foreign_ns,
                    to_app_id = %canonical_h_app_id,
                    "gate_challenge_namespace_backfill: challenge outcome re-filed"
                );
            }
            Ok(Reconciled::Shadowed) => {
                report.outcomes_shadowed += 1;
                tracing::info!(
                    outcome_id = %outcome_id,
                    from_app_id = %foreign_ns,
                    "gate_challenge_namespace_backfill: outcome left in place (canonical twin already filed)"
                );
            }
            Ok(Reconciled::Vanished) => {}
            Err(e) => tracing::warn!(
                error = %e,
                outcome_id = %outcome_id,
                "gate_challenge_namespace_backfill: outcome reconcile failed (skipped; retries next boot)"
            ),
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::challenge_outcomes::ChallengeOutcomeRow;
    use crate::db::gate_decision_challenges::GateDecisionChallengeRow;
    use crate::db::AppContext;
    use crate::test_util::test_pool;

    /// The projection namespace every reader of these tables scopes to.
    const CANONICAL: &str = "lamad";
    /// The conductor's installed-hApp id — what the subscriber was WRONGLY
    /// handed, and what the mis-filed production rows carry.
    const INSTALLED_APP_ID: &str = "elohim";

    fn challenge_row(app_id: &str, challenge_id: &str) -> GateDecisionChallengeRow {
        GateDecisionChallengeRow {
            app_id: app_id.into(),
            challenge_id: challenge_id.into(),
            challenged_decision_cid: "bafyrei-decision-1".into(),
            challenger_id: "uhCAk-matthew".into(),
            grounds: "constitutional".into(),
            summary: "The gate decision overreached its declared bounds".into(),
            evidence_refs: "bafyrei-evidence-1".into(),
            filed_at: "2026-07-20T10:00:00Z".into(),
            reach: "community".into(),
            dht_anchor_hash: format!("uhCkk-anchor-{challenge_id}"),
            created_at: "2026-07-20T10:00:00Z".into(),
            updated_at: "2026-07-20T10:00:00Z".into(),
        }
    }

    fn outcome_row(app_id: &str, outcome_id: &str) -> ChallengeOutcomeRow {
        ChallengeOutcomeRow {
            app_id: app_id.into(),
            outcome_id: outcome_id.into(),
            challenge_cid: "bafyrei-challenge-1".into(),
            verdict: "upheld".into(),
            reviewer_consensus: "uhCAk-adam,uhCAk-jessica".into(),
            reasoning_json: r#"{"summary":"Bounds exceeded","steps":[]}"#.into(),
            decided_at: "2026-07-21T10:00:00Z".into(),
            indemnification_actions_json: "[]".into(),
            dht_anchor_hash: format!("uhCkk-anchor-{outcome_id}"),
            created_at: "2026-07-21T10:00:00Z".into(),
            updated_at: "2026-07-21T10:00:00Z".into(),
        }
    }

    /// The alpha shape: the signal DID land, but under the installed-hApp id, so
    /// the namespace-scoped challenge surface reads empty.
    #[test]
    fn refiles_challenge_written_under_the_installed_app_id() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let row = challenge_row(INSTALLED_APP_ID, "bafyrei-chal-1");
        gate_decision_challenges::upsert(&mut conn, &row).expect("mis-filed write");

        let ctx = AppContext::new(CANONICAL);
        assert!(
            gate_decision_challenges::find_by_id(&mut conn, &ctx.h_app_id, "bafyrei-chal-1")
                .expect("query")
                .is_none(),
            "mis-filed row must be invisible at the canonical ns"
        );

        let report = run_once(&mut conn, CANONICAL).expect("sweep");
        assert_eq!(report.challenges_examined, 1);
        assert_eq!(report.challenges_refiled, 1);
        assert_eq!(report.challenges_shadowed, 0);

        let healed =
            gate_decision_challenges::find_by_id(&mut conn, &ctx.h_app_id, "bafyrei-chal-1")
                .expect("query")
                .expect("row visible at canonical ns after heal");
        assert_eq!(healed.grounds, "constitutional");
        assert_eq!(
            healed.dht_anchor_hash, "uhCkk-anchor-bafyrei-chal-1",
            "provenance survives the re-file"
        );
        assert!(
            gate_decision_challenges::find_by_id(&mut conn, INSTALLED_APP_ID, "bafyrei-chal-1")
                .expect("query")
                .is_none(),
            "the row MOVED — it is not duplicated across namespaces"
        );
    }

    /// Same defect, sibling table.
    #[test]
    fn refiles_outcome_written_under_the_installed_app_id() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        challenge_outcomes::upsert(&mut conn, &outcome_row(INSTALLED_APP_ID, "bafyrei-out-1"))
            .expect("mis-filed write");

        let report = run_once(&mut conn, CANONICAL).expect("sweep");
        assert_eq!(report.outcomes_examined, 1);
        assert_eq!(report.outcomes_refiled, 1);

        let healed = challenge_outcomes::find_by_id(&mut conn, CANONICAL, "bafyrei-out-1")
            .expect("query")
            .expect("row visible at canonical ns after heal");
        assert_eq!(healed.verdict, "upheld");
        assert_eq!(healed.dht_anchor_hash, "uhCkk-anchor-bafyrei-out-1");
    }

    /// A row already at the canonical namespace is never a candidate.
    #[test]
    fn already_canonical_rows_are_not_candidates() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        gate_decision_challenges::upsert(&mut conn, &challenge_row(CANONICAL, "bafyrei-chal-2"))
            .expect("seed");
        challenge_outcomes::upsert(&mut conn, &outcome_row(CANONICAL, "bafyrei-out-2"))
            .expect("seed");

        let report = run_once(&mut conn, CANONICAL).expect("sweep");
        assert!(!report.saw_anything(), "coherent rows are not examined");
        assert!(!report.healed_anything());
    }

    /// Idempotence — the load-bearing property for a boot-time sweep.
    #[test]
    fn second_sweep_is_a_no_op() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        gate_decision_challenges::upsert(&mut conn, &challenge_row(INSTALLED_APP_ID, "chal-3"))
            .expect("seed");
        challenge_outcomes::upsert(&mut conn, &outcome_row(INSTALLED_APP_ID, "out-3"))
            .expect("seed");

        let first = run_once(&mut conn, CANONICAL).expect("first sweep");
        assert!(first.healed_anything());

        let second = run_once(&mut conn, CANONICAL).expect("second sweep");
        assert_eq!(
            second,
            GateChallengeNamespaceReport::default(),
            "re-run must observe and change nothing"
        );
    }

    /// An empty projection is a clean no-op (a fresh node's every boot).
    #[test]
    fn empty_projection_is_a_no_op() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let report = run_once(&mut conn, CANONICAL).expect("sweep");
        assert_eq!(report, GateChallengeNamespaceReport::default());
    }

    /// The composite-PK case `rea_commitments` cannot have: the same id is filed
    /// under BOTH namespaces. Re-filing would violate `(app_id, challenge_id)`,
    /// so the sweep leaves the dark twin alone rather than colliding or deleting.
    #[test]
    fn shadowed_duplicate_is_skipped_not_collided() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let canonical_twin = challenge_row(CANONICAL, "bafyrei-chal-4");
        gate_decision_challenges::upsert(&mut conn, &canonical_twin).expect("canonical seed");
        let mut dark_twin = challenge_row(INSTALLED_APP_ID, "bafyrei-chal-4");
        dark_twin.summary = "stale dark copy".into();
        gate_decision_challenges::upsert(&mut conn, &dark_twin).expect("mis-filed seed");

        let report = run_once(&mut conn, CANONICAL).expect("sweep must not error on collision");
        assert_eq!(report.challenges_examined, 1);
        assert_eq!(report.challenges_refiled, 0);
        assert_eq!(report.challenges_shadowed, 1);

        let served = gate_decision_challenges::find_by_id(&mut conn, CANONICAL, "bafyrei-chal-4")
            .expect("query")
            .expect("canonical row still serves readers");
        assert_eq!(
            served.summary, canonical_twin.summary,
            "the canonical row is untouched — no clobber by the dark twin"
        );
        assert!(
            gate_decision_challenges::find_by_id(&mut conn, INSTALLED_APP_ID, "bafyrei-chal-4")
                .expect("query")
                .is_some(),
            "non-destructive: the dark twin is skipped, never deleted"
        );
    }

    /// The same shadow case on the outcomes table.
    #[test]
    fn shadowed_outcome_duplicate_is_skipped() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        challenge_outcomes::upsert(&mut conn, &outcome_row(CANONICAL, "bafyrei-out-5"))
            .expect("canonical seed");
        challenge_outcomes::upsert(&mut conn, &outcome_row(INSTALLED_APP_ID, "bafyrei-out-5"))
            .expect("mis-filed seed");

        let report = run_once(&mut conn, CANONICAL).expect("sweep must not error on collision");
        assert_eq!(report.outcomes_examined, 1);
        assert_eq!(report.outcomes_refiled, 0);
        assert_eq!(report.outcomes_shadowed, 1);
    }

    /// A node that never had the defect, and one that has a mix — the sweep only
    /// touches the foreign rows and leaves the canonical ones exactly as filed.
    #[test]
    fn heals_only_the_foreign_rows_in_a_mixed_table() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        gate_decision_challenges::upsert(&mut conn, &challenge_row(CANONICAL, "chal-ok"))
            .expect("seed");
        gate_decision_challenges::upsert(&mut conn, &challenge_row(INSTALLED_APP_ID, "chal-dark"))
            .expect("seed");

        let report = run_once(&mut conn, CANONICAL).expect("sweep");
        assert_eq!(report.challenges_examined, 1, "only the foreign row");
        assert_eq!(report.challenges_refiled, 1);

        for id in ["chal-ok", "chal-dark"] {
            assert!(
                gate_decision_challenges::find_by_id(&mut conn, CANONICAL, id)
                    .expect("query")
                    .is_some(),
                "{id} must be visible at the canonical ns"
            );
        }
    }
}
