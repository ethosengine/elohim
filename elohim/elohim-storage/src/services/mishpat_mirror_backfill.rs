//! Level-triggered heal for the mishpat→REA replication mirror.
//!
//! ## The gap it closes (the `h_app_id` pillar-namespace mismatch)
//!
//! `signals::handle_mishpat_signal` projects a notarized `replicates-*`
//! Commitment into TWO `rea_commitments` shapes — the per-commitment mirror
//! ([`crate::db::rea_commitments::mirror_replication_commitment`]) and the
//! `(provider, reach)` provide row
//! ([`crate::db::rea_commitments::record_provide_from_content_commitment`]).
//! Both writes stamp the `h_app_id` the subscriber was handed. On a deployed
//! node that argument was the conductor's **installed-hApp id**
//! (`HOLOCHAIN_APP_ID`, `"elohim"`), while EVERY reader of `rea_commitments`
//! scopes to the **projection namespace** `"lamad"` (the HTTP default in
//! `api::extract_app_context`, and `AppContext::default_lamad()` in-process).
//! The rows landed, correctly shaped and correctly anchored — and were invisible
//! to `GET /api/v1/commitments`, to the resilience snapshot's
//! `commitment_backed_collectives` join, and to every distribution view.
//!
//! The wiring fix (pass the projection namespace) is **edge-triggered**: it only
//! helps commitments notarized after the fix ships. A commitment already
//! notarized never re-signals, so its mis-filed mirror would stay dark forever.
//!
//! ## Why a repair, not an insert
//!
//! `rea_commitments.id` is the sole PRIMARY KEY — `h_app_id` is a scoping
//! column, not part of the key. So a mis-filed row is *also* an
//! insert-blocking duplicate: an `insert_or_ignore` under the correct namespace
//! silently no-ops against it. A back-fill that only inserted would report
//! success and heal nothing. This sweep is therefore a three-way reconcile per
//! derived id: **absent → insert**, **present-but-mis-filed → re-file**,
//! **present-and-correct → no-op**.
//!
//! ## Desired vs actual (P1)
//!
//! - **desired** = the live (`revoked_at IS NULL`) `replicates-*` rows of the
//!   notarized `mishpat_commitments` ledger, run through the same PURE
//!   descriptors the signal path uses (`replication_mirror_for` /
//!   `provide_projection_for`) — so the sweep and the signal can never disagree
//!   about row shape.
//! - **actual** = `rea_commitments` rows at those derived ids, read
//!   namespace-blind.
//!
//! Revoked commitments are skipped: their mirror is `state='cancelled'` (or
//! absent), and a cancelled promise invisible in the wrong namespace reads the
//! same as absent — re-filing it would surface a dead row for no gain.
//!
//! Category C (operational): idempotent and bounded by the notarized commitment
//! ledger, which is small. A second run is a pure no-op, so it is safe to run on
//! every boot.

use diesel::sqlite::SqliteConnection;

use crate::db::models::{MishpatCommitment, NewMishpatCommitment};
use crate::db::rea_commitments;
use crate::error::StorageError;
use crate::mishpat_projection::{provide_projection_for, replication_mirror_for};

/// Outcome counters for one sweep (returned for logging/tests).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MirrorBackfillReport {
    /// Live `replicates-*` commitments examined this sweep.
    pub candidates: usize,
    /// Per-commitment mirror rows that were genuinely missing and were inserted.
    pub mirrors_inserted: usize,
    /// Per-commitment mirror rows found under a foreign namespace and re-filed.
    pub mirrors_refiled: usize,
    /// `(provider, reach)` provide rows that were missing and were inserted.
    pub provides_inserted: usize,
    /// `(provider, reach)` provide rows found under a foreign namespace and
    /// re-filed.
    pub provides_refiled: usize,
}

impl MirrorBackfillReport {
    /// Whether the sweep changed anything (drives log level at the call site).
    pub fn healed_anything(&self) -> bool {
        self.mirrors_inserted > 0
            || self.mirrors_refiled > 0
            || self.provides_inserted > 0
            || self.provides_refiled > 0
    }
}

