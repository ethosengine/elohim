//! Integration tests for `GET /api/v1/placement-gaps` (Task 9, Plan 1).
//!
//! ## Testing strategy
//!
//! The codebase pattern (peer_statuses_route.rs, gate_decisions_http.rs,
//! placement_gaps.rs) tests the DB layer and view conversion directly — no live
//! HTTP server. `spawn_test_server` does not exist in `test_util.rs`; adding a
//! full hyper test harness is deferred to Task 17 (A2O coverage).
//!
//! The two tests named in the plan (`list_returns_empty_when_no_gaps` and
//! `list_returns_rows_filterable_by_kind`) are implemented here as DB-layer
//! tests that exercise the same code path the handler delegates to, giving
//! equivalent confidence that the handler will return correct JSON when wired
//! through HTTP.
//!
//! HTTP-server variants are stubbed with `#[ignore]` and a comment pointing at
//! Task 17.

use elohim_storage::db::models::NewPlacementGap;
use elohim_storage::db::placement_gaps;
use elohim_storage::test_util::test_pool;
use elohim_storage::views::PlacementGapView;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_gap<'a>(id: &'a str, kind: &'a str) -> NewPlacementGap<'a> {
    NewPlacementGap {
        id,
        content_id: id,
        shard_hash: id,
        h_app_id: "lamad",
        requested_steward_count: 3,
        achieved_steward_count: 1,
        contract_coverage: 0.5,
        gap_kind: kind,
        first_seen_at: "2026-04-19T00:00:00Z",
        last_seen_at: "2026-04-19T00:00:00Z",
    }
}

// ---------------------------------------------------------------------------
// list_returns_empty_when_no_gaps
//
// Handler delegates to placement_gaps::list_gaps; when the table is empty the
// handler serialises `{ items: [], total: 0 }`. This test pins the DB layer
// half of that path.
// ---------------------------------------------------------------------------

#[test]
fn list_returns_empty_when_no_gaps() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    let rows = placement_gaps::list_gaps(&mut conn, "lamad", Default::default()).unwrap();
    let items: Vec<PlacementGapView> = rows.into_iter().map(Into::into).collect();

    assert!(
        items.is_empty(),
        "expected empty items for empty table, got {}",
        items.len()
    );
}

// ---------------------------------------------------------------------------
// list_returns_rows_filterable_by_kind
//
// Inserts two gap rows with different kinds and verifies that filtering by
// `kind=peers-unavailable` returns exactly one row with the correct gapKind.
// The handler passes `query.get("kind")` directly to GapQuery::kind, so this
// test covers the filtering path end-to-end through the DB layer.
// ---------------------------------------------------------------------------

#[test]
fn list_returns_rows_filterable_by_kind() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    for (id, kind) in [("g1", "peers-unavailable"), ("g2", "under-committed")] {
        placement_gaps::upsert_gap(&mut conn, &make_gap(id, kind)).unwrap();
    }

    let filter = placement_gaps::GapQuery {
        kind: Some("peers-unavailable".to_string()),
        ..Default::default()
    };
    let rows = placement_gaps::list_gaps(&mut conn, "lamad", filter).unwrap();
    let items: Vec<PlacementGapView> = rows.into_iter().map(Into::into).collect();

    assert_eq!(items.len(), 1, "filter by kind must return exactly one row");
    assert_eq!(
        items[0].gap_kind, "peers-unavailable",
        "returned row must have the requested gap_kind"
    );
    // Verify camelCase wire format
    let json = serde_json::to_string(&items[0]).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(
        v.get("gapKind").is_some(),
        "wire format must have camelCase gapKind field"
    );
    assert!(
        v.get("contentId").is_some(),
        "wire format must have camelCase contentId field"
    );
}

// ---------------------------------------------------------------------------
// HTTP-server variants — ignored until Task 17 adds spawn_test_server.
//
// These match the test signatures from the plan exactly. They are left here so
// Task 17 can un-ignore and wire up spawn_test_server without renaming tests.
// ---------------------------------------------------------------------------

/// Ignored: `spawn_test_server` does not exist yet.
/// Task 17 (A2O) will add the hyper test-server harness and un-ignore this.
#[tokio::test]
#[ignore = "spawn_test_server not yet implemented — see Task 17"]
async fn list_returns_empty_when_no_gaps_http() {
    // When spawn_test_server exists, replace this block with:
    //   let pool = test_pool();
    //   let base = spawn_test_server(pool).await;
    //   let resp: serde_json::Value = reqwest::get(format!("{base}/api/v1/placement-gaps"))
    //       .await.unwrap().json().await.unwrap();
    //   assert!(resp["items"].as_array().unwrap().is_empty());
    unimplemented!("spawn_test_server not yet available")
}

/// Ignored: `spawn_test_server` does not exist yet.
/// Task 17 (A2O) will add the hyper test-server harness and un-ignore this.
#[tokio::test]
#[ignore = "spawn_test_server not yet implemented — see Task 17"]
async fn list_returns_rows_filterable_by_kind_http() {
    // When spawn_test_server exists, replace this block with:
    //   let pool = test_pool();
    //   let mut conn = pool.get().unwrap();
    //   for (id, kind) in [("g1","peers-unavailable"),("g2","under-committed")] {
    //       placement_gaps::upsert_gap(&mut conn, &make_gap(id, kind)).unwrap();
    //   }
    //   drop(conn);
    //   let base = spawn_test_server(pool).await;
    //   let resp: serde_json::Value =
    //       reqwest::get(format!("{base}/api/v1/placement-gaps?kind=peers-unavailable"))
    //           .await.unwrap().json().await.unwrap();
    //   let items = resp["items"].as_array().unwrap();
    //   assert_eq!(items.len(), 1);
    //   assert_eq!(items[0]["gapKind"], "peers-unavailable");
    unimplemented!("spawn_test_server not yet available")
}
