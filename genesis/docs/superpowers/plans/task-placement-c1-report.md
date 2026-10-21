---
id: placement-c1-report
title: "C1 report — seam-contracts: the ruling-7 unanswered type, status channel and probe reading"
status: DONE_WITH_CONCERNS
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#24
actor: agent:implementer@codex
commits: []
habits: [materialize-what-is-read]
cites:
  - "placement-c1-brief | C1 | sha256:a37374e9c1a50600 | path: genesis/docs/superpowers/plans/task-placement-c1-brief.md"
  - "placement-contract-interfaces-design | The placement contract's interfaces | sha256:de0208f9b6e426db | path: genesis/docs/superpowers/specs/2026-10-20-placement-contract-interfaces-design.md"
  - "placement-contract-architecture-plan | The placement contract's architecture | sha256:5417b3363dffdc44 | path: genesis/docs/superpowers/plans/2026-10-20-placement-contract-architecture-plan.md"
---

Item C1 (gap `#24`), built against interfaces spec §1.2, §1.3, §2.0 and §2.I. Write-set
`crates/seam-contracts/**` only. Following the brief (architecture before landing), nothing is staged,
committed or pushed.

## Gate evidence

`just gate seam-contracts` (from `elohim/epr/build-manifest.json` → recipe `_gate-seam-contracts`:
`cargo test --all-features`, `cargo clippy --all-features -- -D warnings`, `cargo fmt --check`,
`cargo build --target wasm32-unknown-unknown --no-default-features`), cargo berth claimed before and
released after:

- first run: `EXIT=1`. The wasm step failed with `E0463 can't find crate for std` because the pinned
  toolchain 1.96.1 did not have `wasm32-unknown-unknown` installed (only `stable` had it). This is
  the environment, not this change: every earlier step passed.
- `rustup target add wasm32-unknown-unknown --toolchain 1.96.1` (the README's documented one-time
  prerequisite), then `just gate seam-contracts`: **`EXIT=0`**. 143 lib tests, 5 in `tests/probe.rs`,
  5 in `tests/unanswered.rs` and 5 doctests (1 ignored, already ignored before) pass; clippy, fmt and
  the wasm build are clean.
- Extra, outside the gate: `cargo clippy --all-features --all-targets -- -D warnings` (the gate's
  clippy skips test targets): `EXIT=0`. `cargo test` with default features: `EXIT=0` (79 lib tests).
- The boundary test `no_heavy_deps_in_dep_tree` passes and `Cargo.lock` is unchanged (18 packages,
  ceiling 20). No dependency was added.

## Files changed

- `crates/seam-contracts/src/unanswered.rs` (new): `Unanswered<R>`, `ComeBack`, `Meanwhile`,
  `Escalation`, `PendingStatus<T, R>`, `StatusCursor`; `new`, `escalate`, `map_why` as in §2.0.
- `crates/seam-contracts/src/probe.rs` (new): `ProbeAnswer<T>`, `UnreachableReason`, `Reading<T>`,
  `EvidenceStage`, `Disclosure`, `PartyRole`, `Probe`, `HolderSink`, `Declassified`, as in §2.I.
- `crates/seam-contracts/src/harness/arbitrated.rs`: the list form of the arbitration harness,
  `check_ordering`, `check_ordering_with`, `assert_ordering`, `OrderingViolation`,
  `OrderingViolationClass` (§1.2 row "the list form of the arbitration harness", §D.5).
- `crates/seam-contracts/src/harness/mod.rs`, `src/lib.rs`: module declarations, re-exports and the
  concern table rows.
- `crates/seam-contracts/tests/unanswered.rs`, `tests/probe.rs` (new): the contract tests §2.0 and
  §2.I name.
- `crates/seam-contracts/seam-registry.yaml`: two rows under the crate's own birth rule,
  `Unanswered` and `ProbeAnswer` (`boundary-answer-type`), validated against
  `elohim/sdk/schemas/v1/manifest/seam-registry.schema.json` with `jsonschema` (`EXIT=0`).

Everything new uses only `String` and `Vec` from alloc (the harness's `std::error::Error` impl
follows the existing `ArbitrationViolation`). Nothing reads a clock: `now_epoch_secs` and
`observed_at` come from the caller.

## Tests added

- In `unanswered.rs`: `new_has_no_escalation`, `escalate_counts_asks_and_keeps_the_three_parts`,
  `escalate_saturates`, `map_why_keeps_the_other_parts`, `label_is_the_reason_label`,
  `come_back_and_meanwhile_carry_their_payloads`, `only_answered_and_withdrawn_are_terminal`,
  `wire_convenience_derives_exist` (serde).
- In `tests/unanswered.rs`: `every_reason_type_used_in_unanswered_is_a_conformant_reason_label`,
  `a_cursor_whose_inputs_did_not_change_returns_none`,
  `a_cursor_reaches_a_terminal_state_only_through_answered_or_withdrawn`,
  `withdrawal_is_terminal_and_never_read_as_answered`, `the_cursor_is_object_safe`.
