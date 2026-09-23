# Brit Content Fingerprint Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade brit's content fingerprint so it actually hashes file CONTENTS resolved through a step's input globs (read from a git tree at a specific commit), not just the glob pattern strings as bytes. This is the prerequisite primitive for content-addressed build dispatch and "artifact X verified by N peers" attestations in future rakia sprints.

**Architecture:** `brit-graph` gains a feature-gated (`repo`) extension `ContentFingerprint::from_repo_globs(repo, commit_id, patterns)` that uses gix to walk the tree at a specific commit, match paths against globs, read blob bytes, and feed them into the existing pure `ContentFingerprint::compute`. brit-graph stays pure when the feature is off; consumers needing IO opt in. `brit-cli`'s `fingerprint` subcommand and `plan` subcommand both consume this, computing fingerprints from real file contents instead of pattern strings. `rakia-core::build_plan::to_build_plan` is refactored to accept fingerprints as a parameter (no longer computes them — keeps rakia-core IO-free).

**Tech Stack:** Rust 2021, gix 0.81 (already in use by rakia-brit), globset (already in use by rakia-core), blake3 (via brit-epr's BritCid). No new external dependencies.

**Spec context:** No formal spec — this is a focused primitive upgrade scoped from the brit candidate threads discussed at sprint close. Design rationale captured in this plan's Design Summary section.

**Build notes:**
- Brit workspace: `cd elohim/brit && RUSTFLAGS="" cargo build`
- Rakia workspace: `cd elohim/rakia && RUSTFLAGS="" cargo build`
- Cross-workspace path deps remain: `brit-cli` references `../../rakia/rakia-core` and `../../rakia/rakia-brit`
- New: `brit-graph` will optionally depend on gix when `--features repo` is active

---

## Design Summary

### What's broken today

`brit_graph::fingerprint::ContentFingerprint::compute` is a pure deterministic hasher over a `BTreeMap<String, Vec<u8>>` of named inputs. The existing callers feed it the WRONG bytes:

- **`brit-cli/src/commands/fingerprint.rs`** builds inputs like `"source:src/**/*.ts" -> b"src/**/*.ts"` — the glob pattern STRING is hashed, not the files it matches.
- **`rakia-core/src/build_plan.rs:compute_fingerprint`** doesn't even use ContentFingerprint — it uses `DefaultHasher` (Rust stdlib's non-crypto hash) over `qualified_name + source_patterns + build_process`. Explicitly labeled placeholder.

Both produce fingerprints that are stable AGAINST PATTERN CHANGES but identical for two different file contents under the same glob. That's not content addressing — it's pattern addressing. Useless for "did peer X build the same artifact as peer Y."

### What this plan delivers

A new fingerprint surface that's content-addressing-correct:

```rust
// In brit-graph (with `repo` feature):
ContentFingerprint::from_repo_globs(
    repo: &gix::Repository,
    commit_id: gix::ObjectId,
    patterns: &[String],
) -> Result<ContentFingerprint, FingerprintError>
```

Behavior:
1. Build a `globset::GlobSet` from the patterns
2. Walk the git tree at `commit_id` (NOT the working tree — must be reproducible)
3. For each tree path matching a pattern, read the blob's bytes
4. Build `BTreeMap<String, Vec<u8>>` keyed by full path (forward-slash, repo-relative)
5. Delegate to existing `ContentFingerprint::compute(&inputs)`

Two callers update:

- **`brit fingerprint <manifest> [--commit <ref>]`** — detects the repo via `gix::discover` from the manifest's parent dir, defaults `--commit HEAD`. Per step, calls `from_repo_globs` with sources + build_process patterns combined.
- **`rakia-core::build_plan::to_build_plan`** — gains a `fingerprints: BTreeMap<String, String>` parameter (qualified_name → hex). The placeholder `compute_fingerprint` is deleted. brit-cli's `plan` subcommand computes the fingerprints itself (where the repo handle is open) and passes them in.

### Why brit-graph keeps gix as a feature

Default-features-off keeps brit-graph pure (no IO, no git). The `repo` feature pulls in gix and unlocks the from_repo_globs constructor. This matches the brit workspace's existing discipline:
- Pure computation crates (brit-epr, brit-graph) avoid IO when possible
- IO-bearing consumers (rakia-brit, brit-cli) take on the gix dependency explicitly

### Why rakia-core stays IO-free

`to_build_plan` is called in test fixtures, in contract validators, in any future programmatic context where there isn't a repo open. Today it's pure (no panics, no errors). Adding gix would make every test that constructs a BuildPlan need to also construct a fake repo — wrong tradeoff.

Instead: brit-cli does the file IO. The fingerprints flow into to_build_plan as data. Tests that don't care can pass `BTreeMap::new()` (empty fingerprints) — the schema accepts the empty string, OR we treat fingerprint as Optional in the BuildPlan schema (TBD per Task 7's design choice — see notes there).

### What's deliberately out of scope

- **Executor declaration in fingerprint.** A step's executor (`kind: shell, command: "..."`) arguably should be part of the fingerprint — different executor = different work. But that's a contract change to BuildExecutor's identity. Defer to a future sprint when rakia-executor exists and we know what shape executor identity should take.
- **Working-tree mode.** Some operators may want to fingerprint based on uncommitted changes (dev-loop "what does the current state hash to?"). Not in scope — must be tree-at-commit for reproducibility. A `--working-tree` flag could be added later as an explicit opt-out of reproducibility guarantees.
- **Caching.** Fingerprinting reads many blobs. For a 1000-file repo this is bounded. Larger trees may benefit from caching, but defer until profiling shows it matters.
- **Submodule and symlink handling.** Gix tree entries can be commit (submodule) or symlink modes. For Stage 1, skip both — most build-manifest source patterns target regular files in regular directories. Document the limitation.

### Compatibility note

The fingerprint hex string changes shape and value. Any caller comparing fingerprints across the boundary of this commit will see a discontinuity. There's no consumer doing that today (the placeholder fingerprints in build_plan have never been load-bearing). After this plan lands, the new fingerprints become the contract. Recording for the sprint-result.

### P2P Design Gate Classification

This plan touches one new entity (`ContentFingerprint` semantics) and refines two existing ones (`BuildPlan.fingerprint` field, `BuildPlan.tool` parameter shape). All inherit the architectural placement set by the predecessor sprint (`docs/superpowers/specs/2026-04-19-rakia-describable-cli-and-schema-ioc.md` — see its P2P Design Gate Classification section).

| Entity | This sprint | Stage 2 (rakia-peer) |
|---|---|---|
| **ContentFingerprint (BritCid hex)** | Operational (C) — derived deterministically from git tree at a commit + glob patterns. No storage; computed on demand. **Content-derived address**: the value IS the address (BritCid by construction). | **Notarized-attestation anchor (A2 derived)** — the per-peer Build attestation references `(plan_fingerprint, step_fingerprint)` as the work-unit identity. Same content + same patterns + same commit MUST yield identical fingerprints across peers; that's the dispatch coordination primitive this sprint enables. |
| **BuildPlan.fingerprint field** (already defined; this sprint just populates it correctly) | Operational (C) — string field on the BuildPlan output | At Stage 2 becomes load-bearing identity for per-step attestation anchoring |
| **rakia-core::build_plan::to_build_plan API change** | Pure refactor — accepts fingerprints as data instead of computing them. Keeps rakia-core IO-free (architectural cleanliness — IO bearer is brit-cli). | Same — API stays pure; brit-cli and future executor compute fingerprints with repo handle |

**Source of truth (this sprint):** Computation. The fingerprint is not stored anywhere — it's recomputed on demand from `(repo, commit, patterns)`. That's intentional: storage of computed values violates content-addressing's invariant (the value IS the address; storing it is redundant unless you're caching for performance).

**Source of truth (Stage 2):** Same — computation. Per-peer Build attestations REFERENCE the fingerprint hex but don't store the input data. Re-derivation is always possible from `(commit, patterns)`.

**Anti-pattern check:**
- *UUID for content-addressed entity*: avoided — fingerprint IS the address, BritCid is the type, no synthetic ID.
- *REST route as design starting point*: not applicable — CLI command, no HTTP.
- *Missing source-of-truth declaration*: this section is the declaration.
- *Putting granular data on the DHT*: not applicable — fingerprint is a 64-char hex string, well under any DHT entry size limit, and at Stage 2 it's referenced by attestations (not stored as a standalone entry).

**Anti-pattern note specific to this work:** the placeholder `compute_fingerprint` in rakia-core was a *hash that wasn't an address* — deterministic but not content-derived. That's an anti-pattern in a content-addressed system: it produces stable values that don't have content-identity semantics, so callers might treat them AS identifiers when they aren't. This plan deletes that anti-pattern.

