//! Repository-local habit declarations and covenant order. Git's index bounds the census;
//! gitlinks and nested checkouts are separate governance roots, never recursively imported.
use super::registers::HabitEntry;
use super::{FlowError, FlowResult};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn invalid(path: &Path, reason: &str) -> FlowError {
    FlowError::InvalidArguments(format!("{}: {reason}", path.display()))
}
fn frontmatter(path: &Path) -> FlowResult<String> {
    let text = std::fs::read_to_string(path).map_err(|source| FlowError::Read {
        path: path.into(),
        source,
    })?;
    let mut lines = text.lines();
    if lines.next() != Some("---") {
        return Err(invalid(path, "missing frontmatter"));
    }
    let mut body = Vec::new();
    for line in lines {
        if line == "---" {
            return Ok(body.join("\n"));
        }
        body.push(line);
    }
    Err(invalid(path, "unterminated frontmatter"))
}
fn keep(path: &Path) -> bool {
    path.parent()
        .and_then(Path::file_name)
        .is_some_and(|name| name == ".epr-meta")
        && path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().ends_with(".habit.md"))
        && !path.components().any(|part| {
            matches!(
                part.as_os_str().to_str(),
                Some("node_modules" | "fixtures" | "__tests__" | "worktrees" | "target")
            )
        })
}
fn walk(root: &Path, dir: &Path, paths: &mut BTreeSet<PathBuf>) -> FlowResult<()> {
    let entries = std::fs::read_dir(dir).map_err(|source| FlowError::Read {
        path: dir.into(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| FlowError::Read {
            path: dir.into(),
            source,
        })?;
        let kind = entry.file_type().map_err(|source| FlowError::Read {
            path: entry.path(),
            source,
        })?;
        let path = entry.path();
        if kind.is_dir() {
            if matches!(
                entry.file_name().to_str(),
                Some(".git" | "target" | "node_modules" | "fixtures" | "__tests__" | "worktrees")
            ) || path.join(".git").exists()
            {
                continue;
            }
            walk(root, &path, paths)?;
        } else if kind.is_file() && keep(&path) {
            paths.insert(
                path.strip_prefix(root)
                    .expect("walk remains below root")
                    .into(),
            );
        }
    }
    Ok(())
}
#[derive(Deserialize)]
struct Declaration {
    #[serde(rename = "epr-habit-version")]
    version: u32,
    invariant: String,
    #[serde(rename = "retire-when")]
    retire_when: String,
    #[serde(flatten)]
    habit: HabitEntry,
}
#[derive(Deserialize)]
struct Covenant {
    version: u32,
    #[serde(default)]
    order: Vec<String>,
}

pub(super) fn read(root: &Path) -> FlowResult<Option<Vec<HabitEntry>>> {
    let output = crate::process::build_command(
        "git",
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            "*.habit.md",
        ],
        root,
        &[],
    )
    .output();
    let mut paths = BTreeSet::new();
    match output {
        Ok(output) if output.status.success() => {
            for raw in output.stdout.split(|b| *b == 0).filter(|s| !s.is_empty()) {
                let rel = std::str::from_utf8(raw)
                    .map_err(|_| invalid(root, "habit declaration path is not UTF-8"))?;
                let path = PathBuf::from(rel);
                if keep(&path)
                    && std::fs::symlink_metadata(root.join(&path)).is_ok_and(|m| m.is_file())
                {
                    paths.insert(path);
                }
            }
        }
        _ => walk(root, root, &mut paths)?,
    }
    if paths.is_empty() {
        return Ok(None);
    }
    let covenant = root.join(".epr-meta/habits-covenant.md");
    let order = if covenant.exists() {
        let declaration: Covenant =
            serde_yaml::from_str(&frontmatter(&covenant)?).map_err(|source| FlowError::Yaml {
                path: covenant.clone(),
                source,
            })?;
        if declaration.version != 1 {
            return Err(invalid(&covenant, "unsupported covenant version"));
        }
        if declaration.order.iter().collect::<BTreeSet<_>>().len() != declaration.order.len() {
            return Err(invalid(&covenant, "duplicate habit priority"));
        }
        declaration.order
    } else {
        Vec::new()
    };
    let mut ids = BTreeSet::new();
    let mut habits = Vec::new();
    for rel in paths {
        let path = root.join(&rel);
        let declaration: Declaration =
            serde_yaml::from_str(&frontmatter(&path)?).map_err(|source| FlowError::Yaml {
                path: path.clone(),
                source,
            })?;
        let mut habit = declaration.habit;
        if declaration.version != 1
            || declaration.invariant.trim().is_empty()
            || declaration.retire_when.trim().is_empty()
            || declaration.retire_when.trim().eq_ignore_ascii_case("never")
            || path.file_name().unwrap() != format!("{}.habit.md", habit.id).as_str()
            || !matches!(habit.status.as_str(), "green" | "red" | "unwired")
            || (habit.status == "unwired" && !habit.checks.is_empty())
            || (habit.status != "unwired"
                && (habit.checks.is_empty() || habit.checks.iter().any(|s| s.trim().is_empty())))
        {
            return Err(invalid(
                &path,
                "invalid habit version, identity, invariant, status/checks or retirement",
            ));
        }
        if !ids.insert(habit.id.clone()) {
            return Err(invalid(&path, "duplicate habit id"));
        }
        habit.source = Some(rel.to_string_lossy().replace('\\', "/"));
        habit.priority = order.iter().position(|id| *id == habit.id);
        habits.push(habit);
    }
    habits.sort_by(|a, b| {
        a.priority
            .unwrap_or(usize::MAX)
            .cmp(&b.priority.unwrap_or(usize::MAX))
            .then_with(|| a.id.cmp(&b.id))
    });
    Ok(Some(habits))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn atom(root: &Path, dir: &str, id: &str) {
        let path = root.join(dir).join(".epr-meta");
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join(format!("{id}.habit.md")), format!("---\nepr-habit-version: 1\nid: {id}\ninvariant: Works repeatedly\nstatus: red\nchecks: [just test]\nretire-when: Superseded by stronger invariant\n---\n")).unwrap();
    }
    #[test]
    fn local_atoms_override_stale_projection_and_follow_covenant() {
        let root = tempfile::tempdir().unwrap();
        atom(root.path(), "", "alpha");
        atom(root.path(), "engine", "beta");
        std::fs::write(
            root.path().join(".epr-meta/habits-covenant.md"),
            "---\nversion: 1\norder: [beta]\n---\n",
        )
        .unwrap();
        let entries = read(root.path()).unwrap().unwrap();
        assert_eq!(
            entries.iter().map(|h| h.id.as_str()).collect::<Vec<_>>(),
            ["beta", "alpha"]
        );
        assert_eq!(entries[0].priority, Some(0));
        assert_eq!(entries[1].priority, None);
        assert_eq!(
            entries[0].source.as_deref(),
            Some("engine/.epr-meta/beta.habit.md")
        );
    }
    #[test]
    fn nested_checkout_and_fixtures_do_not_join_parent_census() {
        let root = tempfile::tempdir().unwrap();
        atom(root.path(), "", "ours");
        atom(root.path(), "child", "theirs");
        atom(root.path(), "fixtures", "fixture");
        std::fs::write(root.path().join("child/.git"), "gitdir: elsewhere").unwrap();
        assert_eq!(read(root.path()).unwrap().unwrap().len(), 1);
    }
    #[test]
    fn malformed_atoms_and_covenants_are_errors_never_empty_success() {
        let root = tempfile::tempdir().unwrap();
        atom(root.path(), "", "ours");
        std::fs::write(
            root.path().join(".epr-meta/habits-covenant.md"),
            "---\nversion: 1\norder: [ours, ours]\n---\n",
        )
        .unwrap();
        assert!(read(root.path()).is_err());
        std::fs::remove_file(root.path().join(".epr-meta/habits-covenant.md")).unwrap();
        std::fs::write(
            root.path().join(".epr-meta/ours.habit.md"),
            "---\nid: ours\n---\n",
        )
        .unwrap();
        assert!(read(root.path()).is_err());
    }
    #[test]
    fn isolated_repositories_read_only_their_own_atoms() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        atom(a.path(), "", "a");
        atom(b.path(), "", "b");
        assert_eq!(read(a.path()).unwrap().unwrap()[0].id, "a");
        assert_eq!(read(b.path()).unwrap().unwrap()[0].id, "b");
    }
}
