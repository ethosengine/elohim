---
id: "backlog-ci-floating-happ-tag-blocks-dna-dispatch-skip"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The floating dev-latest hApp tag has no immutable binding to a DNA build — the already-built dispatch filter must keep the ~53-minute DNA build on every wave where edge survives"
slug: "ci-floating-happ-tag-blocks-dna-dispatch-skip"
written: "2026-09-22"
author: "claude (docs-only backlog entry, operator-directed)"
status: "backlog"
priority: "medium"
ci_status: red
jobs: [elohim-holochain-dna, elohim-edge]
relatedNodeIds: []
tags: [ci, orchestrator, dna, edge, dev-latest, harbor, dispatch-filter, anti-pattern]
cites:
  - elohim/holochain/dna/Jenkinsfile
  - elohim/holochain/Jenkinsfile
  - genesis/orchestrator/timer-dispatch.mjs
  - "orchestrator-already-built-filter-design | The already-built dispatch filter | sha256:c92546f49b159276 | path: genesis/docs/superpowers/specs/2026-09-22-orchestrator-already-built-filter-design.md"
  - "ci-orchestrator-recurring-anti-patterns-museum | History/ADR: CI / orchestrator recurring anti-patterns | sha256:d7703f837b3425f6 | path: genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md"
---

# Floating `dev-latest` hApp tag blocks a safe DNA dispatch skip

## The concern

The DNA pipeline (`elohim/holochain/dna/Jenkinsfile`, hApp/WASM Harbor publish
step, ~line 1057) pushes the packed hApp under the floating tag
(`dev-latest` on non-main branches, `latest` on main) **before** its later
publish steps run — the floating tag is overwritten mid-pipeline, not stamped
once at a verified-green end. The edge pipeline
(`elohim/holochain/Jenkinsfile`, `fetchHappFromHarbor`, ~line 109) fetches the
hApp it deploys by that same floating tag, via `getHappFloatingTag()`.

Two consequences follow from the same root cause — no immutable reference ties
a DNA build to the edge run that consumes it:

1. **Correctness**: a failed or aborted DNA run that starts after a green one
   can leave different bytes under the floating tag than what the last
   *successful* DNA build produced. A DNA run publishes partway (hApp tag
   moved), then fails or is aborted before completing WASM publication or
   before a would-be final re-stamp — the next edge deploy fetches whatever
   currently sits under the tag, which is not provably the artifact any
   specific green DNA build produced.

2. **Dispatch (measured 2026-09-22, orchestrator #1888)**: the per-pipeline
   already-built filter landed in this worktree as `aa922bf43`
   (design doc `genesis/docs/superpowers/specs/2026-09-22-orchestrator-already-built-filter-design.md`)
   was deliberately built with **no controller-evidence exception** for DNA.
   A prior green DNA build's `lastSuccessfulBuild` SHA cannot be trusted to
   authorize skipping a fresh ~53-minute DNA build on a wave where edge
   survives, precisely because a later DNA run (even one on a different
   branch or one that itself fails) can have since overwritten `dev-latest`
   with different bytes than what that historical green build produced.
   Museum row 2a documents this as the closed form: the filter keeps DNA
   alive on every wave edge survives, with no historical-SUCCESS shortcut,
   because `dev-latest` is floating and proves nothing about the bytes the
   consumer will fetch. Astra's review (`astra-review-orch-baseline-r2.result.md`,
   "NEW P1") independently derived the identical failing scenario: DNA build
   N succeeds at sha `B_P`, DNA build N+1 publishes different bytes to
   `dev-latest` and then fails or aborts, and a naive historical-SUCCESS
   exception would let the filter return N's SHA and drop DNA from the wave
   while still keeping edge — an unsafe skip.

The coordinator-only hot-swap path (`happ_manager::sync_coordinators`, gated
by `ALLOW_COORDINATOR_UPDATE`) is unaffected by this — it is a separate,
narrower healing mechanism for coordinator-zome-only changes and does not
touch how the bundle is named or fetched by tag.

## Cure direction

DNA publishes the packed hApp under an **immutable, content-or-sha-addressed
reference** — e.g. `happ-<git-sha>` or the CID/digest of the packed `.happ` —
and records that reference in its baseline/artifact metadata. Edge receives
the reference as an explicit dispatch parameter, or resolves it from the DNA
pipeline's baseline sha, instead of reading `dev-latest`. The floating tag
becomes a **convenience alias**, moved only *after* a fully successful
publish (all artifacts pushed, pipeline green) rather than mid-pipeline
before later steps can still fail.

Once that binding exists, the already-built filter can treat a DNA baseline
as verdict-backed evidence for a specific edge consumer (the historical-
SUCCESS exception Astra's review and museum row 2a both currently refuse to
grant) — because the artifact edge would fetch is then provably the one that
DNA build produced, not whatever happens to sit under a mutable tag at fetch
time.

## Current decision — open, no immutable reference exists yet

Not yet fixed. Until DNA publishes under an immutable reference and edge
consumes it, the already-built filter's conservative behavior (keep DNA
alive on every wave edge survives) is the correct, deliberately-accepted cost
— see museum row 2a — and this entry is the tracked cure for retiring that
cost.

## Evidence pointers

- `genesis/a2o/reports/recovery/serving-edge-20260921/astra-review-orch-baseline-r2.result.md` — NEW P1
- `genesis/a2o/reports/recovery/serving-edge-20260921/astra-review-orch-baseline-r3.result.md`
- `genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md` — row 2a
- `genesis/docs/superpowers/specs/2026-09-22-orchestrator-already-built-filter-design.md`
- `elohim/holochain/dna/Jenkinsfile` ~line 1057 (floating tag overwrite before later publish steps)
- `elohim/holochain/Jenkinsfile` ~line 109 (`fetchHappFromHarbor` / `getHappFloatingTag`)
- Orchestrator #1888; per-pipeline already-built filter landed as `aa922bf43` in this worktree
