//! Confined bounded reads, governance provenance, and optimistic capture validation.
use super::{declaration::Declaration, invalid, Result};
use eprfs_core::BlobCid;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
};

pub fn relative(path: &str) -> Result<()> {
    if path.is_empty()
        || path.contains('\\')
        || Path::new(path)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(invalid(format!(
            "repository-relative path required: {path}"
        )));
    }
    Ok(())
}
pub fn confined(root: &Path, path: &str) -> Result<PathBuf> {
    relative(path)?;
    let mut out = root.to_path_buf();
    for component in Path::new(path).components() {
        out.push(component);
        match fs::symlink_metadata(&out) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(invalid(format!("symlink input refused: {path}")))
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(invalid(e)),
        }
    }
    Ok(out)
}

/// Walk descriptors instead of checking then following pathnames. NONBLOCK prevents a
/// malicious FIFO replacement from hanging before the regular-file check.
pub(super) fn open_file(path: &Path) -> Result<fs::File> {
    use rustix::fs::{open, openat, Mode, OFlags};
    let parts: Vec<_> = path
        .components()
        .filter_map(|c| match c {
            Component::Normal(p) => Some(p),
            _ => None,
        })
        .collect();
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(invalid("absolute confined file path required"));
    }
    let mut dir = open(
        "/",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(invalid)?;
    for (index, part) in parts.iter().enumerate() {
        let last = index + 1 == parts.len();
        let flags = OFlags::RDONLY
            | OFlags::NOFOLLOW
            | OFlags::CLOEXEC
            | if last {
                OFlags::NONBLOCK
            } else {
                OFlags::DIRECTORY
            };
        let next = openat(&dir, *part, flags, Mode::empty()).map_err(invalid)?;
        if last {
            let file = fs::File::from(next);
            if !file.metadata().map_err(invalid)?.is_file() {
                return Err(invalid("input must be a regular file"));
            }
            return Ok(file);
        }
        dir = next;
    }
    Err(invalid("file path required"))
}

