---
title: "History: The storage dual-plane design arc (April 2026)"
id: storage-dual-plane-design-arc
type: history-gotcha
status: noted
tier: history
created: 2026-06-11
topic: [storage, p2p, dual-plane, reach, cache-core, doorway, design-arc]
# Provenance breadcrumb: the three retiring island docs this record distills.
derived_from:
  - elohim/elohim-storage/P2P-ARCHITECTURE.md   # retired to git 2026-06-11 (storage island recompose; authored 2026-04-15)
  - elohim/elohim-storage/EDGE-ARCHITECTURE.md  # retired to git 2026-06-11 (storage island recompose; authored 2026-04-30)
  - elohim/elohim-storage/REACH.md              # retired to git 2026-06-11 (storage island recompose; authored 2026-04-15)
canonical:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md
  - genesis/docs/content/elohim-protocol/history/2026-06-01-dht-is-a-notary-not-a-byte-store.md
cites:
  - doorway/CLAUDE.md
  - doorway/doorway-service/src/services/route_registry.rs
  - doorway/doorway-service/src/services/discovery.rs
  - doorway/doorway-service/src/server/http.rs
  - elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
  - elohim-cache-core-gospel | the extracted crate gospel — where EDGE-ARCH's holochain-cache-core component description now lives | sha256:359677d53fb0dcd7 | path: elohim/elohim-cache-core/CLAUDE.md
  - genesis/plans/2026-03-29-elohim-cache-core-extraction-cache-design.md
  - elohim/elohim-storage/src/p2p/replication.rs
  - elohim/elohim-storage/src/p2p/fanout.rs
  - elohim/elohim-storage/src/p2p/kad_store.rs
  - elohim/elohim-storage/src/p2p/reach_authorization.rs
  - elohim/elohim-storage/src/config.rs
  - epr-acquisition-pull-queue-design | the canonical successor design for the async-replication write split this record traces to its April origin | sha256:24aad9240361c0a4 | path: genesis/docs/superpowers/specs/2026-06-07-epr-acquisition-pull-queue-design.md
  - genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md
  - genesis/data/timeline/backlog/storage-island-harvest-residue.md
  - elohim-pillar-architecture-founding-arc | the sibling recompose record (same day) that canonized the status-tables-rot-silently lesson this record defers to | sha256:7bf421e90410e3ad | path: genesis/docs/content/elohim-protocol/history/2026-06-11-elohim-pillar-architecture-founding-arc.md
memory_anchors:
  - project_inventory_exchange_not_byte_replication
  - project_reach_enum_drift_reconciliation
---

# History: The storage dual-plane design arc (April 2026)

> **Hot-context pointer (the one sentence to remember):**
> A design doc can win the argument and lose every mechanism: April's dual-plane bet
> ("P2P is for durability and distribution, not performance; fast paths stay local")
> became the three-truth-layer canon, while **none of its four concrete proposals
> shipped as drawn** — cite the surviving principle, let git keep the mechanisms.

## The bet that paid

P2P-ARCHITECTURE.md (2026-04-15) drew the dual-plane split — Holochain as the slow,
trusted control plane (identity, provenance, manifests); elohim-storage as the fast,
simple data plane (bytes, transfer, shards) — and closed on the principle quoted in
the pointer above. That bet predates and correctly anticipated the canonical model:
the tiered-quilt design's "Three truth layers" (DHT notarizes `Commitment` entries
for `action="custody-quilt"` + `tier-*` attestations; libp2p operates; bytes live in
the quilt/pantry) and the `dht-is-a-notary-not-a-byte-store` history record, which
distills three later implementation drifts back onto exactly this split. The April
doc is the design-arc origin of that canon; the canon is where it now lives. Do not
read the retired doc for the model — read the two canonical entries above.

## What landed — in evolved shape, never as drawn

