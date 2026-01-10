//! Real-time WebSocket feed for Shefa compute resources dashboard
//!
//! ## Protocol
//!
//! Connect: `ws://localhost:8080/admin/ws`
//!
//! Messages (server → client):
//! - `node_update` - Node status changed
//! - `cluster_update` - Cluster metrics changed
//! - `heartbeat` - Periodic heartbeat with current state
//!
//! Messages (client → server):
//! - `subscribe` - Subscribe to updates (default: all)
//! - `unsubscribe` - Unsubscribe from updates
//! - `ping` - Keep-alive ping
//!
//! ## Example Messages
//!
//! ```json
//! // Server sends node update
//! {
//!   "type": "node_update",
//!   "timestamp": "2024-01-15T10:30:00Z",
//!   "node_id": "node-123",
//!   "status": "online",
//!   "combined_score": 0.85,
//!   "changes": ["status", "combined_score"]
//! }
//!
//! // Server sends cluster update
//! {
//!   "type": "cluster_update",
//!   "timestamp": "2024-01-15T10:30:00Z",
//!   "online_nodes": 5,
//!   "total_nodes": 6,
//!   "health_ratio": 0.83
//! }
//!
//! // Server sends periodic heartbeat
//! {
//!   "type": "heartbeat",
//!   "timestamp": "2024-01-15T10:30:00Z",
//!   "interval_secs": 30
//! }
//! ```

use futures_util::{SinkExt, StreamExt};
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Request, Response, StatusCode};
use hyper::body::Incoming;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::interval;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{debug, error, info, warn};

use crate::orchestrator::{NodeHealthStatus, OrchestratorState};
use crate::server::AppState;

/// WebSocket type after upgrade
type HyperWebSocket = hyper_tungstenite::WebSocketStream<hyper_util::rt::TokioIo<hyper::upgrade::Upgraded>>;

// ============================================================================
// Message Types
// ============================================================================

/// Message sent from server to client
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DashboardMessage {
    /// Node status update
    NodeUpdate {
        timestamp: String,
        node_id: String,
        status: String,
        combined_score: f64,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        changes: Vec<String>,
    },
    /// Cluster-wide update
    ClusterUpdate {
        timestamp: String,
        online_nodes: usize,
        total_nodes: usize,
        health_ratio: f64,
        avg_trust_score: f64,
        avg_impact_score: f64,
    },
    /// Periodic heartbeat
    Heartbeat {
        timestamp: String,
        interval_secs: u64,
    },
    /// Initial state dump after connection
    InitialState {
        timestamp: String,
        nodes: Vec<NodeSnapshot>,
        cluster: ClusterSnapshot,
    },
    /// Error message
    Error {
        message: String,
    },
}

/// Snapshot of node state
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeSnapshot {
    pub node_id: String,
    pub status: String,
    pub combined_score: f64,
    pub steward_tier: Option<String>,
    pub trust_score: Option<f64>,
    pub last_heartbeat_secs_ago: Option<u64>,
}

/// Snapshot of cluster state
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClusterSnapshot {
    pub online_nodes: usize,
    pub total_nodes: usize,
    pub health_ratio: f64,
    pub avg_trust_score: f64,
    pub avg_impact_score: f64,
}

/// Message received from client
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Subscribe to updates
    Subscribe {
        #[serde(default)]
        topics: Vec<String>,
    },
    /// Unsubscribe from updates
    Unsubscribe {
        #[serde(default)]
        topics: Vec<String>,
    },
    /// Keep-alive ping
    Ping,
}

// ============================================================================
// Dashboard Hub
// ============================================================================

/// Hub for broadcasting dashboard updates to connected clients
pub struct DashboardHub {
    sender: broadcast::Sender<DashboardMessage>,
    orchestrator: Option<Arc<OrchestratorState>>,
    heartbeat_interval_secs: u64,
}

impl DashboardHub {
    /// Create a new dashboard hub
    pub fn new(orchestrator: Option<Arc<OrchestratorState>>) -> Self {
        let (sender, _) = broadcast::channel(100);
        Self {
            sender,
            orchestrator,
            heartbeat_interval_secs: 30,
        }
    }

    /// Subscribe to dashboard updates
    pub fn subscribe(&self) -> broadcast::Receiver<DashboardMessage> {
        self.sender.subscribe()
    }

    /// Broadcast a message to all connected clients
    pub fn broadcast(&self, msg: DashboardMessage) {
        // Ignore send errors (no subscribers)
        let _ = self.sender.send(msg);
    }

