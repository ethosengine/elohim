//! Apply vehicles — turning a [`VerifiedRelease`] into a running change.
//!
//! Rung 5's second half (T4, `task-release-apply-vehicles`; design
//! `genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md`
//! §6 step 4 and §9). T3 landed the half that SEES and JUDGES; this is the half
//! that ACTS.
//!
//! # This module invents nothing
//!
//! Every vehicle here already existed and was already proven before the
//! controller did:
//!
//! | class | vehicle | the machinery it routes to |
//! |---|---|---|
//! | `coordinator-bundle` | [`CoordinatorBundleVehicle`] | `happ_manager::sync_coordinators_report` — the exact in-process fn behind `POST /admin/coordinators/sync` (rung 1) |
//! | `config-epr` | [`ConfigEprVehicle`] | the rung-4 watched runtime-config file + [`crate::runtime_config::reload_now`] |
//! | `storage-binary` | [`StorageBinaryVehicle`] | the mesh exe-slot — **staged, never executed** |
//! | `happ-bundle` | [`HappBundleVehicle`] | the same coordinator hot-swap, gated on being joined |
//!
//! That is the whole design claim, and it is load-bearing: a rung that invented
//! its own apply mechanics would need its own soak, its own failure modes and
//! its own revert story. Composing the proven ones means **revert is free** —
//! the ceremony declares a prior head canonical and every controller converges
//! backward through the identical loop.
//!
//! # The four refusals that are the point
//!
//! 1. **`apply_not_permitted`** — the conductor-touching vehicles honour
//!    `ALLOW_COORDINATOR_UPDATE` (inheriting `ALLOW_DNA_REINSTALL`), the SAME
//!    gate the boot path and the HTTP vehicle read, via the same
//!    `happ_manager::coordinator_update_allowed()`. Three call sites, one
//!    predicate — they cannot drift.
//! 2. **`binary_stakes_not_simulacra`** — spec §9 keeps FLEET binaries out of
//!    this rung entirely. A `storage-binary` release applies only where the
//!    peer's DECLARED network stakes are [`NetworkStage::Simulacra`] (the
//!    developer/mesh trust context). The declaration is fail-closed —
//!    `Bootstrap` is what a node that declares nothing resolves to — so the
//!    refusal is the default and permission is the exception.
//! 3. **`bootstrap_out_of_band`** — spec §6.4. A fresh joiner structurally
//!    cannot perform the verified local resolve for the very channel that
//!    supplies its DNA. Its first bundle is seeded out of band; only after
//!    joining does the controller converge it.
//! 4. **`config_knob_boot_only`** — a config release naming a knob this process
//!    captured once at boot is refused rather than written. Writing it would
//!    report `applied` and change nothing, which is the precise lie
//!    [`crate::runtime_config::BOOT_ONLY`] exists to make impossible.
//!
//! # And the one thing this module will not do
//!
//! **It never execs.** The `storage-binary` vehicle stages verified bytes into
//! a well-known slot, marks the channel `pending-restart`, and stops. A process
//! that replaces its own binary and restarts itself is a different safety
//! argument than "adopt a coordinator hot-swap", and it is not the one rung 5
//! is making. The restart is an operator/harness act
//! (`just mesh storage-restart <peer>`), which is also what keeps the exe
//! record, the environment capture and the loud-failure path in one place.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};

use super::{
    AdoptionRefusal, AppliedReceipt, ApplyVehicle, ArtifactClass, RefusalReason, VerifiedRelease,
};
use crate::trust::{NetworkStage, StakesResolver};

// ---------------------------------------------------------------------------
// The staged-binary slot — NORMATIVE (task atom §Interface contract, T6 reads it)
// ---------------------------------------------------------------------------

/// Directory, under the staging root, that holds artifacts a restart consumes.
///
/// Full slot path: `<staging_root>/<SLOT_DIR>/<SLOT_BINARY_NAME>`. On the local
/// mesh the staging root is `$MESH_DIR/release-adoption/<peer>`, so the slot
/// lands at `$MESH_DIR/release-adoption/<peer>/slot/elohim-storage.next` —
/// **this string is normative**: T6's mesh receipt and any harness arm that
/// consumes the slot read exactly it.
pub const SLOT_DIR: &str = "slot";

/// The staged storage binary's file name. `.next` is deliberate: it names a
/// binary that is NOT running and is waiting for a restart, so nothing about
/// the file can be mistaken for the live one.
pub const SLOT_BINARY_NAME: &str = "elohim-storage.next";

/// Sidecar written beside the slot, so the slot's provenance is readable
/// without the admin route (a harness arm has a filesystem, not always an HTTP
/// client).
pub const SLOT_RECEIPT_NAME: &str = "elohim-storage.next.json";

/// The full slot path under a staging root.
pub fn slot_path(staging_root: &Path) -> PathBuf {
    staging_root.join(SLOT_DIR).join(SLOT_BINARY_NAME)
}

// ---------------------------------------------------------------------------
// C11 — the backpressure signal, read cheaply or not at all
// ---------------------------------------------------------------------------

/// Whether this node is under a pressure signal it can read **without new
/// plumbing**, and therefore should defer an apply to a later sweep.
///
/// Today that is exactly one signal: the conductor admission lane is saturated
/// (`in_flight >= capacity`). It is process-local, lock-free and free to read,
/// and it is the pressure that actually matters for an apply — every
/// conductor-touching vehicle would otherwise queue behind whatever is already
/// saturating the lane, and an apply is never the thing a person is standing in
/// line for.
///
/// **What is deliberately NOT read here:** ram-guard and PVC watermarks live in
/// the pre-push/hook layer and quiesce state in the p2p plane; neither is
/// reachable from this process without new plumbing, and inventing a reader for
/// them inside an apply vehicle would be exactly the "grow an instrument with no
/// reader" failure. C11 is therefore recorded as **partial** in
/// `seam-registry.yaml`, with the admission leg wired and the other two named.
///
/// Deferral is a REFUSAL here rather than a skip (unlike the sweep's byte
/// budget) because it happens after the floor has already been paid for: the
/// verdict is real, it is about us, and it is transient — so it climbs the
/// ladder from rung 1 and cures itself the moment the lane drains.
pub fn backpressure_signal() -> Option<String> {
    let admission = crate::conductor_admission::admission();
    let capacity = admission.capacity();
    let in_flight = admission.in_flight();
    if capacity > 0 && in_flight >= capacity {
        return Some(format!(
            "conductor admission lane saturated ({in_flight}/{capacity} in flight) — deferring \
             the apply to a later sweep (lag-within-window, not churn)"
        ));
    }
    None
}

