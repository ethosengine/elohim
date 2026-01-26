//! Holochain Authentication
//!
//! Single responsibility: Obtain authentication tokens and authenticate connections.
//!
//! # Authentication Flow
//!
//! Holochain requires a two-step authentication process:
//!
//! 1. **Get Token from Admin Interface**
//!    - Connect to admin WebSocket (e.g., ws://localhost:4444)
//!    - Send `issue_app_authentication_token` request
//!    - Receive token bytes
//!
//! 2. **Authenticate on App Interface**
//!    - Connect to app WebSocket (e.g., ws://localhost:4445)
//!    - Send `authenticate` message with token
//!    - If invalid, conductor closes connection
//!    - If valid, connection is ready for zome calls
//!
//! # Why This Exists
//!
//! This module isolates the authentication concern so that:
//! - Session can require a valid AuthToken (type-safe guarantee)
//! - Token expiry and renewal are handled in one place
//! - Protocol changes to auth only affect this module

use rmpv::Value;
use std::io::Cursor;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, info};

use super::protocol::encode_admin_request;
use super::transport::Transport;
use crate::error::StorageError;

/// An authentication token obtained from the admin interface.
///
/// This type can only be created via `AuthToken::obtain()`, which
/// connects to the admin interface and requests a token.
///
/// The token is opaque bytes that must be sent to the app interface
/// to authenticate the connection.
#[derive(Clone)]
pub struct AuthToken {
    /// The raw token bytes from the conductor
    bytes: Vec<u8>,
    /// When this token expires (if known)
    #[allow(dead_code)]
    expires_at: Option<std::time::Instant>,
}

impl AuthToken {
    /// Obtain an authentication token from the admin interface.
    ///
    /// # Arguments
    /// * `admin_url` - WebSocket URL of the admin interface (e.g., "ws://localhost:4444")
    /// * `app_id` - The installed app ID to get a token for
    ///
    /// # Returns
    /// An `AuthToken` that can be used to authenticate on the app interface.
    ///
    /// # Errors
    /// - Connection to admin interface fails
    /// - Token request times out (10 seconds)
    /// - Admin returns an error
    pub async fn obtain(admin_url: &str, app_id: &str) -> Result<Self, StorageError> {
        debug!(admin_url = %admin_url, app_id = %app_id, "Obtaining auth token");

        // Connect to admin interface
        let mut transport = Transport::connect(admin_url).await?;

        // Build token request
        let request_value = Value::Map(vec![
            (Value::String("installed_app_id".into()), Value::String(app_id.into())),
            (Value::String("expiry_seconds".into()), Value::Integer(3600.into())),
            (Value::String("single_use".into()), Value::Boolean(false)),
        ]);

        let request_bytes = encode_admin_request(1, "issue_app_authentication_token", request_value)?;

        // Send request
        transport.send(request_bytes).await?;

        // Wait for response with timeout
        let response = timeout(Duration::from_secs(10), transport.recv())
            .await
            .map_err(|_| StorageError::Conductor("Auth token request timeout".into()))?
            .map_err(|e| StorageError::Conductor(format!("Auth token request failed: {}", e)))?
            .ok_or_else(|| StorageError::Conductor("Admin connection closed".into()))?;

        // Extract token from response
        let token_bytes = extract_token_from_response(&response)?;

        info!("Auth token obtained successfully");

        Ok(Self {
            bytes: token_bytes,
            expires_at: Some(std::time::Instant::now() + Duration::from_secs(3600)),
        })
    }

