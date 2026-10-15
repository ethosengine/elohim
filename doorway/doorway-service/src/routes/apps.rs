//! HTML5 App Route Handler — Projection Cache with Storage Fallback
//!
//! Serves HTML5 app assets (JS, CSS, images) from a MongoDB projection cache
//! with transparent fallback to elohim-storage for cache misses. When a browser
//! loads an HTML5 app (e.g., a Sophia quiz), it fires 30+ concurrent requests
//! for assets — the cache absorbs this web2 traffic pattern so storage stays
//! focused on P2P.
//!
//! ## Cache-First Flow
//!
//! ```text
//! Browser → Doorway
//!              ├─ Cache HIT  → serve from MongoDB         (X-Cache: HIT)
//!              ├─ Coalesced  → wait for in-flight fetch    (X-Cache: HIT-COALESCED)
//!              ├─ Cache MISS → fetch from storage, cache   (X-Cache: MISS)
//!              └─ No cache   → direct proxy to storage     (X-Cache: BYPASS)
//! ```
//!
//! ## Endpoints
//!
//! - GET /apps/{slug}/{path} - Serve file from HTML5 app ZIP

use bytes::Bytes;
use http_body_util::Full;
use hyper::header::{HeaderMap, HeaderName, HeaderValue};
use hyper::{Response, StatusCode};
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::cache::AppDescriptor;
use crate::server::AppState;

/// Check whether an identifier looks like a content address (CID) rather than a slug.
///
/// Content addresses use the `sha256-{hex}` format from the storage layer.
/// The length check (>10) avoids false positives on short strings.
fn is_content_address(identifier: &str) -> bool {
    identifier.starts_with("sha256-") && identifier.len() > 10
}

/// Maximum retries for transient failures (502/connection errors).
/// HTML5 apps load 30+ assets concurrently which can overwhelm storage
/// during extraction; a brief retry absorbs the back-pressure.
const MAX_RETRIES: u32 = 2;

/// Base delay between retries (doubles each attempt).
const RETRY_BASE_DELAY_MS: u64 = 100;

/// How long one storage reach verdict for an app is reused by the cache path.
/// A row whose reach narrows is honoured within this window.
const APP_REACH_VERDICT_TTL: std::time::Duration = std::time::Duration::from_secs(30);

/// Storage's reach verdicts for app identifiers, keyed by `storage_url|identifier`:
/// `true` = storage refused an anonymous caller.
static APP_REACH_VERDICTS: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<String, (std::time::Instant, bool)>>,
> = std::sync::LazyLock::new(Default::default);

/// The reach gate in front of the projection cache. The doorway owns no
/// verifier: it asks storage — the one place `/apps/{x}` is judged exactly as
/// `/db/content/{x}` or `/blob/{x}` would be — whether an anonymous caller
/// may have this app, via the cheap `HEAD /apps/{x}/_capability` probe, and
/// reuses the answer for [`APP_REACH_VERDICT_TTL`]. Without this a cache hit
/// served whatever had once been stocked, whatever the row's reach now says.
///
/// Every cache read is anonymous (the cache stocks with no identity), so an
/// explicit 403 means the cache may not serve it to anyone; the request is
/// refused rather than proxied. A probe that fails in transport keeps serving
/// (storage's own fail-open posture for an unreadable projection), logged.
async fn cached_app_reach_refusal(
    storage_url: &str,
    identifier: &str,
) -> Option<Response<Full<Bytes>>> {
    let key = format!("{storage_url}|{identifier}");
    let fresh = APP_REACH_VERDICTS
        .lock()
        .ok()
        .and_then(|m| m.get(&key).copied())
        .filter(|(at, _)| at.elapsed() < APP_REACH_VERDICT_TTL)
        .map(|(_, refused)| refused);
    let refused = match fresh {
        Some(refused) => refused,
        None => {
            let probe = format!(
                "{}/apps/{}/_capability",
                storage_url.trim_end_matches('/'),
                identifier
            );
            match reqwest::Client::new()
                .head(&probe)
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await
            {
                Ok(resp) => {
                    let refused = resp.status() == reqwest::StatusCode::FORBIDDEN;
                    if let Ok(mut m) = APP_REACH_VERDICTS.lock() {
                        m.insert(key, (std::time::Instant::now(), refused));
                    }
                    refused
                }
                Err(e) => {
                    warn!(identifier = %identifier, error = %e, "app reach probe failed; serving (fail-open residual)");
                    false
                }
            }
        }
    };
    refused.then(|| {
        Response::builder()
            .status(StatusCode::FORBIDDEN)
            .header("Content-Type", "application/json")
            .header("X-Cache", "REFUSED")
            .body(Full::new(Bytes::from(
                r#"{"error":"Reach authorization required"}"#,
            )))
            .unwrap()
    })
}

