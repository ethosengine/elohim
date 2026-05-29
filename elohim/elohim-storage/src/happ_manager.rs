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

        // DNA-content drift: the conductor data dir is a persistent PVC, so an
        // already-installed app survives pod restarts AND storage resets (which
        // only wipe content.db, not /var/local/lib/holochain). When the bundled
        // DNA changes but role STRUCTURE stays the same (e.g. an integrity-zome
        // fix → new DNA hash, same roles), `is_stale` reads "not stale" and the
        // conductor keeps the OLD DNA forever — exactly how the Gap-F fix sat
        // built-but-undeployed. Drift detection forces a reinstall in that case.
        //
        // GATED behind ALLOW_DNA_REINSTALL: a reinstall mints a new agent key /
        // cell — fine for ephemeral re-seeded envs (alpha/dev), NOT the prod
        // upgrade path (which needs DNA migration/lineage). Prod leaves the flag
        // unset → no probe runs, no behavior change.
        let allow_reinstall = std::env::var("ALLOW_DNA_REINSTALL")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let drifted =
            allow_reinstall && has_dna_drift(admin_ws, app_info, happ_path, app_id).await;

        if is_stale(app_info) || drifted {
            if drifted {
                warn!(
                    app_id = app_id,
                    "DNA content drift vs bundle (ALLOW_DNA_REINSTALL=true) — reinstalling"
                );
            } else {
                warn!(app_id = app_id, "Stale hApp detected — reinstalling");
            }
            admin_ws
                .uninstall_app(app_id.to_string(), false)
                .await
                .map_err(|e| anyhow::anyhow!("uninstall_app failed: {e}"))?;
            info!(app_id = app_id, "Old hApp removed");
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

/// Map role → DNA hash (as a string) for the provisioned cell of each role in
/// an AppInfo. Only provisioned cells carry the role's canonical DNA hash.
fn provisioned_dna_hashes(
    app_info: &holochain_client::AppInfo,
) -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();
    for (role, cells) in &app_info.cell_info {
        for cell in cells {
            if let CellInfo::Provisioned(p) = cell {
                out.insert(role.clone(), p.cell_id.dna_hash().to_string());
                break;
            }
        }
    }
    out
}

/// Detect whether the installed hApp's DNA content has drifted from the bundle
/// on disk. Computes the bundle's per-role DNA hashes by installing it under a
/// throwaway app id (NOT enabled → no network join), reading the cell DNA
/// hashes, then uninstalling. A DNA hash is derived from DNA content + modifiers
/// and is agent-independent, so the probe's hashes equal what a fresh main
/// install of this bundle would produce.
///
/// Defensive: any failure (probe install/uninstall error) returns `false`
/// (treat as no-drift) so a probe problem never blocks startup — worst case is
/// the prior behavior (keep the installed hApp).
async fn has_dna_drift(
    admin_ws: &AdminWebsocket,
    app_info: &holochain_client::AppInfo,
    happ_path: &Path,
    app_id: &str,
) -> bool {
    let installed = provisioned_dna_hashes(app_info);
    let probe_id = format!("{app_id}-version-probe");

    // Clean any leftover probe from a prior interrupted run.
    if let Ok(apps) = admin_ws.list_apps(None).await {
        if apps.iter().any(|a| a.installed_app_id == probe_id) {
            let _ = admin_ws.uninstall_app(probe_id.clone(), true).await;
        }
    }

    let bundle = match probe_bundle_hashes(admin_ws, happ_path, &probe_id).await {
        Ok(h) => h,
        Err(e) => {
            warn!(error = %e, "DNA-drift probe failed (non-fatal) — keeping installed hApp");
            return false;
        }
    };

    for (role, bundle_hash) in &bundle {
        if let Some(installed_hash) = installed.get(role) {
            if installed_hash != bundle_hash {
                warn!(
                    role = role,
                    installed = installed_hash,
                    bundle = bundle_hash,
                    "DNA drift detected for role"
                );
                return true;
            }
        }
    }
    false
}

/// Install the bundle under a throwaway app id (without enabling), read its
/// per-role provisioned DNA hashes, then uninstall the probe.
async fn probe_bundle_hashes(
    admin_ws: &AdminWebsocket,
    happ_path: &Path,
    probe_id: &str,
) -> anyhow::Result<std::collections::BTreeMap<String, String>> {
    let agent_key = admin_ws
        .generate_agent_pub_key()
        .await
        .map_err(|e| anyhow::anyhow!("probe generate_agent_pub_key failed: {e}"))?;

    let payload = InstallAppPayload {
        source: AppBundleSource::Path(happ_path.to_path_buf()),
        agent_key: Some(agent_key),
        installed_app_id: Some(probe_id.to_string()),
        roles_settings: None,
        network_seed: None,
        ignore_genesis_failure: false,
    };

    // Intentionally NOT enabled — install registers the cells (giving DNA
    // hashes) without joining any network.
    let probe_info = admin_ws
        .install_app(payload)
        .await
        .map_err(|e| anyhow::anyhow!("probe install_app failed: {e}"))?;

    let hashes = provisioned_dna_hashes(&probe_info);

    if let Err(e) = admin_ws.uninstall_app(probe_id.to_string(), true).await {
        warn!(error = %e, "failed to clean up version-probe app (non-fatal)");
    }

    Ok(hashes)
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
