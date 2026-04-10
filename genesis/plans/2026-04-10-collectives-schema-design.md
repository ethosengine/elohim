# Collectives Schema Design — Holonic Constitutional Architecture

**Date:** 2026-04-10
**Status:** Design approved, pending implementation
**Scope:** `genesis/data/collectives/collectives.schema.json`, seed data enrichment, seeder validation

## Problem

The collectives seed data (`genesis/data/collectives/collectives.json`) has no schema validation. The `$schema` reference points to a non-existent file. The seed data lacks:

1. **Holonic structure** — no intimate-scale collectives (couples, families), no constitutional hierarchy
2. **Referential integrity** — `constitutionalParentId` is never validated against existing IDs
3. **Protocol alignment** — `reach` values (`local`, `municipal`) don't match protocol `ReachLevels`
4. **Relationship graph** — no way to express the holonic lattice (a family participates in both a church AND a neighborhood)
5. **EPR coupling** — collectives aren't subject to three-leg coupling (lamad + shefa + qahal)
6. **Dunbar-aware scaling** — no constraints linking reach to participation scale

Additionally, the `extract_app_context()` routing bug in `elohim-storage/src/http.rs:1821` caused `/db/collectives` POST requests to return 405 (already fixed — all Diesel entity prefixes added to `legacy_prefixes`).

## Architectural Context

### Collectives are protocol core, not qahal-only

Collectives define **agent context boundaries** for elohim. When an elohim operates in `couple-matthew-jessica`, it has different provenance, authority, and memory than in `community-local-church`. The relationship graph between collectives defines how provenance flows between agent contexts.

Every pillar needs collective context:
- **lamad**: learning happens in collective contexts (homeschool co-op, family devotions)
- **shefa**: economic events are scoped to collective contexts (household budget, church tithe)
- **imagodei**: identity attestations come from collective contexts (family attests character, workplace attests competence)
- **qahal**: governance is the collective's own self-organization (available, not required)

### The holonic lattice

Collectives form a lattice, not a tree. A family is simultaneously nested in a church (faith governance), a neighborhood (geographic governance), and a co-op (education governance). `constitutionalParentId` names the primary appeal chain (tree). The relationship graph captures the full lattice.

Sociocratic double-linking adds another dimension: collectives inter-link through representative roles. These links are provenance channels for both human governance and agent coordination.

### Shoulder angels and ambient governance

Each human's elohim (shoulder angel) carries contextual memory of the person across all their collective memberships. Under normal conditions, the agent observes and learns governance disposition. Under elevated conditions, it acts within its confidence envelope. Under emergency, it participates in real-time deliberation with other agents at machine speed through the holonic lattice.

The delegation chain: **human → shoulder angel → collective agent context → constitutional hierarchy of agents**.

This dissolves the "consent is slow" objection: consent rounds happen at machine speed between agents who carry genuine contextual representation. Humans are escalated only when decisions exceed the agent's confidence threshold.

The Dunbar limit applies to humans, not agents. An agent maintains provenance across every collective context simultaneously. The holonic lattice operates at human speed for daily governance and agent speed for crisis response.

## P2P Design Gate

### Entity: Collective
- **Classification**: Notarized (A) — DHT entry type exists in Mishpat DNA
- **Justification**: The protocol would be lying if a collective's governance layer, constitutional parent, or membership could be silently altered
- **Content Address Strategy**: Slug — human-navigable identifiers for URL routing and cross-collective references (`family-dowell`, `bible-study-valley`). Mutable entity, so CID doesn't apply. Not agent-scoped.
- **Source of Truth**: Holochain DHT (Mishpat DNA) — SQLite `collectives` table is read-optimized projection
- **Storage Projection**: `collectives` table — **`dht_anchor_hash` column missing, needs future migration** (out of scope for this seed-data work)
- **HTTP Route**: `GET/POST /db/collectives`, `GET /db/collectives/{id}` (exist)

### Entity: CollectiveRelationship (new — seed-data declarative)
- **Classification**: Derived (A2) — anchored via Holochain Link on parent collective's entry
- **Justification**: Relationships have no meaning without the collectives they connect. Link tag carries type, domainOverlap, description (<256 bytes)
- **Content Address Strategy**: Composite tuple `(from_collective_hash, to_collective_hash, relationship_type)`
- **Source of Truth**: Holochain DHT (Link on Mishpat DNA)
- **Storage Projection**: None yet — seed-data declarative only. Future: `collective_relationships` table
- **HTTP Route**: None yet. Future: `GET /db/collectives/{id}/relationships`

