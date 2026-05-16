# Graph-Native Projection Substrate — Design Spec

**Status:** Design (approved, ready for plan)
**Date:** 2026-05-16
**Authors:** Matthew Dowell + Opus 4.7
**Pillar coupling:** cross-cutting — substrate beneath imagodei, lamad, shefa, mishpat, qahal; this sprint extends shefa + lamad as the first demonstrating domains
**Depends on:**
- `2026-04-21-elohim-core-graph-substrate-design.md` — master spec for the EPR-as-atom substrate; this spec is one of its phases (3.7 + 4 folded)
- `2026-05-01-light-up-the-graph-design.md` — Phase 3.5 signal-substrate sprint that this spec builds on
- existing `elohim-storage` projector module + diesel layer
- existing `epr_codec.rs` EprHead + EprRelationship + three-pillar contexts
- existing `app-manifest.schema.json` two-layer envelope/payload type system
- existing storage view-contract pattern (`elohim/sdk/schemas/v1/views/` → Rust struct → TS codegen → Angular adapter)

**Related:**
- `.claude/skills/p2p-design-gate/` — classification: this spec adds operational (C) projection entities, no new substrate-truth concerns
- `.claude/skills/epr-content-addressing/` — content-addressed link architecture
- Memory: `project_first_class_graph_pattern` — EPRs=nodes, couplings/memberships/delegations=edges
- Memory: `project_epr_substrate_vs_vf_graphql` — EPR codec/libp2p IS the graph primitive; VF-GraphQL is app-layer
- Memory: `project_principle_p1_reconciliation_controller` — storage as reconciliation controller
- Memory: `project_doorway_views_through_not_owned` — views served THROUGH a doorway, not OWNED
- Memory: `project_intelligence_scales_to_humans` — first revolution to scale TO human complexity, not flatten it

---

## Preamble: Relationship to the 2026-04-21 Master Spec

This spec **amends-by-extension** the 2026-04-21 elohim-core-graph-substrate master spec. The master spec remains the architectural north star and is not superseded; this spec is one of its phases (Phase 3.7 — graph-native projection target — folded with Phase 4 — GraphQL surface, first subgraph).

**One reframing**, called out explicitly so future readers don't trip:

The 2026-04-21 spec §15 non-goal reads: *"Building a new database engine. elohim-storage remains a projection/index service, not a graph database. Queries plan against indexes; there is no native graph query engine."*

That sentence is **reframed under Reading A**: CozoDB is the projection engine for graph-shaped data, not the source-of-truth authority. The DHT remains canonical (per P1 reconciliation controller). CozoDB is to graph relationships what iroh-sqlite is to relational data — a performant projection of substrate-canonical state, shaped for a particular access pattern. Storage is still the projector, not the authority.

The §15 phrase "no native graph query engine" was implicitly arguing against making elohim-storage itself the graph authority. Adding CozoDB-as-projection does not violate that — it makes storage's projections more powerful while keeping the DHT canonical.

All other 2026-04-21 spec sections (§1–§17) carry forward. This spec extends:
- **§8.3** "projection tables become materialized views over the EPR table" → projection targets are now multi-shape (relational + graph), both materialized from substrate events
- **§9** query surface → typed-REST-views and GraphQL coexist as first-class wire formats (REST preferred for materialized per-shape views; GraphQL preferred for cross-pillar federation queries)
- **§11** reference subgraph inventory → shefa goes first to land the social compute topology substrate; lamad rides along to prove the manifest-extension pattern across more than one domain

---

## 1. Problem

Elohim already has a content-addressed, three-pillar-coupled, edge-bearing atom (EprHead per `epr_codec.rs`). Storage already projects substrate state into 50+ relational view shapes via diesel+sqlite. Cartographer's foundation-closure work (2026-05-16) routed 12 deferred BDD scenarios across content + federation feature files into named follow-on destinations — among them, three "graph-native renderer" scenarios that cannot be solved by HTTP-via-doorway and require traversal-shaped data access.

But the protocol does not yet have a **graph-native projection layer**. Traversal queries decompose into SQL JOINs per hop; multi-hop neighborhood walks are not first-class; vector similarity has no slot; bitemporal version-chain walking is implicit at best. For record-shaped concerns (comments, ballots, posts, agreements) the relational projection is correct. For:

- **Learning maps** — prerequisite/teaches chains, mastery progression, multi-hop "what unlocks this"
- **Network topology** — steward graphs, peer graphs, household-collective edges with reach/trust attribution
- **Search** — combinatorial discovery; vector composed with graph proximity (slot, not yet implemented)
- **Contextual representation / progressive reveal** — the elohim agent at read time traversing "from where the reader stands, expand outward by reach + relevance + reading history"
- **Relational-shape semantics** — love maps between two contributors, bidirectional curriculum bridges, the elohim agent's understanding of its human's experience

…the relational projection is the wrong shape. These are MemPalace-shape queries — the same primitive that makes the agent team functional (wings + entity-relationship graph + temporal timeline + vector retrieval) is what humans need too. The protocol that *scales TO human complexity rather than flattening it* (per memory `project_intelligence_scales_to_humans`) needs that primitive at its core.

This spec introduces a graph-native projection target inside elohim-storage, alongside the existing relational projection, with manifest-driven extensibility for domain shapes. The DHT remains canonical; both projections are derived; the wire surface gains a new GraphQL endpoint alongside the existing typed-REST pattern.

---

## 2. Architectural Principles (carried forward + new)

