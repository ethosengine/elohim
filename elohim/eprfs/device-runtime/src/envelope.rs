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
    /// Unknown owners are not declared dead. PID reuse is death of the original owner.
    pub fn owner_dead(&self, proc_root: &Path) -> bool {
        let (namespace, boot) = owner_context(proc_root);
        match (&self.owner_boot_id, boot) {
            (Some(original), Some(observed)) if *original != observed => return true,
            (Some(_), Some(_)) => {}
            _ => return false,
        }
        if self.owner_pid_namespace.is_none() || namespace != self.owner_pid_namespace {
            return false;
        }
        let Some((pid, start)) = self.identity.owner_pid.zip(self.identity.owner_start_time) else {
            return false;
        };
        // Only an absent PID directory proves death. Missing/unreadable stat is unknown.
        match fs::metadata(proc_root.join(pid.to_string())) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
            Err(_) => false,
            Ok(_) => session::process_start_time(proc_root, pid).is_some_and(|s| s != start),
        }
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
        Self::locked(device_key, false)
    }
    /// Shared lifecycle guard for facet writers; closure takes the exclusive form.
    pub fn shared(device_key: &Path) -> Result<Self> {
        Self::locked(device_key, true)
    }
    fn locked(device_key: &Path, shared: bool) -> Result<Self> {
        let directory = device_key
            .parent()
            .ok_or_else(|| Refusal::new("device-home", "key has no parent"))?
            .join("sessions");
        fs::create_dir_all(&directory)?;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
        let lock = if shared {
            files::Lock::shared(&directory, Duration::from_secs(5))?
        } else {
            files::Lock::acquire(&directory, Duration::from_secs(5))?
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
    /// A successor is admitted when the predecessor is closed and its owner proven dead.
    /// RuntimeAncestor guards against accidental takeover by a stray declaration; it does
    /// not authenticate runtime authorship. Runtime and children are one device principal.
    /// No authority is inherited: clauses must be re-offered and berth facets never transfer.
    pub fn existing(&self, proposed: &Envelope, proc_root: &Path) -> Result<Option<Envelope>> {
        // Only successor admission requires real process evidence: a leaked isolation
        // root must not turn a live predecessor into permission to take over its id.
        // Ordinary liveness/GC retain their caller's root. Fixtures opt in explicitly;
        // this is an accident guard, not authentication against the same device user.
        let admission_proc_root = if std::env::var("EPR_UNDER_TEST").as_deref() == Ok("1") {
            proc_root
        } else {
            Path::new("/proc")
        };
        if let Some(old) = self.read(proposed.id())? {
            if !old.is_live()
                && old.owner_dead(admission_proc_root)
                && proposed.identity.source == Source::RuntimeAncestor
            {
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
    /// Bind once per generation under `existing` admission; retain immutable history.
    pub fn open(&self, mut proposed: Envelope, proc_root: &Path) -> Result<Envelope> {
        proposed.validate_proposed(proc_root)?;
        if let Some(old) = self.existing(&proposed, proc_root)? {
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
            let directory = self.directory.join(format!("{}.gen", old.id()));
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
