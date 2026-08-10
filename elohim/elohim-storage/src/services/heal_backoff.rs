//! HEAL BACKOFF — replay a known `Ok(None)` own-conductor answer instead of
//! re-paying for it, so the content heal leg's fixed wall clock reaches ids
//! whose answer is NOT already known.
//!
//! This is drain lever 2. Its sibling lever is the widened advertiser probe
//! ([`crate::config::evidence_fallback_max_alternates`]). (A third sibling,
//! the heal-leg's own per-id resolve fan-out, was superseded by the batch
//! head-plane arms and its dead config knob removed 2026-08-09, R1 — see
//! `HealPacing::head_batch_fanout` in [`crate::p2p::projection_reconcile`]
//! for what actually paces that leg now.) Design intent:
//! `genesis/data/timeline/backlog/content-gap-limit-cycle-blocks-convergence.md`
//! §"RCA v3 → Open residuals → F-B".
//!
//! ## The waste this closes
//!
//! `heal_content` walks every pending gap id and asks its OWN conductor
//! `resolve_content_head_local`. The leg is bounded by a 120s wall clock on a
//! ~300s cadence, so at the live ~0.75s conductor latency it touched ~160 of
//! ~2.9k pending ids per sweep. Its DOMINANT outcome is `Ok(None)` — the
//! conductor holds no chain and no canonical link for the id (live: ~24.9k
//! `missing` per 2h fleet-wide, against 59 `healed`).
//!
//! That answer is a PREDICTABLE repeat within a sweep horizon. Nothing the heal
//! leg does can change it: acquiring a chain is the job of the author paths and
//! `try_obey_visible_election`, which run LATER in the same tick and on their own
//! budget. So the round-trip buys nothing except the answer we already have —
//! while the ids whose answer is genuinely unknown wait behind it.
//!
//! ## What a replay is, and what it is emphatically NOT
//!
//! A replay reproduces the `Ok(None)` arm's effect EXACTLY: the id counts as
//! conductor-missing, joins the ghost-candidate list, is routed to the adopt arm
//! when a peer advertises a declaration for it, and is `mark_failed` — honest,
//! because we did not heal it. Only the conductor call is skipped.
//!
//! It is therefore **not**:
//!
//! - **not an exclusion.** The id stays in the tracker, stays a gap, and stays a
//!   candidate for the adopt arm's `try_obey_visible_election` probe — the ONLY
//!   arm that converges this class — on every single sweep.
//! - **not a terminality claim.** Nothing is exhausted, tombstoned, or written
//!   off. This is the distinction the F-C residual insists on: the exhausted
//!   sawtooth was a *recount* that pretended to be a verdict, whereas this is a
//!   cached answer with an expiry and no opinion about the id's fate.
//! - **not a new truth.** It replays THIS conductor's own last observed answer,
//!   which was `Answer::Absent` at the local plane (see
//!   `head_adoption::LocalResolve`'s scope note) and is never authoritative about
//!   DHT-wide existence.
//!
//! ## Three automated exits, none of them human
//!
//! 1. **Window expiry** — [`crate::config::heal_missing_backoff_window`], default
//!    600s = 2 sweeps. Deliberately far shorter than the contest backoff's hour:
//!    this window bounds how STALE a replayed answer may be, and the answer is
//!    one the obey/author arms are actively trying to falsify in the same tick.
//! 2. **A different outcome** — [`note_resolved`] clears the entry the moment the
//!    conductor answers anything other than `Ok(None)` for the id.
//! 3. **Chain arrival** — [`note_resolved`] is also called from the author paths
//!    that land a local chain, alongside `contest_backoff::note_local_chain_arrived`.
//!    Without this an id authored at 12:00:30 would keep replaying "missing"
//!    until its window ran out.
//!
//! `window == 0` disables the module entirely: every id pays for a fresh resolve
//! every sweep, which is byte-for-byte the pre-lever leg. Disabling a lever must
//! restore the behaviour before it existed, never something worse.
//!
//! ## Process-local, and deliberately NOT persisted
//!
//! Unlike `contest_backoff` (which snapshots, because its 24h evidence-absent
//! class is expensive to re-learn) this ledger holds at most 10 minutes of
//! knowledge. Losing it on restart costs one sweep of round-trips — and a
//! restart is itself an event that plausibly changed what the conductor holds,
//! so re-asking is arguably correct. No file, no diesel table, no migration
//! timestamp spent.
//!
//! Overflow is fail-OPEN for the same reason as `contest_backoff`: past
//! [`HEAL_BACKOFF_CAP`] the ledger clears rather than evicting selectively, so
//! the worst case is the pre-lever behaviour (resolve everything every sweep),
//! never a silent permanent hold.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Hard cap on the ledger. The live pending set is ~2.9k ids per pod and the
/// fleet-wide contested set ~11.4k; 50k leaves generous headroom while making
/// unbounded growth impossible on a pathological corpus.
///
// bounded-work: memory budget = HEAL_BACKOFF_CAP entries (fail-open clear on
// overflow); time budget = `crate::config::heal_missing_backoff_window()` per
// entry, enforced by `should_replay` and proven finite by
// `a_replay_window_always_expires`. This module adds NO loop, NO retry ladder,
// and NO I/O: it is a pure skip decision plus a bounded map.
pub const HEAL_BACKOFF_CAP: usize = 50_000;

