//! SSR-specific glue between elohim-render and doorway's resolver.
//!
//! ## ResolverFetcher
//!
//! `ResolverFetcher` implements `elohim_render::DataFetcher` by forwarding fetch
//! requests to the configured elohim-storage endpoint via `reqwest`. This lets
//! Angular SSR components call `fetch()` against the local storage sidecar during
//! server-side rendering without needing network access to the public doorway URL.
//!
//! The backing method is a direct `reqwest::Client` GET against `storage_base_url`
//! because `DoorwayResolver::resolve(content_type, id)` is scoped to
//! projection-cache / conductor lookups — it has no concept of arbitrary HTTP paths.
//!
//! ## Render-result cache
//!
//! `render_cache_key` produces a stable key for an SSR render result.
//!
//! MVP cache semantics: key is `(url, spec_version)` only. `fetched_inputs` is
//! recorded in `RenderOutput` for audit but not factored into the lookup key.
//! Invalidation is TTL-based (5-minute default). Hash-aware invalidation (keyed on
//! fetched-content hashes) is deferred until a DHT signal subscriber drives
//! evictions — at that point callers pass the real hash list rather than `&[]`.

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::time::{Duration, Instant};

use elohim_render::{DataFetcher, FetchRequest, FetchResponse, FetcherTrust, RenderError, Result};

use crate::cache::ContentCache;

// =============================================================================
// Cache key
// =============================================================================

/// Build a stable cache key for an SSR render result.
///
/// Inputs:
/// - `url` — the requested URL (path + query)
/// - `fetched_hashes` — content hashes of every resource fetched during
///   rendering; pass `&[]` for MVP TTL-only invalidation
/// - `spec_version` — version tag for the renderer behaviour; bump when
///   the renderer's output changes (e.g. new shim added) to invalidate
///   all cached renders without touching content
///
/// MVP trade-off: the lookup key uses `&[]` so the first request is always MISS
/// and subsequent requests (within TTL) hit. Hash-aware invalidation requires
/// a DHT signal subscriber, which is a follow-up task.
pub fn render_cache_key(url: &str, fetched_hashes: &[String], spec_version: &str) -> String {
    let mut h = Sha256::new();
    h.update(url.as_bytes());
    h.update(b"\0");
    for hash in fetched_hashes {
        h.update(hash.as_bytes());
        h.update(b"\0");
    }
    h.update(spec_version.as_bytes());
    let bytes = h.finalize();
    format!("ssr-{}", hex::encode(&bytes[..16]))
}

// =============================================================================
// ResolverFetcher
// =============================================================================

/// Opaque user credential the V8 fetch shim attaches to outbound storage
/// fetches. Constructed by doorway's session layer; the shim doesn't decode
/// or interpret it. Allows framework-agnostic auth threading: any V8-based
/// renderer can pass through whatever credential shape doorway's session
/// produced (JWT, steward attestation, opaque session cookie, etc.).
#[derive(Debug, Clone)]
pub struct UserCredential {
    pub header_name: String,
    pub header_value: String,
}

/// Forwards Angular SSR `fetch()` calls to the local elohim-storage sidecar.
///
/// Constructed with the shared pooled storage client (from
/// `AppState::storage_proxy_client` — the ONE client per storage upstream; a
/// second SSR-only client to the same upstream fed the shared breaker failure
/// evidence the proxy path then paid for, the 2026-08-13 `/` shed class) and
/// the storage base URL (e.g. `http://localhost:8090`). Any `fetch()` call
/// from the Angular bundle is translated to a `reqwest::Client` request against
/// that base, stripping any scheme/host from the incoming URL first (the bundle's
/// fetch calls use relative paths or doorway-originating absolute URLs).
///
/// The client is injected rather than built per-request so the connection pool is
/// shared across all SSR renders on this doorway instance.
///
/// When `user_credential` is `Some`, the shim attaches it as an HTTP header
/// to every outbound storage fetch, so authenticated SSR renders see the
/// originating user's auth context (commons + reach-aware content).
pub struct ResolverFetcher {
    storage_base: String,
    client: Arc<reqwest::Client>,
    user_credential: Option<UserCredential>,
}

impl ResolverFetcher {
    /// Create a new `ResolverFetcher` using a shared HTTP client.
    ///
    /// `client` should be the doorway-level shared client (from `AppState`).
    /// `storage_base_url` should be the root of the elohim-storage HTTP API,
    /// e.g. `http://localhost:8090` (no trailing slash).
    pub fn new(client: Arc<reqwest::Client>, storage_base_url: String) -> Self {
        Self {
            storage_base: storage_base_url.trim_end_matches('/').to_string(),
            client,
            user_credential: None,
        }
    }

    /// Builder: attach a per-render user credential. The shim will add this
    /// as a header on every outbound storage fetch.
    pub fn with_user_credential(mut self, credential: UserCredential) -> Self {
        self.user_credential = Some(credential);
        self
    }

    /// Convenience: apply credential only if `Some`.
    pub fn maybe_with_user_credential(self, cred: Option<UserCredential>) -> Self {
        match cred {
            Some(c) => self.with_user_credential(c),
            None => self,
        }
    }
}

#[async_trait]
impl DataFetcher for ResolverFetcher {
    /// A `ResolverFetcher` carrying a user credential fetches under THAT user's
    /// authority — reach-aware storage reads answer differently for it than for
    /// the doorway's own identity — so it is `Principal`. Without a credential
    /// it fetches as the doorway itself: `Ambient`.
    ///
    /// This is the declaration the render isolate's trust bookkeeping reads.
    /// `elohim_render::JsRuntime` reuses one V8 isolate across sequential
    /// renders and does NOT reset the JS heap between them, so declaring
    /// `Principal` here is what makes the resulting cross-render residue
    /// visible instead of silent. See the type-level docs on
    /// `elohim_render::runtime::JsRuntime` and
    /// `genesis/data/timeline/backlog/elohim-render-isolate-reuse-trust-boundary.md`.
    fn trust_scope(&self) -> FetcherTrust {
        match self.user_credential {
            Some(_) => FetcherTrust::Principal,
            None => FetcherTrust::Ambient,
        }
    }

    /// The fetch runs under a [`WatchedFetch`]: if the render's soft budget cuts
    /// it (the `TracingFetcher` timeout drops this future), one `warn!` on target
    /// `doorway::ssr::fetch` names the phase it reached and whether it was woken
    /// and left unpolled. A completed fetch logs nothing.
    async fn fetch(&self, request: FetchRequest) -> Result<FetchResponse> {
        let probe = FetchProbe::new(strip_to_path(&request.url));
        let phase = probe.phase_handle();
        WatchedFetch::new(Box::pin(self.send_and_read(request, phase)), probe).await
    }
}

impl ResolverFetcher {
    /// The fetch itself: one request to storage, response head, then body.
    /// `phase` is advanced as each step is reached so an abandonment can say
    /// how far it got.
    async fn send_and_read(
        &self,
        request: FetchRequest,
        phase: Arc<AtomicU8>,
    ) -> Result<FetchResponse> {
        let path = strip_to_path(&request.url);
        let url = format!("{}{}", self.storage_base, path);

        let method = request.method.to_ascii_uppercase();
        let req_builder = match method.as_str() {
            "POST" => self.client.post(&url),
            "PUT" => self.client.put(&url),
            "DELETE" => self.client.delete(&url),
            "PATCH" => self.client.patch(&url),
            _ => self.client.get(&url),
        };

        // Forward request headers
        let mut req_builder = req_builder;
        for (k, v) in &request.headers {
            req_builder = req_builder.header(k.as_str(), v.as_str());
        }

        // Attach the originating user's credential if the shim was given one.
        // The header value is opaque to the shim — doorway's session layer
        // constructed it from the live request's auth context.
        if let Some(cred) = &self.user_credential {
            req_builder = req_builder.header(&cred.header_name, &cred.header_value);
        }

        // Forward body if present
        if let Some(body) = request.body {
            req_builder = req_builder.body(body);
        }

        let resp = req_builder
            .send()
            .await
            .map_err(|e| RenderError::DataFetch(format!("storage fetch error: {e}")))?;
        phase.store(FetchPhase::Body as u8, Ordering::Relaxed);

        let status = resp.status().as_u16();

        // Collect response headers into a flat HashMap (last value wins on dupe keys)
        let mut headers: HashMap<String, String> = HashMap::new();
        for (name, value) in resp.headers() {
            if let Ok(v) = value.to_str() {
                headers.insert(name.to_string(), v.to_string());
            }
        }

        let body = resp
            .bytes()
            .await
            .map_err(|e| RenderError::DataFetch(format!("storage read body: {e}")))?
            .to_vec();

        Ok(FetchResponse {
            status,
            headers,
            body,
            content_hash: None,
        })
    }
}

// =============================================================================
// Test-only fault injection (env-gated)
// =============================================================================

