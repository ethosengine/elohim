//! Conductor bridge health — is the ZOME PATH actually alive?
//!
//! WHY THIS MODULE EXISTS. `/health` answered `200 {"status":"ok"}` for as long
//! as the process was up, and its `conductor` block reported only `mode`
//! (`embedded` | `external`) plus DNA hashes read out of a CACHED `CellId`.
//! Every one of those fields survives a conductor restart untouched, because
//! none of them ever crosses the websocket. So on a conductor-only restart
//! (storage keeps running) the node advertised itself healthy indefinitely
//! while every zome call died `Websocket closed: No connection`, `POST
//! /db/content` was accepted `201` and landed NULL-anchored, and nothing —
//! no probe, no status code, no log line beyond a 60s heartbeat warning —
//! said the truth-writing path was gone.
//!
//! This module is the honest observation. It is deliberately:
//!
//! * **Pure + injectable-clock.** Every decision is a total function of
//!   observations and a `now_ms`, so the whole policy is unit-testable with no
//!   conductor, no websocket and no sleeping.
//! * **Per-instance, not implicitly global.** [`bridge_health`] exposes ONE
//!   process-wide instance for production wiring, but every constructor is
//!   public so tests build their own and cannot race each other through shared
//!   mutable state (the parallel-test flake class documented in `CLAUDE.md`).
//! * **Evidence-shaped, not intention-shaped.** `Unknown` is a real state: a
//!   node that has not yet attempted a zome call has NOT observed a live path
//!   and must not claim one. It also must not cry dead — hence three states,
//!   not two.
//!
//! ## What counts as evidence
//!
//! A zome call that fails for a DOMAIN reason (`ZomeNotFound`, a wasm error, a
//! validation refusal) is proof the path is **LIVE** — the bytes crossed the
//! websocket and the conductor answered. Only a TRANSPORT failure is evidence
//! the path is dead. And a conductor-admission shed is evidence of NOTHING:
//! nothing was dispatched, the conductor never saw the call (see
//! `conductor_admission::ADMISSION_SHED_MARKER`). Collapsing those three into
//! a boolean is exactly how a shed once reached the wire as a bare 500.

//! ## What counts as RECOVERY (corrected 2026-09-20)
//!
//! Only ONE thing: a zome call on the role that SUCCEEDS. Not `enable_app`
//! returning `Ok`, and not the app's own status.
//!
//! The reason is a household reproduction of the alpha incident. A conductor
//! accepts websocket traffic BEFORE its enabled apps finish initializing, and a
//! cell that is installed but absent from the conductor's `running_cells`
//! answers every zome call `CellDisabled` — while `app_info()` cheerfully
//! reports `AppStatus::Enabled`, because the APP is enabled; it is its CELLS
//! that are not running. `enable_app` on an already-enabled app is a NO-OP that
//! returns success without touching those cells.
//!
//! Reading either of those answers as recovery produced this, every ~20s, for
//! as long as the conductor took to finish starting (~11 minutes on one peer):
//!
//! ```text
//! WARN  lamad  conductor app is NOT RUNNING (CellDisabled …)
//! INFO  lamad  conductor app is RUNNING again — the enable backoff is reset
//! WARN  lamad  NOT RUNNING      <- 1.8 s later
//! ```
//!
//! — a false verdict, a log that flaps, and a bounded backoff that never backs
//! off because every false recovery reset it. So the fold is asymmetric on
//! purpose: an app status of "not running" is believed (it can only understate
//! health), an app status of "enabled" is believed about NOTHING (it routinely
//! overstates it), and the ladder is cleared exclusively by
//! [`record_role_success`], which only a real zome call reaches.

//! ## RESPONSIVE is not SERVED (corrected 2026-09-21)
//!
//! Two facts wore one name, and that is the second half of the same defect.
//!
//! * **Responsive** — the conductor answered. A `ZomeNotFound`, a wasm guest
//!   error, a validation refusal: bytes crossed the websocket and something on
//!   the far side composed a reply. That is evidence the TRANSPORT is alive, and
//!   it is evidence of nothing else.
//! * **Served** — a zome call on this role's cell RETURNED. That, and only
//!   that, proves the cell is in the conductor's `running_cells` and this node
//!   can write truth through it.
//!
//! Before this correction every unrecognised error string took
//! [`ZomeObservation::PathResponsive`] (then spelled `PathLive`) straight into
//! `record_success`, which ended the not-running episode, so:
//!
//! * a role whose cells were refusing calls could be flipped to `Live` by a
//!   FAILED call, while `elohim_conductor_app_enabled{role}` — set only by the
//!   real transition — stayed `0`; and
//! * the next genuine success then read `before.status == Live` and skipped
//!   clearing the enable ladder, so a LATER outage inherited the previous
//!   outage's backoff and was met at the 1h cap instead of at 60s.
//!
//! Now [`BridgeHealth::record_responsive_at`] clears a `Dead` verdict (the
//! transport is demonstrably back) and REFUSES to touch an `AppDisabled` one — a
//! domain error from a cell that will not serve is not that cell serving. It
//! touches neither the episode clock, the diagnosis latch, the disabled reason,
//! the gauge, nor the ladder. All five belong to the ONE transition,
//! [`record_role_success`], which is reached from a zome call that returned and
//! from nowhere else — including the supervisor's read-only cell probe
//! ([`crate::services::cell_probe`]), which is a zome call and therefore uses
//! the same single path rather than a second one.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

use crate::services::enable_app_backoff::{enable_ledger, EnableLedger};

/// Wall-clock milliseconds since the epoch, saturating at 0 on a pre-epoch
/// clock. Only used to stamp observations; all policy is a function of
/// differences, so a clock step degrades an age, never a status.
/// THE ONE MONOTONIC OBSERVATION CLOCK for this process.
///
/// Every observation that can ARGUE with another observation is stamped from
/// here: a membership reading, an `app_info` status answer, a proven zome-call
/// success, a transport failure, a not-running verdict. Ordering between them
/// is then a fact, not an inference from two independently sampled wall clocks.
///
/// WHY WALL-CLOCK MILLISECONDS COULD NOT DO THIS JOB. Two observations landing
/// inside the same millisecond compare EQUAL, and a strict `>` then reports the
/// OLDER one as current — which is how a cached absence observed at epoch-ms
/// 1000 could overwrite a recovery that landed later within that same
/// millisecond. And a wall clock can step BACKWARDS (ntp, a suspended VM, a
/// container clock correction), at which point "later" and "larger" stop being
/// the same relation. Epoch-ms stays on every observation for the diagnostics
/// surface — ages, `readAgeSecs`, `notRunningSecs` — and decides nothing.
///
/// `SeqCst` so two threads observing concurrently agree on which observation
/// was last. `0` is reserved for "never observed" and is never handed out.
pub fn next_obs_seq() -> u64 {
    static OBS_SEQ: AtomicU64 = AtomicU64::new(0);
    OBS_SEQ.fetch_add(1, Ordering::SeqCst) + 1
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// The state of the conductor zome path as this node has actually observed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZomePathStatus {
    /// The most recent evidence is a zome call that crossed the websocket.
    Live,
    /// The most recent evidence is a TRANSPORT failure — the websocket is gone.
    Dead,
    /// The most recent evidence is the conductor answering that the APP IS NOT
    /// RUNNING. The websocket is fine; nothing can be written through it.
    ///
    /// Its own state rather than a flavour of `Dead`, because the CURE is
    /// different: `Dead` is repaired by re-minting the bridge, and a re-mint on
    /// a disabled app is pure churn. This one is repaired by `enable_app`.
    AppDisabled,
    /// No evidence either way yet (fresh boot, or only admission sheds so far).
    Unknown,
}

impl ZomePathStatus {
    /// The wire spelling carried on `/health` and `/health/serving`.
    pub fn as_str(self) -> &'static str {
        match self {
            ZomePathStatus::Live => "live",
            ZomePathStatus::Dead => "dead",
            ZomePathStatus::AppDisabled => "app-disabled",
            ZomePathStatus::Unknown => "unknown",
        }
    }
}

/// What one zome-call error proves about the path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZomeObservation {
    /// The conductor answered (even if it answered "no") — the TRANSPORT is
    /// responsive.
    ///
    /// NAMED FOR WHAT IT PROVES, after being named `PathLive` cost a false
    /// recovery. A `ZomeNotFound`, a wasm guest error or a validation refusal
    /// says the websocket carried bytes and the far side replied. It does NOT
    /// say this role's cell served the call — a cell absent from the
    /// conductor's `running_cells` is perfectly capable of producing a domain
    /// error, and reading that as recovery is how a failed call flipped a role
    /// green while its gauge stayed at zero. See the module doc's
    /// "RESPONSIVE is not SERVED".
    PathResponsive,
    /// The websocket is gone — the path is dead.
    PathDead,
    /// The conductor answered, and its answer was that the cell is DISABLED.
    /// Bytes crossed the wire, so this is not a transport failure — but nothing
    /// can be written, so it is emphatically not evidence the path works.
    AppDisabled,
    /// Nothing was dispatched (admission shed): proves nothing either way.
    NoEvidence,
}

/// What one `app_info()` answer proves about the app behind the bridge.
///
/// Separate from [`ZomeObservation`] because the evidence is a STATUS, not an
/// error string: `app_info()` succeeds on a disabled app, and the status it
/// returns is the only field in that answer that can tell the two apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppRunObservation {
    /// The app is enabled — the path is live.
    Running,
    /// The app is NOT running. `reason` is the conductor's own account of why,
    /// rendered for a human: it is the diagnosis nothing was carrying.
    /// `enable` is what that same status says about whether `enable_app` could
    /// change anything.
    NotRunning {
        reason: String,
        enable: AppEnableEvidence,
    },
}

/// What this node's last `app_info` answer says about whether an `enable_app`
/// on that app could do ANYTHING.
///
/// Read straight off the persisted status, and the reason it is a separate
/// answer from "is it running" is that the two drive different decisions: the
/// running question drives whether to keep observing, and THIS one drives
/// whether to spend a mutation. It is deliberately independent of membership —
/// an unreadable running-cell map does not make a known-Enabled app enable-able,
/// and treating it as if it did is how a rung got spent on the fork's proven
/// no-op.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppEnableEvidence {
    /// The app's persisted status is `Enabled`, so `enable_app` returns `Ok`
    /// off the short circuit without checking, creating or retrying one cell.
    AlreadyEnabled,
    /// A status `enable_app` genuinely lifts: it loads ribosomes, awaits cell
    /// creation and persists `Enabled`, returning an error if creation fails.
    EnableCanLift,
    /// A status the conductor REFUSES to enable, or that enabling cannot cure:
    /// `AwaitingMemproofs` is rejected outright by this pin's `enable_app`, and
    /// `Unrecoverable` is terminal. Spending a rung here is not a bounded
    /// retry, it is a bounded mistake.
    EnableRefused,
    /// No `app_info` answer has been observed for this role yet.
    #[default]
    Unknown,
}

impl AppEnableEvidence {
    /// May `enable_app` be attempted on this evidence?
    ///
    /// `Unknown` answers `true` deliberately: before any status has been
    /// observed the pre-existing cure stands, because an absent observation is
    /// not a disproof. The two `false` arms are the ones the conductor's own
    /// source settles.
    pub fn enable_may_help(self) -> bool {
        match self {
            AppEnableEvidence::AlreadyEnabled | AppEnableEvidence::EnableRefused => false,
            AppEnableEvidence::EnableCanLift | AppEnableEvidence::Unknown => true,
        }
    }

    /// The wire/diagnostics spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            AppEnableEvidence::AlreadyEnabled => "already-enabled",
            AppEnableEvidence::EnableCanLift => "enable-can-lift",
            AppEnableEvidence::EnableRefused => "enable-refused",
            AppEnableEvidence::Unknown => "unknown",
        }
    }
}

/// Classify an installed app's status into what it proves about the path.
///
/// Only [`AppStatus::Enabled`] is running. Every other variant — disabled for
/// any reason, awaiting memproofs, awaiting restore, unrecoverable — means zome
/// calls will not land, and each carries its own reason forward verbatim.
///
/// The `enable` half is NOT "is it Enabled": `AwaitingMemproofs` and
/// `Unrecoverable` are both not-Enabled AND not enable-able, so a cure keyed on
/// the boolean would spend rungs on two statuses the conductor settles against.
pub fn classify_app_status(status: &holochain_types::app::AppStatus) -> AppRunObservation {
    use holochain_types::app::{AppStatus, DisabledAppReason};
    let (reason, enable) = match status {
        AppStatus::Enabled => return AppRunObservation::Running,
        AppStatus::Disabled(DisabledAppReason::NeverStarted) => (
            "disabled: never started since install".to_string(),
            AppEnableEvidence::EnableCanLift,
        ),
        AppStatus::Disabled(DisabledAppReason::NotStartedAfterProvidingMemproofs) => (
            "disabled: memproofs provided but the app was never started".to_string(),
            AppEnableEvidence::EnableCanLift,
        ),
        AppStatus::Disabled(DisabledAppReason::User) => (
            "disabled: by an operator through the admin interface".to_string(),
            AppEnableEvidence::EnableCanLift,
        ),
        AppStatus::Disabled(DisabledAppReason::Error(e)) => (
            format!("disabled: the conductor disabled it on an error: {e}"),
            AppEnableEvidence::EnableCanLift,
        ),
        AppStatus::AwaitingMemproofs => (
            "not running: awaiting memproofs — genesis has not completed, and this pin's \
             enable_app REJECTS this status outright"
                .to_string(),
            AppEnableEvidence::EnableRefused,
        ),
        AppStatus::AwaitingRestore => (
            "not running: awaiting restore — zome calls are rejected until every cell restores"
                .to_string(),
            AppEnableEvidence::EnableCanLift,
        ),
        AppStatus::Unrecoverable(cell_id, why) => (
            format!(
                "not running: UNRECOVERABLE on cell {cell_id:?} ({why:?}) — terminal, enable_app \
                 cannot lift it"
            ),
            AppEnableEvidence::EnableRefused,
        ),
    };
    AppRunObservation::NotRunning { reason, enable }
}

/// WHY a role is not serving, as this node has OBSERVED it — joined from the
/// two reads that between them say it: the app's persisted status and the
/// conductor's running-cell map.
///
/// This is the OBSERVATION vocabulary, not the action vocabulary. What to DO
/// about a state is decided by [`AppEnableEvidence`] (the mutation) and
/// [`Self::probe_is_owed_now`] (the read-only probe), because the same observed
/// state can license different actions: `installed-not-running-app-disabled`
/// covers both a status `enable_app` genuinely lifts and `AwaitingMemproofs`,
/// which this pin's `enable_app` rejects outright. Keeping the two vocabularies
/// separate is what stopped one word ("disabled") from covering a case
/// `enable_app` fixes and a case it provably cannot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleCellState {
    /// The cell is absent from the running map AND the app's persisted status
    /// is not `Enabled`. Whether `enable_app` can lift THAT status is a
    /// separate question — see [`AppEnableEvidence`].
    InstalledNotRunningAppDisabled,
    /// **STRANDED.** The cell is absent from the running map while the app's
    /// persisted status IS `Enabled` — so `enable_app` short-circuits on
    /// `if app.status == AppStatus::Enabled { return Ok(app.clone()) }` without
    /// checking, creating or retrying a single cell. Spending a ladder rung here
    /// buys nothing; the honest action is to keep observing membership and say
    /// so once.
    InstalledNotRunningAppEnabled,
    /// Membership says the cell IS in the running map, and this node has not yet
    /// proven it can serve.
    ///
    /// NAMED FOR WHAT IT ESTABLISHES, after the first cut asserted more than the
    /// observations do. Admission is not the obstacle; whether a call now
    /// RETURNS is simply unverified, and the probe is what settles it. The
    /// failure that opened the episode may predate admission entirely, so
    /// nothing here excludes a transport cure or names a cause.
    RunningRecoveryUnverified,
    /// The membership read itself failed, timed out, expired past its authority
    /// window, or was revoked by a bridge re-mint — so whether the cell is in
    /// the running map is NOT established. An absent answer must not be read as
    /// a disproof of anything.
    MembershipUnknown,
    /// Membership says NOT running and this node holds no app-status evidence
    /// for the role yet (a cross-cell role on the first tick, or a role whose
    /// `app_info` probe has never answered). Distinct from
    /// [`Self::MembershipUnknown`] because the halves that are missing are
    /// different halves — and naming it prevents the taxonomy from quietly
    /// calling a role stranded on no evidence.
    InstalledNotRunningStatusUnknown,
}

impl RoleCellState {
    /// The wire/metric spelling. Closed vocabulary — a label value, so it is a
    /// stable identifier and not a sentence.
    pub fn as_str(self) -> &'static str {
        match self {
            RoleCellState::InstalledNotRunningAppDisabled => "installed-not-running-app-disabled",
            RoleCellState::InstalledNotRunningAppEnabled => "installed-not-running-app-enabled",
            // NOT `running-but-call-failed`: a label that asserts a call failed
            // is an instrument asserting a cause it cannot see. The label says
            // what is established (membership present) and what is not
            // (recovery), and nothing else.
            RoleCellState::RunningRecoveryUnverified => "running-recovery-unverified",
            RoleCellState::MembershipUnknown => "membership-unknown",
            RoleCellState::InstalledNotRunningStatusUnknown => {
                "installed-not-running-status-unknown"
            }
        }
    }

    /// Is the read-only cell probe owed RIGHT NOW rather than on the ladder?
    ///
    /// Only where membership says the cell is running: that is the moment the
    /// episode can actually be ended, and making it wait for a rung is how a
    /// recovered role sat red for up to an hour. A probe against a cell
    /// membership says is ABSENT would fail by construction, so it is not asked
    /// there — but a cell whose membership is merely UNKNOWN still earns one on
    /// the ladder, because a quiet role has no other recovery evidence.
    pub fn probe_is_owed_now(self) -> bool {
        matches!(self, RoleCellState::RunningRecoveryUnverified)
    }

    /// Has membership PROVEN this role's cell absent from the running map?
    ///
    /// `true` only for the two states built on a definite `Some(false)`. The
    /// probe is skipped there (it would fail by construction) and nowhere else.
    pub fn membership_proves_absent(self) -> bool {
        matches!(
            self,
            RoleCellState::InstalledNotRunningAppDisabled
                | RoleCellState::InstalledNotRunningAppEnabled
                | RoleCellState::InstalledNotRunningStatusUnknown
        )
    }
}

/// Every state, for metric pre-touch. Absence is unalertable.
pub const ROLE_CELL_STATES: [RoleCellState; 5] = [
    RoleCellState::InstalledNotRunningAppDisabled,
    RoleCellState::InstalledNotRunningAppEnabled,
    RoleCellState::RunningRecoveryUnverified,
    RoleCellState::MembershipUnknown,
    RoleCellState::InstalledNotRunningStatusUnknown,
];

/// Join the two independent reads into ONE observed state.
///
/// Pure and total, so the whole decision table is a unit test with no conductor.
/// `membership_running` is
/// [`crate::services::cell_membership::MembershipCache::cell_running`] (`None` =
/// the read failed, timed out, expired, or was revoked); `enable` is the last
/// `app_info` status evidence for the role.
///
/// The state is an OBSERVATION. It does not decide the mutation — `enable`
/// does, on its own, whatever membership says. See
/// [`crate::hc_client_registry::decide_not_running_action`].
pub fn classify_cell_state(
    membership_running: Option<bool>,
    enable: AppEnableEvidence,
) -> RoleCellState {
    match (membership_running, enable) {
        (None, _) => RoleCellState::MembershipUnknown,
        (Some(true), _) => RoleCellState::RunningRecoveryUnverified,
        (Some(false), AppEnableEvidence::AlreadyEnabled) => {
            RoleCellState::InstalledNotRunningAppEnabled
        }
        (Some(false), AppEnableEvidence::EnableCanLift | AppEnableEvidence::EnableRefused) => {
            RoleCellState::InstalledNotRunningAppDisabled
        }
        (Some(false), AppEnableEvidence::Unknown) => {
            RoleCellState::InstalledNotRunningStatusUnknown
        }
    }
}

/// Does NEWER app-status evidence CONTRADICT a cached membership `true`?
///
/// Compared by OBSERVATION ORDER ([`next_obs_seq`]), never by epoch-ms: two
/// observations inside one millisecond compare equal, and a wall clock that
/// steps backwards inverts the relation outright. A `Disabled` status observed
/// after a clock rollback must still contradict a membership reading taken
/// before it.
///
/// The two halves of [`classify_cell_state`]'s join are read at different
/// times, and a cached `true` can be older than the status read that disagrees
/// with it. The conductor's own ordering settles the argument: `disable_app`
/// REMOVES an app's cells and awaits their cleanup BEFORE it writes the
/// `Disabled` status. So a persisted status of NOT-Enabled, observed after a
/// membership reading that said "running", proves that reading no longer
/// describes the conductor.
///
/// The demotion is to UNKNOWN, never to `Some(false)`: what is established is
/// that the cached answer has stopped being evidence, not that the cell is
/// absent. Unknown is what keeps the ladder running — `ProbeNow` off a stale
/// `true` suppressed the enable until the authority window expired, which the
/// 60s TTL bounds but does not cure.
///
/// `AlreadyEnabled` agrees with a running cell and contradicts nothing;
/// `Unknown` establishes nothing at all. Evidence OLDER than a proven zome-call
/// success cannot contradict anything either — [`record_role_success`] retires
/// it by restamping the role as `AlreadyEnabled` at the success's own
/// sequence, because a call that LANDED proves the cell is in the running map
/// and therefore that `enable_app` has nothing left to do.
pub fn status_contradicts_cached_membership(
    membership_running: Option<bool>,
    membership_seq: u64,
    enable: AppEnableEvidence,
    enable_seq: u64,
) -> bool {
    matches!(membership_running, Some(true))
        && enable_seq > membership_seq
        && matches!(
            enable,
            AppEnableEvidence::EnableCanLift | AppEnableEvidence::EnableRefused
        )
}

/// An immutable reading of [`BridgeHealth`], safe to serialize.
///
/// No longer `Copy`: it carries the disabled REASON, which is a `String`
/// because it is the conductor's own words and the whole point is not to lose
/// them. Every consumer already takes it by reference or by value-move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeHealthSnapshot {
    pub status: ZomePathStatus,
    /// Age of the last observed-live evidence, in whole seconds.
    pub last_success_age_secs: Option<u64>,
    /// Age of the last transport failure, in whole seconds.
    pub last_failure_age_secs: Option<u64>,
    /// Consecutive transport failures since the last live observation.
    pub consecutive_failures: u32,
    /// How many times the supervisor has re-minted the bridge.
    pub reconnects: u64,
    /// Why the app is not running, when [`ZomePathStatus::AppDisabled`] is the
    /// current verdict. `None` at every other status. THE missing diagnosis of
    /// the 2026-09-18 incident: the conductor knew, and nothing asked.
    pub app_disabled_reason: Option<String>,
    /// How long this role has been in its CURRENT not-running episode, in whole
    /// seconds. `None` at every status but [`ZomePathStatus::AppDisabled`].
    ///
    /// Measured from the observation that STARTED the episode, not from the
    /// most recent one — "not running for 11 minutes" and "not running as of a
    /// second ago" are the same fact stated at opposite ends, and only the
    /// first tells an operator whether to wait or to intervene.
    pub not_running_secs: Option<u64>,
}

impl BridgeHealthSnapshot {
    /// Can this node serve its truth-writing path right now?
    ///
    /// `Unknown` answers `true` deliberately: a node with no evidence has not
    /// earned a red. Observed death fails the probe — and so does an observed
    /// DISABLED app, which is a node that cannot write truth by any route.
    pub fn serving_ok(&self) -> bool {
        matches!(self.status, ZomePathStatus::Live | ZomePathStatus::Unknown)
    }
}

/// Observation counters for one node's conductor bridge.
///
/// Cheap enough to touch on the zome-call hot path: four relaxed atomics, no
/// lock, no allocation, no clock read on the success path beyond one
/// `SystemTime::now`.
/// ONE ROLE'S COMPLETE OBSERVATION RECORD.
///
/// Every field here can ARGUE with another field here, so they are read and
/// written TOGETHER, under one lock, and never as independent atomics.
///
/// ## Why atomics were not enough
///
/// The previous cut stored evidence-kind, evidence-sequence, success-sequence,
/// disabled-sequence, membership and episode as separate `Atomic*`s. `SeqCst`
/// orders each individual load and store; it does not make the TUPLE atomic,
/// and every defect the round-4 review modelled was a schedule that split one:
///
/// * a classifier reading `kind` from one observation and `seq` from a later
///   one, and then contradicting a membership reading with the fabricated pair
///   `(EnableCanLift, 12)` that no observation ever produced;
/// * a status writer that claimed a sequence, paused, and then overwrote a
///   NEWER success's `(AlreadyEnabled, 12)` with its own stale
///   `(EnableCanLift, 10)`, because no writer rejected an older sequence;
/// * an absence that passed the guarded check, released the guard, and then
///   opened its episode with a FRESH sequence — undoing a recovery that had
///   landed in between.
///
/// So there is exactly one mutation entry point ([`BridgeHealth::apply`]), it
/// rejects any observation older than the field it would overwrite, and it
/// performs the episode transition inside the same critical section, carrying
/// the observation's OWN sequence rather than minting a new one.
/// ## The invariant that makes "one mutation path" true
///
/// **Every field a classifier reads lives in this struct and is written only by
/// [`BridgeHealth::apply`].** Two members are deliberately outside that rule and
/// both are inert to classification:
///
/// * `diagnosis_said` — a log-dedup flag, claimed by
///   [`BridgeHealth::claim_diagnosis`]. Nothing reads it but the sentence it
///   guards.
/// * `cell_state` — written outside `apply` only by `note_cell_state` /
///   `clear_cell_state`, which are `#[cfg(test)]`. In a release build `apply` is
///   the sole writer.
///
/// Anything else added here must be written through `apply`, or the generation
/// checks that make this record coherent do not cover it.
#[derive(Debug, Clone, Default)]
pub struct RoleObservation {
    /// The app-status answer and the order it was observed in. ONE field, so a
    /// reader cannot pair one observation's kind with another's sequence.
    pub enable: AppEnableEvidence,
    pub enable_seq: u64,
    /// Epoch-ms of the status answer. Diagnostics only.
    pub enable_at_ms: u64,
    /// Order and wall stamp of the last PROVEN-LIVE observation — a zome call
    /// that returned FROM THIS ROLE'S CELL; 0 = never.
    ///
    /// This is the served-cell ordering barrier. Nothing else may write it.
    pub last_success_seq: u64,
    pub last_success_ms: u64,
    /// Order and wall stamp of the last RESPONSIVE observation — the conductor
    /// answered, but not from this role's cell; 0 = never.
    ///
    /// A SEPARATE STREAM, and that separation is load-bearing. While the two
    /// shared `last_success_seq`, a newer transport-responsive answer could
    /// REJECT a delayed genuine success (leaving its evidence unretired), and a
    /// responsive answer could outrank a real absence. Transport
    /// responsiveness is evidence about the socket; it may move the VERDICT off
    /// `Dead`, and it may not stand in for served-cell evidence in any
    /// ordering comparison.
    pub last_responsive_seq: u64,
    pub last_responsive_ms: u64,
    /// Order and wall stamp of the last transport failure; 0 = never.
    pub last_failure_seq: u64,
    pub last_failure_ms: u64,
    /// Order and wall stamp of the last not-running observation; 0 = never.
    pub last_disabled_seq: u64,
    pub last_disabled_ms: u64,
    /// The conductor's own words for the current not-running state.
    pub disabled_reason: Option<String>,
    /// Epoch-ms the CURRENT episode started; 0 = not in one.
    pub episode_started_ms: u64,
    /// Has the enabled-but-not-running diagnosis been said this episode?
    pub diagnosis_said: bool,
    pub consecutive_failures: u32,
    /// The last membership answer accepted for this role, with its own order.
    /// `None` = never observed; `Some((None, seq))` = observed as UNKNOWN.
    pub membership: Option<(Option<bool>, u64)>,
    /// The latched cell state, so a transition can be told from a re-observation.
    pub cell_state: Option<RoleCellState>,
    /// A verdict DERIVED from other observers, not observed here.
    ///
    /// Only the process-wide aggregate carries one. It is a REPLACEMENT, not
    /// evidence: [`status`](Self::status) returns it outright and the served-cell
    /// fold never sees it.
    ///
    /// Feeding the derived verdict back through that fold was a real defect: the
    /// aggregate recorded it as per-cell evidence, so once any role had opened a
    /// `Disabled` episode on the aggregate, a later derived `Dead` could not
    /// clear it (a transport observation may not end an episode — correctly, for
    /// a real role). The published aggregate then kept one role's obsolete
    /// `AppDisabled` and its stale reason while the derivation said `Dead`,
    /// including after that role had recovered.
    pub derived: Option<DerivedVerdict>,
}