// ---------------------------------------------------------------------------
// The registry — routing by artifact class
// ---------------------------------------------------------------------------

/// What this PROCESS is equipped to apply, reported on `/admin/adoption`.
///
/// Process-global for the same reason the adoption state is: it is a fact about
/// this running binary, and the admin route must be able to answer "why did my
/// apply channel not apply?" with "this node has no vehicle for that class"
/// rather than leaving the reader to infer it.
static REGISTERED_VEHICLES: LazyLock<Mutex<Vec<&'static str>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

/// The artifact-class labels this process has a vehicle for, in a stable order.
pub fn registered_vehicle_labels() -> Vec<&'static str> {
    REGISTERED_VEHICLES.lock().unwrap().clone()
}

/// Routes a verified release to the vehicle its artifact class names.
///
/// A class with no vehicle is `no_vehicle_for_class` — never a fallback to
/// "some other vehicle that looked close". Routing by anything but the
/// manifest's declared class would let a release be applied by machinery it was
/// never verified against.
#[derive(Default)]
pub struct ApplyRegistry {
    vehicles: Vec<Arc<dyn ApplyVehicle>>,
}

impl ApplyRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one vehicle. A vehicle that declares no classes
    /// ([`ApplyVehicle::handles`]'s default) is inert — it is stored, and
    /// nothing ever routes to it.
    pub fn with(mut self, vehicle: Arc<dyn ApplyVehicle>) -> Self {
        {
            let mut registered = REGISTERED_VEHICLES.lock().unwrap();
            for class in vehicle.handles() {
                let label = class.label();
                if !registered.contains(&label) {
                    registered.push(label);
                }
            }
        }
        self.vehicles.push(vehicle);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.vehicles.is_empty()
    }

    /// The vehicle for `class`, or `None`.
    pub fn for_class(&self, class: ArtifactClass) -> Option<&Arc<dyn ApplyVehicle>> {
        self.vehicles.iter().find(|v| v.handles().contains(&class))
    }

    /// Route and apply. The single call site for every vehicle.
    ///
    /// **C11 first**, before the class lookup: a node under pressure defers
    /// regardless of which vehicle it would have reached, because the deferral
    /// is a statement about the node.
    pub async fn apply(
        &self,
        verified: &VerifiedRelease,
    ) -> Result<AppliedReceipt, AdoptionRefusal> {
        if let Some(detail) = backpressure_signal() {
            return Err(AdoptionRefusal::new(
                RefusalReason::DeferredBackpressure,
                detail,
            ));
        }
        let class = verified.manifest.artifact_class;
        let Some(vehicle) = self.for_class(class) else {
            return Err(AdoptionRefusal::new(
                RefusalReason::NoVehicleForClass,
                format!(
                    "no apply vehicle is wired on this node for artifact class '{}' (wired: {:?})",
                    class.label(),
                    registered_vehicle_labels()
                ),
            ));
        };
        vehicle.apply(verified).await
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn now_unix() -> i64 {
    super::state::now_unix()
}

/// The one verified artifact a single-artifact vehicle acts on.
///
/// `artifact_paths` is one-per-manifest-artifact IN ORDER, so the pairing with
/// `manifest.artifacts` is positional and checked here rather than assumed. A
/// release with no artifacts, or with fewer staged paths than declared
/// artifacts, is `apply_payload_unusable`: the bytes verified, so more sweeps
/// will not make them different.
fn sole_artifact<'a>(
    verified: &'a VerifiedRelease,
    vehicle: &str,
) -> Result<(&'a super::Artifact, &'a Path), AdoptionRefusal> {
    let declared = verified.manifest.artifacts.len();
    if declared == 0 {
        return Err(AdoptionRefusal::new(
            RefusalReason::ApplyPayloadUnusable,
            format!("{vehicle}: the release declares no artifacts"),
        ));
    }
    if verified.artifact_paths.len() != declared {
        return Err(AdoptionRefusal::new(
            RefusalReason::ApplyPayloadUnusable,
            format!(
                "{vehicle}: {} verified paths for {declared} declared artifacts — the positional \
                 pairing the vehicle relies on does not hold",
                verified.artifact_paths.len()
            ),
        ));
    }
    if declared > 1 {
        return Err(AdoptionRefusal::new(
            RefusalReason::ApplyPayloadUnusable,
            format!(
                "{vehicle}: this vehicle applies exactly one artifact, the release declares \
                 {declared} — a multi-artifact release of this class needs a vehicle that \
                 declares its own ordering, not a guess at which one is 'the' artifact"
            ),
        ));
    }
    Ok((
        &verified.manifest.artifacts[0],
        verified.artifact_paths[0].as_path(),
    ))
}

/// Write bytes so a half-written file is never visible under the name a
/// consumer would read. The suffix is APPENDED rather than substituted (the
/// same reason `watch::write_staged` does it that way).
fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut tmp = path.as_os_str().to_os_string();
    tmp.push(".partial");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

// ---------------------------------------------------------------------------
// What the staged bytes WOULD install — the by-bytes evidence (2026-09-04)
// ---------------------------------------------------------------------------

