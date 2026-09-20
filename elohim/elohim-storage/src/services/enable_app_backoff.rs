//! ENABLE-APP BACKOFF — how often a node may ask the conductor to enable an
//! installed app it has observed DISABLED.
//!
//! ## The incident this bounds (2026-09-18 → 09-20, elohim-alpha)
//!
//! The installed hApp behind both public doorways, and on eve, was `Disabled`
//! for 38+ hours. Every zome call on every supervised role answered
//! `CellDisabled(...)`; storage kept serving its own SQLite projection, so the
//! node looked alive, and nothing in the crate ever asked the conductor to
//! enable it again. The cure is an `enable_app` — and the moment a cure is
//! automatic, the question stops being "does it heal" and becomes "how often
//! may it try".
//!
//! ## The bound
//!
//! bounded-work: time budget = one attempt per role per
//! [`enable_backoff`]`(attempt)` window — 60s doubling to a 1h cap, so a
//! permanently-disabled app costs at most ~24 admin calls a day and ~11 in the
//! first hour. Memory budget = one `AttemptRecord` per SUPERVISED role (4,
//! fixed at compile time in [`crate::hc_client_registry::SUPERVISED_ROLES`]) —
//! the map is keyed by role, never by cell or app id, so it cannot grow with
//! the corpus. NO loop and NO I/O live here: this is a pure decision plus a
//! bounded map, consulted by the supervisor's existing
//! `BRIDGE_PROBE_INTERVAL` tick, which stays the only pacing authority.
//!
//! Three exits, all automatic:
//!
//! 1. **The app runs.** [`EnableLedger::note_running`] clears the role's record
//!    entirely, so a later relapse starts again at 60s rather than at the cap.
//! 2. **The window elapses.** The next probe after the current delay attempts
//!    again; the ladder is capped, never terminal, because an app disabled by
//!    a transient conductor fault must not need a human to come back.
//! 3. **The process restarts.** Deliberately process-local (like
//!    [`crate::services::heal_backoff`] and
//!    [`crate::services::reanchor_backoff`]): a restart plausibly changed what
//!    the conductor holds, and one extra `enable_app` per role per boot is a
//!    cheaper mistake than a node that remembers a hold through a repair.
//!
//! ## What it is NOT
//!
//! It never decides WHETHER enabling is legal — that is
//! [`crate::happ_manager::decide_boot_action`] (terminal states) and
//! [`crate::closed_chain_fence`] (a sealed chain). This only decides whether it
//! is time to ask again.

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Delay after the FIRST attempt. Comfortably longer than the supervisor's
/// 20s probe interval, so a single disabled observation cannot produce two
/// attempts in the same minute.
pub const ENABLE_BACKOFF_BASE: Duration = Duration::from_secs(60);

/// Hard ceiling on the delay: one hour. A disabled app that has resisted an
/// hour of attempts needs an operator, but the node keeps asking anyway —
/// cheaply — because the conductor's refusal is the diagnosis and a silent
/// give-up is what produced the 38-hour gap in the first place.
pub const ENABLE_BACKOFF_CAP: Duration = Duration::from_secs(3_600);

/// How long to wait after `attempt` (1-based) before trying `enable_app` again.
///
/// 60s → 120 → 240 → 480 → 960 → 1920 → 3600 (cap), held at the cap forever
/// after. Pure, total and saturating — the caller runs it with an ever-growing
/// attempt count for as long as the app stays disabled.
pub fn enable_backoff(attempt: u32) -> Duration {
    let shift = attempt.saturating_sub(1).min(63);
    let secs = ENABLE_BACKOFF_BASE
        .as_secs()
        .saturating_mul(1u64.checked_shl(shift).unwrap_or(u64::MAX));
    Duration::from_secs(secs.min(ENABLE_BACKOFF_CAP.as_secs()))
}

