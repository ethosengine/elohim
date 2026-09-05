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
use crate::services::lineage_bridge::{AgentSweep, SweepKey};

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
    /// **Task 17.** The constitutional root this role's INSTALLED cell
    /// declares, read from its DNA modifiers' `properties` (the same
    /// `LineageProperties` map the node-registry integrity zome reads
    /// `lineage` out of). `None` — and so omitted — for a role whose
    /// properties declare no root, which is every role that has never crossed
    /// under a constitution.
    ///
    /// This is what makes `release_adoption::verify_path`'s `root_mismatch`
    /// arm reachable: it compares roots only when the installed role declares
    /// one, and before this field existed every role declared `None`, so a
    /// path notarized under a FOREIGN root was accepted on every live peer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constitution_root: Option<String>,
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
    /// **Task 14b.** The constitutional root the AUTHORING cell declares, read
    /// off the side app's own DNA modifiers exactly as
    /// [`HappRolePassport::constitution_root`] is read off the base app's.
    ///
    /// It exists because after a crossing the two cells declare DIFFERENT
    /// roots, and the role-level field reports the BASE app's. Before this
    /// field, `release_adoption::verify::InstalledReality::from_happ_passport`
    /// had no way to see the root that `happ_manager::install_lineage` minted
    /// into the v2 cell — so `RootSource::Installed` was dead on every crossed
    /// role, and the root check silently fell back to the roster's.
    ///
    /// `None` — and so omitted — when the authoring cell declares no root, or
    /// when the lookup failed (which is reported on the role's `error`, never
    /// smuggled in here as an absent root).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authoring_constitution_root: Option<String>,
    /// **Task 14b.** The AUTHORING cell's coordinator wasm hashes, same
    /// reasoning: after a crossing they are the zomes actually answering for
    /// this role, and the role-level `coordinatorWasmHashes` still describes
    /// the base cell. Empty — and absent — when the lookup failed or no window
    /// is open.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub authoring_coordinator_wasm_hashes: BTreeMap<String, String>,
    pub closed: bool,
    /// **Task 12.** Whether a record authored on V2 during this window can
    /// travel BACKWARD into the v1 line.
    ///
    /// At MVP the answer is `unavailable` whenever the two cells are on
    /// different DNAs, and it is a fact about v1 rather than a limitation of
    /// this build: the v1 integrity zome has no witness entry type, so there is
    /// nothing on that side that could hold a carried v2 record. Forward carry
    /// (v1 → v2, both the apply vehicle's self-carry and the bridge's
    /// held-carry) is unaffected.
    ///
    /// Reported rather than assumed so an operator reading `/version` during a
    /// window is never left to infer it from silence.
    pub backward_carry: BackwardCarry,
    /// **Task 12.** What the trailing bridge sweep has OBSERVED of each
    /// neighbour still authoring on v1, sorted by agent. Empty — and absent
    /// from the wire — before the first sweep, and on a build with no bridge
    /// wired.
    ///
    /// **These are observations, never claims of completeness.** Each entry
    /// describes this peer's own integrated view of that neighbour's chain; a
    /// held page is not self-evidencing. Station 6 establishes completeness by
    /// comparing this view against the neighbour's OWN `export_records`, which
    /// is a harness-side cross-view check — storage never calls a neighbour's
    /// HTTP to manufacture one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sweep: Vec<AgentSweepView>,
}

/// Whether backward (v2 → v1) carry is possible for a role's crossing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackwardCarry {
    /// The two cells are on different DNAs and the v1 line has no witness type
    /// by construction — a record authored on v2 has nowhere to land in v1.
    Unavailable,
    /// Reading and authoring resolve to the same DNA, so nothing has to cross
    /// a line at all.
    Available,
}

