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
//! - On unstable authenticated sessions the loop re-mints its app-auth token
//!   via the injected [`TokenMinter`] (rate-limited): a conductor restart
//!   invalidates previously issued tokens, and without re-minting the pool
//!   stayed broken until the doorway itself was restarted.

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
    /// The WebSocket closed or errored — reconnect.
    ConnectionClosed,
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
            Ok((mut ws_sink, ws_stream)) => {
                // Authenticate if a token is configured (Holochain 0.6 app interface)
                let token = token_slot.read().await.clone();
                if let Some(ref token) = token {
                    if let Err(e) = send_authenticate(&mut ws_sink, token).await {
                        error!("Failed to authenticate with conductor: {}", e);
                        *connected.write().await = false;
                        // The socket died under the auth send — same unstable
                        // signature as an accept-then-drop session, so the
                        // token may be stale here too.
                        remint_if_due(&token_minter, &token_slot, &mut last_remint).await;
                        Some(backoff.next_after_connect_failure())
                    } else {
                        debug!("Authenticated with conductor");
                        run_session(
                            ws_sink,
                            ws_stream,
                            &mut rx,
                            &connected,
                            &mut backoff,
                            &token_slot,
                            &token_minter,
                            &mut last_remint,
                        )
                        .await
                    }
                } else {
                    run_session(
                        ws_sink,
                        ws_stream,
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
    rx: &mut mpsc::Receiver<(Vec<u8>, oneshot::Sender<Vec<u8>>)>,
    connected: &Arc<RwLock<bool>>,
    backoff: &mut ReconnectBackoff,
    token_slot: &Arc<RwLock<Option<Vec<u8>>>>,
    token_minter: &Option<TokenMinter>,
    last_remint: &mut Option<Instant>,
) -> Option<Duration> {
    *connected.write().await = true;
    info!("Connected to conductor");
    let session_start = Instant::now();

    let session_end = handle_messages(ws_sink, ws_stream, rx).await;

    *connected.write().await = false;
    let session_len = session_start.elapsed();

    if matches!(session_end, SessionEnd::ChannelClosed) {
        return None;
    }

    // An unstable session with auth configured is the stale/invalid-token
    // signature (a conductor restart invalidates issued tokens): re-mint,
    // rate-limited so a dead admin interface doesn't get hammered either.
    if session_len < STABLE_SESSION_THRESHOLD {
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

/// Send authenticate message after WebSocket connect.
///
/// Holochain 0.6 app interface format: { type: "authenticate", data: <binary {token: <bytes>}> }
async fn send_authenticate(ws_sink: &mut WsSink, token: &[u8]) -> Result<()> {
    let inner = rmpv::Value::Map(vec![(
        rmpv::Value::String("token".into()),
        rmpv::Value::Binary(token.to_vec()),
    )]);

    let mut inner_buf = Vec::new();
    rmpv::encode::write_value(&mut inner_buf, &inner)
        .map_err(|e| DoorwayError::Holochain(format!("Failed to encode auth: {e}")))?;

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
    rmpv::encode::write_value(&mut buf, &envelope)
        .map_err(|e| DoorwayError::Holochain(format!("Failed to encode auth envelope: {e}")))?;

    ws_sink
        .send(Message::Binary(buf))
        .await
        .map_err(|e| DoorwayError::Holochain(format!("Failed to send auth: {e}")))?;

    // Brief pause — if conductor rejects, it closes the connection
    tokio::time::sleep(Duration::from_millis(50)).await;

    Ok(())
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
                break SessionEnd::ConnectionClosed;
            }
        }
    };

    // Task to handle responses from conductor
    let response_handler = async {
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
                    info!("Conductor closed connection: {:?}", frame);
                    break;
                }
                Err(e) => {
                    error!("Conductor WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
        SessionEnd::ConnectionClosed
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
}
