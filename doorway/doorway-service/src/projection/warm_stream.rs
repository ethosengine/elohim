//! SSE Cache Stream Consumer — warm-up via streaming from peer storage.
//!
//! Replaces the old HTTP pull warm-up (`warm.rs`). Connects to each peer's
//! `GET /api/v1/cache/stream` endpoint and processes SSE events into the
//! projection store (MongoDB).
//!
//! ## Trigger Points
//!
//! 1. **Startup** — for each peer storage URL
//! 2. **Subscriber reconnect** — after AppWebsocket reconnects to conductor

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

use elohim_compute::{CircuitBreaker, CircuitState};
use futures_util::StreamExt;
use serde::Serialize;
use serde_json::Value as JsonValue;
use tracing::{debug, error, info, warn};

/// Maximum retry attempts per peer before giving up.
const MAX_WARMUP_RETRIES: u32 = 5;

/// Base delay between retries (doubles each attempt).
const WARMUP_RETRY_BASE_SECS: u64 = 10;

/// Maximum delay between retries.
const WARMUP_RETRY_MAX_SECS: u64 = 120;

/// Per-stream whole-request timeout. Shrunk from 300s so one upstream cannot
/// burn through the k8s liveness kill window (W5 floor 150s). A healthy-but-
/// large peer cut at this bound still sets `has_content`, breaking its retry
/// loop (no re-stream), so the shrink does not starve healthy peers.
const WARMUP_STREAM_TIMEOUT_SECS: u64 = 45;

/// Hard cap on TOTAL warm-up wall-time across the whole peer set (~0.5x the
/// W5 liveness kill-window floor of 150s). The peer-SET bound the per-peer
/// timeout cannot provide. `pub` so the /health/startup surface reports the
/// enforced value rather than a hand-copied literal that could silently drift.
pub const WARMUP_TOTAL_BUDGET_SECS: u64 = 75;

/// Consecutive failed warm-up outcomes before an upstream circuit opens.
const WARMUP_CIRCUIT_FAIL_THRESHOLD: u32 = 3;

/// Warm-up passes (ticks) an open circuit waits before a half-open trial.
const WARMUP_CIRCUIT_COOLDOWN_TICKS: u64 = 5;

/// Yield to the runtime every N projected entries so warm-up cannot
/// monopolize the throttled tokio workers.
const WARMUP_YIELD_EVERY_N: usize = 64;

/// Real inter-batch pace (ms) applied at each WARMUP_YIELD_EVERY_N boundary.
/// `yield_now()` only REORDERS ready tasks; under a CFS CPU quota (cpu:1) a
/// warm_stream catch-up burst still pegs the quota → CFS throttles the whole
/// process → the /health task can't get a slice inside the 15s probe window →
/// liveness SIGKILL (doorway-freeze 2026-06-14: warm_stream burst-starvation,
/// operator cluster-confirmed). A short real sleep caps warm_stream's CPU duty
/// cycle, leaving headroom for /health + request handling. ~0.45s added over a
/// 3.6k-doc corpus (56 batches × 8ms) — negligible vs the 75s total budget.
/// This is the durable "pace itself" fix the cpu 1→2 bump only band-aided.
/// Default 8ms; override WARMUP_PACE_MS (0 disables — falls back to yield-only).
const WARMUP_PACE_MS_DEFAULT: u64 = 8;

fn parse_warmup_pace(raw: Option<String>) -> std::time::Duration {
    let ms = raw
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(WARMUP_PACE_MS_DEFAULT);
    std::time::Duration::from_millis(ms)
}

fn warmup_pace() -> std::time::Duration {
    parse_warmup_pace(std::env::var("WARMUP_PACE_MS").ok())
}

use super::document::ProjectedDocument;
use super::store::ProjectionStore;

/// A parsed SSE event
#[derive(Debug)]
pub struct SseEvent {
    pub event_type: String,
    pub id: String,
    pub data: JsonValue,
}

/// Parse accumulated SSE lines into an event.
/// Returns None for heartbeats/comments.
pub fn parse_sse_event(lines: &[String]) -> Option<SseEvent> {
    let mut event_type = String::new();
    let mut id = String::new();
    let mut data = String::new();

    for line in lines {
        if line.starts_with(": ") || line == ":" {
            return None; // Comment/heartbeat
        } else if let Some(val) = line.strip_prefix("event: ") {
            event_type = val.to_string();
        } else if let Some(val) = line.strip_prefix("id: ") {
            id = val.to_string();
        } else if let Some(val) = line.strip_prefix("data: ") {
            data = val.to_string();
        }
    }

    if event_type.is_empty() && data.is_empty() {
        return None;
    }

    let data = serde_json::from_str(&data).unwrap_or(JsonValue::Null);

    Some(SseEvent {
        event_type,
        id,
        data,
    })
}

