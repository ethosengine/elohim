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

use super::csp_report::{csp_report_url, reporting_endpoints, CSP_REPORT_PATH};
use crate::cache::AppDescriptor;
use crate::server::http::verify_session_token;
use crate::server::AppState;

/// Check whether an identifier looks like a content address (CID) rather than a slug.
///
/// Content addresses use the `sha256-{hex}` format from the storage layer.
/// The length check (>10) avoids false positives on short strings.
pub(crate) fn is_content_address(identifier: &str) -> bool {
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
/// before.
///
/// When the request's public name has an apps origin (`ctx.edge`), the rule
/// splits by side: on the NAME only a page opening of an html5-app entry is
/// answered (with the shell) and everything else moves to `apps.<name>`; on
/// the APPS origin an html5-app entry the person would otherwise see bare is
/// redirected to the shell, and raw HTML may only be framed by the name.
pub async fn handle_app_request(
    state: Arc<AppState>,
    path: &str,
    headers: &HeaderMap,
    ctx: &AppServeContext,
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

    // Parse /apps/{slug}/{file_path...}. A malformed path is refused for every
    // caller — the admin cache bypass changes where bytes come from, never
    // which paths may be asked for.
    let (slug, file_path) = match parse_app_path(path) {
        Some(parsed) => parsed,
        None => {
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

    // Path traversal protection — defense in depth (storage also validates),
    // on the slug as well as the file, percent-encoded forms included.
    if !safe_app_segments(slug, file_path) {
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
        resolve_descriptor(&state, slug).await
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
        ctx,
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
    ctx: &AppServeContext,
) -> Response<Full<Bytes>> {
    let entry_page = descriptor
        .filter(|d| is_html5_entry(d, file_path))
        .filter(|d| safe_entry_path(d.entry_file()));

    if let Some(edge) = ctx.edge.as_ref() {
        return match edge.pairing.side {
            PairingSide::Apps => {
                // No HTML is shown bare at the top of the content origin —
                // projected entry, non-entry page, content address or no
                // record at all: the person goes to the name's
                // always-render shell, which never redirects back.
                if looks_like_html(file_path) && apps_host_wants_shell(headers) {
                    // A tab opening a projected entry goes to its canonical address on the
                    // name, which renders the shell for a page navigation: the person lands
                    // where they would share it from. Frames, clients without Fetch Metadata
                    // and non-entry pages go to the always-render path, which cannot loop.
                    if entry_page.is_some() && is_top_level_navigation(headers) {
                        return redirect_to_canonical_entry(
                            edge,
                            slug,
                            file_path,
                            ctx.query.as_deref(),
                        );
                    }
                    return redirect_to_shell(edge, slug, file_path, ctx.query.as_deref());
                }
                let mut response =
                    raw_app_bytes(state, storage_url, path, slug, file_path, cache_bypassed).await;
                if let Err(refused) =
                    police_raw_html(state, edge, slug, descriptor, &mut response).await
                {
                    return refused;
                }
                response
                    .headers_mut()
                    .insert(hyper::header::VARY, HeaderValue::from_static(APPS_VARY));
                response
            }
            PairingSide::Name => {
                if let Some(desc) = entry_page {
                    if is_document_navigation(headers) {
                        if let Some(refused) = cached_app_reach_refusal(storage_url, slug).await {
                            return refused;
                        }
                        return content_shell_response(state, slug, file_path, Some(desc), ctx)
                            .await;
                    }
                }
                moved_to_apps_origin(edge, path, ctx.query.as_deref())
            }
        };
    }

    if let Some(desc) = entry_page {
        if is_document_navigation(headers) {
            // Reach before any shell output: the shell names the content
            // (title, address), so a refused app gets the refusal.
            if let Some(refused) = cached_app_reach_refusal(storage_url, slug).await {
                return refused;
            }
            return content_shell_response(state, slug, file_path, Some(desc), ctx).await;
        }
    }

    let mut response =
        raw_app_bytes(state, storage_url, path, slug, file_path, cache_bypassed).await;

    // The same URL can answer with two representations; shared caches must
    // keep them apart even when the projection (and so the descriptor) is
    // unavailable right now and a later request would be answered otherwise.
    response
        .headers_mut()
        .insert(hyper::header::VARY, HeaderValue::from_static(CHROME_VARY));
    response
}

/// The bundle bytes for a request: through the projection cache, or straight
/// from storage while the admin cache bypass is on.
async fn raw_app_bytes(
    state: &Arc<AppState>,
    storage_url: &str,
    path: &str,
    slug: &str,
    file_path: &str,
    cache_bypassed: bool,
) -> Response<Full<Bytes>> {
    if cache_bypassed {
        forward_app_request_with_header(storage_url, path, "BYPASS-ADMIN").await
    } else {
        serve_raw_app_file(state, storage_url, slug, file_path).await
    }
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

/// `Vary` carried by every app response (bytes and shell alike): the
/// same URL answers differently for a page navigation and for a frame load.
const CHROME_VARY: &str = "Sec-Fetch-Dest, Sec-Fetch-Mode";

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

/// A relative entry path the shell may frame: no absolute path, no `..`/empty
/// segments, no backslashes or control characters. Anything else is served raw.
fn safe_entry_path(entry: &str) -> bool {
    !entry.is_empty()
        && !entry.starts_with('/')
        && !entry.contains('\\')
        && !entry.chars().any(char::is_control)
        && entry.split('/').all(|seg| !seg.is_empty() && seg != "..")
}

/// Percent-encode a path segment (unreserved characters pass through).
pub(crate) fn encode_segment(raw: &str) -> String {
    raw.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            b => format!("%{b:02X}"),
        })
        .collect()
}

/// The record id the shell names: the projected content id, else the slug.
fn record_address<'a>(slug: &'a str, desc: &'a AppDescriptor) -> &'a str {
    if desc.content_id.is_empty() {
        slug
    } else {
        desc.content_id.as_str()
    }
}

/// Percent-decode once, refusing invalid UTF-8.
pub(crate) fn percent_decoded(raw: &str) -> Option<String> {
    urlencoding::decode(raw).ok().map(|c| c.into_owned())
}

/// The slug and file of an `/apps/` request are safe to look up and forward,
/// judged AFTER percent-decoding (the form a URL normaliser and storage see),
/// so `%2e%2e` is `..`: no parent segment, no backslash or control character,
/// a slug that is one segment, a file that is relative.
fn safe_app_segments(slug: &str, file_path: &str) -> bool {
    let (Some(slug), Some(file)) = (percent_decoded(slug), percent_decoded(file_path)) else {
        return false;
    };
    let clean =
        |s: &str| !s.contains("..") && !s.contains('\\') && !s.chars().any(char::is_control);
    !slug.is_empty()
        && slug != "."
        && !slug.contains('/')
        && clean(&slug)
        && !file.starts_with('/')
        && clean(&file)
}

/// The projection descriptor for an `/apps/` identifier. A content address
/// resolves (index-only) to the record whose current head it is, pinned to
/// exactly that address; an address no record names has no descriptor.
async fn resolve_descriptor(state: &AppState, identifier: &str) -> Option<AppDescriptor> {
    let cache = state.app_file_cache.as_ref()?;
    if is_content_address(identifier) {
        let slug = cache.slug_for_head(identifier).await?;
        let mut desc = cache.resolve_app_descriptor(&slug).await?;
        desc.blob_hash = identifier.to_string();
        Some(desc)
    } else {
        cache.resolve_app_descriptor(identifier).await
    }
}

/// What the record says, as the content chrome shows it.
#[derive(Debug, Clone, PartialEq)]
struct ChromeReadings {
    record_id: String,
    content_address: String,
    format: String,
    entry: String,
    title: String,
    mode: elohim_chrome_asset::TrustMode,
    reach: Option<String>,
    witness: elohim_chrome_asset::WitnessState,
    capabilities: Vec<String>,
    /// External origins the record declares (the raw content policy's allow list).
    sources: Vec<String>,
    holders: Option<elohim_chrome_asset::HolderReading>,
    theme: Option<elohim_chrome_asset::ChromeTheme>,
}

/// The reach values the context island may carry
/// (`elohim/sdk/schemas/v1/enums/reach.schema.json`, which the island's
/// `content.reach` references).
const ISLAND_REACH_VALUES: &[&str] = &[
    "private",
    "self",
    "intimate",
    "trusted",
    "familiar",
    "community",
    "public",
    "commons",
];

/// The storage route the content-chrome facing is served on.
const CHROME_FACING_ROUTE: &str = "/api/v1/epr/{id}/chrome";

/// How long the shell waits for the facing — inside the chip's budget. A
/// slower facing is read from the cache on the next open; this open falls back
/// to the doorway's own projection descriptor.
const CHROME_FACING_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(300);

/// How long a failed facing read is remembered, so a down peer is not asked
/// on every shell.
const CHROME_FACING_MISS_TTL: std::time::Duration = std::time::Duration::from_secs(5);

/// The ONE storage peer a manifest-declared route dispatches to — the
/// registry's declared-primary match (the same first-match rule the storage
/// proxy uses for non-blob routes), else `STORAGE_URL` — with the route's
/// declared cache TTL. Never a fan-out.
pub(crate) async fn storage_target(
    state: &AppState,
    method: doorway_client::HttpMethod,
    path: &str,
) -> Option<(String, u64)> {
    let matches = state.route_registry.match_request(method, path).await;
    if let Some(endpoint) = matches.first().and_then(|r| r.storage_endpoint()) {
        let ttl = matches.first().map_or(0, |r| r.cache_ttl_secs);
        return Some((endpoint.trim_end_matches('/').to_string(), ttl));
    }
    state
        .args
        .storage_url
        .as_deref()
        .map(|u| (u.trim_end_matches('/').to_string(), 0))
}

/// The content-chrome facing as this doorway reads it off the wire —
/// storage's `ContentChromeFacingView` (`crates/elohim-views`), whose lenses
/// are ABSENT when not selected. Only the readings the island carries are
/// read; everything else stays the element's lazy reads.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChromeFacing {
    record_id: String,
    title: String,
    format: String,
    mode: String,
    reach: String,
    witness: String,
    #[serde(default)]
    content_address: Option<String>,
    #[serde(default)]
    app: Option<FacingApp>,
    #[serde(default)]
    holders: Option<FacingHolders>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FacingApp {
    entry: String,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    theme_hint: Option<FacingThemeHint>,
    /// External origins the record declares its content may reach (the raw
    /// content policy allows them beside its own origin).
    #[serde(default)]
    sources: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FacingThemeHint {
    #[serde(default)]
    color_scheme: Option<String>,
    #[serde(default)]
    theme_color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FacingHolders {
    holders: u32,
    target: u32,
    headline: String,
    #[serde(default)]
    as_of: Option<String>,
}

/// What one facing read said. A refusal is its own answer, never "unknown":
/// only `Unavailable` lets the doorway fall back to its own descriptor.
#[derive(Debug, Clone, PartialEq)]
enum FacingRead {
    /// The record's facing.
    Found(Box<ChromeFacing>),
    /// The peer refused (401/403): the content is not for this viewer.
    Forbidden,
    /// The peer has no such record (404/410).
    NoRecord,
    /// Timeout, network failure, 5xx/429, an oversized or unparseable body —
    /// the only outcome that falls back.
    Unavailable,
}

/// Largest facing body the doorway reads; a larger one is abandoned mid-read.
const CHROME_FACING_MAX_BYTES: usize = 256 * 1024;

/// Most facing reads the edge cache holds; expired entries go first.
const CHROME_FACING_CACHE_CAPACITY: usize = 1024;

type FacingCache =
    std::collections::HashMap<String, (std::time::Instant, std::time::Duration, FacingRead)>;

/// The doorway's edge cache of facing reads, keyed `<storage>|<id>`, each held
/// for the route's declared `cache_ttl` (non-records for
/// [`CHROME_FACING_MISS_TTL`]), bounded by [`CHROME_FACING_CACHE_CAPACITY`].
static CHROME_FACING_CACHE: std::sync::LazyLock<std::sync::Mutex<FacingCache>> =
    std::sync::LazyLock::new(Default::default);

/// Insert into a bounded cache: expired entries are evicted first; when it is
/// still full, the oldest entry makes room.
fn cache_insert_bounded(
    cache: &mut FacingCache,
    key: String,
    entry: (std::time::Instant, std::time::Duration, FacingRead),
    capacity: usize,
) {
    if !cache.contains_key(&key) && cache.len() >= capacity {
        cache.retain(|_, (at, keep, _)| at.elapsed() < *keep);
        while cache.len() >= capacity {
            let Some(oldest) = cache
                .iter()
                .min_by_key(|(_, (at, _, _))| *at)
                .map(|(k, _)| k.clone())
            else {
                break;
            };
            cache.remove(&oldest);
        }
    }
    cache.insert(key, entry);
}

/// Read a response body incrementally, abandoning it past `max` bytes.
async fn read_capped(mut resp: reqwest::Response, max: usize) -> Option<Vec<u8>> {
    let mut body = Vec::new();
    while let Some(chunk) = resp.chunk().await.ok()? {
        if body.len() + chunk.len() > max {
            return None;
        }
        body.extend_from_slice(&chunk);
    }
    Some(body)
}

/// Read the content-chrome facing for `id` (a record id or content address)
/// through the registry route, from the edge cache when fresh. The whole read
/// — connect, headers and a body capped at [`CHROME_FACING_MAX_BYTES`] — sits
/// inside the chip budget.
async fn fetch_chrome_facing(state: &AppState, id: &str) -> FacingRead {
    let path = CHROME_FACING_ROUTE.replace("{id}", &encode_segment(id));
    let Some((endpoint, ttl)) = storage_target(state, doorway_client::HttpMethod::Get, &path).await
    else {
        return FacingRead::Unavailable;
    };
    let key = format!("{endpoint}|{id}");
    if let Some(hit) = CHROME_FACING_CACHE
        .lock()
        .ok()
        .and_then(|m| m.get(&key).cloned())
        .filter(|(at, keep, _)| at.elapsed() < *keep)
    {
        return hit.2;
    }
    let read = async {
        let Ok(resp) = reqwest::Client::new()
            .get(format!("{endpoint}{path}"))
            .send()
            .await
        else {
            return FacingRead::Unavailable;
        };
        match resp.status().as_u16() {
            200..=299 => {}
            401 | 403 => return FacingRead::Forbidden,
            404 | 410 => return FacingRead::NoRecord,
            _ => return FacingRead::Unavailable,
        }
        match read_capped(resp, CHROME_FACING_MAX_BYTES).await {
            Some(body) => serde_json::from_slice::<ChromeFacing>(&body)
                .map(|f| FacingRead::Found(Box::new(f)))
                .unwrap_or(FacingRead::Unavailable),
            None => FacingRead::Unavailable,
        }
    };
    let facing = tokio::time::timeout(CHROME_FACING_TIMEOUT, read)
        .await
        .unwrap_or(FacingRead::Unavailable);
    let keep = match facing {
        FacingRead::Found(_) => std::time::Duration::from_secs(ttl),
        _ => CHROME_FACING_MISS_TTL.min(std::time::Duration::from_secs(ttl)),
    };
    if !keep.is_zero() {
        if let Ok(mut m) = CHROME_FACING_CACHE.lock() {
            cache_insert_bounded(
                &mut m,
                key,
                (std::time::Instant::now(), keep, facing.clone()),
                CHROME_FACING_CACHE_CAPACITY,
            );
        }
    }
    facing
}

/// The content address a FRESH cached facing for `id` names (from any storage
/// endpoint's entry), without fetching — `None` on a miss.
pub(crate) fn cached_facing_address(id: &str) -> Option<String> {
    let suffix = format!("|{id}");
    let cache = CHROME_FACING_CACHE.lock().ok()?;
    cache
        .iter()
        .filter(|(key, (at, keep, _))| key.ends_with(&suffix) && at.elapsed() < *keep)
        .find_map(|(_, (_, _, read))| match read {
            FacingRead::Found(f) => f.content_address.clone(),
            _ => None,
        })
}

/// The reach refusal every shell path answers with — the same shape the
/// storage reach probe's refusal takes (`cached_app_reach_refusal`).
fn reach_refused() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::FORBIDDEN)
        .header("Content-Type", "application/json")
        .header("X-Cache", "REFUSED")
        .body(Full::new(Bytes::from(
            r#"{"error":"Reach authorization required"}"#,
        )))
        .unwrap()
}

/// Map a theme hint into the island's `theme` (object form), when it names
/// anything.
fn theme_from(
    color_scheme: Option<String>,
    theme_color: Option<String>,
) -> Option<elohim_chrome_asset::ChromeTheme> {
    let hint = elohim_chrome_asset::ThemeHint {
        color_scheme,
        theme_color,
        tokens: None,
    };
    (!hint.is_empty()).then(|| hint.into())
}

/// The island's readings from the facing — taken as the facing states them,
/// never re-derived. `desc` only fills what the facing does not carry (the
/// address the doorway is serving when the facing names none).
fn readings_from_facing(
    facing: ChromeFacing,
    slug: &str,
    desc: Option<&AppDescriptor>,
) -> ChromeReadings {
    use elohim_chrome_asset::{HolderReading, TrustMode, WitnessState};
    let entry = facing
        .app
        .as_ref()
        .map(|a| a.entry.clone())
        .or_else(|| desc.map(|d| d.entry_file().to_string()))
        .unwrap_or_else(|| lamad_types::DEFAULT_ENTRY_POINT.to_string());
    let theme = facing
        .app
        .as_ref()
        .and_then(|a| a.theme_hint.clone())
        .and_then(|h| theme_from(h.color_scheme, h.theme_color));
    ChromeReadings {
        content_address: facing
            .content_address
            .or_else(|| desc.map(|d| d.blob_hash.clone()))
            .unwrap_or_else(|| slug.to_string()),
        title: facing.title,
        format: facing.format,
        entry,
        mode: if facing.mode == "trustless" {
            TrustMode::Trustless
        } else {
            TrustMode::Trustful
        },
        reach: Some(facing.reach).filter(|r| !r.is_empty()),
        witness: if facing.witness == "green" {
            WitnessState::Green
        } else {
            WitnessState::Amber
        },
        sources: facing
            .app
            .as_ref()
            .and_then(|a| a.sources.clone())
            .unwrap_or_default(),
        capabilities: facing.app.map(|a| a.capabilities).unwrap_or_default(),
        // Carried only because the facing already took the reading; the shell
        // never makes a reading of its own.
        holders: facing.holders.map(|h| HolderReading {
            holders: h.holders,
            target: h.target,
            headline: Some(h.headline).filter(|s| !s.is_empty()),
            as_of: h.as_of,
        }),
        record_id: facing.record_id,
        theme,
    }
}

/// The island's readings from the doorway's own projection descriptor — the
/// fallback when no facing answered within the chip budget. `holders` stays
/// `None` (the element's lazy read fills it). The witness reads amber: whether
/// a peer other than the author holds the record is the storage peer's reading,
/// and an anchor in the descriptor does not answer it, so the chrome re-reads
/// the facing when page info opens.
fn readings_from_descriptor(slug: &str, desc: &AppDescriptor) -> ChromeReadings {
    let record_id = record_address(slug, desc).to_string();
    let witness = elohim_chrome_asset::WitnessState::Amber;
    let theme = desc.body.theme_hint.as_ref().and_then(|hint| {
        theme_from(
            hint.color_scheme.map(|s| match s {
                lamad_types::ColorScheme::Light => "light".to_string(),
                lamad_types::ColorScheme::Dark => "dark".to_string(),
            }),
            hint.theme_color.clone(),
        )
    });
    ChromeReadings {
        title: desc.title.clone().unwrap_or_else(|| record_id.clone()),
        record_id,
        content_address: desc.blob_hash.clone(),
        format: desc.content_format.clone(),
        entry: desc.entry_file().to_string(),
        mode: elohim_chrome_asset::TrustMode::Trustful,
        reach: desc.reach.clone(),
        witness,
        capabilities: desc
            .body
            .capabilities
            .iter()
            .map(|c| c.as_str().to_string())
            .collect(),
        sources: desc.body.sources.clone(),
        holders: None,
        theme,
    }
}

/// SEAM — the content-chrome facing. The ONE function the island's record
/// readings come from: storage's `GET /api/v1/epr/{id}/chrome` (record id, or
/// the requested content address when no record is projected here), read
/// through the registry route and the doorway's edge cache within the chip
/// budget. A REFUSAL from the facing is the answer (`Forbidden` → the reach
/// refusal, never the descriptor). A facing with no such record (one peer
/// lagging) or an unavailable one (timeout, network, 5xx, oversized or
/// unreadable body) lets the doorway's own projection descriptor fill in; the
/// trustless shell is shown only when NEITHER knows the record.
async fn chrome_readings(state: &AppState, slug: &str, desc: Option<&AppDescriptor>) -> Readings {
    let id = desc.map_or(slug, |d| record_address(slug, d));
    match fetch_chrome_facing(state, id).await {
        FacingRead::Found(facing) => {
            Readings::Record(Box::new(readings_from_facing(*facing, slug, desc)))
        }
        FacingRead::Forbidden => Readings::Forbidden,
        // A peer without the record has usually not caught up: trust lags
        // delivery, so a record the doorway's own projection knows stays
        // trustful (witness per its anchor), never demoted to trustless.
        FacingRead::NoRecord | FacingRead::Unavailable => match desc {
            Some(d) => Readings::Record(Box::new(readings_from_descriptor(slug, d))),
            None => Readings::NoRecord,
        },
    }
}

/// The outcome of [`chrome_readings`].
#[derive(Debug, Clone, PartialEq)]
enum Readings {
    Record(Box<ChromeReadings>),
    NoRecord,
    Forbidden,
}

/// The raw content policy for HTML served on the content origin, from the
/// same readings the shell shows: the record's trust mode and declared
/// `sources` (the shared `elohim_chrome_asset::content_policy`). `None` when
/// the facing refused the content.
async fn raw_policy(
    state: &AppState,
    slug: &str,
    desc: Option<&AppDescriptor>,
    report_uri: &str,
) -> Option<elohim_chrome_asset::ContentPolicy> {
    let (mode, sources) = match chrome_readings(state, slug, desc).await {
        Readings::Record(r) => (r.mode, r.sources),
        Readings::NoRecord => (elohim_chrome_asset::TrustMode::Trustless, Vec::new()),
        Readings::Forbidden => return None,
    };
    Some(elohim_chrome_asset::content_policy(
        mode, &sources, report_uri,
        // No `report-to`: with it present, browsers ignore `report-uri` and queue the report
        // in the Reporting API for about a minute, sent from outside the page. The signal is
        // the algedonic channel, so it goes by `report-uri`, which is delivered promptly.
        None,
    ))
}

/// The delivery path as this doorway can report it, ordered outward: its own
/// hop, then the storage peer whose bytes it serves, named ONLY by the key that
/// peer's manifest reported about itself (`GET /manifest` → `peer.agentKey`).
/// Both hops are reported by the doorway. No `checked` (the doorway did not
/// verify) and no receipt (an `/apps` serve accounts for no exchange today).
async fn delivery_path(
    state: &AppState,
    ctx: &AppServeContext,
) -> elohim_chrome_asset::ContentDelivery {
    use elohim_chrome_asset::{DeliveryHop, HopReporter};
    let me = state.doorway_self();
    let mut path = vec![DeliveryHop::Doorway {
        name: ctx.public_name.clone().unwrap_or_else(|| me.host.clone()),
        id: me.id.clone(),
        did: me.did.clone(),
        reported_by: HopReporter::Doorway,
    }];
    if let Some(storage) = state.args.storage_url.as_deref() {
        if let Some(peer) = state.route_registry.peer_identity(storage).await {
            path.push(DeliveryHop::Holder {
                peer: peer.agent_key,
                transport: None,
                reported_by: HopReporter::Doorway,
            });
        }
    }
    elohim_chrome_asset::ContentDelivery {
        path,
        checked: None,
        receipt: None,
    }
}

/// `/apps/<slug>/<file>[?query]` — what the frame opens. The record's entry
/// is percent-encoded per segment; any other requested file is kept as asked.
fn frame_target(slug: &str, file_path: &str, entry: Option<&str>, query: Option<&str>) -> String {
    let mut target = match entry {
        Some(entry) if entry == file_path => {
            let segments: Vec<String> = entry.split('/').map(encode_segment).collect();
            format!("/apps/{}/{}", encode_segment(slug), segments.join("/"))
        }
        _ => format!("/apps/{slug}/{file_path}"),
    };
    if let Some(q) = query.filter(|q| !q.is_empty()) {
        target.push('?');
        target.push_str(q);
    }
    target
}

/// Build the content shell's context island (typed, schema v1) for one
/// request. With a record: trustful, its readings, and a frame that boots the
/// apps worker on the content origin when the name has one. Without one: the
/// honest "no protocol record" island — trustless, no capabilities, an opaque
/// sandbox, the bytes framed bare.
async fn shell_context(
    state: &AppState,
    slug: &str,
    file_path: &str,
    desc: Option<&AppDescriptor>,
    ctx: &AppServeContext,
) -> Option<elohim_chrome_asset::ChromeContext> {
    use elohim_chrome_asset::{
        ChromeContext, ChromeMedium, ContentFrame, ContentIsland, Isolation, IsolationBoundary,
        TrustMode, WitnessState,
    };
    let readings = match chrome_readings(state, slug, desc).await {
        Readings::Record(readings) => *readings,
        Readings::Forbidden => return None,
        Readings::NoRecord => ChromeReadings {
            record_id: slug.to_string(),
            content_address: slug.to_string(),
            format: "html".to_string(),
            entry: file_path.to_string(),
            title: format!("{slug} \u{2014} no protocol record"),
            mode: TrustMode::Trustless,
            reach: None,
            witness: WitnessState::Amber,
            capabilities: Vec::new(),
            sources: Vec::new(),
            holders: None,
            theme: None,
        },
    };
    let mode = readings.mode;
    let target = frame_target(
        slug,
        file_path,
        Some(readings.entry.as_str()),
        ctx.query.as_deref(),
    );
    let (src, isolation) = match (ctx.edge.as_ref(), mode) {
        // A record on a paired name: the frame boots the worker on the
        // content origin, then opens the target there.
        (Some(edge), TrustMode::Trustful) => (
            format!(
                "{}/_elohim/boot?to={}",
                edge.apps_origin(),
                encode_segment(&target)
            ),
            Isolation {
                boundary: IsolationBoundary::OriginPair,
                content_origin: Some(edge.apps_origin()),
            },
        ),
        // No record: bare bytes in an opaque sandbox (no worker to boot).
        (Some(edge), TrustMode::Trustless) => (
            format!("{}{}", edge.apps_origin(), target),
            Isolation {
                boundary: IsolationBoundary::OpaqueSandbox,
                content_origin: Some(edge.apps_origin()),
            },
        ),
        // No content origin for this name: same-origin bytes, so the chrome
        // must not grant them the page's origin.
        (None, _) => (
            target,
            Isolation {
                boundary: IsolationBoundary::OpaqueSandbox,
                content_origin: None,
            },
        ),
    };
    let mut context = ChromeContext::v1(
        readings.record_id.clone(),
        ctx.viewer,
        ChromeMedium::Web2Doorway,
    );
    context.title = Some(readings.title.clone());
    context.theme = readings.theme.clone();
    context.content = Some(ContentIsland {
        record_id: readings.record_id,
        content_address: readings.content_address,
        format: readings.format,
        entry: readings.entry,
        mode,
        // Display-only; a value outside the island's reach vocabulary (the
        // projection's reach ladder is in known drift) is omitted, never mapped.
        reach: readings
            .reach
            .filter(|r| ISLAND_REACH_VALUES.contains(&r.as_str())),
        witness: readings.witness,
        capabilities: readings.capabilities,
        holders: readings.holders,
        frame: ContentFrame {
            src,
            title: readings.title,
        },
        isolation,
        delivery: delivery_path(state, ctx).await,
    });
    Some(context)
}

/// Answer with the content shell (the shared `render_content_shell`). Never
/// stocked in any cache: `no-store`, no validators, no address claims.
/// Frameable by any page — a third party may frame the CHROMED shell (the
/// chrome travels with the content); no `frame-ancestors`, no
/// `X-Frame-Options`. A refusal from the content-chrome facing answers with
/// the reach refusal, never a shell built from the doorway's own descriptor.
async fn content_shell_response(
    state: &AppState,
    slug: &str,
    file_path: &str,
    desc: Option<&AppDescriptor>,
    ctx: &AppServeContext,
) -> Response<Full<Bytes>> {
    let Some(context) = shell_context(state, slug, file_path, desc, ctx).await else {
        return reach_refused();
    };
    let body = elohim_chrome_asset::render_content_shell(&context);
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Cache-Control", "no-store")
        .header("Vary", CHROME_VARY)
        .header("X-Elohim-Chrome", "content-shell")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

/// `GET /_elohim/content/<slug>/<file>[?query]` on any host: the
/// always-render shell. It never consults Fetch Metadata and never redirects,
/// so the apps origin's redirect to it cannot loop. A record renders its
/// trustful shell; no record renders the honest trustless one.
pub async fn handle_content_shell(
    state: Arc<AppState>,
    path: &str,
    ctx: &AppServeContext,
) -> Response<Full<Bytes>> {
    let rest = path.strip_prefix(CONTENT_SHELL_PREFIX).unwrap_or_default();
    let apps_path = format!("/apps/{rest}");
    let bad_request = || {
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "no-store")
            .body(Full::new(Bytes::from(
                r#"{"error": "Invalid content path"}"#,
            )))
            .unwrap()
    };
    let Some((slug, file_path)) = parse_app_path(&apps_path) else {
        return bad_request();
    };
    if !safe_app_segments(slug, file_path) {
        return bad_request();
    }
    // Reach before any shell output: the shell names the content.
    if let Some(storage_url) = state.args.storage_url.as_deref() {
        if let Some(refused) = cached_app_reach_refusal(storage_url, slug).await {
            return refused;
        }
    }
    let desc = resolve_descriptor(&state, slug)
        .await
        .filter(|d| safe_entry_path(d.entry_file()));
    content_shell_response(&state, slug, file_path, desc.as_ref(), ctx).await
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
// The content origin: a public name pairs with `apps.<name>`
// =============================================================================
//
// Spec: genesis/docs/superpowers/specs/2026-10-14-content-apps-wear-chrome-design.md
// "Decided (2026-10-15)". The pairing is a property of the PUBLIC NAME, derived
// from the Host header alone: any doorway answering the name computes the same
// pairing and keeps no per-instance state, so the rule holds when doorways
// federate a name under DNS anycast. Cache behaviour comes from these headers.

/// Request extension: the request arrived on this doorway's own TLS listener.
#[derive(Debug, Clone, Copy)]
pub struct ArrivedOverTls;

/// Which side of a public name's pairing a request's Host names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingSide {
    /// `<name>` — the chrome's origin (the person's session lives here).
    Name,
    /// `apps.<name>` — the content origin.
    Apps,
}

/// A public name's pairing as one request's Host names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pairing {
    pub side: PairingSide,
    /// The public name, lowercased, port stripped.
    pub name: String,
    /// The port the request named, kept for the URLs built from it.
    pub port: Option<u16>,
}

impl Pairing {
    /// `<name>[:port]`.
    pub fn name_authority(&self) -> String {
        authority(&self.name, self.port)
    }

    /// `apps.<name>[:port]`.
    pub fn apps_authority(&self) -> String {
        authority(&format!("apps.{}", self.name), self.port)
    }
}

fn authority(host: &str, port: Option<u16>) -> String {
    match port {
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    }
}

/// Derive the pairing from a Host header alone. `answered` says whether a bare
/// name is a public name this doorway answers. `apps.<name>` with `<name>`
/// answered is the content origin; an answered `<name>` is the name side;
/// anything else (IP literals included) has no pairing.
pub fn apps_pairing(host: &str, answered: impl Fn(&str) -> bool) -> Option<Pairing> {
    let host = host.trim().to_ascii_lowercase();
    if host.is_empty() || host.starts_with('[') {
        return None;
    }
    let (bare, port) = match host.rsplit_once(':') {
        Some((bare, port)) => (bare, Some(port.parse::<u16>().ok()?)),
        None => (host.as_str(), None),
    };
    let bare = bare.trim_end_matches('.');
    if bare.is_empty() || bare.contains(':') {
        return None;
    }
    if let Some(name) = bare.strip_prefix("apps.") {
        if !name.is_empty() && answered(name) {
            return Some(Pairing {
                side: PairingSide::Apps,
                name: name.to_string(),
                port,
            });
        }
    }
    answered(bare).then(|| Pairing {
        side: PairingSide::Name,
        name: bare.to_string(),
        port,
    })
}

/// The scheme for a URL the doorway builds for this request's browser. Only
/// ever upgrades: a request on the doorway's own TLS listener is `https`
/// whatever any header says, and `X-Forwarded-Proto: https` from a fronting
/// proxy upgrades a plain one — but no header can DOWNGRADE (a caller-supplied
/// `http` is ignored). Otherwise `http` only for a plain request on a loopback
/// or household host (`localhost`, `*.localhost`, `*.local`, `127.*` — names a
/// public CDN never fronts), else `https`.
pub fn request_scheme(headers: &HeaderMap, over_tls: bool, host: &str) -> &'static str {
    if over_tls {
        return "https";
    }
    let forwarded_https = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("https"));
    if forwarded_https {
        return "https";
    }
    let household = crate::services::name_routing::RouteKey::new(Some(host), "/")
        .host
        .is_some_and(|h| {
            h == "localhost"
                || h.ends_with(".localhost")
                || h.ends_with(".local")
                || h.starts_with("127.")
        });
    if household {
        "http"
    } else {
        "https"
    }
}

