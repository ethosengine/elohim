---
title: Valueflow authoring surface — claim, fulfil, rule, context — Implementation Plan
id: valueflow-authoring-surface-plan
status: Draft
class: process-meta
domain: D9
sprint: valueflow-authoring
requires_env: [household-nodes]
cites:
  - "valueflow-authoring-surface-design | the design record this plan renders task by task and does not reopen; every task traces to one of its decision sections | sha256:3036ad9306270f5a | path: genesis/docs/superpowers/specs/2026-09-05-valueflow-authoring-surface-design.md"
  - "epr-rea-valueflow-fabric | the atom definitions (Intent, Commitment, FlowEvent) Tasks 3 and 4 mint against, and the positional slot discipline that keeps existing content addresses stable | sha256:1cec32527dbff6d7 | path: genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md"
  - "actor-plane-implementation-plan | the house shape this plan follows, and the source of the --as / --session / steward arms the new verbs reuse | sha256:3044daacc8d5b48f | path: genesis/docs/superpowers/plans/2026-08-15-actor-plane-implementation-plan.md"
---

# Valueflow authoring surface — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this
> plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the developer valueflow the four verbs it is missing, so a task can be claimed by
an actor, discharged by a task report, ruled on as a record instead of as prose, and read back on
one screen. Then dogfood the whole loop against the Holochain Evolution Epic and leave one delta
line on the habit it serves. The design record is the spec cited above; this plan renders it and
does not reopen it.

## Global constraints

Every task obeys these. They are stated once here so no dispatch has to restate them.

- **Gate:** `just gate eprfs`. That is the only gate for this tree.
- **Cargo environment:** `RUSTFLAGS=""` and `CARGO_TARGET_DIR=/tmp/eprfs-gate-target`, with
  `CARGO_BUILD_JOBS=4`.
- **Berth:** run `berth claim cargo` before any cargo invocation and `berth release cargo` after
  it. Another session's implementers hold that lease intermittently.
- **Never judge a cargo run from piped or tailed output.** Echo `EXIT=$?` on its own line
  immediately after the command and read that line.
- **`cargo nextest` is not installed in this container.** Use plain `cargo test`.
- **Commit trailers**, on every commit:
  ```
  Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01WrbMHsaig9cqEgo5T8Hq4T
  ```
- **Commit subject suffix:** `(valueflow Task N)`.
- **Implementers never ask the user.** A blocked task reports BLOCKED with the blocker named.
- **Report statuses:** `DONE | DONE_WITH_CONCERNS | NEEDS_CONTEXT | BLOCKED | HOLD`, with a
  mandatory gate-evidence line quoting the gate command and its exit status.

## Tasks

### Task 1 — `NoteKind::Ruling` and `NoteKind::Verdict`, plus the `--verdict` slot

**Files:**
- Modify `elohim/eprfs/epr-cli/src/flow/note.rs` — add two `NoteKind` members with tags
  `run:ruling` and `run:verdict`, extend `NoteKind::parse` and `NoteKind::tag`, add a
  `VERDICT_SLOT_PREFIX` const beside the existing reason and switched-to prefixes, and thread an
  optional verdict through the note constructor so the slot lands after `reason:` and before
  `steward:`.
- Modify `elohim/eprfs/epr-cli/src/flow/mod.rs` — parse `--verdict` in `run_note`, and extend the
  `usage()` string's note line with the two new kinds and the flag.
- Test `elohim/eprfs/epr-cli/src/flow/note.rs` — extend the existing in-file unit test module.

Steps:
1. Accepted verdict values are exactly `approved` and `changes-requested`. Anything else is an
   invalid-arguments error naming both.
2. `--kind verdict` without `--verdict` is refused. `--verdict` on any other kind is refused.
   Both refusals name the flag and the kind.
3. Slot order is positional and additive: tag, subject, `reason:`, optional `switched-to:`,
   optional `verdict:`, optional `steward:` last. Adding the slot must not move any existing
   note's content address, which the existing kinds' tests already pin.

Verification: claim the berth, then run the crate's `flow::note` tests with the plan's cargo
environment, echo the exit line, release the berth.

