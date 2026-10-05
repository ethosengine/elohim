//! Coordinator staleness as a standing reading, and the bundle a later sweep
//! is measured against.
//!
//! A conductor that hosts people carries one installed app per hosted person,
//! and `update_coordinators` is addressed per cell. Three things follow, and
//! this module owns the storage half of each
//! (`genesis/data/timeline/backlog/coordinator-acceptance-tightening-contract.md`
//! items 4–6, `hosted-app-coordinator-coverage-gaps.md` gap 1):
//!
//! 1. **The last applied bundle is kept.** A coordinator-only release does not
//!    restart the conductor's pod, so the bundle on the conductor's disk stays
//!    the previous one and a person provisioned afterwards starts on the old
//!    coordinators. Every sweep that APPLIES a bundle the node's own app takes
//!    cleanly records it under `<storage_dir>/coordinators/last-applied.happ`
//!    (write-then-rename). Last applied wins, whichever path applied it: the
//!    boot path (from `--happ-path`), `POST /admin/coordinators/sync` (the
//!    posted body), or release adoption. [`sweep_bundle_path`] names the bundle
//!    a later sweep uses: the persisted one when present, else the boot bundle.
//!
//! 2. **Staleness is a standing reading, not an incident.** A slow background
//!    pass ([`spawn`]) makes ONE `list_apps` call and sweeps only apps it has
//!    not seen against the current bundle — every app on the first pass after
//!    start or after the bundle changes, otherwise only newly installed ones.
//!    Storage is the only writer of coordinators on its conductor, so an app
//!    already read changes only through code that also folds its report here.
//!    The work is bounded before any conductor call, because a conductor call
//!    cannot be cancelled: one app at a time, at most [`MAX_APPS_PER_PASS`]
//!    per pass, one conductor-wide sweep at a time on this node
//!    ([`sweep_lock`]). Every conductor-wide sweep (boot, route, adoption)
//!    folds its report into the same reading.
//!
//! 3. **The reading is published as aggregates only.** Pending hot-swaps,
//!    lineage refusals, distinct installed coordinator sets per role and the
//!    last pass time go to Prometheus and to the projected conductor
//!    diagnostics. App ids never leave this node through either: which people
//!    a conductor hosts is not published. Per-app detail stays on the
//!    node-local `POST /admin/coordinators/sync` report.
//!
//! The statement contract the own app's content-store coordinator declares is
//! read once per distinct coordinator wasm hash, never once per app.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, OnceLock};
use std::time::Duration;

use sha2::{Digest, Sha256};
use tracing::{debug, error, info, warn};

use crate::happ_manager::{
    self, AppSweepSkip, CoordinatorConductorReport, CoordinatorRoleReport, CoordinatorSyncReport,
};
use crate::hc_client::HcClient;

// ---------------------------------------------------------------------------
// The statement contract
// ---------------------------------------------------------------------------

/// What a content-store coordinator states about the signed-statement domains
/// it handles. Deserialized from the extern's snake_case msgpack; serialized
/// camelCase at the HTTP boundary.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct StatementContract {
    pub contract_version: u32,
    pub statements: Vec<StatementRow>,
}

/// One signed-statement domain and what this coordinator does with it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct StatementRow {
    pub statement: String,
    pub form: String,
    /// The coordinator issues statements in this form.
    pub issues: bool,
    /// The coordinator accepts this form for a NEW act.
    pub accepts_new: bool,
    /// The coordinator honors this form on a historical read.
    pub honors_historical: bool,
}

/// Why a statement contract was not read. The label is the stable prefix of
/// `statementContractError` (`<label>: <detail>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractReadFailure {
    /// No app-interface client reaches the node's own app on this path (the
    /// boot sweep runs before one exists).
    NoAppClient,
    /// The client in hand is bound to a different app than the one named as
    /// the node's own.
    ReaderAppMismatch,
    /// The running coordinator has no `statement_contract` extern — it is
    /// older than the contract. Expected on older bundles; never blocking.
    ExternAbsent,
    /// The call failed for any other reason (shed, transport, cell state).
    CallFailed,
    /// The extern answered in a shape this storage cannot read.
    DecodeFailed,
}

impl ContractReadFailure {
    pub const ALL: [ContractReadFailure; 5] = [
        Self::NoAppClient,
        Self::ReaderAppMismatch,
        Self::ExternAbsent,
        Self::CallFailed,
        Self::DecodeFailed,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::NoAppClient => "no_app_client",
            Self::ReaderAppMismatch => "reader_app_mismatch",
            Self::ExternAbsent => "extern_absent",
            Self::CallFailed => "call_failed",
            Self::DecodeFailed => "decode_failed",
        }
    }

    /// Does this outcome describe the COORDINATOR (so it holds until the
    /// coordinator's wasm hash changes), rather than the path that asked?
    /// Only those are cached; the rest are asked again on the next pass.
    pub fn speaks_for_the_coordinator(self) -> bool {
        matches!(self, Self::ExternAbsent | Self::DecodeFailed)
    }

    fn from_label(label: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.label() == label)
    }
}

/// Pure: classify a failed `statement_contract` call.
pub fn classify_contract_error(err: &crate::error::StorageError) -> ContractReadFailure {
    if matches!(err, crate::error::StorageError::Serialization(_)) {
        ContractReadFailure::DecodeFailed
    } else if crate::services::conductor_writes::is_unknown_function_error(err)
        || names_a_missing_zome_fn(err)
    {
        ContractReadFailure::ExternAbsent
    } else {
        ContractReadFailure::CallFailed
    }
}