/// Wrap `inner` with stall fault injection when `DOORWAY_SSR_FAULT_STALL_PATH`
/// is set (off by default — zero production impact). Any outbound SSR fetch whose
/// URL contains the configured substring then never settles, exercising the
/// render-trace `stalled` terminal + fast soft-deadline fallback end-to-end.
///
/// The match is on the *fetch* URL (which carries the content slug), because the
/// renderer's fetcher is fixed at construction and sees every render's fetches —
/// so a single shared fetcher can fault just the configured route. Used by the
/// `render-trace-distinguishes-stall-from-empty` a2o scenario.
pub fn maybe_inject_stall_fault(inner: Arc<dyn DataFetcher>) -> Arc<dyn DataFetcher> {
    match std::env::var("DOORWAY_SSR_FAULT_STALL_PATH") {
        Ok(substr) if !substr.is_empty() => {
            tracing::warn!(
                target: "doorway::ssr",
                fault = %substr,
                "SSR stall fault injection ENABLED (DOORWAY_SSR_FAULT_STALL_PATH — test-only)"
            );
            Arc::new(StallFaultFetcher {
                inner,
                fault_substr: substr,
            })
        }
        _ => inner,
    }
}

// =============================================================================
// Trust-scoped SSR render cache
// =============================================================================

// The shareability RULE is not defined here: it lives in the shared render
// layer as `FetcherTrust::is_cache_shareable`, because every render host must
// make the same decision (doorway is one optional web2 projection of a peer
// runtime capability, never a required component of the render path). These two
// functions are only the doorway-local cache ADAPTER — they bind that shared
// predicate to `ContentCache`. When render serving consolidates into the shared
// render-host layer (see the isolate-reuse backlog record's consolidation note),
// these move; the predicate does not.

/// Trust-scoped read of the SSR render cache.
///
/// Returns `None` for a `Principal` render even on a populated key, so an
/// authenticated visitor is never served another principal's (or a stale
/// anonymous) render of reach-gated content.
pub fn cached_render_for_trust(
    cache: &ContentCache,
    key: &str,
    trust: FetcherTrust,
) -> Option<String> {
    if !trust.is_cache_shareable() {
        return None;
    }
    cache.get_rendered(key)
}

/// Trust-scoped write to the SSR render cache.
///
/// A `Principal` render is silently not cached — this is the guard that closes
/// the credential-blind-key disclosure: `render_cache_key` carries no principal,
/// so storing a credentialed render would serve it to everyone for the TTL.
pub fn cache_render_for_trust(
    cache: &ContentCache,
    key: &str,
    html: String,
    ttl: std::time::Duration,
    trust: FetcherTrust,
) {
    if !trust.is_cache_shareable() {
        tracing::debug!(
            target: "doorway::ssr",
            "credentialed render not cached (trust-scoped SSR cache)"
        );
        return;
    }
    cache.put_rendered(key, html, ttl);
}

/// Decorator that stalls fetches whose URL contains `fault_substr`, delegating
/// everything else to `inner`. See [`maybe_inject_stall_fault`].
struct StallFaultFetcher {
    inner: Arc<dyn DataFetcher>,
    fault_substr: String,
}

#[async_trait]
impl DataFetcher for StallFaultFetcher {
    /// Delegates — a decorator must never answer for itself. This one wraps the
    /// real `ResolverFetcher`, so reporting its own scope would launder a
    /// credentialed fetcher into an ambient one.
    fn trust_scope(&self) -> FetcherTrust {
        self.inner.trust_scope()
    }

    async fn fetch(&self, request: FetchRequest) -> Result<FetchResponse> {
        if request.url.contains(self.fault_substr.as_str()) {
            // Never settle — the TracingFetcher's per-fetch soft-deadline converts
            // this into a recorded `stalled` and a fast fallback.
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            return Err(RenderError::DataFetch("ssr stall fault (test-only)".into()));
        }
        self.inner.fetch(request).await
    }
}

// =============================================================================
// Helpers
// =============================================================================

/// Strip scheme and host from a URL, returning just the path (and query).
///
/// `https://example.com/api/v1/foo?bar=1` → `/api/v1/foo?bar=1`
/// `/api/v1/foo` → `/api/v1/foo`
fn strip_to_path(url: &str) -> &str {
    if let Some(idx) = url.find("://") {
        let after_scheme = &url[idx + 3..];
        if let Some(slash_idx) = after_scheme.find('/') {
            return &after_scheme[slash_idx..];
        }
        return "/";
    }
    url
}

// =============================================================================
// TimedDnsResolver — names a slow name lookup on the SSR render client
// =============================================================================

/// A name lookup on the SSR render client slower than this is logged. It sits
/// well under the render's per-fetch soft budget
/// (`elohim_render::traced_fetcher::DEFAULT_SOFT_BUDGET_MS`, 1200 ms), so a
/// lookup that is on its way to stalling a fetch is named before the cut.
pub const SSR_DNS_SLOW_LOOKUP_MS: u128 = 250;

/// Whether a lookup that took `elapsed_ms` is slow enough to log.
fn ssr_dns_lookup_is_slow(elapsed_ms: u128) -> bool {
    elapsed_ms >= SSR_DNS_SLOW_LOOKUP_MS
}

/// How long a resolved address set is served without asking again. Matches
/// CoreDNS's default record TTL for cluster services (`ttl 30`).
pub const SSR_DNS_FRESH_TTL: Duration = Duration::from_secs(30);

/// How long past freshness a last-good address set may still be served when
/// the refresh does not answer in time (serve-stale, RFC 8767 §5, which caps
/// stale data at one to three days). The render upstream is a cluster Service
/// whose address lives as long as the Service does.
pub const SSR_DNS_STALE_CAP: Duration = Duration::from_secs(24 * 60 * 60);

/// How long a resolve with a stale answer in hand waits for the refresh before
/// serving the stale answer. A healthy lookup answers well inside it; a hung
/// one costs the fetch this much instead of the whole soft budget.
pub const SSR_DNS_STALE_GRACE: Duration = Duration::from_millis(150);

/// The SSR render client's DNS resolver: the system resolver, timed, shared
/// across concurrent fetches, and remembered.
///
/// The render client is pool-free on purpose (`init_ssr_render_client`), so
/// every render fetch opens a fresh connection. Before 2026-10-23 every fetch
/// also performed its own name lookup: a `/` render issued about 29
/// `getaddrinfo` calls for the same host in the same few milliseconds, and on
/// alpha a few of them, late in that burst, did not answer within the 1200 ms
/// soft budget. The drop guards below measured it: 35 of 35 fetches cut on
/// doorway-alpha (2026-10-23, image `04ef84c`) were cut in `sending`, never
/// woken-and-unpolled, each beside a "name lookup abandoned" line
/// (`genesis/data/timeline/backlog/self-heal-alpha-ssr-post-bundle-swap-cold-fetch-stall.md`).
///
/// So resolution goes through an [`SsrAddressBook`]:
///
/// - **single-flight** — concurrent resolves of one host share one lookup;
/// - **fresh hits** — an answer younger than [`SSR_DNS_FRESH_TTL`] is served
///   without a lookup;
/// - **serve-stale** — an older last-good answer (up to [`SSR_DNS_STALE_CAP`])
///   starts or joins a refresh and is served if the refresh has not answered
///   within [`SSR_DNS_STALE_GRACE`];
/// - **detached lookups** — each lookup runs on its own thread and writes its
///   answer into the book, so a fetch cut at the soft budget no longer throws
///   the lookup away: the next render finds the answer.
///
/// Connections stay unpooled: an address list is plain data, owned by no
/// runtime, so sharing it does not reintroduce the cross-runtime hazard the
/// pool-free client exists to avoid.
///
/// The instrument is unchanged: a slow or failed lookup is a `warn!` on target
/// `doorway::ssr::dns` carrying the host and the elapsed time (now once per
/// lookup, not once per fetch), and a fetch that waits on a lookup with no
/// answer in hand and is cut logs "name lookup abandoned before it answered"
/// from [`LookupWatch`].
#[derive(Debug, Default, Clone)]
pub struct TimedDnsResolver {
    book: SsrAddressBook,
}

impl reqwest::dns::Resolve for TimedDnsResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let book = self.book.clone();
        Box::pin(async move {
            let host = name.as_str().to_string();
            match book.resolve(&host).await {
                Ok(addrs) => Ok(Box::new(addrs.into_iter()) as reqwest::dns::Addrs),
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
            }
        })
    }
}

/// A blocking name lookup: host to socket addresses (port 0).
pub type SsrLookupFn =
    Arc<dyn Fn(&str) -> std::io::Result<Vec<std::net::SocketAddr>> + Send + Sync>;

/// The system resolver (`getaddrinfo`), the same one reqwest's default
/// resolver and `tokio::net::lookup_host` run.
fn system_lookup(host: &str) -> std::io::Result<Vec<std::net::SocketAddr>> {
    use std::net::ToSocketAddrs;
    Ok((host, 0).to_socket_addrs()?.collect())
}

/// A lookup's answer as every waiter on it sees it.
type LookupAnswer = std::result::Result<Vec<std::net::SocketAddr>, (std::io::ErrorKind, String)>;

/// One in-flight lookup: waiters hold a receiver; the lookup thread sends.
type InFlight = tokio::sync::watch::Receiver<Option<LookupAnswer>>;

