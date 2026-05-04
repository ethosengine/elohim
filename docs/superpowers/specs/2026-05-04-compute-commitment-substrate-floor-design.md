# Compute Commitments as Bounded REA Primitives — Substrate Floor with Elohim Ceiling

> **Status:** Design / story-harvest spec. Forward-compatible principle, schema sketch
> only. Full schema authoring, validators, and rakia DNA wake-up are deferred to a
> follow-up implementation plan when the protocol-side work is prioritized. This
> document captures the lesson the alpha cluster taught us during the shem outage of
> 2026-05-04 so future plans cite a stake-in-the-ground rather than re-discovering the
> principle.

## Context

On 2026-05-04 a worker node ("shem") suffered a catastrophic PSU failure. Three deployed
contributors (adam, pete, frank) carried `nodeTypes: ["remote"]` and were stranded.
Their 40 GiB openebs-hostpath PVCs were unrecoverable. The operator suspended them
declaratively (commit `ed384b72`), the genesis seeder filtering caught up
(commit `3c026e2f`), and the cluster restabilized.

The outage exposed a question the schema had never been asked:

> When a contributor's compute is offline (or destroyed), what flows continue, what
> flows pause, and what flows breach?

A schema-level audit said the layers are cleanly separated — `ContributorPresence`
states (`unclaimed`/`stewarded`/`claimed`) don't encode liveness, REA `Commitment`
records carry no `paused_due_to_offline` field, and `EconomicEvent` recording has no
liveness gate. But schema cleanliness ≠ functional decoupling. The brainstorm that
followed surfaced the principle this document encodes.

The alpha cluster is severely compute-constrained relative to the network we are
designing toward. This is *transient development friction*. The harvest is **not**
"design for scarcity" — the harvest is **"compute commitments must be first-class
bounded economic objects from day one, because every node, no matter how rich, has
limits."** Real-network family nodes will be 64–128 GiB / GPU-capable, but they will
still commit bounded scopes to bounded counterparties for bounded windows. Breach is
normal, not exceptional. The protocol records it gracefully.

## Principle: Substrate-Deterministic Floor + Elohim-Discernment Ceiling

The protocol has two architectural layers wherever a decision must be made:

| Layer | Job | When it runs | Output |
|---|---|---|---|
| **Substrate floor** | Capacity arithmetic. Standing-agreement execution. Deterministic verdicts on requests. Recording mechanical truth. | Always. No AI required. | Granted / Denied / Pending / Fulfilled / Breached, with explicit reason. |
| **Elohim ceiling** | Discernment, wisdom, value minting, contextual override with rationale. Authoring/revising/retiring standing agreements. | When the household elohim is alive and has context. | `FeedbackSignal` entries linked to the substrate's verdict. Rich provenance. |

**The substrate must be able to function (slower, simpler, less wise) without
elohims.** Elohims enrich, they do not enable. This pattern already governs the reach
gate, recovery flows, and attribution recording. Compute commitments are the next
surface to apply it to consistently.

Two corollaries of the principle:

- **Allocation vs minting are different jobs.** Allocation is mechanical capacity
  arithmetic — substrate's. Minting is contextual valuation of a contribution —
  elohim's. Today the same human might do both; tomorrow they're separate layers in
  the protocol stack.
- **Elohim never gates substrate, only enriches it.** A request that the substrate
  granted remains granted; the elohim layer can record discernment as a parallel
  signal (e.g., "this counterparty has a pattern I distrust"), but the substrate's
  verdict stays queryable as the floor truth. The reverse is true too: the elohim can
  *accept what was substrate-denied* by issuing an exception commitment, but the
  substrate's denial is preserved as the rationale.

## Two Contract Families, Structurally Separated

A contributor's valueflows split into two families with different breach semantics:

### Attribution-class

- **Examples:** authorship, citation, witness, learning-credit, contribution
  attribution, recognition, vouch, sponsor, reputation
- **Liveness sensitivity:** none. These continue regardless of compute state. Authored
  by other peers' cells when the originator is offline. Replicate via DHT and surface
  in storage projections without the originator's signature on every read.
- **Breach semantics:** N/A — they don't breach when compute fails. They are records
  of what happened, not promises of what will happen.

### Compute-class

- **Examples:** reciprocal hosting, validator participation, inference provision,
  shard storage, gossip relay, scheduled task execution
- **Liveness sensitivity:** total. These can only fulfill when the executor is
  available.
- **Breach semantics:** first-class. Breach is recorded as an economic event with
  cause classification (catastrophic-loss, transient-unavailability, capacity-full,
  scope-reduction, voluntary-retirement). **Breach in compute-class does NOT propagate
  to attribution-class.** A contributor whose hardware is destroyed retains their
  authored work, citations, recognition, and standing — only their forward-looking
  compute commitments breach.

This separation is the load-bearing decoupling. Today's `Commitment` record carries no
distinguishing field; the spec sketch below adds one.

## Schema Sketch (Sketch Only — Not Authored)

