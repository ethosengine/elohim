//! Integration-level tests for the dual_publish subsystem.
//!
//! These tests exercise cross-module interactions — e.g. verifying that the
//! `DualGossipPublisher` feeds byte-identical payloads to both mocks for
//! topic strings that come from real production paths (feedback-signal,
//! inventory, identity-binding, recovery).
//!
//! Unit tests for individual modules live in their respective `#[cfg(test)]`
//! sections.

use std::sync::Arc;

use crate::p2p::feedback_signal::{FeedbackSignal, SignalKind, StandingImpact};
use crate::p2p::inventory_gossip::BlobInventorySnapshot;
use crate::p2p::identity_binding_gossip::IdentityBindingGossip;
use crate::p2p::recovery_invitation::RecoveryInvitation;
use crate::p2p::recovery_revocation::RecoveryRevocationMessage;
use crate::services::gossip_flood::{GossipPublisher, PublishError};

use super::{DualGossipPublisher, IrohGossipPublisher, TopicTransports, classify_topic};

// ---------------------------------------------------------------------------
// Shared mock
// ---------------------------------------------------------------------------

struct MockGossipPublisher {
    calls: std::sync::Mutex<Vec<(String, Vec<u8>)>>,
}

impl MockGossipPublisher {
    fn new() -> Self {
        Self {
            calls: std::sync::Mutex::new(vec![]),
        }
    }

