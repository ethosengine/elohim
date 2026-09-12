use diesel::RunQueryDsl;
use elohim_storage::db;
use elohim_storage::db::models::{
    NewCollective, NewHuman, NewPlacementGap, NewReaCommitment, NewShardLocation, NewShardManifest,
    NewStewardedNode,
};
use elohim_storage::db::peer_statuses::PeerStatusRow;
use elohim_storage::db::placement_gaps;
use elohim_storage::services::household_resilience;
use elohim_storage::test_util::test_pool;

fn ctx() -> elohim_storage::db::AppContext {
    elohim_storage::db::AppContext {
        h_app_id: "lamad".into(),
        local_libp2p_peer_id: None,
    }
}

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

/// Seed a human with an EXPLICIT `agent_pub_key` distinct from its `id` — needed
/// to reproduce the duplicate-key shape (two humans rows sharing one
/// `agent_pub_key`). `seed_human` sets `agent_pub_key == id`, so it cannot express
/// this case.
fn seed_human_with_key(
    conn: &mut diesel::SqliteConnection,
    id: &str,
    agent_pub_key: &str,
    household_id: Option<&str>,
) {
    diesel::insert_into(db::diesel_schema::humans::table)
        .values(&NewHuman {
            id: id.into(),
            agent_pub_key: Some(agent_pub_key.into()),
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

/// Seed a peer-status projection row (D2 input). `status` is the wire value:
/// "online" | "degraded" | "offline".
fn seed_peer_status(conn: &mut diesel::SqliteConnection, peer_id: &str, status: &str) {
    db::peer_statuses::upsert(
        conn,
        &PeerStatusRow {
            peer_id: peer_id.into(),
            status: status.into(),
            general_pool_member: 0,
            accepting_stewardship_reserves: 0,
            archetype_class: None,
            timestamp: 1,
            dht_anchor_hash: format!("anchor-{peer_id}"),
            updated_at: 1,
        },
    )
    .unwrap();
}

/// Seed the stewarded_nodes row that binds a peer to a household — the join
/// `count_online_peers_in_households` depends on (stewarded_nodes.id ==
/// peer_statuses.peer_id, filtered by household_id).
fn seed_stewarded_node(
    conn: &mut diesel::SqliteConnection,
    peer_id: &str,
    household_id: Option<&str>,
) {
    diesel::insert_into(db::diesel_schema::stewarded_nodes::table)
        .values(&NewStewardedNode {
            id: peer_id.into(),
            display_name: peer_id.into(),
            claim_status: "claimed".into(),
            cpu_cores: 1,
            memory_gb: 1,
            storage_tb: 0.1,
            bandwidth_mbps: 10,
            steward_tier: "household".into(),
            custodian_opt_in: 0,
            region: None,
            context_epr_id: None,
            dht_anchor_hash: None,
            h_app_id: "lamad".into(),
            device_archetype_id: None,
            household_id: household_id.map(str::to_string),
            hostname: None,
            node_role: None,
            capability_level: None,
            can_steward: 1,
            can_infer: 0,
            can_doorway: 0,
            signature: None,
            signed_at: None,
        })
        .execute(conn)
        .unwrap();
}

/// Seed an REA provide commitment (D3 input). `scope` is the
/// resource_classified_as value — the snapshot counts only
/// `content:<reach>`-scoped, `action=provide`, `state=active` rows.
fn seed_commitment(
    conn: &mut diesel::SqliteConnection,
    id: &str,
    provider: &str,
    action: &str,
    state: &str,
    scope: &str,
) {
    diesel::insert_into(db::diesel_schema::rea_commitments::table)
        .values(&NewReaCommitment {
            id,
            h_app_id: "lamad",
            action,
            provider,
            receiver: "commons",
            resource_conforms_to: None,
            resource_classified_as: Some(scope),
            resource_quantity_value: None,
            resource_quantity_unit: None,
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_beginning: None,
            has_end: None,
            due: None,
            clause_of: None,
            in_scope_of: None,
            medium_of_exchange_id: None,
            state,
            finished: 0,
            note: None,
            metadata_json: None,
            dht_anchor_hash: None,
        })
        .execute(conn)
        .unwrap();
}

/// Seed a collective with an optional region (D5 input). `id` must equal the
/// humans.household_id it backs — compute_regional_distribution joins
/// `collectives.id == humans.household_id`.
fn seed_collective(conn: &mut diesel::SqliteConnection, id: &str, region: Option<&str>) {
    diesel::insert_into(db::diesel_schema::collectives::table)
        .values(&NewCollective {
            id,
            h_app_id: "lamad",
            name: id,
            description: None,
            governance_layer: "community",
            constitutional_parent_id: None,
            reach: "commons",
            region,
            metadata_json: None,
            created_by: None,
            collective_cid: None,
            slug: None,
        })
        .execute(conn)
        .unwrap();
}

/// As [`seed_collective`], but stamps the DHT-canonical `collective_cid` — the
/// anchor the convergent household grouping key is derived from.
fn seed_collective_with_cid(
    conn: &mut diesel::SqliteConnection,
    id: &str,
    region: Option<&str>,
    collective_cid: Option<&str>,
) {
    diesel::insert_into(db::diesel_schema::collectives::table)
        .values(&NewCollective {
            id,
            h_app_id: "lamad",
            name: id,
            description: None,
            governance_layer: "family",
            constitutional_parent_id: None,
            reach: "trusted",
            region,
            metadata_json: None,
            created_by: None,
            collective_cid,
            slug: None,
        })
        .execute(conn)
        .unwrap();
}

/// The a2o footprint judge, in Rust.
///
/// Mirrors `genesis/a2o/src/framework/fixtures/household-mesh.ts`
/// `normalizeHouseholdFootprint`: the ONLY things two doorways must agree on are
/// `commitmentBackedCollectives` and the order-insensitive set of holder
/// `kind:id` strings. Everything else in the snapshot (regions, intra-hub counts,
/// placement-gap ids) is a per-host observation the judge deliberately ignores.
fn footprint(snapshot: &elohim_storage::views::ResilienceSnapshotView) -> (i32, Vec<String>) {
    let mut holders: Vec<String> = snapshot
        .details
        .as_ref()
        .expect("snapshot exposes details")
        .stewarding_collectives
        .iter()
        .map(|entry| format!("{}:{}", entry.kind, entry.id))
        .collect();
    holders.sort();
    (snapshot.commitment_backed_collectives, holders)
}

/// D1 fixture builder: `households` lists (household_id, online_peer_count).
/// Each household gets one human stewarding the content's shard (the
/// junction) plus N peers bound via stewarded_nodes with online
/// peer_statuses. Returns the content id.
fn seed_protection_case(
    conn: &mut diesel::SqliteConnection,
    case: &str,
    households: &[(&str, usize)],
) -> String {
    let content_id = format!("content-{case}");
    let shard = format!("shard-{case}");
    seed_shard_manifest(conn, &content_id, &format!(r#"["{shard}"]"#));
    for (h, peers) in households {
        let agent = format!("agent-{case}-{h}");
        seed_human(conn, &agent, Some(h));
        seed_shard_location(conn, &shard, &agent);
        for p in 0..*peers {
            let peer = format!("peer-{case}-{h}-{p}");
            seed_stewarded_node(conn, &peer, Some(h));
            seed_peer_status(conn, &peer, "online");
        }
    }
    content_id
}

/// Seed the lit-card case — 3 coherent agent-keyed households (peer_id == provider
/// == agent_pub_key), each with an online steward + a provide commitment, mixing the
/// seeder `provide` and runtime `replicates-content` action conventions. Returns the
/// content id. Shared by the golden baseline AND the named lit-card proof so their
/// fixtures cannot drift apart (a one-sided seed change would let the byte-golden
/// silently pin a scenario its name no longer describes).
fn seed_lit_card_case(conn: &mut diesel::SqliteConnection) -> &'static str {
    let content_id = "lit-card-content";
    let shard = "lit-card-shard-1";
    seed_shard_manifest(conn, content_id, &format!(r#"["{shard}"]"#));

    // Three households, each with one steward agent. CRITICAL: the steward's
    // shard_locations.peer_id, the commitment.provider, and humans.agent_pub_key
    // are all the SAME value (the agent key) — the coherent contract.
    let households = [
        ("home-dowell", "agent-dowell", Some("us-east")),
        ("home-ruth", "agent-ruth", Some("us-west")),
        ("church-bethel", "agent-bethel", Some("us-east")),
    ];
    for (i, (household, agent, region)) in households.iter().enumerate() {
        seed_collective(conn, household, *region);
        seed_human(conn, agent, Some(household));
        seed_shard_location(conn, shard, agent); // peer_id == agent_pub_key
        seed_peer_status(conn, agent, "online");
        seed_stewarded_node(conn, agent, Some(household));
        // R2 coverage: mix the seeder convention ("provide") with the runtime
        // mishpat-projection convention ("replicates-content") — both must count.
        let action = if i == 0 {
            "replicates-content"
        } else {
            "provide"
        };
        seed_commitment(
            conn,
            &format!("commit-{agent}"),
            agent, // provider == agent_pub_key
            action,
            "active",
            "content:commons",
        );
    }
    content_id
}

/// Seed the intra-hub case — hub `home-multi` with 2 distinct agents, `home-solo`
/// with 1. Returns the content id. Shared by the golden baseline AND the named intra
/// test so their fixtures cannot drift apart.
fn seed_intra_case(conn: &mut diesel::SqliteConnection) -> &'static str {
    let content_id = "content-intra";
    let shard = "shard-intra";
    seed_shard_manifest(conn, content_id, &format!(r#"["{shard}"]"#));

    // hub home-multi: two distinct agents both hold the shard → intra = 2.
    seed_human(conn, "agent-multi-a", Some("home-multi"));
    seed_human(conn, "agent-multi-b", Some("home-multi"));
    seed_shard_location(conn, shard, "agent-multi-a");
    seed_shard_location(conn, shard, "agent-multi-b");
    // hub home-solo: one agent holds → intra = 1.
    seed_human(conn, "agent-solo", Some("home-solo"));
    seed_shard_location(conn, shard, "agent-solo");
    content_id
}

// =============================================================================
// D1 — Protection-status ladder
// (spec: 2026-06-12-resilience-dimensions-proof-suite-design.md)
// protected ← households≥3 AND online≥2; partial ← households≥2 OR online≥1.
// =============================================================================

fn assert_status(households: &[(&str, usize)], expected: &str, case: &str) {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let content_id = seed_protection_case(&mut conn, case, households);
    let view = household_resilience::compute(&pool, &ctx(), &content_id, None).unwrap();
    assert_eq!(
        view.protection_status, expected,
        "case {case}: households={households:?} → expected {expected}, got {} \
         (stewarding={}, online={})",
        view.protection_status, view.households_stewarding, view.details.online_peer_count
    );
}

#[test]
fn d1_no_households_no_peers_is_at_risk() {
    assert_status(&[], "at-risk", "d1-0h0p");
}

#[test]
fn d1_one_household_no_peers_is_at_risk() {
    assert_status(&[("home-a", 0)], "at-risk", "d1-1h0p");
}

#[test]
fn d1_two_households_no_peers_is_partial() {
    assert_status(&[("home-a", 0), ("home-b", 0)], "partial", "d1-2h0p");
}

#[test]
fn d1_one_household_one_peer_is_partial() {
    assert_status(&[("home-a", 1)], "partial", "d1-1h1p");
}

#[test]
fn d1_three_households_one_peer_is_only_partial() {
    // Peers short of the ≥2 floor — must NOT read protected.
    assert_status(
        &[("home-a", 1), ("home-b", 0), ("home-c", 0)],
        "partial",
        "d1-3h1p",
    );
}

#[test]
fn d1_two_households_two_peers_is_only_partial() {
    // Households short of the ≥3 floor — must NOT read protected.
    assert_status(&[("home-a", 1), ("home-b", 1)], "partial", "d1-2h2p");
}

#[test]
fn d1_three_households_two_peers_is_protected() {
    assert_status(
        &[("home-a", 1), ("home-b", 1), ("home-c", 0)],
        "protected",
        "d1-3h2p",
    );
}

#[test]
fn d1_missing_manifest_is_degenerate_at_risk() {
    let pool = test_pool();
    let view = household_resilience::compute(&pool, &ctx(), "content-never-seeded", None).unwrap();
    assert_eq!(view.protection_status, "at-risk");
    assert_eq!(view.households_stewarding, 0);
    assert_eq!(view.details.online_peer_count, 0);
    assert!(view.details.steward_households.is_empty());
}

#[test]
fn d1_missing_manifest_snapshot_is_unmeasured_not_zero() {
    // 2026-06-12 unmeasured≠zero: a content that never entered the
    // distribution plane must declare itself UNMEASURED — the all-zero counts
    // are non-measurements, and renderers must not show a fake at-risk
    // verdict. (Every bulk-seeded content hits this path today.)
    let pool = test_pool();
    let snapshot =
        household_resilience::snapshot(&pool, &ctx(), "content-never-seeded", None).unwrap();
    assert_eq!(snapshot.distribution_state, "unmeasured");
    // The diagnostic tell stays intact: unknown == 0 with all-zeros.
    assert_eq!(snapshot.regional_distribution.unknown, 0);
    let details = snapshot.details.expect("details present");
    assert_eq!(details.online_peers.live, 0);
    assert_eq!(details.online_peers.known, 0);
    // Absent ≠ 0: a shortfall over a non-measurement would fabricate a
    // measurement, so the key is OMITTED on the unmeasured path.
    assert_eq!(snapshot.coverage_shortfall, None);
}

#[test]
fn d1_manifest_present_snapshot_is_measured() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let content_id = seed_protection_case(&mut conn, "d1-measured", &[("home-a", 0)]);
    let snapshot = household_resilience::snapshot(&pool, &ctx(), &content_id, None).unwrap();
    assert_eq!(snapshot.distribution_state, "measured");
    // A MEASURED snapshot always STATES the floor-relative shortfall as a
    // number: one stewarding collective against the "standard" floor of 3.
    assert_eq!(snapshot.coverage_shortfall, Some(2));
}

#[test]
fn d1_measured_at_floor_states_zero_shortfall_not_absent() {
    // 0 is a MEASUREMENT ("the floor is met"), not a missing lens — the
    // consumer distinguishes present-0 from absent, so the key must be there.
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let content_id = seed_protection_case(
        &mut conn,
        "d1-shortfall-met",
        &[("home-a", 1), ("home-b", 1), ("home-c", 0)],
    );
    let snapshot = household_resilience::snapshot(&pool, &ctx(), &content_id, None).unwrap();
    assert_eq!(snapshot.distribution_state, "measured");
    assert_eq!(snapshot.coverage_shortfall, Some(0));
}

// =============================================================================
// D2 — Peer counts: only online|degraded peers, only within stewarding
// households. These pin the INTENDED semantics — list_by_household's
// household filter (C3 column landed; stub returned all peers and
// multiplied counts per household).
// =============================================================================

#[test]
fn d2_degraded_counts_as_online() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let content_id = seed_protection_case(&mut conn, "d2-degraded", &[("home-a", 0)]);
    seed_stewarded_node(&mut conn, "peer-degraded", Some("home-a"));
    seed_peer_status(&mut conn, "peer-degraded", "degraded");
    let view = household_resilience::compute(&pool, &ctx(), &content_id, None).unwrap();
    assert_eq!(view.details.online_peer_count, 1);
}

#[test]
fn d2_offline_peer_does_not_count() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let content_id = seed_protection_case(&mut conn, "d2-offline", &[("home-a", 0)]);
    seed_stewarded_node(&mut conn, "peer-off", Some("home-a"));
    seed_peer_status(&mut conn, "peer-off", "offline");
    let view = household_resilience::compute(&pool, &ctx(), &content_id, None).unwrap();
    assert_eq!(view.details.online_peer_count, 0);
}

#[test]
fn d2_peer_outside_stewarding_households_does_not_count() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let content_id = seed_protection_case(&mut conn, "d2-outside", &[("home-a", 0)]);
    // Online peer bound to a household that does NOT steward this content.
    seed_stewarded_node(&mut conn, "peer-elsewhere", Some("home-zz"));
    seed_peer_status(&mut conn, "peer-elsewhere", "online");
    let view = household_resilience::compute(&pool, &ctx(), &content_id, None).unwrap();
    assert_eq!(
        view.details.online_peer_count, 0,
        "a peer in a non-stewarding household must not count (and must not \
         multiply per stewarding household)"
    );
}

#[test]
fn d2_snapshot_reports_live_over_known_denominator() {
    // Honest denominators: live = online|degraded peers in stewarding
    // households; known = stewarded nodes registered across those households.
    // Tooltip renders "1/2 peers live", never a bare zero.
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let content_id = seed_protection_case(&mut conn, "d2-live-known", &[("home-a", 0)]);
    seed_stewarded_node(&mut conn, "peer-live", Some("home-a"));
    seed_peer_status(&mut conn, "peer-live", "online");
    seed_stewarded_node(&mut conn, "peer-dark", Some("home-a"));
    // peer-dark has NO PeerStatus row — known but not live.
    // A node in a non-stewarding household counts in neither number.
    seed_stewarded_node(&mut conn, "peer-elsewhere", Some("home-zz"));
    let snapshot = household_resilience::snapshot(&pool, &ctx(), &content_id, None).unwrap();
    let details = snapshot.details.expect("details present");
    assert_eq!(details.online_peers.live, 1, "online peer in home-a");
    // AMENDED 2026-09-12: `known` now resolves peers from BOTH junctions —
    // `stewarded_nodes.household_id` (peer-live, peer-dark) AND
    // `humans.agent_pub_key` (the member row `seed_protection_case` laid so that
    // home-a stewards the content). Three distinct peers genuinely belong to
    // home-a; the prior 2 under-counted by ignoring the member junction, which is
    // the same omission that pinned the live mesh at `known: 0`.
    assert_eq!(
        details.online_peers.known, 3,
        "2 stewarded nodes + the stewarding member, deduplicated"
    );
}

// =============================================================================
// D3 — Commitment-backing: distinct households over provide+active rows
// scoped content:<reach> (reach falls back to "commons" with no content row).
// =============================================================================

#[test]
fn d3_commitment_backing_edges() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let content_id = seed_protection_case(
        &mut conn,
        "d3",
        &[("home-a", 0), ("home-b", 0), ("home-c", 0)],
    );

    let agent_a = "agent-d3-home-a"; // seeded by seed_protection_case
    let agent_b = "agent-d3-home-b";
    let agent_c = "agent-d3-home-c";

    // Counts: active provide, content:commons.
    seed_commitment(
        &mut conn,
        "c1",
        agent_a,
        "provide",
        "active",
        "content:commons",
    );
    // Same household second row — still one household.
    seed_commitment(
        &mut conn,
        "c2",
        agent_a,
        "provide",
        "active",
        "content:commons",
    );
    // proposed does NOT count.
    seed_commitment(
        &mut conn,
        "c3",
        agent_b,
        "provide",
        "proposed",
        "content:commons",
    );
    // Wrong scope does NOT count.
    seed_commitment(
        &mut conn,
        "c4",
        agent_c,
        "provide",
        "active",
        "content:community",
    );
    // Provider with no household junction does NOT count.
    seed_human(&mut conn, "agent-d3-ghost", None);
    seed_commitment(
        &mut conn,
        "c5",
        "agent-d3-ghost",
        "provide",
        "active",
        "content:commons",
    );

    let snapshot = household_resilience::snapshot(&pool, &ctx(), &content_id, None).unwrap();
    assert_eq!(
        snapshot.commitment_backed_collectives, 1,
        "only home-a's active provide content:commons row should count"
    );
}

// =============================================================================
// D3b — `resource_classified_as` is a JSON list by contract (non-commons spec
// §11.2, Option A). The scope match is membership over the parsed list, NOT a
// scalar `.eq()`. These are the rows that read the dark card on alpha (U1,
// 2026-06-19): the seeder/HTTP path stores `["content:commons"]` (JSON list),
// which a scalar equality silently misses.
// =============================================================================

/// KEYSTONE: an array-wrapped `["content:commons"]` provide row counts (it did
/// not under the scalar `.eq("content:commons")` — this is the card-lighting fix).
#[test]
fn d3b_json_list_commons_scope_counts() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let content_id = seed_protection_case(&mut conn, "d3b-list", &[("home-a", 0)]);
    let agent_a = "agent-d3b-list-home-a";

    seed_commitment(
        &mut conn,
        "c1",
        agent_a,
        "provide",
        "active",
        r#"["content:commons"]"#,
    );

    let snapshot = household_resilience::snapshot(&pool, &ctx(), &content_id, None).unwrap();
    assert_eq!(
        snapshot.commitment_backed_collectives, 1,
        "an array-wrapped [\"content:commons\"] row must count via membership"
    );
}

/// A list whose only element is a different scope does NOT count (non-membership).
#[test]
fn d3b_json_list_wrong_scope_does_not_count() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let content_id = seed_protection_case(&mut conn, "d3b-wrong", &[("home-a", 0)]);
    let agent_a = "agent-d3b-wrong-home-a";

    seed_commitment(
        &mut conn,
        "c1",
        agent_a,
        "provide",
        "active",
        r#"["content:household"]"#,
    );

    let snapshot = household_resilience::snapshot(&pool, &ctx(), &content_id, None).unwrap();
    assert_eq!(
        snapshot.commitment_backed_collectives, 0,
        "a [\"content:household\"] list must NOT match scope content:commons"
    );
}