Inherited from the 2026-04-21 master spec:
1. **Publish, do not own.** Elohim-core provides primitives; many parties steward overlapping subgraphs.
2. **Envelope / payload separation.** Protocol owns wire shape, enums, structural validators; domains own interpretation.
3. **ThreeLegCoupling is axiomatic.** Every substantive atom couples knowledge + value + governance.
4. **Schema-first is IoC.** JSON Schema authored first; Rust and TypeScript comply; schemas are themselves content-addressed atoms.
5. **No sovereignty — stewardship.** No party owns any part of the graph.

New for this spec:
6. **Projection over authority.** All read-time substrate engines (diesel relational + CozoDB graph) are projections of DHT-canonical state, rebuildable from canonical bytes. The DHT is the only authority.
7. **Dual-projection-shape.** Different access patterns warrant different projection shapes. Relational projection serves record-shaped queries; graph projection serves traversal-shaped queries. Both materialize from the same substrate events.
8. **Core ↔ manifest IoC for graph types.** Core ships universal graph primitives (node/edge relations, traversal rules); domain manifests declare additional edge types, indexes, and Datalog rules. Same extensibility pattern as content types and signal kinds.
9. **Reach is established at authoring time.** Per the 2026-05-01 spec's reach-earning gate, an EPR's reach is set at compose-time by the author's standing and the manifest's threshold. Read-time projection records but does not filter. Audit-trace is by walking the signed EPR envelope. AttentionMinding is a separate filter-side primitive (out of scope this sprint). Body access via `attestation_requirements` is the existing Layer 2 gate (out of scope this sprint).

---

## 3. Three-Tier Substrate Model

```
DHT (canonical, notarized, expensive)
   │
   ▼  reconciliation controller (P1)
PROJECTIONS (multiple shapes, per-peer, performant)
   ├─ iroh-sqlite / diesel  →  relational projection for record-shaped data
   └─ CozoDB                →  graph projection for traversal/topology/contextual reveal
                                 (and vector slot for future embeddings)
   │
   ▼  view-assembler
WIRE FORMATS (two surfaces, both first-class)
   ├─ Typed REST views      →  materialized per-shape projections (today's pattern, kept)
   └─ GraphQL surface       →  cross-manifest federation queries (Apollo Federation v2)
                                 (new this sprint, server-side only)
```

**Composition rule:** view builders are the single composition layer between the engines and the wire formats. Datalog is internal to CozoDB; raw diesel is internal to the relational projection. View builders consume both and emit typed Rust structs that serialize to the appropriate wire format.

---

## 4. Core Graph Schema

What core ships with — present the moment CozoDB initializes. Domain manifests add to this.

### 4.1 Node relations

```datalog
:create epr_node {
    cid: String =>          # primary key — the EprHead CID
    slug: String,           # stable content ID
    content_cid: String,    # IPLD link to body bytes
    version: Int,
    author_did: String?,    # nullable until W2 lands AgentPeerBinding
    updated_at: Validity,   # Cozo's bitemporal type — supports time-travel queries
    embedding: <F32; 768>?, # vector slot — pipeline deferred; schema present so future sprints land embeddings without migration
}
```

`Validity` is CozoDB's native bitemporal type. Adopting it now means version history and time-travel queries are first-class without writing the temporal layer ourselves.

### 4.2 Edge relation

```datalog
:create epr_edge {
    from_cid: String,
    to_cid: String,
    rel_type: String =>     # PREREQUISITE, TEACHES, STEWARDS, MEMBER_OF, ... — open vocabulary
    asserted_at: Validity,  # when this edge was observed
}
```

Open `rel_type` vocabulary by design. Core does not enumerate the legal set — manifests do. Core accepts any string; the projector enforces manifest-declared types at query-by-named-rule time (not at write time, per Section 6 forward-tolerance).

### 4.3 Three-pillar property relations

One per pillar, 1:1 with EprHead nodes, grounded in actual `EprHead` struct fields (verified against `epr_codec.rs`):

```datalog
:create epr_lamad {
    cid: String =>
    title: String,
    content_type: String,
    description: String?,
    content_format: String?,
    tags: [String],
}

:create epr_shefa {
    cid: String =>
    stewards: [String],      # DIDs
    allocations: [Float],
}

:create epr_qahal {
    cid: String =>
    reach: String?,          # reach lives in qahal — not lamad
    layer: String?,
    attestation_requirements: [String],
}
```

Pillar relations are separate from `epr_node` so domains that don't care about a pillar can ignore it, and pillar-specific indexes can be added without bloating the node relation.

### 4.4 Universal indexes

```datalog
::index create epr_edge:by_rel_type     { rel_type, from_cid }
::index create epr_edge:by_target       { to_cid, rel_type }
::index create epr_qahal:by_reach       { reach }
::index create epr_node:by_author       { author_did }
::index create epr_node:by_updated      { updated_at }
::index create epr_node:by_embedding    HNSW { fields: [embedding], dim: 768, ... }
```

The HNSW index slot is declared though the embedding pipeline is deferred. Near-zero cost when unused; lets future sprints land embeddings without schema migration.

### 4.5 Universal traversal primitives

Named Datalog rules registered at startup. Domain rules compose these:

