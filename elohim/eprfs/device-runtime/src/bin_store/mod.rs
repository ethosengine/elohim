//! Device-private executable custody. Ingest owns bytes; land owns the committed head.
//! Status is a projection, never an authority. CLI parsing lives in the consuming binary.
mod files;
mod ingest;
mod install;
mod landing;
mod launch;
mod lifecycle;
mod migrate;
mod source;

use eprfs_core::BlobCid;
pub use files::{file_cid, sums_manifest};
pub use launch::{launch, launch_os, Resolution};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
pub use source::{CommandOutput, CommandSpec, Runner, SystemRunner};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

/// A named refusal at the device boundary, suitable for CLI and JSON consumers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Refusal {
    /// Stable machine-readable refusal category.
    pub kind: String,
    /// Human-readable explanation and cure; never contains credentials.
    pub detail: String,
}
impl Refusal {
    pub(crate) fn new(kind: &str, detail: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            detail: detail.into(),
        }
    }
}
impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "REFUSED {}: {}", self.kind, self.detail)
    }
}
impl std::error::Error for Refusal {}
impl From<std::io::Error> for Refusal {
    fn from(error: std::io::Error) -> Self {
        Self::new("io", error.to_string())
    }
}
impl From<serde_json::Error> for Refusal {
    fn from(error: serde_json::Error) -> Self {
        Self::new("record-invalid", error.to_string())
    }
}
pub(crate) type Result<T> = std::result::Result<T, Refusal>;

/// Registry of followed tools. Invalid unrelated entries do not prevent name resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    /// Registry format version.
    pub version: u32,
    /// Tool descriptions, validated when selected.
    pub tools: BTreeMap<String, Value>,
}
/// Validated names and source metadata for one followed tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    /// Executable file names in the release.
    pub bins: Vec<String>,
    /// Command aliases and their release file targets.
    #[serde(default)]
    pub aliases: BTreeMap<String, String>,
    /// Explicit candidate environment variables, in preference order, per executable.
    #[serde(default)]
    pub candidate_env: BTreeMap<String, Vec<String>>,
    /// Argument prefixes identifying daemon launches.
    #[serde(default)]
    pub long_running: Vec<Vec<String>>,
    /// Source and attestation configuration from the registry.
    #[serde(default)]
    pub source: Value,
}
impl Registry {
    /// Load an explicitly selected registry.
    pub fn load(path: &Path) -> Result<Self> {
        serde_json::from_slice(
            &std::fs::read(path).map_err(|e| {
                Refusal::new("registry-invalid", format!("{}: {e}", path.display()))
            })?,
        )
        .map_err(|e| Refusal::new("registry-invalid", e.to_string()))
    }
    /// Load the environment override, installed sibling, or repository registry.
    pub fn discover() -> Result<Self> {
        Self::load(&Self::path()?)
    }
    pub(crate) fn path() -> Result<PathBuf> {
        if let Some(p) = files::env("EPR_BIN_TOOLS") {
            return files::absolute(Path::new(&p));
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                let p = parent.join("bin-tools.json");
                if p.is_file() {
                    return Ok(p);
                }
            }
        }
        let start = std::env::current_dir()?;
        start
            .ancestors()
            .map(|p| p.join("genesis/agentic/bin-tools.json"))
            .find(|p| p.is_file())
            .ok_or_else(|| Refusal::new("registry-invalid", "set EPR_BIN_TOOLS to bin-tools.json"))
    }
    /// Validate one tool, including path-safe bin and alias names.
    pub fn tool(&self, name: &str) -> Result<Tool> {
        let value = self
            .tools
            .get(name)
            .ok_or_else(|| Refusal::new("tool-unknown", name))?;
        let tool: Tool = serde_json::from_value(value.clone())
            .map_err(|e| Refusal::new("registry-invalid", e.to_string()))?;
        let valid = |s: &str| {
            !s.is_empty()
                && s != "."
                && s != ".."
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_- .".contains(&b))
                && !s.contains(' ')
        };
        if !valid(name)
            || tool.bins.is_empty()
            || tool.bins.iter().any(|s| !valid(s))
            || tool
                .aliases
                .iter()
                .any(|(k, v)| !valid(k) || !tool.bins.contains(v))
        {
            return Err(Refusal::new(
                "registry-invalid",
                "invalid tool, bin, or alias name",
            ));
        }
        let distinct: std::collections::BTreeSet<_> = tool.bins.iter().collect();
        if distinct.len() != tool.bins.len() {
            return Err(Refusal::new("registry-invalid", "duplicate release bin"));
        }
        Ok(tool)
    }
    /// Resolve an invoked basename to exactly one tool and release executable.
    pub fn invoked(&self, invoked: &Path) -> Result<(String, String)> {
        let name = invoked.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let mut found = Vec::new();
        for (id, raw) in &self.tools {
            let binary = raw["bins"]
                .as_array()
                .is_some_and(|a| a.iter().any(|v| v == name));
            let alias = raw["aliases"].get(name).is_some();
            if binary || alias {
                let tool = self.tool(id)?;
                if binary {
                    found.push((id.clone(), name.to_owned()));
                }
                if let Some(target) = tool.aliases.get(name) {
                    found.push((id.clone(), target.clone()));
                }
            }
        }
        if found.len() != 1 {
            return Err(Refusal::new(
                "launcher-name-invalid",
                format!("{name:?} resolves to {} tools", found.len()),
            ));
        }
        found
            .pop()
            .ok_or_else(|| Refusal::new("launcher-name-invalid", name))
    }
}

