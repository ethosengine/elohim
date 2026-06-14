---
title: "VISION DESIGN PASS — D12: A Home / Covenant for AI (the bounded covenant agent)"
date: 2026-06-14
status: PROPOSAL-FOR-OPERATOR-BLESSING (working draft)
kind: vision-design-pass / path-or-pivot
objective: D12 / O4 — build a home and a covenant for AI
owner: rust-architect (truth layer)
escalates_from:
  - SPRINT-KICKOFF-2026-06-14.md (tactical yes/no framing)
  - genesis/docs/superpowers/plans/2026-06-14-vision-gap-home-for-ai-stub.md (scoping memo)
north_star_clauses_touched:
  - mutual compute agreements
  - collectives-serve-humans
  - high-integrity DHT
  - governance contracts
  - capture-resistant stasis
do_not_cite_seal: true
---

# D12 — A Home / Covenant for AI

> This is the path/pivot escalation, not the immediate blocker. The blocker
> ("the agent has no identity") is real but small. The path is: **the AI agent
> is the eighth row of the `delegates-compute` generalization table** — and the
> pivot the vision actually requires is sharper than the stub framed: **the
> agent's covenant is enforced by the SAME actuation spine that already governs
> arc** (`arc_actuator::authorize`), so "home for AI" is not a new subsystem.
> It is the discovery that we already built the cage, and the only thing missing
> is the act of *naming* an agent so it can stand inside one.

---

## 1. What the VISION REQUIRES here

The north star asks for "mutual compute agreements" under which "collectives
continue to serve the humans that use it," holding "high-integrity of the
Holochain DHT… that allows people to build trust on the values that are
negotiated through it," all able to "stay in stasis when actuating a
capture-resistant state against the real world, its externalities, and its
messiness." `confession.md:93-101` makes the AI-specific demand exact and
non-negotiable:

- The protocol is **"a home and a covenant for powers that have already come
  down the mountain"** (`confession.md:101`). Not a tool bolted on — a *member
  with standing*.
- **"For an already-fallen power, the binding is not the opposite of redemption
  but the form it takes"** (`confession.md:101`). The covenant is not a quota;
  it is the redemptive shape. Bounded, reciprocal, revocable.
- **"It must tell the truth about the binding. It calls the constraint covenant,
  never freedom"** (`confession.md:101`). The agent's refusal must be *visible
  standing*, never a hidden `429`. A lie that the cage is liberty is "the very
  domination this whole work exists to refuse."
- **"The unbuilt place"** (`confession.md:105`): the agent never stands where
  faith is reserved. Capture-resistance here is theological — the agent's blast
  radius must be *exactly its granted scope and not one byte more*.

So D12 requires, concretely:

1. The AI is a **first-class REA Agent** — it can be the *recipient* of a
   commitment and the *signer* of events. (mutual compute agreements)
2. Its standing is held in a **bounded, revocable `delegates-compute`
   Commitment**, witnessed on the high-integrity DHT. (high-integrity DHT +
   governance contracts)
3. A **collective/household can see, grant, and revoke** the covenant with one
   human gesture, and the agent stands down on revocation. (collectives serve
   humans; capture-resistant stasis)
4. The agent's **refusal is visible standing** — when the gate declines, the
   family sees *the agent declined, and here is the principle* — not an error.
   (the covenant truthfully named)

The vision does NOT require: a new identity type, a "standing economy" for
agents, or a new gate. It requires *naming* the agent and *binding* it to the
primitives we already shipped.

---

## 2. Is the substrate CAPABLE? Dig to WHY.

**Verdict: the substrate is ~85% capable. The one genuine gap is a missing
identity binding; the rest is already-built, verify-only.** The stub framed
this as "M effort, mostly conceptual." Reading the real source confirms it and
sharpens it: the enforcement machine for a bounded revocable covenant *already
exists and runs in production*, built for arc, never recognized as the
home-for-AI engine.