/// A verdict computed from other observers and published as-is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedVerdict {
    pub verdict: ZomePathStatus,
    /// The diagnosis that came with it, when the verdict carries one.
    pub reason: Option<String>,
    /// Epoch-ms the aggregate first entered its CURRENT not-running verdict;
    /// 0 when it is not in one. Kept so `/health`'s `notRunningSecs` still
    /// renders for the aggregate.
    pub episode_started_ms: u64,
}

impl RoleObservation {
    /// The CURRENT verdict: three observation streams, latest wins, decided by
    /// ORDER. `0` means "never observed" and can never win.
    pub fn status(&self) -> ZomePathStatus {
        // A DERIVED VERDICT IS THE ANSWER, not an input to one.
        if let Some(derived) = &self.derived {
            return derived.verdict;
        }
        // TWO INDEPENDENT VERDICT STREAMS, and the served-cell one wins.
        //
        // * SERVED-CELL — `Success`, `Status`, `Disabled`, `Membership`. This is
        //   what the cures act on: is there an open not-running episode, and has
        //   a call landed since it opened?
        // * TRANSPORT — `Failure`, `Responsive`. This is what the socket is
        //   doing, and a responsive answer cancels an older failure.
        //
        // Competing all four stamps on one `max` made the verdict depend on
        // arrival order and let one stream answer the other's question: a
        // transport failure could outrank a not-running episode, and a
        // responsive answer could lift one. Splitting them removes both by
        // construction and needs no special case.
        if self.last_disabled_seq > self.last_success_seq {
            // An episode is OPEN. Nothing the transport does lifts it; only a
            // call that RETURNED from this role's cell can.
            return ZomePathStatus::AppDisabled;
        }
        // No episode is open. The transport verdict is `Dead` only while the
        // newest transport observation is a failure — a responsive answer
        // cancels an older one — and only while nothing served-cell is newer.
        if self.last_failure_seq > self.last_responsive_seq
            && self.last_failure_seq > self.last_success_seq
        {
            return ZomePathStatus::Dead;
        }
        if self.last_success_seq > 0 || self.last_responsive_seq > 0 {
            return ZomePathStatus::Live;
        }
        ZomePathStatus::Unknown
    }

    /// Is the role in a not-running episode right now?
    pub fn is_not_running(&self) -> bool {
        self.status() == ZomePathStatus::AppDisabled
    }

    /// Epoch-ms of the newest LIVE-CLASS observation (proven success or
    /// transport-responsive), for the rendered age; 0 = neither has happened.
    /// Rendering only — it orders nothing.
    pub fn last_live_ms(&self) -> u64 {
        if self.last_responsive_seq > self.last_success_seq {
            self.last_responsive_ms
        } else {
            self.last_success_ms
        }
    }
}

/// One thing observed about a role, carrying the ORDER it was observed in.
///
/// The only argument [`BridgeHealth::apply`] accepts. Every variant names a
/// distinct kind of evidence with a distinct cure, because collapsing them is
/// what let a domain error end a not-running episode.
#[derive(Debug, Clone)]
pub enum Observation {
    /// A zome call RETURNED. The ONE proof of recovery this node accepts.
    Success { seq: u64, at_ms: u64 },
    /// The conductor ANSWERED, but not from this role's cell. Never ends an
    /// episode; refused outright while one is open.
    Responsive { seq: u64, at_ms: u64 },
    /// The transport is gone.
    Failure { seq: u64, at_ms: u64 },
    /// The app is NOT RUNNING, in the conductor's own words, with NO status
    /// evidence attached.
    ///
    /// Retained for the ONE caller that genuinely has no `app_info` answer to
    /// carry: [`BridgeHealth::observe_zome_error`]'s `AppDisabled` arm, where a
    /// failing ZOME CALL is the whole observation. Every caller that holds an
    /// `app_info` answer uses [`Self::Status`] instead.
    Disabled {
        seq: u64,
        at_ms: u64,
        reason: String,
    },
    /// An `app_info` status answer: "could `enable_app` do anything?", with no
    /// health claim attached.
    EnableEvidence {
        seq: u64,
        at_ms: u64,
        kind: AppEnableEvidence,
    },
    /// ONE `app_info` answer, whole: the enable evidence it carries AND, when
    /// the app is not running, the conductor's own reason.
    ///
    /// THE POINT IS THAT IT IS ONE OBSERVATION. Publishing the evidence and the
    /// not-running verdict as two `apply`s let a success land between them, and
    /// the second half then reopened recovery under a FRESHLY MINTED sequence —
    /// the exact mechanism removed from the membership path, surviving here.
    /// Both halves now carry this observation's single sequence and are
    /// accepted or rejected together.
    Status {
        seq: u64,
        at_ms: u64,
        evidence: AppEnableEvidence,
        /// `Some(reason)` when the conductor says the app is NOT running.
        not_running: Option<String>,
    },
    /// A membership reading joined with the status evidence into one state.
    Membership {
        seq: u64,
        at_ms: u64,
        running: Option<bool>,
        state: RoleCellState,
    },
}

impl Observation {
    /// The order this observation was made in.
    pub fn seq(&self) -> u64 {
        match *self {
            Observation::Success { seq, .. }
            | Observation::Responsive { seq, .. }
            | Observation::Failure { seq, .. }
            | Observation::Disabled { seq, .. }
            | Observation::EnableEvidence { seq, .. }
            | Observation::Status { seq, .. }
            | Observation::Membership { seq, .. } => seq,
        }
    }

    /// A short label for the superseded-write metric and log line.
    pub fn kind_str(&self) -> &'static str {
        match self {
            Observation::Success { .. } => "success",
            Observation::Responsive { .. } => "responsive",
            Observation::Failure { .. } => "failure",
            Observation::Disabled { .. } => "disabled",
            Observation::EnableEvidence { .. } => "enable-evidence",
            Observation::Status { .. } => "status",
            Observation::Membership { .. } => "membership",
        }
    }
}

/// What one [`BridgeHealth::apply`] did, plus the COHERENT record afterwards.
///
/// `snapshot` is taken inside the same critical section as the mutation, so a
/// caller that classifies from it is classifying from a state that existed.
#[derive(Debug, Clone)]
pub struct ApplyResult {
    /// `false` when the observation was REJECTED as superseded.
    pub applied: bool,
    /// The observation's kind label and sequence, echoed so a caller (and the
    /// rejection counter) can name what was dropped without re-deriving it.
    pub kind: &'static str,
    pub seq: u64,
    /// When rejected, the sequence that outranked it.
    pub superseded_by: u64,
    /// The record as it stands after this call.
    pub snapshot: RoleObservation,
    /// The observation moved the role INTO a not-running episode.
    pub opened_episode: bool,
    /// The latched cell state CHANGED (the only thing worth a log line).
    pub cell_state_changed: bool,
    /// Set on an applied [`Observation::Success`].
    pub recovered_from_not_running: bool,
    /// THE GATE ON A SUCCESS'S SIDE EFFECTS.
    ///
    /// `true` only when the success outranked every health observation it would
    /// undo. A success that landed but was overtaken by a newer `Disabled` is
    /// still RECORDED — a call did land — but it must not raise the gauges or
    /// clear the enable ladder, because the world has moved on since.
    pub recovery_effects_apply: bool,
    /// Episode length in whole seconds, for the recovery line.
    pub not_running_secs: u64,
    /// The verdict this record held BEFORE the observation was folded in.
    /// Read from the same critical section, so it is the state the observation
    /// actually landed on.
    pub previous_status: ZomePathStatus,
}

impl ApplyResult {
    fn rejected(
        kind: &'static str,
        seq: u64,
        snapshot: RoleObservation,
        superseded_by: u64,
    ) -> Self {
        let previous_status = snapshot.status();
        Self {
            applied: false,
            kind,
            seq,
            superseded_by,
            snapshot,
            previous_status,
            opened_episode: false,
            cell_state_changed: false,
            recovered_from_not_running: false,
            recovery_effects_apply: false,
            not_running_secs: 0,
        }
    }
}

/// One role's observer.
///
/// Holds exactly two things: the coherent record behind ONE mutex, and a pure
/// counter that argues with nothing.
#[derive(Debug, Default)]
pub struct BridgeHealth {
    state: std::sync::Mutex<RoleObservation>,
    /// Serializes an `apply` with the GAUGE WRITES derived from its snapshot.
    ///
    /// A second lock rather than a wider one: `apply` must stay callable from
    /// anywhere, and the gauges are a separate consumer of the same decision.
    /// Lock order is always publish → state, and nothing takes them the other
    /// way round.
    publish: std::sync::Mutex<()>,
    reconnects: AtomicU64,
    /// (arrival ticket, minted sequence) for every observation that came
    /// through [`Self::apply_on_arrival`]. Both halves are claimed under the
    /// publication guard, so this log is exactly the guard's own order — which
    /// is what makes "arrival order IS sequence order" assertable rather than
    /// merely commented.
    #[cfg(test)]
    arrivals: std::sync::Mutex<Vec<(u64, u64)>>,
    #[cfg(test)]
    arrival_ticket: AtomicU64,
}

impl BridgeHealth {
    pub fn new() -> Self {
        Self::default()
    }

    /// Take the record's lock, tolerating poisoning: a panic elsewhere must not
    /// wedge every later observation shut.
    fn lock(&self) -> std::sync::MutexGuard<'_, RoleObservation> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// THE ONE MUTATION ENTRY POINT.
    ///
    /// Rejects any observation whose sequence is lower than the field it would
    /// overwrite, performs the episode transition in the same critical section,
    /// and hands back a coherent snapshot of the record afterwards.
    pub fn apply(&self, obs: Observation) -> ApplyResult {
        let result = self.apply_inner(obs.kind_str(), obs.seq(), obs);
        if !result.applied {
            // COUNTED FOR EVERY VARIANT, not just the ones whose wrappers
            // remembered to. A rejection is the mechanism working; a sustained
            // stream of them means two supervisors are fighting over one role
            // and the cure is upstream.
            note_superseded_write(result.kind, result.seq, result.superseded_by);
        }
        result
    }

    fn apply_inner(&self, kind: &'static str, seq: u64, obs: Observation) -> ApplyResult {
        let mut record = self.lock();
        // The verdict BEFORE this observation, read inside the critical section.
        let previous_status = record.status();
        let accept = |record: &RoleObservation| ApplyResult {
            applied: true,
            kind,
            seq,
            superseded_by: 0,
            snapshot: record.clone(),
            opened_episode: false,
            cell_state_changed: false,
            recovered_from_not_running: false,
            recovery_effects_apply: false,
            not_running_secs: 0,
            previous_status,
        };
        match obs {
            Observation::Success { at_ms, .. } => {
                if seq < record.last_success_seq {
                    return ApplyResult::rejected(
                        kind,
                        seq,
                        record.clone(),
                        record.last_success_seq,
                    );
                }
                // EVERY FIELD THIS OVERWRITES IS GENERATION-CHECKED, not just
                // the one that gates acceptance.
                //
                // A success can be claimed, pause before the lock, and resume
                // after a NEWER status answer has published `EnableCanLift` and
                // opened an episode. Checking only `last_success_seq` then let
                // it regress the evidence to `(AlreadyEnabled, older)`, zero the
                // episode clock and drop the conductor's reason — while the
                // verdict stayed `AppDisabled`. With membership absent, that
                // regressed evidence then chose `ObserveOnly` and stopped
                // enabling until a fresh status answer repaired it.
                //
                // A call DID land, so `last_success_seq` is recorded either way:
                // that is a fact about the world. What the success may not do is
                // undo observations that are NEWER than it.
                let outranks_disabled = seq > record.last_disabled_seq;
                let outranks_failure = seq > record.last_failure_seq;
                let outranks_evidence = seq > record.enable_seq;
                let outranks_membership = record.membership.is_none_or(|(_, at)| seq > at);
                // RECOVERY IS A SERVED-CELL FACT, SO ONLY THE SERVED-CELL
                // STREAM GATES IT.
                //
                // Gating it on the transport stream too meant a newer transport
                // failure — already cancelled by a newer responsive answer, and
                // therefore invisible in the verdict — silently suppressed the
                // recovery of a call that DID land: `Live` with
                // `app_enabled=0`, the family latched and the enable ladder
                // inherited by the next outage. A socket hiccup is not a reason
                // to disbelieve a call that returned from the cell.
                let recovery_effects_apply = outranks_disabled;

                let not_running_secs = if record.episode_started_ms > 0 {
                    at_ms.saturating_sub(record.episode_started_ms) / 1000
                } else {
                    0
                };
                record.last_success_seq = seq;
                record.last_success_ms = at_ms;
                if outranks_failure {
                    record.consecutive_failures = 0;
                }
                if outranks_disabled {
                    // The episode is over: forget when it started and re-arm the
                    // diagnosis so a LATER episode is diagnosed on its own terms.
                    record.episode_started_ms = 0;
                    record.diagnosis_said = false;
                    // A live path means the app is running; a stale reason would
                    // outlive the state it describes.
                    record.disabled_reason = None;
                    // The WHY-family answers "why is this role not serving"; a
                    // serving role has no such reason.
                    record.cell_state = None;
                }
                if outranks_evidence {
                    // RETIRE OLDER STATUS EVIDENCE, in the same breath. A call
                    // that LANDED proves the cell is in the running map, so a
                    // `Disabled` answer this success outlived can neither
                    // contradict membership nor license a rung.
                    record.enable = AppEnableEvidence::AlreadyEnabled;
                    record.enable_seq = seq;
                    record.enable_at_ms = at_ms;
                }
                if outranks_membership {
                    // AND RETIRE AN OLDER MEMBERSHIP ANSWER, for the same
                    // reason and with the same gate. A call that returned FROM
                    // THIS ROLE'S CELL is direct evidence the cell is in the
                    // conductor's running map — it is why
                    // `record_role_success` raises `cell_running` without
                    // waiting for the next `ListCellIds`. Leaving a stale
                    // `absent` in the record while the gauge said `1` let the
                    // two disagree purely on arrival order.
                    record.membership = Some((Some(true), seq));
                }
                ApplyResult {
                    recovered_from_not_running: recovery_effects_apply
                        && previous_status == ZomePathStatus::AppDisabled,
                    recovery_effects_apply,
                    not_running_secs,
                    ..accept(&record)
                }
            }
            Observation::Responsive { at_ms, .. } => {
                // RETAINED EVEN DURING AN OPEN EPISODE.
                //
                // It used to be refused there, on the reasoning that a cell
                // absent from `running_cells` still answers domain errors and
                // folding one as a success would end the episode. With the two
                // verdict streams separated it CANNOT end an episode — a
                // responsive answer never outranks `last_disabled_seq` — so
                // refusing it only threw away the evidence that cancels an older
                // transport failure. That loss was observable: `Disabled(9) →
                // Failure(10) → Responsive(11)` ended `AppDisabled`, while the
                // same three arriving as `Disabled(9) → Responsive(11) →
                // delayed Failure(10)` ended `Dead`, because the discarded
                // responsive answer was not there to cancel the failure.
                //
                // Ordered against its OWN stream. It must never reject a
                // delayed genuine success, and it must never stand in for one.
                if seq < record.last_responsive_seq {
                    return ApplyResult::rejected(
                        kind,
                        seq,
                        record.clone(),
                        record.last_responsive_seq,
                    );
                }
                record.last_responsive_seq = seq;
                record.last_responsive_ms = at_ms;
                if seq > record.last_failure_seq {
                    record.consecutive_failures = 0;
                }
                accept(&record)
            }
            Observation::Failure { at_ms, .. } => {
                if seq < record.last_failure_seq {
                    return ApplyResult::rejected(
                        kind,
                        seq,
                        record.clone(),
                        record.last_failure_seq,
                    );
                }
                record.last_failure_seq = seq;
                record.last_failure_ms = at_ms;
                record.consecutive_failures = record.consecutive_failures.saturating_add(1);
                accept(&record)
            }
            Observation::Disabled { at_ms, reason, .. } => {
                let applied = Self::fold_not_running(&mut record, seq, at_ms, &reason);
                match applied {
                    Some(opened_episode) => ApplyResult {
                        opened_episode,
                        ..accept(&record)
                    },
                    None => ApplyResult::rejected(
                        kind,
                        seq,
                        record.clone(),
                        record.last_disabled_seq.max(record.last_success_seq),
                    ),
                }
            }
            Observation::EnableEvidence {
                at_ms, kind: ev, ..
            } => {
                // THE TUPLE IS THE UNIT. A writer holding an older sequence
                // cannot overwrite a newer pair — the schedule that resurrected
                // `EnableCanLift` over a success's `AlreadyEnabled`.
                if seq < record.enable_seq {
                    return ApplyResult::rejected(kind, seq, record.clone(), record.enable_seq);
                }
                record.enable = ev;
                record.enable_seq = seq;
                record.enable_at_ms = at_ms;
                accept(&record)
            }
            Observation::Status {
                at_ms,
                evidence,
                not_running,
                ..
            } => {
                // ONE `app_info` ANSWER, ONE ORDERING DECISION.
                //
                // The evidence and the not-running verdict are two halves of the
                // same observation, so they are accepted or rejected together at
                // ONE sequence. Splitting them let a success land in between and
                // the second half reopen recovery under a fresh, higher
                // sequence — a stale answer outranking the recovery that
                // followed it.
                if seq < record.last_success_seq {
                    // A call has LANDED since this answer was taken. The WHOLE
                    // answer is stale — this is NEW-5's schedule, and rejecting
                    // it wholesale is the point: the half that used to survive
                    // reopened recovery under a freshly minted sequence.
                    return ApplyResult::rejected(
                        kind,
                        seq,
                        record.clone(),
                        record.last_success_seq,
                    );
                }
                // EACH HALF IS GENERATION-GATED ON ITS OWN FIELD, like
                // `Success`. Rejecting the whole answer because its EVIDENCE
                // half was outranked would discard a health claim nothing newer
                // contradicts — and made the verdict depend on whether an
                // unrelated `EnableEvidence` happened to arrive first.
                if seq > record.enable_seq {
                    record.enable = evidence;
                    record.enable_seq = seq;
                    record.enable_at_ms = at_ms;
                }
                let opened_episode = match not_running {
                    Some(reason) => {
                        Self::fold_not_running(&mut record, seq, at_ms, &reason).unwrap_or(false)
                    }
                    None => false,
                };
                ApplyResult {
                    opened_episode,
                    ..accept(&record)
                }
            }
            Observation::Membership {
                at_ms,
                running,
                state,
                ..
            } => {
                // Older than a recovery that already landed: the gauges recovery
                // set are the current truth and this reading describes a world
                // that has moved on.
                if seq < record.last_success_seq {
                    return ApplyResult::rejected(
                        kind,
                        seq,
                        record.clone(),
                        record.last_success_seq,
                    );
                }
                if let Some((_, previous)) = record.membership {
                    if seq < previous {
                        // A NEWER READING OWNS THE ANSWER — but an absence this
                        // node genuinely observed can still refine WHEN the open
                        // episode started. Lower-only: a superseded observation
                        // may sharpen an operator's duration, never open an
                        // episode of its own.
                        if state.membership_proves_absent() {
                            Self::lower_episode_clock(&mut record, seq, at_ms);
                        }
                        return ApplyResult::rejected(kind, seq, record.clone(), previous);
                    }
                }
                record.membership = Some((running, seq));

                // THE EPISODE TRANSITION, IN THIS CRITICAL SECTION, AT THIS
                // OBSERVATION'S OWN SEQUENCE. `ListCellIds` saying the cell is
                // GONE is failure evidence in its own right.
                let mut opened_episode = false;
                if state.membership_proves_absent() {
                    let reason = format!(
                        "membership: this role's cell is ABSENT from the conductor's running-cell \
                         map (AdminRequest::ListCellIds), observed state {}",
                        state.as_str()
                    );
                    if let Some(opened) = Self::fold_not_running(&mut record, seq, at_ms, &reason) {
                        opened_episode = opened;
                    }
                }

                // The WHY-family may only be re-opened on NEW FAILURE EVIDENCE.
                // A role that IS serving has no reason not to be.
                let serving = record.status() == ZomePathStatus::Live;
                let publish_family = !serving || state.membership_proves_absent();
                let cell_state_changed = publish_family && record.cell_state != Some(state);
                if publish_family {
                    record.cell_state = Some(state);
                }
                ApplyResult {
                    opened_episode,
                    cell_state_changed,
                    ..accept(&record)
                }
            }
        }
    }

    /// Fold a NOT-RUNNING observation into `record` at `seq`.
    ///
    /// `Some(opened_episode)` when it was accepted, `None` when a newer
    /// observation already owns the fields it would write. Shared by
    /// [`Observation::Disabled`] and [`Observation::Status`]'s not-running half
    /// so the two cannot drift, and so the second can never mint a sequence of
    /// its own.
    /// Lower an ALREADY-OPEN episode clock toward an earlier observation.
    ///
    /// Never opens one: an observation whose verdict half was superseded has
    /// nothing to say about whether an episode exists, only about when the one
    /// that does started.
    fn lower_episode_clock(record: &mut RoleObservation, seq: u64, at_ms: u64) {
        if seq < record.last_success_seq {
            return;
        }
        if record.episode_started_ms > 0 && at_ms < record.episode_started_ms {
            record.episode_started_ms = at_ms;
        }
    }

    fn fold_not_running(
        record: &mut RoleObservation,
        seq: u64,
        at_ms: u64,
        reason: &str,
    ) -> Option<bool> {
        // A call has landed since this observation: the whole thing is stale,
        // including its contribution to the clock.
        if seq < record.last_success_seq {
            return None;
        }
        // THE EPISODE CLOCK IS A MIN-FOLD over every not-running observation the
        // node holds since the last success — the EARLIEST one, whatever order
        // they arrived in.
        //
        // It starts once and runs until a call lands (restamping it on every
        // transition is how a transport blip inside an outage reported
        // `not_running_secs: 98` on a 108-minute episode). Taking the minimum
        // rather than only the first ARRIVAL is what makes that duration
        // order-independent: two observations of one outage racing must not be
        // able to shorten the number an operator reads.
        if record.episode_started_ms == 0 || at_ms < record.episode_started_ms {
            record.episode_started_ms = at_ms;
        }
        // The STAMP and the REASON belong to the newest not-running observation.
        // An older one has nothing left to say about them.
        if seq < record.last_disabled_seq {
            return None;
        }
        let opened_episode = !record.is_not_running();
        record.last_disabled_seq = seq;
        record.last_disabled_ms = at_ms;
        if opened_episode {
            record.diagnosis_said = false;
        }
        record.disabled_reason = Some(reason.to_string());
        Some(opened_episode)
    }

    #[cfg(test)]
    fn next_arrival_ticket(&self) -> u64 {
        self.arrival_ticket.fetch_add(1, Ordering::SeqCst) + 1
    }

    #[cfg(test)]
    fn note_arrival(&self, ticket: u64, seq: u64) {
        self.arrivals
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((ticket, seq));
    }