/// A request's position on its public name's pairing, with the scheme needed
/// to build URLs on either side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppsEdge {
    pub pairing: Pairing,
    pub scheme: &'static str,
}

impl AppsEdge {
    /// The same pairing, seen from the name's side (a request the doorway
    /// re-addresses to the name, e.g. the worker script).
    pub fn as_name_side(&self) -> Self {
        let mut edge = self.clone();
        edge.pairing.side = PairingSide::Name;
        edge
    }

    /// `<scheme>://<name>[:port]`.
    pub fn name_origin(&self) -> String {
        format!("{}://{}", self.scheme, self.pairing.name_authority())
    }

    /// `<scheme>://apps.<name>[:port]`.
    pub fn apps_origin(&self) -> String {
        format!("{}://{}", self.scheme, self.pairing.apps_authority())
    }
}

/// Whether the public name `name` (normalised) declares an apps origin: the
/// name's membership document says so (`"apps": true`) — the members of one
/// name agree by construction — or, for a name with NO document,
/// `DOORWAY_APPS_NAMES` lists it.
fn name_pairs(
    membership: &crate::services::name_routing::standing::PublicNameMembership,
    fallback: &[String],
    name: &str,
) -> bool {
    match membership.document(Some(name)) {
        Some(doc) => doc.apps,
        None => fallback.iter().any(|n| n == name),
    }
}

