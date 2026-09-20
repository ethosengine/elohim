//! REA SETTLED-VERDICT BACKOFF — replay an ADJUDICATED heal verdict instead of
//! re-deriving it from the own conductor, so the REA heal leg's conductor
//! budget reaches gaps whose answer is NOT already known.
//!
//! ## The waste this closes (2026-09-20 defect, household mesh at rest)
//!
//! On a SETTLED mesh — nobody authoring, nobody reading, every plane
//! `caughtUp` — ~80% of each storage node's conductor calls were one function,
//! `content_store::get_rea_commitment`, at 80-120 calls/min/node, and they rode
//! the INTERACTIVE admission lane (measured by
//! `elohim_conductor_calls_total{zome,fn,class}`; evidence:
//! `genesis/a2o/reports/recovery/serving-edge-20260919/household-at-rest-named.log`).
//!
//! The producer is [`crate::p2p::projection_reconcile`]'s REA heal leg. Every
//! sweep, discovery re-enqueues each commitment a peer still advertises as
//! divergent; heal re-reads it from the OWN conductor and reaches the SAME
//! verdict it reached last sweep:
//!
//! * `ReaHealWrite::Wrote(ReaAnchorWrite::NoAdvance)` — "the own conductor
//!   answered the version this row already holds"; and
//! * `ReaHealWrite::RefusedConductorBehind` — "the own conductor answered from
//!   BEHIND this row's standing".
//!
//! Both arms' own comments already said *re-reading this conductor cannot
//! resolve it* — and then re-read it 30 seconds later, forever, because the
//! tracker that recorded the adjudication is rebuilt per sweep. This module is
//! the missing memory: the verdict is remembered against the EVIDENCE that
//! produced it.
//!
//! ## The key, and why it has four parts
//!
//! `(commitment_id, evidence)` where `evidence` is
//! `advertised_anchor | advertised_state | local_anchor | local_state`:
//!
//! * the two ADVERTISED halves are exactly what discovery learned from the peer
//!   that made this id divergent (both compared axes — a graduation that leaves
//!   the anchor untouched is still new evidence), so ANY change in what peers
//!   advertise re-admits the id;
//! * the two LOCAL halves are the row's own standing at verdict time, so a local
//!   write (HTTP update, the commitment-signal worker, a heal that advanced)
//!   also re-admits it.
//!
//! Both halves are already in hand at discovery time — the advertised pair from
//! the inventory responses, the local pair from the local-inventory query — so
//! the key costs no extra I/O of any kind.
//!
//! ## What a replay is, and what it is emphatically NOT
//!
//! A replay reproduces the adjudicated arm's effect EXACTLY: the id is
//! `mark_completed` on the per-sweep tracker and contributes to
//! `divergent_refused` precisely as it does when the call is made, so
//! `caughtUp`, `converged`, `/p2p/status` and every reconcile gauge read
//! IDENTICALLY to the pre-fix leg. Only the conductor round-trip is elided.
//!
//! Against the substrate trust contract
//! (`genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md`):
//! this arm only READS the own conductor, and a skipped read adds no truth and
//! removes none.
//!
//! * **heal fills, never moves** — a replay writes nothing at all, so it cannot
//!   move a declared head. The two verdicts it replays are the two arms that
//!   already refused to write.
//! * **canonical channels alone move declared heads** — unchanged: the
//!   canonical channels never consulted this ledger and are not gated by it.
//! * **verify-locally-then-serve** — unchanged: nothing served changes. The
//!   projection row is byte-identical either way, because the call that is
//!   skipped is one whose only possible outcome was "leave the row alone".
//!
//! It is therefore **not** an exclusion (the id stays a gap, stays divergent,
//! stays counted), **not** a terminality claim (nothing is exhausted or written
//! off — that is the [`crate::p2p::projection_reconcile::MissLedger`]'s job and
//! it is untouched here), and **not** a new truth (it replays THIS node's own
//! last adjudication, which was never authoritative about anything but this
//! node's conductor at that moment).
//!
//! ## Four automated exits, none of them human
//!
//! 1. **Changed peer evidence** — a peer now advertising a different anchor or a
//!    different lifecycle state re-admits the id on the very NEXT sweep, not at
//!    window expiry. The DHT is the manifest; new evidence must reach the
//!    controller immediately.
//! 2. **A changed local row** — the local anchor/state halves of the key, same
//!    immediacy.
//! 3. **Window expiry** — [`crate::config::heal_missing_backoff_window`]
//!    (`HEAL_MISSING_BACKOFF_SECONDS`, default 600s). Shared DELIBERATELY with
//!    [`crate::services::heal_backoff`] rather than minting a second knob: both
//!    are "how stale may a replayed own-conductor answer be on the heal leg",
//!    one setting names that policy once, and `0` is then a single honest OFF
//!    switch that restores the pre-fix legs on BOTH arms byte-for-byte.
//! 4. **Progress** — [`note_progress`] clears the entry the moment a heal
//!    actually advances the row.
//!
//! Process restart is a fifth exit by construction: this is in-memory only.
//!
//! ## Process-local, and deliberately NOT persisted
//!
//! Like [`crate::services::heal_backoff`] and
//! [`crate::services::reanchor_backoff`], and unlike
//! [`crate::services::contest_backoff`], this holds at most a few sweeps of
//! knowledge. Losing it on restart costs one sweep of round-trips — and a
//! restart plausibly changed what the conductor holds, so re-asking is arguably
//! correct. No file, no diesel table, no migration timestamp spent.
//!
//! Overflow is fail-OPEN: past [`REA_VERDICT_LEDGER_CAP`] the ledger clears
//! rather than evicting selectively, so the worst case is the pre-fix behaviour
//! (re-read every divergent id every sweep), never a silent permanent hold.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::metrics::ReaHealSkip;