/// One role's attempt history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AttemptRecord {
    /// How many enable attempts this process has made while the app stayed
    /// disabled. 1-based once the first attempt lands.
    attempts: u32,
    /// When the most recent attempt was made.
    at: Instant,
}

/// Per-role enable-attempt ledger.
///
/// Constructed with nothing, so every test drives a real one and no test has to
/// touch the process-wide singleton — the parallel-test-flake discipline
/// [`crate::conductor_bridge_health::bridge_health`] documents.
#[derive(Debug, Default)]
pub struct EnableLedger {
    roles: Mutex<BTreeMap<String, AttemptRecord>>,
}

impl EnableLedger {
    pub fn new() -> Self {
        Self::default()
    }

    /// May an `enable_app` be attempted for `role` as of `now`?
    ///
    /// `true` on the first observation (no record), then only once the delay
    /// for the attempts already made has elapsed. A poisoned lock answers
    /// `false` — a bounded cure that cannot read its own bound must not run
    /// unbounded.
    pub fn should_attempt_at(&self, role: &str, now: Instant) -> bool {
        let Ok(guard) = self.roles.lock() else {
            return false;
        };
        match guard.get(role) {
            None => true,
            Some(record) => now.duration_since(record.at) >= enable_backoff(record.attempts),
        }
    }

    /// [`Self::should_attempt_at`] against the monotonic clock.
    pub fn should_attempt(&self, role: &str) -> bool {
        self.should_attempt_at(role, Instant::now())
    }

    /// Record that an attempt was made for `role` at `now`, advancing the
    /// ladder. Called whether the attempt succeeded or failed: a successful
    /// `enable_app` that does not actually start the app must not buy a free
    /// retry, and [`Self::note_running`] is what clears the ladder.
    pub fn note_attempt_at(&self, role: &str, now: Instant) {
        let Ok(mut guard) = self.roles.lock() else {
            return;
        };
        let attempts = guard.get(role).map_or(0, |r| r.attempts).saturating_add(1);
        guard.insert(role.to_string(), AttemptRecord { attempts, at: now });
    }

    /// [`Self::note_attempt_at`] against the monotonic clock.
    pub fn note_attempt(&self, role: &str) {
        self.note_attempt_at(role, Instant::now());
    }

    /// The app behind `role` is RUNNING — forget everything, so a later relapse
    /// is met at 60s rather than at whatever the ladder had climbed to.
    pub fn note_running(&self, role: &str) {
        if let Ok(mut guard) = self.roles.lock() {
            guard.remove(role);
        }
    }

    /// How many attempts have been made for `role` since it was last running.
    /// Observability only.
    pub fn attempts(&self, role: &str) -> u32 {
        self.roles
            .lock()
            .ok()
            .and_then(|g| g.get(role).map(|r| r.attempts))
            .unwrap_or(0)
    }
}

