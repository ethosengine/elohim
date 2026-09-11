---
id: "backlog-shared-worktree-sweep-and-nextest-fail-fast"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Pathspec commits do not protect against another lane's hunks INSIDE a committed file, and the pre-push gate compiles the working tree rather than the commit — edge #1451 shipped three `mod` lines whose files were never added; nextest fail-fast then hid a second live-checkout failure for one more wave"
slug: "shared-worktree-sweep-and-nextest-fail-fast-2026-09-11"
written: "2026-09-11"
author: "shift 2026-09-11T09-00-land-batch-2c338124a (integrator)"
status: "open"
priority: "medium"
tags: [gate-runner, pre-push, shared-worktree, pathspec, nextest, eprfs, elohim-storage, dev-process, D8]
relatedNodeIds: []
cites:
  - genesis/orchestrator/gate-runner.mjs
  - .husky/pre-push.bash
  - elohim/eprfs/Jenkinsfile
  - elohim/elohim-storage/src/services/mod.rs
---

# Two traps, one wave each

## 1. The sweep inside the file (edge #1451, fixed by 3c690ec54)

fb4d10c7d committed `services/mod.rs` and `api/rea_commitments.rs` by pathspec from the shared worktree. Both files already carried another lane's uncommitted hunks (three `pub(crate) mod` lines and a `?refresh` query path calling into one of them). The pathspec guarded WHICH files went in, not WHAT was in them. Local gates passed because the module files exist on disk; CI's Build Storage stage failed `E0583 ×3`. Cure this shift: edits authored in an isolated `git worktree add --detach` at HEAD, `cargo check --features p2p-iroh` there (0 errors), fast-forward, then the two files' pre-fix content restored in the main working copy so the other lane kept compiling.

**Bounded fix:** when the tree is dirty, the pre-push hook (or `gate-runner.mjs`) compiles the PUSHED sha in a temporary detached worktree against the same cargo-pool slot (`cargo check` per touched native crate), not the working tree — one honest attestation for shared lanes. Cost: minutes, incremental. Until then: `git diff --cached` review of every hunk before a pathspec commit in a shared tree.

## 2. Fail-fast hides the next failure (eprfs #29 → #30)

`cargo nextest run --workspace --all-targets` in `elohim/eprfs/Jenkinsfile` stops at the first failure, so #29 reported only the corpus-cite test and #30 surfaced the `git check-ignore` dubious-ownership test after a full wave. Cure this shift: `safe.directory "*"` before nextest (86f2b32e8) and `--no-fail-fast` so one run reports every red.

## Done when

- A commit that declares a module with no file is refused by the local pre-push gate on a dirty tree.
- An eprfs CI run with two independent failing tests reports both in one build.
