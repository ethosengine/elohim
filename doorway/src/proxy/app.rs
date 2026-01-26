//! App interface proxy
//!
//! Simple passthrough WebSocket proxy for app interfaces.
//! No message filtering needed - app interfaces handle their own auth.

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async_with_config, tungstenite::{protocol::Message, http::Request}};
use tracing::{debug, error, info};

use crate::types::{DoorwayError, Result};

type HyperWebSocket = hyper_tungstenite::WebSocketStream<hyper_util::rt::TokioIo<hyper::upgrade::Upgraded>>;

/// Run the app proxy between client and conductor app interface
pub async fn run_proxy(
    client_ws: HyperWebSocket,
    port: u16,
    origin: Option<String>,
    query: Option<String>,
) -> Result<()> {
    // Build app interface URL
    // Filter out Doorway-specific params (apiKey, token) - conductor uses its own auth
    let mut app_url = format!("ws://localhost:{}", port);
    if let Some(q) = query {
        let filtered: Vec<&str> = q
            .split('&')
            .filter(|param| {
                !param.starts_with("apiKey=") && !param.starts_with("token=")
            })
            .collect();
        if !filtered.is_empty() {
            app_url = format!("{}?{}", app_url, filtered.join("&"));
        }
    }

    info!("Creating app proxy to {} (origin: {:?})", app_url, origin);

    // Connect to conductor app interface with proper headers
    let request = Request::builder()
        .uri(&app_url)
        .header("Host", format!("localhost:{}", port))
        .header("Origin", "http://localhost")
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header("Sec-WebSocket-Key", tokio_tungstenite::tungstenite::handshake::client::generate_key())
        .body(())
        .map_err(|e| DoorwayError::Holochain(format!("Failed to build request: {}", e)))?;

    let (conductor_ws, _) = connect_async_with_config(request, None, false)
        .await
        .map_err(|e| DoorwayError::Holochain(format!("Failed to connect to app interface: {}", e)))?;

    info!("Connected to app interface on port {}", port);

    // Split both connections
    let (mut client_sink, mut client_stream) = client_ws.split();
    let (mut conductor_sink, mut conductor_stream) = conductor_ws.split();

    // Bidirectional passthrough - no filtering for app interfaces
    let client_to_conductor = async {
        while let Some(msg) = client_stream.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    if let Err(e) = conductor_sink.send(Message::Binary(data)).await {
                        error!("Failed to send to app interface: {}", e);
                        break;
                    }
                }
                Ok(Message::Text(text)) => {
                    if let Err(e) = conductor_sink.send(Message::Text(text)).await {
                        error!("Failed to send text to app interface: {}", e);
                        break;
                    }
                }
                Ok(Message::Ping(data)) => {
                    let _ = conductor_sink.send(Message::Ping(data)).await;
                }
                Ok(Message::Pong(data)) => {
                    let _ = conductor_sink.send(Message::Pong(data)).await;
                }
                Ok(Message::Close(frame)) => {
                    info!("Client closed app connection: {:?}", frame);
                    let _ = conductor_sink.send(Message::Close(frame)).await;
                    break;
                }
                Ok(Message::Frame(_)) => {}
                Err(e) => {
                    error!("Client app WebSocket error: {}", e);
                    break;
                }
            }
        }
    };

    let conductor_to_client = async {
        while let Some(msg) = conductor_stream.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    if let Err(e) = client_sink.send(Message::Binary(data.into())).await {
                        error!("Failed to send to app client: {}", e);
                        break;
                    }
                }
                Ok(Message::Text(text)) => {
                    if let Err(e) = client_sink.send(Message::Text(text)).await {
                        error!("Failed to send text to app client: {}", e);
                        break;
                    }
                }
                Ok(Message::Ping(data)) => {
                    let _ = client_sink.send(Message::Ping(data.into())).await;
                }
                Ok(Message::Pong(data)) => {
                    let _ = client_sink.send(Message::Pong(data.into())).await;
                }
                Ok(Message::Close(frame)) => {
                    info!("App interface closed connection: {:?}", frame);
                    let _ = client_sink.send(Message::Close(frame)).await;
                    break;
                }
                Ok(Message::Frame(_)) => {}
                Err(e) => {
                    error!("App interface WebSocket error: {}", e);
                    break;
                }
            }
        }
    };

    // Run both directions concurrently
    tokio::select! {
        _ = client_to_conductor => {
            debug!("App client->conductor stream ended");
        }
        _ = conductor_to_client => {
            debug!("App conductor->client stream ended");
        }
    }

    info!("App proxy connection closed (port {})", port);
    Ok(())
}
