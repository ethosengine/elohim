---
id: "backlog-ci-seeder-stamp-conductor-anchor-circularity"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Bulk-seeded rows can't gain provenance: reach-carrying stamp PATCH routes via conductor, which can't re-author never-anchored entries (genesis #1121, 3354/3429 stamp-failed)"
slug: "ci-seeder-stamp-conductor-anchor-circularity"
written: "2026-06-11"
author: "agentic-developer (EPR durability shift, #1121 Grafana correlation)"
status: "wip"
priority: "high"
ci_status: pending-verification
jobs: [elohim-genesis]
tags: [substrate, seeding, provenance, dht-anchor, reach, conductor, coherence-gap]
cites:
  - genesis/seeder/src/seed-sqlite.ts
  - elohim/elohim-storage/src/http.rs
---

# Stamp ↔ conductor ↔ anchor circularity

## Symptom (genesis #1121, first run against the edge-#1058 storage image)

`Batch N/35: 0 inserted, 100 skipped, X stamp-failed` — 3,354 of 3,429 items
failed the provenance stamp (count ≈ the 3,354 items that resolved to
`private` under inverted-burden). **Zero** stamp failures in #1119 against
the previous storage image. The seeder's reach values are valid (canonical
enum includes `private`).

## Mechanism (read from http.rs PATCH branch, ~line 4197)

`stampProvenance` PATCHes `{p2pPublishedAt, reach}`. The NEW storage routes
any patch touching a DNA-notarized field (`blob_hash` or `reach`) through
`ContentService::update_via_conductor` (substrate-correct: closes the
diesel-write-reverted-by-reconciliation gap, Task 8d / reach-floor G1). But
bulk-seeded rows were **never DHT-authored** (the known bulk-seed anchor
gap) — the conductor cannot re-author a nonexistent entry, so the PATCH
fails. Old image: same PATCH fell through to diesel → 200. Net: the very
stamp that grants `require_provenance` read-gate passage is now unreachable
for exactly the rows that need it. Circular.

## Mitigation landed (this shift)

`stampProvenance` retries failed reach-carrying PATCHes with
`p2pPublishedAt`-only (metadata path, diesel) so provenance lands and reads
work; logs a `reachSkipped` count. Reach reconciliation for those rows is
DEFERRED — honest warn, not silent.

## Real fix home

The bulk-seed **anchor step**: seeded content must be DHT-authored (conductor
`create` path or import-anchor batch) so reach re-notarization has an entry
to update — same home as the long-standing local-stack DHT-anchor gap. When
that lands, the fallback in stampProvenance becomes dead code to remove.

shift_objective: |
  Land the bulk-seed anchor step so seeded rows are DHT-authored at ingest,
  reach re-notarization works through the conductor path, and
  stampProvenance's provenance-only fallback is removed (zero reachSkipped
  on a full genesis seed).
