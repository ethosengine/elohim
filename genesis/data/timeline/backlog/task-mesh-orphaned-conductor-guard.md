---
id: "backlog-task-mesh-orphaned-conductor-guard"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Task: mesh lifecycle refuses orphaned-live conductors — a surviving conductor whose data root is unlinked/deleted must be detected and rejected, not served from"
slug: "task-mesh-orphaned-conductor-guard"
written: "2026-09-01"
author: "session-2026-09-01-rung5-design (from the matthew coordswap diagnosis)"
status: "complete"
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

## Completion evidence — 2026-09-01

- `hc-mesh.sh` now identifies each live conductor from its admin listener,
  verifies that the named sandbox still exists, and inspects `/proc/<pid>/fd`
  plus `cwd` for handles ending in `(deleted)`. `status` returns nonzero and
  names `state=orphaned-data-root`; `start`, `probe`, `join-peer`,
  `conductors-restart`, and the guarded `coordswap` path refuse without killing
  the process. Diagnostics include the sandbox path, permissions, uid/gid, and
  the deleted-handle target.
- The live-process fixture deletes a running process's sandbox and proves the
  healthy-to-orphan transition, nonzero `status`, coordswap refusal, unchanged
  PID, and named stop/start remediation:
  `bash app/elohim-app/scripts/__tests__/hc-mesh-orphan-guard.test.sh` — GREEN.
- The original mesh reproduced the exact real failure before remediation:
  matthew, jessica, and james retained deleted Lair `store_file` handles; the
  public `status` and coordswap preflight named and refused all three. A guarded
  stop removed the five owned processes. Two consecutive clean
  `start` → `status` passes then reported all three data roots live with stable
  conductor PIDs (`matthew=1374384`, `jessica=1374385`, `james=1374383`).
- The adjacent T4 restart station is closed in the same harness lane:
  `<storage_dir>/release-adoption/slot/elohim-storage.next` now outranks the
  running executable and restore fallbacks. A healthy boot archives the binary
  and sidecar under an `.applied-<timestamp>` slot path and records that
  executable; a failed boot archives them under `.failed-<timestamp>` and
  restores the previous executable record, so neither outcome can loop
  `.next`. The real-process proof is
  `bash app/elohim-app/scripts/__tests__/hc-mesh-storage-slot.test.sh` — GREEN.
- `just gate app/elohim-app/scripts/hc-mesh.sh` — GREEN (220 files / 4612
  tests), and scoped agent-package projection verification — GREEN (1697
  checks). No Rust, zome, or CI source was edited by this task.
