//! cargo's own records of a workspace, read and nothing more: the JSON of
//! `cargo metadata --format-version 1 --locked`, and the `Cargo.lock` it resolved (the per-package
//! `checksum` lives only there).
//!
//! **Only the fields the bridge reads are declared.** In particular `authors` is never declared,
//! so serde drops it while parsing and no author name or email can reach any record this crate
//! writes. A person is never derived from package metadata; the upstream *project* is named by
//! its repository URL instead (see [`crate::translate::presence_slug`]).

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::translate::Refusal;

/// The subset of `cargo metadata --format-version 1` the bridge reads.
#[derive(Debug, Clone, Deserialize)]
pub struct Metadata {
    pub packages: Vec<Package>,
    pub workspace_members: Vec<String>,
    pub resolve: Option<Resolve>,
    pub workspace_root: String,
    pub version: u32,
}

/// One package. `authors` is deliberately absent (see the module docs).
#[derive(Debug, Clone, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub id: String,
    pub source: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub manifest_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Resolve {
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Node {
    pub id: String,
    #[serde(default)]
    pub deps: Vec<NodeDep>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NodeDep {
    pub pkg: String,
}

/// Parse the metadata JSON, refusing a format version this bridge does not read or a run made
/// with `--no-deps` (no resolve graph: the dependencies cannot be observed).
pub fn parse_metadata(text: &str) -> Result<Metadata, Refusal> {
    let metadata: Metadata = serde_json::from_str(text)
        .map_err(|e| Refusal(format!("cargo metadata JSON is malformed: {e}")))?;
    if metadata.version != 1 {
        return Err(Refusal(format!(
            "cargo metadata format version {} is unknown (this bridge reads 1)",
            metadata.version
        )));
    }
    if metadata.resolve.is_none() {
        return Err(Refusal(
            "cargo metadata has no resolve graph — run it without --no-deps".into(),
        ));
    }
    Ok(metadata)
}

#[derive(Debug, Deserialize)]
struct Lockfile {
    /// Absent in format v1 and v2, which keep checksums in a `[metadata]` table instead.
    version: Option<u32>,
    #[serde(default)]
    package: Vec<LockPackage>,
}

#[derive(Debug, Deserialize)]
struct LockPackage {
    name: String,
    version: String,
    source: Option<String>,
    checksum: Option<String>,
}

/// `(name, version, source)` → the lockfile's `checksum`, for every locked package that has one.
pub type Checksums = BTreeMap<(String, String, String), String>;

/// Read the checksums out of a `Cargo.lock` (format v3 or v4, where each `[[package]]` carries
/// its own `checksum`). A package without one (path and git sources) is simply absent. An older
/// format is refused, naming its version: read here it would silently lose every checksum.
pub fn parse_lockfile(text: &str) -> Result<Checksums, Refusal> {
    let lock: Lockfile =
        toml::from_str(text).map_err(|e| Refusal(format!("Cargo.lock is malformed: {e}")))?;
    match lock.version {
        Some(v) if v >= 3 => {}
        Some(v) => {
            return Err(Refusal(format!(
                "Cargo.lock format v{v} keeps checksums outside its packages — regenerate it \
                 with a current cargo (v3 or later)"
            )))
        }
        None => {
            return Err(Refusal(
                "Cargo.lock has no `version` key (format v1 or v2) — regenerate it with a \
                 current cargo (v3 or later)"
                    .into(),
            ))
        }
    }
    Ok(lock
        .package
        .into_iter()
        .filter_map(|p| {
            let source = p.source?;
            let checksum = p.checksum?;
            Some(((p.name, p.version, source), checksum))
        })
        .collect())
}
