//! CELL MEMBERSHIP — the conductor's own answer to "is this cell RUNNING?"
//!
//! ## Why this module exists
//!
//! `CellDisabled` does not mean what its name suggests. In the conductor fork
//! this node runs, the predicate behind that error is exactly:
//!
//! ```text
//! running_cells.get(cell_id)  -> Some(cell) => dispatch
//!                             -> None       => is it registered in an
//!                                              installed app?
//!                                              yes => CellDisabled
//! ```
//!
//! So `CellDisabled` means **registered in an installed app and ABSENT from
//! the in-memory running map**. It inspects no `AppStatus`, no zome `init`, no
//! wasm state and no network health. And `enable_app` on an app whose PERSISTED
//! status is already `Enabled` short-circuits before it looks at a single cell:
//!
//! ```text
//! // If app is already enabled, short circuit here.
//! if app.status == AppStatus::Enabled { return Ok(app.clone()); }
//! ```
//!
//! Those two facts together are the whole defect this module observes: a role
//! can be `Enabled` and not running INDEFINITELY, and the cure this crate
//! spends its bounded ladder on is, in that state, a no-op that answers `Ok`.
//! Measured on the fleet 2026-09-21: conductors reached `Conductor startup:
//! apps enabled.` between +56 min and +6 h 09 after a restart, and on four of
//! seven peers some roles stayed not-running after startup completed.
//!
//! ## What is, and is not, evidence
//!
//! * **`ListCellIds` IS.** Its admin handler returns `running_cell_ids()` —
//!   the keys of the SAME map zome dispatch looks in. It is the one admin read
//!   that answers the membership question truthfully.
//! * **`app_info` / `list_apps` are NOT.** They answer INSTALLATION and STATUS.
//!   `AppInfo` copies the persisted status, and the provisioned-cell branch of
//!   its constructor carries the upstream comment `populate enabled with cell
//!   state once it is implemented for a base cell` — so `cell_info` presence and
//!   `Enabled` say nothing about the running map.
//! * **Neither proves the NEXT call will succeed.** Membership says the cell is
//!   in the map; a zome call that RETURNS is still the only proof of recovery
//!   this node accepts ([`crate::conductor_bridge_health::record_role_success`]).
//!   Membership's job is to tell the supervisor which CURE is even applicable.
//!
//! ## bounded-work
//!
//! * **Rate budget** — one `ListCellIds` per [`MEMBERSHIP_REFRESH_INTERVAL`]
//!   (15s), process-wide, across every role that asks. Five supervisor tasks on
//!   a 20s tick therefore cost roughly ONE admin read per 15–20s between them,
//!   not five: the gate is on the CACHE, not on the caller, so it holds no
//!   matter how many drivers appear.
//! * **Concurrency budget** — ONE refresh in flight, claimed atomically. The
//!   window check and the window store used to be separate operations, so two
//!   askers could both read "stale" before either stored its attempt and both
//!   dial; and a read outliving the 15s window admitted a second while the
//!   first was still outstanding. Losers now COALESCE onto the answer the
//!   winner will store.
//! * **Deadline budget** — [`MEMBERSHIP_READ_DEADLINE`] (10s), not the
//!   `holochain_client` default of 60s. Both the supervisor tick and the HTTP
//!   diagnostics handler await this inline.
//! * **Attempt-paced, not success-paced.** The window advances on every
//!   ATTEMPT, so an admin read that keeps failing is rate-limited exactly like
//!   one that succeeds — a conductor that refuses this read is, by hypothesis,
//!   already unwell.
//! * **Memory budget** — one `HashSet<CellId>` sized by the conductor's own
//!   running-cell count (roles × hosted agents), replaced wholesale on each
//!   read. Nothing accumulates.
//! * No loop, no task and no timer of its own: the existing
//!   `BRIDGE_PROBE_INTERVAL` tick stays the only pacing authority.
//!
//! ### Why a caller-side deadline is legitimate HERE
//!
//! `src/.epr-meta`'s `conductor-call-is-uncancellable` rule is about ZOME
//! calls: abandoning one leaves the conductor executing wasm and holding a DB
//! read permit with nobody listening, so a caller timeout makes the saturation
//! it reacts to worse. `AdminRequest::ListCellIds` is not that: its handler is
//! `running_cell_ids()` — a lock-and-clone of an in-memory map. There is no
//! wasm body, no DB permit and no queue to inherit, so abandoning the response
//! costs the conductor nothing and the bound is real rather than cosmetic.
//!
//! ## Freshness is part of the answer, and it EXPIRES
//!
//! [`MembershipCache::cell_running`] answers `Some` only while the last
//! SUCCESSFUL read is younger than [`MEMBERSHIP_AUTHORITY_TTL`] and was taken
//! after the last [`MembershipCache::invalidate`]. Older or revoked readings
//! answer `None` — they survive for the diagnostics surface (`readAgeSecs`,
//! `runningCellIds`) and for nothing else.
//!
//! That distinction is the difference between an instrument and a trap. The
//! 15s refresh interval bounds how often the node ASKS; it says nothing about
//! how long an answer stays true. Without an authority window the sequence
//! *(cache holds the cell) → (conductor restarts with its app genuinely
//! Disabled) → (every later `ListCellIds` fails)* keeps answering
//! `Some(true)` forever, which classifies a genuinely-disabled app as
//! "membership present" and probes it on every tick while NEVER enabling it —
//! the recovery ladder disabled indefinitely by a stale `true`.
//!
//! A membership answer that guessed would be strictly worse than the
//! `app_info` it replaces, because it would be believed.
//!
//! ## Every answer carries its ORDER, not just its timestamp
//!
//! A reading is stamped with a sequence from
//! [`crate::conductor_bridge_health::next_obs_seq`] — the one process-wide
//! observation clock this node orders ALL arguable observations by: membership
//! readings, `app_info` status answers, and proven zome-call successes. The
//! epoch-ms stamp stays for the diagnostics surface and decides nothing.
//!
//! Wall time could not do this job. Two observations inside the same
//! millisecond compare EQUAL, so a strict `>` reports the OLDER one as current
//! — which let an absence observed at epoch-ms 1000 overwrite a recovery that
//! landed later within that same millisecond. And a wall clock can step
//! BACKWARDS, at which point "later" and "larger" are simply different
//! relations.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

use holochain_client::CellId;
use tracing::info;

/// Longest a membership reading may be reused before the next asker refreshes
/// it. Deliberately just under the supervisor's `BRIDGE_PROBE_INTERVAL` (20s)
/// so each tick gets a reading of its own, while several roles ticking inside
/// the same window share ONE admin read.
///
/// A constant rather than a settings knob: there is no existing per-observation
/// cadence setting this fits, and inventing one would be a configuration
/// surface with no reader. The pacing authority that matters — how often the
/// CURE is attempted — is already
/// [`crate::services::enable_app_backoff::enable_backoff`].
pub const MEMBERSHIP_REFRESH_INTERVAL: Duration = Duration::from_secs(15);

/// How long a SUCCESSFUL membership reading keeps its AUTHORITY over a recovery
/// decision. Past it, [`MembershipCache::cell_running`] answers `None`.
///
/// 60s = four refresh intervals. Generous enough that one or two failed reads
/// do not flap a healthy role into `membership-unknown`, tight enough that a
/// conductor restart cannot be papered over by a reading taken before it. The
/// reading itself is KEPT for the diagnostics surface; only its authority
/// expires. A constant rather than a settings knob for the same reason as
/// [`MEMBERSHIP_REFRESH_INTERVAL`]: the pacing authority that matters is
/// already [`crate::services::enable_app_backoff::enable_backoff`].
pub const MEMBERSHIP_AUTHORITY_TTL: Duration = Duration::from_secs(60);

