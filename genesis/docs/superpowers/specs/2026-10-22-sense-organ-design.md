---
title: The sense organ — from middot measures to habits to the agentic loops
id: sense-organ-design
status: Draft
class: architecture
serves: sensing-rides-the-measure-plane
date: 2026-10-22
context-tier: disclosed
steward: human:matthew
author: "Claude (Opus 5.5) at the operator's direction, from the operator's autonomous-loop refactoring notes"
tags: [middot, measures, sensing, substrate-bypass, duplication, code-smell, memory-smell, habits, agentic-loop]
graduation-trigger: >-
  On the host store, `epr flow report --headline` shows a `sense:` line whose numb count has
  fallen from its first reading. A declared bypass or duplication probe's candidates each carry a
  ruling, and a fresh agent handed only `epr flow report --breaches` takes one open candidate to
  a ruling or a migration. A restraint fixture's declared intentional adapter stays excepted and
  named through that run.
cites:
  - "middot-measure-primitive-design | Middot | sha256:336ab2b4619b9144 | path: genesis/docs/superpowers/specs/2026-08-04-middot-measure-primitive-design.md"
  - .epr-meta/ledgers-fold-once.habit.md
  - .epr-meta/governance-plane-single-evaluator.habit.md
  - .epr-meta/dev-system-equilibrium.habit.md
---

# The sense organ

## 1. The problem, measured

The operator asked for one sensing system for three kinds of trouble:
- bespoke code that bypasses the native epr / epr-rea primitives;
- duplicated implementations;
- memory and code smells.

It should run from the middot measures through the habits to the agentic loops, so that agents get the benefit without extra effort.

A read-only survey on 2026-10-22 (dev `5ddc92494`) found that the gap is not a missing detector family. The measure plane cannot sense at all:

- **Declared, never observed.** `measures.yaml` declares 86 measures, and every one carries only a prose `procedure:`. `epr flow report` judges 57 bounds:
  - 4 passed;
  - 7 failed;
  - **46 skipped for want of a fold.**

  `ledgers-fold-once` has been red and unobservable since 2026-10-16 for exactly this reason: `ledger-planes@1` was declared and never folded.
- **Findings reach habits outside the fold plane.** They travel as three private JSONL ledgers, which `habits-status.py` joins with its own copy of the `@concern` regex.
  - Each producer carries its own open/close rule. The gate-cycle rule closes after two runs under the ceiling. The resolver rule never closes, and a closed fingerprint can never be filed again.
  - `rs-loc-ceiling` rows carry no concern, so they reach no habit.
- **Breach → dispatch is re-implemented in each producer.** The native `measures::Bound` did not parse `dispatch-*`.
  - `gate-cycle.mjs` parses `measures.yaml` itself and re-judges `at-or-above` itself.
  - `measure-fold.py` keeps a fold store of its own under `.claude/data/measure-folds/`.

The sensing machinery's own bypasses are its first cases.

## 2. Design: one organ, five joints, no new store

**J1 — Probe.** A measure row may carry `probe: {native | run, emits: count|candidates, due, timeout-seconds}`. The probe is ONE implementation of the prose procedure, which stays the contract.
- **Two kinds, matched to durability** (operator ruling 2026-10-22: the lowest-level primitives have first-class Rust support for performance and durability, and Python is a disposable local artifact of the design):
  - **`native: <name>`** is compiled into `epr` and runs in-process. It is the durable form, for the primitives the plane stands on. The first are `concern-joins` and `concern-evidence-unadopted` (`flow::concern`, §3).
  - **`run: <command>`** is a shell command under its budget. It is the disposable form: a detector sketched while its procedure and its precision are still being learned (`ledger-planes`, `limit-models` in `.claude/scripts/probes/`). It graduates to `native:` once its rulings settle, and its script is deleted.
- **Output.** Either kind yields one JSON-shaped output: a `value`, or `candidates` (`subject`, `symbol`, `signature`), plus `detail` lines for people.
- **The fold and the judge are always native.**

**J2 — Sense.** `epr flow sense [--measure M]… [--family F] [--due D] [--dry-run]` runs the selected probes under their budgets and folds what they report through `note::observe`, the writer every other producer already uses.
- **Count.** A `count` reading is folded on subject `.`, and only when it changes.
- **Candidates: transitions only.** A `candidates` run folds:
  - **1 for each newly reported file**;
  - **0 for each file that vanished**.

  The `distinct-subjects-since-reset` derive already counts the subjects whose latest fold is non-zero, so a candidate opens, closes and re-opens with no ledger, cursor or fingerprint file.
