---
title: "Requisite variety as the guidestar — the composition law for epr / epr-rea / eprfs / epr-meta"
id: requisite-variety-guidestar-epr-family-composition
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: >
  Ranks 1-3 of §5 land (floor-aware AlgedonicEvidence with a declared reach; proxy-declarer
  standing in the type; an evidence channel on ValidatorOutcome), AND §3's acceptance criterion
  is demonstrated by a runnable check rather than argued — two agents representing different
  domains negotiating a contract a third party can walk back. Until that check exists this stays
  Draft, because a law with no failable test is exactly the instrument-with-no-reader shape §6.2
  refuses. OR superseded by a fresh reader contesting §4's numbers (see §7 Q6).
created: 2026-08-12
domain: D2
topic: [requisite-variety, ashby, beer, meadows, vsm, epr, epr-rea, eprfs, epr-meta, reach, algedonic, agentic, limitarianism]
cites:
  - holonic-capacity-measure-convergence-pivot | Holonic capacity convergence | sha256:69dcba0c26d7cc65 | path: genesis/docs/superpowers/plans/2026-08-12-holonic-capacity-measure-convergence-pivot.md
  - measure-dynamics-confidence-ontology-design | Measure Dynamics + Confidence Ontology | sha256:52d601baa6117450 | path: genesis/docs/superpowers/specs/2026-08-11-measure-dynamics-confidence-ontology-design.md
  - reach-ontology-vocabulary-split-spec | Reach Ontology/Vocabulary Split | sha256:2a1ef52c1ced3c48 | path: genesis/docs/superpowers/specs/2026-07-22-reach-ontology-vocabulary-split-spec.md
  - epr-reachability-economics | 2026-05-29-epr-reachability-economics | sha256:19e359867f22af5a | path: genesis/docs/superpowers/specs/2026-05-29-epr-reachability-economics.md
  - genesis/research/elohim-as-viable-system-2026-06-04.md
  - genesis/research/beer-designing-freedom-elohim-critique-2026-06-04.md
  - genesis/research/meadows-systems-dynamics-cross-pollination-2026-08-11.md
  - genesis/data/timeline/backlog/commons-holonic-stewardship-backlog.md
  - genesis/data/timeline/backlog/measure-family-borrows-backlog.md
  - elohim/epr/src/measure.rs
  - elohim/epr/src/reach.rs
  - elohim/epr/src/verdict.rs
  - elohim/epr/src/algedonic.rs
  - elohim/epr-rea/src/scope.rs
  - elohim/epr-rea/src/model.rs
---

# Requisite variety as the guidestar

**Read this before designing anything in `elohim/epr`, `elohim/epr-rea`, or `elohim/eprfs`.**
It states the law those crates compose under, the acceptance criterion that replaces "does it
feel right", the measured state of the gap as of 2026-08-12, and — equally important — what
NOT to build.

This document is deliberately contestable. It is the starting point for a fresh reader to
confirm or dispute, not settled canon. Every number in §4 was measured on 2026-08-12; every
one should be re-measured rather than believed.

---

## 1. The law

> **Requisite variety, performance, and composable policy · measure · judgement · control,
> to derive emergent (queryable/aggregatable) projections from any scope, micro to global.**

Five conjuncts, each testable:

| # | Conjunct | Test |
|---|---|---|
| 1 | **Requisite variety** | Can the ontology absorb a valid systems model brought to it, without the model losing what it came with? |
| 2 | **Performance** | Is it expressible at the layer closest to the machine — monomorphized enums and matches, not trait-object soup? |
| 3 | **Composable policy · measure · judgement · control** | Do the four planes compose, or does each re-derive the others? |
| 4 | **Emergent projections** | Is every answer *derived* from witnessed events, never stored as a second home for a number? |
| 5 | **Any scope, micro → global** | Is the answer scale-free by containment, rather than by a tier ladder? |

Conjuncts 4 and 5 are largely won. `Verdict` is Category C (never persisted, reconstructed by
re-evaluation), `Stock` is explicitly never stored, resource state is a pure fold, and
containment landed 2026-08-12 (`epr-rea::scope::Scopes`). **Conjunct 3 is where the family
currently fails** — see §5.

### 1a. Ashby, applied reflexively

A regulator needs at least the variety of what it regulates. Turned on ourselves: **the
family's ontology must carry at least the variety of the space of systems models brought to
it.** That assigns each crate a variety axis rather than merely a layer:

