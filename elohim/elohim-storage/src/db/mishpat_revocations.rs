//! What this peer knows about the revocation of a mishpat commitment, keyed by
//! the commitment's `cid` (its entry hash). Two facts share a row.
//!
//! - `revoked_at`: a projection of a notarized `revokes-commitment` (Category
//!   A; the DHT is the source of truth). The signal handler writes it for every
//!   revocation it projects, whether or not the target row exists yet, and
//!   [`super::mishpat_commitments::upsert_with_anchor`] applies it, so a
//!   commitment that lands after its revocation lands revoked.
//! - the submission record (`issued_at`, `issue_outcome`, `signed_at`,
//!   `attempts`, `last_attempt_at`): this peer's own record that it submitted a
//!   revocation (local, operational). Both submitters, the provide reconciler's
//!   removed-pin arm and the HTTP un-pin route, claim through [`claim`] before
//!   the conductor is called, so one row is never submitted by both, and every
//!   attempt carries the `signed_at` fixed at the first claim, so every attempt
//!   is the same entry.
//!
//! An attempt whose outcome is unknown (`pending`: the process stopped between
//! the claim and the answer; `unconfirmed`: the call returned an error and may
//! have committed) is not dropped. [`claim`] takes it again once it is older
//! than [`RETRY_SETTLE`], up to [`MAX_ATTEMPTS`] attempts, and only while the
//! caller's evidence still holds and no revocation of the target has been
//! projected. Past that it stays open and [`stuck`] reports it.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use super::diesel_schema::mishpat_revocations::dsl as mr;
use super::models::current_timestamp;

/// The claim is recorded; the conductor call has not answered.
pub const ISSUE_PENDING: &str = "pending";
/// The conductor accepted the revocation.
pub const ISSUE_ISSUED: &str = "issued";
/// The conductor call returned an error. It may still have committed.
pub const ISSUE_UNCONFIRMED: &str = "unconfirmed";

/// How long an attempt with an unknown outcome is left before it is taken
/// again. Longer than any conductor call is allowed to run, so a live attempt
/// is never taken over; long enough that the projection of an attempt that did
/// commit has normally arrived and stops the retry.
pub const RETRY_SETTLE: chrono::Duration = chrono::Duration::minutes(15);

/// Attempts at most for one target. Each attempt that is not answered may be a
/// committed `create_commitment` (the same entry each time, but one more action
/// on the source chain), so the count is small; past it the record is reported
/// and left for the operator.
pub const MAX_ATTEMPTS: i32 = 3;

/// One submission record.
#[derive(Debug, Clone, PartialEq, Eq, Queryable)]
pub struct SubmissionRecord {
    pub target_cid: String,
    pub revoked_at: Option<String>,
    pub issued_at: Option<String>,
    pub issue_outcome: Option<String>,
    pub signed_at: Option<String>,
    pub attempts: i32,
    pub last_attempt_at: Option<String>,
}

/// What [`claim`] decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Claim {
    /// This caller holds the attempt: submit now, signed at `signed_at`.
    Submit { signed_at: String, attempt: i32 },
    /// Submit nothing: a revocation of the target is projected, accepted, in
    /// flight, or out of attempts.
    Held,
}

