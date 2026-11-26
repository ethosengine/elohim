---
id: research-disciplines-sprint-journal
title: Research disciplines — sprint journal
date: 2026-11-26
status: OPEN
steward: human:matthew
author: ethosengine session cfde00a4 (Claude Fable 5.2), orchestrating; shem session cacccb55 integrating
serves: [dataplane-convergence, agents-arrive-briefed, research-takes-have-a-home]
cites: []
---

# Research disciplines — sprint journal

Plan: `../../plans/2026-11-26-research-disciplines-exercised.md`.

## What the sprint is for

The research corpus emits and does not absorb: 243 documents authored against 8 absorbed in 90
days, by the habit register's own reading. Eight lessons were each reached by two or more
research passes working from different sources. This sprint takes four of them from a row in a
backlog to a thing an agent meets when it acts, used on real work, with a check that runs. The
other three become the questions a judging seat asks at the close of every flight, and the
eighth is already practice (`retire-when:`).

## Rulings

Given by the steward on 2026-11-26 in one dialog, after he approved the plan. The first is
recorded as covenant clause `bafyreiawtdu3z6…bfewe`; the other three are recorded here in the
orchestrating session's words.

- The write set is the whole sprint, F0 to F4.
- The two untracked research notes on ethosengine (Memorilla, Gesell) are committed in F0.
- The cleanup gate (384 against a watermark of 120) is drained by `memory-stasis-loop` between
  F0 and F1. Asked again when the workflow proved hard-wired to the ethosengine main checkout
  (53 commits behind `dev`, shared with a live session): run it from a current worktree
  (covenant clause `bafyreiadnvhonv…4edu`).
- An isolated agent seat draws the failure in F3; the steward may override the draw.

## Flights

A flight is done at its third rung: said where the act happens, exercised on real work, checked
by something that runs.

| Flight | Discipline | State | Hand-over ref | On dev as |
|---|---|---|---|---|
| F0 | Correct the record, open the sprint | done: judged, landed 2026-11-26 03:43Z, verified on `origin/dev` file by file | `self/ethosengine-disciplines-f0` | `6187dff08`, `ab78d3ad0`, `baeb32247` |
| F1 | Nothing is dropped silently | said, exercised and checked; verdict and landing pending | `self/ethosengine-disciplines-f1` (carries the cleanup pass's two commits) | |
| F2 | Evidence carries its age | not started | | |
| F3 | Another seat chooses the failure | not started | | |
| F4 | The brief carries position and a stop condition | not started | | |

## F0 record

- **Corrected against the code.** The SP-287 note is at v1.4. Rows 13, 16 and 17 of
  `agentic-harness-borrows-backlog` and row 24 of `arch-dataplane-refactor-backlog` carry the
  same corrections, each marked with its date. The corrections came from three read-only
  explorers reading `origin/dev`; the orchestrating session confirmed four of them by reading
  the files (the saga headline, the receipt script's header, the scenario table, the matrix's
  label argument) and took the rest on the explorers' reading.
- **Stale row noted.** `measure-family-borrows-backlog` row 33: its three habits exist, unwired.
- **Two rows added, each with an owner and a date.** `measure-family-borrows-backlog` row 41
  (the viable-system code has no fixture) and `arch-workspace-discipline-backlog` row 85 (no
  gate project covers the saga and habit status scripts or the mesh recovery scripts).
- **Research Index.** An entry for the Memorilla note, and the table "What more than one pass
  reached". The table's citations were compiled by an explorer and not re-read by the author,
  so they were the first thing the F0 judge was asked to check.
- **Verdict: changes-requested, then approved.** The judge was an Opus seat on ethosengine,
  dispatched by the implementing session with a brief the implementer wrote, so it is a
  different tier and a separate context and not a different lab or steward. It read the diffs
  of the Research Index, the three corrected backlog rows and the note (by search), the cited
  code, and every cited take and row. It did not read the whole note, this journal on its first
  pass, or the diffs of `CLUSTERS.md`, `arch-workspace-discipline-backlog` and
  `measure-family-borrows-backlog`. First pass: every code correction true; the table wrong in
  four cells (two Nostr Compass citations swapped across lessons 2 and 5, one Unified
  Attestation study item called a take, and Radicle citations in lessons 1 and 6 that supported
  their lesson only by interpretation); ngit T2 marked weaker, not a fault. The author re-read
  those sources and corrected the table in commit `e03bee37e`: N6 moved to lesson 5 and Compass
  left lesson 2, the study item is named as one, Radicle T8 and T7 were removed, and Radicle T1
  and ngit T2 name the clause that supports the lesson. No lesson fell below two passes. Second
  pass, on that commit: approved, with a note that Memorilla T1 supports lesson 2 only loosely
  and that an earlier wording of this record said five cells where four changed. Implemented by
  the orchestrating session (Fable 5.2, ethosengine).
- **Where the verdict note lives.** It is a `--kind verdict` note on the plan, written in the
  flight worktree's flow sidecar: the main checkout on ethosengine is behind `dev` and does not
  hold the plan yet, so `--root` refused the path. The note is lost if that worktree is removed
  before the main checkout follows `dev`; this record is the durable copy. That is the
  per-checkout sidecar defect the placement design's landing L2 removes.
- **Gate.** `just gate eprfs-live-corpus` was run on ethosengine on `e03bee37e`; the commit
  after it changes this journal's wording only and was not gated here.

## Cleanup pass (between F0 and F1)

A scratch copy of `.claude/workflows/memory-stasis-loop.js`, changed only in where it runs and
in a paragraph of rules for its agents (work in the worktree, commit nothing, name the actor on
every note), ran against a fresh worktree at the F0 tip. Four rounds, eight agents, about
665,000 tokens. It stopped at convergence, not stasis: the composite score read 0.831 against
0.85 at the end as at the start.

- **Drained.** Un-captured documents, 3 to 0. One was this sprint's own plan, which had no
  checkbox stations and so was absent from the placement ledger; it gained them before the loop
  ran (commit `0a84d2bff`), which closes anomaly 6. The loop's first round captured the other
  two as gap items, each citing its source line: the 2026-11-22 agents-arrive-briefed plan (15
  items: 12 open, 3 claimed) and the participation-read design (8 items: 7 open, 1 claimed)
  (commit `856950304`). Those two files are all the loop changed.
- **Not drained, 38 claimed-only documents.** Two rounds ranked them for verification and wrote
  the ranking nowhere, so it is kept here. First, 25 task reports under `plans/` that share one
  parent, `2026-10-20-placement-contract-architecture-plan`; fifteen of them have no gap items,
  and settling their status settles the file. Then five plans whose claim rests only on ticked
  boxes, four whose stated status contradicts open boxes, one mixed plan that touches Mishpat,
  three specs with no gap items, and one sprint document. The ranker's estimate, not a
  measurement, is that one sweep against the parent plan settles about 30 of the 38. Owner: the
  steward's gathered read at this sprint's close.
- **Not drained, 19 memory entries without a cite.** The loop dispatches only the largest
  class, so it never reached these. They were not taken up afterwards either: the tracked ring
  in a worktree and the device's live ring in the main checkout differ today (the main
  checkout holds uncommitted entries), and linking entries from a worktree would set the two
  against each other. Owner: step 7 of the 2026-11-22 plan, the ring sweep on ethosengine,
  after its main checkout follows `dev`.