/// Map SSE event type to ProjectedDocument doc_type
pub fn event_type_to_doc_type(event_type: &str) -> Option<&'static str> {
    match event_type {
        // Paths are now ContentNodes — they arrive as cache.content events
        "cache.content" => Some("Content"),
        "cache.human" => Some("Human"),
        "cache.relationship" => Some("Relationship"),
        _ => None,
    }
}

/// Result of a cache stream warm-up
#[derive(Debug, Default)]
pub struct StreamResult {
    pub content_count: usize,
    pub human_count: usize,
    pub relationship_count: usize,
    pub errors: Vec<String>,
}

/// Per-upstream observable health surfaced on /health/startup (W6).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpstreamHealth {
    pub upstream: String,
    pub error_streak: u32,
    pub last_good: Option<u64>,
    pub circuit: CircuitState,
    pub skipped: bool,
}

/// Structured harvest seam for a failed self-heal / anti-self-partition trip.
/// Plan D (elevate-arm poller) will consume these; here we only emit + warn.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WarmupSelfHealEvent {
    pub event: &'static str,
    pub detail: String,
}

/// Doorway-local per-upstream circuit map + observable snapshot. Cat C
/// node-local operational state (no DHT entry, no table). Mirrors the
/// storage-side peer_statuses consumer idiom (peer_selection.rs:138):
/// consult per-key health before acting.
pub struct WarmStreamHealth {
    breakers: std::sync::Mutex<std::collections::HashMap<String, CircuitBreaker>>,
    fail_threshold: u32,
    cooldown_ticks: u64,
}

impl WarmStreamHealth {
    pub fn new(fail_threshold: u32, cooldown_ticks: u64) -> Self {
        Self {
            breakers: std::sync::Mutex::new(std::collections::HashMap::new()),
            fail_threshold,
            cooldown_ticks,
        }
    }

    pub fn should_skip(&self, url: &str, tick: u64) -> bool {
        let mut map = self.breakers.lock().unwrap();
        let cb = map
            .entry(url.to_string())
            .or_insert_with(|| CircuitBreaker::new(self.fail_threshold, self.cooldown_ticks));
        cb.should_skip(tick)
    }

    pub fn record_outcome(&self, url: &str, ok: bool, tick: u64) {
        let mut map = self.breakers.lock().unwrap();
        let cb = map
            .entry(url.to_string())
            .or_insert_with(|| CircuitBreaker::new(self.fail_threshold, self.cooldown_ticks));
        cb.record_outcome(ok, tick);
    }

    pub fn snapshot(&self) -> Vec<UpstreamHealth> {
        let map = self.breakers.lock().unwrap();
        map.iter()
            .map(|(url, cb)| UpstreamHealth {
                upstream: url.clone(),
                error_streak: cb.error_streak(),
                last_good: cb.last_good_tick(),
                circuit: cb.state(),
                skipped: cb.is_open(),
            })
            .collect()
    }

    /// Filter the upstream set to non-skipped peers BEFORE iterating
    /// (mirrors peer_selection.rs:138 filter-before-act). ANTI-SELF-PARTITION:
    /// if every upstream is circuit-open this returns the least-bad ONE
    /// (lowest error_streak, ties resolved by input order) rather than empty,
    /// and emits an elevate-worthy self-heal event (Plan D harvest seam).
    pub fn gate_upstreams(&self, urls: &[String], tick: u64) -> Vec<String> {
        if urls.is_empty() {
            return Vec::new();
        }
        let admitted: Vec<String> = urls
            .iter()
            .filter(|u| !self.should_skip(u, tick))
            .cloned()
            .collect();
        if !admitted.is_empty() {
            return admitted;
        }
        // All skipped: choose least-bad, never partition to none.
        let map = self.breakers.lock().unwrap();
        let least_bad = urls
            .iter()
            .min_by_key(|u| map.get(*u).map(|cb| cb.error_streak()).unwrap_or(0))
            .cloned()
            .unwrap_or_else(|| urls[0].clone());
        drop(map);
        let event = WarmupSelfHealEvent {
            event: "anti-self-partition",
            detail: format!(
                "all {} upstreams circuit-open; proceeding least-bad {}",
                urls.len(),
                least_bad
            ),
        };
        warn!(
            event = event.event,
            detail = %event.detail,
            "Warm-up anti-self-partition guard tripped (elevate-worthy)"
        );
        vec![least_bad]
    }
}

impl Default for WarmStreamHealth {
    fn default() -> Self {
        Self::new(WARMUP_CIRCUIT_FAIL_THRESHOLD, WARMUP_CIRCUIT_COOLDOWN_TICKS)
    }
}

