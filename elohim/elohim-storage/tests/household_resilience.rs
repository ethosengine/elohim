use diesel::RunQueryDsl;
use elohim_storage::db;
use elohim_storage::db::models::{NewHuman, NewPlacementGap, NewShardLocation, NewShardManifest};
use elohim_storage::db::placement_gaps;
use elohim_storage::services::household_resilience;
use elohim_storage::test_util::test_pool;

fn seed_human(conn: &mut diesel::SqliteConnection, id: &str, household_id: Option<&str>) {
    diesel::insert_into(db::diesel_schema::humans::table)
        .values(&NewHuman {
            id: id.into(),
            agent_pub_key: Some(id.into()),
            display_name: id.into(),
            bio: None,
            affinities: "[]".into(),
            profile_reach: "commons".into(),
            location: None,
            profile_photo_url: None,
            h_app_id: "lamad".into(),
            household_id: household_id.map(str::to_string),
        })
        .execute(conn)
        .unwrap();
}

fn seed_shard_location(conn: &mut diesel::SqliteConnection, shard_hash: &str, peer_id: &str) {
    let loc = NewShardLocation {
        shard_hash,
        peer_id,
        h_app_id: "lamad",
        status: "announced",
    };
    db::shard_locations::upsert_location(conn, &loc).unwrap();
}

fn seed_shard_manifest(
    conn: &mut diesel::SqliteConnection,
    content_id: &str,
    shard_hashes_json: &str,
) {
    let manifest = NewShardManifest {
        content_id,
        h_app_id: "lamad",
        blob_hash: "blob-hash-stub",
        blob_cid: None,
        encoding: "identity",
        data_shard_count: 1,
        parity_shard_count: 0,
        shard_hashes_json,
        total_size_bytes: 0,
        shard_size_bytes: 0,
        mime_type: "application/octet-stream",
        reach: "commons",
    };
    db::shard_manifests::upsert_manifest(conn, &manifest).unwrap();
}

#[test]
fn distinct_households_counted_from_shard_locations() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    seed_human(&mut conn, "agent-alpha-1", Some("home-alpha"));
    seed_human(&mut conn, "agent-alpha-2", Some("home-alpha")); // same household
    seed_human(&mut conn, "agent-beta-1", Some("home-beta"));
    seed_human(&mut conn, "agent-ghost", None);

    seed_shard_location(&mut conn, "shard-x", "agent-alpha-1");
    seed_shard_location(&mut conn, "shard-x", "agent-alpha-2");
    seed_shard_location(&mut conn, "shard-x", "agent-beta-1");
    seed_shard_location(&mut conn, "shard-x", "agent-ghost");

    // Seed the manifest so compute() follows the two-step manifest path:
    // get_manifest -> parse shard_hashes_json -> filter shard_locations by eq_any.
    // Without this, the prior fallback would aggregate across all shard_locations
    // for the h_app_id (inflated count), not the shards belonging to this content.
    seed_shard_manifest(&mut conn, "content-via-shard-x", r#"["shard-x"]"#);

    // Minimal ctx + content: most real services take AppContext + content_id.
    // The function under test should aggregate distinct households for content
    // "content-via-shard-x" whose shards include "shard-x".
    // This test pins the expected household count = 2 (alpha + beta);
    // the agent-ghost should not count.
    let view = household_resilience::compute(
        &pool,
        &elohim_storage::db::AppContext {
            h_app_id: "lamad".into(),
            local_libp2p_peer_id: None,
        },
        "content-via-shard-x",
        None,
    )
    .unwrap();

    assert_eq!(view.households_stewarding, 2);
}

#[test]
fn snapshot_includes_placement_gaps_and_regional_distribution() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    seed_human(&mut conn, "agent-alpha-1", Some("home-alpha"));
    seed_human(&mut conn, "agent-beta-1", Some("home-beta"));
    seed_shard_location(&mut conn, "shard-x", "agent-alpha-1");
    seed_shard_location(&mut conn, "shard-x", "agent-beta-1");

    // Seed the manifest so snapshot() follows the two-step manifest path.
    seed_shard_manifest(&mut conn, "content-via-shard-x", r#"["shard-x"]"#);

    // Insert a known placement_gap row for this content.
    placement_gaps::upsert_gap(
        &mut conn,
        &NewPlacementGap {
            id: "g1",
            content_id: "content-via-shard-x",
            shard_hash: "shard-y",
            h_app_id: "lamad",
            requested_steward_count: 3,
            achieved_steward_count: 0,
            contract_coverage: 0.0,
            gap_kind: "peers-unavailable",
            first_seen_at: "2026-04-19T00:00:00Z",
            last_seen_at: "2026-04-19T00:00:00Z",
        },
    )
    .unwrap();

    let snapshot = household_resilience::snapshot(
        &pool,
        &elohim_storage::db::AppContext {
            h_app_id: "lamad".into(),
            local_libp2p_peer_id: None,
        },
        "content-via-shard-x",
        None,
    )
    .unwrap();

    assert_eq!(snapshot.stewarding_collectives, 2);
    assert_eq!(snapshot.commitment_backed_collectives, 0); // no rea_commitments seeded
    assert_eq!(snapshot.placement_gaps.len(), 1);
    assert_eq!(snapshot.placement_gaps[0].gap_kind, "peers-unavailable");
    // No region data seeded: both households bucketed as unknown
    assert_eq!(snapshot.regional_distribution.unknown, 2);
    assert_eq!(
        snapshot.regional_distribution.local
            + snapshot.regional_distribution.regional
            + snapshot.regional_distribution.global,
        0
    );
}
