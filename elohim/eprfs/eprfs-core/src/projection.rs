use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::address::{BlobCid, EprRef, ProjectionId};
use crate::error::{EprfsError, Result};

/// A validated relative path inside a projection.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectionPath(PathBuf);

impl ProjectionPath {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if path.as_os_str().is_empty() || path.is_absolute() {
            return Err(EprfsError::InvalidProjectionPath(
                path.display().to_string(),
            ));
        }

        for component in path.components() {
            match component {
                Component::Normal(_) => {}
                _ => {
                    return Err(EprfsError::InvalidProjectionPath(
                        path.display().to_string(),
                    ))
                }
            }
        }

        Ok(Self(path.to_path_buf()))
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

/// Projection root requested by a domain adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionRoot {
    pub id: ProjectionId,
    pub root: EprRef,
}

/// What kind of filesystem entry should be materialized.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
}

/// Current local state of a projected entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectionStatus {
    Local,
    Remote,
    Materializing,
    Missing,
    Unknown,
}

/// Policy for turning projection metadata into local files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MaterializationPolicy {
    /// Only write blobs already present locally.
    LocalOnly,
    /// Fetch missing blobs on demand while materializing.
    FetchMissing,
    /// Create metadata-only/sparse entries when bytes are not present.
    Sparse,
}

/// One path entry in a projected filesystem tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionEntry {
    pub path: ProjectionPath,
    pub kind: EntryKind,
    pub epr: Option<EprRef>,
    pub blob: Option<BlobCid>,
    pub size_bytes: Option<u64>,
    pub executable: bool,
    pub status: ProjectionStatus,
    pub metadata: Value,
}

impl ProjectionEntry {
    pub fn file(path: ProjectionPath, blob: BlobCid) -> Self {
        Self {
            path,
            kind: EntryKind::File,
            epr: None,
            blob: Some(blob),
            size_bytes: None,
            executable: false,
            status: ProjectionStatus::Unknown,
            metadata: Value::Null,
        }
    }
}

/// A complete local view of an EPR-backed filesystem tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionManifest {
    pub root: ProjectionRoot,
    pub entries: Vec<ProjectionEntry>,
    pub metadata: Value,
}
