//! Git supplies the baseline; it cannot be widened by editing a debt file.
use crate::scan::{scan, Finding};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Command, Stdio},
};

pub const POLICY: &str = ".epr-meta/rust-style-policy.json";
pub const STRICT: &str = "rust-style-strict@1";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub version: u32,
    pub rules: Vec<String>,
    /// Explicit external source roots; submodules are already opaque gitlinks.
    pub external_roots: Vec<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct Report {
    pub version: u32,
    pub base: String,
    pub head: String,
    pub scanned_files: usize,
    pub crates: BTreeMap<String, CrateReport>,
    pub findings: Vec<Finding>,
    pub failures: Vec<String>,
    pub enrollment: bool,
}
#[derive(Debug, Default, Serialize)]
pub struct CrateReport {
    pub before: usize,
    pub after: usize,
    pub touched: bool,
    pub strict_files: usize,
}

fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(output.stdout)
}

pub fn commit(root: &Path, reference: &str) -> Result<String, String> {
    let bytes = git(
        root,
        &["rev-parse", "--verify", &format!("{reference}^{{commit}}")],
    )?;
    Ok(String::from_utf8_lossy(&bytes).trim().into())
}
pub fn merge_base(root: &Path, base: &str, head: &str) -> Result<String, String> {
    let bytes = git(
        root,
        &["merge-base", &commit(root, base)?, &commit(root, head)?],
    )?;
    Ok(String::from_utf8_lossy(&bytes).trim().into())
}

fn relevant(p: &str) -> bool {
    p.ends_with("clippy.toml")
        || p.ends_with("rustfmt.toml")
        || p.ends_with(".cargo/config.toml")
        || p.ends_with(".rs")
        || p == "Cargo.toml"
        || p.ends_with("/Cargo.toml")
        || p == ".epr-meta"
        || p.ends_with("/.epr-meta")
        || p.ends_with(".epr-meta/manifest.md")
        || p == POLICY
        || p == ".claude/epr-meta/policies.yaml"
}

#[derive(Default)]
pub struct Snapshot {
    pub files: BTreeMap<String, String>,
}
impl Snapshot {
    pub fn load(root: &Path, revision: Option<&str>) -> Result<Self, String> {
        let names = if let Some(rev) = revision {
            git(root, &["ls-tree", "-r", "--name-only", "-z", rev])?
        } else {
            git(
                root,
                &[
                    "ls-files",
                    "--cached",
                    "--others",
                    "--exclude-standard",
                    "-z",
                ],
            )?
        };
        let names: BTreeSet<String> = names
            .split(|b| *b == 0)
            .filter(|s| !s.is_empty())
            .map(|s| String::from_utf8(s.to_vec()).map_err(|_| "non-UTF-8 source path".to_string()))
            .collect::<Result<_, _>>()?;
        let names: Vec<_> = names.into_iter().filter(|p| relevant(p)).collect();
        let mut files = BTreeMap::new();
        if let Some(rev) = revision {
            // One batch avoids spawning git once per Rust file. Drain stdout while
            // another thread supplies object names, so pipe capacity cannot deadlock.
            let mut child = Command::new("git")
                .args(["cat-file", "--batch"])
                .current_dir(root)
                .env_remove("GIT_DIR")
                .env_remove("GIT_WORK_TREE")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .map_err(|e| e.to_string())?;
            let mut stdin = child.stdin.take().ok_or("missing git stdin")?;
            let queries = names
                .iter()
                .map(|p| format!("{rev}:{p}\n"))
                .collect::<String>();
            let writer = std::thread::spawn(move || stdin.write_all(queries.as_bytes()));
            let mut reader = BufReader::new(child.stdout.take().ok_or("missing git stdout")?);
            for name in names {
                let mut header = String::new();
                reader.read_line(&mut header).map_err(|e| e.to_string())?;
                let parts: Vec<_> = header.split_whitespace().collect();
                if parts.len() != 3 || parts[1] != "blob" {
                    return Err(format!("unreadable source {name}: {header}"));
                }
                let size = parts[2].parse::<usize>().map_err(|e| e.to_string())?;
                if size > 32 * 1024 * 1024 {
                    return Err(format!("source too large: {name}"));
                }
                let mut bytes = vec![0; size + 1];
                reader.read_exact(&mut bytes).map_err(|e| e.to_string())?;
                bytes.pop();
                files.insert(name, String::from_utf8(bytes).map_err(|e| e.to_string())?);
            }
            writer
                .join()
                .map_err(|_| "git input worker failed")?
                .map_err(|e| e.to_string())?;
            if !child.wait().map_err(|e| e.to_string())?.success() {
                return Err("git cat-file failed".into());
            }
        } else {
            for name in names {
                let p = root.join(&name);
                if !p.exists() {
                    continue;
                } // deleted working-tree path
                if fs::symlink_metadata(&p)
                    .map_err(|e| e.to_string())?
                    .file_type()
                    .is_symlink()
                {
                    return Err(format!(
                        "source symlink needs an explicit external boundary: {name}"
                    ));
                }
                files.insert(name, fs::read_to_string(p).map_err(|e| e.to_string())?);
            }
        }
        Ok(Self { files })
    }

