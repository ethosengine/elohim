//! Bootstrap service for agent discovery
//!
//! Implements the Holochain bootstrap protocol for agent registration and discovery.
//! Agents register their presence in a DHT space, and other agents can query to
//! find random peers to connect to.
//!
//! Protocol:
//! - POST /bootstrap/put   - Register agent in space (MessagePack body)
//! - POST /bootstrap/random - Get random agents in space (MessagePack body)
//! - POST /bootstrap/now   - Get server timestamp (empty body)

pub mod store;
mod types;

pub use store::BootstrapStore;
pub use types::*;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};

/// Maximum time to hold agent info (1 hour in milliseconds)
pub const MAX_HOLD_MS: u64 = 60 * 60 * 1000;

/// Minimum expiry time (1 minute in milliseconds)
pub const MIN_EXPIRES_MS: u64 = 60 * 1000;

/// Maximum expiry time (1 hour in milliseconds)
pub const MAX_EXPIRES_MS: u64 = 60 * 60 * 1000;

/// Maximum URL size in bytes
pub const MAX_URL_SIZE: usize = 2048;

/// Maximum number of URLs per agent
pub const MAX_URLS: usize = 256;

/// Handle PUT request - register agent in space
pub async fn handle_put(
    store: Arc<BootstrapStore>,
    body: Bytes,
    network: &str,
) -> Response<Full<Bytes>> {
    debug!("Bootstrap PUT request ({} bytes)", body.len());

    // Decode and validate the signed agent info
    let signed_info = match SignedAgentInfo::decode_and_verify(&body) {
        Ok(info) => info,
        Err(e) => {
            warn!("Bootstrap PUT decode error: {}", e);
            return error_response(StatusCode::BAD_REQUEST, &e);
        }
    };

    // Validate timing constraints
    let now_ms = current_time_ms();
    if let Err(e) = signed_info.validate_timing(now_ms) {
        warn!("Bootstrap PUT timing error: {}", e);
        return error_response(StatusCode::BAD_REQUEST, &e);
    }

    // Store the agent info
    let key = store.put(network, &signed_info, body.to_vec());
    debug!("Bootstrap PUT stored agent: {}", key);

    // Return null (success) as MessagePack
    msgpack_response(&rmpv::Value::Nil)
}

/// Handle RANDOM request - get random agents in space
pub async fn handle_random(
    store: Arc<BootstrapStore>,
    body: Bytes,
    network: &str,
) -> Response<Full<Bytes>> {
    debug!("Bootstrap RANDOM request ({} bytes)", body.len());

    // Decode the query
    let query = match RandomQuery::decode(&body) {
        Ok(q) => q,
        Err(e) => {
            warn!("Bootstrap RANDOM decode error: {}", e);
            return error_response(StatusCode::BAD_REQUEST, &e);
        }
    };

    debug!(
        "Bootstrap RANDOM: space={}, limit={}",
        hex::encode(&query.space[..8]),
        query.limit
    );

    // Get random agents
    let agents = store.random(network, &query.space, query.limit);
    debug!("Bootstrap RANDOM returning {} agents", agents.len());

    // Return as MessagePack array of raw agent info bytes
    let values: Vec<rmpv::Value> = agents.into_iter().map(rmpv::Value::Binary).collect();

    msgpack_response(&rmpv::Value::Array(values))
}

/// Handle NOW request - get server timestamp
pub async fn handle_now() -> Response<Full<Bytes>> {
    let now_ms = current_time_ms();
    debug!("Bootstrap NOW: {}", now_ms);
    msgpack_response(&rmpv::Value::Integer(now_ms.into()))
}

/// Get current time in milliseconds since UNIX epoch
fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Create a MessagePack response
fn msgpack_response(value: &rmpv::Value) -> Response<Full<Bytes>> {
    let mut buf = Vec::new();
    if let Err(e) = rmpv::encode::write_value(&mut buf, value) {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("MessagePack encode error: {e}"),
        );
    }

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/msgpack")
        .body(Full::new(Bytes::from(buf)))
        .unwrap()
}

/// Create an error response (MessagePack encoded error string)
fn error_response(status: StatusCode, message: &str) -> Response<Full<Bytes>> {
    let value = rmpv::Value::String(message.into());
    let mut buf = Vec::new();
    let _ = rmpv::encode::write_value(&mut buf, &value);

    Response::builder()
        .status(status)
        .header("Content-Type", "application/msgpack")
        .body(Full::new(Bytes::from(buf)))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_time_ms() {
        let now = current_time_ms();
        // Should be after year 2020 (roughly 1577836800000 ms)
        assert!(now > 1577836800000);
    }
}
