---
name: p2p-design-gate
description: Mandatory gate for any feature design involving data entities (tables, models, routes, sync messages). Forces P2P-native thinking — DHT entry types, content addressing, source-of-truth classification — before proposing design approaches. Use when brainstorming any feature that creates, stores, references, or syncs data entities.
metadata:
  author: elohim-protocol
  version: 1.0.0
---

# P2P Design Gate

This skill is a **mandatory checkpoint** during feature design. It fires between brainstorming (understanding what we need) and design proposal (how we build it). No data entity may be proposed — no table, no model, no route, no sync message — without passing through this gate first.

## When This Gate Fires

This gate is **not optional**. It activates whenever a design conversation involves:

- Creating a new database table or migration
- Defining a new model, struct, or TypeScript interface for persistent data
- Adding an HTTP route that serves or mutates data
- Designing a sync/gossip message between peers
- Proposing a new "entity" of any kind

**Sequence**: The gate sits between step 2 (understanding the domain need) and step 3 (proposing a design). You must complete the gate output before writing any schema, migration, or route code.

If you find yourself reaching for `CREATE TABLE` or `#[derive(Serialize)]` before completing this gate, stop. Back up. Run the gate.

---

## DHT Capacity Constraints (READ FIRST)

The Holochain DHT is a **notary, not a database**. Hard constraints shape every classification decision:

| Constraint | Limit | Current State |
|---|---|---|
| Entry types per DNA | ~100 | Lamad: **83** (near ceiling), Imagodei: 28, Infrastructure: 6 |
| Total DHT entries | ~3000 before degradation | Designed for 100s-1000s |
| Entry size | <1KB target | Proofs only: who (agent key), what (content hash), when (timestamp) |
| Query capability | None — link traversal only | No SQL, no pagination, no filtering |
| Gossip latency | 200-2000ms | Unacceptable for real-time reads |

**Before proposing ANY new DHT entry type**: Check if the DNA has headroom. Lamad at 83/~100 means you likely need to use an EXISTING entry type, not create one. Most entities that need notarization already have entry types — the gap is usually the missing `dht_anchor_hash` in the storage projection, not a missing entry type.

## Step 1: Entity Classification Decision Tree

Every data entity falls into exactly one of five categories. Walk the tree for each entity in the design.

### Category A: Notarized (existing entry type)

**Examples**: content items, economic events (REA), attestations, stewardship allocations, governance proposals, relationships between agents.

**Test**: Would the protocol be lying if this data were silently changed or deleted? AND does a Holochain entry type already exist for this?

**Requirements**:
- Uses an EXISTING Holochain DHT entry type (do NOT create new ones without checking DNA capacity)
- MUST have `dht_anchor_hash NOT NULL` in the SQLite storage projection
- Source of truth is **Holochain DHT** — the SQLite row is a read-optimized projection, not the canonical record
- Post-commit signal projects the entry to elohim-storage for fast query

### Category A2: Derived (anchored via link to existing entry type)

**Examples**: content_tags (link metadata on Content entry), path chapters/steps (links on LearningPath entry), stewardship_allocations (link metadata on Agreement entry).

**Test**: Does this data need notarization, but it's really a **relationship or attribute** of an already-notarized entity — not a standalone entity?

**Requirements**:
- Does NOT need its own DHT entry type — anchored via Holochain Link on an existing entry
- Link tag carries the metadata (type, weight, role — small, <256 bytes)
- Storage projection has `dht_anchor_hash` pointing to the PARENT entry's ActionHash
- Storage projection denormalizes for query convenience, but the link is the truth

**When to use A2 instead of A**: If the entity has no meaning without its parent (a tag without content, a step without a path, an allocation without an agreement), it's derived, not standalone.

### Category B: Agent-Scoped (private)

**Examples**: user preferences, display settings, schedules, session state, draft content, personal bookmarks.

**Test**: Does this data belong to one agent and only matter to them? Would other peers never need to validate it?

