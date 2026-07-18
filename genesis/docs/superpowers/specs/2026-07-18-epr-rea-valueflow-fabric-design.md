---
title: "EPR-REA ValueFlow Fabric — the REA/ValueFlows domain layer over EPR atoms + the walkable process plane"
id: epr-rea-valueflow-fabric
tier: spec
status: Draft
created: 2026-07-18
maintainers: Matthew Dowell + Claude Fable 5
class: protocol-canonical
domain: D9
topic: [rea, valueflows, epr, eprfs, epr-meta, process, recipe, intent, commitment, fulfillment, value-chain, walk, dev-pipeline, dogfood, brit, rakia, mccarthy]
context-tier: disclosed
steward: cartographer
graduation-trigger: decompose-complete OR epr-rea-crate-landed-with-dev-pipeline-recipe-walked
refines:
  - genesis/docs/superpowers/specs/2026-07-10-epr-meta-native-capability-dogfood-and-graph-design.md
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-20-wave3-valueflows-hrea-interop-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md
cites:
  - epr-meta-native-capability-dogfood-and-graph | the spec this REFINES — reserved the EprRef value-plane seam (offline usage floor, REA reconciliation ceiling) this fabric attaches; supplies the floor/ceiling shape and the espresso event anatomy | sha256:99f0bf58985ff85b | path: genesis/docs/superpowers/specs/2026-07-10-epr-meta-native-capability-dogfood-and-graph-design.md
  - records-lifecycle-design | canonical D3 seed — the EPR/Event/Resource state machine and the observation→crystallization gradient (1000:1) the FlowEvent→EconomicEvent split rides | sha256:2b5f54d20108bcf0 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md
  - wave3-valueflows-hrea-interop-design | the D9 VF/hREA interop boundary — supplies the VF-name-alignment discipline (1:1 mechanical bridge translation) and the substrate≠app-layer warning | sha256:c8d903ad73f0284d | path: genesis/docs/content/elohim-protocol/architecture/2026-05-20-wave3-valueflows-hrea-interop-design.md
  - rea-economic-facing-lens-design | the proven fold shapes (commitment_backed, realized_value_flow fulfillment ratio) epr-rea lifts from diesel-specific code into substrate folds | sha256:b83ead21be13bbaa | path: genesis/docs/superpowers/specs/2026-06-19-rea-economic-facing-lens-design.md
  - epr-acquisition-slice2a-rea-rails-plan | the runtime graduation rails that already exist — call_create_rea_economic_event, commitment graduation, bounded_by — the DHT targets the sidecar floor reconciles into | sha256:62a490200c40f5d4 | path: genesis/docs/superpowers/plans/2026-06-08-epr-acquisition-slice2a-rea-rails-plan.md
  - epr-meta-kinship-lineage-reconciliation | lineage-inside-the-hashed-bytes principle reused for fulfills/satisfies edges; the remote verdict keeping value-chain walks honest across substrate boundaries | sha256:adb7385729b94c24 | path: genesis/docs/superpowers/specs/2026-07-12-epr-meta-kinship-lineage-reconciliation-design.md
  - epr-meta-policy-registry-measure | the define-once-bind-many + content/standing two-plane pattern instrumentation policies reuse verbatim — granularity as governed, versioned policy | sha256:474eee1686e3123b | path: genesis/docs/superpowers/specs/2026-07-02-epr-meta-policy-registry-measure-design.md
---

# EPR-REA ValueFlow Fabric

> **One-line:** model the REA/ValueFlows ontology (McCarthy 2019 monograph; VF vocabulary) as a
> first-class domain layer over EPR atoms — recipes, intents, commitments, events, fulfillment —
> so that ValueFlows links live in the filesystem (`.epr-meta` bindings + `.eprfs/status` sidecar),
> agents can stitch them into processes that can be *followed, designed, and adapted*, and a change
> anywhere can be walked down the value chain to the finish. First instance: the dev pipeline itself
> (manifesto → epic → spec → plan → feature → scenario → glue → code → gates → playwright-validated).

## 1. Provenance and the seam this fills

This design **attaches the value plane** that the 2026-07-10 dogfood spec deliberately reserved:
*"the `EprRef` is also the value-flow anchor … the floor is eprfs offline usage collection
accumulated locally against the `EprRef`; the ceiling is honest REA reconciliation on reconnect.
This sprint neither builds nor blocks it — it keeps the seam clean so the value plane can attach
later."* It composes (never forks) from:

- **records-lifecycle** (D3, canonical): the EPR/Event/Resource state machine and the
  observation→crystallization gradient (typically 1000:1) this fabric rides.
- **wave3-valueflows-hrea-interop** (D9): the VF vocabulary alignment discipline — VF names are
  honored 1:1 so bridge translation stays mechanical; the substrate≠app-layer warning holds.
