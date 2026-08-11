---
title: "Measure Dynamics + Confidence Ontology — six laws, each anchored to its enforcing construct"
id: measure-dynamics-confidence-ontology-design
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: Tasks 3-6 of the measure-ontology-slice1-epr-local-first plan land, each satisfying the law this spec names for it (L5 via fold::with_uncertainty, L6 via the .epr-meta measure-tier kind check on both the manifest and registry paths) — L2 stays serializer-proven-but-unwired per §4 until slice 2 builds the typed canonical-bytes entry point, which is explicitly not owed by this plan — OR the plan is superseded
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
unwired at the crate's typed API level. Building the typed entry point (a
`measure::canonical_bytes(&Quantity) -> Result<Vec<u8>>` or equivalent `Ipld`-building function,
mirroring the `envelope.rs`/`witness.rs` pattern) is explicitly **not** a deliverable of this
plan — §4 ("What slice 2 inherits") names it as an inherited gap for slice 2 to build before any
caller can compute or verify a measure's own CID, not a Task 6 obligation this document is still
waiting on.

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

**Enforced by:** `fold::with_uncertainty` (`elohim/epr-rea/src/fold.rs`) — sums the input
intervals, takes the **weakest** claim among its inputs (`claim_rank` reproduces `ClaimKind`'s
own declaration order), and refuses to fold quantities of mixed `MeasureKind`
(`FoldError::MixedKinds`). The crate's original `f64`-only fold, `resource_state`, stands
untouched alongside it — composition, not a fork. Tests:
`elohim/epr-rea/tests/uncertainty_closure.rs` (`a_fold_over_intervals_returns_an_interval`,
`the_aggregate_takes_the_weakest_claim_of_its_inputs`,
`one_unknown_term_makes_the_aggregate_unknown_not_wrong`, `folding_mixed_kinds_is_refused`,
`folding_is_deterministic_under_input_reordering`).

It enforces more than the plan's own tests set out to prove: two `Rate` quantities with
*different* periods also refuse to fold — `MixedKinds(Rate { per: Day }, Rate { per: Year })`
fires — because `Period` rides inside `MeasureKind`'s derived `PartialEq`, so a period mismatch
is a kind mismatch by construction. That closes, on a branch the plan never wrote a test for,
exactly the unit-error class L1 names (a rate compared against a cumulative level with no
compile-time objection).

Interval arithmetic here is deliberately naive — `[a,b]+[c,d]=[a+c,b+d]` over-widens for
independent terms (Q1) — and `f64` addition is commutative but not associative, so cross-peer
reordering determinism is real but explicitly out of scope for slice 1 (§4 item 3).

### L6 — A measure declaration without `kind` is refused at the gate

A `.epr-meta` rule that opts into the measure family (i.e. declares a `measure:` block naming a
quantity this repo tracks) but omits `kind` reintroduces at the governance layer exactly the
ambiguity L1 makes unrepresentable in Rust — a limit written against a level compared to a rate,
or vice versa, with nothing to catch it.