- [ ] **Task 1 deliverable: `epr flow note --kind ruling` and `--kind verdict` work end to end, the
      verdict slot is required and validated, and the usage string names both kinds.**

### Task 2 — Shared latest-event helper and a notes-on-atom reader

**Files:**
- Create `elohim/eprfs/epr-cli/src/flow/read.rs` — the shared read helpers module.
- Modify `elohim/eprfs/epr-cli/src/flow/fulfill.rs` — delete the private
  `commitment_latest_event` body and call the shared one, keeping the exact ordering semantics.
- Modify `elohim/eprfs/epr-cli/src/flow/mod.rs` — declare `pub mod read;`.
- Test `elohim/eprfs/epr-cli/src/flow/read.rs` — in-file unit tests over a hand-built record
  vector.

Steps:
1. Lift `commitment_latest_event` verbatim. Its ordering rule is occurred-at first, then sidecar
   append order as the tie-break, and that rule must not change. It exists so this reader and the
   saga status reader agree on "latest"; a divergence here is a real defect, not a cleanup
   opportunity.
2. Add a `notes_on(records, resource_cid, limit)` reader: `Cite` events whose `resource` equals the
   given address and whose quantity unit is `run-note`, ordered by `occurred_at` then append order,
   newest first, truncated to the limit.
3. The note view parses the positional slots back out: tag to kind, subject, `reason:`,
   `switched-to:`, `verdict:`, `steward:`. An unrecognised slot is carried through, never dropped,
   so a future slot does not silently vanish from a render.
4. Do not touch `walk`'s `Produce` filter. Its JSON contract is a stability promise.

Verification: claim the berth, run the crate's `flow::read` and `flow::fulfill` tests, echo the
exit line, release the berth.

- [ ] **Task 2 deliverable: one shared latest-event helper with fulfill calling it, plus a
      notes-on-atom reader with slot parsing, both unit tested.**

### Task 3 — `epr flow claim`

**Files:**
- Create `elohim/eprfs/epr-cli/src/flow/claim.rs` — argument loop, resolution, minting, outcome
  struct and render, following the `note.rs` idiom.
- Modify `elohim/eprfs/epr-cli/src/flow/mod.rs` — add the `"claim"` dispatch arm and the usage
  line.
- Test `elohim/eprfs/epr-cli/tests/flow_edges.rs` — new integration tests in the existing
  synthetic-git-repo style.

Steps:
1. Flags: `--on`, `--as`, `--brief`, `--serves`, `--session`, `--supersede`, `--json`, `--root`.
2. `--on` resolution order: a content address known to the sidecar, which must resolve to an
   `Intent` or the call is refused; then a gap id matched against `classified_as` slot 1; then a
   repository path resolved to its canonical body address and from there to the intents scoped by
   it. A path resolving to more than one intent is refused with every candidate named.
3. Mint one `Commitment` with `state: Active`, the resolved actor as `provider`,
   `satisfies: [intent_cid]`, `in_scope_of` copied from the intent, and `classified_as` of
   `gap:claimed`, the gap id, optional `brief:<body address>`, optional `habit:<id>`, and
   `steward:<git author email>` last.
4. `--serves` is checked against `genesis/manifests/habits.yaml`; an unknown id is refused and the
   error names the register file. Reuse the register reader from Task 6 if that task lands first,
   otherwise land a minimal reader here and let Task 6 absorb it.
5. Duplicate refusal: an active, undischarged commitment already satisfying the intent refuses and
   names the incumbent, including `tool:decompose-claim` commitments. `--supersede` mints anyway
   and reports what it superseded.
6. Identity is the atom address, so re-running an identical claim appends nothing and reports
   `appended: false`.
7. Timestamps come from the git HEAD author date, never a wall clock.

Verification: claim the berth, run the `flow_edges` integration tests filtered to `claim`, echo the
exit line, release the berth.

- [ ] **Task 3 deliverable: `epr flow claim` resolves all three `--on` forms, mints one commitment,
      refuses duplicates, honours `--supersede` and `--serves`, and is covered by integration
      tests.**

### Task 4 — `epr flow fulfill --on`, the task-report arm

**Files:**
- Modify `elohim/eprfs/epr-cli/src/flow/fulfill.rs` — add the flag-form arm beside the existing
  positional sprint-report path, sharing nothing that would perturb the a2o behaviour.