- In `probe.rs`: `unreachable_reason_refuses_observed_absent`,
  `unreachable_constructor_refuses_observed_absent`, `probe_answer_states_and_reasons_pair`,
  `reading_carries_every_part`, `evidence_stage_labels_are_stable`, `party_role_labels_are_stable`,
  `disclosure_distinguishes_its_forms`, `declassified_carries_counts_only`,
  `wire_convenience_derives_exist` (serde).
- In `tests/probe.rs`: `a_probe_with_nothing_to_read_returns_none`,
  `an_unreachable_answer_always_carries_a_transport_reason`, `holder_readings_reach_only_a_holder_sink`,
  `evidence_is_required_and_fixture_is_not_real`, `every_label_a_probe_counts_is_conformant`.
- In `arbitrated.rs`: `total_ordering_passes`, `ordering_permutation_dependent_is_caught` (this one
  also shows that `check_arbitration` passes the same tie-in-second-place ranker that the list form
  catches), `ordering_nondeterministic_is_caught`, `ordering_foreign_is_caught`,
  `ordering_duplicated_is_caught`, `ordering_not_empty_from_empty_is_caught`,
  `ordering_budget_caps_the_walk`, `ordering_assert_form_panics_on_violation`,
  `ordering_violation_class_is_conformant`.

## Where the code and the interfaces spec disagreed (for §6 to absorb)

1. **§2.I `ProbeAnswer::Unreachable(AnswerReason)` lets the wrong shape be written.** The spec says
   Unreachable-with-the-wrong-reason "cannot be written". But because the variant is public, anyone
   can build `ProbeAnswer::Unreachable(AnswerReason::ObservedAbsent)` without going through
   `unreachable()`, and so can a derived `Deserialize`. Revised to
   `Unreachable(UnreachableReason)`: a newtype whose field is private. Its constructor
   `UnreachableReason::new`, its `TryFrom<AnswerReason>` and its serde `try_from` all refuse
   `ObservedAbsent`. `ProbeAnswer::unreachable(reason: AnswerReason) -> Result<Self, AnswerReason>`
   keeps the spec's signature. On the wire it is still a bare `AnswerReason`.
2. **§2.0's contract "every seam's reason type is a `ReasonLabel`" does not fit the seams' own
   reasons.** `Unanswered<R>` puts no bound on `R` (correctly, because `ItemAbsence`,
   `IneligibleReason { input }` and `ParamRefusal::Unverifiable { input }` carry data, and a
   `ReasonLabel` must be fieldless with a total `ALL`). §1.2 already gives eprfs-serve
   `ItemAbsenceLabel` as a separate face. Suggested restatement: "every seam's reason type is a
   `ReasonLabel`, or exposes a fieldless `ReasonLabel` face". Added
   `impl<R: ReasonLabel> Unanswered<R> { fn label(&self) -> &'static str }` for the fieldless case.
3. **The spec does not say what `Escalation.asked` counts.** Chosen: it counts asks with this one
   included, so the first `escalate` sets 2, each later one adds 1, and it saturates at `u32::MAX`.
   On each escalation, `who_can_be_asked` and `changed` replace the earlier values rather than
   merging with them.
4. **§1.2 names the list form of the arbitration harness without giving it a signature.** It is built
   as `check_ordering(candidates: &[C], order: impl Fn(&[C]) -> Vec<C>) -> Result<ArbitrationReport,
   OrderingViolation>`, using the existing `ArbitrationBudget`/`ArbitrationReport` as §D.5 says. It
   checks that empty input gives empty output, that the output is a sub-multiset of the input, that
   repeated calls agree, and that every input ordering gives the same output sequence. The output is
   candidates, not indices, so placement-core's `assert_ranking_contract` maps `Ranking::rank`'s index
   output back to candidates, and keeps the weight clauses for itself. Its failure classes live in a
   new `OrderingViolationClass` instead of new variants on `ArbitrationViolationClass`, because that
   published enum is exhaustive and new variants would break downstream `match`es.
5. **Additions the spec does not name** (all additive): `PendingStatus::is_terminal`;
   `ProbeAnswer::{state, reason}`, where `reason().state() == state()`; `Reading::is_holder_only`;
   and `ReasonLabel` impls for `EvidenceStage` and `PartyRole`, whose golden labels equal the
   kebab-case values (`fixture`/`real`, `carried-for`, and so on) that
   `enums/evidence-stage.schema.json` is meant to carry. The `StatusCursor` rustdoc states that a
   cursor returns `None` forever after a terminal status.
6. **`Declassified` has public fields, as the spec gives it**, so the type cannot itself hold to
   "constructed only by a holder-facing command". The rustdoc says where that rule has to be held.

## Concerns

- **No serde wire round trip is tested in this crate.** A JSON dev-dependency would add about four
  packages to a lockfile that holds 18 against a ceiling of 20. The tests therefore assert only that
  the derives exist. The wire shapes are owned by lane S's `objects/unanswered.schema.json` and
  `objects/probe-reading.schema.json`. Whatever schema contract test compares them with these
  derives belongs outside this leaf.
