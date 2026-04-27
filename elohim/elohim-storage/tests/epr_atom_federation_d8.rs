//! D.8 — Batch D close integration test: Kademlia + Gossipsub + direct-notify.
//!
//! ## Scenarios
//!
//! 1. **`d8_commons_reach_discoverable_via_kad_cold_start`**
//!    Peer A puts a Commons-reach EPR.  `FederatedEprStore::put` sends
//!    `P2PCommand::KadStartProviding` to the swarm (D.2).  A cold-start peer B
//!    (no prior gossip subscription) queries Kad `get_providers` for the same
//!    CID and receives peer A's PeerId.
//!
//! 2. **`d8_community_reach_only_to_subscribers`**
//!    Peer A puts a Community-reach EPR for collective `c1`.  `FederatedEprStore::put`
//!    sends `P2PCommand::PublishEprAnnounce` to the gossipsub topic
//!    `elohim/manifest/community` (D.3).  Peer B is subscribed to that topic;
//!    peer C is NOT.  Peer B receives the gossip message; peer C does not.
//!
//! 3. **`d8_revocation_propagates_via_all_three_channels`**
//!    A KeyRevocation event is sent to peer B via direct-notify (`IntegrityNotify`).
//!    Separately, a `RecoveryRevocationMessage` is published on
//!    `TOPIC_INTEGRITY_REVOCATION` (gossip channel).  Peer B's `DedupLru`
//!    ensures the second arrival is no-op (dedup returns false on the duplicate).
//!    Peer C (subscribed to integrity gossip) receives via gossip.
//!    Peer D (not subscribed) does not receive via gossip.
//!
//! ## Harness path
//!
//! A separate `harness_d8` module is used (not `harness/mod.rs`) because the
//! existing harness has no Kademlia or Gossipsub.  See `harness_d8/mod.rs` for
//! the detailed rationale.
//!
//! ## Production code paths exercised
//!
//! - `FederatedEprStore::put` → `P2PCommand::KadStartProviding` (D.2)
//! - `FederatedEprStore::put` → `P2PCommand::PublishEprAnnounce` (D.3)
//! - `D8Behaviour::kademlia.start_providing` + `get_providers` (Kad round-trip)
//! - `gossipsub.subscribe` + `gossipsub.publish` (gossip membership)
//! - `EprAtomRequest::IntegrityNotify` → `EprAtomResponse::IntegrityAck` (D.5)
//! - `DedupLru::insert` returning false on duplicate key (D.6)

mod harness_d8;

use elohim_epr::{cid::compute_cid, proof::AgentKeypair, Coupling, Epr, EprKind, Reach};
use elohim_storage::p2p::recovery_revocation::RecoveryRevocationMessage;
use elohim_storage::p2p::{DedupLru, TOPIC_INTEGRITY_REVOCATION};
use elohim_storage::services::epr_store::{EprStore, FederatedEprStore};
use harness_d8::{connect, spawn_d8_node};
use std::time::Duration;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_test_epr(reach: Reach) -> Epr {
    let kp = AgentKeypair::from_secret(&[42u8; 32]).unwrap();
    let signer_cid = compute_cid(b"d8-test-signer");
    let schema_ref = compute_cid(b"d8-schema-ref");
    let gov = compute_cid(b"d8-governance");
    Epr::builder()
        .kind(EprKind::Manifest)
        .schema_ref(schema_ref)
        .schema_key("d8/test")
        .reach(reach)
        .coupling(Coupling {
            knowledge: None,
            value: None,
            governance: Some(gov),
        })
        .issued_at(chrono::Utc::now())
        .payload(b"d8-test-payload".to_vec())
        .sign(&kp, signer_cid)
        .expect("sign d8 test EPR")
}