/// When this conductor last answered `Ok(None)` for an id.
static LEDGER: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();

fn ledger() -> &'static Mutex<HashMap<String, Instant>> {
    LEDGER.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Record that the own conductor answered `Ok(None)` for `id`.
///
/// Called from the heal leg's conductor-missing arm on a REAL answer only —
/// never from the replay path, so a replay can never extend its own window into
/// an unbounded hold. That is the single rule that keeps exit (1) reachable.
pub fn note_conductor_missing(id: &str) {
    let id = id.trim();
    if id.is_empty() {
        return;
    }
    let Ok(mut guard) = ledger().lock() else {
        // Poisoned lock degrades to "no replay" — the pre-lever behaviour.
        return;
    };
    if guard.len() >= HEAL_BACKOFF_CAP && !guard.contains_key(id) {
        tracing::warn!(
            cap = HEAL_BACKOFF_CAP,
            "heal_backoff: ledger hit its cap — clearing (fail-open; every id returns to \
             resolve-every-sweep)"
        );
        guard.clear();
    }
    guard.insert(id.to_string(), Instant::now());
}

/// Clear any standing replay for `id`.
///
/// Two call classes, one meaning ("what we cached is now false"): the heal leg
/// calls it whenever the conductor answers something other than `Ok(None)`, and
/// the author paths call it when they land a local chain.
pub fn note_resolved(id: &str) {
    let id = id.trim();
    if id.is_empty() {
        return;
    }
    if let Ok(mut guard) = ledger().lock() {
        guard.remove(id);
    }
}

/// May the heal leg replay a cached conductor-missing answer for `id`?
///
/// `window` is passed rather than read from config so the OFF behaviour is
/// provable without touching a process-wide `OnceLock` — the parallel-test flake
/// this codebase has already paid for once (see `head_adoption::fallback_warrants`).
pub fn should_replay(id: &str, window: Duration) -> bool {
    if window.is_zero() {
        return false;
    }
    let id = id.trim();
    let Ok(guard) = ledger().lock() else {
        return false;
    };
    guard.get(id).is_some_and(|seen| seen.elapsed() < window)
}

/// Ids currently holding a replay entry. Observability and test assertion only.
pub fn tracked() -> usize {
    ledger().lock().map(|g| g.len()).unwrap_or(0)
}

