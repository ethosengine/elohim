---
id: "backlog-dataplane-reanchor-dead-remaining-rekeyed-peer"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Re-keyed dead anchors need a heal under the current key — dead_remaining survives a pod restart"
slug: "dataplane-reanchor-dead-remaining-rekeyed-peer"
written: "2026-09-06"
author: "story-harvest"
status: "backlog"
priority: "high"
relatedNodeIds: []
tags: [dataplane-convergence, reanchor, p1-reconciliation-controller, re-genesis]
cites:
  - elohim/elohim-storage/src/services/reanchor_backfill.rs
  - .claude/scripts/runtime-harvest.py
---

## Chain

`dataplane-convergence` / between "a content row's anchor is unresolvable under the current
conductor key" → "the row heals to a resolvable anchor (or a declared supersession)" /
**missing node: a controller-driven re-anchor under the CURRENT key for rows anchored by a
pre-re-genesis agent key whose chain nobody holds — today they just sit, stuck, forever.**

## Assertion + probe

Assertion: `dead_remaining` on a peer's reanchor sweep either (a) trends toward zero as rows
heal under the current key, or (b) is explicitly held with a declared supersession record —
never silently persists unchanged across a pod restart. Probe: `GET /p2p/status` on adam
before and after a pod restart — `reanchorDeadRemaining` and `deadRemainingStuck` must not
both survive unchanged (same nonzero count, still stuck) unless a supersession has been
filed for those specific rows.

## Current state — found live 2026-09-06

adam's provide loop shows `reanchorDeadRemaining: 9`, `deadRemainingStuck: true`. This
population **survived the #1432 pod restart**: `stuckSweeps` reset from 55 back down (fresh
counter after restart) then climbed straight back to 9 and re-asserted stuck, meaning the
restart did nothing to help these rows — they are anchored by a pre-re-genesis agent key
(from before the 2026-09-02 alpha re-genesis; see
`project_alpha_dna_migration_2026_09_02` memory) whose source chain no live peer holds, so
every candidate hits the reanchor sweep's skip-guards
(`elohim/elohim-storage/src/services/reanchor_backfill.rs` — the sweep's dead-vs-pending
split at the bottom of the function, `dead_remaining` recounted separately so "a
`dead_remaining` that never moves... is a seed-data correction waiting, not a heal in
progress" per the code's own comment). This is exactly that comment's predicted class,
observed live: a **re-keyed-peer class** of dead anchor that the current sweep logic
correctly detects as stuck but has no cure path for.

## Missing node (concrete shape)

Per P1 (`elohim-storage is a k8s-style controller that eagerly reconciles`), the cure is a
controller action, never a hand PATCH: either (a) a re-anchor pass that re-authors the row
under the CURRENT agent key when the old key's chain is confirmed unrecoverable, or (b) a
declared supersession record that lets `/p2p/status` report these rows as resolved-by-policy
rather than perpetually `deadRemainingStuck: true`. Either path needs a decision about what
"the current key" is authoritative to re-author on behalf of a dead pre-re-genesis identity
— an architect-level call, not a background mechanical fix, but the class itself (re-keyed
dead anchors from a re-genesis event) should have a named cure path rather than sitting as
an ever-present true/9 in `/p2p/status`.

## Note — already surfaced to the poller

The runtime-harvest poller (`.claude/scripts/runtime-harvest.py`, commit 9060c617d) now
files this class of exhaustion into the deterministic ledger
(`.claude/data/runtime-findings.jsonl`) rather than it going unnoticed; this backlog entry is
the canonical concern the poller's fingerprint should resolve to.

## Habit served

`dataplane-convergence` (`elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md`,
status: red, active: true) — invariant: "A peer that missed a deploy heals." A re-keyed dead
anchor is a peer that will never heal under the existing sweep logic; the habit's own guard
section already tracks re-genesis-era edge cases and this is a fresh, live instance.

## TODO (integrator)

`cites:` is a plain path list per this directory's existing convention (no fingerprint
invented). If cite-gen is later mandated for backlog rows, run it here rather than
hand-writing a fingerprint.
