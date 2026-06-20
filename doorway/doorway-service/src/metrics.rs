//! Durable Prometheus app-metrics surface for doorway — the **complement** leg
//! of the design-decision toolkit (storage owns P0/P1; this is the P2 doorway
//! leg).
//!
//! Doorway is the porch: it observes the conductor only over the wire and never
//! introspects the conductor's memory, threads, or corpus. So every metric here
//! is a *doorway-local* signal storage **cannot** see — the watchdog wedge that
//! precedes a doorway self-kill, the conductor reconnect churn doorway drives,
//! how much load doorway sheds, and how its caches absorb reads. The cardinal
//! rule is **complement, never duplicate**: nothing prefixed `elohim_node_*`
//! (storage's per-node memory/corpus surface) belongs here; doorway metrics are
//! prefixed `doorway_*`, matching the existing tracing `counter=` field names so
//! a Loki log line and a Prometheus series carry the same identifier.
//!
//! Idiom mirrors `elohim-storage/src/metrics.rs` exactly: `lazy_static!` metric
//! statics registered into one process-wide [`Registry`], `TextEncoder` for the
//! exposition body, typed setters/inc helpers so call sites never import
//! `prometheus`. Unlike storage there is **no sampler** — doorway's signals are
//! event-driven (counters inc inline at their site; the session-duration
//! histogram observes a value already computed; `heartbeat_age` is derived at
//! scrape time in [`gather_text`]). No background task.
//!
//! Plan: `genesis/docs/superpowers/plans/2026-06-17-design-decision-toolkit-plan.md`
//! Handoff: `HANDOFF-2026-06-17-doorway-metrics.md`

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Once, OnceLock};
use std::time::Instant;

use lazy_static::lazy_static;
use prometheus::{
    Encoder, Histogram, HistogramOpts, IntCounter, IntCounterVec, IntGauge, Opts, Registry,
    TextEncoder,
};

// ── Reconnect-reason label values (M2). The close *cause*, a non-overlapping
//    axis from session *duration* (M3): a sub-threshold session in the duration
//    histogram is the accept-then-drop/auth-reject signature, so it never needs
//    its own reason. ──
/// Connect attempt refused before any session (TCP/connect error).
pub const REASON_CONNECT_REFUSED: &str = "connect_refused";
/// Conductor sent a WebSocket Close frame (graceful close; may carry a code).
pub const REASON_CLOSE_FRAME: &str = "close_frame";
/// WebSocket transport error / send failure mid-session.
pub const REASON_WS_ERROR: &str = "ws_error";
/// Owning handle dropped — the session ended for shutdown, not a fault.
pub const REASON_CHANNEL_CLOSED: &str = "channel_closed";