```datalog
neighborhood[?to, ?hops] :=
    *epr_edge{from_cid: $start, to_cid: ?to},
    ?hops = 1
neighborhood[?to, ?hops] :=
    neighborhood[?via, ?prev_hops],
    *epr_edge{from_cid: ?via, to_cid: ?to},
    ?hops = ?prev_hops + 1,
    ?hops <= $max_hops

path[?from, ?to, ?via_rel_types] := ...
reach_filtered[?node, ?reach_floor] := ...
version_chain[?node] :=
    *epr_edge{from_cid: ?prev, to_cid: ?node, rel_type: "SUPERSEDES"}
```

---

## 5. Manifest Extension Contract

### 5.1 Extension surface in app-manifest.schema.json

Existing `app-manifest.schema.json` gains a new top-level `"graph"` section. One source of truth per domain.

```jsonc
{
  "name": "lamad",
  "version": "1.0.0",
  "contentTypes": { ... },         // existing
  "contentFormats": { ... },       // existing
  "renderers": { ... },            // existing
  "signals": { ... },              // existing
  "graph": {                       // NEW
    "edges": [
      { "type": "PREREQUISITE", "from": "EprHead", "to": "EprHead", "indexed": true, "directional": true, "description": "A must be mastered before B" },
      { "type": "TEACHES",      "from": "EprHead", "to": "EprHead", "indexed": true, "directional": true },
      { "type": "MASTERY_OF",   "from": "ContributorDID", "to": "EprHead", "weighted": true, "temporal": true }
    ],
    "nodes": [
      { "type": "MasteryRecord", "properties": { "contributor_did": "String", "concept_cid": "String", "level": "String", "attested_at": "Validity" } }
    ],
    "indexes": [
      { "name": "prereq_forward",  "on": "epr_edge", "where": "rel_type = 'PREREQUISITE'" },
      { "name": "prereq_backward", "on": "epr_edge", "where": "rel_type = 'PREREQUISITE'", "order_by": "to_cid" }
    ],
    "rules": [
      { "name": "prerequisite_chain", "datalog": "prerequisite_chain[?ancestor, ?node, ?depth] := ..." },
      { "name": "mastery_frontier",   "datalog": "..." }
    ]
  }
}
```

### 5.2 Registration-time validator (core enforces)

When a Manifest EPR is registered:
1. All declared edge `type` values are unique within the manifest and across `extends`'d chain
2. `from`/`to` types reference core node types (`EprHead`, `ContributorDID`) OR types declared in this manifest's `nodes` OR types declared in extended manifests
3. Declared indexes reference relations that exist
4. Datalog rule names are unique within the manifest and don't shadow core primitives (`neighborhood`, `path`, `reach_filtered`, `version_chain`)
5. Datalog rules parse cleanly through CozoDB's parser
6. No rule references undeclared relations

Failed validation = manifest registration rejected. Same hard-gate pattern as content-type schemas today.

### 5.3 Registration flow

```
1. Manifest EPR arrives at storage (PUT /api/v1/epr, kind=Manifest)
2. Existing validator stages: canonical CID, signature, coupling, payload-schema
3. NEW: graph extension validator (5.2 checks)
4. If valid: ManifestRegistry.register(manifest_cid):
   - register each edge type in vocabulary
   - :create relation for each declared node type
   - ::index create for each declared composite index
   - register named rule in rule library
5. GraphQL schema codegen pass runs; SDL is re-emitted
```

Registration is idempotent. Re-registering a different manifest CID at the same name triggers a supersedence projection per the 2026-04-21 spec §5.3.

### 5.4 What core ships vs what manifests provide

| Aspect | Core ships | Manifest provides |
|---|---|---|
| Node relations | `epr_node`, `epr_lamad`, `epr_shefa`, `epr_qahal` | Domain-specific node types (e.g., `MasteryRecord`, `Affinity`) |
| Edge relation | `epr_edge` (open vocabulary) | Specific `rel_type` declarations |
| Indexes | Universal (by_rel_type, by_target, by_reach, by_author, by_updated, HNSW slot) | Domain-specific composite indexes |
| Traversal primitives | `neighborhood`, `path`, `reach_filtered`, `version_chain` | Domain composite rules (`prerequisite_chain`, `household_topology`, `reciprocity_flow_to`) |
| Vector embedding | HNSW slot on `epr_node.embedding` | Embedding pipeline (deferred) |
| GraphQL schema | Universal types | Domain types + resolvers via codegen |

---

## 6. Projection Pipeline

### 6.1 Fan-out point

All EPR arrivals converge on the existing projector module. After this sprint, projector fans out to **both** diesel and CozoDB from the same event source.

Sources of EPR arrival (all unchanged from current behavior):
- `api/epr.rs::put_epr` — external HTTP arrival
- libp2p ingest — gossip + direct
- Holochain signal — DHT-notarized commit
- Local authoring — this peer publishes

### 6.2 Transaction model

**Per-target transactional, cross-target eventually consistent.** No distributed transactions between sqlite and CozoDB.

1. Diesel sink runs `conn.transaction(|tx| { ... })` for relational writes
2. CozoDB sink runs CozoDB's native transaction for graph writes
3. Projector runs them sequentially (relational first, then graph)
4. If graph write fails after relational succeeded: log + enqueue graph-only retry; reconciliation controller catches up eventually
5. If relational write fails: no graph write attempted; substrate replay retries both

Failure-mode invariant: a successful arrival means relational projection succeeded; graph projection may lag by retries but converges. The DHT remains canonical.

### 6.3 Idempotency

Every write is upsert-by-CID:
- Relational: `ON CONFLICT (cid) DO UPDATE SET ...`
- Graph: CozoDB `:put` is upsert by primary key
- Validity timestamps auto-track version timeline without overwriting history

