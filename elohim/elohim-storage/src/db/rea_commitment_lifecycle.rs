//! Atomic application of a prepared, authenticated REA lifecycle observation.
use diesel::prelude::*;

use super::{models::ReaCommitment, rea_commitments, AppContext};
use crate::error::StorageError;

/// Compare every existing field the projection can overwrite. Local graduation
/// can change state without changing the anchor or a timestamp.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Snapshot {
    pub anchor: Option<String>,
    state: String,
    finished: i32,
    scope: Option<String>,
    note: Option<String>,
    metadata: Option<String>,
}

impl From<&ReaCommitment> for Snapshot {
    fn from(row: &ReaCommitment) -> Self {
        Self {
            anchor: row.dht_anchor_hash.clone(),
            state: row.state.clone(),
            finished: row.finished,
            scope: row.in_scope_of.clone(),
            note: row.note.clone(),
            metadata: row.metadata_json.clone(),
        }
    }
}

pub(crate) fn snapshot(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Option<Snapshot>, StorageError> {
    Ok(rea_commitments::get_commitment(conn, ctx, id)?
        .as_ref()
        .map(Snapshot::from))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ApplyOutcome {
    Advanced,
    Unchanged,
    Deferred,
    /// REFUSED: the observation would have walked this row's standing BACKWARDS
    /// through [`crate::db::models::commitment_lifecycle_order`] — a settled
    /// obligation back into a live one. The row is left exactly as it stands.
    ///
    /// The live shape (household, 2026-09-13, a2o `A doorway never forwards a
    /// forwarded request`): a `project-epr` hosting contract is cancelled
    /// through the doorway's admin path, the local row settles to `cancelled`,
    /// alpha's own dispatch correctly 404s — and ~40s later the row is `active`
    /// again, the `ProjectionSignal` re-installs the route, and the visitor gets
    /// a 200 from a contract nobody holds.
    RefusedConductorBehind,
}

/// Would writing `answered_state` REGRESS this row — move it BACKWARDS through
/// [`crate::db::models::commitment_lifecycle_order`], a settled obligation back
/// into a live one?
///
/// STRICT: an equal rank is not a regression. That is what keeps the ANCHOR axis
/// working — a conductor re-answering `active` over a local `active` under a NEW
/// action is a real authority advance and must still be written.
///
/// NARROW on unknown vocabulary. When either side is outside the order this
/// returns `false` (write, as before): the guard fires only where it can PROVE
/// backwards motion. This is the opposite conservatism from
/// [`crate::p2p::projection_reconcile::compare_rea_states`], and deliberately so
/// — there, an unknown string must not be read as agreement (so it counts as a
/// gap); here, an unknown string must not be read as evidence that the answer is
/// stale (so it does not freeze a row heal is supposed to fill). Both rules
/// refuse to infer from ignorance; they just have opposite safe sides.
///
/// ## Why this is needed even with a lineage-ordered observation
///
/// [`crate::services::rea_commitment_record::observe`] already refuses an action
/// that is not strictly newer on the SAME verified root and author, which is a
/// stronger ordering than any string rank. But it is an ordering over ACTIONS,
/// and the projection's standing can move without the anchor moving (local
/// graduation, the event/policy writes `apply`'s same-anchor branch protects).
/// This guard is over the STATE the write would land, at the one place every
/// projection path — signal, HTTP create, HTTP state-update, refresh and the
/// reconciler's heal leg — funnels through. 493adaabb put it in front of the
/// heal leg's upsert; it belongs at the write, where it governs all five.
pub(crate) fn would_regress_state(local_state: &str, answered_state: &str) -> bool {
    use super::models::commitment_lifecycle_order::rank;
    match (rank(local_state), rank(answered_state)) {
        (Some(local), Some(answered)) => answered < local,
        _ => false,
    }
}

/// No conductor I/O occurs inside this transaction. Recheck the entire captured
/// projection snapshot under SQLite's write lock before changing any field.
pub(crate) fn apply(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    expected: Option<&Snapshot>,
    input: rea_commitments::CreateReaCommitmentInput,
    anchor: &str,
    state: &str,
    finished: bool,
) -> Result<ApplyOutcome, StorageError> {
    let id = input
        .id
        .clone()
        .ok_or_else(|| StorageError::InvalidInput("lifecycle projection requires id".into()))?;
    conn.immediate_transaction(|conn| {
        let current = snapshot(conn, ctx, &id)?;
        if current.as_ref() != expected {
            return Ok(ApplyOutcome::Deferred);
        }
        // Never overwrite local graduation/event policy on same-anchor replay.
        if current.as_ref().and_then(|s| s.anchor.as_deref()) == Some(anchor) {
            return Ok(ApplyOutcome::Unchanged);
        }
        // A projection may ADVANCE this row or move its anchor; it may never
        // walk the row's standing backwards. The row is left ENTIRELY untouched
        // — the anchor half of a behind answer is behind too, and writing it
        // would relabel the row onto a superseded action.
        if current
            .as_ref()
            .is_some_and(|s| would_regress_state(&s.state, state))
        {
            return Ok(ApplyOutcome::RefusedConductorBehind);
        }
        rea_commitments::upsert_with_anchor(conn, ctx, input, Some(anchor))?;
        use super::diesel_schema::rea_commitments::dsl as c;
        diesel::update(
            c::rea_commitments
                .filter(c::h_app_id.eq(&ctx.h_app_id))
                .filter(c::id.eq(&id)),
        )
        .set((
            c::dht_anchor_hash.eq(anchor),
            c::state.eq(state),
            c::finished.eq(i32::from(finished)),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Database(format!("lifecycle CAS: {e}")))?;
        Ok(ApplyOutcome::Advanced)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DbPool;

    /// A `project-epr` hosting commitment as the own conductor answers it.
    fn wire(id: &str, state: &str) -> shefa_types::Commitment {
        shefa_types::Commitment {
            id: id.to_string(),
            action: "project-epr".to_string(),
            provider: "doorway:alpha-elohim-host".to_string(),
            receiver: format!("epr:{id}"),
            resource_conforms_to: None,
            resource_inventoried_as: None,
            resource_classified_as_json: "[]".to_string(),
            resource_quantity_value: None,
            resource_quantity_unit: None,
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_point_in_time: None,
            has_beginning: None,
            has_end: None,
            due: None,
            clause_of: None,
            agreed_in: None,
            input_of: None,
            output_of: None,
            satisfies: None,
            in_scope_of_json: format!("[\"doorway:alpha-elohim-host|epr:{id}\"]"),
            finished: false,
            state: state.to_string(),
            note: None,
            metadata_json: "{}".to_string(),
            created_at: "2026-09-13T05:00:00Z".to_string(),
            updated_at: "2026-09-13T05:00:00Z".to_string(),
        }
    }

    /// One projection attempt through the PRODUCTION write, with `expected`
    /// read from the row as every caller reads it.
    fn project(
        pool: &DbPool,
        ctx: &AppContext,
        id: &str,
        state: &str,
        anchor: &str,
    ) -> ApplyOutcome {
        let mut conn = pool.get().expect("pool conn");
        let expected = snapshot(&mut conn, ctx, id).expect("snapshot");
        let c = wire(id, state);
        let input = crate::rea_projection::project_typed_commitment(&c);
        apply(
            &mut conn,
            ctx,
            expected.as_ref(),
            input,
            anchor,
            &c.state,
            c.finished,
        )
        .expect("apply succeeds")
    }

    fn row(pool: &DbPool, ctx: &AppContext, id: &str) -> crate::db::models::ReaCommitment {
        let mut conn = pool.get().expect("pool conn");
        rea_commitments::get_commitment(&mut conn, ctx, id)
            .expect("row query succeeds")
            .expect("row exists")
    }

    #[test]
    fn the_regression_guard_fires_only_on_proven_backwards_motion() {
        // The live shape: a cancelled hosting contract against a conductor that
        // still answers `active`.
        assert!(
            would_regress_state("cancelled", "active"),
            "settled -> live is the regression this guard exists for"
        );
        assert!(
            would_regress_state("active", "proposed"),
            "live -> birth is backwards too"
        );

        // Forward and level motion must still write. Level is the load-bearing
        // one: it is what keeps the ANCHOR axis working when both sides agree on
        // standing.
        assert!(
            !would_regress_state("proposed", "active"),
            "the whole point of the state heal — a frozen row learning the author's standing"
        );
        assert!(
            !would_regress_state("active", "active"),
            "an equal rank is not a regression; the anchor half of this answer may still be news"
        );
        assert!(
            !would_regress_state("cancelled", "revoked"),
            "two settled states are level, not backwards"
        );

        // Unknown vocabulary: NARROW. The guard fires only where it can PROVE
        // backwards motion, so a state the substrate has grown past never
        // freezes a row a projection is supposed to fill.
        assert!(
            !would_regress_state("cancelled", "some-future-state"),
            "an unordered answer is not evidence the writer is stale"
        );
        assert!(
            !would_regress_state("some-future-state", "active"),
            "an unordered local row is not evidence it is ahead"
        );
    }

    #[test]
    fn an_absent_row_is_filled_not_refused() {
        let pool = crate::test_util::test_pool();
        let ctx = AppContext::default_lamad();
        assert_eq!(
            project(
                &pool,
                &ctx,
                "project-epr-nrt-seed",
                "cancelled",
                "uhCkA-head"
            ),
            ApplyOutcome::Advanced,
            "an absent row is FILLED — no guard can fire against a row that is not there"
        );
    }

    #[test]
    fn a_cancelled_contract_is_not_resurrected_by_a_behind_answer() {
        // The household red, exactly (a2o `A doorway never forwards a forwarded
        // request`, 2026-09-13 05:17–05:18Z): the contract is cancelled through
        // the doorway's admin path, alpha's own dispatch correctly 404s, a peer
        // still holding the pre-cancel entry advertises the OLD anchor, the gap
        // is admitted on the ANCHOR axis, and the own conductor answers with the
        // pre-cancel entry. Writing that answer walked `cancelled` back to
        // `active` and re-installed the route.
        let pool = crate::test_util::test_pool();
        let ctx = AppContext::default_lamad();
        let id = "project-epr-nrt-cancelled";
        assert_eq!(
            project(&pool, &ctx, id, "cancelled", "uhCkA-cancel-head"),
            ApplyOutcome::Advanced
        );

        assert_eq!(
            project(&pool, &ctx, id, "active", "uhCkA-precancel-root"),
            ApplyOutcome::RefusedConductorBehind,
            "an answer from behind a settled row must be refused"
        );

        let r = row(&pool, &ctx, id);
        assert_eq!(
            r.state, "cancelled",
            "the cancelled standing must survive the write"
        );
        assert_eq!(
            r.dht_anchor_hash.as_deref(),
            Some("uhCkA-cancel-head"),
            "the row is left ENTIRELY untouched — the behind answer's anchor is behind too"
        );
    }

    #[test]
    fn a_row_that_is_genuinely_behind_still_advances() {
        // The honest half, and the reason the guard is STRICT: a projection that
        // froze at `proposed` while the author graduated the commitment must
        // still learn the author's standing.
        let pool = crate::test_util::test_pool();
        let ctx = AppContext::default_lamad();
        let id = "project-epr-nrt-behind";
        assert_eq!(
            project(&pool, &ctx, id, "proposed", "uhCkA-root"),
            ApplyOutcome::Advanced
        );
        assert_eq!(
            project(&pool, &ctx, id, "active", "uhCkA-activated"),
            ApplyOutcome::Advanced,
            "a forward answer is written"
        );

        let r = row(&pool, &ctx, id);
        assert_eq!(r.state, "active", "standing advanced");
        assert_eq!(
            r.dht_anchor_hash.as_deref(),
            Some("uhCkA-activated"),
            "anchor advanced with it"
        );
    }

    #[test]
    fn an_answer_that_agrees_on_standing_still_moves_the_anchor() {
        // The case the guard must NOT catch: the two sides hold the same
        // standing and a different anchor. That is a real authority advance —
        // the axis the discovery arm admits such a row on — and refusing it
        // would freeze the very rows the reconciler exists to converge.
        let pool = crate::test_util::test_pool();
        let ctx = AppContext::default_lamad();
        let id = "project-epr-nrt-level";
        assert_eq!(
            project(&pool, &ctx, id, "active", "uhCkA-old"),
            ApplyOutcome::Advanced
        );
        assert_eq!(
            project(&pool, &ctx, id, "active", "uhCkA-new"),
            ApplyOutcome::Advanced,
            "equal standing does not block an anchor advance"
        );

        let r = row(&pool, &ctx, id);
        assert_eq!(
            r.dht_anchor_hash.as_deref(),
            Some("uhCkA-new"),
            "anchor moved"
        );
        assert_eq!(r.state, "active", "and the standing was left where it was");
    }

    #[test]
    fn a_same_anchor_replay_leaves_local_graduation_alone() {
        let pool = crate::test_util::test_pool();
        let ctx = AppContext::default_lamad();
        let id = "project-epr-nrt-replay";
        assert_eq!(
            project(&pool, &ctx, id, "active", "uhCkA-same"),
            ApplyOutcome::Advanced
        );
        assert_eq!(
            project(&pool, &ctx, id, "active", "uhCkA-same"),
            ApplyOutcome::Unchanged,
            "the same anchor twice is not news"
        );
    }

    #[test]
    fn a_snapshot_that_moved_under_the_caller_defers() {
        let pool = crate::test_util::test_pool();
        let ctx = AppContext::default_lamad();
        let id = "project-epr-nrt-race";
        assert_eq!(
            project(&pool, &ctx, id, "proposed", "uhCkA-root"),
            ApplyOutcome::Advanced
        );

        // A caller that prepared against an ABSENT row, while another writer
        // filled it, must not clobber the winner.
        let mut conn = pool.get().expect("pool conn");
        let c = wire(id, "active");
        let outcome = apply(
            &mut conn,
            &ctx,
            None,
            crate::rea_projection::project_typed_commitment(&c),
            "uhCkA-later",
            &c.state,
            c.finished,
        )
        .expect("apply succeeds");
        assert_eq!(outcome, ApplyOutcome::Deferred);
    }
}
