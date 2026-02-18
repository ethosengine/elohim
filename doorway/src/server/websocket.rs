//! WebSocket upgrade and connection handling
//!
//! Handles WebSocket upgrades for both admin and app interfaces,
//! then delegates to the appropriate proxy.
//!
//! Authentication flow:
//! 1. Try JWT token from query string (?token=...)
//! 2. Try JWT token from Authorization header
//! 3. Try API key from X-API-Key header
//! 4. Default to Public permission (in dev mode) or Unauthorized (in production)

use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::auth::{
    extract_token_from_header, ApiKeyValidator, Claims, JwtValidator, PermissionLevel,
};
use crate::proxy;
use crate::server::http::AppState;

/// Handle WebSocket upgrade for admin interface
pub async fn handle_admin_upgrade(
    state: Arc<AppState>,
    req: Request<Incoming>,
) -> Response<Full<Bytes>> {
    let origin = req
        .headers()
        .get("origin")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Extract auth from request
    let auth_result = extract_permission(&state, &req);

    // Check if this agent has a conductor assignment for affinity routing
    let assigned_admin_url = resolve_admin_url(&state, &req);

    match auth_result {
        Ok(permission_level) => {
            info!(
                "Admin WebSocket upgrade request (origin: {:?}, permission: {}, affinity: {})",
                origin,
                permission_level,
                assigned_admin_url.as_deref().unwrap_or("default pool")
            );

            match hyper_tungstenite::upgrade(req, None) {
                Ok((response, websocket)) => {
                    let conductor_url = state.args.conductor_url.clone();
                    let dev_mode = state.args.dev_mode;

                    // Use ADMIN pool for admin connections (not app pool)
                    // Admin pool connects to conductor admin interface (port 4444)
                    let admin_pool = state.admin_pool.clone();

                    tokio::spawn(async move {
                        match websocket.await {
                            Ok(ws) => {
                                // Priority 1: Route to assigned conductor's admin (affinity)
                                if let Some(ref admin_url) = assigned_admin_url {
                                    if let Err(e) = proxy::admin::run_proxy(
                                        ws,
                                        admin_url,
                                        origin,
                                        dev_mode,
                                        permission_level,
                                    )
                                    .await
                                    {
                                        error!("Affinity admin proxy error: {:?}", e);
                                    }
                                }
                                // Priority 2: Global admin pool (load-balanced)
                                else if let Some(p) = admin_pool {
                                    if let Err(e) = proxy::pool::run_admin_proxy(
                                        ws,
                                        p,
                                        origin,
                                        dev_mode,
                                        permission_level,
                                    )
                                    .await
                                    {
                                        error!("Pool admin proxy error: {:?}", e);
                                    }
                                }
                                // Priority 3: Direct proxy to default conductor
                                else if let Err(e) = proxy::admin::run_proxy(
                                    ws,
                                    &conductor_url,
                                    origin,
                                    dev_mode,
                                    permission_level,
                                )
                                .await
                                {
                                    error!("Admin proxy error: {:?}", e);
                                }
                            }
                            Err(e) => {
                                error!("WebSocket upgrade failed: {:?}", e);
                            }
                        }
                    });

                    // Convert the upgrade response
                    let (parts, _) = response.into_parts();
                    Response::from_parts(parts, Full::new(Bytes::new()))
                }
                Err(e) => {
                    error!("WebSocket upgrade error: {:?}", e);
                    Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Full::new(Bytes::from(format!(
                            "WebSocket upgrade failed: {e}"
                        ))))
                        .unwrap()
                }
            }
        }
        Err(err_msg) => {
            warn!("Admin WebSocket auth failed: {}", err_msg);
            Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error":"{err_msg}"}}"#
                ))))
                .unwrap()
        }
    }
}

/// Handle WebSocket upgrade for app interface
pub async fn handle_app_upgrade(
    state: Arc<AppState>,
    req: Request<Incoming>,
    port: u16,
) -> Response<Full<Bytes>> {
    let origin = req
        .headers()
        .get("origin")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Preserve query parameters (like auth token)
    let query = req.uri().query().map(|q| q.to_string());

    // Route to the agent's assigned conductor if JWT present, else use default
    let (conductor_host, conductor_port) = resolve_conductor_for_app(&state, &req, port);

    info!(
        "App WebSocket upgrade request for port {} (origin: {:?}, conductor: {}:{})",
        port, origin, conductor_host, conductor_port
    );

    match hyper_tungstenite::upgrade(req, None) {
        Ok((response, websocket)) => {
            // App connections use direct proxy to the conductor hosting this agent
            tokio::spawn(async move {
                match websocket.await {
                    Ok(ws) => {
                        if let Err(e) = proxy::app::run_proxy(
                            ws,
                            conductor_port,
                            origin,
                            query,
                            &conductor_host,
                        )
                        .await
                        {
                            error!("App proxy error (port {}): {:?}", conductor_port, e);
                        }
                    }
                    Err(e) => {
                        error!("WebSocket upgrade failed: {:?}", e);
                    }
                }
            });

            // Convert the upgrade response
            let (parts, _) = response.into_parts();
            Response::from_parts(parts, Full::new(Bytes::new()))
        }
        Err(e) => {
            error!("WebSocket upgrade error: {:?}", e);
            Response::builder()
                .status(hyper::StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(format!(
                    "WebSocket upgrade failed: {e}"
                ))))
                .unwrap()
        }
    }
}