Restart-safe, replay-safe, substrate-resync-safe by construction.

### 6.4 Edge writes — arrival-order tolerance

When an EprHead arrives with relationships, the projector writes one row to `epr_edge` per relationship. **Forward-tolerance of missing targets is required** — P2P arrival order is not guaranteed. If `to_cid` isn't in `epr_node` yet, the edge is written anyway. When the target arrives later, queries naturally pick it up.

CozoDB does not enforce referential integrity on `epr_edge.to_cid` — the substrate doesn't either.

### 6.5 Supersedence handling

Per 2026-04-21 spec §4.6, supersedence is an issuer-attested Claim EPR. When such a claim arrives, the projector writes:
- One row to relational `epr_supersedence` (existing)
- One row to `epr_edge` with `rel_type: "SUPERSEDES"`, from=predecessor, to=successor (new)

The `version_chain` core primitive walks SUPERSEDES edges.

### 6.6 Pillar context handling

Three-pillar contexts upserted alongside the node — one row per present pillar, none for absent pillars (e.g., a Claim EPR with no shefa skips `epr_shefa` write). Datalog queries handle absence with optional-match patterns.

### 6.7 Edge type validation at projection time

The projector does **not** reject edges whose `rel_type` isn't declared in any registered manifest. Manifests arrive over time (manifests-as-EPRs); rejecting unrecognized edges would break arrival-order tolerance.

Manifest registration controls:
- Which `rel_type` values are queryable via named domain rules
- Which edge types are indexed with composite indexes
- Which edge types appear in the GraphQL schema (codegen consumes registered manifests)

Unindexed, unmanifested edges are still in `epr_edge` and queryable via core primitives.

### 6.8 Backfill

```rust
projector::backfill_graph(&mut diesel_conn, &cozo_db, BackfillOpts {
    from_cid: None,           // or resume from checkpoint
    batch_size: 1000,
    progress_callback: ...,
})
```

- Walks `epr_atoms` in CID order
- Re-derives graph projection for each row
- Checkpoints progress (CID watermark)
- Idempotent

Backfill runs:
- **Once at first startup** after CozoDB integration
- **On-demand** at graph schema migrations
- **As reconciliation** when controller detects projection divergence

---

## 7. Query Surface

### 7.1 The three-tier query stack

```
INTERNAL                                          EXTERNAL (wire)
──────────                                        ──────────────
┌─────────────┐    ┌─────────────────┐         ┌──────────────────┐
│  Datalog    │ →  │ Typed Rust      │  ───→   │ REST views       │
│  (CozoDB)   │    │ query APIs      │         │ (typed JSON)     │
│             │    │ (view builders) │         └──────────────────┘
└─────────────┘    │ + diesel for    │
                   │ relational data │         ┌──────────────────┐
                   └────────┬────────┘         │ GraphQL surface  │
                            │              ──→ │ (async-graphql)  │
                            └──────────────    └──────────────────┘
```

### 7.2 Layer 1 — Datalog (internal)

Composed by Rust code in storage; never exposed externally; calls named rules with parameters:

```rust
let neighborhood = cozo.run_script(
    "?[to, hops] := neighborhood[to, hops], hops <= $max",
    &[("max", DataValue::from(2)), ("start", DataValue::from(cid))],
)?;
```

### 7.3 Layer 2 — Typed Rust view builders

The composition layer. One view builder per declared view shape. Lives in `elohim-storage/src/views/`:

```rust
pub fn resolved_atom(
    cid: &str,
    diesel: &mut DieselConn,
    cozo: &CozoDb,
    reader_ctx: &ReaderContext,
) -> Result<ResolvedAtomView, ViewError> {
    let node = cozo.lookup_node(cid)?;
    let lamad = cozo.lookup_lamad(cid)?;
    let shefa = cozo.lookup_shefa(cid)?;
    let qahal = cozo.lookup_qahal(cid)?;
    let author = diesel.find_contributor(&node.author_did)?;
    Ok(ResolvedAtomView { ... })  // reach not filtered at view layer
}
```

View builders are where graph + relational compose, where the wire shape is constructed in camelCase per the existing convention, and where D4's fetch-path decision lives concretely (content_cid + content_store fetch composed inside the builder; no libp2p `GetDocument` verb needed).

**Reach is NOT filtered at the view layer.** The view returns the atom's reach as a field on the typed view. Audit-trace is by walking the signed EPR envelope. AttentionMinding is a separate filter-side primitive (out of scope). Body access via `attestation_requirements` is the existing Layer 2 gate (out of scope).

### 7.4 Layer 3a — REST views (existing pattern, extended)

Per CLAUDE.md's View Schema Contract: JSON schema → Rust struct → schema contract test → TS codegen → Angular adapter.

This sprint adds the following new schemas:

| Schema | Powers | Backing |
|---|---|---|
| `resolved-atom-view.schema.json` | Scenario 1 (three-pillar popover; lamad-shaped consumer in frontend follow-on) | graph + diesel (author lookup) |
| `navigation-context-view.schema.json` | Scenario 2 (origin-context transfer) | graph (neighborhood primitive) |
| `atom-version-chain.schema.json` | Scenario 3 (supersedence affordance) | graph (version_chain primitive) |
| `topology-overview-view.schema.json` | Shefa topology rollup (new schema; no existing fits) | graph (household_topology + collective_topology rules) |