**Implication for the trajectory:** Once this plan lands, Stage 2's "artifact X verified by N peers" composition is unblocked. Per-peer Build attestations can reference `(plan.planFingerprint, step.qualifiedName, step.fingerprint, step.outcome)` as the witnessed unit. The rakia-runnable sprint that follows can build the executor without needing to revisit the fingerprint primitive.

---

## File Structure

### New files

```
elohim/brit/brit-graph/src/repo_fingerprint.rs    # NEW (feature-gated)
elohim/brit/brit-graph/tests/repo_fingerprint.rs  # NEW (feature-gated test)
docs/superpowers/sprint-results/2026-04-19-brit-content-fingerprint.md  # NEW (artifact)
```

### Modified files

```
elohim/brit/brit-graph/Cargo.toml                  # MODIFY (add gix opt dep + `repo` feature)
elohim/brit/brit-graph/src/lib.rs                  # MODIFY (cfg gate the new module)
elohim/brit/brit-graph/src/fingerprint.rs          # MODIFY (add FingerprintError type used by repo_fingerprint)
elohim/brit/brit-cli/Cargo.toml                    # MODIFY (enable brit-graph `repo` feature; add gix dep for repo handle)
elohim/brit/brit-cli/src/commands/fingerprint.rs   # MODIFY (use from_repo_globs, accept --commit)
elohim/brit/brit-cli/src/main.rs                   # MODIFY (add --commit arg to fingerprint subcommand)
elohim/brit/brit-cli/src/commands/plan.rs          # MODIFY (compute fingerprints, pass to to_build_plan)
elohim/rakia/rakia-core/src/build_plan.rs          # MODIFY (delete compute_fingerprint, add fingerprints param)
elohim/rakia/rakia-core/tests/build_plan_schema_contract.rs  # MODIFY (provide fingerprints in tests)
elohim/rakia/rakia-core/tests/fixture_runner.rs    # MODIFY (provide fingerprints to to_build_plan)
elohim/brit/brit-cli/tests/cli_smoke.rs            # MODIFY (smoke fingerprint with new --commit semantics)
```

### File responsibilities

