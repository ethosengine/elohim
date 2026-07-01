---
id: "backlog-crdt-authoritative-content-state-dht-notary-decouple"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "CRDT-authoritative content state, DHT-notary decoupled: make the Automerge plane the robust convergent-state substrate and the DHT a provenance/authority overlay — so serving heals peer-to-peer even when the notary is down (the elohim.host blobHash=null 404 class)"
slug: "crdt-authoritative-content-state-dht-notary-decouple"
written: "2026-07-01"
author: "overnight shift — p2p-sync feature-completeness (automerge-content-sync-projection-completeness) + architect vision steer"
status: "backlog"
priority: "high"
ci_status: blocked
jobs: [elohim]
tags: [automerge, crdt, content-sync, dataplane, dht-notary, provenance, authority, source-of-truth, non-brittleness, elohim-host, blobHash, heal, p2p-design-gate, versioned-entity-head, principle-p1]
cites:
  - elohim/elohim-storage/src/p2p/mod.rs
  - elohim/elohim-storage/src/sync/projector.rs
  - elohim/elohim-storage/src/db/content_diesel.rs
  - genesis/docs/superpowers/plans/2026-06-27-automerge-content-sync-plane-lighting-plan.md
  - genesis/data/timeline/backlog/conductor-websocket-flap-breaks-deploy-write-path.md
---

# CRDT-authoritative content state; DHT as a decoupled notary (architect vision)

## The vision (architect, 2026-07-01)

> The full CRDT-authority (Automerge) is the right vision. But the Holochain DHT
> is like a k8s manifest controller that notarizes the provenance/authority/trust
> over that state. We should be robust enough that no layer is in-of-itself
> brittle, and any tight-coupling occurs AFTER curing battle-ready architecture.

This is the protocol's own canon ("the DHT is a **notary, not a database**",
p2p-design-gate) followed to its conclusion, and it is `project_principle_p1`
("DHT = manifest, libp2p = k8s controller; eager reconcile") applied to content
state.

## The brittleness being cured

Today the DHT-notary does TWO jobs, and the coupling is the bug:
1. **Authority/provenance/trust** — witnessing who authored/authorized a state
   (its proper notary job).
2. **State transport for updates** — the ONLY writer of an existing row's field
   (`blobHash`) is the notarized `PATCH /db/content` → conductor → DHT.

So **state convergence is gated on the notary being up**. When the ethosengine
conductors' DHT arcs went incoherent (see
[[conductor-websocket-flap-breaks-deploy-write-path]]), job #2 took SERVING down:
`elohim.host/`'s `elohim-host-landing` row is stuck at `blobHash: null` → 404
(`App not found`, http.rs:5601), because the only path that updates an existing
row's field is DHT-coupled and the notary is unavailable. The notary being down
should degrade *trust confirmation*, never *state convergence*.

## Target architecture — three independently-robust layers

| Layer | Job | Robust alone |
|---|---|---|
| Automerge CRDT plane | authoritative **convergent state** (content version DAG merges peer-to-peer, DHT-independent) | converges with zero notary reachable |
| Holochain DHT | **notary / controller** — witnesses provenance/authority/trust OVER the converged state; reconciles the manifest | down = trust unconfirmed, not state-lost |
| SQLite projection | read-optimized serving cache of converged state; stamped when the notary confirms | serves converged state immediately |

**HEAD stays declared/notarized, NOT recency** (`versioned_entity_HEAD_is_declared_dependency`):
the CRDT converges the version DAG; the DHT notarizes which version is authoritative
HEAD — asynchronously, over already-converged state. This is what makes 1c principled
rather than last-writer-wins.

## Accurate current-state map (corrected — earlier "no content→SQL heal" was wrong)

- **Shard/replication plane DOES heal content→SQL on INSERT.** `ShardResponse::Content`
  (`p2p/mod.rs:4120`) builds a `CreateContentInput` incl. `blob_hash` and calls
  `bulk_create_content` (`:4178`), then pulls blob bytes (`:4222+`). This stays the
  bulk-record + blob-byte replication path.
- **But `bulk_create_content` is insert-or-SKIP** (`content_diesel.rs:443–446`): an
  EXISTING row is skipped, never field-updated. So a row that exists with a stale/null
  field never re-converges from a peer — the exact elohim.host trap.
- **The Automerge consumer writes the sled DocStore only** (`apply_changes`,
  `p2p/mod.rs:6549`/`6402`); `sync/mod.rs` header calls its docs "content metadata —
  node stubs for fog-of-war." The vision PROMOTES this plane from fog-of-war metadata
  to authoritative convergent state.
- **Producer foundation LANDED 2026-07-01** (`08b284fc8`): full-field projection incl.
  `blobHash` + idempotency + gated corpus back-fill. This is the right first stone —
  the CRDT plane now CARRIES the full content state an authoritative convergent layer
  needs. (Sibling: [[automerge-docstore-corpus-backfill-migration]].)

## Design decisions (architect / `/brainstorm` — p2p-design-gate already run)

1. **Source-of-truth bifurcation** — name it precisely: DHT owns authority/provenance/
   HEAD-notarization; the CRDT-converged doc owns operational convergent state; SQLite
   projects. (Gate output: Content = Category A, address CIDv1 `bafyrei…`, version DAG.)
2. **CRDT→SQL projection (the heal leg)** — on Automerge convergence for
   `PROJECTION_NAMESPACE`, project the converged doc into the SQL `content` row via a
   field-merge UPSERT (reuse `projected_fields` as the read-back contract; mirror the
   `PROJECTION_NAMESPACE` compile-coupling). This is what actually fixes a stale/null
   `blobHash` from a healthy peer WITHOUT the notary.
3. **Non-brittleness law** — convergence must NEVER hard-depend on notarization; the
   notary stamp (dht_anchor_hash / reach proof) is applied as an overlay when available.
   Tight-coupling (e.g. "don't serve un-notarized state") is a LATER, battle-ready-only step.
4. **Provenance bar** — the shard insert path is already permissive (`dht_anchor_hash: None`
   + `p2p_published_at` stamp, `:4171–4174`); decide the eventual notary-confirmation bar
   for CRDT-converged state without making it a convergence precondition.

Domain D5 (data plane) / P1 (reconciliation controller). Depends on: producer
foundation (LANDED). Effort: L–XL (architecture). Route: `/brainstorm` → spec, then a
sequenced build (converge-heal first; notary-overlay coupling last).
