---
name: p2p-design-gate
description: Mandatory gate for any feature design involving data entities (tables, models, routes, sync messages) OR identity/agency/role/capability framing. Forces P2P-native thinking — DHT entry types, content addressing, source-of-truth classification, and identity-ontology framing (imago-dei, not crypto self-sovereignty) — before proposing design approaches. Use when brainstorming any feature that creates, stores, references, or syncs data entities, or that names an identity/agency tier.
metadata:
  sourceRuntime: claude
  master: package
  governance: "epr:elohim-agent/skills/p2p-design-gate"
---

# P2P Design Gate

This skill is a **mandatory checkpoint** during feature design. It fires between brainstorming (understanding what we need) and design proposal (how we build it). No data entity may be proposed — no table, no model, no route, no sync message — without passing through this gate first.

## When This Gate Fires

This gate is **not optional**. It activates whenever a design conversation involves:

- Creating a new database table or migration
- Defining a new model, struct, or TypeScript interface for persistent data
- Adding an HTTP route that serves or mutates data
- Designing a sync/gossip message between peers
- Introducing a new decision predicate, verdict fn, boundary answer type, or reason/outcome enum (Step 4 walks the concern canon)
- Proposing a new "entity" of any kind

**Sequence**: The gate sits between step 2 (understanding the domain need) and step 3 (proposing a design). You must complete the gate output before writing any schema, migration, or route code.

If you find yourself reaching for `CREATE TABLE` or `#[derive(Serialize)]` before completing this gate, stop. Back up. Run the gate.

---

## DHT Capacity Constraints (READ FIRST)

The Holochain DHT is a **notary, not a database**. Hard constraints shape every classification decision:

| Constraint | Limit | Current State |
|---|---|---|
| Entry types per DNA | ~100 | Lamad: **~73** (freed by mishpat split), Mishpat: **11**, Imagodei: 28, Infrastructure: 6 |
| Total DHT entries | ~3000 before degradation | Designed for 100s-1000s |
| Entry size | <1KB target | Proofs only: who (agent key), what (content hash), when (timestamp) |
| Query capability | None — link traversal only | No SQL, no pagination, no filtering |
| Gossip latency | 200-2000ms | Unacceptable for real-time reads |

**Before proposing ANY new DHT entry type**: Check if the DNA has headroom. Lamad at ~73/~100 means you have some breathing room after the mishpat split, but still check existing entry types first. Mishpat (governance) is at 11/~100 with room for growth. Most entities that need notarization already have entry types, not create one. Most entities that need notarization already have entry types — the gap is usually the missing `dht_anchor_hash` in the storage projection, not a missing entry type.

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
                  NO  → Is there DNA headroom? (Lamad: ~73/~100, Mishpat: 11/~100)
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

#### Canonical address forms — CID IS the address; sha256 is only the hash *inside* it

**The recurring mistake this kills:** exposing a bare `sha256-<hex>` (or a UUID) as a content/blob *address*, or putting a `sha256-<hex>` value in a field named `cid`. A CID is not a *different* hash — it is the **same sha2-256 of the bytes, wrapped in a self-describing multihash + codec**. "Use a CID" means *stop exposing the bare hash; expose the CID that wraps it.*

| What is being addressed | Canonical form | How it is minted | Example |
|---|---|---|---|
| Atom / DAG-CBOR content (EPR heads & atoms, manifests, content-set fingerprints, projection digests) | **CIDv1, dag-cbor codec** → `bafyrei…` | `Cid::new_v1(0x71 dag-cbor, Sha2_256(canonical-bytes))` — see `elohim-storage/src/epr_codec.rs` (`DAG_CBOR_CODEC`) | `bafyrei…` |
| Raw blob bytes | **CIDv1, raw codec** → `bafkrei…` | `Cid::new_v1(0x55 raw, Sha2_256(bytes))` — see `doorway/src/routes/blob.rs`; the **same** sha256 you already compute, wrapped | `bafkrei…` |
| Agent / action identity | **Holochain hash** → `uhCAk…` (agent key), action hash for actions | conductor-minted; NOT a CID, NOT a bare sha | `uhCAk…` |

