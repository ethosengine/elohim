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
use tracing::{info, warn};

use crate::hc_client::{HcClient, HcClientConfig};

/// Role-keyed registry of HcClient connections. Fields hold `None` when
/// the role failed to connect at startup; downstream code returns 503
/// (`IMAGODEI_BRIDGE_OFFLINE` etc.) if the role is unavailable.
pub struct HcClientRegistry {
    pub infrastructure: Option<Arc<HcClient>>,
    pub imagodei: Option<Arc<HcClient>>,
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
        Self {
            infrastructure,
            imagodei,
        }
    }

    async fn connect_role(inputs: &HcRegistryInputs, role: &str) -> Option<Arc<HcClient>> {
        match HcClient::connect(HcClientConfig {
            admin_url: inputs.admin_url.clone(),
            app_url: inputs.app_url.clone(),
            app_id: inputs.app_id.clone(),
            role: Some(role.to_string()),
        })
        .await
        {
            Ok(hc) => {
                info!(role, "HcClient connected");
                Some(Arc::new(hc))
            }
            Err(e) => {
                warn!(
                    role,
                    error = %e,
                    "HcClient connect failed — routes for this role will return 503"
                );
                None
            }
        }
    }
}