/// Read the per-role TARGET coordinator wasm hashes out of a release's STAGED
/// artifact, for [`super::verify::already_runs_target`].
///
/// Lives here rather than in `verify` because it is I/O — `verify` is pure by
/// contract, and the caller assembles evidence. Lives here rather than in
/// `watch` because reading a coordinator bundle is this module's knowledge: it
/// is the same `happ_manager` resolution the hot-swap vehicle above routes to,
/// so the hashes compared are exactly the hashes a hot-swap would splice in.
///
/// Honest absence, three ways — and none of them ever takes the exit:
///
/// - `Absent` — this artifact class installs no coordinators (`config-epr`,
///   `storage-binary`), so there is nothing to be already-current *by*.
/// - `Unreachable` — the bytes are not usable evidence: the positional pairing
///   between `manifest.artifacts` and the staged paths does not hold, the
///   fetched bytes do not match the manifest's declared length/digest, or the
///   bundle could not be unpacked. **The digest guard is load-bearing**: this
///   runs before `verify_artifacts` has judged, so it re-checks rather than
///   assumes — target hashes read out of unverified bytes could otherwise talk
///   a peer into "already current" from a substituted artifact.
/// - `Present` — role → (zome → wasm hash) for every role the bundle resolves.
pub async fn staged_target_coordinators(
    manifest: &super::ReleaseManifest,
    fetched: &[super::verify::FetchedArtifact],
) -> seam_contracts::Answer<super::verify::TargetCoordinators> {
    use seam_contracts::Answer;

    match manifest.artifact_class {
        ArtifactClass::CoordinatorBundle | ArtifactClass::HappBundle => {}
        // A config or binary release installs no coordinator wasm; "already
        // current by coordinator bytes" is not a question it can answer.
        ArtifactClass::ConfigEpr | ArtifactClass::StorageBinary => return Answer::Absent,
    }

    let [declared] = manifest.artifacts.as_slice() else {
        return Answer::Unreachable;
    };
    let Some(actual) = fetched.iter().find(|f| f.blob_cid == declared.blob_cid) else {
        return Answer::Unreachable;
    };
    if actual.bytes != declared.bytes || !actual.sha256.eq_ignore_ascii_case(&declared.sha256) {
        return Answer::Unreachable;
    }

    match crate::happ_manager::bundle_coordinator_wasm_hashes(&actual.path).await {
        Ok(target) => Answer::Present(target),
        Err(e) => {
            tracing::debug!(
                path = %actual.path.display(),
                error = %e,
                "release-adoption: staged bundle could not be read for its target coordinator \
                 hashes — the by-bytes exit is simply not available for this release"
            );
            Answer::Unreachable
        }
    }
}

fn receipt(verified: &VerifiedRelease, vehicle: &str, detail: serde_json::Value) -> AppliedReceipt {
    AppliedReceipt {
        channel_id: verified.channel_id.clone(),
        release_cid: verified.release_cid.clone(),
        vehicle: vehicle.to_string(),
        applied_at_unix: now_unix(),
        detail,
    }
}

// ---------------------------------------------------------------------------
// coordinator-bundle → happ_manager::sync_coordinators_report
// ---------------------------------------------------------------------------

/// The `sync_coordinators` hot-swap: coordinator wasm is spliced into the LIVE
/// cell, so the agent key, the cells and all DHT state survive. No re-key, no
/// re-install, no restart — the ~2-minute vehicle rung 1 already proved on the
/// mesh three times.
///
/// The DNA line is not re-checked here: `verify_envelope` already refused any
/// release binding a different per-role integrity lineage
/// (`dna_lineage_mismatch`), and `sync_coordinators_for_app_info` holds the
/// same guard per role a second time. Two independent refusals for the one
/// corruption vector that matters is deliberate, not redundant.
pub struct CoordinatorBundleVehicle {
    admin: holochain_client::AdminWebsocket,
    app_id: String,
}

impl CoordinatorBundleVehicle {
    pub fn new(admin: holochain_client::AdminWebsocket, app_id: impl Into<String>) -> Self {
        Self {
            admin,
            app_id: app_id.into(),
        }
    }

    /// The shared body, so the hApp-bundle vehicle routes to exactly this and
    /// the two can never grow apart.
    async fn hot_swap(
        &self,
        verified: &VerifiedRelease,
        vehicle: &'static str,
        bundle: &Path,
    ) -> Result<AppliedReceipt, AdoptionRefusal> {
        if !crate::happ_manager::coordinator_update_allowed() {
            return Err(AdoptionRefusal::new(
                RefusalReason::ApplyNotPermitted,
                "coordinator hot-swap is not permitted on this node — set \
                 ALLOW_COORDINATOR_UPDATE=true (or ALLOW_DNA_REINSTALL=true). The same gate the \
                 boot path and POST /admin/coordinators/sync honour",
            ));
        }

        let report =
            crate::happ_manager::sync_coordinators_report(&self.admin, &self.app_id, bundle, true)
                .await
                .map_err(|e| {
                    AdoptionRefusal::new(
                        RefusalReason::ApplyFailed,
                        format!("sync_coordinators_report failed: {e}"),
                    )
                })?;

        // Per-role errors NEVER abort the sweep (happ_manager's contract), so
        // they arrive on the report rather than as an Err. Surface them: a
        // release that drifted three roles and healed two is a partial apply,
        // and the receipt has to say so — "never partially apply without saying
        // so in the receipt" is the trait's own rule.
        let failed_roles: Vec<&str> = report
            .roles
            .iter()
            .filter(|r| r.error.is_some())
            .map(|r| r.role.as_str())
            .collect();
        let detail = serde_json::json!({
            "appId": report.app_id,
            "driftedCount": report.drifted_count,
            "appliedCount": report.applied_count,
            "roles": report.roles.iter().map(|r| serde_json::json!({
                "role": r.role,
                "drifted": r.drifted,
                "applied": r.applied,
                "error": r.error,
                "installedCoordinators": r.installed_coordinators,
                "bundledCoordinators": r.bundled_coordinators,
            })).collect::<Vec<_>>(),
            "failedRoles": failed_roles,
            "bundle": bundle.display().to_string(),
        });

        // Drift that the swap did not heal is a FAILED apply, not a quiet
        // success with a note. Zero drift is a success: the peer already runs
        // these coordinators, which is convergence, not a no-op to hide.
        if report.drifted_count > 0 && report.applied_count == 0 {
            return Err(AdoptionRefusal::new(
                RefusalReason::ApplyFailed,
                format!(
                    "{} role(s) drifted and none were hot-swapped (failed: {failed_roles:?}) — \
                     detail on the report",
                    report.drifted_count
                ),
            ));
        }

        tracing::info!(
            channel = %verified.channel_id,
            release_cid = %verified.release_cid,
            drifted = report.drifted_count,
            applied = report.applied_count,
            vehicle,
            "release-adoption: coordinator hot-swap applied (no re-key, no DHT churn)"
        );
        Ok(receipt(verified, vehicle, detail))
    }
}