    /// The guard-ordered (ticket, sequence) log. Tests only.
    #[cfg(test)]
    pub fn arrivals(&self) -> Vec<(u64, u64)> {
        self.arrivals
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Take the publication lock, tolerating poisoning. Held across an `apply`
    /// and the gauge writes derived from its snapshot, so a success landing
    /// between them cannot be undone by a publish that had already decided.
    pub fn publish_guard(&self) -> std::sync::MutexGuard<'_, ()> {
        self.publish.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// REPLACE this observer's verdict with one derived elsewhere.
    ///
    /// For the process-wide aggregate only. It writes the verdict directly
    /// rather than feeding it back through the served-cell fold, because a
    /// derived verdict is not evidence about any cell — reinterpreting it as
    /// such is what let one role's obsolete `AppDisabled` outlive the
    /// derivation that had moved on.
    ///
    /// The episode stamp is carried so `/health`'s `notRunningSecs` still
    /// renders: it starts when the aggregate ENTERS a not-running verdict and
    /// clears when it leaves one.
    fn sync_from_roles(&self, roles: &RoleBridgeHealth, supervised: &[&str], at_ms: u64) {
        // Serialize derivation with publication: a delayed publisher must not
        // overwrite a newer aggregate with a verdict computed before recovery.
        let _publishing = self.publish_guard();
        let verdict = roles.derive_supervised_status(supervised);
        let reason = match verdict {
            ZomePathStatus::AppDisabled => Some(
                roles
                    .supervised_disabled_reason(supervised)
                    .unwrap_or_else(|| "the installed app is not running".to_string()),
            ),
            _ => None,
        };
        let mut record = self.lock();
        let was_not_running = record
            .derived
            .as_ref()
            .is_some_and(|d| d.verdict == ZomePathStatus::AppDisabled);
        let episode_started_ms = match (verdict, was_not_running) {
            (ZomePathStatus::AppDisabled, true) => record
                .derived
                .as_ref()
                .map(|d| d.episode_started_ms)
                .unwrap_or(at_ms),
            (ZomePathStatus::AppDisabled, false) => at_ms,
            _ => 0,
        };
        record.derived = Some(DerivedVerdict {
            verdict,
            reason,
            episode_started_ms,
        });
    }

    /// A COHERENT snapshot of the whole record — the only way to read more than
    /// one field. Every classifier takes this once and decides from it.
    pub fn observe(&self) -> RoleObservation {
        self.lock().clone()
    }

    /// The CURRENT verdict, without building a whole snapshot.
    pub fn status(&self) -> ZomePathStatus {
        self.lock().status()
    }

    /// ARRIVAL ORDER IS SEQUENCE ORDER, for every observation this process
    /// makes about itself.
    ///
    /// Mints the sequence INSIDE the publication guard, immediately before
    /// `apply`. The guard serializes, so two in-process observations can never
    /// be applied in a different order than they were sequenced — which makes
    /// every "older arrives after newer" schedule for these kinds unreachable
    /// by construction rather than by a rejection rule.
    ///
    /// [`Observation::Membership`] deliberately does NOT come through here: its
    /// sequence is claimed before the `ListCellIds` request goes out, because it
    /// is the one kind that is legitimately out of order (the answer describes
    /// the moment of the request, not of the reply). Its rejection rules are
    /// what order it.
    pub(crate) fn apply_on_arrival(&self, make: impl FnOnce(u64) -> Observation) -> ApplyResult {
        let publishing = self.publish_guard();
        self.mint_and_apply(&publishing, make)
    }

    /// THE ONE PLACE AN IN-PROCESS OBSERVATION IS SEQUENCED.
    ///
    /// Takes the caller's own publication guard by reference, so a publisher
    /// that must hold it across the gauge writes too — `record_role_success`,
    /// the `Disabled` publisher, the `Status` publisher — mints through the
    /// SAME seam instead of calling `next_obs_seq()` beside it. That is what
    /// makes "arrival order is sequence order" a property of the code rather
    /// than of five call sites that each remembered.
    ///
    /// The guard is unused except as proof the caller holds it; that is the
    /// point of taking it.
    pub(crate) fn mint_and_apply(
        &self,
        _publishing: &std::sync::MutexGuard<'_, ()>,
        make: impl FnOnce(u64) -> Observation,
    ) -> ApplyResult {
        // The ticket is claimed under the same guard, so a test can assert the
        // invariant directly rather than trusting the comment above.
        #[cfg(test)]
        let ticket = self.next_arrival_ticket();
        let result = self.apply(make(next_obs_seq()));
        #[cfg(test)]
        self.note_arrival(ticket, result.seq);
        result
    }

    /// Record evidence the path is live, stamped at `at_ms`.
    pub fn record_success_at(&self, at_ms: u64) {
        self.apply_on_arrival(|seq| Observation::Success { seq, at_ms });
    }

    /// Record evidence the APP IS NOT RUNNING, keeping the conductor's own
    /// `reason` verbatim. `true` on the transition INTO the episode.
    pub fn record_app_disabled_at(&self, at_ms: u64, reason: &str) -> bool {
        self.apply_on_arrival(|seq| Observation::Disabled {
            seq,
            at_ms,
            reason: reason.to_string(),
        })
        .opened_episode
    }

    /// [`Self::record_app_disabled_at`] against the wall clock.
    pub fn record_app_disabled(&self, reason: &str) -> bool {
        self.record_app_disabled_at(now_ms(), reason)
    }

    /// Claim the right to say, ONCE for this episode, that the conductor
    /// reports the app enabled while its cells are not running.
    ///
    /// NOT an [`Observation`], and that is deliberate: `diagnosis_said` is a
    /// LOG-DEDUP flag. No classifier reads it, no gauge derives from it, and no
    /// cure turns on it — it decides only whether one sentence is printed. It
    /// takes the record's lock (so it cannot tear against `apply`), and the
    /// episode transitions that clear it go through `apply` as they always did.
    /// See [`RoleObservation`]'s invariant note.
    pub fn claim_diagnosis(&self) -> bool {
        let mut record = self.lock();
        if !record.is_not_running() || record.diagnosis_said {
            return false;
        }
        record.diagnosis_said = true;
        true
    }

    /// Record evidence the conductor ANSWERED — the transport is responsive.
    /// `false` when REFUSED because the role is in a not-running episode.
    pub fn record_responsive_at(&self, at_ms: u64) -> bool {
        self.apply_on_arrival(|seq| Observation::Responsive { seq, at_ms })
            .applied
    }

    /// [`Self::record_responsive_at`] against the wall clock.
    pub fn record_responsive(&self) -> bool {
        self.record_responsive_at(now_ms())
    }

    /// Record evidence the path is dead, stamped at `at_ms`.
    pub fn record_failure_at(&self, at_ms: u64) {
        self.apply_on_arrival(|seq| Observation::Failure { seq, at_ms });
    }

    /// Record evidence the path is live, stamped now.
    pub fn record_success(&self) {
        self.record_success_at(now_ms());
    }

    /// Record evidence the path is dead, stamped now.
    pub fn record_failure(&self) {
        self.record_failure_at(now_ms());
    }

    /// Count one supervisor-driven re-mint of the bridge.
    pub fn record_reconnect(&self) {
        self.reconnects.fetch_add(1, Ordering::Relaxed);
    }

    /// Remember the conductor's last answer about this role's APP status.
    /// TEST-ONLY. Production reaches the evidence through
    /// [`Observation::Status`], which carries it with the health claim it came
    /// with and mints under the publication guard. This mints outside any
    /// guard, so compiling it out of the release build keeps the arrival-order
    /// invariant a property of the code.
    #[cfg(test)]
    pub fn note_enable_evidence(&self, evidence: AppEnableEvidence) {
        self.note_enable_evidence_stamped(evidence, now_ms(), next_obs_seq());
    }

    /// [`Self::note_enable_evidence`] stamped with the ORDER and the moment the
    /// status was observed. A stamp older than the stored pair is REJECTED.
    pub fn note_enable_evidence_stamped(&self, evidence: AppEnableEvidence, at_ms: u64, seq: u64) {
        // `apply` counts its own rejections for EVERY variant now, so this
        // wrapper does not (and must not) count a second time.
        self.apply(Observation::EnableEvidence {
            seq,
            at_ms,
            kind: evidence,
        });
    }

    /// The last app-status ENABLE evidence for this role.
    pub fn last_enable_evidence(&self) -> AppEnableEvidence {
        self.lock().enable
    }

    /// Epoch-ms the last [`AppEnableEvidence`] was observed; 0 = never.
    /// DIAGNOSTICS ONLY — ordering is [`Self::last_enable_evidence_seq`].
    pub fn last_enable_evidence_ms(&self) -> u64 {
        self.lock().enable_at_ms
    }

    /// Observation-order stamp of the last [`AppEnableEvidence`]; 0 = never.
    pub fn last_enable_evidence_seq(&self) -> u64 {
        self.lock().enable_seq
    }

    /// Observation-order stamp of the last proven-live call; 0 = never.
    pub fn last_success_seq(&self) -> u64 {
        self.lock().last_success_seq
    }

    /// Epoch-ms of the last observed-live evidence; 0 = never.
    pub fn last_success_ms(&self) -> u64 {
        self.lock().last_success_ms
    }

    /// Latch `state` as this role's current cell state. `true` iff it CHANGED.
    ///
    /// TEST-ONLY. The production path reaches the latch through
    /// [`Observation::Membership`], which decides the WHY-family gate and the
    /// episode transition in one critical section; compiling this out of the
    /// release build is what keeps "one mutation path" a fact rather than a
    /// claim.
    #[cfg(test)]
    pub fn note_cell_state(&self, state: RoleCellState) -> bool {
        let mut record = self.lock();
        let changed = record.cell_state != Some(state);
        record.cell_state = Some(state);
        changed
    }

    /// Forget the latched cell state. TEST-ONLY, for the same reason as
    /// [`Self::note_cell_state`] — a success clears it inside `apply`.
    #[cfg(test)]
    pub fn clear_cell_state(&self) {
        self.lock().cell_state = None;
    }

    /// Is a cell state currently latched? Observability and tests only.
    pub fn has_cell_state(&self) -> bool {
        self.lock().cell_state.is_some()
    }

    /// Fold the record into a status as of `now_ms`.
    pub fn snapshot_at(&self, now_ms: u64) -> BridgeHealthSnapshot {
        let record = self.observe();
        snapshot_of(&record, self.reconnects.load(Ordering::Relaxed), now_ms)
    }

    /// [`Self::snapshot_at`] against the wall clock.
    pub fn snapshot(&self) -> BridgeHealthSnapshot {
        self.snapshot_at(now_ms())
    }

    /// Fold a zome-call error into this observer, classifying it first.
    pub fn observe_zome_error(&self, msg: &str) {
        match classify_zome_error(msg) {
            ZomeObservation::PathResponsive => {
                self.record_responsive();
            }
            ZomeObservation::PathDead => self.record_failure(),
            ZomeObservation::AppDisabled => {
                self.record_app_disabled(msg);
            }
            ZomeObservation::NoEvidence => {}
        }
    }
}

/// Render a coherent record as the serializable snapshot `/health` carries.
///
/// A free function over the RECORD rather than a method on the observer, so a
/// caller that already holds a snapshot never re-reads the lock and never mixes
/// two observations into one rendering.
pub fn snapshot_of(record: &RoleObservation, reconnects: u64, now_ms: u64) -> BridgeHealthSnapshot {
    let status = record.status();
    if let Some(derived) = &record.derived {
        // A DERIVED verdict renders from itself. The per-cell streams below
        // describe observations this observer never made.
        return BridgeHealthSnapshot {
            status,
            last_success_age_secs: None,
            last_failure_age_secs: None,
            consecutive_failures: 0,
            reconnects,
            app_disabled_reason: (status == ZomePathStatus::AppDisabled)
                .then(|| derived.reason.clone())
                .flatten(),
            not_running_secs: (status != ZomePathStatus::Live && derived.episode_started_ms > 0)
                .then(|| now_ms.saturating_sub(derived.episode_started_ms) / 1000),
        };
    }
    let age = |stamp: u64| {
        if stamp == 0 {
            None
        } else {
            Some(now_ms.saturating_sub(stamp) / 1000)
        }
    };
    BridgeHealthSnapshot {
        status,
        last_success_age_secs: age(record.last_live_ms()),
        last_failure_age_secs: age(record.last_failure_ms),
        consecutive_failures: record.consecutive_failures,
        reconnects,
        // Only reported for the state it explains.
        app_disabled_reason: (status == ZomePathStatus::AppDisabled)
            .then(|| record.disabled_reason.clone())
            .flatten(),
        // Gated on `!= Live` rather than `== AppDisabled`: a transport blip
        // INSIDE a not-running episode moves the verdict to `Dead` for a tick,
        // and hiding the episode clock through it is the rendering half of the
        // same reset defect.
        not_running_secs: (status != ZomePathStatus::Live && record.episode_started_ms > 0)
            .then(|| now_ms.saturating_sub(record.episode_started_ms) / 1000),
    }
}

/// Count and name a write that arrived out of order and was DROPPED.
///
/// Superseded writes are not an error — they are the mechanism working — but
/// they must be countable, because a sustained stream of them means two
/// supervisors are fighting over one role and the cure is upstream.
fn note_superseded_write(kind: &str, seq: u64, superseded_by: u64) {
    crate::metrics::note_superseded_observation(kind);
    debug!(
        kind,
        seq,
        superseded_by,
        "observation DROPPED as superseded — a newer one already holds the field"
    );
}

/// The ONE process-wide observer the running node wires into `HcClient`, the
/// bridge supervisor and `/health`.
///
/// Deliberately a `OnceLock` singleton rather than an env-read or a threaded
/// handle: the zome-call site and the HTTP health site are ~30 call sites apart
/// and threading an `Arc` through all of them would be a signature change with
/// no truth-layer benefit. Tests never touch this — they build their own
/// [`BridgeHealth`] so parallel tests cannot poison one another.
///
/// Its `record_success`/`record_failure` calls are driven by
/// [`resync_aggregate`] rather than directly from each zome call: see
/// [`RoleBridgeHealth`] for why a raw interleaved event stream from more than
/// one role flaps.
pub fn bridge_health() -> &'static BridgeHealth {
    static HEALTH: OnceLock<BridgeHealth> = OnceLock::new();
    HEALTH.get_or_init(BridgeHealth::new)
}

/// Role-keyed registry of [`BridgeHealth`] observers.
///
/// WHY THIS EXISTS. `HcClient` instances are per-role (`infrastructure`,
/// `imagodei`, `lamad`, ...), but every one of them used to fold its
/// observations into the SAME global [`bridge_health`] singleton. One role
/// dying on its own cadence (e.g. a stray unsupervised client hitting a dead
/// bridge every 60s) and a completely different, healthy role succeeding on
/// its own cadence (e.g. the bridge supervisor's 20s ping) land in ONE shared
/// counter and alternate the reported verdict on whichever call happened to
/// land last — the observed `zomePath` live↔dead flap. Tracking each role's
/// evidence separately, and deriving the aggregate from every supervised
/// role's CURRENT state (never from raw event order across roles), is the
/// fix. See [`derive_supervised_status`] and `resync_aggregate`.
pub struct RoleBridgeHealth {
    roles: std::sync::RwLock<std::collections::HashMap<String, std::sync::Arc<BridgeHealth>>>,
}

impl RoleBridgeHealth {
    pub fn new() -> Self {
        Self {
            roles: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Get (or lazily create) the observer for `role`. Every `HcClient`
    /// instance funnels its observations here keyed by its OWN configured
    /// role, so a dead role's evidence never lands in another role's stream.
    pub fn for_role(&self, role: &str) -> std::sync::Arc<BridgeHealth> {
        if let Some(h) = self
            .roles
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(role)
        {
            return std::sync::Arc::clone(h);
        }
        let mut w = self.roles.write().unwrap_or_else(|e| e.into_inner());
        std::sync::Arc::clone(
            w.entry(role.to_string())
                .or_insert_with(|| std::sync::Arc::new(BridgeHealth::new())),
        )
    }

    /// Snapshot every role this instance has observed (or had created via
    /// [`Self::for_role`]), as of now. Order is unspecified (backed by a
    /// `HashMap`) — callers that need a stable order sort by key.
    pub fn snapshot_all(&self) -> Vec<(String, BridgeHealthSnapshot)> {
        let now = now_ms();
        self.roles
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|(role, h)| (role.clone(), h.snapshot_at(now)))
            .collect()
    }

    /// Derive the aggregate zome-path verdict from the CURRENT state of every
    /// role in `supervised_roles` — never from raw interleaved events.
    ///
    /// - **Dead** iff ANY supervised role is currently observed dead (one bad
    ///   role is enough to say the truth-writing path is compromised).
    /// - **Live** once at least one supervised role is live and none are
    ///   dead.
    /// - **Unknown** only while EVERY supervised role has no evidence yet
    ///   (fresh boot) — a node with no evidence has not earned a red, per
    ///   [`ZomePathStatus`]'s own contract.
    /// - **AppDisabled** sits between them: no role's websocket is gone, but at
    ///   least one role's app is not running, so the node cannot write truth
    ///   and the cure is an enable rather than a re-mint.
    pub fn derive_supervised_status(&self, supervised_roles: &[&str]) -> ZomePathStatus {
        let mut any_dead = false;
        let mut any_disabled = false;
        let mut any_live = false;
        for role in supervised_roles {
            match self.for_role(role).snapshot().status {
                ZomePathStatus::Dead => any_dead = true,
                ZomePathStatus::AppDisabled => any_disabled = true,
                ZomePathStatus::Live => any_live = true,
                ZomePathStatus::Unknown => {}
            }
        }
        if any_dead {
            ZomePathStatus::Dead
        } else if any_disabled {
            ZomePathStatus::AppDisabled
        } else if any_live {
            ZomePathStatus::Live
        } else {
            ZomePathStatus::Unknown
        }
    }

    /// The reason of the FIRST supervised role currently observed app-disabled,
    /// so the process-wide aggregate can carry a diagnosis rather than a bare
    /// verdict. `None` when no supervised role is disabled.
    pub fn supervised_disabled_reason(&self, supervised_roles: &[&str]) -> Option<String> {
        supervised_roles.iter().find_map(|role| {
            let snap = self.for_role(role).snapshot();
            snap.app_disabled_reason
                .map(|reason| format!("{role}: {reason}"))
        })
    }

    /// Fold PROVEN evidence that `role`'s zome path is live — a zome call on
    /// that role returned — and clear the enable ladder if this ended an
    /// episode.
    ///
    /// The ladder is cleared HERE and nowhere else. `enable_app` answering `Ok`
    /// and `app_info` answering `Enabled` are both compatible with a role whose
    /// cells are not running, so neither may buy the ladder a reset; a call
    /// that actually crossed into the cell is not.
    pub fn record_success_on(
        &self,
        role: &str,
        publishing: &std::sync::MutexGuard<'_, ()>,
        ledger: &EnableLedger,
        at_ms: u64,
    ) -> RecoveryOutcome {
        let observer = self.for_role(role);
        // ONE `apply`, so the "was it an episode / how long / retire the
        // evidence" decisions are taken from the state the success actually
        // landed on — and minted through the SAME seam every other in-process
        // observation uses, under the guard the caller already holds.
        let applied =
            observer.mint_and_apply(publishing, |seq| Observation::Success { seq, at_ms });
        // ONE ledger read on the steady-state path, and a WRITE only when there
        // is something to clear. Keying the clear on `before.status != Live`
        // ALONE was the stranded-backoff half of the 2026-09-20 defect: an
        // episode could climb the ladder, a transport failure could then read
        // `Dead`, a responsive domain error could read `Live`, and this branch
        // would never fire again — so the ladder the episode built outlived it
        // and the NEXT outage was met at the 1h cap. A ladder with attempts on
        // it is cleared by a call that landed, whatever the node believed a
        // millisecond earlier.
        let enable_attempts = ledger.attempts(role);
        // THE LADDER IS CLEARED ONLY BY A SUCCESS THAT STILL DESCRIBES THE
        // WORLD. A success overtaken by a newer `Disabled` is recorded — a call
        // did land — but clearing the ladder on it would hand the outage that
        // outranked it a fresh 60s rung for free.
        if applied.recovery_effects_apply
            && (enable_attempts > 0 || applied.previous_status != ZomePathStatus::Live)
        {
            ledger.note_running(role);
        }
        RecoveryOutcome {
            recovered_from_not_running: applied.recovered_from_not_running,
            not_running_secs: applied.not_running_secs,
            enable_attempts,
            effects_apply: applied.recovery_effects_apply,
        }
    }

    /// Fold evidence that `role`'s app is NOT RUNNING. `true` on the
    /// transition INTO the episode — the one observation that earns a WARN.
    pub fn record_app_disabled_on(&self, role: &str, reason: &str, at_ms: u64) -> bool {
        self.for_role(role).record_app_disabled_at(at_ms, reason)
    }

    /// May the "the app is enabled, its CELLS are not running" diagnosis be
    /// said for `role` right now? True at most once per episode, and never for
    /// a role that is not currently observed not-running.
    ///
    /// Records NOTHING. That is the point: the caller has just been told by the
    /// conductor that the app is enabled, and this module does not accept that
    /// as evidence of anything.
    pub fn note_app_enabled_but_not_running_on(&self, role: &str) -> bool {
        self.for_role(role).claim_diagnosis()
    }
}

/// What folding a proven-live observation into a role established.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryOutcome {
    /// Did the success outrank every health observation it would undo? When
    /// `false` the call landed but a NEWER outage already owns the record, and
    /// the caller must not publish recovery.
    pub effects_apply: bool,
    /// The role WAS not running and a zome call on it has now succeeded. The
    /// only transition this node calls a recovery.
    pub recovered_from_not_running: bool,
    /// How long the role was not running, in whole seconds. `0` when this was
    /// not a recovery.
    pub not_running_secs: u64,
    /// How many ladder rungs were spent on this role since it last served a
    /// call — an `enable_app` attempt for a role that has one, and the
    /// read-only cell probe that rides the same rung. `0` when the ladder was
    /// already clear, which is the steady-state answer.
    pub enable_attempts: u32,
}

impl Default for RoleBridgeHealth {
    fn default() -> Self {
        Self::new()
    }
}

/// The ONE process-wide [`RoleBridgeHealth`]. Every `HcClient`, keyed by its
/// own configured role, observes into here via [`observe_role_zome_error`] /
/// [`record_role_success`] / [`record_role_reconnect`]. `/health`'s `perRole`
/// block (see [`health_block`]) renders this directly.
pub fn role_bridge_health() -> &'static RoleBridgeHealth {
    static REG: OnceLock<RoleBridgeHealth> = OnceLock::new();
    REG.get_or_init(RoleBridgeHealth::new)
}

/// Re-derive the process-wide [`bridge_health`] singleton from every
/// supervised role's CURRENT state in [`role_bridge_health`]. Called after
/// every role-scoped observation so the top-level `zomePath` `/health` has
/// always carried stays in sync with the honest multi-role picture instead of
/// the raw interleaved event stream that used to flap. `Unknown` leaves the
/// singleton untouched (no evidence yet is not itself an observation).
fn resync_aggregate() {
    let roles = role_bridge_health();
    // OBSERVED, not SUPERVISED. `SUPERVISED_ROLES` is the set with a swappable
    // registry slot; `mishpat` is reached cross-cell through another role's
    // client and has no slot, but its cells refuse calls on exactly the same
    // startup window, so its verdict belongs in the aggregate.
    let supervised = &crate::hc_client_registry::OBSERVED_ROLES;
    // PUBLISHED DIRECTLY. The aggregate holds no cells of its own, so this
    // verdict is not evidence to be folded — it is the answer. Folding it meant
    // the aggregate could not follow its own derivation downward once any role
    // had opened an episode on it.
    bridge_health().sync_from_roles(roles, supervised, now_ms());
}

/// Fold a zome-call error into ROLE's own observer (for the per-role map),
/// classifying it first, then re-sync the process-wide aggregate. Call this
/// instead of `bridge_health().observe_zome_error(..)` directly from any
/// `HcClient` call site — the aggregate updates itself.
/// `role` here is the role that owns the CELL the call targeted, never the
/// calling client's configured role — see
/// [`crate::hc_client::target_role_for_cell`]. A mishpat `CellDisabled`
/// recorded against lamad marked the wrong role disabled AND let an ordinary
/// lamad success clear an episode that belonged to mishpat.
pub fn observe_role_zome_error(role: &str, msg: &str) {
    let observer = role_bridge_health().for_role(role);
    if classify_zome_error(msg) == ZomeObservation::AppDisabled {
        // Route through the transition-logging path so a disabled app is loud
        // ONCE rather than on every failing call — a busy node produces
        // thousands of these per minute.
        record_role_app_disabled(role, msg);
        return;
    }
    observer.observe_zome_error(msg);
    resync_aggregate();
}

/// Fold "the conductor answered" for ROLE without claiming its cell served the
/// call, then re-sync the aggregate.
///
/// Exposed for the probe and for tests; the ordinary path reaches it through
/// [`observe_role_zome_error`]. Returns what
/// [`BridgeHealth::record_responsive_at`] returned: `false` means the role is in
/// a not-running episode and the observation was refused.
pub fn record_role_responsive(role: &str) -> bool {
    let folded = role_bridge_health().for_role(role).record_responsive();
    resync_aggregate();
    folded
}

/// Record PROVEN evidence ROLE's path is live — a zome call on this role
/// SUCCEEDED — then re-sync the process-wide aggregate. Call this instead of
/// `bridge_health().record_success()` directly from any `HcClient` call site.
///
/// THE recovery transition, and the only one. Every path that ends a
/// not-running episode, raises the gauge, or clears the enable ladder comes
/// through HERE.
///
/// It is reached from the three `HcClient` zome-call paths — and from nothing
/// else. Not from the supervisor's `app_info` probe, not from an accepted
/// `enable_app`, and not from a FAILED call however the conductor phrased its
/// refusal ([`record_role_responsive`] is where those land). The supervisor's
/// read-only cell probe ([`crate::services::cell_probe`]) is not a second path:
/// it is an ordinary zome call, so a probe that returns arrives here exactly as
/// organic traffic does.
///
/// `role` is the role that owns the CELL the call landed on, never the calling
/// client's configured role.
pub fn record_role_success(role: &str) {
    let observer = role_bridge_health().for_role(role);
    // ONE serialized step: the publish guard is held across the `apply` AND the
    // gauge writes derived from its snapshot, so a membership publish cannot
    // read `last_success_seq`, lose the race, and write its outage gauges after
    // this one wrote recovery's.
    let outcome = {
        let _publishing = observer.publish_guard();
        let outcome =
            role_bridge_health().record_success_on(role, &_publishing, enable_ledger(), now_ms());
        if !outcome.effects_apply {
            // A call landed, and it is recorded — but a NEWER not-running
            // observation already owns this record. Publishing recovery here
            // would write gauges that contradict the accepted state.
            debug!(
                role,
                "a zome call returned but a NEWER not-running observation already holds this \
                 role's record — the call is recorded and no recovery is published"
            );
            // The aggregate is resynchronized EITHER WAY: `last_success_seq`
            // moved, and an early return that skipped this left the process-wide
            // verdict describing a record that had changed underneath it.
            resync_aggregate();
            return;
        }
        // The gauge is a LEVEL and this is the only place entitled to raise it:
        // it now means "a call has landed on this role", not "the conductor says
        // the app is enabled".
        crate::metrics::set_conductor_app_enabled(role, true);
        // A call LANDED, which is direct evidence the cell is in the conductor's
        // running map — so raise the membership gauge here too rather than
        // waiting for the next `ListCellIds`, and ZERO the whole cell-state
        // family. A stale level is worse than an absent one, because it is
        // believed. (`apply` already cleared the latch and retired the older
        // status evidence inside its own critical section.)
        crate::metrics::set_conductor_cell_running(role, Some(true));
        crate::metrics::clear_conductor_cell_state(role);
        outcome
    };
    if outcome.recovered_from_not_running {
        // The recovery line an operator greps for after a heal lands — now
        // carrying what it cost: how long the role was down, and how many
        // enable attempts were spent while it was.
        info!(
            role,
            not_running_secs = outcome.not_running_secs,
            enable_attempts = outcome.enable_attempts,
            "conductor app is RUNNING again — a zome call on this role SUCCEEDED, which is the \
             only proof of recovery this node accepts; the enable backoff is reset"
        );
    }
    resync_aggregate();
}

