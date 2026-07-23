//! Conductor connection manager
//!
//! Maintains a persistent WebSocket connection to the Holochain conductor.
//! Handles reconnection and provides a thread-safe interface for sending requests.
//!
//! Reconnect discipline (the 2026-06-10 doorway→conductor storm fix):
//! - The reconnect backoff only resets after a session survives
//!   [`STABLE_SESSION_THRESHOLD`] — a conductor that accepts the WebSocket and
//!   then drops it (the auth-reject signature) escalates instead of pinning
//!   the delay at base. Without this, every auth-rejecting conductor was
//!   reconnected at ~10Hz forever.
//! - The connection loop exits when its owning [`ConductorConnection`] handle
//!   is dropped. Detached loops used to outlive their handles and reconnect
//!   forever, so every pool-worker reconnect leaked one immortal spammer.
//! - When the conductor REJECTS the app-auth token — a Close/transport error
//!   inside the [`AUTH_ACK_WINDOW`], or a Close frame ending a short session —
//!   the loop re-mints via the injected [`TokenMinter`] (rate-limited): a
//!   conductor restart invalidates previously issued tokens, and without
//!   re-minting the pool stayed broken until the doorway itself was restarted.
//!   A transient stall (transport blip, GC pause) is NOT treated as a rejection,
//!   so it can no longer amplify into a mint/reconnect storm. Authentication is
//!   confirmed by *watching for a rejection* over the official wire encoding —
//!   never by optimistically logging success after a blind sleep.

use futures_util::future::BoxFuture;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};
use tokio::time::timeout;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{http::Request, protocol::Message},
};
use tracing::{debug, error, info, warn};

use crate::types::{DoorwayError, Result};

/// Async closure that mints a fresh app-auth token (an admin-interface
/// round-trip). Injected so this low-level module stays independent of the
/// typed admin client.
pub type TokenMinter = Arc<dyn Fn() -> BoxFuture<'static, Option<Vec<u8>>> + Send + Sync>;

/// Base reconnect delay.
const BASE_RECONNECT_DELAY: Duration = Duration::from_millis(100);

/// Ceiling for the reconnect delay.
const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(30);

/// A session must survive this long before the backoff ladder resets.
/// Auth-rejected connections complete the WebSocket handshake and then get
/// dropped within milliseconds — those must escalate, not reset.
const STABLE_SESSION_THRESHOLD: Duration = Duration::from_secs(10);

/// Minimum interval between token re-mint attempts.
const REMINT_MIN_INTERVAL: Duration = Duration::from_secs(30);

/// How long to watch the conductor socket for an auth rejection after sending
/// the authenticate frame. Holochain's app-interface auth is fire-and-forget on
/// success (no positive ack); a rejection surfaces as a Close frame, a transport
/// error, or stream-end. So a terminal frame inside this window is auth FAILURE,
/// while the window elapsing clean is success. Tunable: widen it if a conductor's
/// reject latency exceeds it (a stale token then self-heals one reconnect later
/// via the re-mint path). Replaces the old blind `sleep(50ms)` that logged
/// success optimistically and produced the ~10s reconnect/re-mint storm.
const AUTH_ACK_WINDOW: Duration = Duration::from_millis(500);

/// Exponential reconnect backoff that only resets after a stable session.
struct ReconnectBackoff {
    delay: Duration,
}

impl ReconnectBackoff {
    fn new() -> Self {
        Self {
            delay: BASE_RECONNECT_DELAY,
        }
    }

    /// Next delay after a connect attempt that never produced a session.
    fn next_after_connect_failure(&mut self) -> Duration {
        let current = self.delay;
        self.delay = (self.delay * 2).min(MAX_RECONNECT_DELAY);
        current
    }

    /// Next delay after a session ended. Stable sessions reset the ladder;
    /// unstable ones (accept-then-drop) escalate exactly like failures.
    fn next_after_session(&mut self, session_len: Duration) -> Duration {
        if session_len >= STABLE_SESSION_THRESHOLD {
            self.delay = BASE_RECONNECT_DELAY;
        }
        self.next_after_connect_failure()
    }
}

/// Why a connection session ended.
enum SessionEnd {
    /// All request senders dropped — the owning handle is gone, shut down.
    ChannelClosed,
    /// The WebSocket closed or errored — reconnect. Carries the close *cause*
    /// (one of `metrics::REASON_*`) and the optional WS close code, so the
    /// reconnect counter classifies churn (T3 connect-storm vs T8 auth-reject).
    ConnectionClosed {
        reason: &'static str,
        close_code: Option<u16>,
    },
}

