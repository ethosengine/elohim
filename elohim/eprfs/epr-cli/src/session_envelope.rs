//! Session CLI: bind the device-private envelope and join each facet at its authority.
use crate::{
    actor, covenant, device_key,
    flow::{memory::recall, note},
    git, session,
};
use elohim_device_runtime::{
    bin_store,
    envelope::{self, Envelope, Store},
};
use elohim_epr_rea::{FlowRecord, FlowStore, SidecarFlowStore};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

type Result<T> = std::result::Result<T, String>;
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// The P1 key shared by all facets. No private fallback identity is minted.
pub fn current_id() -> Result<String> {
    session::resolve().session.ok_or_else(|| {
        "no P1 session: run inside a registered runtime or declare ELOHIM_SESSION_ID".into()
    })
}
fn home() -> Result<PathBuf> {
    device_key::resolve_path().map_err(err)
}
fn persistent_home() -> Result<PathBuf> {
    resolve_persistence_with(&|name: &str| std::env::var_os(name)).map(|p| p.key)
}

/// Environment markers of a platform whose `$HOME` is wiped on restart (Eclipse Che and
/// DevWorkspace pods). Any one set and non-empty makes a home key ephemeral, declared or not.
pub(crate) const EPHEMERAL_HOME_MARKERS: [&str; 3] = [
    "DEVWORKSPACE_ID",
    "CHE_WORKSPACE_ID",
    "DEVWORKSPACE_NAMESPACE",
];

/// Whether this process runs on a platform that announces a home wiped on restart (a workspace
/// container). Read from the process environment; the same markers the persistence basis uses.
pub(crate) fn on_ephemeral_home_platform() -> bool {
    EPHEMERAL_HOME_MARKERS
        .iter()
        .any(|name| std::env::var_os(name).is_some_and(|value| !value.is_empty()))
}

const PERSISTENT_CONFIG_REQUIRED: &str = "envelope-persistent-config-required: set XDG_CONFIG_HOME to an absolute persistent config directory outside $HOME";

/// The named basis on which this device's envelope store is accepted as persistent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PersistenceBasis {
    /// An absolute `XDG_CONFIG_HOME` or `ELOHIM_DEVICE_KEY_FILE` places the key outside `$HOME`.
    ExplicitMount,
    /// The key lives under `$HOME` on a host with no ephemeral-home marker, and its directory
    /// holds a `persistence.json` the key itself signed for that directory.
    DeclaredDurableHome { declared_at: String },
}

impl PersistenceBasis {
    /// The basis's stable wire name.
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::ExplicitMount => "explicit-mount",
            Self::DeclaredDurableHome { .. } => "declared-durable-home",
        }
    }
}

/// A key path accepted as persistent, with the basis that accepted it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Persistence {
    key: PathBuf,
    basis: PersistenceBasis,
}

impl Persistence {
    /// The key path as the device-key module resolves it (unresolved symlinks kept, so every
    /// store opened from it names the same path `home()` does).
    pub(crate) fn key(&self) -> &Path {
        &self.key
    }
    /// Why the key's directory counts as persistent.
    pub(crate) fn basis(&self) -> &PersistenceBasis {
        &self.basis
    }
}

/// Where the device key resolves, and the facts the persistence decision reads about it.
#[derive(Debug, Clone)]
pub(crate) struct KeyPlacement {
    /// The key path as `device_key::resolve_path_with` names it.
    pub(crate) key: PathBuf,
    /// The key's directory with every existing ancestor symlink-resolved.
    pub(crate) dir: PathBuf,
    /// Whether that resolved directory lies under the resolved `$HOME`.
    pub(crate) under_home: bool,
    /// Whether an absolute `XDG_CONFIG_HOME` or `ELOHIM_DEVICE_KEY_FILE` is set.
    pub(crate) absolute_override: bool,
    /// The first ephemeral-home platform marker set, if any.
    pub(crate) ephemeral_marker: Option<&'static str>,
}

/// Resolve the key and decide whether it is home storage, from an environment lookup.
pub(crate) fn key_placement_with<F>(lookup: &F) -> Result<KeyPlacement>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let non_empty = |name: &str| lookup(name).filter(|v| !v.is_empty());
    let absolute_override = ["XDG_CONFIG_HOME", device_key::DEVICE_KEY_ENV]
        .iter()
        .any(|k| non_empty(k).is_some_and(|v| Path::new(&v).is_absolute()));
    let ephemeral_marker = EPHEMERAL_HOME_MARKERS
        .into_iter()
        .find(|k| non_empty(k).is_some());
    // A relative or in-repository key override is refused here, never resolved against the cwd.
    let key = device_key::resolve_path_with(|name| lookup(name)).map_err(err)?;
    // Resolve symlinks as well as lexical .. before deciding whether this is home storage.
    let resolved = resolved_prefix(&key)?;
    let dir = resolved
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| format!("device key path {} has no directory", key.display()))?;
    let under_home = non_empty("HOME")
        .map(PathBuf::from)
        .map(|p| resolved_prefix(&p))
        .transpose()?
        .is_some_and(|p| resolved.starts_with(p));
    Ok(KeyPlacement {
        key,
        dir,
        under_home,
        absolute_override,
        ephemeral_marker,
    })
}

/// The refusal for a key with no accepted basis; the remedy it names depends on the host.
fn persistent_config_required(placement: &KeyPlacement) -> String {
    match (placement.under_home, placement.ephemeral_marker) {
        (true, Some(marker)) => format!(
            "{PERSISTENT_CONFIG_REQUIRED} ({marker} is set: this host's $HOME is wiped on restart)"
        ),
        (true, None) => format!(
            "{PERSISTENT_CONFIG_REQUIRED}; or, when this device's $HOME survives restarts (a \
             laptop, not a container workspace), declare it once with `epr actor device \
             persistence --declare durable-home`"
        ),
        (false, _) => PERSISTENT_CONFIG_REQUIRED.to_string(),
    }
}