The brainstorm settled on **option B**: extend the existing `Commitment` entry via a
manifest-declared `signal_kind` rather than introducing a new entry type or waking the
backburnered rakia DNA prematurely.

### `signal_kind: "compute-allocation"` (new)

A `Commitment` record carrying this `signal_kind` represents a compute-class
commitment. Payload fields (illustrative — final schema deferred):

```jsonc
{
  "signal_kind": "compute-allocation",
  "magnitude": {
    "cpu_m": 500,
    "memory_Mi": 1024,
    "ephemeral_storage_Gi": 10,
    "persistent_storage_Gi": 0,
    "egress_Gi_per_day": 5
  },
  "trigger_kind": "request-driven" | "standing" | "subscription",
  "availability_term": {
    "kind": "always-on" | "scheduled" | "best-effort" | "subscription-window",
    "window": "..."  // structure depends on kind
  },
  "counterparty": {
    "kind": "peer" | "household" | "collective" | "open-substrate",
    "id": "..."  // human-id, household-id, collective-id, or null for open-substrate
  },
  "balance_basis": {
    "capacity_ledger_cid": "...",  // the compute-capacity.json snapshot version this
                                     // commitment was negotiated against
    "headroom_at_negotiation": { "cpu_m": 34820, "memory_Mi": 110584 }
  },
  "breach_terms": {
    "grace_period": "PT15M",
    "subsidy_eligible": true,
    "recovery_contract_required": false
  },
  "negotiated_by": {
    "kind": "substrate-floor" | "elohim",
    "elohim_signature": "..."  // present when elohim authored or revised
  }
}
```

### `signal_kind: "compute-breach"` (new)

A separate signal recorded when a `compute-allocation` commitment can't fulfill:

```jsonc
{
  "signal_kind": "compute-breach",
  "commitment_ref": "...",  // CID of the breached compute-allocation commitment
  "cause": "catastrophic-loss"
        | "transient-unavailability"
        | "capacity-exhausted"
        | "standing-execution-missed"
        | "subscription-window-violated"
        | "scope-reduced"
        | "voluntary-retirement",
  "witness": {
    "kind": "self" | "counterparty" | "collective",
    "agent_id": "..."  // who attests; counterparty is the natural witness when
                        // self is offline (e.g., catastrophic loss)
  },
  "recovery_path": "rebootstrap-required" | "resume-on-return" | "permanent",
  "narrative": "shem PSU failure 2026-05-04, PVCs unrecoverable"
}
```

### Three trigger kinds

The substrate floor is responsible for executing all three; elohim's role differs by
kind.

| trigger_kind | Substrate's job | Elohim's job (when present) |
|---|---|---|
| `request-driven` | Schedule on incoming request, k8s-style. Capacity arithmetic, deterministic verdict. | Discern whether the counterparty/purpose aligns with household values; mint reciprocity credit on fulfillment. |
| `standing` | Execute the agreement deterministically when conditions fire (cron, threshold, gossip-rule). | Author / revise / retire the standing agreement. Once authored, agreement runs on substrate alone. |
| `subscription` | Reserve capacity in the agreed window. Refuse competing requests during the window. | Negotiate the subscription terms; mint relationship/loyalty credit; arbitrate disputes. |

## Why This Lands at Manifest+Signal Rather Than New DNA

- **Existing extensibility pattern** — `signal_kind` is the protocol's declared way to
  introduce new social/economic moves without new entry types. Memory:
  `project_signal_kind_extensible_protocol_class.md`.
- **Preserves DNA capacity** — Lamad DNA is at ~73/~100, Mishpat at 11/~100. Don't
  burn entry-type budget on a primitive that fits the existing Commitment shape.
- **Forward-compat with rakia** — when rakia DNA wakes up to host
  `StewardCommitment` as a Category B2 entry, the payload shape we sketch here
  migrates cleanly. The `compute-capacity.json` ledger schema we already authored is
  intentionally migration-friendly.
- **Substrate functions immediately** — once the manifest declares
  `compute-allocation` and `compute-breach`, the storage layer can record them
  deterministically without a coordinator function. Validators can be added later as
  the discernment layer matures.

## What's Already Built (Inputs to This Design)

- **`genesis/data/rakia/compute-capacity.json`** — operator-authored capacity ledger,
  schema-validated, real cluster numbers. Becomes the `balance_basis` input to
  request-driven negotiations.
- **`genesis/orchestrator/scripts/snapshot-capacity.sh`** — re-runnable kubectl
  extraction script. Future: an elohim subagent runs this and proposes ledger updates.
- **`genesis/orchestrator/data/deployments.json`** — already declares `nodeTypes`,
  `edgenodeMemoryRequest/Limit`, `edgenodeCpuRequest/Limit`, and now `genesisPeer`,
  `suspended`. Migration path: these per-human fields become inputs to a household
  elohim's first-pass standing-agreement portfolio.
