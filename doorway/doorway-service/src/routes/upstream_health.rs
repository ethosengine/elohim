//! Per-upstream circuit breakers for the storage-proxy path.
//!
//! Cat C node-local OPERATIONAL state (no DHT entry, no table). Wraps the
//! shared `elohim_compute::CircuitBreaker` keyed by storage endpoint URL.
//! The breaker is tick-injected; this map feeds it a wall-clock tick
//! (`started.elapsed().as_secs()`) so `cooldown_ticks` == cooldown seconds.

use std::cell::Cell;
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::Instant;

use elohim_compute::{CircuitBreaker, CircuitState};
use tracing::warn;

/// Failed upstream outcomes *within [`UPSTREAM_CIRCUIT_FAIL_WINDOW_SECS`]*
/// before a circuit opens.
///
/// This is a WINDOW threshold, not a streak threshold. See
/// [`UPSTREAM_CIRCUIT_FAIL_WINDOW_SECS`] for why.
pub const UPSTREAM_CIRCUIT_FAIL_THRESHOLD: u32 = 3;
/// How recent failures must be to count toward opening a circuit.
///
/// WHY A WINDOW AND NOT A STREAK. The shared `CircuitBreaker` opens on N
/// *consecutive* failures with no time bound, so a streak accumulated across
/// minutes — or across a single upstream stall — is indistinguishable from a
/// burst. Measured on the local mesh 2026-08-21 19:38: one ~60s conductor stall
/// on matthew produced five connect/timeout failures inside a minute, opened the
/// circuit, and kept shedding for minutes afterwards; storage answered the very
/// same path in 1.6 ms a minute later, `/health` read `degraded`, and saga ch01
/// went red against a peer that was already healthy. A stall is ONE fault, and a
/// breaker that treats its fan-out as N independent faults over-sheds by exactly
/// that fan-out.
///
/// A window makes the open condition falsifiable in time: `THRESHOLD` failures
/// must land inside `WINDOW` seconds of each other. Failures spread wider than
/// the window age out of the tally and never open the circuit on their own — the
/// upstream is slow-and-recovering, not down. Same threshold, bounded evidence.
pub const UPSTREAM_CIRCUIT_FAIL_WINDOW_SECS: u64 = 20;
/// Seconds a circuit stays open before a half-open trial.
pub const UPSTREAM_CIRCUIT_COOLDOWN_SECS: u64 = 30;
/// Belt-and-braces self-heal: how many cooldowns a *consumed* half-open trial
/// may sit without a recorded outcome before the gate re-admits a trial anyway.
///
/// [`BreakerTrial`] is the primary guarantee (every consumed trial records an
/// outcome, even on drop). This constant bounds the blast radius of a FUTURE
/// caller that reaches past the guard to the raw [`UpstreamBreakers::is_open`]
/// and forgets to record: instead of shedding until process restart, the
/// endpoint recovers within `MULTIPLIER × cooldown`.
pub const STALE_HALFOPEN_COOLDOWN_MULTIPLIER: u64 = 4;

/// One endpoint's breaker plus the bookkeeping the stale-half-open self-heal
/// needs (the shared `CircuitBreaker` does not track *when* a trial was
/// admitted, only when the circuit opened).
struct Entry {
    /// The state machine. Constructed with an inner threshold of ONE so it opens
    /// on the single failure [`Entry::fail`] forwards once the *window* verdict
    /// says so — the window, not the inner breaker, owns the open condition.
    cb: CircuitBreaker,
    /// Tick at which the currently-outstanding half-open trial was admitted.
    /// `Some` only while state is `HalfOpen`; cleared whenever an outcome lands.
    halfopen_since: Option<u64>,
    /// Ticks of the failures still inside the window, oldest first. Pruned on
    /// every failure; cleared by a success or by opening the circuit.
    failure_ticks: VecDeque<u64>,
    /// Consecutive recorded failures, unbounded in time. Reported by
    /// [`UpstreamBreakers::snapshot`]; tracked HERE rather than read off the
    /// inner breaker because the inner breaker now only ever sees the one
    /// forwarded failure, so its own streak counter would read 1 forever.
    error_streak: u32,
}

impl Entry {
    fn new(cooldown_ticks: u64) -> Self {
        Self {
            // Threshold 1: the window decides WHEN to open, this decides HOW.
            cb: CircuitBreaker::new(1, cooldown_ticks),
            halfopen_since: None,
            failure_ticks: VecDeque::new(),
            error_streak: 0,
        }
    }

    /// One failed outcome at `tick`. Opens the circuit only when `threshold`
    /// failures fall inside `window_ticks` of each other.
    fn fail(&mut self, tick: u64, window_ticks: u64, threshold: u32) {
        self.error_streak = self.error_streak.saturating_add(1);
        self.halfopen_since = None;
        match self.cb.state() {
            // A failed half-open TRIAL re-opens immediately and unconditionally:
            // the trial is itself the probe, so one failure is complete evidence
            // and no window applies. (This is also the path `abandon_trial`
            // takes for a cancelled trial.)
            CircuitState::HalfOpen => {
                self.cb.record_outcome(false, tick);
                self.failure_ticks.clear();
            }
            // Already shedding — an outcome recorded by a call admitted before
            // the circuit opened. Count the streak, but never refresh
            // `opened_at`: a late straggler must not extend the cooldown.
            CircuitState::Open => {}
            CircuitState::Closed => {
                self.failure_ticks.push_back(tick);
                while let Some(&oldest) = self.failure_ticks.front() {
                    if tick.saturating_sub(oldest) > window_ticks {
                        self.failure_ticks.pop_front();
                    } else {
                        break;
                    }
                }
                if self.failure_ticks.len() as u32 >= threshold {
                    // Inner threshold is 1, so this single forwarded failure
                    // opens the circuit with `opened_at = tick`.
                    self.cb.record_outcome(false, tick);
                    self.failure_ticks.clear();
                }
            }
        }
    }