    fn policy(&self) -> Result<Policy, String> {
        let policy: Policy =
            serde_json::from_str(self.files.get(POLICY).ok_or("missing Rust style policy")?)
                .map_err(|e| e.to_string())?;
        if policy.version != 1 || policy.rules != ["RS-GOV-04"] {
            return Err("unsupported Rust style policy; migrate the checker explicitly".into());
        }
        if policy.external_roots.iter().any(|s| {
            s.is_empty() || s.starts_with('/') || s.split('/').any(|p| p == ".." || p == ".")
        }) {
            return Err("external roots must be nonempty repository-relative paths".into());
        }
        Ok(policy)
    }
    fn crate_for(&self, file: &str) -> String {
        let mut at = Path::new(file).parent();
        while let Some(dir) = at {
            let key = dir.join("Cargo.toml").to_string_lossy().into_owned();
            if let Some(text) = self.files.get(&key) {
                if toml::from_str::<toml::Value>(text)
                    .ok()
                    .is_some_and(|v| v.get("package").is_some())
                {
                    return if dir.as_os_str().is_empty() {
                        ".".into()
                    } else {
                        dir.to_string_lossy().into_owned()
                    };
                }
            }
            at = dir.parent();
        }
        format!(
            "loose:{}",
            Path::new(file).parent().unwrap_or(Path::new("")).display()
        )
    }
    fn manifests(&self) -> Result<tempfile::TempDir, String> {
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        for (name, body) in &self.files {
            if name == ".epr-meta"
                || name.ends_with("/.epr-meta")
                || name.ends_with(".epr-meta/manifest.md")
                || name.ends_with("policies.yaml")
            {
                let path = dir.path().join(name);
                fs::create_dir_all(path.parent().ok_or("manifest has no parent")?)
                    .map_err(|e| e.to_string())?;
                fs::write(path, body).map_err(|e| e.to_string())?;
            }
        }
        Ok(dir)
    }
}

fn external(policy: &Policy, path: &str) -> bool {
    policy
        .external_roots
        .iter()
        .any(|p| path == p || path.starts_with(&format!("{p}/")))
}

/// The native resolver owns inheritance. Strict bindings deliberately have no
/// local `when`/`params`: place one in the directory whose subtree is graduating.
pub fn strict(root: &Path, path: &str) -> Result<bool, String> {
    let resolution = eprfs_meta::resolve_path(root, root.join(path)).map_err(|e| e.to_string())?;
    let bindings: Vec<_> = resolution
        .effective_policies
        .iter()
        .filter(|b| b.policy == STRICT)
        .collect();
    if bindings
        .iter()
        .any(|b| b.when.is_some() || !b.params.is_null())
    {
        return Err("strict Rust scope cannot have when/params overrides; put its manifest at the intended boundary".into());
    }
    Ok(!bindings.is_empty())
}

fn cached_strict(
    root: &Path,
    path: &str,
    cache: &mut BTreeMap<String, bool>,
) -> Result<bool, String> {
    let parent = Path::new(path)
        .parent()
        .unwrap_or(Path::new(""))
        .to_string_lossy()
        .into_owned();
    if let Some(value) = cache.get(&parent) {
        return Ok(*value);
    }
    let value = strict(root, path)?;
    cache.insert(parent, value);
    Ok(value)
}

pub fn evaluate(
    root: &Path,
    base: &str,
    head: Option<&str>,
    scope: Option<&str>,
) -> Result<Report, String> {
    let base = commit(root, base)?;
    let head = head.map(|h| commit(root, h)).transpose()?;
    let mut before = Snapshot::load(root, Some(&base))?;
    let mut after = Snapshot::load(root, head.as_deref())?;
    if let Some(scope) = scope.filter(|s| *s != ".") {
        for snapshot in [&mut before, &mut after] {
            snapshot.files.retain(|path, _| {
                !path.ends_with(".rs") || path == scope || path.starts_with(&format!("{scope}/"))
            });
        }
    }
    compare(
        &before,
        &after,
        base,
        head.unwrap_or_else(|| "working-tree".into()),
    )
}

