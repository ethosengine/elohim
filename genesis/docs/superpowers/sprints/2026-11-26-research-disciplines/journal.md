---
id: research-disciplines-sprint-journal
title: Research disciplines — sprint journal
date: 2026-11-26
status: OPEN
steward: human:matthew
author: ethosengine session cfde00a4 (Claude Fable 5.2), orchestrating; shem session cacccb55 integrating
serves: [dataplane-convergence, agents-arrive-briefed]
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
  F0 and F1.
- An isolated agent seat draws the failure in F3; the steward may override the draw.

## Flights

A flight is done at its third rung: said where the act happens, exercised on real work, checked
by something that runs.

| Flight | Discipline | State | Hand-over ref | On dev as |
|---|---|---|---|---|
| F0 | Correct the record, open the sprint | judged (changes-requested, fixed); handed over | `self/ethosengine-disciplines-f0` | |
| F1 | Nothing is dropped silently | not started | | |
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
| 6 | 2026-11-26 | The plan's F0 step "`epr flow project <plan>`" cannot be done as written: the installed `epr` takes no path there, and a hand-written gap-items file sits outside the sprint's write set | Understood, no action in F0. History: first occurrence; the `/plan` command's text is stale and a correction note is recorded on its package. Redundant path: this journal's flight table and the habit deltas carry the sprint's state. The plan is therefore absent from the placement ledger, which is put to the steward with the cleanup pass |
| 7 | 2026-11-26 | The F0 verdict note, written from the flight worktree without `--root`, recorded `human:matthew` as its actor: the worktree's flow sidecar holds no actor claim for the session, and the note took the participant | Closed the same day: correction `bafyreic65f6s4n…x3pe` names the mis-attributed note, and the verdict is restated with its actor named and closes that correction. The first note cannot be withdrawn (notes are append-only). Practice kept for this sprint: every note written from a worktree passes `--as`. The cause is the per-checkout sidecar, already owned by landing L2 of the placement design |
