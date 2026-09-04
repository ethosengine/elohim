//! hApp Lifecycle Manager
//!
//! Handles first install, stale detection, and re-install of the Elohim hApp.
//! Rust port of `elohim/holochain/edgenode/scripts/install-happ.cjs`.
//!
//! ## Lifecycle
//!
//! 1. Check if the app is already installed via `list_apps`
//! 2. If installed, verify all expected DNA roles are present and provisioned
//! 3. If stale (missing roles or empty cells), reinstall — gated, see below
//! 4. If disabled, enable
//! 5. If not installed, do a fresh install
//! 6. Always ensure an app interface is attached on the expected port
//!
//! ## Reinstall gates — a standing flag is not migration intent
//!
//! A reinstall UNINSTALLS first. Holochain deletes the authored (source-chain)
//! databases, the following install mints a NEW agent key, and the uninstall is
//! **not atomic**: it can fail partway (a conductor-DB lock timeout under a
//! post-restart read storm is enough), leaving the authored DBs gone while
//! conductor state still lists the app Enabled with its cells — after which the
//! conductor panics `CellWithoutGenesis` on every subsequent boot and the node's
//! DHT data is orphaned from chains that cannot be recovered (re-genesis on an
//! empty chain under the same key is a fork). A reinstall is therefore a
//! DATA-DESTROYING migration act, never a routine deploy step.
//!
//! That is why a standing per-environment env var no longer arms it:
//!
//! | Env | Meaning |
//! |-----|---------|
//! | `DNA_MIGRATION_INTENT` | Comma-separated **bundle** DNA hashes (`uhC0k…`) this roll is migrating TO. The only thing that authorises reinstalling an app that holds data. The drift branch proceeds only when EVERY drifted role's bundle DNA hash appears in the list, and the staleness branch only when every MISSING role's bundle hash does — a partial list keeps the installed cells and names the roles it did not cover. |
//! | `ALLOW_DNA_REINSTALL=true` | A standing deploy flag. It permits the strictly-safe coordinator hot-swap ([`coordinator_update_allowed`]) and permits a reinstall ONLY of an app that holds no data. On a node with cells it NO LONGER reinstalls on DNA-content drift. |
//! | `FORCE_DNA_REINSTALL=true` | Operator hammer: skips the drift probe. On an app that holds data it still REFUSES unless `DNA_MIGRATION_INTENT` is present (and covers any drifted role). |
//! | `FORCE_DNA_REINSTALL=wipe` | Unconditional. Reinstalls even an app holding data, with no intent. The one spelling that says "I accept losing these chains". |
//!
//! "Holds data" is read from `app_info` alone — the app has at least one
//! provisioned cell and is past `AwaitingMemproofs`, i.e. genesis has run so a
//! source chain exists on disk. Chain length is never measured. (Note the
//! deliberate conservatism: a *disabled* app with provisioned cells counts as
//! holding data — its chains are on disk regardless of run state.)
//!
//! When drift is detected without intent the node KEEPS its installed cells and
//! keeps serving on the OLD DNA, logging at ERROR once per boot. That is the
//! safe direction: a node alive on an old DNA can still be migrated later; a
//! node whose chains were deleted cannot be un-deleted. (The coordinator
//! hot-swap refuses cross-lineage bundles separately — see
//! [`lineage_mismatch_error`] — so a kept-installed node does not silently take
//! coordinators compiled against integrity zomes it is not running.)
//!
//! Structural staleness (a missing role / a role with no provisioned cell) is a
//! separate branch, and it is gated the SAME way: the repair is the same
//! whole-app uninstall, so on an app that holds data it would delete the source
//! chains of every role that was still healthy in order to fix the one that was
//! not. Stale + no data reinstalls freely; stale + data needs either a
//! `DNA_MIGRATION_INTENT` covering the missing roles' bundle DNA hashes or
//! `FORCE_DNA_REINSTALL=wipe`, and otherwise keeps the installed cells with one
//! ERROR per boot naming the missing roles and both ways forward.
//!
//! A torn uninstall (see [`uninstall_for_reinstall`]) is TERMINAL for this
//! process on an ordinary node — boot fails with the operator recovery
//! instruction and nothing is installed over the half-removed app — while a node
//! whose own policy says it is re-seedable (`GENESIS_SELF_HEAL_IDENTITY`, the
//! fixture/ephemeral shape) self-heals by clearing its conductor data dir once
//! and re-genesising under a NEW agent key.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use holochain_client::{AdminWebsocket, AllowedOrigins, CellInfo, InstallAppPayload};
use holochain_types::app::{
    AppBundle, AppBundleSource, AppStatus, CoordinatorSource, RoleSettings,
    UpdateCoordinatorsPayload,
};
use holochain_types::dna::{CoordinatorBundle, CoordinatorManifest, DnaFile, ZomeManifest};
use holochain_types::prelude::{
    AgentPubKey, DnaDef, DnaHash, DnaModifiersOpt, YamlProperties, ZomeDependency,
};
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

/// How `FORCE_DNA_REINSTALL` was spelled. `true` is the ordinary operator
/// hammer (still refused on an app holding data without migration intent);
/// `wipe` is the unconditional one that accepts chain loss.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ForceMode {
    /// Unset or unrecognised — no forcing.
    #[default]
    Off,
    /// `FORCE_DNA_REINSTALL=true`.
    On,
    /// `FORCE_DNA_REINSTALL=wipe`.
    Wipe,
}

/// The reinstall gates, lifted out of the environment so the decision itself
/// ([`decide_drift_action`]) stays pure and table-testable.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReinstallFlags {
    /// `ALLOW_DNA_REINSTALL=true`. A STANDING deploy flag: it permits the safe
    /// coordinator hot-swap and permits reinstalling an app that holds no data.
    /// It does NOT authorise reinstalling an app that holds data — that needs
    /// `migration_intent`.
    pub allow_reinstall: bool,
    /// `FORCE_DNA_REINSTALL`.
    pub force: ForceMode,
    /// `DNA_MIGRATION_INTENT` — the set of BUNDLE DNA hashes (`uhC0k…`) this
    /// roll is migrating TO. Empty = no intent declared for this boot.
    pub migration_intent: BTreeSet<String>,
}

impl ReinstallFlags {
    /// Read the gates from the process environment (the only impure step).
    pub fn from_env() -> Self {
        Self {
            allow_reinstall: env_flag("ALLOW_DNA_REINSTALL"),
            force: parse_force_mode(std::env::var("FORCE_DNA_REINSTALL").ok().as_deref()),
            migration_intent: parse_migration_intent(
                std::env::var("DNA_MIGRATION_INTENT").ok().as_deref(),
            ),
        }
    }
}

/// `FORCE_DNA_REINSTALL` spelling → [`ForceMode`]. Anything unrecognised is
/// `Off`: a typo must never arm a destructive path.
fn parse_force_mode(raw: Option<&str>) -> ForceMode {
    match raw.map(|v| v.trim().to_ascii_lowercase()).as_deref() {
        Some("wipe") => ForceMode::Wipe,
        Some("true") => ForceMode::On,
        _ => ForceMode::Off,
    }
}

/// `DNA_MIGRATION_INTENT` → the set of bundle DNA hashes named. Comma
/// separated, whitespace tolerated, empties dropped.
fn parse_migration_intent(raw: Option<&str>) -> BTreeSet<String> {
    raw.map(|v| {
        v.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
    })
    .unwrap_or_default()
}

