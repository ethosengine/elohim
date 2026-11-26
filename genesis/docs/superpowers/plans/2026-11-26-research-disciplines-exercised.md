---
title: "Exercise the disciplines: research lessons into practice"
id: research-disciplines-exercised-plan
status: active
class: process-meta
date: 2026-11-26
steward: human:matthew
author: "Claude (session cfde00a4, Fable 5.2), at the steward's direction; three read-only explorers and one independent challenger; plan approved by the steward 2026-11-26"
serves: [dataplane-convergence, agents-arrive-briefed]
tags: [research, absorption, disciplines, evidence-age, failure-seat, mint-pass, arrival-brief, sprint]
cites:
  - genesis/research/apollo-sp287-what-made-apollo-a-success-cross-pollination-2026-11-26.md
  - genesis/research/README.md
  - "agents-arrive-briefed-plan | the active plan that owns the arrival brief; flight F4 proposes amendments to its design and exercises its step 2 dry run, and builds no second path to the brief | sha256:26a1214a7c89914a | path: genesis/docs/superpowers/plans/2026-11-22-agents-arrive-briefed.md"
  - "placement-read-and-notes-store-design | the brief's implementation design (landings L1 to L6); flight F4 appends one proposed section changing its target rule and its L3 and L4 rows | sha256:488cc5ace9f96af4 | path: genesis/docs/superpowers/specs/2026-11-23-placement-read-and-notes-store-design.md"
  - genesis/data/timeline/backlog/agentic-harness-borrows-backlog.md
  - genesis/data/timeline/backlog/arch-dataplane-refactor-backlog.md
  - genesis/data/timeline/backlog/measure-family-borrows-backlog.md
---

# Exercise the disciplines: research lessons into practice

Sprint journal: `genesis/docs/superpowers/sprints/2026-11-26-research-disciplines/journal.md`.

## Context

The steward's direction (2026-11-26): pivot from the research to a sprint that integrates what
the research passes have taught us into our systems, taking the most valuable lessons across
all the passes, not only SP-287, and exercising them as disciplines now.

What three read-only explorers found on `origin/dev`:

- **The corpus emits and does not absorb.** 52 research notes; about 208 research-sourced
  backlog rows still open (28 describe a practice, about 180 a build; the 180 is an estimate).
  The habit register reads 243 documents authored against 8 absorbed in 90 days. The
  `mint-pass-at-close` rule in `genesis/research/.epr-meta` is a reminder with no check, and
  its own `retire-when` says so.
- **Eight lessons were reached by two or more independent passes.** Independent agreement is
  the best evidence of value we have, so these are the sprint's candidates. The table is in the
  Research Index (`genesis/research/README.md`, "What more than one pass reached").
- **An active plan already owns the brief.** `2026-11-22-agents-arrive-briefed.md` and its
  design `specs/2026-11-23-placement-read-and-notes-store-design.md` (landings L1 to L6). This
  sprint proposes amendments to that design; it builds no second path to the brief.
- **The SP-287 rows overstated the gaps in places.** Thirteen claims in rows 13, 16 and 17 of
  `agentic-harness-borrows-backlog` and row 24 of `arch-dataplane-refactor-backlog` were wrong
  or too strong against the code. Correcting them is the sprint's first act.

Intended outcome: four lessons move from a row in a backlog to a thing an agent meets when it
acts, used on real work this sprint, with a check that runs. No Rust, no new register.

An independent seat attacked the draft against the code before the steward read it. It changed
three things: the F1 check verifies takes a note declares (the first design guessed, and
contradicted the rule that a take may die in prose); F3 runs through `just mesh
recovery-matrix`, not `just measure`; and a Rust flight was removed because it forked the brief
design and added a due date nothing reads.

## The lessons and where each goes

| # | Lesson | In this sprint |
|---|---|---|
| 1 | Evidence carries its age and source | Flight 2 |
| 2 | A seat that did not do the work judges, and chooses the failure | Flight 3, and the judge of every flight |
| 3 | The brief is selected from position, not accumulated | Flight 4 |
| 4 | Nothing is dropped silently | Flight 1 |
| 5 | Absence is shown as unmeasured, not as zero | A judge's question in every flight |
| 6 | Evidence is not authority | A judge's question in every flight |
| 7 | A fixture result is not reported as a real one | A judge's question in every flight |
| 8 | Scaffolding carries its exit | `retire-when:` on every rule this sprint adds |

