//! `feedback_application` + `feedback_application_member` — the per-generation
//! record of which acts this peer has already folded into its standing view.
//!
//! Category C (operational): rebuilt from the DHT per accountable-correction
//! contract §7. Nothing here is authoritative.
//!
//! **Unit of application is the OPERATION GROUP, not the action.** A correction's
//! group key is its evidence action hash; an acceptance's group key is the
//! correction group it targets. Keying on the action would mean a
//! late-discovered same-operation act changes the representative and, with it, an
//! already-committed contribution. Keying on the group makes a second act a
//! MEMBER — the contribution is unchanged, and a peer with no access to the
//! origin outbox converges to the same aggregate.

use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel::sqlite::SqliteConnection;

use crate::db::diesel_schema::{feedback_application, feedback_application_member};

// ---------------------------------------------------------------------------
// Status vocabularies (strings on the wire so an older reader degrades, not
// panics — same discipline as the coordinator's fetch outcomes).
// ---------------------------------------------------------------------------

/// A dependency is unfetchable. Retryable. NEVER shown as accepted.
pub const STATUS_PENDING: &str = "pending";
/// The contribution is committed to this generation's aggregate.
pub const STATUS_APPLIED: &str = "applied";
/// A POSITIVE mismatch (wrong author, wrong type, request mismatch). Not a
/// network failure; not retried.
pub const STATUS_REJECTED: &str = "rejected";

pub const MEMBER_STATUS_MEMBER: &str = "member";
pub const MEMBER_STATUS_REJECTED: &str = "rejected";
pub const MEMBER_STATUS_PENDING: &str = "pending";

pub const MEMBER_ROLE_CORRECTION: &str = "correction";
pub const MEMBER_ROLE_ACCEPTANCE: &str = "acceptance";

#[derive(Debug, Clone, PartialEq, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = feedback_application)]
pub struct ApplicationRow {
    pub generation_id: i32,
    pub group_key: String,
    pub status: String,
    pub contribution: i32,
    pub subject_pubkey: Option<Vec<u8>>,
    pub max_included_at_micros: Option<i64>,
    pub accepted: i32,
    pub attempts: i32,
    pub next_retry_at: Option<String>,
    pub applied_at: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = feedback_application_member)]
pub struct MemberRow {
    pub generation_id: i32,
    pub origin_dna_hash: String,
    pub action_hash: String,
    pub group_key: String,
    pub member_status: String,
    pub member_role: String,
    pub author_pubkey: Option<Vec<u8>>,
    pub action_timestamp_micros: Option<i64>,
    pub last_error: Option<String>,
    pub discovered_at: String,
}

pub fn fetch_application(
    conn: &mut SqliteConnection,
    generation_id: i32,
    group_key: &str,
) -> Result<Option<ApplicationRow>, DieselError> {
    use crate::db::diesel_schema::feedback_application::dsl as t;
    t::feedback_application
        .filter(t::generation_id.eq(generation_id))
        .filter(t::group_key.eq(group_key))
        .first::<ApplicationRow>(conn)
        .optional()
}

pub fn upsert_application(
    conn: &mut SqliteConnection,
    row: &ApplicationRow,
) -> Result<(), DieselError> {
    use crate::db::diesel_schema::feedback_application::dsl as t;
    diesel::insert_into(t::feedback_application)
        .values(row)
        .on_conflict((t::generation_id, t::group_key))
        .do_update()
        .set(row)
        .execute(conn)?;
    Ok(())
}

pub fn fetch_member(
    conn: &mut SqliteConnection,
    generation_id: i32,
    origin_dna_hash: &str,
    action_hash: &str,
) -> Result<Option<MemberRow>, DieselError> {
    use crate::db::diesel_schema::feedback_application_member::dsl as t;
    t::feedback_application_member
        .filter(t::generation_id.eq(generation_id))
        .filter(t::origin_dna_hash.eq(origin_dna_hash))
        .filter(t::action_hash.eq(action_hash))
        .first::<MemberRow>(conn)
        .optional()
}

pub fn upsert_member(conn: &mut SqliteConnection, row: &MemberRow) -> Result<(), DieselError> {
    use crate::db::diesel_schema::feedback_application_member::dsl as t;
    diesel::insert_into(t::feedback_application_member)
        .values(row)
        .on_conflict((t::generation_id, t::origin_dna_hash, t::action_hash))
        .do_update()
        .set(row)
        .execute(conn)?;
    Ok(())
}

/// Every act recorded against one group, in a deterministic order.
///
/// Ordered by `(action_timestamp, action_hash)` — the SAME total order the
/// coordinator's version-DAG pick uses, so "the maximum included timestamp" and
/// "the representative member" are computed identically on every peer.
pub fn members_of_group(
    conn: &mut SqliteConnection,
    generation_id: i32,
    group_key: &str,
) -> Result<Vec<MemberRow>, DieselError> {
    use crate::db::diesel_schema::feedback_application_member::dsl as t;
    t::feedback_application_member
        .filter(t::generation_id.eq(generation_id))
        .filter(t::group_key.eq(group_key))
        .order_by((t::action_timestamp_micros.asc(), t::action_hash.asc()))
        .load::<MemberRow>(conn)
}

/// Groups eligible for service on this tick: pending, and whose backoff (if
/// any) has expired. `now_rfc3339` is passed in rather than read here so the
/// caller can test the boundary without moving the clock.
pub fn eligible_pending_groups(
    conn: &mut SqliteConnection,
    generation_id: i32,
    now_rfc3339: &str,
    limit: i64,
) -> Result<Vec<ApplicationRow>, DieselError> {
    use crate::db::diesel_schema::feedback_application::dsl as t;
    t::feedback_application
        .filter(t::generation_id.eq(generation_id))
        .filter(t::status.eq(STATUS_PENDING))
        .filter(
            t::next_retry_at
                .is_null()
                .or(t::next_retry_at.le(now_rfc3339.to_string())),
        )
        .order_by(t::group_key.asc())
        .limit(limit)
        .load::<ApplicationRow>(conn)
}

/// Clear one generation's application + member rows. Used by the in-place reset
/// arm of a rebuild; the fresh-generation arm never needs it.
pub fn clear_generation(
    conn: &mut SqliteConnection,
    generation_id: i32,
) -> Result<usize, DieselError> {
    use crate::db::diesel_schema::feedback_application::dsl as a;
    use crate::db::diesel_schema::feedback_application_member::dsl as m;
    diesel::delete(m::feedback_application_member.filter(m::generation_id.eq(generation_id)))
        .execute(conn)?;
    diesel::delete(a::feedback_application.filter(a::generation_id.eq(generation_id))).execute(conn)
}