/// Ingest provenance; repeated ingest appends evidence without replacing the original source.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provenance {
    /// Source commit observed at ingest.
    pub source_commit: String,
    /// Whether that build's source closure was clean.
    pub tree_clean: bool,
    /// Gate or adoption run that supplied the bytes.
    pub gate_run: Option<String>,
    /// UTC ingest timestamp (display and retention only, never ordering authority).
    pub ingested_at: String,
}
/// An immutable content identity and appendable private provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    /// Raw binary CID or DAG-CBOR sums CID.
    pub cid: String,
    /// Release identity (same as cid).
    pub rcid: String,
    /// Registry tool name.
    pub tool: String,
    /// File names mapped to raw file CIDs, including aliases.
    pub bins: BTreeMap<String, String>,
    /// Version reported by the executable.
    pub version: Option<String>,
    /// Original provenance, retained across reingest.
    #[serde(flatten)]
    pub provenance: Provenance,
    /// Every observed source for these identical bytes.
    #[serde(default)]
    pub sources: Vec<Provenance>,
    /// dev-context, landed, or reverted.
    pub ring: String,
    /// Attestation verdicts keyed by repo, SHA and check (serialized tuple key).
    #[serde(default)]
    pub attestations: BTreeMap<String, Value>,
    /// Additional prior provenance and refusal records retained during migration.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
