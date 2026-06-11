//! Shared HTTP permission-ladder resolution.
//!
//! This is the single source of truth for "what permission level does this HTTP
//! request carry?" It is consumed by every doorway route that gates on
//! [`PermissionLevel`] — the elohim-agent proxy (Authenticated gate) and the
//! seed/cache-mutation routes (Admin gate via [`require_seed_authority`]).
//!
//! The ladder checks (in order): JWT `Authorization: Bearer …` → `X-API-Key`
//! header → dev-mode fallback → Public. It returns [`PermissionLevel::Public`]
//! (never `Err`) when no auth is found — the *caller* decides whether Public is
//! sufficient. The function only reads request headers; it never touches the
//! body, so it is generic over the body type `B`.

use hyper::Request;
use tracing::info;

use crate::auth::{extract_token_from_header, ApiKeyValidator, JwtValidator, PermissionLevel};
use crate::server::AppState;

/// Extract permission level from HTTP request headers.
///
/// Checks (in order): Authorization header (JWT), X-API-Key header, dev-mode
/// fallback. This is the HTTP equivalent of websocket.rs `extract_permission`,
/// but returns [`PermissionLevel::Public`] instead of `Err` when no auth is
/// found (caller decides whether Public is sufficient).
///
/// Generic over the body type `B` because only headers are inspected — a
/// rejected request must never have its body consumed.
pub(crate) fn extract_http_permission<B>(state: &AppState, req: &Request<B>) -> PermissionLevel {
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
        info!("Dev mode: granting authenticated access for HTTP request");
        return PermissionLevel::Authenticated;
    }

    PermissionLevel::Public
}

/// Validate a bearer JWT against the configured (or dev) validator and extract
/// its permission level. Returns `None` if the token is invalid or the
/// validator cannot be constructed.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::TokenInput;
    use crate::config::Args;
    use clap::Parser;
    use http_body_util::Empty;
    use hyper::body::Bytes;

    /// Build an AppState with the given dev_mode and a configured prod JWT
    /// secret so non-dev JWT validation works.
    fn test_state(dev_mode: bool) -> AppState {
        let mut args = Args::parse_from(["doorway", "--listen", "127.0.0.1:0"]);
        args.dev_mode = dev_mode;
        args.api_key_authenticated = Some("test-auth-key".to_string());
        args.api_key_admin = Some("test-admin-key".to_string());
        args.jwt_secret = Some("test-secret-that-is-at-least-32-characters-long".to_string());
        AppState::new(args)
    }

    /// Build a bearer JWT signed with the same prod secret `test_state` uses,
    /// carrying the requested permission level.
    fn bearer_jwt(level: PermissionLevel) -> String {
        let validator = JwtValidator::new(
            "test-secret-that-is-at-least-32-characters-long".into(),
            3600,
        )
        .unwrap();
        let input = TokenInput {
            human_id: "human-123".into(),
            agent_pub_key: "uhCAk...".into(),
            identifier: "ci@example.com".into(),
            permission_level: level,
            session_id: None,
            doorway_id: None,
            doorway_url: None,
            conductor_id: None,
            installed_app_id: None,
            is_steward: false,
            has_local_conductor: false,
        };
        validator.generate_token(input).unwrap()
    }

    fn req_with_bearer(token: Option<&str>) -> Request<Empty<Bytes>> {
        let mut builder = Request::builder().method("PUT").uri("/admin/seed/blob");
        if let Some(t) = token {
            builder = builder.header(hyper::header::AUTHORIZATION, format!("Bearer {t}"));
        }
        builder.body(Empty::<Bytes>::new()).unwrap()
    }

    #[test]
    fn no_auth_non_dev_is_public() {
        let state = test_state(false);
        let req = req_with_bearer(None);
        assert_eq!(
            extract_http_permission(&state, &req),
            PermissionLevel::Public
        );
    }

    #[test]
    fn no_auth_dev_is_authenticated() {
        let state = test_state(true);
        let req = req_with_bearer(None);
        // Behavior byte-identical to the old elohim_agent helper: dev-mode
        // fallback yields Authenticated, never Admin.
        assert_eq!(
            extract_http_permission(&state, &req),
            PermissionLevel::Authenticated
        );
    }

    #[test]
    fn admin_jwt_resolves_admin() {
        let state = test_state(false);
        let token = bearer_jwt(PermissionLevel::Admin);
        let req = req_with_bearer(Some(&token));
        assert_eq!(
            extract_http_permission(&state, &req),
            PermissionLevel::Admin
        );
    }

    #[test]
    fn authenticated_jwt_resolves_authenticated() {
        let state = test_state(false);
        let token = bearer_jwt(PermissionLevel::Authenticated);
        let req = req_with_bearer(Some(&token));
        assert_eq!(
            extract_http_permission(&state, &req),
            PermissionLevel::Authenticated
        );
    }

    #[test]
    fn invalid_bearer_non_dev_is_public() {
        let state = test_state(false);
        let req = req_with_bearer(Some("not-a-real-jwt"));
        assert_eq!(
            extract_http_permission(&state, &req),
            PermissionLevel::Public
        );
    }
}