- **rea-economic-facing lens**: the proven fold shapes (`commitment_backed`, `realized_value_flow`
  fulfillment ratio) this crate lifts from diesel-specific code into substrate folds.
- **slice-2a REA rails (plan)**: the runtime graduation targets that already exist —
  `call_create_rea_economic_event`, commitment graduation `proposed → active`, `bounded_by`.
- **epr-meta policy registry**: the define-once-bind-many + two-plane (content vs standing)
  pattern that instrumentation policies reuse verbatim.
- **epr-meta kinship/lineage**: lineage-inside-the-hashed-bytes; the `remote` verdict for
  well-identified holes when a chain crosses a substrate boundary.

Grounded crate state (verified 2026-07-18): `elohim-epr` has `EprKind::{EconomicEvent, Commitment}`
as *tags only* (coupling rules, no payload structs) plus the witness layer
(`WitnessedInteraction`, `ReaVerb`, `Magnitude`); `eprfs-core` owns the `.epr-meta` in-memory
governance model; `eprfs-meta` parses/evaluates the cascade; `brit-graph` has the generic
DAG/affected-tracking engine (`GraphConnections`). **No Resource / Process / Intent / Fulfillment
/ Satisfaction types exist anywhere in the family; no trait models REA; the epr and eprfs crate
families are deliberately disjoint.** The valueflows bridge is an isolated workspace with
fixture-level VF GraphQL only.

## 2. Ontological commitments (the decisions)

**2.1 Workflow is a projection; the value chain is the model.** Per McCarthy, task/workflow
sequencing is operational detail *under* the economic layer. The fabric models **conversion
processes linked by duality** (use/consume → produce), stitched by resource flows. A "workflow
view" is derived by walking the chain; it is never stored as a first-class task list.

**2.2 Resource is not an entity.** Anything content-addressed *is* a resource — an EPR atom, a
blob, a spec file whose cite fingerprint is its body CID. Resource *state* (quantity on hand,
custody, fulfillment) is a **pure fold over its event history**, never a mutable struct. This is
P1 (storage as reconciliation controller) applied to economics.

**2.3 Fulfillment and Satisfaction are edges, not atoms.** `fulfills: [cid]`
(commitment → discharging event; the DHT already spells this `bounded_by`) and
`satisfies: [cid]` (intent → answering commitment/event) are fields inside the hashed bytes of
events/commitments — tamper-evident by construction, same principle as lineage-inside-the-bytes.

**2.4 Scale-free substrate, governed granularity ("molecules are allowed").** Nothing in the
type layer bounds what may be modeled — an agent CAN meter molecules. What *should* be metered is
a **governance decision expressed as policy**, not a design constant: an instrumentation policy
(Precedent-shaped, registry-defined, bound per scope via `.epr-meta`) declares which recipe edges
are *economically meaningful joints*. Over-instrumentation drowns signal in cost; the judgment of
"what matters here" is itself versioned, pinned, challengeable, and adaptable — per container,
not per protocol.

**2.5 The EPR is the container, and container-appropriateness is fractal.** Every flow event and
commitment carries `in_scope_of: Cid` — the container EPR accountable for it (VF's `in_scope_of`
made structural). Sub-flows aggregate to their container's ledger; the container is itself a
resource in *its parent's* scope. Whether a container is the right boundary at the right
instrumentation level is judged **one level up**, recursively — the `.epr-meta` directory cascade
is the filesystem projection of this fractal; this design generalizes it to the EPR graph.

**2.6 One action vocabulary.** REA actions are already drifting three ways
(`witness::ReaVerb` ≠ storage actions ≠ schema enum). The fabric mints **no fourth**: the action
enum is generated from the protocol schema (`elohim/sdk/schemas/v1` → `schema:codegen:rs`), and
`ReaVerb` + storage actions converge on it as a follow-on reconciliation (tracked; see §8).

**2.7 Three planes, one mechanism** (the ValueFlows levels, materialized):

| VF level | Fabric artifact | Filesystem floor | Graduated home |
|---|---|---|---|
| Knowledge (recipe) | `ProcessSpec` atom + instrumentation policy | `.epr-meta` registry + flow bindings | EPR atom (CID); standing via `Mishpat::Precedent` |
| Plan (intent/commitment) | `Intent`, `Commitment` atoms | sidecar atoms; gap-items are proto-intents | `Mishpat::Commitment` (cid = entry_hash), rea_commitments projection |
| Observation (events) | `FlowEvent` (granular) → `EconomicEvent` (crystallized) | `.eprfs/status/` append-only sidecar | `create_rea_economic_event` + `bounded_by`; gate-passes as Attestations |

## 3. Crate placement

