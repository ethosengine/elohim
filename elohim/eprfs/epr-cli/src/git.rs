//! Repository commands and the device-local, exact-checkout trust declaration.
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    process,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Untracked,
    Conflicted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangedPath {
    pub path: String,
    pub kind: ChangeKind,
}

impl ChangedPath {
    pub fn is_new(&self) -> bool {
        matches!(
            self.kind,
            ChangeKind::Added | ChangeKind::Untracked | ChangeKind::Renamed
        )
    }

    pub fn is_deleted(&self) -> bool {
        self.kind == ChangeKind::Deleted
    }
}

pub fn discover_root(start: &Path) -> Result<PathBuf> {
    let start = if start.is_file() {
        start.parent().unwrap_or(start)
    } else {
        start
    };
    let output = process::run("git", &["rev-parse", "--show-toplevel"], start)?;
    if !output.success || output.stdout.is_empty() {
        return Err(Error::RepositoryNotFound(start.to_path_buf()));
    }
    Ok(PathBuf::from(output.stdout))
}

/// Use Git's stable porcelain wire format as a replaceable repository adapter.
/// All governance semantics remain in Rust; this module can later swap to
/// brit/gix without changing the library or CLI contracts.
pub fn working_tree_changes(root: &Path) -> Result<Vec<ChangedPath>> {
    let output = crate::process::build_command(
        "git",
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
        root,
        &[],
    )
    .output()
    .map_err(|source| Error::Spawn {
        program: "git".into(),
        source,
    })?;
    if !output.status.success() {
        return Err(Error::Command {
            program: "git status".into(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().into(),
        });
    }

    let records: Vec<&[u8]> = output.stdout.split(|byte| *byte == 0).collect();
    let mut changes = BTreeMap::new();
    let mut index = 0;
    while index < records.len() {
        let record = records[index];
        index += 1;
        if record.len() < 4 {
            continue;
        }
        let x = record[0] as char;
        let y = record[1] as char;
        let path = String::from_utf8_lossy(&record[3..]).replace('\\', "/");
        let kind = classify(x, y);
        if matches!(x, 'R' | 'C') || matches!(y, 'R' | 'C') {
            // Porcelain -z emits the source path as the next NUL record. The
            // destination above is the path callers must evaluate.
            index += usize::from(index < records.len());
        }
        changes.insert(path.clone(), ChangedPath { path, kind });
    }
    Ok(changes.into_values().collect())
}

/// Return the committed paths that would gain reach relative to `target`.
/// The three-dot range uses the merge-base, matching the branch-shaped change
/// set a developer normally intends to push or merge.
pub fn changes_against(root: &Path, target: &str) -> Result<Vec<ChangedPath>> {
    let range = format!("{target}...HEAD");
    let output = crate::process::build_command(
        "git",
        &["diff", "--name-status", "-z", "--find-renames", &range],
        root,
        &[],
    )
    .output()
    .map_err(|source| Error::Spawn {
        program: "git".into(),
        source,
    })?;
    if !output.status.success() {
        return Err(Error::Command {
            program: format!("git diff {range}"),
            stderr: String::from_utf8_lossy(&output.stderr).trim().into(),
        });
    }

    parse_name_status(&output.stdout)
}

pub fn default_target(root: &Path) -> String {
    process::run(
        "git",
        &[
            "rev-parse",
            "--abbrev-ref",
            "--symbolic-full-name",
            "@{upstream}",
        ],
        root,
    )
    .ok()
    .filter(|output| output.success && !output.stdout.is_empty())
    .map(|output| output.stdout)
    .unwrap_or_else(|| "origin/dev".into())
}

pub fn content_at(root: &Path, revision: &str, path: &str) -> Option<String> {
    let object = format!("{revision}:{path}");
    let output = crate::process::build_command("git", &["show", &object], root, &[])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn merge_base(root: &Path, target: &str) -> Result<String> {
    let output = process::require("git", &["merge-base", target, "HEAD"], root)?;
    if output.stdout.is_empty() {
        return Err(Error::Command {
            program: "git merge-base".into(),
            stderr: format!("`{target}` and HEAD have no merge base"),
        });
    }
    Ok(output.stdout)
}

pub fn prior_content(root: &Path, path: &str) -> Option<String> {
    content_at(root, "HEAD", path)
}

pub fn parent_is_new(root: &Path, path: &str) -> bool {
    parent_is_new_at(root, "HEAD", path)
}

pub fn parent_is_new_at(root: &Path, revision: &str, path: &str) -> bool {
    let Some((parent, _)) = path.rsplit_once('/') else {
        return false;
    };
    let object = format!("{revision}:{parent}");
    process::run("git", &["cat-file", "-e", &object], root)
        .map(|output| !output.success)
        .unwrap_or(false)
}

fn parse_name_status(bytes: &[u8]) -> Result<Vec<ChangedPath>> {
    let records: Vec<&[u8]> = bytes
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .collect();
    let mut changes = BTreeMap::new();
    let mut index = 0;
    while index < records.len() {
        let status = String::from_utf8_lossy(records[index]);
        index += 1;
        let code = status.chars().next().unwrap_or('M');
        let kind = match code {
            'A' => ChangeKind::Added,
            'D' => ChangeKind::Deleted,
            'R' | 'C' => ChangeKind::Renamed,
            'U' => ChangeKind::Conflicted,
            _ => ChangeKind::Modified,
        };
        if matches!(code, 'R' | 'C') {
            // Rename/copy records contain old path then new path. Reach
            // evaluation is performed on the destination.
            index += 1;
        }
        let Some(path) = records.get(index) else {
            return Err(Error::Command {
                program: "git diff --name-status".into(),
                stderr: "malformed NUL-delimited name-status output".into(),
            });
        };
        index += 1;
        let path = String::from_utf8_lossy(path).replace('\\', "/");
        changes.insert(path.clone(), ChangedPath { path, kind });
    }
    Ok(changes.into_values().collect())
}

fn classify(x: char, y: char) -> ChangeKind {
    if x == '?' && y == '?' {
        ChangeKind::Untracked
    } else if x == 'U' || y == 'U' || (x == 'A' && y == 'A') || (x == 'D' && y == 'D') {
        ChangeKind::Conflicted
    } else if x == 'R' || y == 'R' || x == 'C' || y == 'C' {
        ChangeKind::Renamed
    } else if x == 'D' || y == 'D' {
        ChangeKind::Deleted
    } else if x == 'A' || y == 'A' {
        ChangeKind::Added
    } else {
        ChangeKind::Modified
    }
}

/// Construct a git command rooted in `cwd`, granting only explicitly declared checkout trust.
/// A nested repository has its own boundary; trusting a parent never trusts its submodules.
pub fn command(cwd: &Path) -> std::process::Command {
    let mut command = native_command(cwd);
    command.args(trust_args(cwd));
    command
}

/// The checkout holding `cwd`, when this device declared it trusted.
fn declared_root(cwd: &Path) -> Option<PathBuf> {
    let root = checkout(cwd)?;
    trust_file()
        .ok()
        .is_some_and(|file| declared_at(&root, &file))
        .then_some(root)
}

/// The arguments `command` adds, for a git spawned through another runner.
pub(crate) fn trust_args(cwd: &Path) -> Vec<String> {
    declared_root(cwd)
        .map(|root| {
            vec![
                "-c".to_owned(),
                format!("safe.directory={}", root.display()),
            ]
        })
        .unwrap_or_default()
}

/// The same trust as `GIT_CONFIG_*` variables, for a child that runs git itself (brit).
pub(crate) fn trust_env(cwd: &Path) -> Vec<(String, String)> {
    let existing = std::env::var("GIT_CONFIG_COUNT").ok();
    declared_root(cwd)
        .map(|root| config_env(existing.as_deref(), &root))
        .unwrap_or_default()
}

/// Append one `safe.directory` entry after any the caller already passes. A malformed count is
/// left for git to refuse, never repaired over the caller's own entries.
fn config_env(existing: Option<&str>, root: &Path) -> Vec<(String, String)> {
    let index = match existing.map(|count| count.trim().parse::<usize>()) {
        None => 0,
        Some(Ok(count)) => count,
        Some(Err(_)) => return Vec::new(),
    };
    let Some(count) = index.checked_add(1) else {
        return Vec::new();
    };
    vec![
        (
            format!("GIT_CONFIG_KEY_{index}"),
            "safe.directory".to_owned(),
        ),
        (
            format!("GIT_CONFIG_VALUE_{index}"),
            root.display().to_string(),
        ),
        ("GIT_CONFIG_COUNT".to_owned(), count.to_string()),
    ]
}

fn native_command(cwd: &Path) -> std::process::Command {
    let mut command = std::process::Command::new("git");
    command.current_dir(cwd);
    for key in crate::process::GIT_LOCATION_ENV {
        command.env_remove(key);
    }
    command
}

/// Locate a checkout without asking git to trust it first. Worktree `.git` files are boundaries.
pub(crate) fn checkout(start: &Path) -> Option<PathBuf> {
    let path = start.canonicalize().ok()?;
    path.ancestors()
        .find(|p| p.join(".git").exists())
        .map(Path::to_owned)
}

fn trust_file() -> std::io::Result<PathBuf> {
    let key = crate::device_key::resolve_path()?;
    key.parent()
        .map(|p| p.join("trusted-checkouts"))
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "device key has no directory",
            )
        })
}

