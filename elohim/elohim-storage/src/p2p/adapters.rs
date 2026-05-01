//! Production swarm-backed adapters for back_prop and gossip_flood services.
//!
//! Bridges the existing trait abstractions ([`OutboundSink`], [`GossipPublisher`])
//! to the swarm task's [`P2PCommand`] mpsc channel — actor pattern, no swarm
//! locking required.
//!
//! ## Adapters
//!
//! - [`LibP2POutboundSink`] — implements [`OutboundSink`]; routes one-hop
//!   back-propagation sends via `P2PCommand::SendDirect`.
//! - [`LibP2PGossipPublisher`] — implements [`GossipPublisher`]; routes gossip-
//!   flood broadcasts via `P2PCommand::GossipPublish`.
//!
//! Both adapters use `try_send` on the mpsc channel so they remain sync (matching
//! the trait signatures) while the swarm event loop runs on a separate tokio task.
//! Backpressure (channel full) is surfaced as a best-effort error that callers log
//! and continue from — consistent with the non-fatal contracts of both traits.

use std::str::FromStr;

use libp2p::PeerId;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::Sender;

use crate::p2p::P2PCommand;
use crate::services::back_prop::{OutboundSink, SinkError};
use crate::services::gossip_flood::{GossipPublisher, PublishError};

// ---------------------------------------------------------------------------
// LibP2POutboundSink
// ---------------------------------------------------------------------------

/// Production [`OutboundSink`] backed by the P2P swarm command channel.
///
/// Encodes the payload into a `SendDirect` command and dispatches it to the
/// swarm event loop via a non-blocking `try_send`. Errors on invalid peer IDs,
/// a full channel (backpressure), or a closed channel (swarm stopped).
#[derive(Clone)]
pub struct LibP2POutboundSink {
    tx: Sender<P2PCommand>,
}

impl LibP2POutboundSink {
    /// Construct from the P2P command channel sender.
    pub fn new(tx: Sender<P2PCommand>) -> Self {
        Self { tx }
    }
}

impl OutboundSink for LibP2POutboundSink {
    fn send(&self, peer_id_str: &str, payload: Vec<u8>) -> Result<(), SinkError> {
        let peer = PeerId::from_str(peer_id_str)
            .map_err(|e| SinkError::Send(format!("invalid peer_id '{}': {}", peer_id_str, e)))?;
        self.tx
            .try_send(P2PCommand::SendDirect { peer, payload })
            .map_err(|e| match e {
                TrySendError::Full(_) => {
                    SinkError::Send("backpressure: P2P command channel full".to_string())
                }
                TrySendError::Closed(_) => {
                    SinkError::Send("swarm gone: P2P command channel closed".to_string())
                }
            })
    }
}

// ---------------------------------------------------------------------------
// LibP2PGossipPublisher
// ---------------------------------------------------------------------------

/// Production [`GossipPublisher`] backed by the P2P swarm command channel.
///
/// Encodes the payload into a `GossipPublish` command and dispatches it to the
/// swarm event loop via a non-blocking `try_send`. Errors on a full channel
/// (backpressure) or closed channel (swarm stopped).
#[derive(Clone)]
pub struct LibP2PGossipPublisher {
    tx: Sender<P2PCommand>,
}

impl LibP2PGossipPublisher {
    /// Construct from the P2P command channel sender.
    pub fn new(tx: Sender<P2PCommand>) -> Self {
        Self { tx }
    }
}

impl GossipPublisher for LibP2PGossipPublisher {
    fn publish(&self, topic: &str, payload: Vec<u8>) -> Result<(), PublishError> {
        self.tx
            .try_send(P2PCommand::GossipPublish {
                topic: topic.to_string(),
                payload,
            })
            .map_err(|e| match e {
                TrySendError::Full(_) => {
                    PublishError::Backend("backpressure: P2P command channel full".to_string())
                }
                TrySendError::Closed(_) => {
                    PublishError::Backend("swarm gone: P2P command channel closed".to_string())
                }
            })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    // -----------------------------------------------------------------------
    // Test 1: sink_send_succeeds_with_open_channel
    //
    // Happy path: valid peer ID, open channel. Command arrives as SendDirect.
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn sink_send_succeeds_with_open_channel() {
        let (tx, mut rx) = mpsc::channel::<P2PCommand>(8);
        let sink = LibP2POutboundSink::new(tx);

        // A well-formed base58-encoded libp2p PeerId.
        let valid_peer = "12D3KooWGRUAcEqzj5N1zMyxYBfZjPDKHbz9V3xS1HmKZRwjQR4q";
        sink.send(valid_peer, vec![1, 2, 3])
            .expect("send should succeed");

        let cmd = rx.recv().await.expect("command should arrive");
        match cmd {
            P2PCommand::SendDirect { peer, payload } => {
                assert_eq!(peer.to_string(), valid_peer);
                assert_eq!(payload, vec![1, 2, 3]);
            }
            _ => panic!("expected P2PCommand::SendDirect but got a different variant"),
        }
    }

    // -----------------------------------------------------------------------
    // Test 2: sink_send_returns_err_on_closed_channel
    //
    // Channel closed (swarm stopped): send must return SinkError::Send with
    // "swarm gone" in the message.
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn sink_send_returns_err_on_closed_channel() {
        let (tx, rx) = mpsc::channel::<P2PCommand>(8);
        drop(rx); // Simulate swarm stopped.
        let sink = LibP2POutboundSink::new(tx);

        let valid_peer = "12D3KooWGRUAcEqzj5N1zMyxYBfZjPDKHbz9V3xS1HmKZRwjQR4q";
        let err = sink.send(valid_peer, vec![1]).unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("swarm gone"),
            "error message must mention 'swarm gone', got: {msg}"
        );
    }

    // -----------------------------------------------------------------------
    // Test 3: sink_send_rejects_malformed_peer_id
    //
    // The string "not-a-peer-id" is not a valid libp2p PeerId. The adapter
    // must return SinkError::Send with "invalid peer_id" in the message before
    // attempting to send anything on the channel.
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn sink_send_rejects_malformed_peer_id() {
        let (tx, _rx) = mpsc::channel::<P2PCommand>(8);
        let sink = LibP2POutboundSink::new(tx);

        let err = sink.send("not-a-peer-id", vec![1]).unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("invalid peer_id"),
            "error message must mention 'invalid peer_id', got: {msg}"
        );
    }

    // -----------------------------------------------------------------------
    // Test 4: publisher_publish_succeeds
    //
    // Happy path: valid topic, open channel. Command arrives as GossipPublish
    // with the correct topic and payload.
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn publisher_publish_succeeds() {
        let (tx, mut rx) = mpsc::channel::<P2PCommand>(8);
        let publisher = LibP2PGossipPublisher::new(tx);

        publisher
            .publish("test/topic/1.0.0", vec![9, 9, 9])
            .expect("publish should succeed");

        let cmd = rx.recv().await.expect("command should arrive");
        match cmd {
            P2PCommand::GossipPublish { topic, payload } => {
                assert_eq!(topic, "test/topic/1.0.0");
                assert_eq!(payload, vec![9, 9, 9]);
            }
            _ => panic!("expected P2PCommand::GossipPublish but got a different variant"),
        }
    }
}
