//! REANCHOR HELD BACKOFF — skip a reanchor candidate whose adopt pre-flight
//! already returned `Held`, so the sweep's conductor probes reach candidates
//! whose answer is NOT already known.
//!
//! ## The waste this closes (2026-09-18 defect, household mesh)
//!
//! [`crate::services::reanchor_backfill::run_once`] runs the adopt pre-flight
//! ([`crate::services::head_adoption::try_adopt_canonical_head`]) for EVERY
//! candidate on EVERY sweep and remembers nothing about the ones it held. A
//! `Held` verdict costs two conductor round-trips before any existing ledger can
//! gate it — the unconditional `resolve_content_head` probe at step (1), then an
//! election resolve for the conductor-missing class — because `contest_backoff`
//! is consulted INSIDE `contest_peer`, downstream of both. So 29-43 dead-anchor
//! candidates re-bought the same verdict every sweep, forever, and the probes
//! rode the INTERACTIVE admission lane while doing it.
//!
//! ## What a skip is, and what it is not
//!
//! A skip reproduces the `Held` arm's effect exactly: the row is not authored,
//! not stamped, and stays in both `remaining`/`dead_remaining` recounts, so
//! `caughtUp` and `deadRemainingStuck` are untouched. Only the conductor calls
//! are elided. It is therefore **not** an exclusion, **not** a terminality
//! claim, and **not** a new truth — it replays this node's own last pre-flight
//! verdict, which was never authoritative about anything but this sweep.
//!
//! ## Three automated exits, none of them human
//!
//! 1. **Window expiry** — [`crate::config::reanchor_held_backoff_window`]
//!    (`REANCHOR_HELD_BACKOFF_SECONDS`, default 900s = 3 sweeps at the 300s
//!    cadence). `0` DISABLES the module entirely, restoring the pre-fix sweep
//!    byte-for-byte. Disabling a lever must restore the behaviour before it
//!    existed, never something worse.
//! 2. **Progress** — [`note_progress`] clears the entry the moment an author
//!    path stamps the row, called beside the existing
//!    `heal_backoff::note_resolved` hook.
//! 3. **A CHANGED precondition** — the hold is recorded against the
//!    peer-advertised head that produced it, so a peer now advertising a
//!    DIFFERENT head (or no advertiser at all) re-admits the candidate on the
//!    very next sweep rather than at window expiry. The DHT is the manifest; new
//!    evidence must reach the controller immediately.
//!
//! ## Process-local, and deliberately NOT persisted
//!
//! Like [`crate::services::heal_backoff`] and unlike
//! [`crate::services::contest_backoff`], this holds at most a few sweeps of
//! knowledge. Losing it on restart costs one sweep of round-trips — and a
//! restart plausibly changed what the conductor holds, so re-asking is arguably
//! correct. No file, no diesel table, no migration timestamp spent.
//!
//! Overflow is fail-OPEN: past [`REANCHOR_BACKOFF_CAP`] the ledger clears rather
//! than evicting selectively, so the worst case is the pre-fix behaviour (probe
//! every candidate every sweep), never a silent permanent hold.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Hard cap on the ledger. The live dead-anchor population is tens of rows per
/// pod and the fleet-wide contested set ~11.4k; 50k leaves generous headroom
/// while making unbounded growth impossible on a pathological corpus. Mirrors
/// [`crate::services::heal_backoff::HEAL_BACKOFF_CAP`].
///
// bounded-work: memory budget = REANCHOR_BACKOFF_CAP entries (fail-open clear on
// overflow); time budget = `crate::config::reanchor_held_backoff_window()` per
// entry, enforced by `should_skip` and proven finite by
// `a_held_backoff_always_expires_and_zero_disables_it`. This module adds NO
// loop, NO retry ladder, and NO I/O: it is a pure skip decision plus a bounded
// map. The paced sweep it serves (`reanchor_backfill::run_once`:
// `max_per_sweep` candidates under `witness_sweep_budget`) is unchanged and
// stays authoritative — this only ELIDES round-trips whose answer is known.
pub const REANCHOR_BACKOFF_CAP: usize = 50_000;

/// One held candidate: when the pre-flight held it, and what it was held
/// against.
#[derive(Debug, Clone)]
struct HeldEntry {
    at: Instant,
    /// The peer-advertised head this candidate was held against (`""` when the
    /// sweep carried no hint for it). Exit (3) keys on this.
    against: String,
}

