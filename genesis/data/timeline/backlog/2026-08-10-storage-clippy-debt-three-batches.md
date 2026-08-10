---
id: "backlog-storage-clippy-debt-three-batches"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-storage clippy --all-targets debt (37 errors, all pre-639ef94e6 sites) — drain in three batches; never scope test-exclusive guards before .await"
slug: "storage-clippy-debt-three-batches"
written: "2026-08-10"
author: "batch-3 review session (operator-revised delegation)"
status: "backlog"
priority: "medium"
tags: [clippy, lint-debt, elohim-storage, delegation, codex-claimable]
cites:
  - elohim/elohim-storage/src/services/head_adoption.rs
---

# elohim-storage clippy debt — three batches, one hard constraint

`cargo clippy --all-targets -- -D warnings` on elohim-storage carries 37
pre-existing errors (operator-verified count; all lint sites predate
639ef94e6). Drain plan (operator-revised, 2026-08-10):

1. **Batch A — 21 mechanical/low-risk findings**: dispatchable now
   (doc-lazy-continuation, useless `vec!`, literal-bool asserts,
   constant-value assertions, complex-type, items-after-test-module, etc.).
2. **Batch B — 9 test-lock findings OUTSIDE head_adoption.rs**
   (`MutexGuard held across await` in tests): separate dispatch. **Hard
   constraint: do NOT "scope the guard" before `.await`** — those guards
   (`advertiser_health::test_exclusive()` and kin) intentionally serialize
   process-global test state; dropping them early reintroduces the parallel
   flake they exist to stop. Acceptable fixes: a documented
   `#[allow(clippy::await_holding_lock)]` with a why-comment, or an atomic
   migration of ALL callers to an async-aware mutex — never a partial one.
3. **Batch C — 7 head_adoption.rs findings**: queued until the a9f9d781b
   review-hardening commit is integrated (avoid conflicting edits in the
   ghost-decay region).

Acceptance for every batch: full `cargo test` green, then

```
env RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/tmp/elohim-storage-clippy \
  cargo clippy --all-targets -- -D warnings
```
