//! App interface proxy
//!
//! Simple passthrough WebSocket proxy for app interfaces.
//! No message filtering needed - app interfaces handle their own auth.

use futures_util::{SinkExt, StreamExt};
use std::{io::ErrorKind, time::Duration};
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{
        error::ProtocolError,
        http::Request,
        protocol::{frame::coding::CloseCode, CloseFrame, Message},
        Error,
    },
};
use tracing::{debug, error, info};

use crate::types::{DoorwayError, Result};

type HyperWebSocket =
    hyper_tungstenite::WebSocketStream<hyper_util::rt::TokioIo<hyper::upgrade::Upgraded>>;

// These client-visible labels must never interpolate upstream error details.
fn upstream_failure_reason(error: &Error) -> String {
    match error {
        Error::Io(error) => match error.kind() {
            ErrorKind::ConnectionRefused => "upstream app connection refused".to_owned(),
            ErrorKind::TimedOut => "upstream app connection timed out".to_owned(),
            _ => "upstream app I/O failure".to_owned(),
        },
        Error::Http(response) => {
            format!("upstream app handshake HTTP {}", response.status().as_u16())
        }
        Error::Protocol(error) => {
            let category = match error {
                ProtocolError::HandshakeIncomplete => "incomplete",
                ProtocolError::WrongHttpMethod => "wrong HTTP method",
                ProtocolError::WrongHttpVersion => "wrong HTTP version",
                ProtocolError::MissingConnectionUpgradeHeader => "missing connection upgrade",
                ProtocolError::MissingUpgradeWebSocketHeader => "missing websocket upgrade",
                ProtocolError::MissingSecWebSocketVersionHeader => "missing websocket version",
                ProtocolError::MissingSecWebSocketKey => "missing websocket key",
                ProtocolError::SecWebSocketAcceptKeyMismatch => "accept key mismatch",
                ProtocolError::SecWebSocketSubProtocolError(_) => "subprotocol mismatch",
                ProtocolError::InvalidHeader(_) => "invalid header",
                ProtocolError::HttparseError(_) => "malformed HTTP",
                ProtocolError::JunkAfterRequest => "junk after request",
                ProtocolError::CustomResponseSuccessful => "unexpected successful response",
                _ => "other",
            };
            format!("upstream app handshake protocol error: {category}")
        }
        Error::Tls(_) => "upstream app TLS failure".to_owned(),
        _ => "upstream app connection failed".to_owned(),
    }
}

/// Build the conductor app interface URL from the conductor host (not hardcoded localhost).
///
/// Doorway-specific params (apiKey, token, conductor_id) are for doorway auth/routing only
/// and are stripped. The real Holochain app auth token is sent by AppWebsocket in the WS
/// handshake, not the URL. A surviving query is always preceded by `/`: tungstenite writes
/// the URI's path-and-query verbatim into the request line, so `ws://host:port?x=y` goes out
/// as `GET ?x=y HTTP/1.1`, which the conductor drops before answering the handshake.
fn upstream_app_url(conductor_host: &str, port: u16, query: Option<&str>) -> String {
    let base = format!("ws://{conductor_host}:{port}");
    let filtered: Vec<&str> = query
        .unwrap_or_default()
        .split('&')
        .filter(|param| {
            !param.is_empty()
                && !param.starts_with("apiKey=")
                && !param.starts_with("token=")
                && !param.starts_with("conductor_id=")
        })
        .collect();
    if filtered.is_empty() {
        base
    } else {
        format!("{base}/?{}", filtered.join("&"))
    }
}