/// Re-shape a stored ledger row into the insertable shape the PURE projection
/// descriptors take.
///
/// The descriptors are written against `NewMishpatCommitment` because the signal
/// path holds one pre-write; this sweep holds a post-write `MishpatCommitment`.
/// Dropping `created_at`/`updated_at` is lossless for both descriptors — neither
/// reads them.
fn as_new_row(row: &MishpatCommitment) -> NewMishpatCommitment {
    NewMishpatCommitment {
        cid: row.cid.clone(),
        action: row.action.clone(),
        scope: row.scope.clone(),
        provider: row.provider.clone(),
        recipient: row.recipient.clone(),
        bounds_json: row.bounds_json.clone(),
        valid_from: row.valid_from.clone(),
        valid_until: row.valid_until.clone(),
        revoked_at: row.revoked_at.clone(),
        state: row.state.clone(),
        dht_anchor_hash: row.dht_anchor_hash.clone(),
    }
}

/// Reconcile one derived id into `canonical_h_app_id`.
///
/// Returns `(inserted, refiled)`. `insert` is a closure so the caller supplies
/// the shape-specific write (mirror vs provide) while this keeps the three-way
/// namespace decision in ONE place.
fn reconcile_one(
    conn: &mut SqliteConnection,
    id: &str,
    canonical_h_app_id: &str,
    insert: impl FnOnce(&mut SqliteConnection) -> Result<bool, StorageError>,
) -> Result<(bool, bool), StorageError> {
    match rea_commitments::h_app_id_of(conn, id)? {
        // Genuinely absent — the signal never landed (or predates the mirror).
        None => Ok((insert(conn)?, false)),
        // Present but mis-filed — re-file rather than insert; the PK collision
        // would have made an insert a silent no-op.
        Some(existing) if existing != canonical_h_app_id => {
            let n = rea_commitments::set_h_app_id(conn, id, canonical_h_app_id)?;
            Ok((false, n > 0))
        }
        // Present and correctly filed.
        Some(_) => Ok((false, false)),
    }
}

