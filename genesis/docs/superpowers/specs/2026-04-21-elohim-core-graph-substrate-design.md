# Elohim-Core Graph Substrate — Design Spec

**Status:** Design (brainstorm complete, awaiting user review)
**Date:** 2026-04-21
**Authors:** Matthew Dowell + Opus 4.7
**Pillar coupling:** cross-cutting — foundational substrate under imagodei, lamad, shefa, mishpat, qahal; demonstrated by avodah + doorway
**Depends on:** existing `elohim/sdk/` two-layer type system (envelope/payload); existing `app-manifest.schema.json` ThreeLegCoupling; existing ContentNode + EPR (EntityPortalReference) address model; existing elohim-storage diesel layer; existing `epr-resolver.service.ts`
**Related:**
- `genesis/docs/superpowers/specs/2026-04-18-experience-story-epr-design.md` — prior art for EPR-shaped content atoms; this spec generalizes its pattern
- `elohim/sdk/CLAUDE.md` — capture test as SDK boundary rule
- `.claude/skills/p2p-design-gate/` — A / A2 / B / B2 / C classification
- `.claude/skills/epr-content-addressing/` — content-addressed link architecture
- `.claude/skills/rea-economics/` — REA/VF vocabulary alignment

---

## 1. Problem

Elohim already has the pieces of a graph substrate: content addressing, signed entries, a two-layer envelope/payload type system, the ThreeLegCoupling axiom, a manifest-driven vocabulary, and a signal harness that refuses value-blind interactions. What it does not yet have is a **single protocol-level atom** that unifies those pieces into a publishable unit, nor a **graph surface** (traversal, discovery, federation) that lets third-party apps build on the substrate without owning it.

Pure-Holochain platforms (e.g., R&O) solve small-domain coordination well but cannot carry ecosystem-scale knowledge, value, and governance graphs — the DHT is too slow for content, schema evolution requires network resets, and federation is group-scoped. Elohim's split architecture (Holochain for notarization, elohim-storage + libp2p for content durability) already addresses performance. This spec addresses the remaining gap: **how apps publish graphs into the substrate and query across them, without anyone owning the graph**.

The target is a 20-year primitive: a publish model that survives framework churn, respects P2P, enforces capture-resistance by atom structure, and gives any third-party app first-class graph capability by building on elohim-core.

---

## 2. Architectural principles (inherited from existing work)

1. **Publish, do not own.** Elohim-core provides primitives — content addressing, signed claims, schema registry, index, traversal helpers. Many parties steward overlapping subgraphs that compose by agreement. There is no single logical graph under central authority.
2. **Envelope / payload separation.** Protocol owns wire shape, enums, structural validators. Domains own interpretation. The split is the atom's primary structure, not a codegen convention.
3. **ThreeLegCoupling is axiomatic.** Every substantive atom is required to couple knowledge + value + governance. Enforced at validation time. No value-blind content. No governance-free content. No unobserved claims.
4. **Capture test as the boundary rule.** "Could this capability be captured at scale for rent extraction?" Yes → protocol primitive (envelope). No → domain interpretation (payload).
5. **Schema-first is IoC.** For any wire contract, the JSON Schema is authored first; Rust and TypeScript comply. Extended here: the schema *itself* is a content-addressed atom in the graph.
6. **No sovereignty — stewardship instead.** No party owns any part of the graph. Stewardship is scoped, accountable, auditable, forkable.

---

## 3. Architectural frame: publish-with-batteries

**Elohim-core publishes six primitives** (the substrate):

1. **Content addressing** — every atom has intrinsic CID identity
2. **EPR atom** — the unit of published knowledge (§4)
3. **Manifest graph** — typed namespaces for composable vocabularies (§5)
4. **Subgraph publishing** — discovery + indexing + federation (§6)
5. **Proof & validation** — signature + coupling + payload schema (§7)
6. **Agent hooks** — sense-respond gates on acceptance (§10)

**Elohim-core ships five reference subgraphs** (the pillars — the capture-resistance obligation surface):