/// Run the app proxy between client and conductor app interface.
///
/// `conductor_host` is the hostname of the conductor (e.g. "elohim-edgenode-alpha")
/// extracted from CONDUCTOR_URL. Falls back to "localhost" for local dev.
pub async fn run_proxy(
    mut client_ws: HyperWebSocket,
    port: u16,
    origin: Option<String>,
    query: Option<String>,
    conductor_host: &str,
) -> Result<()> {
    let app_url = upstream_app_url(conductor_host, port, query.as_deref());

    info!("Creating app proxy to {} (origin: {:?})", app_url, origin);

    // Connect to conductor app interface with proper headers
    let request = Request::builder()
        .uri(&app_url)
        .header("Host", format!("{conductor_host}:{port}"))
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

    let (conductor_ws, _) = match connect_async_with_config(request, None, false).await {
        Ok(connection) => connection,
        Err(e) => {
            // Only disclose a bounded transport classification, never the upstream
            // URL, headers, response body or raw error. Native auth has not run.
            let reason = upstream_failure_reason(&e);
            let _ = tokio::time::timeout(
                Duration::from_secs(1),
                client_ws.send(Message::Close(Some(CloseFrame {
                    code: CloseCode::Error,
                    reason: reason.into(),
                }))),
            )
            .await;
            return Err(DoorwayError::Holochain(format!(
                "Failed to connect to app interface: {e}"
            )));
        }
    };

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
                    if let Err(e) = client_sink.send(Message::Binary(data)).await {
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
                    let _ = client_sink.send(Message::Ping(data)).await;
                }
                Ok(Message::Pong(data)) => {
                    let _ = client_sink.send(Message::Pong(data)).await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::{server::conn::http1, service::service_fn};
    use hyper_util::rt::TokioIo;
    use std::convert::Infallible;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    #[tokio::test]
    async fn upstream_handshake_failure_reaches_client_as_safe_1011_close() {
        assert_safe_upstream_close(
            b"HTTP/1.1 403 Forbidden\r\nContent-Length: 21\r\nX-Private: private-token-fixture\r\n\r\nprivate-token-fixture",
            "upstream app handshake HTTP 403",
        )
        .await;
    }

    #[tokio::test]
    async fn incomplete_upstream_handshake_reaches_client_as_safe_1011_close() {
        assert_safe_upstream_close(b"", "upstream app handshake protocol error: incomplete").await;
    }

    #[tokio::test]
    async fn upstream_accept_key_mismatch_reaches_client_as_safe_1011_close() {
        assert_safe_upstream_close(
            b"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Accept: private-token-fixture\r\n\r\n",
            "upstream app handshake protocol error: accept key mismatch",
        )
        .await;
    }

    #[test]
    fn upstream_url_strips_doorway_params_and_keeps_a_rooted_request_target() {
        assert_eq!(upstream_app_url("c", 8445, None), "ws://c:8445");
        assert_eq!(
            upstream_app_url("c", 8445, Some("conductor_id=conductor-1")),
            "ws://c:8445"
        );
        assert_eq!(
            upstream_app_url("c", 8445, Some("token=t&apiKey=k&conductor_id=x")),
            "ws://c:8445"
        );
        assert_eq!(
            upstream_app_url("c", 8445, Some("conductor_id=x&keep=1")),
            "ws://c:8445/?keep=1"
        );
    }

    /// The request line the conductor receives must name a rooted target. A selector
    /// query used to reach it as `GET ?conductor_id=… HTTP/1.1`.
    #[test]
    fn selector_query_reaches_upstream_with_a_rooted_request_line() {
        use tokio_tungstenite::tungstenite::http::Uri;
        // The defect: with no path, the query alone becomes the request target.
        let unrooted: Uri = "ws://127.0.0.1:8445?conductor_id=conductor-1"
            .parse()
            .unwrap();
        assert_eq!(
            unrooted.path_and_query().unwrap().as_str(),
            "?conductor_id=conductor-1"
        );
        for query in [
            "conductor_id=conductor-1",
            "conductor_id=conductor-1&keep=1",
        ] {
            let url = upstream_app_url("127.0.0.1", 8445, Some(query));
            let uri: Uri = url.parse().unwrap();
            let target = uri.path_and_query().unwrap().as_str();
            assert!(target.starts_with('/'), "unrooted request target: {target}");
            assert!(!target.contains("conductor_id"));
        }
    }

    #[test]
    fn invalid_header_diagnostic_does_not_disclose_attacker_controlled_name() {
        let error = Error::Protocol(ProtocolError::InvalidHeader(
            "private-token-fixture".parse().unwrap(),
        ));
        assert_eq!(
            upstream_failure_reason(&error),
            "upstream app handshake protocol error: invalid header"
        );
    }

    async fn assert_safe_upstream_close(response: &'static [u8], expected_reason: &str) {
        tokio::time::timeout(Duration::from_secs(10), async {
            let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let upstream_port = upstream.local_addr().unwrap().port();
            let upstream_task = tokio::spawn(async move {
                let (mut socket, _) = upstream.accept().await.unwrap();
                let mut request = Vec::new();
                let mut buffer = [0; 1024];
                while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                    let count = socket.read(&mut buffer).await.unwrap();
                    assert!(count > 0);
                    request.extend_from_slice(&buffer[..count]);
                }
                socket.write_all(response).await.unwrap();
            });
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let proxy_task = tokio::spawn(async move {
                let (socket, _) = listener.accept().await.unwrap();
                http1::Builder::new()
                    .serve_connection(
                        TokioIo::new(socket),
                        service_fn(move |request| async move {
                            let (response, websocket) =
                                hyper_tungstenite::upgrade(request, None).unwrap();
                            tokio::spawn(async move {
                                let websocket = websocket.await.unwrap();
                                let _ =
                                    run_proxy(websocket, upstream_port, None, None, "127.0.0.1")
                                        .await;
                            });
                            Ok::<_, Infallible>(response)
                        }),
                    )
                    .with_upgrades()
                    .await
                    .unwrap();
            });
            let (mut client, _) = tokio_tungstenite::connect_async(format!("ws://{address}"))
                .await
                .unwrap();
            let message = client.next().await.unwrap().unwrap();
            let Message::Close(Some(frame)) = message else {
                panic!("expected upstream failure to arrive as a websocket close frame");
            };
            assert_eq!(frame.code, CloseCode::Error);
            assert_eq!(frame.reason, expected_reason);
            assert!(!frame.reason.contains("private-token-fixture"));
            assert!(!frame.reason.contains("127.0.0.1"));
            upstream_task.await.unwrap();
            proxy_task.await.unwrap();
        })
        .await
        .expect("real websocket failure propagation must remain bounded");
    }
}
