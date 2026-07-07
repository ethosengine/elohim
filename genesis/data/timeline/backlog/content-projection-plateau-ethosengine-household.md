---
id: "backlog-content-projection-plateau-ethosengine-household"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Content-projection divergent_anchor plateaued on the ethosengine household (james/jessica/matthew) — storm-independent, storage-deploy did not touch it"
slug: "content-projection-plateau-ethosengine-household"
written: "2026-07-07"
author: "pipeline-shakeout shift (storm-heal verification)"
status: "proposed"
priority: "medium"
area: "dataplane/content-projection"
domain: "household-nodes"
jobs: [elohim-edge]
relatedNodeIds:
  - "memory:project_inventory_exchange_not_byte_replication"
  - "memory:project_per_node_memory_is_conductor_authority_arc"
  - "memory:feedback_household_nodes_is_the_stable_floor"
  - "memory:project_alpha_topology_bootstrap_pair"
cites:
  - genesis/data/timeline/backlog/ci-alpha-cluster-degraded-substrate.md
tags: [dataplane, content-projection, convergence, divergent-anchor, household-nodes, ethosengine, node-locality, storm-independent]
---

# Content-projection plateau on the ethosengine household — separate from the inventory-snapshot storm

## What was observed (2026-07-07 ~00:05–02:06Z, alpha)

During verification of the inventory-snapshot storm-heal (see
`ci-alpha-cluster-degraded-substrate.md` and the receive-side snapshot idempotency fix
`b1ef627ed`), an observability re-probe of the alpha mesh — *after* the operator deleted the 7
stale storming conductors — found the storm cleanly resolved on every axis EXCEPT one.

There are two distinct reconcile loops in the conductor logs:

- **DHT anchor sweep** (`projection-reconcile: sweep complete`) — HEALTHY on all sampled peers:
  `caught_up: true`, `divergent_anchor` 0–6.
- **Content-projection sweep** (`projection-reconcile[content]: content sweep complete`) — SPLIT
  by hosting node:
  - **shem-hosted** (susan, gertrude): fully converged — `divergent_anchor: 0`,
    `conductor_missing: 0`, `local_anchored: ~4158`, stable since ≥01:11Z.
  - **ethosengine-hosted M/J/J household** (james, jessica, matthew): **plateaued, not
    trending down** across the full 2h window, *both before and after* the stale-pod deletion:
    - james `divergent_anchor` ~2212–3268 (oscillating, no trend), `conductor_missing` 2461–3402
    - jessica `divergent_anchor` ~2105–2161 (flat)
    - matthew `divergent_anchor` ~1941–2116 (flat)
    Reconcile sweeps run ~every 5 min, so this is ~20+ sweeps with no visible progress — reads as
    plateaued, not "still catching up."
  - **adam, eve**: content-sweep state UNVERIFIED — no matching `content sweep complete` log
    lines found in the probed window (needs a wider window or a different message string).

The deploy's own dataplane-validation stage reported `divergentAnchor=1533` (camelCase); the logs
carry snake_case `divergent_anchor` and no exact 1533 match was found — likely the same
content-projection signal sampled at a different moment, but the exact provenance of the 1533
figure is unconfirmed.

## Why this is a SEPARATE concern (not the storm)

- The plateau is **flat before AND after** the storm-pods existed / were deleted → not caused by
  the inventory-snapshot amplification storm (which was CPU/write-amplification and is now fully
  stopped: zero storm-pod log volume since ~01:50Z, 503s zero, DHT anchor sweep converged).
- The storage-image storm fix (`b1ef627ed`, receive-side snapshot idempotency) targets the
  inventory-apply path, not content-projection replication → it did not (and was not expected to)
  move this number.
- It is **node-localized**: converged on shem, plateaued on ethosengine's household. That locality
  is the strongest clue.

## Likely-related known gaps (hypotheses, cross-linked — dedup before deep work)

- `[[project_inventory_exchange_not_byte_replication]]` — inventory gossip is metadata-only; byte
  replication is a separate plane. `conductor_missing: ~2461–3402` on M/J/J reads like content
  anchors the projector expects but the household conductor does not hold locally → possibly the
  content-byte-replication tail, not a projection bug.
- `[[project_per_node_memory_is_conductor_authority_arc]]` — if the ethosengine household's
  conductors run a reduced arc (or are mid-fill), those anchors legitimately aren't local yet.
  Check `target_arc_factor` and the household conductors' arc coverage vs the shem peers'.
- `[[feedback_household_nodes_is_the_stable_floor]]` — M/J/J is the household floor; a persistent
  content plateau there is worth understanding before leaning on it as the deep-prove surface.

## Not blocking the storm-heal close

This does NOT gate the storm-heal objective: genesis "Seed Substrate" failed on conductor
`call_zome` CPU starvation from the storm (now fixed), not on content-projection convergence;
household/agent-peer bindings are identity-DHT ops, not content projection. So the storm-gated CI
stages are expected to recover independently of this plateau. Flagged so a fresh confirming
trigger that still shows content-read failures on the M/J/J household is read as THIS, not a
storm regression.

## Next (a future shift, not this one)

1. Confirm whether adam/eve's content sweep is converged or also plateaued (wider log window).
2. Determine node/arc cause: are the ~2500–3400 missing anchors absent from the household
   conductor's local store (byte-replication tail), or present-but-unprojected (projection bug)?
   Check per-peer blob counts (per `project_inventory_exchange_not_byte_replication`: metadata
   count ≠ bytes) before concluding.
3. Dedup against any existing content-replication / diversity-placement backlog before deep work.
