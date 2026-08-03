//! CONTEST BACKOFF — hold back contest attempts that are PREDICTABLE repeat
//! failures, so the adopt sweep's fixed budget reaches productive candidates.
//!
//! This is the F-B throughput lever's first half (the second is the bounded
//! fan-out in [`crate::p2p::projection_reconcile::adopt_deferred_heads`]).
//! Design intent: `genesis/data/timeline/backlog/content-gap-limit-cycle-blocks-convergence.md`
//! §"RCA v3 → Open residuals → F-B".
//!
//! ## The waste this closes
//!
//! `adopt_deferred_heads` spends a fixed budget per sweep (200 candidates,
//! 120s wall clock, ~300s cadence). Two contest outcomes are *predictable
//! failures* — asking again next sweep cannot succeed until something else
//! changes — and they were consuming that budget alongside genuinely
//! contestable ids:
//!
//! 1. **`no_local_chain`** — `declare_canonical_head_inner`'s FIRST gate is
//!    `gather_content_chain(id, GetStrategy::Network)?.is_none()`
//!    (`content_store/src/lib.rs:3608`). That gate takes only `id`: it is
//!    **target-independent**, so no candidate shape, no other peer, and no
//!    carried record can pass it. Until this conductor holds a chain for the
//!    id, every contest for it fails identically — at ~0.4s each (the `get`
//!    cascade short-circuits at authority on a full-arc fleet, so the failure
//!    is fast *and* terminal). This is why the ledger keys on **`id` alone**,
//!    not on `(id, target)` like the self-candidacy ledger.
//! 2. **`self_candidacy_failed`** — the fallback arm nominated THIS row's own
//!    declared head and the conductor refused that too. The chain exists (that
//!    gate already passed), so the refusal is about the row's own head being
//!    unresolvable here — which likewise does not change between sweeps.
//! 3. **`evidence_absent`** (2026-08-03) — a `no_local_chain` refusal where the
//!    advertising peer's responder STATED that its own conductor holds no record
//!    for the head it advertises. This is the same gate as (1), but its exit
//!    condition is on a slower clock: (1) waits for THIS conductor to acquire a
//!    chain, while this waits for the bytes to come into existence anywhere on
//!    the fleet. Live evidence (2026-08-03): ~61% of the refusal population were
//!    `e2e-*` phantom ids whose bytes exist nowhere, re-asked every hour
//!    forever. It therefore serves [`BackoffWindows::evidence_absent`] (default
//!    24h) rather than the ordinary window — a longer DEFERRAL, never a
//!    different kind of thing. See
//!    `genesis/data/timeline/backlog/adopt-before-author-evidence-starvation.md`.
//!
//! ## C3: a backoff is never a permanent exclusion
//!
//! Every backed-off id has **two** automated exits, and neither needs a human:
//!
//! - **Time expiry** — [`backoff_is_active`] is false once the class's window
//!   has elapsed. The windows are config knobs
//!   ([`crate::config::contest_backoff_window`], default 3600s = 12 sweeps at
//!   the 300s cadence, deliberately the same ~1h dormancy as
//!   `MISS_READMIT_SWEEPS`; and
//!   [`crate::config::evidence_absent_backoff_window`], default 86400s).
//!   `window == 0` disables that class entirely, which restores the prior
//!   behaviour exactly. **This applies to the 24h class too**: an id whose bytes
//!   appear on the fleet a day later is re-admitted with no intervention and no
//!   human — the long window buys sweep budget, it never writes anything off.
//! - **Local chain arrival** — [`note_local_chain_arrived`] clears the entry the
//!   moment an author path gives this conductor a chain for the id. This is
//!   *load-bearing*, not an optimisation: `adopt_deferred_heads` runs BEFORE
//!   both witness sweeps in the same tick, so an id can fail `no_local_chain`
//!   at 12:00:01 and be authored by `witness_ghost_anchors` at 12:00:30. Without
//!   this hook the now-contestable id would sit backed off for the rest of the
//!   window.
//!
//! Overflow is fail-OPEN for the same reason: past [`CONTEST_BACKOFF_CAP`] the
//! ledger clears rather than evicting selectively, so the worst case is the
//! pre-backoff behaviour (contest every sweep), never a silent permanent hold.
//! This mirrors `head_adoption::SELF_CANDIDATE_LEDGER_CAP`.
//!
//! ## Process-local on purpose
//!
//! Like the self-candidacy ledger this is a de-duplication cache, not a truth
//! store. Losing it on restart costs at most one wasted contest attempt per id —
//! and a restart is itself an event that may have changed what the conductor
//! holds, so re-attempting is arguably right.
//!
//! ## What it deliberately does NOT gate
//!
//! Only the CONTEST arm. `try_obey_visible_election` — the arm that closes the
//! conductor-missing class once a chain-holding peer supplies an election — runs
//! BEFORE the decision rule and is never skipped by this ledger. A backed-off id
//! is DE-PRIORITISED in the sweep (see `adopt_deferred_heads`), not dropped from
//! it, precisely so its obey probe keeps running.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::metrics::ContestSkip;

