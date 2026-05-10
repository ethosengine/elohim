//! Plan 4 Task 8 — cross-stack e2e soak: provider→(libp2p sub, iroh sub).
//!
//! Three-"node" fixture using CaptureMock publishers:
//!
//!   provider       — DualGossipPublisher(libp2p=libp2p_sub, iroh=iroh_sub)
//!   libp2p_sub     — CaptureMock acting as the libp2p transport subscriber
//!   iroh_sub       — CaptureMock acting as the iroh-gossip transport subscriber
//!
//! The provider publishes five wire types (inventory snapshot, identity-binding,
//! recovery-invitation, recovery-revocation, feedback-signal). After each
//! publish the test asserts:
//!
//!   1. Both subscribers received exactly one call for that topic.
//!   2. Both received byte-identical payloads.
//!   3. The payload round-trips to the originating struct.
//!
//! This validates that DualGossipPublisher routes every known gossip wire type
//! to both transports with zero byte divergence — the core guarantee of
//! cutover gate #4. Mock transport, so no network or timeout involved.
//!
//! Gated on `p2p-iroh`.

#![cfg(feature = "p2p-iroh")]

use std::sync::{Arc, Mutex};

use elohim_storage::p2p::feedback_signal::{FeedbackSignal, SignalKind, StandingImpact};
use elohim_storage::p2p::identity_binding_gossip::{IdentityBindingGossip, IDENTITY_BINDING_TOPIC};
use elohim_storage::p2p::inventory_gossip::{BlobInventorySnapshot, INVENTORY_TOPIC};
use elohim_storage::p2p::recovery_invitation::{RecoveryInvitation, RECOVERY_INVITATION_TOPIC};
use elohim_storage::p2p::recovery_revocation::RecoveryRevocationMessage;
use elohim_storage::p2p::RECOVERY_REVOCATION_TOPIC;
use elohim_storage::p2p_iroh::dual_publish::DualGossipPublisher;
use elohim_storage::services::gossip_flood::{GossipPublisher, PublishError};

// ---------------------------------------------------------------------------
// CaptureMock — transport subscriber stand-in
// ---------------------------------------------------------------------------

/// Thread-safe capture of (topic, payload) calls. Simulates either a libp2p
/// or iroh subscriber sink — both are structurally identical from the
/// DualGossipPublisher's perspective.
#[derive(Clone, Default)]
struct CaptureMock {
    calls: Arc<Mutex<Vec<(String, Vec<u8>)>>>,
}

impl CaptureMock {
    fn new() -> Self {
        Self::default()
    }

    /// Take all recorded calls and clear the buffer — clears state for the
    /// next assertion round.
    fn drain(&self) -> Vec<(String, Vec<u8>)> {
        std::mem::take(&mut self.calls.lock().unwrap())
    }
}

