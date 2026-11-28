---
id: feedback-contract-fc4-reading-and-summary-draft
title: Feedback contract, FC-4 — draft shapes for the reading of a claim and the summary that may leave a device
date: 2026-11-27
status: DRAFT
steward: human:matthew
author: shem session 3d494616 (Claude Fable 5.2), called the designing session below, orchestrating and checking; schemas, cases, fixtures and the check by Codex (astra) in two rounds
serves: [private-feedback-loop]
cites:
  - "feedback-loop-contract-plan | the plan whose station FC-4 this draft answers | sha256:a7d1d8a98839c206 | path: genesis/docs/superpowers/plans/2026-11-27-feedback-loop-contract-plan.md"
  - "feedback-as-a-lifecycle-item-design | the design whose sections 5.5 and 6 these schemas and cases put into a form that can be run | sha256:4f57fdff063a4d6c | path: genesis/docs/superpowers/specs/2026-11-26-feedback-as-a-lifecycle-item-design.md"
  - "feedback-contract-fc2-declaration-draft | the sibling draft whose claim-id pattern and test conventions this one reuses | sha256:4f0f5246e0b18034 | path: genesis/docs/superpowers/sprints/2026-11-27-feedback-contract/fc2-declaration-draft.md"
---

# FC-4: draft shapes for the reading of a claim and the summary that may leave a device

This is the draft deliverable of station FC-4 of the feedback loop contract plan. The feedback
lifecycle design describes two things in prose and tables that every consumer has to agree on.
The first is the *reading* of a claim: which of nine words a device's judge gives one problem at
one moment, such as `open`, `on-watch` or `reopened`. The second is the *summary*: the one small
record about a claim that may leave a device. This directory turns both into closed shapes, and
turns the rule for choosing a reading into cases that any implementation can run. The two are
linked: a reading is the judge's verdict on the device, and the summary's `state` is what that
verdict lets the device say to others, by the mapping in `fc4/reading-to-state.json`.

Two words used throughout. A *claim* is one named problem, or one named command, that a tool
declares it can report. An *emitter* is the exact release of the tool that reported it.

It is a draft and it is not in the SDK. Where these schemas live is the steward's decision, and he has
not ruled. The question is decision 2 of section 12 of the lifecycle design's companion,
`genesis/docs/superpowers/specs/2026-11-27-one-loop-for-every-producer-design.md`. No code produces a reading or a
summary yet: the judge and the summary are stations still to be built, and these files are what
they would be built to.

## What is here

| File | What it is |
|---|---|
| `fc4/term-reading.draft.schema.json` | One reading: a claim, the release that reported it, and one of the nine readings |
| `fc4/claim-summary.draft.schema.json` | One summary: nine fields every summary carries, and two counts that depend on its state |
| `fc4/reading-to-state.json` | For each reading: the summary state it sends, and the words a person is shown |
| `fc4/facts.json` | Sixteen yes-or-no facts about what a device holds for one claim, each with the rule of the design that defines it |
| `fc4/precedence-cases.json` | 63 cases: a set of facts, the reading the design gives, and the rule that decides it |
| `fc4/lives.json` | 66 lives of one claim, 465 steps in all: each step is an event, and the reading and summary state that must follow it |
| `fc4/fixtures/` | Sixteen files the schemas accept and thirty-two they refuse, each refused file wrong in one way and named for it |

The checks are a test, `genesis/orchestrator/reading-and-summary-schema.test.mjs`, which the
orchestrator's gate runs at every landing that touches the test, this `fc4/` directory or the
story named below. It is kept there, and not beside the schemas, because no gate runs a script
kept in a documents directory. It moves with the schemas when their home is ruled. To run it by
hand, from the repository's root: `node --test genesis/orchestrator/reading-and-summary-schema.test.mjs`.

## The nine readings, and how one is chosen

A reading is of one *unit*: one claim as reported by one failing release on one device. The
design's section 6 defines the nine and gives the order in which the first that holds wins:
`reopened`, `stale-emitter`, `cure-available`, `on-watch`, `retired`, `superseded`, `review-due`,
`unrecorded`, `open`. `open` is what remains when no other holds, so every unit with a record that
did not pass has exactly one reading. A unit the device holds no record of, or holds only passing
records of, has none.

The cases state that rule as data. Each case is a set of the sixteen facts and the reading that
must result. Together they hold:

- one case for each reading where only it can hold;
- one for each neighbouring pair in the order that can hold together, where the earlier must win;
- for each neighbouring pair that cannot hold together, a case for each side and the reason;
- the two ways a reading ends early: a lagging release gives way once the device has run the
  newer one, and a reopening ends when a later release fixes the problem again;
