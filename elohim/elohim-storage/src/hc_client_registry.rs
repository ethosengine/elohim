//! HcClientRegistry — role-keyed cache of conductor connections.
//!
//! Phase 11 introduces a second HcClient connection (role: "imagodei")
//! alongside the existing infrastructure role used by the heartbeat path.
//! Keeping these in one struct keeps `main.rs` startup tidy and avoids
//! having to thread two `Option<Arc<HcClient>>` parameters separately.
//!
//! Connect failure for any role is logged and non-fatal — the node keeps
//! serving HTTP. Downstream code checks `Option<Arc<HcClient>>` and
//! returns a 503 if the role is unconnected.

use std::sync::{Arc, RwLock};
use std::time::Duration;
use tracing::{info, warn};

use crate::hc_client::{BridgeProbe, HcClient, HcClientConfig};
use crate::lineage_roles::LineageRoles;

/// Capped exponential backoff for a conductor-bridge reconnect loop.
///
/// `attempt` is 1-based. The schedule is 2s → 4s → 8s → 16s → 32s → 60s
/// (cap), held at 60s indefinitely thereafter. This mirrors the
/// persistent-peering redial cadence (`p2p::bootstrap_needing_redial`'s
/// caller) and the original 5-attempt registry ramp, but with NO terminal
/// attempt — a bridge consumer that retries forever calls this on every loop.
///
/// Pure + total (saturating shift, hard cap) so it is unit-testable the same
/// way `bootstrap_needing_redial` is, without a live conductor.
pub fn reconnect_backoff(attempt: u32) -> Duration {
    const BASE_SECS: u64 = 2;
    const CAP_SECS: u64 = 60;
    // attempt 1 → 2^0 * 2 = 2s, attempt 2 → 4s, … saturating to avoid overflow
    // on a never-terminating loop (after ~attempt 36 the shift saturates).
    let shift = attempt.saturating_sub(1).min(63);
    let secs = BASE_SECS.saturating_mul(1u64.checked_shl(shift).unwrap_or(u64::MAX));
    Duration::from_secs(secs.min(CAP_SECS))
}

/// How many backoff steps separate consecutive `WARN`-level "still down"
/// surfacings once the loop has saturated at the 60s cap, so a permanently
/// unreachable conductor logs roughly every ~5 minutes (5 × 60s) rather than
/// once per minute. Pure helper so the cadence is asserted in tests.
pub fn should_warn_still_down(attempt: u32) -> bool {
    // Warn on the first attempt (the give-up→retry transition is loud),
    // then every 5th attempt thereafter (~5 min once saturated at the cap).
    attempt == 1 || attempt.is_multiple_of(5)
}

/// How often the bridge supervisor asks a LIVE role whether it is still alive.
///
/// 20s is deliberately shorter than the heartbeat's 60s tick: the heartbeat
/// only WARNS, so before this supervisor existed the shortest honest window
/// between a conductor restart and any recovery was infinity. 20s bounds the
/// dead window at roughly one probe plus one reconnect (~2s), which the
/// throwaway reproduction measured end-to-end.
pub const BRIDGE_PROBE_INTERVAL: Duration = Duration::from_secs(20);

/// Role-keyed registry of HcClient connections. Every slot holds `None` when
/// the role is not currently connected — at startup, or after the supervisor
/// observed its bridge die — and downstream code returns 503 if the role is
/// unavailable.
///
/// ALL THREE SLOTS ARE INTERIOR-MUTABLE, and that is the whole point. A
/// conductor restart invalidates the app-authentication token and closes both
/// websockets of every role at once; `holochain_client` does not reconnect and
/// the token cannot be reused, so recovery means building a WHOLE NEW
/// [`HcClient`] (re-attach app interface → re-authorize per-cell signing
/// credentials → re-mint the auth token → reconnect the app websocket) and
/// swapping it in behind the handles the HTTP layer already holds. Before this,
/// only `lamad` could be swapped, so `infrastructure` (heartbeat, peer status)
/// and `imagodei` (identity, qahal) stayed permanently dead after any
/// conductor-only restart.
pub struct HcClientRegistry {
    pub infrastructure: RwLock<Option<Arc<HcClient>>>,
    pub imagodei: RwLock<Option<Arc<HcClient>>>,
    /// `lamad` role — hosts the `content_store` zome (REA commitments,
    /// content rows, attestations). Required for the conductor-first HTTP
    /// write path landing per 2026-05-26-substrate-rea-replication-fix.md
    /// (closes Gap C/D — REA + content row replication on alpha).
    pub lamad: RwLock<Option<Arc<HcClient>>>,
    /// `node_registry` role — hosts the `node_registry_coordinator` zome
    /// (shard assignments). Task 6 of the Holochain Evolution Epic MVP:
    /// `NodeRegistryApi` used to own a private, unsupervised `HcClient` of
    /// its own; folding it into this registry gives it the same
    /// bounded-boot-ramp + forever-reconnect + supervisor liveness every
    /// other role already has, instead of connecting once at startup and
    /// staying dead forever after any conductor restart.
    pub node_registry: RwLock<Option<Arc<HcClient>>>,
    /// The conductor URLs every role in this registry dialed with.
    ///
    /// Kept so a caller that needs a client for an app id which is NOT a
    /// supervised role — the lineage ("side") app a `happ-lineage` apply
    /// installs beside the base app (Task 7) — can dial the SAME conductor
    /// without a second copy of the URLs being threaded through half of
    /// `main.rs`. `None` on [`Self::empty`]: a registry that never connected
    /// knows no URLs, and [`Self::connect_app`] fails closed there rather than
    /// guessing a localhost default.
    inputs: Option<HcRegistryInputs>,
}

/// The roles the supervisor keeps alive. Ordered coldest-first, matching the
/// boot ramp: `infrastructure` is the role most likely to be `None`-stamped on
/// a slow conductor boot.
///
/// This is the set with a SWAPPABLE REGISTRY SLOT — a role here has an
/// `HcClient` of its own that the supervisor can clear and re-mint. It is NOT
/// the set of roles whose cell health this node observes; see
/// [`OBSERVED_ROLES`].
pub const SUPERVISED_ROLES: [&str; 4] = ["infrastructure", "imagodei", "lamad", "node_registry"];

/// Roles this node can reach a CELL of, and whose not-running episodes
/// therefore belong in `/health`'s `perRole` map and in the aggregate
/// `zomePath` verdict.
///
/// WHY THIS IS NOT `SUPERVISED_ROLES`. `mishpat` is a cell of the same installed
/// app, reached through [`crate::hc_client::HcClient::call_zome_mishpat`] on
/// ANOTHER role's client, so it has no slot of its own and cannot be re-minted
/// independently — but its cells are absent from the conductor's `running_cells`
/// during exactly the same startup window as every other role's, and a mishpat
/// `CellDisabled` is exactly as much a refusal to write truth. Before
/// 2026-09-21 its observations were filed under the CALLING client's role
/// (normally lamad), which both slandered lamad and let an ordinary lamad
/// success clear an episode that belonged to mishpat.
///
/// Conflating the two sets is what the extra constant prevents: adding
/// `mishpat` to `SUPERVISED_ROLES` would spawn a supervisor task for a role with
/// no slot, hand `LineageRoles` a role it never authors under, and break
/// `every_supervised_role_has_a_swappable_slot` — which is that constant telling
/// the truth about itself.
pub const OBSERVED_ROLES: [&str; 5] = [
    "infrastructure",
    "imagodei",
    "lamad",
    "node_registry",
    "mishpat",
];

/// Observed roles with no registry slot of their own, and therefore no
/// supervisor task: they are tended by the task of the role whose client
/// carries their `CellId`.
pub const CROSS_CELL_ROLES: [&str; 1] = [crate::hc_client::MISHPAT_ROLE];

/// The supervised role whose client tends [`CROSS_CELL_ROLES`].
///
/// Every `HcClient` resolves `mishpat_cell_id` from `app_info`, so any of them
/// could drive the probe; naming ONE keeps the cadence deterministic (a single
/// task, a single ladder) rather than four tasks racing for the same rung. The
/// in-flight gate in [`crate::services::cell_probe`] is the belt under this
/// brace and holds even if this ever becomes more than one driver.
pub const CROSS_CELL_DRIVER_ROLE: &str = "lamad";

/// One tick's reading of a role: the classified state, the status evidence it
/// was classified with, and the SUCCESS ORDER it was classified against.
///
/// The third field is what makes the decision revocable. A classification is a
/// statement about a moment; between that moment and the action a zome call can
/// return, and then the whole plan is about a world that no longer exists.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TickObservation {
    pub state: crate::conductor_bridge_health::RoleCellState,
    pub enable: crate::conductor_bridge_health::AppEnableEvidence,
    pub classified_from_success_seq: u64,
}

/// What one supervisor tick does about a role observed NOT RUNNING.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotRunningAction {
    /// Spend a ladder rung on BOTH: `enable_app` (unless this is a cross-cell
    /// role, which has no app of its own), then the read-only cell probe.
    SpendRungEnableAndProbe,
    /// Spend a ladder rung on the PROBE ONLY. `enable_app` provably cannot act
    /// (the app is already Enabled, or its status is one the conductor refuses
    /// to enable) but membership has NOT proven the cell absent — so the probe
    /// is still the only recovery evidence a quiet role has, and withholding it
    /// would leave such a role red for the life of the process.
    SpendRungProbeOnly,
    /// The ladder window is shut. Nothing is asked this tick.
    WaitForWindow,
    /// Probe the cell NOW, off the ladder — membership says it is running, so
    /// this is the moment the episode can actually be closed.
    ProbeNow,
    /// Keep observing and ask nothing: membership PROVES the cell absent (so a
    /// probe would fail by construction) and the app-status evidence proves
    /// `enable_app` cannot act.
    ObserveOnly,
}

impl NotRunningAction {
    /// Does this action consume a ladder rung?
    pub fn spends_a_rung(self) -> bool {
        matches!(
            self,
            NotRunningAction::SpendRungEnableAndProbe | NotRunningAction::SpendRungProbeOnly
        )
    }
}

/// The whole tick decision, as a pure function of three independent inputs.
///
/// Extracted so the policy is a unit test with no conductor, no websocket and
/// no `HcClient`; [`execute_not_running_action`] is the executor, and its own
/// tests drive it through an injected actuator so a corrupted production arm
/// cannot stay green.
///
/// **The mutation is decided by `enable` ALONE, independently of membership.**
/// That ordering is the fix for the case where `ping()` had just observed
/// `Enabled` and the first membership read failed: folding the status into
/// `MembershipUnknown` discarded the one piece of evidence that settles the
/// question, and the tick spent a rung calling the fork's proven no-op.
/// Membership only adds two refinements on top — probe NOW when it says the
/// cell is running, and skip the probe when it PROVES the cell absent.
pub fn decide_not_running_action(
    state: crate::conductor_bridge_health::RoleCellState,
    enable: crate::conductor_bridge_health::AppEnableEvidence,
    ladder_window_open: bool,
) -> NotRunningAction {
    // Membership says the cell is in the running map: the probe is owed now,
    // off the ladder, because this is the moment the episode can end.
    if state.probe_is_owed_now() {
        return NotRunningAction::ProbeNow;
    }
    // The MUTATION gate, decided by app-status evidence on its own.
    if enable.enable_may_help() {
        return if ladder_window_open {
            NotRunningAction::SpendRungEnableAndProbe
        } else {
            NotRunningAction::WaitForWindow
        };
    }
    // enable_app provably cannot act. Whether anything is still worth asking
    // turns on whether membership PROVED the cell absent.
    if state.membership_proves_absent() {
        // A probe would fail by construction, so nothing is asked and NO rung
        // is spent — the ladder stays clear for a relapse a rung can cure.
        NotRunningAction::ObserveOnly
    } else if ladder_window_open {
        NotRunningAction::SpendRungProbeOnly
    } else {
        NotRunningAction::WaitForWindow
    }
}

