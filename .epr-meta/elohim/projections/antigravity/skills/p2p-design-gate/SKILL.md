---
name: p2p-design-gate
description: Mandatory gate for any feature design involving data entities (tables, models, routes, sync messages) OR identity/agency/role/capability framing. Forces P2P-native thinking — DHT entry types, content addressing, source-of-truth classification, and identity-ontology framing (imago-dei, not crypto self-sovereignty) — before proposing design approaches. Use when brainstorming any feature that creates, stores, references, or syncs data entities, or that names an identity/agency tier.
metadata:
  runtime: antigravity
  sourceRuntime: claude
  master: package
  sourcePath: .epr-meta/elohim/packages/skills/p2p-design-gate.json
  packageKind: SkillPackage
governance: "epr:elohim-agent/skills/p2p-design-gate"
---

# P2P Design Gate

This skill is a **mandatory checkpoint** during feature design. It fires between brainstorming (understanding what we need) and design proposal (how we build it). No data entity may be proposed — no table, no model, no route, no sync message — without passing through this gate first.

It exists to interrupt one specific reflex: AI agents reach for relational-database shapes (a table, a UUID primary key, a REST route) because that is what dominates the training data. This protocol needs the opposite starting point. The same reflex has a second flavor worth naming here — modeling a protocol concern in the k8s/deployments plane, which describes compute and hardware only; the protocol-native home is the DHT/REA substrate ([[feedback_k8s_is_not_the_architecture]]).

## When This Gate Fires

This gate is **not optional**. It activates whenever a design conversation involves:

- Creating a new database table or migration
- Defining a new model, struct, or TypeScript interface for persistent data
- Adding an HTTP route that serves or mutates data
- Designing a sync/gossip message between peers
- Introducing a new decision predicate, verdict fn, boundary answer type, or reason/outcome enum (Step 4 walks the concern canon)
- Proposing a new "entity" of any kind

**Sequence**: The gate sits between step 2 (understanding the domain need) and step 3 (proposing a design). You must complete the gate output before writing any schema, migration, or route code. The Output Format below carries a back-fill detector, because prose alone does not enforce sequence.

If you find yourself reaching for `CREATE TABLE` or `#[derive(Serialize)]` before completing this gate, stop. Back up. Run the gate.

---

## What the DHT Is (READ FIRST)

The Holochain DHT is a **notary, not a database**. Two distinct cost planes shape every classification decision, and an earlier revision of this section conflated them.

**Plane 1 — the notary plane:**

| Constraint | Limit | Consequence for design |
|---|---|---|
| Entry size | <1KB target | Proofs only: who (agent key), what (content hash), when (timestamp) |
| Query capability | None — link traversal only | No SQL, no pagination, no filtering |
| Gossip latency | 200-2000ms | Unacceptable for real-time reads |

**Plane 2 — the head plane. Storage is not the scarce resource; recurring per-item attention is.** Every Notarized/Linked item ALSO becomes a local head row swept by the reconciliation arms, a per-item libp2p Kad record re-provided on the drain ticker (~15s), an election candidate, and a conductor round-trip consumer — forever, per item, on every peer. The measurement (Row 16 evidence pass, 2026-08-08): ~3,469 A-class content heads at genesis quiesce cost ~2.5h after a deploy, at 200 heads/tick on a 300s sweep cadence with one uncancellable conductor WS round-trip per head. The retired "~3000 entries before degradation" line was this plane's cost wearing the notary plane's costume. Step 1.5 prices it.