/// `RibosomeError::ZomeFnNotExists` displays as "Attempted to call a zome
/// function that doesn't exist" — and "doesn't exist" does not contain "not
/// exist", so the shared `is_unknown_function_error` misses the Display form.
fn names_a_missing_zome_fn(err: &crate::error::StorageError) -> bool {
    let lowered = err.to_string().to_ascii_lowercase();
    lowered.contains("function") && lowered.contains("doesn't exist")
}

/// Reads the statement contract from the node's own app's content-store cell.
///
/// A seam rather than a concrete client because the conductor-wide sweep holds
/// only the admin websocket; the app-interface client lives in the registry and
/// is not reachable from every caller (the boot sweep runs before it exists).
#[async_trait::async_trait]
pub trait StatementContractReader: Send + Sync {
    /// The installed app whose cell this reader calls.
    fn app_id(&self) -> &str;
    async fn read(&self) -> Result<StatementContract, (ContractReadFailure, String)>;
}

/// The production reader: the registry's `lamad` client, which is bound to the
/// node's own app.
pub struct HcStatementContractReader(Arc<HcClient>);

impl HcStatementContractReader {
    pub fn new(client: Arc<HcClient>) -> Self {
        Self(client)
    }
}

#[async_trait::async_trait]
impl StatementContractReader for HcStatementContractReader {
    fn app_id(&self) -> &str {
        self.0.app_id()
    }

    async fn read(&self) -> Result<StatementContract, (ContractReadFailure, String)> {
        crate::services::conductor_writes::call_statement_contract(&self.0)
            .await
            .map_err(|e| (classify_contract_error(&e), e.to_string()))
    }
}

/// One read of the contract for `primary`, rendered for the report. Never
/// panics and never blocks a sweep: every failure is a named string.
pub(crate) async fn read_statement_contract(
    reader: Option<&dyn StatementContractReader>,
    primary: &str,
) -> Result<StatementContract, String> {
    let render = |kind: ContractReadFailure, detail: &str| format!("{}: {detail}", kind.label());
    let Some(reader) = reader else {
        return Err(render(
            ContractReadFailure::NoAppClient,
            "no app-interface client reaches the node's own app on this path",
        ));
    };
    if reader.app_id() != primary {
        return Err(render(
            ContractReadFailure::ReaderAppMismatch,
            &format!(
                "the app-interface client is bound to '{}', not the node's own app '{primary}'",
                reader.app_id()
            ),
        ));
    }
    reader.read().await.map_err(|(kind, detail)| {
        if kind == ContractReadFailure::ExternAbsent {
            info!(
                primary,
                "statement_contract extern absent — this coordinator is older than the contract"
            );
        } else {
            warn!(
                primary,
                failure = kind.label(),
                detail = detail.as_str(),
                "statement_contract read failed"
            );
        }
        render(kind, &detail)
    })
}

// ---------------------------------------------------------------------------
// The persisted bundle (gap 1)
// ---------------------------------------------------------------------------

/// File name of the last applied bundle under `<storage_dir>/coordinators/`.
pub const LAST_APPLIED_FILE: &str = "last-applied.happ";

static BUNDLE_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Arm the persisted-bundle store under the node's own storage dir. Without
/// it (unit tests, library consumers) nothing is persisted and
/// [`sweep_bundle_path`] answers from the boot bundle alone.
pub fn init(storage_dir: &Path) {
    let _ = BUNDLE_DIR.get_or_init(|| storage_dir.join("coordinators"));
}

/// Where the last applied bundle lives, when the store is armed.
pub fn persisted_bundle_path() -> Option<PathBuf> {
    BUNDLE_DIR.get().map(|d| d.join(LAST_APPLIED_FILE))
}

/// Which bundle a sweep is measured against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleSource {
    /// The bundle the last clean apply recorded.
    Persisted,
    /// The bundle the node booted with (`--happ-path`).
    Boot,
}

impl BundleSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::Persisted => "persisted",
            Self::Boot => "boot",
        }
    }
}

/// Pure: the bundle a later sweep uses — the persisted one when it is
/// present, else the boot bundle, else none.
pub fn choose_bundle_path(
    persisted: Option<&Path>,
    persisted_present: bool,
    boot: Option<&Path>,
) -> Option<(PathBuf, BundleSource)> {
    match (persisted, persisted_present, boot) {
        (Some(p), true, _) => Some((p.to_path_buf(), BundleSource::Persisted)),
        (_, _, Some(b)) => Some((b.to_path_buf(), BundleSource::Boot)),
        _ => None,
    }
}

/// The bundle a later sweep should use. `boot` is the boot `--happ-path` on a
/// node whose conductor storage boots (embedded); `None` elsewhere, where that
/// path is not this node's bundle.
pub fn sweep_bundle_path(boot: Option<&Path>) -> Option<(PathBuf, BundleSource)> {
    let persisted = persisted_bundle_path();
    let present = persisted.as_deref().is_some_and(Path::is_file);
    choose_bundle_path(persisted.as_deref(), present, boot)
}

