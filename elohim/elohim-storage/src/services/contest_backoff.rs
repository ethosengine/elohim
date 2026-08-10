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
//! ## Process-local on purpose — with a snapshot, not a table
//!
//! Like the self-candidacy ledger this is a de-duplication cache, not a truth
//! store. Losing one entry on restart costs at most one wasted contest attempt,
//! and a restart is itself an event that may have changed what the conductor
//! holds, so re-attempting is arguably right.
//!
//! Losing the WHOLE ledger is a different quantity. Once the evidence-absent
//! class exists (24h windows, ~61% of the live refusal population), a deploy
//! restart re-churns the entire corpus from zero and spends hours re-learning
//! classifications it had already paid for. So the ledger snapshots to ONE file
//! under the storage dir ([`crate::config::Config::contest_backoff_snapshot_path`])
//! and reloads at boot — deliberately NOT a diesel table:
//!
//! - it is Category C operational state, reconstructable by re-learning;
//! - a migration timestamp is a scarce, collision-prone resource, and this state
//!   does not deserve one;
//! - every failure mode degrades to today's behaviour — an unreadable, corrupt,
//!   wrong-version, or absent snapshot starts the ledger EMPTY, and a failed
//!   save leaves the in-memory ledger authoritative.
//!
//! Expiry survives the round trip on absolute (same-node) epoch seconds, so a
//! long restart correctly releases everything that expired while the process was
//! down; the 50k cap applies on load, dropping the SOONEST-expiring entries
//! first (they are the ones re-learned most cheaply).
//!
//! ## What it deliberately does NOT gate
//!
//! Only the CONTEST arm. `try_obey_visible_election` — the arm that closes the
//! conductor-missing class once a chain-holding peer supplies an election — runs
//! BEFORE the decision rule and is never skipped by this ledger. A backed-off id
//! is DE-PRIORITISED in the sweep (see `adopt_deferred_heads`), not dropped from
//! it, precisely so its obey probe keeps running.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::metrics::{BackoffSnapshot, ContestSkip};

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
            ContestSkip::NoLocalChainBackoff
            | ContestSkip::SelfCandidacyBackoff
            | ContestSkip::DeclareErrorBackoff => self.contest,
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
    mark_dirty();
}

/// Pure dwell rule for the ghost-declaration-decay consumer: has an
/// evidence-absent verdict STOOD — still active AND at least `min_dwell` old?
///
/// The decay arm must not treat a single sweep's stated-no-record answer as a
/// fleet-wide fact: post-2026-08-10 the responder's `get_record_for_action`
/// answers from its LOCAL store only, so a clean no-record can also mean
/// not-yet-gossiped (full-arc law). The dwell converts one observation into a
/// standing verdict: the advertiser has had `min_dwell` of gossip time to
/// acquire the bytes and still stated none exist. `min_dwell == 0` restores
/// the active-is-enough behaviour (the OFF switch for the dwell alone).
///
/// **Concerns:** C4 (one observation is not a standing fact — the dwell is
/// what upgrades it); C3 (dwell never strands: it only DELAYS eligibility
/// within an already-finite window).
///
/// **Contract test:** [`tests::evidence_absent_stands_only_after_the_dwell`].
///
/// Pure and total — takes `elapsed` like [`backoff_is_active`], for the same
/// reason.
pub fn evidence_absent_stood(
    class: ContestSkip,
    elapsed: Duration,
    min_dwell: Duration,
    window: Duration,
) -> bool {
    class == ContestSkip::EvidenceAbsentBackoff
        && backoff_is_active(elapsed, window)
        && elapsed >= min_dwell
}