lazy_static! {
    /// The single process-wide registry the `/metrics` endpoint exposes.
    pub static ref REGISTRY: Registry = Registry::new();

    // ── M1: watchdog wedge — the 503-flap proximate trigger ───────────────────

    /// Times the liveness watchdog declared the MAIN runtime WEDGED (heartbeat
    /// stale past threshold). This is the proximate cause of the doorway
    /// self-kill restart — un-gettable from storage, which never sees doorway's
    /// own runtime park. Post-hoc record: during a *fatal* wedge `/metrics` on
    /// the main listener cannot answer, so the live wedge signal is the scrape
    /// target's `up == 0`; this counter shows the wedge after recovery.
    pub static ref WATCHDOG_WEDGED_TOTAL: IntCounter = IntCounter::new(
        "doorway_watchdog_wedged_total",
        "Times the liveness watchdog declared the MAIN runtime wedged (heartbeat stale).",
    )
    .unwrap();

    /// Age (ms) of the MAIN runtime liveness heartbeat at scrape time — derived
    /// in `gather_text()` from the same atomic the watchdog reads, NOT a sampled
    /// task. A rising value while still answering = a runtime parking but not yet
    /// fatally wedged. (A fatal wedge can't be captured here — the main listener
    /// is down; see `up == 0` above.)
    pub static ref HEARTBEAT_AGE_MS: IntGauge = IntGauge::new(
        "doorway_heartbeat_age_ms",
        "Age in ms of the MAIN runtime liveness heartbeat at scrape time.",
    )
    .unwrap();

    // ── M2: conductor reconnect churn — classifies reconnect-storm cause ──────

    /// Conductor↔doorway reconnect events, by cause — a session that ended OR a
    /// connect attempt that never became a session (`connect_refused`). Storage
    /// does not observe doorway's WS lifecycle. label: reason ∈
    /// {connect_refused, close_frame, ws_error, channel_closed}.
    pub static ref CONDUCTOR_RECONNECT_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "doorway_conductor_reconnect_total",
            "Conductor WS reconnect events by cause (session-end or connect failure).",
        ),
        &["reason"],
    )
    .unwrap();

    /// WebSocket close codes the conductor sent, when a Close frame carried one.
    /// Kept a SEPARATE axis from `reconnect_total{reason}` so the close *cause*
    /// and the close *code* never cross-multiply (the handoff's non-conflation
    /// discipline). label: code (the numeric WS close code as a string).
    pub static ref CONDUCTOR_CLOSE_CODE_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "doorway_conductor_close_code_total",
            "WebSocket close codes the conductor sent on graceful close.",
        ),
        &["code"],
    )
    .unwrap();

    // ── M3: session duration — the auth-reject vs idle-reap discriminator ─────

    /// Conductor WS session lifetime in seconds. Buckets straddle sub-second
    /// (auth-reject churn) and 10s (the stable-session threshold): a pile in the
    /// <1s buckets is accept-then-drop/auth-reject; a pile at the long end is
    /// healthy long-lived sessions reaped for other reasons. Session lifetime is
    /// a doorway-side fact storage cannot see.
    pub static ref CONDUCTOR_SESSION_DURATION_SECONDS: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "doorway_conductor_session_duration_seconds",
            "Conductor WS session lifetime in seconds.",
        )
        .buckets(vec![0.05, 0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0, 300.0]),
    )
    .unwrap();

    // ── M5: live session fan-out — the multiplication the RCA wants bounded ────

    /// Live conductor app-WS sessions across EVERY `ConductorConnection` —
    /// inc/dec live in `worker::conductor::run_session`, the convergence point
    /// for all of them: the per-conductor pool workers (the bulk — `worker_count`
    /// × conductors, the RCA's fan-out), plus the NATS processor and the
    /// import/seed client. The signal subscriber runs its OWN loop (not a
    /// `ConductorConnection`) and is NOT counted here. Storage cannot see
    /// doorway's session count.
    pub static ref CONDUCTOR_SESSIONS: IntGauge = IntGauge::new(
        "doorway_conductor_sessions",
        "Live conductor app-WS sessions across all ConductorConnection instances.",
    )
    .unwrap();

    // ── M4: how much doorway sheds, and how its caches absorb reads ───────────

    /// Tiered content resolution outcomes by source tier. The projection cache is
    /// doorway-resident: storage never sees a doorway projection hit (the request
    /// never reaches it). label: tier ∈ {projection, conductor, external}.
    pub static ref RESOLVE_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "doorway_resolve_total",
            "Tiered content resolution outcomes by source tier.",
        ),
        &["tier"],
    )
    .unwrap();

    /// Blob pantry (doorway's projection cache for blobs) outcomes. label:
    /// outcome ∈ {hit, miss, stocked, skipped}. The pantry is doorway-resident;
    /// a hit is served without ever reaching storage.
    pub static ref BLOB_PANTRY_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "doorway_blob_pantry_total",
            "Blob pantry (doorway projection cache) outcomes.",
        ),
        &["outcome"],
    )
    .unwrap();

    /// Requests shed because the per-upstream circuit breaker was OPEN (doorway
    /// refused the call before forwarding). Name matches the existing tracing
    /// `counter=` field. doorway-resident operational state.
    pub static ref UPSTREAM_BREAKER_OPEN_TOTAL: IntCounter = IntCounter::new(
        "doorway_upstream_breaker_open_total",
        "Requests shed because the per-upstream circuit breaker was open.",
    )
    .unwrap();

    /// Upstream backpressure (429/503 from storage) honored by surfacing a
    /// catching-up 503 to the client. Name matches the existing tracing
    /// `counter=` field.
    pub static ref UPSTREAM_BACKPRESSURE_HONORED_TOTAL: IntCounter = IntCounter::new(
        "doorway_upstream_backpressure_honored_total",
        "Upstream 429/503 backpressure honored by shedding to the client.",
    )
    .unwrap();

    /// Inbound requests shed by the global admission gate (semaphore at ceiling).
    /// Name matches the existing tracing `counter=` field. Storage never sees a
    /// doorway shed — the request is refused before forwarding.
    pub static ref ADMISSION_SHED_TOTAL: IntCounter = IntCounter::new(
        "doorway_admission_shed_total",
        "Inbound requests shed by the global admission gate (at ceiling).",
    )
    .unwrap();

    /// The configured inbound admission ceiling (DOORWAY_MAX_INFLIGHT). Doorway's
    /// own config — storage can't see it. Set once at boot; also the
    /// `maxInflight` source for the self_healing AdmissionView (one accessor,
    /// both surfaces — handoff §6 co-delivery).
    pub static ref INBOUND_MAX_INFLIGHT: IntGauge = IntGauge::new(
        "doorway_inbound_max_inflight",
        "Configured inbound admission ceiling (DOORWAY_MAX_INFLIGHT).",
    )
    .unwrap();

    // ── Membrane policy metrics ───────────────────────────────────────────────

    /// Membrane policy verdicts by outcome. label: verdict ∈ {allow, shape, challenge, deny}.
    /// Doorway-resident — storage never sees a membrane-denied request.
    pub static ref MEMBRANE_VERDICT_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "doorway_membrane_verdict_total",
            "Membrane policy verdicts by outcome (allow/shape/challenge/deny).",
        ),
        &["verdict"],
    )
    .unwrap();

    /// Active membrane bans (sources with a ban_until in the future).
    /// Updated on each `maybe_sweep` pass from EdgeGuardStore.
    pub static ref MEMBRANE_BANS_ACTIVE: IntGauge = IntGauge::new(
        "doorway_membrane_bans_active",
        "Number of sources currently under a membrane ban.",
    )
    .unwrap();
}