/// Record `src` as the last applied bundle. Write-then-rename, so a reader
/// never sees a half-written file. Unchanged bytes are not rewritten. A
/// failure is logged, never propagated: the swap it follows already happened.
pub async fn record_applied_bundle(src: &Path) {
    let Some(dest) = persisted_bundle_path() else {
        debug!("persisted-bundle store not armed — last applied bundle not recorded");
        return;
    };
    if src == dest {
        return;
    }
    let bytes = match tokio::fs::read(src).await {
        Ok(b) => b,
        Err(e) => {
            warn!(src = %src.display(), error = %e, "could not read the applied bundle to record it");
            return;
        }
    };
    if let Ok(existing) = tokio::fs::read(&dest).await {
        if existing == bytes {
            return;
        }
    }
    let write = async {
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let mut tmp = dest.as_os_str().to_os_string();
        tmp.push(".partial");
        let tmp = PathBuf::from(tmp);
        tokio::fs::write(&tmp, &bytes).await?;
        tokio::fs::rename(&tmp, &dest).await
    };
    match write.await {
        Ok(()) => info!(
            src = %src.display(),
            dest = %dest.display(),
            bytes = bytes.len(),
            "recorded the last applied coordinator bundle — later sweeps (hosted apps provisioned \
             after this) are measured against it"
        ),
        Err(e) => error!(
            dest = %dest.display(),
            error = %e,
            "could not record the last applied coordinator bundle — a person provisioned later \
             may start on older coordinators until the next apply"
        ),
    }
}

/// Pure: a bundle's identity — its lineage plus its coordinators, hashed.
/// Two bundles with the same identity install the same thing.
pub fn bundle_identity(
    bundle_dna: &BTreeMap<String, String>,
    bundle_coordinators: &BTreeMap<String, BTreeMap<String, String>>,
) -> String {
    let mut h = Sha256::new();
    for (role, dna) in bundle_dna {
        h.update(role.as_bytes());
        h.update(b"=");
        h.update(dna.as_bytes());
        h.update(b"\n");
    }
    h.update(b"--\n");
    for (role, zomes) in bundle_coordinators {
        for (zome, wasm) in zomes {
            h.update(role.as_bytes());
            h.update(b"/");
            h.update(zome.as_bytes());
            h.update(b"=");
            h.update(wasm.as_bytes());
            h.update(b"\n");
        }
    }
    hex::encode(h.finalize())
}

// ---------------------------------------------------------------------------
// The standing reading
// ---------------------------------------------------------------------------

/// The role and zome the statement contract is read from.
const CONTRACT_ROLE: &str = "lamad";
const CONTRACT_ZOME: &str = "content_store";

/// What one role of one app is known to run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct RoleState {
    /// zome → coordinator wasm hash it runs now (the bundle's, once applied).
    installed: BTreeMap<String, String>,
    /// Drifted, not applied, not a lineage refusal: a hot-swap can heal it.
    pending: bool,
    lineage_refused: bool,
    errored: bool,
}

