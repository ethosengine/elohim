---
index: false
name: feedback_plan_checkbox_is_commitment_grain
title: In a plan, `- [ ]` is the commitment grain — one per task, never per step
id: feedback-plan-checkbox-is-commitment-grain
description: "epr flow project mints one gap-item/intent per `- [ ]` line in a plan; the writing-plans skill's per-step checkboxes minted 47 intents for a 17-task plan (2026-09-11). Put the checkbox on the task heading, number the steps."
metadata:
  type: feedback
---

**What happened (2026-09-11):** the governed-discovery stations 0–3 plan followed the writing-plans
skill's `- [ ] **Step N**` convention; `epr flow project` minted 47 intents for it (one per step),
inflating the commitments stock that `dev-system-equilibrium` measures. Re-shaping to one `- [ ]`
per `### Task` heading (steps numbered `1.`) re-projected to 17.

**Why:** `epr flow project` extracts checkbox stations from plans and specs (the kit's
`decompose.py` grain, ported in the memory-kit replacement); a checkbox IS a mintable commitment,
and `epr flow claim --on plans__<slug>#N` takes it. Steps are not claimable units.

**How to apply:** in any plan or spec under `genesis/docs/superpowers/`, checkbox only the units a
seat claims and a reviewer gates (stations or tasks); keep the skill's step discipline as numbered
lines. Say so in the plan header. Repo semantics override the skill here (CLAUDE.md: user/project
instructions take precedence). Related: [[project_valueflow_authoring_surface_landed]].
