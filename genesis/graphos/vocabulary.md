# Elohim Protocol — Design Vocabulary

**Version:** 0.1
**Status:** Active reference
**Last updated:** 2026-04-30

---

## Why this register exists

The protocol is not "blobs in buckets." It is peers stewarding shards in pantries, stitching redundancy through reed-solomon. The wire-level vocabulary inherited from S3-shaped systems (`blob`, `store`, `upload`, `download`) hides that truth. This register names what is actually happening so that design discussions, signal/event names, narrative, and any new concept we invent can speak the protocol's actual shape.

This register governs **design-level language and new concepts**. Wire-level terms that are externally legible — HTTP routes (`/blob/{hash}`), file paths, content addresses (`sha256-{hex}`, CID), and existing internal Rust types (`BlobStore`, `blob_path`, `blob_hash`) — keep their existing names because their interface is read by clients, browsers, and external tools that already understand the S3-style framing.

---

## Boundary rule

| Layer | Language | Examples |
|---|---|---|
| **Wire / interface** (HTTP routes, content addresses, internal Rust types named for HTTP wire shape) | Existing terms — `blob`, `/blob/{hash}`, `sha256-{hex}`, `BlobStore` | Do not rename; externally legible |
| **Design discussion, narrative, signals/events, configmap keys, admin endpoints, tracing spans, new Rust/TS identifiers we invent** | New vocabulary — `quilt`, `pantry`, `stock`, `draw`, `shard`, `RS(N,K)` | Use these consistently |

If you are about to invent a new identifier (signal name, event name, type name, span name, admin endpoint), reach for the new vocabulary. If you are touching an existing wire-level identifier, leave it alone.

---

## Terms

### `quilt`

**Meaning.** The reed-solomon-encoded distribution of a content unit across N shards, of which any K reconstruct the original. A quilt is what we have when content has been encoded for resilient distribution; the bytes it carries are the same bytes you'd get back, but its existence in the network is structurally redundant.

**Verb pairings.**
- "quilt content into N shards" — the encoding operation
- "the quilt for content X" — the name for the redundant fabric
- "re-quilt" — restitch after losses (recover N from K survivors)
- "RS(N,K) quilt" — naming the contract policy explicitly

**Replaces.** Conceptually replaces `blob`-as-monolithic-unit. Does **not** replace the HTTP route `/blob/{hash}` or the Rust type `BlobStore` — those are wire-level.

**Distinct from.** Holochain Moss `weave` / `@theweave/api` / "Weave Tool" / `weave.service.ts`. The Moss "Weave" namespace is a foreign term in the elohim repo; do not conflate. (Decision: 2026-04-30. Earlier candidates `weave`, `lattice`, `weft`, `scatter/gather`, `rs`, `rsweave` were rejected — see `genesis/docs/superpowers/plans/2026-04-30-vocabulary-cleanup-sprint-kickoff.md` Task 1 for full rationale.)

---

### `pantry`

**Meaning.** A peer-tended container that holds shards on behalf of the household it serves. Households tend overlapping pantries — the same shard may live in multiple pantries across multiple peers, and that overlap is the resilience guarantee. A pantry is a *role* a peer plays in the protocol, not a single physical store.

**Verb pairings.**
- "stock the pantry" — add content
- "draw from the pantry" — retrieve content
- "tend a pantry" — keep its contents fresh, restock losses, evict by archetype-tunable policy

**Replaces.** Conceptually replaces "bucket", "store-as-destination", "S3 bucket" framing. Does **not** replace the Rust type `BlobStore` — that names the on-disk structure, which is one component of how a peer tends a pantry.

**Already in narrative.** Existing a2o scenarios use `pantry` in domestic contexts (`genesis/docs/content/elohim-protocol/value_scanner/parent/scenarios/household.feature`, `value_scanner/student/scenarios/household.feature`, `value_scanner/vulnerable_temporary/scenarios/neighborhood.feature`). The protocol-level meaning is consonant with the domestic one — a pantry is what a household tends.

---

### `stock` (verb)

**Meaning.** Deposit content into a pantry. Used for both the act of placing newly-authored content into the local pantry and the act of replenishing a pantry's holdings (e.g., re-quilting after losses).

**Verb pairings.**
- "stock the local pantry with this quilt"
- "stock from a peer" — pull a missing shard from another pantry that has it

**Replaces.** New language for `upload` *where new* — i.e., when naming new signals, events, or admin endpoints. Existing wire-level `upload` (HTTP semantics, multipart bodies) keeps its name.

---

### `draw` (verb)

**Meaning.** Retrieve content from a pantry. The pantry may serve from local cache, from another peer's pantry it has reach into, or from a quilt it must reconstruct on the fly.

