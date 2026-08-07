---
id: "backlog-history-index-lapsed-14-records-unrowed"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "history/INDEX.md has lapsed since 2026-06-11 — 14 records carry no row"
slug: "history-index-lapsed-14-records-unrowed"
written: "2026-08-07"
author: "surfaced by the automerge 0.5.12 → 0.10.0 bump sprint (2026-08-07) while writing a new history record"
status: "backlog"
priority: "medium"
jobs: [genesis]
tags: [history, index, curation, discoverability, memory-hygiene]
---

## The concern

`genesis/docs/content/elohim-protocol/history/INDEX.md` stops at **2026-06-11**. Every history
record written since — **14 of them** — exists on disk with no row in the index.

Surfaced 2026-08-07 while adding
`2026-08-07-version-dag-lives-at-l2-not-in-the-crdt-doc.md`. That record's row WAS added; the
other 14 were not, deliberately, because backfilling them honestly is a different task (see
below) and doing it badly is worse than leaving it visible.

Named examples of unrowed records:

- `2026-07-12-substrate-convergence-five-defect-arc.md`
- `2026-08-04-holochain-iroh-dep-verification-pack.md`

## Why this matters more than a stale index usually does

The history tree is where settled decisions go so a future session does not re-derive them. An
unindexed record is functionally invisible to the reader who most needs it — the one who does not
already know it exists. That is the exact failure the 2026-08-07 record was written to prevent,
so the index lapsing is the same class of loss one level up.

Concretely: the 2026-08-07 record documents that a spec's requirement (REQ-F4's multi-version doc
structure) was superseded by what actually shipped. A reader who never finds that record builds
the superseded thing.

## Why this is not a 10-minute fix

`INDEX.md`'s rows are not filenames — each carries a one-line **lesson**. Writing an honest lesson
line requires reading the record. Fourteen records is a real curation pass, and a mechanically
generated index of titles would satisfy the file's shape while defeating its purpose.

## Fix sketch

1. Read each of the 14 unrowed records; write its lesson line in the established voice.
2. Add the rows in date order.
3. Consider whether the lapse should be prevented structurally rather than remembered — e.g. a
   check that every `history/*.md` has a matching `INDEX.md` row, in the same family as the
   existing placement-audit / cite-gen hygiene tooling. **Prefer this to a resolution to be more
   careful**; the file has now lapsed once for roughly two months without anyone noticing, which
   is evidence about the process, not about the people.

## Open question for whoever takes it

Is `INDEX.md` still the right surface at all, given MemPalace indexes this tree semantically and
`spec-coherence-index.py` covers it lexically? If both machine lenses already find these records,
the index's remaining job is *human* browsing — which is a real job, but a different one, and it
would change what a good row looks like. Decide that before backfilling 14 rows in the current
format.
