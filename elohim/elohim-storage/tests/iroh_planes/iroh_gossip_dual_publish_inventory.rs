//! Integration test: a signed inventory delta via `FanoutGossipPublisher`.
//!
//! Verifies that a delta envelope built by `inventory_broadcaster::build_delta_pages`
//! (the only inventory announcement there is: signed, sequenced, byte CIDs) and
//! published through the `FanoutGossipPublisher` reaches both the libp2p mock and
//! the iroh mock byte-identically on `elohim/inventory/blob/2`, and decodes back to
//! the same signed envelope.
//!
//! Synchronous mocks, not a live swarm: the live two-stack path runs in the
//! cross-stack e2e. Gated on `p2p-iroh`.

use std::sync::{Arc, Mutex};

use elohim_epr::address::compute_bytes_cid;
use elohim_epr::proof::AgentKeypair;
use elohim_storage::p2p::inventory_broadcaster::{build_delta_pages, SequenceAllocator};
use elohim_storage::p2p::inventory_gossip::{BlobAddress, InventoryEnvelope, INVENTORY_TOPIC};
use elohim_storage::p2p_iroh::FanoutGossipPublisher;
use elohim_storage::services::gossip_flood::{GossipPublisher, PublishError};

// ---------------------------------------------------------------------------
// Mock publisher (captures (topic, payload) pairs for assertion)
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
struct CapturePublisher {
    #[allow(
        clippy::type_complexity,
        reason = "a captured (topic, payload) log; a named alias adds nothing"
    )]
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
// Test: FanoutGossipPublisher delivers a signed delta byte-identically
// ---------------------------------------------------------------------------

fn signed_delta(added: &[&[u8]], sequence_from: u64) -> InventoryEnvelope {
    let keypair = AgentKeypair::from_secret(&[7u8; 32]).unwrap();
    let alloc = SequenceAllocator::new(sequence_from);
    let added: Vec<BlobAddress> = added
        .iter()
        .map(|bytes| BlobAddress::from(compute_bytes_cid(bytes)))
        .collect();
    let mut pages = build_delta_pages(
        &keypair,
        "12D3KooWTestPeer",
        &added,
        &[],
        &alloc,
        1_700_000_000_000_000,
        &[],
    );
    assert_eq!(pages.len(), 1, "three addresses fit one page");
    pages.remove(0)
}

#[test]
fn fanout_publisher_routes_a_signed_inventory_delta_to_both_transports() {
    let libp2p_mock = CapturePublisher::new();
    let iroh_mock = CapturePublisher::new();

    let publisher = FanoutGossipPublisher::new(
        Some(Arc::new(libp2p_mock.clone()) as Arc<dyn GossipPublisher>),
        Some(Arc::new(iroh_mock.clone()) as Arc<dyn GossipPublisher>),
    );

    let envelope = signed_delta(&[b"blob a", b"blob b", b"blob c"], 41);
    let bytes = envelope.to_bytes();

    publisher
        .publish(INVENTORY_TOPIC, bytes.clone())
        .expect("FanoutGossipPublisher should succeed");

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
    assert_eq!(lp_calls[0].0, INVENTORY_TOPIC);
    assert_eq!(iroh_calls[0].0, INVENTORY_TOPIC);
    assert_eq!(
        lp_calls[0].1, bytes,
        "libp2p payload must be byte-identical"
    );
    assert_eq!(
        iroh_calls[0].1, bytes,
        "iroh payload must be byte-identical"
    );

    // The received bytes decode back to the same signed envelope.
    let decoded = InventoryEnvelope::from_bytes(&lp_calls[0].1)
        .expect("payload must decode to an InventoryEnvelope");
    assert_eq!(decoded, envelope);
    assert_eq!(decoded.author_peer, "12D3KooWTestPeer");
    assert_eq!(decoded.body.sequence, 42);
    assert_eq!(decoded.body.added.len(), 3);
    assert_eq!(
        decoded.body.added[0],
        BlobAddress::from(compute_bytes_cid(b"blob a"))
    );
}

// ---------------------------------------------------------------------------
// Test: With iroh transport absent, libp2p still receives the delta
// ---------------------------------------------------------------------------

#[test]
fn inventory_delta_reaches_libp2p_when_iroh_absent() {
    let libp2p_mock = CapturePublisher::new();

    let publisher = FanoutGossipPublisher::new(
        Some(Arc::new(libp2p_mock.clone()) as Arc<dyn GossipPublisher>),
        None, // iroh not configured
    );

    let bytes = signed_delta(&[b"only blob"], 0).to_bytes();
    publisher
        .publish(INVENTORY_TOPIC, bytes.clone())
        .expect("publish");

    let calls = libp2p_mock.recorded_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].1, bytes);
}