/// Boot-set handle the watchdog stamps: (`start`, `heartbeat`). `gather_text`
/// reads it to derive `heartbeat_age` at scrape time — the SAME atomic +
/// `Instant` the liveness watchdog reads (`spawn_liveness_heartbeat`), so the
/// two never disagree. Set once in `server::run`.
static HEARTBEAT: OnceLock<(Instant, Arc<AtomicU64>)> = OnceLock::new();

/// Register every doorway collector into [`REGISTRY`]. Idempotent (guarded by a
/// `Once`), so calling it more than once at boot is safe. Call exactly once
/// early in doorway startup; `/metrics` reads the registry thereafter.
pub fn register_all() {
    static REGISTERED: Once = Once::new();
    REGISTERED.call_once(|| {
        // register() errors only on duplicate/invalid collectors; the Once guard
        // already prevents duplicates, so a stray Err is non-fatal.
        let _ = REGISTRY.register(Box::new(WATCHDOG_WEDGED_TOTAL.clone()));
        let _ = REGISTRY.register(Box::new(HEARTBEAT_AGE_MS.clone()));
        let _ = REGISTRY.register(Box::new(CONDUCTOR_RECONNECT_TOTAL.clone()));
        let _ = REGISTRY.register(Box::new(CONDUCTOR_CLOSE_CODE_TOTAL.clone()));
        let _ = REGISTRY.register(Box::new(CONDUCTOR_SESSION_DURATION_SECONDS.clone()));
        let _ = REGISTRY.register(Box::new(CONDUCTOR_SESSIONS.clone()));
        let _ = REGISTRY.register(Box::new(RESOLVE_TOTAL.clone()));
        let _ = REGISTRY.register(Box::new(BLOB_PANTRY_TOTAL.clone()));
        let _ = REGISTRY.register(Box::new(UPSTREAM_BREAKER_OPEN_TOTAL.clone()));
        let _ = REGISTRY.register(Box::new(UPSTREAM_BACKPRESSURE_HONORED_TOTAL.clone()));
        let _ = REGISTRY.register(Box::new(ADMISSION_SHED_TOTAL.clone()));
        let _ = REGISTRY.register(Box::new(INBOUND_MAX_INFLIGHT.clone()));
        let _ = REGISTRY.register(Box::new(MEMBRANE_VERDICT_TOTAL.clone()));
        let _ = REGISTRY.register(Box::new(MEMBRANE_BANS_ACTIVE.clone()));
    });
}