/// Conductor connection manager
pub struct ConductorConnection {
    /// URL of the conductor
    #[allow(dead_code)]
    conductor_url: String,
    /// Channel for sending messages to the conductor
    tx: mpsc::Sender<(Vec<u8>, oneshot::Sender<Vec<u8>>)>,
    /// Whether the connection is alive
    connected: Arc<RwLock<bool>>,
}

impl ConductorConnection {
    /// Create a new conductor connection (no authentication)
    pub async fn connect(conductor_url: &str) -> Result<Self> {
        Self::connect_with_auth(conductor_url, None).await
    }

    /// Create a new conductor connection with optional app authentication token.
    ///
    /// If `auth_token` is provided, the connection sends an `authenticate` message
    /// after each WebSocket connect (including reconnects), matching the Holochain 0.6
    /// app interface protocol used by elohim-storage.
    ///
    /// Waits up to 5s for the initial connection. On timeout the returned error
    /// drops the only handle, which shuts the background loop down cleanly.
    pub async fn connect_with_auth(
        conductor_url: &str,
        auth_token: Option<Vec<u8>>,
    ) -> Result<Self> {
        Self::connect_with_auth_minter(conductor_url, auth_token, None).await
    }

    /// Like [`Self::connect_with_auth`] but with a token minter, so the
    /// connection self-heals after a conductor restart invalidates the
    /// initially issued token.
    pub async fn connect_with_auth_minter(
        conductor_url: &str,
        auth_token: Option<Vec<u8>>,
        token_minter: Option<TokenMinter>,
    ) -> Result<Self> {
        let conn = Self::spawn_with_auth_minter(conductor_url, auth_token, token_minter);

        // Wait for initial connection
        for _ in 0..50 {
            if *conn.connected.read().await {
                return Ok(conn);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Err(DoorwayError::Holochain(
            "Timeout waiting for conductor connection".into(),
        ))
    }

    /// Spawn a conductor connection without waiting for it to come up.
    ///
    /// The background loop owns all reconnection (with stability-gated
    /// exponential backoff) for the lifetime of the returned handle and exits
    /// when the handle is dropped. This is the constructor pool workers use:
    /// create once, never recreate — recreating on failure is how connection
    /// loops used to leak.
    pub fn spawn_with_auth_minter(
        conductor_url: &str,
        auth_token: Option<Vec<u8>>,
        token_minter: Option<TokenMinter>,
    ) -> Self {
        let (tx, rx) = mpsc::channel::<(Vec<u8>, oneshot::Sender<Vec<u8>>)>(1000);
        let connected = Arc::new(RwLock::new(false));

        let conn = Self {
            conductor_url: conductor_url.to_string(),
            tx,
            connected: Arc::clone(&connected),
        };

        let url = conductor_url.to_string();
        let connected_flag = Arc::clone(&connected);
        let token_slot = Arc::new(RwLock::new(auth_token));
        tokio::spawn(async move {
            connection_loop(url, token_slot, token_minter, rx, connected_flag).await;
        });

        conn
    }

    /// Send a request to the conductor and wait for response
    pub async fn request(&self, data: Vec<u8>, timeout_ms: u64) -> Result<Vec<u8>> {
        let (response_tx, response_rx) = oneshot::channel();

        self.tx
            .send((data, response_tx))
            .await
            .map_err(|_| DoorwayError::Holochain("Conductor connection closed".into()))?;

        match timeout(Duration::from_millis(timeout_ms), response_rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(DoorwayError::Holochain("Response channel closed".into())),
            Err(_) => Err(DoorwayError::Holochain("Request timeout".into())),
        }
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }
}

/// Main connection loop with reconnection logic.
///
/// Runs until every [`ConductorConnection`] handle for this loop is dropped
/// (the request channel closes). Backoff escalates on connect failures AND on
/// unstable sessions; it only resets after a session survives
/// [`STABLE_SESSION_THRESHOLD`].
async fn connection_loop(
    conductor_url: String,
    token_slot: Arc<RwLock<Option<Vec<u8>>>>,
    token_minter: Option<TokenMinter>,
    mut rx: mpsc::Receiver<(Vec<u8>, oneshot::Sender<Vec<u8>>)>,
    connected: Arc<RwLock<bool>>,
) {
    let mut backoff = ReconnectBackoff::new();
    let mut last_remint: Option<Instant> = None;

    loop {
        // The owning handle(s) dropped while we were disconnected — shut down
        // instead of reconnecting on behalf of nobody.
        if rx.is_closed() {
            info!(
                "Conductor connection handle dropped; stopping connection loop for {}",
                conductor_url
            );
            return;
        }

        info!("Connecting to conductor at {}", conductor_url);

        let reconnect_delay = match connect_to_conductor(&conductor_url).await {
            Ok((mut ws_sink, mut ws_stream)) => {
                // Authenticate if a token is configured (Holochain app interface).
                let token = token_slot.read().await.clone();
                if let Some(ref token) = token {
                    match authenticate(&mut ws_sink, &mut ws_stream, token).await {
                        Ok(buffered) => {
                            // Confirmed: the ack window elapsed with no rejection.
                            debug!("Authenticated with conductor");
                            run_session(
                                ws_sink,
                                ws_stream,
                                buffered,
                                &mut rx,
                                &connected,
                                &mut backoff,
                                &token_slot,
                                &token_minter,
                                &mut last_remint,
                            )
                            .await
                        }
                        Err(e) => {
                            error!("Failed to authenticate with conductor: {}", e);
                            *connected.write().await = false;
                            // A rejection inside the auth window is the
                            // stale/invalid-token signature (a conductor restart
                            // invalidates issued tokens): re-mint (rate-limited)
                            // so the next reconnect uses a fresh token, and
                            // escalate the backoff like any unstable session.
                            crate::metrics::inc_reconnect(crate::metrics::REASON_WS_ERROR);
                            remint_if_due(&token_minter, &token_slot, &mut last_remint).await;
                            Some(backoff.next_after_connect_failure())
                        }
                    }
                } else {
                    run_session(
                        ws_sink,
                        ws_stream,
                        Vec::new(),
                        &mut rx,
                        &connected,
                        &mut backoff,
                        &token_slot,
                        &token_minter,
                        &mut last_remint,
                    )
                    .await
                }
            }
            Err(e) => {
                error!("Failed to connect to conductor: {}", e);
                crate::metrics::inc_reconnect(crate::metrics::REASON_CONNECT_REFUSED);
                Some(backoff.next_after_connect_failure())
            }
        };

        let Some(reconnect_delay) = reconnect_delay else {
            // Session ended because the handle was dropped — shut down.
            info!(
                "Conductor connection handle dropped; stopping connection loop for {}",
                conductor_url
            );
            return;
        };

        // Wait before reconnecting
        warn!("Reconnecting to conductor in {:?}...", reconnect_delay);
        tokio::time::sleep(reconnect_delay).await;
    }
}

/// Run one connected session to completion and compute the next reconnect
/// delay. Returns `None` when the owning handle dropped (shut down).
#[allow(clippy::too_many_arguments)]
async fn run_session(
    ws_sink: WsSink,
    ws_stream: WsStream,
    buffered: Vec<Message>,
    rx: &mut mpsc::Receiver<(Vec<u8>, oneshot::Sender<Vec<u8>>)>,
    connected: &Arc<RwLock<bool>>,
    backoff: &mut ReconnectBackoff,
    token_slot: &Arc<RwLock<Option<Vec<u8>>>>,
    token_minter: &Option<TokenMinter>,
    last_remint: &mut Option<Instant>,
) -> Option<Duration> {
    *connected.write().await = true;
    crate::metrics::inc_sessions(); // M5: live pool-worker session begins.
    info!("Connected to conductor");
    let session_start = Instant::now();

    let session_end = handle_messages(ws_sink, ws_stream, buffered, rx).await;

    *connected.write().await = false;
    crate::metrics::dec_sessions(); // M5: session ended.
    let session_len = session_start.elapsed();
    // M3: observe every session's lifetime — a pile in the <1s buckets is the
    // accept-then-drop/auth-reject signature; a pile at the long end is healthy.
    crate::metrics::observe_session_duration(session_len.as_secs_f64());

    // M2: classify the session ending for the reconnect counter (+ close code).
    match &session_end {
        SessionEnd::ChannelClosed => {
            crate::metrics::inc_reconnect(crate::metrics::REASON_CHANNEL_CLOSED);
        }
        SessionEnd::ConnectionClosed { reason, close_code } => {
            crate::metrics::inc_reconnect(reason);
            if let Some(code) = close_code {
                crate::metrics::inc_close_code(*code);
            }
        }
    }

    if matches!(session_end, SessionEnd::ChannelClosed) {
        return None;
    }

    // Re-mint ONLY on a confirmed auth-reject signal: a Close FRAME ending a
    // short session is a rejection that landed AFTER the `authenticate` ack
    // window (a slow conductor) — re-mint (rate-limited) so a token invalidated
    // by a restart self-heals. A transport error / stream-end is a transient
    // stall, NOT an auth signal: it escalates the backoff below but must not
    // amplify into a mint/reconnect storm. The fast path (rejection inside the
    // window) already re-minted in `connection_loop`.
    if session_len < STABLE_SESSION_THRESHOLD
        && matches!(
            session_end,
            SessionEnd::ConnectionClosed { reason, .. }
                if reason == crate::metrics::REASON_CLOSE_FRAME
        )
    {
        remint_if_due(token_minter, token_slot, last_remint).await;
    }

    Some(backoff.next_after_session(session_len))
}

/// Re-mint the app auth token if a minter is configured and the rate limit
/// allows. Updates `token_slot` in place on success.
async fn remint_if_due(
    token_minter: &Option<TokenMinter>,
    token_slot: &Arc<RwLock<Option<Vec<u8>>>>,
    last_remint: &mut Option<Instant>,
) {
    let Some(minter) = token_minter else {
        return;
    };
    let due = last_remint.is_none_or(|t| t.elapsed() >= REMINT_MIN_INTERVAL);
    if !due {
        return;
    }
    *last_remint = Some(Instant::now());
    if let Some(new_token) = minter().await {
        info!("Re-minted app auth token after unstable conductor session");
        *token_slot.write().await = Some(new_token);
    } else {
        warn!("Token re-mint failed after unstable conductor session (will retry)");
    }
}

/// Build the official Holochain app-interface authenticate frame for `token`.
///
/// Byte-identical to what `projection::subscriber::send_auth_request` sends:
/// `WireMessage::Authenticate { data: AppAuthenticationRequest { token } }`,
/// serialized via `holochain_serialized_bytes` — NOT a hand-rolled rmpv
/// `{type:"authenticate", data:{token}}` map. The conductor never recognized
/// that legacy shape, so it left every pool worker unauthenticated and dropped
/// the socket at its ~10s auth-timeout: the optimistic-auth reconnect storm.
fn encode_authenticate(token: &[u8]) -> Result<Vec<u8>> {
    use holochain_conductor_api::AppAuthenticationRequest;
    use holochain_serialized_bytes::prelude::*;
    use holochain_websocket::WireMessage;

    let auth_request = AppAuthenticationRequest {
        token: token.to_vec(),
    };
    let inner: SerializedBytes = auth_request.try_into().map_err(|e: SerializedBytesError| {
        DoorwayError::Holochain(format!("Failed to serialize auth request: {e}"))
    })?;
    let wire_msg = WireMessage::Authenticate {
        data: UnsafeBytes::from(inner).into(),
    };
    let outer: SerializedBytes = wire_msg.try_into().map_err(|e: SerializedBytesError| {
        DoorwayError::Holochain(format!("Failed to serialize wire message: {e}"))
    })?;
    Ok(outer.bytes().to_vec())
}

/// Send the authenticate frame, then watch the socket for a rejection.
///
/// Holochain's app-interface authenticate is fire-and-forget on success: the
/// conductor only reacts to a *bad* auth by closing the socket. So we send the
/// official frame ([`encode_authenticate`]) and read from the stream for up to
/// [`AUTH_ACK_WINDOW`]:
/// - a Close frame, a transport error, or stream-end in-window => auth FAILURE
///   (return `Err` — the caller re-mints and escalates backoff);
/// - the window elapsing with no terminal frame => success.
///
/// This replaces the old blind `sleep(50ms); Ok(())` that logged success
/// optimistically even as the conductor was about to drop the socket.
///
/// Any non-terminal frames observed during the window (a Ping to answer, an
/// early signal) are returned so [`run_session`] / [`handle_messages`] can
/// replay them — buffered conductor messages are never dropped.
async fn authenticate(
    ws_sink: &mut WsSink,
    ws_stream: &mut WsStream,
    token: &[u8],
) -> Result<Vec<Message>> {
    let frame = encode_authenticate(token)?;
    ws_sink
        .send(Message::Binary(frame))
        .await
        .map_err(|e| DoorwayError::Holochain(format!("Failed to send auth: {e}")))?;
    debug!("Sent authentication request to conductor");

    let deadline = Instant::now() + AUTH_ACK_WINDOW;
    let mut buffered: Vec<Message> = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            // Window elapsed with no rejection — as confirmed as fire-and-forget
            // auth gets.
            return Ok(buffered);
        }
        match timeout(remaining, ws_stream.next()).await {
            // Window expired mid-read — success.
            Err(_elapsed) => return Ok(buffered),
            // Conductor closed the socket — auth rejection.
            Ok(Some(Ok(Message::Close(_)))) => {
                return Err(DoorwayError::Holochain(
                    "Conductor closed connection during authentication".into(),
                ));
            }
            // Transport error — the socket died under us; treat as auth failure.
            Ok(Some(Err(e))) => {
                return Err(DoorwayError::Holochain(format!(
                    "WebSocket error during authentication: {e}"
                )));
            }
            // Stream ended — the conductor went away during auth.
            Ok(None) => {
                return Err(DoorwayError::Holochain(
                    "Conductor stream ended during authentication".into(),
                ));
            }
            // A non-terminal frame (signal/ping): buffer it and keep watching —
            // a single non-close frame does not prove success (a Close may
            // still follow inside the window).
            Ok(Some(Ok(frame))) => buffered.push(frame),
        }
    }
}