/// Hard cap on the ledger. Mirrors [`crate::services::heal_backoff::HEAL_BACKOFF_CAP`]
/// and [`crate::services::reanchor_backoff::REANCHOR_BACKOFF_CAP`]: the live REA
/// corpus is a few thousand rows per pod, so 50k leaves generous headroom while
/// making unbounded growth impossible on a pathological corpus.
///
// bounded-work: memory budget = REA_VERDICT_LEDGER_CAP entries (fail-open clear
// on overflow); time budget = `crate::config::heal_missing_backoff_window()` per
// entry, enforced by `remembered` and proven finite by
// `a_remembered_verdict_always_expires_and_zero_disables_it`. This module adds
// NO loop, NO retry ladder, and NO I/O: it is a pure replay decision plus a
// bounded map. The paced leg it serves (`heal_rea` under
// `HealPacing::rea_leg_budget`) is unchanged and stays authoritative — this only
// ELIDES round-trips whose answer is known.
pub const REA_VERDICT_LEDGER_CAP: usize = 50_000;

/// One adjudicated verdict: when the heal leg reached it, what it was reached
/// against, and which refusal it was.
#[derive(Debug, Clone)]
struct SettledEntry {
    at: Instant,
    /// The composed evidence this verdict was reached against — the advertised
    /// and local halves of the key. Exits (1) and (2) key on this.
    against: String,
    verdict: ReaHealSkip,
}

fn ledger() -> &'static Mutex<HashMap<String, SettledEntry>> {
    static LEDGER: OnceLock<Mutex<HashMap<String, SettledEntry>>> = OnceLock::new();
    LEDGER.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Record that the heal leg ADJUDICATED `id` as `verdict` against `evidence`.