/// Record the watchdog heartbeat handle so `gather_text` can derive
/// `heartbeat_age` at scrape time. Pass the SAME `start` + `heartbeat` given to
/// `spawn_liveness_heartbeat`. Idempotent: a second call is ignored.
pub fn set_heartbeat_handle(start: Instant, heartbeat: Arc<AtomicU64>) {
    let _ = HEARTBEAT.set((start, heartbeat));
}

/// Render the registry in Prometheus text exposition format (the `/metrics`
/// body). Derives the scrape-time `heartbeat_age` gauge first (no sampler).
pub fn gather_text() -> String {
    // Derive heartbeat_age at scrape time from the watchdog atomic — exactly as
    // `watchdog_liveness_response` computes it, so the gauge and the probe agree.
    if let Some((start, heartbeat)) = HEARTBEAT.get() {
        let last = heartbeat.load(Ordering::Relaxed);
        let age = (start.elapsed().as_millis() as u64).saturating_sub(last);
        HEARTBEAT_AGE_MS.set(age as i64);
    }

    let mut buf = Vec::new();
    let encoder = TextEncoder::new();
    let families = REGISTRY.gather();
    if encoder.encode(&families, &mut buf).is_err() {
        return String::new();
    }
    String::from_utf8(buf).unwrap_or_default()
}

// ── Typed helpers (callers never import `prometheus`) ──

/// M1: the watchdog declared the MAIN runtime wedged.
pub fn inc_watchdog_wedged() {
    WATCHDOG_WEDGED_TOTAL.inc();
}

/// M2: a conductor WS session ended; classify by close cause. Use the
/// `REASON_*` consts in this module.
pub fn inc_reconnect(reason: &str) {
    CONDUCTOR_RECONNECT_TOTAL.with_label_values(&[reason]).inc();
}

/// M2: a conductor Close frame carried a numeric close code.
pub fn inc_close_code(code: u16) {
    CONDUCTOR_CLOSE_CODE_TOTAL
        .with_label_values(&[&code.to_string()])
        .inc();
}

/// M3: observe a finished conductor session's lifetime (seconds).
pub fn observe_session_duration(secs: f64) {
    CONDUCTOR_SESSION_DURATION_SECONDS.observe(secs);
}

/// M5: a pool-worker conductor session became live.
pub fn inc_sessions() {
    CONDUCTOR_SESSIONS.inc();
}

/// M5: a pool-worker conductor session ended.
pub fn dec_sessions() {
    CONDUCTOR_SESSIONS.dec();
}

/// M4: a tiered resolution resolved at `tier` ∈ {projection, conductor, external}.
pub fn inc_resolve(tier: &str) {
    RESOLVE_TOTAL.with_label_values(&[tier]).inc();
}

/// M4: a blob pantry outcome ∈ {hit, miss, stocked, skipped}.
pub fn inc_blob_pantry(outcome: &str) {
    BLOB_PANTRY_TOTAL.with_label_values(&[outcome]).inc();
}

/// M4: a request was shed because the upstream circuit breaker was open.
pub fn inc_breaker_open() {
    UPSTREAM_BREAKER_OPEN_TOTAL.inc();
}

/// M4: upstream backpressure (429/503) was honored by shedding to the client.
pub fn inc_backpressure_honored() {
    UPSTREAM_BACKPRESSURE_HONORED_TOTAL.inc();
}

/// M4: an inbound request was shed by the global admission gate.
pub fn inc_admission_shed() {
    ADMISSION_SHED_TOTAL.inc();
}

/// Record the configured inbound admission ceiling (call once at boot).
pub fn set_inbound_max(max: usize) {
    INBOUND_MAX_INFLIGHT.set(max as i64);
}

/// The configured inbound admission ceiling (0 until `set_inbound_max`). The
/// `maxInflight` source for the self_healing AdmissionView.
pub fn inbound_max() -> i64 {
    INBOUND_MAX_INFLIGHT.get()
}

/// Current `doorway_admission_shed_total` value — the `shedTotal` source for
/// the self_healing AdmissionView (one home for the count; the counter IS the
/// shed atomic the handoff §6 calls for).
pub fn admission_shed_total() -> u64 {
    ADMISSION_SHED_TOTAL.get()
}

/// Membrane: record one verdict outcome ∈ {allow, shape, challenge, deny}.
pub fn inc_membrane_verdict(verdict: &str) {
    MEMBRANE_VERDICT_TOTAL.with_label_values(&[verdict]).inc();
}

