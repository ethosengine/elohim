//! Plan 4 Task 5, Step 5.4 — byte-parity test across all gossip wire types.
//!
//! Constructs one sample of each gossip wire type, publishes it through a
//! `DualGossipPublisher` wrapping two mock publishers, and asserts:
//! 1. Both mocks receive exactly one call per publish.
//! 2. Both mocks receive byte-identical payloads for the same topic.
//! 3. The payload decodes back to the original struct (round-trip sanity).
//!
//! This satisfies the plan requirement #6 third bullet (byte-parity test)
//! covering: BlobInventorySnapshot, IdentityBindingGossip, RecoveryInvitation,
//! RecoveryRevocationMessage, FeedbackSignal.
//!
//! Gated on `p2p-iroh`.

#![cfg(feature = "p2p-iroh")]

use std::sync::{Arc, Mutex};

use elohim_storage::p2p::feedback_signal::{FeedbackSignal, SignalKind, StandingImpact};
use elohim_storage::p2p::identity_binding_gossip::{IdentityBindingGossip, IDENTITY_BINDING_TOPIC};
use elohim_storage::p2p::inventory_gossip::{BlobInventorySnapshot, INVENTORY_TOPIC};
use elohim_storage::p2p::recovery_invitation::{RecoveryInvitation, RECOVERY_INVITATION_TOPIC};
use elohim_storage::p2p::recovery_revocation::RecoveryRevocationMessage;

const RECOVERY_REVOCATION_TOPIC: &str = "recovery.revocation";
use elohim_storage::p2p_iroh::dual_publish::DualGossipPublisher;
use elohim_storage::services::gossip_flood::{GossipPublisher, PublishError};

// ---------------------------------------------------------------------------
// Capture mock
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
struct CaptureMock {
    calls: Arc<Mutex<Vec<(String, Vec<u8>)>>>,
}

impl CaptureMock {
    fn new() -> Self {
        Self::default()
    }