/// The request's pairing, when its Host names a public name that declares an
/// apps origin, or that name's `apps.` origin. Computed once per request (see
/// [`RequestEdge`]).
pub fn apps_edge(state: &AppState, headers: &HeaderMap, over_tls: bool) -> Option<AppsEdge> {
    let host = headers
        .get(hyper::header::HOST)
        .and_then(|v| v.to_str().ok())?;
    let membership = state.public_names.snapshot();
    let fallback = crate::services::name_routing::standing::normalize_names(&state.args.apps_names);
    if membership.is_empty() && fallback.is_empty() {
        return None;
    }
    let pairing = apps_pairing(host, |name| name_pairs(&membership, &fallback, name))?;
    Some(AppsEdge {
        pairing,
        scheme: request_scheme(headers, over_tls, host),
    })
}

/// Request extension carrying the pairing computed once at the top of the
/// request pipeline (`None` = the name has no apps origin).
#[derive(Debug, Clone)]
pub struct RequestEdge(pub Option<AppsEdge>);

/// The request's pairing: the one computed at the top of the pipeline, or —
/// for a request that did not pass through it (tests, internal dispatch) —
/// computed now.
pub fn request_edge<B>(state: &AppState, req: &hyper::Request<B>) -> Option<AppsEdge> {
    match req.extensions().get::<RequestEdge>() {
        Some(edge) => edge.0.clone(),
        None => apps_edge(
            state,
            req.headers(),
            req.extensions().get::<ArrivedOverTls>().is_some(),
        ),
    }
}

/// What the content origin answers. It is a closed surface: the bundles and
/// their blobs (the worker verifies and caches whole bundles), the worker that
/// serves them offline (the NAME's bytes), the reads that worker makes, the
/// boot page and the CSP report relay. Everything else is 404.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppsHostRoute {
    /// `GET /_elohim/boot` — answered here ([`boot_response`]).
    Boot,
    /// `/apps-sw.js` — dispatched as if the Host were the paired name.
    Worker,
    /// `/apps/...`, `/blob/...`, `/api/v1/peers/delivery`, and `POST`
    /// [`CSP_REPORT_PATH`] — dispatched as usual.
    Pass,
    NotFound,
}

