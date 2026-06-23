---
title: "Vision-Gap STUB — O3 Runtime Limit-Respect Governor (observe→govern feedback loop)"
id: vision-gap-limit-governor-stub
date: 2026-06-14
status: stub-greenlight-to-expand
objective: O3 (governance that helps individuals respect THEIR OWN limits + coordinate in virtuous feedback loops)
requires_env: household-nodes
companions:
  - genesis/docs/superpowers/specs/2026-06-09-per-substrate-limitarian-governor-design.md (the DNA wall — built since)
  - genesis/docs/superpowers/specs/2026-06-09-coupling-delay-observed-governed-primitive-design.md (loop-delay honesty)
  - genesis/docs/superpowers/specs/2026-06-09-wisdom-layer-floor-ceiling-judgment-culminating-design.md (floor↔ceiling seam)
cross_plan_edges:
  - CONSUMES P-ACTUATION's ActuationRefusal/RefusalCode (arc_actuator.rs) — do NOT re-own
  - CONSUMES D-DIAGNOSTIC read-model (stability-surface) — felt-surface composes over it
note: >
  Working draft. NOT cite-sealed. file:line are draft pointers. This is a SCOPING
  greenlight memo per the operator's standing rule (self-answer + recommend; the
  value/governance core needs a blessing before expansion).
---

# O3 — Runtime Limit-Respect Governor

## 1. Objective + the felt promise (one paragraph)

A person on the substrate should be able to **set a limit for themselves** — "I don't
want to host more than 40 GB of others' photos," "I don't want to spend more than two
hours a day in qahal deliberation," "stop pulling content once my laptop battery is
under 20%" — and have the protocol *help them keep it* through a gentle, observed
feedback loop rather than a hard wall thrown by someone else. The felt promise: **the
system notices you approaching your own line, tells you before you cross it, eases off
on your behalf, and shows you it did** — and when it cannot honor the limit it says so,
attributing the refusal to a real constraint (capacity, a commitment you made), never
silently overriding you. This is the inverse of the operator-actuatable self-healing arc
(which actuates *node capacity* for resilience): this governor actuates *a human's own
declared limit* for dignity. The two share one cybernetic shape — detect → recover →
verify → elevate — pointed at two different sovereign owners.

## 2. Vision-vs-substrate GAP

The protocol *promises* (resilience epic; O3) a "fabric of governance that helps
individuals respect their own limits in virtuous feedback loops." Today the code is a
**fence, not a loop**, and the fence belongs to the operator/community, not the person:

- **There is exactly one reject-at-write limit wall**: `validate_ratifies_limit_gradient`
  in `mishpat/zomes/mishpat/src/commitments.rs:399-466` (now BUILT — it was vapor when
  the wisdom spec was written; that spec's "Terminator 1 returns zero .rs hits" is STALE).
  It clamps a *community-ratified concentration setpoint* (`C_target`, `k_max`) against
  DNA walls (`:369-373`). It governs the **commons' inequality**, not **a person's own
  declared ceiling**. A refusal here is the *operator's/community's* limit on concentration.
- **The consumer is a global demurrage curve, not a personal observe→govern loop.** The
  limitarian governor (per-substrate-limitarian-governor-design §3) presses *every* agent
  by how concentrated the *commons* is. Nothing reads "this **individual** said their
  personal limit is X; observe their state; ease them toward it; tell them."
- **`apply_decay` was HTTP-poke-only** (coupling-delay-design §1, `api/token.rs:114-128`) —
  no cadence, so even the commons loop had unbounded sense→act lag. The personal loop has
  no cadence *and no sensor and no setpoint*.
- **Refusal is operator-shaped, not person-shaped.** `arc_actuator.rs` owns
  `ActuationRefusal`/`RefusalCode` (P-ACTUATION, dataplane ledger:290) — but it refuses to
  actuate a *node arc* on a coverage-floor breach. There is **no path where a refusal is
  legible to a *person* as "I declined because honoring this would breach a limit YOU set
  or a commitment YOU made"** (the wisdom-spec floor↔ceiling `Settlement{appeal_path}`
  exists as a type, `elohim_gate.rs:224`, but is not wired to a personal-limit setpoint).

**The missing thing in one sentence:** a person cannot today *declare a limit about
themselves* as a first-class governed object, and there is no runtime loop that observes
their state against it, eases on their behalf, signals before the line, and attributes
any refusal back to a constraint they own.

## 3. The MISSING BRIDGE / primitive (concrete)

