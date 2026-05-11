//! ## Source of Truth
//!
//! Operational (Category C) append-only log derived from the DHT-notarized
//! EconomicEvent entry (action='ack-projection'). Rebuildable from any
//! peer's content_store via the rea_projection signal stream. No SQLite
//! row here is authoritative.

use crate::db::diesel_schema::projection_events;
use diesel::prelude::*;

/// Row model for projection_events table (SELECT — excludes auto-increment id).
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = projection_events)]
pub struct ProjectionEventRow {
    pub id: i32,
    pub blob_hash: String,
    pub projector_agent_cid: String,
    pub emitted_at: String,
    pub source_action_hash: String,
}

/// Insert model for projection_events (INSERT — no id field).
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = projection_events)]
pub struct NewProjectionEvent {
    pub blob_hash: String,
    pub projector_agent_cid: String,
    pub emitted_at: String,
    pub source_action_hash: String,
}

/// Append a projection ack event to the log.
///
/// Uses ON CONFLICT DO NOTHING to deduplicate by `source_action_hash`
/// (the DHT action hash that anchors this ack). Idempotent — re-delivering
/// the same DHT signal will not produce duplicate rows.
pub fn append_projection_event(
    conn: &mut SqliteConnection,
    row: &NewProjectionEvent,
) -> diesel::result::QueryResult<()> {
    diesel::insert_into(projection_events::table)
        .values(row)
        .on_conflict(projection_events::source_action_hash)
        .do_nothing()
        .execute(conn)?;
    Ok(())
}

/// List the most recent ack events for a blob, newest first, bounded by `limit`.
pub fn list_recent_for_blob(
    conn: &mut SqliteConnection,
    blob_hash_arg: &str,
    limit: i64,
) -> diesel::result::QueryResult<Vec<ProjectionEventRow>> {
    use crate::db::diesel_schema::projection_events::dsl as pe;
    pe::projection_events
        .filter(pe::blob_hash.eq(blob_hash_arg))
        .order(pe::emitted_at.desc())
        .limit(limit)
        .load(conn)
}

/// Return distinct projector_agent_cid values for a blob — the set of doorways
/// that have acked at least one projection of this CID.
pub fn distinct_projectors_for_blob(
    conn: &mut SqliteConnection,
    blob_hash_arg: &str,
) -> diesel::result::QueryResult<Vec<String>> {
    use crate::db::diesel_schema::projection_events::dsl as pe;
    pe::projection_events
        .filter(pe::blob_hash.eq(blob_hash_arg))
        .select(pe::projector_agent_cid)
        .distinct()
        .load(conn)
}
