---
id: "backlog-bulk-seed-witness-bootstrap-single-head"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Bulk-seed must carry the witness — author ONE notarized content head through the conductor at ingest, not un-witnessed diesel-direct rows that can never green"
slug: "bulk-seed-witness-bootstrap-single-head"
written: "2026-07-07"
author: "frontend-eyes-sprint (amber single-head arc)"
status: "in-progress"
priority: "high"
area: "substrate/content-ingest-notarization"
domain: "operator"
jobs: [elohim, elohim-genesis]
relatedNodeIds:
  - "memory:project_local_stack_dht_anchor_gap"
  - "memory:project_inventory_exchange_not_byte_replication"
  - "memory:project_versioned_entity_head_is_declared_dependency"
  - "memory:feedback-backend-authoritative-frontend-senses"
  - "memory:feedback-cleanup-toward-p2p-dataplane-trajectory"
cites:
  - genesis/docs/superpowers/specs/2026-07-01-crdt-authoritative-content-state-dht-notary-decouple-design.md
  - genesis/data/timeline/backlog/adam-genesis-anchor-sustained-saturation-post-storm.md
tags: [substrate, content-ingest, notarization, single-head, witness-bootstrap, amber-green, dht-anchor, bulk-seed, content-bulk-created, import-anchor-step, compose-with-decouple-spec]
---

# Bulk-seed must carry the witness — one notarized head, not un-witnessed rows

## The principle (architect, 2026-07-07)

There is only ONE EPR/content head. Its trust signal is **derived**, not written:
**green** = the Holochain DHT has agreed/witnessed the claims from this head *in context
with the rest of the network* (the "https padlock"); **amber** = provisional, that
DHT-agreement has not been established yet. A CID can differ between peers (it can carry
perspective — which peer is viewing the same object); green is the DHT witnessing *your*
head's claims, never a global byte-equality check.

The corollary the architect stated: **"we're trying to bootstrap that agreement with the
seed, so we shouldn't see them stay stuck anywhere."** The seed/ingest step must itself
provide the witness bootstrap — author the head *through the conductor* so the DHT witnesses
it and it converges to every peer. The only legitimate case for multiple simultaneous
versions is an intentional collective-edit DAG (an origin-claims **head** coexisting with an
in-dev-claims **branch**) — never an un-witnessed per-peer orphan claim.

## What this session already fixed (the deploy head)

The SPA-bundle deploy path is now single-head (frontend-eyes-sprint):
- `stage-spa-blob.sh` / root `Jenkinsfile`: byte-seed per host (bytes don't auto-replicate),
  but author the `blobHash` head **once** through a conductor-bridged doorway (failover), then
  converge via `run_content_sweep`.
- `elohim-storage`: removed the `?deployTier=amber` / `update_amber` diesel-direct per-host
  write (the un-witnessed divergent-head manufacturer). A notarized PATCH with no conductor
  bridge now honestly 503s and fails over. The amber *serving floor* + `crdt_converged_at` +
  the DocStore reverse-projection drift-heal are KEPT (that is the derived amber signal +
  convergence-window serving, not a write path).

## The gap this item tracks (the bulk-seed / ingest head)

The **general content ingest path still does NOT carry the witness**:
- `spawn_content_projection_listener` intentionally IGNORES `ContentBulkCreated`
  (`elohim/elohim-storage/src/sync/projector.rs` — handles `ContentCreated`/`ContentUpdated`
  only). Bulk-seeded rows never enter the DocStore/CRDT plane.
- Bulk-seeded content never DHT-anchors (`project_local_stack_dht_anchor_gap`: "local bulk
  seed never DHT-anchors → provenance gate 404s all reads by design; real fix = import anchor
  step"). So a freshly-seeded corpus is a field of **un-witnessed (amber-only) rows** that
  cannot green until something re-authors them through a conductor.

Result: seeded content can look "stuck" (amber, un-converged) on peers even though the mesh
is healthy — the ingest never established the witnessing context. This is the same
category error the deploy amber-write committed, one layer down.

## The fix (compose, don't fork)

Compose with the DRAFT decouple spec (`2026-07-01-crdt-authoritative-content-state-dht-notary-decouple-design.md`)
— do NOT redesign. The shape:

1. **Import-anchor step**: at ingest, author each content head ONCE through the conductor
   (`content_store::create_content` via `update_via_conductor` / the seeder's conductor path),
   producing a real `dht_anchor_hash` the DHT witnesses and gossips — the single head.
2. **`ContentBulkCreated` on the witness path**: either project bulk-created rows into the
   DocStore/notarize path, or drive the reconcile sweep to anchor them, so a bulk seed is not
   permanently un-witnessed.
3. Keep bytes on the byte-plane (per-host load-spread; bytes don't auto-replicate yet —
   `project_inventory_exchange_not_byte_replication`), heads on the DHT witness plane.
4. Versions remain a declared-HEAD DAG (`project_versioned_entity_head_is_declared_dependency`)
   — single-head-per-author never forbids a legitimate collective-edit branch.

## Landed (2026-07-09, feat/frontend-eyes-sprint — code-complete, awaiting live verification)

Both legs of "The fix" are implemented, independently reviewed, and committed:

- **Value plane (fix shape #2)** — `c130e46d1`: `spawn_content_projection_listener` no longer
  ignores `ContentBulkCreated`; ids route through `project_content_doc_reconcile`
  (offer-not-fight — a partial re-seed can never regress a peer-converged doc value), chunked
  200/chunk, spawned off the listener loop (single-flight) so a ~3.4k-id seed cannot lag the
  broadcast channel into dropped events.
- **Witness plane (fix shape #1, sweep-driven variant)** — `b7e010214`: the reconcile heal leg
  runs a bounded `witness_bootstrap` step composing the existing
  `reanchor_backfill::run_once` mechanism — NULL-anchor rows are authored once through the
  conductor (author-first-idempotent: duplicate refusal recovers the existing head; never a
  second head), capped 200 rows/tick + 25ms spacing. Heals the pre-existing seeded corpus, not
  just new seeds. Counter: `elohim_content_witness_authored_total`.
- Supporting reliability work (same arc): `4389bb8a4` (discovery/heal decouple + view-federation
  timeout layering + per-protocol /metrics), `fd22cc9b2` (panic-safe heal single-flight flag).

Remaining for DoD: live-GREEN verification on alpha after deploy (seeded corpus greens on the
authoring node, converges to peers) and the a2o scenario below.

## Definition of done

- A bulk-seeded corpus greens (DHT-witnessed, `dht_anchor_hash` present) on the authoring node
  and converges to peers via `run_content_sweep` — no un-witnessed amber-only rows left after
  ingest completes.
- An a2o scenario asserts: seed content → head is witnessed (green) → a second peer converges
  the same head, with NO per-peer divergent write.