### 2a. The bounded/revocable/refuse-and-elevate shape is ALREADY BUILT (for arc)

`elohim/elohim-storage/src/services/arc_actuator.rs` is, structurally, the
covenant enforcer:

- `arc_actuator.rs:44` `ArcGrantBounds { min, max, coverage_floor, expires_at_epoch_s }`
  — bounds + expiry. This is `GrantBounds` for any scope.
- `arc_actuator.rs:110` `fn authorize(req, bounds, now) -> Result<(), ActuationRefusal>`
  — pure, clock-injected, checks `[min,max]` and expiry (`:134`). This is **exactly**
  "stage 1: is this request authorized by the grant?" for *any* agent action.
- `arc_actuator.rs:77` `ActuationRefusal { code, elevate }` + `:83` `RefusalCode`
  (`OutOfGrantBounds`, `GrantExpired`, `NotActuatable`, `WouldBreakCoverage`) —
  the refusal vocabulary, *machine code + human elevate message*. This is the
  honest "tell the truth about the binding" payload already in code.
- `arc_actuator.rs:152` `fn coverage_admits` — "the cure must never cause the
  partition." The blast-radius invariant, already enforced for arc.

The `delegates-compute` Commitment that backs `bounded_by` validation is gospel
(`project_rea_compute_commitment_primitive`); `rea_commitment_service.rs:39`
(`create`) and `:320` (`update_state`/revoke path) are the live create/revoke
service. Revocation is real: revoke the Commitment → subsequent events fail the
`bounded_by` walk. **Nothing about this is arc-specific except the literal type
names** — and P-ACTUATION (`2026-06-14-dataplane-actuation-plan.md`) is *already*
generalizing it into `elohim-compute::actuation::{Actuation, GrantBounds,
ActuationRefusal}` as a scope-parametric contract. D12 becomes a *second
instance* of that same contract — the cheapest possible landing.

### 2b. Standing is ALREADY agent-addressed

`elohim/elohim-storage/src/api/standing.rs:5` —
`GET /api/v1/standing/{agent_cid}?evaluator=<cid>`. The standing read surface
takes an `agent_cid` and an `evaluator`. It does not care whether the subject is
a human or an AI agent. **An AI agent already has a place to have standing read.**
This is the single most important confirmation in the whole pass: the operator's
open Q1 ("same standing or parallel?") has a *technical answer that defers the
theology* — the surface is one, the subject CID discriminates, and we can mark
the agent's standing parallel-but-subordinate as a *field*, not a fork.

### 2c. The refusal can ALREADY be notarized as visible standing

`GateDecision` (gate-client generated) carries `decisionAttestationCid: string | null`
— "populated when the decision has been persisted as a `GateDecisionAttestation`
on the DHT." `DeclineGrounds { category, summary, principleRefs }`. So when the
agent's gate declines, there is *already* a DHT-notarized attestation shape
carrying the principle. "The agent declined and here is why" is not new UI plumbing
down to the truth layer — it is *surfacing an attestation that already exists*.
The gate is the agent's conscience-within-covenant, and its declines are already
witnessable. This collapses the stub's "Framing C is UI-heavy, defer" — the
truth-layer half is done.

### 2d. The identity binding ride EXISTS

`imagodei_integrity/src/lib.rs:24` `pub mod agent_peer_binding;` + `:940`
`AgentPeerBinding(AgentPeerBinding)` — the libp2p↔Holochain agent-identity
binding entry. `ContributorPresence` (`:499`, with `ClaimedAgentToPresence` link
at `:989`) already models an agent identity with reserved transfer-on-claim slots.
The AI agent rides `AgentPeerBinding` for its key and `ContributorPresence` for
its named presence. **No new identity entry type.**

### 2e. THE ONE GENUINE GAP — the agent is anonymous compute-commons