/// One neighbour's trailing sweep, projected from
/// [`crate::services::lineage_bridge::AgentSweep`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSweepView {
    /// The neighbour, canonical `uhCAk…`.
    pub agent: String,
    /// Where the next tick resumes; absent means "at the beginning", which is
    /// equally the fresh state, the end of the local view, and a restart.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<u32>,
    /// The highest action sequence the predecessor observed for this chain —
    /// the ONE number that reaches past this peer's own view, and so the only
    /// cross-view staleness signal a reader gets. Its distance from `total` is
    /// normally large (it spans every action, not just app entries) and is not
    /// a staleness measure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_head: Option<u32>,
    /// The record count of THIS PEER'S view of the neighbour's chain — never
    /// the neighbour's own total.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u32>,
    /// The digest of this peer's view. Two couriers at different catch-up
    /// points legitimately report different digests for the same neighbour;
    /// that is a staleness signal, not a fork.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    /// Records this sweep has NEWLY moved into v2 for this neighbour,
    /// accumulated over ticks. A re-walk of an already-carried view adds
    /// nothing.
    pub carried: u32,
    /// How many action rows the predecessor's POSITION scan read to find the
    /// last page of this neighbour's walk — **risk row R1's metric**, per
    /// neighbour.
    ///
    /// What to read it against: the neighbour's chain length. A page that
    /// resumed a pinned walk reads only its own probe span, so `scanned` stays
    /// small and, the property that actually matters, does not grow with how far
    /// into the chain the page sits. An unpinned page reports the whole chain's
    /// action count, because finding an arbitrary ordinal costs exactly that.
    ///
    /// Absent while the v2 cell's `carry_from` does not forward its export
    /// page's `scanned` — "not reported", never a fabricated 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanned: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sweep: Option<String>,
    /// A page failure, or the `restarted:` note a mid-walk digest change
    /// leaves. Cleared by the next clean page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
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
    /// **Task 12.** Point-in-time copy of the trailing bridge sweep
    /// (`LineageBridge::snapshot()`), keyed `(role, agent)`. Same discipline as
    /// `lineage` above: the passport holds the SNAPSHOT, never the
    /// `Arc<LineageBridge>`. Empty on a node with no bridge wired and on one
    /// that has never swept — both of which serialize identically to the
    /// pre-Task-12 response, because a role with no lineage window emits no
    /// `lineage` object to hang a `sweep` off at all.
    pub sweep: BTreeMap<SweepKey, AgentSweep>,
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
            inspect_installed_happ(admin, &ctx.app_id, &ctx.lineage, &ctx.sweep),
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
    sweep: &BTreeMap<SweepKey, AgentSweep>,
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
        let Some(provisioned) = cells.iter().find_map(|cell| match cell {
            CellInfo::Provisioned(provisioned) => Some(provisioned),
            _ => None,
        }) else {
            roles.push(HappRolePassport {
                role: role.to_string(),
                dna_hash: "unknown".to_string(),
                coordinator_wasm_hashes: BTreeMap::new(),
                error: Some("role has no provisioned cell".to_string()),
                lineage: None,
                constitution_root: None,
            });
            continue;
        };
        let cell_id = provisioned.cell_id.clone();
        // Task 17: the root is a property of the INSTALLED cell's modifiers,
        // which `list_apps` already returned — no extra admin round trip, and
        // no second source that could disagree with the hash beside it.
        let constitution_root =
            constitution_root_from_properties(provisioned.dna_modifiers.properties.bytes());

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
        // **Task 14b.** When a window is open (or was sunset) the AUTHORING
        // cell is the one actually answering for this role, so its identity is
        // read too: its DNA hash, its constitutional root, and its coordinator
        // wasm hashes. All three come from the SAME provisioned cell, in one
        // pass, so they cannot describe different cells — which is exactly the
        // failure mode a second lookup would introduce.
        let (authoring_dna_hash, authoring_cell) = if needs_lineage_view {
            let authoring_app_id = &lineage[role].authoring_app_id;
            match resolve_role_cell(&apps, authoring_app_id, role) {
                Ok(provisioned) => (
                    Some(provisioned.cell_id.dna_hash().to_string()),
                    Some(provisioned),
                ),
                Err(lookup_error) => {
                    if role_error.is_none() {
                        role_error = Some(lookup_error);
                    }
                    (Some("unknown".to_string()), None)
                }
            }
        } else {
            (None, None)
        };
        let authoring_constitution_root = authoring_cell
            .as_ref()
            .and_then(|c| constitution_root_from_properties(c.dna_modifiers.properties.bytes()));
        let authoring_coordinator_wasm_hashes = match authoring_cell {
            Some(provisioned) => match admin.get_dna_definition(provisioned.cell_id).await {
                Ok(definition) => definition
                    .coordinator_zomes
                    .iter()
                    .filter_map(|(name, zome)| {
                        crate::happ_manager::coordinator_wasm_hash(zome)
                            .map(|hash| (name.to_string(), hash.to_string()))
                    })
                    .collect(),
                Err(error) => {
                    if role_error.is_none() {
                        role_error = Some(format!("authoring get_dna_definition failed: {error}"));
                    }
                    BTreeMap::new()
                }
            },
            None => BTreeMap::new(),
        };
        let lineage_view = lineage_view_for(
            role,
            &dna_hash,
            lineage,
            authoring_dna_hash,
            authoring_constitution_root,
            authoring_coordinator_wasm_hashes,
            sweep,
        );

        roles.push(HappRolePassport {
            role: role.to_string(),
            dna_hash,
            coordinator_wasm_hashes,
            error: role_error,
            lineage: lineage_view,
            constitution_root,
        });
    }

    HappPassport {
        app_id: app_id.to_string(),
        roles,
        error: None,
        lineage_apps,
    }
}

