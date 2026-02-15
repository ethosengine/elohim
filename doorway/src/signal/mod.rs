//! Signal service for WebRTC signaling (SBD protocol)
//!
//! Implements the Holochain SBD (Signal Boot Discovery) protocol for WebRTC signaling.
//! Agents connect via WebSocket, authenticate by signing a nonce, and can then
//! relay messages to other connected agents by public key.
//!
//! Protocol:
//! - WS /signal/{pubkey} - WebSocket connection with claimed public key
//! - PUT /signal/authenticate - Get auth token (optional)
//!
//! SBD Commands (server -> client):
//! - `lbrt` - Rate limit (byte-nanos)
//! - `lidl` - Idle timeout (millis)
//! - `areq` - Auth request (32-byte nonce)
//! - `srdy` - Ready (authenticated, can relay)
//!
//! SBD Commands (client -> server):
//! - `keep` - Keepalive
//! - `ares` - Auth response (64-byte signature)
//!
//! Message forwarding:
//! - Client sends: [32-byte dest pubkey][payload]
//! - Server replaces header with sender's pubkey and forwards to dest
//!
//! ## Media Sessions (Extended)
//!
//! The `media` module provides structured WebRTC session management:
//! - `MediaOffer` / `MediaAnswer` for SDP exchange
//! - `IceCandidate` / `IceCandidateBatch` for ICE negotiation
//! - `MediaEnd` for graceful termination
//! - Session state tracking

mod cmd;
pub mod media;
mod store;

pub use media::{
    IceCandidate, MediaCmd, MediaEndReason, MediaQuality, MediaSession, MediaSessionState,
    MediaType,
};
pub use store::SignalStore;

use bytes::Bytes;
use ed25519_dalek::{Signature, VerifyingKey};
use futures_util::{SinkExt, StreamExt};
use http_body_util::Full;
use hyper::{Response, StatusCode};
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, info, warn};

use crate::config::Args;

/// Maximum message size (defined by SBD spec)
pub const MAX_MSG_BYTES: usize = 20_000;

/// Default rate limit in kbps per IP
pub const DEFAULT_RATE_LIMIT_KBPS: i32 = 1000;

/// Default idle timeout in milliseconds
pub const DEFAULT_IDLE_TIMEOUT_MS: i32 = 10_000;

/// Default max clients
pub const DEFAULT_MAX_CLIENTS: usize = 32768;

/// Public key wrapper (32 bytes, Ed25519)
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PubKey(pub Arc<[u8; 32]>);

impl PubKey {
    /// Verify a signature with this public key
    pub fn verify(&self, sig: &[u8; 64], data: &[u8]) -> bool {
        let Ok(key) = VerifyingKey::from_bytes(&self.0) else {
            return false;
        };
        let signature = Signature::from_bytes(sig);
        key.verify_strict(data, &signature).is_ok()
    }

    /// Encode as base64 URL-safe string
    pub fn to_base64(&self) -> String {
        use base64::prelude::*;
        BASE64_URL_SAFE_NO_PAD.encode(*self.0)
    }
}

impl std::fmt::Debug for PubKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PubKey({}...)", &self.to_base64()[..8])
    }
}

/// Convert IP address to canonical IPv6 form
pub fn to_canonical_ip(ip: IpAddr) -> Arc<Ipv6Addr> {
    Arc::new(match ip {
        IpAddr::V4(ip) => ip.to_ipv6_mapped(),
        IpAddr::V6(ip) => ip,
    })
}

/// Handle WebSocket upgrade for signal connection
pub async fn handle_signal_upgrade(
    store: Arc<SignalStore>,
    req: hyper::Request<hyper::body::Incoming>,
    pub_key_str: &str,
    addr: SocketAddr,
    args: &Args,
) -> Response<Full<Bytes>> {
    use base64::prelude::*;

    // Parse public key from URL
    let pk = match BASE64_URL_SAFE_NO_PAD.decode(pub_key_str) {
        Ok(pk) if pk.len() == 32 => {
            let mut sized_pk = [0u8; 32];
            sized_pk.copy_from_slice(&pk);
            PubKey(Arc::new(sized_pk))
        }
        _ => {
            warn!("Signal: invalid pubkey format from {}", addr);
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    r#"{"error": "Invalid public key format"}"#,
                )))
                .unwrap();
        }
    };

    // Check if pubkey starts with command prefix (illegal)
    if &pk.0[..28] == cmd::CMD_PREFIX {
        warn!("Signal: illegal pubkey (cmd prefix) from {}", addr);
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(r#"{"error": "Invalid public key"}"#)))
            .unwrap();
    }

    // Check if at capacity
    if store.is_at_capacity() {
        warn!("Signal: at capacity, rejecting {}", addr);
        return Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(r#"{"error": "Server at capacity"}"#)))
            .unwrap();
    }

    // Perform WebSocket upgrade
    let (response, websocket) = match hyper_tungstenite::upgrade(req, None) {
        Ok(upgrade) => upgrade,
        Err(e) => {
            warn!("Signal: WebSocket upgrade failed for {}: {}", addr, e);
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error": "WebSocket upgrade failed: {e}"}}"#
                ))))
                .unwrap();
        }
    };

    let calc_ip = to_canonical_ip(addr.ip());
    let idle_timeout_ms = args
        .signal_idle_timeout_ms
        .unwrap_or(DEFAULT_IDLE_TIMEOUT_MS);
    let rate_limit_kbps = args
        .signal_rate_limit_kbps
        .unwrap_or(DEFAULT_RATE_LIMIT_KBPS);

    // Spawn handler task
    tokio::spawn(async move {
        match websocket.await {
            Ok(ws) => {
                handle_signal_connection(store, ws, pk, calc_ip, idle_timeout_ms, rate_limit_kbps)
                    .await;
            }
            Err(e) => {
                warn!("Signal: WebSocket connection failed: {}", e);
            }
        }
    });

    response.map(|_| Full::new(Bytes::new()))
}

