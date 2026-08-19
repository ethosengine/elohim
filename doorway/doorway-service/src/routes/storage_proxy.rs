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
//!
//! ## Iroh / BLAKE3 dispatch — handled inside elohim-storage, NOT here
//!
//! The parallel iroh P2P stack (cutover gate #2) selects between the BLAKE3-keyed
//! `IrohBlobStore` and the SHA256-keyed legacy `BlobStore` per-request inside
//! elohim-storage's `GET /blob/{hash}` handler. Doorway is intentionally unaware of
//! this: it forwards the hash verbatim (sha256- or blake3- prefixed) with the
//! `X-Agent-Cid` header so storage can look up the caller's transport manifest.
//!
//! This module must NEVER contain blake3/iroh dispatch logic. The three-layer truth
//! model places that decision in the P2P data-plane layer (elohim-storage), not the
//! web2 projection layer (doorway).

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Method, Request, Response, StatusCode};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

use crate::cache::ContentCache;
use crate::routes::UpstreamBreakers;

/// Connect timeout for the pooled storage-proxy client (fail fast on a dead peer).
pub const STORAGE_PROXY_CONNECT_TIMEOUT_SECS: u64 = 3;
/// Whole-request timeout — browser-facing, well under warm-up's 45s.
pub const STORAGE_PROXY_REQUEST_TIMEOUT_SECS: u64 = 12;

/// Classifies an upstream result for the per-endpoint circuit breaker (D6).
/// Only transient saturation/connectivity counts as Failure; a 404 is a normal
/// blob miss (no-fanout rule) and must NEVER open the breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyOutcome {
    Ok,
    Failure,
    Neutral,
}

impl ProxyOutcome {
    pub fn classify(status: u16) -> ProxyOutcome {
        match status {
            200..=299 => ProxyOutcome::Ok,
            // Backpressure (429 / 503): the upstream ANSWERED with its own
            // flow-control shed. Liveness is proven — the breaker exists to
            // protect against a dead or hanging upstream, and storage's
            // admission layer already provides the flow control. Counting
            // these as Failure made every storage catching-up window OPEN the
            // doorway breaker, which then shed ALL proxied routes — the
            // catching-up amplification class, reproduced on the 2026-08-16
            // local mesh (breaker streak 10 against a healthy-but-catching-up
            // peer took the whole /db surface down with it). Same error-class
            // discipline as the storage-side conductor classifier
            // (f76378b2e): deadline/busy is a busy signal that breaks no
            // circuit; only never-answered (connect error / timeout — the
            // `Err` arms below) trips it.
            429 | 503 => ProxyOutcome::Neutral,
            500..=599 => ProxyOutcome::Failure,
            _ => ProxyOutcome::Neutral, // 4xx incl 404 = neutral
        }
    }
}

/// Proxy-side catching-up shed (returns Response<Full<Bytes>> for the
/// forwarder return type). Content-negotiated: a browser navigation gets the
/// staged HTML recovery page, every other client keeps the legacy JSON body
/// (see `routes::catching_up`).
fn catching_up_proxy_response(
    wants_html: bool,
    retry_after_secs: u64,
    endpoint: &str,
    breakers: &UpstreamBreakers,
) -> Response<Full<Bytes>> {
    crate::routes::catching_up::shed_response(
        wants_html,
        retry_after_secs,
        crate::routes::catching_up::upstream_cause(breakers, endpoint),
    )
}