/// One role whose installed DNA hash differs from the bundle's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriftedRole {
    pub role: String,
    /// The DNA hash of the PROVISIONED cell (`uhC0k…`).
    pub installed_dna_hash: String,
    /// The DNA hash this bundle resolves for the role (`uhC0k…`) — the value an
    /// operator puts in `DNA_MIGRATION_INTENT` to authorise the migration.
    pub bundle_dna_hash: String,
}

/// What the installed app's roles look like, reduced to exactly what the
/// reinstall decision needs — read from `app_info`, never from the filesystem.
///
/// `InstalledRoles`, NOT `InstalledApp`: `holochain_types::app::InstalledApp`
/// is in scope here through the prelude glob, and the collision is a hard
/// ambiguity error. Do not rename this back.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstalledRoles {
    /// role → provisioned-cell DNA hash.
    pub role_dna_hashes: BTreeMap<String, String>,
    /// Genesis has run for at least one cell, so authored (source-chain) DBs
    /// exist on disk and an uninstall would destroy them. See the module doc
    /// for why a disabled app still counts.
    pub has_data: bool,
    /// Expected roles that are missing from `cell_info` or have no provisioned
    /// cell — structural staleness, named rather than a bare bool so the
    /// refusal can tell an operator WHICH roles are gone and which bundle
    /// hashes an intent has to cover.
    pub missing_roles: Vec<String>,
}

impl InstalledRoles {
    fn from_app_info(app_info: &holochain_client::AppInfo) -> Self {
        let role_dna_hashes = provisioned_dna_hashes(app_info);
        // "Has data" from app_info alone: provisioned cells exist AND the app is
        // past the pre-genesis states (in which genesis has NOT completed, so
        // there is no chain to lose). Disabled-with-cells counts as data.
        //
        // 0.7 added `AwaitingRestore` alongside `AwaitingMemproofs`: restore is
        // in progress for one or more cells and zome calls are rejected, so
        // genesis has not completed there either. Storage never sets
        // `restore_from_dht`, so this state is unreachable for us — it is
        // matched explicitly rather than left to the `!matches!` default, so a
        // future restore-capable path cannot silently read as "has data".
        let genesis_ran = !matches!(
            app_info.status,
            AppStatus::AwaitingMemproofs | AppStatus::AwaitingRestore
        );
        Self {
            has_data: genesis_ran && !role_dna_hashes.is_empty(),
            missing_roles: missing_provisioned_roles(app_info),
            role_dna_hashes,
        }
    }
}

/// The verdict for this boot. Every variant that reinstalls goes through
/// [`uninstall_for_reinstall`], which refuses to begin a non-atomic uninstall
/// against a saturated conductor and never retries a torn one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriftAction {
    /// No structural staleness, no content drift — serve the installed app
    /// (coordinator hot-swap path still runs).
    NoOp,
    /// Structural staleness — reinstall. Pre-existing branch, ungated by intent.
    ReinstallStale,
    /// `FORCE_DNA_REINSTALL=wipe`, or `=true` on an app holding no data.
    ReinstallForced,
    /// Content drift AND an explicit `DNA_MIGRATION_INTENT` naming every
    /// drifted role's bundle hash — the authorised migration. Mints a new key.
    ReinstallForMigration { roles: Vec<DriftedRole> },
    /// Content drift with no (or partial) intent — KEEP the installed cells and
    /// keep serving the old DNA. `missing_intent` names the roles whose bundle
    /// hash the operator did not list.
    KeepInstalled {
        drifted: Vec<DriftedRole>,
        missing_intent: Vec<DriftedRole>,
    },
    /// `FORCE_DNA_REINSTALL=true` against an app holding data, with neither
    /// `DNA_MIGRATION_INTENT` nor the `wipe` spelling — refused.
    RefuseForceWithoutIntent { drifted: Vec<DriftedRole> },
    /// Structural staleness on an app that HOLDS DATA, with no intent — refused.
    /// Reinstalling to repair one missing role would delete the source chains of
    /// every role that was healthy, so the node keeps serving what it has.
    RefuseStaleWithoutIntent { missing_roles: Vec<String> },
}

impl DriftAction {
    /// Whether this verdict destroys the installed app before installing.
    pub fn reinstalls(&self) -> bool {
        matches!(
            self,
            DriftAction::ReinstallStale
                | DriftAction::ReinstallForced
                | DriftAction::ReinstallForMigration { .. }
        )
    }
}

/// The whole reinstall decision, isolated from the conductor, the bundle file
/// and the environment — pure, so the gate is a table, not a trace.
///
/// `installed_roles` is the reduced view of `app_info`; `bundle_roles` maps role
/// → the DNA hash the bundle on disk resolves; `flags` are the parsed env gates.
pub fn decide_drift_action(
    installed_roles: &InstalledRoles,
    bundle_roles: &BTreeMap<String, String>,
    flags: &ReinstallFlags,
) -> DriftAction {
    let drifted: Vec<DriftedRole> = bundle_roles
        .iter()
        .filter_map(|(role, bundle_hash)| {
            let installed_hash = installed_roles.role_dna_hashes.get(role)?;
            if installed_hash == bundle_hash {
                return None;
            }
            Some(DriftedRole {
                role: role.clone(),
                installed_dna_hash: installed_hash.clone(),
                bundle_dna_hash: bundle_hash.clone(),
            })
        })
        .collect();

    let missing_intent: Vec<DriftedRole> = drifted
        .iter()
        .filter(|d| !flags.migration_intent.contains(&d.bundle_dna_hash))
        .cloned()
        .collect();
    // An intent must be DECLARED (non-empty) and must cover every drifted role.
    // Empty-intent would satisfy `missing_intent.is_empty()` vacuously when
    // nothing drifted; requiring non-empty keeps "no intent" from ever reading
    // as "intent satisfied".
    let intent_authorises = !flags.migration_intent.is_empty() && missing_intent.is_empty();

    match flags.force {
        // The unconditional hammer: the operator has said, in the one spelling
        // that means it, that chain loss is accepted.
        ForceMode::Wipe => return DriftAction::ReinstallForced,
        ForceMode::On => {
            return if !installed_roles.has_data {
                // Nothing to lose — no cells, or genesis never completed.
                DriftAction::ReinstallForced
            } else if intent_authorises {
                DriftAction::ReinstallForMigration { roles: drifted }
            } else {
                DriftAction::RefuseForceWithoutIntent { drifted }
            };
        }
        ForceMode::Off => {}
    }

    // Structural staleness: a role is missing or has no provisioned cell. The
    // repair is the SAME destructive uninstall — and it takes down the healthy
    // roles' source chains with it, so on an app that holds data it needs the
    // same explicit intent a content migration does. `FORCE_DNA_REINSTALL=wipe`
    // already returned above; a non-empty `DNA_MIGRATION_INTENT` covering the
    // missing roles' bundle hashes is the other way through.
    if !installed_roles.missing_roles.is_empty() {
        let stale_intent_covers = !flags.migration_intent.is_empty()
            && installed_roles.missing_roles.iter().all(|role| {
                bundle_roles
                    .get(role)
                    .is_some_and(|h| flags.migration_intent.contains(h))
            });
        return if !installed_roles.has_data || stale_intent_covers {
            DriftAction::ReinstallStale
        } else {
            DriftAction::RefuseStaleWithoutIntent {
                missing_roles: installed_roles.missing_roles.clone(),
            }
        };
    }

    if drifted.is_empty() {
        return DriftAction::NoOp;
    }

    if intent_authorises {
        DriftAction::ReinstallForMigration { roles: drifted }
    } else if !installed_roles.has_data && flags.allow_reinstall {
        // No chains at risk: the standing flag is still enough here, which is
        // the ephemeral re-seeded-env behaviour this gate was written for.
        DriftAction::ReinstallForced
    } else {
        DriftAction::KeepInstalled {
            drifted,
            missing_intent,
        }
    }
}

