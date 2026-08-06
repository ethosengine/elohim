---
id: "backlog-fork-stage0-harness-commit"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Commit the Stage-0 iroh harness + lock repair to the conductor fork (elohim-0.6.3)"
slug: "fork-stage0-harness-commit"
written: "2026-08-06"
author: "agentic-developer"
status: "open"
priority: "medium"
area: "holochain-fork"
domain: "code"
tags: [iroh, wave2, stage0, holochain-fork, cargo-lock, code-domain]
---

# Commit the Stage-0 harness to the fork so it survives as the Wave-3 regression rail

The Wave-2 Stage-0 mechanism proof (design doc §5.2, PASSED 2026-08-06) lives ONLY in the
`/projects/elohim/elohim/holochain-conductor` worktree as uncommitted state:

- `crates/holochain/tests/iroh_stage0.rs` (untracked) — two SweetConductors, local bootstrap,
  live sovereign relay via `ELOHIM_RELAY_URL`, trailing-dot-aware peer-URL assertions.
- `Cargo.lock` (modified) — the committed lock was ALREADY stale vs its own manifests
  (`--locked` broken before this work; resolution adds exactly 8 packages: tikv-jemalloc*-sys
  + the six tx5-* crates moving to the local `[patch.crates-io]` path). Pristine backup:
  `/projects/elohim/.stage0-logs/holochain-conductor-Cargo.lock.orig`.

Task: commit BOTH to the fork branch `elohim-0.6.3` (ethosengine/holochain) — test file +
lock repair, separate commits. This restores `--locked` for every future builder AND makes the
Stage-0 test the standing regression harness for Stage-2/Wave-3 (re-run:
`cargo test -p holochain --test iroh_stage0 --features test_utils`).

Notes for the claimant:
- Moving the branch HEAD changes what the edgenode image job fetches — the test file is
  build-inert (tests/ dir), and the lock repair only pins what already resolves. Rebuild risk ≈ 0,
  but note it in the commit message.
- Verify before push: `cargo test -p holochain --test iroh_stage0 --features test_utils` green
  AND `cargo build --locked -p holochain` now succeeds (the repair's proof).
- Evidence logs stay in the monorepo (`.stage0-logs/`), referenced by the wave-2 design doc.
