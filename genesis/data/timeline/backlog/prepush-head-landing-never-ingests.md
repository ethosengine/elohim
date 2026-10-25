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

The first diagnosis here ("nothing in pre-push ingests one") was wrong: `_gate-eprfs` ingests the
build it tested. The 33ab3925c push log shows the real cause, `[gate] eprfs attested: receipt
bafyreibvog4h…`. The pre-rebase push (090d68a03) had recorded a gate receipt for epr's source
closure; the rebase left that closure unchanged, so pre-push accepted the receipt and skipped the
recipe, ingest included. The receipt carries the test verdict across commits, not the bytes, so
no candidate ever named the pushed SHA. (cbdb065b4 is the other case: the hook that ran was the
main checkout's, with no landing block.)

## Fix (2026-10-25)

When no candidate names the pushed SHA, the landing block runs
`gate-runner.mjs --target eprfs --recipe _eprfs-deliver`. That builds `epr` and `bin-launch` at
HEAD in the gate's own slot and env and ingests the build with `--gate-run pre-push`, which is
honest because the receipt proved this closure was tested. It then lands the build and installs
the launcher with the landed epr when the slot's `bin-launch` differs from the installed one.
`genesis/orchestrator/prepush-head-landing.test.mjs` drives the hook's actual block.

## Done when

A green push to `dev` that changes eprfs moves this device's head to the pushed commit's build
with no hand step, and `just status device` from a fresh session reads that head.