**Anchor — nested inside the existing policy-owned `measure:` block, not a rule-level `kind:`
key.** An earlier plan draft proposed a top-level `kind:` key on the `.epr-meta` rule itself; that
was overruled on architectural grounds. `measure:` is the **policy-owned** semantics block
(`.claude/scripts/_lib/epr_meta.py`'s `load_policies`, its `class: measure` branch — which, at
this design's authoring time, checked only integer `loc-soft`/`loc-hard`). Separately,
`_BINDING_KEYS = {id, policy, params, when, why}` (`epr_meta.py:192`) causes a policy-**binding**
rule (one that references a registry policy via `policy: <id>@<version>`) to be rejected for
carrying any other top-level key — a rule-level `kind:` on a binding rule would therefore be
undeclarable by exactly the rules that bind a registry policy, giving two homes with no declared
precedence between them. `kind` (and `per`, for a rate-kind measure) instead live **inside** the
`measure:` block: policy-owned, inherited through `load_policies`' expansion step
(`m = dict(pol.get("measure") or {}); m.update(rule.get("params") or {})`), with a binding's
`params` carrying local variance over the policy's declared default.

**Enforced by, on both paths:** `validate_meta`'s `class: measure` branch (manifest-declared
rules, e.g. `genesis/research/.epr-meta`) and `load_policies`'s `class: measure` branch
(registry-declared policies in `.claude/epr-meta/policies.yaml`) both refuse a `measure:` block
that omits `kind`, names a `kind` outside the vocabulary, or declares `kind: rate` with no `per:`
— sharing one vocabulary constant, `MEASURE_KIND_VOCAB = {"level", "rate", "ratio"}`
(`.claude/scripts/_lib/epr_meta.py`), rather than each path carrying its own copy. The
manifest-path gate landed at Task 4's first pass; the registry-path gate was added in a fix round
after review found that a policy-binding rule's *expanded* `measure:` block reached the evaluator
unvalidated — `validate_meta` never sees an expanded binding, only the raw rule, so the registry
loader was the one path `kind` wasn't actually required on. Closed as the same mechanism with a
hole, not forked. Test: `.claude/scripts/_lib/__tests__/epr_meta_measure_ontology_test.py`.

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
- **Q9 — `Interval` has no invariants of its own, and this is reachable, not just theoretical.**
  There is no `lo <= hi` check and no sign-consistency check anywhere in `measure.rs`;
  `Interval::new(f64::INFINITY, f64::NEG_INFINITY)` constructs happily, as does any other
  malformed pairing. The reachable consequence: a fold that sums a `(-∞)` bound against a `(+∞)`
  bound produces `NaN`, not an error — `NaN` then silently satisfies neither `contains()` nor any
  comparison, so a downstream consumer sees an interval that answers `false` to every containment
  check rather than a visible failure. Today this is reachable only through a directly-constructed
  malformed `Interval` (`Interval::new` performs no validation), never through any in-crate
  constructor path — `Interval::exact`, `Interval::unknown`, and every `Confidence` constructor
  build well-formed intervals. This is a distinct question from Q8: Q8 is scoped to
  `Confidence`/`Quantity`'s public fields defeating `widen()`'s narrowing refusal; Q9 is about
  `Interval` itself admitting a value with no ordering relationship between its own two bounds,
  independent of who or what constructed it. What enforcement would look like, without choosing
  one: a fallible `Interval::try_new(lo, hi) -> Result<Interval, IntervalError>` that rejects
  `lo > hi` and non-finite-vs-finite sign mismatches, or accepting `new` as an unchecked
  low-level constructor and pushing the check to whichever slice-2 fold first combines untrusted
  intervals from separate peers. Slice 1 does not decide this.
- **Q10 — a multiplier-based widening scheme can manufacture false precision at a zero base,
  and the fix is a general mechanism, not a one-off.** Task 5's generation ÷ absorption ratio
  computed a bound as `counted × [1.0, 3.0]`; when `counted == 0`, both `0 × 1.0` and `0 × 3.0`
  evaluate to `0`, so the interval collapses to zero width at exactly the moment the underlying
  count is least trustworthy — the false-precision failure this whole ontology exists to prevent,
  and the opposite of what a multiplier-based widening scheme is supposed to produce. Task 5's own
  zero case is fixed (mirroring `Interval::unknown()` when the base is zero rather than
  multiplying through it), but the general mechanism this exposes is not scoped by any existing
  question: **any `interval = scalar × [lo_mult, hi_mult]` construction is unsound at
  `scalar == 0`, regardless of which measure family it is applied to**, because multiplication by
  zero destroys the multiplier's width information rather than propagating it. This is distinct
  from Q6, which asks only whether a ratio's floor should be `0` vs `-inf` and presupposes an
  already-wide interval; Q10 is about the base-value case that makes the interval narrow (or
  zero-width) in the first place, upstream of what its bounds should be. What a general fix looks
  like without choosing one: a `Confidence::multiplier_widen(scalar, lo_mult, hi_mult)`-shaped
  helper that special-cases `scalar == 0` (or any zero-crossing scalar) into `Interval::unknown()`
  or an explicitly-justified fixed floor/ceiling, so each future multiplier-based fold does not
  have to rediscover this failure independently. Slice 1 does not build this helper; it is
  recorded as a mechanism, not just Task 5's instance of it.
- **Q11 — the fold destroys `basis`.** `fold::with_uncertainty` (`elohim/epr-rea/src/fold.rs`)
  sets the aggregate's `confidence.basis` to `format!("fold of {} terms", items.len())`,
  discarding every input's own grounding. `Confidence`'s doc comment says "a bare ± is
  uninterpretable" — the fold's own output basis names no observation, instrument, or window; it
  is a term count, which is exactly a bare ± with a count attached. L5's law text ("a fold over
  interval-carrying quantities returns an interval") is satisfied — the interval is real and
  correctly propagated — but the ontology's own interpretability standard for `basis` is not.
  This is distinct from Q5 (an empty `basis` reaching *construction* uncaught): Q11 is `basis`
  being *overwritten* at fold time, discarding real input grounding that did exist, not `basis`
  being absent from the start. What enforcement would look like, without choosing one: a
  fold-specific basis-composition rule (concatenating or summarizing each input's basis, bounded
  by some length or count), a `Confidence` shape that carries a list of contributing bases rather
  than one string, or treating a term-count summary as sufficient grounding for an *aggregate*
  specifically — a different interpretability bar than a single observation's. Slice 1 decides
  none of these.
