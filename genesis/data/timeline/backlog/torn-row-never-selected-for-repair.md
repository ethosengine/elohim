---
id: "backlog-torn-row-never-selected-for-repair"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A torn declared row is never selected for repair — no candidate query probes on pointer provenance (story 1.4d)"
slug: "torn-row-never-selected-for-repair"
written: "2026-09-20"
author: "serving-edge failover-balance-stream campaign, 2026-09-20 review"
status: "open"
priority: "medium"
jobs: [elohim]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:doorway-failover"
tags: [dataplane, projection-reconcile, doorway-failover, torn-row]
---

**The fact.** `classify_content_gap` (`elohim/elohim-storage/src/p2p/projection_reconcile.rs:4454-4476`) is
the only predicate that decides whether a content row is a heal candidate, and none of its three
inputs reads the served `blob_cid` — a row with `declared_head_action_hash == dht_anchor_hash` and a
`blob_hash` that came from a different action classifies `InSync` and is never re-probed. The one query
that does look at a blob column, `list_half_blob_row_ids` (`content_diesel.rs:1964-1982`, `blob_cid IS
NOT NULL AND blob_hash IS NULL`), detects a *missing* pointer, not a *wrong* one, and its only consumer
(`sync/projector.rs:426`) is a different repair leg entirely. `pointer_heal_patch` (T7,
`services/head_adoption.rs:1720-1748`) can repair a torn row's pointer once it is probed for some other
reason, but nothing drives selection on torn-ness itself.

**Evidence.** `story-1.4a-design.md` §c.3 (`genesis/a2o/reports/recovery/serving-edge-20260919/story-1.4a-design.md:319-332`)
confirms this by exhaustive read of every candidate query feeding `discover_content`, not by inference.
Commit `77cf2ba48` (T-1, 2026-09-20) closed the producer that creates new torn rows — `adopt_local`
now carries the head's own `blob_cid` on a move — but its own commit message says plainly: "it heals no
row that is already torn." The live fleet row this was diagnosed against (elohim.host serving a blob its
declared head does not name) therefore stays torn until a conductor answers `canonical: true` for it, at
which point some *other* probe reason has to fire before T7 gets a chance to run.

**Why it matters.** A row can sit torn indefinitely with no mechanism watching for it — the only detector
is the advisory seam-6 probe (`scripts/ci/substrate-seam-smoke.sh`, `ADVISORY-TORN-ROW` rung, commit
`801ef1267`), which is a CI-time read of one named content id, not a sweep over the corpus. Between T-1
landing and any 1.4d work, "no candidate query selects a row because it is torn" is a permanent property
of the reconcile sweep, not a transient gap.

**Smallest next step.** `story-1.4a-design.md` §g risk R3 names two candidate shapes: extend
`classify_content_gap` with a pointer-provenance column (a cheap SQL comparison, one more `ContentGap`
variant), or add a standalone `list_torn_row_ids` keyset sweep alongside the existing `list_half_blob_row_ids`.
The design pass recommends filing this as its own story (1.4d) rather than growing 1.4a's slice — do that
design pass before writing code, since `stamp_declared_head_mode`'s three documented regressions
(`content_diesel.rs:1531-1594`) make this a function that punishes improvisation.

**Links.** Plan story: `genesis/docs/superpowers/plans/2026-09-19-serving-edge-failover-balance-stream-campaign-plan.md`
story 1.4 ("1.4d (new, unscheduled)"). Design: `genesis/a2o/reports/recovery/serving-edge-20260919/story-1.4a-design.md`
§c.3, §g R3. Habit: `doorway/doorway-service/.epr-meta/doorway-failover.habit.md`.