/// The one persistence resolver: accept the key on a named basis or refuse with the reason.
///
/// 1. `ExplicitMount` — an absolute override is set and the resolved key is outside `$HOME`.
/// 2. `DeclaredDurableHome` — the resolved key is under `$HOME`, no ephemeral-home marker is
///    set, and the key's directory holds a declaration that verifies against the loaded key and
///    that directory.
/// 3. Anything else refuses, before any envelope state can be created.
pub(crate) fn resolve_persistence_with<F>(lookup: &F) -> Result<Persistence>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let placement = key_placement_with(lookup)?;
    if !placement.under_home {
        if placement.absolute_override {
            return Ok(Persistence {
                key: placement.key,
                basis: PersistenceBasis::ExplicitMount,
            });
        }
        return Err(persistent_config_required(&placement));
    }
    let declaration = device_key::persistence_path(&placement.dir);
    if !declaration.try_exists().map_err(err)? {
        return Err(persistent_config_required(&placement));
    }
    if let Some(marker) = placement.ephemeral_marker {
        return Err(format!(
            "envelope-home-ephemeral-platform: {marker} is set, so this host's $HOME is wiped on \
             restart and {} cannot make it durable; set XDG_CONFIG_HOME to an absolute \
             persistent config directory outside $HOME",
            declaration.display()
        ));
    }
    let key = device_key::DeviceKey::load(&placement.key).map_err(|e| {
        format!(
            "envelope-persistence-declaration-invalid: the device key at {} cannot be loaded: {e}",
            placement.key.display()
        )
    })?;
    match device_key::read_persistence_declaration(&placement.dir, &key.did_key()) {
        Ok(Some(Ok(verified))) => Ok(Persistence {
            key: placement.key,
            basis: PersistenceBasis::DeclaredDurableHome {
                declared_at: verified.declared_at,
            },
        }),
        Ok(Some(Err(why))) => Err(format!("envelope-persistence-declaration-invalid: {why}")),
        Ok(None) => Err(persistent_config_required(&placement)),
        Err(e) => Err(format!(
            "envelope-persistence-declaration-invalid: {}: {e}",
            declaration.display()
        )),
    }
}

// Canonicalize existing ancestors too: an absent key under a home symlink must still
// produce the persistent-directory remedy before any state can be created.
pub(crate) fn resolved_prefix(path: &Path) -> Result<PathBuf> {
    match path.canonicalize() {
        Ok(path) => Ok(path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let parent = path.parent().ok_or_else(|| err(&e))?;
            let name = path.file_name().ok_or_else(|| err(&e))?;
            Ok(resolved_prefix(parent)?.join(name))
        }
        Err(e) => Err(err(e)),
    }
}
fn root() -> Result<PathBuf> {
    git::discover_root(&std::env::current_dir().map_err(err)?)
        .map_err(err)?
        .canonicalize()
        .map_err(err)
}

/// The CID of the executable actually running: the pinned release record when it is the
/// session pin (no rehash), else the running bytes (an explicit candidate or a cargo build).
pub fn executor_cid() -> Result<String> {
    let registry = match bin_store::Registry::discover() {
        Ok(registry) => registry,
        Err(_) => return bin_store::file_cid(&std::env::current_exe().map_err(err)?).map_err(err),
    };
    let store = bin_store::Store::from_environment("epr", &registry).map_err(err)?;
    let identity = session::resolve();
    let status = store
        .status(&identity, None, false, &|k| std::env::var(k).ok())
        .map_err(err)?;
    if let Some(cid) = status.session_cid {
        let running = std::env::current_exe()
            .map_err(err)?
            .canonicalize()
            .map_err(err)?;
        let pinned = store
            .root()
            .join(&cid)
            .join("epr")
            .canonicalize()
            .map_err(err)?;
        if running != pinned {
            // An explicit candidate or a cargo build is still a real executor: name its bytes.
            // A later change surfaces as recall's own "executor bytes changed" refusal.
            return bin_store::file_cid(&running).map_err(err);
        }
        // A release may be a closure CID. Recall names the executable's raw CID.
        let release = status
            .candidates
            .iter()
            .find(|r| r.cid == cid)
            .ok_or("pinned release record is absent")?;
        return release
            .bins
            .get("epr")
            .cloned()
            .ok_or_else(|| "pinned release has no epr executor".into());
    }
    bin_store::file_cid(&std::env::current_exe().map_err(err)?).map_err(err)
}

/// Bind once per generation and initialize recall under the same runtime id.
/// Succession requires a closed predecessor and proven-dead owner. RuntimeAncestor guards
/// against accidental takeover by stray declarations, not same-principal children.
/// Safety comes from inheriting no authority: re-offer clauses and transfer no berth facets.
pub fn open(model: Option<&str>) -> Result<Envelope> {
    let root = root()?;
    let key_path = persistent_home()?;
    let key = device_key::DeviceKey::load(&key_path).map_err(err)?;
    let steward = match actor::device_state_read_only(&root, &key_path) {
        actor::DeviceState::Standing(s) => s.subject,
        _ => {
            return Err(
                "device has no standing human; epr device arrive names the enrollment cure".into(),
            )
        }
    };
    let identity = session::resolve();
    // A model the caller names on this call (`--model`), as opposed to one a runtime export
    // supplies on every lazy open.
    let declared = model
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    let model = envelope::model(model, &identity, &session::registry(), &|k| {
        std::env::var(k).ok()
    });
    let proc_root = proc_root();
    let (owner_pid_namespace, owner_boot_id) = envelope::owner_context(&proc_root);
    let proposed = Envelope {
        generation: 1,
        predecessor: None,
        identity,
        owner_pid_namespace,
        owner_boot_id,
        device: key.did_key(),
        steward,
        model,
        repository: root.clone(),
        berth_store: None,
        opened_at: envelope::now(),
        closed_at: None,
        covenant_seal: None,
        witness: None,
        owner_holder: None,
        release_receipt: None,
        retirement: None,
    };
    proposed.validate_proposed(&proc_root).map_err(err)?;
    // Reentrant facet subprocesses (berth fold emission) may already hold a shared
    // lifecycle guard. A fully initialized lazy open is a read, never a lock upgrade; only an
    // unarmed holder asks for the exclusive guard, without waiting, and otherwise stays a read.
    //
    // A model the owner declares now and the record does not carry is a write, not a read: it
    // goes to `open_armed` under the exclusive guard, which sets it once or refuses a different
    // one. Returning the record here dropped the declaration and left every later commit
    // unmeasured (2026-11-16, both devices).
    let mut unarmed = None;
    {
        let store = Store::shared(&key_path).map_err(err)?;
        if let Some(record) = store.existing(&proposed, &proc_root).map_err(err)? {
            let declares_model = declared.is_some() && declared != record.model;
            if root
                .join(recall::RECALL_DIR_REL)
                .join(record.id())
                .join("continuation.json")
                .is_file()
                && !declares_model
            {
                if record.holder_held() {
                    return Ok(record);
                }
                unarmed = Some(record);
            }
        }
    }
    let store = match unarmed {
        Some(record) => match Store::try_lock(&key_path) {
            Ok(store) => store,
            Err(_) => return Ok(record),
        },
        None => Store::lock(&key_path).map_err(err)?,
    };
    let exe = std::env::current_exe().ok();
    let record = store
        .open_armed(proposed, &proc_root, &|store, record| {
            envelope::arm_holder(store, record, exe.as_deref()?)
        })
        .map_err(err)?;
    if root
        .join(recall::RECALL_DIR_REL)
        .join(record.id())
        .join("continuation.json")
        .is_file()
    {
        return Ok(record);
    }
    // Still under the exclusive guard: recall reads the session's claim through it.
    recall::initialize(
        &root,
        record.id(),
        &record.covenant_session(),
        &executor_cid()?,
    )
    .map_err(err)?;
    Ok(record)
}

