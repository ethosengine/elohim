//! Immutable local raw blobs and EPRFS trees. A saved CID proves bytes, never publication.
use super::{capture::relative, invalid, Captured, Projection, Result};
use eprfs_core::{encode_tree, BlobCid, TreeEntry, TreeEntryKind, TreeLimits, TreeNode};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const MAX_BYTES: usize = 256 * 1024 * 1024;
const MAX_METADATA_BYTES: usize = 64 * 1024 * 1024;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    version: u32,
    sources_tree: String,
    projection: Projection,
}

fn safe_store(store: &Path, create: bool) -> Result<PathBuf> {
    let abs = super::capture::absolute_directory(store)?;
    super::capture::open_directory(&abs, create)?;
    Ok(abs)
}

fn get(store: &Path, cid: &str, budget: &mut usize) -> Result<Vec<u8>> {
    let cid = BlobCid::parse(cid).map_err(invalid)?;
    let path = store.join(cid.to_string());
    let metadata = fs::symlink_metadata(&path).map_err(invalid)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(invalid("snapshot object must be a regular file"));
    }
    let mut bytes = Vec::new();
    super::capture::open_file(&path)?
        .take(*budget as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(invalid)?;
    if bytes.len() > *budget {
        return Err(invalid("snapshot byte budget exceeded"));
    }
    *budget -= bytes.len();
    if !cid.verifies(&bytes) {
        return Err(invalid(format!("corrupt snapshot object: {cid}")));
    }
    Ok(bytes)
}
fn put(store: &Path, cid: &BlobCid, bytes: &[u8]) -> Result<()> {
    if !cid.verifies(bytes) {
        return Err(invalid("invalid object CID"));
    }
    let destination = store.join(cid.to_string());
    if destination.try_exists().map_err(invalid)? {
        {
            let mut budget = MAX_BYTES;
            get(store, &cid.to_string(), &mut budget)?;
        }
        return Ok(());
    }
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let directory = super::capture::open_directory(store, false)?;
    let scratch = format!(
        ".capture-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    use rustix::fs::{linkat, openat, unlinkat, AtFlags, Mode, OFlags};
    let fd = openat(
        &directory,
        &scratch,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )
    .map_err(invalid)?;
    let mut file = fs::File::from(fd);
    let result = (|| {
        file.write_all(bytes).map_err(invalid)?;
        file.sync_all().map_err(invalid)?;
        match linkat(
            &directory,
            &scratch,
            &directory,
            cid.to_string(),
            AtFlags::empty(),
        ) {
            Ok(_) => {}
            Err(e) if e == rustix::io::Errno::EXIST => {
                let mut budget = MAX_BYTES;
                get(store, &cid.to_string(), &mut budget)?;
            }
            Err(e) => return Err(invalid(e)),
        }
        directory.sync_all().map_err(invalid)?;
        Ok(())
    })();
    let _ = unlinkat(&directory, &scratch, AtFlags::empty());
    result
}
#[derive(Default)]
struct Directory {
    files: BTreeMap<String, String>,
    children: BTreeMap<String, Directory>,
}
impl Directory {
    fn insert(&mut self, path: &str, cid: &str) -> Result<()> {
        relative(path)?;
        if let Some((name, rest)) = path.split_once('/') {
            if self.files.contains_key(name) {
                return Err(invalid("tree path collision"));
            }
            self.children
                .entry(name.into())
                .or_default()
                .insert(rest, cid)
        } else {
            if self.children.contains_key(path)
                || self.files.insert(path.into(), cid.into()).is_some()
            {
                return Err(invalid("tree path collision"));
            }
            Ok(())
        }
    }
    fn encode(&self, objects: &mut BTreeMap<String, Vec<u8>>, depth: usize) -> Result<BlobCid> {
        if depth > 64 {
            return Err(invalid("snapshot tree depth exceeded"));
        }
        let mut entries = Vec::new();
        for (name, cid) in &self.files {
            entries.push(TreeEntry {
                name: name.as_bytes().to_vec(),
                kind: TreeEntryKind::File {
                    blob: BlobCid::parse(cid).map_err(invalid)?.into(),
                    executable: false,
                },
            });
        }
        for (name, child) in &self.children {
            entries.push(TreeEntry {
                name: name.as_bytes().to_vec(),
                kind: TreeEntryKind::Directory {
                    tree: child.encode(objects, depth + 1)?.into(),
                },
            });
        }
        let (cid, bytes) = encode_tree(
            &TreeNode::new(entries).map_err(invalid)?,
            &TreeLimits::default(),
        )
        .map_err(invalid)?;
        objects.insert(cid.to_string(), bytes);
        Ok(cid)
    }
}
fn trees(sources: &BTreeMap<String, String>) -> Result<(String, BTreeMap<String, Vec<u8>>)> {
    if sources.len() > 100_000 {
        return Err(invalid("snapshot source bound exceeded"));
    }
    let mut directory = Directory::default();
    for (path, cid) in sources {
        directory.insert(path, cid)?;
    }
    let mut objects = BTreeMap::new();
    let root = directory.encode(&mut objects, 0)?;
    Ok((root.to_string(), objects))
}
fn validate(captured: &Captured) -> Result<()> {
    let p = &captured.projection;
    if !matches!(p.version, 1 | 2)
        || (p.version == 2) != p.composition.is_some()
        || p.sources.len() != captured.bytes.len()
        || p.sources.get(&p.declaration) != Some(&p.declaration_cid)
    {
        return Err(invalid("invalid projection manifest"));
    }
    for (path, cid) in &p.sources {
        relative(path)?;
        let bytes = captured
            .bytes
            .get(path)
            .ok_or_else(|| invalid("missing captured source"))?;
        if BlobCid::compute_raw(bytes).to_string() != *cid
            || p.input_versions.get(path) != Some(cid)
        {
            return Err(invalid("source version mismatch"));
        }
    }
    let mut ids = std::collections::BTreeSet::new();
    for subject in &p.subjects {
        if !ids.insert(&subject.identity)
            || p.sources.get(&subject.source) != Some(&subject.version)
        {
            return Err(invalid("invalid subject source or duplicate identity"));
        }
    }
    Ok(())
}
pub fn save(store: &Path, captured: &Captured) -> Result<String> {
    validate(captured)?;
    let (sources_tree, objects) = trees(&captured.projection.sources)?;
    let bytes = serde_json::to_vec(&Snapshot {
        version: 1,
        sources_tree,
        projection: captured.projection.clone(),
    })
    .map_err(invalid)?;
    if captured.bytes.values().map(Vec::len).sum::<usize>() > MAX_BYTES
        || bytes.len() > MAX_METADATA_BYTES
        || objects.values().map(Vec::len).sum::<usize>() > MAX_METADATA_BYTES
    {
        return Err(invalid("snapshot exceeds source or metadata bounds"));
    }
    let store = safe_store(store, true)?;
    for (path, bytes) in &captured.bytes {
        put(
            &store,
            &BlobCid::parse(&captured.projection.sources[path]).map_err(invalid)?,
            bytes,
        )?;
    }
    for (cid, bytes) in objects {
        put(&store, &BlobCid::parse(&cid).map_err(invalid)?, &bytes)?;
    }
    let cid = BlobCid::compute_raw(&bytes);
    put(&store, &cid, &bytes)?;
    reopen(&store, &cid.to_string())?;
    Ok(cid.to_string())
}
pub fn reopen(store: &Path, cid: &str) -> Result<Captured> {
    let store = safe_store(store, false)?;
    let mut metadata_budget = MAX_METADATA_BYTES;
    let raw = get(&store, cid, &mut metadata_budget)?;
    let snapshot: Snapshot = serde_json::from_slice(&raw).map_err(invalid)?;
    if snapshot.version != 1 {
        return Err(invalid("unsupported snapshot version"));
    }
    let (root, objects) = trees(&snapshot.projection.sources)?;
    if root != snapshot.sources_tree {
        return Err(invalid("snapshot source tree mismatch"));
    }
    let mut tree_budget = MAX_METADATA_BYTES;
    for (cid, expected) in objects {
        if get(&store, &cid, &mut tree_budget)? != expected {
            return Err(invalid("corrupt source tree"));
        }
    }
    let mut bytes = BTreeMap::new();
    let mut budget = MAX_BYTES;
    for (path, cid) in &snapshot.projection.sources {
        bytes.insert(path.clone(), get(&store, cid, &mut budget)?);
    }
    let captured = Captured {
        projection: snapshot.projection,
        bytes,
        absent: Default::default(),
    };
    validate(&captured)?;
    Ok(captured)
}

/// Verified immutable envelope bytes for a browser to verify with the same raw CID.
pub fn snapshot_bytes(store: &Path, cid: &str) -> Result<Vec<u8>> {
    reopen(store, cid)?;
    let mut budget = MAX_METADATA_BYTES;
    get(&safe_store(store, false)?, cid, &mut budget)
}
