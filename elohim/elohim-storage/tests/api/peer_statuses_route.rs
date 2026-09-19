//! Integration tests for `GET /api/v1/peer-statuses` (Task B2, Phase 1 closeout).
//!
//! ## Testing strategy
//!
//! Mirrors the pattern used by `gate_decisions_http.rs` and
//! `peer_status_e2e.rs`: no live HTTP server. The handler is thin — it calls
//! `db::peer_statuses::list_current` / `list_by_household` and maps rows to
//! `PeerStatusView`. We test the DB layer and view conversion directly.
//!
//! 1. **list_current empty** — returns empty Vec when table has no rows.
//! 2. **list_current populated** — returns all rows.
//! 3. **view conversion** — PeerStatusView fields are correctly mapped.
//! 4. **list_by_household** — joins through `stewarded_nodes.household_id`
//!    (the C3 column) and returns only peers bound to the queried household.
//! 5. **boolean coercion** — general_pool_member and accepting_stewardship_reserves
//!    are correctly coerced from i32 to bool in PeerStatusView.

use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use elohim_storage::db::peer_statuses::{list_by_household, list_current, upsert, PeerStatusRow};
use elohim_storage::views::PeerStatusView;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup_db() -> SqliteConnection {
    let mut conn =
        SqliteConnection::establish(":memory:").expect("Failed to create in-memory SQLite");

    conn.batch_execute(
        r#"
        CREATE TABLE peer_statuses (
            peer_id TEXT PRIMARY KEY,
            status TEXT NOT NULL,
            general_pool_member INTEGER NOT NULL,
            accepting_stewardship_reserves INTEGER NOT NULL,
            archetype_class TEXT,
            timestamp BIGINT NOT NULL,
            dht_anchor_hash TEXT NOT NULL,
            updated_at BIGINT NOT NULL
        );
        CREATE INDEX idx_peer_statuses_status ON peer_statuses(status);
        CREATE INDEX idx_peer_statuses_pool ON peer_statuses(general_pool_member);
        -- Minimal stewarded_nodes projection: list_by_household joins through
        -- stewarded_nodes.id / household_id only (the C3 column).
        CREATE TABLE stewarded_nodes (
            id TEXT PRIMARY KEY,
            household_id TEXT
        );
        "#,
    )
    .expect("Failed to create test schema");

    conn
}

fn make_row(peer_id: &str, status: &str, pool_member: i32) -> PeerStatusRow {
    PeerStatusRow {
        peer_id: peer_id.into(),
        status: status.into(),
        general_pool_member: pool_member,
        accepting_stewardship_reserves: 1,
        archetype_class: Some("home-nuc".into()),
        timestamp: 1_700_000_000_000_000,
        dht_anchor_hash: "uhCkkANCHOR".into(),
        updated_at: 1_700_000_000_000_000,
    }
}

// ---------------------------------------------------------------------------
// Test 1 — list_current returns empty array when table is empty
// ---------------------------------------------------------------------------

#[test]
fn list_current_empty_returns_empty_vec() {
    let mut conn = setup_db();
    let rows = list_current(&mut conn).unwrap();
    assert!(
        rows.is_empty(),
        "expected empty Vec for empty table, got {} rows",
        rows.len()
    );
}

// ---------------------------------------------------------------------------
// Test 2 — list_current returns all rows when table is populated
// ---------------------------------------------------------------------------

#[test]
fn list_current_populated_returns_all_rows() {
    let mut conn = setup_db();

    upsert(&mut conn, &make_row("peer-alpha", "online", 1)).unwrap();
    upsert(&mut conn, &make_row("peer-beta", "degraded", 1)).unwrap();
    upsert(&mut conn, &make_row("peer-gamma", "offline", 0)).unwrap();

    let rows = list_current(&mut conn).unwrap();
    assert_eq!(rows.len(), 3, "all three rows must be returned");
}

// ---------------------------------------------------------------------------
// Test 3 — PeerStatusView conversion: fields are correctly mapped
// ---------------------------------------------------------------------------

