//! Domain adapter: elohim capability-package tree -> `eprfs_core::ProjectionManifest`.
//!
//! This crate is the *domain* seam. All meaning about what an elohim
//! capability-package tree is lives here; `eprfs-core` stays domain-neutral and
//! only enforces the structural projection invariants (see `eprfs/CLAUDE.md`:
//! `domain adapter -> eprfs -> elohim-storage`).
//!
//! Content addressing is single-sourced through [`eprfs_core::BlobCid::compute`]
//! (CIDv1 / dag-cbor / sha2-256) — this crate never re-implements a CID.

use std::path::Path;

use eprfs_core::{
    BlobCid, EntryKind, EprRef, EprfsError, ProjectionEntry, ProjectionId, ProjectionManifest,
    ProjectionPath, ProjectionRoot, ProjectionSource, ProjectionSourceKind, ProjectionStatus,
    Result,
};
use serde_json::Value;

/// Namespace stamped on every projection source this adapter emits.
const NAMESPACE: &str = "elohim-agent";

/// Build a validated [`ProjectionManifest`] from a capability-package tree on disk.
///
/// Walks `root` deterministically (entries sorted by file name at every level).
/// Directories project as [`EntryKind::Directory`] with a `Container` source and
/// no blob; files project as [`EntryKind::File`] with `blob =
/// BlobCid::compute(bytes)`, a `Content` source, byte size, and
/// [`ProjectionStatus::Local`]. The returned manifest is guaranteed to satisfy
/// [`ProjectionManifest::validate`].
pub fn manifest_from_package_tree(root: &Path) -> Result<ProjectionManifest> {
    let mut entries = Vec::new();
    walk(root, root, &mut entries)?;

    let manifest = ProjectionManifest {
        root: ProjectionRoot {
            id: ProjectionId::new("elohim-agent-packages"),
            root: EprRef::new("epr:elohim-agent/packages"),
        },
        entries,
        metadata: Value::Null,
    };

    // Fail loudly rather than hand back a structurally-invalid projection.
    manifest.validate()?;
    Ok(manifest)
}

/// Collect `(BlobCid, bytes)` for every file in the tree — the raw material a
/// later round-trip test seeds into storage before materializing the manifest.
/// Order is deterministic (same sorted walk as [`manifest_from_package_tree`]).
pub fn blobs_for_tree(root: &Path) -> Result<Vec<(BlobCid, Vec<u8>)>> {
    let mut blobs = Vec::new();
    collect_blobs(root, &mut blobs)?;
    Ok(blobs)
}

/// Sorted directory listing — the single source of deterministic ordering shared
/// by both walks.
fn sorted_children(dir: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut children: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
        .map_err(|source| EprfsError::Io {
            path: dir.to_path_buf(),
            source,
        })?
        .map(|entry| {
            entry.map(|e| e.path()).map_err(|source| EprfsError::Io {
                path: dir.to_path_buf(),
                source,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    children.sort();
    Ok(children)
}

/// True for a directory (does not follow symlinks — a symlink projects as a file
/// via its own bytes, never as a container).
fn is_dir(path: &Path) -> Result<bool> {
    let meta = std::fs::symlink_metadata(path).map_err(|source| EprfsError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(meta.is_dir())
}

/// Read a file's bytes, mapping I/O failure onto the eprfs error surface.
fn read_bytes(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|source| EprfsError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// The domain-neutral source id for an entry: `<Kind>:<file-name>`.
fn source_id(kind: &str, path: &Path) -> String {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    format!("{kind}:{name}")
}

fn walk(base: &Path, current: &Path, entries: &mut Vec<ProjectionEntry>) -> Result<()> {
    for child in sorted_children(current)? {
        let rel = child
            .strip_prefix(base)
            .map_err(|_| EprfsError::InvalidProjectionPath(child.display().to_string()))?;
        let path = ProjectionPath::new(rel)?;

        if is_dir(&child)? {
            entries.push(ProjectionEntry {
                path,
                kind: EntryKind::Directory,
                source: Some(ProjectionSource::new(
                    NAMESPACE,
                    ProjectionSourceKind::Container,
                    source_id("Dir", &child),
                )),
                epr: None,
                blob: None,
                size_bytes: None,
                executable: false,
                status: ProjectionStatus::Local,
                metadata: Value::Null,
            });
            walk(base, &child, entries)?;
        } else {
            let bytes = read_bytes(&child)?;
            let mut entry = ProjectionEntry::file(path, BlobCid::compute(&bytes));
            entry.source = Some(ProjectionSource::new(
                NAMESPACE,
                ProjectionSourceKind::Content,
                source_id("File", &child),
            ));
            entry.size_bytes = Some(bytes.len() as u64);
            entry.status = ProjectionStatus::Local;
            entries.push(entry);
        }
    }
    Ok(())
}

fn collect_blobs(current: &Path, blobs: &mut Vec<(BlobCid, Vec<u8>)>) -> Result<()> {
    for child in sorted_children(current)? {
        if is_dir(&child)? {
            collect_blobs(&child, blobs)?;
        } else {
            let bytes = read_bytes(&child)?;
            blobs.push((BlobCid::compute(&bytes), bytes));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// A throwaway on-disk tree of exactly 1 directory + 2 files, cleaned on drop.
    struct TempTree {
        root: std::path::PathBuf,
    }

    impl TempTree {
        fn new() -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "elohim-agent-adapter-{}-{}",
                std::process::id(),
                nanos
            ));
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    const A_BYTES: &[u8] = b"package: skill\nname: alpha\n";
    const B_BYTES: &[u8] = b"# nested readme\n";

    fn seed(tree: &TempTree) {
        // top-level file
        fs::write(tree.root.join("manifest.yaml"), A_BYTES).unwrap();
        // one directory containing one file
        let sub = tree.root.join("docs");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("readme.md"), B_BYTES).unwrap();
    }

    #[test]
    fn maps_tree_to_valid_manifest() {
        let tree = TempTree::new();
        seed(&tree);

        let manifest = manifest_from_package_tree(&tree.root).expect("manifest builds");

        assert!(
            manifest.validate().is_ok(),
            "manifest must satisfy eprfs invariants"
        );
        assert_eq!(manifest.entries.len(), 3, "1 dir + 2 files == 3 entries");

        // The top-level file's blob is the canonical CID of its bytes.
        let expected = BlobCid::compute(A_BYTES);
        let file_entry = manifest
            .entries
            .iter()
            .find(|e| e.path.as_path() == Path::new("manifest.yaml"))
            .expect("top-level file entry present");
        assert_eq!(file_entry.kind, EntryKind::File);
        assert_eq!(file_entry.blob.as_ref(), Some(&expected));
        assert_eq!(file_entry.status, ProjectionStatus::Local);
        assert_eq!(file_entry.size_bytes, Some(A_BYTES.len() as u64));
    }

    #[test]
    fn blobs_for_tree_covers_every_file() {
        let tree = TempTree::new();
        seed(&tree);

        let blobs = blobs_for_tree(&tree.root).expect("blobs collected");
        assert_eq!(blobs.len(), 2, "two files -> two blobs");

        let expected_a = BlobCid::compute(A_BYTES);
        let expected_b = BlobCid::compute(B_BYTES);
        assert!(blobs
            .iter()
            .any(|(cid, bytes)| cid == &expected_a && bytes == A_BYTES));
        assert!(blobs
            .iter()
            .any(|(cid, bytes)| cid == &expected_b && bytes == B_BYTES));
    }
}