/// Deadline on ONE `ListCellIds` round trip. `holochain_client`'s default
/// request timeout is 60s and both the supervisor tick and the HTTP diagnostics
/// handler await this inline, so an unbounded read stalls a 20s loop for a
/// minute. See the module doc for why a caller-side deadline is sound for this
/// particular admin call and not for a zome call.
pub const MEMBERSHIP_READ_DEADLINE: Duration = Duration::from_secs(10);

/// THE TWO CLOCKS ONE OBSERVATION NEEDS.
///
/// `mono` measures DURATIONS — is the refresh window open, has the authority
/// window expired. `wall_ms` renders AGES for the diagnostics surface and
/// decides nothing.
///
/// They are separate because a wall clock can step BACKWARDS (ntp, a resumed
/// VM, a container clock correction) and `now_ms.saturating_sub(then_ms)` then
/// answers `0` for as long as the rollback lasts. Measured against the
/// production predicates: a one-hour backward step suppressed the next
/// membership refresh for about an hour plus fifteen seconds and kept a reading
/// authoritative for about an hour plus a minute — during which a cached
/// absence with an Enabled app and no organic traffic pins the supervisor on
/// `ObserveOnly`. `Instant` is monotonic by construction and cannot do that.
///
/// Passed explicitly rather than read inside, for the same reason `now_ms` is:
/// a test that drives a synthetic clock while the code reads the real one is
/// measuring skew.
#[derive(Debug, Clone, Copy)]
pub struct ObservedAt {
    /// Monotonic. Decides every window and every expiry.
    pub mono: Instant,
    /// Wall clock, epoch-ms. Diagnostics only.
    pub wall_ms: u64,
}

impl ObservedAt {
    /// Both clocks, read now.
    pub fn now() -> Self {
        Self {
            mono: Instant::now(),
            wall_ms: crate::conductor_bridge_health::now_ms(),
        }
    }

    /// `wall_ms` advanced by `delta`, monotonic advanced by the same amount —
    /// the ordinary forward-clock case, for tests that do not care about skew.
    #[cfg(test)]
    pub fn plus_ms(&self, delta: u64) -> Self {
        Self {
            mono: self.mono + Duration::from_millis(delta),
            wall_ms: self.wall_ms.saturating_add(delta),
        }
    }
}

/// The epoch-ms the test clock's monotonic half is anchored to. Every test
/// stamp is `ANCHOR + delta`, and the monotonic half advances by that same
/// delta. A stamp EARLIER than the anchor clamps to it — a wall clock may step
/// backwards, a monotonic one may not.
#[cfg(test)]
const TEST_CLOCK_ANCHOR_MS: u64 = 1_700_000_000_000;

#[cfg(test)]
impl From<u64> for ObservedAt {
    /// TEST ONLY. Anchors an epoch-ms literal to a fixed process base instant so
    /// an ordinary forward-clock test reads exactly as it did when the two
    /// clocks were one number: `T0 + 1_000` advances BOTH halves by a second.
    ///
    /// A wall stamp EARLIER than the base clamps the monotonic half to the base
    /// rather than moving it back — which is precisely the real semantics, and
    /// why a test that wants a rollback with time still passing constructs the
    /// two halves itself.
    fn from(wall_ms: u64) -> Self {
        // A FIXED epoch anchor, not "whatever the first call passed": tests run
        // in parallel, so anchoring to the first caller made the mapping depend
        // on scheduling order and two tests disagreed about how much time a
        // given stamp represented.
        static MONO_BASE: OnceLock<Instant> = OnceLock::new();
        let mono0 = *MONO_BASE.get_or_init(Instant::now);
        Self {
            mono: mono0 + Duration::from_millis(wall_ms.saturating_sub(TEST_CLOCK_ANCHOR_MS)),
            wall_ms,
        }
    }
}

/// Where a [`MembershipCache`] gets its running-cell set.
///
/// A trait so the whole state machine is drivable with no conductor and no
/// websocket — the same injectable-clock/injectable-source discipline
/// [`crate::conductor_bridge_health`] documents. `Err` carries the conductor's
/// own words: a failed membership read is itself a diagnosis.
#[async_trait::async_trait]
pub trait RunningCellSource: Send + Sync {
    async fn running_cell_ids(&self) -> Result<Vec<CellId>, String>;
}

#[async_trait::async_trait]
impl RunningCellSource for holochain_client::AdminWebsocket {
    async fn running_cell_ids(&self) -> Result<Vec<CellId>, String> {
        // `AdminRequest::ListCellIds` → the conductor's `running_cell_ids()`.
        // The inherent method is what we want here; the trait method above is
        // the injectable wrapper around it.
        holochain_client::AdminWebsocket::list_cell_ids(self)
            .await
            .map_err(|e| e.to_string())
    }
}

/// One successful reading of the conductor's running-cell map.
#[derive(Debug, Clone)]
pub struct MembershipReading {
    pub cells: HashSet<CellId>,
    /// Epoch-ms the reading was taken. DIAGNOSTICS ONLY (`readAgeSecs`) — it
    /// orders nothing and it expires nothing.
    pub at_ms: u64,
    /// MONOTONIC instant the reading was taken. The authority window is measured
    /// from this, so a backward wall-clock step cannot prolong it.
    pub taken_at: Instant,
    /// [`crate::conductor_bridge_health::next_obs_seq`] stamp CLAIMED BEFORE
    /// the request went out.
    ///
    /// Before, not after, and that is the conservative direction on purpose: a
    /// reading describes the running map at some instant between the request
    /// and the response, and stamping it at the request means anything that
    /// happened during the round trip — a zome call landing, an `app_info`
    /// answer — is ordered AFTER it. So a proven recovery always outranks a
    /// reading that was already in flight, and newer status evidence always
    /// gets to contradict it. Both are the safe way to be wrong.
    pub seq: u64,
}

/// A membership answer plus the identity of the observation it came from.
///
/// The pair travels together because a publisher that stamps an answer with its
/// OWN clock claims an observation it did not make — the substitution that let
/// a cached absence, republished a second after a zome call proved the cell
/// alive, overwrite the recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MembershipAnswer {
    /// `Some(true)`/`Some(false)` from an AUTHORITATIVE reading; `None` when
    /// this node has no reading entitled to decide.
    pub running: Option<bool>,
    /// Epoch-ms of the observation. Diagnostics only.
    pub at_ms: u64,
    /// Observation ORDER of the answer — the reading's own `seq`, or, for an
    /// unknown, the sequence of the ATTEMPT that failed to establish one.
    pub seq: u64,
}

/// What one [`MembershipCache::refresh_if_stale_at`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshOutcome {
    /// The cached reading is inside the refresh window; nothing was asked.
    Cached,
    /// Another refresh was already in flight; this caller coalesced onto it and
    /// asked nothing. Distinct from [`Self::Cached`] because the reason differs:
    /// `Cached` means the answer is fresh, `Coalesced` means someone else is
    /// fetching it right now.
    Coalesced,
    /// A fresh `ListCellIds` landed, with this many running cells.
    Read { running: usize },
    /// The read landed but a NEWER reading had already been stored, so this one
    /// was discarded rather than allowed to rewind the answer.
    Superseded,
    /// The admin read failed or blew its deadline. The previous reading (if any)
    /// is KEPT for the diagnostics surface — a failed read is not evidence that
    /// the cells stopped running — but it no longer refreshes the AUTHORITY
    /// window, so an old reading eventually expires into `None`.
    Failed { msg: String },
}