/// The two mutations a not-running tick can make, behind an interface.
///
/// Exists so [`execute_not_running_action`] is testable: the zero-rung
/// assertion that matters is "production called neither `enable_app` nor the
/// probe, and did not touch the ledger", and that is only assertable by
/// executing the production branch against a recording double.
#[async_trait::async_trait]
pub(crate) trait NotRunningActuator: Send + Sync {
    /// Ask the conductor to enable the app behind `role`, on ladder `attempt`.
    async fn enable_app(&self, role: &str, attempt: u32);
    /// Ask `role`'s own cell one read-only question.
    async fn probe_cell(&self, role: &str);
}

/// What [`execute_not_running_action_at`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActionOutcome {
    /// The action ran (including the two that deliberately do nothing).
    Executed,
    /// ABORTED: a zome call RETURNED between the classification and the action,
    /// so the decision was taken against a world that no longer exists.
    Superseded { by_success_seq: u64 },
}

/// Execute one tick's decision. The ONLY place a rung is consumed or a
/// conductor mutation is made on the not-running path.
///
/// `now` is the instant the rung is stamped at. Injected rather than read here
/// because [`crate::services::enable_app_backoff::EnableLedger`] is keyed by
/// `Instant`: a test that drives a synthetic clock through
/// [`decide_not_running_action`] but let this function read the wall clock would
/// be measuring the millisecond skew between the two, not the ladder policy.
pub(crate) async fn execute_not_running_action_at(
    action: NotRunningAction,
    role: &str,
    is_cross_cell: bool,
    ledger: &crate::services::enable_app_backoff::EnableLedger,
    actuator: &dyn NotRunningActuator,
    now: std::time::Instant,
    classified_from_success_seq: u64,
) -> ActionOutcome {
    // ONE GUARDED SECTION: RE-CHECK, THEN DEBIT.
    //
    // The decision above was taken from a snapshot. A zome call can RETURN
    // between that snapshot and this line — and then the role is serving, the
    // episode is over and the ladder has been cleared, while this task is still
    // holding a plan to enable an app and probe a cell that already answered.
    //
    // The re-check and the rung debit are ONE acquisition of the publication
    // guard, because two acquisitions leave a gap a success can land in: the
    // check would pass, the success would clear the ladder, and the debit would
    // then write a rung onto a role that is serving.
    let observer = crate::conductor_bridge_health::role_bridge_health().for_role(role);
    let spends_a_rung = action.spends_a_rung();
    let debit = {
        let _publishing = observer.publish_guard();
        let current = observer.observe();
        if current.last_success_seq > classified_from_success_seq {
            crate::metrics::note_superseded_observation("tick-action");
            tracing::debug!(
                role,
                action = ?action,
                classified_from_success_seq,
                now_success_seq = current.last_success_seq,
                "not-running action ABORTED — a zome call returned after this tick classified; \
                 the decision describes a world that has moved on"
            );
            return ActionOutcome::Superseded {
                by_success_seq: current.last_success_seq,
            };
        }
        if spends_a_rung {
            let previous = ledger.record_for(role);
            let token = ledger.note_attempt_at(role, now);
            Some((token, previous))
        } else {
            None
        }
    };

    match action {
        NotRunningAction::ObserveOnly | NotRunningAction::WaitForWindow => {}
        NotRunningAction::ProbeNow => actuator.probe_cell(role).await,
        NotRunningAction::SpendRungEnableAndProbe | NotRunningAction::SpendRungProbeOnly => {
            let (token, previous) = debit.expect("a rung-spending action debits in the guard");
            let attempt = ledger.attempts(role);
            // A cross-cell role (mishpat) is a CELL of the same installed app,
            // so there is no app of its own to enable — the enable its siblings
            // make is already the whole of that cure. Its rung buys the probe
            // only, exactly as `SpendRungProbeOnly` does for a different reason.
            if action == NotRunningAction::SpendRungEnableAndProbe && !is_cross_cell {
                actuator.enable_app(role, attempt).await;
            }

            // THE RPC IS NOT CANCELLABLE, BUT THE RUNG IS RETRACTABLE.
            //
            // A zome call can land while the enable RPC is in flight — the one
            // window the pre-dispatch re-check cannot close, because the RPC is
            // an await and the guard is not held across it. The enable was then
            // a no-op against an app that is serving, which costs nothing; but
            // CHARGING THE LADDER for it would hand the next outage an
            // inherited backoff window it did not earn. So the attempt is undone
            // to exactly the record it replaced.
            //
            // Residual, stated precisely: ONE no-op enable RPC against an
            // already-serving conductor, and NO rung. Closing that last RPC
            // would mean holding the guard across a conductor round trip, which
            // is the uncancellable-call trap this crate refuses everywhere else.
            let after = {
                let _publishing = observer.publish_guard();
                let after = observer.observe();
                if after.last_success_seq > classified_from_success_seq {
                    // TOKEN-GATED: this removes THIS tick's attempt and nothing
                    // else. If `record_role_success` cleared the ledger while
                    // the RPC was in flight, the entry is gone, the token does
                    // not match, and the recovery's reset STAYS — restoring the
                    // pre-recovery history here is what handed a relapse a
                    // seven-deep ladder and an hour-long window.
                    ledger.retract_attempt(role, token, previous);
                }
                after
            };
            if after.last_success_seq > classified_from_success_seq {
                crate::metrics::note_superseded_observation("tick-rung");
                tracing::debug!(
                    role,
                    classified_from_success_seq,
                    now_success_seq = after.last_success_seq,
                    "enable RPC returned after a zome call LANDED — the rung is retracted so the \
                     next outage does not inherit a backoff window this tick did not earn; the \
                     RPC itself was a no-op against a serving app"
                );
                return ActionOutcome::Superseded {
                    by_success_seq: after.last_success_seq,
                };
            }
            actuator.probe_cell(role).await;
        }
    }
    ActionOutcome::Executed
}

/// [`execute_not_running_action_at`] against the monotonic clock.
pub(crate) async fn execute_not_running_action(
    action: NotRunningAction,
    role: &str,
    is_cross_cell: bool,
    ledger: &crate::services::enable_app_backoff::EnableLedger,
    actuator: &dyn NotRunningActuator,
    classified_from_success_seq: u64,
) -> ActionOutcome {
    execute_not_running_action_at(
        action,
        role,
        is_cross_cell,
        ledger,
        actuator,
        std::time::Instant::now(),
        classified_from_success_seq,
    )
    .await
}

/// Connection inputs. Mirrors the relevant CLI args without depending on
/// the Args struct directly (cleaner test surface).
#[derive(Debug, Clone)]
pub struct HcRegistryInputs {
    pub admin_url: String,
    pub app_url: String,
    pub app_id: String,
    /// Per-role AUTHORING app id resolver (Task 6). With no lineage window
    /// ever opened, `lineage.app_id_for(role)` always returns `app_id`
    /// above — every `HcClientConfig` this registry builds is byte-for-byte
    /// identical to the pre-Task-6 hard-coded `app_id.clone()`.
    pub lineage: Arc<LineageRoles>,
}

impl HcClientRegistry {
    /// Connect each role in sequence. Per-role failure is logged and
    /// returns `None` for that role — the registry as a whole always
    /// constructs.
    pub async fn connect(inputs: &HcRegistryInputs) -> Self {
        let infrastructure = Self::connect_role(inputs, "infrastructure").await;
        let imagodei = Self::connect_role(inputs, "imagodei").await;
        let lamad = Self::connect_role(inputs, "lamad").await;
        let node_registry = Self::connect_role(inputs, "node_registry").await;
        Self {
            infrastructure: RwLock::new(infrastructure),
            imagodei: RwLock::new(imagodei),
            lamad: RwLock::new(lamad),
            node_registry: RwLock::new(node_registry),
            inputs: Some(inputs.clone()),
        }
    }

    /// A registry with every role unconnected — the shape a node has before any
    /// bridge lands, and the shape tests construct.
    pub fn empty() -> Self {
        Self {
            infrastructure: RwLock::new(None),
            imagodei: RwLock::new(None),
            lamad: RwLock::new(None),
            node_registry: RwLock::new(None),
            inputs: None,
        }
    }

    /// Connect a FRESH client to an arbitrary installed app id on the SAME
    /// conductor this registry dials, under `role`.
    ///
    /// The lineage-app path (Task 7's `HappLineageVehicle`): a `happ-lineage`
    /// apply installs `elohim@<hash12>` beside the base app and then has to
    /// drive `carry_from` on the NEW cell. That app is not a supervised role
    /// and must never become one — it is per-crossing, it is not part of this
    /// node's steady-state serving surface, and a slot for it would outlive
    /// the window it belongs to.
    ///
    /// So the handle is returned, never cached: the caller owns it for exactly
    /// the length of one apply and drops it. Nothing here mutates the registry.
    pub async fn connect_app(
        &self,
        app_id: &str,
        role: &str,
    ) -> Result<Arc<HcClient>, crate::error::StorageError> {
        let inputs = self.inputs.as_ref().ok_or_else(|| {
            crate::error::StorageError::Conductor(
                "this registry never connected, so the conductor URLs are unknown — cannot dial a \
                 side app"
                    .into(),
            )
        })?;
        let config = HcClientConfig {
            admin_url: inputs.admin_url.clone(),
            app_url: inputs.app_url.clone(),
            // Deliberately NOT `inputs.lineage.app_id_for(role)`: the caller is
            // naming the app it wants. Routing this through the window resolver
            // would make "connect me to the lineage app" resolve to whatever
            // the window currently says, which is the value the caller is in
            // the middle of establishing.
            app_id: app_id.to_string(),
            role: Some(role.to_string()),
        };
        HcClient::connect(config).await.map(Arc::new)
    }

    fn slot(&self, role: &str) -> Option<&RwLock<Option<Arc<HcClient>>>> {
        match role {
            "infrastructure" => Some(&self.infrastructure),
            "imagodei" => Some(&self.imagodei),
            "lamad" => Some(&self.lamad),
            "node_registry" => Some(&self.node_registry),
            _ => None,
        }
    }