fn role_state(r: &CoordinatorRoleReport) -> RoleState {
    let lineage_refused = happ_manager::is_lineage_refusal(r);
    RoleState {
        installed: if r.applied {
            r.bundled_coordinators.clone()
        } else {
            r.installed_coordinators.clone()
        },
        pending: r.drifted && !r.applied && !lineage_refused,
        lineage_refused,
        errored: r.error.is_some(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ContractReading {
    /// The own app's content-store coordinator hash this read is about.
    coordinator_key: String,
    contract: Option<StatementContract>,
    error: Option<String>,
}

/// The conductor's coordinator reading, measured against one bundle.
#[derive(Debug, Default)]
pub struct CoordinatorReading {
    /// Identity of the bundle the reading is measured against.
    identity: Option<String>,
    bundle_source: Option<BundleSource>,
    /// Apps on the bundle's lineage that a sweep has read → role → state.
    apps: BTreeMap<String, BTreeMap<String, RoleState>>,
    /// Apps the standing pass need not read again against this bundle.
    seen: BTreeSet<String>,
    last_pass_unix: Option<u64>,
    contract: Option<ContractReading>,
}

/// The aggregate the reading publishes. No app id appears in it.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoordinatorReadingSummary {
    /// False until any sweep has completed since this process started.
    pub observed: bool,
    pub pending_roles: usize,
    pub lineage_refused_roles: usize,
    pub errored_roles: usize,
    /// role → number of DISTINCT coordinator sets installed across apps. One
    /// means every app runs the same coordinators for that role.
    pub distinct_coordinator_sets: BTreeMap<String, usize>,
    /// Apps with at least one role on the bundle's lineage.
    pub apps_on_lineage: usize,
    pub last_pass_unix: Option<u64>,
    /// Short identity of the bundle the reading is measured against.
    pub bundle_identity: Option<String>,
    pub bundle_source: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement_contract: Option<StatementContract>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement_contract_error: Option<String>,
}

impl CoordinatorReading {
    /// Start measuring against a different bundle: everything known about
    /// the old one is dropped and every app is read again.
    fn reset_to(&mut self, identity: &str) {
        if self.identity.as_deref() == Some(identity) {
            return;
        }
        self.identity = Some(identity.to_string());
        self.apps.clear();
        self.seen.clear();
    }

    fn retain_listed(&mut self, listed: &BTreeSet<String>) {
        self.apps.retain(|id, _| listed.contains(id));
        self.seen.retain(|id| listed.contains(id));
    }

    /// Record one app's sweep. `settles` = the standing pass need not read it
    /// again against this bundle.
    pub(crate) fn record_app(&mut self, report: &CoordinatorSyncReport, settles: bool) {
        let roles = report
            .roles
            .iter()
            .map(|r| (r.role.clone(), role_state(r)))
            .collect();
        self.apps.insert(report.app_id.clone(), roles);
        if settles {
            self.seen.insert(report.app_id.clone());
        }
    }

    /// The app is not on the bundle's lineage (or cannot be touched): it
    /// carries no coordinator reading, and costs no call to re-list.
    fn forget(&mut self, app_id: &str) {
        self.apps.remove(app_id);
        self.seen.insert(app_id.to_string());
    }

    /// Is the app known to have taken this bundle without any role erroring?
    fn app_healthy(&self, app_id: &str) -> bool {
        self.apps
            .get(app_id)
            .is_some_and(|roles| roles.values().all(|r| !r.errored))
    }

    fn contract_key(&self, own_app: &str) -> Option<String> {
        self.apps.get(own_app).map(|roles| {
            roles
                .get(CONTRACT_ROLE)
                .and_then(|r| r.installed.get(CONTRACT_ZOME))
                .cloned()
                .unwrap_or_default()
        })
    }

    /// Does the standing pass need to (re)read the contract for `own_app`?
    fn contract_wanted(&self, own_app: &str) -> Option<String> {
        let key = self.contract_key(own_app)?;
        match &self.contract {
            Some(c) if c.coordinator_key == key => None,
            _ => Some(key),
        }
    }

    fn record_contract(&mut self, key: String, outcome: &Result<StatementContract, String>) {
        let (contract, error) = match outcome {
            Ok(c) => (Some(c.clone()), None),
            Err(e) => {
                let label = e.split(':').next().unwrap_or("");
                let settles = ContractReadFailure::from_label(label)
                    .is_some_and(ContractReadFailure::speaks_for_the_coordinator);
                if !settles {
                    return;
                }
                (None, Some(e.clone()))
            }
        };
        self.contract = Some(ContractReading {
            coordinator_key: key,
            contract,
            error,
        });
    }

    /// Fold a conductor-wide sweep's report. Its apps ∪ skipped apps are the
    /// whole of `list_apps`, so an app absent from both was uninstalled.
    ///
    /// `gate_apply` is whether THIS node applies hot-swaps: an app a sweep
    /// read in the same mode the standing pass would run, or found current,
    /// is settled; an app a dry run found drifted on a node that applies is
    /// left for the standing pass to heal.
    pub(crate) fn fold_conductor_report(
        &mut self,
        report: &CoordinatorConductorReport,
        identity: &str,
        gate_apply: bool,
        now_unix: u64,
    ) {
        self.reset_to(identity);
        let listed: BTreeSet<String> = report
            .apps
            .iter()
            .map(|a| a.app_id.clone())
            .chain(report.skipped_apps.iter().map(|s| s.app_id.clone()))
            .collect();
        self.retain_listed(&listed);
        for app in &report.apps {
            let pending = app.roles.iter().any(|r| role_state(r).pending);
            self.record_app(app, app.apply == gate_apply || !pending);
        }
        for skip in &report.skipped_apps {
            if !AppSweepSkip::leaves_reading_unchanged(&skip.reason) {
                self.forget(&skip.app_id);
            }
        }
        if let Some(own) = report.primary_app_id.as_deref() {
            let outcome = match (&report.statement_contract, &report.statement_contract_error) {
                (Some(c), _) => Some(Ok(c.clone())),
                (None, Some(e)) => Some(Err(e.clone())),
                (None, None) => None,
            };
            if let (Some(outcome), Some(key)) = (outcome, self.contract_key(own)) {
                self.record_contract(key, &outcome);
            }
        }
        self.last_pass_unix = Some(now_unix);
    }

    /// Pure: the aggregate. App ids stay inside.
    pub fn summary(&self) -> CoordinatorReadingSummary {
        let mut s = CoordinatorReadingSummary {
            observed: self.last_pass_unix.is_some(),
            last_pass_unix: self.last_pass_unix,
            bundle_identity: self.identity.as_ref().map(|i| i.chars().take(16).collect()),
            bundle_source: self.bundle_source.map(BundleSource::label),
            ..Default::default()
        };
        let mut sets: BTreeMap<&str, BTreeSet<&BTreeMap<String, String>>> = BTreeMap::new();
        for roles in self.apps.values() {
            if roles.values().any(|r| !r.lineage_refused) {
                s.apps_on_lineage += 1;
            }
            for (role, r) in roles {
                s.pending_roles += usize::from(r.pending);
                s.lineage_refused_roles += usize::from(r.lineage_refused);
                s.errored_roles += usize::from(r.errored);
                if !r.installed.is_empty() {
                    sets.entry(role.as_str()).or_default().insert(&r.installed);
                }
            }
        }
        s.distinct_coordinator_sets = sets
            .into_iter()
            .map(|(role, set)| (role.to_string(), set.len()))
            .collect();
        if let Some(c) = &self.contract {
            s.statement_contract = c.contract.clone();
            s.statement_contract_error = c.error.clone();
        }
        s
    }
}

/// At most this many apps are read per standing pass. A conductor that gained
/// dozens of apps spreads the reads over passes rather than holding one
/// admin request after another for minutes; each app costs one
/// `get_dna_definition` per role, plus one `update_coordinators` per drifted
/// role on a node that applies.
pub const MAX_APPS_PER_PASS: usize = 8;

/// Pure: which apps a standing pass reads, in order. `eligible` is every app
/// on the bundle's lineage in `list_apps` order. The node's own app goes first
/// when it is unread. `own_blocks` = this pass applies and the own app is
/// known not to have taken the bundle cleanly (or is not on the lineage at
/// all), so no hosted app is read — the canary gate, standing form.
pub fn select_apps_for_pass(
    eligible: &[String],
    seen: &BTreeSet<String>,
    own_app: &str,
    own_blocks: bool,
    cap: usize,
) -> Vec<String> {
    let unseen: Vec<&String> = eligible.iter().filter(|id| !seen.contains(*id)).collect();
    let own_unseen = unseen.iter().any(|id| id.as_str() == own_app);
    if own_blocks && !own_unseen {
        return Vec::new();
    }
    let mut order: Vec<String> = Vec::with_capacity(unseen.len());
    if own_unseen {
        order.push(own_app.to_string());
    }
    order.extend(
        unseen
            .into_iter()
            .filter(|id| id.as_str() != own_app)
            .cloned(),
    );
    order.truncate(cap);
    order
}

static READING: LazyLock<Mutex<CoordinatorReading>> =
    LazyLock::new(|| Mutex::new(CoordinatorReading::default()));

fn reading() -> std::sync::MutexGuard<'static, CoordinatorReading> {
    READING.lock().unwrap_or_else(|e| e.into_inner())
}

static SWEEP_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// One conductor-wide coordinator sweep at a time on this node: the standing
/// pass and the boot/route/adoption sweeps never interleave admin requests.
pub(crate) fn sweep_lock() -> &'static tokio::sync::Mutex<()> {
    &SWEEP_LOCK
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn publish(summary: &CoordinatorReadingSummary) {
    crate::metrics::set_coordinator_reading(
        summary.pending_roles,
        summary.lineage_refused_roles,
        summary.apps_on_lineage,
        &summary.distinct_coordinator_sets,
        summary.last_pass_unix,
    );
}

/// Fold a conductor-wide sweep's report into the standing reading.
pub(crate) fn fold_conductor_report(report: &CoordinatorConductorReport, identity: &str) {
    let gate = happ_manager::coordinator_update_allowed();
    let summary = {
        let mut r = reading();
        r.fold_conductor_report(report, identity, gate, now_unix());
        r.summary()
    };
    publish(&summary);
}

/// Fold one app's sweep into the reading (release adoption sweeps the node's
/// own app on its own before the conductor-wide sweep).
pub fn fold_app_report(report: &CoordinatorSyncReport) {
    let gate = happ_manager::coordinator_update_allowed();
    let summary = {
        let mut r = reading();
        let pending = report.roles.iter().any(|x| role_state(x).pending);
        r.record_app(report, report.apply == gate || !pending);
        r.summary()
    };
    publish(&summary);
}

/// The aggregate reading, for diagnostics.
pub fn summary() -> CoordinatorReadingSummary {
    reading().summary()
}

// ---------------------------------------------------------------------------
// The standing pass
// ---------------------------------------------------------------------------

/// Env var naming the standing pass interval in seconds.
pub const STANDING_INTERVAL_ENV: &str = "ELOHIM_COORDINATOR_STANDING_INTERVAL_SECS";
const DEFAULT_INTERVAL_SECS: u64 = 300;
/// Floor: a pass costs admin requests on a conductor that is serving people.
const MIN_INTERVAL_SECS: u64 = 30;

/// Pure: the pass interval from the env var's raw value.
pub fn standing_interval(raw: Option<&str>) -> Duration {
    let secs = raw
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_INTERVAL_SECS)
        .max(MIN_INTERVAL_SECS);
    Duration::from_secs(secs)
}