/// Releases the single-flight claim on drop, so a panic or an early return
/// inside a refresh cannot wedge the cache shut for the life of the process.
struct RefreshClaim<'a>(&'a AtomicBool);

impl Drop for RefreshClaim<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// The node's cached view of the conductor's running-cell map.
#[derive(Debug, Default)]
pub struct MembershipCache {
    last: RwLock<Option<MembershipReading>>,
    /// Epoch-ms of the last ATTEMPT, successful or not; 0 = never asked.
    /// DIAGNOSTICS ONLY — the refresh window is measured from `last_attempt_at`.
    last_attempt_ms: AtomicU64,
    /// MONOTONIC instant of the last attempt. The refresh window is measured
    /// from this, so a backward wall-clock step cannot suspend refreshing.
    last_attempt_at: RwLock<Option<Instant>>,
    /// Epoch-ms of the last revocation. DIAGNOSTICS ONLY.
    invalidated_at_ms: AtomicU64,
    /// Observation sequence at which every reading taken at or before it lost
    /// its AUTHORITY — set by [`MembershipCache::invalidate`] on a conductor
    /// reconnect. The reading itself is kept; only its right to decide a cure is
    /// revoked.
    ///
    /// ORDER, not milliseconds, and this one is not merely tidiness: comparing
    /// `reading.at_ms <= invalidated_at_ms` after a BACKWARD clock step lets a
    /// reading of the OLD conductor's running map — taken before the re-mint but
    /// stamped with a larger millisecond — survive the revocation. That is the
    /// exact stale-`true` this module exists to refuse, reintroduced through the
    /// clock.
    invalidated_seq: AtomicU64,
    /// Sequence of the last ATTEMPT, successful or not; 0 = never asked.
    ///
    /// It is what an UNKNOWN membership answer is stamped with: "the moment
    /// this node last learned it could not know" is a real observation and
    /// must be orderable against a recovery like any other.
    last_attempt_seq: AtomicU64,
    /// Sequence of the reading currently stored in `last`; 0 = none. Claimed
    /// from [`crate::conductor_bridge_health::next_obs_seq`] before each read,
    /// so a slow response cannot overwrite a newer one that already landed.
    stored_seq: AtomicU64,
    /// Is a refresh in flight? The single-flight claim.
    refreshing: AtomicBool,
    /// The most recent failure's words, kept so the diagnostics surface can
    /// say WHY membership is unknown. Cleared by the next success.
    last_error: RwLock<Option<String>>,
    reads: AtomicU64,
    failures: AtomicU64,
}

