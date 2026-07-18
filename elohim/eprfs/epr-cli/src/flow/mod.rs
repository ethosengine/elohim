//! `epr flow` — derive REA/ValueFlows records from the repository filesystem and walk
//! the developer value chain (spec §3–§5,
//! `genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md`).
//!
//! The projection is deterministic + idempotent: resource identity is the canonical body
//! CID (frontmatter excluded, matching the Python cite oracle), all timestamps come from
//! git, and records are deduped by CID against the existing sidecar before append.

pub mod project;
pub mod registry;
pub mod walk;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use cid::Cid;
use elohim_epr_rea::{atom_cid, AgentRef, PinnedRef};
use eprfs_core::BlobCid;

/// The synthetic agent standing in for the repository as a whole.
pub const REPO_AGENT: &str = "repo:ethosengine/elohim";

/// Errors surfaced by the `flow` command family.
#[derive(Debug, thiserror::Error)]
pub enum FlowError {
    #[error("invalid arguments: {0}")]
    InvalidArguments(String),

    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("recipe registry parse ({path}): {source}")]
    Yaml {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("fabric: {0}")]
    Fabric(#[from] elohim_epr_rea::FabricError),

    #[error("resource not found in sidecar labels or the tree: {0}")]
    UnknownResource(String),
}

pub type FlowResult<T> = std::result::Result<T, FlowError>;

/// A label index: `cid string -> repo-relative path or descriptive label`.
pub type Labels = BTreeMap<String, String>;

/// Options shared by the flow subcommands.
struct GlobalOpts {
    root: PathBuf,
    json: bool,
}

/// Entry point invoked by `main` for `epr flow …`.
pub fn run(args: &[String]) -> FlowResult<ExitCode> {
    let Some(sub) = args.first().map(String::as_str) else {
        return Err(FlowError::InvalidArguments(usage()));
    };

    match sub {
        "project" => {
            let (opts, rest) = parse_global(&args[1..])?;
            let mut recipes = default_recipes(&opts.root);
            let mut i = 0;
            while i < rest.len() {
                match rest[i].as_str() {
                    "--recipes" => {
                        let value = rest.get(i + 1).ok_or_else(|| {
                            FlowError::InvalidArguments("--recipes needs a path".into())
                        })?;
                        recipes = resolve_under(&opts.root, value);
                        i += 2;
                    }
                    other => {
                        return Err(FlowError::InvalidArguments(format!(
                            "unknown project argument `{other}`"
                        )))
                    }
                }
            }
            let summary = project::project(&opts.root, &recipes)?;
            if opts.json {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            } else {
                summary.render(&opts.root);
            }
            Ok(ExitCode::SUCCESS)
        }
        "walk" => {
            let Some(target) = args.get(1).filter(|a| !a.starts_with("--")) else {
                return Err(FlowError::InvalidArguments(
                    "usage: epr flow walk <path> [--json] [--root DIR]".into(),
                ));
            };
            let (opts, rest) = parse_global(&args[2..])?;
            if let Some(other) = rest.first() {
                return Err(FlowError::InvalidArguments(format!(
                    "unknown walk argument `{other}`"
                )));
            }
            let result = walk::walk(&opts.root, target)?;
            if opts.json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                result.render();
            }
            Ok(ExitCode::SUCCESS)
        }
        "status" => {
            let (opts, rest) = parse_global(&args[1..])?;
            if let Some(other) = rest.first() {
                return Err(FlowError::InvalidArguments(format!(
                    "unknown status argument `{other}`"
                )));
            }
            let result = walk::status(&opts.root)?;
            if opts.json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                result.render();
            }
            Ok(ExitCode::SUCCESS)
        }
        other => Err(FlowError::InvalidArguments(format!(
            "unknown flow subcommand `{other}`\n{}",
            usage()
        ))),
    }
}

