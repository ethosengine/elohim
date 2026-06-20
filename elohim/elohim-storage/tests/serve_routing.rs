//! Integration tests for `services::serve_routing` — the Wave-3 capability-aware
//! serve-peer selector (D1 byte axis: storage chooses, doorway never).
//!
//! Tests use `test_util::test_pool` (in-memory SQLite, fully migrated) and seed
//! real column names to verify:
//! - T4: `load_serve_rows` returns correct ServeRow fields from the multi-table join
//! - T5: `select_serve_peers` returns capability/diversity-ordered agent_cids,
//!       and an empty fixture → empty result (caller sheds)

use diesel::prelude::*;
use diesel::RunQueryDsl;

use elohim_storage::db::diesel_schema::{
    humans, node_stewardship, rea_commitments, shard_locations, shard_manifests, stewarded_nodes,
};
use elohim_storage::db::models::{
    NewHuman, NewNodeStewardship, NewShardLocation, NewStewardedNode,
};
use elohim_storage::services::serve_routing::{load_serve_rows, select_serve_peers, MIN_CAP};
use elohim_storage::test_util::test_pool;

// ---------------------------------------------------------------------------
// Seed helpers
// ---------------------------------------------------------------------------

const H_APP_ID: &str = "lamad";

fn seed_human(conn: &mut SqliteConnection, id: &str, agent_key: &str, hh: Option<&str>) {
    diesel::insert_into(humans::table)
        .values(&NewHuman {
            id: id.into(),
            agent_pub_key: Some(agent_key.into()),
            display_name: id.into(),
            bio: None,
            affinities: "[]".into(),
            profile_reach: "commons".into(),
            location: None,
            profile_photo_url: None,
            h_app_id: H_APP_ID.into(),
            household_id: hh.map(str::to_string),
        })
        .execute(conn)
        .expect("seed_human");
}

fn seed_node(conn: &mut SqliteConnection, node_id: &str, human_id: &str, cap: Option<i32>) {
    diesel::insert_into(stewarded_nodes::table)
        .values(&NewStewardedNode {
            id: node_id.into(),
            display_name: node_id.into(),
            claim_status: "active".into(),
            cpu_cores: 4,
            memory_gb: 8,
            storage_tb: 1.0,
            bandwidth_mbps: 100,
            steward_tier: "household".into(),
            custodian_opt_in: 1,
            region: None,
            context_epr_id: None,
            dht_anchor_hash: None,
            h_app_id: H_APP_ID.into(),
            device_archetype_id: None,
            household_id: None,
            hostname: None,
            node_role: None,
            capability_level: cap,
            can_steward: 1,
            can_infer: 0,
            can_doorway: 0,
            signature: None,
            signed_at: None,
        })
        .execute(conn)
        .expect("seed_node");

    diesel::insert_into(node_stewardship::table)
        .values(&NewNodeStewardship {
            node_id: node_id.into(),
            human_id: human_id.into(),
            affinity_score: 1.0,
            relationship: "steward".into(),
            context_epr_id: None,
        })
        .execute(conn)
        .expect("seed_node_stewardship");
}

fn seed_shard_manifest(
    conn: &mut SqliteConnection,
    content_id: &str,
    blob_hash: &str,
    shard_hashes: &[&str],
) {
    let hashes_json = serde_json::to_string(shard_hashes).unwrap();
    diesel::insert_into(shard_manifests::table)
        .values((
            shard_manifests::content_id.eq(content_id),
            shard_manifests::h_app_id.eq(H_APP_ID),
            shard_manifests::blob_hash.eq(blob_hash),
            shard_manifests::blob_cid.eq::<Option<&str>>(None),
            shard_manifests::encoding.eq("RS(4,2)"),
            shard_manifests::data_shard_count.eq(4i32),
            shard_manifests::parity_shard_count.eq(2i32),
            shard_manifests::shard_hashes_json.eq(hashes_json.as_str()),
            shard_manifests::total_size_bytes.eq(1024i64),
            shard_manifests::shard_size_bytes.eq(256i64),
            shard_manifests::mime_type.eq("application/octet-stream"),
            shard_manifests::reach.eq("commons"),
        ))
        .execute(conn)
        .expect("seed_shard_manifest");
}

fn seed_shard_location(conn: &mut SqliteConnection, shard_hash: &str, agent_cid: &str) {
    diesel::insert_into(shard_locations::table)
        .values(&NewShardLocation {
            shard_hash,
            peer_id: agent_cid, // shard_locations.peer_id stores agent_cid (uhCAk…)
            h_app_id: H_APP_ID,
            status: "verified",
        })
        .execute(conn)
        .expect("seed_shard_location");
}

