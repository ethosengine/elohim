---
id: "backlog-ci-substrate-projection-pull-stream-dark"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "projection.streams pull=false on ALL alpha pods — the pull stream is not running anywhere (genesis #1119)"
slug: "ci-substrate-projection-pull-stream-dark"
written: "2026-06-11"
author: "agentic-developer (EPR durability arc, pipeline shakeout)"
status: "backlog"
priority: "medium"
ci_status: red
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
