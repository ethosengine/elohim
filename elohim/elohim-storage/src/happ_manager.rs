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
use holochain_types::app::{AppBundle, AppBundleSource, AppStatus};
use tracing::{error, info, warn};

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
        let force_reinstall = std::env::var("FORCE_DNA_REINSTALL")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let allow_reinstall = std::env::var("ALLOW_DNA_REINSTALL")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        // FORCE_DNA_REINSTALL: unconditional reinstall, skipping the drift probe
        // entirely — the escape hatch for ephemeral envs (or when the probe is
        // suspect). ALLOW_DNA_REINSTALL: reinstall only when the bundle DNA
        // actually differs from the installed DNA (no churn). Reading drift from
        // the local bundle file (has_dna_drift) cannot time out on the conductor.
        let drifted = if force_reinstall {
            warn!(app_id = app_id, "FORCE_DNA_REINSTALL=true — reinstalling unconditionally (drift probe skipped)");
            true
        } else if allow_reinstall {
            has_dna_drift(app_info, happ_path).await
        } else {
            false
        };

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
/// on disk. Reads the bundle's per-role DNA hashes DIRECTLY from the `.happ`
/// file (`AppBundle::unpack` + `resolve_cells`, which applies the manifest's
/// baked modifiers exactly as install does) — NO admin-websocket round-trip.
///
/// This replaces the original sacrificial-`install_app` probe, whose admin-WS
/// call timed out against the busy embedded conductor at startup
/// (`probe install_app failed: Websocket error: Timeout`) → the defensive
/// no-drift fallback fired silently → the Gap-F DNA never auto-deployed on
/// alpha (DNA hashes stayed byte-for-byte identical cluster-wide). Reading the
/// local bundle file cannot time out on the conductor.
///
/// On read/decode error: logs ERROR (a real problem — DNA changes will NOT
/// auto-deploy until fixed) and returns `false` (no-drift) so startup never
/// blocks — worst case is the prior behavior (keep the installed hApp).
async fn has_dna_drift(app_info: &holochain_client::AppInfo, happ_path: &Path) -> bool {
    let installed = provisioned_dna_hashes(app_info);
    let bundle = match bundle_dna_hashes(happ_path).await {
        Ok(h) => h,
        Err(e) => {
            error!(
                error = %e,
                path = %happ_path.display(),
                "DNA-drift bundle read FAILED — keeping installed hApp; DNA changes will NOT \
                 auto-deploy until this is resolved (set FORCE_DNA_REINSTALL=true to bypass)"
            );
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

/// Read the bundle's per-role DNA hashes directly from the `.happ` file. Uses
/// the same resolution path install uses (`AppBundle::resolve_cells` with the
/// manifest's baked modifiers), so the hashes equal what a fresh install of
/// this bundle would produce — but entirely offline from the conductor, so it
/// cannot time out on the admin websocket.
async fn bundle_dna_hashes(
    happ_path: &Path,
) -> anyhow::Result<std::collections::BTreeMap<String, String>> {
    let bytes = std::fs::read(happ_path)
        .map_err(|e| anyhow::anyhow!("read happ {}: {e}", happ_path.display()))?;
    let bundle = AppBundle::unpack(std::io::Cursor::new(bytes))
        .map_err(|e| anyhow::anyhow!("unpack happ bundle: {e}"))?;

    // Empty memproof + existing-cells maps → resolve every role from the bundle.
    // All elohim DNAs are bundled (location: path, packed into the .happ), so
    // resolution reads from the bundle — no network / no conductor.
    let resolution = bundle
        .resolve_cells(Default::default(), Default::default())
        .await
        .map_err(|e| anyhow::anyhow!("resolve_cells: {e}"))?;

    let mut out = std::collections::BTreeMap::new();
    for (role_name, assignment) in &resolution.role_assignments {
        if let Some(primary) = assignment.as_primary() {
            out.insert(role_name.to_string(), primary.dna_hash().to_string());
        }
    }
    Ok(out)
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