#[derive(Default)]
struct HostEntry {
    /// The last good answer and when it arrived.
    last_good: Option<(Vec<std::net::SocketAddr>, Instant)>,
    in_flight: Option<InFlight>,
}

/// The SSR render client's address book: single-flight lookups, a short fresh
/// window, and serve-stale under a hung refresh. See [`TimedDnsResolver`].
#[derive(Clone)]
pub struct SsrAddressBook {
    hosts: Arc<std::sync::Mutex<HashMap<String, HostEntry>>>,
    lookup: SsrLookupFn,
    fresh_ttl: Duration,
    stale_cap: Duration,
    stale_grace: Duration,
}

impl Default for SsrAddressBook {
    fn default() -> Self {
        Self::new(
            Arc::new(system_lookup),
            SSR_DNS_FRESH_TTL,
            SSR_DNS_STALE_CAP,
            SSR_DNS_STALE_GRACE,
        )
    }
}

impl std::fmt::Debug for SsrAddressBook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SsrAddressBook")
            .field("fresh_ttl", &self.fresh_ttl)
            .field("stale_cap", &self.stale_cap)
            .field("stale_grace", &self.stale_grace)
            .finish_non_exhaustive()
    }
}

/// What a resolve found in the book.
enum BookRead {
    Fresh(Vec<std::net::SocketAddr>),
    Stale(Vec<std::net::SocketAddr>, InFlight),
    Empty(InFlight),
}

impl SsrAddressBook {
    pub fn new(
        lookup: SsrLookupFn,
        fresh_ttl: Duration,
        stale_cap: Duration,
        stale_grace: Duration,
    ) -> Self {
        Self {
            hosts: Arc::default(),
            lookup,
            fresh_ttl,
            stale_cap,
            stale_grace,
        }
    }

    /// Resolve `host`: a fresh answer at once, else join or start the one
    /// lookup for it (serving a stale answer if that lookup is slow).
    pub async fn resolve(&self, host: &str) -> std::io::Result<Vec<std::net::SocketAddr>> {
        let started = Instant::now();
        match self.read(host) {
            BookRead::Fresh(addrs) => Ok(addrs),
            BookRead::Stale(stale, rx) => {
                match tokio::time::timeout(self.stale_grace, wait_answer(rx)).await {
                    Ok(Ok(addrs)) => Ok(addrs),
                    Ok(Err(e)) => {
                        tracing::debug!(
                            target: "doorway::ssr::dns",
                            host = %host,
                            error = %e,
                            "SSR render fetch: refresh failed; serving the last good address"
                        );
                        Ok(stale)
                    }
                    Err(_) => {
                        tracing::debug!(
                            target: "doorway::ssr::dns",
                            host = %host,
                            grace_ms = self.stale_grace.as_millis() as u64,
                            "SSR render fetch: refresh slower than the grace; serving the last good address"
                        );
                        Ok(stale)
                    }
                }
            }
            BookRead::Empty(rx) => {
                // Logs from `Drop` if no answer arrives before the render's soft
                // budget drops the fetch. The lookup itself carries on and
                // writes the book.
                let mut watch = LookupWatch::new(host, started);
                let answer = wait_answer(rx).await;
                watch.answered();
                answer
            }
        }
    }

    /// Read the book under its lock, starting a lookup when one is needed and
    /// none is running.
    fn read(&self, host: &str) -> BookRead {
        let mut hosts = self
            .hosts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let entry = hosts.entry(host.to_string()).or_default();
        let now = Instant::now();
        let usable = match &entry.last_good {
            Some((addrs, at)) => {
                let age = now.saturating_duration_since(*at);
                if age < self.fresh_ttl {
                    return BookRead::Fresh(addrs.clone());
                }
                (age < self.fresh_ttl + self.stale_cap).then(|| addrs.clone())
            }
            None => None,
        };
        // A lookup whose thread died without answering is not in flight.
        let joinable = entry
            .in_flight
            .as_ref()
            .filter(|rx| rx.has_changed().is_ok())
            .cloned();
        let rx = match joinable {
            Some(rx) => rx,
            None => {
                let rx = self.start_lookup(host);
                entry.in_flight = Some(rx.clone());
                rx
            }
        };
        match usable {
            Some(addrs) => BookRead::Stale(addrs, rx),
            None => BookRead::Empty(rx),
        }
    }

    #[cfg(test)]
    fn read_last_good(&self, host: &str) -> Option<Vec<std::net::SocketAddr>> {
        let hosts = self
            .hosts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        hosts
            .get(host)
            .and_then(|e| e.last_good.as_ref().map(|(a, _)| a.clone()))
    }

    /// Run one lookup on its own thread. It outlives every waiter: a waiter cut
    /// by the soft budget drops only its receiver, and the answer still lands
    /// in the book.
    fn start_lookup(&self, host: &str) -> InFlight {
        let (tx, rx) = tokio::sync::watch::channel(None);
        let host = host.to_string();
        let hosts = Arc::clone(&self.hosts);
        let lookup = Arc::clone(&self.lookup);
        let spawned = std::thread::Builder::new()
            .name("ssr-dns".into())
            .spawn(move || {
                let started = Instant::now();
                let looked_up = lookup(&host);
                let elapsed_ms = started.elapsed().as_millis();
                let answer: LookupAnswer = match looked_up {
                    Ok(addrs) => {
                        if ssr_dns_lookup_is_slow(elapsed_ms) {
                            tracing::warn!(
                                target: "doorway::ssr::dns",
                                host = %host,
                                elapsed_ms = elapsed_ms as u64,
                                addresses = addrs.len(),
                                "SSR render fetch: slow name lookup"
                            );
                        }
                        Ok(addrs)
                    }
                    Err(e) => {
                        tracing::warn!(
                            target: "doorway::ssr::dns",
                            host = %host,
                            elapsed_ms = elapsed_ms as u64,
                            error = %e,
                            "SSR render fetch: name lookup failed"
                        );
                        Err((e.kind(), e.to_string()))
                    }
                };
                {
                    let mut hosts = hosts
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    let entry = hosts.entry(host).or_default();
                    // A failure or an empty answer keeps the last good one.
                    if let Ok(addrs) = &answer {
                        if !addrs.is_empty() {
                            entry.last_good = Some((addrs.clone(), Instant::now()));
                        }
                    }
                    entry.in_flight = None;
                }
                // No receiver left is fine: the book already holds the answer.
                let _ = tx.send(Some(answer));
            });
        if let Err(e) = spawned {
            tracing::warn!(
                target: "doorway::ssr::dns",
                error = %e,
                "SSR render fetch: could not start a name lookup thread"
            );
        }
        rx
    }
}

/// Wait for a lookup's answer. A lookup that ended without answering (its
/// thread could not start or panicked) reads as an error.
async fn wait_answer(mut rx: InFlight) -> std::io::Result<Vec<std::net::SocketAddr>> {
    let answer = rx
        .wait_for(Option::is_some)
        .await
        .map_err(|_| {
            std::io::Error::other("SSR render fetch: name lookup ended without an answer")
        })?
        .clone();
    match answer {
        Some(Ok(addrs)) => Ok(addrs),
        Some(Err((kind, msg))) => Err(std::io::Error::new(kind, msg)),
        None => Err(std::io::Error::other(
            "SSR render fetch: name lookup ended without an answer",
        )),
    }
}

/// Names a name lookup that was dropped before it answered. Disarmed by
/// [`LookupWatch::answered`]; still armed at `Drop` means the lookup future was
/// abandoned, which on the render path is the soft-budget cut.
struct LookupWatch {
    host: String,
    started: Instant,
    armed: bool,
}

impl LookupWatch {
    fn new(host: &str, started: Instant) -> Self {
        Self {
            host: host.to_string(),
            started,
            armed: true,
        }
    }

    fn answered(&mut self) {
        self.armed = false;
    }
}

impl Drop for LookupWatch {
    fn drop(&mut self) {
        if self.armed {
            tracing::warn!(
                target: "doorway::ssr::dns",
                host = %self.host,
                elapsed_ms = self.started.elapsed().as_millis() as u64,
                "SSR render fetch: name lookup abandoned before it answered"
            );
        }
    }
}

// =============================================================================
// WatchedFetch — names the phase a render fetch was cut in
// =============================================================================
//
// alpha's `/` renders stall at the 1200 ms soft budget on about one cold render
// in three while storage's histogram shows no request slower than 1 s
// (`genesis/data/timeline/backlog/self-heal-alpha-ssr-post-bundle-swap-cold-fetch-stall.md`).
// The candidate this watch was built to test is the render thread not polling
// its own fetches: the render runs on a `current_thread` runtime that also runs
// the V8 isolate, so a fetch woken by its socket waits until the isolate yields.
// A gap between polls alone cannot separate that from honest waiting on the
// network (a future with no wakeup is not polled either), so the watch also
// wraps the waker: it records when the fetch was woken and how long each wake
// waited for its poll. A wake that was never polled before the cut is the direct
// reading of starvation.
//
// Its first reading (doorway-alpha, 2026-10-23) ruled starvation out: every cut
// fetch was in `sending`, with no wake pending and `max_wake_to_poll_ms` 0, beside
// a "name lookup abandoned" line. The time went to the name lookup, which
// `SsrAddressBook` now shares and remembers.