- **The holder-only routing test uses doubles.** It pins the rule a host applies. The real writers
  (`.eprfs/status`, loopback gauges) must assert it again when they adopt `Reading<T>`.
- **Gate environment:** the pinned 1.96.1 toolchain lacked the wasm32 target that the gate's last
  step needs. It is installed now in this workspace, but a fresh workspace will go red at that same
  step until the devfile or `just status device` provisions it.
- An early manual `cargo test` run without `CARGO_TARGET_DIR` left an 85 MB
  `crates/seam-contracts/target/` behind. It is gitignored and was left in place.
- Docs not touched: the crate's `README.md` does not yet list the two new modules.

## Fix round 1

I re-checked all four review findings against the code, and all four held. I declined two
suggestions that came with them; the reasons are under "Declined". The concern "The holder-only
routing test uses doubles" above is now resolved, by item 2 below.

### Fixed

1. **A party-scoped reading reached the public gauge** (tests/probe.rs `route`). The test-local
   `if is_holder_only() { holder } else { public }` sent `Disclosure::Parties` to the public
   writer. Fixed in the crate: `Reading::route(self) -> Routed<T>` sorts a reading into one of
   `Routed::{Holder(HolderReading<T>), Parties(PartyReading<T>), Shared(SharedReading<T>)}`, and
   only `AtReach` becomes `Shared`. The test now asserts that a Parties reading reaches neither the
   holder file nor the gauge, and that only an `AtReach` reading reaches the gauge.
2. **The holder-only contract test checked its own copy of the rule, and `HolderSink` enforced
   nothing** (findings 2 and 4).
   - `HolderReading`, `PartyReading` and `SharedReading` are newtypes with private fields. Only
     `route` builds them.
   - `HolderSink<T>::write_holder(&mut self, &HolderReading<T>)` takes only a holder reading. A new
     `SharedSink<T>::write_shared(&mut self, &SharedReading<T>)` takes only a shared one.
   - The test doubles now implement the crate's sink traits, so the routing under test is the
     crate's own code.
   - Compile_fail doctests pin the split. `SharedSink::write_shared` refuses a `&HolderReading`,
     and a compiling counterpart shows it takes a `&SharedReading`. A `HolderReading` cannot be
     built outside the crate.
   - `Declassified` has private fields with accessors. Its only constructor is
     `HolderReading::declassify(&self, counts, chosen_by)`, which takes the promise from the
     reading. A compile_fail doctest shows that a struct literal is refused.
   - `Declassified` now derives `Serialize` only. Deserializing would rebuild one without a holder
     reading in hand.
   - Registry: the ProbeAnswer row now lists `route_sorts_by_disclosure` and
     `declassified_comes_only_from_a_holder_reading`. Its notes describe the split by type and no
     longer say "re-assert in the real writers".
3. **The status-channel mechanism existed only in a test** (finding 3).
   - Added `StatusSource`, a seam's pure `status(&self, now) -> PendingStatus`.
   - Added `Changes<S>`, a `StatusCursor` that by construction reports only a changed status and
     returns `None` forever after a terminal one. A seam changes its inputs through `source_mut()`.
     I chose a source trait over an `FnMut` closure so that a seam can change its inputs between
     polls without a shared cell.
   - For seams that write their own cursor, added `harness::status::{check_status_cursor,
     assert_status_cursor, StatusCursorReport, StatusCursorViolation, StatusCursorViolationClass}`.
     At each step the check applies the seam's change, polls, then polls again at the same time
     with no change in between. It refuses `repolled_without_change`, `reported_unchanged` and
     `spoke_after_terminal`.
   - tests/unanswered.rs now runs its clauses over `Changes<Inputs>` in place of a hand-written
     cursor, and runs the same script through the harness.
   - Registry: Unanswered's C3 moves from `partial` to `answered` for the channel's contract, and
     the row lists the harness tests.
4. **Tautological tests** (finding 4).
   - `evidence_is_required_and_fixture_is_not_real` is deleted. In its place, `Reading` has a
     compile_fail doctest showing that a struct literal without `evidence` is refused, with a
     compiling counterpart.
   - `declassified_carries_counts_only`, which only summed a vec, is replaced by
     `declassified_comes_only_from_a_holder_reading`.

### Declined

- **`Reading::may_reach_public()`** (offered as optional in finding 1). A boolean predicate brings
  back the `if pred { a } else { b }` pattern that caused the leak. `route()` answers the same
  question as a type the sinks check. `is_holder_only` stays, for inspection only.
- **A `PartySink` trait.** No consumer delivers party-scoped readings yet, and that delivery belongs
  to the seam that carries for the parties. `PartyReading` exists so that neither sink can be
  handed one, and its `roles()` says who may read it.

### Concern: what a type still cannot stop

