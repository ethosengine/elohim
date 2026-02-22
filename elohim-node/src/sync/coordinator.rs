//! Sync coordinator — orchestrates sync across peers using the engine + swarm.
//!
//! Handles incoming sync/doc requests, periodic outbound sync with known peers,
//! and applies replication policy based on reach levels.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info, warn};

use crate::p2p::transport::SwarmEvent;
use crate::storage::reach::Reach;
use crate::sync::merge::SyncEngine;
use crate::sync::protocol::SyncMessage;

/// Tracks sync state for a known peer.
struct PeerSyncState {
    #[allow(dead_code)]
    peer_id: String,
    last_position: u64,
    #[allow(dead_code)]
    trust_level: Reach,
    last_sync_time: u64,
}

/// Commands sent from the coordinator to the swarm task.
pub enum SwarmCommand {
    /// Send a sync request to a peer.
    SendRequest {
        #[allow(dead_code)]
        peer_id: libp2p::PeerId,
        #[allow(dead_code)]
        request: SyncMessage,
    },
    /// Send a response on a channel.
    SendResponse {
        #[allow(dead_code)]
        channel: libp2p::request_response::ResponseChannel<SyncMessage>,
        #[allow(dead_code)]
        response: SyncMessage,
    },
}

/// Orchestrates sync across peers.
pub struct SyncCoordinator {
    engine: Arc<Mutex<SyncEngine>>,
    peers: HashMap<String, PeerSyncState>,
    sync_interval: Duration,
}

impl SyncCoordinator {
    pub fn new(engine: Arc<Mutex<SyncEngine>>, sync_interval_ms: u64) -> Self {
        Self {
            engine,
            peers: HashMap::new(),
            sync_interval: Duration::from_millis(sync_interval_ms),
        }
    }

    /// Run the coordinator event loop.
    ///
    /// Listens for swarm events and periodically initiates sync with peers.
    pub async fn run(
        mut self,
        mut swarm_events: mpsc::Receiver<SwarmEvent>,
        swarm_commands: mpsc::Sender<SwarmCommand>,
    ) {
        let mut sync_timer = tokio::time::interval(self.sync_interval);

        loop {
            tokio::select! {
                Some(event) = swarm_events.recv() => {
                    self.handle_swarm_event(event, &swarm_commands).await;
                }
                _ = sync_timer.tick() => {
                    self.periodic_sync(&swarm_commands).await;
                }
            }
        }
    }

    async fn handle_swarm_event(
        &mut self,
        event: SwarmEvent,
        swarm_commands: &mpsc::Sender<SwarmCommand>,
    ) {
        match event {
            SwarmEvent::PeerDiscovered { peer_id, .. } => {
                let peer_str = peer_id.to_string();
                if !self.peers.contains_key(&peer_str) {
                    info!(%peer_id, "New peer discovered, adding to sync list");
                    self.peers.insert(
                        peer_str.clone(),
                        PeerSyncState {
                            peer_id: peer_str,
                            last_position: 0,
                            trust_level: Reach::Local, // Default for mDNS-discovered peers
                            last_sync_time: 0,
                        },
                    );
                }
            }

            SwarmEvent::PeerExpired { peer_id } => {
                let peer_str = peer_id.to_string();
                if self.peers.remove(&peer_str).is_some() {
                    info!(%peer_id, "Peer expired, removed from sync list");
                }
            }

            SwarmEvent::IncomingRequest {
                peer_id,
                request,
                channel,
            } => {
                let response = self.handle_request(&peer_id.to_string(), request).await;
                let _ = swarm_commands
                    .send(SwarmCommand::SendResponse { channel, response })
                    .await;
            }

            SwarmEvent::ResponseReceived {
                peer_id, response, ..
            } => {
                self.handle_response(&peer_id.to_string(), response).await;
            }

            SwarmEvent::OutboundFailure { peer_id, error, .. } => {
                warn!(%peer_id, %error, "Sync request failed");
            }
        }
    }

    /// Handle an incoming sync request and produce a response.
    async fn handle_request(&self, peer_id: &str, request: SyncMessage) -> SyncMessage {
        match request {
            SyncMessage::SyncRequest { since, limit } => {
                let engine = self.engine.lock().await;
                let mut events = engine.events_since(since);
                let has_more = if let Some(max) = limit {
                    let max = max as usize;
                    if events.len() > max {
                        events.truncate(max);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                debug!(
                    peer_id,
                    since,
                    num_events = events.len(),
                    "Responding to SyncRequest"
                );
                SyncMessage::SyncResponse { events, has_more }
            }

            SyncMessage::DocRequest { doc_id, heads } => {
                let engine = self.engine.lock().await;
                match engine.get_changes_for_peer(&doc_id, &heads) {
                    Ok(changes) => {
                        debug!(
                            peer_id,
                            doc_id,
                            num_changes = changes.len(),
                            "Responding to DocRequest"
                        );
                        SyncMessage::DocResponse { doc_id, changes }
                    }
                    Err(e) => {
                        error!(peer_id, doc_id, error = %e, "Failed to get changes for peer");
                        SyncMessage::DocResponse {
                            doc_id,
                            changes: vec![],
                        }
                    }
                }
            }

            // If we get a response-type message as a request, just echo empty
            other => {
                warn!(peer_id, "Unexpected message type as request: {:?}", other);
                SyncMessage::SyncResponse {
                    events: vec![],
                    has_more: false,
                }
            }
        }
    }

    /// Handle a sync response from a peer.
    async fn handle_response(&mut self, peer_id: &str, response: SyncMessage) {
        match response {
            SyncMessage::SyncResponse { events, has_more } => {
                debug!(
                    peer_id,
                    num_events = events.len(),
                    has_more,
                    "Processing SyncResponse"
                );

                // Track max position from peer events
                let max_position = events.iter().map(|e| e.position).max().unwrap_or(0);

                // Apply changes for each doc referenced
                let engine = self.engine.lock().await;
                for event in &events {
                    // For each event, we'd normally fetch the doc changes.
                    // In the stream model, events are metadata; actual doc content
                    // comes via DocRequest/DocResponse.
                    debug!(
                        peer_id,
                        doc_id = %event.doc_id,
                        position = event.position,
                        "Noted remote event"
                    );
                }
                drop(engine);

                // Update peer position
                if let Some(peer) = self.peers.get_mut(peer_id) {
                    if max_position > peer.last_position {
                        peer.last_position = max_position;
                    }
                    peer.last_sync_time = now();
                }
            }

            SyncMessage::DocResponse { doc_id, changes } => {
                if !changes.is_empty() {
                    let mut engine = self.engine.lock().await;
                    if let Err(e) = engine.apply_remote_changes(&doc_id, &changes) {
                        error!(peer_id, doc_id, error = %e, "Failed to apply remote changes");
                    } else {
                        debug!(peer_id, doc_id, "Applied remote doc changes");
                    }
                }
            }

            other => {
                warn!(peer_id, "Unexpected response type: {:?}", other);
            }
        }
    }

    /// Periodic sync: send SyncRequest to all known peers.
    async fn periodic_sync(&self, swarm_commands: &mpsc::Sender<SwarmCommand>) {
        for (peer_str, peer_state) in &self.peers {
            if let Ok(peer_id) = peer_str.parse::<libp2p::PeerId>() {
                let request = SyncMessage::SyncRequest {
                    since: peer_state.last_position,
                    limit: Some(100),
                };
                let _ = swarm_commands
                    .send(SwarmCommand::SendRequest { peer_id, request })
                    .await;
            }
        }
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
