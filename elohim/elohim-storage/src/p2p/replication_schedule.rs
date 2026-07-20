//! Per-peer replication scheduling — in-flight guard + exponential backoff.
//!
//! ## Why this exists
//!
//! A slow network link must cost convergence *time* only, never local *health*:
//! sync must back off, never thrash. Before this module, `run_replication_cycle`
//! fired a `ShardRequest::ListContent` to EVERY connected peer every 60s with no
//! per-peer in-flight guard and no backoff. libp2p's request timeout is a fixed
//! 30s, so a peer whose response could not fit the link inside 30s produced an
//! `OutboundFailure::Timeout` — and the next 60s tick re-fired the exact same
//! request from scratch. That is a self-sustaining timeout storm (observed in
//! prod: an `Outbound shard request failed … Timeout` flood with the node pinned
//! at multi-core CPU).
//!
//! ## What it does
//!
//! Holds the scheduling state that makes the cycle back off instead of thrash:
//!
//! - **In-flight guard.** A peer with a `ListContent` discovery chain still
//!   outstanding is skipped — never a second concurrent chain to the same peer.
//! - **Exponential backoff.** On an outbound failure (especially `Timeout`) the
//!   peer's next eligibility doubles from the 60s base up to a 15min cap; any
//!   successful response resets it to the base.
//!
//! The state is in-memory: a fresh process has no reason to distrust a peer, so
//! restart correctly resets every peer to the base interval.

use libp2p::PeerId;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Base inter-cycle interval — matches the 60s replication tick. A peer at base
/// is eligible for a new `ListContent` cycle once per tick.
pub const REPL_BASE_INTERVAL: Duration = Duration::from_secs(60);

/// Ceiling on the per-peer backoff interval (15 minutes). A peer that keeps
/// timing out is asked at most this rarely — enough to notice recovery without
/// re-arming the storm.
pub const REPL_MAX_BACKOFF: Duration = Duration::from_secs(15 * 60);

/// Per-peer replication scheduling state: an in-flight guard plus an
/// exponential-backoff eligibility gate. Pure and clock-injected (`now` is a
/// parameter) so the backoff state machine is unit-testable without a wall clock.
#[derive(Debug, Clone)]
struct PeerReplState {
    /// True while a `ListContent` discovery chain is outstanding to this peer.
    /// Blocks a second concurrent chain (the thrash guard). A chain may span
    /// several paginated request-ids; this stays true across the whole chain and
    /// clears only on the final page's success or on any page's failure.
    in_flight: bool,
    /// Current inter-cycle interval. Starts at [`REPL_BASE_INTERVAL`]; doubles on
    /// each outbound failure up to [`REPL_MAX_BACKOFF`]; resets to base on success.
    interval: Duration,
    /// Earliest instant this peer may begin a NEW `ListContent` cycle.
    next_eligible_at: Instant,
}

impl PeerReplState {
    fn new(now: Instant) -> Self {
        Self {
            in_flight: false,
            interval: REPL_BASE_INTERVAL,
            next_eligible_at: now,
        }
    }

    /// May a new `ListContent` cycle start for this peer right now? A chain
    /// already in flight, or a backoff gate not yet elapsed, blocks it.
    fn is_eligible(&self, now: Instant) -> bool {
        !self.in_flight && now >= self.next_eligible_at
    }

    /// A `ListContent` chain has started — set the in-flight guard.
    fn mark_started(&mut self) {
        self.in_flight = true;
    }

    /// The chain completed successfully: clear in-flight, reset the backoff to
    /// base, and schedule the next cycle one base interval out.
    ///
    /// Reset-on-success tradeoff (intentional, standard backoff): a peer that
    /// flaps success/failure never climbs past a single doubling — each success
    /// snaps the interval back to base, so it oscillates around base rather than
    /// escalating to the cap. That is the desired behavior: a peer that
    /// intermittently answers is treated as reachable-but-slow, not as failed.
    /// Only a peer that fails REPEATEDLY WITHOUT an intervening success walks the
    /// interval up to [`REPL_MAX_BACKOFF`] — which is exactly the storm case this
    /// guards against.
    fn mark_success(&mut self, now: Instant) {
        self.in_flight = false;
        self.interval = REPL_BASE_INTERVAL;
        self.next_eligible_at = now + REPL_BASE_INTERVAL;
    }

    /// The chain failed (timeout / transport error): clear in-flight, double the
    /// interval up to the cap, and push eligibility out by the new interval.
    fn mark_failure(&mut self, now: Instant) {
        self.in_flight = false;
        let doubled = self.interval.saturating_mul(2);
        self.interval = doubled.min(REPL_MAX_BACKOFF);
        self.next_eligible_at = now + self.interval;
    }
}

/// Thread-safe per-peer replication scheduler. Owns the `PeerId → PeerReplState`
/// map behind a mutex; the run loop calls it from a single task, so contention is
/// nil. Unknown peers are eligible by default (first cycle asks everyone).
#[derive(Debug, Default)]
pub struct ReplicationScheduler {
    peers: Mutex<HashMap<PeerId, PeerReplState>>,
}