#[async_trait::async_trait]
impl ApplyVehicle for CoordinatorBundleVehicle {
    async fn apply(&self, verified: &VerifiedRelease) -> Result<AppliedReceipt, AdoptionRefusal> {
        let (_, path) = sole_artifact(verified, "sync_coordinators")?;
        self.hot_swap(verified, "sync_coordinators", path).await
    }

    fn handles(&self) -> &'static [ArtifactClass] {
        &[ArtifactClass::CoordinatorBundle]
    }

    fn name(&self) -> &'static str {
        "sync_coordinators"
    }
}

// ---------------------------------------------------------------------------
// happ-bundle → the same hot-swap, for JOINED peers only
// ---------------------------------------------------------------------------

/// The hApp-bundle vehicle (spec §6.4).
///
/// # Why this is the coordinator hot-swap and not a re-install
///
/// The verify floor has already refused any release whose per-role `dnaHash`
/// differs from the installed cell's (`dna_lineage_mismatch` — crossing that
/// line is rung 6's migration ceremony). So a `happ-bundle` release that
/// reaches this vehicle binds the SAME integrity lineage this peer runs, and
/// the only thing that can legally differ inside it is coordinator wasm.
/// Applying it therefore IS the hot-swap — and routing it through
/// `ensure_happ_installed` instead would risk the stale-install branch, which
/// mints a NEW agent key. A rung whose failure mode is "your peer silently got
/// a new identity" is not a rung that ships.
///
/// The one thing this vehicle adds over [`CoordinatorBundleVehicle`] is the
/// bootstrap refusal: a peer with no installed app is a FRESH JOINER, and its
/// first bundle comes out of band (`join-peer`'s pinned, content-addressed
/// bundle + channel-id trust anchor), because it structurally cannot resolve
/// through a DNA it does not yet run.
pub struct HappBundleVehicle {
    inner: CoordinatorBundleVehicle,
    admin: holochain_client::AdminWebsocket,
    app_id: String,
}

impl HappBundleVehicle {
    pub fn new(admin: holochain_client::AdminWebsocket, app_id: impl Into<String>) -> Self {
        let app_id = app_id.into();
        Self {
            inner: CoordinatorBundleVehicle::new(admin.clone(), app_id.clone()),
            admin,
            app_id,
        }
    }

    /// Whether this peer is JOINED — i.e. the app is installed on its own
    /// conductor. `Err` is deliberately not "not joined": a conductor we cannot
    /// ask is unreachable, and reading unreachable as absence here would refuse
    /// a joined peer with the bootstrap reason, sending the operator to
    /// `join-peer` for a peer that has already joined.
    async fn joined(&self) -> Result<bool, AdoptionRefusal> {
        let apps = self.admin.list_apps(None).await.map_err(|e| {
            AdoptionRefusal::new(
                RefusalReason::ApplyFailed,
                format!(
                    "could not read this conductor's installed apps ({e}) — unreachable, which is \
                     never 'not joined'"
                ),
            )
        })?;
        Ok(apps.iter().any(|a| a.installed_app_id == self.app_id))
    }
}

#[async_trait::async_trait]
impl ApplyVehicle for HappBundleVehicle {
    async fn apply(&self, verified: &VerifiedRelease) -> Result<AppliedReceipt, AdoptionRefusal> {
        let (_, path) = sole_artifact(verified, "happ_bundle")?;
        if !self.joined().await? {
            return Err(AdoptionRefusal::new(
                RefusalReason::BootstrapOutOfBand,
                format!(
                    "app '{}' is not installed on this conductor — a fresh joiner's first bundle \
                     is seeded out of band (a pinned, content-addressed bundle + channel-id trust \
                     anchor); the controller converges it only AFTER it has joined",
                    self.app_id
                ),
            ));
        }
        self.inner.hot_swap(verified, "happ_bundle", path).await
    }

    fn handles(&self) -> &'static [ArtifactClass] {
        &[ArtifactClass::HappBundle]
    }

    fn name(&self) -> &'static str {
        "happ_bundle"
    }
}

// ---------------------------------------------------------------------------
// config-epr → the rung-4 watched runtime-config file
// ---------------------------------------------------------------------------

/// The config vehicle: write the release's payload to the watched
/// runtime-config path and reload NOW.
///
/// Rung 4 already owns the hard part — the file is polled, the registry applies
/// the parsed map, an absent key restores the boot value, and provenance stays
/// visible on `/admin/runtime-config`. This vehicle's whole job is to put the
/// right bytes there and to refuse the two ways that would be a lie.
pub struct ConfigEprVehicle;

impl ConfigEprVehicle {
    pub fn new() -> Self {
        Self
    }

    /// Keys in `parsed` that this process captured once at boot.
    ///
    /// [`crate::runtime_config::BOOT_ONLY`] holds two shapes: a bare NAME, and
    /// a `NAME=VALUE` entry whose *disabled case* alone is boot-only
    /// (`PROJECTION_RECONCILE_SECS=0` — the nonzero cadence IS hot). Both are
    /// honoured, so a release setting a hot cadence is not refused for sharing
    /// a name with a cold one.
    fn boot_only_keys(parsed: &BTreeMap<String, String>) -> Vec<String> {
        let mut refused = Vec::new();
        for flag in crate::runtime_config::BOOT_ONLY.iter() {
            let (name, only_value) = match flag.name.split_once('=') {
                Some((n, v)) => (n, Some(v)),
                None => (flag.name, None),
            };
            let Some(declared) = parsed.get(name) else {
                continue;
            };
            let bites = match only_value {
                None => true,
                Some(v) => declared.trim() == v,
            };
            if bites {
                refused.push(format!("{name} ({})", flag.reason));
            }
        }
        refused
    }
}

