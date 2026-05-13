---
name: Memory classes — different shapes need different lifecycle defaults
description: Memory is not one shape; classes (contextual, archival, identity, relational, operational, attestation, wisdom) have different default lifecycle policies; lifecycle primitives (promote/compact/merge/submerge/close-interval/memorialize/forget) are the operations, memory classes are the defaults that determine which operations apply, at what cadence, with what governance authority; substrate must tag every entry with its class
type: project
originSessionId: 10d85ef0-1979-4311-97e9-c2c209de48e2
---
Memory is not one shape. Different classes of memory have fundamentally different lifecycles, and applying a single default policy to all of them wastes compute on transient data, under-preserves cultural artifacts, and mishandles attestation.

**Lifecycle primitives are the operations. Memory classes are the defaults.**

A film and a conversation use the same primitive vocabulary (`promote`, `compact`, `merge`, `close-interval`, `submerge`/`surface`, `memorialize`, `forget`, `quarantine`) but live with radically different defaults — which primitives apply by default, at what cadence, with which governance authority.

**Initial taxonomy (open to refinement):**

| Class | Examples | Default lifecycle character |
|---|---|---|
| **Contextual** | Conversations, decisions, in-the-moment interactions, household coordination | Comet-shaped, decay-active; `submerge`/`surface` heavy when content carries consequence; `forget` aggressive at tail. Lifecycle measured in days to months. |
| **Archival / canonical artifacts** | Films, books, recorded music, photographs, recipes, heirloom-as-data, scientific datasets | Stable-by-design; `merge` heavy across network (dedup equivalent copies via content addressing); rarely `forget`; rarely `submerge`. Lifecycle measured in generations. |
| **Identity** | Profile, attestations, displayed self, key material, recovery shares | Durable-while-you-live; evolves with you; `memorialize` for core, `close-interval` for superseded states. Lifecycle scoped to a person. |
| **Relational** | Bonds, shared histories, trust signals, accumulated insight-with | Fades with relationship cooling, warms with re-engagement; `close-interval` natural when relations end; never fully forgotten (the trajectory matters). |
| **Operational / transient** | System state, working buffers, in-flight task data, configs | `forget` aggressive once operation completes; rarely promoted; audit-trail minimal. Lifecycle scoped to operation. |
| **Attestation / truth** | Factual records, contracts, governance decisions, REA commitments, notarizations | `close-interval` heavy for supersession; **never `forget`** (historical record sacred); citation-anchored; immutable record of "what was attested when." |
| **Wisdom / principle** | Extracted learnings, manifesto-tier statements, principles, distilled experience | `memorialize`-by-default for the core; slow-evolving; high earning threshold for new entries; high earning threshold for revisions. |

**Each class is governed differently:**

- *Contextual* — operator authority for personal, household-steward authority for shared
- *Archival* — qahal-governed distribution; mishpat-governed when content carries harm-class status
- *Identity* — author + recovery-circle authority; structural inviolability for some sub-classes (recovery shares, key material)
- *Relational* — co-authored — both parties have stake
- *Operational* — substrate-managed; minimal governance overhead
- *Attestation* — protocol-substrate-managed; immutability is structural
- *Wisdom* — qahal-promoted with explicit reviewer pass

**Composition with primitives — example matrix:**

A `merge` of two contextual entries (Mira-conversation-A coalesces with Mira-conversation-B touching the same subject) requires operator consent and triggers all the consolidation-event signals.

A `merge` of two archival entries (two households' independent copies of the same film) is a network-efficiency operation gated on the content-addressing-equivalence attestation; no operator consent because nobody owns the artifact uniquely; freed compute redistributes to the network.

A `merge` is structurally not available for attestation entries — supersession via `close-interval` is the only path; the original attestation remains queryable forever.

A `submerge` makes sense for contextual memory (reach-dropped post) and for relational memory (cooled friendship out of active view); it is ill-defined for archival memory (an artwork doesn't submerge from culture because individual users stop engaging) and impossible for attestation memory (the truth doesn't go to subconscious).

**Why this is structurally important:**

Without memory classes, the substrate either:
- Treats everything with archival defaults (preserves forever) — compute footprint unsustainable for household-scale infrastructure
- Treats everything with contextual defaults (fades graciously) — loses cultural artifacts, deletes attestation records, kills the trajectory
- Treats everything with operational defaults (forgets after task) — catastrophically loses identity, relationships, wisdom

The classes ARE the protocol's commitments about what each kind of data deserves.

**Connections to existing project principles:**

- *DHT vs libp2p scoping* — attestation class anchors to DHT (truth needs notarization); archival/relational/contextual ride libp2p (operational state); the class declares which layer of truth model applies
- *epr-content-addressing* — archival class is the heaviest user of content-addressing (cultural artifacts dedupe across the network by content hash)
- *REA economics / shefa* — relational and attestation classes generate the bulk of economic events
- *Household horizontal scaling* — archival class distributes at the network layer (resilience replicas); contextual at the household layer (one household per user)
- *Stewardship philosophy / graduated authority* — each class needs its own authority tier mapping
- *Three-layer truth model* — attestation = DHT; archival/contextual = libp2p; doorway projects all classes to web-2.0 views
- *Reach earned at authoring* — authoring earnings differ by class (a film author earns differently than a conversation participant)
- *Comet shape lifecycle* — applies fully to contextual; partially to relational; barely to archival; not at all to attestation

**How to apply:**

- Every entity in the protocol substrate (memory entries, EPRs, DHT entries, content nodes, scenarios, economic events) MUST declare its memory class at creation. Untyped entries are a design failure.
- Lifecycle policy defaults derive from class. Specific entries can override defaults within policy bounds; defaults are not silent.
- Substrate storage tier is informed by class — archival lives in the quilt (RS-distributed); contextual lives in household pantries; attestation in the DHT.
- When designing new data types, first declare the class; that determines half the design (lifecycle defaults, governance authority, storage tier).
- Class is composable but rarely overlapping — most entries have a primary class. When ambiguous (e.g., a household photo: archival because cultural, contextual because personal), default to the more-protective class (archival in this case) and require explicit declaration for variance.
- The `/dream` skill consumes the memory-class declaration when proposing lifecycle operations; class-inappropriate proposals are filtered before reaching the operator.

**Sources:** brainstorm 2026-05-10, surfaced after the living_memory epic was drafted; the realization that the epic spoke mostly about contextual memory exposed the need to name the other classes explicitly.
