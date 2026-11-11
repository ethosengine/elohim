//! Immutable sysroot payloads; a materialization is only a reconstructable cache.
//!
//! EPRSYS01 format: eight magic bytes, a little-endian u64 canonical-JSON manifest length,
//! the manifest, then (in manifest order) a little-endian u64 length and raw bytes per regular
//! file. Directories have no body. JSON object keys are sorted, arrays are path-sorted. Paths
//! are UTF-8 root-relative slash paths. POSIX modes include the type: 040555 directory,
//! 0100444 regular file. Ownership and time are implicitly zero, never read or serialized.
//! Symlinks are dereferenced only after canonical containment and cycle checks. No host paths
//! enter the manifest. Its raw-byte CID pins the complete file set and observed Requires
//! evidence (ISA notes, script dependencies, libc header version); it is not an ISA admission
//! certificate. In particular a v3 payload stays v3, irrespective of the triple.
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};

use eprfs_core::BlobCid;
use serde::{Deserialize, Serialize};

use super::{ArtifactError, ArtifactResult};

#[path = "sysroot_evidence.rs"]
mod evidence;
#[path = "sysroot_limits.rs"]
pub mod limits;
use limits::{Budget, Limit};
#[cfg(test)]
#[path = "sysroot_tests.rs"]
mod tests;

const MAGIC: &[u8; 8] = b"EPRSYS01";
const DIR: u32 = 0o40555;
const FILE: u32 = 0o100444;
type Members = BTreeMap<String, Option<Vec<u8>>>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    pub version: u32,
    pub triple: String,
    pub files: Vec<Member>,
    pub scripts: Vec<Script>,
    pub isa_notes: Vec<IsaNote>,
    /// From the packed features.h's __GLIBC__/__GLIBC_MINOR__; null when not observable.
    pub libc_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Member {
    pub path: String,
    pub mode: u32,
    pub cid: BlobCid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Script {
    pub path: String,
    pub operands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IsaNote {
    pub path: String,
    pub needed: Vec<String>,
}

fn refused(reason: impl std::fmt::Display) -> ArtifactError {
    ArtifactError::Gap(format!("sysroot: {reason}"))
}

impl Manifest {
    /// Canonical JSON bytes, with lexicographically sorted object keys and no whitespace.
    pub fn canonical_bytes(&self) -> ArtifactResult<Vec<u8>> {
        limits::manifest_size(self)?;
        let mut value = serde_json::to_value(self)?;
        // Remain canonical even when another workspace consumer enables preserve_order.
        value.sort_all_objects();
        Ok(serde_json::to_vec(&value)?)
    }

    pub fn cid(&self) -> ArtifactResult<BlobCid> {
        Ok(BlobCid::compute_bytes(&self.canonical_bytes()?))
    }
}

fn valid_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\0')
        && !path.contains('\\')
        && path
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != "..")
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}

fn walk(
    root: &Path,
    source: &Path,
    relative: &str,
    ancestors: &mut Vec<PathBuf>,
    members: &mut Members,
    budget: &mut Budget,
) -> ArtifactResult<()> {
    if !relative.is_empty() {
        budget.member(relative)?;
    }
    let resolved = fs::canonicalize(source).map_err(refused)?;
    if !resolved.starts_with(root) {
        return Err(refused(format!("symlink escapes root: {relative}")));
    }
    let meta = fs::metadata(&resolved)?;
    if meta.is_dir() {
        if ancestors.contains(&resolved) {
            return Err(refused(format!("directory cycle: {relative}")));
        }
        if !relative.is_empty() {
            members.insert(relative.into(), None);
        }
        ancestors.push(resolved.clone());
        for entry in fs::read_dir(&resolved)? {
            let entry = entry?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| refused("non-UTF-8 path"))?;
            let child = if relative.is_empty() {
                name
            } else {
                format!("{relative}/{name}")
            };
            if !valid_path(&child) {
                return Err(refused("invalid member path"));
            }
            walk(root, &entry.path(), &child, ancestors, members, budget)?;
        }
        ancestors.pop();
    } else if meta.is_file() {
        let bytes = limits::read_file(&resolved, budget.bytes)?;
        budget.file(bytes.len() as u64)?;
        members.insert(relative.into(), Some(bytes));
    } else {
        return Err(refused(format!(
            "not a regular file or directory: {relative}"
        )));
    }
    Ok(())
}