impl MembershipCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Is a refresh due as of `at`? `true` before the first attempt.
    ///
    /// Measured on the MONOTONIC half. With epoch-ms, a one-hour backward clock
    /// step made `now_ms.saturating_sub(last)` answer `0` and suspended
    /// refreshing for about an hour plus the window.
    pub fn is_stale_at(&self, at: ObservedAt) -> bool {
        match *self
            .last_attempt_at
            .read()
            .unwrap_or_else(|e| e.into_inner())
        {
            None => true,
            Some(last) => at.mono.saturating_duration_since(last) >= MEMBERSHIP_REFRESH_INTERVAL,
        }
    }

    /// Refresh from `src` if and only if the window has elapsed AND no other
    /// refresh is in flight.
    ///
    /// The claim is atomic and the window is stored by the WINNER only, so the
    /// check/check/store/store interleaving that let two askers both dial is
    /// closed by construction. Losers return [`RefreshOutcome::Coalesced`] and
    /// read whatever answer is current — they do not queue a second admin call.
    pub async fn refresh_if_stale_at(
        &self,
        src: &dyn RunningCellSource,
        at: ObservedAt,
    ) -> RefreshOutcome {
        if !self.is_stale_at(at) {
            return RefreshOutcome::Cached;
        }
        self.claim_and_refresh_at(src, at).await
    }

    /// The half of [`Self::refresh_if_stale_at`] that runs AFTER a caller has
    /// decided the window looks stale: claim the single flight, then decide
    /// again.
    ///
    /// Extracted so the ONE schedule the window alone cannot refuse is
    /// drivable: a caller reads "stale", is descheduled before it can claim, a
    /// second caller claims/reads/releases inside the same window, and the
    /// first resumes. Its staleness decision is now a fact about a world that
    /// has moved on, so the claim is not permission — the RECHECK under the
    /// claim is. Without it the pair issues two `ListCellIds` for one window.
    ///
    /// A caller that arrives here and finds the window already covered returns
    /// [`RefreshOutcome::Cached`] and releases the claim on drop, exactly as if
    /// it had never passed the outer check.
    async fn claim_and_refresh_at(
        &self,
        src: &dyn RunningCellSource,
        at: ObservedAt,
    ) -> RefreshOutcome {
        // ONE in flight, whoever asks. Claimed BEFORE the window is stored so a
        // loser cannot mistake the winner's store for its own permission.
        if self
            .refreshing
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return RefreshOutcome::Coalesced;
        }
        let _claim = RefreshClaim(&self.refreshing);
        // RE-DECIDE UNDER THE CLAIM. Between the outer check and this line the
        // window may have been filled by a refresh that has already completed.
        if !self.is_stale_at(at) {
            return RefreshOutcome::Cached;
        }
        self.last_attempt_ms.store(at.wall_ms, Ordering::Relaxed);
        *self
            .last_attempt_at
            .write()
            .unwrap_or_else(|e| e.into_inner()) = Some(at.mono);
        // The observation's ORDER, claimed before the request goes out — see
        // `MembershipReading::seq` for why before rather than after. One
        // process-wide clock, so this stamp is comparable with a proven zome
        // success and with an `app_info` answer.
        let seq = crate::conductor_bridge_health::next_obs_seq();
        self.last_attempt_seq.store(seq, Ordering::SeqCst);

        let answer =
            match tokio::time::timeout(MEMBERSHIP_READ_DEADLINE, src.running_cell_ids()).await {
                Ok(answer) => answer,
                Err(_) => Err(format!(
                "ListCellIds did not answer within {}s (deadline; the conductor's admin socket is \
                 accepting but not replying)",
                MEMBERSHIP_READ_DEADLINE.as_secs()
            )),
            };

        match answer {
            Ok(cells) => {
                // NEVER let an older response rewind the answer. Two responses
                // can complete backwards (a slow first, a fast second), and a
                // rewind here is a membership answer that goes backwards in time
                // while claiming to be current.
                if seq <= self.stored_seq.load(Ordering::SeqCst) {
                    return RefreshOutcome::Superseded;
                }
                let cells: HashSet<CellId> = cells.into_iter().collect();
                let running = cells.len();
                *self.last.write().unwrap_or_else(|e| e.into_inner()) = Some(MembershipReading {
                    cells,
                    at_ms: at.wall_ms,
                    taken_at: at.mono,
                    seq,
                });
                self.stored_seq.store(seq, Ordering::SeqCst);
                *self.last_error.write().unwrap_or_else(|e| e.into_inner()) = None;
                self.reads.fetch_add(1, Ordering::Relaxed);
                RefreshOutcome::Read { running }
            }
            Err(msg) => {
                *self.last_error.write().unwrap_or_else(|e| e.into_inner()) = Some(msg.clone());
                self.failures.fetch_add(1, Ordering::Relaxed);
                RefreshOutcome::Failed { msg }
            }
        }
    }

    /// [`Self::refresh_if_stale_at`] against the wall clock.
    pub async fn refresh_if_stale(&self, src: &dyn RunningCellSource) -> RefreshOutcome {
        self.refresh_if_stale_at(src, ObservedAt::now()).await
    }

    /// REVOKE the authority of every reading taken at or before `at_ms`.
    ///
    /// Called when the bridge to the conductor is re-minted: the process on the
    /// far side may be a different one, and a reading of the OLD conductor's
    /// running map says nothing about the new one's. The reading is kept so
    /// `readAgeSecs` and `runningCellIds` still render; it simply stops deciding
    /// cures until a fresh read lands. Also clears the refresh window so the
    /// next asker re-reads immediately rather than waiting out 15s of a reading
    /// that no longer counts.
    pub fn invalidate_at(&self, at_ms: u64, why: &str) {
        self.invalidate_at_seq(at_ms, crate::conductor_bridge_health::next_obs_seq(), why);
    }

    /// The half of [`Self::invalidate_at`] that runs AFTER the revoking
    /// observation has claimed its sequence.
    ///
    /// Extracted so the ONE schedule a plain store cannot refuse is drivable:
    /// two role supervisors invalidate this SHARED cache concurrently, each
    /// claims a sequence, and they arrive OUT OF ORDER. Entering here with the
    /// older sequence IS the slower invalidator resuming.
    pub(crate) fn invalidate_at_seq(&self, at_ms: u64, seq: u64, why: &str) {
        self.invalidated_at_ms.store(at_ms, Ordering::SeqCst);
        // `fetch_max`, NEVER a plain store. A plain store lets the boundary go
        // BACKWARDS: A claims 10 and pauses, a read claims 11, B claims and
        // stores 12 (revoking 11), then A resumes and stores 10 — and reading 11
        // becomes authoritative again, after a re-mint declared it dead. The
        // counter is monotonic; the boundary has to be too.
        self.invalidated_seq.fetch_max(seq, Ordering::SeqCst);
        self.last_attempt_ms.store(0, Ordering::Relaxed);
        self.last_attempt_seq.store(0, Ordering::SeqCst);
        *self
            .last_attempt_at
            .write()
            .unwrap_or_else(|e| e.into_inner()) = None;
        *self.last_error.write().unwrap_or_else(|e| e.into_inner()) = Some(format!(
            "membership authority revoked: {why} — no reading decides a cure until a fresh \
             ListCellIds lands"
        ));
        info!(
            why,
            "cell-membership authority revoked — the next supervisor tick re-reads ListCellIds \
             before choosing any cure"
        );
    }

    /// [`Self::invalidate_at`] against the wall clock.
    pub fn invalidate(&self, why: &str) {
        self.invalidate_at(crate::conductor_bridge_health::now_ms(), why);
    }

    /// The reading that currently has AUTHORITY over a recovery decision, if
    /// any: a success taken after the last revocation and younger than
    /// [`MEMBERSHIP_AUTHORITY_TTL`].
    fn authoritative_at(&self, at: ObservedAt) -> Option<MembershipReading> {
        let guard = self.last.read().unwrap_or_else(|e| e.into_inner());
        let reading = guard.as_ref()?;
        if reading.seq <= self.invalidated_seq.load(Ordering::SeqCst) {
            return None;
        }
        // MONOTONIC expiry. With epoch-ms, a one-hour backward clock step kept a
        // reading authoritative for about an hour past its TTL.
        if at.mono.saturating_duration_since(reading.taken_at) >= MEMBERSHIP_AUTHORITY_TTL {
            return None;
        }
        Some(reading.clone())
    }

    /// Does a reading currently hold authority? Diagnostics and tests.
    pub fn is_authoritative_at(&self, at: ObservedAt) -> bool {
        self.authoritative_at(at).is_some()
    }

    /// Is `cell_id` in the conductor's running map?
    ///
    /// `None` when this node has no AUTHORITATIVE reading — never read, read
    /// but expired past [`MEMBERSHIP_AUTHORITY_TTL`], or revoked by
    /// [`Self::invalidate`]. The honest "I do not know", never a fabricated
    /// `false` and never a believed-forever `true`.
    pub fn cell_running_at(&self, cell_id: &CellId, at: ObservedAt) -> Option<bool> {
        self.authoritative_at(at).map(|r| r.cells.contains(cell_id))
    }

    /// [`Self::cell_running_at`] against the wall clock.
    pub fn cell_running(&self, cell_id: &CellId) -> Option<bool> {
        self.cell_running_at(cell_id, ObservedAt::now())
    }

    /// [`Self::cell_running_at`] PLUS the epoch-ms of the observation the
    /// answer came from — the reading's own `at_ms`, never the moment of the
    /// call.
    ///
    /// This pair is what a publisher must carry. Stamping a publication with
    /// `now` claims an observation was taken now, and that is how a CACHED
    /// absence, published a second after a zome call proved the cell alive,
    /// overwrote a real recovery: `last_success > observed_at` compared a real
    /// success against a fabricated timestamp and read false. The reading's
    /// identity travels with the reading.
    ///
    /// When there is no authoritative reading the answer is `None` and the
    /// stamp is the last ATTEMPT (when this node last learned it could not
    /// know), falling back to `now_ms` before the first attempt. Both are real
    /// observations of an absence of knowledge; neither is a reading's identity
    /// borrowed by something younger.
    pub fn cell_running_observed_at(&self, cell_id: &CellId, at: ObservedAt) -> MembershipAnswer {
        match self.authoritative_at(at) {
            Some(reading) => MembershipAnswer {
                running: Some(reading.cells.contains(cell_id)),
                at_ms: reading.at_ms,
                seq: reading.seq,
            },
            None => {
                // An UNKNOWN is an observation too: "as of my last attempt I
                // could not establish this". It is stamped with that attempt so
                // a recovery that landed afterwards still outranks it — and,
                // before anything has ever been asked, with a FRESH sequence,
                // because "I have never looked" is being observed right now.
                let (at_ms, seq) = match self.last_attempt_seq.load(Ordering::SeqCst) {
                    0 => (at.wall_ms, crate::conductor_bridge_health::next_obs_seq()),
                    seq => (self.last_attempt_ms.load(Ordering::Relaxed), seq),
                };
                MembershipAnswer {
                    running: None,
                    at_ms,
                    seq,
                }
            }
        }
    }

    /// Epoch-ms of the last refresh ATTEMPT, successful or not; 0 = never
    /// asked. Diagnostics only.
    pub fn last_attempt_ms(&self) -> u64 {
        self.last_attempt_ms.load(Ordering::Relaxed)
    }

    /// Observation ORDER of the last refresh attempt; 0 = never asked.
    pub fn last_attempt_seq(&self) -> u64 {
        self.last_attempt_seq.load(Ordering::SeqCst)
    }

    /// How many cells the last successful reading found running.
    pub fn running_count(&self) -> Option<usize> {
        self.last
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|r| r.cells.len())
    }

    /// Age of the last SUCCESSFUL reading in whole seconds, as of `now_ms`.
    pub fn read_age_secs_at(&self, now_ms: u64) -> Option<u64> {
        self.last
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|r| now_ms.saturating_sub(r.at_ms) / 1000)
    }

    /// Why the last membership read failed, if the last one did.
    pub fn last_error(&self) -> Option<String> {
        self.last_error
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Successful reads / failed reads since boot. Observability only.
    pub fn counts(&self) -> (u64, u64) {
        (
            self.reads.load(Ordering::Relaxed),
            self.failures.load(Ordering::Relaxed),
        )
    }

    /// The running cells as canonical strings, sorted, for the diagnostics
    /// surface. `None` when there has never been a successful read.
    pub fn cell_ids_rendered(&self) -> Option<Vec<String>> {
        let guard = self.last.read().unwrap_or_else(|e| e.into_inner());
        let reading = guard.as_ref()?;
        let mut out: Vec<String> = reading
            .cells
            .iter()
            .map(|c| format!("{}/{}", c.dna_hash(), c.agent_pubkey()))
            .collect();
        out.sort();
        Some(out)
    }
}