fn declared_at(root: &Path, file: &Path) -> bool {
    std::fs::read_to_string(file)
        .ok()
        .is_some_and(|text| text.lines().any(|line| Path::new(line) == root))
}

/// Declare one canonical checkout beside the device key. No global git configuration changes.
pub fn trust(path: &Path) -> std::io::Result<PathBuf> {
    let root = checkout(path).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "path is not inside a checkout",
        )
    })?;
    declare_at(&root, &trust_file()?)?;
    Ok(root)
}

fn declare_at(root: &Path, file: &Path) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    let text = root
        .to_str()
        .filter(|s| !s.contains(['\n', '\r']))
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "checkout path must be UTF-8 without newlines",
            )
        })?;
    if let Some(parent) = file.parent() {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)?;
    }
    let mut output = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(file)?;
    rustix::fs::flock(&output, rustix::fs::FlockOperation::LockExclusive)?;
    if !declared_at(root, file) {
        writeln!(output, "{text}")?;
        output.sync_all()?;
    }
    Ok(())
}

/// Check native git acceptance or the exact device declaration, without minting trust.
pub(crate) fn accepted(root: &Path) -> std::io::Result<bool> {
    Ok(command(root)
        .args(["rev-parse", "--show-toplevel"])
        .output()?
        .status
        .success())
}