| Pillar | Dimension | Captures |
|---|---|---|
| imagodei | Identity | agent, delegation, attestation |
| lamad | Story | concept, path, mastery, narrative |
| shefa | Value | commitment, event, fulfillment, resource |
| mishpat | Governance | consent, authority grant, adjudication |
| qahal | Community | affinity, trust, group membership |

**Plus two process-demonstrator subgraphs** (not pillars — they demonstrate protocol-as-process):

- **avodah** — work/contribution flows as first-class protocol participants; the protocol watching itself work
- **doorway** — web2 projection of the protocol

Third-party apps are not required to implement avodah or doorway. They **are** required to participate in the five pillars — co-present knowledge, value, and governance coupling on every substantive atom. This is the anti-capture constraint.

---

## 4. The EPR atom

An **EPR** (EntityPortalReference — existing elohim vocabulary) is the unit of published knowledge in the substrate. Every ContentNode, every Agent, every Claim, every EconomicEvent, every Manifest is an EPR. All share a single envelope shape; each has a kind-specific payload.

### 4.1 Shape

```
EPR {
  // ── Envelope (protocol-owned, capture-resistant, type-uniform across all kinds) ──
  cid:            CID               // self-derived: hash(canonical-cbor(envelope-ex-proof + payload))
  kind:           EPRKind           // Content | Agent | Manifest | Claim | Observation |
                                    // EconomicEvent | Commitment | Attestation | Delegation | ...
  schemaRef:      CID               // CID of a Manifest EPR; payload validated against manifest's content-type schema
  schemaKey:      string            // content-type name within the referenced manifest
  reach:          Reach             // commons | community | collective | steward | private
  coupling: {                       // ThreeLegCoupling attestation refs
    knowledge:    CID?              // CID of a Claim-EPR binding to knowledge graph
    value:        CID?              // CID of an EconomicEvent-EPR (flow triggered/represented)
    governance:   CID?              // CID of a Governance-EPR (reach + model + signal approval)
  }
  claims:         CID[]             // outcome claims this EPR asserts (Claim-EPR refs)
  supersedes:     CID?              // prior version if this is a revision
  supersededBy:   CID?              // forward pointer when issuer has attested a successor
  issuedAt:       RFC3339           // UTC timestamp, included in canonical bytes
  proof:          Signature         // signer AgentCid + sig over canonical-cbor(envelope-ex-proof + payload)

  // ── Payload (domain-owned, schema-validated per kind + schemaRef) ──
  payload:        bytes             // canonical-cbor encoded, shape dictated by the referenced manifest
}
```

### 4.2 EPR kinds

| Kind | Payload (examples) | Required coupling |
|---|---|---|
| `Content` | `{ metadata: PathMetadata, body: EprCompositeBody }` | knowledge + value + governance |
| `Claim` | `{ asserts, contradictedBy, validityHorizon, leg, subject }` | knowledge *or* governance leg |
| `Observation` | `{ polarity, evidenceCid, aboutClaim }` | knowledge leg (backlink to Claim) |
| `EconomicEvent` | VF-shaped event | value leg |
| `Commitment` | VF-shaped commitment | value + governance |
| `Attestation` | `{ attests, aboutCid, context }` | governance |
| `Delegation` | `{ delegator, delegate, scope, expiration }` | governance |
| `Agent` | `{ humanCid, capabilities, delegations }` | governance (self-describing) |
| `Manifest` | vocabulary declaration (full app-manifest.schema.json payload) | governance (self-describing) |

Kind-specific required-coupling rules are declared in the protocol-level enum (`coupling-requirements.schema.json`, new). A malformed EPR — one missing a required coupling ref for its kind — is rejected by the validator before any app logic sees it.

### 4.3 Canonical serialization

All canonical bytes use **deterministic CBOR** (RFC 8949 §4.2.1, "Core Deterministic Encoding Requirements"):