fn seed_rea_provide(conn: &mut SqliteConnection, agent_cid: &str) {
    diesel::insert_into(rea_commitments::table)
        .values((
            rea_commitments::id.eq(format!("bond-{agent_cid}")),
            rea_commitments::h_app_id.eq(H_APP_ID),
            rea_commitments::action.eq("provide"),
            rea_commitments::provider.eq(agent_cid),
            rea_commitments::receiver.eq(""),
            rea_commitments::resource_classified_as.eq::<Option<&str>>(None),
            rea_commitments::state.eq("active"),
            rea_commitments::finished.eq(0i32),
            rea_commitments::created_at.eq("2026-01-01T00:00:00Z"),
        ))
        .execute(conn)
        .expect("seed_rea_provide");
}

// ---------------------------------------------------------------------------
// T4: load_serve_rows integration tests
// ---------------------------------------------------------------------------

/// Seed the 3-node, 2-household fixture used in T4 and T5 tests.
/// Returns (blob_hash, [agent_cid_1, agent_cid_2, agent_cid_3]).
fn seed_three_node_fixture(conn: &mut SqliteConnection) -> (String, Vec<String>) {
    let blob_hash = "sha256-deadbeef01234567";
    let shard_a = "shard-aaa";
    let shard_b = "shard-bbb";
    let shard_c = "shard-ccc";

    // Three agents across two households.
    let cid_1 = "uhCAk-agent-1"; // household h1, cap=5, bonded
    let cid_2 = "uhCAk-agent-2"; // household h2, cap=3, bonded
    let cid_3 = "uhCAk-agent-3"; // household h1, cap=2, NOT bonded

    // Humans.
    seed_human(conn, "human-1", cid_1, Some("h1"));
    seed_human(conn, "human-2", cid_2, Some("h2"));
    seed_human(conn, "human-3", cid_3, Some("h1"));

    // Nodes with capability_level.
    seed_node(conn, "node-1", "human-1", Some(5));
    seed_node(conn, "node-2", "human-2", Some(3));
    seed_node(conn, "node-3", "human-3", Some(2));

    // Shard manifest for the blob.
    seed_shard_manifest(conn, "content-x", blob_hash, &[shard_a, shard_b, shard_c]);

    // Shard locations: agent 1 holds A, agent 2 holds B, agent 3 holds C.
    seed_shard_location(conn, shard_a, cid_1);
    seed_shard_location(conn, shard_b, cid_2);
    seed_shard_location(conn, shard_c, cid_3);

    // REA commitments: agents 1 and 2 are bonded; agent 3 is not.
    seed_rea_provide(conn, cid_1);
    seed_rea_provide(conn, cid_2);

    (
        blob_hash.to_string(),
        vec![cid_1.to_string(), cid_2.to_string(), cid_3.to_string()],
    )
}

#[test]
fn load_serve_rows_returns_three_rows_with_correct_fields() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let (blob_hash, cids) = seed_three_node_fixture(&mut conn);

    let rows = load_serve_rows(&mut conn, &blob_hash).expect("load_serve_rows should not fail");

    assert_eq!(rows.len(), 3, "should return one row per holding peer");

    // Build maps for assertion.
    let by_cid: std::collections::HashMap<String, _> =
        rows.into_iter().map(|r| (r.agent_cid.clone(), r)).collect();

    // Agent 1: household h1, cap=5, bonded.
    let r1 = by_cid.get(&cids[0]).expect("agent-1 not in rows");
    assert_eq!(r1.household_id.as_deref(), Some("h1"));
    assert_eq!(r1.capability_level, Some(5));
    assert!(r1.bonded, "agent-1 should be bonded");
    assert_eq!(r1.current_load, None, "current_load not projected this wave");
    assert_eq!(r1.attested_rtt_ms, None, "attested_rtt not projected this wave");
    assert_eq!(r1.delivery_score, None, "delivery_score not projected this wave");

    // Agent 2: household h2, cap=3, bonded.
    let r2 = by_cid.get(&cids[1]).expect("agent-2 not in rows");
    assert_eq!(r2.household_id.as_deref(), Some("h2"));
    assert_eq!(r2.capability_level, Some(3));
    assert!(r2.bonded);

    // Agent 3: household h1, cap=2, NOT bonded.
    let r3 = by_cid.get(&cids[2]).expect("agent-3 not in rows");
    assert_eq!(r3.household_id.as_deref(), Some("h1"));
    assert_eq!(r3.capability_level, Some(2));
    assert!(!r3.bonded, "agent-3 has no active provide commitment");
}