///
/// Returns TRUE when this is a REPEAT — the same verdict against the same
/// evidence was already standing (the window had simply lapsed, so the leg
/// re-read and learned the same thing). The heal leg uses that to demote its
/// per-id log line to `debug!`, which is what stops the identical WARN/INFO
/// firing 124× per 10 minutes and hiding the treadmill it was describing.
///
/// Called from the REAL adjudication arms only — never from the replay path, so
/// a replay can never extend its own window into an unbounded hold. That is the
/// single rule that keeps exit (3) reachable.
pub fn note_settled(id: &str, evidence: &str, verdict: ReaHealSkip) -> bool {
    let id = id.trim();
    if id.is_empty() {
        return false;
    }
    let evidence = evidence.trim();
    let Ok(mut guard) = ledger().lock() else {
        // Poisoned lock degrades to "no replay" — the pre-fix behaviour.
        return false;
    };
    if guard.len() >= REA_VERDICT_LEDGER_CAP && !guard.contains_key(id) {
        // FAIL-OPEN. Clearing returns every id to read-every-sweep; it can never
        // strand one.
        tracing::warn!(
            cap = REA_VERDICT_LEDGER_CAP,
            "rea_verdict_backoff: ledger hit its cap — clearing (fail-open; every commitment \
             returns to read-every-sweep until the ledger refills)"
        );
        guard.clear();
    }
    let repeat = guard
        .get(id)
        .is_some_and(|e| e.against == evidence && e.verdict == verdict);
    guard.insert(
        id.to_string(),
        SettledEntry {
            at: Instant::now(),
            against: evidence.to_string(),
            verdict,
        },
    );
    repeat
}

/// The row MOVED — a heal applied verified newer authority — so the remembered
/// refusal is now false. Clear it: the very next sweep re-reads.
///
/// Strictly belt-and-braces (an advance changes the local half of the evidence,
/// which already re-admits the id), and cheap enough to keep the invariant
/// "a ledger entry never outlives the row state it describes" true by
/// construction rather than by inference.
pub fn note_progress(id: &str) {
    let id = id.trim();
    if id.is_empty() {
        return;
    }
    if let Ok(mut guard) = ledger().lock() {
        guard.remove(id);
    }
}

/// May the heal leg replay an adjudicated verdict for `id` instead of paying a
/// `get_rea_commitment` round-trip for it?
///
/// `Some(verdict)` only when the leg adjudicated this id inside `window` AND the
/// evidence is the SAME one it adjudicated against — a peer advertising
/// something new, or a local row that has moved, is evidence the remembered
/// verdict cannot speak to.
///
/// `window` is passed rather than read from config so the OFF behaviour is
/// provable without touching a process-wide `OnceLock` — the parallel-test flake
/// this crate has already paid for once (see `heal_backoff::should_replay`).
pub fn remembered(id: &str, evidence: &str, window: Duration) -> Option<ReaHealSkip> {
    if window.is_zero() {
        return None;
    }
    let id = id.trim();
    let evidence = evidence.trim();
    let guard = ledger().lock().ok()?;
    guard
        .get(id)
        .filter(|e| e.at.elapsed() < window && e.against == evidence)
        .map(|e| e.verdict)
}

/// Ids currently holding a verdict entry (expired-but-unobserved included).
/// Observability and test assertion only.
pub fn tracked() -> usize {
    ledger().lock().map(|g| g.len()).unwrap_or(0)
}

