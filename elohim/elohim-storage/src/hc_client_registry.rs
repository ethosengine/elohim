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

use std::sync::Arc;
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
    attempt == 1 || attempt % 5 == 0
}

/// Role-keyed registry of HcClient connections. Fields hold `None` when
/// the role failed to connect at startup; downstream code returns 503
/// (`IMAGODEI_BRIDGE_OFFLINE` etc.) if the role is unavailable.
pub struct HcClientRegistry {
    pub infrastructure: Option<Arc<HcClient>>,
    pub imagodei: Option<Arc<HcClient>>,
    /// `lamad` role — hosts the `content_store` zome (REA commitments,
    /// content rows, attestations). Required for the conductor-first HTTP
    /// write path landing per 2026-05-26-substrate-rea-replication-fix.md
    /// (closes Gap C/D — REA + content row replication on alpha).
    pub lamad: Option<Arc<HcClient>>,
}

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
            infrastructure,
            imagodei,
            lamad,
        }
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
