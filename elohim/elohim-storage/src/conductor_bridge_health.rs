//! Conductor bridge health — is the ZOME PATH actually alive?
//!
//! WHY THIS MODULE EXISTS. `/health` answered `200 {"status":"ok"}` for as long
//! as the process was up, and its `conductor` block reported only `mode`
//! (`embedded` | `external`) plus DNA hashes read out of a CACHED `CellId`.
//! Every one of those fields survives a conductor restart untouched, because
//! none of them ever crosses the websocket. So on a conductor-only restart
//! (storage keeps running) the node advertised itself healthy indefinitely
//! while every zome call died `Websocket closed: No connection`, `POST
//! /db/content` was accepted `201` and landed NULL-anchored, and nothing —
//! no probe, no status code, no log line beyond a 60s heartbeat warning —
//! said the truth-writing path was gone.
//!
//! This module is the honest observation. It is deliberately:
//!
//! * **Pure + injectable-clock.** Every decision is a total function of
//!   observations and a `now_ms`, so the whole policy is unit-testable with no
//!   conductor, no websocket and no sleeping.
//! * **Per-instance, not implicitly global.** [`bridge_health`] exposes ONE
//!   process-wide instance for production wiring, but every constructor is
//!   public so tests build their own and cannot race each other through shared
//!   mutable state (the parallel-test flake class documented in `CLAUDE.md`).
//! * **Evidence-shaped, not intention-shaped.** `Unknown` is a real state: a
//!   node that has not yet attempted a zome call has NOT observed a live path
//!   and must not claim one. It also must not cry dead — hence three states,
//!   not two.
//!
//! ## What counts as evidence
//!
//! A zome call that fails for a DOMAIN reason (`ZomeNotFound`, a wasm error, a
//! validation refusal) is proof the path is **LIVE** — the bytes crossed the
//! websocket and the conductor answered. Only a TRANSPORT failure is evidence
//! the path is dead. And a conductor-admission shed is evidence of NOTHING:
//! nothing was dispatched, the conductor never saw the call (see
//! `conductor_admission::ADMISSION_SHED_MARKER`). Collapsing those three into
//! a boolean is exactly how a shed once reached the wire as a bare 500.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Wall-clock milliseconds since the epoch, saturating at 0 on a pre-epoch
/// clock. Only used to stamp observations; all policy is a function of
/// differences, so a clock step degrades an age, never a status.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// The state of the conductor zome path as this node has actually observed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZomePathStatus {
    /// The most recent evidence is a zome call that crossed the websocket.
    Live,
    /// The most recent evidence is a TRANSPORT failure — the websocket is gone.
    Dead,
    /// No evidence either way yet (fresh boot, or only admission sheds so far).
    Unknown,
}

impl ZomePathStatus {
    /// The wire spelling carried on `/health` and `/health/serving`.
    pub fn as_str(self) -> &'static str {
        match self {
            ZomePathStatus::Live => "live",
            ZomePathStatus::Dead => "dead",
            ZomePathStatus::Unknown => "unknown",
        }
    }
}

/// What one zome-call error proves about the path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZomeObservation {
    /// The conductor answered (even if it answered "no") — the path is live.
    PathLive,
    /// The websocket is gone — the path is dead.
    PathDead,
    /// Nothing was dispatched (admission shed): proves nothing either way.
    NoEvidence,
}

/// An immutable reading of [`BridgeHealth`], safe to serialize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BridgeHealthSnapshot {
    pub status: ZomePathStatus,
    /// Age of the last observed-live evidence, in whole seconds.
    pub last_success_age_secs: Option<u64>,
    /// Age of the last transport failure, in whole seconds.
    pub last_failure_age_secs: Option<u64>,
    /// Consecutive transport failures since the last live observation.
    pub consecutive_failures: u32,
    /// How many times the supervisor has re-minted the bridge.
    pub reconnects: u64,
}

impl BridgeHealthSnapshot {
    /// Can this node serve its truth-writing path right now?
    ///
    /// `Unknown` answers `true` deliberately: a node with no evidence has not
    /// earned a red. Only observed death fails the probe.
    pub fn serving_ok(&self) -> bool {
        self.status != ZomePathStatus::Dead
    }
}

