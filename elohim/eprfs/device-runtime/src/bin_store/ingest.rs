use super::{
    files, parse_payload_formats,
    source::{copy_executable, CommandSpec},
    IngestOptions, Provenance, Refusal, Release, Result, Store, PAYLOAD_FORMATS_FIELD,
    PAYLOAD_FORMATS_PROBE_FIELD,
};
use eprfs_core::{BlobCid, ByteCid};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::{Error, ErrorKind, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
};

impl Store {
    /// Copy to private staging, verify those exact bytes, then atomically publish a candidate.
    /// Ingest never moves current, even if the candidate passed a gate.
    pub fn ingest(&self, source: &Path, options: &IngestOptions) -> Result<Release> {
        self.ingest_with_payload_formats_and_adoption(source, options, None, |_| Ok(None))
    }

    /// Adopt a plain installed path through ingest's locked held-or-repair decision.
    pub(super) fn ingest_adoption(
        &self,
        source: &Path,
        options: &IngestOptions,
        adopted_path: &Path,
    ) -> Result<Release> {
        self.ingest_with_payload_formats_and_adoption(source, options, Some(adopted_path), |_| {
            Ok(None)
        })
    }

    /// Ingest, then derive a payload-format statement from the primary executable held beneath
    /// its release CID. The callback may copy and execute those bytes elsewhere, but never sees
    /// the mutable source pathname. Explicit producer statements bypass the callback.
    pub fn ingest_with_payload_formats(
        &self,
        source: &Path,
        options: &IngestOptions,
        probe: impl FnOnce(&Path) -> std::result::Result<Option<String>, String>,
    ) -> Result<Release> {
        self.ingest_with_payload_formats_and_adoption(source, options, None, probe)
    }

