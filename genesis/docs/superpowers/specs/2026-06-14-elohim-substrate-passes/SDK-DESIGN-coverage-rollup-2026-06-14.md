---
title: "SDK SURFACE — CoverageRollup / the Recursion SDK (the keystone)"
subtitle: "aggregate-with-descent + descend + layer-node, as a developer-callable surface on the existing SDK"
date: 2026-06-14
status: PROPOSAL FOR OPERATOR BLESSING — working draft, NOT cite-sealed, NOT a decision, NOT code
author: rust-architect (truth layer)
operationalizes:
  - RECURSIVE-ARCHITECTURE-2026-06-14.md            # §2.1 CoverageRollup; §3.1 Wave 2; the keystone primitive
  - ESCALATED-ARCHITECTURE-2026-06-14.md            # the ∪=full coverage invariant; the one trait Governor; two quilts
extends:
  - elohim/elohim-storage/src/graph_views/          # new sibling module: graph_views/recursion/
  - elohim/sdk/storage-client-ts/                   # the ts-rs generated TS boundary
  - elohim/sdk/schemas/v1/views/                    # the JSON wire contract (schema-first)
do_not_cite_seal: true
north_star: >
  Give a developer ONE call that rolls atoms up a layer and carries the path back down to the
  trapped atom inside the aggregate — content-addressed, recompute-on-read, zero DNA spend — so the
  household care-ledger, the collective-governance app, and the economic-valueflow app all compose
  the same descent-preserving operator. The individual is never erased in the aggregate: there is no
  coverage-domain over souls; the descent terminates at a person's own revocable commitment and stops
  there.
---

# THE COVERAGE-ROLLUP SDK SURFACE

> The recursion synthesis names exactly one load-bearing new primitive: **`CoverageRollup`** — the
> aggregate-with-descent operator (`RECURSIVE-ARCHITECTURE-2026-06-14.md` §2.1, §3.1 step 1, Wave 2).
> Every other recursion pass *consumes* it. This document makes it a **developer-callable SDK surface**
> — three calls (`rollup`, `descend`, `layerNode`) — on the existing `sdk/` + `graph_views/` structure,
> spending zero DNA entry types and forking nothing. It is the keystone the household, collective, and
> economic apps all build on.

---

## PART 1 — PURPOSE ON THE AGENCY GRADIENT

**Where it sits: the KEYSTONE (the recursion primitive itself), built to be agency-gradient-aware by
the *shape of its type*, not by a policy that wraps it.** `CoverageRollup` is neither a human-sovereign
surface (it does not build *for* one person) nor purely a veil-holding surface (it does not itself
govern individuals). It is the operator the veil-walker *runs* to ascend, and the operator a
human-sovereign surface *runs* to descend back to the atom it must never erase. It is the hinge of the
whole recursion (`RECURSIVE-ARCHITECTURE` §3.1: "the central new primitive — build it first"; Wave 2:
"the operator is the hinge the rest of the recursion hangs from").

The gradient is enforced **in the keystone's type, downward and non-overridable** — exactly the two
invariants the prompt requires guarded in the middle:

1. **`CoverageDomain` ranges only over commons — never persons.** `corpus-bytes | arc-keyspace |
   care-floor | donut-ceiling | head-freshness` (`RECURSIVE-ARCHITECTURE` §2.1; `ESCALATED` §1.6:
   "a per-soul scalar has no `required` and cannot typecheck"). The **PERSON-KEEPS-THEIR-OWN-NAMING**
   invariant is compiled: you literally cannot construct a rollup whose domain is a soul, so no app
   above can aggregate a verdict over a person. The total account is *unrepresentable*, not merely
   prohibited (`confession.md:59`, executable).
