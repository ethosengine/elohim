//! WebSocket Transport Layer
//!
//! Single responsibility: Connect to a WebSocket and send/receive binary messages.
//! No knowledge of Holochain protocol, authentication, or session management.

use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{http::Request, protocol::Message, Error as WsError},
    MaybeTlsStream, WebSocketStream,
};
use tracing::debug;

use crate::error::StorageError;

/// Type alias for the WebSocket send half
pub type WsSink = SplitSink<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>, Message>;

/// Type alias for the WebSocket receive half
pub type WsStream = SplitStream<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>>;

/// A connected WebSocket transport.
///
/// This type represents a raw WebSocket connection with no protocol knowledge.
/// It can only be constructed via `Transport::connect()`.
pub struct Transport {
    sink: WsSink,
    stream: WsStream,
}

impl Transport {
    /// Connect to a WebSocket endpoint.
    ///
    /// Returns a Transport only when the connection is established.
    pub async fn connect(url: &str) -> Result<Self, StorageError> {
        debug!(url = %url, "Connecting to WebSocket");

        let request = Request::builder()
            .uri(url)
            .header("Host", extract_host(url))
            .header("Origin", "http://localhost")
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header(
                "Sec-WebSocket-Key",
                tokio_tungstenite::tungstenite::handshake::client::generate_key(),
            )
            .body(())
            .map_err(|e| StorageError::Conductor(format!("Failed to build request: {}", e)))?;

        let (ws, _) = connect_async_with_config(request, None, false)
            .await
            .map_err(|e| StorageError::Conductor(format!("WebSocket connect failed: {}", e)))?;

        let (sink, stream) = ws.split();

        debug!(url = %url, "WebSocket connected");
        Ok(Self { sink, stream })
    }

    /// Send a binary message.
    pub async fn send(&mut self, data: Vec<u8>) -> Result<(), StorageError> {
        self.sink
            .send(Message::Binary(data))
            .await
            .map_err(|e| StorageError::Conductor(format!("Failed to send: {}", e)))
    }

    /// Receive the next binary message.
    ///
    /// Returns None if the connection is closed.
    /// Skips non-binary messages (ping/pong handled automatically).
    pub async fn recv(&mut self) -> Result<Option<Vec<u8>>, StorageError> {
        loop {
            match self.stream.next().await {
                Some(Ok(Message::Binary(data))) => return Ok(Some(data)),
                Some(Ok(Message::Close(_))) => return Ok(None),
                Some(Ok(Message::Ping(_))) => {
                    // Pong is handled automatically by tungstenite
                    continue;
                }
                Some(Ok(_)) => continue, // Skip text, pong, frame messages
                Some(Err(e)) => {
                    return Err(StorageError::Conductor(format!("WebSocket error: {}", e)))
                }
                None => return Ok(None), // Stream ended
            }
        }
    }

    /// Split into separate sink and stream for concurrent send/receive.
    pub fn split(self) -> (WsSink, WsStream) {
        (self.sink, self.stream)
    }

    /// Check if we can still receive (non-destructive peek attempt).
    pub async fn is_alive(&mut self) -> bool {
        // Try to receive with a very short timeout
        match tokio::time::timeout(std::time::Duration::from_millis(1), self.stream.next()).await {
            Ok(Some(Ok(Message::Close(_)))) => false,
            Ok(Some(Err(_))) => false,
            Ok(None) => false,
            _ => true, // Timeout or got a message = still alive
        }
    }
}

/// Extract host from URL for Host header
fn extract_host(url: &str) -> &str {
    url.split("//")
        .nth(1)
        .and_then(|s| s.split('/').next())
        .unwrap_or("localhost")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_host() {
        assert_eq!(extract_host("ws://localhost:4444"), "localhost:4444");
        assert_eq!(extract_host("wss://example.com/path"), "example.com");
        assert_eq!(extract_host("invalid"), "localhost");
    }
}
