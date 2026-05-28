//! Step zero: ConductorAgentInfo dual-publish byte-parity test.
//!
//! Verifies that publishing a `ConductorAgentInfo` through a
//! `DualGossipPublisher` delivers byte-identical payloads to both the libp2p
//! mock and the iroh mock. The wire format is named-field MessagePack
//! (`rmp_serde::to_vec_named`), matching the inventory + recovery payload
//! convention (see CATALOG row #1, #2, #4).
//!
//! Mirrors `iroh_gossip_dual_publish_identity_binding.rs`. Gated on `p2p-iroh`.

#![cfg(feature = "p2p-iroh")]

use std::sync::{Arc, Mutex};

use elohim_storage::p2p::conductor_agent_info_gossip::{
    ConductorAgentInfo, CONDUCTOR_AGENT_INFO_TOPIC,
};
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

#[test]
fn conductor_agent_info_dual_publish_byte_parity() {
    let libp2p_sub = CaptureMock::new();
    let iroh_sub = CaptureMock::new();

    let publisher = DualGossipPublisher::new(
        Some(Arc::new(libp2p_sub.clone()) as Arc<dyn GossipPublisher>),
        Some(Arc::new(iroh_sub.clone()) as Arc<dyn GossipPublisher>),
    );

    let payload = ConductorAgentInfo {
        agent_info_json: r#"{"agent":"uhCAk_dual_publish_test_pubkey","space":"uhC0k_dual_publish_test_space","urls":["wss://signal.elohim.host/uhCAk_dual_publish_test_pubkey"],"expires_at":1234567890}"#.to_string(),
        published_at: 1_700_000_000_000_000,
    };
    let bytes = payload.to_bytes().expect("to_bytes should succeed");

    publisher
        .publish(CONDUCTOR_AGENT_INFO_TOPIC, bytes.clone())
        .expect("DualGossipPublisher should succeed");

    let lp_calls = libp2p_sub.calls();
    let iroh_calls = iroh_sub.calls();

    assert_eq!(lp_calls.len(), 1, "libp2p sub must receive one payload");
    assert_eq!(iroh_calls.len(), 1, "iroh sub must receive one payload");

    assert_eq!(lp_calls[0].0, CONDUCTOR_AGENT_INFO_TOPIC);
    assert_eq!(iroh_calls[0].0, CONDUCTOR_AGENT_INFO_TOPIC);

    // Byte parity — both transports must receive the SAME bytes.
    assert_eq!(
        lp_calls[0].1, iroh_calls[0].1,
        "both transports must carry byte-identical payloads"
    );
    assert_eq!(
        lp_calls[0].1, bytes,
        "libp2p sub received different bytes than published"
    );

    // Both must decode back to the same struct.
    let decoded_lp = ConductorAgentInfo::from_bytes(&lp_calls[0].1)
        .expect("libp2p payload must decode to ConductorAgentInfo");
    let decoded_iroh = ConductorAgentInfo::from_bytes(&iroh_calls[0].1)
        .expect("iroh payload must decode to ConductorAgentInfo");

    assert_eq!(decoded_lp, payload, "libp2p decoded must equal original");
    assert_eq!(decoded_iroh, payload, "iroh decoded must equal original");
}
