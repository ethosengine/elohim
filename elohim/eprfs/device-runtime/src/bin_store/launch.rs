use super::{files, Pin, Refusal, Registry, Result, Store};
use crate::session::{self, SessionIdentity, Source};
use serde::Serialize;
use std::{
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

/// Native resolution result. A candidate launch deliberately has no computed CID.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolution {
    /// Real executable path (not the launcher).
    pub path: PathBuf,
    /// candidate, explicit-pin, session-pin, current, or current-lock-timeout.
    pub source: String,
    /// Release identity, absent for unhashed candidate exec.
    pub cid: Option<String>,
}
impl Store {
    pub(super) fn pin_path(&self, key: &str) -> Result<PathBuf> {
        if key.is_empty() {
            return Err(Refusal::new("session-invalid", "session key is empty"));
        }
        // Encode unsafe bytes injectively, rather than collide foo/bar and foo_bar.
        let mut name = String::new();
        for byte in key.bytes() {
            if byte.is_ascii_alphanumeric() || b"_.-".contains(&byte) {
                name.push(byte as char);
            } else {
                name.push_str(&format!("%{byte:02x}"));
            }
        }
        if name.len() > 200 {
            name = eprfs_core::BlobCid::compute_raw(key.as_bytes()).to_string();
        }
        if name == "." || name == ".." {
            name = name.replace('.', "%2e");
        }
        Ok(self.root.join("sessions").join(name))
    }
    pub(super) fn existing_pin(&self, identity: &SessionIdentity) -> Option<Pin> {
        let key = identity.session.as_deref()?;
        let pin: Pin = files::read_json(&self.pin_path(key).ok()?)?;
        (pin.session == key && files::require_cid(&pin.cid).is_ok()).then_some(pin)
    }
    /// Publish an immutable session pin from current; no session means an explicit refusal.
    /// Provisional pins require verified bytes and an unreachable attestation, and never move current.
    pub fn pin(
        &self,
        identity: &SessionIdentity,
        cid: Option<&str>,
        provisional: bool,
        repo: Option<&Path>,
    ) -> Result<Pin> {
        if provisional {
            let cid =
                cid.ok_or_else(|| Refusal::new("provisional-pin", "--provisional requires --cid"))?;
            if !self.verified(cid) {
                return Err(Refusal::new(
                    "candidate-absent",
                    "provisional pin requires verified bytes",
                ));
            }
            let mut record = self
                .record(cid)
                .ok_or_else(|| Refusal::new("candidate-absent", cid))?;
            let commit = record.provenance.source_commit.clone();
            let repo = self.repo_root(repo)?;
            if !self.require_attestation(&mut record, &commit, &repo, true)? {
                return Err(Refusal::new(
                    "provisional-pin",
                    "only unreachable attestation permits a provisional pin",
                ));
            }
        }
        let result = self.pin_with_timeout(identity, cid, provisional, files::MUTATION_WAIT)?;
        if provisional {
            self.record_refusal(
                &result.cid,
                &Refusal::new(
                    "provisional-pin",
                    "declared local pin with unreachable attestation; current unchanged",
                ),
            );
        }
        Ok(result)
    }
    pub(super) fn pin_with_timeout(
        &self,
        identity: &SessionIdentity,
        cid: Option<&str>,
        provisional: bool,
        timeout: Duration,
    ) -> Result<Pin> {
        let key=identity.session.as_deref().filter(|_| identity.source!=Source::None).ok_or_else(|| Refusal::new("session-absent","no declared session, registered runtime ancestor, or controlling tty is present"))?;
        if let Some(pin) = self.existing_pin(identity) {
            return Ok(pin);
        }
        let _lock = files::Lock::acquire(&self.root, timeout)?;
        if let Some(pin) = self.existing_pin(identity) {
            return Ok(pin);
        }
        let cid = cid
            .map(str::to_owned)
            .or_else(|| self.head("current"))
            .ok_or_else(|| Refusal::new("head-absent", "no committed current head"))?;
        files::require_cid(&cid)?;
        if !self.usable(&cid, &self.config.bins[0]) {
            return Err(Refusal::new("head-absent", "current is not executable"));
        }
        let pin = Pin {
            cid,
            session: key.into(),
            pinned_at: files::now(),
            owner_pid: identity.owner_pid,
            owner_start_time: identity.owner_start_time.map(|n| n.to_string()),
            provisional,
        };
        files::atomic_json(&self.pin_path(key)?, &pin)?;
        Ok(pin)
    }
    /// Resolve the device head without consulting a session or override.
    pub fn resolve_current(&self, bin: Option<&str>) -> Result<Resolution> {
        let bin = self.bin_name(bin)?;
        let cid = self
            .head("current")
            .filter(|cid| self.usable(cid, bin))
            .ok_or_else(|| {
                Refusal::new(
                    "head-absent",
                    "store has no executable current head; run epr bin bootstrap",
                )
            })?;
        Ok(Resolution {
            path: self.artifact(&cid)?.join(bin),
            source: "current".into(),
            cid: Some(cid),
        })
    }
    fn bin_name<'a>(&'a self, bin: Option<&'a str>) -> Result<&'a str> {
        let bin = bin.unwrap_or(&self.config.bins[0]);
        if !self.config.bins.iter().any(|b| b == bin) {
            return Err(Refusal::new("binary-unusable", "bin is not registered"));
        }
        Ok(bin)
    }
    fn is_launcher(&self, path: &Path) -> bool {
        let canonical = path.canonicalize().ok();
        canonical.is_some()
            && (canonical
                == self
                    .root
                    .parent()
                    .map(|p| p.join("bin/bin-launch"))
                    .and_then(|p| p.canonicalize().ok())
                || canonical
                    == self
                        .root
                        .parent()
                        .map(|p| p.join("bin").join(&self.tool))
                        .and_then(|p| p.canonicalize().ok()))
    }
    /// Resolve candidate, explicit pin, session pin, then current. When exec is true, no
    /// file hashing or executable identity subprocess runs. Daemons supply no session.
    pub fn resolve(
        &self,
        bin: Option<&str>,
        identity: Option<&SessionIdentity>,
        env: &dyn Fn(&str) -> Option<String>,
        exec: bool,
    ) -> Result<Resolution> {
        let bin = self.bin_name(bin)?;
        if let Some(candidate) = self
            .config
            .candidate_env
            .get(bin)
            .into_iter()
            .flatten()
            .filter_map(|k| env(k))
            .find(|s| !s.trim().is_empty())
        {
            let path = files::absolute(Path::new(candidate.trim()))?;
            if !files::executable(&path) {
                return Err(Refusal::new("binary-unusable", path.display().to_string()));
            }
            if self.is_launcher(&path) {
                return Err(Refusal::new("launcher-alias", path.display().to_string()));
            }
            let path = path.canonicalize()?;
            return Ok(Resolution {
                cid: if exec {
                    None
                } else {
                    Some(files::file_cid(&path)?)
                },
                path,
                source: "candidate".into(),
            });
        }
        if let Some(cid) =
            env(&format!("{}_PIN_CID", self.tool.to_uppercase())).filter(|v| !v.trim().is_empty())
        {
            let cid = cid.trim();
            files::require_cid(cid)?;
            if !self.usable(cid, bin) {
                return Err(Refusal::new(
                    "candidate-absent",
                    "explicit pin is not executable",
                ));
            }
            return Ok(Resolution {
                path: self.artifact(cid)?.join(bin),
                source: "explicit-pin".into(),
                cid: Some(cid.into()),
            });
        }
        if let Some(identity) = identity.filter(|i| i.source != Source::None && i.session.is_some())
        {
            let pin = match self.pin_with_timeout(
                identity,
                None,
                false,
                if exec {
                    files::EXEC_WAIT
                } else {
                    files::MUTATION_WAIT
                },
            ) {
                Ok(pin) => pin,
                Err(e) if exec && e.kind == "lock-timeout" => {
                    if let Some(pin) = self.existing_pin(identity) {
                        eprintln!(
                            "WARNING: store lock timed out after 5s; using existing session pin"
                        );
                        pin
                    } else {
                        let mut current = self.resolve_current(Some(bin))?;
                        current.source = "current-lock-timeout".into();
                        eprintln!("WARNING: store lock timed out after 5s; using current without publishing a session pin");
                        return Ok(current);
                    }
                }
                Err(e) => return Err(e),
            };
            if !self.usable(&pin.cid, bin) {
                return Err(Refusal::new(
                    "candidate-absent",
                    "session pin is not executable",
                ));
            }
            return Ok(Resolution {
                path: self.artifact(&pin.cid)?.join(bin),
                source: "session-pin".into(),
                cid: Some(pin.cid),
            });
        }
        self.resolve_current(Some(bin))
    }
    /// Whether an argv prefix declares a long-running process in the registry.
    pub fn long_running(&self, args: &[String]) -> bool {
        self.config
            .long_running
            .iter()
            .any(|p| !p.is_empty() && args.starts_with(p))
    }
    /// Lease a release to the launcher's own PID and start time, which survive exec.
    pub fn daemon_lease(&self, cid: &str, pid: u32, proc_root: &Path) -> Result<Option<Pin>> {
        files::require_cid(cid)?;
        let Some(start) = session::process_start_time(proc_root, pid) else {
            return Ok(None);
        };
        let pin = Pin {
            cid: cid.into(),
            session: format!("daemon-{pid}-{start}"),
            pinned_at: files::now(),
            owner_pid: Some(pid),
            owner_start_time: Some(start.to_string()),
            provisional: false,
        };
        let _lock = files::Lock::acquire(&self.root, files::EXEC_WAIT)?;
        files::atomic_json(&self.pin_path(&pin.session)?, &pin)?;
        Ok(Some(pin))
    }
    /// Refresh the usage grace for an executable about to run.
    pub fn touch_used(&self, cid: &str) -> Result<()> {
        files::touch(&self.artifact(cid)?.join(".used"))
            .map_err(|e| Refusal::new("usage-lease-failed", e.detail))
    }
}
/// Resolve argv[0] and replace this process with its CID executable. Never returns on success.
pub fn launch(registry: &Registry, invoked: &Path, args: &[String]) -> Result<()> {
    launch_os(
        registry,
        invoked,
        &args
            .iter()
            .map(std::ffi::OsString::from)
            .collect::<Vec<_>>(),
    )
}
/// Execute without converting filesystem arguments to UTF-8.
pub fn launch_os(registry: &Registry, invoked: &Path, args: &[std::ffi::OsString]) -> Result<()> {
    let (tool, bin) = registry.invoked(invoked)?;
    let store = Store::from_environment(&tool, registry)?;
    let daemon = store.config.long_running.iter().any(|prefix| {
        !prefix.is_empty()
            && args.len() >= prefix.len()
            && prefix.iter().zip(args).all(|(p, a)| a == p.as_str())
    });
    let start = Instant::now();
    let identity = (!daemon).then(session::resolve);
    if files::env("EPR_LAUNCH_TIMING").is_some() {
        eprintln!(
            "bin-launch session-resolve-us={}",
            start.elapsed().as_micros()
        );
    }
    let resolved = store.resolve(Some(&bin), identity.as_ref(), &files::env, true)?;
    if let Some(cid) = &resolved.cid {
        store.touch_used(cid)?;
        if daemon {
            let _ = store.daemon_lease(
                cid,
                std::process::id(),
                Path::new(&files::env("EPR_PROC_ROOT").unwrap_or_else(|| "/proc".into())),
            );
        }
    }
    let mut command = Command::new(&resolved.path);
    command
        .arg0(invoked.file_name().unwrap_or_default())
        .args(args);
    if let Ok(path) = Registry::path() {
        command.env("EPR_BIN_TOOLS", &path);
        if files::env("EPR_AGENT_RUNTIMES").is_none() {
            if let Some(parent) = path.parent() {
                let runtimes = parent.join("agent-runtimes.json");
                if runtimes.is_file() {
                    command.env("EPR_AGENT_RUNTIMES", runtimes);
                }
            }
        }
    }
    Err(Refusal::new("exec-failed", command.exec().to_string()))
}