    fn recorded_calls(&self) -> Vec<(String, Vec<u8>)> {
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
// Test: flood_feedback end-to-end via DualGossipPublisher (Task 3 TDD)
//
// Mimics the `flood_feedback` call path: encodes a FeedbackSignal then
// publishes it to the reach topic. Both libp2p and iroh mocks must receive
// byte-identical payloads.
// ---------------------------------------------------------------------------

#[test]
fn flood_feedback_dual_publish_byte_identical() {
    use crate::services::gossip_flood::flood_feedback;

    let libp2p = Arc::new(MockGossipPublisher::new());
    let iroh = Arc::new(MockGossipPublisher::new());

    let publisher = DualGossipPublisher::new(
        Some(libp2p.clone() as Arc<dyn GossipPublisher>),
        Some(iroh.clone() as Arc<dyn GossipPublisher>),
    );

    let signal = FeedbackSignal {
        target_cid: "bafyrei_test_target_001".to_string(),
        signal_kind: SignalKind::Correction,
        vouch_kind: None,
        evidence_cid: Some("bafyrei_evidence_001".to_string()),
        standing_impact: StandingImpact::DebitSoft,
        signed_by: "A".repeat(43),
        signature: "B".repeat(88),
    };

    let topic = "/elohim/feedback-signal/bafyrei_test_target_001";
    flood_feedback(&signal, "cid-signal-x", topic, &publisher).expect("flood_feedback should succeed");

    let lp_calls = libp2p.recorded_calls();
    let iroh_calls = iroh.recorded_calls();

    assert_eq!(lp_calls.len(), 1, "libp2p must receive exactly one call");
    assert_eq!(iroh_calls.len(), 1, "iroh must receive exactly one call");
    assert_eq!(lp_calls[0].0, topic, "libp2p topic must match");
    assert_eq!(iroh_calls[0].0, topic, "iroh topic must match");
    assert_eq!(
        lp_calls[0].1, iroh_calls[0].1,
        "payloads must be byte-identical across both transports"
    );

    // Verify payload decodes correctly.
    let decoded: FeedbackSignal =
        rmp_serde::from_slice(&lp_calls[0].1).expect("payload must decode to FeedbackSignal");
    assert_eq!(decoded, signal);
}

// ---------------------------------------------------------------------------
// Test: all wire types produce byte-identical payloads on both transports
//
// This covers Task 5 step 5.4 byte-parity requirement for all gossip types.
// ---------------------------------------------------------------------------

#[test]
fn all_wire_types_byte_parity_across_transports() {
    let libp2p = Arc::new(MockGossipPublisher::new());
    let iroh = Arc::new(MockGossipPublisher::new());

    let publisher = DualGossipPublisher::new(
        Some(libp2p.clone() as Arc<dyn GossipPublisher>),
        Some(iroh.clone() as Arc<dyn GossipPublisher>),
    );

    // --- BlobInventorySnapshot ---
    let snapshot = BlobInventorySnapshot {
        peer_id: "peer-abc".to_string(),
        hashes: vec!["0".repeat(64)],
        sequence: 1,
        snapshot_at: 1_700_000_000_000_000,
        signature: vec![0u8; 1],
    };
    let snap_bytes = snapshot.to_bytes().expect("snapshot encode");
    publisher
        .publish("elohim/inventory/blob", snap_bytes.clone())
        .expect("publish inventory snapshot");

    // --- IdentityBindingGossip ---
    let binding = IdentityBindingGossip {
        peer_id: "12D3KooW_test_peer".to_string(),
        agent_cid: "bafyrei_agent_cid".to_string(),
        valid_from: "2026-05-10T00:00:00Z".to_string(),
        valid_until: None,
        device_archetype: "node".to_string(),
        binding_action_hash: "uhC_test_action_hash".to_string(),
        emitted_at: "2026-05-10T00:00:00Z".to_string(),
        signature: "c3RhZ2UtMS1zaWduYXR1cmU=".to_string(),
    };
    let binding_bytes = binding.to_bytes().expect("binding encode");
    publisher
        .publish("elohim/identity/binding", binding_bytes.clone())
        .expect("publish identity binding");

    // --- RecoveryInvitation ---
    let invitation = RecoveryInvitation {
        request_hash: "uhC_recovery_req_hash".to_string(),
        human_id: "human-abc".to_string(),
        created_at: "2026-05-10T00:00:00Z".to_string(),
    };
    let inv_bytes = invitation.to_bytes().expect("invitation encode");
    publisher
        .publish("recovery.invitation", inv_bytes.clone())
        .expect("publish recovery invitation");

    // --- RecoveryRevocationMessage ---
    let revocation = RecoveryRevocationMessage {
        revocation_id: "rev-001".to_string(),
        human_id: "human-abc".to_string(),
        revoked_key: "base64-key-abc".to_string(),
        trigger_type: "voluntary".to_string(),
        reason: "personal".to_string(),
        status: "pending".to_string(),
        sender_peer_id: "12D3KooW_sender".to_string(),
        sent_at: "2026-05-10T00:00:00Z".to_string(),
    };
    let rev_bytes = revocation.to_bytes().expect("revocation encode");
    publisher
        .publish("recovery.revocation", rev_bytes.clone())
        .expect("publish recovery revocation");

    // --- FeedbackSignal ---
    let signal = FeedbackSignal {
        target_cid: "bafyrei_target_002".to_string(),
        signal_kind: SignalKind::Correction,
        vouch_kind: None,
        evidence_cid: None,
        standing_impact: StandingImpact::DebitSoft,
        signed_by: "A".repeat(43),
        signature: "B".repeat(88),
    };
    let sig_bytes = rmp_serde::to_vec_named(&signal).expect("signal encode");
    publisher
        .publish("/elohim/feedback-signal/bafyrei_target_002", sig_bytes.clone())
        .expect("publish feedback signal");

    // Verify: for each call index, libp2p and iroh received byte-identical payload.
    let lp_calls = libp2p.recorded_calls();
    let iroh_calls = iroh.recorded_calls();

    assert_eq!(
        lp_calls.len(),
        5,
        "libp2p should have received 5 publishes"
    );
    assert_eq!(
        iroh_calls.len(),
        5,
        "iroh should have received 5 publishes"
    );

    for (i, (lp, ir)) in lp_calls.iter().zip(iroh_calls.iter()).enumerate() {
        assert_eq!(
            lp.0, ir.0,
            "call {i}: topic must match across transports"
        );
        assert_eq!(
            lp.1, ir.1,
            "call {i}: payload bytes must be identical across transports"
        );
    }
}

// ---------------------------------------------------------------------------
// Test: classify_topic covers all known topics correctly
// ---------------------------------------------------------------------------

#[test]
fn verdict_table_covers_all_known_topics() {
    let cases = [
        "elohim/inventory/blob",
        "elohim/identity/binding",
        "recovery.invitation",
        "recovery.revocation",
        "elohim/integrity/revocation",
        "/elohim/feedback-signal/bafyreiXXX",
        "elohim/lamad/public",
        "elohim/shefa/trusted/coll-1",
    ];
    for topic in &cases {
        assert_eq!(
            classify_topic(topic),
            TopicTransports::Dual,
            "topic '{topic}' must classify as Dual"
        );
    }
}
