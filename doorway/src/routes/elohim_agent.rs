//! Elohim Agent SDK routes — transparent proxy to elohim-agent-sdk sidecar.
//!
//! Auth policy: health probes are public, invocation requires Authenticated.
//! Compute is a commons resource — any authenticated person in the network
//! can invoke the elohim agent. Quota/reach governance is handled by the
//! sidecar's budget enforcer (alpha) and will be protocol-governed in shefa.

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::auth::{extract_token_from_header, ApiKeyValidator, JwtValidator, PermissionLevel};
use crate::server::AppState;

pub async fn handle_elohim_agent_request(
    req: Request<Incoming>,
    state: Arc<AppState>,
    path: &str,
) -> Response<Full<Bytes>> {
    let sidecar_path = path.strip_prefix("/api/v1/elohim").unwrap_or(path);

    // Health probes are public (NativeBackend.isAvailable() needs unauthenticated access)
    if sidecar_path.ends_with("/health") {
        let agent_url = resolve_agent_url(&state).await;
        return forward_to_agent(req, &agent_url, sidecar_path).await;
    }

    // Invocation requires authentication — compute is shared commons,
    // but only for real people in the network, not anonymous traffic.
    let permission = extract_http_permission(&state, &req);
    if permission < PermissionLevel::Authenticated {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(
                r#"{"error": "Authentication required for elohim agent invocation"}"#,
            )))
            .unwrap();
    }

    let agent_url = resolve_agent_url(&state).await;
    forward_to_agent(req, &agent_url, sidecar_path).await
}