fn manifest(triple: &str, members: &Members) -> ArtifactResult<Manifest> {
    if triple.is_empty() {
        return Err(refused("empty target triple"));
    }
    let mut result = Manifest {
        version: 1,
        triple: triple.into(),
        files: Vec::new(),
        scripts: Vec::new(),
        isa_notes: Vec::new(),
        libc_version: None,
    };
    for (path, content) in members {
        result.files.push(Member {
            path: path.clone(),
            mode: if content.is_some() { FILE } else { DIR },
            cid: BlobCid::compute_bytes(content.as_deref().unwrap_or_default()),
        });
        if let Some(bytes) = content {
            if let Some(operands) = evidence::script(path, bytes, members)? {
                result.scripts.push(Script {
                    path: path.clone(),
                    operands,
                });
            }
            if let Some(needed) = evidence::isa(bytes)? {
                result.isa_notes.push(IsaNote {
                    path: path.clone(),
                    needed,
                });
            }
            if path == "usr/include/features.h" {
                result.libc_version = evidence::libc_version(bytes);
            }
        }
    }
    Ok(result)
}

/// Pack a tree without recording source locations, metadata, or following an escaping symlink.
pub fn pack(input_dir: &Path, triple: &str) -> ArtifactResult<(Vec<u8>, Manifest)> {
    let root = fs::canonicalize(input_dir)?;
    if !root.is_dir() {
        return Err(refused("input is not a directory"));
    }
    let mut members = Members::new();
    let mut budget = Budget::default();
    walk(&root, &root, "", &mut Vec::new(), &mut members, &mut budget)?;
    let manifest = manifest(triple, &members)?;
    let json = manifest.canonical_bytes()?;
    let archive_len = 16 + json.len() as u64 + budget.members * 8 + budget.bytes;
    Limit::ArchiveBytes.check(archive_len)?;
    let mut archive = Vec::new();
    archive
        .try_reserve_exact(usize::try_from(archive_len).map_err(refused)?)
        .map_err(std::io::Error::other)?;
    archive.extend_from_slice(MAGIC);
    archive.extend_from_slice(&(json.len() as u64).to_le_bytes());
    archive.extend(json);
    for bytes in members.values().flatten() {
        archive.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        archive.extend(bytes);
    }
    Ok((archive, manifest))
}

fn take<'a>(bytes: &mut &'a [u8], len: usize) -> ArtifactResult<&'a [u8]> {
    if len > bytes.len() {
        return Err(refused("truncated archive"));
    }
    let (head, tail) = bytes.split_at(len);
    *bytes = tail;
    Ok(head)
}

fn length(bytes: &mut &[u8]) -> ArtifactResult<u64> {
    let mut len = [0; 8];
    len.copy_from_slice(take(bytes, 8)?);
    Ok(u64::from_le_bytes(len))
}

fn sized<'a>(bytes: &mut &'a [u8], limit: Limit) -> ArtifactResult<&'a [u8]> {
    let len = length(bytes)?;
    limit.check(len)?;
    take(bytes, usize::try_from(len).map_err(refused)?)
}

fn file_body<'a>(bytes: &mut &'a [u8], budget: &mut Budget) -> ArtifactResult<&'a [u8]> {
    let len = length(bytes)?;
    budget.file(len)?;
    take(bytes, usize::try_from(len).map_err(refused)?)
}

/// Count the files array without allocating members; manifest bytes were already bounded.
fn preflight_manifest(json: &[u8]) -> ArtifactResult<()> {
    fn count<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
        struct Counter;
        impl<'de> serde::de::Visitor<'de> for Counter {
            type Value = u64;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a bounded member array")
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<u64, A::Error> {
                let mut count = 0;
                while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {
                    count += 1;
                }
                Ok(count)
            }
        }
        deserializer.deserialize_seq(Counter)
    }
    #[derive(Deserialize)]
    struct Counts {
        #[serde(deserialize_with = "count")]
        files: u64,
    }
    let counts: Counts = serde_json::from_slice(json).map_err(refused)?;
    Limit::Members.check(counts.files)
}

fn decode(mut archive: &[u8]) -> ArtifactResult<(Manifest, Members)> {
    Limit::ArchiveBytes.check(archive.len() as u64)?;
    if take(&mut archive, MAGIC.len())? != MAGIC {
        return Err(refused("unknown archive format"));
    }
    let json = sized(&mut archive, Limit::ManifestBytes)?;
    preflight_manifest(json)?;
    let declared: Manifest = serde_json::from_slice(json).map_err(refused)?;
    if declared.version != 1 || declared.canonical_bytes()? != json {
        return Err(refused("noncanonical manifest or unknown version"));
    }
    let mut members = Members::new();
    let mut budget = Budget::default();
    let mut previous = "";
    for file in &declared.files {
        budget.member(&file.path)?;
        if !valid_path(&file.path) || file.path.as_str() <= previous {
            return Err(refused("unsafe, duplicate or unsorted path"));
        }
        previous = &file.path;
        if let Some((parent, _)) = file.path.rsplit_once('/') {
            if members.get(parent) != Some(&None) {
                return Err(refused("missing parent directory"));
            }
        }
        let bytes = match file.mode {
            FILE => {
                let bytes = file_body(&mut archive, &mut budget)?;
                Some(bytes.to_vec())
            }
            DIR => None,
            _ => return Err(refused("non-normalized mode")),
        };
        if BlobCid::compute_bytes(bytes.as_deref().unwrap_or_default()) != file.cid {
            return Err(refused(format!("file CID mismatch: {}", file.path)));
        }
        members.insert(file.path.clone(), bytes);
    }
    if !archive.is_empty() {
        return Err(refused("trailing archive bytes"));
    }
    if manifest(&declared.triple, &members)? != declared {
        return Err(refused("manifest evidence disagrees with payload"));
    }
    Ok((declared, members))
}