- **Dynamic route registration.** EDGE-ARCH proposed a `__doorway_routes` zome
  function so DNAs declare doorway routes. The *idea* (no hard-coded routes) won:
  `route_registry.rs:8` manages dynamic routes, and the live mechanism is
  manifest-declared routes via steward self-registration (`doorway/CLAUDE.md:30`).
  The proposed *mechanism* did not: `discovery.rs:276` `discover_routes` is an
  explicit stub — "Not yet implemented — returns None. Routes come from steward
  self-registration via build_manifest(), not DNA introspection." Only the precursor
  `__doorway_import_config` runs as designed (`content_store/src/lib.rs:1743`).
- **WriteBuffer out of doorway.** EDGE-ARCH "Migration Phase 2" (remove
  `DoorwayWriteBuffer`; agents use cache-core directly) landed verbatim — doorway
  `http.rs:562,667`: "Write batching is handled by agent's elohim-cache-core
  WriteBuffer, NOT here."
- **Cache-core extraction.** EDGE-ARCH's `holochain-cache-core` is now
  `elohim/elohim-cache-core/` (WriteBuffer, BlobCache, ContentResolver) with its own
  gospel (`elohim/elohim-cache-core/CLAUDE.md`) and extraction design
  (`genesis/plans/2026-03-29-elohim-cache-core-extraction-cache-design.md`). Its
  `ReachAwareCache` ended up TypeScript-side, not in the Rust crate.
- **Async replication.** The sync-fast/async-replicate write split is live
  (`src/p2p/replication.rs`, `fanout.rs`) with its canonical successor design in
  the EPR acquisition pull-queue spec (2026-06-07).

## The paths not taken

- **`ContentLocation` DHT entry** (holders list on the Holochain DHT): never built —
  zero hits in `elohim/holochain/dna/`. Live model: Kademlia provider records
  (`src/p2p/kad_store.rs`) + metadata-only inventory gossip
  (`project_inventory_exchange_not_byte_replication`). Putting a who-has-what list
  on the DHT is precisely the notary-cost-for-operational-data anti-pattern the
  canonical history record warns about.
- **Doorway signal-server libp2p bootstrap** (extend `/signal/{pubkey}` for peer
  exchange): never built. Live: direct multiaddr config,
  `src/config.rs:99-102` `p2p_bootstrap_nodes: Vec<String>`.
- **Acknowledgment tiers** (`accepted`/`durable`/`verified`): never implemented as
  vocabulary; superseded by the REA commitment/attestation model — custody-quilt
  `Commitment` + `tier-holdings`/`tier-breach`/`tier-restitution` attestations
  (tiered-quilt :154-159, REA event catalog §4).
- **Delivery-side reach trust-filtering** (REACH.md's `can_serve_blob` gating every
  request by requester trust): philosophically **inverted**, not just unbuilt.
  `src/p2p/reach_authorization.rs` enforces author-side earning + receiver-side
  pre-authorization, with the rationale in its module doc: "Email collapsed because
  anyone could publish to anyone, putting the cost of filtering on receivers …
  The Elohim Protocol places the burden where it belongs: on the author + the peers
  that steward what they author." A retired design can be wrong in *direction*, not
  just detail — the cost moved from the receiving edge to the authoring edge.

## The vocabulary ghost

REACH.md is the design-doc *origin* of the geographic 8-value reach ladder
(private/invited/local/neighborhood/municipal/bioregional/regional/commons) that
matches no live backend vocabulary yet still haunts 4 TypeScript sites — the "4th
strand" recorded in `genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md`.
Retiring the origin doc does **not** resolve the code drift; the backlog entry and
the resilience-epic reconciliation item remain open and unresolved here.

## The rot lesson (one line)

The P2P-ARCHITECTURE status table claimed "libp2p foundation: Dormant … Replication
worker: Missing" against what is now 35 live modules under `src/p2p/` (plus dual
transport `src/p2p_iroh/`) — the same status-tables-rot-silently lesson the sibling
record `elohim-pillar-architecture-founding-arc` canonized today; not re-derived here.

REACH.md's encryption-at-rest-by-reach vision was never built and has no canonical
home (coherence-index negative) — it travels, adopted as an *open vision composing
with the quilt model* (not abandoned), alongside the unconsumed sovereignty/cluster
scaffolding in the residue backlog entry
`genesis/data/timeline/backlog/storage-island-harvest-residue.md`.