pub(super) fn absolute_directory(path: &Path) -> Result<PathBuf> {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().map_err(invalid)?.join(path)
    };
    if abs.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(invalid("parent component in directory path"));
    }
    Ok(abs
        .components()
        .filter(|c| !matches!(c, Component::CurDir))
        .collect())
}
pub(super) fn open_directory(path: &Path, create: bool) -> Result<fs::File> {
    use rustix::fs::{mkdirat, open, openat, Mode, OFlags};
    let abs = absolute_directory(path)?;
    let mut dir = open(
        "/",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(invalid)?;
    for part in abs.components() {
        let Component::Normal(part) = part else {
            continue;
        };
        let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        dir = match openat(&dir, part, flags, Mode::empty()) {
            Ok(next) => next,
            Err(e) if create && e == rustix::io::Errno::NOENT => {
                match mkdirat(&dir, part, Mode::from_raw_mode(0o700)) {
                    Ok(()) => {}
                    Err(e) if e == rustix::io::Errno::EXIST => {}
                    Err(e) => return Err(invalid(e)),
                }
                openat(&dir, part, flags, Mode::empty()).map_err(invalid)?
            }
            Err(e) => {
                return Err(invalid(format!(
                    "directory must exist without symlinks: {e}"
                )))
            }
        };
    }
    Ok(fs::File::from(dir))
}

pub struct Capture {
    pub root: PathBuf,
    pub bytes: BTreeMap<String, Vec<u8>>,
    pub governance: BTreeMap<String, Value>,
    pub governance_sources: BTreeSet<String>,
    pub absent: BTreeSet<String>,
    total: usize,
    max_files: usize,
    max_bytes: usize,
}
impl Capture {
    pub fn new(root: &Path, max_files: usize, max_bytes: usize) -> Result<Self> {
        let root = absolute_directory(root)?;
        open_directory(&root, false)?;
        Ok(Self {
            root,
            bytes: BTreeMap::new(),
            governance: BTreeMap::new(),
            governance_sources: BTreeSet::new(),
            absent: BTreeSet::new(),
            total: 0,
            max_files,
            max_bytes,
        })
    }
    pub fn set_limits(&mut self, files: usize, bytes: usize) -> Result<()> {
        self.max_files = files;
        self.max_bytes = bytes;
        if self.bytes.len() > files || self.total > bytes {
            return Err(invalid("declaration and governance exceed capture bounds"));
        }
        Ok(())
    }
    pub fn read(&mut self, path: &str, optional: bool) -> Result<Option<Vec<u8>>> {
        if let Some(bytes) = self.bytes.get(path) {
            return Ok(Some(bytes.clone()));
        }
        let full = confined(&self.root, path)?;
        if optional && !full.try_exists().map_err(invalid)? {
            self.absent.insert(path.into());
            return Ok(None);
        }
        let file = open_file(&full).map_err(|e| invalid(format!("{path}: {e}")))?;
        if !file.metadata().map_err(invalid)?.is_file() {
            return Err(invalid(format!("not a regular file: {path}")));
        }
        if self.bytes.len() >= self.max_files {
            return Err(invalid("capture file bound exceeded"));
        }
        let mut bytes = Vec::new();
        file.take((self.max_bytes - self.total + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(invalid)?;
        self.total += bytes.len();
        if self.total > self.max_bytes {
            return Err(invalid("capture byte bound exceeded"));
        }
        self.bytes.insert(path.into(), bytes.clone());
        Ok(Some(bytes))
    }
    pub fn govern(&mut self, path: &str) -> Result<()> {
        if self.governance.contains_key(path) {
            return Ok(());
        }
        let full = confined(&self.root, path)?;
        // Inspect and capture every possible ancestor before the resolver reads it.
        let mut parent = full.parent();
        let mut manifests = Vec::new();
        let mut at_root = false;
        while let Some(dir) = parent {
            if !dir.starts_with(&self.root) {
                break;
            }
            for candidate in [dir.join(".epr-meta"), dir.join(".epr-meta/manifest.md")] {
                let rel = candidate
                    .strip_prefix(&self.root)
                    .map_err(invalid)?
                    .to_str()
                    .ok_or_else(|| invalid("non-UTF8 governance path"))?
                    .to_string();
                let safe = confined(&self.root, &rel)?;
                if safe.is_file() {
                    self.governance_sources.insert(rel.clone());
                    let bytes = self.read(&rel, false)?.unwrap();
                    let text = String::from_utf8(bytes).map_err(invalid)?;
                    if let Some((lines, _)) = crate::flow::cites::split_frontmatter(&text) {
                        let cfg = serde_yaml::from_str(&lines.join("\n")).map_err(invalid)?;
                        let problems = eprfs_meta::validate_meta(&cfg);
                        at_root =
                            cfg.get("root").and_then(serde_yaml::Value::as_bool) == Some(true);
                        if !problems.is_empty() {
                            return Err(invalid(format!(
                                "invalid governance {rel}: {}",
                                problems.join("; ")
                            )));
                        }
                    }
                    manifests.push((safe, text));
                    break;
                } else if !safe.exists() {
                    self.absent.insert(rel);
                }
            }
            if at_root || dir == self.root {
                break;
            }
            parent = dir.parent();
        }
        let resolved =
            eprfs_meta::resolve_captured_cascade(&self.root, &full, &manifests).map_err(invalid)?;
        self.governance.insert(
            path.into(),
            serde_json::to_value(resolved).map_err(invalid)?,
        );
        Ok(())
    }
    pub fn verify(&self) -> Result<()> {
        for (path, bytes) in &self.bytes {
            let full = confined(&self.root, path)?;
            let file = open_file(&full)?;
            let mut current = Vec::new();
            file.take(bytes.len() as u64 + 1)
                .read_to_end(&mut current)
                .map_err(invalid)?;
            if current != *bytes {
                return Err(invalid(format!("concurrent input change: {path}")));
            }
        }
        for path in &self.absent {
            if confined(&self.root, path)?.exists() {
                return Err(invalid(format!("concurrent input appeared: {path}")));
            }
        }
        Ok(())
    }
    pub fn versions(&self) -> BTreeMap<String, String> {
        self.bytes
            .iter()
            .map(|(p, b)| (p.clone(), BlobCid::compute_raw(b).to_string()))
            .collect()
    }
}

pub fn discover(
    capture: &Capture,
    declaration: &Declaration,
) -> Result<BTreeMap<String, Vec<String>>> {
    let mut result = BTreeMap::new();
    let mut visited = 0;
    for (name, binding) in &declaration.sources {
        let mut files = Vec::new();
        for location in binding.path.paths() {
            let path = confined(&capture.root, location)?;
            if let Some(pattern) = &binding.pattern {
                if !binding.optional || path.exists() {
                    walk(
                        capture,
                        declaration,
                        &path,
                        &glob::Pattern::new(pattern).map_err(invalid)?,
                        0,
                        &mut visited,
                        &mut files,
                    )?;
                }
            } else if path.exists() || !binding.optional {
                files.push(location.to_string());
            }
        }
        files.sort();
        files.dedup();
        result.insert(name.clone(), files);
    }
    Ok(result)
}
fn walk(
    capture: &Capture,
    declaration: &Declaration,
    dir: &Path,
    pattern: &glob::Pattern,
    depth: usize,
    visited: &mut usize,
    files: &mut Vec<String>,
) -> Result<()> {
    if depth > declaration.bounds.max_depth {
        return Err(invalid("traversal depth exceeded"));
    }
    let directory = open_directory(dir, false)?;
    for entry in rustix::fs::Dir::read_from(&directory).map_err(invalid)? {
        let entry = entry.map_err(invalid)?;
        let name = entry.file_name().to_str().map_err(invalid)?;
        if matches!(name, "." | "..") {
            continue;
        }
        *visited += 1;
        if *visited > declaration.bounds.max_files {
            return Err(invalid("traversal entry bound exceeded"));
        }
        let path = dir.join(name);
        let rel = path
            .strip_prefix(&capture.root)
            .map_err(invalid)?
            .to_str()
            .ok_or_else(|| invalid("non-UTF8 path"))?
            .to_string();
        confined(&capture.root, &rel)?;
        if entry.file_type() == rustix::fs::FileType::Directory {
            walk(
                capture,
                declaration,
                &path,
                pattern,
                depth + 1,
                visited,
                files,
            )?;
        } else if pattern.matches(name) {
            files.push(rel);
        }
    }
    Ok(())
}
