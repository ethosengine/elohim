//! Invocation-local filesystem checks. EPRFS remains the canonical tree/capture owner.
//! The private run parent is trusted; this detects observed changes, not hostile mutate/restore.

use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::{
        fd::AsRawFd,
        unix::{
            ffi::{OsStrExt, OsStringExt},
            fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
        },
    },
    path::{Component, Path, PathBuf},
};

use ark_core::manifest::ArtifactRef;
use eprfs_core::{TreeEntryKind, TreeLimits, VerifiedTree};
use eprfs_local::MAX_CAPTURE_OBJECT_BYTES;
use nix::libc;
use sha2::{Digest, Sha256};

use crate::Failure;

pub(crate) fn failure(error: impl std::fmt::Display) -> Failure {
    Failure::data(format!("native task: {error}"))
}

fn open_at(parent: &File, name: &OsStr, directory: bool) -> Result<File, Failure> {
    use std::os::fd::FromRawFd;
    let name = std::ffi::CString::new(name.as_bytes()).map_err(failure)?;
    let flags = libc::O_RDONLY
        | libc::O_CLOEXEC
        | libc::O_NOFOLLOW
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    // SAFETY: parent is live, name is NUL terminated, and a successful fd is owned once.
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(failure(std::io::Error::last_os_error()));
    }
    // SAFETY: openat returned a fresh owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}

/// Resolve a directory by opening every ancestor without following links.
pub(crate) fn directory(path: &Path) -> Result<(PathBuf, File), Failure> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir().map_err(failure)?.join(path)
    };
    let mut dir = File::open("/").map_err(failure)?;
    let mut clean = PathBuf::from("/");
    for component in absolute.components() {
        match component {
            Component::RootDir | Component::CurDir => {}
            Component::Normal(name) => {
                dir = open_at(&dir, name, true)?;
                clean.push(name);
            }
            _ => return Err(failure("parent traversal is not supported")),
        }
    }
    Ok((clean, dir))
}

pub(crate) fn new_run(path: &Path, store: &Path) -> Result<PathBuf, Failure> {
    let name = path
        .file_name()
        .ok_or_else(|| failure("run directory needs a final name"))?;
    let (parent, _) = directory(
        path.parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )?;
    let run = parent.join(name);
    if run.to_str().is_none_or(|p| p.contains(['{', '}', '\0'])) {
        return Err(failure(
            "absolute run directory must be UTF-8 without template braces or NUL",
        ));
    }
    let (store, store_fd) = directory(store)?;
    if run.starts_with(&store) || store.starts_with(&run) {
        return Err(failure("run directory and object store overlap"));
    }
    let store_identity = store_fd.metadata().map_err(failure)?;
    for ancestor in parent.ancestors() {
        let metadata = fs::metadata(ancestor).map_err(failure)?;
        if (metadata.dev(), metadata.ino()) == (store_identity.dev(), store_identity.ino()) {
            return Err(failure("run directory aliases the object store"));
        }
    }
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&run)
        .map_err(failure)?;
    fs::set_permissions(&run, fs::Permissions::from_mode(0o700)).map_err(failure)?;
    Ok(run)
}

pub(crate) fn private_dir(path: &Path) -> Result<(), Failure> {
    fs::DirBuilder::new()
        .mode(0o700)
        .create(path)
        .map_err(failure)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(failure)
}