/// Handle app requests with cache-first resolution.
///
/// 1. Parse path into slug and file_path
/// 2. Resolve blob_hash from slug index (projection store lookup)
/// 3. If cache available AND blob_hash known: try cache → coalesce → fetch
/// 4. Otherwise: fall through to direct proxy (BYPASS)
///
/// A person who opens an html5-app's entry file directly as a page
/// (`Sec-Fetch-Dest: document` + `Sec-Fetch-Mode: navigate`) is answered with
/// the chrome shell instead of the bytes; every other request is served as
/// before. See [`chrome_shell_applies`].
pub async fn handle_app_request(
    state: Arc<AppState>,
    path: &str,
    headers: &HeaderMap,
) -> Response<Full<Bytes>> {
    let storage_url = match &state.args.storage_url {
        Some(url) => url.clone(),
        None => {
            warn!("Apps proxy called but STORAGE_URL not configured");
            return Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    r#"{"error": "Storage service not configured. Set STORAGE_URL env var."}"#,
                )))
                .unwrap();
        }
    };

    // Admin cache bypass — diagnostic tool for delivery testing.
    // When disabled via POST /admin/cache/disable, skip the projection cache
    // entirely and proxy directly to elohim-storage. Chrome selection still
    // applies: the bypass changes where bytes come from, not what a person
    // opening the page is shown.
    let cache_bypassed = !state
        .cache_enabled
        .load(std::sync::atomic::Ordering::Relaxed);

    // Parse /apps/{slug}/{file_path...}
    let (slug, file_path) = match parse_app_path(path) {
        Some(parsed) => parsed,
        None => {
            if cache_bypassed {
                return forward_app_request_with_header(&storage_url, path, "BYPASS-ADMIN").await;
            }
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Type", "application/json")
                .header("X-Cache", "BYPASS")
                .body(Full::new(Bytes::from(
                    r#"{"error": "Invalid app path. Expected /apps/{slug}/{file_path}"}"#,
                )))
                .unwrap();
        }
    };

    // Path traversal protection — defense in depth (storage also validates)
    if file_path.contains("..") || file_path.contains('\0') || file_path.starts_with('/') {
        if cache_bypassed {
            return forward_app_request_with_header(&storage_url, path, "BYPASS-ADMIN").await;
        }
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "application/json")
            .header("X-Cache", "BYPASS")
            .body(Full::new(Bytes::from(r#"{"error": "Invalid file path"}"#)))
            .unwrap();
    }

    // The projection descriptor is only worth reading for a request that could
    // be an entry page: a document navigation, or an html file.
    let descriptor = if is_document_navigation(headers) || looks_like_html(file_path) {
        match state.app_file_cache {
            Some(ref cache) if !is_content_address(slug) => {
                cache.resolve_app_descriptor(slug).await
            }
            _ => None,
        }
    } else {
        None
    };

    serve_app_file(
        &state,
        &storage_url,
        path,
        slug,
        file_path,
        headers,
        descriptor.as_ref(),
        cache_bypassed,
    )
    .await
}

/// The serve itself, with the projection descriptor already read.
#[allow(clippy::too_many_arguments)]
async fn serve_app_file(
    state: &Arc<AppState>,
    storage_url: &str,
    path: &str,
    slug: &str,
    file_path: &str,
    headers: &HeaderMap,
    descriptor: Option<&AppDescriptor>,
    cache_bypassed: bool,
) -> Response<Full<Bytes>> {
    let entry_page = descriptor.filter(|d| is_html5_entry(d, file_path));

    if let Some(desc) = entry_page {
        if is_document_navigation(headers) {
            // Reach before any shell output: the shell names the content
            // (title, address), so a refused app gets the refusal.
            if let Some(refused) = cached_app_reach_refusal(storage_url, slug).await {
                return refused;
            }
            return chrome_shell_response(storage_url, slug, desc).await;
        }
    }

    let mut response = if cache_bypassed {
        forward_app_request_with_header(storage_url, path, "BYPASS-ADMIN").await
    } else {
        serve_raw_app_file(state, storage_url, slug, file_path).await
    };

    // The same URL now answers with two representations; shared caches must
    // keep them apart.
    if entry_page.is_some() {
        response
            .headers_mut()
            .insert(hyper::header::VARY, HeaderValue::from_static(CHROME_VARY));
    }
    response
}

/// The raw (bytes) serve: projection cache first, storage as fallback.
async fn serve_raw_app_file(
    state: &Arc<AppState>,
    storage_url: &str,
    slug: &str,
    file_path: &str,
) -> Response<Full<Bytes>> {
    let storage_url = storage_url.to_string();

    // Dual-path resolution: if the identifier is already a content address (sha256-...),
    // use it directly as the blob_hash — no slug resolution needed.
    let (resolved_hash, content_slug) = if is_content_address(slug) {
        // CID path: /apps/{sha256-hash}/file — use hash directly
        (Some(slug.to_string()), None)
    } else {
        // Slug path: /apps/{slug}/file — resolve via cache
        let hash = if let Some(ref cache) = state.app_file_cache {
            cache.resolve_blob_hash(slug).await
        } else {
            None
        };
        (hash, Some(slug))
    };

    // Cache-first path: only active when cache service AND blob_hash are available.
    // blob_hash is resolved from the slug index (populated from projection store)
    // or directly from the content address in the URL.
    if let Some(ref cache) = state.app_file_cache {
        if let Some(ref hash) = resolved_hash {
            let cache_slug = content_slug.unwrap_or(slug);

            // Reach before the cache: a cache hit never widens an audience.
            if let Some(refused) = cached_app_reach_refusal(&storage_url, slug).await {
                return refused;
            }

            // --- Try cache lookup ---
            if let Some(cached) = cache.get(cache_slug, file_path, hash).await {
                debug!(slug = %cache_slug, file_path = %file_path, "App file cache HIT");
                return build_app_response(
                    &cached.data,
                    &cached.content_type,
                    "HIT",
                    Some(hash.as_str()),
                    content_slug,
                );
            }

            // --- Try in-flight coalescing ---
            match cache.begin_fetch(cache_slug, file_path) {
                Some(mut rx) => {
                    // Another task is already fetching this file — wait for it
                    debug!(
                        slug = %cache_slug,
                        file_path = %file_path,
                        "Coalescing app file fetch (waiting for leader)"
                    );
                    match rx.recv().await {
                        Ok(Some(cached)) => {
                            return build_app_response(
                                &cached.data,
                                &cached.content_type,
                                "HIT-COALESCED",
                                Some(hash.as_str()),
                                content_slug,
                            );
                        }
                        _ => {
                            // Leader failed or channel closed. The slug was already resolved,
                            // so keep the fallback pinned to the same release rather than
                            // re-resolving a moving slug in storage.
                            warn!(
                                slug = %cache_slug,
                                file_path = %file_path,
                                "Coalesced fetch failed, falling through to proxy"
                            );
                            let resolved_path = resolved_app_path(hash, file_path);
                            return forward_app_request_with_header(
                                &storage_url,
                                &resolved_path,
                                "MISS",
                            )
                            .await;
                        }
                    }
                }
                None => {
                    // We are the leader — fetch from storage, cache, broadcast
                    let response =
                        fetch_and_cache(cache, &storage_url, cache_slug, file_path, hash).await;
                    return response;
                }
            }
        }
    }

    // No cache or no blob_hash — direct proxy, of the parsed file (a directory URL
    // has already been resolved to its index.html).
    let parsed_path = format!("/apps/{slug}/{file_path}");
    forward_app_request_with_header(&storage_url, &parsed_path, "BYPASS").await
}

/// `Vary` carried by html5-app entry responses (bytes and shell alike): the
/// same URL answers differently for a page navigation and for a frame load.
const CHROME_VARY: &str = "Sec-Fetch-Dest, Sec-Fetch-Mode";

/// How long the shell waits for the holders figure before showing the unknown
/// state. The figure decorates the page; it never gates it.
const HOLDERS_FETCH_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(1500);

/// A person opening the URL as a page: Fetch Metadata says `document` +
/// `navigate`. Absent headers (curl, old browsers, service workers fetching
/// subresources) are NOT a navigation.
fn is_document_navigation(headers: &HeaderMap) -> bool {
    let is = |name: &str, want: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.trim().eq_ignore_ascii_case(want))
    };
    is("sec-fetch-dest", "document") && is("sec-fetch-mode", "navigate")
}