/// A multi-element list counts when the scope is one of its members.
#[test]
fn d3b_json_list_multi_element_membership_counts() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let content_id = seed_protection_case(&mut conn, "d3b-multi", &[("home-a", 0)]);
    let agent_a = "agent-d3b-multi-home-a";

    seed_commitment(
        &mut conn,
        "c1",
        agent_a,
        "provide",
        "active",
        r#"["content:household","content:commons"]"#,
    );

    let snapshot = household_resilience::snapshot(&pool, &ctx(), &content_id, None).unwrap();
    assert_eq!(
        snapshot.commitment_backed_collectives, 1,
        "scope content:commons is a member of the multi-element list"
    );
}

/// A provider whose `humans` row carries NULL household_id counts zero —
/// correct-but-dormant honesty (the substrate junction must light it).
#[test]
fn d3b_null_household_id_counts_zero() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let content_id = seed_protection_case(&mut conn, "d3b-null", &[]);

    seed_human(&mut conn, "agent-d3b-null-ghost", None);
    seed_commitment(
        &mut conn,
        "c1",
        "agent-d3b-null-ghost",
        "provide",
        "active",
        r#"["content:commons"]"#,
    );

    let snapshot = household_resilience::snapshot(&pool, &ctx(), &content_id, None).unwrap();
    assert_eq!(
        snapshot.commitment_backed_collectives, 0,
        "a provider with NULL household_id is correct-but-dormant (counts 0)"
    );
}

