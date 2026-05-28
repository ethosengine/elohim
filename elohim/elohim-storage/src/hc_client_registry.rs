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
                        "HcClient connect failed after all attempts — routes for this role will return 503"
                    );
                    return None;
                }
            }
        }
        None
    }
}
