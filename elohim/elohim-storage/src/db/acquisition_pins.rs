//! DevicePin CRUD (Category B local store). Airplane-mode property: nothing
//! here touches p2p, the conductor, or the network — pure local SQLite.
//!
//! Spec: 2026-06-07-epr-acquisition-pull-queue-design.md §1.1, §3.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use super::diesel_schema::acquisition_pins::dsl as pins;
use super::models::{current_timestamp, AcquisitionPin, NewAcquisitionPin};

/// Idempotent upsert on the composite identity (agent, head_ref, kind).
///
/// Re-pinning the same identity refreshes priority and closure_rule_json and
/// revives a 'removed' pin back to 'active'.
///
/// Timestamps are always computed in Rust via `current_timestamp()` so every
/// row in the table uses the same ISO 8601 `YYYY-MM-DDTHH:MM:SSZ` format.
/// The insert path sets both `created_at` and `updated_at` explicitly (via a
/// tuple-values insert) to avoid the SQL `datetime('now')` default that
/// produces a different format (`YYYY-MM-DD HH:MM:SS`).  On conflict only
/// `updated_at` is refreshed — `created_at` is never mutated on re-pin.
pub fn upsert_pin(
    conn: &mut SqliteConnection,
    new: NewAcquisitionPin,
) -> QueryResult<AcquisitionPin> {
    let now = current_timestamp();
    diesel::insert_into(pins::acquisition_pins)
        .values((
            pins::agent_pub_key.eq(&new.agent_pub_key),
            pins::head_ref.eq(&new.head_ref),
            pins::kind.eq(&new.kind),
            pins::closure_rule_json.eq(&new.closure_rule_json),
            pins::priority.eq(new.priority),
            pins::created_at.eq(&now),
            pins::updated_at.eq(&now),
        ))
        .on_conflict((pins::agent_pub_key, pins::head_ref, pins::kind))
        .do_update()
        .set((
            pins::closure_rule_json.eq(new.closure_rule_json.clone()),
            pins::priority.eq(new.priority),
            pins::status.eq("active"),
            pins::updated_at.eq(&now),
        ))
        .execute(conn)?;
    pins::acquisition_pins
        .filter(pins::agent_pub_key.eq(&new.agent_pub_key))
        .filter(pins::head_ref.eq(&new.head_ref))
        .filter(pins::kind.eq(&new.kind))
        .first(conn)
}

/// List pins with status='active', ordered by priority descending (highest first).
pub fn list_active_pins(conn: &mut SqliteConnection) -> QueryResult<Vec<AcquisitionPin>> {
    pins::acquisition_pins
        .filter(pins::status.eq("active"))
        .order(pins::priority.desc())
        .load(conn)
}

/// List all pins regardless of status, ordered by id ascending.
pub fn list_all_pins(conn: &mut SqliteConnection) -> QueryResult<Vec<AcquisitionPin>> {
    pins::acquisition_pins.order(pins::id.asc()).load(conn)
}

/// Update the status of a single pin by its integer id.
///
/// Returns the number of rows affected (0 if the id does not exist).
pub fn set_pin_status(
    conn: &mut SqliteConnection,
    pin_id: i32,
    new_status: &str,
) -> QueryResult<usize> {
    let now = current_timestamp();
    diesel::update(pins::acquisition_pins.filter(pins::id.eq(pin_id)))
        .set((pins::status.eq(new_status), pins::updated_at.eq(&now)))
        .execute(conn)
}