    /// Snapshot the current handle for `role`, or `None` for an unknown role.
    pub fn client(&self, role: &str) -> Option<Arc<HcClient>> {
        self.slot(role)?
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Store (or clear) the handle for `role`. Unknown roles are ignored.
    pub fn set_client(&self, role: &str, hc: Option<Arc<HcClient>>) {
        if let Some(slot) = self.slot(role) {
            *slot.write().unwrap_or_else(|e| e.into_inner()) = hc;
        }
    }

    /// Snapshot the current `infrastructure` handle.
    pub fn infrastructure_client(&self) -> Option<Arc<HcClient>> {
        self.client("infrastructure")
    }

    /// Snapshot the current `imagodei` handle.
    pub fn imagodei_client(&self) -> Option<Arc<HcClient>> {
        self.client("imagodei")
    }

    /// Snapshot the current `lamad` handle. Interior-mutable: the boot-time
    /// late-connect updater (main.rs) stores a connection that lands after the
    /// bounded boot ramp gave up, so the HTTP re-notarize path picks it up
    /// instead of reading a frozen boot-time `None`.
    pub fn lamad_client(&self) -> Option<Arc<HcClient>> {
        self.client("lamad")
    }

    /// Store a (re)connected `lamad` handle (called by the late-connect updater).
    pub fn set_lamad(&self, hc: Option<Arc<HcClient>>) {
        self.set_client("lamad", hc);
    }

    /// An ADMIN websocket handle from ANY currently-live role, or `None` when
    /// no role is connected (fail closed — the caller answers 503).
    ///
    /// Every role dials the SAME conductor admin URL ([`HcRegistryInputs::
    /// admin_url`]), so which role supplies the handle is immaterial for
    /// admin-plane reads (`agent_info`, `dump_network_stats`,
    /// `dump_network_metrics`). Roles are probed in [`SUPERVISED_ROLES`] order.
    /// Because the handle is snapshotted from the supervised registry, a
    /// conductor restart heals this path the same way it heals zome forwarding:
    /// the supervisor swaps in a fresh [`HcClient`] and the next snapshot sees
    /// its live admin connection.
    pub fn any_admin_websocket(&self) -> Option<holochain_client::AdminWebsocket> {
        SUPERVISED_ROLES
            .iter()
            .find_map(|role| self.client(role))
            .map(|hc| hc.admin_websocket())
    }

    async fn connect_role(inputs: &HcRegistryInputs, role: &str) -> Option<Arc<HcClient>> {
        // Retry with exponential backoff. The conductor's cells transition
        // through CellDisabled state during the first ~15s post-pod-boot
        // before the kitsune handshake completes. A single-shot connect at
        // T+9.5s permanently None-stamps the role and breaks the
        // conductor-required project-epr write path
        // (services/rea_commitment_service.rs::create_via_conductor → 503
        // "lamad bridge unavailable"), failing genesis Seed Projections on
        // every CI run since 2026-05-26. Mirrors import_api::connect_conductor's
        // 5-attempt 2s→4s→8s→16s→30s(cap) backoff — proven on
        // elohim-adam-alpha-0 logs to clear the race by attempt 4 (T+15s).
        let config = HcClientConfig {
            admin_url: inputs.admin_url.clone(),
            app_url: inputs.app_url.clone(),
            // Task 6: routes through the per-role lineage resolver. With no
            // window open this returns `inputs.app_id` unchanged — the same
            // value this line hard-coded before.
            app_id: inputs.lineage.app_id_for(role),
            role: Some(role.to_string()),
        };
        let max_attempts: u32 = 5;
        let mut delay = Duration::from_secs(2);
        for attempt in 1..=max_attempts {
            match HcClient::connect(config.clone()).await {
                Ok(hc) => {
                    info!(role, attempt, "HcClient connected");
                    return Some(Arc::new(hc));
                }
                Err(e) if attempt < max_attempts => {
                    warn!(
                        role,
                        attempt,
                        max_attempts,
                        error = %e,
                        delay_secs = delay.as_secs(),
                        "HcClient connect failed — retrying (cells may still be CellDisabled)"
                    );
                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(delay * 2, Duration::from_secs(30));
                }
                Err(e) => {
                    warn!(
                        role,
                        attempt,
                        max_attempts,
                        error = %e,
                        "HcClient connect failed after the bounded boot ramp — \
                         a background reconnect loop will keep retrying indefinitely \
                         (routes for this role return 503 until it lands)"
                    );
                    return None;
                }
            }
        }
        None
    }

    /// Connect a single role, retrying **indefinitely** with capped
    /// exponential backoff (`reconnect_backoff`). Unlike `connect_role` this
    /// never gives up — it is the bridge-survival path for the genesis #1122
    /// shape, where a conductor needs 6+ minutes to enable cells after a
    /// rolling restart. A late connection is exactly as good as an early one:
    /// the returned `Arc<HcClient>` is wired by the caller into the same
    /// downstream tasks the boot-success path uses.
    ///
    /// Resolves only once the connection lands (or `shutdown` fires, returning
    /// `None`). Each state transition is INFO-logged for Loki; while down, a
    /// `WARN` is emitted on the `should_warn_still_down` cadence (~5 min once
    /// saturated at the cap) so a permanently-unreachable conductor is loud
    /// but not spammy.
    pub async fn connect_role_forever(
        inputs: &HcRegistryInputs,
        role: &str,
        mut shutdown: tokio::sync::broadcast::Receiver<()>,
    ) -> Option<Arc<HcClient>> {
        let config = HcClientConfig {
            admin_url: inputs.admin_url.clone(),
            app_url: inputs.app_url.clone(),
            // Task 6: same resolver as `connect_role` above.
            app_id: inputs.lineage.app_id_for(role),
            role: Some(role.to_string()),
        };
        let mut attempt: u32 = 0;
        loop {
            attempt = attempt.saturating_add(1);
            match HcClient::connect(config.clone()).await {
                Ok(hc) => {
                    if attempt == 1 {
                        info!(role, attempt, "HcClient bridge connected");
                    } else {
                        // The fix's signature observability line: a connection
                        // that landed only after the boot ramp gave up.
                        info!(
                            role,
                            attempt,
                            "HcClient bridge connected (late) — wiring same as boot-success path"
                        );
                    }
                    return Some(Arc::new(hc));
                }
                Err(e) => {
                    let delay = reconnect_backoff(attempt);
                    if should_warn_still_down(attempt) {
                        warn!(
                            role,
                            attempt,
                            error = %e,
                            delay_secs = delay.as_secs(),
                            "HcClient bridge still down — retrying forever (conductor cells may still be CellDisabled after a rolling restart)"
                        );
                    } else {
                        info!(
                            role,
                            attempt,
                            error = %e,
                            delay_secs = delay.as_secs(),
                            "HcClient bridge reconnect: retrying"
                        );
                    }
                    tokio::select! {
                        _ = tokio::time::sleep(delay) => {}
                        _ = shutdown.recv() => {
                            info!(role, "HcClient bridge reconnect loop exiting (shutdown)");
                            return None;
                        }
                    }
                }
            }
        }
    }

    /// Keep every role's bridge ALIVE for the life of the process.
    ///
    /// WHY THIS EXISTS. `connect_role_forever` retries only while the connect
    /// FAILS; it returns the instant one succeeds, and nothing then watches the
    /// connection it handed back. `HcClient` holds an `AdminWebsocket` plus an
    /// `AppWebsocket` built from a ONE-SHOT `issue_app_auth_token`;
    /// `holochain_client` 0.8 has no reconnect, and a conductor restart both
    /// closes the sockets and invalidates the token (the conductor's token store
    /// is in-memory — a reconnect with the old token is refused
    /// `Authentication failed with reason: Invalid token`). So a conductor-only
    /// restart under a running storage left every zome call answering
    /// `Websocket closed: No connection` FOREVER, while `/health` answered 200
    /// and `POST /db/content` kept accepting writes it could never anchor.
    ///
    /// Recovery is not a socket-level reconnect — it is a full re-mint, which is
    /// exactly what `HcClient::connect` already does end to end: attach the app
    /// interface, re-authorize per-cell signing credentials for every cell the
    /// happ provisions, issue a FRESH app-authentication token, and reconnect
    /// the app websocket. So the supervisor's whole job is to notice death and
    /// re-arm the existing forever-loop.
    ///
    /// Order matters on the way down: the handle is cleared BEFORE reconnecting,
    /// so routes answer `503 bridge unavailable` (honest backpressure a caller
    /// can retry) rather than `502 Websocket closed` (a broken node) during the
    /// gap.
    /// Ask the conductor to ENABLE the app behind `role`, bounded and fenced.
    ///
    /// The supervisor's half of the 2026-09-18 cure. Three gates, in order, and
    /// the order matters:
    ///
    /// 1. **The closed-chain fence.** A sealed v1 cell must never be re-enabled
    ///    — a crossing DISABLES the old app on purpose, and enabling it invites
    ///    this node to author on a sealed chain (a capability grant is enough),
    ///    which every neighbour warrants into a permanent cell block holochain
    ///    0.7 cannot lift. Checked FIRST, so a fenced role can never consume an
    ///    attempt slot or reach the admin socket.
    /// 2. **MEMBERSHIP — is `enable_app` even capable of helping?** (2026-09-22)
    ///    `ListCellIds` names the conductor's running-cell map; joined with the
    ///    app's persisted status it produces one
    ///    [`crate::conductor_bridge_health::RoleCellState`]. Where that state is
    ///    STRANDED (`installed-not-running-app-enabled`) the conductor's own
    ///    source proves `enable_app` short-circuits without touching a cell, so
    ///    NO rung is spent — the ladder is not "backed off", it is not entered.
    ///    Where membership says the cell IS running, the probe is owed
    ///    immediately instead. Every other state keeps the pre-existing cure
    ///    byte-for-byte, because an absent observation is not a disproof.
    /// 3. **The bounded ladder.** `enable_app` is an admin-plane write against
    ///    a conductor that is, by hypothesis, already unwell. One attempt per
    ///    [`crate::services::enable_app_backoff::enable_backoff`] window —
    ///    60s doubling to a 1h cap — never per 20s probe.
    /// 4. **The conductor's own answer.** On refusal the error is logged
    ///    VERBATIM at WARN, because THAT error is the diagnosis of why the app
    ///    is disabled, which is the one fact the incident never produced.
    ///
    /// Deliberately does NOT clear or re-mint the handle: the websocket is
    /// healthy. And deliberately does not treat its OWN answer as an outcome —
    /// an enable that lands and an enable that was a no-op both answer `Ok`.
    /// The episode ends when a zome call on the role succeeds, and not before.
    ///
    /// ## The rung also buys a read-only PROBE (2026-09-21)
    ///
    /// `3ec3614dd` left a stated gap: a role with no continuing organic traffic
    /// has nothing to prove recovery with, so its episode — and with it
    /// `/health/serving`'s 503 — never ends. On this conductor every restart
    /// opens such an episode on every role called early, so the gap fires
    /// routinely rather than exceptionally.
    ///
    /// So one ladder rung now buys TWO things, in this order: ask the conductor
    /// to enable the app (for a role that has one), then ask the role's own cell
    /// ONE read-only question. The probe is not a second recovery path — it is a
    /// zome call, so it reaches `record_role_success` through the same single
    /// transition organic traffic uses, and a probe that FAILS cannot end the
    /// episode. Every bound on it is in [`crate::services::cell_probe`]; the
    /// pacing authority stays exactly where it was — this ladder.
    ///
    async fn try_enable_disabled_app(
        inputs: &HcRegistryInputs,
        role: &str,
        hc: &Arc<HcClient>,
        reason: &str,
    ) {
        if let Some(fence) = crate::closed_chain_fence::fence() {
            if let Some(record) = fence.closed_record(hc.cell_id()) {
                warn!(
                    role,
                    cell = %record.cell,
                    why = %record.why,
                    reason,
                    "conductor app is not running on a role whose chain is CLOSED — REFUSING to \
                     enable it. A close is a sealing act and a disabled v1 app is the correct end \
                     state of a crossing; writes belong on the successor app."
                );
                return;
            }
        }

        // ASK THE CONDUCTOR WHICH CURE APPLIES, BEFORE SPENDING ONE.
        //
        // `ListCellIds` returns `running_cell_ids()` — the same map zome
        // dispatch looks in — so it is the read that answers what `CellDisabled`
        // actually asked. The app's persisted status decides the MUTATION on its
        // own; membership refines whether and when the probe is owed.
        let tick = Self::observe_and_publish_cell_state(role, hc).await;
        let ledger = crate::services::enable_app_backoff::enable_ledger();
        let action =
            decide_not_running_action(tick.state, tick.enable, ledger.should_attempt(role));

        let actuator = SupervisorActuator { inputs, hc, reason };
        execute_not_running_action(
            action,
            role,
            CROSS_CELL_ROLES.contains(&role),
            ledger,
            &actuator,
            tick.classified_from_success_seq,
        )
        .await;
    }

    /// Read membership, join it with the app-status evidence, PUBLISH the
    /// resulting state, and hand both back so the caller can choose a cure.
    ///
    /// Called on EVERY supervisor tick for every role the client resolves, not
    /// only inside a not-running verdict. That is the fix for the gauge that
    /// stayed at `1` after a conductor restart: with publication gated on an
    /// active `CellDisabled` episode, a role that went transport-dead and then
    /// reconnected to an Enabled app whose cell was absent had nothing to
    /// republish it, because no zome call had failed since the restart.
    ///
    /// The membership read is shared by
    /// [`crate::services::cell_membership::MEMBERSHIP_REFRESH_INTERVAL`] and
    /// single-flighted, so five supervisor tasks on a 20s tick still cost
    /// roughly ONE admin round trip between them.
    async fn observe_and_publish_cell_state(role: &str, hc: &Arc<HcClient>) -> TickObservation {
        let admin = hc.admin_websocket();
        Self::observe_and_publish_cell_state_from(
            role,
            &admin,
            hc.cell_id_for_role(role),
            crate::services::cell_membership::membership(),
            crate::services::cell_membership::ObservedAt::now(),
        )
        .await
    }

    /// The whole of [`Self::observe_and_publish_cell_state`], with its three
    /// conductor-bound dependencies passed in: the membership SOURCE, this
    /// role's `CellId`, and the cache.
    ///
    /// Not a test shim — it is the function, and the wrapper above is four
    /// lines of binding. The seam exists because the supervisor's publication
    /// path (refresh → observation identity → contradiction gate → classify →
    /// publish) was otherwise only reachable through a live conductor, so the
    /// tests that claimed to cover it were driving the publisher by hand and
    /// would have stayed green with the supervisor's call deleted.
    ///
    /// `at` carries BOTH clocks and is injected for the same reason the clock is
    /// injected everywhere else in this pair of modules: a test that drove a
    /// synthetic membership clock while this read the real one would be
    /// measuring skew. Durations come off the monotonic half; the wall half only
    /// renders.
    pub(crate) async fn observe_and_publish_cell_state_from(
        role: &str,
        src: &dyn crate::services::cell_membership::RunningCellSource,
        cell_id: Option<&holochain_client::CellId>,
        cache: &crate::services::cell_membership::MembershipCache,
        at: crate::services::cell_membership::ObservedAt,
    ) -> TickObservation {
        use crate::conductor_bridge_health as health;
        cache.refresh_if_stale_at(src, at).await;

        // THE OBSERVATION'S IDENTITY IS THE READING'S, NOT THIS MOMENT'S — and
        // that identity is an ORDER, not a timestamp.
        //
        // A publication stamped `now` claims an observation it did not make.
        // That substitution is how a CACHED absence, published a second after a
        // zome call proved the cell alive, overwrote the recovery. Stamping it
        // with the reading's epoch-ms fixed the ordinary case and left two:
        // two observations inside one millisecond compare EQUAL (and the older
        // one wins a strict `>`), and a backward clock step inverts the
        // relation outright. So the answer carries a sequence from the one
        // process-wide observation clock, and nothing here compares wall time.
        let answer = match cell_id {
            // A role whose `CellId` this client does not carry cannot be asked
            // at all — `None`, which is `membership-unknown` and changes no
            // mutation decision, because the mutation is the app status's call.
            // It is still an observation made NOW, so it is stamped now.
            None => crate::services::cell_membership::MembershipAnswer {
                running: None,
                at_ms: at.wall_ms,
                seq: health::next_obs_seq(),
            },
            Some(cell_id) => cache.cell_running_observed_at(cell_id, at),
        };

        // ONE SNAPSHOT OF THE WHOLE RECORD, and classify from THAT.
        //
        // The previous cut read the evidence KIND and the evidence SEQUENCE
        // with two separate loads. A success landing between them retired the
        // status and restamped it, and the classifier then contradicted a
        // membership reading with the pair `(EnableCanLift, <success seq>)` —
        // a tuple no observation ever produced. `SeqCst` orders each load; it
        // does not make two of them one read.
        let observed = health::role_bridge_health().for_role(role).observe();
        let mut enable = observed.enable;
        let mut enable_seq = observed.enable_seq;
        // A cross-cell role has no app of its own, so its app STATUS is the
        // status of the app its driver role ALSO lives in. Reading the driver's
        // evidence is not a guess: mishpat is a cell of that same installed app.
        // Its snapshot is taken whole for the same reason.
        if enable == health::AppEnableEvidence::Unknown && CROSS_CELL_ROLES.contains(&role) {
            let driver = health::role_bridge_health()
                .for_role(CROSS_CELL_DRIVER_ROLE)
                .observe();
            enable = driver.enable;
            enable_seq = driver.enable_seq;
        }

        // NEWER STATUS EVIDENCE BEATS AN OLDER CACHED `true`.
        //
        // `disable_app` removes an app's cells and awaits their cleanup BEFORE
        // it writes `Disabled`, so a not-Enabled status observed after a
        // membership reading that said "running" proves that reading has
        // stopped describing the conductor. Without this, the bounded case the
        // review named stands: disable the app without closing its sockets, let
        // every later `ListCellIds` fail, and the cached `true` answers
        // `ProbeNow` — suppressing the enable until the authority window
        // expires. The TTL bounds that; it does not cure it.
        //
        // Evidence older than a proven zome success cannot reach here: that
        // success RETIRES it (`record_role_success` restamps the role
        // `AlreadyEnabled` at the success's own sequence), so a role that is
        // now `Live` can no longer be argued into `membership-unknown` by a
        // `Disabled` answer the success has already outlived.
        let membership = if health::status_contradicts_cached_membership(
            answer.running,
            answer.seq,
            enable,
            enable_seq,
        ) {
            info!(
                role,
                enable = enable.as_str(),
                membership_seq = answer.seq,
                enable_seq,
                "a cached membership `running` was observed BEFORE an app status this conductor \
                 reports as NOT enabled — a disable removes cells before it writes that status, \
                 so the cached reading is no longer evidence. Treating membership as UNKNOWN \
                 until a fresh ListCellIds lands, which keeps the enable ladder running."
            );
            None
        } else {
            answer.running
        };

        let state = health::classify_cell_state(membership, enable);
        health::record_role_cell_state(role, state, membership, enable, answer.seq, answer.at_ms);
        TickObservation {
            state,
            enable,
            // The success order this classification was taken against. The
            // executor compares it before spending anything, so a call that
            // RETURNED between here and there aborts the action instead of
            // enabling an app that is already serving.
            classified_from_success_seq: observed.last_success_seq,
        }
    }

    /// One read-only question to `role`'s own cell. Every bound on it lives in
    /// [`crate::services::cell_probe`]; awaited inline, so this task makes
    /// exactly one and cannot start a second before the first answers.
    async fn probe_the_cell(role: &str, hc: &Arc<HcClient>) {
        let _ = crate::services::cell_probe::probe_cell(
            role,
            hc,
            crate::services::cell_probe::probe_gate(),
        )
        .await;
    }

    /// One `enable_app` attempt, already paced by the caller's ladder rung.
    async fn ask_the_conductor_to_enable(
        inputs: &HcRegistryInputs,
        role: &str,
        hc: &Arc<HcClient>,
        reason: &str,
        attempt: u32,
    ) {
        let app_id = inputs.lineage.app_id_for(role);

        info!(
            role,
            app_id = app_id.as_str(),
            attempt,
            reason,
            "attempting enable_app on an installed app observed NOT RUNNING"
        );
        match hc.admin_websocket().enable_app(app_id.clone()).await {
            Ok(_) => {
                // ACCEPTED IS NOT RECOVERED. `enable_app` on an app that is
                // already enabled returns success WITHOUT restarting its cells,
                // so this answer is equally consistent with a cure and with
                // nothing at all having happened. The ladder is therefore not
                // reset here either; the attempt above already advanced it.
                //
                // From the second attempt on, a full backoff window has elapsed
                // since an accepted enable and the role is still refusing
                // calls — at which point "enable_app cannot lift this" is
                // established, and gets said ONCE for the episode.
                if attempt >= 2 {
                    crate::conductor_bridge_health::note_app_enabled_but_not_running(role);
                }
                info!(
                    role,
                    app_id = app_id.as_str(),
                    attempt,
                    next_attempt_secs =
                        crate::services::enable_app_backoff::enable_backoff(attempt).as_secs(),
                    "enable_app ACCEPTED — this does NOT prove recovery and does NOT reset the \
                     backoff; only a successful zome call on this role does"
                );
            }
            Err(e) => warn!(
                role,
                app_id = app_id.as_str(),
                attempt,
                error = %e,
                next_attempt_secs =
                    crate::services::enable_app_backoff::enable_backoff(attempt).as_secs(),
                "enable_app REFUSED by the conductor — this error IS the missing diagnosis of why \
                 the app is disabled; backing off and trying again"
            ),
        }
    }
}

/// The production [`NotRunningActuator`]: the real `enable_app` and the real
/// cell probe, bound to one role's live client.
struct SupervisorActuator<'a> {
    inputs: &'a HcRegistryInputs,
    hc: &'a Arc<HcClient>,
    reason: &'a str,
}