/// Hard cap on the backoff ledger. The contested set is ~11.4k ids fleet-wide;
/// this leaves generous headroom while making unbounded growth impossible on a
/// pathological corpus. On overflow the ledger CLEARS (fail-open) rather than
/// evicting selectively — see the module note.
///
// bounded-work: memory budget = CONTEST_BACKOFF_CAP entries (fail-open clear on
// overflow); time budget = `crate::config::contest_backoff_window()` per entry,
// enforced by `backoff_is_active` and proven finite by `a_backoff_always_expires`.
// This module adds NO loop and NO retry ladder: it is a pure skip decision plus a
// bounded map. The paced-drain kernel it serves (`adopt_deferred_heads`:
// 200/tick, 25ms item delay, 120s wall clock) is unchanged and stays
// authoritative — this only re-ORDERS what that budget is spent on.
pub const CONTEST_BACKOFF_CAP: usize = 50_000;

/// The window each backoff class serves.
///
/// **Concerns:** C3 (every field is a finite duration, so every class expires);
/// C6a (the whole per-class budget lives here, in one readable struct, rather
/// than as a widening parameter list at each call site).
///
/// **Contract test:** [`tests::each_class_expires_on_its_own_window`].
///
/// Two classes, two clocks, and the distinction is the point:
///
/// - [`contest`] waits for THIS conductor's holdings to change (a chain arrives
///   via an author path). Sweep-scale — an hour is a fair bet.
/// - [`evidence_absent`] waits for BYTES TO EXIST anywhere on the fleet, because
///   the only peer advertising the head has stated its conductor holds no
///   record for it. Human-scale — an unwitnessed story getting witnessed, not a
///   sweep landing.
///
/// A single window cannot serve both without being wrong for one of them, which
/// is why this is a struct and not a `Duration`.
///
/// [`contest`]: Self::contest
/// [`evidence_absent`]: Self::evidence_absent
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackoffWindows {
    /// Window for every predictable-failure class except evidence-absent.
    pub contest: Duration,
    /// Window for [`ContestSkip::EvidenceAbsentBackoff`].
    pub evidence_absent: Duration,
}

impl BackoffWindows {
    /// Read both windows from the process-wide config mirrors.
    ///
    /// A constructor rather than a config read inside [`skip_class`]: the
    /// decision functions stay pure and window-parameterised (testable without a
    /// `OnceLock`), and the config coupling lives at exactly one line.
    pub fn from_config() -> Self {
        Self {
            contest: crate::config::contest_backoff_window(),
            evidence_absent: crate::config::evidence_absent_backoff_window(),
        }
    }

