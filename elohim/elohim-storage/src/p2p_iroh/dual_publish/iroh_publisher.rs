//! Iroh-gossip implementation of [`GossipPublisher`].
//!
//! [`IrohGossipPublisher`] wraps an [`IrohGossip`] handle and implements the
//! existing sync [`GossipPublisher`] trait by forwarding publishes to a
//! background tokio task that owns the per-topic [`GossipSender`]s. The
//! background task subscribes to a topic on first publish (subscribe-on-demand)
//! so the sync trait contract is preserved without blocking the caller.
//!
//! Wire payloads are forwarded byte-identical — no encoding changes. The
//! iroh transport uses the same MessagePack bytes that libp2p-gossipsub uses;
//! topic-id derivation is `BLAKE3(topic_name)[..32]` via
//! [`IrohGossip::topic_id_for`].

use std::collections::HashMap;

use iroh_gossip::api::GossipSender;
use tokio::sync::mpsc;
use tracing::warn;

use crate::p2p_iroh::IrohGossip;
use crate::services::gossip_flood::{GossipPublisher, PublishError};

// ---------------------------------------------------------------------------
// Internal command type
// ---------------------------------------------------------------------------

pub(super) enum IrohGossipCommand {
    Publish { topic: String, payload: Vec<u8> },
}

// ---------------------------------------------------------------------------
// IrohGossipPublisher
// ---------------------------------------------------------------------------

/// Sync [`GossipPublisher`] backed by an iroh-gossip transport.
///
/// Publishes are forwarded via an mpsc channel to a background task that
/// holds per-topic [`GossipSender`]s. The sender subscribes on first publish
/// (subscribe-on-demand) so caller code remains sync.
///
/// Best-effort semantics match the libp2p side:
/// - Channel full → `PublishError::Backend`
/// - Subscribe failure → logged, message dropped (task continues)
/// - Broadcast failure → logged (best-effort, no retry)
pub struct IrohGossipPublisher {
    tx: mpsc::Sender<IrohGossipCommand>,
}

impl IrohGossipPublisher {
    /// Spawn the background task and return a publisher handle.
    ///
    /// The background task runs until the last sender is dropped.
    pub fn spawn(gossip: IrohGossip) -> Self {
        let (tx, mut rx) = mpsc::channel::<IrohGossipCommand>(256);
        tokio::spawn(async move {
            let mut senders: HashMap<String, GossipSender> = HashMap::new();
            while let Some(cmd) = rx.recv().await {
                let IrohGossipCommand::Publish { topic, payload } = cmd;
                // Subscribe-on-demand: first publish on this topic creates the
                // GossipSender; subsequent publishes reuse it.
                let sender = if let Some(s) = senders.get(&topic) {
                    s.clone()
                } else {
                    match gossip.subscribe(&topic, vec![]).await {
                        Ok((s, _r)) => {
                            senders.insert(topic.clone(), s.clone());
                            s
                        }
                        Err(e) => {
                            warn!(
                                target: "elohim_storage::iroh_gossip",
                                topic = %topic,
                                error = ?e,
                                "IrohGossipPublisher: subscribe-on-demand failed; dropping message"
                            );
                            continue;
                        }
                    }
                };
                if let Err(e) = sender.broadcast(bytes::Bytes::from(payload)).await {
                    warn!(
                        target: "elohim_storage::iroh_gossip",
                        topic = %topic,
                        error = ?e,
                        "IrohGossipPublisher: broadcast failed (best-effort)"
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
            .try_send(IrohGossipCommand::Publish {
                topic: topic.to_string(),
                payload,
            })
            .map_err(|e| {
                PublishError::Backend(format!("iroh gossip command channel: {e}"))
            })
    }
}