- **Q12 — `Interval::is_unknown()` has a false positive on the exact shape L3 forbids.**
  `Interval::is_unknown()` (`elohim/epr/src/measure.rs`) checks `lo.is_infinite() &&
  hi.is_infinite()` with no sign check, so `Interval::new(f64::INFINITY, f64::INFINITY)` — a
  zero-width, same-sign collapse, not the `{-inf, +inf}` shape `Interval::unknown()` actually
  constructs — reports `is_unknown() == true`. This is the exact degenerate shape Task 5's fix
  round classified as a conformance bug against L3 (a zero-width `[inf, inf]` asserting spurious
  certainty at the precise moment data is most absent) when it first appeared in
  `doc_dynamics.py`'s pre-fix output. This spec names `Interval::is_unknown()` as L3's enforcing
  detector; concretely, `elohim/epr-rea/tests/uncertainty_closure.rs`'s unknown-term assertion
  (`out.confidence.interval.is_unknown()`) would pass identically whether the aggregate collapsed
  to the honest `{-inf, +inf}` or to the dishonest `[inf, inf]` — the detector cannot tell the two
  apart. Q9 covers `Interval`'s missing `lo <= hi` invariant generally (a malformed interval with
  no ordering relationship between its own bounds); Q12 is narrower and sharper — this specific
  same-sign-infinite shape passes L3's own named detector as if it were honest absence. What
  enforcement would look like, without choosing one: `is_unknown()` additionally requiring
  `lo.is_sign_negative() && hi.is_sign_positive()` (equivalently, `lo == NEG_INFINITY && hi ==
  INFINITY` exactly), or a separate `is_degenerate()` predicate flagging the same-sign-infinite
  case as its own distinguishable state rather than folding it into "unknown." Slice 1 does not
  decide this.