fn looks_like_html(file_path: &str) -> bool {
    let lower = file_path.to_ascii_lowercase();
    lower.ends_with(".html") || lower.ends_with(".htm")
}

/// The chrome branch's content test: an `html5-app` node, asked for its entry file.
fn is_html5_entry(desc: &AppDescriptor, file_path: &str) -> bool {
    desc.content_format == "html5-app" && file_path == desc.entry_file()
}

/// Escape a value for HTML text or a double-quoted attribute.
fn html_escape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for c in raw.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// Percent-encode a path segment (unreserved characters pass through).
fn encode_segment(raw: &str) -> String {
    raw.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            b => format!("%{b:02X}"),
        })
        .collect()
}

/// `(holding, wanted)` households for a content id from storage's household
/// resilience snapshot, or `None` for any failure or an unmeasured snapshot.
async fn fetch_holders(storage_url: &str, content_id: &str) -> Option<(i64, i64)> {
    if content_id.is_empty() {
        return None;
    }
    let url = format!(
        "{}/api/v1/resilience/{}/household",
        storage_url.trim_end_matches('/'),
        encode_segment(content_id)
    );
    let resp = reqwest::Client::new()
        .get(&url)
        .timeout(HOLDERS_FETCH_TIMEOUT)
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: serde_json::Value = resp.json().await.ok()?;
    let floor = body.get("feltStatus")?.get("floor")?;
    Some((
        floor.get("hasHouseholds")?.as_i64()?,
        floor.get("wantsHouseholds")?.as_i64()?,
    ))
}

/// The chrome shell document: the native omnibar, a thin bar carrying the
/// resiliency badge, and the raw bundle framed beneath. Every substituted value
/// is HTML-escaped.
fn render_chrome_shell(slug: &str, desc: &AppDescriptor, holders: Option<(i64, i64)>) -> String {
    let address = if desc.content_id.is_empty() {
        slug
    } else {
        desc.content_id.as_str()
    };
    let title = desc.title.as_deref().unwrap_or(address);
    let epr_href = format!("/epr/{}", encode_segment(address));
    let frame_src = format!("/apps/{}/{}", encode_segment(slug), desc.entry_file());

    let (badge_attrs, badge_text) = match holders {
        Some((n, m)) => (
            format!(" data-holders=\"{n}\" data-target=\"{m}\""),
            format!("Held by {n} of {m} households"),
        ),
        None => (
            String::new(),
            "Held by households \u{b7} see where it lives".to_string(),
        ),
    };

    let html = format!(
        "<!DOCTYPE html><html lang=\"en\" data-elohim-chrome=\"content-shell\"><head>\
<meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
<title>{title}</title>\
<style>html,body{{height:100%;margin:0}}\
body{{display:flex;flex-direction:column;font:14px/1.4 system-ui,sans-serif}}\
.elohim-content-chrome{{flex:none;padding:4px 12px;border-bottom:1px solid #8884}}\
.elohim-content-chrome a{{color:inherit}}\
iframe{{flex:1;width:100%;border:0}}</style></head><body>\
<nav class=\"elohim-content-chrome\" data-testid=\"content-chrome\" aria-label=\"About this content\">\
<a data-testid=\"resiliency-badge\" class=\"resiliency-badge\" href=\"{href}\"{badge_attrs}>{badge_text}</a></nav>\
<iframe data-testid=\"content-app-frame\" title=\"{title}\" src=\"{src}\" \
sandbox=\"allow-scripts allow-same-origin allow-forms allow-popups allow-downloads\"></iframe>\
</body></html>",
        title = html_escape(title),
        href = html_escape(&epr_href),
        src = html_escape(&frame_src),
    );

    let context = elohim_render::ChromeContext {
        slug: address.to_string(),
        authenticated: false,
        title: Some(title.to_string()),
        ..Default::default()
    }
    .to_json();
    elohim_render::inject_element(&html, &context)
}

/// Answer a page navigation to an html5-app entry with the chrome shell.
/// Never stocked in any cache: `no-store`, no validators, no address claims.
async fn chrome_shell_response(
    storage_url: &str,
    slug: &str,
    desc: &AppDescriptor,
) -> Response<Full<Bytes>> {
    let address = if desc.content_id.is_empty() {
        slug
    } else {
        desc.content_id.as_str()
    };
    let holders = fetch_holders(storage_url, address).await;
    let body = render_chrome_shell(slug, desc, holders);
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Cache-Control", "no-store")
        .header("Vary", CHROME_VARY)
        .header("X-Elohim-Chrome", "content-shell")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

/// Parse `/apps/{slug}/{file_path...}` into (slug, file_path).
///
/// Returns `None` if the path doesn't have at least a slug and one file
/// path segment.
fn parse_app_path(path: &str) -> Option<(&str, &str)> {
    // Strip leading "/apps/"
    let rest = path.strip_prefix("/apps/")?;
    if rest.is_empty() {
        return None;
    }

    // Split into slug and file_path at the first '/'
    let (slug, file_path) = match rest.find('/') {
        Some(idx) => {
            let (id, remainder) = rest.split_at(idx);
            // remainder starts with '/', strip it
            (id, &remainder[1..])
        }
        None => {
            // No file path segment — just a slug
            return None;
        }
    };

    if slug.is_empty() {
        return None;
    }

    // A directory URL names the app's front page, as it does on any web server:
    // `/apps/{slug}/` is the address a visitor opens, and the page's relative
    // asset paths resolve under it. (Bare `/apps/{slug}` stays refused — serving
    // the page there would resolve its relative assets one level too high.)
    if file_path.is_empty() {
        return Some((slug, "index.html"));
    }

    Some((slug, file_path))
}

/// Bind an app asset request to the release already selected by slug resolution.
///
/// A slug is a moving pointer. Once doorway has resolved it to a blob hash, a cache
/// miss must not ask storage to resolve the slug again: the pointer may have moved
/// between the two reads, producing an HTML shell from one release and assets from
/// another.
fn resolved_app_path(blob_hash: &str, file_path: &str) -> String {
    format!("/apps/{blob_hash}/{file_path}")
}

/// Fetch a file from storage, cache it, broadcast to waiters, and return the response.
async fn fetch_and_cache(
    cache: &crate::cache::AppFileCacheService,
    storage_url: &str,
    slug: &str,
    file_path: &str,
    blob_hash: &str,
) -> Response<Full<Bytes>> {
    let resolved_path = resolved_app_path(blob_hash, file_path);
    let storage_endpoint = format!("{}{}", storage_url.trim_end_matches('/'), resolved_path);
    let client = reqwest::Client::new();

    let result = client.get(&storage_endpoint).send().await;

    match result {
        Ok(response) => {
            let status = response.status();

            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/octet-stream")
                .to_string();

            // Read content addressing headers from upstream storage response
            let upstream_address = response
                .headers()
                .get("x-content-address")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            let upstream_slug = response
                .headers()
                .get("x-content-slug")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            match response.bytes().await {
                Ok(body) => {
                    if status.is_success() {
                        // Cache the file
                        cache
                            .put(slug, file_path, blob_hash, &content_type, body.to_vec())
                            .await;

                        // Broadcast to coalesced waiters
                        let cached_file = crate::cache::CachedFile {
                            data: body.to_vec(),
                            content_type: content_type.clone(),
                            blob_hash: blob_hash.to_string(),
                            // Mirrors the `put` above, which writes an unmarked
                            // doc. This path IS hash-addressed
                            // (`resolved_app_path`), so it could mark; leaving it
                            // unmarked only costs the warm-shell path one
                            // hash-addressed re-fetch per (slug, head).
                            head_bound: false,
                        };
                        cache.finish_fetch(slug, file_path, Some(cached_file));

                        debug!(
                            slug = %slug,
                            blob_hash = %blob_hash,
                            file_path = %file_path,
                            size = body.len(),
                            "App file cached (MISS)"
                        );

                        build_app_response(
                            &body,
                            &content_type,
                            "MISS",
                            upstream_address.as_deref(),
                            upstream_slug.as_deref(),
                        )
                    } else {
                        // Non-success status — don't cache, broadcast failure
                        cache.finish_fetch(slug, file_path, None);

                        Response::builder()
                            .status(
                                StatusCode::from_u16(status.as_u16())
                                    .unwrap_or(StatusCode::BAD_GATEWAY),
                            )
                            .header("Content-Type", &content_type)
                            .header("X-Cache", "MISS")
                            .body(Full::new(Bytes::from(body.to_vec())))
                            .unwrap()
                    }
                }
                Err(e) => {
                    cache.finish_fetch(slug, file_path, None);
                    warn!(error = %e, "Failed to read storage response body during cache fetch");
                    Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .header("Content-Type", "application/json")
                        .header("X-Cache", "MISS")
                        .body(Full::new(Bytes::from(format!(
                            r#"{{"error": "Failed to read storage response: {e}"}}"#
                        ))))
                        .unwrap()
                }
            }
        }
        Err(e) => {
            cache.finish_fetch(slug, file_path, None);
            warn!(error = %e, "Failed to connect to storage during cache fetch");
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("Content-Type", "application/json")
                .header("X-Cache", "MISS")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error": "Failed to connect to storage: {e}"}}"#
                ))))
                .unwrap()
        }
    }
}

