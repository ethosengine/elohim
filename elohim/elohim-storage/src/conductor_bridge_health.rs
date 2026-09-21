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

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

use crate::services::enable_app_backoff::{enable_ledger, EnableLedger};

/// Wall-clock milliseconds since the epoch, saturating at 0 on a pre-epoch
/// clock. Only used to stamp observations; all policy is a function of
/// differences, so a clock step degrades an age, never a status.
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
    NotRunning { reason: String },
}

/// Classify an installed app's status into what it proves about the path.
///
/// Only [`AppStatus::Enabled`] is running. Every other variant — disabled for
/// any reason, awaiting memproofs, awaiting restore, unrecoverable — means zome
/// calls will not land, and each carries its own reason forward verbatim.
pub fn classify_app_status(status: &holochain_types::app::AppStatus) -> AppRunObservation {
    use holochain_types::app::{AppStatus, DisabledAppReason};
    let reason = match status {
        AppStatus::Enabled => return AppRunObservation::Running,
        AppStatus::Disabled(DisabledAppReason::NeverStarted) => {
            "disabled: never started since install".to_string()
        }
        AppStatus::Disabled(DisabledAppReason::NotStartedAfterProvidingMemproofs) => {
            "disabled: memproofs provided but the app was never started".to_string()
        }
        AppStatus::Disabled(DisabledAppReason::User) => {
            "disabled: by an operator through the admin interface".to_string()
        }
        AppStatus::Disabled(DisabledAppReason::Error(e)) => {
            format!("disabled: the conductor disabled it on an error: {e}")
        }
        AppStatus::AwaitingMemproofs => {
            "not running: awaiting memproofs — genesis has not completed".to_string()
        }
        AppStatus::AwaitingRestore => {
            "not running: awaiting restore — zome calls are rejected until every cell restores"
                .to_string()
        }
        AppStatus::Unrecoverable(cell_id, why) => format!(
            "not running: UNRECOVERABLE on cell {cell_id:?} ({why:?}) — terminal, enable_app \
             cannot lift it"
        ),
    };
    AppRunObservation::NotRunning { reason }
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
#[derive(Debug, Default)]
pub struct BridgeHealth {
    /// Epoch-ms of the last live observation; 0 = never. AGE only.
    last_success_ms: AtomicU64,
    /// Epoch-ms of the last transport failure; 0 = never. AGE only.
    last_failure_ms: AtomicU64,
    /// Monotonic sequence stamped on each observation. The STATUS is decided by
    /// these, never by the millisecond stamps: a failure and the recovery that
    /// follows it land inside the same millisecond routinely (`record_failure`
    /// then `record_success` in a supervisor tick measured identical ms), and
    /// comparing equal stamps silently reports the OLDER observation. Ordering
    /// is the thing that matters, so ordering is what we store.
    seq: AtomicU64,
    last_success_seq: AtomicU64,
    last_failure_seq: AtomicU64,
    /// Epoch-ms / sequence of the last observation that the APP IS NOT RUNNING.
    /// A third stream rather than a flag on the failure stream, because the two
    /// have different cures and an operator must be able to tell them apart.
    last_disabled_ms: AtomicU64,
    last_disabled_seq: AtomicU64,
    /// The conductor's own reason for the most recent disabled observation.
    /// Cleared the moment the path is observed live again, so the surface never
    /// shows a reason for a state the node is no longer in.
    disabled_reason: std::sync::RwLock<Option<String>>,
    /// Epoch-ms of the observation that STARTED the current not-running
    /// episode; 0 = not in one. Distinct from `last_disabled_ms` (the most
    /// recent observation) because the honest recovery line reports how long
    /// the role was down, and every failing call restamps the latter.
    episode_started_ms: AtomicU64,
    /// Has the "the app is enabled and its CELLS are not running" diagnosis
    /// already been said for the current episode? One line per episode: the
    /// sentence is the whole diagnosis, and a sentence repeated every 20s is
    /// how the one line that matters gets buried under copies of itself.
    diagnosis_said: AtomicBool,
    consecutive_failures: AtomicU32,
    reconnects: AtomicU64,
}

impl BridgeHealth {
    pub fn new() -> Self {
        Self::default()
    }

    /// Next observation sequence. `SeqCst` so two threads observing the bridge
    /// concurrently agree on which observation was last.
    fn next_seq(&self) -> u64 {
        self.seq.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// The CURRENT verdict, without building a whole snapshot.
    ///
    /// Three observation streams, one verdict: the LATEST observation wins,
    /// decided by sequence. `0` means "never observed" and can never win, which
    /// reproduces the original two-stream table exactly — (0,0) is Unknown, a
    /// lone stream is its own status, and a later failure beats an earlier
    /// success — while admitting the third stream on equal terms.
    pub fn status(&self) -> ZomePathStatus {
        let candidates = [
            (
                self.last_success_seq.load(Ordering::SeqCst),
                ZomePathStatus::Live,
            ),
            (
                self.last_failure_seq.load(Ordering::SeqCst),
                ZomePathStatus::Dead,
            ),
            (
                self.last_disabled_seq.load(Ordering::SeqCst),
                ZomePathStatus::AppDisabled,
            ),
        ];
        candidates
            .iter()
            .filter(|(seq, _)| *seq > 0)
            .max_by_key(|(seq, _)| *seq)
            .map(|(_, status)| *status)
            .unwrap_or(ZomePathStatus::Unknown)
    }

    /// Record evidence the path is live, stamped at `at_ms`. This is the ONLY
    /// proof of recovery the node accepts, so it is also the only thing that
    /// ends a not-running episode.
    pub fn record_success_at(&self, at_ms: u64) {
        let seq = self.next_seq();
        self.last_success_ms.store(at_ms, Ordering::Relaxed);
        self.last_success_seq.store(seq, Ordering::SeqCst);
        self.consecutive_failures.store(0, Ordering::Relaxed);
        // The episode is over: forget when it started and re-arm the diagnosis
        // so a LATER episode is diagnosed on its own terms rather than being
        // silenced by a sentence said about a different outage.
        self.episode_started_ms.store(0, Ordering::Relaxed);
        self.diagnosis_said.store(false, Ordering::SeqCst);
        // A live path means the app is running; a stale reason would outlive
        // the state it describes.
        *self
            .disabled_reason
            .write()
            .unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// Record evidence the APP IS NOT RUNNING, stamped at `at_ms`, keeping the
    /// conductor's own `reason` verbatim.
    ///
    /// Returns `true` when this observation STARTED an episode (the previous
    /// verdict was anything but [`ZomePathStatus::AppDisabled`]) — the one
    /// moment that earns a WARN. Every subsequent failing call re-observes the
    /// same episode and returns `false`, because a busy node discovers this
    /// state thousands of times a minute.
    pub fn record_app_disabled_at(&self, at_ms: u64, reason: &str) -> bool {
        let new_episode = self.status() != ZomePathStatus::AppDisabled;
        let seq = self.next_seq();
        self.last_disabled_ms.store(at_ms, Ordering::Relaxed);
        self.last_disabled_seq.store(seq, Ordering::SeqCst);
        if new_episode {
            self.episode_started_ms.store(at_ms, Ordering::Relaxed);
            self.diagnosis_said.store(false, Ordering::SeqCst);
        }
        *self
            .disabled_reason
            .write()
            .unwrap_or_else(|e| e.into_inner()) = Some(reason.to_string());
        new_episode
    }

    /// [`Self::record_app_disabled_at`] against the wall clock.
    pub fn record_app_disabled(&self, reason: &str) -> bool {
        self.record_app_disabled_at(now_ms(), reason)
    }

    /// Claim the right to say, ONCE for this episode, that the conductor
    /// reports the app enabled while its cells are not running.
    ///
    /// `false` when the role is not currently observed not-running (there is
    /// nothing to diagnose) and `false` for every call after the first within
    /// one episode. A `compare_exchange` rather than a read-then-write so two
    /// threads observing the same conductor cannot both win.
    pub fn claim_diagnosis(&self) -> bool {
        if self.status() != ZomePathStatus::AppDisabled {
            return false;
        }
        self.diagnosis_said
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// Record evidence the conductor ANSWERED — the transport is responsive.
    /// NOT evidence that this role's cell served the call.
    ///
    /// Returns `true` when the observation was folded, `false` when it was
    /// REFUSED because the role is in a not-running episode. That refusal is the
    /// whole point: a cell absent from `running_cells` still answers domain
    /// errors, and folding one as a success is what let a FAILED call end an
    /// episode, raise no gauge, and strand the enable ladder for the next outage.
    ///
    /// Clears a `Dead` verdict (the websocket is demonstrably back) and the
    /// failure streak. Deliberately touches NONE of: `episode_started_ms`,
    /// `diagnosis_said`, `disabled_reason`, the
    /// `elohim_conductor_app_enabled` gauge, the enable ladder. Those five
    /// belong to [`Self::record_success_at`] / [`record_role_success`] — the one
    /// transition.
    pub fn record_responsive_at(&self, at_ms: u64) -> bool {
        if self.status() == ZomePathStatus::AppDisabled {
            return false;
        }
        let seq = self.next_seq();
        self.last_success_ms.store(at_ms, Ordering::Relaxed);
        self.last_success_seq.store(seq, Ordering::SeqCst);
        self.consecutive_failures.store(0, Ordering::Relaxed);
        true
    }

    /// [`Self::record_responsive_at`] against the wall clock.
    pub fn record_responsive(&self) -> bool {
        self.record_responsive_at(now_ms())
    }

    /// Record evidence the path is dead, stamped at `at_ms`.
    pub fn record_failure_at(&self, at_ms: u64) {
        let seq = self.next_seq();
        self.last_failure_ms.store(at_ms, Ordering::Relaxed);
        self.last_failure_seq.store(seq, Ordering::SeqCst);
        self.consecutive_failures.fetch_add(1, Ordering::Relaxed);
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

    /// Fold the counters into a status as of `now_ms`.
    ///
    /// Total by construction: the status is decided by the OBSERVATION ORDER
    /// (`seq`), never by the millisecond stamps, so neither a same-millisecond
    /// pair nor a clock that steps backwards can invert it. The ms stamps feed
    /// only the ages, which saturate at 0 rather than wrapping.
    pub fn snapshot_at(&self, now_ms: u64) -> BridgeHealthSnapshot {
        let success = self.last_success_ms.load(Ordering::Relaxed);
        let failure = self.last_failure_ms.load(Ordering::Relaxed);
        let status = self.status();
        let episode_started = self.episode_started_ms.load(Ordering::Relaxed);
        let age = |stamp: u64| {
            if stamp == 0 {
                None
            } else {
                Some(now_ms.saturating_sub(stamp) / 1000)
            }
        };
        BridgeHealthSnapshot {
            status,
            last_success_age_secs: age(success),
            last_failure_age_secs: age(failure),
            consecutive_failures: self.consecutive_failures.load(Ordering::Relaxed),
            reconnects: self.reconnects.load(Ordering::Relaxed),
            // Only reported for the state it explains — a reason beside a `live`
            // verdict would read as a live node that is somehow also disabled.
            app_disabled_reason: (status == ZomePathStatus::AppDisabled)
                .then(|| {
                    self.disabled_reason
                        .read()
                        .unwrap_or_else(|e| e.into_inner())
                        .clone()
                })
                .flatten(),
            not_running_secs: (status == ZomePathStatus::AppDisabled && episode_started > 0)
                .then(|| now_ms.saturating_sub(episode_started) / 1000),
        }
    }

    /// [`Self::snapshot_at`] against the wall clock.
    pub fn snapshot(&self) -> BridgeHealthSnapshot {
        self.snapshot_at(now_ms())
    }

    /// Fold a zome-call error into this observer, classifying it first.
    ///
    /// An ERROR can never end a not-running episode: the responsive arm goes to
    /// [`Self::record_responsive`], which refuses while the role is
    /// `AppDisabled`. Only a zome call that RETURNED reaches
    /// [`Self::record_success_at`].
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
        ledger: &EnableLedger,
        at_ms: u64,
    ) -> RecoveryOutcome {
        let observer = self.for_role(role);
        let before = observer.snapshot_at(at_ms);
        observer.record_success_at(at_ms);
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
        if enable_attempts > 0 || before.status != ZomePathStatus::Live {
            ledger.note_running(role);
        }
        RecoveryOutcome {
            recovered_from_not_running: before.status == ZomePathStatus::AppDisabled,
            not_running_secs: before.not_running_secs.unwrap_or(0),
            enable_attempts,
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
    match roles.derive_supervised_status(supervised) {
        ZomePathStatus::Live => bridge_health().record_success(),
        ZomePathStatus::Dead => bridge_health().record_failure(),
        ZomePathStatus::AppDisabled => {
            bridge_health().record_app_disabled(
                &roles
                    .supervised_disabled_reason(supervised)
                    .unwrap_or_else(|| "the installed app is not running".to_string()),
            );
        }
        ZomePathStatus::Unknown => {}
    }
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
    let outcome = role_bridge_health().record_success_on(role, enable_ledger(), now_ms());
    // The gauge is a LEVEL and this is the only place entitled to raise it:
    // it now means "a call has landed on this role", not "the conductor says
    // the app is enabled".
    crate::metrics::set_conductor_app_enabled(role, true);
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
    let new_episode = role_bridge_health().record_app_disabled_on(role, reason, now_ms());
    crate::metrics::set_conductor_app_enabled(role, false);
    if new_episode {
        warn!(
            role,
            reason,
            "conductor app is NOT RUNNING — every zome call on this role will fail while it \
             stays this way, and the node's own projection reads will keep looking healthy. \
             Storage will attempt enable_app on a bounded backoff (60s doubling to a 1h cap); \
             the next line about this role will be the recovery, when a zome call succeeds."
        );
    }
    resync_aggregate();
}

/// Fold one `app_info()` probe answer into ROLE's observer.
///
/// ASYMMETRIC ON PURPOSE. `NotRunning` is recorded, because a conductor saying
/// its own app is not running can only understate this node's health. `Running`
/// records NOTHING — `app_info` answers from the app record, and the app is
/// enabled for the whole of a startup (or a stall) in which its cells are
/// absent from `running_cells` and every zome call answers `CellDisabled`.
/// Recording it was how the verdict flapped every ~20s while the truth-writing
/// path stayed dead.
pub fn observe_role_app_status(role: &str, observation: &AppRunObservation) {
    match observation {
        AppRunObservation::Running => note_app_enabled_but_not_running(role),
        AppRunObservation::NotRunning { reason } => record_role_app_disabled(role, reason),
    }
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
                AppRunObservation::NotRunning { reason } => assert!(
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
        let outcome = roles.record_success_on("lamad", &ledger, T0 + 660_000);

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
        roles.record_success_on("lamad", &ledger, T0 + 60_000);

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

        roles.record_success_on("lamad", &ledger, T0 + 5_000);

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
            !disabled.record_responsive_at(T0 + 1_000),
            "REFUSED: a cell that will not serve still answers domain errors"
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
            // refused; after a Dead it is permitted and parks the role at Live
            // with the ladder still standing — which is exactly the state a
            // status-only guard cannot distinguish from a healthy steady state.
            let detoured = detour == "through-a-dead-websocket";
            if detoured {
                roles.for_role("lamad").record_failure_at(T0 + 1_000);
            }
            let folded = roles.for_role("lamad").record_responsive_at(T0 + 2_000);
            assert_eq!(
                folded, detoured,
                "{detour}: a responsive answer is refused inside an episode and accepted after a \
                 dead websocket"
            );
            if detoured {
                assert_eq!(
                    roles.for_role("lamad").status(),
                    ZomePathStatus::Live,
                    "{detour}: parked at Live with an hour-deep ladder still on the books"
                );
            }

            // A call lands. Whatever the node believed a millisecond earlier, a
            // ladder with attempts on it is cleared by a call that landed.
            let outcome = roles.record_success_on("lamad", &ledger, T0 + 10_000);
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
        roles.record_success_on(role_of(&lamad_cell), &ledger, T0 + 5_000);

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
        let outcome = roles.record_success_on(ROLE, &ledger, T0 + 240_000);

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
        roles.record_success_on("node_registry", &ledger, T0 + 240_000);
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
}