- **Suspension pattern** (commits `ed384b72`, `f96b53ee`, `3c026e2f`) — the operational
  precedent for "this contributor's compute is offline; their attribution-class flows
  must continue." Today expressed in devops state; harvest target is to lift it into
  protocol-recorded breach events.

## What's Deferred

| Item | Why deferred |
|---|---|
| Full schema authoring for `compute-allocation` / `compute-breach` payloads | Sketch is sufficient to ground the principle; final fields settle when the implementation plan starts and we know which validators ship with it. |
| Validator zomes | Need the schema first; need to decide stage 1 (structural/social) vs stage 3 (full elohim enforcement) per memory `project_bootstrap_to_elohim_security_gradient.md`. |
| Rakia DNA wake-up | The lineage upgrade path is in memory `project_lineage_rna_upgrade_path.md`. Wake when the DNA's larger purpose is ready, not piecemeal for compute commitments. |
| Elohim discernment layer for compute negotiation | The substrate floor is the prerequisite. Discernment lights up when household elohim agents are running. |
| Standing-agreement execution engine | Substrate-resident scheduler / gossip-rule engine that fires standing agreements deterministically. Significant engineering surface; out of scope here. |
| Capacity-ledger live ground-truth feedback loop | Today the ledger is operator-refreshed via `snapshot-capacity.sh`. Future: substrate observes its own capacity and the elohim consults the live observation. |

## Story Harvest

A `@regression` a2o feature file lands at
`genesis/a2o/features/deployment/compute-commitment-bounds.feature`. It encodes the
principle as substrate-level scenarios stewards experience: catastrophic compute loss
that does not silence authored work, deterministic substrate verdicts that work
without elohim, and standing agreements that continue firing through human absence.
The scenarios are `@wip` — they describe the contract the protocol must honor when
the implementation lands, not the current state.

A project memory at
`/projects/.claude-config/projects/-projects-elohim/memory/project_substrate_floor_elohim_ceiling.md`
crystallizes the principle so future brainstorms apply it consistently to other
surfaces (recovery, attribution, governance) without re-deriving it.

## Cross-References (Existing Memory)

This design composes with — and does not contradict — these prior memories. New
brainstorms should consult them before proposing alternatives.

- `project_depin_contracts_are_policy.md` — DHT holds policy (commitments), libp2p
  handles mechanism (distribution, availability). Compute commitments are policy.
- `project_placement_signals_are_shefa_inputs.md` — gaps/breaches/recoveries are
  structured economic signals, not operational warnings. `compute-breach` is one such
  signal.
- `project_three_layer_truth_model.md` — DHT=notary, libp2p=data-ops, doorway=web2
  projection. Negotiation messages travel libp2p; final commitment lands on DHT;
  doorway projects breach state to non-substrate consumers.
- `project_reach_gate_is_elohim_mediated_matchmaking.md` — substrate gate is
  deterministic floor returning {Allowed, Blocked, Pending}; elohim discernment is
  additive matchmaking. Compute commitments apply the same pattern.
- `project_signal_kind_extensible_protocol_class.md` — new social/economic moves
  extend `signal_kind`, not entry types. This design respects that.
- `project_elohim_agent_sense_respond_architecture.md` — discernment lives in
  elohim-agent (Rust); manifests declare which gates apply; .ts is sense-and-respond
  only. Compute discernment lives in elohim-agent when it wakes.
- `project_bootstrap_to_elohim_security_gradient.md` — Stage 1 structural/social
  validators; Stage 3 full elohim enforcement. Compute commitments start at Stage 1
  (substrate-deterministic), graduate to Stage 3 as elohim layer matures.
- `project_lineage_rna_upgrade_path.md` — rakia DNA work is backburnered; this design
  is intentionally migration-friendly to it.
- `project_stewardship_philosophy.md` — graduated capability, accountable authority,
  visible shape. Catastrophic compute loss is in the same family as stewardship through
  disability, custody handoff, or elder transition; protocol must treat with equal
  dignity.

## Acceptance Criteria for This Spec

This spec is "done" when:

- [ ] The principle (Substrate-Deterministic Floor + Elohim-Discernment Ceiling) is
      named, documented, and cited from a project memory.
- [ ] The two contract families (attribution-class, compute-class) are documented with
      explicit decoupling guarantee — a compute-class breach must never contaminate
      attribution-class flows.
- [ ] The `signal_kind: "compute-allocation"` and `signal_kind: "compute-breach"`
      sketches exist as forward-compatibility stakes — future implementation plans
      cite them rather than re-deriving.
- [ ] The three trigger kinds (`request-driven`, `standing`, `subscription`) are
      enumerated, so the substrate-floor scope is unambiguous when an implementation
      plan starts.
- [ ] The shem outage is harvested as a `@wip @regression` a2o feature file the future
      implementation must satisfy.
- [ ] Cross-references to all relevant prior memories are explicit.

A future plan implements the schema, validators, and substrate scheduler; this spec
is the stake those plans cite.