/// Drop every entry. Test-support only — the production exits are expiry,
/// [`note_resolved`], and the fail-open cap clear.
#[cfg(test)]
pub(crate) fn reset_for_test() {
    if let Ok(mut guard) = ledger().lock() {
        guard.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serializes this module's tests: `reset_for_test` clears the ONE global
    /// ledger, so a parallel sibling's reset landing between another test's
    /// note and assert wipes its entry (live flake, pre-push gate 2026-08-08).
    static LEDGER_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// The whole lever in one assertion: a fresh id costs a conductor
    /// round-trip, and a second sweep within the window does not.
    #[test]
    fn a_known_missing_id_is_replayed_within_its_window() {
        let _guard = LEDGER_TEST_LOCK.lock().unwrap();
        reset_for_test();
        let window = Duration::from_secs(600);
        let id = "heal-backoff:replayed";

        assert!(
            !should_replay(id, window),
            "an id this conductor has never answered for must be resolved, not replayed"
        );
        note_conductor_missing(id);
        assert!(
            should_replay(id, window),
            "the same predictable answer must not be re-purchased inside the window"
        );
    }

    /// C3, the rule the F-C residual insists on: a deferral is never a
    /// terminality. An elapsed window re-admits the id with no intervention.
    #[test]
    fn a_replay_window_always_expires() {
        let _guard = LEDGER_TEST_LOCK.lock().unwrap();
        reset_for_test();
        let id = "heal-backoff:expires";
        note_conductor_missing(id);

        // A real elapse, not a mocked clock: sleep past a 1ms window so the
        // assertion exercises the same `Instant::elapsed` comparison the sweep
        // does. Milliseconds keep it instant while the production window is
        // minutes.
        std::thread::sleep(Duration::from_millis(5));
        assert!(
            !should_replay(id, Duration::from_millis(1)),
            "an entry older than its window must be re-resolved, never held"
        );
    }

    /// The OFF switch must restore the pre-lever leg EXACTLY: a zero window
    /// replays nothing at all, even for an id the ledger knows.
    #[test]
    fn a_zero_window_disables_every_replay() {
        let _guard = LEDGER_TEST_LOCK.lock().unwrap();
        reset_for_test();
        let id = "heal-backoff:disabled";
        note_conductor_missing(id);
        assert!(
            !should_replay(id, Duration::ZERO),
            "window 0 is the OFF switch — it must resolve every id every sweep"
        );
    }

    /// Exit (2)/(3): a chain arriving, or any non-missing conductor answer,
    /// invalidates the cached verdict immediately rather than at window expiry.
    #[test]
    fn a_resolved_id_stops_replaying_at_once() {
        let _guard = LEDGER_TEST_LOCK.lock().unwrap();
        reset_for_test();
        let window = Duration::from_secs(600);
        let id = "heal-backoff:cleared";
        note_conductor_missing(id);
        assert!(should_replay(id, window));

        note_resolved(id);
        assert!(
            !should_replay(id, window),
            "an author path landed a chain (or the conductor answered) — the cached \
             'missing' is now false and must not survive the window"
        );
    }

    /// Whitespace must not mint two entries for one id: the heal leg and the
    /// author paths reach this module from different call sites, and a trailing
    /// space on one of them would silently strand the replay.
    #[test]
    fn ids_are_matched_trimmed() {
        let _guard = LEDGER_TEST_LOCK.lock().unwrap();
        reset_for_test();
        let window = Duration::from_secs(600);
        note_conductor_missing("  heal-backoff:trimmed  ");
        assert!(should_replay("heal-backoff:trimmed", window));
        note_resolved("heal-backoff:trimmed\n");
        assert!(!should_replay("  heal-backoff:trimmed", window));
    }

    /// An empty id is not an id. Recording one would let a single malformed row
    /// occupy a ledger slot forever.
    #[test]
    fn an_empty_id_is_never_recorded() {
        let _guard = LEDGER_TEST_LOCK.lock().unwrap();
        reset_for_test();
        let before = tracked();
        note_conductor_missing("   ");
        assert_eq!(tracked(), before, "a blank id must not enter the ledger");
        assert!(!should_replay("   ", Duration::from_secs(600)));
    }
}
