---
id: "backlog-staggered-conductor-fleet-restarts"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Stagger conductor STS restarts on edge deploys — a simultaneous fleet roll costs hours of DHT-route outage"
slug: "staggered-conductor-fleet-restarts"
written: "2026-08-06"
author: "agentic-developer"
status: "wip"
priority: "high"
area: "deploy"
domain: "design"
tags: [edge-deploy, conductor, arc-catchup, availability, design-domain, needs-brainstorm]
---

# Staggered conductor fleet restarts

Evidence 2026-08-06: the edge deploy at 16:54Z restarted all 7 alpha conductors simultaneously;
every DHT-dependent doorway route (federation listing, SSR on doorway-A) went dark for ~2.5-3h of
fleet-wide arc catch-up (PTxnGuard write-guard contention on every peer, incl. matthew at
0.6-1.9s holds), recovering ~19:35Z with NO cure deployed — pure catch-up. The trust-contract
runbook's ≈20min churn figure is stale for this fleet size/data volume.

Cost model: every conductor-rolling deploy = hours of degraded DHT routes. The ZomeCaller
failover (landed 2026-08-06) cannot help when ALL conductors are catching up simultaneously.

Design question (ABOVE the iteration ceiling — needs /brainstorm, not a shift): stagger the
STS restarts per-peer with a convergence gate between waves (e.g. restart adam+eve, wait for
their zome-call layer to serve, then next pair), so surviving peers keep routes alive. Needs:
(a) the per-peer restart mechanics in the edge deploy leg (currently one pass over all STSs);
(b) an arc-coverage overlap argument (full-arc fleet: any one converged peer can answer any
read — verify); (c) a cheap converged-enough probe (the federation zome call itself?).
Interim mitigation candidates: deploy-time flag to skip conductor restarts when the image
digest didn't change (exists per memory — verify it actually held tonight: STS-unchanged
skip was expected but all 7 restarted for a doorway+storage-only batch at 16:54Z — why?).

## Codex claim — restart-mechanism verification (2026-08-06)

Claimed read-only investigation scope: `elohim/holochain/Jenkinsfile`, the two
active human StatefulSet source manifests, and this evidence record. No deploy
mechanism changes are authorized by this claim.

**Mechanism confirmed.** `deployHumanManifest` resolves `deployVersion` from the
current edge Git HEAD and substitutes it into `DEPLOY_VERSION_PLACEHOLDER`. Both
active sources stamp that placeholder into the StatefulSet pod-template label
`app.kubernetes.io/version`, not only top-level metadata. Therefore every edge
commit changes every rendered pod template and `kubectl apply` reports each
StatefulSet `configured`, even when conductor, storage image, and hApp bytes are
unchanged.

That defeats the genesis-pair no-op guard: adam and matthew skip only when the
apply log literally reports their StatefulSet `unchanged`. The other five peers
restart unconditionally. `deployHumansInParallel` launches all seven branches
together, which fully explains the simultaneous 16:54Z roll. A failed hApp
digest lookup can independently force another unique pod-template annotation,
but it is not needed to explain this event.

The mechanism question is resolved; the staggered-restart design, arc-overlap
argument, and convergence gate remain open, so the backlog entry stays `wip`.
