---
title: "VISION DESIGN PASS — Self-Limits as Governance Contracts (the limitarian donut ceiling)"
id: vision-design-limit-governance
date: 2026-06-14
status: PROPOSAL (working draft — NOT cite-sealed)
escalates: D10 + D14 (SPRINT-KICKOFF-2026-06-14.md:193,197) and the O3 stub
  (plans/2026-06-14-vision-gap-limit-governor-stub.md) — from "ship a stub" to
  "the structural fork the vision requires"
north_star_clauses_carried:
  - governance contracts that set policies + enforce decisions
  - the donut's regenerative outer ceiling (care economy where value is minted)
  - fractal stewards & hubs (households to factories) that scale sensemaking
  - capture-resistant stasis against the real world's externalities + messiness
  - high-integrity DHT (the trust people build values on)
substrate_read:
  - mishpat/zomes/mishpat/src/commitments.rs (validate_ratifies_limit_gradient BUILT :399; validate_sets_authority_arc BUILT :481)
  - elohim-storage/src/services/arc_actuator.rs (the cybernetic detect→refuse→elevate spine, BUILT)
  - elohim-storage/src/services/concentration_service.rs + db/concentration_snapshots.rs (the limitarian measure, BUILT)
  - specs/2026-06-09-{per-substrate-limitarian-governor,coupling-delay,wisdom-layer-floor-ceiling}
  - p2p/feedback_signal.rs (SignalKind: NO algedonic/approach variant yet)
---

# Self-Limits as Governance Contracts — the limitarian donut ceiling

> **The escalation in one line:** the tactical stub (O3 / D10 / D14) proposes a
> *fourth* near-clone of the actuation spine (`self_limit_governor.rs`) pointed at a
> person. The vision does not ask for a fourth governor. It asks for **one governed
> quantity — the *self-limit* — to become a first-class member of the bounded-authority
> family that `delegates-compute`, `sets-authority-arc`, and `ratifies-limit-gradient`
> already are**, and for the cybernetic spine (`arc_actuator.rs`) to be **generalized
> once** into a `dyn Governor` over `(setpoint, sensor, actuator, owner)` so that the
> operator's resilience machine, the commons' limitarian machine, and the person's
> dignity machine are **the same machine pointed at three sovereign owners**. The fork
> we commit to is not new code volume — it is a **`limit_owner` discriminant raised to a
> substrate invariant** (alongside the existing care-class/compute-class isolation), and
> the **donut ceiling realized as a coverage-invariant over self-limits**, exactly as
> `arc_actuator`'s `∪arcs ≥ r_floor` realizes keyspace coverage.

---

## Part 1 — What the VISION REQUIRES here

The operator's north star names this gap precisely, in three of its clauses:

**(a) "governance contracts that set policies, enforce decisions."** A self-limit ("I
don't host more than 40 GB," "I don't spend more than 2 hours/day in deliberation") is a
*policy a person sets about themselves*. The vision requires it be a **contract** — a
witnessable, citable, revocable governance object — not a config row in one node's
SQLite. The difference is load-bearing: a refusal ("I declined to replicate further") is
only **legible** and **pro-social** if it can non-repudiably cite the limit it honored.
A private config can refuse; only a *contract* can refuse *accountably*. This is why D10
ranks the self-limit as **notarized (A)**, not agent-scoped (B): "the line a person draws
should be answerable, not private-only" (SPRINT-KICKOFF:193).

**(b) "build a donut-like commons — the trust-economy, the care-based economy stories
where value is minted."** Kate Raworth's doughnut is two rings: a **social floor** (no
one below the inner ring — dignity) and an **ecological ceiling** (no one's draw breaches
the outer ring — regeneration). The substrate **already has the inner ring**: the
limitarian governor's `dignity_floor` (`token_decay_service.rs:164` `.max(dignity_floor)`,
sufficientarian, decay-off-below). It **already has the tier shape of the outer ring**:
the `ratio_attestation` sum-to-100 (`commons_pct + dwelling_pct + collective_pct +
free_pct == 100`, `commitments.rs:692`) — the free/dwelling/collective/commons donut tiers
(qahal-epr-household-lattice-design:142). What is **missing is the ceiling as a
*regenerative* invariant**: a structure that says *the union of what stewards commit to
hold must cover the commons, and no single steward's draw may breach the outer ring* —
the limitarian `C_target` (commons concentration ceiling) and the personal self-limits
**composed into one coverage relation**. The vision requires the self-limits not as
isolated personal fences but as the *bricks of the outer ring*.