/// Resolve the conductor host and port for an app WebSocket request.
///
/// Routing priority:
/// 1. Agent is in conductor registry → use assigned conductor's host and port
/// 2. Agent has session affinity (set by admin WS) → reuse it
/// 3. Fall back to default conductor URL and client-requested port
fn resolve_conductor_for_app(
    state: &AppState,
    req: &Request<Incoming>,
    client_port: u16,
) -> (String, u16) {
    let fallback = || {
        (
            extract_conductor_host(&state.args.conductor_url),
            client_port,
        )
    };

    let registry = match &state.conductor_registry {
        Some(r) => r,
        None => return fallback(),
    };

    let claims = match extract_claims(state, req) {
        Some(c) => c,
        None => return fallback(),
    };

    // Priority 1: Agent is in the registry (provisioned)
    if let Some(entry) = registry.get_conductor_for_agent(&claims.agent_pub_key) {
        if let Some((host, port)) = extract_host_and_port(&entry.conductor_url) {
            info!(
                agent = %claims.agent_pub_key,
                conductor = %entry.conductor_id,
                host = %host,
                port = port,
                "App WS routed to assigned conductor"
            );
            return (host, port);
        }
    }

    // Priority 2: Check session affinity cache (set by admin WS connection)
    if let Some(affinity) = state.session_affinity.get(&claims.agent_pub_key) {
        info!(
            agent = %claims.agent_pub_key,
            host = %affinity.conductor_host,
            port = affinity.app_port,
            "App WS using session affinity"
        );
        return (affinity.conductor_host.clone(), affinity.app_port);
    }

    fallback()
}

/// Resolve the admin URL for an admin WebSocket request.
///
/// Routing priority:
/// 1. Agent is in conductor registry → use assigned conductor's admin_url
/// 2. Agent has session affinity (from previous admin connection) → reuse it
/// 3. Registry has conductors → pick least-loaded, cache in session affinity
/// 4. No registry → return None (fall through to admin pool or default)
fn resolve_admin_url(state: &AppState, req: &Request<Incoming>) -> Option<String> {
    let registry = state.conductor_registry.as_ref()?;
    let claims = extract_claims(state, req)?;

    // Priority 1: Agent is in the registry (provisioned)
    if let Some(entry) = registry.get_conductor_for_agent(&claims.agent_pub_key) {
        if let Some(conductor_info) = registry.get_conductor_info(&entry.conductor_id) {
            info!(
                agent = %claims.agent_pub_key,
                conductor = %entry.conductor_id,
                admin_url = %conductor_info.admin_url,
                "Admin WS routed to assigned conductor"
            );
            return Some(conductor_info.admin_url);
        }
    }

    // Priority 2: Check session affinity cache
    if let Some(affinity) = state.session_affinity.get(&claims.agent_pub_key) {
        info!(
            agent = %claims.agent_pub_key,
            admin_url = %affinity.admin_url,
            "Admin WS using session affinity"
        );
        return Some(affinity.admin_url.clone());
    }

    // Priority 3: Pick least-loaded conductor and cache for session affinity
    if let Some(conductor) = registry.find_least_loaded() {
        let app_port = extract_host_and_port(&conductor.conductor_url)
            .map(|(_, port)| port)
            .unwrap_or(4445);
        let affinity = crate::server::http::SessionAffinityEntry {
            admin_url: conductor.admin_url.clone(),
            conductor_host: extract_conductor_host(&conductor.conductor_url),
            app_port,
        };
        info!(
            agent = %claims.agent_pub_key,
            conductor = %conductor.conductor_id,
            admin_url = %conductor.admin_url,
            "Admin WS: session affinity assigned to least-loaded conductor"
        );
        state
            .session_affinity
            .insert(claims.agent_pub_key.clone(), affinity);
        return Some(conductor.admin_url);
    }

    None
}