**Requirements**:
- Private source-chain entry on Holochain (not gossipped to DHT)
- Linked to notarized content by `EntryHash` where applicable
- SQLite projection exists for fast local query only — it is **not** the source of truth
- If the agent migrates devices, this data travels via source-chain export/import
- No HTTP route exposes this to other agents (only the owning agent's UI reads it)

### Category B2: Agent-Scoped with Notarized Attestation

**Examples**: content mastery (private progress, but gates governance participation), votes (private ballot, but tally must be verifiable), assessment responses (private attempt, but credential is public).

**Test**: Does this data belong to one agent, BUT does its effect need to be verifiable by peers?

**Requirements**:
- Raw data is a **private source-chain entry** (Category B)
- When the raw data produces a verifiable result (mastery level, vote tally, credential), a signed **Attestation** is issued on the DHT (using the existing Attestation entry type in imagodei)
- The Attestation is the public proof. The raw data stays private.
- Storage projection for the raw data is agent-scoped. Storage projection for the attestation has `dht_anchor_hash`.

**Pattern**: Agent records private data → system evaluates → system issues Attestation → Attestation is notarized. This avoids putting granular data (every quiz answer, every scroll event) on the DHT while still providing verifiable proofs of outcomes.

### Category C: Operational

**Examples**: cache entries, materialized views, temporary computation state, rate-limit counters, connection pool metadata.

**Test**: Could this data be deleted and reconstructed from notarized or agent-scoped sources? Is it ephemeral?

**Requirements**:
- SQLite-only is acceptable
- MUST document in a code comment why this entity is operational (not notarized or agent-scoped)
- No `dht_anchor_hash` column
- Must declare a reconstruction strategy (how to rebuild from source-of-truth data if lost)

### Decision Flowchart

```
Does the community need to witness/verify this data?
  YES → Does a DHT entry type already exist for this?
          YES → NOTARIZED (Category A)
          NO  → Is it a relationship/attribute of an existing entry?
                  YES → DERIVED (Category A2 — use Link, not new entry type)
                  NO  → Is there DNA headroom? (Lamad: 83/~100)
                          YES → NOTARIZED (Category A — create entry type)
                          NO  → STOP. Refactor existing types or split DNA.
  NO  → Does this data belong to a single agent privately?
          YES → Does its EFFECT need peer verification?
                  YES → AGENT-SCOPED + ATTESTATION (Category B2)
                  NO  → AGENT-SCOPED (Category B)
          NO  → Is it reconstructable from other sources?
                  YES → OPERATIONAL (Category C)
                  NO  → Go back. You missed something. It's probably A or A2.
```

---

## Step 2: Content Address Strategy

For each entity, declare which addressing strategy applies. There are exactly three options.

### Option 1: Content-Derived (CID)

The identity of the entity IS a hash of its content. If the content changes, the address changes — you get a new version, not a mutation.

**Use when**: The entity represents immutable content (articles, assessments, media, attestations). The canonical format is CIDv1 (`bafkrei...`).

**Implication**: No `UPDATE` semantics. New version = new CID. Version chains link CIDs together.

### Option 2: Agent-Scoped Composite

The identity is a tuple of `(AgentPubKey, ContentEntryHash, type_discriminator)`. The agent's relationship to a piece of content is the identity.

**Use when**: The entity represents an agent's stance toward content — a vote, a bookmark, an assessment attempt, a stewardship claim. Two agents holding the same stance toward the same content produce two different entries.

**Implication**: Uniqueness is enforced by the tuple. Lookup is always "agent X's relationship to content Y of type Z."

### Option 3: Slug or UUID

A human-readable slug or a random UUID serves as the identifier.

**Use when**: Neither content-derived nor agent-scoped composite applies. This is rare in the Elohim Protocol. You MUST justify why Options 1 and 2 do not apply.

**Common justifications**:
- Operational entity with no content to hash (e.g., a session token)
- Human-navigable identifier required before content exists (e.g., a community slug for URL routing)
- External system integration where the external ID is the canonical reference

---

## Step 3: API Design Order

Design the API layers in this exact sequence. Do not skip ahead.

### 3a. Holochain Coordinator Function

What zome function creates or reads this entry?

```
coordinator zome: {zome_name}
  create_{entity}(input: Create{Entity}Input) -> EntryHash
  get_{entity}(hash: EntryHash) -> Option<{Entity}>
  // For agent-scoped: get_my_{entity}s() -> Vec<Link>
```

Define the entry type, its validation, and its links FIRST.

### 3b. Post-Commit Signal and Storage Projection

What signal does the post-commit hook emit? What does elohim-storage do when it receives it?

```
post_commit: emit Signal::{Entity}Created { entry_hash, entry }
storage handler: INSERT INTO {table} (..., dht_anchor_hash) VALUES (..., ?)
```

For Notarized entities, the storage projection is a read-optimized cache. For Agent-Scoped entities, the storage projection is a local convenience index.

### 3c. HTTP Route (LAST)

Only after 3a and 3b are defined, design the HTTP route that exposes the projection.

```
GET  /api/{entity}/{id}        -> StorageProjection
POST /api/{entity}             -> Calls coordinator create, returns EntryHash
```

The HTTP route serves the **projection**, not the source of truth. The route is the thinnest possible layer — validation and business logic belong in the coordinator zome.

**Why this order matters**: Starting with HTTP routes produces REST-shaped designs where the database is the source of truth. Starting with DHT entry types produces P2P-native designs where the network is the source of truth and everything else is a projection.

---

## Anti-Pattern Catalog

These are known regressions — design choices that have caused real bugs or architectural debt in this codebase. Check every entity against this table.

| Anti-Pattern | Why It Fails | Correct Approach |
|---|---|---|
| UUID primary key for a notarized entity | The EntryHash IS the identity. A UUID creates a second identity that can drift out of sync with the DHT. | Use `dht_anchor_hash` as the logical primary key. SQLite rowid is internal only. |
| REST route as the design starting point | Produces server-centric designs where the database is truth. Holochain becomes an afterthought bolted on later. | Start with the DHT entry type. The HTTP route is the last layer designed. |
| CID stored as a relational foreign key | The entity IS its content address. Storing a CID as an FK in another table creates a dangling reference when the content is versioned. | Use Holochain links between EntryHashes. Storage projections denormalize for query convenience. |
| Standalone table for agent state | Agent preferences/bookmarks/drafts in a shared table leak private data and create P2P sync conflicts. | Private source-chain entry with local storage projection. No shared table. |
| Three address formats left undefined | The same entity referenced by CID in one place, UUID in another, and slug in a third. Conversion bugs everywhere. | Declare one canonical address format per entity. Document it. All other formats are display aliases resolved at the edge. |
| Missing source-of-truth declaration | A table exists but nobody documented whether Holochain or SQLite is authoritative. Bugs appear when they disagree. | Every table's migration or schema file includes a comment: `-- Source of truth: DHT` or `-- Source of truth: local (operational)`. |
| Creating new entry type when one already exists | Lamad DNA is at 83/~100 entry types. Adding another wastes scarce capacity and fragments the data model. | Check existing entry types first. Use Links (Category A2) for relationships. Only create new types if nothing fits and DNA has headroom. |
| Putting granular data on the DHT | Every quiz answer, scroll event, or preference on the DHT bloats gossip and exceeds the ~3000 entry budget. | Agent-scoped with attestation (Category B2): raw data stays private, signed proof of outcome is notarized. |

---

## Output Format

When the gate is complete, present the result in this format before proceeding to design proposals.

```
## P2P Design Gate: {Feature Name}

### Entity: {EntityName}
- **Classification**: Notarized (A) | Derived (A2) | Agent-Scoped (B) | Agent-Scoped+Attestation (B2) | Operational (C)
- **Justification**: {1-2 sentences on why this classification}
- **Content Address Strategy**: Content-Derived (CID) | Agent-Scoped Composite | Slug/UUID
- **Address Justification**: {Why this strategy, not the others}
- **Source of Truth**: Holochain DHT | Private Source Chain | SQLite (operational)
- **Coordinator Zome**: {zome_name}::{function_name}
- **Storage Projection**: {table_name} (dht_anchor_hash: yes/no)
- **HTTP Route**: {method} {path}
- **Anti-Pattern Check**: {Confirmed none apply, or list which were caught and corrected}

### Entity: {NextEntityName}
... (repeat for each entity)

### Design Constraints Discovered
- {Any cross-entity relationships, ordering dependencies, or migration concerns found during the gate}
```

Only after this output is complete and reviewed should design proposals (schemas, migrations, component architecture) proceed.

---

## Key Files

| File | Purpose |
|------|---------|
| `elohim/elohim-storage/src/views.rs` | Rust view types with `#[derive(TS)]` — the Rust-to-TypeScript boundary |
| `elohim/elohim-storage/src/migrations/` | SQLite migrations — every table must declare source of truth |
| `elohim/sdk/storage-client-ts/src/generated/` | Auto-generated TypeScript types from Rust views |
| `app/elohim-app/src/app/elohim/adapters/` | Adapters that add computed fields — never transform wire format |
| `doorway/doorway-service/src/routes/` | HTTP routes — the thinnest layer, designed last |
| `genesis/docs/content/elohim-protocol/protocol-specification.md` | Full EPR protocol specification |