- **`repo_fingerprint.rs`** — the new constructor. ~80 lines. Walks tree, matches globs, reads blobs, builds inputs map, calls existing compute. Lives separate from `fingerprint.rs` because it has different dependency surface (gix) and to keep `fingerprint.rs` minimal/pure.
- **`fingerprint.rs`** — keeps the pure ContentFingerprint::compute. Gains a `FingerprintError` enum (used by repo_fingerprint, but defined in the parent module so it's always exported — even consumers without the `repo` feature can match on it).
- **`brit-cli/src/commands/fingerprint.rs`** — orchestration: open repo, resolve commit, gather patterns per step, call from_repo_globs, format JSON.
- **`brit-cli/src/commands/plan.rs`** — same plus the constellation walk: for each QualifiedStep in the plan, gather its source_patterns + build_process, compute fingerprint, build the qualified_name → hex map, pass to to_build_plan.
- **`rakia-core/src/build_plan.rs`** — pure conversion now. Accepts the fingerprints map; if a qualified_name is missing from the map, the corresponding PlannedStep gets the empty string `""` (schema permits, since fingerprint is `string` not constrained). Document this as the "no fingerprint context available" sentinel.

---

## Phase 1: brit-graph repo fingerprint extension

### Task 1: Add gix as feature-gated dep + scaffold module

**Files:**
- Modify: `/home/matthew/git/elohim/elohim/brit/brit-graph/Cargo.toml`
- Modify: `/home/matthew/git/elohim/elohim/brit/brit-graph/src/lib.rs`
- Modify: `/home/matthew/git/elohim/elohim/brit/brit-graph/src/fingerprint.rs` (add error type)
- Create: `/home/matthew/git/elohim/elohim/brit/brit-graph/src/repo_fingerprint.rs`

- [ ] **Step 1.1: Edit brit-graph Cargo.toml — add gix optional dep + `repo` feature**

Open `/home/matthew/git/elohim/elohim/brit/brit-graph/Cargo.toml`. Find the `[dependencies]` section. Add:

```toml
globset = { version = "0.4", optional = true }
gix = { version = "0.81", default-features = false, features = ["basic", "blob-diff", "sha1"], optional = true }
```

Add a `[features]` section (or extend existing):

```toml
[features]
default = []
repo = ["dep:gix", "dep:globset"]
```

(`gix` features mirror what `rakia-brit/Cargo.toml` uses for compatibility.)

- [ ] **Step 1.2: Add FingerprintError to fingerprint.rs**

Open `/home/matthew/git/elohim/elohim/brit/brit-graph/src/fingerprint.rs`. Add at the top of the file (after the existing imports, before `pub struct ContentFingerprint`):

```rust
/// Errors produced by repo-aware fingerprint construction.
///
/// This enum is exported from `brit-graph` regardless of the `repo` feature
/// so that downstream code can match on it without conditional compilation.
/// The variants that wrap gix or globset errors only get instantiated when
/// the `repo` feature is enabled and `ContentFingerprint::from_repo_globs`
/// is called.
#[derive(Debug, thiserror::Error)]
pub enum FingerprintError {
    /// Glob pattern compilation failed.
    #[error("invalid glob pattern '{pattern}': {message}")]
    InvalidGlob {
        /// The pattern that failed to compile.
        pattern: String,
        /// The underlying error message.
        message: String,
    },
    /// Resolving the commit ref failed.
    #[error("failed to resolve commit '{commit}': {message}")]
    CommitResolve {
        /// The ref or SHA being resolved.
        commit: String,
        /// The underlying error message.
        message: String,
    },
    /// Walking the git tree failed.
    #[error("tree walk failed: {0}")]
    TreeWalk(String),
    /// Reading a blob's bytes failed.
    #[error("failed to read blob at '{path}': {message}")]
    BlobRead {
        /// The repo-relative path whose blob couldn't be read.
        path: String,
        /// The underlying error message.
        message: String,
    },
    /// Path was not valid UTF-8.
    #[error("non-UTF-8 path in tree: {0:?}")]
    NonUtf8Path(Vec<u8>),
}
```

Add `thiserror = "2"` to `[dependencies]` if it's not already present. Check first:

```bash
grep '^thiserror' /home/matthew/git/elohim/elohim/brit/brit-graph/Cargo.toml
```

If absent, add to `[dependencies]`:
```toml
thiserror = "2"
```

(Looking at the existing brit workspace, `thiserror = "2"` is in `brit-cli/Cargo.toml`. brit-graph may already have it — do not duplicate.)

- [ ] **Step 1.3: Create repo_fingerprint.rs scaffold (gated)**

Create `/home/matthew/git/elohim/elohim/brit/brit-graph/src/repo_fingerprint.rs`:

```rust
//! Repo-aware fingerprint constructor (feature: `repo`).
//!
//! Builds a `ContentFingerprint` from file contents resolved through glob
//! patterns against a git tree at a specific commit. Reads blobs from the
//! tree, NOT the working tree — fingerprints are reproducible across machines
//! given the same commit and patterns.

use std::collections::BTreeMap;

use globset::{Glob, GlobSetBuilder};

use crate::fingerprint::{ContentFingerprint, FingerprintError};

impl ContentFingerprint {
    /// Compute a fingerprint by reading files from a git tree at a specific
    /// commit, matching against the given glob patterns.
    ///
    /// Files are read from the git tree (not the working tree) for
    /// reproducibility. Same commit + same patterns = same fingerprint,
    /// regardless of working-tree state.
    ///
    /// Submodule entries and symlinks in the tree are skipped (only regular
    /// blobs and executable blobs are included). Empty pattern set or no
    /// matching files produces a stable empty-input fingerprint.
    pub fn from_repo_globs(
        repo: &gix::Repository,
        commit_id: gix::ObjectId,
        patterns: &[String],
    ) -> Result<Self, FingerprintError> {
        // Step A: build the GlobSet
        let mut builder = GlobSetBuilder::new();
        for pattern in patterns {
            let glob = Glob::new(pattern).map_err(|e| FingerprintError::InvalidGlob {
                pattern: pattern.clone(),
                message: e.to_string(),
            })?;
            builder.add(glob);
        }
        let globset = builder
            .build()
            .map_err(|e| FingerprintError::InvalidGlob {
                pattern: patterns.join(", "),
                message: e.to_string(),
            })?;

        // Step B: open the tree at the commit
        let object = repo
            .find_object(commit_id)
            .map_err(|e| FingerprintError::CommitResolve {
                commit: commit_id.to_string(),
                message: e.to_string(),
            })?;
        let commit = object
            .try_into_commit()
            .map_err(|e| FingerprintError::CommitResolve {
                commit: commit_id.to_string(),
                message: format!("not a commit: {e}"),
            })?;
        let tree = commit
            .tree()
            .map_err(|e| FingerprintError::TreeWalk(e.to_string()))?;

        // Step C: walk the tree, collect matching (path, blob_bytes)
        let mut inputs: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        let mut walk_errors: Vec<String> = Vec::new();

        tree.traverse()
            .breadthfirst(&mut TreeCollector {
                repo,
                globset: &globset,
                inputs: &mut inputs,
                errors: &mut walk_errors,
                path_prefix: Vec::new(),
            })
            .map_err(|e| FingerprintError::TreeWalk(e.to_string()))?;

        if !walk_errors.is_empty() {
            return Err(FingerprintError::TreeWalk(walk_errors.join("; ")));
        }

        // Step D: delegate to existing pure compute
        Ok(Self::compute(&inputs))
    }
}

/// Visitor that walks a tree, matches paths against globs, and collects
/// blob contents for matching files.
struct TreeCollector<'a> {
    repo: &'a gix::Repository,
    globset: &'a globset::GlobSet,
    inputs: &'a mut BTreeMap<String, Vec<u8>>,
    errors: &'a mut Vec<String>,
    path_prefix: Vec<u8>,
}

impl<'a> gix::traverse::tree::Visit for TreeCollector<'a> {
    fn pop_back_tracked_path_and_set_current(&mut self) {}
    fn pop_front_tracked_path_and_set_current(&mut self) {}
    fn push_back_tracked_path_component(&mut self, _: &gix::bstr::BStr) {}
    fn push_path_component(&mut self, component: &gix::bstr::BStr) {
        if !self.path_prefix.is_empty() {
            self.path_prefix.push(b'/');
        }
        self.path_prefix.extend_from_slice(component);
    }
    fn pop_path_component(&mut self) {
        if let Some(slash_pos) = self.path_prefix.iter().rposition(|&b| b == b'/') {
            self.path_prefix.truncate(slash_pos);
        } else {
            self.path_prefix.clear();
        }
    }

    fn visit_tree(&mut self, _: &gix::objs::tree::EntryRef<'_>) -> gix::traverse::tree::visit::Action {
        gix::traverse::tree::visit::Action::Continue
    }

    fn visit_nontree(&mut self, entry: &gix::objs::tree::EntryRef<'_>) -> gix::traverse::tree::visit::Action {
        // Skip submodules and symlinks
        if !matches!(entry.mode.kind(), gix::objs::tree::EntryKind::Blob | gix::objs::tree::EntryKind::BlobExecutable) {
            return gix::traverse::tree::visit::Action::Continue;
        }

        // Build full path
        let mut full_path = self.path_prefix.clone();
        if !full_path.is_empty() {
            full_path.push(b'/');
        }
        full_path.extend_from_slice(entry.filename);

        // UTF-8 conversion
        let path_str = match std::str::from_utf8(&full_path) {
            Ok(s) => s.to_string(),
            Err(_) => {
                self.errors.push(format!("non-utf8 path: {:?}", full_path));
                return gix::traverse::tree::visit::Action::Continue;
            }
        };

        // Glob match against the path
        if !self.globset.is_match(&path_str) {
            return gix::traverse::tree::visit::Action::Continue;
        }

        // Read the blob
        match self.repo.find_object(entry.oid) {
            Ok(obj) => {
                let bytes = obj.data.clone();
                self.inputs.insert(path_str, bytes);
            }
            Err(e) => {
                self.errors.push(format!("read {}: {e}", path_str));
            }
        }

        gix::traverse::tree::visit::Action::Continue
    }
}
```

(The exact gix tree traversal API may differ slightly across versions. If `gix::traverse::tree::Visit` trait isn't shaped as above for gix 0.81, adapt to whatever the actual trait is. Verify via `cargo doc --open -p gix` or the gix source. The principle stays: depth-first walk, accumulate `path -> blob bytes` for entries matching the GlobSet.)

- [ ] **Step 1.4: Wire into lib.rs (gated)**

Open `/home/matthew/git/elohim/elohim/brit/brit-graph/src/lib.rs`. Find the existing `pub mod` declarations. Add:

```rust
#[cfg(feature = "repo")]
pub mod repo_fingerprint;
```

(Place after `pub mod fingerprint;` — they belong together.)

- [ ] **Step 1.5: Build with default features (gix should NOT be pulled in)**

```bash
cd /home/matthew/git/elohim/elohim/brit
RUSTFLAGS="" cargo build -p brit-graph 2>&1 | tail -10
```

Expected: clean build, no gix in the dependency closure (since `repo` feature is off by default).

- [ ] **Step 1.6: Build with repo feature (gix gets pulled in)**

```bash
cd /home/matthew/git/elohim/elohim/brit
RUSTFLAGS="" cargo build -p brit-graph --features repo 2>&1 | tail -15
```

Expected: clean build. If the `gix::traverse::tree::Visit` trait shape is different, compile errors will guide the adjustment. Iterate on the visitor signature until it compiles.

- [ ] **Step 1.7: Verify existing brit-graph tests still pass under both feature configs**

```bash
cd /home/matthew/git/elohim/elohim/brit
RUSTFLAGS="" cargo test -p brit-graph 2>&1 | grep "test result"
RUSTFLAGS="" cargo test -p brit-graph --features repo 2>&1 | grep "test result"
```

Expected: same test count under both (the new module has no tests yet — Task 2 adds them).

- [ ] **Step 1.8: Commit**

```bash
cd /home/matthew/git/elohim/elohim/brit
git checkout -b feat/brit-content-fingerprint
git status
git add brit-graph/Cargo.toml brit-graph/src/lib.rs brit-graph/src/fingerprint.rs brit-graph/src/repo_fingerprint.rs
git add Cargo.lock   # if changed
git commit -m "feat(brit-graph): scaffold repo_fingerprint module + FingerprintError (feature: repo)"
```

---

### Task 2: Test ContentFingerprint::from_repo_globs against fixtures

**Files:**
- Create: `/home/matthew/git/elohim/elohim/brit/brit-graph/tests/repo_fingerprint.rs`

The test uses a real (temp) git repo with known content so we can assert specific fingerprint properties.

- [ ] **Step 2.1: Create the test file**

`/home/matthew/git/elohim/elohim/brit/brit-graph/tests/repo_fingerprint.rs`:

```rust
//! Integration tests for ContentFingerprint::from_repo_globs.
//! These run only when the `repo` feature is enabled.

#![cfg(feature = "repo")]

use std::collections::BTreeMap;
use std::process::Command;

use brit_graph::fingerprint::ContentFingerprint;
use tempfile::TempDir;

/// Initialize a temp git repo with a few files committed.
/// Returns (TempDir keep-alive, repo path, head ObjectId).
fn init_repo_with_files(files: &[(&str, &str)]) -> (TempDir, std::path::PathBuf, gix::ObjectId) {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().to_path_buf();
    Command::new("git").args(["init", "-q"]).current_dir(&path).status().expect("init");
    Command::new("git").args(["config", "user.email", "t@t.t"]).current_dir(&path).status().expect("");
    Command::new("git").args(["config", "user.name", "t"]).current_dir(&path).status().expect("");

    for (rel, contents) in files {
        let abs = path.join(rel);
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(&abs, contents).expect("write");
        Command::new("git").args(["add", rel]).current_dir(&path).status().expect("add");
    }
    Command::new("git")
        .args(["commit", "-q", "-m", "init"])
        .current_dir(&path)
        .status()
        .expect("commit");

    let repo = gix::open(&path).expect("open");
    let head = repo
        .head_id()
        .expect("head_id");
    let head_id = head.detach();

    (dir, path, head_id)
}

#[test]
fn empty_patterns_produces_empty_inputs_fingerprint() {
    let (_keep, path, head) = init_repo_with_files(&[("a.txt", "hello\n")]);
    let repo = gix::open(&path).expect("open");
    let fp = ContentFingerprint::from_repo_globs(&repo, head, &[]).expect("compute");
    assert!(fp.inputs.is_empty(), "no patterns -> no inputs");

    // Same as compute(empty)
    let baseline = ContentFingerprint::compute(&BTreeMap::new());
    assert_eq!(fp.cid, baseline.cid);
}

#[test]
fn single_pattern_matches_one_file() {
    let (_keep, path, head) = init_repo_with_files(&[
        ("src/foo.ts", "console.log('foo');\n"),
        ("src/bar.rs", "fn bar() {}\n"),
        ("README.md", "# project\n"),
    ]);
    let repo = gix::open(&path).expect("open");

    let patterns = vec!["src/**/*.ts".to_string()];
    let fp = ContentFingerprint::from_repo_globs(&repo, head, &patterns).expect("compute");

    // Only foo.ts should be in the inputs
    assert_eq!(fp.inputs.len(), 1, "one .ts file");
    assert!(fp.inputs.contains_key("src/foo.ts"));
}

#[test]
fn deterministic_across_calls_same_inputs() {
    let (_keep, path, head) = init_repo_with_files(&[
        ("src/a.ts", "a\n"),
        ("src/b.ts", "b\n"),
    ]);
    let repo = gix::open(&path).expect("open");

    let patterns = vec!["src/**/*.ts".to_string()];
    let fp1 = ContentFingerprint::from_repo_globs(&repo, head, &patterns).expect("1");
    let fp2 = ContentFingerprint::from_repo_globs(&repo, head, &patterns).expect("2");

    assert_eq!(fp1.cid, fp2.cid, "deterministic");
    assert_eq!(fp1.inputs.len(), fp2.inputs.len());
}

#[test]
fn different_content_different_fingerprint() {
    // Same patterns, same paths, different file CONTENT -> different fingerprint.
    // This is the property that the OLD pattern-bytes hashing did NOT have.
    let (_keep_a, path_a, head_a) = init_repo_with_files(&[("src/foo.ts", "version 1\n")]);
    let (_keep_b, path_b, head_b) = init_repo_with_files(&[("src/foo.ts", "version 2\n")]);

    let repo_a = gix::open(&path_a).expect("a");
    let repo_b = gix::open(&path_b).expect("b");

    let patterns = vec!["src/**/*.ts".to_string()];
    let fp_a = ContentFingerprint::from_repo_globs(&repo_a, head_a, &patterns).expect("a");
    let fp_b = ContentFingerprint::from_repo_globs(&repo_b, head_b, &patterns).expect("b");

    assert_ne!(fp_a.cid, fp_b.cid, "different content must produce different fingerprint");
}

#[test]
fn no_matching_files_is_empty_fingerprint() {
    let (_keep, path, head) = init_repo_with_files(&[("README.md", "x")]);
    let repo = gix::open(&path).expect("open");
    let patterns = vec!["src/**/*.ts".to_string()];
    let fp = ContentFingerprint::from_repo_globs(&repo, head, &patterns).expect("compute");
    assert!(fp.inputs.is_empty());
}

#[test]
fn multiple_patterns_combine() {
    let (_keep, path, head) = init_repo_with_files(&[
        ("src/foo.ts", "ts\n"),
        ("src/bar.rs", "rs\n"),
        ("README.md", "md\n"),
    ]);
    let repo = gix::open(&path).expect("open");
    let patterns = vec!["src/**/*.ts".to_string(), "src/**/*.rs".to_string()];
    let fp = ContentFingerprint::from_repo_globs(&repo, head, &patterns).expect("compute");
    assert_eq!(fp.inputs.len(), 2);
    assert!(fp.inputs.contains_key("src/foo.ts"));
    assert!(fp.inputs.contains_key("src/bar.rs"));
}

#[test]
fn invalid_glob_returns_error() {
    let (_keep, path, head) = init_repo_with_files(&[("a.txt", "x")]);
    let repo = gix::open(&path).expect("open");
    let patterns = vec!["[invalid".to_string()];
    let err = ContentFingerprint::from_repo_globs(&repo, head, &patterns).unwrap_err();
    assert!(matches!(err, brit_graph::fingerprint::FingerprintError::InvalidGlob { .. }));
}
```

- [ ] **Step 2.2: Add tempfile to brit-graph dev-deps if not already present**

```bash
grep '^tempfile' /home/matthew/git/elohim/elohim/brit/brit-graph/Cargo.toml
```

If not present, edit `/home/matthew/git/elohim/elohim/brit/brit-graph/Cargo.toml` and add to `[dev-dependencies]`:

```toml
tempfile = "3"
```

Also add `gix` to dev-dependencies (since the test uses it directly outside the optional path):

```toml
gix = { version = "0.81", default-features = false, features = ["basic", "blob-diff", "sha1"] }
```

(Test files are always built — they need gix unconditionally, while the library code only needs it under `--features repo`.)

- [ ] **Step 2.3: Run the tests**

```bash
cd /home/matthew/git/elohim/elohim/brit
RUSTFLAGS="" cargo test -p brit-graph --features repo --test repo_fingerprint 2>&1 | tail -25
```

Expected: 7/7 pass.

If a test fails because the visitor walks paths differently than expected (e.g., paths come back without leading directory), inspect the failure and adjust either `repo_fingerprint.rs`'s path-building OR the test expectation. The CONTRACT (path = repo-relative, forward-slash, no leading slash) should be preserved.

- [ ] **Step 2.4: Verify build passes without `repo` feature too**

```bash
cd /home/matthew/git/elohim/elohim/brit
RUSTFLAGS="" cargo build -p brit-graph 2>&1 | tail -5
RUSTFLAGS="" cargo test -p brit-graph 2>&1 | grep "test result" | head -5
```

Expected: builds clean, existing tests pass, the new repo_fingerprint test file is silently excluded (cfg gate).

- [ ] **Step 2.5: Commit**

```bash
cd /home/matthew/git/elohim/elohim/brit
git status
git add brit-graph/Cargo.toml brit-graph/tests/repo_fingerprint.rs
git add Cargo.lock   # if changed
git commit -m "test(brit-graph): repo fingerprint — 7 cases covering empty, single, multi, deterministic, content-sensitivity, error"
```

---

## Phase 2: brit-cli fingerprint subcommand upgrade

### Task 3: Update brit-cli to enable repo feature + add gix dep

**Files:**
- Modify: `/home/matthew/git/elohim/elohim/brit/brit-cli/Cargo.toml`

- [ ] **Step 3.1: Enable brit-graph's `repo` feature**

Open `/home/matthew/git/elohim/elohim/brit/brit-cli/Cargo.toml`. Find the `brit-graph` dependency line. Change from:

```toml
brit-graph = { path = "../brit-graph" }
```

to:

```toml
brit-graph = { path = "../brit-graph", features = ["repo"] }
```

- [ ] **Step 3.2: Add gix dep (brit-cli needs it directly to open the repo + resolve commits)**

Add to `[dependencies]`:

```toml
gix = { version = "0.81", default-features = false, features = ["basic", "revision", "blob-diff", "sha1"] }
```

(Same features as `rakia-brit/Cargo.toml`, with `revision` added for ref-parsing support that --commit needs.)

- [ ] **Step 3.3: Build to confirm**

```bash
cd /home/matthew/git/elohim/elohim/brit
RUSTFLAGS="" cargo build -p brit-cli 2>&1 | tail -10
```

Expected: clean.

- [ ] **Step 3.4: Commit**

```bash
cd /home/matthew/git/elohim/elohim/brit
git status
git add brit-cli/Cargo.toml Cargo.lock
git commit -m "chore(brit-cli): enable brit-graph repo feature + add gix dep"
```

---

### Task 4: Rewrite brit-cli fingerprint subcommand

**Files:**
- Modify: `/home/matthew/git/elohim/elohim/brit/brit-cli/src/main.rs` (add --commit flag)
- Modify: `/home/matthew/git/elohim/elohim/brit/brit-cli/src/commands/fingerprint.rs`

- [ ] **Step 4.1: Add --commit flag to FingerprintArgs**

Open `/home/matthew/git/elohim/elohim/brit/brit-cli/src/main.rs`. Find the `FingerprintArgs` struct. Replace with:

```rust
#[derive(clap::Args)]
struct FingerprintArgs {
    /// Path to a build-manifest.json
    manifest: PathBuf,
    /// Specific step name (default: all steps in the manifest)
    #[arg(long)]
    step: Option<String>,
    /// Git ref or SHA to fingerprint against (default: HEAD)
    #[arg(long, default_value = "HEAD")]
    commit: String,
}
```

Also update the dispatch in `run()` — find the line that calls `commands::fingerprint::run` and pass the commit too:

```rust
        Command::Fingerprint(args) => commands::fingerprint::run(&args.manifest, args.step.as_deref(), &args.commit),
```

- [ ] **Step 4.2: Rewrite fingerprint.rs to use from_repo_globs**

Replace `/home/matthew/git/elohim/elohim/brit/brit-cli/src/commands/fingerprint.rs`:

```rust
//! brit fingerprint — content-addressed hash of step inputs.
//!
//! Resolves each step's source + buildProcess globs against the git tree at
//! the given commit (default HEAD), reads matching blob contents, and computes
//! a deterministic ContentFingerprint per step.

use std::path::Path;

use serde::Serialize;

use crate::error::{CliError, Result};

#[derive(Serialize)]
struct FingerprintOutput {
    manifest: String,
    commit: String,
    fingerprints: Vec<StepFingerprint>,
}

#[derive(Serialize)]
struct StepFingerprint {
    pipeline: String,
    step: String,
    fingerprint: String,
    input_count: usize,
}

pub fn run(manifest_path: &Path, step_filter: Option<&str>, commit_ref: &str) -> Result<()> {
    let text = std::fs::read_to_string(manifest_path)?;
    let m: rakia_core::manifest::BuildManifest = serde_json::from_str(&text)?;

    // Discover the repo from the manifest's parent dir
    let manifest_dir = manifest_path
        .parent()
        .ok_or_else(|| CliError::Args(format!("manifest has no parent dir: {}", manifest_path.display())))?;
    let repo = gix::discover(manifest_dir)
        .map_err(|e| CliError::Baseline(format!("repo discovery failed for {}: {e}", manifest_dir.display())))?;

    // Resolve the commit ref to an ObjectId
    let commit_id = repo
        .rev_parse_single(commit_ref)
        .map_err(|e| CliError::Args(format!("could not resolve commit '{commit_ref}': {e}")))?
        .detach();

    let mut out = Vec::new();
    for (name, step) in &m.steps {
        if let Some(filter) = step_filter {
            if name != filter {
                continue;
            }
        }
        let mut all_patterns: Vec<String> = step.inputs.sources.clone();
        all_patterns.extend(step.inputs.build_process.iter().cloned());

        let fp = brit_graph::fingerprint::ContentFingerprint::from_repo_globs(
            &repo,
            commit_id,
            &all_patterns,
        )
        .map_err(|e| CliError::Args(format!("fingerprint compute failed for step '{name}': {e}")))?;

        out.push(StepFingerprint {
            pipeline: m.pipeline.clone(),
            step: name.clone(),
            fingerprint: fp.cid.as_str().to_string(),
            input_count: fp.inputs.len(),
        });
    }

    crate::output::print_json(&FingerprintOutput {
        manifest: manifest_path.display().to_string(),
        commit: commit_id.to_string(),
        fingerprints: out,
    })?;
    Ok(())
}
```

- [ ] **Step 4.3: Build**

```bash
cd /home/matthew/git/elohim/elohim/brit
RUSTFLAGS="" cargo build -p brit-cli 2>&1 | tail -10
```

Expected: clean.

- [ ] **Step 4.4: Smoke test against the live elohim repo**

```bash
B=/home/matthew/git/elohim/elohim/brit/target/debug/brit
$B fingerprint /home/matthew/git/elohim/app/elohim-app/build-manifest.json
echo "---"
$B fingerprint /home/matthew/git/elohim/app/elohim-app/build-manifest.json --step build-angular
echo "---"
# Test --commit flag with a previous commit
$B fingerprint /home/matthew/git/elohim/app/elohim-app/build-manifest.json --step build-angular --commit HEAD~5
```

Expected:
- First two calls show fingerprints with `input_count` > 0 (real files matched against real globs)
- Third call may show different fingerprint (different commit = different file contents)

If `input_count` is 0 for steps that should have files (e.g., build-angular's `src/**/*.ts` glob), the visitor's path-building has a bug — debug the path encoding (likely a forward-slash or repo-prefix issue).

- [ ] **Step 4.5: Verify determinism**

```bash
B=/home/matthew/git/elohim/elohim/brit/target/debug/brit
diff \
  <($B fingerprint /home/matthew/git/elohim/app/elohim-app/build-manifest.json) \
  <($B fingerprint /home/matthew/git/elohim/app/elohim-app/build-manifest.json) \
  && echo "deterministic: OK" || echo "FAIL: outputs differ"
```

Expected: "deterministic: OK"

- [ ] **Step 4.6: Verify content-sensitivity (commit X != commit Y)**

```bash
B=/home/matthew/git/elohim/elohim/brit/target/debug/brit
FP_HEAD=$($B fingerprint /home/matthew/git/elohim/app/elohim-app/build-manifest.json --step build-angular --commit HEAD | jq -r '.fingerprints[0].fingerprint')
FP_PREV=$($B fingerprint /home/matthew/git/elohim/app/elohim-app/build-manifest.json --step build-angular --commit HEAD~10 | jq -r '.fingerprints[0].fingerprint')
echo "HEAD:    $FP_HEAD"
echo "HEAD~10: $FP_PREV"
[ "$FP_HEAD" != "$FP_PREV" ] && echo "content-sensitive: OK" || echo "WARN: same fingerprint at different commits (could be legitimate if no build-angular sources changed)"
```

Different fingerprints expected if any source file in elohim-app changed between HEAD~10 and HEAD. Identical is acceptable if (legitimately) no source matching the patterns changed.

- [ ] **Step 4.7: Commit**

```bash
cd /home/matthew/git/elohim/elohim/brit
git status
git add brit-cli/src/main.rs brit-cli/src/commands/fingerprint.rs
git commit -m "feat(brit-cli): fingerprint reads file CONTENTS via from_repo_globs (was hashing pattern strings)"
```

---

### Task 5: Update brit-cli smoke test for new fingerprint output shape

**Files:**
- Modify: `/home/matthew/git/elohim/elohim/brit/brit-cli/tests/cli_smoke.rs`

- [ ] **Step 5.1: Inspect current smoke test**

```bash
cat /home/matthew/git/elohim/elohim/brit/brit-cli/tests/cli_smoke.rs
```

Note the existing test (`graph_discover_outputs_json_with_manifests`). The fingerprint subcommand may not have a smoke test today; if not, add one.

- [ ] **Step 5.2: Add a fingerprint smoke test**

Append to `/home/matthew/git/elohim/elohim/brit/brit-cli/tests/cli_smoke.rs`:

```rust
#[test]
fn fingerprint_emits_content_addressed_hex_for_real_manifest() {
    let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../").canonicalize().unwrap();

    let manifest = repo_root.join("app/elohim-app/build-manifest.json");
    if !manifest.exists() {
        // Skip if running outside the elohim repo
        return;
    }

    let out = std::process::Command::new(brit_binary())
        .args(["fingerprint"])
        .arg(&manifest)
        .args(["--step", "build-angular"])
        .output()
        .expect("invoke brit");

    assert!(out.status.success(),
        "exit {} stderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr));

    let stdout = String::from_utf8(out.stdout).expect("utf8");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("parse json");
    let fps = v["fingerprints"].as_array().expect("fingerprints array");
    assert_eq!(fps.len(), 1, "filtered to one step");

    let fp = &fps[0];
    assert_eq!(fp["step"], "build-angular");
    let hex = fp["fingerprint"].as_str().expect("fingerprint string");
    assert_eq!(hex.len(), 64, "blake3 hex is 64 chars");
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit()), "hex");
    let input_count = fp["input_count"].as_u64().expect("input_count");
    assert!(input_count > 0, "build-angular should match real source files");
}
```

(The 64-char length assertion comes from blake3's 256-bit hash → 64 hex chars. If the existing BritCid uses a different format, adjust the assertion.)

- [ ] **Step 5.3: Run the smoke test**

```bash
cd /home/matthew/git/elohim/elohim/brit
RUSTFLAGS="" cargo build -p brit-cli && RUSTFLAGS="" cargo test -p brit-cli fingerprint_emits 2>&1 | tail -10
```

Expected: 1/1 pass.

- [ ] **Step 5.4: Commit**

```bash
cd /home/matthew/git/elohim/elohim/brit
git status
git add brit-cli/tests/cli_smoke.rs
git commit -m "test(brit-cli): smoke test for content-addressed fingerprint output"
```

---

## Phase 3: rakia-core build_plan integration

### Task 6: Refactor to_build_plan to accept fingerprints map

**Files:**
- Modify: `/home/matthew/git/elohim/elohim/rakia/rakia-core/src/build_plan.rs`
- Modify: `/home/matthew/git/elohim/elohim/rakia/rakia-core/tests/build_plan_schema_contract.rs`
- Modify: `/home/matthew/git/elohim/elohim/rakia/rakia-core/tests/fixture_runner.rs`

- [ ] **Step 6.1: Update to_build_plan signature**

Open `/home/matthew/git/elohim/elohim/rakia/rakia-core/src/build_plan.rs`. Find the `to_build_plan` function.

Add a new parameter `fingerprints: &BTreeMap<String, String>` (qualified_name → hex). Delete the placeholder `compute_fingerprint` function. Look up each step's fingerprint from the map; if missing, use empty string `""` (the schema's `fingerprint: { type: string }` accepts empty).

```rust
use std::collections::BTreeMap;

use crate::constellation::{QualifiedStep, TopoPlan};
use crate::generated_types::{
    AffectedReason, Baseline, BuildPlan, Head, PlannedStep, ToolInfo,
};

/// Convert an internal TopoPlan + change-detection context into the
/// schema-conforming BuildPlan.
///
/// `fingerprints` — qualified_name -> BritCid hex string. Missing entries
/// produce an empty fingerprint (`""`). Callers that want content-addressed
/// fingerprints should populate this map via `brit_graph::fingerprint::
/// ContentFingerprint::from_repo_globs` (see brit-cli for the pattern).
pub fn to_build_plan(
    plan: &TopoPlan,
    baseline_ref: &str,
    baseline_commit: &str,
    head_commit: &str,
    changed_paths: &[String],
    tool_version: &str,
    fingerprints: &BTreeMap<String, String>,
) -> BuildPlan {
    let levels: Vec<Vec<PlannedStep>> = plan
        .levels
        .iter()
        .map(|level| {
            level
                .iter()
                .map(|(step, reasons)| PlannedStep {
                    pipeline: step.pipeline.clone(),
                    name: step.step_name.clone(),
                    qualified_name: step.qualified_name.clone(),
                    fingerprint: fingerprints
                        .get(&step.qualified_name)
                        .cloned()
                        .unwrap_or_default(),
                    depends: step.resolved_depends.clone(),
                    affected_by: reasons.clone(),
                })
                .collect()
        })
        .collect();

    BuildPlan {
        plan_version: "1.0".to_string(),
        baseline: Baseline {
            r#ref: baseline_ref.to_string(),
            commit: baseline_commit.to_string(),
        },
        head: Head { commit: head_commit.to_string() },
        changed_paths: Some(changed_paths.to_vec()),
        levels,
        generated_at: chrono::Utc::now().to_rfc3339(),
        tool: ToolInfo {
            name: "brit".to_string(),
            version: tool_version.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constellation::{QualifiedStep, TopoPlan};
    use crate::generated_types::{AffectedReason, Kind};
    use std::path::PathBuf;

    fn sample_step(name: &str) -> QualifiedStep {
        QualifiedStep {
            qualified_name: format!("p:{name}"),
            pipeline: "p".to_string(),
            step_name: name.to_string(),
            description: String::new(),
            source_patterns: vec![],
            build_process: vec![],
            artifacts: vec![],
            resolved_depends: vec![],
            manifest_path: PathBuf::new(),
        }
    }

    #[test]
    fn empty_plan_produces_empty_levels() {
        let plan = TopoPlan { levels: vec![] };
        let fps = BTreeMap::new();
        let bp = to_build_plan(
            &plan,
            "refs/x",
            &"0".repeat(40),
            &"1".repeat(40),
            &[],
            "0.0.0",
            &fps,
        );
        assert_eq!(bp.plan_version, "1.0");
        assert!(bp.levels.is_empty());
    }

    #[test]
    fn fingerprint_from_map_is_propagated_into_planned_step() {
        let step = sample_step("build");
        let reason = AffectedReason {
            kind: Kind::ChangedFile,
            path: Some("src/foo.ts".to_string()),
            upstream: None,
        };
        let plan = TopoPlan { levels: vec![vec![(step, vec![reason])]] };
        let mut fps = BTreeMap::new();
        fps.insert("p:build".to_string(), "deadbeef".repeat(8));
        let bp = to_build_plan(
            &plan,
            "refs/x",
            &"a".repeat(40),
            &"b".repeat(40),
            &["src/foo.ts".to_string()],
            "0.0.0",
            &fps,
        );
        assert_eq!(bp.levels[0][0].fingerprint, "deadbeef".repeat(8));
    }

    #[test]
    fn missing_fingerprint_in_map_produces_empty_string() {
        let step = sample_step("build");
        let reason = AffectedReason {
            kind: Kind::ChangedFile,
            path: Some("src/foo.ts".to_string()),
            upstream: None,
        };
        let plan = TopoPlan { levels: vec![vec![(step, vec![reason])]] };
        let fps = BTreeMap::new();   // empty
        let bp = to_build_plan(
            &plan,
            "refs/x",
            &"a".repeat(40),
            &"b".repeat(40),
            &[],
            "0.0.0",
            &fps,
        );
        assert_eq!(bp.levels[0][0].fingerprint, "");
    }
}
```

- [ ] **Step 6.2: Update the schema contract test**

Open `/home/matthew/git/elohim/elohim/rakia/rakia-core/tests/build_plan_schema_contract.rs`. Find both `to_build_plan(...)` calls. Add a `&BTreeMap::new()` argument at the end:

```rust
use std::collections::BTreeMap;
// ... existing imports ...

#[test]
fn empty_plan_validates() {
    // ... existing setup ...
    let bp = to_build_plan(
        &plan,
        "refs/notes/rakia/baselines/test",
        &"a".repeat(40),
        &"b".repeat(40),
        &[],
        "0.0.0",
        &BTreeMap::new(),
    );
    // ... existing validate ...
}

#[test]
fn populated_plan_validates() {
    // ... existing setup ...
    let bp = to_build_plan(
        &plan,
        "refs/notes/rakia/baselines/elohim-app",
        &"a".repeat(40),
        &"b".repeat(40),
        &["src/foo.ts".to_string()],
        "0.1.0",
        &BTreeMap::new(),
    );
    // ... existing validate ...
}
```

- [ ] **Step 6.3: Update the fixture runner**

Open `/home/matthew/git/elohim/elohim/rakia/rakia-core/tests/fixture_runner.rs`. Find the `to_build_plan(...)` call inside `run_fixture`. Add the empty fingerprints map:

```rust
use std::collections::BTreeMap;
// ... existing imports ...

// inside run_fixture, the to_build_plan call:
    let bp = rakia_core::build_plan::to_build_plan(
        &plan,
        "refs/notes/rakia/baselines/fixture",
        &"a".repeat(40),
        &"b".repeat(40),
        &changed.paths,
        "0.0.0-fixture",
        &BTreeMap::new(),
    );
```

- [ ] **Step 6.4: Run all rakia-core tests**

```bash
cd /home/matthew/git/elohim/elohim/rakia
RUSTFLAGS="" cargo test -p rakia-core 2>&1 | tail -25
```

Expected: all tests pass (now with the new fingerprints param). Test count: same as before plus 2 new in build_plan.rs tests module = previous + 2.

- [ ] **Step 6.5: Commit**

```bash
cd /home/matthew/git/elohim/elohim/rakia
git checkout -b feat/build-plan-fingerprints-param
git status
git add rakia-core/src/build_plan.rs rakia-core/tests/build_plan_schema_contract.rs rakia-core/tests/fixture_runner.rs
git commit -m "refactor(rakia-core): to_build_plan accepts fingerprints map (deletes placeholder compute_fingerprint)"
```

---

### Task 7: brit-cli plan subcommand computes fingerprints

**Files:**
- Modify: `/home/matthew/git/elohim/elohim/brit/brit-cli/src/commands/plan.rs`

- [ ] **Step 7.1: Read current plan.rs**

```bash
cat /home/matthew/git/elohim/elohim/brit/brit-cli/src/commands/plan.rs
```

Note the `to_build_plan` call — it currently passes 6 args. After Task 6 it needs 7 (+ fingerprints map).

- [ ] **Step 7.2: Add fingerprint computation to plan.rs**

Open `/home/matthew/git/elohim/elohim/brit/brit-cli/src/commands/plan.rs`. After computing the plan and BEFORE calling to_build_plan, compute fingerprints. Replace the function with:

```rust
//! brit plan — topologically grouped build plan, conforming to build-plan.schema.json.

use std::collections::BTreeMap;
use std::path::Path;

use crate::error::{CliError, Result};

const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run(
    repo: &Path,
    files: Option<&str>,
    since: Option<&str>,
    pipeline: Option<&str>,
) -> Result<()> {
    let repo = repo.canonicalize().map_err(|source| CliError::RepoNotFound {
        path: repo.display().to_string(),
        source,
    })?;

    let (changed_paths, baseline_ref, baseline_commit, head_commit) = if let Some(files) = files {
        let paths: Vec<String> = files
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        (paths, "(none)".to_string(), "0".repeat(40), "0".repeat(40))
    } else if let Some(since) = since {
        let head_commit_sha = rakia_brit::changes::head_commit(&repo)
            .map_err(|e| CliError::ChangeDetection(format!("{e}")))?;
        let baseline_commit_sha = rakia_brit::changes::resolve_ref(&repo, since)
            .map_err(|e| CliError::ChangeDetection(format!("{e}")))?;
        let paths = rakia_brit::changes::changed_paths_since(&repo, since, "HEAD")
            .map_err(|e| CliError::ChangeDetection(format!("{e}")))?;
        let ref_name = if let Some(p) = pipeline {
            format!("refs/notes/rakia/baselines/{p}")
        } else {
            since.to_string()
        };
        (paths, ref_name, baseline_commit_sha, head_commit_sha)
    } else {
        return Err(CliError::Args("need --files or --since".into()));
    };

    let manifests = rakia_core::discover::discover_manifests(&repo)
        .map_err(|e| CliError::ManifestDiscovery(format!("{e}")))?;
    let constellation = rakia_core::constellation::build_constellation(&manifests)?;
    let plan = rakia_core::constellation::plan_from_changes(&constellation, &changed_paths)?;

    // Compute content-addressed fingerprints for each step in the plan
    // (only steps actually in the plan, not all steps in the constellation).
    let fingerprints = compute_fingerprints(&repo, &head_commit, &plan)?;

    let bp = rakia_core::build_plan::to_build_plan(
        &plan,
        &baseline_ref,
        &baseline_commit,
        &head_commit,
        &changed_paths,
        TOOL_VERSION,
        &fingerprints,
    );

    crate::output::print_json(&bp)?;
    Ok(())
}

/// Compute the ContentFingerprint for each step in the plan, keyed by
/// qualified_name. Uses the head commit as the tree to read from.
///
/// For --files mode, head_commit is "0"*40 (placeholder). Skip fingerprinting
/// in that case (returns empty map; PlannedStep.fingerprint will be "").
fn compute_fingerprints(
    repo_path: &Path,
    head_commit_hex: &str,
    plan: &rakia_core::constellation::TopoPlan,
) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();

    // Skip if we don't have a real commit to fingerprint against
    if head_commit_hex.chars().all(|c| c == '0') {
        return Ok(out);
    }

    let repo = gix::open(repo_path)
        .map_err(|e| CliError::Args(format!("repo open failed: {e}")))?;
    let commit_id: gix::ObjectId = head_commit_hex
        .parse()
        .map_err(|e| CliError::Args(format!("invalid commit hex '{head_commit_hex}': {e}")))?;

    for level in &plan.levels {
        for (step, _reasons) in level {
            let mut all_patterns: Vec<String> = step.source_patterns.clone();
            all_patterns.extend(step.build_process.iter().cloned());

            let fp = brit_graph::fingerprint::ContentFingerprint::from_repo_globs(
                &repo,
                commit_id,
                &all_patterns,
            )
            .map_err(|e| CliError::Args(format!(
                "fingerprint failed for step '{}': {e}", step.qualified_name
            )))?;

            out.insert(step.qualified_name.clone(), fp.cid.as_str().to_string());
        }
    }

    Ok(out)
}
```

- [ ] **Step 7.3: Build**

```bash
cd /home/matthew/git/elohim/elohim/brit
RUSTFLAGS="" cargo build -p brit-cli 2>&1 | tail -10
```

Expected: clean.

- [ ] **Step 7.4: Smoke test against fixture (real fingerprints in --since mode)**

```bash
TMP=$(mktemp -d)
mkdir -p "$TMP/proj-a"
git -C "$TMP" init -q
cat > "$TMP/proj-a/build-manifest.json" <<'EOF'
{
  "manifestVersion": "1.0",
  "pipeline": "proj-a",
  "description": "Project A",
  "steps": {
    "build": {
      "description": "Build A",
      "inputs": { "sources": ["proj-a/src/**/*.ts"], "buildProcess": [] },
      "outputs": { "artifacts": [], "verify": null },
      "depends": [],
      "executor": {}
    }
  }
}
EOF
mkdir -p "$TMP/proj-a/src"
echo "console.log('v1');" > "$TMP/proj-a/src/main.ts"
git -C "$TMP" add . && git -C "$TMP" -c user.email=t@t.t -c user.name=t commit -q -m init

# --since HEAD: real commit, real fingerprint
B=/home/matthew/git/elohim/elohim/brit/target/debug/brit
echo "--- --since HEAD ---"
$B plan --repo "$TMP" --since HEAD | jq '{planVersion, levels, baseline, head}'

# --files mode: placeholder fingerprint (empty string)
echo "--- --files ---"
$B plan --repo "$TMP" --files "proj-a/src/main.ts" | jq '.levels[0][0].fingerprint'

rm -rf "$TMP"
```

Expected:
- `--since HEAD` produces a plan; if there are changes since HEAD (there aren't here, just-init), levels is empty. To test fingerprint output, change a file and add another commit, then `--since HEAD~1`.
- `--files` mode: fingerprint is `""` (empty), since head_commit is "0"*40 placeholder.

For a richer test:

```bash
TMP=$(mktemp -d)
mkdir -p "$TMP/proj-a/src"
git -C "$TMP" init -q
cat > "$TMP/proj-a/build-manifest.json" <<'EOF'
{
  "manifestVersion": "1.0", "pipeline": "proj-a", "description": "P",
  "steps": {
    "build": {
      "description": "B", "inputs": { "sources": ["proj-a/src/**/*.ts"], "buildProcess": [] },
      "outputs": { "artifacts": [], "verify": null }, "depends": [], "executor": {}
    }
  }
}
EOF
echo "v1" > "$TMP/proj-a/src/main.ts"
git -C "$TMP" add . && git -C "$TMP" -c user.email=t@t.t -c user.name=t commit -q -m v1
echo "v2" > "$TMP/proj-a/src/main.ts"
git -C "$TMP" add . && git -C "$TMP" -c user.email=t@t.t -c user.name=t commit -q -m v2

B=/home/matthew/git/elohim/elohim/brit/target/debug/brit
echo "--- plan --since HEAD~1 ---"
$B plan --repo "$TMP" --since HEAD~1 | jq '.levels[][] | {qualifiedName, fingerprint}'
rm -rf "$TMP"
```

Expected: a non-empty fingerprint hex (64 chars) for `proj-a:build` (since `proj-a/src/main.ts` changed).

- [ ] **Step 7.5: Validate output against schema**

```bash
TMP=$(mktemp -d)
# (same fixture setup as above)
mkdir -p "$TMP/proj-a/src"
git -C "$TMP" init -q
cat > "$TMP/proj-a/build-manifest.json" <<'EOF'
{ "manifestVersion": "1.0", "pipeline": "proj-a", "description": "P",
  "steps": { "build": { "description": "B",
    "inputs": { "sources": ["proj-a/src/**/*.ts"], "buildProcess": [] },
    "outputs": { "artifacts": [], "verify": null }, "depends": [], "executor": {} } } }
EOF
echo "v1" > "$TMP/proj-a/src/main.ts"
git -C "$TMP" add . && git -C "$TMP" -c user.email=t@t.t -c user.name=t commit -q -m v1
echo "v2" > "$TMP/proj-a/src/main.ts"
git -C "$TMP" add . && git -C "$TMP" -c user.email=t@t.t -c user.name=t commit -q -m v2

B=/home/matthew/git/elohim/elohim/brit/target/debug/brit
$B plan --repo "$TMP" --since HEAD~1 > /tmp/plan_with_fp.json

# Validate via the local AJV setup (same as Task 17 used in the previous sprint)
cd /home/matthew/git/elohim
node -e "
const Ajv = require('ajv/dist/2020.js').default;
const addFormats = require('ajv-formats').default;
const ajv = new Ajv({allErrors: true, strict: false});
addFormats(ajv);
const schema = JSON.parse(require('fs').readFileSync('elohim/rakia/schemas/v1/build-plan.schema.json', 'utf8'));
const validate = ajv.compile(schema);
const data = JSON.parse(require('fs').readFileSync('/tmp/plan_with_fp.json', 'utf8'));
console.log(validate(data) ? 'PASS' : 'FAIL: ' + JSON.stringify(validate.errors, null, 2));
"
rm -rf "$TMP"
```

Expected: `PASS`. The 64-char blake3 hex fingerprint conforms to the schema's `fingerprint: { type: string }`.

- [ ] **Step 7.6: Commit (in brit submodule)**

```bash
cd /home/matthew/git/elohim/elohim/brit
git status
git add brit-cli/src/commands/plan.rs
git commit -m "feat(brit-cli): plan computes content-addressed fingerprints via from_repo_globs"
```

---

## Phase 4: Schema check + IoC close

### Task 8: Verify fixture runner still validates with new fingerprints (regression check)

**Files:**
- (no changes — this is a verification step that may surface a real issue)

- [ ] **Step 8.1: Re-run the rakia-core fixture runner**

```bash
cd /home/matthew/git/elohim/elohim/rakia
RUSTFLAGS="" cargo test -p rakia-core --test fixture_runner 2>&1 | tail -10
```

Expected: PASS. The fixture_runner uses `&BTreeMap::new()` (Task 6.3) — fingerprints are empty strings. The schema allows `fingerprint: ""` (it's just `type: string`).

If this fails, the schema may have a `pattern` or `minLength` constraint on `fingerprint` that rejects empty. Inspect:

```bash
jq '.["$defs"].plannedStep.properties.fingerprint' /home/matthew/git/elohim/elohim/rakia/schemas/v1/build-plan.schema.json
```

If the schema constrains fingerprint, two options:
- Loosen the schema (allow empty string explicitly) — preserves current contract for callers that don't have a repo
- Make `fingerprint` optional in the schema — but then Optional<String> in Rust, which changes the BuildPlan shape

Choose the loosen-schema option (allow empty string):
```json
"fingerprint": {
  "type": "string",
  "description": "BritCid hex — content-addressed hash of step inputs. Empty string when no repo context was available (e.g., --files mode without explicit fingerprinting)."
}
```

Then run `pnpm run rakia:codegen:rs` to regenerate (no struct change, but verify mode's checksum tracking).

- [ ] **Step 8.2: Re-run the schema validate**

```bash
cd /home/matthew/git/elohim
pnpm run rakia:schema:validate 2>&1 | tail -3
pnpm run rakia:codegen:rs:verify 2>&1 | tail -3
```

Expected: both pass.

- [ ] **Step 8.3: Commit the rakia-core changes (if any schema/regen happened)**

```bash
cd /home/matthew/git/elohim/elohim/rakia
git status
# If schema/generated_types changed:
git add schemas/v1/build-plan.schema.json rakia-core/src/generated_types.rs
git commit -m "fix(rakia/schemas): document empty-fingerprint sentinel for --files mode"
```

If nothing changed, no commit needed.

---

### Task 9: End-to-end smoke + sweep

- [ ] **Step 9.1: Run all rakia tests**

```bash
cd /home/matthew/git/elohim/elohim/rakia
RUSTFLAGS="" cargo test 2>&1 | grep "test result" | head -15
```

Expected: all green.

- [ ] **Step 9.2: Run all brit-graph tests (both feature configs)**

```bash
cd /home/matthew/git/elohim/elohim/brit
RUSTFLAGS="" cargo test -p brit-graph 2>&1 | grep "test result"
RUSTFLAGS="" cargo test -p brit-graph --features repo 2>&1 | grep "test result"
```

Expected: both green; the `--features repo` config has 7 more tests (from `tests/repo_fingerprint.rs`).

- [ ] **Step 9.3: Run brit-cli tests**

```bash
cd /home/matthew/git/elohim/elohim/brit
RUSTFLAGS="" cargo test -p brit-cli 2>&1 | grep "test result"
```

Expected: green, including the new `fingerprint_emits_content_addressed_hex_for_real_manifest` test.

- [ ] **Step 9.4: End-to-end smoke run on live repo**

```bash
B=/home/matthew/git/elohim/elohim/brit/target/debug/brit
echo "=== fingerprint (live) ==="
$B fingerprint /home/matthew/git/elohim/app/elohim-app/build-manifest.json --step build-angular
echo "=== fingerprint (HEAD~5) ==="
$B fingerprint /home/matthew/git/elohim/app/elohim-app/build-manifest.json --step build-angular --commit HEAD~5
```

Expected: both produce 64-char blake3 hex; comparing them tells you whether build-angular's source files changed in those 5 commits.

---

### Task 10: Sprint-result artifact

**Files:**
- Create: `/home/matthew/git/elohim/docs/superpowers/sprint-results/2026-04-19-brit-content-fingerprint.md`

- [ ] **Step 10.1: Author the artifact**

`/home/matthew/git/elohim/docs/superpowers/sprint-results/2026-04-19-brit-content-fingerprint.md`:

```markdown
# Sprint Result: Brit Content Fingerprint

**Date:** 2026-04-19
**Plan:** `docs/superpowers/plans/2026-04-19-brit-content-fingerprint.md`
**Branches:** `feat/brit-content-fingerprint` (brit), `feat/build-plan-fingerprints-param` (rakia)

## What changed

`brit-graph` gained `ContentFingerprint::from_repo_globs(repo, commit_id, patterns)`
behind a new `repo` feature flag. This walks the git tree at a specific commit,
matches paths against globs, reads blob bytes, and feeds them into the existing
pure `ContentFingerprint::compute`.

`brit fingerprint <manifest> [--commit <ref>]` now uses it. Output fingerprints
are 64-char blake3 hex strings derived from actual file contents — not glob
pattern strings.

`brit plan` does the same: for each step in the plan, compute the fingerprint
from the head commit's tree, pass to `to_build_plan` via the new
`fingerprints: BTreeMap<String, String>` parameter.

`rakia-core::build_plan::to_build_plan` no longer computes fingerprints
internally (placeholder DefaultHasher deleted). It accepts the map from callers
who can open the repo. Tests use `&BTreeMap::new()`; the schema accepts empty
fingerprint strings as the "no repo context" sentinel.

## Why

The previous fingerprint hashed glob pattern strings, not file contents. Two
repos with different file contents under the same glob produced identical
fingerprints — useless for the future "artifact X verified by N peers" attestation
flow. This sprint fixes the primitive before downstream work depends on it.

## Verified properties

- Determinism: same repo + same commit + same patterns → same fingerprint (asserted in tests)
- Content sensitivity: same patterns, different file contents → different fingerprint (asserted)
- Reproducibility: reads from git tree, not working tree (skips uncommitted changes)
- Empty case: no patterns or no matches → stable empty-input fingerprint
- Error path: invalid glob → typed error (matched in test)

## Carry-overs

- Submodule and symlink tree entries are skipped — deferred until a manifest cares
- Working-tree mode (`--working-tree` flag) — explicitly out of scope for reproducibility
- Caching — defer until profiling shows it matters
- Including executor declaration in the fingerprint — Stage 2 work, requires BuildExecutor identity contract
- The `fingerprint: ""` sentinel for --files mode is a documented convention; future schema may discriminate "computed" vs "absent" via Optional or a discriminator field

## Sprint statistics

- Tasks: 10
- New code: ~200 lines (repo_fingerprint.rs) + ~150 lines test
- Schema changes: description-only on PlannedStep.fingerprint (if any)
- Test count delta (brit-graph): +7
- Test count delta (brit-cli): +1
- Test count delta (rakia-core): +2 (new tests in build_plan.rs)
- Eliminated: placeholder DefaultHasher in rakia-core

## Next

The fingerprint primitive is ready for the Rakia Runnable sprint to use as the
plan-identity hash for content-addressed dispatch and per-peer attestation
anchoring.
```

- [ ] **Step 10.2: Commit (in parent repo on dev)**

```bash
cd /home/matthew/git/elohim
git status
git add docs/superpowers/plans/2026-04-19-brit-content-fingerprint.md docs/superpowers/sprint-results/2026-04-19-brit-content-fingerprint.md
git commit -m "docs: brit content fingerprint plan + sprint result"
```

(The plan file itself was committed earlier when the plan was authored. If it isn't yet, include it here.)

---

### Task 11: Submodule pointer bumps

- [ ] **Step 11.1: Push feat branches in both submodules**

```bash
git -C /home/matthew/git/elohim/elohim/brit push -u origin feat/brit-content-fingerprint
git -C /home/matthew/git/elohim/elohim/rakia push -u origin feat/build-plan-fingerprints-param
```

- [ ] **Step 11.2: Merge feat → main in each submodule**

```bash
cd /home/matthew/git/elohim/elohim/brit
git checkout main
git merge --no-ff feat/brit-content-fingerprint -m "Merge feat/brit-content-fingerprint: content-addressed fingerprints"
git push origin main

cd /home/matthew/git/elohim/elohim/rakia
git checkout main
git merge --no-ff feat/build-plan-fingerprints-param -m "Merge feat/build-plan-fingerprints-param: to_build_plan accepts fingerprints map"
git push origin main
```

- [ ] **Step 11.3: Bump parent submodule pointers**

```bash
cd /home/matthew/git/elohim
git add elohim/brit elohim/rakia
git commit -m "chore: bump brit + rakia submodules — content fingerprint sprint"
git push origin dev
```

---

## Self-Review

**1. Spec coverage:** No formal spec — design captured inline in Design Summary section. Each design point is implemented:
- `ContentFingerprint::from_repo_globs` constructor → Task 1, 2
- Feature gate (`repo`) → Task 1
- brit-cli fingerprint upgrade → Task 4
- brit-cli plan integration → Task 7
- rakia-core to_build_plan refactor → Task 6
- Schema/sentinel for empty fingerprint → Task 8
- IoC close + tests → Task 9
- Sprint-result + bump → Tasks 10, 11

**2. Placeholder scan:** No "TBD/TODO/implement later" remaining. Each step has executable commands or concrete code. The "may differ slightly across versions" caveat in Task 1.3 is honest about gix API drift, with a clear iteration directive.

**3. Type consistency:**
- `ContentFingerprint::from_repo_globs(repo, commit_id, patterns)` — same signature in scaffold (Task 1) and tests (Task 2).
- `FingerprintError` variants — defined in Task 1.2, matched against in Task 2.1's `invalid_glob_returns_error` test.
- `to_build_plan` new signature — defined in Task 6.1, matches caller updates in Task 6.2 (contract test), 6.3 (fixture runner), 7.2 (brit-cli plan).
- `BTreeMap<String, String>` for fingerprints — consistent across all call sites.
- `compute_fingerprints` helper in plan.rs — defined in Task 7.2, no external callers (private to plan.rs).

Plan is complete and self-consistent.