/// Ledger read for the decay arm: has the advertiser's stated-no-record verdict
/// for `id` stood unrefuted for at least `min_dwell` (and is it still inside
/// its window)? `recorded_at` is the FIRST observation of the current backoff
/// episode — while backed off, contests are skipped, so the entry is never
/// re-noted and the clock is not reset under the id.
pub fn evidence_absent_stood_for(id: &str, min_dwell: Duration, windows: BackoffWindows) -> bool {
    let Ok(guard) = ledger().lock() else {
        return false;
    };
    let Some(entry) = guard.get(id) else {
        return false;
    };
    evidence_absent_stood(
        entry.class,
        entry.recorded_at.elapsed(),
        min_dwell,
        windows.for_class(entry.class),
    )
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
        if guard.remove(id).is_some() {
            // Only a real removal dirties the snapshot: this is called on every
            // author-path landing, most of which touch no backed-off id, and a
            // no-op must not schedule a write.
            mark_dirty();
        }
    }
}

/// Entries currently held (expired-but-unobserved included). Observability and
/// test assertion only.
pub fn tracked() -> usize {
    ledger().lock().map(|g| g.len()).unwrap_or(0)
}

// =============================================================================
// SNAPSHOT PERSISTENCE — the deploy-restart re-churn cure (2026-08-03)
// =============================================================================

/// Snapshot file format version. Bumped when the on-disk shape changes; a
/// snapshot written by any other version is IGNORED (fresh start), never
/// partially interpreted.
pub const SNAPSHOT_FORMAT_VERSION: u32 = 1;

/// How often the snapshot task writes, when the ledger changed.
///
/// Coarse on purpose: this is de-duplication state whose worst-case loss is one
/// interval of re-learning, and the point of persisting is to survive DEPLOYS
/// (hours apart), not to be durable to the second.
pub const SNAPSHOT_INTERVAL: Duration = Duration::from_secs(120);

/// Set by every mutation; cleared when a snapshot is taken. The snapshot task
/// writes only when this is true, so a quiescent node does no periodic I/O at
/// all.
static DIRTY: AtomicBool = AtomicBool::new(false);

fn mark_dirty() {
    DIRTY.store(true, Ordering::Relaxed);
}

/// One persisted backoff, on ABSOLUTE epoch seconds.
///
/// `Instant` is monotonic-from-boot and meaningless across processes, so the
/// round trip goes through wall-clock epoch seconds — same node, same clock, so
/// no cross-node skew question arises. Both timestamps are stored: `recorded_at`
/// reconstructs the entry (letting a CHANGED window apply after a config flip),
/// while `expires_at` is what the load gate and the cap ordering read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedEntry {
    /// The content id held back.
    pub id: String,
    /// [`ContestSkip`]'s wire label. An unrecognised label drops the entry —
    /// same tolerance rule the head-record wire uses: a word this build cannot
    /// read may never be widened into a class it does understand.
    pub class: String,
    /// When the failure was recorded (epoch seconds).
    pub recorded_at_epoch_s: u64,
    /// When the hold lapses under the window in force at snapshot time (epoch
    /// seconds).
    pub expires_at_epoch_s: u64,
}

/// The whole snapshot file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedLedger {
    /// [`SNAPSHOT_FORMAT_VERSION`] at write time.
    pub version: u32,
    /// When the snapshot was taken (epoch seconds) — diagnostic only.
    pub saved_at_epoch_s: u64,
    /// The held entries.
    pub entries: Vec<PersistedEntry>,
}

/// What a load did. Returned rather than only logged so a test can assert the
/// drop reasons apart from each other.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LoadStats {
    /// Entries installed into the ledger.
    pub loaded: usize,
    /// Entries dropped because their hold had already lapsed.
    pub expired: usize,
    /// Entries dropped for an unreadable class or an unrepresentable age.
    pub unreadable: usize,
    /// Entries dropped to respect [`CONTEST_BACKOFF_CAP`].
    pub over_cap: usize,
}

/// Wall clock, epoch seconds. `0` if the clock is before the epoch (which would
/// make every entry read as long-expired — the fail-open direction).
fn now_epoch_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Parse a persisted class label back to its type. Unknown ⇒ `None` ⇒ the entry
/// is dropped.
fn class_from_label(label: &str) -> Option<ContestSkip> {
    use seam_contracts::ReasonLabel as _;
    ContestSkip::ALL
        .iter()
        .find(|c| c.label() == label)
        .copied()
}