type WsSink = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    Message,
>;

type WsStream = futures_util::stream::SplitStream<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
>;

/// Connect to conductor with proper headers
async fn connect_to_conductor(url: &str) -> Result<(WsSink, WsStream)> {
    let request = Request::builder()
        .uri(url)
        .header("Host", url.split("//").last().unwrap_or("localhost"))
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

    let (ws, _) = connect_async_with_config(request, None, false)
        .await
        .map_err(|e| DoorwayError::Holochain(format!("WebSocket connect failed: {e}")))?;

    Ok(ws.split())
}

/// Extract the request ID from a MessagePack envelope.
///
/// Holochain response envelopes have `{ id: <u64>, type: "response"|"error", data: ... }`.
/// Signal messages have `{ type: "signal", data: ... }` with no `id` field.
fn extract_message_id(data: &[u8]) -> Option<u64> {
    let mut cursor = Cursor::new(data);
    let value = rmpv::decode::read_value(&mut cursor).ok()?;
    if let rmpv::Value::Map(ref map) = value {
        for (k, v) in map {
            if let rmpv::Value::String(ref key) = k {
                if key.as_str() == Some("id") {
                    if let rmpv::Value::Integer(ref id) = v {
                        return id.as_u64();
                    }
                }
            }
        }
    }
    None
}