/// Say, at most ONCE per not-running episode, that the conductor reports this
/// app enabled while the role still refuses zome calls.
///
/// That sentence is the diagnosis an operator needs and nothing else produces
/// it: an enabled app whose CELLS are not running is indistinguishable from a
/// healthy one in `app_info`, in `list_apps`, and in `enable_app`'s answer. It
/// is only distinguishable by asking the cell — which is what every failing
/// zome call has already done.
///
/// Records nothing, resets nothing, and returns the role to no one: the state
/// machine is unchanged by a conductor's opinion of itself.
pub fn note_app_enabled_but_not_running(role: &str) {
    if role_bridge_health().note_app_enabled_but_not_running_on(role) {
        info!(
            role,
            "the conductor reports this app ENABLED while every zome call on the role is still \
             refused — the app is enabled and its CELLS are not running. enable_app cannot lift \
             this; waiting on the conductor. Recovery will be claimed only when a zome call on \
             this role succeeds."
        );
    }
}

/// Is ROLE currently observed NOT RUNNING? The supervisor's read, so a heal
/// can be paced by what this node has actually observed rather than by what
/// the conductor says about itself.
pub fn role_is_not_running(role: &str) -> bool {
    role_bridge_health().for_role(role).status() == ZomePathStatus::AppDisabled
}

/// Record that ROLE's app is NOT RUNNING, with the conductor's own `reason`.
///
/// WARNs **once per transition**, not once per observation: the disabled state
/// is discovered by every failing zome call and by every supervisor probe, so
/// logging per observation would bury the one line that matters under thousands
/// of copies of itself. The `elohim_conductor_app_enabled{role}` gauge is set on
/// every call, because a gauge is a level and a level must not be edge-driven.
pub fn record_role_app_disabled(role: &str, reason: &str) {
    // ONE SERIALIZED STEP, like every other publication, and the sequence is
    // minted through `mint_and_apply` INSIDE it — so this observation cannot be
    // sequenced before another in-process observation and then applied after it.
    let observer = role_bridge_health().for_role(role);
    let publishing = observer.publish_guard();
    publish_role_app_disabled(role, &observer, &publishing, reason, None);
}

/// [`record_role_app_disabled`] at an explicit sequence.
///
/// Extracted so the ONE schedule an unguarded gauge write admitted is drivable:
/// a `Disabled` answer that claimed its sequence, paused, and resumed after a
/// success had published recovery. Entering here with the older sequence IS that
/// answer resuming.
pub fn record_role_app_disabled_at_seq(role: &str, reason: &str, seq: u64) {
    let observer = role_bridge_health().for_role(role);
    let publishing = observer.publish_guard();
    publish_role_app_disabled(role, &observer, &publishing, reason, Some(seq));
}

/// Apply a not-running observation and publish its gauge, with the caller
/// holding `role`'s publication guard.
fn publish_role_app_disabled(
    role: &str,
    observer: &std::sync::Arc<BridgeHealth>,
    publishing: &std::sync::MutexGuard<'_, ()>,
    reason: &str,
    replay_seq: Option<u64>,
) {
    let at_ms = now_ms();
    let applied = {
        // `replay_seq` is the deliberate exception: the one test that drives a
        // stale answer resuming needs to name its sequence. Everything else
        // mints through the seam.
        let applied = match replay_seq {
            Some(seq) => observer.apply(Observation::Disabled {
                seq,
                at_ms,
                reason: reason.to_string(),
            }),
            None => observer.mint_and_apply(publishing, |seq| Observation::Disabled {
                seq,
                at_ms,
                reason: reason.to_string(),
            }),
        };
        if applied.applied {
            crate::metrics::set_conductor_app_enabled(role, false);
        }
        applied
    };
    if applied.applied && applied.opened_episode {
        warn!(
            role,
            reason,
            "conductor app is NOT RUNNING — every zome call on this role will fail while it \
             stays this way, and the node's own projection reads will keep looking healthy. \
             Storage will attempt enable_app on a bounded backoff (60s doubling to a 1h cap); \
             the next line about this role will be the recovery, when a zome call succeeds."
        );
    }
    if applied.applied {
        resync_aggregate();
    }
}

/// Fold one `app_info()` probe answer into ROLE's observer.
///
/// ASYMMETRIC ON PURPOSE. `NotRunning` is recorded, because a conductor saying
/// its own app is not running can only understate this node's health. `Running`
/// records NOTHING about HEALTH — `app_info` answers from the app record, and
/// the app is enabled for the whole of a startup (or a stall) in which its
/// cells are absent from `running_cells` and every zome call answers
/// `CellDisabled`. Recording it as health was how the verdict flapped every
/// ~20s while the truth-writing path stayed dead.
///
/// It DOES record the [`AppEnableEvidence`] either way, because that is not a
/// health claim: it is the answer to "could enable_app do anything", and
/// `Enabled` answers that question with a definite no.
pub fn observe_role_app_status(role: &str, observation: &AppRunObservation) {
    let (evidence, not_running) = match observation {
        AppRunObservation::Running => (AppEnableEvidence::AlreadyEnabled, None),
        AppRunObservation::NotRunning { enable, reason } => (*enable, Some(reason.clone())),
    };
    let observer = role_bridge_health().for_role(role);
    // ONE OBSERVATION, ONE SEQUENCE, ONE APPLY.
    //
    // This used to be two: `note_enable_evidence` then `record_role_app_disabled`,
    // each minting its own sequence. A zome success landing between them was
    // then UNDONE by the second half under a freshly minted, higher sequence —
    // a stale `app_info` answer outranking the recovery that followed it. The
    // two halves are one answer and they are now accepted or rejected together.
    let applied = {
        let publishing = observer.publish_guard();
        let at_ms = now_ms();
        let applied = observer.mint_and_apply(&publishing, |seq| Observation::Status {
            seq,
            at_ms,
            evidence,
            not_running: not_running.clone(),
        });
        if applied.applied && not_running.is_some() {
            crate::metrics::set_conductor_app_enabled(role, false);
        }
        applied
    };
    if !applied.applied {
        return;
    }
    match observation {
        AppRunObservation::Running => note_app_enabled_but_not_running(role),
        AppRunObservation::NotRunning { reason, .. } => {
            if applied.opened_episode {
                warn!(
                    role,
                    reason = reason.as_str(),
                    "conductor app is NOT RUNNING — every zome call on this role will fail while \
                     it stays this way, and the node's own projection reads will keep looking \
                     healthy. Storage will attempt enable_app on a bounded backoff (60s doubling \
                     to a 1h cap); the next line about this role will be the recovery, when a \
                     zome call succeeds."
                );
            }
            resync_aggregate();
        }
    }
}

/// The last [`AppEnableEvidence`] this node has for ROLE.
pub fn last_enable_evidence(role: &str) -> AppEnableEvidence {
    role_bridge_health().for_role(role).last_enable_evidence()
}

/// Epoch-ms ROLE's last [`AppEnableEvidence`] was observed; 0 = never.
/// DIAGNOSTICS ONLY — ordering is [`last_enable_evidence_seq`].
pub fn last_enable_evidence_ms(role: &str) -> u64 {
    role_bridge_health()
        .for_role(role)
        .last_enable_evidence_ms()
}

/// Observation ORDER of ROLE's last [`AppEnableEvidence`]; 0 = never.
pub fn last_enable_evidence_seq(role: &str) -> u64 {
    role_bridge_health()
        .for_role(role)
        .last_enable_evidence_seq()
}

/// Observation ORDER of ROLE's last proven-live call; 0 = never.
pub fn last_success_seq(role: &str) -> u64 {
    role_bridge_health().for_role(role).last_success_seq()
}

/// Epoch-ms of ROLE's last proven-live observation; 0 = never.
pub fn last_success_ms(role: &str) -> u64 {
    role_bridge_health().for_role(role).last_success_ms()
}

/// Publish ROLE's observed cell state — one structured state, logged only on a
/// TRANSITION, with the MEMBERSHIP gauge set on every observation because a
/// gauge is a level.
///
/// `observed_seq` is the [`next_obs_seq`] stamp of the MEMBERSHIP READING this
/// state was derived from, not of the publish. A publish whose observation
/// precedes the role's last proven-live call is DROPPED: an observation in
/// flight while a zome call landed would otherwise republish the outage it was
/// measuring and leave a recovered role's gauges reading red. Recovery is the
/// one transition; an older observation does not get to undo it. The check and
/// the writes are ONE serialized step (see [`BridgeHealth::publish_guard`]) —
/// a success landing between them would otherwise slip through a check that had
/// already passed.
///
/// ORDER, NOT MILLISECONDS. The previous cut compared epoch-ms with a strict
/// `>`, so an absence observed at ms 1000 and a success landing later within
/// that same millisecond compared EQUAL and the absence won; a backward clock
/// step defeated it outright. Sequences are minted from one process-wide
/// counter, so ties are impossible by construction and a clock correction
/// changes nothing.
///
/// ## Membership publishes always; the WHY-family does not
///
/// `elohim_conductor_cell_running` is this node's membership level and is
/// published on every observation, gated on nothing — that is the whole of the
/// gauge-stuck-at-1 cure.
///
/// `elohim_conductor_cell_state` is a different instrument: it answers "WHY is
/// this role not serving". A role that IS serving has no such reason, which is
/// why recovery zeroes the family. So the per-tick publish may only re-open it
/// on NEW FAILURE EVIDENCE — a membership reading that PROVES the cell absent,
/// or a role that is not currently serving. Publishing it unconditionally
/// re-opened `running-recovery-unverified=1` on a recovered, quiet role the
/// very next tick, where — with no episode open — no probe ever follows to
/// settle it, and the log line promising one is written into an empty room.
///
/// Returns `true` when the state changed AND was published.
pub fn record_role_cell_state(
    role: &str,
    state: RoleCellState,
    membership: Option<bool>,
    enable: AppEnableEvidence,
    observed_seq: u64,
    observed_at_ms: u64,
) -> bool {
    let observer = role_bridge_health().for_role(role);
    // ONE serialized step: `apply` (which decides supersession, the episode
    // transition and the WHY-family gate in a single critical section) plus the
    // gauge writes derived from ITS snapshot. Nothing is deferred and nothing
    // is re-read.
    let applied = {
        let _publishing = observer.publish_guard();
        let applied = observer.apply(Observation::Membership {
            seq: observed_seq,
            // THE ANSWER'S OWN OBSERVATION TIME, not the moment of publication.
            // The episode clock is a min-fold over not-running observations, so
            // handing it `now` meant it reconstructed the moment this node got
            // around to publishing rather than the moment the conductor's
            // running-cell map was actually read — and `not_running_secs` is an
            // operator-facing duration.
            at_ms: observed_at_ms,
            running: membership,
            state,
        });
        if applied.applied {
            crate::metrics::set_conductor_cell_running(role, membership);
            if applied.snapshot.cell_state == Some(state) {
                crate::metrics::set_conductor_cell_state(role, state);
            }
            if state.membership_proves_absent() {
                // MEMBERSHIP ESTABLISHES THE OUTAGE, so it lowers the same
                // gauge a `Disabled` answer would. Folding the deferred
                // `record_role_app_disabled` call into `apply` moved the episode
                // transition inside the lock and dropped this write with it —
                // leaving `AppDisabled, cell_running=0, app_enabled=1`, which is
                // three gauges disagreeing about one role.
                crate::metrics::set_conductor_app_enabled(role, false);
            }
        }
        applied
    };
    if !applied.applied {
        return false;
    }
    if applied.opened_episode {
        // Logged OUTSIDE the critical section; the transition itself already
        // happened inside it, at THIS observation's sequence.
        warn!(
            role,
            state = state.as_str(),
            reason = applied.snapshot.disabled_reason.as_deref().unwrap_or(""),
            "conductor app is NOT RUNNING — membership proves this role's cell is ABSENT from \
             the conductor's running-cell map. Storage will attempt enable_app on a bounded \
             backoff where the status allows it, and will probe the cell the moment membership \
             says it is running; the next line about this role will be the recovery, when a zome \
             call succeeds."
        );
        resync_aggregate();
    }
    if !applied.cell_state_changed {
        return false;
    }
    match state {
        // The state says the status is not Enabled; the EVIDENCE says whether
        // enabling it is a cure. Conflating them is what let "enable is the
        // right cure" be printed over AwaitingMemproofs, which this pin's
        // enable_app rejects outright.
        RoleCellState::InstalledNotRunningAppDisabled if enable.enable_may_help() => warn!(
            role,
            state = state.as_str(),
            enable = enable.as_str(),
            "this role's cell is ABSENT from the conductor's running-cell map and its app's \
             persisted status is one enable_app genuinely lifts — it will be attempted on the \
             bounded ladder"
        ),
        RoleCellState::InstalledNotRunningAppDisabled => warn!(
            role,
            state = state.as_str(),
            enable = enable.as_str(),
            "this role's cell is ABSENT from the conductor's running-cell map and its app's \
             persisted status is one this conductor REFUSES to enable (AwaitingMemproofs is \
             rejected outright; Unrecoverable is terminal) — no ladder rung will be spent. This \
             needs an operator, not a retry."
        ),
        RoleCellState::InstalledNotRunningAppEnabled => warn!(
            role,
            state = state.as_str(),
            enable = enable.as_str(),
            "STRANDED: this role's cell is ABSENT from the conductor's running-cell map while its \
             app's persisted status IS Enabled. enable_app short-circuits on an already-Enabled \
             app without checking, creating or retrying a single cell, so it CANNOT lift this — no \
             ladder rung will be spent on it. Storage keeps watching ListCellIds and will probe \
             the cell the moment membership says it is running. Cure is in the conductor (finish \
             startup, finish teardown, or a scoped disable→enable)."
        ),
        // SAYS ONLY WHAT IS ESTABLISHED.
        RoleCellState::RunningRecoveryUnverified => info!(
            role,
            state = state.as_str(),
            "membership present; functional recovery unverified — this role's cell IS in the \
             conductor's running-cell map, and whether a call now returns is what the read-only \
             probe is about to settle. No rung is spent and no cause is claimed."
        ),
        RoleCellState::MembershipUnknown => warn!(
            role,
            state = state.as_str(),
            enable = enable.as_str(),
            "the ListCellIds membership read is not established (failed, timed out, expired past \
             its authority window, or revoked by a bridge re-mint), so whether this role's cell is \
             running is NOT known. The app-status evidence still decides the mutation on its own \
             — an absent observation is not a disproof, and it is not a licence either."
        ),
        RoleCellState::InstalledNotRunningStatusUnknown => info!(
            role,
            state = state.as_str(),
            "this role's cell is ABSENT from the conductor's running-cell map and this node holds \
             no app-status evidence for it yet — the pre-existing cure stands until the next \
             app_info answer says otherwise"
        ),
    }
    true
}

/// Count one supervisor-driven re-mint of ROLE's bridge, in both the
/// per-role map and the process-wide total the top-level `bridgeReconnects`
/// field has always carried.
pub fn record_role_reconnect(role: &str) {
    role_bridge_health().for_role(role).record_reconnect();
    bridge_health().record_reconnect();
}

/// Does this error text mean the WEBSOCKET is gone (as opposed to the conductor
/// answering "no")?
///
/// Matched case-insensitively on substrings because the message reaching us is
/// already a `format!`-composed chain: `StorageError::Conductor("Zome call
/// failed: {holochain_client error}")`. Classifying on the string is the same
/// contract `conductor_admission::is_admission_shed` uses, for the same reason:
/// the underlying crate does not expose a transport-vs-domain discriminant.
pub fn is_transport_dead(msg: &str) -> bool {
    let m = msg.to_ascii_lowercase();
    const DEAD_MARKERS: [&str; 7] = [
        "websocket closed",
        "no connection",
        "connection reset",
        "connection refused",
        "connection closed",
        "broken pipe",
        "connect failed",
    ];
    DEAD_MARKERS.iter().any(|marker| m.contains(marker))
}

/// Does this error text mean the conductor ANSWERED, and its answer was that
/// the cell (and therefore the app) is not running?
///
/// Same closed-marker-list contract as [`is_transport_dead`], for the same
/// reason: the underlying crate hands us a formatted `ConductorApiError`, not a
/// discriminant. The first marker is verbatim from the 2026-09-18 incident —
/// `Conductor returned an error while using a ConductorApi: CellDisabled(CellId(
/// DnaHash(…), AgentPubKey(…)))` — which every supervised role answered with,
/// on three pods, for 38 hours, while `/health` said `live`.
pub fn is_cell_disabled(msg: &str) -> bool {
    let m = msg.to_ascii_lowercase();
    const DISABLED_MARKERS: [&str; 3] = ["celldisabled", "cell is disabled", "app is disabled"];
    DISABLED_MARKERS.iter().any(|marker| m.contains(marker))
}

/// Classify one zome-call error into what it proves about the path.
///
/// Order is load-bearing and deliberately conservative. The shed check stays
/// first (nothing was dispatched, so nothing is proven). Transport-death stays
/// SECOND, ahead of the disabled check, so the pre-existing classification of
/// every string that already read as dead is preserved byte-for-byte — a
/// message that somehow named both would keep its older, more urgent verdict.
/// The observed `CellDisabled` text carries no transport marker, so it lands in
/// the arm that exists for it.
///
/// The fall-through is [`ZomeObservation::PathResponsive`] and NOT a claim of
/// recovery. That distinction is enforced downstream rather than here, on
/// purpose: adding a `ZomeNotFound`/`FunctionNotFound` marker list would be a
/// second closed string vocabulary to keep in sync, and it would still leave
/// every unrecognised phrasing able to end an episode. Making the whole
/// responsive CLASS unable to end one closes the hole for strings nobody has
/// seen yet — including a probe pointed at a function the DNA does not have.
pub fn classify_zome_error(msg: &str) -> ZomeObservation {
    if msg.contains(crate::conductor_admission::ADMISSION_SHED_MARKER) {
        return ZomeObservation::NoEvidence;
    }
    if is_transport_dead(msg) {
        return ZomeObservation::PathDead;
    }
    if is_cell_disabled(msg) {
        return ZomeObservation::AppDisabled;
    }
    ZomeObservation::PathResponsive
}

/// The wire shape of one role's entry in the `perRole` map — same field
/// names as the top-level block, minus `mode` (a process-level fact, not a
/// per-role one).
fn role_status_json(snap: &BridgeHealthSnapshot) -> serde_json::Value {
    serde_json::json!({
        "zomePath": snap.status.as_str(),
        "lastZomeCallAgeSecs": snap.last_success_age_secs,
        "lastZomeFailureAgeSecs": snap.last_failure_age_secs,
        "consecutiveFailures": snap.consecutive_failures,
        "bridgeReconnects": snap.reconnects,
        // ADDITIVE: null at every status but `app-disabled`. The field an
        // operator reads to learn WHY, without opening a conductor database.
        "appDisabledReason": snap.app_disabled_reason,
        // ADDITIVE: how long this episode has lasted. A conductor that is still
        // starting and one that is stuck look identical in every other field;
        // they differ only in how long this number has been growing.
        "notRunningSecs": snap.not_running_secs,
    })
}

/// Build the `perRole` object from an explicit [`RoleBridgeHealth`] — factored
/// out of [`health_block`] so "one dead role among N" is a unit test against
/// an isolated registry, never the process-wide singleton (parallel tests
/// must never share that mutable global — see [`bridge_health`]'s doc).
pub fn per_role_block(roles: &RoleBridgeHealth) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (role, snap) in roles.snapshot_all() {
        map.insert(role, role_status_json(&snap));
    }
    serde_json::Value::Object(map)
}

