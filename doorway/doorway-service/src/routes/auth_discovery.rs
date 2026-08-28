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
use sha2::{Digest, Sha256};

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
    /// Every path this document advertises, for the symmetry guard in
    /// `server/http.rs` that asserts each one is an owned auth path. Kept beside
    /// the fields so adding an endpoint without adding it here is visible.
    pub fn paths(&self) -> [&'static str; 10] {
        [
            self.register,
            self.login,
            self.logout,
            self.refresh,
            self.me,
            self.authorize,
            self.token,
            self.session_token,
            self.exchange_session,
            self.portal_host,
        ]
    }

    /// Public so the schema-contract test can build the same document the
    /// handler serves, rather than a hand-copied stand-in of it.
    pub const fn current() -> Self {
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

/// Does an `If-None-Match` header match our current ETag?
///
/// The header is a comma-separated LIST (RFC 9110 §13.1.2) and `*` matches any
/// current representation, so a naive string equality answers 200 to a
/// well-formed conditional request and the validator silently never fires.
fn etag_matches(if_none_match: &str, current: &str) -> bool {
    if_none_match
        .split(',')
        .map(str::trim)
        .any(|candidate| candidate == "*" || candidate == current)
}

/// `GET /.well-known/elohim-auth`
///
/// Unauthenticated by design: it names public endpoints and no secrets, and a
/// human who cannot yet authenticate is exactly who needs to read it.
pub fn handle_auth_discovery(
    state: Arc<AppState>,
    if_none_match: Option<&str>,
) -> Response<Full<Bytes>> {
    let body = AuthDiscovery {
        version: 1,
        doorway_id: state.args.doorway_id.clone(),
        portal: PORTAL_PATH,
        endpoints: AuthEndpoints::current(),
    };

    match serde_json::to_string_pretty(&body) {
        Ok(json) => {
            // Revalidate-first with an ETag over the body, matching
            // /chrome/omni-element.js on this same doorway. A cached
            // "here is where you sign in" answer that outlives a portal move is
            // exactly the upgrade this document exists to make cheap — so
            // clients re-ask every time and skip the body when nothing changed.
            let etag = format!("\"{:x}\"", Sha256::digest(json.as_bytes()));

            // Honour the validator we emit. Without this the ETag is decoration:
            // `must-revalidate` makes every client re-ask on every navigation, and
            // each of those would re-send the whole body unchanged.
            if if_none_match.is_some_and(|v| etag_matches(v, &etag)) {
                return Response::builder()
                    .status(StatusCode::NOT_MODIFIED)
                    .header("Cache-Control", "public, max-age=0, must-revalidate")
                    .header("ETag", etag)
                    .body(Full::new(Bytes::new()))
                    .unwrap();
            }

            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .header("Cache-Control", "public, max-age=0, must-revalidate")
                .header("ETag", etag)
                .body(Full::new(Bytes::from(json)))
                .unwrap()
        }
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
        assert_eq!(
            foreign_locations(&doc()),
            Vec::<String>::new(),
            "the discovery document advertises a location outside its own origin — that makes \
             it an open-redirect primitive"
        );
    }

    /// Walk a serialized document and return every location that escapes the
    /// origin. Shared by the guard test and its detector-control below, so the
    /// control exercises the SAME walker the guard relies on.
    fn foreign_locations(doc: &serde_json::Value) -> Vec<String> {
        fn walk(v: &serde_json::Value, path: &str, out: &mut Vec<String>) {
            match v {
                serde_json::Value::String(s) => {
                    let location_shaped = s.starts_with('/') || s.contains("://");
                    // `//host/x` names ANOTHER origin while passing a naive
                    // leading-slash check — the classic bypass.
                    if location_shaped && (!s.starts_with('/') || s.starts_with("//")) {
                        out.push(format!("{path} = {s}"));
                    }
                }
                serde_json::Value::Object(m) => {
                    for (k, sub) in m {
                        walk(sub, &format!("{path}.{k}"), out);
                    }
                }
                _ => {}
            }
        }
        let mut out = Vec::new();
        walk(doc, "$", &mut out);
        out
    }

    /// DETECTOR CONTROL for the guard above.
    ///
    /// The guard only means something if the walker would actually FAIL on a
    /// document that escapes the origin — a check that can never fire is the
    /// mirrored-test shape the seam census exists to catch. So this feeds
    /// hostile values through the REAL `AuthDiscovery` type and its real
    /// serializer, rather than asserting a hand-written predicate against a
    /// literal, and requires each one to be caught.
    #[test]
    fn the_relative_location_guard_catches_an_escaping_document() {
        for hostile in [
            "//evil.tld/login",
            "https://evil.tld/login",
            "http://evil.tld/login",
        ] {
            let escaping = AuthDiscovery {
                version: 1,
                doorway_id: Some("alpha-elohim-host".to_string()),
                portal: hostile,
                endpoints: AuthEndpoints::current(),
            };
            let doc = serde_json::to_value(&escaping).expect("serializes");
            let caught = foreign_locations(&doc);
            assert!(
                caught.iter().any(|c| c.contains(hostile)),
                "the guard failed to catch a document escaping its origin via {hostile:?} —                  it would pass a discovery document that can aim a Login button anywhere"
            );
        }
    }

    #[test]
    fn a_matching_validator_is_recognised_in_every_legal_header_shape() {
        let tag = "\"abc123\"";
        assert!(etag_matches(tag, tag), "the simple case must match");
        assert!(etag_matches("*", tag), "`*` matches any current representation");
        assert!(
            etag_matches("\"other\", \"abc123\"", tag),
            "If-None-Match is a LIST — a naive equality check answers 200 to a well-formed \
             conditional request and the validator never fires"
        );
        assert!(!etag_matches("\"stale\"", tag), "a stale validator must NOT match");
        assert!(!etag_matches("", tag), "an empty header must not match");
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