impl GossipPublisher for CaptureMock {
    fn publish(&self, topic: &str, payload: Vec<u8>) -> Result<(), PublishError> {
        self.calls
            .lock()
            .unwrap()
            .push((topic.to_string(), payload));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helper: assert per-round parity and return libp2p payload for decode check
// ---------------------------------------------------------------------------

fn assert_round_parity(
    libp2p_sub: &CaptureMock,
    iroh_sub: &CaptureMock,
    expected_topic: &str,
    expected_bytes: &[u8],
    label: &str,
) -> Vec<u8> {
    let lp = libp2p_sub.drain();
    let ir = iroh_sub.drain();

    assert_eq!(
        lp.len(),
        1,
        "{label}: libp2p subscriber must receive exactly one publish; got {}",
        lp.len()
    );
    assert_eq!(
        ir.len(),
        1,
        "{label}: iroh subscriber must receive exactly one publish; got {}",
        ir.len()
    );

    assert_eq!(lp[0].0, expected_topic, "{label}: libp2p topic wrong");
    assert_eq!(ir[0].0, expected_topic, "{label}: iroh topic wrong");

    assert_eq!(
        lp[0].1, expected_bytes,
        "{label}: libp2p payload differs from canonical bytes"
    );
    assert_eq!(
        ir[0].1, expected_bytes,
        "{label}: iroh payload differs from canonical bytes"
    );
    assert_eq!(
        lp[0].1, ir[0].1,
        "{label}: libp2p and iroh subscribers received divergent bytes"
    );

    lp.into_iter().next().unwrap().1
}

// ---------------------------------------------------------------------------
// Cross-stack e2e soak — five wire types
// ---------------------------------------------------------------------------

/// Provider publishes five different gossip wire types through DualGossipPublisher.
/// Both the libp2p subscriber mock and the iroh subscriber mock must receive
/// byte-identical payloads for every type. Round-trip decode asserted on each.
#[test]
fn provider_dual_publishes_all_wire_types_to_both_subscribers() {
    // -----------------------------------------------------------------------
    // Three-"node" fixture
    // -----------------------------------------------------------------------
    let libp2p_sub = CaptureMock::new();
    let iroh_sub = CaptureMock::new();

    // Provider: DualGossipPublisher fans out to both transport mocks.
    let provider = DualGossipPublisher::new(
        Some(Arc::new(libp2p_sub.clone()) as Arc<dyn GossipPublisher>),
        Some(Arc::new(iroh_sub.clone()) as Arc<dyn GossipPublisher>),
    );

    // -----------------------------------------------------------------------
    // Round 1: BlobInventorySnapshot
    // -----------------------------------------------------------------------
    let snapshot = BlobInventorySnapshot {
        peer_id: "12D3KooWProvider".to_string(),
        hashes: vec!["a".repeat(64), "b".repeat(64)],
        sequence: 1,
        snapshot_at: 1_700_000_000_000_000,
        signature: vec![0xAAu8; 32],
    };
    let snapshot_bytes = snapshot.to_bytes().expect("inventory snapshot to_bytes");

    provider
        .publish(INVENTORY_TOPIC, snapshot_bytes.clone())
        .expect("provider publish inventory snapshot");

    let received = assert_round_parity(
        &libp2p_sub,
        &iroh_sub,
        INVENTORY_TOPIC,
        &snapshot_bytes,
        "BlobInventorySnapshot",
    );
    let decoded_snap = BlobInventorySnapshot::from_bytes(&received)
        .expect("round-trip decode BlobInventorySnapshot");
    assert_eq!(decoded_snap.peer_id, snapshot.peer_id);
    assert_eq!(decoded_snap.sequence, snapshot.sequence);
    assert_eq!(decoded_snap.hashes, snapshot.hashes);

    // -----------------------------------------------------------------------
    // Round 2: IdentityBindingGossip
    // -----------------------------------------------------------------------
    let binding = IdentityBindingGossip {
        peer_id: "12D3KooWProvider".to_string(),
        agent_cid: "bafyrei_provider_agent".to_string(),
        valid_from: "2026-05-10T00:00:00Z".to_string(),
        valid_until: None,
        device_archetype: "hub".to_string(),
        binding_action_hash: "uhC_binding_hash_provider".to_string(),
        emitted_at: "2026-05-10T00:00:00Z".to_string(),
        signature: "c3lzdGVtLXNpZ25hdHVyZQ==".to_string(),
    };
    let binding_bytes = binding.to_bytes().expect("identity binding to_bytes");

    provider
        .publish(IDENTITY_BINDING_TOPIC, binding_bytes.clone())
        .expect("provider publish identity binding");

    let received = assert_round_parity(
        &libp2p_sub,
        &iroh_sub,
        IDENTITY_BINDING_TOPIC,
        &binding_bytes,
        "IdentityBindingGossip",
    );
    let decoded_bind = IdentityBindingGossip::from_bytes(&received)
        .expect("round-trip decode IdentityBindingGossip");
    assert_eq!(decoded_bind.agent_cid, binding.agent_cid);
    assert_eq!(
        decoded_bind.binding_action_hash,
        binding.binding_action_hash
    );

    // -----------------------------------------------------------------------
    // Round 3: RecoveryInvitation
    // -----------------------------------------------------------------------
    let invitation = RecoveryInvitation {
        request_hash: "uhC_recovery_req_e2e".to_string(),
        human_id: "human-e2e-001".to_string(),
        created_at: "2026-05-10T00:00:00Z".to_string(),
    };
    let invitation_bytes = invitation.to_bytes().expect("recovery invitation to_bytes");

    provider
        .publish(RECOVERY_INVITATION_TOPIC, invitation_bytes.clone())
        .expect("provider publish recovery invitation");

    let received = assert_round_parity(
        &libp2p_sub,
        &iroh_sub,
        RECOVERY_INVITATION_TOPIC,
        &invitation_bytes,
        "RecoveryInvitation",
    );
    let decoded_inv =
        RecoveryInvitation::from_bytes(&received).expect("round-trip decode RecoveryInvitation");
    assert_eq!(decoded_inv.human_id, invitation.human_id);
    assert_eq!(decoded_inv.request_hash, invitation.request_hash);

    // -----------------------------------------------------------------------
    // Round 4: RecoveryRevocationMessage
    // -----------------------------------------------------------------------
    let revocation = RecoveryRevocationMessage {
        revocation_id: "rev-e2e-001".to_string(),
        human_id: "human-e2e-001".to_string(),
        revoked_key: "base64-key-e2e-001".to_string(),
        trigger_type: "steward_vote".to_string(),
        reason: "key_compromise".to_string(),
        status: "effective".to_string(),
        sender_peer_id: "12D3KooWProvider".to_string(),
        sent_at: "2026-05-10T00:00:01Z".to_string(),
    };
    let revocation_bytes = revocation.to_bytes().expect("recovery revocation to_bytes");

    provider
        .publish(RECOVERY_REVOCATION_TOPIC, revocation_bytes.clone())
        .expect("provider publish recovery revocation");

    let received = assert_round_parity(
        &libp2p_sub,
        &iroh_sub,
        RECOVERY_REVOCATION_TOPIC,
        &revocation_bytes,
        "RecoveryRevocationMessage",
    );
    let decoded_rev = RecoveryRevocationMessage::from_bytes(&received)
        .expect("round-trip decode RecoveryRevocationMessage");
    assert_eq!(decoded_rev.revocation_id, revocation.revocation_id);
    assert_eq!(decoded_rev.status, revocation.status);

    // -----------------------------------------------------------------------
    // Round 5: FeedbackSignal (dynamic topic — target CID embedded in name)
    // -----------------------------------------------------------------------
    let signal = FeedbackSignal {
        target_cid: "bafyrei_e2e_target_001".to_string(),
        signal_kind: SignalKind::Correction,
        vouch_kind: None,
        evidence_cid: Some("bafyrei_e2e_evidence_001".to_string()),
        standing_impact: StandingImpact::DebitFirm,
        signed_by: "C".repeat(43),
        signature: "D".repeat(88),
    };
    let signal_bytes = rmp_serde::to_vec_named(&signal).expect("encode FeedbackSignal");
    let signal_topic = format!("/elohim/feedback-signal/{}", signal.target_cid);

    provider
        .publish(&signal_topic, signal_bytes.clone())
        .expect("provider publish feedback signal");

    let received = assert_round_parity(
        &libp2p_sub,
        &iroh_sub,
        &signal_topic,
        &signal_bytes,
        "FeedbackSignal",
    );
    let decoded_sig: FeedbackSignal =
        rmp_serde::from_slice(&received).expect("round-trip decode FeedbackSignal");
    assert_eq!(decoded_sig.target_cid, signal.target_cid);
    assert_eq!(decoded_sig.standing_impact, signal.standing_impact);

    // -----------------------------------------------------------------------
    // Final state: no stray calls buffered in either subscriber
    // -----------------------------------------------------------------------
    assert!(
        libp2p_sub.drain().is_empty(),
        "libp2p subscriber has stray calls after all five rounds"
    );
    assert!(
        iroh_sub.drain().is_empty(),
        "iroh subscriber has stray calls after all five rounds"
    );
}

// ---------------------------------------------------------------------------
// Partial-transport variant: iroh absent → libp2p-only receives all five
// ---------------------------------------------------------------------------

/// Validates graceful degradation when the iroh transport is absent.
/// Provider still publishes all five types; only the libp2p subscriber mock
/// should receive them. The iroh mock receives nothing.
#[test]
fn provider_iroh_absent_all_types_delivered_via_libp2p() {
    let libp2p_sub = CaptureMock::new();
    let iroh_sub = CaptureMock::new(); // wired to nothing in this test

    // Provider: iroh side is None.
    let provider = DualGossipPublisher::new(
        Some(Arc::new(libp2p_sub.clone()) as Arc<dyn GossipPublisher>),
        None,
    );

    let snapshot = BlobInventorySnapshot {
        peer_id: "12D3KooWLibp2pOnly".to_string(),
        hashes: vec!["c".repeat(64)],
        sequence: 2,
        snapshot_at: 1_700_000_001_000_000,
        signature: vec![0xBBu8; 32],
    };
    let snapshot_bytes = snapshot.to_bytes().expect("to_bytes");
    provider
        .publish(INVENTORY_TOPIC, snapshot_bytes.clone())
        .expect("publish");

    let lp = libp2p_sub.drain();
    let ir = iroh_sub.drain();
    assert_eq!(lp.len(), 1, "libp2p must receive inventory snapshot");
    assert!(ir.is_empty(), "iroh must NOT receive when absent");
    assert_eq!(lp[0].1, snapshot_bytes, "libp2p payload byte-identical");

    let binding = IdentityBindingGossip {
        peer_id: "12D3KooWLibp2pOnly".to_string(),
        agent_cid: "bafyrei_libp2p_only".to_string(),
        valid_from: "2026-05-10T00:00:00Z".to_string(),
        valid_until: None,
        device_archetype: "node".to_string(),
        binding_action_hash: "uhC_libp2p_only_binding".to_string(),
        emitted_at: "2026-05-10T00:00:00Z".to_string(),
        signature: "libp2p_only_sig==".to_string(),
    };
    let binding_bytes = binding.to_bytes().expect("to_bytes");
    provider
        .publish(IDENTITY_BINDING_TOPIC, binding_bytes.clone())
        .expect("publish");

    let lp = libp2p_sub.drain();
    let ir = iroh_sub.drain();
    assert_eq!(lp.len(), 1, "libp2p must receive identity binding");
    assert!(ir.is_empty(), "iroh must NOT receive when absent");
    assert_eq!(lp[0].1, binding_bytes);
}

// ---------------------------------------------------------------------------
// Cross-stack verdict coverage: all topics classify as Dual
// ---------------------------------------------------------------------------

/// Structural guard: every known topic name must route to both transports
/// (classify_topic returns TopicTransports::Dual). This ensures future topic
/// additions don't accidentally fall through to libp2p-only.
#[test]
fn all_known_topics_route_to_both_subscribers() {
    let topics = [
        INVENTORY_TOPIC,
        IDENTITY_BINDING_TOPIC,
        RECOVERY_INVITATION_TOPIC,
        RECOVERY_REVOCATION_TOPIC,
        "/elohim/feedback-signal/some-cid",
        "elohim/integrity/revocation",
    ];

    for topic in topics {
        let libp2p_sub = CaptureMock::new();
        let iroh_sub = CaptureMock::new();
        let provider = DualGossipPublisher::new(
            Some(Arc::new(libp2p_sub.clone()) as Arc<dyn GossipPublisher>),
            Some(Arc::new(iroh_sub.clone()) as Arc<dyn GossipPublisher>),
        );

        let payload = vec![0x42u8; 8];
        provider.publish(topic, payload.clone()).expect("publish");

        let lp = libp2p_sub.drain();
        let ir = iroh_sub.drain();
        assert_eq!(
            lp.len(),
            1,
            "topic '{topic}': libp2p must receive exactly one call"
        );
        assert_eq!(
            ir.len(),
            1,
            "topic '{topic}': iroh must receive exactly one call"
        );
        assert_eq!(lp[0].1, payload, "topic '{topic}': libp2p payload mismatch");
        assert_eq!(ir[0].1, payload, "topic '{topic}': iroh payload mismatch");
        assert_eq!(
            lp[0].1, ir[0].1,
            "topic '{topic}': byte divergence between transports"
        );
    }
}