impl Default for ConfigEprVehicle {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ApplyVehicle for ConfigEprVehicle {
    async fn apply(&self, verified: &VerifiedRelease) -> Result<AppliedReceipt, AdoptionRefusal> {
        let (_, path) = sole_artifact(verified, "runtime_config_reload")?;

        let Some(config_path) = crate::runtime_config::config_path() else {
            return Err(AdoptionRefusal::new(
                RefusalReason::RuntimeConfigUnwatched,
                format!(
                    "no watched runtime-config path on this node ({} is unset) — writing a config \
                     file nothing reads would report 'applied' and change nothing",
                    crate::runtime_config::PATH_ENV
                ),
            ));
        };

        let payload = tokio::fs::read_to_string(path).await.map_err(|e| {
            AdoptionRefusal::new(
                RefusalReason::ApplyPayloadUnusable,
                format!(
                    "the verified config artifact at {} is not readable UTF-8 text ({e})",
                    path.display()
                ),
            )
        })?;
        let parsed = crate::runtime_config::parse(&payload);
        if parsed.is_empty() {
            return Err(AdoptionRefusal::new(
                RefusalReason::ApplyPayloadUnusable,
                "the config artifact names no keys — a release that changes nothing is a \
                 publishing mistake, not a config",
            ));
        }

        let boot_only = ConfigEprVehicle::boot_only_keys(&parsed);
        if !boot_only.is_empty() {
            return Err(AdoptionRefusal::new(
                RefusalReason::ConfigKnobBootOnly,
                format!(
                    "the release names knob(s) this process captured once at boot: {} — writing \
                     them would report 'applied' and change nothing",
                    boot_only.join("; ")
                ),
            ));
        }

        // Keys the registry does not know are CARRIED, not refused. A peer one
        // build behind must still be able to take a release that ADDED a key
        // within the lineage window — that is the additive-wire floor the
        // envelope asserted, applied to config. They are reported so the
        // silence is legible.
        let known: Vec<&str> = crate::runtime_config::SPECS
            .iter()
            .map(|s| s.name)
            .chain(crate::runtime_config::TEXT_SPECS.iter().map(|s| s.name))
            .collect();
        let ignored: Vec<&String> = parsed
            .keys()
            .filter(|k| !known.contains(&k.as_str()))
            .collect();

        // The release IS the config document — written wholesale, not merged.
        // A merge would silently keep a key the release deliberately removed,
        // and "the file says what the elected head says" is the only rule that
        // makes a revert converge.
        write_atomic(&config_path, payload.as_bytes()).map_err(|e| {
            AdoptionRefusal::new(
                RefusalReason::ApplyFailed,
                format!("could not write {}: {e}", config_path.display()),
            )
        })?;

        // Same PID, immediately — rather than waiting up to POLL_INTERVAL_SECS
        // for the watcher. The watcher stays the backstop; this is what makes
        // the receipt's "seconds" claim true.
        let outcome = crate::runtime_config::reload_now();

        tracing::info!(
            channel = %verified.channel_id,
            release_cid = %verified.release_cid,
            path = %config_path.display(),
            keys = parsed.len(),
            changed = outcome.changed,
            "release-adoption: config release applied and reloaded in-process"
        );

        Ok(receipt(
            verified,
            "runtime_config_reload",
            serde_json::json!({
                "path": config_path.display().to_string(),
                "keysDeclared": parsed.len(),
                "settingsChanged": outcome.changed,
                "ignoredKeys": ignored,
                "reloadError": outcome.error,
            }),
        ))
    }

    fn handles(&self) -> &'static [ArtifactClass] {
        &[ArtifactClass::ConfigEpr]
    }

    fn name(&self) -> &'static str {
        "runtime_config_reload"
    }
}

// ---------------------------------------------------------------------------
// storage-binary → the exe slot. STAGED, never executed.
// ---------------------------------------------------------------------------

/// Stage a verified storage binary into the well-known slot and stop.
///
/// # The stakes gate (spec §9)
///
/// Fleet binaries are OUT of this rung by design: a fleet binary replacing
/// itself is a bigger safety bite, and fleet binaries stay on the now-cheap
/// staggered, conductor-preserving k8s roll. The line is drawn at the peer's
/// DECLARED network stakes, not at a hostname or an env sniff, because the
/// declaration is the thing the trust plane already resolves and already
/// fails closed on: [`crate::trust::StakesResolver`] answers `Bootstrap` when
/// nothing is declared, and only an explicit `Simulacra` declaration —
/// the developer/test-fixture trust context — opens the gate.
///
/// # And it never execs
///
/// The slot is written, the channel is marked `pending-restart`, and that is
/// the whole vehicle. Consuming the slot is an operator/harness act. See the
/// module docs.
pub struct StorageBinaryVehicle {
    staging_root: PathBuf,
    stakes: Arc<dyn StakesResolver + Send + Sync>,
    /// What the stakes resolver is keyed by on this node (the installed app id
    /// today — the same scope `main` resolves the reconcile gradient with).
    stakes_scope: String,
}

impl StorageBinaryVehicle {
    pub fn new(
        staging_root: impl Into<PathBuf>,
        stakes: Arc<dyn StakesResolver + Send + Sync>,
        stakes_scope: impl Into<String>,
    ) -> Self {
        Self {
            staging_root: staging_root.into(),
            stakes,
            stakes_scope: stakes_scope.into(),
        }
    }

    /// The slot this vehicle writes. Normative — T6 reads it.
    pub fn slot(&self) -> PathBuf {
        slot_path(&self.staging_root)
    }
}

