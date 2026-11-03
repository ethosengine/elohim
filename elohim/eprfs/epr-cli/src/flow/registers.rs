//! Read repository-local habit declarations and declared gate projects.
//! Habit atoms are authoritative; the legacy generated register is read only when a
//! tree carries no atoms, preserving historical fixture/archive compatibility.
//! Neither reader writes declarations or their projections.

use std::path::{Path, PathBuf};

pub use super::habit_census::parse_habit;
use serde::{Deserialize, Serialize};

use super::{FlowError, FlowResult};

/// The generated habit register, relative to the repository root.
pub const HABITS_REGISTER_REL: &str = "genesis/manifests/habits.yaml";

/// The per-project pipeline manifest whose `gate.projects` says how a tree is gated.
pub const BUILD_MANIFEST_NAME: &str = "build-manifest.json";

#[derive(Debug, Deserialize)]
struct HabitRegister {
    #[serde(default)]
    habits: Vec<HabitEntry>,
}

/// One habit as the register projects it. A habit is a SCOPE — the standard a piece of work is
/// accounted to — so the fields read here are the ones that say what the standard is
/// (`invariant`, `checks`), where it stands (`status`, `active`), and what names it (`refs`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HabitEntry {
    #[serde(default, rename = "_source")]
    pub source: Option<String>,
    #[serde(default, skip_deserializing)]
    pub priority: Option<usize>,
    pub id: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub checks: Vec<String>,
    #[serde(default)]
    pub refs: Vec<String>,
}

/// Absolute path of the habit register under `root`.
pub fn habits_register_path(root: &Path) -> PathBuf {
    root.join(HABITS_REGISTER_REL)
}

/// Every habit the register declares.
///
/// A missing or unparseable register is an ERROR naming the file rather than an empty list: a
/// caller asking "is `x` a declared habit" must never be told "no" because the register could
/// not be read. That is the difference between an absent habit and an absent register.
pub fn read_habits(root: &Path) -> FlowResult<Vec<HabitEntry>> {
    if let Some(habits) = super::habit_census::read(root)? {
        return Ok(habits);
    }
    if root.join(super::repository::DECLARATION).exists() {
        return Err(FlowError::InvalidArguments(
            "no .epr-meta/*.habit.md declarations in this repository; a stale genesis/manifests/habits.yaml projection cannot establish habits".into(),
        ));
    }
    let path = habits_register_path(root);
    let text = std::fs::read_to_string(&path).map_err(|source| FlowError::Read {
        path: path.clone(),
        source,
    })?;
    let register: HabitRegister =
        serde_yaml::from_str(&text).map_err(|source| FlowError::Yaml { path, source })?;
    Ok(register.habits)
}

/// Claim-time evidence only: qualified habits never join the parent's habit census.
/// The returned slots supplement an existing commitment, not its authority or scope.
pub(super) fn claim_habit(root: &Path, selector: &str) -> FlowResult<Vec<String>> {
    let Some((owner, id)) = selector.split_once('#') else {
        if !read_habits(root)?.iter().any(|habit| habit.id == selector) {
            return Err(FlowError::InvalidArguments(format!(
                "unknown habit `{selector}` — this repository declares no such habit; \
                 a habit is DECLARED in its `.epr-meta` atom, never invented at claim time"
            )));
        }
        return Ok(Vec::new());
    };
    qualified_claim_habit(root, selector, owner, id)
}

#[cfg(not(unix))]
fn qualified_claim_habit(
    _root: &Path,
    _selector: &str,
    _owner: &str,
    _id: &str,
) -> FlowResult<Vec<String>> {
    Err(FlowError::InvalidArguments(
        "qualified habit confinement requires Unix".into(),
    ))
}