    /// Get current cluster snapshot
    pub async fn get_cluster_snapshot(&self) -> Option<ClusterSnapshot> {
        let orchestrator = self.orchestrator.as_ref()?;
        let nodes = orchestrator.all_nodes().await;

        let online = nodes.iter().filter(|n| n.status == NodeHealthStatus::Online).count();
        let total = nodes.len();
        let health_ratio = if total > 0 { online as f64 / total as f64 } else { 0.0 };

        let (trust_sum, impact_sum, count) = nodes.iter()
            .filter_map(|n| n.social_metrics.as_ref())
            .fold((0.0, 0.0, 0usize), |(t, i, c), m| {
                (t + m.trust_score, i + m.impact_score(), c + 1)
            });

        let avg_trust = if count > 0 { trust_sum / count as f64 } else { 0.0 };
        let avg_impact = if count > 0 { impact_sum / count as f64 } else { 0.0 };

        Some(ClusterSnapshot {
            online_nodes: online,
            total_nodes: total,
            health_ratio,
            avg_trust_score: avg_trust,
            avg_impact_score: avg_impact,
        })
    }

    /// Get all nodes snapshot
    pub async fn get_nodes_snapshot(&self) -> Vec<NodeSnapshot> {
        let Some(orchestrator) = &self.orchestrator else {
            return Vec::new();
        };

        let nodes = orchestrator.all_nodes().await;
        nodes.iter().map(|n| {
            let (steward_tier, trust_score) = match &n.social_metrics {
                Some(m) => (Some(m.steward_tier.clone()), Some(m.trust_score)),
                None => (None, None),
            };
            NodeSnapshot {
                node_id: n.node_id.clone(),
                status: status_to_string(&n.status),
                combined_score: n.combined_score,
                steward_tier,
                trust_score,
                last_heartbeat_secs_ago: n.last_heartbeat.map(|t| t.elapsed().as_secs()),
            }
        }).collect()
    }

    /// Broadcast node update
    pub fn broadcast_node_update(
        &self,
        node_id: &str,
        status: &NodeHealthStatus,
        combined_score: f64,
        changes: Vec<String>,
    ) {
        self.broadcast(DashboardMessage::NodeUpdate {
            timestamp: now_iso(),
            node_id: node_id.to_string(),
            status: status_to_string(status),
            combined_score,
            changes,
        });
    }

    /// Broadcast cluster update
    pub async fn broadcast_cluster_update(&self) {
        if let Some(snapshot) = self.get_cluster_snapshot().await {
            self.broadcast(DashboardMessage::ClusterUpdate {
                timestamp: now_iso(),
                online_nodes: snapshot.online_nodes,
                total_nodes: snapshot.total_nodes,
                health_ratio: snapshot.health_ratio,
                avg_trust_score: snapshot.avg_trust_score,
                avg_impact_score: snapshot.avg_impact_score,
            });
        }
    }

    /// Start periodic heartbeat task
    pub fn start_heartbeat_task(self: Arc<Self>) {
        let interval_secs = self.heartbeat_interval_secs;
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(interval_secs));
            loop {
                ticker.tick().await;
                self.broadcast(DashboardMessage::Heartbeat {
                    timestamp: now_iso(),
                    interval_secs,
                });
            }
        });
    }
}

// ============================================================================
// WebSocket Handler
// ============================================================================

/// Handle WebSocket upgrade for dashboard feed
pub async fn handle_dashboard_ws(
    state: Arc<AppState>,
    req: Request<Incoming>,
) -> Response<Full<Bytes>> {
    // Check if orchestrator is enabled
    if state.orchestrator.is_none() {
        return Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(
                r#"{"error": "Orchestrator not enabled"}"#,
            )))
            .unwrap();
    }

    // Check if this is a WebSocket upgrade request
    if !hyper_tungstenite::is_upgrade_request(&req) {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(
                r#"{"error": "WebSocket upgrade required"}"#,
            )))
            .unwrap();
    }

    // Perform the upgrade
    let (response, websocket) = match hyper_tungstenite::upgrade(req, None) {
        Ok((resp, ws)) => (resp, ws),
        Err(e) => {
            error!("WebSocket upgrade failed: {}", e);
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from("WebSocket upgrade failed")))
                .unwrap();
        }
    };

    // Spawn task to handle the WebSocket connection
    let orchestrator = state.orchestrator.clone();
    tokio::spawn(async move {
        match websocket.await {
            Ok(ws) => {
                let ws: HyperWebSocket = ws;
                if let Err(e) = handle_dashboard_connection(ws, orchestrator).await {
                    warn!("Dashboard WebSocket error: {}", e);
                }
            }
            Err(e) => {
                error!("WebSocket connection failed: {}", e);
            }
        }
    });

    // Return the upgrade response
    // Convert the response body type
    let (parts, _body) = response.into_parts();
    Response::from_parts(parts, Full::new(Bytes::new()))
}