pub fn compare(
    before: &Snapshot,
    after: &Snapshot,
    base: String,
    head: String,
) -> Result<Report, String> {
    let policy = after.policy()?;
    let enrollment = !before.files.contains_key(POLICY);
    let mut report = Report {
        version: 1,
        base,
        head,
        enrollment,
        ..Report::default()
    };
    if !enrollment && before.policy()? != policy {
        report.failures.push("style policy changed: v1 forbids rule removal or exclusion widening; use a reviewed checker migration".into());
    }
    let old_manifests = before.manifests()?;
    let new_manifests = after.manifests()?;
    let mut baseline = BTreeMap::<String, usize>::new();
    let mut old_strict = BTreeMap::new();
    let mut new_strict = BTreeMap::new();
    for (path, body) in &before.files {
        if !path.ends_with(".rs") || external(&policy, path) {
            continue;
        }
        let findings = scan(path, body)?;
        report
            .crates
            .entry(before.crate_for(path))
            .or_default()
            .before += findings.len();
        for finding in findings {
            *baseline
                .entry(format!("{}|{}", before.crate_for(path), finding.identity()))
                .or_default() += 1;
        }
        if cached_strict(old_manifests.path(), path, &mut old_strict)?
            && !cached_strict(new_manifests.path(), path, &mut new_strict)?
        {
            report.failures.push(format!(
                "{path}: permanent strict scope was removed or weakened"
            ));
        }
    }
    for (path, body) in &after.files {
        if !path.ends_with(".rs") || external(&policy, path) {
            continue;
        }
        report.scanned_files += 1;
        let crate_name = after.crate_for(path);
        let entry = report.crates.entry(crate_name.clone()).or_default();
        entry.touched |= before.files.get(path) != Some(body);
        let is_strict = cached_strict(new_manifests.path(), path, &mut new_strict)?;
        if !before.files.contains_key(path)
            && cached_strict(old_manifests.path(), path, &mut old_strict)?
            && !is_strict
        {
            report.failures.push(format!(
                "{path}: permanent strict scope was weakened for a new descendant"
            ));
        }
        entry.strict_files += usize::from(is_strict);
        let findings = scan(path, body)?;
        entry.after += findings.len();
        for finding in findings {
            let remaining = baseline
                .entry(format!("{crate_name}|{}", finding.identity()))
                .or_default();
            if is_strict || *remaining == 0 {
                report.failures.push(format!(
                    "{}:{} {} {}: {}",
                    path,
                    finding.line,
                    finding.rule,
                    if is_strict { "strict" } else { "new" },
                    finding.detail
                ));
            }
            *remaining = remaining.saturating_sub(1);
            report.findings.push(finding);
        }
    }
    // Deletions and Cargo/policy changes count as touches. A workspace manifest
    // affects every package below it; indirect dependency recompilation does not.
    for path in before
        .files
        .keys()
        .chain(after.files.keys())
        .collect::<BTreeSet<_>>()
    {
        if external(&policy, path) || before.files.get(path) == after.files.get(path) {
            continue;
        }
        if path.ends_with(".rs") {
            if before.files.contains_key(path) {
                report
                    .crates
                    .entry(before.crate_for(path))
                    .or_default()
                    .touched = true;
            }
        } else if path.ends_with("clippy.toml")
            || path.ends_with("rustfmt.toml")
            || path.ends_with(".cargo/config.toml")
            || path.ends_with("Cargo.toml")
            || path.ends_with(".epr-meta")
            || path.ends_with(".epr-meta/manifest.md")
            || path == POLICY
        {
            let owner = after.crate_for(path);
            if let Some(entry) = report.crates.get_mut(&owner) {
                entry.touched = true;
            }
            let dir = if path.ends_with(".epr-meta/manifest.md")
                || path.ends_with(".cargo/config.toml")
            {
                Path::new(path).parent().and_then(Path::parent)
            } else {
                Path::new(path).parent()
            };
            let dir = dir.unwrap_or(Path::new("")).to_string_lossy();
            for (name, entry) in &mut report.crates {
                if dir.is_empty() || name == dir.as_ref() || name.starts_with(&format!("{dir}/")) {
                    entry.touched = true;
                }
            }
        }
    }
    for (name, entry) in &report.crates {
        if !enrollment && entry.touched && entry.before > 0 && entry.after >= entry.before {
            report.failures.push(format!(
                "{name}: debt {} -> {}; touched crate must remove at least one existing finding",
                entry.before, entry.after
            ));
        }
    }
    Ok(report)
}

/// Fast edit-time check against the last committed source. Paydown is a reach
/// obligation; an individual intermediate write may keep existing debt.
pub fn prospective(root: &Path, path: &str, content: &str) -> Result<Vec<String>, String> {
    let policy: Policy =
        serde_json::from_str(&fs::read_to_string(root.join(POLICY)).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if external(&policy, path) || !path.ends_with(".rs") {
        return Ok(Vec::new());
    }
    let previous = git(root, &["show", &format!("HEAD:{path}")])
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .unwrap_or_default();
    let mut baseline = BTreeMap::<String, usize>::new();
    for finding in scan(path, &previous)? {
        *baseline.entry(finding.identity()).or_default() += 1;
    }
    let is_strict = strict(root, path)?;
    let mut messages = Vec::new();
    for finding in scan(path, content)? {
        let count = baseline.entry(finding.identity()).or_default();
        if is_strict || *count == 0 {
            messages.push(format!(
                "{path}:{} {}: {}",
                finding.line, finding.rule, finding.detail
            ));
        }
        *count = count.saturating_sub(1);
    }
    Ok(messages)
}