/// Extract JWT claims from a request (query string or Authorization header).
fn extract_claims(state: &AppState, req: &Request<Incoming>) -> Option<Claims> {
    // Try query string first
    if let Some(token) = extract_token_from_query(req.uri().query()) {
        return decode_jwt_claims(state, &token);
    }

    // Try Authorization header
    let auth_header = req
        .headers()
        .get(hyper::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    if let Some(token) = extract_token_from_header(auth_header) {
        return decode_jwt_claims(state, token);
    }

    None
}

/// Decode JWT token and return full claims (unlike validate_jwt which returns only permission).
fn decode_jwt_claims(state: &AppState, token: &str) -> Option<Claims> {
    let jwt = if state.args.dev_mode {
        JwtValidator::new_dev()
    } else {
        state.args.jwt_secret.as_ref().and_then(|secret| {
            JwtValidator::new(secret.clone(), state.args.jwt_expiry_seconds).ok()
        })?
    };

    let result = jwt.verify_token(token);
    if result.valid {
        result.claims
    } else {
        None
    }
}

/// Extract host and port from a WebSocket URL.
///
/// e.g. "ws://elohim-edgenode-alpha-0.elohim-edgenode-alpha-headless:8445" -> ("elohim-edgenode-alpha-0.elohim-edgenode-alpha-headless", 8445)
fn extract_host_and_port(url: &str) -> Option<(String, u16)> {
    let after_scheme = url.split("://").nth(1)?;
    let colon = after_scheme.rfind(':')?;
    let host = after_scheme[..colon].to_string();
    let port = after_scheme[colon + 1..].parse::<u16>().ok()?;
    Some((host, port))
}

/// Extract the host portion from a conductor URL.
///
/// e.g. "ws://elohim-edgenode-alpha:4445" -> "elohim-edgenode-alpha"
///      "ws://localhost:4445"             -> "localhost"
fn extract_conductor_host(conductor_url: &str) -> String {
    if let Some(after_scheme) = conductor_url.split("://").nth(1) {
        // Strip port if present
        if let Some(colon) = after_scheme.rfind(':') {
            return after_scheme[..colon].to_string();
        }
        return after_scheme.to_string();
    }
    "localhost".to_string()
}

/// Extract permission level from request
///
/// Authentication sources (in order of precedence):
/// 1. JWT token from query string (?token=...)
/// 2. JWT token from Authorization header
/// 3. API key from X-API-Key header
/// 4. Default to Public permission in dev mode, or error in production
fn extract_permission(
    state: &AppState,
    req: &Request<Incoming>,
) -> Result<PermissionLevel, String> {
    // Try JWT token from query string
    if let Some(token) = extract_token_from_query(req.uri().query()) {
        if let Some(claims) = validate_jwt(state, &token) {
            return Ok(claims);
        }
    }

    // Try JWT token from Authorization header
    let auth_header = req
        .headers()
        .get(hyper::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    if let Some(token) = extract_token_from_header(auth_header) {
        if let Some(claims) = validate_jwt(state, token) {
            return Ok(claims);
        }
    }

    // Try API key from X-API-Key header
    let api_key = req.headers().get("x-api-key").and_then(|v| v.to_str().ok());

    let api_validator = ApiKeyValidator::new(
        state.args.api_key_authenticated.clone(),
        state.args.api_key_admin.clone(),
    );

    if let Some(permission) = api_validator.validate(api_key) {
        // API key validation succeeded (or no key required)
        // In production without auth, Public is only returned if no key was provided
        // and API keys aren't configured
        if api_key.is_some() || !api_validator.is_configured() {
            return Ok(permission);
        }
    }

    // No valid auth found
    if state.args.dev_mode {
        // In dev mode, allow public access by default
        info!("Dev mode: allowing public access without authentication");
        Ok(PermissionLevel::Public)
    } else {
        Err("Authentication required".to_string())
    }
}

/// Extract token from query string
fn extract_token_from_query(query: Option<&str>) -> Option<String> {
    let query = query?;
    for param in query.split('&') {
        if let Some((key, value)) = param.split_once('=') {
            if key == "token" {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// Validate JWT token and return permission level
fn validate_jwt(state: &AppState, token: &str) -> Option<PermissionLevel> {
    let jwt = if state.args.dev_mode {
        JwtValidator::new_dev()
    } else {
        state.args.jwt_secret.as_ref().and_then(|secret| {
            JwtValidator::new(secret.clone(), state.args.jwt_expiry_seconds).ok()
        })?
    };

    let result = jwt.verify_token(token);
    if result.valid {
        result.claims.map(|c| c.permission_level)
    } else {
        None
    }
}
