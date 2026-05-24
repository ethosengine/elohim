---
title: Records Lifecycle — Wiring the EPR / Event / Resource Substrate
tier: architecture
status: Draft (in-flight; Part A.1 + Part B.1 reviewed, remainder stubbed)
created: 2026-05-24
authors: Matthew Dowell + Opus 4.7
pillar coupling: elohim (substrate primitive), shefa (the canonical application), lamad (Resource state for content), imagodei (custody anchors), mishpat (governance of demotion/dissolution/forget)
realizes:
  - genesis/docs/content/elohim-protocol/economic_coordination/epic.md (Beer's Cybersyn on P2P; the Okonkwo family's economic visibility)
  - genesis/docs/content/elohim-protocol/value_scanner/epic.md (household care made legible via inventory-elohim narration)
  - genesis/docs/content/elohim-protocol/social_medium/epic.md (YouTube/Drive/WordPress shape applications on P2P substrate)
  - genesis/docs/content/elohim-protocol/living_memory/epic.md (the lifecycle gradient: birth → flow → cistern → dissolution)
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-observation-event-layer-design.md (Observation tier; STRICT prerequisite — this spec extends it)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-attestation-consolidation-design.md (sibling cut on the notary side)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md (cold-archive substrate this lifecycle terminates into)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-20-wave3-valueflows-hrea-interop-design.md (the REA / ValueFlows substrate this lifecycle wires)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-23-multi-collective-collaboration-epr-design.md (EPR custody handoff between collectives — the cross-collective half of re-elevation)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-10-memory-lifecycle-design.md (parallel `submerge`/`surface` vocabulary; this spec reconciles)
  - genesis/docs/content/elohim-protocol/architecture/2026-04-18-experience-story-epr-design.md (foundational EPR design)
  - genesis/docs/content/elohim-protocol/architecture/2026-04-21-elohim-epr-integrator-compatibility-contract.md (EPR contract layers; Gap 6 prerequisite)
  - genesis/docs/content/elohim-protocol/architecture/2026-04-23-epr-phase-2c-libp2p-federation-design.md (EPR transport over libp2p)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-15-dna-signal-as-epr-envelope.md (signal-as-EPR-envelope pattern)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md (web2 projection tiers; bridge layer for legacy systems)