- Map keys sorted lexicographically
- Integers in shortest form
- Floats in shortest form that round-trips
- Indefinite-length items forbidden
- UTF-8 strings, no duplicate keys

The canonical byte string includes every envelope field **except** `cid` (self-derived), `proof` (applied after), and `supersededBy` (forward pointer, attested later — see §4.6). The payload is canonical-CBOR-encoded before being included.

`supersededBy` is a **derived field** at the struct level — it is served to readers from the `epr_supersedence` index (§8.1), not stored in canonical bytes. This is why two agents re-deriving an EPR's CID get identical results regardless of whether the EPR has since been superseded.

### 4.4 CID derivation

```
cid = multihash(
  sha-256(
    canonical-cbor(envelope-ex-proof-ex-supersededBy ++ payload)
  )
)
encoded as CIDv1, codec = 0x71 (dag-cbor)
```

Same content → same CID across languages and implementations.

### 4.5 Proof

`proof` is a detached signature over the canonical bytes, using the issuer agent's key material. Signing scheme: **Ed25519** (matches Holochain and most P2P stacks). The proof carries:

```
Signature {
  signer:    CID          // CID of the issuer's Agent EPR
  algorithm: "ed25519"
  signature: bytes
}
```

Verifying a proof = dereference the Agent EPR → extract public key → verify over canonical bytes.

### 4.6 supersededBy handling

`supersededBy` is the issuer's after-the-fact attestation that a later EPR replaces this one. It is **not** part of the original canonical bytes. An issuer publishes a `Supersedence` Claim-EPR whose payload is `{ predecessor: CID, successor: CID }`, signs it, and elohim-storage indexes it to serve `supersededBy` pointer-following transparently. Readers can follow chains both ways.

---

## 5. The manifest graph

### 5.1 Manifests are EPRs

A Manifest EPR has `kind: Manifest` and a payload conforming to `app-manifest.schema.json`. Its CID is the canonical reference for "the vocabulary declared by this app at this version." Manifests are immutable; new versions create new Manifest EPRs that declare `supersedes` on the prior version.

### 5.2 schemaRef resolution

Every non-Manifest EPR carries `schemaRef: CID` + `schemaKey: string`. Resolution:

1. Fetch the Manifest EPR by `schemaRef` CID from the substrate
2. Look up `schemaKey` in the manifest's `vocabulary.contentTypes` (or `contentFormats` for format schemas)
3. Retrieve the JSON Schema for the payload
4. Validate payload bytes against schema

Every app discovering a new EPR can resolve its schema by graph traversal. **There is no central schema registry.** Schemas are data in the graph, addressed by CID, versioned by supersedence.

### 5.3 Vocabulary composition

Manifests can declare:

- `extends: CID[]` — inherits content types, formats, relationships, signals from parent manifests (e.g., lamad extends imagodei's `Agent`)
- `imports: { alias: CID }` — references another manifest's vocabulary under a namespace alias (e.g., `{ vf: cid(vf-graphql-manifest) }`)
- `native` types — this manifest's own vocabulary

Composition is resolved at validation time by traversing the extends/imports chain. Circular composition is rejected. Version pinning is automatic because schemaRefs are CIDs — a manifest extending `lamad@v1.2.3` is extending an immutable CID; upgrading lamad requires a new manifest CID, preserving old consumer behavior.

### 5.4 VF-GraphQL as one manifest among many

The ValueFlows vocabulary is published as a standard Manifest EPR at a well-known (but forkable) CID. Shefa's native manifest extends it:

```yaml
name: shefa
extends:
  - <cid-of-vf-graphql-manifest>
  - <cid-of-imagodei-manifest>
native:
  contentTypes:
    StewardshipContract:
      extends: vf:Agreement
      ...
```

R&O's `hrea.dna` vocabulary is likewise publishable as a Manifest EPR. R&O's EconomicEvent entries become EPRs referencing R&O's manifest CID. Elohim queries can traverse them transparently — same atom shape, same CID resolution, same federation.

---

## 6. Subgraph publishing

### 6.1 Subgraph manifests

A `SubgraphManifest` EPR (a specific Manifest subtype) declares:

```yaml
kind: Manifest
payload:
  type: subgraph
  name: "shefa-economic-flows"
  publisher: <agent-cid>
  schemaRef: <manifest-cid>        # which vocabulary this subgraph publishes
  indexes:
    - by: [kind, schemaKey, reach]
    - by: [coupling.value]
    - by: [coupling.governance]
  endpoints:
    graphql: "https://..."         # optional — if publisher offers hosted surface
    libp2p:  "/elohim/subgraph/1.0.0"  # peer protocol for P2P query
  signedBy: <agent-cid>
```

Publishers announce their subgraph manifests via:

1. **DHT registration** — the subgraph manifest CID is a notarized DHT entry (lightweight, just the CID + publisher attestation)
2. **libp2p gossip** — periodic announce over a topic channel

Discovery is decentralized — any peer can list known subgraphs and query them.

### 6.2 Indexes

A publisher maintains indexes over its subgraph. At minimum:

- `(kind, schemaKey)` — enumerate EPRs of a given type
- `coupling.knowledge`, `coupling.value`, `coupling.governance` — traverse by coupling dimension
- `supersedes` / `supersededBy` — version chains
- Kind-specific payload indexes (declared in the manifest) — e.g., shefa might index EconomicEvents by resourceConformsTo

Indexes live in elohim-storage (diesel tables). They are **projections**, not source of truth — rebuildable from canonical bytes at any time.

### 6.3 Federation

Federation is a query pattern, not a central service. A **federated resolver** accepts a query that references EPRs across multiple subgraphs. It:

1. Plans the query by following schemaRef CIDs
2. Dispatches sub-queries to each subgraph's endpoint (graphql or libp2p)
3. Joins results at coupling refs (which are CIDs, uniformly resolvable)

The resolver can run in elohim-storage (default), in doorway (for web2 clients), or in a peer's own node (for P2P clients). It is stateless and cacheable.

---

## 7. Proof & validation

Every EPR entering elohim-storage passes a four-stage validator:

1. **Canonicalization check** — reserialize envelope+payload and verify CID matches declared `cid` field
2. **Signature verification** — dereference signer Agent EPR, verify signature over canonical bytes
3. **Coupling check** — for the EPR's kind, verify all required coupling refs are present and resolvable
4. **Payload schema check** — resolve schemaRef, fetch content-type schema, validate payload JSON against it

Validation failures reject the EPR. Elohim-storage never persists an invalid EPR. Agent hooks (§10) run after all four stages pass but before index commit.

---

## 8. Storage layout (elohim-storage)

### 8.1 Canonical storage

EPRs are stored in two diesel tables:

```sql
CREATE TABLE epr_atoms (
  cid           TEXT PRIMARY KEY,           -- CIDv1 string
  kind          TEXT NOT NULL,
  schema_ref    TEXT NOT NULL,
  schema_key    TEXT NOT NULL,
  reach         TEXT NOT NULL,
  issued_at     TIMESTAMP NOT NULL,
  signer_cid    TEXT NOT NULL,
  supersedes    TEXT,                       -- FK to epr_atoms.cid, nullable
  canonical_bytes BLOB NOT NULL,            -- the full canonical-cbor bytes
  -- indexes on: kind, schema_ref, reach, signer_cid, supersedes
);

CREATE TABLE epr_coupling (
  epr_cid       TEXT NOT NULL REFERENCES epr_atoms(cid),
  leg           TEXT NOT NULL,              -- 'knowledge' | 'value' | 'governance'
  target_cid    TEXT NOT NULL,              -- CID of the coupling-target EPR
  PRIMARY KEY (epr_cid, leg)
);

CREATE TABLE epr_claims (
  epr_cid       TEXT NOT NULL REFERENCES epr_atoms(cid),
  claim_cid     TEXT NOT NULL,
  PRIMARY KEY (epr_cid, claim_cid)
);

CREATE TABLE epr_supersedence (
  predecessor   TEXT NOT NULL,
  successor     TEXT NOT NULL,
  attested_by   TEXT NOT NULL,
  attested_at   TIMESTAMP NOT NULL,
  PRIMARY KEY (predecessor, successor)
);
```

Payloads are decoded on read via a schema-aware decoder. The canonical bytes are the source of truth; any projection can be rebuilt from them.

### 8.2 Content-store coexistence

The existing `content_store` zome + blob storage (chunked, RS-4-7, RS-8-12) continues to serve large content bodies. An EPR with a large payload (e.g., a Content EPR with video body) stores:

- Envelope + small metadata in `epr_atoms.canonical_bytes`
- Large body blob in the existing content_store (addressed by the body's own CID)
- `payload.bodyCid: CID` reference from the envelope to the blob

This preserves the split architecture: notarization + envelope in DHT/storage-atoms; bulk content in libp2p/erasure-coded blobs.

### 8.3 Per-pillar projection tables

Existing pillar projection tables (content_nodes, learning_paths, economic_events, etc.) are **kept**. They become materialized views over the EPR table — populated on insert by a kind-aware projector. Apps continue to query them via typed REST as today. The EPR table is the source of truth; the projections are indexes.

---

## 9. Query surface

### 9.1 What stays REST

- `/content/{cid}` — fetch an EPR by CID (returns canonical bytes + decoded JSON)
- `/blobs/{cid}` — fetch a content-store blob
- `/manifests/{cid}` — fetch a manifest EPR
- Doorway routing
- Auth/session endpoints

REST serves content addressing (look up by CID). No change to existing `storage-client-ts` typed clients; the EPR table is transparent to them.

### 9.2 What's new: GraphQL

A new query surface serves **graph traversal**. Shape:

- **Schema is generated from manifests.** The GraphQL schema is not hand-authored. A codegen step walks the manifest graph and emits SDL for each content type, relationship, and signal. Running codegen is equivalent to "composing the federated schema."
- **Resolvers are kind-aware.** Default resolvers traverse coupling refs, follow supersedence, filter by reach (caller's identity scope). Domain manifests can register custom resolvers.
- **Queries are federated by default.** A query that references multiple manifests automatically plans sub-queries to each subgraph's endpoint.
- **Subscriptions.** GraphQL subscriptions are served over WebSocket for web clients and over libp2p gossip for P2P clients. Subscription keys are CID-filters (e.g., "any EPR where coupling.value = X").

Reference implementation language: **async-graphql in Rust** (maps cleanly to the existing elohim-storage tokio runtime; matches darksoil/Holochain stack). **Apollo Federation v2 subgraph spec** (the `@key` / `@external` / `_service` / `_entities` contracts) is the wire contract that lets subgraphs be composed — *without* requiring an Apollo gateway binary. Any federation-aware resolver (our own, Apollo Router open-source, Mesh, Hot Chocolate) can compose subgraphs that speak this spec. Choosing the spec decouples us from any single implementation.

### 9.3 Query examples

```graphql
# Cross-pillar: humans in my affinity group who have mastered this concept
# and have an active offer related to it
query {
  me {
    affinities(type: "close") {
      members {
        masteries(concept: $conceptCid, minLevel: COMPETENT) {
          level
          attestedAt
        }
        offers(relatedConcept: $conceptCid, status: ACTIVE) {
          title
          economicEvent { resourceConformsTo, quantity }
        }
      }
    }
  }
}
```

Single round trip. Pre-split architecture: impossible without glue code. With EPR substrate: natural.

---

## 10. Agent hooks

Per the `elohim-agent sense-and-respond` memory: discernment and gates live in Rust (elohim-agent), not TypeScript. TypeScript senses and responds; it never evaluates.

Agent hooks attach at three points in the EPR lifecycle:

1. **Pre-acceptance (gate).** Before an EPR is indexed, elohim-agent evaluates applicable gates (declared in the manifest). A gate may accept, reject, or escalate.
2. **Post-acceptance (signal).** After indexing, elohim-agent emits sense events for subscribers (other agents, dashboards, governance signals).
3. **Observation accumulation.** Observation EPRs targeting a Claim EPR trigger re-evaluation of the claim's validity. When negative observations accumulate past threshold, the agent emits a `ClaimExpired` signal and schedules a review obligation.

Gates and signals are declared per content-type in the manifest. The gate surface is declarative + auditable: any third party can see what gates apply to any EPR kind by reading the manifest.

---

## 11. Reference subgraph inventory

Each pillar's reference subgraph ships with elohim-core and is published at a well-known CID as the "canonical starting point." Apps can extend or replace.

| Subgraph | Manifest publishes | Canonical starting entities |
|---|---|---|
| imagodei | Agent, Delegation, Attestation, ContributorPresence | Humans, device bindings |
| lamad | Content (concept/lesson/path/mastery/assessment/...), ContentMastery, LearningPath | `Human` + `Feature` + `Role` triples (from experience-story EPR prior art) |
| shefa | Agreement, Commitment, EconomicEvent, Measure — extending vf-graphql | Stewardship contracts, flows |
| mishpat | Proposal, Vote, Challenge, Appeal, Delegation | Adjudication graph |
| qahal | Affinity, Trust, GroupMembership | Social graph scopes |

**Process demonstrator subgraphs:**

| Subgraph | What it demonstrates |
|---|---|
| avodah | "Protocol as process" — how contribution/work flows are themselves EPRs; every commit, every spec, every governance decision is a published atom in the graph |
| doorway | Web2 projection — shows how external clients query the substrate through a blind proxy |

---

## 12. hREA / VF-GraphQL alignment

The strategic lever for HC-team landing:

1. Publish the **VF-GraphQL manifest** as a canonical Manifest EPR (one-time, by the elohim project or the VF team)
2. Shefa's native manifest **extends** the VF manifest — every shefa EconomicEvent is a valid VF EconomicEvent at the envelope level
3. R&O (or any hREA hApp) publishes its own manifest EPR extending VF
4. Cross-app queries traverse coupling refs uniformly — R&O offers become queryable from elohim without translation
5. hREA DNAs continue to run inside hApps for group-scoped governance; elohim serves as the **ecosystem-scale graph** over them

This is the concrete deliverable for "R&O legacy in elohim": R&O's `Request` and `Offer` records become EPRs published to the substrate, joined to shefa's commitment graph, visible in elohim-app without any migration of R&O's local DHT.

---

## 13. Incremental adoption path

Each phase is independently mergeable and produces demonstrable value. Phase N does not require Phase N+1.

**Phase 0 — Foundation (this spec).** Design spec committed. Implementation plan written.

**Phase 1 — Core codec crate.** New Rust crate `elohim-epr` (in `elohim/sdk/` or a new `elohim/epr/`): canonical CBOR, CID derivation, Ed25519 signing/verification, EPR struct, kind enum, canonical JSON interop. TypeScript port for UI-side verification.

**Phase 2 — Storage integration.** elohim-storage gains `epr_atoms` + `epr_coupling` + `epr_claims` + `epr_supersedence` tables. Existing content_nodes and friends become projections. Validator (§7) runs on insert. Existing ingestion paths write EPRs in parallel with legacy projections (write-through, zero behavior change for consumers).

**Phase 3 — Manifest graph resolver.** Subgraph manifest EPR kind implemented. Manifest-graph traversal resolver in elohim-storage. schemaRef → JSON Schema lookup working end-to-end. All existing `elohim/sdk/domains/*/manifest.json` republished as Manifest EPRs.

**Phase 4 — GraphQL surface, single subgraph.** async-graphql in elohim-storage. Schema codegen from manifest graph. Ship **shefa subgraph** first (VF-shape, highest strategic value). Reference query: R&O hypothetical offer EPR resolves through cross-manifest federation.

**Phase 5 — Pillar rollout.** lamad, imagodei, mishpat, qahal subgraphs published. Angular side adopts Apollo Client for graph queries (kept REST for content fetch). Cross-pillar query examples proven. avodah demonstrator publishes protocol-development EPRs.

**Phase 6 — Subscriptions + federation.** GraphQL subscriptions over WebSocket + libp2p. External publishers (R&O, other hREA hApps) discoverable via subgraph manifests. Federated resolver dispatches across endpoints.

**Phase 7 — HC-team landing demo.** End-to-end demo: R&O hApp publishes subgraph manifest, elohim-app queries across R&O + shefa + lamad in one round trip. Written up as "R&O graduation path" doc. Presentable to Sasha + VF team.

---

## 14. What stays unchanged

- Existing elohim-storage REST endpoints and typed clients
- Existing content_store zome, chunked blobs, erasure coding
- Existing Holochain DNAs (lamad, mishpat, imagodei, infrastructure, node-registry) — they remain notarization authorities
- Existing pillar Angular services, routes, components
- Existing `elohim/sdk/domains/*/manifest.json` files (become the source for the first-generation Manifest EPRs)
- Existing Signal Harness pattern (now wired to emit EconomicEvent EPRs instead of direct records)
- Doorway's role as blind proxy

---

## 15. Non-goals

- **Replacing Holochain.** Holochain remains the notarization + integrity authority for the DNAs that use it. The EPR substrate is one layer higher.
- **Replacing diesel.** elohim-storage continues as a relational store; EPR tables are just more diesel tables.
- **Requiring GraphQL for everything.** REST is correct for content addressing. GraphQL is correct for graph traversal. Both coexist.
- **Building a new database engine.** elohim-storage remains a projection/index service, not a graph database. Queries plan against indexes; there is no native graph query engine.
- **Requiring all apps to use elohim-core.** Apps that don't need graph substrate don't opt in. R&O can be a graph citizen without rewriting its DNA.
- **Competing with Moss.** Moss groups federate via Weave for group-scale coordination. Elohim serves ecosystem-scale graph composition *across* such groups. They compose.
- **Solving content-distribution here.** The existing content_store + libp2p + RS erasure coding handles bulk content. EPR envelope is lightweight; payload bodies > 64 KB are CID-referenced to content_store.

---

## 16. Risks & open questions

| Risk | Mitigation |
|---|---|
| Canonical CBOR ecosystem maturity | Use `ciborium` (Rust) + `cbor-x` (TS); write a shared test vector suite; interop-test Rust ↔ TS on every CI run |
| Signature scheme changes (post-quantum) | `Signature.algorithm` field is open-ended; new schemes added via manifest-declared extensions; EPR shape unchanged |
| Schema versioning cascading (lamad v2 breaks consumers) | Consumers pin schemaRef CID; upgrading is voluntary; supersedence chains are follow-at-your-own-pace |
| Subscription delivery at P2P scale | Start with WebSocket-only (Phase 6); libp2p gossip for subscriptions deferred to post-Phase-7 if scale demands |
| Federated query performance | Cache subgraph endpoints + cache manifest CIDs; dataloader pattern for batching; measure Phase 4; optimize before Phase 5 |
| Agent gate latency in hot path | Gates declared per-kind; high-throughput kinds (Observation) get fast-path gates; slow gates run async with post-acceptance revocation |
| Adoption resistance ("just another graph layer") | Incremental phases ship value standalone; no big-bang migration; existing code keeps working |

**Open questions for the implementation plan:**

1. Does the canonical CBOR + Ed25519 codec crate live under `elohim/sdk/` or under a new `elohim/epr/` root?
2. Who publishes the first-generation VF-GraphQL manifest EPR — elohim project, or outreach to VF team to co-publish?
3. GraphQL endpoint hosting: inside elohim-storage binary (simpler) or new `elohim-graphql` service (cleaner boundaries)?
4. Subscription key format — CID filter spec needs its own micro-design
5. Gate declaration schema — does this extend app-manifest.schema.json or live in a sibling gate-manifest.schema.json?

---

## 17. Success criteria

This design is implemented successfully when:

1. **Any elohim-app consumer can fetch a Content EPR by CID and validate its envelope + signature + coupling + payload schema** without depending on app-specific code.
2. **Shefa's EconomicEvents are queryable via VF-GraphQL** at the elohim-storage endpoint, returning valid VF-shaped results.
3. **A cross-pillar query** (the example in §9.3) returns in a single round trip and traverses imagodei + qahal + lamad + shefa subgraphs correctly.
4. **A hypothetical R&O subgraph publication** — R&O offers republished as EPRs in elohim — is queryable from elohim-app without code changes to elohim-app's query layer.
5. **A new app built on elohim-core** that declares a manifest, publishes EPRs, and uses the GraphQL surface gets graph traversal, federation, subscription, and schema validation **for free**.
6. **The HC team (Sasha + VF team) can run the Phase-7 demo** and see R&O + elohim + hypothetical additional hApps composing in a single graph query.

---

## 18. Strategic framing (why this is the HC landing zone)

Holochain's current story is: "hApps are sovereign coordination tools running inside Moss groups." That story caps at medium-sized cooperatives. It does not explain how the protocol carries an ecosystem.

What this spec delivers is: **elohim-core is the substrate that lets hApps mean "specialized governance/coordination DNA" rather than "walled garden."** R&O becomes a first-class citizen of a larger graph without migrating its DHT. Any REA-flavored hApp inherits elohim's graph surface by publishing its manifest. ValueFlows becomes the wire vocabulary; hREA becomes one implementation among many; Holochain becomes the notarization authority for the claims that matter.

This is a concrete gift to the HC team: the piece of their architecture they have not yet built, delivered in a way that respects their existing primitives, doesn't compete with Moss, doesn't replace hApps, and gives them an ecosystem story for Sasha's next pitch. Every hApp on Holochain becomes composable with every other hApp on Holochain — not by network merge, but by graph publication.

That is the 20-year primitive.

---

## Appendix A — Decisions incorporated

- **A-1** Publish-with-batteries over own-the-graph (brainstorm resolution, 2026-04-21)
- **A-2** EPR envelope + payload over flat SCAC (brainstorm resolution, 2026-04-21)
- **A-3** ThreeLegCoupling as atomic, intrinsic to envelope (inherited from `app-manifest.schema.json`)
- **A-4** Capture test as SDK boundary rule (inherited from `elohim/sdk/CLAUDE.md`)
- **A-5** Manifests as EPRs (extension of existing manifest.id = CID convention)
- **A-6** CBOR + Ed25519 + CIDv1 as wire primitives (new, codified here)
- **A-7** REST for content addressing, GraphQL for graph traversal (new, codified here)
- **A-8** Avodah and doorway as process demonstrators, not pillars (clarified 2026-04-21)

## Appendix B — Glossary

| Term | Meaning |
|---|---|
| EPR | EntityPortalReference — a content-addressed, signed, coupled atom |
| Envelope | Protocol-owned fields common to all EPR kinds |
| Payload | Domain-owned, schema-validated body specific to the kind |
| ThreeLegCoupling | Knowledge + value + governance coupling requirement |
| Capture test | "Could this be captured at scale for rent extraction?" Yes → protocol; no → domain |
| Manifest EPR | An EPR whose payload is an app-manifest; publishes vocabulary |
| Subgraph manifest | A Manifest EPR subtype declaring an indexed, queryable subgraph |
| Reach | Envelope-level enum: commons / community / collective / steward / private |
| Supersedence | Issuer-attested successor relationship between two EPR versions |
| Federation | Cross-manifest query execution via coupling-ref traversal |
| Pillar | One of imagodei / lamad / shefa / mishpat / qahal — capture-resistance obligation surface |
| Process demonstrator | Subgraph that demonstrates protocol-as-process (avodah, doorway) |
