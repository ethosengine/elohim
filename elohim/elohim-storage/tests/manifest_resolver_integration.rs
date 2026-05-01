//! T14 — Manifest resolver integration tests (EPR Phase 3 / Phase 3.5).
//!
//! Four scenarios:
//!
//! 1. `manifest_creation_projects_to_registry` — Manifest EPR authored + ingested
//!    via FederatedEprStore::put; registry loaded from DB returns expected pillar
//!    mapping.
//!
//! 2. `cold_fetch_resolves_manifest_from_peer` — Cross-peer manifest cold-fetch.
//!    Phase 3.5: lifted from #[ignore] using the existing harness_d8 infrastructure.
//!    Two D8Nodes are spawned; node_a holds a Manifest EPR; node_b cold-fetches via
//!    FederatedEprStore wired to node_b's swarm. The test exercises the full
//!    KadGetProviders → FetchEprAtomFromPeer → EprAtomRequest::Fetch round-trip.
//!
//! 3. `floor_cid_lookup_unconditional_at_unknown_standing` — CID lookup via
//!    LocalEprStore::fetch succeeds regardless of standing; floor protection
//!    verified by fetching after ingest at Standing::Unknown.
//!
//! 4. `floor_protocol_load_bearing_schemaref_full_depth` — walk_schemaref with
//!    EprKind::Manifest walks 6-deep chain even at StandingScore::Floor (depth
//!    limit 3 for non-Manifest kinds).  Uses direct DB seeding (ManifestRow
//!    inserts) to avoid complex cross-CID chaining through EPR builder.

mod harness_d8;

use elohim_epr::{cid::compute_cid, proof::AgentKeypair, Coupling, EprKind, Reach};
use elohim_storage::db::manifests::{insert_manifest, ManifestRow};
use elohim_storage::services::epr_service::ingest;
use elohim_storage::services::epr_store::{EprStore, FederatedEprStore, LocalEprStore};
use elohim_storage::services::manifest_registry::ManifestRegistry;
use elohim_storage::services::schemaref_resolver::walk_schemaref;
use elohim_storage::services::standing::{Standing, StandingScore};
use elohim_storage::test_util::test_pool;
use harness_d8::{connect, spawn_d8_node};
use std::time::Duration;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Build a minimal signed Manifest EPR with a pillar-projection payload.
///
/// The Manifest kind requires only the `governance` coupling leg, so we set
/// that leg to a deterministic CID.  The payload encodes a pillar-projection
/// manifest shape: `{ "pillar": <pillar>, "manifestKind": "pillar-projection",
/// "kinds": [<kinds…>] }`.
fn build_manifest_epr(pillar: &str, kinds: &[&str]) -> elohim_epr::Epr {
    let seed = {
        let mut s = [0u8; 32];
        let b = pillar.as_bytes();
        let len = b.len().min(32);
        s[..len].copy_from_slice(&b[..len]);
        s
    };
    let kp = AgentKeypair::from_secret(&seed).expect("agent keypair");
    let signer_cid = compute_cid(pillar.as_bytes());
    let gov = compute_cid(b"test-governance-manifest-resolver");
    let schema_ref_cid = compute_cid(b"test-schema-ref-manifest-resolver");
    let coupling = Coupling {
        knowledge: None,
        value: None,
        governance: Some(gov),
    };
    let payload = serde_json::json!({
        "manifestKind": "pillar-projection",
        "pillar": pillar,
        "kinds": kinds,
    })
    .to_string()
    .into_bytes();

    elohim_epr::Epr::builder()
        .kind(EprKind::Manifest)
        .schema_ref(schema_ref_cid)
        .schema_key("elohim/manifest/pillar-projection")
        .reach(Reach::Commons)
        .coupling(coupling)
        .issued_at(chrono::Utc::now())
        .payload(payload)
        .sign(&kp, signer_cid)
        .expect("sign manifest epr")
}