/// Pull `--root` and `--json` out of `args`, returning the remainder untouched.
fn parse_global(args: &[String]) -> FlowResult<(GlobalOpts, Vec<String>)> {
    let mut root = PathBuf::from(".");
    let mut json = false;
    let mut rest = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => {
                json = true;
                i += 1;
            }
            "--root" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| FlowError::InvalidArguments("--root needs a path".into()))?;
                root = PathBuf::from(value);
                i += 2;
            }
            _ => {
                rest.push(args[i].clone());
                i += 1;
            }
        }
    }
    let root = std::fs::canonicalize(&root).unwrap_or(root);
    Ok((GlobalOpts { root, json }, rest))
}

fn default_recipes(root: &Path) -> PathBuf {
    root.join(".claude/epr-meta/recipes.yaml")
}

fn resolve_under(root: &Path, value: &str) -> PathBuf {
    let p = PathBuf::from(value);
    if p.is_absolute() {
        p
    } else {
        root.join(p)
    }
}

fn usage() -> String {
    "usage: epr flow <project [--root DIR] [--recipes PATH] | walk <path> [--json] [--root DIR] \
     | status [--root DIR] [--json]>"
        .to_string()
}

// ---------------------------------------------------------------------------
// Canonical body → CID (the cite-oracle-parity encoder)
// ---------------------------------------------------------------------------

/// Extract the canonical BODY of a document exactly as the Python cite oracle does
/// (`_lib/cite_graph.body_of` + `_body_digest`): frontmatter excluded when a `---`
/// fenced block opens the file, then `.strip()`-equivalent trimming. A file with no
/// frontmatter (`.feature`, `.json`) canonicalizes to its whole, trimmed contents.
pub fn canonical_body(text: &str) -> String {
    let body = strip_frontmatter(text);
    body.trim().to_string()
}

/// Split the frontmatter from a document body, mirroring `_lib.frontmatter.parse`.
fn strip_frontmatter(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.first().map(|l| l.trim()) != Some("---") {
        return text.to_string();
    }
    let mut end = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            end = Some(i);
            break;
        }
    }
    match end {
        Some(e) => lines[e + 1..].join("\n"),
        None => text.to_string(),
    }
}

/// The canonical body CID (`CIDv1 · raw codec 0x55 · sha2-256`) of a document — the same
/// content-address whose short form is the doc's cite fingerprint (2026-07-12 convergence).
pub fn body_cid(text: &str) -> Cid {
    *BlobCid::compute_raw(canonical_body(text).as_bytes()).as_cid()
}

/// Read a file and return its canonical body CID, or `None` if the file cannot be read.
pub fn body_cid_of_file(path: &Path) -> Option<Cid> {
    let text = std::fs::read_to_string(path).ok()?;
    Some(body_cid(&text))
}

/// The repo-scope atom CID — the container of last resort for repo-wide flows.
pub fn repo_scope_atom() -> FlowResult<Cid> {
    Ok(atom_cid(&PinnedRef {
        id: REPO_AGENT.to_string(),
        version: 1,
    })?)
}

/// The synthetic repository agent.
pub fn repo_agent() -> AgentRef {
    AgentRef(REPO_AGENT.to_string())
}

// ---------------------------------------------------------------------------
// Frontmatter (Rust reader — scalar + string-list subset, matching the oracle)
// ---------------------------------------------------------------------------

/// A parsed frontmatter block: scalar fields and string-list fields.
#[derive(Debug, Default)]
pub struct Frontmatter {
    scalars: BTreeMap<String, String>,
    lists: BTreeMap<String, Vec<String>>,
}

impl Frontmatter {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.scalars.get(key).map(String::as_str)
    }

    pub fn list(&self, key: &str) -> &[String] {
        self.lists.get(key).map(Vec::as_slice).unwrap_or(&[])
    }
}

