use super::{files, source::CommandSpec, Refusal, Release, Result, Store};
use serde_json::{json, Value};
use std::{path::Path, time::Duration};

#[derive(Clone)]
/// Expected committed head for one landing attempt.
pub(super) enum Expect {
    /// Allow a caller that does not name a prior; CAS callers use `Prior`.
    Any,
    /// Require the exact prior CID, or no committed head when absent.
    Prior(Option<String>),
}

impl Expect {
    fn check(&self, found: &Option<String>) -> Result<()> {
        let Self::Prior(expected) = self else {
            return Ok(());
        };
        if expected == found {
            return Ok(());
        }
        Err(Refusal::new(
            "current-moved",
            format!(
                "expected {}, found {}",
                expected.as_deref().unwrap_or("none"),
                found.as_deref().unwrap_or("none")
            ),
        ))
    }
}

impl Store {
    fn validate_land(
        &self,
        cid: &str,
        commit: &str,
        repo: &Path,
        revert: bool,
        cached: bool,
    ) -> Result<(Release, Option<String>, Value)> {
        self.require_migrated()?;
        let record = self
            .record(cid)
            .filter(|_| self.admissible(cid))
            .ok_or_else(|| {
                Refusal::new(
                    "candidate-absent",
                    format!("{cid} is not a verified candidate"),
                )
            })?;
        let source = record.source(commit).ok_or_else(|| {
            Refusal::new(
                "commit-mismatch",
                format!("no ingest provenance for {commit}"),
            )
        })?;
        if !source.tree_clean {
            return Err(Refusal::new(
                "dirty-build",
                format!("{cid} was ingested from a dirty tree"),
            ));
        }
        if cached {
            let head = self.git(repo, &["rev-parse", "HEAD"])?;
            if !self.descends(repo, commit, &head)? {
                return Err(Refusal::new(
                    "non-descendant",
                    "cached commit is not an ancestor of checkout HEAD",
                ));
            }
        } else {
            let state = self.source_state(repo)?;
            if state["head"] != commit {
                return Err(Refusal::new(
                    "head-mismatch",
                    format!("checkout HEAD {} does not match {commit}", state["head"]),
                ));
            }
            if state["clean"] != true {
                return Err(Refusal::new("checkout-dirty", repo.display().to_string()));
            }
        }
        // Gitlink publication is a property of the containing repository commit.
        // Its pinned tool commit was checked against source_state above.
        let published_commit = if self.config.source["kind"] == "gitlink" {
            self.git(repo, &["rev-parse", "HEAD"])?
        } else {
            commit.to_owned()
        };
        self.require_published(repo, &published_commit)?;
        // A legacy link without metadata is an interrupted first publication, not a
        // committed head. Admission below still requires explicit verified provenance.
        let old = self
            .root
            .join("current.json")
            .is_file()
            .then(|| self.head("current"))
            .flatten();
        let metadata: Value =
            files::read_json(&self.root.join("current.json")).unwrap_or(Value::Null);
        if let Some(old) = &old {
            if metadata["cid"] != old.as_str() {
                return Err(Refusal::new(
                    "current-metadata-invalid",
                    "current.json does not describe current",
                ));
            }
            if old != cid
                && !revert
                && !self.descends(repo, metadata["commit"].as_str().unwrap_or(""), commit)?
            {
                return Err(Refusal::new(
                    "non-descendant",
                    format!("{commit} does not descend from {}", metadata["commit"]),
                ));
            }
        }
        Ok((record, old, metadata))
    }
    /// Promote a verified, clean, descended and healthy candidate without a caller-supplied prior.
    /// Health runs outside the lock; source state and metadata are revalidated before the move.
    pub fn land(&self, cid: &str, commit: &str, repo: &Path, revert: bool) -> Result<Release> {
        self.land_mode(cid, commit, repo, revert, false, Expect::Any)
    }
    /// Promote a candidate only when the committed head equals `expected`; `None` expects no head.
    pub fn land_expected(
        &self,
        cid: &str,
        commit: &str,
        repo: &Path,
        revert: bool,
        expected: Option<&str>,
    ) -> Result<Release> {
        if let Some(expected) = expected {
            files::require_cid(expected)?;
        }
        self.land_mode(
            cid,
            commit,
            repo,
            revert,
            false,
            Expect::Prior(expected.map(str::to_owned)),
        )
    }
    pub(super) fn land_mode(
        &self,
        cid: &str,
        commit: &str,
        repo: &Path,
        revert: bool,
        cached: bool,
        expect: Expect,
    ) -> Result<Release> {
        let outcome = (|| {
            let (mut record, expected, expected_meta) = {
                let _lock = files::Lock::acquire(&self.root, files::MUTATION_WAIT)?;
                let found = self
                    .root
                    .join("current.json")
                    .is_file()
                    .then(|| self.head("current"))
                    .flatten();
                expect.check(&found)?;
                let state = self.validate_land(cid, commit, repo, revert, cached)?;
                files::touch(&self.artifact(cid)?.join(".used"))?;
                state
            };
            self.require_attestation(&mut record, commit, repo, false)?;
            let health_note = self.health(cid, commit, repo)?;
            let _lock = files::Lock::acquire(&self.root, files::MUTATION_WAIT)?;
            let found = self
                .root
                .join("current.json")
                .is_file()
                .then(|| self.head("current"))
                .flatten();
            expect.check(&found)?;
            let (mut latest, old, meta) = self.validate_land(cid, commit, repo, revert, cached)?;
            if old != expected || meta != expected_meta {
                return Err(Refusal::new(
                    "current-moved",
                    format!(
                        "expected {}, found {}; committed head moved during health check",
                        expected.as_deref().unwrap_or("none"),
                        old.as_deref().unwrap_or("none")
                    ),
                ));
            }
            let has_health_note = health_note.is_some();
            if let Some((kind, detail)) = health_note {
                Self::add_note(&mut latest, kind, detail);
            }
            if old.as_deref() == Some(cid) {
                // Health/admission still run, but this is maintenance, not a landing.
                if has_health_note {
                    self.save(&latest)?;
                }
                self.repair_heads_locked()?;
                return Ok(latest);
            }
            latest.attestations.extend(record.attestations);
            latest.ring = if revert { "reverted" } else { "landed" }.into();
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos();
            let at = format!("{}.{nanos:09}Z", files::now().trim_end_matches('Z'));
            latest.extra.insert("landedAt".into(), json!(at));
            self.save(&latest)?;
            if cached {
                latest.extra.insert("source".into(), json!("cache"));
                self.save(&latest)?;
            }
            let previous = if old.as_deref() == Some(cid) {
                meta["previous"].clone()
            } else {
                json!(old)
            };
            // One rename commits both heads. Symlinks are disposable projections.
            let subject = self
                .git(repo, &["show", "-s", "--format=%s", commit])
                .unwrap_or_default();
            files::atomic_json(
                &self.root.join("current.json"),
                &json!({"cid":cid,"commit":commit,"subject":subject.trim(),"landedAt":at,"previous":previous}),
            )?;
            self.repair_heads_locked()?;
            Ok(latest)
        })();
        if let Err(error) = &outcome {
            self.record_refusal(cid, error);
        }
        outcome
    }
    pub(super) fn head_repair_needed(&self) -> bool {
        if !self.root.join("current.json").exists() {
            return self.root.join("current").symlink_metadata().is_ok();
        }
        ["current", "previous"]
            .iter()
            .any(|name| files::read_link(&self.root.join(name)) != self.head(name))
    }
    /// Repair disposable head links from the atomically committed record.
    pub fn repair_heads(&self) -> Result<()> {
        let _lock = files::Lock::acquire(&self.root, files::MUTATION_WAIT)?;
        self.require_migrated()?;
        self.repair_heads_locked()
    }
    pub(super) fn repair_heads_locked(&self) -> Result<()> {
        if !self.root.join("current.json").exists() {
            return Ok(());
        }
        let record: Value = files::read_json(&self.root.join("current.json")).ok_or_else(|| {
            Refusal::new("current-metadata-invalid", "current.json is unreadable")
        })?;
        files::require_cid(record["cid"].as_str().unwrap_or(""))?;
        for name in ["current", "previous"] {
            let link = self.root.join(name);
            if let Some(cid) = self.head(name) {
                if files::read_link(&link).as_deref() != Some(&cid) {
                    files::atomic_link(&link, Path::new(&cid))?;
                }
            } else if link.symlink_metadata().is_ok() {
                std::fs::remove_file(&link)?;
                files::sync_parent(&link)?;
            }
        }
        Ok(())
    }
    pub(super) fn record_refusal(&self, cid: &str, error: &Refusal) {
        if error.kind == "lock-timeout" || error.kind == "migration-incomplete" {
            return;
        }
        let Ok(_lock) = files::Lock::acquire(&self.root, files::MUTATION_WAIT) else {
            return;
        };
        if let Some(mut record) = self.record(cid) {
            let list = record.extra.entry("refusals").or_insert(json!([]));
            if !list.is_array() {
                *list = json!([]);
            }
            if let Some(list) = list.as_array_mut() {
                // A repeated refusal refreshes its row; the record names each reason once.
                let nanos = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_nanos();
                let at = format!("{}.{nanos:09}Z", files::now().trim_end_matches('Z'));
                match list.iter_mut().find(|row| {
                    row["kind"] == error.kind.as_str() && row["detail"] == error.detail.as_str()
                }) {
                    Some(row) => row["at"] = json!(at),
                    None => list.push(json!({"kind":error.kind,"detail":error.detail,"at":at})),
                }
            }
            let _ = self.save(&record);
        }
    }
    fn add_note(record: &mut Release, kind: &str, detail: &str) {
        let list = record.extra.entry("notes").or_insert(json!([]));
        if !list.is_array() {
            *list = json!([]);
        }
        if let Some(list) = list.as_array_mut() {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos();
            let at = format!("{}.{nanos:09}Z", files::now().trim_end_matches('Z'));
            match list
                .iter_mut()
                .find(|row| row["kind"] == kind && row["detail"] == detail)
            {
                Some(row) => row["at"] = json!(at),
                None => list.push(json!({"kind":kind,"detail":detail,"at":at})),
            }
        }
    }
    fn health(
        &self,
        cid: &str,
        commit: &str,
        repo: &Path,
    ) -> Result<Option<(&'static str, &'static str)>> {
        let dir = self.artifact(cid)?;
        self.identity(&dir, cid, commit)
            .map_err(|e| Refusal::new("health-identity-mismatch", e.detail))?;
        if self.tool == "epr" {
            let mut spec = CommandSpec::new(
                dir.join("epr").to_string_lossy(),
                ["flow", "report", "scope"],
            );
            spec.cwd = Some(repo.into());
            if !self
                .run(&spec)
                .map_err(|e| Refusal::new("health-flow-failed", e.detail))?
                .success
            {
                return Err(Refusal::new(
                    "health-flow-failed",
                    "flow report scope failed",
                ));
            }
            let candidate = dir.join("epr");
            let check_args = vec![
                "peer".to_owned(),
                "serve".to_owned(),
                "--check".to_owned(),
                "--root".to_owned(),
                repo.to_string_lossy().into_owned(),
                "--json".to_owned(),
            ];
            let mut spec = CommandSpec::new(candidate.to_string_lossy(), check_args);
            spec.cwd = Some(repo.into());
            spec.timeout = Duration::from_secs(30);
            spec.env.insert(
                "EPR_BIN".into(),
                Some(candidate.to_string_lossy().into_owned()),
            );
            let checked = self
                .run(&spec)
                .map_err(|error| Refusal::new("health-peer-check-failed", error.detail))?;
            if !checked.success {
                if checked.stderr.contains("unknown peer argument `--check`") {
                    return Ok(Some((
                        "peer-check-unsupported",
                        "candidate does not advertise peer serve --check",
                    )));
                }
                let refusal_kind = serde_json::from_str::<Value>(&checked.stdout)
                    .ok()
                    .and_then(|value| value["kind"].as_str().map(str::to_owned));
                if refusal_kind.as_deref() == Some("not-a-device") {
                    return Ok(Some((
                        "peer-check-not-a-device",
                        "candidate check found no device standing in this checkout",
                    )));
                }
                return Err(Refusal::new(
                    "health-peer-check-failed",
                    "peer serve --check refused the candidate",
                ));
            }
        }
        Ok(None)
    }
    pub(super) fn require_attestation(
        &self,
        record: &mut Release,
        commit: &str,
        repo: &Path,
        provisional: bool,
    ) -> Result<bool> {
        if self.tool != "brit" {
            return Ok(false);
        }
        let declaration = &self.config.source["attestation"];
        let key = serde_json::to_string(&(
            declaration["repo"].as_str().unwrap_or(""),
            commit,
            declaration["check"].as_str().unwrap_or(""),
        ))?;
        let verdict = if let Some(cached) = record.attestations.get(&key) {
            cached.clone()
        } else {
            let script = repo.join("genesis/orchestrator/gate-attest.mjs");
            let mut spec = CommandSpec::new(
                "node",
                [
                    script.to_string_lossy().as_ref(),
                    "--judge",
                    self.config.source["path"].as_str().unwrap_or("elohim/brit"),
                    "--sha",
                    commit,
                ],
            );
            spec.cwd = Some(repo.into());
            spec.timeout = Duration::from_secs(75);
            self.run(&spec)
                .ok()
                .and_then(|o| serde_json::from_str::<Value>(&o.stdout).ok())
                .unwrap_or_else(
                    || json!({"outcome":"pass","tier":"claimed","conclusion":"unreachable"}),
                )
        };
        if verdict["outcome"] == "pass"
            && verdict["tier"] == "witnessed"
            && verdict["conclusion"] == "success"
        {
            record.attestations.insert(key, verdict);
            // Persist the witnessed result even when later health fails; re-land need not ask gh again.
            let _lock = files::Lock::acquire(&self.root, files::MUTATION_WAIT)?;
            if let Some(mut latest) = self.record(&record.cid) {
                latest.attestations.extend(record.attestations.clone());
                self.save(&latest)?;
            }
            return Ok(false);
        }
        let unreachable = verdict["conclusion"] == "unreachable";
        if provisional && unreachable {
            return Ok(true);
        }
        Err(Refusal::new(
            if unreachable {
                "attestation-unreachable"
            } else {
                "attestation-refused"
            },
            if unreachable {
                "attestation is claimed, not witnessed; epr bin pin --tool brit --cid <cid> --provisional keeps a local pin without moving current"
            } else {
                "attestation did not witness successful checks"
            },
        ))
    }
}