/// Take a snapshot IF the ledger changed since the last one, clearing the dirty
/// flag. `None` means "nothing to write".
///
/// **Concerns:** C6a — the map is read and converted UNDER the lock (cheap,
/// allocation-only) and the file I/O happens outside it, so a slow disk can
/// never block the sweep's `skip_class` reads.
///
/// Expired entries are omitted rather than written and re-dropped on load.
pub fn take_snapshot_if_dirty(windows: BackoffWindows) -> Option<PersistedLedger> {
    if !DIRTY.swap(false, Ordering::Relaxed) {
        return None;
    }
    let now = now_epoch_s();
    let entries = {
        let Ok(guard) = ledger().lock() else {
            // Poisoned: restore the dirty flag so a later tick retries.
            mark_dirty();
            return None;
        };
        // bounded-work: one pass over the ledger, itself bounded by
        // CONTEST_BACKOFF_CAP. No I/O inside the lock.
        guard
            .iter()
            .filter_map(|(id, entry)| {
                use seam_contracts::ReasonLabel as _;
                let window = windows.for_class(entry.class);
                let elapsed = entry.recorded_at.elapsed();
                if !backoff_is_active(elapsed, window) {
                    return None;
                }
                let recorded_at_epoch_s = now.saturating_sub(elapsed.as_secs());
                Some(PersistedEntry {
                    id: id.clone(),
                    class: entry.class.label().to_string(),
                    recorded_at_epoch_s,
                    expires_at_epoch_s: recorded_at_epoch_s.saturating_add(window.as_secs()),
                })
            })
            .collect::<Vec<_>>()
    };
    Some(PersistedLedger {
        version: SNAPSHOT_FORMAT_VERSION,
        saved_at_epoch_s: now,
        entries,
    })
}

/// Which persisted entries may be installed, and in what shape — the pure half
/// of the load.
///
/// **Concerns:** C3 (an entry whose expiry has passed is DROPPED, so a long
/// downtime releases exactly what it should — persistence must never turn a
/// deferral into a longer one than the window promised). C6a (the cap is applied
/// here, dropping the SOONEST-expiring entries first: they are the ones a live
/// sweep re-learns most cheaply).
///
/// **Contract tests:** [`tests::an_expired_entry_is_dropped_on_load`],
/// [`tests::a_load_respects_the_cap_dropping_soonest_expiry_first`],
/// [`tests::an_unreadable_class_drops_only_its_own_entry`].
pub(crate) fn admissible(
    entries: Vec<PersistedEntry>,
    now_epoch_s: u64,
) -> (Vec<PersistedEntry>, LoadStats) {
    let mut stats = LoadStats::default();
    let mut kept: Vec<PersistedEntry> = Vec::new();
    // bounded-work: one pass over the file's entries, then one sort. The file is
    // written by this same module and is cap-bounded; a hand-edited oversized
    // file is truncated by the cap gate below rather than trusted.
    for entry in entries {
        if class_from_label(&entry.class).is_none() {
            stats.unreadable += 1;
            continue;
        }
        if entry.expires_at_epoch_s <= now_epoch_s {
            stats.expired += 1;
            continue;
        }
        if entry.recorded_at_epoch_s > now_epoch_s {
            // A future record time cannot be turned into an `Instant` in the
            // past; treat it as unreadable rather than inventing an age.
            stats.unreadable += 1;
            continue;
        }
        kept.push(entry);
    }
    if kept.len() > CONTEST_BACKOFF_CAP {
        // Latest expiry survives: those entries have the most remaining value
        // and are the most expensive to re-learn.
        kept.sort_by_key(|e| std::cmp::Reverse(e.expires_at_epoch_s));
        stats.over_cap = kept.len() - CONTEST_BACKOFF_CAP;
        kept.truncate(CONTEST_BACKOFF_CAP);
    }
    stats.loaded = kept.len();
    (kept, stats)
}