#[async_trait::async_trait]
impl NotRunningActuator for SupervisorActuator<'_> {
    async fn enable_app(&self, role: &str, attempt: u32) {
        HcClientRegistry::ask_the_conductor_to_enable(
            self.inputs,
            role,
            self.hc,
            self.reason,
            attempt,
        )
        .await;
    }

    async fn probe_cell(&self, role: &str) {
        HcClientRegistry::probe_the_cell(role, self.hc).await;
    }
}

impl HcClientRegistry {
    pub fn spawn_bridge_supervisor(
        self: Arc<Self>,
        inputs: HcRegistryInputs,
        shutdown: tokio::sync::broadcast::Sender<()>,
    ) {
        for role in SUPERVISED_ROLES {
            let registry = Arc::clone(&self);
            let inputs = inputs.clone();
            let mut shutdown_rx = shutdown.subscribe();
            let reconnect_shutdown = shutdown.clone();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = tokio::time::sleep(BRIDGE_PROBE_INTERVAL) => {}
                        _ = shutdown_rx.recv() => {
                            info!(role, "bridge supervisor exiting (shutdown)");
                            return;
                        }
                    }

                    // A role with no handle is either still in its boot ramp or
                    // already being re-armed by this same loop's previous pass;
                    // either way `connect_role_forever` below owns it.
                    let Some(hc) = registry.client(role) else {
                        continue;
                    };

                    // PUBLISH MEMBERSHIP EVERY TICK, whatever the verdict.
                    //
                    // Not gated on a not-running episode, and that is the point:
                    // a role that went transport-dead and then reconnected to an
                    // Enabled app whose cell is ABSENT produces no failing zome
                    // call, so an episode-gated observer had nothing to lower the
                    // gauge with and it stayed at the `1` a much earlier success
                    // had published. The read is shared and single-flighted, so
                    // publishing unconditionally costs no extra admin round trip.
                    // The publish is the point; the verdict is re-derived below
                    // with the FRESH app-status evidence `ping` is about to
                    // produce, so nothing here is carried forward.
                    let _tick = Self::observe_and_publish_cell_state(role, &hc).await;

                    // ADOPT THE ROLES WITH NO TASK OF THEIR OWN. `mishpat` is a
                    // cell of the same installed app reached through this
                    // client, so it has no slot and no supervisor loop — and
                    // before this it therefore had no cure at all: its episodes
                    // could only be ended by commitment traffic that a household
                    // may never generate. One driver, so the cadence is one
                    // ladder rather than four tasks racing for the same rung.
                    if role == CROSS_CELL_DRIVER_ROLE {
                        for cross in CROSS_CELL_ROLES {
                            // Same unconditional publish: a cross-cell role has
                            // no task of its own, so this is its only tick.
                            let _tick = Self::observe_and_publish_cell_state(cross, &hc).await;
                            if crate::conductor_bridge_health::role_is_not_running(cross) {
                                Self::try_enable_disabled_app(
                                    &inputs,
                                    cross,
                                    &hc,
                                    "this cell refuses zome calls and has no bridge of its own — \
                                     tended by the driver role's supervisor",
                                )
                                .await;
                            }
                        }
                    }

                    // `ping` crosses the authenticated APP websocket used by
                    // zome calls (not merely the independently-live admin
                    // websocket) and folds its result into the zome-path
                    // observer, so a node with zero zome traffic still reports
                    // honestly and an app-only death triggers a full re-mint.
                    //
                    // Three outcomes, three cures. `is_ok()` used to collapse
                    // the first two — which is how a DISABLED app read as a
                    // healthy bridge for 38 hours on 2026-09-18.
                    match hc.ping().await {
                        Ok(BridgeProbe::Running) => {
                            // The conductor says the APP is enabled. It does
                            // NOT say its cells are running, and on 2026-09-20
                            // the household answered exactly this while every
                            // zome call on the same role answered CellDisabled
                            // for eleven minutes. So the ladder is NOT reset
                            // here — only a successful zome call resets it
                            // (`conductor_bridge_health::record_role_success`).
                            //
                            // If this node's own observations still say the
                            // role refuses calls, the episode is not over: keep
                            // asking, on the SAME bounded ladder (60s doubling
                            // to a 1h cap) that `try_enable_disabled_app` owns.
                            if crate::conductor_bridge_health::role_is_not_running(role) {
                                Self::try_enable_disabled_app(
                                    &inputs,
                                    role,
                                    &hc,
                                    "the conductor reports this app ENABLED while zome calls on \
                                     the role are refused — its cells are not running",
                                )
                                .await;
                            }
                            continue;
                        }
                        Ok(BridgeProbe::NotRunning { reason }) => {
                            // The WEBSOCKET IS FINE. Re-minting the bridge
                            // would be pure churn against a conductor that is
                            // answering perfectly well; the cure for this state
                            // is `enable_app`, on a bounded ladder.
                            Self::try_enable_disabled_app(&inputs, role, &hc, &reason).await;
                            continue;
                        }
                        Err(_) => {}
                    }

                    warn!(
                        role,
                        "conductor bridge is DEAD (ping failed) — clearing the handle and \
                         re-minting the app auth token; zome routes answer 503 until it lands"
                    );
                    // Drop the dead handle FIRST. Anything holding an Arc clone
                    // keeps failing until it re-reads, but nothing NEW picks up
                    // a corpse, and `/health` stops advertising its DNA hashes.
                    registry.set_client(role, None);
                    drop(hc);
                    crate::conductor_bridge_health::record_role_reconnect(role);
                    // REVOKE the membership answer with the bridge that produced
                    // it. The process on the far side may be a different one, and
                    // a reading of the OLD conductor's running map decides nothing
                    // about the new one's — believing it is how a stale `true`
                    // could classify a genuinely-Disabled app as "membership
                    // present" and suppress its enable ladder indefinitely.
                    crate::services::cell_membership::membership()
                        .invalidate(&format!("the {role} bridge died and is being re-minted"));

                    if let Some(fresh) =
                        Self::connect_role_forever(&inputs, role, reconnect_shutdown.subscribe())
                            .await
                    {
                        registry.set_client(role, Some(fresh));
                        info!(
                            role,
                            "conductor bridge RE-MINTED after a conductor restart — fresh app \
                             auth token, signing credentials re-authorized, zome path live"
                        );
                    } else {
                        // Only `None` on shutdown.
                        return;
                    }
                }
            });
        }
    }
}