**New crate `elohim/epr-rea`** (name `elohim-epr-rea`) — the REA domain layer. Depends on
`elohim-epr` (kinds, coupling, witness, cid). Consumed by: `eprfs-meta` (flow-binding parsing →
typed model), `epr-cli` (the walk commands), later `elohim-storage` (aligning the diesel rails)
and `brit` (dev-pipeline attestation nodes). `eprfs-core` stays decoupled (its Cargo-cycle
constraint is respected: `eprfs-meta → {eprfs-core, epr-rea}` is acyclic). `brit-graph`'s
`GraphConnections` is the traversal engine the walk adapts — compose, don't fork a second DAG.

Contents:

```rust
// Payload structs — canonical dag-cbor, CID'd, VF-named
pub struct ProcessSpec { stages: Vec<StageSpec>, edges: Vec<EdgeSpec> }
pub struct StageSpec  { name, artifact_kind /* schema_ref pattern */ }
pub struct EdgeSpec   { from, to, validators: Vec<ValidatorRef>, meaningful: bool }
pub struct Intent     { action, resource_spec, in_scope_of, raised_by }
pub struct Commitment { action, provider, receiver, resource_spec, bounds,
                        valid_from, valid_until, state, in_scope_of, satisfies: Vec<Cid> }
pub struct FlowEvent  { action, provider, receiver, resource: Cid, quantity: Magnitude,
                        process: Option<Cid>, in_scope_of: Cid,
                        fulfills: Vec<Cid>, satisfies: Vec<Cid>, occurred_at }
pub struct Process    { spec: PinnedRef /* recipe@version — declared pin, never recency */,
                        in_scope_of: Cid, inputs: Vec<Cid>, outputs: Vec<Cid> }

// Traits — only at genuinely polymorphic seams (multiple substrates implement)
pub trait ReaResource { fn cid(&self) -> Cid; fn classified_as(&self) -> &[String]; }
pub trait ReaAgent    { fn agent_cid(&self) -> Cid; }   // uhCAk… canonical; transports resolve TO it
pub trait FlowStore   { /* append + query events/commitments/intents by resource/agent/scope */ }
pub trait FlowWalk    { fn walk_back(&self, cid) -> Lineage;      // provenance to epic/manifesto
                        fn walk_forward(&self, cid) -> Frontier;  // dependents + newly-unfulfilled
}

// Folds — pure fns, lifted from the rea-facing lens shapes
pub mod fold { resource_state(events) -> ResourceState;
               fulfillment_ratio(commitment, events) -> Ratio;
               crystallize(observations, policy) -> Vec<FlowEvent> /* 1000:1 graduation */ }
```

`FlowStore` implementations: sidecar (`.eprfs/status/`, offline floor), diesel
(`economic_events`/`rea_commitments`, projection), DHT (conductor rails, truth). One model, three
depths — the floor/ceiling shape the dogfood spec established for governance, repeated on the
value plane exactly as it predicted.

## 4. The filesystem fabric (what agents stitch)

- **`.epr-meta` gains a `flows:` binding** (schema owned by eprfs-core's model, parsed by
  eprfs-meta): a directory binds `recipe: <id>@<version>` + `role: <stage>` — declaring "artifacts
  under here are this stage of that process." Same cascade, same pin discipline, same
  policy-registry lift as governance rules today. A compose-gate rule is hereby understood as the
  special case: *a governance predicate attached to a recipe edge*.
