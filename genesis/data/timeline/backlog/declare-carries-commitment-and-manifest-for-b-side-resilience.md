---
id: "backlog-declare-carries-commitment-and-manifest"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "ch10 needs declare-carries-Commitment + carried-manifest — the resilience card's stewardingCollectives (shard_locations) and commonsCommitments (rea_commitments) are gossip-blocked on B, and declare-carries-Record covers neither table"
slug: "declare-carries-commitment-and-manifest-for-b-side-resilience"
written: "2026-07-26"
author: "claude (resiliency-saga sprint-3 delivery — ch10 root-cause trace)"
status: "open"
priority: "high"
ci_status: blocked
jobs: [elohim-edge]
tags: [resiliency-saga, ch10, declare-carries, commitment, shard-locations, rea, mishpat, gossip-gap, full-arc, carried-record]
cites:
  - resiliency-saga-sprint3-objective | Resiliency Saga Sprint 3 Objective | path: genesis/docs/superpowers/plans/2026-07-26-resiliency-saga-sprint3-objective.md
  - security-declare-carries-record-carried-evidence-bounds | carried-record bounds | path: genesis/data/timeline/backlog/security-declare-carries-record-carried-evidence-bounds.md
  - elohim/elohim-storage/src/services/household_resilience.rs
  - elohim/elohim-storage/src/db/rea_commitments.rs
  - elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
---

# ch10 is a distinct gap from ch06 — declare-carries-Record does not cover it

## The finding (2026-07-26, sprint-3 delivery, live-measured + traced)

Sprint-3's centerpiece (declare-carries-Record, commit b91168724) carries a
**content HEAD Record** so a canonical head lands on B without gossip — that
flips **ch06** (anchor equality). But **ch10** (the resilience card reporting
the same non-zero `stewardingCollectives` on both doorways) has a **separate
root cause the centerpiece never touches**, confirmed by read-path trace:

- `stewardingCollectives` = `resiliency::stewarding_hubs(relation).len()` — reads
  **`shard_locations ⋈ humans ⟕ collectives`** (`household_resilience.rs:58-94`,
  fold `elohim-facings/src/folds/resiliency.rs:25-27`). **Not commitments.**
- `commonsCommitments` = `commitment_backed_replication_for_households(...)` —
  reads **`rea_commitments`** filtered `action IN REPLICATION_RELATION_ACTIONS
  AND state='active'` (`db/rea_commitments.rs:750-812`), projected from Mishpat
  Commitment entries. It **short-circuits to 0 when `stewardingCollectives` is 0**
  (`replication_commitment.rs:242`) — so B's two zeros are not independent.

Both tables are populated only by **local-cell** producers: `signals::
handle_mishpat_signal` on `CommitmentCommitted` (fires only when the entry is
committed/received in *this* conductor's cell) and the `projection_reconcile`
REA arm, which **discovers** gaps from peer inventory but **heals from the
node's OWN conductor** (`get_rea_commitment` → `None` while ungossiped →
`mark_failed`, retried forever; design contract `projection_reconcile.rs:21-35`
"peer bytes are NEVER written"). On B (adam) both tables are empty because the
entries never gossiped there — the same full-arc (`target_arc_factor=1`) gap
b91168724 was written to bypass **for content**. `mishpat_mirror_backfill`
reports `candidates=0` on B precisely because it mirrors an empty local ledger;
it is structurally incapable of manufacturing the rows.

There is **no `carried_commitment` / no declare-carries on any Mishpat/REA
Commitment or on shard_locations** — grep of all `carried_record` uses returns
content-head sites exclusively.

## The work (new feature — analogous to declare-carries-Record)

1. **declare-carries-Commitment.** Extend the mishpat/REA commitment-authoring
   extern with `Option<carried_record>` for the notarized Commitment entry; add
   a source-side `get_record_for_commitment` read on the authoring conductor;
   add a wasm `validate_carried_commitment` mirroring `validate_carried_record`
   (`content_store/src/lib.rs:3287` — action-hash equality + author signature +
   entry↔action binding). Then B's existing `handle_mishpat_signal` mirror +
   `projection_reconcile` REA heal succeed because the entry is locally
   retrievable.
2. **carried-manifest for shard_locations.** `stewardingCollectives` reads
   `shard_locations`/`shard_manifests`, NOT commitments — so a commitment carry
   alone lights `commonsCommitments` but leaves `stewardingCollectives=0` and,
   via the empty-scope short-circuit, zero commons anyway. A companion
   carried-manifest declare (or extending the content carry to cover the shard
   manifest) is required for the card to actually converge.
3. **Doorway carry step.** A `stage-spa-blob.sh DECLARE_ONLY`-style leg so that
   when B declares/imports content it also carries the covering `replicates-*`
   commitment Records and shard manifest, landing them on evidence not gossip.

## Carries the same carried-evidence caveat

This inherits the bound recorded in
[[security-declare-carries-record-carried-evidence-bounds]]: carried entries
prove self-consistency, not DNA-membership. A `validate_carried_commitment`
must land the same fifth author-membership gate before the declarer
authorization is tightened.

## Ceiling classification

New design + feature work beyond sprint-3's declare-carries-Record scope —
routes to `/brainstorm` (P2P design gate: notarized-entry class, DHT-carry
mechanism), not an iteration-loop grind. The sprint-3 objective conflated ch06
and ch10; ch06 ships this sprint, ch10 is this item.