| Crate | Layer | Variety axis it must be sufficient on |
|---|---|---|
| `elohim/epr` | assertion | what kind of claim · who may see it · what it answers to · how well it is known |
| `elohim/epr-rea` | flow | agents · events · resources · promises · stocks · limits · containment |
| `elohim/eprfs` | projection | presence · custody · awareness of where bytes are |
| `eprfs-meta` + `.claude/epr-meta` | normative | how a rule attaches, evaluates, and retires |

### 1b. Reach IS the attenuator

Beer's variety attenuation and amplification are already implemented in this repo under a
different name. `epr::reach::Reach` — `Private → SelfScope → Intimate → Trusted → Familiar →
Community → Public → Commons`, with `openness()` 1–8 — is the vocabulary deciding how far a
signal travels. That *is* attenuation upward and amplification downward, so the reach epic is
not adjacent to this law: it is **conjunct 2 (performance) and conjunct 5 (scale) implemented
as one mechanism.** How a p2p system stays usable at global scale without collapsing is that
not everything reaches everywhere, and reach is earned and declared rather than assumed.
`2026-05-29-epr-reachability-economics.md` is the economic half of the same claim.

The defensible form of the variety claim is already written, and this document adopts it
verbatim rather than restating it (`2026-07-22-reach-ontology-vocabulary-split-spec.md`):

> the elohim does not achieve requisite variety *for a person*; it is a high-variety attenuator
> **that asks the human how the attenuation should be done**, making variety-attenuation
> accountable to the attenuated.

**Consequence for the control plane.** A feedback signal has a reach. `AlgedonicEvidence` today
carries a `target` (the CID of the threatened promise) and no reach at all — which is precisely
why it has no destination. The shape it wants is a signal that is *anonymous, non-binding, and
bounded*: it does not accuse, it carries no standing consequence (`STANDING_IMPACT` is pinned
to `"advisory"`), and it **knows who to report to**. That last clause is a reach, not a layer
string. The same spec already states the law: *"A referral that can only be heard by the thing
being complained about is not a referral."*

---

## 2. The premise that changes the shape: abundant intelligence

The protocol assumes an eventual state of abundant intelligence. That is not decoration on the
systems-thinking inheritance — it inverts one of its binding constraints.

**What always broke was attenuation.** A watershed compressed into a quarterly report loses
precisely what mattered, irreversibly, and no amount of System 3 rigor recovers it. That
compression ratio was the constraint, and it was *cognitive*. Beer, Meadows, Foster and
McCarthy did not model an ecosystem being **represented agentically** — an agent that can
converse, negotiate, learn, and contract on the domain's behalf, across languages — because it
was unavailable, not because it was rejected. Where it was imagined at all, it was dismissed.

Take the premise seriously and the binding constraint moves: **how efficiently a sufficiently
capable model can be deployed to represent the requisite variety of the context it is
positioned in, faithfully, in service to human flourishing — including the earth we share with
nature and machines.**

This is also why re-approaching technology design *itself* from a viable-system frame is worth
the trouble: VSM supplies the language for every other domain built with it. The dev process is
therefore the **first domain modeled, not a side-quest** — `epr flow project`, the `.epr-meta`
compose gates, the valueflow over this repo. If the frame cannot model software development it
will not model a watershed. The broken seams in §5 are not incidental to that demonstration;
they *are* the demonstration failing.

### 2a. What this premise changes about the substrate's job

If agents carry domain variety, **the substrate does not need the union of every framework's
ontology.** It needs no Georgist rent term, no Keynesian multiplier, no Robeynsian band as a
native type. The agent positioned at the boundary is the variety amplifier; the substrate is
what makes the agent's representation **disputable**.

That is a much smaller, sharper job — and roughly what the epr family already tries to be:
claim, confidence, basis, coupling, witness, verdict, reach.

### 2b. Faithfulness is not sufficient — the telos does real work

An agent can *faithfully* represent an extractive interest. Fidelity has no direction. Bound to
a telos, the same representation becomes assessable as faithful-and-serving versus
faithful-and-extractive.

The substrate's job is **not to prevent the second** — it cannot, and attempting it builds a
censor. It is to make it visible and attributed. This is the "third horn" the Meadows survey
named as ours: *make the consequence visible and attributed without making the resource
ownable.* That is how a telos is served without a center, and the only form compatible with the
p2p premise.