/// Retain exactly the pinned bytes before the driver reopens a pathname. A replacement of
/// the caller's artifact can no longer change the program. The private parent is trusted.
pub(crate) fn retain_artifact(
    source: &Path,
    run: &Path,
    artifact: &ArtifactRef,
) -> Result<PathBuf, Failure> {
    let ArtifactRef::Pinned { cid, sha256, .. } = artifact else {
        return Err(failure("artifact must be pinned"));
    };
    let mut source = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(source)
        .map_err(failure)?;
    let metadata = source.metadata().map_err(failure)?;
    if !metadata.is_file()
        || metadata.mode() & 0o111 == 0
        || metadata.len() > MAX_CAPTURE_OBJECT_BYTES as u64
    {
        return Err(failure(
            "artifact must be an executable regular file at most 64 MiB",
        ));
    }
    let mut bytes = Vec::new();
    (&mut source)
        .take(MAX_CAPTURE_OBJECT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(failure)?;
    if bytes.len() > MAX_CAPTURE_OBJECT_BYTES {
        return Err(failure("artifact object limit exceeded"));
    }
    if hex::encode(Sha256::digest(&bytes)) != *sha256 {
        return Err(Failure {
            code: crate::EXIT_ARTIFACT,
            message: "native task: artifact digest mismatch".into(),
        });
    }
    // The byte CID is the native identity; the sha256 above is evidence, checked too.
    if let Some(cid) = cid {
        let claimed = elohim_epr::cid::ByteCid::parse(cid).map_err(failure)?;
        if elohim_epr::cid::compute_bytes_cid(&bytes) != claimed {
            return Err(Failure {
                code: crate::EXIT_ARTIFACT,
                message: "native task: artifact byte CID mismatch".into(),
            });
        }
    }
    let target = run.join("artifact");
    let mut copy = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o500)
        .open(&target)
        .map_err(failure)?;
    copy.set_permissions(fs::Permissions::from_mode(0o500))
        .map_err(failure)?;
    copy.write_all(&bytes)
        .and_then(|()| copy.sync_all())
        .map_err(failure)?;
    File::open(run)
        .and_then(|dir| dir.sync_all())
        .map_err(failure)?;
    Ok(target)
}

/// Unique stored leaves are not the materialized cost: repeated paths are charged each time.
pub(crate) fn expanded_bytes(tree: &VerifiedTree, limits: &TreeLimits) -> Result<u64, Failure> {
    let mut pending = vec![(&tree.root, 0usize)];
    let mut bytes = 0u64;
    let mut entries = 0usize;
    while let Some((cid, depth)) = pending.pop() {
        if depth > limits.max_depth {
            return Err(failure("input depth limit"));
        }
        let node = tree
            .nodes
            .get(cid)
            .ok_or_else(|| failure("incomplete input node"))?;
        for entry in &node.entries {
            entries += 1;
            if entries > limits.max_total_entries {
                return Err(failure("expanded input inventory limit"));
            }
            match &entry.kind {
                TreeEntryKind::Directory { tree } => pending.push((tree.as_blob_cid(), depth + 1)),
                TreeEntryKind::File { blob, .. } | TreeEntryKind::Symlink { target: blob } => {
                    let size = tree
                        .leaf(blob.as_blob_cid())
                        .ok_or_else(|| failure("incomplete input leaf"))?
                        .len();
                    if size > MAX_CAPTURE_OBJECT_BYTES {
                        return Err(failure("input object exceeds 64 MiB"));
                    }
                    bytes = bytes
                        .checked_add(size as u64)
                        .ok_or_else(|| failure("expanded input size overflow"))?;
                    if bytes > limits.max_total_bytes.min(1024 * 1024 * 1024) as u64 {
                        return Err(failure("expanded input exceeds 1 GiB"));
                    }
                }
                TreeEntryKind::External { .. } => return Err(failure("external input boundary")),
            }
        }
    }
    Ok(bytes)
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Entry {
    Directory(u32),
    File {
        mode: u32,
        size: u64,
        digest: Vec<u8>,
    },
    Symlink(PathBuf),
}

pub(crate) type Inventory = BTreeMap<PathBuf, Entry>;

/// The baseline must match the verified candidate, not merely whatever happens to be on
/// disk after restore. Permission bits not carried by Tree v1 are normalized only here;
/// the before/after inventory comparison still detects every later mode change.
pub(crate) fn expected_input(
    tree: &VerifiedTree,
    limits: &TreeLimits,
) -> Result<Inventory, Failure> {
    let mut expected = BTreeMap::from([(PathBuf::new(), Entry::Directory(0))]);
    let mut pending = vec![(PathBuf::new(), &tree.root)];
    while let Some((prefix, cid)) = pending.pop() {
        for entry in &tree.nodes[cid].entries {
            let path = prefix.join(std::ffi::OsString::from_vec(entry.name.clone()));
            if expected.len() > limits.max_total_entries
                || path.as_os_str().len() > limits.max_path_bytes
            {
                return Err(failure("expected input inventory limit"));
            }
            let value = match &entry.kind {
                TreeEntryKind::Directory { tree } => {
                    pending.push((path.clone(), tree.as_blob_cid()));
                    Entry::Directory(0)
                }
                TreeEntryKind::File { blob, executable } => {
                    let bytes = tree
                        .leaf(blob.as_blob_cid())
                        .ok_or_else(|| failure("missing expected input leaf"))?;
                    Entry::File {
                        mode: u32::from(*executable),
                        size: bytes.len() as u64,
                        digest: Sha256::digest(bytes).to_vec(),
                    }
                }
                TreeEntryKind::Symlink { target } => {
                    let bytes = tree
                        .leaf(target.as_blob_cid())
                        .ok_or_else(|| failure("missing expected symlink"))?;
                    Entry::Symlink(std::ffi::OsString::from_vec(bytes.to_vec()).into())
                }
                TreeEntryKind::External { .. } => return Err(failure("external input boundary")),
            };
            expected.insert(path, value);
        }
    }
    Ok(expected)
}

pub(crate) fn matches_candidate(actual: &Inventory, expected: &Inventory) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .all(|(path, actual)| match (actual, expected.get(path)) {
                (Entry::Directory(_), Some(Entry::Directory(_))) => true,
                (
                    Entry::File { mode, size, digest },
                    Some(Entry::File {
                        mode: executable,
                        size: expected_size,
                        digest: expected_digest,
                    }),
                ) => {
                    u32::from(mode & 0o111 != 0) == *executable
                        && size == expected_size
                        && digest == expected_digest
                }
                (Entry::Symlink(target), Some(Entry::Symlink(expected))) => target == expected,
                _ => false,
            })
}