/// The identity-bearing DNA properties, as far as the PASSPORT needs to read
/// them.
///
/// A storage-side mirror of the node-registry integrity zome's
/// `LineageProperties` (`node_registry_integrity/src/lib.rs`), narrowed to the
/// one field the passport projects. It is a mirror rather than a shared type
/// because the zome's own struct is behind the `lineage-witness` cargo feature
/// and holds `Vec<DnaHash>`, which this side has no reason to decode.
///
/// Unknown keys are IGNORED by construction (no `deny_unknown_fields`), which
/// is load-bearing: the same properties map carries the bootstrap steward's
/// `progenitor_pubkey` and the `lineage` list, and a strict decode would read
/// every one of today's installed cells as having no root.
#[derive(Debug, Clone, serde::Deserialize)]
struct PassportLineageProperties {
    #[serde(default)]
    constitution_root: Option<String>,
}

/// **Task 17.** The `constitution_root` an installed cell's DNA modifiers
/// declare, or `None`.
///
/// `properties` is the msgpack `SerializedBytes` the conductor stored at
/// install time (see `happ_manager::install_lineage`, which writes the same
/// map through `YamlProperties`). Every failure mode — properties that are
/// nil, properties that are not a map, a map with no `constitution_root`, a
/// root declared as the empty string — is `None`, i.e. "this role declares no
/// root".
///
/// `None` is SAFE here, and deliberately so: `verify_path` compares roots only
/// when the installed role declares one, so a role with no readable root
/// imposes no root constraint rather than refusing every path. The alternative
/// — inventing a root, or failing the whole passport — would either fabricate
/// a constitutional fact or take the node's `/version` down over a property we
/// simply could not read.
pub(crate) fn constitution_root_from_properties(properties: &[u8]) -> Option<String> {
    rmp_serde::from_slice::<PassportLineageProperties>(properties)
        .ok()?
        .constitution_root
        .filter(|root| !root.is_empty())
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
fn resolve_role_cell(
    apps: &[holochain_client::AppInfo],
    app_id: &str,
    role: &str,
) -> Result<holochain_client::ProvisionedCell, String> {
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
            CellInfo::Provisioned(provisioned) => Some(provisioned.clone()),
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
#[allow(clippy::too_many_arguments)]
fn lineage_view_for(
    role: &str,
    dna_hash: &str,
    snapshot: &BTreeMap<String, RoleLineage>,
    authoring_dna_hash: Option<String>,
    authoring_constitution_root: Option<String>,
    authoring_coordinator_wasm_hashes: BTreeMap<String, String>,
    sweep: &BTreeMap<SweepKey, AgentSweep>,
) -> Option<RoleLineageView> {
    let entry = snapshot.get(role)?;
    if entry.authoring_app_id == entry.reading_app_id && !entry.closed {
        return None;
    }
    let authoring_dna_hash = authoring_dna_hash.unwrap_or_else(|| "unknown".to_string());
    // An `"unknown"` authoring hash is NOT read as "same DNA": a lookup that
    // failed is not evidence that backward carry is possible, and reporting
    // `available` from a failed lookup is the one direction of error that
    // false-reassures.
    let backward_carry = if authoring_dna_hash == dna_hash {
        BackwardCarry::Available
    } else {
        BackwardCarry::Unavailable
    };
    Some(RoleLineageView {
        reading_app_id: entry.reading_app_id.clone(),
        authoring_app_id: entry.authoring_app_id.clone(),
        reading_dna_hash: dna_hash.to_string(),
        authoring_dna_hash,
        authoring_constitution_root,
        authoring_coordinator_wasm_hashes,
        closed: entry.closed,
        backward_carry,
        sweep: sweep_view_for(role, sweep),
    })
}

/// This role's slice of the bridge sweep, agent-sorted (the `BTreeMap` key
/// order already is).
fn sweep_view_for(role: &str, sweep: &BTreeMap<SweepKey, AgentSweep>) -> Vec<AgentSweepView> {
    sweep
        .iter()
        .filter(|((swept_role, _), _)| swept_role == role)
        .map(|((_, agent), state)| AgentSweepView {
            agent: agent.clone(),
            cursor: state.cursor,
            observed_head: state.observed_head,
            total: state.total,
            digest: state.last_digest.clone(),
            carried: state.carried,
            scanned: state.scanned,
            last_sweep: state.last_sweep.clone(),
            last_error: state.last_error.clone(),
        })
        .collect()
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
            sweep: BTreeMap::new(),
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
            sweep: BTreeMap::new(),
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
            constitution_root: None,
        };
        let json = serde_json::to_value(role).unwrap();
        assert!(json.get("dnaHash").is_some());
        assert!(json.get("coordinatorWasmHashes").is_some());
        assert!(json.get("dna_hash").is_none());
        assert!(json.get("lineage").is_none());
        // **Task 17.** A role declaring no root omits the key entirely —
        // byte-identical to the pre-Task-17 shape, which is what keeps this
        // an additive `/version` change.
        assert!(json.get("constitutionRoot").is_none());
    }

    /// **Task 17.** The properties decode, against the msgpack shape the
    /// conductor actually stores.
    ///
    /// Modifiers `properties` is a `SerializedBytes` msgpack MAP, written at
    /// install time from the same JSON `happ_manager::install_lineage` builds.
    /// The fixture therefore carries the OTHER keys that really live beside
    /// the root — `progenitor_pubkey` (the bootstrap steward) and `lineage`
    /// (the DNA chain the node-registry integrity zome validates against) —
    /// because a strict decode would read every installed cell today as
    /// having no root, and this test is the thing that would notice.
    #[test]
    fn constitution_root_is_read_from_the_cells_own_properties() {
        #[derive(serde::Serialize)]
        struct Props<'a> {
            progenitor_pubkey: &'a str,
            lineage: Vec<&'a str>,
            constitution_root: Option<&'a str>,
        }

        let with_root = rmp_serde::to_vec_named(&Props {
            progenitor_pubkey: "uhCAkBootstrapStewardKey",
            lineage: vec!["uhC0kV1NodeRegistry"],
            constitution_root: Some("bafyLineageConstitutionRoot"),
        })
        .expect("encode properties");
        assert_eq!(
            constitution_root_from_properties(&with_root).as_deref(),
            Some("bafyLineageConstitutionRoot"),
            "the root must survive the two sibling keys it shares the map with"
        );

        // A cell whose properties declare no root — every role today.
        let without_root = rmp_serde::to_vec_named(&Props {
            progenitor_pubkey: "uhCAkBootstrapStewardKey",
            lineage: vec![],
            constitution_root: None,
        })
        .expect("encode properties");
        assert!(constitution_root_from_properties(&without_root).is_none());

        // …and every unreadable shape is the same honest `None`, never a
        // fabricated root and never a failed passport: empty properties (the
        // `SerializedBytes::default()` a plain install leaves), msgpack nil,
        // a non-map, and a root declared as the empty string.
        assert!(constitution_root_from_properties(&[]).is_none());
        assert!(constitution_root_from_properties(&rmp_serde::to_vec(&()).unwrap()).is_none());
        assert!(constitution_root_from_properties(&rmp_serde::to_vec(&7u8).unwrap()).is_none());
        let empty_root = rmp_serde::to_vec_named(&Props {
            progenitor_pubkey: "uhCAkBootstrapStewardKey",
            lineage: vec![],
            constitution_root: Some(""),
        })
        .expect("encode properties");
        assert!(
            constitution_root_from_properties(&empty_root).is_none(),
            "an empty root declares nothing — it must never become a root every path is \
             compared against"
        );
    }

    /// **Task 17, the seam.** A root read off the installed cell reaches
    /// `verify_path` through `InstalledReality::from_happ_passport` — the one
    /// line that made `root_mismatch` reachable at all. Pinned here because
    /// the passport is where the value originates; the refusal itself is
    /// pinned in `release_adoption::verify`.
    #[test]
    fn the_passports_root_reaches_installed_reality() {
        use crate::services::release_adoption::verify::InstalledReality;
        use seam_contracts::Answer;

        let happ = HappPassport {
            app_id: "elohim".to_string(),
            roles: vec![HappRolePassport {
                role: "node_registry".to_string(),
                dna_hash: "uhC0k-example".to_string(),
                coordinator_wasm_hashes: BTreeMap::new(),
                error: None,
                lineage: None,
                constitution_root: Some("bafyLineageConstitutionRoot".to_string()),
            }],
            error: None,
            lineage_apps: Vec::new(),
        };
        let Answer::Present(installed) = InstalledReality::from_happ_passport(&happ) else {
            panic!("a role with no error is installed reality, not an outage");
        };
        assert_eq!(
            installed.roles["node_registry"]
                .constitution_root
                .as_deref(),
            Some("bafyLineageConstitutionRoot")
        );
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
                lineage: lineage_view_for(
                    "node_registry",
                    "uhC0k-example",
                    &BTreeMap::new(),
                    None,
                    None,
                    BTreeMap::new(),
                    &BTreeMap::new(),
                ),
                constitution_root: None,
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
        assert!(lineage_view_for(
            "node_registry",
            "uhC0k-base",
            &snapshot,
            None,
            None,
            BTreeMap::new(),
            &BTreeMap::new()
        )
        .is_none());
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
                origin: None,
            },
        );
        let view = lineage_view_for(
            "node_registry",
            "uhC0k-base",
            &snapshot,
            Some("uhC0k-lineage".to_string()),
            Some("root-v2".to_string()),
            [(
                "node_registry_coordinator".to_string(),
                "uhCok-v2".to_string(),
            )]
            .into_iter()
            .collect(),
            &BTreeMap::new(),
        )
        .expect("open window projects a view");
        assert_eq!(view.reading_app_id, "elohim");
        assert_eq!(view.authoring_app_id, "elohim@EKiIscIk5BDd");
        assert_eq!(view.reading_dna_hash, "uhC0k-base");
        assert_eq!(view.authoring_dna_hash, "uhC0k-lineage");
        // **Task 14b.** The AUTHORING cell's own identity — the root
        // `install_lineage` minted into v2, and the zomes actually answering
        // for this role now. The role-level fields still describe the base
        // cell, which is why these had to exist.
        assert_eq!(view.authoring_constitution_root.as_deref(), Some("root-v2"));
        assert_eq!(
            view.authoring_coordinator_wasm_hashes
                .get("node_registry_coordinator")
                .map(String::as_str),
            Some("uhCok-v2")
        );
        assert!(!view.closed);
        // Task 12: the two cells are on different DNAs, so a v2 record has
        // nowhere to land in the v1 line.
        assert_eq!(view.backward_carry, BackwardCarry::Unavailable);
        assert!(view.sweep.is_empty(), "no bridge sweep has run yet");
    }

    /// **Task 12.** The sweep projection: this role's slice only, agent-sorted,
    /// with the courier's own observations verbatim — and NEVER a completeness
    /// claim, which is why there is no `complete` field to assert on.
    #[test]
    fn the_sweep_view_carries_this_roles_neighbours_only() {
        use crate::services::lineage_bridge::AgentSweep;

        let mut snapshot = BTreeMap::new();
        snapshot.insert(
            "node_registry".to_string(),
            RoleLineage {
                reading_app_id: "elohim".to_string(),
                authoring_app_id: "elohim@EKiIscIk5BDd".to_string(),
                closed: false,
                origin: None,
            },
        );
        let mut sweep = BTreeMap::new();
        sweep.insert(
            ("node_registry".to_string(), "uhCAkJessica".to_string()),
            AgentSweep {
                cursor: Some(16),
                last_digest: Some("digest-a".into()),
                resume: None,
                scanned: Some(8),
                observed_head: Some(57),
                total: Some(41),
                carried: 16,
                last_sweep: Some("2026-09-04T00:00:00Z".into()),
                last_error: None,
                halted: false,
            },
        );
        // A different ROLE's neighbour must not leak into this role's view.
        sweep.insert(
            ("lamad".to_string(), "uhCAkAdam".to_string()),
            AgentSweep::default(),
        );

        let view = lineage_view_for(
            "node_registry",
            "uhC0k-base",
            &snapshot,
            Some("uhC0k-lineage".to_string()),
            None,
            BTreeMap::new(),
            &sweep,
        )
        .expect("open window projects a view");
        assert_eq!(view.sweep.len(), 1);
        let jessica = &view.sweep[0];
        assert_eq!(jessica.agent, "uhCAkJessica");
        assert_eq!(jessica.cursor, Some(16));
        assert_eq!(jessica.observed_head, Some(57));
        assert_eq!(jessica.total, Some(41));
        assert_eq!(jessica.digest.as_deref(), Some("digest-a"));
        assert_eq!(jessica.carried, 16);
        assert_eq!(
            jessica.scanned,
            Some(8),
            "R1's metric is visible per neighbour, not only in a log line"
        );
        assert!(jessica.last_error.is_none());

        // And it reaches the wire in camelCase, under the role's `lineage`.
        let json = serde_json::to_value(&view).unwrap();
        assert_eq!(json["backwardCarry"], "unavailable");
        // Both Task 14b fields are omitted when the authoring lookup yielded
        // nothing — a build with no root declared stays byte-identical here.
        assert!(json.get("authoringConstitutionRoot").is_none());
        assert!(json.get("authoringCoordinatorWasmHashes").is_none());
        assert_eq!(json["sweep"][0]["observedHead"], 57);
        assert_eq!(json["sweep"][0]["lastSweep"], "2026-09-04T00:00:00Z");
        assert!(json["sweep"][0].get("lastError").is_none());
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
                origin: None,
            },
        );
        let view = lineage_view_for(
            "node_registry",
            "uhC0k-base",
            &snapshot,
            None,
            None,
            BTreeMap::new(),
            &BTreeMap::new(),
        )
        .expect("closed=true alone still projects a view");
        assert_eq!(view.reading_app_id, "elohim");
        assert_eq!(view.authoring_app_id, "elohim");
        assert_eq!(view.authoring_dna_hash, "unknown");
        assert!(view.closed);
        // An `"unknown"` authoring hash is never read as "same DNA" — a failed
        // lookup must not report backward carry as available.
        assert_eq!(view.backward_carry, BackwardCarry::Unavailable);
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
                origin: None,
            },
        );
        assert!(lineage_view_for(
            "node_registry",
            "uhC0k-base",
            &snapshot,
            None,
            None,
            BTreeMap::new(),
            &BTreeMap::new()
        )
        .is_none());
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
