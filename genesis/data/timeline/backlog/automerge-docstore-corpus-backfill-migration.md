---
id: "backlog-automerge-docstore-corpus-backfill-migration"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Automerge DocStore corpus back-fill — idempotent migration to project pre-existing seeded content into the sync plane"
slug: "automerge-docstore-corpus-backfill-migration"
written: "2026-06-27"
author: "plan flow Step 1c(3) complementary-capture (automerge content-sync plane sprint)"
status: "in-review"
priority: "medium"
jobs: [elohim]
---

## LANDED (producer side) — 2026-07-01 (committed feat branch, CI-gate pending)

Implemented by the `automerge-content-sync-projection-completeness` shift:
`sync::projector::backfill_content_docs` (gated `ELOHIM_DOCSTORE_BACKFILL`, batched,
yields) over `content_diesel::list_all_content_rows` (unscoped, provenance-ungated),
projecting via the now-**idempotent** `project_content_doc` (skip-if-unchanged, so
re-runs never inflate change history). Wired one-shot at startup (main.rs libp2p
block). Full-field projection now includes `blobHash`/`serverBlobHash`/`blobCid`/
`contentSizeBytes`. Verified: 11/11 `sync::` lib tests green (incl. DB-driven
`backfill_projects_all_rows_and_is_idempotent`). Reset-recovery story = re-run on
cold start with the env flag (idempotent, safe). **Note the capstone is still open:**
the consumer heal path — see [[automerge-consumer-reverse-projection-docstore-to-sql]].
Close this item when the change clears the dev CI gate.

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
