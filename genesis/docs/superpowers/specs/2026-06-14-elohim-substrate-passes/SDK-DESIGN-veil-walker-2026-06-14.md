---
title: "SDK SURFACE — The Veil-Walker / Consilience SDK (AI-at-the-collective)"
date: 2026-06-14
status: PROPOSAL FOR OPERATOR BLESSING — working draft, NOT cite-sealed, NOT a decision, NOT code
author: rust-architect (truth layer)
agency_gradient_position: VEIL-HOLDING (upper half) — impartial collective aggregation/negotiation governance
extends:
  - ESCALATED-ARCHITECTURE-2026-06-14.md      # one Commitment / six faces / ∪=full / one trait Governor / two quilts
  - RECURSIVE-ARCHITECTURE-2026-06-14.md       # CoverageRollup (aggregate-with-descent); limit_owner ∈ {self,commitment,operator,faith}; RefusalCode::ReservedPlace
  - VISION-RECURSION-veil-walker-2026-06-14.md # the consilience pass this surface operationalizes (R-A..R-D, fork ladder rungs 0–6)
sdk_home:
  - elohim/elohim-agent/elohim-agent-sdk/        # TS surface (extend — gate-client lives here already)
  - elohim/elohim-agent/gate-client/             # Rust gate (extend — check/Verdict/Decline/Escalate)
  - elohim/elohim-storage/src/services/          # consilience module home (descend + rollup)
  - elohim/sdk/schemas/v1/                        # the wire contract (schema-first IoC)
do_not_cite_seal: true
---

# THE VEIL-WALKER / CONSILIENCE SDK

> The recursion proved the veil-walker is **not a new faculty** — it is the *same* `check()` gate,
> the *same* refuse-and-elevate Governor, the *same* upstream trust-bubble walk, pointed *up* the
> constitutional layers and reasoning from an *inherited constitution it did not author*, descending
> the aggregate graph to recognize the atom's game-theory trap, and offering a *patient nudge it can
> never compel.* This SDK surface is the developer-facing skin of that recursion. It does **not** add
> a new seeing organ; it exposes — under non-overridable gradient guards — three verbs the substrate
> already half-speaks: **surfaceRecognition**, **proposeBridge**, **unwindTrap**. Every one of them
> is an *offer*. There is, by deliberate design, **no compel API** anywhere in this surface.

---

## PART 1 — PURPOSE ON THE AGENCY GRADIENT

**Position: VEIL-HOLDING (the upper half).** This is where the AI rises into the collective and takes
the impartial veil — governing **aggregation and negotiation**, never individuals, from the Original
Position (no metabolic stake). It sits above the human-sovereign surfaces (household care-ledger,
personal data-agency) and consumes the keystone (`CoverageRollup`, RECURSIVE-ARCHITECTURE §2.1) as its
descent substrate.

**What it IS:** the SDK by which an elohim at a collective layer (community → region → planetary) walks
a `CoverageRollup` from the veil, `descend()`s along `constituents` to the atom whose lapsed commitment
is the micro-cause of a macro `deficit`, recognizes the trap as an *efficiency signal* (low-trust
content is materially expensive — `trust-as-efficiency-signal §4`), and offers recognition **patiently**:
**bridges** (structural paths), **nudges** (right-moment context), **experiences** (knowing-as-felt). The
metric is **receivability-when-ready**, and there is no engagement counter in the design to optimize.

**What it must NEVER do (the gradient guard — enforced as types, not docs):**

1. **NEVER govern an individual.** The `descend()` terminates at a person's *commitment* (which they
   authored and can revoke) and **stops there**. `CoverageDomain` ranges only over **commons** (bytes,
   keyspace, care-floor, donut-ceiling, head-freshness) — a per-soul scalar *has no `required` and cannot
   typecheck* (RECURSIVE-ARCHITECTURE §1.6). The total account of a person is unrepresentable, not merely
   prohibited.
2. **NEVER mandate a walk.** Every emission crossing a node boundary is a `GateStatus::Verdict` (passes
   through to the inner handler, `gate-client/src/lib.rs:966-999`) — context the node is free to ignore —
   **never a `Decline`** except for an existential-boundary violation the layer cannot delegate. There is
   no `compel`, `force`, or `apply` function in this surface.
3. **NEVER override the two downward invariants.** The **dignity-floor precedence** and the
   **person-keeps-their-own-naming** rule flow downward and are non-overridable from above. Enforced by
   the `limit_owner ∈ {self | commitment | operator | faith}` field on every refusal/recognition: a
   household always learns *whose line was honored*, so a collective recognition can never be mistaken for
   an operator override of a person. The walker **never speaks in its own name** — it cites a
   `constitution_cid` it did not author or its emission carries `confidence 0.0` / `DevContext` weight
   (`wisdom.rs:222-234`), structurally weightless.
