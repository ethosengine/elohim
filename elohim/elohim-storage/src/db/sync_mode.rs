//! Sync-mode operational state + bounded transition history (Category C).
//!
//! Source of truth: local SQLite — node-local bandwidth-posture control for
//! the unified `p2p::sync_gate::SyncGate`. No peer validates this; on loss
//! the gate reconstructs to `mode=sync`, `network=unknown` (pre-feature
//! behavior). History is an operator-facing audit log, capped at
//! [`TRANSITION_HISTORY_CAP`] rows (oldest pruned on insert).
//!
//! Design: `genesis/data/timeline/backlog/p2p-sync-mode-unified-gate-design.md`.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use super::diesel_schema::sync_mode_state::dsl as state;
use super::diesel_schema::sync_mode_transitions::dsl as transitions;
use super::models::current_timestamp;

/// Bounded retention for `sync_mode_transitions` (design: cap ~500, prune oldest).
pub const TRANSITION_HISTORY_CAP: i64 = 500;

/// The singleton operational state row.
#[derive(Debug, Clone, Queryable)]
pub struct SyncModeState {
    pub id: i32,
    pub mode: String,
    pub network_class: String,
    pub updated_at: String,
}

/// One transition history row.
#[derive(Debug, Clone, Queryable)]
pub struct SyncModeTransition {
    pub id: i32,
    pub at: String,
    pub from_mode: String,
    pub to_mode: String,
    pub source: String,
    pub reason: String,
}

/// Load the persisted (mode, network_class), if any. `None` means the node
/// has never persisted a preference — callers fall back to the gate defaults.
pub fn load_state(conn: &mut SqliteConnection) -> QueryResult<Option<(String, String)>> {
    let row: Option<SyncModeState> = state::sync_mode_state
        .filter(state::id.eq(1))
        .first(conn)
        .optional()?;
    Ok(row.map(|r| (r.mode, r.network_class)))
}

/// Upsert the singleton state row (id=1).
pub fn save_state(conn: &mut SqliteConnection, mode: &str, network_class: &str) -> QueryResult<()> {
    let now = current_timestamp();
    diesel::insert_into(state::sync_mode_state)
        .values((
            state::id.eq(1),
            state::mode.eq(mode),
            state::network_class.eq(network_class),
            state::updated_at.eq(&now),
        ))
        .on_conflict(state::id)
        .do_update()
        .set((
            state::mode.eq(mode),
            state::network_class.eq(network_class),
            state::updated_at.eq(&now),
        ))
        .execute(conn)?;
    Ok(())
}

/// Append a transition row and prune the history to [`TRANSITION_HISTORY_CAP`]
/// rows (oldest first) in the same call.
pub fn append_transition(
    conn: &mut SqliteConnection,
    from_mode: &str,
    to_mode: &str,
    source: &str,
    reason: &str,
) -> QueryResult<()> {
    diesel::insert_into(transitions::sync_mode_transitions)
        .values((
            transitions::at.eq(current_timestamp()),
            transitions::from_mode.eq(from_mode),
            transitions::to_mode.eq(to_mode),
            transitions::source.eq(source),
            transitions::reason.eq(reason),
        ))
        .execute(conn)?;
    // Prune: keep only the newest TRANSITION_HISTORY_CAP rows.
    diesel::sql_query(format!(
        "DELETE FROM sync_mode_transitions WHERE id NOT IN \
         (SELECT id FROM sync_mode_transitions ORDER BY id DESC LIMIT {})",
        TRANSITION_HISTORY_CAP
    ))
    .execute(conn)?;
    Ok(())
}

/// List transitions newest-first (bounded by the retention cap).
pub fn list_transitions(conn: &mut SqliteConnection) -> QueryResult<Vec<SyncModeTransition>> {
    transitions::sync_mode_transitions
        .order(transitions::id.desc())
        .limit(TRANSITION_HISTORY_CAP)
        .load(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn setup() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory DB");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    #[test]
    fn state_defaults_to_absent_then_round_trips() {
        let mut conn = setup();
        assert_eq!(load_state(&mut conn).unwrap(), None);
        save_state(&mut conn, "wifi-only", "cellular").unwrap();
        assert_eq!(
            load_state(&mut conn).unwrap(),
            Some(("wifi-only".to_string(), "cellular".to_string()))
        );
        // Upsert path: second save updates the singleton, never a second row.
        save_state(&mut conn, "sync", "wifi").unwrap();
        assert_eq!(
            load_state(&mut conn).unwrap(),
            Some(("sync".to_string(), "wifi".to_string()))
        );
    }

    #[test]
    fn transitions_append_and_list_newest_first() {
        let mut conn = setup();
        append_transition(
            &mut conn,
            "sync",
            "paused",
            "operator",
            "operator set sync mode",
        )
        .unwrap();
        append_transition(
            &mut conn,
            "paused",
            "sync",
            "operator",
            "operator set sync mode",
        )
        .unwrap();
        let rows = list_transitions(&mut conn).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].to_mode, "sync"); // newest first
        assert_eq!(rows[1].to_mode, "paused");
        assert!(!rows[0].at.is_empty());
        assert_eq!(rows[0].source, "operator");
    }

    #[test]
    fn transition_history_prunes_to_cap() {
        let mut conn = setup();
        for i in 0..(TRANSITION_HISTORY_CAP + 25) {
            append_transition(
                &mut conn,
                "sync",
                "paused",
                "operator",
                &format!("transition {}", i),
            )
            .unwrap();
        }
        let rows = list_transitions(&mut conn).unwrap();
        assert_eq!(rows.len() as i64, TRANSITION_HISTORY_CAP);
        // Newest row survived; the oldest 25 were pruned.
        assert_eq!(
            rows[0].reason,
            format!("transition {}", TRANSITION_HISTORY_CAP + 24)
        );
        assert_eq!(rows.last().unwrap().reason, "transition 25");
    }
}