/// Observation counters for one node's conductor bridge.
///
/// Cheap enough to touch on the zome-call hot path: four relaxed atomics, no
/// lock, no allocation, no clock read on the success path beyond one
/// `SystemTime::now`.
#[derive(Debug, Default)]
pub struct BridgeHealth {
    /// Epoch-ms of the last live observation; 0 = never. AGE only.
    last_success_ms: AtomicU64,
    /// Epoch-ms of the last transport failure; 0 = never. AGE only.
    last_failure_ms: AtomicU64,
    /// Monotonic sequence stamped on each observation. The STATUS is decided by
    /// these, never by the millisecond stamps: a failure and the recovery that
    /// follows it land inside the same millisecond routinely (`record_failure`
    /// then `record_success` in a supervisor tick measured identical ms), and
    /// comparing equal stamps silently reports the OLDER observation. Ordering
    /// is the thing that matters, so ordering is what we store.
    seq: AtomicU64,
    last_success_seq: AtomicU64,
    last_failure_seq: AtomicU64,
    consecutive_failures: AtomicU32,
    reconnects: AtomicU64,
}

impl BridgeHealth {
    pub fn new() -> Self {
        Self::default()
    }

    /// Next observation sequence. `SeqCst` so two threads observing the bridge
    /// concurrently agree on which observation was last.
    fn next_seq(&self) -> u64 {
        self.seq.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Record evidence the path is live, stamped at `at_ms`.
    pub fn record_success_at(&self, at_ms: u64) {
        let seq = self.next_seq();
        self.last_success_ms.store(at_ms, Ordering::Relaxed);
        self.last_success_seq.store(seq, Ordering::SeqCst);
        self.consecutive_failures.store(0, Ordering::Relaxed);
    }

    /// Record evidence the path is dead, stamped at `at_ms`.
    pub fn record_failure_at(&self, at_ms: u64) {
        let seq = self.next_seq();
        self.last_failure_ms.store(at_ms, Ordering::Relaxed);
        self.last_failure_seq.store(seq, Ordering::SeqCst);
        self.consecutive_failures.fetch_add(1, Ordering::Relaxed);
    }

    /// Record evidence the path is live, stamped now.
    pub fn record_success(&self) {
        self.record_success_at(now_ms());
    }

    /// Record evidence the path is dead, stamped now.
    pub fn record_failure(&self) {
        self.record_failure_at(now_ms());
    }

    /// Count one supervisor-driven re-mint of the bridge.
    pub fn record_reconnect(&self) {
        self.reconnects.fetch_add(1, Ordering::Relaxed);
    }

    /// Fold the counters into a status as of `now_ms`.
    ///
    /// Total by construction: the status is decided by the OBSERVATION ORDER
    /// (`seq`), never by the millisecond stamps, so neither a same-millisecond
    /// pair nor a clock that steps backwards can invert it. The ms stamps feed
    /// only the ages, which saturate at 0 rather than wrapping.
    pub fn snapshot_at(&self, now_ms: u64) -> BridgeHealthSnapshot {
        let success = self.last_success_ms.load(Ordering::Relaxed);
        let failure = self.last_failure_ms.load(Ordering::Relaxed);
        let success_seq = self.last_success_seq.load(Ordering::SeqCst);
        let failure_seq = self.last_failure_seq.load(Ordering::SeqCst);
        let status = match (success_seq, failure_seq) {
            (0, 0) => ZomePathStatus::Unknown,
            (_, 0) => ZomePathStatus::Live,
            (0, _) => ZomePathStatus::Dead,
            (s, f) if f > s => ZomePathStatus::Dead,
            _ => ZomePathStatus::Live,
        };
        let age = |stamp: u64| {
            if stamp == 0 {
                None
            } else {
                Some(now_ms.saturating_sub(stamp) / 1000)
            }
        };
        BridgeHealthSnapshot {
            status,
            last_success_age_secs: age(success),
            last_failure_age_secs: age(failure),
            consecutive_failures: self.consecutive_failures.load(Ordering::Relaxed),
            reconnects: self.reconnects.load(Ordering::Relaxed),
        }
    }

    /// [`Self::snapshot_at`] against the wall clock.
    pub fn snapshot(&self) -> BridgeHealthSnapshot {
        self.snapshot_at(now_ms())
    }

    /// Fold a zome-call error into this observer, classifying it first.
    pub fn observe_zome_error(&self, msg: &str) {
        match classify_zome_error(msg) {
            ZomeObservation::PathLive => self.record_success(),
            ZomeObservation::PathDead => self.record_failure(),
            ZomeObservation::NoEvidence => {}
        }
    }
}

/// The ONE process-wide observer the running node wires into `HcClient`, the
/// bridge supervisor and `/health`.
///
/// Deliberately a `OnceLock` singleton rather than an env-read or a threaded
/// handle: the zome-call site and the HTTP health site are ~30 call sites apart
/// and threading an `Arc` through all of them would be a signature change with
/// no truth-layer benefit. Tests never touch this — they build their own
/// [`BridgeHealth`] so parallel tests cannot poison one another.
pub fn bridge_health() -> &'static BridgeHealth {
    static HEALTH: OnceLock<BridgeHealth> = OnceLock::new();
    HEALTH.get_or_init(BridgeHealth::new)
}

