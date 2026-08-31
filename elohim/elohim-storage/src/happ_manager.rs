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
use holochain_types::app::{
    AppBundle, AppBundleSource, AppStatus, CoordinatorSource, UpdateCoordinatorsPayload,
};
use holochain_types::dna::{CoordinatorBundle, CoordinatorManifest, DnaFile, ZomeManifest};
use holochain_types::prelude::{DnaDef, ZomeDependency};
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

/// Parse a boolean deploy-gate env var the way every hApp-lifecycle gate in
/// this module parses one: present and case-insensitively `"true"` → on,
/// anything else (including unset) → off.
fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Whether coordinator-zome hot-swaps may be APPLIED (as opposed to merely
/// detected). `ALLOW_COORDINATOR_UPDATE` when set; otherwise it inherits
/// `ALLOW_DNA_REINSTALL`'s value — a node that already permits the heavier
/// reinstall path implicitly permits the strictly-safer hot-swap.
///
/// Shared by the boot path ([`ensure_happ_installed`]) and the node-local
/// `POST /admin/coordinators/sync` vehicle so the two can never drift.
pub fn coordinator_update_allowed() -> bool {
    std::env::var("ALLOW_COORDINATOR_UPDATE")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or_else(|_| env_flag("ALLOW_DNA_REINSTALL"))
}

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
        let force_reinstall = env_flag("FORCE_DNA_REINSTALL");
        let allow_reinstall = env_flag("ALLOW_DNA_REINSTALL");
        // FORCE_DNA_REINSTALL: unconditional reinstall, skipping the drift probe
        // entirely — the escape hatch for ephemeral envs (or when the probe is
        // suspect). ALLOW_DNA_REINSTALL: reinstall only when the bundle DNA
        // actually differs from the installed DNA (no churn). Reading drift from
        // the local bundle file (has_dna_drift) cannot time out on the conductor.
        let drifted = if force_reinstall {
            warn!(
                app_id = app_id,
                "FORCE_DNA_REINSTALL=true — reinstalling unconditionally (drift probe skipped)"
            );
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
        } else {
            // No integrity-level drift and not structurally stale. A
            // coordinator-ONLY change is still invisible here: the DNA hash
            // covers integrity zomes + modifiers only, so has_dna_drift reads
            // "no drift" while the conductor keeps serving OLD coordinator
            // wasm from its PVC (exactly how the 2f02879d attestation fix sat
            // built-but-undelivered). Coordinator drift is healed by the
            // conductor's update_coordinators hot-swap — agent key, cells, and
            // DHT state all preserved, so it is safe wherever a deploy is.
            let allow_coordinator_update = coordinator_update_allowed();
            match sync_coordinators(admin_ws, app_info, happ_path, allow_coordinator_update).await {
                Ok(0) => info!(app_id = app_id, "No coordinator-zome drift"),
                Ok(n) => info!(
                    app_id = app_id,
                    drifted_roles = n,
                    applied = allow_coordinator_update,
                    "Coordinator-zome drift handled"
                ),
                Err(e) => error!(
                    app_id = app_id,
                    error = %e,
                    "coordinator drift check FAILED — coordinator-only changes will NOT deploy until resolved"
                ),
            }

            if matches!(app_info.status, AppStatus::Disabled(_)) {
                info!(app_id = app_id, "Enabling disabled app");
                admin_ws
                    .enable_app(app_id.to_string())
                    .await
                    .map_err(|e| anyhow::anyhow!("enable_app failed: {e}"))?;
                info!(app_id = app_id, "App enabled");
            }
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

/// Map zome name → coordinator wasm hash (as a string) for a DnaDef.
///
/// This is the drift unit for coordinator-zome changes: the DNA hash covers
/// ONLY integrity zomes + modifiers, so a coordinator-only change (new
/// coordinator wasm, same integrity) produces a bundle whose DNA hashes are
/// byte-identical to the installed app — invisible to [`has_dna_drift`]. The
/// coordinator wasm hashes are where that change IS visible.
fn coordinator_wasm_hashes(dna_def: &DnaDef) -> std::collections::BTreeMap<String, String> {
    dna_def
        .coordinator_zomes
        .iter()
        .filter_map(|(name, zome_def)| {
            zome_def
                .wasm_hash(name)
                .ok()
                .map(|h| (name.to_string(), h.to_string()))
        })
        .collect()
}

/// Resolve the bundle's per-role [`DnaFile`]s directly from the `.happ` file —
/// same offline resolution path as [`bundle_dna_hashes`], but keeping the full
/// DnaFile (coordinator zome defs + wasm code) instead of just the DNA hash.
async fn bundle_role_dna_files(
    happ_path: &Path,
) -> anyhow::Result<std::collections::BTreeMap<String, DnaFile>> {
    let bytes = std::fs::read(happ_path)
        .map_err(|e| anyhow::anyhow!("read happ {}: {e}", happ_path.display()))?;
    let bundle = AppBundle::unpack(std::io::Cursor::new(bytes))
        .map_err(|e| anyhow::anyhow!("unpack happ bundle: {e}"))?;
    let resolution = bundle
        .resolve_cells(Default::default(), Default::default())
        .await
        .map_err(|e| anyhow::anyhow!("resolve_cells: {e}"))?;

    let by_hash: std::collections::HashMap<_, _> = resolution
        .dnas_to_register
        .into_iter()
        .map(|(dna_file, _proof)| (dna_file.dna_hash().clone(), dna_file))
        .collect();

    let mut out = std::collections::BTreeMap::new();
    for (role_name, assignment) in &resolution.role_assignments {
        if let Some(primary) = assignment.as_primary() {
            if let Some(dna_file) = by_hash.get(primary.dna_hash()) {
                out.insert(role_name.to_string(), dna_file.clone());
            }
        }
    }
    Ok(out)
}

/// Build a [`CoordinatorBundle`] from a DnaFile's coordinator zomes — the
/// payload shape `update_coordinators` consumes. Resources are the coordinator
/// wasm bytes already carried in the DnaFile's code map.
async fn coordinator_bundle_from_dna_file(dna_file: &DnaFile) -> anyhow::Result<CoordinatorBundle> {
    let mut zomes = Vec::new();
    let mut resources = Vec::new();
    for (name, zome_def) in &dna_file.dna_def().coordinator_zomes {
        let wasm_hash = zome_def
            .wasm_hash(name)
            .map_err(|e| anyhow::anyhow!("coordinator zome '{name}' has no wasm hash: {e}"))?;
        let wasm = dna_file
            .code()
            .get(&wasm_hash)
            .ok_or_else(|| anyhow::anyhow!("wasm for coordinator zome '{name}' not in bundle"))?;
        let dependencies = zome_def
            .as_any_zome_def()
            .dependencies()
            .iter()
            .map(|dep| ZomeDependency { name: dep.clone() })
            .collect::<Vec<_>>();
        let manifest_zome = ZomeManifest {
            name: name.clone(),
            hash: Some(wasm_hash.into()),
            path: format!("{name}.wasm"),
            dependencies: Some(dependencies),
        };
        resources.push((manifest_zome.resource_id(), wasm.code.to_vec().into()));
        zomes.push(manifest_zome);
    }
    let manifest = CoordinatorManifest { zomes };
    let bundle = mr_bundle::Bundle::new(manifest, resources)
        .map_err(|e| anyhow::anyhow!("build coordinator bundle: {e}"))?;
    Ok(bundle.into())
}

/// Per-role outcome of a coordinator-zome drift check / hot-swap.
///
/// `installed_coordinators` / `bundled_coordinators` map zome name → coordinator
/// wasm hash, rendered as the standard **base64 holo-hash string** (`uhCok…` —
/// `WasmHash::to_string()`), NOT hex. That is the drift unit for a
/// coordinator-only change, which the DNA hash cannot see (the DNA hash covers
/// integrity zomes + modifiers only).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoordinatorRoleReport {
    pub role: String,
    pub drifted: bool,
    pub applied: bool,
    /// zome name → coordinator wasm hash (base64 holo-hash, `uhCok…`).
    pub installed_coordinators: std::collections::BTreeMap<String, String>,
    /// zome name → coordinator wasm hash (base64 holo-hash, `uhCok…`).
    pub bundled_coordinators: std::collections::BTreeMap<String, String>,
    /// Set when this role could not be evaluated or its hot-swap failed. A
    /// per-role error NEVER aborts the sweep — the remaining roles still run.
    pub error: Option<String>,
}

/// Whole-app outcome of a coordinator-zome drift check / hot-swap.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoordinatorSyncReport {
    pub app_id: String,
    /// False = dry-run (detect only). True = drifted roles were hot-swapped.
    pub apply: bool,
    pub roles: Vec<CoordinatorRoleReport>,
    pub drifted_count: usize,
    pub applied_count: usize,
}