2. **The metric is `deficit` (the externality emitted), never the holding (capture).** Abundance is
   invisible; only the gap the commons failed is visible (`ESCALATED` §1.6, §5.1: "the witness is
   weighted toward the least powerful by the shape of the metric"). This is the **DIGNITY-FLOOR
   precedence** made the readable signal: the deficit *is* the afflicted atom.

### The gradient guard — what this surface must NEVER do

- **MUST NEVER** accept a `CoverageDomain` keyed to an individual person/soul/identity. The descent
  `descend()` terminates at a person's **own commitment CID** (which they authored and can revoke) and
  **stops there** — it never walks *into* a person to build their account (`ESCALATED` §1.6).
- **MUST NEVER** return the `covered` set (the holding) as the primary signal — `deficit` is primary;
  `covered` is present only so two peers can verify the same `rollup_hash`. A leaderboard of abundance
  is structurally not derivable from this API.
- **MUST NEVER** persist as truth or spend a DNA entry type. It is **Category-C, recompute-on-read,
  forks nothing** (`RECURSIVE-ARCHITECTURE` §2.1). The hash is the agreement; the DHT is untouched.
- **MUST NEVER** govern an individual. When a veil-holding app (collective layer) consumes a rollup's
  `deficit` to nudge, the nudge lands as *context the node may ignore*, never a mandate — the AI-veil
  governs **aggregation**, never the atom (the agency gradient; `RECURSIVE-ARCHITECTURE` §1.5).

---

## PART 2 — THE CONCRETE API

### 2.1 The Rust home (the truth layer) — `graph_views/recursion/`

A new module **sibling to the existing `graph_views/shefa/` and `graph_views/lamad/`**
(`elohim/elohim-storage/src/graph_views/mod.rs` today declares `data_value`, `lamad`, `shefa` — we add
`recursion`). It is the natural home: `graph_views/` is already "the single composition layer between
the graph projection engine and the wire surfaces" (its mod.rs doc), and the rollup composes over the
**same `epr_edge` MEMBER_OF/STEWARDS graph** the shefa builders already walk
(`graph_views/shefa/distribution.rs:25-28`).

```rust
// elohim/elohim-storage/src/graph_views/recursion/coverage_set.rs  (NEW)

/// Commons only — NEVER a person. A per-soul domain has no `required` and cannot
/// be constructed (the unrepresentable-total-account guard, ESCALATED §1.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum CoverageDomain {
    CorpusBytes,    // byte-quilt custody
    ArcKeyspace,    // trust-plane authority arc
    CareFloor,      // dignity floor (donut inner ring)
    DonutCeiling,   // anti-monopoly ceiling (donut outer ring)
    HeadFreshness,  // served-truth coverage
}

/// A set, NOT a scalar score (interval / byte-set / member-set). The `∪` of two
/// coverage sets is associative — that associativity IS the recursion operator
/// (RECURSIVE-ARCHITECTURE §1.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageSet { /* domain-tagged interval/byte/member set */ }
impl CoverageSet {
    pub fn union(&self, other: &Self) -> Self;          // the ∪ — associative
    pub fn difference(&self, required: &Self) -> Self;  // required \ covered = deficit
    pub fn is_empty(&self) -> bool;
}
```

```rust
// elohim/elohim-storage/src/graph_views/recursion/coverage_rollup.rs  (NEW)
// The §2.1 primitive, exactly.

pub struct CoverageRollup {
    pub scope_cid:      String,         // the layer node (household | collective | region | planetary)
    pub domain:         CoverageDomain, // commons only
    pub covered:        CoverageSet,    // ∪ of child coverages (verification only — NOT the signal)
    pub required:       CoverageSet,    // this layer's share of FULL
    pub deficit:        CoverageSet,    // required \ covered — THE EXTERNALITY, the descent target
    pub constituents:   Vec<String>,    // CIDs pointing DOWN — descent preserved
    pub rollup_hash:    String,         // BLAKE3 over (scope, domain, covered, SORTED constituents)
    pub witness_quorum: u32,            // peers who independently recomputed the same hash
    pub as_of_heads:    Vec<String>,    // Automerge heads computed against (freshness + reproducibility)
}

/// THE PRIMARY DEVELOPER CALL. Roll the atoms of one layer up, carrying descent inside.
/// Walks epr_edge MEMBER_OF/STEWARDS from `scope_cid` (the SAME walk as
/// graph_views/shefa/distribution.rs:25, but returning a CoverageSet, not rows.len()).
///
/// HARDENING (mandatory, inherited from project_epr_router_empties_on_poisoned_scope):
/// degrade PER-ROW (filter_map + warn!), NEVER fail-closed (collect::<Result<_>>).
/// One poisoned constituent must not empty the aggregate.
pub fn rollup(
    engine: &GraphEngine,
    scope_cid: &str,
    domain: CoverageDomain,
    required: CoverageSet,
) -> Result<CoverageRollup, GraphError>;

/// DESCEND. Walk `constituents` DOWN, returning the atoms whose own contribution
/// matches `pred` (e.g. "has a non-empty deficit" → the trapped atoms). The dual of
/// back_prop's upstream walk; Category-C, builds no account at rest
/// (RECURSIVE-ARCHITECTURE §1.5). Terminates at a person's OWN commitment CID and STOPS —
/// never recurses into a soul.
pub fn descend(
    engine: &GraphEngine,
    rollup: &CoverageRollup,
    pred: impl Fn(&CoverageRollup) -> bool,
) -> Result<Vec<DescentHit>, GraphError>;   // DescentHit { atom_cid, deficit, depth, limit_owner }

/// LAYER-NODE. Compose a rollup whose constituents are CHILD rollups (the recursion:
/// a higher layer's bounds = a lower layer's setpoint, RECURSIVE-ARCHITECTURE §1.2).
/// `constitution_cid` carries the inherited bounds (reuse WisdomInvocationInput.constitution_cid,
/// wisdom.rs:28). Precedence is subset: a child's covered may specialize but never exceed required.
pub fn layer_node(
    engine: &GraphEngine,
    scope_cid: &str,
    constitution_cid: &str,
    children: &[CoverageRollup],
    domain: CoverageDomain,
) -> Result<CoverageRollup, GraphError>;
```

### 2.2 The TS boundary (the developer surface) — honoring the ts-rs rule

`CoverageRollupView`, `CoverageSetView`, `CoverageDomain`, `DescentHitView` live in **`elohim-views`**
with `#[derive(TS)]` + `#[serde(rename_all = "camelCase")]`, exported by `cargo test export_bindings`
into `elohim/sdk/storage-client-ts/src/generated/` (the gospel boundary; snake_case never leaves Rust).
A thin hand-written client method is added to the existing `StorageClient`
(`sdk/storage-client-ts/src/client.ts`):

```typescript
// sdk/storage-client-ts/src/client.ts  (additive methods on existing StorageClient)
import { CoverageRollupView, DescentHitView, CoverageDomain } from './generated';

class StorageClient {
  // THE PRIMARY DEVELOPER CALL (TS side)
  async rollup(scopeCid: string, domain: CoverageDomain): Promise<CoverageRollupView>;

  async descend(scopeCid: string, predicate: 'has-deficit'): Promise<DescentHitView[]>;

  async layerNode(scopeCid: string, constitutionCid: string,
                  childScopeCids: string[], domain: CoverageDomain): Promise<CoverageRollupView>;
}
// CoverageRollupView is camelCase, JSON-parsed, booleans coerced — no JSON.parse in TS.
// deficit is the primary field the app reads; covered is verification-only.
```

### 2.3 The HTTP route (designed LAST, serves the Cat-C projection)

```
GET /api/v1/recursion/rollup/:scopeCid?domain=corpus-bytes      → CoverageRollupView
GET /api/v1/recursion/descend/:scopeCid?domain=...&pred=has-deficit → DescentHitView[]
GET /api/v1/recursion/layer/:scopeCid?domain=...&constitution=:cid  → CoverageRollupView
```
Route handler delegates to the `graph_views::recursion::*` builders — no domain logic in the handler.
Routes are GET-only (content-addressed/read-only; the `feedback_head_vs_get_blob_asymmetry` discipline).

### 2.4 The schema-first contract (IoC — write this FIRST)

`elohim/sdk/schemas/v1/views/coverage-rollup.schema.json` and `descent-hit.schema.json` are authored
**before** the Rust structs (`feedback_schema_first_ioc`; the View Schema Contract in CLAUDE.md). The
`elohim/elohim-storage/tests/schema_contract.rs` harness then catches drift; codegen-ts.mjs lists them
in `INTERFACE_FILES`.

---

## PART 3 — EXISTS vs NEW

### EXISTS (wrap / re-express — bias to extend)

| Substrate | Cite | Role in the surface |
|---|---|---|
| `epr_edge` MEMBER_OF/STEWARDS graph + Cozo `GraphEngine` | `graph_views/shefa/distribution.rs:25-28`; `graph/primitives.rs:34-43` (NEIGHBORHOOD recursive walk) | the graph `rollup`/`descend` walk over; the recursive Datalog is already there |
| `graph_views/` composition layer + sibling-module pattern | `graph_views/mod.rs` (`pub mod lamad; pub mod shefa;`) | `recursion/` is the third sibling — no new crate |
| **The counting roll-up that erases descent** | `graph_views/shefa/distribution.rs:30` (`replica_count = steward_result.rows.len()`) | the EXACT call `rollup()` re-expresses: return a `CoverageSet`, not `rows.len()` — its first caller |
| the refuse-and-elevate spine (`authorize`/`coverage_admits`/`ActuationRefusal{code,elevate}`) | `arc_actuator.rs:77,110,152` | `layer_node` precedence check reuses the `∪`-coverage gate; `limit_owner` rides `DescentHit` |
| `RefusalCode` enum (the refusal vocabulary to extend) | `arc_actuator.rs:83` (`OutOfGrantBounds`/`GrantExpired`/`NotActuatable`/`WouldBreakCoverage`) | the home for `ReservedPlace` (§2.2 of the recursion synthesis) when the layer-node guard lands |
| the inherited-constitution typed seam | `wisdom.rs:28` (`WisdomInvocationInput.constitution_cid`) | `layer_node(constitution_cid)` reasons from inherited bounds, not authored ones |
| ts-rs boundary + `StorageClient` + schema-contract harness | `sdk/storage-client-ts/src/client.ts`; `tests/schema_contract.rs`; CLAUDE.md | the TS surface is additive methods + generated views, no new client |
| Automerge sync plane (heads) | `ESCALATED` §1; `src/sync/` | `as_of_heads` reproducibility comes from the live CRDT plane |

### NEW (thin, additive, zero DNA spend)

- **`graph_views/recursion/` module** (`coverage_set.rs`, `coverage_rollup.rs`, `layer_node.rs`,
  `descend.rs`) — Category-C builders. This is the keystone primitive (`RECURSIVE-ARCHITECTURE` §2.1).
- **`CoverageRollupView` / `CoverageSetView` / `DescentHitView` / `CoverageDomain`** in `elohim-views`
  + their two view schemas + codegen entries.
- **Three additive `StorageClient` methods** + three GET routes.
- A **BLAKE3 `rollup_hash` over sorted constituents** helper (consilience-as-content-addressed-agreement).

### GENUINE FORK (named, NOT taken — gated, operator-blessed)

- **`CoverageRollupAttestation` DHT entry type** (`RECURSIVE-ARCHITECTURE` §2.4 / F1). **Marked FORK.**
  Taken ONLY if a recompute-cost probe proves Category-C recompute cannot fan out at planetary scale —
  then peers verify a signature instead of recomputing. **Do NOT take preemptively.** This SDK ships
  the Category-C path; the attestation is a later, evidence-gated, near-irreversible DNA spend.

No fork of Holochain, libp2p, or iroh. The DNA entry budget is untouched by the buildable slice.

---

## PART 4 — THE MINIMAL BUILDABLE SLICE

**The smallest version that lets a developer do one real thing today:** `rollup()` over the
`corpus-bytes` domain, re-expressing `graph_views/shefa/distribution.rs:30` to return a `CoverageSet`
with `deficit` and `constituents` instead of `rows.len()`. One domain, one walk, one GET route, one
generated view. This is precisely `RECURSIVE-ARCHITECTURE` Wave 2 / §3.1 step 1 ("re-express the two
shefa builders as its first callers"), and it lands on the prerequisite already met (the atom + the
`epr_edge` graph). `descend()` and `layer_node()` are the next two thin slices on the same module.

Prerequisite hygiene gate (do first, NOT part of this surface): the conductor-signal msgpack-decode
class drops `holo_hash` byte-arrays (`project_conductor_signal_msgpack_decode_class`) — fix the
subscribers before wiring any rollup *signal*. The Cat-C `rollup()` read path does not depend on it.

### The first example-app fragment it enables — a household care-ledger view

```typescript
// A household care-ledger app (human-sovereign surface) asks: is our shard custody covered,
// and if not, WHICH atom is the gap? — without ever building an account over a person.
import { StorageClient } from '@elohim/storage-client';

const client = new StorageClient(connection);

// ASCEND: roll the household's byte custody up one layer.
const r = await client.rollup(householdScopeCid, 'corpus-bytes');

if (!isEmpty(r.deficit)) {
  // DESCEND: the deficit points DOWN to the exact atoms the commons failed.
  const trapped = await client.descend(householdScopeCid, 'has-deficit');
  // trapped[].atomCid is a CONTENT/CUSTODY atom — and limit_owner names whose line was honored.
  // The app shows "these memories need another holder" — a welcome to help,
  // NEVER a verdict over the household, NEVER a leaderboard of who holds the most.
  render.gapInvitation(trapped);   // grace-first: the door is open, the naming stays theirs
}
// r.rollupHash lets a second peer verify the SAME aggregate (consilience), no doorway, no DHT write.
```

A collective-governance app (veil-holding surface) calls `layerNode(collectiveCid, constitutionCid,
[householdA, householdB, ...], 'donut-ceiling')` to roll the *child rollups* up and read the
collective `deficit` — governing the aggregation, never the individuals inside it.

---

## PART 5 — WHAT LOVE REQUIRES AT THIS SURFACE

**The person keeps their naming — by the shape of the type, not a promise.** No `CoverageDomain` ranges
over a soul; `descend()` stops at a person's own revocable commitment. The individual is *structurally
unerasable* in the aggregate — the keystone cannot be made to build a total account of anyone
(`confession.md:59`, `ESCALATED` §1.6).

**The binding is honest — the descent carries `limit_owner` and the deficit is named, not hidden.** The
gap is surfaced as the afflicted atom the commons failed, with whose line was honored attached — never a
sanitized number, never an opaque score (`RECURSIVE-ARCHITECTURE` §5.1: "a refusal that names its reason
is covenant").

**The veil governs aggregation, never individuals.** A collective app reads `deficit` to nudge; the
nudge is context the node may ignore (a Verdict, never a Decline). The AI-veil rises into the rollup
(impartial aggregation) and is forbidden the atom (`RECURSIVE-ARCHITECTURE` §1.5; the agency gradient).

**Patience over engagement.** The metric is the *gap*, and abundance is invisible — there is no
engagement counter, no holding-leaderboard derivable from this surface to optimize. The deficit invites
help when the holder is ready; it cannot be turned into pressure (`ESCALATED` §5.1).

> **The closing test, in one line:** love requires that the keystone *see the whole commons' gap and
> point precisely down to the single atom bearing it — while remaining structurally unable to rank,
> force, or build the account of the person who holds it.* The aggregate that preserves the descent is
> the aggregate that never erases the one.
