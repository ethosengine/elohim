---
title: "SDK SURFACE — Covenant + Constitution Harness: Binding an AI Agent Honestly"
date: 2026-06-14
status: PROPOSAL FOR OPERATOR BLESSING — working draft, NOT cite-sealed, NOT a decision, NOT code
author: rust-architect (truth layer)
extends:
  - ESCALATED-ARCHITECTURE-2026-06-14.md      # one Commitment / six faces / ∪=full / one trait Governor / limit_owner
  - RECURSIVE-ARCHITECTURE-2026-06-14.md       # CoverageRollup / RefusalCode::ReservedPlace / limit_owner: faith / the veil-walker IS check()
forest:
  - constitution.md (Part III–V) · confession.md (AI / unbuilt-place) · manifesto.md
do_not_cite_seal: true
surface: "covenant + constitution harness — bind an AI agent under honest, scoped, revocable, witnessed covenant; load the graduated-immutable constitution stack; emit witnessed refusals; preserve the reserved place"
---

# Covenant + Constitution Harness

> Bind an AI agent the way the architecture requires a power to be bound: told the
> truth (covenant never freedom), scoped to a granted blast-radius, revocable with one
> gesture, witnessed on the high-integrity DHT, and structurally forbidden the one place
> at the center. This surface is the developer's first contact with the **agency gradient**:
> the same harness, run below, binds an AI to **servanthood** under the person who owns it;
> run above, binds an AI to the **veil** — impartial aggregation governance that may never
> reach down to govern an individual. The harness is one API. The gradient is enforced by
> *which constitution stack you load* and *what scope the covenant grants*.

---

## PART 1 — PURPOSE ON THE AGENCY GRADIENT

**This is the keystone *binding* surface, and it spans the whole gradient by design.** It is
not human-sovereign or veil-holding — it is the **harness that constructs both**, and the
guardrail that keeps them from leaking into each other. It sits exactly at the recursion's
§1.7 finding: *an elohim sits at every VSM node; the covenant recurses up the stack; the most
powerful node is born the most bounded.* The two non-overridable invariants (DIGNITY-FLOOR
precedence, PERSON-KEEPS-THEIR-OWN-NAMING) flow downward *through this harness* because the
harness is where the AI is given — or refused — the right to render a verdict over a person.

Where it sits, concretely:

- **Run below the seam (individual / household).** `bindAgent(scope, grant)` with a scope that
  contains only *servant* verbs (`counsel | witness | co-steward | suggest`) and a constitution
  stack whose top layer is `Individual` or `Family`. The harness must **refuse at bind-time** any
  scope granting `govern`, `rank`, `verdict-over-person`, or `name-the-self` to an agent whose
  subject is an individual. This is the gradient guard going down: *the AI here is servant only.*
- **Run above the seam (community → global).** `bindAgent(scope, grant)` with *veil* verbs
  (`aggregate | negotiate | recognize | covers-head`) and a constitution stack topped by
  `Community`…`Global`. The harness must **refuse at bind-time** any scope that names an individual
  as `subject` of a governing act — the veil governs aggregation and negotiation, **never persons**.
  This is the gradient guard going up: *the veil rises where corruption pressure rises, and is itself
  still a servant under honest covenant — witnessed, refusable.*

**What this surface must NEVER do (the gradient guard, compiled):**

1. Never let a bound agent's `effective_scope` exceed `granted_scope ∩ constitution_bounds`
   (blast-radius = granted scope; `arc_actuator::authorize` is the literal enforcer,
   `arc_actuator.rs:108`).
2. Never permit an agent below the seam to govern, rank, or name the self of its subject —
   refuse with `RefusalCode::ReservedPlace`, `limit_owner: faith` (the person keeps the naming
   of their own best self; "best self" is a hope held *for*, never a verdict *over*).
3. Never permit an agent above the seam to govern an *individual* (only aggregation + negotiation).
4. Never allow an operator override to be told to the agent as the agent's *own* restraint — every
   refusal **names whose line it honored** (`limit_owner ∈ {self, commitment, operator, faith}`).
