---
title: "Measure Dynamics + Confidence Ontology — six laws, each anchored to its enforcing construct"
id: measure-dynamics-confidence-ontology-design
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: Tasks 3-6 of the measure-ontology-slice1-epr-local-first plan land, each satisfying the law this spec names for it (L5 via fold::with_uncertainty, L6 via the .epr-meta measure-tier kind check, L2's typed canonical-bytes entry point wired) OR the plan is superseded
created: 2026-08-11
domain: D2
topic: [measure, middot, confidence, interval, uncertainty, dag-cbor, canonical-bytes, epr-meta, rea, meadows]
cites:
  - genesis/data/timeline/backlog/measure-family-borrows-backlog.md
  - middot-measure-primitive-design | Extends: measure family kind/rate vocabulary generalizes the same shape Middot already generalizes across lint findings, compute cycles, listening minutes, view counts, bytes stored, watts consumed, time spent | sha256:336ab2b4619b9144 | path: genesis/docs/superpowers/specs/2026-08-04-middot-measure-primitive-design.md
  - genesis/research/meadows-systems-dynamics-cross-pollination-2026-08-11.md
  - elohim/epr/src/measure.rs
  - elohim/epr/tests/measure_ontology.rs
  - elohim/epr/src/cbor.rs
  - elohim/epr/src/cid.rs
  - elohim/epr-rea/src/fold.rs
  - .claude/scripts/_lib/epr_meta.py
---

# Measure Dynamics + Confidence Ontology

## 1. Position (MAP: D2 Evidence Primitives)

Task 2 of the `2026-08-11-measure-ontology-slice1-epr-local-first` plan. Task 1 landed the
vocabulary this spec formalizes — `elohim/epr/src/measure.rs` (commit `e4af04b6b`): `Period`,
`MeasureKind` (`Level` / `Rate { per }` / `Ratio`), `Interval`, `ClaimKind`, `Confidence`
(`claim` / `interval` / `basis`), `Quantity`. This spec is the "formalize and seal" half of the
operator's dispatch: it states the laws that vocabulary encodes and answers rows 12–18 of
[measure-family-borrows-backlog](epr:backlog-measure-family-borrows-backlog), sourced from the
[Meadows systems-dynamics survey](epr:meadows-systems-dynamics-cross-pollination-2026-08-11) §3
(stock/flow/rate confusion) and the operator's 2026-08-11 confidence-qualifier dispatch. Tasks
3–6 cite this spec and build the constructs it names but does not yet find on disk.

**The governing rule, repeated because it is the whole point of this document:** every law below
names the construct that enforces it — either an already-landed piece of `measure.rs` (proven
now), or a specific task in this six-task plan committed to building it (named and dated, not a
someday). A claim with neither is not a law. It is numbered in §3 and left explicitly open.

## 2. Six laws

### L1 — A rate declares its period; a level does not have one

A quantity's dynamic type is not free-floating metadata alongside its value — `Rate` cannot be
constructed without also carrying its denominator.

**Enforced by:** `MeasureKind::Rate { per: Period }` (`elohim/epr/src/measure.rs:25-29`) — the
variant's own shape makes a periodless rate unrepresentable; there is no `MeasureKind::Rate`
constructor that omits `per`. `MeasureKind::period()` (`measure.rs:32-37`) returns `Some(per)`
only for `Rate`, `None` for `Level`/`Ratio`. Test:
`a_rate_cannot_exist_without_a_period` (`tests/measure_ontology.rs:3-11`).

This is the type-level fix for the exact defect the backlog row names: `spatial.rs:169`'s
`max_sustainable_yield` carries only `unit: String` — a rate with no time denominator — compared
at `spatial_capacity.rs:150` against an all-time cumulative sum (the live unit error captured
2026-08-11 in `genesis/data/timeline/backlog/2026-08-11-carrying-capacity-cumulative-vs-rate-unit-error.md`).
`MeasureKind` makes that specific confusion a compile error going forward, once a call site
is migrated onto it (migration itself is out of scope for this plan).

### L2 — Confidence rides inside the quantity's canonical bytes

An estimate whose interval can be detached and narrowed after the fact is not an estimate — it
is an unfalsifiable claim wearing an estimate's clothes. The interval must travel inside the same
bytes that get content-addressed, not as a sibling document.

**Enforced by, proven at the serializer level:** this crate's canonical-bytes contract is
**dag-cbor, not JSON** — `elohim/epr/src/cbor.rs` wraps `serde_ipld_dagcbor` (RFC 8949 §4.2.1
core deterministic encoding: sorted map keys, shortest-form integers, no indefinite-length
items), and `elohim/epr/src/cid.rs::compute_cid` derives a CIDv1 (codec `0x71`, dag-cbor) by
hashing exactly those bytes. `Quantity` (`measure.rs:134-140`) holds `confidence: Confidence` as
a plain struct field, not a link — there is no way to serialize a `Quantity` that drops or
detaches its `Confidence`. Independently reverified in this task (Task 1's own canonical-bytes
claim, re-run rather than taken on faith): `serde_ipld_dagcbor::to_vec(&quantity)` encodes a
`Quantity` to **116 bytes**; two `Quantity` values differing only in `Confidence.interval`
produce **different** 116-byte canonical encodings; `cbor::decode_strict`'s
re-encode-must-equal-input check (`cbor.rs:21-32`) passes against hand-built
`Ipld::Map`-shaped canonical bytes for a quantity; the round trip (`serde_ipld_dagcbor::from_slice`
then compare) is equality-preserving.

**Not yet wired — say this plainly rather than implying it is wired.** The crate's own
`cbor::encode` signature is `encode(value: &Ipld)` (`cbor.rs:10-12`) — it accepts only `Ipld`,
never a typed value directly. Every existing canonical-bytes producer in this crate hand-builds
a `BTreeMap<String, Ipld>` before calling it: `envelope.rs:87` (`Ipld::Map(map)` built from an
`Epr`'s fields), `witness.rs:115` and `witness.rs:204` (same pattern for `WitnessedInteraction`
and `FrameVerdict`). There is today **no function that takes a `Quantity` and returns its
canonical bytes or CID** through the crate's own API. L2 is proven at the serializer level and
unwired at the crate's typed API level; Task 6 must add the typed entry point (a
`measure::canonical_bytes(&Quantity) -> Result<Vec<u8>>` or equivalent `Ipld`-building function,
mirroring the `envelope.rs`/`witness.rs` pattern) before any caller can compute or verify a
measure's own CID.