- Modify `elohim/eprfs/epr-cli/src/flow/mod.rs` — `run_fulfill` branches on whether the first
  argument is a flag, and the usage string gains the flag form.
- Test `elohim/eprfs/epr-cli/tests/flow_edges.rs` — new integration tests.

Steps:
1. Flags: `--on`, `--report`, `--status`, repeatable `--commit`, `--as`, `--session`, `--json`,
   `--root`. The positional form stays byte-compatible; its tests must still pass untouched.
2. Emit one `Produce` flow event, unit `task-report`, `fulfills: [commitment_cid]`, and
   `classified_as` of `report:<status>`, the label, `evidence:<body address of report>`, one
   `commit:<sha>` per flag, and `steward:` last. `occurred_at` is the git HEAD author date.
3. Status gate: only `DONE` and `DONE_WITH_CONCERNS` are accepted. `NEEDS_CONTEXT`, `BLOCKED`, and
   `HOLD` are refused, and the error names `epr flow note --kind observation` as the correct
   record. This is load bearing for the equilibrium habit's outflow classifier, which keys on
   `fulfills`.
4. Gap-id resolution picks the newest active commitment whose `classified_as` carries that id.
5. A second fulfilment of an already discharged commitment reports `already_fulfilled` and appends
   nothing.

Verification: claim the berth, run the `flow_edges` integration tests filtered to `fulfill`, echo
the exit line, release the berth.

- [ ] **Task 4 deliverable: `epr flow fulfill --on` discharges a gap-item commitment from a task
      report, refuses the three non-discharging statuses, and leaves the sprint-report arm
      unchanged.**

### Task 5 — `epr flow context`, sections 1 through 5

**Files:**
- Create `elohim/eprfs/epr-cli/src/flow/context.rs` — the `ContextResult` struct, the library entry
  `pub fn context(root, target) -> FlowResult<ContextResult>`, the human render, and the argument
  loop.
- Modify `elohim/eprfs/epr-cli/src/flow/mod.rs` — the `"context"` dispatch arm and the usage line.
- Test `elohim/eprfs/epr-cli/tests/flow_edges.rs` — integration tests calling the library function,
  as the existing walk tests do.

Steps:
1. Flags: `--notes N` defaulting to 5, `--json`, `--root`.
2. Section 1 identity, section 2 intents and section 3 commitments come from `walk`'s existing
   scoped-intents and frontier data plus the Task 2 latest-event helper. Section 4 notes come from
   the Task 2 notes reader. Section 5 seals comes from the existing edges data.
3. A content-address target resolves without a path and skips sections 5 through 8, printing a
   one-line note saying why.
4. The human render stays at or under 40 lines. Truncate lists with an explicit "and N more" line
   rather than silently.
5. `--json` serialises the whole `ContextResult` with every section present, absent sections as
   explicit nulls or empty collections, never omitted keys.

Verification: claim the berth, run the `flow_edges` integration tests filtered to `context`, echo
the exit line, release the berth.

- [ ] **Task 5 deliverable: `epr flow context <path|cid>` renders identity, intents, commitments,
      notes and seals, under 40 lines human and complete under `--json`.**

### Task 6 — Habit and gate readers, context sections 6 and 7

**Files:**
- Modify `elohim/eprfs/epr-cli/src/flow/context.rs` — the habit section and the gate section.
- Create `elohim/eprfs/epr-cli/src/flow/registers.rs` — the register reader (serde_yaml over the
  generated habits projection) and the gate reader (walk up to the nearest `build-manifest.json`,
  read `gate.projects`).
- Modify `elohim/eprfs/epr-cli/Cargo.toml` only if serde_yaml is not already a dependency; the flow
  module already surfaces a Yaml error variant, so check before adding.
- Test `elohim/eprfs/epr-cli/src/flow/registers.rs` — in-file unit tests writing fixture
  `habits.yaml` and `build-manifest.json` files under a `tempfile::TempDir`.

Steps:
1. Habit matching, in this order: any register entry whose `checks:` or `refs:` strings contain the
   atom's repository path; then any habit declared in the nearest ancestor `.epr-meta` directory as
   `<id>.habit.md`. Print id, status, active, and the first check, one line each.
