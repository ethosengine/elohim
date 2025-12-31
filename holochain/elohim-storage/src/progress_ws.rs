//! WebSocket handler for import progress streaming
//!
//! Provides real-time progress updates to connected clients via WebSocket.
//!
//! ## Protocol
//!
//! ### Client → Server
//! ```json
//! {"type": "subscribe", "batch_ids": ["batch-1", "batch-2"]}
//! {"type": "unsubscribe", "batch_ids": ["batch-1"]}
//! {"type": "ping"}
//! ```
//!
//! ### Server → Client
//! ```json
//! {"type": "initial_state", "batches": [...]}
//! {"type": "progress", "batch_id": "...", ...}
//! {"type": "complete", "batch_id": "...", ...}
//! {"type": "heartbeat", "timestamp": "..."}
//! ```

use crate::progress_hub::{ProgressHub, ProgressMessage};
use futures_util::{SinkExt, StreamExt};
use hyper::{Request, Response, StatusCode};
use hyper::body::Incoming;
use http_body_util::Full;
use bytes::Bytes;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;
use tracing::{debug, error, info, warn};

/// WebSocket type after upgrade (using hyper-tungstenite)
type HyperWebSocket = hyper_tungstenite::WebSocketStream<hyper_util::rt::TokioIo<hyper::upgrade::Upgraded>>;

/// Messages from client to server
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Subscribe to progress updates for specific batches
    Subscribe {
        /// Batch IDs to subscribe to (empty = all batches)
        batch_ids: Vec<String>,
    },
    /// Unsubscribe from specific batches
    Unsubscribe {
        batch_ids: Vec<String>,
    },
    /// Ping to keep connection alive
    Ping,
}

/// Check if the request is a WebSocket upgrade request
pub fn is_websocket_upgrade(req: &Request<Incoming>) -> bool {
    // Check for WebSocket upgrade headers
    let connection = req
        .headers()
        .get(hyper::header::CONNECTION)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_lowercase())
        .unwrap_or_default();

    let upgrade = req
        .headers()
        .get(hyper::header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_lowercase())
        .unwrap_or_default();

    connection.contains("upgrade") && upgrade.contains("websocket")
}

/// Handle WebSocket upgrade for progress endpoint
///
/// Uses hyper-tungstenite for proper RFC 6455 handshake with SHA-1
pub async fn handle_progress_upgrade(
    req: Request<Incoming>,
    hub: Arc<ProgressHub>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    // Check if this is a WebSocket upgrade request
    if !hyper_tungstenite::is_upgrade_request(&req) {
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Full::new(Bytes::from("Expected WebSocket upgrade")))
            .unwrap());
    }

    info!("WebSocket upgrade request for /import/progress");

    // Perform the upgrade using hyper-tungstenite (handles SHA-1 correctly)
    let (response, websocket) = match hyper_tungstenite::upgrade(req, None) {
        Ok((resp, ws)) => (resp, ws),
        Err(e) => {
            error!("WebSocket upgrade failed: {}", e);
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from("WebSocket upgrade failed")))
                .unwrap());
        }
    };

    // Spawn task to handle the connection after upgrade completes
    tokio::spawn(async move {
        match websocket.await {
            Ok(ws_stream) => {
                if let Err(e) = handle_connection(ws_stream, hub).await {
                    warn!(error = %e, "WebSocket connection error");
                }
            }
            Err(e) => {
                error!(error = %e, "WebSocket connection failed");
            }
        }
    });

    // Return the upgrade response (hyper-tungstenite handles the correct headers)
    let (parts, _body) = response.into_parts();
    Ok(Response::from_parts(parts, Full::new(Bytes::new())))
}

