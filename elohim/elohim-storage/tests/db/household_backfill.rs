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

/// Helper: insert a slug-keyed human whose membership match must go through the
/// agent_pub_key arm (production shape — `humans.id` is a slug, not the cid).
fn insert_human(
    conn: &mut diesel::SqliteConnection,
    id: &str,
    agent_pub_key: Option<&str>,
    household_id: Option<&str>,
) {
    use diesel::prelude::*;
    diesel::insert_into(db::diesel_schema::humans::table)
        .values(&NewHuman {
            id: id.into(),
            agent_pub_key: agent_pub_key.map(|s| s.to_string()),
            display_name: id.into(),
            bio: None,
            affinities: "[]".into(),
            profile_reach: "commons".into(),
            location: None,
            profile_photo_url: None,
            h_app_id: "lamad".into(),
            household_id: household_id.map(|s| s.to_string()),
        })
        .execute(conn)
        .unwrap();
}

/// The DHT replayer emits `(member_cid = "agent:<key>", household_cid)`. The
/// dual-match variant must stamp a production-shaped (slug-id) human via the
/// agent_pub_key arm — the id-only `run_once` would miss it.
#[test]
fn by_membership_stamps_via_agent_pub_key_arm() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    // Slug id, raw pubkey, null household — must be stamped from the membership.
    insert_human(&mut conn, "human-alice-n1", Some("uhCAkAliceN1"), None);

    // member_cid carries the `agent:` prefix, as the Membership entry encodes it.
    let mapping = vec![(
        "agent:uhCAkAliceN1".to_string(),
        "collective:uhCkkHousehold1".to_string(),
    )];

    let filled = household_backfill::run_once_by_membership(&pool, mapping).unwrap();
    assert_eq!(filled, 1);

    let alice = db::humans::get_human_by_id(&mut conn, "human-alice-n1")
        .unwrap()
        .unwrap();
    assert_eq!(
        alice.household_id.as_deref(),
        Some("collective:uhCkkHousehold1")
    );
}

/// Composition guarantee: the dual-match variant must NEVER overwrite a human
/// who already carries a household_id (create-time seed or live-stamped value),
/// even when the membership mapping references them by agent key.
#[test]
fn by_membership_never_overwrites_existing_household_id() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    // Already in a household via the create-time bridge / live signal.
    insert_human(
        &mut conn,
        "human-bob-n1",
        Some("uhCAkBobN1"),
        Some("collective:original-home"),
    );

    // The replayer reports a (contradictory) different household for the same
    // member — both the id arm and the agent-key arm must be NULL-guarded.
    let mapping = vec![
        (
            "agent:uhCAkBobN1".to_string(),
            "collective:wrong-home".to_string(),
        ),
        (
            "human-bob-n1".to_string(),
            "collective:also-wrong".to_string(),
        ),
    ];

    let filled = household_backfill::run_once_by_membership(&pool, mapping).unwrap();
    assert_eq!(filled, 0, "non-null household_id must be left untouched");

    let bob = db::humans::get_human_by_id(&mut conn, "human-bob-n1")
        .unwrap()
        .unwrap();
    assert_eq!(
        bob.household_id.as_deref(),
        Some("collective:original-home"),
        "original household_id must survive"
    );
}

/// The id arm still works (some humans ARE keyed by the member_cid directly),
/// and an unknown member is a no-op.
#[test]
fn by_membership_matches_id_arm_and_skips_unknown() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    // Human keyed by the bare member_cid (no agent_pub_key set).
    insert_human(&mut conn, "agent:uhCAkCarol", None, None);

    let mapping = vec![
        (
            "agent:uhCAkCarol".to_string(),
            "collective:carol-home".to_string(),
        ),
        // Unknown member — no row matches either arm.
        (
            "agent:uhCAkGhost".to_string(),
            "collective:nowhere".to_string(),
        ),
    ];

    let filled = household_backfill::run_once_by_membership(&pool, mapping).unwrap();
    assert_eq!(filled, 1);

    let carol = db::humans::get_human_by_id(&mut conn, "agent:uhCAkCarol")
        .unwrap()
        .unwrap();
    assert_eq!(carol.household_id.as_deref(), Some("collective:carol-home"));
}