/// Build a minimal ManifestRow for direct DB insertion (schemaRef chain tests).
fn chain_row(cid: &str, next: Option<&str>) -> ManifestRow {
    ManifestRow {
        cid: cid.to_string(),
        manifest_kind: "pillar-projection".to_string(),
        pillar: Some("lamad".to_string()),
        payload_json: "{}".to_string(),
        schema_ref: next.map(String::from),
        signer_pubkey: vec![0u8; 32],
        created_at: "2026-04-30T00:00:00Z".to_string(),
        verified_at: Some("2026-04-30T00:00:00Z".to_string()),
        revision: 1,
    }
}

// ---------------------------------------------------------------------------
// Test 1: manifest_creation_projects_to_registry
//
// Author a pillar-projection Manifest EPR for "lamad" with kinds=[Content,
// Observation], put it via FederatedEprStore, load ManifestRegistry from DB,
// assert 2 kind→pillar mappings; pillar_for_kind(Content, Unknown) == "lamad".
// ---------------------------------------------------------------------------

#[tokio::test]
async fn manifest_creation_projects_to_registry() {
    let pool = test_pool();
    let mut conn = pool.get().expect("connection");

    // Author + put a Manifest EPR for lamad covering Content + Observation.
    let epr = build_manifest_epr("lamad", &["Content", "Observation"]);
    let expected_cid = epr.envelope.cid.to_string();

    let store = FederatedEprStore::new(); // no swarm needed for local put
    let result = store.put(&mut conn, epr).expect("put must succeed");
    assert_eq!(
        result.cid, expected_cid,
        "put result CID must match authored EPR CID"
    );

    // Load the registry from the manifests projection table.
    let registry = ManifestRegistry::new();
    let loaded = registry
        .load_from_db(&mut conn)
        .expect("load_from_db must succeed");

    // pillar-projection for lamad has 2 kinds: Content + Observation.
    assert_eq!(
        loaded, 2,
        "registry must load 2 kind→pillar mappings (content, observation)"
    );
    assert!(
        !registry.is_empty(),
        "registry must not be empty after load"
    );

    // Floor assertion: CID lookup is unconditional; pillar resolve ignores standing.
    let pillar = registry.pillar_for_kind(EprKind::Content, Standing::Unknown);
    assert_eq!(
        pillar,
        Some("lamad".to_string()),
        "pillar_for_kind(Content, Unknown) must return Some(lamad)"
    );

    let pillar_obs = registry.pillar_for_kind(EprKind::Observation, Standing::Unknown);
    assert_eq!(
        pillar_obs,
        Some("lamad".to_string()),
        "pillar_for_kind(Observation, Unknown) must return Some(lamad)"
    );

    // Manifest kind itself is not declared in the payload's kinds array.
    let pillar_manifest = registry.pillar_for_kind(EprKind::Manifest, Standing::Unknown);
    assert!(
        pillar_manifest.is_none(),
        "pillar_for_kind(Manifest, Unknown) must return None (not in kinds array)"
    );
}