`Reading<T>`'s fields stay public, as the spec gives them. A producer can still build a reading with
a false `disclosure`, and a writer can copy fields out of `reading()` into another channel. The split
holds the sink API to "the wrong call cannot be written". It cannot stop someone copying the data
out by hand.

### Spec revisions for §6 to absorb (fix round 1)

- §2.I: `HolderSink { fn write_holder(&mut self, promise: &str, bytes: &[u8]) }` becomes
  `HolderSink<T> { fn write_holder(&mut self, reading: &HolderReading<T>) }`, and the sink encodes
  the reading itself. Taking bytes let any sink be handed any encoded reading.
- §2.I adds `Reading::route`, `Routed<T>`, `HolderReading<T>`, `PartyReading<T>`,
  `SharedReading<T>` and `SharedSink<T>`.
- §2.I: `Declassified` has private fields, is built only by `HolderReading::declassify`, and derives
  `Serialize` only. The type now holds the spec's "constructed only by a holder-facing command".
- §2.0 adds `StatusSource`, `Changes<S>` and the `harness::status` check. Two rules move from each
  seam's own code into the crate's mechanism: "a cursor whose inputs did not change returns `None`"
  and "nothing after terminal".

### Files changed in this round

- `crates/seam-contracts/src/probe.rs`
- `crates/seam-contracts/src/unanswered.rs`
- `crates/seam-contracts/src/lib.rs`
- `crates/seam-contracts/src/harness/mod.rs`
- `crates/seam-contracts/src/harness/status.rs` (new)
- `crates/seam-contracts/tests/probe.rs`
- `crates/seam-contracts/tests/unanswered.rs`
- `crates/seam-contracts/seam-registry.yaml`, validated against `seam-registry.schema.json` with
  `jsonschema`: `EXIT=0`

No dependency was added and `Cargo.lock` is unchanged.

### Gate evidence (fix round 1)

I claimed the cargo berth before the runs (`berth claim cargo --class verify --ttl 3600`, `EXIT=0`)
and released it after (`EXIT=0`).

1. `just gate seam-contracts`, first run: `EXIT=1`. Only `cargo fmt --check` failed, on
   `src/harness/status.rs` and `tests/probe.rs`. The tests and clippy had passed.
2. `cargo fmt --manifest-path crates/seam-contracts/Cargo.toml`: `EXIT=0`.
3. `just gate seam-contracts`: **`EXIT=0`**. Passed: 150 lib tests, 4 in `tests/probe.rs`, 6 in
   `tests/unanswered.rs`, and 11 doctests, 4 of them compile_fail. One doctest is ignored, as it
   was before this round. Clippy, fmt and the wasm build are clean.

Outside the gate:

- `cargo clippy --all-features --all-targets -- -D warnings`: `EXIT=0`.
- `cargo test` with default features: `EXIT=0` (80 lib tests).

## Fix round 2

I re-checked all four findings against the code. Findings 1 and 2 are the same defect and both
held. Finding 3 held. Finding 4 held as a description of the drift, but its fix, editing the
interfaces spec, is outside this lane's write-set, so I refused that part and did the rest in this
report and in the code.

### Fixed

1. **A cursor that never speaks passed the status harness** (findings 1 and 2). I confirmed it:
   `check_status_cursor` returned errors only when a poll returned `Some`, so an always-`None`
   cursor got `Ok(StatusCursorReport { statuses: 0, .. })`, and so did a cursor that reported
   `Queued` and then never reported `Answered`. I took finding 2's shape, because finding 1's
   `bool` closure would say only that *something* changed, not what the cursor should report.
   - `check_status_cursor` and `assert_status_cursor` take a fourth argument,
     `expected: Fn(&C, u64) -> PendingStatus<C::Value, C::Reason>`. It gives the status the
     cursor's inputs hold at `now`, after that step's `change`.
   - The first poll of each step must return `Some(expected)` when `expected` differs from the
     last status reported, or when nothing has been reported yet. It must return `None` when they
     are equal. After a terminal status every poll must return `None`, whatever `expected` says.
   - There are two new failure classes on `StatusCursorViolationClass`:
     - `SilentAfterChange` (`silent_after_change`): the status changed and the first poll after
       the change returned `None`.
     - `ReportedOtherThanExpected` (`reported_other_than_expected`): the cursor reported a status
       that its inputs do not hold.

     Adding variants to this enum is safe because it was born in round 1 and has no consumer
     outside the crate.
   - New tests:
     - `a_cursor_that_never_speaks_is_caught`: an always-`None` `Silent` cursor fails at step 0.
     - `a_missed_answer_is_caught`: a cursor that reports `Queued` and then stays silent fails at
       the step where its inputs became `Answered(4)`.
     - `reported_other_than_expected_is_caught`.
   - `assert_form_panics_on_violation` now panics on the silent cursor. The four existing tests
     and the `Changes` runs (`changes_passes_the_harness` and
     `tests/unanswered.rs::changes_over_a_seam_source_passes_the_status_harness`) pass
     `|c, now| c.source().status(now)` as `expected`.
   - Registry: Unanswered's C3 stays `answered`. Its justification now names the liveness half,
     lists the two new tests, and says the check is only as good as the `expected` the seam's test
     supplies. The module doc and the `lib.rs` concern-table row say the same.