### 2c. The bet must degrade gracefully

Abundant intelligence is a bet. If it is late or partial, what we build must still work — with
humans and ordinary tooling, at lower variety. Today it does, by construction: a bound can be
declared by a human, a fold runs with no agent, containment is walked by code. **Nothing landed
on 2026-08-12 requires a capable model; it only becomes more useful with one.**

Preserve that asymmetry deliberately. The tempting designs are the ones that only work if the
model is good.

---

## 3. The acceptance criterion

Replacing "does the ontology have requisite variety" (untestable) with:

> **Can two agents, each faithfully representing a different domain under a different
> framework, negotiate a contract on this substrate such that a THIRD PARTY can walk the result
> back and disagree with the method?**

This is the commons-holonic "fruits" test promoted from design heuristic to acceptance
criterion. It is testable, it is failable, and it fails today (§5).

Note what it demands that a type-check does not: the current system can satisfy every type,
verify every CID, and still fail, because **a verified derivation is not a walkable one.**

### 3a. The admission rule — how the ontology grows without over-engineering

> **Admit a new primitive when a SECOND independent framework needs the same missing
> distinction. Never on the first. Never speculatively.**

Bespoke code appearing at a seam is the *evidence* that a distinction is missing; one framework
wanting it is a `hold`, two is an `admit`.

This rule is not novel here — the repo wrote it down in June, for this exact class of code
(`elohim/elohim-storage/src/services/measure.rs:6`):

