//! WebSocket upgrade and connection handling
//!
//! Handles WebSocket upgrades for both admin and app interfaces,
//! then delegates to the appropriate proxy.
//!
//! Authentication flow:
//! 1. Try JWT token from query string (?token=...)
//! 2. Try JWT token from Authorization header
//! 3. Try API key from X-API-Key header
//! 4. No credential ⇒ the native local-first grant ([`native_local_first_operator`])
//!    or Unauthorized. Never a `DEV_MODE` fallthrough.

use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use seam_contracts::freshness::NetworkStage;
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
    peer_is_loopback: bool,
) -> Response<Full<Bytes>> {
    let origin = req
        .headers()
        .get("origin")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Extract auth from request. This is now the SINGLE authorization decision
    // for the conductor admin socket — the route arm in `http.rs` used to carry
    // a second, `dev_mode`-keyed one, and the proxy a third.
    let auth_result = extract_permission(&state, &req, peer_is_loopback);

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

    // Priority 0: conductor_id in JWT claims (deterministic, no registry lookup)
    if let Some(ref conductor_id) = claims.conductor_id {
        if let Some(ref registry_ref) = state.conductor_registry {
            if let Some(info) = registry_ref.get_conductor_info(conductor_id) {
                if let Some((host, port)) = extract_host_and_port(&info.conductor_url) {
                    info!(
                        agent = %claims.agent_pub_key,
                        conductor = %conductor_id,
                        host = %host,
                        port = port,
                        "App WS routed via JWT conductor_id"
                    );
                    return (host, port);
                }
            }
        }
    }

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

    // Priority 0: conductor_id in JWT claims (deterministic, no registry lookup)
    if let Some(ref conductor_id) = claims.conductor_id {
        if let Some(info) = registry.get_conductor_info(conductor_id) {
            info!(
                agent = %claims.agent_pub_key,
                conductor = %conductor_id,
                admin_url = %info.admin_url,
                "Admin WS routed via JWT conductor_id"
            );
            return Some(info.admin_url);
        }
    }

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

    // Priority 2: Pick least-loaded conductor (no session affinity needed —
    // conductor_id in JWT provides deterministic routing after chaperone)
    if let Some(conductor) = registry.find_least_loaded() {
        info!(
            agent = %claims.agent_pub_key,
            conductor = %conductor.conductor_id,
            admin_url = %conductor.admin_url,
            "Admin WS: routed to least-loaded conductor"
        );
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
    let jwt = JwtValidator::from_config(
        state.args.configured_jwt_secret(),
        state.args.jwt_expiry_seconds,
    )
    .ok()?;

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
/// Is this caller the operator of a developer's OWN box, on a doorway that is
/// not a deployment at all?
///
/// Three conjuncts, and all three are needed:
///
/// 1. **On the box** — the peer address is loopback. In the devspace and in
///    plain `ng serve` local dev, the browser reaches the doorway through the
///    Angular dev-server proxy running in the same container, so the proxy's
///    hop is loopback. From the open internet it never is: every deployed
///    doorway is reached through an ingress, whose hop carries the ingress
///    pod's address.
/// 2. **Pre-coordination** — the network has not DECLARED a coordinated stage
///    (`ELOHIM_NETWORK_STAKES`, fail-closed to `Bootstrap`). Same discipline as
///    `auth/http_permission.rs` and elohim-storage `trust/stage.rs`.
/// 3. **No declared doorway identity** — `JWT_SECRET` is unset. This is the
///    crate's established presence-keyed discriminator (`configured_jwt_secret`,
///    `configured_seed_key`): a doorway that was handed a signing secret was
///    handed it BY a deployment, and all five deployed manifests populate it
///    from a `secretKeyRef`. A developer's own `hc-start.sh` box has none.
///
/// This is what the old `if state.args.dev_mode` fallthrough was reaching for
/// and got wrong. `DEV_MODE: "true"` is set on EVERY deployed manifest, so
/// keying the conductor admin socket on it left an unauthenticated, UNFILTERED
/// Holochain admin interface reachable from the open internet on the whole
/// fleet. The conjunction above cannot be true on any of them.
///
/// The grant is `Admin` and it is deliberate: on your own machine, talking to
/// your own conductor, with no identity system configured, you ARE that
/// conductor's operator. That is the native local-first developer mode. Every
/// other mode presents a credential — the web devspace and the deployed fleet
/// both go through `POST /hc/connect` (the chaperone), which binds a browser
/// session to a DHT-authoritative agent key.
fn native_local_first_operator(state: &AppState, peer_is_loopback: bool) -> bool {
    peer_is_loopback
        && state.network_stage < NetworkStage::Coordinated
        && state.args.configured_jwt_secret().is_none()
}

/// Authentication sources (in order of precedence):
/// 1. JWT token from query string (?token=...)
/// 2. JWT token from Authorization header
/// 3. API key from X-API-Key header
/// 4. No credential ⇒ [`native_local_first_operator`] or refuse.
fn extract_permission<B>(
    state: &AppState,
    req: &Request<B>,
    peer_is_loopback: bool,
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

    // A PRESENTED key is the only thing this arm may act on. The old
    // `|| !api_validator.is_configured()` disjunct was the second credential-free
    // path to a grant, and it did not even need `dev_mode` to fire: with no API
    // keys configured, `validate(None)` returns `Some(Public)` and the disjunct
    // made it a `return`. That is precisely the devspace/mesh shape, which is why
    // this arm and the `dev_mode` fallthrough below had to close together — shutting
    // either one alone leaves the ladder open through the other.
    if api_key.is_some() {
        if let Some(permission) = api_validator.validate(api_key) {
            return Ok(permission);
        }
    }

    // No credential presented.
    if native_local_first_operator(state, peer_is_loopback) {
        info!("Native local-first operator (loopback, pre-coordination, no declared JWT secret): granting admin access to this box's own conductor");
        return Ok(PermissionLevel::Admin);
    }

    Err("Authentication required. Use POST /hc/connect.".to_string())
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
    let jwt = JwtValidator::from_config(
        state.args.configured_jwt_secret(),
        state.args.jwt_expiry_seconds,
    )
    .ok()?;

    let result = jwt.verify_token(token);
    if result.valid {
        result.claims.map(|c| c.permission_level)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Args;
    use clap::Parser;

    /// Build an `AppState` with an explicit posture. `jwt_secret` present models
    /// a DECLARED deployment (all five deployed manifests populate it from a
    /// `secretKeyRef`); absent models a developer's own `hc-start.sh` box.
    fn state_with(stage: NetworkStage, jwt_secret: Option<&str>, dev_mode: bool) -> AppState {
        let mut args = Args::parse_from(["doorway", "--listen", "127.0.0.1:0"]);
        args.dev_mode = dev_mode;
        args.jwt_secret = jwt_secret.map(str::to_string);
        let mut state = AppState::new(args);
        state.network_stage = stage;
        state
    }

    #[test]
    fn native_local_first_requires_all_three_conjuncts() {
        // The developer's own box: loopback, pre-coordination, no declared secret.
        let bare = state_with(NetworkStage::Bootstrap, None, false);
        assert!(native_local_first_operator(&bare, true));

        // Off the box — the open internet never gets this grant.
        assert!(!native_local_first_operator(&bare, false));

        // A DECLARED deployment (JWT_SECRET set) never gets it, loopback or not.
        // This is the conjunct that protects the fleet: alpha/alpha-b/prod/
        // staging/staging-read all populate JWT_SECRET, and all sit at the
        // fail-closed `Bootstrap` stage, so stage alone would not have saved them.
        let declared = state_with(
            NetworkStage::Bootstrap,
            Some("a-real-deployment-secret"),
            false,
        );
        assert!(!native_local_first_operator(&declared, true));

        // Once the network declares coordination, the grant retires on its own.
        let coordinated = state_with(NetworkStage::Coordinated, None, false);
        assert!(!native_local_first_operator(&coordinated, true));
    }

    #[test]
    fn the_grant_does_not_depend_on_dev_mode() {
        // The whole point: DEV_MODE is not an auth posture. Flipping it must not
        // move this predicate in either direction.
        let off = state_with(NetworkStage::Bootstrap, None, false);
        let on = state_with(NetworkStage::Bootstrap, None, true);
        assert_eq!(
            native_local_first_operator(&off, true),
            native_local_first_operator(&on, true)
        );
        let declared_off = state_with(NetworkStage::Bootstrap, Some("secret-value-here"), false);
        let declared_on = state_with(NetworkStage::Bootstrap, Some("secret-value-here"), true);
        assert_eq!(
            native_local_first_operator(&declared_off, true),
            native_local_first_operator(&declared_on, true)
        );
    }

    fn anonymous_request() -> Request<()> {
        Request::builder().uri("/hc/admin").body(()).unwrap()
    }

    #[test]
    fn anonymous_remote_caller_is_refused_even_with_dev_mode() {
        // The live fleet shape: DEV_MODE=true, JWT_SECRET declared, stage
        // fail-closed to Bootstrap, caller arriving through an ingress. Before
        // this change the request reached the conductor admin socket UNFILTERED.
        let state = state_with(
            NetworkStage::Bootstrap,
            Some("a-real-deployment-secret"),
            true,
        );
        let err = extract_permission(&state, &anonymous_request(), false)
            .expect_err("an anonymous internet caller must not reach the admin socket");
        assert!(
            err.contains("/hc/connect"),
            "refusal should name the chaperone: {err}"
        );
    }

    #[test]
    fn anonymous_loopback_on_a_declared_doorway_is_refused() {
        // A declared deployment does not hand out the local-first grant just
        // because a hop happened to be loopback (an in-pod sidecar, say).
        let state = state_with(
            NetworkStage::Bootstrap,
            Some("a-real-deployment-secret"),
            true,
        );
        assert!(extract_permission(&state, &anonymous_request(), true).is_err());
    }

    #[test]
    fn anonymous_loopback_on_a_bare_dev_box_is_the_conductor_operator() {
        // Native local-first mode keeps working with no credential configured.
        let state = state_with(NetworkStage::Bootstrap, None, false);
        let level = extract_permission(&state, &anonymous_request(), true)
            .expect("the developer on their own box owns their own conductor");
        assert_eq!(level, PermissionLevel::Admin);
    }

    #[test]
    fn a_keyless_doorway_no_longer_grants_public_to_a_remote_anonymous_caller() {
        // Regression for the SECOND credential-free arm — the
        // `|| !api_validator.is_configured()` disjunct. With no API keys set it
        // returned Ok(Public) to a caller who presented nothing, independently
        // of dev_mode. Closing only the dev_mode arm would have left this open.
        let state = state_with(NetworkStage::Bootstrap, None, false);
        assert!(
            extract_permission(&state, &anonymous_request(), false).is_err(),
            "no API keys configured must not mean no authentication required"
        );
    }
}
