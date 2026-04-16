//! hApp Lifecycle Manager
//!
//! Handles first install, stale detection, and re-install of the Elohim hApp.
//! Rust port of `elohim/holochain/edgenode/scripts/install-happ.cjs`.
//!
//! ## Lifecycle
//!
//! 1. Check if the app is already installed via `list_apps`
//! 2. If installed, verify all expected DNA roles are present and provisioned
//! 3. If stale (missing roles or empty cells), uninstall and reinstall
//! 4. If disabled, enable
//! 5. If not installed, do a fresh install
//! 6. Always ensure an app interface is attached on the expected port

use std::path::Path;

use holochain_client::{AdminWebsocket, AllowedOrigins, CellInfo, InstallAppPayload};
use holochain_types::app::{AppBundleSource, AppStatus};
use tracing::{info, warn};

/// Default installed app ID.
pub const APP_ID: &str = "elohim";

/// Port for the app WebSocket interface (zome calls).
pub const APP_INTERFACE_PORT: u16 = 4445;

/// DNA roles that a fully-provisioned Elohim hApp must contain.
pub const EXPECTED_ROLES: &[&str] = &[
    "lamad",
    "infrastructure",
    "imagodei",
    "mishpat",
    "node_registry",
];

/// Ensure the hApp is installed, up-to-date, enabled, and reachable.
///
/// Performs the full lifecycle check:
/// - Lists apps to find existing install
/// - Detects stale installs (missing roles / empty cells) and reinstalls
/// - Enables disabled apps
/// - Fresh-installs when no app exists
/// - Attaches an app interface on [`APP_INTERFACE_PORT`]
pub async fn ensure_happ_installed(
    admin_ws: &AdminWebsocket,
    happ_path: &Path,
    app_id: &str,
) -> anyhow::Result<()> {
    let apps = admin_ws
        .list_apps(None)
        .await
        .map_err(|e| anyhow::anyhow!("list_apps failed: {e}"))?;

    let existing = apps.iter().find(|a| a.installed_app_id == app_id);

    if let Some(app_info) = existing {
        info!(
            app_id = app_id,
            status = ?app_info.status,
            "App already installed"
        );

        if is_stale(app_info) {
            warn!(app_id = app_id, "Stale hApp detected — reinstalling");
            admin_ws
                .uninstall_app(app_id.to_string(), false)
                .await
                .map_err(|e| anyhow::anyhow!("uninstall_app failed: {e}"))?;
            info!(app_id = app_id, "Stale hApp removed");
            install_fresh(admin_ws, happ_path, app_id).await?;
        } else if matches!(app_info.status, AppStatus::Disabled(_)) {
            info!(app_id = app_id, "Enabling disabled app");
            admin_ws
                .enable_app(app_id.to_string())
                .await
                .map_err(|e| anyhow::anyhow!("enable_app failed: {e}"))?;
            info!(app_id = app_id, "App enabled");
        }
    } else {
        info!(app_id = app_id, "App not found — installing fresh");
        install_fresh(admin_ws, happ_path, app_id).await?;
    }

    ensure_app_interface(admin_ws).await?;
    Ok(())
}

/// Detect whether an installed hApp is stale.
///
/// A hApp is stale if:
/// - Any expected role is missing from `cell_info`
/// - Any role has zero provisioned cells
fn is_stale(app_info: &holochain_client::AppInfo) -> bool {
    let cell_info = &app_info.cell_info;

    for role in EXPECTED_ROLES {
        match cell_info.get(*role) {
            None => {
                warn!(role = role, "Stale: missing role");
                return true;
            }
            Some(cells) => {
                let provisioned = cells.iter().any(|c| matches!(c, CellInfo::Provisioned(_)));
                if !provisioned {
                    warn!(role = role, "Stale: role has no provisioned cells");
                    return true;
                }
            }
        }
    }

    false
}

/// Install the hApp from disk and enable it.
async fn install_fresh(
    admin_ws: &AdminWebsocket,
    happ_path: &Path,
    app_id: &str,
) -> anyhow::Result<()> {
    info!(
        app_id = app_id,
        path = %happ_path.display(),
        "Installing hApp"
    );

    let agent_key = admin_ws
        .generate_agent_pub_key()
        .await
        .map_err(|e| anyhow::anyhow!("generate_agent_pub_key failed: {e}"))?;

    let payload = InstallAppPayload {
        source: AppBundleSource::Path(happ_path.to_path_buf()),
        agent_key: Some(agent_key),
        installed_app_id: Some(app_id.to_string()),
        roles_settings: None,
        network_seed: None,
        ignore_genesis_failure: false,
    };

    admin_ws
        .install_app(payload)
        .await
        .map_err(|e| anyhow::anyhow!("install_app failed: {e}"))?;
    info!(app_id = app_id, "hApp installed");

    admin_ws
        .enable_app(app_id.to_string())
        .await
        .map_err(|e| anyhow::anyhow!("enable_app failed: {e}"))?;
    info!(app_id = app_id, "hApp enabled");

    Ok(())
}

/// Ensure an app WebSocket interface exists on [`APP_INTERFACE_PORT`].
///
/// Idempotent — if the interface already exists the conductor returns an error
/// which we treat as success.
async fn ensure_app_interface(admin_ws: &AdminWebsocket) -> anyhow::Result<()> {
    match admin_ws
        .attach_app_interface(APP_INTERFACE_PORT, None, AllowedOrigins::Any, None)
        .await
    {
        Ok(port) => {
            info!(port = port, "App interface attached");
        }
        Err(e) => {
            // Interface may already exist — that's fine
            warn!(
                port = APP_INTERFACE_PORT,
                error = %e,
                "attach_app_interface error (may already exist)"
            );
        }
    }
    Ok(())
}