#[cfg(test)]
mod backoff_policy_tests {
    use super::{reconnect_backoff, should_warn_still_down};
    use std::time::Duration;

    #[test]
    fn backoff_ramps_then_caps_at_60s() {
        // 2 → 4 → 8 → 16 → 32 → 60 (cap), held forever after.
        assert_eq!(reconnect_backoff(1), Duration::from_secs(2));
        assert_eq!(reconnect_backoff(2), Duration::from_secs(4));
        assert_eq!(reconnect_backoff(3), Duration::from_secs(8));
        assert_eq!(reconnect_backoff(4), Duration::from_secs(16));
        assert_eq!(reconnect_backoff(5), Duration::from_secs(32));
        assert_eq!(reconnect_backoff(6), Duration::from_secs(60));
        assert_eq!(reconnect_backoff(7), Duration::from_secs(60));
    }

    #[test]
    fn backoff_never_overflows_on_a_forever_loop() {
        // The loop calls this with an ever-growing attempt; the saturating
        // shift must hold the cap rather than panic/overflow (genesis #1122:
        // a conductor down for hours means thousands of attempts).
        assert_eq!(reconnect_backoff(40), Duration::from_secs(60));
        assert_eq!(reconnect_backoff(1_000), Duration::from_secs(60));
        assert_eq!(reconnect_backoff(u32::MAX), Duration::from_secs(60));
    }

    #[test]
    fn warn_cadence_is_loud_then_every_fifth() {
        // First attempt is loud (the give-up→retry transition); then every
        // 5th (~5 min once saturated at the 60s cap), quiet in between.
        assert!(should_warn_still_down(1));
        assert!(!should_warn_still_down(2));
        assert!(!should_warn_still_down(3));
        assert!(!should_warn_still_down(4));
        assert!(should_warn_still_down(5));
        assert!(!should_warn_still_down(6));
        assert!(should_warn_still_down(10));
    }
}

#[cfg(test)]
mod supervised_slot_tests {
    use super::*;

    #[test]
    fn every_supervised_role_has_a_swappable_slot() {
        // The defect this guards: `infrastructure` and `imagodei` used to be
        // plain `Option` fields, so the supervisor could clear and re-mint only
        // `lamad`. A conductor restart kills all three bridges at once — a role
        // the supervisor cannot swap is a role that stays dead forever.
        let reg = HcClientRegistry::empty();
        for role in SUPERVISED_ROLES {
            assert!(
                reg.slot(role).is_some(),
                "supervised role '{role}' has no swappable slot"
            );
        }
    }

    /// The two role sets say two different things, and the difference is the
    /// whole of F4's second half: `SUPERVISED_ROLES` is "has a swappable
    /// bridge", `OBSERVED_ROLES` is "this node can reach a cell of it, so its
    /// refusals belong in the health verdict". A role can be the second without
    /// being the first; nothing may be the first without being the second.
    #[test]
    fn every_supervised_role_is_observed_and_every_cross_cell_role_is_observed_without_a_slot() {
        let reg = HcClientRegistry::empty();
        for role in SUPERVISED_ROLES {
            assert!(
                OBSERVED_ROLES.contains(&role),
                "supervised role '{role}' is not observed — its CellDisabled would never reach \
                 /health/serving"
            );
        }
        for role in CROSS_CELL_ROLES {
            assert!(
                OBSERVED_ROLES.contains(&role),
                "cross-cell role '{role}' must be observed: it refuses calls on the same startup \
                 window as every other cell"
            );
            assert!(
                reg.slot(role).is_none(),
                "'{role}' has a registry slot, so it is not a cross-cell role — move it to \
                 SUPERVISED_ROLES instead of tending it from another role's task"
            );
            assert!(
                !SUPERVISED_ROLES.contains(&role),
                "'{role}' cannot be both supervised and cross-cell"
            );
        }
        assert_eq!(
            OBSERVED_ROLES.len(),
            SUPERVISED_ROLES.len() + CROSS_CELL_ROLES.len(),
            "every observed role is either supervised or cross-cell — an observed role that is \
             neither has no path to a probe and would sink the aggregate forever"
        );
        assert!(
            SUPERVISED_ROLES.contains(&CROSS_CELL_DRIVER_ROLE),
            "the cross-cell driver must itself be a supervised role, or nothing spawns the task \
             that tends mishpat"
        );
    }

    #[test]
    fn every_slot_starts_empty_and_reads_back_none() {
        let reg = HcClientRegistry::empty();
        assert!(reg.infrastructure_client().is_none());
        assert!(reg.imagodei_client().is_none());
        assert!(reg.lamad_client().is_none());
        assert!(reg.client("node_registry").is_none());
        // An unknown role is a miss, never a panic.
        assert!(reg.client("qahal").is_none());
    }

    #[test]
    fn clearing_a_slot_is_the_503_gap_the_supervisor_opens() {
        // The supervisor clears BEFORE reconnecting so routes answer 503
        // (retryable backpressure) instead of 502 (a broken node). Clearing an
        // already-empty slot, and clearing an unknown role, must both be no-ops.
        let reg = HcClientRegistry::empty();
        reg.set_client("infrastructure", None);
        reg.set_client("nonexistent-role", None);
        assert!(reg.infrastructure_client().is_none());
    }

    #[test]
    fn any_admin_websocket_fails_closed_on_an_empty_registry() {
        // The conductor-diagnostics fallback (`GET /db/p2p/conductor-
        // diagnostics` on an external-conductor node) must 503 honestly when
        // NO admin-capable connection exists — an empty registry yields None,
        // never a stale or fabricated handle. (The positive path — a live
        // role's admin handle — needs a real conductor and is exercised by
        // the a2o dataplane scenarios, not constructible offline.)
        let reg = HcClientRegistry::empty();
        assert!(reg.any_admin_websocket().is_none());
    }

    #[test]
    fn probe_interval_is_shorter_than_the_heartbeat_tick() {
        // The heartbeat (60s) only WARNS. The supervisor must probe strictly
        // more often than that, or the dead window is bounded by the thing that
        // never fixes anything.
        assert!(BRIDGE_PROBE_INTERVAL < Duration::from_secs(60));
        assert!(BRIDGE_PROBE_INTERVAL >= Duration::from_secs(5));
    }

    #[tokio::test]
    async fn spawn_bridge_supervisor_needs_no_policy_config() {
        // Regression guard for the corollary defect: `spawn_bridge_supervisor`
        // used to be called from inside `PolicyConfig::load`'s `Ok(..)` arm in
        // main.rs, so a peer started without ELOHIM_STORAGE_PEER_POLICY_PATH
        // (or pointed at a missing/malformed policy file — the common shape for
        // a throwaway or fresh dev conductor) got NO bridge supervision at all,
        // reproducing the original stale-token defect exactly: a restarted
        // conductor stayed dead forever with nothing watching it. The
        // function's own signature never took a `PolicyConfig` — proven here by
        // construction: it spawns from nothing but a registry + connection
        // inputs + a shutdown sender.
        let reg = std::sync::Arc::new(HcClientRegistry::empty());
        let (shutdown_tx, _rx) = tokio::sync::broadcast::channel::<()>(1);
        reg.spawn_bridge_supervisor(
            HcRegistryInputs {
                admin_url: "ws://127.0.0.1:1".into(),
                app_url: "ws://127.0.0.1:1".into(),
                app_id: "elohim".into(),
                lineage: std::sync::Arc::new(LineageRoles::new("elohim", &SUPERVISED_ROLES)),
            },
            shutdown_tx.clone(),
        );
        // Signal shutdown immediately so the spawned per-role loops exit
        // promptly rather than sleeping out BRIDGE_PROBE_INTERVAL during the
        // test run — this test asserts the call succeeds without a
        // PolicyConfig, not the supervisor's steady-state probing behavior.
        let _ = shutdown_tx.send(());
    }
    // ---- the membership-informed tick decision (2026-09-22) ---------------

    use crate::conductor_bridge_health::AppEnableEvidence;
    use crate::services::enable_app_backoff::EnableLedger;