/// Parse the `---`-fenced frontmatter of a document (subset: `key: value` scalars and
/// `key:` followed by `- item` lists — the shape spec/plan frontmatter uses).
pub fn parse_frontmatter(text: &str) -> Frontmatter {
    let mut fm = Frontmatter::default();
    let lines: Vec<&str> = text.lines().collect();
    if lines.first().map(|l| l.trim()) != Some("---") {
        return fm;
    }
    let mut end = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            end = Some(i);
            break;
        }
    }
    let Some(end) = end else {
        return fm;
    };

    let mut pending: Option<String> = None;
    for line in &lines[1..end] {
        let trimmed = line.trim_end();
        if trimmed.trim().is_empty() {
            pending = None;
            continue;
        }
        // List continuation `  - item`
        if let Some(key) = &pending {
            let l = trimmed.trim_start();
            if let Some(item) = l.strip_prefix("- ") {
                fm.lists
                    .entry(key.clone())
                    .or_default()
                    .push(item.trim().to_string());
                continue;
            }
        }
        // `key: value` / `key:`
        if let Some((key, val)) = split_kv(trimmed) {
            if val.is_empty() {
                fm.lists.entry(key.clone()).or_default();
                pending = Some(key);
            } else {
                fm.scalars
                    .insert(key, val.trim_matches('"').trim_matches('\'').to_string());
                pending = None;
            }
        } else {
            pending = None;
        }
    }
    fm
}

fn split_kv(line: &str) -> Option<(String, String)> {
    // Only treat as a top-level key when there is no leading indentation.
    if line.starts_with(char::is_whitespace) {
        return None;
    }
    let (key, val) = line.split_once(':')?;
    let key = key.trim();
    if key.is_empty()
        || !key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return None;
    }
    Some((key.to_string(), val.trim().to_string()))
}

/// Extract the `path:` locator from a single cite envelope line, if present.
/// Envelope: `<slug> | <desc> | <fingerprint> [| status: …] [| path: <locator>]`.
pub fn cite_path(entry: &str) -> Option<String> {
    for segment in entry.split('|') {
        let seg = segment.trim();
        if let Some(rest) = seg.strip_prefix("path:") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

/// Extract the `sha256:hex16` fingerprint token from a cite envelope line, if present.
pub fn cite_fingerprint(entry: &str) -> Option<String> {
    for segment in entry.split('|') {
        let seg = segment.trim();
        if seg.starts_with("sha256:") {
            return Some(seg.to_string());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Git provenance
// ---------------------------------------------------------------------------

/// The `(author-email, iso-timestamp)` of the commit that ADDED `rel_path`, falling back
/// to the most recent commit touching it. `None` when git has no history for the path.
pub fn producing_commit(root: &Path, rel_path: &str) -> Option<(String, String)> {
    // Oldest add is the last line of the reverse-chronological add-filtered log.
    if let Some(pair) = git_log_pairs(root, &["--diff-filter=A"], rel_path)
        .into_iter()
        .last()
    {
        return Some(pair);
    }
    git_log_pairs(root, &["-1"], rel_path).into_iter().next()
}

fn git_log_pairs(root: &Path, extra: &[&str], rel_path: &str) -> Vec<(String, String)> {
    let mut args: Vec<&str> = vec!["log"];
    args.extend_from_slice(extra);
    args.extend_from_slice(&["--format=%ae%x1f%aI", "--", rel_path]);
    let Ok(out) = Command::new("git").args(&args).current_dir(root).output() else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let (email, ts) = line.split_once('\u{1f}')?;
            if email.is_empty() {
                return None;
            }
            Some((email.to_string(), ts.to_string()))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Turn an absolute path under `root` into a `/`-joined repo-relative string.
pub fn rel_to_root(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// A short, human-glanceable rendering of a CID for fallback labels.
pub fn short_cid(cid: &Cid) -> String {
    let s = cid.to_string();
    if s.len() > 12 {
        format!("{}…{}", &s[..8], &s[s.len() - 4..])
    } else {
        s
    }
}