impl ReplicationScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter `candidates` to the peers eligible to START a new `ListContent`
    /// cycle now: not in-flight and past their backoff gate. A never-seen peer is
    /// eligible (returns it) — the entry is created lazily by [`Self::mark_started`].
    pub async fn eligible_peers(&self, candidates: &[PeerId]) -> Vec<PeerId> {
        let now = Instant::now();
        let map = self.peers.lock().await;
        candidates
            .iter()
            .copied()
            .filter(|p| map.get(p).is_none_or(|s| s.is_eligible(now)))
            .collect()
    }

    /// Record that a `ListContent` chain has started for `peer` (sets the
    /// in-flight guard, creating the entry if absent).
    pub async fn mark_started(&self, peer: PeerId) {
        let now = Instant::now();
        let mut map = self.peers.lock().await;
        map.entry(peer)
            .or_insert_with(|| PeerReplState::new(now))
            .mark_started();
    }

    /// Record a successful chain completion for `peer` — reset to base cadence.
    pub async fn mark_success(&self, peer: PeerId) {
        let now = Instant::now();
        let mut map = self.peers.lock().await;
        map.entry(peer)
            .or_insert_with(|| PeerReplState::new(now))
            .mark_success(now);
    }

    /// Record a chain failure for `peer` — double the backoff up to the cap.
    pub async fn mark_failure(&self, peer: PeerId) {
        let now = Instant::now();
        let mut map = self.peers.lock().await;
        map.entry(peer)
            .or_insert_with(|| PeerReplState::new(now))
            .mark_failure(now);
    }

    /// Prune scheduling state for peers not in `keep`, bounding the map to the
    /// currently-connected set. Called once per cycle with the connected peers,
    /// so churned peers do not leak entries (and a still-connected peer keeps its
    /// backoff — no premature reset from a single connection closing).
    pub async fn retain(&self, keep: &[PeerId]) {
        let mut map = self.peers.lock().await;
        map.retain(|p, _| keep.contains(p));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_peer_is_eligible_immediately() {
        let t0 = Instant::now();
        let s = PeerReplState::new(t0);
        assert!(s.is_eligible(t0), "a fresh peer must be askable right away");
    }

    #[test]
    fn in_flight_blocks_eligibility_regardless_of_clock() {
        let t0 = Instant::now();
        let mut s = PeerReplState::new(t0);
        s.mark_started();
        assert!(
            !s.is_eligible(t0 + Duration::from_secs(3600)),
            "an outstanding chain must block a second concurrent chain"
        );
    }

    #[test]
    fn success_resets_to_base_interval() {
        let t0 = Instant::now();
        let mut s = PeerReplState::new(t0);
        // Push the interval up via a failure, then a success must reset it.
        s.mark_started();
        s.mark_failure(t0); // interval now 120s
        s.mark_started();
        s.mark_success(t0); // reset to base 60s
        assert_eq!(s.interval, REPL_BASE_INTERVAL);
        assert!(!s.is_eligible(t0 + Duration::from_secs(59)));
        assert!(s.is_eligible(t0 + Duration::from_secs(60)));
    }

    #[test]
    fn failure_doubles_interval_up_to_cap() {
        let t0 = Instant::now();
        let mut s = PeerReplState::new(t0);
        s.mark_failure(t0);
        assert_eq!(s.interval, Duration::from_secs(120), "60 -> 120");
        s.mark_failure(t0);
        assert_eq!(s.interval, Duration::from_secs(240), "120 -> 240");
        s.mark_failure(t0);
        assert_eq!(s.interval, Duration::from_secs(480), "240 -> 480");
        s.mark_failure(t0);
        assert_eq!(s.interval, REPL_MAX_BACKOFF, "480 -> 960 capped to 900");
        s.mark_failure(t0);
        assert_eq!(s.interval, REPL_MAX_BACKOFF, "stays pinned at the cap");
    }

    #[test]
    fn failure_pushes_eligibility_out_by_new_interval() {
        let t0 = Instant::now();
        let mut s = PeerReplState::new(t0);
        s.mark_failure(t0); // interval 120, eligible at t0 + 120
        assert!(!s.is_eligible(t0 + Duration::from_secs(119)));
        assert!(s.is_eligible(t0 + Duration::from_secs(120)));
    }

    #[test]
    fn failure_clears_in_flight_so_backoff_can_gate() {
        let t0 = Instant::now();
        let mut s = PeerReplState::new(t0);
        s.mark_started();
        s.mark_failure(t0);
        assert!(!s.in_flight, "a failed chain is no longer in flight");
        // Blocked by backoff, not by the in-flight guard.
        assert!(!s.is_eligible(t0 + Duration::from_secs(119)));
        assert!(s.is_eligible(t0 + Duration::from_secs(120)));
    }

    #[tokio::test]
    async fn scheduler_skips_in_flight_and_prunes_on_retain() {
        let sched = ReplicationScheduler::new();
        let a = PeerId::random();
        let b = PeerId::random();

        // Both eligible initially (never seen).
        let elig = sched.eligible_peers(&[a, b]).await;
        assert_eq!(elig.len(), 2);

        // Start a chain to `a`: now only `b` is eligible.
        sched.mark_started(a).await;
        let elig = sched.eligible_peers(&[a, b]).await;
        assert_eq!(elig, vec![b]);

        // Retain only `a`: `b`'s (implicit) state is irrelevant, but a stale
        // entry for a departed peer would be pruned. Prove `a` survives retain.
        sched.retain(&[a]).await;
        // `a` still in flight → not eligible; `b` unknown again → eligible.
        let elig = sched.eligible_peers(&[a, b]).await;
        assert_eq!(elig, vec![b]);
    }
}