/// Build an HTTP response for an app file with standard headers.
///
/// Sets COEP/CORP headers (required for cross-origin embedding in Angular),
/// cache control, and the X-Cache diagnostic header. Optionally includes
/// `X-Content-Address` and `X-Content-Slug` headers for content addressing.
fn build_app_response(
    data: &[u8],
    content_type: &str,
    cache_status: &str,
    blob_hash: Option<&str>,
    content_slug: Option<&str>,
) -> Response<Full<Bytes>> {
    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", content_type)
        .header("Cross-Origin-Resource-Policy", "cross-origin")
        .header("Cross-Origin-Embedder-Policy", "credentialless")
        .header("X-Cache", cache_status)
        .header("Cache-Control", "public, max-age=3600");

    if let Some(hash) = blob_hash {
        builder = builder.header("X-Content-Address", hash);
    }
    if let Some(slug) = content_slug {
        builder = builder.header("X-Content-Slug", slug);
    }

    // Provenance header: all publicly served app content is commons-reach.
    // Future: resolve per-content reach from projection cache.
    builder = builder.header("X-Reach", "commons");

    builder.body(Full::new(Bytes::from(data.to_vec()))).unwrap()
}

/// Forward a /apps/* request to elohim-storage with retry for transient errors.
///
/// This is the existing proxy path, preserved for BYPASS and fallback scenarios.
/// Adds the X-Cache header to the response.
async fn forward_app_request_with_header(
    storage_url: &str,
    path: &str,
    cache_status: &str,
) -> Response<Full<Bytes>> {
    let storage_endpoint = format!("{}{}", storage_url.trim_end_matches('/'), path);

    debug!(url = %storage_endpoint, "Forwarding app request to elohim-storage");

    let client = reqwest::Client::new();

    for attempt in 0..=MAX_RETRIES {
        let result = client.get(&storage_endpoint).send().await;

        match result {
            Ok(response) => {
                let status = response.status();

                // Retry on 502 (storage overloaded/restarting)
                if status == reqwest::StatusCode::BAD_GATEWAY && attempt < MAX_RETRIES {
                    let delay = RETRY_BASE_DELAY_MS * 2u64.pow(attempt);
                    warn!(
                        attempt = attempt + 1,
                        delay_ms = delay,
                        path = %path,
                        "Storage returned 502, retrying"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    continue;
                }

                let content_type = response
                    .headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("application/octet-stream")
                    .to_string();

                let cache_control = response
                    .headers()
                    .get("cache-control")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());

                let etag = response
                    .headers()
                    .get("etag")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());

                // Forward content addressing headers from upstream storage
                let content_address = response
                    .headers()
                    .get("x-content-address")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());

                let content_slug = response
                    .headers()
                    .get("x-content-slug")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());

                match response.bytes().await {
                    Ok(body) => {
                        info!(
                            status = %status,
                            size = body.len(),
                            path = %path,
                            "Forwarded app response"
                        );

                        let mut builder = Response::builder()
                            .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK))
                            .header("Content-Type", content_type)
                            // Required for COEP: require-corp in Angular app
                            .header("Cross-Origin-Resource-Policy", "cross-origin")
                            // Required for iframes embedded in COEP pages
                            .header("Cross-Origin-Embedder-Policy", "credentialless")
                            .header("X-Cache", cache_status);

                        if let Some(cc) = cache_control {
                            builder = builder.header("Cache-Control", cc);
                        }

                        if let Some(et) = etag {
                            builder = builder.header("ETag", et);
                        }

                        if let Some(ref addr) = content_address {
                            builder = builder.header("X-Content-Address", addr.as_str());
                        }

                        if let Some(ref slug) = content_slug {
                            builder = builder.header("X-Content-Slug", slug.as_str());
                        }

                        return builder.body(Full::new(Bytes::from(body.to_vec()))).unwrap();
                    }
                    Err(e) if attempt < MAX_RETRIES => {
                        let delay = RETRY_BASE_DELAY_MS * 2u64.pow(attempt);
                        warn!(
                            attempt = attempt + 1,
                            delay_ms = delay,
                            error = %e,
                            "Failed to read storage body, retrying"
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                        continue;
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to read storage response body");
                        return Response::builder()
                            .status(StatusCode::BAD_GATEWAY)
                            .header("Content-Type", "application/json")
                            .header("X-Cache", cache_status)
                            .body(Full::new(Bytes::from(format!(
                                r#"{{"error": "Failed to read storage response: {e}"}}"#
                            ))))
                            .unwrap();
                    }
                }
            }
            Err(e) if attempt < MAX_RETRIES => {
                let delay = RETRY_BASE_DELAY_MS * 2u64.pow(attempt);
                warn!(
                    attempt = attempt + 1,
                    delay_ms = delay,
                    error = %e,
                    "Failed to connect to storage, retrying"
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                continue;
            }
            Err(e) => {
                warn!(error = %e, url = %storage_endpoint, "Failed to forward to storage");
                return Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .header("Content-Type", "application/json")
                    .header("X-Cache", cache_status)
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "Failed to connect to storage: {e}"}}"#
                    ))))
                    .unwrap();
            }
        }
    }

    // Unreachable, but satisfy the compiler
    Response::builder()
        .status(StatusCode::BAD_GATEWAY)
        .header("Content-Type", "application/json")
        .header("X-Cache", cache_status)
        .body(Full::new(Bytes::from(
            r#"{"error": "Max retries exhausted"}"#,
        )))
        .unwrap()
}