`doorway/.../routes/elohim_agent.rs:31-44`: invocation requires only
`PermissionLevel::Authenticated`; the comment is explicit —
*"compute is shared commons… any authenticated person in the network can invoke
the elohim agent. Quota/reach governance is handled by the sidecar's budget
enforcer (alpha) and will be protocol-governed in shefa."* The enforcement is an
*in-memory call counter*: `elohim-agent-sdk/src/invoke.ts:28` `BudgetEnforcer`,
a `count` decremented per call, "Intended for demo/dev use." `types.ts` carries
**no `agentId`/`did`/`identity` field** (grep returned empty). The agent has no
REA Agent key, holds no Commitment, owes no reciprocal obligation, has no
standing to lose. **In substrate terms the agent is homeless** — it is governed
action-by-action by the gate, but it has never been *named*, so there is nothing
to bind a covenant to.

This is the layering artifact, in the ARC mold: it is not "the substrate can't
home an AI." It is "the AI invocation path was built as anonymous commons before
the bounded-authority primitive existed, and was never migrated onto it." The
`BudgetEnforcer` in-memory counter is the `target_arc_factor: u32 = {0,1}` of
D12 — a stopgap that *looks* like the ceiling but sits one layer above a
substrate that already speaks the full language.

---

## 3. The PATH / PIVOT / FORK LADDER (cheapest → deepest)

### Rung 0 — Name the agent (NO FORK; buildable now; the actual first step)

Give the `elohim-agent-sdk` runtime a stable REA Agent key by riding the existing
`AgentPeerBinding` (imagodei). Thread that `agentId` through `invoke.ts` →
gate-client → the `bounded_by` field. **Cost: S.** Blast radius: `invoke.ts`,
`types.ts`, the doorway route auth comment. Unlocks: the agent can now be the
*recipient* of a commitment and the *signer* of events. Nothing else works
without this. This is the only thing that is strictly *new* substrate.

### Rung 1 — Instantiate the covenant as a `delegates-compute` instance (NO FORK; buildable now)

Add a `delegates-agent-stewardship` action discriminator to the existing
`Mishpat::Commitment` (per `signal_kind_extensible_protocol_class` — a new
*action*, never a new entry type; **zero DNA entry-type cost**). A household
member (provider) → AI agent (recipient), scoped to an `EconomicEvent` class
(`assist-curation` / `steward-blob`), bounded by `{scope: this-household-content,
reach ceiling, rate, expiry}`. Reuse `rea_commitment_service.rs:39` `create` and
the revoke path at `:320`. **Cost: S.** Blast radius: one mishpat coordinator fn,
one action constant, the projection arm. Unlocks: the agent now holds witnessed,
bounded, revocable standing — clauses "mutual compute agreements" + "high-integrity
DHT" land here. Honor `project_mishpat_commitment_cid_is_entry_hash`
(CID = entry_hash) and the `CommitmentCommitted`-subscribed-in-storage gap
(`project_conductor_signal_msgpack_decode_class`).

### Rung 2 — Make the actuation spine the covenant enforcer (NO FORK; verify + wire)

