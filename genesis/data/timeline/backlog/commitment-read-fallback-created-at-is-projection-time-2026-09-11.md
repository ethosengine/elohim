---
id: "backlog-commitment-read-fallback-created-at-is-projection-time"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A commitment fetched by cid from the DHT (read fallback) reports createdAt = projection time and an entry-derived state — the notarization time lives in validFrom and the CommitmentByState links are not read"
slug: "commitment-read-fallback-created-at-is-projection-time-2026-09-11"
written: "2026-09-11"
author: "doorway-federation sprint — Task 17c (storage read-back fallback, fb4d10c7d)"
status: "open"
priority: "low"
tags: [elohim-storage, mishpat, commitments, read-fallback, semantics, C4, C5, D1]
relatedNodeIds: []
cites:
  - elohim/elohim-storage/src/services/commitment_read_fallback.rs
  - elohim/elohim-storage/src/api/rea_commitments.rs
  - elohim/elohim-storage/seam-registry.yaml
  - genesis/a2o/features/auth/hosted-human/07-hosted-by-a-household.feature
---

# What a DHT-fetched commitment says about itself

## Landed (fb4d10c7d)

`GET /api/v1/commitments/{id}` now falls through `rea_commitments` → `mishpat_commitments` → ONE bounded
`mishpat::get_commitment` via the peer's own conductor → projects the row. Any peer holding the mishpat cell can read a
notarized commitment back by cid (story 07 scenario 3's claim, proven live on jessica `:8091`).

## Two semantics left open, by design of the single-call budget

1. **`createdAt` on a fetched row is the projection moment**, not when the notary witnessed it. `validFrom` carries the
   grant's own start; the action's timestamp is on the `dhtAnchorHash` action. Story 07 scenario 5 ("carrying the date it
   was made") passes because the step only asserts non-empty — the value is not the date it was made. Decision needed:
   surface the action timestamp as `notarizedAt` (from the record's action header, available in the same `get`), and
   keep `createdAt` as projection time, or rename.
2. **State is entry-derived.** The fallback reads the entry, not the `CommitmentByState` links, so a commitment revoked
   provider-side and FIRST seen by a non-authoring peer projects as `proposed`/live until a reconcile reads the links
   (`gapNote` on seam row `read_notarized_with`). A second bounded read of the state links at fallback time — or the
   reconcile arm treating fallback-projected rows as amber until the links are read — closes it. Until then, a stale
   "live" answer is possible for a revoked grant on a peer that never saw it before.

Neither is a regression: before fb4d10c7d the answer was 404 on every peer, including the issuer.
