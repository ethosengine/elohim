//! Integration tests for `db::placement_gaps` CRUD (Task 4, Plan 1).
//!
//! Uses the same in-memory SQLite + batch_execute pattern as sibling test files
//! (peer_statuses_route.rs, gate_decisions_http.rs). No live HTTP server needed:
//! the DB layer and gap logic are tested directly.

use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use elohim_storage::db::models::NewPlacementGap;
use elohim_storage::db::placement_gaps;

// ---------------------------------------------------------------------------
// Setup helper
// ---------------------------------------------------------------------------

fn setup_db() -> SqliteConnection {
    let mut conn =
        SqliteConnection::establish(":memory:").expect("Failed to create in-memory SQLite");

    conn.batch_execute(
        r#"
        CREATE TABLE placement_gaps (
            id                      TEXT PRIMARY KEY NOT NULL,
            content_id              TEXT NOT NULL,
            shard_hash              TEXT NOT NULL,
            h_app_id                TEXT NOT NULL,
            requested_steward_count INTEGER NOT NULL,
            achieved_steward_count  INTEGER NOT NULL,
            contract_coverage       REAL NOT NULL,
            gap_kind                TEXT NOT NULL,
            first_seen_at           TEXT NOT NULL,
            last_seen_at            TEXT NOT NULL
        );
        CREATE UNIQUE INDEX idx_placement_gaps_unique
            ON placement_gaps(content_id, shard_hash, h_app_id, gap_kind);
        CREATE INDEX idx_placement_gaps_content  ON placement_gaps(content_id);
        CREATE INDEX idx_placement_gaps_kind     ON placement_gaps(gap_kind);
        CREATE INDEX idx_placement_gaps_last_seen ON placement_gaps(last_seen_at);
        "#,
    )
    .expect("Failed to create test schema");

    conn
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn upsert_new_row_inserts() {
    let mut conn = setup_db();

    let gap = NewPlacementGap {
        id: "gap-1",
        content_id: "content-alpha",
        shard_hash: "shard-h1",
        h_app_id: "lamad",
        requested_steward_count: 3,
        achieved_steward_count: 1,
        contract_coverage: 0.33,
        gap_kind: "peers-unavailable",
        first_seen_at: "2026-04-19T00:00:00Z",
        last_seen_at: "2026-04-19T00:00:00Z",
    };

    placement_gaps::upsert_gap(&mut conn, &gap).unwrap();

    let rows = placement_gaps::list_gaps(&mut conn, "lamad", Default::default()).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].gap_kind, "peers-unavailable");
    assert_eq!(rows[0].achieved_steward_count, 1);
}

#[test]
fn upsert_existing_bumps_last_seen_keeps_first_seen() {
    let mut conn = setup_db();

    let gap_v1 = NewPlacementGap {
        id: "gap-2",
        content_id: "content-beta",
        shard_hash: "shard-h2",
        h_app_id: "lamad",
        requested_steward_count: 3,
        achieved_steward_count: 2,
        contract_coverage: 0.66,
        gap_kind: "under-committed",
        first_seen_at: "2026-04-19T00:00:00Z",
        last_seen_at: "2026-04-19T00:00:00Z",
    };
    placement_gaps::upsert_gap(&mut conn, &gap_v1).unwrap();

    let gap_v2 = NewPlacementGap {
        id: "gap-ignored", // existing row keeps its id (unique index on content+shard+app+kind)
        last_seen_at: "2026-04-19T01:30:00Z",
        achieved_steward_count: 3, // healed one household
        contract_coverage: 1.0,
        ..gap_v1
    };
    placement_gaps::upsert_gap(&mut conn, &gap_v2).unwrap();

    let rows = placement_gaps::list_gaps(&mut conn, "lamad", Default::default()).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].first_seen_at, "2026-04-19T00:00:00Z");
    assert_eq!(rows[0].last_seen_at, "2026-04-19T01:30:00Z");
    assert_eq!(rows[0].achieved_steward_count, 3);
}

#[test]
fn list_filters_by_kind() {
    let mut conn = setup_db();

    for (id, kind) in [
        ("a", "peers-unavailable"),
        ("b", "under-committed"),
        ("c", "peers-unavailable"),
    ] {
        placement_gaps::upsert_gap(
            &mut conn,
            &NewPlacementGap {
                id,
                content_id: id,
                shard_hash: id,
                h_app_id: "lamad",
                requested_steward_count: 3,
                achieved_steward_count: 0,
                contract_coverage: 0.0,
                gap_kind: kind,
                first_seen_at: "2026-04-19T00:00:00Z",
                last_seen_at: "2026-04-19T00:00:00Z",
            },
        )
        .unwrap();
    }

    let filter = placement_gaps::GapQuery {
        kind: Some("peers-unavailable".into()),
        ..Default::default()
    };
    let rows = placement_gaps::list_gaps(&mut conn, "lamad", filter).unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r.gap_kind == "peers-unavailable"));
}

#[test]
fn gc_stale_removes_old_rows() {
    let mut conn = setup_db();

    let fresh = NewPlacementGap {
        id: "fresh",
        content_id: "a",
        shard_hash: "shard-fresh",
        h_app_id: "lamad",
        requested_steward_count: 3,
        achieved_steward_count: 0,
        contract_coverage: 0.0,
        gap_kind: "peers-unavailable",
        first_seen_at: "2026-04-19T00:00:00Z",
        last_seen_at: "2026-04-19T00:00:00Z",
    };
    // Different shard_hash so the unique index (content_id, shard_hash, h_app_id, gap_kind)
    // allows both rows to coexist in the table.
    let stale = NewPlacementGap {
        id: "stale",
        shard_hash: "shard-stale",
        last_seen_at: "2026-04-17T00:00:00Z",
        ..fresh
    };
    placement_gaps::upsert_gap(&mut conn, &fresh).unwrap();
    placement_gaps::upsert_gap(&mut conn, &stale).unwrap();

    let removed = placement_gaps::gc_stale(&mut conn, "2026-04-18T00:00:00Z").unwrap();
    assert_eq!(removed, 1);

    let rows = placement_gaps::list_gaps(&mut conn, "lamad", Default::default()).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "fresh");
}