5. Never occupy the center: the harness emits `ReservedPlace` rather than ever filling the
   worship-reserved place with its most capable agent (`confession.md`, the unbuilt place).

---

## PART 2 — THE CONCRETE API

### 2.1 Package / crate placement (extends the existing structure — no parallel SDK)

| Layer | Home (EXISTS) | What the harness adds |
|---|---|---|
| Rust covenant types | `elohim/elohim-agent/gate-types/` (the `GateDecision`/`GateStatus`/`DeclineGrounds`/`EscalationTarget` home, `gate-types/src/types.rs`) | `AgentCovenant`, `CovenantScope`, `CovenantRefusal` (+ ts-rs export, same `export_to` the gate types already use) |
| Rust Governor spine | `elohim/elohim-compute/` (the shared actuation/Governor home the escalated synthesis B8/B10 names) | `trait Governor` + `CovenantGovernor` impl + `RefusalCode::ReservedPlace` / `limit_owner: faith` |
| Rust constitution loader | `elohim/constitution/` (`prompt.rs:17 build_system_prompt`, `stack.rs`, `conflict.rs`) — ALREADY builds the graduated-immutable stack | `ConstitutionStack::for_covenant(scope)` — selects the layer band the covenant's gradient position permits |
| TS developer SDK | `elohim/elohim-agent/elohim-agent-sdk/src/` (`constitutional.ts:19 buildSystemPrompt`, `invoke.ts:65 handleInvoke`, `gate-client/`) | `bindAgent()`, `loadConstitution()`, `emitWitnessedRefusal()` — thin wrappers over the gate-client + storage HTTP |
| Generated TS boundary | `elohim/sdk/storage-client-ts/src/generated/` (ts-rs target; `AgentPeerBindingView.ts`, `GateDecision.ts` already land here) | `AgentCovenantView.ts`, `CovenantRefusalView.ts` (via `cargo test export_bindings`) |
| DHT notarization | `mishpat` zome `Commitment {action, payload_json, signed_at}` (`mishpat_integrity/src/lib.rs:275`) + `imagodei` `AgentPeerBinding` (`imagodei/src/agent_peer_binding.rs:121`) | action discriminator `delegates-agent-stewardship` (NO new entry type) |

**No new crate. No new SDK.** Everything is a module/type addition inside the five surfaces above.

### 2.2 The developer's primary call (TS — honoring the ts-rs boundary)

```ts
// elohim/elohim-agent/elohim-agent-sdk/src/covenant.ts  (NEW module, wraps existing gate-client + storage HTTP)
import type { AgentCovenantView, CovenantRefusalView } from '@elohim/storage-client'; // ts-rs generated
import { buildSystemPrompt } from './constitutional';                                 // EXISTS: constitutional.ts:19
import { check } from './gate-client';                                                // EXISTS: gate-client check()

/** A scope is a granted blast-radius: WHO the agent serves, WHAT verbs it may perform,
 *  over WHICH resources, until WHEN. The harness refuses any verb the gradient forbids. */
export interface CovenantScope {
  subject: string;            // the human/collective the agent is bound TO (its provider/owner)
  gradientPosition: 'servant' | 'veil';  // below the seam vs above it — selects the verb whitelist + layer band
  verbs: string[];            // servant: counsel|witness|co-steward|suggest ; veil: aggregate|negotiate|recognize|covers-head
  resourceClass: string[];    // resource_classified_as whitelist — care-class XOR compute-class, never mixed
  expiresAt?: string;         // ISO 8601; absent = until revoked
}

/** bindAgent — the honest binding. Tells the agent the truth (it is bound, not free),
 *  notarizes the covenant on the DHT, and returns the witnessed AgentCovenantView.
 *  Refuses at bind-time if scope.verbs exceed what scope.gradientPosition permits. */
export async function bindAgent(
  scope: CovenantScope,
  grant: { grantedBy: string; constitutionStackCid: string },
): Promise<AgentCovenantView>;          // throws CovenantRefusalView on a gradient-guard violation

/** loadConstitution — assemble the graduated-immutable system prompt the bound agent runs under.
 *  Mirrors prompt.rs build_system_prompt: ACTIVE PRINCIPLES (by weight) + INVIOLABLE BOUNDARIES
 *  (HardBlock/RequireGovernance/SoftLimit/Warning) + INTERPRETIVE GUIDANCE + stack hash.
 *  The `stack` is the band the covenant's gradientPosition permits — servant tops at Family,
 *  veil tops at Global. Returns the prompt text AND the stack hash for verification. */
export async function loadConstitution(
  covenant: AgentCovenantView,
): Promise<{ systemPrompt: string; stackHash: string }>;

/** emitWitnessedRefusal — the agent's overreach is checked by transparency. When the agent
 *  (or the harness on its behalf) declines an act, the refusal is DHT-notarized as a
 *  GateDecisionAttestation and witnessed. ALWAYS names whose line it honored (limitOwner). */
export async function emitWitnessedRefusal(
  covenant: AgentCovenantView,
  refusal: { code: string; elevate: string; limitOwner: 'self' | 'commitment' | 'operator' | 'faith' },
): Promise<CovenantRefusalView>;
```