- **`.eprfs/status/` sidecar** becomes the observation floor: append-only flow events (dag-cbor
  lines, CID'd) accumulated offline against the container `EprRef` — tokens spent against a
  capability, watch-minutes, gate results — reconciled honestly on reconnect (the named espresso
  shape: resource+provenance · provider · measure · time · attribution · consumer).
- **The walk** (`epr flow walk|frontier|status` in epr-cli): stand at any artifact; walk **back**
  (what commitment produced this, under which recipe stage, fulfilling which intent, from which
  epic) and **forward** (what does this feed; which downstream commitments are now unfulfilled
  because I changed this). The forward walk is the mechanical form of "if you change A, walk the
  valueflow to the finish before you claim coherence" — the cross-surface sync problem where
  interfaces are hard to define becomes a traversal, which is exactly the workflow minutiae that
  kills humans and that agents absorb. `remote` verdicts (kinship spec) keep walks honest across
  substrate boundaries: identified-but-not-local is never dead.

## 5. First instance — the dev pipeline as the virtual model

The dev pipeline is the dogfood recipe (`elohim-dev-pipeline@1`) whose walking teaches us what
real-world EPR processes need before they carry households:

```
manifesto → epic → architecture-seed → spec → plan → gap-items → a2o feature/scenario
        → glue → implementation → push-gates → playwright/a2o run → delta (spine/close-loop)
```

Mapping (every instrument already exists; emitters are thin adapters, not new checks):

| Fabric concept | Dev-pipeline instance (existing tool) |
|---|---|
| Resource CIDs | cite fingerprints = body CIDs (2026-07-12 convergence); code artifacts via brit CIDs |
| Intent | gap-item (`decompose.py` output; OPEN = unsatisfied intent) |
| Commitment | a /shift Objective claiming gap-items; CLAIMED status = commitment awaiting fulfillment evidence |
| FlowEvent | seal, decompose, scenario-parse, gate pass, a2o verdict, close-loop delta |
| Attestation | pre-push gate pass (already framed as reach-earned attestation); parity fixtures |
| Fulfillment | playwright/a2o green `fulfills` the scenario commitment; delta closes the epic edge |
| in_scope_of | the governing spec/epic EPR; directories via `.epr-meta` cascade |

**Minimal validation joints, epic → working code** (the recipe's `meaningful: true` edges — the
governance answer to "what really needs to be validated," v1):
1. spec born-linked (cites sealed) — *satisfies* the epic intent;
2. plan decomposed to gap-items — intents minted;
3. feature/scenario exists and parses (a2o) — commitment articulated as verifiable claim;
4. push-gates green — attestation event;
5. playwright/a2o green against the running system — fulfillment of the scenario commitment;
6. delta recorded (spine/close-loop) — the chain's terminal produce-event: trust.

Everything between joints is unmetered by default; a scope may bind a finer policy (2.4). Trust-
legibility states (2026-07-18 atlas) are this fabric read backward: an unearned-trust state is an
unfulfilled commitment with a declared earn-path; the gauge is fulfillment ratio.

## 6. P2P Design Gate output (summary)

- ProcessSpec: **A (existing)** — EPR content atom, CID; standing via `Mishpat::Precedent` lift. No new entry type.
- Intent: floor **C** (reconstructable from gap-items); graduation reuses existing DNA REA/proposal types — **verify headroom + existing types before ANY mint** (open verification item).
- Commitment: **A (existing)** — `Mishpat::Commitment`, cid = entry_hash (never action_hash); Slice-2a graduation rails.
- FlowEvent→EconomicEvent: **B2 via the D2 observation↔attestation split** — granular stays sidecar/local (never DHT; ~3000-entry budget), crystallizes per recipe-declared graduation policy into existing `EconomicEvent` (`bounded_by`) or `Attestation`.
- Process instance: **A2** — links anchored on the container EPR; no standalone entry type.
- Resource: no entity; state = fold (**C**, reconstructable).
- Fulfillment/Satisfaction: edges in hashed bytes + A2 links; not atoms.
- Address strategy: content-derived CID everywhere; `agent_cid` the only agent join key; no bare
  sha as address (cite short-form is a rendering of the body CID, per convergence).

## 7. Graduation map (virtual → real world)

The same shapes lift without redesign: sidecar atom → DHT entry (Commitment→Mishpat, FlowEvent→
EconomicEvent), diesel projections feed the rea-facing lens folds unchanged, and the VF bridge
translates 1:1 because names were VF-aligned from birth (M2+ bridge work consumes this crate's
types instead of thin local structs). Real-world scenarios (household care narration, mutual
storage, creator payments) then reuse the *same* recipe/plan/observation planes the dev pipeline
proved — with their own governed instrumentation policies deciding what matters at their scale.

## 8. Non-goals / follow-on captures

- **No UI** and **no new DHT entry types** in the first slice; no DNA action mints.
- **Action-vocabulary reconciliation** (`ReaVerb` ↔ storage actions ↔ schema enum → one generated
  enum) is a tracked follow-on, prerequisite for the storage `FlowStore` impl — sibling of the
  reach-enum-drift item.
- **Automatic recipe inference** (mining a recipe from observed flows) — research-flavored, later.
- **The elohim behavioral ceiling** (judge-a-walk-in-good-faith) — assumed-possible, not built;
  the floor is shaped to accept it (same anchor, deeper validation).
- **bridges/valueflows M2+** — consumes this, not built here.

## 9. Slices (sequence)

1. **`epr-rea` crate** — payload structs + folds + `FlowStore`/`FlowWalk` traits + sidecar
   `FlowStore` impl; unit tests are hand-built `Vec<FlowEvent>` folds (DB-free, the rea-facing
   test-first shape).
2. **`.epr-meta` `flows:` binding** — eprfs-core model + eprfs-meta parsing + cascade tests;
   dev-pipeline recipe authored as the first registry entry.
3. **Dev-pipeline emitters** — thin adapters appending flow events at the six joints (seal,
   decompose, a2o parse, push-gate, playwright verdict, close-loop delta).
4. **The walk** — `epr flow walk|frontier|status` over sidecar + cites graph + brit-graph
   adapter; a2o scenario: change a spec, walk forward, see the unfulfilled frontier.
5. **Storage alignment** — `FlowStore` over diesel rails (post action-vocabulary
   reconciliation); rea-facing folds consume epr-rea types.