/// Resolves the conductor admin connection afresh for each pass.
pub type AdminSource = Arc<dyn Fn() -> Option<holochain_client::AdminWebsocket> + Send + Sync>;
/// Resolves the app-interface reader for the node's own app afresh each pass.
pub type ReaderSource = Arc<dyn Fn() -> Option<Arc<dyn StatementContractReader>> + Send + Sync>;

pub struct StandingInputs {
    /// The node's own app id.
    pub own_app: String,
    /// The boot bundle, on a node whose conductor storage boots.
    pub boot_happ_path: Option<PathBuf>,
    pub admin: AdminSource,
    pub reader: ReaderSource,
}

/// Bundle cache key: re-unpack only when the file changed.
type BundleKey = (PathBuf, u64, Option<std::time::SystemTime>);

/// Start the standing pass. Off the readiness path; the first pass waits one
/// interval, by which time the boot sweep has folded the boot report.
pub fn spawn(inputs: StandingInputs) -> tokio::task::JoinHandle<()> {
    let interval = standing_interval(std::env::var(STANDING_INTERVAL_ENV).ok().as_deref());
    info!(
        own_app = inputs.own_app.as_str(),
        interval_secs = interval.as_secs(),
        max_apps_per_pass = MAX_APPS_PER_PASS,
        "coordinator standing reading armed"
    );
    tokio::spawn(async move {
        let mut cache: Option<(BundleKey, Arc<happ_manager::LoadedBundle>)> = None;
        loop {
            tokio::time::sleep(interval).await;
            run_pass(&inputs, &mut cache).await;
        }
    })
}

async fn bundle_for_pass(
    path: &Path,
    cache: &mut Option<(BundleKey, Arc<happ_manager::LoadedBundle>)>,
) -> Option<Arc<happ_manager::LoadedBundle>> {
    let meta = tokio::fs::metadata(path).await.ok()?;
    let key: BundleKey = (path.to_path_buf(), meta.len(), meta.modified().ok());
    if let Some((k, b)) = cache.as_ref() {
        if *k == key {
            return Some(Arc::clone(b));
        }
    }
    match happ_manager::load_bundle(path).await {
        Ok(b) => {
            let b = Arc::new(b);
            *cache = Some((key, Arc::clone(&b)));
            Some(b)
        }
        Err(e) => {
            warn!(bundle = %path.display(), error = %e, "coordinator standing pass: bundle unreadable — pass skipped");
            None
        }
    }
}

