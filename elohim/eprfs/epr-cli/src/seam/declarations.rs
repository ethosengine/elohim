//! Repository-owned declarations and freshly verified evidence; no ownership registry in core.
use std::path::{Component, Path, PathBuf};

use eprfs_core::BlobCid;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const MAX_BYTES: usize = 512 * 1024;
pub fn identity(bytes: &[u8]) -> String {
    BlobCid::compute_raw(bytes).to_string()
}

/// Paths are repository-relative and may not escape through symlinks or independent repos.
pub fn local_path(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let path = Path::new(rel);
    if rel.is_empty()
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(format!("non-local path: {rel}"));
    }
    let canonical_root = root.canonicalize().map_err(|e| e.to_string())?;
    let mut current = canonical_root.clone();
    for part in path.components() {
        current.push(part);
        if current.join(".git").exists() {
            return Err(format!("independent repository: {rel}"));
        }
        if current.exists()
            && !current
                .canonicalize()
                .map_err(|e| e.to_string())?
                .starts_with(&canonical_root)
        {
            return Err(format!("path escapes repository: {rel}"));
        }
    }
    Ok(current)
}

pub fn read(root: &Path, rel: &str) -> Result<String, String> {
    let path = local_path(root, rel)?;
    if std::fs::metadata(&path)
        .map_err(|e| format!("{rel}: {e}"))?
        .len()
        > MAX_BYTES as u64
    {
        return Err(format!("source exceeds {MAX_BYTES} bytes: {rel}"));
    }
    std::fs::read_to_string(path).map_err(|e| format!("{rel}: {e}"))
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Source {
    pub path: String,
    pub needle: String,
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Declaration {
    pub version: u32,
    pub id: String,
    pub responsibility: String,
    pub owner: Vec<String>,
    pub allowed_consumers: Vec<String>,
    pub authoritative_contracts: Vec<Source>,
    pub reusable_interfaces: Vec<Source>,
    pub detection: Vec<String>,
    pub composition: Vec<String>,
    pub exclusions: Vec<String>,
    pub applies_to: Vec<String>,
}

pub fn load(root: &Path, rel: &str) -> Result<(Declaration, String), String> {
    let raw = read(root, rel)?;
    let d: Declaration =
        serde_json::from_str(&raw).map_err(|e| format!("invalid declaration {rel}: {e}"))?;
    if d.version != 1
        || d.id.is_empty()
        || d.responsibility.is_empty()
        || d.owner.is_empty()
        || d.applies_to.is_empty()
        || d.detection.is_empty()
        || d.authoritative_contracts.is_empty()
        || d.detection
            .iter()
            .chain(&d.composition)
            .any(|s| s.is_empty())
    {
        return Err(format!(
            "invalid declaration {rel}: required values are empty or unsupported"
        ));
    }
    for pattern in d
        .owner
        .iter()
        .chain(&d.allowed_consumers)
        .chain(&d.exclusions)
        .chain(&d.applies_to)
    {
        if pattern.is_empty() {
            return Err(format!("empty path pattern in {rel}"));
        }
        glob::Pattern::new(pattern).map_err(|e| format!("invalid glob in {rel}: {e}"))?;
    }
    for source in d
        .authoritative_contracts
        .iter()
        .chain(&d.reusable_interfaces)
    {
        if source.path.is_empty()
            || source.needle.is_empty()
            || !["implemented", "contract", "planned"].contains(&source.status.as_str())
        {
            return Err(format!("invalid source in {rel}"));
        }
    }
    Ok((d, identity(raw.as_bytes())))
}

pub fn matches(patterns: &[String], path: &str) -> bool {
    patterns
        .iter()
        .any(|p| glob::Pattern::new(p).is_ok_and(|g| g.matches(path)))
}

pub fn evidence(root: &Path, source: &Source) -> Result<Value, String> {
    let raw = read(root, &source.path)?;
    let start = raw
        .find(&source.needle)
        .ok_or_else(|| format!("source anchor absent: {}", source.path))?;
    Ok(
        json!({"path": source.path, "sourceId": identity(raw.as_bytes()),
        "startByte": start, "endByte": start + source.needle.len(),
        "line": raw[..start].bytes().filter(|b| *b == b'\n').count() + 1,
        "text": source.needle, "status": source.status}),
    )
}