2. When the target is itself a `.habit.md` atom, render the habit as a scope instead: its status,
   active flag and checks; the open commitments carrying its `habit:<id>` slot; the specs and plans
   whose `refs:` or cites name it; and its notes newest first.
3. Gate: from the atom's directory walk up to the nearest `build-manifest.json`, then select the
   `gate.projects` entry whose `dir` is a prefix of the atom's repository path. Print
   `just gate <name>` plus the cargo target directory and rustflags when the entry declares them.
   No match prints an honest "no gate project covers this path" line, never a guess.
4. The register YAML is a generated projection. Read it as data and never write it.
5. Fixture tests must not read the live repository. Build both files under a temp directory.

Verification: claim the berth, run the crate's `flow::registers` and `flow::context` tests, echo the
exit line, release the berth.

- [ ] **Task 6 deliverable: context names the covering habit and the owning gate for a path, and
      renders a `.habit.md` target as a scope, with fixture-backed unit tests.**

### Task 7 — Three governance inject rules at the repository root

**Files:**
- Modify `.epr-meta/manifest.md` — three rules of class `inject` in the `rules:` list, plus a short
  prose section in the body explaining the trio.
- Test: none in code. The verification is a live hook observation, recorded in the task report.

Steps:
1. Rule `brief-is-a-claim`, `when: { write: ".superpowers/sdd/**/task-*-brief.md" }`, whose `why`
   names `epr flow claim --on <gap-id> --as agent:implementer@<model> --brief <this file>`.
2. Rule `report-is-a-fulfilment`, `when: { write: ".superpowers/sdd/**/task-*-report.md" }`, whose
   `why` names
   `epr flow fulfill --on <gap-id> --report <this file> --status <DONE|DONE_WITH_CONCERNS>`.
3. Rule `rulings-are-notes`, `when: { write: ".superpowers/sdd/**/progress.md" }`, whose `why` names
   `epr flow note --on <gap-id|plan> --kind ruling --reason '...'` and states that the progress file
   is a projection and never the record.
4. Each rule carries a `retire-when:` clause, as every rule in this manifest does.
5. Then verify the gate actually fires. Write a throwaway
   `.superpowers/sdd/valueflow-probe/task-1-brief.md` through the Write tool and record whether the
   inject signal appeared. Those paths are gitignored, so the hook may skip them. Report the
   observed result honestly either way. If it skips, say so in the task report and in a one-line
   note in the manifest body, and leave the rules in place as documentation of the pattern. Delete
   the probe file afterwards.

Verification: write the probe file through the Write tool, read the hook output, then
`git diff --stat .epr-meta/manifest.md`.

- [ ] **Task 7 deliverable: three inject rules land at the repository root and the task report
      states, from observation, whether the compose gate fires on gitignored authoring paths.**

### Task 8 — Three native SkillPackages

**Files:**
- Create `.epr-meta/elohim/packages/skills/valueflow-authoring.json` — the orchestrator method.
- Create `.epr-meta/elohim/packages/skills/valueflow-implementer.json` — the implementer seat.
- Create `.epr-meta/elohim/packages/skills/valueflow-reviewer.json` — the reviewer seat.
- Modify: the generated projections under `.claude/skills/`, `.codex/skills/` and `.agents/skills/`
  are written by the projection CLI, never by hand.
- Test: `pnpm run elohim-agent:packages:verify` is the test.

Steps:
1. Each package is `kind: SkillPackage`, `apiVersion: elohim-agent/v1alpha1`,
   `metadata.sourceRuntime: "elohim-agent"`, `metadata.master: "package"`, and
   `runtimeTargets: ["claude", "codex", "antigravity"]`.
2. Native packages get no generated governance block. Hand-write `metadata.governance` with
   `eprRef: "epr:elohim-agent/skills/<id>"`, `policy: "capability-governance@1"`,
   `gates: ["epr-meta-resolver", "elohim-agent:packages:verify"]`, and
   `ledger: ".claude/data/governance-findings.jsonl"`.