/// The ONE process-wide ledger the bridge supervisor consults.
pub fn enable_ledger() -> &'static EnableLedger {
    static LEDGER: OnceLock<EnableLedger> = OnceLock::new();
    LEDGER.get_or_init(EnableLedger::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ladder_doubles_from_a_minute_to_an_hour_and_stops_there() {
        assert_eq!(enable_backoff(1), Duration::from_secs(60));
        assert_eq!(enable_backoff(2), Duration::from_secs(120));
        assert_eq!(enable_backoff(3), Duration::from_secs(240));
        assert_eq!(enable_backoff(4), Duration::from_secs(480));
        assert_eq!(enable_backoff(5), Duration::from_secs(960));
        assert_eq!(enable_backoff(6), Duration::from_secs(1_920));
        // 60 * 2^6 = 3840 > the cap.
        assert_eq!(enable_backoff(7), ENABLE_BACKOFF_CAP);
        assert_eq!(enable_backoff(8), ENABLE_BACKOFF_CAP);
    }

    #[test]
    fn the_ladder_never_overflows_on_a_permanently_disabled_app() {
        // An app disabled for a week is thousands of probes; the saturating
        // shift must hold the cap rather than panic.
        assert_eq!(enable_backoff(64), ENABLE_BACKOFF_CAP);
        assert_eq!(enable_backoff(u32::MAX), ENABLE_BACKOFF_CAP);
    }

    /// The bound, stated as the thing it forbids: a probe every 20s must NOT
    /// become an `enable_app` every 20s.
    #[test]
    fn enable_attempts_back_off_and_are_capped() {
        let ledger = EnableLedger::new();
        let t0 = Instant::now();

        // First observation of a disabled app: attempt immediately.
        assert!(ledger.should_attempt_at("lamad", t0));
        ledger.note_attempt_at("lamad", t0);

        // Every supervisor tick INSIDE the 60s window must be refused (the
        // probe fires at 20s; the cure must not).
        for tick in 1..=2 {
            assert!(
                !ledger.should_attempt_at("lamad", t0 + Duration::from_secs(20 * tick)),
                "tick {tick}: a 20s probe must not drive a 20s enable loop"
            );
        }
        // 60s after the attempt — the boundary — the window is open again.
        assert!(ledger.should_attempt_at("lamad", t0 + Duration::from_secs(60)));

        // Climb the ladder: each window is shut for its whole length and opens
        // exactly at its end, and each one is twice the last until the cap.
        let mut now = t0; // one attempt recorded above, so the window is 60s
        let mut widths = Vec::new();
        for _ in 0..8 {
            let window = enable_backoff(ledger.attempts("lamad"));
            widths.push(window.as_secs());
            assert!(
                !ledger.should_attempt_at("lamad", now + window - Duration::from_secs(1)),
                "shut for the whole {}s window",
                window.as_secs()
            );
            now += window;
            assert!(
                ledger.should_attempt_at("lamad", now),
                "reopens exactly at the end of the {}s window",
                window.as_secs()
            );
            ledger.note_attempt_at("lamad", now);
        }
        assert_eq!(
            widths,
            vec![60, 120, 240, 480, 960, 1_920, 3_600, 3_600],
            "doubling from a minute, capped at an hour"
        );

        // Saturated — and still not terminal. A node whose app stays disabled
        // keeps asking, hourly, forever: the conductor's refusal IS the
        // diagnosis, and a silent give-up is what produced the 38-hour gap.
        assert_eq!(
            enable_backoff(ledger.attempts("lamad")),
            ENABLE_BACKOFF_CAP,
            "the ladder saturates rather than growing without bound"
        );
        assert!(ledger.should_attempt_at("lamad", now + ENABLE_BACKOFF_CAP));
    }

    #[test]
    fn and_reset_when_the_app_runs() {
        let ledger = EnableLedger::new();
        let t0 = Instant::now();
        for step in 0..5 {
            let at = t0 + Duration::from_secs(step * 4_000);
            ledger.note_attempt_at("imagodei", at);
        }
        assert_eq!(ledger.attempts("imagodei"), 5);
        let climbed = t0 + Duration::from_secs(4 * 4_000);
        assert!(
            !ledger.should_attempt_at("imagodei", climbed + Duration::from_secs(600)),
            "ten minutes into an hour-long window, the door is shut"
        );

        // The app comes back. The ladder is forgotten, not merely paused.
        ledger.note_running("imagodei");
        assert_eq!(ledger.attempts("imagodei"), 0);
        assert!(
            ledger.should_attempt_at("imagodei", climbed + Duration::from_secs(600)),
            "a relapse after a recovery is met immediately, not an hour later"
        );
    }

    #[test]
    fn roles_ladder_independently() {
        // One dead role must not silence the cure for another.
        let ledger = EnableLedger::new();
        let t0 = Instant::now();
        ledger.note_attempt_at("lamad", t0);
        assert!(!ledger.should_attempt_at("lamad", t0));
        assert!(
            ledger.should_attempt_at("node_registry", t0),
            "a role with no history is unblocked"
        );
    }
}