/// A normalized facet destination and the lifecycle lock that protects its writes.
pub struct FacetGuard {
    pub root: PathBuf,
    _store: Option<Store>,
}

/// Read the current authority namespace (covenant clauses and actor claims) under a shared guard.
/// Mutations also use live_guard; closure holds the exclusive store and passes its namespace.
pub fn covenant_scope(id: &str) -> Result<(String, Option<Store>)> {
    if !envelope::is_envelope_id(id) {
        return Err(format!("session-invalid: {id}"));
    }
    let key = home()?;
    if !key.parent().is_some_and(|p| p.join("sessions").is_dir()) {
        return Ok((id.to_owned(), None));
    }
    let store = Store::shared(&key).map_err(err)?;
    let scope = store
        .read(id)
        .map_err(err)?
        .map_or_else(|| id.to_owned(), |record| record.covenant_session());
    Ok((scope, Some(store)))
}

/// The declared runtime of `id` in this device's persistent envelope store, live or closed.
///
/// Carry grants deliberately outlive the recording session. This read therefore checks durable
/// existence and device identity without applying [`Store::require_live`], and preserves the
/// recording runtime after berth no longer lists the ended session.
pub(crate) fn envelope_runtime_on_this_device(id: &str) -> Result<Option<String>> {
    if !envelope::is_envelope_id(id) {
        return Ok(None);
    }
    let key_path = home()?;
    if !key_path
        .parent()
        .is_some_and(|parent| parent.join("sessions").is_dir())
    {
        return Ok(None);
    }
    let device = device_key::DeviceKey::load(&key_path)
        .map_err(err)?
        .did_key();
    let store = Store::shared(&key_path).map_err(err)?;
    Ok(store
        .read(id)
        .map_err(err)?
        .and_then(|record| (record.device == device).then_some(record.identity.runtime)))
}

/// Resolve once, validate that destination, and retain it for the facet writer.
/// Existing standalone facet stores remain usable without an envelope.
pub fn live_guard(id: Option<&str>, destination: &Path) -> Result<FacetGuard> {
    let root = git::discover_root(destination)
        .map_err(err)?
        .canonicalize()
        .map_err(err)?;
    let mut guard = FacetGuard { root, _store: None };
    let Some(id) = id else {
        return Ok(guard);
    };
    // Facet stores key records by this label (brit's covenant refs nest a `/` under another
    // session's prefix), so a label an envelope could never have is refused on every device.
    if !envelope::is_envelope_id(id) {
        return Err(format!(
            "session-invalid: {id}: a session id is a simple local name (1–96 characters)"
        ));
    }
    let key = home()?;
    if !key.parent().is_some_and(|p| p.join("sessions").is_dir()) {
        return Ok(guard);
    }
    let store = Store::shared(&key).map_err(err)?;
    if let Some(record) = store
        .binding(id, &session::resolve(), &proc_root())
        .map_err(err)?
    {
        Store::require_live(&record).map_err(err)?;
        if guard.root != record.repository {
            return Err(format!(
                "envelope-checkout-mismatch: {id}: {}",
                guard.root.display()
            ));
        }
    }
    guard._store = Some(store);
    Ok(guard)
}

/// Owner, liveness and device of a binding, for a facet whose store is device-scoped (berth).
/// Unlike [`live_guard`] it never compares checkouts. A device with no envelope directory is a
/// standalone store and reads as unbound.
fn device_scoped_binding(id: &str) -> Result<Option<Envelope>> {
    let key = home()?;
    if !key.parent().is_some_and(|p| p.join("sessions").is_dir()) {
        return Ok(None);
    }
    let store = Store::shared(&key).map_err(err)?;
    let Some(record) = store
        .binding(id, &session::resolve(), &proc_root())
        .map_err(err)?
    else {
        return Ok(None);
    };
    Store::require_live(&record).map_err(err)?;
    // The device key is the one fact `open` re-proposed that is not tied to a checkout.
    if device_key::DeviceKey::load(&persistent_home()?)
        .map_err(err)?
        .did_key()
        != record.device
    {
        return Err(format!(
            "session-conflict: {id}: bound to another device key"
        ));
    }
    Ok(Some(record))
}

fn pins(record: &Envelope) -> Result<BTreeMap<String, String>> {
    let registry = bin_store::Registry::discover().map_err(err)?;
    let mut pins = BTreeMap::new();
    for tool in registry.tools.keys() {
        let store = bin_store::Store::from_environment(tool, &registry).map_err(err)?;
        let status = store
            .status(&record.identity, None, false, &|_| None)
            .map_err(err)?;
        if let Some(cid) = status.session_cid {
            pins.insert(tool.clone(), cid);
        }
    }
    Ok(pins)
}
fn proc_root() -> PathBuf {
    session::proc_root()
}

