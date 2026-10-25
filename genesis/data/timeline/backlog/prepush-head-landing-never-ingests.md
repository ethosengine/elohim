---
id: "backlog-prepush-head-landing-never-ingests"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Pre-push's household-head landing can never land: nothing ingests the candidate it looks for"
slug: "prepush-head-landing-never-ingests"
written: "2026-10-25"
author: "claude-opus-5-5 (device-runtime slices A and A2)"
status: "open"
priority: "high"
tags: [device-runtime, bin-store, pre-push, landing, household-follows-latest, epr]
cites:
  - .husky/pre-push.bash
  - elohim/eprfs/device-runtime/src/bin_store/landing.rs
  - "household-follows-latest-design | The household follows latest | sha256:e23c1d03016b4baf | path: genesis/docs/superpowers/specs/2026-10-23-household-follows-latest-design.md"
---

# Pre-push's head landing never finds a candidate to land

## Observed

Two green pushes to `dev` that changed eprfs ended with the same line and no head move:

- `cbdb065b4` (slice A follow-up, before the main checkout ran dev's hook): no landing block ran
  at all — the hook that executed was the main checkout's, 135 commits behind.
- `33ab3925c` (slice A2, 2026-10-25, hook current):
  `[pre-push] epr household head not landed: no ingested epr candidate names the SHA targeting dev`.

Both times the device head was moved by hand: `epr bin ingest` of the eprfs gate's build
(`/tmp/eprfs-gate-target-wt-<worktree>/debug/epr`) at the pushed commit, then `epr bin land`
(`--revert` once, for a pre-rebase head that `dev` does not descend from), then
`epr bin install-launcher` from the same build.

## Cause

The landing block in `.husky/pre-push.bash` asks `epr bin status --commit <sha>` for an ingested
candidate and lands it. Nothing in pre-push ingests one: the eprfs gate builds `epr` into its
cargo slot and stops. The block can therefore only land when someone ingested that exact SHA by
hand beforehand.

## Fix shape

After the eprfs gate is green, pre-push ingests the gate's own build (the slot path the gate
prints as `[eprfs] cargo target:`) with `--commit <sha> --gate-run pre-push`, then lands it — the
gate-receipt path from slice A.4 already names the build it tested. Install the launcher from the
same slot when `bin-launch` changed. A regression test drives the hook's landing block with a fake
gate slot and asserts the head moves.

## Done when

A green push to `dev` that changes eprfs moves this device's head to the pushed commit's build
with no hand step, and `just status device` from a fresh session reads that head.