3. Bodies follow spec section 8 exactly. `valueflow-authoring` is the orchestrator method, verb by
   verb, one command per verb, plus the friction principle, the WIP fence, the dispatch prompt shape
   and the do-not list. `valueflow-implementer` and `valueflow-reviewer` are seat contracts, and
   each seat skill ends in exactly ONE verb.
4. Project each package, once per id:
   ```
   node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs project \
     --write-fixtures --write-runtime --only SkillPackage:valueflow-authoring
   ```
   and the same for `SkillPackage:valueflow-implementer` and `SkillPackage:valueflow-reviewer`.
5. Do not hand-edit any projected file. If a projection looks wrong, the package JSON is wrong.

Verification: `pnpm run elohim-agent:packages:verify` with `EXIT=$?` echoed on its own line.

- [ ] **Task 8 deliverable: three native SkillPackages authored with hand-written governance blocks,
      projected to all three runtimes, with the package verifier green.**

### Task 9 — The a2o story and its blind-reader loop

**Files:**
- Create `genesis/a2o/features/devflow/valueflow-authoring.feature` — the story.
- Test: the four Rust assertions from Tasks 3 through 6 are the executing proof.

Steps:
1. Feature tagged `@concern:valueflow-authoring`, plus the suite-routing tags the sibling devflow
   features carry. Four scenarios, from spec section 11: a claim mints one commitment and a
   duplicate is refused; a DONE report discharges and a BLOCKED report does not; a ruling and a
   verdict are readable in context newest first; context names the habit and the gate for a storage
   source file.
2. If no step definition drives the feature, tag it `@wip` at the feature line and open the file
   with a header comment saying so, exactly as the run-plane feature does. A claimed green with no
   step definitions is the dishonesty this convention exists to prevent.
3. **Obligation from `genesis/a2o/.epr-meta`:** the `a2o-story-blind-reader-review` inject rule
   requires that, after the authoring pass, a fresh-context blind-reader is dispatched with ONLY the
   completed feature path and the `a2o-story` review profile. No conversation, no plan, no diff, no
   related docs. Revise from its interpretability and coherence findings, then repeat with a NEW
   reader until the story is READY or the operator explicitly defers named findings. This is one
   review loop per completed authoring pass, not one dispatch per edit. Record each round's verdict
   in the task report.
4. The `@concern:` tag is the join across the register, CI and Gherkin. It must be globally unique,
   and it must match the string used in the habit's `checks:` if this concern is ever added there.

Verification: run the a2o lint, then a Gherkin dry run scoped to this one feature file with
`--config` pointed at an empty config. A bare profile run would merge the profile's `paths` with the
positional and run the whole suite.

- [ ] **Task 9 deliverable: the feature file lands with four scenarios and an honest `@wip` state if
      unwired, and the blind-reader loop has run to READY or to an operator deferral.**

### Task 10 — Gate green and the binary installed

**Files:**
- Modify: none. This task verifies.

Steps:
1. `berth claim cargo`, then `just gate eprfs`, then `berth release cargo`. Echo `EXIT=$?` on its
   own line. Read that line, not the tail of the output.
2. Install the binary into the devspace slot:
   ```
   cargo install --path elohim/eprfs/epr-cli --root /opt/rust/cargo \
     --target-dir /tmp/eprfs-gate-target
   ```
3. Confirm the surface: `epr flow` with no subcommand prints the usage string, and that string names
   `claim`, `context`, the `fulfill --on` flag form, and the `ruling` and `verdict` note kinds.
4. If the gate defers under disk pressure, that is not a pass. Reclaim and re-run, or report BLOCKED
   naming the watermark.

Verification: the gate's own exit line, then `epr flow` with no arguments, reading the first twenty
lines of its usage output.

- [ ] **Task 10 deliverable: `just gate eprfs` exits zero with the exit line quoted, the binary is
      installed at `/opt/rust/cargo/bin/epr`, and its usage names the new verbs and kinds.**

### Task 11 — Dogfood the loop against the Holochain Evolution Epic

**Files:**
- Modify `.claude/memory-kit/gap-items/<epic-plan-slug>.json` — regenerated by decompose, never
  hand-edited.
- Modify `.eprfs/status/flows.jsonl` — appended by the verbs, append-only and idempotent by content
  address.