/// The ONE process-wide cache. Tests build their own [`MembershipCache`] — the
/// parallel-test discipline `conductor_bridge_health::bridge_health` documents.
pub fn membership() -> &'static MembershipCache {
    static CACHE: OnceLock<MembershipCache> = OnceLock::new();
    CACHE.get_or_init(MembershipCache::new)
}

#[cfg(test)]
pub(crate) mod fake {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    /// A [`RunningCellSource`] whose answer a test sets, counting every call,
    /// and which can be PARKED mid-call so a concurrency bound is asserted
    /// against a real overlap rather than a sequential simulation of one.
    ///
    /// The call COUNT is the load-bearing half: the refresh window and the
    /// single-flight claim are both bounds on admin round trips, and a bound is
    /// only asserted by counting them.
    pub struct FakeAdmin {
        answer: std::sync::Mutex<Result<Vec<CellId>, String>>,
        calls: AtomicUsize,
        /// When set, the call parks until `release` is notified. Models the slow
        /// conductor the deadline and the single-flight claim exist for.
        parked: AtomicBool,
        release: tokio::sync::Notify,
    }

    impl FakeAdmin {
        pub fn new(answer: Result<Vec<CellId>, String>) -> Self {
            Self {
                answer: std::sync::Mutex::new(answer),
                calls: AtomicUsize::new(0),
                parked: AtomicBool::new(false),
                release: tokio::sync::Notify::new(),
            }
        }

        /// Change what the conductor answers from the next call on.
        pub fn set_tail(&self, answer: Result<Vec<CellId>, String>) {
            *self.answer.lock().unwrap_or_else(|e| e.into_inner()) = answer;
        }

        pub fn call_count(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }

        /// Make every subsequent call park until [`Self::release`].
        pub fn park(&self) {
            self.parked.store(true, Ordering::SeqCst);
        }

        /// Let a parked call finish (and stop parking later ones).
        pub fn release(&self) {
            self.parked.store(false, Ordering::SeqCst);
            self.release.notify_waiters();
        }

        /// Resolve once a call has been entered.
        ///
        /// Polled rather than notified on purpose: a `Notify` created after the
        /// notification has already fired loses the wakeup, and this is exactly
        /// the race the caller is trying to stand inside.
        pub async fn wait_until_entered(&self) {
            while self.calls.load(Ordering::SeqCst) == 0 {
                tokio::task::yield_now().await;
            }
        }
    }

    #[async_trait::async_trait]
    impl RunningCellSource for FakeAdmin {
        async fn running_cell_ids(&self) -> Result<Vec<CellId>, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.parked.load(Ordering::SeqCst) {
                self.release.notified().await;
            }
            self.answer
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::FakeAdmin;
    use super::*;

    const T0: u64 = 1_700_000_000_000;

    /// An epoch-ms literal as a two-clock observation instant. See
    /// [`ObservedAt`]'s test-only `From<u64>`: both halves advance together, so
    /// these tests read as they did when the clock was one number.
    fn at(wall_ms: u64) -> ObservedAt {
        ObservedAt::from(wall_ms)
    }

    fn cell(seed: u8) -> CellId {
        use holochain_types::prelude::{AgentPubKey, DnaHash};
        CellId::new(
            DnaHash::from_raw_32(vec![seed; 32]),
            AgentPubKey::from_raw_32(vec![seed.wrapping_add(100); 32]),
        )
    }

    #[tokio::test]
    async fn a_never_read_cache_answers_unknown_and_never_false() {
        let cache = MembershipCache::new();
        assert_eq!(
            cache.cell_running_at(&cell(1), at(T0)),
            None,
            "a cache with no reading must not fabricate a membership answer"
        );
        assert_eq!(cache.running_count(), None);
        assert_eq!(cache.read_age_secs_at(T0), None);
        assert!(!cache.is_authoritative_at(at(T0)));
    }

    #[tokio::test]
    async fn a_successful_read_answers_membership_both_ways() {
        let cache = MembershipCache::new();
        let admin = FakeAdmin::new(Ok(vec![cell(1), cell(2)]));
        assert_eq!(
            cache.refresh_if_stale_at(&admin, at(T0)).await,
            RefreshOutcome::Read { running: 2 }
        );
        assert_eq!(cache.cell_running_at(&cell(1), at(T0)), Some(true));
        assert_eq!(cache.cell_running_at(&cell(2), at(T0)), Some(true));
        assert_eq!(
            cache.cell_running_at(&cell(3), at(T0)),
            Some(false),
            "registered-but-absent is the whole point: a definite NOT running"
        );
        assert_eq!(cache.running_count(), Some(2));
        assert_eq!(cache.read_age_secs_at(T0 + 4_000), Some(4));
        assert!(cache.is_authoritative_at(at(T0 + 4_000)));
    }

    /// The bound, stated as the thing it forbids: five supervisor tasks on a
    /// 20s tick must not become five admin reads per tick.
    #[tokio::test]
    async fn the_refresh_window_is_shared_by_every_asker() {
        let cache = MembershipCache::new();
        let admin = FakeAdmin::new(Ok(vec![cell(1)]));

        assert_eq!(
            cache.refresh_if_stale_at(&admin, at(T0)).await,
            RefreshOutcome::Read { running: 1 }
        );
        for role in 0..5 {
            assert_eq!(
                cache
                    .refresh_if_stale_at(&admin, at(T0 + 1_000 + role * 100))
                    .await,
                RefreshOutcome::Cached,
                "role {role} inside the window must reuse the reading"
            );
        }
        assert_eq!(admin.call_count(), 1, "one admin read, five askers");

        // A tick after the window: exactly one more read.
        assert!(cache.is_stale_at(at(T0 + 15_000)));
        assert_eq!(
            cache.refresh_if_stale_at(&admin, at(T0 + 15_000)).await,
            RefreshOutcome::Read { running: 1 }
        );
        assert_eq!(admin.call_count(), 2);
    }

    // ---- F1: the authority window ----------------------------------------

    /// THE P1. `cell_running` used to be timestamp-free, so a reading survived
    /// its own relevance: fifteen seconds bounded how often the node ASKS, not
    /// how long an answer stays true.
    #[tokio::test]
    async fn an_expired_reading_loses_its_authority_but_stays_readable() {
        let cache = MembershipCache::new();
        let admin = FakeAdmin::new(Ok(vec![cell(1)]));
        cache.refresh_if_stale_at(&admin, at(T0)).await;

        let ttl = MEMBERSHIP_AUTHORITY_TTL.as_millis() as u64;
        assert_eq!(
            cache.cell_running_at(&cell(1), at(T0 + ttl - 1)),
            Some(true),
            "inside the TTL the answer decides cures"
        );
        assert_eq!(
            cache.cell_running_at(&cell(1), at(T0 + ttl)),
            None,
            "at the TTL its authority is gone — expired is UNKNOWN, not stale-true"
        );
        assert!(!cache.is_authoritative_at(at(T0 + ttl)));
        // ...and it is still readable for the operator surface.
        assert_eq!(cache.read_age_secs_at(T0 + ttl), Some(ttl / 1000));
        assert_eq!(cache.running_count(), Some(1));
        assert!(cache.cell_ids_rendered().is_some());
    }