    fn ingest_with_payload_formats_and_adoption(
        &self,
        source: &Path,
        options: &IngestOptions,
        adoption: Option<&Path>,
        probe: impl FnOnce(&Path) -> std::result::Result<Option<String>, String>,
    ) -> Result<Release> {
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
                payload_formats: value[PAYLOAD_FORMATS_FIELD].as_str().map(str::to_owned),
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
            let mut sums = BTreeMap::new();
            for name in &self.config.bins {
                let file = if source.is_file() && self.config.bins.first() == Some(name) {
                    source.to_owned()
                } else if source.is_file() {
                    source
                        .parent()
                        .ok_or_else(|| Refusal::new("path-invalid", "binary has no parent"))?
                        .join(name)
                } else {
                    source.join(name)
                };
                // Historical followed Brit releases declare exactly the original trio.
                // A missing helper is accepted only if the checksum manifest also omits it.
                if self.tool == "brit"
                    && name == "brit-rebase-retained"
                    && file
                        .symlink_metadata()
                        .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
                {
                    continue;
                }
                if !files::executable(&file) {
                    return Err(Refusal::new(
                        "binary-unusable",
                        format!("no executable {name}: {}", file.display()),
                    ));
                }
                copy_executable(&file, &stage.join(name))?;
                sums.insert(name.clone(), files::file_cid(&stage.join(name))?);
            }
            // Existing release downloads supply SHA256SUMS. It remains a checksum transport,
            // never an identity; canonical local closures instead carry sums.cbor.
            if source.is_dir() && (options.closure || self.config.bins.len() > 1) {
                self.verify_source_manifest(source, &stage, &sums)?;
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
            let held = self.record(&cid);
            let artifact_verified = artifact.exists()
                && sums.iter().all(|(name, sum)| {
                    files::executable(&artifact.join(name))
                        && files::file_cid(&artifact.join(name)).ok().as_ref() == Some(sum)
                });
            if artifact_verified {
                if let (Some(adopted_path), Some(release)) = (adoption, held.as_ref()) {
                    let events = artifact.join("adoptions.jsonl");
                    if let Err(error) = append_adoption(&events, adopted_path, &self.tool) {
                        eprintln!(
                            "warning: could not append adoption event to {}: {}; continuing launcher installation",
                            events.display(),
                            error
                        );
                    }
                    return Ok(release.clone());
                }
            }
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
            let probed_payload_formats = if options.payload_formats.is_none() {
                self.config
                    .bins
                    .first()
                    .map_or(Ok(None), |name| probe(&artifact.join(name)))
            } else {
                Ok(None)
            };
            let provenance = Provenance {
                source_commit: commit,
                tree_clean: clean,
                gate_run: options.gate_run.clone(),
                ingested_at: files::now(),
            };
            let mut release = held.unwrap_or_else(|| Release {
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
            if !matches!(
                release.ring.as_str(),
                "landed" | "reverted" | "lineage-rebased"
            ) {
                release.ring = "dev-context".into();
            }
            let statement = options
                .payload_formats
                .as_deref()
                .or_else(|| probed_payload_formats.as_ref().ok()?.as_deref());
            if let Some(statement) = statement {
                parse_payload_formats(Some(statement))
                    .map_err(|detail| Refusal::new("payload-format-statement-invalid", detail))?;
                if let Some(held) = release
                    .extra
                    .get(PAYLOAD_FORMATS_FIELD)
                    .and_then(Value::as_str)
                {
                    if held != statement {
                        return Err(Refusal::new(
                            "payload-format-statement-mismatch",
                            format!(
                                "identical release bytes were already held with {held}, not {statement}"
                            ),
                        ));
                    }
                }
                release.extra.insert(
                    PAYLOAD_FORMATS_FIELD.into(),
                    Value::String(statement.to_owned()),
                );
                release.extra.remove(PAYLOAD_FORMATS_PROBE_FIELD);
            } else if let Err(reason) = probed_payload_formats {
                release.extra.insert(
                    PAYLOAD_FORMATS_PROBE_FIELD.into(),
                    Value::String(format!("failed: {reason}")),
                );
            }
            self.save(&release)?;
            Ok(release)
        })();
        let _ = fs::remove_dir_all(stage);
        result
    }
    fn verify_source_manifest(
        &self,
        source: &Path,
        stage: &Path,
        sums: &BTreeMap<String, String>,
    ) -> Result<()> {
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
        for name in sums.keys() {
            // Transport SHA-256 authenticates the staged bytes, independently of their
            // native BLAKE3 identity. Reuse the same SHA2 helper as legacy migration.
            let cid = BlobCid::compute_raw(&fs::read(stage.join(name))?);
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
        _cid: &str,
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
            // --version identifies the executable, while a multi-bin release is the
            // CID of its sums manifest. Verify both identities at their own boundaries.
            let binary_cid = files::file_cid(&binary)?;
            if identity["cid"] != binary_cid {
                return Err(Refusal::new(
                    "identity-mismatch",
                    format!("binary reports {}, bytes are {binary_cid}", identity["cid"]),
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
    /// The release CID [`Store::ingest`] would record for the bins in `dir`, computed without
    /// writing, executing or locking anything. A caller holding bytes from elsewhere (a sibling's
    /// crossed Produce) compares this with the CID it was promised before any byte is ingested,
    /// because ingest runs the binary to read its identity.
    ///
    /// # Errors
    ///
    /// `binary-unusable` when a registered bin is not an executable file in `dir`; `checksums-*`
    /// when an alias present in `dir` differs from its target.
    pub fn release_cid_of(&self, dir: &Path) -> Result<String> {
        let mut sums = BTreeMap::new();
        for name in &self.config.bins {
            let file = dir.join(name);
            if !files::executable(&file) {
                return Err(Refusal::new(
                    "binary-unusable",
                    format!("no executable {name}: {}", file.display()),
                ));
            }
            sums.insert(name.clone(), files::file_cid(&file)?);
        }
        for (alias, target) in &self.config.aliases {
            let target_cid = sums.get(target).cloned().ok_or_else(|| {
                Refusal::new(
                    "checksums-mismatch",
                    format!("alias {alias} has no target {target}"),
                )
            })?;
            if dir.join(alias).is_file() && files::file_cid(&dir.join(alias))? != target_cid {
                return Err(Refusal::new(
                    "checksums-mismatch",
                    format!("alias {alias} differs from {target}"),
                ));
            }
            sums.insert(alias.clone(), target_cid);
        }
        self.release_cid(&sums)
    }
    /// Whether `follow` would land a stored release for `commit` without building: a verified
    /// record, built from a clean tree at that commit, carrying every declared bin (the same
    /// test `prepare_declared` applies before it reaches for a closure or a compiler).
    pub fn holds_verified_for(&self, commit: &str) -> bool {
        self.records().into_iter().any(|record| {
            record.source(commit).is_some_and(|s| s.tree_clean)
                && self
                    .config
                    .bins
                    .iter()
                    .all(|bin| record.bins.contains_key(bin))
                && self.verified(&record.cid)
        })
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
            || (names != record.bins.keys().collect()
                && !self.historical_brit_inventory(&record.bins)
                && !(self.tool == "epr"
                    && names.len() == 2
                    && names.iter().any(|name| name.as_str() == "epr")
                    && names.iter().any(|name| name.as_str() == "bin-launch")
                    && record.bins.len() == 1
                    && record.bins.contains_key("epr")))
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
            // A release file is payload bytes: its sum must be a byte-profile CID, admitted by
            // the kernel's full decode, never a prefix test.
            let (Ok(cid), Ok(bytes)) = (ByteCid::parse(sum), fs::read(dir.join(name))) else {
                return false;
            };
            if !BlobCid::from(cid).verifies(&bytes) {
                return false;
            }
        }
        self.release_cid(&record.bins)
            .is_ok_and(|computed| computed == cid)
    }
    // The only Brit compatibility inventory: the original three executables plus rakia.
    // Do not admit arbitrary subsets or ignore a checksum for a declared helper.
    fn historical_brit_inventory(&self, bins: &BTreeMap<String, String>) -> bool {
        self.tool == "brit"
            && bins.keys().map(String::as_str).eq([
                "brit",
                "brit-build-ref",
                "brit-verify",
                "rakia",
            ])
            && self
                .config
                .bins
                .iter()
                .map(String::as_str)
                .collect::<std::collections::BTreeSet<_>>()
                == [
                    "brit",
                    "brit-build-ref",
                    "brit-rebase-retained",
                    "brit-verify",
                ]
                .into_iter()
                .collect()
            && self.config.aliases.len() == 1
            && self
                .config
                .aliases
                .get("rakia")
                .is_some_and(|target| target == "brit")
    }
    /// Admission requires the complete inventory or the historical followed Brit release.
    pub(super) fn admissible(&self, cid: &str) -> bool {
        self.verified(cid)
            && self.record(cid).is_some_and(|record| {
                self.historical_brit_inventory(&record.bins)
                    || self
                        .config
                        .bins
                        .iter()
                        .chain(self.config.aliases.keys())
                        .all(|bin| record.bins.contains_key(bin))
            })
    }
    pub(super) fn usable(&self, cid: &str, bin: &str) -> bool {
        self.artifact(cid)
            .is_ok_and(|dir| files::executable(&dir.join(bin)))
    }
}

/// Append at most one event for an adopted path beneath this release CID. The caller holds the
/// store lock, so checking and appending form one decision with held-release verification.
fn append_adoption(events: &Path, adopted_path: &Path, tool: &str) -> Result<()> {
    if fs::read_to_string(events).is_ok_and(|contents| {
        contents.lines().any(|line| {
            serde_json::from_str::<Value>(line)
                .ok()
                .is_some_and(|event| event["path"] == json!(adopted_path))
        })
    }) {
        return Ok(());
    }
    let existed = events.exists();
    let mut line = serde_json::to_vec(&json!({
        "at": files::now(),
        "path": adopted_path,
        "session": crate::session::resolve().session,
        "tool": tool,
    }))?;
    line.push(b'\n');
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(events)?;
    if file.write(&line)? != line.len() {
        return Err(Error::new(
            ErrorKind::WriteZero,
            "adoption event was not appended as one complete line",
        )
        .into());
    }
    file.sync_all()?;
    if !existed {
        files::sync_parent(events)?;
    }
    Ok(())
}