**(c) "the system can stay in stasis when actuating a capture-resistant state against the
real world, its externalities, and its messiness."** A self-limit governor that can *ease
a person's participation* is itself a capture surface (the stub names this, §7b: "an
over-eager governor that pauses participation is itself a capture surface"). The vision
requires the **floor↔ceiling seam** (wisdom spec): the *deterministic floor* (the
self-limit is honored, refusals are witnessed, easing never presses below dignity) holds
with no LLM and no network; the *elohim ceiling* (judgment about *how* to ease, *whether*
to nudge) is advisory and human-vetoable. Capture-resistance is **stasis**: the loop
self-corrects toward the person's declared line and *signals before* it ever actuates
near it — never silently overriding the human.

**(d) "fractal stewards and hubs (households to factories) that scale the sensemaking."**
The self-limit must compose: a *household* sets a collective self-limit; a *factory-scale
hub* sets its own; and these nest without a central authority — the same subsidiarity
lattice (`ConstitutionalLayer` Individual=1 … Global=7) the limitarian governor already
rides. The vision requires the self-limit to be **scope-polymorphic** (subject = an agent,
a household hub, a collective hub), so the donut ceiling is *fractal*.

**The synthesis the vision forces:** a self-limit is **not a new feature** — it is the
**self-reflexive member of the one bounded-authority primitive** (`project_rea_compute_commitment_primitive`).
Where `delegates-compute` is *"I grant you bounded authority,"* `respects-self-limit` is
*"I bind myself to a bounded behavior."* And the donut ceiling is **not a new economy** —
it is the **coverage-invariant relation** over those self-limits plus the commons
`C_target`, enforced the way `arc_actuator` already enforces keyspace coverage.

---

## Part 2 — Is the substrate CAPABLE? Dig to WHY (the exact layer)

**The headline finding that reshapes the whole pass: the tactical stub is written against
a STALE reading of the substrate.** The stub (and the wisdom spec it cites) say
"`validate_ratifies_limit_gradient` returns zero `.rs` hits — it is vapor / a citation
ring." **That is no longer true.** Reading the real source today:

| Substrate piece the vision needs | Stub/wisdom-spec assumption | ACTUAL state (read 2026-06-14) | file:line |
|---|---|---|---|
| The reject-at-write limit wall | "vapor, zero .rs hits" | **BUILT** — `validate_ratifies_limit_gradient` clamps `α∈[1,2]`, `C_target∈[0.05,0.30]`, `k_max∈[0.01,0.10]`, `base_rate`, `gamma`; loosening-witness check | `commitments.rs:399-466` |
| A second governed-quantity wall already proving the pattern generalizes | not noted | **BUILT** — `validate_sets_authority_arc` (arc factor `{0,1}`, coverage_floor>0) — the *second* governed-quantity action arm, same mold | `commitments.rs:481-538` |
| The cybernetic detect→refuse→elevate spine | "near-clone it for the person" | **BUILT** — `arc_actuator.rs`: `authorize → coverage_admits → plan_actuation → compute_actuation`, pure-core + impure-shell, refuse-and-elevate with `ActuationRefusal{code, elevate}` | `arc_actuator.rs:110,152,177,312` |
| The limitarian concentration measure (GE + top-share) | "greenfield `elohim-core::measure`" | **BUILT** — `concentration_service.rs` + `db/concentration_snapshots.rs` exist | `services/concentration_service.rs` |
| The donut tier shape (free/dwelling/collective/commons) | not connected to limits | **BUILT** as `ratio_attestation` sum-to-100, validated reject-at-write | `commitments.rs:672-697` |
| The dignity floor (donut inner ring) | reuse verbatim | **BUILT** — `.max(dignity_floor)` | `token_decay_service.rs:164` |