pub(crate) fn inventory(root: &Path, limits: &TreeLimits) -> Result<Inventory, Failure> {
    let (_, dir) = directory(root)?;
    let mut inventory = BTreeMap::from([(
        PathBuf::new(),
        Entry::Directory(dir.metadata().map_err(failure)?.mode() & 0o7777),
    )]);
    let mut bytes = 0u64;
    walk(&dir, Path::new(""), 0, limits, &mut inventory, &mut bytes)?;
    Ok(inventory)
}

fn walk(
    dir: &File,
    prefix: &Path,
    depth: usize,
    limits: &TreeLimits,
    out: &mut Inventory,
    bytes: &mut u64,
) -> Result<(), Failure> {
    if depth > limits.max_depth {
        return Err(failure("inventory depth limit"));
    }
    let fd_path = PathBuf::from(format!("/proc/self/fd/{}", dir.as_raw_fd()));
    let mut count = 0usize;
    for entry in fs::read_dir(&fd_path).map_err(failure)? {
        let entry = entry.map_err(failure)?;
        let name = entry.file_name();
        let path = prefix.join(&name);
        count += 1;
        if count > limits.max_entries_per_node
            || out.len() >= limits.max_total_entries
            || path.as_os_str().len() > limits.max_path_bytes
        {
            return Err(failure("inventory size limit"));
        }
        let metadata = fs::symlink_metadata(fd_path.join(&name)).map_err(failure)?;
        if metadata.is_dir() {
            let child = open_at(dir, &name, true)?;
            if child.metadata().map_err(failure)?.ino() != metadata.ino() {
                return Err(failure("directory changed during inventory"));
            }
            out.insert(path.clone(), Entry::Directory(metadata.mode() & 0o7777));
            walk(&child, &path, depth + 1, limits, out, bytes)?;
        } else if metadata.file_type().is_symlink() {
            let target = fs::read_link(fd_path.join(&name)).map_err(failure)?;
            charge(target.as_os_str().len() as u64, bytes, limits)?;
            out.insert(path, Entry::Symlink(target));
        } else if metadata.is_file() {
            let mut file = open_at(dir, &name, false)?;
            let before = file.metadata().map_err(failure)?;
            if !before.is_file() || before.ino() != metadata.ino() || before.dev() != metadata.dev()
            {
                return Err(failure("file changed during inventory"));
            }
            charge(before.len(), bytes, limits)?;
            let mut digest = Sha256::new();
            let mut read_bytes = 0u64;
            let mut buffer = [0u8; 16384];
            loop {
                let read = file.read(&mut buffer).map_err(failure)?;
                if read == 0 {
                    break;
                }
                read_bytes += read as u64;
                if read_bytes > before.len() {
                    return Err(failure("file grew during inventory"));
                }
                digest.update(&buffer[..read]);
            }
            let after = file.metadata().map_err(failure)?;
            if read_bytes != before.len()
                || before.mtime_nsec() != after.mtime_nsec()
                || before.mtime() != after.mtime()
                || before.ctime_nsec() != after.ctime_nsec()
                || before.mode() != after.mode()
            {
                return Err(failure("file changed during inventory"));
            }
            out.insert(
                path,
                Entry::File {
                    mode: before.mode() & 0o7777,
                    size: read_bytes,
                    digest: digest.finalize().to_vec(),
                },
            );
        } else {
            return Err(failure("special file in task inventory"));
        }
    }
    Ok(())
}

