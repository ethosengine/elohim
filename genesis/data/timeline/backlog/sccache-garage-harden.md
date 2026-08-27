---
id: "backlog-sccache-garage-harden"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Harden sccache+garage reliability OR add a retry-without-sccache pre-push fallback (NoSuchKey poisons rustc output)"
slug: "sccache-garage-harden"
written: "2026-06-02"
author: "cartographer"
status: "proposed"
priority: "high"
area: "cargo"
recurrence: 3
source_shifts:
  - "2026-05-17"
  - "2026-05-18"
domain: "code"
relatedNodeIds:
  - "memory:project_quilt_pantry_vocabulary"
  - "memory:feedback_env_var_test_flakiness"
tags: [cargo, sccache, garage, cache, prepush, code-domain, recurring]
shift_objective: |
  When the sccache S3 backend returns NoSuchKey / null bytes, the error stream interleaves
  into rustc's diagnostic output, producing spurious "unclosed delimiter" / unparseable
  compiler errors that look like a code bug but are cache corruption. `RUSTC_WRAPPER=""`
  bypasses sccache, but the poisoned key persists for the next build (observed 2026-05-17,
  05-18). There is no documented runbook for the heal, so each occurrence is re-diagnosed
  from scratch.
  Resolve it on two fronts: (1) harden the sccache+garage path so a NoSuchKey can't interleave
  into rustc output (or fail the cache read cleanly), and (2) add a retry-without-sccache
  fallback in the pre-push hook so a poisoned cache degrades to a slow-but-correct build
  instead of a spurious compile error. Document the heal runbook (SCCACHE_RECACHE=1 to force
  re-cache, or repave the poisoned key). This is code-domain (pre-push hook + cache config +
  runbook). Done when a poisoned cache key produces a clean cache-miss/fallback rather than a
  spurious rustc error, and a runbook documents the heal.
---

# Harden sccache/garage + add a retry-without-sccache fallback

## Why this matters

Code-domain. This failure is doubly costly: it wastes a build AND it presents as a code bug
("unclosed delimiter") that sends the author hunting in the wrong place. A clean
cache-miss/fallback plus a documented heal removes both costs.

## The failure shape

- sccache S3 read returns NoSuchKey / null bytes.
- The error interleaves into rustc's diagnostic stream → spurious "unclosed delimiter".
- `RUSTC_WRAPPER=""` bypasses it, but the poisoned key persists for the next build.
- No runbook → each occurrence re-diagnosed from scratch.

## Shape of the fix (code-domain)

1. Harden the sccache+garage read path so a NoSuchKey fails cleanly (clean cache-miss) and
   never interleaves into rustc output. (sccache targets the quilt — `project_quilt_pantry_vocabulary`.)
2. Add a retry-without-sccache fallback in the pre-push hook (poisoned cache → slow-but-correct
   build, not a spurious error).
3. Document the heal runbook: `SCCACHE_RECACHE=1` to force re-cache, or repave the key. (Mind
   env-var test flakiness when adding coverage — `feedback_env_var_test_flakiness`.)

## Acceptance

A poisoned cache key produces a clean cache-miss/fallback rather than a spurious rustc error;
a runbook documents the heal.

## Recurrence 3 — 2026-08-27, a NEW shape: dead Garage key (elohim-holochain #1403)

Not corruption this time — the credential. `sccache: error: Server startup failed: cache storage failed to read: PermissionDenied (permanent) … S3Error { code: "AccessDenied", message: "Forbidden: No such key: GK4e34b68f3f9bcca5ce769366", resource: "/sccache-elohim/.sccache_check" }` — the Garage access key mounted from the `jenkins`-ns `sccache-credentials` Secret no longer exists on the Garage server. The server refuses to start, `sccache rustc -vV` exits 2, `cargo metadata` exits 101, `DNA BUILD FAILED` 85 s after dispatch — before any compile. #1402 (2026-08-19) was green, so the key died in that window.

**What it did and did not block:** the DNA job is `longRunning`, so the orchestrator dispatches it fire-and-forget and records an optimistic success — its FAILURE is invisible to the `levelFailed` guard. Orchestrator #1733 therefore still dispatched `elohim-edge` #1386 (after the wait:true app build) and will trail genesis. Consequence worth knowing: a dead sccache key makes the DNA lane silently red — nothing upstream notices, the DNA baseline stays stale, and the red is visible only in the `elohim-holochain` job view. (First read of the guard assumed it would withhold edge; it does not — checked against #1386.)

**Two fixes, two owners:**
1. *Operator:* re-provision the Garage key and refresh `sccache-credentials` in the `jenkins` namespace (runbook: `genesis/manifests/RUNBOOK-minio-sccache-2026-05-09.md` predates the Garage move — update it while there).
2. *Repo (this backlog, front 2 above, made concrete):* `elohim/holochain/dna/elohim/flake.nix` `shellHook` sets `RUSTC_WRAPPER=sccache` on binary presence alone. Probe the server (`sccache --start-server` / `--show-stats` with a short timeout) and leave `RUSTC_WRAPPER` unset with a loud stderr line when it fails — a dead cache becomes a cold compile, never a level-0 red. Ship it in a batch that already churns the fleet (it dispatches the DNA job and, by cascade, edge).
