//! Defense-in-depth: per-source admission verdict over a pluggable store + clock.

/// The membrane policy verdict for one inbound request from `source`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Allow,
    Shape { delay_ms: u64 },
    Challenge,
    Deny,
}

/// Monotonic-ish wall clock in whole seconds (the runtime supplies it; tests inject a fixed value).
pub trait Clock {
    fn now_secs(&self) -> u64;
}

/// Pluggable defense state. Storage backs this with SQLite; doorway with an in-memory/edge store.
///
/// **Windowing is the implementer's responsibility.** `assess` only ever asks for the in-window
/// count via `count_since(source, since)`, so an implementer should evict/ignore hits older than the
/// window (SQLite `DELETE WHERE ts < since`; in-memory sweep) to bound state for a high-rate but
/// not-yet-banned source. The crate guarantees only that a *banned* source is never recorded.
pub trait GuardStore {
    fn record(&mut self, source: &str, ts_secs: u64);
    fn count_since(&self, source: &str, since_secs: u64) -> u32;
    fn is_banned(&self, source: &str, now_secs: u64) -> bool;
    fn ban_until(&mut self, source: &str, until_secs: u64);
}

/// Thresholds for the sliding-window rate response. `shape <= challenge <= ban`.
#[derive(Debug, Clone)]
pub struct GuardConfig {
    pub window_secs: u64,
    pub shape_threshold: u32,
    pub challenge_threshold: u32,
    pub ban_threshold: u32,
    pub ban_secs: u64,
    pub shape_delay_ms: u64,
}

impl Default for GuardConfig {
    fn default() -> Self {
        Self {
            window_secs: 60,
            shape_threshold: 20,
            challenge_threshold: 60,
            ban_threshold: 200,
            ban_secs: 900,
            shape_delay_ms: 250,
        }
    }
}

/// Assess one request from `source`. Banned sources are denied WITHOUT recording (no unbounded growth).
/// Otherwise record the hit, count the in-window rate, and escalate: Allow → Shape → Challenge → Deny(+ban).
pub fn assess<S: GuardStore, C: Clock>(
    store: &mut S,
    clock: &C,
    cfg: &GuardConfig,
    source: &str,
) -> Verdict {
    let now = clock.now_secs();
    if store.is_banned(source, now) {
        return Verdict::Deny;
    }
    store.record(source, now);
    let since = now.saturating_sub(cfg.window_secs);
    let count = store.count_since(source, since);
    if count > cfg.ban_threshold {
        store.ban_until(source, now.saturating_add(cfg.ban_secs));
        Verdict::Deny
    } else if count > cfg.challenge_threshold {
        Verdict::Challenge
    } else if count > cfg.shape_threshold {
        Verdict::Shape {
            delay_ms: cfg.shape_delay_ms,
        }
    } else {
        Verdict::Allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct FixedClock(u64);
    impl Clock for FixedClock {
        fn now_secs(&self) -> u64 {
            self.0
        }
    }

    #[derive(Default)]
    struct MemStore {
        hits: HashMap<String, Vec<u64>>,
        bans: HashMap<String, u64>,
    }
    impl GuardStore for MemStore {
        fn record(&mut self, source: &str, ts: u64) {
            self.hits.entry(source.into()).or_default().push(ts);
        }
        fn count_since(&self, source: &str, since: u64) -> u32 {
            self.hits
                .get(source)
                .map_or(0, |v| v.iter().filter(|&&t| t >= since).count() as u32)
        }
        fn is_banned(&self, source: &str, now: u64) -> bool {
            self.bans.get(source).is_some_and(|&until| until > now)
        }
        fn ban_until(&mut self, source: &str, until: u64) {
            self.bans.insert(source.into(), until);
        }
    }

    fn cfg() -> GuardConfig {
        GuardConfig {
            window_secs: 60,
            shape_threshold: 3,
            challenge_threshold: 6,
            ban_threshold: 10,
            ban_secs: 300,
            shape_delay_ms: 250,
        }
    }

    #[test]
    fn first_request_is_allowed() {
        let mut s = MemStore::default();
        assert_eq!(
            assess(&mut s, &FixedClock(1000), &cfg(), "ip:1.2.3.4"),
            Verdict::Allow
        );
    }

    #[test]
    fn crossing_shape_threshold_shapes_then_challenges_then_bans() {
        let mut s = MemStore::default();
        let clk = FixedClock(1000);
        let c = cfg();
        // Pre-load 4 hits in-window → next assess sees 5 → Shape (>=shape, <challenge).
        for _ in 0..4 {
            s.record("src", 1000);
        }
        assert!(matches!(
            assess(&mut s, &clk, &c, "src"),
            Verdict::Shape { .. }
        ));
        // Push to challenge band.
        for _ in 0..3 {
            s.record("src", 1000);
        }
        assert_eq!(assess(&mut s, &clk, &c, "src"), Verdict::Challenge);
        // Push past ban threshold → Deny + future ban set.
        for _ in 0..5 {
            s.record("src", 1000);
        }
        assert_eq!(assess(&mut s, &clk, &c, "src"), Verdict::Deny);
        assert!(s.is_banned("src", 1000));
    }

    #[test]
    fn banned_source_is_denied_without_recording() {
        let mut s = MemStore::default();
        s.ban_until("bad", 2000);
        assert_eq!(
            assess(&mut s, &FixedClock(1500), &cfg(), "bad"),
            Verdict::Deny
        );
        assert_eq!(
            s.count_since("bad", 0),
            0,
            "a banned source must not be recorded (no unbounded growth)"
        );
    }

    #[test]
    fn ban_expires_and_traffic_resumes() {
        let mut s = MemStore::default();
        s.ban_until("x", 2000);
        assert_eq!(
            assess(&mut s, &FixedClock(2001), &cfg(), "x"),
            Verdict::Allow
        );
    }
}
