//! Lens-market C-class fold-input CRUD (lens-market S4).
//!
//! Operational (Category C) records that the facing folds aggregate ON READ:
//! `lens_selections` → `affinity_in_scope`, `lens_verdicts` → `contention_index`.
//! NO `dht_anchor_hash` — affinity/contention are computed, never notarized
//! (spec §4.4). DORMANT in this slice (the production write-path is the deferred
//! ballot/selection leg, plan A6); these read EMPTY until a producer lands, and
//! the facing degrades to affinity=0 / contention=0.
//!
//! Plan: 2026-06-27-plural-mishpat-lenses-service-layer-plan.md (S4).

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use super::diesel_schema::{lens_selections::dsl as sel, lens_verdicts::dsl as vdt};
use super::models::{LensSelection, LensVerdict, NewLensSelection, NewLensVerdict};

/// Insert (idempotent on the slug `id`) a lens-selection record.
pub fn insert_selection(conn: &mut SqliteConnection, new: NewLensSelection) -> QueryResult<usize> {
    diesel::insert_into(sel::lens_selections)
        .values(&new)
        .on_conflict(sel::id)
        .do_nothing()
        .execute(conn)
}

/// All selection records in an EPR scope — the affinity fold input.
pub fn selections_in_scope(
    conn: &mut SqliteConnection,
    epr_scope: &str,
) -> QueryResult<Vec<LensSelection>> {
    sel::lens_selections
        .filter(sel::epr_scope.eq(epr_scope))
        .load(conn)
}

/// Insert (idempotent on the slug `id`) a lens-verdict record.
pub fn insert_verdict(conn: &mut SqliteConnection, new: NewLensVerdict) -> QueryResult<usize> {
    diesel::insert_into(vdt::lens_verdicts)
        .values(&new)
        .on_conflict(vdt::id)
        .do_nothing()
        .execute(conn)
}

/// All verdict records in an EPR scope — the contention fold input.
pub fn verdicts_in_scope(
    conn: &mut SqliteConnection,
    epr_scope: &str,
) -> QueryResult<Vec<LensVerdict>> {
    vdt::lens_verdicts
        .filter(vdt::epr_scope.eq(epr_scope))
        .load(conn)
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

    #[test]
    fn selections_round_trip_and_scope_filter() {
        let mut conn = test_conn();
        insert_selection(
            &mut conn,
            NewLensSelection {
                id: "lens:g1:epr:lamad-spa:agentA".to_string(),
                lens_cid: "lens:g1".to_string(),
                selector_agent: "agentA".to_string(),
                epr_scope: "epr:lamad-spa".to_string(),
                selected_at: "2026-06-27T00:00:00Z".to_string(),
            },
        )
        .expect("insert sel");
        // A selection in a different scope must not leak.
        insert_selection(
            &mut conn,
            NewLensSelection {
                id: "lens:g1:epr:other:agentB".to_string(),
                lens_cid: "lens:g1".to_string(),
                selector_agent: "agentB".to_string(),
                epr_scope: "epr:other".to_string(),
                selected_at: "2026-06-27T00:00:00Z".to_string(),
            },
        )
        .expect("insert sel2");

        let rows = selections_in_scope(&mut conn, "epr:lamad-spa").expect("query");
        assert_eq!(rows.len(), 1, "only the in-scope selection");
        assert_eq!(rows[0].selector_agent, "agentA");
    }

    #[test]
    fn verdicts_round_trip_and_scope_filter() {
        let mut conn = test_conn();
        insert_verdict(
            &mut conn,
            NewLensVerdict {
                id: "epr:lamad-spa:lens:g1:agentA".to_string(),
                epr_scope: "epr:lamad-spa".to_string(),
                lens_cid: "lens:g1".to_string(),
                verdict: "agree".to_string(),
                agent: "agentA".to_string(),
                created_at: "2026-06-27T00:00:00Z".to_string(),
            },
        )
        .expect("insert vdt");

        let rows = verdicts_in_scope(&mut conn, "epr:lamad-spa").expect("query");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].verdict, "agree");
        // Empty scope → empty.
        assert!(verdicts_in_scope(&mut conn, "epr:nope")
            .expect("query")
            .is_empty());
    }

    #[test]
    fn insert_selection_is_idempotent_on_id() {
        let mut conn = test_conn();
        let row = || NewLensSelection {
            id: "dup".to_string(),
            lens_cid: "lens:g1".to_string(),
            selector_agent: "agentA".to_string(),
            epr_scope: "epr:lamad-spa".to_string(),
            selected_at: "2026-06-27T00:00:00Z".to_string(),
        };
        insert_selection(&mut conn, row()).expect("first");
        insert_selection(&mut conn, row()).expect("second (idempotent)");
        assert_eq!(
            selections_in_scope(&mut conn, "epr:lamad-spa")
                .unwrap()
                .len(),
            1,
            "duplicate id must not create a second row"
        );
    }
}
