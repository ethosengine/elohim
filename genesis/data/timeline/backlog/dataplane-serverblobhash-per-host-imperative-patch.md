---
id: "backlog-dataplane-serverblobhash-per-host-imperative-patch"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "serverBlobHash is a per-host deploy-time PATCH, not a derived attribute of the declared head — violates dataplane-convergence"
slug: "dataplane-serverblobhash-per-host-imperative-patch"
written: "2026-09-06"
author: "story-harvest"
status: "backlog"
priority: "high"
relatedNodeIds: []
tags: [dataplane-convergence, doorway, ssr, content-service, per-host-write]
cites:
  - doorway/doorway-service/src/render/registry.rs
  - elohim/elohim-storage/src/services/content_service.rs
---

## Chain

`dataplane-convergence` / between "elected content head is declared" → "every doorway serves
that head's server-rendered bundle" / **missing node: `serverBlobHash` derives from the
declared head as a projected attribute, instead of being written imperatively, per-host, at
deploy time.**

## Assertion + probe

Assertion: two doorways serving the same declared content head report the same
`serverBlobHash` and the same `dhtAnchorState` for the identical `blobHash`, because the
field is computed/read from shared converged state, not from whichever deploy happened to
PATCH that one host. Probe: `GET` the content row for the same `blobHash` from doorway-A and
doorway-B's storage peers and diff `serverBlobHash` / `dhtAnchorState` — they must match.

## Current state — found live 2026-09-06

Each doorway's SSR render registry PATCHes its own storage peer's content row directly
(`doorway/doorway-service/src/render/registry.rs` — the module doc: "storage content row's
`serverBlobHash`... moves whenever CI re-deploys" via a background reconcile tick that
issues an HTTP PATCH; `elohim/elohim-storage/src/services/content_service.rs:245` names this
"Substrate-correct PATCH path" but the PATCH still lands only on the ONE storage peer behind
the doorway that issued it — the write never crosses to peer storage). Observed during the
crossing shift: adam (doorway B's storage peer) **never received** the PATCH for a bundle
matthew (doorway A's storage peer) had already served — adam's row showed `serverBlobHash:
null`, `dhtAnchorState: unverified`, while matthew's row for the identical `blobHash` showed
`dhtAnchorState: live`. Same content, same blob, two different served realities depending
solely on which doorway happened to run the reconcile tick.

This is a direct instance of the `dataplane-convergence` habit's own invariant text: "no
per-host imperative write is load-bearing." `serverBlobHash` is load-bearing — it gates
whether SSR serves the pre-rendered bundle or falls through — and today it is exactly a
per-host imperative write with no cross-peer propagation.

## Missing node (concrete shape)

`serverBlobHash` becomes a **derived attribute of the elected head record** — computed from
(or synced alongside) the same declared-head mechanism that `dataplane-convergence`'s other
checks already exercise (reconcile/inventory/projector), so it converges the way any other
row attribute does, rather than requiring N independent per-doorway PATCH reconcile ticks to
each separately discover and write it. A doorway that never runs its own reconcile tick (or
runs it late) should still observe the correct `serverBlobHash` because it reads converged
state, not because it happened to PATCH itself.

## Habit served

`dataplane-convergence` (`elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md`,
status: red, active: true) — invariant: "Serving-critical content state converges
peer-to-peer; no per-host imperative write is load-bearing. A peer that missed a deploy
heals." This finding is a fresh, concrete instance of exactly that red.

## TODO (integrator)

`cites:` is a plain path list per this directory's existing convention (no fingerprint
invented). If cite-gen is later mandated for backlog rows, run it here rather than
hand-writing a fingerprint.
