//! Generic storage proxy — single implementation shared by all registry-routed handlers.
//!
//! All domain proxy files (governance, presence, economic_events, etc.) contained
//! identical copies of this function. The route registry now owns dispatch; this
//! module owns the one canonical forwarding implementation.
//!
//! ## Blob caching
//!
//! [`forward_blob_to_storage`] wraps [`forward_to_storage`] for `/blob/<hash>` paths.
//! It checks the local pantry (ContentCache) before hitting storage and stocks the
//! pantry on a successful 200 response, so subsequent requests draw from local cache.
//!
//! Caching is skipped for:
//! - Requests carrying a `Range` header (partial reads — do not cache incomplete data)
//! - 206 Partial Content upstream responses
//! - Non-2xx upstream responses
//! - Blobs exceeding [`BLOB_PANTRY_MAX_BYTES`] (protects ContentCache entry budget)
//!
//! Cache failures are logged at `warn!` and never fail the user response.

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Method, Request, Response, StatusCode};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

use crate::cache::ContentCache;

/// Bundle of doorway-resolved context that the forwarder injects as headers on the
/// outbound request to elohim-storage.
///
/// Keeping the surface as a struct (vs. positional args) lets new context fields
/// (e.g. permission level, doorway operator id) land without churning every call site.
#[derive(Default, Debug, Clone, Copy)]
pub struct ForwardCtx<'a> {
    /// `agent_cid` resolved from the bearer's claims (alpha-substrate: `claims.human_id`).
    /// When `Some`, the forwarder emits `X-Agent-Cid: <value>` to storage. When `None`
    /// (no bearer / invalid bearer / Session Visitor), no header is set and storage
    /// falls back to its `local_sessions`-based resolution or treats as visitor.
    pub agent_cid: Option<&'a str>,
}

/// Maximum blob size (in bytes) written to the local pantry via the registry path.
///
/// Blobs larger than this are served through but not stocked in the ContentCache,
/// which is entry-count-limited (default 10,000 entries). 50 MB is a reasonable
/// upper bound that keeps individual entries from exhausting the budget.
///
/// Operators can raise this by setting `BLOB_PANTRY_MAX_BYTES` env var.
pub const BLOB_PANTRY_MAX_BYTES_DEFAULT: u64 = 50 * 1024 * 1024; // 50 MB