/// Parse `/apps/{slug}/_capability` into the slug.
///
/// Returns `None` if the path doesn't match the expected pattern.
fn parse_capability_path(path: &str) -> Option<&str> {
    let slug = path
        .strip_prefix("/apps/")
        .and_then(|s| s.strip_suffix("/_capability"))?;
    if slug.is_empty() || slug.contains('/') {
        return None;
    }
    Some(slug)
}

/// Handle delivery capability probe for an HTML5 app.
///
/// Route: HEAD /apps/{slug}/_capability
///
/// Returns an empty body with headers describing the delivery readiness
/// of this doorway for the given app. If doorway has a projection cache,
/// Doorway lives on the same node as elohim-storage. Storage owns compute
/// reporting — doorway augments it with its projection cache info.
///
/// Flow: proxy to storage for the node's real capability, then overlay
/// doorway's projection cache status. One node, one capability report.
pub async fn handle_app_capability(state: Arc<AppState>, path: &str) -> Response<Full<Bytes>> {
    let slug = match parse_capability_path(path) {
        Some(s) => s,
        None => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Length", "0")
                .body(Full::new(Bytes::new()))
                .unwrap();
        }
    };

    let storage_url = match &state.args.storage_url {
        Some(url) => url,
        None => {
            return Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header("Content-Length", "0")
                .body(Full::new(Bytes::new()))
                .unwrap();
        }
    };

    // Step 1: Get the node's capability from storage (same node)
    let endpoint = format!("{}{}", storage_url.trim_end_matches('/'), path);
    let storage_resp = reqwest::Client::new().head(&endpoint).send().await;

    // Storage's reach verdict passes through: a bundle storage refuses to
    // describe is not described here either, from the projection or otherwise.
    if matches!(&storage_resp, Ok(resp) if resp.status() == reqwest::StatusCode::FORBIDDEN) {
        return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .header("Content-Length", "0")
            .body(Full::new(Bytes::new()))
            .unwrap();
    }

    // One value per header name. Storage's capability headers are forwarded, then the
    // doorway's own values REPLACE them: an appended second X-Blob-Hash or
    // X-Delivery-Mode reaches a browser as one comma-joined value ("h, h"), which
    // names no blob and no mode.
    let mut headers = HeaderMap::new();

    // Forward storage's capability headers (extraction cache state, blob hash, etc.)
    if let Ok(resp) = &storage_resp {
        for (name, value) in resp.headers() {
            if name.as_str().starts_with("x-") {
                if let (Ok(n), Ok(v)) = (
                    HeaderName::from_bytes(name.as_str().as_bytes()),
                    HeaderValue::from_bytes(value.as_bytes()),
                ) {
                    headers.insert(n, v);
                }
            }
        }
    }

    // Step 2: Augment with doorway's projection cache info.
    // If doorway has projection cache, it UPGRADES the delivery mode —
    // the client gets "extracted" from MongoDB even if storage's extraction
    // cache is cold, because doorway's projection cache sits in front.
    if let Some(ref cache) = state.app_file_cache {
        let blob_hash = cache.resolve_blob_hash(slug).await;
        let projection_ready = blob_hash.is_some();

        // Override: doorway's projection cache is the outermost layer
        set_header(&mut headers, "x-delivery-mode", "extracted");
        set_header(&mut headers, "x-cache-tier", "projection");
        set_header(
            &mut headers,
            "x-projection-ready",
            if projection_ready { "true" } else { "false" },
        );

        // Also include storage's extraction cache readiness (from forwarded headers)
        // so the client sees the full node picture. Storage's X-Ready becomes
        // X-Extraction-Ready; doorway adds X-Projection-Ready.
        if let Some(hash) = &blob_hash {
            set_header(&mut headers, "x-blob-hash", hash);
        }
    } else {
        // No doorway projection-cache layer. Storage still serves individual app
        // files on demand via GET /apps/{slug}/{file} (extracting lazily on first
        // access), so the RELIABLE delivery is "extracted" (per-file fetch), NOT
        // the compressed-zip SW fallback whose async extraction races the first
        // request → white screen (observed on elohim-host-landing). Prefer
        // extracted whenever storage is reachable; only report degraded when it
        // is not. (Warm projection-cache fast-path is Sprint 2 — this restores
        // reliable, if not yet blazing, delivery.)
        match &storage_resp {
            Ok(_) => {
                set_header(&mut headers, "x-delivery-mode", "extracted");
                set_header(&mut headers, "x-cache-tier", "storage-ondemand");
            }
            Err(_) => {
                return Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .header("Content-Length", "0")
                    .body(Full::new(Bytes::new()))
                    .unwrap();
            }
        }
    }

    let mut builder = Response::builder().status(StatusCode::OK);
    for (name, value) in &headers {
        builder = builder.header(name, value);
    }
    builder
        .header("Content-Length", "0")
        .body(Full::new(Bytes::new()))
        .unwrap()
}