/// Classify a request on the content origin.
pub fn apps_host_route(method: &hyper::Method, path: &str) -> AppsHostRoute {
    if method == hyper::Method::POST {
        return if path.starts_with(CSP_REPORT_PATH) {
            AppsHostRoute::Pass
        } else {
            AppsHostRoute::NotFound
        };
    }
    let is_get = method == hyper::Method::GET;
    if !is_get && method != hyper::Method::HEAD {
        return AppsHostRoute::NotFound;
    }
    match path {
        "/_elohim/boot" if is_get => AppsHostRoute::Boot,
        "/apps-sw.js" => AppsHostRoute::Worker,
        "/api/v1/peers/delivery" => AppsHostRoute::Pass,
        p if p.starts_with("/apps/") || p.starts_with("/blob/") => AppsHostRoute::Pass,
        _ => AppsHostRoute::NotFound,
    }
}

/// The content origin's answer for anything outside its surface.
pub fn apps_host_not_found() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("Content-Type", "text/plain; charset=utf-8")
        .header("Cache-Control", "no-store")
        .body(Full::new(Bytes::from_static(
            b"Not found: this origin serves content bundles only.",
        )))
        .unwrap()
}

/// `Vary` on the content origin: its answer also depends on who is framing.
const APPS_VARY: &str = "Sec-Fetch-Dest, Sec-Fetch-Mode, Sec-Fetch-Site";

/// On the content origin, an html5-app entry the person would otherwise see
/// without the chrome is sent to the shell: a page opening, a frame from
/// another site, or a request with no Fetch Metadata at all. A same-site frame
/// (a wrapper on the paired name) and the worker's own reads get the bytes.
fn apps_host_wants_shell(headers: &HeaderMap) -> bool {
    let get = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.trim().to_ascii_lowercase())
    };
    match get("sec-fetch-dest").as_deref() {
        None | Some("document") => true,
        Some("iframe" | "frame" | "embed" | "object") => get("sec-fetch-site")
            .as_deref()
            .is_none_or(|site| site == "cross-site"),
        Some(_) => false,
    }
}

/// The path prefix of the always-render content shell on the name.
pub const CONTENT_SHELL_PREFIX: &str = "/_elohim/content/";

/// Append `?query` when there is one.
fn with_query(mut url: String, query: Option<&str>) -> String {
    if let Some(q) = query.filter(|q| !q.is_empty()) {
        url.push('?');
        url.push_str(q);
    }
    url
}

/// 302 to the name's always-render shell for `/apps/<slug>/<file>`, keeping
/// the query. The shell path never consults Fetch Metadata, so this cannot
/// loop back here.
fn redirect_to_shell(
    edge: &AppsEdge,
    slug: &str,
    file_path: &str,
    query: Option<&str>,
) -> Response<Full<Bytes>> {
    let location = with_query(
        format!(
            "{}{CONTENT_SHELL_PREFIX}{slug}/{file_path}",
            edge.name_origin()
        ),
        query,
    );
    Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", location)
        .header("Cache-Control", "no-store")
        .header("Vary", APPS_VARY)
        .body(Full::new(Bytes::new()))
        .unwrap()
}

/// True for a top-level page navigation that states it is one through Fetch
/// Metadata (`Sec-Fetch-Dest: document` and `Sec-Fetch-Mode: navigate`).
fn is_top_level_navigation(headers: &HeaderMap) -> bool {
    let has = |name: &str, want: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.eq_ignore_ascii_case(want))
    };
    has("sec-fetch-dest", "document") && has("sec-fetch-mode", "navigate")
}

/// 302 to a projected entry's canonical address on the name,
/// `/apps/<slug>/<entry>`, keeping the query. The name answers a page
/// navigation there with the shell, so only a request carrying that Fetch
/// Metadata is sent here; anything else goes to [`redirect_to_shell`].
fn redirect_to_canonical_entry(
    edge: &AppsEdge,
    slug: &str,
    file_path: &str,
    query: Option<&str>,
) -> Response<Full<Bytes>> {
    let location = with_query(
        format!("{}/apps/{slug}/{file_path}", edge.name_origin()),
        query,
    );
    Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", location)
        .header("Cache-Control", "no-store")
        .header("Vary", APPS_VARY)
        .body(Full::new(Bytes::new()))
        .unwrap()
}

/// 308 from the name to the same path (and query) on its content origin.
pub fn moved_to_apps_origin(
    edge: &AppsEdge,
    path: &str,
    query: Option<&str>,
) -> Response<Full<Bytes>> {
    let location = with_query(format!("{}{}", edge.apps_origin(), path), query);
    Response::builder()
        .status(StatusCode::PERMANENT_REDIRECT)
        .header("Location", location)
        .header("Cache-Control", "no-store")
        .header("Vary", CHROME_VARY)
        .body(Full::new(Bytes::new()))
        .unwrap()
}

/// Raw HTML on the content origin carries the shared content policy
/// (`elohim_chrome_asset::content_policy`: Report-Only for trustful content,
/// enforced for trustless) reporting to the doorway's relay under the content
/// address the bytes were served as (the response's `X-Content-Address`, else
/// the requested identifier), and is never stored (a cached copy could be
/// answered to a request the routing would have sent to the shell).
///
/// No `frame-ancestors`: an enforced one checks EVERY ancestor, so it refused
/// the raw bytes whenever a third party framed the chromed shell (third-party
/// page → shell on the name → bytes on `apps.<name>`). Who may see raw bytes
/// at the top or inside a foreign frame is enforced by Fetch Metadata instead
/// (`apps_host_wants_shell`: a page opening, a cross-site frame and a request
/// with no Fetch Metadata are all sent to the shell); only a same-site frame
/// gets the bytes.
///
/// `Err` is the reach refusal: the content-chrome facing refused the content.
async fn police_raw_html(
    state: &AppState,
    edge: &AppsEdge,
    slug: &str,
    desc: Option<&AppDescriptor>,
    response: &mut Response<Full<Bytes>>,
) -> Result<(), Response<Full<Bytes>>> {
    let is_html = response
        .headers()
        .get(hyper::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ct| ct.trim().to_ascii_lowercase().starts_with("text/html"));
    if !is_html {
        return Ok(());
    }
    let address = response
        .headers()
        .get("x-content-address")
        .and_then(|v| v.to_str().ok())
        .filter(|a| !a.is_empty())
        .unwrap_or(slug)
        .to_string();
    let report_uri = csp_report_url(Some(edge.apps_origin()), &address);
    let Some(policy) = raw_policy(state, slug, desc, &report_uri).await else {
        return Err(reach_refused());
    };
    if let (Ok(name), Ok(value)) = (
        HeaderName::from_bytes(policy.header_name.as_bytes()),
        HeaderValue::from_str(&policy.value),
    ) {
        response.headers_mut().insert(name, value);
    }
    if let Ok(endpoints) = HeaderValue::from_str(&reporting_endpoints(&report_uri)) {
        response
            .headers_mut()
            .insert(HeaderName::from_static("reporting-endpoints"), endpoints);
    }
    response.headers_mut().insert(
        hyper::header::CACHE_CONTROL,
        HeaderValue::from_static("no-store"),
    );
    Ok(())
}

/// Whether a path stays under `/apps/` once a browser normalises it: decoded
/// segment by segment the way the URL standard treats `%2e` as `.`, no dot
/// segment survives, nothing is a backslash or control character, and the
/// path does not leave through `//`.
fn stays_under_apps(path: &str) -> bool {
    let Some(decoded) = percent_decoded(path) else {
        return false;
    };
    path.starts_with("/apps/")
        && decoded.starts_with("/apps/")
        && !path.starts_with("//")
        && !decoded.contains('\\')
        && !decoded.chars().any(char::is_control)
        && decoded.split('/').all(|seg| seg != ".." && seg != ".")
        && path
            .split('/')
            .all(|seg| !seg.eq_ignore_ascii_case("%2e") && !seg.eq_ignore_ascii_case("%2e%2e"))
}

/// The `to` of `/_elohim/boot?to=…`, percent-decoded once (the query layer),
/// when its path stays under `/apps/` after normalisation. Its own query
/// (the app's parameters) is kept.
fn boot_target(query: Option<&str>) -> Option<String> {
    let raw = query?
        .split('&')
        .find_map(|pair| pair.strip_prefix("to="))?;
    let to = urlencoding::decode(raw).ok()?.into_owned();
    if to.contains('#') || to.chars().any(char::is_control) {
        return None;
    }
    let path = to.split('?').next().unwrap_or_default();
    stays_under_apps(path).then_some(to)
}

/// `GET /_elohim/boot?to=/apps/<slug>/<entry>` on the content origin: register
/// the apps worker (scope `/apps/`), wait until it is active or 1500 ms pass,
/// then replace this page with `to`. No-store.
///
/// No `frame-ancestors` (it checks every ancestor and refused the boot page
/// when a third party framed the chromed shell). The same Fetch Metadata rule
/// as raw content governs it instead: only a same-site frame (the shell on the
/// paired name) runs it; a page opening, a cross-site frame or a request with
/// no Fetch Metadata is sent to the shell for `to`. That matters because the
/// boot page's own navigation to `to` is same-origin, so a boot page loaded
/// under a foreign parent would otherwise hand that parent the raw bytes.
pub fn boot_response(
    query: Option<&str>,
    headers: &HeaderMap,
    edge: &AppsEdge,
) -> Response<Full<Bytes>> {
    let Some(to) = boot_target(query) else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "text/plain; charset=utf-8")
            .header("Cache-Control", "no-store")
            .body(Full::new(Bytes::from_static(
                b"Bad request: `to` must be a path under /apps/.",
            )))
            .unwrap();
    };
    if apps_host_wants_shell(headers) {
        let (path, query) = to
            .split_once('?')
            .map_or((to.as_str(), None), |(p, q)| (p, Some(q)));
        let rest = path.strip_prefix("/apps/").unwrap_or_default();
        let location = with_query(
            format!("{}{CONTENT_SHELL_PREFIX}{rest}", edge.name_origin()),
            query,
        );
        return Response::builder()
            .status(StatusCode::FOUND)
            .header("Location", location)
            .header("Cache-Control", "no-store")
            .header("Vary", APPS_VARY)
            .body(Full::new(Bytes::new()))
            .unwrap();
    }
    // JSON string literal with `<` neutralised, so no `</script>` can close it.
    let to_literal = serde_json::to_string(&to)
        .unwrap_or_else(|_| "\"/apps/\"".to_string())
        .replace('<', "\\u003c");
    let body = format!(
        "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\">\
<title>Opening\u{2026}</title></head><body><script>\
(function(){{var to={to_literal};var done=false;\
function go(){{if(!done){{done=true;location.replace(to);}}}}\
setTimeout(go,1500);\
if(!('serviceWorker' in navigator)){{go();return;}}\
navigator.serviceWorker.register('/apps-sw.js',{{scope:'/apps/'}}).then(function(r){{\
if(r.active){{go();return;}}\
var w=r.installing||r.waiting;if(!w){{go();return;}}\
w.addEventListener('statechange',function(){{if(w.state==='activated'){{go();}}}});\
}},go);}})();</script></body></html>"
    );
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Cache-Control", "no-store")
        .header("Vary", APPS_VARY)
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

/// Everything about the request the `/apps` serve needs beyond the path.
#[derive(Debug, Clone)]
pub struct AppServeContext {
    /// The public name's pairing, when it declares an apps origin.
    pub edge: Option<AppsEdge>,
    /// Who is looking, from a VALIDATED session (a header's mere presence is
    /// not one): `hosted-session` when the bearer verifies, else `visitor`.
    pub viewer: elohim_chrome_asset::ViewerKind,
    /// The request's query string (kept on every redirect and frame target).
    pub query: Option<String>,
    /// The public name the request came in on (lowercased, port stripped).
    pub public_name: Option<String>,
}

impl Default for AppServeContext {
    fn default() -> Self {
        Self {
            edge: None,
            viewer: elohim_chrome_asset::ViewerKind::Visitor,
            query: None,
            public_name: None,
        }
    }
}