    /// One window for every class — the shape the module had before the
    /// evidence-absent split, kept for tests and for any caller that genuinely
    /// has one horizon.
    pub fn uniform(window: Duration) -> Self {
        Self {
            contest: window,
            evidence_absent: window,
        }
    }

    /// The window `class` serves.
    ///
    /// Pure and total. A ZERO `evidence_absent` window means the class is
    /// DISABLED at the recording site (`head_adoption::evidence_backoff_class`
    /// records the ordinary class instead), so an entry only reaches this arm
    /// with a zero window if the env flipped mid-process — in which case
    /// [`backoff_is_active`] releases it, which is the fail-open direction.
    pub fn for_class(&self, class: ContestSkip) -> Duration {
        match class {
            ContestSkip::EvidenceAbsentBackoff => self.evidence_absent,
            ContestSkip::NoLocalChainBackoff | ContestSkip::SelfCandidacyBackoff => self.contest,
        }
    }
}

/// Is a backoff recorded `elapsed` ago still holding, given `window`?
///
/// **Concerns:** C3 (liveness — for any finite `window` there exists an
/// `elapsed` for which this is false, so a backed-off id ALWAYS becomes
/// eligible again without intervention); C6a (bounded work — this predicate is
/// the whole budget of the skip decision).
///
/// **Contract tests:** [`tests::a_backoff_always_expires`],
/// [`tests::a_zero_window_disables_the_backoff`],
/// [`crate::liveness_contract::tests::the_contest_backoff_is_never_a_permanent_exclusion`].
///
/// Pure and total. Takes `elapsed` rather than two [`Instant`]s so the C3
/// property is expressible without constructing a past instant (which is
/// fallible on some platforms) — the ledger does the clock reading.
///
/// `window == 0` means DISABLED: never active, so the contest arm behaves
/// exactly as it did before this module existed.
pub fn backoff_is_active(elapsed: Duration, window: Duration) -> bool {
    window > Duration::ZERO && elapsed < window
}

/// One backed-off id: when it was recorded, and which predictable failure put
/// it there (the label the skip counter reports).
#[derive(Debug, Clone, Copy)]
struct BackoffEntry {
    recorded_at: Instant,
    class: ContestSkip,
}

fn ledger() -> &'static Mutex<HashMap<String, BackoffEntry>> {
    static LEDGER: OnceLock<Mutex<HashMap<String, BackoffEntry>>> = OnceLock::new();
    LEDGER.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Record that contesting `id` failed predictably, so the next sweeps may skip
/// it. Re-recording an already-backed-off id RESTARTS its window — the failure
/// just recurred, so the evidence is fresh.
///
/// A poisoned lock is ignored (the backoff is an optimisation; losing it costs
/// throughput, never correctness).
pub fn note(id: &str, class: ContestSkip) {
    let Ok(mut guard) = ledger().lock() else {
        // Frequency check: this fires once per poisoned lock (a prior panic
        // while holding it), not per call — safe at `warn!`.
        tracing::warn!("contest_backoff: ledger lock poisoned; skipping this note (backoff is an optimisation, not correctness)");
        return;
    };
    if guard.len() >= CONTEST_BACKOFF_CAP && !guard.contains_key(id) {
        // FAIL-OPEN. Clearing returns every id to the pre-backoff behaviour
        // (contested every sweep); it can never strand one.
        tracing::warn!(
            cap = CONTEST_BACKOFF_CAP,
            "contest_backoff: ledger hit its cap — clearing (fail-open; every id returns to \
             contested-every-sweep until the ledger refills)"
        );
        crate::metrics::inc_contest_backoff_cleared();
        guard.clear();
    }
    guard.insert(
        id.to_string(),
        BackoffEntry {
            recorded_at: Instant::now(),
            class,
        },
    );
}