This sprint extends the following EXISTING schemas with graph backing in their view builders (schema shape unchanged):
- `peer-topology-view.schema.json` (graph-backed via household_topology rule)
- `reciprocity-view.schema.json` (graph-backed via reciprocity_flow_to rule)
- `my-cluster-view.schema.json` (graph-backed for peer/device topology)
- `resilience-snapshot-view.schema.json` (graph-backed for peer-mesh resilience walks)
- `distribution-summary.schema.json` + `distribution-details.schema.json` (graph-backed for content distribution graph)

### 7.5 Layer 3b — GraphQL surface (new, server-side only)

- **Server:** async-graphql in Rust, mounted at `POST /api/v1/graphql`
- **Schema codegen:** new pass walks registered manifests, emits Apollo Federation v2 subgraph SDL with `@key(fields: "cid")` on every EPR type
- **Resolvers:** default resolvers traverse `epr_edge` via Datalog; pillar resolvers read from `epr_lamad/shefa/qahal`; domain manifests can register custom resolvers (out of scope this sprint)
- **Endpoint:** single endpoint, both subgraphs (shefa + lamad) composed internally; multi-publisher federation deferred per 2026-04-21 Phase 6

**Demonstration queries** (acceptance gate — must both work):

```graphql
# Lamad: prerequisite chain traversal
query LearningNeighborhood($cid: String!) {
  eprHead(cid: $cid) {
    cid
    lamad { title contentType tags }
    qahal { reach }
    prerequisites(maxDepth: 3) {
      cid
      lamad { title }
    }
    teaches {
      cid
      lamad { title }
    }
  }
}

# Shefa: household topology walk
query HouseholdTopology($contributorDid: String!) {
  contributor(did: $contributorDid) {
    did
    household {
      members { did displayName }
      devices { id metrics }
      reciprocityInbound { from amount }
    }
  }
}
```

### 7.6 Layer 3c — Out of scope this sprint

| Capability | Why deferred | Lands in |
|---|---|---|
| Angular Apollo Client wiring | Server-side proves the surface; consumer wiring is focused frontend work | Frontend follow-on sprint |
| Subscriptions (WebSocket + libp2p) | 2026-04-21 Phase 6 | Phase 6 sprint |
| Multi-publisher federation | 2026-04-21 Phase 6 | Phase 6 sprint |
| Imagodei love-map / curriculum-bridge | Need imagodei manifest extension first | Future sprint |
| Elohim agent contextual-reveal | Application layer atop the substrate | After substrate proven |
| Embedding pipeline | Slot exists; pipeline is its own bite | Independent sprint |
| AttentionMinding integration | Separate primitive, separate sprint | Independent |

---

## 8. Sprint Scope (Option B — both manifests, backend only)

### 8.1 Substrate

- CozoDB embedded as Rust dependency in elohim-storage
- Sqlite as CozoDB's persistence backend (operational symmetry with diesel; separate file)
- Core graph schema (§4) — relations, indexes, traversal primitives
- HNSW vector slot declared (pipeline deferred)
- Projection pipeline extension (§6) — projector fans out to both targets
- Backfill command — `projector::backfill_graph`
- Manifest extension contract (§5) — `"graph"` section + registration-time validator

### 8.2 Shefa manifest extension (headline)

Edges:
- `STEWARDS` (DID → Resource)
- `VALUE_FLOW` (Resource → Resource, temporal+weighted)
- `MEMBER_OF` (DID → Household, DID → Collective)
- `RECIPROCATES_WITH` (DID → DID, temporal+weighted)
- `OPERATES_DEVICE` (DID → Device)

Rules:
- `household_topology` — walks MEMBER_OF (Household) edges + OPERATES_DEVICE for each member
- `collective_topology` — walks MEMBER_OF (Collective) edges
- `reciprocity_flow_to` — directional walk of RECIPROCATES_WITH with weights
- `value_flow_chain` — multi-hop VALUE_FLOW traversal

View builders:
- `peer_topology` → existing `peer-topology-view.schema.json` (graph-backed)
- `reciprocity` → existing `reciprocity-view.schema.json` (graph-backed)
- `cluster` → existing `my-cluster-view.schema.json` (graph-backed)
- `resilience_snapshot` → existing `resilience-snapshot-view.schema.json` (graph-backed)
- `distribution` → existing `distribution-*.schema.json` (graph-backed)
- `topology_overview` → new `topology-overview-view.schema.json` (consolidated rollup)

### 8.3 Lamad manifest extension (rides along)

Edges:
- `PREREQUISITE`, `TEACHES`, `CONTAINS`, `REFERENCES` (EprHead → EprHead)
- `MASTERY_OF` (DID → EprHead, temporal+weighted)
- `SUPERSEDES` (EprHead → EprHead — version chain)

Rules:
- `prerequisite_chain` — recursive PREREQUISITE walk
- `mastery_frontier` — concepts the contributor could now approach (prereqs satisfied, target not yet mastered)

View builders:
- `resolved_atom` → new `resolved-atom-view.schema.json`
- `navigation_context` → new `navigation-context-view.schema.json`
- `atom_version_chain` → new `atom-version-chain.schema.json`

### 8.4 REST surface

New routes:
- `GET /api/v1/views/resolved-atom/{cid}`
- `GET /api/v1/views/navigation-context/{cid}?origin={origin_cid}`
- `GET /api/v1/views/atom-version-chain/{cid}`
- `GET /api/v1/views/topology-overview/{did}`