    /// Authenticate a transport connection using this token.
    ///
    /// # Arguments
    /// * `transport` - A connected transport to the app interface
    ///
    /// # Returns
    /// Ok(()) if authentication succeeded.
    ///
    /// # How It Works
    /// Sends the token to the conductor and waits briefly.
    /// If the token is invalid, the conductor closes the connection.
    /// If valid, the connection remains open and is ready for use.
    pub async fn authenticate(&self, transport: &mut Transport) -> Result<(), StorageError> {
        use super::protocol::encode_authenticate;

        debug!("Authenticating connection");

        let auth_bytes = encode_authenticate(&self.bytes)?;
        transport.send(auth_bytes).await?;

        // Conductor doesn't send an explicit success response.
        // It just closes the connection if auth fails.
        // Wait briefly then check if connection is still alive.
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Try to receive - if we get a Close message, auth failed
        let check_result = timeout(Duration::from_millis(50), transport.recv()).await;

        match check_result {
            Ok(Ok(None)) => {
                // Connection closed = auth rejected
                Err(StorageError::Conductor("Authentication rejected - connection closed".into()))
            }
            Ok(Err(e)) => {
                // Error = auth rejected
                Err(StorageError::Conductor(format!("Authentication failed: {}", e)))
            }
            Ok(Ok(Some(_))) => {
                // Unexpected message, but connection is alive
                debug!("Received unexpected message during auth, but connection alive");
                Ok(())
            }
            Err(_) => {
                // Timeout = connection still open = auth succeeded
                debug!("Authentication successful");
                Ok(())
            }
        }
    }

    /// Get the raw token bytes (for debugging/logging only).
    #[allow(dead_code)]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Extract the token bytes from an admin response.
///
/// Response structure:
/// ```text
/// {
///     "id": 1,
///     "type": "response",
///     "data": <binary> -> {
///         "type": "...",
///         "value": {
///             "token": <binary>,
///             "expires_at": <timestamp>
///         }
///     }
/// }
/// ```
fn extract_token_from_response(data: &[u8]) -> Result<Vec<u8>, StorageError> {
    use rmpv::decode::read_value;

    let mut cursor = Cursor::new(data);
    let value = read_value(&mut cursor)
        .map_err(|e| StorageError::Internal(format!("Failed to decode response: {}", e)))?;

    // Navigate: envelope.data -> inner.value -> token
    let envelope = value
        .as_map()
        .ok_or_else(|| StorageError::Internal("Response is not a map".into()))?;

    // Get envelope.data (binary containing inner msgpack)
    let envelope_data = envelope
        .iter()
        .find(|(k, _)| k.as_str() == Some("data"))
        .map(|(_, v)| v)
        .ok_or_else(|| StorageError::Internal("No 'data' in envelope".into()))?;

    let inner_bytes = envelope_data
        .as_slice()
        .ok_or_else(|| StorageError::Internal("Envelope data is not binary".into()))?;

    // Parse inner response
    let mut inner_cursor = Cursor::new(inner_bytes);
    let inner_value = read_value(&mut inner_cursor)
        .map_err(|e| StorageError::Internal(format!("Failed to decode inner response: {}", e)))?;

    let inner_map = inner_value
        .as_map()
        .ok_or_else(|| StorageError::Internal("Inner response is not a map".into()))?;

    // Get inner.value (contains token and expires_at)
    let token_struct = inner_map
        .iter()
        .find(|(k, _)| k.as_str() == Some("value"))
        .map(|(_, v)| v)
        .ok_or_else(|| StorageError::Internal("No 'value' in inner response".into()))?;

    let token_map = token_struct
        .as_map()
        .ok_or_else(|| StorageError::Internal("Token struct is not a map".into()))?;

    // Get the actual token bytes
    let token_value = token_map
        .iter()
        .find(|(k, _)| k.as_str() == Some("token"))
        .map(|(_, v)| v)
        .ok_or_else(|| StorageError::Internal("No 'token' field".into()))?;

    // Token might be binary or array of integers (depending on msgpack version)
    match token_value {
        Value::Binary(bytes) => Ok(bytes.clone()),
        Value::Array(arr) => {
            // Convert array of integers to bytes
            arr.iter()
                .map(|v| match v {
                    Value::Integer(i) => i
                        .as_u64()
                        .map(|n| n as u8)
                        .ok_or("Invalid byte value"),
                    _ => Err("Token array contains non-integer"),
                })
                .collect::<Result<Vec<u8>, _>>()
                .map_err(|e| StorageError::Internal(format!("Failed to convert token: {}", e)))
        }
        _ => Err(StorageError::Internal(format!(
            "Unexpected token type: {:?}",
            token_value
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests would require a running conductor
    // Unit tests for the extraction logic could go here
}