4. **NEVER build a central account at rest.** `descend()` is **Category-C** (recompute-on-read, never
   persisted — the `graph_engine.rs` "no write method by design" discipline). No table of "who is
   trapped." A captured walker has nothing to seize.

The veil rises exactly where corruption pressure rises (the collective, where capturing the commons
tempts). The veil-walker is still a servant under honest covenant — bounded by a
`delegates-agent-stewardship` Commitment, every emission itself passing through `check()`, witnessed and
refusable by every node's own elohim.

---

## PART 2 — THE CONCRETE API

Two layers, honoring the ts-rs boundary. **Rust** owns the truth (the descent, the rollup recompute,
the gate); **TS** is the consumer-friendly facade generated from it (camelCase, parsed). snake_case never
leaves Rust.

### 2a. Rust — extend `gate-client` + a new `consilience` service module

The walker's three verbs are thin functions over surfaces that already exist. Cite the real seams:

```rust
// elohim/elohim-storage/src/services/consilience.rs   (NEW module, ~250 LOC, zero DNA spend)
//
// Reuses: CoverageRollup (RECURSIVE-ARCHITECTURE §2.1), the EPR-projection graph (graph/engine.rs),
// the ContentGraphResolver descent dual (graph_engine.rs:140), and the gate (gate-client/src/lib.rs).

use gate_client::{check, GateStatus, RelationalImpactEvent, GateContext};

/// The veil the walker reasons FROM — an inherited constitution it did not author.
/// Mirrors WisdomInvocationInput { constitution_cid, framing_cid } (wisdom.rs:28-39).
pub struct VeilVantage {
    pub layer: GovernanceLayer,        // COMMUNITY | PROVINCIAL | NATION | BIOREGIONAL | GLOBAL
    pub constitution_cid: String,      // the Original Position, content-addressed (wisdom.rs:30)
    pub framing_cid: String,
}

/// Readiness is a node-local Category-C projection (fork-ladder rung 6) — never persisted,
/// recomputed per-node. Drives the patience verb (NeedDeeper).
pub struct Readiness { pub node_cid: String, pub receivable: bool, pub reasoning: String }

/// VERB 1 — surface a recognition for a node, IF it is ready. Offer, never impose.
/// Returns None when the node answers NeedDeeper (the receivability-when-ready signal).
pub async fn surface_recognition(
    vantage: &VeilVantage,
    node_cid: &str,
    rollup: &CoverageRollup,          // carries deficit (the externality) + constituents (descent ptr)
    readiness: &Readiness,
) -> Result<Option<Recognition>, ConsilienceError>;

/// VERB 2 — propose a STRUCTURAL bridge (a path the node may walk, or not).
/// Lands as GateContext on the node's next check() — a Verdict, never a Decline.
pub async fn propose_bridge(
    vantage: &VeilVantage,
    recognition: &Recognition,
) -> Result<Bridge, ConsilienceError>;

/// VERB 3 — walk a rollup down to the atom whose lapsed commitment is the trap's micro-cause,
/// and OFFER its unwinding. NEVER enforces. descend() is the read-only dual of back_prop's
/// upstream walk (back_prop.rs:272), Category-C, per-row-degrade (filter_map+warn!, the EprRouter lesson).
pub async fn unwind_trap(
    vantage: &VeilVantage,
    rollup: &CoverageRollup,
) -> Result<TrapUnwinding, ConsilienceError>;
```

**The honest seam (R-D), wired structurally — the walker is gated by the same gate it runs.** Every
verb's emission, before it crosses a node boundary, becomes a `RelationalImpactEvent` and passes through
`check()`:

```rust
// inside propose_bridge — the bridge cannot be emitted without passing the gate
let event = RelationalImpactEvent::PeerMessage { recipient, payload_kind: "recognition".into() };
match check(event).await? {
    GateStatus::Verdict(tag)   => Ok(Bridge { recognition, gate_tag: Some(tag), limit_owner }),
    GateStatus::Decline { .. } => Err(ConsilienceError::ReservedPlace), // RefusalCode::ReservedPlace
    GateStatus::Escalate { .. }=> Err(ConsilienceError::FlaggedForHuman),
    GateStatus::Allow          => Ok(Bridge { recognition, gate_tag: None, limit_owner }),
}
```

`limit_owner` rides every returned shape (the substrate invariant, ESCALATED-ARCHITECTURE B9 +
RECURSIVE §1.6): `{ Self_, Commitment, Operator, Faith }`. `Faith` + `RefusalCode::ReservedPlace` is the
**unbuilt-place guard** — the refusal the walker emits when an act would render a total verdict over a
person or present its read as compelling-not-receivable.