### Entity: CollectiveParticipation (existing)
- **Classification**: Derived (A2) — link between Human (Imagodei DNA) and Collective (Mishpat DNA)
- **Content Address Strategy**: Composite tuple `(human_agent_key, collective_entry_hash, role_context)`
- **Source of Truth**: Holochain DHT (cross-DNA link)
- **Storage Projection**: `collective_participations` table — **`dht_anchor_hash` column missing, needs future migration**
- **HTTP Route**: `GET/POST /db/collectives/{id}/participants` (exists)

### Entity: ReachConstraints, RelationshipTypeVocabulary
- **Classification**: Operational (C) — protocol configuration embedded in JSON Schema, not stored data
- **Source of Truth**: `genesis/data/collectives/collectives.schema.json` (file)

### Design Constraints Discovered
1. **Missing `dht_anchor_hash`** on `collectives` and `collective_participations` tables — out of scope for this work but tracked as technical debt
2. **Cross-DNA linking** for participations (Human in Imagodei, Collective in Mishpat) requires bridge pattern
3. **Seed data operates at the projection layer** — seeder POSTs to SQLite; DHT notarization happens when peers ingest via pull-based seeding model
4. **Relationship graph is seed-data-only** — `relationships[]` array is declarative with no HTTP surface yet

## Design

### Two-section seed data structure

`collectives.json` gains two top-level arrays:

```json
{
  "$schema": "./collectives.schema.json",
  "version": "1.0.0",
  "description": "Holonic collective definitions with constitutional relationships",
  "collectives": [ ... ],
  "relationships": [ ... ]
}
```

### Collective entity schema

Each collective:

```json
{
  "id": "family-dowell",
  "name": "Dowell Family",
  "governanceLayer": "family",
  "reach": "trusted",
  "constitutionalParentId": "neighborhood-valley",
  "description": "Nuclear family — Matthew, Jessica, Timothy",
  "governanceModel": "steward-consent",
  "domain": "family",
  "place": null,
  "coupling": {
    "lamad": "household-wisdom",
    "shefa": "household-economy",
    "qahal": "steward-consent"
  }
}
```

#### Field definitions

| Field | Required | Type | Source of Truth |
|-------|----------|------|-----------------|
| `id` | yes | string (slug) | unique within file |
| `name` | yes | string | human-readable |
| `governanceLayer` | yes | enum | Rust `governance_layers::ALL`: `family`, `neighborhood`, `faith`, `education`, `interest`, `geographic`, `workplace`, `economic`, `community` |
| `reach` | yes | enum | Protocol `ReachLevels`: `private`, `self`, `intimate`, `trusted`, `familiar`, `community`, `public`, `commons` |
| `constitutionalParentId` | no | string | Must reference another collective's `id` — primary appeal chain |
| `description` | no | string | Human context |
| `governanceModel` | no | enum | `consent`, `steward-consent`, `community-vote`, `constitutional`, `consensus` |
| `domain` | no | enum | What this collective governs: `household`, `curriculum`, `worship`, `infrastructure`, `trade`, `land-use`, `economy`, `defense` |
| `place` | no | string / null | H3 cell ID for geographic grounding |
| `coupling` | no | object | Three-leg EPR coupling declaration |

#### Coupling declaration

Makes collectives full EPR citizens. Each leg names what this collective stewards:

```json
"coupling": {
  "lamad": "household-wisdom",
  "shefa": "household-economy",
  "qahal": "steward-consent"
}
```

A collective without declared coupling is inert — a social group, not a protocol entity. The coupling is what makes it alive in the protocol. The coupling values are descriptive labels (not enum-constrained) that declare what each leg means in this context.

#### Storage mapping

Fields that don't exist as Rust columns (`governanceModel`, `domain`, `place`, `coupling`) flow through `metadata_json` on `CreateCollectiveInputView`. The seeder maps them into the `metadata` bag before POSTing. Zero Rust schema changes.

### Reach constraints (Dunbar-aware)

Fuzzy ranges, not hard limits. The protocol observes what works and nudges; it doesn't prevent local interpretation.