Existing routes for peer-topology / reciprocity / cluster / resilience / distribution keep their wire shape; backing graduates to graph where the rule wins.

### 8.5 GraphQL surface (server-side only)

- async-graphql server mounted at `POST /api/v1/graphql`
- Schema codegen pass consuming registered manifests (core + shefa + lamad)
- Default + pillar resolvers
- Apollo Federation v2 subgraph spec compliance — validated via Apollo's spec test suite at build time

### 8.6 Out of scope (named explicitly)

| Item | Lands in |
|---|---|
| Angular Apollo Client wiring | Frontend follow-on sprint |
| 5 shefa topology @wip BDD scenarios (m1-matthew-timothy-delivery.feature) | Frontend follow-on sprint |
| 3 lamad @wip BDD scenarios (epr-content-addressing.feature:96/113/129) | Frontend follow-on sprint |
| 11 shefa resilience @wip scenarios (human-resilience.feature) | Future sprint (needs more frontend than just substrate) |
| Imagodei love-map manifest extension | After shefa+lamad land |
| Shefa+VF-GraphQL alignment | Natural follow-on after this sprint |
| Lit pivot for `<epr-popover>` | Frontend follow-on sprint |
| New BDD scenarios for graph-native semantics specifically | Deferred per operator decision |
| Multi-publisher GraphQL federation | 2026-04-21 Phase 6 |
| GraphQL subscriptions | 2026-04-21 Phase 6 |
| Embedding pipeline | Independent future sprint |

---

## 9. Definition of Done (Closing Conditions)

Sprint is done when all of the following are true simultaneously on the dev branch:

1. **Engine landed.** CozoDB is a dep of elohim-storage; `RUSTFLAGS="" cargo build --release` clean.
2. **Core schema applied.** Storage startup creates core relations + indexes; idempotent on restart.
3. **Projection working.** Integration test ingests an EPR via `api/epr.rs::put_epr` and verifies both diesel and CozoDB hold the projected state.
4. **Backfill working.** Integration test starts storage against a populated diesel DB with empty CozoDB; runs backfill; verifies graph state matches.
5. **Both manifests register.** Lamad and shefa manifests register cleanly with `"graph"` sections; declared rules become queryable; validator rejects malformed extensions.
6. **All 9 view builders work.** Integration tests cover 3 lamad (`resolved_atom`, `navigation_context`, `atom_version_chain`) + 6 shefa (`peer_topology`, `reciprocity`, `cluster`, `resilience_snapshot`, `distribution`, `topology_overview`) view builders, proving graph-backed views return expected shapes for representative fixture data. "Graph-backed" means the builder composes Datalog queries against CozoDB with diesel reads where relational data is needed — not that all data comes from graph.
7. **GraphQL demonstration queries work.** Both `LearningNeighborhood` (lamad) and `HouseholdTopology` (shefa) queries return expected shape via `POST /api/v1/graphql`; generated SDL validates as Apollo Federation v2 subgraph spec.
8. **No relational regressions.** All existing storage tests pass; existing REST views unchanged in wire shape.
9. **Pre-push hooks pass.** `pnpm run schema:check-dna`, `pnpm run schema:validate`, `pnpm run lint`, `cargo clippy -- -D warnings`, `cargo fmt --check`.
10. **CI green on orchestrator pipeline** for elohim-storage + elohim-app + DNA touchpoints (DNA changes not expected; if they happen, sweettest on Jenkins per `feedback_shift_measure_jenkins`).
11. **No @wip BDD scenarios lift.** This is backend-only; frontend follow-on lifts both lamad and shefa sets.

---

## 10. Risks and Mitigations

### Engine risks

| Risk | Mitigation |
|---|---|
| CozoDB production-maturity unknown | Pin known-good version; integration-test against fixture loads; backfill is rebuild path; "graph disabled, relational only" config flag as escape hatch; Apache-2.0 source-available |
| CozoDB + diesel both on sqlite | Separate sqlite files; no shared connection pool; smoke test at startup |
| Datalog learning curve | Named primitives keep most query composition behind typed Rust APIs; `docs/datalog-primer.md` for the team; SQL-equivalent comments on rules where they exist |
| Performance at scale unknown | Benchmark suite in sprint (10K/100K/1M atoms; edge fanout 5/20; measure neighborhood/path/version_chain at depth 2-3); document baselines; engine is replaceable since it's projection layer |
| Backfill duration on large existing populations | Batched + CID-watermark resume; runs async after startup; system functional with partial backfill |

### Schema / migration risks

| Risk | Mitigation |
|---|---|
| Graph schema evolution | Backfill is the migration tool — drop relations, recreate, re-project; `Validity` preserves history within version |
| Manifest validation too strict or too loose | Open `rel_type` vocabulary; strict on rule-name shadowing of core; strict on type references; lenient on adding new rules |
| Lamad + shefa edge type collisions | Names scoped to manifest namespace internally; GraphQL emits prefixed types; validator catches unscoped collisions |

### Wire format risks

| Risk | Mitigation |
|---|---|
| GraphQL codegen produces invalid Apollo Federation v2 SDL | Validate against Apollo's spec test suite at build time; reject on failure |
| REST view backwards-compatibility | This sprint ADDS schemas; doesn't modify existing ones |
| GraphQL endpoint expensive queries vector | Per-query timeout (default 5s); max depth limit (default 6); per-rule cost annotation deferred (shape supports it) |

### Projection consistency risks