/// Back-fill the notarized commitment back-reference on a pin after the
/// provide reconciler authors its replicates-commons Commitment.
///
/// Returns the number of rows affected (0 if the id does not exist).
pub fn set_commitment_cid(
    conn: &mut SqliteConnection,
    pin_id: i32,
    commitment_cid: &str,
) -> QueryResult<usize> {
    let now = current_timestamp();
    diesel::update(pins::acquisition_pins.filter(pins::id.eq(pin_id)))
        .set((
            pins::commitment_cid.eq(commitment_cid),
            pins::updated_at.eq(&now),
        ))
        .execute(conn)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn test_conn() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory SQLite");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    fn sample_pin(priority: i32) -> NewAcquisitionPin {
        NewAcquisitionPin {
            agent_pub_key: "local-device".into(),
            head_ref: "epr:album-1".into(),
            kind: "item".into(),
            closure_rule_json: None,
            priority,
        }
    }

    #[test]
    fn upsert_is_idempotent_on_composite_identity() {
        let mut conn = test_conn();

        // First upsert — priority 1.
        let pin1 = upsert_pin(&mut conn, sample_pin(1)).expect("first upsert");

        // Second upsert — same composite identity, higher priority.
        let pin2 = upsert_pin(&mut conn, sample_pin(5)).expect("second upsert");

        // Same database row (same id).
        assert_eq!(pin1.id, pin2.id, "re-upsert must update the existing row");

        // Priority was refreshed.
        assert_eq!(pin2.priority, 5);

        // Exactly one active pin in the table.
        let active = list_active_pins(&mut conn).expect("list_active_pins");
        assert_eq!(active.len(), 1, "exactly one active pin after two upserts");
        assert_eq!(active[0].status, "active");
    }

    #[test]
    fn removed_pins_drop_out_of_active_and_revive_on_repin() {
        let mut conn = test_conn();

        let pin = upsert_pin(&mut conn, sample_pin(1)).expect("upsert");

        // Mark removed — must disappear from active list.
        set_pin_status(&mut conn, pin.id, "removed").expect("set removed");
        let active = list_active_pins(&mut conn).expect("list after remove");
        assert!(
            active.is_empty(),
            "removed pin must not appear in active list"
        );

        // Removed pins must still appear in list_all (soft-delete, not hard-delete).
        let all = list_all_pins(&mut conn).expect("list_all after remove");
        assert_eq!(all.len(), 1, "removed pin must remain in list_all");

        // Re-upsert same identity — must revive to active.
        upsert_pin(&mut conn, sample_pin(1)).expect("re-upsert after remove");
        let active2 = list_active_pins(&mut conn).expect("list after re-pin");
        assert_eq!(active2.len(), 1, "revived pin must appear in active list");
        assert_eq!(active2[0].status, "active");
    }

    #[test]
    fn commitment_cid_defaults_null_and_set_backfills() {
        let mut conn = test_conn();

        // A fresh pin is born without a commitment_cid (NULL).
        let pin = upsert_pin(&mut conn, sample_pin(1)).expect("upsert");
        assert!(
            pin.commitment_cid.is_none(),
            "a fresh pin must have NULL commitment_cid (notarized shadow not yet authored)"
        );

        // Back-fill the commitment_cid after the reconciler authors the
        // replicates-commons Commitment.
        let affected = set_commitment_cid(&mut conn, pin.id, "uhCkk-commons-commit-1")
            .expect("set_commitment_cid");
        assert_eq!(affected, 1, "set_commitment_cid must affect exactly one row");

        let reread = list_all_pins(&mut conn).expect("list_all");
        assert_eq!(reread.len(), 1);
        assert_eq!(
            reread[0].commitment_cid.as_deref(),
            Some("uhCkk-commons-commit-1"),
            "commitment_cid must reflect the back-filled value"
        );

        // Re-upsert (re-pin) must NOT clobber the commitment_cid — only
        // updated_at/priority/status are refreshed on conflict.
        upsert_pin(&mut conn, sample_pin(9)).expect("re-upsert");
        let after_repin = list_all_pins(&mut conn).expect("list_all after re-pin");
        assert_eq!(
            after_repin[0].commitment_cid.as_deref(),
            Some("uhCkk-commons-commit-1"),
            "re-pin must preserve the notarized commitment_cid back-reference"
        );
    }
}