    /// One successful outcome at `tick`: closes the circuit and empties the
    /// window — recovery is not partial credit.
    fn succeed(&mut self, tick: u64) {
        self.error_streak = 0;
        self.halfopen_since = None;
        self.failure_ticks.clear();
        self.cb.record_outcome(true, tick);
    }

    /// How many failures are still inside the window as of `tick` (read-only —
    /// does not prune). Observability for `/admin/self-healing`.
    fn failures_in_window(&self, tick: u64, window_ticks: u64) -> u32 {
        self.failure_ticks
            .iter()
            .filter(|&&t| tick.saturating_sub(t) <= window_ticks)
            .count() as u32
    }
}

/// What the gate decided for one call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Admission {
    /// Circuit open (or a trial already outstanding) — shed without calling.
    Shed,
    /// Circuit closed — proceed normally; no trial was consumed.
    Closed,
    /// The one half-open trial was consumed by THIS call. An outcome MUST be
    /// recorded or the circuit latches (see [`BreakerTrial`]).
    Trial,
}

/// Per-endpoint breaker map for the storage proxy.
pub struct UpstreamBreakers {
    breakers: Mutex<HashMap<String, Entry>>,
    started: Instant,
    fail_threshold: u32,
    cooldown_ticks: u64,
    /// Seconds within which `fail_threshold` failures must land to open a
    /// circuit. See [`UPSTREAM_CIRCUIT_FAIL_WINDOW_SECS`].
    fail_window_ticks: u64,
}

impl UpstreamBreakers {
    /// Threshold + cooldown with the default failure window
    /// ([`UPSTREAM_CIRCUIT_FAIL_WINDOW_SECS`]).
    pub fn new(fail_threshold: u32, cooldown_secs: u64) -> Self {
        Self::with_window(
            fail_threshold,
            cooldown_secs,
            UPSTREAM_CIRCUIT_FAIL_WINDOW_SECS,
        )
    }

    /// Full policy: `fail_threshold` failures within `window_secs` open the
    /// circuit, which then sheds for `cooldown_secs` before one half-open trial.
    pub fn with_window(fail_threshold: u32, cooldown_secs: u64, window_secs: u64) -> Self {
        Self {
            breakers: Mutex::new(HashMap::new()),
            started: Instant::now(),
            fail_threshold: fail_threshold.max(1),
            cooldown_ticks: cooldown_secs,
            fail_window_ticks: window_secs,
        }
    }

    /// The live open-condition policy, for `/admin/self-healing` and
    /// `/status.json`. Read from the SAME fields `record_at` decides with, so an
    /// operator reading the window can never be reading a different number from
    /// the one that opened the circuit (C7).
    pub fn policy(&self) -> BreakerPolicy {
        BreakerPolicy {
            fail_threshold: self.fail_threshold,
            fail_window_secs: self.fail_window_ticks,
            cooldown_secs: self.cooldown_ticks,
        }
    }

    fn tick(&self) -> u64 {
        self.started.elapsed().as_secs()
    }

    /// Ticks a consumed-but-unrecorded half-open trial may persist before the
    /// gate re-admits one anyway. Never zero (a zero-cooldown breaker would
    /// otherwise self-heal on the very next call and mask the latch).
    fn stale_halfopen_ticks(&self) -> u64 {
        self.cooldown_ticks
            .saturating_mul(STALE_HALFOPEN_COOLDOWN_MULTIPLIER)
            .max(1)
    }

    /// The one gate. Advances Open→HalfOpen when the cooldown has elapsed
    /// (admitting exactly one trial) and re-admits a trial when an outstanding
    /// one has gone stale.
    fn admit(&self, endpoint: &str, tick: u64) -> Admission {
        let cooldown_ticks = self.cooldown_ticks;
        let stale_ticks = self.stale_halfopen_ticks();
        let mut map = self.breakers.lock().unwrap();
        let entry = map
            .entry(endpoint.to_string())
            .or_insert_with(|| Entry::new(cooldown_ticks));

        if entry.cb.state() == CircuitState::HalfOpen {
            // A trial is already outstanding. Normally: shed until its outcome
            // lands. If it has been outstanding for `stale_ticks`, whoever
            // consumed it is never going to record — re-arm and admit another
            // rather than shedding forever.
            let since = entry.halfopen_since.unwrap_or(tick);
            if tick.saturating_sub(since) >= stale_ticks {
                warn!(
                    target: "upstream_shed",
                    counter = "doorway_upstream_stale_halfopen_readmit_total",
                    endpoint = %endpoint,
                    outstanding_secs = tick.saturating_sub(since),
                    "half-open trial outstanding with no recorded outcome — re-admitting a trial (a caller skipped record())"
                );
                entry.halfopen_since = Some(tick);
                return Admission::Trial;
            }
            return Admission::Shed;
        }

        if entry.cb.should_skip(tick) {
            return Admission::Shed;
        }
        if entry.cb.state() == CircuitState::HalfOpen {
            // should_skip just advanced Open→HalfOpen: THIS call holds the trial.
            entry.halfopen_since = Some(tick);
            Admission::Trial
        } else {
            Admission::Closed
        }
    }