| Risk | Mitigation |
|---|---|
| Graph projection lags relational on partial failure | Reconciliation controller per P1 retries; DHT canonical; converges eventually; documented SLA |
| Edge arrives before target | By design (forward-tolerance); documented in view builder contracts |
| Manifest registers after edges of its types already written | Rules read live `epr_edge`; retroactively queryable without backfill |

### Process / execution risks

| Risk | Mitigation |
|---|---|
| Subagent scope creep (per `feedback_subagent_scope_guardrails`) | Tasks scoped to single-file/module; explicit version-forbid + scope-creep-forbid in dispatch; no git-revert authority |
| Cascade-hidden test surface (per `feedback_cascade_halt_masks_failures`) | Budget extra iterations; track "scenarios green / scenarios actually run" not raw count |
| Diesel migration timestamp collision (per `feedback_diesel_migration_timestamp_collision`) | Manifest-driven schema changes go in CozoDB, not diesel — sidesteps |
| Schema-data enum drift (per `feedback_schema_data_enum_drift_cascade`) | schema:validate + schema:check-dna gate on every shefa/lamad manifest commit |
| DNA touch needs sweettest (per `feedback_shift_measure_jenkins`) | This sprint shouldn't touch any DNA; confirm at planning; if forced, route to Jenkins |
| Cargo target pool pressure (per `feedback_multi_agent_pvc_pacing`) | CozoDB adds compile time; CARGO_TARGET_DIR per-workspace per `feedback_cargo_target_dir_for_native_builds` |

### Spec relationship risk

| Risk | Mitigation |
|---|---|
| Future readers of 2026-04-21 master spec hit §15 "no native graph query engine" | Preamble of THIS spec addresses explicitly; companion note appended to 2026-04-21 spec routing to this one |

### Honest unknowns

| Unknown | Disposition |
|---|---|
| HNSW vector index performance at scale if embedding pipeline doesn't land within 2-3 sprints | Slot has near-zero cost when unused; re-evaluate at embedding-pipeline kickoff; drop via backfill if needed |

---

## 11. P2P Design Gate Output

Per `.claude/skills/p2p-design-gate/` — classification for new entities introduced.

### Entity: CozoDB graph projection (epr_node, epr_edge, epr_lamad, epr_shefa, epr_qahal, plus manifest-declared node relations)

- **Classification:** Operational (C) — projection of substrate truth, rebuildable from canonical bytes via backfill
- **Address:** N/A — internal storage primary keys derived from substrate CIDs
- **Source of truth:** DHT (per Reading A — CozoDB is projection, not authority)
- **Reconstruction strategy:** `projector::backfill_graph` from `epr_atoms` relational table; ultimately from substrate canonical bytes
- **Anti-pattern check:** ✓ Not source-of-truth; ✓ identity derived from CID; ✓ not HTTP-route-first; ✓ no new DHT entry types; ✓ no new identity-bearing entities

### Entity: Graph rule registrations in ManifestRegistry

- **Classification:** Operational (C) — derived from Manifest EPRs (which are Notarized A per 2026-04-21 spec)
- **Source of truth:** registered Manifest EPRs
- **Reconstruction:** re-register from manifests at startup

### Entity: New view shapes (ResolvedAtomView, NavigationContextView, AtomVersionChain, TopologyOverview)

- **Classification:** Operational (C) — wire-format projections
- **Source of truth:** composed from substrate by view builders
- **Reconstruction:** regenerate on each query

### Design constraints discovered

1. **No new DHT entry types added.** This sprint is purely projection-layer. Lamad ~73/100 and Mishpat ~11/100 entry-type headroom preserved.
2. **No new identity-bearing entities.** All identity flows from existing substrate (DIDs from imagodei, CIDs from EPR atoms).
3. **No new HTTP-route-first design.** New REST routes (`/api/v1/views/...`) project from view builders; route shapes follow existing convention.
4. **Schema-first IoC honored.** New view schemas (`resolved-atom-view`, `navigation-context-view`, `atom-version-chain`, `topology-overview-view`) authored first in `elohim/sdk/schemas/v1/views/`; Rust structs comply; TS codegen flows.

---

## 12. What Stays Unchanged