**Verb pairings.**
- "draw the thumbnail from the pantry"
- "draw via reach" — retrieve through the trust graph rather than the open mesh

**Replaces.** New language for `download` *where new*. Existing wire-level `download`/HTTP GET semantics keep their names.

---

### `shard`

**Meaning.** One piece of an RS-encoded quilt. Held by a peer, addressed by `sha256-{hex}` of its bytes. A quilt is N shards; any K of those shards reconstruct the original content.

**Verb pairings.**
- "tend a shard" — keep it healthy, available, verified
- "the shard for content X at index i"

**Replaces.** Already in use; no replacement. This entry exists to fix the relationship: **a quilt is N shards**.

**Wire.** Shards are addressed at the wire layer by `sha256-{hex}`. The HTTP route `/blob/{hash}` serves either a whole content unit or a single shard, depending on what the hash addresses.

---

### `RS(N,K)`

**Meaning.** The contract policy for a quilt. `N` total shards encoded; any `K` reconstruct. `N - K` is the redundancy budget — that many shards may be lost before the quilt is unrecoverable.

**Verb pairings.**
- "RS(8,4) quilt" — 8 shards, any 4 reconstruct
- "tighten the RS contract" — increase K relative to N
- "loosen the RS contract" — decrease K relative to N

**Replaces.** Replaces the "S3-style replication factor" framing. Replication factor N=3 says "three copies"; `RS(N,K)` says "N pieces, any K reconstruct" — a richer description that names the actual resilience property.

**Archetype-tunable.** RS(N,K) is set by archetype default → policy.toml → env/CLI → sync admin trigger, per the 4-layer cadence model. Different deployment archetypes (household cluster, mobile, hosted-doorway, shem) get different defaults.

---

## Reserved words — do not reuse

Three words have established meanings in the elohim repo and must not be reused for new concepts:

| Word | Belongs to | Don't reuse for |
|---|---|---|
| `weave` | Holochain Moss (`@theweave/api`, `weave.service.ts`, the Moss Weave Tool) | Storage / RS distribution — use `quilt` instead |
| `lattice` | Cross-collective governance ("the holonic lattice" in `genesis/plans/2026-04-10-collectives-schema-design.md`) | Storage, mesh topology, anything else |
| `quilt` | Storage RS-distribution (this register) | Governance, Moss applets, anything else |

If you find yourself reaching for one of these in a new context, pick a different word.

---

## Replacement table for legacy storage paths

The HTTP routes `/store/{hash}` (legacy on doorway) and `/api/blob/{hash}` (legacy alias) are deprecated and being removed. The canonical path is `/blob/{hash}`. The `POST /api/blob/verify` endpoint is **not** part of this deprecation — it remains as a separate verification endpoint.

| Legacy | Canonical | Notes |
|---|---|---|
| `GET /store/{hash}` | `GET /blob/{hash}` | Doorway dispatch arm being deleted; routed via registry to elohim-storage |
| `HEAD /store/{hash}` | `HEAD /blob/{hash}` | Same |
| `GET /api/blob/{hash}` | `GET /blob/{hash}` | Same |
| `HEAD /api/blob/{hash}` | `HEAD /blob/{hash}` | Same |
| `POST /api/blob/verify` | (unchanged) | Verification endpoint, not a content path; stays |

---

## Where this register applies

- **Specs and design docs:** `genesis/docs/superpowers/specs/`, `genesis/docs/plans/`, this file
- **Narrative and a2o scenarios:** `genesis/a2o/features/`, `genesis/docs/content/elohim-protocol/`
- **Signal and event names:** any new wire-protocol message, libp2p signal, DHT signal stream
- **Tracing spans, log fields, configmap keys, admin endpoints:** any new identifier we invent
- **CLAUDE.md files:** when explaining storage/distribution concepts, link here

## Where it does not apply

- Existing HTTP routes (`/blob/{hash}`)
- Content addresses (`sha256-{hex}`, CID)
- Existing internal Rust types named for the HTTP wire shape (`BlobStore`, `blob_path`, `blob_hash`)
- External-facing API documentation that describes wire format to clients
- Historical specs and sprint plans (do not rewrite history)

---

## Cross-references

- Resolution and rationale: `genesis/docs/superpowers/plans/2026-04-30-vocabulary-cleanup-sprint-kickoff.md`
- Boundary rule precedent: substrate-spec composition framing in `genesis/docs/superpowers/specs/2026-04-21-elohim-core-graph-substrate-design.md:475` (Moss-Weave is foreign)
- Related: `genesis/plans/2026-04-10-collectives-schema-design.md` (the holonic lattice)
- Related: `genesis/docs/plans/2026-04-21-rno-lessons-roadmap-handoff.md` sub-project #8 (lamad-as-Moss-Weave-Tool)