/// Supply the same git policy to bin-store's bounded subprocess runner.
pub(crate) struct DeviceRunner;
impl elohim_device_runtime::bin_store::Runner for DeviceRunner {
    fn run(
        &self,
        spec: &elohim_device_runtime::bin_store::CommandSpec,
    ) -> std::result::Result<
        elohim_device_runtime::bin_store::CommandOutput,
        elohim_device_runtime::bin_store::Refusal,
    > {
        use elohim_device_runtime::bin_store::SystemRunner;
        let mut spec = spec.clone();
        if spec.program == "git" {
            let cwd = spec.cwd.clone().unwrap_or_else(|| PathBuf::from("."));
            let command = command(&cwd);
            let mut args: Vec<_> = command
                .get_args()
                .map(|s| s.to_string_lossy().into_owned())
                .collect();
            args.append(&mut spec.args);
            spec.args = args;
            for (key, value) in command.get_envs() {
                spec.env.insert(
                    key.to_string_lossy().into_owned(),
                    value.map(|s| s.to_string_lossy().into_owned()),
                );
            }
        }
        SystemRunner.run(&spec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_porcelain_statuses() {
        assert_eq!(classify('?', '?'), ChangeKind::Untracked);
        assert_eq!(classify('A', ' '), ChangeKind::Added);
        assert_eq!(classify(' ', 'D'), ChangeKind::Deleted);
        assert_eq!(classify('R', ' '), ChangeKind::Renamed);
        assert_eq!(classify('M', 'M'), ChangeKind::Modified);
    }

    #[test]
    fn trust_env_appends_after_the_callers_config_entries() {
        let root = Path::new("/projects/elohim");
        let fresh = config_env(None, root);
        assert_eq!(
            fresh[0],
            ("GIT_CONFIG_KEY_0".into(), "safe.directory".into())
        );
        assert_eq!(
            fresh[1],
            ("GIT_CONFIG_VALUE_0".into(), "/projects/elohim".into())
        );
        assert_eq!(fresh[2], ("GIT_CONFIG_COUNT".into(), "1".into()));
        let after_two = config_env(Some("2"), root);
        assert_eq!(after_two[0].0, "GIT_CONFIG_KEY_2");
        assert_eq!(after_two[2], ("GIT_CONFIG_COUNT".into(), "3".into()));
        assert!(config_env(Some("bogus"), root).is_empty());
        assert!(config_env(Some(&usize::MAX.to_string()), root).is_empty());
    }

    #[test]
    fn parses_nul_delimited_name_status_and_rename_destination() {
        let changes =
            parse_name_status(b"A\0new.md\0M\0src/lib.rs\0R100\0old.md\0new-name.md\0").unwrap();
        assert_eq!(changes.len(), 3);
        assert_eq!(changes[0].path, "new-name.md");
        assert_eq!(changes[0].kind, ChangeKind::Renamed);
        assert_eq!(changes[1].path, "new.md");
        assert_eq!(changes[1].kind, ChangeKind::Added);
    }
}