/// Handle an individual dashboard WebSocket connection
async fn handle_dashboard_connection(
    ws: HyperWebSocket,
    orchestrator: Option<Arc<OrchestratorState>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (mut sender, mut receiver) = ws.split();

    info!("Dashboard WebSocket client connected");

    // Create a hub for this connection
    let hub = Arc::new(DashboardHub::new(orchestrator));

    // Send initial state
    let nodes = hub.get_nodes_snapshot().await;
    let cluster = hub.get_cluster_snapshot().await.unwrap_or(ClusterSnapshot {
        online_nodes: 0,
        total_nodes: 0,
        health_ratio: 0.0,
        avg_trust_score: 0.0,
        avg_impact_score: 0.0,
    });

    let initial_msg = DashboardMessage::InitialState {
        timestamp: now_iso(),
        nodes,
        cluster,
    };

    let json = serde_json::to_string(&initial_msg)?;
    sender.send(WsMessage::Text(json)).await?;

    // Subscribe to broadcasts
    let mut rx = hub.subscribe();

    // Handle messages
    loop {
        tokio::select! {
            // Broadcast message from hub
            msg = rx.recv() => {
                match msg {
                    Ok(dashboard_msg) => {
                        let json = serde_json::to_string(&dashboard_msg)?;
                        if sender.send(WsMessage::Text(json)).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                }
            }

            // Message from client
            msg = receiver.next() => {
                match msg {
                    Some(Ok(WsMessage::Text(text))) => {
                        debug!("Received from dashboard client: {}", text);
                        // Parse and handle client message
                        if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                            match client_msg {
                                ClientMessage::Ping => {
                                    let pong = serde_json::json!({"type": "pong", "timestamp": now_iso()});
                                    let _ = sender.send(WsMessage::Text(pong.to_string())).await;
                                }
                                ClientMessage::Subscribe { topics } => {
                                    debug!("Client subscribing to topics: {:?}", topics);
                                    // Topics not implemented yet, but acknowledged
                                }
                                ClientMessage::Unsubscribe { topics } => {
                                    debug!("Client unsubscribing from topics: {:?}", topics);
                                }
                            }
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) => {
                        info!("Dashboard WebSocket client disconnected");
                        break;
                    }
                    Some(Ok(WsMessage::Ping(data))) => {
                        let _ = sender.send(WsMessage::Pong(data)).await;
                    }
                    Some(Err(e)) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }
        }
    }

    info!("Dashboard WebSocket connection closed");
    Ok(())
}

// ============================================================================
// Helpers
// ============================================================================

fn status_to_string(status: &NodeHealthStatus) -> String {
    match status {
        NodeHealthStatus::Discovered => "discovered".to_string(),
        NodeHealthStatus::Registering => "registering".to_string(),
        NodeHealthStatus::Online => "online".to_string(),
        NodeHealthStatus::Degraded => "degraded".to_string(),
        NodeHealthStatus::Offline => "offline".to_string(),
        NodeHealthStatus::Failed => "failed".to_string(),
    }
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_snapshot_serialization() {
        let snapshot = NodeSnapshot {
            node_id: "test-node".to_string(),
            status: "online".to_string(),
            combined_score: 0.85,
            steward_tier: Some("guardian".to_string()),
            trust_score: Some(0.9),
            last_heartbeat_secs_ago: Some(5),
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("test-node"));
        assert!(json.contains("guardian"));
    }

    #[test]
    fn test_dashboard_message_serialization() {
        let msg = DashboardMessage::NodeUpdate {
            timestamp: "2024-01-15T10:30:00Z".to_string(),
            node_id: "node-1".to_string(),
            status: "online".to_string(),
            combined_score: 0.85,
            changes: vec!["status".to_string()],
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"node_update\""));
        assert!(json.contains("node-1"));
    }

    #[test]
    fn test_cluster_snapshot_serialization() {
        let snapshot = ClusterSnapshot {
            online_nodes: 5,
            total_nodes: 6,
            health_ratio: 0.833,
            avg_trust_score: 0.85,
            avg_impact_score: 0.72,
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("\"onlineNodes\":5"));
        assert!(json.contains("\"healthRatio\":0.833"));
    }
}
