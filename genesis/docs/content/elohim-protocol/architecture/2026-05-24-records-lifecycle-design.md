---
title: Records Lifecycle — Wiring the EPR / Event / Resource Substrate
id: records-lifecycle-design
tier: architecture
status: Draft (Parts A + B + D content-complete; Part C deferred to dev-sprint measurement; Part E migration-plan stubbed; awaiting Wave 2 application-archetype full-drafts)
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

**Draft, content-complete on Parts A + B + D.** Part A primitives (A.1 EPR exemplar + A.2-A.8 walkthroughs landed via Phase 1 parallel-agent dispatch with structured concerns reports) are fully drafted. Part B application archetypes refactored to per-file `applications/` directory (Mint/Monarch full draft; Khan-Academy / Google Drive / Google Photos / Meta-Facebook / Patreon / Requests-and-Offers / AWS-Compute composition drafts pending Wave 2 full-draft dispatch — now safe to dispatch since Wave A vocabulary-drift gaps have closed). Part D substrate wiring (20 subsections D.1-D.20 across Waves A-E) is content-complete; this addresses the gaps identified in the Phase 2 findings synthesis (the original 10-gap list expanded to 21 substrate gaps; D.1 merges Gaps 1+2 so the spec carries 20 subsections). Part C composability stress-test is **deferred** to development-sprint measurement per operator direction — placeholders point to the 10 measurement scenarios the alpha-cluster dev work will populate. Part E migration plan is stubbed; follows from Part D's wave-ordering once implementation work begins.

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