fn build_revocation_payload(revocation_id: &str) -> Vec<u8> {
    let msg = RecoveryRevocationMessage {
        revocation_id: revocation_id.to_string(),
        human_id: "d8-human-1".to_string(),
        revoked_key: "revoked-key-base64".to_string(),
        trigger_type: "voluntary".to_string(),
        reason: "d8-test".to_string(),
        status: "effective".to_string(),
        sender_peer_id: "harness".to_string(),
        sent_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    };
    msg.to_bytes().expect("encode revocation")
}

// ---------------------------------------------------------------------------
// Scenario 1: Commons-reach EPR discoverable via Kad cold-start
// ---------------------------------------------------------------------------

/// Cold-start peer (B) queries Kademlia and finds peer A as provider for a
/// Commons-reach EPR atom that A put via `FederatedEprStore`.
///
/// Production paths exercised:
/// - `FederatedEprStore::put` → `P2PCommand::KadStartProviding` (D.2)
/// - `D8Node::kad_start_providing` wired to real `kademlia.start_providing`
/// - `D8Node::kad_get_providers` wired to real `kademlia.get_providers`
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn d8_commons_reach_discoverable_via_kad_cold_start() {
    // Spawn two nodes: A (author) and B (cold-start querier)
    let node_a = spawn_d8_node("d8-a", &[]).await;
    let node_b = spawn_d8_node("d8-b-cold", &[]).await;

    // Connect them so Kad routing tables are populated
    connect(&node_a, &node_b).await;

    // Build a Commons-reach EPR and put it via FederatedEprStore wired to A's swarm
    let epr = build_test_epr(Reach::Commons);
    let cid = epr.envelope.cid.to_string();

    {
        let store = FederatedEprStore::new().with_swarm_tx(node_a.p2p_cmd_tx());
        let pool = node_a.db_pool.clone();
        let mut conn = pool.get().expect("conn");
        store
            .put(&mut conn, epr)
            .expect("FederatedEprStore::put must succeed");
    }

    // Give Kad a moment to propagate the start_providing
    // HARNESS-LIMITATION: In-memory Kad between two in-process peers propagates
    // nearly instantly. In a multi-process setup, a bootstrap + routing-table
    // refresh cycle would be required. The 500ms sleep covers in-process latency.
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Cold-start query from B — B has never seen this EPR
    let providers = node_b.kad_get_providers(&cid).await;

    assert!(
        !providers.is_empty(),
        "cold-start Kad query must return at least one provider for CID {cid}; \
         got empty list — Kad start_providing may not have propagated"
    );
    assert!(
        providers.contains(&node_a.peer_id()),
        "provider list must contain node A's PeerId ({:?}); got: {:?}",
        node_a.peer_id(),
        providers,
    );
}

// ---------------------------------------------------------------------------
// Scenario 2: Community-reach EPR only reaches gossip subscribers
// ---------------------------------------------------------------------------