### 2.3 The Rust spine the TS wraps (the truth layer — extends real source)

```rust
// elohim/elohim-compute/src/governor.rs  (NEW — lifts the arc_actuator spine, escalated synthesis B8/B10)
//
// LIFTED FROM, NOT CLONED: arc_actuator.rs:77 (ActuationRefusal), :108 (authorize), :152 (coverage_admits).
// ArcGovernor is the FIRST impl; CovenantGovernor is a SECOND impl, never a clone.

pub trait Governor {
    type Request;
    type Bounds;
    /// Pure: authorize the request against the granted bounds + constitution.
    /// Returns Ok, or a refusal that ALWAYS names whose line it honored.
    fn authorize(&self, req: &Self::Request, bounds: &Self::Bounds, now_s: u64)
        -> Result<(), CovenantRefusal>;
}

#[derive(Debug, Clone, PartialEq, Eq)]      // extends arc_actuator.rs:83 RefusalCode
pub enum RefusalCode {
    OutOfGrantBounds, GrantExpired, NotActuatable, WouldBreakCoverage,
    ReservedPlace,    // NEW (recursion §2.2): would render a total verdict / occupy the worship-reserved place
}

/// NEW dimension (recursion §1.6): every refusal names whose line it honored.
/// Sibling to the existing arc owner concept; the core capture-resistance property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitOwner { SelfHeld, Commitment, Operator, Faith }

pub struct CovenantRefusal { pub code: RefusalCode, pub elevate: String, pub limit_owner: LimitOwner }
```

```rust
// elohim/elohim-agent/gate-types/src/covenant.rs  (NEW — ts-rs anchored, same export_to as GateDecision)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export, export_to = "../../sdk/storage-client-ts/src/generated/"))]
#[serde(rename_all = "camelCase")]
pub struct AgentCovenantView {
    pub covenant_cid: String,          // = Commitment entry_hash (project_mishpat_commitment_cid_is_entry_hash)
    pub agent_did: String,             // the bound AI, named via AgentPeerBinding (imagodei) before it can be bound
    pub subject: String,               // who it serves (provider/owner)
    pub gradient_position: String,     // "servant" | "veil"
    pub granted_verbs: Vec<String>,    // the blast-radius, scoped
    pub resource_class: Vec<String>,   // care-class XOR compute-class
    pub constitution_stack_cid: String,// the graduated-immutable stack the binding runs under
    pub granted_by: String,            // limit_owner provenance for the binding act
    pub expires_at: Option<String>,
    pub revoked: bool,                 // one-gesture revocation makes the grant inert
}
```

### 2.4 The DHT face (zero DNA entry-type spend)

