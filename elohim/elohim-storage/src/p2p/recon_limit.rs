//! Per-peer inbound limit for `/elohim/inventory-recon/1.0.0`.
//!
//! A recon session is a few messages, so a healthy initiator sends a handful per
//! reconcile cadence. A peer past [`RECON_REQUESTS_PER_MINUTE`] in a sliding minute is
//! answered with [`super::recon_wire::RECON_BUSY_REPLY`] before any work is done for it:
//! the limit is checked ahead of admission and the servable index.

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Requests one peer may send in [`RECON_WINDOW`]. A full session over a large set is
/// bounded by `RECON_PAGE_LIMIT` pages; 60/min leaves room for several concurrent
/// sessions from one honest peer.
pub const RECON_REQUESTS_PER_MINUTE: usize = 60;

/// The sliding window the limit counts over.
pub const RECON_WINDOW: Duration = Duration::from_secs(60);

/// Peers tracked at once; past it, peers with no request in the window are dropped, and
/// if all are live the oldest-seen is dropped (the map never grows unbounded).
const MAX_TRACKED_PEERS: usize = 4096;

/// Sliding-window counter keyed by peer.
pub struct ReconLimiter<K> {
    max: usize,
    window: Duration,
    seen: Mutex<HashMap<K, VecDeque<Instant>>>,
}

impl<K: Hash + Eq + Clone> ReconLimiter<K> {
    pub fn new(max: usize, window: Duration) -> Self {
        Self {
            max,
            window,
            seen: Mutex::new(HashMap::new()),
        }
    }

    /// The production limit.
    pub fn standard() -> Self {
        Self::new(RECON_REQUESTS_PER_MINUTE, RECON_WINDOW)
    }

    /// Record a request from `peer` at `now`; `false` when it is over the limit (a
    /// refused request is not recorded, so a flood does not extend its own penalty).
    pub fn admit(&self, peer: &K, now: Instant) -> bool {
        let mut seen = self.seen.lock().unwrap_or_else(|p| p.into_inner());
        if seen.len() >= MAX_TRACKED_PEERS && !seen.contains_key(peer) {
            let window = self.window;
            seen.retain(|_, hits| {
                hits.back()
                    .is_some_and(|last| now.duration_since(*last) < window)
            });
            if seen.len() >= MAX_TRACKED_PEERS {
                let oldest = seen
                    .iter()
                    .filter_map(|(k, hits)| hits.back().map(|t| (k.clone(), *t)))
                    .min_by_key(|(_, t)| *t)
                    .map(|(k, _)| k);
                if let Some(oldest) = oldest {
                    seen.remove(&oldest);
                }
            }
        }
        let hits = seen.entry(peer.clone()).or_default();
        while hits
            .front()
            .is_some_and(|t| now.duration_since(*t) >= self.window)
        {
            hits.pop_front();
        }
        if hits.len() >= self.max {
            return false;
        }
        hits.push_back(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::PeerId;

    #[test]
    fn inbound_recon_is_rate_limited_per_peer() {
        let limiter = ReconLimiter::new(3, Duration::from_secs(60));
        let (a, b) = (PeerId::random(), PeerId::random());
        let t0 = Instant::now();
        for _ in 0..3 {
            assert!(limiter.admit(&a, t0));
        }
        assert!(
            !limiter.admit(&a, t0),
            "the fourth in the window is refused"
        );
        assert!(limiter.admit(&b, t0), "another peer is unaffected");
        assert!(
            !limiter.admit(&a, t0 + Duration::from_secs(59)),
            "still inside the window"
        );
        assert!(
            limiter.admit(&a, t0 + Duration::from_secs(61)),
            "the window slid past the first requests"
        );
    }

    #[test]
    fn the_peer_map_stays_bounded() {
        let limiter: ReconLimiter<u32> = ReconLimiter::new(1, Duration::from_secs(60));
        let now = Instant::now();
        for i in 0..(MAX_TRACKED_PEERS as u32 + 50) {
            limiter.admit(&i, now);
        }
        let len = limiter.seen.lock().unwrap().len();
        assert!(len <= MAX_TRACKED_PEERS);
    }
}
