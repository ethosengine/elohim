use std::{
    collections::HashSet,
    path::{Component, Path, PathBuf},
};

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

/// Domain-neutral identity for the object that produced a projection entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionSource {
    pub namespace: String,
    pub kind: ProjectionSourceKind,
    pub id: String,
}

impl ProjectionSource {
    pub fn new(
        namespace: impl Into<String>,
        kind: ProjectionSourceKind,
        id: impl Into<String>,
    ) -> Self {
        Self {
            namespace: namespace.into(),
            kind,
            id: id.into(),
        }
    }
}

/// Broad source class independent of any specific storage or domain adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectionSourceKind {
    Content,
    Container,
    Link,
    External,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<ProjectionSource>,
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
            source: None,
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

impl ProjectionManifest {
    /// Validate projection invariants that must hold across all domain adapters.
    pub fn validate(&self) -> Result<()> {
        let mut paths = HashSet::with_capacity(self.entries.len());

        for entry in &self.entries {
            if let Some(source) = &entry.source {
                if source.namespace.trim().is_empty() || source.id.trim().is_empty() {
                    return Err(EprfsError::InvalidProjectionManifest(format!(
                        "projection source is missing namespace or id for path: {}",
                        entry.path.as_path().display()
                    )));
                }
            }

            if !paths.insert(entry.path.clone()) {
                return Err(EprfsError::InvalidProjectionManifest(format!(
                    "duplicate projection path: {}",
                    entry.path.as_path().display()
                )));
            }

            match entry.kind {
                EntryKind::Directory if entry.blob.is_some() => {
                    return Err(EprfsError::InvalidProjectionManifest(format!(
                        "directory path has blob: {}",
                        entry.path.as_path().display()
                    )));
                }
                EntryKind::Directory
                    if matches!(
                        entry.source.as_ref().map(|source| &source.kind),
                        Some(ProjectionSourceKind::Content | ProjectionSourceKind::Link)
                    ) =>
                {
                    return Err(EprfsError::InvalidProjectionManifest(format!(
                        "directory path has content/link source: {}",
                        entry.path.as_path().display()
                    )));
                }
                EntryKind::File | EntryKind::Symlink if entry.blob.is_none() => {
                    return Err(EprfsError::MissingBlob(entry.path.as_path().to_path_buf()));
                }
                EntryKind::File
                    if matches!(
                        entry.source.as_ref().map(|source| &source.kind),
                        Some(ProjectionSourceKind::Container | ProjectionSourceKind::External)
                    ) =>
                {
                    return Err(EprfsError::InvalidProjectionManifest(format!(
                        "file path has container/external source: {}",
                        entry.path.as_path().display()
                    )));
                }
                _ => {}
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_duplicate_projection_paths() {
        let path = ProjectionPath::new("README.md").unwrap();
        let manifest = ProjectionManifest {
            root: ProjectionRoot {
                id: ProjectionId::new("test"),
                root: EprRef::new("epr:test"),
            },
            entries: vec![
                ProjectionEntry::file(path.clone(), BlobCid::compute(b"blob:one")),
                ProjectionEntry::file(path, BlobCid::compute(b"blob:two")),
            ],
            metadata: Value::Null,
        };

        assert!(matches!(
            manifest.validate(),
            Err(EprfsError::InvalidProjectionManifest(_))
        ));
    }

    #[test]
    fn rejects_blobless_symlink() {
        let manifest = ProjectionManifest {
            root: ProjectionRoot {
                id: ProjectionId::new("test"),
                root: EprRef::new("epr:test"),
            },
            entries: vec![ProjectionEntry {
                path: ProjectionPath::new("current").unwrap(),
                kind: EntryKind::Symlink,
                source: None,
                epr: None,
                blob: None,
                size_bytes: None,
                executable: false,
                status: ProjectionStatus::Unknown,
                metadata: Value::Null,
            }],
            metadata: Value::Null,
        };

        assert!(matches!(
            manifest.validate(),
            Err(EprfsError::MissingBlob(path)) if path == Path::new("current")
        ));
    }

    #[test]
    fn rejects_file_with_external_source() {
        let manifest = ProjectionManifest {
            root: ProjectionRoot {
                id: ProjectionId::new("test"),
                root: EprRef::new("epr:test"),
            },
            entries: vec![ProjectionEntry {
                path: ProjectionPath::new("vendor/lib").unwrap(),
                kind: EntryKind::File,
                source: Some(ProjectionSource::new(
                    "git",
                    ProjectionSourceKind::External,
                    "abc123",
                )),
                epr: None,
                blob: Some(BlobCid::compute(b"blob:vendor")),
                size_bytes: None,
                executable: false,
                status: ProjectionStatus::Unknown,
                metadata: Value::Null,
            }],
            metadata: Value::Null,
        };

        assert!(matches!(
            manifest.validate(),
            Err(EprfsError::InvalidProjectionManifest(_))
        ));
    }

    #[test]
    fn rejects_empty_source_identity() {
        let manifest = ProjectionManifest {
            root: ProjectionRoot {
                id: ProjectionId::new("test"),
                root: EprRef::new("epr:test"),
            },
            entries: vec![ProjectionEntry {
                path: ProjectionPath::new("README.md").unwrap(),
                kind: EntryKind::File,
                source: Some(ProjectionSource::new(
                    "",
                    ProjectionSourceKind::Content,
                    "abc123",
                )),
                epr: None,
                blob: Some(BlobCid::compute(b"blob:readme")),
                size_bytes: None,
                executable: false,
                status: ProjectionStatus::Unknown,
                metadata: Value::Null,
            }],
            metadata: Value::Null,
        };

        assert!(matches!(
            manifest.validate(),
            Err(EprfsError::InvalidProjectionManifest(_))
        ));
    }
}