- **Not drained, 6 documents without a status.** Each is another lane's plan or spec
  (`2026-10-12-native-seam-guidance-design`, `2026-11-06-licence-tiers-decision`, three brit
  plans, one task report), and giving one a status is a judgment about that lane's work. Owner:
  the gathered read.
- **Two dimensions the loop cannot move.** Root instructions over their size budget (75%),
  owned by step 5 of the 2026-11-22 plan; and "docs carry explained trace links" at 0.0%, which
  has no owner this journal could find.
- **The counter is not reset.** The session-start `cleanup:` line counts drifted subjects in the
  main checkout's flow store, 384 against 120. This pass touched neither that store nor that
  checkout. Recording the reset would turn the line green with its cause unaddressed, so it is
  left as it reads and put to the steward.

## F1 record

- **Built.** `.claude/scripts/research-mint-check.py` and its test, 18 cases, run directly
  (`python3 .claude/scripts/_lib/__tests__/research_mint_check_test.py`, exit 0). No gate
  project covers the script (row 85), so no gate is reported as passed for it.
- **The design changed twice, and both are on the record.** The first design guessed takes
  from prose and was abandoned before any code, on the challenger's reading; it is recorded
  as a `--kind failed-approach` note on the plan. In flight a fourth home was added, `owed:`,
  when three Radicle takes turned out to have no row anywhere: the approved three forms
  left only "undeclared" for them, which is a silent drop. The plan says so under F1.
- **Declared.** Eight notes, 66 takes: 23 on a backlog row that names the note, 18 on a file,
  3 released with the note's own reason, 22 owed (hypha-dao 6, memorilla 6,
  hypha-distributed-press 5, radicle 3, freenet 2). Rows 14 to 17 of
  `agentic-harness-borrows-backlog` did not name the survey they came from; they do now.