**NOT addresses — keep these as bare sha256.** The discriminator: *does something resolve / dereference / fetch it?* If no, it is not an address — leave it.
- **Dedup / fingerprint keys** — e.g. `fp = sha256(node|class|provenance)[:12]` (findings / runtime sentinels). An internal index key, never fetched.
- **Byte-equality verification** — e.g. `sha256-verify the ts-rs codegen diff`, blob-arrival integrity checks. Comparing bytes, not naming content.
- **Cite fingerprints** — `cites:` frontmatter `sha256:<hex>` is now formally the **short-form projection of the body CID** (`CIDv1(raw 0x55, sha2-256(canonical-body))`) per the 2026-07-12 cite↔CID convergence — one digest, two renderings, not a separate system. Envelopes stay short-form; it is still tool-generated (`cite-gen`) — **never hand-edit.**

**Legacy / in-migration:** the bare `sha256-<hex>` blob wire marker (currently described in `elohim-storage/CLAUDE.md` / `doorway/CLAUDE.md`, on the `/blob/<hash>` path) is the **legacy** form; the canonical target is the wrapping CID `bafkrei…`. Moving the blob plane (`/blob`, `BlobStore`, inventory-gossip wire, seeder) from bare-hash to CID is a named **downstream migration arc** — describe *current* behavior accurately, design *new* surfaces CID-first.

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

## Step 4: Concern-Canon Answer (the birth rule)

Steps 1-3 classify and place a data entity. A **decision predicate, verdict fn,
boundary answer type, sync/gossip message, or HTTP route** additionally answers the
concern canon — sixteen recurring failure classes (C0 plane location; C1
anti-self-election; C2 monotonic authority; C3 liveness; C4 honest absence; C5
evidence-not-authority; C6a bounded work / C6b idempotent effect; C7 advertise/serve
symmetry; C8 observability-per-decision; C9 identity-lineage continuity; C10
contract-evolution honesty; C11 externally-imposed backpressure; C12
consent/authorization; C13 graduated authority; C14 witnessed residual) mined from
repeated production incidents across every substrate family. This is the birth rule
the seam-concern contract architecture exists to enforce: **a concern solved once
must not be rediscovered bespoke at the next seam** — answer it (or justify
`n-a`) here, before the code is written, not after the third incident.

Two moves, both cheap now and expensive later:

1. **Answer every class, don't skip.** For C0-C14 (16 ids — C6 splits into
   C6a/C6b), record one of the registry's own four states — `answered` (logic
   plus a contract test pin it) / `partial` (a real, named gap) / `unbound` (the
   concern is the definitional question here and nothing currently answers it) /
   `n-a` (state why it doesn't apply — a silent skip is exactly what this step
   exists to catch). Read the guarantee from its canon home, don't restate from
   memory: predicate-bearing classes are enforcement rows in
   `.claude/epr-meta/policies.yaml` (currently C2, C6a); the rest are
   Precedent-shaped rows in `.claude/epr-meta/concerns.yaml`.
2. **Register the point at birth.** Add a row to the crate's `seam-registry.yaml`
   (schema: `elohim/sdk/schemas/v1/manifest/seam-registry.schema.json`; create the
   file if this crate has none yet) — name, kind (`pure-decision-predicate` /
   `verdict-fn` / `boundary-answer-type` / `reason-outcome-enum`), source location,
   the concern answers from move 1, and `contractTests` (explicit `null` +
   `gapNote` when none exists yet — never a silently-omitted field).