/// Stream cacheable content from a peer's storage into the projection store.
pub async fn stream_from_peer(store: Arc<ProjectionStore>, storage_url: &str) -> StreamResult {
    let mut result = StreamResult::default();
    let base = storage_url.trim_end_matches('/');
    let url = format!("{base}/api/v1/cache/stream");

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(WARMUP_STREAM_TIMEOUT_SECS))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            result
                .errors
                .push(format!("Failed to build HTTP client: {e}"));
            return result;
        }
    };

    let response = match client.get(&url).send().await {
        Ok(resp) => {
            if !resp.status().is_success() {
                result
                    .errors
                    .push(format!("HTTP {} from {url}", resp.status()));
                return result;
            }
            resp
        }
        Err(e) => {
            result
                .errors
                .push(format!("Failed to connect to {url}: {e}"));
            return result;
        }
    };

    info!(url = %url, "Connected to cache stream");

    // Stream response bytes and parse SSE lines
    let mut byte_stream = response.bytes_stream();
    let mut line_buffer = String::new();
    let mut event_lines: Vec<String> = Vec::new();
    let mut projected: usize = 0;
    let pace = warmup_pace();

    while let Some(chunk_result) = byte_stream.next().await {
        let chunk = match chunk_result {
            Ok(bytes) => bytes,
            Err(e) => {
                warn!(error = %e, "Error reading stream chunk");
                result.errors.push(format!("Stream read error: {e}"));
                break;
            }
        };

        let text = match std::str::from_utf8(&chunk) {
            Ok(t) => t,
            Err(e) => {
                warn!(error = %e, "Non-UTF8 chunk in SSE stream");
                continue;
            }
        };

        line_buffer.push_str(text);

        // Process complete lines from the buffer
        while let Some(newline_pos) = line_buffer.find('\n') {
            let line = line_buffer[..newline_pos]
                .trim_end_matches('\r')
                .to_string();
            line_buffer = line_buffer[newline_pos + 1..].to_string();

            if line.is_empty() {
                // Empty line = end of SSE event
                if !event_lines.is_empty() {
                    if let Some(event) = parse_sse_event(&event_lines) {
                        // Check for done signal
                        if event.event_type == "cache.done" {
                            info!(
                                content = result.content_count,
                                humans = result.human_count,
                                relationships = result.relationship_count,
                                "Cache stream completed (done event)"
                            );
                            event_lines.clear();
                            return result;
                        }

                        // Map event type to doc_type and project
                        if let Some(doc_type) = event_type_to_doc_type(&event.event_type) {
                            let doc_id = if !event.id.is_empty() {
                                event.id.clone()
                            } else {
                                event
                                    .data
                                    .get("id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")
                                    .to_string()
                            };

                            let doc = ProjectedDocument::new(
                                doc_type,
                                &doc_id,
                                "cache-stream",
                                "cache-stream",
                                event.data,
                            );

                            if let Err(e) = store.set(doc).await {
                                warn!(
                                    doc_type,
                                    doc_id = %doc_id,
                                    error = %e,
                                    "Failed to project streamed entry"
                                );
                                result.errors.push(format!("{doc_type}:{doc_id}: {e}"));
                            } else {
                                match doc_type {
                                    "Content" => result.content_count += 1,
                                    "Human" => result.human_count += 1,
                                    "Relationship" => result.relationship_count += 1,
                                    _ => {}
                                }
                                debug!(doc_type, doc_id = %doc_id, "Projected streamed entry");
                                projected += 1;
                                if projected.is_multiple_of(WARMUP_YIELD_EVERY_N) {
                                    tokio::task::yield_now().await;
                                    // Real pace, not just a reorder — caps CPU duty
                                    // cycle so /health gets a runtime slice under a
                                    // 1-core CFS quota (see WARMUP_PACE_MS_DEFAULT).
                                    if !pace.is_zero() {
                                        tokio::time::sleep(pace).await;
                                    }
                                }
                            }
                        }
                    }
                    event_lines.clear();
                }
            } else {
                event_lines.push(line);
            }
        }
    }

    info!(
        content = result.content_count,
        humans = result.human_count,
        relationships = result.relationship_count,
        "Cache stream finished (stream ended)"
    );

    result
}

/// How long to wait before re-warming after a pass that produced nothing.
///
/// The empty pass is cheap and the upstream it is waiting for is a pod that is
/// still starting, so this polls on the same order as the EPR router's own
/// self-heal refresh (30s) rather than backing off — the whole point is to be
/// warm shortly after storage appears, without an operator restart.
pub const WARMUP_REWARM_POLL_SECS: u64 = 30;