**So the substrate is FAR MORE capable than the tactical layer believes.** Three of the
four hardest pieces (the wall, the spine, the measure) are already shipping and tested.
The stub's "Effort: M, build a near-clone" is over-scoped in code and *under-scoped in
architecture*: it would ship a **fourth parallel governor** (arc, limitarian-demurrage,
self-limit, and the self-healing arm) each re-implementing detect/refuse/elevate, when the
real move is to **lift the spine once**.

**Now — where IS the genuine limit (the fork candidate)?** Three exact layers:

**Limit α (the real one): `arc_actuator.rs`'s spine is a CONCRETE function family, not a
trait.** `authorize`/`coverage_admits`/`plan_actuation` are hard-typed to `ArcGrantBounds`
/ `CoverageSnapshot` / `target_factor: u32`. There is no `trait Governor` abstraction. To
make the self-limit governor *the same machine* (not a fourth clone) we must generalize the
spine into a trait — `trait Governor { type Setpoint; type Reading; type Actuation; fn
detect(...) -> Reading; fn plan(setpoint, reading, snapshot) -> Result<Actuation,
ActuationRefusal>; fn elevate(...); }` — with `ArcGovernor` as the first impl (a refactor,
not a rewrite). **This is a fork of our own architecture, cheap, and it is the spine of the
whole proposal.** (Substrate-floor discipline: the trait is deterministic; the *judgment*
about whether to nudge stays in the elohim ceiling.)

**Limit β: `SignalKind` has no algedonic / band-edge-approach variant.** `feedback_signal.rs:49`
enumerates `{Squelch, Correction, Retraction, Quarantine, Vouch}` — all *epistemic/governance*
signals. The self-limit loop's **"signal before the line"** (coupling-delay §5(ii)
band-edge-approach algedonic) has **no carrier**. Per the substrate's own rule
([[project_signal_kind_extensible_protocol_class]]), a new social/sensing move is a
`signal_kind` *extension*, never a new entry type — so this is **a `SIGNAL_KINDS` whitelist
entry + a `SignalKind::Approach` variant**, the exact Vouch-precedent path. **Buildable now,
not a fork.** (`SIGNAL_KINDS` whitelist: `content_store_integrity/src/feedback_signal.rs:42`.)