impl Release {
    pub(crate) fn source(&self, commit: &str) -> Option<&Provenance> {
        self.sources
            .iter()
            .rev()
            .find(|p| p.source_commit == commit)
            .or_else(|| (self.provenance.source_commit == commit).then_some(&self.provenance))
    }
}
/// A session pin or daemon lease. Owner identity is PID plus start time, never PID alone.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pin {
    /// Release retained by this owner.
    pub cid: String,
    /// Executable identity observed by the daemon itself; absent before exec or on old leases.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub running_cid: Option<String>,
    /// Admission route for a running daemon; absent on ordinary pins and older leases.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Stable session id or daemon pid/start-time key.
    pub session: String,
    /// UTC creation timestamp for ownerless grace.
    pub pinned_at: String,
    /// Owning process, if observed.
    pub owner_pid: Option<u32>,
    /// Linux clock ticks as decimal text, lossless in JavaScript.
    pub owner_start_time: Option<String>,
    /// Explicitly accepted unreachable attestation; never authorizes current.
    #[serde(default)]
    pub provisional: bool,
}
/// Ephemeral JSON view; consumers need not infer any private store paths.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// Tool name.
    pub tool: String,
    /// Absolute private store location, for diagnostics only.
    pub store: PathBuf,
    /// Current session key, if any.
    pub session: Option<String>,
    /// Session's immutable pin.
    pub session_cid: Option<String>,
    /// Committed device head.
    pub head: Option<String>,
    /// Whether head and session pin differ.
    pub behind: bool,
    /// Resolution provenance or unresolved.
    pub source: String,
    /// Resolved binary's release CID.
    pub resolved_cid: Option<String>,
    /// Executable path returned by native resolution.
    pub resolved: Option<PathBuf>,
    /// Whether current's files verify against its CID.
    pub verified_current: bool,
    /// Derived head links differ from the authoritative record; explicit repair is available.
    pub head_repair_needed: bool,
    /// Ingested candidates matching the optional commit filter.
    pub candidates: Vec<Release>,
}
/// Options at the ingest boundary; explicit provenance is used by gates and migration.
#[derive(Default)]
pub struct IngestOptions {
    /// Checkout supplying source state when not explicit.
    pub repo: Option<PathBuf>,
    /// Source commit override.
    pub source_commit: Option<String>,
    /// Gate run identifier.
    pub gate_run: Option<String>,
    /// Source cleanliness override.
    pub tree_clean: Option<bool>,
    /// Require a sums manifest even for a single file (local media import).
    pub closure: bool,
}
/// A device-local store for one registered tool.
#[derive(Clone)]
pub struct Store {
    root: PathBuf,
    tool: String,
    config: Tool,
    runner: Arc<dyn Runner>,
}
impl Store {
    /// Open a store description without mutating it.
    pub fn new(root: impl AsRef<Path>, tool: &str, registry: &Registry) -> Result<Self> {
        Ok(Self {
            root: files::absolute(root.as_ref())?,
            tool: tool.into(),
            config: registry.tool(tool)?,
            runner: Arc::new(SystemRunner),
        })
    }
    /// Select TOOL_STORE, BIN_STORE/tool, or the persistent device default.
    pub fn from_environment(tool: &str, registry: &Registry) -> Result<Self> {
        let path = files::env(&format!("{}_STORE", tool.to_uppercase()))
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(
                    files::env("BIN_STORE")
                        .unwrap_or_else(|| "/projects/.claude-config/tools/bin-store".into()),
                )
                .join(tool)
            });
        Self::new(path, tool, registry)
    }
    /// Supply an external-command boundary for isolated build and attestation tests.
    pub fn with_runner(mut self, runner: Arc<dyn Runner>) -> Self {
        self.runner = runner;
        self
    }
    /// Absolute store root, for native filesystem adapters only.
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub(crate) fn artifact(&self, cid: &str) -> Result<PathBuf> {
        files::require_cid(cid)?;
        Ok(self.root.join(cid))
    }
    pub(crate) fn record(&self, cid: &str) -> Option<Release> {
        files::read_json(&self.artifact(cid).ok()?.join("record.json"))
    }
    pub(crate) fn head(&self, name: &str) -> Option<String> {
        let path = self.root.join("current.json");
        if path.exists() {
            let record: Value = files::read_json(&path)?;
            let key = if name == "current" { "cid" } else { "previous" };
            return record[key]
                .as_str()
                .filter(|s| files::require_cid(s).is_ok())
                .map(str::to_owned);
        }
        files::read_link(&self.root.join(name))
    }
    pub(crate) fn records(&self) -> Vec<Release> {
        let mut records: Vec<_> = files::children(&self.root)
            .into_iter()
            .filter(|p| !p.is_symlink())
            .filter_map(|p| p.file_name()?.to_str().and_then(|cid| self.record(cid)))
            .collect();
        records.sort_by(|a, b| b.provenance.ingested_at.cmp(&a.provenance.ingested_at));
        records
    }
    pub(crate) fn save(&self, release: &Release) -> Result<()> {
        files::atomic_json(&self.artifact(&release.cid)?.join("record.json"), release)
    }
    pub(crate) fn release_cid(&self, sums: &BTreeMap<String, String>) -> Result<String> {
        if sums.len() == 1 {
            return sums
                .values()
                .next()
                .cloned()
                .ok_or_else(|| Refusal::new("checksums-absent", "empty release"));
        }
        Ok(BlobCid::compute(&sums_manifest(sums)?).to_string())
    }
}

#[cfg(test)]
mod tests;
