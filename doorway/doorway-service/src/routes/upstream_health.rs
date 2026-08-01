//! Per-upstream circuit breakers for the storage-proxy path.
//!
//! Cat C node-local OPERATIONAL state (no DHT entry, no table). Wraps the
//! shared `elohim_compute::CircuitBreaker` keyed by storage endpoint URL.
//! The breaker is tick-injected; this map feeds it a wall-clock tick
//! (`started.elapsed().as_secs()`) so `cooldown_ticks` == cooldown seconds.

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use elohim_compute::{CircuitBreaker, CircuitState};
use tracing::warn;

/// Consecutive failed upstream outcomes before a circuit opens.
pub const UPSTREAM_CIRCUIT_FAIL_THRESHOLD: u32 = 3;
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
    cb: CircuitBreaker,
    /// Tick at which the currently-outstanding half-open trial was admitted.
    /// `Some` only while state is `HalfOpen`; cleared whenever an outcome lands.
    halfopen_since: Option<u64>,
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
}

impl UpstreamBreakers {
    pub fn new(fail_threshold: u32, cooldown_secs: u64) -> Self {
        Self {
            breakers: Mutex::new(HashMap::new()),
            started: Instant::now(),
            fail_threshold,
            cooldown_ticks: cooldown_secs,
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
        let fail_threshold = self.fail_threshold;
        let cooldown_ticks = self.cooldown_ticks;
        let stale_ticks = self.stale_halfopen_ticks();
        let mut map = self.breakers.lock().unwrap();
        let entry = map.entry(endpoint.to_string()).or_insert_with(|| Entry {
            cb: CircuitBreaker::new(fail_threshold, cooldown_ticks),
            halfopen_since: None,
        });

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

    /// True if a call to `endpoint` should be SHED (circuit open and not yet
    /// admitting a half-open trial). Side effect: advances Open→HalfOpen when
    /// the cooldown has elapsed (admits exactly one trial).
    ///
    /// PREFER [`UpstreamBreakers::begin`]: a caller that gets `false` here has
    /// possibly consumed the one half-open trial and MUST record an outcome on
    /// every terminal path — *including paths that never run* because the
    /// request future was dropped (client disconnect, task cancellation). Only
    /// the guard returned by `begin` can honour that on a dropped future.
    pub fn is_open(&self, endpoint: &str) -> bool {
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
        let mut map = self.breakers.lock().unwrap();
        let entry = map.entry(endpoint.to_string()).or_insert_with(|| Entry {
            cb: CircuitBreaker::new(fail_threshold, cooldown_ticks),
            halfopen_since: None,
        });
        entry.cb.record_outcome(ok, tick);
        entry.halfopen_since = None;
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
                // HalfOpen + failure → Open with opened_at = now.
                entry.cb.record_outcome(false, tick);
                entry.halfopen_since = None;
            }
        }
    }

    /// Read-only snapshot of every known endpoint's breaker — the accessor the
    /// self_healing UpstreamView needs (handoff §6 co-delivery). Uses `state()`,
    /// NOT `should_skip()`, so it never admits a half-open trial as a side
    /// effect of being observed.
    pub fn snapshot(&self) -> Vec<BreakerSnapshot> {
        let map = self.breakers.lock().unwrap();
        map.iter()
            .map(|(endpoint, entry)| {
                let cb = &entry.cb;
                let (circuit, skipped) = match cb.state() {
                    CircuitState::Closed => ("closed", false),
                    CircuitState::HalfOpen => ("half-open", false),
                    CircuitState::Open => ("open", true),
                };
                BreakerSnapshot {
                    endpoint: endpoint.clone(),
                    circuit,
                    error_streak: cb.error_streak(),
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
    pub error_streak: u32,
    /// True when the circuit is OPEN (currently shedding, no trial admitted).
    pub skipped: bool,
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
        assert!(!b.is_open(ep), "closed on first sight");
        b.record(ep, false);
        b.record(ep, false);
        assert!(!b.is_open(ep), "2 < 3: still closed");
        b.record(ep, false);
        assert!(b.is_open(ep), "3rd failure opens -> shed");
    }

    #[test]
    fn success_keeps_closed() {
        let b = UpstreamBreakers::new(3, 30);
        let ep = "http://healthy:8090";
        for _ in 0..10 {
            b.record(ep, true);
        }
        assert!(!b.is_open(ep));
    }

    #[test]
    fn distinct_endpoints_isolated() {
        let b = UpstreamBreakers::new(1, 1_000_000);
        b.record("http://a", false); // a opens
        assert!(b.is_open("http://a"));
        assert!(!b.is_open("http://b"), "b unaffected by a");
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
            !b.is_open(ep),
            "cooldown elapsed: half-open admits one trial"
        );
        assert_eq!(circuit_of(&b, ep), "half-open");
        // Every subsequent is_open() call — with NO record() in between —
        // reports open (shed) forever. No amount of further calling or time
        // passing recovers it; only a recorded outcome can.
        for _ in 0..50 {
            assert!(
                b.is_open(ep),
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
        assert!(!b.is_open(ep), "cooldown elapsed: admits first trial");
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
            !b.is_open(ep),
            "cooldown elapsed again: admits a fresh trial"
        );
        b.record(ep, true); // this time the trial succeeds
        assert_eq!(circuit_of(&b, ep), "closed");
        assert!(!b.is_open(ep), "recorded success closes the circuit");
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
        assert!(!b.is_open(ep));
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
        assert!(!b.is_open(ep));
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
        assert!(b.is_open("http://x"));
    }
}
