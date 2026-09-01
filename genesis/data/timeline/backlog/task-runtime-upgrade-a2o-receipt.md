---
id: "backlog-task-runtime-upgrade-a2o-receipt"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Task: the rung-5 a2o story + mesh receipt — publish → elect → adopt → attest → promote → converge → revert-by-re-election, with one peer riding an experiment channel throughout"
slug: "task-runtime-upgrade-a2o-receipt"
written: "2026-09-01"
author: "session-2026-09-01-rung5-design"
status: "open"
priority: "high"
jobs: [elohim-genesis, elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-upgrade-propagation-p2p-design-arc"
  - "spec:runtime-artifacts-elected-content"
  - "backlog-task-release-channel-ceremony-driver"
  - "backlog-task-release-apply-vehicles"
tags: [upgrade-propagation, rung5, a2o, receipt, story-first, delegable]
---

**Claimable by any implementation agent. The STORY half can (and should)
start immediately — story-first is the repo default; the RECEIPT half is
green only when T1-T4 land (T5 enriches it). This is the spec's
graduation-trigger artifact (§10).**

## Why

The receipt is the whole point of the arc: the moment this passes on the
mesh, the CI roll is no longer the delivery path for the coordinator class,
and the spec's constitutional posture has its first lived demonstration —
the protocol stewarding itself, and the diversity that teaches it.

## P2P design-gate decision

Ephemeral (C) throughout — story, steps, and receipt script stage fixtures
and read receipts; nothing new is persisted or notarized. C4: every
convergence assertion reads from EVERY peer, and unreachable ≠ absent.

## Scope

1. **Story** (`genesis/a2o/features/delivery/runtime-upgrade-propagation.feature`,
   `@concern:runtime-upgrade-propagation @requires:household-nodes`, act
   placement per `project_tests_layered_as_acts_of_one_story`): a household
   mesh receives a coordinator release through the ceremony — staged, soaked
   by a canary, promoted on evidence, converged, and REVERTED by re-election
   when the household finds it wanting — while one peer rides a compatible
   experiment channel and is heard, not outvoted. Written in learner/household
   language, NOT ops language. **MUST enter the context-isolated blind-reader
   revision loop required by `genesis/a2o/.epr-meta` before merge.**
2. **Steps + receipt script**
   (`genesis/a2o/scripts/runtime-upgrade-receipt.ts` + step definitions
   composing it): drives T2's ceremony verbs and reads T3/T4's
   `/admin/adoption`, `/version` coordinator hashes, and T2 `status` JSON.
   Receipt = the §10 chain with timings per station; exit 2 names the
   stalled station. Negative controls: an envelope-broken release is REFUSED
   by every peer's verify arm (typed reason observed); an unauthorized
   declare is refused (T2's control).
3. **Close the loop**: cycle-time delta row appended to the arc doc's table
   (`upgrade-propagation-p2p-design-arc.md`) + a one-line DELTA to the habit
   ledger the a2o `@concern` joins (no status flip without fleet evidence —
   flip authority stays with observation).

## Disjointness contract

- MAY create the feature file, steps, receipt script; append the arc-doc
  table row + habit delta; edit this atom.
- MUST NOT edit Rust source, zomes, `hc-mesh.sh`, or sibling task scripts —
  a missing station is reported as a story-graph node (chain / between /
  assertion + probe / current state), never patched around from here.

## DoD + verification

- Story passes the blind-reader loop; `just test mesh
  genesis/a2o/features/delivery/runtime-upgrade-propagation.feature` green
  twice consecutively on a fresh mesh (fresh channel ids each run).
- The receipt transcript shows: staging convergence on 3/3, canary
  adopt+attest, earned promotion, 3/3 apply convergence with conductor PIDs
  unchanged, revert convergence, and the experiment-channel peer diverging
  compatibly throughout.
- Arc-doc cycle-time row + habit delta appended; `habits-project.py --check`
  clean.
