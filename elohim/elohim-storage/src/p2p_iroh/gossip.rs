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
//! Both planes carry byte-identical payloads. iroh-gossip has no origin
//! authentication (`delivered_from` is the neighbour that relayed a message), so a
//! payload that makes a claim about its author must authenticate it end to end:
//! the inventory envelope ([`crate::p2p::inventory_gossip::InventoryEnvelope`]) is
//! signed by the holder key its `author_peer` embeds and is admitted on iroh under
//! [`crate::p2p::inventory_gossip::AuthorWitness::EndToEnd`]; an unsigned custody
//! announcement carries no claim on this plane.
//!
//! ## Publishing
//!
//! [`IrohGossipPublisher`] sends on this plane; [`FanoutGossipPublisher`] sends one
//! payload on every configured plane (the former per-topic dual-publish table had
//! every topic on both planes, so it is a plain fan-out).

use std::collections::HashMap;
use std::sync::Arc;

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

use crate::services::gossip_flood::{GossipPublisher, PublishError};

/// Publishes on the iroh plane through a background task that subscribes to a topic
/// on first use and keeps its sender.
pub struct IrohGossipPublisher {
    tx: tokio::sync::mpsc::Sender<(String, Vec<u8>)>,
}

impl IrohGossipPublisher {
    /// Spawn the publishing task. bounded-work: a 256-message command channel; a
    /// full channel refuses the publish rather than queueing without bound.
    pub fn spawn(gossip: IrohGossip) -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<(String, Vec<u8>)>(256);
        tokio::spawn(async move {
            let mut senders: HashMap<String, GossipSender> = HashMap::new();
            while let Some((topic, payload)) = rx.recv().await {
                let sender = match senders.get(&topic) {
                    Some(sender) => sender.clone(),
                    None => match gossip.subscribe(&topic, vec![]).await {
                        Ok((sender, _receiver)) => {
                            senders.insert(topic.clone(), sender.clone());
                            sender
                        }
                        Err(error) => {
                            tracing::warn!(
                                target: "elohim_storage::iroh_gossip",
                                %topic, ?error,
                                "iroh publish: subscribe-on-demand failed; message dropped"
                            );
                            continue;
                        }
                    },
                };
                if let Err(error) = sender.broadcast(bytes::Bytes::from(payload)).await {
                    tracing::warn!(
                        target: "elohim_storage::iroh_gossip",
                        %topic, ?error,
                        "iroh publish: broadcast failed (best-effort)"
                    );
                }
            }
        });
        Self { tx }
    }
}

impl GossipPublisher for IrohGossipPublisher {
    fn publish(&self, topic: &str, payload: Vec<u8>) -> Result<(), PublishError> {
        self.tx
            .try_send((topic.to_string(), payload))
            .map_err(|e| PublishError::Backend(format!("iroh gossip command channel: {e}")))
    }
}

/// Sends one payload on every configured plane. Succeeds if any plane accepted it;
/// with no plane configured it is a no-op.
pub struct FanoutGossipPublisher {
    libp2p: Option<Arc<dyn GossipPublisher>>,
    iroh: Option<Arc<dyn GossipPublisher>>,
}

impl FanoutGossipPublisher {
    pub fn new(
        libp2p: Option<Arc<dyn GossipPublisher>>,
        iroh: Option<Arc<dyn GossipPublisher>>,
    ) -> Self {
        Self { libp2p, iroh }
    }
}

impl GossipPublisher for FanoutGossipPublisher {
    fn publish(&self, topic: &str, payload: Vec<u8>) -> Result<(), PublishError> {
        let mut accepted = false;
        let mut last_error = None;
        for plane in [&self.libp2p, &self.iroh].into_iter().flatten() {
            match plane.publish(topic, payload.clone()) {
                Ok(()) => accepted = true,
                Err(error) => last_error = Some(error),
            }
        }
        match (accepted, last_error) {
            (false, Some(error)) => Err(error),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Recorder {
        calls: Mutex<Vec<(String, Vec<u8>)>>,
        fail: bool,
    }

    impl GossipPublisher for Recorder {
        fn publish(&self, topic: &str, payload: Vec<u8>) -> Result<(), PublishError> {
            if self.fail {
                return Err(PublishError::Backend("down".into()));
            }
            self.calls
                .lock()
                .unwrap()
                .push((topic.to_string(), payload));
            Ok(())
        }
    }

    #[test]
    fn fanout_sends_identical_bytes_on_every_plane() {
        let a = Arc::new(Recorder::default());
        let b = Arc::new(Recorder::default());
        let fanout = FanoutGossipPublisher::new(Some(a.clone()), Some(b.clone()));
        fanout
            .publish(crate::p2p::inventory_gossip::INVENTORY_TOPIC, vec![1, 2, 3])
            .unwrap();
        assert_eq!(*a.calls.lock().unwrap(), *b.calls.lock().unwrap());
        assert_eq!(a.calls.lock().unwrap().len(), 1);
    }

    #[test]
    fn fanout_succeeds_when_one_plane_does() {
        let down = Arc::new(Recorder {
            fail: true,
            ..Default::default()
        });
        let up = Arc::new(Recorder::default());
        let one = FanoutGossipPublisher::new(Some(down.clone()), Some(up));
        assert!(one.publish("t", vec![1]).is_ok());
        let none_up = FanoutGossipPublisher::new(Some(down), None);
        assert!(none_up.publish("t", vec![1]).is_err());
        assert!(FanoutGossipPublisher::new(None, None)
            .publish("t", vec![1])
            .is_ok());
    }

    #[test]
    fn topic_id_is_deterministic() {
        let a = IrohGossip::topic_id_for("elohim/inventory/blob/2");
        let b = IrohGossip::topic_id_for("elohim/inventory/blob/2");
        assert_eq!(a, b);
    }

    #[test]
    fn topic_id_distinguishes_names() {
        let inv = IrohGossip::topic_id_for("elohim/inventory/blob/2");
        let identity = IrohGossip::topic_id_for("elohim/identity/binding");
        assert_ne!(inv, identity);
    }

    #[test]
    fn topic_id_matches_blake3_prefix() {
        let name = "elohim/inventory/blob/2";
        let expected: [u8; 32] = *blake3::hash(name.as_bytes()).as_bytes();
        let topic = IrohGossip::topic_id_for(name);
        // TopicId in iroh-gossip 0.92 wraps a 32-byte array; AsRef<[u8]>
        // gives us the canonical comparison surface without committing to
        // a specific From/Into impl that may shift across versions.
        assert_eq!(topic.as_bytes(), &expected);
    }
}
