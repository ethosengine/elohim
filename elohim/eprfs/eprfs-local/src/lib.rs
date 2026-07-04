//! Local materialization of EPR filesystem projection manifests.

use std::path::{Path, PathBuf};

use eprfs_core::{
    BlobPresence, EntryKind, EprfsError, EprfsStorage, FetchPolicy, MaterializationPolicy,
    ProjectionEntry, ProjectionManifest, Result,
};

/// Writes an EPR projection manifest into an ordinary local directory.
pub struct LocalMaterializer<S> {
    storage: S,
}

impl<S> LocalMaterializer<S> {
    pub fn new(storage: S) -> Self {
        Self { storage }
    }
}

impl<S> LocalMaterializer<S>
where
    S: EprfsStorage,
{
    pub async fn materialize(
        &self,
        manifest: &ProjectionManifest,
        target: impl AsRef<Path>,
        policy: MaterializationPolicy,
    ) -> Result<MaterializationReport> {
        let target = target.as_ref();
        manifest.validate()?;
        let mut report = MaterializationReport::default();

        for entry in &manifest.entries {
            self.materialize_entry(entry, target, &policy, &mut report)
                .await?;
        }

        Ok(report)
    }

    async fn materialize_entry(
        &self,
        entry: &ProjectionEntry,
        target: &Path,
        policy: &MaterializationPolicy,
        report: &mut MaterializationReport,
    ) -> Result<()> {
        let path = target.join(entry.path.as_path());

        match entry.kind {
            EntryKind::Directory => {
                create_dir_all(&path).await?;
                report.directories += 1;
            }
            EntryKind::Symlink => {
                let Some(blob) = entry.blob.as_ref() else {
                    return Err(EprfsError::MissingBlob(entry.path.as_path().to_path_buf()));
                };

                match self.storage.has_blob(blob).await? {
                    BlobPresence::Local => {
                        let handle = self
                            .storage
                            .fetch_blob(blob, FetchPolicy::LocalOnly)
                            .await?;
                        write_symlink(&path, &handle.bytes).await?;
                        report.symlinks_written += 1;
                    }
                    BlobPresence::Remote | BlobPresence::Unknown => match policy {
                        MaterializationPolicy::FetchMissing => {
                            let handle = self
                                .storage
                                .fetch_blob(blob, FetchPolicy::FetchIfMissing)
                                .await?;
                            write_symlink(&path, &handle.bytes).await?;
                            report.symlinks_written += 1;
                            report.files_fetched += 1;
                        }
                        MaterializationPolicy::Sparse => {
                            write_sparse_marker(&path, blob.as_str()).await?;
                            report.skipped_sparse += 1;
                        }
                        MaterializationPolicy::LocalOnly => {
                            return Err(EprfsError::BlobNotLocal(blob.clone()));
                        }
                    },
                    BlobPresence::Missing => match policy {
                        MaterializationPolicy::Sparse => {
                            write_sparse_marker(&path, blob.as_str()).await?;
                            report.skipped_sparse += 1;
                        }
                        _ => return Err(EprfsError::BlobNotFound(blob.clone())),
                    },
                }
            }
            EntryKind::File => {
                let Some(blob) = entry.blob.as_ref() else {
                    return Err(EprfsError::MissingBlob(entry.path.as_path().to_path_buf()));
                };

                match self.storage.has_blob(blob).await? {
                    BlobPresence::Local => {
                        let handle = self
                            .storage
                            .fetch_blob(blob, FetchPolicy::LocalOnly)
                            .await?;
                        write_file(&path, &handle.bytes).await?;
                        apply_executable_bit(&path, entry.executable).await?;
                        report.files_written += 1;
                    }
                    BlobPresence::Remote | BlobPresence::Unknown => match policy {
                        MaterializationPolicy::FetchMissing => {
                            let handle = self
                                .storage
                                .fetch_blob(blob, FetchPolicy::FetchIfMissing)
                                .await?;
                            write_file(&path, &handle.bytes).await?;
                            apply_executable_bit(&path, entry.executable).await?;
                            report.files_written += 1;
                            report.files_fetched += 1;
                        }
                        MaterializationPolicy::Sparse => {
                            write_sparse_marker(&path, blob.as_str()).await?;
                            report.skipped_sparse += 1;
                        }
                        MaterializationPolicy::LocalOnly => {
                            return Err(EprfsError::BlobNotLocal(blob.clone()));
                        }
                    },
                    BlobPresence::Missing => match policy {
                        MaterializationPolicy::Sparse => {
                            write_sparse_marker(&path, blob.as_str()).await?;
                            report.skipped_sparse += 1;
                        }
                        _ => return Err(EprfsError::BlobNotFound(blob.clone())),
                    },
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MaterializationReport {
    pub directories: usize,
    pub files_written: usize,
    pub symlinks_written: usize,
    pub files_fetched: usize,
    pub skipped_sparse: usize,
}

async fn create_dir_all(path: &Path) -> Result<()> {
    tokio::fs::create_dir_all(path)
        .await
        .map_err(|source| EprfsError::Io {
            path: path.to_path_buf(),
            source,
        })
}

async fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        create_dir_all(parent).await?;
    }
    tokio::fs::write(path, bytes)
        .await
        .map_err(|source| EprfsError::Io {
            path: path.to_path_buf(),
            source,
        })
}

#[cfg(unix)]
async fn write_symlink(path: &Path, target: &[u8]) -> Result<()> {
    if target.is_empty() {
        return Err(EprfsError::InvalidProjectionManifest(format!(
            "symlink target is empty for {}",
            path.display()
        )));
    }

    if let Some(parent) = path.parent() {
        create_dir_all(parent).await?;
    }

    match tokio::fs::remove_file(path).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(EprfsError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    }

    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

    std::os::unix::fs::symlink(OsStr::from_bytes(target), path).map_err(|source| EprfsError::Io {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(not(unix))]
async fn write_symlink(path: &Path, _target: &[u8]) -> Result<()> {
    Err(EprfsError::Storage(format!(
        "symlink materialization is not supported on this platform: {}",
        path.display()
    )))
}

#[cfg(unix)]
async fn apply_executable_bit(path: &Path, executable: bool) -> Result<()> {
    if !executable {
        return Ok(());
    }

    use std::os::unix::fs::PermissionsExt;

    let metadata = tokio::fs::metadata(path)
        .await
        .map_err(|source| EprfsError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(permissions.mode() | 0o111);
    tokio::fs::set_permissions(path, permissions)
        .await
        .map_err(|source| EprfsError::Io {
            path: path.to_path_buf(),
            source,
        })
}

#[cfg(not(unix))]
async fn apply_executable_bit(_path: &Path, _executable: bool) -> Result<()> {
    Ok(())
}

async fn write_sparse_marker(path: &Path, cid: &str) -> Result<()> {
    let marker = sparse_marker_path(path);
    write_file(&marker, format!("remote blob: {cid}\n").as_bytes()).await
}

fn sparse_marker_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("entry");
    path.with_file_name(format!("{file_name}.eprfs-remote"))
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use eprfs_core::{
        BlobCid, EntryKind, ProjectionEntry, ProjectionId, ProjectionPath, ProjectionRoot,
        ProjectionStatus,
    };
    use eprfs_storage::MemoryStorage;

    use super::*;

    #[tokio::test]
    async fn materializes_local_file() {
        let storage = MemoryStorage::default();
        storage
            .insert_blob(BlobCid::from("bafk-test"), Bytes::from_static(b"hello"))
            .await;

        let manifest = ProjectionManifest {
            root: ProjectionRoot {
                id: ProjectionId::new("test"),
                root: "epr:test".into(),
            },
            entries: vec![ProjectionEntry::file(
                ProjectionPath::new("README.md").unwrap(),
                "bafk-test".into(),
            )],
            metadata: serde_json::Value::Null,
        };

        let target = std::env::temp_dir().join(format!("eprfs-local-test-{}", std::process::id()));
        let _ = tokio::fs::remove_dir_all(&target).await;

        let materializer = LocalMaterializer::new(storage);
        let report = materializer
            .materialize(&manifest, &target, MaterializationPolicy::LocalOnly)
            .await
            .unwrap();

        assert_eq!(report.files_written, 1);
        assert_eq!(
            tokio::fs::read_to_string(target.join("README.md"))
                .await
                .unwrap(),
            "hello"
        );

        let _ = tokio::fs::remove_dir_all(&target).await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn materializes_local_symlink() {
        let storage = MemoryStorage::default();
        storage
            .insert_blob(
                BlobCid::from("target-blob"),
                Bytes::from_static(b"actual.txt"),
            )
            .await;

        let manifest = ProjectionManifest {
            root: ProjectionRoot {
                id: ProjectionId::new("test"),
                root: "epr:test".into(),
            },
            entries: vec![ProjectionEntry {
                path: ProjectionPath::new("current").unwrap(),
                kind: EntryKind::Symlink,
                epr: None,
                blob: Some("target-blob".into()),
                size_bytes: Some(10),
                executable: false,
                status: ProjectionStatus::Unknown,
                metadata: serde_json::Value::Null,
            }],
            metadata: serde_json::Value::Null,
        };

        let target =
            std::env::temp_dir().join(format!("eprfs-local-symlink-test-{}", std::process::id()));
        let _ = tokio::fs::remove_dir_all(&target).await;

        let materializer = LocalMaterializer::new(storage);
        let report = materializer
            .materialize(&manifest, &target, MaterializationPolicy::LocalOnly)
            .await
            .unwrap();

        assert_eq!(report.symlinks_written, 1);
        assert_eq!(
            tokio::fs::read_link(target.join("current")).await.unwrap(),
            PathBuf::from("actual.txt")
        );

        let _ = tokio::fs::remove_dir_all(&target).await;
    }
}