    /// Read-only availability query for a *planning* caller: true when a call
    /// to `endpoint` would be shed right now. It NEVER mutates — it does not
    /// advance Open→HalfOpen, and it never admits or consumes a half-open
    /// trial.
    ///
    /// Use this to DECIDE (serve warm? attempt a fetch?). Use
    /// [`UpstreamBreakers::begin`] to CALL.
    ///
    /// Deliberately conservative: an Open circuit whose cooldown has already
    /// elapsed still reads as "would shed", even though the next gate call
    /// would admit a trial. A planner that guessed "available" here would only
    /// go on to lose the gate race behind it; a planner that serves from its
    /// warm cache instead has avoided a doomed upstream read. The guarded
    /// caller does the advancing — that is the whole division of labour.
    ///
    /// WHY THIS EXISTS. The obvious-looking `is_open` is a GATE, not a read: it
    /// consumes the one half-open trial and hands it to a caller with no guard,
    /// so no outcome is ever recorded. Two `/`-path planners called it, and
    /// because the breaker map is keyed by ENDPOINT the resulting stuck
    /// half-open shed EVERY route on that peer — including `/db/content` reads
    /// that answer in tens of milliseconds. Fixed 2026-07-21 (f5e22baa2),
    /// reintroduced 2026-08-18 (f0b908660), re-fixed here with a read that
    /// cannot steal.
    pub fn would_shed(&self, endpoint: &str) -> bool {
        let map = self.breakers.lock().unwrap();
        map.get(endpoint)
            .is_some_and(|entry| entry.cb.state() != CircuitState::Closed)
    }

    /// TEST-ONLY gate primitive: true if a call should be SHED.
    ///
    /// **This is not a read.** It advances Open→HalfOpen when the cooldown has
    /// elapsed and ADMITS (consumes) the one half-open trial, handing it to a
    /// caller with no guard and no `Drop` — so if that caller never records an
    /// outcome, the breaker sheds every route on the endpoint until a stale
    /// re-admit, which the same caller then steals again.
    ///
    /// Production must use [`UpstreamBreakers::begin`] (gate + RAII guard that
    /// records on every terminal path, including a dropped future) or
    /// [`UpstreamBreakers::would_shed`] (a true read). This is `cfg(test)`
    /// precisely so the footgun cannot be reached from production a third time:
    /// the unit tests below already asserted the latch behaviour and still did
    /// not stop two call sites from reintroducing it, because nothing tested
    /// that no PRODUCTION site calls it. A compile error is that missing rail.
    #[cfg(test)]
    pub fn is_open_admitting_trial(&self, endpoint: &str) -> bool {
        self.admit(endpoint, self.tick()) == Admission::Shed
    }

    /// Gate + trial guard in one: `None` means SHED (do not call the upstream);
    /// `Some(trial)` means proceed. The guard records exactly one outcome —
    /// explicitly via [`BreakerTrial::record`], or, if the request future is
    /// dropped before any terminal path runs, via its `Drop` impl.
    ///
    /// This is the structural answer to the wedge class documented in
    /// `halfopen_without_record_deadlocks_forever`: with the guard there is no
    /// terminal path — sync, `?`-propagated, panicking, or *cancelled* — that
    /// can leave a consumed trial unresolved.
    pub fn begin(&self, endpoint: &str) -> Option<BreakerTrial<'_>> {
        let consumed_trial = match self.admit(endpoint, self.tick()) {
            Admission::Shed => return None,
            Admission::Closed => false,
            Admission::Trial => true,
        };
        Some(BreakerTrial {
            breakers: self,
            endpoint: endpoint.to_string(),
            consumed_trial,
            done: Cell::new(false),
        })
    }

    /// Record an outcome for `endpoint` (ok=false counts toward opening).
    pub fn record(&self, endpoint: &str, ok: bool) {
        self.record_at(endpoint, ok, self.tick());
    }

    fn record_at(&self, endpoint: &str, ok: bool, tick: u64) {
        let fail_threshold = self.fail_threshold;
        let cooldown_ticks = self.cooldown_ticks;
        let window_ticks = self.fail_window_ticks;
        let mut map = self.breakers.lock().unwrap();
        let entry = map
            .entry(endpoint.to_string())
            .or_insert_with(|| Entry::new(cooldown_ticks));
        if ok {
            entry.succeed(tick);
        } else {
            entry.fail(tick, window_ticks, fail_threshold);
        }
    }

    /// A consumed half-open trial ended with NO outcome — the request future was
    /// dropped (client disconnect / task cancellation / panic) before any
    /// terminal path ran. Restore the pre-trial shape: re-open with a FRESH
    /// cooldown so the next cooldown re-admits a trial, instead of latching
    /// HalfOpen forever.
    ///
    /// A cancelled request is not evidence the upstream is healthy, so this
    /// does count as one failed outcome (error_streak +1) — but only for a
    /// trial we actually consumed. A cancelled request on a CLOSED circuit is
    /// never penalised (the guard's `Drop` doesn't call this).
    fn abandon_trial(&self, endpoint: &str) {
        let tick = self.tick();
        let mut map = self.breakers.lock().unwrap();
        if let Some(entry) = map.get_mut(endpoint) {
            if entry.cb.state() == CircuitState::HalfOpen {
                // HalfOpen + failure → Open with opened_at = now. Routed through
                // `fail` so the abandoned trial is counted on the same streak
                // ledger every other outcome uses (the HalfOpen arm there is
                // unconditional — no window applies to a failed trial).
                entry.fail(tick, self.fail_window_ticks, self.fail_threshold);
            }
        }
    }

    /// Read-only snapshot of every known endpoint's breaker — the accessor the
    /// self_healing UpstreamView needs (handoff §6 co-delivery). Uses `state()`,
    /// NOT `should_skip()`, so it never admits a half-open trial as a side
    /// effect of being observed.
    pub fn snapshot(&self) -> Vec<BreakerSnapshot> {
        let tick = self.tick();
        let window_ticks = self.fail_window_ticks;
        let map = self.breakers.lock().unwrap();
        map.iter()
            .map(|(endpoint, entry)| {
                let cb = &entry.cb;
                // `skipped` answers "is this endpoint shedding right now?".
                // HalfOpen reported FALSE, which read as reassuring on
                // /admin/self-healing during the 2026-08-20 incident — but a
                // HalfOpen circuit has its one trial outstanding and sheds
                // every OTHER caller, so the honest answer is true.
                let (circuit, skipped) = match cb.state() {
                    CircuitState::Closed => ("closed", false),
                    CircuitState::HalfOpen => ("half-open", true),
                    CircuitState::Open => ("open", true),
                };
                BreakerSnapshot {
                    endpoint: endpoint.clone(),
                    circuit,
                    error_streak: entry.error_streak,
                    recent_failures: entry.failures_in_window(tick, window_ticks),
                    skipped,
                }
            })
            .collect()
    }
}

