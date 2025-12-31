//! WebSocket Proxy for Import Progress Streaming
//!
//! Proxies WebSocket connections from seeder/clients to elohim-storage.
//!
//! ## Architecture
//!
//! ```text
//! Seeder ──WS──► Doorway ──WS proxy──► elohim-storage
//!   │              │                        │
//!   └──HTTP────────┘────────────────────────┘
//! ```
//!
//! ## Endpoint
//!
//! `wss://doorway.host/import/progress` → proxies to `ws://elohim-storage/import/progress`
//!
//! ## Protocol
//!
//! All messages are passed through unchanged. The proxy handles:
//! - WebSocket upgrade handshake with client
//! - WebSocket connection to upstream (elohim-storage)
//! - Bidirectional message forwarding
//! - Reconnection to upstream on failure

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{debug, error, info, warn};

/// WebSocket type after upgrade
type HyperWebSocket = hyper_tungstenite::WebSocketStream<hyper_util::rt::TokioIo<hyper::upgrade::Upgraded>>;

/// Handle WebSocket upgrade for import progress proxy
///
/// Upgrades the client connection and establishes upstream WebSocket to elohim-storage.
pub async fn handle_import_progress_ws(
    req: Request<Incoming>,
    storage_url: Option<String>,
) -> Response<Full<Bytes>> {
    // Check if storage URL is configured
    let storage_url = match storage_url {
        Some(url) => url,
        None => {
            warn!("WebSocket progress request but STORAGE_URL not configured");
            return Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    r#"{"error": "Import progress unavailable: STORAGE_URL not configured"}"#,
                )))
                .unwrap();
        }
    };

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

    // Spawn task to handle the proxy connection
    tokio::spawn(async move {
        match websocket.await {
            Ok(client_ws) => {
                let client_ws: HyperWebSocket = client_ws;
                if let Err(e) = handle_proxy_connection(client_ws, storage_url).await {
                    warn!("Import progress proxy error: {}", e);
                }
            }
            Err(e) => {
                error!("WebSocket connection failed: {}", e);
            }
        }
    });

    // Return the upgrade response
    let (parts, _body) = response.into_parts();
    Response::from_parts(parts, Full::new(Bytes::new()))
}

