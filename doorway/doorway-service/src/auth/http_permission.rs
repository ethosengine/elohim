//! Shared HTTP permission-ladder resolution.
//!
//! This is the single source of truth for "what permission level does this HTTP
//! request carry?" It is consumed by every doorway route that gates on
//! [`PermissionLevel`] — the elohim-agent proxy (Authenticated gate) and the
//! seed/cache-mutation routes (Admin gate via [`require_seed_authority`]).
//!
//! The ladder checks (in order): JWT `Authorization: Bearer …` → `X-API-Key`
//! header → on-the-box caller while the DECLARED stage is pre-coordination →
//! Public. It returns [`PermissionLevel::Public`]
//! (never `Err`) when no auth is found — the *caller* decides whether Public is
//! sufficient. The function only reads request headers; it never touches the
//! body, so it is generic over the body type `B`.

use hyper::Request;
use seam_contracts::freshness::NetworkStage;
use tracing::info;

use crate::auth::{extract_token_from_header, ApiKeyValidator, JwtValidator, PermissionLevel};
use crate::server::AppState;

/// Extract permission level from HTTP request headers.
///
/// Checks (in order): Authorization header (JWT), X-API-Key header, then the
/// pre-coordination on-the-box affordance. This is the HTTP equivalent of
/// websocket.rs `extract_permission`, but returns [`PermissionLevel::Public`]
/// instead of `Err` when no auth is found (caller decides whether Public is
/// sufficient).
///
/// `peer_is_loopback` MUST come from the accepted socket's peer address
/// (`addr.ip().is_loopback()`), never from a header — see the body.
///
/// Generic over the body type `B` because only headers are inspected — a
/// rejected request must never have its body consumed.
pub(crate) fn extract_http_permission<B>(
    state: &AppState,
    req: &Request<B>,
    peer_is_loopback: bool,
) -> PermissionLevel {
    resolve_http_permission(state, req, peer_is_loopback, false)
}