// Resolve and validate the reference without ever recreating a lost authority store.
fn berth_path(record: &Envelope) -> Result<Option<PathBuf>> {
    let Some(path) = &record.berth_store else {
        return Ok(None);
    };
    let resolved = path.canonicalize().map_err(|e| {
        format!(
            "envelope-berth-unreachable: {}: {}: {e}",
            record.id(),
            path.display()
        )
    })?;
    if resolved != *path || !resolved.is_dir() || std::fs::read_dir(&resolved).is_err() {
        return Err(format!(
            "envelope-berth-unreachable: {}: {}",
            record.id(),
            path.display()
        ));
    }
    Ok(Some(resolved))
}

/// The launcher's lazy open, for a runtime with no start hook. In the bound checkout (or for an
/// unbound session) this is `open`, which also repairs a missing recall continuation. From any
/// other checkout a bound session is left as it is: re-proposing it there would read that
/// repository's roster and refuse a session that is already open.
pub fn open_lazily() -> Result<()> {
    let id = current_id()?;
    let key = home()?;
    let closed = if key.parent().is_some_and(|p| p.join("sessions").is_dir()) {
        Store::shared(&key)
            .map_err(err)?
            .read(&id)
            .map_err(err)?
            .is_some_and(|record| !record.is_live())
    } else {
        false
    };
    // A closed record needs the exclusive opener's successor admission. Ordinary facet
    // validation never performs a takeover and continues to require the current owner.
    if closed {
        return open(None).map(drop);
    }
    if let Some(record) = device_scoped_binding(&id)? {
        if root().ok().as_deref() != Some(record.repository.as_path()) {
            return Ok(());
        }
    }
    open(None).map(drop)
}

/// Berth's store is device-scoped: a lease taken from any checkout, the main one or a linked
/// worktree, belongs to the same envelope. Only an unbound session is opened here, because
/// `open` proposes the current checkout and would refuse a bound one called from a worktree.
// Called before any shared mutation guard: lazy creation and reference binding both
// need the exclusive lock. The caller then takes the shared lock and revalidates.
fn prepare_berth(id: &str, path: &Path) -> Result<u32> {
    if current_id()? != id {
        return Err(format!("envelope-owner-mismatch: {id}"));
    }
    // Refuse an unusable destination before lazy opening can publish a successor.
    let path = path
        .canonicalize()
        .map_err(|e| format!("envelope-berth-unreachable: {id}: {}: {e}", path.display()))?;
    if !path.is_dir() || std::fs::read_dir(&path).is_err() {
        return Err(format!(
            "envelope-berth-unreachable: {id}: {}",
            path.display()
        ));
    }
    open_lazily()?;
    let store = Store::lock(&home()?).map_err(err)?;
    let mut record = store
        .binding(id, &session::resolve(), &proc_root())
        .map_err(err)?
        .ok_or("session is not open")?;
    Store::require_live(&record).map_err(err)?;
    if let Some(original) = berth_path(&record)? {
        if original != path {
            return Err(format!(
                "envelope-berth-mismatch: {id}: {}",
                original.display()
            ));
        }
    } else {
        record.berth_store = Some(path);
        store.save(&record).map_err(err)?;
    }
    Ok(record.generation)
}

