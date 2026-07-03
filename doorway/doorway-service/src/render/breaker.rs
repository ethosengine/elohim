//! SSR render-failure circuit breaker.
//!
//! # Why
//!
//! When every render attempt fails identically (e.g. the Angular server bundle
//! carries a Node-builtin import the runtime cannot resolve), each page request
//! still burns a doomed V8 render before falling back — and the failure is not
//! cached, so every request pays the full render-attempt latency first (~12s
//! TTFB observed on alpha). After `FAILURE_THRESHOLD` consecutive render errors
//! this breaker OPENS for a cooldown window: requests skip the SSR diversion
//! entirely and go straight to the fallback (tagged `x-ssr-skipped:
//! error-backoff`), so a systemic render failure degrades to today's
//! projected-bundle serving at full speed instead of a doomed-render tax. A
//! single successful render CLOSES it again.
//!
//! # One global breaker (not per-route)
//!
//! The doorway drives a SINGLE V8 renderer isolate, so a systemic render failure
//! fails EVERY route identically — per-route granularity would just multiply
//! identical trips with no added signal. If the renderer ever becomes a pool of
//! independently-failing bundles, revisit for per-bundle breakers.
//!
//! # State
//!
//! Two atomics carry all state, so a shared `&AppState` actuates the breaker with
//! no locking: a consecutive-failure count and the unix-ms timestamp of the most
//! recent failure. The open/closed decision is a pure function of those two plus
//! the current time, unit-tested below without any clock.

use std::sync::atomic::{AtomicI64, AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Consecutive render errors that trip the breaker OPEN.
pub const FAILURE_THRESHOLD: u32 = 3;

/// How long the breaker stays open (skipping SSR) after it trips, in milliseconds.
pub const COOLDOWN_MS: i64 = 5 * 60 * 1000;

/// Global SSR render-failure circuit breaker. See the module docs.
#[derive(Debug, Default)]
pub struct SsrRenderBreaker {
    consecutive_failures: AtomicU32,
    /// Unix-ms of the most recent render failure. `0` = none recorded.
    last_failure_ms: AtomicI64,
}

impl SsrRenderBreaker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Is the breaker OPEN right now (skip SSR, serve the fallback)?
    pub fn is_open(&self) -> bool {
        self.is_open_at(now_ms())
    }

    /// Is the breaker OPEN at `now_ms`? Injectable clock for deterministic tests.
    pub fn is_open_at(&self, now_ms: i64) -> bool {
        breaker_is_open(
            self.consecutive_failures.load(Ordering::Relaxed),
            self.last_failure_ms.load(Ordering::Relaxed),
            now_ms,
            FAILURE_THRESHOLD,
            COOLDOWN_MS,
        )
    }

    /// Record a render failure. Returns `true` exactly on the trip transition
    /// (the failure that first reaches `FAILURE_THRESHOLD`) so the caller can emit
    /// ONE warn when the breaker opens, not one per subsequent skipped request.
    pub fn record_failure(&self) -> bool {
        self.record_failure_at(now_ms())
    }

    /// [`record_failure`](Self::record_failure) with an injectable clock.
    pub fn record_failure_at(&self, now_ms: i64) -> bool {
        self.last_failure_ms.store(now_ms, Ordering::Relaxed);
        let prev = self.consecutive_failures.fetch_add(1, Ordering::Relaxed);
        opens_on_failure(prev, FAILURE_THRESHOLD)
    }

    /// Record a successful render — resets the breaker to CLOSED.
    pub fn record_success(&self) {
        self.consecutive_failures.store(0, Ordering::Relaxed);
        self.last_failure_ms.store(0, Ordering::Relaxed);
    }
}

/// Pure breaker-open decision. OPEN iff the failure count has reached the
/// threshold AND the cooldown window since the last failure has not yet elapsed.
/// Once the window elapses the breaker auto-closes so a single "half-open" trial
/// render is allowed; if that trial fails the window reopens (last-failure bumps),
/// if it succeeds the count resets via [`SsrRenderBreaker::record_success`].
fn breaker_is_open(
    failures: u32,
    last_failure_ms: i64,
    now_ms: i64,
    threshold: u32,
    cooldown_ms: i64,
) -> bool {
    failures >= threshold
        && last_failure_ms != 0
        && now_ms.saturating_sub(last_failure_ms) < cooldown_ms
}