**Limit γ (the deep one — the donut ceiling): there is no coverage-invariant over
self-limits.** `arc_actuator::coverage_admits` enforces `observed_n - 1 ≥ r_floor` for
*keyspace* coverage. The donut **outer ceiling** needs the dual: *the union of stewards'
self-limits + the commons `C_target` must cover the commons' regenerative need, AND no
single steward's actual draw breaches the outer ring.* The substrate has the **tier shape**
(`ratio_attestation`) and the **concentration measure** (`concentration_service`) but **no
relation that composes self-limits into a regenerative ceiling**. This is the genuine new
primitive — and it is **a projection/relation, not a DHT entry type** (it is *recomputed*
from the notarized self-limit Commitments + the `concentration_snapshot`, Category-C, the
way `arc_actuator`'s coverage gate is recomputed from the live peer count). **A roadmap
commitment, not a fork of Holochain.**

**Verdict: the substrate is CAPABLE at the floor; the gaps are (α) our own un-abstracted
spine [cheap fork of our code], (β) a missing signal_kind [buildable now], (γ) the
donut-ceiling coverage relation [new Category-C relation, roadmap].** None requires forking
Holochain, kitsune2, or libp2p. The vision lands on the substrate-floor we already have.

---

## Part 3 — The PATH / PIVOT / FORK LADDER (cheapest → deepest)

### Rung 0 — Settle the refusal-legibility vocabulary as a SUBSTRATE INVARIANT (D14)
**Cost: XS (one enum + one discriminant). Blast radius: P-ACTUATION's `RefusalCode` enum
(one owner) + every future governor.** Add `RefusalCode::SelfLimitConflict` and a
`limit_owner: Self | Commitment | Operator` field to `ActuationRefusal` (`arc_actuator.rs:77`).
**Raise `limit_owner` to a documented substrate invariant** — peer to the existing
care-class/compute-class isolation rule. A refusal must *always* name whose line it hit; an
operator-veto smell must *never* leak into a person's lever. **Unlocks:** the felt promise
"I declined because *you* set this / *you* promised hub:B / the *operator's* coverage floor"
— uniform across S-LIMIT, S-AGENCY, S-SPINE. This is D14, blessed once.

### Rung 1 — `respects-self-limit` as a coordinator action arm (DNA-hash-neutral)
**Cost: S (mirror `sets-authority-arc`). Blast radius: `commitments.rs` validator dispatch
(additive arm, coordinate with P-ACTUATION owner) — integrity zome UNTOUCHED, hot-swaps via
`update_coordinators`.** Payload `{subject, signal, bound:{kind,value,unit}, on_approach:
{threshold_pct, action}, valid_from, valid_until}`, with the **invariant `subject == author`**
(you may only bind *yourself*; binding another is a different, governance-gated act). Reuses
`Mishpat::Commitment` (no new entry type; Mishpat stays ~11/~100), CID = `entry_hash`
([[project_mishpat_commitment_cid_is_entry_hash]]). **Unlocks:** the self-limit is now a
notarized, citable, revocable contract (D10 = A). A refusal can cite it.

### Rung 2 — `SignalKind::Approach` (the algedonic "signal before the line")
**Cost: S (Vouch-precedent extension). Blast radius: `SIGNAL_KINDS` whitelist + the
`SignalKind` enum + schema.** The band-edge-approach signal from coupling-delay §5(ii),
routed *to the person* at `threshold_pct`, BEFORE the line. **Unlocks:** the felt "the
system noticed you approaching your own line and told you" — without it the loop can only
refuse *after*, never warn *before*.

### Rung 3 — Generalize the spine: `trait Governor` (the fork of OUR architecture)
**Cost: M (refactor `arc_actuator` behind a trait; `ArcGovernor` = first impl). Blast
radius: `arc_actuator.rs` internal shape; callers unchanged (the impure shell stays).**
`trait Governor { type Setpoint; type Reading; type Actuation; fn detect; fn plan ->
Result<Actuation, ActuationRefusal>; fn elevate; }`. Then `SelfLimitGovernor` is a *second
impl*, not a clone — detect=read held-bytes, plan=ease (throttle/defer, `.max(dignity_floor)`),
elevate=`SignalKind::Approach`. **Unlocks:** one machine, three owners (operator-arc /
commons-demurrage / person-self-limit). The vision's "one substrate, many instantiations"
made literal in the control plane, mirroring how `Commitment` is one entry, many actions.

### Rung 4 — The donut-ceiling coverage relation (the deep new primitive)
**Cost: L (new Category-C relation + the regenerative-ceiling math). Blast radius: a new
read-side projection composing self-limit Commitments + `concentration_snapshot`; touches no
DHT entry type, no transport.** The dual of `coverage_admits`: given the union of stewards'
`respects-self-limit` ceilings and the commons `C_target`, compute whether the commons is
**covered** (inner ring: no one below dignity) and **bounded** (outer ring: no draw breaches
regeneration), and **refuse-and-elevate** a self-limit ratification that would open a
regenerative gap (e.g. *everyone* capping hosting at 1 GB → commons can't be held →
elevate "the commons needs N GB of committed coverage; current pledges sum to M < N").
**Unlocks:** the donut as a *governance contract*, not a metaphor — the outer ceiling
enforced through the same refuse-and-elevate the keyspace floor uses. **This is the genuine
roadmap commitment.**

### Rung 5 (PIVOT, on-mission, deferred) — fractal/factory-scale self-limits + cross-hub coverage
**Cost: XL. Blast radius: subsidiarity lattice + cross-collective GE decomposition (the
limitarian spec's deferred §2 federated property).** Scope-polymorphic subjects (household
hub, collective hub, factory hub) and the **federated coverage invariant** across hubs — the
"households to factories" clause. Depends on F-COHERENCE/F-BOOTSTRAP cross-edge truth
(VISION-GAP-PLANS:2b). **Unlocks:** the donut composes fractally; a factory's regenerative
ceiling nests inside a bioregion's. Park behind Rung 4 + the federation work.

---

## Part 4 — The recommended ESCALATION (defended) + what it COMMITS US TO

**Recommend: ship Rungs 0–2 now (this sprint), commit to Rung 3 as the architectural fork,
schedule Rung 4 as the roadmap primitive, park Rung 5.** Defended:

- **Rungs 0–2 are buildable-now and unlock the felt promise** — a notarized self-limit, a
  refusal that names its owner, and a before-the-line signal. They are the honest v1 the
  operator's standing rule asks for, and they require *no fork* — the wall, the Commitment
  mold, and the signal_kind extension path all exist.
- **Rung 3 is the escalation the vision REQUIRES and the tactical stub MISSES.** The stub
  would ship a fourth clone; the vision's "one substrate, three instantiations" demands we
  lift the spine. This is a **fork of our own architecture** (a trait over `arc_actuator`),
  not of Holochain — cheap, reversible, and the single most vision-aligned move. **We commit
  to: `trait Governor` becomes the canonical control-plane shape**; new governors are impls,
  never clones (the same discipline as `signal_kind`-not-new-entry-type).
- **Rung 4 is the genuine new primitive and the deepest vision payload** — the donut ceiling
  as a coverage-invariant. **We commit to: a roadmap item — the regenerative-ceiling coverage
  relation — sibling to the limitarian-governor v1 follow-ons.** It is Category-C (recomputed,
  no DHT spend), so it is reversible and does not touch the precious entry budget.

**What this explicitly does NOT commit us to (capture-resistance discipline):** it does NOT
commit us to a governor that can *hard-pause* a person's participation (Rung 3's
`SelfLimitGovernor` ships throttle/defer-only; `on_approach.action: "pause"` is deferred
until the person can preview it — easing that pauses is the operator-veto smell the
`limit_owner` invariant exists to forbid). It does NOT commit us to an AI that *sets* the
cadence (coupling-delay §5: the human is sovereign over the clock; the elohim signals before
it steers to a band edge). And it does NOT extend the vapor-no-more validator into a runtime-τ
wall — the delay-margin check is advisory + a live `SignalKind::Approach`, never a DNA wall.

