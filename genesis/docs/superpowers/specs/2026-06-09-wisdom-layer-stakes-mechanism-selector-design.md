---
title: "Wisdom Layer — Stakes→Mechanism Selector (floor-meets-ceiling judgment)"
id: wisdom-layer-stakes-mechanism-selector-design
date: 2026-06-09
status: design
cluster: wisdom (culminating)
predecessors:
  - 2026-06-09-per-substrate-limitarian-governor-design.md
  - 2026-06-09-cluster2-sacredness-surface-firewall-anti-capture-design.md
  - 2026-06-09-cluster3-substrate-signal-migration-governance-signal-flow-design.md
requires_env: household-nodes
---

# Wisdom Layer — Stakes→Mechanism Selector

> This is the culminating spec Clusters #1–3 route their parked decisions to. It does **not**
> invent a parallel judgment substrate. It generalizes the one already in-tree — `ElohimGate`'s
> `InferenceTier::classify`, the `constitution` crate's `PromptAssembler` JSON contract, and the
> `eae` subsidiarity/escalation/precedent machinery — and tops it out with a stakes dimension, an
> anonymity-mode output, and a notarized accountability record.

This design area covers ONE of the wisdom cluster's surfaces: **DESIGN AREA 1 — the
stakes→mechanism selector**. Sibling areas (the JudgmentCall record schema, the eae↔gate wiring,
apex accountability-of-process) are referenced where they bound this one but specified at their own
altitude.

---

## Area

How the system maps a decision/poll/governance-act to an **accountability+anonymity MODE** on a
floor–ceiling gradient, where:

- **FLOOR** = the minimum guarantee that always holds, deterministically, even with no LLM, no
  network, no human in the loop. (k≥5 anonymity for low-stakes; a vote is at least counted; an
  inviolable boundary is never crossed.)
- **CEILING** = full attribution / maximum accountability — fully-attributed roll-call, precedent
  citation, super-quorum, human-ratified, appeal-bearing.

The operator's spectrum is `where's lunch` (low-stakes → anonymous, zero standing impact, no REA
claim) ↔ constitutional limit-gradient ratification (high-stakes → fully-attributed, touches
standing/reach/REA, appeal-able). The selector picks a **mode per decision** from
`stakes = f(reach affected, standing weight, REA-claims at risk)`.

The hard problem, named by the operator: **the stakes-assessment is itself a judgment call.** This
spec bounds it (a deterministic floor classification + a judgment ceiling) so it does not regress
infinitely — and shows where the recursion **terminates**.

---

## Proposal (concrete; the floor and the ceiling explicitly)

Today `InferenceTier::classify(mutation, ctx)` (`elohim-storage/src/services/elohim_gate.rs:79`)
emits exactly one axis: ceremony tier `{None, Light, Full, Constitutional}`, keyed on
`MutationType × composite_trust`. The proposal adds a **second axis (AnonymityMode)** and a
**second input (an explicit Stakes vector)** to the *same* selector, and makes the selection itself
emit a notarized record.

### The selector becomes a two-axis function

```rust
// elohim-storage/src/services/elohim_gate.rs (generalized)
pub struct Selection {
    pub tier: InferenceTier,          // existing: ceremony amount
    pub mode: AnonymityMode,          // NEW: accountability/attribution mode
    pub stakes_class: StakesClass,    // NEW: the deterministic floor class
    pub witnessed_by: JudgmentCallRef,// NEW: notarized record of THIS selection
}

pub enum AnonymityMode {
    /// k-anon aggregate; no per-voter identity reaches any read path.
    /// Backed by aggregator.rs k≥5 Suppress floor.
    AnonymousAggregate,
    /// HMAC-treated stable pseudonym (cluster #2 mold); linkable within a poll,
    /// not across polls; no issuer_cid in clear.
    Pseudonymous,
    /// Per-voter issuer_cid + vote_value auditable-by-design (cluster #3 §3.3),
    /// gated by a read-path guard. The high-stakes default.
    Attributed,
    /// Private source-chain entry + selective attestation; the voter's record
    /// is private but a B2 attestation witnesses participation.
    PrivateWithAttestation,
}