/// Pure trip-transition test: does recording a failure at `prev` consecutive
/// failures move the breaker across the threshold for the FIRST time? (Only the
/// crossing failure returns true, so the "breaker opened" warn fires once per
/// outage, not on every subsequent failure while it stays open.)
fn opens_on_failure(prev_failures: u32, threshold: u32) -> bool {
    prev_failures + 1 == threshold
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_until_threshold_then_opens() {
        let b = SsrRenderBreaker::new();
        // Two failures: still CLOSED (threshold is 3), and neither is the trip.
        assert!(!b.record_failure_at(1_000), "1st failure is not the trip");
        assert!(!b.is_open_at(1_000), "closed after 1 failure");
        assert!(!b.record_failure_at(1_000), "2nd failure is not the trip");
        assert!(!b.is_open_at(1_000), "closed after 2 failures");
        // Third failure trips it OPEN and returns true exactly once.
        assert!(b.record_failure_at(1_000), "3rd failure is the trip");
        assert!(b.is_open_at(1_000), "open after threshold reached");
    }

    #[test]
    fn trip_warn_fires_only_once_while_open() {
        let b = SsrRenderBreaker::new();
        b.record_failure_at(1_000);
        b.record_failure_at(1_000);
        assert!(
            b.record_failure_at(1_000),
            "trip transition returns true once"
        );
        // Further failures while already open do NOT re-trigger the warn signal.
        assert!(
            !b.record_failure_at(1_000),
            "4th failure is not a fresh trip"
        );
        assert!(
            !b.record_failure_at(1_000),
            "5th failure is not a fresh trip"
        );
    }

    #[test]
    fn closes_after_cooldown_elapses() {
        let b = SsrRenderBreaker::new();
        b.record_failure_at(1_000);
        b.record_failure_at(1_000);
        b.record_failure_at(1_000);
        assert!(b.is_open_at(1_000), "open at trip time");
        assert!(
            b.is_open_at(1_000 + COOLDOWN_MS - 1),
            "still open just before the cooldown elapses"
        );
        assert!(
            !b.is_open_at(1_000 + COOLDOWN_MS),
            "closed (half-open trial allowed) once the cooldown has elapsed"
        );
    }

    #[test]
    fn success_resets_to_closed() {
        let b = SsrRenderBreaker::new();
        b.record_failure_at(1_000);
        b.record_failure_at(1_000);
        b.record_failure_at(1_000);
        assert!(b.is_open_at(1_000), "open after threshold");
        b.record_success();
        assert!(
            !b.is_open_at(1_000),
            "a successful render closes the breaker"
        );
        // And the counter is genuinely reset — it takes a full threshold again.
        assert!(
            !b.record_failure_at(2_000),
            "post-reset 1st failure is not the trip"
        );
        assert!(
            !b.record_failure_at(2_000),
            "post-reset 2nd failure is not the trip"
        );
        assert!(
            b.record_failure_at(2_000),
            "post-reset 3rd failure trips again"
        );
    }

    #[test]
    fn pure_decision_helpers() {
        // breaker_is_open: needs threshold reached, a real last-failure, and an
        // unexpired window.
        assert!(!breaker_is_open(2, 100, 100, 3, 1_000), "below threshold");
        assert!(breaker_is_open(3, 100, 500, 3, 1_000), "in window");
        assert!(!breaker_is_open(3, 100, 1_100, 3, 1_000), "window elapsed");
        assert!(!breaker_is_open(3, 0, 100, 3, 1_000), "no failure recorded");
        // opens_on_failure: only the crossing failure returns true.
        assert!(!opens_on_failure(0, 3));
        assert!(!opens_on_failure(1, 3));
        assert!(opens_on_failure(2, 3), "prev=2 -> new=3 == threshold");
        assert!(!opens_on_failure(3, 3), "already open, not a fresh trip");
    }
}