/// The ladder, with the JWT leg selectable. `operator_actuator` = true makes
/// the JWT leg validate only tokens THIS doorway issued, with this doorway's
/// own configuration, and fail closed when no signing secret is configured.
fn resolve_http_permission<B>(
    state: &AppState,
    req: &Request<B>,
    peer_is_loopback: bool,
    operator_actuator: bool,
) -> PermissionLevel {
    // Try JWT from Authorization header
    let auth_header = req
        .headers()
        .get(hyper::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    if let Some(token) = extract_token_from_header(auth_header) {
        let level = if operator_actuator {
            validate_local_jwt_token(state, token)
        } else {
            validate_jwt_token(state, token)
        };
        if let Some(level) = level {
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

    // THE CALLER IS ON THE BOX, while the DECLARED stage is pre-coordination.
    //
    // Mirrors authority (1) of `require_seed_authority` (routes/seed.rs): the
    // affordance is priced against `ELOHIM_NETWORK_STAKES` (resolved once at
    // boot into `AppState::network_stage`, fail-closed to `Bootstrap`) and
    // conjoined with a peer address taken from the ACCEPTED SOCKET — never
    // `X-Forwarded-For`. An attacker sets headers; they do not set the kernel's
    // notion of who connected.
    //
    // WHY NOT `dev_mode`: that flag is `"true"` on every deployed manifest
    // (alpha, alpha-b, prod, staging, staging-read), so a gate keyed on it is
    // ungated in practice — it promoted every anonymous caller on the open web
    // to `Authenticated`. The canon states the rule:
    // `2026-08-25-doorway-auth-posture-declared-stage.md`.
    //
    // WHAT THIS CHANGES: behind a cluster ingress the peer is the ingress pod,
    // so the fleet now resolves anonymous callers to `Public`. The only route
    // that consumes an `Authenticated` verdict is the elohim-agent invocation
    // proxy, whose own contract says it should refuse exactly these callers —
    // "compute is shared commons, but only for real people in the network, not
    // anonymous traffic" (routes/elohim_agent.rs). `/health` stays public
    // (bypassed before this gate) and a signed-in browser still sends a bearer
    // token, so the graduated user is unaffected. On a developer's box the peer
    // is 127.0.0.1, so local dev and the mesh are unchanged.
    //
    // THE EXPIRY IS DESIGNED: declaring `coordinated` retires this affordance by
    // itself, with no flag to remember to unset.
    if state.network_stage < NetworkStage::Coordinated && peer_is_loopback {
        info!("Pre-coordination loopback caller: granting authenticated access");
        return PermissionLevel::Authenticated;
    }

    PermissionLevel::Public
}

/// Is this method+path one of the operator actuators that mutate hosting,
/// graduation, federation or route/bundle state and therefore need Admin?
///
/// An EXACT mirror of the dispatch arms in `server/http.rs` (same prefixes, same
/// methods), so a path the router would not dispatch to an actuator is never
/// gated here, and every GET stays public peer telemetry.
pub(crate) fn is_operator_actuator(method: &hyper::Method, path: &str) -> bool {
    use hyper::Method;
    match (method, path) {
        (&Method::POST, "/admin/hosted-users")
        | (&Method::POST, "/admin/federation/peers")
        | (&Method::DELETE, "/admin/federation/peers")
        | (&Method::POST, "/admin/federation/peers/refresh")
        | (&Method::POST, "/admin/steward-peers/refresh")
        | (&Method::POST, "/admin/ssr-bundle/refresh") => true,
        (&Method::DELETE, p) => p.starts_with("/admin/hosted-users/"),
        (&Method::POST, p) => p.starts_with("/admin/graduation/force/"),
        _ => false,
    }
}

/// Require Admin authority on a request: a Bearer Admin JWT or the configured
/// `API_KEY_ADMIN` via `X-API-Key`, resolved by [`extract_http_permission`].
///
/// 401 when no valid credential resolved (nothing presented, or invalid);
/// 403 when a caller resolved but below Admin. Never consults `dev_mode`.
#[allow(clippy::result_large_err)]
pub(crate) fn require_admin_permission<B>(
    state: &AppState,
    req: &Request<B>,
    peer_is_loopback: bool,
) -> Result<(), hyper::Response<http_body_util::Full<hyper::body::Bytes>>> {
    let level = resolve_http_permission(state, req, peer_is_loopback, true);
    if level >= PermissionLevel::Admin {
        return Ok(());
    }
    let (status, message) = if level >= PermissionLevel::Authenticated {
        (
            hyper::StatusCode::FORBIDDEN,
            "Admin permission required for this operator action",
        )
    } else {
        (
            hyper::StatusCode::UNAUTHORIZED,
            "Authentication required for this operator action",
        )
    };
    let body = serde_json::json!({ "success": false, "error": message });
    Err(hyper::Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(http_body_util::Full::new(hyper::body::Bytes::from(
            body.to_string(),
        )))
        .unwrap())
}

/// Validate a bearer JWT for an OPERATOR ACTUATOR: only a token this doorway
/// issued, checked with the same validator its own auth path uses
/// ([`AppState::federation_jwt_validator`] — own doorway id, node verifying
/// key), and only when a signing secret is actually configured.
///
/// Fails closed (`None`) when `JWT_SECRET` is absent or blank: the fallback
/// `JwtValidator::new_dev()` secret is public, so a token "verified" with it
/// proves nothing and must never confer Admin. Tokens whose `kid` names any
/// other doorway (federated JWKS) are refused — they authenticate a foreign
/// operator, not a local one. Never consults `dev_mode`.
fn validate_local_jwt_token(state: &AppState, token: &str) -> Option<PermissionLevel> {
    state.args.configured_jwt_secret()?;
    let header = jsonwebtoken::decode_header(token).ok()?;
    if let Some(kid) = header.kid.as_deref() {
        if state.args.doorway_id.as_deref() != Some(kid) {
            return None;
        }
    }
    let jwt = state.federation_jwt_validator()?;
    let result = jwt.verify_token(token);
    if result.valid {
        result.claims.map(|c| c.permission_level)
    } else {
        None
    }
}

/// Validate a bearer JWT against the configured (or dev) validator and extract
/// its permission level. Returns `None` if the token is invalid or the
/// validator cannot be constructed.
fn validate_jwt_token(state: &AppState, token: &str) -> Option<PermissionLevel> {
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
            extract_http_permission(&state, &req, false),
            PermissionLevel::Public
        );
    }

    /// THE SECURITY PROPERTY. `dev_mode` is `"true"` on every deployed manifest,
    /// so before the stage derivation this returned `Authenticated` to any
    /// anonymous caller on the open web. Behind an ingress the peer is the
    /// ingress pod, never loopback — so the fleet now resolves `Public`.
    #[test]
    fn remote_anonymous_is_public_even_with_dev_mode() {
        let state = test_state(true);
        let req = req_with_bearer(None);
        assert_eq!(
            extract_http_permission(&state, &req, false),
            PermissionLevel::Public,
            "a remote anonymous caller must not be promoted by a mode flag"
        );
    }

    /// The developer's box keeps working with no credential: pre-coordination
    /// stage (the fail-closed `Bootstrap` default) plus an on-the-box peer.
    /// Yields `Authenticated`, never `Admin`.
    #[test]
    fn loopback_anonymous_pre_coordination_is_authenticated() {
        let state = test_state(true);
        let req = req_with_bearer(None);
        assert_eq!(
            extract_http_permission(&state, &req, true),
            PermissionLevel::Authenticated
        );
    }

    /// The affordance is not keyed on `dev_mode` at all: an on-the-box caller
    /// pre-coordination is admitted even with the flag off.
    #[test]
    fn loopback_grant_does_not_depend_on_dev_mode() {
        let state = test_state(false);
        let req = req_with_bearer(None);
        assert_eq!(
            extract_http_permission(&state, &req, true),
            PermissionLevel::Authenticated
        );
    }

    /// THE DESIGNED EXPIRY. Declaring `coordinated` retires the on-the-box
    /// affordance by itself — no flag to remember to unset.
    #[test]
    fn coordinated_stage_retires_the_loopback_grant() {
        let mut state = test_state(true);
        state.network_stage = NetworkStage::Coordinated;
        let req = req_with_bearer(None);
        assert_eq!(
            extract_http_permission(&state, &req, true),
            PermissionLevel::Public
        );
    }

    #[test]
    fn admin_jwt_resolves_admin() {
        let state = test_state(false);
        let token = bearer_jwt(PermissionLevel::Admin);
        let req = req_with_bearer(Some(&token));
        assert_eq!(
            extract_http_permission(&state, &req, false),
            PermissionLevel::Admin
        );
    }

    #[test]
    fn authenticated_jwt_resolves_authenticated() {
        let state = test_state(false);
        let token = bearer_jwt(PermissionLevel::Authenticated);
        let req = req_with_bearer(Some(&token));
        assert_eq!(
            extract_http_permission(&state, &req, false),
            PermissionLevel::Authenticated
        );
    }

    #[test]
    fn invalid_bearer_non_dev_is_public() {
        let state = test_state(false);
        let req = req_with_bearer(Some("not-a-real-jwt"));
        assert_eq!(
            extract_http_permission(&state, &req, false),
            PermissionLevel::Public
        );
    }

    const ACTUATORS: [(&str, &str); 8] = [
        ("POST", "/admin/hosted-users"),
        ("DELETE", "/admin/hosted-users/uhCAkagent"),
        ("POST", "/admin/graduation/force/uhCAkagent"),
        ("POST", "/admin/federation/peers"),
        ("DELETE", "/admin/federation/peers"),
        ("POST", "/admin/federation/peers/refresh"),
        ("POST", "/admin/steward-peers/refresh"),
        ("POST", "/admin/ssr-bundle/refresh"),
    ];

    fn actuator_req(method: &str, path: &str) -> hyper::http::request::Builder {
        Request::builder().method(method).uri(path)
    }

    #[test]
    fn actuator_predicate_names_exactly_the_eight_and_no_reads() {
        for (m, p) in ACTUATORS {
            let method = hyper::Method::from_bytes(m.as_bytes()).unwrap();
            assert!(is_operator_actuator(&method, p), "{m} {p} must be gated");
        }
        for (m, p) in [
            ("GET", "/admin/hosted-users"),
            ("GET", "/admin/federation/peers"),
            ("GET", "/admin/graduation/pending"),
            ("GET", "/admin/capabilities"),
            ("GET", "/admin/ws"),
            ("POST", "/admin/conductors/assign"),
            ("POST", "/admin/cache/warm"),
            ("PUT", "/admin/dev/shed"),
            ("GET", "/admin/steward-peers/refresh"),
        ] {
            let method = hyper::Method::from_bytes(m.as_bytes()).unwrap();
            assert!(
                !is_operator_actuator(&method, p),
                "{m} {p} must not be gated here"
            );
        }
    }

    #[test]
    fn actuators_401_without_credential_403_below_admin_pass_for_admin() {
        // dev_mode both ways: it must change nothing.
        for dev_mode in [false, true] {
            let state = test_state(dev_mode);
            let auth = bearer_jwt(PermissionLevel::Authenticated);
            let admin = bearer_jwt(PermissionLevel::Admin);
            for (m, p) in ACTUATORS {
                let status = |r: Request<Empty<Bytes>>| {
                    require_admin_permission(&state, &r, false)
                        .err()
                        .map(|resp| resp.status().as_u16())
                };
                let none = actuator_req(m, p).body(Empty::<Bytes>::new()).unwrap();
                assert_eq!(status(none), Some(401), "{m} {p} anon dev={dev_mode}");
                let bad = actuator_req(m, p)
                    .header("authorization", "Bearer not-a-jwt")
                    .body(Empty::<Bytes>::new())
                    .unwrap();
                assert_eq!(status(bad), Some(401), "{m} {p} bad jwt dev={dev_mode}");
                let wrongkey = actuator_req(m, p)
                    .header("x-api-key", "wrong")
                    .body(Empty::<Bytes>::new())
                    .unwrap();
                assert_eq!(
                    status(wrongkey),
                    Some(401),
                    "{m} {p} wrong key dev={dev_mode}"
                );
                let low = actuator_req(m, p)
                    .header("authorization", format!("Bearer {auth}"))
                    .body(Empty::<Bytes>::new())
                    .unwrap();
                assert_eq!(
                    status(low),
                    Some(403),
                    "{m} {p} ordinary jwt dev={dev_mode}"
                );
                let lowkey = actuator_req(m, p)
                    .header("x-api-key", "test-auth-key")
                    .body(Empty::<Bytes>::new())
                    .unwrap();
                assert_eq!(
                    status(lowkey),
                    Some(403),
                    "{m} {p} authenticated key dev={dev_mode}"
                );
                let ok_jwt = actuator_req(m, p)
                    .header("authorization", format!("Bearer {admin}"))
                    .body(Empty::<Bytes>::new())
                    .unwrap();
                assert_eq!(status(ok_jwt), None, "{m} {p} admin jwt dev={dev_mode}");
                let ok_key = actuator_req(m, p)
                    .header("x-api-key", "test-admin-key")
                    .body(Empty::<Bytes>::new())
                    .unwrap();
                assert_eq!(status(ok_key), None, "{m} {p} admin key dev={dev_mode}");
            }
        }
    }

    #[test]
    fn actuator_denial_is_a_json_error_body() {
        let state = test_state(true);
        let req = actuator_req("POST", "/admin/hosted-users")
            .body(Empty::<Bytes>::new())
            .unwrap();
        let resp = require_admin_permission(&state, &req, false).unwrap_err();
        assert_eq!(resp.headers()["content-type"], "application/json");
    }

    // ── Operator-actuator guard: local tokens only, fail closed ───────────

    fn token_input(level: PermissionLevel) -> TokenInput {
        TokenInput {
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
        }
    }

    fn creds_req(token: Option<&str>, key: Option<&str>) -> Request<Empty<Bytes>> {
        let mut b = Request::builder()
            .method("POST")
            .uri("/admin/ssr-bundle/refresh");
        if let Some(t) = token {
            b = b.header(hyper::header::AUTHORIZATION, format!("Bearer {t}"));
        }
        if let Some(k) = key {
            b = b.header("x-api-key", k);
        }
        b.body(Empty::<Bytes>::new()).unwrap()
    }

    fn coordinated_state(secret: Option<&str>, doorway_id: Option<&str>) -> AppState {
        let mut state = test_state(false);
        state.args.jwt_secret = secret.map(str::to_string);
        state.args.doorway_id = doorway_id.map(str::to_string);
        state.network_stage = NetworkStage::Coordinated;
        state
    }

    fn status_of(r: Result<(), hyper::Response<http_body_util::Full<Bytes>>>) -> u16 {
        match r {
            Ok(()) => 200,
            Err(resp) => resp.status().as_u16(),
        }
    }

    #[test]
    fn actuator_guard_accepts_a_kid_bearing_token_this_doorway_minted() {
        let state = coordinated_state(
            Some("test-secret-that-is-at-least-32-characters-long"),
            Some("self-doorway"),
        );
        // The real minting path: the validator the doorway itself mints with.
        let minter = state.federation_jwt_validator().unwrap();
        let admin = minter
            .generate_token(token_input(PermissionLevel::Admin))
            .unwrap();
        let header = jsonwebtoken::decode_header(&admin).unwrap();
        assert_eq!(
            header.kid.as_deref(),
            Some("self-doorway"),
            "token carries kid"
        );
        let req = creds_req(Some(&admin), None);
        assert_eq!(
            status_of(require_admin_permission(&state, &req, false)),
            200
        );

        let low = minter
            .generate_token(token_input(PermissionLevel::Authenticated))
            .unwrap();
        let req = creds_req(Some(&low), None);
        assert_eq!(
            status_of(require_admin_permission(&state, &req, false)),
            403
        );
    }

    #[test]
    fn actuator_guard_refuses_a_token_from_a_foreign_doorway() {
        let state = coordinated_state(
            Some("test-secret-that-is-at-least-32-characters-long"),
            Some("self-doorway"),
        );
        let foreign = JwtValidator::new(
            "test-secret-that-is-at-least-32-characters-long".into(),
            3600,
        )
        .unwrap()
        .with_doorway_id("other-doorway");
        let admin = foreign
            .generate_token(token_input(PermissionLevel::Admin))
            .unwrap();
        let header = jsonwebtoken::decode_header(&admin).unwrap();
        assert_eq!(header.kid.as_deref(), Some("other-doorway"));
        let req = creds_req(Some(&admin), None);
        assert_eq!(
            status_of(require_admin_permission(&state, &req, false)),
            401
        );
    }

    #[test]
    fn actuator_guard_fails_closed_on_jwt_without_a_configured_secret() {
        for secret in [None, Some(""), Some("   ")] {
            let state = coordinated_state(secret, None);
            // Anyone can mint this: the dev validator's secret is public.
            let forged = JwtValidator::new_dev()
                .generate_token(token_input(PermissionLevel::Admin))
                .unwrap();
            let req = creds_req(Some(&forged), None);
            assert_eq!(
                status_of(require_admin_permission(&state, &req, false)),
                401,
                "forged admin jwt with secret {secret:?}"
            );
            // The configured admin API key still works.
            let req = creds_req(None, Some("test-admin-key"));
            assert_eq!(
                status_of(require_admin_permission(&state, &req, false)),
                200
            );
        }
    }
}
