//! EPR Head proxy routes
//!
//! Proxies EPR Head requests to elohim-storage with Accept header forwarding.
//!
//! - `GET /api/epr-head/{id}` → `GET {storage_url}/epr-head/{id}`
//! - `PUT /api/epr-head/{id}` → `PUT {storage_url}/epr-head/{id}`
//!
//! ## Validators, not caching (serving-edge story 6.1)
//!
//! An EPR-head declaration is mutable — a collective can re-declare it at any
//! time — so it must never be answered by a CDN as fresh-cacheable. It CAN be
//! made cheaply REVALIDATABLE: the `GET` arm mints a strong `ETag` over the
//! exact response body (`routes::freshness::mint_served_head`, the same
//! content-address convention the amber pantry uses) and honours
//! `If-None-Match` with a `304` when it matches, so a repeat ask that changed
//! nothing costs an empty body instead of the full envelope. `Cache-Control`
//! is always a `no-cache` variant (revalidate every time, never serve stale)
//! — `private, no-cache` when the caller is authenticated, since reach can
//! make the body differ per bearer. Only a `GET` with a 2xx upstream answer
//! gets a validator: a `PUT` is a write, and an error body is not a
//! representation to revalidate.

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{header, Method, Request, Response, StatusCode};
use tracing::debug;

use crate::routes::freshness::mint_served_head;
use crate::routes::validators::etag_matches;
use crate::server::http::{determine_auth_posture, AuthPosture};

/// Handle EPR Head proxy requests.
///
/// Forwards to elohim-storage's `/epr-head/{id}` endpoint, preserving
/// the Accept header for content negotiation (JSON vs DAG-CBOR).
pub async fn handle_epr_head_request(
    req: Request<Incoming>,
    storage_url: &str,
    id: &str,
) -> Result<Response<Full<Bytes>>, String> {
    if id.is_empty() {
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(r#"{"error":"Missing EPR Head ID"}"#)))
            .unwrap());
    }

    let method = req.method().clone();
    let upstream_url = format!(
        "{}/epr-head/{}",
        storage_url.trim_end_matches('/'),
        urlencoding::encode(id)
    );

    debug!(method = %method, id = %id, upstream = %upstream_url, "Proxying EPR Head request");

    let client = reqwest::Client::new();

    match method {
        Method::GET => {
            // Forward Accept header for content negotiation
            let accept = req
                .headers()
                .get(header::ACCEPT)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/json")
                .to_string();
            // Captured BEFORE the request; `determine_auth_posture` reuses the
            // one predicate the crate already recognises an authenticated
            // caller by (Authorization bearer, or a `doorway_session=` /
            // `steward_attestation=` cookie) — never a second definition.
            let if_none_match = req
                .headers()
                .get(header::IF_NONE_MATCH)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string);
            let authenticated = !matches!(determine_auth_posture(&req), AuthPosture::Anonymous);

            let response = client
                .get(&upstream_url)
                .header("Accept", &accept)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
                .map_err(|e| format!("Upstream request failed: {e}"))?;

            proxy_response_validated(response, if_none_match.as_deref(), authenticated).await
        }
        Method::PUT => {
            // Extract Content-Type before consuming the body
            let content_type = req
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/json")
                .to_string();

            let body = req
                .collect()
                .await
                .map_err(|e| format!("Failed to read body: {e}"))?;
            let data = body.to_bytes();

            let response = client
                .put(&upstream_url)
                .header("Content-Type", &content_type)
                .body(data.to_vec())
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
                .map_err(|e| format!("Upstream request failed: {e}"))?;

            // A write gets no validator — the answer to a PUT is not a cached
            // representation and must never be revalidated in its place.
            proxy_response(response).await
        }
        _ => Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Full::new(Bytes::from("Method not allowed")))
            .unwrap()),
    }
}

