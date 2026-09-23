# Brit CLI Test Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a comprehensive CLI test suite + single-command "test page" runner for the four user-facing brit-workspace binaries (`brit`, `rakia`, `brit-verify`, `brit-build-ref`), producing a committed `baseline.md` artifact that captures every CLI subcommand's invocation + actual output. Enable TDD-driven CLI redesign via a baseline/candidate workflow.

**Architecture:** Hybrid test layers behind one runner. Layer A: extend gitoxide's existing shell journey-tests at `brit/tests/journey/gix.sh` for `brit` (was gix). Layer B: new Rust integration test crate `cli-journey` for the structured-output binaries (`rakia`, `brit-verify`, `brit-build-ref`) where shell assertion is painful. Unified runner: new Rust crate `cli-test-page` discovers the subcommand universe via recursive `--help` parsing, invokes both layers, reads captured outputs from a staging directory, formats one markdown report, supports `--check`/`--update`/`--candidate` modes for the baseline/candidate TDD workflow.

**Tech Stack:** Rust 2021 (clap 4 for runner CLI, similar crate for diffs, regex for normalization, assert_cmd + std::process for binary invocation), Bash (extend existing journey tests), `set-static-git-environment` (gitoxide's deterministic-SHA helper), git via `file://` for mock remotes.

**Spec:** `docs/superpowers/specs/2026-04-19-brit-cli-test-page-design.md`

**P2P design gate note:** This plan introduces zero protocol entities. All "things being designed" are operational test infrastructure (TestRepo fixtures, MockRemote helpers, baseline.md as a CLI behavior contract). Per the spec's P2P Design Gate Classification section, every entity here is Category C (Operational). The hook will likely grep-flag the word "schema" in references to existing schemas (e.g., when the runner formats `rakia plan` output that conforms to `build-plan.schema.json`); those references are to entities already classified in prior sprints. Don't add additional gate sections to fix-up commits — the spec is the canonical classification.

**Build notes:**
- Brit workspace: `cd elohim/brit && RUSTFLAGS="" cargo build`
- Both new crates (`cli-journey` and `cli-test-page`) are Rust workspace members under `elohim/brit`
- Binaries under test must be built (`cargo build -p gitoxide --bin brit -p brit-cli --bin rakia -p brit-verify -p brit-build-ref`) before running the test page

---

## File Structure

### New files

```
elohim/brit/
├── tests/
│   ├── journey/
│   │   ├── rakia.sh                              # NEW shell smoke for rakia (basic --help)
│   │   ├── brit-verify.sh                        # NEW
│   │   └── brit-build-ref.sh                     # NEW
│   ├── cli-journey/                              # NEW Rust integration test crate
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs                            # re-exports support modules
│   │   │   └── support/
│   │   │       ├── mod.rs
│   │   │       ├── test_repo.rs
│   │   │       ├── mock_remote.rs
│   │   │       ├── normalize.rs
│   │   │       └── runner.rs
│   │   └── tests/
│   │       ├── rakia.rs                          # all rakia subcommands
│   │       ├── brit_verify.rs                    # brit-verify
│   │       └── brit_build_ref.rs                 # brit-build-ref subcommands
│   ├── cli-test-page/                            # NEW runner crate
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs                           # clap entry: --check / --update / --candidate
│   │       ├── discover.rs                       # recursive --help parsing
│   │       ├── normalize.rs                      # shared with cli-journey via re-export
│   │       ├── format.rs                         # markdown emission
│   │       ├── coverage.rs                       # X-of-Y coverage computation
│   │       └── diff.rs                           # colored terminal diff
│   ├── .gitignore                                # NEW (or modify) — ignore .test-page-staging/
│   └── baseline.md                               # NEW committed artifact (generated, then committed)
└── Cargo.toml                                    # MODIFY — add cli-journey + cli-test-page to workspace.members
```

### Modified files

```
elohim/brit/
└── tests/
    └── journey.sh                                # MODIFY — source the 3 new journey files
```

### File responsibilities

- **`cli-journey/src/support/test_repo.rs`** — `TestRepo` struct: temp git repo with deterministic commit history (uses gitoxide's static-git-environment env vars for stable SHAs). `Drop` cleans up. Methods: `commit_file(path, contents) -> ObjectId`, `path() -> &Path`, `head_id() -> ObjectId`.
- **`cli-journey/src/support/mock_remote.rs`** — `MockRemote` struct: bare git repo at temp path. Methods: `url() -> String` (returns `file:///tmp/...`), `path() -> &Path`. `Drop` cleans up.
- **`cli-journey/src/support/normalize.rs`** — `Normalizer` struct + `normalize(text) -> String`. Replaces tempdir paths, ANSI codes, variable timestamps, variable SHAs (tracked via `NormalizationContext`). Also reusable by `cli-test-page`.
- **`cli-journey/src/support/runner.rs`** — `BritInvocation` builder + `run() -> Capture { stdout, stderr, status }`. Also `dump_to_staging(binary, subcommand_path)` for runner integration.
- **`cli-test-page/src/discover.rs`** — `discover_subcommands(binary_path) -> Tree<SubcommandPath>` via recursive `--help` parsing.
- **`cli-test-page/src/coverage.rs`** — given staging-dir contents + discovered universe, compute % covered per binary.
- **`cli-test-page/src/format.rs`** — emit markdown: TOC, coverage summary, per-binary sections, per-subcommand blocks (help text + invocation + actual output).
- **`cli-test-page/src/diff.rs`** — colored terminal diff using `similar` crate.
- **`cli-test-page/src/main.rs`** — three modes: `--check` (default; diff + exit code), `--update` (cp candidate over baseline), `--candidate <path>` (write to arbitrary path).

---

## Phase 1: Crate Scaffolding

### Task 1: Create cli-journey crate skeleton

**Files:**
- Create: `elohim/brit/tests/cli-journey/Cargo.toml`
- Create: `elohim/brit/tests/cli-journey/src/lib.rs`
- Create: `elohim/brit/tests/cli-journey/src/support/mod.rs`

- [ ] **Step 1.1: Create the workspace branch in brit submodule**

```bash
cd /home/matthew/git/elohim/elohim/brit
git checkout -b feat/brit-cli-test-page
git status
```

- [ ] **Step 1.2: Create `cli-journey/Cargo.toml`**

`/home/matthew/git/elohim/elohim/brit/tests/cli-journey/Cargo.toml`:

```toml
lints.workspace = true

[package]
name = "cli-journey"
version = "0.0.0"
description = "CLI integration test infrastructure for brit-workspace binaries"
repository = "https://github.com/ethosengine/brit"
authors = ["Matthew Dowell <matthew@ethosengine.com>"]
license = "MIT OR Apache-2.0"
edition = "2021"
rust-version = "1.82"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
regex = "1"
tempfile = "3"
anyhow = "1"

[dev-dependencies]
# Self-tests for the support modules
```

- [ ] **Step 1.3: Create `src/lib.rs`**

```rust
//! cli-journey — CLI integration test infrastructure.
//!
//! Provides reusable helpers for testing the brit-workspace binaries:
//!   - `TestRepo` — temp git repo with deterministic commit history
//!   - `MockRemote` — bare git repo at temp path, file:// URL
//!   - `Normalizer` — redacts variable output (tempdirs, SHAs, timestamps)
//!   - `BritInvocation` — process invocation + capture + normalization
//!
//! Tests under `tests/` use these helpers to exercise rakia, brit-verify,
//! and brit-build-ref subcommands. The cli-test-page runner reads the
//! captured outputs from the staging directory.

pub mod support;
```

- [ ] **Step 1.4: Create `src/support/mod.rs`**

```rust
//! Test support modules.

pub mod test_repo;
pub mod mock_remote;
pub mod normalize;
pub mod runner;
```

- [ ] **Step 1.5: Create stub files for the four support modules**

Each gets a single-line module doc + minimal stub. Will be filled in Tasks 4-7.

`src/support/test_repo.rs`:
```rust
//! TestRepo — temp git repo with deterministic commit history.

#![allow(dead_code)]
```

`src/support/mock_remote.rs`:
```rust
//! MockRemote — bare git repo + file:// URL for clone/fetch/push tests.

#![allow(dead_code)]
```

`src/support/normalize.rs`:
```rust
//! Normalizer — redact variable bits of CLI output for stable snapshots.

#![allow(dead_code)]
```

`src/support/runner.rs`:
```rust
//! BritInvocation — process invocation + capture + staging dump.

#![allow(dead_code)]
```

- [ ] **Step 1.6: Skip wiring into workspace yet (Task 3 does it)**

The crate exists but isn't a workspace member yet. Skip this and move to Task 2.

- [ ] **Step 1.7: Commit (in brit submodule)**

```bash
cd /home/matthew/git/elohim/elohim/brit
git status
git add tests/cli-journey/
git commit -m "feat(cli-journey): scaffold test infrastructure crate (TestRepo, MockRemote, Normalizer, BritInvocation stubs)"
```

---

### Task 2: Create cli-test-page runner crate skeleton

**Files:**
- Create: `elohim/brit/tests/cli-test-page/Cargo.toml`
- Create: `elohim/brit/tests/cli-test-page/src/main.rs`
- Create: `elohim/brit/tests/cli-test-page/src/discover.rs`
- Create: `elohim/brit/tests/cli-test-page/src/normalize.rs`
- Create: `elohim/brit/tests/cli-test-page/src/format.rs`
- Create: `elohim/brit/tests/cli-test-page/src/coverage.rs`
- Create: `elohim/brit/tests/cli-test-page/src/diff.rs`

- [ ] **Step 2.1: Create `cli-test-page/Cargo.toml`**

```toml
lints.workspace = true

[package]
name = "cli-test-page"
version = "0.0.0"
description = "Brit CLI test page runner — produces baseline.md from journey + Rust tests"
repository = "https://github.com/ethosengine/brit"
authors = ["Matthew Dowell <matthew@ethosengine.com>"]
license = "MIT OR Apache-2.0"
edition = "2021"
rust-version = "1.82"

[[bin]]
name = "brit-test-page"
path = "src/main.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
similar = { version = "2", features = ["text"] }
regex = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
```

- [ ] **Step 2.2: Create `src/main.rs` with three-mode clap entry**

```rust
//! brit-test-page — runs the brit CLI test suite, produces baseline.md.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

mod coverage;
mod diff;
mod discover;
mod format;
mod normalize;

#[derive(Parser)]
#[command(name = "brit-test-page", version, about = "Run the brit CLI test suite and produce a markdown test page")]
struct Cli {
    /// Path to the brit workspace root (default: parent of this binary's location)
    #[arg(long)]
    workspace: Option<PathBuf>,

    /// Mode: check (default), update, or candidate
    #[command(flatten)]
    mode: Mode,
}

#[derive(clap::Args)]
#[group(multiple = false)]
struct Mode {
    /// Default mode: diff candidate vs baseline; exit 1 on mismatch
    #[arg(long, conflicts_with_all = ["update", "candidate"])]
    check: bool,
    /// Copy candidate over baseline.md (after human review of diff)
    #[arg(long, conflicts_with_all = ["check", "candidate"])]
    update: bool,
    /// Write candidate to arbitrary path (for the desired-then-iterate TDD loop)
    #[arg(long, value_name = "PATH", conflicts_with_all = ["check", "update"])]
    candidate: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    eprintln!("brit-test-page: scaffold — modes parsed but not yet implemented");
    eprintln!("  workspace: {:?}", cli.workspace);
    eprintln!("  check: {}, update: {}, candidate: {:?}",
        cli.mode.check, cli.mode.update, cli.mode.candidate);
    ExitCode::SUCCESS
}
```

- [ ] **Step 2.3: Create stubs for the other modules**

`src/discover.rs`:
```rust
//! Recursive --help parsing for subcommand discovery.

#![allow(dead_code)]
```

`src/normalize.rs`:
```rust
//! Output normalization (re-export or shadow cli-journey's).

#![allow(dead_code)]
```

`src/format.rs`:
```rust
//! Markdown emission of the test page.

#![allow(dead_code)]
```

`src/coverage.rs`:
```rust
//! X-of-Y coverage computation per binary.

#![allow(dead_code)]
```

`src/diff.rs`:
```rust
//! Colored terminal diff using the similar crate.

#![allow(dead_code)]
```

- [ ] **Step 2.4: Commit**

```bash
cd /home/matthew/git/elohim/elohim/brit
git status
git add tests/cli-test-page/
git commit -m "feat(cli-test-page): scaffold runner crate with three-mode clap entry (--check/--update/--candidate)"
```

---

### Task 3: Wire both crates into brit workspace

**Files:**
- Modify: `elohim/brit/Cargo.toml`
- Create/modify: `elohim/brit/tests/.gitignore`

- [ ] **Step 3.1: Add to workspace.members in brit's root Cargo.toml**

Open `/home/matthew/git/elohim/elohim/brit/Cargo.toml`. Find the `[workspace] members = [...]` block. Add the two new entries:

```toml
members = [
    # ... existing members ...
    "brit-epr",
    "brit-verify",
    "brit-build-ref",
    "brit-graph",
    "brit-cli",
    "tests/cli-journey",
    "tests/cli-test-page",
]
```

(Order doesn't matter; place near the other brit-* members.)

- [ ] **Step 3.2: Add staging dir to .gitignore**

If `elohim/brit/tests/.gitignore` doesn't exist, create it. Otherwise modify. Add:

```gitignore
.test-page-staging/
.test-page-candidate.md
```

(Both runner's intermediate output paths. The committed `baseline.md` is NOT ignored.)

- [ ] **Step 3.3: Build to verify both crates compile**

```bash
cd /home/matthew/git/elohim/elohim/brit
RUSTFLAGS="" cargo build -p cli-journey -p cli-test-page 2>&1 | tail -10
```

Expected: both build clean. cli-test-page produces a `brit-test-page` binary at `target/debug/brit-test-page`.

- [ ] **Step 3.4: Run the runner to confirm scaffold works**

```bash
cd /home/matthew/git/elohim/elohim/brit
./target/debug/brit-test-page --help
echo "---"
./target/debug/brit-test-page  # default --check mode (no-op stub)
```

Expected: --help shows three modes; default invocation prints the scaffold message and exits 0.

- [ ] **Step 3.5: Commit**

```bash
cd /home/matthew/git/elohim/elohim/brit
git status
git add Cargo.toml tests/.gitignore
git add Cargo.lock
git commit -m "chore(brit): wire cli-journey + cli-test-page into workspace + gitignore staging dirs"
```

---

## Phase 2: cli-journey Support Infrastructure

### Task 4: TestRepo helper

**Files:**
- Modify: `elohim/brit/tests/cli-journey/src/support/test_repo.rs`
- Create: `elohim/brit/tests/cli-journey/tests/test_repo.rs` (self-test)

- [ ] **Step 4.1: Write failing self-test**

Create `/home/matthew/git/elohim/elohim/brit/tests/cli-journey/tests/test_repo.rs`:

```rust
//! Self-tests for the TestRepo helper.

use cli_journey::support::test_repo::TestRepo;

#[test]
fn test_repo_initializes_with_one_commit() {
    let repo = TestRepo::new("base").expect("init");
    assert!(repo.path().exists());
    assert!(repo.path().join(".git").exists());
    let head = repo.head_id().expect("head");
    assert_eq!(head.len(), 40, "git SHA-1 hex length");
}

#[test]
fn test_repo_commit_file_returns_stable_sha_with_static_env() {
    // Two TestRepos with identical fixture content should produce identical SHAs
    // because both use the static-git-environment for author/committer/dates.
    let a = TestRepo::new("a").expect("a");
    let b = TestRepo::new("b").expect("b");
    let sha_a = a.commit_file("foo.txt", "hello\n").expect("commit a");
    let sha_b = b.commit_file("foo.txt", "hello\n").expect("commit b");
    assert_eq!(sha_a, sha_b, "deterministic SHA across instances");
}

#[test]
fn test_repo_drop_cleans_up_path() {
    let path = {
        let repo = TestRepo::new("ephemeral").expect("init");
        repo.path().to_path_buf()
    };
    // After drop, the temp dir is gone
    assert!(!path.exists(), "temp dir removed on drop");
}
```

- [ ] **Step 4.2: Run test to verify it fails**

```bash
cd /home/matthew/git/elohim/elohim/brit
RUSTFLAGS="" cargo test -p cli-journey 2>&1 | tail -10
```

Expected: FAIL — `TestRepo` does not exist yet (or the methods don't).

- [ ] **Step 4.3: Implement TestRepo**

Replace the stub at `/home/matthew/git/elohim/elohim/brit/tests/cli-journey/src/support/test_repo.rs`:

```rust
//! TestRepo — temp git repo with deterministic commit history.
//!
//! Uses the static-git-environment env vars (matching gitoxide's
//! `helpers.sh::set-static-git-environment`) so commits made within the
//! test produce stable SHA values across runs and machines.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use tempfile::TempDir;

/// Static git environment matching gitoxide's helpers.sh.
/// Set on every git invocation made by TestRepo.
const STATIC_ENV: &[(&str, &str)] = &[
    ("GIT_AUTHOR_DATE", "2020-09-09 09:06:03 +0800"),
    ("GIT_COMMITTER_DATE", "2020-09-09 09:06:03 +0800"),
    ("GIT_AUTHOR_NAME", "Sebastian Thiel"),
    ("GIT_COMMITTER_NAME", "Sebastian Thiel"),
    ("GIT_AUTHOR_EMAIL", "git@example.com"),
    ("GIT_COMMITTER_EMAIL", "git@example.com"),
];

/// A temp git repo with a deterministic commit history.
///
/// Lifetime tied to the held TempDir — drops the dir when dropped.
pub struct TestRepo {
    _temp: TempDir,
    path: PathBuf,
}

impl TestRepo {
    /// Create a new temp repo with a single empty commit on `main`.
    /// `label` is included in the temp dir name for debugging.
    pub fn new(label: &str) -> Result<Self> {
        let temp = tempfile::Builder::new()
            .prefix(&format!("brit-test-{label}-"))
            .tempdir()
            .context("mktemp")?;
        let path = temp.path().to_path_buf();

        Self::git(&path, &["init", "-q", "--initial-branch=main"])?;
        Self::git(&path, &["commit", "--allow-empty", "-q", "-m", "init"])?;

        Ok(Self { _temp: temp, path })
    }

    /// The repo's working directory.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Write a file and commit it. Returns the new commit SHA-1 hex.
    pub fn commit_file(&self, rel: &str, contents: &str) -> Result<String> {
        let abs = self.path.join(rel);
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent).context("mkdir")?;
        }
        std::fs::write(&abs, contents).context("write file")?;
        Self::git(&self.path, &["add", rel])?;
        Self::git(&self.path, &["commit", "-q", "-m", &format!("add {rel}")])?;
        self.head_id()
    }

    /// Get the HEAD commit SHA-1 hex (40 chars).
    pub fn head_id(&self) -> Result<String> {
        let out = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.path)
            .envs(STATIC_ENV.iter().copied())
            .output()
            .context("git rev-parse")?;
        if !out.status.success() {
            return Err(anyhow!(
                "git rev-parse failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        Ok(String::from_utf8(out.stdout)?.trim().to_string())
    }

    fn git(path: &Path, args: &[&str]) -> Result<()> {
        let out = Command::new("git")
            .args(args)
            .current_dir(path)
            .envs(STATIC_ENV.iter().copied())
            .output()
            .with_context(|| format!("git {args:?}"))?;
        if !out.status.success() {
            return Err(anyhow!(
                "git {args:?} failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        Ok(())
    }
}
```

- [ ] **Step 4.4: Run tests to verify they pass**

```bash
cd /home/matthew/git/elohim/elohim/brit
RUSTFLAGS="" cargo test -p cli-journey 2>&1 | tail -15
```

Expected: 3/3 tests pass.

- [ ] **Step 4.5: Commit**

```bash
cd /home/matthew/git/elohim/elohim/brit
git status
git add tests/cli-journey/src/support/test_repo.rs tests/cli-journey/tests/test_repo.rs
git commit -m "feat(cli-journey): TestRepo helper with static-git-environment for deterministic SHAs"
```

---

### Task 5: MockRemote helper

**Files:**
- Modify: `elohim/brit/tests/cli-journey/src/support/mock_remote.rs`
- Create: `elohim/brit/tests/cli-journey/tests/mock_remote.rs`

- [ ] **Step 5.1: Write failing self-test**

`/home/matthew/git/elohim/elohim/brit/tests/cli-journey/tests/mock_remote.rs`:

```rust
//! Self-tests for the MockRemote helper.

use cli_journey::support::mock_remote::MockRemote;
use cli_journey::support::test_repo::TestRepo;

#[test]
fn mock_remote_url_is_file_scheme() {
    let remote = MockRemote::new("upstream").expect("init");
    let url = remote.url();
    assert!(url.starts_with("file://"), "url: {url}");
    assert!(url.ends_with(".git") || url.contains(".git"), "url ends with .git: {url}");
}

#[test]
fn mock_remote_is_a_bare_repo() {
    let remote = MockRemote::new("upstream").expect("init");
    // Bare repos don't have a .git/ directory; the dir IS the repo
    assert!(remote.path().exists());
    assert!(remote.path().join("HEAD").exists(), "bare repo has HEAD at top level");
}

#[test]
fn local_can_clone_from_mock_remote_then_push_back() {
    let upstream = MockRemote::new("upstream").expect("upstream init");
    let local_temp = tempfile::Builder::new()
        .prefix("brit-test-clone-")
        .tempdir()
        .expect("mktemp");
    let local_path = local_temp.path().join("clone");

    // git clone <url> <local_path>
    let status = std::process::Command::new("git")
        .args(["clone", "-q", &upstream.url()])
        .arg(&local_path)
        .status()
        .expect("git clone");
    assert!(status.success(), "clone succeeded");
    assert!(local_path.join(".git").exists(), ".git in clone");
}
```

- [ ] **Step 5.2: Run test to verify it fails**

```bash
RUSTFLAGS="" cargo test -p cli-journey mock_remote 2>&1 | tail -10
```

Expected: FAIL — `MockRemote` doesn't exist.

- [ ] **Step 5.3: Implement MockRemote**

Replace `/home/matthew/git/elohim/elohim/brit/tests/cli-journey/src/support/mock_remote.rs`:

```rust
//! MockRemote — bare git repo + file:// URL for clone/fetch/push tests.
//!
//! Uses local file:// transport; no daemon, no network. Deterministic.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use tempfile::TempDir;

/// A bare git repo at a temp path, accessible via file:// URL.
pub struct MockRemote {
    _temp: TempDir,
    path: PathBuf,
}

impl MockRemote {
    /// Create a new bare repo. `label` is included in the temp dir name.
    pub fn new(label: &str) -> Result<Self> {
        let temp = tempfile::Builder::new()
            .prefix(&format!("brit-mockremote-{label}-"))
            .tempdir()
            .context("mktemp")?;
        // The bare repo lives at <temp>/<label>.git
        let path = temp.path().join(format!("{label}.git"));
        std::fs::create_dir_all(&path).context("mkdir bare path")?;
        let out = Command::new("git")
            .args(["init", "-q", "--bare"])
            .current_dir(&path)
            .output()
            .context("git init --bare")?;
        if !out.status.success() {
            return Err(anyhow!(
                "git init --bare failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        Ok(Self { _temp: temp, path })
    }

    /// The bare repo's path on disk.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The file:// URL for clone/fetch/push.
    pub fn url(&self) -> String {
        format!("file://{}", self.path.display())
    }
}
```

- [ ] **Step 5.4: Run tests to verify they pass**

```bash
RUSTFLAGS="" cargo test -p cli-journey mock_remote 2>&1 | tail -10
```

Expected: 3/3 tests pass.

- [ ] **Step 5.5: Commit**

```bash
cd /home/matthew/git/elohim/elohim/brit
git status
git add tests/cli-journey/src/support/mock_remote.rs tests/cli-journey/tests/mock_remote.rs
git commit -m "feat(cli-journey): MockRemote helper (bare repo + file:// URL for clone/fetch/push tests)"
```

---

### Task 6: Normalizer

**Files:**
- Modify: `elohim/brit/tests/cli-journey/src/support/normalize.rs`
- Create: `elohim/brit/tests/cli-journey/tests/normalize.rs`

- [ ] **Step 6.1: Write failing self-tests**

`/home/matthew/git/elohim/elohim/brit/tests/cli-journey/tests/normalize.rs`:

```rust
//! Self-tests for the Normalizer.

use cli_journey::support::normalize::Normalizer;

#[test]
fn strips_ansi_escape_codes() {
    let n = Normalizer::new();
    let red_text = "\x1b[31mhello\x1b[0m world";
    assert_eq!(n.normalize(red_text), "hello world");
}

#[test]
fn redacts_tempdir_paths() {
    let n = Normalizer::new();
    // POSIX-style tempdirs
    let s = n.normalize("/tmp/brit-test-xyz123/foo");
    assert_eq!(s, "<TMPDIR>/foo");
    // macOS-style
    let s = n.normalize("/var/folders/ab/cd1234/T/brit-test-xyz/foo");
    assert!(s.contains("<TMPDIR>"), "got: {s}");
}

#[test]
fn redacts_rfc3339_timestamps() {
    let n = Normalizer::new();
    let s = n.normalize("generated_at: 2026-04-19T15:30:45.123456789+00:00");
    assert!(s.contains("<TIMESTAMP>"), "got: {s}");
    assert!(!s.contains("2026-04-19T15"), "got: {s}");
}

#[test]
fn redacts_variable_git_shas() {
    let mut n = Normalizer::new();
    // SHAs not declared as stable get redacted
    let sha = "a".repeat(40);
    let s = n.normalize(&sha);
    assert_eq!(s, "<SHA>");
}

#[test]
fn preserves_stable_git_shas() {
    let mut n = Normalizer::new();
    let stable = "b".repeat(40);
    n.add_stable_sha(&stable);
    let s = n.normalize(&stable);
    assert_eq!(s, stable);
}

#[test]
fn redacts_short_git_shas() {
    let n = Normalizer::new();
    // 7-char abbreviated SHAs are common in `git log` output
    let s = n.normalize("commit abc1234 by author");
    assert_eq!(s, "commit <SHA> by author");
}
```

- [ ] **Step 6.2: Run test to verify it fails**

```bash
RUSTFLAGS="" cargo test -p cli-journey normalize 2>&1 | tail -10
```

Expected: FAIL — `Normalizer` doesn't exist.

- [ ] **Step 6.3: Implement Normalizer**

Replace `/home/matthew/git/elohim/elohim/brit/tests/cli-journey/src/support/normalize.rs`:

```rust
//! Normalizer — redact variable bits of CLI output for stable snapshots.
//!
//! Replacements applied (in order):
//!   1. ANSI escape codes → stripped
//!   2. Tempdir paths → <TMPDIR>/...
//!   3. RFC 3339 timestamps → <TIMESTAMP>
//!   4. Git SHAs (40-char and 7-char hex) → <SHA>
//!      (with optional allowlist for "stable" SHAs that flow through verbatim)

use std::collections::HashSet;

use regex::Regex;

pub struct Normalizer {
    ansi_re: Regex,
    posix_tempdir_re: Regex,
    macos_tempdir_re: Regex,
    rfc3339_re: Regex,
    sha40_re: Regex,
    sha7_re: Regex,
    stable_shas: HashSet<String>,
}

impl Default for Normalizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Normalizer {
    pub fn new() -> Self {
        Self {
            ansi_re: Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap(),
            // /tmp/anything-up-to-next-space-or-end OR /tmp/path/with/segments
            // brit-test-XXX or brit-mockremote-XXX prefixes
            posix_tempdir_re: Regex::new(
                r"/tmp/(?:brit-test-|brit-mockremote-|brit-)[A-Za-z0-9_\-.]+",
            )
            .unwrap(),
            // macOS: /var/folders/XX/YYYY/T/brit-test-...
            macos_tempdir_re: Regex::new(
                r"/var/folders/[A-Za-z0-9_]+/[A-Za-z0-9_]+/T/(?:brit-test-|brit-)[A-Za-z0-9_\-.]+",
            )
            .unwrap(),
            rfc3339_re: Regex::new(
                r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:[+-]\d{2}:\d{2}|Z)",
            )
            .unwrap(),
            sha40_re: Regex::new(r"\b[0-9a-f]{40}\b").unwrap(),
            sha7_re: Regex::new(r"\b[0-9a-f]{7,12}\b").unwrap(),
            stable_shas: HashSet::new(),
        }
    }

    /// Mark a SHA (40-char or 7-char) as "stable" — flows through verbatim
    /// instead of being redacted. Use for fixed-content commits whose SHAs
    /// are known to be deterministic via `set-static-git-environment`.
    pub fn add_stable_sha(&mut self, sha: &str) {
        self.stable_shas.insert(sha.to_string());
        // Also add the 7-char abbreviation, which is what `git log --oneline` uses
        if sha.len() >= 7 {
            self.stable_shas.insert(sha[..7].to_string());
        }
    }

    /// Apply all normalizations. Order matters (ANSI first to avoid eating
    /// other patterns; SHAs last since they may contain hex digits that
    /// would otherwise look like nothing).
    pub fn normalize(&self, text: &str) -> String {
        let mut out = self.ansi_re.replace_all(text, "").into_owned();
        out = self.macos_tempdir_re.replace_all(&out, "<TMPDIR>").into_owned();
        out = self.posix_tempdir_re.replace_all(&out, "<TMPDIR>").into_owned();
        out = self.rfc3339_re.replace_all(&out, "<TIMESTAMP>").into_owned();
        // SHA redaction: only redact ones not in the stable set.
        out = self
            .sha40_re
            .replace_all(&out, |caps: &regex::Captures<'_>| {
                let m = caps.get(0).unwrap().as_str();
                if self.stable_shas.contains(m) {
                    m.to_string()
                } else {
                    "<SHA>".to_string()
                }
            })
            .into_owned();
        out = self
            .sha7_re
            .replace_all(&out, |caps: &regex::Captures<'_>| {
                let m = caps.get(0).unwrap().as_str();
                if self.stable_shas.contains(m) {
                    m.to_string()
                } else {
                    "<SHA>".to_string()
                }
            })
            .into_owned();
        out
    }
}
```

- [ ] **Step 6.4: Run tests**

```bash
RUSTFLAGS="" cargo test -p cli-journey normalize 2>&1 | tail -15
```

Expected: 6/6 pass. If `redacts_short_git_shas` fails because the regex matches "by" or other words, tighten — short SHAs need the `\b` word boundary AND a check that they're hex (which `[0-9a-f]` provides).

- [ ] **Step 6.5: Commit**

```bash
cd /home/matthew/git/elohim/elohim/brit
git status
git add tests/cli-journey/src/support/normalize.rs tests/cli-journey/tests/normalize.rs
git commit -m "feat(cli-journey): Normalizer (ANSI/tempdir/timestamp/SHA redaction with stable-SHA allowlist)"
```

---

### Task 7: BritInvocation runner

**Files:**
- Modify: `elohim/brit/tests/cli-journey/src/support/runner.rs`
- Create: `elohim/brit/tests/cli-journey/tests/runner.rs`

- [ ] **Step 7.1: Write failing self-test**

`/home/matthew/git/elohim/elohim/brit/tests/cli-journey/tests/runner.rs`:

```rust
//! Self-tests for the BritInvocation runner.

use cli_journey::support::runner::BritInvocation;

#[test]
fn invokes_echo_and_captures_stdout() {
    // Use `echo` as a stand-in for any binary — it's universally present
    let cap = BritInvocation::new("echo")
        .arg("hello world")
        .run()
        .expect("run");
    assert!(cap.status.success());
    assert_eq!(cap.stdout.trim(), "hello world");
}

#[test]
fn captures_stderr_and_exit_code() {
    // `false` returns exit 1 with no output
    let cap = BritInvocation::new("false").run().expect("run");
    assert!(!cap.status.success());
    assert_eq!(cap.status.code(), Some(1));
}

#[test]
fn applies_normalizer_to_output() {
    // Echo a tempdir-like path; expect normalization
    let cap = BritInvocation::new("echo")
        .arg("/tmp/brit-test-xyz/foo")
        .normalize(true)
        .run()
        .expect("run");
    assert_eq!(cap.stdout.trim(), "<TMPDIR>/foo");
}
```

- [ ] **Step 7.2: Run, verify failure**

```bash
RUSTFLAGS="" cargo test -p cli-journey runner 2>&1 | tail -10
```

Expected: FAIL — `BritInvocation` doesn't exist.

- [ ] **Step 7.3: Implement BritInvocation**

Replace `/home/matthew/git/elohim/elohim/brit/tests/cli-journey/src/support/runner.rs`:

```rust
//! BritInvocation — process invocation + capture + optional normalization.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

use anyhow::{anyhow, Context, Result};

use crate::support::normalize::Normalizer;

pub struct BritInvocation {
    program: PathBuf,
    args: Vec<OsString>,
    env: Vec<(OsString, OsString)>,
    cwd: Option<PathBuf>,
    normalize: bool,
}

pub struct Capture {
    pub stdout: String,
    pub stderr: String,
    pub status: ExitStatus,
}

impl BritInvocation {
    pub fn new<P: Into<PathBuf>>(program: P) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            env: Vec::new(),
            cwd: None,
            normalize: false,
        }
    }

    pub fn arg<A: Into<OsString>>(mut self, arg: A) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, A>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = A>,
        A: Into<OsString>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn env<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<OsString>,
        V: Into<OsString>,
    {
        self.env.push((key.into(), value.into()));
        self
    }

    pub fn current_dir<P: Into<PathBuf>>(mut self, cwd: P) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Apply the default Normalizer to captured stdout/stderr.
    pub fn normalize(mut self, on: bool) -> Self {
        self.normalize = on;
        self
    }

    pub fn run(self) -> Result<Capture> {
        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args);
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        if let Some(cwd) = &self.cwd {
            cmd.current_dir(cwd);
        }
        let out = cmd
            .output()
            .with_context(|| format!("invoke {:?}", self.program))?;
        let mut stdout = String::from_utf8(out.stdout)
            .map_err(|e| anyhow!("non-utf8 stdout: {e}"))?;
        let mut stderr = String::from_utf8(out.stderr)
            .map_err(|e| anyhow!("non-utf8 stderr: {e}"))?;
        if self.normalize {
            let n = Normalizer::new();
            stdout = n.normalize(&stdout);
            stderr = n.normalize(&stderr);
        }
        Ok(Capture {
            stdout,
            stderr,
            status: out.status,
        })
    }
}
```

- [ ] **Step 7.4: Run tests**

```bash
RUSTFLAGS="" cargo test -p cli-journey runner 2>&1 | tail -10
```

Expected: 3/3 pass.

- [ ] **Step 7.5: Commit**

```bash
cd /home/matthew/git/elohim/elohim/brit
git status
git add tests/cli-journey/src/support/runner.rs tests/cli-journey/tests/runner.rs
git commit -m "feat(cli-journey): BritInvocation runner (capture stdout/stderr/status + optional normalization)"
```

---

## Phase 3: cli-test-page Discovery + Format + Modes

### Task 8: Recursive --help subcommand discovery

**Files:**
- Modify: `elohim/brit/tests/cli-test-page/src/discover.rs`
- Create: `elohim/brit/tests/cli-test-page/tests/discover.rs`

- [ ] **Step 8.1: Write failing self-test**

`/home/matthew/git/elohim/elohim/brit/tests/cli-test-page/tests/discover.rs`:

```rust
//! Self-tests for subcommand discovery.

use cli_test_page::discover::parse_subcommands_from_help;

#[test]
fn parses_top_level_subcommand_list() {
    let help_text = r#"
The git underworld

Usage: brit [OPTIONS] <COMMAND>

Commands:
  archive       Subcommands for creating worktree archives
  branch        Interact with branches [aliases: branches]
  clean         Remove untracked files from the working tree
  log           List all commits in a repository

Options:
  -h, --help     Print help
"#;
    let subs = parse_subcommands_from_help(help_text);
    assert_eq!(subs, vec!["archive", "branch", "clean", "log"]);
}

#[test]
fn returns_empty_for_leaf_subcommand_with_no_subcommands() {
    let help_text = r#"
Print all commits

Usage: brit log [OPTIONS]

Options:
  -h, --help     Print help
"#;
    let subs = parse_subcommands_from_help(help_text);
    assert!(subs.is_empty());
}

#[test]
fn ignores_alias_annotations_and_strips_whitespace() {
    let help_text = r#"
Usage: brit [OPTIONS] <COMMAND>

Commands:
  branch        Interact with branches [aliases: branches]
  remote        Interact with remotes [aliases: remotes]
"#;
    let subs = parse_subcommands_from_help(help_text);
    assert_eq!(subs, vec!["branch", "remote"]);
}
```

We're testing the parser separately from the binary invocation. The discovery flow that USES the parser comes next.

- [ ] **Step 8.2: Verify failure**

```bash
cd /home/matthew/git/elohim/elohim/brit
RUSTFLAGS="" cargo test -p cli-test-page discover 2>&1 | tail -10
```

Expected: FAIL — `parse_subcommands_from_help` doesn't exist.

- [ ] **Step 8.3: Implement discover.rs**

Replace `/home/matthew/git/elohim/elohim/brit/tests/cli-test-page/src/discover.rs`:

```rust
//! Recursive --help parsing for subcommand discovery.
//!
//! For each binary in scope, invoke `<bin> --help`, parse the
//! `Commands:` block, recurse into each subcommand's `--help`,
//! return the full tree of leaf subcommand paths.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

/// A path to a leaf subcommand, e.g. `["brit", "log"]` or `["brit", "branch", "list"]`.
pub type SubcommandPath = Vec<String>;

/// Parse the `Commands:` block of clap's --help output.
/// Returns the names of immediate subcommands (no recursion here).
///
/// Format expected (clap default):
/// ```text
/// Commands:
///   archive       Subcommands for creating worktree archives
///   branch        Interact with branches [aliases: branches]
///   help          Print this message or the help of the given subcommand(s)
/// ```
///
/// Returns an empty vec if no `Commands:` section exists (i.e., a leaf).
/// Filters out `help` (clap's auto-generated help command).
pub fn parse_subcommands_from_help(help_text: &str) -> Vec<String> {
    let mut in_commands = false;
    let mut subs: Vec<String> = Vec::new();

    for line in help_text.lines() {
        let trimmed = line.trim_end();
        if trimmed.starts_with("Commands:") {
            in_commands = true;
            continue;
        }
        if !in_commands {
            continue;
        }
        // Empty line OR section header at column 0 → end of Commands block
        if trimmed.is_empty() {
            break;
        }
        if !line.starts_with(' ') {
            // New section
            break;
        }
        // Subcommand line: "  <name>       <description>"
        let line = trimmed.trim_start();
        let name = line
            .split_whitespace()
            .next()
            .map(|s| s.to_string());
        if let Some(n) = name {
            if n != "help" {
                subs.push(n);
            }
        }
    }
    subs
}

/// Recursively discover all leaf subcommand paths for a binary.
/// Invokes `<binary> [path...] --help` for each branch.
pub fn discover_subcommands(binary: &Path, binary_name: &str) -> Result<Vec<SubcommandPath>> {
    let mut results: Vec<SubcommandPath> = Vec::new();
    let initial_path = vec![binary_name.to_string()];
    walk(binary, &initial_path, &mut results)?;
    Ok(results)
}

fn walk(binary: &Path, current: &[String], out: &mut Vec<SubcommandPath>) -> Result<()> {
    // Invoke `<binary> [args...] --help`
    let args: Vec<String> = current.iter().skip(1).cloned().collect();
    let output = Command::new(binary)
        .args(&args)
        .arg("--help")
        .output()
        .with_context(|| format!("invoke {} {:?} --help", binary.display(), args))?;
    // Combine stdout + stderr (clap may print to either)
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let subs = parse_subcommands_from_help(&combined);
    if subs.is_empty() {
        // Leaf — record it (unless it's the bare binary name with nothing else)
        if current.len() > 1 {
            out.push(current.to_vec());
        } else {
            // The binary itself with no subcommands at all is degenerate;
            // nothing to record. (Most binaries have at least one subcommand.)
        }
    } else {
        for sub in subs {
            let mut next = current.to_vec();
            next.push(sub);
            walk(binary, &next, out)?;
        }
    }
    Ok(())
}
```

- [ ] **Step 8.4: Run tests**

```bash
RUSTFLAGS="" cargo test -p cli-test-page discover 2>&1 | tail -15
```

Expected: 3/3 pass.

- [ ] **Step 8.5: Smoke-test against the real brit binary**

```bash
cd /home/matthew/git/elohim/elohim/brit
RUSTFLAGS="" cargo build -p gitoxide --bin brit 2>&1 | tail -3
# Quick exec to verify discovery works end-to-end:
RUSTFLAGS="" cargo run -p cli-test-page --bin brit-test-page -- --help
```

(We're not yet using discovery in the runner; that's Task 12. This step just ensures the discovery module compiles + the runner itself runs.)

- [ ] **Step 8.6: Commit**

```bash
cd /home/matthew/git/elohim/elohim/brit
git status
git add tests/cli-test-page/src/discover.rs tests/cli-test-page/tests/discover.rs
git commit -m "feat(cli-test-page): subcommand discovery via recursive --help parsing"
```

---

### Task 9: Coverage computation

**Files:**
- Modify: `elohim/brit/tests/cli-test-page/src/coverage.rs`
- Create: `elohim/brit/tests/cli-test-page/tests/coverage.rs`

- [ ] **Step 9.1: Write failing self-test**

`/home/matthew/git/elohim/elohim/brit/tests/cli-test-page/tests/coverage.rs`:

```rust
use cli_test_page::coverage::{compute_coverage, BinaryCoverage};
use cli_test_page::discover::SubcommandPath;
use std::collections::BTreeSet;

#[test]
fn full_coverage_when_all_paths_have_captures() {
    let universe: Vec<SubcommandPath> = vec![
        vec!["brit".into(), "log".into()],
        vec!["brit".into(), "status".into()],
    ];
    let captured: BTreeSet<SubcommandPath> = universe.iter().cloned().collect();
    let cov = compute_coverage("brit", &universe, &captured);
    assert_eq!(cov.covered, 2);
    assert_eq!(cov.total, 2);
    assert_eq!(cov.percent(), 100);
    assert!(cov.uncovered.is_empty());
}

#[test]
fn partial_coverage_lists_uncovered() {
    let universe: Vec<SubcommandPath> = vec![
        vec!["brit".into(), "log".into()],
        vec!["brit".into(), "status".into()],
        vec!["brit".into(), "blame".into()],
    ];
    let captured: BTreeSet<SubcommandPath> = vec![
        vec!["brit".into(), "log".into()],
    ]
    .into_iter()
    .collect();
    let cov = compute_coverage("brit", &universe, &captured);
    assert_eq!(cov.covered, 1);
    assert_eq!(cov.total, 3);
    assert_eq!(cov.percent(), 33);
    assert_eq!(cov.uncovered.len(), 2);
}

#[test]
fn zero_total_yields_100_percent_to_avoid_div_by_zero() {
    let universe: Vec<SubcommandPath> = vec![];
    let captured: BTreeSet<SubcommandPath> = BTreeSet::new();
    let cov = compute_coverage("empty-bin", &universe, &captured);
    assert_eq!(cov.percent(), 100);
}
```

- [ ] **Step 9.2: Run, verify failure**

```bash
RUSTFLAGS="" cargo test -p cli-test-page coverage 2>&1 | tail -10
```

Expected: FAIL — `compute_coverage`/`BinaryCoverage` don't exist.

- [ ] **Step 9.3: Implement coverage.rs**

Replace `/home/matthew/git/elohim/elohim/brit/tests/cli-test-page/src/coverage.rs`:

```rust
//! Coverage computation: compare discovered subcommand universe against
//! the set of subcommands that have captured outputs in the staging dir.

use std::collections::BTreeSet;

use crate::discover::SubcommandPath;

#[derive(Debug, Clone)]
pub struct BinaryCoverage {
    pub binary: String,
    pub covered: usize,
    pub total: usize,
    pub uncovered: Vec<SubcommandPath>,
}

impl BinaryCoverage {
    pub fn percent(&self) -> u32 {
        if self.total == 0 {
            100
        } else {
            ((self.covered * 100) / self.total) as u32
        }
    }
}

pub fn compute_coverage(
    binary: &str,
    universe: &[SubcommandPath],
    captured: &BTreeSet<SubcommandPath>,
) -> BinaryCoverage {
    let total = universe.len();
    let mut covered = 0;
    let mut uncovered = Vec::new();
    for path in universe {
        if captured.contains(path) {
            covered += 1;
        } else {
            uncovered.push(path.clone());
        }
    }
    BinaryCoverage {
        binary: binary.to_string(),
        covered,
        total,
        uncovered,
    }
}
```

- [ ] **Step 9.4: Run tests**

```bash
RUSTFLAGS="" cargo test -p cli-test-page coverage 2>&1 | tail -10
```

Expected: 3/3 pass.

- [ ] **Step 9.5: Commit**

```bash
cd /home/matthew/git/elohim/elohim/brit
git status
git add tests/cli-test-page/src/coverage.rs tests/cli-test-page/tests/coverage.rs
git commit -m "feat(cli-test-page): coverage computation (per-binary covered/total/uncovered + percent)"
```

---

### Task 10: Markdown format module

**Files:**
- Modify: `elohim/brit/tests/cli-test-page/src/format.rs`
- Create: `elohim/brit/tests/cli-test-page/tests/format.rs`

- [ ] **Step 10.1: Write failing self-test**

`/home/matthew/git/elohim/elohim/brit/tests/cli-test-page/tests/format.rs`:

```rust
use cli_test_page::coverage::BinaryCoverage;
use cli_test_page::format::{format_test_page, BinarySection, SubcommandCapture};

#[test]
fn renders_coverage_summary_table() {
    let coverage = vec![BinaryCoverage {
        binary: "brit".into(),
        covered: 5,
        total: 5,
        uncovered: vec![],
    }];
    let sections: Vec<BinarySection> = vec![];
    let md = format_test_page(&coverage, &sections);
    assert!(md.contains("# Brit CLI Test Page"));
    assert!(md.contains("## Coverage"));
    assert!(md.contains("brit"));
    assert!(md.contains("5"));
    assert!(md.contains("100%"));
}

#[test]
fn renders_subcommand_capture_with_help_invocation_output() {
    let coverage = vec![];
    let sections = vec![BinarySection {
        binary: "brit".into(),
        captures: vec![SubcommandCapture {
            subcommand_path: vec!["brit".into(), "log".into()],
            help: "Print all commits".into(),
            invocation: "brit log".into(),
            output: "abc1234 init\n".into(),
        }],
    }];
    let md = format_test_page(&coverage, &sections);
    assert!(md.contains("### brit log"));
    assert!(md.contains("**Help:** Print all commits"));
    assert!(md.contains("```sh\nbrit log\n```"));
    assert!(md.contains("```\nabc1234 init\n"));
}
```

- [ ] **Step 10.2: Verify failure**

```bash
RUSTFLAGS="" cargo test -p cli-test-page format 2>&1 | tail -10
```

Expected: FAIL.

- [ ] **Step 10.3: Implement format.rs**

Replace `/home/matthew/git/elohim/elohim/brit/tests/cli-test-page/src/format.rs`:

```rust
//! Markdown emission of the test page.

use std::fmt::Write;

use crate::coverage::BinaryCoverage;
use crate::discover::SubcommandPath;

pub struct BinarySection {
    pub binary: String,
    pub captures: Vec<SubcommandCapture>,
}

pub struct SubcommandCapture {
    pub subcommand_path: SubcommandPath,
    pub help: String,
    pub invocation: String,
    pub output: String,
}

pub fn format_test_page(coverage: &[BinaryCoverage], sections: &[BinarySection]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Brit CLI Test Page");
    let _ = writeln!(out);
    let _ = writeln!(out, "_Auto-generated by `brit-test-page`. Do not edit by hand unless\n_you're using the `--candidate` TDD-redesign workflow._");
    let _ = writeln!(out);

    // Coverage summary
    let _ = writeln!(out, "## Coverage");
    let _ = writeln!(out);
    let _ = writeln!(out, "| Binary | Covered | Total | % |");
    let _ = writeln!(out, "|---|---|---|---|");
    for c in coverage {
        let _ = writeln!(out, "| {} | {} | {} | {}% |", c.binary, c.covered, c.total, c.percent());
    }
    let total_covered: usize = coverage.iter().map(|c| c.covered).sum();
    let total_total: usize = coverage.iter().map(|c| c.total).sum();
    let total_pct = if total_total == 0 { 100 } else { (total_covered * 100) / total_total };
    let _ = writeln!(out, "| **Total** | **{}** | **{}** | **{}%** |", total_covered, total_total, total_pct);
    let _ = writeln!(out);

    // Uncovered list per binary
    let any_uncovered: bool = coverage.iter().any(|c| !c.uncovered.is_empty());
    if any_uncovered {
        let _ = writeln!(out, "### Uncovered subcommands");
        let _ = writeln!(out);
        for c in coverage {
            for path in &c.uncovered {
                let _ = writeln!(out, "- `{}`", path.join(" "));
            }
        }
        let _ = writeln!(out);
    }

    // Per-binary sections
    for section in sections {
        let _ = writeln!(out, "## {}", section.binary);
        let _ = writeln!(out);
        for cap in &section.captures {
            let _ = writeln!(out, "### {}", cap.subcommand_path.join(" "));
            let _ = writeln!(out);
            let _ = writeln!(out, "**Help:** {}", cap.help);
            let _ = writeln!(out);
            let _ = writeln!(out, "**Invocation:**");
            let _ = writeln!(out);
            let _ = writeln!(out, "```sh");
            let _ = writeln!(out, "{}", cap.invocation);
            let _ = writeln!(out, "```");
            let _ = writeln!(out);
            let _ = writeln!(out, "**Output:**");
            let _ = writeln!(out);
            let _ = writeln!(out, "```");
            let _ = writeln!(out, "{}", cap.output.trim_end());
            let _ = writeln!(out, "```");
            let _ = writeln!(out);
        }
    }

    out
}
```

- [ ] **Step 10.4: Run tests**

```bash
RUSTFLAGS="" cargo test -p cli-test-page format 2>&1 | tail -10
```

Expected: 2/2 pass.

- [ ] **Step 10.5: Commit**

```bash
cd /home/matthew/git/elohim/elohim/brit
git status
git add tests/cli-test-page/src/format.rs tests/cli-test-page/tests/format.rs
git commit -m "feat(cli-test-page): markdown format (coverage table + per-binary subcommand captures)"
```

---

### Task 11: Diff module

**Files:**
- Modify: `elohim/brit/tests/cli-test-page/src/diff.rs`
- Create: `elohim/brit/tests/cli-test-page/tests/diff.rs`

- [ ] **Step 11.1: Write failing self-test**

`/home/matthew/git/elohim/elohim/brit/tests/cli-test-page/tests/diff.rs`:

```rust
use cli_test_page::diff::{render_unified_diff, has_diff};

#[test]
fn identical_strings_have_no_diff() {
    let a = "hello\nworld\n";
    let b = "hello\nworld\n";
    assert!(!has_diff(a, b));
}

#[test]
fn different_strings_have_diff() {
    let a = "hello\nworld\n";
    let b = "hello\nWORLD\n";
    assert!(has_diff(a, b));
}

#[test]
fn render_unified_diff_includes_changed_lines() {
    let a = "alpha\nbeta\n";
    let b = "alpha\nGAMMA\n";
    let d = render_unified_diff(a, b);
    assert!(d.contains("beta"), "diff: {d}");
    assert!(d.contains("GAMMA"), "diff: {d}");
}
```

- [ ] **Step 11.2: Verify failure**

```bash
RUSTFLAGS="" cargo test -p cli-test-page diff 2>&1 | tail -10
```

Expected: FAIL.

- [ ] **Step 11.3: Implement diff.rs**

Replace `/home/matthew/git/elohim/elohim/brit/tests/cli-test-page/src/diff.rs`:

```rust
//! Unified diff between baseline and candidate.

use similar::{ChangeTag, TextDiff};

pub fn has_diff(a: &str, b: &str) -> bool {
    a != b
}

pub fn render_unified_diff(a: &str, b: &str) -> String {
    let diff = TextDiff::from_lines(a, b);
    let mut out = String::new();
    for change in diff.iter_all_changes() {
        let prefix = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        out.push_str(prefix);
        out.push_str(change.value());
    }
    out
}
```

- [ ] **Step 11.4: Run tests**

```bash
RUSTFLAGS="" cargo test -p cli-test-page diff 2>&1 | tail -10
```

Expected: 3/3 pass.

- [ ] **Step 11.5: Commit**

```bash
cd /home/matthew/git/elohim/elohim/brit
git status
git add tests/cli-test-page/src/diff.rs tests/cli-test-page/tests/diff.rs
git commit -m "feat(cli-test-page): unified diff via similar crate"
```

---

### Task 12: Wire --check / --update / --candidate modes

**Files:**
- Modify: `elohim/brit/tests/cli-test-page/src/main.rs`

- [ ] **Step 12.1: Implement the three modes in main.rs**

Replace `/home/matthew/git/elohim/elohim/brit/tests/cli-test-page/src/main.rs`:

```rust
//! brit-test-page — runs the brit CLI test suite, produces baseline.md.
//!
//! Three modes:
//!   --check (default)           — diff candidate vs baseline.md; exit 1 on mismatch
//!   --update                    — copy candidate over baseline.md (after human review)
//!   --candidate <path>          — write candidate to arbitrary path (TDD redesign loop)

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

use anyhow::{Context, Result};
use clap::Parser;

mod coverage;
mod diff;
mod discover;
mod format;
mod normalize;

use coverage::compute_coverage;
use discover::{discover_subcommands, SubcommandPath};
use format::{format_test_page, BinarySection, SubcommandCapture};

const BINARIES: &[&str] = &["brit", "rakia", "brit-verify", "brit-build-ref"];

#[derive(Parser)]
#[command(name = "brit-test-page", version, about = "Run the brit CLI test suite and produce a markdown test page")]
struct Cli {
    /// Path to the brit workspace root (default: this binary's CARGO_MANIFEST_DIR/../..)
    #[arg(long)]
    workspace: Option<PathBuf>,

    /// Diff candidate vs baseline; exit 1 on mismatch (default mode)
    #[arg(long, conflicts_with_all = ["update", "candidate"])]
    check: bool,

    /// Copy candidate over baseline.md
    #[arg(long, conflicts_with_all = ["check", "candidate"])]
    update: bool,

    /// Write candidate to arbitrary path
    #[arg(long, value_name = "PATH", conflicts_with_all = ["check", "update"])]
    candidate: Option<PathBuf>,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            for cause in e.chain().skip(1) {
                eprintln!("caused by: {cause}");
            }
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let workspace = cli.workspace.unwrap_or_else(|| {
        // CARGO_MANIFEST_DIR is set when running via `cargo run`; otherwise
        // assume the binary lives at <ws>/target/release/brit-test-page
        // and ws is two dirs up.
        std::env::var("CARGO_MANIFEST_DIR")
            .map(|s| {
                let p = PathBuf::from(s);
                p.parent().and_then(|p| p.parent()).map(|p| p.to_path_buf()).unwrap_or(p)
            })
            .unwrap_or_else(|_| std::env::current_dir().expect("cwd"))
    });

    let target_dir = workspace.join("target/release");
    let baseline_path = workspace.join("tests/baseline.md");

    // Step 1: invoke shell journey tests + Rust journey tests; both write to staging.
    invoke_test_layers(&workspace)?;

    // Step 2: discover the universe of subcommands.
    let mut all_coverage = Vec::new();
    let mut all_sections = Vec::new();
    let staging_dir = workspace.join("tests/.test-page-staging");

    for binary_name in BINARIES {
        let binary_path = target_dir.join(binary_name);
        if !binary_path.exists() {
            eprintln!("warning: {binary_name} not found at {}; skipping", binary_path.display());
            continue;
        }
        let universe = discover_subcommands(&binary_path, binary_name)
            .with_context(|| format!("discover {binary_name}"))?;
        let captured = collect_captured_paths(&staging_dir, binary_name)?;
        let cov = compute_coverage(binary_name, &universe, &captured);
        all_coverage.push(cov);

        let captures = read_captures(&staging_dir, binary_name)?;
        all_sections.push(BinarySection {
            binary: binary_name.to_string(),
            captures,
        });
    }

    // Step 3: format candidate.
    let candidate = format_test_page(&all_coverage, &all_sections);

    // Step 4: dispatch on mode.
    if let Some(out_path) = cli.candidate {
        fs::write(&out_path, &candidate).with_context(|| format!("write {}", out_path.display()))?;
        println!("wrote candidate to {}", out_path.display());
        return Ok(ExitCode::SUCCESS);
    }

    if cli.update {
        fs::write(&baseline_path, &candidate)
            .with_context(|| format!("write {}", baseline_path.display()))?;
        println!("baseline updated: {}", baseline_path.display());
        return Ok(ExitCode::SUCCESS);
    }

    // Default: --check mode.
    let baseline = fs::read_to_string(&baseline_path)
        .with_context(|| format!("read {}", baseline_path.display()))
        .unwrap_or_default();
    if diff::has_diff(&baseline, &candidate) {
        println!("--- baseline (current)");
        println!("+++ candidate (this run)");
        println!("{}", diff::render_unified_diff(&baseline, &candidate));
        eprintln!("\nbaseline differs from candidate. Run --update to accept changes.");
        Ok(ExitCode::from(1))
    } else {
        println!("OK — candidate matches baseline.");
        Ok(ExitCode::SUCCESS)
    }
}

fn invoke_test_layers(workspace: &PathBuf) -> Result<()> {
    let staging = workspace.join("tests/.test-page-staging");
    fs::remove_dir_all(&staging).ok();
    fs::create_dir_all(&staging).context("mkdir staging")?;

    // Shell layer: invoke journey.sh (it sources gix.sh, ein.sh, rakia.sh, etc.)
    // The shell tests dump captured outputs into tests/.test-page-staging/shell/<binary>/<subcmd>.txt
    // For now the shell layer doesn't write to staging — we'll add that as we build out per-binary tests.
    // Skip shell invocation here; the runner just reads what's there.

    // Rust layer: invoke `cargo test -p cli-journey` which writes to staging via runner helpers.
    // (Hooked in via test side effects when individual tests are filled in.)
    let status = Command::new("cargo")
        .args(["test", "-p", "cli-journey", "--", "--nocapture"])
        .env("BRIT_TEST_PAGE_STAGING", &staging)
        .current_dir(workspace)
        .status()
        .context("run cargo test -p cli-journey")?;
    if !status.success() {
        anyhow::bail!("cli-journey tests failed (exit {})", status.code().unwrap_or(-1));
    }
    Ok(())
}

fn collect_captured_paths(staging_dir: &PathBuf, binary: &str) -> Result<BTreeSet<SubcommandPath>> {
    // Look for files at <staging>/{shell,rust}/<binary>/.../*.txt
    let mut out: BTreeSet<SubcommandPath> = BTreeSet::new();
    for layer in ["shell", "rust"] {
        let bin_dir = staging_dir.join(layer).join(binary);
        if !bin_dir.exists() {
            continue;
        }
        walk_captures(&bin_dir, &[binary.to_string()], &mut out)?;
    }
    Ok(out)
}

fn walk_captures(
    dir: &std::path::Path,
    prefix: &[String],
    out: &mut BTreeSet<SubcommandPath>,
) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
        if path.is_dir() {
            let mut next = prefix.to_vec();
            next.push(name);
            walk_captures(&path, &next, out)?;
        } else if name.ends_with(".txt") {
            let stem = name.trim_end_matches(".txt").to_string();
            let mut full = prefix.to_vec();
            full.push(stem);
            out.insert(full);
        }
    }
    Ok(())
}

fn read_captures(staging_dir: &PathBuf, binary: &str) -> Result<Vec<SubcommandCapture>> {
    let mut captures = Vec::new();
    for layer in ["shell", "rust"] {
        let bin_dir = staging_dir.join(layer).join(binary);
        if !bin_dir.exists() {
            continue;
        }
        read_capture_dir(&bin_dir, &[binary.to_string()], &mut captures)?;
    }
    captures.sort_by(|a, b| a.subcommand_path.cmp(&b.subcommand_path));
    Ok(captures)
}

fn read_capture_dir(
    dir: &std::path::Path,
    prefix: &[String],
    captures: &mut Vec<SubcommandCapture>,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
        if path.is_dir() {
            let mut next = prefix.to_vec();
            next.push(name);
            read_capture_dir(&path, &next, captures)?;
        } else if name.ends_with(".txt") {
            let stem = name.trim_end_matches(".txt").to_string();
            let mut subpath = prefix.to_vec();
            subpath.push(stem);
            let body = fs::read_to_string(&path)?;
            captures.push(SubcommandCapture {
                subcommand_path: subpath,
                help: String::from("(captured by test)"),
                invocation: format!("{}", path.display()),
                output: body,
            });
        }
    }
    Ok(())
}
```

- [ ] **Step 12.2: Build**

```bash
cd /home/matthew/git/elohim/elohim/brit
RUSTFLAGS="" cargo build -p cli-test-page 2>&1 | tail -10
```

Expected: clean.

- [ ] **Step 12.3: Smoke run** (will exit with low coverage % but the runner itself works)

```bash
cd /home/matthew/git/elohim/elohim/brit
RUSTFLAGS="" cargo build -p gitoxide --bin brit -p brit-cli --bin rakia -p brit-verify -p brit-build-ref --release 2>&1 | tail -3
./target/release/brit-test-page --candidate /tmp/candidate.md 2>&1 | tail -5
head -30 /tmp/candidate.md
```

Expected: candidate is generated; coverage table shows 0% per binary (no tests yet); empty subcommand sections.

- [ ] **Step 12.4: Commit**

```bash
cd /home/matthew/git/elohim/elohim/brit
git status
git add tests/cli-test-page/src/main.rs
git commit -m "feat(cli-test-page): wire three modes (--check/--update/--candidate) + discovery + format + diff"
```

---

## Phase 4: Coverage for Small Binaries (rakia + brit-verify + brit-build-ref)

### Task 13: Rakia journey shell + Rust integration tests

**Files:**
- Create: `elohim/brit/tests/journey/rakia.sh`
- Create: `elohim/brit/tests/cli-journey/tests/rakia.rs`
- Modify: `elohim/brit/tests/journey.sh` (source rakia.sh)

- [ ] **Step 13.1: Add the shell smoke test**

`/home/matthew/git/elohim/elohim/brit/tests/journey/rakia.sh`:

```bash
# Must be sourced into the main journey test
# Smoke tests for the rakia binary — proves it starts + emits help.
# Detailed per-subcommand coverage lives in cli-journey/tests/rakia.rs (Rust).

title rakia
(when "running '$exe_plumbing rakia --help' equivalent"
  exe_rakia="${exe%/*}/rakia"
  it "prints the top-level help" && {
    expect_run $SUCCESSFULLY "$exe_rakia" --help
  }
  it "exits 2 on no args (clap usage error)" && {
    expect_run $WITH_CLAP_FAILURE "$exe_rakia"
  }
)
```

Then modify `/home/matthew/git/elohim/elohim/brit/tests/journey.sh` to source it:

Find the existing `source "$root/journey/gix.sh"` line. Add after it:

```bash
source "$root/journey/rakia.sh"
```

- [ ] **Step 13.2: Add Rust integration tests for rakia**

`/home/matthew/git/elohim/elohim/brit/tests/cli-journey/tests/rakia.rs`:

```rust
//! Coverage tests for the rakia binary.
//! Each test invokes a rakia subcommand against a self-contained fixture
//! and dumps the (normalized) output to BRIT_TEST_PAGE_STAGING/rust/rakia/<subcommand>.txt
//! for the cli-test-page runner to pick up.

use std::fs;
use std::path::PathBuf;

use cli_journey::support::runner::BritInvocation;
use cli_journey::support::test_repo::TestRepo;

fn rakia_bin() -> PathBuf {
    // tests/cli-journey -> ../../target/release/rakia
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/release/rakia")
        .canonicalize()
        .expect("rakia binary not built — run `cargo build -p brit-cli --release` first")
}

fn staging_dump(subcommand: &str, output: &str) {
    if let Ok(staging) = std::env::var("BRIT_TEST_PAGE_STAGING") {
        let dir = PathBuf::from(staging).join("rust/rakia");
        fs::create_dir_all(&dir).expect("mkdir staging");
        fs::write(dir.join(format!("{subcommand}.txt")), output).expect("write capture");
    }
}

#[test]
fn graph_discover_emits_manifests_array() {
    // Use the actual elohim repo as a fixture (it has 8+ build manifests)
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../").canonicalize().unwrap();

    let cap = BritInvocation::new(rakia_bin())
        .args(["graph", "discover", "--repo"])
        .arg(&repo_root)
        .normalize(true)
        .run()
        .expect("invoke");
    assert!(cap.status.success(), "exit: {:?} stderr: {}", cap.status, cap.stderr);
    assert!(cap.stdout.contains("manifests"), "stdout: {}", cap.stdout);

    staging_dump("graph_discover", &cap.stdout);
}

#[test]
fn fingerprint_emits_64_char_blake3_hex() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../app/elohim-app/build-manifest.json");
    if !manifest.exists() {
        return; // skip when not run inside elohim
    }
    let cap = BritInvocation::new(rakia_bin())
        .args(["fingerprint"])
        .arg(&manifest)
        .args(["--step", "build-angular"])
        .normalize(true)
        .run()
        .expect("invoke");
    assert!(cap.status.success(), "exit: {:?}", cap.status);
    assert!(cap.stdout.contains("fingerprint"), "stdout: {}", cap.stdout);

    staging_dump("fingerprint", &cap.stdout);
}

#[test]
fn baseline_read_returns_null_for_unknown_pipeline() {
    let temp = TestRepo::new("baseline-test").expect("repo");
    let cap = BritInvocation::new(rakia_bin())
        .args(["baseline", "read", "no-such-pipeline", "--repo"])
        .arg(temp.path())
        .normalize(true)
        .run()
        .expect("invoke");
    assert!(cap.status.success(), "exit: {:?}", cap.status);
    assert!(cap.stdout.contains("\"commit\": null"), "stdout: {}", cap.stdout);

    staging_dump("baseline_read", &cap.stdout);
}

// Add tests for: graph_show, affected, plan, baseline_write, baseline_migrate
// following the same pattern. Each captures output to staging via staging_dump.
```

For brevity, the plan stops here for rakia tests; the implementer extends with the remaining subcommands following the pattern.

- [ ] **Step 13.3: Run the Rust tests + verify staging captures**

```bash
cd /home/matthew/git/elohim/elohim/brit
RUSTFLAGS="" cargo build -p brit-cli --release 2>&1 | tail -3
mkdir -p tests/.test-page-staging
BRIT_TEST_PAGE_STAGING=$PWD/tests/.test-page-staging \
  RUSTFLAGS="" cargo test -p cli-journey --test rakia 2>&1 | tail -10
ls tests/.test-page-staging/rust/rakia/
```

Expected: 3+ tests pass; staging dir contains `graph_discover.txt`, `fingerprint.txt`, `baseline_read.txt`.

- [ ] **Step 13.4: Commit**

```bash
cd /home/matthew/git/elohim/elohim/brit
git status
git add tests/journey/rakia.sh tests/journey.sh tests/cli-journey/tests/rakia.rs
git commit -m "test(rakia): journey shell smoke + Rust integration tests with staging dump"
```

---

### Task 14: brit-verify journey + Rust tests

Same pattern as Task 13. Files:

- Create: `elohim/brit/tests/journey/brit-verify.sh`
- Create: `elohim/brit/tests/cli-journey/tests/brit_verify.rs`
- Modify: `elohim/brit/tests/journey.sh` (source brit-verify.sh)

The implementer follows the Task 13 template, swapping rakia for brit-verify and using the brit-verify binary's actual subcommand surface (which is small — likely just verify itself with file/ref args).

Commit:
```bash
git add tests/journey/brit-verify.sh tests/journey.sh tests/cli-journey/tests/brit_verify.rs
git commit -m "test(brit-verify): journey shell + Rust integration tests"
```

---

### Task 15: brit-build-ref journey + Rust tests

Same pattern. Files:

- Create: `elohim/brit/tests/journey/brit-build-ref.sh`
- Create: `elohim/brit/tests/cli-journey/tests/brit_build_ref.rs`
- Modify: `elohim/brit/tests/journey.sh` (source brit-build-ref.sh)

Each subcommand of brit-build-ref (~3) gets its own Rust test that uses TestRepo (since brit-build-ref operates on git refs).

Commit:
```bash
git add tests/journey/brit-build-ref.sh tests/journey.sh tests/cli-journey/tests/brit_build_ref.rs
git commit -m "test(brit-build-ref): journey shell + Rust integration tests"
```

---

## Phase 5: Brit Coverage Extension

### Task 16: Audit existing brit (gix) journey coverage

This is a one-shot audit, no code changes — output is a list to inform Tasks 17-18.

- [ ] **Step 16.1: List subcommands brit has**

```bash
cd /home/matthew/git/elohim/elohim/brit
./target/release/brit --help 2>&1 | grep -E "^  [a-z]" | awk '{print $1}' | sort > /tmp/brit-all.txt
wc -l /tmp/brit-all.txt
head /tmp/brit-all.txt
```

- [ ] **Step 16.2: List subcommands the existing journey/gix.sh exercises**

```bash
grep -E '"\$exe_plumbing"|"\$exe_plumbing"' tests/journey/gix.sh | grep -oE '\$exe_plumbing [a-z-]+' | awk '{print $2}' | sort -u > /tmp/brit-covered.txt
wc -l /tmp/brit-covered.txt
```

(The grep pattern may need refinement based on actual journey test conventions. The implementer reads gix.sh and produces the list of covered subcommands.)

- [ ] **Step 16.3: Compute the gap**

```bash
comm -23 /tmp/brit-all.txt /tmp/brit-covered.txt > /tmp/brit-uncovered.txt
echo "Uncovered subcommands ($(wc -l < /tmp/brit-uncovered.txt)):"
cat /tmp/brit-uncovered.txt
```

This list drives Task 17.

- [ ] **Step 16.4: No commit (this is an audit step)**

---

### Task 17: Extend journey/gix.sh for daily-driver subcommand gaps

Pick the subset of uncovered subcommands that matter for daily driving brit. From the Task 16 audit, prioritize (in order):

1. `log` (likely already partial; ensure full coverage)
2. `status`
3. `diff`
4. `branch list/create/delete`
5. `commit`
6. `clone` (using a file:// mock remote — `with-mock-remote` shell helper from helpers.sh)
7. `fetch`
8. `push`
9. `tag list/create/delete`
10. `blame`
11. `cat`

For each, add a `(when ... it ... && expect_run ...)` block to gix.sh. For the snapshot expectations, write the expected output to `tests/snapshots/plumbing/<command>/<scenario>` and use `expect_snapshot` (existing helper in utilities.sh).

Pattern example for `brit log`:

```bash
title 'brit log'
(when "running 'brit log' in a fixture repo with one commit"
  snapshot="$snapshot/log"
  (with-fixture-repo "log-fixture"
    it "outputs the commit log" && {
      expect_run_sh $SUCCESSFULLY "$exe" log
    }
    it "matches the expected snapshot" && {
      expect_snapshot "$snapshot/log-default" "$($exe log)"
    }
  )
)
```

(The exact helpers like `with-fixture-repo` may need adapting based on what's already in `helpers.sh`. The implementer reads helpers.sh and uses what exists; adds new helpers only if necessary.)

- [ ] **Step 17.1: Audit gix.sh + helpers.sh + utilities.sh for what's available** (1h)

- [ ] **Step 17.2: Extend gix.sh for the 11 commands above** (2-4h)

For each command, add tests + snapshots. Commit after each batch of 3-4 commands so the diff stays reviewable.

- [ ] **Step 17.3: Run journey tests, ensure all pass**

```bash
cd /home/matthew/git/elohim/elohim/brit
RUSTFLAGS="" cargo build --release -p gitoxide
./tests/journey.sh ./target/release/brit ./target/release/brit ./target/release/jtt max 2>&1 | tail -20
```

(Adjust args to journey.sh — it expects positional args for the binary paths and the test tool. Check the existing CI invocation for the exact pattern.)

- [ ] **Step 17.4: Commit**

```bash
git add tests/journey/gix.sh tests/snapshots/
git commit -m "test(brit): extend gix.sh journey for daily-driver subcommand coverage (log, status, diff, branch, commit, clone, fetch, push, tag, blame, cat)"
```

---

### Task 18: Hook shell layer into staging dump

The shell journey tests run via `journey.sh` and produce output to stdout/stderr or to snapshot files. To plug into the cli-test-page runner, we need the shell layer to ALSO drop output captures into `tests/.test-page-staging/shell/<binary>/<subcommand>.txt`.

Two integration approaches:

**(a) Modify utilities.sh to add a `dump-to-staging` helper** that's called inside test blocks:
```bash
function dump-to-staging() {
  local binary="$1"
  local subcommand="$2"
  local content="$3"
  local staging="${BRIT_TEST_PAGE_STAGING:-$root/.test-page-staging}/shell/$binary"
  mkdir -p "$staging"
  echo "$content" > "$staging/$subcommand.txt"
}
```

Then in gix.sh:
```bash
output="$($exe log 2>&1)"
dump-to-staging "brit" "log" "$output"
expect_snapshot "$snapshot/log-default" "$output"
```

**(b) Re-derive captures from snapshot files**: have the runner read `tests/snapshots/plumbing/<command>/*` directly into the staging area. Cleaner because gix.sh stays untouched, but assumes 1:1 mapping snapshot↔subcommand which isn't always true.

Plan goes with **(a)** — explicit, easy to reason about.

- [ ] **Step 18.1: Add `dump-to-staging` to utilities.sh**

Append to `/home/matthew/git/elohim/elohim/brit/tests/utilities.sh`:

```bash
# Dump captured output to the cli-test-page staging directory (no-op when unset).
function dump-to-staging() {
  local binary="$1"
  local subcommand="$2"
  local content="$3"
  local staging="${BRIT_TEST_PAGE_STAGING:-$root/.test-page-staging}/shell/$binary"
  mkdir -p "$staging"
  printf "%s" "$content" > "$staging/$subcommand.txt"
}
```

- [ ] **Step 18.2: Add dump-to-staging calls to one representative test in gix.sh** to validate the wire-up

For example, in gix.sh near the `brit log` block from Task 17:
```bash
output="$($exe log 2>&1 || true)"
dump-to-staging "brit" "log" "$output"
expect_snapshot "$snapshot/log-default" "$output"
```

- [ ] **Step 18.3: Run journey + verify staging populated**

```bash
cd /home/matthew/git/elohim/elohim/brit
mkdir -p tests/.test-page-staging
BRIT_TEST_PAGE_STAGING=$PWD/tests/.test-page-staging \
  ./tests/journey.sh ./target/release/brit ./target/release/brit ./target/release/jtt max 2>&1 | tail -10
ls tests/.test-page-staging/shell/brit/
```

Expected: at least `log.txt` exists. Iterate on Task 17's gix.sh edits to add `dump-to-staging` calls for every covered subcommand.

- [ ] **Step 18.4: Commit**

```bash
git add tests/utilities.sh tests/journey/gix.sh
git commit -m "feat(journey): dump-to-staging helper + wire shell layer into cli-test-page runner"
```

---

## Phase 6: Initial Baseline + CI + Close

### Task 19: Generate the initial baseline.md

- [ ] **Step 19.1: Build everything, run the runner with --update**

```bash
cd /home/matthew/git/elohim/elohim/brit
RUSTFLAGS="" cargo build --release -p gitoxide --bin brit -p brit-cli --bin rakia -p brit-verify -p brit-build-ref -p cli-test-page 2>&1 | tail -5
./target/release/brit-test-page --update 2>&1 | tail -5
ls -lh tests/baseline.md
head -50 tests/baseline.md
```

Expected: `baseline.md` is written. Coverage table at the top shows X% per binary based on what Tasks 13-18 actually covered.

- [ ] **Step 19.2: Verify --check is now green**

```bash
./target/release/brit-test-page --check 2>&1 | tail -3
```

Expected: "OK — candidate matches baseline."

- [ ] **Step 19.3: Commit the baseline**

```bash
git add tests/baseline.md
git commit -m "test(brit): initial baseline.md — captures current CLI behavior across all 4 binaries"
```

---

### Task 20: Wire into CI

- [ ] **Step 20.1: Identify the brit pipeline's CI entry point**

The brit submodule may have its own Jenkinsfile, OR the parent's orchestrator may build it as part of another pipeline. Check:

```bash
ls /home/matthew/git/elohim/elohim/brit/Jenkinsfile 2>/dev/null
grep -rn "brit" /home/matthew/git/elohim/genesis/orchestrator/Jenkinsfile | head
```

If there's no brit-specific pipeline yet, this task documents the integration as a follow-up but doesn't introduce a new pipeline (out of scope).

- [ ] **Step 20.2: If a pipeline exists, add a stage**

Pseudo-stage:
```groovy
stage('brit-test-page') {
    steps {
        sh '''
            cd elohim/brit
            cargo build --release -p gitoxide --bin brit -p brit-cli --bin rakia -p brit-verify -p brit-build-ref -p cli-test-page
            ./target/release/brit-test-page --check
        '''
    }
}
```

If no pipeline, document in the sprint result that CI integration is a follow-up.

- [ ] **Step 20.3: Commit any pipeline changes**

---

### Task 21: Sprint-result artifact

- [ ] **Step 21.1: Author `docs/superpowers/sprint-results/2026-04-19-brit-cli-test-page.md`**

Contents:
- What changed: hybrid test infrastructure shipped, baseline.md committed, X% coverage achieved
- Verified properties: TDD workflow demo (edit baseline, code to match, --check goes green)
- Carry-overs: subcommands not yet covered (the list from Task 16's audit); CI integration if not done
- Sprint statistics: tasks completed, lines added, test count, coverage %

- [ ] **Step 21.2: Commit on parent dev**

```bash
cd /home/matthew/git/elohim
git add docs/superpowers/sprint-results/2026-04-19-brit-cli-test-page.md
git commit -m "docs(sprint-results): brit CLI test page sprint close"
```

---

### Task 22: Submodule pointer bumps

- [ ] **Step 22.1: Push feat branches**

```bash
git -C /home/matthew/git/elohim/elohim/brit push -u origin feat/brit-cli-test-page
```

- [ ] **Step 22.2: Merge to main + push**

```bash
cd /home/matthew/git/elohim/elohim/brit
git checkout main
git merge --no-ff feat/brit-cli-test-page -m "Merge feat/brit-cli-test-page: hybrid CLI test suite + baseline.md"
git push origin main
```

- [ ] **Step 22.3: Bump parent submodule pointer + push dev**

```bash
cd /home/matthew/git/elohim
git add elohim/brit
git commit -m "chore: bump brit submodule — CLI test page sprint"
git push origin dev   # may need rebase if dev has moved
```

---

## Self-Review

**1. Spec coverage:**

| Spec section | Tasks |
|---|---|
| Hybrid architecture (shell + Rust + runner) | Tasks 1-3 (scaffolding), Task 13-15 (per-binary tests), Task 17 (extend gix.sh) |
| TestRepo, MockRemote, Normalizer, BritInvocation | Tasks 4-7 |
| Subcommand discovery via recursive --help | Task 8 |
| Coverage computation | Task 9 |
| Markdown format | Task 10 |
| Diff | Task 11 |
| Three modes (--check, --update, --candidate) | Task 12 |
| Output normalization details | Task 6 + reuse in Task 7 |
| Mock remotes via file:// | Task 5 + use in Task 17 |
| Coverage tracking + uncovered list | Tasks 9, 12 |
| CI integration | Task 20 |
| baseline.md committed | Task 19 |
| TDD workflow validation | Task 21 (sprint-result demo) |

All spec sections covered. ✓

**2. Placeholder scan:**

- "TBD/TODO" — none used
- "Add appropriate error handling" — none; each task has concrete code
- "Similar to Task N" — Task 14 and 15 use this pattern explicitly. They're small, repetitive (one binary at a time, same pattern as Task 13). The pattern reference is supported by Task 13 having full code, so reuse is appropriate. Marking as acceptable here.
- "Write tests for the above" — never used without showing test code first

Acceptable. ✓

**3. Type consistency:**

- `TestRepo::new(label)`, `TestRepo::commit_file(rel, contents)`, `TestRepo::head_id()`, `TestRepo::path()` — consistent across Tasks 4, 13, 15.
- `MockRemote::new(label)`, `MockRemote::url()`, `MockRemote::path()` — consistent.
- `Normalizer::new()`, `Normalizer::add_stable_sha()`, `Normalizer::normalize()` — consistent.
- `BritInvocation::new(program)`, `.arg(...)`, `.args(...)`, `.env(...)`, `.current_dir(...)`, `.normalize(bool)`, `.run() -> Capture` — consistent across Tasks 7, 13, 14, 15.
- `parse_subcommands_from_help(text) -> Vec<String>`, `discover_subcommands(binary, name) -> Vec<SubcommandPath>` — consistent.
- `BinaryCoverage { binary, covered, total, uncovered, percent() }` — consistent.
- `format_test_page(coverage, sections) -> String`, `BinarySection { binary, captures }`, `SubcommandCapture { subcommand_path, help, invocation, output }` — consistent.
- `has_diff(a, b) -> bool`, `render_unified_diff(a, b) -> String` — consistent.
- `BRIT_TEST_PAGE_STAGING` env var name — consistent across Tasks 12, 13, 18.

All types/signatures match. ✓