    /// THE CURE, as its decision table.
    ///
    /// The MUTATION column is decided by the app-status evidence alone;
    /// membership only adds probe-now (it says running) and observe-only (it
    /// PROVES the cell absent, so a probe would fail by construction).
    #[test]
    fn the_tick_action_follows_the_status_evidence_and_membership_refines_it() {
        use crate::conductor_bridge_health::RoleCellState::*;
        use AppEnableEvidence::*;
        use NotRunningAction::*;
        let rows = [
            // (state, enable evidence, ladder open) -> action
            // Membership says RUNNING: probe now, whatever else is true.
            (RunningRecoveryUnverified, AlreadyEnabled, false, ProbeNow),
            (RunningRecoveryUnverified, EnableCanLift, true, ProbeNow),
            (RunningRecoveryUnverified, Unknown, false, ProbeNow),
            // enable CAN lift: the pre-existing ladder, unchanged.
            (
                InstalledNotRunningAppDisabled,
                EnableCanLift,
                true,
                SpendRungEnableAndProbe,
            ),
            (
                InstalledNotRunningAppDisabled,
                EnableCanLift,
                false,
                WaitForWindow,
            ),
            (
                MembershipUnknown,
                EnableCanLift,
                true,
                SpendRungEnableAndProbe,
            ),
            (MembershipUnknown, EnableCanLift, false, WaitForWindow),
            (MembershipUnknown, Unknown, true, SpendRungEnableAndProbe),
            (
                InstalledNotRunningStatusUnknown,
                Unknown,
                true,
                SpendRungEnableAndProbe,
            ),
            // STRANDED: membership PROVES absent and enable is a no-op → nothing.
            (
                InstalledNotRunningAppEnabled,
                AlreadyEnabled,
                true,
                ObserveOnly,
            ),
            (
                InstalledNotRunningAppEnabled,
                AlreadyEnabled,
                false,
                ObserveOnly,
            ),
            // enable REFUSED and membership proves absent → also nothing.
            (
                InstalledNotRunningAppDisabled,
                EnableRefused,
                true,
                ObserveOnly,
            ),
            // enable cannot act but membership is NOT established: the probe is
            // still the only recovery evidence a quiet role has, so it rides a
            // rung rather than being withheld forever.
            (MembershipUnknown, AlreadyEnabled, true, SpendRungProbeOnly),
            (MembershipUnknown, EnableRefused, true, SpendRungProbeOnly),
            (MembershipUnknown, AlreadyEnabled, false, WaitForWindow),
        ];
        for (state, enable, open, expected) in rows {
            assert_eq!(
                decide_not_running_action(state, enable, open),
                expected,
                "state={} enable={} ladder_open={open}",
                state.as_str(),
                enable.as_str()
            );
        }
    }

    /// F4's failing scenario, as its own test: `ping()` just observed `Enabled`
    /// and the FIRST membership read failed. The status evidence must still
    /// suppress the mutation — folding it into `membership-unknown` was how a
    /// rung got spent on the fork's proven no-op.
    #[test]
    fn a_known_enabled_app_never_spends_a_rung_even_with_unknown_membership() {
        use crate::conductor_bridge_health::RoleCellState;
        let action = decide_not_running_action(
            RoleCellState::MembershipUnknown,
            AppEnableEvidence::AlreadyEnabled,
            true,
        );
        assert_eq!(
            action,
            NotRunningAction::SpendRungProbeOnly,
            "the rung buys the PROBE (the only recovery evidence a quiet role has) and \
             emphatically not the enable"
        );
        assert!(action.spends_a_rung(), "a probe rung is still a rung");
    }

    /// And `AwaitingMemproofs` is the same story from the other side: not
    /// Enabled, and still never enabled, because this pin rejects it outright.
    #[test]
    fn awaiting_memproofs_never_spends_an_enable() {
        use crate::conductor_bridge_health::RoleCellState;
        for state in [
            RoleCellState::InstalledNotRunningAppDisabled,
            RoleCellState::MembershipUnknown,
        ] {
            let action = decide_not_running_action(state, AppEnableEvidence::EnableRefused, true);
            assert_ne!(
                action,
                NotRunningAction::SpendRungEnableAndProbe,
                "{}: the conductor REFUSES to enable this status",
                state.as_str()
            );
        }
    }

    /// The genuinely-disabled case's ladder is UNCHANGED by this cure: one
    /// attempt then 60s → 120 → 240, exactly as `enable_app_backoff` pins it.
    #[test]
    fn a_genuinely_disabled_role_keeps_the_existing_ladder() {
        use crate::conductor_bridge_health::RoleCellState;
        let ledger = EnableLedger::new();
        let mut now = std::time::Instant::now();

        let mut windows = Vec::new();
        for _ in 0..3 {
            assert_eq!(
                decide_not_running_action(
                    RoleCellState::InstalledNotRunningAppDisabled,
                    AppEnableEvidence::EnableCanLift,
                    ledger.should_attempt_at("lamad", now),
                ),
                NotRunningAction::SpendRungEnableAndProbe,
                "the window is open, so the rung is spent"
            );
            ledger.note_attempt_at("lamad", now);
            let window =
                crate::services::enable_app_backoff::enable_backoff(ledger.attempts("lamad"));
            windows.push(window.as_secs());

            // Every 20s probe inside the window is refused, as before.
            let mut probe_at = now + BRIDGE_PROBE_INTERVAL;
            while probe_at < now + window {
                assert_eq!(
                    decide_not_running_action(
                        RoleCellState::InstalledNotRunningAppDisabled,
                        AppEnableEvidence::EnableCanLift,
                        ledger.should_attempt_at("lamad", probe_at),
                    ),
                    NotRunningAction::WaitForWindow
                );
                probe_at += BRIDGE_PROBE_INTERVAL;
            }
            now += window;
        }
        assert_eq!(
            windows,
            vec![60, 120, 240],
            "the pre-existing ladder, untouched by the membership join"
        );
    }

    // ---- F7: the EXECUTOR, driven through an injected actuator -------------

    /// Records what production actually asked the conductor to do.
    #[derive(Default)]
    struct RecordingActuator {
        enables: std::sync::Mutex<Vec<(String, u32)>>,
        probes: std::sync::Mutex<Vec<String>>,
    }

    impl RecordingActuator {
        fn enable_calls(&self) -> Vec<(String, u32)> {
            self.enables
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        }
        fn probe_calls(&self) -> Vec<String> {
            self.probes
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        }
    }

    #[async_trait::async_trait]
    impl NotRunningActuator for RecordingActuator {
        async fn enable_app(&self, role: &str, attempt: u32) {
            self.enables
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push((role.to_string(), attempt));
        }
        async fn probe_cell(&self, role: &str) {
            self.probes
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(role.to_string());
        }
    }

    /// NEW-2's action half: a success landing between the classification and the
    /// action ABORTS the action, rather than enabling an app that is serving.
    ///
    /// A decision is a statement about a moment. The tick classifies from a
    /// snapshot, and between that snapshot and the actuator a zome call can
    /// RETURN — at which point the role is serving, the episode is over and the
    /// ladder has been cleared, while this task still holds a plan to enable and
    /// probe. One coherent re-read under the record's lock settles it.
    #[tokio::test]
    async fn an_action_classified_before_a_success_is_aborted() {
        use crate::conductor_bridge_health::{record_role_success, role_bridge_health};
        const ROLE: &str = "test-action-aborted-by-later-success";
        let ledger = EnableLedger::new();

        // The tick classifies while nothing has succeeded…
        let classified_from = role_bridge_health()
            .for_role(ROLE)
            .observe()
            .last_success_seq;
        // …and a zome call RETURNS before the action runs.
        record_role_success(ROLE);

        let actuator = RecordingActuator::default();
        let outcome = execute_not_running_action_at(
            NotRunningAction::SpendRungEnableAndProbe,
            ROLE,
            false,
            &ledger,
            &actuator,
            std::time::Instant::now(),
            classified_from,
        )
        .await;

        assert!(
            matches!(outcome, ActionOutcome::Superseded { .. }),
            "the plan describes a world that has moved on: {outcome:?}"
        );
        assert_eq!(ledger.attempts(ROLE), 0, "no rung was spent");
        assert!(
            actuator.enable_calls().is_empty(),
            "and the conductor was not asked to enable an app that is serving"
        );
        assert!(actuator.probe_calls().is_empty());

        // The SAME action, classified from the current world, runs normally.
        let now_seq = role_bridge_health()
            .for_role(ROLE)
            .observe()
            .last_success_seq;
        let outcome = execute_not_running_action_at(
            NotRunningAction::SpendRungEnableAndProbe,
            ROLE,
            false,
            &ledger,
            &actuator,
            std::time::Instant::now(),
            now_seq,
        )
        .await;
        assert_eq!(outcome, ActionOutcome::Executed);
        assert_eq!(ledger.attempts(ROLE), 1);
        assert_eq!(actuator.enable_calls().len(), 1);
    }

    /// ITEM 3: a success landing while the enable RPC is IN FLIGHT retracts the
    /// rung.
    ///
    /// The pre-dispatch re-check cannot close this window — the RPC is an await
    /// and the guard is not held across it. So the ledger entry is captured
    /// under the guard before the attempt and restored after, and the residual
    /// is stated exactly: one no-op enable RPC against an already-serving
    /// conductor, and NO rung.
    #[tokio::test]
    async fn a_success_landing_during_the_enable_rpc_retracts_the_rung() {
        use crate::conductor_bridge_health::record_role_success;
        const ROLE: &str = "test-rung-retracted-when-success-lands-mid-rpc";
        // Recovery and executor must use the SAME ledger, as production does.
        // An empty local ledger hid resurrection of six pre-recovery attempts.
        let ledger = crate::services::enable_app_backoff::enable_ledger();
        let now = std::time::Instant::now();
        for _ in 0..6 {
            ledger.note_attempt_at(ROLE, now - std::time::Duration::from_secs(7_200));
        }
        const SIBLING: &str = "test-rung-retraction-unrelated-role";
        ledger.note_attempt_at(SIBLING, now);
        let sibling_record = ledger.record_for(SIBLING);

        /// An actuator whose `enable_app` lands a zome-call success while the
        /// RPC is still in flight — the schedule the re-check cannot see.
        struct SuccessDuringRpc {
            inner: RecordingActuator,
        }
        #[async_trait::async_trait]
        impl NotRunningActuator for SuccessDuringRpc {
            async fn enable_app(&self, role: &str, attempt: u32) {
                self.inner.enable_app(role, attempt).await;
                // …and the call returns while we are still awaiting.
                record_role_success(role);
            }
            async fn probe_cell(&self, role: &str) {
                self.inner.probe_cell(role).await;
            }
        }

        let classified_from = crate::conductor_bridge_health::role_bridge_health()
            .for_role(ROLE)
            .observe()
            .last_success_seq;
        let actuator = SuccessDuringRpc {
            inner: RecordingActuator::default(),
        };
        let outcome = execute_not_running_action_at(
            NotRunningAction::SpendRungEnableAndProbe,
            ROLE,
            false,
            ledger,
            &actuator,
            std::time::Instant::now(),
            classified_from,
        )
        .await;

        assert!(
            matches!(outcome, ActionOutcome::Superseded { .. }),
            "the tick is superseded by the call that landed: {outcome:?}"
        );
        assert_eq!(
            ledger.attempts(ROLE),
            0,
            "THE RUNG IS RETRACTED — charging the ladder for a no-op enable would hand the next \
             outage a backoff window this tick did not earn"
        );
        assert_eq!(ledger.record_for(SIBLING), sibling_record);
        ledger.note_attempt_at(ROLE, now);
        assert_eq!(ledger.attempts(ROLE), 1, "a relapse starts a fresh ladder");
        assert!(ledger.should_attempt_at(ROLE, now + std::time::Duration::from_secs(60)));
        ledger.note_running(ROLE);
        ledger.note_running(SIBLING);
        assert!(
            ledger.should_attempt_at(ROLE, std::time::Instant::now()),
            "and the ladder is CLEAR, not merely backed off"
        );
        // The stated residual: the RPC itself did happen, exactly once.
        assert_eq!(
            actuator.inner.enable_calls().len(),
            1,
            "one no-op enable RPC against a serving app is the residual, and it is bounded"
        );
        assert!(
            actuator.inner.probe_calls().is_empty(),
            "and nothing further was asked of a role that is already serving"
        );
    }