/// How far a render fetch got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FetchPhase {
    /// Lookup, connect, request write, waiting for the response head.
    Sending = 0,
    /// Response head received; reading the body.
    Body = 1,
}

impl FetchPhase {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => FetchPhase::Body,
            _ => FetchPhase::Sending,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            FetchPhase::Sending => "sending",
            FetchPhase::Body => "body",
        }
    }
}

/// When the watched future was first woken since its last poll, as
/// microseconds after `origin` plus one (0 = no pending wake).
struct WakeLedger {
    origin: Instant,
    pending_wake_us: AtomicU64,
}

impl WakeLedger {
    fn now_us(&self) -> u64 {
        u64::try_from(self.origin.elapsed().as_micros()).unwrap_or(u64::MAX - 1)
    }

    fn mark_woken(&self) {
        let stamp = self.now_us().saturating_add(1);
        // Keep the EARLIEST wake since the last poll: that is the one whose wait
        // measures how long the thread took to come back.
        let _ =
            self.pending_wake_us
                .compare_exchange(0, stamp, Ordering::AcqRel, Ordering::Relaxed);
    }

    /// Microseconds the pending wake (if any) has waited as of now.
    fn take_pending_wait_us(&self) -> Option<u64> {
        let stamp = self.pending_wake_us.swap(0, Ordering::AcqRel);
        (stamp != 0).then(|| self.now_us().saturating_sub(stamp - 1))
    }

    fn peek_pending_wait_us(&self) -> Option<u64> {
        let stamp = self.pending_wake_us.load(Ordering::Acquire);
        (stamp != 0).then(|| self.now_us().saturating_sub(stamp - 1))
    }
}

/// The waker handed to the watched future: stamps the ledger, then wakes the
/// task the runtime gave us.
struct RecordingWaker {
    inner: Waker,
    ledger: Arc<WakeLedger>,
}

impl Wake for RecordingWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.ledger.mark_woken();
        self.inner.wake_by_ref();
    }
}

/// What a fetch looked like when it was abandoned. Logged once, from `Drop`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbandonedFetch {
    pub path: String,
    pub phase: FetchPhase,
    pub elapsed_ms: u64,
    pub polls: u32,
    /// Longest stretch between two polls, counting the last poll to the drop.
    pub max_poll_gap_ms: u64,
    /// Longest a wake waited for the poll it asked for.
    pub max_wake_to_poll_ms: u64,
    /// The fetch was woken and dropped before that wake was polled: how long
    /// the wake had been waiting. `Some` here is the render thread not polling.
    pub woken_unpolled_ms: Option<u64>,
}

/// The per-fetch state a [`WatchedFetch`] reports from.
pub struct FetchProbe {
    path: String,
    started: Instant,
    phase: Arc<AtomicU8>,
    ledger: Arc<WakeLedger>,
    last_poll: Option<Instant>,
    polls: u32,
    max_poll_gap: Duration,
    max_wake_to_poll_us: u64,
    done: bool,
}

impl FetchProbe {
    pub fn new(path: &str) -> Self {
        let started = Instant::now();
        Self {
            path: path.to_string(),
            started,
            phase: Arc::new(AtomicU8::new(FetchPhase::Sending as u8)),
            ledger: Arc::new(WakeLedger {
                origin: started,
                pending_wake_us: AtomicU64::new(0),
            }),
            last_poll: None,
            polls: 0,
            max_poll_gap: Duration::ZERO,
            max_wake_to_poll_us: 0,
            done: false,
        }
    }

    /// The handle the fetch body advances as it reaches each phase.
    pub fn phase_handle(&self) -> Arc<AtomicU8> {
        Arc::clone(&self.phase)
    }

    fn on_poll(&mut self) {
        let now = Instant::now();
        if let Some(last) = self.last_poll {
            self.max_poll_gap = self.max_poll_gap.max(now - last);
        }
        self.last_poll = Some(now);
        self.polls = self.polls.saturating_add(1);
        if let Some(waited) = self.ledger.take_pending_wait_us() {
            self.max_wake_to_poll_us = self.max_wake_to_poll_us.max(waited);
        }
    }

    /// `None` once the fetch completed; otherwise the reading at this instant.
    pub fn abandonment(&self) -> Option<AbandonedFetch> {
        if self.done {
            return None;
        }
        let gap_to_now = self.last_poll.map(|t| t.elapsed()).unwrap_or_default();
        Some(AbandonedFetch {
            path: self.path.clone(),
            phase: FetchPhase::from_u8(self.phase.load(Ordering::Relaxed)),
            elapsed_ms: self.started.elapsed().as_millis() as u64,
            polls: self.polls,
            max_poll_gap_ms: self.max_poll_gap.max(gap_to_now).as_millis() as u64,
            max_wake_to_poll_ms: self.max_wake_to_poll_us / 1_000,
            woken_unpolled_ms: self.ledger.peek_pending_wait_us().map(|us| us / 1_000),
        })
    }
}

/// Polls `inner` through a [`RecordingWaker`] and reports from `Drop` if it was
/// abandoned before it completed.
pub struct WatchedFetch<F: Future> {
    inner: Pin<Box<F>>,
    probe: FetchProbe,
    /// (the runtime's waker, the recording waker wrapping it), rebuilt only
    /// when the runtime hands over a different task waker.
    waker: Option<(Waker, Waker)>,
}

impl<F: Future> WatchedFetch<F> {
    pub fn new(inner: Pin<Box<F>>, probe: FetchProbe) -> Self {
        Self {
            inner,
            probe,
            waker: None,
        }
    }

    #[cfg(test)]
    fn probe(&self) -> &FetchProbe {
        &self.probe
    }
}

impl<F: Future> Future for WatchedFetch<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
        // Every field is Unpin (`inner` is boxed), so the wrapper is too.
        let this = self.get_mut();
        this.probe.on_poll();
        let recording = match &this.waker {
            Some((runtime, recording)) if runtime.will_wake(cx.waker()) => recording.clone(),
            _ => {
                let recording = Waker::from(Arc::new(RecordingWaker {
                    inner: cx.waker().clone(),
                    ledger: Arc::clone(&this.probe.ledger),
                }));
                this.waker = Some((cx.waker().clone(), recording.clone()));
                recording
            }
        };
        let mut watched_cx = Context::from_waker(&recording);
        let out = this.inner.as_mut().poll(&mut watched_cx);
        if out.is_ready() {
            this.probe.done = true;
        }
        out
    }
}

impl<F: Future> Drop for WatchedFetch<F> {
    fn drop(&mut self) {
        if let Some(a) = self.probe.abandonment() {
            tracing::warn!(
                target: "doorway::ssr::fetch",
                path = %a.path,
                phase = a.phase.as_str(),
                elapsed_ms = a.elapsed_ms,
                polls = a.polls,
                max_poll_gap_ms = a.max_poll_gap_ms,
                max_wake_to_poll_ms = a.max_wake_to_poll_ms,
                woken_unpolled_ms = ?a.woken_unpolled_ms,
                "SSR render fetch abandoned before it answered"
            );
        }
    }
}

// =============================================================================
// DoorwayBundleSource — blocking BundleSource over the storage HTTP API
// =============================================================================

/// Implements `elohim_render::BundleSource` by hitting the storage HTTP API
/// synchronously (blocking reqwest).
///
/// ## Async-caller safety
///
/// `init_renderer()` is called from FOUR `AppState` constructors. Three are plain
/// sync fns — fine. The fourth, `AppState::with_projection`, is an **async fn**.
/// `reqwest::blocking` internally creates a new tokio runtime and calls
/// `block_on`; doing that while already inside a tokio worker thread panics with
/// "Cannot start a runtime from within a runtime."
///
/// To handle both callers safely each blocking HTTP call is dispatched to a
/// **dedicated OS thread** via `std::thread::spawn(...).join()`. The spawned
/// thread has no ambient tokio context, so `reqwest::blocking` can create its
/// own runtime freely. The brief join is acceptable for a one-shot boot
/// materialize (this runs at most once per doorway startup, not per request).
pub struct DoorwayBundleSource {
    storage_base: String,
    /// Per-request ceiling for the `/db/content/{slug}` metadata read.
    head_timeout: Duration,
    /// Per-request ceiling for the `/blob/{hash}` byte read (a server bundle is
    /// megabytes, so it legitimately needs a longer window than the head read).
    blob_timeout: Duration,
    /// Optional WHOLE-PASS deadline. Every request clamps its own timeout to the
    /// remaining budget, and once the deadline has passed a request fails
    /// immediately WITHOUT touching the network. `None` = unbounded pass (the
    /// reconcile/adoption path, which runs off the boot critical path).
    deadline: Option<Instant>,
}

/// Default `/db/content/{slug}` metadata-read ceiling for a BOOT materialize.
/// A healthy peer answers this in milliseconds; the value only has to cover a
/// slow-but-alive upstream, because a stalled one must not hold the listener.
pub const DEFAULT_BOOT_HEAD_TIMEOUT_SECS: u64 = 5;