- **Deleted files.** A deleted file cannot take a zero, because an observation needs a readable subject. In that case the run folds the declared `<id>-reset@1` and re-opens the live set after it. Each candidate fold carries `env: epoch=<short reset CID>`, so a re-opened subject mints a new record instead of deduplicating into its pre-reset twin.
- **First reading.** A clean first `candidates` run opens no subject, so on its own it would write nothing, and the lens would read `skipped — no fold`: numbness passing as health. The first live reading hit this on `concern-joins-open@1`. When nothing of the measure has ever been folded and no reset exists, a clean run instead folds the declared `<id>-reset@1` once (`env: opens=first-reading`). That opens the window, and the lens reads a witnessed 0.
- **Refusal.** A probe that crashes, overruns its budget or prints anything but its JSON writes nothing, and the command exits 1. A crashed detector is not a measured zero.

**J3 — Classification at the owning boundary.**
- **How.** A candidate is classified by a native `ruling` note on its own file. The reason reads `sense <measure@v> <class>: <contract difference>`. `epr flow sense rule …` writes it, and refuses one with no stated difference.
- **Classes:**
  - `duplication`
  - `intentional`
  - `canonical` (the owner)
  - `unresolved`
  - `false-positive`
- **Effect.**
  - `intentional`, `canonical` and `false-positive` EXCEPT the subject: it is healed, counted, and named in every run's output, so restraint is visible.
  - `duplication` and `unresolved` annotate the candidate and keep it open.
- **Deviation from the handoff.** The handoff proposed an `except:` key in `.epr-meta`. A YAML exception list would need a Python reader and a Rust reader, and two readers are a declared twin: `governance-plane-single-evaluator` Risk 1. A ruling note is already content-addressed, attributed, sits on the owning file, and supersedes by append.

**J4 — Lens → habit.** A candidate measure is read by a lens with `derive: distinct-subjects-since-reset`, `compare: at-or-above` and `hard: 1`, plus a `concern:`.
- `habits-status.py`'s existing cost join reads any active lens that has a concern, so no new reader is needed.
- The first two lenses, `limit-models-open@1` and `concern-joins-open@1` (§3), price the new habit `sensing-rides-the-measure-plane`.
- `ledger-planes-ceiling@1` already prices `ledgers-fold-once`.
- The memory and drift lenses price `dev-system-equilibrium` (operator ruling, 2026-10-22). Cleanup pressure is a stock filling faster than it drains.

**J5 — Breach → loop.**
- **`report --breaches`.** `measures::Bound` now carries `dispatch_agent` and `dispatch_prompt`. `epr flow report --breaches [--json]` returns each failed bound of the primary recipe with its concern, its dispatch and its outstanding subjects. Those subjects come from the same `latest_per_subject` the derive counts with, so "open" means one thing in the bound, in `sense`, and in the breach list.
- **The `sense:` headline slot,** appended after `cost`. It reads `N of M bounds observed · K breached · numb J (first three names) → epr flow sense --due session`. Numbness becomes visible at session start, and unmeasured renders differently from zero (middot §4).
- **Shift.** A contract serves the habit, and its objective `measure` is `epr flow sense --measure M --json`, filtered to one subject. Its restraint oracle is a fixture whose declared intentional adapter must stay excepted (notes §9: detection AND restraint). This is the existing controller; no second controller is added.
- **Learning slot (operator-selected, 4:1).** `sense-classes@1` folds the per-class count for each candidates run. Precision is `false-positive` against the rest, and `unruled` is review still owed. These numbers are shown beside the reasons and never rank candidates (notes §5).

## 3. A concern is a declared entity with relations

The first probe over `@concern` (`concern-tag-parsers@1`) treated four spellings as four grammars to converge. That was the restriction reflex. The operator's ruling (2026-10-22) was that the language does not need a narrower grammar: it needs the ontology made explicit. A **concern** is a declared entity, and a reference to one is valid when it **resolves**, never because it matches a character class.

**Three homes declare concerns:**

| Home | What it declares |
|---|---|
| **habit**: `<dir>/.epr-meta/<id>.habit.md` | the habit's `id`, plus every `@concern:` its `checks:` adopt as addresses |
| **canon**: `.claude/epr-meta/concerns.yaml` / `policies.yaml` | invariant classes (`C0`…`C11`) and their row ids, as Precedent-shaped rows |
| **story**: an a2o `.feature` tag line | what that scenario is evidence of |

**Four relations.** Each one is fixed by where a reference is written, not by how it is spelled:

| Relation | Written as | Resolves when |
|---|---|---|
| **evidence-of** | a habit check's `@concern:X` | a story declares X |
| **prices** | a lens's `concern:` | a habit declares X (as id or adopted address) |
| **governed-by** | a policy's `concern:` (`C6a`) | the canon declares X |
| **pain-of** | an open findings-ledger row's `concern` | a habit or the canon declares X |

**Reading it in the substrate's own terms** (a reading, not new code):
- a habit is a standing commitment;
- a story is a declared intent to prove;
- a check adopting a story is the habit committing to be satisfied by it;
- a lens is a bound priced against the commitment;
- a canon class is a Precedent that governs it;
- a finding is pain observed against it.

**Consequences:**
- **What gets measured.** `concern-joins-unresolved@1` (candidates: files holding a reference that does not resolve in its relation's home) superseded `concern-tag-parsers@1`. `concern-evidence-unadopted@1` (count) covers the other side: story concerns no habit adopts. That count is not a broken join, since a story may precede its habit. It is proof serving no declared promise, and adoption (or holding the story) drains it.
- **Readers that resolve are correct readers.** `declared-concerns.ts` resolves tags against `habits.yaml`; `walk.rs` matches declared habit ids. The S0 rulings that classed them `duplication` on character-class grounds are withdrawn with their measure (they stay in the record as history).
- **First reading, 2026-10-22:**
  - **0 unresolved joins** across all four relations;
  - **69 story concerns unadopted.**

  The joins were sound all along.
- **Debt the ontology names (docs lead, code follows).** The key `concern:` carries two relations: *prices* on a lens and *governed-by* on a policy. Today the relation is inferred from the file the key sits in. Whether each relation gets its own spelling is open. Until then, the file is the relation.

## 4. The first census, as rulings (learning-slot sample 0)

Seven signatures were inspected against their contracts before any probe existed:

| # | Signature | Verdict |
|---|---|---|
| 1 | two limit models | partly expressible: inclusive comparison with a hard watermark and a unit only |
| 2 | `stocks.rs` re-implements the judge | FALSE POSITIVE: it calls native `breached_by` (see note below) |
| 3 | private ledgers | real; owned by `ledgers-fold-once` |
| 4 | `@concern` parsers | duplication with a demonstrated split |
| 5 | parity twins | intentional; governed by `governance-plane-single-evaluator` |
| 6 | non-REA flow modules | mostly intentional |
| 7 | wall clock | 4 of 13 sites measurement-relevant |

Signature 2 surfaced a real latent defect instead: `stocks.rs:941` checks only ceilings, and `:1089` emits pain on a floor bound, which `fold::bound_evidence` deliberately does not.

The probes then found more than the census:
- `concern-tag-parsers@1` found four spellings of the tag token across Python, TypeScript, `.mjs` and Rust. It was superseded by the resolution reading in §3: under that reading every reference resolves, so the variance is not a defect.
- `limit-models@1` found 12 files: 2 duplication, 4 unresolved, 3 intentional, 3 false positive.

The rulings are recorded on the host store (§5).

## 5. Slices

- **S0.** This spec, plus the census and probe rulings recorded as notes.
- **S1.** `probe:`/`dispatch-*` parse, `flow/sense.rs`, three probes, two lenses, `--breaches`, the `sense:` slot, and `tests/flow_sense.rs`.
- **S2.** `concern: dev-system-equilibrium` on the memory/drift lenses. The `*-drift-signal` producers become probes as each one's procedure is made exact.
- **S3.** A shift contract example and the restraint fixture under `genesis/agentic/fixtures/sense-restraint/`.
- **S4.** A write-time `inject` rail (`contains-any`: watermark comparisons, `.jsonl` appenders, wall-clock reads in measurement code) that names `epr flow sense` and `elohim_epr_rea::Bound`.

**Later, under their owning habits:**
- migrating the findings ledgers (`ledgers-fold-once`);
- unifying the two limit models (habit-progress §8);
- spelling each concern relation explicitly (§3's named debt), if the inferred-from-file reading proves ambiguous.

## 6. Evidence

- `cargo test -p elohim-epr-cli --test flow_sense`.
- `EPR_BIN=<built epr> python3 -I genesis/agentic/fixtures/sense-restraint/restraint_oracle.py`. It checks four things: the bypass is detected; the ruled adapter is excepted and named; the migrated bypass heals; the reintroduced bypass re-opens while the adapter stays excepted.
- The live `epr flow sense` / `report --bound` readings recorded in `sensing-rides-the-measure-plane`'s DELTA.
