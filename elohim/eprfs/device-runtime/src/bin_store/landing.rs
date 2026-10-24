use super::{files, source::CommandSpec, Refusal, Release, Result, Store};
use serde_json::{json, Value};
use std::{path::Path, time::Duration};

impl Store {
    fn validate_land(
        &self,
        cid: &str,
        commit: &str,
        repo: &Path,
        revert: bool,
    ) -> Result<(Release, Option<String>, Value)> {
        let record = self
            .record(cid)
            .filter(|_| self.verified(cid))
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
        // Gitlink publication is a property of the containing repository commit.
        // Its pinned tool commit was checked against source_state above.
        let published_commit = if self.config.source["kind"] == "gitlink" {
            self.git(repo, &["rev-parse", "HEAD"])?
        } else {
            commit.to_owned()
        };
        self.require_published(repo, &published_commit)?;
        let old = self.head("current");
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
    /// Promote a verified, clean, descended and healthy candidate. Health runs outside the lock;
    /// both source state and current metadata are revalidated before committing the move.
    pub fn land(&self, cid: &str, commit: &str, repo: &Path, revert: bool) -> Result<Release> {
        let outcome = (|| {
            let (mut record, expected, expected_meta) = {
                let _lock = files::Lock::acquire(&self.root, files::MUTATION_WAIT)?;
                let state = self.validate_land(cid, commit, repo, revert)?;
                files::touch(&self.artifact(cid)?.join(".used"))?;
                state
            };
            self.require_attestation(&mut record, commit, repo, false)?;
            self.health(cid, commit, repo)?;
            let _lock = files::Lock::acquire(&self.root, files::MUTATION_WAIT)?;
            let (mut latest, old, meta) = self.validate_land(cid, commit, repo, revert)?;
            if old != expected || meta != expected_meta {
                return Err(Refusal::new(
                    "current-moved",
                    "committed head moved during health check",
                ));
            }
            latest.attestations.extend(record.attestations);
            latest.ring = if revert { "reverted" } else { "landed" }.into();
            let at = files::now();
            latest.extra.insert("landedAt".into(), json!(at));
            self.save(&latest)?;
            if let Some(previous) = old.as_ref().filter(|old| old.as_str() != cid) {
                files::atomic_link(&self.root.join("previous"), Path::new(previous))?;
            }
            // The lock serializes readers which publish pins; an interrupted metadata pair refuses land.
            files::atomic_link(&self.root.join("current"), Path::new(cid))?;
            files::atomic_json(
                &self.root.join("current.json"),
                &json!({"cid":cid,"commit":commit,"landedAt":at,"previous":old}),
            )?;
            Ok(latest)
        })();
        if let Err(error) = &outcome {
            self.record_refusal(cid, error);
        }
        outcome
    }
    pub(super) fn record_refusal(&self, cid: &str, error: &Refusal) {
        if error.kind == "lock-timeout" {
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
                list.push(json!({"kind":error.kind,"detail":error.detail,"at":files::now()}));
            }
            let _ = self.save(&record);
        }
    }
    fn health(&self, cid: &str, commit: &str, repo: &Path) -> Result<()> {
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
        }
        Ok(())
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