/// RAII guard for one admitted upstream call.
///
/// Held from the gate ([`UpstreamBreakers::begin`]) across the upstream await
/// to the response. Exactly one outcome reaches the breaker:
///
/// * [`BreakerTrial::record`] on any terminal path (first call wins), or
/// * `Drop` without a record — the request future was cancelled. If this guard
///   consumed the half-open trial, the circuit re-opens with a fresh cooldown
///   (recovery in ≤ one cooldown) instead of latching HalfOpen until restart.
///
/// A cancelled call on a CLOSED circuit records nothing: a client disconnect is
/// not upstream evidence and must never trip a healthy breaker.
pub struct BreakerTrial<'a> {
    breakers: &'a UpstreamBreakers,
    endpoint: String,
    /// True when THIS guard consumed the one half-open trial.
    consumed_trial: bool,
    done: Cell<bool>,
}

impl BreakerTrial<'_> {
    /// Record this call's outcome. Idempotent — the first call wins, so a
    /// terminal path that records twice cannot double-count.
    pub fn record(&self, ok: bool) {
        if self.done.replace(true) {
            return;
        }
        self.breakers.record(&self.endpoint, ok);
    }

    /// True when this guard holds the one half-open trial (test observability).
    #[cfg(test)]
    pub fn consumed_trial(&self) -> bool {
        self.consumed_trial
    }
}

impl Drop for BreakerTrial<'_> {
    fn drop(&mut self) {
        if self.done.get() || !self.consumed_trial {
            return;
        }
        warn!(
            target: "upstream_shed",
            counter = "doorway_upstream_trial_abandoned_total",
            endpoint = %self.endpoint,
            "half-open trial abandoned with no outcome (request cancelled) — re-opening with a fresh cooldown"
        );
        self.breakers.abandon_trial(&self.endpoint);
    }
}

/// A read-only point-in-time view of one upstream's breaker, for the
/// self_healing read model. (last-good time is not tracked by the breaker, so
/// the view's `lastGood` stays null until that's added upstream.)
pub struct BreakerSnapshot {
    pub endpoint: String,
    /// "closed" | "half-open" | "open"
    pub circuit: &'static str,
    /// Consecutive recorded failures, unbounded in time (reset by any success).
    pub error_streak: u32,
    /// Failures still inside the open-window as of this observation. THIS is
    /// the number measured against `BreakerPolicy::fail_threshold`; the streak
    /// above is the older, time-blind count kept for continuity.
    pub recent_failures: u32,
    /// True when the circuit is OPEN (currently shedding, no trial admitted).
    pub skipped: bool,
}

/// The live open-condition policy: N failures within T seconds open a circuit,
/// which then sheds for `cooldown_secs`. Rendered on `/admin/self-healing` and
/// `/status.json` so the number an operator reads is the number the gate used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreakerPolicy {
    pub fail_threshold: u32,
    pub fail_window_secs: u64,
    pub cooldown_secs: u64,
}

