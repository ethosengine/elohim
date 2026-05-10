//! Integration test — Plan 4 Task 4: inventory snapshot via DualGossipPublisher.
//!
//! Verifies that `P2PNode::broadcast_inventory_snapshot` (via the
//! `DualGossipPublisher` wired through `set_gossip_publisher`) delivers
//! byte-identical payloads to both the libp2p mock and the iroh mock.
//!
//! This test uses synchronous mocks (not a live swarm) because:
//! - Live two-stack tests run under Task 8 (cross-stack e2e soak).
//! - The value here is validating that the correct bytes flow from
//!   `BlobInventorySnapshot::to_bytes()` through `DualGossipPublisher`
//!   to both transports.
//!
//! Gated on `p2p-iroh`.

#![cfg(feature = "p2p-iroh")]

use std::sync::{Arc, Mutex};

use elohim_storage::p2p::inventory_gossip::{BlobInventorySnapshot, INVENTORY_TOPIC};
use elohim_storage::p2p_iroh::dual_publish::DualGossipPublisher;
use elohim_storage::services::gossip_flood::{GossipPublisher, PublishError};

// ---------------------------------------------------------------------------
// Mock publisher (captures (topic, payload) pairs for assertion)
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
struct CapturePublisher {
    calls: Arc<Mutex<Vec<(String, Vec<u8>)>>>,
}

impl CapturePublisher {
    fn new() -> Self {
        Self::default()
    }

    fn recorded_calls(&self) -> Vec<(String, Vec<u8>)> {
        self.calls.lock().unwrap().clone()
    }
}

impl GossipPublisher for CapturePublisher {
    fn publish(&self, topic: &str, payload: Vec<u8>) -> Result<(), PublishError> {
        self.calls
            .lock()
            .unwrap()
            .push((topic.to_string(), payload));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Test: DualGossipPublisher delivers inventory snapshot byte-identically
// ---------------------------------------------------------------------------

#[test]
fn dual_publisher_routes_inventory_snapshot_to_both_transports() {
    let libp2p_mock = CapturePublisher::new();
    let iroh_mock = CapturePublisher::new();

    let publisher = DualGossipPublisher::new(
        Some(Arc::new(libp2p_mock.clone()) as Arc<dyn GossipPublisher>),
        Some(Arc::new(iroh_mock.clone()) as Arc<dyn GossipPublisher>),
    );

    // Build a realistic inventory snapshot.
    let snapshot = BlobInventorySnapshot {
        peer_id: "12D3KooWTestPeer".to_string(),
        hashes: vec!["a".repeat(64), "b".repeat(64), "c".repeat(64)],
        sequence: 42,
        snapshot_at: 1_700_000_000_000_000,
        signature: vec![0u8; 32],
    };

    let bytes = snapshot
        .to_bytes()
        .expect("BlobInventorySnapshot should encode");

    publisher
        .publish(INVENTORY_TOPIC, bytes.clone())
        .expect("DualGossipPublisher should succeed");

    // Both transports must receive exactly one call with byte-identical payload.
    let lp_calls = libp2p_mock.recorded_calls();
    let iroh_calls = iroh_mock.recorded_calls();

    assert_eq!(
        lp_calls.len(),
        1,
        "libp2p mock must receive exactly one publish"
    );
    assert_eq!(
        iroh_calls.len(),
        1,
        "iroh mock must receive exactly one publish"
    );

    assert_eq!(
        lp_calls[0].0, INVENTORY_TOPIC,
        "libp2p topic must be INVENTORY_TOPIC"
    );
    assert_eq!(
        iroh_calls[0].0, INVENTORY_TOPIC,
        "iroh topic must be INVENTORY_TOPIC"
    );

    assert_eq!(
        lp_calls[0].1, bytes,
        "libp2p payload must be byte-identical to original"
    );
    assert_eq!(
        iroh_calls[0].1, bytes,
        "iroh payload must be byte-identical to original"
    );

    assert_eq!(
        lp_calls[0].1, iroh_calls[0].1,
        "both transports must receive byte-identical payloads"
    );

    // Verify the received bytes decode back to the original struct.
    let decoded = BlobInventorySnapshot::from_bytes(&lp_calls[0].1)
        .expect("payload must decode to BlobInventorySnapshot");
    assert_eq!(decoded.peer_id, snapshot.peer_id);
    assert_eq!(decoded.hashes, snapshot.hashes);
    assert_eq!(decoded.sequence, snapshot.sequence);
}

// ---------------------------------------------------------------------------
// Test: With iroh transport absent, libp2p still receives inventory snapshot
// ---------------------------------------------------------------------------

#[test]
fn inventory_snapshot_reaches_libp2p_when_iroh_absent() {
    let libp2p_mock = CapturePublisher::new();

    let publisher = DualGossipPublisher::new(
        Some(Arc::new(libp2p_mock.clone()) as Arc<dyn GossipPublisher>),
        None, // iroh not configured
    );

    let snapshot = BlobInventorySnapshot {
        peer_id: "test-peer".to_string(),
        hashes: vec!["0".repeat(64)],
        sequence: 1,
        snapshot_at: 1_000_000,
        signature: vec![],
    };

    let bytes = snapshot.to_bytes().expect("encode");
    publisher
        .publish(INVENTORY_TOPIC, bytes.clone())
        .expect("publish");

    let calls = libp2p_mock.recorded_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].1, bytes);
}