#[test]
fn load_serve_rows_unknown_blob_hash_returns_empty() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    let rows = load_serve_rows(&mut conn, "sha256-does-not-exist")
        .expect("load_serve_rows should not error on unknown hash");
    assert!(rows.is_empty(), "unknown blob → empty (no manifest)");
}

#[test]
fn load_serve_rows_no_shard_locations_returns_empty() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    // Manifest exists but no locations seeded.
    seed_shard_manifest(&mut conn, "content-orphan", "sha256-orphan", &["shard-x"]);

    let rows =
        load_serve_rows(&mut conn, "sha256-orphan").expect("load_serve_rows should not error");
    assert!(rows.is_empty(), "manifest with no locations → empty");
}

// ---------------------------------------------------------------------------
// T5: select_serve_peers integration tests
// ---------------------------------------------------------------------------

#[test]
fn select_serve_peers_returns_two_spread_across_households() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let (blob_hash, cids) = seed_three_node_fixture(&mut conn);

    // Request 2 peers; expect one from each household (h1, h2).
    let chosen =
        select_serve_peers(&mut conn, &blob_hash, 2).expect("select_serve_peers should not fail");

    assert_eq!(chosen.len(), 2, "should return exactly 2 peers");
    // Both selected agents must be in our seeded set.
    for cid in &chosen {
        assert!(cids.contains(cid), "unknown cid in result: {cid}");
    }

    // Verify household diversity: chosen set spans both h1 and h2.
    // Seeded: cid_1=h1, cid_2=h2, cid_3=h1.
    let has_h1 = chosen.contains(&cids[0]) || chosen.contains(&cids[2]);
    let has_h2 = chosen.contains(&cids[1]);
    assert!(has_h1, "selection must include at least one h1 peer");
    assert!(has_h2, "selection must include at least one h2 peer (diverse)");
}

#[test]
fn select_serve_peers_empty_fixture_returns_empty_caller_sheds() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    // No shard manifest → no candidates → empty result (caller sheds, no fanout).
    let chosen = select_serve_peers(&mut conn, "sha256-nothing", 3)
        .expect("select_serve_peers on empty fixture should not error");
    assert!(
        chosen.is_empty(),
        "no eligible candidates → empty → caller sheds (never fans out)"
    );
}

#[test]
fn select_serve_peers_bonded_peer_preferred_over_unbonded_same_household() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let (blob_hash, cids) = seed_three_node_fixture(&mut conn);

    // Request 1 peer.  From h1 we have cid_1 (bonded, cap=5) and cid_3 (unbonded, cap=2).
    // The bonded+higher-cap peer should rank first overall.
    let chosen =
        select_serve_peers(&mut conn, &blob_hash, 1).expect("select_serve_peers should not fail");
    assert_eq!(chosen.len(), 1);
    // Highest-scoring peer: cid_1 (bonded, cap=5, full headroom) should win.
    // cid_2 (bonded, cap=3, h2) is competitive — either may win depending on score tie.
    // The important invariant: cid_3 (unbonded, cap=2) should NOT win if bonded peers are present.
    assert!(
        chosen[0] == cids[0] || chosen[0] == cids[1],
        "cid_3 (unbonded) should not outrank bonded peers; got {chosen:?}"
    );
}

#[test]
fn select_serve_peers_min_cap_zero_never_excludes_peers() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    // Seed a peer with no capability_level in stewarded_nodes (no node row at all —
    // the agent is in shard_locations but not in humans/stewarded_nodes).
    let blob_hash = "sha256-bare-peer";
    seed_shard_manifest(&mut conn, "content-bare", blob_hash, &["shard-bare"]);
    seed_shard_location(&mut conn, "shard-bare", "uhCAk-bare-peer");
    // No humans row for this peer → household_id=None, capability_level=None in ServeRow.
    // fold_candidates maps those to household_id="" and MIN_CAP=0.
    // MIN_CAP=0 means the peer is NOT filtered by capability floor.

    let chosen = select_serve_peers(&mut conn, blob_hash, 1)
        .expect("select_serve_peers with unknown-household peer should not error");
    assert_eq!(
        chosen.len(),
        1,
        "MIN_CAP=0 must not exclude a peer with unknown capability; peer should still be returned"
    );
    assert_eq!(chosen[0], "uhCAk-bare-peer");
}