/// Install a decoded snapshot into the live ledger. Existing entries are
/// preserved (a snapshot load happens at boot, when the ledger is empty; if it
/// ever ran later, a live entry is the fresher evidence and wins).
pub fn install(snapshot: PersistedLedger) -> LoadStats {
    if snapshot.version != SNAPSHOT_FORMAT_VERSION {
        tracing::warn!(
            found = snapshot.version,
            expected = SNAPSHOT_FORMAT_VERSION,
            "contest_backoff: snapshot version mismatch — starting with an empty ledger"
        );
        return LoadStats::default();
    }
    let now = now_epoch_s();
    let (entries, stats) = admissible(snapshot.entries, now);
    let Ok(mut guard) = ledger().lock() else {
        tracing::warn!("contest_backoff: ledger lock poisoned; snapshot not installed");
        return LoadStats::default();
    };
    let base = Instant::now();
    for entry in entries {
        if guard.len() >= CONTEST_BACKOFF_CAP {
            break;
        }
        let Some(class) = class_from_label(&entry.class) else {
            continue;
        };
        let age = Duration::from_secs(now.saturating_sub(entry.recorded_at_epoch_s));
        let Some(recorded_at) = base.checked_sub(age) else {
            // Monotonic clock younger than the entry's age (a fresh boot after a
            // long uptime). Dropping is the FAIL-OPEN direction — the id is
            // contested next sweep, exactly as it would be with no snapshot.
            continue;
        };
        // A LIVE entry always wins: it is this process's own fresher evidence,
        // and at boot (the only call site) the ledger is empty anyway.
        guard
            .entry(entry.id)
            .or_insert(BackoffEntry { recorded_at, class });
    }
    stats
}

/// Write a snapshot atomically (temp + rename), creating the parent dir if
/// needed. Mirrors `services::arc_actuator`'s config-write discipline: a crash
/// mid-write can never leave a half-file where a whole one was.
pub fn save_to_path(path: &Path, snapshot: &PersistedLedger) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec(snapshot)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(&tmp, path)
}

/// Restore the ledger from `path` at boot.
///
/// EVERY failure is non-fatal and lands on the pre-snapshot behaviour (an empty
/// ledger): no file, unreadable file, malformed JSON, wrong version. This is
/// operational state — it may never be a reason a node does not start.
pub fn load_from_path(path: &Path) {
    let raw = match std::fs::read(path) {
        Ok(r) => r,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::info!(
                path = %path.display(),
                "contest backoff ledger: no snapshot found — starting fresh (every id contestable)"
            );
            return;
        }
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "contest backoff ledger: snapshot unreadable — starting fresh"
            );
            crate::metrics::inc_contest_backoff_snapshot(BackoffSnapshot::LoadFailed);
            return;
        }
    };
    let snapshot: PersistedLedger = match serde_json::from_slice(&raw) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "contest backoff ledger: snapshot corrupt — starting fresh"
            );
            crate::metrics::inc_contest_backoff_snapshot(BackoffSnapshot::LoadFailed);
            return;
        }
    };
    let stats = install(snapshot);
    crate::metrics::inc_contest_backoff_snapshot(BackoffSnapshot::Loaded);
    tracing::info!(
        path = %path.display(),
        loaded = stats.loaded,
        dropped_expired = stats.expired,
        dropped_unreadable = stats.unreadable,
        dropped_over_cap = stats.over_cap,
        "contest backoff ledger restored from snapshot — these ids keep serving the deferral \
         they had earned before the restart, instead of re-churning the corpus from zero"
    );
}