/// Is `id` currently backed off? `Some(class)` when it is — the class is the
/// label the caller reports on `elohim_content_contest_skipped_total`.
///
/// Expired entries are dropped as they are observed, so the ledger sheds memory
/// on the same reads that use it and no separate sweeper is needed.
///
/// The window is resolved PER CLASS ([`BackoffWindows::for_class`]): an id held
/// because the fleet has no bytes for it waits longer than one held because this
/// conductor has no chain for it. A zero window for the recorded class means
/// "not held" — the OFF switch, per class.
pub fn skip_class(id: &str, windows: BackoffWindows) -> Option<ContestSkip> {
    let mut guard = ledger().lock().ok()?;
    let entry = *guard.get(id)?;
    if backoff_is_active(entry.recorded_at.elapsed(), windows.for_class(entry.class)) {
        Some(entry.class)
    } else {
        guard.remove(id);
        None
    }
}

/// This conductor now HOLDS a local chain for `id` (an author path just landed),
/// so the `no_local_chain` verdict that backed it off is stale. Clear it: the
/// very next sweep may contest successfully.
///
/// Called from every site whose own comment already says "the conductor now has
/// a local chain for this id" — see the module note on why this is load-bearing
/// rather than an optimisation.
pub fn note_local_chain_arrived(id: &str) {
    if let Ok(mut guard) = ledger().lock() {
        guard.remove(id);
    }
}

/// Entries currently held (expired-but-unobserved included). Observability and
/// test assertion only.
pub fn tracked() -> usize {
    ledger().lock().map(|g| g.len()).unwrap_or(0)
}

/// Drop every entry. Test isolation only — the process-local ledger is shared
/// across `#[test]` fns in one binary.
///
/// MUST be called while holding [`test_exclusive`]: `cargo test` runs tests in
/// parallel threads of ONE process, so a bare `reset()` in one test wipes
/// another's entries mid-assertion. Tests that do not need whole-ledger state
/// should use unique ids and skip both.
#[cfg(test)]
pub(crate) fn reset() {
    if let Ok(mut guard) = ledger().lock() {
        guard.clear();
    }
}