impl Default for UpstreamBreakers {
    fn default() -> Self {
        Self::new(
            UPSTREAM_CIRCUIT_FAIL_THRESHOLD,
            UPSTREAM_CIRCUIT_COOLDOWN_SECS,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_after_threshold_then_sheds() {
        let b = UpstreamBreakers::new(3, 1_000_000); // huge cooldown so it stays open
        let ep = "http://broken:8090";
        assert!(!b.is_open_admitting_trial(ep), "closed on first sight");
        b.record(ep, false);
        b.record(ep, false);
        assert!(!b.is_open_admitting_trial(ep), "2 < 3: still closed");
        b.record(ep, false);
        assert!(b.is_open_admitting_trial(ep), "3rd failure opens -> shed");
    }

    #[test]
    fn success_keeps_closed() {
        let b = UpstreamBreakers::new(3, 30);
        let ep = "http://healthy:8090";
        for _ in 0..10 {
            b.record(ep, true);
        }
        assert!(!b.is_open_admitting_trial(ep));
    }

    // ---------------------------------------------------------------------
    // The rail that was missing on 2026-08-18. `halfopen_without_record_
    // deadlocks_forever` (below) already asserted the LATCH, and two production
    // call sites still reintroduced it, because nothing asserted that the
    // planner's availability query is non-consuming. These do.
    // ---------------------------------------------------------------------

    #[test]
    fn would_shed_never_consumes_the_half_open_trial() {
        // Zero cooldown so the trial is available immediately — same harness
        // convention as `halfopen_without_record_deadlocks_forever` (there is
        // no wall-clock fast-forward here; the tick-injected variant below
        // covers the cooldown boundary).
        let b = UpstreamBreakers::new(1, 0);
        let ep = "http://peer:8090";
        b.record(ep, false); // -> Open, with a trial immediately available
        assert_eq!(circuit_of(&b, ep), "open");

        // A planner asks a hundred times. It must steal nothing.
        for _ in 0..100 {
            assert!(b.would_shed(ep), "an open circuit reads as shedding");
        }
        assert_eq!(
            circuit_of(&b, ep),
            "open",
            "would_shed must NOT advance Open->HalfOpen: the trial is still unclaimed"
        );

        // ...so the guarded caller behind it still gets the trial. Under the
        // old `is_open` planner this returned None and the breaker latched.
        let trial = b
            .begin(ep)
            .expect("the trial survived the planner and reached the guarded caller");
        trial.record(true);
        assert_eq!(
            circuit_of(&b, ep),
            "closed",
            "the guarded caller's recorded success closes the circuit"
        );
    }

    #[test]
    fn would_shed_does_not_advance_across_the_cooldown_boundary() {
        // The tick-injected version: `admit` is the mutating gate, `would_shed`
        // is the read. Crossing the cooldown must be the GATE's doing, never
        // the planner's.
        let b = UpstreamBreakers::new(1, 30);
        let ep = "http://peer:8090";
        b.record(ep, false); // opens at tick ~0

        for _ in 0..50 {
            assert!(b.would_shed(ep));
        }
        // The gate, asked at a tick past the cooldown, still finds the trial
        // unconsumed — proof the 50 reads above advanced nothing.
        assert_eq!(
            b.admit(ep, 31),
            Admission::Trial,
            "the cooldown-crossing trial was never stolen by would_shed"
        );
    }

    #[test]
    fn would_shed_is_true_while_a_trial_is_outstanding() {
        // A HalfOpen circuit sheds every caller except the one holding the
        // trial, so a planner must read it as unavailable and serve warm.
        let b = UpstreamBreakers::new(1, 0);
        let ep = "http://peer:8090";
        b.record(ep, false);
        let _outstanding = b.begin(ep).expect("cooldown 0 admits a trial at once");
        assert_eq!(circuit_of(&b, ep), "half-open");
        assert!(
            b.would_shed(ep),
            "half-open with an outstanding trial sheds everyone else"
        );
    }

    #[test]
    fn would_shed_is_false_for_an_unknown_or_healthy_endpoint() {
        let b = UpstreamBreakers::new(3, 30);
        assert!(!b.would_shed("http://never-seen:8090"), "no breaker yet");
        b.record("http://fine:8090", true);
        assert!(!b.would_shed("http://fine:8090"));
    }

    #[test]
    fn snapshot_reports_half_open_as_skipping() {
        // /admin/self-healing showed `circuit: half-open, skipped: false` during
        // the 2026-08-20 elohim.host incident, which read as "not shedding"
        // while every route on the peer was in fact shedding.
        let b = UpstreamBreakers::new(1, 0);
        let ep = "http://peer:8090";
        b.record(ep, false);
        let _outstanding = b.begin(ep).expect("admits a trial");
        let snap = b
            .snapshot()
            .into_iter()
            .find(|s| s.endpoint == ep)
            .expect("snapshot carries the endpoint");
        assert_eq!(snap.circuit, "half-open");
        assert!(
            snap.skipped,
            "a half-open circuit with an outstanding trial IS skipping calls"
        );
    }

    #[test]
    fn distinct_endpoints_isolated() {
        let b = UpstreamBreakers::new(1, 1_000_000);
        b.record("http://a", false); // a opens
        assert!(b.is_open_admitting_trial("http://a"));
        assert!(!b.is_open_admitting_trial("http://b"), "b unaffected by a");
    }

    /// Non-consuming state lookup for one endpoint via `snapshot()` (never
    /// admits a half-open trial as a side effect — unlike `is_open()`).
    fn circuit_of(b: &UpstreamBreakers, ep: &str) -> &'static str {
        b.snapshot()
            .into_iter()
            .find(|s| s.endpoint == ep)
            .map(|s| s.circuit)
            .expect("endpoint must have a breaker by now")
    }

    #[test]
    fn halfopen_without_record_deadlocks_forever() {
        // Documents the bug class this discipline guards against: whoever
        // calls is_open() and gets a half-open trial admitted (`false`) MUST
        // eventually call record() with the outcome. If nothing ever records
        // an outcome, should_skip()/is_open() latches HalfOpen PERMANENTLY —
        // no further trial is ever admitted, no cooldown ever re-opens the
        // gate, and the endpoint sheds every request until a process
        // restart. This is exactly what happened live: `compose_render_with_
        // shell` called `is_open()` (consuming the trial) but had no
        // `record()` on any terminal path.
        //
        // The latch is bounded (not literally forever) only by the
        // belt-and-braces stale-half-open re-admit — `STALE_HALFOPEN_COOLDOWN_
        // MULTIPLIER × cooldown`, exercised by
        // `stale_halfopen_readmits_a_trial`. Within that window the shed is
        // total, which is why `begin()`/`BreakerTrial` (not this raw API) is
        // what every caller must use — see
        // `abandoned_trial_reopens_instead_of_latching`.
        //
        // Zero cooldown so the half-open transition happens on the very next
        // is_open() call — this harness has no wall-clock fast-forward, but
        // the state machine's latch behavior is identical for any cooldown
        // length (see
        // `elohim_compute::peers::circuit_should_skip_open_until_cooldown_then_halfopen_one_trial`
        // for the timed-cooldown version of the same one-trial admission).
        let b = UpstreamBreakers::new(1, 0);
        let ep = "http://consume-without-record:8090";
        b.record(ep, false); // opens
        assert_eq!(circuit_of(&b, ep), "open");
        // The first is_open() call after the (zero) cooldown IS the
        // half-open-admitting call: it returns false ("not open" -> caller
        // may proceed) and consumes the one trial.
        assert!(
            !b.is_open_admitting_trial(ep),
            "cooldown elapsed: half-open admits one trial"
        );
        assert_eq!(circuit_of(&b, ep), "half-open");
        // Every subsequent is_open() call — with NO record() in between —
        // reports open (shed) forever. No amount of further calling or time
        // passing recovers it; only a recorded outcome can.
        for _ in 0..50 {
            assert!(
                b.is_open_admitting_trial(ep),
                "half-open trial already consumed and never recorded: permanent shed"
            );
        }
        assert_eq!(
            circuit_of(&b, ep),
            "half-open",
            "never advances past half-open without a recorded outcome"
        );
    }

    #[test]
    fn halfopen_record_false_reopens_then_cooldown_readmits_a_trial() {
        // The healthy discipline (what the compose_render_with_shell fix
        // restores): a consumer that DOES record the trial's outcome never
        // wedges. record(false) re-opens the circuit (a normal Open, not a
        // permanent half-open latch); because the cooldown is already
        // elapsed, the NEXT is_open() call admits a fresh trial rather than
        // shedding forever; recording success this time closes it.
        let b = UpstreamBreakers::new(1, 0); // zero cooldown: immediate half-open
        let ep = "http://records-correctly:8090";
        b.record(ep, false); // opens
        assert!(
            !b.is_open_admitting_trial(ep),
            "cooldown elapsed: admits first trial"
        );
        assert_eq!(circuit_of(&b, ep), "half-open");
        b.record(ep, false); // the trial's outcome: failure -> re-opens
        assert_eq!(
            circuit_of(&b, ep),
            "open",
            "recorded failure re-opens as a normal Open circuit, not a stuck half-open"
        );
        // Unlike the deadlocked case above, this endpoint recovers: the next
        // is_open() call (cooldown already elapsed) admits a fresh trial
        // instead of shedding forever.
        assert!(
            !b.is_open_admitting_trial(ep),
            "cooldown elapsed again: admits a fresh trial"
        );
        b.record(ep, true); // this time the trial succeeds
        assert_eq!(circuit_of(&b, ep), "closed");
        assert!(
            !b.is_open_admitting_trial(ep),
            "recorded success closes the circuit"
        );
    }

    #[test]
    fn abandoned_trial_reopens_instead_of_latching() {
        // The live 2026-08-01 wedge, at unit scale: a caller takes the
        // half-open trial and its request future is DROPPED before any
        // terminal path runs (client disconnect / task cancellation), so no
        // `record()` call ever executes. With the raw `is_open()` API that
        // latches consumed-HalfOpen permanently (see
        // `halfopen_without_record_deadlocks_forever`). With the `begin()`
        // guard, `Drop` resolves the trial: the circuit re-opens with a fresh
        // cooldown and recovers on the next gate call.
        let b = UpstreamBreakers::new(1, 0); // zero cooldown: immediate half-open
        let ep = "http://cancelled-mid-flight:8090";
        b.record(ep, false); // opens
        assert_eq!(circuit_of(&b, ep), "open");

        {
            let trial = b
                .begin(ep)
                .expect("cooldown elapsed: half-open admits one trial");
            assert!(
                trial.consumed_trial(),
                "this guard holds the one half-open trial"
            );
            // No record() — the request future is dropped here.
        }

        assert_eq!(
            circuit_of(&b, ep),
            "open",
            "an abandoned trial re-opens with a fresh cooldown, never latches half-open"
        );
        // ...and unlike the latched case, the endpoint recovers.
        let trial = b.begin(ep).expect("cooldown elapsed: fresh trial admitted");
        trial.record(true);
        assert_eq!(circuit_of(&b, ep), "closed");
        assert!(!b.is_open_admitting_trial(ep));
    }

    #[test]
    fn abandoned_call_on_closed_circuit_is_not_penalized() {
        // A client disconnect is not upstream evidence: cancelling a request
        // against a healthy (closed) upstream must never trip its breaker.
        // Guards that did not consume a trial record nothing on drop.
        let b = UpstreamBreakers::new(3, 30);
        let ep = "http://healthy-but-clients-abort:8090";
        for _ in 0..10 {
            let trial = b.begin(ep).expect("closed circuit admits");
            assert!(!trial.consumed_trial());
            drop(trial); // cancelled, no outcome
        }
        assert_eq!(circuit_of(&b, ep), "closed");
        assert_eq!(
            b.snapshot()[0].error_streak,
            0,
            "cancellation on a closed circuit records nothing"
        );
        assert!(!b.is_open_admitting_trial(ep));
    }

    #[test]
    fn trial_records_exactly_once() {
        // A terminal path that records twice (or a record followed by drop)
        // must not double-count: first call wins.
        let b = UpstreamBreakers::new(1, 1_000_000);
        let ep = "http://records-twice:8090";
        {
            let trial = b.begin(ep).expect("fresh breaker is closed");
            trial.record(false); // opens (threshold 1)
            trial.record(true); // ignored
        }
        assert_eq!(circuit_of(&b, ep), "open", "second record is a no-op");
        assert_eq!(b.snapshot()[0].error_streak, 1);
    }

    #[test]
    fn stale_halfopen_readmits_a_trial() {
        // Belt-and-braces for a FUTURE caller that bypasses the guard and
        // forgets to record: after MULTIPLIER × cooldown with an outstanding
        // trial, the gate re-admits rather than shedding until restart.
        // Tick-injected (no wall clock) so it is deterministic.
        let b = UpstreamBreakers::new(1, 30);
        let ep = "http://stale-halfopen:8090";
        b.record_at(ep, false, 0); // opens at tick 0
        assert_eq!(b.admit(ep, 10), Admission::Shed, "inside cooldown");
        assert_eq!(b.admit(ep, 30), Admission::Trial, "cooldown elapsed");
        // The trial is consumed and never recorded.
        assert_eq!(b.admit(ep, 60), Admission::Shed);
        assert_eq!(
            b.admit(ep, 149),
            Admission::Shed,
            "still inside 4 x 30s of the trial admitted at tick 30"
        );
        assert_eq!(
            b.admit(ep, 150),
            Admission::Trial,
            "stale trial — re-admit rather than shed forever"
        );
        // Re-armed: the clock restarts from the re-admission.
        assert_eq!(b.admit(ep, 200), Admission::Shed);
    }

    // ---------------------------------------------------------------------
    // The open condition is a WINDOW, not a streak (2026-08-21 mesh evidence:
    // one ~60s stall on matthew fanned out into five failures inside a minute,
    // opened the circuit, and shed for minutes while storage answered the same
    // path in 1.6ms). Tick-injected via `record_at` so these are deterministic.
    // ---------------------------------------------------------------------

    #[test]
    fn two_failures_outside_the_window_do_not_open() {
        // Threshold 2 makes the WINDOW the only thing under test: two failures
        // is enough by count, and they still must not open, because 30s apart is
        // wider than the 20s window. A slow-and-recovering upstream is not a
        // down one.
        let b = UpstreamBreakers::with_window(2, 30, 20);
        let ep = "http://slow-but-alive:8090";
        b.record_at(ep, false, 0);
        b.record_at(ep, false, 30);
        assert_eq!(
            circuit_of(&b, ep),
            "closed",
            "the tick-0 failure aged out of the 20s window before tick 30"
        );
        assert_eq!(
            b.admit(ep, 30),
            Admission::Closed,
            "a closed circuit admits normally — no shed"
        );
        // The streak still counts them (that is the DEGRADING signal, a
        // separate honesty axis), but only the in-window tally opens.
        let snap = b.snapshot().into_iter().find(|s| s.endpoint == ep).unwrap();
        assert_eq!(
            snap.error_streak, 2,
            "both failures are still on the streak"
        );
    }

    #[test]
    fn three_failures_inside_the_window_open() {
        let b = UpstreamBreakers::with_window(3, 30, 20);
        let ep = "http://really-down:8090";
        b.record_at(ep, false, 0);
        b.record_at(ep, false, 10);
        assert_eq!(circuit_of(&b, ep), "closed", "2 of 3 inside the window");
        b.record_at(ep, false, 19);
        assert_eq!(
            circuit_of(&b, ep),
            "open",
            "3 failures within 20s is a burst — open"
        );
        assert_eq!(b.admit(ep, 20), Admission::Shed, "and it sheds");
    }

    #[test]
    fn failures_straddling_the_window_edge_never_accumulate() {
        // The regression shape itself: a steady trickle of failures, each one
        // further apart than the window, must NEVER open the circuit no matter
        // how many arrive. Under the old consecutive-streak rule this opened on
        // the third and shed for a cooldown, repeatedly.
        let b = UpstreamBreakers::with_window(3, 30, 20);
        let ep = "http://trickle:8090";
        for i in 0..20u64 {
            b.record_at(ep, false, i * 21);
            assert_eq!(
                circuit_of(&b, ep),
                "closed",
                "failure #{i} at tick {} is alone in its window",
                i * 21
            );
        }
    }

    #[test]
    fn a_success_empties_the_window() {
        // Recovery is not partial credit: two in-window failures plus a success
        // must not leave the circuit one failure from opening.
        let b = UpstreamBreakers::with_window(3, 30, 20);
        let ep = "http://recovers:8090";
        b.record_at(ep, false, 0);
        b.record_at(ep, false, 1);
        b.record_at(ep, true, 2);
        b.record_at(ep, false, 3);
        b.record_at(ep, false, 4);
        assert_eq!(
            circuit_of(&b, ep),
            "closed",
            "the success cleared the window; two fresh failures are not three"
        );
        b.record_at(ep, false, 5);
        assert_eq!(circuit_of(&b, ep), "open", "the third fresh failure opens");
    }

    #[test]
    fn window_open_then_half_open_trial_recovers() {
        // Full half-open recovery under the window rule: burst opens, cooldown
        // admits exactly one trial, a successful trial closes and re-arms the
        // window from empty.
        let b = UpstreamBreakers::with_window(3, 30, 20);
        let ep = "http://recovers-via-trial:8090";
        b.record_at(ep, false, 0);
        b.record_at(ep, false, 5);
        b.record_at(ep, false, 10);
        assert_eq!(circuit_of(&b, ep), "open");

        assert_eq!(b.admit(ep, 20), Admission::Shed, "inside the 30s cooldown");
        assert_eq!(
            b.admit(ep, 40),
            Admission::Trial,
            "cooldown elapsed: exactly one trial"
        );
        assert_eq!(
            b.admit(ep, 41),
            Admission::Shed,
            "everyone else still sheds"
        );

        b.record_at(ep, true, 42); // the trial succeeded
        assert_eq!(circuit_of(&b, ep), "closed");
        // ...and the window is empty, so a single failure does not re-open.
        b.record_at(ep, false, 43);
        assert_eq!(
            circuit_of(&b, ep),
            "closed",
            "one failure after recovery is not a burst"
        );
    }

    #[test]
    fn a_failed_half_open_trial_reopens_without_waiting_for_a_window() {
        // The trial IS the probe: one failed trial is complete evidence, so the
        // window must not require THRESHOLD more failures before re-opening.
        let b = UpstreamBreakers::with_window(3, 30, 20);
        let ep = "http://trial-fails:8090";
        b.record_at(ep, false, 0);
        b.record_at(ep, false, 1);
        b.record_at(ep, false, 2);
        assert_eq!(b.admit(ep, 40), Admission::Trial);
        b.record_at(ep, false, 41);
        assert_eq!(
            circuit_of(&b, ep),
            "open",
            "a single failed trial re-opens immediately"
        );
        assert_eq!(b.admit(ep, 50), Admission::Shed, "with a fresh cooldown");
    }

    #[test]
    fn policy_reports_the_numbers_the_gate_actually_used() {
        // C7: what /admin/self-healing and /status.json render must be read off
        // the same fields record_at decides with — no restated constants.
        let b = UpstreamBreakers::with_window(4, 45, 15);
        let p = b.policy();
        assert_eq!(p.fail_threshold, 4);
        assert_eq!(p.fail_window_secs, 15);
        assert_eq!(p.cooldown_secs, 45);

        let ep = "http://policy:8090";
        // Exactly `fail_threshold` failures inside exactly `fail_window_secs`.
        for i in 0..p.fail_threshold as u64 {
            b.record_at(ep, false, i * 5);
        }
        assert_eq!(
            circuit_of(&b, ep),
            "open",
            "threshold failures spanning {}s (window {}s) opened it",
            (p.fail_threshold as u64 - 1) * 5,
            p.fail_window_secs
        );

        let d = UpstreamBreakers::default();
        assert_eq!(d.policy().fail_threshold, UPSTREAM_CIRCUIT_FAIL_THRESHOLD);
        assert_eq!(
            d.policy().fail_window_secs,
            UPSTREAM_CIRCUIT_FAIL_WINDOW_SECS
        );
        assert_eq!(d.policy().cooldown_secs, UPSTREAM_CIRCUIT_COOLDOWN_SECS);
    }

    #[test]
    fn snapshot_recent_failures_counts_only_the_window() {
        let b = UpstreamBreakers::with_window(5, 30, 20);
        let ep = "http://window-count:8090";
        b.record(ep, false);
        b.record(ep, false);
        let snap = b.snapshot().into_iter().find(|s| s.endpoint == ep).unwrap();
        assert_eq!(snap.error_streak, 2);
        assert_eq!(
            snap.recent_failures, 2,
            "both failures landed in the same wall-clock second"
        );
        b.record(ep, true);
        let snap = b.snapshot().into_iter().find(|s| s.endpoint == ep).unwrap();
        assert_eq!(snap.recent_failures, 0, "a success empties the window");
        assert_eq!(snap.error_streak, 0);
    }

    #[test]
    fn snapshot_reports_open_without_admitting_trial() {
        let b = UpstreamBreakers::new(1, 1_000_000); // huge cooldown — stays open
        b.record("http://x", false); // opens (threshold 1)
        let snap = b.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].endpoint, "http://x");
        assert_eq!(snap[0].circuit, "open");
        assert!(snap[0].skipped, "open circuit is shedding");
        assert_eq!(snap[0].error_streak, 1);
        // snapshot() must NOT have advanced Open→HalfOpen (it used state(), not
        // should_skip): the breaker is still open on the next read.
        assert!(b.is_open_admitting_trial("http://x"));
    }
}