/// Default `/blob/{hash}` byte-read ceiling (megabytes off a local peer).
pub const DEFAULT_BLOB_TIMEOUT_SECS: u64 = 30;

impl DoorwayBundleSource {
    pub fn new(storage_base_url: String) -> Self {
        Self {
            storage_base: storage_base_url.trim_end_matches('/').to_string(),
            head_timeout: Duration::from_secs(DEFAULT_BLOB_TIMEOUT_SECS),
            blob_timeout: Duration::from_secs(DEFAULT_BLOB_TIMEOUT_SECS),
            deadline: None,
        }
    }

    /// A source scoped to ONE bounded pass: per-request ceilings plus a whole-pass
    /// deadline. This is what the boot path uses — the doorway must not hold its
    /// listener bind behind an unresponsive storage peer (measured 2026-08-22 on
    /// the household mesh: with all three peers SIGSTOPped, two slugs × two
    /// 30s resolves serialized to a 91s pre-listen stall, i.e. the node was OFF
    /// THE NETWORK — no `/health`, no watchdog — for the whole window).
    pub fn bounded(
        storage_base_url: String,
        head_timeout: Duration,
        blob_timeout: Duration,
        deadline: Instant,
    ) -> Self {
        Self {
            storage_base: storage_base_url.trim_end_matches('/').to_string(),
            head_timeout,
            blob_timeout,
            deadline: Some(deadline),
        }
    }

    /// Whether the pass budget is spent. Callers iterating slugs check this
    /// BETWEEN slugs so the remainder is deferred rather than half-attempted.
    pub fn budget_exhausted(&self) -> bool {
        self.deadline.is_some_and(|d| Instant::now() >= d)
    }

    /// This request's effective timeout: the configured ceiling clamped to what
    /// is left of the pass budget. `None` once the budget is spent — the caller
    /// must fail without a network call.
    fn effective_timeout(&self, configured: Duration) -> Option<Duration> {
        match self.deadline {
            None => Some(configured),
            Some(deadline) => {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    None
                } else {
                    Some(configured.min(remaining))
                }
            }
        }
    }

    /// Build a fresh blocking client. Each call site spawns its own thread, so
    /// sharing a client across threads is not needed — a fresh per-call client
    /// keeps the implementation simple and avoids `Arc` across the spawn boundary.
    fn make_client(timeout: Duration) -> reqwest::blocking::Client {
        reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_default()
    }
}

impl DoorwayBundleSource {
    /// Shared thread-isolated `GET /db/content/{slug}` returning the raw body.
    ///
    /// Both `resolve_blob_hash` and `resolve_server_blob_hash` hit the same
    /// endpoint and differ only in which JSON field they parse, so the blocking
    /// HTTP call (and its dedicated-OS-thread isolation) lives here once.
    fn fetch_content_body(&self, slug: &str) -> elohim_render::Result<String> {
        let url = format!("{}/db/content/{}", self.storage_base, slug);
        let Some(timeout) = self.effective_timeout(self.head_timeout) else {
            return Err(RenderError::Bootstrap(format!(
                "content GET skipped: bundle pass budget exhausted before `{url}`"
            )));
        };
        // Run on a dedicated OS thread so `reqwest::blocking` can create its own
        // tokio runtime without conflicting with an ambient tokio context (the
        // `with_projection` async constructor calls `init_renderer`).
        std::thread::spawn(move || {
            let client = DoorwayBundleSource::make_client(timeout);
            let resp = client
                .get(&url)
                .send()
                .map_err(|e| RenderError::Bootstrap(format!("content GET failed: {e}")))?;
            let status = resp.status();
            resp.error_for_status()
                .map_err(|_| {
                    RenderError::Bootstrap(format!("content GET: HTTP {status} for slug `{url}`"))
                })?
                .text()
                .map_err(|e| RenderError::Bootstrap(format!("content GET read body: {e}")))
        })
        .join()
        .map_err(|_| RenderError::Bootstrap("content GET: thread panicked".into()))?
    }
}

impl elohim_render::BundleSource for DoorwayBundleSource {
    fn resolve_blob_hash(&self, slug: &str) -> elohim_render::Result<String> {
        let body = self.fetch_content_body(slug)?;
        parse_blob_hash(&body)
    }

    fn resolve_server_blob_hash(&self, slug: &str) -> elohim_render::Result<String> {
        let body = self.fetch_content_body(slug)?;
        parse_server_blob_hash(&body)
    }

    fn fetch_blob(&self, hash: &str) -> elohim_render::Result<Vec<u8>> {
        let url = format!("{}/blob/{}", self.storage_base, hash);
        let Some(timeout) = self.effective_timeout(self.blob_timeout) else {
            return Err(RenderError::Bootstrap(format!(
                "fetch_blob skipped: bundle pass budget exhausted before `{url}`"
            )));
        };
        // Same thread-isolation rationale as resolve_blob_hash above.
        std::thread::spawn(move || {
            let client = DoorwayBundleSource::make_client(timeout);
            let resp = client
                .get(&url)
                .send()
                .map_err(|e| RenderError::Bootstrap(format!("fetch_blob GET failed: {e}")))?;
            let status = resp.status();
            resp.error_for_status()
                .map_err(|_| {
                    RenderError::Bootstrap(format!("fetch_blob: HTTP {status} for hash `{url}`"))
                })?
                .bytes()
                .map_err(|e| RenderError::Bootstrap(format!("fetch_blob read bytes: {e}")))
                .map(|b| b.to_vec())
        })
        .join()
        .map_err(|_| RenderError::Bootstrap("fetch_blob: thread panicked".into()))?
    }
}

/// Parse the `blobHash` field from a JSON body returned by `GET /db/content/{slug}`.
///
/// The storage boundary is camelCase, so the field is `blobHash`. This is the
/// **browser** bundle pointer.
pub fn parse_blob_hash(body: &str) -> elohim_render::Result<String> {
    let v: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| RenderError::Bootstrap(format!("parse_blob_hash: invalid JSON: {e}")))?;
    v.get("blobHash")
        .and_then(|h| h.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| RenderError::Bootstrap("parse_blob_hash: missing `blobHash` field".into()))
}