fn ensure_row(conn: &mut SqliteConnection, target_cid: &str, now: &str) -> QueryResult<usize> {
    diesel::insert_or_ignore_into(mr::mishpat_revocations)
        .values((
            mr::target_cid.eq(target_cid),
            mr::attempts.eq(0),
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

/// The record for `target_cid`, if any.
pub fn get(conn: &mut SqliteConnection, target_cid: &str) -> QueryResult<Option<SubmissionRecord>> {
    mr::mishpat_revocations
        .filter(mr::target_cid.eq(target_cid))
        .select((
            mr::target_cid,
            mr::revoked_at,
            mr::issued_at,
            mr::issue_outcome,
            mr::signed_at,
            mr::attempts,
            mr::last_attempt_at,
        ))
        .first(conn)
        .optional()
}

/// Take the next attempt at revoking `target_cid`, or learn that none is due.
///
/// The first claim fixes `signed_at` (the caller's evidence time); later
/// attempts reuse it, so every attempt submits the same entry. A target whose
/// revocation is projected, or whose submission was accepted, is never claimed.
/// An unknown outcome (`pending`, `unconfirmed`) is claimed again once its last
/// attempt is older than [`RETRY_SETTLE`] and fewer than [`MAX_ATTEMPTS`] were
/// made. Each transition is one conditional update, so two callers can never
/// both hold an attempt.
pub fn claim(
    conn: &mut SqliteConnection,
    target_cid: &str,
    signed_at: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> QueryResult<Claim> {
    let stamp = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    ensure_row(conn, target_cid, &stamp)?;

    // First attempt: nothing recorded yet.
    let first = diesel::update(
        mr::mishpat_revocations
            .filter(mr::target_cid.eq(target_cid))
            .filter(mr::revoked_at.is_null())
            .filter(mr::issued_at.is_null()),
    )
    .set((
        mr::issued_at.eq(&stamp),
        mr::issue_outcome.eq(ISSUE_PENDING),
        mr::signed_at.eq(signed_at),
        mr::attempts.eq(1),
        mr::last_attempt_at.eq(&stamp),
        mr::updated_at.eq(&stamp),
    ))
    .execute(conn)?;
    if first == 1 {
        return Ok(Claim::Submit {
            signed_at: signed_at.to_string(),
            attempt: 1,
        });
    }

    // A later attempt: the outcome is unknown and has had time to settle.
    let settled_before = (now - RETRY_SETTLE)
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let Some(record) = get(conn, target_cid)? else {
        return Ok(Claim::Held);
    };
    let retry = diesel::update(
        mr::mishpat_revocations
            .filter(mr::target_cid.eq(target_cid))
            .filter(mr::revoked_at.is_null())
            .filter(mr::issue_outcome.eq_any([ISSUE_PENDING, ISSUE_UNCONFIRMED]))
            .filter(mr::attempts.eq(record.attempts))
            .filter(mr::attempts.lt(MAX_ATTEMPTS))
            .filter(mr::last_attempt_at.le(&settled_before)),
    )
    .set((
        mr::issue_outcome.eq(ISSUE_PENDING),
        mr::attempts.eq(record.attempts + 1),
        mr::last_attempt_at.eq(&stamp),
        mr::updated_at.eq(&stamp),
    ))
    .execute(conn)?;
    match (retry, record.signed_at) {
        (1, Some(fixed)) => Ok(Claim::Submit {
            signed_at: fixed,
            attempt: record.attempts + 1,
        }),
        _ => Ok(Claim::Held),
    }
}

/// Record how attempt `attempt` ended (`issued` or `unconfirmed`). Ignored if a
/// later attempt has been taken meanwhile.
pub fn finish(
    conn: &mut SqliteConnection,
    target_cid: &str,
    attempt: i32,
    outcome: &str,
) -> QueryResult<usize> {
    let now = current_timestamp();
    diesel::update(
        mr::mishpat_revocations
            .filter(mr::target_cid.eq(target_cid))
            .filter(mr::attempts.eq(attempt)),
    )
    .set((mr::issue_outcome.eq(outcome), mr::updated_at.eq(&now)))
    .execute(conn)
}

/// Give back attempt `attempt`, which submitted NOTHING (its precondition
/// failed before the conductor was called). A first attempt leaves no record
/// at all, so the target can be claimed afresh, with fresh evidence, if it is
/// withdrawn again; a later attempt returns to `unconfirmed`, because an
/// earlier attempt's outcome is still unknown.
pub fn release(conn: &mut SqliteConnection, target_cid: &str, attempt: i32) -> QueryResult<usize> {
    let now = current_timestamp();
    let scoped = mr::mishpat_revocations
        .filter(mr::target_cid.eq(target_cid))
        .filter(mr::attempts.eq(attempt))
        .filter(mr::issue_outcome.eq(ISSUE_PENDING));
    if attempt <= 1 {
        diesel::update(scoped)
            .set((
                mr::issued_at.eq(None::<String>),
                mr::issue_outcome.eq(None::<String>),
                mr::signed_at.eq(None::<String>),
                mr::attempts.eq(0),
                mr::last_attempt_at.eq(None::<String>),
                mr::updated_at.eq(&now),
            ))
            .execute(conn)
    } else {
        diesel::update(scoped)
            .set((
                mr::issue_outcome.eq(ISSUE_UNCONFIRMED),
                mr::attempts.eq(attempt - 1),
                mr::updated_at.eq(&now),
            ))
            .execute(conn)
    }
}

/// Every cid with an open or accepted submission (anything but a free target),
/// for planners that should not re-plan them as fresh work. A planner still
/// offers retry-eligible targets to [`claim`] through [`retry_due`].
pub fn claimed_targets(
    conn: &mut SqliteConnection,
) -> QueryResult<std::collections::HashSet<String>> {
    Ok(mr::mishpat_revocations
        .filter(mr::issued_at.is_not_null())
        .select(mr::target_cid)
        .load::<String>(conn)?
        .into_iter()
        .collect())
}

/// Records whose outcome is unknown, no revocation is projected, and the last
/// attempt is older than [`RETRY_SETTLE`]: candidates for another attempt, or,
/// at [`MAX_ATTEMPTS`], for [`stuck`].
fn unsettled(
    conn: &mut SqliteConnection,
    now: chrono::DateTime<chrono::Utc>,
) -> QueryResult<Vec<SubmissionRecord>> {
    let settled_before = (now - RETRY_SETTLE)
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    mr::mishpat_revocations
        .filter(mr::revoked_at.is_null())
        .filter(mr::issue_outcome.eq_any([ISSUE_PENDING, ISSUE_UNCONFIRMED]))
        .filter(mr::last_attempt_at.le(&settled_before))
        .select((
            mr::target_cid,
            mr::revoked_at,
            mr::issued_at,
            mr::issue_outcome,
            mr::signed_at,
            mr::attempts,
            mr::last_attempt_at,
        ))
        .order(mr::target_cid.asc())
        .load(conn)
}

/// Unknown outcomes that may be attempted again now.
pub fn retry_due(
    conn: &mut SqliteConnection,
    now: chrono::DateTime<chrono::Utc>,
) -> QueryResult<Vec<SubmissionRecord>> {
    Ok(unsettled(conn, now)?
        .into_iter()
        .filter(|r| r.attempts < MAX_ATTEMPTS)
        .collect())
}

/// Unknown outcomes out of attempts: a revocation this peer may or may not
/// have made, with none projected. Reported every pass; never retried
/// automatically.
pub fn stuck(
    conn: &mut SqliteConnection,
    now: chrono::DateTime<chrono::Utc>,
) -> QueryResult<Vec<SubmissionRecord>> {
    Ok(unsettled(conn, now)?
        .into_iter()
        .filter(|r| r.attempts >= MAX_ATTEMPTS)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_pool;

    fn at(minutes: i64) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-10-29T12:00:00Z")
            .expect("time")
            .with_timezone(&chrono::Utc)
            + chrono::Duration::minutes(minutes)
    }

    #[test]
    fn one_attempt_is_held_at_a_time() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        assert_eq!(
            claim(&mut conn, "uhCEkA", "2026-10-29T00:00:00Z", at(0)).expect("first"),
            Claim::Submit {
                signed_at: "2026-10-29T00:00:00Z".to_string(),
                attempt: 1
            }
        );
        assert_eq!(
            claim(&mut conn, "uhCEkA", "2026-10-29T05:00:00Z", at(1)).expect("second"),
            Claim::Held,
            "an attempt in flight is not taken by another submitter"
        );
    }

    #[test]
    fn an_accepted_submission_is_never_made_again() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        claim(&mut conn, "uhCEkA", "s", at(0)).expect("claim");
        finish(&mut conn, "uhCEkA", 1, ISSUE_ISSUED).expect("finish");
        assert_eq!(
            claim(&mut conn, "uhCEkA", "s", at(60 * 24)).expect("later"),
            Claim::Held
        );
        assert!(retry_due(&mut conn, at(60 * 24)).expect("due").is_empty());
    }

    #[test]
    fn an_attempt_that_never_answered_is_retried_with_the_same_entry() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        // The process stops between the claim and the conductor's answer.
        claim(&mut conn, "uhCEkA", "2026-10-29T00:00:00Z", at(0)).expect("claim");
        assert_eq!(
            claim(&mut conn, "uhCEkA", "other", at(5)).expect("too soon"),
            Claim::Held
        );
        let due = retry_due(&mut conn, at(16)).expect("due");
        assert_eq!(due.len(), 1, "a pending attempt is not silently dropped");
        assert_eq!(
            claim(&mut conn, "uhCEkA", "other", at(16)).expect("retry"),
            Claim::Submit {
                signed_at: "2026-10-29T00:00:00Z".to_string(),
                attempt: 2
            },
            "the retry carries the signing time of the first claim"
        );
    }

    #[test]
    fn attempts_are_bounded_and_then_reported() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        claim(&mut conn, "uhCEkA", "s", at(0)).expect("1");
        finish(&mut conn, "uhCEkA", 1, ISSUE_UNCONFIRMED).expect("f1");
        claim(&mut conn, "uhCEkA", "s", at(20)).expect("2");
        finish(&mut conn, "uhCEkA", 2, ISSUE_UNCONFIRMED).expect("f2");
        assert!(matches!(
            claim(&mut conn, "uhCEkA", "s", at(40)).expect("3"),
            Claim::Submit { attempt: 3, .. }
        ));
        assert!(stuck(&mut conn, at(41)).expect("stuck").is_empty());
        assert_eq!(
            claim(&mut conn, "uhCEkA", "s", at(80)).expect("4"),
            Claim::Held
        );
        let stuck_now = stuck(&mut conn, at(80)).expect("stuck");
        assert_eq!(stuck_now.len(), 1);
        assert_eq!(stuck_now[0].attempts, MAX_ATTEMPTS);
    }

    #[test]
    fn a_projected_revocation_ends_retries_and_is_not_a_submission() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        claim(&mut conn, "uhCEkA", "s", at(0)).expect("claim");
        record_projected(&mut conn, "uhCEkA", "2026-10-29T00:00:00Z").expect("project");
        assert!(retry_due(&mut conn, at(30)).expect("due").is_empty());
        assert!(stuck(&mut conn, at(30)).expect("stuck").is_empty());
        assert_eq!(
            claim(&mut conn, "uhCEkA", "s", at(30)).expect("claim"),
            Claim::Held
        );

        record_projected(&mut conn, "uhCEkB", "2026-10-13T00:00:00Z").expect("project");
        record_projected(&mut conn, "uhCEkB", "2027-01-01T00:00:00Z").expect("replay");
        assert_eq!(
            projected_revoked_at(&mut conn, "uhCEkB")
                .expect("read")
                .as_deref(),
            Some("2026-10-13T00:00:00Z"),
            "the first revocation time stands"
        );
        assert!(!claimed_targets(&mut conn).expect("set").contains("uhCEkB"));
    }

    #[test]
    fn a_released_first_attempt_leaves_the_target_free() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        claim(&mut conn, "uhCEkA", "old-evidence", at(0)).expect("claim");
        release(&mut conn, "uhCEkA", 1).expect("release");
        assert!(!claimed_targets(&mut conn).expect("set").contains("uhCEkA"));
        assert_eq!(
            claim(&mut conn, "uhCEkA", "new-evidence", at(1)).expect("claim again"),
            Claim::Submit {
                signed_at: "new-evidence".to_string(),
                attempt: 1
            }
        );
    }

    #[test]
    fn a_released_retry_keeps_the_earlier_unknown_outcome_open() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        claim(&mut conn, "uhCEkA", "s", at(0)).expect("1");
        claim(&mut conn, "uhCEkA", "s", at(20)).expect("2");
        release(&mut conn, "uhCEkA", 2).expect("release");
        let record = get(&mut conn, "uhCEkA").expect("get").expect("row");
        assert_eq!(record.issue_outcome.as_deref(), Some(ISSUE_UNCONFIRMED));
        assert_eq!(record.attempts, 1);
    }
}
