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

## Step 1: Entity Classification Decision Tree

Every data entity falls into exactly one of three categories. Walk the tree for each entity in the design.

### Category A: Notarized

**Examples**: content items, economic events (REA), attestations, stewardship allocations, governance proposals, votes, relationships between agents.

**Test**: Would the protocol be lying if this data were silently changed or deleted? Is this something the community needs to witness and verify?

**Requirements**:
- MUST have a Holochain DHT entry type defined in a coordinator zome
- MUST have `dht_anchor_hash NOT NULL` in the SQLite storage projection
- Source of truth is **Holochain DHT** — the SQLite row is a read-optimized projection, not the canonical record
- Entry validation callback must exist (even if permissive initially)
- Post-commit signal projects the entry to elohim-storage for fast query

### Category B: Agent-Scoped

**Examples**: user preferences, display settings, schedules, session state, draft content not yet published, personal bookmarks.

**Test**: Does this data belong to one agent and only matter to them? Would other peers never need to validate it?

**Requirements**:
- Private source-chain entry on Holochain (not gossipped to DHT)
- Linked to notarized content by `EntryHash` where applicable
- SQLite projection exists for fast local query only — it is **not** the source of truth
- If the agent migrates devices, this data travels via source-chain export/import
- No HTTP route exposes this to other agents (only the owning agent's UI reads it)

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
  YES --> NOTARIZED (Category A)
  NO  --> Does this data belong to a single agent privately?
            YES --> AGENT-SCOPED (Category B)
            NO  --> Is it reconstructable from other sources?
                      YES --> OPERATIONAL (Category C)
                      NO  --> Go back. You missed something. It's probably Notarized.
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

---

## Output Format

When the gate is complete, present the result in this format before proceeding to design proposals.

```
## P2P Design Gate: {Feature Name}

### Entity: {EntityName}
- **Classification**: Notarized | Agent-Scoped | Operational
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