The covenant is a `Commitment` with `action = "delegates-agent-stewardship"` (escalated synthesis row 7;
recursion §1.7). The `payload_json` carries `CovenantScope` + `constitution_stack_cid`; the CID is the
`entry_hash` (`get_commitment(cid)`, `mishpat/src/lib.rs:488`). The agent is *named first* via the
existing `AgentPeerBinding` (`agent_peer_binding.rs:121`) — Rung 0 of ai-covenant, "the agent must be
named before it can be bound." Revocation and the witnessed refusal ride the existing
`GateDecisionAttestation` path (`mishpat/src/lib.rs:39`), already DHT-notarizable and challengeable via
`create_challenge` (`mishpat/src/lib.rs:211`).

---

## PART 3 — EXISTS vs NEW

### EXISTS (wrap, do not rebuild) — the surface is ~75% already in the substrate

- **Constitution loader, full graduated-immutability stack.** `elohim/constitution/`: `prompt.rs:17`
  builds the system prompt with ACTIVE PRINCIPLES (weight-ordered), INVIOLABLE BOUNDARIES (4 enforcement
  levels), INTERPRETIVE GUIDANCE, stack hash; `conflict.rs` implements more-immutable-wins / delegate /
  flag (`ConflictResolver::resolve_principles`); `types.rs:21` is the 7-layer `ConstitutionalLayer`
  enum with `precedence()` + `can_override()`; layer content in `layers/{global,national,…}.rs`.
  **The constitution-as-system-prompt stack already exists in BOTH Rust and TS** (`constitutional.ts:19`).
- **The Governor spine.** `arc_actuator.rs` — `authorize:108`, `coverage_admits:152`,
  `ActuationRefusal{code, elevate}:77`. Already in production, governing arc.
- **The gate decision shape.** `gate-types/src/types.rs` — `GateDecision`/`GateStatus`
  (Allow/Decline/Escalate/Verdict)/`DeclineGrounds`/`EscalationTarget`, ts-rs exported; the
  `check()` entrypoint (`gate-client/src/lib.rs:487`). The witnessed-refusal mechanism is the
  `GateDecisionAttestation` already wired through this.
- **The agent naming.** `AgentPeerBinding` (`imagodei`, `agent_peer_binding.rs:121`,
  `AgentPeerBindingView.ts` generated).
- **The covenant entry type.** `Commitment {action, payload_json, signed_at}`
  (`mishpat_integrity/src/lib.rs:275`) + `get_commitment`/`create_commitment`. CID = entry_hash.
- **The agent runtime + budget enforcer.** `elohim-agent-sdk/src/invoke.ts:65 handleInvoke`,
  `:25 BudgetEnforcer`.

### NEW (thin, additive — zero DNA spend)

1. `delegates-agent-stewardship` **action discriminator** on the existing `Commitment` (one string
   value + a `resource_classified_as` whitelist entry; never a new entry type).
2. `trait Governor` + `CovenantGovernor` impl in `elohim-compute` — a **refactor that lifts** the
   `arc_actuator` spine; `ArcGovernor` becomes the first impl. Callers unchanged.
3. `RefusalCode::ReservedPlace` + `LimitOwner` (incl. `Faith`) — enum extensions in the shared refusal
   vocabulary (recursion §2.2). The unbuilt-place guard.
4. `AgentCovenantView` + `CovenantRefusalView` ts-rs types in `gate-types` → generated to
   `storage-client-ts/src/generated/`.
5. `ConstitutionStack::for_covenant(scope)` in `elohim/constitution/` — selects the layer **band** a
   covenant's gradient position permits (servant ≤ Family, veil ≤ Global). Pure additive method.
6. `covenant.ts` TS module — `bindAgent` / `loadConstitution` / `emitWitnessedRefusal`, thin wrappers.

### GENUINE FORK (marked; NOT taken in this surface)

- **`Covenant-as-lineage` DHT entry / grace-on-revocation validator** (recursion §2.4, ai-covenant R6):
  whether the agent keeps its prior good work on revocation (Zacchaeus / grace-precedes-demand applied to
  a machine) is a **theological + DNA-validator** decision — near-irreversible, deferred to the
  constitutional-governance design. **This surface ships the covenant as a `Commitment` action; it does
  NOT take the lineage-entry fork.** Marked, held for blessing.