    /// A conductor re-mint revokes authority immediately, without waiting out
    /// the TTL: the process on the far side may be a different one.
    #[tokio::test]
    async fn a_conductor_reconnect_revokes_authority_at_once() {
        let cache = MembershipCache::new();
        let admin = FakeAdmin::new(Ok(vec![cell(1)]));
        cache.refresh_if_stale_at(&admin, at(T0)).await;
        assert_eq!(cache.cell_running_at(&cell(1), at(T0 + 1_000)), Some(true));

        cache.invalidate_at(T0 + 2_000, "bridge re-mint (test)");
        assert_eq!(
            cache.cell_running_at(&cell(1), at(T0 + 2_100)),
            None,
            "a reading of the OLD conductor's map decides nothing about the new one"
        );
        assert!(
            cache.is_stale_at(at(T0 + 2_100)),
            "and the refresh window is cleared so the next tick re-reads at once"
        );

        // A fresh read after the revocation is authoritative again.
        admin.set_tail(Ok(vec![]));
        assert_eq!(
            cache.refresh_if_stale_at(&admin, at(T0 + 2_200)).await,
            RefreshOutcome::Read { running: 0 }
        );
        assert_eq!(cache.cell_running_at(&cell(1), at(T0 + 2_300)), Some(false));
    }

    /// THE FAILING SCENARIO THE REVIEW NAMED, end to end on the cache: a
    /// successful read, then a conductor restart, then every later read fails.
    /// The stale `true` must NOT survive as an authority — otherwise a
    /// genuinely-Disabled app classifies as "membership present" and the
    /// recovery ladder never runs.
    #[tokio::test]
    async fn a_restart_plus_repeated_read_failures_ends_in_unknown_not_stale_true() {
        let cache = MembershipCache::new();
        let admin = FakeAdmin::new(Ok(vec![cell(1)]));
        cache.refresh_if_stale_at(&admin, at(T0)).await;
        assert_eq!(cache.cell_running_at(&cell(1), at(T0)), Some(true));

        // The conductor restarts; storage re-mints its bridge.
        cache.invalidate_at(T0 + 1_000, "conductor restart (test)");
        admin.set_tail(Err("ListCellIds failed: K2SpaceNotFound".to_string()));

        // Every later read fails, for ten minutes of ticks.
        for tick in 1..=30u64 {
            let now = T0 + 1_000 + tick * 20_000;
            let outcome = cache.refresh_if_stale_at(&admin, at(now)).await;
            assert!(
                matches!(outcome, RefreshOutcome::Failed { .. }),
                "tick {tick}: the read really is failing"
            );
            assert_eq!(
                cache.cell_running_at(&cell(1), at(now)),
                None,
                "tick {tick}: an unreadable membership is UNKNOWN — the stale true is gone"
            );
        }
        assert_eq!(cache.counts().1, 30, "thirty failed reads, all bounded");
    }

    #[tokio::test]
    async fn a_failed_read_keeps_the_previous_answer_for_diagnostics_and_is_rate_limited_too() {
        let cache = MembershipCache::new();
        let admin = FakeAdmin::new(Ok(vec![cell(1)]));
        cache.refresh_if_stale_at(&admin, at(T0)).await;
        assert_eq!(cache.cell_running_at(&cell(1), at(T0)), Some(true));

        admin.set_tail(Err("ListCellIds failed: K2SpaceNotFound".to_string()));
        let outcome = cache.refresh_if_stale_at(&admin, at(T0 + 15_000)).await;
        assert!(matches!(outcome, RefreshOutcome::Failed { .. }));
        assert_eq!(
            cache.cell_running_at(&cell(1), at(T0 + 15_000)),
            Some(true),
            "inside the TTL one bad round trip is not evidence the cell stopped running"
        );
        assert_eq!(
            cache.last_error().as_deref(),
            Some("ListCellIds failed: K2SpaceNotFound")
        );

        // The window advanced on the ATTEMPT: a failing conductor is not asked
        // harder than a working one.
        assert_eq!(
            cache.refresh_if_stale_at(&admin, at(T0 + 20_000)).await,
            RefreshOutcome::Cached
        );
        assert_eq!(admin.call_count(), 2);

        // But a failed read does NOT refresh the authority window, so the old
        // answer expires on schedule from when it was TAKEN.
        let ttl = MEMBERSHIP_AUTHORITY_TTL.as_millis() as u64;
        assert_eq!(cache.cell_running_at(&cell(1), at(T0 + ttl)), None);

        // And a later success clears the error and the answer moves.
        admin.set_tail(Ok(vec![]));
        let later = T0 + ttl + 1_000;
        assert_eq!(
            cache.refresh_if_stale_at(&admin, at(later)).await,
            RefreshOutcome::Read { running: 0 }
        );
        assert_eq!(cache.cell_running_at(&cell(1), at(later)), Some(false));
        assert_eq!(cache.last_error(), None);
        assert_eq!(cache.counts(), (2, 1));
    }

    #[tokio::test]
    async fn a_first_read_that_fails_leaves_membership_unknown() {
        // The one state that must never become a `false`: nothing has ever been
        // observed, so nothing may be claimed.
        let cache = MembershipCache::new();
        let admin = FakeAdmin::new(Err("no connection".to_string()));
        assert!(matches!(
            cache.refresh_if_stale_at(&admin, at(T0)).await,
            RefreshOutcome::Failed { .. }
        ));
        assert_eq!(cache.cell_running_at(&cell(1), at(T0)), None);
        assert_eq!(cache.cell_ids_rendered(), None);
    }

    // ---- F3: single-flight, deadline, no rewind ---------------------------

    /// TWO WAYS a second asker can arrive while a read is outstanding, and
    /// neither may dial a second `ListCellIds`.
    ///
    /// The review named both: the check/check/store/store interleaving (two
    /// askers inside the same window), and a read LASTING LONGER THAN THE WINDOW,
    /// which lets a later tick in while the first is still outstanding. The first
    /// is refused by the window, the second by the single-flight claim, and only
    /// the second can reach `Coalesced` — so both are asserted.
    #[tokio::test]
    async fn a_second_asker_never_dials_a_second_admin_read() {
        let cache = std::sync::Arc::new(MembershipCache::new());
        let admin = std::sync::Arc::new(FakeAdmin::new(Ok(vec![cell(1)])));
        admin.park();

        let (c1, a1) = (cache.clone(), admin.clone());
        let winner = tokio::spawn(async move { c1.refresh_if_stale_at(a1.as_ref(), at(T0)).await });
        // Stand inside the overlap: the winner is in the call and has not returned.
        admin.wait_until_entered().await;

        // (a) Same instant — the window the winner stored already covers it.
        assert_eq!(
            cache
                .refresh_if_stale_at(admin.as_ref(), at(T0 + 1_000))
                .await,
            RefreshOutcome::Cached,
            "an asker inside the refresh window must reuse, never dial"
        );

        // (b) A tick PAST the window while the first read is still outstanding —
        // the case a window alone cannot close. The claim must.
        let (c2, a2) = (cache.clone(), admin.clone());
        let late = tokio::spawn(async move {
            c2.refresh_if_stale_at(a2.as_ref(), at(T0 + 15_000 + 1))
                .await
        });
        assert_eq!(
            late.await.unwrap(),
            RefreshOutcome::Coalesced,
            "a read outliving its window must not admit a second flight"
        );

        admin.release();
        assert_eq!(winner.await.unwrap(), RefreshOutcome::Read { running: 1 });
        assert_eq!(
            admin.call_count(),
            1,
            "ONE admin read across both askers — check/check/store/store used to admit two"
        );
    }