/// Extract permission level from HTTP request headers.
///
/// Checks (in order): Authorization header (JWT), X-API-Key header, dev-mode fallback.
/// This is the HTTP equivalent of websocket.rs `extract_permission`, but returns
/// `PermissionLevel::Public` instead of `Err` when no auth is found (caller decides
/// whether Public is sufficient).
fn extract_http_permission(state: &AppState, req: &Request<Incoming>) -> PermissionLevel {
    // Try JWT from Authorization header
    let auth_header = req
        .headers()
        .get(hyper::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    if let Some(token) = extract_token_from_header(auth_header) {
        if let Some(level) = validate_jwt_token(state, token) {
            return level;
        }
    }

    // Try API key from X-API-Key header
    let api_key = req.headers().get("x-api-key").and_then(|v| v.to_str().ok());
    let api_validator = ApiKeyValidator::new(
        state.args.api_key_authenticated.clone(),
        state.args.api_key_admin.clone(),
    );
    if let Some(permission) = api_validator.validate(api_key) {
        if api_key.is_some() {
            return permission;
        }
    }

    // Dev mode: allow authenticated-level access for local development
    if state.args.dev_mode {
        info!("Dev mode: granting authenticated access for elohim agent");
        return PermissionLevel::Authenticated;
    }

    PermissionLevel::Public
}

fn validate_jwt_token(state: &AppState, token: &str) -> Option<PermissionLevel> {
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

/// Resolve which agent URL to use — try elohim-node first, fall back to sidecar.
async fn resolve_agent_url(state: &AppState) -> String {
    if !state.args.elohim_node_url.is_empty() {
        let node_health = format!(
            "{}/elohim/health",
            state.args.elohim_node_url.trim_end_matches('/')
        );
        match reqwest::get(&node_health).await {
            Ok(resp) if resp.status().is_success() => {
                debug!(
                    "elohim-node healthy, routing to {}",
                    state.args.elohim_node_url
                );
                return state.args.elohim_node_url.clone();
            }
            _ => {
                debug!("elohim-node unreachable, falling back to sidecar");
            }
        }
    }
    state.args.elohim_agent_url.clone()
}

async fn forward_to_agent(
    req: Request<Incoming>,
    agent_url: &str,
    path: &str,
) -> Response<Full<Bytes>> {
    let agent_endpoint = format!("{}{}", agent_url.trim_end_matches('/'), path);

    let query = req.uri().query();
    let full_url = match query {
        Some(q) => format!("{agent_endpoint}?{q}"),
        None => agent_endpoint,
    };

    let method = req.method().clone();
    debug!(method = %method, url = %full_url, "Forwarding request to elohim-agent-sdk sidecar");

    let client = reqwest::Client::new();
    let mut builder = match method {
        Method::GET => client.get(&full_url),
        Method::POST => client.post(&full_url),
        Method::PUT => client.put(&full_url),
        Method::DELETE => client.delete(&full_url),
        Method::HEAD => client.head(&full_url),
        Method::PATCH => client.patch(&full_url),
        _ => {
            return Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap();
        }
    };

    if let Some(ct) = req.headers().get("content-type") {
        if let Ok(ct_str) = ct.to_str() {
            builder = builder.header("Content-Type", ct_str);
        }
    }

    if let Some(auth) = req.headers().get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            builder = builder.header("Authorization", auth_str);
        }
    }

    if matches!(method, Method::POST | Method::PUT | Method::PATCH) {
        match req.collect().await {
            Ok(collected) => {
                builder = builder.body(collected.to_bytes().to_vec());
            }
            Err(e) => {
                warn!(error = %e, "Failed to read request body");
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "Failed to read request body: {e}"}}"#
                    ))))
                    .unwrap();
            }
        }
    }

    match builder.send().await {
        Ok(response) => {
            let status = response.status();
            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/json")
                .to_string();
            let budget_remaining = response
                .headers()
                .get("x-elohim-budget-remaining")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            match response.bytes().await {
                Ok(body) => {
                    let mut resp = Response::builder()
                        .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK))
                        .header("Content-Type", content_type)
                        .header("Cross-Origin-Resource-Policy", "cross-origin");
                    if let Some(budget) = budget_remaining {
                        resp = resp.header("X-Elohim-Budget-Remaining", budget);
                    }
                    resp.body(Full::new(Bytes::from(body.to_vec()))).unwrap()
                }
                Err(e) => {
                    warn!(error = %e, "Failed to read agent response body");
                    Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .header("Content-Type", "application/json")
                        .body(Full::new(Bytes::from(format!(
                            r#"{{"error": "Failed to read agent response: {e}"}}"#
                        ))))
                        .unwrap()
                }
            }
        }
        Err(e) => {
            warn!(error = %e, path = %path, "Failed to forward request to elohim-agent-sdk sidecar");
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error": "Failed to connect to agent sidecar: {e}"}}"#
                ))))
                .unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Args;
    use clap::Parser;

    fn test_state(dev_mode: bool) -> AppState {
        let mut args = Args::parse_from(["doorway", "--listen", "127.0.0.1:0"]);
        args.dev_mode = dev_mode;
        args.api_key_authenticated = Some("test-auth-key".to_string());
        args.api_key_admin = Some("test-admin-key".to_string());
        AppState::new(args)
    }

    #[test]
    fn test_path_mapping() {
        let path = "/api/v1/elohim/invoke";
        let sidecar_path = path.strip_prefix("/api/v1/elohim").unwrap_or(path);
        assert_eq!(sidecar_path, "/invoke");
    }

    #[test]
    fn test_health_path_mapping() {
        let path = "/api/v1/elohim/invoke/health";
        let sidecar_path = path.strip_prefix("/api/v1/elohim").unwrap_or(path);
        assert_eq!(sidecar_path, "/invoke/health");
    }

    #[test]
    fn test_health_path_is_public() {
        let path = "/api/v1/elohim/invoke/health";
        let sidecar_path = path.strip_prefix("/api/v1/elohim").unwrap_or(path);
        assert!(sidecar_path.ends_with("/health"));
    }

    #[test]
    fn test_url_built_with_query() {
        let base = "http://localhost:8095";
        let path = "/invoke";
        let query = "dry_run=true";
        let url = format!("{}{}?{}", base.trim_end_matches('/'), path, query);
        assert_eq!(url, "http://localhost:8095/invoke?dry_run=true");
    }

    #[test]
    fn test_url_built_without_query() {
        let base = "http://localhost:8095/";
        let path = "/invoke";
        let url = format!("{}{}", base.trim_end_matches('/'), path);
        assert_eq!(url, "http://localhost:8095/invoke");
    }

    #[test]
    fn test_api_key_validator_rejects_no_key_when_configured() {
        // Simulates the auth logic: when API keys are configured but no key provided,
        // the validator returns Public — which is below Authenticated threshold
        let validator =
            ApiKeyValidator::new(Some("test-auth-key".into()), Some("test-admin-key".into()));
        let result = validator.validate(None);
        assert_eq!(result, Some(PermissionLevel::Public));
        // Public is below the Authenticated gate
        assert!(PermissionLevel::Public < PermissionLevel::Authenticated);
    }

    #[test]
    fn test_api_key_validator_accepts_auth_key() {
        let validator =
            ApiKeyValidator::new(Some("test-auth-key".into()), Some("test-admin-key".into()));
        let result = validator.validate(Some("test-auth-key"));
        assert_eq!(result, Some(PermissionLevel::Authenticated));
        assert!(PermissionLevel::Authenticated >= PermissionLevel::Authenticated);
    }

    #[test]
    fn test_api_key_validator_accepts_admin_key() {
        let validator =
            ApiKeyValidator::new(Some("test-auth-key".into()), Some("test-admin-key".into()));
        let result = validator.validate(Some("test-admin-key"));
        assert_eq!(result, Some(PermissionLevel::Admin));
        // Admin >= Authenticated — compute is shared, admin can use it too
        assert!(PermissionLevel::Admin >= PermissionLevel::Authenticated);
    }

    #[test]
    fn test_api_key_validator_rejects_wrong_key() {
        let validator =
            ApiKeyValidator::new(Some("test-auth-key".into()), Some("test-admin-key".into()));
        let result = validator.validate(Some("wrong-key"));
        assert_eq!(result, None);
    }

    #[test]
    fn test_dev_mode_state_enables_dev_mode() {
        let state = test_state(true);
        assert!(state.args.dev_mode);
    }

    #[test]
    fn test_prod_mode_state_disables_dev_mode() {
        let state = test_state(false);
        assert!(!state.args.dev_mode);
    }

    #[test]
    fn test_elohim_node_url_default_is_empty() {
        let state = test_state(false);
        assert!(state.args.elohim_node_url.is_empty());
    }

    #[test]
    fn test_sidecar_url_used_when_node_url_empty() {
        let state = test_state(false);
        // When elohim_node_url is empty, resolve_agent_url should use elohim_agent_url
        assert!(state.args.elohim_node_url.is_empty());
        assert!(!state.args.elohim_agent_url.is_empty());
    }
}