fn charge(size: u64, bytes: &mut u64, limits: &TreeLimits) -> Result<(), Failure> {
    if size > MAX_CAPTURE_OBJECT_BYTES as u64 {
        return Err(failure("inventory object exceeds 64 MiB"));
    }
    *bytes = bytes
        .checked_add(size)
        .ok_or_else(|| failure("inventory byte overflow"))?;
    if *bytes > limits.max_total_bytes.min(1024 * 1024 * 1024) as u64 {
        return Err(failure("inventory exceeds 1 GiB"));
    }
    Ok(())
}

pub(crate) fn selected_outputs(
    inventory: &Inventory,
    outputs: &[String],
) -> Result<Vec<PathBuf>, Failure> {
    let mut selected = std::collections::BTreeSet::new();
    for output in outputs {
        let path = Path::new(output);
        let entry = inventory
            .get(path)
            .ok_or_else(|| failure(format!("missing declared output {output}")))?;
        match entry {
            Entry::Directory(_) => {
                let mut found = false;
                for (child, entry) in inventory {
                    if child.starts_with(path) && !matches!(entry, Entry::Directory(_)) {
                        selected.insert(child.clone());
                        found = true;
                    }
                }
                if !found {
                    return Err(failure(format!(
                        "declared output directory {output} has no capturable leaves"
                    )));
                }
            }
            _ => {
                selected.insert(path.to_owned());
            }
        }
    }
    Ok(selected.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use eprfs_core::{BlobCid, TreeEntry, TreeNode};

    #[test]
    fn expanded_input_counts_repeated_paths_and_enforces_object_and_closure_caps() {
        let blob = BlobCid::compute_raw(b"abc");
        let node = TreeNode::new(vec![
            TreeEntry {
                name: b"a".to_vec(),
                kind: TreeEntryKind::File {
                    blob: blob.clone().into(),
                    executable: false,
                },
            },
            TreeEntry {
                name: b"b".to_vec(),
                kind: TreeEntryKind::File {
                    blob: blob.clone().into(),
                    executable: false,
                },
            },
        ])
        .expect("node");
        let root = eprfs_core::encode_tree(&node, &TreeLimits::default())
            .expect("root")
            .0;
        let tree = VerifiedTree {
            root: root.clone(),
            nodes: [(root, node)].into(),
            leaves: [(blob, b"abc".to_vec().into())].into(),
            external_paths: Vec::new(),
        };
        assert_eq!(
            expanded_bytes(&tree, &TreeLimits::default()).expect("expanded cost"),
            6
        );
        let limits = TreeLimits {
            max_total_bytes: 5,
            ..TreeLimits::default()
        };
        assert!(
            expanded_bytes(&tree, &limits).is_err(),
            "dedup size 3 cannot stand for expanded size 6"
        );
        assert!(charge(
            MAX_CAPTURE_OBJECT_BYTES as u64 + 1,
            &mut 0,
            &TreeLimits::default()
        )
        .is_err());
        assert!(charge(1, &mut (1024 * 1024 * 1024), &TreeLimits::default()).is_err());
    }

    #[test]
    fn retained_artifact_is_private_verified_and_independent_of_original_replacement() {
        let temp = tempfile::tempdir().expect("fixture");
        let source = temp.path().join("source");
        fs::write(&source, b"original executable bytes").expect("artifact");
        fs::set_permissions(&source, fs::Permissions::from_mode(0o700)).expect("mode");
        let artifact: ArtifactRef = serde_json::from_value(serde_json::json!({"pinned": {"sha256": hex::encode(Sha256::digest(b"original executable bytes"))}})).expect("pinned artifact");
        let run = temp.path().join("run");
        private_dir(&run).expect("run");
        let retained = retain_artifact(&source, &run, &artifact).expect("verified retained copy");
        fs::write(&source, b"replacement").expect("replace source");
        assert_eq!(
            fs::read(retained).expect("retained bytes"),
            b"original executable bytes"
        );
        let wrong = temp.path().join("wrong");
        private_dir(&wrong).expect("wrong run");
        assert!(retain_artifact(&source, &wrong, &artifact).is_err());
        assert!(!wrong.join("artifact").exists());
        let file = OpenOptions::new()
            .write(true)
            .open(&source)
            .expect("open source");
        file.set_len(MAX_CAPTURE_OBJECT_BYTES as u64 + 1)
            .expect("sparse oversize");
        assert!(retain_artifact(&source, &wrong, &artifact).is_err());
    }

    #[test]
    fn sha256_evidence_is_verified_when_present() {
        let temp = tempfile::tempdir().expect("fixture");
        let source = temp.path().join("source");
        let bytes = b"original executable bytes";
        fs::write(&source, bytes).expect("artifact");
        fs::set_permissions(&source, fs::Permissions::from_mode(0o700)).expect("mode");
        let cid = elohim_epr::cid::compute_bytes_cid(bytes).to_string();
        let pin = |cid: &str, sha: String| -> ArtifactRef {
            serde_json::from_value(serde_json::json!({"pinned": {"cid": cid, "sha256": sha}}))
                .expect("pinned artifact")
        };
        let good = hex::encode(Sha256::digest(bytes));
        let run = temp.path().join("ok");
        private_dir(&run).expect("run");
        assert!(retain_artifact(&source, &run, &pin(&cid, good.clone())).is_ok());
        // Wrong sha256 evidence is refused even with a right byte CID.
        let bad = temp.path().join("bad");
        private_dir(&bad).expect("run");
        assert!(retain_artifact(&source, &bad, &pin(&cid, "0".repeat(64))).is_err());
        // A byte CID of other bytes is refused even with right sha256 evidence.
        let other = elohim_epr::cid::compute_bytes_cid(b"other").to_string();
        let mismatch = temp.path().join("mismatch");
        private_dir(&mismatch).expect("run");
        assert!(retain_artifact(&source, &mismatch, &pin(&other, good)).is_err());
        assert!(!mismatch.join("artifact").exists());
    }

    #[test]
    fn private_layout_refuses_existing_overlap_and_symlink_ancestors() {
        let temp = tempfile::tempdir().expect("fixture");
        let store = temp.path().join("store");
        fs::create_dir(&store).expect("store");
        assert!(new_run(&store.join("run"), &store).is_err());
        let run = new_run(&temp.path().join("run"), &store).expect("new private run");
        assert!(new_run(&run, &store).is_err());
        let alias = temp.path().join("alias");
        std::os::unix::fs::symlink(&run, &alias).expect("alias");
        assert!(new_run(&alias.join("child"), &store).is_err());
        assert!(!run.join("child").exists());
    }

    #[test]
    fn the_initial_inventory_must_match_the_candidate_before_execution() {
        let temp = tempfile::tempdir().expect("fixture");
        let source = temp.path().join("source");
        let store_path = temp.path().join("store");
        fs::create_dir(&source).expect("source");
        fs::write(source.join("value"), b"A").expect("A");
        let limits = TreeLimits::default();
        let captured = futures::executor::block_on(eprfs_local::capture_tree(
            &source,
            &store_path,
            &["value".into()],
            &limits,
        ))
        .expect("capture");
        let store = eprfs_storage::DirectoryStorage::open(&store_path).expect("store");
        let tree = futures::executor::block_on(eprfs_core::load_verified_tree(
            &store,
            &captured.root,
            eprfs_core::FetchPolicy::LocalOnly,
            &limits,
        ))
        .expect("tree");
        let expected = expected_input(&tree, &limits).expect("expected inventory");
        assert!(matches_candidate(
            &inventory(&source, &limits).expect("actual A"),
            &expected
        ));
        fs::write(source.join("value"), b"B").expect("change before first observation");
        let actual = inventory(&source, &limits).expect("actual B");
        assert!(!matches_candidate(&actual, &expected));
        fs::write(source.join("value"), b"A").expect("restore A");
        fs::create_dir(source.join("extra-empty")).expect("new directory");
        assert!(!matches_candidate(
            &inventory(&source, &limits).expect("extra directory"),
            &expected
        ));
    }
}