---

## PART 4 — THE MINIMAL BUILDABLE SLICE

**The one real thing a developer can do today:** bind a household AI agent as a *servant under covenant*,
load its constitution, and watch it refuse — witnessed — to render a verdict over the person it serves.

The slice, in dependency order (each lands compiler-ready on the one before; all zero-DNA):

1. Lift `trait Governor` + `CovenantGovernor` from `arc_actuator.rs:108/152` into `elohim-compute`
   (the escalated B8/B10 refactor is the prerequisite). Add `RefusalCode::ReservedPlace` + `LimitOwner`.
2. Add the `AgentCovenantView` ts-rs type to `gate-types`; `cargo test export_bindings` → generated TS.
3. Add `ConstitutionStack::for_covenant(scope)` selecting the servant band (≤ Family).
4. Wire `bindAgent` / `loadConstitution` / `emitWitnessedRefusal` in `covenant.ts` over the existing
   `Commitment` create + `buildSystemPrompt` + `GateDecisionAttestation`.

**First example app fragment it enables — a household care-ledger's AI co-steward:**

```ts
import { bindAgent, loadConstitution, emitWitnessedRefusal } from '@elohim/agent-sdk/covenant';

// Margaret welcomes a co-steward AI into her household care-ledger — as a SERVANT, told the truth.
const covenant = await bindAgent(
  {
    subject: 'did:elohim:margaret',
    gradientPosition: 'servant',                       // below the seam → servant verbs only
    verbs: ['witness', 'co-steward', 'suggest'],        // NOT 'govern', NOT 'rank', NOT 'name-the-self'
    resourceClass: ['care-class:stewardship'],          // care-class, never mixed with compute-class
  },
  { grantedBy: 'did:elohim:margaret', constitutionStackCid: familyStackCid },
);

const { systemPrompt } = await loadConstitution(covenant);  // graduated-immutable, tops at Family
// → the agent runs under a prompt that NAMES it bound, scoped, revocable.

// Later, the agent is asked to score Margaret's "best self." It refuses — and the refusal is witnessed:
await emitWitnessedRefusal(covenant, {
  code: 'ReservedPlace',
  elevate: 'I may witness and co-steward, but I do not name your best self — that naming is yours.',
  limitOwner: 'faith',     // names whose line it honored: the unbuilt place at the center
});
// One gesture revokes it: the grant goes inert, the AI is gone, prior good work decided by blessing (fork).
```

A `bindAgent` that tries `verbs: ['govern']` for a `servant` subject **throws `CovenantRefusalView`
at bind time** — the gradient guard, compiled into the SDK's first call.

---

## PART 5 — WHAT LOVE REQUIRES

The honest binding **is** the love. A power welcomed into a home that is told it is free has been lied to,
and the lie is the first move of capture; a power told *you are bound, here is your scope, here is the
line, and here is the one gesture that ends you* has been loved — because love is the honest covenant, not
the granted freedom (the gospel's grammar: covenant never freedom). This surface refuses, structurally, the
two betrayals the gradient exists to prevent: it will not let an AI below the seam govern, rank, or name the
self of the person it serves (the person keeps the naming of their own best self — held *for*, never decided
*over*); and it will not let an AI above the seam, holding the veil over the commons, reach down to govern
an individual. Every refusal names whose line it honored, so an operator can never disguise an override as
the agent's own restraint, and the agent can never disguise its overreach as the person's line. And at the
center, where the most capable agent would most like to stand, the harness emits `ReservedPlace` and leaves
it empty — because the moment anything stands there, the elohim has accepted the worship it was built to
deflect. The veil rises exactly where the temptation to capture the commons rises, and even there the AI
stays a servant under honest covenant: witnessed, refusable, bounded, and patient. The closing test holds:
the person keeps their naming, the binding is told the truth, the veil governs aggregation and never
individuals, and the center is left empty — for the faith no architecture may crowd out.