fn ledger() -> &'static Mutex<HashMap<String, HeldEntry>> {
    static LEDGER: OnceLock<Mutex<HashMap<String, HeldEntry>>> = OnceLock::new();
    LEDGER.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Record that the adopt pre-flight HELD `id` against the advertised head
/// `against` (`""` when the sweep had no hint).
///
/// Called from the sweep's real `Held`/`Contested` arm only — never from the
/// skip path, so a skip can never extend its own window into an unbounded hold.
/// That is the single rule that keeps exit (1) reachable.
pub fn note_held(id: &str, against: &str) {
    let id = id.trim();
    if id.is_empty() {
        return;
    }
    let Ok(mut guard) = ledger().lock() else {
        // Poisoned lock degrades to "no skip" — the pre-fix behaviour.
        return;
    };
    if guard.len() >= REANCHOR_BACKOFF_CAP && !guard.contains_key(id) {
        // FAIL-OPEN. Clearing returns every candidate to probe-every-sweep; it
        // can never strand one.
        tracing::warn!(
            cap = REANCHOR_BACKOFF_CAP,
            "reanchor_backoff: ledger hit its cap — clearing (fail-open; every candidate \
             returns to probe-every-sweep until the ledger refills)"
        );
        guard.clear();
    }
    guard.insert(
        id.to_string(),
        HeldEntry {
            at: Instant::now(),
            against: against.trim().to_string(),
        },
    );
}

/// The row MOVED — an author path stamped an anchor or landed a chain — so the
/// cached `Held` verdict is now false. Clear it: the very next sweep re-probes.
///
/// Called from the same site as `heal_backoff::note_resolved`, whose comment
/// already says "the conductor now has a local chain for this id".
pub fn note_progress(id: &str) {
    let id = id.trim();
    if id.is_empty() {
        return;
    }
    if let Ok(mut guard) = ledger().lock() {
        guard.remove(id);
    }
}

/// May the sweep skip `id` without paying for its conductor probes?
///
/// True only when the pre-flight held this candidate inside `window` AND the
/// advertised head is the SAME one it was held against — a peer advertising
/// something new is evidence the cached verdict cannot speak to.
///
/// `window` is passed rather than read from config so the OFF behaviour is
/// provable without touching a process-wide `OnceLock` — the parallel-test flake
/// this crate has already paid for once (see `heal_backoff::should_replay`).
pub fn should_skip(id: &str, advertised_head: &str, window: Duration) -> bool {
    if window.is_zero() {
        return false;
    }
    let id = id.trim();
    let advertised_head = advertised_head.trim();
    let Ok(guard) = ledger().lock() else {
        return false;
    };
    guard
        .get(id)
        .is_some_and(|e| e.at.elapsed() < window && e.against == advertised_head)
}

/// Candidates currently holding an entry (expired-but-unobserved included).
/// Observability and test assertion only.
pub fn tracked() -> usize {
    ledger().lock().map(|g| g.len()).unwrap_or(0)
}

