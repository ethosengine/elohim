//! Integration tests verifying that `ObservationRow` / `ObservationDiversitySummaryRow`
//! are Diesel-queryable and that `ObservationView` / `ObservationDiversitySummaryView`
//! convert correctly from those row types.
//!
//! The view structs in `views.rs` follow the file's established pattern:
//! `#[derive(Debug, Clone, Serialize, TS)]` — no `Queryable`. Diesel queries
//! return model rows (`*Row`); the HTTP layer converts to views via `From`.

use diesel::prelude::*;
use diesel::RunQueryDsl;
use elohim_storage::db::diesel_schema::{observation_diversity_summary, observations};
use elohim_storage::db::models::{ObservationDiversitySummaryRow, ObservationRow};
use elohim_storage::test_util::test_pool;
use elohim_storage::views::{ObservationDiversitySummaryView, ObservationView};

#[test]
fn observation_row_loads_inserted_row() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    diesel::insert_into(observations::table)
        .values((
            observations::observer_cid.eq("agent:test"),
            observations::log_cid.eq("blake3:test"),
            observations::log_offset.eq(0_i64),
            observations::observed_at.eq(1_715_420_400_i64),
            observations::seq.eq(0_i64),
            observations::observation_kind.eq("infrastructure:doorway-heartbeat"),
            observations::subject_cid.eq(Some("doorway:abc")),
            observations::subject_kind.eq(Some("doorway")),
            observations::payload_json.eq("{}"),
            observations::observer_household_cid.eq::<Option<String>>(None),
            observations::observer_collective_cid.eq::<Option<String>>(None),
            observations::observer_region.eq::<Option<String>>(None),
            observations::observer_archetype.eq::<Option<String>>(None),
            observations::observer_compute_class.eq::<Option<String>>(None),
            observations::signature_b64.eq(""),
        ))
        .execute(&mut conn)
        .expect("insert observation");

    let row: ObservationRow = observations::table.first(&mut conn).expect("query");
    assert_eq!(row.observer_cid, "agent:test");
    assert_eq!(row.log_offset, 0);
    assert_eq!(row.subject_kind, Some("doorway".to_string()));
    assert_eq!(row.observer_household_cid, None);
}

#[test]
fn observation_view_converts_from_row() {
    let row = ObservationRow {
        observer_cid: "agent:test".to_string(),
        log_cid: "blake3:abc".to_string(),
        log_offset: 7,
        observed_at: 1_715_420_400,
        seq: 3,
        observation_kind: "infrastructure:doorway-heartbeat".to_string(),
        subject_cid: Some("doorway:xyz".to_string()),
        subject_kind: Some("doorway".to_string()),
        payload_json: r#"{"uptime":42}"#.to_string(),
        observer_household_cid: Some("household:h1".to_string()),
        observer_collective_cid: None,
        observer_region: Some("us-west".to_string()),
        observer_archetype: Some("tier-1-hub".to_string()),
        observer_compute_class: None,
        signature_b64: "abc123".to_string(),
    };

    let view: ObservationView = row.into();
    assert_eq!(view.observer_cid, "agent:test");
    assert_eq!(view.log_offset, 7);
    assert_eq!(view.subject_kind, Some("doorway".to_string()));
    assert_eq!(view.observer_region, Some("us-west".to_string()));
    assert_eq!(view.observer_compute_class, None);
    assert_eq!(view.signature_b64, "abc123");
}

#[test]
fn diversity_summary_row_loads_zero_rows_when_empty() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let rows: Vec<ObservationDiversitySummaryRow> =
        observation_diversity_summary::table.load(&mut conn).expect("load empty view");
    assert!(rows.is_empty());
}

#[test]
fn diversity_summary_view_converts_from_row() {
    let row = ObservationDiversitySummaryRow {
        subject_cid: "doorway:abc".to_string(),
        observation_kind: "infrastructure:doorway-heartbeat".to_string(),
        distinct_agents: 3,
        distinct_households: 2,
        distinct_collectives: 1,
        distinct_regions: 1,
        distinct_archetypes: 2,
        distinct_compute_classes: 1,
        total_count: 5,
        first_observed_at: 1_715_420_000,
        last_observed_at: 1_715_420_400,
    };

    let view: ObservationDiversitySummaryView = row.into();
    assert_eq!(view.subject_cid, "doorway:abc");
    assert_eq!(view.distinct_agents, 3);
    assert_eq!(view.distinct_households, 2);
    assert_eq!(view.total_count, 5);
    assert_eq!(view.first_observed_at, 1_715_420_000);
    assert_eq!(view.last_observed_at, 1_715_420_400);
}
