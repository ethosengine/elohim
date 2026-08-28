//! Auth discovery — `GET /.well-known/elohim-auth`.
//!
//! The one thing a page always knows without being told is the origin it was
//! served from. This document turns that single fact into everything a client
//! needs to authenticate a human, so an app carries no auth configuration at
//! all: no login path, no endpoint list, no client identifier.
//!
//! # Why `.well-known/` and not `/auth/config`
//!
//! `/auth/*` paths the doorway does not own fall through to the EPR router and
//! are answered with the SPA shell. Measured on the local mesh 2026-08-28:
//! `GET /auth/config` returns `<!doctype html>` with HTTP **200**. A client
//! probing there gets a JSON parse error rather than a branchable 404 — the
//! same misdiagnosis shape as the `/auth/portal` incident. `/.well-known/` is
//! already in `is_service_path`, so an unknown path under it 404s honestly.
//!
//! # Why every value is a RELATIVE path
//!
//! A discovery document that could name another origin would be an open-redirect
//! primitive: whoever answered it could point a Login button at an attacker's
//! portal. This document is structurally incapable of that — it emits paths, and
//! the client resolves them against the origin it already trusted enough to load
//! from. There is no field in which a foreign origin can be expressed.
//!
//! That is also why `portalHostUrl` is NOT here. A graduated steward's portal
//! genuinely lives on another origin, but it is per-human, arrives on `/auth/me`,
//! and must be validated against that human's registered PortalHost — it is
//! session state, not public configuration, and mixing it in would re-open the
//! hole this design closes.

use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use serde::Serialize;

use crate::server::AppState;

/// The auth endpoints this doorway serves, as origin-relative paths.
///
/// Every path here is exact-matched by `AUTH_OWNED_PATHS` in `server/http.rs`;
/// a path that stops being owned must be removed from BOTH or the document
/// starts advertising a route that answers with the SPA shell.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthEndpoints {
    pub register: &'static str,
    pub login: &'static str,
    pub logout: &'static str,
    pub refresh: &'static str,
    pub me: &'static str,
    /// RFC-6749 authorization endpoint.
    pub authorize: &'static str,
    /// RFC-6749 token endpoint.
    pub token: &'static str,
    /// Mints the opaque single-use token that carries a session to another app.
    pub session_token: &'static str,
    /// Redeems a token minted by `session_token`.
    pub exchange_session: &'static str,
    /// Where a graduated steward's own portal is discovered, per human.
    pub portal_host: &'static str,
}

impl AuthEndpoints {
    const fn current() -> Self {
        Self {
            register: "/auth/register",
            login: "/auth/login",
            logout: "/auth/logout",
            refresh: "/auth/refresh",
            me: "/auth/me",
            authorize: "/auth/authorize",
            token: "/auth/token",
            session_token: "/auth/session-token",
            exchange_session: "/auth/exchange-session",
            portal_host: "/auth/portal-host",
        }
    }
}

/// Response body for `GET /.well-known/elohim-auth`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthDiscovery {
    /// Document shape version. Bump when a client could misread an older body.
    pub version: u32,
    /// Which doorway answered — lets a client notice it moved between origins.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doorway_id: Option<String>,
    /// Where to SEND a human to sign in. Origin-relative by construction.
    pub portal: &'static str,
    pub endpoints: AuthEndpoints,
}

/// The doorway-hosted sign-in portal, served by doorway-app under `/threshold/*`.
const PORTAL_PATH: &str = "/threshold/login";

/// `GET /.well-known/elohim-auth`
///
/// Unauthenticated by design: it names public endpoints and no secrets, and a
/// human who cannot yet authenticate is exactly who needs to read it.
pub fn handle_auth_discovery(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let body = AuthDiscovery {
        version: 1,
        doorway_id: state.args.doorway_id.clone(),
        portal: PORTAL_PATH,
        endpoints: AuthEndpoints::current(),
    };

    match serde_json::to_string_pretty(&body) {
        Ok(json) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            // Short: a doorway that gains a portal should not be shadowed by a
            // client's cached "no portal" answer for an hour.
            .header("Cache-Control", "public, max-age=300")
            .body(Full::new(Bytes::from(json)))
            .unwrap(),
        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(format!(
                r#"{{"error": "Serialization failed: {e}"}}"#
            ))))
            .unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> serde_json::Value {
        let d = AuthDiscovery {
            version: 1,
            doorway_id: Some("alpha-elohim-host".to_string()),
            portal: PORTAL_PATH,
            endpoints: AuthEndpoints::current(),
        };
        serde_json::to_value(&d).expect("discovery serializes")
    }

    /// The whole security property in one test: nothing in this document can
    /// name another origin, so it cannot become an open-redirect primitive.
    #[test]
    fn every_advertised_location_is_origin_relative() {
        fn assert_relative(v: &serde_json::Value, path: &str) {
            match v {
                serde_json::Value::String(s) => {
                    // Only check location-shaped fields; doorwayId is a name.
                    if s.starts_with('/') || s.contains("://") {
                        assert!(
                            s.starts_with('/') && !s.starts_with("//"),
                            "{path} advertises a non-relative location: {s:?} — a discovery \
                             document that can name another origin is an open redirect"
                        );
                    }
                }
                serde_json::Value::Object(m) => {
                    for (k, sub) in m {
                        assert_relative(sub, &format!("{path}.{k}"));
                    }
                }
                _ => {}
            }
        }
        assert_relative(&doc(), "$");
    }

    /// A protocol-relative URL (`//evil.tld/x`) is the classic bypass of a
    /// "starts with /" check — pinned so a future edit cannot slip one in.
    #[test]
    fn a_protocol_relative_location_would_be_rejected() {
        let hostile = serde_json::json!({ "portal": "//evil.tld/login" });
        let caught = std::panic::catch_unwind(|| {
            let s = hostile["portal"].as_str().unwrap();
            assert!(s.starts_with('/') && !s.starts_with("//"));
        });
        assert!(caught.is_err(), "the relative-location check must reject //host");
    }

    #[test]
    fn the_portal_is_the_doorway_hosted_sign_in_path() {
        assert_eq!(doc()["portal"], "/threshold/login");
    }

    #[test]
    fn the_document_carries_the_endpoints_a_client_cannot_guess() {
        let d = doc();
        let e = &d["endpoints"];
        // camelCase on the wire — the client is TypeScript.
        for key in [
            "register",
            "login",
            "logout",
            "refresh",
            "me",
            "authorize",
            "token",
            "sessionToken",
            "exchangeSession",
            "portalHost",
        ] {
            assert!(e.get(key).is_some(), "discovery is missing endpoint {key}");
        }
    }

    /// A missing doorway_id is omitted rather than serialized as null, so a
    /// client can branch on presence without a null check.
    #[test]
    fn an_absent_doorway_id_is_omitted() {
        let d = AuthDiscovery {
            version: 1,
            doorway_id: None,
            portal: PORTAL_PATH,
            endpoints: AuthEndpoints::current(),
        };
        let v = serde_json::to_value(&d).unwrap();
        assert!(v.get("doorwayId").is_none());
    }
}
