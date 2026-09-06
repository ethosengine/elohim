//! `feedback_subscriptions` + `feedback_rotation_cursor` — durable discovery.
//!
//! Accountable-correction contract §3. Two member kinds, because acceptance
//! vouches link from the CORRECTION action, not from the content action: a peer
//! that only subscribed to content targets would discover the correction and
//! never its acceptance.
//!
//! The cursor here is a FAIRNESS position over the subscription set, **not** a
//! high-water mark over DHT history. Link enumeration is unordered and a late
//! link must still be picked up, so there is nothing to high-water.

use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel::sqlite::SqliteConnection;

use crate::db::diesel_schema::{feedback_rotation_cursor, feedback_subscriptions};

pub const KIND_CONTENT_TARGET: &str = "content-target";
pub const KIND_CORRECTION_ACTION: &str = "correction-action";

pub const SOURCE_STEWARD: &str = "steward";
pub const SOURCE_DISCOVERED: &str = "discovered";
pub const SOURCE_NOTIFIED: &str = "notified";

#[derive(Debug, Clone, PartialEq, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = feedback_subscriptions)]
pub struct SubscriptionRow {
    pub member_kind: String,
    pub member_key: String,
    pub origin_dna_hash: String,
    pub source: String,
    pub added_at: String,
    pub last_visited_at: Option<String>,
    pub visit_count: i32,
}

#[derive(Debug, Clone, PartialEq, Queryable, Selectable)]
#[diesel(table_name = feedback_rotation_cursor)]
pub struct CursorRow {
    pub id: i32,
    pub cursor_kind: Option<String>,
    pub cursor_key: Option<String>,
    pub updated_at: String,
}

/// Add a member if absent. Idempotent: re-notification of an already-known
/// correction action must not reset its visit accounting.
pub fn add_member(
    conn: &mut SqliteConnection,
    member_kind: &str,
    member_key: &str,
    origin_dna_hash: &str,
    source: &str,
    now: &str,
) -> Result<bool, DieselError> {
    use crate::db::diesel_schema::feedback_subscriptions::dsl as t;
    let inserted = diesel::insert_into(t::feedback_subscriptions)
        .values((
            t::member_kind.eq(member_kind),
            t::member_key.eq(member_key),
            t::origin_dna_hash.eq(origin_dna_hash),
            t::source.eq(source),
            t::added_at.eq(now),
            t::visit_count.eq(0),
        ))
        .on_conflict((t::member_kind, t::member_key))
        .do_nothing()
        .execute(conn)?;
    Ok(inserted == 1)
}

pub fn count(conn: &mut SqliteConnection) -> Result<i64, DieselError> {
    use crate::db::diesel_schema::feedback_subscriptions::dsl as t;
    t::feedback_subscriptions.count().get_result(conn)
}

pub fn read_cursor(conn: &mut SqliteConnection) -> Result<Option<CursorRow>, DieselError> {
    use crate::db::diesel_schema::feedback_rotation_cursor::dsl as t;
    t::feedback_rotation_cursor
        .filter(t::id.eq(1))
        .first::<CursorRow>(conn)
        .optional()
}

pub fn write_cursor(
    conn: &mut SqliteConnection,
    kind: Option<&str>,
    key: Option<&str>,
    now: &str,
) -> Result<(), DieselError> {
    use crate::db::diesel_schema::feedback_rotation_cursor::dsl as t;
    diesel::insert_into(t::feedback_rotation_cursor)
        .values((
            t::id.eq(1),
            t::cursor_kind.eq(kind.map(|s| s.to_string())),
            t::cursor_key.eq(key.map(|s| s.to_string())),
            t::updated_at.eq(now),
        ))
        .on_conflict(t::id)
        .do_update()
        .set((
            t::cursor_kind.eq(kind.map(|s| s.to_string())),
            t::cursor_key.eq(key.map(|s| s.to_string())),
            t::updated_at.eq(now),
        ))
        .execute(conn)?;
    Ok(())
}

/// The next `budget` members after the persisted cursor, WRAPPING to the start
/// of the set when the tail is reached.
///
/// This is the anti-starvation shape. `release_adoption/watch.rs` takes the
/// first N of an unsorted set every tick, so the ninth member of a nine-member
/// set is never visited; here the cursor advances past whatever was served, so
/// N members under a per-tick budget of B are each visited within ceil(N/B)
/// ticks. The cursor is advanced by the caller on FAILURE as well as success —
/// a member that keeps failing must not monopolise the rotation.
pub fn next_members(
    conn: &mut SqliteConnection,
    budget: i64,
) -> Result<Vec<SubscriptionRow>, DieselError> {
    use crate::db::diesel_schema::feedback_subscriptions::dsl as t;
    if budget <= 0 {
        return Ok(vec![]);
    }
    let cursor = read_cursor(conn)?;
    let (ck, cv) = match cursor {
        Some(c) => (c.cursor_kind, c.cursor_key),
        None => (None, None),
    };

    let mut out: Vec<SubscriptionRow> = match (ck.as_deref(), cv.as_deref()) {
        (Some(kind), Some(key)) => t::feedback_subscriptions
            // Strictly-after in the (kind, key) total order.
            .filter(
                t::member_kind.gt(kind.to_string()).or(t::member_kind
                    .eq(kind.to_string())
                    .and(t::member_key.gt(key.to_string()))),
            )
            .order_by((t::member_kind.asc(), t::member_key.asc()))
            .limit(budget)
            .load::<SubscriptionRow>(conn)?,
        _ => t::feedback_subscriptions
            .order_by((t::member_kind.asc(), t::member_key.asc()))
            .limit(budget)
            .load::<SubscriptionRow>(conn)?,
    };

    // WRAP: a short tail is topped up from the head of the set, so the budget is
    // spent on real members rather than on the fact that the cursor is late in
    // the ordering.
    if (out.len() as i64) < budget {
        let remaining = budget - out.len() as i64;
        let head = t::feedback_subscriptions
            .order_by((t::member_kind.asc(), t::member_key.asc()))
            .limit(remaining)
            .load::<SubscriptionRow>(conn)?;
        for row in head {
            if !out
                .iter()
                .any(|r| r.member_kind == row.member_kind && r.member_key == row.member_key)
            {
                out.push(row);
            }
        }
    }
    Ok(out)
}

/// Record a visit. Called whether the visit SUCCEEDED or FAILED — the whole
/// point of the rotation is that a failing member yields its turn.
pub fn mark_visited(
    conn: &mut SqliteConnection,
    member_kind: &str,
    member_key: &str,
    now: &str,
) -> Result<(), DieselError> {
    use crate::db::diesel_schema::feedback_subscriptions::dsl as t;
    diesel::update(
        t::feedback_subscriptions
            .filter(t::member_kind.eq(member_kind))
            .filter(t::member_key.eq(member_key)),
    )
    .set((
        t::last_visited_at.eq(now),
        t::visit_count.eq(t::visit_count + 1),
    ))
    .execute(conn)?;
    Ok(())
}
