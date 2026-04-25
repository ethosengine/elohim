//! Detection stub — subscribes to ReconcileController events and logs.
//!
//! M5 emits ZERO attestations. M6+ replaces this with real detection logic.

use tokio::sync::broadcast;
use tracing::debug;

#[derive(Debug, Clone)]
pub enum ObservedEvent {
    KeyRotation(serde_json::Value),
    KeyRevocation(serde_json::Value),
    AgentPeerBinding(serde_json::Value),
    RevocationAttestation(serde_json::Value),
    PortalHostCreated(serde_json::Value),
    PortalHostRemoved(serde_json::Value),
}

pub async fn run_detection_loop(mut events: broadcast::Receiver<ObservedEvent>) {
    while let Ok(event) = events.recv().await {
        // STUB: M5 logs only.
        debug!("defender observed: {:?}", event);
    }
}