2. **An existing reading could be relabelled with a wider disclosure** (finding 3). I confirmed
   it: `Reading { disclosure: AtReach { .. }, ..held.clone() }` compiled from outside the crate,
   and so did `r.disclosure = ..`. I took the finding's first option.
   - `Reading.disclosure` is now a private field. It is set only by the new constructor
     `Reading::new(promise, concern, subject, answer, evidence, disclosure, observed_at)` and read
     by `Reading::disclosure() -> &Disclosure`. The other six fields stay public, as the spec
     gives them.
   - Two compile_fail doctests on `Reading` pin the change. One shows the struct-update relabel
     from a clone is refused; the other shows the in-place assignment is refused. A compiling
     counterpart builds a reading with `Reading::new`.
   - The round-1 compile_fail doctest about a missing `evidence` field is gone. Every literal now
     fails because of the private field, so that test would pass for the wrong reason. Evidence
     and disclosure are both required arguments of `Reading::new`, and neither type has a
     default.
   - `tests/probe.rs` now builds its party-scoped and at-reach readings with a `rebuilt` helper
     that calls `Reading::new`. That is a producer declaring a new reading, not a relabel.
   - I also narrowed the claim. The registry's ProbeAnswer notes, the `Reading` rustdoc and the
     `lib.rs` row now say that routing follows the disclosure a reading was built with. They also
     say that a producer who builds a new reading from copied fields declares a new disclosure,
     and that nothing checks a disclosure against the promise's row in spec §9.
3. **Spec-shaped code will not compile against the shipped types** (finding 4, the part inside
   this write-set). The spec's example `ProbeAnswer::Unreachable(r) => r.label()` failed because
   `UnreachableReason` had no `label()`. `UnreachableReason` now implements `ReasonLabel`: `ALL`
   is the five transport reasons in `AnswerReason`'s order, and each label is the label of the
   reason inside. New test: `unreachable_reason_labels_are_answer_reason_labels`, which is listed
   in the registry. `tests/probe.rs::every_label_a_probe_counts_is_conformant` also checks it.

### Refused

- **Editing the interfaces spec (§2.0, §2.I, §1.2's ownership row and §6)** (finding 4's fix).
  The brief's write-set is `crates/seam-contracts/**`, and the brief says how to handle this
  case: "where the code proves a signature wrong, fix the code to the spec's intent and record the
  revision in your report so the spec can absorb it". Editing
  `genesis/docs/superpowers/specs/2026-10-20-placement-contract-interfaces-design.md` from this
  seat would go outside the write-set. I agree with the finding's substance: lanes E and S and
  placement-core read the spec, and until §6 absorbs these revisions they will write code against
  shapes that are gone. That makes the spec revision a prerequisite for those lanes, and it
  belongs to the seat that owns the spec. The complete list of revisions is the table below, so
  that seat need not reconstruct it from three rounds of notes.

### Spec revisions for §6 to absorb (all rounds, consolidated)

| Spec section | Spec gives | Shipped | Round |
|---|---|---|---|
| §2.I | `ProbeAnswer::Unreachable(AnswerReason)` | `Unreachable(UnreachableReason)`: a private-field newtype that refuses `ObservedAbsent` in `new`, `TryFrom` and serde; on the wire it is still a bare `AnswerReason`; it implements `ReasonLabel` | 0, 2 |
| §2.I | `Reading<T>` with every field public | `disclosure` is private; adds `Reading::new(..7 args)` and `Reading::disclosure()`; the other fields stay public | 2 |
| §2.I | (not named) | `Reading::route(self) -> Routed<T>`; `Routed::{Holder, Parties, Shared}`; private-field newtypes `HolderReading<T>`, `PartyReading<T>` (`roles()`) and `SharedReading<T>` (`openness()`) | 1 |
| §2.I | `trait HolderSink { fn write_holder(&mut self, promise: &str, bytes: &[u8]); }` | `trait HolderSink<T> { fn write_holder(&mut self, reading: &HolderReading<T>); }` | 1 |
| §2.I | (not named) | `trait SharedSink<T> { fn write_shared(&mut self, reading: &SharedReading<T>); }` | 1 |
| §2.I | `Declassified` with public fields | private fields with accessors; built only by `HolderReading::declassify(&self, counts, chosen_by)`; `Serialize` only | 1 |
| §2.I | (not named) | `ProbeAnswer::{state, reason}`, `Reading::is_holder_only`, and `ReasonLabel` for `EvidenceStage` and `PartyRole` | 0 |
| §2.0 | "every seam's reason type is a `ReasonLabel`" | `Unanswered<R>` has no bound on `R`; `Unanswered::label` exists where `R: ReasonLabel`; a seam with a data-bearing reason exposes a fieldless face | 0 |
| §2.0 | `Escalation.asked` unspecified | counts asks including this one: the first `escalate` sets 2, and the count saturates | 0 |
| §2.0 | (not named) | `PendingStatus::is_terminal`; `trait StatusSource`; `Changes<S>` (a `StatusCursor` that reports changes only and stops at terminal) | 0, 1 |
| §2.0 | (not named) | `harness::status::check_status_cursor(cursor, times, change, expected)` and `assert_status_cursor`, with five violation classes, including `silent_after_change` | 1, 2 |
| §1.2 | the list form of the arbitration harness, no signature given | `check_ordering(candidates, order) -> Result<ArbitrationReport, OrderingViolation>`; `check_ordering_with`; `assert_ordering`; `OrderingViolationClass` | 0 |
| §1.2 ownership row | names `Unanswered`, `PendingStatus`, `StatusCursor`, `ProbeAnswer`, `Reading`, `HolderSink`, `Declassified` | add `UnreachableReason`, `Routed`, `HolderReading`, `PartyReading`, `SharedReading`, `SharedSink`, `StatusSource`, `Changes` and `harness::status` | 2 |

