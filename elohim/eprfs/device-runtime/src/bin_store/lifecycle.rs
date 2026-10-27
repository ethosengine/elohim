use super::{
    files,
    source::{self, CommandSpec},
    IngestOptions, Pin, Refusal, Release, Result, Status, Store,
};
use crate::session::{self, SessionIdentity};
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    fs,
    path::Path,
    time::{Duration, UNIX_EPOCH},
};
impl Store {
    /// Reconstruct the status view. Optional lazy pin creation follows normal resolution.
    pub fn status(
        &self,
        identity: &SessionIdentity,
        commit: Option<&str>,
        create_pin: bool,
        env: &dyn Fn(&str) -> Option<String>,
    ) -> Result<Status> {
        if create_pin {
            self.repair_heads()?;
        }
        let head = self.head("current");
        if create_pin && head.is_some() && identity.session.is_some() {
            self.pin(identity, None, false, None)?;
        }
        let resolved = self
            .resolve_mode(None, Some(identity), env, false, create_pin)
            .ok();
        let pin = self.existing_pin(identity).map(|p| p.cid);
        let candidates = self
            .records()
            .into_iter()
            .filter(|r| commit.is_none_or(|c| r.source(c).is_some()))
            .collect();
        Ok(Status {
            tool: self.tool.clone(),
            store: self.root.clone(),
            session: identity.session.clone(),
            session_cid: pin.clone(),
            behind: pin.is_some() && head.is_some() && pin != head,
            verified_current: head.as_ref().is_some_and(|cid| self.verified(cid)),
            head,
            source: resolved
                .as_ref()
                .map_or("unresolved", |r| r.source.as_str())
                .into(),
            resolved_cid: resolved.as_ref().and_then(|r| r.cid.clone()),
            resolved: resolved.map(|r| r.path),
            candidates,
            head_repair_needed: self.head_repair_needed(),
        })
    }
    /// Keep current, previous, every live owner, and fourteen days of ingest/use history.
    pub fn gc(&self, proc_root: &Path, at: Option<u64>) -> Result<Value> {
        let at = at.unwrap_or_else(files::now_seconds);
        let _lock = files::Lock::acquire(&self.root, files::MUTATION_WAIT)?;
        self.require_migrated()?;
        let mut keep: BTreeSet<_> = [self.head("current"), self.head("previous")]
            .into_iter()
            .flatten()
            .collect();
        let records = self.records();
        let recent = |seconds| at.saturating_sub(seconds) < files::GRACE;
        for record in &records {
            let used = self
                .artifact(&record.cid)?
                .join(".used")
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok());
            if files::parse_time(&record.provenance.ingested_at).is_some_and(recent)
                || used.is_some_and(|d| recent(d.as_secs()))
            {
                keep.insert(record.cid.clone());
            }
        }
        let mut removed_sessions = Vec::new();
        for path in files::children(&self.root.join("sessions")) {
            let pin: Option<Pin> = files::read_json(&path);
            let retain = pin.as_ref().is_some_and(|pin| {
                if files::require_cid(&pin.cid).is_err() {
                    return false;
                }
                match (pin.owner_pid, pin.owner_start_time.as_deref()) {
                    (Some(pid), Some(start)) => session::process_start_time(proc_root, pid)
                        .is_some_and(|now| now.to_string() == start),
                    _ => files::parse_time(&pin.pinned_at).is_some_and(recent),
                }
            });
            if retain {
                if let Some(pin) = pin {
                    keep.insert(pin.cid);
                }
            } else {
                fs::remove_file(&path)?;
                removed_sessions.push(
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
        // slice B: cross-device custody must add publisher/second-device obligations to keep here.
        let mut removed = Vec::new();
        for record in records {
            if !keep.contains(&record.cid) {
                fs::remove_dir_all(self.artifact(&record.cid)?)?;
                removed.push(record.cid);
            }
        }
        removed.sort();
        removed_sessions.sort();
        Ok(json!({"kept":keep,"removed":removed,"removedSessions":removed_sessions}))
    }
    /// Seed a store locally before attempting builds or downloads. A verified current needs no network.
    pub fn bootstrap(&self, repo: &Path, closure: Option<&Path>) -> Result<Release> {
        self.repair_heads()?;
        if let Some(cid) = self
            .head("current")
            // Admissible, not merely verified: a same-commit epr-only current would otherwise
            // stand forever once the declaration adds bin-launch.
            .filter(|c| self.root.join("current.json").is_file() && self.admissible(c))
        {
            return self
                .record(&cid)
                .ok_or_else(|| Refusal::new("unverified-current", "missing current record"));
        }
        if self.config.source["kind"] == "repository" {
            let head = self.git(repo, &["rev-parse", "HEAD"])?;
            let mut eligible = Vec::new();
            for record in self
                .records()
                .into_iter()
                .filter(|r| self.admissible(&r.cid))
            {
                for source in std::iter::once(&record.provenance).chain(&record.sources) {
                    let commit = &source.source_commit;
                    if source.tree_clean
                        && self.require_published(repo, commit).is_ok()
                        && self.descends(repo, commit, &head)?
                    {
                        let distance = self
                            .git(repo, &["rev-list", "--count", &format!("{commit}..{head}")])
                            .ok()
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or(u64::MAX);
                        eligible.push((distance, commit.clone(), record.cid.clone()));
                    }
                }
            }
            eligible.sort();
            eligible.dedup();
            for (_, commit, cid) in eligible {
                if let Ok(record) = self.land_mode(&cid, &commit, repo, false, true) {
                    return Ok(record);
                }
            }
        }
        let state = self.source_state(repo)?;
        if state["clean"] != true {
            return Err(Refusal::new(
                "bootstrap-dirty",
                "bootstrap requires clean source inputs",
            ));
        }
        let commit = state["head"]
            .as_str()
            .ok_or_else(|| Refusal::new("source-mismatch", "missing HEAD"))?;
        self.adopt(repo, commit, closure, "bootstrap")
    }
    fn adopt(
        &self,
        repo: &Path,
        commit: &str,
        closure: Option<&Path>,
        gate: &str,
    ) -> Result<Release> {
        self.prepare_declared(
            repo,
            commit,
            closure,
            gate,
            ark_core::declaration::FollowMode::Apply,
            None,
        )
    }
    fn prepare_declared(
        &self,
        repo: &Path,
        commit: &str,
        closure: Option<&Path>,
        gate: &str,
        mode: ark_core::declaration::FollowMode,
        pin: Option<&str>,
    ) -> Result<Release> {
        let mut records = self.records();
        if let Some(pin) = pin {
            records.sort_by_key(|record| record.cid != pin);
        }
        for record in records {
            // Historical closures remain valid custody for existing sessions, but
            // following a changed declaration must supply every declared executable.
            let complete = self
                .config
                .bins
                .iter()
                .all(|bin| record.bins.contains_key(bin));
            if record.source(commit).is_some_and(|s| s.tree_clean)
                && (complete || pin == Some(record.cid.as_str()))
                && self.verified(&record.cid)
            {
                return self.finish_declared(record, commit, repo, mode, pin);
            }
        }
        let mut attempts=vec!["verified current unavailable (epr bin status --tool <tool> --json)".to_owned(),"no verified stored candidate for wanted commit (epr bin status --tool <tool> --commit <sha> --json)".into()];
        if let Some(closure) = closure {
            match self.ingest(
                closure,
                &IngestOptions {
                    repo: Some(repo.into()),
                    source_commit: Some(commit.into()),
                    gate_run: Some(gate.into()),
                    tree_clean: Some(true),
                    closure: true,
                },
            ) {
                Ok(record) => return self.finish_declared(record, commit, repo, mode, pin),
                Err(e) => attempts.push(format!(
                    "local closure: {}; epr bin ingest --tool {} --closure {}",
                    e.kind,
                    self.tool,
                    closure.display()
                )),
            }
        } else {
            attempts.push(
                "no local closure supplied (epr bin ingest --tool <tool> --closure <path>)".into(),
            );
        }
        match self.build_release(repo, commit, gate, &mut attempts, &files::env) {
            Ok(record) => self.finish_declared(record, commit, repo, mode, pin),
            Err(e) => {
                attempts.push(format!("{}: {}", e.kind, e.detail));
                Err(Refusal::new("bootstrap-unavailable", attempts.join("; ")))
            }
        }
    }
    fn finish_declared(
        &self,
        mut record: Release,
        commit: &str,
        repo: &Path,
        mode: ark_core::declaration::FollowMode,
        pin: Option<&str>,
    ) -> Result<Release> {
        if pin.is_some_and(|pin| pin != record.cid) {
            return Err(Refusal::new(
                "release-pin-mismatch",
                "candidate differs from the device manifest pin",
            ));
        }
        if mode.applies() {
            return self.land(&record.cid, commit, repo, false);
        }
        self.require_published(repo, commit)?;
        self.require_attestation(&mut record, commit, repo, false)?;
        self.save(&record)?;
        Ok(record)
    }
    /// Stage and verify an observed head without moving current; canary/apply retain the
    /// existing attestation and health floor before landing. A pin constrains the release.
    pub fn follow_declared(
        &self,
        repo: &Path,
        closure: Option<&Path>,
        follow: &ark_core::declaration::Follow,
    ) -> Result<Value> {
        if follow.mode.applies() && follow.pin.is_none() {
            return self.follow(repo, closure);
        }
        let state = self.source_state(repo)?;
        let commit = state["head"]
            .as_str()
            .ok_or_else(|| Refusal::new("source-mismatch", "no source HEAD"))?;
        if state["clean"] != true {
            return Err(Refusal::new("checkout-dirty", repo.display().to_string()));
        }
        let record = self.prepare_declared(
            repo,
            commit,
            closure,
            "follow",
            follow.mode,
            follow.pin.as_deref(),
        )?;
        Ok(json!({"followed":follow.mode.applies(), "observed":record.cid, "mode":follow.mode}))
    }
    pub(super) fn build_release(
        &self,
        repo: &Path,
        commit: &str,
        gate: &str,
        attempts: &mut Vec<String>,
        env: &dyn Fn(&str) -> Option<String>,
    ) -> Result<Release> {
        let target = self.build_target(repo)?;
        if self.tool == "epr" {
            for offline in [true, false] {
                let mut spec = CommandSpec::new(
                    "cargo",
                    ["build", "--locked", "--bin", "epr", "--bin", "bin-launch"],
                );
                if offline {
                    spec.args.push("--offline".into());
                }
                spec.cwd = Some(repo.join("elohim/eprfs"));
                spec.timeout = Duration::from_secs(1800);
                spec.env = self.build_environment(repo, &target);
                if !offline && env("CARGO_NET_OFFLINE").as_deref() == Some("true") {
                    attempts.push("network build disabled by CARGO_NET_OFFLINE=true (CARGO_NET_OFFLINE=false cargo build --locked --bin epr)".into());
                    continue;
                }
                if self.run(&spec).is_ok_and(|o| o.success) {
                    return self.ingest(
                        &target.join("debug/epr"),
                        &IngestOptions {
                            source_commit: Some(commit.into()),
                            tree_clean: Some(true),
                            gate_run: Some(gate.into()),
                            ..Default::default()
                        },
                    );
                }
                attempts.push(format!(
                    "cargo build {}--locked --bin epr failed",
                    if offline { "--offline " } else { "" }
                ));
            }
            return Err(Refusal::new(
                "build-failed",
                "all local and network builds failed",
            ));
        }
        // Build an exact gitlink checkout, never modify the user's brit submodule checkout.
        fs::create_dir_all(&self.root)?;
        let work = files::temporary(&self.root.join("build"));
        let git_dir = self.brit_git_dir(repo)?;
        let added = self.git_ok(
            &git_dir,
            &[
                "worktree",
                "add",
                "--detach",
                &work.to_string_lossy(),
                commit,
            ],
        );
        if added {
            let built = (|| {
                let build_target = target
                    .join("brit-follow")
                    .join(commit.get(..12).unwrap_or(commit));
                for stage in 0..3 {
                    if stage > 0 && env("CARGO_NET_OFFLINE").as_deref() == Some("true") {
                        attempts.push("network build disabled (CARGO_NET_OFFLINE=false cargo build --locked --profile release-github)".into());
                        break;
                    }
                    let mut spec = CommandSpec::new(
                        "cargo",
                        [
                            "build",
                            "--locked",
                            "--profile",
                            "release-github",
                            "-p",
                            "brit-cli",
                            "--bin",
                            "brit",
                            "-p",
                            "brit-build-ref",
                            "--bin",
                            "brit-build-ref",
                            "-p",
                            "brit-verify",
                            "--bin",
                            "brit-verify",
                            "--no-default-features",
                            "--features",
                            "brit-cli/max-pure",
                        ],
                    );
                    spec.cwd = Some(work.clone());
                    spec.timeout = Duration::from_secs(1800);
                    spec.env = self.build_environment(repo, &build_target);
                    spec.env.insert("CARGO_BUILD_JOBS".into(), Some("1".into()));
                    for key in ["CARGO_REGISTRIES_ELOHIM_TOKEN", "NPM_TOKEN"] {
                        spec.env.insert(key.into(), None);
                    }
                    if stage == 0 {
                        spec.args.push("--offline".into());
                    }
                    if stage == 2 {
                        let credentials = source::registry_credential(env);
                        if credentials.is_empty() {
                            attempts.push("authenticated build unavailable (set CARGO_REGISTRIES_ELOHIM_TOKEN or NPM_TOKEN, then epr bin bootstrap --tool brit)".into());
                            break;
                        }
                        spec.env.extend(credentials);
                    }
                    if self.run(&spec).is_ok_and(|o| o.success) {
                        let release = build_target.join("release-github");
                        let mut sums = std::collections::BTreeMap::new();
                        for bin in &self.config.bins {
                            sums.insert(bin.clone(), files::file_cid(&release.join(bin))?);
                        }
                        for (alias, bin) in &self.config.aliases {
                            sums.insert(alias.clone(), sums[bin].clone());
                        }
                        files::atomic_bytes(
                            &release.join("sums.cbor"),
                            &super::sums_manifest(&sums)?,
                            0o444,
                        )?;
                        return self.ingest(
                            &release,
                            &IngestOptions {
                                source_commit: Some(commit.into()),
                                tree_clean: Some(true),
                                gate_run: Some(gate.into()),
                                ..Default::default()
                            },
                        );
                    }
                    attempts.push(format!(
                        "brit build {} failed (cargo build {}--locked --profile release-github)",
                        ["offline", "anonymous", "authenticated"][stage],
                        if stage == 0 { "--offline " } else { "" }
                    ));
                }
                Err(Refusal::new(
                    "brit-build-failed",
                    "build attempts exhausted",
                ))
            })();
            let _ = self.git_output(
                &git_dir,
                &["worktree", "remove", "--force", &work.to_string_lossy()],
            );
            if built.is_ok() {
                return built;
            }
        } else {
            attempts.push("exact source unavailable (git fetch origin <pin>; git worktree add --detach <dir> <pin>)".into());
        }
        if env("CARGO_NET_OFFLINE").as_deref() == Some("true") {
            return Err(Refusal::new(
                "brit-bytes-unavailable",
                "release download disabled by offline mode",
            ));
        }
        let download = files::temporary(&self.root.join("download"));
        fs::create_dir_all(&download)?;
        let result = (|| {
            let release = format!(
                "{}{}",
                self.config.source["releasePrefix"]
                    .as_str()
                    .unwrap_or("brit-build-"),
                commit.get(..12).unwrap_or(commit)
            );
            let mut spec = CommandSpec::new(
                "gh",
                [
                    "release",
                    "download",
                    &release,
                    "--repo",
                    self.config.source["repo"]
                        .as_str()
                        .unwrap_or("ethosengine/brit"),
                    "--dir",
                    &download.to_string_lossy(),
                ],
            );
            spec.timeout = Duration::from_secs(120);
            if !self.run(&spec).is_ok_and(|o| o.success) {
                return Err(Refusal::new(
                    "brit-bytes-unavailable",
                    format!("gh release download {release} --repo ethosengine/brit failed"),
                ));
            }
            use std::os::unix::fs::PermissionsExt;
            for bin in &self.config.bins {
                fs::set_permissions(download.join(bin), fs::Permissions::from_mode(0o555))?;
            }
            self.ingest(
                &download,
                &IngestOptions {
                    source_commit: Some(commit.into()),
                    tree_clean: Some(true),
                    gate_run: Some(gate.into()),
                    ..Default::default()
                },
            )
        })();
        let _ = fs::remove_dir_all(download);
        result
    }
    /// Follow an already published clean descendant held by this checkout.
    pub fn follow(&self, repo: &Path, closure: Option<&Path>) -> Result<Value> {
        let landed: Value =
            files::read_json(&self.root.join("current.json")).unwrap_or(Value::Null);
        let Some(old) = landed["commit"]
            .as_str()
            .filter(|_| self.head("current").is_some())
        else {
            return Ok(
                json!({"followed":false,"why":"no landed head; bootstrap seeds the first one"}),
            );
        };
        let state = self.source_state(repo)?;
        let commit = state["head"]
            .as_str()
            .ok_or_else(|| Refusal::new("source-mismatch", "no source HEAD"))?;
        let inventory_complete = self
            .head("current")
            .and_then(|cid| self.record(&cid))
            .is_some_and(|record| {
                self.config
                    .bins
                    .iter()
                    .all(|bin| record.bins.contains_key(bin))
            });
        if commit == old && inventory_complete {
            return Ok(json!({"followed":false,"why":"the landed head is this checkout's HEAD"}));
        }
        let head = self.git(repo, &["rev-parse", "HEAD"])?;
        if self.require_published(repo, &head).is_err() {
            if self.tool == "brit" {
                return Err(Refusal::new("unpublished", "HEAD is not published on dev"));
            }
            return Ok(json!({"followed":false,"why":"HEAD is not published on dev"}));
        }
        if !self.descends(repo, old, commit)? {
            if self.tool == "brit" {
                return Err(Refusal::new(
                    "non-descendant",
                    "brit pin does not descend from landed head",
                ));
            }
            return Ok(
                json!({"followed":false,"why":"HEAD does not descend from the landed head"}),
            );
        }
        if self.tool == "epr" && inventory_complete {
            let paths = self.source_paths(repo);
            let mut args = vec!["diff", "--quiet", old, commit, "--"];
            args.extend(paths.iter().map(String::as_str));
            if self.git_ok(repo, &args) {
                return Ok(
                    json!({"followed":false,"why":"no epr source change since the landed head"}),
                );
            }
        }
        if state["clean"] != true {
            return Ok(json!({"followed":false,"why":"epr sources are dirty in this checkout"}));
        }
        let record = self.adopt(repo, commit, closure, "follow")?;
        let mut value = serde_json::to_value(record)?;
        value["followed"] = json!(true);
        Ok(value)
    }
}