/// Does this error text mean the WEBSOCKET is gone (as opposed to the conductor
/// answering "no")?
///
/// Matched case-insensitively on substrings because the message reaching us is
/// already a `format!`-composed chain: `StorageError::Conductor("Zome call
/// failed: {holochain_client error}")`. Classifying on the string is the same
/// contract `conductor_admission::is_admission_shed` uses, for the same reason:
/// the underlying crate does not expose a transport-vs-domain discriminant.
pub fn is_transport_dead(msg: &str) -> bool {
    let m = msg.to_ascii_lowercase();
    const DEAD_MARKERS: [&str; 7] = [
        "websocket closed",
        "no connection",
        "connection reset",
        "connection refused",
        "connection closed",
        "broken pipe",
        "connect failed",
    ];
    DEAD_MARKERS.iter().any(|marker| m.contains(marker))
}

/// Classify one zome-call error into what it proves about the path.
pub fn classify_zome_error(msg: &str) -> ZomeObservation {
    if msg.contains(crate::conductor_admission::ADMISSION_SHED_MARKER) {
        return ZomeObservation::NoEvidence;
    }
    if is_transport_dead(msg) {
        return ZomeObservation::PathDead;
    }
    ZomeObservation::PathLive
}

/// The `conductor` block `/health` and `/health/serving` both render.
///
/// One builder, two surfaces — so the body a person reads and the status code a
/// probe reads can never disagree about the same node (the drift that let
/// `/health` say `ok` while every read 503'd).
pub fn health_block(mode: &str, snap: &BridgeHealthSnapshot) -> serde_json::Value {
    serde_json::json!({
        "mode": mode,
        "zomePath": snap.status.as_str(),
        "lastZomeCallAgeSecs": snap.last_success_age_secs,
        "lastZomeFailureAgeSecs": snap.last_failure_age_secs,
        "consecutiveFailures": snap.consecutive_failures,
        "bridgeReconnects": snap.reconnects,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: u64 = 1_700_000_000_000;

    #[test]
    fn fresh_observer_is_unknown_and_serves() {
        let h = BridgeHealth::new();
        let snap = h.snapshot_at(T0);
        assert_eq!(snap.status, ZomePathStatus::Unknown);
        assert_eq!(snap.last_success_age_secs, None);
        assert_eq!(snap.last_failure_age_secs, None);
        // A node with no evidence has not earned a red.
        assert!(snap.serving_ok());
    }

    #[test]
    fn success_then_failure_reads_dead() {
        let h = BridgeHealth::new();
        h.record_success_at(T0);
        h.record_failure_at(T0 + 5_000);
        let snap = h.snapshot_at(T0 + 10_000);
        assert_eq!(snap.status, ZomePathStatus::Dead);
        assert!(!snap.serving_ok(), "a dead zome path must fail the probe");
        assert_eq!(snap.last_success_age_secs, Some(10));
        assert_eq!(snap.last_failure_age_secs, Some(5));
        assert_eq!(snap.consecutive_failures, 1);
    }

    #[test]
    fn failure_then_success_reads_live_and_clears_the_streak() {
        // This is the recovery assertion: the supervisor re-mints the bridge,
        // the next call lands, and the node must go green WITHOUT a restart.
        let h = BridgeHealth::new();
        h.record_failure_at(T0);
        h.record_failure_at(T0 + 1_000);
        assert_eq!(h.snapshot_at(T0 + 1_000).consecutive_failures, 2);
        h.record_success_at(T0 + 2_000);
        let snap = h.snapshot_at(T0 + 2_000);
        assert_eq!(snap.status, ZomePathStatus::Live);
        assert!(snap.serving_ok());
        assert_eq!(snap.consecutive_failures, 0);
    }

    #[test]
    fn only_failures_reads_dead() {
        let h = BridgeHealth::new();
        h.record_failure_at(T0);
        assert_eq!(h.snapshot_at(T0).status, ZomePathStatus::Dead);
    }

    #[test]
    fn reconnect_counter_is_surfaced() {
        let h = BridgeHealth::new();
        h.record_reconnect();
        h.record_reconnect();
        assert_eq!(h.snapshot_at(T0).reconnects, 2);
    }

    #[test]
    fn a_backwards_clock_saturates_the_age_and_never_inverts_the_status() {
        let h = BridgeHealth::new();
        h.record_success_at(T0 + 10_000);
        let snap = h.snapshot_at(T0);
        assert_eq!(snap.last_success_age_secs, Some(0));
        assert_eq!(snap.status, ZomePathStatus::Live);
    }

    // ---- error classification -------------------------------------------

    #[test]
    fn the_observed_live_failure_string_classifies_dead() {
        // Verbatim from the reproduction (throwaway conductor restarted under a
        // running storage, 2026-08-21): this exact string must read as DEAD or
        // the whole supervisor never arms.
        let msg = "Zome call failed: Websocket error: Websocket closed: No connection";
        assert!(is_transport_dead(msg));
        assert_eq!(classify_zome_error(msg), ZomeObservation::PathDead);
    }

    #[test]
    fn the_observed_ping_failure_string_classifies_dead() {
        let msg = "Conductor ping failed: Websocket error: Websocket closed: No connection";
        assert_eq!(classify_zome_error(msg), ZomeObservation::PathDead);
    }

    #[test]
    fn a_domain_error_proves_the_path_is_live() {
        // The conductor ANSWERED — the bytes crossed the websocket. Treating
        // this as death would flap the node red on ordinary refusals.
        for msg in [
            "Zome call failed: ZomeNotFound: mishpat",
            "Zome call failed: Wasm error while working with Ribosome: Guest(\"no record\")",
            "Zome call failed: Unauthorized",
        ] {
            assert!(
                !is_transport_dead(msg),
                "{msg} must not read as transport-dead"
            );
            assert_eq!(classify_zome_error(msg), ZomeObservation::PathLive, "{msg}");
        }
    }

    #[test]
    fn an_admission_shed_proves_nothing() {
        // Nothing was dispatched; the conductor never saw the call. Recording
        // this as a live path would launder a shed into evidence of health.
        let msg = format!(
            "{}: no conductor permit for content_store within 250ms",
            crate::conductor_admission::ADMISSION_SHED_MARKER
        );
        assert_eq!(classify_zome_error(&msg), ZomeObservation::NoEvidence);

        let h = BridgeHealth::new();
        h.observe_zome_error(&msg);
        assert_eq!(
            h.snapshot_at(T0).status,
            ZomePathStatus::Unknown,
            "a shed must not move the status"
        );
    }

    #[test]
    fn same_millisecond_observations_are_ordered_by_sequence_not_clock() {
        // REGRESSION (caught by the first run of the suite below): a supervisor
        // tick records a failure and the recovery that follows inside the SAME
        // millisecond routinely. Deciding the status by comparing equal ms
        // stamps reports the older observation — a recovered node reading dead,
        // or worse, a dead node reading live.
        let h = BridgeHealth::new();
        h.record_success_at(T0);
        h.record_failure_at(T0); // identical stamp, later observation
        assert_eq!(h.snapshot_at(T0).status, ZomePathStatus::Dead);

        let h = BridgeHealth::new();
        h.record_failure_at(T0);
        h.record_success_at(T0); // identical stamp, later observation
        assert_eq!(h.snapshot_at(T0).status, ZomePathStatus::Live);
    }

    #[test]
    fn observe_zome_error_routes_each_class() {
        let h = BridgeHealth::new();
        h.observe_zome_error("Zome call failed: ZomeNotFound: mishpat");
        assert_eq!(h.snapshot().status, ZomePathStatus::Live);
        h.observe_zome_error("Zome call failed: Websocket closed: No connection");
        assert_eq!(h.snapshot().status, ZomePathStatus::Dead);
    }

    // ---- the wire shape --------------------------------------------------

    #[test]
    fn health_block_carries_the_zome_path_verdict() {
        let h = BridgeHealth::new();
        h.record_success_at(T0);
        h.record_failure_at(T0 + 1_000);
        let block = health_block("external", &h.snapshot_at(T0 + 61_000));
        assert_eq!(block["mode"], "external");
        assert_eq!(block["zomePath"], "dead");
        assert_eq!(block["lastZomeCallAgeSecs"], 61);
        assert_eq!(block["lastZomeFailureAgeSecs"], 60);
        assert_eq!(block["consecutiveFailures"], 1);
    }

    #[test]
    fn health_block_nulls_the_age_before_any_call() {
        let block = health_block("embedded", &BridgeHealth::new().snapshot_at(T0));
        assert_eq!(block["zomePath"], "unknown");
        assert!(block["lastZomeCallAgeSecs"].is_null());
    }
}