/// Membrane: update the active-ban gauge (called from `EdgeGuardStore::maybe_sweep`).
pub fn set_membrane_bans_active(count: i64) {
    MEMBRANE_BANS_ACTIVE.set(count);
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests assert PRESENCE and per-call DELTAS, never absolute counter values:
    // every test in this binary shares one process-global REGISTRY and parallel
    // tests mutate it, so absolutes are flaky. (Storage's metrics.rs asserts
    // presence for the same reason.)

    #[test]
    fn register_all_idempotent_and_gathers_all_metrics() {
        register_all();
        register_all(); // Once-guarded — must not panic or double-register.

        // Touch each label-bearing collector so its series renders.
        inc_reconnect(REASON_CLOSE_FRAME);
        inc_close_code(1000);
        observe_session_duration(0.2);
        inc_resolve("projection");
        inc_blob_pantry("hit");
        inc_membrane_verdict("allow");

        let text = gather_text();
        for name in [
            "doorway_watchdog_wedged_total",
            "doorway_heartbeat_age_ms",
            "doorway_conductor_reconnect_total",
            "doorway_conductor_close_code_total",
            "doorway_conductor_session_duration_seconds",
            "doorway_conductor_sessions",
            "doorway_resolve_total",
            "doorway_blob_pantry_total",
            "doorway_upstream_breaker_open_total",
            "doorway_upstream_backpressure_honored_total",
            "doorway_admission_shed_total",
            "doorway_membrane_verdict_total",
            "doorway_membrane_bans_active",
        ] {
            assert!(text.contains(name), "missing metric {name}:\n{text}");
        }
        // Label rendering sanity (the classifier dimensions).
        assert!(text.contains("reason=\"close_frame\""), "{text}");
        assert!(text.contains("tier=\"projection\""), "{text}");
        // Complement discipline: never re-expose a storage per-node metric.
        assert!(
            !text.contains("elohim_node_"),
            "doorway must not duplicate storage's elohim_node_* surface:\n{text}"
        );
    }

    #[test]
    fn session_gauge_inc_dec_is_balanced() {
        register_all();
        let before = CONDUCTOR_SESSIONS.get();
        inc_sessions();
        assert_eq!(CONDUCTOR_SESSIONS.get() - before, 1, "inc raises by one");
        dec_sessions();
        assert_eq!(
            CONDUCTOR_SESSIONS.get(),
            before,
            "dec restores the baseline"
        );
    }

    #[test]
    fn reconnect_counter_increments_by_reason_delta() {
        register_all();
        let before = CONDUCTOR_RECONNECT_TOTAL
            .with_label_values(&[REASON_WS_ERROR])
            .get();
        inc_reconnect(REASON_WS_ERROR);
        let after = CONDUCTOR_RECONNECT_TOTAL
            .with_label_values(&[REASON_WS_ERROR])
            .get();
        assert_eq!(after - before, 1);
    }

    #[test]
    fn heartbeat_age_derived_at_scrape_time() {
        register_all();
        // Stamp a heartbeat "120ms ago": start in the past, last-stamp lagging.
        let start = Instant::now() - std::time::Duration::from_millis(500);
        let hb = Arc::new(AtomicU64::new(380)); // elapsed≈500, last=380 ⇒ age≈120
        set_heartbeat_handle(start, hb);
        let text = gather_text();
        assert!(
            text.contains("doorway_heartbeat_age_ms"),
            "heartbeat age gauge must render after a scrape:\n{text}"
        );
    }

    #[test]
    fn membrane_verdict_counter_increments_by_verdict_delta() {
        register_all();
        let before = MEMBRANE_VERDICT_TOTAL.with_label_values(&["deny"]).get();
        inc_membrane_verdict("deny");
        let after = MEMBRANE_VERDICT_TOTAL.with_label_values(&["deny"]).get();
        assert_eq!(after - before, 1);
    }

    #[test]
    fn membrane_bans_active_gauge_set() {
        register_all();
        set_membrane_bans_active(7);
        assert_eq!(MEMBRANE_BANS_ACTIVE.get(), 7);
        set_membrane_bans_active(0);
        assert_eq!(MEMBRANE_BANS_ACTIVE.get(), 0);
    }
}