// ---------------------------------------------------------------------------
// Test 2: cold_fetch_resolves_manifest_from_peer
//
// Phase 3.5: real cross-peer harness lifts the ignore.
//
// Two D8Nodes (node_a = author, node_b = cold-fetcher) are wired together
// via the existing harness_d8 infrastructure.  Node_a ingests a Manifest EPR
// and publishes a Kad provider record.  Node_b's FederatedEprStore::fetch
// exercises the full KadGetProviders → FetchEprAtomFromPeer → EprAtomRequest::
// Fetch round-trip.  Harness_d8 was extended (T19) to handle FetchEprAtomFromPeer
// P2PCommands and serve incoming EprAtomRequest::Fetch from the node's local DB.
//
// Design: harness_d8 reuse instead of a new multi_peer.rs module.  The
// harness_d8 already has Kad + Gossipsub + request-response (683 lines); a
// parallel module would duplicate working code (YAGNI).
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cold_fetch_resolves_manifest_from_peer() {
    // -----------------------------------------------------------------------
    // Step 1: spawn two nodes and connect them.
    // -----------------------------------------------------------------------
    let node_a = spawn_d8_node("t19-a", &[]).await;
    let node_b = spawn_d8_node("t19-b", &[]).await;

    connect(&node_a, &node_b).await;

    // -----------------------------------------------------------------------
    // Step 2: author a Manifest EPR on node_a and ingest it into node_a's DB.
    // -----------------------------------------------------------------------
    let epr = build_manifest_epr("shefa", &["EconomicEvent", "Stewardship"]);
    let expected_cid = epr.envelope.cid.to_string();

    {
        let pool_a = node_a.db_pool.clone();
        let mut conn_a = pool_a.get().expect("node_a db connection");
        ingest(&mut conn_a, epr).expect("ingest must succeed on node_a");
    }

    // -----------------------------------------------------------------------
    // Step 3: publish a Kad provider record on node_a for this CID.
    // FederatedEprStore::put would do this automatically, but we ingested
    // directly (bypassing put) to keep the EPR out of node_b's DB.
    // -----------------------------------------------------------------------
    node_a.kad_start_providing(&expected_cid).await;

    // Give Kad a moment to propagate the provider record across the in-process
    // mesh.  Same reasoning as the D.8 scenario 1 harness-limitation comment.
    tokio::time::sleep(Duration::from_millis(500)).await;

    // -----------------------------------------------------------------------
    // Step 4: cold-fetch from node_b via FederatedEprStore.
    //
    // FederatedEprStore::fetch uses blocking_send + blocking_recv internally;
    // we call it from a spawn_blocking so the tokio executor threads remain
    // free to drive the swarms.
    // -----------------------------------------------------------------------
    let pool_b = node_b.db_pool.clone();
    let swarm_tx_b = node_b.p2p_cmd_tx();
    let cid_for_fetch = expected_cid.clone();

    let fetch_result = tokio::time::timeout(
        Duration::from_secs(10),
        tokio::task::spawn_blocking(move || {
            let store = FederatedEprStore::new().with_swarm_tx(swarm_tx_b);
            let mut conn = pool_b.get().expect("node_b db connection");
            store.fetch(&mut conn, &cid_for_fetch)
        }),
    )
    .await
    .expect("cold_fetch_resolves_manifest_from_peer: 10s timeout exceeded")
    .expect("spawn_blocking join error")
    .expect("FederatedEprStore::fetch must not error");

    // -----------------------------------------------------------------------
    // Step 5: assert node_b received and cached the EPR.
    // -----------------------------------------------------------------------
    assert!(
        fetch_result.is_some(),
        "cold_fetch_resolves_manifest_from_peer: FederatedEprStore::fetch must return Some(_) \
         after resolving via Kad + request-response from node_a; got None — \
         CID: {expected_cid}"
    );

    let outcome = fetch_result.unwrap();
    assert_eq!(
        outcome.fetched.atom.cid, expected_cid,
        "cold-fetched CID must match the authored Manifest EPR CID"
    );

    // The EPR must now be cached locally in node_b's DB.
    let pool_b2 = node_b.db_pool.clone();
    let cid_for_local = expected_cid.clone();
    let local_hit = tokio::task::spawn_blocking(move || {
        let store = LocalEprStore::new();
        let mut conn = pool_b2.get().expect("node_b db connection (local check)");
        store.fetch(&mut conn, &cid_for_local)
    })
    .await
    .expect("spawn_blocking join error")
    .expect("LocalEprStore::fetch must not error");

    assert!(
        local_hit.is_some(),
        "after cold-fetch, the EPR must be cached in node_b's local store (CID: {expected_cid})"
    );
}

// ---------------------------------------------------------------------------
// Test 3: floor_cid_lookup_unconditional_at_unknown_standing
//
// Author + ingest a Manifest EPR, then verify that LocalEprStore::fetch
// (which has no standing gate) returns Some(_) — proving CID lookup is
// unconditional regardless of standing.
// ---------------------------------------------------------------------------