Three pieces, all riding existing molds — **zero new DHT entry types**:

1. **A self-declared limit as a `Commitment` action `respects-self-limit`.** A new
   *coordinator-side* action arm in `commitments.rs` (DNA-hash-NEUTRAL, exactly like
   `sets-authority-arc` at `:481` and `replicates-content` — the integrity zome is
   untouched, the validator hot-swaps via `update_coordinators`). Payload: `{ subject:
   <self-agent-cid>, signal: "storage"|"attention"|"compute"|"energy"|..., bound: {kind:
   "ceiling"|"floor"|"rate", value, unit}, on_approach: {threshold_pct, action:
   "signal"|"ease"|"pause"}, valid_from, valid_until }`. The author and the subject MUST be
   the same agent (validator invariant — you can only set a limit *about yourself*; setting
   one about someone else is a different, governance-gated act). This is **an instantiation
   of the existing `Mishpat::Commitment`/`delegates-compute` family** (per
   `project_rea_compute_commitment_primitive`), not a new primitive — it is the
   *self-reflexive* member of that family: where `delegates-compute` is "I grant you bounded
   authority," `respects-self-limit` is "I bind *myself* to a bounded behavior."

2. **A runtime governor service** `self_limit_governor.rs` in `elohim-storage/src/services/`,
   shaped as a **near-clone of `arc_actuator.rs`'s spine** (authorize → coverage/state
   gate → plan → apply), but the four arms are the cybernetic loop:
   - **detect** — read the subject's current state for the signal (storage bytes held;
     attention-seconds; battery) against the `respects-self-limit` bound;
   - **recover (ease)** — apply a *gentle, person-owned* easing (throttle inbound
     replication, defer a pull, surface a "want to wrap up?" nudge) — NEVER a confiscation;
     the dignity-floor reuse from the limitarian governor (`token_decay_service.rs:164`
     `.max(dignity_floor)`) is the precedent that easing never presses below subsistence;
   - **verify** — measure the loop delay `τ_loop` *in-controller* (coupling-delay-design §4,
     bypass the projection) and confirm the state moved toward the bound;
   - **elevate (signal)** — at `threshold_pct` raise an **algedonic** `FeedbackSignal`
     (`SignalKind` exists, `kind.rs:25`; the band-edge-approach signal from coupling-delay
     §5(ii) is the exact shape) routed *to the person*, BEFORE the line, not after.

3. **A felt-surface refusal contract.** When honoring the limit conflicts with a
   commitment the person already made (e.g. a `replicates-dwelling` they signed), the
   governor emits a **person-attributed `ActuationRefusal`** — CONSUMING P-ACTUATION's
   `RefusalCode` enum (do NOT re-own it; add a `RefusalCode::SelfLimitConflict` variant via
   a cross-plan edge request), so the person sees "*I eased off as far as your active
   stewardship commitment allows; honoring the rest would breach a promise you made to hub:B
   — here's the appeal/renegotiate path.*" The refusal is the *person's* constraint made
   legible, never the operator's.

**The load-bearing distinction (human's limit vs operator's limit):** the
`respects-self-limit` Commitment's `subject == author` invariant is what makes the refused
actuation *the human's own limit*. The operator's limit (the limitarian `C_target` wall,
the arc coverage-floor) stays exactly where it is and is never confused with this. A
refusal carries a **`limit_owner: "self"|"commitment"|"operator"`** discriminant so the
felt surface can always say *whose* line was hit. This is the single most important design
honesty in the stub.

## 4. p2p-design-gate ANSWERS (all four)

1. **Class:** **A (notarized)** for the `respects-self-limit` declaration — it is a
   Commitment (a person binding themselves is a witnessable promise, and a refusal must be
   able to cite it non-repudiably). The **governor's loop state** (`τ_loop`, current
   reading, last ease) is **C (operational, node-local)** — pure control state on the
   subject's own node, NEVER projected as an observation (per coupling-delay-design §6
   `[adversarial-fix]`: loop delay is node-local C, not an A2 observation). The
   **approach/refusal signal** is **B2 (agent-scoped + attestation)** — a signed
   `FeedbackSignal` the person's node emits about itself.
2. **Does a DHT entry type already EXIST to ride?** **YES — `Mishpat::Commitment`** (no new
   entry type; Mishpat stays ~11/~100). New work is a *coordinator action arm*
   (`respects-self-limit`) + its validator, exactly the DNA-hash-neutral pattern
   `sets-authority-arc` established (`commitments.rs:481`). The B2 signal rides the existing
   `FeedbackSignal`/`SignalKind` mold.