    /// THE ZERO-RUNG ASSERTION, on the production branch.
    ///
    /// The earlier version of this test called only the pure decision function,
    /// so corrupting the `ObserveOnly` arm to increment the ledger or call
    /// `enable_app` could not have failed it. This drives the real executor.
    #[tokio::test]
    async fn observe_only_calls_nothing_and_touches_no_ledger() {
        let ledger = EnableLedger::new();
        let actuator = RecordingActuator::default();
        for tick in 0..30 {
            execute_not_running_action(
                NotRunningAction::ObserveOnly,
                "lamad",
                false,
                &ledger,
                &actuator,
                u64::MAX,
            )
            .await;
            assert_eq!(
                ledger.attempts("lamad"),
                0,
                "tick {tick}: ObserveOnly must not advance the ladder"
            );
        }
        assert!(
            actuator.enable_calls().is_empty(),
            "ObserveOnly called enable_app: {:?}",
            actuator.enable_calls()
        );
        assert!(
            actuator.probe_calls().is_empty(),
            "ObserveOnly probed a cell membership PROVED absent: {:?}",
            actuator.probe_calls()
        );
        assert!(
            ledger.should_attempt_at("lamad", std::time::Instant::now()),
            "and the ladder is CLEAR, not merely backed off"
        );
    }

    /// `WaitForWindow` is the OTHER no-op, and it must be distinguishable: it
    /// also asks nothing, but it means the ladder was entered and is backing off.
    #[tokio::test]
    async fn wait_for_window_calls_nothing_either() {
        let ledger = EnableLedger::new();
        let actuator = RecordingActuator::default();
        execute_not_running_action(
            NotRunningAction::WaitForWindow,
            "lamad",
            false,
            &ledger,
            &actuator,
            // No success can outrank this: the executor's supersession
            // re-check is exercised by its own test, and these pin the ACTIONS.
            u64::MAX,
        )
        .await;
        assert_eq!(ledger.attempts("lamad"), 0);
        assert!(actuator.enable_calls().is_empty());
        assert!(actuator.probe_calls().is_empty());
    }

    #[tokio::test]
    async fn spend_rung_enable_and_probe_calls_both_and_advances_the_ladder() {
        let ledger = EnableLedger::new();
        let actuator = RecordingActuator::default();
        execute_not_running_action(
            NotRunningAction::SpendRungEnableAndProbe,
            "lamad",
            false,
            &ledger,
            &actuator,
            // No success can outrank this: the executor's supersession
            // re-check is exercised by its own test, and these pin the ACTIONS.
            u64::MAX,
        )
        .await;
        assert_eq!(ledger.attempts("lamad"), 1, "the rung was spent");
        assert_eq!(actuator.enable_calls(), vec![("lamad".to_string(), 1)]);
        assert_eq!(actuator.probe_calls(), vec!["lamad".to_string()]);

        // A second rung carries the incremented attempt number.
        execute_not_running_action(
            NotRunningAction::SpendRungEnableAndProbe,
            "lamad",
            false,
            &ledger,
            &actuator,
            // No success can outrank this: the executor's supersession
            // re-check is exercised by its own test, and these pin the ACTIONS.
            u64::MAX,
        )
        .await;
        assert_eq!(
            actuator.enable_calls(),
            vec![("lamad".to_string(), 1), ("lamad".to_string(), 2)]
        );
    }

    /// A cross-cell role has no app of its own, so its rung buys the probe only
    /// — asserted as a CALL, not as a comment.
    #[tokio::test]
    async fn a_cross_cell_rung_buys_the_probe_and_never_an_enable() {
        let ledger = EnableLedger::new();
        let actuator = RecordingActuator::default();
        execute_not_running_action(
            NotRunningAction::SpendRungEnableAndProbe,
            crate::hc_client::MISHPAT_ROLE,
            true,
            &ledger,
            &actuator,
            // No success can outrank this: the executor's supersession
            // re-check is exercised by its own test, and these pin the ACTIONS.
            u64::MAX,
        )
        .await;
        assert_eq!(ledger.attempts(crate::hc_client::MISHPAT_ROLE), 1);
        assert!(
            actuator.enable_calls().is_empty(),
            "mishpat is a CELL of another app — there is no app of its own to enable"
        );
        assert_eq!(
            actuator.probe_calls(),
            vec![crate::hc_client::MISHPAT_ROLE.to_string()]
        );
    }

    /// `SpendRungProbeOnly` spends a rung and calls the probe, and NEVER the
    /// enable — that is the whole reason the variant exists.
    #[tokio::test]
    async fn spend_rung_probe_only_never_calls_enable() {
        let ledger = EnableLedger::new();
        let actuator = RecordingActuator::default();
        execute_not_running_action(
            NotRunningAction::SpendRungProbeOnly,
            "imagodei",
            false,
            &ledger,
            &actuator,
            // No success can outrank this: the executor's supersession
            // re-check is exercised by its own test, and these pin the ACTIONS.
            u64::MAX,
        )
        .await;
        assert_eq!(
            ledger.attempts("imagodei"),
            1,
            "a probe rung is still a rung"
        );
        assert!(
            actuator.enable_calls().is_empty(),
            "the app is already Enabled (or refuses enabling) — enable_app is a proven no-op"
        );
        assert_eq!(actuator.probe_calls(), vec!["imagodei".to_string()]);
    }

    /// `ProbeNow` probes OFF the ladder: no rung, whatever the ladder has
    /// climbed to. Making this wait for a rung is how a recovered quiet role sat
    /// red for up to an hour at the cap.
    #[tokio::test]
    async fn probe_now_probes_off_the_ladder() {
        let ledger = EnableLedger::new();
        let t0 = std::time::Instant::now();
        // Climb to the cap so the ladder is as shut as it ever gets.
        for step in 0..8 {
            ledger.note_attempt_at(
                "imagodei",
                t0 + std::time::Duration::from_secs(step * 4_000),
            );
        }
        let last_attempt = t0 + std::time::Duration::from_secs(7 * 4_000);
        let now = last_attempt + BRIDGE_PROBE_INTERVAL;
        assert_eq!(
            crate::services::enable_app_backoff::enable_backoff(ledger.attempts("imagodei")),
            crate::services::enable_app_backoff::ENABLE_BACKOFF_CAP,
            "precondition: eight attempts put the ladder at its 1h cap"
        );
        assert!(
            !ledger.should_attempt_at("imagodei", now),
            "precondition: 20s after the last attempt, the 1h window is shut"
        );

        let before = ledger.attempts("imagodei");
        let actuator = RecordingActuator::default();
        execute_not_running_action(
            NotRunningAction::ProbeNow,
            "imagodei",
            false,
            &ledger,
            &actuator,
            // No success can outrank this: the executor's supersession
            // re-check is exercised by its own test, and these pin the ACTIONS.
            u64::MAX,
        )
        .await;
        assert_eq!(
            ledger.attempts("imagodei"),
            before,
            "ProbeNow must not consume a rung"
        );
        assert!(actuator.enable_calls().is_empty());
        assert_eq!(actuator.probe_calls(), vec!["imagodei".to_string()]);
    }

    // ---- F1 / the integration sequence: driven through the real cache -------

    /// A cell id for a role under test.
    fn registry_test_cell(seed: u8) -> holochain_client::CellId {
        use holochain_types::prelude::{AgentPubKey, DnaHash};
        holochain_client::CellId::new(
            DnaHash::from_raw_32(vec![seed; 32]),
            AgentPubKey::from_raw_32(vec![seed.wrapping_add(19); 32]),
        )
    }

    /// F1's failing scenario, driven through the REAL cache: a successful
    /// membership read, then a conductor restart that RE-MINTS the bridge (the
    /// production `invalidate` call), then repeated `ListCellIds` failures.
    ///
    /// The ladder MUST still progress. Before the authority window a stale
    /// `Some(true)` classified this as "membership present" and answered
    /// `ProbeNow` forever without ever enabling. Nothing here hands the
    /// classifier a literal `None`: every membership answer comes out of
    /// `MembershipCache` after real revocation and real failed reads.
    #[tokio::test]
    async fn a_restart_with_a_disabled_app_and_unreadable_membership_still_ladders() {
        use crate::conductor_bridge_health::{classify_cell_state, RoleCellState};
        use crate::services::cell_membership::{
            fake::FakeAdmin, MembershipCache, RefreshOutcome, MEMBERSHIP_AUTHORITY_TTL,
        };
        const ROLE: &str = "test-restart-disabled-unreadable-ladders";

        let cell = registry_test_cell(61);
        let cache = MembershipCache::new();
        let admin = FakeAdmin::new(Ok(vec![cell.clone()]));
        let base: u64 = 1_700_000_000_000;

        // BEFORE the restart: a real read says the cell IS running.
        assert_eq!(
            cache.refresh_if_stale_at(&admin, base.into()).await,
            RefreshOutcome::Read { running: 1 }
        );
        assert_eq!(cache.cell_running_at(&cell, base.into()), Some(true));

        // The conductor restarts; the supervisor clears the handle and revokes
        // the reading with the bridge that produced it.
        cache.invalidate_at(
            base + 1_000,
            "the bridge died and is being re-minted (test)",
        );
        // The app comes back genuinely Disabled, so every later membership read
        // fails against a conductor whose cells never joined.
        admin.set_tail(Err("ListCellIds failed: K2SpaceNotFound".to_string()));

        let ledger = EnableLedger::new();
        let actuator = RecordingActuator::default();
        let mut mono = std::time::Instant::now();
        let mut wall = base + 1_000;
        let mut windows = Vec::new();
        // Nothing has ever succeeded on this role, so no success can outrank
        // the classification; the supersession re-check has its own test.

        for rung in 0..3 {
            wall += 20_000;
            let outcome = cache.refresh_if_stale_at(&admin, wall.into()).await;
            assert!(
                matches!(outcome, RefreshOutcome::Failed { .. }),
                "rung {rung}: the membership read really is failing, got {outcome:?}"
            );
            let membership = cache.cell_running_at(&cell, wall.into());
            assert_eq!(
                membership, None,
                "rung {rung}: a revoked reading decides nothing — this is where the stale true was"
            );
            let state = classify_cell_state(membership, AppEnableEvidence::EnableCanLift);
            assert_eq!(state, RoleCellState::MembershipUnknown);

            let action = decide_not_running_action(
                state,
                AppEnableEvidence::EnableCanLift,
                ledger.should_attempt_at(ROLE, mono),
            );
            assert_eq!(
                action,
                NotRunningAction::SpendRungEnableAndProbe,
                "rung {rung}: the app is Disabled and enable_app lifts it — an unreadable \
                 membership must not disable the cure"
            );
            // Stamp the rung at the SAME synthetic instant the decision read.
            execute_not_running_action_at(action, ROLE, false, &ledger, &actuator, mono, u64::MAX)
                .await;
            let window = crate::services::enable_app_backoff::enable_backoff(ledger.attempts(ROLE));
            windows.push(window.as_secs());
            mono += window;
        }
        assert_eq!(
            windows,
            vec![60, 120, 240],
            "the ladder PROGRESSES rather than being pinned by a stale membership true"
        );
        assert_eq!(
            actuator.enable_calls().len(),
            3,
            "three real enable_app attempts: {:?}",
            actuator.enable_calls()
        );

        // The OTHER way authority is lost — plain expiry, no re-mint — reaches
        // the same decision. A read succeeds, then nothing refreshes it.
        let fresh = MembershipCache::new();
        let quiet = FakeAdmin::new(Ok(vec![cell.clone()]));
        fresh.refresh_if_stale_at(&quiet, base.into()).await;
        let expired = base + MEMBERSHIP_AUTHORITY_TTL.as_millis() as u64;
        assert_eq!(fresh.cell_running_at(&cell, expired.into()), None);
        assert_eq!(
            decide_not_running_action(
                classify_cell_state(
                    fresh.cell_running_at(&cell, expired.into()),
                    AppEnableEvidence::EnableCanLift
                ),
                AppEnableEvidence::EnableCanLift,
                true,
            ),
            NotRunningAction::SpendRungEnableAndProbe,
            "an EXPIRED reading must not disable the cure either"
        );
    }

