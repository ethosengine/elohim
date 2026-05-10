//! Plan 4 Task 6 — recovery-invitation + recovery-revocation dual-publish test.
//!
//! Verifies that `RecoveryInvitation` and `RecoveryRevocationMessage` payloads
//! are published byte-identically to both libp2p and iroh mocks via
//! `DualGossipPublisher`. Wire format is named MessagePack (`to_vec_named`),
//! unchanged from the existing libp2p path.
//!
//! Gated on `p2p-iroh`.

#![cfg(feature = "p2p-iroh")]

use std::sync::{Arc, Mutex};

use elohim_storage::p2p::recovery_invitation::{RecoveryInvitation, RECOVERY_INVITATION_TOPIC};
use elohim_storage::p2p::recovery_revocation::RecoveryRevocationMessage;
use elohim_storage::p2p_iroh::dual_publish::DualGossipPublisher;
use elohim_storage::services::gossip_flood::{GossipPublisher, PublishError};

const RECOVERY_REVOCATION_TOPIC: &str = "recovery.revocation";

#[derive(Clone, Default)]
struct CaptureMock {
    calls: Arc<Mutex<Vec<(String, Vec<u8>)>>>,
}

impl CaptureMock {
    fn new() -> Self {
        Self::default()
    }

    fn calls(&self) -> Vec<(String, Vec<u8>)> {
        self.calls.lock().unwrap().clone()
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
// Test: RecoveryInvitation byte parity across both transports
// ---------------------------------------------------------------------------

#[test]
fn recovery_invitation_dual_publish_byte_parity() {
    let libp2p_sub = CaptureMock::new();
    let iroh_sub = CaptureMock::new();

    let publisher = DualGossipPublisher::new(
        Some(Arc::new(libp2p_sub.clone()) as Arc<dyn GossipPublisher>),
        Some(Arc::new(iroh_sub.clone()) as Arc<dyn GossipPublisher>),
    );

    let invitation = RecoveryInvitation {
        request_hash: "uhC_recovery_req_001".to_string(),
        human_id: "human-recovery-001".to_string(),
        created_at: "2026-05-10T00:00:00Z".to_string(),
    };
    let payload_bytes = invitation.to_bytes().expect("to_bytes should succeed");

    publisher
        .publish(RECOVERY_INVITATION_TOPIC, payload_bytes.clone())
        .expect("DualGossipPublisher should succeed");

    let lp_calls = libp2p_sub.calls();
    let iroh_calls = iroh_sub.calls();

    assert_eq!(lp_calls.len(), 1, "libp2p sub must receive one payload");
    assert_eq!(iroh_calls.len(), 1, "iroh sub must receive one payload");

    assert_eq!(
        lp_calls[0].0, RECOVERY_INVITATION_TOPIC,
        "topic mismatch libp2p"
    );
    assert_eq!(
        iroh_calls[0].0, RECOVERY_INVITATION_TOPIC,
        "topic mismatch iroh"
    );

    assert_eq!(
        lp_calls[0].1, iroh_calls[0].1,
        "both transports must carry byte-identical payloads"
    );

    // Verify round-trip decode.
    let decoded = RecoveryInvitation::from_bytes(&lp_calls[0].1)
        .expect("payload must decode to RecoveryInvitation");
    assert_eq!(decoded, invitation);
}

// ---------------------------------------------------------------------------
// Test: RecoveryRevocationMessage byte parity across both transports
// ---------------------------------------------------------------------------

#[test]
fn recovery_revocation_dual_publish_byte_parity() {
    let libp2p_sub = CaptureMock::new();
    let iroh_sub = CaptureMock::new();

    let publisher = DualGossipPublisher::new(
        Some(Arc::new(libp2p_sub.clone()) as Arc<dyn GossipPublisher>),
        Some(Arc::new(iroh_sub.clone()) as Arc<dyn GossipPublisher>),
    );

    let revocation = RecoveryRevocationMessage {
        revocation_id: "rev-recovery-001".to_string(),
        human_id: "human-recovery-001".to_string(),
        revoked_key: "base64-revoked-key-001".to_string(),
        trigger_type: "voluntary".to_string(),
        reason: "personal".to_string(),
        status: "pending".to_string(),
        sender_peer_id: "12D3KooWSenderRecovery".to_string(),
        sent_at: "2026-05-10T00:00:00Z".to_string(),
    };
    let payload_bytes = revocation.to_bytes().expect("to_bytes should succeed");

    publisher
        .publish(RECOVERY_REVOCATION_TOPIC, payload_bytes.clone())
        .expect("DualGossipPublisher should succeed");

    let lp_calls = libp2p_sub.calls();
    let iroh_calls = iroh_sub.calls();

    assert_eq!(lp_calls.len(), 1, "libp2p sub must receive one payload");
    assert_eq!(iroh_calls.len(), 1, "iroh sub must receive one payload");

    assert_eq!(
        lp_calls[0].0, RECOVERY_REVOCATION_TOPIC,
        "topic mismatch libp2p"
    );
    assert_eq!(
        iroh_calls[0].0, RECOVERY_REVOCATION_TOPIC,
        "topic mismatch iroh"
    );

    assert_eq!(
        lp_calls[0].1, iroh_calls[0].1,
        "both transports must carry byte-identical payloads"
    );

    // Verify round-trip decode.
    let decoded = RecoveryRevocationMessage::from_bytes(&lp_calls[0].1)
        .expect("payload must decode to RecoveryRevocationMessage");
    assert_eq!(decoded, revocation);
}