/// The `conductor` block `/health` and `/health/serving` both render.
///
/// One builder, two surfaces — so the body a person reads and the status code a
/// probe reads can never disagree about the same node (the drift that let
/// `/health` say `ok` while every read 503'd).
///
/// `zomePath` and the other top-level scalars come from `snap` (typically
/// [`bridge_health`]'s singleton, kept in sync with the honest multi-role
/// picture by `resync_aggregate` on every role-scoped observation — see
/// [`RoleBridgeHealth`]). `perRole` is ADDED beside them: a role → status map
/// so a reader can see WHICH role is dead instead of only that something is.
pub fn health_block(mode: &str, snap: &BridgeHealthSnapshot) -> serde_json::Value {
    serde_json::json!({
        "mode": mode,
        "zomePath": snap.status.as_str(),
        "lastZomeCallAgeSecs": snap.last_success_age_secs,
        "lastZomeFailureAgeSecs": snap.last_failure_age_secs,
        "consecutiveFailures": snap.consecutive_failures,
        "bridgeReconnects": snap.reconnects,
        "appDisabledReason": snap.app_disabled_reason,
        "notRunningSecs": snap.not_running_secs,
        "perRole": per_role_block(role_bridge_health()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::enable_app_backoff::enable_backoff;
    use std::time::{Duration, Instant};

    const T0: u64 = 1_700_000_000_000;

    #[test]
    fn fresh_observer_is_unknown_and_serves() {
        let h = BridgeHealth::new();
        let snap = h.snapshot_at(T0);
        assert_eq!(snap.status, ZomePathStatus::Unknown);
        assert_eq!(snap.last_success_age_secs, None);
        assert_eq!(snap.last_failure_age_secs, None);
        // A node with no evidence has not earned a red.
        assert!(snap.serving_ok());
    }

    #[test]
    fn aggregate_publication_replaces_an_obsolete_disabled_episode() {
        let roles = RoleBridgeHealth::new();
        let aggregate = BridgeHealth::new();
        let supervised = ["lamad", "imagodei"];
        roles
            .for_role("lamad")
            .record_app_disabled_at(T0, "cell missing");
        aggregate.sync_from_roles(&roles, &supervised, T0);
        let disabled = aggregate.snapshot_at(T0 + 2_000);
        assert_eq!(disabled.status, ZomePathStatus::AppDisabled);
        assert_eq!(disabled.not_running_secs, Some(2));
        assert!(disabled.app_disabled_reason.is_some());

        roles.for_role("imagodei").record_failure_at(T0 + 3_000);
        aggregate.sync_from_roles(&roles, &supervised, T0 + 3_000);
        assert_eq!(aggregate.status(), ZomePathStatus::Dead);

        roles.for_role("lamad").record_success_at(T0 + 4_000);
        aggregate.sync_from_roles(&roles, &supervised, T0 + 4_000);
        let dead = aggregate.snapshot_at(T0 + 5_000);
        assert_eq!(dead.status, ZomePathStatus::Dead);
        assert_eq!(dead.app_disabled_reason, None);
        assert_eq!(dead.not_running_secs, None);

        roles.for_role("imagodei").record_success_at(T0 + 6_000);
        aggregate.sync_from_roles(&roles, &supervised, T0 + 6_000);
        assert_eq!(aggregate.status(), ZomePathStatus::Live);
    }

    #[test]
    fn success_then_failure_reads_dead() {
        let h = BridgeHealth::new();
        h.record_success_at(T0);
        h.record_failure_at(T0 + 5_000);
        let snap = h.snapshot_at(T0 + 10_000);
        assert_eq!(snap.status, ZomePathStatus::Dead);
        assert!(!snap.serving_ok(), "a dead zome path must fail the probe");
        assert_eq!(snap.last_success_age_secs, Some(10));
        assert_eq!(snap.last_failure_age_secs, Some(5));
        assert_eq!(snap.consecutive_failures, 1);
    }

    #[test]
    fn failure_then_success_reads_live_and_clears_the_streak() {
        // This is the recovery assertion: the supervisor re-mints the bridge,
        // the next call lands, and the node must go green WITHOUT a restart.
        let h = BridgeHealth::new();
        h.record_failure_at(T0);
        h.record_failure_at(T0 + 1_000);
        assert_eq!(h.snapshot_at(T0 + 1_000).consecutive_failures, 2);
        h.record_success_at(T0 + 2_000);
        let snap = h.snapshot_at(T0 + 2_000);
        assert_eq!(snap.status, ZomePathStatus::Live);
        assert!(snap.serving_ok());
        assert_eq!(snap.consecutive_failures, 0);
    }

    #[test]
    fn only_failures_reads_dead() {
        let h = BridgeHealth::new();
        h.record_failure_at(T0);
        assert_eq!(h.snapshot_at(T0).status, ZomePathStatus::Dead);
    }

    #[test]
    fn reconnect_counter_is_surfaced() {
        let h = BridgeHealth::new();
        h.record_reconnect();
        h.record_reconnect();
        assert_eq!(h.snapshot_at(T0).reconnects, 2);
    }

    #[test]
    fn a_backwards_clock_saturates_the_age_and_never_inverts_the_status() {
        let h = BridgeHealth::new();
        h.record_success_at(T0 + 10_000);
        let snap = h.snapshot_at(T0);
        assert_eq!(snap.last_success_age_secs, Some(0));
        assert_eq!(snap.status, ZomePathStatus::Live);
    }

    // ---- error classification -------------------------------------------

    #[test]
    fn the_observed_live_failure_string_classifies_dead() {
        // Verbatim from the reproduction (throwaway conductor restarted under a
        // running storage, 2026-08-21): this exact string must read as DEAD or
        // the whole supervisor never arms.
        let msg = "Zome call failed: Websocket error: Websocket closed: No connection";
        assert!(is_transport_dead(msg));
        assert_eq!(classify_zome_error(msg), ZomeObservation::PathDead);
    }

    #[test]
    fn the_observed_ping_failure_string_classifies_dead() {
        let msg = "Conductor ping failed: Websocket error: Websocket closed: No connection";
        assert_eq!(classify_zome_error(msg), ZomeObservation::PathDead);
    }

    /// RENAMED WITH ITS SUBJECT (2026-09-21): a domain error proves the path is
    /// RESPONSIVE, which is all it ever proved. Treating it as death would flap
    /// the node red on ordinary refusals; treating it as recovery is the other
    /// error, and `a_zome_not_found_answer_does_not_end_an_episode` below is
    /// where that half is pinned.
    #[test]
    fn a_domain_error_proves_the_path_is_responsive() {
        for msg in [
            "Zome call failed: ZomeNotFound: mishpat",
            "Zome call failed: Wasm error while working with Ribosome: Guest(\"no record\")",
            "Zome call failed: Unauthorized",
        ] {
            assert!(
                !is_transport_dead(msg),
                "{msg} must not read as transport-dead"
            );
            assert_eq!(
                classify_zome_error(msg),
                ZomeObservation::PathResponsive,
                "{msg}"
            );
        }
    }

    /// THE 2026-09-18 DEFECT, as a claim about evidence.
    ///
    /// Verbatim from Loki (elohim-alpha, matthew/adam/eve storage pods,
    /// 2026-09-18T13:54Z onward): every zome call on every supervised role came
    /// back with this, for two days, while `/health` reported `zomePath: live`
    /// — because the marker list held only TRANSPORT failures and everything
    /// else fell through to `PathLive`. The conductor DID answer, so the bytes
    /// crossed; but the answer was that nothing can be written. Calling that
    /// evidence of a live path is how a disabled app hid for 38 hours.
    #[test]
    fn a_cell_disabled_error_is_not_evidence_of_a_live_path() {
        let msg = "Zome call failed: Conductor returned an error while using a ConductorApi: \
                   CellDisabled(CellId(DnaHash(uhC0kY8xE), AgentPubKey(uhCAkR2vQ)))";
        assert!(
            is_cell_disabled(msg),
            "the observed live string must be recognised as a disabled cell"
        );
        assert_eq!(
            classify_zome_error(msg),
            ZomeObservation::AppDisabled,
            "a disabled cell is its own class — not live, and not a dead websocket"
        );
        assert_ne!(
            classify_zome_error(msg),
            ZomeObservation::PathResponsive,
            "EVIDENCE THE PATH WORKS is exactly the lie that cost two days"
        );

        // And it must move the observed status off live, so `/health/serving`
        // can red and an operator has something to look at.
        let h = BridgeHealth::new();
        h.record_success_at(T0);
        h.observe_zome_error(msg);
        let snap = h.snapshot();
        assert_eq!(snap.status, ZomePathStatus::AppDisabled);
        assert!(
            !snap.serving_ok(),
            "a node whose app is disabled cannot write truth"
        );
        assert!(
            snap.app_disabled_reason.is_some(),
            "the reason is the missing diagnosis — it must reach the surface"
        );
    }

    /// The probe half of the same defect: `spawn_bridge_supervisor` pinged with
    /// `app_info()`, which SUCCEEDS on a disabled app, and threw the returned
    /// status away. The status is the only thing in that answer that can tell
    /// a running app from a dead one.
    #[test]
    fn an_app_info_status_of_disabled_is_not_live() {
        use holochain_types::app::{AppStatus, DisabledAppReason};

        assert_eq!(
            classify_app_status(&AppStatus::Enabled),
            AppRunObservation::Running
        );

        for (status, needle) in [
            (
                AppStatus::Disabled(DisabledAppReason::NeverStarted),
                "never started",
            ),
            (AppStatus::Disabled(DisabledAppReason::User), "operator"),
            (
                AppStatus::Disabled(DisabledAppReason::Error("cell db locked".into())),
                "cell db locked",
            ),
            (AppStatus::AwaitingMemproofs, "memproof"),
            (AppStatus::AwaitingRestore, "restore"),
        ] {
            match classify_app_status(&status) {
                AppRunObservation::NotRunning { reason, .. } => assert!(
                    reason.to_ascii_lowercase().contains(needle),
                    "the reason must name why: {reason} (looking for {needle})"
                ),
                AppRunObservation::Running => {
                    panic!("{status:?} is not running and must never read as running")
                }
            }
        }
    }

    #[test]
    fn an_admission_shed_proves_nothing() {
        // Nothing was dispatched; the conductor never saw the call. Recording
        // this as a live path would launder a shed into evidence of health.
        let msg = format!(
            "{}: no conductor permit for content_store within 250ms",
            crate::conductor_admission::ADMISSION_SHED_MARKER
        );
        assert_eq!(classify_zome_error(&msg), ZomeObservation::NoEvidence);

        let h = BridgeHealth::new();
        h.observe_zome_error(&msg);
        assert_eq!(
            h.snapshot_at(T0).status,
            ZomePathStatus::Unknown,
            "a shed must not move the status"
        );
    }

    #[test]
    fn same_millisecond_observations_are_ordered_by_sequence_not_clock() {
        // REGRESSION (caught by the first run of the suite below): a supervisor
        // tick records a failure and the recovery that follows inside the SAME
        // millisecond routinely. Deciding the status by comparing equal ms
        // stamps reports the older observation — a recovered node reading dead,
        // or worse, a dead node reading live.
        let h = BridgeHealth::new();
        h.record_success_at(T0);
        h.record_failure_at(T0); // identical stamp, later observation
        assert_eq!(h.snapshot_at(T0).status, ZomePathStatus::Dead);

        let h = BridgeHealth::new();
        h.record_failure_at(T0);
        h.record_success_at(T0); // identical stamp, later observation
        assert_eq!(h.snapshot_at(T0).status, ZomePathStatus::Live);
    }

    #[test]
    fn observe_zome_error_routes_each_class() {
        let h = BridgeHealth::new();
        h.observe_zome_error("Zome call failed: ZomeNotFound: mishpat");
        assert_eq!(h.snapshot().status, ZomePathStatus::Live);
        h.observe_zome_error("Zome call failed: Websocket closed: No connection");
        assert_eq!(h.snapshot().status, ZomePathStatus::Dead);
    }

    // ---- the wire shape --------------------------------------------------

    #[test]
    fn health_block_carries_the_zome_path_verdict() {
        let h = BridgeHealth::new();
        h.record_success_at(T0);
        h.record_failure_at(T0 + 1_000);
        let block = health_block("external", &h.snapshot_at(T0 + 61_000));
        assert_eq!(block["mode"], "external");
        assert_eq!(block["zomePath"], "dead");
        assert_eq!(block["lastZomeCallAgeSecs"], 61);
        assert_eq!(block["lastZomeFailureAgeSecs"], 60);
        assert_eq!(block["consecutiveFailures"], 1);
    }

    #[test]
    fn health_block_nulls_the_age_before_any_call() {
        let block = health_block("embedded", &BridgeHealth::new().snapshot_at(T0));
        assert_eq!(block["zomePath"], "unknown");
        assert!(block["lastZomeCallAgeSecs"].is_null());
    }

    // ---- per-role aggregation ---------------------------------------------
    //
    // These build their OWN `RoleBridgeHealth` instance rather than touching
    // the process-wide `role_bridge_health()` singleton, for the same reason
    // every other test in this file builds its own `BridgeHealth`: parallel
    // `cargo test` threads share one process, and a test that mutated the
    // real singleton would be flaky by construction (see `bridge_health`'s
    // doc). The logic under test (`derive_supervised_status`, `per_role_block`)
    // is the exact same code the production singleton uses — only the
    // instance is isolated.

    const SUPERVISED: [&str; 3] = ["infrastructure", "imagodei", "lamad"];

    #[test]
    fn one_dead_role_among_n_reads_dead_and_the_map_names_which() {
        let roles = RoleBridgeHealth::new();
        roles.for_role("infrastructure").record_success();
        roles.for_role("imagodei").record_success();
        roles.for_role("lamad").record_failure();

        assert_eq!(
            roles.derive_supervised_status(&SUPERVISED),
            ZomePathStatus::Dead,
            "one dead role among N must sink the aggregate"
        );

        let block = per_role_block(&roles);
        assert_eq!(
            block["lamad"]["zomePath"], "dead",
            "the map must name lamad"
        );
        assert_eq!(block["infrastructure"]["zomePath"], "live");
        assert_eq!(block["imagodei"]["zomePath"], "live");
    }

    #[test]
    fn recovering_the_dead_role_flips_the_aggregate_back_to_live() {
        // The exact recovery this whole module exists to prove: a supervisor
        // re-mint lands, the next call on the recovered role succeeds, and
        // the derived aggregate must clear WITHOUT anything else changing.
        let roles = RoleBridgeHealth::new();
        roles.for_role("infrastructure").record_success();
        roles.for_role("imagodei").record_success();
        roles.for_role("lamad").record_failure();
        assert_eq!(
            roles.derive_supervised_status(&SUPERVISED),
            ZomePathStatus::Dead
        );

        roles.for_role("lamad").record_success();
        assert_eq!(
            roles.derive_supervised_status(&SUPERVISED),
            ZomePathStatus::Live
        );
    }

    #[test]
    fn a_role_never_observed_does_not_manufacture_a_red() {
        // A node that doesn't provision every supervised DNA (or hasn't made
        // its first call yet) must not read Dead just because a role sits at
        // Unknown while siblings are Live.
        let roles = RoleBridgeHealth::new();
        roles.for_role("infrastructure").record_success();
        assert_eq!(
            roles.derive_supervised_status(&SUPERVISED),
            ZomePathStatus::Live
        );
    }

    #[test]
    fn fresh_registry_with_no_evidence_anywhere_is_unknown() {
        let roles = RoleBridgeHealth::new();
        assert_eq!(
            roles.derive_supervised_status(&SUPERVISED),
            ZomePathStatus::Unknown
        );
    }

    // ---- the verdict and the ladder (corrected 2026-09-20) ----------------
    //
    // THE DEFECT these pin, observed on the household mesh at 19:16–19:18Z:
    //
    //   19:16:19 WARN  lamad  conductor app is NOT RUNNING (CellDisabled …)
    //   19:16:39 INFO  lamad  conductor app is RUNNING again — backoff reset
    //   19:17:40 WARN  lamad  NOT RUNNING      <- 1.8 s later
    //
    // A conductor accepts websocket traffic before its apps finish starting.
    // `app_info` reports the app `Enabled` the whole time; `enable_app` on an
    // already-enabled app returns `Ok` without touching its cells; and every
    // zome call answers `CellDisabled` until the cells reach `running_cells`
    // (~11 minutes, on one peer). Reading either conductor answer as recovery
    // produced a false verdict AND pinned a 60s-doubling ladder to its first
    // rung. These drive their OWN registry and ledger, for the parallel-test
    // reason every other test in this file does.

    /// Verbatim from the household reproduction.
    const CELL_DISABLED: &str = "Zome call failed: Conductor returned an error while using a \
                                 ConductorApi: CellDisabled(CellId(DnaHash(uhC0keuLMYBe0), \
                                 AgentPubKey(uhCAkR2vQ)))";

    #[test]
    fn enable_ok_on_an_already_enabled_app_does_not_prove_recovery() {
        let roles = RoleBridgeHealth::new();
        let ledger = EnableLedger::new();
        let t0 = Instant::now();

        // A zome call answered CellDisabled: this role cannot write truth.
        assert!(roles.record_app_disabled_on("lamad", CELL_DISABLED, T0));

        // One attempt is spent on the ladder and the conductor answers `Ok` —
        // as it always does for an app that is already enabled — and the probe
        // that follows reports the app `Enabled`. Together, the whole of what
        // the conductor is willing to tell us, and none of it is evidence.
        assert!(ledger.should_attempt_at("lamad", t0));
        ledger.note_attempt_at("lamad", t0);
        assert!(roles.note_app_enabled_but_not_running_on("lamad"));

        assert_eq!(
            roles.for_role("lamad").status(),
            ZomePathStatus::AppDisabled,
            "an accepted enable and an `Enabled` app status must not move the verdict"
        );
        assert_eq!(
            roles.derive_supervised_status(&SUPERVISED),
            ZomePathStatus::AppDisabled,
            "nor the aggregate — `/health/serving` must stay red while calls are refused"
        );
        assert!(!roles.for_role("lamad").snapshot_at(T0).serving_ok());
        assert_eq!(
            ledger.attempts("lamad"),
            1,
            "and the ladder must still be standing where the attempt left it"
        );
    }

    #[test]
    fn a_no_op_enable_does_not_reset_the_backoff() {
        let roles = RoleBridgeHealth::new();
        let ledger = EnableLedger::new();
        roles.record_app_disabled_on("lamad", CELL_DISABLED, T0);

        let mut now = Instant::now();
        let mut windows = Vec::new();
        for round in 1..=3 {
            assert!(
                ledger.should_attempt_at("lamad", now),
                "round {round}: the window must be open"
            );
            ledger.note_attempt_at("lamad", now);
            // `enable_app` answers Ok, the probe answers `Enabled`, and the
            // next zome call is still refused. None of it may clear the ladder.
            roles.note_app_enabled_but_not_running_on("lamad");
            roles.record_app_disabled_on("lamad", CELL_DISABLED, T0 + round * 1_000);

            let window = enable_backoff(ledger.attempts("lamad"));
            windows.push(window.as_secs());
            assert!(
                !ledger.should_attempt_at("lamad", now + window - Duration::from_secs(1)),
                "round {round}: shut for the whole {}s window",
                window.as_secs()
            );
            now += window;
        }
        assert_eq!(
            windows,
            vec![60, 120, 240],
            "three Ok-but-still-refused attempts must DOUBLE the wait each time — the observed \
             defect held this at 60s forever by resetting on the conductor's own answer"
        );
    }

    #[test]
    fn recovery_is_proven_by_a_successful_zome_call_and_resets_the_backoff() {
        let roles = RoleBridgeHealth::new();
        let ledger = EnableLedger::new();
        let t0 = Instant::now();
        roles.record_app_disabled_on("lamad", CELL_DISABLED, T0);
        for step in 0..3 {
            ledger.note_attempt_at("lamad", t0 + Duration::from_secs(step * 60));
        }

        // Eleven minutes in, the conductor finishes registering its cells and
        // the next organic call lands. THAT — and nothing before it — is the
        // recovery.
        let outcome = {
            let observer = roles.for_role("lamad");
            let publishing = observer.publish_guard();
            roles.record_success_on("lamad", &publishing, &ledger, T0 + 660_000)
        };

        assert!(outcome.recovered_from_not_running);
        assert_eq!(
            outcome.not_running_secs, 660,
            "the recovery line reports the whole episode, not the age of its last observation"
        );
        assert_eq!(
            outcome.enable_attempts, 3,
            "and what the episode cost in admin calls"
        );
        assert_eq!(roles.for_role("lamad").status(), ZomePathStatus::Live);
        assert!(roles
            .for_role("lamad")
            .snapshot_at(T0 + 660_000)
            .serving_ok());
        assert_eq!(
            ledger.attempts("lamad"),
            0,
            "proven recovery — and only proven recovery — clears the ladder"
        );
    }

    #[test]
    fn the_not_running_warning_is_logged_once_per_episode_not_per_call() {
        // The WARN is driven by this return value, so the cadence is asserted
        // without asserting on a log sink. A busy node discovers a disabled
        // cell thousands of times a minute; one line per episode is the whole
        // difference between a diagnosis and a wall of copies of itself.
        let roles = RoleBridgeHealth::new();
        assert!(
            roles.record_app_disabled_on("lamad", CELL_DISABLED, T0),
            "the transition into the episode is the line that matters"
        );
        for tick in 1..=50u64 {
            assert!(
                !roles.record_app_disabled_on("lamad", CELL_DISABLED, T0 + tick * 100),
                "observation {tick} is the SAME episode and must stay quiet"
            );
        }
    }

    #[test]
    fn the_already_enabled_diagnosis_is_said_once_per_episode() {
        let roles = RoleBridgeHealth::new();

        // Nothing to diagnose about a role that is not refusing calls.
        assert!(!roles.note_app_enabled_but_not_running_on("lamad"));
        roles.for_role("lamad").record_success_at(T0);
        assert!(
            !roles.note_app_enabled_but_not_running_on("lamad"),
            "a live role's `Enabled` status is unremarkable"
        );

        roles.record_app_disabled_on("lamad", CELL_DISABLED, T0 + 1_000);
        assert!(
            roles.note_app_enabled_but_not_running_on("lamad"),
            "enabled + refusing calls = its CELLS are not running: say so"
        );
        for tick in 1..=20 {
            assert!(
                !roles.note_app_enabled_but_not_running_on("lamad"),
                "probe {tick}: said once per episode, not once per 20s probe"
            );
        }
    }

    #[test]
    fn a_second_episode_after_recovery_starts_a_fresh_backoff() {
        let roles = RoleBridgeHealth::new();
        let ledger = EnableLedger::new();
        let t0 = Instant::now();

        // Episode one climbs the ladder and is diagnosed.
        roles.record_app_disabled_on("lamad", CELL_DISABLED, T0);
        for step in 0..5 {
            ledger.note_attempt_at("lamad", t0 + Duration::from_secs(step * 4_000));
        }
        assert!(roles.note_app_enabled_but_not_running_on("lamad"));
        assert_eq!(
            enable_backoff(ledger.attempts("lamad")),
            Duration::from_secs(960)
        );

        // A zome call lands: episode one is over.
        let climbed = t0 + Duration::from_secs(4 * 4_000);
        {
            let observer = roles.for_role("lamad");
            let publishing = observer.publish_guard();
            roles.record_success_on("lamad", &publishing, &ledger, T0 + 60_000)
        };

        // A relapse is a NEW episode in every respect — it WARNs again, it is
        // diagnosable again, and it is met at once rather than at the hour cap
        // the previous outage had climbed to.
        let relapse = climbed + Duration::from_secs(3_600);
        assert!(
            roles.record_app_disabled_on("lamad", CELL_DISABLED, T0 + 3_660_000),
            "a relapse earns its own WARN"
        );
        assert!(
            roles.note_app_enabled_but_not_running_on("lamad"),
            "and its own diagnosis — episode one's sentence must not silence episode two"
        );
        assert!(
            ledger.should_attempt_at("lamad", relapse),
            "met immediately, not an hour later"
        );
        ledger.note_attempt_at("lamad", relapse);
        assert_eq!(
            enable_backoff(ledger.attempts("lamad")),
            Duration::from_secs(60),
            "back to the first rung"
        );
    }

    #[test]
    fn one_role_recovering_does_not_mark_another_running() {
        // Cells are registered per app, and the household watched one peer
        // finish while another sat at zero for a further six minutes. Evidence
        // from one role says nothing whatever about another's.
        let roles = RoleBridgeHealth::new();
        let ledger = EnableLedger::new();
        let t0 = Instant::now();
        for role in ["lamad", "imagodei"] {
            roles.record_app_disabled_on(role, CELL_DISABLED, T0);
            ledger.note_attempt_at(role, t0);
        }

        {
            let observer = roles.for_role("lamad");
            let publishing = observer.publish_guard();
            roles.record_success_on("lamad", &publishing, &ledger, T0 + 5_000)
        };

        assert_eq!(roles.for_role("lamad").status(), ZomePathStatus::Live);
        assert_eq!(
            roles.for_role("imagodei").status(),
            ZomePathStatus::AppDisabled,
            "a call landing on lamad proves nothing about imagodei's cells"
        );
        assert_eq!(
            ledger.attempts("imagodei"),
            1,
            "nor may it clear imagodei's ladder"
        );
        assert_eq!(
            roles.derive_supervised_status(&SUPERVISED),
            ZomePathStatus::AppDisabled,
            "and one role still refusing calls keeps the aggregate honest"
        );
        assert!(roles.note_app_enabled_but_not_running_on("imagodei"));
        assert!(
            !roles.note_app_enabled_but_not_running_on("lamad"),
            "the recovered role has nothing to diagnose"
        );
    }

    /// The same claim as the tests above, but against the PRODUCTION wiring —
    /// the free functions `HcClient` and the bridge supervisor actually call,
    /// with the process-wide registry, ledger and gauge behind them.
    ///
    /// The instance tests pin the folds; this pins that the folds are what is
    /// wired. Under the pre-correction wiring
    /// (`observe_role_app_status(Running) -> record_role_success`) it fails on
    /// its first assertion, which is the defect exactly: one `app_info` answer
    /// ended an episode the conductor had not ended.
    ///
    /// Safe against the process-wide singletons the rest of this file avoids,
    /// because every one of them is keyed by ROLE and this role name belongs to
    /// this test alone — it is not in `SUPERVISED_ROLES`, so it cannot even
    /// reach the aggregate.
    #[test]
    fn the_probe_cannot_end_an_episode_the_conductor_has_not_ended() {
        const ROLE: &str = "test-probe-cannot-end-an-episode";
        let ledger = crate::services::enable_app_backoff::enable_ledger();

        // A zome call answered CellDisabled.
        record_role_app_disabled(ROLE, CELL_DISABLED);
        assert!(role_is_not_running(ROLE));

        // One enable attempt is spent, then the conductor spends the next
        // several minutes insisting the app is enabled — which it is. Its cells
        // are not. Twenty probes, one per supervisor tick.
        ledger.note_attempt(ROLE);
        for tick in 1..=20 {
            observe_role_app_status(ROLE, &AppRunObservation::Running);
            assert!(
                role_is_not_running(ROLE),
                "probe {tick}: an `Enabled` app status must not end the episode"
            );
            assert_eq!(
                ledger.attempts(ROLE),
                1,
                "probe {tick}: nor rewind the bounded ladder"
            );
        }

        // The conductor finishes starting and a zome call lands.
        record_role_success(ROLE);
        assert!(!role_is_not_running(ROLE));
        assert_eq!(
            role_bridge_health().for_role(ROLE).status(),
            ZomePathStatus::Live
        );
        assert_eq!(
            ledger.attempts(ROLE),
            0,
            "and THAT — a call that crossed into the cell — clears the ladder"
        );
    }

    #[test]
    fn the_wire_names_how_long_the_role_has_been_refusing_calls() {
        // A conductor that is still starting and one that is stuck are
        // identical in every other field; they differ only in how long this
        // number has been growing.
        let h = BridgeHealth::new();
        h.record_app_disabled_at(T0, CELL_DISABLED);
        h.record_app_disabled_at(T0 + 600_000, CELL_DISABLED);
        let snap = h.snapshot_at(T0 + 660_000);
        assert_eq!(
            snap.not_running_secs,
            Some(660),
            "measured from the start of the episode, not from its last observation"
        );
        assert_eq!(role_status_json(&snap)["notRunningSecs"], 660);

        h.record_success_at(T0 + 700_000);
        let recovered = h.snapshot_at(T0 + 700_000);
        assert_eq!(recovered.status, ZomePathStatus::Live);
        assert!(
            recovered.not_running_secs.is_none(),
            "a live role is not 'not running for N seconds'"
        );
        assert!(role_status_json(&recovered)["notRunningSecs"].is_null());
    }

    // ---- responsive is not served (F7, corrected 2026-09-21) --------------

    /// A wrong zome or function name answers `ZomeNotFound` / `FunctionNotFound`.
    /// That is the conductor ANSWERING, and before this correction every such
    /// answer took the `PathLive` arm straight into `record_success` — which
    /// ended the episode. It is also the reason `3ec3614dd` deliberately shipped
    /// NO probe: a probe aimed at a name the DNA lacks would have manufactured
    /// recovery out of its own misconfiguration. Closing this is what makes a
    /// probe safe to add at all.
    #[test]
    fn a_zome_not_found_answer_does_not_end_an_episode() {
        let h = BridgeHealth::new();
        h.record_app_disabled_at(T0, CELL_DISABLED);
        assert_eq!(h.snapshot_at(T0).status, ZomePathStatus::AppDisabled);

        for msg in [
            "Zome call failed: ZomeNotFound: mishpat",
            "Zome call failed: FunctionNotFound(\"is_bootstrap_steward\")",
            "Zome call failed: Wasm error while working with Ribosome: Guest(\"no record\")",
            "Zome call failed: something nobody has ever seen before",
        ] {
            assert_eq!(
                classify_zome_error(msg),
                ZomeObservation::PathResponsive,
                "{msg} is the conductor answering — that much is true"
            );
            h.observe_zome_error(msg);
            assert_eq!(
                h.snapshot_at(T0 + 1_000).status,
                ZomePathStatus::AppDisabled,
                "...but a FAILED call must never end a not-running episode: {msg}"
            );
            assert!(
                !h.snapshot_at(T0 + 1_000).serving_ok(),
                "and /health/serving must stay red through it: {msg}"
            );
        }

        // The episode clock is untouched too — a recovery line that reported the
        // age of the last domain error instead of the length of the outage would
        // tell an operator to wait when they should intervene.
        assert_eq!(h.snapshot_at(T0 + 660_000).not_running_secs, Some(660));
        assert!(h.snapshot_at(T0).app_disabled_reason.is_some());
    }

    /// The other half: a responsive observation IS worth something — it clears a
    /// DEAD verdict, because a conductor that answers has a live websocket. The
    /// asymmetry is the whole design: responsive beats dead, served beats
    /// disabled, and nothing else moves.
    #[test]
    fn a_responsive_answer_clears_a_dead_verdict_but_never_a_disabled_one() {
        let dead = BridgeHealth::new();
        dead.record_failure_at(T0);
        assert_eq!(dead.snapshot_at(T0).status, ZomePathStatus::Dead);
        assert!(dead.record_responsive_at(T0 + 1_000), "folded");
        assert_eq!(dead.snapshot_at(T0 + 1_000).status, ZomePathStatus::Live);
        assert_eq!(dead.snapshot_at(T0 + 1_000).consecutive_failures, 0);

        let disabled = BridgeHealth::new();
        disabled.record_app_disabled_at(T0, CELL_DISABLED);
        assert!(
            disabled.record_responsive_at(T0 + 1_000),
            "RETAINED: it is real evidence about the SOCKET, and with the two verdict streams \
             separated it can no longer be mistaken for evidence about the cell. Discarding it \
             only threw away what cancels a delayed transport failure."
        );
        assert_eq!(
            disabled.snapshot_at(T0 + 1_000).status,
            ZomePathStatus::AppDisabled
        );
    }

    /// Through the PRODUCTION free functions, with the real gauge and the real
    /// ledger: an unrecognised error is recorded as responsive and changes
    /// nothing else. Under the pre-correction wiring the first gauge assertion
    /// fails — that gauge staying at 0 beside a `live` verdict was the
    /// observable shape of the defect.
    #[test]
    fn an_unrecognised_error_is_responsive_but_not_recovered() {
        const ROLE: &str = "test-responsive-not-recovered";
        let ledger = crate::services::enable_app_backoff::enable_ledger();
        let gauge = crate::metrics::CONDUCTOR_APP_ENABLED.with_label_values(&[ROLE]);

        record_role_app_disabled(ROLE, CELL_DISABLED);
        assert_eq!(gauge.get(), 0, "the episode lowered the level");
        ledger.note_attempt(ROLE);
        ledger.note_attempt(ROLE);
        assert_eq!(ledger.attempts(ROLE), 2);

        for tick in 1..=5 {
            observe_role_zome_error(ROLE, "Zome call failed: ZomeNotFound: content_store");
            assert!(
                role_is_not_running(ROLE),
                "answer {tick}: the episode stands"
            );
            assert_eq!(
                gauge.get(),
                0,
                "answer {tick}: the gauge is raised by the ONE transition and nothing else"
            );
            assert_eq!(
                ledger.attempts(ROLE),
                2,
                "answer {tick}: the ladder is untouched"
            );
        }

        // And the real recovery still works, from the same state.
        record_role_success(ROLE);
        assert!(!role_is_not_running(ROLE));
        assert_eq!(gauge.get(), 1);
        assert_eq!(ledger.attempts(ROLE), 0);
    }

    /// THE STRANDED BACKOFF, as the sequence that produced it. A responsive
    /// answer used to be able to park the role at `Live` while the ladder still
    /// carried an episode's worth of attempts; the next genuine success then
    /// read `before.status == Live`, skipped `note_running`, and the NEXT outage
    /// was met at the hour cap instead of at 60s.
    ///
    /// Driven through BOTH orderings, including the transport-failure detour that
    /// a status-only guard cannot see.
    #[test]
    fn a_second_outage_starts_a_fresh_ladder_after_any_recovery_path() {
        for detour in ["plain", "through-a-dead-websocket"] {
            let roles = RoleBridgeHealth::new();
            let ledger = EnableLedger::new();
            let t0 = Instant::now();

            // Episode one climbs to the cap.
            roles.record_app_disabled_on("lamad", CELL_DISABLED, T0);
            for step in 0..7 {
                ledger.note_attempt_at("lamad", t0 + Duration::from_secs(step * 4_000));
            }
            assert_eq!(
                enable_backoff(ledger.attempts("lamad")),
                Duration::from_secs(3_600),
                "{detour}: the ladder is at the hour cap"
            );

            // The path that used to strand it: a transport failure, then a
            // responsive domain error. With the episode still open the fold is
            // refused; after a `Dead` it is PERMITTED — a responsive answer is
            // real evidence that the websocket came back.
            //
            // What changed (2026-09-22): folding it no longer LIFTS the
            // not-running verdict. A responsive answer now cancels a failure and
            // nothing else, so the detour parks the role at `AppDisabled` — the
            // episode nothing entitled to end it has ended — instead of at
            // `Live` with an hour-deep ladder underneath, which is precisely the
            // state this test's earlier cut called "indistinguishable from a
            // healthy steady state". The ladder assertions below are unchanged;
            // they were always the point.
            let detoured = detour == "through-a-dead-websocket";
            if detoured {
                roles.for_role("lamad").record_failure_at(T0 + 1_000);
            }
            let folded = roles.for_role("lamad").record_responsive_at(T0 + 2_000);
            assert!(
                folded,
                "{detour}: a responsive answer is RETAINED either way — it is transport-stream \
                 evidence, and the served-cell stream is what owns the verdict"
            );
            assert_eq!(
                roles.for_role("lamad").status(),
                ZomePathStatus::AppDisabled,
                "{detour}: the transport came back, but the not-running episode stands — a \
                 responsive answer cancels a failure, never a disabled"
            );

            // A call lands. Whatever the node believed a millisecond earlier, a
            // ladder with attempts on it is cleared by a call that landed.
            let outcome = {
                let observer = roles.for_role("lamad");
                let publishing = observer.publish_guard();
                roles.record_success_on("lamad", &publishing, &ledger, T0 + 10_000)
            };
            assert_eq!(
                ledger.attempts("lamad"),
                0,
                "{detour}: proven recovery clears the ladder"
            );
            assert!(
                outcome.enable_attempts >= 7,
                "{detour}: and reports what the episode cost"
            );

            // The relapse is met at the FIRST rung.
            let relapse = t0 + Duration::from_secs(40_000);
            roles.record_app_disabled_on("lamad", CELL_DISABLED, T0 + 40_000_000);
            assert!(
                ledger.should_attempt_at("lamad", relapse),
                "{detour}: met immediately"
            );
            ledger.note_attempt_at("lamad", relapse);
            assert_eq!(
                enable_backoff(ledger.attempts("lamad")),
                Duration::from_secs(60),
                "{detour}: back to 60s — a later outage must not inherit an earlier one's backoff"
            );
        }
    }

    /// F4 at the health layer, composed with the attribution decision the three
    /// `HcClient` paths now share. The existing `one_role_recovering_does_not_
    /// mark_another_running` pins the FOLD with hand-written role keys; this
    /// derives the keys the way production does, so restoring `role_key()` at
    /// the call sites fails here too.
    #[test]
    fn a_lamad_success_does_not_end_a_mishpat_episode() {
        use holochain_types::prelude::{AgentPubKey, CellId, DnaHash};

        let cell = |tag: u8| {
            CellId::new(
                DnaHash::from_raw_32(vec![tag; 32]),
                AgentPubKey::from_raw_32(vec![0xAA; 32]),
            )
        };
        let lamad_cell = cell(1);
        let mishpat_cell = cell(2);
        // The production shape: ONE client, configured `lamad`, holding the
        // mishpat cell for cross-cell calls.
        let role_of = |target: &CellId| {
            crate::hc_client::target_role_for_cell("lamad", Some(&mishpat_cell), None, target)
        };
        assert_eq!(role_of(&lamad_cell), "lamad");
        assert_eq!(role_of(&mishpat_cell), "mishpat");

        let roles = RoleBridgeHealth::new();
        let ledger = EnableLedger::new();
        let t0 = Instant::now();

        // A governance call answers CellDisabled. It is MISHPAT's episode.
        roles.record_app_disabled_on(role_of(&mishpat_cell), CELL_DISABLED, T0);
        ledger.note_attempt_at(role_of(&mishpat_cell), t0);
        assert_eq!(
            roles.for_role("lamad").status(),
            ZomePathStatus::Unknown,
            "lamad has not been slandered: nothing was observed about its cell"
        );

        // An ordinary content read succeeds on the lamad cell.
        {
            let observer = roles.for_role(role_of(&lamad_cell));
            let publishing = observer.publish_guard();
            roles.record_success_on(role_of(&lamad_cell), &publishing, &ledger, T0 + 5_000)
        };

        assert_eq!(roles.for_role("lamad").status(), ZomePathStatus::Live);
        assert_eq!(
            roles.for_role("mishpat").status(),
            ZomePathStatus::AppDisabled,
            "a call landing on the lamad cell proves nothing about the mishpat cell"
        );
        assert_eq!(
            ledger.attempts("mishpat"),
            1,
            "nor may it clear mishpat's ladder"
        );
        assert_eq!(
            roles.derive_supervised_status(&crate::hc_client_registry::OBSERVED_ROLES),
            ZomePathStatus::AppDisabled,
            "and mishpat is OBSERVED, so its refusal reaches /health/serving instead of hiding"
        );
    }

    /// A quiet role — one with no organic traffic to prove recovery with — is
    /// carried to green by the probe alone, and by the SAME transition organic
    /// traffic uses.
    ///
    /// The probe's conductor round-trip is not constructible offline, so what is
    /// driven here is everything either side of it: the admission gates decide
    /// the probe may ask, and the return is folded by the one function the
    /// `HcClient` path calls. The absence of a second transition inside the probe
    /// is pinned by `cell_probe::the_probe_declares_no_recovery_of_its_own`.
    #[test]
    fn a_quiet_role_recovers_through_the_probe_without_organic_traffic() {
        const ROLE: &str = "node_registry";
        let roles = RoleBridgeHealth::new();
        let ledger = EnableLedger::new();
        let t0 = Instant::now();

        // The restart episode every node now gets: cells absent from
        // `running_cells`, answering CellDisabled.
        roles.record_app_disabled_on(ROLE, CELL_DISABLED, T0);
        assert!(!roles.for_role(ROLE).snapshot_at(T0).serving_ok());

        // This role's only organic caller is upload shard assignment, and no
        // upload happens. Three ladder rungs pass with nothing to prove recovery
        // with — which before the probe was the end of the story, forever.
        for step in 0..3 {
            ledger.note_attempt_at(ROLE, t0 + Duration::from_secs(step * 60));
            assert_eq!(
                roles.for_role(ROLE).status(),
                ZomePathStatus::AppDisabled,
                "rung {step}: nothing else can end this"
            );
        }

        // The role HAS a probe — without one it could never clear.
        let probe = crate::services::cell_probe::probe_for(ROLE)
            .expect("a quiet role must have a probe or its episode is permanent");
        assert!(
            crate::chain_write_gate::is_read_fn(probe.fn_name),
            "and it must be a classified read, or it would queue behind writers on a call nothing \
             can cancel"
        );

        // The conductor finishes starting; the probe's call RETURNS. That return
        // is an ordinary zome-call success, so it arrives here.
        let outcome = {
            let observer = roles.for_role(ROLE);
            let publishing = observer.publish_guard();
            roles.record_success_on(ROLE, &publishing, &ledger, T0 + 240_000)
        };

        assert!(outcome.recovered_from_not_running);
        assert_eq!(outcome.not_running_secs, 240);
        assert_eq!(outcome.enable_attempts, 3);
        assert_eq!(roles.for_role(ROLE).status(), ZomePathStatus::Live);
        assert!(roles.for_role(ROLE).snapshot_at(T0 + 240_000).serving_ok());
        assert_eq!(ledger.attempts(ROLE), 0);
        // The probe stopping once the role is healthy is
        // `cell_probe::the_probe_never_runs_while_the_role_is_healthy`, which
        // drives the admission gate directly rather than through this registry.
    }

    /// The consumer contract F6 turns on, pinned at the function both surfaces
    /// render through.
    ///
    /// Established by tracing it: `GET /health/serving` on storage answers 503 +
    /// `Retry-After: 20` from `snap.serving_ok()`, and the doorway performs one
    /// bounded 2s GET of it per request with NO cache, maps any non-2xx to
    /// `refused`, and answers its own `/health/serving` 503. Neither service's
    /// `/health`, `/ready` or `/health/startup` moves, no Kubernetes probe reads
    /// it, the doorway's upstream breaker treats a 503 as neutral, and no CI gate
    /// consumes it — so the cost of a stuck-red role is a permanently dishonest
    /// signal on two endpoints, not an outage. Which is precisely why it must not
    /// be able to stick: an endpoint that cries wolf trains its readers to ignore
    /// it, and that is the failure this endpoint was created to prevent.
    #[test]
    fn health_serving_is_red_through_an_episode_and_green_once_the_probe_proves_recovery() {
        let roles = RoleBridgeHealth::new();
        let ledger = EnableLedger::new();
        let observed = &crate::hc_client_registry::OBSERVED_ROLES;

        // Four of five roles serving; the quiet one refusing.
        for role in ["infrastructure", "imagodei", "lamad", "mishpat"] {
            roles.for_role(role).record_success_at(T0);
        }
        roles.record_app_disabled_on("node_registry", CELL_DISABLED, T0);

        let derived = roles.derive_supervised_status(observed);
        assert_eq!(
            derived,
            ZomePathStatus::AppDisabled,
            "ONE refusing role sinks the aggregate — deliberately pessimistic"
        );
        assert_eq!(derived.as_str(), "app-disabled");
        let reason = roles
            .supervised_disabled_reason(observed)
            .expect("the aggregate carries a diagnosis, not a bare verdict");
        assert!(
            reason.starts_with("node_registry:"),
            "and it NAMES the role, so an operator does not have to guess: {reason}"
        );

        // While red, the body says how long — the one field that distinguishes a
        // conductor still starting from one that is stuck.
        let block = per_role_block(&roles);
        assert_eq!(block["node_registry"]["zomePath"], "app-disabled");
        assert_eq!(block["lamad"]["zomePath"], "live");

        // The probe's call returns. Nothing else about the node changed.
        {
            let observer = roles.for_role("node_registry");
            let publishing = observer.publish_guard();
            roles.record_success_on("node_registry", &publishing, &ledger, T0 + 240_000)
        };
        assert_eq!(
            roles.derive_supervised_status(observed),
            ZomePathStatus::Live,
            "probe-proven recovery is the whole of what turns /health/serving back to 200"
        );
        assert!(roles
            .for_role("node_registry")
            .snapshot_at(T0 + 240_000)
            .serving_ok());
        assert_eq!(roles.supervised_disabled_reason(observed), None);
    }

    #[test]
    fn a_role_with_no_traffic_yet_is_absent_but_lazily_creatable() {
        // `for_role` on a role nothing has touched yet must not panic and
        // must read Unknown — the same "no evidence has not earned a red"
        // contract a single `BridgeHealth` carries.
        let roles = RoleBridgeHealth::new();
        assert_eq!(
            roles.for_role("mishpat").snapshot().status,
            ZomePathStatus::Unknown
        );
    }
    // ---- cell membership × app status: the joined state (2026-09-22) -------

    /// The whole OBSERVATION table. Every row is a pair of independent reads.
    #[test]
    fn the_joined_state_names_what_was_observed() {
        use AppEnableEvidence::*;
        use RoleCellState::*;
        let rows = [
            // (membership running, enable evidence) -> observed state
            (Some(false), EnableCanLift, InstalledNotRunningAppDisabled),
            (Some(false), EnableRefused, InstalledNotRunningAppDisabled),
            (Some(false), AlreadyEnabled, InstalledNotRunningAppEnabled),
            (Some(false), Unknown, InstalledNotRunningStatusUnknown),
            (Some(true), AlreadyEnabled, RunningRecoveryUnverified),
            (Some(true), EnableCanLift, RunningRecoveryUnverified),
            (Some(true), Unknown, RunningRecoveryUnverified),
            (None, AlreadyEnabled, MembershipUnknown),
            (None, EnableCanLift, MembershipUnknown),
            (None, EnableRefused, MembershipUnknown),
            (None, Unknown, MembershipUnknown),
        ];
        for (membership, enable, expected) in rows {
            assert_eq!(
                classify_cell_state(membership, enable),
                expected,
                "membership={membership:?} enable={}",
                enable.as_str()
            );
        }
    }

    /// F4. THE MUTATION GATE IS THE APP STATUS, ON ITS OWN.
    ///
    /// `enable_app` short-circuits on an already-Enabled app, and this pin
    /// REJECTS `AwaitingMemproofs` outright — so neither may license a rung
    /// whatever membership says. `Unknown` and `EnableCanLift` keep the cure,
    /// because an absent observation is not a disproof.
    #[test]
    fn the_app_status_alone_decides_whether_enable_may_be_attempted() {
        assert!(!AppEnableEvidence::AlreadyEnabled.enable_may_help());
        assert!(!AppEnableEvidence::EnableRefused.enable_may_help());
        assert!(AppEnableEvidence::EnableCanLift.enable_may_help());
        assert!(
            AppEnableEvidence::Unknown.enable_may_help(),
            "no status evidence must not be read as 'already enabled'"
        );
    }

    /// And the STATUSES map onto that evidence the way the conductor's own
    /// source does — which is NOT "is it Enabled". `AwaitingMemproofs` and
    /// `Unrecoverable` are both not-Enabled AND not enable-able.
    #[test]
    fn awaiting_memproofs_and_unrecoverable_are_not_enable_able() {
        use holochain_types::app::{AppStatus, DisabledAppReason};
        let cases = [
            (AppStatus::Disabled(DisabledAppReason::User), true),
            (AppStatus::Disabled(DisabledAppReason::NeverStarted), true),
            (
                AppStatus::Disabled(DisabledAppReason::Error("boom".into())),
                true,
            ),
            (AppStatus::AwaitingRestore, true),
            (AppStatus::AwaitingMemproofs, false),
        ];
        for (status, may_help) in cases {
            match classify_app_status(&status) {
                AppRunObservation::NotRunning { enable, .. } => assert_eq!(
                    enable.enable_may_help(),
                    may_help,
                    "{status:?} → {}",
                    enable.as_str()
                ),
                AppRunObservation::Running => panic!("{status:?} is not running"),
            }
        }
        assert_eq!(
            classify_app_status(&AppStatus::Enabled),
            AppRunObservation::Running
        );
    }

    /// The probe is owed the instant membership says the cell is running, and
    /// never against a cell membership PROVED absent (that call fails by
    /// construction). A merely UNKNOWN membership proves nothing either way, so
    /// it does not suppress the probe.
    #[test]
    fn the_probe_is_owed_exactly_when_membership_says_running() {
        for state in ROLE_CELL_STATES {
            assert_eq!(
                state.probe_is_owed_now(),
                state == RoleCellState::RunningRecoveryUnverified,
                "{}",
                state.as_str()
            );
        }
        assert!(
            !RoleCellState::MembershipUnknown.membership_proves_absent(),
            "an unreadable membership must not be read as a proof of absence"
        );
        assert!(
            !RoleCellState::RunningRecoveryUnverified.membership_proves_absent(),
            "membership said the cell IS running"
        );
        for state in [
            RoleCellState::InstalledNotRunningAppDisabled,
            RoleCellState::InstalledNotRunningAppEnabled,
            RoleCellState::InstalledNotRunningStatusUnknown,
        ] {
            assert!(state.membership_proves_absent(), "{}", state.as_str());
        }
    }

    /// Every state has a distinct, stable label — the metric label value and the
    /// log line must not drift apart, and two states sharing a spelling would
    /// make the one-hot gauge unreadable.
    #[test]
    fn every_cell_state_has_a_distinct_stable_label() {
        let mut seen = std::collections::BTreeSet::new();
        for state in ROLE_CELL_STATES {
            assert!(
                seen.insert(state.as_str()),
                "duplicate label: {}",
                state.as_str()
            );
        }
        assert_eq!(seen.len(), 5);
        assert!(seen.contains("installed-not-running-app-disabled"));
        assert!(seen.contains("installed-not-running-app-enabled"));
        assert!(seen.contains("membership-unknown"));
        assert!(
            seen.contains("running-recovery-unverified"),
            "the label must not assert that a call failed — it names what is \
             established (membership) and what is not (recovery)"
        );
        assert!(
            !seen.contains("running-but-call-failed"),
            "the old label asserted a cause these observations do not establish"
        );
    }

    /// One line per transition, not one per tick: the supervisor re-derives this
    /// every 20s for as long as an episode lasts, and a sentence repeated every
    /// 20s buries the one that matters under copies of itself.
    #[test]
    fn a_cell_state_is_logged_on_transitions_only() {
        let h = BridgeHealth::new();
        assert!(
            h.note_cell_state(RoleCellState::InstalledNotRunningAppEnabled),
            "the first observation is a transition"
        );
        for tick in 1..=20 {
            assert!(
                !h.note_cell_state(RoleCellState::InstalledNotRunningAppEnabled),
                "tick {tick}: re-observing the same state is not news"
            );
        }
        assert!(
            h.note_cell_state(RoleCellState::RunningRecoveryUnverified),
            "the cell joined the running map — that IS news"
        );
        assert!(!h.note_cell_state(RoleCellState::RunningRecoveryUnverified));
    }

    /// The app-status half of the join is REMEMBERED from the probe that already
    /// asked, so the decision costs no admin round trip.
    #[test]
    fn app_status_evidence_starts_absent_and_then_tracks_the_probe() {
        let h = BridgeHealth::new();
        assert_eq!(
            h.last_enable_evidence(),
            AppEnableEvidence::Unknown,
            "a node that has never probed holds no status evidence"
        );
        h.note_enable_evidence(AppEnableEvidence::AlreadyEnabled);
        assert_eq!(h.last_enable_evidence(), AppEnableEvidence::AlreadyEnabled);
        h.note_enable_evidence(AppEnableEvidence::EnableRefused);
        assert_eq!(h.last_enable_evidence(), AppEnableEvidence::EnableRefused);
        h.note_enable_evidence(AppEnableEvidence::EnableCanLift);
        assert_eq!(h.last_enable_evidence(), AppEnableEvidence::EnableCanLift);
    }

    /// And it is wired to the probe's own fold, both ways — the `Running` answer
    /// records `AlreadyEnabled` even though it is worth nothing as evidence of
    /// health, because "could enable_app act" is a different question.
    #[test]
    fn the_app_info_probe_records_the_status_it_must_not_believe() {
        const ROLE: &str = "test-status-evidence-recorded";
        observe_role_app_status(
            ROLE,
            &AppRunObservation::NotRunning {
                reason: "disabled: by an operator through the admin interface".to_string(),
                enable: AppEnableEvidence::EnableCanLift,
            },
        );
        assert_eq!(last_enable_evidence(ROLE), AppEnableEvidence::EnableCanLift);
        assert!(role_is_not_running(ROLE), "and the episode opened");

        observe_role_app_status(ROLE, &AppRunObservation::Running);
        assert_eq!(
            last_enable_evidence(ROLE),
            AppEnableEvidence::AlreadyEnabled,
            "the status is recorded — that is how the STRANDED state becomes nameable"
        );
        assert!(
            role_is_not_running(ROLE),
            "...and it still proves nothing about health: the episode stands"
        );
    }

    // ---- F5 / the NEW finding: publication, driven through production -------

    /// A cell id for a role under test. Two halves so membership matching is
    /// the full `CellId` the production path compares, not a DNA prefix.
    fn publish_test_cell(seed: u8) -> holochain_client::CellId {
        use holochain_types::prelude::{AgentPubKey, DnaHash};
        holochain_client::CellId::new(
            DnaHash::from_raw_32(vec![seed; 32]),
            AgentPubKey::from_raw_32(vec![seed.wrapping_add(77); 32]),
        )
    }

    /// Drive the SUPERVISOR'S publication path — the one the per-role loop
    /// calls on every tick — against an injected membership source.
    ///
    /// Not a hand-rolled publisher: this is
    /// `HcClientRegistry::observe_and_publish_cell_state_from`, and the loop's
    /// own call site is four lines of binding over it. Deleting the
    /// publication from it fails every test below.
    async fn supervisor_tick(
        role: &str,
        src: &dyn crate::services::cell_membership::RunningCellSource,
        cell: &holochain_client::CellId,
        cache: &crate::services::cell_membership::MembershipCache,
        wall_ms: u64,
    ) -> RoleCellState {
        crate::hc_client_registry::HcClientRegistry::observe_and_publish_cell_state_from(
            role,
            src,
            Some(cell),
            cache,
            crate::services::cell_membership::ObservedAt::from(wall_ms),
        )
        .await
        .state
    }

    /// An observation in flight while a zome call LANDED must not republish the
    /// outage it was measuring.
    ///
    /// Driven through the production timestamp substitution, not by handing the
    /// publisher a literal `1`: the cached reading is taken 30 seconds before
    /// the recovery, the supervisor tick that publishes it happens AFTER the
    /// recovery, and the only thing that saves the gauges is the publication
    /// carrying the READING's own `at_ms` rather than the moment of the publish.
    #[tokio::test]
    async fn an_observation_older_than_a_recovery_is_dropped() {
        use crate::services::cell_membership::{fake::FakeAdmin, MembershipCache};
        const ROLE: &str = "test-stale-publish-dropped";
        let running = crate::metrics::CONDUCTOR_CELL_RUNNING.with_label_values(&[ROLE]);
        let stranded = crate::metrics::CONDUCTOR_CELL_STATE
            .with_label_values(&[ROLE, RoleCellState::InstalledNotRunningAppEnabled.as_str()]);

        let cell = publish_test_cell(31);
        let cache = MembershipCache::new();
        // The conductor's running map does NOT hold this role's cell.
        let admin = FakeAdmin::new(Ok(vec![publish_test_cell(32)]));

        // An outage, and `app_info` reporting the app ENABLED: the STRANDED shape.
        record_role_app_disabled(ROLE, CELL_DISABLED);
        observe_role_app_status(ROLE, &AppRunObservation::Running);
        assert_eq!(
            last_enable_evidence(ROLE),
            AppEnableEvidence::AlreadyEnabled
        );

        // A membership reading taken 10s ago — INSIDE the 15s refresh window, so
        // the tick that republishes it later will reuse it rather than re-read.
        let taken = now_ms() - 10_000;
        assert_eq!(
            supervisor_tick(ROLE, &admin, &cell, &cache, taken).await,
            RoleCellState::InstalledNotRunningAppEnabled
        );
        assert_eq!(stranded.get(), 1);
        assert_eq!(running.get(), 0);

        // A zome call LANDS — after the reading was taken.
        record_role_success(ROLE);
        assert_eq!(running.get(), 1);
        assert_eq!(stranded.get(), 0);

        // The next tick happens NOW — later than the recovery — and finds the
        // refresh window still covered, so it republishes the SAME cached
        // absence. That is the whole race: stamped with the TICK's clock the
        // publish is younger than the recovery and overwrites it; stamped with
        // the READING's own `at_ms` it is older, and dropped.
        supervisor_tick(ROLE, &admin, &cell, &cache, now_ms() + 1_000).await;
        assert_eq!(
            running.get(),
            1,
            "the recovered gauge survives a republished CACHED absence"
        );
        assert_eq!(stranded.get(), 0, "and so does the cleared state family");

        // A FRESH reading of a relapse — past the refresh window, so a real new
        // `ListCellIds` lands — is still honoured.
        supervisor_tick(ROLE, &admin, &cell, &cache, now_ms() + 16_000).await;
        assert_eq!(stranded.get(), 1, "a genuinely new observation still lands");
        assert_eq!(running.get(), 0);
    }

    /// F5's exact failing scenario, driven through the supervisor's publication
    /// path: a success publishes `1`; the conductor restarts; the role goes
    /// transport-DEAD; it reconnects to an Enabled app whose cell is ABSENT.
    ///
    /// No zome call has failed since the restart, so nothing opened a
    /// `CellDisabled` episode — and with publication gated on an active episode
    /// the gauge stayed at the `1` the much earlier success had set. It must now
    /// go to `0` off the membership reading alone, and the WHY-family must name
    /// which state it is in.
    #[tokio::test]
    async fn a_restart_lowers_the_gauge_without_any_zome_call_having_failed() {
        use crate::services::cell_membership::{fake::FakeAdmin, MembershipCache};
        const ROLE: &str = "test-gauge-after-restart-no-failed-call";
        let running = crate::metrics::CONDUCTOR_CELL_RUNNING.with_label_values(&[ROLE]);
        let stranded = crate::metrics::CONDUCTOR_CELL_STATE
            .with_label_values(&[ROLE, RoleCellState::InstalledNotRunningAppEnabled.as_str()]);

        // A call landed: the cell is in the running map, and the gauges say so.
        record_role_success(ROLE);
        assert_eq!(running.get(), 1);
        assert_eq!(stranded.get(), 0);

        // The conductor restarts. The role's bridge dies — a TRANSPORT failure,
        // which is not an `AppDisabled` episode.
        record_role_reconnect(ROLE);
        role_bridge_health().for_role(ROLE).record_failure();
        assert_eq!(
            role_bridge_health().for_role(ROLE).status(),
            ZomePathStatus::Dead
        );
        assert!(
            !role_is_not_running(ROLE),
            "precondition: NO CellDisabled episode is open — nothing has failed a zome call"
        );

        // It reconnects to an app the conductor reports ENABLED, whose cell is
        // absent from the running map.
        observe_role_app_status(ROLE, &AppRunObservation::Running);
        let cell = publish_test_cell(41);
        let cache = MembershipCache::new();
        let admin = FakeAdmin::new(Ok(vec![publish_test_cell(42)]));

        // The next supervisor tick reads membership and publishes it, gated on
        // no episode at all.
        let state = supervisor_tick(ROLE, &admin, &cell, &cache, now_ms() + 1_000).await;
        assert_eq!(state, RoleCellState::InstalledNotRunningAppEnabled);
        assert_eq!(
            running.get(),
            0,
            "the gauge must follow MEMBERSHIP, not wait for a zome call to fail"
        );
        assert_eq!(stranded.get(), 1, "and the state family names why");
    }

    /// THE NEW FINDING. A recovered, QUIET role's next tick reads a fresh
    /// `Some(true)` — and must NOT re-open the diagnostic family that recovery
    /// just cleared.
    ///
    /// `elohim_conductor_cell_state` answers "WHY is this role not serving". A
    /// serving role has no such reason. Publishing `running-recovery-unverified`
    /// unconditionally left that series at `1` forever on a healthy role,
    /// because with no episode open the supervisor schedules no probe to settle
    /// it — and wrote a log line promising one into an empty room.
    #[tokio::test]
    async fn a_recovered_quiet_role_keeps_its_cleared_state_family() {
        use crate::services::cell_membership::{fake::FakeAdmin, MembershipCache};
        const ROLE: &str = "test-quiet-role-family-stays-clear";
        let running = crate::metrics::CONDUCTOR_CELL_RUNNING.with_label_values(&[ROLE]);
        let family = |state: RoleCellState| {
            crate::metrics::CONDUCTOR_CELL_STATE.with_label_values(&[ROLE, state.as_str()])
        };

        // An outage, then a zome call that RETURNED: the one recovery transition.
        record_role_app_disabled(ROLE, CELL_DISABLED);
        record_role_success(ROLE);
        assert_eq!(running.get(), 1);
        for state in ROLE_CELL_STATES {
            assert_eq!(family(state).get(), 0, "recovery zeroes {}", state.as_str());
        }

        // The role is now quiet: no traffic, no failures. Its next tick reads a
        // fresh membership `true`.
        let cell = publish_test_cell(51);
        let cache = MembershipCache::new();
        let admin = FakeAdmin::new(Ok(vec![cell.clone()]));
        let state = supervisor_tick(ROLE, &admin, &cell, &cache, now_ms() + 1_000).await;
        assert_eq!(state, RoleCellState::RunningRecoveryUnverified);

        assert_eq!(running.get(), 1, "membership publishes, always");
        for state in ROLE_CELL_STATES {
            assert_eq!(
                family(state).get(),
                0,
                "{} was re-opened on a recovered, serving role — the WHY-family answers a \
                 question this role does not pose",
                state.as_str()
            );
        }
        assert!(
            !role_bridge_health().for_role(ROLE).has_cell_state(),
            "nothing was latched, so no transition line was logged — in particular not the one \
             promising a probe that no episode will schedule"
        );
        // ...and the publisher says so directly.
        assert!(
            !record_role_cell_state(
                ROLE,
                RoleCellState::RunningRecoveryUnverified,
                Some(true),
                AppEnableEvidence::Unknown,
                next_obs_seq(),
                now_ms(),
            ),
            "a serving role's WHY-family publish must report itself as not published"
        );

        // A RELAPSE still opens it: new failure evidence is the gate, not silence.
        record_role_app_disabled(ROLE, CELL_DISABLED);
        assert!(record_role_cell_state(
            ROLE,
            RoleCellState::RunningRecoveryUnverified,
            Some(true),
            AppEnableEvidence::Unknown,
            next_obs_seq(),
            now_ms(),
        ));
        assert_eq!(family(RoleCellState::RunningRecoveryUnverified).get(), 1);
    }

    // ---- A: ONE monotonic observation order, not two wall clocks -----------

    #[test]
    fn membership_publication_keeps_the_readings_episode_start_time() {
        const ROLE: &str = "test-membership-publisher-episode-clock";
        let observer = role_bridge_health().for_role(ROLE);
        let membership_seq = next_obs_seq();
        observer.record_app_disabled_at(T0 + 10_000, CELL_DISABLED);
        record_role_cell_state(
            ROLE,
            RoleCellState::InstalledNotRunningAppEnabled,
            Some(false),
            AppEnableEvidence::AlreadyEnabled,
            membership_seq,
            T0,
        );
        assert_eq!(observer.snapshot_at(T0 + 20_000).not_running_secs, Some(20));
    }

    /// THE TIE EPOCH-MS COULD NOT BREAK. An absence observed at ms 1000 and a
    /// success landing later WITHIN that same millisecond compared equal, and
    /// `last_success_ms > observed_at_ms` was false — so the republished
    /// absence restored an outage that had already ended.
    ///
    /// Driven with ONE frozen millisecond for every observation, so the only
    /// thing that can order them is the sequence.
    #[test]
    fn a_success_inside_the_same_millisecond_still_outranks_an_earlier_absence() {
        const ROLE: &str = "test-same-millisecond-tie";
        const FROZEN: u64 = 1_700_000_000_000;
        let running = crate::metrics::CONDUCTOR_CELL_RUNNING.with_label_values(&[ROLE]);
        let absent = RoleCellState::InstalledNotRunningAppEnabled;
        let family =
            crate::metrics::CONDUCTOR_CELL_STATE.with_label_values(&[ROLE, absent.as_str()]);

        // The absence is OBSERVED first…
        let absence_seq = next_obs_seq();
        record_role_app_disabled(ROLE, CELL_DISABLED);
        assert!(record_role_cell_state(
            ROLE,
            absent,
            Some(false),
            AppEnableEvidence::AlreadyEnabled,
            absence_seq,
            FROZEN,
        ));
        assert_eq!(family.get(), 1);

        // …then a call lands. Every stamp in this test shares one millisecond.
        role_bridge_health()
            .for_role(ROLE)
            .record_success_at(FROZEN);
        crate::metrics::set_conductor_cell_running(ROLE, Some(true));
        crate::metrics::clear_conductor_cell_state(ROLE);
        assert!(
            last_success_seq(ROLE) > absence_seq,
            "the success was observed after the absence, so it must RANK after it"
        );

        // The cached absence is republished, carrying its own (older) order.
        assert!(
            !record_role_cell_state(
                ROLE,
                absent,
                Some(false),
                AppEnableEvidence::AlreadyEnabled,
                absence_seq,
                T0,
            ),
            "an absence observed BEFORE the success must be dropped — with epoch-ms the two \
             compared equal and the absence won"
        );
        assert_eq!(running.get(), 1, "the recovery survives");
        assert_eq!(family.get(), 0);
    }

    /// A WALL CLOCK THAT STEPS BACKWARDS changes nothing: ordering is the
    /// counter, and the counter never goes down.
    #[test]
    fn a_backward_clock_step_does_not_invert_observation_order() {
        const ROLE: &str = "test-backward-clock-ordering";
        let running = crate::metrics::CONDUCTOR_CELL_RUNNING.with_label_values(&[ROLE]);
        let absent = RoleCellState::InstalledNotRunningAppEnabled;

        // A success at a LATE wall-clock stamp…
        let observer = role_bridge_health().for_role(ROLE);
        observer.record_success_at(2_000);
        let success_seq = last_success_seq(ROLE);

        // …then the clock jumps back a minute and an absence is observed.
        // Its wall stamp (1_000) is SMALLER, its order is LARGER.
        let absence_seq = next_obs_seq();
        assert!(absence_seq > success_seq);
        record_role_app_disabled(ROLE, CELL_DISABLED);
        assert!(
            record_role_cell_state(
                ROLE,
                absent,
                Some(false),
                AppEnableEvidence::AlreadyEnabled,
                absence_seq,
                T0,
            ),
            "a LATER observation must publish even though its wall-clock stamp went backwards"
        );
        assert_eq!(running.get(), 0);
        assert_eq!(
            observer.last_success_ms(),
            2_000,
            "and the ms stamps really are inverted — they are diagnostics, not ordering"
        );
    }

    /// F1's residual: `record_role_success` must RETIRE older status evidence.
    ///
    /// A role observed `Disabled`, then proven `Live` by a call that landed,
    /// could still be argued into `membership-unknown` by that stale `Disabled`
    /// answer — and the classifier never consulted the newer success.
    #[tokio::test]
    async fn a_success_retires_the_disabled_evidence_that_preceded_it() {
        use crate::services::cell_membership::{fake::FakeAdmin, MembershipCache};
        const ROLE: &str = "test-success-retires-stale-disabled-evidence";

        // A Disabled answer, then a call that RETURNS.
        observe_role_app_status(
            ROLE,
            &AppRunObservation::NotRunning {
                reason: "disabled: by an operator through the admin interface".to_string(),
                enable: AppEnableEvidence::EnableCanLift,
            },
        );
        let stale_seq = last_enable_evidence_seq(ROLE);
        assert_eq!(last_enable_evidence(ROLE), AppEnableEvidence::EnableCanLift);

        record_role_success(ROLE);
        assert_eq!(
            last_enable_evidence(ROLE),
            AppEnableEvidence::AlreadyEnabled,
            "a call that LANDED proves the cell is in the running map, so enable_app has nothing \
             left to do — the Disabled answer is retired, not merely outranked"
        );
        assert!(
            last_enable_evidence_seq(ROLE) > stale_seq,
            "and it is restamped at the success's own order"
        );
        assert!(
            !AppEnableEvidence::AlreadyEnabled.enable_may_help(),
            "so no rung can be spent off evidence the success outlived"
        );

        // A fresh membership `true` on the now-Live role classifies as PRESENT,
        // not as `membership-unknown` argued from the retired evidence.
        let cell = publish_test_cell(101);
        let cache = MembershipCache::new();
        let admin = FakeAdmin::new(Ok(vec![cell.clone()]));
        let state = supervisor_tick(ROLE, &admin, &cell, &cache, now_ms() + 1_000).await;
        assert_eq!(
            state,
            RoleCellState::RunningRecoveryUnverified,
            "a Live role must not be talked back into membership-unknown by retired evidence"
        );
    }

    /// THE NEW P2, AS ASTRA GAVE IT: `Live` → definitive absence → membership
    /// returns → a probe is owed → success clears the family.
    ///
    /// The r3 gate published `running=1` on the return tick and left
    /// `installed-not-running-app-enabled=1` latched, because no episode
    /// existed to schedule a recovery and `Live` short-circuited the family
    /// write. `ListCellIds` saying the cell is GONE is failure evidence in its
    /// own right; it does not have to wait for a zome call to fail first. So a
    /// definitive absence now OPENS the recovery episode, which is what earns
    /// the role a `ProbeNow` the moment membership returns.
    #[tokio::test]
    async fn a_definitive_absence_opens_an_episode_so_the_family_can_clear() {
        use crate::services::cell_membership::{fake::FakeAdmin, MembershipCache};
        const ROLE: &str = "test-absence-opens-a-recovery-episode";
        let running = crate::metrics::CONDUCTOR_CELL_RUNNING.with_label_values(&[ROLE]);
        let family = |state: RoleCellState| {
            crate::metrics::CONDUCTOR_CELL_STATE.with_label_values(&[ROLE, state.as_str()])
        };
        let absent = RoleCellState::InstalledNotRunningAppEnabled;

        let cell = publish_test_cell(81);
        let cache = MembershipCache::new();
        let admin = FakeAdmin::new(Ok(vec![cell.clone()]));

        // A call landed: Live, family cleared.
        record_role_success(ROLE);
        assert_eq!(running.get(), 1);
        assert!(!role_is_not_running(ROLE));

        // The conductor reports the app ENABLED while the cell leaves the map.
        observe_role_app_status(ROLE, &AppRunObservation::Running);
        admin.set_tail(Ok(vec![publish_test_cell(82)]));
        let state = supervisor_tick(ROLE, &admin, &cell, &cache, now_ms() + 1_000).await;
        assert_eq!(state, absent);
        assert_eq!(running.get(), 0, "membership says the cell is gone");
        assert_eq!(family(absent).get(), 1, "and the family names why");
        assert!(
            role_is_not_running(ROLE),
            "THE FIX: a definitive absence opens a recovery-required episode, so the supervisor \
             has something to schedule against. Without it the family latches on a role the \
             supervisor believes is fine, and nothing ever clears it."
        );

        // Membership RETURNS. The role is in an episode, so the family moves to
        // `running-recovery-unverified` and the probe is owed NOW.
        admin.set_tail(Ok(vec![cell.clone(), publish_test_cell(82)]));
        let state = supervisor_tick(ROLE, &admin, &cell, &cache, now_ms() + 20_000).await;
        assert_eq!(state, RoleCellState::RunningRecoveryUnverified);
        assert_eq!(running.get(), 1);
        assert_eq!(
            family(absent).get(),
            0,
            "the absence family must not survive membership returning"
        );
        assert_eq!(family(RoleCellState::RunningRecoveryUnverified).get(), 1);
        assert!(
            state.probe_is_owed_now()
                && crate::hc_client_registry::decide_not_running_action(
                    state,
                    last_enable_evidence(ROLE),
                    false,
                ) == crate::hc_client_registry::NotRunningAction::ProbeNow,
            "and a read-only probe is scheduled off the ladder"
        );

        // Only the probe's call RETURNING clears the family.
        record_role_success(ROLE);
        for state in ROLE_CELL_STATES {
            assert_eq!(
                family(state).get(),
                0,
                "{} survived an actual success",
                state.as_str()
            );
        }
        assert_eq!(running.get(), 1);
        assert!(!role_is_not_running(ROLE));
    }

    /// THE INVERSE, and it must stay red: membership returns but NO call ever
    /// succeeds. The family stays latched — honestly — because nothing has
    /// proven this role can serve.
    #[tokio::test]
    async fn without_a_success_the_family_stays_latched_honestly() {
        use crate::services::cell_membership::{fake::FakeAdmin, MembershipCache};
        const ROLE: &str = "test-absence-without-success-stays-latched";
        let family = |state: RoleCellState| {
            crate::metrics::CONDUCTOR_CELL_STATE.with_label_values(&[ROLE, state.as_str()])
        };

        let cell = publish_test_cell(91);
        let cache = MembershipCache::new();
        let admin = FakeAdmin::new(Ok(vec![publish_test_cell(92)]));

        record_role_success(ROLE);
        observe_role_app_status(ROLE, &AppRunObservation::Running);
        supervisor_tick(ROLE, &admin, &cell, &cache, now_ms() + 1_000).await;
        assert!(role_is_not_running(ROLE));

        // Membership returns; no probe result ever arrives.
        admin.set_tail(Ok(vec![cell.clone()]));
        for tick in 0..5u64 {
            let state =
                supervisor_tick(ROLE, &admin, &cell, &cache, now_ms() + 20_000 * (tick + 1)).await;
            assert_eq!(state, RoleCellState::RunningRecoveryUnverified);
            assert_eq!(
                family(RoleCellState::RunningRecoveryUnverified).get(),
                1,
                "tick {tick}: membership present is not recovery, and the family says so"
            );
            assert!(
                role_is_not_running(ROLE),
                "tick {tick}: the episode stands until a call RETURNS"
            );
        }
    }

    // ---- r5: the record is ONE coherent observation ------------------------

    /// NEW-1. The absence's episode transition happens in the SAME critical
    /// section as its acceptance, carrying its OWN sequence.
    ///
    /// The r4 cut passed the guarded check, released, and then opened the
    /// episode through a second call that minted a FRESH, higher sequence — so
    /// this schedule undid a recovery:
    ///
    /// ```text
    /// absence seq=10 accepted; opens_recovery_episode = true
    /// guard released
    /// success seq=11 → Live, family cleared
    /// deferred disabled write, fresh seq=12 → AppDisabled again
    /// ```
    ///
    /// Driven here by applying the absence and the success in that order and
    /// then re-presenting the absence at its ORIGINAL sequence, which is the
    /// only sequence a deferred write could honestly have carried.
    #[test]
    fn an_absence_cannot_reopen_an_episode_a_later_success_closed() {
        const ROLE: &str = "test-absence-cannot-reopen-after-success";
        let observer = role_bridge_health().for_role(ROLE);
        let absent = RoleCellState::InstalledNotRunningAppEnabled;

        let absence_seq = next_obs_seq();
        let applied = observer.apply(Observation::Membership {
            seq: absence_seq,
            at_ms: T0,
            running: Some(false),
            state: absent,
        });
        assert!(applied.applied);
        assert!(
            applied.opened_episode,
            "the absence opens the episode INSIDE apply, at its own sequence"
        );
        assert_eq!(
            applied.snapshot.last_disabled_seq, absence_seq,
            "and the disabled stream carries THAT sequence, not a fresh one"
        );

        // A call RETURNS.
        let success = observer.apply(Observation::Success {
            seq: next_obs_seq(),
            at_ms: T0 + 1,
        });
        assert!(success.applied);
        assert_eq!(success.snapshot.status(), ZomePathStatus::Live);

        // The same absence arrives late. It cannot mint a newer sequence,
        // because the transition is no longer a separate act.
        let late = observer.apply(Observation::Membership {
            seq: absence_seq,
            at_ms: T0,
            running: Some(false),
            state: absent,
        });
        assert!(!late.applied, "a superseded absence is DROPPED");
        assert_eq!(late.superseded_by, success.snapshot.last_success_seq);
        assert_eq!(
            observer.observe().status(),
            ZomePathStatus::Live,
            "the recovery stands — the deferred write used to undo it"
        );
    }

    /// NEW-2, first schedule: the evidence KIND and its SEQUENCE are one read.
    ///
    /// A classifier that loaded them separately could pair `EnableCanLift` from
    /// observation 10 with the sequence of the success that retired it, and then
    /// contradict a membership reading with a tuple no observation produced.
    #[test]
    fn the_evidence_kind_and_its_sequence_are_read_as_one() {
        const ROLE: &str = "test-evidence-tuple-is-coherent";
        let observer = role_bridge_health().for_role(ROLE);

        observer.apply(Observation::EnableEvidence {
            seq: next_obs_seq(),
            at_ms: T0,
            kind: AppEnableEvidence::EnableCanLift,
        });
        let before = observer.observe();
        assert_eq!(before.enable, AppEnableEvidence::EnableCanLift);

        // A success retires the evidence — kind AND sequence together.
        observer.apply(Observation::Success {
            seq: next_obs_seq(),
            at_ms: T0 + 1,
        });
        let after = observer.observe();
        assert_eq!(after.enable, AppEnableEvidence::AlreadyEnabled);
        assert_eq!(
            after.enable_seq, after.last_success_seq,
            "retired AT the success's own order, so the pair is never mixed"
        );
        assert!(after.enable_seq > before.enable_seq);
        assert!(
            !status_contradicts_cached_membership(
                Some(true),
                after.last_success_seq - 1,
                after.enable,
                after.enable_seq,
            ),
            "and the coherent pair cannot contradict anything — only a REAL \
             not-Enabled observation can"
        );
    }

    /// NEW-2, second schedule: a status writer that claimed an older sequence,
    /// paused, and then tried to overwrite a newer pair is REJECTED.
    #[test]
    fn a_paused_status_writer_cannot_overwrite_a_newer_pair() {
        const ROLE: &str = "test-paused-status-writer-rejected";
        let observer = role_bridge_health().for_role(ROLE);

        // The status writer claims its sequence…
        let stale_seq = next_obs_seq();
        // …and pauses. Meanwhile a success lands and retires the evidence.
        observer.apply(Observation::Success {
            seq: next_obs_seq(),
            at_ms: T0,
        });
        let after_success = observer.observe();
        assert_eq!(after_success.enable, AppEnableEvidence::AlreadyEnabled);

        // The writer resumes with its stale claim.
        let late = observer.apply(Observation::EnableEvidence {
            seq: stale_seq,
            at_ms: T0,
            kind: AppEnableEvidence::EnableCanLift,
        });
        assert!(
            !late.applied,
            "an older sequence may not overwrite the field"
        );
        assert_eq!(late.superseded_by, after_success.enable_seq);
        assert_eq!(
            observer.observe().enable,
            AppEnableEvidence::AlreadyEnabled,
            "the retired Disabled answer stays retired"
        );
    }

    /// Every observation kind rejects an older sequence on the field it owns,
    /// and says which sequence outranked it.
    #[test]
    fn every_observation_kind_rejects_a_superseded_write() {
        const ROLE: &str = "test-every-kind-rejects-superseded";
        let observer = role_bridge_health().for_role(ROLE);
        let stale = next_obs_seq();
        let fresh = next_obs_seq();

        for (name, newer, older) in [
            (
                "failure",
                Observation::Failure {
                    seq: fresh,
                    at_ms: T0,
                },
                Observation::Failure {
                    seq: stale,
                    at_ms: T0,
                },
            ),
            (
                "enable-evidence",
                Observation::EnableEvidence {
                    seq: fresh,
                    at_ms: T0,
                    kind: AppEnableEvidence::AlreadyEnabled,
                },
                Observation::EnableEvidence {
                    seq: stale,
                    at_ms: T0,
                    kind: AppEnableEvidence::EnableCanLift,
                },
            ),
        ] {
            assert!(observer.apply(newer).applied, "{name}: the newer one lands");
            let rejected = observer.apply(older);
            assert!(!rejected.applied, "{name}: the older one is DROPPED");
            assert_eq!(
                rejected.superseded_by, fresh,
                "{name}: and names its usurper"
            );
        }
    }

    // ---- r6: cross-kind ordering, and the schedules that split observations -

    /// The DECISION fields of a record — everything a classifier, a cure or a
    /// gauge reads.
    ///
    /// Deliberately EXCLUDES `consecutive_failures`, `last_responsive_seq`,
    /// `last_failure_seq` and `last_disabled_seq`. Every one of them is a stamp
    /// or a tally over observations that were ACTUALLY FOLDED IN, so a rejected
    /// observation contributing nothing is the mechanism working rather than a
    /// divergence. None is read by a classifier, a cure or a gauge: they reach
    /// the outside world only through `status()`, which IS compared. And because
    /// sequences come from one monotonic counter, any later observation outranks
    /// both histories, so a transient difference in these stamps can never flip
    /// a later generation check — the two records converge on the next write.
    ///
    /// `last_success_seq` is NOT in that class and IS compared: it is the
    /// served-cell ordering barrier that `record_role_cell_state` and the tick
    /// executor read directly.
    #[derive(Debug, PartialEq, Eq)]
    struct TerminalState {
        status: ZomePathStatus,
        last_success_seq: u64,
        enable: AppEnableEvidence,
        enable_seq: u64,
        episode_started_ms: u64,
        disabled_reason: Option<String>,
        membership: Option<(Option<bool>, u64)>,
        cell_state: Option<RoleCellState>,
    }

    fn terminal(record: &RoleObservation) -> TerminalState {
        TerminalState {
            status: record.status(),
            last_success_seq: record.last_success_seq,
            enable: record.enable,
            enable_seq: record.enable_seq,
            episode_started_ms: record.episode_started_ms,
            disabled_reason: record.disabled_reason.clone(),
            membership: record.membership,
            cell_state: record.cell_state,
        }
    }

    /// One observation of each kind, at `seq`, with a fixed payload so the
    /// matrix below compares ORDER and nothing else.
    fn observation_of(kind: &str, seq: u64) -> Observation {
        let at_ms = T0 + seq;
        match kind {
            "success" => Observation::Success { seq, at_ms },
            "responsive" => Observation::Responsive { seq, at_ms },
            "failure" => Observation::Failure { seq, at_ms },
            "disabled" => Observation::Disabled {
                seq,
                at_ms,
                reason: "CellDisabled (matrix)".to_string(),
            },
            "enable-evidence" => Observation::EnableEvidence {
                seq,
                at_ms,
                kind: AppEnableEvidence::EnableCanLift,
            },
            "status" => Observation::Status {
                seq,
                at_ms,
                evidence: AppEnableEvidence::EnableCanLift,
                not_running: Some("disabled: by an operator (matrix)".to_string()),
            },
            "membership" => Observation::Membership {
                seq,
                at_ms,
                running: Some(false),
                state: RoleCellState::InstalledNotRunningAppEnabled,
            },
            other => panic!("unknown observation kind {other}"),
        }
    }

    const OBSERVATION_KINDS: [&str; 7] = [
        "success",
        "responsive",
        "failure",
        "disabled",
        "enable-evidence",
        "status",
        "membership",
    ];

    /// THE ORDERING CONTRACT, RESTRICTED TO WHAT PRODUCTION GUARANTEES.
    ///
    /// Since arrival order IS sequence order for every in-process observation
    /// (`apply_on_arrival` mints under the publication guard), the only kind
    /// that can genuinely arrive out of order is `Membership` — its sequence is
    /// claimed before the `ListCellIds` request goes out, so the answer can land
    /// after observations sequenced later. Same-kind pairs are the other case
    /// worth pinning, because a wrapper could always regress its own stream.
    ///
    /// So the claim is exactly those two families, and no more. The previous
    /// cut asserted every 7×7 pair, which was BROADER than production: it
    /// implied out-of-order arrival was safe for kinds that can no longer
    /// arrive that way, and the r6 review found a pure-`apply` counterexample
    /// (`D10@100 → S11@200 → D12@300` vs `D10 → D12 → S11`) that production
    /// guards prevent but the unrestricted claim did not.
    #[test]
    fn a_membership_or_same_kind_observation_arriving_late_changes_nothing() {
        let pairs = OBSERVATION_KINDS.iter().flat_map(|newer| {
            OBSERVATION_KINDS
                .iter()
                .filter(move |older| **older == "membership" || *older == newer)
                .map(move |older| (*older, *newer))
        });
        for (older_kind, newer_kind) in pairs {
            let chronological = BridgeHealth::new();
            chronological.apply(observation_of(older_kind, 1));
            chronological.apply(observation_of(newer_kind, 2));

            let reversed = BridgeHealth::new();
            reversed.apply(observation_of(newer_kind, 2));
            reversed.apply(observation_of(older_kind, 1));

            assert_eq!(
                terminal(&reversed.observe()),
                terminal(&chronological.observe()),
                "older `{older_kind}`(1) arriving after newer `{newer_kind}`(2) left a \
                 DIFFERENT decision state than applying them in order"
            );
        }
    }

    /// ITEM 1'S INVARIANT, ASSERTED: for every observation this process makes
    /// about itself, arrival order IS sequence order.
    ///
    /// N threads hammer all five guarded wrappers on ONE role. The ticket and
    /// the minted sequence are both claimed inside the publication guard, so the
    /// log is the guard's own order — and the sequences in it must be strictly
    /// increasing. If a wrapper minted before taking the guard, two observations
    /// could be sequenced in one order and applied in the other, which is the
    /// shape every "older arrives after newer" schedule needs.
    #[test]
    fn arrival_order_is_sequence_order_for_every_in_process_observation() {
        const THREADS: usize = 8;
        const PER_THREAD: usize = 25;
        let observer = std::sync::Arc::new(BridgeHealth::new());

        std::thread::scope(|scope| {
            for t in 0..THREADS {
                let observer = std::sync::Arc::clone(&observer);
                scope.spawn(move || {
                    for i in 0..PER_THREAD {
                        let at_ms = T0 + (t * PER_THREAD + i) as u64;
                        match (t + i) % 5 {
                            0 => observer.record_success_at(at_ms),
                            1 => observer.record_failure_at(at_ms),
                            2 => {
                                observer.record_responsive_at(at_ms);
                            }
                            3 => {
                                observer.record_app_disabled_at(at_ms, CELL_DISABLED);
                            }
                            _ => {
                                observer.apply_on_arrival(|seq| Observation::Status {
                                    seq,
                                    at_ms,
                                    evidence: AppEnableEvidence::EnableCanLift,
                                    not_running: Some("status (hammer)".to_string()),
                                });
                            }
                        }
                    }
                });
            }
        });

        let arrivals = observer.arrivals();
        assert_eq!(arrivals.len(), THREADS * PER_THREAD);
        let mut sorted = arrivals.clone();
        sorted.sort_by_key(|(ticket, _)| *ticket);
        assert_eq!(
            sorted, arrivals,
            "the log is appended under the guard, so it is already in guard order"
        );
        for window in sorted.windows(2) {
            let ((prev_ticket, prev_seq), (ticket, seq)) = (window[0], window[1]);
            assert!(
                seq > prev_seq,
                "arrival {ticket} minted sequence {seq}, which does not outrank arrival \
                 {prev_ticket}'s {prev_seq} — a wrapper minted OUTSIDE the guard, and two \
                 in-process observations can be sequenced in one order and applied in the other"
            );
        }
    }

    /// THREE-OBSERVATION HISTORIES through the guarded wrappers, which the pair
    /// matrix cannot reach.
    ///
    /// Each is driven in both of the orders the wrappers can actually produce,
    /// and both must agree — because both ARE chronological order once the
    /// sequence is minted under the guard.
    #[test]
    fn three_observation_histories_through_the_wrappers_agree() {
        let histories: [(&str, &[&str]); 6] = [
            (
                "disabled→failure→responsive",
                &["disabled", "failure", "responsive"],
            ),
            (
                "disabled→responsive→failure",
                &["disabled", "responsive", "failure"],
            ),
            (
                "failure→responsive→disabled",
                &["failure", "responsive", "disabled"],
            ),
            (
                "disabled→responsive→success",
                &["disabled", "responsive", "success"],
            ),
            (
                "disabled→failure→success",
                &["disabled", "failure", "success"],
            ),
            (
                "success→failure→responsive",
                &["success", "failure", "responsive"],
            ),
        ];
        let expected: [(&str, ZomePathStatus); 6] = [
            ("disabled→failure→responsive", ZomePathStatus::AppDisabled),
            ("disabled→responsive→failure", ZomePathStatus::AppDisabled),
            ("failure→responsive→disabled", ZomePathStatus::AppDisabled),
            ("disabled→responsive→success", ZomePathStatus::Live),
            ("disabled→failure→success", ZomePathStatus::Live),
            ("success→failure→responsive", ZomePathStatus::Live),
        ];
        for ((name, steps), (_, want)) in histories.iter().zip(expected.iter()) {
            let observer = BridgeHealth::new();
            for (i, step) in steps.iter().enumerate() {
                let at_ms = T0 + i as u64;
                match *step {
                    "success" => observer.record_success_at(at_ms),
                    "failure" => observer.record_failure_at(at_ms),
                    "responsive" => {
                        observer.record_responsive_at(at_ms);
                    }
                    "disabled" => {
                        observer.record_app_disabled_at(at_ms, CELL_DISABLED);
                    }
                    other => panic!("unknown step {other}"),
                }
            }
            assert_eq!(
                observer.status(),
                *want,
                "{name}: the two verdict streams must agree on this history"
            );
        }
    }

    /// NEW-9: the two arrival orders of one three-observation history converge.
    ///
    /// ```text
    /// Disabled(9) → Failure(10) → Responsive(11)   →  AppDisabled
    /// Disabled(9) → Responsive(11) → Failure(10)   →  used to be Dead
    /// ```
    ///
    /// The second order used to discard the responsive answer (refused during an
    /// open episode), so it was not there to cancel the delayed failure. Both
    /// are now retained, and the served-cell stream owns the verdict either way.
    #[test]
    fn a_retained_responsive_cancels_a_delayed_failure_in_both_arrival_orders() {
        let chronological = BridgeHealth::new();
        chronological.apply(observation_of("disabled", 9));
        chronological.apply(observation_of("failure", 10));
        chronological.apply(observation_of("responsive", 11));

        let delayed_failure = BridgeHealth::new();
        delayed_failure.apply(observation_of("disabled", 9));
        delayed_failure.apply(observation_of("responsive", 11));
        delayed_failure.apply(observation_of("failure", 10));

        assert_eq!(chronological.status(), ZomePathStatus::AppDisabled);
        assert_eq!(
            delayed_failure.status(),
            ZomePathStatus::AppDisabled,
            "the same three observations must not produce two health verdicts"
        );
        assert_eq!(
            delayed_failure.observe().last_responsive_seq,
            11,
            "and the responsive answer is RETAINED during the episode — discarding it is what \
             left the delayed failure uncancelled"
        );
    }

    /// NEW-10: a transport failure, already cancelled by a newer responsive
    /// answer, must not suppress the recovery of a call that LANDED.
    ///
    /// ```text
    /// Disabled(10): app_enabled=0, episode + ladder populated
    /// Failure(12), Responsive(13): transport noise, cancelled
    /// Success(11) resumes  →  used to be Live with app_enabled=0 and the
    ///                          ladder inherited by the next outage
    /// ```
    #[test]
    fn transport_noise_does_not_suppress_a_landed_calls_recovery() {
        const ROLE: &str = "test-transport-noise-does-not-suppress-recovery";
        let roles = RoleBridgeHealth::new();
        let ledger = EnableLedger::new();
        let observer = roles.for_role(ROLE);

        observer.apply(observation_of("disabled", 10));
        ledger.note_attempt_at(ROLE, Instant::now());
        // Transport noise sequenced AFTER the success that is still in flight —
        // the r6 schedule exactly. The success carries 11; the failure 12 and
        // the responsive answer that cancels it 13.
        observer.apply(observation_of("failure", 12));
        observer.apply(observation_of("responsive", 13));

        let applied = observer.apply(Observation::Success {
            seq: 11,
            at_ms: T0 + 11,
        });
        assert!(applied.applied);
        assert!(
            applied.recovery_effects_apply,
            "a newer transport failure — itself already cancelled — is not a reason to disbelieve \
             a call that returned from the cell. Gating recovery on the transport stream left \
             `Live` with app_enabled=0, the family latched and the ladder inherited."
        );
        assert!(applied.recovered_from_not_running);
        assert_eq!(
            observer.status(),
            ZomePathStatus::Live,
            "the success outranks the disabled episode, so the served-cell verdict is Live"
        );

        // …and the wrapper that owns the ladder clears it whenever the effects
        // apply, which is what makes the gauges and the backoff agree.
        let outcome = {
            let observer = roles.for_role(ROLE);
            let publishing = observer.publish_guard();
            roles.record_success_on(ROLE, &publishing, &ledger, T0 + 14)
        };
        assert!(outcome.effects_apply);
        assert_eq!(
            ledger.attempts(ROLE),
            0,
            "the ladder is cleared — it used to be inherited by the next outage"
        );
    }

    /// The transport verdict is still REPORTED when nothing served-cell is
    /// newer: a failure after a success, with no episode open, reads `Dead`.
    #[test]
    fn a_failure_newer_than_a_success_with_no_episode_reads_dead() {
        let observer = BridgeHealth::new();
        observer.record_success_at(T0);
        assert_eq!(observer.status(), ZomePathStatus::Live);
        observer.record_failure_at(T0 + 1);
        assert_eq!(
            observer.status(),
            ZomePathStatus::Dead,
            "separating the streams must not make the transport verdict unreportable"
        );
        observer.record_responsive_at(T0 + 2);
        assert_eq!(
            observer.status(),
            ZomePathStatus::Live,
            "and a responsive answer cancels the older failure"
        );
    }

    /// NEW-5, as its schedule: one NotRunning `app_info` answer must not become
    /// two independently ordered observations.
    ///
    /// ```text
    /// NotRunning status publishes EnableCanLift, seq=10
    /// concurrent zome success, seq=11: clears the episode, retires the evidence
    /// the SAME status resumes  →  used to mint seq=12 and reopen recovery
    /// ```
    #[test]
    fn a_not_running_status_that_resumes_after_a_success_cannot_reopen_recovery() {
        let observer = BridgeHealth::new();
        let status_seq = next_obs_seq();

        // The success lands while the status answer is still in flight.
        let success = observer.apply(Observation::Success {
            seq: next_obs_seq(),
            at_ms: T0 + 1,
        });
        assert!(success.applied);
        assert_eq!(success.snapshot.status(), ZomePathStatus::Live);

        // The status answer resumes, WHOLE, carrying its own sequence.
        let late = observer.apply(Observation::Status {
            seq: status_seq,
            at_ms: T0,
            evidence: AppEnableEvidence::EnableCanLift,
            not_running: Some("disabled: by an operator through the admin interface".to_string()),
        });
        assert!(
            !late.applied,
            "a status answer older than a landed call is stale WHOLE"
        );
        assert_eq!(late.superseded_by, success.snapshot.last_success_seq);

        let after = observer.observe();
        assert_eq!(
            after.status(),
            ZomePathStatus::Live,
            "the stale half used to reopen recovery under a freshly minted sequence"
        );
        assert_eq!(
            after.enable,
            AppEnableEvidence::AlreadyEnabled,
            "and its evidence half could not resurrect EnableCanLift either"
        );
        assert_eq!(after.episode_started_ms, 0);
        assert_eq!(after.disabled_reason, None);
    }

    /// NEW-5 AT ITS CALLER: one `app_info` answer must consume exactly ONE
    /// sequence, so there is no gap for a success to land in.
    ///
    /// The schedule that reopened recovery is only reachable when the answer is
    /// TWO observations — evidence then Disabled, each minting its own stamp.
    /// The property that makes it unreachable is assertable directly: both
    /// halves of one answer carry the SAME sequence. The previous cut published
    /// them separately and the second half's freshly minted, higher stamp
    /// outranked the recovery that had landed between them.
    #[test]
    fn one_app_info_answer_consumes_exactly_one_sequence() {
        const ROLE: &str = "test-status-is-one-observation";
        let before = next_obs_seq();
        observe_role_app_status(
            ROLE,
            &AppRunObservation::NotRunning {
                reason: "disabled: by an operator through the admin interface".to_string(),
                enable: AppEnableEvidence::EnableCanLift,
            },
        );
        let after = role_bridge_health().for_role(ROLE).observe();

        assert_eq!(
            after.enable_seq, after.last_disabled_seq,
            "the evidence half and the not-running half of ONE app_info answer must share one \
             sequence — splitting them is what let a success land between them and be undone"
        );
        assert!(after.enable_seq > before);
        assert_eq!(after.enable, AppEnableEvidence::EnableCanLift);
        assert!(after.is_not_running());

        // And a `Running` answer is the same single observation, with no
        // not-running half at all.
        const RUNNING_ROLE: &str = "test-status-running-is-one-observation";
        observe_role_app_status(RUNNING_ROLE, &AppRunObservation::Running);
        let running = role_bridge_health().for_role(RUNNING_ROLE).observe();
        assert_eq!(running.enable, AppEnableEvidence::AlreadyEnabled);
        assert_eq!(
            running.last_disabled_seq, 0,
            "a Running answer makes no not-running claim"
        );
    }

    /// NEW-6, as its schedule: a paused success must not regress fields a NEWER
    /// observation owns.
    ///
    /// ```text
    /// success claims seq=11, pauses before the state lock
    /// status publishes EnableCanLift + Disabled at seq=12
    /// success resumes
    /// ```
    ///
    /// The call DID land, so `last_success_seq` records it. What it may not do
    /// is zero an episode clock, drop the conductor's reason, or regress the
    /// evidence to `(AlreadyEnabled, 11)` — which, with membership absent, then
    /// chose `ObserveOnly` and stopped enabling until a fresh status repaired it.
    #[test]
    fn a_paused_success_does_not_regress_fields_a_newer_observation_owns() {
        let observer = BridgeHealth::new();
        let success_seq = next_obs_seq();

        let status = observer.apply(Observation::Status {
            seq: next_obs_seq(),
            at_ms: T0 + 5,
            evidence: AppEnableEvidence::EnableCanLift,
            not_running: Some("disabled: by an operator through the admin interface".to_string()),
        });
        assert!(status.applied && status.opened_episode);

        let late = observer.apply(Observation::Success {
            seq: success_seq,
            at_ms: T0,
        });
        assert!(
            late.applied,
            "the call landed — that is a fact and it is recorded"
        );
        assert!(
            !late.recovery_effects_apply,
            "but a NEWER outage owns this record, so no recovery may be published"
        );
        assert!(!late.recovered_from_not_running);

        let after = observer.observe();
        assert_eq!(after.last_success_seq, success_seq, "recorded");
        assert_eq!(
            after.status(),
            ZomePathStatus::AppDisabled,
            "the newer not-running verdict stands"
        );
        assert_eq!(
            after.enable,
            AppEnableEvidence::EnableCanLift,
            "the newer evidence is NOT regressed to AlreadyEnabled"
        );
        assert_eq!(after.enable_seq, status.snapshot.enable_seq);
        assert_eq!(
            after.episode_started_ms,
            T0 + 5,
            "the episode clock is not zeroed by a success the outage outran"
        );
        assert!(
            after.disabled_reason.is_some(),
            "and the conductor's own reason survives"
        );
    }

    /// NEW-7, schedule two: membership establishing the outage must lower
    /// `app_enabled`, not leave three gauges disagreeing about one role.
    ///
    /// Needs no concurrency at all: success, then a newer membership-absence.
    #[tokio::test]
    async fn membership_absence_lowers_the_app_enabled_gauge() {
        use crate::services::cell_membership::{fake::FakeAdmin, MembershipCache};
        const ROLE: &str = "test-absence-lowers-app-enabled";
        let app_enabled = crate::metrics::CONDUCTOR_APP_ENABLED.with_label_values(&[ROLE]);
        let running = crate::metrics::CONDUCTOR_CELL_RUNNING.with_label_values(&[ROLE]);

        record_role_success(ROLE);
        assert_eq!(app_enabled.get(), 1);
        assert_eq!(running.get(), 1);

        observe_role_app_status(ROLE, &AppRunObservation::Running);
        let cell = publish_test_cell(111);
        let cache = MembershipCache::new();
        let admin = FakeAdmin::new(Ok(vec![publish_test_cell(112)]));
        let state = supervisor_tick(ROLE, &admin, &cell, &cache, now_ms() + 1_000).await;

        assert_eq!(state, RoleCellState::InstalledNotRunningAppEnabled);
        assert!(role_is_not_running(ROLE));
        assert_eq!(running.get(), 0);
        assert_eq!(
            app_enabled.get(),
            0,
            "three gauges must not disagree about one role: AppDisabled with cell_running=0 and \
             app_enabled=1 was the shape the deferred Disabled call used to prevent"
        );
    }

    /// NEW-7, schedule one: a `Disabled` gauge write must not resume after a
    /// success has published recovery.
    #[test]
    fn a_disabled_gauge_write_cannot_resume_after_recovery() {
        const ROLE: &str = "test-disabled-gauge-cannot-outlive-recovery";
        let app_enabled = crate::metrics::CONDUCTOR_APP_ENABLED.with_label_values(&[ROLE]);

        record_role_app_disabled(ROLE, CELL_DISABLED);
        assert_eq!(app_enabled.get(), 0);
        record_role_success(ROLE);
        assert_eq!(app_enabled.get(), 1);

        // The same stale Disabled answer arrives again. Its apply is rejected
        // under the publish guard, so its gauge write never happens.
        record_role_app_disabled_at_seq(ROLE, CELL_DISABLED, 1);
        assert_eq!(
            app_enabled.get(),
            1,
            "an unguarded gauge write used to leave a Live role reading app_enabled=0 forever"
        );
        assert!(!role_is_not_running(ROLE));
    }

    /// Item 4: a newer RESPONSIVE answer must not reject a delayed genuine
    /// success. While the two shared `last_success_seq`, transport
    /// responsiveness was the ordering barrier for served-cell evidence.
    #[test]
    fn a_newer_responsive_does_not_reject_a_delayed_success() {
        let observer = BridgeHealth::new();
        let success_seq = next_obs_seq();

        let responsive = observer.apply(Observation::Responsive {
            seq: next_obs_seq(),
            at_ms: T0 + 1,
        });
        assert!(responsive.applied);

        let delayed = observer.apply(Observation::Success {
            seq: success_seq,
            at_ms: T0,
        });
        assert!(
            delayed.applied,
            "a transport-responsive answer is not served-cell evidence and may not outrank one"
        );
        let after = observer.observe();
        assert_eq!(after.last_success_seq, success_seq);
        assert_eq!(
            after.enable,
            AppEnableEvidence::AlreadyEnabled,
            "and the success still retires the status evidence"
        );
        assert_eq!(after.status(), ZomePathStatus::Live);
    }

    /// The two streams stay separate, and the verdict rule is exactly
    /// "responsive cancels a failure, never a disabled".
    #[test]
    fn responsive_cancels_a_failure_and_never_a_disabled() {
        let cleared = BridgeHealth::new();
        cleared.apply(Observation::Failure {
            seq: next_obs_seq(),
            at_ms: T0,
        });
        assert_eq!(cleared.status(), ZomePathStatus::Dead);
        cleared.apply(Observation::Responsive {
            seq: next_obs_seq(),
            at_ms: T0 + 1,
        });
        assert_eq!(cleared.status(), ZomePathStatus::Live);
        assert_eq!(
            cleared.observe().last_success_seq,
            0,
            "and it did NOT write the served-cell stream"
        );

        // Both arrival orders of disabled/responsive end AppDisabled.
        for reversed in [false, true] {
            let h = BridgeHealth::new();
            let disabled = Observation::Disabled {
                seq: if reversed { 1 } else { 2 },
                at_ms: T0,
                reason: CELL_DISABLED.to_string(),
            };
            let resp = Observation::Responsive {
                seq: if reversed { 2 } else { 1 },
                at_ms: T0,
            };
            if reversed {
                h.apply(resp);
                h.apply(disabled);
            } else {
                h.apply(disabled);
                h.apply(resp);
            }
            assert_eq!(
                h.status(),
                ZomePathStatus::AppDisabled,
                "reversed={reversed}: a responsive answer never lifts a not-running verdict"
            );
        }
    }

    // ---- F1: newer status evidence beats an older cached membership ---------

    /// The bounded case the review named: disable the app WITHOUT closing its
    /// sockets, keep a cached membership `true`, and let every later
    /// `ListCellIds` fail. A cached `true` answered `ProbeNow` and suppressed
    /// the enable until the 60s authority window expired.
    ///
    /// The conductor's own ordering settles it: `disable_app` removes an app's
    /// cells and awaits their cleanup BEFORE it writes `Disabled`. So a
    /// not-Enabled status observed AFTER the reading proves the reading stale.
    #[test]
    fn newer_not_enabled_status_contradicts_an_older_cached_membership_true() {
        use AppEnableEvidence::*;
        let rows = [
            // (membership, reading SEQ, evidence, evidence SEQ) -> contradicted?
            (Some(true), 10, EnableCanLift, 11, true),
            (Some(true), 10, EnableRefused, 11, true),
            // Older status evidence settles nothing — the reading is younger.
            (Some(true), 11, EnableCanLift, 10, false),
            // An Enabled app AGREES with a running cell.
            (Some(true), 10, AlreadyEnabled, 11, false),
            // No status evidence establishes nothing at all.
            (Some(true), 10, Unknown, 11, false),
            // Nothing to contradict.
            (Some(false), 10, EnableCanLift, 11, false),
            (None, 10, EnableCanLift, 11, false),
            // ADJACENT sequences still order strictly — the epoch-ms version
            // compared two observations inside one millisecond as EQUAL and
            // let the older one win.
            (Some(true), 1, EnableCanLift, 2, true),
        ];
        for (membership, seq, enable, enable_seq, expected) in rows {
            assert_eq!(
                status_contradicts_cached_membership(membership, seq, enable, enable_seq),
                expected,
                "membership={membership:?}#{seq} enable={}#{enable_seq}",
                enable.as_str()
            );
        }
    }

    /// THE BOUNDED CASE, END TO END THROUGH THE SUPERVISOR'S OWN PUBLICATION
    /// PATH: disable the app without closing its sockets, keep a cached
    /// membership `true`, and let the membership read stop refreshing.
    ///
    /// Before the contradiction gate this published
    /// `running-recovery-unverified` and answered `ProbeNow`, suppressing the
    /// enable for as long as the cached reading held authority. The 60s TTL
    /// bounds that window; it does not cure it, and the review was right that
    /// bounding authority is not the same as bounding recovery latency.
    #[tokio::test]
    async fn a_cached_membership_true_loses_to_a_newer_not_enabled_status() {
        use crate::services::cell_membership::{fake::FakeAdmin, MembershipCache};
        const ROLE: &str = "test-cached-true-loses-to-newer-status";
        let running = crate::metrics::CONDUCTOR_CELL_RUNNING.with_label_values(&[ROLE]);

        let cell = publish_test_cell(61);
        let cache = MembershipCache::new();
        // The conductor's running map DID hold the cell when it was last read.
        let admin = FakeAdmin::new(Ok(vec![cell.clone()]));
        let taken = now_ms() - 5_000;
        cache.refresh_if_stale_at(&admin, taken.into()).await;
        let reading_seq = cache.last_attempt_seq();
        assert_eq!(
            cache.cell_running_at(&cell, (taken + 1_000).into()),
            Some(true),
            "precondition: the cache holds a `running` reading"
        );

        // NOW the app is disabled — sockets intact, so nothing re-mints the
        // bridge and nothing revokes the reading. `app_info` answers Disabled,
        // stamped LATER than the reading.
        observe_role_app_status(
            ROLE,
            &AppRunObservation::NotRunning {
                reason: "disabled: by an operator through the admin interface".to_string(),
                enable: AppEnableEvidence::EnableCanLift,
            },
        );
        assert!(
            last_enable_evidence_seq(ROLE) > reading_seq,
            "precondition: the status evidence is OBSERVED AFTER the reading (order, not ms)"
        );

        // A tick inside the refresh window, so the CACHED `true` is the only
        // membership answer available.
        let state = supervisor_tick(ROLE, &admin, &cell, &cache, taken + 2_000).await;
        assert_eq!(
            state,
            RoleCellState::MembershipUnknown,
            "a cached `running` older than a not-Enabled status is no longer evidence"
        );
        assert_eq!(
            running.get(),
            -1,
            "and the gauge says NOT ESTABLISHED, never a fabricated 0 or a believed 1"
        );
        assert_eq!(
            crate::hc_client_registry::decide_not_running_action(
                state,
                AppEnableEvidence::EnableCanLift,
                true,
            ),
            crate::hc_client_registry::NotRunningAction::SpendRungEnableAndProbe,
            "which is what puts the enable back on the ladder instead of probing a cell the \
             conductor has already torn down"
        );
    }

    /// And the DEMOTION is to UNKNOWN, never to a fabricated `false`: what is
    /// established is that the cached answer stopped being evidence. Unknown is
    /// what keeps the ladder running.
    #[test]
    fn a_contradicted_membership_becomes_unknown_and_the_ladder_runs() {
        let state = classify_cell_state(None, AppEnableEvidence::EnableCanLift);
        assert_eq!(state, RoleCellState::MembershipUnknown);
        assert!(
            !state.membership_proves_absent(),
            "a demoted reading is not a proof of absence"
        );
        assert!(
            !state.probe_is_owed_now(),
            "and it no longer answers ProbeNow, which is what suppressed the enable"
        );
        assert_eq!(
            crate::hc_client_registry::decide_not_running_action(
                state,
                AppEnableEvidence::EnableCanLift,
                true
            ),
            crate::hc_client_registry::NotRunningAction::SpendRungEnableAndProbe
        );
    }

    // ---- the episode clock does not reset (2026-09-22) ---------------------
    // ---- the episode clock does not reset (2026-09-22) ---------------------

    /// THE MEASURED DEFECT. `not_running_secs` used to be restamped whenever the
    /// CURRENT verdict had moved off `AppDisabled` and back — and the bridge
    /// supervisor produces exactly that shape on every re-mint. Susan reported
    /// `not_running_secs: 98` on a ~108-minute episode, which reads as "still
    /// starting, wait" at the moment the honest number says "intervene".
    #[test]
    fn a_transport_blip_inside_an_episode_does_not_restart_the_clock() {
        let h = BridgeHealth::new();
        h.record_app_disabled_at(T0, CELL_DISABLED);

        // ~108 minutes of episode, punctuated by transport deaths and re-mints.
        for minute in [10_u64, 30, 55, 80, 107] {
            let at = T0 + minute * 60_000;
            h.record_failure_at(at);
            assert_eq!(
                h.snapshot_at(at).status,
                ZomePathStatus::AppDisabled,
                "minute {minute}: the websocket really did die, and the transport stream records \
                 it — but a blip does not change the fact that this cell is not serving, so the \
                 open episode keeps the verdict"
            );
            assert_eq!(
                h.observe().last_failure_ms,
                at,
                "minute {minute}: …and the transport observation is recorded on its own stream"
            );
            assert_eq!(
                h.snapshot_at(at).not_running_secs,
                Some(minute * 60),
                "minute {minute}: a Dead interlude must not HIDE the episode clock either"
            );
            h.record_app_disabled_at(at + 2_000, CELL_DISABLED);
        }

        let snap = h.snapshot_at(T0 + 108 * 60_000);
        assert_eq!(snap.status, ZomePathStatus::AppDisabled);
        assert_eq!(
            snap.not_running_secs,
            Some(108 * 60),
            "the episode is 108 minutes old and must say so — the reset reported 98 seconds"
        );
    }

    /// The clock is cleared by ONE thing, and it is the same one thing that ends
    /// the episode: a zome call that returned.
    #[test]
    fn only_a_successful_call_clears_the_episode_clock() {
        let h = BridgeHealth::new();
        h.record_app_disabled_at(T0, CELL_DISABLED);
        // Neither a responsive domain error (refused while disabled) nor a
        // transport death nor a re-observation may clear it.
        h.observe_zome_error("Zome call failed: ZomeNotFound: content_store");
        h.record_failure_at(T0 + 1_000);
        h.record_app_disabled_at(T0 + 2_000, CELL_DISABLED);
        assert_eq!(h.snapshot_at(T0 + 600_000).not_running_secs, Some(600));

        h.record_success_at(T0 + 601_000);
        assert_eq!(h.snapshot_at(T0 + 601_000).not_running_secs, None);

        // And a RELAPSE starts a fresh clock from its own first observation.
        h.record_app_disabled_at(T0 + 700_000, CELL_DISABLED);
        assert_eq!(h.snapshot_at(T0 + 760_000).not_running_secs, Some(60));
    }

    /// The recovery line's cost figures come off the same clock, so the fix
    /// reaches the sentence an operator greps for.
    #[test]
    fn the_recovery_line_reports_the_whole_episode_across_a_blip() {
        let roles = RoleBridgeHealth::new();
        let ledger = EnableLedger::new();
        roles.record_app_disabled_on("lamad", CELL_DISABLED, T0);
        roles.for_role("lamad").record_failure_at(T0 + 60_000);
        roles.record_app_disabled_on("lamad", CELL_DISABLED, T0 + 62_000);

        let outcome = {
            let observer = roles.for_role("lamad");
            let publishing = observer.publish_guard();
            roles.record_success_on("lamad", &publishing, &ledger, T0 + 660_000)
        };
        assert!(outcome.recovered_from_not_running);
        assert_eq!(
            outcome.not_running_secs, 660,
            "the whole outage, not the segment since the last blip"
        );
    }
}