    fn take_calls(&self) -> Vec<(String, Vec<u8>)> {
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
// Helper: assert byte-parity for one publish
// ---------------------------------------------------------------------------

fn assert_parity(
    libp2p: &CaptureMock,
    iroh: &CaptureMock,
    expected_topic: &str,
    expected_bytes: &[u8],
    label: &str,
) {
    let lp = libp2p.take_calls();
    let ir = iroh.take_calls();

    assert_eq!(lp.len(), 1, "{label}: libp2p mock must receive exactly one call");
    assert_eq!(ir.len(), 1, "{label}: iroh mock must receive exactly one call");

    assert_eq!(lp[0].0, expected_topic, "{label}: libp2p topic mismatch");
    assert_eq!(ir[0].0, expected_topic, "{label}: iroh topic mismatch");

    assert_eq!(
        lp[0].1, expected_bytes,
        "{label}: libp2p payload differs from original bytes"
    );
    assert_eq!(
        ir[0].1, expected_bytes,
        "{label}: iroh payload differs from original bytes"
    );
    assert_eq!(
        lp[0].1, ir[0].1,
        "{label}: libp2p and iroh payloads are not byte-identical"
    );
}

// ---------------------------------------------------------------------------
// Test: BlobInventorySnapshot byte parity
// ---------------------------------------------------------------------------

#[test]
fn inventory_snapshot_byte_parity() {
    let libp2p = CaptureMock::new();
    let iroh = CaptureMock::new();
    let publisher = DualGossipPublisher::new(
        Some(Arc::new(libp2p.clone()) as Arc<dyn GossipPublisher>),
        Some(Arc::new(iroh.clone()) as Arc<dyn GossipPublisher>),
    );

    let snapshot = BlobInventorySnapshot {
        peer_id: "12D3KooWPeer1".to_string(),
        hashes: vec!["a".repeat(64), "b".repeat(64)],
        sequence: 7,
        snapshot_at: 1_700_000_000_000_000,
        signature: vec![1u8; 32],
    };
    let bytes = snapshot.to_bytes().expect("encode");

    publisher.publish(INVENTORY_TOPIC, bytes.clone()).expect("publish");
    assert_parity(&libp2p, &iroh, INVENTORY_TOPIC, &bytes, "BlobInventorySnapshot");

    // Round-trip decode sanity.
    let decoded = BlobInventorySnapshot::from_bytes(&bytes).expect("decode");
    assert_eq!(decoded.peer_id, snapshot.peer_id);
    assert_eq!(decoded.sequence, snapshot.sequence);
}

// ---------------------------------------------------------------------------
// Test: IdentityBindingGossip byte parity
// ---------------------------------------------------------------------------

#[test]
fn identity_binding_byte_parity() {
    let libp2p = CaptureMock::new();
    let iroh = CaptureMock::new();
    let publisher = DualGossipPublisher::new(
        Some(Arc::new(libp2p.clone()) as Arc<dyn GossipPublisher>),
        Some(Arc::new(iroh.clone()) as Arc<dyn GossipPublisher>),
    );

    let binding = IdentityBindingGossip {
        peer_id: "12D3KooWTest".to_string(),
        agent_cid: "bafyrei_agent_x".to_string(),
        valid_from: "2026-05-10T00:00:00Z".to_string(),
        valid_until: None,
        device_archetype: "node".to_string(),
        binding_action_hash: "uhC_action_hash_x".to_string(),
        emitted_at: "2026-05-10T00:00:00Z".to_string(),
        signature: "c3RhZ2UtMS1zaWduYXR1cmU=".to_string(),
    };
    let bytes = binding.to_bytes().expect("encode");

    publisher.publish(IDENTITY_BINDING_TOPIC, bytes.clone()).expect("publish");
    assert_parity(&libp2p, &iroh, IDENTITY_BINDING_TOPIC, &bytes, "IdentityBindingGossip");

    let decoded = IdentityBindingGossip::from_bytes(&bytes).expect("decode");
    assert_eq!(decoded.agent_cid, binding.agent_cid);
}

// ---------------------------------------------------------------------------
// Test: RecoveryInvitation byte parity
// ---------------------------------------------------------------------------

#[test]
fn recovery_invitation_byte_parity() {
    let libp2p = CaptureMock::new();
    let iroh = CaptureMock::new();
    let publisher = DualGossipPublisher::new(
        Some(Arc::new(libp2p.clone()) as Arc<dyn GossipPublisher>),
        Some(Arc::new(iroh.clone()) as Arc<dyn GossipPublisher>),
    );

    let invitation = RecoveryInvitation {
        request_hash: "uhC_req_hash_1".to_string(),
        human_id: "human-001".to_string(),
        created_at: "2026-05-10T00:00:00Z".to_string(),
    };
    let bytes = invitation.to_bytes().expect("encode");

    publisher.publish(RECOVERY_INVITATION_TOPIC, bytes.clone()).expect("publish");
    assert_parity(&libp2p, &iroh, RECOVERY_INVITATION_TOPIC, &bytes, "RecoveryInvitation");

    let decoded = RecoveryInvitation::from_bytes(&bytes).expect("decode");
    assert_eq!(decoded.human_id, invitation.human_id);
}

// ---------------------------------------------------------------------------
// Test: RecoveryRevocationMessage byte parity
// ---------------------------------------------------------------------------

#[test]
fn recovery_revocation_byte_parity() {
    let libp2p = CaptureMock::new();
    let iroh = CaptureMock::new();
    let publisher = DualGossipPublisher::new(
        Some(Arc::new(libp2p.clone()) as Arc<dyn GossipPublisher>),
        Some(Arc::new(iroh.clone()) as Arc<dyn GossipPublisher>),
    );

    let revocation = RecoveryRevocationMessage {
        revocation_id: "rev-002".to_string(),
        human_id: "human-002".to_string(),
        revoked_key: "base64key002".to_string(),
        trigger_type: "steward_vote".to_string(),
        reason: "misconduct".to_string(),
        status: "effective".to_string(),
        sender_peer_id: "12D3KooWSender2".to_string(),
        sent_at: "2026-05-10T00:00:01Z".to_string(),
    };
    let bytes = revocation.to_bytes().expect("encode");

    publisher.publish(RECOVERY_REVOCATION_TOPIC, bytes.clone()).expect("publish");
    assert_parity(&libp2p, &iroh, RECOVERY_REVOCATION_TOPIC, &bytes, "RecoveryRevocationMessage");

    let decoded = RecoveryRevocationMessage::from_bytes(&bytes).expect("decode");
    assert_eq!(decoded.revocation_id, revocation.revocation_id);
    assert_eq!(decoded.status, revocation.status);
}

// ---------------------------------------------------------------------------
// Test: FeedbackSignal byte parity
// ---------------------------------------------------------------------------

#[test]
fn feedback_signal_byte_parity() {
    let libp2p = CaptureMock::new();
    let iroh = CaptureMock::new();
    let publisher = DualGossipPublisher::new(
        Some(Arc::new(libp2p.clone()) as Arc<dyn GossipPublisher>),
        Some(Arc::new(iroh.clone()) as Arc<dyn GossipPublisher>),
    );

    let signal = FeedbackSignal {
        target_cid: "bafyrei_target_003".to_string(),
        signal_kind: SignalKind::Correction,
        vouch_kind: None,
        evidence_cid: Some("bafyrei_evidence_003".to_string()),
        standing_impact: StandingImpact::DebitFirm,
        signed_by: "A".repeat(43),
        signature: "B".repeat(88),
    };
    let bytes = rmp_serde::to_vec_named(&signal).expect("encode FeedbackSignal");
    let topic = "/elohim/feedback-signal/bafyrei_target_003";

    publisher.publish(topic, bytes.clone()).expect("publish");
    assert_parity(&libp2p, &iroh, topic, &bytes, "FeedbackSignal");

    let decoded: FeedbackSignal = rmp_serde::from_slice(&bytes).expect("decode");
    assert_eq!(decoded.target_cid, signal.target_cid);
}
