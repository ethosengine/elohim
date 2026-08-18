---
id: "backlog-orphan-shard-location-hygiene-sweep"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Orphan shard-location hygiene sweep — drain the honest-unknown custody population"
slug: "orphan-shard-location-hygiene-sweep"
written: "2026-08-18"
author: "claude-fable-5 (ch07 custody-rotation session)"
status: "backlog"
priority: "medium"
relatedNodeIds: []
tags: [dataplane, custody, shard-locations, alpha, metrics]
cites:
  - genesis/docs/superpowers/specs/2026-06-24-blob-custody-phase3-xor-salvage-placement-design.md
  - elohim/elohim-storage/src/services/custody_rotation.rs
  - genesis/a2o/features/dataplane/resiliency-saga/07-custody-witnessed.feature
shift_objective: |
  A bounded, level-triggered sweep that prunes shard_locations rows whose
  shard_hash is named by no shard_manifests row AND is older than N days,
  counted per decision outcome, so elohim_custody_class_count{class="unknown"}
  reads as coverage rather than noise. Probe: unknown count trending down
  across deploys while stocked holds.
---

# Orphan shard-location hygiene sweep

`elohim_custody_class_count{class="unknown"}` = 1543 on alpha-A (2026-08-18):
shard_locations rows keyed under superseded bundles' shard hashes that no
manifest names any more. `prune_orphaned_locations` only runs for hashes
superseded by LOCAL manifest re-stamps (`shard_manifest_backfill` Part B), so
peer-announced rows from rotated-away bundles accumulate forever.

Saga ch07's finish line (`stocked >= 1`) is deliberately independent of this —
see the feature preamble's scope note in `07-custody-witnessed.feature`. This
entry is the "separate hygiene chapter" that note names.

Fix shape: a bounded level-triggered sweep (locations whose shard_hash joins no
shard_manifests row for the scope AND last_seen older than N days → prune,
with per-reason counters and a per-pass cap), honoring the content-addressing
safety argument in `db::shard_locations::prune_orphaned_locations` (a hash any
manifest still names survives). TDD: orphan pruned; still-named hash survives;
cap bounds the pass; counters increment per decision.