// =============================================================================
// D4 — Diversity score: REAL fault-domain diversity — the distinct household
// fault-domain count over the same holder-relation, normalized against the RS
// 4+3 baseline (7), clamped 0..1. This REPLACES the prior ad-hoc coverage proxy
// (min(stewarding, max(commitment_backed, 1)) / 7), which falsely capped the
// score by the count of commitment records. The truthful measure counts the
// independent fault domains actually holding the content — commitment paperwork
// no longer gates it.
// =============================================================================

#[test]
fn d4_zero_stewarding_is_zero_diversity() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let content_id = seed_protection_case(&mut conn, "d4-zero", &[]);
    let snapshot = household_resilience::snapshot(&pool, &ctx(), &content_id, None).unwrap();
    assert_eq!(snapshot.diversity_score, 0.0);
}

#[test]
fn d4_three_household_fault_domains_is_three_sevenths() {
    // NEW truthful semantics: 3 distinct household fault domains hold the content
    // → 3/7. Under the OLD commitment-clamped proxy this same case (0 commitments)
    // read 1/7 — the falsely-low value the fault-domain fold corrects. No
    // commitment rows are seeded here on purpose: the score no longer depends on
    // them.
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let content_id = seed_protection_case(
        &mut conn,
        "d4-fault-domains",
        &[("home-a", 0), ("home-b", 0), ("home-c", 0)],
    );
    let snapshot = household_resilience::snapshot(&pool, &ctx(), &content_id, None).unwrap();
    assert!(
        (snapshot.diversity_score - 3.0 / 7.0).abs() < 1e-6,
        "expected 3/7 (3 household fault domains, commitment-independent), got {}",
        snapshot.diversity_score
    );
}

