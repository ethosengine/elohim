//! Node-local runtime passport served by `GET /version`.
//!
//! This is a Category-C operational observation: every response is rebuilt
//! from the running process, its conductor connection, and the local host. It
//! is never persisted, notarized, or advertised over either P2P transport.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use holochain_client::{AdminWebsocket, CellInfo};
use serde::Serialize;

use crate::lineage_roles::RoleLineage;

const HAPP_INVENTORY_BUDGET: Duration = Duration::from_secs(5);

/// Additive `/version` response.
///
/// `BuildInfo` remains flattened at the top level so every legacy field keeps
/// its original key and value. The richer document lives under `passport`
/// because legacy `BuildInfo.service` already owns the top-level `service` key.
#[derive(Debug, Clone, Serialize)]
pub struct RuntimeVersionResponse {
    #[serde(flatten)]
    pub build: elohim_compute::BuildInfo,
    pub passport: RuntimePassport,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePassport {
    pub service: ServicePassport,
    pub conductor: ConductorPassport,
    pub happ: HappPassport,
    pub host: HostPassport,
    pub flags: BootFlagsPassport,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServicePassport {
    #[serde(flatten)]
    pub build: elohim_compute::BuildInfo,
    pub compiled_features: CompiledFeatures,
    pub active_transports: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledFeatures {
    pub p2p: bool,
    #[serde(rename = "p2p-iroh")]
    pub p2p_iroh: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConductorPassport {
    pub mode: String,
    pub version: String,
    pub version_source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HappPassport {
    pub app_id: String,
    pub roles: Vec<HappRolePassport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Every installed `<app_id>@…` lineage side-app under this peer's
    /// agent key (Task 8), sorted. Empty (and so omitted) on a peer that
    /// has never opened a lineage window — the pre-Task-8 shape.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lineage_apps: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HappRolePassport {
    pub role: String,
    pub dna_hash: String,
    pub coordinator_wasm_hashes: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Dual-cell view (Task 8): present only while a lineage window is open
    /// on this role, or after it has been sunset. `None` — and so omitted —
    /// on the default single-cell shape every role starts and normally
    /// stays in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lineage: Option<RoleLineageView>,
}

/// The dual-cell state of one role: which app id currently serves reads,
/// which authors, each cell's DNA hash, and whether the window has been
/// permanently closed (sunset). Projected from `LineageRoles::snapshot()`
/// by [`lineage_view_for`] — never held live, always a point-in-time copy.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleLineageView {
    pub reading_app_id: String,
    pub authoring_app_id: String,
    pub reading_dna_hash: String,
    pub authoring_dna_hash: String,
    pub closed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostPassport {
    pub kernel: String,
    pub release: String,
    pub os: String,
    pub arch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootFlagsPassport {
    pub allow_coordinator_update: bool,
    pub obey_carried_election: bool,
    pub adopt_before_author: bool,
    pub transport_backend: String,
    pub adaptive_transport_selection: bool,
}

/// Inputs already held by `HttpServer`; keeping them here avoids adding a
/// second configuration home or touching the composition root.
pub struct StoragePassportContext {
    pub embedded_conductor: bool,
    pub external_conductor_configured: bool,
    pub admin_websocket: Option<AdminWebsocket>,
    pub app_id: String,
    pub libp2p_active: bool,
    pub iroh_active: bool,
    /// Point-in-time copy of every tracked role's lineage state
    /// (`LineageRoles::snapshot()`, Task 8). The passport never holds the
    /// `Arc<LineageRoles>` itself — only this snapshot, taken fresh on
    /// every `/version` call. Empty on a node with no lineage window ever
    /// opened, which is byte-identical to the pre-Task-8 response.
    pub lineage: BTreeMap<String, RoleLineage>,
}

/// Assemble a fresh storage runtime passport.
///
/// Conductor inventory is best-effort and bounded because `/version` is a
/// deployment probe. Missing or slow admin state is rendered honestly inside
/// `happ.error`; it never hides the service and host facts or changes HTTP 200.
pub async fn assemble_storage_passport(ctx: StoragePassportContext) -> RuntimeVersionResponse {
    let build = elohim_compute::BuildInfo::new("elohim-storage");
    let active_transports = active_transports(ctx.libp2p_active, ctx.iroh_active);
    let transport_backend = transport_backend_name(ctx.libp2p_active, ctx.iroh_active).to_string();
    let conductor_mode = if ctx.embedded_conductor {
        "embedded"
    } else if ctx.external_conductor_configured {
        "external"
    } else {
        "none"
    };
    let (conductor_version, version_source) = conductor_version();

    let happ = match ctx.admin_websocket.as_ref() {
        Some(admin) => match tokio::time::timeout(
            HAPP_INVENTORY_BUDGET,
            inspect_installed_happ(admin, &ctx.app_id, &ctx.lineage),
        )
        .await
        {
            Ok(happ) => happ,
            Err(_) => HappPassport {
                app_id: ctx.app_id.clone(),
                roles: Vec::new(),
                error: Some(format!(
                    "conductor inventory timed out after {}s",
                    HAPP_INVENTORY_BUDGET.as_secs()
                )),
                lineage_apps: Vec::new(),
            },
        },
        None => HappPassport {
            app_id: ctx.app_id.clone(),
            roles: Vec::new(),
            error: Some("conductor admin connection unavailable".to_string()),
            lineage_apps: Vec::new(),
        },
    };

    RuntimeVersionResponse {
        build: build.clone(),
        passport: RuntimePassport {
            service: ServicePassport {
                build,
                compiled_features: CompiledFeatures {
                    p2p: cfg!(feature = "p2p"),
                    p2p_iroh: cfg!(feature = "p2p-iroh"),
                },
                active_transports,
            },
            conductor: ConductorPassport {
                mode: conductor_mode.to_string(),
                version: conductor_version,
                version_source,
            },
            happ,
            host: host_passport(),
            flags: BootFlagsPassport {
                allow_coordinator_update: crate::happ_manager::coordinator_update_allowed(),
                obey_carried_election: crate::config::obey_carried_election_enabled(),
                adopt_before_author: crate::config::adopt_before_author_enabled(),
                transport_backend,
                adaptive_transport_selection: adaptive_transport_selection_enabled(),
            },
        },
    }
}

fn active_transports(libp2p_active: bool, iroh_active: bool) -> Vec<String> {
    let mut transports = Vec::with_capacity(2);
    if libp2p_active {
        transports.push("libp2p".to_string());
    }
    if iroh_active {
        transports.push("iroh".to_string());
    }
    transports
}

fn transport_backend_name(libp2p_active: bool, iroh_active: bool) -> &'static str {
    match (libp2p_active, iroh_active) {
        (true, true) => "dual",
        (true, false) => "libp2p",
        (false, true) => "iroh",
        (false, false) => "none",
    }
}

fn conductor_version() -> (String, String) {
    // The pinned Holochain admin API has no version request. The image tag is
    // the runtime-visible build identity once the companion Dockerfile task
    // persists it; local/older images degrade explicitly to unknown.
    match std::env::var("CONDUCTOR_IMAGE_TAG") {
        Ok(tag) if !tag.trim().is_empty() => (tag, "CONDUCTOR_IMAGE_TAG".to_string()),
        _ => ("unknown".to_string(), "unknown".to_string()),
    }
}

fn adaptive_transport_selection_enabled() -> bool {
    #[cfg(feature = "p2p")]
    {
        crate::p2p::transport_paths::selection_enabled()
    }
    #[cfg(not(feature = "p2p"))]
    {
        false
    }
}

async fn inspect_installed_happ(
    admin: &AdminWebsocket,
    app_id: &str,
    lineage: &BTreeMap<String, RoleLineage>,
) -> HappPassport {
    let apps = match admin.list_apps(None).await {
        Ok(apps) => apps,
        Err(error) => {
            return HappPassport {
                app_id: app_id.to_string(),
                roles: Vec::new(),
                error: Some(format!("list_apps failed: {error}")),
                lineage_apps: Vec::new(),
            };
        }
    };
    let installed_app_ids: Vec<String> = apps
        .iter()
        .map(|app| app.installed_app_id.clone())
        .collect();
    let lineage_apps = lineage_apps_for(app_id, &installed_app_ids);

    let Some(app) = apps.iter().find(|app| app.installed_app_id == app_id) else {
        return HappPassport {
            app_id: app_id.to_string(),
            roles: Vec::new(),
            error: Some(format!("app '{app_id}' is not installed")),
            lineage_apps,
        };
    };

    let mut roles = Vec::new();
    for (role, cells) in &app.cell_info {
        let Some(cell_id) = cells.iter().find_map(|cell| match cell {
            CellInfo::Provisioned(provisioned) => Some(provisioned.cell_id.clone()),
            _ => None,
        }) else {
            roles.push(HappRolePassport {
                role: role.to_string(),
                dna_hash: "unknown".to_string(),
                coordinator_wasm_hashes: BTreeMap::new(),
                error: Some("role has no provisioned cell".to_string()),
                lineage: None,
            });
            continue;
        };

        let dna_hash = cell_id.dna_hash().to_string();
        let (coordinator_wasm_hashes, mut role_error) =
            match admin.get_dna_definition(cell_id).await {
                Ok(definition) => (
                    // Mirrors happ_manager's coordinator drift readback. The
                    // per-zome extractor is shared (`happ_manager::coordinator_wasm_hash`)
                    // so the 0.7 `ZomeDef::Wasm` field access lives in exactly one
                    // place; the passport still owns its own projection shape.
                    definition
                        .coordinator_zomes
                        .iter()
                        .filter_map(|(name, zome)| {
                            crate::happ_manager::coordinator_wasm_hash(zome)
                                .map(|hash| (name.to_string(), hash.to_string()))
                        })
                        .collect(),
                    None,
                ),
                Err(error) => (
                    BTreeMap::new(),
                    Some(format!("get_dna_definition failed: {error}")),
                ),
            };

        // Task 8: a lineage view is only worth an extra lookup when the
        // snapshot actually says this role is dual-celled (an open window
        // or a sunset) — `lineage_view_for` itself would return `None`
        // otherwise, so skip the app-inventory scan in the common
        // single-cell case.
        let needs_lineage_view = lineage
            .get(role)
            .map(|entry| entry.authoring_app_id != entry.reading_app_id || entry.closed)
            .unwrap_or(false);
        let authoring_dna_hash = if needs_lineage_view {
            let authoring_app_id = &lineage[role].authoring_app_id;
            match resolve_role_dna_hash(&apps, authoring_app_id, role) {
                Ok(hash) => Some(hash),
                Err(lookup_error) => {
                    if role_error.is_none() {
                        role_error = Some(lookup_error);
                    }
                    Some("unknown".to_string())
                }
            }
        } else {
            None
        };
        let lineage_view = lineage_view_for(role, &dna_hash, lineage, authoring_dna_hash);

        roles.push(HappRolePassport {
            role: role.to_string(),
            dna_hash,
            coordinator_wasm_hashes,
            error: role_error,
            lineage: lineage_view,
        });
    }

    HappPassport {
        app_id: app_id.to_string(),
        roles,
        error: None,
        lineage_apps,
    }
}

/// Every installed app id that is a lineage side-app of `base_app_id`
/// (`"<base_app_id>@…"`), sorted. Pure projection over the app-id list
/// `list_apps` already returned — no conductor call of its own.
fn lineage_apps_for(base_app_id: &str, installed_app_ids: &[String]) -> Vec<String> {
    let prefix = format!("{base_app_id}@");
    let mut apps: Vec<String> = installed_app_ids
        .iter()
        .filter(|id| id.starts_with(&prefix))
        .cloned()
        .collect();
    apps.sort();
    apps
}

/// The DNA hash of `role`'s provisioned cell inside `app_id`, read from an
/// already-fetched `list_apps` result — no extra admin round trip. Used to
/// resolve the AUTHORING cell's DNA hash for a dual-celled role; the
/// READING cell's hash is already computed by the caller from the base
/// app's own cell inventory.
fn resolve_role_dna_hash(
    apps: &[holochain_client::AppInfo],
    app_id: &str,
    role: &str,
) -> Result<String, String> {
    let app = apps
        .iter()
        .find(|app| app.installed_app_id == app_id)
        .ok_or_else(|| format!("authoring app '{app_id}' is not installed"))?;
    let cells = app
        .cell_info
        .get(role)
        .ok_or_else(|| format!("authoring app '{app_id}' has no role '{role}'"))?;
    cells
        .iter()
        .find_map(|cell| match cell {
            CellInfo::Provisioned(provisioned) => Some(provisioned.cell_id.dna_hash().to_string()),
            _ => None,
        })
        .ok_or_else(|| format!("authoring app '{app_id}' role '{role}' has no provisioned cell"))
}

/// Project one role's dual-cell state from the lineage snapshot into the
/// passport's wire shape. `dna_hash` is the READING cell's DNA hash the
/// caller already computed from the base app's cell inventory;
/// `authoring_dna_hash` is a best-effort lookup the caller performs only
/// when this function's own presence condition would otherwise return
/// `Some` — `"unknown"` when that lookup failed.
///
/// `None` in two cases: the role has no snapshot entry at all, or it does
/// but is still in the untouched single-cell state
/// (`authoring_app_id == reading_app_id && !closed`) — the shape every role
/// starts in and the shape a byte-identical-with-empty-snapshot response
/// depends on.
fn lineage_view_for(
    role: &str,
    dna_hash: &str,
    snapshot: &BTreeMap<String, RoleLineage>,
    authoring_dna_hash: Option<String>,
) -> Option<RoleLineageView> {
    let entry = snapshot.get(role)?;
    if entry.authoring_app_id == entry.reading_app_id && !entry.closed {
        return None;
    }
    Some(RoleLineageView {
        reading_app_id: entry.reading_app_id.clone(),
        authoring_app_id: entry.authoring_app_id.clone(),
        reading_dna_hash: dna_hash.to_string(),
        authoring_dna_hash: authoring_dna_hash.unwrap_or_else(|| "unknown".to_string()),
        closed: entry.closed,
    })
}

pub fn host_passport() -> HostPassport {
    HostPassport {
        kernel: read_trimmed("/proc/sys/kernel/ostype")
            .unwrap_or_else(|| std::env::consts::OS.to_string()),
        release: read_trimmed("/proc/sys/kernel/osrelease")
            .unwrap_or_else(|| "unknown".to_string()),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        container_hint: container_hint(),
    }
}

fn read_trimmed(path: impl AsRef<Path>) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn container_hint() -> Option<String> {
    if std::env::var_os("KUBERNETES_SERVICE_HOST").is_some() {
        return Some("kubernetes".to_string());
    }
    if Path::new("/.dockerenv").exists() {
        return Some("docker".to_string());
    }
    if Path::new("/run/.containerenv").exists() {
        return Some("container".to_string());
    }
    let cgroup = std::fs::read_to_string("/proc/1/cgroup").ok()?;
    for (needle, hint) in [
        ("kubepods", "kubernetes"),
        ("containerd", "containerd"),
        ("docker", "docker"),
        ("podman", "podman"),
    ] {
        if cgroup.contains(needle) {
            return Some(hint.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn runtime_passport_keeps_every_legacy_field_at_the_top_level() {
        let expected = elohim_compute::BuildInfo::new("elohim-storage");
        let expected_json = serde_json::to_value(&expected).unwrap();
        let response = assemble_storage_passport(StoragePassportContext {
            embedded_conductor: false,
            external_conductor_configured: false,
            admin_websocket: None,
            app_id: "elohim".to_string(),
            libp2p_active: false,
            iroh_active: false,
            lineage: BTreeMap::new(),
        })
        .await;
        let actual = serde_json::to_value(response).unwrap();

        for key in [
            "version",
            "commit",
            "commitFull",
            "buildTime",
            "rustcVersion",
            "service",
        ] {
            assert_eq!(actual[key], expected_json[key], "legacy field {key}");
        }
        assert_eq!(actual["service"], serde_json::json!("elohim-storage"));
        assert!(actual["passport"]["service"].is_object());
    }

    #[tokio::test]
    async fn runtime_passport_serializes_new_fields_in_camel_case() {
        let response = assemble_storage_passport(StoragePassportContext {
            embedded_conductor: true,
            external_conductor_configured: false,
            admin_websocket: None,
            app_id: "elohim".to_string(),
            libp2p_active: true,
            iroh_active: true,
            lineage: BTreeMap::new(),
        })
        .await;
        let json = serde_json::to_value(response).unwrap();
        let passport = &json["passport"];

        assert!(passport["service"].get("compiledFeatures").is_some());
        assert!(passport["service"].get("activeTransports").is_some());
        assert!(passport["conductor"].get("versionSource").is_some());
        assert!(passport["happ"].get("appId").is_some());
        assert!(passport["host"].get("arch").is_some());
        assert!(passport["host"].get("container_hint").is_none());
        assert!(passport["flags"].get("allowCoordinatorUpdate").is_some());
        assert!(passport["flags"].get("obeyCarriedElection").is_some());
        assert!(passport["flags"].get("adoptBeforeAuthor").is_some());
        assert!(passport["flags"].get("transportBackend").is_some());
        assert_eq!(passport["flags"]["transportBackend"], "dual");
        assert_eq!(passport["conductor"]["mode"], "embedded");
    }

    #[test]
    fn happ_role_uses_coordinator_wasm_hashes_camel_case_key() {
        let role = HappRolePassport {
            role: "content".to_string(),
            dna_hash: "uhC0k-example".to_string(),
            coordinator_wasm_hashes: BTreeMap::from([(
                "content_store".to_string(),
                "uhCok-example".to_string(),
            )]),
            error: None,
            lineage: None,
        };
        let json = serde_json::to_value(role).unwrap();
        assert!(json.get("dnaHash").is_some());
        assert!(json.get("coordinatorWasmHashes").is_some());
        assert!(json.get("dna_hash").is_none());
        assert!(json.get("lineage").is_none());
    }

    /// **Task 8.** An empty snapshot (the default, and every node until a
    /// lineage window is ever opened) serializes with NO `lineage` key on
    /// any role and NO `lineageApps` key on `happ` — byte-identical to the
    /// pre-Task-8 shape.
    #[test]
    fn happ_passport_with_empty_lineage_snapshot_carries_no_lineage_keys() {
        let happ = HappPassport {
            app_id: "elohim".to_string(),
            roles: vec![HappRolePassport {
                role: "node_registry".to_string(),
                dna_hash: "uhC0k-example".to_string(),
                coordinator_wasm_hashes: BTreeMap::new(),
                error: None,
                lineage: lineage_view_for("node_registry", "uhC0k-example", &BTreeMap::new(), None),
            }],
            error: None,
            lineage_apps: lineage_apps_for("elohim", &[]),
        };
        let json = serde_json::to_value(happ).unwrap();
        assert!(json.get("lineageApps").is_none());
        assert!(json["roles"][0].get("lineage").is_none());
    }

    /// `lineage_view_for`'s three cases: no snapshot entry, an open window,
    /// and a sunset that left the ids equal (the OR-condition edge the
    /// presence rule names explicitly).
    #[test]
    fn lineage_view_for_no_entry_returns_none() {
        let snapshot: BTreeMap<String, RoleLineage> = BTreeMap::new();
        assert!(lineage_view_for("node_registry", "uhC0k-base", &snapshot, None).is_none());
    }

    #[test]
    fn lineage_view_for_open_window_returns_some() {
        let mut snapshot = BTreeMap::new();
        snapshot.insert(
            "node_registry".to_string(),
            RoleLineage {
                reading_app_id: "elohim".to_string(),
                authoring_app_id: "elohim@EKiIscIk5BDd".to_string(),
                closed: false,
            },
        );
        let view = lineage_view_for(
            "node_registry",
            "uhC0k-base",
            &snapshot,
            Some("uhC0k-lineage".to_string()),
        )
        .expect("open window projects a view");
        assert_eq!(view.reading_app_id, "elohim");
        assert_eq!(view.authoring_app_id, "elohim@EKiIscIk5BDd");
        assert_eq!(view.reading_dna_hash, "uhC0k-base");
        assert_eq!(view.authoring_dna_hash, "uhC0k-lineage");
        assert!(!view.closed);
    }

    #[test]
    fn lineage_view_for_sunset_with_equal_ids_returns_some_closed() {
        // Mirrors the `closed` half of the OR condition: even when a caller
        // has (e.g. via `reset_all` after a hand-rolled sunset) left
        // authoring_app_id == reading_app_id, `closed: true` alone still
        // surfaces the view rather than silently reverting to the
        // untouched-role shape.
        let mut snapshot = BTreeMap::new();
        snapshot.insert(
            "node_registry".to_string(),
            RoleLineage {
                reading_app_id: "elohim".to_string(),
                authoring_app_id: "elohim".to_string(),
                closed: true,
            },
        );
        let view = lineage_view_for("node_registry", "uhC0k-base", &snapshot, None)
            .expect("closed=true alone still projects a view");
        assert_eq!(view.reading_app_id, "elohim");
        assert_eq!(view.authoring_app_id, "elohim");
        assert_eq!(view.authoring_dna_hash, "unknown");
        assert!(view.closed);
    }

    #[test]
    fn lineage_view_for_untouched_role_returns_none() {
        // The state every role starts in and stays in with no window ever
        // opened: same ids, not closed.
        let mut snapshot = BTreeMap::new();
        snapshot.insert(
            "node_registry".to_string(),
            RoleLineage {
                reading_app_id: "elohim".to_string(),
                authoring_app_id: "elohim".to_string(),
                closed: false,
            },
        );
        assert!(lineage_view_for("node_registry", "uhC0k-base", &snapshot, None).is_none());
    }

    #[test]
    fn lineage_apps_for_filters_and_sorts_by_base_app_prefix() {
        let installed = vec![
            "elohim".to_string(),
            "elohim@EKiIscIk5BDd".to_string(),
            "elohim-not-a-lineage-app".to_string(),
            "elohim@AAAA".to_string(),
            "other-app".to_string(),
        ];
        assert_eq!(
            lineage_apps_for("elohim", &installed),
            vec!["elohim@AAAA".to_string(), "elohim@EKiIscIk5BDd".to_string()]
        );
    }

    #[test]
    fn lineage_apps_for_empty_list_returns_empty() {
        assert!(lineage_apps_for("elohim", &[]).is_empty());
    }
}
