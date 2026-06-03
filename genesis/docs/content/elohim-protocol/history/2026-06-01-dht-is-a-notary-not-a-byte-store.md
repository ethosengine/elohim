---
title: "History/ADR: The DHT is a notary, not a byte store"
id: dht-is-a-notary-not-a-byte-store
type: history-gotcha
status: Accepted
created: 2026-06-01
topic: [storage, dht, blob, libp2p, quilt, substrate-scoping]
# This record DISTILLS three abandoned/pivoted implementation paths into one lesson.
# It is the compacted tail; the raw plan bodies live in git history at the pointers below.
distills:
  - genesis/docs/superpowers/plans/2026-04-22-elohim-epr-storage-foundation-plan-BATCH-C-PIVOT.md
  - genesis/docs/superpowers/specs/2026-05-02-blob-custody-reconciliation-design.md
  - genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md
# Bidirectional: these are the CANONICAL specs this gotcha points back to.
canonical:
  - genesis/docs/content/elohim-protocol/architecture/2026-04-21-elohim-core-graph-substrate-design.md   # split architecture (§ "large content")
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md       # where bytes actually live
memory_anchors:
  - project_three_layer_truth_model        # DHT=notary, libp2p=data-ops, doorway=web2 projection
  - project_dht_vs_libp2p_scoping
  - project_inventory_exchange_not_byte_replication
  - project_principle_p1_reconciliation_controller
---

# History/ADR: The DHT is a notary, not a byte store

> **Hot-context pointer (the one sentence to remember):**
> **We do not put blob bytes — or operational byte-movement state — on the Holochain DHT.**
> The DHT *notarizes* (signed manifests, ~500B envelopes, CIDs + attestations); **bytes live in the
> quilt/pantry, and byte-movement is libp2p-operational.** We drifted toward the wrong layer three
> times and corrected each. If you are about to reach for a DHT record, a local diesel store, a
> Kademlia provider-record for a blob hash, or a DB `PATCH` to "deploy" — stop and read the result below.

**Canonical (where bytes actually live):**
[Elohim-Core Graph Substrate §"large content"](../architecture/2026-04-21-elohim-core-graph-substrate-design.md)
— *"notarization + envelope in DHT/storage-atoms; bulk content in libp2p/erasure-coded blobs"*, envelope
carries `payload.bodyCid: CID` → the blob. And [Tiered Quilt Stewardship](../architecture/2026-05-11-tiered-quilt-stewardship-design.md)
defines the temperature-tiered blob home (quilt/pantry, iroh `/blob` hot plane). The architecture was
**always** split — the lessons below are about *implementations* drifting off it.

---

## The three paths not taken (each: what we tried · the result · the correction)

### 1. EPR routes as single-node REST over local diesel  *(2026-04-22)*
- **Tried:** Batch C of the EPR storage foundation designed `/api/v1/epr` routes as REST over a local
  diesel store, **ignoring** the `/elohim/epr/1.0.0` libp2p protocol, the Kademlia store, and the swarm
  behaviour *already active* in elohim-storage.
- **Result:** would have shipped a non-federated content silo behind a federated-looking API — a one-node
  dead end the moment a second peer existed.
- **Correction:** the **BATCH-C-PIVOT** — routes talk only to an `EprStore` trait (`LocalEprStore` now,
  `FederatedEprStore` later); P2P-federated from day one; provider queries answered from the DHT, never
  the blob bytes. → `plans/2026-04-22-elohim-epr-storage-foundation-plan-BATCH-C-PIVOT.md` (git history)

### 2. Blob custody as discovery-first (Kad-lookup → fetch)  *(2026-05-02)*
- **Tried:** "Light Up the Topology" Phase 2 planned blob replication as a discovery mechanism —
  Kademlia provider lookup for blob hashes, then fetch — assuming a `SwarmClient` abstraction and
  sha256-of-bytes verification.
- **Result:** the implementer **correctly BLOCKED before writing code** and surfaced six substrate-vs-plan
  mismatches: no `SwarmClient` (substrate is `mpsc::Sender<P2PCommand>`), **no Kademlia provider track for
  blob hashes** (only EPR *atom* CIDs are advertised), EPR verification is CBOR-canonical CID recompute
  (not sha256), and the plan targeted the wrong structs.
- **Correction:** **manifest-vs-reality-first**, not discovery-first — the three-surface reconciliation
  controller (DHT *manifest* of custody commitments / libp2p *reality* of who-has-what via gossip / the
  *diff*). And the load-bearing scoping rule: **do not sign gossip inventory at DHT cost** — operational
  chatter collapses on the first failed fetch, so verifiable serves are the integrity floor, not signatures.
  → `specs/2026-05-02-blob-custody-reconciliation-design.md`

### 3. "Deploy" as a DB `PATCH` that never told the DHT  *(2026-05-25)*
- **Tried:** the Z.1 deploy path `PATCH /db/content/{slug}` — upload the blob, PATCH the content rows green.
- **Result:** app pipeline #1464 went **mechanically green** (blob up, rows patched, verifies passed) yet
  `alpha.elohim.host` **served stale content**, because the DHT was never told the blob changed. A green
  build that lies.
- **Correction:** deploy is an **`EprHead` republish** carried by a reciprocal REA compute commitment, not
  a DB mutation — the substrate-correct path (Z.D). → `specs/2026-05-25-stagespablob-substrate-correct-deploy.md`
  *(this spec is gospel-tier — it names the REA compute-commitment primitive — and is **kept canonical**, not retired.)*

---

## Why this keeps happening (so you can feel the pull and resist it)

Every one of these is the same reflex under three disguises: an AI agent (or a hurried human) reaches for
the **familiar relational/centralized shape** — a local DB, a single REST surface, a provider-record index,
an in-place row PATCH — because it is the default mental model. The protocol's truth is layered:

| Layer | Job | What belongs here | What does **not** |
|---|---|---|---|
| **Holochain DHT** | notarize | signed manifests, CIDs, ~500B envelopes, custody *commitments* | blob bytes; operational who-has-what; deploy mutations |
| **libp2p** | operate | gossip inventory, fetch, replication, the reconciliation diff | anything that needs forge-proof signing |
| **quilt / pantry** | hold bytes | the actual blobs, temperature-tiered, erasure-coded | identity / authority claims |
| **doorway** | project to web2 | caches, single-target HTTP | source of truth |

The DHT is the *most expensive* layer. Putting bytes or byte-movement there is paying notary cost for
operational data — the exact anti-pattern `project_dht_vs_libp2p_scoping` was written to stop.

---

## Bidirectional links

- **This gotcha → canonical:** [graph-substrate split architecture](../architecture/2026-04-21-elohim-core-graph-substrate-design.md), [tiered quilt (blob home)](../architecture/2026-05-11-tiered-quilt-stewardship-design.md)
- **Canonical → this gotcha:** the graph-substrate spec's "large content" section now carries a back-pointer to this record (added 2026-06-01).
- **Distilled-from (raw bodies in git history):** BATCH-C-PIVOT, blob-custody-reconciliation, stagespablob/Z.D (linked above).