#[test]
fn d4_commitment_count_does_not_lower_diversity() {
    // Explicit regression against the old proxy: 3 household fault domains with
    // ZERO commitment rows must still read 3/7 (not the old 1/7). The diversity
    // score is a function of fault domains, not commitment paperwork.
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let content_id = seed_protection_case(
        &mut conn,
        "d4-no-commit",
        &[("home-a", 0), ("home-b", 0), ("home-c", 0)],
    );
    let snapshot = household_resilience::snapshot(&pool, &ctx(), &content_id, None).unwrap();
    assert_eq!(
        snapshot.commitment_backed_collectives, 0,
        "no commitments seeded"
    );
    assert!(
        snapshot.diversity_score > 1.0 / 7.0 + 1e-6,
        "commitment-free content must NOT be clamped to the old 1/7 floor: got {}",
        snapshot.diversity_score
    );
}

#[test]
fn d4_full_breadth_clamps_at_one() {
    // 8 distinct household fault domains → 8/7 clamps to 1.0 (never over-full).
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let households: Vec<String> = (0..8).map(|i| format!("home-{i}")).collect();
    let pairs: Vec<(&str, usize)> = households.iter().map(|h| (h.as_str(), 0)).collect();
    let content_id = seed_protection_case(&mut conn, "d4-clamp", &pairs);
    let snapshot = household_resilience::snapshot(&pool, &ctx(), &content_id, None).unwrap();
    assert_eq!(snapshot.diversity_score, 1.0);
}

// =============================================================================
// DUP — duplicate agent_pub_key must NOT inflate the holder relation.
// `humans.agent_pub_key` is only non-uniquely indexed (idx_humans_agent_pub_key,
// NOT UNIQUE). A CID-keyed fallback human row can coexist with the canonical
// slug-keyed row (reconcile/controller.rs tolerates the shape), and membership
// projection can stamp the SAME key onto both. One shard_location matching two
// such humans rows must collapse to ONE physical holder (last-write-wins by
// humans.id, consistent with salvage_commitment_author) — never two — or
// distinct_household / intra_hub / diversity silently inflate.
// =============================================================================

#[test]
fn dup_agent_pub_key_different_households_does_not_inflate_diversity() {
    // Worst case for household + diversity inflation: two humans rows, SAME
    // agent_pub_key, different ids AND different households.
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let shared_key = "uhCAkDUP";
    seed_shard_manifest(&mut conn, "content-dup", r#"["shard-dup"]"#);
    seed_human_with_key(
        &mut conn,
        "human-canonical-slug",
        shared_key,
        Some("home-real"),
    );
    seed_human_with_key(
        &mut conn,
        "cid-keyed-fallback",
        shared_key,
        Some("home-phantom"),
    );
    // ONE physical holder (one shard_location keyed by the shared agent key).
    seed_shard_location(&mut conn, "shard-dup", shared_key);

    let snapshot = household_resilience::snapshot(&pool, &ctx(), "content-dup", None).unwrap();
    assert_eq!(
        snapshot.stewarding_collectives,
        1,
        "one physical holder must not inflate to two households: {:?}",
        snapshot.details.as_ref().map(|d| &d.stewarding_collectives)
    );
    assert!(
        (snapshot.diversity_score - 1.0 / 7.0).abs() < 1e-6,
        "diversity must reflect ONE household fault domain (1/7), not two: got {}",
        snapshot.diversity_score
    );
}

#[test]
fn dup_agent_pub_key_same_household_does_not_inflate_intra_hub() {
    // Worst case for intra-hub distinct-agent inflation: two humans rows, SAME
    // agent_pub_key + SAME household, different ids.
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let shared_key = "uhCAkDUP2";
    seed_shard_manifest(&mut conn, "content-dup2", r#"["shard-dup2"]"#);
    seed_human_with_key(
        &mut conn,
        "human-canonical-2",
        shared_key,
        Some("home-shared"),
    );
    seed_human_with_key(&mut conn, "cid-fallback-2", shared_key, Some("home-shared"));
    seed_shard_location(&mut conn, "shard-dup2", shared_key);

    let snapshot = household_resilience::snapshot(&pool, &ctx(), "content-dup2", None).unwrap();
    assert_eq!(
        snapshot.stewarding_collectives, 1,
        "same household, one physical key — still one collective"
    );
    let details = snapshot.details.expect("details present");
    let intra = details
        .stewarding_collectives
        .iter()
        .find(|e| e.id == "home-shared")
        .and_then(|e| e.intra_hub_peers);
    assert_eq!(
        intra,
        Some(1),
        "one physical agent (shared key across two humans rows) must count as 1 \
         intra-hub peer, not 2: {:?}",
        details.stewarding_collectives
    );
}

// =============================================================================
// D5 — Projection: local / regional / global / unknown, household-deduped,
// viewer-relative.
// =============================================================================

/// Two stewarding households with regions; viewer optionally in a region.
fn d5_snapshot(
    case: &str,
    viewer_household: Option<&str>,
    households: &[(&str, Option<&str>)],
) -> elohim_storage::views::ResilienceSnapshotView {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let pairs: Vec<(&str, usize)> = households.iter().map(|(h, _)| (*h, 0usize)).collect();
    let content_id = seed_protection_case(&mut conn, case, &pairs);
    for (h, region) in households {
        seed_collective(&mut conn, h, *region);
    }
    if let Some(vh) = viewer_household {
        // Viewer household exists with its own region row ("us-east").
        seed_collective(&mut conn, vh, Some("us-east"));
    }
    household_resilience::snapshot(&pool, &ctx(), &content_id, viewer_household).unwrap()
}

#[test]
fn d5_no_viewer_steward_with_region_is_global() {
    let s = d5_snapshot("d5-global", None, &[("home-a", Some("eu-west"))]);
    assert_eq!(s.regional_distribution.global, 1);
    assert_eq!(s.regional_distribution.unknown, 0);
}

#[test]
fn d5_no_viewer_no_region_is_unknown() {
    let s = d5_snapshot("d5-unknown", None, &[("home-a", None)]);
    assert_eq!(s.regional_distribution.unknown, 1);
}

#[test]
fn d5_same_region_is_local() {
    let s = d5_snapshot(
        "d5-local",
        Some("home-viewer"),
        &[("home-a", Some("us-east"))],
    );
    assert_eq!(
        s.regional_distribution.local, 1,
        "{:?}",
        s.regional_distribution
    );
}

#[test]
fn d5_different_region_is_regional() {
    let s = d5_snapshot(
        "d5-regional",
        Some("home-viewer"),
        &[("home-a", Some("eu-west"))],
    );
    assert_eq!(
        s.regional_distribution.regional, 1,
        "{:?}",
        s.regional_distribution
    );
}

#[test]
fn d5_viewer_with_region_steward_without_is_unknown() {
    let s = d5_snapshot("d5-vr-unknown", Some("home-viewer"), &[("home-a", None)]);
    assert_eq!(
        s.regional_distribution.unknown, 1,
        "{:?}",
        s.regional_distribution
    );
}

#[test]
fn d5_two_peers_one_household_bucket_once() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let content_id = seed_protection_case(&mut conn, "d5-dedupe", &[("home-a", 0)]);
    // Second peer in the SAME household stewarding the same shard.
    seed_human(&mut conn, "agent-d5-dedupe-2", Some("home-a"));
    seed_shard_location(&mut conn, "shard-d5-dedupe", "agent-d5-dedupe-2");
    seed_collective(&mut conn, "home-a", Some("eu-west"));
    let s = household_resilience::snapshot(&pool, &ctx(), &content_id, None).unwrap();
    assert_eq!(
        s.regional_distribution.global, 1,
        "two stewards in one household must bucket once: {:?}",
        s.regional_distribution
    );
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

// =============================================================================
// INTRA-HUB FOLD — the composability demonstration (resilience-facings design §3).
// The SAME materialized holder-relation yields BOTH the inter-hub count
// (stewardingCollectives, distinct hubs) AND the intra-hub count (distinct agents
// WITHIN a hub) as folds — "a new lens is a new fold over the same relation".
// home-multi has 2 distinct agents holding; home-solo has 1.
// =============================================================================

#[test]
fn intra_hub_peers_counts_distinct_agents_per_hub() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let content_id = seed_intra_case(&mut conn);

    let snapshot = household_resilience::snapshot(&pool, &ctx(), content_id, None).unwrap();

    // Inter-hub fold: 2 distinct hubs steward the content.
    assert_eq!(
        snapshot.stewarding_collectives, 2,
        "inter-hub count = distinct hubs"
    );

    // Intra-hub fold: distinct agents per hub, off the SAME relation.
    let details = snapshot.details.expect("details present");
    let by_id: std::collections::HashMap<&str, Option<i32>> = details
        .stewarding_collectives
        .iter()
        .map(|e| (e.id.as_str(), e.intra_hub_peers))
        .collect();
    assert_eq!(
        by_id.get("home-multi").copied().flatten(),
        Some(2),
        "two distinct agents in home-multi: {by_id:?}"
    );
    assert_eq!(
        by_id.get("home-solo").copied().flatten(),
        Some(1),
        "one agent in home-solo: {by_id:?}"
    );
}

// =============================================================================
// LIT-CARD PROOF — the resilience card lights with coherent agent-keyed substrate.
//
// This is the deterministic proof for the dark-card investigation (2026-06-14):
// the snapshot joins are CORRECT for agent-keyed data; the live card is dark only
// because the runtime writes the libp2p peer_id (not the agent_pub_key) into
// shard_locations.peer_id / rea_commitments.provider, mismatching humans.
// Seeded coherently (peer_id == provider == humans.agent_pub_key), every column
// lights. If this test goes red, the read path regressed.
// =============================================================================

#[test]
fn resilience_card_lights_with_coherent_agent_keyed_substrate() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let content_id = seed_lit_card_case(&mut conn);

    let snapshot = household_resilience::snapshot(&pool, &ctx(), content_id, None).unwrap();

    // distribution_state flips unmeasured -> measured (a manifest exists).
    assert_eq!(snapshot.distribution_state, "measured");
    // Steward join lights — 3 distinct households hold the shard.
    assert_eq!(
        snapshot.stewarding_collectives, 3,
        "steward join must light"
    );
    // Commitment join lights under BOTH action conventions (R2).
    assert_eq!(
        snapshot.commitment_backed_collectives, 3,
        "commitment join must count provide + replicates-content"
    );
    // Protected: >=3 households + >=2 online peers.
    assert_eq!(snapshot.protection_status, "protected");
    // Region data present (no longer "no region data"). With no viewer context,
    // stewards in known regions bucket as `global` (local/regional need a viewer);
    // the card shows real geographic data either way — only all-unknown/zero is dark.
    let rd = &snapshot.regional_distribution;
    assert!(
        rd.local + rd.regional + rd.global > 0,
        "regional distribution must light (non-unknown): {rd:?}"
    );

    let details = snapshot.details.expect("details present");
    assert_eq!(details.stewarding_collectives.len(), 3);
    for entry in &details.stewarding_collectives {
        assert!(
            entry.label.is_some(),
            "each holder must render a name: {entry:?}"
        );
    }
    assert!(
        details.online_peers.live >= 2,
        "online peers: {:?}",
        details.online_peers
    );

    // The felt projection — names, not nines — reads "protected" with all holders.
    let felt = snapshot.felt_status.expect("felt_status present");
    assert_eq!(felt.reassurance, "protected");
    assert_eq!(felt.held_by.len(), 3);
    assert_eq!(felt.floor.has_households, 3);
    assert!(
        felt.headline.starts_with("Held by 3 households:"),
        "headline names the holders: {}",
        felt.headline
    );
}