/// Observable warmup state — read by /health/startup, written by spawn_stream_task.
pub struct WarmupState {
    pub in_progress: AtomicBool,
    pub attempts: AtomicU32,
    pub max_attempts: AtomicU32,
    pub last_error: std::sync::Mutex<Option<String>>,
    /// Warmed, and warmed against something. See [`WarmupState::completed_empty`]
    /// for why those are not the same claim.
    pub completed: AtomicBool,
    /// A pass ran to the end and produced NOTHING — storage was unreachable, or
    /// reachable and empty.
    ///
    /// WHY THIS EXISTS. MEASURED (local mesh 2026-08-21): doorway A booted at
    /// 22:55; storage came up at ~23:07. The warmup ran against empty/unreachable
    /// storage and reported `completed: true` with `projection.content: 0` and
    /// `servedBundleHeads: []` — and a completed warmup never re-ran, so the
    /// doorway served an empty projection until someone restarted it. `completed:
    /// true` could not distinguish "warmed" from "warmed against nothing", which
    /// is the boot-order hazard behind the AGENT_KEY_ERROR / empty-projection
    /// symptoms we had been curing by hand. (The role-discovery retry in
    /// da63fd7a0 fixed the CONDUCTOR half of this; this is the storage half.)
    ///
    /// A pass that produced nothing is not a completed warmup — it is a warmup
    /// still waiting for its upstreams. `completed` stays FALSE and this reads
    /// true, and the task re-warms on its own once the pool has something.
    pub completed_empty: AtomicBool,
    /// Records streamed by the most recent pass (content + humans +
    /// relationships, across peers). The evidence `completed` is derived from.
    pub produced: AtomicU64,
    pub health: WarmStreamHealth,
}

impl WarmupState {
    /// True when the last pass finished having produced nothing, so the doorway
    /// is serving an empty projection and is waiting to re-warm.
    pub fn is_empty_warm(&self) -> bool {
        self.completed_empty.load(Ordering::Relaxed) && !self.in_progress.load(Ordering::Relaxed)
    }
}

impl WarmupState {
    pub fn new() -> Self {
        Self {
            in_progress: AtomicBool::new(false),
            attempts: AtomicU32::new(0),
            max_attempts: AtomicU32::new(MAX_WARMUP_RETRIES),
            last_error: std::sync::Mutex::new(None),
            completed: AtomicBool::new(false),
            completed_empty: AtomicBool::new(false),
            produced: AtomicU64::new(0),
            health: WarmStreamHealth::new(
                WARMUP_CIRCUIT_FAIL_THRESHOLD,
                WARMUP_CIRCUIT_COOLDOWN_TICKS,
            ),
        }
    }
}

impl Default for WarmupState {
    fn default() -> Self {
        Self::new()
    }
}

