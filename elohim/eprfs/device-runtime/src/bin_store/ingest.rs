use super::{
    files,
    source::{copy_executable, CommandSpec},
    IngestOptions, Provenance, Refusal, Release, Result, Store,
};
use eprfs_core::BlobCid;
use serde_json::Value;
use std::{collections::BTreeMap, fs, path::Path};

impl Store {
    /// Copy to private staging, verify those exact bytes, then atomically publish a candidate.
    /// Ingest never moves current, even if the candidate passed a gate.
    pub fn ingest(&self, source: &Path, options: &IngestOptions) -> Result<Release> {
        // Closure provenance travels with the bytes; caller overrides never relabel it.
        let closure_options;
        let options = if options.closure {
            let value: Value = files::read_json(&source.join("release.json")).ok_or_else(||
                Refusal::new("closure-provenance-missing", "include release.json beside sums.cbor or SHA256SUMS with sourceCommit, treeClean and gateRun"))?;
            let commit = value["sourceCommit"]
                .as_str()
                .filter(|s| matches!(s.len(), 40 | 64) && s.bytes().all(|b| b.is_ascii_hexdigit()))
                .ok_or_else(|| {
                    Refusal::new(
                        "closure-provenance-invalid",
                        "release.json requires a full sourceCommit",
                    )
                })?;
            let clean = value["treeClean"].as_bool().ok_or_else(|| {
                Refusal::new(
                    "closure-provenance-invalid",
                    "release.json requires treeClean",
                )
            })?;
            let gate = value["gateRun"]
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| {
                    Refusal::new(
                        "closure-provenance-invalid",
                        "release.json requires gateRun",
                    )
                })?;
            let repo = self.repo_root(options.repo.as_deref())?;
            let source_repo = if self.tool == "brit" {
                self.brit_git_dir(&repo)?
            } else {
                repo.clone()
            };
            if !self.git_ok(
                &source_repo,
                &["cat-file", "-e", &format!("{commit}^{{commit}}")],
            ) {
                return Err(Refusal::new("closure-provenance-unverified", "sourceCommit is absent locally; fetch its history before epr bin ingest --closure"));
            }
            closure_options = IngestOptions {
                repo: Some(repo),
                source_commit: Some(commit.into()),
                tree_clean: Some(clean),
                gate_run: Some(gate.into()),
                closure: true,
            };
            &closure_options
        } else {
            options
        };
        let state = if options.source_commit.is_none() || options.tree_clean.is_none() {
            Some(self.source_state(&self.repo_root(options.repo.as_deref())?)?)
        } else {
            None
        };
        let commit = options
            .source_commit
            .clone()
            .or_else(|| state.as_ref()?.get("head")?.as_str().map(str::to_owned))
            .ok_or_else(|| Refusal::new("source-mismatch", "source commit unavailable"))?;
        let clean = options
            .tree_clean
            .unwrap_or_else(|| state.as_ref().is_some_and(|s| s["clean"] == true));
        let _lock = files::Lock::acquire(&self.root, files::MUTATION_WAIT)?;
        self.require_migrated()?;
        let stage = files::temporary(&self.root.join("staging"));
        fs::create_dir(&stage)?;
        let result = (|| {
            for name in &self.config.bins {
                let file = if self.config.bins.len() == 1 && source.is_file() {
                    source.to_owned()
                } else {
                    source.join(name)
                };
                if !files::executable(&file) {
                    return Err(Refusal::new(
                        "binary-unusable",
                        format!("no executable {name}: {}", file.display()),
                    ));
                }
                copy_executable(&file, &stage.join(name))?;
            }
            let mut sums = BTreeMap::new();
            for name in &self.config.bins {
                sums.insert(name.clone(), files::file_cid(&stage.join(name))?);
            }
            // Existing release downloads supply SHA256SUMS. It remains a checksum transport,
            // never an identity; canonical local closures instead carry sums.cbor.
            if source.is_dir() && (options.closure || self.config.bins.len() > 1) {
                self.verify_source_manifest(source, &sums)?;
            }
            for (alias, target) in &self.config.aliases {
                if source.join(alias).is_file()
                    && files::file_cid(&source.join(alias))? != sums[target]
                {
                    return Err(Refusal::new(
                        "checksums-mismatch",
                        format!("alias {alias} differs from {target}"),
                    ));
                }
                copy_executable(&stage.join(target), &stage.join(alias))?;
                sums.insert(alias.clone(), sums[target].clone());
            }
            let cid = self.release_cid(&sums)?;
            let version = self.identity(&stage, &cid, &commit)?;
            let artifact = self.artifact(&cid)?;
            if artifact.exists() {
                for name in sums.keys() {
                    if !files::executable(&artifact.join(name))
                        || files::file_cid(&artifact.join(name)).ok().as_ref() != sums.get(name)
                    {
                        copy_executable(&stage.join(name), &artifact.join(name))?;
                    }
                }
            } else {
                fs::rename(&stage, &artifact)?;
                files::sync_parent(&artifact)?;
            }
            if sums.len() > 1 {
                files::atomic_bytes(
                    &artifact.join("sums.cbor"),
                    &super::sums_manifest(&sums)?,
                    0o444,
                )?;
            }
            let provenance = Provenance {
                source_commit: commit,
                tree_clean: clean,
                gate_run: options.gate_run.clone(),
                ingested_at: files::now(),
            };
            let mut release = self.record(&cid).unwrap_or_else(|| Release {
                cid: cid.clone(),
                rcid: cid.clone(),
                tool: self.tool.clone(),
                bins: sums,
                version,
                provenance: provenance.clone(),
                sources: Vec::new(),
                ring: "dev-context".into(),
                attestations: BTreeMap::new(),
                extra: Default::default(),
            });
            if release.sources.is_empty()
                && release.provenance.ingested_at != provenance.ingested_at
            {
                release.sources.push(release.provenance.clone());
            }
            release.provenance.ingested_at = provenance.ingested_at.clone();
            // The same bytes from the same source and run refresh their row; a retrying
            // bootstrap must not grow the record once per attempt.
            match release.sources.iter_mut().find(|row| {
                row.source_commit == provenance.source_commit
                    && row.tree_clean == provenance.tree_clean
                    && row.gate_run == provenance.gate_run
            }) {
                Some(row) => row.ingested_at = provenance.ingested_at,
                None => release.sources.push(provenance),
            }
            if release.ring != "landed" {
                release.ring = "dev-context".into();
            }
            self.save(&release)?;
            Ok(release)
        })();
        let _ = fs::remove_dir_all(stage);
        result
    }
    fn verify_source_manifest(&self, source: &Path, sums: &BTreeMap<String, String>) -> Result<()> {
        let mut with_aliases = sums.clone();
        for (alias, target) in &self.config.aliases {
            with_aliases.insert(alias.clone(), sums[target].clone());
        }
        if let Ok(bytes) = fs::read(source.join("sums.cbor")) {
            if bytes != super::sums_manifest(&with_aliases)? {
                return Err(Refusal::new(
                    "checksums-mismatch",
                    "sums.cbor does not match every release file",
                ));
            }
            return Ok(());
        }
        let declared = fs::read_to_string(source.join("SHA256SUMS")).map_err(|_| {
            Refusal::new(
                "checksums-absent",
                "closure requires sums.cbor or SHA256SUMS",
            )
        })?;
        let mut expected = String::new();
        for (name, cid) in sums {
            let cid = files::require_cid(cid)?;
            let digest: String = cid
                .as_cid()
                .hash()
                .digest()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            expected.push_str(&format!("{digest}  {name}\n"));
        }
        if declared != expected {
            return Err(Refusal::new(
                "checksums-mismatch",
                "SHA256SUMS is not the canonical checksum set",
            ));
        }
        Ok(())
    }
    pub(super) fn identity(
        &self,
        directory: &Path,
        cid: &str,
        commit: &str,
    ) -> Result<Option<String>> {
        let binary = directory.join(&self.config.bins[0]);
        let mut spec = CommandSpec::new(binary.to_string_lossy(), ["--version"]);
        spec.timeout = std::time::Duration::from_secs(15);
        let out = self.run(&spec).map_err(|e| {
            if e.kind == "command-timeout" {
                e
            } else {
                Refusal::new("identity-failed", e.detail)
            }
        })?;
        if !out.success {
            return Err(Refusal::new("identity-failed", "--version failed"));
        }
        if self.tool == "epr" {
            let identity: Value = serde_json::from_str(&out.stdout).map_err(|_| {
                Refusal::new("identity-invalid", "epr --version did not return JSON")
            })?;
            if identity["cid"] != cid {
                return Err(Refusal::new(
                    "identity-mismatch",
                    format!("binary reports {}, bytes are {cid}", identity["cid"]),
                ));
            }
            Ok(identity["version"].as_str().map(str::to_owned))
        } else if self.tool == "brit" {
            if commit.len() == 40
                && commit.bytes().all(|b| b.is_ascii_hexdigit())
                && !out
                    .stdout
                    .lines()
                    .any(|line| line == format!("source HEAD: {commit}"))
            {
                return Err(Refusal::new(
                    "source-mismatch",
                    format!("brit --version does not report source HEAD: {commit}"),
                ));
            }
            Ok(out.stdout.lines().next().map(str::to_owned))
        } else {
            Err(Refusal::new("tool-unknown", &self.tool))
        }
    }
    /// Verify all release bytes and the release identity, never just a metadata claim.
    pub fn verified(&self, cid: &str) -> bool {
        let Some(record) = self.record(cid) else {
            return false;
        };
        let names: std::collections::BTreeSet<_> = self
            .config
            .bins
            .iter()
            .chain(self.config.aliases.keys())
            .collect();
        if record.tool != self.tool
            || record.cid != cid
            || record.rcid != cid
            || names != record.bins.keys().collect()
        {
            return false;
        }
        let Ok(dir) = self.artifact(cid) else {
            return false;
        };
        for (name, sum) in &record.bins {
            if !files::executable(&dir.join(name)) {
                return false;
            }
            let (Ok(cid), Ok(bytes)) = (BlobCid::parse(sum), fs::read(dir.join(name))) else {
                return false;
            };
            if !sum.starts_with("bafkrei") || !cid.verifies(&bytes) {
                return false;
            }
        }
        self.release_cid(&record.bins)
            .is_ok_and(|computed| computed == cid)
    }
    pub(super) fn usable(&self, cid: &str, bin: &str) -> bool {
        self.artifact(cid)
            .is_ok_and(|dir| files::executable(&dir.join(bin)))
    }
}
