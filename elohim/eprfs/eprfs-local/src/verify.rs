//! Projection drift verification: compare a manifest to an on-disk tree.
//! The inverse of `LocalMaterializer` — feeds `projection-drift-detected`.

use std::path::Path;

use eprfs_core::{
    BlobCid, EntryKind, LocalOverlayStatus, ProjectionManifest, ProjectionPath, Result,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryDrift {
    pub path: ProjectionPath,
    pub expected: BlobCid,
    pub actual: Option<BlobCid>,
    pub status: LocalOverlayStatus,
}

pub async fn verify_projection(
    manifest: &ProjectionManifest,
    target: impl AsRef<Path>,
) -> Result<Vec<EntryDrift>> {
    let target = target.as_ref();
    let mut drifts = Vec::new();

    for entry in &manifest.entries {
        if entry.kind != EntryKind::File {
            continue; // V2: agents are files; dir/symlink drift is a later wave.
        }
        let Some(expected) = entry.blob.clone() else {
            continue;
        };
        let path = target.join(entry.path.as_path());

        let (actual, status) = match tokio::fs::read(&path).await {
            Ok(bytes) => {
                let actual = BlobCid::compute(&bytes);
                let status = if actual == expected {
                    LocalOverlayStatus::Clean
                } else {
                    LocalOverlayStatus::Dirty
                };
                (Some(actual), status)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                (None, LocalOverlayStatus::Dirty)
            }
            Err(source) => {
                return Err(eprfs_core::EprfsError::Io {
                    path: path.clone(),
                    source,
                });
            }
        };

        drifts.push(EntryDrift {
            path: entry.path.clone(),
            expected,
            actual,
            status,
        });
    }

    Ok(drifts)
}

/// True iff any entry is not Clean — the signal that fires `projection-drift-detected`.
pub fn has_drift(drifts: &[EntryDrift]) -> bool {
    drifts.iter().any(|d| d.status != LocalOverlayStatus::Clean)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eprfs_core::{
        BlobCid, ProjectionEntry, ProjectionId, ProjectionManifest, ProjectionPath, ProjectionRoot,
    };

    fn manifest_for(bytes: &[u8]) -> ProjectionManifest {
        ProjectionManifest {
            root: ProjectionRoot {
                id: ProjectionId::new("t"),
                root: "epr:t".into(),
            },
            entries: vec![ProjectionEntry::file(
                ProjectionPath::new("a.md").unwrap(),
                BlobCid::compute(bytes),
            )],
            metadata: serde_json::Value::Null,
        }
    }

    #[tokio::test]
    async fn clean_when_disk_matches_manifest() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join("a.md"), b"hello")
            .await
            .unwrap();
        let drifts = verify_projection(&manifest_for(b"hello"), dir.path())
            .await
            .unwrap();
        assert_eq!(drifts[0].status, LocalOverlayStatus::Clean);
        assert!(!has_drift(&drifts));
    }

    #[tokio::test]
    async fn dirty_when_disk_was_hand_edited() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join("a.md"), b"HAND EDITED")
            .await
            .unwrap();
        let drifts = verify_projection(&manifest_for(b"hello"), dir.path())
            .await
            .unwrap();
        assert_eq!(drifts[0].status, LocalOverlayStatus::Dirty);
        assert!(drifts[0].actual.is_some());
        assert!(has_drift(&drifts));
    }

    #[tokio::test]
    async fn dirty_and_absent_when_not_materialized() {
        let dir = tempfile::tempdir().unwrap();
        let drifts = verify_projection(&manifest_for(b"hello"), dir.path())
            .await
            .unwrap();
        assert_eq!(drifts[0].status, LocalOverlayStatus::Dirty);
        assert!(drifts[0].actual.is_none());
        assert!(has_drift(&drifts));
    }
}
