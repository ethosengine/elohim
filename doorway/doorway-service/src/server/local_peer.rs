//! Opt-in local developer relay for private declared views (`/api/local-views`).
//!
//! Projection semantics stay in the peer's hub service (`elohim-hub::local_views`); this
//! module only forwards a bounded request shape to a reader on the same machine. It is
//! dormant unless `LOCAL_VIEW_READER_URL` names a loopback `http` reader: every
//! `/api/local-views` request then answers 404 with `Cache-Control: private, no-store`,
//! and no other path is touched. `--local-view-relay <loopback addr>` runs the relay alone,
//! without Mongo, conductors or hosted-agent services (see `main.rs`).
use bytes::Bytes;
use http_body_util::Full;
use hyper::{Method, Request, Response, StatusCode};
use std::sync::OnceLock;

const PREFIX: &str = "/api/local-views";

/// Whether `path` belongs to the relay. Everything else stays with the ordinary router.
pub fn is_local_view_path(path: &str) -> bool {
    path == PREFIX
        || path
            .strip_prefix(PREFIX)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// The reader URL the relay may forward to, or `None` when it must stay dormant: only a
/// bare `http://127.0.0.1[:port]/` or `http://[::1][:port]/` with no credentials, query
/// or fragment qualifies.
pub fn reader_target(raw: Option<&str>) -> Option<reqwest::Url> {
    let url = reqwest::Url::parse(raw?.trim()).ok()?;
    let loopback = matches!(url.host_str(), Some("127.0.0.1" | "[::1]"));
    (url.scheme() == "http"
        && loopback
        && url.username().is_empty()
        && url.password().is_none()
        && url.path() == "/"
        && url.query().is_none()
        && url.fragment().is_none())
    .then_some(url)
}

/// `LOCAL_VIEW_READER_URL`, read once per process so the request path never reads the
/// environment.
fn configured_target() -> Option<&'static reqwest::Url> {
    static TARGET: OnceLock<Option<reqwest::Url>> = OnceLock::new();
    TARGET
        .get_or_init(|| reader_target(std::env::var("LOCAL_VIEW_READER_URL").ok().as_deref()))
        .as_ref()
}

/// View ids are `[A-Za-z0-9_-]` and snapshot CIDs are base32; nothing else (no `.`, `%`
/// or empty segment) may reach the reader, so the forwarded path cannot be re-normalised
/// into a different route.
fn forwardable_path(path: &str) -> bool {
    is_local_view_path(path)
        && !path.contains("//")
        && !path.ends_with('/')
        && path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_'))
}

/// The client that carries the reader capability. The reader is on this machine, so the
/// request must never leave it: `no_proxy` drops every proxy, including the system proxy
/// reqwest otherwise reads from `HTTP_PROXY`/`ALL_PROXY` (which would receive the
/// Authorization header whenever `NO_PROXY` does not name loopback), and redirects are
/// not followed.
fn reader_client(builder: reqwest::ClientBuilder) -> Option<reqwest::Client> {
    builder
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .ok()
}

fn refused() -> Response<Full<Bytes>> {
    let mut response = Response::new(Full::new(Bytes::new()));
    *response.status_mut() = StatusCode::NOT_FOUND;
    response.headers_mut().insert(
        hyper::header::CACHE_CONTROL,
        hyper::header::HeaderValue::from_static("private, no-store"),
    );
    response
}

/// Relay one `/api/local-views` request using the process-wide configuration.
pub async fn relay<B>(req: Request<B>) -> Response<Full<Bytes>> {
    relay_to(req, configured_target()).await
}

/// Relay one request to `target`. Answers 404 for anything outside the bounded shape:
/// no target, another path, a query, a method other than GET/POST, or no Authorization.
pub async fn relay_to<B>(req: Request<B>, target: Option<&reqwest::Url>) -> Response<Full<Bytes>> {
    let Some(target) = target else {
        return refused();
    };
    let path = req.uri().path();
    // No cookies, credentials or origins are manufactured. The access adapter supplies
    // the reader capability; the reader checks it even when the relay is reached directly.
    let Some(authorization) = req.headers().get(hyper::header::AUTHORIZATION).cloned() else {
        return refused();
    };
    if !forwardable_path(path)
        || req.uri().query().is_some()
        || !matches!(*req.method(), Method::GET | Method::POST)
    {
        return refused();
    }
    static CLIENT: OnceLock<Option<reqwest::Client>> = OnceLock::new();
    let Some(client) = CLIENT
        .get_or_init(|| reader_client(reqwest::Client::builder()))
        .as_ref()
    else {
        return refused();
    };
    let url = format!("{}{}", target.as_str().trim_end_matches('/'), path);
    let upstream = match client
        .request(req.method().clone(), url)
        .header(reqwest::header::AUTHORIZATION, authorization)
        .send()
        .await
    {
        Ok(upstream) => upstream,
        Err(_) => return refused(),
    };
    let status = upstream.status();
    let Ok(bytes) = upstream.bytes().await else {
        return refused();
    };
    let mut response = Response::new(Full::new(bytes));
    *response.status_mut() = status;
    let headers = response.headers_mut();
    headers.insert(
        hyper::header::CONTENT_TYPE,
        hyper::header::HeaderValue::from_static("application/json"),
    );
    headers.insert(
        hyper::header::CACHE_CONTROL,
        hyper::header::HeaderValue::from_static("private, no-store"),
    );
    response
}

