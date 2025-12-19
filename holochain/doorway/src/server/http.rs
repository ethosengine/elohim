//! HTTP server implementation
//!
//! Pattern adapted from holo-host/rust/holo-gateway/src/lib.rs
//! Uses hyper http1 with TokioIo for async handling.

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info, warn};

use crate::config::Args;
use crate::db::MongoClient;
use crate::nats::{HostRouter, NatsClient};
use crate::routes;
use crate::server::websocket;
use crate::types::DoorwayError;
use crate::worker::WorkerPool;

type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

/// Shared application state
pub struct AppState {
    pub args: Args,
    pub mongo: Option<MongoClient>,
    pub nats: Option<NatsClient>,
    pub router: HostRouter,
    /// Worker pool for request routing (always available)
    pub pool: Option<Arc<WorkerPool>>,
}

impl AppState {
    /// Create AppState without external services (dev mode, direct proxy)
    pub fn new(args: Args) -> Self {
        Self {
            args,
            mongo: None,
            nats: None,
            router: HostRouter::new(None),
            pool: None,
        }
    }

    /// Create AppState with services but no worker pool (direct proxy mode)
    pub fn with_services(
        args: Args,
        mongo: Option<MongoClient>,
        nats: Option<NatsClient>,
    ) -> Self {
        let router = HostRouter::new(nats.clone());
        Self {
            args,
            mongo,
            nats,
            router,
            pool: None,
        }
    }

    /// Create AppState with worker pool (pooled connection mode)
    pub fn with_pool(
        args: Args,
        mongo: Option<MongoClient>,
        nats: Option<NatsClient>,
        pool: Arc<WorkerPool>,
    ) -> Self {
        let router = HostRouter::new(nats.clone());
        Self {
            args,
            mongo,
            nats,
            router,
            pool: Some(pool),
        }
    }
}

/// Start the HTTP server
pub async fn run(state: Arc<AppState>) -> Result<(), DoorwayError> {
    let listener = TcpListener::bind(state.args.listen).await?;

    info!(
        "Doorway listening on {} as node {}",
        state.args.listen, state.args.node_id
    );

    if state.args.dev_mode {
        warn!("Development mode enabled - authentication disabled");
    }

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                let state = Arc::clone(&state);
                tokio::spawn(async move {
                    let io = TokioIo::new(stream);

                    let service = service_fn(move |req| {
                        let state = Arc::clone(&state);
                        async move { handle_request(state, addr, req).await }
                    });

                    if let Err(err) = http1::Builder::new()
                        .preserve_header_case(true)
                        .title_case_headers(true)
                        .serve_connection(io, service)
                        .with_upgrades()
                        .await
                    {
                        error!("Error serving connection from {}: {:?}", addr, err);
                    }
                });
            }
            Err(e) => {
                error!("Error accepting connection: {:?}", e);
            }
        }
    }
}

/// Route incoming HTTP requests
async fn handle_request(
    state: Arc<AppState>,
    addr: SocketAddr,
    req: Request<Incoming>,
) -> Result<Response<BoxBody>, hyper::Error> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    info!("[{}] {} {}", addr, method, path);

    // Handle auth routes (/auth/*) - these consume the request
    if path.starts_with("/auth") {
        if let Some(response) = routes::handle_auth_request(req, Arc::clone(&state)).await {
            return Ok(response);
        }
        // If handle_auth_request returns None, it didn't handle the request
        // This shouldn't happen since we checked the path prefix, but handle gracefully
        return Ok(to_boxed(not_found_response(&path)));
    }

    let response = match (method, path.as_str()) {
        // Health check endpoints
        (Method::GET, "/health") | (Method::GET, "/healthz") => {
            to_boxed(routes::health_check(&state.args))
        }

        // CORS preflight
        (Method::OPTIONS, _) => to_boxed(preflight_response()),

        // WebSocket upgrade for admin interface
        (Method::GET, "/") | (Method::GET, "/admin") => {
            if hyper_tungstenite::is_upgrade_request(&req) {
                to_boxed(websocket::handle_admin_upgrade(state, req).await)
            } else {
                to_boxed(not_found_response(&path))
            }
        }

        // WebSocket upgrade for app interface
        (Method::GET, p) if p.starts_with("/app/") => {
            if hyper_tungstenite::is_upgrade_request(&req) {
                // Extract port from /app/:port
                let port_str = p.strip_prefix("/app/").unwrap_or("");
                match port_str.parse::<u16>() {
                    Ok(port) if state.args.is_valid_app_port(port) => {
                        to_boxed(websocket::handle_app_upgrade(state, req, port).await)
                    }
                    _ => to_boxed(bad_request_response("Invalid app port")),
                }
            } else {
                to_boxed(not_found_response(p))
            }
        }

        // Status endpoint with runtime info
        (Method::GET, "/status") => to_boxed(routes::status_check(Arc::clone(&state)).await),

        // Not found
        _ => to_boxed(not_found_response(&path)),
    };

    Ok(response)
}

/// Convert a Full<Bytes> body to BoxBody
fn to_boxed(response: Response<Full<Bytes>>) -> Response<BoxBody> {
    response.map(|body| body.map_err(|never| match never {}).boxed())
}

/// CORS preflight response
fn preflight_response() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Headers", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        .body(Full::new(Bytes::new()))
        .unwrap()
}

/// Not found response
fn not_found_response(path: &str) -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "error": "Not Found",
        "path": path,
        "hint": "Use WebSocket connection to /admin or /app/:port"
    });

    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

/// Bad request response
fn bad_request_response(message: &str) -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "error": "Bad Request",
        "message": message
    });

    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

