//! ADVERTISER HEALTH — a process-local, bounded, decaying record of which peers
//! actually SERVE head-record bytes, used to pick who to ask when the hinted
//! advertiser cannot.
//!
//! Design intent: `genesis/data/timeline/backlog/adopt-before-author-evidence-starvation.md`
//! §"Addendum 2026-08-03 (evening)" — item 2, requester-side advertiser
//! diversity.
//!
//! ## The waste this closes
//!
//! The adopt-evidence path asks the FIRST peer that advertised a declaration for
//! an id (`projection_reconcile`'s or-insert harvest). On the live fleet that
//! peer was usually a 100%-CFS-throttled conductor: `adopt_evidence{budget_elapsed}`
//! dominated the population while idle peers — very likely holding the same
//! genesis-seeded rows and records — were never asked once. Adopt-before-author
//! has therefore never minted, not because its design is wrong but because it
//! only ever asked the weakest member.
//!
//! The protocol adapts. This module is the memory that makes "ask someone else"
//! a choice rather than a coin flip: a peer whose answers have carried bytes is
//! preferred over one whose answers have not.
//!
//! ## What it is NOT
//!
//! - **Not gossip.** Nothing here is exchanged, published, or believed from a
//!   peer's own claim. It is this node's first-hand experience of who answered
//!   with bytes, and it never leaves the process.
//! - **Not reputation, and never an exclusion.** A peer with a poor carried
//!   share is de-PRIORITISED as a *courier of bytes*, never refused, never
//!   distrusted, and never excluded from any other plane. The primary advertiser
//!   is still asked FIRST, every time; this only chooses who to ask second.
//! - **Not a truth store.** Losing it on restart costs at most a few
//!   less-informed alternate picks.
//!
//! ## Bounded by construction
//!
// bounded-work: memory budget = HEALTH_PEER_CAP entries (fail-open clear on
// overflow), each a fixed-size struct; observation budget = counts are halved at
// DECAY_AT so they can never exceed it, i.e. the health of a peer is always a
// function of its ~last DECAY_AT answers. This module adds NO loop, NO retry
// ladder, and NO I/O: `record` is a map upsert and `choose` is one pass over a
// candidate slice that the caller already bounds (`PeerHeadHint::alternates`,
// itself capped at harvest time). The ONE fallback fetch it enables is bounded
// at its call site in `head_adoption::resolve_peer_evidence`.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Hard cap on the health table. The alpha fabric is 6 peers and a large mesh is
/// human-scale by construction, so this is generous headroom rather than a
/// working limit — its job is to make unbounded growth impossible if a
/// pathological peer set ever appears. On overflow the table CLEARS (fail-open):
/// every peer becomes unproven again, which is the pre-lever behaviour, not a
/// stranding. Mirrors `contest_backoff::CONTEST_BACKOFF_CAP`'s discipline.
pub const HEALTH_PEER_CAP: usize = 512;

/// Observations after which a peer's counts are halved.
///
/// This is the DECAY, and it is what makes the health a rolling window rather
/// than a lifetime tally: a peer that was saturated for an hour and then
/// recovered must become preferable again within a few sweeps, not after its
/// historical failures are outnumbered. 64 is ~8 sweeps of a fan-out-8 arm.
const DECAY_AT: u32 = 64;

/// The score given to a peer we have never asked.
///
/// An unproven peer is neither vouched for nor condemned, and 0.5 places it
/// exactly there: it loses to a peer that has actually carried bytes more than
/// half the time, and beats one that has carried them less than half. This is
/// what keeps a fresh peer reachable (an all-zero prior would make the first
/// peer we ever ask permanently preferred) without letting it displace a proven
/// courier.
const UNTRACKED_PRIOR: f64 = 0.5;

/// One peer's rolling evidence record: how many of its recent head-record
/// answers carried bytes, and how many did not.
///
/// `degraded` deliberately includes `no_record`. This is a measure of BYTE
/// SUPPLY, not of virtue — a peer that honestly reports it holds no record is a
/// peer that cannot serve us bytes, and for the purpose of "who should we ask
/// next" those are the same fact. Nothing else in the system reads this type, so
/// the reading cannot leak into any judgement about the peer itself.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PeerEvidenceHealth {
    /// Answers that carried `Record` bytes.
    pub carried: u32,
    /// Answers that did not (any reason, including a stated `no_record`).
    pub degraded: u32,
}