/// Handle an established signal WebSocket connection
async fn handle_signal_connection(
    store: Arc<SignalStore>,
    ws: hyper_tungstenite::WebSocketStream<hyper_util::rt::TokioIo<hyper::upgrade::Upgraded>>,
    pk: PubKey,
    ip: Arc<Ipv6Addr>,
    idle_timeout_ms: i32,
    rate_limit_kbps: i32,
) {
    let (write, read) = ws.split();
    let write = Arc::new(Mutex::new(write));
    let read = Arc::new(Mutex::new(read));

    info!("Signal: new connection from {:?}", pk);

    // Calculate byte-nanos rate limit
    let byte_nanos = 8_000_000 / rate_limit_kbps;

    // Send rate limit
    if send_message(&write, cmd::SbdCmd::limit_byte_nanos(byte_nanos))
        .await
        .is_err()
    {
        return;
    }

    // Send idle timeout
    if send_message(&write, cmd::SbdCmd::limit_idle_millis(idle_timeout_ms))
        .await
        .is_err()
    {
        return;
    }

    // Generate auth nonce
    let mut nonce = [0u8; 32];
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut nonce);

    // Send auth request
    if send_message(&write, cmd::SbdCmd::auth_req(&nonce))
        .await
        .is_err()
    {
        return;
    }

    // Wait for auth response with timeout
    let idle_dur = std::time::Duration::from_millis(idle_timeout_ms as u64);

    let auth_result = tokio::time::timeout(idle_dur, async {
        loop {
            let msg = {
                let mut read_guard = read.lock().await;
                match read_guard.next().await {
                    Some(Ok(msg)) => msg,
                    Some(Err(e)) => {
                        warn!("Signal: read error during auth: {}", e);
                        return Err(());
                    }
                    None => return Err(()),
                }
            };

            let data = match msg {
                Message::Binary(data) => data,
                Message::Close(_) => return Err(()),
                _ => continue,
            };

            match cmd::SbdCmd::parse(&data) {
                Ok(cmd::SbdCmd::AuthRes(sig)) => {
                    if pk.verify(&sig, &nonce) {
                        return Ok(());
                    } else {
                        warn!("Signal: invalid signature from {:?}", pk);
                        return Err(());
                    }
                }
                Ok(cmd::SbdCmd::Message(_)) => {
                    warn!("Signal: message before auth from {:?}", pk);
                    return Err(());
                }
                Ok(cmd::SbdCmd::Keepalive) => continue,
                Ok(cmd::SbdCmd::Unknown) => continue,
                Err(_) => return Err(()),
            }
        }
    })
    .await;

    if auth_result.is_err() || auth_result.unwrap().is_err() {
        debug!("Signal: auth failed for {:?}", pk);
        let _ = write.lock().await.close().await;
        return;
    }

    // Send ready
    if send_message(&write, cmd::SbdCmd::ready()).await.is_err() {
        return;
    }

    info!("Signal: authenticated {:?}", pk);

    // Register in store
    store.insert(pk.clone(), Arc::clone(&write), Arc::clone(&ip));

    // Message relay loop
    loop {
        let msg = tokio::time::timeout(idle_dur, async {
            let mut read_guard = read.lock().await;
            read_guard.next().await
        })
        .await;

        let msg = match msg {
            Ok(Some(Ok(msg))) => msg,
            _ => break,
        };

        let data = match msg {
            Message::Binary(data) => data,
            Message::Close(_) => break,
            Message::Ping(data) => {
                let _ = send_message(&write, Message::Pong(data).into_data()).await;
                continue;
            }
            _ => continue,
        };

        match cmd::SbdCmd::parse(&data) {
            Ok(cmd::SbdCmd::Keepalive) => continue,
            Ok(cmd::SbdCmd::AuthRes(_)) => break, // Invalid after auth
            Ok(cmd::SbdCmd::Unknown) => continue,
            Ok(cmd::SbdCmd::Message(mut payload)) => {
                // Forward message to destination
                if payload.len() < 32 {
                    continue;
                }

                // Extract destination pubkey
                let mut dest_pk = [0u8; 32];
                dest_pk.copy_from_slice(&payload[..32]);
                let dest = PubKey(Arc::new(dest_pk));

                // Replace header with sender's pubkey
                payload[..32].copy_from_slice(&pk.0[..]);

                // Forward to destination
                if let Some(dest_write) = store.get(&dest) {
                    let _ = send_message(&dest_write, payload).await;
                }
            }
            Err(_) => break,
        }
    }

    // Cleanup
    store.remove(&pk);
    let _ = write.lock().await.close().await;
    info!("Signal: disconnected {:?}", pk);
}

/// Send a binary message on the WebSocket
async fn send_message<S>(
    write: &Arc<Mutex<futures_util::stream::SplitSink<S, Message>>>,
    data: Vec<u8>,
) -> Result<(), ()>
where
    S: futures_util::Sink<Message> + Unpin,
{
    let mut guard = write.lock().await;
    guard.send(Message::Binary(data)).await.map_err(|_| ())
}