**One escalation flag for the operator (a genuine fork decision, not mine to make):** Rung 4's
regenerative ceiling needs a *coverage target* for the commons (how much committed self-limit
coverage the commons regeneration requires). That target is the **outer-ring equivalent of the
`dignity_floor`** — a value-laden DNA-wall-class number, unargued today. Per the limitarian
spec's Decision 2 ("no value-neutral width"), **this is operator-set, once, by whoever writes
core.** I recommend shipping the *shape* (the coverage relation + refuse-and-elevate) with the
target marked `TBD-operator`, exactly as the limitarian walls shipped shape-first.

---

## Part 5 — COUPLING: story + value + governance as one whole

The proposal is coherent only because it **couples the three planes through one object** —
the self-limit Commitment — exactly as the north star's "coupled story+value+governance" demands:

**STORY (the felt surface).** Maria sets "I host at most 40 GB of family photos." The system
*notices her approaching* (Rung 2 `SignalKind::Approach` at 36 GB), *eases on her behalf*
(Rung 3 throttle), *shows her it acted* (the felt read-model composes over D-DIAGNOSTIC), and
when it cannot ease further without breaking her `replicates-dwelling` promise to hub:B, it
*names whose line it is* (Rung 0 `limit_owner: Commitment`) and offers renegotiation, never a
silent override. The story is the grandma-vertical's scenario 3–4 (S-SPINE) seen from the
limit side: *participating never quietly costs more than she chose to give.*