- **Not declared.** 46 notes, reported by the check as unmeasured; ten of them have no
  frontmatter, and two that have it carry no `id` (hypha-distributed-press, whose five takes
  are therefore all owed, and search-discovery, left undeclared because its only list is
  "proposed, not yet folded" and whether that is its take list is its author's to say).
- **Dispositions.** The inventory's 28 lines named 29 rows. All 29 carry one: six exercised
  with their evidence named, five claimed by a flight of this sprint, three claimed by an
  existing plan or habit, two carried as a judge's question, one released, twelve held for
  one gathered read at the sprint's close.
- **Said.** The `mint-pass-at-close` rule names the declared form and the command, and was
  seen firing with the new text on the next write in that directory. The research
  directory's governance moved to the directory form (`.epr-meta/manifest.md`) so the habit
  could live beside its rule. The `valueflow-authoring` skill carries the closing rule for an
  anomaly met in flight, projected to the three runtimes and their fixture copies
  (`.epr-meta/elohim/projections/`, which the write-set clause covers in its words, "with
  their projections", and not in a glob).
- **Checked.** Habit `research-takes-have-a-home`, born red. Its strict run exits 1 on
  exactly the 22 owed takes; the approved plan's "a strict run exits 0" was replaced, and the
  plan says why.
- **Memorilla.** A correction note is recorded on the note: its Outputs describe a mint pass
  and an index entry that are not in the tree. Its index entry was moved under the Discovery
  Problem, where the note says it belongs.
- **Seats.** The check was written by an Opus seat from a brief, with one follow-up; two
  small changes (a bare `takes:` key is unmeasured; the `owed:` form) were made by the
  orchestrating session. The declarations were made by a Sonnet seat in two passes, with the
  check as its acceptance test, and re-run by the orchestrating session. The dispositions
  are the orchestrating session's.
- **Brit gaps met.** `brit rebase` refuses with "shallow repositories require Git"; `brit mv`
  is a bridge to git; cherry-pick is absent. Each fallback was stated where it was used.

## Stop conditions, every flight

Stop and report when: a gate reds twice on one cause; a write is needed outside the covenant
write set; the landing lane is occupied; a mesh refusal names `MESH_RESET`; a flight would add
a register, ledger or ranking script other than the one check in F1.

## Anomalies

Each closes as a commitment with an owner and a date, or as "understood, no action" with the
failure's history and the redundant path.

| # | Seen | Anomaly | Disposition |
|---|---|---|---|
| 1 | 2026-11-26 | Root `CLAUDE.md` calls the push-time receipt "warn-only"; since 2026-11-24 it refuses a serving change at `dev` and `main` | Owner: this sprint. Correction note recorded in F0; the sentence is fixed in F2 through the root package |
| 2 | 2026-11-26 | No gate project covers `saga-status.py`, `habits-status.py` or `hc-mesh-recovery*.sh` | Owner: this sprint, row 85. Closed or released with a reason by the sprint's close |
| 3 | 2026-11-26 | A gate verdict run on ethosengine for a handed-over commit did not stand in on shem: the integrator picked the commit onto a newer tip, which changed the closure, and shem ran the gate itself (262 s) | Understood, no action. History: one of three hand-overs that day; the first was reused (69 s). Redundant path: the integrator's own gate run, which is what happened. Practice kept: rebase onto the fetched `origin/dev` tip immediately before the gate run |
| 4 | 2026-11-26 | The ethosengine main checkout is more than 41 commits behind `origin/dev`, holds three local commits that duplicate landed ones, and has two stranded autostash entries | Owner: the steward, told in the session report of 2026-11-26. Outside this sprint's write set |
| 5 | 2026-11-26 | A `PostToolUse:Edit` hook timed out once during the research session | Owner: the steward, told the same day. Not investigated; outside this sprint's write set |
| 6 | 2026-11-26 | The plan's F0 step "`epr flow project <plan>`" cannot be done as written: the installed `epr` takes no path there, and a hand-written gap-items file sits outside the sprint's write set | Understood, no action in F0. History: first occurrence; the `/plan` command's text is stale and a correction note is recorded on its package. Redundant path: this journal's flight table and the habit deltas carry the sprint's state. Closed 2026-11-26: the coverage read counts a plan as captured when it yields checkbox stations, so the plan gained a Stations section (commit `0a84d2bff`) and left the un-captured list |
| 7 | 2026-11-26 | The F0 verdict note, written from the flight worktree without `--root`, recorded `human:matthew` as its actor: the worktree's flow sidecar holds no actor claim for the session, and the note took the participant | Closed the same day: correction `bafyreic65f6s4n…x3pe` names the mis-attributed note, and the verdict is restated with its actor named and closes that correction. The first note cannot be withdrawn (notes are append-only). Practice kept for this sprint: every note written from a worktree passes `--as`. The cause is the per-checkout sidecar, already owned by landing L2 of the placement design |
| 8 | 2026-11-26 | The cleanup workflow dispatches only the largest pressure class and says "from /projects/elohim" in every prompt: it ranked one class twice, wrote the ranking nowhere, never reached two smaller classes, and as written would have edited a stale shared checkout | Correction note recorded on `.claude/workflows/memory-stasis-loop.js`. Owner of the fix: the gathered read (the workflow is outside this sprint's write set) |
| 9 | 2026-11-26 | The `cleanup:` gate still reads 384 against 120 after the pass | Owner: the steward. The reset is his to record or withhold; the count lives in a checkout this sprint did not touch |
| 10 | 2026-11-26 | A cherry-pick refused over a staged rename left its sequence pending; a later `--continue` treated the next commit as the first pick's result and skipped that pick | Closed the same day: seen by checking the result (the plan had no stations), repaired by picking the skipped commit alone. Understood, no further action. History: first occurrence. Redundant path: the result was read before anything was pushed |
| 11 | 2026-11-26 | Twelve research notes cannot declare takes as they stand: ten have no frontmatter and two have no `id` | Owner: the gathered read. The check counts all of them as unmeasured meanwhile |