- Existing relational projection (diesel + sqlite) — full current scope
- Existing 50+ REST view schemas — unchanged in wire shape (5 get new graph-backing, schemas don't change)
- HTTP view contract pattern (schema → Rust struct → schema contract test → TS codegen → Angular adapter)
- DHT/libp2p substrate — graph projection is read-time consumer, not substrate participant
- Doorway role — single-target dispatch, projection of storage views
- Existing validator stages from 2026-04-21 §7 — all four still gate projection
- Agent hooks from 2026-04-21 §10 — graph projection runs between gate and signal
- Content-store + erasure-coded blob handling — graph stores body CID references only
- Existing Holochain DNAs — remain notarization authorities; this sprint shouldn't touch any
- Existing storage-client-ts adapters — unchanged
- Existing Angular adapter pattern — unchanged (no Angular work this sprint)
- Reach-earning gate from 2026-05-01 spec — unchanged; reach established at authoring, not read
- AttentionMinding — untouched (separate primitive, separate sprint)
- Layer-2 body access via `attestation_requirements` — untouched

---

## 13. Open Questions Deferred

- **Embedding pipeline.** Model choice, compute cadence, refresh policy. Slot is declared in core schema; pipeline is its own sprint.
- **Per-rule cost annotation in manifests.** GraphQL query cost limiting — shape supports it; not implemented this sprint.
- **Custom resolvers in domain manifests.** Manifest registry supports declaration; only default resolvers implemented this sprint.
- **Graph projection observability.** Reconciliation lag metrics, query latency percentiles, backfill progress dashboards. Out of scope; future sprint.
- **CozoDB query optimization tooling.** Datalog query plan visualization, slow-query log. Out of scope; future sprint.

---

## 14. References

- [2026-04-21 Elohim-Core Graph Substrate Design](2026-04-21-elohim-core-graph-substrate-design.md) — master spec; this spec is its Phase 3.7+4
- [2026-05-01 Light Up the Graph Design](2026-05-01-light-up-the-graph-design.md) — Phase 3.5 signal-substrate sprint
- [2026-04-18 Experience Story EPR Design](2026-04-18-experience-story-epr-design.md) — EPR atom prior art
- [2026-04-23 EPR Phase 2C Libp2p Federation Design](2026-04-23-epr-phase-2c-libp2p-federation-design.md)
- [2026-05-16 EPR WIP Disposition](../plans/2026-05-16-epr-wip-disposition.md) — routes the 3 graph-native scenarios this sprint addresses

### Memory citations

- `project_principle_p1_reconciliation_controller` — storage as reconciliation controller
- `project_first_class_graph_pattern` — EPRs=nodes, couplings=edges
- `project_epr_substrate_vs_vf_graphql` — EPR codec is the graph primitive
- `project_doorway_views_through_not_owned` — views THROUGH a doorway, not OWNED
- `project_doorway_single_target_no_fanout` — single-target dispatch
- `project_three_layer_truth_model` — DHT notary, libp2p data-ops, doorway web2 projection
- `project_intelligence_scales_to_humans` — protocol scales TO complexity
- `project_design_for_a_generation_no_shortcuts` — long-horizon discipline
- `feedback_shift_measure_jenkins` — sweettest validation venue
- `feedback_cargo_target_dir_for_native_builds` — CARGO_TARGET_DIR per workspace
- `feedback_subagent_scope_guardrails` — dispatch discipline
- `feedback_cascade_halt_masks_failures` — iteration budgeting
- `feedback_diesel_migration_timestamp_collision` — diesel migration hygiene
- `feedback_schema_data_enum_drift_cascade` — schema validation gates
- `feedback_a2o_narrative_is_opus_work` — narrative-preservation discipline for any a2o touch

---

## Appendix A — Decisions Incorporated

- **A-1** Reading A: CozoDB is projection engine, not authority; preserves 2026-04-21 §15 spirit (no source-of-truth graph engine) while enabling native traversal in projection
- **A-2** Dual-projection-shape: relational AND graph projection from same substrate events
- **A-3** Core ↔ manifest IoC for graph types: core ships primitives; manifests declare domain edges/rules
- **A-4** CozoDB as the engine choice (over Oxigraph, Kuzu, petgraph+custom); justification: Apache-2.0 license, embeddable Rust library, hybrid relational+graph+vector via Datalog, sqlite backend for operational symmetry
- **A-5** REST views AND GraphQL coexist as first-class wire formats; REST for materialized per-shape views; GraphQL for cross-pillar federation
- **A-6** Sprint Option B: both shefa AND lamad manifests extended this sprint; backend only; no @wip BDD scenarios lift; frontend follow-on lifts both sets
- **A-7** Shefa is headline (social compute substrate); lamad rides along (manifest pattern coverage + future popover work unblocked)
- **A-8** Reach established at authoring time, not read time; view builders return reach as data, do not filter; AttentionMinding is separate filter-side primitive (out of scope)
- **A-9** Validity bitemporal type used throughout (`updated_at`, `asserted_at`) for first-class temporal queries
- **A-10** HNSW vector slot declared though embedding pipeline deferred — schema-level commitment, deferred operational commitment
- **A-11** Edge arrival-order tolerance (forward-tolerance of missing targets); manifest-registration tolerance (edges with unmanifested types are written, queryable via core primitives, not via named rules until manifest arrives)
- **A-12** Per-target transactions, eventually-consistent cross-target; no distributed transactions sqlite ↔ CozoDB

## Appendix B — Glossary

| Term | Meaning |
|---|---|
| Graph-native | Storage layer treats edges and traversal as first-class, not as JOINs over relational rows |
| Dual-projection-shape | Two parallel projection targets from same substrate events: relational (diesel) + graph (CozoDB) |
| CozoDB | Embedded graph + vector + relational database engine via Datalog; Apache-2.0; sqlite-backed in our config |
| Datalog | Declarative recursive query language; CozoDB's native query format |
| Validity | CozoDB's bitemporal type; tracks transaction time + valid time |
| HNSW | Hierarchical Navigable Small Worlds — vector index algorithm; CozoDB native |
| Forward-tolerance | Property: edge with `to_cid` not in `epr_node` is still written; query resolves when target arrives |
| Manifest extension | Domain manifest declaring `"graph"` section with edges/nodes/indexes/rules |
| View builder | Typed Rust function composing Datalog + diesel queries into a typed wire shape |
| Apollo Federation v2 subgraph | Wire contract spec for composable GraphQL subgraphs; our generated SDL conforms |
| Reading A | Reframing of 2026-04-21 §15: CozoDB is projection engine, not source-of-truth; preserves "no native graph query engine" spirit |
| Option B | Sprint decomposition: both shefa AND lamad manifests extended; backend only |
| Three-pillar coupling | Knowledge (lamad) + value (shefa) + governance (qahal) attached to every substantive atom |