    /// THE FLEET SEQUENCE, END TO END IN ONE PROCESS.
    ///
    /// successful membership → conductor restart (bridge re-mint invalidates the
    /// reading) → persisted Disabled app → repeated `ListCellIds` failures →
    /// real enable progression observed on the recording actuator → a zome call
    /// that RETURNS → gauges correct → and the recovered, QUIET role keeps a
    /// cleared state family on its next tick.
    ///
    /// ## EXACTLY WHAT THIS TEST DOES NOT COVER
    ///
    /// Stated as a list rather than a disclaimer, because "deleting those
    /// production connections can leave this test green" is a true and useful
    /// thing to know about it:
    ///
    /// 1. **Supervisor scheduling** — `spawn_bridge_supervisor`'s per-role
    ///    `tokio::spawn` loop, its `select!` on `BRIDGE_PROBE_INTERVAL` vs
    ///    shutdown, and the `role == CROSS_CELL_DRIVER_ROLE` adoption of
    ///    `mishpat`. Here the ticks are a `for` loop.
    /// 2. **`ping()` classification** — the three-way
    ///    `Running` / `NotRunning` / `Err` triage that decides between the
    ///    ladder, `enable_app` and a full bridge re-mint.
    /// 3. **Reconnect / invalidation wiring** — `registry.set_client(role,
    ///    None)`, `record_role_reconnect`, `membership().invalidate(...)` and
    ///    `connect_role_forever`. This test calls the cache's `invalidate_at`
    ///    directly; it does not prove the loop calls it.
    /// 4. **`SupervisorActuator`** — the production actuator. This drives
    ///    `RecordingActuator`, so the real `enable_app` RPC and the real
    ///    `cell_probe::probe_cell` are not exercised.
    /// 5. **A probe result reaching the success hook** — the probe's zome call
    ///    returning and landing in `record_role_success`. Here
    ///    `record_role_success` is called directly.
    ///
    /// Stated the other way round, as the r4 review put it: this test **cannot
    /// detect** the deletion of supervisor scheduling, of `ping()`
    /// classification, of the reconnect/invalidation wiring, of
    /// `SupervisorActuator`'s real RPCs, or of probe-result delivery to the
    /// success hook. The conductor transitions are SUPPLIED here (the fake's
    /// answer is changed by the test) and the successful call is a direct hook
    /// call — so neither this test nor the doorway scenario demonstrates actual
    /// fleet recovery through the affected workers. What it does pin is the
    /// decision spine between those points.
    ///
    /// ## Why the cheap seam was NOT taken
    ///
    /// Driving the loop in-process needs the registry to be able to HOLD a fake,
    /// i.e. `Arc<dyn HcClientLike>` in the slots rather than `Arc<HcClient>`.
    /// Measured on this tree: `Arc<HcClient>` appears **149 times across 38
    /// files**, with **59** call sites on the registry's own accessors
    /// (`lamad_client()`, `imagodei_client()`, `client(role)`), and every one of
    /// them calls concrete inherent methods (`call_zome`, `cell_id`,
    /// `dna_hash`, `mishpat_cell_id`). The loop also reaches
    /// `cell_probe::probe_cell(role, &HcClient, gate)` and
    /// `admin_websocket() -> holochain_client::AdminWebsocket`, so the trait
    /// would need a second trait behind it for `enable_app`. That is an order of
    /// magnitude past the ~150-line budget this was weighed against, and it
    /// would be a refactor of the crate's conductor handle rather than a test
    /// seam — so it is NOT done here, deliberately, and the five items above
    /// stay a2o territory
    /// (`features/doorway/peer-conductor-connection-resilience.feature`).
    ///
    /// EVERYTHING BETWEEN those points is production code driven here: the
    /// membership cache, the publication path
    /// (`observe_and_publish_cell_state_from`, which the loop calls directly),
    /// the contradiction gate, the decision (`decide_not_running_action`), the
    /// executor (`execute_not_running_action_at`) and the recovery fold
    /// (`record_role_success`).
    #[tokio::test]
    async fn the_whole_restart_to_recovery_sequence_on_one_role() {
        use crate::conductor_bridge_health::{
            self as health, AppRunObservation, RoleCellState, ROLE_CELL_STATES,
        };
        use crate::services::cell_membership::{fake::FakeAdmin, MembershipCache};
        const ROLE: &str = "test-integration-restart-to-recovery";

        let cell = registry_test_cell(71);
        let sibling = registry_test_cell(72);
        let cache = MembershipCache::new();
        let admin = FakeAdmin::new(Ok(vec![cell.clone(), sibling.clone()]));

        let running = crate::metrics::CONDUCTOR_CELL_RUNNING.with_label_values(&[ROLE]);
        let family = |state: RoleCellState| {
            crate::metrics::CONDUCTOR_CELL_STATE.with_label_values(&[ROLE, state.as_str()])
        };
        let tick = |wall_ms: u64| {
            let (cache, admin, cell) = (&cache, &admin, &cell);
            async move {
                HcClientRegistry::observe_and_publish_cell_state_from(
                    ROLE,
                    admin,
                    Some(cell),
                    cache,
                    crate::services::cell_membership::ObservedAt::from(wall_ms),
                )
                .await
            }
        };

        // ── (0) SERVING. A call has landed and membership agrees.
        health::record_role_success(ROLE);
        let t0 = health::now_ms() + 5_000;
        assert_eq!(
            tick(t0).await.state,
            RoleCellState::RunningRecoveryUnverified
        );
        assert_eq!(running.get(), 1);
        for state in ROLE_CELL_STATES {
            assert_eq!(
                family(state).get(),
                0,
                "a serving role's WHY-family stays cleared ({})",
                state.as_str()
            );
        }

        // ── (1) THE CONDUCTOR RESTARTS. The bridge dies, is re-minted, and the
        // reading is revoked with it; every later ListCellIds fails.
        health::record_role_reconnect(ROLE);
        cache.invalidate_at(t0 + 1_000, "the bridge died and is being re-minted (test)");
        admin.set_tail(Err(
            "ListCellIds failed: Websocket closed: No connection".to_string()
        ));

        // ── (2) THE APP IS PERSISTED DISABLED, and a zome call says so.
        health::observe_role_app_status(
            ROLE,
            &AppRunObservation::NotRunning {
                reason: "disabled: by an operator through the admin interface".to_string(),
                enable: AppEnableEvidence::EnableCanLift,
            },
        );
        assert!(
            health::role_is_not_running(ROLE),
            "the episode is open — this is what the ladder is paced against"
        );

        // ── (3) REPEATED MEMBERSHIP-READ FAILURES, and the ladder progresses.
        //
        // The PROCESS-WIDE ledger, not a local one: `record_role_success` below
        // resets the ladder through `enable_app_backoff::enable_ledger()`, which
        // is the ledger the supervisor spends rungs against. A local ledger
        // would be reset by nothing and the recovery assertion would be a
        // fiction. Keyed by role, and ROLE is unique to this test.
        let ledger = crate::services::enable_app_backoff::enable_ledger();
        let actuator = RecordingActuator::default();
        let mut mono = std::time::Instant::now();
        let mut wall = t0 + 1_000;
        let mut windows = Vec::new();
        for rung in 0..3 {
            wall += 20_000;
            let observed = tick(wall).await;
            let (state, enable) = (observed.state, observed.enable);
            let tick_seq = observed.classified_from_success_seq;
            assert_eq!(
                state,
                RoleCellState::MembershipUnknown,
                "rung {rung}: the read failed, so membership is not established"
            );
            assert_eq!(
                running.get(),
                -1,
                "rung {rung}: an unreadable membership publishes -1, NEVER a 0 that reads as a \
                 diagnosis"
            );
            assert_eq!(family(RoleCellState::MembershipUnknown).get(), 1);
            let action =
                decide_not_running_action(state, enable, ledger.should_attempt_at(ROLE, mono));
            assert_eq!(action, NotRunningAction::SpendRungEnableAndProbe);
            execute_not_running_action_at(action, ROLE, false, ledger, &actuator, mono, tick_seq)
                .await;
            let window = crate::services::enable_app_backoff::enable_backoff(ledger.attempts(ROLE));
            windows.push(window.as_secs());
            mono += window;
        }
        assert_eq!(windows, vec![60, 120, 240]);
        assert_eq!(
            actuator.enable_calls(),
            vec![
                (ROLE.to_string(), 1),
                (ROLE.to_string(), 2),
                (ROLE.to_string(), 3)
            ],
            "three REAL enable_app attempts on the recording actuator"
        );
        assert_eq!(
            actuator.probe_calls().len(),
            3,
            "each rung also bought a probe"
        );

        // ── (4) THE CONDUCTOR FINISHES STARTING. Membership reads again, the
        // probe is owed OFF the ladder, and the probe's call returns.
        admin.set_tail(Ok(vec![cell.clone(), sibling.clone()]));
        wall += 20_000;
        let observed = tick(wall).await;
        let (state, enable) = (observed.state, observed.enable);
        let tick_seq = observed.classified_from_success_seq;
        assert_eq!(state, RoleCellState::RunningRecoveryUnverified);
        assert_eq!(running.get(), 1, "membership says the cell joined the map");
        let action = decide_not_running_action(state, enable, ledger.should_attempt_at(ROLE, mono));
        assert_eq!(
            action,
            NotRunningAction::ProbeNow,
            "this is the moment the episode can end, so the probe does not wait for a rung"
        );
        let rungs_before = ledger.attempts(ROLE);
        execute_not_running_action_at(action, ROLE, false, ledger, &actuator, mono, tick_seq).await;
        assert_eq!(
            ledger.attempts(ROLE),
            rungs_before,
            "ProbeNow spends no rung"
        );
        assert_eq!(actuator.probe_calls().len(), 4);

        // The probe's zome call RETURNS — the one transition this node accepts.
        health::record_role_success(ROLE);
        assert!(!health::role_is_not_running(ROLE));
        assert_eq!(
            ledger.attempts(ROLE),
            0,
            "proven recovery clears the ladder"
        );
        assert_eq!(running.get(), 1);
        for state in ROLE_CELL_STATES {
            assert_eq!(
                family(state).get(),
                0,
                "recovery zeroes the whole WHY-family ({})",
                state.as_str()
            );
        }

        // ── (5) THE RECOVERED ROLE IS QUIET. Its next tick reads a fresh
        // `Some(true)` and must NOT re-open the family recovery just cleared.
        wall += 20_000;
        assert_eq!(
            tick(wall).await.state,
            RoleCellState::RunningRecoveryUnverified
        );
        assert_eq!(running.get(), 1);
        for state in ROLE_CELL_STATES {
            assert_eq!(
                family(state).get(),
                0,
                "{} was re-opened on a recovered, quiet role — with no episode open no probe \
                 follows, so it would stand at 1 for the life of the process",
                state.as_str()
            );
        }
        assert!(
            !health::role_bridge_health().for_role(ROLE).has_cell_state(),
            "and nothing was latched, so no transition line promising a probe was written"
        );
    }
}
