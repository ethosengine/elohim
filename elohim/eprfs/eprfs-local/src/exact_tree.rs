//! Fresh-destination, exact restore of a verified immutable tree.
//!
//! The destination's existing parent must be trusted against concurrent mutation.
//! We reject symlinked ancestors at entry and use a parent directory descriptor for
//! no-replace promotion, but stage construction is path based. A hostile process
//! able to rename that parent or alter its children is outside this lane's threat
//! boundary. Promotion is atomic on supported Linux filesystems; power-loss
//! durability is not claimed because this lane does not fsync files and parents.

use std::path::{Component, Path, PathBuf};

use eprfs_core::{
    tree::{load_verified_tree, TreeEntryKind, TreeLimits, VerifiedTree},
    BlobCid, EprfsStorage, FetchPolicy,
};
use eprfs_host::HostProfile;
#[cfg(target_os = "linux")]
use eprfs_host::{Capability, SymlinkMode};

#[derive(Debug, thiserror::Error)]
pub enum ExactRestoreError {
    #[error(transparent)]
    Tree(#[from] Box<eprfs_core::tree::TreeError>),
    #[error("exact restore refused: {0}")]
    Refused(String),
    #[error("filesystem I/O at {path}: {source}", path = .path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl From<eprfs_core::tree::TreeError> for ExactRestoreError {
    fn from(error: eprfs_core::tree::TreeError) -> Self {
        Self::Tree(Box::new(error))
    }
}

type RestoreResult<T> = std::result::Result<T, ExactRestoreError>;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ExactRestoreReport {
    pub files: usize,
    pub directories: usize,
    pub symlinks: usize,
    pub bytes_written: usize,
}

/// Verify the entire closure and restore it into a new, absent destination.
/// External entries make a complete exact filesystem restore impossible and
/// are refused. Storage CID integrity proves bytes, not authority for the root.
pub async fn restore_exact_tree<S: EprfsStorage>(
    storage: &S,
    root: &BlobCid,
    destination: impl AsRef<Path>,
    fetch_policy: FetchPolicy,
    limits: &TreeLimits,
    host: &HostProfile,
) -> RestoreResult<ExactRestoreReport> {
    let destination = destination.as_ref();
    preflight_host(host)?;
    let parent = preflight_destination(destination)?;
    let verified = load_verified_tree(storage, root, fetch_policy, limits).await?;
    preflight_tree(&verified, limits)?;
    stage_and_promote(&verified, destination, &parent, || Ok(()))
}

#[cfg(target_os = "linux")]
fn preflight_host(host: &HostProfile) -> RestoreResult<()> {
    if host.atomic_rename != Capability::Supported
        || host.executable_bits != Capability::Supported
        || host.case_sensitive_paths != Capability::Supported
        || host.symlinks != SymlinkMode::Native
    {
        return Err(ExactRestoreError::Refused(format!(
            "host profile {} cannot express exact tree restore",
            host.name
        )));
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn preflight_host(_host: &HostProfile) -> RestoreResult<()> {
    Err(ExactRestoreError::Refused(
        "fresh-target atomic exact restore is supported on Linux only".into(),
    ))
}

fn preflight_destination(destination: &Path) -> RestoreResult<PathBuf> {
    let absolute = if destination.is_absolute() {
        destination.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|source| io_error(destination, source))?
            .join(destination)
    };
    if absolute
        .components()
        .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Err(ExactRestoreError::Refused(
            "destination must not contain dot or parent traversal".into(),
        ));
    }
    let name = absolute.file_name().ok_or_else(|| {
        ExactRestoreError::Refused("destination must name a new directory".into())
    })?;
    if name.is_empty() {
        return Err(ExactRestoreError::Refused("empty destination name".into()));
    }
    let parent = absolute
        .parent()
        .ok_or_else(|| ExactRestoreError::Refused("destination has no parent".into()))?;
    let mut prefix = PathBuf::new();
    for component in parent.components() {
        prefix.push(component.as_os_str());
        let meta =
            std::fs::symlink_metadata(&prefix).map_err(|source| io_error(&prefix, source))?;
        if meta.file_type().is_symlink() || !meta.is_dir() {
            return Err(ExactRestoreError::Refused(format!(
                "destination parent chain is not a real directory at {}",
                prefix.display()
            )));
        }
    }
    match std::fs::symlink_metadata(&absolute) {
        Ok(_) => Err(ExactRestoreError::Refused(format!(
            "destination already exists: {}",
            absolute.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(parent.to_path_buf()),
        Err(source) => Err(io_error(&absolute, source)),
    }
}

fn preflight_tree(tree: &VerifiedTree, limits: &TreeLimits) -> RestoreResult<()> {
    if !tree.external_paths.is_empty() {
        return Err(ExactRestoreError::Refused(format!(
            "tree has {} external boundaries; complete exact filesystem restore is unavailable",
            tree.external_paths.len()
        )));
    }
    let mut expanded_bytes = 0usize;
    let mut pending = vec![tree.root_node()];
    while let Some(node) = pending.pop() {
        let mut folded = std::collections::HashSet::new();
        for entry in &node.entries {
            let name = &entry.name;
            if name.is_empty()
                || name == b"."
                || name == b".."
                || name.contains(&0)
                || name.contains(&b'/')
                || name.len() > 255
            {
                return Err(ExactRestoreError::Refused("unsafe host entry name".into()));
            }
            // Conservative policy: refuse ASCII case twins even on a host that
            // advertises case sensitivity. A mounted directory may override
            // the platform default, and a case-folded child would lose bytes.
            if !folded.insert(name.iter().map(u8::to_ascii_lowercase).collect::<Vec<_>>()) {
                return Err(ExactRestoreError::Refused(
                    "case-colliding sibling names".into(),
                ));
            }
            match &entry.kind {
                TreeEntryKind::File { blob, .. } => {
                    let bytes = tree.leaf(blob.as_blob_cid()).ok_or_else(|| {
                        ExactRestoreError::Refused("verified file leaf missing".into())
                    })?;
                    expanded_bytes = expanded_bytes.checked_add(bytes.len()).ok_or_else(|| {
                        ExactRestoreError::Refused("expanded restore byte budget overflow".into())
                    })?;
                }
                TreeEntryKind::Symlink { target } => {
                    let bytes = tree.leaf(target.as_blob_cid()).ok_or_else(|| {
                        ExactRestoreError::Refused("verified symlink leaf missing".into())
                    })?;
                    if bytes.is_empty() || bytes.contains(&0) || bytes.len() > 4095 {
                        return Err(ExactRestoreError::Refused(
                            "symlink target is not host-representable".into(),
                        ));
                    }
                    expanded_bytes = expanded_bytes.checked_add(bytes.len()).ok_or_else(|| {
                        ExactRestoreError::Refused("expanded restore byte budget overflow".into())
                    })?;
                }
                TreeEntryKind::Directory { tree: link } => {
                    pending.push(tree.nodes.get(link.as_blob_cid()).ok_or_else(|| {
                        ExactRestoreError::Refused("verified directory missing".into())
                    })?);
                }
                TreeEntryKind::External { .. } => {
                    return Err(ExactRestoreError::Refused("external boundary".into()));
                }
            }
            if expanded_bytes > limits.max_total_bytes {
                return Err(ExactRestoreError::Refused(
                    "expanded restore byte budget exceeded".into(),
                ));
            }
        }
    }
    Ok(())
}

fn stage_and_promote(
    tree: &VerifiedTree,
    destination: &Path,
    parent: &Path,
    before_promote: impl FnOnce() -> RestoreResult<()>,
) -> RestoreResult<ExactRestoreReport> {
    let stage = tempfile::Builder::new()
        .prefix(".eprfs-restore-")
        .tempdir_in(parent)
        .map_err(|source| io_error(parent, source))?;
    let mut report = ExactRestoreReport::default();
    write_node(tree, tree.root_node(), stage.path(), &mut report)?;
    // TempDir's private root mode (0700) is retained at the destination.
    // Tree entries do not encode root-directory mode, and keeping it private
    // avoids exposing staged private content before promotion.
    before_promote()?;
    promote_no_replace(stage.path(), destination, parent)?;
    // The old temporary pathname no longer exists after promotion. Retire the
    // cleanup guard so it cannot act on a future occupant of that pathname.
    let _old_stage_path = stage.keep();
    Ok(report)
}

#[cfg(unix)]
fn write_node(
    tree: &VerifiedTree,
    node: &eprfs_core::tree::TreeNode,
    directory: &Path,
    report: &mut ExactRestoreReport,
) -> RestoreResult<()> {
    use std::os::unix::ffi::OsStrExt;
    for entry in &node.entries {
        let path = directory.join(std::ffi::OsStr::from_bytes(&entry.name));
        match &entry.kind {
            TreeEntryKind::File { blob, executable } => {
                let bytes = tree.leaf(blob.as_blob_cid()).ok_or_else(|| {
                    ExactRestoreError::Refused("verified file leaf missing".into())
                })?;
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)
                    .map_err(|source| io_error(&path, source))?;
                std::io::Write::write_all(&mut file, bytes)
                    .map_err(|source| io_error(&path, source))?;
                set_mode(&path, if *executable { 0o755 } else { 0o644 })?;
                report.files += 1;
                report.bytes_written += bytes.len();
            }
            TreeEntryKind::Directory { tree: link } => {
                std::fs::create_dir(&path).map_err(|source| io_error(&path, source))?;
                let child = tree.nodes.get(link.as_blob_cid()).ok_or_else(|| {
                    ExactRestoreError::Refused("verified directory missing".into())
                })?;
                write_node(tree, child, &path, report)?;
                set_mode(&path, 0o755)?;
                report.directories += 1;
            }
            TreeEntryKind::Symlink { target } => {
                let bytes = tree.leaf(target.as_blob_cid()).ok_or_else(|| {
                    ExactRestoreError::Refused("verified symlink leaf missing".into())
                })?;
                std::os::unix::fs::symlink(std::ffi::OsStr::from_bytes(bytes), &path)
                    .map_err(|source| io_error(&path, source))?;
                report.symlinks += 1;
            }
            TreeEntryKind::External { .. } => {
                return Err(ExactRestoreError::Refused("external boundary".into()));
            }
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn write_node(
    _tree: &VerifiedTree,
    _node: &eprfs_core::tree::TreeNode,
    _directory: &Path,
    _report: &mut ExactRestoreReport,
) -> RestoreResult<()> {
    Err(ExactRestoreError::Refused(
        "native byte paths unavailable".into(),
    ))
}

#[cfg(target_os = "linux")]
fn promote_no_replace(stage: &Path, destination: &Path, parent: &Path) -> RestoreResult<()> {
    use rustix::fs::{renameat_with, RenameFlags};
    let dir = std::fs::File::open(parent).map_err(|source| io_error(parent, source))?;
    let from = stage
        .file_name()
        .ok_or_else(|| ExactRestoreError::Refused("stage has no name".into()))?;
    let to = destination
        .file_name()
        .ok_or_else(|| ExactRestoreError::Refused("destination has no name".into()))?;
    renameat_with(&dir, from, &dir, to, RenameFlags::NOREPLACE)
        .map_err(|error| io_error(destination, std::io::Error::from(error)))
}

#[cfg(not(target_os = "linux"))]
fn promote_no_replace(_stage: &Path, _destination: &Path, _parent: &Path) -> RestoreResult<()> {
    Err(ExactRestoreError::Refused(
        "Linux no-replace rename unavailable".into(),
    ))
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> RestoreResult<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
        .map_err(|source| io_error(path, source))
}

fn io_error(path: &Path, source: std::io::Error) -> ExactRestoreError {
    ExactRestoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use bytes::Bytes;
    use eprfs_core::tree::{encode_tree, TreeEntry, TreeNode};
    use eprfs_core::BlobLink;
    use eprfs_storage::MemoryStorage;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::PermissionsExt;

    async fn fixture() -> (MemoryStorage, BlobCid) {
        let store = MemoryStorage::default();
        let limits = TreeLimits::default();
        let file = b"\0binary\xff\n";
        let file_cid = BlobCid::compute_raw(file);
        store
            .insert_blob(file_cid.clone(), Bytes::copy_from_slice(file))
            .await;
        let target = b"../\xff-target";
        let target_cid = BlobCid::compute_raw(target);
        store
            .insert_blob(target_cid.clone(), Bytes::copy_from_slice(target))
            .await;
        let inner = TreeNode::new(vec![
            TreeEntry {
                name: b"run\xff".to_vec(),
                kind: TreeEntryKind::File {
                    blob: BlobLink::from(file_cid),
                    executable: true,
                },
            },
            TreeEntry {
                name: b"link".to_vec(),
                kind: TreeEntryKind::Symlink {
                    target: BlobLink::from(target_cid),
                },
            },
        ])
        .unwrap();
        let (inner_cid, inner_bytes) = encode_tree(&inner, &limits).unwrap();
        store
            .insert_blob(inner_cid.clone(), Bytes::from(inner_bytes))
            .await;
        let outer = TreeNode::new(vec![TreeEntry {
            name: b"deep".to_vec(),
            kind: TreeEntryKind::Directory {
                tree: BlobLink::from(inner_cid),
            },
        }])
        .unwrap();
        let (root, bytes) = encode_tree(&outer, &limits).unwrap();
        store.insert_blob(root.clone(), Bytes::from(bytes)).await;
        (store, root)
    }

    #[tokio::test]
    async fn restores_nested_raw_bytes_executable_mode_non_utf8_names_and_symlink_target() {
        let (store, root) = fixture().await;
        let parent = tempfile::tempdir().unwrap();
        let destination = parent.path().join("restored");
        let report = restore_exact_tree(
            &store,
            &root,
            &destination,
            FetchPolicy::LocalOnly,
            &TreeLimits::default(),
            &HostProfile::linux_native(),
        )
        .await
        .unwrap();
        assert_eq!(report.files, 1);
        assert_eq!(report.directories, 1);
        assert_eq!(report.symlinks, 1);
        let file = destination
            .join("deep")
            .join(std::ffi::OsStr::from_bytes(b"run\xff"));
        assert_eq!(std::fs::read(&file).unwrap(), b"\0binary\xff\n");
        assert_eq!(
            std::fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o755
        );
        assert_eq!(
            std::fs::read_link(destination.join("deep/link"))
                .unwrap()
                .as_os_str()
                .as_bytes(),
            b"../\xff-target"
        );
    }

    #[tokio::test]
    async fn corrupt_late_leaf_leaves_absent_destination_and_existing_sibling_unchanged() {
        let (store, root) = fixture().await;
        let parent = tempfile::tempdir().unwrap();
        let sibling = parent.path().join("keep");
        std::fs::write(&sibling, b"untouched").unwrap();
        let destination = parent.path().join("restored");
        // Replace one leaf's bytes under the correct CID. The root and child
        // directory still verify, making this a late closure failure.
        let tree = load_verified_tree(
            &store,
            &root,
            FetchPolicy::LocalOnly,
            &TreeLimits::default(),
        )
        .await
        .unwrap();
        let leaf = tree.leaves.keys().next().unwrap().clone();
        store
            .insert_blob(leaf, Bytes::from_static(b"corrupt"))
            .await;
        let error = restore_exact_tree(
            &store,
            &root,
            &destination,
            FetchPolicy::LocalOnly,
            &TreeLimits::default(),
            &HostProfile::linux_native(),
        )
        .await
        .unwrap_err();
        assert!(matches!(error, ExactRestoreError::Tree(_)));
        assert!(!destination.exists());
        assert_eq!(std::fs::read(&sibling).unwrap(), b"untouched");
    }

    #[tokio::test]
    async fn refuses_existing_destination_even_when_empty() {
        let (store, root) = fixture().await;
        let parent = tempfile::tempdir().unwrap();
        let destination = parent.path().join("restored");
        std::fs::create_dir(&destination).unwrap();
        let error = restore_exact_tree(
            &store,
            &root,
            &destination,
            FetchPolicy::LocalOnly,
            &TreeLimits::default(),
            &HostProfile::linux_native(),
        )
        .await
        .unwrap_err();
        assert!(matches!(error, ExactRestoreError::Refused(_)));
        assert_eq!(std::fs::read_dir(&destination).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn interruption_before_promotion_never_exposes_destination() {
        let (store, root) = fixture().await;
        let tree = load_verified_tree(
            &store,
            &root,
            FetchPolicy::LocalOnly,
            &TreeLimits::default(),
        )
        .await
        .unwrap();
        let parent = tempfile::tempdir().unwrap();
        let destination = parent.path().join("restored");
        let error = stage_and_promote(&tree, &destination, parent.path(), || {
            assert!(!destination.exists());
            Err(ExactRestoreError::Refused("injected interruption".into()))
        })
        .unwrap_err();
        assert!(error.to_string().contains("injected interruption"));
        assert!(!destination.exists());
        assert_eq!(std::fs::read_dir(parent.path()).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn refuses_symlinked_parent_and_unsafe_tree_name_before_any_write() {
        let (store, root) = fixture().await;
        let parent = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(parent.path(), parent.path().join("alias")).unwrap();
        let destination = parent.path().join("alias/restored");
        let error = restore_exact_tree(
            &store,
            &root,
            &destination,
            FetchPolicy::LocalOnly,
            &TreeLimits::default(),
            &HostProfile::linux_native(),
        )
        .await
        .unwrap_err();
        assert!(matches!(error, ExactRestoreError::Refused(_)));
        assert!(!parent.path().join("restored").exists());

        let mut tree = load_verified_tree(
            &store,
            &root,
            FetchPolicy::LocalOnly,
            &TreeLimits::default(),
        )
        .await
        .unwrap();
        tree.nodes.get_mut(&root).unwrap().entries[0].name = b"../escape".to_vec();
        assert!(preflight_tree(&tree, &TreeLimits::default()).is_err());
    }

    #[tokio::test]
    async fn concurrent_destination_creation_is_never_replaced() {
        let (store, root) = fixture().await;
        let tree = load_verified_tree(
            &store,
            &root,
            FetchPolicy::LocalOnly,
            &TreeLimits::default(),
        )
        .await
        .unwrap();
        let parent = tempfile::tempdir().unwrap();
        let destination = parent.path().join("restored");
        let error = stage_and_promote(&tree, &destination, parent.path(), || {
            std::fs::create_dir(&destination).unwrap();
            std::fs::write(destination.join("owner"), b"other writer").unwrap();
            Ok(())
        })
        .unwrap_err();
        assert!(matches!(error, ExactRestoreError::Io { .. }));
        assert_eq!(
            std::fs::read(destination.join("owner")).unwrap(),
            b"other writer"
        );
        assert_eq!(std::fs::read_dir(parent.path()).unwrap().count(), 1);
    }

    #[tokio::test]
    async fn expanded_repeated_leaf_bytes_are_bounded_before_staging() {
        let (store, root) = fixture().await;
        let tree = load_verified_tree(
            &store,
            &root,
            FetchPolicy::LocalOnly,
            &TreeLimits::default(),
        )
        .await
        .unwrap();
        let limits = TreeLimits {
            max_total_bytes: 4,
            ..TreeLimits::default()
        };
        let error = preflight_tree(&tree, &limits).unwrap_err();
        assert!(error.to_string().contains("expanded restore byte budget"));
    }

    #[tokio::test]
    async fn external_boundary_and_nul_symlink_target_refuse_exact_restore() {
        let (store, root) = fixture().await;
        let mut tree = load_verified_tree(
            &store,
            &root,
            FetchPolicy::LocalOnly,
            &TreeLimits::default(),
        )
        .await
        .unwrap();
        tree.external_paths.push(vec![b"submodule".to_vec()]);
        assert!(preflight_tree(&tree, &TreeLimits::default()).is_err());
        tree.external_paths.clear();
        let target_cid = tree
            .leaves
            .iter()
            .find(|(_, bytes)| bytes.as_ref() == b"../\xff-target")
            .map(|(cid, _)| cid.clone())
            .unwrap();
        tree.leaves
            .insert(target_cid, Bytes::from_static(b"bad\0target"));
        let error = preflight_tree(&tree, &TreeLimits::default()).unwrap_err();
        assert!(error.to_string().contains("symlink target"));
    }
}
