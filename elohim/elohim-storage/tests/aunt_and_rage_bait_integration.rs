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
use elohim_storage::p2p::feedback_signal::{FeedbackSignal, SignalKind, StandingImpact, VouchKind};
use elohim_storage::services::back_prop::{
    back_prop_one_hop, record_predecessor, SealingPubKeys, UnsealingKeys,
};
use elohim_storage::services::bootstrap_manifests::seed_if_empty;
use elohim_storage::services::epr_compose::{compose_epr, ComposeError};
use elohim_storage::services::epr_kind::Reach;
use elohim_storage::services::gossip_flood::{
    flood_feedback, handle_received_signal, GossipPublisher, PublishError, ReceiveDecision,
};
use elohim_storage::services::manifest_registry::ManifestRegistry;
use elohim_storage::services::reach_earning::{BlockReason, ReachVerdict};
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

    // 2-of-2 negative case: substituting a wrong imagodei secret key MUST fail.
    // (T11 unit tests cover this exhaustively — this assertion verifies the
    // integration path uses the same crypto, not a degraded variant.)
    {
        let (_wrong_imagodei_pk, wrong_imagodei_sk) = crypto_box_seed_keypair(&[42u8; 32]);
        let bad_decrypt = unseal(
            &sealed,
            &mishpat_pk,
            &mishpat_sk,
            &imagodei_pk,
            &ImagodeiSecretKey(wrong_imagodei_sk),
        );
        assert!(
            bad_decrypt.is_err(),
            "decrypt with wrong imagodei secret key MUST fail (2-of-2 property)"
        );
    }

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
    // Phase 5: Reach-earning gate — deferred to after Phase 8.
    //
    // compose_epr(District) requires Bob's standing to be evaluated.
    // Bob's standing row is written in Phase 8 (project_signal). The gate
    // check is placed there, immediately after the standing projection, where
    // it can assert Blocked(StandingBelowThreshold) rather than Pending
    // (which would be the verdict when the standing row does not yet exist).
    //
    // See "Phase 5 gate check" immediately after Phase 8 below.
    // =========================================================================

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
    // Phase 5 gate check (deferred from Phase 5 block above).
    //
    // Now that Bob's standing row is projected (Computed { score: Floor }),
    // the reach-earning gate can return a definitive verdict:
    //   - Reach::District threshold = "neutral" per bootstrap-standing-policy.
    //   - Bob's score (Floor) < Neutral → Blocked(StandingBelowThreshold).
    //
    // The ManifestRegistry is loaded from Sarah's db (bootstrap standing-policy
    // manifest seeded in Phase 2 via seed_if_empty).
    //
    // Direct service call: services::epr_compose::compose_epr.
    // harness_d8: NOT used — gate is a local computation.
    // =========================================================================

    {
        let registry = ManifestRegistry::new();
        {
            let mut sarah_conn = sarah.db_pool.get().expect("sarah db conn for registry");
            registry
                .load_from_db(&mut sarah_conn)
                .expect("load_from_db must succeed after seed_if_empty");
        }
        let mut sarah_conn = sarah.db_pool.get().expect("sarah db conn for compose_epr");
        let gate_result = compose_epr(
            &SARAH_EVALUATOR_BYTES,
            &BOB_SUBJECT_BYTES,
            Reach::District,
            &mut sarah_conn,
            &registry,
        );
        match gate_result {
            Err(ComposeError::ReachDenied(ReachVerdict::Blocked {
                reason: BlockReason::StandingBelowThreshold,
                ..
            })) => {}
            other => panic!(
                "Phase 5: expected ReachDenied(Blocked(StandingBelowThreshold)) for Bob at \
                 District reach after Correction, got: {other:?}"
            ),
        }
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
    // Phase 11: VOUCH — Sarah vouches for Bob, recovering his standing.
    //
    // The Vouch primitive is a FeedbackSignal with signal_kind = Vouch and
    // vouch_kind = AcceptCorrection. Sarah (the third-party voucher) issues
    // two Vouch(DebitSoft) signals targeting BOB_CONTENT_CID. Each carries
    // DefaultDebitWeightPolicy weight = -3.
    //
    // Starting debit_weight_sum = 8 (from Phase 8 Correction(DebitFirm)):
    //   vouch 1: sum = 8 + (-3) = 5  → Low
    //   vouch 2: sum = 5 + (-3) = 2  → Neutral
    //
    // Neutral satisfies the District threshold ("neutral" in bootstrap policy),
    // so Bob's subsequent compose_epr(District) must return Ok.
    //
    // No-self-vouch constraint: signed_by is set to BOB_SUBJECT_BYTES (same
    // HONEST DEVIATION as Phase 6) so that project_signal targets Bob's
    // standing_view row. In production, the projector resolves the content
    // author from the EPR store; that lookup is post-3.5. The no-self-vouch
    // invariant (signed_by ≠ vouched-subject's key) is enforced at the
    // coordinator/integrity-zome level (T7 unit tests). Sarah's identity as
    // the issuer is carried by the fact that this signal arrives from Sarah's
    // node — the test models this by constructing the signal on the Sarah path.
    //
    // Direct service call: services::standing_projector::project_signal (×2).
    // harness_d8: NOT used — projection is local to Sarah's DB.
    // =========================================================================

    // Vouch #1: sum = 8 → 5 (Low)
    let vouch_signal_1 = FeedbackSignal {
        target_cid: BOB_CONTENT_CID.to_string(),
        signal_kind: SignalKind::Vouch,
        vouch_kind: Some(VouchKind::AcceptCorrection),
        evidence_cid: None,
        standing_impact: StandingImpact::DebitSoft,
        signed_by: bob_subject_b64.clone(),
        signature: BASE64.encode([0xAAu8; 64]),
    };
    vouch_signal_1
        .validate()
        .expect("vouch_signal_1 must pass validate()");

    let score_after_vouch_1 = {
        let mut sarah_conn = sarah.db_pool.get().expect("sarah db conn");
        project_signal(
            &mut sarah_conn,
            &DefaultDebitWeightPolicy,
            &SARAH_EVALUATOR_BYTES,
            &vouch_signal_1,
            "bootstrap:standing-policy:v1",
        )
        .expect("project_signal (vouch 1) must succeed")
    };

    // sum = 8 + (-3) = 5 → Low
    assert_eq!(
        score_after_vouch_1,
        StandingScore::Low,
        "Vouch(DebitSoft) #1: sum = 8 + (-3) = 5 → Low"
    );

    // Vouch #2: sum = 5 → 2 (Neutral)
    let vouch_signal_2 = FeedbackSignal {
        target_cid: BOB_CONTENT_CID.to_string(),
        signal_kind: SignalKind::Vouch,
        vouch_kind: Some(VouchKind::AcceptCorrection),
        evidence_cid: None,
        standing_impact: StandingImpact::DebitSoft,
        signed_by: bob_subject_b64.clone(),
        signature: BASE64.encode([0xABu8; 64]),
    };
    vouch_signal_2
        .validate()
        .expect("vouch_signal_2 must pass validate()");

    let score_after_vouch_2 = {
        let mut sarah_conn = sarah.db_pool.get().expect("sarah db conn");
        project_signal(
            &mut sarah_conn,
            &DefaultDebitWeightPolicy,
            &SARAH_EVALUATOR_BYTES,
            &vouch_signal_2,
            "bootstrap:standing-policy:v1",
        )
        .expect("project_signal (vouch 2) must succeed")
    };

    // sum = 5 + (-3) = 2 → Neutral (boundary: 0..=2 → Neutral)
    assert_eq!(
        score_after_vouch_2,
        StandingScore::Neutral,
        "Vouch(DebitSoft) #2: sum = 5 + (-3) = 2 → Neutral (District threshold)"
    );

    // Verify via Standing::evaluate.
    {
        let mut sarah_conn = sarah.db_pool.get().expect("sarah db conn");
        let standing =
            Standing::evaluate(&SARAH_EVALUATOR_BYTES, &BOB_SUBJECT_BYTES, &mut sarah_conn);
        assert_eq!(
            standing,
            Standing::Computed {
                score: StandingScore::Neutral
            },
            "after two Vouch(DebitSoft) signals, Standing::evaluate must return Computed(Neutral)"
        );
    }

    // =========================================================================
    // Phase 12: COMPOSE RECOVERY — Bob's District compose now succeeds.
    //
    // After the two vouch signals, Bob's debit_weight_sum = 2 → Neutral.
    // The District threshold is "neutral" in the bootstrap standing-policy.
    // Neutral ≥ Neutral → compose_epr must return Ok.
    //
    // Direct service call: services::epr_compose::compose_epr.
    // =========================================================================

    {
        let registry = ManifestRegistry::new();
        {
            let mut sarah_conn = sarah.db_pool.get().expect("sarah db conn for registry");
            registry
                .load_from_db(&mut sarah_conn)
                .expect("load_from_db must succeed for compose recovery check");
        }
        let mut sarah_conn = sarah
            .db_pool
            .get()
            .expect("sarah db conn for compose recovery");
        let recovery_result = compose_epr(
            &SARAH_EVALUATOR_BYTES,
            &BOB_SUBJECT_BYTES,
            Reach::District,
            &mut sarah_conn,
            &registry,
        );
        assert!(
            recovery_result.is_ok(),
            "after Vouch recovery (sum=2 → Neutral), compose_epr(District) must return Ok; \
             got: {recovery_result:?}"
        );
    }
}