fn berth(record: &Envelope, verb: &str) -> Result<Value> {
    let Some(path) = berth_path(record)? else {
        // No mooring or lease was ever acquired through this envelope.
        return Ok(json!({}));
    };
    let mut command = Command::new("python3");
    let script = record.repository.join("genesis/agentic/bin/berth");
    if verb == "unmoor" {
        // Close/GC already holds the exclusive lifecycle lock and validated the owner
        // (or proved it dead). Cleanup must not re-enter the live-only public adapter.
        command
            .args([
                "-c",
                "import runpy,sys; b=runpy.run_path(sys.argv[1])['Berth'](); b._unmoor(sys.argv[2]); failures=b.take_release_fold_failures(); print(failures, file=sys.stderr) if failures else None; sys.exit(bool(failures))",
            ])
            .arg(script)
            .arg(record.id());
    } else {
        command.arg(script).args([verb, "--session", record.id()]);
    }
    let output = command
        .env("BERTH_DIR", path)
        // Facet observers must not recursively open while closure holds the lock.
        .env("EPR_BIN", std::env::current_exe().map_err(err)?)
        .env_remove("EPR_SESSION_LAZY")
        .output()
        .map_err(err)?;
    if !output.status.success() {
        return Err(format!(
            "envelope-berth-release: {}: {}{}",
            record.id(),
            String::from_utf8_lossy(&output.stderr),
            if verb == "verify-released" {
                String::from_utf8_lossy(&output.stdout).into_owned()
            } else {
                String::new()
            }
        ));
    }
    if verb == "session-show" {
        serde_json::from_slice(&output.stdout).map_err(err)
    } else {
        Ok(Value::Null)
    }
}
fn claims(record: &Envelope) -> Result<Vec<Value>> {
    if !record.repository.join(".eprfs/status/flows.jsonl").exists() {
        return Ok(vec![]);
    }
    let store = SidecarFlowStore::open(&record.repository).map_err(err)?;
    let key = format!("session:{}", record.covenant_session());
    store
        .records()
        .map_err(err)?
        .into_iter()
        .filter_map(|(cid, r)| match r {
            FlowRecord::Commitment(c) if c.resource_spec.classified_as.contains(&key) => Some(
                serde_json::to_value(c)
                    .map(|c| json!({"cid": cid.to_string(), "commitment": c}))
                    .map_err(err),
            ),
            _ => None,
        })
        .collect()
}
/// Read model only: facet data is neither copied into the binding nor treated as new authority.
pub fn view(record: &Envelope) -> Result<Value> {
    let mut current = actor::current_on_device_scoped(
        &record.repository,
        &record.covenant_session(),
        Some(&home()?),
    )
    .map_err(err)?;
    current.session = Some(record.id().to_owned());
    let tier = current
        .claim
        .as_ref()
        .and_then(|c| {
            if c.claimed.starts_with("human:") {
                Some("human".into())
            } else {
                c.claimed.split_once('@').map(|(_, m)| m.to_owned())
            }
        })
        .unwrap_or_else(|| "unknown".into());
    let covenant = covenant::show(&record.repository, record.id(), &record.covenant_session())?;
    Ok(
        json!({"live": record.is_live(), "envelope": record, "berth": berth(record, "session-show")?,
        "claims": claims(record)?, "covenant": covenant, "standing": current,
        "capabilityTier": tier, "recallSession": record.id(), "pinnedTools": pins(record)?}),
    )
}
/// Verify the berth facet is released and save the receipt beside the archive.
fn verify_release(store: &Store, record: &mut Envelope) -> Result<()> {
    if record.release_receipt.is_some() {
        return Ok(());
    }
    // An envelope that never bound a berth store acquired nothing through it.
    if berth_path(record)?.is_some() {
        berth(record, "verify-released")?;
    }
    let receipt = json!({
        "session": record.id(),
        "generation": record.generation,
        "berthStore": record.berth_store,
        "verifiedAt": envelope::now(),
        "verifier": "berth verify-released",
    });
    record.release_receipt = Some(
        store
            .save_record(record.id(), record.generation, "release", &receipt)
            .map_err(err)?,
    );
    store.save(record).map_err(err)?;
    Ok(())
}
fn close(store: &Store, mut record: Envelope) -> Result<Envelope> {
    if record.closed_at.is_some() && record.release_receipt.is_some() {
        return Ok(record);
    }
    if record.closed_at.is_none() {
        berth_path(&record)?;
        // Checkpoint the seal before release. Retry reuses it and never invents a grant/revocation.
        if record.covenant_seal.is_none() {
            let clauses =
                covenant::show(&record.repository, record.id(), &record.covenant_session())?;
            record.covenant_seal = Some(store.seal_covenant(&clauses).map_err(err)?);
            store.save(&record).map_err(err)?;
        }
        berth(&record, "unmoor")?;
    }
    // A legacy closed record has no receipt: only verification repeats, never unmooring.
    verify_release(store, &mut record)?;
    if record.closed_at.is_some() {
        return Ok(record);
    }
    if record.witness.is_none() {
        let reason = format!(
            "session {} closed; opened {}; covenant {}",
            record.covenant_session(),
            record.opened_at,
            record.covenant_seal.as_deref().unwrap_or_default()
        );
        // A crash can happen after the flow append and before the envelope checkpoint. Re-read
        // the existing authority by this stable reason, even if git HEAD moved during the retry.
        let store = SidecarFlowStore::open(&record.repository).map_err(err)?;
        record.witness = store
            .records()
            .map_err(err)?
            .into_iter()
            .find_map(|(cid, r)| match r {
                FlowRecord::Event(e) if e.classified_as.contains(&format!("reason:{reason}")) => {
                    Some(cid.to_string())
                }
                _ => None,
            });
        if record.witness.is_none() {
            let witness = note::note(
                &record.repository,
                ".epr-meta/manifest.md",
                "observation",
                &reason,
                None,
                None,
                &note::NoteActor {
                    as_ref: Some("agent:harness@device-runtime".into()),
                    session: None,
                },
            )
            .map_err(err)?;
            record.witness = Some(witness.record_cid);
        }
    }
    record.closed_at = Some(envelope::now());
    store.save(&record).map_err(err)?;
    Ok(record)
}
/// Predecessor history for a restart successor's offer: the sealed view's held clauses,
/// labelled as history. Absent for a first generation or a predecessor that never sealed.
pub(crate) fn predecessor_history(id: &str) -> Result<Option<Value>> {
    let key = home()?;
    if !key.parent().is_some_and(|p| p.join("sessions").is_dir()) {
        return Ok(None);
    }
    let store = Store::shared(&key).map_err(err)?;
    let Some(record) = store.read(id).map_err(err)? else {
        return Ok(None);
    };
    let Some(seal) = record
        .predecessor
        .as_ref()
        .and_then(|p| p.covenant_seal.clone())
    else {
        return Ok(None);
    };
    if record.generation < 2 {
        return Ok(None);
    }
    let view = store
        .read_seal(&seal)
        .ok_or_else(|| format!("envelope-seal-unreadable: {id}: {seal}"))?;
    let clauses: Vec<Value> = view["clauses"]
        .as_array()
        .map(|all| {
            all.iter()
                .filter(|c| c["honoured"] == true)
                .map(|c| json!({"kind": c["kind"], "quote": c["quote"], "cid": c["cid"]}))
                .collect()
        })
        .unwrap_or_default();
    Ok(Some(json!({
        "generation": record.generation - 1,
        "sealCid": seal,
        "clauses": clauses,
    })))
}

/// Hidden verb spawned by `open`: hold one generation's owner lock until its owner is gone.
fn hold(args: &[String]) -> Result<ExitCode> {
    let (mut session, mut generation, mut pid, mut start) = (None, None, None, None);
    let mut i = 0;
    while i < args.len() {
        let value = args.get(i + 1).ok_or("flag needs a value")?;
        match args[i].as_str() {
            "--session" => session = Some(value.clone()),
            "--generation" => generation = Some(value.parse::<u32>().map_err(err)?),
            "--owner-pid" => pid = Some(value.parse::<u32>().map_err(err)?),
            "--owner-start" => start = Some(value.parse::<u64>().map_err(err)?),
            other => return Err(format!("unexpected hold argument {other}")),
        }
        i += 2;
    }
    let session = session.ok_or("--session is required")?;
    if !envelope::is_envelope_id(&session) {
        return Err(format!("session-invalid: {session}"));
    }
    let sessions = Store::sessions_directory(&home()?).map_err(err)?;
    let lock = envelope::holder_lock(
        &sessions,
        &session,
        generation.ok_or("--generation is required")?,
    );
    envelope::hold_owner(
        &lock,
        pid.ok_or("--owner-pid is required")?,
        start.ok_or("--owner-start is required")?,
        &proc_root(),
    )
    .map_err(err)?;
    Ok(ExitCode::SUCCESS)
}