#[cfg(unix)]
fn qualified_claim_habit(
    root: &Path,
    selector: &str,
    owner: &str,
    id: &str,
) -> FlowResult<Vec<String>> {
    let invalid = |reason: &str| {
        FlowError::InvalidArguments(format!("qualified habit `{selector}`: {reason}"))
    };
    if selector.len() > 512
        || !plain_relative(owner)
        || id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        || matches!(id, "." | "..")
    {
        return Err(invalid("expected canonical gitlink-path#habit-id"));
    }
    let head = habit_git(root, &["rev-parse", "--verify", "HEAD"], 1024)?;
    let head = head.trim();
    let link = habit_git(root, &["ls-tree", "-z", head, "--", owner], 4096)?;
    let (mode, pin, name) =
        tree_entry(&link).ok_or_else(|| invalid("owner is not one committed gitlink"))?;
    if mode != "160000" || name != owner {
        return Err(invalid("owner is not one committed gitlink"));
    }
    let indexed = habit_git(root, &["ls-files", "--stage", "-z", "--", owner], 4096)?;
    if indexed != format!("160000 {pin} 0\t{owner}\0") {
        return Err(invalid("owner gitlink index differs from committed pin"));
    }
    // Refuse symlinked ancestors before asking Git to enter the explicitly named owner.
    let owner_dir = confined_directory(root, owner)?;
    let owner_root = root.join(owner);
    let current = habit_git(
        &owner_root,
        &["rev-parse", "--verify", "HEAD^{commit}"],
        1024,
    )?;
    if current.trim() != pin {
        return Err(invalid("owner HEAD differs from committed gitlink pin"));
    }
    let pattern = format!(":(glob)**/.epr-meta/{id}.habit.md");
    // Index queries are filtered by the exact filename. First prove those entries and
    // repository attribution still match the pinned tree; never scan a whole repository.
    let changed = habit_git(
        &owner_root,
        &[
            "diff",
            "--cached",
            "--name-only",
            "-z",
            pin,
            "--",
            super::repository::DECLARATION,
            &pattern,
        ],
        65536,
    )?;
    if changed
        .split('\0')
        .filter(|path| !path.is_empty())
        .any(|path| {
            path == super::repository::DECLARATION || super::habit_census::keep(Path::new(path))
        })
    {
        return Err(invalid("owner declaration index differs from pinned tree"));
    }
    let candidates = habit_git(
        &owner_root,
        &["ls-files", "-z", "--cached", "--", &pattern],
        65536,
    )?;
    let paths: Vec<&str> = candidates
        .split('\0')
        .filter(|p| !p.is_empty() && super::habit_census::keep(Path::new(p)))
        .take(2)
        .collect();
    if paths.len() != 1 {
        return Err(invalid(
            "habit declaration missing or ambiguous in pinned owner",
        ));
    }
    let path = paths[0];
    if !plain_relative(path) {
        return Err(invalid("habit declaration path is unsupported"));
    }
    let declaration = pinned_habit_text(&owner_root, &owner_dir, pin, path)?;
    let habit = parse_habit(Path::new(path), &declaration)?;
    if habit.id != id {
        return Err(invalid("habit declaration identity differs from selector"));
    }
    let owner_text =
        pinned_habit_text(&owner_root, &owner_dir, pin, super::repository::DECLARATION)?;
    let agent = super::repository::parse_repository_agent(
        Path::new(super::repository::DECLARATION),
        &owner_text,
    )?;
    Ok(vec![
        format!("habit-owner:{}", agent.0),
        format!("habit-owner-path:{owner}"),
        format!("habit-owner-commit:{pin}"),
        format!("habit-parent-commit:{head}"),
        format!("habit-declaration-path:{path}"),
        format!(
            "habit-declaration:{}",
            eprfs_core::BlobCid::compute_raw(declaration.as_bytes())
        ),
        format!(
            "habit-owner-declaration:{}",
            eprfs_core::BlobCid::compute_raw(owner_text.as_bytes())
        ),
    ])
}

