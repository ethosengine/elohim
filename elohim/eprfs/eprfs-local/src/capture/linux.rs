use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fs::File,
    io::Read,
    os::{fd::OwnedFd, unix::ffi::OsStrExt},
    path::{Component, Path, PathBuf},
};

use bytes::Bytes;
use eprfs_core::{
    encode_tree, load_verified_tree, BlobCid, EprfsStorage, FetchPolicy, TreeEntry, TreeEntryKind,
    TreeLimits, TreeNode,
};
use eprfs_storage::{DirectoryStorage, MemoryStorage};
use rustix::fs::{fstat, openat, readlinkat_raw, FileType, Mode, OFlags, Stat, CWD};

use super::{CaptureError, CaptureReport, MAX_CAPTURE_OBJECT_BYTES};

type Result<T> = std::result::Result<T, CaptureError>;
type Names = Vec<Vec<u8>>;

// Private deterministic race injection points; production supplies a no-op.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Phase {
    BeforeOpen,
    Opened,
    Reading,
    Read,
    Recheck,
}

fn io(path: &Path, error: impl Into<std::io::Error>) -> CaptureError {
    CaptureError::Io {
        path: path.into(),
        source: error.into(),
    }
}

fn identity(a: &Stat, b: &Stat) -> bool {
    a.st_dev == b.st_dev && a.st_ino == b.st_ino
}

// atime is deliberately excluded: our own read can move it. ctime catches a
// same-size rewrite whose mtime was restored, and mode is part of the value.
fn unchanged(a: &Stat, b: &Stat) -> bool {
    identity(a, b)
        && a.st_mode == b.st_mode
        && a.st_size == b.st_size
        && a.st_nlink == b.st_nlink
        && a.st_mtime == b.st_mtime
        && a.st_mtime_nsec == b.st_mtime_nsec
        && a.st_ctime == b.st_ctime
        && a.st_ctime_nsec == b.st_ctime_nsec
}

fn stable(path: &Path, before: &Stat, after: &Stat) -> Result<()> {
    if unchanged(before, after) {
        Ok(())
    } else {
        Err(CaptureError::Changed(path.into()))
    }
}

fn absolute(path: &Path) -> Result<PathBuf> {
    let path = if path.is_absolute() {
        path.into()
    } else {
        std::env::current_dir().map_err(|e| io(path, e))?.join(path)
    };
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(CaptureError::Refused(
            "root paths must not contain parent traversal".into(),
        ));
    }
    Ok(path.components().collect())
}

/// Pin every directory component without following symlinks. Only the last fd
/// stays open, so a selection never consumes one descriptor per selected file.
struct Directory {
    fd: OwnedFd,
    chain: Vec<Stat>,
}

fn directory(path: &Path) -> Result<Directory> {
    let flags = OFlags::PATH | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let mut fd = openat(CWD, "/", flags, Mode::empty()).map_err(|e| io(path, e))?;
    let mut chain = vec![fstat(&fd).map_err(|e| io(path, e))?];
    for component in path.components() {
        if let Component::Normal(name) = component {
            fd = openat(&fd, name, flags, Mode::empty()).map_err(|e| io(path, e))?;
            chain.push(fstat(&fd).map_err(|e| io(path, e))?);
        }
    }
    Ok(Directory { fd, chain })
}

struct Roots {
    source: PathBuf,
    source_dir: Directory,
    store: PathBuf,
    store_parent: Directory,
    store_dir: Option<Directory>,
}

