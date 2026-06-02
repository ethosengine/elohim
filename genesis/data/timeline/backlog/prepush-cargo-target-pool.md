---
id: "backlog-prepush-cargo-target-pool"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Pre-push hook bypasses the cargo-target-pool — no per-crate CARGO_TARGET_DIR → ENOSPC"
slug: "prepush-cargo-target-pool"
written: "2026-06-02"
author: "cartographer"
status: "proposed"
priority: "medium"
area: "cargo"
recurrence: 1
source_shifts:
  - "2026-05-18"
domain: "code"
relatedNodeIds:
  - "memory:feedback_multi_agent_pvc_pacing"
  - "memory:project_devspace_disk_cleanup_procedure"
  - "memory:project_ci_storage_topology"
tags: [cargo, prepush, target-pool, enospc, code-domain]
shift_objective: |
  The pre-push hook compiles into the default target dir instead of sourcing a per-crate key
  from the cargo-target-pool, so a multi-crate push piles all build artifacts into one
  location and runs the volume out of space (ENOSPC) — observed 2026-05-18. The pool exists
  precisely to scope each crate's target dir, but the hook doesn't use it.
  Resolve it by having the pre-push hook source a per-crate CARGO_TARGET_DIR from the
  cargo-target-pool (the same keying CI uses), so each crate's artifacts land in their own
  pooled slot and a push doesn't ENOSPC. This is code-domain (.husky pre-push + the
  cargo-pool helper). Mind the shared-target hazard (concurrent agents sharing a target dir is
  unsafe — feedback_multi_agent_pvc_pacing) and the disk-pressure thresholds
  (project_devspace_disk_cleanup_procedure). Done when a multi-crate pre-push uses per-crate
  pooled target dirs and no longer ENOSPCs.
---

# Pre-push hook sources the cargo-target-pool per crate

## Why this matters

Code-domain. An ENOSPC at pre-push time blocks the push entirely and often requires a manual
target-dir clean to recover — friction on every multi-crate change. The pool already solves
this for CI; the hook just doesn't use it.

## The failure shape

- Pre-push compiles into the default target dir (no per-crate CARGO_TARGET_DIR).
- A multi-crate push accumulates all artifacts in one place.
- The volume runs out of space → ENOSPC → push blocked.

## Shape of the fix (code-domain)

The `.husky` pre-push hook sources a per-crate CARGO_TARGET_DIR from the cargo-target-pool
(matching CI's keying), so artifacts land in per-crate pooled slots. Respect the shared-target
hazard — concurrent agents must not share a slot (`feedback_multi_agent_pvc_pacing`) — and the
85% PVC act-threshold / cleanup procedure (`project_devspace_disk_cleanup_procedure`).

## Acceptance

A multi-crate pre-push uses per-crate pooled target dirs and no longer ENOSPCs.
