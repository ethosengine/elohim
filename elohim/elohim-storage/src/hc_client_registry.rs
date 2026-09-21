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
    /// 2. **The bounded ladder.** `enable_app` is an admin-plane write against
    ///    a conductor that is, by hypothesis, already unwell. One attempt per
    ///    [`crate::services::enable_app_backoff::enable_backoff`] window —
    ///    60s doubling to a 1h cap — never per 20s probe.
    /// 3. **The conductor's own answer.** On refusal the error is logged
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

        let ledger = crate::services::enable_app_backoff::enable_ledger();
        if !ledger.should_attempt(role) {
            return;
        }
        ledger.note_attempt(role);
        let attempt = ledger.attempts(role);

        // A cross-cell role (mishpat) is a CELL of the same installed app, so
        // there is no app of its own to enable — the enable its siblings make is
        // already the whole of that cure. Its rung buys the probe only.
        if !CROSS_CELL_ROLES.contains(&role) {
            Self::ask_the_conductor_to_enable(inputs, role, hc, reason, attempt).await;
        }

        // THEN one read-only question to the role's own cell, on this same rung.
        // Awaited inline, so this task makes exactly one and cannot start a
        // second before the first answers; the gate in `cell_probe` holds the
        // same invariant against any other driver.
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

                    // ADOPT THE ROLES WITH NO TASK OF THEIR OWN. `mishpat` is a
                    // cell of the same installed app reached through this
                    // client, so it has no slot and no supervisor loop — and
                    // before this it therefore had no cure at all: its episodes
                    // could only be ended by commitment traffic that a household
                    // may never generate. One driver, so the cadence is one
                    // ladder rather than four tasks racing for the same rung.
                    if role == CROSS_CELL_DRIVER_ROLE {
                        for cross in CROSS_CELL_ROLES {
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
}