- **Q13 — the zero-absorption fix corrected the interval and the display, not the point value.**
  `.claude/scripts/_lib/doc_dynamics.py`'s `generation_absorption_ratio` still computes `value =
  ratio(generated, 0) = inf` when `absorbed_counted == 0` — only `confidence.interval`
  short-circuits to `{-inf, +inf}` (Task 5's fix round, mirroring `Interval::unknown()`), and
  `habits-status.py` special-cases the printed line to read "unknown" rather than a bare `inf`. A
  second consumer that reads the documented public return shape (`{"value": ..., "kind": ...,
  "confidence": {...}}`) directly — rather than going through `habits-status.py`'s display logic
  — sees `value == float("inf")` and `value > 1.0` evaluates `True`: "confirmed unbounded
  overshoot," the exact misreading the fix existed to prevent, one layer below where it was
  fixed. Q7 names the general shape of this gap ("nothing ties `Quantity.value` to its own
  `confidence.interval`") but does not name this live, already-shipped instance of it. What
  enforcement would look like, without choosing one: `value` becoming `None`/absent when the
  interval is `Interval::unknown()`-shaped, a documented contract that callers must check
  `interval.is_unknown()` before trusting `value`, or `value` computed as the interval's midpoint
  (undefined for an unbounded interval, which would force the zero-absorption case to make a
  choice it currently avoids). Slice 1 does not decide this, and no code changes here.
- **Q14 — the two `class: measure` gates disagree on what a measure rule minimally is.**
  `validate_meta`'s `class: measure` branch (manifest-declared rules) requires `kind` but no
  quantity being bounded; `load_policies`'s `class: measure` branch (registry-declared policies)
  requires integer `loc-soft`/`loc-hard` *and* `kind`. A manifest rule can therefore satisfy L6
  (declare a valid `kind`) while declaring no quantity a `loc-soft`/`loc-hard`-style measure would
  actually bound. Consequence on disk: `genesis/research/.epr-meta`'s
  `readme-blind-reader-review` rule computes nothing — it has no `loc-soft`/`loc-hard` pair; it is
  an observation-only relaxation of a blind-reader-review rule, not a line-count ceiling — and yet
  now declares `measure: { kind: level }`, a rule that measures no quantity declaring what kind of
  quantity it measures. Recorded as the question; **not fixed here** — whether every `class:
  measure` rule must carry an actual bound/quantity, or whether a bare `kind:` declaration with no
  bound is a legitimate lighter-weight use of the class, is a design decision this slice does not
  make.

## 4. What slice 2 inherits — the network boundary, explicitly not crossed

Slice 1 (Tasks 1–5 of the `2026-08-11-measure-ontology-slice1-epr-local-first` plan) proves the
ontology holds *locally, single-peer, single-process*. A network rung — sync, gossip, rollup
across peers — is a different set of guarantees, and this section states the line precisely so
slice 2 does not have to re-derive what it may assume and what it must still build.

**Inherited — slice 1 guarantees these hold locally, and slice 2 may build on them without
re-proving them:**

- Every quantity declares its kind (L1) — a rate cannot be constructed without its period.
- A rate carries its period inside the same value as its magnitude (L1).
- Confidence rides inside the canonical bytes at the *serializer* level (L2, per Task 6 Step 1's
  guard: two quantities differing only in confidence produce different dag-cbor bytes, and
  therefore different CIDs under `cid::compute_cid`).
- Folds return intervals, not bare scalars, and take the weakest (widest) claim among their
  inputs (L5, `elohim-epr-rea::fold::with_uncertainty`).
- Honest absence is the degenerate interval — `Interval::unknown()` — not a nullable field or a
  third state alongside measured/estimated (L3).
- A `.epr-meta` measure-tier declaration without a `kind` is refused at the governance gate (L6),
  so the network layer never has to reconcile an undeclared-kind quantity that slipped past local
  policy.

**NOT inherited — slice 2 must solve each of these before any network rung, none of them follow
automatically from the local guarantees above:**

1. **Per-fold anonymity.** A rollup over a sparse holon re-identifies its members — household
   attribution is near-automatic for exactly the measures a network-wide orchestra most wants. A
   k-anonymity floor or a differential-privacy noise budget is needed *per fold level*, and the
   noise budget is itself a depleting stock that wants its own `kind: level` treatment. This
   **blocks the network rung outright** — [commons-holonic-stewardship-backlog](epr:commons-holonic-stewardship-backlog)
   row 10 names it as the unsolved half of row 9.
2. **Correlation-aware interval arithmetic.** Slice 1's naive interval addition
   (`[a,b] + [c,d] = [a+c, b+d]`, Q1) assumes perfect correlation between terms and therefore
   over-widens for independent ones. Over-widening never manufactures false precision, so it is
   safe to ship locally — but it is wasteful once a fold runs at network scale over genuinely
   independent peer contributions, and a correlation-aware fold is explicitly slice-2 work that
   must not be faked here.
3. **Cross-peer determinism at a band edge.** The same ULP-exposure question the `bound_stock`
   determinism decision names (algedonic-phase2 row 5): two peers computing the same fold over
   the same inputs must not disagree on whether an interval crosses a threshold, and floating-point
   interval arithmetic gives no structural guarantee of that today.
4. **Statistical-method application at the ceiling.** Reconciling divergent estimates —
   `ComputationAttestation` at a graduated-rigor tier, where a contested narrow interval escalates
   a tier exactly as a contested clustering does — is elohim-ceiling discernment (Q2), not
   substrate arithmetic. Slice 1 ships the inputs to that reconciliation; it never performs it.
5. **Index lenses.** World3 / GNP / GDP / Donut / planetary-boundary lenses over the aggregate
   dataset ([commons-holonic-stewardship-backlog](epr:commons-holonic-stewardship-backlog) rows 9
   and 13) need items 1–4 above landed first — a lens over ungoverned anonymity, uncorrelated
   arithmetic, non-deterministic band edges, and unreconciled contested estimates would just
   launder those gaps into a headline number.

**Also inherited as an open gap, not a guarantee — the missing typed entry point.** L2 above is
proven at the *serializer* level (Task 6's guard test operates on
`serde_ipld_dagcbor::to_vec(&quantity)` directly) but is explicitly **not wired** to this crate's
own typed API: `cbor::encode` accepts only `&Ipld`, and there is still no
`measure::canonical_bytes(&Quantity) -> Result<Vec<u8>>` (or equivalent `Ipld`-building function,
mirroring `envelope.rs`/`witness.rs`'s hand-built `BTreeMap<String, Ipld>` pattern) anywhere in
this crate. Slice 2 inherits this as unfinished, not as done: **no caller can compute or verify a
`Quantity`'s own CID through the crate's public API today**, and any network-layer code that
needs to address a measure by its canonical bytes must build (or wait for) that entry point
first — it cannot assume the serializer-level proof already gives it a callable function.

## 5. What this spec seals

The six laws above, and no more, are canon as of this document: L1, L3, L4 (with its Q8 caveat),
L5, and L6 (enforced on both the manifest and registry `.epr-meta` paths) are enforced now by
code that exists and is tested. L2 is proven at the dag-cbor serializer level and explicitly not
yet wired to a typed `Quantity` entry point — §4 names that gap as an explicit slice-2
inheritance, not a task this plan still owes. Q1–Q14 are recorded as open rather than smuggled in
as unenforced laws. Tasks 3–6 built against this vocabulary; nothing downstream should re-litigate
L1–L6 without citing back here.
