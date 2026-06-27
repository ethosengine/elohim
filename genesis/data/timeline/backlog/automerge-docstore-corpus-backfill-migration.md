---
id: "backlog-automerge-docstore-corpus-backfill-migration"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Automerge DocStore corpus back-fill — idempotent migration to project pre-existing seeded content into the sync plane"
slug: "automerge-docstore-corpus-backfill-migration"
written: "2026-06-27"
author: "plan flow Step 1c(3) complementary-capture (automerge content-sync plane sprint)"
status: "backlog"
priority: "medium"
jobs: [elohim]
---

## What

The content-sync plane lighting plan
(`genesis/docs/superpowers/plans/2026-06-27-automerge-content-sync-plane-lighting-plan.md`)
ships a **go-forward-only** producer: it projects content into the Automerge DocStore on
each *new* write (`ContentCreated`/`ContentUpdated`). It deliberately does NOT back-fill the
large already-seeded SQL corpus, and the sled DocStore wipes on PVC/sled reset.

Consequence: after a fresh deploy or sled reset, **existing seeded content does not
retroactively sync** until each node is re-written. This item is that missing back-fill.

## Why deferred (not in the spine)

Back-fill is O(total content rows), re-incurred each reset — an unbounded cost that would
blow the bounded per-write spine. It belongs in a separate, idempotent, **gated** one-shot
migration, never in the per-write listener path.

## Shape (to design when picked)

- A one-shot startup migration (gated by an env flag / a "docstore_backfilled" marker) that
  SELECTs all content rows and calls the producer's `project_content_doc` for each, under
  `h_app_id = "elohim"` (the load-bearing namespace — see the plan's Global Constraints).
- Idempotent: re-running must not duplicate change history (get_or_create_doc + only put if
  the doc heads are empty / value differs).
- Batched / rate-limited so it doesn't contend with the live write path or the 60s sync timer
  (note the bulk write path already pauses p2p sync for bulk ≥50, `http.rs:4460`).
- Decide the reset-recovery story: re-run on every cold start with empty sled, or a separate
  operator-triggered reconcile.

P2P-design-gate note: the Automerge docs are PROJECTIONS of already-notarized content
entities (not a new DHT entry type) — this is a projection-rebuild, not new-entity design.

Domain D5 (data plane). Depends on: the spine producer (`project_content_doc`) landing first.
Effort: M.
