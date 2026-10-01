//! Admin interface proxy
//!
//! Bidirectional WebSocket proxy between client and Holochain conductor admin
//! interface. EVERY client frame is filtered against the caller's permission
//! level — there is no passthrough mode.
//!
//! There used to be one, keyed on `dev_mode`. Because `DEV_MODE: "true"` is set
//! on every deployed manifest, that made this proxy forward raw, unfiltered
//! frames to the conductor admin interface for every caller on the fleet — and
//! the caller did not need a credential to get here, so `install_app`,
//! `uninstall_app` and `revoke_agent_key` were reachable anonymously from the
//! open internet. Filtering is now unconditional and defends in depth even if a
//! caller reaches this proxy with a level it should not have.

use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{http::Request, protocol::Message},
};
use tracing::{debug, error, info, warn};

use crate::auth::{is_operation_allowed, AdminCaller, PermissionLevel};
use crate::proxy::holochain::{encode_error, parse_message, ParsedMessage};
use crate::types::{DoorwayError, Result};

type HyperWebSocket =
    hyper_tungstenite::WebSocketStream<hyper_util::rt::TokioIo<hyper::upgrade::Upgraded>>;

/// Run the admin proxy between client and conductor
pub async fn run_proxy(
    client_ws: HyperWebSocket,
    conductor_url: &str,
    _origin: Option<String>,
    permission_level: PermissionLevel,
    caller: AdminCaller,
) -> Result<()> {
    info!(
        "Creating filtered admin proxy to {} (permission: {})",
        conductor_url, permission_level
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
        .map_err(|e| DoorwayError::Holochain(format!("Failed to build request: {e}")))?;

    let (conductor_ws, _) = connect_async_with_config(request, None, false)
        .await
        .map_err(|e| DoorwayError::Holochain(format!("Failed to connect to conductor: {e}")))?;

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
                    // Parse message and check permissions
                    match filter_message(&data, permission_level, &caller) {
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
                            // Unparseable frame. Never forwarded: an operation we
                            // cannot name is an operation we cannot authorize.
                            warn!("Blocking unparseable admin frame");
                            let error_response = encode_error("Invalid message format");
                            let mut sink = client_sink_for_client.lock().await;
                            if let Err(e) = sink.send(Message::Binary(error_response)).await {
                                error!("Failed to send error to client: {}", e);
                                break;
                            }
                        }
                    }
                }
                Ok(Message::Text(_)) => {
                    // Holochain admin speaks binary MessagePack only. A text frame
                    // cannot be parsed into an operation, so it cannot be
                    // authorized — never forwarded, same refusal as an
                    // unparseable binary frame.
                    warn!("Blocking text admin frame");
                    let error_response = encode_error("Invalid message format");
                    let mut sink = client_sink_for_client.lock().await;
                    if let Err(e) = sink.send(Message::Binary(error_response)).await {
                        error!("Failed to send error to client: {}", e);
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

/// Filter a message based on permission level and the caller's own scope
fn filter_message(
    data: &[u8],
    permission_level: PermissionLevel,
    caller: &AdminCaller,
) -> FilterResult {
    match parse_message(data) {
        Ok(parsed) => match authorize_admin_request(&parsed, permission_level, caller) {
            Ok(()) => {
                debug!(
                    "Allowing operation '{}' for permission level {}",
                    parsed.operation, permission_level
                );
                FilterResult::Allow
            }
            Err(reason) => FilterResult::Deny(reason),
        },
        Err(e) => {
            debug!("Failed to parse message: {}", e);
            FilterResult::PassThrough
        }
    }
}

/// Operations whose payload names a cell (`cell_id: [dna_hash, agent_pub_key]`).
const CELL_BEARING_OPERATIONS: &[&str] = &[
    "grant_zome_call_capability",
    "revoke_zome_call_capability",
    "dump_state",
    "dump_full_state",
    "dump_op_timings",
];

/// Operations whose payload names an installed app (`installed_app_id`).
const APP_BEARING_OPERATIONS: &[&str] =
    &["issue_app_authentication_token", "list_capability_grants"];

/// Authorize one parsed admin request: the operation must be allowed at the
/// caller's level (name check), and below `Admin` a cell- or app-bearing
/// operation must target the caller's OWN cell/app. `Admin` is unchanged.
///
/// Fails closed: if the caller's identity is unknown or the cell/app cannot be
/// read from the payload, the request is refused.
pub(crate) fn authorize_admin_request(
    parsed: &ParsedMessage,
    permission_level: PermissionLevel,
    caller: &AdminCaller,
) -> std::result::Result<(), String> {
    let operation = parsed.operation.as_str();
    if !is_operation_allowed(operation, permission_level) {
        return Err(format!(
            "Operation '{operation}' requires higher permission level (current: {permission_level})"
        ));
    }
    if permission_level >= PermissionLevel::Admin {
        return Ok(());
    }

    if CELL_BEARING_OPERATIONS.contains(&operation) {
        let Some(own_agent) = caller.agent.as_deref() else {
            return Err(format!(
                "Operation '{operation}' requires a caller bound to an agent key"
            ));
        };
        return match cell_agent(&parsed.data) {
            Some(agent) if agent == own_agent => Ok(()),
            Some(_) => Err(format!(
                "Operation '{operation}' may only target the caller's own cell"
            )),
            None => Err(format!(
                "Operation '{operation}' refused: cell_id could not be determined"
            )),
        };
    }

    if APP_BEARING_OPERATIONS.contains(&operation) {
        let Some(own_app) = caller.installed_app_id.as_deref() else {
            return Err(format!(
                "Operation '{operation}' requires a caller bound to an installed app"
            ));
        };
        return match single_string_field(&parsed.data, "installed_app_id") {
            Some(app) if app == own_app => Ok(()),
            Some(_) => Err(format!(
                "Operation '{operation}' may only target the caller's own app"
            )),
            None => Err(format!(
                "Operation '{operation}' refused: installed_app_id could not be determined"
            )),
        };
    }

    Ok(())
}

/// The raw 39-byte agent key of the payload's `cell_id` (`[dna_hash, agent]`).
/// `None` unless the payload has exactly one well-formed `cell_id`.
fn cell_agent(data: &rmpv::Value) -> Option<Vec<u8>> {
    let rmpv::Value::Map(map) = data else {
        return None;
    };
    let mut found = map
        .iter()
        .filter(|(k, _)| k.as_str() == Some("cell_id"))
        .map(|(_, v)| v);
    let cell_id = found.next()?;
    if found.next().is_some() {
        return None;
    }
    let rmpv::Value::Array(parts) = cell_id else {
        return None;
    };
    match parts.as_slice() {
        [rmpv::Value::Binary(_dna), rmpv::Value::Binary(agent)] if agent.len() == 39 => {
            Some(agent.clone())
        }
        _ => None,
    }
}

/// A string field that appears exactly once in the payload map.
fn single_string_field(data: &rmpv::Value, key: &str) -> Option<String> {
    let rmpv::Value::Map(map) = data else {
        return None;
    };
    let mut found = map
        .iter()
        .filter(|(k, _)| k.as_str() == Some(key))
        .map(|(_, v)| v);
    let value = found.next()?;
    if found.next().is_some() {
        return None;
    }
    value.as_str().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmpv::Value;

    const OWN: [u8; 39] = [7; 39];
    const OTHER: [u8; 39] = [9; 39];

    fn s(v: &str) -> Value {
        Value::String(v.into())
    }

    /// `{ type: "request", id, data: <msgpack { type: op, data }> }` — the
    /// double-encoded shape @holochain/client sends.
    fn frame(op: &str, data: Value) -> Vec<u8> {
        let inner = Value::Map(vec![(s("type"), s(op)), (s("data"), data)]);
        let mut inner_bytes = Vec::new();
        rmpv::encode::write_value(&mut inner_bytes, &inner).unwrap();
        let outer = Value::Map(vec![
            (s("id"), Value::from(1)),
            (s("type"), s("request")),
            (s("data"), Value::Binary(inner_bytes)),
        ]);
        let mut out = Vec::new();
        rmpv::encode::write_value(&mut out, &outer).unwrap();
        out
    }

    fn cell_payload(agent: &[u8]) -> Value {
        Value::Map(vec![(
            s("cell_id"),
            Value::Array(vec![
                Value::Binary(vec![1; 39]),
                Value::Binary(agent.to_vec()),
            ]),
        )])
    }

    fn caller() -> AdminCaller {
        AdminCaller {
            agent: Some(OWN.to_vec()),
            installed_app_id: Some("elohim-alice".into()),
        }
    }

    fn allowed(op: &str, data: Value, level: PermissionLevel, who: &AdminCaller) -> bool {
        matches!(
            filter_message(&frame(op, data), level, who),
            FilterResult::Allow
        )
    }

    #[test]
    fn authenticated_cannot_grant_on_a_foreign_cell() {
        let who = caller();
        assert!(!allowed(
            "grant_zome_call_capability",
            cell_payload(&OTHER),
            PermissionLevel::Authenticated,
            &who
        ));
        assert!(allowed(
            "grant_zome_call_capability",
            cell_payload(&OWN),
            PermissionLevel::Authenticated,
            &who
        ));
    }

    #[test]
    fn authenticated_cannot_dump_full_state_of_a_foreign_cell() {
        let who = caller();
        for op in [
            "dump_state",
            "dump_full_state",
            "revoke_zome_call_capability",
        ] {
            assert!(!allowed(
                op,
                cell_payload(&OTHER),
                PermissionLevel::Authenticated,
                &who
            ));
            assert!(allowed(
                op,
                cell_payload(&OWN),
                PermissionLevel::Authenticated,
                &who
            ));
        }
    }

    #[test]
    fn admin_may_target_any_cell() {
        let who = AdminCaller::default();
        assert!(allowed(
            "grant_zome_call_capability",
            cell_payload(&OTHER),
            PermissionLevel::Admin,
            &who
        ));
        assert!(allowed(
            "dump_full_state",
            cell_payload(&OTHER),
            PermissionLevel::Admin,
            &who
        ));
    }

    #[test]
    fn undeterminable_cell_is_refused_at_authenticated() {
        let who = caller();
        let malformed = [
            Value::Nil,
            Value::Map(vec![]),
            Value::Map(vec![(s("cell_id"), s("not-a-cell"))]),
            // Agent half is the wrong length.
            cell_payload(&[7; 32]),
            // Two cell_id keys: parsers could disagree about which one wins.
            Value::Map(vec![
                (
                    s("cell_id"),
                    Value::Array(vec![
                        Value::Binary(vec![1; 39]),
                        Value::Binary(OWN.to_vec()),
                    ]),
                ),
                (
                    s("cell_id"),
                    Value::Array(vec![
                        Value::Binary(vec![1; 39]),
                        Value::Binary(OTHER.to_vec()),
                    ]),
                ),
            ]),
        ];
        for data in malformed {
            assert!(!allowed(
                "grant_zome_call_capability",
                data,
                PermissionLevel::Authenticated,
                &who
            ));
        }
    }

    #[test]
    fn caller_without_an_agent_fails_closed_on_cell_operations() {
        // An API-key caller binds no agent.
        let who = AdminCaller::default();
        assert!(!allowed(
            "dump_state",
            cell_payload(&OWN),
            PermissionLevel::Authenticated,
            &who
        ));
    }

    #[test]
    fn app_bearing_operations_are_scoped_to_the_callers_app() {
        let who = caller();
        let app = |id: &str| Value::Map(vec![(s("installed_app_id"), s(id))]);
        for op in ["issue_app_authentication_token", "list_capability_grants"] {
            assert!(allowed(
                op,
                app("elohim-alice"),
                PermissionLevel::Authenticated,
                &who
            ));
            assert!(!allowed(
                op,
                app("elohim-bob"),
                PermissionLevel::Authenticated,
                &who
            ));
            assert!(!allowed(
                op,
                Value::Nil,
                PermissionLevel::Authenticated,
                &who
            ));
            assert!(!allowed(
                op,
                app("elohim-alice"),
                PermissionLevel::Authenticated,
                &AdminCaller::default()
            ));
            assert!(allowed(
                op,
                app("elohim-bob"),
                PermissionLevel::Admin,
                &AdminCaller::default()
            ));
        }
    }

    #[test]
    fn name_check_still_applies_before_scope() {
        let who = caller();
        assert!(!allowed(
            "install_app",
            Value::Nil,
            PermissionLevel::Authenticated,
            &who
        ));
        assert!(!allowed(
            "grant_zome_call_capability",
            cell_payload(&OWN),
            PermissionLevel::Public,
            &who
        ));
    }

    #[test]
    fn unscoped_authenticated_operations_are_unchanged() {
        let who = AdminCaller::default();
        assert!(allowed(
            "generate_agent_pub_key",
            Value::Nil,
            PermissionLevel::Authenticated,
            &who
        ));
        assert!(allowed(
            "list_apps",
            Value::Nil,
            PermissionLevel::Public,
            &who
        ));
    }

    #[test]
    fn unparseable_frame_is_not_allowed() {
        assert!(matches!(
            filter_message(&[0xc1, 0x00], PermissionLevel::Admin, &caller()),
            FilterResult::PassThrough
        ));
    }
}
