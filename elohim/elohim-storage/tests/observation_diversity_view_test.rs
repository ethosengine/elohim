//! Smoke test: the diversity view is queryable and aggregates correctly when
//! rows are inserted into `observations`.
//!
//! Uses `elohim_storage::test_util::test_pool` — an in-memory SQLite pool with
//! all migrations applied, isolated per test call.

use diesel::prelude::*;
use diesel::RunQueryDsl;
use elohim_storage::db::diesel_schema::{observation_diversity_summary, observations};
use elohim_storage::test_util::test_pool;

#[test]
fn diversity_view_returns_zero_rows_when_observations_empty() {
    let pool = test_pool();
    let mut conn = pool.get().expect("test connection");

    let rows: Vec<(String, String, i64)> = observation_diversity_summary::table
        .select((
            observation_diversity_summary::subject_cid,
            observation_diversity_summary::observation_kind,
            observation_diversity_summary::distinct_agents,
        ))
        .load(&mut conn)
        .expect("query empty view");

    assert!(rows.is_empty());
}

#[test]
fn diversity_view_aggregates_households() {
    let pool = test_pool();
    let mut conn = pool.get().expect("test connection");

    // Three observations: agents 0,1,2 — agents 0 and 1 share household h1,
    // agent 2 is in household h2. All on the same subject+kind.
    let cases: &[(&str, &str)] = &[
        ("agent:test-0", "h1"),
        ("agent:test-1", "h1"),
        ("agent:test-2", "h2"),
    ];

    for (i, (agent_cid, household)) in cases.iter().enumerate() {
        diesel::insert_into(observations::table)
            .values((
                observations::observer_cid.eq(*agent_cid),
                observations::log_cid.eq("blake3:test"),
                observations::log_offset.eq(i as i64),
                observations::observed_at.eq(1_715_420_400_i64 + i as i64),
                observations::seq.eq(i as i64),
                observations::observation_kind.eq("infrastructure:doorway-heartbeat"),
                observations::subject_cid.eq(Some("doorway:abc")),
                observations::subject_kind.eq(Some("doorway")),
                observations::payload_json.eq("{}"),
                observations::observer_household_cid.eq(Some(format!("household:{}", household))),
                observations::observer_collective_cid.eq::<Option<String>>(None),
                observations::observer_region.eq::<Option<String>>(None),
                observations::observer_archetype.eq::<Option<String>>(None),
                observations::observer_compute_class.eq::<Option<String>>(None),
                observations::signature_b64.eq(""),
            ))
            .execute(&mut conn)
            .expect("insert observation");
    }

    let (agents, households, total): (i64, i64, i64) = observation_diversity_summary::table
        .filter(observation_diversity_summary::subject_cid.eq("doorway:abc"))
        .filter(
            observation_diversity_summary::observation_kind
                .eq("infrastructure:doorway-heartbeat"),
        )
        .select((
            observation_diversity_summary::distinct_agents,
            observation_diversity_summary::distinct_households,
            observation_diversity_summary::total_count,
        ))
        .first(&mut conn)
        .expect("query view");

    assert_eq!(agents, 3, "three distinct observer_cids");
    assert_eq!(households, 2, "two distinct households");
    assert_eq!(total, 3, "three total rows");
}