```json
"reachConstraints": {
  "private":   { "suggestedRange": [1, 1],     "cautionAbove": 1    },
  "self":      { "suggestedRange": [1, 1],     "cautionAbove": 1    },
  "intimate":  { "suggestedRange": [2, 7],     "cautionAbove": 7    },
  "trusted":   { "suggestedRange": [5, 20],    "cautionAbove": 20   },
  "familiar":  { "suggestedRange": [15, 65],   "cautionAbove": 65   },
  "community": { "suggestedRange": [50, 250],  "cautionAbove": 250  },
  "public":    { "directParticipants": false },
  "commons":   { "directParticipants": false }
}
```

- Ranges overlap intentionally — a group of 15 could be `trusted` or `familiar` depending on relational quality
- `cautionAbove` triggers elohim observation signals, not hard validation errors
- `public` and `commons` collectives have no direct human participants — governance is through sub-collective delegates only
- A commune of 40 at `trusted` reach that works beautifully generates positive observations; the protocol learns
- Cultural contexts (polycules, extended families, communes) are valid interpretations within the ranges

### Relationship type vocabulary

Protocol-core relationship types with provenance flow semantics:

```json
"relationshipTypes": {
  "contains": {
    "description": "Holonic nesting — parent holon contains child holon",
    "provenanceFlow": "bidirectional-filtered",
    "agentSemantics": "Parent context sees child summaries; child inherits constitutional constraints",
    "humanSemantics": "Child collective operates within parent's constitutional frame",
    "constraint": "Only valid within same constitutional chain — church does not contain family"
  },
  "participates-in": {
    "description": "Collective-to-collective membership — a holon participates in a larger context",
    "provenanceFlow": "directional-read",
    "agentSemantics": "Agent references shared context without inheriting full history",
    "humanSemantics": "Collective joins a larger context but retains autonomy — can withdraw",
    "constraint": "Cross-domain participation — family participates-in church, not contained-by"
  },
  "delegates-to": {
    "description": "Sociocratic double-link — representative carries governance between circles",
    "provenanceFlow": "directional-governance",
    "agentSemantics": "Agent carries decisions, precedents, and confidence envelopes along this edge",
    "humanSemantics": "Representative carries consent of constituents; can be recalled",
    "confidenceThreshold": "Agent acts within envelope; escalates to human above threshold"
  },
  "peers-with": {
    "description": "Horizontal governance coupling — collectives coordinate on overlapping domains",
    "provenanceFlow": "bidirectional-scoped",
    "agentSemantics": "Agents share provenance within declared domain overlap only",
    "humanSemantics": "Collectives coordinate on shared concerns without hierarchy",
    "constraint": "Requires declared domain overlap"
  },
  "succeeds": {
    "description": "Generational succession — new collective inherits precedent history",
    "provenanceFlow": "directional-historical",
    "agentSemantics": "New context inherits precedent and memory from predecessor",
    "humanSemantics": "Governance continuity across generational transitions"
  }
}
```

#### Key structural rules

1. **`contains` is domain-scoped**: A church does not contain a family. A family *participates in* a church. `contains` only flows within the same governance domain (family contains couple, county contains community).
2. **`participates-in` allows withdrawal**: This is the dissenter protection mechanism. A family can withdraw from a tribal collective without dissolving. The family is a sovereign holon.
3. **`delegates-to` carries confidence thresholds**: Under normal governance, the delegate is a human. Under ambient/emergency governance, the delegate's shoulder angel acts within its confidence envelope. The relationship type carries both modes.
4. **`public` and `commons` collectives exist only through relationships**: No direct human participants. Governance is exclusively through `delegates-to` chains from sub-collectives.

### Relationship entries

Each relationship in the seed data:

```json
{
  "type": "contains",
  "from": "family-dowell",
  "to": "couple-matthew-jessica",
  "description": "Nuclear family contains founding couple"
}
```

For `peers-with`, a `domainOverlap` field declares the shared concern:

```json
{
  "type": "peers-with",
  "from": "community-local-church",
  "to": "community-homeschool-coop",
  "domainOverlap": "child-development",
  "description": "Church and co-op coordinate on children's holistic development"
}
```

### Validation rules

#### Static (JSON Schema)

- Enum validation: `governanceLayer`, `reach`, `governanceModel`, relationship `type`
- Required fields: `id`, `name`, `governanceLayer`, `reach`
- Pattern constraints: `id` matches slug format `[a-z0-9-]+`
- `place` matches H3 cell pattern when present
- `coupling` object has only `lamad`, `shefa`, `qahal` keys