3. **Identity:** **content-derived (CID = `entry_hash`)** — per
   `project_mishpat_commitment_cid_is_entry_hash`: the CID is the bounds-gate key, NEVER
   the `action_hash`. The governor's C-state keys on `(subject_cid, signal)` (node-local,
   not addressed). The B2 signal CID is over `(subject_cid, signal, signed_at)`.
4. **Coordinator fn / projecting signal:** `create_commitment` with `action:
   "respects-self-limit"` CREATES it (reusing the existing extern, `commitments.rs:31`);
   the `CommitmentCommitted` post-commit signal (already must be subscribed in storage —
   `project_mishpat_commitment_cid_is_entry_hash` 2a gap) PROJECTS it into the
   `mishpat_commitments` write-through cache, where the governor reads its active bound. The
   approach signal projects via the `ReconcileController` (the canonical signal-handler home)
   into a person-facing read view.

## 5. Existing substrate to build on (file:line) + what NOT to re-own

**Build on:**
- `mishpat/zomes/mishpat/src/commitments.rs:185` (action dispatch), `:481`
  (`sets-authority-arc` — the exact DNA-hash-neutral new-action template), `:399`
  (`validate_ratifies_limit_gradient` — the built reject-at-write wall pattern).
- `elohim-storage/src/services/arc_actuator.rs:110,152,177,312,345` (`authorize` →
  `coverage_admits` → `plan_actuation` → `compute_actuation` → `apply_arc_actuation`) — the
  proven actuation spine to clone for the governor's detect→ease→verify→elevate.
- `token_decay_service.rs:160-196` (the `apply_decay` Steps 4-6 + the `.max(dignity_floor)`
  clamp at `:164`) — easing must never press below dignity, reuse verbatim.
- `elohim_gate.rs:203,224` (`GateResult::{Pause, Settlement{appeal_path}}`) — the
  floor↔ceiling seam the refusal surface routes through.
- `FeedbackSignal`/`SignalKind` (`kind.rs:25`) + `ReconcileController` (the signal→projector
  home) — the approach/refusal signal carrier.

**Do NOT re-own (cross-plan edges):**
- **`ActuationRefusal`/`RefusalCode`** — owned by **P-ACTUATION** (`arc_actuator.rs`,
  dataplane ledger:290, c66355fd0). This stub CONSUMES it and *requests* a
  `RefusalCode::SelfLimitConflict` + a `limit_owner` discriminant via a cross-plan edge —
  never forks the enum.
- **The stability/diagnostic read-model** — owned by **D-DIAGNOSTIC**
  (`stability-surface-read-model-plan`, vision-alignment ledger:122). The felt-surface
  composes the person-facing limit view OVER that read-model; it does not author a parallel
  one.
- **The libp2p/iroh transport** and **the limitarian `C_target` commons wall** — untouched;
  this is a *personal* limit layered beside the commons limit, not a change to it.

## 6. The FIRST a2o SCENARIO (story-first — the spec)

`genesis/a2o/features/qahal/self-limit-respect.feature` (proposed; qahal = governance pillar).

```gherkin
@requires:household-nodes
Feature: The protocol helps me respect a limit I set for myself
  As a person stewarding family content on my own node
  I want to set a limit on how much I host and be helped to keep it
  So that participating never quietly costs me more than I chose to give

  Background:
    Given Maria runs a household node with 100 GB free
    And Maria has declared a respects-self-limit commitment
      | signal  | bound          | on_approach            |
      | storage | ceiling: 40 GB | at 90%: signal then ease |

  Scenario: The loop signals me before I cross my own line
    Given Maria's node currently holds 35 GB of replicated family photos
    When new replication offers would push her toward 38 GB
    Then the governor raises an algedonic approach-signal to Maria at 90% (36 GB)
    And the signal is attributed to Maria's OWN limit, not the operator's
    And inbound replication eases (throttles) on Maria's behalf
    And Maria's node never silently exceeds 40 GB

  Scenario: A refusal names whose limit it is
    Given Maria also signed a replicates-dwelling commitment to hub:B
    And honoring her 40 GB ceiling would breach that stewardship promise
    When the governor cannot ease further without breaking the commitment
    Then it emits an ActuationRefusal with limit_owner = "commitment"
    And Maria sees "honoring the rest would breach a promise you made to hub:B"
    And Maria is offered a renegotiate/appeal path (not a silent override)

  Scenario: The loop closes and shows it closed
    When the governor eases inbound replication
    Then the measured loop delay tau_loop is bounded and recorded node-locally
    And Maria can see the system acted on her behalf and her holding settled under 40 GB
```

