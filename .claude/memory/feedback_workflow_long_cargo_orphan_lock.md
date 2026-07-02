---
id: feedback-workflow-long-cargo-orphan-lock
name: workflow-long-cargo-orphan-lock
title: Workflow long-cargo orphan lock
description: "Bash timeout orphans cargo still holding .cargo-lock; let it finish (work lands on disk), keep one profile per gate phase, run_in_background for >10min cargo."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 22ae7e20-f5ce-4831-9568-a0a648ab063a
cites:
  - .husky/pre-push
---

During the url-routing-slice1 workflow (2026-06-04), a gate agent's `cargo test export_bindings` in the elohim-storage workspace (~35 min: debug-profile test binaries from cold, since the prior nextest pass was release-profile) outlived the Bash tool timeout. The cargo process kept running detached, holding the `.cargo-lock` on the pool slot; the agent's retries blocked on the lock and a pgrep quoting quirk made it mis-read the orphan as dead.

**Why:** Bash tool timeout kills the session, not the spawned cargo; cargo holds the artifact-dir lock until exit. The orphan still does the real work (bindings regenerated on disk) — only its stdout is lost.

**How to apply:** (1) In workflow gate prompts, instruct agents to run long cargo via `run_in_background` + log-file polling, or set an explicit generous `timeout` (≥600000ms). (2) When an orphan already holds the lock: do NOT kill it — let it finish; the queued retry becomes a fast warm no-op and verification proceeds from the on-disk artifacts. (3) Profile mismatch is the hidden cost driver: a release-profile nextest pass shares nothing with a debug-profile `cargo test` — keep gate phases on ONE profile. Related: [[cargo-target-dir-for-native-builds]], [[concurrent-sessions-shared-worktree]].