> v1 placement: storage-local (arc decision #2); **graduates to a shared crate when a second
> consumer appears.**

**Worked application.** `delay` qualifies today: Meadows needs it (perception delay is his third
cause of overshoot; `respite_response` supplies only the ratio), Keynes needs it (multiplier
lag), Beer needs it (System 2 damping, unbuilt — lagged quota response against a regenerating
stock is the canonical fishery collapse). Three frameworks, one absent primitive. A **band**
(floor *and* ceiling on one quantity, `Commitment.bound` being singular) is currently one
framework — Robeyns — and therefore a `hold` until a second (Raworth's doughnut, a
safe-operating-space model) asks for the same shape.

---

## 4. Measured state, 2026-08-12

Every row was measured on the tree that day. **Re-measure rather than believe.**

| Claim | Evidence |
|---|---|
| Variety is rhetoric, not a quantity | **Zero identifiers** named `variety` in the tree — no struct, field, fn, or metric. No entropy/HHI/cardinality instrument. Only doc-comments, one homonym schema (`SignalVariety`, a disclosure taxonomy with zero consumers), and a string in a findings ledger |
| One genuine cardinality instrument exists, unconnected | `elohim-storage/src/liveness_contract.rs:237` `state_space()` — a 4×2×2×3 = 48-state enumeration asserted `live==13 / terminal==19 / unreachable==16`, with `LivenessBudget::max_states: 4096` (`crates/seam-contracts`). A real Ashby-variety measure, built for deadlock regression, connected to no variety reasoning |
| Requisite-variety routing is a hardcoded constant | `ReferQuestion.layer`: **3 producers, all literals** (`"community"` ×2 in `reach_earning.rs` / `epistemic.rs`, `"operator"` ×1 in `_lib/epr_meta.py`), **0 readers**, no competence model, no ordering. The ordered `ConstitutionalLayer` with `precedence()`/`can_override()` that could route lives in `elohim/constitution` and is not the type used |
| The control plane emits nothing | `algedonic::should_emit` has **no production caller**; `cite_gate` / `into_verdict` have **no production callers**; the `eae` crate (MACE, subsidiarity, precedent, anomaly) has **zero consumers**, untouched since 2026-01-13 |
| `should_emit` is not hysteresis | One threshold, **no `should_clear` predicate at all**, no time dimension. A stock oscillating at the band edge re-fires every cycle under any caller that closes on non-crossing — the exact failure hysteresis exists to prevent |
| Beer's five gaps, ten weeks on | **0 of 5 substantively closed.** Two documentation closures, one vocabulary closure. The algedonic module is typed, schema-conformant, extensively tested, **and connected to nothing** |
| Real damping exists — on the wrong plane | `head_adoption` contest-then-obey, `contest_backoff` dwell, three circuit breakers, the period-2 oscillation harness. All machine-against-infrastructure. **Nothing damps behaviour between agents** — Beer's System 2, the gap all three readings named |

### 4a. The diagnostic, observed three times

> **Bespoke machinery appearing at a seam is the symptom of a variety deficit one layer down.**

| Instance | Shape |
|---|---|
| `epr-cli/src/repository_validators.rs:508` `test_bench_aggregate_capacity` | bare `i64` sums against a ceiling — no kind, unit, period, or confidence — **inside the governance layer whose sibling crate exists to make that unrepresentable** |
| `elohim-storage/src/services/spatial_capacity.rs` | the carrying-capacity defect that began this arc: a hand-rolled window, unit parser, and denominator vocabulary re-deriving what `epr-rea` models with intervals |
| `elohim-storage/src/services/measure.rs` | GE(α), Gini, top-quantile-share, composite concentration as bare `f32`; **zero contact** with `elohim_epr` (`grep -c` for `MeasureKind\|Confidence\|Quantity\|Interval` = 0); `return 0.0` on empty input — measured-zero for absence, the precise C4 violation the ontology exists to prevent |

All three are in the **right place with the wrong types**, because the ontology crate was
unreachable or unused. That is one deficit observed three times, not three bugs.

### 4b. A finding about findability

The code implementing the protocol's limitarian position **contains no occurrence of the word
"limitarian."** `measure.rs`'s functions are named for the statistics, not the framework. A
grep-driven audit therefore concluded "docs-only, zero code" and was wrong. The
framework → implementation link is unindexed, which is the concrete reason this document is
cite-sealed and bound via `.epr-meta` rather than left as prose.

---

## 5. The four broken seams, ranked under the flourishing telos

Conjunct 3 is a repair list. Each break is a variety deficit *at a seam*: the receiving side
carries less variety than what must cross it.

### Rank 1 — the substrate cannot signal insufficiency
`AlgedonicEvidence` is ceiling-signed on the wire: `crossed()` is `stock >= band_edge`, the
shape is schema-pinned (`additionalProperties: false`) inside the DHT FeedbackSignal whitelist,
and `algedonic.rs` states plainly that a floor bound *"is not modeled."*

`Bound` gained `Sense::{Ceiling, Floor}` on 2026-08-12; the signal plane did not. Floor evidence
is therefore **deliberately withheld** rather than inverted (`fold::bound_evidence` returns
`None` for a floor; pinned by `a_floor_bound_withholds_evidence_rather_than_inverting_it`),
because emitting through a ceiling-signed shape would mint a signal whose own `crossed()` reads
**false** — pain that denies itself, worse than silence.

**Why this ranks first:** ceilings express restraint (*take no more than*); floors express
thriving (*keep at least*). A system that can only say the first can prevent harm and cannot
constitute flourishing. Pain is currently expressible only as "too much," never "too little" —
and insufficiency is what flourishing fails as. Registered as `C7: partial` in
`elohim/epr-rea/seam-registry.yaml`; **that ranking is too low and this document supersedes
it.**

Moves a wire shape → needs its own p2p-design-gate pass. Should carry the reach question from
§1b at the same time: a feedback signal's destination is a reach, not a layer string.

### Rank 2 — no standing for a subject that cannot speak
`declarer` carries **no distinction between self-report and proxy-report**. A forest cannot
declare; nor, differently, can a machine without standing. An ecosystem's pain is always
declared on its behalf, and the model cannot say by whom, or on what standing.

Pre-abundance this is a modeling nicety. Under §2 it is **the central capture vector**: an agent
representing an ecosystem that nobody can dispute is strictly worse than no representation,
because it launders a claim into the accountability spine.

The canon already has the frame — `constitution.md` App. C Q5 (non-human entities), and the
standing position that neither species is terminal, the method is. The **type system has no
vocabulary for speaking on behalf of.**

### Rank 3 — a number dies at the policy boundary
`ValidatorOutcome::{Pass, Flag{reason: String}, Unavailable}` is variety-poor on the *return*
path. A policy that computes a real quantity must stringify it, and the quantity is lost.
`epr::verdict::CheckWitness` has exactly the missing field — `observed: Option<Value>` — and
`eprfs-meta` cannot reach it (`elohim-epr` is deliberately unreachable from that crate, and
correctly so).

This is why `eprfs-meta`'s `measure:` predicate degenerated into two `u64` line-count ceilings
while the registry, the Python gate, and `measures.yaml` all speak the full `level|rate|ratio`
ontology the native evaluator cannot read.

**Smallest of the four.** Widening the outcome's evidence channel does not require `eprfs-meta`
to depend on `elohim-epr` — the value crosses as data, the way `cid`/`fuel` now do
(`8fa1e5bed`).

### Rank 4 — no runtime POSIWID watcher
*The purpose of a system is what it does.* Beer's Gap 3: a runtime watcher of aggregate
behaviour against the constitutional telos. `posiwid` appears in 12 files, **all prose**.

Least tractable while the telos was implicit; most necessary now that it is named. Ranked last
only because ranks 1–3 make its inputs honest.

### 5a. The safety pair
Two of the above are not merely correctness items — they are what makes abundant intelligence
safe to build on rather than a way to industrialize plausible claims:

- **`fuel`** — an algorithm is a *variety amplifier*: it produces more distinguishable outputs
  than its inputs justify. `fuel` was the bound on it. An unmetered amplifier inside a system
  whose entire claim is requisite variety is the one component that can outrun its own
  regulator. Partially revived 2026-08-12 (`8fa1e5bed`): the declaration now reaches the host
  boundary; `eprfs-meta` still meters nothing.
- **Proxy-declarer standing** (rank 2) — an amplifier speaking for something that cannot
  contradict it.

---

## 6. What NOT to build

1. **Not a union ontology.** Do not add a Georgist rent term, a Keynesian multiplier, or a
   Robeynsian band because a framework mentions one. Under §2a the agent carries domain variety.
   Adding domain primitives speculatively is the over-engineering this law exists to prevent,
   and it would encode the pre-abundance assumption into the type system.
2. **Not a variety metric for its own sake.** `liveness_contract::state_space()` earns its keep
   because a decision surface's state count is *actionable* (it caught deadlocks). A general
   "variety score" would be a number with no reader — the instrument-with-no-reader shape this
   layer has repeatedly grown.
3. **Not a second measure ontology.** Three instances in §4a already re-derived one. The cure is
   reaching the existing one (or widening the seam so it can be reached), never minting another.
4. **Not a trait per validator.** Content addressing and compile-time binding are incompatible:
   you cannot `impl Validator for` a CID resolved at runtime. `ValidatorProvider` is correctly
   *one* interface with data crossing it — identity by content and accountability by
   declaration, not identity by type.
5. **Not a design that requires a capable model.** See §2c.

---

## 7. Open questions — for the fresh reader to confirm or contest

These are genuinely open. Nothing below is settled by this document.

- **Q1.** Is §3's acceptance criterion right, or does it under-weight conjunct 2 (performance)?
  A fully walkable derivation may be expensive at global scale; reach (§1b) is the proposed
  answer, but it has not been tested against the criterion.
- **Q2.** Does the admission rule (§3a) hold when the *second* framework arrives through an agent
  rather than a human designer? An agent can manufacture a second demand cheaply, which makes
  "two frameworks" a weaker gate than it looks under abundance.
- **Q3.** Should `Bound` become plural on `Commitment` (a band), or should a band be two
  commitments? §3a says `hold`; that is a judgement about evidence, not a design conclusion.
- **Q4.** Is `Sense` the right axis, or a special case of a more general *direction of concern*
  that would also cover rate-of-change bounds (accelerating depletion inside a level that is
  still comfortable)?
- **Q5.** Rank 1 proposes carrying reach on a feedback signal. Does that collapse into the
  existing `Reach` enum, or does a signal's "who to report to" need a different vocabulary from
  a content item's "who may see it"? The two may be homonyms.
- **Q6.** §4 claims 0 of 5 Beer gaps closed. Measured once, by one pass. **Re-measure.**
- **Q7.** Is the `eae` crate (zero consumers, a real subsidiarity implementation, an ordered
  `ConstitutionalLayer`) dead code to delete, or the un-wired answer to rank 2 and the `layer`
  routing gap? It has not been evaluated either way.

---

## 8. How to use this document

- **Designing in `epr`/`epr-rea`/`eprfs`:** §1 is the law, §6 is the refusal list, §3a is the
  admission rule. The `.epr-meta` in those crates binds a rule that surfaces this document on
  writes to the decision surface.
- **Picking up the repair work:** §5, in rank order. Ranks 1 and 2 move wire shapes and need
  p2p-design-gate passes; rank 3 does not.
- **Contesting it:** §7 first, then §4 — the numbers are the load-bearing part and they were
  measured once. A contested number is more useful here than a contested opinion.