The first green slice is **Scenario 1 on household-nodes** (no shem): a self-limit
Commitment, a storage-signal governor tick that eases inbound replication, and a
person-attributed approach-signal. This couples O7 (the loop proves itself) to O3 (it
respects the person's own limit), O5 (the person controls the bound), and O1/O2 (it is the
grandma/family-photos scene from the felt side).

## 7. Effort (S/M/L) + risk + why it serves the objective

**Effort: M.** The spine exists (`arc_actuator.rs` is a near-template), the carrier exists
(Commitment + the DNA-hash-neutral action pattern is well-worn), the seam exists
(`GateResult`/`FeedbackSignal`). The genuinely new work: the `respects-self-limit`
validator (S — mirror `sets-authority-arc`), the `self_limit_governor.rs` four-arm loop
(M — the detect-sensor per signal is the real cost), the felt-surface refusal wiring (S,
mostly a cross-plan edge request + Angular composition over D-DIAGNOSTIC).

**Risk: MEDIUM, concentrated in two places.** (a) **The sensor per signal is value-laden**
— "what counts as a unit of attention/storage/compute the person is limiting" is the same
`x_i`-adapter value choice the limitarian spec flagged (§6, must be gated, not
adapter-author discretion). v1 ships the **storage-bytes-held** sensor only (objective,
filesystem-counted, the inventory-vs-bytes honesty applies — count bytes, not gossip).
(b) **The ease action must be genuinely gentle and reversible** — an over-eager governor
that pauses participation is itself a capture surface (the wisdom-spec's "a refused
actuation is the operator's limit" trap one level in). Mitigation: easing is **throttle/defer
only** in v1, never hard-pause; `on_approach.action: "pause"` is deferred until the person
can preview it.

**Why it serves O3:** it is the literal inversion of a fence into a loop, and the literal
inversion of operator-actuation into person-actuation. It is the one stub that makes "the
system helps you respect *your own* limit" a running feedback loop rather than a slogan,
and it does so by reusing the exact cybernetic spine O7's self-healing plane already proves
— turning the operator's resilience machine into the person's dignity machine without
building a second one.

## 8. OPEN QUESTIONS for the operator (decisions only you can make)

1. **Is `respects-self-limit` the right shape, or should a self-limit be agent-scoped (B)
   rather than notarized (A)?** Recommend **A (Commitment)** — a refusal must non-repudiably
   cite the limit it honored, and a person may *want* a household to witness "I chose to cap
   my hosting." But A means a self-limit is gossiped; if a person's limits should be private,
   it must be B (private source-chain) with the refusal citing only a local hash. **This is
   the privacy-vs-accountability tradeoff the wisdom spec named, applied to self-limits — and
   only you can set where it sits.**
2. **Which signals ship in v1's sensor set?** Recommend **storage-bytes-held alone**
   (objective, household-floor-testable, byte-counted not gossip-counted). Attention and
   energy sensors are richer O3 stories but carry the `x_i`-adapter value choice — defer
   until each adapter is gated like a wall.
3. **Default `on_approach.action` — `signal` only, or `signal`+`ease`?** Recommend
   **signal+ease (throttle/defer), never hard-pause in v1.** A governor that can pause
   participation is a capture surface; the person should preview pause before it can act.
4. **The `RefusalCode::SelfLimitConflict` + `limit_owner` discriminant — does P-ACTUATION
   accept the cross-plan edge,** or does the self-limit governor get its own refusal type?
   Recommend **accept the edge** (one refusal vocabulary, so the felt surface is uniform);
   this needs the P-ACTUATION owner's sign-off, not a solo decision.
5. **Does the self-limit governor share the limitarian governor's cadence/`ReconcileController`
   tick, or run its own?** Recommend **share the tick** (one cadence, the coupling-delay
   loop-delay honesty already lives there) — but confirm, since it couples two governors'
   timing.

---

> **GREENLIGHT-TO-EXPAND.** This is a scoping memo. The value/governance core —
> notarized-vs-private self-limits (Q1), the sensor value-choices (Q2), and whether the
> system may ease a person's participation at all (Q3) — needs your blessing before a full
> implementation plan. Self-answered recommendations are in §8; the design-doc artifact +
> go-ahead is the gate.
