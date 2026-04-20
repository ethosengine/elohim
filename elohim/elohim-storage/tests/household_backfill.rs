use diesel::prelude::*;

use elohim_storage::db;
use elohim_storage::db::models::NewHuman;
use elohim_storage::services::household_backfill;
use elohim_storage::test_util::test_pool;

#[test]
fn backfill_fills_null_household_ids_from_dht_map() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    // Pre-populate humans — one with household_id set, one null.
    diesel::insert_into(db::diesel_schema::humans::table)
        .values(&NewHuman {
            id: "h-adam".into(),
            agent_pub_key: None,
            display_name: "Adam".into(),
            bio: None,
            affinities: "[]".into(),
            profile_reach: "commons".into(),
            location: None,
            profile_photo_url: None,
            h_app_id: "lamad".into(),
            household_id: None,
        })
        .execute(&mut conn)
        .unwrap();

    diesel::insert_into(db::diesel_schema::humans::table)
        .values(&NewHuman {
            id: "h-eve".into(),
            agent_pub_key: None,
            display_name: "Eve".into(),
            bio: None,
            affinities: "[]".into(),
            profile_reach: "commons".into(),
            location: None,
            profile_photo_url: None,
            h_app_id: "lamad".into(),
            household_id: Some("eden".into()),
        })
        .execute(&mut conn)
        .unwrap();

    // Simulated DHT mapping of humanId -> householdId.
    let mapping = vec![("h-adam".to_string(), "eden".to_string())];

    let filled = household_backfill::run_once(&pool, mapping).unwrap();
    assert_eq!(filled, 1);

    let adam = db::humans::get_human_by_id(&mut conn, "h-adam")
        .unwrap()
        .unwrap();
    assert_eq!(adam.household_id.as_deref(), Some("eden"));

    let eve = db::humans::get_human_by_id(&mut conn, "h-eve")
        .unwrap()
        .unwrap();
    assert_eq!(eve.household_id.as_deref(), Some("eden")); // untouched
}

#[test]
fn backfill_ignores_missing_humans() {
    let pool = test_pool();
    let mapping = vec![("ghost".into(), "nowhere".into())];
    let filled = household_backfill::run_once(&pool, mapping).unwrap();
    assert_eq!(filled, 0);
}