/// Emit the operator-facing narration for a verdict. Kept beside the pure
/// decision so the decision itself never logs.
///
/// The `DNA drift detected for role` and `Stale hApp detected` shapes are
/// grepped by CI — do not reword them.
fn log_drift_action(app_id: &str, action: &DriftAction) {
    // The no-intent refusals are loud but not per-tick: once per process each.
    static DRIFT_REFUSAL_LOGGED: AtomicBool = AtomicBool::new(false);
    static STALE_REFUSAL_LOGGED: AtomicBool = AtomicBool::new(false);

    let narrate_drift = |roles: &[DriftedRole]| {
        for d in roles {
            warn!(
                role = d.role.as_str(),
                installed = d.installed_dna_hash.as_str(),
                bundle = d.bundle_dna_hash.as_str(),
                "DNA drift detected for role"
            );
        }
    };

    match action {
        DriftAction::NoOp => {}
        DriftAction::ReinstallStale => {
            warn!(app_id = app_id, "Stale hApp detected — reinstalling");
        }
        DriftAction::ReinstallForced => {
            warn!(
                app_id = app_id,
                "FORCE_DNA_REINSTALL — reinstalling unconditionally (a new agent key will be minted)"
            );
        }
        DriftAction::ReinstallForMigration { roles } => {
            narrate_drift(roles);
            warn!(
                app_id = app_id,
                roles = roles.len(),
                "DNA content drift vs bundle with DNA_MIGRATION_INTENT covering every drifted role \
                 — migrating (this mints a new agent key and abandons the current source chains)"
            );
        }
        DriftAction::KeepInstalled {
            drifted,
            missing_intent,
        } => {
            narrate_drift(drifted);
            if !DRIFT_REFUSAL_LOGGED.swap(true, Ordering::Relaxed) {
                for d in missing_intent {
                    error!(
                        app_id = app_id,
                        role = d.role.as_str(),
                        installed = d.installed_dna_hash.as_str(),
                        bundle = d.bundle_dna_hash.as_str(),
                        "DNA drift detected but no migration intent for {}: installed={} bundle={} \
                         — keeping the installed cells; set DNA_MIGRATION_INTENT={} to migrate \
                         (this mints a new agent key)",
                        d.role,
                        d.installed_dna_hash,
                        d.bundle_dna_hash,
                        d.bundle_dna_hash
                    );
                }
            }
        }
        DriftAction::RefuseStaleWithoutIntent { missing_roles } => {
            if !STALE_REFUSAL_LOGGED.swap(true, Ordering::Relaxed) {
                error!(
                    app_id = app_id,
                    missing_roles = missing_roles.join(","),
                    "Structurally stale hApp but no migration intent: roles [{}] have no provisioned \
                     cell — NOT reinstalling, because the repair uninstalls the whole app and \
                     would delete the source chains of every role that is still healthy. Keeping \
                     the installed cells. Two ways forward: set \
                     DNA_MIGRATION_INTENT=<bundle DNA hashes of those roles> to repair by \
                     migration, or FORCE_DNA_REINSTALL=wipe to accept the chain loss (both mint a \
                     new agent key)",
                    missing_roles.join(", ")
                );
            }
        }
        DriftAction::RefuseForceWithoutIntent { drifted } => {
            narrate_drift(drifted);
            error!(
                app_id = app_id,
                "FORCE_DNA_REINSTALL=true REFUSED — this app holds data (provisioned cells past \
                 genesis) and an uninstall would delete its source chains irrecoverably; set \
                 DNA_MIGRATION_INTENT=<bundle DNA hashes> to migrate, or FORCE_DNA_REINSTALL=wipe \
                 to accept the loss"
            );
        }
    }
}

/// How long the conductor gets to answer a `list_apps` before we refuse to
/// begin a non-atomic uninstall. Short on purpose: this is a liveness probe of
/// the very conductor DB the uninstall transaction needs, not a data fetch.
const UNINSTALL_PREFLIGHT_BUDGET: Duration = Duration::from_secs(10);

/// Set for the duration of an uninstall and left set if it did not complete.
/// Process-local: once an uninstall has torn, this process must never begin
/// another one — a retry against a half-removed app is how a recoverable
/// timeout becomes a second round of deletions.
static UNINSTALL_TORN: AtomicBool = AtomicBool::new(false);

/// The recovery instruction that must travel with a torn uninstall — the node
/// cannot fix this itself, and guessing (re-installing over a half-removed app)
/// makes it worse.
fn torn_state_error(app_id: &str, cause: &str) -> anyhow::Error {
    anyhow::anyhow!(
        "uninstall of '{app_id}' did not complete ({cause}) — conductor state may be TORN \
         (authored databases removed while the app still lists its cells; the conductor will \
         panic CellWithoutGenesis on the next boot). Refusing to install over it. Recovery is \
         operator-owned: clear `databases/conductor/` in the conductor data dir (the DHT/cache \
         databases can be kept), then let the node re-install. This process will not retry the \
         uninstall."
    )
}

