//! What this peer knows about the revocation of a mishpat commitment, keyed by
//! the commitment's `cid` (entry hash). Two facts share a row:
//!
//! - `revoked_at` — a projection of a notarized `revokes-commitment` (Category
//!   A; source of truth is the DHT). The signal handler writes it for every
//!   revocation it projects, whether or not the target row exists yet, and
//!   [`super::mishpat_commitments::upsert_with_anchor`] reads it, so a replayed
//!   or late-arriving commitment can never read as live after its revocation.
//! - `issued_at` / `issue_outcome` — this peer's own record that it SUBMITTED a
//!   revocation (local, operational). The provide reconciler writes it before
//!   the conductor call and never submits for a cid that has one, which is what
//!   makes a revocation once-per-row across restarts, lost signals and a call
//!   that committed but returned an error.

use std::collections::HashSet;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use super::diesel_schema::mishpat_revocations::dsl as mr;
use super::models::current_timestamp;

/// The submission is recorded; the conductor call has not returned.
pub const ISSUE_PENDING: &str = "pending";
/// The conductor accepted the revocation.
pub const ISSUE_ISSUED: &str = "issued";
/// The conductor call returned an error. It may still have committed, so it is
/// not submitted again.
pub const ISSUE_UNCONFIRMED: &str = "unconfirmed";

fn ensure_row(conn: &mut SqliteConnection, target_cid: &str, now: &str) -> QueryResult<usize> {
    diesel::insert_or_ignore_into(mr::mishpat_revocations)
        .values((
            mr::target_cid.eq(target_cid),
            mr::created_at.eq(now),
            mr::updated_at.eq(now),
        ))
        .execute(conn)
}

/// Record a projected revocation of `target_cid`. The first timestamp stands.
pub fn record_projected(
    conn: &mut SqliteConnection,
    target_cid: &str,
    revoked_at: &str,
) -> QueryResult<usize> {
    let now = current_timestamp();
    ensure_row(conn, target_cid, &now)?;
    diesel::update(
        mr::mishpat_revocations
            .filter(mr::target_cid.eq(target_cid))
            .filter(mr::revoked_at.is_null()),
    )
    .set((mr::revoked_at.eq(revoked_at), mr::updated_at.eq(&now)))
    .execute(conn)
}

/// The projected revocation time of `target_cid`, if one has been seen.
pub fn projected_revoked_at(
    conn: &mut SqliteConnection,
    target_cid: &str,
) -> QueryResult<Option<String>> {
    Ok(mr::mishpat_revocations
        .filter(mr::target_cid.eq(target_cid))
        .select(mr::revoked_at)
        .first::<Option<String>>(conn)
        .optional()?
        .flatten())
}

/// Record that this peer is about to submit a revocation of `target_cid`.
///
/// Returns `true` when this call made the record, `false` when a submission was
/// already recorded (in any outcome): the caller must then submit nothing.
pub fn begin_issue(conn: &mut SqliteConnection, target_cid: &str) -> QueryResult<bool> {
    let now = current_timestamp();
    ensure_row(conn, target_cid, &now)?;
    let claimed = diesel::update(
        mr::mishpat_revocations
            .filter(mr::target_cid.eq(target_cid))
            .filter(mr::issued_at.is_null()),
    )
    .set((
        mr::issued_at.eq(&now),
        mr::issue_outcome.eq(ISSUE_PENDING),
        mr::updated_at.eq(&now),
    ))
    .execute(conn)?;
    Ok(claimed == 1)
}

/// Record how a submission begun with [`begin_issue`] ended.
pub fn finish_issue(
    conn: &mut SqliteConnection,
    target_cid: &str,
    outcome: &str,
) -> QueryResult<usize> {
    let now = current_timestamp();
    diesel::update(
        mr::mishpat_revocations
            .filter(mr::target_cid.eq(target_cid))
            .filter(mr::issued_at.is_not_null()),
    )
    .set((mr::issue_outcome.eq(outcome), mr::updated_at.eq(&now)))
    .execute(conn)
}

/// Every cid this peer has a recorded submission for, in any outcome.
pub fn issued_targets(conn: &mut SqliteConnection) -> QueryResult<HashSet<String>> {
    Ok(mr::mishpat_revocations
        .filter(mr::issued_at.is_not_null())
        .select(mr::target_cid)
        .load::<String>(conn)?
        .into_iter()
        .collect())
}

/// The recorded outcome of this peer's submission for `target_cid`, if any.
pub fn issue_outcome(conn: &mut SqliteConnection, target_cid: &str) -> QueryResult<Option<String>> {
    Ok(mr::mishpat_revocations
        .filter(mr::target_cid.eq(target_cid))
        .filter(mr::issued_at.is_not_null())
        .select(mr::issue_outcome)
        .first::<Option<String>>(conn)
        .optional()?
        .flatten())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_pool;

    #[test]
    fn a_submission_is_recorded_once() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        assert!(begin_issue(&mut conn, "uhCEkA").expect("first"));
        assert!(!begin_issue(&mut conn, "uhCEkA").expect("second"));
        assert_eq!(
            issue_outcome(&mut conn, "uhCEkA").expect("read").as_deref(),
            Some(ISSUE_PENDING)
        );
        finish_issue(&mut conn, "uhCEkA", ISSUE_UNCONFIRMED).expect("finish");
        assert!(
            !begin_issue(&mut conn, "uhCEkA").expect("third"),
            "an unconfirmed submission is not submitted again"
        );
        assert!(issued_targets(&mut conn).expect("set").contains("uhCEkA"));
    }

    #[test]
    fn a_projected_revocation_is_not_a_submission() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        record_projected(&mut conn, "uhCEkB", "2026-10-13T00:00:00Z").expect("project");
        record_projected(&mut conn, "uhCEkB", "2027-01-01T00:00:00Z").expect("replay");
        assert_eq!(
            projected_revoked_at(&mut conn, "uhCEkB")
                .expect("read")
                .as_deref(),
            Some("2026-10-13T00:00:00Z"),
            "the first revocation time stands"
        );
        assert!(issued_targets(&mut conn).expect("set").is_empty());
        assert!(begin_issue(&mut conn, "uhCEkB").expect("begin"));
        assert_eq!(
            projected_revoked_at(&mut conn, "uhCEkB")
                .expect("read")
                .as_deref(),
            Some("2026-10-13T00:00:00Z")
        );
    }
}