/// Handle an established WebSocket connection
async fn handle_connection(
    ws_stream: HyperWebSocket,
    hub: Arc<ProgressHub>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (mut ws_sink, mut ws_stream) = ws_stream.split();

    // Set of batch IDs this client is subscribed to (empty = all)
    let mut subscriptions: HashSet<String> = HashSet::new();
    let mut subscribe_all = false;

    // Subscribe to progress hub
    let mut progress_rx = hub.subscribe();

    // Heartbeat interval
    let heartbeat_interval = hub.heartbeat_interval();
    let mut heartbeat_timer = tokio::time::interval(heartbeat_interval);

    info!("WebSocket connection established for /import/progress");

    loop {
        tokio::select! {
            // Handle incoming messages from client
            msg = ws_stream.next() => {
                match msg {
                    Some(Ok(WsMessage::Text(text))) => {
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(ClientMessage::Subscribe { batch_ids }) => {
                                if batch_ids.is_empty() {
                                    subscribe_all = true;
                                    subscriptions.clear();
                                    debug!("Client subscribed to all batches");
                                } else {
                                    subscribe_all = false;
                                    for id in &batch_ids {
                                        subscriptions.insert(id.clone());
                                    }
                                    debug!(batch_ids = ?batch_ids, "Client subscribed to batches");
                                }

                                // Send initial state for subscribed batches
                                let states = if subscribe_all {
                                    hub.get_batch_states().await
                                } else {
                                    let ids: Vec<_> = subscriptions.iter().cloned().collect();
                                    hub.get_batch_states_filtered(&ids).await
                                };

                                let initial = ProgressMessage::InitialState { batches: states };
                                let json = serde_json::to_string(&initial)?;
                                ws_sink.send(WsMessage::Text(json.into())).await?;
                            }
                            Ok(ClientMessage::Unsubscribe { batch_ids }) => {
                                for id in batch_ids {
                                    subscriptions.remove(&id);
                                }
                                if subscriptions.is_empty() && !subscribe_all {
                                    // If no specific subscriptions and not subscribed to all,
                                    // default to subscribing to all
                                    subscribe_all = true;
                                }
                                debug!("Client unsubscribed from batches");
                            }
                            Ok(ClientMessage::Ping) => {
                                // Respond with pong (heartbeat)
                                let heartbeat = ProgressMessage::Heartbeat {
                                    timestamp: chrono::Utc::now().to_rfc3339(),
                                };
                                let json = serde_json::to_string(&heartbeat)?;
                                ws_sink.send(WsMessage::Text(json.into())).await?;
                            }
                            Err(e) => {
                                warn!(error = %e, text = %text, "Failed to parse client message");
                            }
                        }
                    }
                    Some(Ok(WsMessage::Ping(data))) => {
                        ws_sink.send(WsMessage::Pong(data)).await?;
                    }
                    Some(Ok(WsMessage::Pong(_))) => {
                        // Client responded to ping, connection is alive
                    }
                    Some(Ok(WsMessage::Close(_))) => {
                        info!("WebSocket client disconnected");
                        break;
                    }
                    Some(Ok(_)) => {
                        // Ignore other message types
                    }
                    Some(Err(e)) => {
                        warn!(error = %e, "WebSocket receive error");
                        break;
                    }
                    None => {
                        // Stream ended
                        break;
                    }
                }
            }

            // Handle progress updates from hub
            result = progress_rx.recv() => {
                match result {
                    Ok(message) => {
                        // Filter by subscription
                        let should_send = subscribe_all || match &message {
                            ProgressMessage::Progress { batch_id, .. } |
                            ProgressMessage::Complete { batch_id, .. } |
                            ProgressMessage::Error { batch_id, .. } => {
                                subscriptions.contains(batch_id)
                            }
                            ProgressMessage::InitialState { .. } |
                            ProgressMessage::Heartbeat { .. } => true,
                        };

                        if should_send {
                            let json = serde_json::to_string(&message)?;
                            if let Err(e) = ws_sink.send(WsMessage::Text(json.into())).await {
                                warn!(error = %e, "Failed to send progress to client");
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!(skipped = n, "Client lagged behind, skipped messages");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("Progress hub closed");
                        break;
                    }
                }
            }

            // Send periodic heartbeats
            _ = heartbeat_timer.tick() => {
                let heartbeat = ProgressMessage::Heartbeat {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };
                let json = serde_json::to_string(&heartbeat)?;
                if let Err(e) = ws_sink.send(WsMessage::Text(json.into())).await {
                    warn!(error = %e, "Failed to send heartbeat");
                    break;
                }
            }
        }
    }

    info!("WebSocket connection closed for /import/progress");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_parsing() {
        let subscribe_json = r#"{"type": "subscribe", "batch_ids": ["batch-1", "batch-2"]}"#;
        let msg: ClientMessage = serde_json::from_str(subscribe_json).unwrap();
        match msg {
            ClientMessage::Subscribe { batch_ids } => {
                assert_eq!(batch_ids.len(), 2);
                assert_eq!(batch_ids[0], "batch-1");
            }
            _ => panic!("Expected Subscribe"),
        }

        let ping_json = r#"{"type": "ping"}"#;
        let msg: ClientMessage = serde_json::from_str(ping_json).unwrap();
        assert!(matches!(msg, ClientMessage::Ping));
    }
}
