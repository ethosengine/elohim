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
        self.require_migrated()?;
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
        self.require_migrated()?;
        if let Some(pin) = self.existing_pin(identity) {
            return Ok(pin);
        }
        let _lock = files::Lock::acquire(&self.root, timeout)?;
        self.require_migrated()?;
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
            running_cid: None,
            source: None,
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
    fn is_launcher(&self, path: &Path, exec: bool) -> bool {
        let canonical = path.canonicalize().ok();
        let own = std::env::current_exe()
            .ok()
            .and_then(|p| p.canonicalize().ok());
        if exec && canonical.is_some() && canonical == own {
            return true;
        }
        canonical.is_some()
            && canonical
                == self
                    .root
                    .parent()
                    .map(|p| p.join("bin/bin-launch"))
                    .and_then(|p| p.canonicalize().ok())
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
        self.resolve_mode(bin, identity, env, exec, true)
    }
    pub(super) fn resolve_mode(
        &self,
        bin: Option<&str>,
        identity: Option<&SessionIdentity>,
        env: &dyn Fn(&str) -> Option<String>,
        exec: bool,
        publish: bool,
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
            if self.is_launcher(&path, exec) {
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
            let pin = if !publish {
                match self.existing_pin(identity) {
                    Some(pin) => pin,
                    None => return self.resolve_current(Some(bin)),
                }
            } else {
                match self.pin_with_timeout(
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
                }
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
            running_cid: None,
            source: None,
        };
        let _lock = files::Lock::acquire(&self.root, files::EXEC_WAIT)?;
        self.require_migrated()?;
        files::atomic_json(&self.pin_path(&pin.session)?, &pin)?;
        Ok(Some(pin))
    }
    /// Record the bytes actually executing in the existing PID/start-time daemon lease.
    /// Direct invocations use the same lease as launcher invocations, never a head-table row.
    pub fn record_running_daemon(
        &self,
        executable: &Path,
        pid: u32,
        proc_root: &Path,
        candidate: Option<&Path>,
    ) -> Result<()> {
        let cid = files::file_cid(executable)?;
        let explicit = candidate
            .and_then(|p| p.canonicalize().ok())
            .zip(executable.canonicalize().ok())
            .is_some_and(|(candidate, actual)| candidate == actual);
        let _lock = files::Lock::acquire(&self.root, files::EXEC_WAIT)?;
        self.require_migrated()?;
        let source = if explicit {
            "candidate"
        } else {
            if !self.verified(&cid) {
                return Err(Refusal::new("daemon-release-unverified",
                    "running executable is not a verified release; run epr device arrive, or set EPR_BIN to this exact candidate path for testing"));
            }
            if self.head("current").as_deref() != Some(&cid) {
                return Err(Refusal::new("daemon-not-current",
                    "running release is not current; restart with the device current, or set EPR_BIN to this exact candidate path for testing"));
            }
            "current"
        };
        let start = session::process_start_time(proc_root, pid).ok_or_else(|| {
            Refusal::new(
                "daemon-owner-unavailable",
                "cannot read daemon start time; restore proc access before epr peer serve",
            )
        })?;
        let lease = Pin {
            cid: cid.clone(),
            running_cid: Some(cid),
            source: Some(source.into()),
            session: format!("daemon-{pid}-{start}"),
            pinned_at: files::now(),
            owner_pid: Some(pid),
            owner_start_time: Some(start.to_string()),
            provisional: false,
        };
        files::atomic_json(&self.pin_path(&lease.session)?, &lease)?;
        Ok(())
    }
    /// Read live daemon leases. PID reuse and stale leases never describe a running daemon.
    pub fn live_daemons(&self, proc_root: &Path) -> Result<Vec<Pin>> {
        let directory = self.root.join("sessions");
        let entries = match std::fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        let mut leases = Vec::new();
        for entry in entries {
            let entry = entry?;
            let Some(pin) = files::read_json::<Pin>(&entry.path()) else {
                continue;
            };
            if pin.session.starts_with("daemon-")
                && files::require_cid(&pin.cid).is_ok()
                && pin
                    .owner_pid
                    .zip(pin.owner_start_time.as_deref())
                    .is_some_and(|(pid, start)| {
                        session::process_start_time(proc_root, pid)
                            .is_some_and(|now| now.to_string() == start)
                    })
            {
                leases.push(pin);
            }
        }
        Ok(leases)
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
    let resolution = (|| {
        Ok(if daemon {
            // A daemon follows the device head, including when its caller exported an explicit pin.
            let resolved = store.resolve_current(Some(&bin))?;
            let cid = resolved
                .cid
                .as_deref()
                .ok_or_else(|| Refusal::new("head-absent", "daemon needs a release"))?;
            if !store.verified(cid) {
                return Err(Refusal::new(
                    "identity-mismatch",
                    "daemon release failed the exec floor",
                ));
            }
            resolved
        } else {
            store.resolve(Some(&bin), identity.as_ref(), &files::env, true)?
        })
    })();
    let resolved = match resolution {
        Ok(resolved) => resolved,
        Err(error)
            if error.kind == "head-absent" && store.config.source["kind"] == "repository" =>
        {
            queue_bootstrap()?;
            return Err(Refusal::new("bootstrapping", "retry shortly"));
        }
        Err(error) => return Err(error),
    };
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
    // Identity, custody, and arrival verbs must work before an envelope can exist.
    if tool == "epr"
        && !daemon
        && identity.as_ref().is_some_and(|i| i.session.is_some())
        && !args.first().is_some_and(|a| {
            ["session", "bin", "device", "--version", "--help"]
                .iter()
                .any(|v| a == v)
        })
        && (args.first().is_none_or(|a| a != "actor") || args.get(1).is_some_and(|a| a == "claim"))
    {
        command.env("EPR_SESSION_LAZY", "1");
    }

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

// Reuse the arrival hook's detached, flock-guarded entry; no second build workflow.
fn queue_bootstrap() -> Result<()> {
    let start = files::env("EPR_REPO_ROOT")
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);
    let (root, script) = start
        .ancestors()
        .find_map(|root| {
            let script = root.join("genesis/agentic/native-epr-bootstrap.mjs");
            script.is_file().then_some((root, script))
        })
        .ok_or_else(|| {
            Refusal::new(
                "head-absent",
                "repository bootstrap entry unavailable; set EPR_REPO_ROOT and retry",
            )
        })?;
    let mut child = Command::new("node")
        .arg(script)
        .arg(root)
        .current_dir(root)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|e| {
            Refusal::new(
                "head-absent",
                format!("bootstrap unavailable: {e}; run epr bin bootstrap"),
            )
        })?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