**VALUE (the donut, where value is minted).** The self-limit is a **care commitment** — and
care-class stays categorically isolated from compute-class ([[project_compute_commitments_bounded]]):
a self-limit *easing* is a dignity act, never a compute-breach debit, and a compute breach never
debits a care self-limit. The donut's **inner ring is the existing `dignity_floor`** (no easing
presses below subsistence); the **outer ring is Rung 4's coverage ceiling** (no steward's draw
breaches regeneration, and the union of pledges must cover the commons). Value is *minted* in
the regenerative band between the rings — the `ratio_attestation` tiers (free→dwelling→collective→commons)
are the bricks, and a self-limit that *pledges* commons coverage is the act of minting care into
the commons. The trust-economy is the accumulated, witnessable record of stewards honoring their
own declared lines.

**GOVERNANCE (the contract that sets policy + enforces decisions, capture-resistantly).** The
self-limit is a *governance contract*: notarized (D10=A), citable, revocable
(`revokes-commitment`). It rides the subsidiarity lattice — a person sets their own; a household
hub sets a collective one; the commons `C_target` is ratified M-of-N (`ratifies-limit-gradient`).
Enforcement is the **floor↔ceiling seam**: the deterministic floor (honor the limit, witness the
refusal, ease never below dignity) holds with no LLM, no network — the **capture-resistant
stasis** the vision names. The elohim ceiling (judgment about *how* gently to nudge) is advisory
and human-vetoable (`Pause{confirm_token}`). And the `limit_owner` invariant is the structural
guarantee that **the system actuating on a person's behalf can never be mistaken for an operator
overriding them** — the single most important capture-resistance property of the whole design.

**The fractal close (households to factories).** Because the self-limit is the self-reflexive
member of the *one* bounded-authority primitive, and the donut ceiling is a coverage *relation*
over those commitments, the structure composes without a center: a factory hub's regenerative
ceiling nests inside a bioregion's the way a household's nests inside a collective's — fractal
stewardship, scaling sensemaking, with no apex that could capture it (the DNA wall + human veto +
public record are the only terminators, never a higher council — wisdom spec). **One machine
(`trait Governor`), one contract (`Commitment`), one invariant (`limit_owner`), three sovereign
owners, two donut rings — staying in stasis against the world's messiness because the person's
line, the commons' ceiling, and the operator's floor are each enforced as the same refuse-and-
elevate, and each one names itself.**

---

## Appendix — the honest substrate-currency correction this pass surfaced

The tactical stub (`plans/2026-06-14-vision-gap-limit-governor-stub.md`) and the wisdom spec it
cites both assert `validate_ratifies_limit_gradient` is "vapor / zero .rs hits." **This is STALE
as of 2026-06-14** — the validator is BUILT (`commitments.rs:399-466`), as is `validate_sets_authority_arc`
(`:481`) and the `arc_actuator` spine. Any expansion of this proposal MUST re-read the real source,
not the stub's stale citations. Surface-migration note for the resilience-epic Part IX honesty
matrix ([[feedback_living_doc_honesty_matrix_maintenance]]): the "Terminator 1 is unbuilt" line in
`wisdom-layer-floor-ceiling-judgment-culminating-design.md:217` is now false and should be
corrected via cite tooling when this proposal is blessed.
