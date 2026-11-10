//! Private device-local session binding. Facet authority stays in its original store.
use crate::{
    bin_store::{files, Refusal, Result},
    session::{self, SessionIdentity, Source},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    time::Duration,
};

/// One harness session generation, bound once and never reopened after closure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Envelope {
    /// Monotonic generation; bindings written before generations existed are generation one.
    #[serde(default = "first_generation")]
    pub generation: u32,
    /// Exact archived predecessor and its untouched covenant seal.
    #[serde(default)]
    pub predecessor: Option<Predecessor>,
    /// P1 identity; its session string is the envelope key.
    pub identity: SessionIdentity,
    /// PID namespace inode observed through the configured proc root at opening.
    #[serde(default)]
    pub owner_pid_namespace: Option<String>,
    /// Boot identity observed through the same proc root; absent evidence preserves owners.
    #[serde(default)]
    pub owner_boot_id: Option<String>,
    /// Public identity read from the existing device key.
    pub device: String,
    /// Standing human from the device roster, never an agent's claim.
    pub steward: String,
    /// Declared model, absent when the runtime supplies none.
    pub model: Option<String>,
    /// Checkout whose facet stores this envelope joins.
    pub repository: PathBuf,
    /// Canonical berth facet store, bound before the first mooring or lease.
    #[serde(default)]
    pub berth_store: Option<PathBuf>,
    /// UTC opening time.
    pub opened_at: String,
    /// UTC closure time, only after release and witness succeed.
    pub closed_at: Option<String>,
    /// CID of the sealed covenant clause view at closure.
    pub covenant_seal: Option<String>,
    /// Content address of the single close witness.
    pub witness: Option<String>,
    /// Detached lock holder armed for this generation; its lock file is held for as long as the
    /// owner lives, so an observer in another pid namespace can read the owner's death.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_holder: Option<OwnerHolder>,
    /// Raw CID of the verified facet-release receipt saved beside the archive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_receipt: Option<String>,
    /// Raw CID of the steward's retirement record; authorization, never death evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retirement: Option<String>,
}

/// The armed owner-holder lock: a file whose exclusive flock is held by a detached process
/// that lives exactly as long as the owner does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnerHolder {
    /// Lock file beside the envelope, `<id>.gen<N>.owner.lock`.
    pub lock: PathBuf,
    /// Generation the holder was armed for; a mismatch never proves death.
    pub generation: u32,
    /// UTC arming time.
    pub armed_at: String,
    /// Inode of the lock file at arming; a replaced file proves nothing.
    pub inode: u64,
}

/// Content-addressed lineage of a private session binding, not a grant of its facets.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Predecessor {
    /// Raw CID over the exact archived record bytes.
    pub record_cid: String,
    /// Predecessor's covenant checkpoint, absent on a legacy closed-only record.
    pub covenant_seal: Option<String>,
}

fn first_generation() -> u32 {
    1
}

impl Envelope {
    /// Private covenant namespace. A slash cannot collide with a valid envelope id.
    pub fn covenant_session(&self) -> String {
        if self.generation == 1 {
            self.id().to_owned()
        } else {
            format!("{}/generations/{}", self.id(), self.generation)
        }
    }
    /// A seal fences new work even when closure has not finished releasing its facets.
    pub fn is_live(&self) -> bool {
        self.closed_at.is_none() && self.covenant_seal.is_none()
    }