/// Local delivery adapter mode: no Mongo, conductors, swarms or hosted agent services.
pub async fn serve(listen: std::net::SocketAddr) -> anyhow::Result<()> {
    anyhow::ensure!(listen.ip().is_loopback(), "local relay requires loopback");
    let listener = tokio::net::TcpListener::bind(listen).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            let service = hyper::service::service_fn(|request| async move {
                Ok::<_, std::convert::Infallible>(relay(request).await)
            });
            // A dropped connection ends only its own task; the listener keeps serving.
            let _ = hyper::server::conn::http1::Builder::new()
                .serve_connection(hyper_util::rt::TokioIo::new(stream), service)
                .await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    fn request(method: Method, uri: &str, authorization: Option<&str>) -> Request<()> {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(value) = authorization {
            builder = builder.header("authorization", value);
        }
        builder.body(()).expect("test request")
    }

    #[test]
    fn only_the_local_view_prefix_belongs_to_the_relay() {
        assert!(is_local_view_path("/api/local-views"));
        assert!(is_local_view_path("/api/local-views/orchard/history"));
        assert!(!is_local_view_path("/api/local-viewsx"));
        assert!(!is_local_view_path("/api/v1/cache/content"));
        assert!(!is_local_view_path("/"));
    }

    #[test]
    fn only_a_bare_loopback_http_reader_qualifies() {
        assert!(reader_target(Some("http://127.0.0.1:8097")).is_some());
        assert!(reader_target(Some("http://127.0.0.1:8097/")).is_some());
        assert!(reader_target(Some("http://[::1]:8097")).is_some());
        for refused in [
            None,
            Some(""),
            Some("https://127.0.0.1:8097"),
            Some("http://localhost:8097"),
            Some("http://10.0.0.5:8097"),
            Some("http://127.0.0.1:8097/prefix"),
            Some("http://user:pw@127.0.0.1:8097"),
            Some("http://127.0.0.1:8097/?x=1"),
        ] {
            assert!(reader_target(refused).is_none(), "{refused:?}");
        }
    }

    #[tokio::test]
    async fn without_a_reader_every_local_view_request_is_404() {
        let response = relay_to(
            request(Method::GET, "/api/local-views", Some("Bearer t")),
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers()[hyper::header::CACHE_CONTROL],
            "private, no-store"
        );
    }

    #[tokio::test]
    async fn malformed_requests_are_refused_before_any_forward() {
        // Port 1 is never a reader: a request that got past the checks would fail to
        // connect, so this test also proves nothing is attempted for refused shapes.
        let target = reader_target(Some("http://127.0.0.1:1")).unwrap();
        for req in [
            request(Method::GET, "/api/local-views/orchard/history", None),
            request(
                Method::GET,
                "/api/local-views/orchard/history?root=/etc",
                Some("Bearer t"),
            ),
            request(
                Method::DELETE,
                "/api/local-views/orchard/history",
                Some("Bearer t"),
            ),
            request(Method::GET, "/api/local-views/../admin", Some("Bearer t")),
            request(Method::GET, "/api/local-views/%2e%2e/x", Some("Bearer t")),
            request(Method::GET, "/api/other", Some("Bearer t")),
        ] {
            assert_eq!(
                relay_to(req, Some(&target)).await.status(),
                StatusCode::NOT_FOUND
            );
        }
    }

    #[tokio::test]
    async fn the_reader_client_never_routes_through_a_proxy() {
        // A stand-in proxy that would receive the capability if the client used it.
        let proxy = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_url = format!("http://{}", proxy.local_addr().unwrap());
        let reader = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let reader_address = reader.local_addr().unwrap();
        tokio::spawn(async move {
            let (stream, _) = reader.accept().await.unwrap();
            let service =
                hyper::service::service_fn(|_req: Request<hyper::body::Incoming>| async {
                    Ok::<_, std::convert::Infallible>(Response::new(Full::new(Bytes::from(
                        "reader",
                    ))))
                });
            let _ = hyper::server::conn::http1::Builder::new()
                .serve_connection(hyper_util::rt::TokioIo::new(stream), service)
                .await;
        });
        // Even a builder that already names a proxy loses it: `no_proxy` clears explicit
        // proxies as well as the system (environment) proxy.
        let client = reader_client(
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .proxy(reqwest::Proxy::all(&proxy_url).unwrap()),
        )
        .unwrap();
        let body = client
            .get(format!("http://{reader_address}/api/local-views"))
            .header(reqwest::header::AUTHORIZATION, "Bearer reader-token")
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert_eq!(body, "reader");
        let reached_proxy =
            tokio::time::timeout(std::time::Duration::from_millis(200), proxy.accept()).await;
        assert!(reached_proxy.is_err(), "the proxy received a connection");
    }

    #[tokio::test]
    async fn a_configured_relay_forwards_method_path_and_authorization_only() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let service =
                hyper::service::service_fn(|req: Request<hyper::body::Incoming>| async move {
                    let seen = format!(
                        "{} {} {}",
                        req.method(),
                        req.uri(),
                        req.headers()
                            .get("authorization")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("-")
                    );
                    let mut response = Response::new(Full::new(Bytes::from(seen)));
                    *response.status_mut() = StatusCode::CREATED;
                    Ok::<_, std::convert::Infallible>(response)
                });
            let _ = hyper::server::conn::http1::Builder::new()
                .serve_connection(hyper_util::rt::TokioIo::new(stream), service)
                .await;
        });
        let target = reader_target(Some(&format!("http://{address}"))).unwrap();
        let response = relay_to(
            request(
                Method::POST,
                "/api/local-views/orchard/capture",
                Some("Bearer reader-token"),
            ),
            Some(&target),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(
            response.headers()[hyper::header::CACHE_CONTROL],
            "private, no-store"
        );
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            body,
            Bytes::from("POST /api/local-views/orchard/capture Bearer reader-token")
        );
    }
}
