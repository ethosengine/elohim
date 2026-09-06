---
id: "backlog-runtime-fleet-adoption-observability-doorway-projection"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Fleet-side adoption observability — /admin/adoption is storage-local, never doorway-projected"
slug: "runtime-fleet-adoption-observability-doorway-projection"
written: "2026-09-06"
author: "story-harvest"
status: "backlog"
priority: "medium"
relatedNodeIds:
  - "genesis/a2o/features/delivery/workspace-to-fleet-release.feature"
tags: [runtime-upgrade-propagation, doorway, observability, p2p-design-gate, ephemeral]
cites:
  - elohim/elohim-storage/src/services/release_adoption/state.rs
  - genesis/a2o/steps/delivery/workspace-to-fleet-release.steps.ts
  - genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md
---

## Chain

`runtime-upgrade-propagation` / between "the workspace peer's release crosses to the fleet
(Station 4 — carried election, resolvedHead observed on each fleet controller)" → "the
crossing is provable from OUTSIDE that one peer" / **missing node: a doorway-projected,
read-only fleet adoption summary — today the proof exists only on the workspace peer's own
`/admin/adoption`, so seven other controllers' resolution is unobservable externally.**

## Assertion + probe

Assertion: a released/staged content head's adoption state (channel, mode, resolvedHead
cid/tier, verdict, appliedRelease) is readable per-peer from a doorway endpoint, not only
from each storage peer's own loopback `/admin/adoption`. Probe: `curl
https://doorway-alpha.elohim.host/admin/adoption-summary` (or equivalent) returns an array
with one entry per known fleet peer; today no such route exists — the only way to check is
`curl http://127.0.0.1:<port>/admin/adoption` against each storage peer directly, which is
unreachable from outside the cluster/mesh.

## Current state — found live 2026-09-06, iteration 2 of the crossing

During Station 4 of the `workspace-to-fleet-first-crossing` Objective, the workspace peer's
own `/admin/adoption` showed `resolvedHead uhCkkqmSrk6…`, tier `staging`, verdict `ok`,
`appliedRelease null`, attestations `0/0` (threshold 1) — a real, verified reading. But
whether the **seven observe-mode alpha controllers** (the fleet peers carrying
`runtime:coordinators:elohim:workspace=observe`) resolved that same head to the same
verdict was **unobservable from outside** during the shift: Loki 502'd when queried for
corroboration, and there is no fleet-facing endpoint that aggregates or even proxies each
peer's own `/admin/adoption`. The crossing was provable only through one peer's local state,
which is not the same claim as "the fleet observed it."

## P2P design gate

Answered per the mandatory gate before any HTTP route is proposed: this is a **projection of
existing controller state**, not a new entity. Classification: **Ephemeral (C)** — it
reflects live in-memory adoption/watch state already held by
`release_adoption::state::AdoptionState` on each peer; nothing new is notarized, no new DHT
entry type, no new head-plane cost. The coordinator function is unchanged (no zome touched;
this is doorway-side aggregation of an HTTP read each storage peer already serves on
`GET /admin/adoption`). Identity: keyed by the existing peer/agent identity, not a new
minted id.

## Missing node (concrete shape)

A doorway route (or the existing conductor-diagnostics fan-out path) that, per known fleet
peer, proxies or caches `GET /admin/adoption` and returns `{peer, channel, mode,
resolvedHead: {cid, tier}, verdict, appliedRelease, attestations: {count, threshold}}`. Once
this exists, the runtime-harvest poller (`.claude/scripts/runtime-harvest.py`) and the a2o
Station 4 step both read the FLEET, not just the workspace peer, closing the exact gap this
shift hit.

## Habit served

`runtime-upgrade-propagation` (`elohim/elohim-storage/.epr-meta/runtime-upgrade-propagation.habit.md`)
— the invariant requires the release to reach "every peer... converged, and revertible" with
evidence; without fleet-side observability that evidence bottoms out at n=1.

## TODO (integrator)

`cites:` is a plain path list per this directory's existing convention (no fingerprint
invented). If cite-gen is later mandated for backlog rows, run it here rather than
hand-writing a fingerprint.