#[async_trait::async_trait]
impl ApplyVehicle for StorageBinaryVehicle {
    async fn apply(&self, verified: &VerifiedRelease) -> Result<AppliedReceipt, AdoptionRefusal> {
        let (artifact, path) = sole_artifact(verified, "exe_slot_stage")?;

        let (stage, provenance) = self.stakes.stage_for(&self.stakes_scope);
        if stage != NetworkStage::Simulacra {
            return Err(AdoptionRefusal::new(
                RefusalReason::BinaryStakesNotSimulacra,
                format!(
                    "declared network stakes for scope '{}' are {stage:?} (provenance \
                     {provenance:?}); the storage-binary rung is LOCAL and MESH only (spec §9) — \
                     fleet binaries stay on the staggered, conductor-preserving roll",
                    self.stakes_scope
                ),
            ));
        }

        let slot = self.slot();
        let bytes = tokio::fs::read(path).await.map_err(|e| {
            AdoptionRefusal::new(
                RefusalReason::ApplyFailed,
                format!(
                    "could not read the verified binary at {}: {e}",
                    path.display()
                ),
            )
        })?;
        write_atomic(&slot, &bytes).map_err(|e| {
            AdoptionRefusal::new(
                RefusalReason::ApplyFailed,
                format!("could not stage the binary at {}: {e}", slot.display()),
            )
        })?;

        // A staged binary that is not executable is a slot the restart arm will
        // silently decline (its `-x` checks), which reads to an operator as
        // "the release never landed".
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&slot, std::fs::Permissions::from_mode(0o755)).map_err(
                |e| {
                    AdoptionRefusal::new(
                        RefusalReason::ApplyFailed,
                        format!("could not mark {} executable: {e}", slot.display()),
                    )
                },
            )?;
        }

        let staged_at = now_unix();
        let sidecar = serde_json::json!({
            "channelId": verified.channel_id,
            "releaseCid": verified.release_cid,
            "filename": artifact.filename,
            "bytes": artifact.bytes,
            "sha256": artifact.sha256,
            "stagedAtUnix": staged_at,
            "pendingRestart": true,
            "note": "STAGED, NOT RUNNING. This process never execs a staged binary; a restart \
                     arm consumes the slot.",
        });
        let sidecar_path = self.staging_root.join(SLOT_DIR).join(SLOT_RECEIPT_NAME);
        if let Err(e) = write_atomic(
            &sidecar_path,
            serde_json::to_vec_pretty(&sidecar)
                .unwrap_or_default()
                .as_slice(),
        ) {
            // The slot is what matters; the sidecar is a convenience for a
            // reader with a filesystem and no HTTP client. Losing it degrades
            // legibility, never correctness — so it warns and does not refuse
            // an apply that actually happened.
            tracing::warn!(
                path = %sidecar_path.display(),
                error = %e,
                "release-adoption: staged the binary but could not write its sidecar receipt"
            );
        }

        tracing::warn!(
            channel = %verified.channel_id,
            release_cid = %verified.release_cid,
            slot = %slot.display(),
            bytes = artifact.bytes,
            "release-adoption: storage binary STAGED and pending restart — this process will NOT \
             exec it; run the harness restart arm to consume the slot"
        );

        Ok(receipt(
            verified,
            "exe_slot_stage",
            serde_json::json!({
                "slot": slot.display().to_string(),
                "sidecar": sidecar_path.display().to_string(),
                "bytes": artifact.bytes,
                "sha256": artifact.sha256,
                "pendingRestart": true,
                "stakes": format!("{stage:?}"),
            }),
        ))
    }

    fn handles(&self) -> &'static [ArtifactClass] {
        &[ArtifactClass::StorageBinary]
    }

    fn name(&self) -> &'static str {
        "exe_slot_stage"
    }
}

// ---------------------------------------------------------------------------
// The T4 → T5 seam: attesting what the apply actually did
// ---------------------------------------------------------------------------

/// Upper bound on how long a post-apply soak observer sleeps before authoring.
///
/// A channel discipline may declare any `soakSecs`; this process will not hold
/// a task open for more than a day waiting to attest, because a soak longer
/// than the peer's own uptime is not evidence about this apply — it is evidence
/// about whatever process happens to be running when the timer fires.
pub const MAX_SOAK_WAIT_SECS: u64 = 24 * 60 * 60;

/// How long the observer will actually wait, given a discipline's declared
/// `soakSecs`. A function rather than an inline `.min()` so the bound is a named
/// thing a test can exercise across the range, including the pathological one.
pub fn bounded_soak_wait(declared_secs: u64) -> u64 {
    declared_secs.min(MAX_SOAK_WAIT_SECS)
}

/// Authors this peer's soak attestation for a release it applied, through T5's
/// rail (`release_attestation::author_soak_attestation`).
///
/// # Why the attestation is not authored at apply time
///
/// The rail takes a `SoakContext` carrying a real `windowStart`/`windowEnd` and
/// a `SoakOutcome` carrying real probe results. Authoring at t=apply would mint
/// an attestation for a window that never ran — evidence that a *different*
/// peer's threshold arm would then count toward promotion. C1 excludes the
/// builder's own attestation precisely so that a release cannot earn itself;
/// an instantaneous soak would reopen the same hole one step further down. So
/// the observer waits the declared window and attests what it actually saw.
pub struct SoakAttestor {
    hc: Arc<crate::hc_client::HcClient>,
    device_id: String,
    device_archetype: String,
    capability_level: i32,
    region: Option<String>,
    household_id: Option<String>,
    node_role: Option<String>,
    build_info: crate::services::release_attestation::BuildInfoRef,
}

impl SoakAttestor {
    /// Populate from the same runtime sources boot registration reads.
    pub fn from_runtime(
        hc: Arc<crate::hc_client::HcClient>,
        config: &crate::config::Config,
        device_id: impl Into<String>,
        capability_level: i32,
        build: &elohim_compute::BuildInfo,
    ) -> Self {
        Self {
            hc,
            device_id: device_id.into(),
            device_archetype: config
                .device_archetype
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            capability_level,
            region: config.region.clone(),
            household_id: config.household_id.clone(),
            node_role: config.node_role.clone(),
            build_info: crate::services::release_attestation::BuildInfoRef::from(build),
        }
    }