// =============================================================================
// STOPGAP PROOF — resolver spec §3.4 / §5 step 1
// (genesis/docs/superpowers/specs/2026-06-15-coherent-transport-identity-resolver-design.md)
//
// Reproduces the PRODUCTION shape end-to-end: a slug-keyed human created with
// NULL agent_pub_key (as `seed-humans.ts` writes it) + household_id set, plus an
// active provide commitment whose provider is the truthful uhCAk agent key (as
// `seed-provide-rows.ts` writes it). With agent_pub_key NULL the snapshot's
// direct-equality join (humans.agent_pub_key = rea_commitments.provider) is
// structurally empty → card reads 0. After the STOPGAP `heal_human_identity`
// stamps agent_pub_key = uhCAk, the SAME unchanged join lights. This is the
// whole effect of the stopgap; the join itself is NOT modified (that is
// resolver step 3, out of scope here).
// =============================================================================

#[test]
fn stopgap_heal_lights_commitment_backed_via_direct_join() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    let content_id = "stopgap-content";
    let shard = "stopgap-shard-1";
    seed_shard_manifest(&mut conn, content_id, &format!(r#"["{shard}"]"#));

    let household = "household-dowell";
    let agent_key = "uhCAkMATTHEW"; // the truthful uhCAk key from /auth/me

    // Production human shape: slug id, household_id SET (seed-humans bridge),
    // agent_pub_key NULL (seed-humans sends agentPubKey:null).
    db::humans::create_human(
        &mut conn,
        db::humans::CreateHumanInput {
            id: "human-matthew-manager".into(),
            agent_pub_key: None,
            display_name: "Matthew".into(),
            bio: None,
            affinities: "[]".into(),
            profile_reach: "commons".into(),
            location: None,
            profile_photo_url: None,
            h_app_id: "lamad".into(),
            household_id: Some(household.into()),
        },
    )
    .unwrap();

    // Provider-side row already coherent (seed-provide-rows wrote provider=uhCAk).
    seed_collective(&mut conn, household, Some("us-east"));
    seed_commitment(
        &mut conn,
        "stopgap-commit",
        agent_key, // provider == the uhCAk agent key
        "provide",
        "active",
        "content:commons",
    );

    // BEFORE heal: agent_pub_key is NULL → join empty → column dark.
    let before = household_resilience::snapshot(&pool, &ctx(), content_id, None).unwrap();
    assert_eq!(
        before.commitment_backed_collectives, 0,
        "with agent_pub_key NULL the direct-equality join is structurally empty (the dark-card root cause)"
    );

    // STOPGAP: heal the NULL agent_pub_key to the truthful uhCAk key.
    let healed =
        db::humans::heal_human_identity(&mut conn, "human-matthew-manager", Some(agent_key), None)
            .unwrap();
    assert_eq!(healed.agent_pub_key.as_deref(), Some(agent_key));
    // household_id was already set — heal must not have touched it.
    assert_eq!(healed.household_id.as_deref(), Some(household));

    // AFTER heal: the SAME unchanged join lights commitment_backed.
    let after = household_resilience::snapshot(&pool, &ctx(), content_id, None).unwrap();
    assert!(
        after.commitment_backed_collectives >= 1,
        "after healing agent_pub_key=uhCAk the direct join lights: got {}",
        after.commitment_backed_collectives
    );
}

// =============================================================================
// GOLDEN BYTE-IDENTICAL-JSON BASELINE — the cutover gate for the elohim-facings
// crate extraction (2026-06-19 facings design §11). Captures the serialized
// ResilienceSnapshotView for 3 representative cases. The same 3 strings must
// reproduce across the behavior-preserving refactor (folds → elohim-facings).
//
// Determinism: each helper computes snapshot() TWICE from scratch (fresh queries,
// fresh HashMaps with fresh RandomState) and asserts the serializations are
// byte-identical — catching any HashMap-iteration-order leak into a serialized
// Vec. Cross-PROCESS stability was additionally confirmed by running this binary
// 3× during Phase 0 capture. This is a PERMANENT regression guard.
// =============================================================================

fn det_snapshot_json(pool: &elohim_storage::db::DbPool, content_id: &str, case: &str) -> String {
    let s1 = serde_json::to_string(
        &household_resilience::snapshot(pool, &ctx(), content_id, None).unwrap(),
    )
    .unwrap();
    let s2 = serde_json::to_string(
        &household_resilience::snapshot(pool, &ctx(), content_id, None).unwrap(),
    )
    .unwrap();
    assert_eq!(
        s1, s2,
        "{case}: snapshot JSON is non-deterministic across fresh computations \
         (a HashSet/HashMap order leaked into a serialized Vec — sort before serialize)"
    );
    s1
}

fn golden_lit_card_json() -> String {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let content_id = seed_lit_card_case(&mut conn);
    drop(conn);
    det_snapshot_json(&pool, content_id, "lit-card")
}

fn golden_unmeasured_json() -> String {
    let pool = test_pool();
    det_snapshot_json(&pool, "content-never-seeded", "unmeasured")
}

fn golden_intra_json() -> String {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let content_id = seed_intra_case(&mut conn);
    drop(conn);
    det_snapshot_json(&pool, content_id, "intra-hub")
}

#[test]
fn golden_resilience_snapshot_json_baseline() {
    let lit = golden_lit_card_json();
    let unmeasured = golden_unmeasured_json();
    let intra = golden_intra_json();

    // Byte-identical gate. Captured 2026-06-19 (Phase 0, BEFORE the elohim-facings
    // refactor) and reproduced across 3 fresh processes. The behavior-preserving
    // refactor must reproduce these EXACT serializations; any drift fails here.
    //
    // AMENDED 2026-07-26 — the ONLY sanctioned drift since Phase 0: the trailing
    // `commitmentBackedReplication` block. That fold already ran inside
    // `compute_base` and was dropped on the floor before reaching the wire; it is
    // now surfaced on `ResilienceSnapshotView` (the shape the HTTP route actually
    // serves). Purely ADDITIVE — every pre-existing key and value above is
    // byte-identical to the Phase-0 capture, which is what makes this an honest
    // amendment rather than a rebaseline. Note the lit-card block reads
    // `commonsCommitments: 1` (not zeros): the relation is genuinely folded, not
    // defaulted. The unmeasured/intra cases have no replication commitments, so
    // their blocks are true measured zeros.
    //
    // AMENDED 2026-09-12 — second sanctioned drift, same honesty rule: the
    // `coverageShortfall` key on the two MEASURED goldens (lit-card `0`, intra
    // `1`). The relational path computed the floor comparison all along and hard-
    // set the field to `None`, so `feltStatus.floor` stated the shortfall while
    // the dedicated field withheld it. Present-0 on the lit card is part of the
    // baseline BECAUSE 0 is a measurement ("3 of 3 households, the floor is met")
    // and a reader must be able to tell it from "we never looked" — which is why
    // the UNMEASURED golden keeps NO key at all. Again purely additive: every
    // pre-existing key and value is byte-identical to the Phase-0 capture.
    const GOLDEN_LIT_CARD: &str = r#"{"contentId":"lit-card-content","distributionState":"measured","stewardingCollectives":3,"commitmentBackedCollectives":3,"diversityScore":0.42857143,"regionalDistribution":{"local":0,"regional":0,"global":3,"unknown":0},"placementGaps":[],"protectionStatus":"protected","reciprocatingCollectives":0,"details":{"stewardingCollectives":[{"id":"church-bethel","kind":"household","label":"church-bethel","intraHubPeers":1},{"id":"home-dowell","kind":"household","label":"home-dowell","intraHubPeers":1},{"id":"home-ruth","kind":"household","label":"home-ruth","intraHubPeers":1}],"onlinePeers":{"live":3,"known":3},"healthScore":1.0},"feltStatus":{"headline":"Held by 3 households: church-bethel, home-dowell, home-ruth","reassurance":"protected","heldBy":[{"id":"church-bethel","kind":"household","label":"church-bethel","intraHubPeers":1},{"id":"home-dowell","kind":"household","label":"home-dowell","intraHubPeers":1},{"id":"home-ruth","kind":"household","label":"home-ruth","intraHubPeers":1}],"floor":{"tier":"standard","tierDeclared":false,"wantsHouseholds":3,"hasHouseholds":3}},"coverageShortfall":0,"commitmentBackedReplication":{"dwellingCommitments":0,"collectiveCommitments":0,"commonsCommitments":1,"totalPledgedBytes":0}}"#;
    const GOLDEN_UNMEASURED: &str = r#"{"contentId":"content-never-seeded","distributionState":"unmeasured","stewardingCollectives":0,"commitmentBackedCollectives":0,"diversityScore":0.0,"regionalDistribution":{"local":0,"regional":0,"global":0,"unknown":0},"placementGaps":[],"protectionStatus":"at-risk","reciprocatingCollectives":0,"details":{"stewardingCollectives":[],"onlinePeers":{"live":0,"known":0},"healthScore":0.0},"feltStatus":{"headline":"We can't confirm these are backed up yet","reassurance":"not-yet-seen","heldBy":[],"floor":{"tier":"standard","tierDeclared":false,"wantsHouseholds":3,"hasHouseholds":0},"suggestedAction":"Invite a household to help hold these"},"commitmentBackedReplication":{"dwellingCommitments":0,"collectiveCommitments":0,"commonsCommitments":0,"totalPledgedBytes":0}}"#;
    // AMENDED 2026-09-12 — third sanctioned drift, same honesty rule: the intra
    // golden's `onlinePeers.known` moves 0 → 3. `known` used to read ONLY
    // `stewarded_nodes.household_id`, so a fixture (and a whole live mesh) whose
    // devices were never registered reported zero KNOWN peers for households that
    // demonstrably had members holding shards. It now resolves peers from both
    // junctions — `stewarded_nodes` AND `humans.agent_pub_key` — so the intra
    // case's 3 member peers (2 in home-multi, 1 in home-solo) are counted. `live`
    // stays 0 because the fixture seeds no peer_statuses: known-but-dark is the
    // honest reading, and it is exactly the gap the denominator exists to show.
    // Every other key and value is byte-identical to the Phase-0 capture —
    // `coverageShortfall` (1), `stewardingCollectives` (2), `diversityScore`, and
    // the whole `feltStatus` block are untouched.
    //
    // diversityScore = 2/7 (0.2857143): the intra case has 2 distinct household
    // fault domains (home-multi, home-solo). Under the OLD commitment-clamped proxy
    // this read 1/7 (0.14285715) — 0 commitments capped it; the fault-domain fold
    // corrects it to the real distinct-household count over the RS baseline.
    const GOLDEN_INTRA: &str = r#"{"contentId":"content-intra","distributionState":"measured","stewardingCollectives":2,"commitmentBackedCollectives":0,"diversityScore":0.2857143,"regionalDistribution":{"local":0,"regional":0,"global":0,"unknown":2},"placementGaps":[],"protectionStatus":"partial","reciprocatingCollectives":0,"details":{"stewardingCollectives":[{"id":"home-multi","kind":"household","intraHubPeers":2},{"id":"home-solo","kind":"household","intraHubPeers":1}],"onlinePeers":{"live":0,"known":3},"healthScore":0.0},"feltStatus":{"headline":"Held by 2 of the 3 households this should live in","reassurance":"watching","heldBy":[{"id":"home-multi","kind":"household","intraHubPeers":2},{"id":"home-solo","kind":"household","intraHubPeers":1}],"floor":{"tier":"standard","tierDeclared":false,"wantsHouseholds":3,"hasHouseholds":2}},"coverageShortfall":1,"commitmentBackedReplication":{"dwellingCommitments":0,"collectiveCommitments":0,"commonsCommitments":0,"totalPledgedBytes":0}}"#;

    assert_eq!(
        lit, GOLDEN_LIT_CARD,
        "lit-card snapshot JSON drifted from the Phase-0 golden baseline"
    );
    assert_eq!(
        unmeasured, GOLDEN_UNMEASURED,
        "unmeasured snapshot JSON drifted from the Phase-0 golden baseline"
    );
    assert_eq!(
        intra, GOLDEN_INTRA,
        "intra-hub snapshot JSON drifted from the Phase-0 golden baseline"
    );
}

// ---------------------------------------------------------------------------
// Footprint convergence (2026-09-12 doorway-footprint-convergence)
// ---------------------------------------------------------------------------

/// The DHT-canonical household identity every peer holds — the ONE fact that is
/// genuinely replicated (`p2p::projection_reconcile`'s collectives arm converges
/// exactly this, under whatever local routing alias each peer happens to use).
const DOWELL_CID: &str = "collective:uhCkkD5Z9cTKCB9b2gbuAriONHz9TFBIoqP3WQoxn_dFVTaPMSpnD";
const K_JESSICA: &str = "uhCAkbciV6YEVgcySQXakaQwzGQ0s_YAzNPvWYKt3U09p_fcZLCZq";
const K_JAMES: &str = "uhCAkk2lvx1mkSBfCUlMRhusXMcKNl6-X6e_jXSOnTILn9KHsVEQ3";
const SHARD: &str = "sha256-footprint-shard-1";
const FOOTPRINT_CONTENT: &str = "elohim-host-landing";

/// Seed the DHT-side facts every peer shares: the shard manifest, the custody
/// observation, and one commons provide commitment.
fn seed_shared_dht_facts(conn: &mut diesel::SqliteConnection) {
    seed_shard_manifest(conn, FOOTPRINT_CONTENT, &format!("[\"{SHARD}\"]"));
    seed_shard_location(conn, SHARD, K_JESSICA);
    seed_shard_location(conn, SHARD, K_JAMES);
    seed_commitment(
        conn,
        "commitment-dowell-commons",
        K_JESSICA,
        "provide",
        "active",
        "content:commons",
    );
}

/// **The convergence bar, as a unit test.**
///
/// Three peers hold IDENTICAL DHT facts (one `Collective` cid, one shard
/// manifest, the same two custody observations, the same commons commitment) but
/// the three DIVERGENT local shapes measured on the household mesh on
/// 2026-09-12:
///
/// - **matthew** (the seeding peer) alias-merged the cid onto the seeded
///   `family-dowell` row, so the slug its members actually carry
///   (`household-dowell`) is UN-ANCHORED. All its humans are slug-vocabulary.
/// - **jessica** never had a `family-dowell` row, so `project_collective` minted
///   a cid-KEYED placeholder; the B1 membership gap-fill has since stamped the
///   cid onto `household-dowell` too. One human is slug-vocabulary, the other is
///   the `identity_fill` CREATE shape (`id = agent:{key}`, `household_id = cid`).
/// - **james** is jessica's mirror image, with the agent keys swapped.
///
/// Before the fix these answered `stewardingCollectives` 1 / 2 / 2 and holder
/// sets `{household-dowell}` / `{household-dowell, collective:uhCkk…}` /
/// `{household-dowell, collective:uhCkk…}` — two doorways, two truths, from the
/// same custody facts.
///
/// The fold now groups on the DHT cid and PRESENTS the human-facing slug, so all
/// three testify the same footprint. This covers the migration window too:
/// matthew's bucket is un-anchored and jessica's is cid-keyed, and they still
/// agree, because the presented id is resolved from the household vocabulary the
/// members speak — which is seed data, identical everywhere.
#[test]
fn footprint_converges_across_divergent_local_alias_inventories() {
    // --- peer matthew: cid anchored on `family-dowell`, members on the slug ---
    let matthew_pool = test_pool();
    {
        let mut conn = matthew_pool.get().unwrap();
        seed_collective_with_cid(
            &mut conn,
            "family-dowell",
            Some("tech-valley"),
            Some(DOWELL_CID),
        );
        seed_collective_with_cid(&mut conn, "household-dowell", Some("tech-valley"), None);
        seed_human_with_key(
            &mut conn,
            "human-jessica-spouse",
            K_JESSICA,
            Some("household-dowell"),
        );
        seed_human_with_key(
            &mut conn,
            "human-james-son",
            K_JAMES,
            Some("household-dowell"),
        );
        seed_shared_dht_facts(&mut conn);
    }

    // --- peer jessica: cid-keyed placeholder + B1-stamped slug, mixed humans ---
    let jessica_pool = test_pool();
    {
        let mut conn = jessica_pool.get().unwrap();
        // `idx_collectives_cid_unique` makes ONE household ONE anchored row, so
        // the B1 gap-fill RE-POINTED the cid off the placeholder onto the member
        // slug; the stub survives un-anchored.
        seed_collective_with_cid(&mut conn, DOWELL_CID, None, None);
        seed_collective_with_cid(&mut conn, "household-dowell", None, Some(DOWELL_CID));
        seed_human_with_key(
            &mut conn,
            "human-jessica-spouse",
            K_JESSICA,
            Some("household-dowell"),
        );
        seed_human_with_key(
            &mut conn,
            &format!("agent:{K_JAMES}"),
            K_JAMES,
            Some(DOWELL_CID),
        );
        seed_shared_dht_facts(&mut conn);
    }

    // --- peer james: jessica's mirror (agent keys swapped) ---
    let james_pool = test_pool();
    {
        let mut conn = james_pool.get().unwrap();
        seed_collective_with_cid(&mut conn, DOWELL_CID, None, None);
        seed_collective_with_cid(&mut conn, "household-dowell", None, Some(DOWELL_CID));
        seed_human_with_key(
            &mut conn,
            "human-james-son",
            K_JAMES,
            Some("household-dowell"),
        );
        seed_human_with_key(
            &mut conn,
            &format!("agent:{K_JESSICA}"),
            K_JESSICA,
            Some(DOWELL_CID),
        );
        seed_shared_dht_facts(&mut conn);
    }

    let matthew = household_resilience::snapshot(&matthew_pool, &ctx(), FOOTPRINT_CONTENT, None)
        .expect("matthew snapshot");
    let jessica = household_resilience::snapshot(&jessica_pool, &ctx(), FOOTPRINT_CONTENT, None)
        .expect("jessica snapshot");
    let james = household_resilience::snapshot(&james_pool, &ctx(), FOOTPRINT_CONTENT, None)
        .expect("james snapshot");

    let (m_backed, m_holders) = footprint(&matthew);
    let (j_backed, j_holders) = footprint(&jessica);
    let (k_backed, k_holders) = footprint(&james);

    // The judge's exact comparison — pairwise, all three.
    assert_eq!(
        (m_backed, &m_holders),
        (j_backed, &j_holders),
        "matthew and jessica must testify the same footprint"
    );
    assert_eq!(
        (j_backed, &j_holders),
        (k_backed, &k_holders),
        "jessica and james must testify the same footprint"
    );

    // And it converges on the HUMAN-FACING slug, not a raw cid — the felt
    // surface still reads "names, not nines".
    assert_eq!(
        m_holders,
        vec!["household:household-dowell".to_string()],
        "one physical household, presented under the slug its members carry"
    );
    assert_eq!(
        matthew.stewarding_collectives, 1,
        "matthew folds the household once"
    );
    assert_eq!(
        jessica.stewarding_collectives, 1,
        "jessica's mixed slug/cid vocabularies fold to ONE household, not two"
    );
    assert_eq!(
        james.stewarding_collectives, 1,
        "james's mixed slug/cid vocabularies fold to ONE household, not two"
    );

    // Every peer's shortfall is stated against the same fold, so the
    // floor-relative honesty guard converges with it.
    assert_eq!(matthew.coverage_shortfall, jessica.coverage_shortfall);
    assert_eq!(jessica.coverage_shortfall, james.coverage_shortfall);
    assert_eq!(
        matthew.coverage_shortfall,
        Some(2),
        "1 of the 3-household standard floor"
    );
}

/// **The cascade guard.**
///
/// `stewarded_nodes.household_id` and the `peer_statuses ⋈ stewarded_nodes` join
/// store whichever LOCAL household vocabulary their writer saw — here the SLUG —
/// while the fold now groups on the DHT cid. If the fold fed those joins the bare
/// group key they would match nothing, `onlinePeers.live`/`.known` would read 0,
/// and the verdict would flip to `at-risk` on every peer that anchored its slug.
///
/// This pins the both-ways resolution: a cid-keyed group still finds its
/// slug-keyed nodes.
#[test]
fn online_and_known_peers_survive_the_cid_grouping_key() {
    let pool = test_pool();
    {
        let mut conn = pool.get().unwrap();
        // The slug is ANCHORED, so the fold's group key becomes the cid …
        seed_collective_with_cid(
            &mut conn,
            "household-dowell",
            Some("tech-valley"),
            Some(DOWELL_CID),
        );
        seed_human_with_key(
            &mut conn,
            "human-jessica-spouse",
            K_JESSICA,
            Some("household-dowell"),
        );
        // … while the node rows stay keyed on the SLUG.
        seed_stewarded_node(&mut conn, K_JESSICA, Some("household-dowell"));
        seed_stewarded_node(&mut conn, K_JAMES, Some("household-dowell"));
        seed_peer_status(&mut conn, K_JESSICA, "online");
        seed_peer_status(&mut conn, K_JAMES, "offline");

        seed_shard_manifest(&mut conn, FOOTPRINT_CONTENT, &format!("[\"{SHARD}\"]"));
        seed_shard_location(&mut conn, SHARD, K_JESSICA);
    }

    let snapshot =
        household_resilience::snapshot(&pool, &ctx(), FOOTPRINT_CONTENT, None).expect("snapshot");
    let details = snapshot.details.as_ref().expect("details");

    assert_eq!(
        details.stewarding_collectives.len(),
        1,
        "the anchored slug folds onto the cid as one household"
    );
    assert_eq!(
        details.online_peers.known, 2,
        "known must NOT collapse to 0 — the cid group resolves back to its slug-keyed nodes"
    );
    assert_eq!(
        details.online_peers.live, 1,
        "live must NOT collapse to 0 — one of the two slug-keyed nodes is online"
    );
    assert_eq!(
        snapshot.protection_status, "partial",
        "a live peer keeps the verdict off at-risk"
    );
}

/// A holder in a household with NO cid anywhere must stay its own bucket — two
/// un-anchored households must never merge just because neither is anchored.
#[test]
fn unanchored_households_never_merge_into_one_bucket() {
    let pool = test_pool();
    {
        let mut conn = pool.get().unwrap();
        seed_collective(&mut conn, "home-alpha", None);
        seed_collective(&mut conn, "home-beta", None);
        seed_human_with_key(&mut conn, "human-a", K_JESSICA, Some("home-alpha"));
        seed_human_with_key(&mut conn, "human-b", K_JAMES, Some("home-beta"));
        seed_shard_manifest(&mut conn, FOOTPRINT_CONTENT, &format!("[\"{SHARD}\"]"));
        seed_shard_location(&mut conn, SHARD, K_JESSICA);
        seed_shard_location(&mut conn, SHARD, K_JAMES);
    }

    let snapshot =
        household_resilience::snapshot(&pool, &ctx(), FOOTPRINT_CONTENT, None).expect("snapshot");
    let (_, holders) = footprint(&snapshot);
    assert_eq!(
        holders,
        vec![
            "household:home-alpha".to_string(),
            "household:home-beta".to_string()
        ],
        "two un-anchored households stay two buckets"
    );
    assert_eq!(snapshot.stewarding_collectives, 2);
}

/// **Liveness must be REAL on the household mesh.**
///
/// Measured 2026-09-12: every content answered `onlinePeers {live: 0, known: 0}`
/// while the very same households reported `intraHubPeers: 3`. The counts read
/// only `stewarded_nodes.household_id`, which nothing had populated — three
/// demonstrably online peers reported as zero. That is not a measurement; it is a
/// missing join wearing a measurement's clothes, and it matters because the chaos
/// drill showed custody rows OUTLIVE a killed peer: without honest liveness a
/// household keeps being told "3 copies" after a copy dies.
///
/// Three humans with agent keys in the anchored household, three online
/// peer_statuses rows, NO stewarded_nodes rows at all → live 3, known 3.
#[test]
fn online_peers_are_real_from_the_member_junction_alone() {
    let pool = test_pool();
    let third_key = "uhCAk2avo9u40OVb2v_LZGrCdDSa3_AHP5DgqPMVb6k3YTk7UMFBp";
    {
        let mut conn = pool.get().unwrap();
        seed_collective_with_cid(
            &mut conn,
            "household-dowell",
            Some("tech-valley"),
            Some(DOWELL_CID),
        );
        seed_human_with_key(
            &mut conn,
            "human-jessica-spouse",
            K_JESSICA,
            Some("household-dowell"),
        );
        seed_human_with_key(
            &mut conn,
            "human-james-son",
            K_JAMES,
            Some("household-dowell"),
        );
        // The third member arrives in the CID vocabulary — both fold to one household.
        seed_human_with_key(&mut conn, "agent:matthew", third_key, Some(DOWELL_CID));

        for peer in [K_JESSICA, K_JAMES, third_key] {
            seed_peer_status(&mut conn, peer, "online");
        }
        // Deliberately NO stewarded_nodes rows — the household-mesh shape.

        seed_shard_manifest(&mut conn, FOOTPRINT_CONTENT, &format!("[\"{SHARD}\"]"));
        seed_shard_location(&mut conn, SHARD, K_JESSICA);
    }

    let snapshot =
        household_resilience::snapshot(&pool, &ctx(), FOOTPRINT_CONTENT, None).expect("snapshot");
    let details = snapshot.details.as_ref().expect("details");

    assert_eq!(
        details.online_peers.known, 3,
        "all three members are KNOWN peers of the household, across both vocabularies"
    );
    assert_eq!(
        details.online_peers.live, 3,
        "all three are online — liveness must not read 0 just because no device registered"
    );
}

/// Liveness must DEGRADE when a custody peer dies, even though its custody rows
/// survive it — the signal the chaos cascade drill needs so the badge can drop
/// from "3 copies" to an honest partial.
#[test]
fn online_peers_degrade_when_a_custody_peer_goes_offline() {
    let pool = test_pool();
    {
        let mut conn = pool.get().unwrap();
        seed_collective_with_cid(
            &mut conn,
            "household-dowell",
            Some("tech-valley"),
            Some(DOWELL_CID),
        );
        seed_human_with_key(
            &mut conn,
            "human-jessica-spouse",
            K_JESSICA,
            Some("household-dowell"),
        );
        seed_human_with_key(&mut conn, "human-james-son", K_JAMES, Some(DOWELL_CID));
        seed_peer_status(&mut conn, K_JESSICA, "online");
        // James was killed — his peer_status flipped, but his custody row remains.
        seed_peer_status(&mut conn, K_JAMES, "offline");

        seed_shard_manifest(&mut conn, FOOTPRINT_CONTENT, &format!("[\"{SHARD}\"]"));
        seed_shard_location(&mut conn, SHARD, K_JESSICA);
        seed_shard_location(&mut conn, SHARD, K_JAMES);
    }

    let snapshot =
        household_resilience::snapshot(&pool, &ctx(), FOOTPRINT_CONTENT, None).expect("snapshot");
    let details = snapshot.details.as_ref().expect("details");

    assert_eq!(
        details.stewarding_collectives[0].intra_hub_peers,
        Some(2),
        "custody survives the kill — the household still HOLDS two copies"
    );
    assert_eq!(
        details.online_peers.known, 2,
        "both peers stay KNOWN — the denominator does not shrink when one dies"
    );
    assert_eq!(
        details.online_peers.live, 1,
        "only the survivor is LIVE — this is the signal the badge degrades on"
    );
}