impl PeerEvidenceHealth {
    /// Observations behind this record.
    pub fn total(&self) -> u32 {
        self.carried.saturating_add(self.degraded)
    }

    /// Share of recent answers that carried bytes, or `None` when this peer has
    /// never been asked (which is DISTINCT from a 0.0 share — unproven is not
    /// proven-bad, and [`choose`] treats them differently).
    pub fn carried_share(&self) -> Option<f64> {
        let total = self.total();
        if total == 0 {
            None
        } else {
            Some(f64::from(self.carried) / f64::from(total))
        }
    }

    /// Fold one answer in, decaying when the window fills.
    fn observe(&mut self, carried: bool) {
        if carried {
            self.carried = self.carried.saturating_add(1);
        } else {
            self.degraded = self.degraded.saturating_add(1);
        }
        if self.total() >= DECAY_AT {
            self.carried /= 2;
            self.degraded /= 2;
        }
    }
}

fn table() -> &'static Mutex<HashMap<String, PeerEvidenceHealth>> {
    static TABLE: OnceLock<Mutex<HashMap<String, PeerEvidenceHealth>>> = OnceLock::new();
    TABLE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Record what one peer's head-record answer established: `carried == true` iff
/// it served `Record` bytes.
///
/// Called from the single evidence-classification site
/// (`head_adoption::resolve_peer_evidence`), for BOTH the primary and any
/// fallback ask — so the table's denominator is exactly the adopt-evidence
/// meter's, per peer.
///
/// A poisoned lock is ignored: the health table is an optimisation, and losing a
/// datum costs a less-informed pick, never correctness.
pub fn record(peer_id: &str, carried: bool) {
    let peer_id = peer_id.trim();
    if peer_id.is_empty() {
        return;
    }
    let Ok(mut guard) = table().lock() else {
        tracing::warn!(
            "advertiser_health: table lock poisoned; dropping this observation (health is an \
             optimisation, not correctness)"
        );
        return;
    };
    if guard.len() >= HEALTH_PEER_CAP && !guard.contains_key(peer_id) {
        // FAIL-OPEN, exactly as the contest-backoff ledger does: clearing
        // returns every peer to unproven (the pre-lever behaviour), which can
        // never strand one.
        tracing::warn!(
            cap = HEALTH_PEER_CAP,
            "advertiser_health: table hit its cap — clearing (fail-open; every peer returns to \
             unproven)"
        );
        guard.clear();
    }
    guard
        .entry(peer_id.to_string())
        .or_default()
        .observe(carried);
}

/// This peer's current record, if it has one. Observability and test assertion.
pub fn health(peer_id: &str) -> Option<PeerEvidenceHealth> {
    let guard = table().lock().ok()?;
    guard.get(peer_id.trim()).copied()
}

/// Peers currently tracked. Observability and test assertion only.
pub fn tracked() -> usize {
    table().lock().map(|g| g.len()).unwrap_or(0)
}

/// Pick the healthiest DISTINCT alternative from `candidates`, or `None` when
/// there is no distinct candidate at all.
///
/// Reads the process-local table; the decision itself is [`choose`], which is
/// pure and is where the rule is tested.
pub fn select_alternative(primary: &str, candidates: &[String]) -> Option<String> {
    let scores: HashMap<String, f64> = {
        let Ok(guard) = table().lock() else {
            // Lock poisoned: fall through with no scores, so `choose` picks the
            // first distinct candidate. Degraded, never dead.
            return choose(primary, candidates, |_| None).map(str::to_string);
        };
        candidates
            .iter()
            .filter_map(|c| {
                guard
                    .get(c.trim())
                    .and_then(PeerEvidenceHealth::carried_share)
                    .map(|s| (c.trim().to_string(), s))
            })
            .collect()
    };
    choose(primary, candidates, |peer| scores.get(peer).copied()).map(str::to_string)
}

