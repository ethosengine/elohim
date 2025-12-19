//! NATS-based proxy for scalable request routing
//!
//! Instead of direct WebSocket proxy connections, this module routes requests
//! through NATS JetStream to backend workers. This allows for:
//! - Horizontal scaling of workers
//! - Better resource management
//! - Request queuing and retry
//! - Separation of gateway and conductor concerns

use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio_tungstenite::tungstenite::protocol::Message;
use tracing::{error, info, warn};

use crate::auth::{is_operation_allowed, PermissionLevel};
use crate::nats::GatewayPublisher;
use crate::proxy::holochain::{encode_error, parse_message};
use crate::types::Result;

type HyperWebSocket = hyper_tungstenite::WebSocketStream<hyper_util::rt::TokioIo<hyper::upgrade::Upgraded>>;

/// Run the NATS-based admin proxy
///
/// Routes admin requests through NATS to workers instead of direct conductor connection.
pub async fn run_admin_proxy(
    client_ws: HyperWebSocket,
    gateway: Arc<GatewayPublisher>,
    origin: Option<String>,
    dev_mode: bool,
    permission_level: PermissionLevel,
) -> Result<()> {
    info!(
        "Starting NATS admin proxy (origin: {:?}, permission: {})",
        origin, permission_level
    );

    let (mut client_sink, mut client_stream) = client_ws.split();

    while let Some(msg_result) = client_stream.next().await {
        match msg_result {
            Ok(Message::Binary(data)) => {
                // Check if this call is allowed based on permission level
                if !dev_mode {
                    match parse_message(&data) {
                        Ok(parsed) => {
                            if !is_operation_allowed(&parsed.operation, permission_level) {
                                warn!("Blocked admin call '{}' due to insufficient permissions", parsed.operation);

                                // Send error response
                                let error_response = encode_error(&format!(
                                    "Operation '{}' requires higher permission level (current: {})",
                                    parsed.operation, permission_level
                                ));
                                if let Err(e) = client_sink.send(Message::Binary(error_response)).await {
                                    error!("Failed to send error response: {}", e);
                                    break;
                                }
                                continue;
                            }
                        }
                        Err(e) => {
                            warn!("Blocking unparseable message in production mode: {}", e);
                            let error_response = encode_error("Invalid message format");
                            if let Err(e) = client_sink.send(Message::Binary(error_response)).await {
                                error!("Failed to send error response: {}", e);
                                break;
                            }
                            continue;
                        }
                    }
                }

                // Route through NATS
                match gateway.request(data, "admin", None).await {
                    Ok(response) => {
                        if let Err(e) = client_sink.send(Message::Binary(response)).await {
                            error!("Failed to send response to client: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("NATS request failed: {}", e);
                        // Create an error response in MessagePack format
                        let error_response = create_error_response(&e.to_string());
                        if let Err(e) = client_sink.send(Message::Binary(error_response)).await {
                            error!("Failed to send error response: {}", e);
                            break;
                        }
                    }
                }
            }
            Ok(Message::Ping(data)) => {
                let _ = client_sink.send(Message::Pong(data)).await;
            }
            Ok(Message::Close(frame)) => {
                info!("Client closed admin connection: {:?}", frame);
                break;
            }
            Ok(_) => {}
            Err(e) => {
                error!("Admin WebSocket error: {}", e);
                break;
            }
        }
    }

    info!("NATS admin proxy connection closed");
    Ok(())
}

/// Run the NATS-based app proxy
///
/// Routes app requests through NATS to workers.
pub async fn run_app_proxy(
    client_ws: HyperWebSocket,
    gateway: Arc<GatewayPublisher>,
    port: u16,
    origin: Option<String>,
) -> Result<()> {
    info!(
        "Starting NATS app proxy to port {} (origin: {:?})",
        port, origin
    );

    let (mut client_sink, mut client_stream) = client_ws.split();

    while let Some(msg_result) = client_stream.next().await {
        match msg_result {
            Ok(Message::Binary(data)) => {
                // Route through NATS with app port
                match gateway.request(data, "app", Some(port)).await {
                    Ok(response) => {
                        if let Err(e) = client_sink.send(Message::Binary(response)).await {
                            error!("Failed to send response to client: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("NATS request failed: {}", e);
                        let error_response = create_error_response(&e.to_string());
                        if let Err(e) = client_sink.send(Message::Binary(error_response)).await {
                            error!("Failed to send error response: {}", e);
                            break;
                        }
                    }
                }
            }
            Ok(Message::Text(text)) => {
                // Some clients send text - convert to binary
                match gateway.request(text.into_bytes(), "app", Some(port)).await {
                    Ok(response) => {
                        if let Err(e) = client_sink.send(Message::Binary(response)).await {
                            error!("Failed to send response to client: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("NATS request failed: {}", e);
                        break;
                    }
                }
            }
            Ok(Message::Ping(data)) => {
                let _ = client_sink.send(Message::Pong(data)).await;
            }
            Ok(Message::Close(frame)) => {
                info!("Client closed app connection: {:?}", frame);
                break;
            }
            Ok(_) => {}
            Err(e) => {
                error!("App WebSocket error: {}", e);
                break;
            }
        }
    }

    info!("NATS app proxy connection closed (port {})", port);
    Ok(())
}

/// Create a MessagePack error response
fn create_error_response(message: &str) -> Vec<u8> {
    encode_error(message)
}
