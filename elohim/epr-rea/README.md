# elohim-epr-rea

The REA/ValueFlows domain layer over EPR atoms — the value-chain fabric.

Spec: `genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md`.

## What this crate provides

REA (Resource-Event-Agent, McCarthy 2019) and its ValueFlows vocabulary, modeled as
canonical dag-cbor atoms over `elohim-epr`'s codec — the same CID mint, the same
tamper-evident-by-construction principle, applied to economics rather than to content.

### The ontological commitments

- **Workflow is a projection; the value chain is the model.** A "task list" view is
  derived by walking the chain; it is never stored as a first-class sequence. What's
  stored is conversion processes linked by duality (use/consume → produce).
- **Resource is not an entity.** Anything content-addressed *is* a resource — an EPR
  atom, a blob, a spec file whose cite fingerprint is its body CID. Resource *state*
  (quantity, custody, fulfillment) is a **pure fold over event history**
  ([`fold::resource_state`], [`fold::fulfillment`]), never a mutable struct — P1
  (storage as reconciliation controller) applied to economics.
- **Fulfillment and Satisfaction are edges, not atoms.** `fulfills: [Cid]`
  (commitment → discharging event) and `satisfies: [Cid]` (intent → answering
  commitment/event) are fields inside the hashed bytes — restating an edge changes
  the atom's own CID, so the edge is tamper-evident the same way lineage is.
- **Scale-free substrate, governed granularity.** Nothing in the type layer bounds
  what may be modeled — an agent *can* meter molecules. What *should* be metered is a
  governance decision expressed as policy (a recipe edge's `meaningful: bool`), bound
  per scope via `.epr-meta`, never a constant baked into this crate.
- **The EPR is the container, and container-appropriateness is fractal.** Every event
  and commitment carries `in_scope_of: Cid` — the accountable container. A container
  is itself a resource in its parent's scope; whether it's the right boundary at the
  right instrumentation level is judged one level up, recursively.
- **One action vocabulary.** REA actions are `elohim_epr::witness::ReaVerb`, re-exported
  here — deliberately no fourth enum alongside storage actions and the schema enum
  (reconciling those three is a tracked follow-on, spec §8).

### Three planes, one mechanism

| VF level | Type | What it captures |
|---|---|---|
| Knowledge (recipe) | [`model::ProcessSpec`] (`stages`, `edges`) | a named process definition — which stages exist, which edges between them are economically meaningful |
| Plan | [`model::Intent`], [`model::Commitment`] | a desired flow, a promised flow (`satisfies` an intent, carries `state: proposed\|active\|fulfilled\|revoked`) |
| Observation | [`model::FlowEvent`] | what actually happened — `fulfills` a commitment, `satisfies` an intent, moves a `resource: Cid` by a `quantity: Magnitude` |

[`model::Process`] is a live run of a recipe: `spec: PinnedRef` (a declared `id@version`
pin — which version applies is a dependency, never recency), grouping the `inputs`
consumed and `outputs` produced by its events.

## Public surface

```rust
// Payload structs (model) — canonical dag-cbor, CID via atom_cid()
ProcessSpec { id, version, stages: Vec<StageSpec>, edges: Vec<EdgeSpec> }
Intent      { action: ReaVerb, resource_spec, in_scope_of, raised_by }
Commitment  { action, provider, receiver, resource_spec, in_scope_of,
              valid_from, valid_until, state: CommitmentState, satisfies: Vec<Cid> }
FlowEvent   { action, provider, receiver, resource: Cid, quantity: Magnitude,
              process: Option<Cid>, in_scope_of, fulfills: Vec<Cid>,
              satisfies: Vec<Cid>, occurred_at }
Process     { spec: PinnedRef, in_scope_of, inputs: Vec<Cid>, outputs: Vec<Cid> }

// Folds (fold) — pure functions, DB-free
fold::resource_state(resource: &Cid, events: &[FlowEvent]) -> ResourceState
fold::fulfillment(commitment_cid, commitment, events) -> FulfillmentStatus  // .ratio()

// FlowStore (store) — the polymorphic persistence seam
trait FlowStore {
    fn append(&mut self, record: FlowRecord) -> Result<Cid>;
    fn records(&self) -> Result<Vec<(Cid, FlowRecord)>>;
    // + events(), commitments(), processes(), unfulfilled_in_scope() default methods
}
MemoryFlowStore   // in-memory — tests, short-lived walks
SidecarFlowStore  // the offline floor — see below

// FlowWalk (walk) — blanket-implemented for every FlowStore
trait FlowWalk {
    fn walk_back(&self, resource: &Cid) -> Result<Lineage>;      // -> producing events,
                                                                  //    processes, inputs,
                                                                  //    commitments, intents
    fn walk_forward(&self, resource: &Cid) -> Result<Frontier>;  // -> dependents, outputs,
                                                                  //    unfulfilled commitments
}
```

`FlowRecord` is the append-only envelope (`Intent | Commitment | Event | Process | Spec`);
its `cid()` is always the atom CID of the *payload*, never the envelope — the envelope
tag is a storage detail and never participates in identity.

## The sidecar floor

`SidecarFlowStore` opens (creating as needed) `<root>/.eprfs/status/flows.jsonl` — an
append-only JSON-lines log, one record per line, each line carrying its own dag-cbor
atom CID alongside the record. On read, every line's CID is recomputed and checked
against the stored value: a mismatch returns `FabricError::Integrity { stored,
computed }` rather than silently accepting a tampered or corrupted line. This is the
observation floor described in the spec — flow events accumulated locally against a
container `EprRef` (tokens spent, watch-minutes, gate results) before any network
reconciliation, honored by construction rather than by trust.

## Graduation map

The same shapes lift without redesign as they move from the sidecar floor toward the
DHT ceiling:

| Sidecar (this crate) | Graduated home |
|---|---|
| `Commitment` | `Mishpat::Commitment` — `cid = entry_hash`, never `action_hash`; Slice-2a graduation rails (`proposed → active`) |
| `FlowEvent` (granular) | crystallizes per recipe-declared policy into `EconomicEvent` via `call_create_rea_economic_event` (`fulfills` → DHT `bounded_by`) or an `Attestation` |
| `Intent` | reconstructable from gap-items (floor tier C); DNA graduation reuses existing REA/proposal types — verify headroom before any new mint |
| `ProcessSpec` | standing via `Mishpat::Precedent`; no new DHT entry type |
| diesel projections (`economic_events`, `rea_commitments`) | unchanged — the rea-facing-lens folds this crate's shapes were lifted from keep consuming them |
| VF bridge (`bridges/valueflows`) | translates 1:1 once M2+ work consumes this crate's types directly, because names were VF-aligned from birth |

No new DHT entry types and no UI ship in this slice; see the spec's §8 for the
follow-ons this deliberately defers (action-vocabulary reconciliation, automatic
recipe inference, the elohim behavioral ceiling on judging a walk in good faith).

## The dev-pipeline dogfood instance

The first recipe walked with this crate is the repository's own development pipeline
(`elohim-dev-pipeline@1`, registered in `.claude/epr-meta/recipes.yaml`): manifesto →
epic → architecture-seed → spec → plan → gap-item intents → a2o scenario →
validation. Every instrument in that chain already exists (cite seals, `decompose.py`,
a2o parse/run, push-gate results) — projecting it into `FlowRecord`s and walking it
with [`walk::FlowWalk`] is how "if you change A, walk the valueflow to the finish
before you claim coherence" becomes a traversal an agent can run instead of a judgment
call it has to remember to make.