- the two cases with no reading;
- the whole life of one unit, in ten steps, from its first failure through two fixes and two
  retirements; and a unit that is observed again before its first fix was acted on;
- four cases added after the integrator's reader found two faults in the amended design: a
  claim retired by a person's ruling while its aggregates remain, the same claim failing again,
  a claim whose fix was a condition and not a release failing after compaction, and a retired
  claim that is later seen only to pass;
- four more after the same reader found a third: a claim whose only remaining records are ones
  compaction may not replace, a late failure from an old release arriving after the note, the
  same once compacted, and kept records beside aggregates.

The test also goes through every combination of the sixteen facts, 65,536 of them, and requires
that each gives one of the nine readings when something failed and none when nothing did, and
that all nine can be reached. That check exists because three faults in a row were states the
written cases had not thought of.

Cases about facts turned out not to be enough. A reviewer found, five times in a row, an
ordinary sequence of events after which the reading was wrong, each time in a state the last
correction had opened. So the test now also carries a small model of one claim's life: an event
goes in (a failure, a pass, a release arriving, the retire command, a ruling, the watch
elapsing), the model updates what the device holds, the facts are worked out from that, and the
reading comes out. `fc4/lives.json` scripts 66 such lives and states the reading and the
summary state after every step. Twenty-one of them are a reviewer's own, written to break the
model, and seven exist because the reviewer showed that a change to the model went unnoticed. Among them: a claim fixed by a release through two fixes and two
notes; a test that goes red, green, red, green; a claim closed by a person's ruling; and the
sequences the reviewer found. The design's section 6 is written from the same eighteen rules the
model runs, numbered R1 to R18, and the cases and lives cite those numbers.

The model is a reference for the rules and not an implementation of the judge: the real judge
has to work the facts out from a store, which nothing here does.

Three readings are shown to a person in plain words: `stale-emitter` as "behind",
`cure-available` as "cure available" and `on-watch` as "on watch". The story
`genesis/a2o/features/devflow/feedback-drains-when-cured.feature` uses those words, and the test
checks that every reading word and state word in the story is one the mapping or the schema
lists.

## What the summary schema decides

These follow the design's section 5.5 as amended on 2026-11-27. They are the designing session's
working rules for a draft, and none is a ruling of the steward's.

- **Nine fields are required** (`fc4/claim-summary.draft.schema.json`, `required`). Six are the
  closed ones the design names as present in every summary: `claim`, `emitter`, `polarity`,
  `severity`, `state` and `class`. Three are measured and are carried in every state:
  `sessions`, `first-day` and `last-day`. The other two measured fields, `dropped` and `bytes`,
  are required in a `cured` or `retired` summary, refused in an `open` one, and in a
  `reopened` one are present together or not at all. `polarity` is `negative` or `positive`: whether the claim names a problem met or a
  command that worked, as the tool's declaration says. Severity here is one of the three levels
  a device handles (`info`, `warn`, `critical`), not the six names a declaration is written in.
- **`state` is one of four**: `open`, `cured`, `retired`, `reopened`. **`class` is one of two**:
  `fixture` or `real`.
- **`cure` is allowed only in a `cured`, `retired` or `reopened` summary.** An `open` summary
  with a `cure` is refused, and so is an `open` summary with `dropped` or `bytes`.
- **Counts are whole numbers that are not negative, and days are calendar days**, not times.
- **Nothing else may be in a summary.** A file with a `message`, an `about` or any unknown field
  is refused.

## Evidence

Everything below was done on 2026-11-27, on shem, one of the steward's devices (a development
workspace).

- Codex (astra) wrote a first draft and reported it incomplete: the summary schema did not
  compile, and the order of readings in the design could not be run as written. That report is
  why the design's sections 5.5, 6 and 7 were amended (see the design's provenance and this
  sprint's journal).
- Codex (astra) then redid the draft against the amended sections. The designing session ran its
  check: 16 accepted fixtures, 32 refused, 37 cases, no failures, no case left open.
- The integrator's reader then found two faults in the amended design that those 37 cases did
  not reach. The design was corrected again and four cases were added, making 41.
- The same reader then found a third: a state with no reading at all, a fix present and nothing
  left for compaction to replace. The design now makes `open` what remains, one fact was dropped
  as no longer needed, four cases were added, making 45, and the check of every combination was
  added. Put back, the old rule fails that check and one case.
- The same reader then found two states that read wrongly, which no check of single states could
  catch: a claim fixed by a condition stayed `reopened` for good after its first recurrence, and
  a claim could read `retired` on the strength of a note for a fix it had since reopened
  against. Codex (astra) built the model and the lives from sixteen rules the designing session
  wrote down; the designing session ran them (26 tests passed), read the main lives step by
  step, and put each of the two faults back in turn: the first makes six lives fail, the second
  makes one fail. There were then 46 cases.
