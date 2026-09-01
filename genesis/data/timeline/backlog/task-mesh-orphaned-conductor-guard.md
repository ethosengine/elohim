---
id: "backlog-task-mesh-orphaned-conductor-guard"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Task: mesh lifecycle refuses orphaned-live conductors — a surviving conductor whose data root is unlinked/deleted must be detected and rejected, not served from"
slug: "task-mesh-orphaned-conductor-guard"
written: "2026-09-01"
author: "session-2026-09-01-rung5-design (from the matthew coordswap diagnosis)"
status: "open"
priority: "medium"
jobs: [elohim-genesis]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-task-fleet-version-matrix-probe"
  - "backlog-upgrade-propagation-p2p-design-arc"
tags: [mesh, harness, conductor, lifecycle, coordswap, delegable]
---

**Claimable by any implementation agent. Born from the 2026-09-01 matthew
coordswap diagnosis (recorded in `task-fleet-version-matrix-probe.md:83`):
the rung-1 guard behaved correctly over legitimate coordinator-only drift,
but apply failed because matthew's mesh conductor survived a failed sandbox
regeneration (PermissionDenied) and kept serving from an unlinked/deleted
data root. The corrected coordswap invocation cannot be proven until the
mesh lifecycle rejects that state.**

## Why

An orphaned-live conductor is the data-root sibling of the exe-slot
`(deleted)` binary class the harness already detects (`resolve_exe` strips
the suffix to catch replaced binaries). A conductor serving from a deleted
root passes health probes while its persistence is a ghost — every write is
lost on exit, and admin operations (update_coordinators) fail in confusing
ways downstream of the real defect.

## Scope

1. `app/elohim-app/scripts/hc-mesh.sh` (the harness owns lifecycle): at
   `status`, `probe`, and before any conductor-touching verb
   (`conductors-restart`, the coordswap path), check each running conductor's
   data-root liveness — the sandbox dir exists AND the conductor's open
   handles resolve to non-deleted paths (`/proc/<pid>/` inspection, same
   toolkit as `resolve_exe`). An orphan is reported LOUDLY with a named
   state (`orphaned-data-root`) and the affected verbs refuse with that
   reason instead of proceeding.
2. `start`/`join-peer` refuse to stage over an orphan (kill + regenerate is
   the operator-visible remediation the message names — the guard itself
   does not auto-kill).
3. Diagnose-only for the PermissionDenied sandbox-regeneration trigger:
   record what denied (path + mode) when detected; the fix for THAT may be
   container/user-mapping and is out of scope here.

## DoD

- A fixture orphan (delete a test conductor's sandbox dir while it runs) is
  detected by `status` and named; coordswap against it refuses with
  `orphaned-data-root`; a healthy mesh is unaffected (two consecutive clean
  `just mesh start` → `status` runs).
- MUST NOT edit Rust source, zomes, or CI scripts.