## Shape of the sprint

Five flights, in order. A flight adds one discipline and lands by itself. Each has three
rungs, and it is not called done below the third:

- **Said**: the rule stands where the act happens (the lightest `.epr-meta` signal, a skill
  line, a field in a brief), with `why:` and `retire-when:`.
- **Exercised**: used on named real work during the flight.
- **Checked**: a runnable check named on a habit, then one delta line in that habit's atom
  and `.claude/scripts/habits-project.py`.

| Flight | Discipline | Serves habit | Code touched |
|---|---|---|---|
| F0 | Correct the record, open the sprint | the three below | none (documents) |
| F1 | Nothing is dropped silently | `research-takes-have-a-home` (new, born red in F1) | one Python check |
| F2 | Evidence carries its age | `dataplane-convergence` (active, top red) | Python, bash |
| F3 | Another seat chooses the failure | `dataplane-convergence` | bash, one mesh run |
| F4 | The brief carries position and a stop condition | `agents-arrive-briefed` (red) | none (documents, skills) |

The active set stays as it is (`dataplane-convergence`, `push-delivers-within-budget`); a
plan may serve a red habit that is not active, as the 2026-11-22 plan does.

### F0. Correct the record, open the sprint

1. The steward's four rulings (below), recorded as a covenant clause.
2. Corrections into the SP-287 note (v1.4) and rows 13, 16, 17 and 24:
   - the serving receipt refuses at `dev` and `main` since 2026-11-24
     (`genesis/orchestrator/scripts/t2-receipt.sh`); only the dataplane-only branch warns;
   - habit readings and locally reused gate greens already show age
     (`.claude/scripts/habits-status.py`, `genesis/orchestrator/gate-runner.mjs`);
   - the household report already records `gitCommit` and `env.sut`; only the recovery
     timeline lacks source identity;
   - `claim` records a provider and a start date, no due date, and needs an intent or a path
     (`elohim/eprfs/epr-cli/src/flow/claim.rs`); a due date needs a reader before it is worth
     adding;
   - `.claude/workflows/recall-standing-reader.js` already runs agents judged by another
     tier; what is missing is its arrival mode;
   - `epr flow context <cid>` works only for a CID the local sidecar knows, and prints fewer
     sections; root files are 66,172 B (`CLAUDE.md`) and 71,146 B (`AGENTS.md`).
   Row 33 of `measure-family-borrows-backlog` was stale (its three habits exist, unwired).
3. A correction note on `CLAUDE.md` for "warn-only T2 receipt" (the sentence is fixed in F2).
4. The eight-lesson table in the Research Index, and an index entry for the Memorilla note.
5. Two new rows, each with an owner and a date: `measure-family-borrows-backlog` row 41 (the
   viable-system code has no fixture; the regulator has not been run against a modelled world,
   the lead candidate for the next sprint) and `arch-workspace-discipline-backlog` row 85 (no
   gate project covers the saga and habit status scripts or the mesh recovery scripts).
6. This plan published, the journal opened, cites sealed, gap items projected.

### F1. Nothing is dropped silently

- **Said.** A note declares its takes: a `takes:` list in its frontmatter, each entry an id
  and one of three homes: a cluster row (`<cluster>#<row>`), a named plan or sprint file, or
  `released: <reason>`. `genesis/research/.epr-meta` rule `mint-pass-at-close` names that
  form and the check command. One line in the `valueflow-authoring` skill package: an
  anomaly found in flight closes as a claimed commitment with a date, or as "understood, no
  action" with its two facts (the failure's history, and the redundant path or why none is
  needed).
