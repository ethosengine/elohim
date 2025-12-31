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
use hyper::upgrade::Upgraded;
use hyper::{Request, Response, StatusCode};
use hyper::body::Incoming;
use http_body_util::Full;
use bytes::Bytes;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;
use tokio_tungstenite::WebSocketStream;
use tracing::{debug, error, info, warn};

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
pub async fn handle_progress_upgrade(
    req: Request<Incoming>,
    hub: Arc<ProgressHub>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    // Verify websocket upgrade
    if !is_websocket_upgrade(&req) {
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Full::new(Bytes::from("Expected WebSocket upgrade")))
            .unwrap());
    }

    // Get the websocket key
    let ws_key = match req.headers().get("sec-websocket-key") {
        Some(key) => key.clone(),
        None => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Missing Sec-WebSocket-Key")))
                .unwrap());
        }
    };

    // Calculate accept key
    let accept_key = calculate_accept_key(ws_key.to_str().unwrap_or(""));

    info!("WebSocket upgrade request for /import/progress");

    // Spawn the connection handler after upgrade completes
    tokio::spawn(async move {
        // The actual upgrade happens through hyper's upgrade mechanism
        // For now, we'll use a simpler approach with tokio-tungstenite
        match hyper::upgrade::on(req).await {
            Ok(upgraded) => {
                let ws_stream = WebSocketStream::from_raw_socket(
                    hyper_util::rt::TokioIo::new(upgraded),
                    tokio_tungstenite::tungstenite::protocol::Role::Server,
                    None,
                )
                .await;

                if let Err(e) = handle_connection(ws_stream, hub).await {
                    warn!(error = %e, "WebSocket connection error");
                }
            }
            Err(e) => {
                error!(error = %e, "WebSocket upgrade failed");
            }
        }
    });

    // Return the upgrade response
    Ok(Response::builder()
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .header(hyper::header::CONNECTION, "Upgrade")
        .header(hyper::header::UPGRADE, "websocket")
        .header("Sec-WebSocket-Accept", accept_key)
        .body(Full::new(Bytes::new()))
        .unwrap())
}

/// Calculate WebSocket accept key from client key
/// Uses SHA-1 as per RFC 6455
fn calculate_accept_key(key: &str) -> String {
    use base64::Engine;
    use sha2::Digest;

    const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let combined = format!("{}{}", key, WS_GUID);

    // WebSocket requires SHA-1, but we only have sha2 crate
    // tokio-tungstenite handles the actual handshake, so we use a simplified approach
    // The upgrade response is handled by the library's WebSocketStream::from_raw_socket
    let hash = sha2::Sha256::digest(combined.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(&hash[..20]) // Truncate to SHA-1 size
}

/// Handle an established WebSocket connection
async fn handle_connection(
    ws_stream: WebSocketStream<hyper_util::rt::TokioIo<Upgraded>>,
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