/// Uninstall as the first half of a reinstall — with the two guards the alpha
/// 2026-09-02 loss showed were missing.
///
/// 1. **Preflight.** `uninstall_app` opens a conductor-DB transaction that a
///    post-restart read storm can starve into a 30 s lock timeout — after the
///    authored DBs are already gone. A `list_apps` that cannot answer inside
///    [`UNINSTALL_PREFLIGHT_BUDGET`] says the DB is already saturated, so we
///    refuse to start rather than tear.
/// 2. **No retry into a half-state.** The torn latch is set BEFORE the call and
///    cleared only on success, so a failure (or a crash mid-call) permanently
///    disarms further uninstalls in this process.
async fn uninstall_for_reinstall(admin_ws: &AdminWebsocket, app_id: &str) -> anyhow::Result<()> {
    if UNINSTALL_TORN.load(Ordering::Acquire) {
        return Err(torn_state_error(
            app_id,
            "a previous uninstall in this process did not complete",
        ));
    }

    match tokio::time::timeout(UNINSTALL_PREFLIGHT_BUDGET, admin_ws.list_apps(None)).await {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => {
            error!(
                app_id = app_id,
                error = %e,
                budget_secs = UNINSTALL_PREFLIGHT_BUDGET.as_secs(),
                "conductor DB is saturated — refusing to begin a non-atomic uninstall"
            );
            return Err(anyhow::anyhow!(
                "conductor DB is saturated — refusing to begin a non-atomic uninstall of \
                 '{app_id}' (preflight list_apps failed: {e})"
            ));
        }
        Err(_elapsed) => {
            error!(
                app_id = app_id,
                budget_secs = UNINSTALL_PREFLIGHT_BUDGET.as_secs(),
                "conductor DB is saturated — refusing to begin a non-atomic uninstall"
            );
            return Err(anyhow::anyhow!(
                "conductor DB is saturated — refusing to begin a non-atomic uninstall of \
                 '{app_id}' (preflight list_apps did not answer within {}s)",
                UNINSTALL_PREFLIGHT_BUDGET.as_secs()
            ));
        }
    }

    UNINSTALL_TORN.store(true, Ordering::Release);
    let outcome = admin_ws.uninstall_app(app_id.to_string(), false).await;
    match outcome {
        Ok(()) => {
            UNINSTALL_TORN.store(false, Ordering::Release);
            info!(app_id = app_id, "Old hApp removed");
            Ok(())
        }
        Err(e) => {
            error!(
                app_id = app_id,
                error = %e,
                "uninstall_app FAILED mid-flight — conductor state may be torn; NOT installing over it"
            );
            Err(torn_state_error(
                app_id,
                &format!("uninstall_app failed: {e}"),
            ))
        }
    }
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
        // fix → new DNA hash, same roles), the staleness probe reads "not stale" and the
        // conductor keeps the OLD DNA forever.
        //
        // Healing that drift means UNINSTALLING — deleting the authored source
        // chains and minting a new agent key — so it is gated on an explicit,
        // per-roll `DNA_MIGRATION_INTENT` naming the bundle DNA hashes being
        // migrated TO, never on a standing env flag. See the module doc. The
        // probe itself is unconditional and cheap (it reads the local bundle
        // file, so it cannot time out on the conductor): a node that is NOT
        // authorised to migrate should still say loudly that it has drifted.
        let flags = ReinstallFlags::from_env();
        let installed = InstalledRoles::from_app_info(app_info);
        let bundle = match bundle_dna_hashes(happ_path).await {
            Ok(h) => h,
            Err(e) => {
                error!(
                    error = %e,
                    path = %happ_path.display(),
                    "DNA-drift bundle read FAILED — keeping installed hApp; DNA changes will NOT \
                     auto-deploy until this is resolved"
                );
                // Empty bundle map ⇒ no drifted roles ⇒ the serve path. Worst
                // case is the prior behaviour: keep the installed hApp.
                BTreeMap::new()
            }
        };

        let action = decide_drift_action(&installed, &bundle, &flags);
        log_drift_action(app_id, &action);

        if action.reinstalls() {
            // Guarded: refuses to begin against a saturated conductor DB, and a
            // torn uninstall is fatal for this boot rather than retried.
            uninstall_for_reinstall(admin_ws, app_id).await?;
            install_fresh(admin_ws, happ_path, app_id).await?;
        } else {
            // Serving the INSTALLED app: NoOp, KeepInstalled (drift without
            // intent) and RefuseForceWithoutIntent all land here — the node
            // stays alive on the DNA it already has. A coordinator-ONLY
            // change is invisible to the drift probe either way: the DNA hash
            // covers integrity zomes + modifiers only, so the probe reads
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

            // 0.7 `AppStatus::Unrecoverable(cell_id, reason)` is TERMINAL: restore
            // hit a permanent failure (a locally-validated ChainIntegrityWarrant
            // against the agent) and the app cannot be enabled. It is deliberately
            // NOT folded into `Disabled(_)` — automatically enabling, reinstalling,
            // uninstalling or hot-swapping on it is the same class of unsafe
            // auto-remediation as the torn-uninstall guard. Log it once, loudly,
            // with the reason payload, and leave the DNA_MIGRATION_INTENT /
            // FORCE_DNA_REINSTALL=wipe gates as the only path off it.
            if let AppStatus::Unrecoverable(cell_id, reason) = &app_info.status {
                error!(
                    app_id = app_id,
                    cell_id = ?cell_id,
                    reason = ?reason,
                    "App is UNRECOVERABLE — restore failed permanently. Not enabling, \
                     not reinstalling: operator action required via DNA_MIGRATION_INTENT \
                     or FORCE_DNA_REINSTALL=wipe."
                );
            } else if matches!(app_info.status, AppStatus::Disabled(_)) {
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

/// Which expected roles are structurally stale — missing from `cell_info`, or
/// present with no provisioned cell. Empty = the app is structurally whole.
///
/// Returns the NAMES (not a bool) because staleness on an app that holds data
/// no longer auto-reinstalls: the refusal has to name the roles, and the
/// migration intent has to cover their bundle DNA hashes. Every role is
/// checked, so a partially-torn app reports all of its gaps at once.
fn missing_provisioned_roles(app_info: &holochain_client::AppInfo) -> Vec<String> {
    let cell_info = &app_info.cell_info;
    let mut missing = Vec::new();

    for role in EXPECTED_ROLES {
        match cell_info.get(*role) {
            None => {
                warn!(role = role, "Stale: missing role");
                missing.push((*role).to_string());
            }
            Some(cells) => {
                let provisioned = cells.iter().any(|c| matches!(c, CellInfo::Provisioned(_)));
                if !provisioned {
                    warn!(role = role, "Stale: role has no provisioned cells");
                    missing.push((*role).to_string());
                }
            }
        }
    }

    missing
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
        // 0.7 addition. `true` suppresses genesis and reconstructs each cell's
        // source chain by fetching the agent's prior chain from the DHT, and
        // requires an agent key that already HAS a chain out there. This path
        // mints a brand-new key immediately above, so there is nothing to
        // restore — `false` is the only coherent value here, and it preserves
        // the 0.6 behaviour exactly. Chain restoration is a recovery/migration
        // concern, not an install-if-absent one.
        restore_from_dht: false,
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

/// The lineage-scoped installed-app-id for a second app sharing the base
/// app's agent key: `{base}@{first 12 chars of the DNA hash after the
/// "uhC0k" HoloHash prefix}` — e.g. `elohim@yvKwO2J5u3mf`. Deterministic and
/// collision-resistant enough for the evaluation window a lineage app lives
/// in (it is never the base app's permanent identity).
pub fn lineage_app_id(base: &str, dna_hash: &str) -> String {
    let short: String = dna_hash.chars().skip(5).take(12).collect();
    format!("{base}@{short}")
}

/// Read the network seed the base app's `role` cell was provisioned with, so
/// a lineage install can inherit it. Cells only share a DHT/network family
/// when they share a network seed — installing a lineage app WITHOUT this
/// would silently fork the new cell onto its own network, isolated from the
/// base app's peers, rather than joining the same evolving DHT.
async fn base_role_seed(
    admin_ws: &AdminWebsocket,
    base_app_id: &str,
    role: &str,
) -> anyhow::Result<String> {
    let apps = admin_ws
        .list_apps(None)
        .await
        .map_err(|e| anyhow::anyhow!("list_apps: {e}"))?;
    let base = apps
        .iter()
        .find(|a| a.installed_app_id == base_app_id)
        .ok_or_else(|| anyhow::anyhow!("base app '{base_app_id}' is not installed"))?;
    let cells = base
        .cell_info
        .get(role)
        .ok_or_else(|| anyhow::anyhow!("base app '{base_app_id}' has no role '{role}'"))?;
    let provisioned = cells
        .iter()
        .find_map(|c| match c {
            CellInfo::Provisioned(p) => Some(p),
            _ => None,
        })
        .ok_or_else(|| {
            anyhow::anyhow!("base app '{base_app_id}' role '{role}' has no provisioned cell")
        })?;
    Ok(provisioned.dna_modifiers.network_seed.clone())
}

/// Whether an ALREADY-INSTALLED lineage app still needs its `enable_app` half.
///
/// [`install_lineage`] is install-then-enable, which is two admin calls and
/// therefore two chances to fail between. If the first call installs and then
/// dies at `enable_app`, the app exists on the conductor `Disabled`. A retry
/// that treated "the app id is in `list_apps`" as done would return `Ok` on an
/// app whose cells never started — and the caller (the lineage vehicle) would
/// go on to dial it, fail, and refuse. Forever, on every sweep, with an
/// increasingly confusing error.
///
/// So only [`AppStatus::Enabled`] is done. `Disabled(_)` is the resumable case
/// this predicate exists for. `AwaitingMemproofs` also answers `true`, and
/// `enable_app` will refuse it — deliberately: a legible error every sweep is
/// a better report than a silent `Ok` on an app that cannot serve.
pub(crate) fn lineage_install_needs_enable(status: &holochain_types::prelude::AppStatus) -> bool {
    !matches!(status, holochain_types::prelude::AppStatus::Enabled)
}

/// Install a second app alongside the base app ([`APP_ID`]), under the SAME
/// agent key — never a new one — so both apps' source chains are authored by
/// the one identity. Used to bring a lineage DNA (a coordinator- or
/// integrity-zome evolution of an already-installed role) onto the conductor
/// for evaluation without touching the base app's chains: no uninstall, no
/// re-key, no data loss — see the module doc for why a reinstall is never the
/// answer here.
///
/// `roles_settings[role]` carries a `lineage` property (the DNA hashes this
/// bundle's cell supersedes), an optional `constitution_root` (Task 17b — the
/// constitution the crossing is notarized under), and the base app's network
/// seed for `role`, so the new cell lands on the same DHT the base app's cell
/// for that role is on rather than forking a private network.
///
/// # Why the root is written HERE and not only read
///
/// `runtime_passport::constitution_root_from_properties` reads a role's root
/// off its installed cell's modifiers, and `verify_path` refuses a path whose
/// root disagrees with it. Until this function wrote one, nothing ever did —
/// every cell installed rootless, every passport reported `None`, and the
/// root check could not fire on a live peer. Writing it at install is what
/// closes that loop: the cell this crossing MINTS declares the root its own
/// successor will be checked against, so the constitution is carried forward
/// by the act of crossing rather than re-declared by each release.
///
/// `None` writes no key at all rather than a null — undeclared is a real
/// state (the passport reports `None`, `verify_path` falls back to the
/// manifest's declaration and says `root: undeclared` if there is none
/// either), whereas an empty root is a root nothing can equal.
///
/// Idempotent, and idempotent about the WHOLE job: this function installs AND
/// enables, so "already installed" is only a no-op when the existing app is
/// also enabled. An app left `Disabled` by a previous call that installed and
/// then failed at `enable_app` gets its enable RECONCILED here — see
/// [`lineage_install_needs_enable`] for why returning `Ok` on a disabled app
/// would strand a retry forever.
/// The DNA properties a lineage side app is installed with.
///
/// Pure, so the msgpack shape that lands in the cell's modifiers can be
/// round-tripped against `runtime_passport::constitution_root_from_properties`
/// in a test with no conductor — the two are opposite ends of one wire, and
/// this is the seam that lets them be pinned together.
///
/// A `None` root omits the KEY, rather than writing a null. Both decode to
/// "declares no root" today, but omission is the honest encoding of a fact
/// nobody stated, and it matches what a bundle's own `happ.yaml` looks like
/// before a constitution exists.
fn lineage_properties_json(
    lineage: &[DnaHash],
    constitution_root: Option<&str>,
) -> serde_json::Value {
    let mut props = serde_json::json!({
        "lineage": lineage.iter().map(|h| h.to_string()).collect::<Vec<_>>(),
    });
    if let Some(root) = constitution_root.filter(|r| !r.is_empty()) {
        props["constitution_root"] = serde_json::Value::String(root.to_string());
    }
    props
}

pub async fn install_lineage(
    admin_ws: &AdminWebsocket,
    happ_path: &Path,
    lineage_app_id: &str,
    agent_key: AgentPubKey,
    lineage: &[DnaHash],
    role: &str,
    constitution_root: Option<&str>,
) -> anyhow::Result<()> {
    let apps = admin_ws
        .list_apps(None)
        .await
        .map_err(|e| anyhow::anyhow!("list_apps: {e}"))?;
    if let Some(existing) = apps.iter().find(|a| a.installed_app_id == lineage_app_id) {
        if lineage_install_needs_enable(&existing.status) {
            info!(
                app_id = lineage_app_id,
                status = ?existing.status,
                "lineage app is installed but not enabled — reconciling the enable half"
            );
            admin_ws
                .enable_app(lineage_app_id.to_string())
                .await
                .map_err(|e| {
                    anyhow::anyhow!("enable_app({lineage_app_id}) on retry failed: {e}")
                })?;
            info!(app_id = lineage_app_id, "lineage app enabled on retry");
            return Ok(());
        }
        info!(
            app_id = lineage_app_id,
            "lineage app already installed and enabled — idempotent"
        );
        return Ok(());
    }

    let network_seed = base_role_seed(admin_ws, APP_ID, role).await?;

    // YamlProperties wraps a private `yaml_serde::Value`; the crate is not a
    // direct dependency here, so we go through its generic Deserialize impl
    // (self-describing, format-agnostic) via serde_json instead of naming
    // the type directly.
    let properties: YamlProperties =
        serde_json::from_value(lineage_properties_json(lineage, constitution_root))
            .map_err(|e| anyhow::anyhow!("build lineage properties: {e}"))?;

    let mut roles_settings = std::collections::HashMap::new();
    roles_settings.insert(
        role.to_string(),
        RoleSettings::Provisioned {
            membrane_proof: None,
            modifiers: Some(DnaModifiersOpt {
                network_seed: Some(network_seed),
                properties: Some(properties),
            }),
            init_properties: None,
        },
    );

    let payload = InstallAppPayload {
        source: AppBundleSource::Path(happ_path.to_path_buf()),
        agent_key: Some(agent_key),
        installed_app_id: Some(lineage_app_id.to_string()),
        roles_settings: Some(roles_settings),
        network_seed: None,
        ignore_genesis_failure: false,
        restore_from_dht: false,
    };

    admin_ws
        .install_app(payload)
        .await
        .map_err(|e| anyhow::anyhow!("install_app({lineage_app_id}) failed: {e}"))?;
    info!(
        app_id = lineage_app_id,
        "lineage app installed beside the base app under the existing key"
    );

    admin_ws
        .enable_app(lineage_app_id.to_string())
        .await
        .map_err(|e| anyhow::anyhow!("enable_app failed: {e}"))?;
    info!(app_id = lineage_app_id, "lineage app enabled");

    Ok(())
}

/// Map zome name → coordinator wasm hash (as a string) for a DnaDef.
///
/// This is the drift unit for coordinator-zome changes: the DNA hash covers
/// ONLY integrity zomes + modifiers, so a coordinator-only change (new
/// coordinator wasm, same integrity) produces a bundle whose DNA hashes are
/// byte-identical to the installed app — invisible to the DNA-hash drift probe
/// ([`decide_drift_action`]). The coordinator wasm hashes are where that change
/// IS visible.
fn coordinator_wasm_hashes(dna_def: &DnaDef) -> std::collections::BTreeMap<String, String> {
    dna_def
        .coordinator_zomes
        .iter()
        .filter_map(|(name, zome_def)| {
            coordinator_wasm_hash(zome_def).map(|h| (name.to_string(), h.to_string()))
        })
        .collect()
}

/// The wasm hash of a coordinator zome, or `None` for an inline zome.
///
/// On Holochain 0.6 this was `CoordinatorZomeDef::wasm_hash(&name) -> Result<_>`.
/// 0.7 removed the method: `wasm_hash` is now a plain field on the
/// `ZomeDef::Wasm` variant, reached through `as_any_zome_def()`. The `Option`
/// return carries the same meaning the old `Err` did — an inline zome has no
/// wasm hash — so both call sites keep their existing skip/report behaviour.
pub(crate) fn coordinator_wasm_hash(
    zome_def: &holochain_types::prelude::CoordinatorZomeDef,
) -> Option<holochain_types::prelude::WasmHash> {
    match zome_def.as_any_zome_def() {
        holochain_types::prelude::ZomeDef::Wasm(w) => Some(w.wasm_hash.clone()),
        _ => None,
    }
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

/// The bundle's per-role TARGET coordinator wasm hashes: role name → (zome
/// name → coordinator wasm hash, base64 `uhCok…`).
///
/// The same two steps [`sync_coordinators_for_app_info`] takes to compute a
/// role's `bundled_coordinators`, exposed on their own and **entirely offline
/// from the conductor** — no admin websocket, no `list_apps`, no
/// `get_dna_definition`. That matters because the release-adoption controller
/// needs to know what a staged bundle WOULD install before it decides whether
/// applying it would change anything: a release manifest declares only the
/// coordinator hashes it SUPERSEDES (the packager reads them off the packaging
/// peer's live passport), never the ones it provides, so the target hashes are
/// knowable only from the artifact bytes themselves.
///
/// Same resolution path as [`bundle_role_dna_files`], so the hashes equal what
/// a hot-swap of this exact bundle would splice in — computed once, from the
/// bundle, rather than re-derived by a second implementation that could drift.
pub(crate) async fn bundle_coordinator_wasm_hashes(
    happ_path: &Path,
) -> anyhow::Result<std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>>
{
    let role_dnas = bundle_role_dna_files(happ_path).await?;
    Ok(role_dnas
        .into_iter()
        .map(|(role, dna_file)| (role, coordinator_wasm_hashes(dna_file.dna_def())))
        .collect())
}

/// Build a [`CoordinatorBundle`] from a DnaFile's coordinator zomes — the
/// payload shape `update_coordinators` consumes. Resources are the coordinator
/// wasm bytes already carried in the DnaFile's code map.
async fn coordinator_bundle_from_dna_file(dna_file: &DnaFile) -> anyhow::Result<CoordinatorBundle> {
    let mut zomes = Vec::new();
    let mut resources = Vec::new();
    for (name, zome_def) in &dna_file.dna_def().coordinator_zomes {
        let wasm_hash = coordinator_wasm_hash(zome_def)
            .ok_or_else(|| anyhow::anyhow!("coordinator zome '{name}' has no wasm hash"))?;
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
/// This guard is LOAD-BEARING on the boot path now that DNA-content drift no
/// longer forces a reinstall without `DNA_MIGRATION_INTENT` (see the module
/// doc): a node kept alive on its old DNA reaches the coordinator sweep with a
/// bundle from a different lineage, and every drifted role must be refused here
/// rather than hot-swapped. The HTTP vehicle accepts an ARBITRARY posted bundle
/// and needs the same guard. Returns `Some(error)` when the role must be
/// refused.
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
                ZomeDef::Wasm(WasmZomeDef {
                    wasm_hash: integrity_hash,
                    dependencies: vec![],
                })
                .into(),
            )],
            coordinator_zomes: vec![(
                coordinator_name,
                ZomeDef::Wasm(WasmZomeDef {
                    wasm_hash: coordinator_hash,
                    dependencies: vec![integrity_name],
                })
                .into(),
            )],
        };

        DnaFile::new(dna_def, vec![integrity, coordinator]).await
    }

    /// **Task 17b — the properties WIRE, both ends.**
    ///
    /// `lineage_properties_json` builds what the conductor stores in the side
    /// app's DNA modifiers; `runtime_passport::constitution_root_from_properties`
    /// reads it back off an installed cell. They are opposite ends of ONE wire
    /// and nothing else pins them together, so this round-trips through the
    /// actual msgpack encoding rather than asserting each side in isolation —
    /// which is how a writer and a reader drift into two different ideas of the
    /// same map.
    ///
    /// `YamlProperties` wraps a private yaml value and is not constructible
    /// here without going through serde, so the encode step mirrors what
    /// `install_lineage` does: JSON in, msgpack out, the way
    /// `SerializedBytes` stores it.
    #[test]
    fn install_properties_round_trip_to_the_passport_decoder() {
        use crate::runtime_passport::constitution_root_from_properties;

        let lineage = vec![DnaHash::from_raw_32(vec![0x42; 32])];
        let root = "bafyLineageConstitutionRoot";

        let with_root = lineage_properties_json(&lineage, Some(root));
        assert_eq!(with_root["constitution_root"], root);
        assert_eq!(with_root["lineage"][0], lineage[0].to_string());
        let encoded = rmp_serde::to_vec_named(&with_root).expect("encode properties");
        assert_eq!(
            constitution_root_from_properties(&encoded).as_deref(),
            Some(root),
            "what install writes must be exactly what the passport reads — this is the seam \
             verify_path's root check stands on"
        );

        // No root: the KEY is absent, not null — and the passport reads that
        // as "this role declares no root", which is the honest state for every
        // cell installed before the property existed.
        let without = lineage_properties_json(&lineage, None);
        assert!(
            without.get("constitution_root").is_none(),
            "an undeclared root omits the key rather than writing a null"
        );
        let encoded = rmp_serde::to_vec_named(&without).expect("encode properties");
        assert!(constitution_root_from_properties(&encoded).is_none());
        // The lineage the integrity zome validates against is unchanged by
        // either shape — the root rides ALONGSIDE it, never instead of it.
        assert_eq!(without["lineage"][0], lineage[0].to_string());

        // An empty root is treated as undeclared at the WRITE side too, so an
        // empty `--constitution-root` can never mint a cell under a root
        // nothing can equal.
        assert!(lineage_properties_json(&lineage, Some(""))
            .get("constitution_root")
            .is_none());
    }

    /// **The install-ok/enable-fail resume.** `install_lineage` is two admin
    /// calls; a crash between them leaves the app installed and `Disabled`.
    /// Only `Enabled` counts as done — every other status must send the retry
    /// back through `enable_app` rather than short-circuiting on "the app id
    /// is in list_apps", which would strand the vehicle dialling a dead app on
    /// every sweep forever.
    ///
    /// A predicate rather than an end-to-end test: `AdminWebsocket` has no
    /// offline constructor in this crate (these tests exercise pure hashing),
    /// so the DECISION is what is testable here and the two admin calls around
    /// it are the mesh's to prove (Task 11).
    #[test]
    fn an_installed_but_disabled_lineage_app_still_needs_enabling() {
        use holochain_types::prelude::{AppStatus, DisabledAppReason};
        assert!(!lineage_install_needs_enable(&AppStatus::Enabled));
        for disabled in [
            DisabledAppReason::NeverStarted,
            DisabledAppReason::NotStartedAfterProvidingMemproofs,
            DisabledAppReason::User,
            DisabledAppReason::Error("enable_app died mid-install".into()),
        ] {
            assert!(
                lineage_install_needs_enable(&AppStatus::Disabled(disabled.clone())),
                "a {disabled:?} app is resumable, never done"
            );
        }
        // Not enableable by `enable_app` — but answering `true` means the
        // retry reports a legible refusal every sweep instead of an Ok on an
        // app whose cells never started.
        assert!(lineage_install_needs_enable(&AppStatus::AwaitingMemproofs));
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

    // ---------------------------------------------------------------------
    // The reinstall gate. `decide_drift_action` is pure, so the whole
    // data-destroying decision reads as a table rather than a trace. Written
    // against the alpha 2026-09-02 loss: a STANDING `ALLOW_DNA_REINSTALL=true`
    // met an unintended integrity-hash change on 7 nodes holding ~2.5 GB each,
    // took the reinstall branch, and a transient conductor-DB lock timeout
    // turned a non-atomic uninstall into irrecoverable source-chain loss.
    // ---------------------------------------------------------------------

    /// Installed roles + whether the app holds data. `stale: false` — the
    /// structural branch is exercised separately.
    fn installed_app(roles: &[(&str, &str)], has_data: bool) -> InstalledRoles {
        InstalledRoles {
            role_dna_hashes: roles
                .iter()
                .map(|(r, h)| (r.to_string(), h.to_string()))
                .collect(),
            has_data,
            missing_roles: Vec::new(),
        }
    }

    /// Structurally stale: `role` is expected but has no provisioned cell.
    fn stale_app(roles: &[(&str, &str)], has_data: bool, missing: &[&str]) -> InstalledRoles {
        InstalledRoles {
            missing_roles: missing.iter().map(|r| r.to_string()).collect(),
            ..installed_app(roles, has_data)
        }
    }

    fn bundle_roles(roles: &[(&str, &str)]) -> std::collections::BTreeMap<String, String> {
        roles
            .iter()
            .map(|(r, h)| (r.to_string(), h.to_string()))
            .collect()
    }

    fn gates(allow: bool, force: ForceMode, intent: &[&str]) -> ReinstallFlags {
        ReinstallFlags {
            allow_reinstall: allow,
            force,
            migration_intent: intent.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// THE incident. A standing deploy flag met an unintended integrity-hash
    /// change on nodes holding data — and must no longer arm the uninstall.
    #[test]
    fn drift_without_intent_keeps_installed_cells() {
        let action = decide_drift_action(
            &installed_app(&[("lamad", "uhC0k-old")], true),
            &bundle_roles(&[("lamad", "uhC0k-new")]),
            &gates(true, ForceMode::Off, &[]),
        );
        match &action {
            DriftAction::KeepInstalled {
                drifted,
                missing_intent,
            } => {
                assert_eq!(drifted.len(), 1);
                assert_eq!(drifted[0].role, "lamad");
                assert_eq!(drifted[0].installed_dna_hash, "uhC0k-old");
                assert_eq!(drifted[0].bundle_dna_hash, "uhC0k-new");
                assert_eq!(missing_intent, drifted, "no intent ⇒ every role uncovered");
            }
            other => panic!("a standing flag must not reinstall a node with data: {other:?}"),
        }
        assert!(!action.reinstalls(), "nothing may be uninstalled here");
    }

    /// Naming every drifted role's BUNDLE hash is the migration authorisation.
    #[test]
    fn drift_with_full_intent_migrates() {
        let action = decide_drift_action(
            &installed_app(&[("lamad", "uhC0k-old"), ("imagodei", "uhC0k-i-old")], true),
            &bundle_roles(&[("lamad", "uhC0k-new"), ("imagodei", "uhC0k-i-new")]),
            &gates(true, ForceMode::Off, &["uhC0k-new", "uhC0k-i-new"]),
        );
        match &action {
            DriftAction::ReinstallForMigration { roles } => {
                let named: Vec<&str> = roles.iter().map(|r| r.role.as_str()).collect();
                assert_eq!(
                    named,
                    vec!["imagodei", "lamad"],
                    "both drifted roles carried"
                );
            }
            other => {
                panic!("intent covering every drifted role authorises the migration: {other:?}")
            }
        }
        assert!(action.reinstalls());
    }

    /// Intent is all-or-nothing: a partial list is an operator who did not see
    /// one of the roles move, which is exactly the wave-4 shape.
    #[test]
    fn drift_with_partial_intent_keeps_and_names_the_missing_role() {
        let action = decide_drift_action(
            &installed_app(&[("lamad", "uhC0k-old"), ("mishpat", "uhC0k-m-old")], true),
            &bundle_roles(&[("lamad", "uhC0k-new"), ("mishpat", "uhC0k-m-new")]),
            &gates(true, ForceMode::Off, &["uhC0k-new"]),
        );
        match &action {
            DriftAction::KeepInstalled {
                drifted,
                missing_intent,
            } => {
                assert_eq!(drifted.len(), 2);
                assert_eq!(missing_intent.len(), 1);
                assert_eq!(missing_intent[0].role, "mishpat");
                assert_eq!(
                    missing_intent[0].bundle_dna_hash, "uhC0k-m-new",
                    "the refusal must name the hash the operator has to add"
                );
            }
            other => panic!("a partial intent must not migrate: {other:?}"),
        }
    }

    /// `FORCE_DNA_REINSTALL=true` is an operator hammer, not a consent to lose
    /// chains — on an app holding data it refuses without intent or `wipe`.
    #[test]
    fn force_without_intent_or_wipe_refuses_on_an_app_with_data() {
        let action = decide_drift_action(
            &installed_app(&[("lamad", "uhC0k-old")], true),
            &bundle_roles(&[("lamad", "uhC0k-new")]),
            &gates(true, ForceMode::On, &[]),
        );
        assert!(
            matches!(action, DriftAction::RefuseForceWithoutIntent { .. }),
            "got {action:?}"
        );
        assert!(!action.reinstalls());
    }

    /// …and proceeds once the operator names the target.
    #[test]
    fn force_with_intent_migrates_an_app_with_data() {
        let action = decide_drift_action(
            &installed_app(&[("lamad", "uhC0k-old")], true),
            &bundle_roles(&[("lamad", "uhC0k-new")]),
            &gates(false, ForceMode::On, &["uhC0k-new"]),
        );
        assert!(
            matches!(action, DriftAction::ReinstallForMigration { .. }),
            "got {action:?}"
        );
    }

    /// The one spelling that accepts chain loss outright.
    #[test]
    fn force_wipe_reinstalls_an_app_with_data() {
        let action = decide_drift_action(
            &installed_app(&[("lamad", "uhC0k-old")], true),
            &bundle_roles(&[("lamad", "uhC0k-new")]),
            &gates(false, ForceMode::Wipe, &[]),
        );
        assert_eq!(action, DriftAction::ReinstallForced);
        assert!(action.reinstalls());
    }

    /// Nothing to lose (no provisioned cells / genesis never completed) — the
    /// ephemeral re-seeded-env behaviour the standing flag was written for
    /// survives untouched.
    #[test]
    fn reinstalls_freely_when_the_app_holds_no_data() {
        let no_data = installed_app(&[("lamad", "uhC0k-old")], false);
        let bundle = bundle_roles(&[("lamad", "uhC0k-new")]);

        assert_eq!(
            decide_drift_action(&no_data, &bundle, &gates(true, ForceMode::Off, &[])),
            DriftAction::ReinstallForced,
            "ALLOW_DNA_REINSTALL still reinstalls a data-less app on drift"
        );
        assert_eq!(
            decide_drift_action(&no_data, &bundle, &gates(false, ForceMode::On, &[])),
            DriftAction::ReinstallForced,
            "FORCE needs no intent when there are no chains to destroy"
        );
    }

    /// Without any flag and without intent, a data-less drifted app is still
    /// left alone — the gate never widens on the way to the safe case.
    #[test]
    fn no_flags_keeps_installed_even_without_data() {
        let action = decide_drift_action(
            &installed_app(&[("lamad", "uhC0k-old")], false),
            &bundle_roles(&[("lamad", "uhC0k-new")]),
            &gates(false, ForceMode::Off, &[]),
        );
        assert!(
            matches!(action, DriftAction::KeepInstalled { .. }),
            "got {action:?}"
        );
    }

    /// No drift: no uninstall, whatever the flags say (short of the hammer).
    #[test]
    fn no_drift_is_a_no_op() {
        let same = &[("lamad", "uhC0k-same"), ("imagodei", "uhC0k-i")];
        assert_eq!(
            decide_drift_action(
                &installed_app(same, true),
                &bundle_roles(same),
                &gates(true, ForceMode::Off, &[]),
            ),
            DriftAction::NoOp
        );
        assert_eq!(
            decide_drift_action(
                &installed_app(same, true),
                &bundle_roles(same),
                &gates(true, ForceMode::Off, &["uhC0k-same"]),
            ),
            DriftAction::NoOp,
            "a declared intent that changes nothing must not churn the app"
        );
    }

    /// A role present in the bundle but absent from the installed app is NOT
    /// content drift (there is no installed hash to compare) — structural
    /// staleness owns that case.
    #[test]
    fn a_role_missing_from_the_installed_app_is_not_content_drift() {
        let action = decide_drift_action(
            &installed_app(&[("lamad", "uhC0k-same")], true),
            &bundle_roles(&[("lamad", "uhC0k-same"), ("mishpat", "uhC0k-m")]),
            &gates(true, ForceMode::Off, &[]),
        );
        assert_eq!(action, DriftAction::NoOp);
    }

    /// Structural staleness on an app holding data is NOT a licence to
    /// uninstall: repairing one missing role would delete the source chains of
    /// every role that was still healthy.
    #[test]
    fn stale_with_data_keeps_installed_without_intent() {
        let action = decide_drift_action(
            &stale_app(&[("lamad", "uhC0k-same")], true, &["mishpat"]),
            &bundle_roles(&[("lamad", "uhC0k-same"), ("mishpat", "uhC0k-m")]),
            &gates(true, ForceMode::Off, &[]),
        );
        match &action {
            DriftAction::RefuseStaleWithoutIntent { missing_roles } => {
                assert_eq!(missing_roles, &vec!["mishpat".to_string()]);
            }
            other => panic!("stale + data + no intent must keep the installed cells: {other:?}"),
        }
        assert!(!action.reinstalls());
    }

    /// …and an intent naming the missing roles' BUNDLE hashes repairs it. A
    /// partial intent does not.
    #[test]
    fn stale_with_data_reinstalls_under_intent() {
        let app = stale_app(&[("lamad", "uhC0k-same")], true, &["mishpat", "imagodei"]);
        let bundle = bundle_roles(&[
            ("lamad", "uhC0k-same"),
            ("mishpat", "uhC0k-m"),
            ("imagodei", "uhC0k-i"),
        ]);

        assert_eq!(
            decide_drift_action(
                &app,
                &bundle,
                &gates(false, ForceMode::Off, &["uhC0k-m", "uhC0k-i"]),
            ),
            DriftAction::ReinstallStale,
            "intent covering every missing role authorises the repair"
        );
        assert!(
            matches!(
                decide_drift_action(&app, &bundle, &gates(false, ForceMode::Off, &["uhC0k-m"])),
                DriftAction::RefuseStaleWithoutIntent { .. }
            ),
            "a partial intent must not uninstall"
        );
        assert_eq!(
            decide_drift_action(&app, &bundle, &gates(false, ForceMode::Wipe, &[])),
            DriftAction::ReinstallForced,
            "the wipe hammer is the other way through"
        );
    }

    /// No chains at risk — staleness reinstalls as it always did.
    #[test]
    fn stale_without_data_reinstalls() {
        assert_eq!(
            decide_drift_action(
                &stale_app(&[], false, &["lamad"]),
                &bundle_roles(&[("lamad", "uhC0k-l")]),
                &gates(false, ForceMode::Off, &[]),
            ),
            DriftAction::ReinstallStale
        );
    }

    /// A missing role whose bundle hash is unknown (absent from the bundle too)
    /// can never be "covered" — the refusal stands rather than guessing.
    #[test]
    fn stale_role_absent_from_the_bundle_is_never_covered_by_intent() {
        assert!(matches!(
            decide_drift_action(
                &stale_app(&[("lamad", "uhC0k-same")], true, &["node_registry"]),
                &bundle_roles(&[("lamad", "uhC0k-same")]),
                &gates(true, ForceMode::Off, &["uhC0k-anything"]),
            ),
            DriftAction::RefuseStaleWithoutIntent { .. }
        ));
    }

    /// A typo must never arm a destructive path.
    #[test]
    fn force_mode_parses_only_exact_spellings() {
        assert_eq!(parse_force_mode(None), ForceMode::Off);
        assert_eq!(parse_force_mode(Some("")), ForceMode::Off);
        assert_eq!(parse_force_mode(Some("yes")), ForceMode::Off);
        assert_eq!(parse_force_mode(Some("1")), ForceMode::Off);
        assert_eq!(parse_force_mode(Some("WIPE ")), ForceMode::Wipe);
        assert_eq!(parse_force_mode(Some(" True")), ForceMode::On);
    }

    #[test]
    fn migration_intent_parses_comma_separated_hashes() {
        assert!(parse_migration_intent(None).is_empty());
        assert!(parse_migration_intent(Some("  ")).is_empty());
        assert!(parse_migration_intent(Some(",,")).is_empty());
        let set = parse_migration_intent(Some("uhC0k-a, uhC0k-b ,,uhC0k-c"));
        assert_eq!(set.len(), 3);
        assert!(
            set.contains("uhC0k-b"),
            "whitespace around a hash is tolerated"
        );
    }

    /// Only the three reinstalling verdicts may reach `uninstall_app`.
    #[test]
    fn only_reinstalling_verdicts_report_reinstalls() {
        assert!(DriftAction::ReinstallStale.reinstalls());
        assert!(DriftAction::ReinstallForced.reinstalls());
        assert!(DriftAction::ReinstallForMigration { roles: vec![] }.reinstalls());
        assert!(!DriftAction::NoOp.reinstalls());
        assert!(!DriftAction::KeepInstalled {
            drifted: vec![],
            missing_intent: vec![]
        }
        .reinstalls());
        assert!(!DriftAction::RefuseForceWithoutIntent { drifted: vec![] }.reinstalls());
        assert!(!DriftAction::RefuseStaleWithoutIntent {
            missing_roles: vec![]
        }
        .reinstalls());
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
                coordinator_wasm_hash(zome_def).map(|h| (name.to_string(), h.to_string()))
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
    /// Diagnostic (ignored): print the per-role DNA hashes storage resolves from a bundle, so an
    /// installed-vs-bundle mismatch can be attributed to the resolver or to the install path.
    /// Run: ELOHIM_HAPP_PROBE=<path.happ> cargo test -p elohim-storage --lib happ_manager::tests::probe_bundle_dna_hashes -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "diagnostic: set ELOHIM_HAPP_PROBE to a .happ path"]
    async fn probe_bundle_dna_hashes() {
        let Ok(path) = std::env::var("ELOHIM_HAPP_PROBE") else {
            return;
        };
        let hashes = bundle_dna_hashes(std::path::Path::new(&path))
            .await
            .expect("resolve");
        for (role, h) in &hashes {
            eprintln!("[probe] bundle_dna_hashes {role} = {h}");
        }
        let files = bundle_role_dna_files(std::path::Path::new(&path))
            .await
            .expect("resolve files");
        for (role, f) in &files {
            eprintln!("[probe] bundle_role_dna_files {role} = {}", f.dna_hash());
        }
    }

    #[test]
    fn lineage_app_id_takes_12_chars_after_the_uhc0k_prefix() {
        let hash = "uhC0kyvKwO2J5u3mf52tjASWe0ryhdpNYalrSeMGJODF3OpUxyeoH";
        assert_eq!(lineage_app_id("elohim", hash), "elohim@yvKwO2J5u3mf");
    }
}
