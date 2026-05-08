//! Iroh-gossip wrapper — Phase 4 of the parallel iroh stack.
//!
//! Replaces the libp2p gossipsub plane (inventory, identity-binding,
//! integrity-revocation, recovery-invitation, recovery-revocation,
//! attention, feedback) on the iroh transport.
//!
//! ## Topic mapping
//!
//! libp2p gossipsub uses string topic names (`"elohim/inventory/blob"`).
//! iroh-gossip uses 32-byte [`TopicId`]s. We derive the iroh `TopicId` as
//! `BLAKE3(topic_name)[..32]` so the mapping is deterministic and any
//! peer can independently compute the iroh topic for a known name.
//!
//! Wire payloads remain MessagePack (rmp_serde) for parity with the
//! libp2p side — see e.g. [`crate::p2p::inventory_gossip::BlobInventorySnapshot`].
//! Cutover does not change message schemas, only the transport.

use anyhow::Result;
use iroh::{Endpoint, NodeId};
use iroh_gossip::{
    api::{GossipReceiver, GossipSender},
    net::Gossip,
    proto::TopicId,
};

/// Wrapper around [`Gossip`] that exposes the elohim topic-naming
/// convention (string → BLAKE3 → `TopicId`) and the subscribe / broadcast
/// API used by the protocol layer.
#[derive(Clone, Debug)]
pub struct IrohGossip {
    inner: Gossip,
}

impl IrohGossip {
    /// Spawn the gossip subsystem against an existing [`Endpoint`].
    /// The returned [`Gossip`] handle is registered on the Router by
    /// [`crate::p2p_iroh::IrohNode::start_with_protocols`] under
    /// [`iroh_gossip::ALPN`].
    pub fn new(endpoint: Endpoint) -> Self {
        let inner = Gossip::builder().spawn(endpoint);
        Self { inner }
    }

    /// Borrow the inner [`Gossip`] handle (for Router registration or
    /// out-of-band ops). Most callers should use [`IrohGossip::subscribe`]
    /// or [`IrohGossip::broadcast`] instead.
    pub fn inner(&self) -> &Gossip {
        &self.inner
    }

    /// Map a string topic name to its iroh [`TopicId`] via
    /// `BLAKE3(name)[..32]`. Deterministic — any peer with the same name
    /// computes the same id.
    pub fn topic_id_for(name: &str) -> TopicId {
        let hash = blake3::hash(name.as_bytes());
        TopicId::from(*hash.as_bytes())
    }

    /// Subscribe to a topic by string name. Returns a (sender, receiver)
    /// pair. `bootstrap` is the list of known peers to dial; pass an
    /// empty vec for the initial subscriber.
    pub async fn subscribe(
        &self,
        name: &str,
        bootstrap: Vec<NodeId>,
    ) -> Result<(GossipSender, GossipReceiver)> {
        let topic = Self::topic_id_for(name);
        let handle = self.inner.subscribe(topic, bootstrap).await?;
        Ok(handle.split())
    }

    /// Subscribe and wait until at least one peer is connected on this
    /// topic. Useful in tests; production code should listen for
    /// [`Event::NeighborUp`] explicitly.
    pub async fn subscribe_and_join(
        &self,
        name: &str,
        bootstrap: Vec<NodeId>,
    ) -> Result<(GossipSender, GossipReceiver)> {
        let topic = Self::topic_id_for(name);
        let mut handle = self.inner.subscribe(topic, bootstrap).await?;
        handle.joined().await?;
        Ok(handle.split())
    }
}

/// Re-export so callers don't need to depend on iroh-gossip directly.
pub use iroh_gossip::api::Event as GossipEvent;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_id_is_deterministic() {
        let a = IrohGossip::topic_id_for("elohim/inventory/blob");
        let b = IrohGossip::topic_id_for("elohim/inventory/blob");
        assert_eq!(a, b);
    }

    #[test]
    fn topic_id_distinguishes_names() {
        let inv = IrohGossip::topic_id_for("elohim/inventory/blob");
        let identity = IrohGossip::topic_id_for("elohim/identity/binding");
        assert_ne!(inv, identity);
    }

    #[test]
    fn topic_id_matches_blake3_prefix() {
        let name = "elohim/inventory/blob";
        let expected: [u8; 32] = *blake3::hash(name.as_bytes()).as_bytes();
        let topic = IrohGossip::topic_id_for(name);
        // TopicId in iroh-gossip 0.92 wraps a 32-byte array; AsRef<[u8]>
        // gives us the canonical comparison surface without committing to
        // a specific From/Into impl that may shift across versions.
        assert_eq!(topic.as_bytes(), &expected);
    }
}
