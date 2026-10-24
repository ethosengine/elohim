use super::{
    files,
    source::{copy_executable, CommandSpec},
    IngestOptions, Refusal, Registry, Result, Store,
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

impl Store {
    /// List commands preceding the native launcher on PATH; mark only stale build links repairable.
    pub fn shadows(&self, path_env: &str) -> Vec<Value> {
        let mut found = Vec::new();
        let Some(base) = self.root.parent() else {
            return found;
        };
        let mut launchers = vec![
            base.join("bin/bin-launch"),
            base.join("bin").join(&self.tool),
        ];
        launchers = launchers
            .into_iter()
            .filter_map(|p| p.canonicalize().ok())
            .collect();
        for name in self.config.bins.iter().chain(self.config.aliases.keys()) {
            let mut before = Vec::new();
            for directory in path_env.split(':').filter(|s| !s.is_empty()) {
                let path = Path::new(directory).join(name);
                if !path.exists() && !path.is_symlink() {
                    continue;
                }
                let canonical = path.canonicalize().ok();
                if canonical.as_ref().is_some_and(|p| launchers.contains(p)) {
                    found.extend(before);
                    break;
                }
                let target = fs::read_link(&path).ok();
                let superseded = target.is_some()
                    && (canonical.is_none()
                        || canonical.as_ref().is_some_and(|p| {
                            p.starts_with("/projects/.cargo-target-pool")
                                || p.starts_with("/tmp/eprfs-gate-target")
                                || p.starts_with(&self.root)
                        }));
                before
                    .push(json!({"name":name,"path":path,"target":target,"superseded":superseded}));
            }
        }
        found
    }
    /// Install the native launcher and registry after verifying current. Preserve plain commands
    /// as dirty candidates, validate installed execution, and restore changed links on failure.
    pub fn install_launcher(
        &self,
        native: &Path,
        destination: &Path,
        registry: &Registry,
        repo: &Path,
    ) -> Result<Value> {
        let head = self
            .head("current")
            .filter(|cid| self.verified(cid))
            .ok_or_else(|| {
                Refusal::new(
                    "unverified-current",
                    "launcher left untouched; run epr bin bootstrap",
                )
            })?;
        if !files::executable(native) {
            return Err(Refusal::new(
                "launcher-absent",
                "build bin-launch and pass --launcher <path>",
            ));
        }
        let destination = files::absolute(destination)?;
        let base = self
            .root
            .parent()
            .ok_or_else(|| Refusal::new("path-invalid", "store has no parent"))?
            .join("bin");
        fs::create_dir_all(&base)?;
        let launcher = base.join("bin-launch");
        // All tools share these files. No caller holds a store lock while entering install;
        // serialize adoption, mutation, validation and rollback as one installation.
        let _install_lock = files::Lock::acquire(&base, files::MUTATION_WAIT)?;
        let adopted = self.adopt_plain(&destination, repo)?;
        let persistent_adopted = if destination == base {
            BTreeMap::new()
        } else {
            self.adopt_plain(&base, repo)?
        };
        // Snapshot every shared installed file before any mutation, including absence.
        let managed = [
            launcher.clone(),
            base.join("bin-tools.json"),
            base.join("agent-runtimes.json"),
        ];
        let snapshots: Vec<_> = managed
            .iter()
            .map(|p| (p.clone(), fs::read(p).ok()))
            .collect();
        let mut changes: Vec<(PathBuf, Option<PathBuf>)> = Vec::new();
        let mut plain_backups: Vec<(PathBuf, Vec<u8>, u32)> = Vec::new();
        let result = (|| {
            if native.canonicalize().ok() != launcher.canonicalize().ok() {
                copy_executable(native, &launcher)?;
            }
            files::atomic_json(&base.join("bin-tools.json"), registry)?;
            let runtimes = repo.join("genesis/agentic/agent-runtimes.json");
            if runtimes.is_file() {
                files::atomic_bytes(
                    &base.join("agent-runtimes.json"),
                    &fs::read(runtimes)?,
                    0o444,
                )?;
            }
            for name in self.config.bins.iter().chain(self.config.aliases.keys()) {
                for path in [base.join(name), destination.join(name)] {
                    if path == launcher {
                        continue;
                    }
                    if path.canonicalize().ok() == Some(launcher.clone()) {
                        continue;
                    }
                    if path.is_file() && !path.is_symlink() {
                        use std::os::unix::fs::PermissionsExt;
                        plain_backups.push((
                            path.clone(),
                            fs::read(&path)?,
                            path.metadata()?.permissions().mode(),
                        ));
                    }
                    let old = fs::read_link(&path).ok().or_else(|| {
                        if path.parent() == Some(base.as_path()) {
                            persistent_adopted.get(name).cloned()
                        } else {
                            adopted.get(name).cloned()
                        }
                    });
                    files::atomic_link(&path, &launcher)?;
                    changes.push((path, old));
                }
            }
            let primary = destination.join(&self.config.bins[0]);
            let mut spec = CommandSpec::new(primary.to_string_lossy(), ["--version"]);
            spec.env.insert("PATH".into(), Some("/usr/bin:/bin".into()));
            spec.env.insert(
                format!("{}_STORE", self.tool.to_uppercase()),
                Some(self.root.to_string_lossy().into()),
            );
            spec.env.insert(
                "EPR_PROC_ROOT".into(),
                Some("/nonexistent/installed-check".into()),
            );
            spec.env.insert("ELOHIM_SESSION_ID".into(), None);
            spec.env.insert(
                "EPR_BIN_TOOLS".into(),
                Some(base.join("bin-tools.json").to_string_lossy().into()),
            );
            spec.env
                .insert(format!("{}_PIN_CID", self.tool.to_uppercase()), None);
            for key in self.config.candidate_env.values().flatten() {
                spec.env.insert(key.clone(), None);
            }
            if !self.run(&spec)?.success {
                return Err(Refusal::new(
                    "launcher-verification-failed",
                    "previous links restored",
                ));
            }
            Ok(())
        })();
        if let Err(error) = result {
            for (path, old) in changes.iter().rev() {
                if let Some(old) = old {
                    files::atomic_link(path, old)?;
                } else if path.is_symlink() {
                    fs::remove_file(path)?;
                }
            }
            for (path, bytes, mode) in plain_backups.iter().rev() {
                files::atomic_bytes(path, bytes, *mode)?;
            }
            for (path, bytes) in snapshots.iter().rev() {
                if let Some(bytes) = bytes {
                    files::atomic_bytes(
                        path,
                        bytes,
                        if path == &launcher { 0o555 } else { 0o600 },
                    )?;
                } else if path.exists() {
                    fs::remove_file(path)?;
                }
            }
            return Err(error);
        }
        for shadow in self.shadows(&files::env("PATH").unwrap_or_default()) {
            if shadow["superseded"] == true {
                if let Some(path) = shadow["path"].as_str() {
                    files::atomic_link(Path::new(path), &launcher)?;
                }
            }
        }
        Ok(
            json!({"launcher":launcher,"head":head,"adopted":adopted,"persistentAdopted":persistent_adopted,"shadows":self.shadows(&files::env("PATH").unwrap_or_default())}),
        )
    }
    fn adopt_plain(&self, destination: &Path, repo: &Path) -> Result<BTreeMap<String, PathBuf>> {
        let plain: BTreeMap<_, _> = self
            .config
            .bins
            .iter()
            .chain(self.config.aliases.keys())
            .map(|name| (name.clone(), destination.join(name)))
            .filter(|(_, p)| p.is_file() && !p.is_symlink())
            .collect();
        let mut adopted = BTreeMap::new();
        if plain.is_empty() {
            return Ok(adopted);
        }
        let current = self
            .resolve_current(None)?
            .path
            .parent()
            .ok_or_else(|| Refusal::new("head-absent", "current directory missing"))?
            .to_owned();
        let stage = files::temporary(&self.root.join("adopt"));
        fs::create_dir_all(&stage)?;
        let result = (|| {
            for (name, path) in &plain {
                let target = self.config.aliases.get(name).unwrap_or(name);
                let mut sums = BTreeMap::new();
                for bin in &self.config.bins {
                    let file = if bin == target {
                        path.clone()
                    } else {
                        plain
                            .get(bin)
                            .cloned()
                            .or_else(|| {
                                self.config
                                    .aliases
                                    .iter()
                                    .find(|(_, t)| *t == bin)
                                    .and_then(|(a, _)| plain.get(a))
                                    .cloned()
                            })
                            .unwrap_or_else(|| current.join(bin))
                    };
                    copy_executable(&file, &stage.join(bin))?;
                    sums.insert(bin.clone(), files::file_cid(&stage.join(bin))?);
                }
                for (alias, target) in &self.config.aliases {
                    sums.insert(alias.clone(), sums[target].clone());
                }
                if sums.len() > 1 {
                    files::atomic_bytes(
                        &stage.join("sums.cbor"),
                        &super::sums_manifest(&sums)?,
                        0o444,
                    )?;
                }
                let release = match self.ingest(
                    &stage,
                    &IngestOptions {
                        repo: Some(repo.into()),
                        source_commit: Some("unknown".into()),
                        tree_clean: Some(false),
                        gate_run: Some("adopted-from-path".into()),
                        closure: false,
                    },
                ) {
                    Ok(release) => release,
                    Err(error) if error.kind == "identity-mismatch" && self.tool == "epr" => {
                        self.preserve_legacy_executable(&stage.join("epr"))?
                    }
                    Err(error) => return Err(error),
                };
                adopted.insert(name.clone(), self.artifact(&release.cid)?.join(target));
            }
            Ok(adopted)
        })();
        let _ = fs::remove_dir_all(stage);
        result
    }
}
