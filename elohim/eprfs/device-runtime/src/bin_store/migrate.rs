use super::{files, Refusal, Release, Result, Store};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, path::Path};

impl Store {
    pub(super) fn preserve_legacy_executable(&self, source: &Path) -> Result<Release> {
        if self.tool != "epr" {
            return Err(Refusal::new(
                "identity-mismatch",
                "legacy adoption only supports epr",
            ));
        }
        let bytes = fs::read(source)?;
        let cid = eprfs_core::BlobCid::compute_raw(&bytes);
        let old = format!(
            "sha256:{}",
            cid.as_cid()
                .hash()
                .digest()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        let _lock = files::Lock::acquire(&self.root, files::MUTATION_WAIT)?;
        let stage = files::temporary(&self.root.join("legacy-adopt"));
        fs::create_dir(&stage)?;
        let result = (|| {
            files::atomic_bytes(&stage.join("epr"), &bytes, 0o555)?;
            let check = self.run(&super::source::CommandSpec::new(
                stage.join("epr").to_string_lossy(),
                ["--version"],
            ))?;
            let identity: Value = serde_json::from_str(&check.stdout)?;
            if !check.success || identity["cid"] != old {
                return Err(Refusal::new(
                    "identity-mismatch",
                    "legacy executable identity does not verify",
                ));
            }
            files::atomic_json(
                &stage.join("record.json"),
                &json!({"cid":old,"rcid":old,"tool":"epr","version":identity["version"],"bins":{"epr":old},"source_commit":"unknown","tree_clean":false,"gate_run":"adopted-from-path","ingested_at":files::now(),"ring":"dev-context"}),
            )?;
            let destination = self.root.join(&old);
            if !destination.exists() {
                fs::rename(&stage, &destination)?;
                files::sync_parent(&destination)?;
            }
            Ok::<(), Refusal>(())
        })();
        let _ = fs::remove_dir_all(stage);
        result?;
        drop(_lock);
        self.migrate()?;
        self.record(&cid.to_string()).ok_or_else(|| {
            Refusal::new(
                "migration-invalid",
                "legacy adoption did not produce a release",
            )
        })
    }
    pub(super) fn require_migrated(&self) -> Result<()> {
        if self.root.join("migration.pending").exists() {
            return Err(Refusal::new(
                "migration-incomplete",
                "unfinished migration; run epr bin migrate before mutating this store",
            ));
        }
        Ok(())
    }
    /// Re-address legacy SHA-256 releases and references in place. The mapping is persisted
    /// before references change; rerunning after interruption resumes it. Old bytes remain as
    /// inert migration history, so already-running old executables are never unlinked.
    pub fn migrate(&self) -> Result<Value> {
        self.migrate_checkpoint(|| Ok(()))
    }
    pub(super) fn migrate_checkpoint(
        &self,
        after_releases: impl FnOnce() -> Result<()>,
    ) -> Result<Value> {
        let _lock = files::Lock::acquire(&self.root, files::MUTATION_WAIT)?;
        let marker = self.root.join("migration.pending");
        files::atomic_json(&marker, &json!({"version":1}))?;
        let journal = self.root.join("migration.json");
        let mut mapping: BTreeMap<String, String> = files::read_json(&journal).unwrap_or_default();
        for dir in files::children(&self.root) {
            if dir.is_symlink() {
                continue;
            }
            let Some(old) = dir.file_name().and_then(|s| s.to_str()).filter(|s| {
                s.starts_with("sha256:")
                    && s.len() == 71
                    && s[7..].bytes().all(|b| b.is_ascii_hexdigit())
            }) else {
                continue;
            };
            if mapping.get(old).is_some_and(|cid| self.verified(cid)) {
                continue;
            }
            let mut record: Value =
                files::read_json(&dir.join("record.json")).ok_or_else(|| {
                    Refusal::new("migration-invalid", format!("{old}: missing record"))
                })?;
            let mut sums = BTreeMap::new();
            let mut legacy_sums = String::new();
            let mut names = self.config.bins.clone();
            names.sort();
            for name in &names {
                let path = dir.join(name);
                if !files::executable(&path) {
                    return Err(Refusal::new("binary-unusable", path.display().to_string()));
                }
                let cid = files::file_cid(&path)?;
                let raw = files::require_cid(&cid)?;
                let digest: String = raw
                    .as_cid()
                    .hash()
                    .digest()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect();
                let expected = format!("sha256:{digest}");
                if record["bins"]
                    .get(name)
                    .and_then(Value::as_str)
                    .unwrap_or(old)
                    != expected
                {
                    return Err(Refusal::new(
                        "checksums-mismatch",
                        format!("legacy {old}/{name} changed"),
                    ));
                }
                legacy_sums.push_str(&format!("{digest}  {name}\n"));
                sums.insert(name.clone(), cid);
            }
            let legacy_hash = if names.len() == 1 {
                files::require_cid(&sums[&names[0]])?
            } else {
                eprfs_core::BlobCid::compute_raw(legacy_sums.as_bytes())
            };
            let expected = format!(
                "sha256:{}",
                legacy_hash
                    .as_cid()
                    .hash()
                    .digest()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            );
            if old != expected {
                return Err(Refusal::new(
                    "checksums-mismatch",
                    format!("legacy release {old} has wrong sum"),
                ));
            }
            for (alias, target) in &self.config.aliases {
                sums.insert(alias.clone(), sums[target].clone());
            }
            let cid = self.release_cid(&sums)?;
            let dest = self.artifact(&cid)?;
            fs::create_dir_all(&dest)?;
            for name in sums.keys() {
                let source = self.config.aliases.get(name).unwrap_or(name);
                super::source::copy_executable(&dir.join(source), &dest.join(name))?;
            }
            if sums.len() > 1 {
                files::atomic_bytes(
                    &dest.join("sums.cbor"),
                    &super::sums_manifest(&sums)?,
                    0o444,
                )?;
            }
            normalize(&mut record);
            record["legacyProvenance"] =
                files::read_json::<Value>(&dir.join("record.json")).unwrap_or(Value::Null);
            record["cid"] = json!(cid);
            record["rcid"] = json!(cid);
            record["bins"] = json!(sums);
            record["tool"] = json!(self.tool);
            if record.get("ring").is_none() {
                record["ring"] = json!("dev-context");
            }
            if record.get("sourceCommit").is_none() {
                record["sourceCommit"] = json!("unknown");
            }
            if record.get("treeClean").is_none() {
                record["treeClean"] = json!(false);
            }
            if record.get("ingestedAt").is_none() {
                record["ingestedAt"] = json!(files::now());
            }
            let mut release: Release = serde_json::from_value(record)?;
            if let Some(mut existing) = self.record(&cid) {
                existing.sources.extend(release.sources);
                existing.sources.push(release.provenance);
                for (k, v) in release.extra {
                    existing.extra.entry(k).or_insert(v);
                }
                release = existing;
            }
            self.save(&release)?;
            if let Ok(meta) = dir.join(".used").metadata() {
                if let Ok(time) = meta.modified() {
                    let time = dest
                        .join(".used")
                        .metadata()
                        .and_then(|m| m.modified())
                        .map(|existing| existing.max(time))
                        .unwrap_or(time);
                    files::touch(&dest.join(".used"))?;
                    fs::File::open(dest.join(".used"))?.set_modified(time)?;
                }
            }
            files::sync_parent(&dest)?;
            mapping.insert(old.into(), cid);
            files::atomic_json(&journal, &mapping)?;
        }
        after_releases()?;
        for name in ["current", "previous"] {
            if let Ok(target) = fs::read_link(self.root.join(name)) {
                if let Some(new) = target.to_str().and_then(|old| mapping.get(old)) {
                    files::atomic_link(&self.root.join(name), Path::new(new))?;
                }
            }
        }
        let metadata = self.root.join("current.json");
        if let Some(mut value) = files::read_json::<Value>(&metadata) {
            normalize(&mut value);
            repoint(&mut value, &mapping);
            files::atomic_json(&metadata, &value)?;
        }
        for path in files::children(&self.root.join("sessions")) {
            if let Some(mut value) = files::read_json::<Value>(&path) {
                normalize(&mut value);
                repoint(&mut value, &mapping);
                let destination = value["session"]
                    .as_str()
                    .map(|key| self.pin_path(key))
                    .transpose()?
                    .unwrap_or_else(|| path.clone());
                if destination != path && destination.exists() {
                    let existing: Value = files::read_json(&destination).ok_or_else(|| {
                        Refusal::new("migration-pin-conflict", "destination pin unreadable")
                    })?;
                    if existing != value {
                        return Err(Refusal::new(
                            "migration-pin-conflict",
                            "two pins name the same session; preserve both for review",
                        ));
                    }
                }
                files::atomic_json(&destination, &value)?;
                if destination != path {
                    fs::remove_file(&path)?;
                    files::sync_parent(&path)?;
                }
            }
        }
        self.repair_heads_locked()?;
        fs::remove_file(&marker)?;
        files::sync_parent(&marker)?;
        Ok(json!({"migrated":mapping,"legacyRetained":true}))
    }
}
fn normalize(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (old, new) in [
                ("source_commit", "sourceCommit"),
                ("tree_clean", "treeClean"),
                ("gate_run", "gateRun"),
                ("ingested_at", "ingestedAt"),
                ("landed_at", "landedAt"),
                ("pinned_at", "pinnedAt"),
                ("owner_pid", "ownerPid"),
                ("owner_starttime", "ownerStartTime"),
            ] {
                if let Some(v) = map.remove(old) {
                    map.entry(new).or_insert(v);
                }
            }
            for v in map.values_mut() {
                normalize(v);
            }
        }
        Value::Array(rows) => {
            for row in rows {
                normalize(row);
            }
        }
        _ => {}
    }
}
fn repoint(value: &mut Value, mapping: &BTreeMap<String, String>) {
    match value {
        Value::String(s) => {
            if let Some(new) = mapping.get(s) {
                *s = new.clone();
            }
        }
        Value::Array(rows) => {
            for row in rows {
                repoint(row, mapping);
            }
        }
        Value::Object(map) => {
            for (k, v) in map {
                if k != "legacyProvenance" {
                    repoint(v, mapping);
                }
            }
        }
        _ => {}
    }
}
