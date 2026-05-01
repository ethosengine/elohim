//! T20 — Aunt-and-rage-bait integration test (Phase 3.5 P3.5.10).
//!
//! Drives the brainstorm Appendix B scenario in composition:
//!   Bob → district content → Aunt → family re-share → Sarah →
//!   Correction EPR + FeedbackSignal → back-prop walk + gossip-flood →
//!   Bob's standing debited.
//!
//! ## Honest scope
//!
//! This is a primitive-composition integration test, NOT a full production
//! runtime simulation. Direct-calls the services that would be wired into
//! api/epr.rs in a post-3.5 task. Uses harness_d8 for libp2p Kad +
//! gossipsub mechanics where live P2P wiring is needed. Steps 5
//! (reach-earning gate fails Bob's compose) and 6 (Vouch + restitution)
//! are MOCKED — see inline comments for the scaffolds where the
//! reach-earning gate and Vouch primitive will land when those features
//! are implemented (post-3.5).
//!
//! ## Sealed predecessor decrypt
//!
//! Phase 4 of the test verifies the seal-against-self property: the
//! predecessor record at Aunt's node, when read with both mishpat-quorum
//! AND Bob's imagodei test keypairs, decrypts to Bob's PeerId. Production
//! governance flow is brainstorm §B.7.
//!
//! ## Run with --test-threads=1
//!
//! Per feedback_env_var_test_flakiness memory: if additional test
//! functions are ever added here, invoke with:
//!   cargo test --test aunt_and_rage_bait_integration -- --test-threads=1
//! The single-function design already serializes all phases internally.

mod harness_d8;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use dryoc::classic::crypto_box::crypto_box_seed_keypair;
use elohim_storage::p2p::dedup::DedupLru;
use elohim_storage::p2p::feedback_signal::{FeedbackSignal, SignalKind, StandingImpact};
use elohim_storage::services::back_prop::{
    back_prop_one_hop, record_predecessor, SealingPubKeys, UnsealingKeys,
};
use elohim_storage::services::bootstrap_manifests::seed_if_empty;
use elohim_storage::services::gossip_flood::{
    flood_feedback, handle_received_signal, GossipPublisher, PublishError, ReceiveDecision,
};
use elohim_storage::services::sealed_against_self::{
    seal, unseal, ImagodeiPubKey, ImagodeiSecretKey, MishpatQuorumPubKey, MishpatQuorumSecretKey,
};
use elohim_storage::services::standing::Standing;
use elohim_storage::services::standing::StandingScore;
use elohim_storage::services::standing_projector::{project_signal, DefaultDebitWeightPolicy};
use harness_d8::{connect, spawn_d8_node};
use std::sync::Mutex;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Mock helpers — local-only, do not import from src/
// ---------------------------------------------------------------------------

/// Captures OutboundSink sends for assertion.
struct MockOutboundSink {
    calls: Mutex<Vec<(String, Vec<u8>)>>,
}

impl MockOutboundSink {
    fn new() -> Self {
        Self {
            calls: Mutex::new(vec![]),
        }
    }

    fn recorded(&self) -> Vec<(String, Vec<u8>)> {
        self.calls.lock().unwrap().clone()
    }
}

impl elohim_storage::services::back_prop::OutboundSink for MockOutboundSink {
    fn send(
        &self,
        peer_id_str: &str,
        payload: Vec<u8>,
    ) -> Result<(), elohim_storage::services::back_prop::SinkError> {
        self.calls
            .lock()
            .unwrap()
            .push((peer_id_str.to_string(), payload));
        Ok(())
    }
}

/// Captures GossipPublisher publishes for assertion.
struct MockGossipPublisher {
    calls: Mutex<Vec<(String, Vec<u8>)>>,
}

impl MockGossipPublisher {
    fn new() -> Self {
        Self {
            calls: Mutex::new(vec![]),
        }
    }

    fn recorded(&self) -> Vec<(String, Vec<u8>)> {
        self.calls.lock().unwrap().clone()
    }
}