### Files changed in this round

- `crates/seam-contracts/src/harness/status.rs`
- `crates/seam-contracts/src/probe.rs`
- `crates/seam-contracts/src/lib.rs` (concern-table rows)
- `crates/seam-contracts/tests/probe.rs`
- `crates/seam-contracts/tests/unanswered.rs`
- `crates/seam-contracts/seam-registry.yaml`, including ProbeAnswer's `sourceLocation.line`
  moving from 112 to 133. Validated against `seam-registry.schema.json` with `jsonschema`:
  `EXIT=0`.

No dependency was added and `Cargo.lock` is unchanged.

### Gate evidence (fix round 2)

I claimed the cargo berth before the runs (`berth claim cargo --class verify --ttl 3600`, `EXIT=0`)
and released it after (`berth release cargo`, `EXIT=0`).

1. `cargo fmt --manifest-path crates/seam-contracts/Cargo.toml`: `EXIT=0`.
2. `just gate seam-contracts`: **`EXIT=0`**. Passed: 154 lib tests, 4 in `tests/probe.rs`, 6 in
   `tests/unanswered.rs`, and 12 doctests, 5 of them compile_fail. One doctest is ignored, as
   before. `cargo fmt --check` and the wasm32 build are clean.

Outside the gate:

- `cargo clippy --all-features --all-targets -- -D warnings`: `EXIT=0`.
- `cargo test` with default features: `EXIT=0` (81 lib tests).

## Codex review 1 fixes

I re-checked all five findings against the code. Findings 2 and 3 held, and I fixed them. Finding 4
held, but its full fix would break placement-core, which is outside this lane's write-set, so I did
the part inside the lane and refused the rest. Finding 1 asks for no code change, and its spec edit
is outside the write-set. Finding 5's premise is false: seam-contracts has never been published.

### Fixed

1. **Deserializing a `Reading` skipped `Reading::new`** (finding 2). Confirmed: `Reading<T>`
   derived `Deserialize` with `disclosure` private, so a holder reading could be serialized, its
   `disclosure` edited, and deserialized back as a shared reading. I took the finding's first
   option.
   - `Reading<T>` now derives `Serialize` only, the same as `Declassified`. placement-core's
     `RealEvidence::from_readings` serializes readings but never deserializes them, and grep finds
     no other consumer that deserializes one.
   - A new compile_fail doctest on `Reading` shows that
     `<Reading<u8> as serde::Deserialize>::deserialize(d)` does not compile.
   - `wire_convenience_derives_exist` now checks `ser::<Reading<u32>>()` instead of `both`.
   - The claim that a disclosure is "set only by `Reading::new`" now holds under
     `--all-features`, and the rustdoc and registry notes say why.
