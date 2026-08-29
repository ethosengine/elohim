//! Transport self-awareness — `PathObservation` + `select_path`
//! (spec `genesis/docs/superpowers/specs/2026-08-24-transport-self-awareness-diversity-harness-design.md` §3.1).
//!
//! **Race the cheap operations, select the expensive ones — and the races ARE
//! the probe.** Small operations (a blob read-miss heal, a head-record fetch)
//! already fire on both live planes and take the first verified answer; every
//! such race yields a fresh RTT sample per plane at ~zero marginal cost. Bulk
//! operations (the acquisition pull of a whole record + blob, a shard push)
//! SELECT one plane from those samples, because duplicating a large transfer
//! is genuinely expensive. An exploration floor keeps the non-preferred live
//! plane sampled so its recovery is detectable (C3) and a dynamic preference
//! can never harden into transport monoculture — canon's anti-capture clause.
//!
//! Entity class: **Ephemeral (C)**. Local, asymmetric measurement (my RTT to
//! you ≠ yours to me), fully reconstructable by re-observing. In-memory ring,
//! no table, no migration, no `dht_anchor_hash`; keys are bounded by the peer
//! book (`MAX_KEYS`). Cold start = `Unknown` everywhere.
//!
//! Keyed by the peer's cross-plane LABEL — the libp2p PeerId when the peer is
//! dual (the same string `pull_core`, `acquisition_dispatch` and the reconcile
//! arms use), else its agent CID, else the iroh node id — never by raw-comparing
//! a libp2p PeerId against an iroh NodeId.
//!
//! Flag: `ELOHIM_TRANSPORT_SELECTION=off|0|false|no` disables selection (every
//! Bulk decision falls back to the static prior — today's behaviour); sampling
//! and `/p2p/status.transportPaths` stay on so the before/after is observable.

use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    Libp2p,
    Iroh,
}