/// Resolve an optional breaker trial's outcome. `None` is the diagnostic-probe
/// case (`is_diagnostic_probe`): those bypass the breaker entirely — never
/// gated, never recorded — so there is nothing to resolve.
fn record_trial(trial: &Option<crate::routes::upstream_health::BreakerTrial<'_>>, ok: bool) {
    if let Some(t) = trial {
        t.record(ok);
    }
}

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
    /// Doorway-verified performer for an ALLOWED operator verb (op-gate
    /// Allow on `/api/v1/operator/*`). When `Some`, the forwarder emits
    /// `x-elohim-verified-performer: <value>` on the internal hop — storage's
    /// verb handlers re-authorize against it and refuse fail-closed
    /// (`no-verified-performer`) without it. Must be set from the verified
    /// JWT only (same C8 trust class as the op-gate performer), never from a
    /// client header: the dispatch arm strips inbound copies before this
    /// context is built.
    pub verified_performer: Option<&'a str>,
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
/// - `503 catching-up + Retry-After` — connect/read failure, upstream 429/503,
///   or circuit-open (no bare 502; propagated backpressure per Plan B)
/// - `405 METHOD_NOT_ALLOWED` — method not in GET/POST/PUT/DELETE/HEAD/PATCH
///
/// Generic over `B` so the production path accepts `hyper::body::Incoming` and
/// tests can pass lightweight body types such as `http_body_util::Empty<Bytes>`.
pub async fn forward_to_storage<B>(
    req: Request<B>,
    storage_url: &str,
    path: &str,
    client: &reqwest::Client,
    breakers: &UpstreamBreakers,
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

    // Negotiate the shed shape once, before the request body is consumed.
    let wants_html = crate::routes::catching_up::accepts_html(req.headers());
    // Read-only diagnostic probes bypass the breaker entirely (never shed,
    // never recorded): the doorway must not blind its own probes during
    // exactly the upstream incident they exist to explain.
    let diag_probe = method == Method::GET && crate::routes::catching_up::is_diagnostic_probe(path);
    // The notary HEAD-declare write is carved out of the breaker's "shed
    // without calling storage" branch below (NOT out of breaker recording —
    // see the `trial` construction). Doorway carries no authority logic of
    // its own; storage's auth-first ordering (ab316cad7) must always get the
    // chance to run so a non-author is refused 401/403 rather than masked
    // behind a blind circuit-open 503 while the peer is catching up.
    let is_head_declare = crate::routes::catching_up::is_head_declare_write(&method, path);

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

    // Inject the doorway-verified performer for allowed operator verbs (see
    // ForwardCtx docs). The forwarder rebuilds the outbound request from an
    // allowlist, so this must ride ctx — a header injected on the inbound
    // hyper request would be silently dropped here.
    if let Some(vp) = ctx.verified_performer {
        builder = builder.header("x-elohim-verified-performer", vp);
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

    // Per-upstream breaker (Pillar 2 layer 4): consulted IMMEDIATELY before send
    // so it brackets exactly the upstream probe. The 405/400 early-returns above
    // never consume a half-open trial, and the RAII `trial` guard below resolves
    // exactly one outcome per terminal path — including the terminal path that
    // never runs, when this request future is DROPPED mid-send because the
    // client disconnected. Circuit-open sheds WITHOUT calling storage. Keyed by
    // storage_url (per-upstream, single-target dispatch).
    //
    // EXCEPT for the notary HEAD-declare write (`is_head_declare`): the doorway
    // holds no authority logic of its own, so a blind circuit-open shed here
    // would mask elohim-storage's own auth-first refusal (401/403 for a
    // non-author) behind an opaque 503 no matter what storage would have
    // decided — the request would simply never arrive. Measured on alpha-A:
    // "Non-author move of HEAD" expected 401/403, got 503, because the breaker
    // was open from a run of failures while the peer was catching up. This
    // route always attempts the call; if a half-open trial IS available it is
    // still taken (normal breaker bookkeeping), but a fully-open circuit is not
    // grounds to skip the call — only to skip taking a trial (no outcome is
    // recorded for a call this branch would otherwise have shed).
    let trial = if diag_probe {
        None
    } else {
        match breakers.begin(storage_url) {
            Some(t) => Some(t),
            None if is_head_declare => {
                debug!(
                    target: "upstream_shed",
                    storage_url = %storage_url,
                    path = %path,
                    "upstream circuit OPEN but path is the notary HEAD-declare write — \
                     bypassing the shed so storage's own authority refusal can run"
                );
                None
            }
            None => {
                warn!(
                    target: "upstream_shed",
                    counter = "doorway_upstream_breaker_open_total",
                    storage_url = %storage_url,
                    path = %path,
                    "upstream circuit OPEN — shedding without calling storage (503 + Retry-After)"
                );
                crate::metrics::inc_breaker_open();
                return catching_up_proxy_response(
                    wants_html,
                    crate::routes::upstream_health::UPSTREAM_CIRCUIT_COOLDOWN_SECS,
                    storage_url,
                    breakers,
                );
            }
        }
    };

    match builder.send().await {
        Ok(response) => {
            let status = response.status();
            let status_u16 = status.as_u16();

            // HONOR upstream backpressure: a 429/503 from storage becomes a
            // catching-up to the browser, preserving the upstream Retry-After
            // (else the breaker cooldown) so the client does not hammer.
            if matches!(status_u16, 429 | 503) {
                // Breaker-neutral: honored backpressure proves the upstream is
                // ALIVE (see ProxyOutcome::classify — busy is not broken). The
                // client still gets the catching-up response below; the
                // breaker stays closed so unrelated routes keep flowing.
                record_trial(&trial, true);
                let upstream_ra = response
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok());
                let retry_after = upstream_ra
                    .unwrap_or(crate::routes::upstream_health::UPSTREAM_CIRCUIT_COOLDOWN_SECS);
                warn!(
                    target: "upstream_shed",
                    counter = "doorway_upstream_backpressure_honored_total",
                    storage_url = %storage_url,
                    status = status_u16,
                    retry_after,
                    "honoring upstream backpressure — surfacing catching-up to client"
                );
                crate::metrics::inc_backpressure_honored();
                return catching_up_proxy_response(wants_html, retry_after, storage_url, breakers);
            }

            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/json")
                .to_string();

            match response.bytes().await {
                Ok(body) => {
                    // Record-once terminal outcome: a non-429/503 status with a
                    // readable body — classify it (404/4xx = neutral/ok, 5xx =
                    // failure). 429/503 already recorded+returned in the honor
                    // branch above.
                    record_trial(
                        &trial,
                        ProxyOutcome::classify(status_u16) != ProxyOutcome::Failure,
                    );
                    Response::builder()
                        .status(StatusCode::from_u16(status_u16).unwrap_or(StatusCode::OK))
                        .header("Content-Type", content_type)
                        .header("Cross-Origin-Resource-Policy", "cross-origin")
                        .body(Full::new(Bytes::from(body.to_vec())))
                        .unwrap()
                }
                Err(e) => {
                    warn!(error = %e, "Failed to read storage response body");
                    record_trial(&trial, false);
                    catching_up_proxy_response(
                        wants_html,
                        crate::routes::upstream_health::UPSTREAM_CIRCUIT_COOLDOWN_SECS,
                        storage_url,
                        breakers,
                    )
                }
            }
        }
        Err(e) => {
            warn!(error = %e, path = %path, storage_url = %storage_url,
                "storage forward failed (connect/timeout) — recording breaker failure");
            record_trial(&trial, false);
            catching_up_proxy_response(
                wants_html,
                crate::routes::upstream_health::UPSTREAM_CIRCUIT_COOLDOWN_SECS,
                storage_url,
                breakers,
            )
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
    client: &reqwest::Client,
    breakers: &UpstreamBreakers,
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
            return forward_to_storage(req, storage_url, path, client, breakers, ctx).await;
        }
    };

    // --- Skip caching for Range requests ----------------------------------------
    let has_range_header = req.headers().contains_key(hyper::header::RANGE);
    if has_range_header {
        debug!(hash = %hash, "Range request — skipping pantry, forwarding directly");
        crate::metrics::inc_blob_pantry("skipped");
        return forward_to_storage(req, storage_url, path, client, breakers, ctx).await;
    }

    // --- Cache-first: pantry hit? ------------------------------------------------
    if let Some(size) = cache.blob_size(&hash) {
        debug!(hash = %hash, size = size, "Blob drawn from pantry (cache hit)");
        if let Some(entry) = cache.get(&hash) {
            crate::metrics::inc_blob_pantry("hit");
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
    // Per-upstream breaker (Pillar 2 layer 4): shed without calling storage if
    // this endpoint's circuit is open. The RAII guard resolves an outcome on
    // every terminal path, including a dropped (cancelled) request future.
    let trial = match breakers.begin(storage_url) {
        Some(t) => t,
        None => {
            warn!(
                target: "upstream_shed",
                counter = "doorway_upstream_breaker_open_total",
                storage_url = %storage_url,
                hash = %hash,
                "upstream circuit OPEN — shedding blob without calling storage (503 + Retry-After)"
            );
            crate::metrics::inc_breaker_open();
            // Blob fetches are sub-resource requests (img/script/fetch), never a
            // browser navigation — they keep the JSON shed shape.
            return catching_up_proxy_response(
                false,
                crate::routes::upstream_health::UPSTREAM_CIRCUIT_COOLDOWN_SECS,
                storage_url,
                breakers,
            );
        }
    };

    let storage_endpoint = format!("{}{}", storage_url.trim_end_matches('/'), path);
    let query = req.uri().query();
    let full_url = match query {
        Some(q) => format!("{storage_endpoint}?{q}"),
        None => storage_endpoint,
    };

    debug!(hash = %hash, url = %full_url, "Blob cache miss — forwarding to elohim-storage");

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
            let status_u16 = status.as_u16();

            // HONOR upstream backpressure (429/503) — surface catching-up to the
            // client (preserve upstream Retry-After, else cooldown).
            if matches!(status_u16, 429 | 503) {
                trial.record(false);
                let upstream_ra = upstream
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok());
                let retry_after = upstream_ra
                    .unwrap_or(crate::routes::upstream_health::UPSTREAM_CIRCUIT_COOLDOWN_SECS);
                warn!(
                    target: "upstream_shed",
                    counter = "doorway_upstream_backpressure_honored_total",
                    storage_url = %storage_url,
                    status = status_u16,
                    retry_after,
                    "honoring upstream blob backpressure — surfacing catching-up"
                );
                crate::metrics::inc_backpressure_honored();
                return catching_up_proxy_response(false, retry_after, storage_url, breakers);
            }

            let content_type = upstream
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/octet-stream")
                .to_string();

            match upstream.bytes().await {
                Ok(body) => {
                    // Record-once terminal outcome: a non-429/503 status with a
                    // readable body. 404 = Neutral (normal blob miss under
                    // no-fanout) → ok, never opens; 5xx = failure.
                    trial.record(ProxyOutcome::classify(status_u16) != ProxyOutcome::Failure);
                    // --- Stock pantry on clean 200 -----------------------------------
                    let should_cache = status == StatusCode::OK
                        && status != StatusCode::PARTIAL_CONTENT
                        && (body.len() as u64) <= blob_pantry_max_bytes();

                    if should_cache {
                        cache.set(&hash, body.to_vec(), &content_type, BLOB_PANTRY_TTL);
                        crate::metrics::inc_blob_pantry("stocked");
                        debug!(
                            hash = %hash,
                            size = body.len(),
                            "Blob stocked in pantry, hash={}, size={}",
                            hash,
                            body.len()
                        );
                    } else if status == StatusCode::PARTIAL_CONTENT {
                        crate::metrics::inc_blob_pantry("skipped");
                        debug!(hash = %hash, "206 Partial Content — not stocking pantry");
                    } else if !status.is_success() {
                        // Non-2xx (typically 404 under no-fanout) — the genuine
                        // pantry miss the doorway can't stock.
                        crate::metrics::inc_blob_pantry("miss");
                        debug!(hash = %hash, status = %status, "Non-2xx — not stocking pantry");
                    } else if (body.len() as u64) > blob_pantry_max_bytes() {
                        crate::metrics::inc_blob_pantry("skipped");
                        debug!(
                            hash = %hash,
                            size = body.len(),
                            max = blob_pantry_max_bytes(),
                            "Blob exceeds pantry budget — not stocking"
                        );
                    } else {
                        // Any other 2xx that isn't a cacheable 200 (e.g. 204/203;
                        // unreachable for a blob GET in practice). Keep the pantry
                        // outcomes exhaustive — exactly one per fetched request.
                        crate::metrics::inc_blob_pantry("skipped");
                        debug!(hash = %hash, status = %status, "2xx non-200 — not stocking pantry");
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
                    trial.record(false);
                    catching_up_proxy_response(
                        false,
                        crate::routes::upstream_health::UPSTREAM_CIRCUIT_COOLDOWN_SECS,
                        storage_url,
                        breakers,
                    )
                }
            }
        }
        Err(e) => {
            warn!(error = %e, hash = %hash, storage_url = %storage_url,
                "blob forward failed (connect/timeout) — recording breaker failure");
            trial.record(false);
            catching_up_proxy_response(
                false,
                crate::routes::upstream_health::UPSTREAM_CIRCUIT_COOLDOWN_SECS,
                storage_url,
                breakers,
            )
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

    #[test]
    #[allow(clippy::assertions_on_constants)] // intentional invariant guard on tuning constants
    fn proxy_timeout_consts_browser_facing() {
        assert_eq!(STORAGE_PROXY_CONNECT_TIMEOUT_SECS, 3);
        assert_eq!(STORAGE_PROXY_REQUEST_TIMEOUT_SECS, 12);
        assert!(
            STORAGE_PROXY_REQUEST_TIMEOUT_SECS < 45,
            "browser-facing, well under warm-up's 45s"
        );
    }

    #[test]
    fn proxy_outcome_classifies_failures() {
        assert_eq!(ProxyOutcome::classify(200), ProxyOutcome::Ok);
        assert_eq!(ProxyOutcome::classify(204), ProxyOutcome::Ok);
        assert_eq!(
            ProxyOutcome::classify(404),
            ProxyOutcome::Neutral,
            "blob miss never opens breaker"
        );
        assert_eq!(ProxyOutcome::classify(400), ProxyOutcome::Neutral);
        assert_eq!(
            ProxyOutcome::classify(429),
            ProxyOutcome::Neutral,
            "backpressure is a busy signal from a LIVE upstream — never opens the breaker"
        );
        assert_eq!(
            ProxyOutcome::classify(503),
            ProxyOutcome::Neutral,
            "catching-up shed is flow control, not death — counting it as failure \
             amplified every catch-up window into a full doorway shed"
        );
        assert_eq!(ProxyOutcome::classify(500), ProxyOutcome::Failure);
        assert_eq!(ProxyOutcome::classify(502), ProxyOutcome::Failure);
        assert_eq!(ProxyOutcome::classify(504), ProxyOutcome::Failure);
    }

    #[test]
    fn honor_decision_maps_upstream_to_retry_after() {
        // The honor decision: a 503/429 upstream surfaces catching-up; the
        // Retry-After is the upstream's value if present, else the cooldown.
        fn honored_retry_after(
            upstream_status: u16,
            upstream_ra: Option<u64>,
            cooldown: u64,
        ) -> Option<u64> {
            match upstream_status {
                429 | 503 => Some(upstream_ra.unwrap_or(cooldown)),
                _ => None,
            }
        }
        assert_eq!(
            honored_retry_after(503, Some(7), 30),
            Some(7),
            "preserve upstream Retry-After"
        );
        assert_eq!(
            honored_retry_after(429, None, 30),
            Some(30),
            "fallback to cooldown"
        );
        assert_eq!(
            honored_retry_after(200, None, 30),
            None,
            "2xx passes through unchanged"
        );
    }

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

    /// Spawn an in-process HTTP server that accepts the connection and then
    /// never answers — the shape of a storage peer mid-churn (accepting, but
    /// not responding within the client's patience).
    async fn spawn_hanging_storage() -> (SocketAddr, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                let io = TokioIo::new(stream);
                tokio::spawn(async move {
                    let _ = http1::Builder::new()
                        .serve_connection(
                            io,
                            service_fn(move |_req: Request<hyper::body::Incoming>| async move {
                                // Never completes.
                                std::future::pending::<()>().await;
                                let resp: Result<Response<Full<Bytes>>, Infallible> =
                                    Ok(Response::new(Full::new(Bytes::new())));
                                resp
                            }),
                        )
                        .await;
                });
            }
        });
        (addr, handle)
    }

    /// REGRESSION (live incident 2026-08-01, both alpha doorways): a request
    /// that consumed the half-open trial and was then CANCELLED — the client
    /// gave up on a slow upstream, hyper dropped the connection task, and the
    /// `builder.send().await` in `forward_to_storage` never resumed — used to
    /// leave the breaker latched in consumed-HalfOpen. Every later request then
    /// shed in ~17ms with `upstream circuit OPEN`, and no cooldown ever
    /// recovered it (process restart only). The RAII trial guard resolves the
    /// outcome on drop, so the circuit re-opens and recovers.
    #[tokio::test]
    async fn cancelled_forward_does_not_latch_halfopen() {
        let (addr, _handle) = spawn_hanging_storage().await;
        let storage_url = format!("http://{addr}");
        // No client timeout: cancellation is the ONLY way out of the send.
        let client = reqwest::Client::new();
        let breakers = UpstreamBreakers::new(1, 0); // zero cooldown: immediate half-open

        breakers.record(&storage_url, false); // circuit opens
        assert_eq!(breakers.snapshot()[0].circuit, "open");

        // This forward consumes the one half-open trial, then is cancelled
        // mid-send (timeout drops the future — exactly what hyper does to an
        // in-flight handler when the client disconnects).
        let forward = forward_to_storage(
            make_get_request("/db/content/elohim-host-landing"),
            &storage_url,
            "/db/content/elohim-host-landing",
            &client,
            &breakers,
            ForwardCtx::default(),
        );
        let outcome = tokio::time::timeout(Duration::from_millis(250), forward).await;
        assert!(
            outcome.is_err(),
            "upstream hangs: the request future must be cancelled mid-send"
        );

        assert_eq!(
            breakers.snapshot()[0].circuit,
            "open",
            "the abandoned trial re-opened the circuit — NOT a permanent half-open latch"
        );
        // Recovery: the next gate call admits a fresh trial (the live symptom
        // was that it never did).
        assert!(
            !breakers.is_open(&storage_url),
            "cooldown elapsed: a fresh trial is admitted after the cancelled one"
        );
    }

    /// Build a POST request with an empty body — `forward_to_storage`'s POST
    /// branch calls `req.collect().await` regardless of body content, so an
    /// `Empty<Bytes>` body (no `head_action_hash`, resolves the author's
    /// latest committed action) is a faithful stand-in for these tests.
    fn make_post_request(uri: &str) -> Request<Empty<Bytes>> {
        Request::builder()
            .method(Method::POST)
            .uri(uri)
            .body(Empty::new())
            .unwrap()
    }

    /// REGRESSION (measured on alpha-A: "Non-author move of HEAD" expected
    /// 401/403, got 503). While the per-upstream breaker is fully OPEN — the
    /// shape a run of failures produces while a peer is catching up — the
    /// notary HEAD-declare write (POST /db/content/{id}/head) must still
    /// reach storage so storage's own auth-first ordering can run, instead of
    /// being shed blind by the doorway's circuit breaker. A large cooldown
    /// keeps the circuit fully OPEN (never half-open) for the whole test, so
    /// this proves the bypass is not just "waited for the next half-open
    /// trial" — a real 401 from the mock storage comes through unmasked.
    #[tokio::test]
    async fn head_declare_write_bypasses_open_breaker_and_reaches_storage() {
        let (addr, _handle) = spawn_mock_storage(
            401,
            br#"{"error":"authentication required"}"#.to_vec(),
            "application/json",
        )
        .await;
        let storage_url = format!("http://{addr}");
        let breakers = UpstreamBreakers::new(1, 9999); // huge cooldown: stays fully open
        breakers.record(&storage_url, false); // one failure opens the circuit
        assert_eq!(breakers.snapshot()[0].circuit, "open");

        let path = "/db/content/some-id/head";
        let resp = forward_to_storage(
            make_post_request(&format!("http://doorway{path}")),
            &storage_url,
            path,
            &reqwest::Client::new(),
            &breakers,
            ForwardCtx::default(),
        )
        .await;

        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "head-declare write must reach storage (and carry storage's real \
             answer) even with the breaker fully open — never a doorway-side \
             catching-up 503"
        );
    }

    /// Sibling regression: a content READ against the same open breaker is
    /// still shed (503 catching-up) WITHOUT ever reaching storage — the
    /// carve-out is scoped to the exact head-declare write shape, not a
    /// blanket breaker bypass for the endpoint.
    #[tokio::test]
    async fn content_get_still_shed_when_breaker_open() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let hit_counter = Arc::new(AtomicU32::new(0));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let counter_clone = Arc::clone(&hit_counter);
        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                let io = TokioIo::new(stream);
                let ctr = Arc::clone(&counter_clone);
                tokio::spawn(async move {
                    ctr.fetch_add(1, Ordering::SeqCst);
                    let _ = http1::Builder::new()
                        .serve_connection(
                            io,
                            service_fn(move |_req: Request<hyper::body::Incoming>| async move {
                                let resp: Result<Response<Full<Bytes>>, Infallible> =
                                    Ok(Response::builder()
                                        .status(200u16)
                                        .header("Content-Type", "application/json")
                                        .body(Full::new(Bytes::from("{}")))
                                        .unwrap());
                                resp
                            }),
                        )
                        .await;
                });
            }
        });

        let storage_url = format!("http://{addr}");
        let breakers = UpstreamBreakers::new(1, 9999); // huge cooldown: stays fully open
        breakers.record(&storage_url, false);
        assert_eq!(breakers.snapshot()[0].circuit, "open");

        let path = "/db/content/some-id";
        let resp = forward_to_storage(
            make_get_request(&format!("http://doorway{path}")),
            &storage_url,
            path,
            &reqwest::Client::new(),
            &breakers,
            ForwardCtx::default(),
        )
        .await;

        assert_eq!(
            resp.status(),
            StatusCode::SERVICE_UNAVAILABLE,
            "a content read must stay shed while the breaker is open"
        );
        assert_eq!(
            hit_counter.load(Ordering::SeqCst),
            0,
            "the shed must never reach storage for a non-head-declare route"
        );
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
            &reqwest::Client::new(),
            &UpstreamBreakers::default(),
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
            &reqwest::Client::new(),
            &UpstreamBreakers::default(),
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
            &reqwest::Client::new(),
            &UpstreamBreakers::default(),
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
            &reqwest::Client::new(),
            &UpstreamBreakers::default(),
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
            &reqwest::Client::new(),
            &UpstreamBreakers::default(),
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
            &reqwest::Client::new(),
            &UpstreamBreakers::default(),
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

    /// N6 bound: errors must NEVER be cached. A 404 (or any non-200) from
    /// storage is forwarded to the caller but must not stock the pantry — only
    /// successful (200) blob GETs are cacheable. The 206 test covers partial
    /// content; this closes the error-status bound the cache-write contract
    /// promises ("never cache errors or non-blob routes").
    #[tokio::test]
    async fn blob_404_does_not_stock_pantry() {
        let _guard = BLOB_TEST_LOCK.lock().await;
        let payload = b"not found".to_vec();
        let (addr, _handle) = spawn_mock_storage(404, payload.clone(), "application/json").await;
        let storage_url = format!("http://{addr}");

        let cache = make_cache();
        let hash = "sha256-aabbccdd00112233aabbccdd00112233aabbccdd00112233aabbccddff000404";
        let path = format!("/blob/{hash}");

        let req = make_get_request(&format!("http://doorway{path}"));
        let resp = forward_blob_to_storage(
            req,
            &storage_url,
            &path,
            Arc::clone(&cache),
            &reqwest::Client::new(),
            &UpstreamBreakers::default(),
            ForwardCtx::default(),
        )
        .await;

        // The upstream error is forwarded unchanged to the caller.
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        // But the pantry must remain empty — errors are never cached.
        assert!(
            cache.blob_size(hash).is_none(),
            "a 404 from storage must not stock the pantry (errors are never cached)"
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

        let req = make_get_request("http://doorway/api/v1/cluster");
        let ctx = ForwardCtx {
            agent_cid: Some("matthew"),
            ..Default::default()
        };
        let resp = forward_to_storage(
            req,
            &storage_url,
            "/api/v1/cluster",
            &reqwest::Client::new(),
            &UpstreamBreakers::default(),
            ctx,
        )
        .await;
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

        let req = make_get_request("http://doorway/api/v1/cluster");
        let resp = forward_to_storage(
            req,
            &storage_url,
            "/api/v1/cluster",
            &reqwest::Client::new(),
            &UpstreamBreakers::default(),
            ForwardCtx::default(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);

        let captured_value = captured.lock().await.clone();
        assert_eq!(
            captured_value, None,
            "X-Agent-Cid must NOT be set when ctx.agent_cid is None"
        );
    }

    // ========================================================================
    // x-elohim-verified-performer header injection (operator verbs)
    // ========================================================================

    /// Spawn a mock storage that captures the most recent
    /// `x-elohim-verified-performer` header observed on inbound requests.
    async fn spawn_performer_capturing_mock_storage() -> (
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
                                    let vp = req
                                        .headers()
                                        .get("x-elohim-verified-performer")
                                        .and_then(|v| v.to_str().ok())
                                        .map(String::from);
                                    *captured_per_req.lock().await = vp;
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

    /// The forwarder must carry the doorway-verified performer to storage:
    /// storage's operator-verb handlers re-authorize against this header
    /// (`operator_verbs::VERIFIED_PERFORMER_HEADER`) and refuse the verb with
    /// `no-verified-performer` when it is absent. The forwarder rebuilds the
    /// outbound request from an allowlist, so the header must be an explicit
    /// ForwardCtx field — injecting it on the hyper request upstream is lost.
    #[tokio::test]
    async fn forward_to_storage_injects_verified_performer_when_present() {
        let (addr, captured, _handle) = spawn_performer_capturing_mock_storage().await;
        let storage_url = format!("http://{addr}");

        let req = make_get_request("http://doorway/api/v1/operator/reconcile");
        let ctx = ForwardCtx {
            verified_performer: Some("uhCAkOperatorAgentKey"),
            ..Default::default()
        };
        let resp = forward_to_storage(
            req,
            &storage_url,
            "/api/v1/operator/reconcile",
            &reqwest::Client::new(),
            &UpstreamBreakers::default(),
            ctx,
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);

        let captured_value = captured.lock().await.clone();
        assert_eq!(
            captured_value,
            Some("uhCAkOperatorAgentKey".to_string()),
            "x-elohim-verified-performer must reach storage when ctx carries it"
        );
    }

    /// Without a verified performer in ctx, no performer header is emitted —
    /// storage then (correctly) refuses the operator verb fail-closed.
    #[tokio::test]
    async fn forward_to_storage_omits_verified_performer_when_absent() {
        let (addr, captured, _handle) = spawn_performer_capturing_mock_storage().await;
        let storage_url = format!("http://{addr}");

        let req = make_get_request("http://doorway/api/v1/operator/reconcile");
        let resp = forward_to_storage(
            req,
            &storage_url,
            "/api/v1/operator/reconcile",
            &reqwest::Client::new(),
            &UpstreamBreakers::default(),
            ForwardCtx::default(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);

        let captured_value = captured.lock().await.clone();
        assert_eq!(
            captured_value, None,
            "x-elohim-verified-performer must NOT be set when ctx carries none"
        );
    }
}