    /// The one facet key. Persisted envelopes always have a validated P1 id.
    pub fn id(&self) -> &str {
        self.identity.session.as_deref().unwrap_or_default()
    }
    fn validate_owner(
        &self,
        owner: &SessionIdentity,
        namespace: Option<&str>,
        boot: Option<&str>,
    ) -> Result<()> {
        if owner.session.is_none()
            || owner.owner_pid.is_none()
            || owner.owner_start_time.is_none()
            || self.identity.owner_pid.is_none()
            || self.identity.owner_start_time.is_none()
            || namespace.is_none()
            || boot.is_none()
            || self.owner_pid_namespace.is_none()
            || self.owner_boot_id.is_none()
        {
            return Err(Refusal::new("envelope-owner-unknown", self.id()));
        }
        if self.identity.session != owner.session
            || self.identity.owner_pid != owner.owner_pid
            || self.identity.owner_start_time != owner.owner_start_time
            || self.owner_pid_namespace.as_deref() != namespace
            || self.owner_boot_id.as_deref() != boot
        {
            return Err(Refusal::new("envelope-owner-mismatch", self.id()));
        }
        Ok(())
    }
    /// Verify complete, live ownership evidence before opening any persistent state.
    pub fn validate_proposed(&self, proc_root: &Path) -> Result<()> {
        let (namespace, boot) = owner_context(proc_root);
        let mut observed = self.identity.clone();
        observed.owner_start_time = observed
            .owner_pid
            .and_then(|pid| session::process_start_time(proc_root, pid));
        self.validate_owner(&observed, namespace.as_deref(), boot.as_deref())
    }
    /// Unknown owners are not declared dead. PID reuse is death of the original owner. From a
    /// foreign pid namespace on the same boot only a released holder lock speaks, and it carries
    /// one accepted residual: a holder that died while its owner lives, read by an observer who
    /// cannot see the owner's pid. A live owner re-arms its holder on every `open`.
    pub fn owner_dead(&self, proc_root: &Path) -> bool {
        let (namespace, boot) = owner_context(proc_root);
        match (&self.owner_boot_id, boot) {
            (Some(original), Some(observed)) if *original != observed => return true,
            (Some(_), Some(_)) => {}
            _ => return false,
        }
        match (&self.owner_pid_namespace, &namespace) {
            (None, _) => return false,
            (Some(recorded), Some(observed)) if recorded == observed => {}
            _ => return self.holder_released(),
        }
        let Some((pid, start)) = self.identity.owner_pid.zip(self.identity.owner_start_time) else {
            return false;
        };
        owner_gone(proc_root, pid, start)
    }
    /// The owner is observable in this very namespace on this boot and is provably the same
    /// process: the one case where a steward's word must not retire it.
    pub fn owner_observably_alive(&self, proc_root: &Path) -> bool {
        let (namespace, boot) = owner_context(proc_root);
        let same_boot = matches!((&self.owner_boot_id, &boot), (Some(a), Some(b)) if a == b);
        let same_namespace =
            matches!((&self.owner_pid_namespace, &namespace), (Some(a), Some(b)) if a == b);
        let Some((pid, start)) = self.identity.owner_pid.zip(self.identity.owner_start_time) else {
            return false;
        };
        same_boot && same_namespace && session::process_start_time(proc_root, pid) == Some(start)
    }
    /// True only when the armed holder lock exists unchanged and nobody holds it.
    fn holder_released(&self) -> bool {
        let Some(holder) = &self.owner_holder else {
            return false;
        };
        holder.generation == self.generation && probe_holder(holder) == Some(false)
    }
    /// The holder is armed for this generation and its lock is held now.
    pub fn holder_held(&self) -> bool {
        let Some((pid, start)) = self.identity.owner_pid.zip(self.identity.owner_start_time) else {
            return false;
        };
        self.owner_holder.as_ref().is_some_and(|h| {
            h.generation == self.generation
                && probe_holder(h) == Some(true)
                && holder_watches(&h.lock, pid, start)
        })
    }
}

/// The owner tuple (`pid start`) a holder wrote into its lock file after locking it. A held
/// lock whose tuple differs, or is not written yet, is some other incarnation's holder.
fn holder_watches(lock: &Path, pid: u32, start: u64) -> bool {
    use std::os::unix::fs::OpenOptionsExt;
    let Ok(mut file) = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(lock)
    else {
        return false;
    };
    let mut text = String::new();
    std::io::Read::read_to_string(&mut file, &mut text).is_ok()
        && text.trim() == format!("{pid} {start}")
}