/// Set a header, replacing any value already present under that name.
fn set_header(headers: &mut HeaderMap, name: &'static str, value: &str) {
    if let Ok(v) = HeaderValue::from_str(value) {
        headers.insert(HeaderName::from_static(name), v);
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_cache_path_asks_storage_for_reach_and_refuses_what_storage_refuses() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let storage = MockServer::start().await;
        Mock::given(method("HEAD"))
            .and(path("/apps/household/_capability"))
            .respond_with(ResponseTemplate::new(403))
            .expect(1)
            .mount(&storage)
            .await;
        Mock::given(method("HEAD"))
            .and(path("/apps/open/_capability"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&storage)
            .await;
        let url = storage.uri();
        let refused = cached_app_reach_refusal(&url, "household").await.unwrap();
        assert_eq!(refused.status(), StatusCode::FORBIDDEN);
        // The verdict is reused inside its window: one probe, not one per asset.
        assert!(cached_app_reach_refusal(&url, "household").await.is_some());
        assert!(cached_app_reach_refusal(&url, "open").await.is_none());
    }

    #[tokio::test]
    async fn a_capability_storage_refuses_is_refused_without_a_blob_hash() {
        use clap::Parser;
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let storage = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&storage)
            .await;
        let mut args = crate::config::Args::parse_from(["doorway", "--listen", "127.0.0.1:0"]);
        args.storage_url = Some(storage.uri());
        let state = Arc::new(AppState::new(args));
        let resp = handle_app_capability(state, "/apps/household/_capability").await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        assert!(resp.headers().get("x-blob-hash").is_none());
    }

    #[test]
    fn test_parse_app_path_normal() {
        let (slug, file_path) = parse_app_path("/apps/my-app/index.html").unwrap();
        assert_eq!(slug, "my-app");
        assert_eq!(file_path, "index.html");
    }

    #[test]
    fn test_parse_app_path_nested() {
        let (slug, file_path) = parse_app_path("/apps/quiz-123/js/vendor/pixi.min.js").unwrap();
        assert_eq!(slug, "quiz-123");
        assert_eq!(file_path, "js/vendor/pixi.min.js");
    }

    #[test]
    fn test_parse_app_path_no_file() {
        assert!(parse_app_path("/apps/my-app").is_none());
    }

    #[test]
    fn test_parse_app_path_no_slug() {
        assert!(parse_app_path("/apps/").is_none());
    }

    #[test]
    fn test_parse_app_path_directory_serves_index() {
        let (slug, file_path) = parse_app_path("/apps/my-app/").unwrap();
        assert_eq!(slug, "my-app");
        assert_eq!(file_path, "index.html");
    }

    #[test]
    fn test_parse_app_path_not_apps() {
        assert!(parse_app_path("/other/my-app/file.js").is_none());
    }

    #[test]
    fn test_parse_app_path_content_address() {
        let (slug, file_path) =
            parse_app_path("/apps/sha256-abc123def456789012/index.html").unwrap();
        assert_eq!(slug, "sha256-abc123def456789012");
        assert_eq!(file_path, "index.html");
    }

    #[test]
    fn cache_miss_stays_pinned_to_the_resolved_release() {
        let path = resolved_app_path("sha256-release-a", "assets/chunks/main-release-a.js");

        assert_eq!(
            path,
            "/apps/sha256-release-a/assets/chunks/main-release-a.js"
        );
        assert!(!path.contains("elohim-host-landing"));
    }

    // =========================================================================
    // is_content_address tests
    // =========================================================================

    #[test]
    fn test_is_content_address_valid() {
        assert!(is_content_address("sha256-abc123def456"));
    }

    #[test]
    fn test_is_content_address_too_short() {
        // "sha256-" is 7 chars, need >10 total
        assert!(!is_content_address("sha256-abc"));
    }

    #[test]
    fn test_is_content_address_slug() {
        assert!(!is_content_address("my-cool-app"));
    }

    #[test]
    fn test_is_content_address_empty() {
        assert!(!is_content_address(""));
    }

    // Path traversal detection (defense in depth — validated in handle_app_request)
    #[test]
    fn test_parse_app_path_traversal_passes_parse_but_caught_later() {
        // parse_app_path doesn't reject traversal — that's handle_app_request's job
        let result = parse_app_path("/apps/my-app/../../../etc/passwd");
        assert!(result.is_some()); // parser succeeds
        let (slug, file_path) = result.unwrap();
        assert_eq!(slug, "my-app");
        assert!(file_path.contains("..")); // but file_path contains traversal
    }

    #[test]
    fn test_build_app_response_headers() {
        let resp = build_app_response(b"hello", "text/html", "HIT", None, None);
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers().get("Content-Type").unwrap(), "text/html");
        assert_eq!(resp.headers().get("X-Cache").unwrap(), "HIT");
        assert_eq!(
            resp.headers().get("Cross-Origin-Resource-Policy").unwrap(),
            "cross-origin"
        );
        assert_eq!(
            resp.headers().get("Cross-Origin-Embedder-Policy").unwrap(),
            "credentialless"
        );
        assert_eq!(
            resp.headers().get("Cache-Control").unwrap(),
            "public, max-age=3600"
        );
        // No content addressing headers when None
        assert!(resp.headers().get("X-Content-Address").is_none());
        assert!(resp.headers().get("X-Content-Slug").is_none());
    }

    #[test]
    fn test_build_app_response_with_content_address_headers() {
        let resp = build_app_response(
            b"data",
            "text/html",
            "MISS",
            Some("sha256-abc123def456"),
            Some("my-app"),
        );
        assert_eq!(
            resp.headers().get("X-Content-Address").unwrap(),
            "sha256-abc123def456"
        );
        assert_eq!(resp.headers().get("X-Content-Slug").unwrap(), "my-app");
    }

    #[test]
    fn test_build_app_response_with_address_only() {
        let resp = build_app_response(
            b"data",
            "text/html",
            "HIT",
            Some("sha256-abc123def456"),
            None,
        );
        assert_eq!(
            resp.headers().get("X-Content-Address").unwrap(),
            "sha256-abc123def456"
        );
        assert!(resp.headers().get("X-Content-Slug").is_none());
    }

    #[test]
    fn test_build_app_response_bypass() {
        let resp = build_app_response(b"data", "application/javascript", "BYPASS", None, None);
        assert_eq!(resp.headers().get("X-Cache").unwrap(), "BYPASS");
    }

    #[test]
    fn test_build_app_response_coalesced() {
        let resp = build_app_response(b"data", "text/css", "HIT-COALESCED", None, None);
        assert_eq!(resp.headers().get("X-Cache").unwrap(), "HIT-COALESCED");
    }

    // =========================================================================
    // Capability path parsing tests
    // =========================================================================

    #[test]
    fn capability_override_replaces_the_forwarded_value() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("x-delivery-mode"),
            HeaderValue::from_static("compressed"),
        );
        headers.insert(
            HeaderName::from_static("x-blob-hash"),
            HeaderValue::from_static("bafkreiabc"),
        );
        set_header(&mut headers, "x-delivery-mode", "extracted");
        set_header(&mut headers, "x-blob-hash", "bafkreiabc");
        let modes: Vec<_> = headers.get_all("x-delivery-mode").iter().collect();
        let hashes: Vec<_> = headers.get_all("x-blob-hash").iter().collect();
        assert_eq!(modes, vec!["extracted"]);
        assert_eq!(hashes, vec!["bafkreiabc"]);
    }

    #[test]
    fn test_parse_capability_path_normal() {
        let slug = parse_capability_path("/apps/my-app/_capability").unwrap();
        assert_eq!(slug, "my-app");
    }

    #[test]
    fn test_parse_capability_path_with_hash() {
        let slug = parse_capability_path("/apps/bafkrei-abc123/_capability").unwrap();
        assert_eq!(slug, "bafkrei-abc123");
    }

    #[test]
    fn test_parse_capability_path_empty_slug() {
        assert!(parse_capability_path("/apps//_capability").is_none());
    }

    #[test]
    fn test_parse_capability_path_no_capability_suffix() {
        assert!(parse_capability_path("/apps/my-app/index.html").is_none());
    }

    #[test]
    fn test_parse_capability_path_nested_slashes() {
        // slug should not contain slashes
        assert!(parse_capability_path("/apps/my/nested/app/_capability").is_none());
    }

    #[test]
    fn test_parse_capability_path_wrong_prefix() {
        assert!(parse_capability_path("/other/my-app/_capability").is_none());
    }

    #[test]
    fn test_parse_capability_path_only_prefix() {
        assert!(parse_capability_path("/apps/_capability").is_none());
    }

    // =========================================================================
    // Content-app chrome shell
    // =========================================================================

    fn nav_headers() -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert("sec-fetch-dest", HeaderValue::from_static("document"));
        h.insert("sec-fetch-mode", HeaderValue::from_static("navigate"));
        h
    }

    fn frame_headers() -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert("sec-fetch-dest", HeaderValue::from_static("iframe"));
        h.insert("sec-fetch-mode", HeaderValue::from_static("navigate"));
        h
    }

    fn descriptor(format: &str, title: &str) -> AppDescriptor {
        AppDescriptor {
            blob_hash: "sha256-abcdef123456".into(),
            content_id: "quiz-1".into(),
            content_format: format.into(),
            entry_point: None,
            title: Some(title.into()),
        }
    }

    async fn state_for(storage: &str) -> Arc<AppState> {
        use clap::Parser;
        let mut args = crate::config::Args::parse_from(["doorway", "--listen", "127.0.0.1:0"]);
        args.storage_url = Some(storage.to_string());
        Arc::new(AppState::new(args))
    }

    /// A storage that is open to anonymous readers and serves one raw entry page.
    async fn storage_with_entry() -> wiremock::MockServer {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let storage = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&storage)
            .await;
        Mock::given(method("GET"))
            .and(path("/apps/quiz-1/index.html"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/html")
                    .set_body_bytes(b"<h1>raw bundle</h1>".to_vec()),
            )
            .mount(&storage)
            .await;
        storage
    }

    async fn serve(
        state: &Arc<AppState>,
        storage: &wiremock::MockServer,
        file: &str,
        headers: &HeaderMap,
        desc: Option<&AppDescriptor>,
    ) -> Response<Full<Bytes>> {
        serve_app_file(
            state,
            &storage.uri(),
            &format!("/apps/quiz-1/{file}"),
            "quiz-1",
            file,
            headers,
            desc,
            false,
        )
        .await
    }

    async fn body_of(resp: Response<Full<Bytes>>) -> String {
        use http_body_util::BodyExt;
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn a_page_navigation_to_an_html5_entry_gets_the_shell() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};
        let storage = storage_with_entry().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/resilience/quiz-1/household"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "contentId": "quiz-1",
                "feltStatus": { "floor": { "hasHouseholds": 2, "wantsHouseholds": 3 } }
            })))
            .mount(&storage)
            .await;
        let state = state_for(&storage.uri()).await;
        let desc = descriptor("html5-app", "A quiz");
        let resp = serve(&state, &storage, "index.html", &nav_headers(), Some(&desc)).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let h = resp.headers();
        assert_eq!(h.get("content-type").unwrap(), "text/html; charset=utf-8");
        assert_eq!(h.get("cache-control").unwrap(), "no-store");
        assert_eq!(h.get("vary").unwrap(), "Sec-Fetch-Dest, Sec-Fetch-Mode");
        assert_eq!(h.get("x-elohim-chrome").unwrap(), "content-shell");
        assert!(h.get("etag").is_none());
        assert!(h.get("x-content-address").is_none());
        assert!(h.get("x-cache").is_none());
        let body = body_of(resp).await;
        assert!(body
            .starts_with("<!DOCTYPE html><html lang=\"en\" data-elohim-chrome=\"content-shell\">"));
        assert!(body.contains("data-testid=\"content-chrome\""));
        assert!(body.contains(
            "<a data-testid=\"resiliency-badge\" class=\"resiliency-badge\" href=\"/epr/quiz-1\" data-holders=\"2\" data-target=\"3\">Held by 2 of 3 households</a>"
        ));
        assert!(body.contains(
            "data-testid=\"content-app-frame\" title=\"A quiz\" src=\"/apps/quiz-1/index.html\""
        ));
        assert!(body.contains("id=\"elohim-omni-context\""));
        assert!(body.contains("/chrome/omni-element."));
        // The shell never asked storage for the bundle bytes.
        let gets: Vec<_> = storage
            .received_requests()
            .await
            .unwrap()
            .into_iter()
            .filter(|r| r.url.path().starts_with("/apps/"))
            .filter(|r| r.method.as_str() == "GET")
            .collect();
        assert!(
            gets.is_empty(),
            "shell must not fetch or stock bundle bytes"
        );
        eprintln!("SHELL:\n{body}");
    }

    #[tokio::test]
    async fn a_frame_load_gets_the_same_bytes_as_no_headers_plus_vary() {
        let storage = storage_with_entry().await;
        let state = state_for(&storage.uri()).await;
        let desc = descriptor("html5-app", "A quiz");
        let plain = serve(
            &state,
            &storage,
            "index.html",
            &HeaderMap::new(),
            Some(&desc),
        )
        .await;
        let framed = serve(
            &state,
            &storage,
            "index.html",
            &frame_headers(),
            Some(&desc),
        )
        .await;
        for r in [&plain, &framed] {
            assert_eq!(
                r.headers().get("vary").unwrap(),
                "Sec-Fetch-Dest, Sec-Fetch-Mode"
            );
            assert!(r.headers().get("x-elohim-chrome").is_none());
        }
        let (a, b) = (body_of(plain).await, body_of(framed).await);
        assert_eq!(a, "<h1>raw bundle</h1>");
        assert_eq!(a, b);
    }

    #[tokio::test]
    async fn non_entry_other_format_and_missing_descriptor_are_served_raw() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};
        let storage = storage_with_entry().await;
        Mock::given(method("GET"))
            .and(path("/apps/quiz-1/js/app.js"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"js".to_vec()))
            .mount(&storage)
            .await;
        let state = state_for(&storage.uri()).await;
        let html5 = descriptor("html5-app", "A quiz");
        let spa = descriptor("spa-bundle", "A quiz");

        let sub = serve(&state, &storage, "js/app.js", &nav_headers(), Some(&html5)).await;
        assert!(sub.headers().get("x-elohim-chrome").is_none());
        assert!(sub.headers().get("vary").is_none());
        assert_eq!(body_of(sub).await, "js");

        let other = serve(&state, &storage, "index.html", &nav_headers(), Some(&spa)).await;
        assert!(other.headers().get("x-elohim-chrome").is_none());
        assert!(other.headers().get("vary").is_none());
        assert_eq!(body_of(other).await, "<h1>raw bundle</h1>");

        let none = serve(&state, &storage, "index.html", &nav_headers(), None).await;
        assert!(none.headers().get("x-elohim-chrome").is_none());
        assert_eq!(body_of(none).await, "<h1>raw bundle</h1>");

        // Through the public entry with no projection cache: no descriptor, raw.
        let via =
            handle_app_request(state.clone(), "/apps/quiz-1/index.html", &nav_headers()).await;
        assert!(via.headers().get("x-elohim-chrome").is_none());
        assert_eq!(body_of(via).await, "<h1>raw bundle</h1>");
    }

    #[tokio::test]
    async fn a_declared_entry_point_is_the_chrome_target_not_index_html() {
        let storage = storage_with_entry().await;
        let state = state_for(&storage.uri()).await;
        let mut desc = descriptor("html5-app", "A quiz");
        desc.entry_point = Some("game.html".into());
        let index = serve(&state, &storage, "index.html", &nav_headers(), Some(&desc)).await;
        assert!(index.headers().get("x-elohim-chrome").is_none());
        let game = serve(&state, &storage, "game.html", &nav_headers(), Some(&desc)).await;
        assert_eq!(
            game.headers().get("x-elohim-chrome").unwrap(),
            "content-shell"
        );
        assert!(body_of(game)
            .await
            .contains("src=\"/apps/quiz-1/game.html\""));
    }

    #[tokio::test]
    async fn a_reach_refusal_comes_before_any_shell_output() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let storage = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&storage)
            .await;
        let state = state_for(&storage.uri()).await;
        let desc = descriptor("html5-app", "Private");
        let resp = serve_app_file(
            &state,
            &storage.uri(),
            "/apps/refused-app/index.html",
            "refused-app",
            "index.html",
            &nav_headers(),
            Some(&desc),
            false,
        )
        .await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        assert!(resp.headers().get("x-elohim-chrome").is_none());
    }

    #[tokio::test]
    async fn the_admin_cache_bypass_does_not_bypass_chrome_selection() {
        let storage = storage_with_entry().await;
        let state = state_for(&storage.uri()).await;
        state
            .cache_enabled
            .store(false, std::sync::atomic::Ordering::Relaxed);
        let desc = descriptor("html5-app", "A quiz");
        let resp = serve_app_file(
            &state,
            &storage.uri(),
            "/apps/quiz-1/index.html",
            "quiz-1",
            "index.html",
            &nav_headers(),
            Some(&desc),
            true,
        )
        .await;
        assert_eq!(
            resp.headers().get("x-elohim-chrome").unwrap(),
            "content-shell"
        );
    }

    #[tokio::test]
    async fn a_failing_holders_fetch_still_yields_a_shell_with_the_unknown_badge() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};
        let storage = storage_with_entry().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/resilience/quiz-1/household"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&storage)
            .await;
        let state = state_for(&storage.uri()).await;
        let desc = descriptor("html5-app", "A quiz");
        let resp = serve(&state, &storage, "index.html", &nav_headers(), Some(&desc)).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_of(resp).await;
        assert!(body.contains(
            "<a data-testid=\"resiliency-badge\" class=\"resiliency-badge\" href=\"/epr/quiz-1\">Held by households \u{b7} see where it lives</a>"
        ));
        assert!(!body.contains("data-holders"));
    }

    #[test]
    fn a_hostile_title_is_escaped_everywhere_it_lands() {
        let desc = descriptor("html5-app", "</title><script>alert(1)</script>\"'&");
        let html = render_chrome_shell("quiz-1", &desc, None);
        // Only the omnibar's two real script elements close; the island's copy
        // of the title is JSON data with `</` neutralised.
        assert_eq!(html.matches("</script>").count(), 2);
        assert!(html.contains(
            "<title>&lt;/title&gt;&lt;script&gt;alert(1)&lt;/script&gt;&quot;&#39;&amp;</title>"
        ));
        assert!(html.contains("title=\"&lt;/title&gt;"));
    }

    #[test]
    fn fetch_metadata_must_say_document_and_navigate() {
        assert!(is_document_navigation(&nav_headers()));
        assert!(!is_document_navigation(&frame_headers()));
        assert!(!is_document_navigation(&HeaderMap::new()));
        let mut only_dest = HeaderMap::new();
        only_dest.insert("sec-fetch-dest", HeaderValue::from_static("document"));
        assert!(!is_document_navigation(&only_dest));
    }
}