2. **Every `AtReach` reading reached any `SharedSink`** (finding 3). Confirmed: `route` sent every
   `AtReach { openness }` to `Shared` whatever its openness, including 0 and 255, and `PublicGauge`
   in `tests/probe.rs` exported a trusted-reach (4) reading. That left the reach comparison to each
   adapter sink. I took the finding's first option, made it mandatory, and added the range check
   from its second option.
   - `SharedSink<T>` now has `fn audience_openness(&self) -> u8`, the openness of its most open
     reader. Its `write_shared` takes `&Admitted<'_, T>`, a private-field wrapper that only
     `SharedReading::deliver` builds.
   - `SharedReading::deliver(&self, sink) -> Result<(), ShareRefusal>` compares the reading's
     reach with the sink's audience, and when it refuses it writes nothing. It refuses:
     - `ReachOutOfRange` when the reading's openness is outside 1..=8;
     - `AudienceOutOfRange` when the sink's audience is outside 1..=8;
     - `AudienceWiderThanReach` when the sink's readers are more open than the reading's reach.
   - `ShareRefusal` is a `ReasonLabel`, so each refusal is counted under its own label.
   - New constants: `OPENNESS_MIN` (1), `OPENNESS_MAX` (8) and `PUBLIC_OPENNESS` (7). A public
     gauge declares `PUBLIC_OPENNESS`. The leaf boundary forbids depending on elohim-epr, and
     editing elohim-epr is outside the write-set, so the pin is
     `tests/probe.rs::openness_ordinals_match_the_reach_schema`. That test reads
     `elohim/sdk/schemas/v1/enums/reach.schema.json`, the file elohim-epr's
     `openness_matches_generated_ordinal` pins `Reach::openness()` to, so the two crates agree
     through the schema.
   - Two new compile_fail doctests: a direct `sink.write_shared(shared_reading)` does not compile,
     and neither does building `Admitted(r)`. The doctest that compiles now calls
     `r.deliver(sink)`.
   - New tests:
     - `deliver_compares_the_reach_with_the_audience`: every openness below public, public and
       commons, out-of-range readings, out-of-range audiences, and a sink with a trusted
       audience;
     - `share_refusal_labels_are_stable`;
     - `tests/probe.rs::a_shared_reading_reaches_only_a_sink_inside_its_reach`: the public gauge
       refuses a trusted reading and openness 0 and 9, and exports a commons reading;
     - `tests/probe.rs::openness_ordinals_match_the_reach_schema`.

     `holder_readings_reach_only_a_holder_sink` now delivers its at-reach reading at public
     openness instead of 4.
   - Claim narrowed in the registry notes: a sink's `audience_openness()` is its own declaration,
     and a sink that forwards an `Admitted` it was handed to a second sink is outside what the type
     stops.

### Fixed in part

3. **`evidence` and `answer` were still `pub`** (finding 4). Confirmed: on a `&mut Reading`,
   `r.evidence = EvidenceStage::Real` and `r.answer = ProbeAnswer::Present(..)` compile.
   - Done: getters `promise()`, `concern()`, `subject()`, `answer()`, `evidence()` and
     `observed_at()`. This crate's tests and doctests read only through them. In the registry,
     ProbeAnswer's C5 is now `partial` (it was `answered`), and its justification names the open
     hole. The `Reading` rustdoc and the `lib.rs` C4 + C5 row say the same.
   - Refused in this round: making the fields private.
     `elohim/placement/core/src/adopt.rs` (`RealEvidence::from_readings`, lines 236-261) reads
     `r.evidence`, `r.promise`, `r.subject`, `r.answer` and `r.observed_at` as fields, so private
     fields would break placement-core. This round is limited to the lane's write-set.
   - What is left: once placement-core reads through the getters (each `r.x` becomes `r.x()`),
     the rest is one edit in this crate. Drop `pub` from the six fields, add compile_fail doctests
     for `r.evidence = EvidenceStage::Real` and for a struct-update relabel, and set C5 back to
     `answered`. That edit belongs to a seat that holds both write-sets.

### Refused

- **Finding 1 (spec signatures).** It asks for no code change. Absorbing the revision table into
  §2.I, §2.0 and §1.2 belongs to the seat that owns the spec, as fix round 2's refusal says. This
  round adds rows to that table, below.
- **Finding 5 (version bump).** Its premise is false. In the `elohim` sparse index,
  `el/oh/elohim-seam-contracts` returns 404, while `elohim-epr` and `eprfs-core` return 200. So no
  0.1.0 without these modules exists in the registry.
  - The published `eprfs-core` versions (0.1.0, 0.1.1 and 0.1.2) have no first-party
    dependency, so no published consumer can resolve a stale seam-contracts.
  - `cargo publish` refuses an eprfs-core that names `registry = "elohim"` for seam-contracts
    until seam-contracts itself is published, and that first publish will carry these modules.
  - A bump now would change the `elohim-seam-contracts` entry in at least eight lockfiles outside
    this write-set, and every `--locked` gate over them would refuse until each one is refreshed.
    They are `elohim/Cargo.lock`, `elohim/eprfs/Cargo.lock`, `elohim/elohim-storage/Cargo.lock`,
    `doorway/doorway-service/Cargo.lock`, `steward/node/Cargo.lock`, `bridges/k8s/Cargo.lock`,
    `crates/elohim-sdk/Cargo.lock` and `elohim/holochain/dna/elohim/Cargo.lock`.
  - The finding's rule applies once 0.1.0 is in the registry: from then on, the next API addition
    bumps the version in the same change.

### Spec revisions for §6 to absorb (Codex review 1, adding to the consolidated table)