### L3 — Honest absence is the degenerate interval, not a nullable field

"Unmeasured" is not a third state alongside "measured" and "estimated" that needs its own
`Option<Confidence>` — it is the estimate whose interval admits everything.

**Enforced by:** `Interval::unknown()` (`measure.rs:54-59`, returns
`{ lo: NEG_INFINITY, hi: INFINITY }`) and `Interval::is_unknown()`
(`measure.rs:60-62`, `lo.is_infinite() && hi.is_infinite()`). Test:
`interval_unknown_is_the_degenerate_case_of_honest_absence`
(`tests/measure_ontology.rs:13-21`) — asserts `Interval::unknown().is_unknown()`,
`!Interval::exact(3.0).is_unknown()`, and that the unknown interval `contains` both
`f64::MIN_POSITIVE` and `1e300`.

This subsumes C4 (honest absence) rather than merely satisfying it, per
[measure-family-borrows-backlog](epr:backlog-measure-family-borrows-backlog) row 16's framing:
the render distinction between "unmeasured" and "measured-zero" falls directly out of the
ontology (an unknown interval vs. `Interval::exact(0.0)`) instead of needing a separate
special-cased nullable field.

### L4 — Widening is free; narrowing requires a new observation

An agent may always admit it knows less than it claimed. It may never claim to know more without
producing a new observation to justify the tighter bound.

**Enforced by:** `Confidence::widen(&self, to: Interval) -> Result<Confidence, ConfidenceError>`
(`measure.rs:122-131`), which succeeds iff `self.interval.is_widened_by(&to)`
(`measure.rs:70-72` — true iff `other.lo <= self.lo && other.hi >= self.hi`, i.e. the argument
interval is at least as wide as the receiver on both sides), and otherwise returns
`Err(ConfidenceError::NarrowingRefused)` at `measure.rs:129` (the `NarrowingRefused` variant
itself, carrying the message "narrowing an interval requires a new observation, not a mutation",
is defined at `measure.rs:93-97`). Test: `widening_is_free_and_narrowing_is_refused`
(`tests/measure_ontology.rs:23-36`).

Naming note: the method is `is_widened_by`, not `is_widening_of` as an earlier plan draft called
it. The earlier name read backwards — the body returns `true` when the *argument* widens the
*receiver*, not the reverse. This spec names the method that actually exists.

**Caveat — this is a call-site convention, not a structural invariant of the type.**
`Confidence`'s fields (`measure.rs:86-91`: `claim`, `interval`, `basis`) are all `pub`, and so
are `Quantity`'s. Nothing in the type prevents a caller from bypassing `widen()` entirely with a
direct struct literal — `Confidence { interval: narrower, ..existing }` — which performs exactly
the narrowing `widen()` refuses, with no error and no compiler complaint. L4 holds for any code
that routes a narrowing attempt through `Confidence::widen`; it holds for nothing else. See §3
Q8.