    /// THE F3 SCHEDULE THE WINDOW ALONE CANNOT REFUSE, driven at the point it
    /// happens: caller A reads "stale" and is descheduled BEFORE it can claim;
    /// caller B claims, reads and releases inside the same window; A resumes
    /// into the claim.
    ///
    /// A's staleness decision is, by then, a fact about a world that has moved
    /// on. Entering [`MembershipCache::claim_and_refresh_at`] IS A resuming —
    /// the outer check it already passed is not replayed, which is precisely
    /// the schedule the previous cut's check-then-claim admitted a second
    /// `ListCellIds` on.
    #[tokio::test]
    async fn a_caller_parked_before_the_claim_does_not_re_read_inside_the_window() {
        let cache = MembershipCache::new();
        let admin = FakeAdmin::new(Ok(vec![cell(1)]));

        // A checks: nothing has ever been read, so the window is stale.
        assert!(cache.is_stale_at(at(T0)), "A's check: a refresh is due");
        // ...and A parks here, before it can claim.

        // B runs the whole refresh to completion inside the same window.
        assert_eq!(
            cache.refresh_if_stale_at(&admin, at(T0)).await,
            RefreshOutcome::Read { running: 1 }
        );
        assert_eq!(admin.call_count(), 1);

        // A resumes INTO THE CLAIM, carrying its stale decision. The recheck
        // under the claim is what must refuse it.
        assert_eq!(
            cache.claim_and_refresh_at(&admin, at(T0 + 1)).await,
            RefreshOutcome::Cached,
            "A's pre-claim staleness decision must be re-made under the claim"
        );
        assert_eq!(
            admin.call_count(),
            1,
            "ONE admin read for the window — the check/park/claim schedule used to make two"
        );
        assert!(
            !cache.refreshing.load(Ordering::SeqCst),
            "and the claim A took was released, so the next window is not wedged shut"
        );

        // The window still expires normally afterwards.
        assert_eq!(
            cache
                .refresh_if_stale_at(
                    &admin,
                    at(T0 + MEMBERSHIP_REFRESH_INTERVAL.as_millis() as u64)
                )
                .await,
            RefreshOutcome::Read { running: 1 }
        );
        assert_eq!(admin.call_count(), 2);
    }

    /// THE INVARIANT the read-sequence guard exists for, asserted as an
    /// OUTCOME through the real entry point: a burst of askers arriving while a
    /// slow read is outstanding must never leave the stored answer older than
    /// the newest completed read.
    ///
    /// Driven with real requests rather than by rewinding the private sequence,
    /// because a rewind asserts the guard's arithmetic and not the behaviour.
    /// Note what this pins: under single-flight the guard is UNREACHABLE
    /// through this API — every late asker coalesces, and a read abandoned on
    /// the deadline has its request future dropped with it, so no response can
    /// arrive late enough to rewind. The guard stays as the belt under that
    /// brace; the invariant is what is asserted.
    #[tokio::test]
    async fn a_burst_of_askers_never_leaves_an_older_answer_stored() {
        let cache = std::sync::Arc::new(MembershipCache::new());
        let slow = std::sync::Arc::new(FakeAdmin::new(Ok(vec![cell(1)])));
        slow.park();

        let (c1, s1) = (cache.clone(), slow.clone());
        let first = tokio::spawn(async move { c1.refresh_if_stale_at(s1.as_ref(), at(T0)).await });
        slow.wait_until_entered().await;

        // Six askers arrive while the first read is still outstanding, spread
        // across and past the refresh window.
        for (i, delta) in [1_000u64, 5_000, 14_999, 15_000, 30_000, 60_000]
            .into_iter()
            .enumerate()
        {
            let outcome = cache
                .refresh_if_stale_at(slow.as_ref(), at(T0 + delta))
                .await;
            assert!(
                matches!(outcome, RefreshOutcome::Cached | RefreshOutcome::Coalesced),
                "asker {i} at +{delta}ms must not start a second flight, got {outcome:?}"
            );
        }

        slow.release();
        assert_eq!(first.await.unwrap(), RefreshOutcome::Read { running: 1 });
        assert_eq!(
            slow.call_count(),
            1,
            "ONE admin read across the whole burst"
        );

        // A genuinely newer read lands and becomes the answer.
        let fast = FakeAdmin::new(Ok(vec![cell(2)]));
        let later = T0 + 80_000;
        assert_eq!(
            cache.refresh_if_stale_at(&fast, at(later)).await,
            RefreshOutcome::Read { running: 1 }
        );
        assert_eq!(cache.cell_running_at(&cell(2), at(later)), Some(true));
        assert_eq!(
            cache.cell_running_at(&cell(1), at(later)),
            Some(false),
            "the stored answer is the NEWEST completed read, never an older one"
        );
    }

    /// A publication must carry the READING's own identity — its ORDER and its
    /// timestamp — not the moment it is published.
    #[tokio::test]
    async fn a_membership_answer_carries_the_identity_of_the_read_that_produced_it() {
        let cache = MembershipCache::new();
        let admin = FakeAdmin::new(Ok(vec![cell(1)]));
        cache.refresh_if_stale_at(&admin, at(T0)).await;
        let read_seq = cache.last_attempt_seq();
        assert!(read_seq > 0, "the read claimed an observation sequence");

        // Read it much later: the ANSWER is the cached one, and so is its
        // identity. `now` must not leak into either half.
        let answer = cache.cell_running_observed_at(&cell(2), at(T0 + 9_000));
        assert_eq!(answer.running, Some(false));
        assert_eq!(
            answer.at_ms, T0,
            "the stamp is the reading's own at_ms, not the moment of the call"
        );
        assert_eq!(
            answer.seq, read_seq,
            "and its ORDER is the sequence the read claimed before it went out"
        );

        // Past the authority window: unknown, stamped with the last ATTEMPT —
        // when this node last learned it could not know.
        let ttl = MEMBERSHIP_AUTHORITY_TTL.as_millis() as u64;
        let answer = cache.cell_running_observed_at(&cell(1), at(T0 + ttl));
        assert_eq!(answer.running, None);
        assert_eq!(
            answer.at_ms, T0,
            "the last attempt, not the read-out moment"
        );
        assert_eq!(answer.seq, read_seq, "ordered by the attempt, not by now");

        // Before anything has ever been asked, the unknown is being observed
        // RIGHT NOW, so it earns a fresh sequence rather than 0 — a 0 would
        // rank below every recovery and silence the gauge forever.
        let fresh = MembershipCache::new();
        let answer = fresh.cell_running_observed_at(&cell(1), at(T0 + 1234));
        assert_eq!(answer.running, None);
        assert_eq!(answer.at_ms, T0 + 1234);
        assert!(
            answer.seq > read_seq,
            "a never-asked cache observes its own ignorance now: {} must outrank {read_seq}",
            answer.seq
        );
        assert_eq!(fresh.last_attempt_ms(), 0);
        assert_eq!(fresh.last_attempt_seq(), 0);
    }