/// Build the per-role drift verdict from the two coordinator-wasm-hash maps.
/// Pure — the whole drift decision, isolated from the conductor and the bundle.
fn role_report(
    role: &str,
    installed: std::collections::BTreeMap<String, String>,
    bundled: std::collections::BTreeMap<String, String>,
) -> CoordinatorRoleReport {
    CoordinatorRoleReport {
        role: role.to_string(),
        drifted: installed != bundled,
        applied: false,
        installed_coordinators: installed,
        bundled_coordinators: bundled,
        error: None,
    }
}

/// Lineage guard. `update_coordinators` splices coordinator wasm into a LIVE
/// cell and matches integrity dependencies **by name only** — so a bundle from
/// a different DNA lineage (any integrity-zome or modifier change moves the DNA
/// hash) would install coordinators compiled against integrity zomes that are
/// not the ones running in that cell. That is a corruption vector, not an
/// upgrade: an integrity change needs the reinstall/migration path, never a
/// hot-swap.
///
/// The boot path holds this implicitly — `has_dna_drift` runs first and forces
/// a reinstall on any DNA-hash difference — so this is a no-op there. The HTTP
/// vehicle accepts an ARBITRARY posted bundle, so it needs the guard
/// explicitly. Returns `Some(error)` when the role must be refused.
fn lineage_mismatch_error(installed_dna_hash: &str, bundled_dna_hash: &str) -> Option<String> {
    if installed_dna_hash == bundled_dna_hash {
        return None;
    }
    Some(format!(
        "dnaHashMismatch: bundle carries a different DNA lineage (integrity change) — \
         hot-swap refused; this needs the reinstall/migration path \
         (installed={installed_dna_hash}, bundle={bundled_dna_hash})"
    ))
}

