---
id: "backlog-security-doorway-blob-pantry-ungated"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Doorway blob pantry caches and re-serves any 200 blob body with no reach or Authorization re-check — one authorized fetch of a private blob makes it anonymous-readable until eviction"
slug: "security-doorway-blob-pantry-ungated"
written: "2026-08-23"
author: "fable-5 (red-team finding folded from the swarm-curve + blind-custody design pass)"
status: "refined"
priority: "high"
area: "doorway/serving-path"
domain: "protocol"
jobs: [elohim-holochain]
relatedNodeIds:
  - "habit:reach-enforced-everywhere"
  - "habit:blob-durability"
cites:
  - "swarm-curve-and-blind-custody-design | The swarm curve and blind custody | sha256:ef23b30ec9b8145c | path: genesis/docs/superpowers/specs/2026-08-23-swarm-curve-and-blind-custody-design.md"
  - genesis/data/timeline/backlog/arch-confidentiality-plane-backlog.md
tags: [security, doorway, blob, reach, cache, bounded-feature, codex-claimable, agent-agnostic]
---

# Doorway blob pantry is ungated

**Finding (CONFIRMED 2026-08-23, red-team lens).** `doorway/doorway-service/src/routes/storage_proxy.rs`
`forward_blob_to_storage` buffers the upstream body (~:905) and stocks the pantry on the bare
condition `status == 200 && status != 206 && len <= blob_pantry_max_bytes()` (~:911-917) — **no
reach check, no `Authorization` check**. The hit path (~:775-790) serves the cached bytes to any
later requester for that hash without re-evaluating `ctx.agent_cid`. The freshness pantry next door
already does this right: `routes/freshness.rs` `should_stock` (~:210) / `reach_is_stockable`
(~:243) refuses `Authorization`-bearing and non-public/commons bodies.

**Why it matters now.** Any private-reach blob (or, once blind custody lands, a key ring) fetched
once through a doorway by an authorized reader becomes anonymous-readable from that doorway until
eviction. This is a live `reach-enforced-everywhere` hole on the serve path — exactly the boundary
that habit's `first_move` names.

## Scope (doorway-service only)
1. Apply a `reach_is_stockable`-equivalent predicate to the blob pantry's stock decision: never
   stock a body whose request carried `Authorization`, and never stock a body whose reach (from
   the storage response headers / the EPR lookup the forward already does) is not public/commons.
2. Hit path: if a cached entry exists but the *current* request would not have been stockable
   (authenticated request for a non-public hash), bypass the pantry and forward.
3. Counter: `doorway_blob_pantry_skipped_total{reason="authz"|"reach"}`.

## DoD / verification
- Unit test: a 200 body for a private-reach hash with `Authorization` is NOT stocked; a commons
  body is; a second anonymous request for the private hash forwards (miss) rather than hits.
- `just gate doorway-service` → `GATE_EXIT=0` echoed on its own line.
- Household lane: `just test mesh features/dataplane/blob-replication.feature` and the
  `reach-enforced-everywhere` scenarios show no new red.
- Commit path-limited (`git commit -- <paths>`); never `--amend`.

## Disjointness
Write-set is `storage_proxy.rs` (+ its tests). Does not touch `freshness.rs`, elohim-storage, or
the blob swarm rows. Independent of every other row in the spec's §9.
