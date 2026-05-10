//! Plan 4 Task 5, Step 5.5 — identity-binding dual-publish integration test.
//!
//! Verifies that publishing an `IdentityBindingGossip` through a
//! `DualGossipPublisher` delivers byte-identical payloads to both the libp2p
//! mock and the iroh mock. The wire format is positional MessagePack
//! (`rmp_serde::to_vec`), unchanged from the existing libp2p path.
//!
//! Gated on `p2p-iroh`.

#![cfg(feature = "p2p-iroh")]

use std::sync::{Arc, Mutex};

use elohim_storage::p2p::identity_binding_gossip::{IdentityBindingGossip, IDENTITY_BINDING_TOPIC};
use elohim_storage::p2p_iroh::dual_publish::DualGossipPublisher;
use elohim_storage::services::gossip_flood::{GossipPublisher, PublishError};

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

/// Provider holds `DualGossipPublisher(libp2p, iroh)`, publishes an
/// `IdentityBindingGossip`. Both mocks must receive the payload and it
/// must decode to the same struct.
#[test]
fn identity_binding_dual_publish_byte_parity() {
    let libp2p_sub = CaptureMock::new();
    let iroh_sub = CaptureMock::new();

    let publisher = DualGossipPublisher::new(
        Some(Arc::new(libp2p_sub.clone()) as Arc<dyn GossipPublisher>),
        Some(Arc::new(iroh_sub.clone()) as Arc<dyn GossipPublisher>),
    );

    let binding = IdentityBindingGossip {
        peer_id: "12D3KooWDualPublishTest".to_string(),
        agent_cid: "bafyrei_dual_agent".to_string(),
        valid_from: "2026-05-10T00:00:00Z".to_string(),
        valid_until: None,
        device_archetype: "node".to_string(),
        binding_action_hash: "uhC_dual_action_hash".to_string(),
        emitted_at: "2026-05-10T00:00:00Z".to_string(),
        signature: "c3RhZ2UtMS1zaWduYXR1cmU=".to_string(),
    };

    let payload_bytes = binding.to_bytes().expect("to_bytes should succeed");

    publisher
        .publish(IDENTITY_BINDING_TOPIC, payload_bytes.clone())
        .expect("DualGossipPublisher should succeed");

    // Both subscribers must receive the payload.
    let lp_calls = libp2p_sub.calls();
    let iroh_calls = iroh_sub.calls();

    assert_eq!(lp_calls.len(), 1, "libp2p sub must receive one payload");
    assert_eq!(iroh_calls.len(), 1, "iroh sub must receive one payload");

    // Topic must be correct.
    assert_eq!(lp_calls[0].0, IDENTITY_BINDING_TOPIC);
    assert_eq!(iroh_calls[0].0, IDENTITY_BINDING_TOPIC);

    // Payloads must be byte-identical.
    assert_eq!(
        lp_calls[0].1, iroh_calls[0].1,
        "both transports must carry byte-identical payloads"
    );
    assert_eq!(lp_calls[0].1, payload_bytes);

    // Both must decode to the same struct.
    let decoded_lp = IdentityBindingGossip::from_bytes(&lp_calls[0].1)
        .expect("libp2p payload must decode to IdentityBindingGossip");
    let decoded_iroh = IdentityBindingGossip::from_bytes(&iroh_calls[0].1)
        .expect("iroh payload must decode to IdentityBindingGossip");

    assert_eq!(decoded_lp, binding, "libp2p decoded must equal original");
    assert_eq!(decoded_iroh, binding, "iroh decoded must equal original");
}