/// Handle messages between request channel and conductor WebSocket.
///
/// Uses ID-based matching: each outgoing request has an `id` field, and
/// Holochain echoes that `id` in the response. Unsolicited messages
/// (signals) have no `id` and are safely skipped.
///
/// Returns WHY the session ended so the caller can distinguish "owner went
/// away" (shut down) from "conductor went away" (reconnect).
async fn handle_messages(
    ws_sink: WsSink,
    mut ws_stream: WsStream,
    buffered: Vec<Message>,
    rx: &mut mpsc::Receiver<(Vec<u8>, oneshot::Sender<Vec<u8>>)>,
) -> SessionEnd {
    // Pending responses keyed by request ID
    let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Vec<u8>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let pending_for_send = Arc::clone(&pending);

    // Monotonic request ID counter
    let next_id = Arc::new(std::sync::atomic::AtomicU64::new(1));

    // Wrap sink in Arc<Mutex> for sharing
    let ws_sink = Arc::new(Mutex::new(ws_sink));
    let ws_sink_for_rx = Arc::clone(&ws_sink);

    // Replay any frames buffered during the auth ack window so none are dropped.
    // `pending` is empty at session start, so a buffered Binary is an
    // unsolicited signal (skipped, matching the no-id path in the response
    // handler below); a buffered Ping is answered with a Pong.
    for msg in buffered {
        match msg {
            Message::Ping(data) => {
                let mut sink = ws_sink.lock().await;
                let _ = sink.send(Message::Pong(data)).await;
            }
            Message::Binary(_) => {
                debug!("Replayed conductor signal from auth window (skipping)");
            }
            _ => {}
        }
    }

    // Task to handle incoming requests
    let request_handler = async {
        loop {
            let Some((data, response_tx)) = rx.recv().await else {
                // All senders dropped — the owning handle is gone.
                break SessionEnd::ChannelClosed;
            };

            // The caller already embeds an `id` in the envelope.
            // Extract it so we can register the pending response under that ID.
            let req_id = extract_message_id(&data).unwrap_or_else(|| {
                // Fallback: assign our own ID (shouldn't happen with well-formed envelopes)
                next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            });

            // Register pending response
            {
                let mut pending = pending_for_send.lock().await;
                pending.insert(req_id, response_tx);
            }

            // Send to conductor
            let mut sink = ws_sink_for_rx.lock().await;
            if let Err(e) = sink.send(Message::Binary(data)).await {
                error!("Failed to send to conductor: {}", e);
                let mut pending = pending_for_send.lock().await;
                pending.remove(&req_id);
                break SessionEnd::ConnectionClosed {
                    reason: crate::metrics::REASON_WS_ERROR,
                    close_code: None,
                };
            }
        }
    };

    // Task to handle responses from conductor
    let response_handler = async {
        // Track WHY the stream ended so the reconnect counter can classify it.
        // Default = stream ended (None) with no Close/Err frame.
        let mut reason = crate::metrics::REASON_WS_ERROR;
        let mut close_code: Option<u16> = None;
        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    let data_vec = data.to_vec();
                    match extract_message_id(&data_vec) {
                        Some(resp_id) => {
                            // Response with matching ID — deliver to caller
                            let maybe_sender = {
                                let mut pending = pending.lock().await;
                                pending.remove(&resp_id)
                            };

                            if let Some(sender) = maybe_sender {
                                let _ = sender.send(data_vec);
                            } else {
                                debug!(
                                    id = resp_id,
                                    "Response for unknown request ID (already timed out?)"
                                );
                            }
                        }
                        None => {
                            // No ID = signal or other unsolicited message — skip
                            debug!(len = data_vec.len(), "Received conductor signal (skipping)");
                        }
                    }
                }
                Ok(Message::Ping(data)) => {
                    let mut sink = ws_sink.lock().await;
                    let _ = sink.send(Message::Pong(data)).await;
                }
                Ok(Message::Close(frame)) => {
                    // Capture the close code BEFORE logging — the worker path
                    // kept the frame; we now also classify it (handoff §M2).
                    close_code = frame.as_ref().map(|f| u16::from(f.code));
                    info!("Conductor closed connection: {:?}", frame);
                    reason = crate::metrics::REASON_CLOSE_FRAME;
                    break;
                }
                Err(e) => {
                    error!("Conductor WebSocket error: {}", e);
                    reason = crate::metrics::REASON_WS_ERROR;
                    break;
                }
                _ => {}
            }
        }
        SessionEnd::ConnectionClosed { reason, close_code }
    };

    // Run both handlers concurrently
    tokio::select! {
        end = request_handler => {
            debug!("Request handler ended");
            end
        }
        end = response_handler => {
            debug!("Response handler ended");
            end
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::net::TcpListener;

    /// How a test WebSocket server treats accepted connections.
    #[derive(Clone, Copy)]
    enum ServerBehavior {
        /// Complete the WS handshake and keep the connection open.
        Hold,
        /// Complete the WS handshake, then immediately drop the connection
        /// (mimics a conductor rejecting an unauthenticated/stale-token client).
        DropImmediately,
    }

    /// Minimal WS server that counts completed handshakes.
    async fn ws_test_server(behavior: ServerBehavior) -> (SocketAddr, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let count_for_server = Arc::clone(&count);

        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                let count = Arc::clone(&count_for_server);
                tokio::spawn(async move {
                    if let Ok(mut ws) = tokio_tungstenite::accept_async(stream).await {
                        count.fetch_add(1, Ordering::SeqCst);
                        match behavior {
                            ServerBehavior::Hold => {
                                // Drain frames until the client goes away
                                while let Some(Ok(_)) = ws.next().await {}
                            }
                            ServerBehavior::DropImmediately => {
                                drop(ws);
                            }
                        }
                    }
                });
            }
        });

        (addr, count)
    }

    // --- Backoff policy (pure) ---

    #[test]
    fn backoff_escalates_on_unstable_sessions() {
        let mut backoff = ReconnectBackoff::new();
        // A session that died well before the stability threshold must NOT
        // reset the delay — this is the auth-reject storm mode.
        let d1 = backoff.next_after_session(Duration::from_millis(20));
        let d2 = backoff.next_after_session(Duration::from_millis(20));
        let d3 = backoff.next_after_session(Duration::from_millis(20));
        assert_eq!(d1, BASE_RECONNECT_DELAY);
        assert_eq!(d2, BASE_RECONNECT_DELAY * 2);
        assert_eq!(d3, BASE_RECONNECT_DELAY * 4);
    }

    #[test]
    fn backoff_resets_after_stable_session() {
        let mut backoff = ReconnectBackoff::new();
        for _ in 0..5 {
            backoff.next_after_connect_failure();
        }
        let d = backoff.next_after_session(STABLE_SESSION_THRESHOLD + Duration::from_secs(1));
        assert_eq!(
            d, BASE_RECONNECT_DELAY,
            "stable session must reset the backoff to base"
        );
    }

    #[test]
    fn backoff_caps_at_max() {
        let mut backoff = ReconnectBackoff::new();
        let mut last = Duration::ZERO;
        for _ in 0..32 {
            last = backoff.next_after_connect_failure();
        }
        assert_eq!(last, MAX_RECONNECT_DELAY);
    }

    // --- Lifecycle: no leaked reconnect loops ---

    #[tokio::test]
    async fn connection_loop_exits_when_handle_dropped() {
        let (addr, count) = ws_test_server(ServerBehavior::Hold).await;
        let url = format!("ws://{addr}");

        let conn = ConductorConnection::connect(&url).await.expect("connect");
        assert!(conn.is_connected().await);
        assert_eq!(count.load(Ordering::SeqCst), 1);

        drop(conn);

        // Give a leaked loop ample time to betray itself: at the 100ms base
        // delay a leak reconnects ~10x/second.
        tokio::time::sleep(Duration::from_millis(1500)).await;
        assert_eq!(
            count.load(Ordering::SeqCst),
            1,
            "connection loop must exit when its handle is dropped, not keep reconnecting"
        );
    }

    #[tokio::test]
    async fn unstable_sessions_back_off_exponentially() {
        let (addr, count) = ws_test_server(ServerBehavior::DropImmediately).await;
        let url = format!("ws://{addr}");

        // Spawn without waiting for a stable connection — the server drops
        // every session instantly, so `connected` never holds.
        let _conn = ConductorConnection::spawn_with_auth_minter(&url, None, None);

        tokio::time::sleep(Duration::from_secs(3)).await;
        let attempts = count.load(Ordering::SeqCst);
        // With escalation (100+200+400+800+1600ms) we expect ~5-6 attempts in 3s.
        // Without it (the storm bug) the loop reconnects every ~100ms → ~30.
        assert!(
            attempts <= 8,
            "expected backoff to limit reconnects to <=8 in 3s, got {attempts} (storm behavior)"
        );
        assert!(
            attempts >= 2,
            "loop should still be retrying, got {attempts}"
        );
    }

    // --- Token re-mint on unstable auth sessions ---

    #[tokio::test]
    async fn unstable_auth_session_triggers_token_remint() {
        let (addr, _count) = ws_test_server(ServerBehavior::DropImmediately).await;
        let url = format!("ws://{addr}");

        let mint_calls = Arc::new(AtomicUsize::new(0));
        let mint_calls_for_minter = Arc::clone(&mint_calls);
        let minter: TokenMinter = Arc::new(move || {
            let calls = Arc::clone(&mint_calls_for_minter);
            Box::pin(async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Some(vec![9, 9, 9])
            })
        });

        let _conn =
            ConductorConnection::spawn_with_auth_minter(&url, Some(vec![1, 2, 3]), Some(minter));

        tokio::time::sleep(Duration::from_millis(1200)).await;
        assert!(
            mint_calls.load(Ordering::SeqCst) >= 1,
            "an unstable authenticated session must trigger a token re-mint"
        );
    }

    // --- Auth wire encoding ---

    /// The pre-fix hand-rolled rmpv envelope, kept ONLY as a negative fixture so
    /// the encoding test can prove the fix actually changed the wire bytes (the
    /// conductor never recognized this shape — the optimistic-auth storm bug).
    fn legacy_rmpv_authenticate(token: &[u8]) -> Vec<u8> {
        let inner = rmpv::Value::Map(vec![(
            rmpv::Value::String("token".into()),
            rmpv::Value::Binary(token.to_vec()),
        )]);
        let mut inner_buf = Vec::new();
        rmpv::encode::write_value(&mut inner_buf, &inner).unwrap();
        let envelope = rmpv::Value::Map(vec![
            (
                rmpv::Value::String("type".into()),
                rmpv::Value::String("authenticate".into()),
            ),
            (
                rmpv::Value::String("data".into()),
                rmpv::Value::Binary(inner_buf),
            ),
        ]);
        let mut buf = Vec::new();
        rmpv::encode::write_value(&mut buf, &envelope).unwrap();
        buf
    }

    #[test]
    fn authenticate_encoding_matches_official_wire_format() {
        use holochain_conductor_api::AppAuthenticationRequest;
        use holochain_serialized_bytes::prelude::*;
        use holochain_websocket::WireMessage;

        let token = vec![7u8, 8, 9, 10, 11, 12, 13];

        // The production encoder — exactly what the worker pool now sends.
        let produced = encode_authenticate(&token).expect("encode auth frame");

        // Independent reconstruction via the official Holochain wire types — the
        // SAME path `projection::subscriber::send_auth_request` uses, built here
        // from the real types (NOT a copied byte literal). A regression to a
        // hand-rolled rmpv shape would make these diverge and fail the test.
        let auth_request = AppAuthenticationRequest {
            token: token.clone(),
        };
        let inner: SerializedBytes = auth_request.try_into().expect("serialize auth request");
        let wire_msg = WireMessage::Authenticate {
            data: UnsafeBytes::from(inner).into(),
        };
        let outer: SerializedBytes = wire_msg.try_into().expect("serialize wire message");
        let expected = outer.bytes().to_vec();

        assert_eq!(
            produced, expected,
            "auth frame must be the official WireMessage::Authenticate encoding"
        );

        // Anti-tautology guard: the official frame must NOT equal the legacy
        // rmpv envelope — i.e. the fix genuinely changed the wire bytes.
        assert_ne!(
            produced,
            legacy_rmpv_authenticate(&token),
            "encoder must not regress to the legacy rmpv envelope the conductor rejected"
        );
    }

    // --- Re-mint discipline: clean shutdown vs. auth rejection ---

    #[tokio::test]
    async fn clean_shutdown_does_not_remint() {
        // Part 1: a stable authenticated session that ends because the OWNER
        // dropped the handle (ChannelClosed) must NOT re-mint — re-minting on a
        // clean shutdown was never the intent and would hammer the admin
        // interface. The Hold server keeps the socket open, so the auth ack
        // window elapses clean (success) and the session is stable.
        let (addr, _count) = ws_test_server(ServerBehavior::Hold).await;
        let url = format!("ws://{addr}");

        let mint_calls = Arc::new(AtomicUsize::new(0));
        let mint_calls_for_minter = Arc::clone(&mint_calls);
        let minter: TokenMinter = Arc::new(move || {
            let calls = Arc::clone(&mint_calls_for_minter);
            Box::pin(async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Some(vec![9, 9, 9])
            })
        });

        let conn =
            ConductorConnection::connect_with_auth_minter(&url, Some(vec![1, 2, 3]), Some(minter))
                .await
                .expect("authenticated connect");
        assert!(conn.is_connected().await);

        // Clean shutdown: drop the only handle → ChannelClosed.
        drop(conn);
        tokio::time::sleep(Duration::from_millis(400)).await;
        assert_eq!(
            mint_calls.load(Ordering::SeqCst),
            0,
            "a clean ChannelClosed shutdown must not trigger a token re-mint"
        );

        // Part 2 (contrast): a conductor that drops the socket at auth time is a
        // rejection, which MUST re-mint so a stale token self-heals.
        let (addr2, _count2) = ws_test_server(ServerBehavior::DropImmediately).await;
        let url2 = format!("ws://{addr2}");

        let reject_calls = Arc::new(AtomicUsize::new(0));
        let reject_calls_for_minter = Arc::clone(&reject_calls);
        let reject_minter: TokenMinter = Arc::new(move || {
            let calls = Arc::clone(&reject_calls_for_minter);
            Box::pin(async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Some(vec![5, 5, 5])
            })
        });

        let _conn2 = ConductorConnection::spawn_with_auth_minter(
            &url2,
            Some(vec![1, 2, 3]),
            Some(reject_minter),
        );
        tokio::time::sleep(Duration::from_millis(1200)).await;
        assert!(
            reject_calls.load(Ordering::SeqCst) >= 1,
            "a windowed auth rejection must trigger a token re-mint"
        );
    }
}