/// Only an absent PID directory or a different start time proves the owner gone. Missing or
/// unreadable stat is unknown, never death.
fn owner_gone(proc_root: &Path, pid: u32, start: u64) -> bool {
    match fs::metadata(proc_root.join(pid.to_string())) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
        Err(_) => false,
        Ok(_) => session::process_start_time(proc_root, pid).is_some_and(|s| s != start),
    }
}

/// Probe a holder lock without waiting: `Some(true)` held, `Some(false)` free, `None` when the
/// file is missing, replaced (symlink or another inode) or unreadable.
fn probe_holder(holder: &OwnerHolder) -> Option<bool> {
    use std::os::unix::fs::OpenOptionsExt;
    let meta = fs::symlink_metadata(&holder.lock).ok()?;
    if !meta.is_file() || meta.ino() != holder.inode {
        return None;
    }
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&holder.lock)
        .ok()?;
    // The opened file, not the path, must be the armed inode.
    if file.metadata().ok()?.ino() != holder.inode {
        return None;
    }
    match rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockShared) {
        Ok(()) => Some(false),
        Err(e) if e == rustix::io::Errno::WOULDBLOCK => Some(true),
        Err(_) => None,
    }
}

/// Hold the owner lock for `generation` until the owner is provably gone, then release it.
/// Takes no other resource and writes nothing but the empty lock file. Refuses when the lock is
/// already held. Death is read exactly as `owner_dead` reads it: unknown keeps holding.
pub fn hold_owner(lock: &Path, pid: u32, start: u64, proc_root: &Path) -> Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    if lock.is_symlink() {
        return Err(Refusal::new(
            "session-invalid",
            "holder lock must not be a symlink",
        ));
    }
    let file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(lock)?;
    // A concurrent probe takes the lock shared for an instant; retry briefly before refusing.
    let mut attempts = 0;
    loop {
        match rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockExclusive) {
            Ok(()) => break,
            Err(e) if e == rustix::io::Errno::WOULDBLOCK && attempts < 20 => {
                attempts += 1;
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(e) if e == rustix::io::Errno::WOULDBLOCK => {
                return Err(Refusal::new(
                    "owner-holder-held",
                    lock.display().to_string(),
                ));
            }
            Err(e) => return Err(std::io::Error::from(e).into()),
        }
    }
    // Bind the lock to the owner incarnation it watches, after locking.
    file.set_len(0)?;
    std::io::Write::write_all(&mut &file, format!("{pid} {start}\n").as_bytes())?;
    while !owner_gone(proc_root, pid, start) {
        std::thread::sleep(Duration::from_secs(1));
    }
    Ok(())
}

/// The holder lock's name beside the envelopes.
pub fn holder_lock(sessions: &Path, id: &str, generation: u32) -> PathBuf {
    sessions.join(format!("{id}.gen{generation}.owner.lock"))
}