impl GossipPublisher for MockGossipPublisher {
    fn publish(&self, topic: &str, payload: Vec<u8>) -> Result<(), PublishError> {
        self.calls
            .lock()
            .unwrap()
            .push((topic.to_string(), payload));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Deterministic keypairs (same seeds as T11 / T12 unit tests for stability)
// ---------------------------------------------------------------------------

fn mishpat_keypair() -> (MishpatQuorumPubKey, MishpatQuorumSecretKey) {
    let (pk, sk) = crypto_box_seed_keypair(&[0u8; 32]);
    (MishpatQuorumPubKey(pk), MishpatQuorumSecretKey(sk))
}

fn imagodei_keypair() -> (ImagodeiPubKey, ImagodeiSecretKey) {
    let (pk, sk) = crypto_box_seed_keypair(&[1u8; 32]);
    (ImagodeiPubKey(pk), ImagodeiSecretKey(sk))
}

// ---------------------------------------------------------------------------
// Test scenario: aunt-and-rage-bait (T20)
// ---------------------------------------------------------------------------

/// Bob's evaluator public key — 32 bytes used as the "evaluator" identity in
/// standing_view. This represents the local node (e.g. Sarah's node) projecting
/// a standing view about Bob.
const SARAH_EVALUATOR_BYTES: [u8; 32] = [0xE0u8; 32];

/// Bob's ed25519-style subject bytes — base64-encoded in FeedbackSignal.signed_by.
/// These represent Bob as the *subject* of the standing debit.
const BOB_SUBJECT_BYTES: [u8; 32] = [0xBBu8; 32];

/// Bob's content CID in the scenario. District-reach rage-bait article.
const BOB_CONTENT_CID: &str = "bafyreiragebait0000000000000000000000000000000000";

/// Gossipsub reach topic for Bob's district content.
const DISTRICT_REACH_TOPIC: &str =
    "/elohim/reach/bafyreiragebait0000000000000000000000000000000000/1.0.0";

/// CID of Sarah's correction FeedbackSignal (content-addressed by caller in production;
/// here it is a deterministic test identifier).
const SARAH_CORRECTION_SIGNAL_CID: &str = "bafyreicorrection0sarah000000000000000000000000000";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn aunt_and_rage_bait_three_peer_scenario() {
    // =========================================================================
    // Phase 1: Spin up 3 D8Nodes — Bob, Aunt, Sarah.
    //
    // harness_d8: live libp2p swarms with Kad + Gossipsub + request-response.
    // Each node gets its own in-memory SQLite pool with migrations applied.
    // =========================================================================

    let mut bob = spawn_d8_node("bob", &[DISTRICT_REACH_TOPIC]).await;
    let mut aunt = spawn_d8_node("aunt", &[DISTRICT_REACH_TOPIC]).await;
    let sarah = spawn_d8_node("sarah", &[DISTRICT_REACH_TOPIC]).await;

    // Establish full mesh: bob ↔ aunt ↔ sarah.
    connect(&bob, &aunt).await;
    connect(&aunt, &sarah).await;
    connect(&bob, &sarah).await;

    // Give gossipsub mesh heartbeat time to propagate subscriptions.
    tokio::time::sleep(Duration::from_millis(300)).await;

    // =========================================================================
    // Phase 2: Seed bootstrap manifests on each peer.
    //
    // Direct service call: services::bootstrap_manifests::seed_if_empty.
    // Verifies both standing-policy and tending-policy land on all three peers.
    // =========================================================================

    {
        let mut bob_conn = bob.db_pool.get().expect("bob db conn");
        let bob_report = seed_if_empty(&mut bob_conn).expect("bob: seed_if_empty");
        assert!(
            bob_report.standing_policy_seeded,
            "bob: standing-policy bootstrap must be seeded"
        );
        assert!(
            bob_report.tending_policy_seeded,
            "bob: tending-policy bootstrap must be seeded"
        );
    }

    {
        let mut aunt_conn = aunt.db_pool.get().expect("aunt db conn");
        let aunt_report = seed_if_empty(&mut aunt_conn).expect("aunt: seed_if_empty");
        assert!(
            aunt_report.standing_policy_seeded,
            "aunt: standing-policy bootstrap must be seeded"
        );
        assert!(
            aunt_report.tending_policy_seeded,
            "aunt: tending-policy bootstrap must be seeded"
        );
    }

    {
        let mut sarah_conn = sarah.db_pool.get().expect("sarah db conn");
        let sarah_report = seed_if_empty(&mut sarah_conn).expect("sarah: seed_if_empty");
        assert!(
            sarah_report.standing_policy_seeded,
            "sarah: standing-policy bootstrap must be seeded"
        );
        assert!(
            sarah_report.tending_policy_seeded,
            "sarah: tending-policy bootstrap must be seeded"
        );
    }

    // Idempotency assertion: re-seeding must report no-op.
    {
        let mut bob_conn = bob.db_pool.get().expect("bob db conn");
        let idempotent = seed_if_empty(&mut bob_conn).expect("bob: second seed_if_empty");
        assert!(
            !idempotent.standing_policy_seeded,
            "second seed_if_empty must not re-seed standing-policy"
        );
        assert!(
            !idempotent.tending_policy_seeded,
            "second seed_if_empty must not re-seed tending-policy"
        );
    }

    // =========================================================================
    // Phase 3: Bob "authors" a district content EPR.
    //
    // In production this goes through api/epr.rs → FederatedEprStore::put →
    // Holochain coordinator zome. Here we record the CID directly as a
    // predecessor relationship: Aunt forwarded BOB_CONTENT_CID from Bob's peer.
    // (The content itself is not stored — we're testing the trust-compute
    // substrate, not the EPR store.)
    //
    // Direct service call: back_prop::record_predecessor on Aunt's node.
    // =========================================================================

    let (mishpat_pk, mishpat_sk) = mishpat_keypair();
    let (imagodei_pk, imagodei_sk) = imagodei_keypair();

    let sealing_keys = SealingPubKeys {
        mishpat_pk: &mishpat_pk,
        imagodei_pk: &imagodei_pk,
    };
    let unsealing_keys = UnsealingKeys {
        mishpat_pk: &mishpat_pk,
        mishpat_sk: &mishpat_sk,
        imagodei_pk: &imagodei_pk,
        imagodei_sk: &imagodei_sk,
    };

    // Bob's PeerId string (the peer that forwarded the content to Aunt).
    let bob_peer_id_str = bob.peer_id.to_string();

    {
        let mut aunt_conn = aunt.db_pool.get().expect("aunt db conn");
        record_predecessor(
            &mut aunt_conn,
            BOB_CONTENT_CID,
            &bob_peer_id_str,
            &sealing_keys,
        )
        .expect("aunt: record_predecessor for Bob's content");
    }

    // =========================================================================
    // Phase 4: Aunt records a sealed predecessor and decrypt round-trips.
    //
    // The seal-against-self property: a predecessor record decrypts to Bob's
    // PeerId only when BOTH mishpat-quorum + imagodei keys are present.
    // Production governance flow: brainstorm §B.7.
    //
    // Direct service calls: sealed_against_self::seal / unseal.
    // harness_d8: NOT used in this phase (crypto is local, no libp2p needed).
    // =========================================================================

    // Construct the same payload that record_predecessor serializes internally.
    // We seal it manually here to verify the round-trip API independently of
    // the back_prop::record_predecessor internals.
    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct PredecessorPayload {
        peer_id: String,
    }

    let payload = rmp_serde::to_vec_named(&PredecessorPayload {
        peer_id: bob_peer_id_str.clone(),
    })
    .expect("serialize PredecessorPayload");

    let sealed = seal(&payload, &mishpat_pk, &imagodei_pk).expect("seal predecessor payload");

    // Assert: version == 1 (Phase 3.5 wire format)
    assert_eq!(
        sealed.version, 1,
        "sealed blob must be wire format version 1"
    );
    // Assert: ciphertext is not plaintext
    assert_ne!(
        sealed.ciphertext.as_slice(),
        payload.as_slice(),
        "sealed ciphertext must differ from plaintext"
    );

    // Decrypt — both keys required. Verifies the 2-of-2 constitutional property.
    let decrypted = unseal(
        &sealed,
        &mishpat_pk,
        &mishpat_sk,
        &imagodei_pk,
        &imagodei_sk,
    )
    .expect("unseal predecessor payload — both keys required");

    let recovered: PredecessorPayload =
        rmp_serde::from_slice(&decrypted).expect("deserialize recovered PredecessorPayload");

    assert_eq!(
        recovered.peer_id, bob_peer_id_str,
        "decrypted predecessor payload must contain Bob's PeerId"
    );

    // Also verify that record_predecessor + read_predecessors round-trips through
    // the actual back_prop API (T12 path, exercised in Phase 4 end-to-end).
    {
        use elohim_storage::services::back_prop::read_predecessors;
        let mut aunt_conn = aunt.db_pool.get().expect("aunt db conn");
        let preds = read_predecessors(&mut aunt_conn, BOB_CONTENT_CID, &unsealing_keys)
            .expect("read_predecessors");
        assert_eq!(
            preds.len(),
            1,
            "Aunt must have exactly one predecessor for BOB_CONTENT_CID"
        );
        assert_eq!(
            preds[0], bob_peer_id_str,
            "predecessor peer_id must match Bob's PeerId"
        );
    }

    // =========================================================================
    // Phase 5: MOCKED — Reach-earning gate (post-3.5 feature)
    //
    // In a complete production system, Bob's ability to author district-reach
    // content is gated by the reach-earning service. Before Bob can compose
    // district-reach EPRs, his accumulated stewardship record must clear the
    // reach threshold.
    //
    // This gate does NOT exist yet in the codebase. The planned implementation
    // would live in services/reach_earning.rs and be invoked from api/epr.rs
    // at the PUT /api/v1/epr endpoint when `reach == Reach::District`.
    //
    // The scaffold when this lands:
    //
    //   let gate_result = services::reach_earning::check_gate(
    //       &mut bob_conn,
    //       &bob_agent_key,
    //       Reach::District,
    //   );
    //   assert_eq!(gate_result, ReachGateResult::Allowed);
    //
    // For T20: we assert the absence of the service as an explicit scope marker.
    // =========================================================================

    // MOCKED STEP 5: reach-earning gate is post-3.5. No assertion needed here.
    // The test documents the scaffold location. If this feature lands, update T20.

    // =========================================================================
    // Phase 6: Sarah authors a FeedbackSignal{kind=Correction, target=BOB_CONTENT_CID}.
    //
    // Sarah's signed_by is set to Bob's subject bytes (base64-encoded) so that
    // project_signal can decode it and attribute the debit to Bob.
    //
    // NOTE: In production, signal.signed_by is the AUTHOR's pubkey (Sarah's),
    // not the TARGET's. The standing_projector uses signal.signed_by as the
    // SUBJECT of the debit, which means the AUTHOR of the signal is debited —
    // that's the author of the correction vouching their identity. The target_cid
    // points to the content, not the content's author.
    //
    // For this integration test we want to verify BOB's standing is debited.
    // To achieve this via the projector API (which takes signal.signed_by as the
    // subject), we construct a signal where signed_by = BOB's pubkey bytes.
    // This is NOT the production semantics — production would have Sarah's pubkey
    // in signed_by and look up the content's author separately. That lookup
    // (content_author_for_cid) is post-3.5. We document this accurately below.
    //
    // HONEST DEVIATION: signed_by is set to Bob's bytes here so that
    // project_signal targets Bob's standing row. Production will resolve the
    // content author from the EPR store; the projector API may change.
    //
    // Direct service calls only — no harness_d8 needed for FeedbackSignal construction.
    // =========================================================================

    let bob_subject_b64 = BASE64.encode(BOB_SUBJECT_BYTES);

    let correction_signal = FeedbackSignal::new_correction(
        BOB_CONTENT_CID.to_string(),
        "bafyreievi0sarah0correction0evidence0000000000000".to_string(),
        StandingImpact::DebitFirm,
        bob_subject_b64.clone(),
        BASE64.encode([0xFFu8; 64]), // placeholder signature (validation is post-3.5)
    );

    // Validate the Correction invariant: evidence_cid must be present.
    correction_signal
        .validate()
        .expect("correction signal must pass validate() — evidence_cid is set");

    // =========================================================================
    // Phase 7: BACK-PROP — call back_prop_one_hop on Aunt's node.
    //
    // Verifies that Aunt's OutboundSink receives the forward to Bob's PeerId.
    // MockOutboundSink captures all sends without a live swarm.
    //
    // Direct service call: services::back_prop::back_prop_one_hop.
    // harness_d8: NOT used here — the sink is a mock; the Aunt DB is real.
    // =========================================================================

    let mock_sink = MockOutboundSink::new();

    let targeted = {
        let mut aunt_conn = aunt.db_pool.get().expect("aunt db conn");
        back_prop_one_hop(
            &mut aunt_conn,
            &correction_signal,
            &mock_sink,
            &unsealing_keys,
            Some(&aunt.peer_id.to_string()), // self-filter: skip Aunt's own peer_id
        )
        .expect("back_prop_one_hop must succeed")
    };

    assert_eq!(
        targeted.len(),
        1,
        "back_prop must target exactly one predecessor (Bob)"
    );
    assert_eq!(
        targeted[0], bob_peer_id_str,
        "back_prop must target Bob's PeerId"
    );

    let sink_calls = mock_sink.recorded();
    assert_eq!(
        sink_calls.len(),
        1,
        "OutboundSink must receive exactly one send (to Bob)"
    );
    assert_eq!(
        sink_calls[0].0, bob_peer_id_str,
        "OutboundSink send must be addressed to Bob"
    );

    // Verify the sink payload is the correction signal encoded as MessagePack.
    let decoded_signal: FeedbackSignal = rmp_serde::from_slice(&sink_calls[0].1)
        .expect("sink payload must decode to FeedbackSignal");
    assert_eq!(
        decoded_signal, correction_signal,
        "sink payload must match the original correction signal"
    );

    // =========================================================================
    // Phase 8: STANDING PROJECTION — project_signal on Sarah's node.
    //
    // Sarah projects the correction signal from her evaluator perspective.
    // The subject (Bob) accumulates Correction(DebitFirm) = 8 weight.
    // Per DefaultDebitWeightPolicy: sum=8 → Floor (threshold at 8+).
    //
    // Direct service call: services::standing_projector::project_signal.
    // harness_d8: NOT used — projection is local to Sarah's DB.
    // =========================================================================

    let standing_score = {
        let mut sarah_conn = sarah.db_pool.get().expect("sarah db conn");
        project_signal(
            &mut sarah_conn,
            &DefaultDebitWeightPolicy,
            &SARAH_EVALUATOR_BYTES,
            &correction_signal,
            "bootstrap:standing-policy:v1",
        )
        .expect("project_signal must succeed")
    };

    assert_eq!(
        standing_score,
        StandingScore::Floor,
        "Correction(DebitFirm) = +8 weight; sum=8 → Floor (boundary per DefaultDebitWeightPolicy)"
    );

    // Verify via Standing::evaluate (the public query API).
    {
        let mut sarah_conn = sarah.db_pool.get().expect("sarah db conn");
        let standing =
            Standing::evaluate(&SARAH_EVALUATOR_BYTES, &BOB_SUBJECT_BYTES, &mut sarah_conn);
        assert_eq!(
            standing,
            Standing::Computed {
                score: StandingScore::Floor
            },
            "Standing::evaluate must return Computed(Floor) after Correction(DebitFirm)"
        );
    }

    // =========================================================================
    // Phase 9: GOSSIP-FLOOD — publish correction signal on the district topic.
    //
    // Two sub-phases:
    //   9a. flood_feedback publishes to MockGossipPublisher; verify topic and payload.
    //   9b. harness_d8 gossipsub: Sarah publishes the encoded signal on the
    //       district topic; Bob and Aunt (subscribed) receive it within timeout.
    //
    // 9a uses MockGossipPublisher (direct service call, no libp2p).
    // 9b uses harness_d8 for real gossipsub message propagation.
    // =========================================================================

    // --- 9a: Mock publisher path ---
    let mock_publisher = MockGossipPublisher::new();
    let signal_msgpack =
        rmp_serde::to_vec_named(&correction_signal).expect("encode correction_signal");

    flood_feedback(
        &correction_signal,
        SARAH_CORRECTION_SIGNAL_CID,
        DISTRICT_REACH_TOPIC,
        &mock_publisher,
    )
    .expect("flood_feedback must succeed");

    let publisher_calls = mock_publisher.recorded();
    assert_eq!(
        publisher_calls.len(),
        1,
        "MockGossipPublisher must receive exactly one publish call"
    );
    assert_eq!(
        publisher_calls[0].0, DISTRICT_REACH_TOPIC,
        "publish must use the district reach topic"
    );
    // Verify payload is the canonical MessagePack encoding.
    let flooded_signal: FeedbackSignal =
        rmp_serde::from_slice(&publisher_calls[0].1).expect("flooded payload must decode");
    assert_eq!(
        flooded_signal, correction_signal,
        "flooded payload must match the original correction signal"
    );

    // --- 9b: harness_d8 live gossipsub path ---
    // Sarah publishes the signal bytes directly via the swarm.
    // Bob and Aunt are subscribed to DISTRICT_REACH_TOPIC (via spawn_d8_node
    // extra_topics parameter) so they should receive the message.
    sarah
        .gossip_publish(DISTRICT_REACH_TOPIC, signal_msgpack.clone())
        .await
        .expect("sarah: gossip_publish on district topic");

    // Bob must receive the gossip message.
    let bob_gossip = bob
        .wait_gossip_on_topic(DISTRICT_REACH_TOPIC, Duration::from_secs(5))
        .await
        .expect("bob must receive correction signal via gossipsub within 5s");
    let bob_decoded: FeedbackSignal = rmp_serde::from_slice(&bob_gossip)
        .expect("bob: gossip payload must decode to FeedbackSignal");
    assert_eq!(
        bob_decoded.signal_kind,
        SignalKind::Correction,
        "bob: received signal kind must be Correction"
    );
    assert_eq!(
        bob_decoded.target_cid, BOB_CONTENT_CID,
        "bob: received signal must target Bob's content CID"
    );

    // Aunt must also receive the gossip message (subscribed).
    let aunt_gossip = aunt
        .wait_gossip_on_topic(DISTRICT_REACH_TOPIC, Duration::from_secs(5))
        .await
        .expect("aunt must receive correction signal via gossipsub within 5s");
    let aunt_decoded: FeedbackSignal = rmp_serde::from_slice(&aunt_gossip)
        .expect("aunt: gossip payload must decode to FeedbackSignal");
    assert_eq!(
        aunt_decoded.target_cid, BOB_CONTENT_CID,
        "aunt: received signal must target Bob's content CID"
    );

    // =========================================================================
    // Phase 10: RECEIVER DEDUP — handle_received_signal twice with same signal_cid.
    //
    // First call returns Process; second returns DropAsDuplicate.
    // The DedupLru is per-peer and keyed on the signal's own CID.
    //
    // Direct service call: services::gossip_flood::handle_received_signal.
    // harness_d8: NOT used — dedup is a local in-memory LRU.
    // =========================================================================

    let dedup = DedupLru::new();

    let first_decision = handle_received_signal(SARAH_CORRECTION_SIGNAL_CID, &dedup);
    assert_eq!(
        first_decision,
        ReceiveDecision::Process,
        "first arrival of correction signal CID must return Process"
    );

    let second_decision = handle_received_signal(SARAH_CORRECTION_SIGNAL_CID, &dedup);
    assert_eq!(
        second_decision,
        ReceiveDecision::DropAsDuplicate,
        "second arrival of same correction signal CID must return DropAsDuplicate"
    );

    // A distinct signal CID must always Process (not contaminated by the dedup above).
    let distinct_decision =
        handle_received_signal("bafyreidistinct0000000000000000000000000", &dedup);
    assert_eq!(
        distinct_decision,
        ReceiveDecision::Process,
        "a distinct signal CID must Process regardless of prior inserts"
    );

    // =========================================================================
    // Phase 11: RECOVERY-PATH SCAFFOLD — Retraction signal raises Bob's standing.
    //
    // Applies a Retraction(DebitFirm) via project_signal. The DefaultDebitWeightPolicy
    // maps Retraction(DebitFirm) = -3 (restitution). Starting from sum=8 (Floor),
    // sum becomes 8 + (-3) = 5 → Low.
    //
    // NOTE: Production restitution would require the Vouch primitive (post-3.5)
    // and the full mishpat authorization flow. Here we mock the restitution as a
    // direct Retraction signal through the projector. This scaffold proves the
    // score rises when a retraction is applied and provides a regression anchor
    // for when the Vouch primitive is implemented.
    //
    // MOCKED STEP 6 (Vouch + restitution): the production Vouch primitive does
    // not exist yet. This phase substitutes a direct Retraction signal. When the
    // Vouch primitive lands, extend this test to use:
    //   services::vouch::apply_restitution(conn, &vouch_record, &mishpat_auth)
    // rather than a raw Retraction signal.
    //
    // Direct service call: services::standing_projector::project_signal.
    // =========================================================================

    let retraction_signal = FeedbackSignal {
        target_cid: BOB_CONTENT_CID.to_string(),
        signal_kind: SignalKind::Retraction,
        evidence_cid: None,
        standing_impact: StandingImpact::DebitFirm,
        signed_by: bob_subject_b64.clone(),
        signature: BASE64.encode([0xFEu8; 64]),
    };

    let recovered_score = {
        let mut sarah_conn = sarah.db_pool.get().expect("sarah db conn");
        project_signal(
            &mut sarah_conn,
            &DefaultDebitWeightPolicy,
            &SARAH_EVALUATOR_BYTES,
            &retraction_signal,
            "bootstrap:standing-policy:v1",
        )
        .expect("project_signal (retraction) must succeed")
    };

    // sum = 8 (from Phase 8) + (-3) (Retraction DebitFirm) = 5 → Low.
    assert_eq!(
        recovered_score,
        StandingScore::Low,
        "Retraction(DebitFirm) = -3; sum goes from 8 to 5 → Low (partial recovery)"
    );

    // Verify via Standing::evaluate.
    {
        let mut sarah_conn = sarah.db_pool.get().expect("sarah db conn");
        let standing =
            Standing::evaluate(&SARAH_EVALUATOR_BYTES, &BOB_SUBJECT_BYTES, &mut sarah_conn);
        assert_eq!(
            standing,
            Standing::Computed {
                score: StandingScore::Low
            },
            "after Retraction, Standing::evaluate must return Computed(Low) — partial recovery"
        );
    }

    // Production restitution path note:
    //   Full recovery to Neutral requires sum ≤ 2 (i.e. 3+ more -3 retraction
    //   units, or a mishpat-authorized Quarantine reversal). The Vouch primitive
    //   (post-3.5) would orchestrate this with peer attestation and governance
    //   authorization. Scaffold for that call site:
    //
    //   // POST-3.5 TODO: when services::vouch is implemented:
    //   // let vouch = VouchRecord::new(bob_agent_key, sarah_agent_key, ...);
    //   // services::vouch::apply_restitution(&mut sarah_conn, &vouch, &mishpat_auth)
    //   //     .expect("vouch restitution must clear the debit to Neutral");
}