/// Who is looking: `hosted-session` only once the doorway has VERIFIED the
/// session credential the request presents. Both halves are the ones its
/// authenticated routes use: EXTRACTION by `auth::operator::
/// session_token_from_headers` (the bearer first — an invalid bearer is not
/// rescued by a cookie — else the first `doorway_token` cookie) and
/// VALIDATION by `server::http::verify_session_token`. A forged, expired or
/// merely present credential (including the display-only `elohim_session`
/// projection cookie) is a `visitor`.
fn viewer_kind(state: &AppState, headers: &HeaderMap) -> elohim_chrome_asset::ViewerKind {
    let verified = crate::auth::operator::session_token_from_headers(headers)
        .is_some_and(|token| verify_session_token(state, &token).is_some());
    if verified {
        elohim_chrome_asset::ViewerKind::HostedSession
    } else {
        elohim_chrome_asset::ViewerKind::Visitor
    }
}

/// Build the serve context for one request.
pub fn serve_context<B>(state: &AppState, req: &hyper::Request<B>) -> AppServeContext {
    let edge = request_edge(state, req);
    let viewer = viewer_kind(state, req.headers());
    let host = req
        .headers()
        .get(hyper::header::HOST)
        .and_then(|v| v.to_str().ok());
    AppServeContext {
        public_name: edge
            .as_ref()
            .map(|e| e.pairing.name.clone())
            .or_else(|| crate::services::name_routing::RouteKey::new(host, "/").host),
        query: req
            .uri()
            .query()
            .filter(|q| !q.is_empty())
            .map(str::to_string),
        viewer,
        edge,
    }
}

/// The delivery facts this doorway reports about ITSELF, constant for the
/// process (hoisted into `AppState::doorway_self` at boot).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoorwaySelf {
    /// This doorway instance's id (the one `x-elohim-served-by` names).
    pub id: String,
    /// The `did:web` its `/.well-known/did.json` serves, when it names a real host.
    pub did: Option<String>,
    /// Its own host, the name a hop falls back to when a request named none.
    pub host: String,
}

impl DoorwaySelf {
    pub fn from_args(args: &crate::config::Args) -> Self {
        let did = super::identity::doorway_did(args);
        Self {
            id: crate::server::http::self_doorway_id(args),
            host: did
                .as_deref()
                .and_then(|d| d.strip_prefix("did:web:"))
                .map(str::to_string)
                .unwrap_or_else(|| crate::server::http::self_doorway_id(args)),
            did,
        }
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

    fn fetch_meta(dest: &str, site: Option<&str>) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert("sec-fetch-dest", HeaderValue::from_str(dest).unwrap());
        h.insert("sec-fetch-mode", HeaderValue::from_static("navigate"));
        if let Some(site) = site {
            h.insert("sec-fetch-site", HeaderValue::from_str(site).unwrap());
        }
        h
    }

    fn descriptor(format: &str, title: &str) -> AppDescriptor {
        AppDescriptor {
            blob_hash: "sha256-abcdef123456".into(),
            content_id: "quiz-1".into(),
            content_format: format.into(),
            title: Some(title.into()),
            body: lamad_types::Html5AppBody {
                capabilities: vec![
                    lamad_types::FrameCapability::Forms,
                    lamad_types::FrameCapability::Fullscreen,
                ],
                ..Default::default()
            },
            dht_anchor_hash: None,
            reach: Some("commons".into()),
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
        // By default the facing is UNAVAILABLE (5xx) — the one outcome that
        // lets the descriptor fill in. Tests that need a facing answer mount
        // their own (a lower priority number wins).
        Mock::given(method("GET"))
            .and(wiremock::matchers::path_regex(
                r"^/api/v1/epr/[^/]+/chrome$",
            ))
            .respond_with(ResponseTemplate::new(503))
            .with_priority(10)
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
        serve_at(
            state,
            storage,
            file,
            headers,
            desc,
            &AppServeContext::default(),
        )
        .await
    }

    async fn serve_at(
        state: &Arc<AppState>,
        storage: &wiremock::MockServer,
        file: &str,
        headers: &HeaderMap,
        desc: Option<&AppDescriptor>,
        ctx: &AppServeContext,
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
            ctx,
        )
        .await
    }

    /// A serve context on `host`, paired as a name declaring `apps` would be.
    fn paired(host: &str) -> AppServeContext {
        let pairing = apps_pairing(host, |n| n == "elohim.localhost").unwrap();
        AppServeContext {
            public_name: Some(pairing.name.clone()),
            edge: Some(AppsEdge {
                pairing,
                scheme: "http",
            }),
            ..AppServeContext::default()
        }
    }

    fn island_of(body: &str) -> serde_json::Value {
        let start =
            body.find("id=\"elohim-omni-context\">").unwrap() + "id=\"elohim-omni-context\">".len();
        let end = start + body[start..].find("</script>").unwrap();
        serde_json::from_str(&body[start..end]).unwrap()
    }

    async fn body_of(resp: Response<Full<Bytes>>) -> String {
        use http_body_util::BodyExt;
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    async fn shell_island(
        desc: Option<&AppDescriptor>,
        ctx: &AppServeContext,
    ) -> serde_json::Value {
        let state = state_for("http://127.0.0.1:9").await;
        let context = shell_context(&state, "quiz-1", "index.html", desc, ctx)
            .await
            .expect("an unavailable facing never refuses");
        serde_json::to_value(&context).unwrap()
    }

    #[tokio::test]
    async fn a_page_navigation_to_an_html5_entry_gets_the_shell() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};
        let storage = storage_with_entry().await;
        // Holders are the element's lazy read, never a fetch on the shell path.
        Mock::given(method("GET"))
            .and(path("/api/v1/resilience/quiz-1/household"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
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
        // A third party may frame the CHROMED shell: no framing restriction.
        assert!(h.get("content-security-policy").is_none());
        assert!(h.get("x-frame-options").is_none());
        assert!(h.get("etag").is_none());
        assert!(h.get("x-content-address").is_none());
        assert!(h.get("x-cache").is_none());
        let body = body_of(resp).await;
        // The shared renderer: no bar, no iframe of its own.
        assert!(!body.contains("content-chrome"));
        assert!(!body.contains("<iframe"));
        assert!(body.contains(
            "<main id=\"elohim-content-frame\" data-frame-src=\"/apps/quiz-1/index.html\" data-frame-title=\"A quiz\">"
        ));
        assert!(body.contains("/chrome/omni-element."));
        let island = island_of(&body);
        assert_eq!(island["version"], 1);
        assert_eq!(island["medium"], "web2-doorway");
        assert_eq!(island["viewer"]["kind"], "visitor");
        assert_eq!(island["authenticated"], false);
        assert_eq!(island["slug"], "quiz-1");
        let content = &island["content"];
        assert_eq!(content["recordId"], "quiz-1");
        assert_eq!(content["contentAddress"], "sha256-abcdef123456");
        assert_eq!(content["format"], "html5-app");
        assert_eq!(content["entry"], "index.html");
        assert_eq!(content["mode"], "trustful");
        assert_eq!(content["witness"], "amber");
        assert_eq!(content["reach"], "commons");
        assert_eq!(
            content["capabilities"],
            serde_json::json!(["forms", "fullscreen"])
        );
        assert!(content["holders"].is_null());
        assert_eq!(
            content["frame"],
            serde_json::json!({ "src": "/apps/quiz-1/index.html", "title": "A quiz" })
        );
        // Unpaired: the bytes share the page's origin, so they run sandboxed.
        assert_eq!(
            content["isolation"],
            serde_json::json!({ "boundary": "opaque-sandbox" })
        );
        let delivery = &content["delivery"];
        assert_eq!(delivery["path"][0]["role"], "doorway");
        assert_eq!(delivery["path"][0]["reportedBy"], "doorway");
        assert!(delivery.get("checked").is_none());
        assert!(delivery.get("receipt").is_none());
        // No facing answered (404), so the descriptor filled in; the shell
        // never asked storage for the bundle bytes nor took a holders reading.
        let gets: Vec<String> = storage
            .received_requests()
            .await
            .unwrap()
            .into_iter()
            .filter(|r| r.method.as_str() == "GET")
            .map(|r| r.url.path().to_string())
            .collect();
        assert_eq!(gets, vec!["/api/v1/epr/quiz-1/chrome".to_string()]);
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
        let cold = serve(&state, &storage, "index.html", &HeaderMap::new(), None).await;
        assert_eq!(
            cold.headers().get("vary").unwrap(),
            "Sec-Fetch-Dest, Sec-Fetch-Mode"
        );
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
        assert_eq!(body_of(sub).await, "js");

        let other = serve(&state, &storage, "index.html", &nav_headers(), Some(&spa)).await;
        assert!(other.headers().get("x-elohim-chrome").is_none());
        assert_eq!(body_of(other).await, "<h1>raw bundle</h1>");

        let none = serve(&state, &storage, "index.html", &nav_headers(), None).await;
        assert!(none.headers().get("x-elohim-chrome").is_none());
        assert_eq!(body_of(none).await, "<h1>raw bundle</h1>");

        // Through the public entry with no projection cache: no descriptor, raw.
        let via = handle_app_request(
            state.clone(),
            "/apps/quiz-1/index.html",
            &nav_headers(),
            &AppServeContext::default(),
        )
        .await;
        assert!(via.headers().get("x-elohim-chrome").is_none());
        assert_eq!(body_of(via).await, "<h1>raw bundle</h1>");
    }

    #[tokio::test]
    async fn a_declared_entry_point_is_the_chrome_target_not_index_html() {
        let storage = storage_with_entry().await;
        let state = state_for(&storage.uri()).await;
        let desc = AppDescriptor::from_data(&bson::doc! {
            "id": "quiz-1",
            "contentFormat": "html5-app",
            "blobHash": "sha256-abcdef123456",
            "title": "A quiz",
            "contentBody": r#"{"slug":"quiz-1","entryPoint":"game.html"}"#,
        })
        .unwrap();
        let index = serve(&state, &storage, "index.html", &nav_headers(), Some(&desc)).await;
        assert!(index.headers().get("x-elohim-chrome").is_none());
        let game = serve(&state, &storage, "game.html", &nav_headers(), Some(&desc)).await;
        assert_eq!(
            game.headers().get("x-elohim-chrome").unwrap(),
            "content-shell"
        );
        assert!(body_of(game)
            .await
            .contains("data-frame-src=\"/apps/quiz-1/game.html\""));
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
            &AppServeContext::default(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        assert!(resp.headers().get("x-elohim-chrome").is_none());
        // The always-render shell path refuses the same way.
        let shell = handle_content_shell(
            state,
            "/_elohim/content/refused-app/index.html",
            &AppServeContext::default(),
        )
        .await;
        assert_eq!(shell.status(), StatusCode::FORBIDDEN);
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
            &AppServeContext::default(),
        )
        .await;
        assert_eq!(
            resp.headers().get("x-elohim-chrome").unwrap(),
            "content-shell"
        );
    }

    #[tokio::test]
    async fn traversal_is_refused_encoded_and_under_the_admin_bypass() {
        use wiremock::MockServer;
        let storage = MockServer::start().await;
        let state = state_for(&storage.uri()).await;
        state
            .cache_enabled
            .store(false, std::sync::atomic::Ordering::Relaxed);
        for path in [
            "/apps/%2e%2e/db/content/x",
            "/apps/%2E%2E/db/content/x",
            "/apps/quiz-1/..%2fsecret",
            "/apps/quiz-1/%2e%2e/x.html",
            "/apps/a%2Fb/index.html",
            "/apps/quiz-1/a%5Cb.html",
            "/apps/quiz-1",
        ] {
            let resp = handle_app_request(
                state.clone(),
                path,
                &HeaderMap::new(),
                &AppServeContext::default(),
            )
            .await;
            assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "{path}");
        }
        assert!(
            storage.received_requests().await.unwrap().is_empty(),
            "nothing refused may be forwarded, bypass or not"
        );
        assert!(safe_app_segments("quiz-1", "dir/a%20b.html"));
        assert!(safe_app_segments("sha256-abc", "index.html"));
    }

    #[tokio::test]
    async fn without_a_facing_the_witness_reads_amber_even_when_anchored() {
        // Whether a peer other than the author holds the record is the storage
        // peer's reading; the doorway's own descriptor can't answer it, so the
        // fallback never turns green from an anchor alone.
        let mut desc = descriptor("html5-app", "A quiz");
        let ctx = AppServeContext::default();
        assert_eq!(
            shell_island(Some(&desc), &ctx).await["content"]["witness"],
            "amber"
        );
        desc.dht_anchor_hash = Some("uhCkkAnchor".into());
        assert_eq!(
            shell_island(Some(&desc), &ctx).await["content"]["witness"],
            "amber"
        );
    }