#[test]
fn view_conversion_maps_all_fields_correctly() {
    let row = PeerStatusRow {
        peer_id: "uhCAkPEER".into(),
        status: "online".into(),
        general_pool_member: 1,
        accepting_stewardship_reserves: 0,
        archetype_class: Some("blade".into()),
        timestamp: 1_700_000_000_123_456,
        dht_anchor_hash: "uhCkkDEFGHIJ".into(),
        updated_at: 1_700_000_000_999_000,
    };

    let view = PeerStatusView::from(row);

    assert_eq!(view.peer_id, "uhCAkPEER");
    assert_eq!(view.status, "online");
    assert!(view.general_pool_member, "general_pool_member 1 → true");
    assert!(
        !view.accepting_stewardship_reserves,
        "accepting_stewardship_reserves 0 → false"
    );
    assert_eq!(view.archetype_class.as_deref(), Some("blade"));
    assert_eq!(view.timestamp, "1700000000123456");
    assert_eq!(view.dht_anchor_hash, "uhCkkDEFGHIJ");
    assert_eq!(view.updated_at, "1700000000999000");
    // elohim_capability is None until Phase 9 populates it
    assert!(
        view.elohim_capability.is_none(),
        "elohim_capability must be None before Phase 9"
    );
}

// ---------------------------------------------------------------------------
// Test 4 — list_by_household filters through stewarded_nodes.household_id (C3)
// ---------------------------------------------------------------------------

#[test]
fn list_by_household_returns_only_peers_bound_to_household() {
    let mut conn = setup_db();

    upsert(&mut conn, &make_row("peer-one", "online", 1)).unwrap();
    upsert(&mut conn, &make_row("peer-two", "online", 0)).unwrap();
    upsert(&mut conn, &make_row("peer-unbound", "online", 1)).unwrap();

    // Bind peer-one to the queried household and peer-two to a different one;
    // peer-unbound has no stewarded_nodes row at all. Only peer-one may come
    // back — the old stub returned ALL peers per household, which inflated
    // onlinePeerCount (see list_by_household's doc comment).
    conn.batch_execute(
        r#"
        INSERT INTO stewarded_nodes (id, household_id) VALUES
            ('peer-one', 'household-abc123'),
            ('peer-two', 'household-other');
        "#,
    )
    .expect("Failed to seed stewarded_nodes");

    let rows = list_by_household(&mut conn, "household-abc123").unwrap();
    assert_eq!(
        rows.len(),
        1,
        "list_by_household must return only peers whose stewarded_nodes row matches the household"
    );
    assert_eq!(rows[0].peer_id, "peer-one");
}

// ---------------------------------------------------------------------------
// Test 5 — boolean coercion: both flag columns coerce correctly
// ---------------------------------------------------------------------------

#[test]
fn boolean_flags_coerce_from_i32_correctly() {
    let row_true = PeerStatusRow {
        general_pool_member: 1,
        accepting_stewardship_reserves: 1,
        ..make_row("peer-flags-true", "online", 1)
    };
    let row_false = PeerStatusRow {
        general_pool_member: 0,
        accepting_stewardship_reserves: 0,
        ..make_row("peer-flags-false", "offline", 0)
    };

    let view_true = PeerStatusView::from(row_true);
    let view_false = PeerStatusView::from(row_false);

    assert!(view_true.general_pool_member);
    assert!(view_true.accepting_stewardship_reserves);
    assert!(!view_false.general_pool_member);
    assert!(!view_false.accepting_stewardship_reserves);
}

// ---------------------------------------------------------------------------
// Test 6 — views can be serialized to JSON array (simulates HTTP response body)
// ---------------------------------------------------------------------------

#[test]
fn views_serialize_to_json_array() {
    let mut conn = setup_db();

    upsert(&mut conn, &make_row("peer-serialize", "online", 1)).unwrap();

    let rows = list_current(&mut conn).unwrap();
    let views: Vec<PeerStatusView> = rows.into_iter().map(Into::into).collect();

    let json = serde_json::to_string(&views).expect("views must serialize to JSON");
    let parsed: serde_json::Value =
        serde_json::from_str(&json).expect("serialized views must parse");

    assert!(parsed.is_array(), "serialized views must be a JSON array");
    assert_eq!(
        parsed.as_array().unwrap().len(),
        1,
        "array must contain one entry"
    );

    // Verify camelCase keys at the wire boundary
    let entry = &parsed[0];
    assert!(
        entry.get("peerId").is_some(),
        "camelCase peerId field must be present"
    );
    assert!(
        entry.get("generalPoolMember").is_some(),
        "camelCase generalPoolMember field must be present"
    );
    assert!(
        entry.get("acceptingStewardshipReserves").is_some(),
        "camelCase acceptingStewardshipReserves field must be present"
    );
    assert!(
        entry.get("dhtAnchorHash").is_some(),
        "camelCase dhtAnchorHash field must be present"
    );
}