### L5 — A fold over interval-carrying quantities returns an interval

A fold that sums `Quantity` values carrying `Confidence` and returns a bare `f64` has manufactured
false precision — the mechanism this spec's own preamble half-quotes ("lies, damn lies, and
statistics"). A fold over uncertain inputs must return an uncertain output.

**Enforced by: not yet — named for Task 3.** `elohim/epr-rea/src/fold.rs` today folds
`FlowEvent`s into `ResourceState`/`FulfillmentStatus` over bare `f64` totals
(`resource_state`, `fold.rs:34-46`); there is no interval-aware fold anywhere in this crate as of
this spec. Task 3 adds `fold::with_uncertainty` and a closure test asserting the return type
carries an interval end to end — that test is the enforcing construct once it exists. Until Task
3 lands, L5 is a stated law with its enforcing construct named and its build committed within
this plan, not yet live. It is not an open question (§3) because a specific task owns building
it; it is also not yet true of any code on disk.

### L6 — A measure declaration without `kind` is refused at the gate

A `.epr-meta` rule that opts into the measure family (i.e. declares a `measure:` block naming a
quantity this repo tracks) but omits `kind` reintroduces at the governance layer exactly the
ambiguity L1 makes unrepresentable in Rust — a limit written against a level compared to a rate,
or vice versa, with nothing to catch it.

**Anchor — nested inside the existing policy-owned `measure:` block, not a rule-level `kind:`
key.** An earlier plan draft proposed a top-level `kind:` key on the `.epr-meta` rule itself; that
was overruled on architectural grounds. `measure:` is the **policy-owned** semantics block
(`.claude/scripts/_lib/epr_meta.py:1331`, the class-`measure` validator, currently checking only
integer `loc-soft`/`loc-hard`). Separately, `_BINDING_KEYS = {id, policy, params, when, why}`
(`epr_meta.py:192`) causes a policy-**binding** rule (one that references a registry policy via
`policy: <id>@<version>`) to be rejected for carrying any other top-level key — a rule-level
`kind:` on a binding rule would therefore be undeclarable by exactly the rules that bind a
registry policy, giving two homes with no declared precedence between them. `kind` (and `per`,
for a rate-kind measure) instead live **inside** the `measure:` block: policy-owned, inherited
through the existing merge (`epr_meta.py:1404-1405`,
`m = dict(pol.get("measure") or {}); m.update(rule.get("params") or {})`), with a binding's
`params` carrying local variance over the policy's declared default.

**Enforced by: not yet — named for Task 4.** The existing `measure:` validator
(`epr_meta.py:1331-1341`) checks only the pre-existing line-count measure class
(`loc-soft`/`loc-hard` integers); it does not today require or even recognize `kind`. Task 4
extends this validation so a measure-family policy or binding whose `measure:` block omits
`kind` is refused at load (`errs.append(...); continue`, the same reject-and-report shape the
`loc-soft`/`loc-hard` check already uses at `epr_meta.py:1337-1340`) rather than silently loaded
with an undeclared kind. Until Task 4 lands, L6 is — like L5 — a stated law with its enforcing
construct's location and owning task named, not yet live.

## 3. Open questions — no enforcing construct, and saying so is the point

Per the governing rule in §1: a claim with no construct that enforces it, and no task in this
plan committed to building one, is written here instead of as a law.

- **Q1 — interval arithmetic is deliberately naive in slice 1.**
  `[a,b] + [c,d] = [a+c, b+d]` assumes perfect correlation between the two intervals and
  therefore *over-widens* for independent terms. Over-widening is the safe direction — it never
  manufactures precision — but a correlation-aware fold is slice-2 work and must not be faked
  here. (No arithmetic operators exist on `Interval` in `measure.rs` today at all; this question
  is about the shape the eventual Task 3 fold must not silently assume.)
- **Q2 — statistical-method application is ceiling work, not substrate work.** Reconciling
  divergent estimates belongs at the elohim ceiling as a `ComputationAttestation` at a
  graduated-rigor tier, where a contested narrow interval escalates exactly as a contested
  clustering does. Slice 1 ships the *inputs* to that reconciliation, never the reconciliation
  itself.
- **Q3 — the uncertainty work-queue (measure-family-borrows-backlog row 18) is not built here.**
  Decomposing an aggregate's uncertainty by contribution — "which edge, if measured better, most
  tightens this aggregate?" — needs L5 in place first, and is out of scope for this plan.