/// Detect and (when `apply`) heal coordinator-zome drift between the installed
/// cells and the bundle on disk via the conductor's `update_coordinators`
/// hot-swap — which preserves the agent key, the cell, and all DHT state
/// (unlike a reinstall, which mints a new key).
///
/// Returns the number of roles whose coordinators drifted. Per-role failures
/// are logged and skipped so one bad role can never block startup or the
/// remaining roles. Thin wrapper over [`sync_coordinators_for_app_info`] — the
/// boot path keeps its count-only contract while the HTTP vehicle reads the
/// full report from the same single implementation.
async fn sync_coordinators(
    admin_ws: &AdminWebsocket,
    app_info: &holochain_client::AppInfo,
    happ_path: &Path,
    apply: bool,
) -> anyhow::Result<usize> {
    sync_coordinators_for_app_info(admin_ws, app_info, happ_path, apply)
        .await
        .map(|report| report.drifted_count)
}

/// Report-returning entry point keyed by installed app id — the node-local
/// `POST /admin/coordinators/sync` vehicle (rung 1 of the upgrade-velocity
/// debt snowball). Resolves `app_info` the same way the boot path does
/// (`list_apps` → match on `installed_app_id`) and then runs the identical
/// per-role sweep.
pub async fn sync_coordinators_report(
    admin_ws: &AdminWebsocket,
    app_id: &str,
    happ_path: &Path,
    apply: bool,
) -> anyhow::Result<CoordinatorSyncReport> {
    let apps = admin_ws
        .list_apps(None)
        .await
        .map_err(|e| anyhow::anyhow!("list_apps failed: {e}"))?;
    let app_info = apps
        .iter()
        .find(|a| a.installed_app_id == app_id)
        .ok_or_else(|| anyhow::anyhow!("app '{app_id}' is not installed on this conductor"))?;
    sync_coordinators_for_app_info(admin_ws, app_info, happ_path, apply).await
}