#[cfg(unix)]
fn plain_relative(path: &str) -> bool {
    !path.is_empty()
        && path.split('/').all(|part| {
            !part.is_empty()
                && !matches!(part, "." | "..")
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
}

#[cfg(unix)]
fn habit_git(root: &Path, args: &[&str], cap: usize) -> FlowResult<String> {
    let mut argv = crate::git::trust_args(root);
    argv.extend([
        "--no-replace-objects".into(),
        "-c".into(),
        "protocol.allow=never".into(),
        "-c".into(),
        "core.fsmonitor=false".into(),
        "-C".into(),
        root.to_str()
            .ok_or_else(|| FlowError::InvalidArguments("habit owner path is not UTF-8".into()))?
            .into(),
    ]);
    argv.extend(args.iter().map(|arg| (*arg).into()));
    let out = super::memory::recall::bounded_process("git", &argv, cap, 10.0, None)?;
    if out.error.is_some() || out.status != Some(0) {
        return Err(FlowError::InvalidArguments(format!(
            "habit owner Git evidence unavailable: {} {}",
            out.error.unwrap_or_default(),
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    String::from_utf8(out.stdout)
        .map_err(|_| FlowError::InvalidArguments("habit owner Git evidence is not UTF-8".into()))
}

#[cfg(unix)]
fn tree_entry(row: &str) -> Option<(&str, &str, &str)> {
    let row = row.strip_suffix('\0')?;
    if row.contains('\0') {
        return None;
    }
    let (fields, name) = row.split_once('\t')?;
    let mut fields = fields.split(' ');
    let mode = fields.next()?;
    let _kind = fields.next()?;
    let oid = fields.next()?;
    (fields.next().is_none()).then_some((mode, oid, name))
}

#[cfg(unix)]
fn confined_directory(root: &Path, rel: &str) -> FlowResult<std::os::fd::OwnedFd> {
    use rustix::fs::{open, openat, Mode, OFlags};
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let mut dir = open(root, flags, Mode::empty()).map_err(std::io::Error::from)?;
    for component in rel.split('/') {
        dir = openat(&dir, component, flags, Mode::empty()).map_err(std::io::Error::from)?;
    }
    Ok(dir)
}

#[cfg(unix)]
fn pinned_habit_text(
    root: &Path,
    owner: &std::os::fd::OwnedFd,
    pin: &str,
    rel: &str,
) -> FlowResult<String> {
    use rustix::fs::{openat, Mode, OFlags};
    use std::io::Read;
    let invalid =
        |reason: &str| FlowError::InvalidArguments(format!("habit owner `{rel}`: {reason}"));
    let entry = habit_git(root, &["ls-tree", "-z", pin, "--", rel], 4096)?;
    let (mode, oid, name) =
        tree_entry(&entry).ok_or_else(|| invalid("missing pinned declaration"))?;
    if !matches!(mode, "100644" | "100755") || name != rel {
        return Err(invalid("pinned declaration must be a regular file"));
    }
    let text = habit_git(root, &["cat-file", "blob", oid], 65536)?;
    let mut components = rel.split('/').peekable();
    let mut directory = rustix::io::dup(owner).map_err(std::io::Error::from)?;
    while let Some(component) = components.next() {
        let mut flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK;
        if components.peek().is_some() {
            flags |= OFlags::DIRECTORY;
        }
        let next =
            openat(&directory, component, flags, Mode::empty()).map_err(std::io::Error::from)?;
        if components.peek().is_none() {
            let file = std::fs::File::from(next);
            if !file.metadata()?.is_file() {
                return Err(invalid("working declaration is not a regular file"));
            }
            let mut bytes = Vec::new();
            file.take(65537).read_to_end(&mut bytes)?;
            if bytes != text.as_bytes() {
                return Err(invalid("working declaration differs from pinned bytes"));
            }
            break;
        }
        directory = next;
    }
    Ok(text)
}

/// The habits whose `checks:` or `refs:` mention `rel_path` — the register's own statement that
/// this file's behaviour is covered by that standard.
///
/// Substring matching, deliberately: a check is a runnable command line and a ref is a prose
/// locator, so neither is a structured path field that could be compared exactly.
pub fn habits_covering(habits: &[HabitEntry], rel_path: &str) -> Vec<HabitEntry> {
    if rel_path.is_empty() {
        return Vec::new();
    }
    habits
        .iter()
        .filter(|habit| {
            habit
                .checks
                .iter()
                .chain(habit.refs.iter())
                .any(|line| line.contains(rel_path))
        })
        .cloned()
        .collect()
}

/// A `gate.projects` entry: what `just gate <name>` actually runs for a tree.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GateCargo {
    #[serde(default)]
    pub target_dir: Option<String>,
    #[serde(default)]
    pub rustflags: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct GateRun {
    #[serde(default)]
    cargo: Option<GateCargo>,
}

#[derive(Debug, Clone, Deserialize)]
struct GateProject {
    #[serde(default)]
    dir: String,
    #[serde(default)]
    run: Option<GateRun>,
}

#[derive(Debug, Deserialize)]
struct GateSection {
    #[serde(default)]
    projects: std::collections::BTreeMap<String, GateProject>,
}

#[derive(Debug, Deserialize)]
struct BuildManifest {
    #[serde(default)]
    gate: Option<GateSection>,
}

/// The gate that owns a path: project name, the command to run it, and the cargo environment
/// the manifest declares — or, when more than one entry covers the path equally well, the tie
/// itself.
///
/// `project` and `command` are `Option` for exactly one reason: a tie has no single answer, and
/// a scalar that had to hold one anyway would hold a guess. A reader that prints `command`
/// unconditionally would print a gate that does not gate the file.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct GateView {
    /// The one project that gates this path. `None` when [`Self::ambiguous`] is non-empty.
    pub project: Option<String>,
    /// `just gate <project>`. `None` when the match is ambiguous.
    pub command: Option<String>,
    pub target_dir: Option<String>,
    pub rustflags: Option<String>,
    /// The manifest this was read from, repo-relative — so a reader can check the claim.
    pub manifest: String,
    /// Every project that tied, sorted. Empty on an unambiguous match.
    pub ambiguous: Vec<String>,
}

/// Walk up from `rel_path`'s directory to the nearest `build-manifest.json`, then select the
/// `gate.projects` entry whose `dir` covers `rel_path`.
///
/// `None` is an honest "no gate project covers this path". The alternative — returning the
/// nearest project regardless of its `dir` — would print a gate command that does not gate the
/// file, which is worse than printing nothing, because it would be followed.
pub fn gate_for_path(root: &Path, rel_path: &str) -> Option<GateView> {
    let mut dir = PathBuf::from(rel_path);
    // A path names a file; start the walk at its directory. An empty relative path means the
    // repository root itself, where the walk is a single look.
    if !dir.pop() {
        dir = PathBuf::new();
    }
    loop {
        let manifest_rel = dir.join(BUILD_MANIFEST_NAME);
        let manifest_abs = root.join(&manifest_rel);
        if manifest_abs.is_file() {
            if let Some(view) = gate_in_manifest(&manifest_abs, &manifest_rel, rel_path) {
                return Some(view);
            }
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Select within one manifest, in two tiers.
///
/// **Tier 1, the projects that actually name a directory containing the path**, longest `dir`
/// first: a nested project is more specific than the tree that contains it, and the more
/// specific gate is the one that runs the file.
///
/// **Tier 2, the root-scoped projects (`dir` of `.` or empty), as a LAST RESORT.** They cover
/// everything, which means they discriminate nothing, so they must never compete with a project
/// that names the tree. `genesis/build-manifest.json` declares five of them; treating `.` as an
/// ordinary match made all five tie at dir length 1, and a stable sort over a `BTreeMap` then
/// returned the alphabetically first — `constants-sync` for every file under `genesis/docs`.
/// That is a gate that does not gate the file, printed with the authority of a measurement.
///
/// **A tie within the chosen tier is REPORTED, never resolved.** Two projects that cover the
/// path equally well are two honest answers, and picking one by map order is the guess spec §6.7
/// forbids.
fn gate_in_manifest(manifest_abs: &Path, manifest_rel: &Path, rel_path: &str) -> Option<GateView> {
    let text = std::fs::read_to_string(manifest_abs).ok()?;
    let manifest: BuildManifest = serde_json::from_str(&text).ok()?;
    let projects = manifest.gate?.projects;
    let manifest = manifest_rel.to_string_lossy().replace('\\', "/");

    let mut candidates: Vec<(&String, &GateProject)> = projects
        .iter()
        .filter(|(_, project)| path_is_under(rel_path, &project.dir))
        .collect();
    if candidates.is_empty() {
        // Tier 2. Reached only because nothing specific covers the path.
        candidates = projects
            .iter()
            .filter(|(_, project)| normalize_dir(&project.dir).is_empty())
            .collect();
    }
    let longest = candidates
        .iter()
        .map(|(_, project)| normalize_dir(&project.dir).len())
        .max()?;
    let mut tied: Vec<(&String, &GateProject)> = candidates
        .into_iter()
        .filter(|(_, project)| normalize_dir(&project.dir).len() == longest)
        .collect();
    tied.sort_by_key(|(name, _)| (*name).clone());

    if tied.len() > 1 {
        return Some(GateView {
            project: None,
            command: None,
            target_dir: None,
            rustflags: None,
            manifest,
            ambiguous: tied.into_iter().map(|(name, _)| name.clone()).collect(),
        });
    }

    let (name, project) = tied.first()?;
    let cargo = project.run.as_ref().and_then(|run| run.cargo.clone());
    Some(GateView {
        project: Some((*name).clone()),
        command: Some(format!("just gate {name}")),
        target_dir: cargo.as_ref().and_then(|c| c.target_dir.clone()),
        rustflags: cargo.as_ref().and_then(|c| c.rustflags.clone()),
        manifest,
        ambiguous: Vec::new(),
    })
}

/// A declared `dir`, with its trailing slash and its `.` spelling of "the repository root"
/// normalized away, so the two spellings of root cannot be ranked against each other by length.
fn normalize_dir(dir: &str) -> &str {
    let dir = dir.trim_end_matches('/');
    if dir == "." {
        ""
    } else {
        dir
    }
}

/// Is `rel_path` inside the declared `dir`? The match is on a path BOUNDARY, never a bare
/// prefix, so `elohim/eprfs` never claims `elohim/eprfs-extra`.
///
/// A root-scoped `dir` is deliberately NOT a match here. It covers every path, so it can never
/// tell one path from another, and letting it answer would let it outrank — by nothing but map
/// order — the project that actually owns the tree. Root-scoped entries are the caller's
/// last-resort tier instead.
fn path_is_under(rel_path: &str, dir: &str) -> bool {
    let dir = normalize_dir(dir);
    if dir.is_empty() {
        return false;
    }
    rel_path == dir || rel_path.starts_with(&format!("{dir}/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write(root: &Path, rel: &str, contents: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    /// A fixture register and a fixture manifest, both under a temp dir. The live repository is
    /// never read: a test that asserted against the real register would fail the day someone
    /// legitimately re-projected it.
    fn fixture() -> TempDir {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write(
            root,
            HABITS_REGISTER_REL,
            r#"version: 1
habits:
  - id: dev-system-equilibrium
    status: red
    active: false
    checks:
      - "epr flow stocks --window <START..END> --per week --stock commitments --check"
    refs:
      - "elohim/eprfs/epr-cli/src/flow/stocks.rs"
  - id: epr-atom-home
    status: green
    active: true
    checks:
      - "a2o @concern:epr-atom-home"
    refs: []
"#,
        );
        write(
            root,
            "elohim/eprfs/build-manifest.json",
            r#"{
  "manifestVersion": "1.0",
  "gate": {
    "projects": {
      "eprfs": {
        "dir": "elohim/eprfs",
        "run": {
          "kind": "root-just",
          "cargo": { "targetDir": "/tmp/eprfs-gate-target", "rustflags": "" }
        }
      }
    }
  }
}
"#,
        );
        dir
    }

    #[test]
    fn the_register_reads_as_data_and_an_unknown_id_is_simply_absent() {
        let dir = fixture();
        let habits = read_habits(dir.path()).expect("register reads");
        let ids: Vec<&str> = habits.iter().map(|h| h.id.as_str()).collect();
        assert_eq!(ids, vec!["dev-system-equilibrium", "epr-atom-home"]);
        assert_eq!(habits[0].status, "red");
        assert!(!habits[0].active);
        assert!(habits[1].active);
        assert!(!ids.contains(&"not-a-habit"));
    }

    #[test]
    fn a_missing_register_is_an_error_that_names_the_file_never_an_empty_list() {
        let dir = TempDir::new().unwrap();
        let err = read_habits(dir.path()).expect_err("an unreadable register is not an empty one");
        assert!(
            err.to_string().contains(HABITS_REGISTER_REL),
            "the refusal must name the register file; got: {err}"
        );
    }

    #[test]
    fn a_declared_repository_cannot_resurrect_a_deleted_habit_from_its_projection() {
        let dir = fixture();
        write(
            dir.path(),
            super::super::repository::DECLARATION,
            "version: 1\nagent: repo:fixture/modern\n",
        );
        assert!(read_habits(dir.path())
            .unwrap_err()
            .to_string()
            .contains("stale"));
    }

    #[test]
    fn covering_habits_are_the_ones_whose_checks_or_refs_name_the_path() {
        let dir = fixture();
        let habits = read_habits(dir.path()).expect("register reads");
        let covering = habits_covering(&habits, "elohim/eprfs/epr-cli/src/flow/stocks.rs");
        assert_eq!(covering.len(), 1);
        assert_eq!(covering[0].id, "dev-system-equilibrium");
        assert!(habits_covering(&habits, "app/elohim-app/src/main.ts").is_empty());
        assert!(habits_covering(&habits, "").is_empty());
    }

    /// A manifest carrying TWO root-scoped (`.`) projects and one real directory — the shape
    /// `genesis/build-manifest.json` actually has, where five entries declare `.`.
    fn tie_fixture() -> TempDir {
        let dir = TempDir::new().unwrap();
        write(
            dir.path(),
            "genesis/build-manifest.json",
            r#"{
  "gate": {
    "projects": {
      "schema-validate": { "dir": "." },
      "constants-sync":  { "dir": "." },
      "genesis-a2o":     { "dir": "genesis/a2o" }
    }
  }
}
"#,
        );
        dir
    }

    #[test]
    fn a_real_dir_beats_every_root_scoped_entry_rather_than_tying_with_them() {
        let dir = tie_fixture();
        let gate = gate_for_path(dir.path(), "genesis/a2o/features/x.feature")
            .expect("the specific project owns the path");
        assert_eq!(gate.project.as_deref(), Some("genesis-a2o"));
        assert!(gate.ambiguous.is_empty());
    }

    /// The defect this test exists for: `.` used to match every path, so all the root-scoped
    /// entries tied at dir length 1 and BTreeMap order silently returned the alphabetically
    /// first — `constants-sync` for any `genesis/docs` file, a gate that does not gate it.
    #[test]
    fn tied_root_scoped_projects_are_reported_as_ambiguous_never_picked_by_map_order() {
        let dir = tie_fixture();
        let gate = gate_for_path(dir.path(), "genesis/docs/superpowers/plans/x.md")
            .expect("the tie is reported, not resolved");
        assert!(
            gate.project.is_none() && gate.command.is_none(),
            "a tie names no single project; got {:?}",
            gate.project
        );
        assert_eq!(
            gate.ambiguous,
            vec!["constants-sync".to_string(), "schema-validate".to_string()],
            "every tied candidate is named, sorted, and none is chosen"
        );
    }

    #[test]
    fn a_lone_root_scoped_project_still_resolves_as_the_last_resort() {
        let dir = TempDir::new().unwrap();
        write(
            dir.path(),
            "build-manifest.json",
            r#"{ "gate": { "projects": { "only": { "dir": "." } } } }"#,
        );
        let gate = gate_for_path(dir.path(), "anywhere/at/all.md").expect("last resort matches");
        assert_eq!(gate.project.as_deref(), Some("only"));
        assert!(gate.ambiguous.is_empty());
    }

    #[test]
    fn the_gate_is_the_nearest_manifest_entry_whose_dir_covers_the_path() {
        let dir = fixture();
        let gate = gate_for_path(dir.path(), "elohim/eprfs/epr-cli/src/flow/claim.rs")
            .expect("the eprfs gate covers this path");
        assert_eq!(gate.project.as_deref(), Some("eprfs"));
        assert_eq!(gate.command.as_deref(), Some("just gate eprfs"));
        assert_eq!(gate.target_dir.as_deref(), Some("/tmp/eprfs-gate-target"));
        assert_eq!(gate.rustflags.as_deref(), Some(""));
        assert_eq!(gate.manifest, "elohim/eprfs/build-manifest.json");
    }

    #[test]
    fn a_path_no_gate_project_declares_returns_none_rather_than_a_guess() {
        let dir = fixture();
        assert!(
            gate_for_path(dir.path(), "app/elohim-app/src/main.ts").is_none(),
            "a gate command that does not gate the file is worse than none"
        );
        // A sibling directory that merely shares a name PREFIX is not under the project.
        assert!(!path_is_under("elohim/eprfs-extra/x.rs", "elohim/eprfs"));
        assert!(path_is_under("elohim/eprfs/x.rs", "elohim/eprfs"));
        // A root-scoped dir discriminates nothing, so it is NOT a match here — it is a
        // last-resort group the selector reaches only when no specific project covers the path.
        assert!(!path_is_under("anything", ""));
        assert!(!path_is_under("anything", "."));
    }
}