> **Naming.** "Record" is the at-rest, notarized half of the term; the link primitive itself is the **Elohim Protocol *Reference*** (href's accountable cousin). Both expansions are canonical, kept on purpose — see the protocol specification, Part II ("On the name — two answers, on purpose"). Reference → Record is the same atom at compose-time and at rest.

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

### 1. What it is

A Resource is a **thing with economic value that flows through the network** — a photo in a family archive, a token of recognition earned by a contributor, a kilowatt-hour of stored energy, a quilt of bytes committed to a peer pantry, a content node that learners consume. Implemented as an `EconomicResource` DHT entry with a `classified_as_json` discriminator carrying one or more URI-class labels (e.g., `"content"`, `"compute"`, `"care-token"`, `"backup-state"`). Resources do not store their own balance or current state; those are **derived views** over the Event history that has acted upon them. After the Gap 5 consolidation, `StewardedResource` — a parallel entry type that stored computed capacity, allocation, and usage fields directly — is retired; all stewardship-variant semantics move into `EconomicResource` with appropriate `resource_classified_as` discrimination. A Resource's identity is content-addressed (CID), which means its identity survives subordination under a `parent_epr_cid`, cold-archive into the quilt, and `surface` re-elevation without any identity mutation.

### 2. Hyperscale analog

**Think S3 object + event-sourced aggregate, where current state is the reduce function over its Event history.** Like S3, a Resource has a stable content-addressed identity (the CID plays the role of the S3 key + ETag) and can reference bytes of any size via iroh-blob. Unlike S3, the resource's "current balance" (how much is available? who has custody?) is never stored as a field — it is materialized by the storage projection by replaying all `EconomicEvent` entries whose `resource_inventoried_as` references this resource's id. Concretely: `accounting_quantity_value` on the DHT entry is the value at write time; the *current* quantity is a derived SQL view: `SELECT SUM(quantity_delta) FROM economic_events WHERE resource_inventoried_as = ?`. A skeptical accountant knows this shape as double-entry bookkeeping without a running ledger; a skeptical storage engineer knows it as event-sourcing without snapshots. The hyperscale win: no balance update race, no reconciliation step, no separate table to keep in sync — the ledger *is* the state.

### 3. Data flow

```
Authoring agent (human or elohim) writes EconomicResource entry
        │
        ▼
Coordinator zome (content_store) → integrity validators:
  - classified_as values against RESOURCE_CLASSIFICATIONS whitelist
  - primary_accountable / custodian are valid agent IDs
  - if parent_epr_cid set: parent EPR exists; author has custody rights
        │
        ▼
DHT validator quorum gossip-validates + replicates to neighborhood
        │
        ▼
Post-commit signal → ReconcileController → SQL projection
  (economic_resources table; classified_as column indexed)
        │
        ▼
Events accumulate against the resource (via resource_inventoried_as)
        │
        ▼
Balance derived view:
  SELECT SUM(quantity_delta) FROM economic_events
  WHERE resource_inventoried_as = <resource_id>
        │
        ▼
Application queries: dashboard, shefa views, doorway projection
```

### 4. Physical storage

| Layer | What lives there | Shape |
|---|---|---|
| **Source of truth** | Author's source-chain entry + DHT shard replicas | Holochain DHT; `EconomicResource` entry in `content_store_integrity` |
| **Operational copy** | SQL projection on every peer with reach | `economic_resources` table; `classified_as` column indexed; `lifecycle_state` column (`active` / `subordinate` / `shelved` / `closed`) |
| **Balance derived view** | Materialized by query over `economic_events` | SQL view `resource_balance_view`; no stored balance field |
| **Web2 projection** | Optional doorway-service cache | Redis-shape cache keyed by resource CID; serves unconnected web clients |
| **Large attachments** | Bytes referenced by `tracking_identifier` CID | iroh-blob; pull-fetched on demand; never gossiped |

### 5. Gossip / sync layer

- **DHT**: `EconomicResource` entry payload, typically 2–5 KB (metadata + `classified_as_json` + optional `tracking_identifier` CID reference + timestamps). Gossip latency 200–2000 ms. The DHT carries the *declaration* of the resource, not the bytes and not the balance.
- **libp2p sync plane**: SQL projection rows sync via cursor-tracked delta-sync. Balance views re-derive on the receiving peer once their `economic_events` table is current. A peer that is one sync cycle behind may show a stale balance — this is expected; the substrate commitment is convergence, not immediate consistency.
- **iroh-blob plane**: For content resources (photos, documents, quilts), actual bytes are pull-fetched on demand using the CID from `tracking_identifier` or `content_node_id`. Bytes never gossip; only the DHT entry (the manifest) gossips.
- **Inventory gossip (libp2p)**: `peer_blob_inventory` projections gossip via the `elohim/inventory/blob` topic, carrying which peers hold which resource CIDs at which tier. This is the operational discovery plane for resource availability — `project_inventory_exchange_not_byte_replication` applies — not for resource truth.

### 6. Provenance — maintained vs intentionally degraded

**Maintained cryptographically forever:**
- Author signature on the DHT entry (who declared this resource, when)
- The resource's CID (content-addressed; immutable identity)
- The chain of `EconomicEvent` entries that have acted upon this resource (each carries `resource_inventoried_as` back to this id; the chain is append-only and gossip-validated)
- The `primary_accountable` agent at each historical point (agent key is non-repudiable)

**Intentionally degraded (access cost, not truth):**
- **At subordination** (`parent_epr_cid` set): the resource does not independently gossip its balance view; queryable only via traversal from the parent EPR's reach scope — one extra network hop vs. direct query
- **At cold-archive** (`lifecycle_state: shelved`, Commitment with `action: "custody-quilt"` and `tier_floor: "shelved"`): payload erasure-coded across quilt peers via Reed-Solomon; CID still verifiable, but drawing the bytes requires K-of-N pull from quilt peers (multi-second latency)
- **At dissolution** (`lifecycle_state: closed`): future Events targeting this resource are rejected by integrity validators; existing Event history remains queryable forever; the resource cannot receive new value flows, but its accounting chain is cryptographically permanent
- **Right-to-be-forgotten**: mishpat governance may trigger root-rewrite on the `EconomicResource` entry, stripping PII content fields while preserving the structural accounting chain; downstream Event references retain `redaction-applied` markers

The substrate's commitment: **the CID is forever; the cost of retrieving the bytes scales with lifecycle stage.** A dissolved photo resource's CID proves the photo existed; drawing the actual image requires quilt reconstruction. This cost-shedding is what makes 8B-scale inventory possible.

### 7. Agentic intelligence at scale

Where elohim cognition is load-bearing:

- **Inventory narration** — the `inventory-elohim` agent continuously evaluates whether resources are still actively flowing or should subordinate/shelve. A household's ~10k physical objects (furniture, appliances, tools, vehicles) each have a Resource; no human can actively maintain 10k entries × 100M households. Inventory-elohim does.
- **Balance materialization at care-economy frequency** — recognition tokens, care-tokens, and time-tokens accumulate via Events at a rate humans cannot manually reconcile. The `care-stewardship-elohim` materializes the running balance and narrates context: "14 care-hours contributed this week, 23% above your household average."
- **Classification enforcement** — when a bridge (e.g., `bridges/plaid/`) imports legacy financial data as `EconomicEvent` rows, the `stewardship-elohim` classifies resulting `EconomicResource` entries via `resource_classified_as` (e.g., `["currency", "financial-asset"]`), ensuring the substrate sees the household's full economic picture without manual tagging.
- **Dissolution narration** — when a resource becomes inactive (no Events for N days, below threshold balance), the domain-elohim authors `Event(action="dispose")` to transition `lifecycle_state` to `closed`, notifies linked EPRs, and authors the custody Commitment routing the quilt to cold-archive. Humans can confirm or override.

What humans alone can't do at care-economy scale: continuously maintain resource state for millions of low-value household items and care-economy tokens. Elohim agents narrate the mundane. **This is the value-prop unlock — an economy that can see the nurse's 3 AM compassion as a Resource with an accounting history.**

### 8. Scale: household → hub → global

- **Local DHT (household elohim-node)**: a typical household holds ~1k–10k Resources across all classifications (physical inventory, recognition tokens, financial assets, compute credits, content). At 3 KB per Resource DHT entry, that is 3–30 MB of DHT writes. The balance views derive from `economic_events`; with ~100k events/year per household, balance queries run in milliseconds on local SQLite.
- **Hub (collective elohim-node)**: the hub does not replicate individual household Resources unless they elect commons reach. Hub-level Resources are collective-level entities: shared assets, pool contributions, collective compute credits. Hub balance queries aggregate by joining `economic_events` projected from federated household nodes — the hub computes collective balances without storing member Resources.
- **Global**: only Resources with `reach: commons` (commons-attested) propagate to the DHT's global scope — protocol-level resources such as learning-points standards, recognition-score schemas, compute-credit specifications. Personal inventory, financial assets, and care tokens never reach global scope.

### 9. Limit-awareness / capture prevention

- DHT validator quorum prevents any single peer from falsifying resource creation or custodian assignment
- `resource_classified_as` discrimination gates which classifications can be authored under which agent types — compute-class resources cannot be re-labelled as care-class resources by fiat; they ride separate `signal_kind` streams (see `project_compute_commitments_bounded`)
- Care-class and compute-class Resource balances are maintained in isolated accounting streams: compute breach signals cannot contaminate care attribution, and care debits cannot gate compute placement — this is a substrate-invariant wired through `signal_kind` and `resource_classified_as` whitelists, not a convention
- `primary_accountable` is a stewardship record, not an ownership claim; accumulating disproportionate resource stewardship triggers rising friction via standing-curve flattening (friction-gradient limitarianism)
- Dissolution is an integrity-layer constraint — a closed Resource cannot receive new value flows without a mishpat governance `action: "revive"` requiring quorum consent; the integrity zome validates this at write time

### 10. Network resilience

- DHT shard-N redundancy: each `EconomicResource` entry replicated to ~10 neighborhood peers; single-peer loss does not affect resource accessibility
- Partition recovery: cursor-tracked libp2p sync ensures Event history (and therefore balance correctness) converges after a partition heals; a peer offline for 30 days re-derives the correct balance after catching up its `economic_events` projection
- Cold-archive recoverability: quilts holding resource bytes use Reed-Solomon erasure coding; any K-of-N peers can reconstruct the full payload — even `shelved` resources are recoverable if K honest quilt peers are reachable
- Doorway projection: for web2 / browser clients without a Holochain peer, doorway serves the `economic-resource-view.schema.json` wire shape from its Redis-shape cache; balance fields are omitted or served as cached snapshots with a declared staleness bound
- CID continuity: a Resource that moves through `active` → `subordinate` → `shelved` → `surfaced` retains its CID at every stage; re-elevation via `surface_resource(resource_cid, new_parent_epr_cid)` authors a new Event and updates `lifecycle_state` via ReconcileController but does not create a new DHT entry — the existing entry's CID is the permanent identity

### 11. Dashboard worked example (preview)

In the Google Photos substrate-native media library (`applications/google-photos-application-design.md`), every photo has a companion Resource with `resource_classified_as: ["backup-state"]` tracking how many copies exist across the peer mesh and cold archive. The vision-elohim dashboard renders a per-album backup health row:

```sql
-- Per-album backup health: photo count and bytes at each lifecycle tier
SELECT
    e.parent_epr_cid        AS album_cid,
    r.lifecycle_state,
    COUNT(*)                AS photo_count,
    SUM(ev.resource_quantity_value) AS total_bytes
FROM economic_resources r
JOIN epr_atoms e ON e.resource_id = r.id
LEFT JOIN economic_events ev ON ev.resource_inventoried_as = r.id
    AND ev.action = 'custody-quilt'
WHERE r.classified_as LIKE '%backup-state%'
GROUP BY e.parent_epr_cid, r.lifecycle_state;
```

For each shared album, the dashboard shows how many photos are `active` (locally warm), `shelved` (cold-archive quilt), or `closed` (disposed). The Resource's CID is the stable key that survives photo re-uploads and re-archiving cycles without identity collision. The byte-count balance derives from quilt Commitment Event history — no stored field, always current.

Full walk in `applications/google-photos-application-design.md`.

## A.4 Observation

> **Canonical spec**: `genesis/docs/content/elohim-protocol/architecture/2026-05-11-observation-event-layer-design.md`. This section is a digested citation — the 11 sub-sections are answered concisely, pointing to the canonical for depth. Amend the canonical and update this citation; do not re-derive the design here.

### 1. What it is

Per the canonical Observation spec (§3), an Observation is ephemeral peer-witnessed evidence on the Track 2 substrate data plane — never a DHT entry. A single `Observation` struct (`observer_cid`, `log_cid`, `log_offset`, `observation_kind`, `subject_cid`, `payload_json`, inline diversity tags, `signature`) carries all raw evidence: doorway heartbeats, card-swipes, content-views, mastery-check results, blob-served ticks. It lives in an observer-controlled iroh-blob append-only log; cursor announcements gossip via libp2p gossipsub; subscribed receivers pull segments via ALPN. Individual Observations are designed to dissolve — their life-span ends when their retention class expires or their evidence has been cited in a graduated artifact on the DHT. The records lifecycle uses Observations as the feed that graduation evaluators crystallize into Events and Attestations, making care-economy authorship tractable at human scale.

### 2. Hyperscale analog

Think Splunk's structured log stream plus Kafka's topic-subscription model, with S3-shaped content-addressed append-only log storage behind it — but ephemeral by design and graduated by policy rather than retained forever. Like Splunk, Observations have `observation_kind` namespaces, retention classes, and aggregation queries. Like Kafka, each `kind_namespace` is a topic; peers subscribe by role. Unlike either: there is no central log server — each observer owns their iroh-blob log; gossip is cursor-only (~200 bytes); bytes mobilize via pull-fetch. The surveillance capitalism anti-pattern is structural: the log cannot be read by anyone not subscribed to the gossip topic, and reach gates both subscription and graduation. See canonical spec §5 for the complete wire protocol and §7.2 for the retention-class table.

### 3. Data flow

```
Observer (human, elohim, or bridge-stewardship-elohim) signs and appends row
    to local iroh-blob log (BLAKE3-chunked; root advances to new log_cid)
        │
        ▼
Cursor announcement gossiped (~200 bytes) via libp2p gossipsub
    topic: elohim/observations/<kind_namespace>
        │
        ▼ (peers subscribed to the topic per role policy)
Subscribed peer pull-fetches new log segment via iroh-blob / libp2p ALPN
    verifies BLAKE3 integrity + per-row observer signature
    projects rows to local `observations` table
        │
        ▼
Graduation evaluator (per-pillar tokio task) polls `observation_diversity_summary`
        │
        ├── Path 1: diversity threshold met →
        │       issues Attestation (Content + content_type: "attestation:<subtype>") on DHT
        │       populates metadata.evidence_json.observation_refs with iroh:// tuples
        │
        └── Path 2: summarize policy →
                issues summary EconomicEvent on DHT
                (one entry replaces O(1000) raw observations; observation_refs attached)

Application queries read graduated Events / Attestations.
Audit-replay re-fetches iroh-blob segments via observation_refs.
```

See canonical spec §5 (three substrate moves per observation) and §8 (graduation paths) for per-step detail.

### 4. Physical storage

| Layer | What lives there | Shape |
|---|---|---|
| **Source of truth** | Observer's iroh-blob append-only log; `log_cid` is the BLAKE3 Merkle root; advances per append | iroh-blob store (local to observer node) |
| **Operational copy** | SQL `observations` table on subscribed peers | PK: `(observer_cid, log_cid, log_offset)`; indexes on `(subject_cid, observation_kind, observed_at)` and `(observer_cid, seq)`; see canonical spec §4.4 |
| **Cursor index** | `observation_cursors` table per `(observer_cid, viewer_peer_id)` | SQL — `last_projected_offset`, `last_seen_at`; drives partition recovery |
| **Diversity summary** | `observation_diversity_summary` materialized view | SQL — `COUNT(DISTINCT observer_household_cid)` etc. per `(subject_cid, observation_kind)`; see canonical spec §6.2 |
| **Web2 projection** | Doorway SSR serves diversity summaries for civic legibility (Track 4) | Redis-shape query cache; reach-bounded by subject EPR's scope |
| **Graduated artifacts** | Events and Attestations that cite observations | DHT (notarized) — separate primitives (A.2 and A.6); Observation primitive has no DHT footprint of its own |

`FeedbackSignal` is the documented edge case that lands on DHT for reach-coupling; raw Observations never do. See canonical spec §3 (the architectural cut table) and records-lifecycle §2 (the primitives table).

### 5. Gossip / sync layer

Per canonical spec §5.1 and §5.2: three moves per Observation — (1) append to local iroh-log, (2) gossip cursor ~200 bytes, (3) pull-fetch segment on demand. The cursor announcement carries `observer_cid`, `kind`, `log_cid`, `latest_offset`, optional `subject_cid`, and a time window. Receivers pull only when they care; bytes do not fan out to all subscribers.

- **Cursor gossip**: ~200 bytes per tick; high-frequency-tolerable under gossipsub flow control
- **Segment fetch**: delta only — `(last_projected_offset .. latest_offset)`; BLAKE3-verified; O(KB) per sync cycle per observer per subscriber
- **Backpressure**: receivers self-prioritize by retention class — `wisdom` keeps flowing; pure `operational` degrades first (canonical spec §5.6)
- **DHT write pressure**: zero — Observations do not consume the ~3000 entry/peer DHT budget; only graduated artifacts do

### 6. Provenance — maintained vs intentionally degraded

**Maintained cryptographically forever:**
- Per-row observer signature (every row signed; auditor can re-verify at any time via iroh-blob fetch)
- BLAKE3-chunked iroh-log integrity (`log_cid` is the Merkle root; chunks independently verifiable)
- `iroh://<observer_cid>@<log_cid>#<offset>` reference tuples cited in Attestation `evidence_json.observation_refs` — the durable audit-replay anchor
- Inline diversity dimensions recorded at write (`observer_household_cid`, `observer_collective_cid`, `observer_region`, `observer_archetype`, `observer_compute_class`)

**Intentionally degraded (access cost, not truth):**
- `operational` class: iroh-log pruned after graduation window closes; 7-day SQL hot; 90-day SQL summarized; raw payload gone
- `contextual` class: 30-day SQL hot; log retained; SQL trimmed at consolidation event
- `attestation-feeding` rows: retained until the citing Attestation is itself superseded or redacted
- Right-to-be-forgotten: iroh-log publishes a redacted root (root-rewrite via mishpat governance); downstream Attestations carry `redaction-applied` in `metadata.revocation`; the decision is itself a public `attestation:forget-decision` (canonical spec §9.4)

The substrate's commitment is truth-verifiability, not free-access-forever. The `iroh://` audit reference is forever; cost of re-verification scales with retention class.

### 7. Agentic intelligence at scale

Per canonical spec §8 (graduation paths) and the observer-protocol's "ephemeral witness in service of flourishing":

The graduation evaluator (per-pillar tokio task inside elohim-storage) is elohim cognition that is genuinely load-bearing — humans cannot synthesize 1000:1 evidence ratios. The `infrastructure:blob-served` graduation example (canonical spec §8.2) replaces 1,247 raw Observations with one summary EconomicEvent. For personal finance, `shefa:card-swipe` observations graduate to transfer Events 1:1 (high-signal evidence); for community infrastructure, `infrastructure:doorway-heartbeat` observations graduate to `attestation:doorway-health` only when 3+ households across 2+ regions concur.

**Stewardship-elohim** authors bridge Observations (Plaid card-swipe, Stripe commerce) under its signature so households get automated evidence without raw data leaving the node. **Care-stewardship-elohim** authors `shefa:caregiving-hour` observations from household rhythms — graduation evaluators crystallize these into REA Events that make care work legible to the household's economic picture. Without this elohim layer, the care economy remains invisible to the substrate; with it, **the protocol can scale love and care.**

### 8. Scale: household → hub → global

Per canonical spec §5.3 (subscription matrix) and §7.2 (retention classes):

- **Local DHT (household elohim-node)**: holds SQL projections of subscribed `observation_kind` namespaces relevant to its custodial role. Typical footprint: ~50k rows × ~500 bytes ≈ ~25 MB hot SQL, pruned to ~5 MB after 7-day `operational` window. Zero DHT entries — the Observation primitive does not consume the ~3000 entry/peer budget.
- **Hub (collective elohim-node)**: subscribes to `infrastructure` + pillar namespaces across its membership. Holds diversity summaries across member households; does NOT replicate raw member Observations — per `project_node_metrics_vs_hub_aggregation_boundary`, per-node metrics stay per-node; hub aggregates. Runs the cross-household graduation evaluators (doorway-health attestations requiring 3+ households) that cannot be satisfied from a single household's observations.
- **Global**: Observations never reach the global DHT. Only graduated artifacts (Attestations, summary Events) federate at commons-attested reach — with `iroh://` audit-replay paths attached, not raw payloads. An Ophanim peer (high-diversity volunteer witness hub per observer-protocol Part II) subscribes to all `kind_namespace` topics to raise diversity threshold satisfaction; it does not centralize data.

### 9. Limit-awareness / capture prevention

Per canonical spec §9.1 (what observers cannot do) and the `observer-protocol.md` constitutional safeguards:

- **No privileged read path**: observation consumers are equal peers; subscriptions governed by `peer_transport_manifest` role policy; no single actor has a special endpoint for all observations
- **Physical privacy switches** (observer-protocol Part I): the substrate has no path to compel observation — an observer not running emits no cursors; absence is detectable but content is not inferrable
- **Reach as earned at kind level**: `observation_kind` reach is manifest-declared; manifest amendments go through governance (DHT-notarized); no operator can unilaterally elevate an `agent-private` kind to `commons`
- **Diversity thresholds prevent monoculture capture**: graduation requires distinct households, collectives, and regions; a single actor controlling many agent keys cannot satisfy the household-diversity threshold (canonical spec §6.1 anti-Sybil weighting)
- **Forget as governed, not silent**: right-to-be-forgotten flows through mishpat governance; the decision is itself a public Attestation explaining what was done and why (canonical spec §9.4)

### 10. Network resilience

Per canonical spec §5.5 (cursor tracking) and §5.7 (partition handling):

- **Partition recovery**: on reconnect, a peer reads its `observation_cursors`, polls gossip for current announcements, computes the delta, and pulls iroh-segments — no special recovery protocol; the cursor model handles it deterministically
- **Observer-log redundancy**: the iroh-blob log is content-addressed; any peer that fetched a segment before pruning retains a verifiable copy; graduation evaluators hold `attestation-feeding` rows until the citing Attestation is issued
- **Doorway projection**: Track 4 doorway serves `observation_diversity_summary` views for civic legibility per the Public Observer epic — the unconnected (browser, no P2P peer) reads aggregated summaries without access to raw observations
- **Gossip backpressure**: under sustained load, gossipsub flow control limits cursor frequency; the iroh-log is the buffer; receivers can lag and catch up without causality breaks
- **DHT independence**: the Observation plane is fully decoupled from DHT health — a DHT partition halts only the graduation of new Attestations and summary Events, not ongoing observation collection

### 11. Dashboard worked example

In `applications/meta-facebook-application-design.md`, posts receive engagement — views, reactions, comments. Before becoming social moves (`FeedbackSignal`), these are `imagodei:content-viewed` Observations. The feed-ranking query uses the diversity summary rather than raw engagement counts:

```sql
-- Feed ranking: reach-breadth over engagement-optimization
SELECT
    e.id                          AS post_epr_cid,
    e.content                     AS post_preview,
    ods.distinct_households       AS social_reach_breadth,
    ods.distinct_collectives      AS collective_breadth,
    COUNT(fs.id)                  AS feedback_signal_count
FROM eprs e
LEFT JOIN observation_diversity_summary ods
    ON ods.subject_cid = e.id
    AND ods.observation_kind = 'imagodei:content-viewed'
LEFT JOIN feedback_signals fs
    ON fs.target_cid = e.id
WHERE e.content_type = 'post'
  AND e.reach IN ('community', 'commons')
ORDER BY ods.distinct_households DESC, feedback_signal_count DESC
LIMIT 50;
```

`distinct_households` is organic reach-breadth — how many distinct households found this worth engaging with, not predicted-attention. Raw view counts are `operational`-class and prune after 7 days; what persists is the diversity summary and `FeedbackSignal` DHT entries that earned reach-coupling. Per `meta-facebook-application-design.md`, the feed ranks by standing + recency + signal-density, not by predicted-attention — this is how the substrate delivers Facebook-shape without engagement-optimization extraction.

For the mint-monarch archetype: grandma's coffee-shop transaction begins as a `shefa:card-swipe` Observation on the libp2p plane, graduates to a `transfer` Event on the DHT within the graduation window, and renders in the Monarch dashboard as a SQL row — zero DHT footprint for the raw observation, one ~2 KB DHT entry for the graduated Event, and an `iroh://` audit trail anchoring both. See `applications/mint-monarch-application-design.md` §"How one transaction flows" for the step-by-step.

## A.5 Commitment

### 1. What it is

A Commitment is the substrate's **promise primitive** — a notarized pledge of future economic activity. Implemented as the `Commitment` entry type in the elohim DNA's `content_store_integrity` zome (`content_store_integrity:1342`). It holds the action verb (`action: String`), the parties (`provider`, `receiver`), the resource being promised (`resource_classified_as_json`, `resource_quantity_value`), and the timing (`has_point_in_time`, `has_beginning`, `has_end`, `due`). State advances through six canonical values defined in `COMMITMENT_STATES`: `proposed → accepted → in-progress → fulfilled → cancelled → breached`. It is linked into the fulfillment graph via `EventFulfillsCommitment` DHT links (EconomicEvent action_hash → Commitment action_hash) and scoped into bilateral agreements via `clause_of: Option<String>` (Agreement.id).

Commitments are **dual-role**: simultaneously a **planning primitive** (a scheduled future Event that hasn't fired yet) and a **custody primitive** (the structural author of cold-archive stewardship — specifically `action: "custody-quilt"` with `tier_floor` embedded in `resource_classified_as_json`). The same entry type carries both roles, distinguished entirely by the `action` verb. This duality is architecturally load-bearing: planning Commitments and custody Commitments share the same validation path, gossip path, and SQL projection table. The design consequence is intentional — a stewardship promise and a patronage promise are structurally equivalent.

### 2. Hyperscale analog

**Think Spring-Batch scheduled job record fused with an AWS Reserved Instance lease — both on a shared DHT notary.** Like a Spring-Batch job definition, a planning Commitment (`action: "subscribe"`, `action: "provide-compute"`) describes future work, a timing window, and records the execution result (the fulfilling Event). Like a Reserved Instance lease, a custody Commitment (`action: "custody-quilt"`) locks a steward to a storage tier-floor for a specific CID over a time window, with breach-penalty attestation semantics. Neither analogy is sufficient alone — the fusion is what makes Commitments load-bearing across both the economics plane (Patreon recurring patronage, household budgets) and the infrastructure plane (quilt cold-archive stewardship). This is Stripe Subscription records + AWS Reserved Instance contracts collapsed into one DHT-notarized primitive.

### 3. Data flow

```
Author (human, shefa-elohim, or TierController) signs Commitment entry
        │
        ▼
Holochain coordinator zome validates:
  action verb ∈ REA_ACTIONS
  + state ∈ COMMITMENT_STATES
  + required fields populated for this action role
        │
        ▼
DHT validator quorum gossip-validates (O(log N) neighborhood)
  → notarized: Commitment hash is unforgeable, party-bound
        │
        ▼
Post-commit signal → ReconcileController → SQL projection
  (rea_commitments table indexed by provider/receiver/action/state/due)
        │
        ├── Planning path: shefa-elohim timer task polls
        │   WHERE state='accepted' AND due <= now()
        │   → fires fulfilling EconomicEvent
        │   → creates EventFulfillsCommitment DHT link
        │   → updates Commitment state to 'fulfilled' or 'in-progress' (recurring)
        │
        └── Custody path: TierController reads
            WHERE action='custody-quilt' AND state='accepted'
            → enforces tier_floor from resource_classified_as_json
            → on breach: BreachScanner authors tier-breach Attestation
            → BreachScanner feeds quilt_tier_state projection
            → application queries (dashboard, AccountingAggregator, shefa standing)
```

### 4. Physical storage

| Layer | What lives there | Shape |
|---|---|---|
| **Source of truth** | Author's source-chain + DHT shard replicas | Holochain DHT |
| **Operational copy** | SQL: `rea_commitments` table | SQLite rows indexed by `provider`, `receiver`, `action`, `state`, `due` |
| **Fulfillment graph** | `EventFulfillsCommitment` DHT links | Link entries: EconomicEvent action_hash → Commitment action_hash |
| **Custody payload** | `resource_classified_as_json` field | JSON: `{cid, tier_floor, shelf_destination, diversity_role}` (tiered-quilt spec §4) |
| **Web2 projection** | Optional doorway SSR read for patron-visible recurring state | Redis-shape row keyed by `commitment_id` |
| **Breach history** | `tier-breach` Attestation entries (`category="storage-stewardship"`) | DHT notarized; reference Commitment hash |

### 5. Gossip / sync layer

- **DHT payload**: 1–3 KB per entry (metadata-dominated; `resource_classified_as_json` carries ~200–500 B for custody payloads). Gossiped once at authoring; state transitions author new link entries on `CommitmentByState` anchors — Holochain immutable source-chain discipline prevents in-place mutation.
- **libp2p sync plane**: `rea_commitments` SQL table syncs as delta projections via cursor-tracked sync. A peer coming online after a state change pulls the delta rather than full DHT traversal. Fulfilling `EconomicEvent` rows arrive as separate delta rows against `economic_events`.
- **iroh-blob plane**: not applicable to Commitment metadata. Custody Commitments reference CIDs that live in iroh-blob, but the Commitment entry is pure metadata — no bytes attached.
- **Rate ceiling**: a household with 50 active patronage subscriptions + a TierController governing 500 quilts emits ~550 Commitment entries total. Well within the ~3000-entry-per-peer DHT budget. At Patreon-scale across the full network, entries distribute across the patron graph — no single peer holds all 1M.

### 6. Provenance — maintained vs intentionally degraded

**Maintained cryptographically forever:**
- Signature chain (author-signed at write; DHT validator quorum at gossip time)
- Content-address (Commitment CID)
- `EventFulfillsCommitment` link traversal to all fulfilling Events — the complete payment and fulfillment history is permanent
- Breach attestations: `tier-breach` Attestations reference the Commitment hash; a steward who breached a custody Commitment carries that record permanently in the attestation graph
- Agreement clause membership: `clause_of` field binds the Commitment to its bilateral Agreement; both survive independently

**Intentionally degraded (access cost, not truth):**
- **Cancelled Commitments**: state transitions to `cancelled`; excluded from active SQL views by default but queryable forever as audit history
- **Fulfilled recurring Commitments**: once a subscription Commitment fulfills and a new one is authored for the next cycle, the prior transitions to `fulfilled` and moves to history view — same DHT permanence, lower query priority
- **Cold-archive custody Commitments for dissolved content**: when content reaches `closed` lifecycle state, its custody Commitment transitions to `fulfilled` or `cancelled`; the CID reference becomes a quilt archive pointer, recoverable via K-of-N

The substrate's commitment is to **truth-verifiability, not free-access-forever.** A breach from three years ago is not rewritten — it remains in the attestation record. The CID is forever; the cost of retrieval scales with lifecycle stage.

### 7. Agentic intelligence at scale

Where elohim cognition is load-bearing:

- **Shefa-elohim (scheduler)**: fires planned fulfillment Events on time, retries gracefully, detects stuck cursors (bridge observation graduation halted — e.g., Stripe is down), escalates to patron or creator with actionable context. Without this, every recurring patronage payment requires human confirmation — unworkable at Patreon-shape scale.
- **TierController + BreachScanner (custody elohim)**: reads the custody Commitment floor from `rea_commitments`, compares to observed tier via holdings-attestation probes, authors `tier-breach` Attestation when a steward falls below their floor. No human can monitor 500-quilt commitments per household across a 100M-household network.
- **Stewardship-elohim (authoring)**: determines when a content artifact crosses the cold-archive threshold (reach-decline + age + storage-budget pressure), authors the `custody-quilt` Commitment on behalf of the household, selects appropriate stewards via affinity and prior-fulfillment records. The household never sees this unless the elohim surfaces an ambient notification.
- **Care-class isolation invariant**: custody Commitments (`action: "custody-quilt"`) are compute-class; patronage Commitments (`action: "subscribe"`) are care-class. Shefa-elohim discriminates via `signal_kind` and `resource_classified_as` whitelists. A custody breach signal must never debit patronage standing, and patronage fulfillment attribution must never gate compute placement decisions. This is a substrate-invariant wired through `signal_kind` discrimination — not an ad-hoc field convention.

**This is the value-prop unlock**: the promise economy (recurring patronage, long-term stewardship) is tractable only because elohim agents fire, monitor, and repair Commitments at machine-speed. Human-only operational overhead consumes the value before it reaches the intended recipient.

### 8. Scale: household → hub → global

- **Local DHT (household elohim-node)**: active Commitments per household — ~50 patronage subscriptions + ~10–20 budget Commitments + ~100–500 custody-quilt Commitments if the household is a quilt steward. At ~2 KB each: ~1–2 MB SQL projection. Comfortably within the ~3000 DHT entry budget per peer. Patronage Commitments authored by patrons live in the *patron's* DHT neighborhood — the creator's node queries them via reach scope rather than replicating all entries.
- **Hub (collective elohim-node)**: aggregates fulfillment views via federated SQL — "what is our collective's total committed patronage this month?" — without replicating individual member Commitments. The hub holds collective-level Commitments: a joint stewardship agreement for a shared quilt (family photo archive spanning cities), a shared capacity lease for a collective compute node.
- **Global**: Commitment entries at network scale are bounded by the agent-reach graph. A Commitment is visible only to its parties and their elohim agents — not all 8B participants. Custody Commitments for commons-reach content may propagate via `CommitmentByReceiver` anchor links scoped to the content's reach boundary, so any potential steward can verify the floor before accepting.

### 9. Limit-awareness / capture prevention

- **DHT validator quorum** prevents a single peer from falsifying a Commitment — forging a stewardship pledge they didn't author or backdating a fulfillment
- **`clause_of` Agreement scoping**: bilateral Agreements are co-authored and co-signed; a Commitment exceeding its Agreement's authorization fails integrity validation; neither party can unilaterally overcommit
- **Custody-floor enforcement is peer-local, not hub-controlled**: the `TierController` reads each peer's own Commitments and enforces them locally — honest under partition, honest under hub-failure; no central orchestrator can dictate what floor every peer must hold
- **Breach → Attestation → shefa standing**: a steward who breaches custody Commitments accumulates `tier-breach` Attestations; future placement decisions factor these in via standing-curve pressure — friction-gradient limitarianism applied to storage; a chronic non-deliverer becomes expensive to use as a steward, not globally banned
- **Subscription concentration**: a creator with 1M patrons has 1M `subscribe` Commitments spread across the patron graph; the creator's node receives the projection via reach-scoped sync — it does not hold all 1M DHT entries

### 10. Network resilience

- **DHT shard-N redundancy**: each Commitment entry replicates to ~10 neighborhood peers; survives single-node failures
- **Partition recovery**: cursor-tracked libp2p sync heals `rea_commitments` projection gaps on reconnect; a patron offline when a state change occurred gets the delta on next sync
- **Custody breach under partition**: the BreachScanner uses an observation-gap first pass (holdings-attestation gap must exceed the manifest-declared breach window, archetype-tuned: longer for mobile household nodes, shorter for wired steward nodes) before issuing a breach Attestation — temporary partition is not breach
- **Fulfillment under offline conditions**: the shefa-elohim scheduler is peer-local — recurring Events accumulate on the source chain and gossip when connectivity restores; the protocol does not depend on doorway availability to honor Commitments
- **Cold-archive K-of-N recoverability**: a custody Commitment with `tier_floor=shelved` maps to the quilt's K-of-N erasure-coding guarantee (4-of-7 minimum recovery); if 3 of 7 stewards breach, the 4 survivors reconstruct; breach Attestations from failed stewards inform future diversified placement

### 11. Dashboard worked example (preview)

**Patreon archetype — patron and creator dashboard**

The patron's "recurring giving" tile reads from local SQL — zero network at render:

```sql
SELECT
  c.id                         AS commitment_id,
  c.receiver                   AS creator_agent,
  c.resource_quantity_value    AS monthly_amount,
  c.resource_quantity_unit     AS unit,
  c.due                        AS next_due,
  c.state                      AS commitment_state,
  COUNT(ee.id)                 AS payments_made
FROM rea_commitments c
LEFT JOIN economic_events ee
  ON ee.id IN (
    SELECT event_id FROM commitment_fulfillment
    WHERE commitment_id = c.id
  )
WHERE c.provider = :patron_agent_id
  AND c.action   = 'subscribe'
  AND c.state NOT IN ('cancelled', 'breached')
GROUP BY c.id
ORDER BY c.due ASC;
```

`SUM(monthly_amount)` = total monthly outflow from the patron. Creator view inverts: `receiver = :creator_agent_id` gives the MRR chart Patreon used to own.

**Cold-archive stewardship dashboard** — per-quilt custody status for a steward node operator:

```sql
SELECT
  json_extract(rc.resource_classified_as_json, '$.cid')        AS quilt_cid,
  json_extract(rc.resource_classified_as_json, '$.tier_floor') AS committed_floor,
  rc.resource_quantity_value                                    AS committed_bytes,
  qts.observed_tier,
  CASE WHEN qts.observed_tier < committed_floor
       THEN 'AT RISK' ELSE 'OK' END                            AS status
FROM rea_commitments rc
LEFT JOIN quilt_tier_state qts
  ON qts.cid = json_extract(rc.resource_classified_as_json, '$.cid')
WHERE rc.action   = 'custody-quilt'
  AND rc.provider = :this_agent_id
  AND rc.state    = 'accepted';
```

Both queries read entirely from local SQL projection — zero network at render time. Full patron lifecycle walk-through (subscription → payment → tier-change → cancel) lives in [`applications/patreon-application-design.md`](./applications/patreon-application-design.md). Capacity-declaration Commitments (`action: "provide-compute"`, `action: "request-compute"`) are the supply side of [`applications/aws-compute-application-design.md`](./applications/aws-compute-application-design.md).

---

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

> *[A.7 written below — stub removed]*

### 1. What it is

A FeedbackSignal is the substrate's **social nervous system** — a notarized record of one agent's social act toward a piece of content or another signal. Implemented as a `FeedbackSignal` entry type in the `content_store_integrity` zome, it carries: the `target_cid` of the EPR being acted on, a `signal_kind` discriminator (whitelisted from `SIGNAL_KINDS`), an optional `vouch_kind` sub-discriminator, an optional `evidence_cid` for corrections, a `standing_impact` classification, and a `signer_pubkey` (raw ed25519 bytes, canonically verified by the coordinator before `create_entry`). The entry is **immutable after creation** — `validate_update_entry` rejects all updates; retraction is its own new signal. What makes FeedbackSignal architecturally exceptional among the eight primitives is its **documented edge case status**: it is the only social-move surface that earns DHT-tier cost. Every other social vocabulary extension (likes, endorsements, comments, reactions) would cost nothing if they lived off-DHT — but they cannot, because standing-curve mechanics and reach-coupling require authoring-time notarization to be unforgeable. This does not violate the "no new DHT entry types" principle: `FeedbackSignal` is an existing entry type; `signal_kind` extensibility means new social moves land as whitelist additions, never as new entry types.

### 2. Hyperscale analog

**Think Stripe webhook events filtered by the receiver's credit score, with a public audit log that both the sender and the network can cite as evidence.** Like a Stripe webhook, a FeedbackSignal fires when an agent acts. Like a filtered webhook, the signal only propagates to peers within the signer's reach scope — delivery is gated by earned standing, not by subscription. Unlike any webhook system: the signal is content-addressed, DHT-notarized (signer identity is unforgeable), and accumulates into a standing-curve that affects the signer's future reach capacity. A skeptical systems architect would recognize the shape as: **event log with built-in trust-compute gradient**, where each record contributes to its author's permission surface for future records.

### 3. Data flow

```
Agent authors social act (endorse, comment, report, correction, vouch, forget-request)
          │
          ▼
Coordinator zome (T8): verifies ed25519 signature over canonical bytes
          │  cross-entity rules deferred from HDI (project_hdi_no_get_links_in_validators):
          │    retraction signer == original author (must_get_action on target)
          │    correction evidence_cid → existing Correction EPR (DHT lookup)
          │    vouch: no-self-vouch (must_get_valid_record on target signal)
          ▼
create_entry(EntryTypes::FeedbackSignal) → source chain + DHT gossip
          │
          ▼
Gossip neighborhood validates via integrity zome floors:
  Floor 1: signal_kind ∈ SIGNAL_KINDS whitelist
  Floor 2: standing_impact ∈ STANDING_IMPACTS whitelist
  Floor 3: squelch ⇒ standing_impact == advisory
  Floor 4: correction ⇒ evidence_cid is Some
  Floor 5: vouch ⇒ vouch_kind ∈ VOUCH_KINDS; non-vouch ⇒ vouch_kind is None
          │
          ▼
Post-commit signal → ElohimContentDispatcher → FeedbackSignalProjector
          │           (routes by entry type per ReconcileController discipline)
          ▼
Diesel upsert in `feedback_signals` table with `dht_anchor_hash`
          │
          ▼ (parallel)
T1 libp2p EPR-atom plane:  EprFanOutCtx fan-out to predecessor + reach-neighborhood peers
iroh gossip plane:         broadcast to BLAKE3(reach-scope) topic
          │
          ▼
Standing-curve service re-derives signer's standing score from aggregate signals
          │
          ▼
Application queries (feed ranking, reach gating, moderation dashboard)
```

The T1 wire path (`/elohim/epr-atom/2.0.0`, MessagePack via `FeedbackSignal` in `p2p/feedback_signal.rs`) carries signals between peers in real time before DHT notarization completes. T1 is fast and operational; DHT is the unforgeable record. They are complementary, not redundant.

### 4. Physical storage

| Layer | What lives there | Shape |
|---|---|---|
| **Source of truth** | Author's source-chain entry + DHT shard replicas | Holochain DHT (`content_store_integrity` FeedbackSignal entry type) |
| **Operational copy** | SQL projection on peers with reach to the target EPR | SQLite `feedback_signals` table with `dht_anchor_hash` |
| **Real-time transport** | Live signal propagation before DHT confirms | libp2p `/elohim/epr-atom/2.0.0` + iroh gossip plane (MessagePack `FeedbackSignal` from `p2p/feedback_signal.rs`) |
| **Standing aggregate** | Derived standing score per signer | SQLite view re-derived from signal aggregates; never stored independently of signals |
| **Large evidence** | Correction EPR body (claims + citations) | iroh-blob (referenced by `evidence_cid`; the FeedbackSignal entry itself is ~200–400 bytes) |

### 5. Gossip / sync layer

- **DHT**: full FeedbackSignal payload (~200–400 bytes: five primitive fields + 32-byte signer_pubkey); gossip latency 200–2000 ms; per-peer DHT budget shared with EPRs, Events, Attestations (~3000 entries total before degradation)
- **libp2p EPR-atom plane**: fan-out via `EprFanOutCtx` (predecessor peer routing + reach-neighborhood flood); T22 fan-out path activates when `fan_out_ctx` is wired in `AppState`; enables real-time signal delivery before DHT settles
- **iroh gossip plane**: broadcast to BLAKE3 topic mapped from target EPR's reach scope; peers in the same reach-cluster receive the signal without polling
- **Evidence pull**: Correction EPR bodies are never gossiped — pull-fetched via iroh-blob on demand when a peer evaluates epistemic weight

Rates: a Meta-shaped application at 500 interactions/user/day generates ~100 KB/day DHT writes per active user (~500 signals × 200 bytes). At the ~3000-entry DHT budget, a single user's footprint pressures limits after ~six months of heavy use. The standing-curve addresses this by making high-standing peers carry proportional cost and by enabling signal-stream subordination to aggregates.

### 6. Provenance — maintained vs intentionally degraded

**Maintained cryptographically forever:**
- Ed25519 signature over canonical bytes (`targetCid || signalKind || evidenceCid? || standingImpact || signedBy`) — signer cannot deny authorship
- `signer_pubkey` raw bytes on DHT — unforgeable agent identity
- `target_cid` content-addresses the specific EPR version the signal evaluated — permanently couples signal to subject
- DHT validator quorum attestation at write time — neighborhood confirmed the signal met floor rules
- `dht_anchor_hash` in SQL projection links every operational query to the notarized truth

**Intentionally degraded (access cost, not truth):**
- **Squelch signals are locally private**: Floor 3 (`squelch ⇒ standing_impact == advisory`) is enforced at the DHT layer; squelch entries are authored under agent scope, never propagating to the target's reach neighborhood — truth preserved, effect is local
- **Cold-archive path for signal-dense content**: high-volume signal streams (a post with 50k endorsements) can be subordinated to a Signal-Aggregate Commitment after standing-curve crystallization; individual signals become shelved under the aggregate, recoverable via K-of-N quilt pull
- **Standing score is derived, not stored**: if underlying signals are archived, re-derivation requires a quilt pull — retrieval cost scales with history depth, truth intact

The substrate's commitment is to truth-verifiability, not free-access-forever. The signer's identity and target CID are forever; the standing-score's real-time accessibility scales with lifecycle stage.

### 7. Agentic intelligence at scale

Where elohim cognition is load-bearing:

- **Standing-curve stewardship** (`standing-stewardship-elohim`): continuously re-derives standing scores from signal aggregates, detects Sybil-shaped vouch clusters and standing-farm patterns, proposes adjustments to mishpat governance. Humans cannot monitor 8B signal streams for coordinated manipulation.
- **Correction routing** (`epistemic-elohim`): when a `correction` signal arrives with `evidence_cid` pointing to a Correction EPR, evaluates the evidence chain (observation_refs, citation quality, vouching signers) and advises the mishpat pipeline on whether the correction merits `debit-firm` escalation.
- **Reach-mutation recommendations**: after standing-curve re-derivation, proposes `grant-reach` or `revoke-reach` Events on affected content EPRs (Gap 10 in Part D), closing the loop between social feedback and content visibility.
- **Forget-request routing** (`rights-stewardship-elohim`): receives `signal_kind: "forget-request"` (declared in the `elohim` pillar manifest under `signalKinds`), evaluates against the subject's standing and mishpat constitutional constraints, and authors or withholds the `attestation:forget-decision` EPR.

What humans alone cannot do at care-economy scale: evaluate 500 signals/user/day across hundreds of millions of users for manipulation patterns, evidence quality, and appropriate reach-mutation recommendations. **This is the value-prop unlock**: a social network where standing is earned and manipulation is expensive — not because a platform enforces it centrally, but because the substrate's social nervous system is elohim-mediated at scale.

### 8. Scale: household → hub → global

- **Local DHT (household elohim-node)**: signal footprint = signals authored + signals on content in reach scope. At 500 signals/day, a household node holds ~180k signals/year in SQL — manageable. DHT budget pressure emerges after ~six months of heavy use; standing-curve cost distribution and aggregate subordination are the release valves.
- **Hub (collective elohim-node)**: holds signals for content in the collective's reach scope. A 200-member qahal with active discussion (~50 signals/member/day) generates ~10k signals/day; the hub's SQL projection is the aggregation surface for collective-level standing curves and community moderation dashboards. Hub does NOT replicate every member's personal signals — only signals in collective reach scope.
- **Global**: only signals whose `target_cid` is a commons-attested EPR federate to the global DHT neighborhood. Personal post endorsements stay in the poster's reach cluster; commons-level corrections federate globally. This scope-gating prevents billions of likes from overwhelming the global DHT.

The scaling discipline: `signal_kind` extensibility means new social vocabulary never spawns new DHT entry types. New `react-emoji`, `tag-person`, `bookmark` kinds all land as SIGNAL_KINDS whitelist additions + manifest declarations. The DNA entry count is precious; the social vocabulary is open.

### 9. Limit-awareness / capture prevention

- **Whitelist-gated vocabulary**: `SIGNAL_KINDS` and `STANDING_IMPACTS` constants in `content_store_integrity/src/feedback_signal.rs` are the floor; a new kind requires protocol-schema amendment + whitelist update + manifest declaration — three independent gates against arbitrary vocabulary proliferation
- **Squelch is advisory-only**: Floor 3 prevents squelch from being weaponized as a standing-debit instrument; squelch is steward discretion, not a weapon
- **Correction requires evidence**: Floor 4 prevents bare-assertion corrections from reaching DHT; coordinated false-correction attacks must produce entire chains of fake Correction EPRs, each facing validator quorum
- **No-self-vouch** (coordinator-level; deferred from HDI because `must_get_valid_record` requires HDK): prevents standing-farm-by-self-endorsement
- **Standing-curve friction-gradient**: agents who author many `debit-firm` quarantines face standing depletion if signals are subsequently retracted or vouched-against, making coordinated quarantine attacks self-limiting — friction-gradient limitarianism embedded in signal economics
- **Care-class / compute-class isolation**: `standing_impact` values are strictly care-class primitives — they affect content standing and reach, never compute-tier breach signals; the `signal_kind` whitelist is the enforcement surface for this invariant (`project_compute_commitments_bounded`)

### 10. Network resilience

- **DHT shard redundancy**: every FeedbackSignal entry is replicated to ~10 neighborhood peers; cannot be silently deleted by any single peer
- **T1 wire redundancy**: `EprFanOutCtx` fan-out (predecessor peer + reach-topic gossip) ensures signals authored during a DHT write delay still reach operational projections on neighboring peers before DHT confirms
- **Partition recovery**: cursor-tracked libp2p sync on the EPR-atom plane ensures signals authored during a partition re-propagate on reconnect; `dht_anchor_hash` lets the SQL projection detect and fill gaps
- **Cold-archive path for signal-dense streams**: high-signal EPRs can have historical signal streams subordinated under Signal-Aggregate Commitments (custody-quilt, tier_floor=shelved); individual signals are K-of-N recoverable from quilt
- **Doorway projection**: web2 consumers receive accumulated standing scores and signal-density counts as read-optimized SQL views; individual signals are not replicated to doorway — only their aggregate effect on the standing curve

### 11. Dashboard worked example (preview)

In the Meta / Facebook substrate-native application (`applications/meta-facebook-application-design.md`), the FeedbackSignal is the primary social-move primitive across six surfaces simultaneously:

| Social act | FeedbackSignal shape |
|---|---|
| Like / endorse on a post | `signal_kind: "endorse"`, `standing_impact: "advisory"` |
| Comment on a post | `signal_kind: "comment"`, `standing_impact: "advisory"` — comment body is a child Post EPR; the FeedbackSignal links them |
| Reaction (😂, 🔥, ❤️) | `signal_kind: "react"`, reaction-type in metadata, `standing_impact: "advisory"` |
| Report a post | `signal_kind: "report"`, `standing_impact: "debit-soft"` — escalates to qahal governance |
| Moderation quarantine | `signal_kind: "quarantine"`, `standing_impact: "debit-firm"` — requires mishpat/qahal authorization |
| Vouch for a correction | `signal_kind: "vouch"`, `vouch_kind: "accept-correction"` — standing recovery for the corrected author |

The feed-ranking query for a user's home feed:

```sql
-- Ranked post feed: posts in reach scope, ordered by earned standing + signal density
SELECT
    e.id,
    e.title,
    e.created_at,
    COUNT(DISTINCT fs.id)            AS signal_density,
    SUM(CASE WHEN fs.standing_impact = 'debit-firm' THEN -3
             WHEN fs.standing_impact = 'debit-soft' THEN -1
             ELSE 1 END)             AS net_standing_weight,
    ss.standing_score                AS author_standing
FROM content e
JOIN standing_scores ss ON ss.agent_pubkey = e.author_pubkey
LEFT JOIN feedback_signals fs ON fs.target_cid = e.cid
    AND fs.signal_kind IN ('endorse', 'react', 'vouch')
WHERE e.content_type = 'post'
  AND e.reach_scope IN ('community', 'commons')
  AND e.app_id = ?
GROUP BY e.id
ORDER BY (author_standing * 0.4 + net_standing_weight * 0.4 + signal_density * 0.2) DESC,
         e.created_at DESC
LIMIT 100;
```

Ranking is by standing, not predicted engagement — the substrate's feed is anti-extractive by construction. Full composition walk in `applications/meta-facebook-application-design.md`. The Patreon archetype (`applications/patreon-application-design.md`) uses FeedbackSignals for patron-creator social acts within tier-gated communities; the R&O archetype (`applications/requests-offers-application-design.md`) uses FeedbackSignal `correction` and `vouch` signals to power the offer-quality reputation curve that makes cooperative commerce self-policing.

## A.8 Links (graph edges)

### 1. What it is

A Link is a directed edge in the DHT graph — a Holochain `create_link` call that records a typed pointer from a source hash to a target entry hash, with an optional tag payload. Links are **not entries**: they carry no independent identity, they consume no EntryTypes budget, and their validator quorum is faster than entry creation because validators only check the edge relationship, not a payload schema. The `content_store_integrity` zome today holds **225 link type variants** in its `LinkTypes` enum (a `u8` discriminant — hard ceiling 256). Because links are cheap per instance, they are the connective tissue that makes EPRs, Events, Resources, Attestations, FeedbackSignals, and Commitments into a queryable graph rather than a collection of isolated notary records. Every DHT entry in the substrate becomes a node; every `create_link` call is a traversable edge. This spec adds two new link types via Part D.1 — `EprToEvent` and `EprToResource` — whose full specification lives there; this section explains the Links primitive holistically.

### 2. Hyperscale analog

**Think GraphQL edges with DHT notarization and a u8-typed discriminant.** In GraphQL, a field resolver traverses a typed edge: `Account → [Transaction]` is resolved by following a typed connection from one node to related nodes. DHT links carry the same shape — a typed edge from source hash to target hash, traversable by anyone with access to either endpoint. Unlike a GraphQL resolver (which hits a single master database), each DHT link is gossiped to a neighborhood of peers: the link is the proof that the connection exists, not just a query result. Unlike a foreign key (which assumes a master database), a DHT link survives peer churn because it is replicated across DHT neighbors. Unlike a relational join (which scans a table), DHT traversal follows content-addressed hashes — identity travels with bytes, not a server-vended row ID. The "cheap, unbudgeted" framing is precise: links cost one gossip write and consume zero EntryTypes budget. At 225 link types defined and a hard ceiling of 256, the **type budget** is not free in the long run, but each individual link **instance** is still far cheaper to author than an entry instance.

### 3. Data flow

```
Author calls create_link(source_hash, target_hash, LinkType, tag)
        │
        ▼
DHT validator quorum: link-type validation (O(log N) neighborhood)
    - Is source_hash a valid anchor for this LinkType?
    - Is the author permitted to create this edge?
    - Is the tag payload well-formed?
        │
        ▼
Link replicates to neighborhood peers (smaller payload than entries)
        │
        ▼
post_commit signal → ReconcileController projects edge into SQL adjacency table
        │
        ▼
SQL adjacency row written (e.g., epr_event_edges, attestation_subject_edges)
        │
        ▼
Application queries traverse graph at zero network cost
    e.g., SELECT events.* FROM economic_events e
          JOIN epr_event_edges ee ON ee.event_id = e.id
          WHERE ee.epr_id = $account_epr_id
          ORDER BY e.has_point_in_time DESC;
```

After projection, traversal is pure SQL — **zero DHT calls at query time** for data within the local peer's reach scope.

### 4. Physical storage

| Layer | What lives there | Shape |
|---|---|---|
| **Source of truth** | Author's source-chain link record + DHT link neighborhood replicas | Holochain DHT `create_link` action |
| **Operational copy** | SQL adjacency tables projected from post-commit signals | SQLite `*_edges` tables (e.g., `epr_event_edges`, `epr_resource_edges`, `attestation_subject_edges`) |
| **Web2 projection** | Doorway SQL-over-HTTP query for unconnected clients | Same adjacency tables mirrored in doorway SQL projection |
| **Tag payloads** | Small tag bytes in the DHT link record itself | ≤ 256 bytes typical; large evidence lives in iroh-blob, referenced by CID in the tag |

The SQL adjacency projection is what enables graph queries at near-zero latency after initial sync. A fresh peer pulls DHT links via the libp2p sync plane and projects them into adjacency tables; from that point, graph traversal is local SQL with no network round-trips for reach-scope data.

### 5. Gossip / sync layer

- **DHT**: link payload ≈ 32 bytes source hash + 32 bytes target hash + 1 byte link type discriminant + optional tag (0–256 bytes typical). Gossip latency 200–2000 ms. **Faster validation than entries** — validators inspect the relationship semantics, not a field-by-field payload schema.
- **libp2p sync plane**: link projections delta-sync to SQL adjacency tables via the same cursor model as entry projections — link-create signals project as edge-upserts; link-delete signals project as edge-removes. Sync rate matches the entry plane (Phase 11 SyncManagerBackend).
- **iroh-blob**: links themselves never transit iroh-blob. A link tag may carry the CID of a blob (e.g., `AttestationToSubject` tag carrying `evidence_cid`), but blob bytes move independently over the iroh plane.
- **Practical traversal cost post-projection**: any graph query against the local SQL is O(1) indexed lookup — the gossip cost was paid once at link-creation time.

### 6. Provenance — maintained vs intentionally degraded

**Maintained cryptographically forever:**
- Author signature on the `create_link` action (unforgeable who created this edge and when)
- Source and target hashes (both endpoints are CID-pinned at link-creation time; the relationship is content-addressed)
- DHT validator quorum attestation (neighborhood peers witnessed edge creation)
- Link type discriminant (structural semantics — `AttestationToSubject` vs `EventFulfillsCommitment` — are part of the notarized record)
- Tag payload, if present (signed with the link; carries relationship-specific metadata)

**Intentionally degraded (access cost, not truth):**
- **When source EPR is subordinated**: links from the EPR don't independently gossip; traversal costs one extra hop through the parent's reach scope
- **When source EPR is shelved**: the DHT link persists, but K-of-N quilt pull applies if the target entry is also cold-archived
- **When source EPR is closed**: existing links remain queryable; validation rejects new link creation targeting the closed EPR
- **Right-to-be-forgotten**: mishpat-governed link deletion removes an edge from new traversals; a redaction-applied marker preserves the structural provenance of the former relationship per the authorship-floor constitutional commitment

The substrate's commitment is to **truth-verifiability, not free traversal forever.** A notarized edge cannot be falsified; the cost of traversing it scales with the lifecycle stage of its endpoints.

### 7. Agentic intelligence at scale

Where elohim cognition is load-bearing:

- **Link authoring at care-economy frequency**: inventory-elohim, vehicle-elohim, care-stewardship-elohim author `EprToResource` and `EprToEvent` links that bind narrated events to their parent EPRs. Humans author at event-horizon scale (major purchases, life events); elohim authors the continuous connective tissue between them, building the traversable record of lived household experience.
- **Link-type triage at the 256-cap**: when a new application pattern proposes a new link type, the governance-elohim (or rust-architect) evaluates: is this a genuinely structural DHT relationship or a query-time filter that belongs in SQL? The LINK_ARCHITECTURE.md triage rule — "if it exists only for queries, use projection" — becomes elohim-assisted at fleet scale. The governance-elohim can surface which link types have no validators depending on them and are projection-candidates for deprecation, reclaiming budget for new structural edges.
- **Social graph traversal (Meta archetype)**: `RelationshipBySource`, `RelationshipByTarget`, and `RelationshipPendingConsent` links are created by the social-elohim when consent is mutual. Friends-of-friends traversal federates through reach-attested projections — each peer's local SQL holds their immediate graph; second-degree traversal is a federated query to neighboring nodes, not full-graph replication.
- **Attestation chain traversal**: `AttestationToSubject` and `GovernanceActionChild` links are elohim-traversed to construct evidence chains for standing computation. Standing-elohim walks these edges to derive reach scores without human navigation at scale.

**This is the value-prop unlock**: links are the edges that let elohim agents navigate substrate-native graphs at care-economy frequency, binding every household narration into a traversable record of lived experience.

### 8. Scale: household → hub → global

- **Local DHT (household elohim-node)**: a typical household has ~1k EPRs. With Gap 1 closed, each EPR accumulates ~10 Events and ~5 Resources over its lifetime — roughly 15k `EprToEvent`/`EprToResource` edge projections. Add ~5k `AttestationToSubject` and social-graph edges. Total household link projection: ~20k SQL adjacency rows ≈ 1–2 MB on disk. This is the in-reach-scope graph.
- **Hub (collective elohim-node)**: holds collective-level EPR edges (shared assets, joint commitments) plus `AttestationToSubject` and `FeedbackSignal` links for member contributions. Does not replicate member household link graphs unless members elect commons-reach. Hub link set: ~50k rows for a medium collective (200 members × ~250 reach-elected links each).
- **Global**: DHT replicates each link to ~10 peers. At 8B households × 20k links each, the aggregate DHT link volume is enormous — but each peer holds only its **reach-scope slice**, not the whole graph. Commons-reach links (public content, commons-attested resources) are the only links gossipped beyond reach scope. The link graph is naturally partitioned by reach scope; no single peer sees the whole graph.

### 9. Limit-awareness / capture prevention

- **u8 discriminant ceiling (256 types, 225 currently used)**: the 31 remaining link type slots are precious. Every new link type proposal must pass LINK_ARCHITECTURE.md triage: genuinely structural relationship vs. projection-candidate. The anti-pattern is "add a link type for each new query pattern" — that path exhausts the budget against a hard wall. The LINK_ARCHITECTURE.md explicitly lists ~50 `*By{Attribute}` variants as deprecation candidates; retiring those reclaims headroom for structural edges like `EprToEvent` and `EprToResource`.
- **Validator quorum gates edge creation**: a link from source A to target B requires the creating agent to have structural standing to create that relationship. `AttestationToSubject` validators check issuer identity. `EventFulfillsCommitment` validators check event/commitment correlation. This prevents link-flood attacks where a malicious peer creates millions of spurious edges to overwhelm a link neighborhood.
- **Reach-scoped traversal prevents graph harvesting**: query-time traversal is gated by local SQL reach scope. An attacker cannot request "give me all edges from all EPRs" — they see only the reach-scoped projection their standing permits.
- **Link deletion is governance-mediated for structural relationships**: mishpat consent flows gate edge removal for accountability chains (REA fulfillment, attestation subjects). Ephemeral edges can be revoked by their author.
- **No self-vouchable social edges**: the no-self-vouch invariant from FeedbackSignal extends structurally — `RelationshipWithCustody` links require mutual consent; agents cannot create structural custody-edges to themselves.

### 10. Network resilience

- **DHT shard-N redundancy**: each link replicates to ~10 neighborhood peers; recoverable from any one honest peer.
- **Partition recovery**: links authored during a partition sit on the author's source chain; they gossip to the DHT and project into SQL adjacency tables once connectivity resumes. The libp2p sync plane's cursor model handles link-create backfill across the partition gap.
- **Cold-archive survival**: links survive even when their target entries are shelved to the quilt. The link is the navigation handle that enables surface re-elevation; without the DHT link, re-elevation would require a full DHT scan by hash rather than a link-traversal lookup.
- **Doorway projection for unconnected clients**: doorway mirrors the SQL adjacency tables. Browser clients traverse graph results via REST without running a Holochain conductor; link provenance (author + quorum) is surfaced in responses for clients that need to verify.

### 11. Dashboard worked example

**Monarch/Mint personal-finance dashboard — account→event→resource traversal:**

The household opens their finance dashboard. The shefa pillar renders current balances, recent transactions, and category spending. Each view is a graph traversal over link-projected adjacency tables:

```sql
-- Account EPR → its Events (requires EprToEvent link type per Gap 1 / Part D.1)
SELECT e.id, e.action, e.resource_quantity_value, e.has_point_in_time
FROM economic_events e
JOIN epr_event_edges ee ON ee.event_id = e.id
WHERE ee.epr_id = $account_epr_id
  AND e.has_point_in_time >= date('now', '-30 days')
ORDER BY e.has_point_in_time DESC;

-- Account EPR → its current Resources (balances and holdings)
SELECT r.id, r.name, r.accounting_quantity_value, r.accounting_quantity_unit
FROM economic_resources r
JOIN epr_resource_edges er ON er.resource_id = r.id
WHERE er.epr_id = $account_epr_id;
```

Both queries are pure SQL after sync — zero DHT calls at render time. The `epr_event_edges` and `epr_resource_edges` adjacency tables are projected from `EprToEvent` and `EprToResource` link-create signals via the ReconcileController. Full composition in `applications/mint-monarch-application-design.md`.

**Meta/Facebook social-graph traversal**: `RelationshipBySource` and `RelationshipByTarget` links enable friend-graph queries powering the feed. Each user's local SQL holds their immediate connection graph; second-degree traversal federates via reach-attested peer projections. Full pattern in `applications/meta-facebook-application-design.md`.

**Google Drive folder→document traversal**: `parent_epr_cid` on a document EPR is a declarative field encoding parentage; the `EprToResource` link (or analogous folder-content edge) is what projects into a SQL adjacency table and enables efficient `SELECT * WHERE parent_epr_cid = $folder_id` queries at render time. Drive-shape folder trees emerge from EPR link hierarchies. Full pattern in `applications/google-drive-application-design.md`.

The architectural distinction is this: **`parent_epr_cid` is a content-addressed field that declares a relationship; the `EprToEvent`/`EprToResource` links are the gossiped edges that project that relationship into traversable SQL adjacency tables.** Both are necessary; neither alone is sufficient. Fields encode; links enable traversal.

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

# Part C — Composability Stress-Test

> *Deferred to development-sprint measurement per operator direction.* The composability stress-test ("what happens when ONE household's elohim-node participates simultaneously in all eight active application archetypes?") is a question whose answer falls out of real measurement, not authored fiction. The scenarios below are placeholders pointing to the dev-work surfaces that will populate Part C with measured numbers from the alpha cluster.

**Stress-test scenarios that will populate Part C (measured during development sprints, not authored here):**

1. **Per-peer working-set footprint at full-stack participation.** One household elohim-node running concurrently: Mint/Monarch personal finance + Khan-Academy learning trajectory + Google-Drive document collaboration + Google-Photos media library + Meta/Facebook social graph + Patreon creator monetization + Requests & Offers cooperative commerce + AWS-shape compute participation. Target measurement: SQL projection size, iroh-blob working set, cold-archive residue. Expected order of magnitude: ~150 GB operational with cold archive elsewhere.

2. **DHT entry visibility per peer scope.** Number of entries the household-peer maintains visibility on under typical reach scoping. Expected: ~100k entries within reach (per the back-of-envelope §1.1 sketch). Measured against real network traffic on the alpha cluster.

3. **Libp2p sync-plane bandwidth profile.** Monthly bandwidth per household across all active application participation. Expected: <200 MB/month aggregate. Measured against alpha-cluster real traffic.

4. **Observation rate at care-economy frequency.** Local-only Observation rate (per the 2026-05-11 observation spec retention model) before graduation thresholds fire. Expected: ~1k/day local. Critical that 99% of observations never touch DHT.

5. **Graduation-evaluator throughput at hub scale.** Per kind_namespace shard (per D.6 D-1a), measure events-per-second throughput. Validates the D.6 architecture decisions.

6. **Standing-curve recompute latency at moderation-active hub.** With D.18 signal_class isolation + D.14 60s staleness SLA, measure recompute time under moderation-heavy load (many quarantine/vouch cycles).

7. **Recurring Commitment fulfillment under thundering-herd.** D.17 stagger discipline validated against 1M-patron first-of-month simulation: verify the BLAKE3-distributed delay actually flattens the receive-side rate to ~278/sec.

8. **Link adjacency query latency under 500+ subordinates.** Per A.8 concern: a household-inventory EPR with 500+ subordinate Resources, render-time query latency for the "my stuff" view. D.1 bifurcated projection (Diesel for one-shot) measured against expected sub-100ms render.

9. **DHT entry budget under sustained social velocity.** D.12 aggregate-subordinate release-valve validated: at 500 signals/user/day across a peer's reach scope, verify DHT entry count stays bounded by the aggregate-subordination cadence.

10. **8B-user back-of-envelope verification.** The §1.1 math (100M households × per-household profile = planet-scale storage with cold tier as the planet-scale layer) validated against measurement projections from alpha-cluster scaling tests.

The development-sprint measurement plan that closes Part C is a follow-up artifact, tracked separately from this spec. When measurement data exists, Part C transitions from placeholders to authored content. *Households subsume hyperscale datacenters* is the claim; the alpha-cluster numbers are the proof.

---

# Part D — Lifecycle wiring (the twenty substrate gaps)

> Each gap has its own subsection with motivation, design (entry types, link types, fields, coordinator functions, validation rules), manifest declarations, migration story, test surface, and **code anchors** specifying the exact files it touches. The spec→code graph is walkable from any gap. **Wave structure (per Phase 2 findings synthesis):** Wave A is the unblocker (D.10 schema-first IoC governance, D.11 substrate-floor validator backfill, D.13 missing view schemas, D.5 observation spec implementation prerequisite); Wave B is substrate primitives (D.1 subordination, D.4 EconomicResource consolidation, D.12 checkpoint primitive); Wave C is lifecycle operations (D.2 surface, D.3 submerge, D.7 dissolution, D.9 reach-mutation); Wave D is patterns + interop (D.6 elohim-authoring, D.8 bridge pattern, D.15 cross-DNA coordination, D.16 multi-oracle confirmation, D.17 Commitment stagger, D.18 signal_class isolation, D.19 EPR-mediated key recovery, D.20 Layered Commons); Wave E is validator + policy (D.14 standing-curve view).

### D.1 Subordination Architecture (Gaps 1+2: `EprToEvent` / `EprToResource` link types + `parent_epr_cid` field) — Wave B

**Motivation.** Subordinate Events and Resources need a parent EPR so their custody and gossip cost shed under the parent's reach scope. Today, every EconomicEvent and EconomicResource lives at the top level — they're not structurally bound to the EPR whose state they represent. Without subordination:

- A Monarch dashboard rendering "events under this account" requires querying all Events whose `provider` or `receiver` matches the account agent — slow + ambiguous (events touch many agents)
- A household-inventory's couches don't know they belong to the household — every Resource is structurally peer with every other Resource at the same reach scope
- Cold-archive of a parent EPR can't sweep its children efficiently — no link to traverse
- Field-encoded relationships (`provider`, `receiver`) are queryable but not gossiped as edges — projection-lag means a fresh peer sees the field but not the structural binding

The substrate's first-class graph pattern (per `project_first_class_graph_pattern`) treats EPRs as nodes and Couplings/Memberships/Delegations as edges. Subordination needs to be one of those edge types.

**Design — field + link + adjacency table as a triple.** Phase 1 A.8 surfaced that any single one of these is incomplete:

- Field alone: declarative but unqueryable without projection; projection-lag = silent missed subordinates during fresh-peer cold reads
- Link alone: traversable but doesn't survive entry-content rehydration without the SQL projection
- Adjacency table alone: queryable but disconnected from DHT truth — drift risk if the projection isn't synchronized

So all three ship together as the subordination primitive:

**Field on entry struct.**
```rust
// elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs
pub struct EconomicEvent {
    // ... existing fields ...
    pub parent_epr_cid: Option<String>,  // NEW — Option for backward-compatibility
}

pub struct EconomicResource {
    // ... existing fields ...
    pub parent_epr_cid: Option<String>,  // NEW — same shape
}
```

`Option<String>` permits existing parentless entries to continue functioning; new subordinate Events/Resources set the field at creation.

**Link types on DHT.**
```rust
pub enum LinkTypes {
    // ... existing variants ...
    EprToEvent,         // NEW — parent EPR → child Event
    EprToResource,      // RENAMED IN PLACE from ContentToResource (zero callers confirmed)
}
```

The `EprToResource` rename in place resolves the overlap Phase 1 A.8 flagged between the proposed `EprToResource` and the existing `ContentToResource` — same semantic, cleaner name. Caller spot-check confirmed zero call sites of `LinkTypes::ContentToResource` outside the enum declaration; no migration cost.

**SQL adjacency tables (Diesel migration).**
```sql
-- elohim/elohim-storage/migrations/<date>-epr-adjacency-tables/up.sql
CREATE TABLE epr_event_edges (
    parent_epr_cid TEXT NOT NULL,
    child_event_cid TEXT NOT NULL,
    edge_created_at TEXT NOT NULL,
    PRIMARY KEY (parent_epr_cid, child_event_cid)
);
CREATE INDEX idx_epr_event_edges_parent ON epr_event_edges(parent_epr_cid, edge_created_at);

CREATE TABLE epr_resource_edges (
    parent_epr_cid TEXT NOT NULL,
    child_resource_cid TEXT NOT NULL,
    edge_created_at TEXT NOT NULL,
    PRIMARY KEY (parent_epr_cid, child_resource_cid)
);
CREATE INDEX idx_epr_resource_edges_parent ON epr_resource_edges(parent_epr_cid, edge_created_at);
```

The ReconcileController projects `EprToEvent` / `EprToResource` link-create signals into these adjacency rows. This is the **canonical projection target for one-shot parent-child lookups** (per operator decision B-2 bifurcation): Diesel adjacency tables for shallow queries; CozoDB `graph_views/` module for multi-hop graph walks.

**Canonical projection target rules (per B-2 bifurcation):**

| Query shape | Projection target | Rationale |
|---|---|---|
| List children of one parent (1-hop) | Diesel `epr_*_edges` table | Single-index lookup; SQLite-fast |
| Multi-hop traversal (friend-of-friend; ancestor chain) | CozoDB `graph_views/` | Graph-native query; richer composition |
| Aggregation across children (sum balances) | Diesel adjacency + JOIN against `economic_*` tables | Standard SQL pattern |
| Pattern detection (find cycles, dense subgraphs) | CozoDB | Graph-native is the right tool |

D.1 declares the rule in spec; D.6 (elohim-authoring pattern) wires the projector accordingly; the application archetypes (Wave 2) use whichever target matches their query shape.

**Coordinator functions.**
```rust
// elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs

create_event_under_epr(parent_epr_cid: Cid, event_data: EconomicEvent)
  // 1. Validate parent EPR exists and is not in `closed` lifecycle_state (D.7 interlock)
  // 2. Validate authoring agent has Membership in parent's custody scope
  // 3. Create EconomicEvent entry with parent_epr_cid set
  // 4. Create EprToEvent link from parent → event
  // 5. ReconcileController projects into epr_event_edges

create_resource_under_epr(parent_epr_cid: Cid, resource_data: EconomicResource)
  // Same shape; EprToResource link; epr_resource_edges projection
```

**Validation rules (integrity zome).**

- `EprToEvent` / `EprToResource` link creation rejected when parent EPR has `lifecycle_state = "closed"` (D.7 interlock — closed EPRs cannot accept new subordinates)
- Link creation rejected when authoring agent has no Membership in parent's custody scope (configurable per pillar manifest — some pillars allow public child creation under a public-collective parent; defaults to "custody-required")
- Field `parent_epr_cid` validated as a valid Cid format at write time
- `EprToResource` (renamed from `ContentToResource`) inherits any prior validation rules from the renamed variant

**Manifest declaration.** Each pillar manifest declares which `content_type` parent EPRs accept which subordinate Event/Resource shapes:

```jsonc
// elohim/sdk/domains/shefa/manifest.json
{
  "subordination_rules": [
    {
      "parent_content_type": "household",
      "accepts_event_actions": ["transfer", "checkpoint", "close-account"],
      "accepts_resource_classifications": ["currency-USD", "currency-community"]
    },
    {
      "parent_content_type": "household-inventory",
      "accepts_event_actions": ["receive", "transfer", "transform", "dispose"],
      "accepts_resource_classifications": ["furniture", "vehicle", "tool", "digital-media", "stewarded-physical"]
    }
  ]
}
```

D.10's vocabulary governance gate validates that referenced content_types, action verbs, and resource_classifications are all declared elsewhere in the manifest.

**Query patterns** (representative — shapes the application archetypes use):

```sql
-- One-shot: list events under an account (Monarch dashboard)
SELECT e.* FROM economic_events e
JOIN epr_event_edges ed ON ed.child_event_cid = e.cid
WHERE ed.parent_epr_cid = :account_cid
ORDER BY e.observed_at DESC LIMIT 50;

-- One-shot: list resources under household-inventory (Monarch "my stuff")
SELECT r.* FROM economic_resources r
JOIN epr_resource_edges ed ON ed.child_resource_cid = r.cid
WHERE ed.parent_epr_cid = :inventory_cid;
```

```cozo
// Multi-hop: traverse the household-inventory hierarchy for cold archive sweep
?[parent, descendant] :=
    *epr_resource_edges[root, descendant], root = $household_cid;
*epr_resource_edges[descendant, deeper] => ?[descendant, deeper]
```

**Migration.** Pre-launch hard cutover; no shim. Existing parentless Events and Resources keep `parent_epr_cid: None` and continue functioning. New Events and Resources can set the field at creation. ReconcileController backfills `epr_*_edges` tables from existing entries on first run (one-time scan).

**Touches:**
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — add to `LinkTypes` enum (`EprToEvent`); rename `ContentToResource` → `EprToResource` (zero callers); add `parent_epr_cid: Option<String>` field to `EconomicEvent` + `EconomicResource` structs
- `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` — new coordinator functions `create_event_under_epr`, `create_resource_under_epr`
- `elohim/elohim-storage/migrations/<date>-epr-adjacency-tables/up.sql` — **new Diesel migration** creating `epr_event_edges` + `epr_resource_edges` tables with indices on parent_epr_cid
- `elohim/elohim-storage/src/services/reconcile_controller.rs` — project `EprToEvent` / `EprToResource` link-create signals into adjacency tables
- `elohim/elohim-storage/src/graph_views/{shefa,lamad,...}.rs` — CozoDB graph-view builders for multi-hop traversals (per B-2 bifurcation)
- `elohim/elohim-storage/src/views.rs` — extend `EconomicEventView` + `EconomicResourceView` with `parent_epr_cid: Option<String>` field
- `elohim/sdk/schemas/v1/views/economic-event-view.schema.json` — add `parent_epr_cid` field
- `elohim/sdk/schemas/v1/views/economic-resource-view.schema.json` — add `parent_epr_cid` field (this schema authored in D.13)
- `elohim/sdk/domains/*/manifest.json` — `subordination_rules` section per pillar (validated by D.10's gate)
- `elohim/holochain/dna/LINK_ARCHITECTURE.md` — note the `ContentToResource → EprToResource` rename in the link-type history

---

### D.2 Surface (Re-elevation) Operation (Gap 3) — Wave C

**Motivation.** The resell-the-couch case — a Resource that's been demoted to subordinate or shelved needs a path back to active EPR-tier status. Without it, the lifecycle gradient is a one-way trapdoor: every shelved Resource is permanently lost from active flow. The household's economic legibility breaks: "we sold the couch we put away" can't be represented; the protocol effectively erases re-use, repurposing, and gift-economy patterns that are everyday occurrences at household scale.

**Design — event-sourced state machine (C-1 resolution).** The lifecycle state is **never stored on the entry**; it's derived from event history. Holochain entries are immutable; the substrate honors that by tracking state transitions via Events rather than mutating entries.

```rust
// elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs

surface_resource(resource_cid: Cid, new_parent_epr_cid: Option<Cid>, reach_target: Option<Reach>)
  // 1. Read current lifecycle_state from projection (latest of: active | subordinate | shelved | closed)
  //    Validates: state must be in { subordinate, shelved } (closed Resources need revive, not surface)
  // 2. Author Event(action="surface", subject_cid=resource_cid, new_parent=..., new_reach=...)
  // 3. ReconcileController consumes the Event, updates projection's lifecycle_state to "active"
  //    + updates parent_epr_cid in epr_resource_edges (D.1 adjacency table)
  //    + updates reach scope per new_reach (D.9 reach-mutation interlock)
  // 4. The DHT entry's CID is unchanged; identity is preserved across surface
```

The Resource's CID survives across surface because the entry itself doesn't change. What changes is the projection's view of its current state, derived from the accumulated event history. This is event-sourcing all the way: state lives in the event stream, not on the entry.

**C-2 resolution — stake-class-tiered authorship authority.** Authority scales with the reach the surface produces:

| Surface reach | Authority required |
|---|---|
| `household` (kid takes couch from family inventory) | Current custodian's Membership OR a stewardship-elohim under stewardship-commitment Attestation authored by the custodian |
| `community` (list couch for sale within a collective) | Custodian + at least one peer-witness Attestation from another Membership in the receiving community |
| `commons` / `commons-attested` (donate to public commons; sell broadly) | Mishpat-governance attestation chain (M-of-N council quorum per `2026-05-11-attestation-consolidation-design.md` §3.4) |

Validation runs in the integrity zome:

```
validate_surface_event(event, action) {
  let target_reach = event.new_reach.unwrap_or(current_reach_from_projection(event.subject_cid));
  match target_reach {
    Reach::Household | Reach::AgentPrivate => require_custodian_or_stewardship_elohim(),
    Reach::Community | Reach::Collective => require_custodian + peer_witness_attestation(),
    Reach::Commons | Reach::CommonsAttested => require_mishpat_governance_chain(M_of_N),
  }
}
```

The substrate-floor invariant: surface authorship cannot exceed the authority chain its target reach demands. Authority scales with visibility; the higher the reach the surface produces, the more witnesses are required to authorize it.

**Custody transfer.** Surface may include a `new_parent_epr_cid`:
- `None` → Resource becomes "free" (no parent custody); rare; usually only for transition-out-of-substrate cases
- `Some(new_parent_cid)` → custody transfers to the new parent. The `epr_resource_edges` adjacency table (D.1) updates: the prior parent-child edge is closed; the new parent-child edge is created. Both edges remain in event history; only the current-state projection shows the new parent.

**Couch-resell flow (canonical example).**

```
Year 1: household buys couch
   Event(action="receive", provider=furniture-store, receiver=household,
         resource=Couch, parent_epr_cid=household-inventory-cid)
   → Resource Couch created at lifecycle_state="active"

Year 5: couch put in cold archive (used less)
   Commitment(action="custody-quilt", tier_floor="shelved", subject_cid=couch-cid)
   → ReconcileController fans out: lifecycle_state="shelved"; couch bytes
     move to peer-cellar quilt storage

Year 7: kid moves out, wants couch
   Event(action="surface", subject_cid=couch-cid,
         new_parent_epr_cid=kid-household-inventory-cid,
         new_reach="household")
   → Validation: custodian (parent) authors; kid-household-reach is household-scope
     → authority satisfied
   → ReconcileController: lifecycle_state="active"; epr_resource_edges row
     created for kid-household-inventory → couch
   → Couch's CID is the same as Year 1 — full provenance chain preserved
```

The Couch Resource entity's CID is constant across 7 years and three lifecycle states. Its full event history is queryable from any peer that can resolve its CID. Anyone interested in the couch's history can audit-verify the entire chain.

**Manifest declaration.** `action: "surface"` in elohim pillar manifest with stake_class declarations per the reach-tiered authority model:

```jsonc
// elohim/sdk/domains/elohim/manifest.json
{
  "vocabulary_declarations": {
    "action_verbs": [
      {
        "verb": "surface",
        "stake_class_by_target_reach": {
          "household": "high",
          "agent-private": "high",
          "community": "high",
          "collective": "high",
          "commons": "governance-quorum",
          "commons-attested": "governance-quorum"
        },
        "validates_against": ["lifecycle_state in {subordinate, shelved}", "authorship per reach-tier"]
      }
    ]
  }
}
```

D.10's vocabulary governance gate validates the action verb is declared; the integrity zome enforces the per-reach authority model at write time.

**Touches:**
- `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` — new coordinator function `surface_resource`
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — add `"surface"` to `REA_ACTIONS` (per D.10); validation rules enforcing reach-tiered authority
- `elohim/elohim-storage/src/services/reconcile_controller.rs` — consume `Event(action="surface")`, update `lifecycle_state` projection + `epr_resource_edges` adjacency (D.1 interlock) + reach scope (D.9 interlock)
- `elohim/sdk/domains/elohim/manifest.json` — declare `surface` action verb + per-reach stake_class model
- D.1 interlock — surface updates parent_epr_cid in `epr_resource_edges`
- D.7 interlock — closed Resources cannot be surfaced (must `revive` first per D.7)
- D.9 interlock — surface may include `new_reach` parameter; per-reach authority validation matches D.9's reach-mutation Events
- D.20 interlock — surface to `commons` / `commons-attested` reach engages the Global Commons elohim-council attestation chain

---

### D.3 Submerge Canonical Signal Reconciliation (Gap 4) — Wave C

**Motivation.** Phase 1 A.5 surfaced that the substrate has two parallel vocabularies for "moved from active to cold archive": `submerge` (per `2026-05-10-memory-lifecycle-design.md` — for content / context / scenarios) and `quilt-demoted` (per `2026-05-11-tiered-quilt-stewardship-design.md` — for blob storage tier transitions). They describe the same lifecycle move from different angles. Without reconciliation, the substrate has two operations authoring the same substrate state, with drift-risk at every junction.

**Design — single canonical authoring event with downstream projection fan-out.**

The canonical authoring form is a Commitment:

```rust
Commitment {
    action: "custody-quilt",
    subject_cid: <epr_or_resource_cid>,
    resource_classified_as_json: serde_json::json!([
        "custody-shelf",
        {
            "tier_floor": "shelved",
            "shelf_destination": "<URI>",
            "diversity_role": "<optional>",
            "covered_window": { "period_start": ..., "period_end": ... }
        }
    ]).to_string(),
    state: "accepted",
    primary_accountable: <authoring_elohim_or_human>,
    // ... standard Commitment fields ...
}
```

When this Commitment lands on DHT, the ReconcileController fans out:

1. **memory-lifecycle effect**: project as `submerge` lifecycle operation in the memory subsystem (covered entries get `lifecycle_state="shelved"`, queryable but not actively maintained)
2. **tiered-quilt effect**: project as `quilt-demoted` storage-tier transition (bytes move from peer-cellar warm tier to shelved cold-archive destination per `shelf_destination` URI)
3. **records-lifecycle effect**: `lifecycle_state` projection set to `"shelved"` for the subject EPR/Resource (D.7 interlock — closed-state cannot be reached via custody-quilt; closure is `Event(action="dispose")` per D.7)
4. **signal-aggregate interaction (D.12 interlock)**: when paired with `Commitment(action="aggregate-subordinate")` from D.12, this Commitment IS the authority that permits the aggregated signals to move to cold archive

**Both upstream specs gain amendment notes** pointing here:
- `2026-05-10-memory-lifecycle-design.md` — `submerge` is the downstream effect; the authoring event is `Commitment(action="custody-quilt", tier_floor="shelved")` per D.3
- `2026-05-11-tiered-quilt-stewardship-design.md` — `quilt-demoted` is the downstream effect; same canonical authoring event

The two prior vocabularies become aliases for projection effects of one substrate event.

**C-3 resolution — shelf_destination vocabulary expansion.** The original tiered-quilt schema's `shelf_destination` covered only infrastructure URIs (`peer-cellar://household/H`, `external-archive://minio/`). Memory-lifecycle named seven socio-institutional destinations that the schema doesn't cover. Extending the existing enum with URI-scheme namespacing captures both classes in one field:

```
shelf_destination URI schemes:
  Infrastructure (existing):
    peer-cellar://<custody-collective>/<custodian-id>
    external-archive://<external-system>/<bucket>
    quilt://<quilt-network>/<storage-pool>

  Socio-institutional (new — per memory-lifecycle's 7 destinations):
    therapist-collective://<licensed-collective-cid>/<session-cid>
    research-observatory://<observatory-collective-cid>/<study-cid>
    gov-evidence-store://<jurisdiction>/<case-cid>
    cultural-archive://<archive-collective-cid>/<collection>
    lineage-archive://<lineage-collective-cid>/<generation>
    subconscious://<agent-cid>                            (personal subconscious — agent-private)
    peer-cellar://<custody-collective>/<custodian-id>     (alias as community subconscious)
```

Each URI scheme has manifest-declared validation rules: which collective archetypes can be addressed; whose stewardship-commitment is required to author a Commitment with that destination; what reach scope the destination is observable at.

Substrate-floor invariant: a custody-quilt Commitment with `shelf_destination` scheme `therapist-collective://` must be authored by a human with active Membership in a licensed therapist-collective (validated at integrity zome via Membership-attestation chain). Other socio-institutional schemes have analogous integrity constraints.

**Cancellation flow (Phase 1 A.5 addendum).** A custody-quilt Commitment is `accepted` at authoring; cancellation moves to `cancelled` state. The unresolved questions Phase 1 raised:

- **Who can author cancellation?** Either party (the authoring elohim/human OR the custody-receiving collective's steward) can author a `cancel-commitment` Event. Mishpat governance for high-reach Commitments.
- **In-progress fulfillment Events?** If the custody-quilt window is active (covered_window in progress) and a cancellation lands, in-progress signals revert to active DHT state until next custody-quilt Commitment lands.
- **Custody-quilt handoff?** Cancellation of a custody-quilt Commitment optionally triggers a handoff Commitment to another steward, declared via `cancel_handoff: Some(new_steward_cid)`. If `None`, the bytes revert to peer-cellar warm tier and the source CID re-enters active gossip.

**Touches:**
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — `REA_ACTIONS` include `custody-quilt` (canonical) + `cancel-commitment` (cancellation flow); validation rules enforcing per-`shelf_destination`-scheme integrity constraints
- `elohim/elohim-storage/src/services/reconcile_controller.rs` — fan-out projection: project `Commitment(action="custody-quilt", tier_floor=shelved)` into memory-lifecycle submerge + tiered-quilt quilt-demoted + records-lifecycle lifecycle_state + (if D.12 aggregate-subordinate paired) signal-aggregate state update
- `elohim/sdk/schemas/v1/views/commitment-view.schema.json` — extend `shelf_destination` URI-scheme namespace to cover socio-institutional destinations
- `elohim/sdk/domains/{mishpat,imagodei,lamad,shefa}/manifest.json` — declare per-scheme validation rules + which collective archetypes can be addressed under each scheme
- `2026-05-10-memory-lifecycle-design.md` — amendment note: `submerge` is downstream of D.3 canonical authoring event
- `2026-05-11-tiered-quilt-stewardship-design.md` — amendment note: `quilt-demoted` is downstream of D.3 canonical authoring event

### D.4 EconomicResource Consolidation (Gap 5) — Wave B

**Motivation.** The substrate currently has two distinct DHT entry types representing "resource with state": `EconomicResource` (REA canonical) and `StewardedResource` (added later for stewardship-specific tracking). Their fields overlap. REA discipline says one canonical type with classification-based variants — not parallel types. Phase 1 A.3 surfaced that consolidation is structurally clean *if* the migration sequence is right; doing it wrong has clear downstream consequences (capacity-planning, household-resilience, and node-stewardship dashboards all depend on `StewardedResource` fields).

**Design — one canonical type with classification + field additions.** `StewardedResource` retires; its semantic surface folds into `EconomicResource` via two moves: (a) the `resource_classified_as` discrimination carries stewardship variants; (b) two fields that have no clean classification-mapping become first-class on `EconomicResource`.

**Classification mapping (B-3 + B-4 reference field landings below).**

| StewardedResource semantic | Lands as | Notes |
|---|---|---|
| Generic stewarded asset | `resource_classified_as: "stewarded-physical"` | Furniture, vehicles, tools |
| Stewarded digital asset | `resource_classified_as: "stewarded-digital"` | Documents, photos, media |
| Stewarded compute capacity | `resource_classified_as: "stewarded-compute"` | AWS-shape provider capacity declaration |
| Stewarded space | `resource_classified_as: "stewarded-space"` | Storage, square footage |
| Stewarded labor capacity | `resource_classified_as: "stewarded-care-hour"` | Caregiving time available |

**Field additions on `EconomicResource` (B-3 + B-4 resolved per operator lean).**

```rust
// elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs
pub struct EconomicResource {
    // ... existing fields ...
    pub parent_epr_cid: Option<String>,       // from D.1
    pub governed_by: Option<String>,          // NEW — references collective governance handle (B-3)
    pub data_quality: Option<DataQuality>,    // NEW — provenance signal for derived dashboards (B-4)
}

pub enum DataQuality {
    Measured,   // sensor / instrumented source
    Estimated,  // elohim-inferred from observations
    Manual,     // human-entered
    Mixed,      // mixed-provenance accumulated state
}
```

**`governed_by` (B-3 resolution).** `Option<String>` referencing a Collective EPR's CID that holds governance authority over this Resource. When set, allocation Events on this Resource require Membership-attested authorship from within the governed collective. When unset, household-scope custody applies (default). The field preserves the `StewardedResource.governed_by` semantic for shared resources (energy pools, commons compute credits, joint household assets) without spawning a new substrate primitive.

**`data_quality` (B-4 resolution).** `Option<DataQuality>` provenance signal for the current state. Set by the elohim-agent authoring the Resource based on observation_refs of the contributing Events (bridge-authored Events with `observation_refs` → `Measured`; manually entered → `Manual`; elohim-inferred without source observation → `Estimated`; accumulated state across mixed-provenance event history → `Mixed`). Monarch's data-confidence view reads this field directly to gray-out estimated values, badge bridge-authored balances, etc.

**Field migration map (StewardedResource → EconomicResource).**

| StewardedResource field | Lands at | Notes |
|---|---|---|
| `steward_id` | `primary_accountable` (existing) | semantic match |
| `governed_by` | `governed_by` (NEW field B-3) | first-class preservation |
| `data_quality` | `data_quality` (NEW field B-4) | first-class preservation |
| `total_capacity_value` | **derived view** over event history | not stored — derived |
| `total_allocated_value` | **derived view** over allocation Events | not stored — derived |
| `total_used_value` | **derived view** over consumption Events | not stored — derived |
| `available_value` | **derived view** (capacity − allocated − used) | not stored — derived |
| `allocations_json` | **derived view** joining `epr_event_edges` against allocation Events | not stored — replaced by D.1 subordination + adjacency |
| `recent_usage_json` | **derived view** over recent consumption Events with timestamp filter | not stored — derived |
| `trends_json` | **derived view** with rolling window aggregation | not stored — derived |
| `acquisition_event_id` | **derived view** — first receive/produce Event in history | not stored — derived |
| `last_valuation_event_id` | **derived view** — most recent valuation/reclassification Event | not stored — derived |

**Hard-cutover prerequisite — derived views must work before retirement.** Phase 1 A.3 flagged that Gap 5 is a migration landmine if the entry type is retired before the derived views replacing the computed fields are wired. The sequencing:

1. Author derived-view services (`elohim/elohim-storage/src/services/resource_state_service.rs` planned) that produce the same surface the dashboards expected from `StewardedResource`
2. Green-test the derived views against fixture data
3. Migrate existing StewardedResource entries to EconomicResource (one-time Diesel migration with classification set per existing usage)
4. Retire the `StewardedResource` entry type from the integrity zome enum
5. Run `pnpm test` to verify capacity-planning + household-resilience + node-stewardship dashboards still pass

Steps 1-2 are blocking prerequisites; step 4 cannot land before they're green.

**Manifest declaration.** Pillar manifests declare which `resource_classified_as` values their pillar uses; D.10's vocabulary governance gate validates these against the EconomicResource classifications enum.

```jsonc
// elohim/sdk/domains/shefa/manifest.json
{
  "vocabulary_declarations": {
    "resource_classifications": [
      {"classification": "currency-USD", "stewardship_variant": false},
      {"classification": "currency-community", "stewardship_variant": false},
      {"classification": "stewarded-physical", "stewardship_variant": true},
      {"classification": "stewarded-digital", "stewardship_variant": true},
      {"classification": "stewarded-compute", "stewardship_variant": true}
    ]
  }
}
```

**Budget win.** -1 variant from elohim DNA `EntryTypes` enum (StewardedResource retired). The reclaimed slot is available for future structural-type additions, partially offsetting D.1's two new link types and any future entry-type pressure.

**Migration strategy.** Pre-launch hard cutover; no backwards-compat shim. All `StewardedResource` callers (capacity-planning service, household-resilience service, node-stewardship dashboard surfaces, content store coordinator) migrate to `EconomicResource` with appropriate classification + the new fields. Single Diesel migration drops the `stewarded_resources` projection table. One TS codegen pass refreshes types via `@elohim/storage-client`.

**Touches:**
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — retire `StewardedResource` entry type; add `governed_by` + `data_quality` fields to `EconomicResource`; add `DataQuality` enum
- `elohim/elohim-storage/src/services/resource_state_service.rs` — **new service** providing derived views (capacity / allocated / used / available / allocations / recent_usage / trends) — **blocking prerequisite for entry-type retirement**
- `elohim/elohim-storage/src/views.rs` — extend `EconomicResourceView` with `governed_by`, `data_quality`, `parent_epr_cid`; retire `StewardedResourceView`
- `elohim/sdk/schemas/v1/views/economic-resource-view.schema.json` — add new fields (authored in D.13)
- `elohim/elohim-storage/migrations/<date>-retire-stewarded-resource/up.sql` — drop `stewarded_resources` projection table; backfill `economic_resources` from prior `stewarded_resources` rows with classification set per row's usage pattern
- `elohim/sdk/domains/{shefa,lamad,imagodei,qahal}/manifest.json` — declare stewardship-variant `resource_classifications`; validated by D.10's gate
- All callers of `StewardedResource` (capacity-planning, household-resilience, node-stewardship dashboard, content_store coordinator) — migrate to `EconomicResource` with classification

---

### D.5 Observation Spec Implementation Prerequisite (Gap 6) — Wave A prerequisite

**Motivation.** This spec's records-lifecycle gradient depends on the Observation tier as its upstream. Observations are where high-frequency, low-stake evidence lives; graduation policies (Path 1 → Attestation; Path 2 → summary Event) are where Observations crystallize into substrate primitives that records-lifecycle then wires. **Until the Observation tier is implemented end-to-end, the records-lifecycle gradient has no source for its early stages.** Two specific dependencies make this a strict Wave A prerequisite, not a parallel concern:

- **The graduation evaluator (canonical 2026-05-11 spec Stage 5) is the producer for every Event/Attestation that records-lifecycle treats as graduated.** D.6 (elohim-authoring pattern) and D.7 (dissolution semantics) both reference graduation provenance; D.1 (subordination links) needs Events to subordinate; D.4 (Resource consolidation) needs the graduated Events to derive Resource state.
- **Stage 6 retirement of `DoorwayHeartbeat`, `DoorwayHeartbeatSummary`, `HealthAttestation` (infrastructure DNA)** removes the operational-DHT-entries that today muddy the EPR-vs-Observation boundary. Until those go, "what's an EPR vs what's an Observation" remains incoherent in the current substrate. Records-lifecycle Part A.1 (EPR walkthrough) is structurally hand-waved if these heartbeat entries still exist.

**Design.** Records-lifecycle does not re-spec the Observation tier; the canonical work lives in `2026-05-11-observation-event-layer-design.md`. This subsection records the **prerequisite chain** that gates records-lifecycle implementation against observation-spec completion:

**Observation spec stage sequencing (canonical):**

1. **Stage 1**: Manifest declarations (per-pillar `observation_kinds` arrays)
2. **Stage 2**: Wire format + ALPN (libp2p + iroh parity)
3. **Stage 3**: Storage tables (`observations`, `observation_logs`, `observation_cursors`, `observation_diversity_summary` view, `audit_observations`)
4. **Stage 4**: `ObservationManagerBackend` neutral service (cursor-tracked sync; manages subscription matrices per peer role)
5. **Stage 5**: Per-pillar graduation evaluator (tokio task — see D.6 for architecture decision on sharding + rate-ceiling + failover)
6. **Stage 6**: Infrastructure DNA retirement — `DoorwayHeartbeat`, `DoorwayHeartbeatSummary`, `HealthAttestation` removed; their consumers re-route to graduated `attestation:doorway-health` issued by the graduation evaluator
7. **Stage 7**: HTTP API + storage-client surfaces (doorway routes for observation diversity queries)
8. **Stage 8**: Existing-table reclassification (doc-only — `peer_blob_inventory`, `system_metrics`, `projection_events` retro-tag as observation projections)

**Hard-cutover discipline** (per Phase 1 Meta-Pattern 5): Stage 5 graduation evaluator must work end-to-end with green tests **before** Stage 6 retirement commit. Otherwise heartbeat consumers (doorway-health dashboards, infrastructure-attestation issuers, hub-availability gossip) all go dark simultaneously. This is the same migration-landmine pattern that D.4 EconomicResource consolidation has to navigate; both must wire the derived-views-or-replacement-machinery before the retirement.

**Records-lifecycle dependency on Stage 5 specifically.** D.6 (elohim-authoring pattern) operationalizes the graduation evaluator's domain-specialized agents (inventory-elohim narrates `shefa:card-swipe` observations into transfer Events; vision-elohim narrates `lamad:image-captured` observations into auto-tag Attestations; care-stewardship-elohim narrates `imagodei:care-act` observations into care-Events). Without Stage 5 working, D.6 has no graduation surface to specialize.

**Records-lifecycle dependency on Stage 6 specifically.** D.10 (vocabulary governance) catches the broader drift; Stage 6 closes the specific case where the infrastructure DNA still treats operational evidence as DHT entries. Until then, the EPR-as-vessel framing has counterexamples in the substrate (heartbeats look like notarized records but behave like operational evidence).

**Manifest declarations records-lifecycle adds via Stage 1:** the Wave B/C/D gaps add `observation_kinds` declarations across `shefa`, `lamad`, `imagodei`, `qahal` pillars for the application-archetype evidence streams. D.10's vocabulary governance gate validates these as they land.

**Touches:**
- This subsection is a citation of `2026-05-11-observation-event-layer-design.md`; that spec is the canonical implementation plan
- `elohim/elohim-storage/src/services/observation_manager.rs` (planned per Stage 4) — `ObservationManagerBackend` neutral service
- `elohim/elohim-storage/src/services/graduation_evaluator.rs` (planned per Stage 5) — per-pillar graduation tokio task (architecture decisions resolved in D.6)
- `elohim/holochain/dna/infrastructure/zomes/*/src/lib.rs` — retire `DoorwayHeartbeat`, `DoorwayHeartbeatSummary`, `HealthAttestation` (Stage 6)
- `elohim/sdk/domains/infrastructure/manifest.json` — declare observation_kinds for `infrastructure:doorway-heartbeat`, `infrastructure:blob-served`, `infrastructure:system-sample` (Stage 1)
- `elohim/sdk/domains/{shefa,lamad,imagodei,qahal}/manifest.json` — declare per-pillar observation_kinds via Wave B/C/D additions (Stage 1)

### D.6 Elohim-Authoring Pattern — Domain-Specialized Agents (Gap 7) — Wave D

**Motivation.** The records-lifecycle's value-prop unlock — elohim narrating mundane care, inventory, stewardship into REA-shape Events at the everyday-frequency humans won't bear — requires domain-specialized agents. A generic graduation-evaluator can't handle inventory narration (needs visual-recognition of receipts), vehicle stewardship (needs maintenance-cycle awareness), care-economy bookkeeping (needs household-cadence familiarity), etc. Domain specialization is the substrate's mechanism for making elohim cognition load-bearing at the right granularity. **This is "the economy that scales love and care" made operational.**

**Design — domain-elohim as graduation-evaluator specialization.**

Each domain-elohim:
1. Subscribes to specific `observation_kind` namespaces per pillar manifest (e.g., `inventory-elohim` subscribes to `shefa:receipt-scanned`, `shefa:item-disposed`, `lamad:object-recognized`; `care-stewardship-elohim` subscribes to `imagodei:care-act`, `imagodei:meal-prepared`, `imagodei:caregiving-hour`)
2. Runs as a tokio task (or separate worker — see D-1a architecture decision below) inside elohim-storage or as a sidecar service
3. Evaluates graduation policies per its domain (manifest-declared per observation_kind)
4. Authors Events / Attestations / subordination links on behalf of the household under stewardship-commitment Attestation authority
5. Reports to the operator (via Angular UI surface) when judgment is needed beyond the elohim's confidence threshold

**D-1a resolution — sharded by kind_namespace.** Per operator lean: graduation evaluators shard by `kind_namespace` (within a pillar, parallel per observation_kind). Pillar-level serialization is too coarse for hub scale; EPR-level fragmentation is too fine. Kind_namespace is the natural unit of independent work.

```rust
// elohim/elohim-storage/src/services/graduation_evaluator.rs
struct GraduationEvaluator {
    kind_namespace: String,  // e.g., "shefa:card-swipe"
    domain_elohim: DomainElohimAgent,  // the agent specialization
    rate_ceiling: RateCeiling,  // per D-1b
}
```

Each (pillar, kind_namespace) pair gets its own evaluator task. Cross-namespace parallelism is the substrate's natural throughput model.

**D-1b resolution — manifest-declared rate-ceiling per attestation_kind / event_action.** Per operator lean: rate-ceiling is structural, not standing-modulated (standing-curve coupling deferred until standing system is solidly working).

```jsonc
// pillar manifest declaration
{
  "vocabulary_declarations": {
    "attestation_kinds": [
      {
        "kind": "attestation:doorway-health",
        "rate_ceiling": { "max_per_subject_per_day": 24, "max_per_period_hours": 1 }
      }
    ],
    "action_verbs": [
      {
        "verb": "transfer",
        "rate_ceiling": null  // no ceiling on transfer events
      },
      {
        "verb": "served-blob-summary",
        "rate_ceiling": { "max_per_provider_per_period": 1, "period_seconds": 3600 }
      }
    ]
  }
}
```

Substrate-floor enforcement: integrity zome rejects authoring beyond the declared ceiling. Per-pillar manifest amendment process tunes the ceiling over time.

**D-1c resolution — first-quorum-wins hub failover.** Per operator lean: no designated backup hub. When a hub goes offline mid-graduation-window, any hub-in-reach can take over. The substrate's natural P2P model handles this — the first hub to reach quorum on a community-scoped Attestation issues it; subsequent attempts no-op (the integrity zome rejects duplicate Attestations with the same subject_cid + kind + window via CRDT-style dedup).

```
when community-scoped graduation window closes:
  every hub in reach evaluates the observation_diversity_summary
  first one to reach quorum authors the Attestation
  the integrity zome's idempotent-check prevents duplicates
  hubs that lost the race silently no-op
```

This eliminates the single-point-of-failure of designated backup; the substrate's P2P quorum naturally handles failover.

**Domain-elohim specializations (initial set).**

| Agent | Pillar | Watches | Produces |
|---|---|---|---|
| `inventory-elohim` | shefa | `shefa:receipt-scanned`, `shefa:item-disposed`, `shefa:item-acquired` | `Event(action="receive")`, `Resource` creates, subordination links |
| `vehicle-elohim` | shefa | `shefa:vehicle-maintenance-log`, `shefa:vehicle-mileage` | `Event(action="maintain")`, depreciation summary Events |
| `care-stewardship-elohim` | imagodei | `imagodei:care-act`, `imagodei:meal-prepared`, `imagodei:caregiving-hour` | Care-Events with quantities; care-account Resource updates |
| `learning-elohim` | lamad | `lamad:content-viewed`, `lamad:mastery-check-result`, `lamad:reflection-authored` | `Event(action="attempted-quiz")`, mastery Resource updates, Attestation issuance |
| `vision-elohim` | lamad | `lamad:image-captured`, `lamad:video-captured` | `attestation:auto-tag`, `attestation:face-cluster` |
| `compute-stewardship-elohim` | shefa | `shefa:compute-cycle-consumed`, `shefa:compute-cycle-available` | `Event(action="executed-compute")`, compute-Resource state |
| `commons-stewardship-elohim` | elohim | watches commons-tier reach mutations + Global Commons fee Events | Allocation Events from Global Commons; apex-elohim council attestations (D.20 interlock) |
| `bridge-stewardship-elohim` | (per-bridge) | watches bridge-vendor's webhook stream (Plaid, Stripe, etc.) | Bridge-authored Observations under stewardship-commitment Attestation (D.8 interlock) |

Each agent ships its own TypeScript service implementation in `app/elohim-app/src/app/elohim/elohim-agents/` following the existing `compute-operator-elohim` pattern.

**Authority model.** Domain-elohim authors under a `stewardship-commitment` Attestation chain — the household authorizes the agent at setup time; revocation closes the Commitment chain, ending the agent's authority to author on the household's behalf. The substrate-floor invariant: any Event authored by a domain-elohim must have a current-and-valid stewardship-commitment Attestation referenced via `observation_refs` or `evidence_refs`.

**Bridge-stewardship-elohim parallel-author fallback** (Phase 1 A.4 addendum): when a bridge-stewardship-elohim's primary instance is offline, a backup-stewardship-elohim (declared at bridge-vendor's Collective EPR) can continue authoring. Bridge-collective manifests declare the fallback chain.

**Floor-permissive principle (carried from §1.3 of this spec).** Humans CAN author the same Events directly. The graduation evaluator + domain-elohim pattern doesn't replace human authoring — it makes the everyday-frequency narration tractable. ISO/logistics shows humans CAN bear high-flow REA; the value-prop is that elohim make the care-economy-frequency narration feasible without requiring that bookkeeping discipline of every household.

**Touches:**
- `app/elohim-app/src/app/elohim/elohim-agents/` — TypeScript service implementations for the 8 initial domain-elohim specializations (planned per task)
- `elohim/elohim-storage/src/services/graduation_evaluator.rs` — extended with kind_namespace sharding + rate-ceiling enforcement + first-quorum-wins idempotent dedup
- `elohim/sdk/domains/*/manifest.json` — declare which elohim-agent watches which observation_kinds + handles which graduation policies + per-attestation_kind/event_verb rate_ceiling
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — validate stewardship-commitment Attestation chain on domain-elohim-authored Events; idempotent-dedup check for community-scoped Attestations
- D.8 interlock — bridge-stewardship-elohim is the bridge-side specialization
- D.20 interlock — commons-stewardship-elohim is the Global Commons stewardship specialization

---

### D.7 Dissolution Semantics (Gap 8) — Wave C

**Motivation.** When something gets thrown away, that's a substrate event. Without explicit dissolution semantics, the substrate has no clean way to express end-of-life for a Resource or EPR — the lifecycle gradient stops at "shelved" (cold archive) and never reaches "closed" (terminal). This produces three downstream pathologies:

- A disposed couch keeps appearing in current inventory views because no event marks its termination
- Closed bank accounts continue to be queried as live, producing nonsense balance estimates
- BreachScanner flags custody-quilt Commitments against CIDs that no longer have active state, generating false-positive `tier-breach` Attestations

Per operator direction, this subsection closes the **implementation loop** — concrete dissolution mechanics for everyday end-of-life. The broader cradle-to-cradle philosophy (designing the disposition with the original creation in mind; ensuring every birth knows what the end looks like) is a separate design session noted in `defers:` and not within D.7's scope.

**Design — close / revive lifecycle.**

```rust
Event {
    action: "dispose",  // or "close-account" for account-EPRs; "close-organization" for collectives
    subject_cid: <resource_or_epr_cid>,
    provider: <current_custodian>,
    receiver: <terminal_destination>,  // landfill / recycler / void / etc.
    resource_quantity_value: <last_recorded_quantity>,
    metadata_json: serde_json::json!({
        "disposition_kind": "recycled" | "landfill" | "transferred-to-charity" | "sold-on-resell" | "consumed" | "unspecified",
        // disposition_kind is the cradle-to-cradle hook (default "unspecified"; manifest declares upgrade path)
    }).to_string(),
    // ... standard Event fields ...
}
```

When this Event lands, ReconcileController updates the subject's `lifecycle_state` projection to `"closed"`. The Event is permanent record of disposition; the projection's `closed` state is the operational signal that future-Event validation reads.

**Revive (`Event(action="revive")`).** A misfire or accidental disposal can be reversed via `Event(action="revive", subject_cid=...)`. Authority for revive is the same as the original dispose (`provider` of the dispose Event, or mishpat governance). Revive transitions `lifecycle_state` from `closed` back to `active` (or to whichever state the projection's most recent non-disposal Event would have produced). Used for: undoing accidental disposals; restoring a closed account that was wrongly classified.

**Field projection (derived, not stored).**

```rust
pub enum LifecycleState {
    Active,        // current, queryable, accepts new Events
    Subordinate,   // under a parent EPR's custody (D.1); queryable through parent
    Shelved,       // cold-archive per D.3 custody-quilt
    Closed,        // terminal — future Events fail validation unless action=revive
}
```

`lifecycle_state` is **derived** from event history: the most recent Event whose action transitions state (`receive`/`produce` → active; subordination link create → subordinate; `custody-quilt tier_floor=shelved` → shelved; `dispose`/`close-account` → closed; `revive` → reverts to prior state per event history). This is per C-1 event-sourced state machine discipline.

**Substrate-floor validation invariant.** Integrity zome rejects any new Event whose `subject_cid` resolves to a `closed` Resource/EPR projection, unless the new Event's action is `revive`:

```
validate_event(event) {
  let target_state = lifecycle_state_from_projection(event.subject_cid);
  if target_state == Closed && event.action != "revive" {
    return Err("cannot author Event against closed subject");
  }
}
```

This invariant is the substrate-floor enforcement of "closed means closed." A disposed Resource cannot accumulate new state; an old bank account cannot receive new transfers. Future Events that would target the closed CID fail at the integrity layer.

**Custody Commitment lifecycle when CID dissolves (Phase 1 A.5 addendum).** When a subject CID transitions to `closed`, any outstanding `custody-quilt` Commitments referencing that CID need to be cleaned up:

```
ReconcileController.on_dispose_event(event) {
  let closed_cid = event.subject_cid;

  // 1. Find any outstanding custody-quilt Commitments where subject_cid = closed_cid
  let outstanding = query_commitments(action="custody-quilt", subject_cid=closed_cid, state="accepted");

  // 2. Author Event(action="cancel-commitment") for each, with reason="subject-disposed"
  for c in outstanding {
    author_event(action="cancel-commitment", subject_cid=c.cid, reason="subject-disposed");
  }

  // 3. BreachScanner now skips closed-CID Commitments (lifecycle_state filter in its query)
  //    — no false-positive tier-breach Attestations on dissolved content
}
```

The interlock between D.7 (dissolution), D.3 (custody-quilt authoring), and the BreachScanner (tiered-quilt enforcement) is bidirectional: dissolution cleans up outstanding Commitments; BreachScanner respects closed-state when iterating its watch list.

**Cradle-to-cradle hook (deferred to later design session).** The `disposition_kind` field on the dispose Event carries the early hook for future cradle-to-cradle accounting (`recycled` flows into recycling-credit Attestations; `transferred-to-charity` flows into community-benefit Resource state; etc.). Defaults to `"unspecified"`; pillar manifests declare valid disposition_kinds per pillar. This is the minimal close-the-loop while preserving forward-compat for the broader cradle-to-cradle work.

**Manifest declaration.** `dispose`, `close-account`, `close-organization`, `revive` action verbs in elohim pillar manifest:

```jsonc
{
  "vocabulary_declarations": {
    "action_verbs": [
      {"verb": "dispose",          "stake_class": "high", "stake_class_by_target_reach": {"household": "high", "community": "governance-quorum"}},
      {"verb": "close-account",    "stake_class": "high"},
      {"verb": "close-organization", "stake_class": "governance-quorum"},
      {"verb": "revive",           "stake_class": "high", "authority": "matches prior dispose authority"}
    ]
  }
}
```

D.10's vocabulary governance gate validates these declarations; the integrity zome enforces the per-action authority model.

**Touches:**
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — `REA_ACTIONS` add `dispose`, `close-account`, `close-organization`, `revive`; validation invariant: closed Resources/EPRs reject new Events except `revive`
- `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` — coordinator functions handle the dispose/close/revive flows
- `elohim/elohim-storage/src/services/reconcile_controller.rs` — derive `lifecycle_state` from event history per the C-1 event-sourced state machine discipline; cancel outstanding custody-quilt Commitments on dispose
- `elohim/sdk/schemas/v1/views/economic-resource-view.schema.json` + `economic-event-view.schema.json` — `lifecycle_state` field derived in view (not entry-side field)
- `elohim/sdk/domains/elohim/manifest.json` — declare action verbs + per-pillar `disposition_kind` enumerations for the cradle-to-cradle hook
- D.3 interlock — closed CIDs don't receive new custody-quilt Commitments
- D.1 interlock — subordination link creation rejected against closed parents
- D.20 interlock — high-reach dispose Events route through Global Commons elohim-council attestation (commons-reach disposal of public-good Resources needs witness)

---

### D.8 Bridge Pattern for Legacy Systems (Gap 9) — Wave D

**Motivation.** Legacy systems — banks, payment processors, KYC vendors, regulators, existing platforms — exist outside the substrate. The protocol's posture is parallel operation + subsumption-by-merit, not displacement. Without a bridge pattern, every legacy-interop need becomes a bespoke integration with no shared substrate semantics; users can't cash out cleanly; the substrate's commitment to bidirectional legibility breaks.

**Reframe — bridges as substrate-native Collective EPRs (per operator's mid-Phase-2 correction).** Bridges aren't passthrough adapters. Plaid Inc, Stripe Inc, banking-API providers, KYC vendors each have their own substrate presence as **Collective EPRs**. Their employees are Memberships of the collective; their commercial revenue flows into their Bridge Commons (a Commons-held Resource per D.20). Their stewardship-elohim agents handle legacy-protocol translation as a *service* — translating between the legacy vendor's API and the substrate's primitives, on behalf of households that use that translation. **Bridges ARE substrate participants**, not outside-of-substrate adapters.

**Design — per-vendor bridge structure.**

Each legacy vendor gets a `bridges/<vendor>/` crate (pattern reference: `bridges/valueflows/`) with three sub-crates:

```
bridges/<vendor>/
  ├── <vendor>-bridge/        # the bridge service: speaks vendor API + authors substrate Events
  ├── <vendor>-types/         # vendor wire shapes + manifest declarations
  └── <vendor>-tests/         # contract tests against vendor sandbox + substrate fixtures
```

The `<vendor>-bridge` service:
- Listens for vendor webhooks (OAuth/HTTP) via `doorway/doorway-service/src/handlers/bridges/<vendor>/`
- Maps incoming webhook events to substrate Observations
- Signs the Observations as the **bridge-stewardship-elohim** under a stewardship-commitment Attestation referencing the household-that-authorized-this-bridge
- ReconcileController graduates the Observations per the pillar's graduation policies (D.5 + D.6 interlock)
- For events the substrate authors that flow back to the vendor (user-initiated transfers, etc.), translates the substrate Event into vendor API calls; records confirmation as observation_refs on the Event

**Countersignature through bridges (A-2 resolution interlock).** Per the substrate-invariant countersignature decision: when grandma's account-EPR signs a `transfer` Event and the receiver (Joe's Coffee) isn't substrate-native yet, the bridge-stewardship-elohim signs receiver-side on behalf of a stub-EPR for Joe's Coffee. When Joe's Coffee later adopts substrate, they can attest/claim the stub-EPR's history. **The bilateral-transfer invariant is preserved through bridges.**

**Fee mechanics defer to D.20.** The Bridge Commons revenue model (small Commons fee per facilitated transaction + Global Commons fee per same transaction; cash-out path to legacy money) is the substrate of D.20 Layered Commons. D.8 specifies the bridge-translation machinery; D.20 specifies how value flows into/out of Bridge Commons.

**Backfill pattern.** When a household first authorizes a bridge to a legacy system (e.g., Plaid sees their bank's 10-year transaction history), the bridge authors `Observation(observation_kind="bridge:backfill", payload_json={...})` for each legacy record. Graduation policies batch-graduate these into substrate Events under stewardship-elohim signature. The substrate gains a full event-history that matches the legacy system's record; the household's Monarch-shape dashboard immediately shows 10 years of substrate-native history.

**Cash-out — bidirectional structural property.**

When a household wants to leave a bridge:
1. Authorize-revocation Event from the household → bridge-stewardship-elohim's commitment chain closes
2. Bridge stops authoring new Observations
3. Existing Events stay in substrate (permanent record)
4. Household exports bridge-authored Events to legacy format (QFX/CSV/etc.) if needed for legacy continuity

When the bridge-collective wants to cash-out their Commons-accumulated value to legacy money:
1. Bridge-stewardship authors `Event(action="cash-out", provider=bridge_commons, receiver=legacy-USD-bank-account, observation_refs=[bank-transfer-confirmation])`
2. Vendor-side: bridge service initiates the legacy bank-transfer via vendor API
3. On confirmation, observation_refs link to the legacy transfer proof; Bridge Commons balance decreases by the cashed-out amount

Cash-out is **structural**, not policy. Both sides can leave their relationship to the bridge at any time without losing accumulated value.

**PII removal under right-to-be-forgotten (Phase 1 A.6 addendum).** Bridge-authored Attestations (especially KYC) may contain PII in `metadata_json` (name fragments, DOB, ID numbers). When a `attestation:forget-decision` Event lands per mishpat governance:
1. ReconcileController identifies all bridge-authored Attestations with matching subject
2. Coordinator scrubs PII fields from `metadata_json` (replaces with `<redacted>` markers); the Attestation entry's CID changes (new entry; old entry root-rewritten); downstream attestations referencing the old CID add `redaction-applied` notes
3. Storage projection drops the PII columns from the row-level view

This is the same right-to-be-forgotten flow described in `2026-05-11-observation-event-layer-design.md` §9.4, applied to bridge-authored Attestations specifically.

**KYC bridge migration (Phase 1 A.6 addendum).** When a household migrates from one KYC bridge (e.g., Stripe Identity) to another (e.g., Onfido):
1. Prior bridge's Attestations stay valid (append-only DHT); the household still has the credentials issued under the prior bridge
2. Prior bridge can NO LONGER issue new credentials (their stewardship-commitment Attestation expires per household's revocation)
3. New bridge's stewardship-commitment Attestation activates; new bridge can issue new credentials
4. Household's effective identity is the union of all valid bridge-authored credentials (no single bridge holds identity authority alone)

This is the same parallel-credentials pattern as multi-doorway human registration (per `project_multi_doorway_human_registration` memory).

**Manifest declaration.** Per-pillar declarations for bridge kinds + per-bridge fee schedules (feed D.20):

```jsonc
// elohim/sdk/domains/shefa/manifest.json
{
  "bridge_kinds": [
    {
      "vendor": "plaid",
      "stewardship_collective_cid": "<plaid-collective-cid>",
      "fee_schedule": {  // feeds D.20
        "transaction_fee_pct": 0.005,
        "bridge_commons_share": 0.5,
        "global_commons_share": 0.5
      },
      "observation_kinds_authored": ["shefa:card-swipe", "shefa:bank-statement-parsed", "shefa:account-balance-refresh"],
      "attestation_kinds_authored": ["attestation:identity-credential", "attestation:account-linked"]
    }
  ]
}
```

**Touches:**
- `bridges/<vendor>/` — new per-vendor crates following the existing `bridges/valueflows/` pattern reference
- `doorway/doorway-service/src/handlers/bridges/<vendor>/` — HTTP route surface for vendor webhooks + OAuth callbacks
- `elohim/sdk/domains/{shefa,lamad,imagodei,qahal}/manifest.json` — `bridge_kinds` per pillar with fee_schedule + observation_kinds_authored declarations
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — validate bridge-stewardship-elohim-authored Events against the bridge-collective's stewardship-commitment Attestation chain
- `elohim/elohim-storage/src/services/bridge_stewardship_service.rs` — bridge-stewardship-elohim implementation (planned)
- D.6 interlock — bridge-stewardship-elohim is a domain-elohim specialization
- D.20 interlock — bridge fee schedules feed the Layered Commons ratchet
- A-2 interlock — bridge-side countersignature preserves the bilateral-transfer invariant

---

### D.15 Cross-DNA Coordination Patterns (Gap 16) — Wave D

**Motivation.** Phase 1 A.8 surfaced three places where the substrate's DNA boundaries produce coordination ambiguity:

- `GovernanceActionChild` link from elohim DNA → mishpat DNA (governance actions live in mishpat; the link to track child outcomes lives in elohim) — but cross-DNA `create_link` is not standard HDK. The current code works because elohim DNA's coordinator zome makes a `call(CallTargetCell::OtherRole("mishpat"), "...", ...)` — that pattern isn't documented.
- Hub-graduation failover (resolved in D.6 as first-quorum-wins) crosses DNA boundaries when the issuing hub is on a different elohim DNA cell than the graduation evaluator.
- Meta archetype's "friend / follow" link types ambiguously cite "imagodei `AgentToRelationship`" vs. "lamad `ContentToRelated`" vs. "elohim `RelationshipBySource/Target`" — without a clear answer on which zome owns the social-graph link types, two teams could build incompatible graph projections.

**Design — three coordinated patterns.**

**Pattern 1: Cross-DNA link creation via coordinator-zome bridge call.** The standard pattern for creating a link in another DNA is:

```rust
// from elohim DNA's content_store coordinator
let result = call(
    CallTargetCell::OtherRole("mishpat".to_string()),
    ZomeName::from("governance_actions"),
    FunctionName::from("create_link_in_dna"),
    None,  // cap_secret
    payload,
).await?;
```

The target DNA's coordinator zome exposes a `create_link_in_dna(base, target, link_type, tag)` function that wraps `create_link!` in its own integrity zome's namespace. The bridge-call pattern works at the DHT layer because both DNAs participate in the same cell's signal-fan-out.

**Worked example: `GovernanceActionChild`.** When elohim DNA needs to record a governance-action child outcome, it calls into mishpat DNA's coordinator zome. Mishpat's `create_governance_action_child_link(parent_cid, child_cid)` creates the link in mishpat's link-types namespace; elohim's projection queries against mishpat via the bridge-call pattern when needed.

**Pattern 2: D-2 resolution — social-graph zome ownership.** Per operator lean:

| Social-graph primitive | Zome ownership | Semantic |
|---|---|---|
| **Symmetric friend** | imagodei DNA's `AgentToRelationship` link type | mutual reach grant; both parties' identity-graph |
| **Asymmetric follow** | Collective EPR Membership in the followed Profile's collective | reach-extending; receiver doesn't reciprocate |
| **Block / mute** | FeedbackSignal (`signal_kind: "mute"` per D.18 — Wave 2 application archetype dispatches add this to manifest) | locally-private; affects feed-ranking only |

Meta archetype updates its application-design.md to cite these zomes precisely (operator follow-up note for the Wave 2 archetype-drafting dispatch).

**Pattern 3: Hub-failover signaling across DNAs.** When the elohim DNA's graduation evaluator is sharded across multiple cell-instances (D.6 D-1a kind_namespace sharding), and one cell goes offline, the surviving cells need to detect the absence. Pattern:

- Cells publish heartbeat Observations (`observation_kind: "elohim:graduation-evaluator-heartbeat"`) per shard
- A peer-witness detection: when consensus-of-peers observes absence of expected heartbeat for >threshold, the first-quorum-wins re-takeover happens at the cell-shard level
- The substrate's substrate-floor invariant: graduation Attestations carry a `shard_id` field in metadata; CRDT-style dedup prevents duplicates across failover (D.6 idempotent-check)

**Failover scope.** Per-shard failover handles graduation-evaluator outages cleanly. Bridge-stewardship-elohim failover (Phase 1 A.4 + D.6 interlock) operates similarly: bridge-collective declares the fallback chain; first-quorum-wins among the chain takes over.

**Touches:**
- DNA boundaries (`elohim/holochain/dna/{elohim,imagodei,mishpat}/zomes/`) — document the `call(CallTargetCell::OtherRole(...))` bridge pattern in each coordinator zome's CLAUDE.md
- Hub-failover detection in `peer_transport_manifest` watcher in elohim-storage
- Meta archetype's `applications/meta-facebook-application-design.md` — update to cite imagodei `AgentToRelationship` for symmetric friend; Collective Membership for asymmetric follow (operator follow-up)
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — `shard_id` field on Attestation metadata for failover-dedup

---

### D.16 Multi-Oracle Confirmation-Class Attestation (Gap 17) — Wave D

**Motivation.** Phase 1 A.6 surfaced that Monarch's `attestation:price-feed` (the daily price quote that drives investment-balance calculation) is structurally vulnerable to single-oracle-elohim chokepoint: if one oracle is captured or compromised, every household's net worth calculation is wrong. The `proof_evidence.class = confirmation` tier was designed for this — multi-attestor confirmation — but the format for the confirmer chain was never specified.

**Design — multi-attestor confirmation chain format.**

```rust
pub struct ProofEvidence {
    pub class: ProofEvidenceClass,  // witness | audit | proof | confirmation
    // ... existing fields ...

    // confirmation-class specific:
    pub confirmer_signatures: Option<Vec<ConfirmerSignature>>,
}

pub struct ConfirmerSignature {
    pub confirmer_cid: Cid,         // the confirming agent (oracle-elohim, council member, etc.)
    pub confirmer_collective_cid: Option<Cid>,  // diversity-grouping
    pub signature: Signature,         // signs the same fact as the issuer
    pub signed_at: i64,
}
```

**Manifest-declared confirmation requirements per attestation_kind.**

```jsonc
// elohim/sdk/domains/shefa/manifest.json
{
  "vocabulary_declarations": {
    "attestation_kinds": [
      {
        "kind": "attestation:price-feed",
        "default_proof_class": "confirmation",
        "confirmation_requirements": {
          "min_confirmer_count": 3,
          "diversity_requirements": {
            "distinct_confirmer_collectives": 3,  // each confirmer from a different oracle-collective
            "distinct_regions": 2                   // geographic diversity
          },
          "max_confirmer_clock_skew_seconds": 300  // confirmations within 5min
        }
      }
    ]
  }
}
```

**Validator floor (extends Floor 8).** Integrity zome rejects `proof_evidence.class = confirmation` attestations without:
- Minimum confirmer count met
- Diversity requirements met (verified via the inline diversity tags model from observation spec §4.3 + 6.1)
- All confirmer signatures cryptographically valid (each signs the same canonical fact)

**Applies to.** Per Phase 1 findings:

| Attestation kind | Why confirmation-class |
|---|---|
| `attestation:price-feed` | Monarch net-worth depends on it; single-oracle chokepoint risk |
| `attestation:computation` | AWS-shape compute verification — proof that paid-for work ran |
| `attestation:doorway-health` | Infrastructure liveness — single-witness chokepoint at hub scale |
| `attestation:identity-credential` | KYC chain — multi-bridge diversity prevents identity-authority capture (D.8 interlock) |

**Touches:**
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/attestation_validator.rs` — extend Floor 8 with multi-attestor validation; verify min_confirmer_count + diversity_requirements + signature chain
- `elohim/sdk/schemas/v1/views/attestation-view.schema.json` — add `confirmer_signatures` field with the ConfirmerSignature shape
- `elohim/sdk/domains/{shefa,lamad,infrastructure}/manifest.json` — declare `confirmation_requirements` per high-stakes attestation_kind
- D.6 interlock — graduation-evaluator authors confirmation-class attestations only when sufficient peer-witness confirmations land in the window

---

### D.17 Recurring Commitment Scheduler Stagger (Gap 18) — Wave D

**Motivation.** Phase 1 A.5 surfaced that 1M-patron Patreon-shape recurring billing creates thundering-herd on the creator's projection node on first-of-month. No stagger discipline exists in the current Commitment scheduler. TierController has one (`blake3(cid || peer-id || epoch) % stagger_window`); the Commitment scheduler needs the analog.

**Design — manifest-declared stagger discipline.**

```rust
// elohim/elohim-storage/src/services/commitment_scheduler.rs (planned)
fn fulfillment_delay(commitment: &Commitment, agent_id: &Cid, billing_epoch: u64) -> Duration {
    let stagger_window_seconds = get_manifest_stagger_window(commitment.action);
    let seed = blake3(format!("{}{}{}", commitment.cid, agent_id, billing_epoch));
    let delay_seconds = u64::from_be_bytes(seed.as_bytes()[0..8].try_into().unwrap()) % stagger_window_seconds;
    Duration::from_secs(delay_seconds)
}
```

When a `Commitment(action="subscribe")` fires its monthly fulfillment, the scheduler delays the actual Event-authoring by `fulfillment_delay()`. Within the billing window (default 1 hour), 1M patrons distribute uniformly via the hash; the creator's projection node sees a constant ~278/sec inbound rather than 1M-at-once.

**Manifest-declared stagger windows per action_verb.**

```jsonc
// elohim/sdk/domains/shefa/manifest.json
{
  "vocabulary_declarations": {
    "action_verbs": [
      {
        "verb": "subscribe",
        "fulfillment_pattern": "recurring",
        "stagger_window_seconds": 3600  // 1 hour billing distribution
      },
      {
        "verb": "checkpoint",
        "fulfillment_pattern": "recurring",
        "stagger_window_seconds": 86400  // 24 hours (low priority)
      },
      {
        "verb": "transfer",
        "fulfillment_pattern": "immediate",
        "stagger_window_seconds": null
      }
    ]
  }
}
```

`null` for `immediate`-fulfillment verbs (transfers fire immediately; no stagger). Recurring verbs (subscribe, checkpoint, custody-quilt periodic renewal) declare their stagger window.

**Substrate-floor invariant.** Recurring-fulfillment verbs without manifest-declared stagger_window get the default `3600s` (1 hour). D.10's vocabulary governance gate validates declarations.

**Touches:**
- `elohim/elohim-storage/src/services/commitment_scheduler.rs` — new service implementing stagger discipline per the BLAKE3 hash distribution
- `elohim/sdk/domains/*/manifest.json` — declare `fulfillment_pattern` + `stagger_window_seconds` per recurring-fulfillment action verb
- D.12 interlock — `checkpoint` Commitment authoring uses the same stagger model

---

### D.18 FeedbackSignal `signal_class` Field — Care/Compute Isolation (Gap 19) — Wave D

**Motivation.** Phase 1 A.7 found that `debit-firm` quarantine on a bad-compute provider would violate compute/care isolation silently — the `SIGNAL_KINDS` whitelist doesn't distinguish what kind of standing is being debited. Per `project_compute_commitments_bounded`, compute-class and care-class standing must stay isolated.

**Design — `signal_class` field on FeedbackSignal.**

```rust
pub struct FeedbackSignal {
    // ... existing fields ...
    pub signal_class: SignalClass,  // NEW
}

pub enum SignalClass {
    Care,        // care-economy social moves (endorse, comment, react in care contexts)
    Compute,     // compute-tier evidence (compute-provider reliability)
    Governance,  // governance-evidence signals (report, dispute)
    Trust,       // identity-trust signals (vouch, key-recovery-witness)
}
```

**Manifest-declared `signal_kind → signal_class` mapping.**

```jsonc
// elohim/sdk/domains/imagodei/manifest.json
{
  "vocabulary_declarations": {
    "signal_kinds": [
      {"kind": "endorse",  "signal_class": "care",       "standing_impact": "credit-soft"},
      {"kind": "comment",  "signal_class": "care",       "standing_impact": null},
      {"kind": "react",    "signal_class": "care",       "standing_impact": null},
      {"kind": "report",   "signal_class": "governance", "standing_impact": "debit-soft"},
      {"kind": "vouch",    "signal_class": "trust",      "standing_impact": "credit-firm"},
      {"kind": "quarantine", "signal_class": "governance", "standing_impact": "debit-firm"}
    ]
  }
}

// elohim/sdk/domains/shefa/manifest.json
{
  "vocabulary_declarations": {
    "signal_kinds": [
      {"kind": "compute-failure-report", "signal_class": "compute", "standing_impact": "debit-soft"}
    ]
  }
}
```

**Standing-curve isolation.** The standing-curve service computes a per-(author, signal_class) standing tuple, not a single global standing:

```rust
pub struct StandingScore {
    pub author_cid: Cid,
    pub care_standing: f64,
    pub compute_standing: f64,
    pub governance_standing: f64,
    pub trust_standing: f64,
}
```

Care debits affect care_standing only; compute debits affect compute_standing only. Bad-compute providers are not flagged as low-care; bad-care actors are not flagged as low-compute. Reach gating per-class: care-content reach uses care_standing; compute-marketplace participation uses compute_standing.

**Substrate-floor invariant.** Integrity zome validates that signal_class on a FeedbackSignal matches its signal_kind's manifest-declared class (D.10 vocabulary governance enforces). Standing-curve service consumes signal_class to keep classes isolated.

**Touches:**
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/feedback_signal.rs` — add `signal_class: SignalClass` enum field on FeedbackSignal; validation enforcing manifest-declared mapping
- `elohim/sdk/schemas/v1/p2p/feedback-signal.schema.json` — add `signalClass` field to wire schema
- `elohim/sdk/domains/*/manifest.json` — declare per-signal_kind signal_class (D.10 gate validates)
- `elohim/elohim-storage/src/services/standing_curve_service.rs` (planned per D.14) — derive per-class standing rather than global
- D.14 interlock — `standing_scores` SQL view returns per-class tuple

---

### D.19 agent-private EPR-Mediated Key Recovery (Gap 20) — Wave D

**Motivation.** Phase 1 A.4 (and observation spec open question #3) — when a human migrates to a new device, the observer's `agent-private` encryption key (for private observations) must be recoverable without putting the burden on the human to remember a master passphrase. Per `project_socially_derived_security` + `project_recovery_grandma_standard`: relationships are the primary security primitive; cryptography enforces what relationships authorize. **Per operator correction: EPR-mediated recovery is the primary path; cryptographic Shamir is an opt-in add-on.**

**Design — primary path (EPR-mediated trusted-circle recovery).**

Each agent maintains a `trusted-circle` Membership EPR — a Collective EPR (`content_type: "trusted-circle"`) where the agent's intimate household + collective stewards are Memberships. The trusted-circle is itself a substrate primitive; the agent authors it at first device-setup; Memberships can be added/removed over time via household governance.

Recovery flow:

```
1. Human on new device authenticates with their substrate identity (private key) — possible
   because device-setup ceremony loaded a partial-key recovery surface

2. Or: human on new device has lost their identity-private key entirely
   - Initiates recovery via doorway-service or local recovery flow
   - Recovery flow surfaces a list of trusted-circle Memberships
   - Each trusted-circle member receives a notification (in their elohim-app) to attest
     to the recovery: "Mira is trying to recover her account on a new device. Do you
     recognize this person?"
   - Each member authors:
       Attestation(content_type="attestation:key-recovery-authorization",
                   subject_cid=recovering_agent_cid,
                   metadata_json={"recovering_device_pubkey": "...", "context": "..."})

3. When N members have authored (manifest-declared threshold, default 3 of 5):
   - Substrate-floor invariant: the recovering device's pubkey is now authorized
   - The agent's elohim-node on the trusted-circle stewards re-derives the
     `agent-private` encryption key from the social-attestation chain
   - The re-derived key is delivered to the new device via libp2p direct-message
     under reach=agent-private

4. The new device now has the agent's keys (both substrate identity AND observer's
   agent-private encryption key); the human is fully restored
```

**"Log in with help from your people" applied to encryption keys.** The substrate doesn't ask the human to remember anything cryptographic — it asks their trusted-circle to attest that they recognize them. Grandma can recover her account because her trusted-circle (household + neighbors + community stewards) recognize her.

**Design — optional add-on (Shamir secret-sharing).**

Manifest opt-in: households can layer Shamir-N-of-M cryptographic split — each trusted-circle member additionally holds a Shamir share. Recovery then requires BOTH the EPR-attestation threshold AND the cryptographic share threshold.

**Defense in depth where external threat surface warrants it** — not as a wealth-class concern (the substrate's friction-gradient and Global Commons ratchet prevent accumulation classes from forming structurally; there is no "high-net-worth" stratum on this protocol). The Shamir layer addresses external adversarial threat that makes social-attestation recovery insufficient on its own:

- Journalists protecting sources from state subpoena (state could subpoena trusted-circle members but not their cryptographic shares without escalation)
- Dissidents under state threat
- Intimate-partner-violence survivors where the trusted-circle pattern itself can be compromised by an abuser within the social graph (the abuser is in the trusted-circle; the Shamir share is held by an external trusted-circle member that the abuser doesn't have access to)
- Households stewarding strategic public-good Resources where the threat surface extends beyond household-scale relationships

**Manifest-declared per-household policy.**

```jsonc
// per-household manifest
{
  "recovery_policy": {
    "recovery_mode": "epr_mediated_with_shamir",  // or "epr_mediated"
    "trusted_circle_threshold": { "n": 3, "of": 5 },
    "shamir_threshold": { "n": 3, "of": 5 },  // null if recovery_mode is "epr_mediated"
    "trusted_circle_cid": "<membership-epr-cid>"
  }
}
```

**Substrate-floor enforcement.** Integrity zome rejects key-recovery without sufficient `attestation:key-recovery-authorization` attestations. Coordinator-side flow handles Shamir share verification when manifest opts in.

**Touches:**
- `imagodei` DNA recovery flow — new `recover_agent_keys` coordinator function
- Source-chain export logic — includes trusted-circle Membership reference
- Multi-device pairing flow — handles the recovery ceremony
- Trusted-circle Membership EPR pattern (declared per pillar manifest)
- `attestation:key-recovery-authorization` subtype declaration (D.10 vocabulary governance)
- `elohim/sdk/schemas/v1/views/attestation-view.schema.json` — `key-recovery-authorization` shape

---

### D.20 Layered Commons + elohim-Mediated Global Commons (Gap 21) — Wave D

**Motivation.** Every value flow through the substrate ratchets a manifest-declared slice into one or more Commons-held Resources at different governance scopes. **This is where the elohim-apex thinking gets cashed out as actual value flow** — the network self-funds its development, infrastructure, and anti-concentration enforcement through substrate-native fee flows mediated by elohim councils. Not a tax (no enforcement layer); a manifest-declared substrate invariant. Per `project_elohim_councils_capture_apex` + `project_commons_elohim_co_steward`: wisdom holds the structural top of authority; the Global Commons is where wisdom-stewardship of public-good value flows lives.

**Design — three Commons scopes.**

```
┌─────────────────────────────────────────────────┐
│  Every Event(action="transfer") ratchets:        │
│                                                  │
│  Grandma  ─99%─►  Joe's Coffee  (the actual      │
│                                   value transfer)│
│        │                                         │
│        ├──0.5%──►  Bridge Commons               │
│        │           (Plaid Inc Collective revenue)│
│        │                                         │
│        └──0.5%──►  Global Commons               │
│                    (elohim-mediated public goods)│
└─────────────────────────────────────────────────┘
```

(Percentages are illustrative; manifest-declared per pillar/per bridge/per transaction-class.)

**Three Commons scopes — each a substrate-composable EPR (no new entry types).**

| Scope | What it is | Stewardship |
|---|---|---|
| **Bridge Commons** | Bridge-collective's revenue (Plaid, Stripe, banking-API, KYC, etc.) | Bridge-collective's own Memberships govern; legacy cash-out via D.8 |
| **Collective Commons** | Collective EPR's shared pool (multi-family pools, credit-union-style mutual aid, organization shared funds) | Collective members + collective's co-steward elohim govern |
| **Global Commons** | Protocol-wide; funds substrate development, new bridges, public goods, anti-concentration redistribution | **Elohim-mediated** per apex-elohim council pattern; humans don't accumulate it |

**Composable EPRs (substrate-floor primitives).**

```rust
// Each Commons is a Content EPR with a Membership of stewards
Content {
    content_type: "commons",
    metadata_json: serde_json::json!({
        "scope": "global" | "collective" | "bridge",
        "stewardship_membership_cid": "<membership-epr-cid>",
        "anti_concentration_policy": {...},  // per pillar/scope
    }).to_string(),
}

// Commons-held value is an EconomicResource
EconomicResource {
    resource_classified_as: "commons-credit",  // or "currency-USD" for stablecoin-denominated
    parent_epr_cid: <commons_epr_cid>,
    primary_accountable: <commons_steward_or_council_cid>,
    governed_by: <stewardship_collective_cid>,  // D.4 governed_by field interlock
}
```

**Fee mechanics — substrate-floor mass-conservation.** Every Event(action="transfer") may declare a manifest-defined fee split:

```rust
Event {
    action: "transfer",
    provider: <source>,
    receiver: <destination>,
    resource_quantity_value: 100.0,  // the receiver's share
    metadata_json: serde_json::json!({
        "fee_splits": [
            {"commons_cid": "<bridge_commons_cid>",  "amount": 0.5},
            {"commons_cid": "<global_commons_cid>",  "amount": 0.5}
        ],
        "total_authored_amount": 101.0  // mass-conservation: receiver + sum(fees) = total
    }).to_string(),
    // ...
}
```

Companion fee Events land alongside the main transfer Event. Substrate-floor validation: sum of all fee_splits + receiver_amount = total_authored_amount (mass-conservation check enforced by integrity zome).

**Global Commons elohim-council governance.** The apex elohim councils participate as Memberships of the Global Commons EPR. Allocation Events from the Global Commons (funding decisions — new bridges, infrastructure, public goods) require quorum of council attestations per the apex-elohim governance pattern. **The substrate's self-funding mechanism cannot be captured by self-interest** because the elohim councils don't accumulate (per `project_elohim_councils_capture_apex` — wisdom holds the apex; cannot extract for personal gain).

```
allocation flow:
  1. Funding proposal: any participant authors a `proposal` Content EPR
     describing what should be funded (new bridge, infrastructure work,
     public-good investment, etc.)
  2. Apex elohim councils (Memberships of Global Commons EPR) attest to
     the proposal under apex-elohim governance rules
  3. When quorum is reached:
     Event(action="allocate", provider=global_commons_resource,
           receiver=proposal_recipient_resource, quantity=<amount>)
  4. The recipient (a person's elohim-agent, a Collective EPR, a bridge-
     collective, etc.) receives the allocation; can author Events from it
     per the allocation's authorized purpose
```

**Anti-concentration ratchet.** Per `project_friction_gradient_limitarianism`: the fee_split percentages are manifest-declared as a **friction gradient** — as an EPR accumulates large balances (relative to manifest-declared scale), a larger fraction of incoming Events ratchets into Commons. This makes accumulation mechanically expensive at the substrate floor; no high-net-worth class forms because every accumulation operation is friction-resisted.

Friction-gradient declaration (per pillar manifest):

```jsonc
// elohim/sdk/domains/shefa/manifest.json
{
  "friction_gradient": {
    "scale": "currency-USD",
    "tiers": [
      {"upper_bound": 100000,    "global_commons_share": 0.005},   // 0.5%
      {"upper_bound": 1000000,   "global_commons_share": 0.02},    // 2%
      {"upper_bound": 10000000,  "global_commons_share": 0.10},    // 10%
      {"upper_bound": null,      "global_commons_share": 0.30}     // 30% above $10M-equivalent
    ]
  }
}
```

As an EPR's holdings grow into higher tiers, the ratchet rate increases. Substrate refuses concentration ops at the floor by making them mechanically expensive — not by prohibition.

**Cash-out path.** Bridge-collectives can author `Event(action="cash-out", provider=bridge_commons, receiver=legacy-USD-bank-account, observation_refs=[bank-transfer-confirmation])` to convert Commons-held value to legacy money. Substrate doesn't subsidize legacy translation — bridges do, at their own cost, recouped via the Commons fee. The cash-out is itself a substrate-native Event that ratchets through the Global Commons (the substrate takes its share of bridge-collective cash-out, returning value to the network even as the bridge bridges to legacy).

**Manifest declaration.** Per-pillar fee splits, per-bridge fee schedules, per-Collective Commons policies, Global Commons allocation rules:

```jsonc
// elohim/sdk/domains/elohim/manifest.json — Global Commons declarations
{
  "global_commons": {
    "epr_cid": "<global-commons-epr-cid>",
    "apex_elohim_council_membership_cid": "<apex-elohim-council-membership-cid>",
    "allocation_quorum": { "n": 5, "of": 7 },
    "purpose_categories": ["bridge-development", "infrastructure", "public-good", "anti-concentration-redistribution"]
  }
}
```

**Touches:**
- `elohim/sdk/domains/elohim/manifest.json` — Global Commons declarations + apex-elohim council Membership EPR
- `elohim/sdk/domains/*/manifest.json` — per-pillar friction_gradient + per-pillar fee schedules
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — fee-split validation (mass-conservation check on Events with fee_splits); reject Events whose fee_splits don't sum to (total - receiver_amount)
- `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` — coordinator functions for `create_commons`, `allocate_from_commons` (apex-elohim governance gate)
- `elohim/elohim-storage/src/services/commons_steward_service.rs` (planned) — commons-stewardship-elohim agent (D.6 interlock)
- `elohim/elohim-storage/src/services/friction_gradient_service.rs` (planned) — per-EPR balance lookup → ratchet-tier resolution at Event-authoring time
- D.8 interlock — bridge fee schedules feed Bridge Commons; bridge-collective stewards govern
- D.6 interlock — commons-stewardship-elohim is the apex-elohim-council agent specialization
- D.9 interlock — reach to commons-tier engages Global Commons elohim-council attestation chain
- D.7 interlock — disposal of public-good Resources at commons-reach engages council witness

### D.9 Reach-Mutation Events (Gap 10) — Wave C

**Motivation.** Reach is the substrate's nervous system — when an EPR or Resource's reach changes, the substrate's visibility, gossip cost, and attestation requirements all shift. Today, reach changes are often implicit side effects of other operations (a Resource listed for sale via marketplace UI; a Post made public via app-side toggle). The audit trail is opaque; "who could see this when?" can't be reliably reconstructed. Three concrete problems:

- **Forensic ambiguity**: A Resource that was at reach=community for a month then narrowed to household can't be replayed precisely. Anyone whose access depended on the prior reach has no record of when it changed.
- **Compounding-reach attacks**: a malicious agent can systematically widen reach beyond their actual authority by exploiting reach-change paths that don't validate authority — the substrate currently has no per-mutation authority check.
- **Reach-mutation as first-class event interlocks**: D.1 (subordination), D.2 (surface), D.3 (submerge), D.7 (dissolution), D.20 (Layered Commons) all reference reach-state but have no canonical Event to bind to. Each operation re-derives reach-state from current-state observation rather than from a verifiable event history.

Making reach-mutation a first-class Event closes all three.

**Design — three new action verbs as substrate-floor Events.**

```rust
Event { action: "grant-reach",      subject_cid: <epr_or_resource>, metadata_json: serde_json::json!({"target_reach": "community", "rationale": "..."}).to_string(), ... }
Event { action: "revoke-reach",     subject_cid: <epr_or_resource>, metadata_json: serde_json::json!({"target_reach": "household", "rationale": "..."}).to_string(), ... }
Event { action: "reclassify-reach", subject_cid: <epr_or_resource>, metadata_json: serde_json::json!({"target_reach": "commons-attested", "rationale": "...", "prior_reach": "commons"}).to_string(), ... }
```

The Event records the transition; the projection updates the current-effective-reach view. **Current reach is derived from event history** (most recent reach-mutation Event for the subject), not stored on the entry — same C-1 event-sourced state machine discipline.

**Validation rules — substrate-floor authority chain.** Reach changes validate against current standing + the per-target-reach authority model from D.2 surface (authority scales with the reach the mutation produces):

```
validate_reach_mutation_event(event) {
  let target_reach = event.metadata_json.target_reach;
  let current_reach = current_effective_reach_from_projection(event.subject_cid);
  let authoring_standing = standing_score_for_author(event.provider);

  // Substrate-floor: can't grant reach you don't have
  let max_reach_author_can_grant = author_max_grantable_reach(event.provider);
  if target_reach > max_reach_author_can_grant {
    return Err("authoring agent cannot grant reach above their standing");
  }

  // Per-target-reach authority chain
  match target_reach {
    Reach::Household => require_custodian_or_authorized_stewardship_elohim(),
    Reach::Community | Reach::Collective => require_custodian + peer_witness_attestation(),
    Reach::Commons => require_mishpat_governance_chain(),
    Reach::CommonsAttested => require_apex_elohim_council_quorum(),  // D.20 interlock
  }
}
```

**Council-arbitration for `commons` / `commons-attested` elevations.** When a Resource or EPR moves to commons-tier reach (publicly visible across the network), the elohim councils participate in the attestation chain (per `project_elohim_councils_capture_apex`). This is the same mechanism D.20 uses for Global Commons stewardship — reach to commons IS visibility to the protocol-wide layer, which is where the apex-elohim governance lives.

**Audit trail — reach history is a derived view.** A query against `reach_history(subject_cid)` returns the full event-history of reach-mutations for the subject:

```sql
SELECT
  e.observed_at,
  (e.metadata_json::jsonb->>'target_reach') AS new_reach,
  (e.metadata_json::jsonb->>'prior_reach') AS prior_reach,
  e.provider AS authoring_agent,
  e.observation_refs
FROM economic_events e
WHERE e.subject_cid = :subject_cid
  AND e.action IN ('grant-reach', 'revoke-reach', 'reclassify-reach')
ORDER BY e.observed_at ASC;
```

The view is queryable by anyone with current reach to the subject. Forensic replay: "who could see this when?" answered deterministically from the audit chain.

**Interlocks with Wave A/B/C gaps.**

- **D.1 (subordination)**: a Resource subordinating under a parent EPR adopts the parent's reach by default. If the subordination needs a different effective reach, a paired `reclassify-reach` Event lands alongside the subordination link.
- **D.2 (surface)**: surface to a `commons` reach requires the council-arbitration chain; D.2's per-reach authority model uses D.9's reach-mutation Events as the canonical authority record.
- **D.3 (submerge)**: shelved content has reach behavior governed by the `shelf_destination` URI scheme; reach-mutation Events for shelved content honor the destination's reach semantics.
- **D.7 (dissolution)**: closed Resources/EPRs reject all reach-mutation Events (closed CIDs cannot have new reach state mutated).
- **D.20 (Layered Commons)**: reach mutations to commons-tier engage the apex-elohim council attestation; these reach mutations are the trigger that may also produce Signal-Aggregate Commitments (per D.12) — when a post's reach widens and then contracts, its accumulated signals may aggregate-subordinate at the contraction event.

**Manifest declaration.**

```jsonc
// elohim/sdk/domains/elohim/manifest.json
{
  "vocabulary_declarations": {
    "action_verbs": [
      {"verb": "grant-reach",       "stake_class": "high"},
      {"verb": "revoke-reach",      "stake_class": "high"},
      {"verb": "reclassify-reach",  "stake_class": "high"}
    ]
  }
}
```

All three have `stake_class: high` — reach mutations are not graduatable from observations; they require direct authoring with explicit authority. D.10's vocabulary governance gate validates the declarations.

**Touches:**
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — `REA_ACTIONS` add `grant-reach`, `revoke-reach`, `reclassify-reach`; validation rules enforcing per-target-reach authority chain
- `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` — coordinator handles reach-mutation Events; ReconcileController derives current-effective-reach projection
- `elohim/elohim-storage/src/views.rs` — `reach_state` derived view (current effective reach + history); used by every primitive that respects reach
- `elohim/sdk/domains/elohim/manifest.json` — declare action verbs
- `elohim/sdk/schemas/v1/views/economic-event-view.schema.json` — metadata_json schema for reach-mutation Events (target_reach + rationale fields)
- D.1, D.2, D.3, D.7, D.20 interlocks (above)

---

### D.10 Schema-First IoC Governance (Gap 11) — Wave A prerequisite

**Motivation.** Phase 1 architectural composition found that the substrate has at least six extensible vocabulary surfaces — action verbs (`REA_ACTIONS`), signal kinds (`SIGNAL_KINDS`), observation kinds, attestation kinds (`ATTESTATION_KINDS`), resource classifications (`RESOURCE_CLASSIFICATIONS`), content types — each declared across four authoritative locations: Rust whitelist constants, JSON view/wire schemas, pillar manifests, and codegen output. These locations have drifted independently. As a result, four active application archetypes today reference vocabulary that fails substrate validation:

- **Patreon archetype** uses `action="subscribe"` — not in `REA_ACTIONS`
- **Meta archetype** uses `signal_kind: "comment" | "endorse" | "react" | "report"` — none in `SIGNAL_KINDS`
- **Photos archetype** uses `attestation:auto-tag` / `attestation:face-cluster` — not in `ATTESTATION_KINDS`
- **Mint archetype** queries with `parent_epr_cid` filter — field doesn't exist yet (lands in D.1)

Plus a codegen bug: a literal `"$ref"` string appears as a member of the generated `ATTESTATION_KINDS` array, meaning an attestation with `content_type: "$ref"` would currently pass Floor 1 integrity validation.

These are **surface-drift bugs**, not authoring bugs. The schema-first IoC discipline (per `feedback_schema_first_ioc` memory) has not been applied uniformly. Without a structural fix, every sprint that adds vocabulary in one location without the others compounds the drift, producing silent 503/401 cascades downstream (per `feedback_schema_data_enum_drift_cascade`).

**Design — unified extensibility-vocabulary CI gate.** A single CI script (`pnpm run schema:check-extensibility-vocabulary`) validates each governed vocabulary across all four authoritative surfaces:

```
For each extensible vocabulary V in {
  REA_ACTIONS, SIGNAL_KINDS, OBSERVATION_KINDS,
  ATTESTATION_KINDS, RESOURCE_CLASSIFICATIONS, CONTENT_TYPES
}:
  Surfaces:
    R = Rust whitelist constant (e.g., elohim/.../feedback_signal.rs SIGNAL_KINDS)
    S = JSON schema enum (e.g., schemas/v1/p2p/feedback-signal.schema.json $.signalKind.enum)
    M = pillar manifests (e.g., elohim/sdk/domains/*/manifest.json $.vocabulary_declarations.signal_kinds[].kind)
    C = codegen output (e.g., generated_signal_kinds.rs)

  Assertions:
    1. Every value in R appears in S            — ERR schema-out-of-date
    2. Every value in S appears in at least one M — ERR manifest-out-of-date
    3. Every value in M is reflected in C        — WARN codegen-needs-rerun
    4. No literal codegen sentinels in R or C    — ERR codegen-bug
    5. Vocabulary declared in M but absent from R — WARN (supports "ship declaration before code")
```

**Pillar manifest extension.** Each pillar manifest gains a `vocabulary_declarations:` section:

```jsonc
{
  "manifest_kind": "shefa",
  "vocabulary_declarations": {
    "action_verbs": [
      {"verb": "transfer", "stake_class": "high"},
      {"verb": "subscribe", "stake_class": "high"},
      {"verb": "checkpoint", "stake_class": "operational"}
    ],
    "signal_kinds": [
      {"kind": "comment", "signal_class": "care", "validator": "..."},
      {"kind": "endorse", "signal_class": "care", "standing_impact": "credit-soft"}
    ],
    "observation_kinds": [...],
    "attestation_kinds": [...],
    "resource_classifications": [...],
    "content_types": [...]
  }
}
```

The gate reads these declarations to validate surface consistency. The manifest is the **source of truth** for vocabulary; Rust whitelists + JSON schemas + codegen are projections of it.

**Failure modes the gate catches** (Phase 1 inventory):
- `ATTESTATION_KINDS` missing `mastery`, `content-quality`, `custodian-commitment`, `auto-tag`, `face-cluster`, `computation`
- `SIGNAL_KINDS` missing `comment`, `endorse`, `react`, `report`
- `REA_ACTIONS` missing `subscribe`, `checkpoint`, `aggregate-subordinate`, `surface`, `cash-out`, `grant-reach`, `revoke-reach`, `reclassify-reach`, `mint`
- `RESOURCE_CLASSIFICATIONS` missing `backup-state`, `commons-credit`, `mastery`, `stewarded-physical`, `stewarded-digital`, `stewarded-compute`
- `proofClass` enum mismatch between `attestation-view.schema.json` and `attestation_validator.rs`
- `forget-request` in `SIGNAL_KINDS` whitelist but missing from `p2p/feedback-signal.schema.json` enum
- `"$ref"` literal in generated `ATTESTATION_KINDS`

**Migration discipline.** The gate is the prerequisite for closing all the vocabulary drift identified in Phase 1. Before any downstream gap (D.1, D.4, D.6, D.7, D.8, D.18, D.20) lands its substrate-side declarations, this gate must be wired so the additions don't drift again.

**Pre-launch hard cutover.** No backwards-compat shim. The first run of the gate against the current substrate produces an error report; subsequent tasks (the per-vocabulary remediation specified in their respective subsections) close each surface item.

**Touches:**
- `elohim/sdk/schemas/scripts/check-extensibility-vocabulary.mjs` — new CI gate script
- `package.json` (root) + per-pillar — new `schema:check-extensibility-vocabulary` npm script
- `.husky/pre-push` — invoke the gate when relevant files changed (manifests, whitelist constants, schemas, codegen outputs)
- `elohim/sdk/domains/*/manifest.json` — add `vocabulary_declarations:` section per pillar
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/generated_attestation_kinds.rs` — fix `$ref` sentinel bug
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/feedback_signal.rs` — `SIGNAL_KINDS` const remediation per downstream gaps
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — `REA_ACTIONS`, `RESOURCE_CLASSIFICATIONS`, content-type constants remediation
- `elohim/sdk/schemas/v1/views/attestation-view.schema.json` — `proofClass` enum reconciliation
- `elohim/sdk/schemas/v1/p2p/feedback-signal.schema.json` — `signalKind` enum reconciliation

---

### D.11 Substrate-Floor Validator Backfill (Gap 12) — Wave A prerequisite

**Motivation.** Phase 1 found that several substrate-floor invariants are aspirational documentation, not enforced code. The validator layer is the substrate's contract with its participants — when a documented invariant has no code enforcement, operators rely on human discipline to maintain it. That discipline is brittle: any agent that didn't read the documentation can violate the invariant, and the substrate accepts the violation silently. Three specific gaps surfaced in Phase 1, each separately documented but together representing a class of debt:

- **Attestation validator floors F2 / F4 / F6 are ACCEPT-all stubs** (marked TODO Task C.3 in `attestation_validator.rs`). Today, any agent with DHT write access can issue `attestation:mastery` for any subject in any concept domain without holding `attestation:steward` in that domain. Floor F4 (issuer eligibility for `attestation:content-quality`) and Floor F6 (subject domain match) have the same shape.
- **Attestation validator Floor 5 temporal completeness** is incomplete (TODO Task C.2 — the op's Action timestamp isn't threaded into the validator). A vote submitted after `closes_at` can currently pass integrity validation if the parent governance-action parses correctly. The "deterministic deadline enforcement" the canonical spec claims is not yet fully wired.
- **Manifest schema validator has no retention-class governance.** A manifest author can declare `lamad:sensor-biometric` with `retention_class: wisdom` and the protocol accepts it — directly violating `observer-protocol.md` Part VIII ("store video beyond 3-second processing window: forbidden"). The constitutional posture is enforced by human discipline, not by code.
- **LINK_ARCHITECTURE.md deprecation checklist is incomplete.** ~50 `*By{Attribute}` query-index link types violate the DHT-as-notary principle (per `project_three_layer_truth_model`) but haven't been retired. Every sprint that adds a new structural link burns the 256-cap further while these unretired query-index links continue consuming slots.

**Design — three coordinated backfills shipping together.** These are independent code changes but share a sprint because they together move the substrate-floor enforcement story from aspirational to actual. Shipping any one in isolation would leave the substrate looking partially invariant-protected, which is worse than transparently un-protected.

**Backfill 1 — Attestation validator floors closure.** Close Floors F2 (steward authorization for `attestation:mastery`), F4 (issuer eligibility for `attestation:content-quality`), and F6 (subject domain match) per the attestation-consolidation canonical spec §4.2. Close Floor 5 temporal completeness (Task C.2) — thread the op's Action timestamp through the validator for strict `closes_at` enforcement. Each floor reads its policy from the relevant pillar manifest's `attestation_kinds` declarations (per D.10 vocabulary governance), so manifests can amend authorization rules without code changes.

**Backfill 2 — Retention-class manifest validator extension.** Extend the manifest schema validation pipeline (`pnpm run schema:validate`) to reject `retention_class: wisdom | archival | attestation-feeding` on `observation_kind` schemas whose `subject_kind ∈ { environment, sensor }`. This is the substrate-floor enforcement of the witness-not-surveillance constitutional commitment. Validation runs at manifest write time + at every codegen run + at pre-push.

**Backfill 3 — LINK_ARCHITECTURE deprecation sweep.** Formally retire the `*By{Attribute}` query-index link types from the `LinkTypes` enum. Migration: every retired `*By*` link gets its query workload moved to SQL projection (the operational layer that should have been carrying it from the start). LINK_ARCHITECTURE.md updates the deprecation checklist to show closure. Slots reclaimed are returned to the 256-cap budget; future structural link additions (D.1's `EprToEvent` + `EprToResource`) can land without immediately crowding the cap.

**Sequencing.** Backfill 1 depends on D.10 (vocabulary governance landed first, so manifest authorization rules can be read by the floor logic). Backfills 2 and 3 are independent. All three land in Wave A so downstream gaps (D.1, D.6, D.7) ship against a substrate-floor that actually enforces what it documents.

**Touches:**
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/attestation_validator.rs` — Floor F2, F4, F6 close (Task C.3); Floor 5 temporal completeness (Task C.2)
- `elohim/sdk/schemas/scripts/validate-manifest.mjs` (or similar) — extend manifest schema validator with retention-class governance for sensor/environment observations
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — `LinkTypes` enum retire `*By{Attribute}` variants
- `elohim/holochain/dna/LINK_ARCHITECTURE.md` — close deprecation checklist; update the 256-cap accounting
- `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` — coordinator functions that previously created `*By*` links: re-route to SQL projection upserts via ReconcileController
- `elohim/elohim-storage/src/services/reconcile_controller.rs` — handle the SQL-projection upserts for queries that previously used `*By*` link traversal

---

### D.12 Checkpoint / Snapshot / Aggregate-Subordination Primitive (Gap 13) — Wave B

**Motivation.** Phase 1 returns identified a single shared shape across multiple primitives: event-sourced state grows unboundedly on the read side. Specifically:

- **A.3 Resource**: 10 years × 50 transactions/day per account = 182k events per Resource. `SUM(quantity_delta)` becomes non-trivial; balance materialization at dashboard render time gets slow.
- **A.7 FeedbackSignal**: social-velocity DHT-budget exhaustion is structurally unrelieved. 500 signals/user/day × 200 bytes hits the ~3000-entry neighborhood budget within months. Signal-Aggregate Commitment was named as the release valve but not wired.
- **A.6 Attestation**: graduation rate-ceiling missing; policy bugs can over-issue Attestations by orders of magnitude.
- **A.2 Event**: graduation evaluator throughput at hub scale ceiling.

These are read-side cost-growth problems on long-lived high-volume primitives. The substrate's event-sourcing discipline is correct (no balance-as-stored-field; no signal-as-pre-aggregated-counter) — but it needs an explicit release valve, or every long-lived Resource and high-velocity signal stream becomes mechanically expensive over time. This gap formalizes the release valve as **two new Commitment action verbs** that ride on existing primitives.

**Design — checkpoint Commitment (read-side balance snapshot).**

```rust
Commitment {
    action: "checkpoint",
    subject_cid: <resource_cid>,
    period_start: <unix_timestamp>,
    period_end: <unix_timestamp>,
    metadata_json: serde_json::json!({
        "balance_snapshot": { "quantity": ..., "unit": ..., "by_classification": {...} },
        "event_count_covered": <count>,
        "merkle_root_of_covered_events": <hash>,
    }).to_string(),
    state: "fulfilled",  // checkpoints land in fulfilled state directly
    // ... standard Commitment fields ...
}
```

A `checkpoint` Commitment authoritatively summarizes the balance state of a Resource at `period_end`. Read-path optimization: when a derived view queries balance, it finds the most recent `checkpoint` Commitment, takes the snapshot as the starting state, and only sums Events from `checkpoint.period_end` forward. The 10-year-deep Resource becomes a 1-quarter-deep read against an authoritative snapshot — orders of magnitude faster.

**Design — aggregate-subordinate Commitment (signal-stream subordination).**

```rust
Commitment {
    action: "aggregate-subordinate",
    subject_cid: <target_cid_being_aggregated_under>,
    period_start: <unix_timestamp>,
    period_end: <unix_timestamp>,
    resource_classified_as_json: serde_json::json!([
        "aggregation:feedback-signal",
        { "signal_kind": "endorse", "shelf_destination": "peer-cellar://..." }
    ]).to_string(),
    metadata_json: serde_json::json!({
        "signal_count_aggregated": <count>,
        "aggregate_metrics": { "total_endorse_count": ..., "distinct_authors": ..., "standing_impact_sum": ... },
        "merkle_root_of_aggregated_signals": <hash>,
    }).to_string(),
    state: "accepted",
    // ... standard Commitment fields ...
}
```

The Commitment serves dual roles:
1. **As Commitment**: it's an authoritative summary of the aggregated window (for reads — standing computation can use the aggregate metrics instead of re-deriving from individual signals)
2. **As custody-quilt authority**: it permits the underlying individual FeedbackSignal entries to subordinate per the canonical submerge pattern (Gap 4 / D.3). The signals move to cold archive; the Commitment retains the aggregate.

This is per A.7's surfaced finding: signal-dense content needs a DHT-budget release valve. The Commitment IS that release valve.

**B-5 resolution: aggregate-subordinate trigger threshold.** Manifest-declared per `signal_kind` with time-based fallback. Per-signal-kind policies live in pillar manifests:

```jsonc
// elohim/sdk/domains/imagodei/manifest.json
{
  "vocabulary_declarations": {
    "signal_kinds": [
      {
        "kind": "endorse",
        "aggregate_subordinate_policy": {
          "trigger_age_days": 30,           // signals older than 30d eligible
          "trigger_min_count": 100,         // OR 100+ signals on same target
          "trigger_after_window_closed": true  // OR standing-curve has crystallized
        }
      },
      {
        "kind": "comment",
        "aggregate_subordinate_policy": {
          "trigger_age_days": 90,           // comments stay hot longer
          "trigger_min_count": 1000
        }
      },
      {
        "kind": "report",
        "aggregate_subordinate_policy": null  // reports never subordinate (governance evidence stays hot)
      }
    ]
  }
}
```

Time-based fallback (90 days) applies when manifest doesn't override. `null` policy means "never subordinate" (for governance-critical signal_kinds like `report` that must remain queryable indefinitely).

**B-6 resolution: balance checkpoint trigger.** Manifest-declared per `resource_classified_as` with row-count floor as universal fallback:

```jsonc
// elohim/sdk/domains/shefa/manifest.json
{
  "vocabulary_declarations": {
    "resource_classifications": [
      {
        "classification": "currency-USD",
        "checkpoint_policy": {
          "trigger_cadence_days": 90,    // quarterly checkpoint
          "trigger_event_count": 10000   // OR every 10k events
        }
      },
      {
        "classification": "stewarded-physical",
        "checkpoint_policy": null  // furniture rarely needs checkpoints
      }
    ]
  },
  "checkpoint_floor": {
    "event_count": 50000  // any Resource crossing 50k events triggers checkpoint regardless
  }
}
```

`null` policy = no scheduled checkpoint (low-activity Resources). Floor ensures runaway Resources can't dodge checkpointing entirely.

**Coordinator functions.**
```rust
create_checkpoint_commitment(subject_cid: Cid)
  // 1. Determine period_start (most recent prior checkpoint's period_end, or Resource creation)
  // 2. Iterate events in [period_start, now) — compute balance snapshot
  // 3. Build merkle root of covered event hashes
  // 4. Author Commitment(action="checkpoint", metadata=snapshot)
  // 5. Mark as fulfilled immediately

create_aggregate_subordinate_commitment(target_cid: Cid, signal_kind: String, window: TimeRange)
  // 1. Query FeedbackSignals on target_cid in window
  // 2. Compute aggregate metrics + merkle root
  // 3. Author Commitment(action="aggregate-subordinate", metadata=metrics)
  // 4. ReconcileController fans out to:
  //    - memory-lifecycle submerge for the aggregated signals
  //    - tiered-quilt quilt-demoted (custody-quilt, tier_floor=shelved)
  //    - update standing-curve to use aggregate metrics for queries on this window
```

**Validation rules.**

- `checkpoint` Commitment authored by Resource's current custodian OR an authorized elohim-agent with stewardship-commitment Attestation
- `aggregate-subordinate` Commitment authored by an elohim-agent with subscription to the relevant signal_kind namespace; manifest-declared trigger threshold must be met
- Both Commitments carry a `merkle_root` of the data they summarize; downstream queries can audit-verify by re-fetching the underlying entries and recomputing

**Read-path optimization (the actual speed win).**

```sql
-- Without checkpoint: full event-history scan
SELECT SUM(quantity_delta) FROM economic_events
WHERE provider = :account_cid OR receiver = :account_cid;  -- 182k rows

-- With checkpoint: snapshot + delta
WITH latest_checkpoint AS (
  SELECT (metadata_json::jsonb->'balance_snapshot'->>'quantity')::numeric AS snap_balance,
         (metadata_json::jsonb->>'period_end')::int AS period_end
  FROM commitments
  WHERE action = 'checkpoint' AND subject_cid = :account_cid
  ORDER BY period_end DESC LIMIT 1
)
SELECT
  COALESCE(lc.snap_balance, 0) + COALESCE(SUM(e.quantity_delta), 0) AS balance
FROM latest_checkpoint lc
LEFT JOIN economic_events e ON
  (e.provider = :account_cid OR e.receiver = :account_cid)
  AND e.observed_at > COALESCE(lc.period_end, 0);
```

The 10-year-deep query collapses to (snapshot + recent-quarter deltas) — usually <500 rows.

**Manifest declaration validated by D.10's gate.** All `resource_classified_as` checkpoint policies and `signal_kind` aggregate_subordinate policies are declared in pillar manifests; the vocabulary governance gate ensures the classifications and signal_kinds referenced actually exist in their respective whitelists.

**Touches:**
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — add `"checkpoint"` and `"aggregate-subordinate"` to `REA_ACTIONS` whitelist (per D.10 vocabulary governance)
- `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` — new coordinator functions `create_checkpoint_commitment`, `create_aggregate_subordinate_commitment`
- `elohim/elohim-storage/src/services/checkpoint_service.rs` — **new service** computing balance snapshots; consumed by `resource_state_service.rs` (D.4) for read-path optimization
- `elohim/elohim-storage/src/services/signal_aggregate_service.rs` — **new service** computing signal aggregates; coordinates with submerge fan-out via ReconcileController (D.3)
- `elohim/sdk/domains/*/manifest.json` — `checkpoint_policy` per resource_classification; `aggregate_subordinate_policy` per signal_kind; `checkpoint_floor` (universal fallback)

---

### D.13 Missing View Schemas + Enum Reconciliation (Gap 14) — Wave A prerequisite

**Motivation.** Phase 1 architectural composition found three concrete schema-layer gaps that produce silent cascade failures (per `feedback_schema_data_enum_drift_cascade`):

- **`economic-resource-view.schema.json` does not exist.** The Resource is one of the eight foundational primitives (Part A.3) and a load-bearing primitive across every application archetype, yet the JSON schema declaring its HTTP wire shape was never authored. Without it, the doorway projection shape is undefined, the schema-contract test (`schema_contract.rs`) cannot detect drift, and the TS codegen has no source for the `EconomicResourceView` type. Any sprint that adds a Resource HTTP route is building on undeclared ground.
- **`proofClass` enum drift in `attestation-view.schema.json`.** The view schema declares `proofClass: "witness | self-attest | audit-signature | computational"`. The validator (`attestation_validator.rs` Floor 8) and the canonical computation-attestation spec (`2026-05-01-computation-attestation-graduated-rigor-design.md`) use a different set: `witness | audit | proof | confirmation`. Any client validating the wire shape against the schema fails on attestations using validator-canonical class names. This is the exact 503/401 cascade shape the memory anchor warns about.
- **`forget-request` in `SIGNAL_KINDS` whitelist but not in `p2p/feedback-signal.schema.json` enum.** The Rust whitelist accepts it (Floor 1 passes); the p2p wire-layer JSON schema rejects it. A signal that validates on DHT fails on the libp2p wire layer.

These are not architecture decisions; they are schema-authoring debts. D.10 (vocabulary governance) catches future drift; D.13 closes the existing drift.

**Design — three coordinated schema authorings.**

**Authoring 1 — `economic-resource-view.schema.json`.** Author the missing schema per the 10 conventions at `elohim/sdk/schemas/v1/views/CONVENTIONS.md`. Required fields derived from the `EconomicResource` Rust struct + the planned additions from D.1 (`parent_epr_cid: Option<Cid>`), D.4 (`resource_classified_as: Vec<String>` after StewardedResource consolidation; addenda for `governed_by` and `data_quality` per operator decisions), and D.7 (`lifecycle_state: enum { active, subordinate, shelved, closed }`). Add the corresponding entry to `INTERFACE_FILES` in `elohim/sdk/schemas/scripts/codegen-ts.mjs` so the TS codegen picks it up. Add a contract test in `elohim/elohim-storage/tests/schema_contract.rs` to catch future drift.

**Authoring 2 — `proofClass` enum reconciliation.** Canonical values are `witness | audit | proof | confirmation` (per `2026-05-01-computation-attestation-graduated-rigor-design.md` and the validator). Update `attestation-view.schema.json` to match. Update any consumer of the prior enum values (search for `self-attest` and `audit-signature` and `computational` across the repo) to use the canonical names. D.10's CI gate prevents this from drifting again.

**Authoring 3 — `feedback-signal.schema.json` enum sync.** Add `forget-request` to the `signalKind` enum so it matches `SIGNAL_KINDS`. This is the minimal closure; the broader signal-kind expansion (`comment`, `endorse`, `react`, `report` for the Meta archetype) lands per D.18 (signal_class field) and the application-archetype Wave 2 dispatches that follow.

**Migration discipline.** Authored schemas ship with their contract tests in the same commit. The contract tests are the substrate-floor invariant: `cargo test schema_contract` must pass before any sprint that touches a view ships. D.10's CI gate is the structural backstop; the contract tests are the immediate proof.

**Touches:**
- `elohim/sdk/schemas/v1/views/economic-resource-view.schema.json` — **new file**; required shape per CONVENTIONS.md
- `elohim/sdk/schemas/scripts/codegen-ts.mjs` — extend `INTERFACE_FILES` to generate `EconomicResourceView` TypeScript type
- `elohim/sdk/schemas/v1/views/attestation-view.schema.json` — `proofClass` enum reconciliation to canonical 4-value set
- `elohim/sdk/schemas/v1/p2p/feedback-signal.schema.json` — `signalKind` enum: add `forget-request`
- `elohim/elohim-storage/tests/schema_contract.rs` — add `EconomicResourceView` contract test; verify `proofClass` enum match between validator and view; verify `signalKind` enum match between whitelist and wire schema
- `elohim/elohim-storage/src/views.rs` — extend or confirm `EconomicResourceView` Rust struct matches the new JSON schema (one or the other is the source of truth; the contract test fails the build if they diverge)

---

### D.14 Standing-Curve View + Policy Declaration (Gap 15) — Wave E

**Motivation.** Phase 1 A.7 found that `standing_scores` is queried as a SQL view/table by feed-ranking (Meta archetype), reach-arbitration (every primitive that respects reach), and graduation-evaluation (D.6 elohim-authoring pattern) — but the view definition logic is not documented anywhere. The standing-curve policy (decay rate, vouch recovery fraction, debit weights, update frequency) needs explicit manifest declaration so collectives can tune it without code changes, and so the substrate's nervous system (reach as earned per `project_social_reach_nervous_system`) has a concrete substrate-floor specification.

**Design — `standing_scores` view contract.**

Per D.18 signal_class isolation, the view returns per-(author, signal_class) tuples rather than a global standing:

```sql
CREATE VIEW standing_scores AS
WITH signal_aggregates AS (
    SELECT
        fs.signer_cid AS author_cid,
        fs.signal_class,
        SUM(
            CASE
                WHEN fs.standing_impact = 'credit-soft' THEN policy.credit_soft_weight
                WHEN fs.standing_impact = 'credit-firm' THEN policy.credit_firm_weight
                WHEN fs.standing_impact = 'debit-soft' THEN policy.debit_soft_weight
                WHEN fs.standing_impact = 'debit-firm' THEN policy.debit_firm_weight
                ELSE 0
            END
            * EXP(-policy.decay_rate_per_day * (NOW_EPOCH() - fs.observed_at) / 86400)
        ) AS raw_score,
        COUNT(*) FILTER (WHERE fs.signal_kind = 'vouch') * policy.vouch_recovery_fraction AS vouch_offset,
        MAX(fs.observed_at) AS most_recent_signal_at
    FROM feedback_signals fs
    JOIN standing_curve_policies policy ON policy.signal_class = fs.signal_class
    -- Optionally apply aggregate-subordinate shortcuts (D.12 interlock):
    --   if a signal_class window has been aggregate-subordinated, use the
    --   Commitment's aggregate_metrics instead of summing individual signals
    GROUP BY fs.signer_cid, fs.signal_class
)
SELECT
    author_cid,
    signal_class,
    raw_score + vouch_offset AS standing_score,
    most_recent_signal_at
FROM signal_aggregates;
```

The view is **derived only**; never written to directly. Services consume it as read-only.

**Manifest-declared standing-curve policy per signal_class.**

```jsonc
// elohim/sdk/domains/elohim/manifest.json (or per-pillar override)
{
  "standing_curve_policies": [
    {
      "signal_class": "care",
      "decay_rate_per_day": 0.01,
      "vouch_recovery_fraction": 0.5,
      "credit_soft_weight": 1,
      "credit_firm_weight": 5,
      "debit_soft_weight": -1,
      "debit_firm_weight": -10,
      "update_frequency": "eventual_60s"  // per E-1 below
    },
    {
      "signal_class": "compute",
      "decay_rate_per_day": 0.05,  // compute standing decays faster (recent performance more salient)
      "credit_firm_weight": 3,
      "debit_firm_weight": -8
    },
    {
      "signal_class": "governance",
      "decay_rate_per_day": 0.005,  // governance standing decays slower (cumulative)
      "credit_firm_weight": 10,
      "debit_firm_weight": -20
    },
    {
      "signal_class": "trust",
      "decay_rate_per_day": 0.001,  // trust nearly permanent
      "vouch_recovery_fraction": 1.0  // vouch fully offsets prior debits in trust class
    }
  ]
}
```

D.10's vocabulary governance gate validates that signal_class values referenced in standing_curve_policies match the SignalClass enum from D.18.

**E-1 resolution — eventual-consistency with 60-second staleness SLA.** Per operator lean: re-derive the view on a 60-second schedule rather than on every signal arrival. The trade-off (feed ranking with 60s standing-staleness vs. hub serialization at scale) favors hub-throughput; 60s standing-staleness is imperceptible to users; real-time was premature optimization on already-noisy signal.

```rust
// elohim/elohim-storage/src/services/standing_curve_service.rs (planned)
pub struct StandingCurveService {
    update_interval: Duration,  // 60s default from manifest
}

impl StandingCurveService {
    async fn refresh_standing_scores(&self) {
        // 1. Read latest feedback_signals delta since last refresh
        // 2. Apply aggregate-subordinate shortcuts (D.12) where windows already-aggregated
        // 3. Recompute standing_scores view by re-executing the policy formula
        // 4. Stamp the projection with refresh_timestamp
    }
}
```

Feed-ranking and reach-arbitration query the view, get scores stamped with their refresh_timestamp; consumers know freshness without needing to drive recomputation.

**Standing-stewardship-elohim.** Per D.6: a `standing-stewardship-elohim` agent specialization drives the periodic recompute, monitors for anomalies (signal-cluster attacks; standing-curve oscillations), and proposes manifest amendments when the curve isn't producing healthy network dynamics.

**Hub-scale efficiency.** Per Phase 1 A.7 concern: hub with 10k signals/day and 200+ active authors needs O(signals) refresh, not O(authors × signals). The 60s interval batches all signal arrivals into one recompute pass. With D.12 aggregate-subordinate handling old signals, the recompute scope is bounded by the active window.

**Interlocks.**

- **D.18 (signal_class field)**: per-class standing tuples are the output structure
- **D.12 (aggregate-subordinate)**: aggregated signal windows use the Commitment's aggregate_metrics as a shortcut in the view computation
- **D.6 (elohim-authoring)**: standing-stewardship-elohim is the agent specialization driving the recompute
- **D.10 (vocabulary governance)**: standing_curve_policies references signal_class values; validated by the gate
- **D.20 (Layered Commons)**: standing-curve influences reach-mutation authority (D.9 D-2 interlock — author's standing caps the max reach they can grant); this is the substrate's nervous system in action

**Touches:**
- `elohim/elohim-storage/src/db/views/standing_scores.sql` (planned) — the SQL view definition
- `elohim/elohim-storage/src/services/standing_curve_service.rs` (planned) — periodic recompute service with 60s default interval
- `elohim/sdk/domains/elohim/manifest.json` (and per-pillar overrides) — declare standing_curve_policies per signal_class
- `elohim/sdk/schemas/v1/views/standing-score-view.schema.json` (planned) — wire shape for the view
- `app/elohim-app/src/app/elohim/elohim-agents/standing-stewardship-elohim.service.ts` (planned) — agent specialization (D.6 interlock)
- D.18 interlock — signal_class enum determines view output shape
- D.12 interlock — aggregate-subordinate shortcuts in the view computation

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