**The census is the enforcement backstop, not a courtesy reminder.**
`placement-audit.py --epr-meta` reads every crate's `seam-registry.yaml` and fails
loud on a missing registration, an uncited contract test, or a mirrored test (the
anti-mirror fixture — a passing test measuring the same helper the code under
test uses). The co-located `.epr-meta` `inject` rules on decision-surface `.rs`
diffs (e.g. `doorway/doorway-service/src/.epr-meta`, `steward/node/src/.epr-meta`,
`crates/seam-contracts/.epr-meta`) are this step's edit-time half — they fire
on the same shapes this step asks you to walk through at design time. Skip this
step and the gap doesn't vanish; it surfaces later, at the census or in
production, which is exactly the cost this plan exists to avoid.

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
| Bare `sha256-<hex>` exposed as a content/blob address, or a `cid:` field holding a `sha256-<hex>` | A bare hash is not self-describing (no codec, no hash-fn tag) and silently competes with the CID it should be. Calling a sha a "cid" is the conflation that recurs. | Expose the CID that wraps the SAME bytes: `bafyrei…` (dag-cbor) for atoms/content, `bafkrei…` (raw) for blobs — sha2-256 is only the multihash inside it. Bare sha is for dedup keys / byte-verify / cite-fps only, never an address. See Step 2 "Canonical address forms." |
| An amber/green (DHT-agreement) trust signal *written* as a per-host "mode" instead of *derived*; per-host authoring of a notarized field | amber/green means "the DHT has witnessed this head's claims in network context" (green) vs "not yet" (amber) — it is a DERIVED read signal, never a write. Writing it (e.g. a per-storage diesel-direct `?deployTier=amber` PATCH stamping `crdt_converged_at`, never `dht_anchor_hash`) mints a divergent, un-witnessed head on EACH backend that can never green and never converges — the `public@adam` vs `private@matthew` / elohim.host "stuck" class. The doorway/storage is a gateway, never the witness. | Author a notarized field ONCE through the conductor (the DHT witnesses it; gossip / `run_content_sweep` converges it to every peer — no conductor bridge → fail loud & fail over, never a local un-witnessed write). Derive amber/green from `dht_anchor_hash` presence (the DHT witnessing) — never a stored write-choice and never a value-equality check. (The EPR content CID does NOT carry peer perspective: the signer is EXCLUDED from the canonical hash — `elohim/epr/src/envelope.rs:58-60` — and rides the detached `proof` / the `dht_anchor_hash` ActionHash, so two peers signing identical content get the SAME CID; the claims-coupling, by contrast, is baked INTO that shared CID.) The correct model is NOT per-host at all: ONE blob is seeded once and synced across peers by the p2p dataplane, servable as soon as present (amber), DHT-notarized → green — never two different blobs after seeding. Any per-host byte-seeding is an interim SCAFFOLD (p2p byte-replication is unbuilt — `project_inventory_exchange_not_byte_replication`) to be deleted, never the design, and NEVER a per-host head/claim write. Multiple simultaneous versions are legitimate ONLY as an intentional declared-HEAD collective-edit DAG. |
| Missing source-of-truth declaration | A table exists but nobody documented whether Holochain or SQLite is authoritative. Bugs appear when they disagree. | Every table's migration or schema file includes a comment: `-- Source of truth: DHT` or `-- Source of truth: local (operational)`. |
| Creating new entry type when one already exists | Lamad DNA is at ~73/~100, Mishpat at 11/~100. Adding another wastes scarce capacity and fragments the data model. | Check existing entry types first. Use Links (Category A2) for relationships. Only create new types if nothing fits and DNA has headroom. |
| Putting granular data on the DHT | Every quiz answer, scroll event, or preference on the DHT bloats gossip and exceeds the ~3000 entry budget. | Agent-scoped with attestation (Category B2): raw data stays private, signed proof of outcome is notarized. |
| Cross-namespace identity string-equality | The same agent has three identities (Holochain `uhCAk…` key, libp2p `12D3Koo…` peer id, iroh NodeId). Joining/matching one against another by raw string silently empties the join (caused the all-zeros resilience card, repeatedly). | Resolve through the canonical agent↔transport resolver (the `AgentPeerBinding` projection / `peer_transport_manifest`). Never string-compare identities across namespaces; pick `agent_cid` as the canonical join key and resolve transport ids TO it. |
| `self-sovereign` / "true data sovereignty" as the **apex** identity, agency, or capability tier | Imports the silicon-crypto sovereignty ontology the protocol explicitly *subordinates* to community governance. The apex-tier label reads as a neutral capability level ("more keys → more autonomy → higher") and sails past review as a load-bearing ontological claim. Also silently excludes everyone who holds the right *through others* (children, IDD, seniors, wards). | Frame the high-autonomy tier as **community-grounded** (e.g. *node-stewardship standing*), not *self-sovereign*. Key-location is a mechanical fact (`custodial` → on-device → always-on), not an ascent toward sovereignty. Sovereignty is the *adversary frame*, never the protocol's own apex. Confirm the corrected lexicon with the architect. See "Identity Ontology Guard." |

---

## Identity & Transport-Identity Coherence

`agent_cid` (`uhCAk…`) is the canonical agent identity throughout the protocol. The libp2p peer id (`12D3Koo…`) and the iroh `NodeId` are transport-plane identities that resolve TO `agent_cid` — they are NOT interchangeable with it.

The resolution substrate is the notarized `AgentPeerBinding` DHT integrity entry (projected by `ReconcileController::on_agent_peer_binding` into the `peer_identity_bindings` table), materialized locally in `peer_transport_manifest` (`elohim/elohim-storage/src/p2p_iroh/peer_map.rs`).