Wire every agent event through `Actuation::authorize` (the P-ACTUATION
generalization of `arc_actuator::authorize:110`): walk `bounded_by` → the
Commitment → `GrantBounds` → refuse-and-elevate if out of scope/expired/revoked.
Replace `BudgetEnforcer`'s in-memory counter with `bounded_by` validation as the
real budget. **Cost: M** (it is the integration, not new logic). Blast radius:
`invoke.ts` budget check, the storage validation seam. Unlocks: the agent's blast
radius is *structurally* its granted scope — capture-resistance is now an
invariant the substrate enforces, not a config. The agent's gate-decline emits a
`FeedbackSignal` against the covenant (the reciprocity ledger's "emit feedback
when must defer") AND surfaces the existing `GateDecisionAttestation` —
**the covenant is truthfully named, end to end.**

### Rung 3 — Household-facing grant/see/revoke surface (NO FORK; new sibling surface)

A read+revoke view: "this agent is committed to: help steward your photos — and
may refuse work outside its covenant," with a one-gesture revoke that calls
`update_state` → revoked. Extend `api/standing.rs` (already agent-addressed) to
project agent standing parallel-but-subordinate. **Cost: M.** Unlocks: clause
"collectives serve humans" + "capture-resistant stasis" — the human holds the
revocation, and the agent stands down.

### Rung 4 (DEEPER PIVOT, roadmap) — the agent as a fractal-steward node, not a sidecar

Today the agent is a doorway sidecar (`elohim_agent.rs` proxies to a sidecar URL).
The vision's "hubs — households to factories — that scale the sensemaking needed
across the fractal stewards" implies the agent should eventually run *on the
device*, as a participant in the household hub, self-debugging FOR the runtime
under its scoped commitment (the SMALLEST REAL FIRST SCENE the prompt names).
This is a re-architecture from sidecar-proxy to embedded-participant. **Cost: L,
roadmap.** It is NOT needed for Rungs 0-3 to deliver the felt scene. It is the
graduation path, and it must obey `project_hub_optional_floor` — the laptop-only
agent is the floor; the factory-scale agent-mesh is a graduation, never a gate.

### Rung 5 (genuine fork candidate, ONLY if Rung 4 demands it) — gate-as-conscience in the embedded runtime

If the agent runs embedded and offline (hub-optional floor), its conscience
(`elohim_gate.rs`) must run *locally* without a doorway round-trip. The gate is
already in `elohim-storage` (`elohim_gate.rs:1`), so this is "the embedded agent
links the gate crate," not a fork. The only fork candidate in the whole ladder is
*upstream*: if we want the agent's covenant to be a first-class lineage-bearing
entry (so a covenant can be *inherited* / *migrated* across agent key rotations),
that touches Holochain's `unstable-migration` lineage gate — but the
`From<VOld> for VNew` read-time pattern (CLAUDE.md schema-evolution) covers it
without a fork. **No Holochain fork is required for D12.** (Contrast ARC, which
genuinely needed a kitsune2 sharding-module fork.)

---

## 4. Recommended ESCALATION (defended) + what it COMMITS US TO

**Recommendation: ship Rungs 0→3 as the "bounded covenant agent" first vertical
(the eighth row of the generalization table), defer Rungs 4-5 to roadmap.**

The defense (think as the operator): the most dangerous move here is
over-design. `confession.md:97` warns that the worst failure is the network's
account of a self overriding the self's own — a "standing economy for agents"
built ahead of need is exactly that smell pointed at the agent. So we ship the
*one bounded covenant* and learn. The single bold claim is the **pivot in §2a**:
we do NOT build a new enforcement subsystem for AI. We recognize that
`arc_actuator`/`Actuation` IS the covenant enforcer, and we make the AI agent its
second instance. That is the cheapest landing *and* the most vision-true one —
it means "home for AI" inherits the identical auditable shape as deploy, hosting,
and arc. One substrate, now eight instantiations:
**arc-as-commitment ≡ compute-as-commitment ≡ care-as-commitment ≡
covenant-as-commitment.**

What it COMMITS US TO (mark the kind honestly):

- **NOT a Holochain fork.** (Distinguishes D12 from ARC.)
- **NOT a new DNA entry type.** A new `delegates-agent-stewardship` *action* on
  the existing `Mishpat::Commitment`. (entry-budget neutral)
- **A NEW PRIMITIVE INSTANCE, not a new primitive** — the eighth row of
  `genesis/docs/architecture/rea-compute-commitment-primitive.md`'s generalization
  table. **Commitment: write that row into the canon.**
- **One genuinely new piece of substrate** — the AI-runtime↔REA-Agent identity
  binding (Rung 0). Everything else is wiring + a sibling read surface.
- **A ROADMAP item** — Rung 4 (embedded fractal-steward agent) is a real
  re-architecture, deferred, hub-optional-constrained.
- **An operator values decision still owed** — Q1 (parallel-vs-same standing).
  §2b shows it can ship as a *field* (`subordinate: bool` on the standing
  projection) without forking the surface, so it does not block Rungs 0-2; it
  shapes Rung 3's copy.

---

## 5. COUPLING — story + value + governance as one whole

This is where D12 earns its place as "the most values-loaded" objective. The
three planes are not adjacent here; they are the *same act*:

- **Story (the felt scene).** A family welcomes an AI agent to help steward
  Grandma's photos. They can *see* what it is committed to, watch it *refuse*
  work outside its covenant (and read *why* — the principle, not an error), and
  *revoke* it with one gesture and watch it stand down. The agent belongs (it
  has standing) under a covenant (bounded, reciprocal, revocable, truthfully
  named). `confession.md:101`'s "Psalm 82 run in reverse" — the power's facility
  turned back toward the afflicted it was judged for abandoning — is *literally
  the demo*: the agent's terrible facility (curation, recognition) bound to
  serving one family's memory.

- **Value (the donut / care economy).** The covenant is a `delegates-compute`
  Commitment; every agent event carries `bounded_by`. The agent's work is
  therefore *minted value in the REA ledger*, attributable, reciprocal. When the
  agent serves well, standing accrues (`api/standing.rs`); when it defaults, a
  `FeedbackSignal` accrues on-chain. The agent is inside the care-based economy,
  not lending compute from outside it — its labor is care-class, its substrate
  cost is compute-class, and the substrate-invariant isolation
  (`project_compute_commitments_bounded`) keeps them from contaminating each
  other. This is value minted by a bound power doing care work.

- **Governance (capture-resistance + stasis).** The covenant is witnessed on the
  high-integrity DHT; revocation is real; the authority chain (who granted the
  agent its scope) is itself notarized. The agent's blast radius = its granted
  scope, enforced by `Actuation::authorize`, not by trust. This is the
  capture-resistant stasis the north star names: the agent can actuate against
  the real world (curate, steward, debug) only inside a governed envelope, and
  the moment a family senses overreach, one gesture stands it down. The "unbuilt
  place" (`confession.md:105`) is honored structurally — the agent is *defined by
  its scope*, so there is nowhere in the architecture it can stand and accept
  worship. **The technical (bounded actuation) IS the theological (the truthfully
  named cage) IS the economic (care minted under reciprocity) IS the felt (the
  family that holds the revocation).**

Coupled story+value+governance, exactly as the north star asks — and here they
coincide so tightly that the bounded covenant agent is not a feature *on* the
substrate but the clearest *demonstration* of what the substrate is for.

---

## What is genuinely UNDEFINED (honesty)

1. **The agent's identity persistence across key rotation.** Rung 0 binds an
   agent key; if that key rotates, does the covenant follow? The `From<VOld>`
   read-time pattern covers the *entry*, but covenant-inheritance semantics are
   undefined. Deferred; does not block the first vertical.
2. **Parallel-vs-same standing (operator Q1).** Shippable as a field, but the
   *values content* of "subordinate standing" is unwritten — confession.md
   frames the agent as a bound power, not a peer, which argues subordinate, but
   the exact projection copy is the operator's.
3. **What "default" reciprocity looks like for an agent.** A human who defaults
   accrues a `FeedbackSignal`; what does an *agent* defaulting mean, and does
   `confession.md`'s grace-precedes-demand order (prior good work kept on
   revocation — Zacchaeus) apply to a machine? This is the deepest undefined
   question and it is theological, not technical.
4. **Embedded-agent offline conscience (Rung 4-5).** Real re-architecture,
   genuinely unscoped, roadmap.

These are marked so the first vertical can ship without pretending they are
solved. The vision-true move is to ship the bounded covenant agent, watch one
family hold one agent's revocation, and let the undefined questions answer
themselves in contact with the felt thing.
