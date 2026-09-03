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
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HappRolePassport {
    pub role: String,
    pub dna_hash: String,
    pub coordinator_wasm_hashes: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
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
            inspect_installed_happ(admin, &ctx.app_id),
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
            },
        },
        None => HappPassport {
            app_id: ctx.app_id.clone(),
            roles: Vec::new(),
            error: Some("conductor admin connection unavailable".to_string()),
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

async fn inspect_installed_happ(admin: &AdminWebsocket, app_id: &str) -> HappPassport {
    let apps = match admin.list_apps(None).await {
        Ok(apps) => apps,
        Err(error) => {
            return HappPassport {
                app_id: app_id.to_string(),
                roles: Vec::new(),
                error: Some(format!("list_apps failed: {error}")),
            };
        }
    };
    let Some(app) = apps.iter().find(|app| app.installed_app_id == app_id) else {
        return HappPassport {
            app_id: app_id.to_string(),
            roles: Vec::new(),
            error: Some(format!("app '{app_id}' is not installed")),
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
            });
            continue;
        };

        let dna_hash = cell_id.dna_hash().to_string();
        match admin.get_dna_definition(cell_id).await {
            Ok(definition) => roles.push(HappRolePassport {
                role: role.to_string(),
                dna_hash,
                // Mirrors happ_manager's coordinator drift readback. The
                // per-zome extractor is shared (`happ_manager::coordinator_wasm_hash`)
                // so the 0.7 `ZomeDef::Wasm` field access lives in exactly one
                // place; the passport still owns its own projection shape.
                coordinator_wasm_hashes: definition
                    .coordinator_zomes
                    .iter()
                    .filter_map(|(name, zome)| {
                        crate::happ_manager::coordinator_wasm_hash(zome)
                            .map(|hash| (name.to_string(), hash.to_string()))
                    })
                    .collect(),
                error: None,
            }),
            Err(error) => roles.push(HappRolePassport {
                role: role.to_string(),
                dna_hash,
                coordinator_wasm_hashes: BTreeMap::new(),
                error: Some(format!("get_dna_definition failed: {error}")),
            }),
        }
    }

    HappPassport {
        app_id: app_id.to_string(),
        roles,
        error: None,
    }
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
        };
        let json = serde_json::to_value(role).unwrap();
        assert!(json.get("dnaHash").is_some());
        assert!(json.get("coordinatorWasmHashes").is_some());
        assert!(json.get("dna_hash").is_none());
    }
}