/// Parse the `serverBlobHash` field from a JSON body returned by
/// `GET /db/content/{slug}`.
///
/// The storage boundary is camelCase, so the field is `serverBlobHash`. This is
/// the **server** bundle pointer — the deploy-time hash of the Angular SSR
/// *server* bundle on the one EPR node. The SSR runtime materializes this, not
/// `blobHash`. Absence is an honest, non-fatal condition (the caller treats it
/// as "SSR not materializable → CSR fallback"), so a missing field is a clean
/// `Err`, never a panic.
pub fn parse_server_blob_hash(body: &str) -> elohim_render::Result<String> {
    let v: serde_json::Value = serde_json::from_str(body).map_err(|e| {
        RenderError::Bootstrap(format!("parse_server_blob_hash: invalid JSON: {e}"))
    })?;
    v.get("serverBlobHash")
        .and_then(|h| h.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            RenderError::Bootstrap("parse_server_blob_hash: missing `serverBlobHash` field".into())
        })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use elohim_render::BundleSource;

    // ── TimedDnsResolver ────────────────────────────────────────────────────

    #[test]
    fn ssr_dns_slow_threshold_sits_under_the_fetch_soft_budget() {
        assert!(!ssr_dns_lookup_is_slow(0));
        assert!(!ssr_dns_lookup_is_slow(SSR_DNS_SLOW_LOOKUP_MS - 1));
        assert!(ssr_dns_lookup_is_slow(SSR_DNS_SLOW_LOOKUP_MS));
        // A lookup must be named before it can stall a fetch, never after.
        assert!(SSR_DNS_SLOW_LOOKUP_MS < 1_200);
    }

    #[tokio::test]
    async fn timed_dns_resolver_resolves_like_the_system_resolver() {
        use reqwest::dns::Resolve;
        use std::str::FromStr;

        let name = reqwest::dns::Name::from_str("localhost").expect("valid name");
        let addrs: Vec<std::net::SocketAddr> = TimedDnsResolver::default()
            .resolve(name)
            .await
            .expect("localhost resolves")
            .collect();
        assert!(!addrs.is_empty());
        assert!(addrs.iter().all(|a| a.ip().is_loopback()));
    }

    #[tokio::test]
    async fn ssr_render_client_still_fetches_through_the_timed_resolver() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let port = listener.local_addr().expect("addr").port();
        tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.expect("accept");
            let mut buf = [0u8; 1024];
            let _ = sock.read(&mut buf).await;
            let _ = sock
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\nconnection: close\r\n\r\nok")
                .await;
        });
        // A HOSTNAME, so the request goes through the resolver (an IP literal
        // would bypass it).
        let client = crate::server::http::init_ssr_render_client();
        let body = client
            .get(format!("http://localhost:{port}/x"))
            .send()
            .await
            .expect("send")
            .text()
            .await
            .expect("body");
        assert_eq!(body, "ok");
    }

    // ── WatchedFetch: the phase and polling reading at the cut ──────────────

    /// Stores the waker it is polled with and never completes — a fetch parked
    /// on a socket.
    struct Parked(Arc<std::sync::Mutex<Option<Waker>>>);

    impl Future for Parked {
        type Output = ();
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            *self.0.lock().expect("slot") = Some(cx.waker().clone());
            Poll::Pending
        }
    }

    fn parked() -> (WatchedFetch<Parked>, Arc<std::sync::Mutex<Option<Waker>>>) {
        let slot = Arc::new(std::sync::Mutex::new(None));
        let w = WatchedFetch::new(
            Box::pin(Parked(Arc::clone(&slot))),
            FetchProbe::new("/api/v1/x"),
        );
        (w, slot)
    }

    fn poll_once<F: Future>(w: &mut WatchedFetch<F>) -> Poll<F::Output> {
        let mut cx = Context::from_waker(Waker::noop());
        Pin::new(w).poll(&mut cx)
    }

    fn wake(slot: &Arc<std::sync::Mutex<Option<Waker>>>) {
        slot.lock()
            .expect("slot")
            .as_ref()
            .expect("polled once")
            .wake_by_ref();
    }

    #[test]
    fn a_completed_watched_fetch_reports_nothing() {
        let mut w = WatchedFetch::new(Box::pin(std::future::ready(7u8)), FetchProbe::new("/x"));
        assert_eq!(poll_once(&mut w), Poll::Ready(7));
        assert!(w.probe().abandonment().is_none());
    }

    #[test]
    fn a_parked_fetch_never_woken_reads_as_waiting_not_starved() {
        let (mut w, _slot) = parked();
        assert!(poll_once(&mut w).is_pending());
        std::thread::sleep(Duration::from_millis(30));
        let a = w.probe().abandonment().expect("not completed");
        assert_eq!(a.phase, FetchPhase::Sending);
        assert_eq!(a.polls, 1);
        assert_eq!(a.woken_unpolled_ms, None, "no wake => nothing was starved");
        assert!(
            a.max_poll_gap_ms >= 25,
            "the last poll to now counts: {a:?}"
        );
        assert_eq!(a.path, "/api/v1/x");
    }

    #[test]
    fn a_wake_left_unpolled_at_the_cut_is_named() {
        let (mut w, slot) = parked();
        assert!(poll_once(&mut w).is_pending());
        wake(&slot);
        std::thread::sleep(Duration::from_millis(30));
        let a = w.probe().abandonment().expect("not completed");
        let waited = a.woken_unpolled_ms.expect("woken and not polled");
        assert!(waited >= 25, "the wake has waited ~30 ms: {a:?}");
    }

    #[test]
    fn a_late_poll_after_a_wake_is_measured_and_clears_the_pending_wake() {
        let (mut w, slot) = parked();
        assert!(poll_once(&mut w).is_pending());
        wake(&slot);
        // A second wake before the poll must not move the stamp later.
        std::thread::sleep(Duration::from_millis(30));
        wake(&slot);
        assert!(poll_once(&mut w).is_pending());
        let a = w.probe().abandonment().expect("not completed");
        assert_eq!(a.polls, 2);
        assert_eq!(a.woken_unpolled_ms, None, "the wake was polled");
        assert!(
            a.max_wake_to_poll_ms >= 25,
            "the earliest wake counts: {a:?}"
        );
    }

    #[test]
    fn the_phase_handle_moves_the_reading() {
        let (w, _slot) = parked();
        w.probe()
            .phase_handle()
            .store(FetchPhase::Body as u8, Ordering::Relaxed);
        assert_eq!(
            w.probe().abandonment().expect("not completed").phase,
            FetchPhase::Body
        );
        assert_eq!(FetchPhase::Body.as_str(), "body");
        assert_eq!(FetchPhase::Sending.as_str(), "sending");
    }

    #[test]
    fn an_abandoned_lookup_watch_is_armed_and_an_answered_one_is_not() {
        let mut answered = LookupWatch::new("h", Instant::now());
        answered.answered();
        assert!(!answered.armed);
        let abandoned = LookupWatch::new("h", Instant::now());
        assert!(abandoned.armed, "dropped without an answer => it logs");
    }

    // ── SsrAddressBook ──────────────────────────────────────────────────────

    use std::sync::atomic::AtomicUsize;

    fn addr(last: u8) -> std::net::SocketAddr {
        std::net::SocketAddr::from(([10, 0, 0, last], 0))
    }

    /// A lookup that counts its calls, answers from `answers` (cycling the
    /// last), and blocks while `gate` is held.
    fn scripted_lookup(
        calls: Arc<AtomicUsize>,
        gate: Arc<std::sync::Mutex<()>>,
        answers: Vec<std::io::Result<Vec<std::net::SocketAddr>>>,
    ) -> SsrLookupFn {
        let answers = Arc::new(std::sync::Mutex::new(answers));
        Arc::new(move |_host: &str| {
            let n = calls.fetch_add(1, Ordering::SeqCst);
            drop(gate.lock().expect("gate"));
            let answers = answers.lock().expect("answers");
            match &answers[n.min(answers.len() - 1)] {
                Ok(a) => Ok(a.clone()),
                Err(e) => Err(std::io::Error::new(e.kind(), e.to_string())),
            }
        })
    }

    fn book(lookup: SsrLookupFn, fresh: Duration, cap: Duration) -> SsrAddressBook {
        SsrAddressBook::new(lookup, fresh, cap, Duration::from_millis(50))
    }

    #[tokio::test]
    async fn a_burst_of_resolves_for_one_host_shares_one_lookup() {
        let calls = Arc::new(AtomicUsize::new(0));
        let gate = Arc::new(std::sync::Mutex::new(()));
        let held = gate.lock().expect("gate");
        let book = book(
            scripted_lookup(calls.clone(), gate.clone(), vec![Ok(vec![addr(1)])]),
            SSR_DNS_FRESH_TTL,
            SSR_DNS_STALE_CAP,
        );
        // The shape of one `/` render: 29 fetches resolve the same host at once.
        let waiters: Vec<_> = (0..29)
            .map(|_| {
                let book = book.clone();
                tokio::spawn(async move { book.resolve("storage.svc").await })
            })
            .collect();
        tokio::time::sleep(Duration::from_millis(20)).await;
        drop(held);
        for w in waiters {
            assert_eq!(w.await.expect("join").expect("resolves"), vec![addr(1)]);
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1, "one lookup, not 29");
    }

    #[tokio::test]
    async fn a_fresh_answer_is_served_without_a_lookup() {
        let calls = Arc::new(AtomicUsize::new(0));
        let gate = Arc::new(std::sync::Mutex::new(()));
        let book = book(
            scripted_lookup(calls.clone(), gate, vec![Ok(vec![addr(1)])]),
            Duration::from_secs(60),
            SSR_DNS_STALE_CAP,
        );
        book.resolve("h").await.expect("first");
        book.resolve("h").await.expect("second");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_cut_waiter_does_not_throw_the_lookup_away() {
        let calls = Arc::new(AtomicUsize::new(0));
        let gate = Arc::new(std::sync::Mutex::new(()));
        let held = gate.lock().expect("gate");
        let book = book(
            scripted_lookup(calls.clone(), gate.clone(), vec![Ok(vec![addr(7)])]),
            Duration::from_secs(60),
            SSR_DNS_STALE_CAP,
        );
        // The soft budget cuts the fetch while its lookup hangs.
        let cut = tokio::time::timeout(Duration::from_millis(30), book.resolve("h")).await;
        assert!(cut.is_err(), "the hung lookup outlasts the budget");
        // The lookup answers after the cut; its answer still lands in the book.
        drop(held);
        for _ in 0..100 {
            if book.read_last_good("h").is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert_eq!(book.resolve("h").await.expect("cached"), vec![addr(7)]);
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "the next render needs no lookup"
        );
    }

    #[tokio::test]
    async fn a_hung_refresh_serves_the_last_good_answer_within_the_grace() {
        let calls = Arc::new(AtomicUsize::new(0));
        let gate = Arc::new(std::sync::Mutex::new(()));
        let book = book(
            scripted_lookup(
                calls.clone(),
                gate.clone(),
                vec![Ok(vec![addr(1)]), Ok(vec![addr(2)])],
            ),
            Duration::ZERO, // every answer is immediately stale
            SSR_DNS_STALE_CAP,
        );
        assert_eq!(book.resolve("h").await.expect("first"), vec![addr(1)]);
        let held = gate.lock().expect("gate");
        let started = Instant::now();
        let served = book.resolve("h").await.expect("stale");
        assert_eq!(served, vec![addr(1)], "the last good answer, not a stall");
        assert!(started.elapsed() < Duration::from_millis(1_000));
        drop(held);
        for _ in 0..100 {
            if book.read_last_good("h") == Some(vec![addr(2)]) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert_eq!(
            book.read_last_good("h"),
            Some(vec![addr(2)]),
            "refresh landed"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn a_failed_lookup_is_not_remembered_and_keeps_the_last_good() {
        let calls = Arc::new(AtomicUsize::new(0));
        let gate = Arc::new(std::sync::Mutex::new(()));
        let book = book(
            scripted_lookup(
                calls.clone(),
                gate,
                vec![Err(std::io::Error::other("nxdomain")), Ok(vec![addr(3)])],
            ),
            Duration::from_secs(60),
            SSR_DNS_STALE_CAP,
        );
        assert!(book.resolve("h").await.is_err());
        assert!(book.read_last_good("h").is_none());
        assert_eq!(book.resolve("h").await.expect("retried"), vec![addr(3)]);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn an_answer_past_the_stale_cap_is_not_served() {
        let calls = Arc::new(AtomicUsize::new(0));
        let gate = Arc::new(std::sync::Mutex::new(()));
        let book = book(
            scripted_lookup(
                calls.clone(),
                gate.clone(),
                vec![Ok(vec![addr(1)]), Ok(vec![addr(2)])],
            ),
            Duration::ZERO,
            Duration::ZERO,
        );
        book.resolve("h").await.expect("first");
        // No usable answer in hand: the resolve waits for the lookup.
        assert_eq!(book.resolve("h").await.expect("second"), vec![addr(2)]);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    // ── boot materialize budget (structural no-overwhelm) ───────────────────
    //
    // The pass runs BEFORE the listener binds, so an unbounded resolve is not
    // "slow boot" — it is the node absent from the network with no /health and
    // no watchdog. Measured 2026-08-22 on the household mesh with every storage
    // peer SIGSTOPped: 91s pre-listen stall, and the a2o restart drill's 90s
    // health budget expired against a port that was never opened.

    #[test]
    fn an_unbounded_source_keeps_its_configured_timeout() {
        let src = DoorwayBundleSource::new("http://x".into());
        assert_eq!(
            src.effective_timeout(Duration::from_secs(30)),
            Some(Duration::from_secs(30)),
            "no deadline configured => the request keeps its own ceiling"
        );
        assert!(!src.budget_exhausted());
    }

    #[test]
    fn a_bounded_source_clamps_each_request_to_the_remaining_budget() {
        let src = DoorwayBundleSource::bounded(
            "http://x".into(),
            Duration::from_secs(5),
            Duration::from_secs(30),
            Instant::now() + Duration::from_secs(2),
        );
        let effective = src
            .effective_timeout(Duration::from_secs(30))
            .expect("budget is not spent yet");
        assert!(
            effective <= Duration::from_secs(2),
            "a 30s blob read may not outlive a 2s pass budget (got {effective:?})"
        );
        assert!(!src.budget_exhausted());
    }

    #[test]
    fn a_spent_budget_fails_without_a_network_call() {
        let src = DoorwayBundleSource::bounded(
            "http://x".into(),
            Duration::from_secs(5),
            Duration::from_secs(30),
            Instant::now() - Duration::from_secs(1),
        );
        assert!(src.budget_exhausted(), "the deadline is already behind us");
        assert_eq!(
            src.effective_timeout(Duration::from_secs(30)),
            None,
            "past the deadline the caller must fail fast, never dial"
        );
        // And the BundleSource surface honours it: an Err, not a 30s hang.
        let err = src
            .resolve_server_blob_hash("elohim-host-landing")
            .expect_err("a spent budget cannot resolve a head");
        assert!(
            format!("{err}").contains("budget exhausted"),
            "the refusal names the budget so a deferral is not read as an outage: {err}"
        );
    }

    #[test]
    fn strip_absolute_url_to_path() {
        assert_eq!(
            strip_to_path("https://example.com/api/v1/foo?bar=1"),
            "/api/v1/foo?bar=1"
        );
    }

    // ── ResolverFetcher trust-scope declaration ─────────────────────────────
    //
    // These pin the input to `elohim_render::JsRuntime`'s isolate trust
    // bookkeeping. The render isolate is reused across sequential renders and
    // its JS heap is never reset, so a mis-declared `Ambient` here would make a
    // real cross-principal residue channel invisible. See
    // `genesis/data/timeline/backlog/elohim-render-isolate-reuse-trust-boundary.md`.

    #[test]
    fn anonymous_resolver_fetcher_declares_ambient_trust() {
        let f = ResolverFetcher::new(Arc::new(reqwest::Client::new()), "http://x".into());
        assert_eq!(f.trust_scope(), FetcherTrust::Ambient);
    }

    #[test]
    fn credentialed_resolver_fetcher_declares_principal_trust() {
        let f = ResolverFetcher::new(Arc::new(reqwest::Client::new()), "http://x".into())
            .with_user_credential(UserCredential {
                header_name: "Cookie".into(),
                header_value: "doorway_session=abc".into(),
            });
        assert_eq!(
            f.trust_scope(),
            FetcherTrust::Principal,
            "a fetcher carrying an end-user credential reaches reach-gated data \
             on that user's behalf and MUST declare Principal, so isolate reuse \
             after it is reported as a trust crossing"
        );
    }

    #[test]
    fn stall_fault_decorator_delegates_trust_instead_of_laundering_it() {
        let inner: Arc<dyn DataFetcher> = Arc::new(
            ResolverFetcher::new(Arc::new(reqwest::Client::new()), "http://x".into())
                .with_user_credential(UserCredential {
                    header_name: "Authorization".into(),
                    header_value: "Bearer t".into(),
                }),
        );
        let decorated = StallFaultFetcher {
            inner,
            fault_substr: "nope".into(),
        };
        assert_eq!(decorated.trust_scope(), FetcherTrust::Principal);
    }

    // ── Trust-scoped render cache ───────────────────────────────────────────
    //
    // The disclosure this closes: `render_cache_key` carries NO principal, so a
    // credentialed render written into the cache is served to every other
    // requester for the TTL. The gate is the fetcher's declared trust scope.

    fn test_cache() -> ContentCache {
        ContentCache::new(crate::cache::CacheConfig::default())
    }

    const TTL: std::time::Duration = std::time::Duration::from_secs(300);

    #[test]
    fn ambient_render_round_trips_through_the_cache() {
        let cache = test_cache();
        cache_render_for_trust(
            &cache,
            "k-ambient",
            "<p>public</p>".into(),
            TTL,
            FetcherTrust::Ambient,
        );
        assert_eq!(
            cached_render_for_trust(&cache, "k-ambient", FetcherTrust::Ambient),
            Some("<p>public</p>".to_string()),
            "anonymous renders must still be cached and served — the whole point \
             of the SSR cache is preserved for the shareable path"
        );
    }

    #[test]
    fn principal_render_is_never_written_to_the_cache() {
        let cache = test_cache();
        cache_render_for_trust(
            &cache,
            "k-principal",
            "<p>alice-private</p>".into(),
            TTL,
            FetcherTrust::Principal,
        );
        // Read back on the ANONYMOUS path — this is the disclosure direction.
        assert_eq!(
            cached_render_for_trust(&cache, "k-principal", FetcherTrust::Ambient),
            None,
            "a credentialed render must never land in the cache: the key carries \
             no principal, so an anonymous request for the same URL would be \
             served another user's reach-gated HTML"
        );
        assert_eq!(
            cache.get_rendered("k-principal"),
            None,
            "nothing may reach the underlying store either"
        );
    }

    #[test]
    fn principal_render_is_never_served_from_the_cache() {
        let cache = test_cache();
        // A legitimately-cached anonymous render...
        cache_render_for_trust(
            &cache,
            "k-shared",
            "<p>public</p>".into(),
            TTL,
            FetcherTrust::Ambient,
        );
        // ...must not be handed to a credentialed request, which is entitled to
        // a reach-aware render, not a stale anonymous one.
        assert_eq!(
            cached_render_for_trust(&cache, "k-shared", FetcherTrust::Principal),
            None,
            "a credentialed request must render fresh, never inherit the \
             anonymous cache entry"
        );
    }

    #[test]
    fn only_ambient_is_cache_shareable() {
        // The rule itself lives in elohim-render (FetcherTrust::is_cache_shareable);
        // this pins doorway's adapter to it so a local re-derivation would fail.
        assert!(FetcherTrust::Ambient.is_cache_shareable());
        assert!(!FetcherTrust::Principal.is_cache_shareable());
    }

    // ── ResolverFetcher user_credential threading ───────────────────────────

    #[tokio::test]
    async fn fetcher_forwards_user_credential_header() {
        use wiremock::{matchers, Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(matchers::method("GET"))
            .and(matchers::header("authorization", "Bearer user-token"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&server)
            .await;
        let client = Arc::new(reqwest::Client::new());
        let fetcher =
            ResolverFetcher::new(client, server.uri()).with_user_credential(UserCredential {
                header_name: "Authorization".into(),
                header_value: "Bearer user-token".into(),
            });
        let req = FetchRequest {
            url: "/api/private".into(),
            method: "GET".into(),
            headers: HashMap::new(),
            body: None,
        };
        let resp = fetcher.fetch(req).await.expect("fetch ok");
        assert_eq!(resp.status, 200);
    }

    #[tokio::test]
    async fn fetcher_omits_credential_when_none() {
        use wiremock::{matchers, Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(matchers::method("GET"))
            .and(matchers::path("/api/public"))
            .respond_with(ResponseTemplate::new(200).set_body_string("public"))
            .mount(&server)
            .await;
        let client = Arc::new(reqwest::Client::new());
        let fetcher = ResolverFetcher::new(client, server.uri());
        let req = FetchRequest {
            url: "/api/public".into(),
            method: "GET".into(),
            headers: HashMap::new(),
            body: None,
        };
        let resp = fetcher.fetch(req).await.expect("fetch ok");
        assert_eq!(resp.status, 200);
        let received = server.received_requests().await.unwrap();
        let auth_header = received[0].headers.get("authorization");
        assert!(
            auth_header.is_none(),
            "no credential → no Authorization header on outbound fetch"
        );
    }

    #[tokio::test]
    async fn fetcher_maybe_with_user_credential_none_is_noop() {
        let client = Arc::new(reqwest::Client::new());
        let fetcher = ResolverFetcher::new(client, "http://localhost:8090".to_string())
            .maybe_with_user_credential(None);
        // Just verify it compiles and doesn't panic.
        // Behavioral coverage is in fetcher_omits_credential_when_none.
        assert_eq!(fetcher.storage_base, "http://localhost:8090");
    }

    #[test]
    fn strip_absolute_url_no_path() {
        assert_eq!(strip_to_path("https://example.com"), "/");
    }

    #[test]
    fn strip_relative_url_unchanged() {
        assert_eq!(strip_to_path("/api/v1/foo"), "/api/v1/foo");
    }

    #[test]
    fn strip_relative_url_with_query() {
        assert_eq!(strip_to_path("/api/v1/foo?x=1"), "/api/v1/foo?x=1");
    }

    // ── render_cache_key ────────────────────────────────────────────────────

    #[test]
    fn render_cache_key_same_inputs_produce_same_key() {
        let k1 = render_cache_key("/lamad/concept/abc", &[], "v1");
        let k2 = render_cache_key("/lamad/concept/abc", &[], "v1");
        assert_eq!(k1, k2, "identical inputs must produce identical keys");
    }

    #[test]
    fn render_cache_key_different_url_produces_different_key() {
        let k1 = render_cache_key("/lamad/concept/abc", &[], "v1");
        let k2 = render_cache_key("/lamad/concept/xyz", &[], "v1");
        assert_ne!(k1, k2, "different URLs must produce different keys");
    }

    #[test]
    fn render_cache_key_different_fetched_hashes_produces_different_key() {
        let k1 = render_cache_key("/lamad/concept/abc", &[], "v1");
        let k2 = render_cache_key("/lamad/concept/abc", &["sha256-deadbeef".to_string()], "v1");
        assert_ne!(
            k1, k2,
            "different fetched hashes must produce different keys"
        );
    }

    #[test]
    fn render_cache_key_different_spec_version_produces_different_key() {
        let k1 = render_cache_key("/lamad/concept/abc", &[], "v1");
        let k2 = render_cache_key("/lamad/concept/abc", &[], "v2");
        assert_ne!(
            k1, k2,
            "different spec versions must produce different keys"
        );
    }

    #[test]
    fn render_cache_key_empty_hashes_still_produces_a_key() {
        let k = render_cache_key("/", &[], "v1");
        assert!(
            k.starts_with("ssr-"),
            "key must start with 'ssr-' prefix; got: {k}"
        );
        // 32 hex chars for 16 bytes + 4 for "ssr-" prefix = 36 total
        assert_eq!(k.len(), 36, "key must be exactly 36 characters; got: {k}");
    }

    // ── parse_blob_hash ─────────────────────────────────────────────────────

    #[test]
    fn parse_blob_hash_valid_returns_hash() {
        let body = r#"{"blobHash":"sha256-abc"}"#;
        let result = parse_blob_hash(body);
        assert_eq!(result.unwrap(), "sha256-abc");
    }

    #[test]
    fn parse_blob_hash_missing_field_returns_err() {
        let body = r#"{"someOtherField":"value"}"#;
        let result = parse_blob_hash(body);
        assert!(result.is_err(), "expected Err on missing blobHash field");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("blobHash"),
            "error should mention missing field"
        );
    }

    #[test]
    fn parse_blob_hash_invalid_json_returns_err() {
        let body = "not json at all";
        let result = parse_blob_hash(body);
        assert!(result.is_err(), "expected Err on invalid JSON");
    }

    #[test]
    fn parse_blob_hash_blob_hash_not_string_returns_err() {
        let body = r#"{"blobHash":42}"#;
        let result = parse_blob_hash(body);
        assert!(
            result.is_err(),
            "expected Err when blobHash is not a string"
        );
    }

    // ── parse_server_blob_hash ──────────────────────────────────────────────

    #[test]
    fn parse_server_blob_hash_returns_server_not_browser() {
        // A body carrying BOTH pointers — must select the SERVER one. This is
        // the exact shape `GET /db/content/elohim-host-landing` returns after
        // the row collapse (one EPR node, both hashes).
        let body = r#"{"blobHash":"sha256-browser","serverBlobHash":"sha256-server"}"#;
        let result = parse_server_blob_hash(body);
        assert_eq!(
            result.unwrap(),
            "sha256-server",
            "must return serverBlobHash, not blobHash"
        );
    }

    #[test]
    fn parse_server_blob_hash_missing_field_returns_err() {
        // `blobHash` present but `serverBlobHash` absent (the migration-safety
        // shape: SSR simply isn't materializable yet → caller does CSR fallback).
        let body = r#"{"blobHash":"sha256-browser"}"#;
        let result = parse_server_blob_hash(body);
        assert!(
            result.is_err(),
            "expected Err when serverBlobHash field is absent"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("serverBlobHash"),
            "error should mention the missing field; got: {msg}"
        );
    }

    #[test]
    fn parse_server_blob_hash_invalid_json_returns_err() {
        let result = parse_server_blob_hash("not json at all");
        assert!(result.is_err(), "expected Err on invalid JSON");
    }

    #[test]
    fn parse_server_blob_hash_not_string_returns_err() {
        let body = r#"{"serverBlobHash":42}"#;
        let result = parse_server_blob_hash(body);
        assert!(
            result.is_err(),
            "expected Err when serverBlobHash is not a string"
        );
    }

    #[tokio::test]
    async fn bundle_source_resolve_server_blob_hash_ok_from_async_context() {
        use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(matchers::method("GET"))
            .and(matchers::path("/db/content/elohim-host-landing"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"{"blobHash":"sha256-browser","serverBlobHash":"sha256-server"}"#,
            ))
            .mount(&server)
            .await;

        let src = DoorwayBundleSource::new(server.uri());
        // Thread-isolation regression: must not panic from within a tokio runtime.
        let result = src.resolve_server_blob_hash("elohim-host-landing");
        assert_eq!(
            result.expect("resolve_server_blob_hash must succeed from async context"),
            "sha256-server"
        );
    }

    #[test]
    fn render_cache_key_hash_ordering_matters() {
        let hashes_a = vec!["hash1".to_string(), "hash2".to_string()];
        let hashes_b = vec!["hash2".to_string(), "hash1".to_string()];
        let k1 = render_cache_key("/url", &hashes_a, "v1");
        let k2 = render_cache_key("/url", &hashes_b, "v1");
        assert_ne!(
            k1, k2,
            "hash ordering must affect the key (list is ordered)"
        );
    }

    // ── DoorwayBundleSource async-caller regression ─────────────────────────
    //
    // These tests run from WITHIN a tokio runtime (via #[tokio::test]) to prove
    // that `DoorwayBundleSource::resolve_blob_hash` does NOT panic with "Cannot
    // start a runtime from within a runtime." Before the thread-isolation fix the
    // blocking reqwest call would create its own tokio runtime on the tokio
    // worker thread — a nested-runtime panic.  After the fix the blocking call
    // runs on a dedicated OS thread with no ambient tokio context.

    #[tokio::test]
    async fn bundle_source_resolve_blob_hash_ok_from_async_context() {
        use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

        // Start a wiremock server (runs its own tokio task — fine; we just call
        // the blocking DoorwayBundleSource from within this async test).
        let server = MockServer::start().await;
        Mock::given(matchers::method("GET"))
            .and(matchers::path("/db/content/my-bundle-slug"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(r#"{"blobHash":"sha256-deadbeef"}"#),
            )
            .mount(&server)
            .await;

        let src = DoorwayBundleSource::new(server.uri());
        // This call PANICS before the thread-isolation fix; it must return Ok after.
        let result = src.resolve_blob_hash("my-bundle-slug");
        assert_eq!(
            result.expect("resolve_blob_hash must succeed from async context"),
            "sha256-deadbeef"
        );
    }

    #[tokio::test]
    async fn bundle_source_resolve_blob_hash_404_returns_http_status_err() {
        use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(matchers::method("GET"))
            .and(matchers::path("/db/content/missing-slug"))
            .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
            .mount(&server)
            .await;

        let src = DoorwayBundleSource::new(server.uri());
        let err = src
            .resolve_blob_hash("missing-slug")
            .expect_err("404 must produce an Err");
        let msg = err.to_string();
        // The error must mention the HTTP status so callers get a useful message
        // instead of a misleading "missing blobHash field".
        assert!(
            msg.contains("404"),
            "error message must mention the HTTP status; got: {msg}"
        );
    }
}