#[test]
fn floor_cid_lookup_unconditional_at_unknown_standing() {
    let pool = test_pool();
    let mut conn = pool.get().expect("connection");

    let epr = build_manifest_epr("imagodei", &["Agent"]);
    let expected_cid = epr.envelope.cid.to_string();

    // Ingest directly (no reach-earning gate in LocalEprStore/ingest).
    ingest(&mut conn, epr).expect("ingest must succeed");

    // CID lookup via LocalEprStore::fetch — no standing gate at this layer.
    let store = LocalEprStore::new();
    let outcome = store
        .fetch(&mut conn, &expected_cid)
        .expect("fetch must not error");

    assert!(
        outcome.is_some(),
        "CID lookup must return Some(_) at Standing::Unknown — floor protection: \
         CID-addressed fetch is unconditional"
    );

    let outcome = outcome.unwrap();
    assert_eq!(
        outcome.fetched.atom.cid, expected_cid,
        "returned CID must match ingested CID"
    );

    // Also confirm a non-existent CID returns None (not an error).
    let miss_cid = compute_cid(b"never-ingested-manifest-resolver-test").to_string();
    let miss = store
        .fetch(&mut conn, &miss_cid)
        .expect("fetch of non-existent CID must not error");
    assert!(
        miss.is_none(),
        "non-existent CID must return None, not an error"
    );
}

// ---------------------------------------------------------------------------
// Test 4: floor_protocol_load_bearing_schemaref_full_depth
//
// Direct DB seeding of a 6-node schemaRef chain, then walk at StandingScore::Floor.
//
// Floor standing clips non-Manifest kinds to depth 3.  EprKind::Manifest is
// protocol-load-bearing — walk_schemaref bypasses the standing depth limit via
// is_protocol_load_bearing_schemaref(Manifest) == true and uses usize::MAX
// instead.  A 6-deep chain must therefore succeed for Manifest kind even at
// Floor standing.
//
// For completeness, we also assert that the same chain clips at depth 3 for
// EprKind::Content (Floor standing limit = 3).
// ---------------------------------------------------------------------------

#[test]
fn floor_protocol_load_bearing_schemaref_full_depth() {
    use elohim_storage::services::schemaref_resolver::SchemaRefError;

    let pool = test_pool();
    let mut conn = pool.get().expect("connection");

    // Build a chain: a → b → c → d → e → f (terminal, no schemaRef).
    // Depth of fully-walked chain = 5 hops (6 nodes).
    let chain = [
        ("t14-a", Some("t14-b")),
        ("t14-b", Some("t14-c")),
        ("t14-c", Some("t14-d")),
        ("t14-d", Some("t14-e")),
        ("t14-e", Some("t14-f")),
        ("t14-f", None),
    ];
    for (cid, next) in chain {
        insert_manifest(&mut conn, &chain_row(cid, next)).expect("insert chain row");
    }

    let floor_standing = Standing::Computed {
        score: StandingScore::Floor,
    };

    // --- Manifest kind: full depth, bypasses floor limit ---
    let walk = walk_schemaref(&mut conn, "t14-a", EprKind::Manifest, floor_standing)
        .expect("walk_schemaref must succeed for Manifest kind at Floor standing");

    assert_eq!(
        walk.manifest_cids.len(),
        6,
        "Manifest kind at Floor standing must walk all 6 nodes (protocol-load-bearing floor)"
    );
    assert_eq!(
        walk.manifest_cids,
        vec![
            "t14-a".to_string(),
            "t14-b".to_string(),
            "t14-c".to_string(),
            "t14-d".to_string(),
            "t14-e".to_string(),
            "t14-f".to_string(),
        ],
        "manifest_cids must list all chain nodes in order"
    );
    assert_eq!(walk.depth, 5, "depth must be 5 (5 hops across 6 nodes)");

    // --- Content kind: clips at depth 3 (Floor limit) ---
    // Content requires Knowledge + Value + Governance coupling but walk_schemaref
    // reads from the manifests table only (not the epr_atoms table), so the
    // chain rows we inserted are sufficient.
    let clip_result = walk_schemaref(&mut conn, "t14-a", EprKind::Content, floor_standing);
    match clip_result {
        Err(SchemaRefError::DepthExceeded { limit, .. }) => {
            assert_eq!(
                limit, 3,
                "Floor standing must clip Content kind walk at depth limit 3"
            );
        }
        other => panic!(
            "expected SchemaRefError::DepthExceeded for Content kind at Floor standing, got: {other:?}"
        ),
    }
}