**Capacity is a footnote, not a gate.** The real ceiling is `EntryDefIndex(pub u8)` — 256 entry types per integrity zome. No DNA is anywhere near it, which is exactly why capacity must never be the deciding question: "there is room" is not a reason to mint a type, and "we are running low" is not the reason to reuse one. Counts age; do not trust a tally written in a prompt — read them where they live (`#[hdk_entry_types]` in each DNA's `*_integrity` zome; note lamad is packed from `dna/elohim/`, and node-registry has its own set that discussions routinely forget). Step 1 asks the two questions that actually decide, in this order: *is this an attribute of something already notarized* (which classifies it), and only then *does a type already exist to reuse* (which prices it). One more shape to know: **entry types migrate between DNAs.** Attestations, proposals, challenges, governance reactions, and votes were consolidated off imagodei/mishpat/infrastructure onto the elohim DNA, leaving thin bridge views behind. Extending the consolidating home plus a bridge view beats minting a fresh type on a second DNA.

## Step 1: Entity Classification Decision Tree

Every data entity falls into exactly one of five categories. The letters are shorthand; the names are the meaning.

| Category | The test | Source of truth | Head-plane footprint |
|---|---|---|---|
| **Notarized (A)** | The protocol would be lying if this changed silently — and it is a thing in its own right | DHT entry | own entry + own head |
| **Linked (A2)** | Same lying test, but it is a relationship or attribute of something already notarized | Holochain Link on the parent entry | link tag, no new head |
| **Private (B)** | Belongs to one agent; no peer ever needs to validate it | private source chain | none |
| **Attested-Private (B2)** | Belongs to one agent, but its *effect* must be verifiable by peers | private source chain + a notarized attestation | attestation only |
| **Ephemeral (C)** | Delete it and rebuild it from A / A2 / B | SQLite | none |

### Notarized (A)

**Examples**: content items, economic events (REA), relationships between agents.

**Test**: Would the protocol be lying if this data were silently changed or deleted — and is it a thing in its own right rather than an attribute of one? That is the whole classification test. *Whether an entry type already exists* is the next question, not part of this one: a genuinely new notarized thing is still Notarized (A); it just costs a new entry type, which the flowchart prices separately.

**Requirements**:
- Uses an EXISTING Holochain DHT entry type where one fits (check first — see the capacity footnote's migration shape)
- MUST have `dht_anchor_hash NOT NULL` in the SQLite storage projection **once witnessed** — the contract holds from the moment the witness sweep anchors the row, not at insert. **Bulk-seed amber window (honesty clause):** a bulk-seeded Notarized row is born with `dht_anchor_hash` NULL and stays amber until the witness sweep reaches it — hours at the cadence Step 1.5 measures. Design reads to tolerate the amber window — derive amber/green from anchor presence (see the amber/green anti-pattern row) — instead of treating NULL as corruption or, worse, stamping the field locally
- Source of truth is **Holochain DHT** — the SQLite row is a read-optimized projection, not the canonical record
- Post-commit signal projects the entry to elohim-storage for fast query

### Linked (A2)

**Examples**: path chapters/steps (links on a LearningPath entry). (`stewardship_allocations` is *classified* here but is not actually anchored — see the Step 3a worked example; its schema FK is `content_id`, not an Agreement reference.)

**Test**: Does this data need notarization, but it is really a **relationship or attribute** of an already-notarized entity — not a standalone entity?

**Requirements**:
- Does NOT need its own DHT entry type — anchored via Holochain Link on an existing entry
- Link tag carries the metadata (type, weight, role — small, <256 bytes)
- Storage projection has `dht_anchor_hash` pointing to the PARENT entry's ActionHash
- Storage projection denormalizes for query convenience, but the link is the truth

**A2 instead of A**: if the entity has no meaning without its parent (a tag without content, a step without a path, an allocation without an agreement), it is derived, not standalone.

**C instead of A2**: an *authored* edge is notarized; a *computed* edge is not. The lamad relationship graph is deliberately resolved in native Rust rather than stored — a worked precedent for this fork ([[project_content_graph_native_rust_not_cozo_apollo]]). If nobody authored the edge, nobody needs to witness it.

### Private (B)

**Examples**: user preferences, display settings, schedules, session state, draft content, personal bookmarks.

**Test**: Does this data belong to one agent and only matter to them? Would other peers never need to validate it?

**Requirements**:
- Private source-chain entry on Holochain (not gossipped to DHT)
- Linked to notarized content by `EntryHash` where applicable
- SQLite projection exists for fast local query only — it is **not** the source of truth
- If the agent migrates devices, this data travels via source-chain export/import
- No HTTP route exposes this to other agents (only the owning agent's UI reads it)

### Attested-Private (B2)

**Examples**: content mastery (private progress, but gates governance participation), votes (private ballot, but the tally must be verifiable), assessment attempts (private, but the credential is public).

**Test**: Does this data belong to one agent, BUT does its effect need to be verifiable by peers?

**Requirements**:
- Raw data is a **private source-chain entry** (Private/B)
- When the raw data produces a verifiable result (mastery level, vote tally, credential), a signed **attestation** is notarized. Find where that attestation shape actually lives before designing against it — the identity-credential attestation is a content-typed entry on the elohim DNA (`attestation:identity-credential`), not a standalone `Attestation` type on imagodei; the same consolidation moved the vote entries. Grep the integrity zomes; do not trust a type name quoted from a prompt.
- The attestation is the public proof. The raw data stays private.
- Storage projection for the raw data is agent-scoped. Storage projection for the attestation has `dht_anchor_hash`.

**Pattern**: agent records private data → system evaluates → system issues an attestation → the attestation is notarized. This keeps granular data (every quiz answer, every scroll event) off the DHT while still providing verifiable proofs of outcomes.

### Ephemeral (C)

**Examples**: cache entries, materialized views, computed relationship edges, temporary computation state, rate-limit counters, connection pool metadata.

**Test**: Could this data be deleted and reconstructed from notarized or agent-scoped sources? Is it ephemeral?

**Requirements**:
- SQLite-only is acceptable
- MUST document in a code comment why this entity is ephemeral (not notarized or agent-scoped)
- No `dht_anchor_hash` column
- Must declare a reconstruction strategy (how to rebuild from source-of-truth data if lost)

### Decision Flowchart

```
Does the community need to witness/verify this data?
  YES -> Is it a relationship/attribute of an already-notarized entity?
          YES -> LINKED (A2 - use a Link, not a new entry type)
          NO  -> NOTARIZED (A). Now price it: does an entry type already exist?
                  YES -> reuse it. No DNA-hash move.
                  NO  -> Can it ride an existing type on the DNA that already
                         hosts its neighbours (extend + thin bridge view)?
                          YES -> extend that DNA's type
                          NO  -> new entry type. Declare the DNA-hash cost
                                 (Step 3a) and the head-plane cost (Step 1.5).
                                 Capacity is NOT the question.
  NO  -> Does this data belong to a single agent privately?
          YES -> Does its EFFECT need peer verification?
                  YES -> ATTESTED-PRIVATE (B2)
                  NO  -> PRIVATE (B)
          NO  -> Is it reconstructable from other sources?
                  YES -> EPHEMERAL (C)
                  NO  -> Go back. You missed something. It is probably A or A2.
```

---

## Step 1.5: Head-Plane Cost Budget (Notarized / Linked only)

Classification says WHERE truth lives; this step prices WHAT it costs to keep it live. Declare, for each A/A2 entity:

1. **Expected item count** — at seed and at 1 year (order of magnitude is fine; "unbounded" is an answer, and it triggers requirement 3).
2. **Which recurring costs it joins** — a conductor round-trip per item per sweep, a Kad record re-provided on the drain ticker, election candidacy, adjudication surface. There is no closed-form formula to plug into, and the anchor above is a single observed data point whose stated inputs do not by themselves reproduce its stated wall-clock (real sweeps contend with catch-up, gossip, and restart churn). Use the anchor for order-of-magnitude reasoning and say so; do not present an extrapolation from it as a computed number.
3. **Above ~500 items: a bundling justification or an explicit operator sign-off.** Name which bundling shape applies — **composite root** (many items under one declared head), **A2-via-link** (attribute of an existing head, no new head), or **corpus digest** (one fingerprint answers "are we in sync?" for the whole set) — or record the operator's sign-off treating the head-plane cost as declared stakes.

A design that passes Step 1 classification but adds thousands of per-item heads MUST state what that does to quiesce. "It's notarized, so it's correct" is not an answer to "what does the sweep pay every deploy?"

---

## Network Stakes Stage (the declared-stakes axis)

The network runs at a declared stage — `Simulacra < Bootstrap < Coordinated < Enforced` (`elohim/elohim-storage/src/trust/stage.rs`): `Simulacra` for dev/staging/genesis fixtures (cheap verification, fast CI loops — reached ONLY by explicit declaration, never a default, never derived from any `DEV_MODE`), `Enforced` for the live runtime where friction, negotiation, and trust-building are the product. Verification cost is priced against the declared stage (`genesis/docs/content/elohim-protocol/architecture/trust-as-efficiency-signal.md` §6; implementation program: `genesis/docs/superpowers/plans/2026-08-08-head-plane-trust-gradient-program-plan.md`) — the same machinery, priced by declared stakes, never a dev hack beside the real path.

Every new entity and decision predicate declares:

- **Which stages it must behave under.** Most entities: all four. A genesis-fixture-only surface may be Simulacra-only — but must say so explicitly.
- **Which of its costs are stage-priceable vs floor-protected.** A stage-priceable cost (e.g. full-chain re-verification of an already-witnessed, digest-matching head) may cheapen at lower declared stakes. A **floor-protected** cost NEVER cheapens at any stage, including Simulacra: `Constitutional` (manifests, attestations, delegations), `LocalRelationship` (local-relationship reach, unconditional), `CounterEvidence` (corrections always reach the creator, un-filterable). The floor invariant is pinned by property test (`trust::pricer` — `floor != None ⇒ FullChain` over the full stage×floor×reach×standing product).

---

## Step 2: Content Address Strategy

For each entity, declare which addressing strategy applies. There are exactly three options.

### Option 1: Content-Derived (CID)

The identity of the entity IS a hash of its content. If the content changes, the address changes — you get a new version, not a mutation.

**Use when**: the entity represents immutable content (articles, assessments, media, attestations). The canonical format is CIDv1 (`bafkrei...`).

**Implication**: no `UPDATE` semantics. New version = new CID. Versions form a DAG, not a line — and **which version applies is a DECLARED dependency, never resolved by recency**. A cid-pin is a lockfile: a consumer names the head it depends on, and "latest" is not an answer ([[project_versioned_entity_head_is_declared_dependency]]). Head-selection is a binding-layer decision your design must place, not a default it can inherit.

#### Canonical address forms — CID IS the address; sha256 is only the hash *inside* it

**The recurring mistake this kills:** exposing a bare `sha256-<hex>` (or a UUID) as a content/blob *address*, or putting a `sha256-<hex>` value in a field named `cid`. A CID is not a *different* hash — it is the **same sha2-256 of the bytes, wrapped in a self-describing multihash + codec**. "Use a CID" means *stop exposing the bare hash; expose the CID that wraps it.*

| What is being addressed | Canonical form | How it is minted | Example |
|---|---|---|---|
| Atom / DAG-CBOR content (EPR heads & atoms, manifests, content-set fingerprints, projection digests) | **CIDv1, dag-cbor codec** → `bafyrei…` | `Cid::new_v1(0x71 dag-cbor, Sha2_256(canonical-bytes))` — see `elohim/elohim-storage/src/epr_codec.rs` (`DAG_CBOR_CODEC`) | `bafyrei…` |
| Raw blob bytes | **CIDv1, raw codec** → `bafkrei…` | `Cid::new_v1(0x55 raw, Sha2_256(bytes))` — see `doorway/doorway-service/src/routes/blob.rs`; the **same** sha256 you already compute, wrapped | `bafkrei…` |
| Agent / action identity | **Holochain hash** → `uhCAk…` (agent key), action hash for actions | conductor-minted; NOT a CID, NOT a bare sha | `uhCAk…` |

**NOT addresses — keep these as bare sha256.** The discriminator: *does something resolve / dereference / fetch it?* If no, it is not an address — leave it.
- **Dedup / fingerprint keys** — e.g. `fp = sha256(node|class|provenance)[:12]` (findings / runtime sentinels). An internal index key, never fetched.
- **Byte-equality verification** — e.g. `sha256-verify the ts-rs codegen diff`, blob-arrival integrity checks. Comparing bytes, not naming content.
- **Cite fingerprints** — `cites:` frontmatter `sha256:<hex>` is formally the **short-form projection of the body CID** (`CIDv1(raw 0x55, sha2-256(canonical-body))`) per the 2026-07-12 cite↔CID convergence — one digest, two renderings, not a separate system. Envelopes stay short-form; it is tool-generated (`cite-gen`) — **never hand-edit.**

**Legacy / in-migration:** the bare `sha256-<hex>` blob wire marker (described in `elohim/elohim-storage/CLAUDE.md` / `doorway/CLAUDE.md`, on the `/blob/<hash>` path) is the **legacy** form; the canonical target is the wrapping CID `bafkrei…`. Moving the blob plane (`/blob`, `BlobStore`, inventory-gossip wire, seeder) from bare-hash to CID is a named **downstream migration arc** — describe existing behavior accurately, design *new* surfaces CID-first.

### Option 2: Agent-Scoped Composite

The identity is a tuple of `(AgentPubKey, target, type_discriminator)` — the agent's relationship to the target IS the identity. The target is usually a `ContentEntryHash`, but it may be **another `AgentPubKey`** when the stance is agent-toward-agent (an endorsement, a delegation, a trust assertion). Do not read the content-target case as the only one and fall through to Option 3; an agent→agent stance is still Option 2.

**Use when**: the entity represents an agent's stance toward something already addressed — a vote, a bookmark, an assessment attempt, a stewardship claim, a vouch for another agent. Two agents holding the same stance toward the same target produce two different entries.

**Implication**: uniqueness is enforced by the tuple. Lookup is always "agent X's relationship to content Y of type Z."

### Option 3: Slug or UUID

A human-readable slug or a random UUID serves as the identifier.

**Use when**: neither content-derived nor agent-scoped composite applies. This is rare in the Elohim Protocol. You MUST justify why Options 1 and 2 do not apply.

**Common justifications**:
- Operational entity with no content to hash (e.g., a session token)
- Human-navigable identifier required before content exists (e.g., a community slug for URL routing)
- External system integration where the external ID is the canonical reference

### Step 2b: Transport Affinity (byte-bearing entities only)

An address says *what* to fetch. It does not say *which swarm carries it*. Per-object `transport_affinity` (`libp2p` / `iroh` / `auto`) is a live field — `elohim/elohim-storage/src/{http_blob_router.rs, db/models.rs, db/peer_blob_inventory.rs}` — and `TransportBackend::Dual` boots both swarms co-resident. Declare the affinity a byte-bearing entity gets, and why `auto` is or is not right for it.

Two adjacent confusions to refuse while you do: notarization and byte-availability are **decoupled** — inventory gossip is metadata-only, so "source of truth is the DHT" never implies the bytes are reachable ([[project_inventory_exchange_not_byte_replication]]) — and reach, content_head, and replication are three orthogonal planes; collapsing any two of them is the recurring bug ([[feedback_reach_head_replication_distinct_planes]]).

---

## Step 3: API Design Order

Design the API layers in this exact sequence. Do not skip ahead.

### 3a. Integrity Zome, then Coordinator Function

Two zomes, two costs, and the split is a first-order decision — not an implementation detail:

- **Entry types, links, and validation live in the INTEGRITY zome.** Changing them moves the DNA hash → reinstall, new agent key, migration/lineage on prod. Label it `DNA-HASH-MOVING`.
- **Functions live in the COORDINATOR zome.** Coordinator-only changes never move the DNA hash and are healed by the `update_coordinators` hot-swap — no re-key, no DHT churn ([[project_dna_hash_blind_to_coordinator_zomes]]). Label it `DNA-hash-NEUTRAL`.

Declare which class your change is before writing code.

```
integrity zome: {integrity_zome}     // entry type + validation + link types
coordinator zome: {coordinator_zome}
  // Notarized (A) — creates an entry:
  create_{entity}(input: Create{Entity}Input) -> EntryHash
  get_{entity}(hash: EntryHash) -> Option<{Entity}>
  // Linked (A2) — creates a LINK, not an entry:
  create_{entity}(input: Create{Entity}Input) -> ActionHash   // create_link returns ActionHash
  get_{entity}s_for(base: EntryHash) -> Vec<Link>
```

**Zome names are not derived from the DNA directory — look them up.** The integrity zome is NOT `{dna_name}_integrity`. The elohim DNA's integrity zome is **`content_store_integrity`**, and that same DNA is called "lamad" in some docs and packed from `dna/elohim/` (a `dna/lamad-v1/` directory also exists and is a v1 archive, not the live one). Read the real names from `elohim/holochain/dna/*/dna.yaml` before writing either name down.

**Which hash you return is load-bearing, and it differs by classification.** For a Notarized entity, the `cid` is the **entry hash** and the `action_hash` is only ever the `dht_anchor_hash` — returning the wrong one passes every unit test and breaks every bounds-gate at integration ([[project_mishpat_commitment_cid_is_entry_hash]]). For a **Linked (A2)** entity there is no entry at all: `create_link` returns an **ActionHash**, and the `dht_anchor_hash` points at the PARENT entry. An A2 coordinator function that advertises `-> EntryHash` is a type lie.

**Worked example — what skipping 3a actually costs.** `stewardship_allocations` is classified Notarized/Linked, but no coordinator function creates it: there is no `create_stewardship_allocation` in any zome. The live path is `handle_create_allocation` (`elohim/elohim-storage/src/api/stewardship.rs`) — a direct HTTP→SQLite write with **no conductor call at all**, carrying its own TODO that `dht_anchor_hash` is never populated because no post-commit signal exists to populate it. The insert struct omits the field entirely.

That is this step's failure mode, shipped: the route and the table exist, the classification says DHT-notarized, and the anchor column is permanently NULL — so the entity can never green, and its "source of truth is the DHT" claim is unbacked. Nothing in the HTTP layer fails; the gap is only visible if you ask 3a's question first. Treat this as a known open gap to repair, not a pattern to copy.

### 3b. Post-Commit Signal and Storage Projections

What signal does the post-commit hook emit, and what does elohim-storage do when it receives it? There are **two** projections, not one — declare both.

```
post_commit: emit Signal::{Entity}Created { entry_hash, entry }
storage handler: INSERT INTO {table} (..., dht_anchor_hash) VALUES (..., ?)
```

1. **The SQLite projection** — read-optimized cache for Notarized entities; local convenience index for Private ones.
2. **The Automerge sync projection** — `spawn_content_projection_listener` (`elohim/elohim-storage/src/sync/projector.rs`) projects broadcast-tier Notarized content into a per-doc Automerge DocStore, gated by a fail-closed reach filter with an empty-never-projects invariant. State whether your entity projects here. **The reach-tier name is legitimately unanswerable right now** — reach vocabulary is in declared multi-vocabulary drift and `elohim/elohim-storage/CLAUDE.md` forbids canonizing any single vocabulary until it resolves. Write `unresolved — reach vocabulary in declared drift` rather than inventing a tier name; a made-up tier is a canonization this gate has no authority to make. (Links are neither content nor entries, so a Linked (A2) entity does not project here at all.)

### 3c. HTTP Route (LAST) — and you almost certainly do not write one

Only after 3a and 3b are defined, declare the HTTP route that exposes the projection.

```
GET  /api/{entity}/{id}        -> StorageProjection
POST /api/{entity}             -> Calls coordinator create, returns the 3a hash
```

**Declare it in elohim-storage's `build_manifest()` (`http.rs`) — do NOT add a file under `doorway/doorway-service/src/routes/`.** Routes are manifest-driven: a peer's storage declares them, the registry compiles them, the doorway serves them. Thirteen identical per-domain proxy files were deleted for this reason and must not come back; a doorway is not the author of substrate logic, it is one surface the substrate is reached through. You touch doorway-service only when the route needs doorway-*specific* logic — see `doorway/CLAUDE.md` for that decision criteria.

The HTTP route serves the **projection**, not the source of truth. It is the thinnest possible layer — and it is thin for a structural reason, not a stylistic one: the doorway *is* the federation layer, riding OVER the p2p substrate (DHT conductor + iroh/libp2p), not part of it ([[feedback_p2p_vs_federation_layer_vocabulary]]). Validation and business logic belong in the coordinator zome, one layer down.

**Why this order matters**: starting with HTTP routes produces REST-shaped designs where the database is the source of truth. Starting with DHT entry types produces P2P-native designs where the network is the source of truth and everything else is a projection.

## Step 4: Concern-Canon Answer (the birth rule)

Steps 1-3 classify and place a data entity. A **decision predicate, verdict fn, boundary answer type, sync/gossip message, or HTTP route** additionally answers the concern canon — sixteen recurring failure classes (C0 plane location; C1 anti-self-election; C2 monotonic authority; C3 liveness; C4 honest absence; C5 evidence-not-authority; C6a bounded work / C6b idempotent effect; C7 advertise/serve symmetry; C8 observability-per-decision; C9 identity-lineage continuity; C10 contract-evolution honesty; C11 externally-imposed backpressure; C12 consent/authorization; C13 graduated authority; C14 witnessed residual) mined from repeated production incidents across every substrate family. This is the birth rule the seam-concern contract architecture exists to enforce: **a concern solved once must not be rediscovered bespoke at the next seam** — answer it (or justify `n-a`) here, before the code is written, not after the third incident.

Two moves, both cheap now and expensive later:

1. **Answer every class, don't skip.** For C0-C14 (16 ids — C6 splits into C6a/C6b), record one of the registry's own four states — `answered` (logic plus a contract test pin it) / `partial` (a real, named gap) / `unbound` (the concern is the definitional question here and nothing answers it) / `n-a` (state why it doesn't apply — a silent skip is exactly what this step exists to catch). Read the guarantee from its canon home, don't restate from memory: predicate-bearing classes are enforcement rows in `.claude/epr-meta/policies.yaml`; the rest are Precedent-shaped rows in `.claude/epr-meta/concerns.yaml`.
2. **Register the point at birth.** Add a row to the crate's `seam-registry.yaml` (schema: `elohim/sdk/schemas/v1/manifest/seam-registry.schema.json`; create the file if this crate has none yet) — name, kind (`pure-decision-predicate` / `verdict-fn` / `boundary-answer-type` / `reason-outcome-enum`), source location, the concern answers from move 1, and `contractTests` (explicit `null` + `gapNote` when none exists yet — never a silently-omitted field).

**The census is the enforcement backstop, not a courtesy reminder.** `placement-audit.py --epr-meta` reads every crate's `seam-registry.yaml` and fails loud on a missing registration, an uncited contract test, or a mirrored test (the anti-mirror fixture — a passing test measuring the same helper the code under test uses). The co-located `.epr-meta` `inject` rules on decision-surface `.rs` diffs (e.g. `doorway/doorway-service/src/.epr-meta`, `steward/node/src/.epr-meta`, `crates/seam-contracts/.epr-meta`) are this step's edit-time half — they fire on the same shapes this step asks you to walk through at design time. Skip this step and the gap doesn't vanish; it surfaces later, at the census or in production, which is exactly the cost this gate exists to avoid.

---

## Anti-Pattern Catalog

These are known regressions — design choices that have caused real bugs or architectural debt in this codebase. Check every entity against this table.

| Anti-Pattern | Why It Fails | Correct Approach |
|---|---|---|
| UUID primary key for a notarized entity | The EntryHash IS the identity. A UUID creates a second identity that can drift out of sync with the DHT. | Use `dht_anchor_hash` as the logical primary key. SQLite rowid is internal only. |
| REST route as the design starting point | Produces server-centric designs where the database is truth. Holochain becomes an afterthought bolted on later. | Start with the integrity zome's entry type. The HTTP route is the last layer designed. |
| CID stored as a relational foreign key | The entity IS its content address. Storing a CID as an FK in another table creates a dangling reference when the content is versioned. | Use Holochain links between EntryHashes. Storage projections denormalize for query convenience. |
| Standalone table for agent state | Agent preferences/bookmarks/drafts in a shared table leak private data and create P2P sync conflicts. | Private source-chain entry with local storage projection. No shared table. |
| Three address formats left undefined | The same entity referenced by CID in one place, UUID in another, and slug in a third. Conversion bugs everywhere. | Declare one canonical address format per entity. Document it. All other formats are display aliases resolved at the edge. |
| Bare `sha256-<hex>` exposed as a content/blob address, or a `cid:` field holding a `sha256-<hex>` | A bare hash is not self-describing (no codec, no hash-fn tag) and silently competes with the CID it should be. Calling a sha a "cid" is the conflation that recurs. | Expose the CID that wraps the SAME bytes: `bafyrei…` (dag-cbor) for atoms/content, `bafkrei…` (raw) for blobs — sha2-256 is only the multihash inside it. Bare sha is for dedup keys / byte-verify / cite-fps only, never an address. See Step 2 "Canonical address forms." |
| "Is there DNA headroom?" used as the create-or-reuse decision | Capacity is not scarce (256 types per integrity zome) and a stale scarcity number produces the right answer for a reason that inverts the moment someone checks it. Reuse-because-crowded collapses into create-because-roomy. | Decide on the upstream tests, in order: is this an attribute of an already-notarized entity (A2 via Link), and only then does a type already exist to reuse? If you must create, declare the DNA-hash class (3a) and the head-plane cost (1.5) — those are the real prices. |
| Minting a fresh entry type on a second DNA when the concept already has a consolidating home | Fragments the data model across DNAs, doubles the validation surface, and inverts a consolidation the codebase already paid for. | Extend the DNA that already hosts the concept's neighbours and leave a thin bridge view behind — the shape used when attestations, proposals, and votes consolidated onto the elohim DNA. |
| Modeling a protocol concern in the k8s / deployments plane | k8s manifests describe compute and hardware only. A gap there is not a protocol gap, and a "fix" there is not a protocol design ([[feedback_k8s_is_not_the_architecture]]). | Place the concern in its peer-native DHT/REA home first. Compute placement is downstream of that, never a substitute for it. |
| An amber/green (DHT-agreement) trust signal *written* as a per-host "mode" instead of *derived*; per-host authoring of a notarized field | amber/green means "the DHT has witnessed this head's claims in network context" (green) vs "not yet" (amber) — it is a DERIVED read signal, never a write. Writing it (e.g. a per-storage diesel-direct `?deployTier=amber` PATCH stamping `crdt_converged_at`, never `dht_anchor_hash`) mints a divergent, un-witnessed head on EACH backend that can never green and never converges — the `public@adam` vs `private@matthew` / elohim.host "stuck" class. The doorway/storage is a gateway, never the witness. | Author a notarized field ONCE through the conductor (the DHT witnesses it; gossip / `run_content_sweep` converges it to every peer — no conductor bridge → fail loud and fail over, never a local un-witnessed write). Derive amber/green from `dht_anchor_hash` presence — never a stored write-choice and never a value-equality check. (The EPR content CID does NOT carry peer perspective: the canonical hash EXCLUDES `cid`, `proof`, and `supersededBy` — `elohim/epr/src/envelope.rs` — so signature material rides the detached `proof` and the `dht_anchor_hash` ActionHash, and two peers signing identical content get the SAME CID; the claims-coupling, by contrast, is baked INTO that shared CID.) The correct model is NOT per-host at all: ONE blob is seeded once and synced across peers by the p2p dataplane, servable as soon as present (amber), DHT-notarized → green. Any per-host byte-seeding is an interim SCAFFOLD to be deleted, never extended and never the design ([[feedback-cleanup-toward-p2p-dataplane-trajectory]], [[project_inventory_exchange_not_byte_replication]]). Multiple simultaneous versions are legitimate ONLY as an intentional declared-HEAD collective-edit DAG. |
| Missing source-of-truth declaration | A table exists but nobody documented whether Holochain or SQLite is authoritative. Bugs appear when they disagree. | Every table's migration or schema file includes a comment: `-- Source of truth: DHT` or `-- Source of truth: local (operational)`. |
| Putting granular data on the DHT | Every quiz answer, scroll event, or preference becomes a permanent head-plane resident — a sweep row, a Kad record, an election candidate, a conductor round-trip, forever, on every peer. | Attested-Private (B2): raw data stays private, a signed proof of the outcome is notarized. Price it with Step 1.5 before you commit. |
| Cross-namespace identity string-equality | The same agent has several identities (Holochain `uhCAk…` key, libp2p `12D3Koo…` peer id, iroh NodeId, doorway JWT subject). Joining/matching one against another by raw string silently empties the join (caused the all-zeros resilience card, repeatedly). | Resolve through the canonical agent↔transport resolver (the `AgentPeerBinding` projection / `peer_transport_manifest`; `hosted_agent_bindings` for the federation namespace). Never string-compare identities across namespaces; pick `agent_cid` as the canonical join key and resolve the others TO it. |
| `self-sovereign` / "true data sovereignty" as the **apex** identity, agency, or capability tier | Imports the silicon-crypto sovereignty ontology the protocol explicitly *subordinates* to community governance. The apex-tier label reads as a neutral capability level ("more keys → more autonomy → higher") and sails past review as a load-bearing ontological claim. Also silently excludes everyone who holds the right *through others* (children, IDD, seniors, wards). | Frame the high-autonomy tier as **community-grounded** (e.g. *node-stewardship standing*), not *self-sovereign*. Key-location is a mechanical fact (`custodial` → on-device → always-on), not an ascent toward sovereignty. Sovereignty is the *adversary frame*, never the protocol's own apex ([[feedback-identity-sovereignty-ontology-guard]]). Confirm the corrected lexicon with the architect. See "Identity Ontology Guard." |

**The conductor call you cannot cancel.** `HcClient::call_zome` has no timeout and no cancellation path — a caller-side timeout abandons the call while the conductor keeps executing it with nobody listening, still holding the read permit whose saturation the timeout was reacting to (the failure compounds exactly when it hurts most). Design every conductor-touching surface so the WORK is bounded before the call is made: batch externs carry their own in-wasm deadline and return partial results; callers size batches and interpret `unattempted`, never bolt a timeout onto the call. Edit-time rail: `conductor-call-is-uncancellable` in `elohim/elohim-storage/src/.epr-meta`.

---

## Identity & Transport-Identity Coherence

`agent_cid` (`uhCAk…`) is the canonical agent identity throughout the protocol. Three other namespaces name the same agent and are NOT interchangeable with it:

- **libp2p peer id** (`12D3Koo…`) and **iroh `NodeId`** — transport-plane identities.
- **doorway / JWT subject** — the federation-plane identity, resolved via `hosted_agent_bindings` and `PeerJwksCache`. Same raw-string-compare failure class, one layer up: the doorway rides OVER the p2p substrate rather than being part of it ([[feedback_p2p_vs_federation_layer_vocabulary]]).

The transport resolution substrate is the notarized `AgentPeerBinding` DHT integrity entry (projected by `ReconcileController::on_agent_peer_binding` into the `peer_identity_bindings` table), materialized locally in `peer_transport_manifest` (`elohim/elohim-storage/src/p2p_iroh/peer_map.rs`).

**Rule**: any new entity that references a peer, provider, steward, or hosted-agent identity must declare which namespace it stores, and must resolve through the canonical resolver when joining or matching across namespaces. Never raw-compare `agent_cid` against a transport or JWT id.

**Honesty clause — the general resolver is not built.** A general transport-id → `agent_cid` resolver is specced but blocked, and today's bindings are self-asserted and unsigned (`elohim/elohim-storage/CLAUDE.md`). **Do not consume bindings for economic attribution** — reputation aggregation, payout, standing — until a cross-signed control proof lands. If your entity needs cross-namespace resolution for an attribution purpose, that is a blocker to declare in your gate output, not a detail to design around.

---

## Identity Ontology Guard (imago-dei floor, not crypto self-sovereignty)

This gate guards the **framing** of identity, not only its addressing. A human's identity in this protocol is *imago dei* — an inviolable right **backstopped by community and institutional expression**, not a self-asserted cryptographic primitive. Individual sovereignty is **subordinated** to community-adjudicated governance; it is never the apex value ([[feedback-identity-sovereignty-ontology-guard]]). **Canonical home:** `genesis/docs/architecture/stewardship-over-sovereignty.md` (Canon status: Foundational, "read it first" — *"We do not consider sovereignty itself to be the right framing"*; §3 reserves *stewardship / agency / authority* with discipline and explicitly excludes "sovereignty") and its life-stage companion `genesis/docs/architecture/cradle-to-grave-capability-gradient.md`. The imagodei domain gospel echoes it: identity grounded in "demonstrated capability and community trust."

**The recurring drift this kills:** naming the top of an identity / agency / capability gradient `self-sovereign`, or celebrating "true data sovereignty" as an achievement. AI agents default to the silicon-crypto sovereignty ontology because it dominates the training data, and it slips past review **at tier-naming** — an apex tier called "self-sovereign" reads as a neutral capability level rather than the load-bearing ontological claim it actually is. This is the identity-vocabulary sibling of the relational-default mistakes above.

**Rules when designing ANY identity, agency, role, capability, or key-location entity** (enum, struct, schema, tier ladder — or the prose that documents one):

- **Never make `sovereignty` / `self-sovereign` the positive apex** of a gradient. Frame the high-autonomy tier as community-grounded — e.g. *node-stewardship standing*, not *self-sovereign*. Key-location is a mechanical fact (`custodial` → on-device → always-on); do not dress it as an ascent toward sovereignty.
- **Sovereignty as an adversary frame is correct.** Quoting "the crypto sovereignty frame" as the thing being *resisted*, or modeling state/platform sovereignty as a threat, is fine. Asserting it as the protocol's *own* top value is the drift.
- **Model the whole human life.** The ontology must hold for those who exercise the right *with/through others* — children, IDD, seniors, legal wards. The protocol expresses this as **graduated, mediated agency** (ward = "mediated agency, guardian co-authors"; voice-retention for seniors; supported decision-making) — the canonical §2 life-stage transition map is `genesis/docs/architecture/cradle-to-grave-capability-gradient.md`. An identity model whose apex is "full autonomy, keys on device" silently excludes them. Note: this is canon-written with concrete ward/guardian specs pending — there is no guardian/ward DHT *entity*, and `custodial` key-holding is device-convenience, NOT incapacity-guardianship; do not conflate the two.
- **Cryptography is an ACCELERATOR of community recovery, never the gate** — Shamir, threshold signatures, hardware-rooted attestation all *speed* the recovery paths, but their absence must never prevent recovery (`stewardship-over-sovereignty.md` §4; `project_socially_derived_security`). So "harden a high-risk user" (e.g. a dissident under state duress) means *optional* cryptographic hardening layered onto the social-recovery floor — NOT elevating them to a "more sovereign" tier. The protocol's standing defense against duress is the non-firable elohim-counsel (`project_elohim_as_counsel`), not stronger keys.

If a design names a tier `self-sovereign`, proposes `sovereignty`-as-achievement, or treats individual autonomy as the protocol's ceiling: **stop, reclassify the framing, and confirm the corrected apex lexicon with the architect** before writing the enum/schema. This gate stops design-time bleeding at the source; pre-existing leaks in shipped surfaces are a rename concern, not a reason to add another.

---

## Output Format

When the gate is complete, present the result in this format before proceeding to design proposals.

```
## P2P Design Gate: {Feature Name}

### Entity: {EntityName}
- **Classification**: Notarized (A) | Linked (A2) | Private (B) | Attested-Private (B2) | Ephemeral (C)
- **Justification**: {1-2 sentences on why this classification}
- **Head-Plane Cost Budget** (A/A2 only): {count at seed / at 1yr; the recurring cost formula joined; above ~500 items — bundling shape (composite root | A2-via-link | corpus digest) or operator sign-off}
- **Network Stakes**: {stages this entity must behave under; which costs are stage-priceable vs floor-protected}
- **Content Address Strategy**: Content-Derived (CID) | Agent-Scoped Composite | Slug/UUID
- **Address Justification**: {Why this strategy, not the others; for CID, who declares the applicable head}
- **Transport Affinity** (byte-bearing only): libp2p | iroh | auto — {why}
- **Source of Truth**: Holochain DHT | Private Source Chain | SQLite (operational)
- **Integrity Zome + DNA-hash class**: {real zome name from dna.yaml — NOT {dna}_integrity} — DNA-HASH-MOVING | DNA-hash-NEUTRAL
- **Coordinator Zome**: {zome}::{fn} -> EntryHash (Notarized) | ActionHash (Linked — create_link)
- **Projections**: SQLite {table_name} (dht_anchor_hash: yes/no); Automerge sync (yes / no / n-a for Linked) at reach tier: {name | `unresolved — reach vocabulary in declared drift`}
- **HTTP Route**: {method} {path}, declared in elohim-storage `build_manifest()` — {which hash `{id}` carries}
- **Anti-Pattern Check**: {Confirmed none apply, or list which were caught and corrected}

### Entity: {NextEntityName}
... (repeat for each entity)

### Design Constraints Discovered
- {Any cross-entity relationships, ordering dependencies, or migration concerns found during the gate}
```

**Back-fill detector.** This template is answerable bottom-up — an author who arrived with `GET /api/v1/thing` already in mind can fill it in reverse and nothing catches them. Three questions that cannot be answered in reverse. If any is blank or was answered *from* the route, the gate was not run:

1. What does your coordinator function **return**, and is that the same hash your route's `{id}` accepts? (If you had to read your own route to answer, you back-filled.)
2. Which **integrity zome** does the entry type live in, and does your change move the DNA hash?
3. What is the item count **at 1 year**, and what does that do to the quiesce number?

Only after this output is complete and reviewed should design proposals (schemas, migrations, component architecture) proceed.

---

## Key Files

| File | Purpose |
|------|---------|
| `elohim/holochain/dna/*/zomes/*_integrity/src/lib.rs` | **Where entry types actually live** — grep `#[hdk_entry_types]` here to answer Step 1's "does a type already exist?". Read `dna.yaml` for the real zome names |
| `elohim/elohim-views/src/` | Rust view types with `#[derive(TS)]` — the real Rust-to-TypeScript boundary (`elohim-storage/src/views.rs` re-exports these and carries no ts-rs derives of its own) |
| `elohim/elohim-storage/migrations/` | SQLite migrations — every table must declare source of truth |
| `elohim/elohim-storage/src/sync/projector.rs` | The Automerge sync projection — Step 3b's second projection |
| `elohim/elohim-storage/src/http.rs` (`build_manifest()`) | **Where you declare an HTTP route** — the registry compiles it and the doorway serves it. Step 3c's real target |
| `elohim/sdk/storage-client-ts/src/generated/` | Auto-generated TypeScript types from Rust views |
| `app/elohim-app/src/app/elohim/adapters/` | Adapters that add computed fields — never transform wire format |
| `doorway/CLAUDE.md` | When a route *does* need doorway-specific logic — the only reason to touch `doorway-service` |
| `.claude/epr-meta/policies.yaml` + `.claude/epr-meta/concerns.yaml` | The concern canon (C0-C14) — enforcement rows (predicate-bearing classes) vs Precedent-shaped rows; Step 4's canon homes |
| `elohim/sdk/schemas/v1/manifest/seam-registry.schema.json` | Per-crate decision-point registry schema — where Step 4's registration lands |
| `.claude/scripts/memory-kit/placement-audit.py` (`--epr-meta`) | The census — the enforcement backstop for Step 4, reads every crate's `seam-registry.yaml` |
| `genesis/docs/content/elohim-protocol/protocol-specification.md` | Full EPR protocol specification |