/// Drop every entry. Test-support only — the production exits are changed
/// evidence, expiry, [`note_progress`], the fail-open cap clear, and restart.
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

    const W: Duration = Duration::from_secs(600);

    /// THE DEFECT, as one assertion at the ledger seam: a verdict the heal leg
    /// already adjudicated must not be re-derived while the evidence that
    /// produced it is unchanged. Before this module, every divergent commitment
    /// re-bought the same `NoAdvance` from the own conductor every 30s, forever.
    #[test]
    fn a_verdict_is_not_re_derived_while_its_evidence_is_unchanged() {
        let _g = exclusive();
        reset_for_test();
        let id = "rea-verdict:settled";
        let ev = "anchorA|active|anchorB|active";

        assert!(
            remembered(id, ev, W).is_none(),
            "an id never adjudicated must be read, not replayed"
        );
        assert!(
            !note_settled(id, ev, ReaHealSkip::NoAdvance),
            "the FIRST adjudication is not a repeat"
        );
        assert_eq!(
            remembered(id, ev, W),
            Some(ReaHealSkip::NoAdvance),
            "the same unresolvable verdict must not be re-purchased inside the window"
        );
    }

    /// Exit (1). The verdict was reached against ONE peer advertisement. A peer
    /// now advertising a different anchor — or a different lifecycle state at the
    /// same anchor, which is the graduation case — is a changed precondition, so
    /// the id is eligible on the very next sweep rather than at window expiry.
    #[test]
    fn changed_peer_evidence_re_admits_the_id() {
        let _g = exclusive();
        reset_for_test();
        let id = "rea-verdict:peer-moved";
        note_settled(id, "anchorA|active|anchorA|active", ReaHealSkip::NoAdvance);
        assert!(remembered(id, "anchorA|active|anchorA|active", W).is_some());

        assert!(
            remembered(id, "anchorZ|active|anchorA|active", W).is_none(),
            "a peer advertising a different ANCHOR is new evidence — read it now"
        );
        assert!(
            remembered(id, "anchorA|settled|anchorA|active", W).is_none(),
            "a peer advertising a graduated STATE at the same anchor is new evidence too"
        );
        assert!(
            remembered(id, "||anchorA|active", W).is_none(),
            "losing the advertiser is also a change; an unadvertised sweep must not inherit \
             the verdict"
        );
    }

    /// Exit (2). The verdict described a local row too. An HTTP update, the
    /// commitment-signal worker, or any other writer moving that row makes the
    /// remembered refusal false at once.
    #[test]
    fn a_changed_local_row_re_admits_the_id() {
        let _g = exclusive();
        reset_for_test();
        let id = "rea-verdict:local-moved";
        note_settled(id, "anchorA|active|anchorA|active", ReaHealSkip::NoAdvance);
        assert!(remembered(id, "anchorA|active|anchorA|active", W).is_some());

        assert!(
            remembered(id, "anchorA|active|anchorA|cancelled", W).is_none(),
            "the LOCAL row's standing moved — the remembered refusal is about a row that no \
             longer exists in that shape"
        );
        assert!(
            remembered(id, "anchorA|active|anchorQ|active", W).is_none(),
            "the LOCAL anchor moved — same reasoning"
        );

        // And the explicit progress hook does the same, immediately.
        note_settled(id, "anchorA|active|anchorA|active", ReaHealSkip::NoAdvance);
        note_progress(id);
        assert!(
            remembered(id, "anchorA|active|anchorA|active", W).is_none(),
            "a heal advanced the row — the cached refusal must not survive the window"
        );
    }

    /// Exit (3): a deferral, never an exclusion. An elapsed window re-admits with
    /// no intervention; a zero window is the shared OFF switch and restores the
    /// pre-fix leg byte-for-byte.
    #[test]
    fn the_window_lapsing_re_admits_the_id() {
        let _g = exclusive();
        reset_for_test();
        let id = "rea-verdict:expires";
        let ev = "a|b|c|d";
        note_settled(id, ev, ReaHealSkip::ConductorBehind);
        // A real elapse, not a mocked clock: sleep past a 1ms window so the
        // assertion exercises the same `Instant::elapsed` comparison the leg
        // does. Milliseconds keep it instant while the production window is
        // minutes.
        std::thread::sleep(Duration::from_millis(5));
        assert!(
            remembered(id, ev, Duration::from_millis(1)).is_none(),
            "an entry older than its window must be re-read, never held"
        );

        note_settled(id, ev, ReaHealSkip::ConductorBehind);
        assert!(
            remembered(id, ev, Duration::ZERO).is_none(),
            "window 0 is the OFF switch — it must read every divergent id every sweep"
        );
    }

    /// The verdict is carried, not flattened: the replay must reproduce the SAME
    /// refusal the leg would have reached, because the skip counter and the
    /// heal-outcome bookkeeping are labelled by it.
    #[test]
    fn a_remembered_verdict_keeps_its_own_class() {
        let _g = exclusive();
        reset_for_test();
        let ev = "a|b|c|d";
        note_settled("rea-verdict:na", ev, ReaHealSkip::NoAdvance);
        note_settled("rea-verdict:cb", ev, ReaHealSkip::ConductorBehind);
        assert_eq!(
            remembered("rea-verdict:na", ev, W),
            Some(ReaHealSkip::NoAdvance)
        );
        assert_eq!(
            remembered("rea-verdict:cb", ev, W),
            Some(ReaHealSkip::ConductorBehind)
        );
    }

    /// The repeat bit is what demotes the per-id log line. A FIRST adjudication,
    /// and any adjudication whose evidence or verdict CHANGED, must stay loud.
    #[test]
    fn only_an_unchanged_re_adjudication_reads_as_a_repeat() {
        let _g = exclusive();
        reset_for_test();
        let id = "rea-verdict:repeat";
        let ev = "a|b|c|d";
        assert!(
            !note_settled(id, ev, ReaHealSkip::NoAdvance),
            "first = loud"
        );
        assert!(
            note_settled(id, ev, ReaHealSkip::NoAdvance),
            "same verdict, same evidence = a repeat, and must be demoted"
        );
        assert!(
            !note_settled(id, ev, ReaHealSkip::ConductorBehind),
            "the verdict CHANGED — a conductor that has fallen behind is news, not noise"
        );
        assert!(
            !note_settled(id, "a|b|c|moved", ReaHealSkip::ConductorBehind),
            "the evidence CHANGED — a fresh adjudication, not a repeat"
        );
    }

    /// Whitespace must not mint two entries for one id: the adjudication arms and
    /// the progress hook reach this module from different call sites, and a
    /// trailing space on one of them would silently strand the replay.
    #[test]
    fn ids_and_evidence_are_matched_trimmed() {
        let _g = exclusive();
        reset_for_test();
        note_settled(
            "  rea-verdict:trimmed  ",
            " a|b|c|d ",
            ReaHealSkip::NoAdvance,
        );
        assert!(remembered("rea-verdict:trimmed", "a|b|c|d", W).is_some());
        note_progress("rea-verdict:trimmed\n");
        assert!(remembered("  rea-verdict:trimmed", "a|b|c|d", W).is_none());
    }

    /// An empty id is not an id. Recording one would let a single malformed row
    /// occupy a ledger slot forever.
    #[test]
    fn an_empty_id_is_never_recorded() {
        let _g = exclusive();
        reset_for_test();
        let before = tracked();
        assert!(!note_settled("   ", "a|b|c|d", ReaHealSkip::NoAdvance));
        assert_eq!(tracked(), before, "a blank id must not enter the ledger");
        assert!(remembered("   ", "a|b|c|d", W).is_none());
    }

    /// Memory is bounded and the bound fails OPEN — worst case is the pre-fix
    /// behaviour (read everything every sweep), never a silent permanent hold.
    #[test]
    fn the_ledger_is_bounded_and_fails_open() {
        let _g = exclusive();
        reset_for_test();
        // bounded-work: exactly REA_VERDICT_LEDGER_CAP iterations — the fill loop
        // that proves the cap, bounded by the constant it is asserting.
        for n in 0..REA_VERDICT_LEDGER_CAP {
            note_settled(&format!("cap:{n}"), "a|b|c|d", ReaHealSkip::NoAdvance);
        }
        assert_eq!(tracked(), REA_VERDICT_LEDGER_CAP);
        note_settled("cap:overflow", "a|b|c|d", ReaHealSkip::NoAdvance);
        assert!(
            tracked() <= REA_VERDICT_LEDGER_CAP,
            "the ledger must never grow past its cap"
        );
        assert!(
            remembered("cap:0", "a|b|c|d", W).is_none(),
            "overflow must RELEASE ids (fail-open), never strand them"
        );
        reset_for_test();
    }
}