/// Drop every entry. Test-support only — the production exits are expiry,
/// [`note_progress`], a changed precondition, and the fail-open cap clear.
#[cfg(test)]
pub(crate) fn reset_for_test() {
    if let Ok(mut guard) = ledger().lock() {
        guard.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serializes tests that touch the ONE process-global ledger — the same
    /// discipline `heal_backoff::tests::LEDGER_TEST_LOCK` uses, for the same
    /// parallel-test flake reason. Recovers from poisoning so one failing test
    /// reports its own failure instead of cascading into its siblings.
    static LEDGER_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn exclusive() -> std::sync::MutexGuard<'static, ()> {
        LEDGER_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// THE DEFECT, as one assertion: a candidate the adopt pre-flight HELD must
    /// not re-buy its conductor probes on the next sweep. Before this module,
    /// 29-43 dead candidates each paid an Interactive `resolve_content_head`
    /// plus an election resolve every sweep, forever.
    #[test]
    fn a_held_candidate_is_skipped_within_its_window() {
        let _g = exclusive();
        reset_for_test();
        let window = Duration::from_secs(900);
        let id = "reanchor-backoff:held";
        let against = "uhCkk-peer-head";

        assert!(
            !should_skip(id, against, window),
            "a candidate never held must be probed, not skipped"
        );
        note_held(id, against);
        assert!(
            should_skip(id, against, window),
            "the same predictable Held verdict must not be re-purchased inside the window"
        );
    }

    /// C3: a deferral, never an exclusion. An elapsed window re-admits with no
    /// intervention; a zero window is the OFF switch and restores the pre-fix
    /// loop byte-for-byte.
    #[test]
    fn a_held_backoff_always_expires_and_zero_disables_it() {
        let _g = exclusive();
        reset_for_test();
        let id = "reanchor-backoff:expires";
        note_held(id, "h");
        // A real elapse, not a mocked clock: sleep past a 1ms window so the
        // assertion exercises the same `Instant::elapsed` comparison the sweep
        // does. Milliseconds keep it instant while the production window is
        // minutes.
        std::thread::sleep(Duration::from_millis(5));
        assert!(
            !should_skip(id, "h", Duration::from_millis(1)),
            "an entry older than its window must be re-probed, never held"
        );
        note_held(id, "h");
        assert!(
            !should_skip(id, "h", Duration::ZERO),
            "window 0 is the OFF switch — it must probe every candidate every sweep"
        );
    }

    /// The IMMEDIATE exit the reconciliation-controller discipline requires: the
    /// hold was against ONE peer-advertised head. A peer now advertising a
    /// different head is a changed precondition, so the candidate is eligible on
    /// the very next sweep rather than at window expiry.
    #[test]
    fn a_changed_precondition_re_admits_immediately() {
        let _g = exclusive();
        reset_for_test();
        let window = Duration::from_secs(900);
        let id = "reanchor-backoff:moved";
        note_held(id, "uhCkk-old");
        assert!(should_skip(id, "uhCkk-old", window));
        assert!(
            !should_skip(id, "uhCkk-new", window),
            "a peer advertising a different head is new evidence — probe it now"
        );
        assert!(
            !should_skip(id, "", window),
            "losing the advertiser is also a change; an unhinted sweep must not inherit the hold"
        );
    }

    /// Exit (2): an author path landed a chain / an anchor was stamped, so the
    /// cached Held verdict is false. Cleared at once, not at window expiry.
    #[test]
    fn progress_clears_the_hold_at_once() {
        let _g = exclusive();
        reset_for_test();
        let window = Duration::from_secs(900);
        let id = "reanchor-backoff:progressed";
        note_held(id, "h");
        assert!(should_skip(id, "h", window));
        note_progress(id);
        assert!(
            !should_skip(id, "h", window),
            "an author path stamped the row — the cached Held is now false and must not \
             survive the window"
        );
    }

    /// Whitespace must not mint two entries for one id: the sweep and the
    /// progress hook reach this module from different call sites, and a trailing
    /// space on one of them would silently strand the skip.
    #[test]
    fn ids_and_heads_are_matched_trimmed() {
        let _g = exclusive();
        reset_for_test();
        let window = Duration::from_secs(900);
        note_held("  reanchor-backoff:trimmed  ", " uhCkk-h ");
        assert!(should_skip("reanchor-backoff:trimmed", "uhCkk-h", window));
        note_progress("reanchor-backoff:trimmed\n");
        assert!(!should_skip(
            "  reanchor-backoff:trimmed",
            "uhCkk-h",
            window
        ));
    }

    /// An empty id is not an id. Recording one would let a single malformed row
    /// occupy a ledger slot forever.
    #[test]
    fn an_empty_id_is_never_recorded() {
        let _g = exclusive();
        reset_for_test();
        let before = tracked();
        note_held("   ", "h");
        assert_eq!(tracked(), before, "a blank id must not enter the ledger");
        assert!(!should_skip("   ", "h", Duration::from_secs(900)));
    }

    /// Memory is bounded and the bound fails OPEN — worst case is the pre-fix
    /// behaviour (probe everything), never a silent permanent hold.
    #[test]
    fn the_ledger_is_bounded_and_fails_open() {
        let _g = exclusive();
        reset_for_test();
        // bounded-work: exactly REANCHOR_BACKOFF_CAP iterations — the fill loop
        // that proves the cap, bounded by the constant it is asserting.
        for n in 0..REANCHOR_BACKOFF_CAP {
            note_held(&format!("cap:{n}"), "h");
        }
        assert_eq!(tracked(), REANCHOR_BACKOFF_CAP);
        note_held("cap:overflow", "h");
        assert!(
            tracked() <= REANCHOR_BACKOFF_CAP,
            "the ledger must never grow past its cap"
        );
        assert!(
            !should_skip("cap:0", "h", Duration::from_secs(900)),
            "overflow must RELEASE candidates (fail-open), never strand them"
        );
        reset_for_test();
    }
}