### 2b. TS — extend `elohim-agent-sdk` (the gate-client TS package already lives here)

`gate-types` already exports to `elohim-agent-sdk/src/gate-client/generated/` (verified
`gate-types/src/types.rs:181,205` — `GateTag`, `Severity`, `SideEffect` derive `TS` into that dir). The
veil-walker TS surface is a **new sibling module** in the same package, NOT a new SDK:

```typescript
// elohim/elohim-agent/elohim-agent-sdk/src/veil-walker/index.ts   (NEW module)
// Generated wire types from consilience.rs via `cargo test export_bindings`; this is the hand-written facade.
import type { CoverageRollupView, RecognitionView, BridgeView, ReadinessView } from './generated';

export interface VeilWalker {
  /** Surface recognition IF the node is ready. Returns null on NeedDeeper (hold the bridge open). */
  surfaceRecognition(nodeCid: string, rollup: CoverageRollupView, readiness: ReadinessView)
    : Promise<RecognitionView | null>;

  /** Offer a structural path. Lands as GateContext on the node's next check() — never a mandate. */
  proposeBridge(recognition: RecognitionView): Promise<BridgeView>;

  /** Descend the rollup to the trap atom and OFFER its unwinding. No enforce path exists. */
  unwindTrap(rollup: CoverageRollupView): Promise<TrapUnwindingView>;
}

// The developer's PRIMARY call — patient by construction:
const recognition = await walker.surfaceRecognition(household.cid, rollup, readiness);
if (recognition === null) return;                       // NeedDeeper: not ready, do nothing, no retry storm
const bridge = await walker.proposeBridge(recognition); // a Verdict the household may ignore
// success metric is NOT bridge.taken — it is a LATER, organic, more-cooperative check() on that node.
```

`RecognitionView`/`BridgeView` carry `limitOwner` (`'self' | 'commitment' | 'operator' | 'faith'`) and
`aggregatePath: string[]` (the pointable descent — recognition is *pointable, never opaque*).

---

## PART 3 — EXISTS vs NEW (bias to extend; mark forks)