/// THE SELECTION RULE, pure and total.
///
/// **Concerns:** C6a (one pass over a caller-bounded slice; no allocation, no
/// I/O). C8 (deterministic — a tie resolves to the earliest candidate, so two
/// runs over the same inputs pick the same peer and a live decision is
/// reproducible from a log line).
///
/// **Contract tests:** [`tests::the_healthiest_distinct_candidate_wins`],
/// [`tests::an_unproven_peer_outranks_a_proven_starving_one`],
/// [`tests::the_primary_is_never_its_own_alternative`],
/// [`tests::no_distinct_candidate_means_no_alternative`].
///
/// `score` returns the candidate's carried-share, or `None` for a peer never
/// asked — which scores as [`UNTRACKED_PRIOR`] rather than zero. Strict `>`
/// makes the FIRST candidate win a tie, which is the deterministic order the
/// doc-comment promises.
pub(crate) fn choose<'a, F>(primary: &str, candidates: &'a [String], score: F) -> Option<&'a str>
where
    F: Fn(&str) -> Option<f64>,
{
    let primary = primary.trim();
    let mut best: Option<(&'a str, f64)> = None;
    // bounded-work: exactly one pass over `candidates`, which the caller bounds
    // (`PeerHeadHint::alternates`, capped at harvest time). No retry, no loop.
    for candidate in candidates {
        let candidate = candidate.trim();
        if candidate.is_empty() || candidate == primary {
            continue;
        }
        let s = score(candidate).unwrap_or(UNTRACKED_PRIOR);
        match best {
            Some((_, best_score)) if s <= best_score => {}
            _ => best = Some((candidate, s)),
        }
    }
    best.map(|(peer, _)| peer)
}

/// The healthiest DISTINCT alternatives from `candidates`, best-first, at most
/// `max` of them — the plural of [`select_alternative`].
///
/// **Concerns:** C6a (the returned length IS the amplification bound: one extra
/// fetch per element, and the caller may not exceed it). C8 (deterministic —
/// same ordering rule as [`choose`], so a tie keeps the earliest candidate and a
/// live decision is reproducible from a log line).
///
/// **Contract tests:** [`tests::ranking_orders_by_health_and_never_repeats`],
/// [`tests::a_zero_budget_ranks_nothing`].
///
/// Why a second function rather than calling [`choose`] in a loop: `choose` is
/// the SINGLE-pick rule and its callers depend on it staying that (it is the one
/// asserted by the existing selection tests). Ranking is a different question —
/// "in what order would I ask them all?" — and answering it once, totally, is
/// what keeps the widened probe loop-free at the call site.
pub fn select_alternatives(primary: &str, candidates: &[String], max: usize) -> Vec<String> {
    if max == 0 {
        return Vec::new();
    }
    let scores: HashMap<String, f64> = match table().lock() {
        Ok(guard) => candidates
            .iter()
            .filter_map(|c| {
                guard
                    .get(c.trim())
                    .and_then(PeerEvidenceHealth::carried_share)
                    .map(|s| (c.trim().to_string(), s))
            })
            .collect(),
        // Lock poisoned: rank with no scores, which degrades to candidate order.
        // Degraded, never dead — the same rule `select_alternative` follows.
        Err(_) => HashMap::new(),
    };
    rank(primary, candidates, max, |peer| scores.get(peer).copied())
        .into_iter()
        .map(str::to_string)
        .collect()
}

/// THE RANKING RULE, pure and total. See [`select_alternatives`].
///
// bounded-work: one pass to filter + one sort over `candidates`, which the
// caller bounds (`PeerHeadHint::alternates`, capped at harvest time), then
// truncation to `max`. No I/O, no retry, no loop over the network.
pub(crate) fn rank<'a, F>(
    primary: &str,
    candidates: &'a [String],
    max: usize,
    score: F,
) -> Vec<&'a str>
where
    F: Fn(&str) -> Option<f64>,
{
    let primary = primary.trim();
    let mut seen: Vec<&'a str> = Vec::new();
    let mut scored: Vec<(&'a str, f64)> = Vec::new();
    for candidate in candidates {
        let candidate = candidate.trim();
        // DISTINCTNESS is load-bearing, not tidiness: a duplicate entry would
        // spend one of the caller's bounded fetches re-asking a peer that just
        // answered, which is the amplification this bound exists to prevent.
        if candidate.is_empty() || candidate == primary || seen.contains(&candidate) {
            continue;
        }
        seen.push(candidate);
        scored.push((candidate, score(candidate).unwrap_or(UNTRACKED_PRIOR)));
    }
    // STABLE sort, descending by health: equal scores keep candidate order, so
    // the first-ranked element is byte-identical to what `choose` picks.
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    scored.truncate(max);
    scored.into_iter().map(|(peer, _)| peer).collect()
}

/// Drop every entry. Test isolation only — the table is process-local and shared
/// across `#[test]` fns in one binary. MUST be called while holding
/// [`test_exclusive`], for the reason `contest_backoff::reset` documents.
#[cfg(test)]
pub(crate) fn reset() {
    if let Ok(mut guard) = table().lock() {
        guard.clear();
    }
}

/// Serialises the tests that need the WHOLE table to themselves.
#[cfg(test)]
pub(crate) fn test_exclusive() -> std::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peers(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    /// The whole point of the lever: the alternative with the best carried share
    /// is the one asked. A rule that picked arbitrarily would route around a
    /// throttled peer onto another throttled peer half the time.
    #[test]
    fn the_healthiest_distinct_candidate_wins() {
        let candidates = peers(&["starved", "idle", "middling"]);
        let picked = choose("primary", &candidates, |p| match p {
            "starved" => Some(0.0),
            "middling" => Some(0.4),
            "idle" => Some(0.95),
            _ => None,
        });
        assert_eq!(picked, Some("idle"));
    }

    /// Unproven is not proven-bad. A peer we have never asked must beat one we
    /// have asked and which never carried — otherwise the first peer we happen
    /// to ask becomes permanently preferred and the diversity never materialises.
    #[test]
    fn an_unproven_peer_outranks_a_proven_starving_one() {
        let candidates = peers(&["starved", "unproven"]);
        let picked = choose("primary", &candidates, |p| match p {
            "starved" => Some(0.0),
            _ => None,
        });
        assert_eq!(picked, Some("unproven"));

        // ...and a PROVEN courier still beats an unproven one.
        let candidates = peers(&["unproven", "proven"]);
        let picked = choose("primary", &candidates, |p| match p {
            "proven" => Some(0.9),
            _ => None,
        });
        assert_eq!(picked, Some("proven"));
    }

    /// Ties resolve deterministically to the first candidate, so the same inputs
    /// always name the same peer — a live decision must be reproducible from its
    /// log line.
    #[test]
    fn ties_resolve_to_the_first_candidate() {
        let candidates = peers(&["a", "b", "c"]);
        for _ in 0..8 {
            assert_eq!(choose("primary", &candidates, |_| Some(0.5)), Some("a"));
        }
        // Untracked everywhere is the same tie.
        assert_eq!(choose("primary", &candidates, |_| None), Some("a"));
    }

    /// The amplification bound's first half: re-asking the SAME peer is not a
    /// fallback, it is a retry — and this arm has none. The primary must be
    /// filtered out even when it appears in its own alternates list (a harvest
    /// bug must not become a retry ladder).
    #[test]
    fn the_primary_is_never_its_own_alternative() {
        let candidates = peers(&["primary", " primary ", "other"]);
        assert_eq!(choose("primary", &candidates, |_| None), Some("other"));
        assert_eq!(
            choose("primary", &peers(&["primary"]), |_| None),
            None,
            "a candidate list containing only the primary offers no alternative"
        );
    }

    /// No candidate ⇒ no pick. The caller counts `no_alternative` and returns the
    /// primary's outcome unchanged, which is exactly the pre-lever behaviour.
    #[test]
    fn no_distinct_candidate_means_no_alternative() {
        assert_eq!(choose("primary", &[], |_| None), None);
        assert_eq!(
            choose("primary", &peers(&["", "   "]), |_| None),
            None,
            "blank peer ids are not candidates"
        );
    }

    /// The rolling window is a WINDOW: counts cannot grow without bound, so a
    /// peer's health always reflects its recent answers rather than its history.
    /// Without the decay a peer saturated for an hour would stay de-prioritised
    /// long after it recovered.
    #[test]
    fn health_counts_decay_and_stay_bounded() {
        let mut h = PeerEvidenceHealth::default();
        assert_eq!(h.carried_share(), None, "unasked is not zero");
        // bounded-work: fixed 500-iteration probe of the decay invariant.
        for i in 0..500 {
            h.observe(i % 4 == 0);
            assert!(
                h.total() < DECAY_AT,
                "counts must stay inside the decay window"
            );
        }
        let share = h.carried_share().expect("observed peer has a share");
        assert!(
            (0.1..0.4).contains(&share),
            "a 1-in-4 carrier should read near 0.25, got {share}"
        );
    }

    /// A recovered peer becomes preferable again within the window — the
    /// property that makes this a health signal rather than a blacklist.
    #[test]
    fn a_recovered_peer_becomes_preferable_again() {
        let mut h = PeerEvidenceHealth::default();
        for _ in 0..40 {
            h.observe(false);
        }
        assert!(h.carried_share().unwrap() < UNTRACKED_PRIOR);
        for _ in 0..60 {
            h.observe(true);
        }
        assert!(
            h.carried_share().unwrap() > UNTRACKED_PRIOR,
            "a peer that has recovered must outrank an unproven one again"
        );
    }

    /// Through the real table: recording makes a peer selectable by health, and
    /// the table is what `select_alternative` reads.
    #[test]
    fn recorded_health_drives_the_live_selection() {
        let _exclusive = test_exclusive();
        reset();
        for _ in 0..5 {
            record("live:idle", true);
            record("live:starved", false);
        }
        assert_eq!(
            health("live:idle").map(|h| h.carried),
            Some(5),
            "carried answers are recorded"
        );
        assert_eq!(
            select_alternative("live:primary", &peers(&["live:starved", "live:idle"])),
            Some("live:idle".to_string())
        );
        reset();
    }

    /// Bounded memory, failing OPEN — the same direction the contest-backoff
    /// ledger fails in. Worst case every peer is unproven again.
    #[test]
    fn the_table_is_bounded_and_fails_open() {
        let _exclusive = test_exclusive();
        reset();
        // bounded-work: exactly HEALTH_PEER_CAP iterations — the fill loop that
        // proves the cap, bounded by the constant it asserts.
        for n in 0..HEALTH_PEER_CAP {
            record(&format!("cap:{n}"), true);
        }
        assert_eq!(tracked(), HEALTH_PEER_CAP);
        record("cap:overflow", true);
        assert!(tracked() <= HEALTH_PEER_CAP);
        assert_eq!(
            health("cap:0"),
            None,
            "overflow must RELEASE peers (fail-open), never keep a stale verdict"
        );
        reset();
    }

    /// DRAIN LEVER 3's rule. Widening the probe is only worth anything if the
    /// SECOND ask goes to the second-healthiest peer — and it must never spend
    /// one of its bounded fetches on a repeat or on the primary itself.
    #[test]
    fn ranking_orders_by_health_and_never_repeats() {
        let candidates = peers(&["starved", "idle", "middling", "idle", "primary", "  "]);
        let ranked = rank("primary", &candidates, 3, |p| match p {
            "starved" => Some(0.0),
            "middling" => Some(0.4),
            "idle" => Some(0.95),
            _ => None,
        });
        assert_eq!(
            ranked,
            vec!["idle", "middling", "starved"],
            "best-first, primary excluded, duplicates and blanks dropped"
        );
        assert_eq!(
            ranked.first().copied(),
            choose("primary", &candidates, |p| match p {
                "starved" => Some(0.0),
                "middling" => Some(0.4),
                "idle" => Some(0.95),
                _ => None,
            }),
            "the first-ranked peer must be exactly the one the single-pick rule chooses — \
             widening the probe may extend the ladder, never re-order its top rung"
        );
    }

    /// The OFF switch is a bound, not a branch: `max = 0` ranks nothing, so the
    /// caller's loop body never runs and the path is the pre-lever single ask.
    #[test]
    fn a_zero_budget_ranks_nothing() {
        let candidates = peers(&["idle", "middling"]);
        assert!(rank("primary", &candidates, 0, |_| None).is_empty());
        assert!(select_alternatives("primary", &candidates, 0).is_empty());
    }

    /// The bound the caller relies on: `max` caps the returned length even when
    /// far more distinct candidates exist. This IS the amplification bound —
    /// every element is one extra round-trip.
    #[test]
    fn the_ranking_never_exceeds_its_budget() {
        let candidates = peers(&["a", "b", "c", "d", "e"]);
        assert_eq!(rank("primary", &candidates, 2, |_| None).len(), 2);
    }
}