- **Q4 — network semantics are out of scope by design.** Confidence/interval propagation across
  a P2P sync boundary (whose interval wins on conflicting replicas, whether a wider interval
  from a stale peer should ever be preferred over a narrower one from a fresher peer) is not
  addressed by this spec. See Task 6.
- **Q5 — `basis` non-emptiness is documented but unenforced.** `Confidence`'s doc comment
  (`measure.rs:89`) reads "what grounds this claim. A bare ± is uninterpretable," and
  `witnessed_and_estimated_are_distinguishable_and_basis_is_required`
  (`tests/measure_ontology.rs:38-46`) asserts one literal instance is non-empty — but
  `Confidence::estimated(interval, "")` constructs fine; nothing rejects an empty `basis`. There
  is no enforcing construct, so per this spec's own rule this is a question, not a law. What
  enforcement would look like, without choosing one: a fallible constructor
  (`Confidence::try_estimated(..) -> Result<Confidence, ConfidenceError>` that rejects an empty
  or whitespace-only basis), a `Basis` newtype wrapping a validated non-empty `String`, or a
  separate validator invoked at the same boundary where `.epr-meta` validates `measure:` blocks
  (L6's home). Slice 1 decides none of these.
- **Q6 — the zero-absorption lower bound.** Task 5 computes a generation ÷ absorption ratio
  where, when counted absorption is zero, the interval's upper bound is `+∞` (division toward
  zero from the denominator side). Whether the *lower* bound should then be `0.0` rather than
  `-∞` is genuinely unsettled: a ratio of counts cannot be negative, so `-∞` may be a category
  error inherited from `Interval::unknown()`'s symmetric `{-∞, +∞}` shape rather than a
  deliberate choice for this specific ratio family — but narrowing it to `[0, +∞)` is itself a
  claim (that the numerator and denominator are both non-negative-definite counts, which holds
  for harvest/regeneration but is not a property of `Interval` in general) that needs its own
  justification rather than a default. Slice 1 deliberately does not decide this; Task 5 must
  not silently pick a side by construction.
- **Q7 — nothing ties `Quantity.value` to `Quantity.confidence.interval`.** `Quantity`
  (`measure.rs:134-140`) stores `value: f64` and `confidence: Confidence` as independent fields;
  no constructor or validator checks `confidence.interval.contains(value)`. A `Quantity` can be
  built today with `value: 100.0` and `confidence.interval = Interval::new(0.0, 10.0)` —
  internally incoherent, since the interval is supposed to bound the very value it sits beside —
  and nothing in the type or the test suite catches it. This is not named as a law anywhere in
  the source plan; it is recorded here as a genuine gap surfaced by applying the governing rule
  adversarially to the type as it exists, not because a construct was claimed and found missing.
  What enforcement would look like: a `Quantity::new(value, kind, confidence) -> Result<...>`
  constructor that checks containment, or accepting the current shape as intentional (a witness
  can legitimately record a point reading that later turns out to lie outside its own stated
  confidence band, and rejecting that at construction time would make the type unable to
  represent its own falsification). Slice 1 does not decide which.
- **Q8 — should `Confidence`/`Quantity` fields become private to make L4 a structural
  invariant?** L4's caveat above records that `widen()`'s narrowing refusal is trivially
  bypassed by direct struct construction, since every field involved is `pub`. Making the fields
  private and routing all construction through `witnessed()` / `estimated()` / `unknown()` /
  `widen()` would upgrade L4 from a convention observed by well-behaved callers to a type-level
  guarantee — at the cost of losing `..struct update` ergonomics used throughout the existing
  test suite and possibly elsewhere in this plan's later tasks. Slice 1 does not decide this
  either way; it is recorded so Task 6 (or whichever task next touches `measure.rs`'s public
  surface) does not silently narrow it without the tradeoff being named.

## 4. What this spec seals

The six laws above, and no more, are canon as of this document: L1, L3, L4 (with its Q8 caveat)
are enforced now by code that exists and is tested. L2 is proven at the dag-cbor serializer level
and explicitly not yet wired to a typed `Quantity` entry point. L5 and L6 are committed to Tasks 3
and 4 respectively, named precisely enough that "the law this spec promised" is a checkable
claim against those tasks' own deliverables, not a vague aspiration. Q1–Q8 are recorded as open
rather than smuggled in as unenforced laws. Tasks 3–6 build against this vocabulary; none of them
should re-litigate L1–L6 without citing back here.
