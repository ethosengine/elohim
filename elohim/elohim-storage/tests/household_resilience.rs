use diesel::RunQueryDsl;
use elohim_storage::db;
use elohim_storage::db::models::{NewHuman, NewShardLocation};
use elohim_storage::services::household_resilience;
use elohim_storage::test_util::test_pool;

fn seed_human(conn: &mut diesel::SqliteConnection, id: &str, household_id: Option<&str>) {
    diesel::insert_into(db::diesel_schema::humans::table)
        .values(&NewHuman {
            id: id.into(), agent_pub_key: Some(id.into()), display_name: id.into(),
            bio: None, affinities: "[]".into(), profile_reach: "commons".into(),
            location: None, profile_photo_url: None, h_app_id: "lamad".into(),
            household_id: household_id.map(str::to_string),
        })
        .execute(conn)
        .unwrap();
}

fn seed_shard_location(conn: &mut diesel::SqliteConnection, shard_hash: &str, peer_id: &str) {
    let loc = NewShardLocation {
        shard_hash, peer_id, h_app_id: "lamad", status: "announced",
    };
    db::shard_locations::upsert_location(conn, &loc).unwrap();
}

#[test]
fn distinct_households_counted_from_shard_locations() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    seed_human(&mut conn, "agent-alpha-1", Some("home-alpha"));
    seed_human(&mut conn, "agent-alpha-2", Some("home-alpha")); // same household
    seed_human(&mut conn, "agent-beta-1",  Some("home-beta"));
    seed_human(&mut conn, "agent-ghost",   None);

    seed_shard_location(&mut conn, "shard-x", "agent-alpha-1");
    seed_shard_location(&mut conn, "shard-x", "agent-alpha-2");
    seed_shard_location(&mut conn, "shard-x", "agent-beta-1");
    seed_shard_location(&mut conn, "shard-x", "agent-ghost");

    // Minimal ctx + content: most real services take AppContext + content_id.
    // The function under test should aggregate distinct households for content
    // "content-via-shard-x" whose shards include "shard-x".
    // This test pins the expected household count = 2 (alpha + beta);
    // the agent-ghost should not count.
    let view = household_resilience::compute(
        &pool,
        &elohim_storage::db::AppContext { h_app_id: "lamad".into() },
        "content-via-shard-x",
        None,
    ).unwrap();

    assert_eq!(view.households_stewarding, 2);
}
