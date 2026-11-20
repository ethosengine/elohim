---
name: feedback_sprints_fulfil_in_the_ledgers_unit
title: "Sprints fulfil in the ledger's own unit"
description: "Operator 2026-10-18: a 55 h marathon read 0 mints / 0 discharges on the commitment ledger; a granted sprint must name ≤3 commitments, write `fulfills:` on landing, and project at close so its window is dated."
id: feedback-sprints-fulfil-in-the-ledgers-unit
metadata:
  node_type: memory
  title: Sprints fulfil in the ledger's own unit
  type: feedback
  originSessionId: 09892ed7-62ff-451d-a4cc-7871ccfa5ccb
  modified: 2026-10-18T17:53:54.929Z
cites:
  - elohim/eprfs/epr-cli/src/flow/
  - "marathon-retrospective-20261018 | Retrospective | sha256:cbef898ec7bda733 | path: genesis/docs/superpowers/sprints/2026-10-18-marathon-retrospective.md"
---

The 2026-10-12..18 marathon (shem's 40 h + my 15 h, 260 commits) produced two `fulfills:` edges
and zero dated commitment events; `epr flow stocks --window 2026-10-11..2026-10-18` returned
REFUSED (no events in window). The operator's reading: "0 of 0 after working 40+ hours straight
is concerning." The register also moved 15 → 14 green and 15 backlog rows were born against 0
closed. Full read: `genesis/docs/superpowers/sprints/2026-10-18-marathon-retrospective.md`.

**Why:** the ledger's commitments are gap lines from spec/plan docs (4 940, 74% in `draft`
docs from May–June, none DONE) plus one-shot `a2o:scenario-green` per feature file, minted only
when the hand-run `epr flow project` runs (last dated mints: September). The habit register,
which CLAUDE.md names as the unit of work, mints nothing and `claim --serves` has never been
used. A sprint that does exactly what the gospel says reads 0 of 0 by construction. The
instrument and the work were never joined; it is not that the controllers drifted.

**How to apply:** do not read a 0-of-0 stocks window as "the sprint delivered nothing"; read it
as "the ledger is not joined to the register" until S2 of the retro lands (red habit mints one
commitment, flip discharges it, regression dismisses it; draft gaps demote to intents;
scenario-green discharges per window). Until then judge a sprint on: reds moved with run ids,
regressions, and the green ratio. When S2 lands, the sprint close runs `epr flow project` and
the window must not read REFUSED. Related: [[feedback_waiting_turns_end_with_a_wakeup]],
[[feedback_memory_index_is_projected_never_hand_edited]],
[[project_shem_workspace_runs_long_sprints]].
