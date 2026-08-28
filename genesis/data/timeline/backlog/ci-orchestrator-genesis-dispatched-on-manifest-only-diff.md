---
id: "backlog-ci-orchestrator-genesis-dispatched-on-manifest-only-diff"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Orchestrator #1739 dispatched elohim-genesis on a diff (c4a148db6..4f4785e03: genesis/build-manifest.json + one backlog file + ci data) that matches none of genesis's source globs — CI-side change detection diverges from graph-walker.mjs"
slug: "ci-orchestrator-genesis-dispatched-on-manifest-only-diff"
written: "2026-08-28"
author: "shift 2026-08-28T03-25-shakeout-landing-perf-trust-hybrid (post-close)"
status: "open"
priority: "medium"
jobs: [elohim-orchestrator]
tags: [ci, orchestrator, dispatch-drift, principle-7]
---

## Observed

Push `4f4785e03` (narrowed `genesis/data/**` → consumed subdirs; a `runtime-*` backlog entry; ci ledgers). Local pre-flight `graph-walker.mjs` on the staged diff: **[]**. Orchestrator #1739 plan: **`auto: elohim-genesis`**. Archived `pipeline-baselines.json`: #1738 `__global__=c4a148db6`, `elohim-genesis=c4a148db6`; #1739 `__global__=4f4785e03`, genesis still `c4a148db6` — so the CI changeset was `c4a148db6..4f4785e03`, the same three-file diff the local walker rejects.

CI change detection runs in `genesis/orchestrator/build-graph.groovy` (`checkSourceChanges` via `matchesGlob`, `checkBuildProcessChanges` via sha256 of `buildProcess` refs), not in `graph-walker.mjs`; the two are meant to agree. The orchestrator console for #1739 could not be read (Jenkins log endpoints timed out repeatedly while the controller was loaded), so the matched reason is **not captured**. Candidates: (a) `matchesGlob`'s `**`→regex conversion treating `genesis/data/lamad/**` differently from picomatch; (b) an implicit "this pipeline's own build-manifest.json changed → stale" rule; (c) the `buildProcess` hash path (genesis/Jenkinsfile unchanged in that diff, so unlikely).

## Why it matters

Each unwanted genesis dispatch is a full seed storm + doorway churn on the fleet (see `runtime-shem-edgenode-container-exit-139-chronic`) and a redded measure. Principle 7: the dispatched set must be what a developer reading the diff predicts.

## Next (bounded)

Read #1739's "Determine Build Plan" block once the controller is quiet (`mcp__jenkins__getBuildLog` skip≈420–660) and record the `reason:` line; then either fix the Groovy matcher to agree with picomatch or make the manifest-changed rule explicit in both walkers and the README.