/// Spawn cache stream warm-up for multiple peers as a background task.
/// This is the startup entry point — called from main.rs.
///
/// Retries each peer up to [`MAX_WARMUP_RETRIES`] times with exponential
/// backoff (base [`WARMUP_RETRY_BASE_SECS`]s, cap [`WARMUP_RETRY_MAX_SECS`]s)
/// so that k8s pod restart ordering doesn't permanently empty the cache.
pub fn spawn_stream_task(
    store: Arc<ProjectionStore>,
    storage_urls: Vec<String>,
    delay_secs: u64,
    warmup_state: Option<Arc<WarmupState>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // Let services settle before streaming
        tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;

        // RE-WARM LOOP. A pass that produced nothing is not a finished warmup —
        // storage was unreachable, or reachable and empty, and a doorway that
        // stops there serves an empty projection until an operator restarts it
        // (measured 2026-08-21: boot at 22:55, storage up at ~23:07, empty
        // forever after). Each iteration is one full pass; a pass that produces
        // records is terminal, a pass that produces none sleeps and tries again,
        // so the doorway warms itself shortly after storage appears.
        let mut rewarm_attempt: u32 = 0;
        loop {
            rewarm_attempt += 1;
            if let Some(ref ws) = warmup_state {
                ws.in_progress.store(true, Ordering::Relaxed);
                // Each pass is judged on its OWN production.
                ws.produced.store(0, Ordering::Relaxed);
            }

            // Single startup pass. A tick is a warm-up-PASS counter, and there is
            // exactly one pass here, so tick stays 0. Cross-pass cooldown/half-open
            // recovery is intentionally INERT in doorway: per-conductor subscribers
            // warm a single peer on reconnect (subscriber.rs), where gating would
            // risk starving the only upstream — so the reconnect path records
            // outcomes for observability but does not gate. The cooldown/half-open
            // machinery is exercised by Plan B's continuous admission path (real
            // clock, peer-SET). What protects warm-up HERE is tick-independent: the
            // startup gate_upstreams + the mid-pass is_open() early-bail + the 75s
            // total budget + the per-entry yield.
            let tick: u64 = 0;

            // Gate the upstream set BEFORE iterating (filter-before-act); the guard
            // guarantees a non-empty result if storage_urls is non-empty.
            let gated: Vec<String> = match warmup_state {
                Some(ref ws) => ws.health.gate_upstreams(&storage_urls, tick),
                None => storage_urls.clone(),
            };

            info!(
                peer_count = gated.len(),
                total_peers = storage_urls.len(),
                "Starting cache stream warm-up (health-gated)"
            );

            let budget = std::time::Duration::from_secs(WARMUP_TOTAL_BUDGET_SECS);
            let pass = async {
                for storage_url in &gated {
                    let mut attempt: u32 = 0;
                    loop {
                        attempt += 1;
                        if let Some(ref ws) = warmup_state {
                            ws.attempts.store(attempt, Ordering::Relaxed);
                        }

                        let result = stream_from_peer(Arc::clone(&store), storage_url).await;
                        let streamed =
                            result.content_count + result.human_count + result.relationship_count;
                        let has_content = streamed > 0;
                        // NOTE: an EMPTY-but-reachable peer still reads `ok` here —
                        // deliberately, because an empty peer is not a failed peer
                        // and must not open the warmup breaker. What it must not do
                        // is count as a COMPLETED warmup; that verdict is taken from
                        // `produced` at the end of the pass, not from `ok`.
                        let ok = result.errors.is_empty() || has_content;

                        if let Some(ref ws) = warmup_state {
                            ws.produced.fetch_add(streamed as u64, Ordering::Relaxed);
                            ws.health.record_outcome(storage_url, ok, tick);
                        }

                        if ok {
                            info!(
                                storage_url = %storage_url,
                                content = result.content_count,
                                humans = result.human_count,
                                relationships = result.relationship_count,
                                attempt,
                                "Cache stream warm-up completed successfully"
                            );
                            break;
                        }

                        if let Some(ref ws) = warmup_state {
                            if let Ok(mut guard) = ws.last_error.lock() {
                                *guard = result.errors.first().cloned();
                            }
                            // Breaker opened mid-pass: stop burning the backoff ladder.
                            if ws.health.should_skip(storage_url, tick) {
                                warn!(
                                    storage_url = %storage_url,
                                    attempt,
                                    "Upstream circuit opened; bailing retry ladder early"
                                );
                                break;
                            }
                        }

                        if attempt >= MAX_WARMUP_RETRIES {
                            error!(
                                storage_url = %storage_url,
                                attempts = attempt,
                                errors = ?result.errors,
                                "Cache stream warm-up failed after max retries"
                            );
                            break;
                        }

                        let retry_delay = WARMUP_RETRY_BASE_SECS
                            .saturating_mul(2u64.pow(attempt - 1))
                            .min(WARMUP_RETRY_MAX_SECS);
                        warn!(
                            storage_url = %storage_url,
                            attempt,
                            max_retries = MAX_WARMUP_RETRIES,
                            retry_delay_secs = retry_delay,
                            errors = ?result.errors,
                            "Cache stream warm-up failed, retrying"
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(retry_delay)).await;
                    }
                }
            };

            if tokio::time::timeout(budget, pass).await.is_err() {
                let event = WarmupSelfHealEvent {
                    event: "budget-exhausted",
                    detail: format!("warm-up exceeded {WARMUP_TOTAL_BUDGET_SECS}s total budget"),
                };
                warn!(
                    event = event.event,
                    detail = %event.detail,
                    "Warm-up total budget exhausted; stopping pass (elevate-worthy)"
                );
            }

            // THE VERDICT. Previously an unconditional `completed = true`: the pass
            // reached its end, therefore it was declared warm — regardless of
            // whether a single record had been streamed. Production, not arrival at
            // the end of a loop, is what makes a warmup complete.
            let produced = match warmup_state {
                Some(ref ws) => {
                    let produced = ws.produced.load(Ordering::Relaxed);
                    ws.in_progress.store(false, Ordering::Relaxed);
                    ws.completed.store(produced > 0, Ordering::Relaxed);
                    ws.completed_empty.store(produced == 0, Ordering::Relaxed);
                    produced
                }
                // No observable state to report into: keep the old single-pass
                // shape rather than looping invisibly forever.
                None => break,
            };

            if produced > 0 {
                info!(
                    produced,
                    rewarm_attempt, "Cache stream warm-up complete (projection populated)"
                );
                break;
            }

            warn!(
                rewarm_attempt,
                retry_secs = WARMUP_REWARM_POLL_SECS,
                "Warm-up pass produced NOTHING (storage unreachable or empty) — \
             not marking completed; will re-warm. This doorway is serving an \
             empty projection until a pass produces records."
            );
            tokio::time::sleep(std::time::Duration::from_secs(WARMUP_REWARM_POLL_SECS)).await;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The false-green this closes: `completed` was stored unconditionally at
    /// the end of the pass, so a warmup that streamed NOTHING (storage
    /// unreachable, or reachable and empty) reported itself warm and never
    /// re-ran. Measured on the local mesh 2026-08-21 — doorway up at 22:55,
    /// storage at ~23:07, empty projection served until restart.
    #[test]
    fn a_pass_that_produced_nothing_is_not_completed() {
        let ws = WarmupState::new();
        // End-of-pass verdict, taken from production rather than from arrival.
        let produced = ws.produced.load(Ordering::Relaxed);
        ws.in_progress.store(false, Ordering::Relaxed);
        ws.completed.store(produced > 0, Ordering::Relaxed);
        ws.completed_empty.store(produced == 0, Ordering::Relaxed);

        assert!(
            !ws.completed.load(Ordering::Relaxed),
            "producing nothing is not a completed warmup"
        );
        assert!(ws.completed_empty.load(Ordering::Relaxed));
        assert!(
            ws.is_empty_warm(),
            "and the doorway must report itself as serving an empty projection"
        );
    }

    #[test]
    fn a_pass_that_produced_records_is_completed() {
        let ws = WarmupState::new();
        ws.produced.store(42, Ordering::Relaxed);
        let produced = ws.produced.load(Ordering::Relaxed);
        ws.in_progress.store(false, Ordering::Relaxed);
        ws.completed.store(produced > 0, Ordering::Relaxed);
        ws.completed_empty.store(produced == 0, Ordering::Relaxed);

        assert!(ws.completed.load(Ordering::Relaxed));
        assert!(!ws.completed_empty.load(Ordering::Relaxed));
        assert!(!ws.is_empty_warm());
    }

    /// A pass still running is never "empty-warm" — that verdict belongs only to
    /// a pass that FINISHED having produced nothing, or a doorway mid-warmup
    /// would 503 its own serving probe on every boot.
    #[test]
    fn a_warmup_in_progress_is_not_empty_warm() {
        let ws = WarmupState::new();
        ws.in_progress.store(true, Ordering::Relaxed);
        ws.completed_empty.store(true, Ordering::Relaxed);
        assert!(!ws.is_empty_warm());
    }

    #[test]
    fn warmup_pace_parses_env_and_defaults() {
        assert_eq!(
            parse_warmup_pace(None).as_millis(),
            WARMUP_PACE_MS_DEFAULT as u128
        );
        assert_eq!(parse_warmup_pace(Some("20".to_string())).as_millis(), 20);
        assert_eq!(
            parse_warmup_pace(Some("0".to_string())).as_millis(),
            0,
            "0 explicitly disables the real pace (yield-only fallback)"
        );
        assert_eq!(
            parse_warmup_pace(Some("bad".to_string())).as_millis(),
            WARMUP_PACE_MS_DEFAULT as u128
        );
    }

    #[test]
    fn test_parse_sse_event_content() {
        let lines = vec![
            "event: cache.content".to_string(),
            "id: manifesto".to_string(),
            r#"data: {"id":"manifesto","title":"Test"}"#.to_string(),
        ];
        let event = parse_sse_event(&lines).unwrap();
        assert_eq!(event.event_type, "cache.content");
        assert_eq!(event.id, "manifesto");
        assert!(event.data.is_object());
    }

    #[test]
    fn test_parse_sse_event_done() {
        let lines = vec![
            "event: cache.done".to_string(),
            r#"data: {"content":10,"paths":3,"humans":2,"relationships":5}"#.to_string(),
        ];
        let event = parse_sse_event(&lines).unwrap();
        assert_eq!(event.event_type, "cache.done");
    }

    #[test]
    fn test_parse_sse_heartbeat_returns_none() {
        let lines = vec![": heartbeat".to_string()];
        assert!(parse_sse_event(&lines).is_none());
    }

    #[test]
    fn test_parse_sse_bare_comment_returns_none() {
        let lines = vec![":".to_string()];
        assert!(parse_sse_event(&lines).is_none());
    }

    #[test]
    fn test_parse_sse_empty_lines_returns_none() {
        let lines: Vec<String> = vec![];
        assert!(parse_sse_event(&lines).is_none());
    }

    #[test]
    fn test_parse_sse_event_with_no_id() {
        let lines = vec![
            "event: cache.human".to_string(),
            r#"data: {"id":"agent-123","name":"Alice"}"#.to_string(),
        ];
        let event = parse_sse_event(&lines).unwrap();
        assert_eq!(event.event_type, "cache.human");
        assert_eq!(event.id, "");
        assert!(event.data.is_object());
    }

    #[test]
    fn test_parse_sse_event_invalid_json_data() {
        let lines = vec![
            "event: cache.content".to_string(),
            "id: test".to_string(),
            "data: not-valid-json".to_string(),
        ];
        let event = parse_sse_event(&lines).unwrap();
        assert_eq!(event.event_type, "cache.content");
        assert_eq!(event.data, JsonValue::Null);
    }

    #[test]
    fn test_event_type_to_doc_type() {
        assert_eq!(event_type_to_doc_type("cache.content"), Some("Content"));
        assert_eq!(event_type_to_doc_type("cache.human"), Some("Human"));
        assert_eq!(
            event_type_to_doc_type("cache.relationship"),
            Some("Relationship")
        );
        assert_eq!(event_type_to_doc_type("cache.done"), None);
        // cache.path no longer exists — paths are ContentNodes now
        assert_eq!(event_type_to_doc_type("cache.path"), None);
    }

    #[test]
    fn test_event_type_to_doc_type_unknown() {
        assert_eq!(event_type_to_doc_type("cache.unknown"), None);
        assert_eq!(event_type_to_doc_type(""), None);
        assert_eq!(event_type_to_doc_type("something.else"), None);
    }

    #[test]
    fn test_stream_result_default() {
        let result = StreamResult::default();
        assert_eq!(result.content_count, 0);
        assert_eq!(result.human_count, 0);
        assert_eq!(result.relationship_count, 0);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_retry_delay_calculation() {
        let base_delay_secs: u64 = 10;
        let max_delay_secs: u64 = 120;

        assert_eq!(
            base_delay_secs
                .saturating_mul(2u64.pow(0))
                .min(max_delay_secs),
            10
        );
        assert_eq!(
            base_delay_secs
                .saturating_mul(2u64.pow(1))
                .min(max_delay_secs),
            20
        );
        assert_eq!(
            base_delay_secs
                .saturating_mul(2u64.pow(2))
                .min(max_delay_secs),
            40
        );
        assert_eq!(
            base_delay_secs
                .saturating_mul(2u64.pow(3))
                .min(max_delay_secs),
            80
        );
        assert_eq!(
            base_delay_secs
                .saturating_mul(2u64.pow(4))
                .min(max_delay_secs),
            120
        );
    }

    #[test]
    #[allow(clippy::assertions_on_constants)] // intentional invariant guard on tuning constants
    fn per_stream_timeout_under_liveness_window() {
        // kill-window floor is 150s; per-stream timeout must stay well under it.
        assert_eq!(WARMUP_STREAM_TIMEOUT_SECS, 45);
        assert!(
            WARMUP_STREAM_TIMEOUT_SECS < 150 - 60,
            "must keep >=60s margin under floor"
        );
    }

    #[test]
    #[allow(clippy::assertions_on_constants)] // intentional invariant guard on tuning constants
    fn warmup_budget_constants_sane() {
        assert_eq!(WARMUP_TOTAL_BUDGET_SECS, 75); // ~0.5x kill-window floor 150
        assert!(WARMUP_TOTAL_BUDGET_SECS <= 150 / 2);
        assert_eq!(WARMUP_CIRCUIT_FAIL_THRESHOLD, 3);
        assert_eq!(WARMUP_CIRCUIT_COOLDOWN_TICKS, 5);
        assert_eq!(WARMUP_YIELD_EVERY_N, 64);
    }

    #[test]
    fn upstream_health_serializes_camel_case() {
        let h = UpstreamHealth {
            upstream: "http://peer-a:8090".to_string(),
            error_streak: 4,
            last_good: Some(2),
            circuit: CircuitState::Open,
            skipped: true,
        };
        let json = serde_json::to_string(&h).unwrap();
        assert!(json.contains("\"upstream\""));
        assert!(json.contains("\"errorStreak\""));
        assert!(json.contains("\"lastGood\""));
        assert!(json.contains("\"circuit\":\"open\""));
        assert!(json.contains("\"skipped\":true"));
    }

    #[test]
    fn self_heal_event_serializes_camel_case() {
        let e = WarmupSelfHealEvent {
            event: "anti-self-partition",
            detail: "all upstreams circuit-open; proceeding least-bad".to_string(),
        };
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("\"event\":\"anti-self-partition\""));
        assert!(json.contains("\"detail\""));
    }

    #[test]
    fn warm_stream_health_skips_after_threshold() {
        let h = WarmStreamHealth::new(WARMUP_CIRCUIT_FAIL_THRESHOLD, WARMUP_CIRCUIT_COOLDOWN_TICKS);
        let url = "http://broken:8090";
        for tick in 0..WARMUP_CIRCUIT_FAIL_THRESHOLD as u64 {
            assert!(!h.should_skip(url, tick), "closed before threshold");
            h.record_outcome(url, false, tick);
        }
        // Now open: next pass within cooldown skips.
        assert!(h.should_skip(url, WARMUP_CIRCUIT_FAIL_THRESHOLD as u64));
    }

    #[test]
    fn warm_stream_health_snapshot_reports_fields() {
        let h = WarmStreamHealth::new(1, 5);
        h.record_outcome("http://a:8090", false, 0); // opens
        h.record_outcome("http://b:8090", true, 0); // good
        let mut snap = h.snapshot();
        snap.sort_by(|x, y| x.upstream.cmp(&y.upstream));
        assert_eq!(snap.len(), 2);
        assert_eq!(snap[0].circuit, CircuitState::Open);
        assert_eq!(snap[1].circuit, CircuitState::Closed);
        assert_eq!(snap[1].last_good, Some(0));
    }

    #[test]
    fn gate_filters_open_keeps_closed() {
        let h = WarmStreamHealth::new(1, 100);
        let urls = vec!["http://a".to_string(), "http://b".to_string()];
        h.record_outcome("http://a", false, 0); // a opens
        let gated = h.gate_upstreams(&urls, 1);
        assert_eq!(
            gated,
            vec!["http://b".to_string()],
            "open peer filtered, closed kept"
        );
    }

    #[test]
    fn gate_never_self_partitions_to_empty() {
        let h = WarmStreamHealth::new(1, 1_000_000);
        let urls = vec!["http://a".to_string(), "http://b".to_string()];
        // Open BOTH with different streaks: a worse (3 fails), b less-bad (1 fail).
        h.record_outcome("http://a", false, 0);
        h.record_outcome("http://a", false, 1);
        h.record_outcome("http://a", false, 2);
        h.record_outcome("http://b", false, 0);
        let gated = h.gate_upstreams(&urls, 3);
        assert_eq!(gated.len(), 1, "must NOT be empty when all open");
        assert_eq!(
            gated[0], "http://b",
            "least-bad (lowest error_streak) chosen"
        );
    }

    #[test]
    fn gate_empty_input_returns_empty() {
        let h = WarmStreamHealth::new(1, 5);
        assert!(h.gate_upstreams(&[], 0).is_empty());
    }

    #[test]
    fn yield_gate_fires_every_n() {
        fn should_yield(n: usize) -> bool {
            n.is_multiple_of(WARMUP_YIELD_EVERY_N)
        }
        assert!(should_yield(WARMUP_YIELD_EVERY_N));
        assert!(should_yield(WARMUP_YIELD_EVERY_N * 2));
        assert!(!should_yield(1));
        assert!(!should_yield(WARMUP_YIELD_EVERY_N - 1));
    }

    #[test]
    fn doorway_single_pass_frozen_tick_no_inpass_recovery() {
        // Pins the PRODUCTION wiring: spawn_stream_task runs ONE startup pass
        // with a constant tick=0 (warm_stream.rs). A breaker opened at tick 0
        // never satisfies `elapsed >= cooldown` within that pass, so the
        // cooldown/half-open recovery is intentionally inert in doorway — the
        // tick-independent is_open() early-bail + the budget guard protect
        // instead. (Cross-pass cooldown recovery is Plan B's concern, driven by
        // a real clock.) Every other CircuitBreaker test injects an ADVANCING
        // tick the doorway warm-up never produces; this one closes that gap.
        let h = WarmStreamHealth::new(1, WARMUP_CIRCUIT_COOLDOWN_TICKS);
        h.record_outcome("http://p:8090", false, 0); // opens at the frozen tick
        assert!(
            h.should_skip("http://p:8090", 0),
            "at the frozen production tick the opened breaker stays skipped (no recovery)"
        );
        // The observable signal that DOES work: error_streak + circuit state.
        let snap = h.snapshot();
        assert_eq!(snap[0].error_streak, 1);
        assert_eq!(snap[0].circuit, CircuitState::Open);
    }

    #[test]
    fn open_breaker_bails_inner_retry_early() {
        // Simulate the inner-loop decision: after K failures recorded, the
        // loop must observe is_open and stop, not keep retrying.
        let h = WarmStreamHealth::new(WARMUP_CIRCUIT_FAIL_THRESHOLD, 100);
        let url = "http://slowbroken:8090";
        let mut bailed_at = None;
        for attempt in 1..=MAX_WARMUP_RETRIES {
            h.record_outcome(url, false, 0);
            if h.should_skip(url, 0) {
                bailed_at = Some(attempt);
                break;
            }
        }
        assert_eq!(
            bailed_at,
            Some(WARMUP_CIRCUIT_FAIL_THRESHOLD),
            "must bail at K, not run all {MAX_WARMUP_RETRIES} retries"
        );
    }
}