/// Run one reconcile sweep over the notarized `replicates-*` ledger.
///
/// `canonical_h_app_id` is the PROJECTION namespace every `rea_commitments`
/// reader scopes to (`AppContext::default_lamad().h_app_id`) — never the
/// conductor's installed-hApp id.
///
/// A per-row failure is logged and skipped, never fatal: one malformed ledger
/// row must not deny the heal to every other commitment (the same per-row
/// degradation discipline a resolver feeding a live table owes its callers).
pub fn run_once(
    conn: &mut SqliteConnection,
    canonical_h_app_id: &str,
) -> Result<MirrorBackfillReport, StorageError> {
    let ledger = crate::db::mishpat_commitments::list_all(conn)
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let mut report = MirrorBackfillReport::default();

    for row in ledger.iter().filter(|r| r.revoked_at.is_none()) {
        let new_row = as_new_row(row);

        // --- per-commitment mirror ---------------------------------------
        if let Some(mirror) = replication_mirror_for(&new_row) {
            report.candidates += 1;
            let cid = mirror.cid.clone();
            match reconcile_one(conn, &cid, canonical_h_app_id, |c| {
                rea_commitments::mirror_replication_commitment(c, canonical_h_app_id, &mirror)
            }) {
                Ok((inserted, refiled)) => {
                    report.mirrors_inserted += usize::from(inserted);
                    report.mirrors_refiled += usize::from(refiled);
                    if inserted || refiled {
                        tracing::info!(
                            cid = %cid,
                            action = %row.action,
                            inserted,
                            refiled,
                            "mishpat_mirror_backfill: replication mirror healed"
                        );
                    }
                }
                Err(e) => tracing::warn!(
                    error = %e,
                    cid = %cid,
                    "mishpat_mirror_backfill: mirror reconcile failed (skipped; retries next boot)"
                ),
            }
        }

        // --- (provider, reach) provide row --------------------------------
        if let Some(provide) = provide_projection_for(&new_row) {
            let id = rea_commitments::provide_projection_id(&provide.provider, &provide.reach);
            let anchor = provide.dht_anchor_hash.clone();
            match reconcile_one(conn, &id, canonical_h_app_id, |c| {
                rea_commitments::record_provide_from_content_commitment(
                    c,
                    canonical_h_app_id,
                    &provide.provider,
                    &provide.reach,
                    anchor.as_deref(),
                )
            }) {
                Ok((inserted, refiled)) => {
                    report.provides_inserted += usize::from(inserted);
                    report.provides_refiled += usize::from(refiled);
                    if inserted || refiled {
                        tracing::info!(
                            id = %id,
                            inserted,
                            refiled,
                            "mishpat_mirror_backfill: provide row healed"
                        );
                    }
                }
                Err(e) => tracing::warn!(
                    error = %e,
                    id = %id,
                    "mishpat_mirror_backfill: provide reconcile failed (skipped; retries next boot)"
                ),
            }
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::NewMishpatCommitment;
    use crate::db::AppContext;
    use crate::test_util::test_pool;

    const CANONICAL: &str = "lamad";
    /// The conductor's installed-hApp id — what the subscriber was WRONGLY
    /// handed, and what the mis-filed production rows carry.
    const INSTALLED_APP_ID: &str = "elohim";

    fn content_commitment(cid: &str, provider: &str, head_ref: &str) -> NewMishpatCommitment {
        NewMishpatCommitment {
            cid: cid.to_string(),
            action: "replicates-commons".to_string(),
            scope: "replicates-commons".to_string(),
            provider: provider.to_string(),
            recipient: head_ref.to_string(),
            bounds_json:
                r#"{"reach":"commons","reach_ceiling":"commons","epr_scope":["epr:album-1"]}"#
                    .to_string(),
            valid_from: "2026-07-26T00:00:00Z".to_string(),
            valid_until: "2026-10-26T00:00:00Z".to_string(),
            revoked_at: None,
            state: "active".to_string(),
            dht_anchor_hash: Some(format!("uhCkk-anchor-{cid}")),
        }
    }

    fn seed_ledger(pool: &crate::db::DbPool, row: NewMishpatCommitment) {
        let mut conn = pool.get().expect("conn");
        crate::db::mishpat_commitments::upsert_with_anchor(&mut conn, row).expect("seed ledger");
    }

    /// The alpha shape: the signal DID land, but under the installed-hApp id, so
    /// the namespace-scoped list reads empty. The sweep must RE-FILE (an insert
    /// would no-op against the PK).
    #[test]
    fn refiles_mirror_written_under_the_installed_app_id() {
        let pool = test_pool();
        let row = content_commitment("uhCEk-mirror-1", "uhCAk-matthew", "epr:album-1");
        seed_ledger(&pool, row.clone());

        let mut conn = pool.get().expect("conn");
        // Reproduce the production write: correct shape, wrong namespace.
        let mirror = replication_mirror_for(&row).expect("replicates-commons mirrors");
        rea_commitments::mirror_replication_commitment(&mut conn, INSTALLED_APP_ID, &mirror)
            .expect("mis-filed write");

        let ctx = AppContext::new(CANONICAL);
        let before = rea_commitments::get_commitment(&mut conn, &ctx, "uhCEk-mirror-1")
            .expect("query")
            .is_some();
        assert!(
            !before,
            "mis-filed row must be invisible at the canonical ns"
        );

        let report = run_once(&mut conn, CANONICAL).expect("sweep");
        assert_eq!(report.mirrors_refiled, 1, "re-filed, not inserted");
        assert_eq!(report.mirrors_inserted, 0);

        let after = rea_commitments::get_commitment(&mut conn, &ctx, "uhCEk-mirror-1")
            .expect("query")
            .expect("row visible at canonical ns after heal");
        assert_eq!(after.action, "replicates-commons");
        assert_eq!(after.state, "active");
        assert_eq!(
            after.dht_anchor_hash.as_deref(),
            Some("uhCkk-anchor-uhCEk-mirror-1"),
            "provenance survives the re-file"
        );
    }

    /// A commitment whose signal predates the mirror entirely — nothing at the
    /// derived id, so the sweep INSERTS.
    #[test]
    fn inserts_mirror_that_was_never_projected() {
        let pool = test_pool();
        seed_ledger(
            &pool,
            content_commitment("uhCEk-mirror-2", "uhCAk-jessica", "epr:album-2"),
        );

        let mut conn = pool.get().expect("conn");
        let report = run_once(&mut conn, CANONICAL).expect("sweep");
        assert_eq!(report.mirrors_inserted, 1);
        assert_eq!(report.mirrors_refiled, 0);

        let ctx = AppContext::new(CANONICAL);
        assert!(
            rea_commitments::get_commitment(&mut conn, &ctx, "uhCEk-mirror-2")
                .expect("query")
                .is_some()
        );
    }

    /// Idempotence — the load-bearing property for a boot-time sweep.
    #[test]
    fn second_sweep_is_a_no_op() {
        let pool = test_pool();
        seed_ledger(
            &pool,
            content_commitment("uhCEk-mirror-3", "uhCAk-james", "epr:album-3"),
        );

        let mut conn = pool.get().expect("conn");
        let first = run_once(&mut conn, CANONICAL).expect("first sweep");
        assert!(first.healed_anything());

        let second = run_once(&mut conn, CANONICAL).expect("second sweep");
        assert!(!second.healed_anything(), "re-run must change nothing");
        assert_eq!(second.candidates, 1, "still observed, just already correct");
    }

    /// The content variant also mints the `(provider, reach)` provide row the
    /// resilience snapshot joins on — healed by the same three-way reconcile.
    #[test]
    fn heals_the_provide_row_alongside_the_mirror() {
        let pool = test_pool();
        seed_ledger(
            &pool,
            content_commitment("uhCEk-mirror-4", "uhCAk-adam", "epr:album-4"),
        );

        let mut conn = pool.get().expect("conn");
        let report = run_once(&mut conn, CANONICAL).expect("sweep");
        assert_eq!(report.provides_inserted, 1);

        let ctx = AppContext::new(CANONICAL);
        let id = rea_commitments::provide_projection_id("uhCAk-adam", "commons");
        let row = rea_commitments::get_commitment(&mut conn, &ctx, &id)
            .expect("query")
            .expect("provide row visible at canonical ns");
        assert_eq!(row.action, "provide");
        assert!(row.has_classification("content:commons"));
    }

    /// A revoked commitment is NOT resurrected: an invisible cancelled promise
    /// and an absent one read the same, and re-filing would surface a dead row.
    #[test]
    fn skips_revoked_commitments() {
        let pool = test_pool();
        let mut row = content_commitment("uhCEk-mirror-5", "uhCAk-eve", "epr:album-5");
        row.revoked_at = Some("2026-07-26T12:00:00Z".to_string());
        seed_ledger(&pool, row);

        let mut conn = pool.get().expect("conn");
        let report = run_once(&mut conn, CANONICAL).expect("sweep");
        assert_eq!(report.candidates, 0, "revoked rows are not candidates");
        assert!(!report.healed_anything());
    }

    /// Non-replication actions are never mirrored — the sweep must not widen
    /// what `MIRRORED_REPLICATION_ACTIONS` admits.
    #[test]
    fn ignores_non_replication_actions() {
        let pool = test_pool();
        let mut row = content_commitment("uhCEk-compute-1", "uhCAk-matthew", "uhCAk-deploy");
        row.action = "delegates-compute".to_string();
        row.scope = "republish-epr".to_string();
        seed_ledger(&pool, row);

        let mut conn = pool.get().expect("conn");
        let report = run_once(&mut conn, CANONICAL).expect("sweep");
        assert_eq!(report.candidates, 0);
        assert!(!report.healed_anything());
    }
}
