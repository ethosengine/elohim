//! Admin interface proxy
//!
//! Bidirectional WebSocket proxy between client and Holochain conductor admin interface.
//! In dev mode: passthrough all messages
//! In production: filter messages based on permission level

use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{http::Request, protocol::Message},
};
use tracing::{debug, error, info, warn};

use crate::auth::{is_operation_allowed, PermissionLevel};
use crate::proxy::holochain::{encode_error, parse_message};
use crate::types::{DoorwayError, Result};

type HyperWebSocket =
    hyper_tungstenite::WebSocketStream<hyper_util::rt::TokioIo<hyper::upgrade::Upgraded>>;

/// Run the admin proxy between client and conductor
pub async fn run_proxy(
    client_ws: HyperWebSocket,
    conductor_url: &str,
    _origin: Option<String>,
    dev_mode: bool,
    permission_level: PermissionLevel,
) -> Result<()> {
    info!(
        "Creating {} admin proxy to {} (permission: {})",
        if dev_mode { "passthrough" } else { "filtered" },
        conductor_url,
        permission_level
    );

    // Connect to conductor with proper headers
    // Holochain requires an Origin header for WebSocket connections
    let request = Request::builder()
        .uri(conductor_url)
        .header(
            "Host",
            conductor_url.split("//").last().unwrap_or("localhost"),
        )
        .header("Origin", "http://localhost")
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header(
            "Sec-WebSocket-Key",
            tokio_tungstenite::tungstenite::handshake::client::generate_key(),
        )
        .body(())
        .map_err(|e| DoorwayError::Holochain(format!("Failed to build request: {}", e)))?;

    let (conductor_ws, _) = connect_async_with_config(request, None, false)
        .await
        .map_err(|e| DoorwayError::Holochain(format!("Failed to connect to conductor: {}", e)))?;

    info!("Connected to conductor at {}", conductor_url);

    // Split both connections and wrap sinks in Arc<Mutex> for shared access
    let (client_sink, mut client_stream) = client_ws.split();
    let (conductor_sink, mut conductor_stream) = conductor_ws.split();

    let client_sink = Arc::new(Mutex::new(client_sink));
    let conductor_sink = Arc::new(Mutex::new(conductor_sink));

    // Clone for the two async tasks
    let client_sink_for_client = Arc::clone(&client_sink);
    let conductor_sink_for_client = Arc::clone(&conductor_sink);
    let client_sink_for_conductor = Arc::clone(&client_sink);

    // Bidirectional message forwarding
    let client_to_conductor = async move {
        while let Some(msg) = client_stream.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    // In dev mode, passthrough without filtering
                    if dev_mode {
                        let mut sink = conductor_sink_for_client.lock().await;
                        if let Err(e) = sink.send(Message::Binary(data)).await {
                            error!("Failed to send to conductor: {}", e);
                            break;
                        }
                    } else {
                        // Parse message and check permissions
                        match filter_message(&data, permission_level) {
                            FilterResult::Allow => {
                                let mut sink = conductor_sink_for_client.lock().await;
                                if let Err(e) = sink.send(Message::Binary(data)).await {
                                    error!("Failed to send to conductor: {}", e);
                                    break;
                                }
                            }
                            FilterResult::Deny(error_msg) => {
                                warn!("Blocked operation: {}", error_msg);
                                // Send error response back to client
                                let error_response = encode_error(&error_msg);
                                let mut sink = client_sink_for_client.lock().await;
                                if let Err(e) = sink.send(Message::Binary(error_response)).await {
                                    error!("Failed to send error to client: {}", e);
                                    break;
                                }
                            }
                            FilterResult::PassThrough => {
                                // Couldn't parse message, block in production
                                warn!("Blocking unparseable message in production mode");
                                let error_response = encode_error("Invalid message format");
                                let mut sink = client_sink_for_client.lock().await;
                                if let Err(e) = sink.send(Message::Binary(error_response)).await {
                                    error!("Failed to send error to client: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                }
                Ok(Message::Text(text)) => {
                    // Forward text messages (shouldn't happen with Holochain, but handle it)
                    let mut sink = conductor_sink_for_client.lock().await;
                    if let Err(e) = sink.send(Message::Text(text)).await {
                        error!("Failed to send text to conductor: {}", e);
                        break;
                    }
                }
                Ok(Message::Ping(data)) => {
                    let mut sink = conductor_sink_for_client.lock().await;
                    if let Err(e) = sink.send(Message::Ping(data)).await {
                        debug!("Failed to forward ping: {}", e);
                    }
                }
                Ok(Message::Pong(data)) => {
                    let mut sink = conductor_sink_for_client.lock().await;
                    if let Err(e) = sink.send(Message::Pong(data)).await {
                        debug!("Failed to forward pong: {}", e);
                    }
                }
                Ok(Message::Close(frame)) => {
                    info!("Client closed connection: {:?}", frame);
                    let mut sink = conductor_sink_for_client.lock().await;
                    let _ = sink.send(Message::Close(frame)).await;
                    break;
                }
                Ok(Message::Frame(_)) => {
                    // Raw frame, ignore
                }
                Err(e) => {
                    error!("Client WebSocket error: {}", e);
                    break;
                }
            }
        }
    };

    let conductor_to_client = async move {
        while let Some(msg) = conductor_stream.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    // Forward response back to client (no filtering on responses)
                    let mut sink = client_sink_for_conductor.lock().await;
                    if let Err(e) = sink.send(Message::Binary(data)).await {
                        error!("Failed to send to client: {}", e);
                        break;
                    }
                }
                Ok(Message::Text(text)) => {
                    let mut sink = client_sink_for_conductor.lock().await;
                    if let Err(e) = sink.send(Message::Text(text)).await {
                        error!("Failed to send text to client: {}", e);
                        break;
                    }
                }
                Ok(Message::Ping(data)) => {
                    let mut sink = client_sink_for_conductor.lock().await;
                    if let Err(e) = sink.send(Message::Ping(data)).await {
                        debug!("Failed to forward ping to client: {}", e);
                    }
                }
                Ok(Message::Pong(data)) => {
                    let mut sink = client_sink_for_conductor.lock().await;
                    if let Err(e) = sink.send(Message::Pong(data)).await {
                        debug!("Failed to forward pong to client: {}", e);
                    }
                }
                Ok(Message::Close(frame)) => {
                    info!("Conductor closed connection: {:?}", frame);
                    let mut sink = client_sink_for_conductor.lock().await;
                    let _ = sink.send(Message::Close(frame)).await;
                    break;
                }
                Ok(Message::Frame(_)) => {
                    // Raw frame, ignore
                }
                Err(e) => {
                    error!("Conductor WebSocket error: {}", e);
                    break;
                }
            }
        }
    };

    // Run both directions concurrently until one side closes
    tokio::select! {
        _ = client_to_conductor => {
            info!("Client->Conductor stream ended");
        }
        _ = conductor_to_client => {
            info!("Conductor->Client stream ended");
        }
    }

    info!("Admin proxy connection closed");
    Ok(())
}

/// Result of filtering a message
enum FilterResult {
    /// Allow the message to pass through
    Allow,
    /// Deny the message with an error message
    Deny(String),
    /// Couldn't parse the message
    PassThrough,
}

/// Filter a message based on permission level
fn filter_message(data: &[u8], permission_level: PermissionLevel) -> FilterResult {
    match parse_message(data) {
        Ok(parsed) => {
            let operation = &parsed.operation;

            if is_operation_allowed(operation, permission_level) {
                debug!(
                    "Allowing operation '{}' for permission level {}",
                    operation, permission_level
                );
                FilterResult::Allow
            } else {
                FilterResult::Deny(format!(
                    "Operation '{}' requires higher permission level (current: {})",
                    operation, permission_level
                ))
            }
        }
        Err(e) => {
            debug!("Failed to parse message: {}", e);
            FilterResult::PassThrough
        }
    }
}