- **Build.** `.claude/scripts/research-mint-check.py`, with a test in
  `.claude/scripts/_lib/__tests__/`. It verifies declared takes only: the row exists and
  names the note's slug, or the file exists, or a reason is given. A note with no `takes:`
  is counted as undeclared and printed as unmeasured, not as passing. `--strict` exits
  non-zero on a declared take with no home, or when the undeclared count rises above its
  best observed. Pattern to copy: the script check of
  `genesis/orchestrator/.epr-meta/security-alerts-close-in-flight.habit.md`.
- **Exercised.** `takes:` is written for the notes whose lessons this sprint touches: the
  three T-numbered notes (SP-287, ngit, Radicle; current tables only, not a superseded
  appendix), Memorilla, and the notes with stranded takes (search-discovery §10, Freenet
  4/6/10, Compass N9, Hypha lifts 4/6). Each of the 28 practice-shaped rows gets one
  disposition written into the row: exercised in a flight, claimed with an owner and a date,
  or released with a reason. The 180 build rows are not touched.
- **Checked.** New habit `genesis/research/.epr-meta/research-takes-have-a-home.habit.md`,
  born red, `checks:` the script, `retire-when:` the mint pass is mechanical. A level check
  does not fit `dev-system-equilibrium`, whose invariant is a rate; a research-take stock
  under `epr flow stocks` is recorded as the follow-up.

### F2. Evidence carries its age

- **Build.** `load_reports()` in `.claude/scripts/habits-status.py` keeps the report's
  `gitCommit`; `saga-status.py` `headline()` then prints the report's `runId`, its age, and
  the number of commits on `dev` since that commit, labelled "all paths". A count filtered
  to the dataplane closure waits until `DATAPLANE_RE` (`t2-receipt.sh`, mirrored in
  `sut.ts`) has one shared source; that is recorded in row 24, not built here.
  `recovery_write_record` in `app/elohim-app/scripts/hc-mesh-recovery.sh` adds the commit
  and the binary identity that `preflight_binary_freshness` already computes.
- **Said.** One line in the `valueflow-reviewer` skill package: a green cited in a report
  states its source and its age. The root "warn-only" sentence is fixed through
  `.epr-meta/elohim/packages/agentdocs/elohim-root-gospel.json` and re-projected, after
  `berth impact` on that package and a berth announcement; if another session holds it, the
  fix is handed to that session with a date.
- **Exercised.** The next session start on this device shows the saga line with its age. The
  flight's own hand-over states the age of each green.
- **Checked.** A real `headline()` test (the existing hook test stubs the script out) and a
  record test beside `recovery_timeline_local_test.py`, run directly and named in the
  journal. No gate project covers these files (row 85); the journal says so and does not
  report a gate as passed for them. Not in scope: the sibling reuse line, which has no clock
  reading by design.

### F3. Another seat chooses the failure