| Spec section | Spec gives | Shipped |
|---|---|---|
| §2.I | `Reading<T>`: `Serialize + Deserialize` | `Serialize` only |
| §2.I | `Reading<T>` with every field `pub` | every field except `disclosure` is still `pub`, and all seven have getters; the six become private once placement-core reads through the getters |
| §2.I | (not named) | `trait SharedSink<T> { fn audience_openness(&self) -> u8; fn write_shared(&mut self, reading: &Admitted<'_, T>); }`; `SharedReading::deliver(&self, sink) -> Result<(), ShareRefusal>`; `Admitted<'a, T>`; `ShareRefusal::{ReachOutOfRange, AudienceOutOfRange, AudienceWiderThanReach}` (a `ReasonLabel`); `OPENNESS_MIN`, `OPENNESS_MAX`, `PUBLIC_OPENNESS` |
| §1.2 ownership row | as in fix round 2 | add `Admitted`, `ShareRefusal` and the three openness constants |

### Files changed in this round

- `crates/seam-contracts/src/probe.rs`
- `crates/seam-contracts/src/lib.rs`: re-exports, and the C4 + C5 and C1 + C7 concern-table rows.
- `crates/seam-contracts/tests/probe.rs`
- `crates/seam-contracts/seam-registry.yaml`: in the ProbeAnswer row, C5 is now `partial`, there
  are four more contract tests, and the notes are revised. Validated against
  `seam-registry.schema.json` with `jsonschema`: `EXIT=0`.

No dependency was added. `Cargo.toml` and `Cargo.lock` are unchanged.

### Gate evidence (Codex review 1)

I claimed the cargo berth before the runs (`berth claim cargo --class verify --ttl 3600`,
`EXIT=0`) and released it after (`berth release cargo`, `EXIT=0`).

1. `cargo fmt --manifest-path crates/seam-contracts/Cargo.toml`: `EXIT=0`.
2. `just gate seam-contracts`: **`EXIT=0`**.
   - Passed: 156 lib tests, 6 in `tests/probe.rs`, 6 in `tests/unanswered.rs`, and 15 doctests,
     8 of them compile_fail. One doctest is ignored, as before.
   - `cargo clippy --all-features -- -D warnings`, `cargo fmt --check` and the wasm32 build are
     clean.

Outside the gate:

- `cargo clippy --all-features --all-targets -- -D warnings`: `EXIT=0`.
- `cargo test` with default features: `EXIT=0` (83 lib tests, 6 and 5 integration tests, 14
  doctests).
- A scratch crate built with `features = ["serde"]` confirmed that each new compile_fail doctest
  fails for its intended reason:
  - E0277: `Reading<u8>: Deserialize` is not satisfied.
  - E0308: `write_shared` was given a `&SharedReading`.
  - E0423: `Admitted` has a private field.

  The crate's positive `ProbeAnswer` deserialize compiled. `cargo check` returned `EXIT=101` on
  exactly those three errors, and the scratch crate was then removed.
- Consumer check: `just gate elohim-placement` returned `EXIT=0`. placement-core compiles and its
  tests pass now that `Reading` is Serialize-only and has getters.

### Concern

- C5 stays `partial` until placement-core moves to the getters (see "Fixed in part"). Until then a
  fixture reading can be relabelled real in place, and placement-core's run-time `evidence` check
  in `from_readings` cannot detect it.

## Codex integration, 2026-10-21

Resumed by `agent:implementer@codex` from the interrupted workflow at base `6cc1a17d5`.
The preceding concern is now fixed across the two owning crates: placement-core's
`RealEvidence::from_readings` uses the existing getters, and every `Reading<T>` field is private.
The three added compile-fail examples reject evidence reassignment, answer replacement and a
struct-update evidence relabel. The registry's C5 answer and the public concern table now describe
that enforced boundary. Constructing a new reading still requires an honest producer; field
privacy does not authenticate a producer's evidence.

The wire-view documentation points at `crates/elohim-views` and explicitly names the per-promise
wire structs as future work. No wire view or deserialization path was added. The interfaces spec's
§2.I and §6 absorb the private-field/getter shape; this supersedes the earlier table's temporary
public-field exception.

Files in this integration: `src/probe.rs`, `src/lib.rs`, `seam-registry.yaml`, and the companion
consumer edit in `elohim/placement/core/src/adopt.rs` (reported with C2).
`seam-registry.yaml` validates against the existing registry schema (`EXIT=0`).
Fresh integration evidence: `just gate seam-contracts` returned **`EXIT=0`** on 2026-10-21.
It passed 156 unit tests, 12 integration tests and 18 doctests (one ignored), including all three
new compile-fail examples. Clippy with warnings denied, formatting and the no-default-features
WASM build passed. The durable receipt is
`genesis/local-dev/placement-codex-handoff/seam-contracts-gate.log`.
The final storage gate also compiled the placement consumer with these private fields and passed
(`c5-gate-3.log`, `EXIT=0`); C2 records its focused placement gate separately.
Earlier gate results above belong only to their named rounds.

Status remains **DONE_WITH_CONCERNS**: the constructor still trusts the producer's declared evidence,
and per-promise wire views plus the dependent publication/licensing decision remain follow-ons.
The native construction and disclosure boundary is enforced; no runtime attestation claim is made.