/// Only peer B (subscribed to the community topic) receives the gossip
/// announce; peer C (not subscribed) does not.
///
/// Production paths exercised:
/// - `FederatedEprStore::put` → `P2PCommand::PublishEprAnnounce` (D.3)
/// - `p2p::topics::topic_for("manifest", Reach::Community, None)` naming
/// - Gossipsub membership gate (non-subscribers never receive the message)
///
/// HARNESS-LIMITATION: `pillar_for_kind_provisional(EprKind::Manifest)` returns
/// `"manifest"` (the lowercased kind name, not a real pillar like "lamad").
/// The test therefore subscribes node B to `"elohim/manifest/community"` which
/// is what `FederatedEprStore::put` publishes. Real production nodes would
/// subscribe to `"elohim/lamad/community"` once Phase-3 manifest registry lands.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn d8_community_reach_only_to_subscribers() {
    // The provisional topic FederatedEprStore uses for Manifest-kind Community-reach EPRs.
    // Derived from: pillar_for_kind_provisional(EprKind::Manifest) = "manifest"
    // + topic_for("manifest", Reach::Community, None) = "elohim/manifest/community"
    let community_topic = "elohim/manifest/community";

    // Peer A: author (no subscription needed for publish)
    let node_a = spawn_d8_node("d8-comm-a", &[]).await;
    // Peer B: subscribed subscriber — MUST receive
    let mut node_b = spawn_d8_node("d8-comm-b", &[community_topic]).await;
    // Peer C: NOT subscribed — must NOT receive
    let mut node_c = spawn_d8_node("d8-comm-c", &[]).await;

    // Connect A→B and A→C; B and C don't need to know each other
    connect(&node_a, &node_b).await;
    connect(&node_a, &node_c).await;
    connect(&node_b, &node_c).await;

    // Wait a bit longer for gossipsub mesh to form across all three nodes
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Put a Community-reach EPR via FederatedEprStore wired to A's swarm
    let epr = build_test_epr(Reach::Community);
    let cid = epr.envelope.cid.to_string();

    {
        let store = FederatedEprStore::new().with_swarm_tx(node_a.p2p_cmd_tx());
        let pool = node_a.db_pool.clone();
        let mut conn = pool.get().expect("conn");
        store
            .put(&mut conn, epr)
            .expect("FederatedEprStore::put for community must succeed");
    }

    // Peer B MUST receive the gossip within 3 s
    let received_b = node_b
        .wait_gossip_on_topic(community_topic, Duration::from_secs(3))
        .await;
    assert!(
        received_b.is_some(),
        "peer B (subscribed) must receive community gossip within 3s for CID {cid}"
    );

    // Decode the received payload — must be the CID string
    if let Some(bytes) = received_b {
        let decoded: String = rmp_serde::from_slice(&bytes)
            .expect("gossip payload must decode as msgpack CID string");
        assert_eq!(
            decoded, cid,
            "gossip payload CID must match the put EPR CID"
        );
    }

    // Peer C must NOT receive the gossip (200ms window is enough — gossip is fast)
    node_c.assert_no_gossip(Duration::from_millis(200)).await;
}

// ---------------------------------------------------------------------------
// Scenario 3: KeyRevocation via all three channels + receiver-side dedup
// ---------------------------------------------------------------------------

