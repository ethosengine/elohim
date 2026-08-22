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

use crate::hc_client::{HcClient, HcClientConfig};

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
}

/// The roles the supervisor keeps alive. Ordered coldest-first, matching the
/// boot ramp: `infrastructure` is the role most likely to be `None`-stamped on
/// a slow conductor boot.
pub const SUPERVISED_ROLES: [&str; 3] = ["infrastructure", "imagodei", "lamad"];

/// Connection inputs. Mirrors the relevant CLI args without depending on
/// the Args struct directly (cleaner test surface).
#[derive(Debug, Clone)]
pub struct HcRegistryInputs {
    pub admin_url: String,
    pub app_url: String,
    pub app_id: String,
}

impl HcClientRegistry {
    /// Connect each role in sequence. Per-role failure is logged and
    /// returns `None` for that role — the registry as a whole always
    /// constructs.
    pub async fn connect(inputs: &HcRegistryInputs) -> Self {
        let infrastructure = Self::connect_role(inputs, "infrastructure").await;
        let imagodei = Self::connect_role(inputs, "imagodei").await;
        let lamad = Self::connect_role(inputs, "lamad").await;
        Self {
            infrastructure: RwLock::new(infrastructure),
            imagodei: RwLock::new(imagodei),
            lamad: RwLock::new(lamad),
        }
    }

    /// A registry with every role unconnected — the shape a node has before any
    /// bridge lands, and the shape tests construct.
    pub fn empty() -> Self {
        Self {
            infrastructure: RwLock::new(None),
            imagodei: RwLock::new(None),
            lamad: RwLock::new(None),
        }
    }

    fn slot(&self, role: &str) -> Option<&RwLock<Option<Arc<HcClient>>>> {
        match role {
            "infrastructure" => Some(&self.infrastructure),
            "imagodei" => Some(&self.imagodei),
            "lamad" => Some(&self.lamad),
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
            app_id: inputs.app_id.clone(),
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
            app_id: inputs.app_id.clone(),
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

                    // `ping` folds its own result into the zome-path observer,
                    // so a node with zero zome traffic still reports honestly.
                    if hc.ping().await.is_ok() {
                        continue;
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

    #[test]
    fn every_slot_starts_empty_and_reads_back_none() {
        let reg = HcClientRegistry::empty();
        assert!(reg.infrastructure_client().is_none());
        assert!(reg.imagodei_client().is_none());
        assert!(reg.lamad_client().is_none());
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