/// Spawn the detached holder for `record`'s owner and wait up to two seconds until its lock is
/// observed held. `None` on any failure: an unarmed record behaves exactly as before.
pub fn arm_holder(store: &Store, record: &Envelope, exe: &Path) -> Option<OwnerHolder> {
    use std::os::unix::process::CommandExt;
    let (pid, start) = record
        .identity
        .owner_pid
        .zip(record.identity.owner_start_time)?;
    let lock = holder_lock(&store.directory, record.id(), record.generation);
    let candidate = |inode| OwnerHolder {
        lock: lock.clone(),
        generation: record.generation,
        armed_at: now(),
        inode,
    };
    // Only a holder that watches THIS owner incarnation counts as armed.
    let armed = |inode| {
        let holder = candidate(inode);
        (probe_holder(&holder) == Some(true) && holder_watches(&lock, pid, start)).then_some(holder)
    };
    if let Some(inode) = fs::symlink_metadata(&lock).ok().map(|m| m.ino()) {
        if let Some(holder) = armed(inode) {
            return Some(holder);
        }
    }
    let mut command = std::process::Command::new(exe);
    command
        .args(["session", "hold", "--session", record.id(), "--generation"])
        .arg(record.generation.to_string())
        .args([
            "--owner-pid",
            &pid.to_string(),
            "--owner-start",
            &start.to_string(),
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    // SAFETY: the closure runs between fork and exec and calls only setsid, which is
    // async-signal-safe and touches no Rust-managed state.
    unsafe {
        command.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    let mut child = command.spawn().ok()?;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        if let Some(inode) = fs::symlink_metadata(&lock).ok().map(|m| m.ino()) {
            if let Some(holder) = armed(inode) {
                return Some(holder);
            }
        }
        if child.try_wait().ok().flatten().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    None
}

/// Process evidence for successor admission: only the real `/proc` counts unless a fixture
/// opts in, so a leaked isolation root cannot turn a live predecessor into permission.
pub fn admission_proc_root(proc_root: &Path) -> PathBuf {
    if std::env::var("EPR_UNDER_TEST").as_deref() == Ok("1") {
        proc_root.to_owned()
    } else {
        PathBuf::from("/proc")
    }
}

/// Serialized envelope mutations, independent of CLI, git, and facet implementations.
pub struct Store {
    directory: PathBuf,
    _lock: files::Lock,
}
impl Store {
    /// Lock the sessions directory beside the device key. Never reads or creates a key.
    pub fn lock(device_key: &Path) -> Result<Self> {
        Self::locked(device_key, false, Duration::from_secs(5))
    }
    /// The exclusive guard without waiting; refuses at once when any other guard is held.
    pub fn try_lock(device_key: &Path) -> Result<Self> {
        Self::locked(device_key, false, Duration::ZERO)
    }
    /// Shared lifecycle guard for facet writers; closure takes the exclusive form.
    pub fn shared(device_key: &Path) -> Result<Self> {
        Self::locked(device_key, true, Duration::from_secs(5))
    }
    fn locked(device_key: &Path, shared: bool, wait: Duration) -> Result<Self> {
        let directory = device_key
            .parent()
            .ok_or_else(|| Refusal::new("device-home", "key has no parent"))?
            .join("sessions");
        fs::create_dir_all(&directory)?;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
        let lock = if shared {
            files::Lock::shared(&directory, wait)?
        } else {
            files::Lock::acquire(&directory, wait)?
        };
        Ok(Self {
            directory,
            _lock: lock,
        })
    }
    /// Validate a P1 key without sanitizing it into another session's name.
    pub fn path(&self, id: &str) -> Result<PathBuf> {
        if !is_envelope_id(id) {
            return Err(Refusal::new(
                "session-invalid",
                "P1 id must be a simple local name (1–96 characters)",
            ));
        }
        let path = self.directory.join(format!("{id}.json"));
        if path.is_symlink() {
            return Err(Refusal::new(
                "session-invalid",
                "envelope must not be a symlink",
            ));
        }
        Ok(path)
    }
    /// Read a binding, distinguishing absence from corrupt state.
    pub fn read(&self, id: &str) -> Result<Option<Envelope>> {
        let path = self.path(id)?;
        match fs::read(path) {
            Ok(bytes) => {
                let record: Envelope = serde_json::from_slice(&bytes)?;
                if record.id() != id {
                    return Err(Refusal::new("session-invalid", "record key differs"));
                }
                Ok(Some(record))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    /// One owner-validated lookup for opening, reading and mutating a session's facets.
    pub fn binding(
        &self,
        id: &str,
        owner: &SessionIdentity,
        proc_root: &Path,
    ) -> Result<Option<Envelope>> {
        let record = self.read(id)?;
        let (namespace, boot) = owner_context(proc_root);
        if let Some(record) = &record {
            record.validate_owner(owner, namespace.as_deref(), boot.as_deref())?;
        }
        Ok(record)
    }
    /// Refuse both completed closure and its durable seal checkpoint.
    pub fn require_live(record: &Envelope) -> Result<()> {
        if !record.is_live() {
            return Err(Refusal::new("envelope-closed", record.id()));
        }
        Ok(())
    }
    /// Check an existing binding without creating state, including from a shared guard.
    /// A successor is admitted when the predecessor is closed, its facets are verified released,
    /// and either its owner is proven dead or the steward retired it with successor admission.
    /// RuntimeAncestor guards against accidental takeover by a stray declaration; it does
    /// not authenticate runtime authorship. Runtime and children are one device principal.
    /// No authority is inherited: clauses must be re-offered and berth facets never transfer.
    pub fn existing(&self, proposed: &Envelope, proc_root: &Path) -> Result<Option<Envelope>> {
        // Only successor admission requires real process evidence: a leaked isolation
        // root must not turn a live predecessor into permission to take over its id.
        // Ordinary liveness/GC retain their caller's root. Fixtures opt in explicitly;
        // this is an accident guard, not authentication against the same device user.
        let admission_proc_root = admission_proc_root(proc_root);
        if let Some(old) = self.read(proposed.id())? {
            // A sealed-but-unclosed record is an interrupted close: its facets may still be
            // held, so only completed closure can admit a successor.
            if old.closed_at.is_some()
                && proposed.identity.source == Source::RuntimeAncestor
                && (old.owner_dead(&admission_proc_root) || self.retirement_admits(&old))
            {
                if !self.receipt_verified(&old, "release", old.release_receipt.as_deref()) {
                    return Err(Refusal::new(
                        "envelope-release-unverified",
                        format!(
                            "{}: facet release was never verified; run `epr session retire \
                             --session {} --expect-generation {} --expect-record <cid> --party \
                             {} --quote <words> --admit-successor`",
                            old.id(),
                            old.id(),
                            old.generation,
                            old.steward
                        ),
                    ));
                }
                // A resumed runtime may come back in another checkout of the same device.
                // The successor binds to its own checkout; the predecessor's stays in the
                // archived record. Device and steward are who the session is: never changed.
                if old.device != proposed.device || old.steward != proposed.steward {
                    return Err(Refusal::new(
                        "session-conflict",
                        "successor changes device/steward",
                    ));
                }
                // The exclusive opener repeats admission before archiving or changing the head.
                return Ok(None);
            }
            old.validate_owner(
                &proposed.identity,
                proposed.owner_pid_namespace.as_deref(),
                proposed.owner_boot_id.as_deref(),
            )?;
            Self::require_live(&old)?;
            if old.identity != proposed.identity
                || old.device != proposed.device
                || old.steward != proposed.steward
                || old.repository != proposed.repository
            {
                return Err(Refusal::new(
                    "session-conflict",
                    "session is bound to a different identity/device/checkout",
                ));
            }
            return Ok(Some(old));
        }
        Ok(None)
    }
    /// Directory of one generation's content-addressed records (`<id>.gen/`).
    fn generation_directory(&self, id: &str) -> PathBuf {
        self.directory.join(format!("{id}.gen"))
    }
    fn record_path(&self, id: &str, generation: u32, kind: &str, cid: &str) -> PathBuf {
        let name = if kind == "retirement" {
            format!("{generation}.{kind}.{cid}.json")
        } else {
            format!("{generation}.{kind}.json")
        };
        self.generation_directory(id).join(name)
    }
    /// Save a small JSON record beside the archive and return its raw CID. A symlinked
    /// directory or file is refused; the record is never replaced by different bytes.
    pub fn save_record(
        &self,
        id: &str,
        generation: u32,
        kind: &str,
        value: &serde_json::Value,
    ) -> Result<String> {
        let directory = self.generation_directory(id);
        let bytes = serde_json::to_vec(value)?;
        let cid = eprfs_core::BlobCid::compute_raw(&bytes).to_string();
        let path = self.record_path(id, generation, kind, &cid);
        if directory.is_symlink() || path.is_symlink() {
            return Err(Refusal::new(
                "session-invalid",
                "record must not be a symlink",
            ));
        }
        // A retirement is write-once under its own CID: a superseding one never replaces
        // the record it cites.
        if kind == "retirement" && fs::read(&path).is_ok_and(|held| held == bytes) {
            return Ok(cid);
        }
        files::atomic_bytes(&path, &bytes, 0o600)?;
        Ok(cid)
    }
    /// Read a saved record only when its bytes still hash to `cid`.
    pub fn read_record(
        &self,
        id: &str,
        generation: u32,
        kind: &str,
        cid: &str,
    ) -> Option<serde_json::Value> {
        if !cid.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return None;
        }
        let path = self.record_path(id, generation, kind, cid);
        if path.is_symlink() || self.generation_directory(id).is_symlink() {
            return None;
        }
        let bytes = fs::read(path).ok()?;
        (eprfs_core::BlobCid::compute_raw(&bytes).to_string() == cid)
            .then(|| serde_json::from_slice(&bytes).ok())
            .flatten()
    }
    /// A receipt names this session and generation and still hashes to the recorded CID.
    fn receipt_verified(&self, record: &Envelope, kind: &str, cid: Option<&str>) -> bool {
        cid.and_then(|cid| self.read_record(record.id(), record.generation, kind, cid))
            .is_some_and(|v| v["session"] == record.id() && v["generation"] == record.generation)
    }
    /// The steward's retirement record names this record and explicitly permits a successor.
    fn retirement_admits(&self, record: &Envelope) -> bool {
        record
            .retirement
            .as_deref()
            .and_then(|cid| self.read_record(record.id(), record.generation, "retirement", cid))
            .is_some_and(|v| {
                v["session"] == record.id()
                    && v["generation"] == record.generation
                    && v["admitSuccessor"] == true
            })
    }
    /// Raw CID of the exact bytes of an envelope's current record.
    pub fn record_cid(&self, id: &str) -> Result<Option<String>> {
        match fs::read(self.path(id)?) {
            Ok(bytes) => Ok(Some(eprfs_core::BlobCid::compute_raw(&bytes).to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    /// Bind once per generation under `existing` admission; retain immutable history.
    pub fn open(&self, proposed: Envelope, proc_root: &Path) -> Result<Envelope> {
        self.open_armed(proposed, proc_root, &|_, _| None)
    }
    /// [`Store::open`] that also arms the owner holder: for a new generation before the record
    /// is saved, and for an existing live record re-opened by its own owner whose holder is not
    /// held. A failed arming leaves `owner_holder` as it was.
    pub fn open_armed(
        &self,
        mut proposed: Envelope,
        proc_root: &Path,
        arm: &dyn Fn(&Store, &Envelope) -> Option<OwnerHolder>,
    ) -> Result<Envelope> {
        proposed.validate_proposed(proc_root)?;
        if let Some(mut old) = self.existing(&proposed, proc_root)? {
            if !old.holder_held() {
                if let Some(holder) = arm(self, &old) {
                    old.owner_holder = Some(holder);
                    self.save(&old)?;
                }
            }
            return Ok(old);
        }
        proposed.generation = 1;
        proposed.predecessor = None;
        if let Some(old) = self.read(proposed.id())? {
            proposed.generation = old
                .generation
                .checked_add(1)
                .ok_or_else(|| Refusal::new("envelope-generation-overflow", old.id()))?;
            let bytes = fs::read(self.path(old.id())?)?;
            let directory = self.generation_directory(old.id());
            let archive = directory.join(format!("{}.json", old.generation));
            if directory.is_symlink() || archive.is_symlink() {
                return Err(Refusal::new(
                    "session-invalid",
                    "generation must not be a symlink",
                ));
            }
            // A crash after archival can retry, but can never replace different history.
            match fs::read(&archive) {
                Ok(retained) if retained == bytes => {}
                Ok(_) => return Err(Refusal::new("envelope-generation-conflict", old.id())),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    files::atomic_bytes(&archive, &bytes, 0o600)?;
                }
                Err(e) => return Err(e.into()),
            }
            proposed.predecessor = Some(Predecessor {
                record_cid: eprfs_core::BlobCid::compute_raw(&bytes).to_string(),
                covenant_seal: old.covenant_seal,
            });
        }
        proposed.berth_store = None;
        proposed.closed_at = None;
        proposed.covenant_seal = None;
        proposed.witness = None;
        proposed.owner_holder = None;
        proposed.release_receipt = None;
        proposed.retirement = None;
        proposed.owner_holder = arm(self, &proposed);
        self.save(&proposed)?;
        Ok(proposed)
    }
    /// Persist an updated close checkpoint while holding the same lock as facet release.
    pub fn save(&self, record: &Envelope) -> Result<()> {
        files::atomic_json(&self.path(record.id())?, record)
    }
    /// Retain the exact private covenant view whose CID witnesses closure; this grants nothing.
    pub fn seal_covenant(&self, view: &serde_json::Value) -> Result<String> {
        let bytes = serde_json::to_vec(view)?;
        let cid = eprfs_core::BlobCid::compute_raw(&bytes).to_string();
        files::atomic_bytes(
            &self.directory.join("seals").join(format!("{cid}.json")),
            &bytes,
            0o600,
        )?;
        Ok(cid)
    }

    /// Read a retained seal only when its bytes still hash to `cid`.
    pub fn read_seal(&self, cid: &str) -> Option<serde_json::Value> {
        if !cid.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return None;
        }
        let bytes = fs::read(self.directory.join("seals").join(format!("{cid}.json"))).ok()?;
        (eprfs_core::BlobCid::compute_raw(&bytes).to_string() == cid)
            .then(|| serde_json::from_slice(&bytes).ok())
            .flatten()
    }
    /// The sessions directory beside a device key, without creating or locking anything.
    pub fn sessions_directory(device_key: &Path) -> Result<PathBuf> {
        Ok(device_key
            .parent()
            .ok_or_else(|| Refusal::new("device-home", "key has no parent"))?
            .join("sessions"))
    }

    /// Enumerate envelopes for observed-owner cleanup.
    pub fn records(&self) -> Result<Vec<Envelope>> {
        let mut records = Vec::new();
        for entry in fs::read_dir(&self.directory)? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "json") {
                if let Some(id) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Some(record) = self.read(id)? {
                        records.push(record);
                    }
                }
            }
        }
        Ok(records)
    }
}
/// Choose explicit model, then declared runtime exports. Never infer from process names.
pub fn model(
    explicit: Option<&str>,
    identity: &SessionIdentity,
    registry: &session::Registry,
    env: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    explicit
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            registry
                .runtimes
                .iter()
                .find(|r| r.id == identity.runtime)?
                .model_env
                .iter()
                .filter_map(|key| env(key))
                .map(|s| s.trim().to_owned())
                .find(|s| !s.is_empty())
        })
}
/// Whether `id` can name an envelope: a P1 session id is a simple local name of 1–96
/// characters (ASCII letters, digits, `_`, `.`, `-`; not `.` or `..`). Facet writers key their
/// records by session, so they refuse any other label, bound or not.
#[must_use]
pub fn is_envelope_id(id: &str) -> bool {
    !id.is_empty()
        && id.as_bytes()[0].is_ascii_alphanumeric()
        && id.len() <= 96
        && id != "."
        && id != ".."
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
}

/// UTC timestamp shared with the bin store's durable records.
pub fn now() -> String {
    files::now()
}

/// Observe namespace and boot through the same proc root as session identity resolution.
pub fn owner_context(proc_root: &Path) -> (Option<String>, Option<String>) {
    let namespace = fs::metadata(proc_root.join("self/ns/pid"))
        .ok()
        .map(|m| m.ino().to_string());
    let boot = fs::read_to_string(proc_root.join("sys/kernel/random/boot_id"))
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());
    (namespace, boot)
}
