---
id: "backlog-epr-slice2b-provide-loop-followons"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "EPR Slice 2b provide-loop — deferred follow-ons (graduation-audit, immutability sweettest, passive-commons scoring, un-provide UX)"
slug: "epr-slice2b-provide-loop-followons"
written: "2026-06-09"
author: "claude-opus-slice2b-execution"
status: "documented"
priority: "medium"
# Slice 2b (the user-facing provide loop) landed all 14 plan tasks on
# feat/native-content-graph-seam, functionally + visibly live, holistically
# reviewed. These are the non-blocking follow-ons the final review + per-task
# reviews surfaced. None breaks the v1 provide-loop invariants.
relatedNodeIds:
  - "backlog-ci-storage-workspace-tests-uncovered"
tags: [epr, slice-2b, provide-loop, replicates-commons, follow-on, hardening, household-nodes]
cites:
  - genesis/docs/superpowers/specs/2026-06-08-epr-acquisition-slice2b-provide-loop-design.md
  - genesis/docs/superpowers/plans/2026-06-08-epr-acquisition-slice2b-provide-loop-plan.md
  - elohim/elohim-storage/src/rea_projection.rs
  - elohim/elohim-storage/src/services/replication_prioritizer.rs
  - elohim/elohim-storage/src/services/conductor_commitment_author.rs
---

# EPR Slice 2b provide-loop — deferred follow-ons

All 14 plan tasks shipped (gate + DNA action/validators/schemas + storage
projection/validator/emit + provide-reconciler + revocation + CommitmentByState
graduation link + scorer Medium arm + per-EPR view/API + Angular rung-4 + a2o),
holistically reviewed. These follow-ons are non-blocking for v1.

## 1. Graduation race — `proposed`-stuck if signal ordering is unlucky (hardening)
The author creates the commitment then immediately emits the `ProvideAnnounce`
(bounds-checked via `ConductorCommitmentFetcher`, which reads the conductor
directly — correct). But graduation runs in `rea_projection` when the EVENT
projects: it calls `graduate_to_active(cid)` which needs the commitment's
`mishpat_commitments` row (projected async via the `CommitmentCommitted` signal,
now wired). If the event projects BEFORE the commitment row lands,
`graduate_to_active` returns 0 rows (no-op) and there is no retry — the row stays
`proposed` and the `CommitmentByState` link is never authored.
**Bounded severity:** the bounds-gate uses `dht_anchor_hash NOT NULL` (set by the
commitment projection), not `state`, so subsequent provide events are still
accepted — the loop is not broken; only the `state` column + the DHT state-link
lag. **Fix:** a periodic graduation-audit sweep — for `mishpat_commitments` rows
`state='proposed'` with a matching `ProvideAnnounce` (`bounded_by==cid`) in
`economic_events`, re-run graduation + state-link authoring.

## 2. Commitment-immutability sweettest (coverage gap)
Spec §10 called for a sweettest that creates a `Commitment`, attempts an
`update_entry`, and asserts rejection. The integrity zome's `validate_update_entry`
DOES refuse updates, but no dedicated sweettest exercises the rejection path end-
to-end (it's only implicitly covered by the `to_app_option` readback pattern).
One-test follow-on: `elohim/holochain/tests/sweettest/.../mishpat_commitment_immutability.rs`.

## 3. Passive-commons scoring (Slice-3, gossip-wire)
`score_advertised_blob` returns Medium correctly, but the passive replication
gossip path passes `content_id_ctx=None`, so commons-Medium fires only from the
acquisition reconcile loop (which supplies `head_ref`). This is the intentional
v1 design (spec §6.5 — no `BlobHint` wire change in v1). Widening `BlobHint` to
carry `content_id`/`head_ref` so passive gossip can score commons-Medium is
Slice-3, alongside the closure resolver (the `blob_cid ↔ content_id` bridge).

## 4. Un-provide cancel UX (next rung)
The rung-4 `PinProgressComponent` "cancel" is UI-local (stops polling, removes the
bar) — it does NOT revoke the provide server-side. Real un-provide is a REA
revocation: the existing `DELETE /api/v1/pins/{id}` triggers the T10
`revokes-commitment` author path. Wiring the UI cancel to that DELETE (with the
"you'll stop serving these bytes" affordance) is the next rung — p2p-design-gate
territory for the UX, the backend revocation already exists.

## Status
`documented` — pick up post-merge. None blocks the Slice-2b landing.