impl Roots {
    fn open(source: &Path, store: &Path) -> Result<Self> {
        let source = absolute(source)?;
        let store = absolute(store)?;
        if source.starts_with(&store) || store.starts_with(&source) {
            return Err(CaptureError::Refused("source and store overlap".into()));
        }
        let source_dir = directory(&source)?;
        let parent = store
            .parent()
            .ok_or_else(|| CaptureError::Refused("store has no parent".into()))?;
        let store_parent = directory(parent)?;
        let store_dir = match directory(&store) {
            Ok(dir) => Some(dir),
            Err(CaptureError::Io { source, .. })
                if source.kind() == std::io::ErrorKind::NotFound =>
            {
                None
            }
            Err(error) => return Err(error),
        };
        let source_id = source_dir.chain.last().unwrap();
        if store_parent.chain.iter().any(|s| identity(source_id, s))
            || store_dir.as_ref().is_some_and(|dir| {
                let store_id = dir.chain.last().unwrap();
                source_dir.chain.iter().any(|s| identity(store_id, s))
            })
        {
            return Err(CaptureError::Refused(
                "source and store directory identities overlap".into(),
            ));
        }
        Ok(Self {
            source,
            source_dir,
            store,
            store_parent,
            store_dir,
        })
    }

    fn recheck(&self) -> Result<()> {
        // Ancestors outside the source are compared by identity, not timestamps:
        // other captures legitimately create sibling stores while this one reads.
        for (path, before) in [
            (self.source.as_path(), &self.source_dir),
            (self.store.parent().unwrap(), &self.store_parent),
        ] {
            let after = directory(path)?;
            if before.chain.len() != after.chain.len()
                || !before
                    .chain
                    .iter()
                    .zip(&after.chain)
                    .all(|(a, b)| identity(a, b))
            {
                return Err(CaptureError::Changed(path.into()));
            }
        }
        match (&self.store_dir, directory(&self.store)) {
            (Some(before), Ok(after))
                if identity(before.chain.last().unwrap(), after.chain.last().unwrap()) => {}
            (None, Err(CaptureError::Io { source, .. }))
                if source.kind() == std::io::ErrorKind::NotFound => {}
            // A concurrent creator may install a real private store. No content
            // writes have begun; revalidate overlap and DirectoryStorage policy.
            (None, Ok(_)) => {
                Self::open(&self.source, &self.store)?;
            }
            _ => return Err(CaptureError::Changed(self.store.clone())),
        }
        Ok(())
    }
}

fn selections(selected: &[PathBuf], limits: &TreeLimits) -> Result<BTreeSet<Names>> {
    if selected.is_empty() {
        return Err(CaptureError::Refused("explicit selection is empty".into()));
    }
    if selected.len() > limits.max_total_entries {
        return Err(CaptureError::Budget("selection entries"));
    }
    let mut names = BTreeSet::new();
    let mut retained_paths = 0usize;
    for path in selected {
        let bytes = path.as_os_str().as_bytes();
        if bytes.len() > limits.max_path_bytes {
            return Err(CaptureError::Budget("path bytes"));
        }
        retained_paths = retained_paths
            .checked_add(bytes.len())
            .ok_or(CaptureError::Budget("selection bytes"))?;
        if retained_paths > limits.max_total_bytes {
            return Err(CaptureError::Budget("selection bytes"));
        }
        let mut components = Vec::new();
        // Path::components normalizes a/./b, a//b and trailing /. Check original bytes.
        for name in bytes.split(|b| *b == b'/') {
            if name.is_empty()
                || name == b"."
                || name == b".."
                || name.contains(&0)
                || name.iter().any(|b| b"*?[]{}".contains(b))
                || [b".git".as_slice(), b".eprfs", b".brit"].contains(&name)
            {
                return Err(CaptureError::Refused(format!(
                    "unsafe literal selection: {}",
                    path.display()
                )));
            }
            if name.len() > 255 {
                return Err(CaptureError::Refused("unrepresentable name".into()));
            }
            components.push(name.to_vec());
        }
        if components.len() - 1 > limits.max_depth {
            return Err(CaptureError::Budget("tree depth"));
        }
        if !names.insert(components) {
            return Err(CaptureError::Refused("duplicate selection".into()));
        }
    }
    let mut previous: Option<&Names> = None;
    for name in &names {
        if previous.is_some_and(|p| name.starts_with(p)) {
            return Err(CaptureError::Refused("ancestor selection alias".into()));
        }
        previous = Some(name);
    }
    Ok(names)
}