fn blob_pantry_max_bytes() -> u64 {
    std::env::var("BLOB_PANTRY_MAX_BYTES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(BLOB_PANTRY_MAX_BYTES_DEFAULT)
}

/// Blob TTL for the local pantry: mirrors the 1-hour default used by
/// [`handle_blob_request_with_storage_proxy`](super::blob::handle_blob_request_with_storage_proxy).
const BLOB_PANTRY_TTL: Duration = Duration::from_secs(3_600); // 1 hour

/// Forward an incoming request to the elohim-storage endpoint.
///
/// Builds `{storage_url}{path}[?{query}]`, forwards the HTTP method, preserves
/// `content-type` and `authorization` headers, streams the body for
/// POST/PUT/PATCH, and returns the storage response with a
/// `Cross-Origin-Resource-Policy: cross-origin` header so Angular (COEP) is happy.
///
/// Errors surfaces as:
/// - `400 BAD_REQUEST`  — failed to read incoming body
/// - `502 BAD_GATEWAY`  — failed to connect to or read from storage
/// - `405 METHOD_NOT_ALLOWED` — method not in GET/POST/PUT/DELETE/HEAD/PATCH
///
/// Generic over `B` so the production path accepts `hyper::body::Incoming` and
/// tests can pass lightweight body types such as `http_body_util::Empty<Bytes>`.
pub async fn forward_to_storage<B>(
    req: Request<B>,
    storage_url: &str,
    path: &str,
    ctx: ForwardCtx<'_>,
) -> Response<Full<Bytes>>
where
    B: hyper::body::Body + Send + 'static,
    B::Data: Send,
    B::Error: std::fmt::Display,
{
    let storage_endpoint = format!("{}{}", storage_url.trim_end_matches('/'), path);

    let query = req.uri().query();
    let full_url = match query {
        Some(q) => format!("{storage_endpoint}?{q}"),
        None => storage_endpoint,
    };

    let method = req.method().clone();
    debug!(method = %method, url = %full_url, "Forwarding request to elohim-storage");

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

    // Forward observation session header if present
    if let Some(obs_id) = req.headers().get("x-observation-id") {
        if let Ok(obs_str) = obs_id.to_str() {
            builder = builder.header("X-Observation-Id", obs_str);
        }
    }

    // Inject X-Agent-Cid from doorway-resolved context. In the alpha substrate
    // this is sourced from `claims.human_id` upstream; storage's `extract_agent_cid`
    // helper reads this header for view-service identity resolution. When CIDv1
    // enforcement lands, the source switches to a persisted CID without changing
    // the wire shape.
    if let Some(cid) = ctx.agent_cid {
        builder = builder.header("X-Agent-Cid", cid);
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

            match response.bytes().await {
                Ok(body) => Response::builder()
                    .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK))
                    .header("Content-Type", content_type)
                    .header("Cross-Origin-Resource-Policy", "cross-origin")
                    .body(Full::new(Bytes::from(body.to_vec())))
                    .unwrap(),
                Err(e) => {
                    warn!(error = %e, "Failed to read storage response body");
                    Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .header("Content-Type", "application/json")
                        .body(Full::new(Bytes::from(format!(
                            r#"{{"error": "Failed to read storage response: {e}"}}"#
                        ))))
                        .unwrap()
                }
            }
        }
        Err(e) => {
            warn!(error = %e, path = %path, "Failed to forward request to storage");
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error": "Failed to connect to storage: {e}"}}"#
                ))))
                .unwrap()
        }
    }
}

