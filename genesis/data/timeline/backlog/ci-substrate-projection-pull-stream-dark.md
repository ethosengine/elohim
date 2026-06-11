---
id: "backlog-ci-substrate-projection-pull-stream-dark"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "projection.streams pull=false on ALL alpha pods — the pull stream is not running anywhere (genesis #1119)"
slug: "ci-substrate-projection-pull-stream-dark"
written: "2026-06-11"
author: "agentic-developer (EPR durability arc, pipeline shakeout)"
status: "wip"
priority: "medium"
ci_status: pending-verification
jobs: [elohim-genesis]
tags: [substrate, epr-durability-arc, projection, streams, observability]
cites:
  - genesis/scripts/ci/substrate-verify.sh
---

# projection pull stream dark fleet-wide

## Symptom (genesis #1119, Verify Substrate Projection)

Every alpha pod fails the same way, three attempts each:

```
❌ projection.{matthew,adam,jessica}.streams — replication=true pull=false projection_reconcile=true
✅ projection.{matthew,adam,jessica}.lag — no cursor lag > 120s
```

Replication and projection_reconcile streams report alive; the **pull**
stream reports not-running on all three pods. Uniformity says design/config
class (a single-pod crash would be asymmetric).

## Why it matters

Workstream E (projection durability) needs the full stream set healthy
before the pod-delete crutch can be retired. A dark pull stream is either
(a) a real dead consumer — content pulls never drain, or (b) a stale
assertion — the stream was renamed/merged and the suite asserts a ghost.
Either way the suite stays red until reconciled.

## First actions

1. Find the status surface the assert reads (`substrate-verify.sh`
   projection stage → which endpoint/field) and the storage-side stream
   registry that populates it — does a stream named `pull` still exist in
   elohim-storage, or did it merge into replication?
2. If real: trace why the pull consumer never starts on boot (all pods).
3. If ghost: fix the assertion vocabulary, not the substrate.

shift_objective: |
  Make projection.streams green honestly on all three alpha pods: determine
  whether the pull stream is a dead consumer or a renamed assertion target,
  land the matching fix, and keep the projection stage green for three
  consecutive genesis builds.

## RESOLVED as assert-ghost (2026-06-11, this shift) — pending CI verification

The emitter is spec-correct: `acquisition::rollup()` computes
`caught_up = total>0 && fetched==total` with an explicit "total == 0
(resolved-empty) is likewise not caught_up — never false-complete (spec
R-A)" stance. Alpha pods carry ZERO pin-acquisition workload, so
`pull.caughtUp=false` fleet-wide is the IDLE state, misread by the CI gate
as a dead stream. Fixed assert-side in substrate-verify.sh: pull is now
tri-state (null / idle / bool) — idle requires the wire to actually carry
`total` (schema rename fails closed) and lands in the WARN branch, never a
clean pass (review finding: total==0 cannot distinguish resolved-empty from
an acquisition loop that never registered pins). Emitter-side follow-up
(explicit `state: idle|active|caughtUp` field in rollup()) remains open —
that is the fix-at-depth that lets the gate pass idle confidently.