async fn run_pass(
    inputs: &StandingInputs,
    cache: &mut Option<(BundleKey, Arc<happ_manager::LoadedBundle>)>,
) {
    let Some((path, source)) = sweep_bundle_path(inputs.boot_happ_path.as_deref()) else {
        debug!("coordinator standing pass: no bundle to measure against on this node");
        return;
    };
    let Some(bundle) = bundle_for_pass(&path, cache).await else {
        return;
    };
    let Some(admin) = (inputs.admin)() else {
        debug!("coordinator standing pass: no conductor admin connection");
        return;
    };
    let _one_sweep = sweep_lock().lock().await;

    let apps = match admin.list_apps(None).await {
        Ok(apps) => apps,
        Err(e) => {
            warn!(error = %e, "coordinator standing pass: list_apps failed — pass skipped");
            return;
        }
    };
    let identity = bundle.identity();
    let gate = happ_manager::coordinator_update_allowed();
    let own = inputs.own_app.as_str();

    let selected = {
        let mut r = reading();
        r.reset_to(&identity);
        r.bundle_source = Some(source);
        let listed: BTreeSet<String> = apps.iter().map(|a| a.installed_app_id.clone()).collect();
        r.retain_listed(&listed);
        let mut eligible = Vec::new();
        for app in &apps {
            let id = app.installed_app_id.as_str();
            let skip = happ_manager::app_sweep_skip(
                &happ_manager::installed_role_dna(app),
                &bundle.bundle_dna,
                matches!(
                    app.status,
                    holochain_types::app::AppStatus::Unrecoverable(..)
                ),
                false,
                id == own,
            );
            match skip {
                Some(_) => r.forget(id),
                None => eligible.push(id.to_string()),
            }
        }
        let own_unseen = eligible.iter().any(|id| id == own) && !r.seen.contains(own);
        let own_blocks = gate && !own_unseen && !r.app_healthy(own);
        select_apps_for_pass(&eligible, &r.seen, own, own_blocks, MAX_APPS_PER_PASS)
    };

    for id in &selected {
        let Some(app) = apps.iter().find(|a| &a.installed_app_id == id) else {
            continue;
        };
        let report = happ_manager::sweep_app_roles(&admin, app, &bundle.role_dnas, gate).await;
        let canary_failed = id == own && gate && !happ_manager::role_errors(&report).is_empty();
        reading().record_app(&report, true);
        if canary_failed {
            warn!(
                own_app = own,
                "coordinator standing pass: the node's own app did not take the bundle cleanly — \
                 no hosted app is read in this pass"
            );
            break;
        }
        tokio::task::yield_now().await;
    }

    // The contract: once per distinct own-app content-store coordinator hash.
    let wanted = reading().contract_wanted(own);
    if let Some(key) = wanted {
        let reader = (inputs.reader)();
        let outcome = read_statement_contract(reader.as_deref(), own).await;
        reading().record_contract(key, &outcome);
    }

    let summary = {
        let mut r = reading();
        r.last_pass_unix = Some(now_unix());
        r.summary()
    };
    if !selected.is_empty() {
        info!(
            apps_read = selected.len(),
            pending_roles = summary.pending_roles,
            apps_on_lineage = summary.apps_on_lineage,
            bundle_source = source.label(),
            gate_allows_apply = gate,
            "coordinator standing pass complete"
        );
    }
    publish(&summary);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::happ_manager::{CoordinatorAppSkip, CoordinatorRoleReport, CoordinatorSyncReport};

    fn zomes(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(z, h)| (z.to_string(), h.to_string()))
            .collect()
    }

    fn role(name: &str, installed: &str, bundled: &str) -> CoordinatorRoleReport {
        CoordinatorRoleReport {
            role: name.to_string(),
            drifted: installed != bundled,
            applied: false,
            installed_coordinators: zomes(&[("content_store", installed)]),
            bundled_coordinators: zomes(&[("content_store", bundled)]),
            error: None,
        }
    }

    fn app(id: &str, apply: bool, roles: Vec<CoordinatorRoleReport>) -> CoordinatorSyncReport {
        CoordinatorSyncReport {
            app_id: id.to_string(),
            apply,
            drifted_count: roles.iter().filter(|r| r.drifted).count(),
            applied_count: roles.iter().filter(|r| r.applied).count(),
            roles,
        }
    }

    fn skip(id: &str, reason: AppSweepSkip) -> CoordinatorAppSkip {
        CoordinatorAppSkip {
            app_id: id.to_string(),
            reason: reason.label().to_string(),
            installed_dna: BTreeMap::new(),
            detail: None,
        }
    }

    fn conductor(
        apply: bool,
        apps: Vec<CoordinatorSyncReport>,
        skipped: Vec<CoordinatorAppSkip>,
    ) -> CoordinatorConductorReport {
        CoordinatorConductorReport::build(
            apply,
            apps.len() + skipped.len(),
            apps,
            skipped,
            BTreeMap::new(),
            Some("elohim"),
        )
    }

    #[test]
    fn the_reading_folds_a_report_into_aggregates_with_no_app_id() {
        let mut applied = role("lamad", "old", "new");
        applied.applied = true;
        let mut refused = role("imagodei", "old", "new");
        refused.error = Some("dnaHashMismatch: other lineage".to_string());
        let report = conductor(
            true,
            vec![
                app("elohim", true, vec![applied]),
                // A hosted app whose swap failed: drifted, errored, pending.
                app(
                    "hosted-a",
                    true,
                    vec![{
                        let mut r = role("lamad", "old", "new");
                        r.error = Some("update_coordinators failed: CellMissing".to_string());
                        r
                    }],
                ),
                // A hosted app wholly on another lineage for this role.
                app("hosted-b", true, vec![refused]),
            ],
            vec![skip("foreign", AppSweepSkip::NoRoleInBundle)],
        );
        let mut reading = CoordinatorReading::default();
        reading.fold_conductor_report(&report, "bundle-1", true, 1_700_000_000);
        let s = reading.summary();

        assert!(s.observed);
        assert_eq!(
            s.pending_roles, 1,
            "only hosted-a/lamad can still be healed"
        );
        assert_eq!(s.lineage_refused_roles, 1);
        assert_eq!(s.errored_roles, 2);
        // lamad: elohim runs `new`, hosted-a still `old` → two distinct sets.
        assert_eq!(s.distinct_coordinator_sets.get("lamad"), Some(&2));
        assert_eq!(s.distinct_coordinator_sets.get("imagodei"), Some(&1));
        // hosted-b's only role is refused for lineage; it is not on the lineage.
        assert_eq!(s.apps_on_lineage, 2);
        assert_eq!(s.last_pass_unix, Some(1_700_000_000));

        let wire = serde_json::to_string(&s).expect("summary serializes");
        for id in ["elohim", "hosted-a", "hosted-b", "foreign"] {
            assert!(
                !wire.contains(id),
                "the published reading names no app ({id})"
            );
        }
        assert!(wire.contains("pendingRoles"));
    }

    #[test]
    fn a_new_bundle_drops_the_old_reading_and_an_uninstalled_app_leaves_it() {
        let mut reading = CoordinatorReading::default();
        let first = conductor(
            true,
            vec![
                app("elohim", true, vec![role("lamad", "a", "a")]),
                app("hosted-a", true, vec![role("lamad", "a", "a")]),
            ],
            vec![],
        );
        reading.fold_conductor_report(&first, "bundle-1", true, 1);
        assert_eq!(reading.summary().apps_on_lineage, 2);
        assert!(reading.seen.contains("hosted-a"));

        // Same bundle; hosted-a uninstalled.
        let second = conductor(
            true,
            vec![app("elohim", true, vec![role("lamad", "a", "a")])],
            vec![],
        );
        reading.fold_conductor_report(&second, "bundle-1", true, 2);
        assert_eq!(reading.summary().apps_on_lineage, 1);
        assert!(!reading.seen.contains("hosted-a"));

        // A different bundle: nothing about the old one carries over, and an
        // app held back by the canary is not read in this pass.
        let third = conductor(
            true,
            vec![app("elohim", true, vec![role("lamad", "a", "b")])],
            vec![skip("hosted-c", AppSweepSkip::PrimaryNotHealthy)],
        );
        reading.fold_conductor_report(&third, "bundle-2", true, 3);
        assert_eq!(reading.identity.as_deref(), Some("bundle-2"));
        assert!(!reading.seen.contains("hosted-c"));
        assert!(!reading.apps.contains_key("hosted-c"));
    }

    #[test]
    fn a_dry_run_that_finds_drift_on_an_applying_node_leaves_the_app_for_the_standing_pass() {
        let mut reading = CoordinatorReading::default();
        let report = conductor(
            false,
            vec![
                app("drifted", false, vec![role("lamad", "old", "new")]),
                app("current", false, vec![role("lamad", "new", "new")]),
            ],
            vec![],
        );
        reading.fold_conductor_report(&report, "bundle-1", true, 1);
        assert!(
            !reading.seen.contains("drifted"),
            "the standing pass will apply it"
        );
        assert!(reading.seen.contains("current"));
        // On a node that does not apply, the dry run IS the standing pass's read.
        let mut reading = CoordinatorReading::default();
        reading.fold_conductor_report(&report, "bundle-1", false, 1);
        assert!(reading.seen.contains("drifted"));
    }

    #[test]
    fn a_standing_pass_reads_unseen_apps_own_first_and_at_most_the_cap() {
        let eligible: Vec<String> = ["h1", "h2", "elohim", "h3", "h4"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let none = BTreeSet::new();
        assert_eq!(
            select_apps_for_pass(&eligible, &none, "elohim", false, 3),
            vec!["elohim", "h1", "h2"]
        );
        // Seen apps are not read again; the cap spreads the rest over passes.
        let seen: BTreeSet<String> = ["elohim", "h1"].iter().map(|s| s.to_string()).collect();
        assert_eq!(
            select_apps_for_pass(&eligible, &seen, "elohim", false, 2),
            vec!["h2", "h3"]
        );
        // The canary, standing form: an own app known unhealthy holds every
        // hosted app back...
        assert!(select_apps_for_pass(&eligible, &seen, "elohim", true, 8).is_empty());
        // ...unless the own app is itself unread, in which case it goes first
        // and the pass re-judges after it.
        assert_eq!(
            select_apps_for_pass(&eligible, &none, "elohim", true, 2),
            vec!["elohim", "h1"]
        );
        assert!(select_apps_for_pass(&eligible, &none, "elohim", false, 0).is_empty());
    }

    #[test]
    fn the_sweep_bundle_is_the_persisted_one_when_present_else_the_boot_one() {
        let persisted = Path::new("/data/coordinators/last-applied.happ");
        let boot = Path::new("/opt/holochain/elohim.happ");
        assert_eq!(
            choose_bundle_path(Some(persisted), true, Some(boot)),
            Some((persisted.to_path_buf(), BundleSource::Persisted))
        );
        assert_eq!(
            choose_bundle_path(Some(persisted), false, Some(boot)),
            Some((boot.to_path_buf(), BundleSource::Boot))
        );
        assert_eq!(
            choose_bundle_path(None, false, Some(boot)),
            Some((boot.to_path_buf(), BundleSource::Boot))
        );
        assert_eq!(choose_bundle_path(Some(persisted), false, None), None);
        assert_eq!(
            choose_bundle_path(Some(persisted), true, None),
            Some((persisted.to_path_buf(), BundleSource::Persisted))
        );
    }

    #[test]
    fn a_contract_is_read_once_per_coordinator_and_a_path_failure_is_not_cached() {
        let mut reading = CoordinatorReading::default();
        let mut lamad = role("lamad", "old", "new");
        lamad.applied = true;
        reading.record_app(&app("elohim", true, vec![lamad]), true);
        assert_eq!(reading.contract_wanted("elohim"), Some("new".to_string()));

        // A path that had no client says nothing about the coordinator.
        reading.record_contract(
            "new".to_string(),
            &Err("no_app_client: boot path".to_string()),
        );
        assert_eq!(reading.contract_wanted("elohim"), Some("new".to_string()));

        // An older coordinator without the extern is a fact about it: held
        // until its hash changes.
        reading.record_contract(
            "new".to_string(),
            &Err("extern_absent: zome function not found".to_string()),
        );
        assert_eq!(reading.contract_wanted("elohim"), None);
        assert_eq!(
            reading.summary().statement_contract_error.as_deref(),
            Some("extern_absent: zome function not found")
        );

        let mut newer = role("lamad", "new", "newer");
        newer.applied = true;
        reading.record_app(&app("elohim", true, vec![newer]), true);
        assert_eq!(reading.contract_wanted("elohim"), Some("newer".to_string()));
        let contract = StatementContract {
            contract_version: 1,
            statements: vec![StatementRow {
                statement: "accepted-content-head".to_string(),
                form: "elohim:accepted-content-head:v4".to_string(),
                issues: true,
                accepts_new: true,
                honors_historical: true,
            }],
        };
        reading.record_contract("newer".to_string(), &Ok(contract.clone()));
        assert_eq!(reading.contract_wanted("elohim"), None);
        assert_eq!(reading.summary().statement_contract, Some(contract));
    }

    #[test]
    fn the_contract_decodes_snake_case_and_serializes_camel_case() {
        #[derive(serde::Serialize)]
        struct Wire<'a> {
            contract_version: u32,
            statements: Vec<WireRow<'a>>,
        }
        #[derive(serde::Serialize)]
        struct WireRow<'a> {
            statement: &'a str,
            form: &'a str,
            issues: bool,
            accepts_new: bool,
            honors_historical: bool,
        }
        let bytes = rmp_serde::to_vec_named(&Wire {
            contract_version: 2,
            statements: vec![WireRow {
                statement: "head-delegation",
                form: "v3",
                issues: false,
                accepts_new: false,
                honors_historical: true,
            }],
        })
        .expect("encode");
        let c: StatementContract = rmp_serde::from_slice(&bytes).expect("decode");
        assert_eq!(c.contract_version, 2);
        assert!(c.statements[0].honors_historical);
        let v = serde_json::to_value(&c).expect("json");
        assert_eq!(v["contractVersion"], 2);
        assert_eq!(v["statements"][0]["acceptsNew"], false);
        assert!(v.get("contract_version").is_none());
    }

    #[test]
    fn contract_failures_are_classified_by_name() {
        use crate::error::StorageError;
        assert_eq!(
            classify_contract_error(&StorageError::Serialization("bad".into())),
            ContractReadFailure::DecodeFailed
        );
        assert_eq!(
            classify_contract_error(&StorageError::Internal(
                "Attempted to call a zome function that doesn't exist: Zome: content_store Fn statement_contract".into()
            )),
            ContractReadFailure::ExternAbsent
        );
        assert_eq!(
            classify_contract_error(&StorageError::Internal("deadline has elapsed".into())),
            ContractReadFailure::CallFailed
        );
        for f in ContractReadFailure::ALL {
            assert_eq!(ContractReadFailure::from_label(f.label()), Some(f));
        }
    }

    #[test]
    fn bundle_identity_moves_with_coordinators_and_lineage() {
        let dna = zomes(&[("lamad", "uhC0k-A")]);
        let mut coords = BTreeMap::new();
        coords.insert("lamad".to_string(), zomes(&[("content_store", "uhCok-1")]));
        let a = bundle_identity(&dna, &coords);
        assert_eq!(a, bundle_identity(&dna, &coords));
        coords.insert("lamad".to_string(), zomes(&[("content_store", "uhCok-2")]));
        assert_ne!(a, bundle_identity(&dna, &coords));
        assert_ne!(a, bundle_identity(&zomes(&[("lamad", "uhC0k-B")]), &coords));
    }

    #[test]
    fn the_interval_defaults_and_is_floored() {
        assert_eq!(standing_interval(None), Duration::from_secs(300));
        assert_eq!(standing_interval(Some("garbage")), Duration::from_secs(300));
        assert_eq!(standing_interval(Some("5")), Duration::from_secs(30));
        assert_eq!(standing_interval(Some(" 900 ")), Duration::from_secs(900));
    }
}