/// Convert a reqwest response to a hyper response, unchanged. Used for the
/// `PUT` arm (and by [`proxy_response_validated`] for a non-2xx `GET`
/// answer): no `ETag`, no `Cache-Control` — nothing here is a representation
/// a client should ever revalidate.
async fn proxy_response(response: reqwest::Response) -> Result<Response<Full<Bytes>>, String> {
    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();

    let data = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read upstream body: {e}"))?;

    Ok(Response::builder()
        .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
        .header(header::CONTENT_TYPE, &content_type)
        .header("Cross-Origin-Resource-Policy", "cross-origin")
        .body(Full::new(Bytes::from(data.to_vec())))
        .unwrap())
}

/// Convert a reqwest response to a hyper response for the `GET` arm, adding a
/// strong validator over the exact body bytes on a 2xx answer.
///
/// A non-2xx upstream answer falls through to the unchanged [`proxy_response`]
/// shape — an error body is not a representation to revalidate. On a 2xx
/// answer: mint the `ETag`, choose `Cache-Control` from `authenticated`
/// (`private, no-cache` vs `no-cache` — always revalidate, never served fresh
/// by a shared cache), and answer `304` with an empty body when `if_none_match`
/// already names the current `ETag`. `Vary: Authorization, Cookie` names BOTH
/// headers `determine_auth_posture` reads, since either can change the body a
/// shared cache would otherwise conflate across callers.
async fn proxy_response_validated(
    response: reqwest::Response,
    if_none_match: Option<&str>,
    authenticated: bool,
) -> Result<Response<Full<Bytes>>, String> {
    let status = response.status();
    if !status.is_success() {
        return proxy_response(response).await;
    }

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();

    let data = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read upstream body: {e}"))?;

    let etag = format!("\"{}\"", mint_served_head(&data));
    let cache_control = if authenticated {
        "private, no-cache"
    } else {
        "no-cache"
    };

    if if_none_match.is_some_and(|v| etag_matches(v, &etag)) {
        return Ok(Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .header("ETag", &etag)
            .header("Cache-Control", cache_control)
            .header("Vary", "Authorization, Cookie")
            .header("Cross-Origin-Resource-Policy", "cross-origin")
            .body(Full::new(Bytes::new()))
            .unwrap());
    }

    Ok(Response::builder()
        .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
        .header(header::CONTENT_TYPE, &content_type)
        .header("Cross-Origin-Resource-Policy", "cross-origin")
        .header("ETag", &etag)
        .header("Cache-Control", cache_control)
        .header("Vary", "Authorization, Cookie")
        .body(Full::new(Bytes::from(data.to_vec())))
        .unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::method as wm_method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn get_validated(
        server: &MockServer,
        if_none_match: Option<&str>,
        authenticated: bool,
    ) -> Response<Full<Bytes>> {
        let response = reqwest::Client::new()
            .get(server.uri())
            .send()
            .await
            .expect("mock upstream reachable");
        proxy_response_validated(response, if_none_match, authenticated)
            .await
            .expect("validated proxy response")
    }

    fn header(resp: &Response<Full<Bytes>>, name: &str) -> Option<String> {
        resp.headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
    }

    async fn body_bytes(resp: Response<Full<Bytes>>) -> Bytes {
        resp.into_body().collect().await.unwrap().to_bytes()
    }

    #[tokio::test]
    async fn etag_is_stable_for_identical_bodies_and_differs_for_different_bodies() {
        let server_a = MockServer::start().await;
        Mock::given(wm_method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("same-body"))
            .mount(&server_a)
            .await;
        let server_a2 = MockServer::start().await;
        Mock::given(wm_method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("same-body"))
            .mount(&server_a2)
            .await;
        let server_b = MockServer::start().await;
        Mock::given(wm_method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("different-body"))
            .mount(&server_b)
            .await;

        let a1 = get_validated(&server_a, None, false).await;
        let a2 = get_validated(&server_a2, None, false).await;
        let b = get_validated(&server_b, None, false).await;

        let etag_a1 = header(&a1, "etag").expect("etag present");
        let etag_a2 = header(&a2, "etag").expect("etag present");
        let etag_b = header(&b, "etag").expect("etag present");

        assert_eq!(
            etag_a1, etag_a2,
            "identical bytes must mint the identical ETag on every doorway"
        );
        assert_ne!(
            etag_a1, etag_b,
            "different bytes must mint a different ETag"
        );
        assert!(
            etag_a1.starts_with('"') && etag_a1.ends_with('"'),
            "quoted strong validator"
        );
    }

    #[tokio::test]
    async fn matching_if_none_match_returns_304_with_empty_body_and_same_etag() {
        let server = MockServer::start().await;
        Mock::given(wm_method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("epr-head-json"))
            .mount(&server)
            .await;

        let first = get_validated(&server, None, false).await;
        let etag = header(&first, "etag").expect("etag present");

        let second = get_validated(&server, Some(&etag), false).await;
        assert_eq!(second.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(header(&second, "etag"), Some(etag));
        assert_eq!(
            body_bytes(second).await,
            Bytes::new(),
            "304 carries no body"
        );
    }

    #[tokio::test]
    async fn non_matching_if_none_match_returns_200_with_etag() {
        let server = MockServer::start().await;
        Mock::given(wm_method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("epr-head-json"))
            .mount(&server)
            .await;

        let resp = get_validated(&server, Some("\"stale-tag\""), false).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(header(&resp, "etag").is_some());
        assert_eq!(body_bytes(resp).await, Bytes::from_static(b"epr-head-json"));
    }

    #[tokio::test]
    async fn star_if_none_match_matches_any_current_representation() {
        let server = MockServer::start().await;
        Mock::given(wm_method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("epr-head-json"))
            .mount(&server)
            .await;

        let resp = get_validated(&server, Some("*"), false).await;
        assert_eq!(resp.status(), StatusCode::NOT_MODIFIED);
    }

    #[tokio::test]
    async fn non_2xx_upstream_gets_no_etag() {
        let server = MockServer::start().await;
        Mock::given(wm_method("GET"))
            .respond_with(ResponseTemplate::new(404).set_body_string(r#"{"error":"not-found"}"#))
            .mount(&server)
            .await;

        let resp = get_validated(&server, None, false).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        assert!(header(&resp, "etag").is_none());
        assert!(header(&resp, "cache-control").is_none());
    }

    #[tokio::test]
    async fn put_arm_gets_no_etag() {
        // PUT never reaches `proxy_response_validated` at all — it always uses
        // the unchanged `proxy_response`, which this pins directly: a mocked
        // 200 upstream answer must carry no ETag / Cache-Control.
        let server = MockServer::start().await;
        Mock::given(wm_method("PUT"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"ok":true}"#))
            .mount(&server)
            .await;

        let response = reqwest::Client::new()
            .put(server.uri())
            .send()
            .await
            .expect("mock upstream reachable");
        let resp = proxy_response(response).await.expect("proxy response");
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(header(&resp, "etag").is_none());
        assert!(header(&resp, "cache-control").is_none());
    }

    #[tokio::test]
    async fn authenticated_request_gets_private_no_cache() {
        let server = MockServer::start().await;
        Mock::given(wm_method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("epr-head-json"))
            .mount(&server)
            .await;

        let anon = get_validated(&server, None, false).await;
        assert_eq!(header(&anon, "cache-control"), Some("no-cache".to_string()));

        let authed = get_validated(&server, None, true).await;
        assert_eq!(
            header(&authed, "cache-control"),
            Some("private, no-cache".to_string())
        );
    }

    #[tokio::test]
    async fn validated_response_carries_a_vary_header_naming_both_auth_seams() {
        let server = MockServer::start().await;
        Mock::given(wm_method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("epr-head-json"))
            .mount(&server)
            .await;

        let resp = get_validated(&server, None, false).await;
        let vary = header(&resp, "vary").expect("vary present");
        assert!(vary.contains("Authorization"));
        assert!(vary.contains("Cookie"));
    }
}