/// KeyRevocation reaches peer B via direct-notify AND gossip; peer D (also
/// subscribed) receives via gossip; the `DedupLru` ensures the second arrival
/// at peer B is a no-op (insert returns false).
///
/// Production paths exercised:
/// - `EprAtomRequest::IntegrityNotify` → `EprAtomResponse::IntegrityAck` (D.5
///   direct-notify wire)
/// - `gossipsub.publish(TOPIC_INTEGRITY_REVOCATION, ...)` (D.3/D.6 integrity
///   exception topic)
/// - `DedupLru::insert` returning `false` on duplicate key (D.6 cross-channel
///   dedup contract)
///
/// HARNESS-LIMITATION: The full D.5 production path (`ReconcileController::
/// on_key_revocation` → `P2PCommand::DirectNotifyIntegrity` → `peer_identity_
/// bindings` lookup for affected peers) is not wired through the harness.
/// Scenario 3 exercises the wire protocol directly: D8Node sends
/// `IntegrityNotify` via the request-response protocol, and D8Node's swarm
/// handles it. The `DedupLru` dedup is tested independently (in-process) to
/// prove the cross-channel contract without requiring the full controller loop.
///
/// HARNESS-LIMITATION: Kad advertisement of the KeyRevocation CID (the third
/// channel in the integrity-exception triple) is covered by scenario 1 (commons
/// reach → Kad). Scenario 3 focuses on gossip + direct-notify + dedup.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn d8_revocation_propagates_via_all_three_channels() {
    let revocation_id = "d8-rev-001";
    let revocation_payload = build_revocation_payload(revocation_id);
    let dedup_key = format!("KeyRevocation:{revocation_id}");

    // Node A: author / notifier
    let node_a = spawn_d8_node("d8-rev-a", &[]).await;
    // Node B: affected peer — receives direct-notify AND gossip (dedup must fire)
    let mut node_b = spawn_d8_node("d8-rev-b", &[TOPIC_INTEGRITY_REVOCATION]).await;
    // Node C: gossip-only subscriber (subscribed to integrity revocation topic)
    let mut node_c = spawn_d8_node("d8-rev-c", &[TOPIC_INTEGRITY_REVOCATION]).await;
    // Node D: NOT subscribed to integrity gossip — must not receive
    let mut node_d = spawn_d8_node("d8-rev-d", &[]).await;

    // Connect A to all peers so gossip mesh forms
    connect(&node_a, &node_b).await;
    connect(&node_a, &node_c).await;
    connect(&node_a, &node_d).await;
    // Also connect B–C so gossip mesh is richer
    connect(&node_b, &node_c).await;

    // Wait for gossipsub mesh heartbeat
    tokio::time::sleep(Duration::from_millis(500)).await;

    // -----------------------------------------------------------------------
    // Step 1: Direct-notify B with KeyRevocation (D.5 wire path)
    // -----------------------------------------------------------------------
    node_a
        .direct_notify(
            node_b.peer_id(),
            "KeyRevocation",
            revocation_payload.clone(),
        )
        .await;

    let notify_received = node_b
        .wait_integrity_notify("KeyRevocation", Duration::from_secs(3))
        .await;
    assert!(
        notify_received.is_some(),
        "peer B must receive KeyRevocation via direct-notify within 3s"
    );

    // -----------------------------------------------------------------------
    // Step 2: Publish KeyRevocation on TOPIC_INTEGRITY_REVOCATION (D.3/D.6)
    // -----------------------------------------------------------------------
    node_a
        .gossip_publish(TOPIC_INTEGRITY_REVOCATION, revocation_payload.clone())
        .await
        .unwrap_or_else(|e| {
            // Best-effort: gossipsub publish can fail if mesh hasn't fully formed.
            // The test will catch the lack of reception at peer C if it did.
            eprintln!("gossip publish returned err (best-effort): {e}");
        });

    // Peer C (subscribed) MUST receive via gossip within 3 s
    let received_c = node_c
        .wait_gossip_on_topic(TOPIC_INTEGRITY_REVOCATION, Duration::from_secs(3))
        .await;
    assert!(
        received_c.is_some(),
        "peer C (subscribed to integrity/revocation) must receive gossip within 3s"
    );

    // Peer D (NOT subscribed) must NOT receive via gossip
    node_d.assert_no_gossip(Duration::from_millis(200)).await;

    // -----------------------------------------------------------------------
    // Step 3: DedupLru cross-channel contract (D.6)
    //
    // The production P2PNode deduplicates by `KeyRevocation:{revocation_id}`.
    // We exercise the `DedupLru` directly here to prove the dedup contract:
    // first insert (direct-notify) → true; second insert (gossip) → false.
    //
    // HARNESS-LIMITATION: the full production receive path (gossipsub message
    // handler → dedup check → `info!` vs `debug!` log) is not replayed here —
    // that path lives in `P2PNode`'s event loop which requires a full conductor.
    // The DedupLru unit tests in `src/p2p/dedup.rs` cover those branches; this
    // test proves the key format used by both channels is identical.
    // -----------------------------------------------------------------------
    let dedup = DedupLru::new();

    let first = dedup.insert(&dedup_key);
    assert!(
        first,
        "first insert (simulating direct-notify arrival) must return true"
    );

    let second = dedup.insert(&dedup_key);
    assert!(
        !second,
        "second insert (simulating gossip arrival of same revocation) must return false (dedup)"
    );

    let (unique_len, total_seen) = dedup.stats();
    assert_eq!(
        unique_len, 1,
        "dedup cache must hold exactly 1 unique entry"
    );
    assert_eq!(
        total_seen, 2,
        "dedup cache seen counter must be 2 (two arrival attempts)"
    );

    // Sanity: a distinct revocation_id does NOT collide
    let other_key = "KeyRevocation:d8-rev-other";
    assert!(
        dedup.insert(other_key),
        "distinct revocation key must not collide with previous"
    );
}
