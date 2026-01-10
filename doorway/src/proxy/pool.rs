//! Pool-based proxy for WebSocket routing
//!
//! Routes requests through an in-process worker pool instead of creating
//! direct conductor connections per client. Provides:
//! - Connection pooling (fixed number of conductor connections)
//! - Request queuing under load
//! - No thread starvation

use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio_tungstenite::tungstenite::protocol::Message;
use tracing::{error, info, warn};

use crate::auth::{is_operation_allowed, PermissionLevel};
use crate::proxy::holochain::{encode_error, parse_message};
use crate::types::Result;
use crate::worker::WorkerPool;

type HyperWebSocket = hyper_tungstenite::WebSocketStream<hyper_util::rt::TokioIo<hyper::upgrade::Upgraded>>;

/// Run the pool-based admin proxy
pub async fn run_admin_proxy(
    client_ws: HyperWebSocket,
    pool: Arc<WorkerPool>,
    origin: Option<String>,
    dev_mode: bool,
    permission_level: PermissionLevel,
) -> Result<()> {
    info!(
        "Starting pool admin proxy (origin: {:?}, permission: {})",
        origin, permission_level
    );

    let (mut client_sink, mut client_stream) = client_ws.split();

    while let Some(msg_result) = client_stream.next().await {
        match msg_result {
            Ok(Message::Binary(data)) => {
                info!("Admin proxy received binary message ({} bytes)", data.len());
                // Check permissions in production mode
                if !dev_mode {
                    match parse_message(&data) {
                        Ok(parsed) => {
                            if !is_operation_allowed(&parsed.operation, permission_level) {
                                warn!("Blocked admin call '{}' due to insufficient permissions", parsed.operation);
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

                // Route through worker pool
                match pool.request(data).await {
                    Ok(response) => {
                        if let Err(e) = client_sink.send(Message::Binary(response)).await {
                            error!("Failed to send response to client: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Pool request failed: {}", e);
                        let error_response = encode_error(&e.to_string());
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

    info!("Pool admin proxy connection closed");
    Ok(())
}

/// Run the pool-based app proxy
pub async fn run_app_proxy(
    client_ws: HyperWebSocket,
    pool: Arc<WorkerPool>,
    port: u16,
    origin: Option<String>,
) -> Result<()> {
    info!(
        "Starting pool app proxy to port {} (origin: {:?})",
        port, origin
    );

    let (mut client_sink, mut client_stream) = client_ws.split();

    while let Some(msg_result) = client_stream.next().await {
        match msg_result {
            Ok(Message::Binary(data)) => {
                match pool.request(data).await {
                    Ok(response) => {
                        if let Err(e) = client_sink.send(Message::Binary(response)).await {
                            error!("Failed to send response to client: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Pool request failed: {}", e);
                        let error_response = encode_error(&e.to_string());
                        if let Err(e) = client_sink.send(Message::Binary(error_response)).await {
                            error!("Failed to send error response: {}", e);
                            break;
                        }
                    }
                }
            }
            Ok(Message::Text(text)) => {
                match pool.request(text.into_bytes()).await {
                    Ok(response) => {
                        if let Err(e) = client_sink.send(Message::Binary(response)).await {
                            error!("Failed to send response to client: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Pool request failed: {}", e);
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

    info!("Pool app proxy connection closed (port {})", port);
    Ok(())
}