    #[tokio::test]
    async fn capabilities_reach_and_theme_come_from_the_projected_record() {
        // contentBody as a JSON string, then as a BSON document.
        for body in [
            bson::Bson::String(
                r#"{"slug":"quiz-1","capabilities":["popups","downloads","bogus"],"themeHint":{"colorScheme":"dark"}}"#
                    .into(),
            ),
            bson::Bson::Document(bson::doc! {
                "slug": "quiz-1",
                "capabilities": ["popups", "downloads", "bogus"],
                "themeHint": { "colorScheme": "dark" },
            }),
        ] {
            let desc = AppDescriptor::from_data(&bson::doc! {
                "id": "quiz-1", "contentFormat": "html5-app", "blobHash": "sha256-abcdef123456",
                "reach": "local", "contentBody": body,
            })
            .unwrap();
            let island = shell_island(Some(&desc), &AppServeContext::default()).await;
            assert_eq!(
                island["content"]["capabilities"],
                serde_json::json!(["popups", "downloads"])
            );
            // "local" is outside the island's reach vocabulary: omitted, not mapped.
            assert!(island["content"].get("reach").is_none());
            assert_eq!(island["theme"], serde_json::json!({ "colorScheme": "dark" }));
        }
        // None declared: an empty list, never a default grant; no theme.
        let bare = AppDescriptor::from_data(&bson::doc! {
            "id": "quiz-1", "contentFormat": "html5-app", "blobHash": "sha256-x",
        })
        .unwrap();
        let island = shell_island(Some(&bare), &AppServeContext::default()).await;
        assert_eq!(island["content"]["capabilities"], serde_json::json!([]));
        assert!(island.get("theme").is_none());
    }

    fn facing_json(record: &str) -> serde_json::Value {
        serde_json::json!({
            "recordId": record,
            "title": "A quiz, as the record says",
            "format": "html5-app",
            "mode": "trustful",
            "reach": "community",
            "witness": "green",
            "computedAt": "2026-10-15T00:00:00Z",
            "contentAddress": "bafkreifacing",
            "app": {
                "entry": "index.html",
                "capabilities": ["popups"],
                "presents": false,
                "themeHint": { "colorScheme": "dark", "themeColor": "#112233" },
            },
            "holders": {
                "holders": 2, "target": 3,
                "headline": "Held by 2 of 3 households",
                "reassurance": "watching",
                "asOf": "2026-10-15T00:00:00Z",
                "heldBy": [],
            },
            "claims": { "attestations": [] },
            "steward": { "stewards": [] },
        })
    }