#### Referential integrity (companion validator)

- All `constitutionalParentId` values reference existing collective `id`s
- All relationship `from` and `to` values reference existing collective `id`s
- No circular `constitutionalParentId` chains
- `contains` relationships only flow within compatible governance domains
- `peers-with` relationships require `domainOverlap` field
- `public`/`commons` reach collectives have no direct `constitutionalParentId` (they're composed of sub-collectives via relationships)

#### Reach coherence (warnings, not errors)

- Participation count vs. reach `suggestedRange` — warn if `cautionAbove` is likely exceeded
- Child collective reach should not exceed parent collective reach
- `contains` target has narrower or equal reach to source
- `participates-in` target has broader or equal reach to source

### Reach alignment (existing seed data)

| Current Value | Corrected Value | Rationale |
|---------------|----------------|-----------|
| `private` | `private` | Stays — individuals |
| `local` | `familiar` | "People who know each other" — maps to Dunbar close group |
| `community` | `community` | Stays — Dunbar-scale |
| `municipal` | `community` | Municipal isn't a protocol reach level; these orgs serve community-scale |

### New collectives (holonic genesis humans)

Five new collectives modeling intimate-scale governance for the genesis scenario:

| ID | Name | Layer | Reach | Constitutional Parent | Humans |
|----|------|-------|-------|-----------------------|--------|
| `couple-adam-eve` | Adam & Eve | family | intimate | — | Adam, Eve |
| `couple-matthew-jessica` | Matthew & Jessica | family | intimate | `family-dowell` | Matthew, Jessica |
| `family-dowell` | Dowell Family | family | trusted | `neighborhood-valley` | Matthew, Jessica, Timothy |
| `bible-study-valley` | Valley Bible Study | faith | familiar | `community-local-church` | Matthew, Jessica, Timothy, Pete |
| `neighborhood-valley` | Valley Neighborhood | geographic | familiar | `community-neighborhood-association` | Matthew, Jessica, Timothy, Nancy |

### Existing collective renames

| Old ID | New ID | Reason |
|--------|--------|--------|
| `household-dowell` | `family-dowell` | Consistent naming — `family-` prefix for family-layer collectives |
| `household-eden` | `family-eden` | Same pattern |
| `household-valley-economy` | `neighborhood-valley-economy` | Was neighborhood layer, not family |
| `household-extended` | `neighborhood-extended` | Was neighborhood layer, not family |

### Genesis holonic lattice

```
Constitutional hierarchy (contains):

  valley-community [geographic, community]
  ├── community-local-church [faith, community]
  │   └── bible-study-valley [faith, familiar]
  │       Humans: Matthew, Jessica, Timothy, Pete
  ├── community-neighborhood-association [geographic, community]
  │   └── neighborhood-valley [geographic, familiar]
  │       Humans: Matthew, Jessica, Timothy, Nancy
  └── community-homeschool-coop [education, community]

  family-dowell [family, trusted]
    constitutional parent: neighborhood-valley
    Humans: Matthew, Jessica, Timothy
    └── couple-matthew-jessica [family, intimate]
        Humans: Matthew, Jessica

Lateral relationships (participates-in, peers-with, succeeds):

  couple-adam-eve ──succeeds──▶ couple-matthew-jessica
  family-dowell ──participates-in──▶ community-local-church
  family-dowell ──participates-in──▶ neighborhood-valley
  family-dowell ──participates-in──▶ community-homeschool-coop
  community-local-church ──peers-with──▶ community-homeschool-coop
    domainOverlap: child-development
  bible-study-valley ──delegates-to──▶ community-local-church
```

### Agent topology (same graph, dual interpretation)

The holonic lattice is simultaneously:
- **Human governance topology**: consent flows, delegation chains, constitutional appeals
- **Agent coordination topology**: provenance flow, confidence envelopes, ambient governance

Each collective is an agent context boundary. The shoulder angel (personal elohim) maintains provenance across all collective contexts a human participates in. Under emergency, agents deliberate in real-time through the lattice while humans sleep. Escalation to human attention occurs when decisions exceed the agent's confidence threshold.

The `delegates-to` relationship carries dual semantics:
- **Human mode**: representative attends parent circle, carries constituent consent
- **Agent mode**: shoulder angel participates in parent context deliberation within confidence envelope, escalates beyond it

### Scaling to nation-state and global

The holonic lattice scales through delegation chains:

```
Couple (intimate, 2-7)
  → Family (trusted, 5-20)
    → Village/Neighborhood (familiar, 15-65)
      → Community (community, 50-250)
        → District (public, ~30 community delegates)
          → County (public, ~20 district delegates)
            → Bioregion (commons, ~15 county delegates)
              → National (commons, ~8 bioregion delegates)
```

At every level above Dunbar, governance is through delegates carrying validated consent. No collective has direct authority over more than ~250 humans. Beyond that, governance flows through the holonic lattice — always through consent at the sub-collective level.

`public` and `commons` reach collectives have no direct human participants. They exist entirely through relationship chains. A national-level collective of 200 delegates is within Dunbar. Power is distributed, not concentrated. Each delegate can be recalled by their sub-collective.

### Resilience under threat (red-teamed)

**External threat (nation-state invasion):**
- Distributed information through the lattice — no single command center to target
- Shoulder angels coordinate defense response at machine speed through the constitutional hierarchy
- Economic flows redirect through shefa coupling for emergency resource mobilization
- Emergency response is real-time ambient governance, not slow consent rounds
- Humans escalated for decisions exceeding agent confidence

**Internal charismatic rejection:**
- `participates-in` allows withdrawal — families can exit hostile collectives without dissolving
- Constitutional challenges propagate up the hierarchy (immune system response)
- Economic flows degrade through broken three-leg coupling (violence voids governance leg)
- Dissenter families' shoulder angels signal distress up the chain
- The family holon is sovereign — no parent collective can override intimate-scale autonomy

### Domain-bounded authority

A collective's `domain` field limits what it can govern:

| Domain | Governs | Cannot Govern |
|--------|---------|--------------|
| `household` | Daily life, resource allocation, child-rearing | Doctrine, land-use |
| `curriculum` | Learning paths, assessment methods | Household decisions |
| `worship` | Doctrine, spiritual practice, pastoral care | Land-use, economy |
| `land-use` | Property, infrastructure, shared spaces | Doctrine, household |
| `economy` | Resource flows, stewardship, exchange | Doctrine, curriculum |
| `trade` | Inter-collective economic agreements | Internal household |
| `infrastructure` | Shared systems, commons maintenance | Personal household |
| `defense` | Physical security, emergency response | Normal governance |

Domain boundaries are enforced through the coupling declaration. A church collective with `coupling.qahal: "worship-governance"` cannot make land-use decisions. A neighborhood with `coupling.qahal: "land-use-governance"` cannot set doctrine.

## Files to create/modify

| Action | File | Purpose |
|--------|------|---------|
| **Create** | `genesis/data/collectives/collectives.schema.json` | JSON Schema — enum validation, field types, relationship structure |
| **Modify** | `genesis/data/collectives/collectives.json` | Add holonic collectives, relationships, fix reach values, wire hierarchy |
| **Modify** | `genesis/seeder/src/seed-collectives.ts` | Schema validation, metadata mapping, topological sort for POST order |
| **Already done** | `elohim/elohim-storage/src/http.rs:1821` | `legacy_prefixes` expanded (routing fix) |

### Not in scope

- No Rust schema changes — new fields ride `metadata_json`
- No new API endpoints — relationships are seed-data declarative for now
- No view schema in `sdk/schemas/v1/views/` — comes when relationships get HTTP surface
- No migration of `collectives` table — columns stay as-is
- No qahal manifest changes — governance mechanics layer on top, separate work

## Relationship to existing protocol concepts

| Concept | How collectives connect |
|---------|----------------------|
| EPR three-leg coupling | `coupling` field makes collectives EPR-compliant |
| ReachLevels | `reach` field uses protocol enum; `reachConstraints` adds Dunbar semantics |
| Elohim agent contexts | Each collective IS an agent context boundary |
| Reconnoiter pattern | Shoulder angels carry provenance between collective contexts |
| Constitutional hierarchy | `contains` + `delegates-to` relationships form the hierarchy |
| Steward affinity | Governance weight proportional to stewardship, not collective size |
| ResponsibilityDemandParam | Higher reach = higher responsibility demand |
| Feedback as information flow | Elohim observe collective health signals, nudge via reach constraints |
