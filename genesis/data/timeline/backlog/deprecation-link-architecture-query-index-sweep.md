---
id: "backlog-deprecation-link-architecture-query-index-sweep"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Retire the ~50 *By{Attribute} query-index link types (DHT-as-notary sweep)"
slug: "deprecation-link-architecture-query-index-sweep"
written: "2026-06-11"
author: "deprecation-triage"
status: "backlog"
priority: "medium"
deprecation_status: blocked
severity: medium
fingerprints: ["4b3ce06c317d"]
relatedNodeIds: []
tags: [deprecation, holochain, dna, link-types, dht-as-notary, content_store_integrity, records-lifecycle]
cites:
  - elohim/holochain/dna/LINK_ARCHITECTURE.md
  - genesis/docs/superpowers/plans/2026-05-24-records-lifecycle-part-d-substrate-gaps-plan.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md
  - elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs
  - elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
  - elohim/elohim-storage/src/services/reconcile_controller.rs
---

## What is deprecated

A checklist line in `elohim/holochain/dna/LINK_ARCHITECTURE.md` (§"Query-Only
Links"):

```
- [ ] Mark ~50 query-only links as DEPRECATED
```

This is NOT a tooling-emitted runtime warning — it is a static, unchecked
planning-checklist marker captured by the deprecation sentinel because the line
literally reads as a deprecation to-do. The underlying concern is the
**`*By{Attribute}` query-index link-type sweep**: ~50 `LinkTypes` enum variants
in `content_store_integrity` (e.g. `TypeToContent`, `TagToContent`,
`AuthorToContent`, `EventByAction`, `EventByLamadType`, `ResourceBySpec`, and
"many more `*By*` patterns") that exist purely to serve attribute queries. They
violate the DHT-as-notary principle (per `project_three_layer_truth_model`):
query indices belong in the SQL projection layer, not as notarized DHT links.
Every unretired `*By*` variant also burns a slot against the 256 link-type cap.

## Usage inventory

This concern is **already canonically scoped** in the records-lifecycle Part-D
substrate-gaps work — it is Backfill 3 of the three-coordinated substrate-floor
backfills. The authoritative usage/touch inventory lives there; reproduced here
as the citation anchor:

- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` —
  `LinkTypes` enum: retire the `*By{Attribute}` variants.
- `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` — coordinator
  functions that currently CREATE the `*By*` links: re-route their query
  workload to SQL-projection upserts via the ReconcileController.
- `elohim/elohim-storage/src/services/reconcile_controller.rs` — absorb the
  SQL-projection upserts for queries that previously traversed `*By*` links.
- `elohim/holochain/dna/LINK_ARCHITECTURE.md` — close the deprecation checklist
  and update the 256-cap accounting (this doc is itself slated for retirement in
  the holochain `dna/` island recompose; whichever lands first should carry the
  closure note).

Canonical homes (the decision record this entry points the sentinel at):
- Plan — `genesis/docs/superpowers/plans/2026-05-24-records-lifecycle-part-d-substrate-gaps-plan.md`
  (Task A2 / D.11 substrate-floor validator backfill, "Backfill 3 — LINK_ARCHITECTURE deprecation sweep", ~L125).
- Design — `genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md`
  (D.11 "Backfill 3 — LINK_ARCHITECTURE deprecation sweep", ~L2827).

## Migration path

Per the design's Backfill 3: each retired `*By*` link gets its query workload
moved to the SQL projection (the operational layer that should have carried it
from the start). Reclaimed slots return to the 256-cap budget so future
structural link additions (D.1's `EprToEvent` + `EprToResource`) can land
without crowding the cap. **Sequencing constraint:** Backfill 3 ships in Wave A
alongside Backfill 1 (attestation-validator floors, which depends on D.10
vocabulary governance landing first) and Backfill 2 (retention-class manifest
validator). The design is explicit that the three land together — shipping any
one in isolation leaves the substrate looking partially invariant-protected,
"which is worse than transparently un-protected."

## Current decision

**Blocked — already canonically tracked; needs the operator-initiated
records-lifecycle Wave A substrate sprint, not a background landing.** Two
reasons this is out of background-agent scope:

1. **It is a notarized-substrate change, not a config/rename/API-swap.**
   Retiring `LinkTypes` enum variants changes the integrity zome's accepted
   link set AND requires re-routing every coordinator that created those links
   to SQL-projection upserts — a DHT-truth-layer migration with validation,
   coordinator, and storage-projection touches that must land coherently.
2. **It is sequenced inside a coordinated three-backfill sprint** gated on D.10
   vocabulary governance (Backfill 1's dependency). The records-lifecycle plan
   already owns the sequencing, test surface, and operator-decision register;
   forking a fix here would duplicate that canonical home and risk landing
   Backfill 3 out of its required Wave-A coherence.

This entry exists only to give the sentinel a deterministic citation so the
checklist line stops re-firing a dispatch. The work itself is owned by the
records-lifecycle Part-D plan; the stasis sweep owns the re-check (and the
LINK_ARCHITECTURE.md island retirement may close the line incidentally — see
`genesis/data/timeline/backlog/pillar-island-recompose-recipe.md`).

## Verification

N/A — not yet fixed. On the records-lifecycle Wave A sprint, verification =
the integrity-zome test surface from the plan green (retired `*By*` variants no
longer accepted at write time; queries that previously traversed them resolve
via SQL projection), the DNA sweettest gate green, and the
`LINK_ARCHITECTURE.md` checklist line closed (or the doc retired in the island
recompose). Then delete fingerprint `4b3ce06c317d` from the ledger and this
entry.