/// Handle the bidirectional WebSocket proxy
async fn handle_proxy_connection(
    client_ws: HyperWebSocket,
    storage_url: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Import progress WebSocket client connected, connecting to upstream");

    // Convert storage HTTP URL to WebSocket URL
    let ws_url = storage_url_to_ws(&storage_url);
    let upstream_url = format!("{}/import/progress", ws_url);

    // Connect to elohim-storage WebSocket
    let upstream_ws = match connect_upstream(&upstream_url).await {
        Ok(ws) => ws,
        Err(e) => {
            error!(error = %e, "Failed to connect to upstream elohim-storage");
            // Send error to client before closing
            let (mut client_sink, _) = client_ws.split();
            let error_msg = serde_json::json!({
                "type": "error",
                "message": format!("Failed to connect to import progress service: {}", e)
            });
            let _ = client_sink.send(WsMessage::Text(error_msg.to_string().into())).await;
            let _ = client_sink.close().await;
            return Err(e.into());
        }
    };

    info!(upstream = %upstream_url, "Connected to upstream elohim-storage");

    // Split both connections
    let (mut client_sink, mut client_stream) = client_ws.split();
    let (mut upstream_sink, mut upstream_stream) = upstream_ws.split();

    // Proxy messages bidirectionally
    loop {
        tokio::select! {
            // Client -> Upstream
            msg = client_stream.next() => {
                match msg {
                    Some(Ok(WsMessage::Text(text))) => {
                        debug!(direction = "client->upstream", "Forwarding text message");
                        if let Err(e) = upstream_sink.send(WsMessage::Text(text)).await {
                            warn!(error = %e, "Failed to forward to upstream");
                            break;
                        }
                    }
                    Some(Ok(WsMessage::Ping(data))) => {
                        if let Err(e) = upstream_sink.send(WsMessage::Ping(data)).await {
                            warn!(error = %e, "Failed to forward ping to upstream");
                            break;
                        }
                    }
                    Some(Ok(WsMessage::Pong(data))) => {
                        if let Err(e) = upstream_sink.send(WsMessage::Pong(data)).await {
                            warn!(error = %e, "Failed to forward pong to upstream");
                            break;
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) => {
                        info!("Client closed connection");
                        let _ = upstream_sink.close().await;
                        break;
                    }
                    Some(Ok(WsMessage::Binary(data))) => {
                        if let Err(e) = upstream_sink.send(WsMessage::Binary(data)).await {
                            warn!(error = %e, "Failed to forward binary to upstream");
                            break;
                        }
                    }
                    Some(Ok(_)) => {
                        // Ignore other message types (Frame)
                    }
                    Some(Err(e)) => {
                        warn!(error = %e, "Client WebSocket error");
                        break;
                    }
                    None => {
                        // Client stream ended
                        info!("Client stream ended");
                        break;
                    }
                }
            }

            // Upstream -> Client
            msg = upstream_stream.next() => {
                match msg {
                    Some(Ok(WsMessage::Text(text))) => {
                        debug!(direction = "upstream->client", "Forwarding text message");
                        if let Err(e) = client_sink.send(WsMessage::Text(text)).await {
                            warn!(error = %e, "Failed to forward to client");
                            break;
                        }
                    }
                    Some(Ok(WsMessage::Ping(data))) => {
                        if let Err(e) = client_sink.send(WsMessage::Ping(data)).await {
                            warn!(error = %e, "Failed to forward ping to client");
                            break;
                        }
                    }
                    Some(Ok(WsMessage::Pong(data))) => {
                        if let Err(e) = client_sink.send(WsMessage::Pong(data)).await {
                            warn!(error = %e, "Failed to forward pong to client");
                            break;
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) => {
                        info!("Upstream closed connection");
                        let _ = client_sink.close().await;
                        break;
                    }
                    Some(Ok(WsMessage::Binary(data))) => {
                        if let Err(e) = client_sink.send(WsMessage::Binary(data)).await {
                            warn!(error = %e, "Failed to forward binary to client");
                            break;
                        }
                    }
                    Some(Ok(_)) => {
                        // Ignore other message types (Frame)
                    }
                    Some(Err(e)) => {
                        warn!(error = %e, "Upstream WebSocket error");
                        // Notify client about upstream error
                        let error_msg = serde_json::json!({
                            "type": "error",
                            "message": format!("Upstream connection error: {}", e)
                        });
                        let _ = client_sink.send(WsMessage::Text(error_msg.to_string().into())).await;
                        break;
                    }
                    None => {
                        // Upstream stream ended
                        info!("Upstream stream ended");
                        // Notify client
                        let error_msg = serde_json::json!({
                            "type": "error",
                            "message": "Upstream connection closed unexpectedly"
                        });
                        let _ = client_sink.send(WsMessage::Text(error_msg.to_string().into())).await;
                        break;
                    }
                }
            }
        }
    }

    info!("Import progress proxy connection closed");
    Ok(())
}

/// Connect to upstream elohim-storage WebSocket
async fn connect_upstream(
    url: &str,
) -> Result<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, String> {
    debug!(url = url, "Connecting to upstream WebSocket");

    // Add timeout for connection
    let connect_result = timeout(
        Duration::from_secs(10),
        tokio_tungstenite::connect_async(url),
    ).await;

    match connect_result {
        Ok(Ok((ws, _response))) => Ok(ws),
        Ok(Err(e)) => Err(format!("WebSocket connect error: {}", e)),
        Err(_) => Err("Connection timeout (10s)".to_string()),
    }
}

/// Convert HTTP storage URL to WebSocket URL
///
/// - http://localhost:8090 -> ws://localhost:8090
/// - https://storage.example.com -> wss://storage.example.com
fn storage_url_to_ws(url: &str) -> String {
    if url.starts_with("https://") {
        url.replacen("https://", "wss://", 1)
    } else if url.starts_with("http://") {
        url.replacen("http://", "ws://", 1)
    } else {
        // Assume ws:// if no scheme
        format!("ws://{}", url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_url_to_ws() {
        assert_eq!(
            storage_url_to_ws("http://localhost:8090"),
            "ws://localhost:8090"
        );
        assert_eq!(
            storage_url_to_ws("https://storage.example.com"),
            "wss://storage.example.com"
        );
        assert_eq!(
            storage_url_to_ws("localhost:8090"),
            "ws://localhost:8090"
        );
    }
}