pub enum StakesClass {        // the DETERMINISTIC floor (no judgment)
    Trivial,                  // no reach/standing/REA touched
    Standing,                 // standing weight at risk (non-zero)
    Reach,                    // reach affected (always elevated — reach_earning)
    ReaBound,                 // a REA Commitment's bounds are at risk
    Constitutional,           // a limit-gradient / wall / apex ratification
}
```

### The two halves

- **FLOOR (`StakesClass::classify`)** — a pure, deterministic function over the three grounded
  primitives: *does this act touch reach? does it carry standing weight? does it put a REA
  Commitment's bounds at risk?* This is a `match`/threshold, not a judgment. It can never under-shoot
  the safe mode: a `Constitutional`-class act is *always* `Attributed`, full ceremony, appeal-bearing,
  regardless of what the ceiling later refines. The floor ships on household-nodes with no LLM.
- **CEILING (the elohim judgment call)** — *within* the band the floor opens, the LLM-mediated
  judgment (or, in v1, a human) refines: a `Standing`-class act the floor would route to `Light`
  ceremony might be escalated to `Full` if the deliberation context is novel; an `AnonymousAggregate`
  poll the floor permits might be recommended-Attributed if the elohim's reasoning flags
  accountability need. **The ceiling can only tighten toward more accountability, never loosen below
  the floor.** (This is the same monotonicity `standing.rs:56 with_lift` and `reach_earning` never-erode
  already enforce on their own quantities — the selector inherits it.)

The selector returns `Selection`, which feeds the existing `GateResult`
(`elohim_gate.rs:203`): `PassThrough | Enriched | Pause{confirm_token} | Settlement{appeal_path}`.
No new result enum — the mode rides as a field on the reasoning and the appeal path is the existing one.

---

## Floor (deterministic guarantee) vs Ceiling (stakes-sensitive judgment) — the seam

The seam is **already typed** in `GateResult` and `EnforcementLevel` — this spec only names where
each half lives and forbids the ceiling from faking the floor's job.

| | FLOOR (deterministic) | CEILING (judgment) |
|---|---|---|
| **Computed by** | `StakesClass::classify` + `AnonymityMode::floor_for(class)` + `stack.check_boundaries` (`constitution/src/stack.rs:266`) | LLM via `PromptAssembler::build_reasoning_prompt` (`prompt.rs:77`), or a human via `Pause` |
| **Guarantee** | k≥5 anonymity for `Trivial`; vote counted; inviolable boundary (`EnforcementLevel::HardBlock`) never crossed; `Constitutional`-class → `Attributed`+full ceremony | refines *upward within the floor's band*; recommends approve/deny/escalate/defer — never actuates a contested case |
| **Where it stops / starts** | floor stops at `EnforcementLevel::RequireGovernance` (`types.rs:226`) — the explicit "deterministic code stops, judgment begins" enum value | ceiling starts there; output is `recommendation`, not command (`prompt.rs:108`) |
| **Fail mode** | fail-safe-to-coherent: if the ceiling is absent/errors, `GateResult::PassThrough` at the floor's already-safe mode (`evaluate` returns PassThrough on no-router/error, `elohim_gate.rs` Sprint-1 path) | never fail-open: a contested case with no ceiling and no human → stays at the floor's conservative mode, escalates to `Pause`/human |

**The load-bearing invariant:** the ceiling is *advisory to ratification, never an actuator of a
`RequireGovernance` case* (Cluster #1 §Decision 3: "`C(D)` is advisory input to the M-of-N
proposal"). The interpretive guidance the prompt already injects encodes this:
*"Flag ambiguous cases for human deliberation rather than deciding"* (`prompt.rs:62`). The v1 stub
**must not** let `MockEngine`/`InferenceRouter`'s hardcoded `Light` (`inference_router.rs:23`)
silently auto-satisfy a `Constitutional`-class selection — see §v1 STUB.

---

## Stakes dimensioning (reach × standing × REA-claims set the stakes)

Stakes ≠ trust. Today the selector keys on `composite_trust` (a 6-signal *trustworthiness* blend,
`elohim_gate.rs:171`) — that answers "how much do we trust this actor," not "how much is at risk in
this act." The wisdom layer adds the second, orthogonal vector from the grounded primitives:

```
stakes(act) = f(
    reach_affected,    // epr_kind::Reach — earned-at-authoring, never erodes (reach_earning.rs:4).
                       //   An act that changes who-sees-what is Reach-class, ALWAYS Constitutional
                       //   ceremony today (elohim_gate.rs:133). Reach affected → high stakes.
    standing_weight,   // StandingScore {Floor..Trusted} (standing.rs:29), never-demote (with_lift:56).
                       //   An act that consumes/risks standing carries weight ∝ the score put at risk.
    rea_claims_at_risk // Mishpat Commitment bounds (commitments.rs). An act that could breach a
                       //   Commitment's `bounds` (→ FeedbackSignal, revocable) is ReaBound-class.
)
```

Worked endpoints of the operator's spectrum:

| Act | reach_affected | standing_weight | rea_at_risk | → StakesClass | → AnonymityMode (floor) |
|---|---|---|---|---|---|
| "where's lunch?" poll | none | none | none | `Trivial` | `AnonymousAggregate` (k≥5) |
| ranked-choice on a discussion thread | none | low | none | `Standing` | `Pseudonymous` |
| dispute filing | none | high (filer's standing on the line) | none | `Standing`→elevated | `Attributed` |
| reach change (who sees content) | yes | — | — | `Reach` | `Attributed` |
| ratify a `LimitGradientConfig` (wall width) | bioregional | — | the Commitment itself | `Constitutional` | `Attributed` + super-quorum |

**`Reach`/`ReaBound`/`Constitutional` classes are floor-deterministic-elevated** — they map to
high-accountability modes *without* a judgment call, because reach and REA-bounds are the protocol's
own ledgered consequences. Only the `Trivial`↔`Standing` band is where the ceiling's discretion
actually lives — which is the right place for it (most decisions, lowest cost, reversible).

---

## Accountability model (per component; to whom; and where it TERMINATES)

The operator's recursive question — *what accountability for the components of the judgment
process?* — is answered per component, and the recursion is **bounded** (not infinite) because each
component terminates at one of three non-recursive anchors.

| Component | Accountable for | To whom | How (mechanism) | Terminates at |
|---|---|---|---|---|
| **Floor classification** (`StakesClass::classify`) | picking a non-under-shooting class | everyone (deterministic, inspectable) | pure function + `stack_hash` of the active constitution stamped on the record | **the DNA validator** — `validate_ratifies_limit_gradient` rejects-at-write any class that would breach a wall (Cluster #1 §5.2). The wall is the one non-capturable element. |
| **Mode selection** (the elohim judgment) | choosing anonymity mode + ceremony tier | the deliberating cohort + future cases | a notarized **JudgmentCall** record (A outcome + B private deliberation + B2 attested mode-selection) | **the PrecedentStore** (`precedent.rs`) — cited, weighted, searchable case-law; a bad selection becomes a citable counter-precedent |
| **The LLM verdict** | its `recommendation`/`confidence` | the human ratifier | `build_reasoning_prompt` JSON contract (`prompt.rs:108`); `confidence` + `precedents` exposed | **`EnforcementLevel::RequireGovernance`** — the LLM can never auto-satisfy it; a human ratifies (`Pause{confirm_token}` / M-of-N tally) |
| **The ratification** | the actual decision | the DNA wall + appellants | M-of-N `propose→vote→tally` (`governance_action.rs:260`) within walls; `appeal_path` (`GateResult::Settlement`) | **the DNA wall** (can't ratify outside it) + **human sovereignty** (appeal) |

**Where it terminates — the three anchors (none recurse):**

1. **The DNA clamp wall** — the *only* structurally non-capturable element across Clusters #1–3.
   `can_override` is pure precedence (`types.rs:46`), so altitude is not immunity; the wall, set once
   by whoever writes core and enforced reject-at-write, is the floor under everything including the
   apex. The recursion cannot ascend past it because it is not a layer — it is the validator.
2. **The precedent ledger** — a notarized, citation-weighted audit trail. A judgment is accountable
   *over time*: a captured/wrong selection becomes a counter-precedent that lowers the weight of its
   line of reasoning. This is the "accountability-of-process" that **replaces altitude-immunity** for
   the apex (Cluster #1 §5.4): the apex's authority is bounded not by being un-overridable but by
   being the *most-attributed, most-precedent-exposed, super-quorum* mode.
3. **Human sovereignty** — `EnforcementLevel::RequireGovernance` + `DecisionType::DeferToHuman`
   (`eae/types.rs:184`) + `Pause{confirm_token}` + `appeal_path`. A human can always halt and appeal;
   the LLM never closes a contested case. The escalation ladder itself terminates:
   `next_layer(Global) → None` (`eae/governance/escalation.rs:173`) — "can't escalate beyond global,"
   and at Global the only move left is DeferToHuman.

**The recursion is bounded by construction:** the *mode-selection* is witnessed by a JudgmentCall
(component 2); the JudgmentCall's *correctness* is judged by precedent + DNA wall (anchors 1–2), NOT
by a meta-selector. We do not select-the-selection-of-the-selection. The buck stops at the wall and
the human — both non-recursive, both already in-tree as the one non-capturable element and the
sovereign override.

---

## Human feedback & representation (ubiquitous-intimate-scalable; where the human is sovereign)

The vision (`ubiquitous-wisdom-dissolves-chokepoint.md`) is wisdom at every node, with humans
retaining meaningful feedback. The selector serves that:

- **Ubiquitous** — the floor runs at every node with no LLM (household-nodes is the stable floor;
  `feedback_household_nodes_is_the_stable_floor`). Every node can classify stakes and pick a safe mode
  deterministically. No chokepoint.
- **Intimate** — most acts are `Trivial`/`Standing`: low-ceremony, anonymous-or-pseudonymous, zero
  standing impact. The lunch poll never summons the apex. Friction is proportional.
- **Scalable** — the LLM ceiling is the *same prompt contract* (`prompt.rs:77`) whether it runs on a
  laptop sidecar (`sidecar_engine.rs:197`, `ELOHIM_AGENT_URL`) or a hosted backend. The selector code
  is identical; only the engine swaps.
- **Sovereign** — three guaranteed human controls, all already typed:
  1. `Pause{confirm_token}` — the human-in-loop friction moment; a `Constitutional`-class selection
     *cannot* settle without a human confirm-token in v1.
  2. `appeal_path` on `Settlement` — every constitutional settlement is appealable.
  3. M-of-N ratification — wall-widths and apex acts are ratified by humans via
     `propose→vote→tally`, with the elohim's reasoning as advisory input only.

The honest stance: **the elohim proposes/ranks/interprets; humans (or the DNA wall) decide the
contested cases.** No fake automation crosses `RequireGovernance`.

---

## Entities/signals (p2p-gate)

Per the mandatory p2p-design-gate, the one new entity:

**`JudgmentCall`** — *not a new DHT entry type.* It is a **composite of three existing molds**, so
Lamad/Mishpat entry-type headroom is untouched:

1. **Classification / source-of-truth** — the *act* being judged is whatever it already is
   (a `governance-action` Content entry via `propose_governance_action` (`governance_action.rs:260`),
   or a Mishpat `Commitment`). The JudgmentCall does not duplicate it; it *references* its
   `entry_hash` (CID = entry_hash; `project_mishpat_commitment_cid_is_entry_hash`).
2. **Outcome (A — notarized, attributed per the selected mode)** — reuse `PrecedentOutcome`
   (`types.rs:293`: `Approved{conditions}|Denied{reason}|Escalated{to_layer}|Deferred{until,reason}`)
   notarized into the PrecedentStore. The outcome's *attribution* is exactly the selected
   `AnonymityMode`: `AnonymousAggregate` → no issuer; `Attributed` → `issuer_cid` in clear.
3. **Private deliberation (B — agent-scoped, private)** — the LLM reasoning trace (`ElohimReasoning`,
   currently the Sprint-2 placeholder `elohim_gate.rs:245`) stored as a `Visibility::Private`
   source-chain entry, disclosure-tier-marked. Granular reasoning is private; only the outcome is
   public per the mode.
4. **Mode-selection witness (B2 — agent-scoped with attestation)** — a `FeedbackSignal`/attestation
   recording *that a mode was selected and by which classification*, so the selection step is itself
   witnessable. This is the recursive-accountability hook made concrete.

**Identity** — JudgmentCall's identity is content-derived: `hash(act_cid + stack_hash + selector_version)`.
No slug, no UUID. The `stack_hash` (`prompt.rs:67`) binds the judgment to the exact constitution
version it was made under — currentness is auditable.

**Coordinator / signal** — the selector runs inside `ElohimGate::handle` (already wired,
`api/mod.rs` dispatch). On a `Constitutional`/`Reach`/`ReaBound` selection it emits a
`JudgmentCallRecorded` signal; the precedent projector consumes it (the recompute-on-signal pattern
`mechanism_selection.rs:7-12` already uses). The mode-by-stakes policy is **another key in the same
governed EPR** the mechanism ladder already lives in: the `pillar-projection` Manifest
(`mechanism_selection.rs:96`), ratified by the same M-of-N path, bounded by the same DNA wall. So the
policy is not a hardcoded constant — it is itself governed, the way the ladder already is.

---

## Resolves which parked decision(s)

- **Votes auditable-vs-private (Cluster #3 Decision 4)** — **RESOLVED as stakes-sensitive mode
  selection.** `AnonymityMode` is now a first-class selector output, not a global constant. Lunch
  (`Trivial`) → `AnonymousAggregate` (k≥5 floor); constitutional ratification (`Constitutional`) →
  `Attributed` + read-path guard. The auditable-vs-private fork is decided per-act by `StakesClass`,
  with the policy itself governed in the pillar-projection Manifest.
- **Wall widths (Cluster #1 Decision 2)** — **RESOLVED: sociocratically set within a deterministic
  DNA floor.** The width is a `LimitGradientConfig` Commitment ratified via `propose→vote→tally`; that
  ratification is a `Constitutional`-class act → `Attributed` + super-quorum + `Pause`/appeal. The
  *floor* (the wall) is DNA-enforced reject-at-write (`validate_ratifies_limit_gradient`); the *width
  within it* is sociocratic. There is no value-neutral width — so the elohim's reasoning supplies the
  value-premise→width derivation as **advisory input** to human ratifiers (never an actuator).
- **Apex non-accumulability (Cluster #1 Decision 1)** — **RESOLVED via accountability-of-process, not
  altitude.** We do *not* claim `Global=7` is structurally immune (it isn't; `can_override` is pure
  precedence). Instead the apex's mode is the strictly-most-accountable: `Attributed` + super-quorum +
  mandatory precedent citation + a deliberation window + a `loosening_acknowledged` flag (so loosening
  the commons is witnessed, distinct from tightening). The wall, not the apex, remains the real
  backstop — stated honestly per Cluster #1 §5.4.
- **k_max ratifiability (Cluster #1 Decision 1)** — **RESOLVED: stakes-bounded within a DNA floor.**
  A `k_max` change is `Constitutional`-class (it can weaponize demurrage / confiscate-in-one-tick),
  so it is forced to the apex mode above AND is reject-at-write clamped by the DNA wall to a bounded
  per-tick delta. The floor prevents the confiscate-all case structurally; the ceiling debates the
  width inside it.
- **k≥5 under-measurement (Cluster #1 §8.3 / Cluster #2)** — **RESOLVED: judgment compensates where
  the measure is structurally blind.** The k≥5 `Suppress` floor (`aggregator.rs:90`) under-counts
  concentration in small high-capture collectives (`GE_assembled ≤ GE_true`). The wisdom layer's
  answer: in a small cohort (participation < a manifest-set `small_cohort_threshold`), a
  `Standing`+-class act is escalated by the ceiling from `AnonymousAggregate` toward
  `PrivateWithAttestation` — the *fact of participation* is witnessed (B2 attestation) even though the
  *vote value* stays private, so capture in a 4-person cohort leaves an attested trail the k-anon
  measure couldn't see. The measure stays blind; the judgment compensates, and the compensation is
  itself a precedent-logged JudgmentCall.

---

## v1 STUB (what ships deterministic now; the correctly-shaped interface)

**Ships now, deterministic, on household-nodes (no LLM, no network required):**

- `StakesClass::classify` — pure function over reach/standing/REA primitives. Fully testable, no
  judgment.
- `AnonymityMode::floor_for(class)` — the deterministic floor mode per class. `Trivial`→
  `AnonymousAggregate` backed by the *existing* `aggregator.rs` k≥5 Suppress; `Constitutional`→
  `Attributed` backed by the *existing* `attestations` `issuer_cid` path.
- The `Selection` struct as a new field on the existing `GateResult` flow; the existing
  `Pause`/`Settlement`/`appeal_path` seam unchanged.
- `stack.check_boundaries` keyword floor (`stack.rs:266`) — inviolable boundaries enforced
  deterministically, as today.

**The correctly-shaped interface where judgment plugs in later (built, content stubbed):**

- The ceiling is `PromptAssembler::build_reasoning_prompt` → `InferenceEngine::evaluate`
  (`inference_engine.rs:99`). Today only `MockEngine` and the hardcoded-`Light` `InferenceRouter`
  (`inference_router.rs:23`) exist. **The swap-point is the `InferenceEngine` trait** — `SidecarEngine`
  (`sidecar_engine.rs:197`) is the live plug; `eae::decide_with_llm` (`mace/decider.rs:303`) is the
  real reference implementation.
- **The honest-stub rule (load-bearing):** the v1 stub MUST NOT let the hardcoded-`Light` router
  auto-satisfy a `Constitutional`/`Reach`/`ReaBound` selection. When the ceiling is absent on such a
  class, the gate degrades to `Pause{confirm_token}` (human-in-loop) — NOT `PassThrough`. PassThrough-
  on-no-router is the safe default *only* for `Trivial`/`Standing` classes. This is the difference
  between "the floor is safe without judgment" (true for low stakes) and "we faked judgment for a
  high-stakes case" (forbidden).
- `ElohimReasoning` (`elohim_gate.rs:245`, the Sprint-2 placeholder) is the shaped-but-thin carrier;
  v1 fills it with the floor classification + a `mock-principle` marker so the absence of real
  reasoning is *legible*, never disguised as a real verdict.

**Explicitly deferred (named, not faked):** the real LLM backend wiring + budget path
(`inference_engine.rs:34 BudgetExhausted`); the eae↔gate composition (eae is library-only today, not
on the storage gate path — Grounding #4 gap 1); the JudgmentCall notarization into a persisted
PrecedentStore (in-memory HashMap today, `precedent.rs:34`); sortition/Council substrate (narrative
only in `governance/epic.md`).

---

## Open questions

1. **Pseudonymous mode primitive** — `Pseudonymous` (HMAC-treated stable-within-poll) is proposed in
   Cluster #2 but has no code. Does it ship in v1's floor, or does the floor collapse
   `Standing`→`Attributed` until the HMAC treatment lands? (Recommend: collapse to `Attributed` in v1
   — over-attribution is the safe direction; under-attribution is not.)
2. **Who ratifies the mode-by-stakes policy itself?** It lives in the pillar-projection Manifest. Is
   changing it a `Constitutional`-class act (self-referential — the policy that decides classes
   decides its own change-class)? Recommend: yes, pin it `Constitutional` by definition to avoid a
   community quietly lowering its own accountability floor.
3. **`small_cohort_threshold` value** — the k≥5 compensation needs a number. It is a wall-width-class
   unargued value (Cluster #1 Decision 2 applies recursively here). Set sociocratically within a DNA
   floor of `>= k_threshold` (5)?
4. **eae vs ElohimGate canonicity** — two judgment substrates consume `constitution` but don't
   compose (Grounding #4 gap 1, #2 gap 6). The selector lives in ElohimGate (on the live path); does
   eae's subsidiarity/escalation become the *escalation router* the gate calls, or stay separate?
   This spec assumes ElohimGate is canonical for the *selector* and eae supplies the *termination
   machinery* (escalation ladder, DeferToHuman) — but the wiring is a sibling spec.

## Hardest unanswered tradeoff

**The floor classification is itself a value judgment masquerading as deterministic.** `StakesClass`
is "deterministic" only relative to a *prior choice* of which acts touch reach/standing/REA and how
much weight each carries — and that choice (the weights `W_*` in `elohim_gate.rs:162-167`, the
`stakes = f(...)` coefficients) is unargued in exactly the way wall-widths are. We have **pushed the
regress down, not eliminated it**: the selection-of-mode is witnessed by the floor; the floor's
*coefficients* are not witnessed by anything below them. The honest terminus is that the coefficients
are core-set (like the DNA wall) and changeable only by a `Constitutional`-class ratification — i.e.
the floor's own value-premises are governed at the apex mode, recursively. This is coherent but
**uncomfortable**: it means there *is* a value-laden floor under the "deterministic" floor, and its
only protection is that touching it is maximally accountable. Whether that is genuine
non-accumulability or just "capture is expensive here" is the same question Cluster #1 left open about
the apex — and the honest answer remains: **the DNA wall is the backstop; everything above it,
including the stakes coefficients, is defended by accountability-of-process, not by structural
immunity.** A truly value-neutral floor does not exist, and this spec does not pretend to supply one.
