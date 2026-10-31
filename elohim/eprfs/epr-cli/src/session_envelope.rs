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
    let absolute_override = ["XDG_CONFIG_HOME", "ELOHIM_DEVICE_KEY_FILE"]
        .iter()
        .any(|k| std::env::var_os(k).is_some_and(|v| Path::new(&v).is_absolute()));
    if !absolute_override {
        return Err("envelope-persistent-config-required: set XDG_CONFIG_HOME to an absolute persistent config directory outside $HOME".into());
    }
    let key = match std::env::var_os("ELOHIM_DEVICE_KEY_FILE").filter(|v| !v.is_empty()) {
        Some(value) => PathBuf::from(value),
        None => home()?,
    };
    // Resolve symlinks as well as lexical .. before deciding whether this is home storage.
    let resolved = resolved_prefix(&key)?;
    let under_home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|p| resolved_prefix(&p))
        .transpose()?
        .is_some_and(|p| resolved.starts_with(p));
    if under_home {
        return Err("envelope-persistent-config-required: set XDG_CONFIG_HOME to an absolute persistent config directory outside $HOME".into());
    }
    home()
}
// Canonicalize existing ancestors too: an absent key under a home symlink must still
// produce the persistent-directory remedy before any state can be created.
fn resolved_prefix(path: &Path) -> Result<PathBuf> {
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
    };
    proposed.validate_proposed(&proc_root).map_err(err)?;
    // Reentrant facet subprocesses (berth fold emission) may already hold a shared
    // lifecycle guard. A fully initialized lazy open is a read, never a lock upgrade.
    {
        let store = Store::shared(&key_path).map_err(err)?;
        if let Some(record) = store.existing(&proposed, &proc_root).map_err(err)? {
            if root
                .join(recall::RECALL_DIR_REL)
                .join(record.id())
                .join("continuation.json")
                .is_file()
            {
                return Ok(record);
            }
        }
    }
    let store = Store::lock(&key_path).map_err(err)?;
    let record = store.open(proposed, &proc_root).map_err(err)?;
    if root
        .join(recall::RECALL_DIR_REL)
        .join(record.id())
        .join("continuation.json")
        .is_file()
    {
        return Ok(record);
    }
    recall::initialize(&root, record.id(), &executor_cid()?).map_err(err)?;
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
fn prepare_berth(id: &str, path: &Path) -> Result<()> {
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
    Ok(())
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
            "envelope-berth-release: {}: {}",
            record.id(),
            String::from_utf8_lossy(&output.stderr)
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
fn close(store: &Store, mut record: Envelope) -> Result<Envelope> {
    if record.closed_at.is_some() {
        return Ok(record);
    }
    berth_path(&record)?;
    // Checkpoint the seal before release. Retry reuses it and never invents a grant/revocation.
    if record.covenant_seal.is_none() {
        let clauses = covenant::show(&record.repository, record.id(), &record.covenant_session())?;
        record.covenant_seal = Some(store.seal_covenant(&clauses).map_err(err)?);
        store.save(&record).map_err(err)?;
    }
    berth(&record, "unmoor")?;
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
/// Command entry point, including observed-owner collection through the explicit close path.
pub fn run(args: &[String]) -> Result<ExitCode> {
    // `epr session stasis ...` is the dialogue-stasis fold; it renders its own output and exits.
    if args.first().map(String::as_str) == Some("stasis") {
        return Ok(crate::session_stasis::run(&args[1..]));
    }
    let verb = args.first().map(String::as_str).unwrap_or("help");
    if args.iter().any(|s| s == "--help") || verb == "help" {
        println!(
            "epr session id|open [--model MODEL]|show|close|gc [--json]|stasis fold|decide|status"
        );
        return Ok(ExitCode::SUCCESS);
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
        prepare_berth(
            &explicit_session.map(Ok).unwrap_or_else(current_id)?,
            &berth_store.ok_or("--store is required")?,
        )?;
        return Ok(ExitCode::SUCCESS);
    }
    // Internal, output-free validation used by the Python adapter while it holds the
    // same shared lifecycle flock. All owner and seal decisions stay in this lookup.
    if verb == "validate" {
        device_scoped_binding(&explicit_session.map(Ok).unwrap_or_else(current_id)?)?;
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