informs:
  - All future sprint-shape specs that touch EPR / Event / Resource / Observation / Commitment / Attestation / FeedbackSignal primitives
  - Any new pillar manifest declaration (new action verbs, observation_kinds, signal_kinds, resource classifications, content_types)
  - Any new bridges/<vendor>/ crate (the Gap 9 pattern is normative)
  - Any new lifecycle operation that traverses Active ↔ Subordinate ↔ Shelved ↔ Closed (must reconcile with this spec's vocabulary)
memory_anchors:
  - project_three_layer_truth_model
  - project_dht_vs_libp2p_scoping
  - project_first_class_graph_pattern
  - project_signal_kind_extensible_protocol_class
  - project_epr_substrate_vs_vf_graphql
  - project_consolidation_events_economic_feedback
  - project_memory_lifecycle_comet_shape
  - project_memory_classes
  - project_submerge_destinations_stewardship_routing
  - project_social_reach_nervous_system
  - project_inventory_exchange_not_byte_replication
  - project_doorway_single_target_no_fanout
  - project_substrate_scale_ceiling
  - project_household_horizontal_scaling
  - project_collapse_bureaucracy_into_protocol
  - project_no_sovereignty_stewardship_over_ownership
  - project_friction_gradient_limitarianism
  - project_storage_actor_vs_forwarder_patterns
defers:
  - Cradle-to-cradle dissolution philosophy (this spec closes the implementation loop; the design session for "every birth knows what the end looks like" is separate)
  - Bridge implementation for any specific legacy vendor (Plaid, Stripe, banking APIs, KYC providers) — pattern is defined here; per-vendor bridge crates are separate sprints
  - EconomicEvent → Event / EconomicResource → Resource DHT-entry rename (treated semantically here; rename is a separable substrate cleanup)
  - Cross-collective EPR custody handoff mechanics (covered by 2026-05-23-multi-collective-collaboration-epr-design.md; this spec references but does not re-spec)
  - Organization-dissolution lifecycle (household-ending, collective-ending) — separate concern from Resource/Event dissolution
---

## Status

**Draft, in-flight.** Part A.1 (EPR primitive walkthrough) and Part B.1 (Monarch/Mint personal-finance worked example) are reviewed and approved as the section templates. Remaining primitives (A.2–A.8), remaining applications (B.2–B.6), the composability stress-test (Part C), the substrate wiring (Part D), and the migration plan (Part E) are stubbed with section headers + brief notes. Each subsequent drafting pass extends from this scaffold.

This spec is **the navigable junction between manifesto and code**. The anchor sections below let any reader walk the graph from vision (epics) → architecture (this spec) → implementation (code) and back, in either direction.

## Manifesto anchors (epic → this spec)

The narrative work this spec realizes at protocol layer:

| Epic | Lives at | This spec realizes it through |
|---|---|---|
| **Economic Coordination** — Beer's Cybersyn on P2P; the Okonkwo family wakes up to economic visibility | `genesis/docs/content/elohim-protocol/economic_coordination/epic.md` | Part B.1 (Monarch personal finance), Part B.5 (Factory-as-collective), Part B.6 (Bank-as-collective + bridges) |
| **Value Scanner** — household care made legible across 21 life-stage archetypes | `genesis/docs/content/elohim-protocol/value_scanner/epic.md` | Part A (§7 of every primitive: where agentic intelligence carries the load); Part B.1 "Where agentic intelligence carries the load" |
| **Social Medium** — the public-facing platform surface (creators, communities, content) | `genesis/docs/content/elohim-protocol/social_medium/epic.md` | Part B.2 (YouTube), Part B.3 (Drive/Photos), Part B.4 (WordPress) |
| **Living Memory** — the lifecycle of memory: birth, flow, cistern, dissolution | `genesis/docs/content/elohim-protocol/living_memory/epic.md` | Part D (the 10 substrate gaps — birth/flow/cistern/dissolution mechanics) |
| **Observer Protocol** — witness without surveillance | `genesis/docs/content/elohim-protocol/observer-protocol.md` | Cited prerequisite — 2026-05-11 observation spec (Part A.4 is a citation) |
| **Public Observer** — Sarah's Tuesday at the school board; community visibility | `genesis/docs/content/elohim-protocol/public_observer/epic.md` | Part C composability stress-test (multi-household, multi-collective participation) |
| **Manifesto** — the protocol's foundational commitments | `genesis/docs/content/elohim-protocol/manifesto.md` | §1 Motivation + §1.4 Subsumption posture |
| **Constitution** — the protocol's invariants | `genesis/docs/content/elohim-protocol/constitution.md` | §1.3 Authorship floor + §9 of each primitive (limit-awareness) |

## Code anchors (this spec → implementation)

Where the bits live and what this spec touches:

| Surface | Path | What's here |
|---|---|---|
| **Elohim DNA — integrity zome** | `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` | EntryTypes enum (~75 variants); EconomicEvent, EconomicResource, StewardedResource, Content, FeedbackSignal structs; LinkTypes enum; validation rules |
| **Elohim DNA — coordinator zome** | `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` | Write entry-points: create_content, create_economic_event, signal handlers |
| **Storage views (Rust→TS boundary)** | `elohim/elohim-storage/src/views.rs` | JSON wire shapes with `#[derive(TS)]` — source for storage-client-ts codegen |
| **View JSON schemas** | `elohim/sdk/schemas/v1/views/*.schema.json` | Schema-first governance for the Rust↔TS boundary (per CONVENTIONS.md) |
| **Pillar manifests** | `elohim/sdk/domains/{elohim,shefa,lamad,imagodei,mishpat,qahal,avodah,infrastructure}/manifest.json` | Extensible: action verbs, signal_kinds, observation_kinds, resource classifications, graduation policies |
| **Bridges (legacy interop)** | `bridges/valueflows/` (reference); future `bridges/<vendor>/` | Bidirectional adapters: external system ↔ substrate-native Observations under stewardship-elohim signature |
| **Doorway HTTP API** | `doorway/doorway-service/src/handlers/` | Web2 projection routes; bridge HTTP surfaces; SSR cache |
| **Storage client (TS SDK)** | `elohim/sdk/storage-client-ts/src/generated/` | Auto-generated camelCase TS types; published as `@elohim/storage-client` |
| **Elohim app (Angular consumer)** | `app/elohim-app/src/app/{shefa,lamad,imagodei,qahal,elohim}/` | Pillar services that consume the storage client; where Monarch-shape dashboards live |
| **Domain elohim-agents** | `app/elohim-app/src/app/elohim/elohim-agents/` (planned) | Where inventory-elohim, vehicle-elohim, care-stewardship-elohim service implementations live |
| **Graduation evaluator** | `elohim/elohim-storage/src/services/` (planned per 2026-05-11 spec Stage 5) | Per-pillar tokio task that crystallizes observations into Events/Attestations |

Each subsection of Part D names the **specific** files and structs the gap touches. The intent: a developer reading the spec can immediately navigate to the code, and a developer reading the code (with this spec linked from a file header) can navigate back to the vision.

## How to read this spec

If you understand SQL, Postgres replication, S3, Kafka, GraphQL, Spring Batch scheduled jobs, Redis cache invalidation, double-ledger accounting, or service-oriented architecture, you will recognize every primitive in Part A by analogy. The spec deliberately reaches for those analogies because the architecture must be intelligible to someone fluent in hyperscale patterns; the substrate's novelty is in the *composition*, not in any single primitive.

If you want the technical translation of a specific epic, jump to:

| If you read this epic | Read this section |
|---|---|
| `economic_coordination/epic.md` (Okonkwo family) | Part B.1 (Monarch personal finance), Part B.6 (Bank-as-collective + bridges) |
| `value_scanner/epic.md` (care made legible) | Part A.7 (agentic-IQ section of every primitive); Part B.1 §"Where agentic intelligence carries the load" |
| `social_medium/epic.md` (the public-facing surface) | Part B.2 (YouTube), Part B.3 (Drive/Photos), Part B.4 (WordPress) |
| `living_memory/epic.md` (lifecycle / memory) | Part D (the 10 substrate gaps — birth/flow/cistern/dissolution mechanics) |
| `observer-protocol.md` (witness without surveillance) | Cited prerequisite — 2026-05-11 observation spec |

---

## 1. Motivation

The Elohim Protocol must scale to **8 billion humans** without overwhelming DHT gossip, peer storage, or network bandwidth, while keeping the user experience at the grandma-managing-Google-Photos-on-Android bar. The substrate has the right primitives — but the *connective tissue between them* is unwired in ten places. This spec closes those gaps under a single coherent narrative.

### 1.1 The 8B scaling pressure

| Layer | Naïve approach | What the substrate already commits to |
|---|---|---|
| Per-household observation volume | ~14/min sensor blips + UI events | Stays on libp2p, never DHT (2026-05-11 spec) |
| Crystallization to events | 1:1 entry per observation | Manifest-declared graduation policies, typically 1000:1 |
| Cross-household visibility | Replicate everything | Reach-scoped; only commons-attested federates |
| Long-tail inventory | Gossip every item | Subordinate under parent EPR; not independently gossiped |
| Historical depth | Hot replicas forever | Cold-archive in quilt; surface-on-demand |
| Care-economy authorship | Humans log every act | Elohim agents narrate the mundane (the value-prop unlock) |

Each layer is a cost-shedding mechanism designed for ~order-of-magnitude reduction in what the next layer must carry. The math works *if and only if* these mechanisms are wired correctly. Today they aren't.

### 1.2 The ten substrate gaps

```
 1.  Link types: EprToEvent, EprToResource                    (substrate)
 2.  parent_epr_cid: Option<Cid> on Event and Resource        (substrate)
 3.  Surface (re-elevation) operation                         (lifecycle op)
 4.  Submerge canonical signal reconciliation                 (lifecycle op)
 5.  EconomicResource ← StewardedResource consolidation       (cleanup)
 6.  Observation spec implementation + Stage 6 cleanup        (prerequisite)
 7.  Elohim-authoring pattern (domain-specialized agents)     (pattern)
 8.  Dissolution semantics (close/revive lifecycle)           (lifecycle op)
 9.  Bridge pattern for legacy systems                        (interop)
10.  Reach-mutation Events                                    (lifecycle op)
```

Part D specifies each.

### 1.3 Authorship floor + elohim feasibility-at-scale

The substrate is **floor-permissive**: humans, elohim, and bridge-stewardship-elohim can all author Events, Resources, and EPRs. Validity-ceiling (reach scope) is elohim-arbitrated based on standing, evidence, and council consensus.

The empirical claim is narrower than "elohim are required": humans *can* and do author REA at high-flow scale (ISO logistics, industrial supply chains, professional bookkeeping). What humans won't bear is the **everyday-care-economy frequency** — narrating every grocery trip, every caregiving hour, every shared meal, every couch in the garage. Elohim agents make REA tractable for that scale of frequency. Without them, the care economy is invisible to the substrate; with them, "the network sees the nurse's 3 AM compassion" (per `economic_coordination/epic.md`).

This is the value-prop unlock: **an economy that can scale love and care.**

### 1.4 Subsumption posture (not displacement)

Legacy banks, factories, payment processors, KYC providers, regulators exist outside the substrate. The protocol's posture is **parallel operation + subsumption-by-merit**, not displacement. Bridges (Gap 9) translate legacy systems into substrate-native Observations authored under stewardship-elohim signature. The protocol's commitment is that the substrate-native version always wins on merit — and if it doesn't yet, the bridge keeps things working. Cash-out (exit) is bidirectional and structural.

---

## 2. The eight foundational primitives

Everything composes from these eight. Subordinate records aren't a ninth primitive — they're an Event or Resource *with* `parent_epr_cid`. Cold records aren't a ninth primitive — they're a Commitment with `custody-quilt, tier_floor=shelved`. The lifecycle gradient lives in *transitions between these eight*, not in new types.

| # | Primitive | Where it lives | Hyperscale analog |
|---|---|---|---|
| A.1 | **EPR** (`Content` + `content_type`) | DHT (notarized) | Postgres row + S3 object fused; content-addressed identity |
| A.2 | **Event** (`EconomicEvent` + action verb) | DHT (notarized) | Kafka event with built-in double-ledger discipline |
| A.3 | **Resource** (`EconomicResource` + classification) | DHT (notarized) | S3 object with event-sourced state |
| A.4 | **Observation** | libp2p / iroh-blob (ephemeral) | Splunk / structured-log stream with retention classes |
| A.5 | **Commitment** | DHT (notarized) | Spring-Batch scheduled job + custody primitive |
| A.6 | **Attestation** (`Content` + `content_type: "attestation:*"`) | DHT (notarized) | PKI certificate with auditable evidence chain |
| A.7 | **FeedbackSignal** | DHT (notarized) | Webhook gated by reach |
| A.8 | **Links** (graph edges) | DHT (notarized) | GraphQL edges; cheap, unbudgeted |

### 2.1 Section template per primitive

```
 1. What it is (one paragraph)
 2. Hyperscale analog
 3. Data flow (author → validate → gossip → sync → project → query)
 4. Physical storage (source of truth, operational copy, web2 projection, attachments)
 5. Gossip / sync layer (what goes over DHT, libp2p sync plane, iroh-blob)
 6. Provenance — maintained forever vs intentionally degraded
 7. Agentic intelligence at scale (where elohim cognition is load-bearing)
 8. Scale: household → hub → global
 9. Limit-awareness / capture prevention
10. Network resilience (redundancy, partition recovery, cold-archive recoverability)
11. Dashboard worked example (how this primitive shows up in the Monarch surface)
```

### 2.2 Naming note

Throughout Part A and Part B, we use **Event** and **Resource** semantically (the polymorphic frame: a learner attempt is an event but not strictly economic; a content view is an event without being a transaction). The live DHT entry-type names today are `EconomicEvent` and `EconomicResource` per REA convention. Treating the rename as a separable substrate cleanup (deferred above).

---

# Part A — Foundational primitives walkthrough

## A.1 EPR (Elohim Protocol Record)

### 1. What it is

An EPR is the substrate's **vessel** — a notarized record on the DHT that holds *state* and *identity*. Implemented as a `Content` entry with a `content_type` discriminator (`content_type: "household"`, `"video"`, `"account"`, `"site"`, etc.). Per the substrate's "no new DHT entry types" commitment, every new EPR-shape is a new manifest-declared `content_type` value, not a new entry type. EPRs accumulate Events and Resources under their custody (via `parent_epr_cid` from Gap 1/2), participate in reach-coupling (visibility scoping), and traverse lifecycle transitions (active → subordinate-of-parent → shelved → closed).

### 2. Hyperscale analog

**Think Postgres row + S3 object fused, with content-addressed identity replacing master/replica topology.** Like Postgres, an EPR has typed fields and is referenced by stable identity. Like S3, the identity is content-derived (CID) — no server needed to vend an authoritative copy. Unlike either, an EPR is *gossip-validated through DHT validators* (Holochain integrity zomes) rather than synchronized to a master. The cost: writes are gossiped, not transactional. The win: no master, no shard manager, no replication lag, identity travels with the bytes, and you can't lose your data when AWS has a bad day.

### 3. Data flow

```
Author signs entry  (floor-permissive: anyone — human or elohim)
        │
        ▼
Holochain coordinator zome accepts → calls integrity validators
        │
        ▼
DHT validator quorum gossip-validates (O(log N) neighborhood)
        │
        ▼
Entry replicates to neighborhood peers per DHT redundancy policy
        │
        ▼
Subscribers projecting this content_type pull via libp2p sync plane
        │
        ▼
Local SQL projection ──▶ application queries (dashboard, app, doorway)
```

### 4. Physical storage

| Layer | What lives there | Shape |
|---|---|---|
| **Source of truth** | Author's source-chain entry + DHT shard replicas | Holochain DHT |
| **Operational copy** | SQL projection on every peer with reach | Postgres/SQLite `content` table (+ `epr_atoms` post-Wave-3) |
| **Web2 projection** | Optional doorway-service SSR cache | Redis-shape cache + SQL view |
| **Large attachments** | Referenced by CID, bytes elsewhere | iroh-blob (pull-fetched) |

### 5. Gossip / sync layer

- **DHT**: full entry payload (1–10 KB typical: metadata + small payload, or metadata + blob_cid for large content); gossip latency 200–2000 ms; per-peer entry budget ~3000 before degradation
- **libp2p sync plane**: SQL row projections sync via the Phase 11 SyncManagerBackend pattern; delta-sync per cursor; high-throughput, low-latency, cheap-trust
- **iroh-blob plane**: large referenced assets (video, photo, document body) — pull-fetched on demand, never gossiped

### 6. Provenance — maintained vs intentionally degraded

**Maintained cryptographically forever:**
- Signature chain (author signed at write)
- Content-address (CID)
- DHT validator quorum (other peers attested to validity at write time)
- Accumulated Event references (the EPR's history-of-touches via `EprToEvent` links)

**Intentionally degraded (access cost, not truth):**
- **At cold-archive** (shelved): payload erasure-coded across fewer peers; CID still verifiable but retrieval is K-of-N pull from quilt
- **Subordinate under parent**: don't independently gossip — queryable only via parent's reach scope, with a single network hop instead of N
- **At dissolution** (closed): future-Events cannot bind to the closed EPR (validation rejects); existing Event-history remains queryable forever
- **Right-to-be-forgotten**: subject-EPR root-rewrite via mishpat governance; downstream attestations carry `redaction-applied` notes (truth preserved at the structural level; PII removed at the content level)

The substrate's commitment is to **truth-verifiability, not free-access-forever.** The CID is forever; the cost of retrieval scales with lifecycle stage. This cost-shedding is what makes 8B-scale possible.

### 7. Agentic intelligence at scale

Where elohim cognition is load-bearing:

- **Reach arbitration** — the floor admits everyone; the ceiling (community / commons / commons-attested reach) is elohim-mediated based on standing, evidence, and council consensus
- **Demotion** — when a Resource stops actively flowing, the domain-elohim (inventory-elohim, vehicle-elohim) authors the subordination link to its parent EPR
- **Dissolution narration** — when an Event(action="dispose") lands, the elohim closes the loop on dependent Resources, notifies linked EPRs, archives the history into quilt
- **Bridge stewardship** — when external evidence enters via a `bridges/*` crate (Plaid bank-import, Stripe commerce, KYC), a stewardship-elohim signs the resulting Observations on behalf of the household

What humans alone can't do at care-economy scale: continuously narrate "couch is still in the garage, still part of household inventory, still worth ~$200." Across 10k stuff-objects × 100M households, the maintenance load is impossible for humans. Elohim agents carry it. **This is the value-prop unlock.**

### 8. Scale: household → hub → global

- **Local DHT (household elohim-node)**: holds EPRs in your reach scope. ~1k EPRs × 10 KB ≈ ~10 MB SQL projection per household for "everything I care about."
- **Hub (collective elohim-node)**: aggregates queries across member households via *federated SQL* — does NOT replicate member EPRs unless they elect commons-attested reach. Hub holds collective-level EPRs (the collective itself, shared assets, joint commitments).
- **Global**: the DHT itself sees ~3000 entries/peer before degradation. Practical capacity: tens-to-hundreds of millions of EPRs cross-network. NOT 8B-of-everything — the DHT carries what *earns* commons-reach. Personal/household EPRs stay local; only public/commons EPRs federate.

### 9. Limit-awareness / capture prevention

- DHT validator quorum prevents single-peer falsification
- Friction-gradient limitarianism: an entity accumulating EPR-authorship concentration faces rising friction (rate-limit-by-standing)
- Reach is **earned**, not declared — high-reach EPRs face elohim arbitration before federating
- Anti-concentration recurses to elohim-agents themselves (no single elohim can dominate reach-gating; council pattern checks)

### 10. Network resilience

- DHT shard-N redundancy: every entry replicated to ~10 peers
- Partition recovery: cursor-tracked libp2p sync handles partition heal-up; missing entries pulled by CID
- Cold-archive recoverability: K-of-N erasure coding in quilt; recover with K honest peer responses
- Doorway projection: web2 reach for the unconnected (browser, no Holochain peer)

### 11. Dashboard worked example (preview)

A Monarch dashboard's primary working surface is the Household EPR + its child Account EPRs + the Household-Inventory EPR. These ~50–100 EPRs sit in your local SQL projection from initial sync. Reads at render time: **zero network**. Each EPR's CID is the stable identity that lets aggregation across the 3-family collective work without replicating the underlying EPRs into the collective hub — the hub queries each household's local projection. Full walk in Part B.1.

---

## A.2 Event (`EconomicEvent` + action verb)

### 1. What it is

An Event is the substrate's **fact-of-action** — an immutable, DHT-notarized record that something happened between a provider and a receiver involving a resource. Implemented as `EconomicEvent` in the elohim DNA's `content_store_integrity` zome, with a mandatory `action` field drawn from the manifest-declared `REA_ACTIONS` vocabulary (24 verbs today: `transfer`, `produce`, `consume`, `use`, `cite`, `work`, `deliver-service`, `custody-blob`, `serve-blob`, `operate-doorway`, and more). Every action has a provider, a receiver, a resource shape (`resource_classified_as_json`, `resource_conforms_to`, `resource_inventoried_as`), an optional quantity, a timestamp, and optional links to the Commitment it fulfills, the Agreement it realizes, and the Processes it feeds. Events are append-only — corrections author a new Event with `triggered_by` pointing to the superseded one; the original remains in the audit trail. Events accumulate under an EPR via the `parent_epr_cid` field (Gap 2) and the `EprToEvent` link (Gap 1); in isolation they also carry their own DHT-indexed links (`IdToEvent`, `ProviderToEvent`, `ReceiverToEvent`, `EventByAction`, `EventByLamadType`, `EventFulfillsCommitment`).

### 2. Hyperscale analog

**Think Kafka event record + double-entry bookkeeping constraint, with no Kafka broker and no accountant.** Like a Kafka record, an Event is immutable, timestamped, carries a typed payload, and is referenced by stable identity. Like a double-entry ledger row, it encodes the *duality* — the provider's outflow and the receiver's inflow are the same record, not two separate book entries (REA's "independent view"). Unlike either, the record is gossip-validated by a DHT neighborhood rather than appended to a managed partition: no Kafka cluster to lose, no accountant to bribe. The mass-balance discipline is structural — the substrate can compute conservation across any set of Events without a reconciliation step, because the ledger was never split. The cost is gossip latency (200–2000 ms per write); the win is a self-auditing ledger that no single party can manipulate.

### 3. Data flow

```
Author decides: direct author (stake_class=high) or graduation path (stake_class=operational)?
        │
        ├── High: human or elohim signs EconomicEvent directly
        │          │
        │          ▼
        │     Coordinator zome: create_economic_event()
        │     Integrity validator checks action verb ∈ REA_ACTIONS, required fields
        │
        └── Operational: observation stream graduates via elohim graduation-evaluator
                   │
                   ▼
              Graduation-evaluator (tokio task in elohim-storage) fires
              Observation window closes → summary EconomicEvent authored
              observation_refs[] populated (iroh:// URIs per 2026-05-11 spec)
        │
        ▼ (both paths converge here)
DHT validator quorum gossip-validates (O(log N) neighborhood)
        │
        ▼
Entry replicates to neighborhood; IndexByAction anchor + ProviderToEvent link written
        │
        ▼
post-commit signal → ElohimContentSignal dispatcher → ReconcileController projector
        │
        ▼
Diesel upsert: economic_events table (dht_anchor_hash NOT NULL)
        │
        ▼
Balance derived views recompute → application queries (dashboard, Monarch surface)
```

### 4. Physical storage

| Layer | What lives there | Shape |
|---|---|---|
| **Source of truth** | Author's source-chain entry + DHT shard replicas; countersigned by both parties for `transfer` | Holochain DHT, `content_store_integrity::EconomicEvent` |
| **Operational copy** | SQL projection via ReconcileController | `economic_events` table (Diesel), `dht_anchor_hash` column links to DHT |
| **Derived balances** | SQL views over events group-by resource | No stored state — recomputed on read; zero DHT entries per Resource balance |
| **Observation evidence** | `observation_refs[]` iroh-blob log positions for graduated Events | iroh-blob (pull-fetched by auditors; never gossiped in full) |
| **Web2 projection** | Doorway SSR cache for the unconnected | Redis-shape summary (balance + recent transactions per EPR scope) |

### 5. Gossip / sync layer

- **DHT**: one ~1–3 KB entry per authored Event (field payload + links + countersignature for transfers). DHT gossip latency 200–2000 ms. Operational events that graduate as summaries — e.g., `served-blob-summary` covering 1,247 raw serves — contribute **one** DHT entry not 1,247 (the 1000:1 graduation ratio in practice). Per-peer DHT budget ~3000 entries before degradation; high-frequency operational activity must graduate, or the DHT saturates within weeks.
- **libp2p sync plane**: SQL projection rows delta-sync'd via cursor-tracked `ReconcileController` on the sync plane. After DHT write, local peers catch up within one sync cycle. High throughput, low-trust transport.
- **iroh-blob plane**: `observation_refs` payloads (the raw observation logs that fed a graduated summary Event) stay in iroh-blob; pull-fetched by auditors or attestation issuers following `iroh://<observer_cid>@<log_cid>#<offset>` URIs. Never gossiped in full.

**The stake-class gate** (from 2026-05-11 observation/event layer spec §8.3): the manifest declares each action verb as `stake_class: high` (coordinator accepts direct authored writes) or `stake_class: operational` (coordinator requires non-empty `observation_refs` + valid `graduation_policy` provenance). This gate is the DHT budget's primary safeguard — accidental high-volume direct writes are rejected at validation, not at post-hoc cleanup.

### 6. Provenance — maintained vs intentionally degraded

**Maintained cryptographically forever:**
- Signature chain (author signed at write; countersignature for two-party transfers)
- DHT validator quorum (neighborhood attested validity at write)
- `action` verb, `provider`, `receiver`, `has_point_in_time` — core audit facts
- `triggered_by` chain linking a correction to the superseded Event
- `observation_refs[]` CIDs — the iroh-blob log URIs remain verifiable as long as any peer retains the log segment (erasure-coded per retention_class)

**Intentionally degraded (access cost, not truth):**
- **Graduated summary Events**: raw observation payloads pruned from iroh after retention window (7 days operational, 90 days summarized); the graduated EconomicEvent remains on DHT; auditors need to pull iroh-blobs to reconstruct individual observations
- **Subordinate Events** (child of a shelved EPR): accessible only via parent's reach scope; not independently gossiped when parent is cold-archived
- **Corrected Events** (state=`corrected`): remain on DHT but are excluded from balance derivation views; accessible via the `triggered_by` chain for audit purposes
- **Dissolution**: `Event(action="dispose")` closes the subject Resource; future Events targeting the closed Resource fail integrity validation deterministically; the dispose Event itself stays on DHT as the record of closure

The substrate's commitment is to **truth-verifiability, not free-access-forever.** Every Event's DHT entry is permanently accessible; cost of retrieving the evidence it references scales with time and retention class.

### 7. Agentic intelligence at scale

Where elohim cognition is load-bearing:

- **Graduation-elohim (per pillar)**: The graduation-evaluator tokio task is what makes the 1000:1 ratio possible. A human cannot manually batch 1,247 card-swipe observations into a single hourly summary Event while preserving every `observation_ref`. The shefa-elohim does this as protocol plumbing — observation window closes, summary authored, iroh-blob references populated, DHT write emitted.
- **Care-stewardship-elohim**: care-economy Events (meals provided, hours of caregiving, couch-still-in-garage) are the output of an elohim agent observing household activity and authoring `work` or `produce` Events. Without this agent, care labor is invisible to REA. With it, "the network sees the nurse's 3 AM compassion" — the economic_coordination epic's core value claim.
- **Correction-elohim**: when a bridge-authored Event is disputed (bank import error, merchant dispute), the correction flow (new Event with `triggered_by`) can be initiated by the stewardship-elohim and confirmed by the human, rather than requiring the human to navigate a correction UI.
- **Reach-mutation-elohim** (Gap 10): when a `grant-reach` or `revoke-reach` Event is needed, the elohim evaluates standing and council signals before authoring — humans don't track those signals.

What humans alone can't do at care-economy scale: continuously author REA-shaped Events for ambient household activity at ~14 events/minute. **This is the value-prop unlock** — elohim authoring is what makes care, stewardship, and contribution economically legible without burdening the person with bookkeeping.

### 8. Scale: household → hub → global

- **Local DHT (household elohim-node)**: 10 years × 50 events/day × 2 KB = ~365k entries × 2 KB ≈ ~730 MB raw DHT payload, but most arrives via graduation (1000:1 for card-swipes, 1:1 for transfers). Practical entry count: ~180k Events/10 years if graduation is wired correctly. SQL projection: ~90 MB per the Monarch footprint analysis in `applications/mint-monarch-application-design.md`.
- **Hub (collective elohim-node)**: the hub holds collective-level Events (joint commitments fulfilled, shared asset transfers, governance enactments) but does NOT replicate member household Events. Cross-household aggregation is a federated SQL query, not a data copy. The 3-family collective view in Monarch queries each household's local projection without pulling Events to hub.
- **Global**: only commons-attested Events federate across the full DHT. Personal finance Events stay household-scoped; public-good Events (care-attestation, commons-contribution) earn commons-reach through elohim arbitration. The ~3000-entry-per-peer DHT budget is protected by stake-class gating and graduation compression.

### 9. Limit-awareness / capture prevention

- **DHT validator quorum**: no single peer can author a false Event without a neighborhood of validators rejecting it. Transfer Events require countersignature — both provider and receiver attest.
- **Immutability + correction chain**: you cannot delete an Event; you can only author a correction that supersedes it. The correction itself is visible. This is the substrate's anti-fraud primitive.
- **Stake-class gate in coordinator**: manifest-declared `stake_class: operational` Events require graduation provenance (`observation_refs` non-empty). Direct high-volume writes are rejected at the coordinator, not after the fact.
- **Friction-gradient limitarianism**: an agent authoring an anomalous volume of high-value `transfer` Events faces rising DHT validator scrutiny (elohim arbitration, standing checks). Concentration of authorship triggers the friction gradient.
- **Care-class / compute-class isolation**: care Events (`work`, `produce` with `resource_classified_as=care-token`) and compute breach signals ride separate `signal_kind` streams. Compute Events cannot contaminate care attribution; care debits cannot gate compute placement. This is a substrate-invariant enforced through `resource_classified_as` whitelists, not ad-hoc fields.

### 10. Network resilience

- **DHT shard-N redundancy**: every Event entry replicated to ~10 neighborhood peers; losing any peer loses nothing.
- **Partition recovery**: cursor-tracked libp2p sync handles partition heal-up; Events missed during partition are pulled by CID when the peer reconnects. The correction-chain pattern means that even if an Event was authored during a partition, the superseded+corrected chain is coherent once the partition heals.
- **Observation evidence**: iroh-blob logs are erasure-coded K-of-N (Reed-Solomon via quilt substrate); if the authoring peer goes offline, the observation evidence is recoverable from the quilt. This matters for graduated Events whose `observation_refs` point to logs that may have been pruned from hot storage.
- **Doorway projection**: the doorway SSR cache surfaces balance summaries and recent transactions for web2-only clients (no Holochain peer). When the peer comes back online, the projection reconciles against the DHT authoritative state.
- **Correction resilience**: because corrections are new Events (not mutations), a network partition cannot cause two conflicting authoritative versions. The latest correction in the `triggered_by` chain wins for balance derivation; all prior Events remain queryable.

### 11. Dashboard worked example (preview)

In the Monarch/Mint personal-finance dashboard (`applications/mint-monarch-application-design.md`), Events are the primary data in four widgets:

**Recent transactions widget:**
```sql
SELECT action, provider, receiver, resource_quantity_value, resource_quantity_unit,
       event_classified_as, has_point_in_time, note
FROM economic_events
WHERE parent_epr_cid = :account_cid
  AND state != 'corrected'
ORDER BY has_point_in_time DESC
LIMIT 50;
```

**Monthly cash flow (spend by category):**
```sql
SELECT event_classified_as,
       SUM(resource_quantity_value) AS total,
       COUNT(*) AS count
FROM economic_events
WHERE provider = :agent_id
  AND action = 'transfer'
  AND has_point_in_time BETWEEN :month_start AND :month_end
  AND state NOT IN ('corrected', 'disputed')
GROUP BY event_classified_as
ORDER BY total DESC;
```

Both queries run against the local Diesel SQL projection — zero network calls at render time. The DHT `dht_anchor_hash` column lets any widget offer a "verify on-chain" affordance: clicking it opens the Holochain entry in a block-explorer-style view. Full walk in Part B.1.

## A.3 Resource (`EconomicResource` + classification)

> *Stubbed — full draft pending. Hyperscale analog: S3 object whose state is mass-balance-derived from event history (event-sourcing with REA as the reduce function). Critical sub-sections: balance-as-derived-view (not stored); `resource_classified_as` for stewardship variants (subsumes StewardedResource per Gap 5); CID continuity across surface re-elevation; subordination via `parent_epr_cid`; dissolution to closed state.*

## A.4 Observation

> *Stubbed — full draft pending. This section is **largely a citation** of 2026-05-11-observation-event-layer-design.md. Hyperscale analog: Splunk / structured-log stream with retention classes. Critical sub-sections to integrate from the cited spec: the ephemeral nature (libp2p + iroh-blob, never DHT); the diversity-tagging model; the graduation paths (Path 1: Attestation; Path 2: summary Event); the retention-class table (operational 7d, contextual 30d, archival 90d, wisdom indefinite); the "witness, not surveillance" constitutional posture.*

## A.5 Commitment

> *Stubbed — full draft pending. Hyperscale analog: Spring-Batch scheduled job (planned-future-Event) AND the custody primitive for cold-archive (`Commitment(action="custody-quilt", tier_floor=shelved)`). Critical sub-sections: dual-role nature (planning AND custody); the `custody-blob`/`custody-quilt`/`custody-shelved` ladder; how Commitments fulfill into Events; the cancellation flow.*

## A.6 Attestation (`Content` + `content_type: "attestation:*"`)

> *Canonical spec: [2026-05-11-attestation-consolidation-design.md](./2026-05-11-attestation-consolidation-design.md) — read end-to-end for the full consolidation rationale, the 18+ entry-type migration, M-of-N governance-action decomposition, Shamir-off-DHT decoupling, and the 7-stage migration plan. This section integrates Attestation into the records-lifecycle vocabulary and surfaces the tensions specific to this spec's concerns.*

### 1. What it is

An Attestation is a **validated claim derived from one or more Observations** — notarized on the DHT as a `Content` entry with `content_type: "attestation:<subtype>"`. No new DHT entry type is introduced; the carrier reuses `Content`'s full field set (`id`, `author_id`, `reach`, `metadata_json`, `related_node_ids`) plus a typed `AttestationToSubject` link from the attestation to the entity it attests about. The **issuer** is `author_id` (signed by the Holochain action); the **subject** is the link target. Subtypes are declared in pillar manifests and generated into `ATTESTATION_KINDS` in `content_store_integrity/src/generated_attestation_kinds.rs` at codegen time — the integrity zome's Floor 1 check rejects unknown subtypes unconditionally. Attestations are the protocol's notary layer for human capability, content quality, device health, governance decisions, and computation results. They answer "how does the substrate know this claim is true?" with a signed, gossip-validated, evidence-linked entry that costs DHT budget proportional to trust requirement.

### 2. Hyperscale analog

**Think X.509 PKI certificate + an auditable evidence chain back to structured log positions — but without a certificate authority.** Like a PKI cert, an Attestation carries: issuer identity, subject identity, claim content, validity window, and signature. Unlike PKI, there is no CA; the DHT validator quorum replaces the CA's root-of-trust. The issuer's standing (derived from prior Attestations + FeedbackSignal history) plays the role that CA reputation plays in traditional PKI. Unlike a cert, every Attestation optionally carries `evidence_json.observation_refs` — iroh-blob CIDs pointing to the raw evidence behind the claim. A skeptical auditor can pull those bytes and verify the claim without trusting the issuer. The four `proof_evidence.class` tiers from `2026-05-01-computation-attestation-graduated-rigor-design.md` map to familiar validation depths: domain-validated (witness), organization-validated (audit), extended-validation (proof), hardware-attestation (confirmation).

### 3. Data flow

```
Graduation evaluator / elohim-agent / human issuer
        │
        ▼
coordinator: issue_attestation(input)
  ├─ create Content entry  (content_type: "attestation:<subtype>")
  └─ create AttestationToSubject link  (same action)
        │
        ▼
integrity validator  (attestation_validator.rs discriminator-chain)
  F1: subtype ∈ ATTESTATION_KINDS               [IMPLEMENTED]
  F5: expires_at RFC3339; parent gov-action exists [IMPLEMENTED]
  F7: supersedes_cid → same-kind, same-issuer   [IMPLEMENTED]
  F8: proof_evidence.class declared + material  [IMPLEMENTED]
  FG3: recovery-approval ≠ Shamir share bytes   [IMPLEMENTED]
  F2/F4/F6: manifest-aware issuer / eligibility [TODO(C.3)]
        │
        ▼
DHT gossip  →  neighborhood validation quorum
        │
        ▼
post-commit signal  →  ElohimContentSignal dispatcher
        │  →  AttestationProjector
        ▼
Diesel upsert into `attestations` table  (dht_anchor_hash NOT NULL)
        │
        ▼
application queries: GET /api/v1/attestations?subject={cid}&kind={subtype}
```

### 4. Physical storage

| Layer | What lives there | Shape |
|---|---|---|
| **Source of truth** | Author source-chain + DHT neighborhood replicas | Holochain DHT (elohim DNA `content_store_integrity`) |
| **Operational copy** | Unified projection rebuilt from signal stream | SQLite `attestations` table (per 2026-05-11 spec §7.4; `dht_anchor_hash` NOT NULL) |
| **Tally projection** | M-of-N governance-action state | SQLite `governance_action_tally` table (Category C — rebuildable any time) |
| **Evidence bytes** | Raw observation logs cited by `observation_refs` | iroh-blob (pull-fetched by CID; never gossiped) |
| **Web2 projection** | Optional doorway SSR cache | Redis-shape; filtered by `attestation_kind` + expiry + `supersedes_cid` chain |

### 5. Gossip / sync layer

- **DHT**: full `Content` entry payload — typically 1–3 KB for attestation metadata + proof material; `AttestationToSubject` link tag (subject_kind); gossip latency 200–2000 ms. Revocations gossip as new Content entries (append-only) — the projection's `supersedes_cid` join surfaces current status.
- **libp2p sync plane**: `attestations` projection rows delta-sync via cursor; M-of-N vote children arrive as individual entries, tally converges within one sync cycle of quorum; expiry filtering is projection-layer, not DHT-layer.
- **iroh-blob**: raw observation evidence — the log positions cited in `evidence_json.observation_refs`; pull-fetched only when an auditor needs the backing evidence; sizes range from < 1 KB (single-observation summary) to many MB (audit-class Merkle-rooted computation inputs); never gossiped; cold-archivable via quilt K-of-N.

### 6. Provenance — maintained vs intentionally degraded

**Maintained cryptographically forever:**
- Issuer signature (Holochain action header; unforgeable; carried in the Content entry)
- Content-address CID (content-derived; the attestation identity is its bytes)
- DHT validator quorum acceptance (other peers validated at write time; recorded in action headers)
- `AttestationToSubject` link (subject identity verifiable by any peer with reach)
- `proof_evidence.class` and all required proof material (`merkle_root`, `proof_blob`, `confirmer_signature` per class — stored on-entry, not reconstructed)
- Revocation chain: the `supersedes_cid` trail is append-only and forever traversable

**Intentionally degraded (access cost, not truth):**
- **Cold observation evidence**: iroh-blob log positions in `observation_refs` may move to quilt (K-of-N recovery); the CIDs remain forever; retrieval cost scales with cold-archive tier
- **Expired attestations**: filtered from "current capability" queries in the projection layer; remain on DHT, queryable by CID forever
- **Revoked attestations**: projection-filtered by `supersedes_cid` chain; the original remains on DHT (right-to-be-forgotten is a mishpat root-rewrite at the EPR layer, not at individual Attestation entries)
- **Vote children on failed proposals**: lose projection visibility once tally closes; persist on DHT forever as auditable record of the governance deliberation

The substrate's commitment is **truth-verifiability, not free-access-forever.** A revoked attestation is verifiably revoked; a cold-archived observation is verifiably retrievable at cost. The CID is forever.

### 7. Agentic intelligence at scale

Attestations are where elohim cognition is most load-bearing across four specializations:

- **Graduation evaluator** (`elohim-storage/src/services/graduation_evaluator.rs`): per-pillar tokio task watching Observation accumulation; fires `issue_attestation` when a policy threshold is met — e.g., 5 health check observations with p95 latency < 500 ms in 24 hours → `attestation:device-health` at class=witness. Humans cannot monitor 100M device streams.
- **elohim-vision-agent**: for Google-Photos-shape applications, issues `attestation:auto-tag` and `attestation:face-cluster` referencing photo EPRs by CID. Without the vision-agent, no one tags 100k household photos; face clusters link multiple EPRs via `subject_face_cid` in `evidence_json`.
- **stewardship-elohim**: signs Observations authored by legacy bridges (Plaid, Stripe, KYC providers); the graduation evaluator decides when accumulated observations warrant a credential attestation — `attestation:identity-credential` after successful KYC bridge verification.
- **computation-elohim**: for AWS-compute-shape workloads, issues `attestation:computation` at the appropriate `proof_evidence.class` per the compute-attestation gradient — interpreting four signals: stakes, spread, consensus-deficit (FeedbackSignal::Correction firing rate), and provability ceiling. See `2026-05-01-computation-attestation-graduated-rigor-design.md` for the full treatment.

What humans alone can't do at care-economy scale: issue, track, and revoke thousands of graduated capability claims per household member per year across mastery, health, governance, and computation domains. **Elohim agents carry the continuous certification load — this is the value-prop unlock.**

### 8. Scale: household → hub → global

- **Local DHT (household elohim-node)**: ~50 Attestations in the Monarch/Mint archetype (price feeds, KYC credentials, health certifications) × ~3 KB = ~150 KB SQL projection. For a Photos household: thousands of auto-tag Attestations × 1 KB = a few MB SQL. Both fit comfortably in the household footprint.
- **Hub (collective elohim-node)**: hub holds governance-action parent entries (proposals, challenges, elections) and the `governance_action_tally` projections for its governance scope. Hub does NOT replicate individual household Attestations unless they elect community/commons reach. Vote children and stewardship-grants are community-reach and gossip to hub peers.
- **Global**: `attestation:humanness` and `attestation:identity-credential` are the highest-reach Attestations — ~1–3 per human × 8B humans = ~8–24B global entries. These earn their DHT budget: humanness-attestation is the substrate's primary Sybil defense. All other subtypes are scoped narrower (community / household / agent-private).

### 9. Limit-awareness / capture prevention

- **Manifest-gated subtypes (Floor 1, implemented)**: only subtypes in `ATTESTATION_KINDS` (generated from pillar manifests) can be written. A new claim vocabulary requires a manifest PR — not just a write attempt. Unknown subtypes fail-closed at the integrity layer.
- **Issuer-authorization floors (F2/F4/F6 — TODO C.3)**: when wired, these gates ensure `attestation:mastery` can only be issued by an agent who themselves holds `attestation:steward` in that concept domain — preventing a single-issuer monopoly on capability recognition.
- **Shamir-off-DHT (Floor G3, implemented)**: `attestation:recovery-approval` entries are rejected by the integrity zome if they contain `share_data`, `share_index`, or `share_blob` anywhere in their metadata. The architectural boundary is substrate-enforced, not advisory.
- **Revocation is issuer-scoped (Floor 7, implemented)**: only the original issuer can revoke an attestation (same `author_id` verified via `must_get`); third-party revocation requires a mishpat governance-action.
- **Friction-gradient**: high-volume Attestation issuers face rising DHT write friction proportional to standing; commons-reach Attestations face elohim-council arbitration before federating.

### 10. Network resilience

- **DHT shard-N**: each Attestation Content entry replicates to ~10 neighborhood peers; the entry survives any single-peer failure
- **Partition recovery**: cursor-tracked libp2p sync delivers M-of-N vote children across partition heal; tally converges on next sync cycle; `governance_action_tally` is Category C and fully rebuildable from the signal stream
- **Cold-archive of evidence**: iroh-blob positions in `observation_refs` survive quilt K-of-N erasure; a future audit can reconstruct the backing evidence even if original peers have long since left the network
- **Revocation propagation**: new revocation Content gossips the same as any Attestation; projection `supersedes_cid` join surfaces current status immediately; doorway serves the filtered view to unconnected clients
- **Governance deadline resilience**: `closes_at` is structurally validated (Floor 5); late votes after `closes_at` fail at the integrity layer — deterministic deadline enforcement is preferred over eventual-consistency ambiguity for binding acts

### 11. Dashboard worked example

Three application archetypes where Attestations are load-bearing:

**Monarch/Mint** (`applications/mint-monarch-application-design.md`): The personal-finance dashboard holds ~50 Attestations in local SQL — primarily `attestation:price-feed` entries (daily-cadence, reach=commons, issued by oracle-elohim) that enable net-worth calculation for investment accounts, and `attestation:identity-credential` entries that KYC-gate bridge access (Plaid, Stripe). Price-feed Attestations earn their DHT budget: without them, investment balances are stale. KYC-credential Attestations are agent-private; they exist locally but never federate.

**Google Photos** (`applications/google-photos-application-design.md`): Vision-agent issues `attestation:auto-tag` and `attestation:face-cluster` per photo EPR. The local SQL query for "all photos of Maya" is:
```sql
SELECT a.subject_cid FROM attestations a
WHERE a.attestation_kind = 'attestation:face-cluster'
  AND json_extract(a.evidence_json, '$.face_label') = 'Maya'
```
Zero DHT reads at query time. Attestations stay agent-private until the family opts shared albums in; face clusters never federate without explicit reach grant.

**AWS Compute** (`applications/aws-compute-application-design.md`): `attestation:computation` `proof_evidence.class` is manifest-declared per workload type — witness for trusted high-standing providers, audit when FeedbackSignal::Correction rates rise, proof for high-stakes results. A provider's standing accumulates from fulfilled Commitments + computation Attestations not subsequently Corrected. The computation-elohim interprets the four-signal gradient at each issuance; the tally projection surfaces provider reputation without a centralized rating authority.

## A.7 FeedbackSignal

> *Stubbed — full draft pending. Hyperscale analog: webhook / event-notification gated by reach. Critical sub-sections: the documented edge case (the ONE social-move surface that earns DHT-tier cost, because reach-coupling requires authoring-time notarization); `signal_kind` extensibility; how FeedbackSignals contribute to reach earning/decay; manifest-declared validators.*

## A.8 Links (graph edges)

> *Stubbed — full draft pending. Hyperscale analog: GraphQL edges; cheap, unbudgeted. Critical sub-sections: the elohim DNA LinkTypes enum and current usage; the NEW link types added by this spec (`EprToEvent`, `EprToResource` per Gap 1); the existing patterns (`AttestationToSubject`, `Coupling`, `Membership`, `Delegation`); how links carry the graph that EPRs project as nodes.*

---

# Part B — Application archetypes (the proof gallery)

Each application archetype lives in its own canonical-architecture file under [`applications/`](./applications/), with its own frontmatter bridging epic → technical composition → code. Together they prove the substrate at the patterns the protocol is actively subsuming. Read [`applications/INDEX.md`](./applications/INDEX.md) for the architect-audience framing and the reading guide.

**Active subsumption targets:**

| File | Replaces | Pillar | Status |
|---|---|---|---|
| [`applications/mint-monarch-application-design.md`](./applications/mint-monarch-application-design.md) | Mint / Monarch.app | shefa | Full draft — exemplar |
| [`applications/khan-academy-application-design.md`](./applications/khan-academy-application-design.md) | Khan Academy | lamad | Composition draft |
| [`applications/google-drive-application-design.md`](./applications/google-drive-application-design.md) | Google Drive | lamad + elohim | Composition draft |
| [`applications/google-photos-application-design.md`](./applications/google-photos-application-design.md) | Google Photos | lamad + elohim | Composition draft |
| [`applications/meta-facebook-application-design.md`](./applications/meta-facebook-application-design.md) | Meta / Facebook | imagodei + qahal | Composition draft |
| [`applications/patreon-application-design.md`](./applications/patreon-application-design.md) | Patreon | shefa + lamad | Composition draft |
| [`applications/requests-offers-application-design.md`](./applications/requests-offers-application-design.md) | Amazon cooperative commerce | shefa | Composition draft |
| [`applications/aws-compute-application-design.md`](./applications/aws-compute-application-design.md) | AWS cloud compute | shefa + elohim | Composition draft |

**Horizons (deferred-but-coherent):** YouTube, WordPress, Factory, Bank — each in [`horizons/`](./horizons/) with graduation criteria.

The eight active archetypes test the substrate against eight different stress profiles. If they all hold, the substrate's theory is real. If even one breaks, the theory needs revision.

---

## Why this Part B is structured as separate files

Per the user's framing during the brainstorming: these archetypes are **technical-story archetypes parallel to the human-story archetypes in `value_scanner/epic.md`**. Each deserves its own frontmatter bridge from epic-narrative to technical composition to code anchors. They are not stubbed sections in this spec — they are first-class proofs that this spec's primitives compose into recognizable patterns at planet scale.

A skeptical systems architect should land in `applications/` and find each archetype concrete enough to battle-test on its own. That requires per-archetype frontmatter and code-anchor depth — not sub-sections in this primitives spec.

---

# Part C — Composability stress-test

> *Stubbed — full draft pending. **Load-bearing centerpiece.** Walks through what happens when ONE household's elohim-node participates simultaneously in all eight active application archetypes (per Part B): Mint/Monarch personal finance + Khan-Academy learning + Google-Drive document collaboration + Google-Photos media library + Meta/Facebook social graph + Patreon creator monetization + Requests & Offers cooperative commerce + AWS-shape peer compute. Demonstrates per-peer working-set bounds (~150 GB operational with cold archive elsewhere), DHT entry visibility (~100k entries per peer scope), libp2p bandwidth (~100 MB/month aggregate), observation rate (~1k/day local-only). The "households subsume hyperscale datacenters" claim is proven here: every household is its own datacenter for its own working set; the substrate federates queries (not data) when collectives need cross-household views; cold archive is itself peer-distributed in the quilt. The substrate's own use of compute (AI inference, sweettest jobs, graduation evaluators) is the bootstrapping demand for the AWS-shape archetype's supplier side.*

---

# Part D — Lifecycle wiring (the ten substrate gaps)

> *Stubbed — full draft pending. Each gap gets its own subsection with: motivation, design (entry types, link types, fields, coordinator functions, validation rules), manifest declarations, migration story, and test surface. Each subsection names the **specific code surfaces it touches** so the spec→code graph is walkable from any gap.*

### D.1 Subordination architecture (Gaps 1+2: `EprToEvent` / `EprToResource` link types + `parent_epr_cid` field)

**Touches:**
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — add to `LinkTypes` enum (`EprToEvent`, `EprToResource`); add `parent_epr_cid: Option<String>` field to `EconomicEvent` and `EconomicResource` structs
- `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` — new coordinator functions `create_event_under_epr`, `create_resource_under_epr`
- `elohim/elohim-storage/src/views.rs` — extend Event and Resource view types with `parent_epr_cid`
- `elohim/sdk/schemas/v1/views/economic-event-view.schema.json`, `economic-resource-view.schema.json` — add field
- `elohim/sdk/domains/*/manifest.json` — declare which content_types accept subordinate Events/Resources

### D.2 Surface (re-elevation) operation (Gap 3)

**Touches:**
- `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` — new coordinator function `surface_resource(resource_cid, new_parent_epr_cid)` that authors the surface Event + updates custody
- `elohim/sdk/domains/elohim/manifest.json` — declare `action: "surface"` verb with stake-class
- Validation: integrity zome validates that surface authorship comes from current custodian or elohim-with-stewardship-commitment

### D.3 Submerge canonical signal reconciliation (Gap 4)

**Touches:**
- `elohim/elohim-storage/src/services/reconcile_controller.rs` (planned) — project `Commitment(action="custody-quilt", tier_floor=shelved)` into both `memory-lifecycle/submerge` and `tiered-quilt/quilt-demoted` downstream effects
- `elohim/sdk/domains/elohim/manifest.json` — canonical `custody-quilt` action verb with `tier_floor` parameter
- Retire parallel vocabularies in `2026-05-10-memory-lifecycle-design.md` and `2026-05-11-tiered-quilt-stewardship-design.md` (those specs get amendment notes pointing here)

### D.4 EconomicResource consolidation (Gap 5)

**Touches:**
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — retire `StewardedResource` entry type; collapse its fields into `EconomicResource` via `resource_classified_as` discrimination
- Migration scripts under `genesis/migrations/` — pre-launch hard cutover; no backwards-compat shim
- Update all callers of `StewardedResource` to use `EconomicResource` with appropriate classification
- Budget win: -1 variant in EntryTypes enum

### D.5 Observation spec implementation prerequisite (Gap 6)

**Touches:**
- `elohim/elohim-storage/src/services/observation_manager.rs` (planned per 2026-05-11 Stage 4) — `ObservationManagerBackend` neutral service
- `elohim/holochain/dna/infrastructure/zomes/*/src/lib.rs` — retire `DoorwayHeartbeat`, `DoorwayHeartbeatSummary`, `HealthAttestation` (Stage 6 cleanup)
- `elohim/sdk/domains/infrastructure/manifest.json` — declare observation_kinds for heartbeat / blob-served / system-sample
- This gap is a **strict prerequisite** for this spec's substrate stages; see 2026-05-11 spec for full plan

### D.6 Elohim-authoring pattern — domain-specialized agents (Gap 7)

**Touches:**
- `app/elohim-app/src/app/elohim/elohim-agents/` (new) — TypeScript service implementations for `InventoryElohimService`, `VehicleElohimService`, `CareStewardshipElohimService`, etc., following the existing operator-elohim pattern
- `elohim/elohim-storage/src/services/graduation_evaluator.rs` (extend) — per-pillar tokio tasks that the elohim-agents drive
- `elohim/sdk/domains/*/manifest.json` — declare which elohim-agent watches which observation_kinds + handles which graduation policies

### D.7 Dissolution semantics (Gap 8)

**Touches:**
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — add validation: future Events targeting a closed Resource/EPR fail (substrate-floor invariant)
- `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` — coordinator handles `Event(action="dispose"|"close-account")` and updates Resource/EPR state
- `elohim/sdk/domains/elohim/manifest.json` — declare `dispose`, `close-account`, `revive` action verbs
- `elohim/sdk/schemas/v1/views/economic-resource-view.schema.json` — add `lifecycle_state: "active" | "subordinate" | "shelved" | "closed"` field

### D.8 Bridge pattern for legacy systems (Gap 9)

**Touches:**
- `bridges/<vendor>/` (new crates per vendor — e.g., `bridges/plaid/`, `bridges/stripe/`, `bridges/banking-api/`) — pattern reference: existing `bridges/valueflows/`
- `doorway/doorway-service/src/handlers/bridges/` (new) — HTTP route surface for bridge ingress/egress
- `elohim/sdk/domains/shefa/manifest.json` (and others) — declare `observation_kind` for each bridge type
- Stewardship-elohim signing pattern documented; bridges authenticate via stewardship-commitment Attestations

### D.9 Reach-mutation Events (Gap 10)

**Touches:**
- `elohim/sdk/domains/elohim/manifest.json` — declare `grant-reach`, `revoke-reach`, `reclassify-reach` action verbs
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — validate reach changes against current standing + elohim arbitration
- `elohim/elohim-storage/src/views.rs` — reach-state derived view for any EPR/Resource (current effective reach + history)

---

# Part E — Migration plan, scope, success criteria

> *Stubbed — full draft pending. Staged migration following the observation-spec pattern (manifest → wire/schema → storage → manager service → graduation evaluators → entry-type retirements → HTTP/storage-client → existing-table reclassification). Pre-launch hard cutover; no backwards-compat shims.*

---

## 99. Out of scope (deferred items)

- **Cradle-to-cradle dissolution philosophy** — "every birth knows what the end looks like" is a separate design session. This spec closes the implementation loop (close/revive lifecycle ops) but does not enumerate the full ethical/material decomposition discipline.
- **Per-vendor bridge implementations** — Plaid, Stripe, banking APIs, KYC providers, etc. The pattern is defined here (Gap 9); each vendor crate is its own sprint.
- **EconomicEvent → Event / EconomicResource → Resource DHT-entry rename** — treated semantically in this spec; the rename migration is a separable substrate cleanup.
- **Cross-collective EPR custody handoff mechanics** — owned by 2026-05-23-multi-collective-collaboration-epr-design.md; this spec references but does not re-spec.
- **Organization-dissolution lifecycle** (household-ending, collective-ending) — separate concern from Resource/Event dissolution.
- **Polis-style opinion clustering** — application layer over Observation diversity views; out of substrate scope.

## 100. Open questions

1. **Surface authorship**: when a shelved Resource re-elevates, can any agent author the surface Event, only the original custodian's lineage, or only an elohim with active stewardship commitment? Substrate-floor invariant decision.
2. **Dissolution Event's required fields**: does `Event(action="dispose")` require a `disposition_kind` (recycled / landfill / transferred-to-charity / sold) for the cradle-to-cradle hook even though full cradle-to-cradle is deferred? Probably yes; defaults to `unspecified` with a manifest-declared upgrade path.
3. **Bridge reach scope at write**: when a stewardship-elohim authors an Observation from a Plaid bridge, what's the default reach? Probably `agent-private` for personal-finance bridges, `community` for commerce/factory bridges. Pillar-manifest declaration.
4. **`event_classified_as` granularity**: is this a free-string discrimination, a manifest-declared enum, or a hierarchical taxonomy? Affects how dashboards group / how elohim auto-tag.
5. **Subordinate-Resource query cost when parent has high reach**: if a household-inventory EPR is opted into a collective with reach=commons-attested, do the subordinate couch Resources federate too? Default should probably be NO — subordination is per-level reach, not inherited.

## 101. Success criteria

- A new EPR with subordinate Events and Resources is queryable as a graph in local SQL within one libp2p sync cycle.
- A Resource transitions through Active → Subordinate → Shelved → Surfaced → Closed without losing CID identity or event-history queryability.
- The Monarch dashboard (Part B.1) renders against a fresh household elohim-node in under 200ms with 10 years of simulated event history.
- A 3-family collective view aggregates net worth without replicating any individual household's transaction-level Events.
- A Plaid bridge (or stub) authors Observations that graduate to Events under stewardship-elohim signature; disconnecting the bridge halts authoring but preserves prior Events.
- A `dispose` Event closes a Resource; subsequent Events targeting the closed Resource fail validation deterministically.
- Per-peer storage footprint under the composability stress-test (Part C) stays under 200 MB SQL projection for full participation in all six application patterns.
- `submerge` (memory-lifecycle) and `quilt-demoted` (tiered-quilt) signal flows unify behind a single canonical `Commitment(custody-quilt, tier_floor=shelved)` authoring event.
- `StewardedResource` is retired; all callers use `EconomicResource` with `resource_classified_as` discrimination.

---

*This spec exists to make the records lifecycle executable so the substrate can scale love and care to 8 billion humans without melting the peer fabric at the edges.*