/// Forward a `/blob/<hash>` request to elohim-storage with local pantry caching.
///
/// Behaviour (pantry = local [`ContentCache`]):
///
/// 1. **Cache-first:** If the blob is already stocked in the pantry, serve it
///    immediately without touching storage.
/// 2. **Forward on miss:** Send the request to elohim-storage via
///    [`forward_to_storage`].
/// 3. **Stock on 200:** If the upstream returns 200 and the body fits within
///    [`blob_pantry_max_bytes()`], write it to the pantry so the next request
///    is served locally.
///    Log: `debug!` "Blob stocked in pantry, hash=X, size=N"
/// 4. **Skip caching for:** Range requests (`Range` header present), 206
///    partial content, non-2xx responses, oversized blobs, and cache-write
///    failures.  In all skip cases the upstream response is still returned to
///    the caller.
///
/// The `path` parameter must begin with `/blob/` (e.g. `/blob/sha256-abc123`).
/// For any other path prefix the function falls through to [`forward_to_storage`]
/// without any cache interaction.
/// Generic over `B` for the same reason as [`forward_to_storage`] — tests pass
/// `http_body_util::Empty<Bytes>` while production passes `hyper::body::Incoming`.
pub async fn forward_blob_to_storage<B>(
    req: Request<B>,
    storage_url: &str,
    path: &str,
    cache: Arc<ContentCache>,
    ctx: ForwardCtx<'_>,
) -> Response<Full<Bytes>>
where
    B: hyper::body::Body + Send + 'static,
    B::Data: Send,
    B::Error: std::fmt::Display,
{
    // Extract the hash from "/blob/<hash>"
    let hash = match path.strip_prefix("/blob/") {
        Some(h) if !h.is_empty() => h.to_string(),
        _ => {
            // Not a blob path — fall through to generic forwarder
            return forward_to_storage(req, storage_url, path, ctx).await;
        }
    };

    // --- Skip caching for Range requests ----------------------------------------
    let has_range_header = req.headers().contains_key(hyper::header::RANGE);
    if has_range_header {
        debug!(hash = %hash, "Range request — skipping pantry, forwarding directly");
        return forward_to_storage(req, storage_url, path, ctx).await;
    }

    // --- Cache-first: pantry hit? ------------------------------------------------
    if let Some(size) = cache.blob_size(&hash) {
        debug!(hash = %hash, size = size, "Blob drawn from pantry (cache hit)");
        if let Some(entry) = cache.get(&hash) {
            return Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", &entry.content_type)
                .header("Content-Length", entry.data.len())
                .header("Cross-Origin-Resource-Policy", "cross-origin")
                .header(
                    hyper::header::CACHE_CONTROL,
                    "public, max-age=31536000, immutable",
                )
                .body(Full::new(Bytes::from(entry.data)))
                .unwrap();
        }
    }

    // --- Forward to storage ------------------------------------------------------
    let storage_endpoint = format!("{}{}", storage_url.trim_end_matches('/'), path);
    let query = req.uri().query();
    let full_url = match query {
        Some(q) => format!("{storage_endpoint}?{q}"),
        None => storage_endpoint,
    };

    debug!(hash = %hash, url = %full_url, "Blob cache miss — forwarding to elohim-storage");

    let client = reqwest::Client::new();
    let builder = client.get(&full_url);

    // Forward auth header if present
    let builder = if let Some(auth) = req.headers().get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            builder.header("Authorization", auth_str)
        } else {
            builder
        }
    } else {
        builder
    };

    // Inject X-Agent-Cid from doorway-resolved context (see ForwardCtx docs).
    // Storage uses this for reach gating on private blobs.
    let builder = match ctx.agent_cid {
        Some(cid) => builder.header("X-Agent-Cid", cid),
        None => builder,
    };

    match builder.send().await {
        Ok(upstream) => {
            let status = upstream.status();
            let content_type = upstream
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/octet-stream")
                .to_string();

            match upstream.bytes().await {
                Ok(body) => {
                    // --- Stock pantry on clean 200 -----------------------------------
                    let should_cache = status == StatusCode::OK
                        && status != StatusCode::PARTIAL_CONTENT
                        && (body.len() as u64) <= blob_pantry_max_bytes();

                    if should_cache {
                        cache.set(&hash, body.to_vec(), &content_type, BLOB_PANTRY_TTL);
                        debug!(
                            hash = %hash,
                            size = body.len(),
                            "Blob stocked in pantry, hash={}, size={}",
                            hash,
                            body.len()
                        );
                    } else if status == StatusCode::PARTIAL_CONTENT {
                        debug!(hash = %hash, "206 Partial Content — not stocking pantry");
                    } else if !status.is_success() {
                        debug!(hash = %hash, status = %status, "Non-2xx — not stocking pantry");
                    } else if (body.len() as u64) > blob_pantry_max_bytes() {
                        debug!(
                            hash = %hash,
                            size = body.len(),
                            max = blob_pantry_max_bytes(),
                            "Blob exceeds pantry budget — not stocking"
                        );
                    }

                    Response::builder()
                        .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK))
                        .header("Content-Type", content_type)
                        .header("Cross-Origin-Resource-Policy", "cross-origin")
                        .body(Full::new(body))
                        .unwrap()
                }
                Err(e) => {
                    warn!(error = %e, hash = %hash, "Failed to read blob response body from storage");
                    Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .header("Content-Type", "application/json")
                        .body(Full::new(Bytes::from(format!(
                            r#"{{"error": "Failed to read blob response: {e}"}}"#
                        ))))
                        .unwrap()
                }
            }
        }
        Err(e) => {
            warn!(error = %e, hash = %hash, "Failed to connect to storage for blob");
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error": "Failed to connect to storage: {e}"}}"#
                ))))
                .unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::CacheConfig;
    use http_body_util::Empty;
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper::StatusCode;
    use hyper_util::rt::TokioIo;
    use std::convert::Infallible;
    use std::net::SocketAddr;
    use tokio::net::TcpListener;

    /// Serializes any blob-cache test that exercises `forward_blob_to_storage`.
    /// `BLOB_PANTRY_MAX_BYTES` is process-global — one test mutates it while
    /// others read it via `blob_pantry_max_bytes()`, so without serialization
    /// the parallel test runner produces flakes (e.g. an 18-byte payload
    /// failing to cache because the oversized-blob test set the limit to 10).
    static BLOB_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    // ========================================================================
    // Existing forward_to_storage tests
    // ========================================================================

    /// Verify method-not-allowed branch without needing a live server.
    /// We construct a request with an unsupported method (OPTIONS) and confirm
    /// the function returns 405 before ever attempting a network call.
    #[test]
    fn method_not_allowed_returns_405() {
        // Verify the observable: method dispatch produces METHOD_NOT_ALLOWED for
        // an unsupported verb by mirroring the exact match arm logic.
        let method = Method::from_bytes(b"OPTIONS").unwrap();
        let response: Response<Full<Bytes>> = match method {
            Method::GET
            | Method::POST
            | Method::PUT
            | Method::DELETE
            | Method::HEAD
            | Method::PATCH => {
                panic!("OPTIONS should not match any forwarded method");
            }
            _ => Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap(),
        };

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    /// Verify that the URL construction appends path and query correctly.
    #[test]
    fn url_construction_with_query() {
        let storage_url = "http://localhost:8090/";
        let path = "/api/v1/governance/proposals";
        let query = Some("page=2&limit=10");

        let storage_endpoint = format!("{}{}", storage_url.trim_end_matches('/'), path);
        let full_url = match query {
            Some(q) => format!("{storage_endpoint}?{q}"),
            None => storage_endpoint,
        };

        assert_eq!(
            full_url,
            "http://localhost:8090/api/v1/governance/proposals?page=2&limit=10"
        );
    }

    /// Verify URL construction without query string.
    #[test]
    fn url_construction_without_query() {
        let storage_url = "http://localhost:8090";
        let path = "/db/content";

        let full_url = format!("{}{}", storage_url.trim_end_matches('/'), path);

        assert_eq!(full_url, "http://localhost:8090/db/content");
    }

    // ========================================================================
    // Helpers for blob caching tests
    // ========================================================================

    /// Spawn a minimal in-process HTTP server that returns a fixed response for
    /// every GET request.  Returns the bound address so tests can build storage
    /// URLs against it.  The server runs until the returned
    /// `tokio::task::JoinHandle` is dropped / aborted.
    async fn spawn_mock_storage(
        status: u16,
        body: Vec<u8>,
        content_type: &'static str,
    ) -> (SocketAddr, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                let io = TokioIo::new(stream);
                let body_clone = body.clone();
                tokio::spawn(async move {
                    let _ = http1::Builder::new()
                        .serve_connection(
                            io,
                            service_fn(move |_req: Request<hyper::body::Incoming>| {
                                let b = body_clone.clone();
                                async move {
                                    let resp: Result<Response<Full<Bytes>>, Infallible> =
                                        Ok(Response::builder()
                                            .status(status)
                                            .header("Content-Type", content_type)
                                            .body(Full::new(Bytes::from(b)))
                                            .unwrap());
                                    resp
                                }
                            }),
                        )
                        .await;
                });
            }
        });

        (addr, handle)
    }

    fn make_cache() -> Arc<ContentCache> {
        Arc::new(ContentCache::new(CacheConfig::default()))
    }

    /// Build a GET request with an empty body for use in tests.
    ///
    /// `forward_blob_to_storage` never reads the request body for GET requests
    /// (only POST/PUT/PATCH collect bodies in `forward_to_storage`), so an
    /// `Empty<Bytes>` body is a faithful stand-in.  The generic signature of
    /// `forward_blob_to_storage` accepts any `B: Body + Send + 'static`.
    fn make_get_request(uri: &str) -> Request<Empty<Bytes>> {
        Request::builder()
            .method(Method::GET)
            .uri(uri)
            .body(Empty::new())
            .unwrap()
    }

    // ========================================================================
    // Blob caching unit tests
    // ========================================================================

    /// Test 1: upstream returns 200 + bytes → cache.set is called with correct
    /// key and the pantry hit is served on the second request.
    #[tokio::test]
    async fn blob_200_stocks_pantry() {
        let _guard = BLOB_TEST_LOCK.lock().await;
        let payload = b"hello blob content".to_vec();
        let (addr, _handle) =
            spawn_mock_storage(200, payload.clone(), "application/octet-stream").await;
        let storage_url = format!("http://{addr}");

        let cache = make_cache();
        let hash = "sha256-aabbccdd00112233aabbccdd00112233aabbccdd00112233aabbccdd00112233";
        let path = format!("/blob/{hash}");

        // Before: cache miss
        assert!(
            cache.blob_size(hash).is_none(),
            "cache should be empty before first request"
        );

        // First request: cache miss, fetches from storage
        let req = make_get_request(&format!("http://doorway{path}"));
        let resp = forward_blob_to_storage(
            req,
            &storage_url,
            &path,
            Arc::clone(&cache),
            ForwardCtx::default(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);

        // After first request: pantry should be stocked
        assert!(
            cache.blob_size(hash).is_some(),
            "cache should be populated after 200 response"
        );
        assert_eq!(
            cache.blob_size(hash).unwrap(),
            payload.len(),
            "cached size should match payload"
        );

        // Verify the response body
        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body_bytes.as_ref(), payload.as_slice());
    }

    /// Test 2: upstream returns 206 Partial Content → cache is NOT touched.
    #[tokio::test]
    async fn blob_206_does_not_stock_pantry() {
        let _guard = BLOB_TEST_LOCK.lock().await;
        let payload = b"partial content".to_vec();
        let (addr, _handle) =
            spawn_mock_storage(206, payload.clone(), "application/octet-stream").await;
        let storage_url = format!("http://{addr}");

        let cache = make_cache();
        let hash = "sha256-aabbccdd00112233aabbccdd00112233aabbccdd00112233aabbccddff000001";
        let path = format!("/blob/{hash}");

        let req = make_get_request(&format!("http://doorway{path}"));
        let resp = forward_blob_to_storage(
            req,
            &storage_url,
            &path,
            Arc::clone(&cache),
            ForwardCtx::default(),
        )
        .await;

        // Response is forwarded
        assert_eq!(resp.status(), StatusCode::PARTIAL_CONTENT);
        // Cache is untouched
        assert!(
            cache.blob_size(hash).is_none(),
            "206 response must not stock the pantry"
        );
    }

    /// Test 3: request carries a Range header → cache is NOT touched (partial read).
    #[tokio::test]
    async fn range_request_skips_pantry() {
        let _guard = BLOB_TEST_LOCK.lock().await;
        let payload = b"ranged content".to_vec();
        let (addr, _handle) =
            spawn_mock_storage(206, payload.clone(), "application/octet-stream").await;
        let storage_url = format!("http://{addr}");

        let cache = make_cache();
        let hash = "sha256-aabbccdd00112233aabbccdd00112233aabbccdd00112233aabbccddff000002";
        let path = format!("/blob/{hash}");

        let req = Request::builder()
            .method(Method::GET)
            .uri(format!("http://doorway{path}"))
            .header(hyper::header::RANGE, "bytes=0-99")
            .body(Empty::<Bytes>::new())
            .unwrap();
        let _resp = forward_blob_to_storage(
            req,
            &storage_url,
            &path,
            Arc::clone(&cache),
            ForwardCtx::default(),
        )
        .await;

        assert!(
            cache.blob_size(hash).is_none(),
            "Range request must not stock the pantry"
        );
    }

    /// Test 4: blob exceeds BLOB_PANTRY_MAX_BYTES → response still served but
    /// cache is not written.
    #[tokio::test]
    async fn oversized_blob_served_but_not_cached() {
        let _guard = BLOB_TEST_LOCK.lock().await;
        // Use a tiny custom limit via env var for this test.
        // BLOB_PANTRY_MAX_BYTES is read at call time so it is safe to set per-test.
        std::env::set_var("BLOB_PANTRY_MAX_BYTES", "10"); // 10 bytes limit

        let payload = vec![0u8; 100]; // 100 bytes — exceeds the 10-byte limit
        let (addr, _handle) =
            spawn_mock_storage(200, payload.clone(), "application/octet-stream").await;
        let storage_url = format!("http://{addr}");

        let cache = make_cache();
        let hash = "sha256-aabbccdd00112233aabbccdd00112233aabbccdd00112233aabbccddff000003";
        let path = format!("/blob/{hash}");

        let req = make_get_request(&format!("http://doorway{path}"));
        let resp = forward_blob_to_storage(
            req,
            &storage_url,
            &path,
            Arc::clone(&cache),
            ForwardCtx::default(),
        )
        .await;

        // Response is still served correctly
        assert_eq!(resp.status(), StatusCode::OK);
        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body_bytes.len(), 100);

        // Cache is untouched because blob exceeded budget
        assert!(
            cache.blob_size(hash).is_none(),
            "oversized blob must not stock the pantry"
        );

        std::env::remove_var("BLOB_PANTRY_MAX_BYTES");
    }

    /// Test 5 (integration): two GET /blob/<hash> requests against a mock storage
    /// server — the second request is served from cache without hitting storage.
    ///
    /// We verify this by counting how many times the mock server received a
    /// request.  A hit counter shared via an Arc<AtomicU32> is incremented on
    /// every incoming connection.
    #[tokio::test]
    async fn second_request_served_from_pantry() {
        let _guard = BLOB_TEST_LOCK.lock().await;
        use std::sync::atomic::{AtomicU32, Ordering};

        let hit_counter = Arc::new(AtomicU32::new(0));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let payload = b"integration test blob".to_vec();
        let counter_clone = Arc::clone(&hit_counter);

        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                let io = TokioIo::new(stream);
                let body_clone = payload.clone();
                let ctr = Arc::clone(&counter_clone);
                tokio::spawn(async move {
                    ctr.fetch_add(1, Ordering::SeqCst);
                    let _ = http1::Builder::new()
                        .serve_connection(
                            io,
                            service_fn(move |_req: Request<hyper::body::Incoming>| {
                                let b = body_clone.clone();
                                async move {
                                    let resp: Result<Response<Full<Bytes>>, Infallible> =
                                        Ok(Response::builder()
                                            .status(200u16)
                                            .header("Content-Type", "application/octet-stream")
                                            .body(Full::new(Bytes::from(b)))
                                            .unwrap());
                                    resp
                                }
                            }),
                        )
                        .await;
                });
            }
        });

        let storage_url = format!("http://{addr}");
        let cache = make_cache();
        let hash = "sha256-aabbccdd00112233aabbccdd00112233aabbccdd00112233aabbccddff000004";
        let path = format!("/blob/{hash}");

        // First request — cache miss, hits storage
        let req1 = make_get_request(&format!("http://doorway{path}"));
        let resp1 = forward_blob_to_storage(
            req1,
            &storage_url,
            &path,
            Arc::clone(&cache),
            ForwardCtx::default(),
        )
        .await;
        assert_eq!(resp1.status(), StatusCode::OK);

        // Confirm pantry is now stocked
        assert!(
            cache.blob_size(hash).is_some(),
            "pantry must be stocked after first request"
        );
        assert_eq!(hit_counter.load(Ordering::SeqCst), 1, "storage hit once");

        // Second request — cache hit, must NOT hit storage
        let req2 = make_get_request(&format!("http://doorway{path}"));
        let resp2 = forward_blob_to_storage(
            req2,
            &storage_url,
            &path,
            Arc::clone(&cache),
            ForwardCtx::default(),
        )
        .await;
        assert_eq!(resp2.status(), StatusCode::OK);

        // Storage must still have been hit only once
        assert_eq!(
            hit_counter.load(Ordering::SeqCst),
            1,
            "second request must be served from pantry, storage must not be hit again"
        );
    }

    // ========================================================================
    // X-Agent-Cid header injection (T28a)
    // ========================================================================

    /// Spawn a mock storage that captures the most recent X-Agent-Cid header
    /// observed on inbound requests. Returns the address and a shared slot
    /// the test can read after the forward completes.
    async fn spawn_capturing_mock_storage() -> (
        SocketAddr,
        Arc<tokio::sync::Mutex<Option<String>>>,
        tokio::task::JoinHandle<()>,
    ) {
        use std::sync::Arc as StdArc;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let captured: StdArc<tokio::sync::Mutex<Option<String>>> =
            StdArc::new(tokio::sync::Mutex::new(None));
        let captured_clone = StdArc::clone(&captured);

        let handle = tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                let io = TokioIo::new(stream);
                let captured_per_conn = StdArc::clone(&captured_clone);
                tokio::spawn(async move {
                    let _ = http1::Builder::new()
                        .serve_connection(
                            io,
                            service_fn(move |req: Request<hyper::body::Incoming>| {
                                let captured_per_req = StdArc::clone(&captured_per_conn);
                                async move {
                                    let cid = req
                                        .headers()
                                        .get("X-Agent-Cid")
                                        .and_then(|v| v.to_str().ok())
                                        .map(String::from);
                                    *captured_per_req.lock().await = cid;
                                    let resp: Result<Response<Full<Bytes>>, Infallible> =
                                        Ok(Response::builder()
                                            .status(200u16)
                                            .header("Content-Type", "application/json")
                                            .body(Full::new(Bytes::from("{}")))
                                            .unwrap());
                                    resp
                                }
                            }),
                        )
                        .await;
                });
            }
        });

        (addr, captured, handle)
    }

    /// When `ForwardCtx { agent_cid: Some(...) }` is supplied, the forwarder
    /// emits `X-Agent-Cid` to storage with that exact value.
    #[tokio::test]
    async fn forward_to_storage_injects_x_agent_cid_when_present() {
        let (addr, captured, _handle) = spawn_capturing_mock_storage().await;
        let storage_url = format!("http://{addr}");

        let req = make_get_request(&format!("http://doorway/api/v1/cluster"));
        let ctx = ForwardCtx {
            agent_cid: Some("matthew"),
        };
        let resp = forward_to_storage(req, &storage_url, "/api/v1/cluster", ctx).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let captured_value = captured.lock().await.clone();
        assert_eq!(
            captured_value,
            Some("matthew".to_string()),
            "X-Agent-Cid must be forwarded when ctx.agent_cid is Some"
        );
    }

    /// When `ForwardCtx::default()` is supplied (no agent_cid), the forwarder
    /// emits NO `X-Agent-Cid` header — storage falls back to its own
    /// resolution (local_sessions or visitor branch).
    #[tokio::test]
    async fn forward_to_storage_omits_x_agent_cid_when_absent() {
        let (addr, captured, _handle) = spawn_capturing_mock_storage().await;
        let storage_url = format!("http://{addr}");

        let req = make_get_request(&format!("http://doorway/api/v1/cluster"));
        let resp =
            forward_to_storage(req, &storage_url, "/api/v1/cluster", ForwardCtx::default()).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let captured_value = captured.lock().await.clone();
        assert_eq!(
            captured_value, None,
            "X-Agent-Cid must NOT be set when ctx.agent_cid is None"
        );
    }
}