/// The single coordinator-drift implementation. Per-role failures are recorded
/// on the role's report and logged — never propagated — so one bad role can
/// neither block startup nor hide the remaining roles.
async fn sync_coordinators_for_app_info(
    admin_ws: &AdminWebsocket,
    app_info: &holochain_client::AppInfo,
    happ_path: &Path,
    apply: bool,
) -> anyhow::Result<CoordinatorSyncReport> {
    let role_dnas = bundle_role_dna_files(happ_path).await?;
    let mut roles: Vec<CoordinatorRoleReport> = Vec::new();
    let mut drifted = 0usize;
    let mut applied_count = 0usize;

    for (role, cells) in &app_info.cell_info {
        let Some(dna_file) = role_dnas.get(role) else {
            continue;
        };
        let Some(cell_id) = cells.iter().find_map(|c| match c {
            CellInfo::Provisioned(p) => Some(p.cell_id.clone()),
            _ => None,
        }) else {
            continue;
        };

        let installed_def = match admin_ws.get_dna_definition(cell_id.clone()).await {
            Ok(def) => def,
            Err(e) => {
                error!(role = role.as_str(), error = %e, "get_dna_definition failed — skipping coordinator drift check for role");
                roles.push(CoordinatorRoleReport {
                    role: role.to_string(),
                    drifted: false,
                    applied: false,
                    installed_coordinators: Default::default(),
                    bundled_coordinators: coordinator_wasm_hashes(dna_file.dna_def()),
                    error: Some(format!("get_dna_definition failed: {e}")),
                });
                continue;
            }
        };

        let installed = coordinator_wasm_hashes(&installed_def);
        let bundled = coordinator_wasm_hashes(dna_file.dna_def());
        let mut report = role_report(role, installed, bundled);
        if report.drifted {
            drifted += 1;
        }

        // Lineage guard — REFUSE before any swap. Reported identically in
        // dry-run and apply mode so an operator sees the mismatch without
        // having to attempt the write. Per-role: other roles proceed.
        let installed_dna_hash = cell_id.dna_hash().to_string();
        let bundled_dna_hash = dna_file.dna_hash().to_string();
        if let Some(err) = lineage_mismatch_error(&installed_dna_hash, &bundled_dna_hash) {
            error!(
                role = role.as_str(),
                installed_dna = installed_dna_hash.as_str(),
                bundle_dna = bundled_dna_hash.as_str(),
                "DNA lineage mismatch — REFUSING coordinator hot-swap for role (integrity change needs reinstall/migration, not update_coordinators)"
            );
            report.error = Some(err);
            roles.push(report);
            continue;
        }

        if !report.drifted {
            roles.push(report);
            continue;
        }
        warn!(
            role = role.as_str(),
            installed = ?report.installed_coordinators,
            bundle = ?report.bundled_coordinators,
            "Coordinator-zome drift (DNA hash unchanged — integrity-only hashing cannot see this)"
        );

        if !apply {
            warn!(
                role = role.as_str(),
                "NOT hot-swapping — set ALLOW_COORDINATOR_UPDATE=true (or ALLOW_DNA_REINSTALL=true) to apply; \
                 the conductor keeps serving the OLDER coordinator wasm until then"
            );
            roles.push(report);
            continue;
        }

        let coordinator_bundle = match coordinator_bundle_from_dna_file(dna_file).await {
            Ok(b) => b,
            Err(e) => {
                error!(role = role.as_str(), error = %e, "failed to build coordinator bundle — skipping role");
                report.error = Some(format!("failed to build coordinator bundle: {e}"));
                roles.push(report);
                continue;
            }
        };
        match admin_ws
            .update_coordinators(UpdateCoordinatorsPayload {
                cell_id,
                source: CoordinatorSource::Bundle(Box::new(coordinator_bundle)),
            })
            .await
        {
            Ok(()) => {
                info!(
                    role = role.as_str(),
                    "Coordinator zomes hot-swapped to bundle version"
                );
                report.applied = true;
                applied_count += 1;
            }
            Err(e) => {
                error!(role = role.as_str(), error = %e, "update_coordinators failed — role keeps old coordinators");
                report.error = Some(format!("update_coordinators failed: {e}"));
            }
        }
        roles.push(report);
    }

    Ok(CoordinatorSyncReport {
        app_id: app_info.installed_app_id.clone(),
        apply,
        roles,
        drifted_count: drifted,
        applied_count,
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use holochain_types::prelude::*;

    /// Build a DnaFile with one integrity zome and one coordinator zome from
    /// raw (never-executed) wasm bytes — hashing is all these tests exercise.
    async fn test_dna_file(integrity_wasm: &[u8], coordinator_wasm: &[u8]) -> DnaFile {
        let integrity = DnaWasm::from(integrity_wasm.to_vec());
        let coordinator = DnaWasm::from(coordinator_wasm.to_vec());
        let integrity_hash = DnaWasmHashed::from_content(integrity.clone())
            .await
            .into_hash();
        let coordinator_hash = DnaWasmHashed::from_content(coordinator.clone())
            .await
            .into_hash();

        let integrity_name: ZomeName = "test_integrity".into();
        let coordinator_name: ZomeName = "test_coordinator".into();

        let dna_def = DnaDef {
            name: "test-dna".to_string(),
            modifiers: DnaModifiers {
                network_seed: "test-seed".to_string(),
                properties: SerializedBytes::default(),
            },
            integrity_zomes: vec![(
                integrity_name.clone(),
                ZomeDef::Wasm(WasmZome {
                    wasm_hash: integrity_hash,
                    dependencies: vec![],
                })
                .into(),
            )],
            coordinator_zomes: vec![(
                coordinator_name,
                ZomeDef::Wasm(WasmZome {
                    wasm_hash: coordinator_hash,
                    dependencies: vec![integrity_name],
                })
                .into(),
            )],
        };

        DnaFile::new(dna_def, vec![integrity, coordinator]).await
    }

    #[tokio::test]
    async fn coordinator_drift_is_visible_when_dna_hash_is_not() {
        // Same integrity wasm (→ identical DNA hash), different coordinator
        // wasm — the exact shape of the 2f02879d attestation fix that the
        // DNA-hash drift check was blind to.
        let installed = test_dna_file(b"integrity-v1", b"coordinator-v1").await;
        let bundle = test_dna_file(b"integrity-v1", b"coordinator-v2").await;

        assert_eq!(
            installed.dna_hash(),
            bundle.dna_hash(),
            "precondition: coordinator-only change must NOT move the DNA hash"
        );
        assert_ne!(
            coordinator_wasm_hashes(installed.dna_def()),
            coordinator_wasm_hashes(bundle.dna_def()),
            "coordinator wasm hashes must expose the drift the DNA hash hides"
        );
    }

    #[tokio::test]
    async fn coordinator_wasm_hashes_equal_for_identical_bundles() {
        let a = test_dna_file(b"integrity-v1", b"coordinator-v1").await;
        let b = test_dna_file(b"integrity-v1", b"coordinator-v1").await;
        assert_eq!(
            coordinator_wasm_hashes(a.dna_def()),
            coordinator_wasm_hashes(b.dna_def())
        );
    }

    /// The drift verdict itself, isolated from the conductor and the bundle:
    /// equal coordinator-hash maps → clean, any difference → drifted.
    #[test]
    fn role_report_flags_drift_only_on_hash_difference() {
        let a: std::collections::BTreeMap<String, String> =
            [("z".to_string(), "uhCok-v1".to_string())]
                .into_iter()
                .collect();
        let b: std::collections::BTreeMap<String, String> =
            [("z".to_string(), "uhCok-v2".to_string())]
                .into_iter()
                .collect();

        let clean = role_report("lamad", a.clone(), a.clone());
        assert!(!clean.drifted);
        assert!(!clean.applied, "a fresh report is never pre-marked applied");
        assert!(clean.error.is_none());

        let dirty = role_report("lamad", a, b);
        assert!(dirty.drifted);
        assert_eq!(dirty.role, "lamad");
    }

    /// A role missing a zome the other side has is drift too (added/removed
    /// coordinator zome, not just a changed one).
    #[test]
    fn role_report_flags_added_or_removed_zome_as_drift() {
        let one: std::collections::BTreeMap<String, String> =
            [("z".to_string(), "uhCok-v1".to_string())]
                .into_iter()
                .collect();
        let two: std::collections::BTreeMap<String, String> = [
            ("z".to_string(), "uhCok-v1".to_string()),
            ("z2".to_string(), "uhCok-new".to_string()),
        ]
        .into_iter()
        .collect();
        assert!(role_report("imagodei", one, two).drifted);
    }

    /// The lineage guard: identical DNA hash → hot-swap permitted; any
    /// difference → refused. `update_coordinators` matches integrity
    /// dependencies BY NAME, so a cross-lineage bundle would splice
    /// coordinators onto integrity zomes they were never compiled against.
    #[test]
    fn lineage_guard_permits_same_dna_and_refuses_different() {
        assert!(lineage_mismatch_error("uhC0k-abc", "uhC0k-abc").is_none());

        let err = lineage_mismatch_error("uhC0k-installed", "uhC0k-other")
            .expect("a different DNA lineage must be refused");
        assert!(err.starts_with("dnaHashMismatch:"), "err was: {err}");
        assert!(err.contains("uhC0k-installed"));
        assert!(err.contains("uhC0k-other"));
        assert!(
            err.contains("reinstall"),
            "the refusal must name the path that IS correct for an integrity change"
        );
    }

    /// A refused role reports the mismatch and MUST NOT be marked applied —
    /// the shape the HTTP vehicle serves in both dry-run and apply mode.
    #[test]
    fn lineage_mismatch_role_report_is_never_applied() {
        let installed: std::collections::BTreeMap<String, String> =
            [("content_store".to_string(), "uhCok-old".to_string())]
                .into_iter()
                .collect();
        let bundled: std::collections::BTreeMap<String, String> =
            [("content_store".to_string(), "uhCok-new".to_string())]
                .into_iter()
                .collect();
        let mut role = role_report("lamad", installed, bundled);
        role.error = lineage_mismatch_error("uhC0k-installed", "uhC0k-other");

        assert!(role.drifted, "coordinator hashes really do differ");
        assert!(!role.applied, "a refused role is never applied");

        let v = serde_json::to_value(&role).expect("role serializes");
        assert_eq!(v["applied"], false);
        assert!(v["error"]
            .as_str()
            .expect("error is a string")
            .starts_with("dnaHashMismatch:"));
    }

    /// snake_case never leaves the Rust boundary — the wire shape is camelCase.
    #[test]
    fn coordinator_sync_report_serializes_camel_case() {
        let installed: std::collections::BTreeMap<String, String> =
            [("content_store".to_string(), "uhCok-old".to_string())]
                .into_iter()
                .collect();
        let bundled: std::collections::BTreeMap<String, String> =
            [("content_store".to_string(), "uhCok-new".to_string())]
                .into_iter()
                .collect();
        let mut role = role_report("lamad", installed, bundled);
        role.applied = true;

        let report = CoordinatorSyncReport {
            app_id: "elohim".to_string(),
            apply: true,
            roles: vec![role],
            drifted_count: 1,
            applied_count: 1,
        };
        let v = serde_json::to_value(&report).expect("report serializes");

        assert_eq!(v["appId"], "elohim");
        assert_eq!(v["apply"], true);
        assert_eq!(v["driftedCount"], 1);
        assert_eq!(v["appliedCount"], 1);
        assert!(
            v.get("app_id").is_none(),
            "snake_case must not reach the wire"
        );
        assert!(v.get("drifted_count").is_none());

        let r = &v["roles"][0];
        assert_eq!(r["role"], "lamad");
        assert_eq!(r["drifted"], true);
        assert_eq!(r["applied"], true);
        assert_eq!(r["installedCoordinators"]["content_store"], "uhCok-old");
        assert_eq!(r["bundledCoordinators"]["content_store"], "uhCok-new");
        assert!(r["error"].is_null());
        assert!(r.get("installed_coordinators").is_none());
        assert!(r.get("bundled_coordinators").is_none());
    }

    #[tokio::test]
    async fn coordinator_bundle_round_trips_with_matching_hashes() {
        let dna_file = test_dna_file(b"integrity-v1", b"coordinator-v7").await;
        let expected = coordinator_wasm_hashes(dna_file.dna_def());

        let bundle = coordinator_bundle_from_dna_file(&dna_file)
            .await
            .expect("coordinator bundle builds from DnaFile");

        // into_zomes re-hashes the carried wasm bytes — the round-tripped
        // hashes must equal the DnaFile's, proving the hot-swap payload
        // delivers exactly the bundle's coordinators.
        let (zomes, wasms) = bundle.into_zomes().await.expect("bundle resolves to zomes");
        assert_eq!(wasms.len(), 1);
        let round_tripped: std::collections::BTreeMap<String, String> = zomes
            .iter()
            .filter_map(|(name, zome_def)| {
                zome_def
                    .wasm_hash(name)
                    .ok()
                    .map(|h| (name.to_string(), h.to_string()))
            })
            .collect();
        assert_eq!(round_tripped, expected);

        // Dependencies must survive — update_coordinators rejects coordinators
        // whose integrity dependency is missing.
        let (_, zome_def) = &zomes[0];
        assert_eq!(
            zome_def.as_any_zome_def().dependencies(),
            &[ZomeName::from("test_integrity")]
        );
    }
}