/// Verify every member, script resolution and ELF observation before recording the archive.
pub fn verify(archive: &[u8]) -> ArtifactResult<Manifest> {
    Ok(decode(archive)?.0)
}

fn check_cache(root: &Path, members: &Members) -> ArtifactResult<()> {
    fn visit(
        root: &Path,
        relative: &str,
        found: &mut Members,
        budget: &mut Budget,
    ) -> ArtifactResult<()> {
        if !relative.is_empty() {
            budget.member(relative)?;
        }
        let path = root.join(relative);
        let meta = fs::symlink_metadata(&path)?;
        if meta.is_dir() && meta.permissions().mode() & 0o7777 == 0o555 {
            if !relative.is_empty() {
                found.insert(relative.into(), None);
            }
            for entry in fs::read_dir(path)? {
                let name = entry?
                    .file_name()
                    .into_string()
                    .map_err(|_| refused("cache path is not UTF-8"))?;
                let child = if relative.is_empty() {
                    name
                } else {
                    format!("{relative}/{name}")
                };
                visit(root, &child, found, budget)?;
            }
        } else if meta.is_file() && meta.permissions().mode() & 0o7777 == 0o444 {
            let bytes = limits::read_file(&path, budget.bytes)?;
            budget.file(bytes.len() as u64)?;
            found.insert(relative.into(), Some(bytes));
        } else {
            return Err(refused(
                "cache contains a link, special file or writable member",
            ));
        }
        Ok(())
    }
    let mut found = Members::new();
    visit(root, "", &mut found, &mut Budget::default())?;
    if &found != members {
        return Err(refused("incomplete or corrupt materialization"));
    }
    Ok(())
}

/// Verify and atomically materialize at `sysroots/<archive CID>/root`. Existing complete trees
/// are verified hits; damage is refused, never mistaken for a hit or silently overwritten.
pub fn materialize(archive: &[u8], cache_root: &Path) -> ArtifactResult<PathBuf> {
    let (_, members) = decode(archive)?;
    let parent = cache_root.join("sysroots");
    fs::create_dir_all(&parent)?;
    let destination = parent.join(BlobCid::compute_bytes(archive).to_string());
    let root = destination.join("root");
    if fs::symlink_metadata(&destination).is_ok() {
        if fs::symlink_metadata(&destination)?.file_type().is_symlink() {
            return Err(refused("cache destination is a symlink"));
        }
        check_cache(&root, &members)?;
        return Ok(root);
    }
    let stage = tempfile::Builder::new()
        .prefix(".sysroot-")
        .tempdir_in(&parent)?;
    let staged_root = stage.path().join("root");
    fs::create_dir(&staged_root)?;
    for (path, content) in &members {
        let path = staged_root.join(path);
        if let Some(bytes) = content {
            fs::write(&path, bytes)?;
            fs::set_permissions(path, fs::Permissions::from_mode(0o444))?;
        } else {
            fs::create_dir(path)?;
        }
    }
    for (path, _) in members.iter().rev().filter(|(_, bytes)| bytes.is_none()) {
        fs::set_permissions(staged_root.join(path), fs::Permissions::from_mode(0o555))?;
    }
    fs::set_permissions(&staged_root, fs::Permissions::from_mode(0o555))?;
    fs::set_permissions(stage.path(), fs::Permissions::from_mode(0o555))?;
    let renamed = fs::rename(stage.path(), &destination);
    if renamed.is_err() {
        // TempDir cannot remove a frozen tree as an ordinary user. Thaw only our unpublished
        // staging directories, including when another materializer won the rename.
        fs::set_permissions(stage.path(), fs::Permissions::from_mode(0o755))?;
        fs::set_permissions(&staged_root, fs::Permissions::from_mode(0o755))?;
        for (path, _) in members.iter().filter(|(_, bytes)| bytes.is_none()) {
            fs::set_permissions(staged_root.join(path), fs::Permissions::from_mode(0o755))?;
        }
    }
    match renamed {
        Ok(()) => {}
        Err(error) => {
            // A concurrent complete producer may have won the atomic rename.
            if !fs::symlink_metadata(&destination).is_ok_and(|meta| meta.is_dir()) {
                return Err(error.into());
            }
            check_cache(&root, &members)?;
        }
    }
    Ok(root)
}