/// Explicit steward retirement of a stranded session, then the ordinary close. It records
/// authorization, never death: `owner_dead` is untouched.
fn retire(args: &[String]) -> Result<ExitCode> {
    let (mut session, mut generation, mut record_cid) = (None, None, None);
    let (mut party, mut quote, mut admit) = (None, None, false);
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--admit-successor" {
            admit = true;
            i += 1;
            continue;
        }
        let value = args.get(i + 1).ok_or("flag needs a value")?.clone();
        match args[i].as_str() {
            "--session" => session = Some(value),
            "--expect-generation" => generation = Some(value.parse::<u32>().map_err(err)?),
            "--expect-record" => record_cid = Some(value),
            "--party" => party = Some(value),
            "--quote" => quote = Some(value),
            other => return Err(format!("unexpected retire argument {other}")),
        }
        i += 2;
    }
    let id = session.ok_or("--session is required")?;
    let generation = generation.ok_or("--expect-generation is required")?;
    let record_cid = record_cid.ok_or("--expect-record is required")?;
    let party = party.ok_or("--party is required")?;
    let quote = quote.ok_or("--quote is required")?;
    if !envelope::is_envelope_id(&id) {
        return Err(format!("session-invalid: {id}"));
    }
    if quote.trim().is_empty() {
        return Err("retire-quote-blank: the steward's words are required".into());
    }
    let key = home()?;
    if !key.parent().is_some_and(|p| p.join("sessions").is_dir()) {
        return Err(format!("session is not open: {id}"));
    }
    // Never lazy-opens the target: only the exclusive lifecycle guard and a plain read.
    let store = Store::lock(&key).map_err(err)?;
    let mut record = store
        .read(&id)
        .map_err(err)?
        .ok_or_else(|| format!("session is not open: {id}"))?;
    if record.generation != generation {
        return Err(format!(
            "envelope-generation-mismatch: {id}: record is generation {}",
            record.generation
        ));
    }
    if store.record_cid(&id).map_err(err)?.as_deref() != Some(record_cid.as_str()) {
        return Err(format!("envelope-record-mismatch: {id}"));
    }
    let actor = current_id()?;
    // The acting session speaks in ITS covenant namespace (its own generation), read under the
    // lifecycle guard already held: a generation-one claim never authorizes a generation-two actor.
    let actor_scope = store
        .read(&actor)
        .map_err(err)?
        .map_or_else(|| actor.clone(), |acting| acting.covenant_session());
    let standing = covenant::steward_standing(&root()?, &actor_scope)?;
    if party != record.steward || standing.as_deref() != Some(party.as_str()) {
        return Err(format!(
            "retire-party-refused: {party} is not the steward of record with standing here \
             (steward {}, standing {})",
            record.steward,
            standing.as_deref().unwrap_or("none")
        ));
    }
    let observed = envelope::admission_proc_root(&proc_root());
    let dead = record.owner_dead(&observed);
    if !dead && record.owner_observably_alive(&observed) {
        return Err(format!(
            "envelope-owner-live: {id}: the owner is alive in this namespace"
        ));
    }
    let (namespace, boot) = envelope::owner_context(&observed);
    let liveness = if dead {
        "dead"
    } else if record.owner_pid_namespace.is_none() || record.owner_boot_id.is_none() {
        "absent"
    } else if namespace != record.owner_pid_namespace || boot != record.owner_boot_id {
        "foreign-namespace"
    } else {
        "unknown"
    };
    let prior = record
        .retirement
        .as_deref()
        .and_then(|cid| store.read_record(&id, generation, "retirement", cid));
    // A recorded retirement that already admits (or was not asked to) stands; otherwise supersede.
    let reuse = prior
        .as_ref()
        .is_some_and(|p| p["party"] == party.as_str() && (!admit || p["admitSuccessor"] == true));
    if !reuse {
        let retirement = json!({
            "session": id,
            "generation": generation,
            "recordCid": record_cid,
            "party": party,
            "quote": quote,
            "actorSession": actor,
            "observedLiveness": liveness,
            "admitSuccessor": admit,
            "supersedes": record.retirement,
            "at": envelope::now(),
        });
        record.retirement = Some(
            store
                .save_record(&id, generation, "retirement", &retirement)
                .map_err(err)?,
        );
        store.save(&record).map_err(err)?;
    }
    let closed = close(&store, record)?;
    println!("{}", serde_json::to_string_pretty(&closed).map_err(err)?);
    Ok(ExitCode::SUCCESS)
}
/// Command entry point, including observed-owner collection through the explicit close path.
pub fn run(args: &[String]) -> Result<ExitCode> {
    // `epr session stasis ...` is the dialogue-stasis fold; it renders its own output and exits.
    if args.first().map(String::as_str) == Some("stasis") {
        return Ok(crate::session_stasis::run(&args[1..]));
    }
    let verb = args.first().map(String::as_str).unwrap_or("help");
    if args.iter().any(|s| s == "--help") || verb == "help" {
        println!(
            "epr session id|open [--model MODEL]|show|commit-trailers|close|retire|gc [--json]|stasis fold|decide|status"
        );
        return Ok(ExitCode::SUCCESS);
    }
    if verb == "hold" {
        return hold(&args[1..]);
    }
    if verb == "retire" {
        return retire(&args[1..]);
    }
    let mut model = None;
    let mut explicit_session = None;
    let mut berth_store = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => {}
            "--session" if matches!(verb, "validate" | "prepare-berth") => {
                i += 1;
                explicit_session = Some(args.get(i).ok_or("--session needs a value")?.clone());
            }
            "--store" if verb == "prepare-berth" => {
                i += 1;
                berth_store = Some(PathBuf::from(args.get(i).ok_or("--store needs a value")?));
            }
            "--model" if verb == "open" => {
                i += 1;
                model = Some(args.get(i).ok_or("--model needs a value")?.as_str());
            }
            other => return Err(format!("unexpected session argument {other}")),
        }
        i += 1;
    }
    if verb == "prepare-berth" {
        let generation = prepare_berth(
            &explicit_session.map(Ok).unwrap_or_else(current_id)?,
            &berth_store.ok_or("--store is required")?,
        )?;
        println!("{}", json!({"generation": generation}));
        return Ok(ExitCode::SUCCESS);
    }
    // Internal, output-free validation used by the Python adapter while it holds the
    // same shared lifecycle flock. All owner and seal decisions stay in this lookup.
    if verb == "validate" {
        device_scoped_binding(&explicit_session.map(Ok).unwrap_or_else(current_id)?)?;
        return Ok(ExitCode::SUCCESS);
    }
    if verb == "commit-trailers" {
        let identity = session::resolve();
        let registry = session::registry();
        if registry.runtimes.is_empty() {
            return Err("session attribution unmeasured: runtime registry unavailable".into());
        }
        if matches!(
            identity.source,
            session::Source::None | session::Source::TtyLeader
        ) {
            println!("{}", json!({"status": "not-applicable", "trailers": []}));
            return Ok(ExitCode::SUCCESS);
        }
        let record = device_scoped_binding(&current_id()?)?
            .ok_or("session attribution unmeasured: no mounted envelope")?;
        let trailers = crate::session_attribution::decorate(&record, &registry)?;
        println!("{}", json!({"status": "measured", "trailers": trailers}));
        return Ok(ExitCode::SUCCESS);
    }
    let value = if verb == "open" {
        serde_json::to_value(open(model)?).map_err(err)?
    } else {
        let store = Store::lock(&home()?).map_err(err)?;
        if verb == "gc" {
            let proc_root = proc_root();
            let mut closed = Vec::new();
            let mut failed = Vec::new();
            for record in store.records().map_err(err)? {
                if record.closed_at.is_none() && record.owner_dead(&proc_root) {
                    let session = record.id().to_owned();
                    // One unclosable record must not keep every other dead owner's envelope open.
                    match close(&store, record) {
                        Ok(record) => closed.push(record),
                        Err(error) => failed.push(json!({"session": session, "error": error})),
                    }
                }
            }
            let succeeded = failed.is_empty();
            let value = json!({"closed": closed, "failed": failed});
            println!("{}", serde_json::to_string_pretty(&value).map_err(err)?);
            return Ok(if succeeded {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            });
        } else {
            let record = store
                .binding(&current_id()?, &session::resolve(), &proc_root())
                .map_err(err)?
                .ok_or("session is not open; run epr session open")?;
            match verb {
                "show" => view(&record)?,
                "close" => serde_json::to_value(close(&store, record)?).map_err(err)?,
                _ => return Err(format!("unknown session command {verb}")),
            }
        }
    };
    println!("{}", serde_json::to_string_pretty(&value).map_err(err)?);
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::ffi::OsString;

    fn env(pairs: &[(&str, &Path)]) -> impl Fn(&str) -> Option<OsString> {
        let map: HashMap<String, OsString> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.as_os_str().to_owned()))
            .collect();
        move |name| map.get(name).cloned()
    }

    /// A laptop-shaped home: `<base>/home/.config/elohim/device/ed25519.seed`, key minted.
    fn laptop() -> (tempfile::TempDir, PathBuf, device_key::DeviceKey) {
        let base = tempfile::tempdir().unwrap();
        let home = base.path().join("home");
        let key_path = home.join(".config/elohim/device/ed25519.seed");
        let key = device_key::DeviceKey::load_or_generate(&key_path).unwrap();
        (base, home, key)
    }

    fn canonical_dir(key: &device_key::DeviceKey) -> PathBuf {
        key.path().parent().unwrap().canonicalize().unwrap()
    }

    fn declare(key: &device_key::DeviceKey, dir: &Path, into: &Path) {
        let declaration =
            device_key::PersistenceDeclaration::sign(key, dir, "2026-11-16T00:00:00Z").unwrap();
        device_key::write_persistence_declaration(into, &declaration).unwrap();
    }

    #[test]
    fn a_durable_home_with_its_own_declaration_is_accepted() {
        let (_base, home, key) = laptop();
        let dir = canonical_dir(&key);
        declare(&key, &dir, &dir);
        let persistence = resolve_persistence_with(&env(&[("HOME", &home)])).unwrap();
        assert_eq!(
            persistence.basis(),
            &PersistenceBasis::DeclaredDurableHome {
                declared_at: "2026-11-16T00:00:00Z".into()
            }
        );
        assert_eq!(persistence.basis().name(), "declared-durable-home");
        assert_eq!(persistence.key(), key.path());
    }

    #[test]
    fn a_home_key_without_a_declaration_names_the_declare_act() {
        let (_base, home, _key) = laptop();
        let refusal = resolve_persistence_with(&env(&[("HOME", &home)])).unwrap_err();
        assert!(
            refusal.starts_with("envelope-persistent-config-required: set XDG_CONFIG_HOME"),
            "{refusal}"
        );
        assert!(
            refusal.contains("epr actor device persistence --declare durable-home"),
            "{refusal}"
        );
    }

    #[test]
    fn an_ephemeral_home_platform_refuses_even_a_valid_declaration() {
        let (_base, home, key) = laptop();
        let id = Path::new("workspace-1234");
        for marker in EPHEMERAL_HOME_MARKERS {
            // Without a declaration: the long-standing remedy, and no declare suggestion.
            let refusal =
                resolve_persistence_with(&env(&[("HOME", &home), (marker, id)])).unwrap_err();
            assert!(
                refusal.starts_with("envelope-persistent-config-required:"),
                "{refusal}"
            );
            assert!(refusal.contains(marker), "{refusal}");
            assert!(!refusal.contains("--declare"), "{refusal}");
        }
        let dir = canonical_dir(&key);
        declare(&key, &dir, &dir);
        for marker in EPHEMERAL_HOME_MARKERS {
            let refusal =
                resolve_persistence_with(&env(&[("HOME", &home), (marker, id)])).unwrap_err();
            assert!(
                refusal.starts_with("envelope-home-ephemeral-platform:"),
                "{refusal}"
            );
        }
        // An empty marker is not a marker.
        let empty = Path::new("");
        assert!(
            resolve_persistence_with(&env(&[("HOME", &home), ("DEVWORKSPACE_ID", empty)])).is_ok()
        );
    }

    #[test]
    fn a_declaration_from_another_device_is_refused() {
        let (_base, home, key) = laptop();
        let (_other_base, _other_home, other) = laptop();
        let dir = canonical_dir(&key);
        // Signed by another device, even naming this directory.
        declare(&other, &dir, &dir);
        let refusal = resolve_persistence_with(&env(&[("HOME", &home)])).unwrap_err();
        assert!(
            refusal.starts_with("envelope-persistence-declaration-invalid:"),
            "{refusal}"
        );
        assert!(refusal.contains("device mismatch"), "{refusal}");
    }

    #[test]
    fn a_declaration_copied_to_another_directory_is_refused() {
        let (base, _home, key) = laptop();
        let original = canonical_dir(&key);
        declare(&key, &original, &original);
        // The same key and its declaration copied into a second home.
        let second = base.path().join("second");
        let copy = second.join(".config/elohim/device");
        std::fs::create_dir_all(&copy).unwrap();
        for name in ["ed25519.seed", device_key::PERSISTENCE_FILE] {
            std::fs::copy(original.join(name), copy.join(name)).unwrap();
        }
        let refusal = resolve_persistence_with(&env(&[("HOME", &second)])).unwrap_err();
        assert!(
            refusal.starts_with("envelope-persistence-declaration-invalid:"),
            "{refusal}"
        );
        assert!(refusal.contains("directory mismatch"), "{refusal}");
    }

    #[test]
    fn a_tampered_declaration_is_refused() {
        let (_base, home, key) = laptop();
        let dir = canonical_dir(&key);
        declare(&key, &dir, &dir);
        let path = device_key::persistence_path(&dir);
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(
            &path,
            text.replace("2026-11-16T00:00:00Z", "2020-01-01T00:00:00Z"),
        )
        .unwrap();
        let refusal = resolve_persistence_with(&env(&[("HOME", &home)])).unwrap_err();
        assert!(
            refusal.starts_with("envelope-persistence-declaration-invalid:")
                && refusal.contains("signature does not verify"),
            "{refusal}"
        );
        // An added field is tampering too, and so is a file that does not parse.
        std::fs::write(&path, text.replacen('{', "{\"extra\":1,", 1)).unwrap();
        let refusal = resolve_persistence_with(&env(&[("HOME", &home)])).unwrap_err();
        assert!(refusal.contains("does not parse"), "{refusal}");
    }

    #[test]
    fn an_explicit_mount_outside_home_keeps_its_basis() {
        let (base, home, _key) = laptop();
        let xdg = base.path().join("persistent-config");
        let persistence =
            resolve_persistence_with(&env(&[("HOME", &home), ("XDG_CONFIG_HOME", &xdg)])).unwrap();
        assert_eq!(persistence.basis(), &PersistenceBasis::ExplicitMount);
        assert_eq!(persistence.basis().name(), "explicit-mount");
        assert_eq!(
            persistence.key(),
            xdg.join("elohim/device/ed25519.seed").as_path()
        );
        let seed = base.path().join("mounted/ed25519.seed");
        let persistence = resolve_persistence_with(&env(&[
            ("HOME", &home),
            (device_key::DEVICE_KEY_ENV, &seed),
        ]))
        .unwrap();
        assert_eq!(persistence.basis(), &PersistenceBasis::ExplicitMount);
        assert_eq!(persistence.key(), seed.as_path());
        // A relative XDG_CONFIG_HOME is ignored: the key falls back under $HOME and refuses.
        let refusal = resolve_persistence_with(&env(&[
            ("HOME", &home),
            ("XDG_CONFIG_HOME", Path::new("relative/config")),
        ]))
        .unwrap_err();
        assert!(refusal.starts_with("envelope-persistent-config-required:"));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_homes_and_keys_resolve_before_the_home_decision() {
        let (base, home, _key) = laptop();
        // An absolute override that is a symlink INTO $HOME is still home storage.
        let alias = base.path().join("alias");
        std::os::unix::fs::symlink(home.join(".config"), &alias).unwrap();
        let refusal =
            resolve_persistence_with(&env(&[("HOME", &home), ("XDG_CONFIG_HOME", &alias)]))
                .unwrap_err();
        assert!(
            refusal.starts_with("envelope-persistent-config-required:"),
            "{refusal}"
        );
        // A $HOME that is itself a symlink resolves too.
        let home_link = base.path().join("home-link");
        std::os::unix::fs::symlink(&home, &home_link).unwrap();
        let refusal = resolve_persistence_with(&env(&[
            ("HOME", &home_link),
            ("XDG_CONFIG_HOME", &home.join(".config")),
        ]))
        .unwrap_err();
        assert!(
            refusal.starts_with("envelope-persistent-config-required:"),
            "{refusal}"
        );
        // An absent key under a home symlink refuses before any state is created.
        let missing = home_link.join("missing/ed25519.seed");
        let refusal = resolve_persistence_with(&env(&[
            ("HOME", &home),
            (device_key::DEVICE_KEY_ENV, &missing),
        ]))
        .unwrap_err();
        assert!(
            refusal.starts_with("envelope-persistent-config-required:"),
            "{refusal}"
        );
        assert!(!home.join("missing").exists());
        // A declaration is read from the RESOLVED directory, so one reached through the alias
        // verifies against the canonical directory it was signed for.
        let key = device_key::DeviceKey::load(&alias.join("elohim/device/ed25519.seed")).unwrap();
        let dir = home.join(".config/elohim/device").canonicalize().unwrap();
        declare(&key, &dir, &dir);
        let persistence =
            resolve_persistence_with(&env(&[("HOME", &home), ("XDG_CONFIG_HOME", &alias)]))
                .unwrap();
        assert!(matches!(
            persistence.basis(),
            PersistenceBasis::DeclaredDurableHome { .. }
        ));
    }
}