**Rule**: any new entity that references a peer, provider, or steward identity must declare which namespace it stores, and must resolve through the canonical resolver when joining or matching across namespaces. Never raw-compare `agent_cid` against a transport id.

---

## Identity Ontology Guard (imago-dei floor, not crypto self-sovereignty)

This gate guards the **framing** of identity, not only its addressing. A human's identity in this protocol is *imago dei* — an inviolable right **backstopped by community and institutional expression**, not a self-asserted cryptographic primitive. Individual sovereignty is **subordinated** to community-adjudicated governance; it is never the apex value. **Canonical home:** `genesis/docs/architecture/stewardship-over-sovereignty.md` (Canon status: Foundational, "read it first" — *"We do not consider sovereignty itself to be the right framing"*; §3 reserves *stewardship / agency / authority* with discipline and explicitly excludes "sovereignty") and its life-stage companion `genesis/docs/architecture/cradle-to-grave-capability-gradient.md`. The imagodei domain gospel echoes it: identity grounded in "demonstrated capability and community trust."

**The recurring drift this kills:** naming the top of an identity / agency / capability gradient `self-sovereign`, or celebrating "true data sovereignty" as an achievement. AI agents default to the silicon-crypto sovereignty ontology because it dominates the training data, and it slips past review **at tier-naming** — an apex tier called "self-sovereign" reads as a neutral capability level ("more keys → more autonomy → higher") rather than the load-bearing ontological claim it actually is. This is the identity-vocabulary sibling of the relational-default mistakes above.

**Rules when designing ANY identity, agency, role, capability, or key-location entity** (enum, struct, schema, tier ladder — or the prose that documents one):

- **Never make `sovereignty` / `self-sovereign` the positive apex** of a gradient. Frame the high-autonomy tier as community-grounded — e.g. *node-stewardship standing*, not *self-sovereign*. Key-location is a mechanical fact (`custodial` → on-device → always-on); do not dress it as an ascent toward sovereignty.
- **Sovereignty as an adversary frame is correct.** Quoting "the crypto sovereignty frame" as the thing being *resisted*, or modeling state/platform sovereignty as a threat, is fine. Asserting it as the protocol's *own* top value is the drift.
- **Model the whole human life.** The ontology must hold for those who exercise the right *with/through others* — children, IDD, seniors, legal wards. The protocol expresses this as **graduated, mediated agency** (ward = "mediated agency, guardian co-authors"; voice-retention for seniors; supported decision-making) — the canonical §2 life-stage transition map is `genesis/docs/architecture/cradle-to-grave-capability-gradient.md`. An identity model whose apex is "full autonomy, keys on device" silently excludes them. Note: this is canon-written, concrete ward/guardian specs pending — there is no guardian/ward DHT *entity* built yet, and `custodial` key-holding is device-convenience, NOT incapacity-guardianship; do not conflate the two.
- **Cryptography is an ACCELERATOR of community recovery, never the gate** — Shamir, threshold signatures, hardware-rooted attestation all *speed* the recovery paths but their absence must never prevent recovery (`stewardship-over-sovereignty.md` §4; `project_socially_derived_security`). So "harden a high-risk user" (e.g. a dissident under state duress) means *optional* cryptographic hardening layered onto the social-recovery floor — NOT elevating them to a "more sovereign" tier. The protocol's standing defense against duress is the non-firable elohim-counsel (`project_elohim_as_counsel`), not stronger keys.

If a design names a tier `self-sovereign`, proposes `sovereignty`-as-achievement, or treats individual autonomy as the protocol's ceiling: **stop, reclassify the framing, and confirm the corrected apex lexicon with the architect** before writing the enum/schema. This gate stops *new* design-time bleeding; the existing leaks (the live agency ladder, `hardware-spec.md` "true data sovereignty") are a separate de-drift/rename pass.

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
| `.claude/epr-meta/policies.yaml` + `.claude/epr-meta/concerns.yaml` | The concern canon (C0-C14) — enforcement rows (predicate-bearing classes) vs Precedent-shaped rows; Step 4's canon homes |
| `elohim/sdk/schemas/v1/manifest/seam-registry.schema.json` | Per-crate decision-point registry schema — where Step 4's registration lands |
| `.claude/scripts/memory-kit/placement-audit.py` (`--epr-meta`) | The census — the enforcement backstop for Step 4, reads every crate's `seam-registry.yaml` |
| `genesis/docs/content/elohim-protocol/protocol-specification.md` | Full EPR protocol specification |