impl Transport {
    pub fn as_str(self) -> &'static str {
        match self {
            Transport::Libp2p => "libp2p",
            Transport::Iroh => "iroh",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OpClass {
    /// Latency-critical and cheap to duplicate: raced on every live plane.
    Small,
    /// Expensive to duplicate: one plane is selected from the samples.
    Bulk,
}

impl OpClass {
    pub fn as_str(self) -> &'static str {
        match self {
            OpClass::Small => "small",
            OpClass::Bulk => "bulk",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PathState {
    /// No sample yet. Its own state — never coerced to "slow" (C4).
    Unknown,
    Sampled,
    /// Success rate below [`SUCCESS_FLOOR`] over the ring, or the remote
    /// signalled backpressure on the last sample (C11: lowers weight, never
    /// triggers retries).
    Degraded,
}

/// Samples kept per key — the fixed-size ring (C6a).
pub const RING: usize = 16;
/// EWMA weight of the newest verified-success RTT.
pub const EWMA_ALPHA: f64 = 0.3;
/// Ring success rate below which a plane reads `Degraded`.
pub const SUCCESS_FLOOR: f64 = 0.5;
/// Samples the ring must hold before the success rate can demote a plane —
/// one blip (a peer mid-restart, measured 2026-08-29) is not evidence of a
/// degraded path; a saturation hint still demotes immediately (C11).
pub const MIN_SAMPLES_FOR_DEGRADED: usize = 3;
/// Exploration floor for `Bulk`: one pick in this many goes to a non-best live
/// plane (the spec's `explore = 0.10`, made deterministic on a per-key pick
/// counter so the C4 contract "explores within N picks" is provable).
pub const EXPLORE_EVERY: u32 = 10;
/// Upper bound on observation keys — the peer book is the real bound; this is
/// the backstop against a hostile announcer minting labels.
pub const MAX_KEYS: usize = 4096;

#[derive(Debug, Clone, Default)]
pub struct PathObservation {
    pub rtt_ewma_ms: Option<f64>,
    ring: VecDeque<bool>,
    pub attempts: u64,
    pub successes: u64,
    pub last_sample: Option<Instant>,
    pub saturation_hint: bool,
}

impl PathObservation {
    pub fn record(&mut self, rtt: Option<Duration>, ok: bool, saturation_hint: bool) {
        self.attempts += 1;
        if ok {
            self.successes += 1;
            if let Some(rtt) = rtt {
                let ms = rtt.as_secs_f64() * 1000.0;
                self.rtt_ewma_ms = Some(match self.rtt_ewma_ms {
                    Some(prev) => prev * (1.0 - EWMA_ALPHA) + ms * EWMA_ALPHA,
                    None => ms,
                });
            }
        }
        if self.ring.len() == RING {
            self.ring.pop_front();
        }
        self.ring.push_back(ok);
        // A hint is a statement about the LAST exchange: a clean success clears it.
        self.saturation_hint = saturation_hint;
        self.last_sample = Some(Instant::now());
    }

    pub fn success_rate(&self) -> Option<f64> {
        if self.ring.is_empty() {
            return None;
        }
        let ok = self.ring.iter().filter(|b| **b).count();
        Some(ok as f64 / self.ring.len() as f64)
    }

    pub fn state(&self) -> PathState {
        if self.attempts == 0 {
            return PathState::Unknown;
        }
        if self.saturation_hint {
            return PathState::Degraded;
        }
        if self.ring.len() >= MIN_SAMPLES_FOR_DEGRADED
            && self.success_rate().unwrap_or(0.0) < SUCCESS_FLOOR
        {
            return PathState::Degraded;
        }
        PathState::Sampled
    }

    pub fn last_sample_age(&self) -> Option<Duration> {
        self.last_sample.map(|t| t.elapsed())
    }
}

/// What `select_path` sees per eligible plane — a pure projection of a
/// [`PathObservation`] so the predicate stays testable without the store.
#[derive(Debug, Clone, PartialEq)]
pub struct PathInput {
    pub transport: Transport,
    pub state: PathState,
    pub rtt_ewma_ms: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    /// Fire on every listed plane, first verified answer wins.
    Race(Vec<Transport>),
    Single(Transport),
    None,
}

/// `select_path(eligible, op_class, pick) -> (Route, reason)` — the pure
/// decision predicate (registered in `seam-registry.yaml`). `eligible` is the
/// set canon's `select_transport` rules 1/3 derive (planes BOTH peers support
/// for the plane); Track3/NoShared never reach here. `pick` is the per-key
/// monotonic pick counter that drives the deterministic exploration floor.
///
/// Rules (spec §3.1):
/// - 0 eligible → `None`/`no_plane`; 1 eligible → `Single`/`only_plane`.
/// - `Small` → `Race(all)`; an `Unknown` plane is IN the race, never last.
/// - `Bulk` → `Single(best)`: lowest `rtt_ewma_ms` among `Sampled`; when
///   every known plane is `Degraded` an `Unknown` plane is preferred over
///   them (`unknown_over_degraded`); all-`Unknown` falls to canon's prior
///   (`prior_iroh`). Every `EXPLORE_EVERY`-th pick explores a non-best live
///   plane — an `Unknown` one first (`explore_unknown`, C4), else the other
///   `Sampled` one (`explore`); a `Degraded` plane is explored only every
///   2×`EXPLORE_EVERY` picks so a large transfer never lands on it twice in a
///   row (`explore_degraded`).
pub fn select_path(eligible: &[PathInput], class: OpClass, pick: u32) -> (Route, &'static str) {
    match eligible.len() {
        0 => return (Route::None, "no_plane"),
        1 => return (Route::Single(eligible[0].transport), "only_plane"),
        _ => {}
    }
    if class == OpClass::Small {
        let reason = if eligible.iter().any(|p| p.state == PathState::Unknown) {
            "race_unknown"
        } else {
            "race_small"
        };
        return (
            Route::Race(eligible.iter().map(|p| p.transport).collect()),
            reason,
        );
    }

    let sampled: Vec<&PathInput> = eligible
        .iter()
        .filter(|p| p.state == PathState::Sampled)
        .collect();
    let unknown: Vec<&PathInput> = eligible
        .iter()
        .filter(|p| p.state == PathState::Unknown)
        .collect();
    let degraded: Vec<&PathInput> = eligible
        .iter()
        .filter(|p| p.state == PathState::Degraded)
        .collect();

    let best = sampled
        .iter()
        .min_by(|a, b| {
            a.rtt_ewma_ms
                .unwrap_or(f64::MAX)
                .partial_cmp(&b.rtt_ewma_ms.unwrap_or(f64::MAX))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|p| p.transport);

    // C4: an Unknown plane IS the exploration pick until it earns a sample —
    // not on a tick, immediately. Measured 2026-08-29: with a tick-only floor
    // an 11-pull recovery never tried iroh on Bulk while its Small RTT read
    // 15 ms against libp2p's 816 ms; one sample is all Bulk needs to decide.
    if let (Some(_), Some(u)) = (best, unknown.first()) {
        return (Route::Single(u.transport), "explore_unknown");
    }
    // Exploration floor (C3): a non-best live plane keeps earning samples.
    let explore_tick = pick.is_multiple_of(EXPLORE_EVERY);
    if explore_tick {
        if let Some(b) = best {
            if let Some(other) = sampled.iter().find(|p| p.transport != b) {
                return (Route::Single(other.transport), "explore");
            }
            if pick.is_multiple_of(2 * EXPLORE_EVERY) {
                if let Some(d) = degraded.first() {
                    return (Route::Single(d.transport), "explore_degraded");
                }
            }
        }
    }

    if let Some(b) = best {
        return (Route::Single(b), "best_rtt");
    }
    // Nothing Sampled: an Unknown plane beats a Degraded one (C4 — absence is
    // not slowness); all-Unknown takes canon's prior.
    if !unknown.is_empty() {
        let reason = if degraded.is_empty() {
            "prior_iroh"
        } else {
            "unknown_over_degraded"
        };
        let t = unknown
            .iter()
            .find(|p| p.transport == Transport::Iroh)
            .or(unknown.first())
            .map(|p| p.transport)
            .unwrap_or(Transport::Libp2p);
        return (Route::Single(t), reason);
    }
    // All Degraded: least bad by RTT (C11 lowers weight, never refuses).
    let least_bad = degraded
        .iter()
        .min_by(|a, b| {
            a.rtt_ewma_ms
                .unwrap_or(f64::MAX)
                .partial_cmp(&b.rtt_ewma_ms.unwrap_or(f64::MAX))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|p| p.transport)
        .unwrap_or(eligible[0].transport);
    (Route::Single(least_bad), "all_degraded")
}

/// `ELOHIM_TRANSPORT_SELECTION=off|0|false|no` → static prior only.
pub fn selection_enabled() -> bool {
    match std::env::var("ELOHIM_TRANSPORT_SELECTION") {
        Ok(v) => !matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "off" | "0" | "false" | "no"
        ),
        Err(_) => true,
    }
}

/// One row of `/p2p/status.transportPaths` — the peer's-eye view of its own
/// network, one per `(peer, transport, class)` that has a key.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportPathView {
    pub peer: String,
    pub transport: Transport,
    pub op_class: OpClass,
    pub state: PathState,
    pub rtt_ewma_ms: Option<f64>,
    pub success_rate: Option<f64>,
    pub last_sample_age_s: Option<u64>,
    pub saturation_hint: bool,
    pub attempts: u64,
}

type Key = (String, Transport, OpClass);

#[derive(Default)]
pub struct PathObservations {
    inner: Mutex<HashMap<Key, PathObservation>>,
    picks: Mutex<HashMap<(String, OpClass), u32>>,
}

static GLOBAL: OnceLock<PathObservations> = OnceLock::new();

/// The process-wide store — one per node, shared by every plane's legs.
pub fn global() -> &'static PathObservations {
    GLOBAL.get_or_init(PathObservations::default)
}

impl PathObservations {
    /// Record one sample. `rtt` is only meaningful on `ok`; a `None` RTT on a
    /// success still counts toward the success rate (the caller had no clock).
    pub fn record(
        &self,
        peer: &str,
        transport: Transport,
        class: OpClass,
        rtt: Option<Duration>,
        ok: bool,
        saturation_hint: bool,
    ) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let key = (peer.to_string(), transport, class);
        if !inner.contains_key(&key) && inner.len() >= MAX_KEYS {
            return; // backstop — the peer book is the real bound
        }
        inner
            .entry(key)
            .or_default()
            .record(rtt, ok, saturation_hint);
        if let (true, Some(rtt)) = (ok, rtt) {
            crate::metrics::observe_transport_path_rtt(
                peer,
                transport.as_str(),
                class.as_str(),
                rtt.as_secs_f64() * 1000.0,
            );
        }
    }

    pub fn inputs(&self, peer: &str, eligible: &[Transport], class: OpClass) -> Vec<PathInput> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        eligible
            .iter()
            .map(|t| {
                let obs = inner.get(&(peer.to_string(), *t, class));
                PathInput {
                    transport: *t,
                    state: obs
                        .map(PathObservation::state)
                        .unwrap_or(PathState::Unknown),
                    rtt_ewma_ms: obs.and_then(|o| o.rtt_ewma_ms),
                }
            })
            .collect()
    }

    /// Decide a route for `peer` over `eligible`, advancing the per-key pick
    /// counter and counting the decision (C8). With selection disabled a Bulk
    /// decision is the static prior (iroh when eligible) — today's behaviour —
    /// labelled `selection_off` so the before/after is legible in the metric.
    pub fn route(&self, peer: &str, eligible: &[Transport], class: OpClass) -> Route {
        if eligible.is_empty() {
            crate::metrics::inc_transport_route("none", class.as_str(), "no_plane");
            return Route::None;
        }
        let pick = {
            let mut picks = self.picks.lock().unwrap_or_else(|e| e.into_inner());
            let c = picks.entry((peer.to_string(), class)).or_insert(0);
            *c = c.wrapping_add(1);
            *c
        };
        let (route, reason) =
            if class == OpClass::Bulk && !selection_enabled() && eligible.len() > 1 {
                let t = if eligible.contains(&Transport::Iroh) {
                    Transport::Iroh
                } else {
                    eligible[0]
                };
                (Route::Single(t), "selection_off")
            } else {
                select_path(&self.inputs(peer, eligible, class), class, pick)
            };
        match &route {
            Route::Race(ts) => {
                for t in ts {
                    crate::metrics::inc_transport_route(t.as_str(), class.as_str(), reason);
                }
            }
            Route::Single(t) => {
                crate::metrics::inc_transport_route(t.as_str(), class.as_str(), reason)
            }
            Route::None => crate::metrics::inc_transport_route("none", class.as_str(), reason),
        }
        route
    }

    /// Count a route taken OUTSIDE `route()` — a fallback to the other plane
    /// after the selected one failed — so every decision is observable (C8).
    pub fn note_fallback(&self, transport: Transport, class: OpClass) {
        crate::metrics::inc_transport_route(transport.as_str(), class.as_str(), "fallback");
    }

    pub fn view(&self) -> Vec<TransportPathView> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let mut rows: Vec<TransportPathView> = inner
            .iter()
            .map(|((peer, transport, class), o)| TransportPathView {
                peer: peer.clone(),
                transport: *transport,
                op_class: *class,
                state: o.state(),
                rtt_ewma_ms: o.rtt_ewma_ms,
                success_rate: o.success_rate(),
                last_sample_age_s: o.last_sample_age().map(|d| d.as_secs()),
                saturation_hint: o.saturation_hint,
                attempts: o.attempts,
            })
            .collect();
        rows.sort_by(|a, b| {
            (a.peer.as_str(), a.transport.as_str(), a.op_class.as_str()).cmp(&(
                b.peer.as_str(),
                b.transport.as_str(),
                b.op_class.as_str(),
            ))
        });
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(t: Transport, s: PathState, rtt: Option<f64>) -> PathInput {
        PathInput {
            transport: t,
            state: s,
            rtt_ewma_ms: rtt,
        }
    }

    #[test]
    fn no_plane_and_only_plane() {
        assert_eq!(
            select_path(&[], OpClass::Bulk, 1),
            (Route::None, "no_plane")
        );
        let one = [p(Transport::Libp2p, PathState::Unknown, None)];
        assert_eq!(
            select_path(&one, OpClass::Bulk, 1),
            (Route::Single(Transport::Libp2p), "only_plane")
        );
        assert_eq!(
            select_path(&one, OpClass::Small, 1),
            (Route::Single(Transport::Libp2p), "only_plane")
        );
    }

    /// C4 honest absence — an Unknown plane is IN the Small race and is the
    /// Bulk exploration pick within EXPLORE_EVERY picks; it never sorts last.
    #[test]
    fn c4_unknown_races_on_small_and_is_explored_on_bulk_within_n_picks() {
        let both = [
            p(Transport::Libp2p, PathState::Sampled, Some(20.0)),
            p(Transport::Iroh, PathState::Unknown, None),
        ];
        let (route, reason) = select_path(&both, OpClass::Small, 1);
        assert_eq!(route, Route::Race(vec![Transport::Libp2p, Transport::Iroh]));
        assert_eq!(reason, "race_unknown");

        // Unknown is the Bulk pick IMMEDIATELY (until it earns a sample), on
        // every pick — never parked behind the exploration tick.
        for pick in 1..=EXPLORE_EVERY {
            assert_eq!(
                select_path(&both, OpClass::Bulk, pick),
                (Route::Single(Transport::Iroh), "explore_unknown"),
                "pick {pick}"
            );
        }
    }

    #[test]
    fn bulk_picks_lowest_rtt_and_all_unknown_takes_the_iroh_prior() {
        let both = [
            p(Transport::Libp2p, PathState::Sampled, Some(20.0)),
            p(Transport::Iroh, PathState::Sampled, Some(35.0)),
        ];
        assert_eq!(
            select_path(&both, OpClass::Bulk, 1),
            (Route::Single(Transport::Libp2p), "best_rtt")
        );
        // the exploration tick goes to the OTHER sampled plane
        assert_eq!(
            select_path(&both, OpClass::Bulk, EXPLORE_EVERY),
            (Route::Single(Transport::Iroh), "explore")
        );
        let unknown = [
            p(Transport::Libp2p, PathState::Unknown, None),
            p(Transport::Iroh, PathState::Unknown, None),
        ];
        assert_eq!(
            select_path(&unknown, OpClass::Bulk, 1),
            (Route::Single(Transport::Iroh), "prior_iroh")
        );
    }

    #[test]
    fn degraded_is_avoided_and_never_explored_twice_in_a_row() {
        let both = [
            p(Transport::Libp2p, PathState::Sampled, Some(50.0)),
            p(Transport::Iroh, PathState::Degraded, Some(5.0)),
        ];
        // fastest-by-RTT but Degraded: not the best
        assert_eq!(
            select_path(&both, OpClass::Bulk, 1),
            (Route::Single(Transport::Libp2p), "best_rtt")
        );
        // explored only on the 2×EXPLORE_EVERY tick, never on consecutive picks
        let mut last_was_degraded = false;
        for pick in 1..=(4 * EXPLORE_EVERY) {
            let (route, _) = select_path(&both, OpClass::Bulk, pick);
            let now = route == Route::Single(Transport::Iroh);
            assert!(
                !(now && last_was_degraded),
                "Degraded plane picked twice in a row at pick {pick}"
            );
            last_was_degraded = now;
        }
        assert_eq!(
            select_path(&both, OpClass::Bulk, 2 * EXPLORE_EVERY),
            (Route::Single(Transport::Iroh), "explore_degraded")
        );
        // Unknown beats Degraded when nothing is Sampled (C4)
        let mixed = [
            p(Transport::Libp2p, PathState::Degraded, Some(5.0)),
            p(Transport::Iroh, PathState::Unknown, None),
        ];
        assert_eq!(
            select_path(&mixed, OpClass::Bulk, 1),
            (Route::Single(Transport::Iroh), "unknown_over_degraded")
        );
    }

    /// C3 liveness — a Degraded plane that starts succeeding returns to
    /// Sampled without operator action (ring-based rate + hint clears).
    #[test]
    fn c3_degraded_plane_recovers_to_sampled_on_its_own() {
        let mut o = PathObservation::default();
        assert_eq!(o.state(), PathState::Unknown);
        o.record(None, false, false);
        assert_eq!(
            o.state(),
            PathState::Sampled,
            "one blip is not a degraded path"
        );
        for _ in 0..7 {
            o.record(None, false, false);
        }
        assert_eq!(o.state(), PathState::Degraded);
        o.record(Some(Duration::from_millis(10)), true, true);
        assert_eq!(
            o.state(),
            PathState::Degraded,
            "a saturation hint keeps it Degraded"
        );
        for _ in 0..12 {
            o.record(Some(Duration::from_millis(10)), true, false);
        }
        assert_eq!(o.state(), PathState::Sampled);
        assert!(o.rtt_ewma_ms.unwrap() > 9.0 && o.rtt_ewma_ms.unwrap() < 11.0);
        assert_eq!(o.ring.len(), RING);
    }

    #[test]
    fn store_routes_and_views_and_is_bounded() {
        let s = PathObservations::default();
        assert_eq!(s.route("peerA", &[], OpClass::Bulk), Route::None);
        assert_eq!(
            s.route(
                "peerA",
                &[Transport::Libp2p, Transport::Iroh],
                OpClass::Bulk
            ),
            Route::Single(Transport::Iroh)
        );
        s.record(
            "peerA",
            Transport::Libp2p,
            OpClass::Bulk,
            Some(Duration::from_millis(3)),
            true,
            false,
        );
        let v = s.view();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].state, PathState::Sampled);
        assert_eq!(v[0].attempts, 1);
    }
}