fn path(names: &[Vec<u8>]) -> PathBuf {
    names.iter().map(|name| OsStr::from_bytes(name)).collect()
}

struct Source {
    roots: Roots,
    directories: BTreeMap<Names, Stat>,
    leaves: BTreeMap<Names, Stat>,
    retained_path_bytes: usize,
    limits: TreeLimits,
}

impl Source {
    fn charge_path(&mut self, names: &[Vec<u8>]) -> Result<()> {
        let len: usize = names.iter().map(Vec::len).sum();
        self.retained_path_bytes = self
            .retained_path_bytes
            .checked_add(len)
            .ok_or(CaptureError::Budget("retained path bytes"))?;
        if self.retained_path_bytes > self.limits.max_total_bytes {
            return Err(CaptureError::Budget("retained path bytes"));
        }
        // The root is not an entry; before inserting this path there are
        // directories + leaves - 1 entries, so this also bounds the next one.
        if self.directories.len() + self.leaves.len() > self.limits.max_total_entries {
            return Err(CaptureError::Budget("total entries"));
        }
        Ok(())
    }

    fn leaf(&mut self, names: &Names) -> Result<(OwnedFd, OwnedFd, Stat)> {
        let mut parent =
            rustix::io::dup(&self.roots.source_dir.fd).map_err(|e| io(&self.roots.source, e))?;
        for depth in 0..names.len() {
            let prefix = names[..depth].to_vec();
            let current = fstat(&parent).map_err(|e| io(&path(&prefix), e))?;
            if let Some(before) = self.directories.get(&prefix) {
                stable(&path(&prefix), before, &current)?;
            } else {
                self.charge_path(&prefix)?;
                self.directories.insert(prefix, current);
            }
            if depth + 1 < names.len() {
                parent = openat(
                    &parent,
                    OsStr::from_bytes(&names[depth]),
                    OFlags::PATH | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map_err(|e| io(&path(&names[..=depth]), e))?;
            }
        }
        let relative = path(names);
        let fd = openat(
            &parent,
            OsStr::from_bytes(names.last().unwrap()),
            OFlags::PATH | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|e| io(&relative, e))?;
        let stat = fstat(&fd).map_err(|e| io(&relative, e))?;
        match FileType::from_raw_mode(stat.st_mode) {
            FileType::RegularFile | FileType::Symlink => {}
            _ => {
                return Err(CaptureError::Refused(format!(
                    "selection is not a file or literal symlink: {}",
                    relative.display()
                )))
            }
        }
        if let Some(before) = self.leaves.get(names) {
            stable(&relative, before, &stat)?;
        }
        Ok((parent, fd, stat))
    }

    fn read(
        &mut self,
        names: &Names,
        limit: usize,
        hook: &mut impl FnMut(Phase, &Path),
    ) -> Result<Bytes> {
        let relative = path(names);
        let (parent, pinned, before) = self.leaf(names)?;
        hook(Phase::BeforeOpen, &relative);
        let bytes = if FileType::from_raw_mode(before.st_mode) == FileType::Symlink {
            let mut target = [0u8; 4096];
            hook(Phase::Opened, &relative);
            let len = readlinkat_raw(&pinned, "", &mut target).map_err(|e| io(&relative, e))?;
            if len == target.len() {
                return Err(CaptureError::Budget("symlink target bytes"));
            }
            if len > limit {
                return Err(CaptureError::Budget("leaf bytes"));
            }
            Bytes::copy_from_slice(&target[..len])
        } else {
            let fd = openat(
                &parent,
                OsStr::from_bytes(names.last().unwrap()),
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|e| io(&relative, e))?;
            stable(
                &relative,
                &before,
                &fstat(&fd).map_err(|e| io(&relative, e))?,
            )?;
            hook(Phase::Opened, &relative);
            let file = File::from(fd);
            let expected = (before.st_size as usize).min(limit);
            let mut bytes = Vec::with_capacity(expected);
            let mut reader = (&file).take(expected as u64 + 1);
            let mut chunk = [0u8; 16 * 1024];
            loop {
                let len = reader.read(&mut chunk).map_err(|e| io(&relative, e))?;
                if len == 0 {
                    break;
                }
                bytes.extend_from_slice(&chunk[..len]);
                hook(Phase::Reading, &relative);
            }
            if bytes.len() > limit {
                return Err(CaptureError::Budget("leaf bytes"));
            }
            stable(
                &relative,
                &before,
                &fstat(&file).map_err(|e| io(&relative, e))?,
            )?;
            Bytes::from(bytes)
        };
        hook(Phase::Read, &relative);
        stable(
            &relative,
            &before,
            &fstat(&pinned).map_err(|e| io(&relative, e))?,
        )?;
        let (_, _, after) = self.leaf(names)?;
        stable(&relative, &before, &after)?;
        if bytes.len() as u64 != before.st_size as u64 {
            return Err(CaptureError::Changed(relative));
        }
        Ok(bytes)
    }
}

fn bounded(limits: &TreeLimits) -> TreeLimits {
    let defaults = TreeLimits::default();
    TreeLimits {
        max_node_bytes: limits.max_node_bytes.min(defaults.max_node_bytes),
        max_entries_per_node: limits
            .max_entries_per_node
            .min(defaults.max_entries_per_node),
        max_depth: limits.max_depth.min(defaults.max_depth),
        max_total_entries: limits.max_total_entries.min(defaults.max_total_entries),
        max_total_bytes: limits.max_total_bytes.min(defaults.max_total_bytes),
        max_path_bytes: limits.max_path_bytes.min(defaults.max_path_bytes),
        max_pending_entries: limits.max_pending_entries.min(defaults.max_pending_entries),
    }
}

/// The prepared closure is bounded and held before storage mutation. Bytes are
/// shared (Bytes) while the existing loader verifies canonical nodes and leaves.
pub(super) struct Prepared {
    pub report: CaptureReport,
    objects: Vec<(BlobCid, Bytes)>,
    limits: TreeLimits,
}

impl Prepared {
    pub(super) async fn write<S: EprfsStorage>(&self, storage: &S) -> Result<()> {
        for (cid, bytes) in &self.objects {
            storage.put_blob_verified(cid, bytes.clone()).await?;
        }
        load_verified_tree(
            storage,
            &self.report.root,
            FetchPolicy::LocalOnly,
            &self.limits,
        )
        .await?;
        Ok(())
    }
}

pub(super) fn prepare(
    source: &Path,
    store: &Path,
    selected: &[PathBuf],
    limits: &TreeLimits,
    mut hook: impl FnMut(Phase, &Path),
) -> Result<(Prepared, impl FnOnce() -> Result<()>)> {
    let limits = bounded(limits);
    let selected = selections(selected, &limits)?;
    let roots = Roots::open(source, store)?;
    let mut source = Source {
        roots,
        directories: BTreeMap::new(),
        leaves: BTreeMap::new(),
        retained_path_bytes: 0,
        limits,
    };
    let mut declared_bytes = 0usize;
    for names in &selected {
        let (_, _, stat) = source.leaf(names)?;
        let size = usize::try_from(stat.st_size).map_err(|_| CaptureError::Budget("leaf bytes"))?;
        if size > MAX_CAPTURE_OBJECT_BYTES {
            return Err(CaptureError::Budget("object bytes"));
        }
        declared_bytes = declared_bytes
            .checked_add(size)
            .ok_or(CaptureError::Budget("closure bytes"))?;
        if declared_bytes > limits.max_total_bytes {
            return Err(CaptureError::Budget("closure bytes"));
        }
        source.charge_path(names)?;
        source.leaves.insert(names.clone(), stat);
        if source.leaves.len() + source.directories.len() - 1 > limits.max_total_entries {
            return Err(CaptureError::Budget("total entries"));
        }
    }
    let mut nodes: BTreeMap<Names, Vec<TreeEntry>> = source
        .directories
        .keys()
        .map(|p| (p.clone(), vec![]))
        .collect();
    let mut objects = Vec::new();
    let mut files = 0;
    let mut symlinks = 0;
    let mut expanded_bytes = 0;
    for names in &selected {
        let bytes = source.read(
            names,
            MAX_CAPTURE_OBJECT_BYTES.min(limits.max_total_bytes - expanded_bytes),
            &mut hook,
        )?;
        expanded_bytes += bytes.len();
        let cid = BlobCid::compute_raw(&bytes);
        let stat = &source.leaves[names];
        let kind = if FileType::from_raw_mode(stat.st_mode) == FileType::Symlink {
            symlinks += 1;
            TreeEntryKind::Symlink {
                target: cid.clone().into(),
            }
        } else {
            files += 1;
            TreeEntryKind::File {
                blob: cid.clone().into(),
                executable: stat.st_mode & 0o111 != 0,
            }
        };
        objects.push((cid, bytes));
        nodes
            .get_mut(&names[..names.len() - 1])
            .unwrap()
            .push(TreeEntry {
                name: names.last().unwrap().clone(),
                kind,
            });
    }
    hook(Phase::Recheck, Path::new(""));
    // Re-open and compare every selected leaf once. Metadata timestamps can
    // share a filesystem tick: equality alone cannot catch an already-read byte
    // changing during the read of a later file. This is a bounded second pass,
    // not a retry loop or a claim that the filesystem was atomically frozen.
    for (names, (_, captured)) in selected.iter().zip(&objects) {
        let checked = source.read(names, MAX_CAPTURE_OBJECT_BYTES, &mut |_, _| {})?;
        if checked != *captured {
            return Err(CaptureError::Changed(path(names)));
        }
    }
    source.roots.recheck()?;
    let directories = nodes.len() - 1;
    let root = loop {
        // Lexicographic reverse order visits descendants before their parents.
        let (names, entries) = nodes.pop_last().unwrap();
        let (cid, bytes) = encode_tree(&TreeNode::new(entries)?, &limits)?;
        expanded_bytes = expanded_bytes
            .checked_add(bytes.len())
            .ok_or(CaptureError::Budget("closure bytes"))?;
        if expanded_bytes > limits.max_total_bytes {
            return Err(CaptureError::Budget("closure bytes"));
        }
        objects.push((cid.clone(), Bytes::from(bytes)));
        if names.is_empty() {
            break cid;
        }
        nodes
            .get_mut(&names[..names.len() - 1])
            .unwrap()
            .push(TreeEntry {
                name: names.last().unwrap().clone(),
                kind: TreeEntryKind::Directory { tree: cid.into() },
            });
    };
    let prepared = Prepared {
        report: CaptureReport {
            root,
            selected_paths: selected.iter().map(|n| path(n)).collect(),
            files,
            directories,
            symlinks,
            bytes_captured: declared_bytes,
            published: false,
        },
        objects,
        limits,
    };
    Ok((prepared, move || source.roots.recheck()))
}

pub(super) async fn capture(
    source: &Path,
    store: &Path,
    selected: &[PathBuf],
    limits: &TreeLimits,
    hook: impl FnMut(Phase, &Path),
) -> Result<CaptureReport> {
    // Resolve both relative roots against one cwd observation. Later awaits or
    // another thread changing cwd must not redirect writes away from the store
    // whose ancestry and overlap we checked.
    let cwd = if source.is_absolute() && store.is_absolute() {
        PathBuf::new()
    } else {
        std::env::current_dir().map_err(|e| io(Path::new("."), e))?
    };
    let source = cwd.join(source);
    let store = cwd.join(store);
    let (prepared, recheck) = prepare(&source, &store, selected, limits, hook)?;
    // This exercises the same closure verifier before any filesystem mutation,
    // including pending-frontier bounds that depend on canonical directory shape.
    prepared.write(&MemoryStorage::default()).await?;
    recheck()?;
    // create handles competing creators and synchronizes the root's parent;
    // falling back to a read-only open would lose that durability obligation.
    let storage = DirectoryStorage::create(&store)?;
    prepared.write(&storage).await?;
    Ok(prepared.report)
}