    /// ORDER SURVIVES A CLOCK THAT GOES BACKWARDS, and a tie cannot happen.
    ///
    /// The two shapes epoch-ms comparison could not answer: a second reading
    /// taken with a SMALLER wall-clock stamp than the first (ntp correction, a
    /// resumed VM), and two observations inside the same millisecond.
    #[tokio::test]
    async fn observation_order_is_independent_of_the_wall_clock() {
        let cache = MembershipCache::new();
        let admin = FakeAdmin::new(Ok(vec![cell(1)]));

        cache.refresh_if_stale_at(&admin, at(T0)).await;
        let first = cache.last_attempt_seq();

        // The bridge is re-minted and the clock steps BACKWARDS by a minute, so
        // the next read is taken "earlier" in wall time than the reading the
        // revocation is meant to kill. With `reading.at_ms <= invalidated_at_ms`
        // this arm revoked the NEW reading and kept nothing; the inverse
        // rollback let a reading of the OLD conductor survive the re-mint.
        admin.set_tail(Ok(vec![cell(2)]));
        let rolled_back = T0 - 60_000;
        cache.invalidate_at(rolled_back, "bridge re-mint under a clock rollback (test)");
        cache.refresh_if_stale_at(&admin, at(rolled_back)).await;
        let second = cache.last_attempt_seq();
        assert_eq!(
            cache.cell_running_at(&cell(1), at(rolled_back)),
            Some(false),
            "the reading taken AFTER the revocation is authoritative, even though its wall stamp \
             is older than the revoked one's"
        );

        assert!(
            second > first,
            "the later observation must rank higher even though its ms stamp is smaller \
             ({second} vs {first})"
        );
        let answer = cache.cell_running_observed_at(&cell(2), at(rolled_back));
        assert_eq!(
            answer.running,
            Some(true),
            "the newer reading is the answer"
        );
        assert!(answer.at_ms < T0, "its wall stamp really did go backwards");
        assert_eq!(answer.seq, second);

        // And no two observations ever share a sequence.
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..50 {
            assert!(
                seen.insert(crate::conductor_bridge_health::next_obs_seq()),
                "the observation clock handed out a duplicate"
            );
        }
        assert!(
            !seen.contains(&0),
            "0 is reserved for `never observed` and must never be handed out"
        );
    }

    /// NEW-3. Two supervisors invalidating the SHARED cache concurrently must
    /// not move the revocation boundary BACKWARDS.
    ///
    /// The exact interleaving, which a plain store admitted:
    ///
    /// ```text
    /// invalidator A claims seq=10, pauses before its store
    /// a membership read claims seq=11 and lands
    /// invalidator B claims and stores seq=12 — reading 11 is revoked
    /// A resumes and stores seq=10 — reading 11 is AUTHORITATIVE again
    /// ```
    ///
    /// A revoked reading coming back to life is the stale-`true` this module
    /// exists to refuse, arriving through the revocation itself.
    #[tokio::test]
    async fn a_late_invalidation_cannot_resurrect_a_revoked_reading() {
        let cache = MembershipCache::new();
        let admin = FakeAdmin::new(Ok(vec![cell(1)]));

        // Invalidator A claims its sequence and is descheduled before its store.
        let a_seq = crate::conductor_bridge_health::next_obs_seq();

        // The membership read claims a LATER sequence and lands.
        cache.refresh_if_stale_at(&admin, at(T0)).await;
        assert!(cache.last_attempt_seq() > a_seq);
        assert_eq!(cache.cell_running_at(&cell(1), at(T0 + 1_000)), Some(true));

        // Invalidator B claims a later sequence still, and completes.
        cache.invalidate_at(T0 + 2_000, "invalidator B (test)");
        assert_eq!(
            cache.cell_running_at(&cell(1), at(T0 + 2_100)),
            None,
            "B's revocation kills the reading"
        );

        // A RESUMES — through the production store, carrying its older sequence.
        cache.invalidate_at_seq(T0 + 2_500, a_seq, "invalidator A resuming (test)");
        assert_eq!(
            cache.cell_running_at(&cell(1), at(T0 + 3_000)),
            None,
            "the revoked reading STAYS revoked — a plain store let A's older boundary \
             resurrect it"
        );
    }

    /// NEW-4. A one-hour BACKWARD wall-clock step must not suspend refreshing
    /// or prolong a reading's authority.
    ///
    /// Measured against the old epoch-ms predicates: the next refresh was
    /// suppressed for about an hour plus fifteen seconds and the reading kept
    /// deciding cures for about an hour plus a minute — during which a cached
    /// absence with an Enabled app and no organic traffic pins the supervisor
    /// on `ObserveOnly`.
    ///
    /// The two halves are constructed by hand here on purpose: this is the one
    /// case where they DISAGREE — real time keeps passing while the wall clock
    /// goes back — and the test-only `From<u64>` deliberately cannot express it.
    #[tokio::test]
    async fn a_backward_wall_clock_step_suspends_neither_refresh_nor_expiry() {
        const HOUR_MS: u64 = 3_600_000;
        let cache = MembershipCache::new();
        let admin = FakeAdmin::new(Ok(vec![cell(1)]));

        let start = at(T0);
        cache.refresh_if_stale_at(&admin, start).await;
        assert_eq!(cache.cell_running_at(&cell(1), start), Some(true));
        assert_eq!(admin.call_count(), 1);

        // THE ROLLBACK. Twenty real seconds pass; the wall clock reports an hour
        // EARLIER than the reading was taken.
        let rolled_back = ObservedAt {
            mono: start.mono + Duration::from_secs(20),
            wall_ms: T0 - HOUR_MS,
        };
        assert!(
            rolled_back.wall_ms < start.wall_ms,
            "precondition: the wall clock really did go backwards"
        );
        assert!(
            cache.is_stale_at(rolled_back),
            "a refresh is DUE — 20 real seconds is past the 15s window, whatever the wall says"
        );
        admin.set_tail(Ok(vec![]));
        assert_eq!(
            cache.refresh_if_stale_at(&admin, rolled_back).await,
            RefreshOutcome::Read { running: 0 },
            "and the read actually happens"
        );
        assert_eq!(admin.call_count(), 2);

        // AUTHORITY EXPIRY is monotonic too. A reading taken at `start` is past
        // its 60s TTL after 61 real seconds, whatever the wall clock says.
        let fresh = MembershipCache::new();
        let quiet = FakeAdmin::new(Ok(vec![cell(1)]));
        fresh.refresh_if_stale_at(&quiet, start).await;
        let much_later = ObservedAt {
            mono: start.mono + MEMBERSHIP_AUTHORITY_TTL + Duration::from_secs(1),
            wall_ms: T0 - HOUR_MS,
        };
        assert_eq!(
            fresh.cell_running_at(&cell(1), much_later),
            None,
            "the authority window expired on REAL elapsed time; epoch-ms kept it alive for an \
             hour past its TTL"
        );
        // ...and the diagnostics surface still renders off the wall clock.
        assert_eq!(fresh.read_age_secs_at(T0 + 5_000), Some(5));
    }

    /// A read that never answers must not park the tick for the client's 60s
    /// default; it must fail on this module's own deadline.
    #[tokio::test(start_paused = true)]
    async fn a_read_that_never_answers_fails_on_the_deadline() {
        let cache = MembershipCache::new();
        let admin = FakeAdmin::new(Ok(vec![cell(1)]));
        admin.park(); // never released

        let outcome = cache.refresh_if_stale_at(&admin, at(T0)).await;
        match outcome {
            RefreshOutcome::Failed { msg } => assert!(
                msg.contains("deadline"),
                "the deadline must name itself: {msg}"
            ),
            other => panic!("expected a deadline failure, got {other:?}"),
        }
        assert_eq!(
            cache.cell_running_at(&cell(1), at(T0)),
            None,
            "and nothing was learned"
        );
        // The claim was released on drop, so the next tick may try again.
        assert!(!cache.refreshing.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn rendered_cell_ids_are_sorted_and_carry_both_halves() {
        let cache = MembershipCache::new();
        let admin = FakeAdmin::new(Ok(vec![cell(9), cell(2)]));
        cache.refresh_if_stale_at(&admin, at(T0)).await;
        let rendered = cache.cell_ids_rendered().expect("a reading exists");
        assert_eq!(rendered.len(), 2);
        let mut sorted = rendered.clone();
        sorted.sort();
        assert_eq!(rendered, sorted, "stable order for a diffable surface");
        assert!(
            rendered.iter().all(|s| s.contains('/')),
            "each entry names the DNA and the agent: {rendered:?}"
        );
    }
}
