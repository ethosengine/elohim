//! Projector cursor — tracks how far the projector has advanced per (pillar, kind).
//!
//! The `projector_cursor` table is Category C operational: it stewards the
//! controller's local "how far have I projected" state. Fully rebuildable
//! from `epr_atoms` replay. No DHT anchor by design.

use chrono::Utc;
use diesel::prelude::*;
use thiserror::Error;

use crate::db::diesel_schema::projector_cursor;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors from cursor read/write operations.
#[derive(Debug, Error)]
pub enum CursorError {
    #[error("diesel error: {0}")]
    Diesel(#[from] diesel::result::Error),
}

// ---------------------------------------------------------------------------
// Diesel model
// ---------------------------------------------------------------------------

/// Diesel model for a row in `projector_cursor`.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = projector_cursor)]
pub struct ProjectorCursorRow {
    pub pillar: String,
    pub kind: String,
    /// CID of the last EPR atom projected for this (pillar, kind) pair.
    /// `None` when the projector has never run for this pair.
    pub last_epr_cid: Option<String>,
    /// ISO-8601 timestamp of the last projected atom's `issued_at`.
    /// `None` when the projector has never run for this pair.
    pub last_issued_at: Option<String>,
    /// ISO-8601 timestamp of when this cursor row was last updated.
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// Query helpers
// ---------------------------------------------------------------------------

/// Load the cursor for a (pillar, kind) pair.
///
/// Returns `None` if no cursor row exists yet (projector has never run for
/// this projection).
pub fn load_cursor(
    conn: &mut SqliteConnection,
    pillar: &str,
    kind: &str,
) -> Result<Option<ProjectorCursorRow>, CursorError> {
    let row = projector_cursor::table
        .find((pillar, kind))
        .first::<ProjectorCursorRow>(conn)
        .optional()?;
    Ok(row)
}

/// Advance the cursor to record that `cid` (with `issued_at`) has been projected.
///
/// Upserts the (pillar, kind) row. On conflict, updates `last_epr_cid`,
/// `last_issued_at`, and `updated_at`.
pub fn advance_cursor(
    conn: &mut SqliteConnection,
    pillar: &str,
    kind: &str,
    cid: &str,
    issued_at: &str,
) -> Result<(), CursorError> {
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    diesel::insert_into(projector_cursor::table)
        .values((
            projector_cursor::pillar.eq(pillar),
            projector_cursor::kind.eq(kind),
            projector_cursor::last_epr_cid.eq(cid),
            projector_cursor::last_issued_at.eq(issued_at),
            projector_cursor::updated_at.eq(&now),
        ))
        .on_conflict((projector_cursor::pillar, projector_cursor::kind))
        .do_update()
        .set((
            projector_cursor::last_epr_cid.eq(cid),
            projector_cursor::last_issued_at.eq(issued_at),
            projector_cursor::updated_at.eq(&now),
        ))
        .execute(conn)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_pool;

    #[test]
    fn load_cursor_returns_none_when_absent() {
        let pool = test_pool();
        let mut conn = pool.get().expect("connection");
        let cursor = load_cursor(&mut conn, "shefa", "EconomicEvent").unwrap();
        assert!(cursor.is_none(), "no cursor should exist yet");
    }

    #[test]
    fn advance_cursor_creates_and_updates_row() {
        let pool = test_pool();
        let mut conn = pool.get().expect("connection");

        // First advance — creates the row.
        advance_cursor(
            &mut conn,
            "shefa",
            "EconomicEvent",
            "cid-1",
            "2026-01-01T00:00:00Z",
        )
        .unwrap();
        let row = load_cursor(&mut conn, "shefa", "EconomicEvent")
            .unwrap()
            .expect("row must exist after first advance");
        assert_eq!(row.last_epr_cid.as_deref(), Some("cid-1"));

        // Second advance — updates the row.
        advance_cursor(
            &mut conn,
            "shefa",
            "EconomicEvent",
            "cid-2",
            "2026-01-02T00:00:00Z",
        )
        .unwrap();
        let row2 = load_cursor(&mut conn, "shefa", "EconomicEvent")
            .unwrap()
            .expect("row must still exist");
        assert_eq!(row2.last_epr_cid.as_deref(), Some("cid-2"));
    }
}
