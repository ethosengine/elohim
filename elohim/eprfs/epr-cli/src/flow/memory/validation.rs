use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use cid::Cid;
use elohim_epr_rea::{FlowRecord, FlowStore, SidecarFlowStore};
use eprfs_agent::memory::{Collective, Contribution, Feedback, FileRef, ProjectionRequest, Reach};
use eprfs_core::BlobCid;
use serde_json::{json, Value};

use super::{refused, FlowResult};

pub const COLLECTIVE_PATH: &str = ".epr-meta/collective.json";
const MAX_BYTES: usize = 262144;
const MAX_FILES: usize = 32;

#[derive(Clone)]
pub struct ReadFile {
    pub reference: FileRef,
    pub text: String,
}

pub struct Reader {
    pub root: PathBuf,
    files: BTreeMap<String, ReadFile>,
    bytes: usize,
    records: Option<Vec<(Cid, FlowRecord)>>,
    sidecar_bytes: u64,
}

impl Reader {
    pub fn new(root: &Path) -> FlowResult<Self> {
        Ok(Self {
            root: root.canonicalize()?,
            files: BTreeMap::new(),
            bytes: 0,
            records: None,
            sidecar_bytes: 0,
        })
    }

    fn path(&self, path: &str) -> FlowResult<PathBuf> {
        let rel = Path::new(path);
        if rel.as_os_str().is_empty()
            || rel.components().any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(refused(
                "path must be relative with no dot, parent or root components",
            ));
        }
        let mut full = self.root.clone();
        for part in rel.components() {
            full.push(part);
            let meta = std::fs::symlink_metadata(&full)?;
            if meta.file_type().is_symlink() {
                return Err(refused("symlink source paths are refused"));
            }
        }
        Ok(full)
    }

    pub fn read(&mut self, path: &str) -> FlowResult<ReadFile> {
        if let Some(file) = self.files.get(path) {
            return Ok(file.clone());
        }
        if self.files.len() >= MAX_FILES {
            return Err(refused("source file budget exhausted"));
        }
        let full = self.path(path)?;
        let metadata = std::fs::metadata(&full)?;
        if !metadata.is_file() {
            return Err(refused("source is not a regular file"));
        }
        let remaining = MAX_BYTES.saturating_sub(self.bytes);
        if metadata.len() > remaining as u64 {
            return Err(refused("source byte budget exhausted"));
        }
        let mut bytes = Vec::new();
        File::open(&full)?
            .take(remaining as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > remaining {
            return Err(refused("source grew beyond byte budget"));
        }
        let text = String::from_utf8(bytes.clone()).map_err(|_| refused("source is not UTF-8"))?;
        let file = ReadFile {
            reference: FileRef {
                path: path.into(),
                cid: BlobCid::compute_raw(&bytes).to_string(),
            },
            text,
        };
        self.bytes += bytes.len();
        self.files.insert(path.into(), file.clone());
        Ok(file)
    }

    pub fn pinned(&mut self, reference: &FileRef) -> FlowResult<ReadFile> {
        let file = self.read(&reference.path)?;
        if file.reference != *reference {
            return Err(refused(format!(
                "source version changed: {}",
                reference.path
            )));
        }
        Ok(file)
    }

    pub fn unchanged(&self, reference: &FileRef) -> FlowResult<()> {
        let path = self.path(&reference.path)?;
        let mut bytes = Vec::new();
        File::open(path)?
            .take(MAX_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_BYTES || BlobCid::compute_raw(&bytes).to_string() != reference.cid {
            return Err(refused("source changed at write boundary"));
        }
        Ok(())
    }

    pub fn sidecar(&self, path: &str) -> FlowResult<()> {
        let full = self.path(path)?;
        if std::fs::metadata(full)?.len() > 33554432 {
            return Err(refused("native sidecar exceeds 32MiB local read budget"));
        }
        Ok(())
    }

    pub fn records(&mut self) -> FlowResult<&[(Cid, FlowRecord)]> {
        if self.records.is_none() {
            self.sidecar(".eprfs/status/flows.jsonl")?;
            self.sidecar_bytes =
                std::fs::metadata(self.root.join(".eprfs/status/flows.jsonl"))?.len();
            self.records = Some(SidecarFlowStore::open(&self.root)?.records()?);
        }
        Ok(self.records.as_deref().unwrap_or(&[]))
    }

    pub fn collective(&mut self) -> FlowResult<(FileRef, Collective)> {
        let file = self.read(COLLECTIVE_PATH)?;
        let declaration: Collective = serde_json::from_str(&file.text)?;
        version(declaration.version)?;
        if !declaration.id.starts_with("collective:")
            || declaration.steward != crate::flow::REPO_AGENT
            || declaration.participation != "registered-local-session"
        {
            return Err(refused(
                "unsupported collective relationship or participation policy",
            ));
        }
        bounded_text(&declaration.display_name, "displayName", 256)?;
        bounded_text(&declaration.charter, "charter", 4000)?;
        if declaration.source_rules.is_empty() || declaration.source_rules.len() > 16 {
            return Err(refused("requires 1..16 source rules"));
        }
        let mut rules = BTreeSet::new();
        for rule in &declaration.source_rules {
            if !rules.insert(&rule.path) {
                return Err(refused("duplicate source rule path"));
            }
            if rule.path.is_empty()
                || Path::new(&rule.path)
                    .components()
                    .any(|c| !matches!(c, Component::Normal(_)))
            {
                return Err(refused("source rule must be a normalized relative path"));
            }
        }
        Ok((file.reference, declaration))
    }

    pub fn require_collective(&self, supplied: &FileRef, actual: &FileRef) -> FlowResult<()> {
        if supplied != actual {
            return Err(refused(
                "collective declaration version/path differs; re-read current governance",
            ));
        }
        Ok(())
    }

    pub fn allowed(&self, path: &str, reach: Reach, collective: &Collective) -> FlowResult<()> {
        if reach > self.policy_reach(path, collective)? {
            return Err(refused(format!(
                "source policy forbids requested reach: {path}"
            )));
        }
        Ok(())
    }

    pub fn policy_reach(&self, path: &str, collective: &Collective) -> FlowResult<Reach> {
        // Most specific component-prefix wins; duplicate rule paths are refused.
        collective
            .source_rules
            .iter()
            .filter(|r| Path::new(path).starts_with(&r.path))
            .max_by_key(|r| Path::new(&r.path).components().count())
            .map(|r| r.max_reach)
            .ok_or_else(|| {
                refused(format!(
                    "source is outside declared collective policy: {path}"
                ))
            })
    }

    pub fn effective_reach(
        &mut self,
        reference: &FileRef,
        collective: &Collective,
        depth: usize,
    ) -> FlowResult<Reach> {
        if depth > 8 {
            return Err(refused(
                "feedback provenance depth exceeds 8; stop or narrow the chain",
            ));
        }
        let file = self.pinned(reference)?;
        let mut reach = self.policy_reach(&reference.path, collective)?;
        if let Ok(assertion) = serde_json::from_str::<Contribution>(&file.text) {
            reach = reach.min(assertion.reach);
        }
        if let Ok(request) = serde_json::from_str::<ProjectionRequest>(&file.text) {
            reach = reach.min(request.audience);
        }
        if let Ok(feedback) = serde_json::from_str::<Feedback>(&file.text) {
            reach = reach.min(self.effective_reach(&feedback.target, collective, depth + 1)?);
        }
        if let Ok(value) = serde_json::from_str::<Value>(&file.text) {
            match value.get("operation").and_then(Value::as_str) {
                Some("project") => {
                    let audience: Reach =
                        serde_json::from_value(value["receipt"]["audience"].clone())?;
                    let request: FileRef =
                        serde_json::from_value(value["receipt"]["request"].clone())?;
                    reach = reach
                        .min(audience)
                        .min(self.policy_reach(&request.path, collective)?);
                }
                Some("feedback" | "contribute" | "graduate") => {
                    let inherited: Reach = serde_json::from_value(value["effectiveReach"].clone())
                        .map_err(|_| {
                            refused("saved native receipt has no verifiable inherited reach")
                        })?;
                    reach = reach.min(inherited);
                    if value["operation"] == "feedback" {
                        let target: FileRef = serde_json::from_value(value["target"].clone())?;
                        let request: FileRef = serde_json::from_value(value["resource"].clone())?;
                        reach = reach
                            .min(self.effective_reach(&target, collective, depth + 1)?)
                            .min(self.policy_reach(&request.path, collective)?);
                    }
                }
                Some("pin") => reach = Reach::Private,
                _ => {}
            }
        }
        Ok(reach)
    }

    pub fn contribution(
        &mut self,
        c: &Contribution,
        reference: &FileRef,
        collective: &Collective,
    ) -> FlowResult<()> {
        version(c.version)?;
        self.require_collective(&c.collective, reference)?;
        if c.steward != collective.steward
            || !["session", "workspace", "repository"].contains(&c.scope.as_str())
        {
            return Err(refused("contribution steward or scope is not declared"));
        }
        elohim_epr_rea::parse_agent_ref(&c.author)?;
        bounded_text(&c.concern, "concern", 256)?;
        bounded_text(&c.claim, "claim", 1000)?;
        strings(&c.uncertainty, "uncertainty", 8, 300)?;
        if c.sources.is_empty()
            || c.sources.len() > 8
            || c.supersedes.len() > 8
            || c.contradicts.len() > 8
        {
            return Err(refused(
                "contribution needs 1..8 sources and at most 8 links of each kind",
            ));
        }
        for source in &c.sources {
            self.allowed(&source.resource.path, source.reach, collective)?;
            if c.reach > source.reach {
                return Err(refused("contribution cannot widen source restriction"));
            }
            let file = self.pinned(&source.resource)?;
            if c.reach > self.effective_reach(&file.reference, collective, 0)? {
                return Err(refused(
                    "source or receipt metadata retains its inherited reach restriction",
                ));
            }
        }
        for link in c.supersedes.iter().chain(&c.contradicts) {
            self.allowed(&link.path, c.reach, collective)?;
            if c.reach > self.effective_reach(link, collective, 0)? {
                return Err(refused(
                    "linked contribution metadata retains its reach restriction",
                ));
            }
        }
        Ok(())
    }

    pub fn usage(&self) -> Value {
        json!({"sourceFiles":self.files.len(),"sourceBytes":self.bytes,"nativeSidecarBytes":self.sidecar_bytes,
            "accounting":"Unique input bytes; write-boundary fingerprint reread and existing native sidecar evaluation are additional bounded work."})
    }
}

pub fn version(version: u32) -> FlowResult<()> {
    if version != 1 {
        return Err(refused("unsupported contract version"));
    }
    Ok(())
}

pub fn bounded_text(value: &str, name: &str, max: usize) -> FlowResult<()> {
    if value.trim().is_empty() || value.len() > max {
        return Err(refused(format!(
            "{name} must be nonempty and <= {max} bytes"
        )));
    }
    Ok(())
}

pub fn strings(values: &[String], name: &str, count: usize, max: usize) -> FlowResult<()> {
    if values.len() > count {
        return Err(refused(format!("too many {name} values")));
    }
    for value in values {
        bounded_text(value, name, max)?;
    }
    Ok(())
}