    #[tokio::test]
    async fn the_shell_takes_its_readings_from_the_content_chrome_facing() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};
        let storage = storage_with_entry().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/epr/quiz-1/chrome"))
            .respond_with(ResponseTemplate::new(200).set_body_json(facing_json("quiz-1")))
            .expect(1)
            .mount(&storage)
            .await;
        // The shell itself never takes a holders reading.
        Mock::given(method("GET"))
            .and(path("/api/v1/resilience/quiz-1/household"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&storage)
            .await;
        let state = state_for(&storage.uri()).await;
        // The doorway's own descriptor disagrees on everything the facing
        // carries; the facing wins and nothing is re-derived.
        let desc = descriptor("html5-app", "stale title");
        let resp = serve(&state, &storage, "index.html", &nav_headers(), Some(&desc)).await;
        let island = island_of(&body_of(resp).await);
        let content = &island["content"];
        assert_eq!(island["title"], "A quiz, as the record says");
        assert_eq!(
            island["theme"],
            serde_json::json!({ "colorScheme": "dark", "themeColor": "#112233" })
        );
        assert_eq!(content["witness"], "green");
        assert_eq!(content["reach"], "community");
        assert_eq!(content["contentAddress"], "bafkreifacing");
        assert_eq!(content["capabilities"], serde_json::json!(["popups"]));
        // Carried only because the facing already took the reading.
        assert_eq!(
            content["holders"],
            serde_json::json!({
                "holders": 2, "target": 3,
                "headline": "Held by 2 of 3 households",
                "asOf": "2026-10-15T00:00:00Z",
            })
        );
    }

    #[tokio::test]
    async fn the_facing_is_read_through_the_registry_route_and_its_cache() {
        use doorway_client::{DoorwayRoutesBuilder, Route};
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};
        let storage = storage_with_entry().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/epr/sha256-registry-cached/chrome"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(facing_json("sha256-registry-cached")),
            )
            .expect(1)
            .mount(&storage)
            .await;
        // A doorway whose STORAGE_URL is elsewhere: the registry's declared
        // route decides the one peer asked, and its cache_ttl the reuse.
        let state = state_for("http://127.0.0.1:9").await;
        state
            .route_registry
            .install_steward_routes(
                &storage.uri(),
                &DoorwayRoutesBuilder::new()
                    .route(
                        Route::get("/api/v1/epr/{id}/chrome")
                            .handler("epr_chrome")
                            .cache_ttl(30)
                            .build(),
                    )
                    .build(),
            )
            .await;
        for _ in 0..2 {
            match fetch_chrome_facing(&state, "sha256-registry-cached").await {
                FacingRead::Found(facing) => {
                    assert_eq!(facing.record_id, "sha256-registry-cached")
                }
                other => panic!("expected the facing, got {other:?}"),
            }
        }
        // A record no projection here names, found by the facing: trustful.
        match chrome_readings(&state, "sha256-registry-cached", None).await {
            Readings::Record(r) => assert_eq!(r.mode, elohim_chrome_asset::TrustMode::Trustful),
            other => panic!("expected a record, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn the_viewer_is_a_hosted_session_only_with_a_validated_session() {
        use clap::Parser;
        const SECRET: &str = "test-secret-that-is-at-least-32-characters-long";
        let mut args = crate::config::Args::parse_from(["doorway", "--listen", "127.0.0.1:0"]);
        args.jwt_secret = Some(SECRET.into());
        let state = AppState::new(args);
        let mint = |secret: &str, expiry: u64| {
            crate::auth::jwt::JwtValidator::new(secret.into(), expiry)
                .unwrap()
                .generate_token(crate::auth::jwt::TokenInput {
                    human_id: "human-maria".into(),
                    agent_pub_key: "uhCAkMaria".into(),
                    identifier: "maria@example.com".into(),
                    permission_level: crate::auth::PermissionLevel::Authenticated,
                    session_id: None,
                    doorway_id: None,
                    doorway_url: None,
                    conductor_id: None,
                    installed_app_id: None,
                    is_steward: false,
                    has_local_conductor: false,
                })
                .unwrap()
        };
        let req = |auth: Option<&str>, cookie: Option<&str>| {
            let mut b = hyper::Request::builder().uri("/apps/quiz-1/index.html");
            if let Some(a) = auth {
                b = b.header("authorization", a);
            }
            if let Some(c) = cookie {
                b = b.header("cookie", c);
            }
            b.body(()).unwrap()
        };
        // A credential's mere presence is not a session: no header, a junk
        // bearer, the display-only session projection, a forged session cookie
        // (signed with another secret), an expired one.
        let forged = mint("another-secret-that-is-at-least-32-characters", 3600);
        // Expired well past the validator's clock leeway: the same claims,
        // re-signed with the doorway's own secret, `exp` an hour ago.
        let expired = {
            use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
            let mut insecure = Validation::default();
            insecure.insecure_disable_signature_validation();
            insecure.validate_exp = false;
            let mut claims = decode::<serde_json::Value>(
                &mint(SECRET, 3600),
                &DecodingKey::from_secret(b"unused"),
                &insecure,
            )
            .unwrap()
            .claims;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            claims["iat"] = serde_json::json!(now - 7200);
            claims["exp"] = serde_json::json!(now - 3600);
            encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(SECRET.as_bytes()),
            )
            .unwrap()
        };
        for r in [
            req(None, None),
            req(Some("Bearer not-a-token"), None),
            req(None, Some("doorway_session=anything")),
            req(
                None,
                Some("elohim_session=%7B%22humanId%22%3A%22maria%22%7D"),
            ),
            req(None, Some(&format!("doorway_token={forged}"))),
            req(None, Some(&format!("theme=dark; doorway_token={expired}"))),
            req(None, Some("doorway_token=garbage.not.ajwt")),
        ] {
            assert_eq!(
                serve_context(&state, &r).viewer,
                elohim_chrome_asset::ViewerKind::Visitor
            );
        }
        let token = mint(SECRET, 3600);
        // Chosen the way the authenticated routes choose (auth::operator): an
        // invalid bearer is not rescued by a valid cookie, and of duplicate
        // `doorway_token` cookies the first is the one presented.
        for r in [
            req(
                Some("Bearer not-a-token"),
                Some(&format!("doorway_token={token}")),
            ),
            req(
                None,
                Some(&format!("doorway_token=garbage; doorway_token={token}")),
            ),
        ] {
            assert_eq!(
                serve_context(&state, &r).viewer,
                elohim_chrome_asset::ViewerKind::Visitor
            );
        }
        assert_eq!(
            serve_context(
                &state,
                &req(
                    None,
                    Some(&format!("doorway_token={token}; doorway_token=garbage"))
                )
            )
            .viewer,
            elohim_chrome_asset::ViewerKind::HostedSession
        );
        // The session cookie the doorway's authenticated routes accept…
        let cookie = serve_context(
            &state,
            &req(None, Some(&format!("theme=dark; doorway_token={token}"))),
        );
        assert_eq!(
            cookie.viewer,
            elohim_chrome_asset::ViewerKind::HostedSession
        );
        // …and the bearer.
        let ctx = serve_context(&state, &req(Some(&format!("Bearer {token}")), None));
        assert_eq!(ctx.viewer, elohim_chrome_asset::ViewerKind::HostedSession);
        let island =
            serde_json::to_value(shell_context(&state, "quiz-1", "index.html", None, &ctx).await)
                .unwrap();
        // (no storage: the facing is unavailable and there is no descriptor,
        // so this is the no-record shell — still carrying the verified viewer)
        assert_eq!(island["viewer"]["kind"], "hosted-session");
        assert_eq!(island["authenticated"], true);
    }

    #[tokio::test]
    async fn delivery_reports_the_doorway_hop_and_the_holder_by_its_own_key() {
        use clap::Parser;
        let mut args = crate::config::Args::parse_from([
            "doorway",
            "--listen",
            "127.0.0.1:0",
            "--doorway-id",
            "alpha",
            "--doorway-url",
            "https://alpha.elohim.host",
        ]);
        args.storage_url = Some("http://storage-a:8090/".into());
        let me = DoorwaySelf::from_args(&args);
        assert_eq!(me.id, "alpha");
        assert_eq!(me.did.as_deref(), Some("did:web:alpha"));
        let state = AppState::new(args);
        // Fixed at boot: the same facts every request.
        assert_eq!(state.doorway_self(), &me);

        let ctx = paired("elohim.localhost:8888");
        // Before the storage peer's manifest named its key: the doorway hop only.
        let delivery = serde_json::to_value(delivery_path(&state, &ctx).await).unwrap();
        assert_eq!(
            delivery,
            serde_json::json!({
                "path": [{
                    "role": "doorway",
                    "name": "elohim.localhost",
                    "id": "alpha",
                    "did": "did:web:alpha",
                    "reportedBy": "doorway",
                }],
            })
        );
        // The manifest's `peer` (what storage says about itself) names the
        // holder hop — by key only, reported by the doorway, nothing checked.
        let (_, identity) =
            crate::services::route_registry::parse_manifest_with_peer(serde_json::json!({
                "version": 1, "routes": [],
                "peer": { "agentKey": "uhCAkMatthewPeer", "transportId": "12D3KooW" },
            }))
            .unwrap();
        state
            .route_registry
            .record_peer_identity("http://storage-a:8090", identity)
            .await;
        let delivery = serde_json::to_value(delivery_path(&state, &ctx).await).unwrap();
        assert_eq!(
            delivery["path"][1],
            serde_json::json!({
                "role": "holder",
                "peer": "uhCAkMatthewPeer",
                "reportedBy": "doorway",
            })
        );
        assert!(delivery.get("checked").is_none());
        // No name on the request: the doorway's own host, never a guess.
        let delivery =
            serde_json::to_value(delivery_path(&state, &AppServeContext::default()).await).unwrap();
        assert_eq!(delivery["path"][0]["name"], "alpha");
        // A doorway with no declared id/url has no did:web to report.
        let bare = crate::config::Args::parse_from(["doorway", "--listen", "127.0.0.1:0"]);
        assert_eq!(DoorwaySelf::from_args(&bare).did, None);
    }

    #[tokio::test]
    async fn a_hostile_title_is_escaped_everywhere_it_lands() {
        let state = state_for("http://127.0.0.1:9").await;
        let desc = descriptor("html5-app", "</title><script>alert(1)</script><!--\"'&");
        let resp = content_shell_response(
            &state,
            "quiz-1",
            "index.html",
            Some(&desc),
            &AppServeContext::default(),
        )
        .await;
        let html = body_of(resp).await;
        // Only the island and the element script close; the island's copies of
        // the title are JSON data that still parses.
        assert_eq!(html.matches("</script>").count(), 2);
        assert!(!html.contains("<title></title>"));
        assert_eq!(
            island_of(&html)["content"]["frame"]["title"],
            "</title><script>alert(1)</script><!--\"'&"
        );
    }

    #[tokio::test]
    async fn entry_paths_are_encoded_per_segment_and_hostile_ones_get_no_shell() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};
        let storage = storage_with_entry().await;
        Mock::given(method("GET"))
            .and(path("/apps/quiz-1/../x.html"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&storage)
            .await;
        let state = state_for(&storage.uri()).await;

        let mut spaced = descriptor("html5-app", "A quiz");
        spaced.body.entry_point = "dir/a b.html".into();
        let resp = serve(
            &state,
            &storage,
            "dir/a b.html",
            &nav_headers(),
            Some(&spaced),
        )
        .await;
        assert_eq!(
            resp.headers().get("x-elohim-chrome").unwrap(),
            "content-shell"
        );
        assert!(body_of(resp)
            .await
            .contains("data-frame-src=\"/apps/quiz-1/dir/a%20b.html\""));

        for hostile in ["../x.html", "/abs.html", "a\\b.html", "a/../b.html"] {
            let mut d = descriptor("html5-app", "A quiz");
            d.body.entry_point = hostile.into();
            let file = d.entry_file().to_string();
            let resp = serve(&state, &storage, &file, &nav_headers(), Some(&d)).await;
            assert!(
                resp.headers().get("x-elohim-chrome").is_none(),
                "{hostile} must not get a shell"
            );
        }
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

    // =========================================================================
    // The content origin: apps.<name>
    // =========================================================================

    #[test]
    fn the_pairing_is_derived_from_the_host_alone() {
        let answered = |n: &str| n == "elohim.host" || n == "elohim.localhost";
        assert_eq!(
            apps_pairing("Apps.Elohim.Host", answered),
            Some(Pairing {
                side: PairingSide::Apps,
                name: "elohim.host".into(),
                port: None
            })
        );
        let name = apps_pairing("elohim.localhost:8888", answered).unwrap();
        assert_eq!(name.side, PairingSide::Name);
        assert_eq!(name.name_authority(), "elohim.localhost:8888");
        assert_eq!(name.apps_authority(), "apps.elohim.localhost:8888");
        for host in [
            "other.host",
            "apps.other.host",
            "apps.",
            "127.0.0.1:8888",
            "[::1]:8888",
            "elohim.host:http",
            "",
        ] {
            assert_eq!(apps_pairing(host, answered), None, "{host}");
        }
    }

    #[test]
    fn built_urls_never_downgrade_the_scheme() {
        let none = HeaderMap::new();
        let mut fwd_http = HeaderMap::new();
        fwd_http.insert("x-forwarded-proto", HeaderValue::from_static("http"));
        let mut fwd_https = HeaderMap::new();
        fwd_https.insert("x-forwarded-proto", HeaderValue::from_static("https"));
        assert_eq!(
            request_scheme(&none, false, "elohim.localhost:8888"),
            "http"
        );
        assert_eq!(request_scheme(&none, false, "alpha.elohim.host"), "https");
        // Arriving over TLS is https whatever a header claims.
        assert_eq!(request_scheme(&fwd_http, true, "elohim.localhost"), "https");
        // A caller-supplied http never downgrades a public name.
        assert_eq!(
            request_scheme(&fwd_http, false, "alpha.elohim.host"),
            "https"
        );
        // A fronting proxy's https upgrades.
        assert_eq!(
            request_scheme(&fwd_https, false, "elohim.localhost"),
            "https"
        );
    }

    #[test]
    fn only_names_that_declare_apps_pair() {
        use crate::services::name_routing::standing::{
            parse_document, PublicNameDocument, PublicNameMembership,
        };
        use clap::Parser;
        // The flag is part of the membership document's wire shape.
        let doc = parse_document(br#"{"name":"elohim.host","members":[],"apps":true}"#).unwrap();
        assert!(doc.apps);
        let legacy = parse_document(br#"{"name":"elohim.local","members":[]}"#).unwrap();
        assert!(!legacy.apps);

        let mut args = crate::config::Args::parse_from(["doorway", "--listen", "127.0.0.1:0"]);
        // A fallback entry for a name that HAS a document is ignored.
        args.apps_names = vec!["elohim.local".into(), "Elohim.Localhost".into()];
        let state = AppState::new(args);
        state
            .public_names
            .install(PublicNameMembership::from_documents(vec![
                doc,
                legacy,
                PublicNameDocument {
                    name: "alpha.elohim.local".into(),
                    members: vec![],
                    apps: false,
                },
            ]));
        let edge = |host: &'static str| {
            let mut h = HeaderMap::new();
            h.insert("host", HeaderValue::from_static(host));
            apps_edge(&state, &h, false).map(|e| e.pairing.side)
        };
        assert_eq!(edge("apps.elohim.host"), Some(PairingSide::Apps));
        assert_eq!(edge("elohim.host"), Some(PairingSide::Name));
        // The household's lane names keep their behaviour.
        assert_eq!(edge("elohim.local:8888"), None);
        assert_eq!(edge("apps.elohim.local:8888"), None);
        assert_eq!(edge("alpha.elohim.local"), None);
        // A name with no document pairs through the fallback list.
        assert_eq!(edge("elohim.localhost:8888"), Some(PairingSide::Name));
        assert_eq!(edge("apps.elohim.localhost:8888"), Some(PairingSide::Apps));

        // Nothing declared at all: no pairing anywhere.
        let bare = AppState::new(crate::config::Args::parse_from([
            "doorway",
            "--listen",
            "127.0.0.1:0",
        ]));
        let mut h = HeaderMap::new();
        h.insert("host", HeaderValue::from_static("apps.elohim.host"));
        assert_eq!(apps_edge(&bare, &h, false), None);
    }

    #[test]
    fn the_apps_host_answers_only_its_closed_surface() {
        use hyper::Method;
        assert_eq!(
            apps_host_route(&Method::GET, "/_elohim/boot"),
            AppsHostRoute::Boot
        );
        assert_eq!(
            apps_host_route(&Method::GET, "/apps-sw.js"),
            AppsHostRoute::Worker
        );
        for (method, path) in [
            (Method::HEAD, "/apps/quiz-1/_capability"),
            (Method::GET, "/apps/quiz-1/index.html"),
            (Method::GET, "/blob/sha256-abc"),
            (Method::GET, "/api/v1/peers/delivery"),
            (Method::POST, CSP_REPORT_PATH),
        ] {
            assert_eq!(
                apps_host_route(&method, path),
                AppsHostRoute::Pass,
                "{method} {path}"
            );
        }
        for (method, path) in [
            (Method::GET, "/"),
            (Method::GET, "/epr/quiz-1"),
            (Method::GET, "/auth/login"),
            (Method::GET, "/_elohim/content/quiz-1/index.html"),
            (Method::GET, "/api/v1/cache/content"),
            (Method::POST, "/apps/quiz-1/index.html"),
            (Method::HEAD, "/_elohim/boot"),
        ] {
            assert_eq!(
                apps_host_route(&method, path),
                AppsHostRoute::NotFound,
                "{method} {path}"
            );
        }
        assert_eq!(apps_host_not_found().status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn on_the_apps_host_html_people_would_see_bare_goes_to_the_shell() {
        let storage = storage_with_entry().await;
        let state = state_for(&storage.uri()).await;
        let desc = descriptor("html5-app", "A quiz");
        let mut ctx = paired("apps.elohim.localhost:8888");
        ctx.query = Some("level=2".into());
        let shell = "http://elohim.localhost:8888/_elohim/content/quiz-1/index.html?level=2";
        let canonical = "http://elohim.localhost:8888/apps/quiz-1/index.html?level=2";
        // A page opening, a cross-site frame, no Fetch Metadata at all.
        for (is_page, headers) in [
            (true, nav_headers()),
            (false, fetch_meta("iframe", Some("cross-site"))),
            (false, HeaderMap::new()),
        ] {
            for desc in [Some(&desc), None] {
                let resp = serve_at(&state, &storage, "index.html", &headers, desc, &ctx).await;
                assert_eq!(resp.status(), StatusCode::FOUND);
                // A tab opening a projected entry lands on its canonical address (the
                // name renders the shell for a page navigation there); everything else
                // goes to the always-render path, which cannot loop.
                let expected = if is_page && desc.is_some() {
                    canonical
                } else {
                    shell
                };
                assert_eq!(resp.headers().get("location").unwrap(), expected);
                assert_eq!(resp.headers().get("cache-control").unwrap(), "no-store");
                assert_eq!(resp.headers().get("vary").unwrap(), APPS_VARY);
            }
        }
        // A content-address path is no exception.
        let addressed = serve_app_file(
            &state,
            &storage.uri(),
            "/apps/sha256-abcdef123456/index.html",
            "sha256-abcdef123456",
            "index.html",
            &nav_headers(),
            None,
            false,
            &ctx,
        )
        .await;
        assert_eq!(addressed.status(), StatusCode::FOUND);
        assert_eq!(
            addressed.headers().get("location").unwrap(),
            "http://elohim.localhost:8888/_elohim/content/sha256-abcdef123456/index.html?level=2"
        );
    }

    #[tokio::test]
    async fn the_redirect_chain_ends_at_a_shell_that_always_renders() {
        let storage = storage_with_entry().await;
        let state = state_for(&storage.uri()).await;
        // Whatever the follow-up carries (here: nothing), the shell path
        // renders; with no projected record it is the honest trustless shell.
        let ctx = paired("elohim.localhost:8888");
        let resp = handle_content_shell(
            state.clone(),
            "/_elohim/content/quiz-1/index.html",
            &AppServeContext {
                query: Some("level=2".into()),
                ..ctx
            },
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get("x-elohim-chrome").unwrap(),
            "content-shell"
        );
        let island = island_of(&body_of(resp).await);
        let content = &island["content"];
        assert_eq!(content["mode"], "trustless");
        assert_eq!(content["capabilities"], serde_json::json!([]));
        assert_eq!(content["witness"], "amber");
        assert_eq!(
            content["isolation"],
            serde_json::json!({
                "boundary": "opaque-sandbox",
                "contentOrigin": "http://apps.elohim.localhost:8888",
            })
        );
        assert_eq!(
            content["frame"]["src"],
            "http://apps.elohim.localhost:8888/apps/quiz-1/index.html?level=2"
        );
        // A malformed or traversing shell path is refused, never redirected.
        for bad in [
            "/_elohim/content/quiz-1",
            "/_elohim/content/%2e%2e/index.html",
        ] {
            let resp = handle_content_shell(state.clone(), bad, &AppServeContext::default()).await;
            assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "{bad}");
        }
    }

    #[tokio::test]
    async fn on_the_apps_host_a_same_site_frame_gets_raw_bytes_under_the_shared_policy() {
        let storage = storage_with_entry().await;
        let state = state_for(&storage.uri()).await;
        let desc = descriptor("html5-app", "A quiz");
        let ctx = paired("apps.elohim.localhost:8888");
        for site in ["same-site", "same-origin"] {
            let headers = fetch_meta("iframe", Some(site));
            let resp = serve_at(&state, &storage, "index.html", &headers, Some(&desc), &ctx).await;
            assert_eq!(resp.status(), StatusCode::OK, "{site}");
            let h = resp.headers();
            // The shared content policy (trustful → Report-Only), reporting
            // under the content's own address (here the requested identifier —
            // the storage mock sends no X-Content-Address). No frame-ancestors:
            // Fetch Metadata already decided this frame may have the bytes.
            let expected = elohim_chrome_asset::content_policy(
                elohim_chrome_asset::TrustMode::Trustful,
                &[],
                "http://apps.elohim.localhost:8888/_elohim/csp-report/quiz-1",
                None,
            );
            assert!(
                !expected.value.contains("report-to"),
                "report-to would make browsers queue the signal instead of sending it"
            );
            assert_eq!(expected.header_name, "Content-Security-Policy-Report-Only");
            assert_eq!(
                h.get("content-security-policy-report-only").unwrap(),
                expected.value.as_str()
            );
            assert!(h.get("content-security-policy").is_none());
            assert!(!expected.value.contains("frame-ancestors"));
            assert_eq!(
                h.get("reporting-endpoints").unwrap(),
                "elohim-signals=\"http://apps.elohim.localhost:8888/_elohim/csp-report/quiz-1\""
            );
            assert_eq!(h.get("cache-control").unwrap(), "no-store");
            assert_eq!(h.get("vary").unwrap(), APPS_VARY);
            assert!(h.get("x-elohim-chrome").is_none());
            assert_eq!(body_of(resp).await, "<h1>raw bundle</h1>");
        }
        // The worker's own read of a subresource: raw, no frame policy.
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};
        Mock::given(method("GET"))
            .and(path("/apps/quiz-1/js/app.js"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/javascript")
                    .set_body_bytes(b"js".to_vec()),
            )
            .mount(&storage)
            .await;
        let headers = fetch_meta("script", Some("same-origin"));
        let js = serve_at(&state, &storage, "js/app.js", &headers, Some(&desc), &ctx).await;
        assert_eq!(js.status(), StatusCode::OK);
        assert!(js.headers().get("content-security-policy").is_none());
        assert_eq!(body_of(js).await, "js");
    }

    #[tokio::test]
    async fn on_the_name_host_anything_but_a_page_opening_moves_to_the_apps_origin() {
        let storage = storage_with_entry().await;
        let state = state_for(&storage.uri()).await;
        let desc = descriptor("html5-app", "A quiz");
        let mut ctx = paired("elohim.localhost:8888");
        ctx.query = Some("v=2".into());
        for headers in [frame_headers(), HeaderMap::new()] {
            let resp = serve_at(&state, &storage, "index.html", &headers, Some(&desc), &ctx).await;
            assert_eq!(resp.status(), StatusCode::PERMANENT_REDIRECT);
            assert_eq!(
                resp.headers().get("location").unwrap(),
                "http://apps.elohim.localhost:8888/apps/quiz-1/index.html?v=2"
            );
        }
        let sub = serve_at(
            &state,
            &storage,
            "js/app.js",
            &nav_headers(),
            Some(&desc),
            &ctx,
        )
        .await;
        assert_eq!(sub.status(), StatusCode::PERMANENT_REDIRECT);
        let probe =
            moved_to_apps_origin(ctx.edge.as_ref().unwrap(), "/apps/quiz-1/_capability", None);
        assert_eq!(
            probe.headers().get("location").unwrap(),
            "http://apps.elohim.localhost:8888/apps/quiz-1/_capability"
        );
    }

    #[tokio::test]
    async fn on_the_name_host_the_shell_frames_the_boot_page_and_sends_no_framing_restriction() {
        let storage = storage_with_entry().await;
        let state = state_for(&storage.uri()).await;
        let desc = descriptor("html5-app", "A quiz");
        let mut ctx = paired("elohim.localhost:8888");
        ctx.query = Some("level=2".into());
        let resp = serve_at(
            &state,
            &storage,
            "index.html",
            &nav_headers(),
            Some(&desc),
            &ctx,
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get("x-elohim-chrome").unwrap(),
            "content-shell"
        );
        // The shell sends no framing restriction (headers only: that a third
        // party's frame actually renders it is the household lane's proof).
        assert!(resp.headers().get("content-security-policy").is_none());
        assert!(resp.headers().get("x-frame-options").is_none());
        let body = body_of(resp).await;
        let boot = "http://apps.elohim.localhost:8888/_elohim/boot?to=%2Fapps%2Fquiz-1%2Findex.html%3Flevel%3D2";
        let island = island_of(&body);
        assert_eq!(island["content"]["frame"]["src"], boot);
        assert_eq!(
            island["content"]["isolation"],
            serde_json::json!({
                "boundary": "origin-pair",
                "contentOrigin": "http://apps.elohim.localhost:8888",
            })
        );
        assert_eq!(
            island["content"]["delivery"]["path"][0]["name"],
            "elohim.localhost"
        );
        assert!(!body.contains("<iframe"));
    }

    #[test]
    fn the_boot_page_opens_only_a_path_under_apps() {
        let edge = paired("apps.elohim.localhost:8888").edge.unwrap();
        let framed = fetch_meta("iframe", Some("same-site"));
        for bad in [
            None,
            Some(""),
            Some("to=%2Fepr%2Fquiz-1"),
            Some("to=https%3A%2F%2Fevil.example%2Fapps%2Fx"),
            Some("to=%2F%2Fevil.example%2Fapps%2F"),
            Some("to=%2Fapps%2F..%2Fauth%2Flogin"),
            // Encoded dot segments a browser would normalise away.
            Some("to=%2Fapps%2F%252e%252e%2F_elohim%2Fboot"),
            Some("to=%2Fapps%2F%252E%252E%2F_elohim%2Fboot"),
            Some("to=%2Fapps%2Fx%2F%252e%2F..%2F..%2Fauth"),
            Some("to=%2Fapps%2Fx%5Cy"),
        ] {
            let resp = boot_response(bad, &framed, &edge);
            assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "{bad:?}");
        }
        let ok = boot_response(
            Some("to=%2Fapps%2Fquiz-1%2Findex.html%3Flevel%3D2"),
            &framed,
            &edge,
        );
        assert_eq!(ok.status(), StatusCode::OK);
        let h = ok.headers();
        assert_eq!(h.get("cache-control").unwrap(), "no-store");
        let body = futures::executor::block_on(body_of(ok));
        // The app's own query survives into the target.
        assert!(body.contains("var to=\"/apps/quiz-1/index.html?level=2\""));
        assert!(body.contains("register('/apps-sw.js',{scope:'/apps/'})"));
        assert!(body.contains("setTimeout(go,1500)"));
        let sneaky = boot_response(Some("to=%2Fapps%2Fx%3C%2Fscript%3E"), &framed, &edge);
        let body = futures::executor::block_on(body_of(sneaky));
        assert_eq!(body.matches("</script>").count(), 1);
    }

    /// `frame-ancestors` is absent from the boot page and from raw content:
    /// enforced, it checks EVERY ancestor and so refused the bytes whenever a
    /// third party framed the chromed shell. Fetch Metadata is the
    /// enforcement, one case each.
    #[test]
    fn fetch_metadata_not_frame_ancestors_decides_who_gets_the_boot_page() {
        let edge = paired("apps.elohim.localhost:8888").edge.unwrap();
        let to = Some("to=%2Fapps%2Fquiz-1%2Findex.html%3Flevel%3D2");
        let shell = "http://elohim.localhost:8888/_elohim/content/quiz-1/index.html?level=2";
        // A cross-site frame: sent to the chromed shell.
        let cross = boot_response(to, &fetch_meta("iframe", Some("cross-site")), &edge);
        assert_eq!(cross.status(), StatusCode::FOUND);
        assert_eq!(cross.headers().get("location").unwrap(), shell);
        // No Fetch Metadata at all: sent to the shell too.
        let bare = boot_response(to, &HeaderMap::new(), &edge);
        assert_eq!(bare.status(), StatusCode::FOUND);
        assert_eq!(bare.headers().get("location").unwrap(), shell);
        // A page opening: the shell.
        let top = boot_response(to, &nav_headers(), &edge);
        assert_eq!(top.status(), StatusCode::FOUND);
        // A same-site frame (the shell on the paired name): the boot page runs.
        let same = boot_response(to, &fetch_meta("iframe", Some("same-site")), &edge);
        assert_eq!(same.status(), StatusCode::OK);
        for resp in [&cross, &bare, &top, &same] {
            assert!(resp.headers().get("content-security-policy").is_none());
            assert!(resp.headers().get("x-frame-options").is_none());
        }
    }

    #[tokio::test]
    async fn a_facing_refusal_is_the_answer_never_the_descriptor() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};
        let storage = storage_with_entry().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/epr/quiz-1/chrome"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&storage)
            .await;
        let state = state_for(&storage.uri()).await;
        let desc = descriptor("html5-app", "A quiz");
        // The name host's page opening: the reach refusal, no shell.
        let shell = serve(&state, &storage, "index.html", &nav_headers(), Some(&desc)).await;
        assert_eq!(shell.status(), StatusCode::FORBIDDEN);
        assert_eq!(shell.headers().get("x-cache").unwrap(), "REFUSED");
        assert!(shell.headers().get("x-elohim-chrome").is_none());
        // The always-render shell path refuses the same way.
        let always = handle_content_shell(
            state.clone(),
            "/_elohim/content/quiz-1/index.html",
            &AppServeContext::default(),
        )
        .await;
        assert_eq!(always.status(), StatusCode::FORBIDDEN);
        // Raw HTML on the content origin is refused too.
        let ctx = paired("apps.elohim.localhost:8888");
        let raw = serve_at(
            &state,
            &storage,
            "index.html",
            &fetch_meta("iframe", Some("same-site")),
            Some(&desc),
            &ctx,
        )
        .await;
        assert_eq!(raw.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn a_lagging_peer_without_the_record_does_not_demote_a_known_record() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};
        let storage = storage_with_entry().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/epr/quiz-1/chrome"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&storage)
            .await;
        let state = state_for(&storage.uri()).await;
        // (a) 404, but the doorway's descriptor knows the record: trustful,
        // witness per the descriptor's anchor (amber here: none yet).
        let desc = descriptor("html5-app", "A quiz");
        let resp = serve(&state, &storage, "index.html", &nav_headers(), Some(&desc)).await;
        let island = island_of(&body_of(resp).await);
        assert_eq!(island["content"]["mode"], "trustful");
        assert_eq!(island["content"]["witness"], "amber");
        assert_eq!(
            island["content"]["capabilities"],
            serde_json::json!(["forms", "fullscreen"])
        );
        // (b) 404 and no descriptor: neither knows it — the trustless shell.
        let resp = handle_content_shell(
            state.clone(),
            "/_elohim/content/quiz-1/index.html",
            &AppServeContext::default(),
        )
        .await;
        let island = island_of(&body_of(resp).await);
        assert_eq!(island["content"]["mode"], "trustless");
        assert_eq!(island["content"]["capabilities"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn an_oversized_facing_is_abandoned_and_the_descriptor_fills_in() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};
        let storage = storage_with_entry().await;
        let mut huge = facing_json("quiz-1");
        huge["title"] = serde_json::Value::String("x".repeat(CHROME_FACING_MAX_BYTES + 1));
        Mock::given(method("GET"))
            .and(path("/api/v1/epr/quiz-1/chrome"))
            .respond_with(ResponseTemplate::new(200).set_body_json(huge))
            .mount(&storage)
            .await;
        let state = state_for(&storage.uri()).await;
        assert_eq!(
            fetch_chrome_facing(&state, "quiz-1").await,
            FacingRead::Unavailable
        );
        let desc = descriptor("html5-app", "A quiz");
        let resp = serve(&state, &storage, "index.html", &nav_headers(), Some(&desc)).await;
        let island = island_of(&body_of(resp).await);
        assert_eq!(island["title"], "A quiz");
        assert_eq!(island["content"]["mode"], "trustful");
    }

    #[test]
    fn the_facing_cache_is_bounded_and_evicts_expired_entries_first() {
        let now = std::time::Instant::now();
        let long = std::time::Duration::from_secs(60);
        let expired = std::time::Duration::from_secs(0);
        let mut cache = FacingCache::new();
        cache_insert_bounded(
            &mut cache,
            "a".into(),
            (now, expired, FacingRead::NoRecord),
            2,
        );
        cache_insert_bounded(&mut cache, "b".into(), (now, long, FacingRead::NoRecord), 2);
        // Full: the expired entry goes first.
        cache_insert_bounded(&mut cache, "c".into(), (now, long, FacingRead::NoRecord), 2);
        assert_eq!(cache.len(), 2);
        assert!(!cache.contains_key("a"));
        // Still full with live entries: the oldest makes room.
        let later = now + std::time::Duration::from_millis(5);
        cache_insert_bounded(
            &mut cache,
            "d".into(),
            (later, long, FacingRead::NoRecord),
            2,
        );
        assert_eq!(cache.len(), 2);
        assert!(cache.contains_key("d"));
    }
}
