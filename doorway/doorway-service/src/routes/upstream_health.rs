//! Per-upstream circuit breakers for the storage-proxy path.
//!
//! Cat C node-local OPERATIONAL state (no DHT entry, no table). Wraps the
//! shared `elohim_compute::CircuitBreaker` keyed by storage endpoint URL.
//! The breaker is tick-injected; this map feeds it a wall-clock tick
//! (`started.elapsed().as_secs()`) so `cooldown_ticks` == cooldown seconds.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use elohim_compute::{CircuitBreaker, CircuitState};

/// Consecutive failed upstream outcomes before a circuit opens.
pub const UPSTREAM_CIRCUIT_FAIL_THRESHOLD: u32 = 3;
/// Seconds a circuit stays open before a half-open trial.
pub const UPSTREAM_CIRCUIT_COOLDOWN_SECS: u64 = 30;

/// Per-endpoint breaker map for the storage proxy.
pub struct UpstreamBreakers {
    breakers: Mutex<HashMap<String, CircuitBreaker>>,
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

    /// True if a call to `endpoint` should be SHED (circuit open and not yet
    /// admitting a half-open trial). Side effect: advances Open→HalfOpen when
    /// the cooldown has elapsed (admits exactly one trial).
    pub fn is_open(&self, endpoint: &str) -> bool {
        let tick = self.tick();
        let mut map = self.breakers.lock().unwrap();
        let cb = map
            .entry(endpoint.to_string())
            .or_insert_with(|| CircuitBreaker::new(self.fail_threshold, self.cooldown_ticks));
        cb.should_skip(tick)
    }

    /// Record an outcome for `endpoint` (ok=false counts toward opening).
    pub fn record(&self, endpoint: &str, ok: bool) {
        let tick = self.tick();
        let mut map = self.breakers.lock().unwrap();
        let cb = map
            .entry(endpoint.to_string())
            .or_insert_with(|| CircuitBreaker::new(self.fail_threshold, self.cooldown_ticks));
        cb.record_outcome(ok, tick);
    }

    /// Read-only snapshot of every known endpoint's breaker — the accessor the
    /// self_healing UpstreamView needs (handoff §6 co-delivery). Uses `state()`,
    /// NOT `should_skip()`, so it never admits a half-open trial as a side
    /// effect of being observed.
    pub fn snapshot(&self) -> Vec<BreakerSnapshot> {
        let map = self.breakers.lock().unwrap();
        map.iter()
            .map(|(endpoint, cb)| {
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