/// Serialises the tests that need the WHOLE ledger to themselves (`reset` /
/// [`tracked`] assertions) against each other.
///
/// The alternative — every test resetting and hoping — is the parallel-test
/// flake this codebase has already paid for once (`feedback_env_var_test_flakiness`:
/// a hidden shared global read as green until scheduling changed). Recovers from
/// poisoning so one failing test reports its own failure instead of cascading.
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

    const WINDOW: Duration = Duration::from_secs(3600);
    const WINDOWS: BackoffWindows = BackoffWindows {
        contest: WINDOW,
        evidence_absent: WINDOW,
    };

    /// C3, as a property rather than an example: whatever finite window is
    /// configured, there is an elapsed time past which the id is eligible again.
    /// This is the assertion that makes the backoff a DELAY and not an
    /// exclusion, and it is the one a future edit must keep true.
    #[test]
    fn a_backoff_always_expires() {
        for secs in [1u64, 60, 300, 3600, 86_400] {
            let window = Duration::from_secs(secs);
            assert!(
                backoff_is_active(Duration::ZERO, window),
                "a just-recorded backoff must hold ({secs}s window)"
            );
            assert!(
                !backoff_is_active(window, window),
                "at exactly the window the backoff must have expired ({secs}s)"
            );
            assert!(
                !backoff_is_active(window + Duration::from_secs(1), window),
                "past the window the backoff must have expired ({secs}s)"
            );
        }
    }

    /// The OFF switch is a real off switch: window 0 restores the pre-F-B
    /// behaviour (every candidate contested every sweep) with no other change.
    ///
    /// Unique ids, no `reset()` — see [`test_exclusive`] on why a bare reset in
    /// a parallel test run is a flake and not isolation.
    #[test]
    fn a_zero_window_disables_the_backoff() {
        let _exclusive = test_exclusive();
        assert!(!backoff_is_active(Duration::ZERO, Duration::ZERO));
        note("zero-window:id", ContestSkip::NoLocalChainBackoff);
        assert_eq!(
            skip_class("zero-window:id", BackoffWindows::uniform(Duration::ZERO)),
            None,
            "with the backoff disabled nothing may be skipped, even a recorded id"
        );
    }

    /// The ledger holds a recorded id and reports the class that put it there —
    /// the two failure shapes must stay distinguishable on the dashboard.
    #[test]
    fn a_recorded_id_is_skipped_with_its_own_class() {
        let _exclusive = test_exclusive();
        note("class:no-chain", ContestSkip::NoLocalChainBackoff);
        note("class:self-cand", ContestSkip::SelfCandidacyBackoff);
        assert_eq!(
            skip_class("class:no-chain", WINDOWS),
            Some(ContestSkip::NoLocalChainBackoff)
        );
        assert_eq!(
            skip_class("class:self-cand", WINDOWS),
            Some(ContestSkip::SelfCandidacyBackoff)
        );
        assert_eq!(
            skip_class("class:never-recorded", WINDOWS),
            None,
            "an id that never failed must never be skipped"
        );
    }

    /// The SECOND automated exit, and the load-bearing one within a single
    /// sweep cycle: the witness sweeps author a chain seconds after the adopt
    /// sweep gave up on it, and the id must be contestable immediately rather
    /// than at the end of the window.
    #[test]
    fn a_local_chain_arrival_clears_the_backoff_immediately() {
        let _exclusive = test_exclusive();
        note("arrival:authored", ContestSkip::NoLocalChainBackoff);
        assert!(skip_class("arrival:authored", WINDOWS).is_some());
        note_local_chain_arrived("arrival:authored");
        assert_eq!(
            skip_class("arrival:authored", WINDOWS),
            None,
            "an authored id is contestable on the very next sweep, not in an hour"
        );
    }

    /// Memory is bounded, and the bound fails OPEN. Past the cap the ledger
    /// clears: worst case every id is contested again (the pre-backoff cost),
    /// which is exactly the direction a bound on a liveness-sensitive cache must
    /// fail in.
    ///
    /// Whole-ledger assertions ⇒ takes [`test_exclusive`].
    #[test]
    fn the_ledger_is_bounded_and_fails_open() {
        let _exclusive = test_exclusive();
        reset();
        // bounded-work: exactly CONTEST_BACKOFF_CAP iterations — the fill loop
        // that proves the cap, bounded by the constant it is asserting.
        for n in 0..CONTEST_BACKOFF_CAP {
            note(&format!("cap:{n}"), ContestSkip::NoLocalChainBackoff);
        }
        assert_eq!(tracked(), CONTEST_BACKOFF_CAP);
        note("cap:overflow", ContestSkip::NoLocalChainBackoff);
        assert!(
            tracked() <= CONTEST_BACKOFF_CAP,
            "the ledger must never grow past its cap"
        );
        assert_eq!(
            skip_class("cap:0", WINDOWS),
            None,
            "overflow must RELEASE ids (fail-open), never strand them"
        );
        reset();
    }

    /// PER-CLASS CLOCKS. The evidence-absent class must serve its OWN window,
    /// not the contest one — that is the entire point of the split, and a
    /// `for_class` that quietly returned `contest` for everything would make the
    /// 24h lever a silent no-op while every test above still passed.
    #[test]
    fn each_class_expires_on_its_own_window() {
        let windows = BackoffWindows {
            contest: Duration::from_secs(3600),
            evidence_absent: Duration::from_secs(86_400),
        };
        assert_eq!(
            windows.for_class(ContestSkip::NoLocalChainBackoff),
            Duration::from_secs(3600)
        );
        assert_eq!(
            windows.for_class(ContestSkip::SelfCandidacyBackoff),
            Duration::from_secs(3600)
        );
        assert_eq!(
            windows.for_class(ContestSkip::EvidenceAbsentBackoff),
            Duration::from_secs(86_400),
            "the evidence-absent class must wait on its own, slower clock"
        );

        // At 2h the ordinary class has expired and the evidence-absent one has
        // not — the observable consequence of the two clocks.
        let two_hours = Duration::from_secs(7200);
        assert!(!backoff_is_active(
            two_hours,
            windows.for_class(ContestSkip::NoLocalChainBackoff)
        ));
        assert!(backoff_is_active(
            two_hours,
            windows.for_class(ContestSkip::EvidenceAbsentBackoff)
        ));
    }

    /// C3 FOR THE LONG CLASS, as a property. 24h is long enough that "is this
    /// still a deferral or has it become an exclusion?" is a fair question to
    /// ask of it — so it gets the same expiry proof the 1h class has, plus the
    /// re-admission assertion through the real ledger.
    #[test]
    fn the_evidence_absent_class_is_a_deferral_not_an_exclusion() {
        let _exclusive = test_exclusive();
        for secs in [1u64, 3600, 86_400, 604_800] {
            let windows = BackoffWindows {
                contest: WINDOW,
                evidence_absent: Duration::from_secs(secs),
            };
            let w = windows.for_class(ContestSkip::EvidenceAbsentBackoff);
            assert!(backoff_is_active(Duration::ZERO, w));
            assert!(
                !backoff_is_active(w, w),
                "at exactly the {secs}s window the evidence-absent hold must have expired"
            );
        }

        // Through the real ledger: recorded → held; the same id read against an
        // already-expired window → RE-ADMITTED (and the entry shed).
        note("evidence:absent-id", ContestSkip::EvidenceAbsentBackoff);
        assert_eq!(
            skip_class(
                "evidence:absent-id",
                BackoffWindows {
                    contest: WINDOW,
                    evidence_absent: Duration::from_secs(86_400),
                }
            ),
            Some(ContestSkip::EvidenceAbsentBackoff)
        );
        // A window shorter than the entry's age (zero here) is the expiry the
        // sweep would see a day later: the id becomes contestable again with no
        // intervention, which is what makes bytes-appear-later self-healing.
        assert_eq!(
            skip_class(
                "evidence:absent-id",
                BackoffWindows {
                    contest: WINDOW,
                    evidence_absent: Duration::ZERO,
                }
            ),
            None,
            "an expired evidence-absent hold must release the id, never strand it"
        );
    }

    /// The ordinary classes must NOT inherit the long window — a shared clock
    /// would quietly 24× the no-chain backoff and starve the ids that are only
    /// waiting on an author path.
    #[test]
    fn a_long_evidence_window_does_not_lengthen_the_ordinary_classes() {
        let _exclusive = test_exclusive();
        note("mixed:no-chain", ContestSkip::NoLocalChainBackoff);
        assert_eq!(
            skip_class(
                "mixed:no-chain",
                BackoffWindows {
                    contest: Duration::ZERO,
                    evidence_absent: Duration::from_secs(86_400),
                }
            ),
            None,
            "a disabled contest window must release a no-chain entry regardless of how long \
             the evidence-absent window is"
        );
    }

    /// A recurrence restarts the window rather than letting a stale record age
    /// out while the failure is still happening every sweep.
    ///
    /// Asserts on [`tracked`] ⇒ takes [`test_exclusive`].
    #[test]
    fn re_recording_refreshes_the_window() {
        let _exclusive = test_exclusive();
        reset();
        note("recur:id", ContestSkip::NoLocalChainBackoff);
        note("recur:id", ContestSkip::SelfCandidacyBackoff);
        assert_eq!(
            skip_class("recur:id", WINDOWS),
            Some(ContestSkip::SelfCandidacyBackoff),
            "the most recent failure class is the one reported"
        );
        assert_eq!(tracked(), 1, "a recurrence must not add a second entry");
        reset();
    }
}