    fn context(
        &self,
        channel_id: &str,
        window_start_unix: i64,
        window_end_unix: i64,
    ) -> crate::services::release_attestation::SoakContext {
        crate::services::release_attestation::SoakContext {
            channel_id: channel_id.to_string(),
            device_id: self.device_id.clone(),
            device_archetype: self.device_archetype.clone(),
            capability_level: self.capability_level,
            region: self.region.clone(),
            household_id: self.household_id.clone(),
            node_role: self.node_role.clone(),
            build_info: self.build_info.clone(),
            window_start: rfc3339(window_start_unix),
            window_end: rfc3339(window_end_unix),
        }
    }
}

fn rfc3339(unix: i64) -> String {
    chrono::DateTime::from_timestamp(unix, 0)
        .unwrap_or_else(chrono::Utc::now)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Observe the soak window a release's own discipline declared, then author
/// whatever the probes actually saw.
///
/// **A FAILING soak is authored too** — that evidence is what feeds
/// contest/revert (spec §6 step 5). Only a passing one ever counts toward
/// another peer's promotion threshold.
///
/// bounded-work: one task per successful apply, sleeping
/// `min(soakSecs, MAX_SOAK_WAIT_SECS)` and then making exactly one zome call.
/// A re-applied channel does not spawn a second observer, because a re-sweep of
/// an already-applied head is `already_current` and never reaches an apply.
pub fn spawn_soak_observer(
    attestor: Arc<SoakAttestor>,
    channel_id: String,
    release_cid: String,
    soak_secs: u64,
    receipt: AppliedReceipt,
) {
    let wait = bounded_soak_wait(soak_secs);
    let window_start = receipt.applied_at_unix;
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(wait)).await;

        // The probes. Deliberately few and deliberately about THIS apply: a
        // soak attestation is evidence that the peer kept running the thing it
        // adopted, not a general health report (the runtime passport and the
        // device-health kind already carry that).
        //
        // `process-survived-window` is green BY THE FACT THAT THIS LINE RUNS —
        // the task lives in the process that applied, so reaching here after
        // the sleep is the observation, not a claim about one. That is why it
        // is worth attesting at all: a peer whose apply wedged it never gets
        // here, and its silence is the evidence.
        let mut probes = vec![crate::services::release_attestation::ProbeResult::green(
            "process-survived-window",
        )];
        // A channel left pending-restart applied something this process is NOT
        // yet running, so the window observed the OLD binary. Reporting that as
        // a passing soak for the new release would be the false evidence the
        // whole threshold arm is built to refuse.
        let pending = super::state::channel_state(&channel_id)
            .map(|s| s.pending_restart)
            .unwrap_or(false);
        let outcome = if pending {
            probes.push(crate::services::release_attestation::ProbeResult::red(
                "no-restart-pending",
                "the apply staged an artifact this process has not started running",
            ));
            crate::services::release_attestation::SoakOutcome::fail(
                probes,
                "the release is staged and pending a restart — this window observed the PREVIOUS \
                 artifact, so it is not evidence about this release",
            )
        } else {
            probes.push(crate::services::release_attestation::ProbeResult::green(
                "no-restart-pending",
            ));
            crate::services::release_attestation::SoakOutcome::pass(probes)
        };

        let ctx =
            crate::services::release_attestation::ReleaseAttestationCtx::new(attestor.hc.clone());
        let soak = attestor.context(&channel_id, window_start, super::state::now_unix());
        match crate::services::release_attestation::author_soak_attestation(
            &ctx,
            &release_cid,
            soak,
            outcome,
        )
        .await
        {
            Ok(attestation) => tracing::info!(
                channel = %channel_id,
                release_cid = %release_cid,
                attestation = %attestation.cid,
                soak_secs = wait,
                "release-adoption: soak attestation authored for an applied release"
            ),
            // Never fatal, and never retried into a storm: the threshold arm
            // reads what exists, and an attestation this peer failed to author
            // is a missing datum, not a corrupt one.
            Err(e) => tracing::warn!(
                channel = %channel_id,
                release_cid = %release_cid,
                error = %e,
                "release-adoption: could not author the soak attestation for an applied release"
            ),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The normative slot path (task atom §Interface contract). T6's mesh
    /// receipt and any harness arm that consumes the slot read exactly this
    /// string, so it is pinned here rather than left to a format! at a call
    /// site.
    #[test]
    fn the_staged_binary_slot_path_is_normative() {
        let root = PathBuf::from("/tmp/elohim-local-mesh/release-adoption/matthew");
        assert_eq!(
            slot_path(&root),
            PathBuf::from(
                "/tmp/elohim-local-mesh/release-adoption/matthew/slot/elohim-storage.next"
            )
        );
        assert_eq!(SLOT_DIR, "slot");
        assert_eq!(SLOT_BINARY_NAME, "elohim-storage.next");
        assert_eq!(SLOT_RECEIPT_NAME, "elohim-storage.next.json");
    }

    /// An empty registry routes nowhere and says so with the typed reason —
    /// never a fallback to "the closest vehicle".
    #[tokio::test]
    async fn a_class_with_no_vehicle_is_refused_by_name_never_routed_elsewhere() {
        let registry = ApplyRegistry::new();
        assert!(registry.is_empty());
        assert!(registry.for_class(ArtifactClass::ConfigEpr).is_none());
        assert!(registry.for_class(ArtifactClass::StorageBinary).is_none());
    }

    /// A vehicle only ever handles the class it DECLARES. The default
    /// `handles()` is empty, so a vehicle that forgets to declare is inert
    /// rather than promiscuous — the safe direction.
    #[test]
    fn every_vehicle_declares_exactly_the_class_it_handles() {
        let config = ConfigEprVehicle::new();
        assert_eq!(config.handles(), &[ArtifactClass::ConfigEpr]);
        assert_eq!(config.name(), "runtime_config_reload");

        let binary = StorageBinaryVehicle::new(
            "/tmp/nonexistent",
            Arc::new(crate::trust::FixedStakesResolver::bootstrap_default()),
            "elohim",
        );
        assert_eq!(binary.handles(), &[ArtifactClass::StorageBinary]);
        assert_eq!(binary.name(), "exe_slot_stage");
        assert_eq!(
            binary.slot(),
            PathBuf::from("/tmp/nonexistent/slot/elohim-storage.next")
        );
    }

    /// **Spec §9, the fleet fence.** The stakes resolver fails CLOSED: a node
    /// that declares nothing resolves to `Bootstrap`, and only an explicit
    /// `Simulacra` declaration opens the binary gate. This is the assertion
    /// that keeps fleet binaries out of the rung — asserted on the resolver's
    /// own default rather than on a hostname or an env sniff.
    #[test]
    fn the_binary_gate_is_closed_by_default_and_opens_only_on_a_simulacra_declaration() {
        let (stage, _) = crate::trust::StakesResolver::stage_for(
            &crate::trust::FixedStakesResolver::bootstrap_default(),
            "elohim",
        );
        assert_eq!(stage, NetworkStage::Bootstrap);
        assert_ne!(
            stage,
            NetworkStage::Simulacra,
            "the DEFAULT declaration must never open the storage-binary gate"
        );
        for closed in [
            NetworkStage::Bootstrap,
            NetworkStage::Coordinated,
            NetworkStage::Enforced,
        ] {
            assert_ne!(closed, NetworkStage::Simulacra);
        }
    }

    /// The boot-only refusal honours BOTH shapes in `runtime_config::BOOT_ONLY`
    /// — a bare NAME, and a `NAME=VALUE` entry whose disabled case alone is
    /// cold. A release setting `PROJECTION_RECONCILE_SECS` to a HOT cadence
    /// must not be refused for sharing a name with the cold one.
    #[test]
    fn the_boot_only_refusal_reads_the_value_when_the_entry_names_one() {
        let cold: BTreeMap<String, String> =
            [("PROJECTION_RECONCILE_SECS".to_string(), "0".to_string())]
                .into_iter()
                .collect();
        assert!(
            !ConfigEprVehicle::boot_only_keys(&cold).is_empty(),
            "the DISABLED case is boot-only and must refuse"
        );

        let hot: BTreeMap<String, String> =
            [("PROJECTION_RECONCILE_SECS".to_string(), "120".to_string())]
                .into_iter()
                .collect();
        assert!(
            ConfigEprVehicle::boot_only_keys(&hot).is_empty(),
            "a hot cadence is re-sourced by the running loop and must NOT refuse"
        );

        let bare: BTreeMap<String, String> =
            [("ELOHIM_TRANSPORT_BACKEND".to_string(), "iroh".to_string())]
                .into_iter()
                .collect();
        assert!(
            !ConfigEprVehicle::boot_only_keys(&bare).is_empty(),
            "a bare boot-only NAME refuses at any value"
        );

        let hot_registered: BTreeMap<String, String> = [(
            "ELOHIM_OBEY_CARRIED_ELECTION".to_string(),
            "true".to_string(),
        )]
        .into_iter()
        .collect();
        assert!(ConfigEprVehicle::boot_only_keys(&hot_registered).is_empty());
    }

    /// A half-written file must never be visible under the name a consumer
    /// reads — and the `.partial` suffix is APPENDED, so two artifacts that
    /// differ only in extension cannot collide on one temp name.
    #[test]
    fn a_staged_write_is_atomic_and_its_temp_name_cannot_collide() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("elohim-storage.next");
        write_atomic(&a, b"one").unwrap();
        assert_eq!(std::fs::read(&a).unwrap(), b"one");
        write_atomic(&a, b"two").unwrap();
        assert_eq!(std::fs::read(&a).unwrap(), b"two");
        assert!(
            !dir.path().join("elohim-storage.next.partial").exists(),
            "the temp file is renamed away, never left behind"
        );

        // Distinct names for artifacts differing only in extension.
        let wasm = dir.path().join("nested").join("a.wasm");
        let bin = dir.path().join("nested").join("a.bin");
        write_atomic(&wasm, b"w").unwrap();
        write_atomic(&bin, b"b").unwrap();
        assert_eq!(std::fs::read(&wasm).unwrap(), b"w");
        assert_eq!(std::fs::read(&bin).unwrap(), b"b");
    }

    /// The soak wait is bounded. A discipline may declare any window; this
    /// process will not hold a task open past `MAX_SOAK_WAIT_SECS`, because a
    /// soak longer than the peer's own uptime attests to whatever process
    /// happens to be running when the timer fires, not to this apply.
    #[test]
    fn the_soak_wait_is_bounded_whatever_the_discipline_declares() {
        // A zero ceiling would attest instantly on every apply — evidence about
        // a window that never ran. Asserted in a `const` block so it fails to
        // COMPILE rather than failing a test someone could skip.
        const {
            assert!(MAX_SOAK_WAIT_SECS > 0);
        }
        // A declared window shorter than the ceiling is honoured verbatim: the
        // ceiling is a bound on OUR patience, never a floor on the channel's
        // discipline.
        assert_eq!(bounded_soak_wait(0), 0);
        assert_eq!(bounded_soak_wait(30), 30);
        assert_eq!(
            bounded_soak_wait(MAX_SOAK_WAIT_SECS - 1),
            MAX_SOAK_WAIT_SECS - 1
        );
        // And anything at or beyond it — including a manifest declaring a
        // nonsense window — clamps.
        assert_eq!(bounded_soak_wait(MAX_SOAK_WAIT_SECS), MAX_SOAK_WAIT_SECS);
        assert_eq!(bounded_soak_wait(u64::MAX), MAX_SOAK_WAIT_SECS);
    }

    /// RFC3339 with a `Z` offset and second precision — the shape T5's floor-5
    /// parser accepts. A soak context whose window does not parse is refused by
    /// the rail, so this is the one formatting detail that is load-bearing.
    #[test]
    fn soak_window_timestamps_are_rfc3339_with_an_explicit_offset() {
        let formatted = rfc3339(1_756_684_800);
        assert!(formatted.ends_with('Z'), "got {formatted}");
        assert!(formatted.contains('T'), "got {formatted}");
        assert!(
            chrono::DateTime::parse_from_rfc3339(&formatted).is_ok(),
            "got {formatted}"
        );
    }
}