- **Build.** `app/elohim-app/scripts/hc-mesh-recovery-matrix.sh`: a draw file
  (`MESH_RECOVERY_DRAW`) fixes the scenarios, their order and the recovering slot, and is
  refused together with `MESH_RECOVERY_SCENARIOS`; the `expect` label leaves the run's
  arguments (nothing reads it, and the verdict already uses the matrix's own value); the
  record carries `chosen_by` and the draw's hash. A shell test beside
  `hc-mesh-happ-freshness.test.sh`.
- **Said.** One line in the `hc-dev-orchestrator` skill package: a recovery receipt offered
  for a dataplane change is drawn by a seat other than the change's author.
- **Exercised.** One drawn run of three scenarios on the household mesh, through
  `just mesh recovery-matrix` with the `hc-dev-orchestrator` skill loaded. That recipe claims
  the measure-class berth itself. If this berth refuses it, the override
  `MEASURE_ON_DEV_BERTH` is not set without the steward's word; the run is offered to shem
  over berth, or the journal line reads "exercise owed", not done.
- **Checked.** The test; the run's record shows `chosen_by` differs from the runner.

### F4. The brief carries position and a stop condition

- **Said.** One section, marked proposed, appended to the placement design: changes to §2.4
  and to the L3 and L4 rows. The subject is an EPR, of which a path is one case; rank against
  the session-open ceiling; deliver, make reachable or withhold; the stop condition is delivered
  whatever else is cut; arrival conditions are selected by the act (the required-or-not table) inside
  the placement read, not by a hook; a control room is this same read taken from a seat
  (SP-287 study items S8, S9). The steward rules on it; the build stays with that plan.
  A `source-priority` skill package, projected to all three runtimes: which source wins in
  each pairing and what "agree" means. It is a skill because `AGENTS.md` is already over
  Codex's 65,536-byte limit; its `retire-when:` is the root cut that makes room for it.
  `stop:` joins the dispatch lines in the `orchestrate` and `valueflow-implementer` packages.
- **Exercised.** Every dispatch in this sprint carries goal, target and checkout, rung, and
  stop condition. One arrival read, labelled as step 2's "one tier as a dry run" of the
  2026-11-22 plan, with its method recorded: one subagent sent to a target with a one-line
  task, what it received captured, the five questions asked, judged by a different tier.
- **Checked.** `epr flow cites verify` on the amended design; the arrival read recorded as a
  `--kind verdict` note and a delta in `genesis/agentic/.epr-meta/agents-arrive-briefed.habit.md`.

## How the work is run

- One worktree under `/projects/elohim/.worktrees/`, `brit` for repository operations,
  campaign state in `genesis/local-dev/research-disciplines/`.
- The work is done on ethosengine: each flight is one hand-over to
  `refs/heads/self/ethosengine-disciplines-f<N>`, corrections stacked on that ref, one final
  tip sent to the shem integrator, the selected gate run on ethosengine while it waits.
- Skill and root text is edited in its package and re-projected (`elohim-package-authoring`).
- Seats: the orchestrating session holds design, briefs and alignment review; an Opus seat
  implements; a Sonnet seat does mechanical folds; at most three agents at once.
- **The judge.** Each flight closes with one `--kind verdict` note from a seat that did not
  do the work, given the artifact and the trail only. Its questions: was the discipline used
  on real work or only written down; does any reading show zero where it should say
  unmeasured; does any evidence stand in for a decision; is any fixture result reported as
  real. The note names who implemented and who judged (model, device, what each read).
- **Stop conditions, every flight.** Stop and report when: a gate reds twice on one cause; a
  write is needed outside the covenant write set; the landing lane is occupied; a mesh
  refusal names `MESH_RESET`; a flight would add a register, ledger or ranking script other
  than the one check in F1.

## The steward's rulings (2026-11-26, one dialog)

1. **Write set**: the whole sprint, F0 to F4 (covenant clause `bafyreiawtdu3z6…bfewe`).
2. **The two untracked notes** (Memorilla, Gesell): commit both in F0.
3. **The cleanup gate** (384 against 120): run `memory-stasis-loop` between F0 and F1.
4. **Who draws in F3**: an isolated agent seat; the steward may override the draw.

## Not in this sprint (each has a row)

Any Rust: the act column on arrival conditions and a due date on `claim` (both wait for a
reader in the placement read); lessons 5 to 7 as code (`evidence_class`,
`measure-family-borrows-backlog` row 28); the per-turn run-state projection
(`agentic-harness-borrows-backlog` row 1); the arrival reader's failure injection (row 17);
placement landings L1 to L6; a research-take stock in `epr flow stocks`; the 180 build-shaped
rows; a viable-system fixture (`measure-family-borrows-backlog` row 41).

## Verification

- `python3 .claude/scripts/research-mint-check.py --strict` exits 0; 28 of 28 practice rows
  carry a disposition; the undeclared-note count is printed and recorded as best observed.
- The session-start saga line names its report, age and commits since; the newest recovery
  record carries the commit, `chosen_by` and the draw hash.
- The named tests pass when run directly; `just gate eprfs-live-corpus` for every
  documentation landing and `just gate orchestrator` where it is selected;
  `python3 .claude/scripts/habits-project.py --check`; `epr flow cites verify` on this plan
  and the amended design.
- Five verdict notes, one per flight, each from a seat that did not do the work.
- `python3 .claude/scripts/habits-status.py`: the absorbed count has risen from 8.
- Each landing verified on `origin/dev` after the integrator reports it.