- The integrator's reader then copied the model, ran twenty-one lives of its own against it and
  changed the model twenty-four ways to see which changes the lives would notice. It found four
  more faults and seven changes nothing noticed. The rules were revised (R4, R10, R13, R14) and
  one added (R17); Codex (astra) changed the model, brought the reviewer's twenty-one lives in
  under their own names, and added a life for each unnoticed change. The designing session ran
  the result (55 tests passed), read the lives for the four faults step by step, and put the
  first fault back: three lives fail. There were then 49 cases and 43 lives.
- A further pass by the same reader, fourteen lives and thirty-five changes to the model, closed
  those four and found two more things. Which fix is acted on when a release fix and a condition
  fix are both present is now a rule, with a life. A device that skips the release that carried
  a fix does not see the fix: that is stated as a gap in the design and pinned by a life named
  `gap-skipped-curing-release`, so that it is not mistaken for intended behaviour. Five lives and
  four checked changes were added: 48 lives, 325 steps, 60 tests.
- The skipped-release gap went to the brit lane, which chose a cumulative list of fixes and
  fixed its shape for brit 0.3.12 (see the FC-2 note beside this one). The model was changed to
  read it: a release now carries its declaration's whole list, a fix can be withdrawn, a release
  fix ends a reopening of either kind, and a claim whose fix was counted against can be moved to
  a later fix and watched again. Codex (astra) made the change in three rounds; the first stopped
  at two steps the rules did not decide, and the third at one step where the designing session's
  brief had expected the wrong reading. The integrator's reader judged the result sound against
  thirteen more lives of its own. Both known gaps are closed by it. 61 lives, 432 steps, 63
  cases, 74 tests.
- A later batch pinned two corners of the first rule that no life had exercised, and in doing so
  found the model wrong about a person's own report: written in a session that already held an
  automatic record it replaced that record, and written after that record had been compacted it
  was not stored at all. Every such report is now its own record. One earlier life changed its
  readings. The working names in the lives' notes were replaced by the design's rule numbers,
  and a guard the integrator's reader had shown to be redundant was removed. 66 lives, 465 steps,
  63 cases, 79 tests.
- One thing the model does behind a named switch, because it is not the designing session's to
  decide: whether a machine-checked recovery may close a claim while a person's own report of
  its return still stands. The switch is `KEPT_REPORT_HOLDS_A_CLAIM_OPEN`, true until the steward
  rules. Three lives show the claim staying open; three twins, whose names end
  `-if-decision-10-lets-the-machine-close`, show it watched and closed under the later fix.
- The designing session then changed the check six ways on a scratch copy, and each change
  failed as it should: a reopening with no end (four cases failed); `superseded` placed
  before `retired` (one case); a reading taken from the list; a refused fixture replaced by a
  valid file; a shown word changed so the story no longer matched; and the design's original
  rule for `cure-available`, with which four cases failed. The last is the fault the first draft
  found.
- The test in the orchestrator's suite was written from that check by Codex (astra) and run by
  the designing session; its results are in the journal entry for this station.

## Not settled by these files

- **How a summary names the fixing release.** The design now says `cure` holds the producer and
  the version of the declaration's entry, and no longer an address, since a device that skipped a
  release holds no address for it. The draft schema still accepts any text there: the exact
  spelling is not fixed.
- **Where a `reopened` summary's numbers came from.** The design allows three sources, and a
  shape cannot tell which was used; the schema accepts all three.
- **Anything a shape cannot check**: that a claim and its release are ones a declaration lists,
  that the severity was folded correctly, that the counts are true, that the watch was really
  met, that consent was given. Those need the device's records, not a schema.

## Not proven

- No judge, no compaction, no retirement and no summary exists in code. Every fixture here was
  written by hand.
- Only Ajv 8.18.0, a JavaScript JSON Schema validator, was run.
- The amended sections of the design were read by three blind readers, by Codex in writing
  these cases, and twice by the integrator's reader. Each found faults the one before had not.
  The sprint journal in this directory (`journal.md`) records each round and what it found. The
  corrections made after the last reading have been read by nobody but their author; what stands
  behind them is the check of every combination and the scripted lives. A life nobody has
  thought of is still the likeliest place for a fault: the reviewer's twenty-one found four.
- The model takes the order of events as time and holds one unit. What the judge does with a
  record dated before the event it follows, and how two units of one claim from two emitters
  stand side by side, are not modelled and not designed.