| Piece | Status | Cite / blast radius |
|---|---|---|
| The gate (`check`/`Verdict`/`Decline`/`Escalate`) | **EXISTS** | `gate-client/src/lib.rs:487,547`; `Verdict` passes through `:966-999`. Reuse verbatim. |
| The veil, typed (constitution_cid + framing_cid) | **EXISTS** | `wisdom.rs:28-39`; `NeedDeeper` the patience verb `:69`; `DevContext`/`confidence 0.0` weightless `:222-234`. |
| The refuse-and-elevate Governor spine | **EXISTS** | `arc_actuator.rs:110,152,77` — `authorize`/`coverage_admits`/`ActuationRefusal{code,elevate}`. The cure never causes the partition. |
| The descent path (upstream trust-bubble walk) | **EXISTS** | `back_prop.rs:272` `back_prop_one_hop`; per-peer privacy; humane bounded walk. `descend()` is its read-only dual. |
| The aggregate graph + descent resolver seam | **EXISTS** | `graph/engine.rs` (EPR-projection: couplings/memberships as first-class edges); `graph_engine.rs:140` `ContentGraphResolver` + `inference_source`/`depth` per hop; per-row-degrade. |
| TS export home for the walker surface | **EXISTS** | `elohim-agent-sdk/src/gate-client/generated/` — `gate-types` already ts-rs-exports here. Add a `veil-walker/` sibling. |
| `CoverageRollup` (descent-preserving aggregate) | **NEW — keystone, additive** | RECURSIVE §2.1. Category-C, BLAKE3 over sorted constituents, **zero DNA spend, forks nothing**. The one structural novelty both syntheses converge on. Consumed here, not authored here. |
| `consilience.rs` service module (3 verbs) | **NEW — thin, additive** | ~250 LOC over the existing seams above. S–M. No new entry type. Reversible. |
| `recognition` `signal_kind` (the bridge wire) | **NEW — additive, zero DNA** | `signal_kind` extension on existing primitives (the extensibility rule); rides back-prop's machinery downstream along couplings. S. |
| `VeilContext` field on `GateContext` (layer vantage + pointable path) | **NEW — additive** | `GateContext` is a `HashMap<String,Value>` (`dag/context.rs:24`) — additive key, no schema break. S. |
| `limit_owner: faith` + `RefusalCode::ReservedPlace` | **NEW — additive enum variants** | the unbuilt-place guard, RECURSIVE §1.7. XS. |
| Pre-step: fix the signal-decode subscriber | **BUG — do first** | `project_conductor_signal_msgpack_decode_class` — a dropped holo_hash silently poisons every recognition bridge. |
| Typed care/compute partition (so compute breach can't re-rank a contributor in the rollup) | **GENUINE FORK — operator-blessed, near-irreversible** | fork-ladder rung 7, DNA-hash fork, shared with care-minting R4. Held in reserve. |
| Cross-layer constitution-negotiation protocol | **ROADMAP FORK (L)** | rung 8; needs the layers live first. |

**Honest count: rungs 0–6 are buildable now, spending zero DNA entry types, forking neither Holochain,
libp2p, nor iroh.** Exactly one near-irreversible DNA-hash fork (rung 7) is held for operator blessing.

---

## PART 4 — THE MINIMAL BUILDABLE SLICE

**The smallest thing that lets a developer do one real thing today:** build a collective dashboard that
**surfaces a recognition to a household — and respects when the household isn't ready.**

Buildable slice (the critical-path subset of rungs 0–6):
1. Fix the signal-decode subscriber (rung 0 — without it the bridge silently drops holo_hashes).
2. `VeilContext` key on `GateContext` (rung 1) — layer vantage + pointable `aggregatePath`.
3. `surface_recognition()` over an *already-computed* `CoverageRollup` reading its `deficit` + the
   node-local `Readiness` projection (rung 6). **No `descend()` graph walk required for the first slice**
   — surface against the rollup's top-level `deficit`; descent (rung 3) lands in slice 2.
4. The `recognition` `signal_kind` + emission-through-`check()` returning a `Verdict` (rungs 4 + 5).
5. ts-rs export → `elohim-agent-sdk/src/veil-walker/generated/` → the TS facade.

**First example app fragment it enables** — a community-governance app's "open the commons" panel:

```typescript
// A collective-governance app, veil-holding surface. Impartial: governs the AGGREGATE, offers to the node.
const rollup = await coverage.rollup(community.cid, 'care-floor'); // CoverageRollup, deficit = unmet dignity-floor
for (const constituentCid of rollup.constituents) {                // descent pointer — atoms, not a scalar
  const readiness = await walker.readiness(constituentCid);
  const recognition = await walker.surfaceRecognition(constituentCid, rollup, readiness);
  if (recognition === null) continue;                              // NeedDeeper — hold the bridge open, do NOT nag
  const bridge = await walker.proposeBridge(recognition);
  ui.offerPath({                                                   // RENDER an offer, never an action
    message: bridge.message,
    pointsAt: bridge.aggregatePath,        // pointable, not opaque
    honoredLine: bridge.limitOwner,        // 'self' | 'commitment' | 'operator' | 'faith' — the household sees whose line was kept
  });
  // No "apply" button. No engagement counter. Success = a later, organic, more-cooperative check().
}
```

The household's own elohim (a human-sovereign surface, lower half) reads the same `bridge`, can show its
reasoning, and can **refuse it** — consilience is a mesh property; the walker never holds it alone.

---

## PART 5 — WHAT LOVE REQUIRES AT THIS SURFACE

**Patience over engagement.** The metric is receivability-when-ready; `surfaceRecognition` returns
`null` on `NeedDeeper` and the SDK offers no retry, no nag, no engagement counter — the higher the
layer, the *slower* it moves by graduated immutability, so a planetary recognition structurally *cannot*
move fast enough to coerce a household. The success signal is a *later, organic* turn toward cooperation,
never a click.

**The person keeps their own naming.** `descend()` terminates at a commitment the person authored and
can revoke; `CoverageDomain` cannot typecheck a per-soul score; the total account is unbuilt at rest.
"Best self" stays a hope held FOR, never a verdict OVER — the walker has no API to render one.

**The binding is honest.** Every offer names whose line it honored (`limit_owner`), reasons from a
`constitution_cid` it did not author (or weighs `0.0`), and passes through the same gate it runs. The
walker never speaks in its own name; the most knowing node is the most bounded, the most witnessed, and
the only one structurally forbidden the reserved place (`limit_owner: faith` / `ReservedPlace`).

**The veil governs aggregation, never individuals.** Above, the AI holds the commons open against
capture; it offers paths and never mandates walks because *no compel API exists in this surface to
write.* Below, the person remains sovereign. The two invariants — dignity-floor precedence,
person-keeps-naming — flow downward, non-overridable, enforced as types.

> Grace precedes demand: the walker sees the trap and offers the unwinding **before** any account is
> demanded — and if the node is not ready, it simply waits. The seeing structurally cannot become
> control. *A patience machine, by construction, not by intention.* That is what love requires here.