- Modify `.epr-meta/dev-system-equilibrium.habit.md` — one delta line.
- Modify `genesis/manifests/habits.yaml` — regenerated by the register projector, never hand-edited.

Steps:
1. Re-run decompose so Tasks 16 through 20 exist as gap items. The script takes one positional
   document path and no flags (its module docstring is the only usage text; read it first):
   ```
   python3 .claude/scripts/memory-kit/decompose.py \
     genesis/docs/superpowers/plans/2026-09-04-holochain-evolution-epic-mvp-plan.md
   ```
2. Read the commitments stock BEFORE, over the last seven days:
   ```
   epr flow stocks --window 2026-08-29..2026-09-05 --per day --stock commitments \
     --root /projects/elohim
   ```
   Record level, inflow, outflow and verdict verbatim.
3. `epr flow project` to mint the new intents.
4. For each landed task, claim then fulfil with `--as agent:implementer@claude-opus-5`. Task 16 with
   commits `825a090df` and its fix `4425bb6fb`; Task 17 with `10cb3dc00`; Task 18 with `4fe69b918`.
   Use `--status DONE` and point `--report` at the task's own report file. If a
   `tool:decompose-claim` commitment already holds the intent, the refusal will name it; use
   `--supersede` and say so in the report.
5. Record one `--kind ruling` on the epic plan and one `--kind verdict` (with `--verdict approved`)
   on one of the three landed tasks, so the context render has both to show.
6. Run `epr flow context` on
   `genesis/docs/superpowers/plans/2026-09-04-holochain-evolution-epic-mvp-plan.md` and confirm the
   intents, the three commitments, the ruling and the verdict all appear on one screen.
7. Read the stock AFTER with the identical window and flags. The outflow arm must have moved. If it
   did not, that is the finding, and the report says so rather than claiming a drain.
8. Append ONE delta line to `.epr-meta/dev-system-equilibrium.habit.md` in the existing evidence
   ledger style: dated, naming the before and after readings, and stating explicitly that this is an
   event and not a rate. **No status flip.** The habit stays red.
9. Re-project the register with `python3 .claude/scripts/habits-project.py`. Never hand-edit
   `genesis/manifests/habits.yaml`.

Verification: the before and after stock readings quoted side by side, then
`python3 .claude/scripts/habits-project.py --check` with `EXIT=$?` echoed on its own line.

- [ ] **Task 11 deliverable: three landed epic tasks are claimed and fulfilled by an implementer
      actor, a ruling and a verdict are readable in context, the commitments outflow reading is
      recorded before and after, and one delta line lands on the habit with no status flip.**

## Self-review (done at authoring)

**Spec-coverage cross-check.** Every decision section of the spec maps to at least one task, and
every task traces back to a decision.

| Spec section | Task |
|---|---|
| 1, the measured friction | the whole plan; Task 11 measures it |
| 2, vision grounding | Task 8, the authoring skill states it |
| 3, habits as scope | Task 3 (`--serves`), Task 6 (habit as scope), Task 11 (delta line) |
| 4, NoteKind ruling and verdict | Task 1 |
| 5.1, `claim` | Task 3, on the Task 2 helpers |
| 5.2, `fulfill --on` | Task 4 |
| 6, context sections 1 to 5 | Task 5, on the Task 2 notes reader |
| 6, context sections 6 and 7 | Task 6 |
| 6, context section 8 governance | Task 5, reusing the existing explain data |
| 7, three inject rules | Task 7 |
| 8, three skill packages | Task 8 |
| 9, out of scope | no task, by construction |
| 10, gate and install | Task 10 |
| 10, dogfood evidence | Task 11 |
| 11, story | Task 9 |

**Placeholder scan.** No TBD, no TODO, no unnamed file, no "etc." standing in for a decision. Every
Files block names exact paths and the mechanism in one line. The one deliberately open item is Task 7
step 5, where the hook's behaviour on gitignored paths is a fact to observe rather than a decision to
make, and the task says to report it honestly either way.

**One checkbox per task.** Eleven tasks, eleven `- [ ] **Task N deliverable:` lines, and no other
checkbox anywhere in the document. The valueflow mints one gap item per checkbox, so this count is
the contract.