/// Start the periodic snapshot writer. Call once, from `main`, after the backoff
/// windows are published.
///
/// The write happens on the BLOCKING pool: a snapshot is a filesystem syscall
/// and a core tokio worker has no yield point inside one (the doorway
/// `getaddrinfo` wedge class).
pub fn spawn_snapshot_task(path: PathBuf) {
    tokio::spawn(async move {
        // bounded-work: one write per SNAPSHOT_INTERVAL, and only when the
        // ledger changed. Each write is a single file of at most
        // CONTEST_BACKOFF_CAP entries. No retry ladder — a failed write simply
        // re-marks the ledger dirty and the NEXT tick carries it.
        let mut ticker = tokio::time::interval(SNAPSHOT_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            let Some(snapshot) = take_snapshot_if_dirty(BackoffWindows::from_config()) else {
                continue;
            };
            let entries = snapshot.entries.len();
            let target = path.clone();
            let result =
                tokio::task::spawn_blocking(move || save_to_path(&target, &snapshot)).await;
            match result {
                Ok(Ok(())) => {
                    crate::metrics::inc_contest_backoff_snapshot(BackoffSnapshot::Saved);
                    tracing::debug!(
                        entries,
                        path = %path.display(),
                        "contest backoff ledger snapshotted"
                    );
                }
                Ok(Err(e)) => {
                    mark_dirty();
                    crate::metrics::inc_contest_backoff_snapshot(BackoffSnapshot::SaveFailed);
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "contest backoff ledger: snapshot write failed — the in-memory ledger \
                         stays authoritative and the next tick retries"
                    );
                }
                Err(e) => {
                    mark_dirty();
                    crate::metrics::inc_contest_backoff_snapshot(BackoffSnapshot::SaveFailed);
                    tracing::warn!(
                        error = %e,
                        "contest backoff ledger: snapshot task join failed — retrying next tick"
                    );
                }
            }
        }
    });
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

    /// The decay consumer's dwell contract: an evidence-absent verdict STANDS
    /// only inside the window AND past the dwell — a fresh observation never
    /// qualifies, an expired one never qualifies, a non-evidence class never
    /// qualifies, and dwell 0 restores active-is-enough.
    #[test]
    fn evidence_absent_stands_only_after_the_dwell() {
        let window = Duration::from_secs(21_600);
        let dwell = Duration::from_secs(3_600);
        let ea = ContestSkip::EvidenceAbsentBackoff;
        // Fresh observation: active but not yet standing.
        assert!(!evidence_absent_stood(ea, Duration::ZERO, dwell, window));
        // Past the dwell, inside the window: standing.
        assert!(evidence_absent_stood(ea, dwell, dwell, window));
        assert!(evidence_absent_stood(
            ea,
            window - Duration::from_secs(1),
            dwell,
            window
        ));
        // Window expired: no longer standing (C3 — never a permanent licence).
        assert!(!evidence_absent_stood(ea, window, dwell, window));
        // Wrong class never stands, whatever the clock says.
        assert!(!evidence_absent_stood(
            ContestSkip::NoLocalChainBackoff,
            dwell,
            dwell,
            window
        ));
        // Dwell 0 = active-is-enough (the dwell's own OFF switch).
        assert!(evidence_absent_stood(
            ea,
            Duration::ZERO,
            Duration::ZERO,
            window
        ));
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
            windows.for_class(ContestSkip::DeclareErrorBackoff),
            Duration::from_secs(3600),
            "a generic declare error must use the ordinary contest clock"
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

    // =========================================================================
    // SNAPSHOT PERSISTENCE
    // =========================================================================

    fn persisted(id: &str, class: ContestSkip, recorded: u64, expires: u64) -> PersistedEntry {
        use seam_contracts::ReasonLabel as _;
        PersistedEntry {
            id: id.to_string(),
            class: class.label().to_string(),
            recorded_at_epoch_s: recorded,
            expires_at_epoch_s: expires,
        }
    }

    /// THE POINT OF THE FEATURE: a deferral survives a restart. Without this,
    /// every deploy re-churns the corpus from zero — hours of re-learning the
    /// classifications the node had already paid conductor round-trips for.
    #[test]
    fn a_snapshot_round_trips_the_deferral_through_a_restart() {
        let _exclusive = test_exclusive();
        reset();
        note("snap:no-chain", ContestSkip::NoLocalChainBackoff);
        note("snap:evidence", ContestSkip::EvidenceAbsentBackoff);

        let snapshot =
            take_snapshot_if_dirty(WINDOWS).expect("a mutated ledger must produce a snapshot");
        assert_eq!(snapshot.version, SNAPSHOT_FORMAT_VERSION);
        assert_eq!(snapshot.entries.len(), 2);

        // Serde round trip — the file is JSON, so prove the shape survives it.
        let bytes = serde_json::to_vec(&snapshot).expect("snapshot serialises");
        let decoded: PersistedLedger = serde_json::from_slice(&bytes).expect("snapshot decodes");
        assert_eq!(decoded, snapshot);

        // "Restart": wipe the in-memory ledger, then install the snapshot.
        reset();
        assert_eq!(skip_class("snap:evidence", WINDOWS), None);
        let stats = install(decoded);
        assert_eq!(stats.loaded, 2);
        assert_eq!(
            skip_class("snap:no-chain", WINDOWS),
            Some(ContestSkip::NoLocalChainBackoff),
            "a restored entry keeps serving its deferral"
        );
        assert_eq!(
            skip_class(
                "snap:evidence",
                BackoffWindows {
                    contest: WINDOW,
                    evidence_absent: Duration::from_secs(86_400),
                }
            ),
            Some(ContestSkip::EvidenceAbsentBackoff),
            "the CLASS survives the round trip, not just the fact of a hold"
        );
        reset();
    }

    /// A quiescent node writes nothing: the dirty flag is the whole I/O budget.
    #[test]
    fn an_unchanged_ledger_produces_no_snapshot() {
        let _exclusive = test_exclusive();
        reset();
        note("dirty:id", ContestSkip::NoLocalChainBackoff);
        assert!(take_snapshot_if_dirty(WINDOWS).is_some());
        assert!(
            take_snapshot_if_dirty(WINDOWS).is_none(),
            "a second snapshot with no mutation in between must not write"
        );
        note("dirty:id2", ContestSkip::NoLocalChainBackoff);
        assert!(
            take_snapshot_if_dirty(WINDOWS).is_some(),
            "a mutation re-arms the writer"
        );
        reset();
    }

    /// C3 ACROSS THE RESTART. Persistence must not turn a deferral into a longer
    /// one than the window promised: an entry whose expiry passed while the
    /// process was down is DROPPED, so a day-long outage releases the corpus
    /// exactly as an uptime of the same length would have.
    #[test]
    fn an_expired_entry_is_dropped_on_load() {
        let now = 1_000_000u64;
        let (kept, stats) = admissible(
            vec![
                persisted("live", ContestSkip::NoLocalChainBackoff, now - 60, now + 60),
                persisted(
                    "lapsed",
                    ContestSkip::NoLocalChainBackoff,
                    now - 7200,
                    now - 1,
                ),
                persisted(
                    "exactly-now",
                    ContestSkip::EvidenceAbsentBackoff,
                    now - 100,
                    now,
                ),
            ],
            now,
        );
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].id, "live");
        assert_eq!(stats.expired, 2, "at exactly the expiry the hold is over");
    }

    /// Tolerance rule, same as the head-record wire's: a class label this build
    /// cannot read drops ITS OWN entry and nothing else. A future class must
    /// never be widened into one we do understand, and must never poison the
    /// whole restore (the fail-closed-collect shape that once emptied the EPR
    /// router).
    #[test]
    fn an_unreadable_class_drops_only_its_own_entry() {
        let now = 1_000_000u64;
        let (kept, stats) = admissible(
            vec![
                PersistedEntry {
                    id: "future".into(),
                    class: "some_class_from_2027".into(),
                    recorded_at_epoch_s: now - 10,
                    expires_at_epoch_s: now + 600,
                },
                persisted(
                    "known",
                    ContestSkip::NoLocalChainBackoff,
                    now - 10,
                    now + 600,
                ),
                PersistedEntry {
                    id: "time-traveller".into(),
                    class: "no_local_chain_backoff".into(),
                    recorded_at_epoch_s: now + 5000,
                    expires_at_epoch_s: now + 9000,
                },
            ],
            now,
        );
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].id, "known");
        assert_eq!(stats.unreadable, 2);
    }

    /// The cap holds on the LOAD path too, and drops the soonest-expiring
    /// entries — the ones a live sweep re-learns most cheaply.
    #[test]
    fn a_load_respects_the_cap_dropping_soonest_expiry_first() {
        let now = 1_000_000u64;
        // bounded-work: CONTEST_BACKOFF_CAP + 2 synthetic entries — the fixture
        // that proves the cap, bounded by the constant it asserts.
        let mut entries = Vec::with_capacity(CONTEST_BACKOFF_CAP + 2);
        entries.push(persisted(
            "soonest",
            ContestSkip::NoLocalChainBackoff,
            now - 10,
            now + 1,
        ));
        entries.push(persisted(
            "second-soonest",
            ContestSkip::NoLocalChainBackoff,
            now - 10,
            now + 2,
        ));
        for n in 0..CONTEST_BACKOFF_CAP {
            entries.push(persisted(
                &format!("bulk:{n}"),
                ContestSkip::EvidenceAbsentBackoff,
                now - 10,
                now + 86_400,
            ));
        }
        let (kept, stats) = admissible(entries, now);
        assert_eq!(kept.len(), CONTEST_BACKOFF_CAP);
        assert_eq!(stats.over_cap, 2);
        assert!(
            !kept
                .iter()
                .any(|e| e.id == "soonest" || e.id == "second-soonest"),
            "the soonest-expiring entries are the ones dropped"
        );
    }

    /// A snapshot from another format version is IGNORED, never partially
    /// interpreted — the same rule the wire uses for a word it cannot read.
    #[test]
    fn a_foreign_version_starts_fresh() {
        let _exclusive = test_exclusive();
        reset();
        let stats = install(PersistedLedger {
            version: SNAPSHOT_FORMAT_VERSION + 1,
            saved_at_epoch_s: now_epoch_s(),
            entries: vec![persisted(
                "foreign:id",
                ContestSkip::NoLocalChainBackoff,
                now_epoch_s(),
                now_epoch_s() + 3600,
            )],
        });
        assert_eq!(stats.loaded, 0);
        assert_eq!(tracked(), 0, "a foreign snapshot installs nothing");
        reset();
    }

    /// Through the real filesystem: write, read back, and confirm a CORRUPT file
    /// degrades to today's behaviour (empty ledger) rather than a failed boot.
    /// This is operational state; it may never be a reason a node does not start.
    #[test]
    fn a_corrupt_or_missing_snapshot_file_starts_fresh() {
        let _exclusive = test_exclusive();
        reset();
        let dir = tempfile::tempdir().expect("tempdir");

        // Missing file — silent fresh start.
        let missing = dir.path().join("nested").join("contest-backoff.json");
        load_from_path(&missing);
        assert_eq!(tracked(), 0);

        // Real round trip through the file, including parent-dir creation.
        note("file:id", ContestSkip::EvidenceAbsentBackoff);
        let snapshot = take_snapshot_if_dirty(WINDOWS).expect("snapshot");
        save_to_path(&missing, &snapshot).expect("save creates its parent dir and writes");
        reset();
        load_from_path(&missing);
        assert_eq!(
            skip_class("file:id", WINDOWS),
            Some(ContestSkip::EvidenceAbsentBackoff),
            "the hold survived a full file round trip"
        );

        // Corrupt file — fresh start, no panic.
        reset();
        std::fs::write(&missing, b"{not json at all").expect("write garbage");
        load_from_path(&missing);
        assert_eq!(tracked(), 0, "a corrupt snapshot must start empty");
        reset();
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
