---
description: Kick off an agentic developer shift — interactive Objective authoring, pre-shift readiness check, iteration loop, sprint result on close.
---

# /shift

Invokes the `agentic-developer` skill to run an agentic developer shift.

## Usage

- `/shift` — interactive kickoff (author Objective live, compose palette, start iteration)
- `/shift resume <shift-id>` — *(v2, not yet implemented)* resume a bailed shift after operator answers the bail question

## What it does

1. Runs the `generalize-permissions` skill on the current allowlist
   (bulk-collapse proposals).
2. Interviews the user for the Objective (name, measure command,
   baseline, scope, budget).
3. Composes a shift id, writes Objective YAML to
   `.claude/shifts/<shift-id>.objective.yaml`, writes initial journal
   to `.claude/shifts/<shift-id>.journal.md`.
4. Pattern-matches the predicted command palette against current
   allowlists; proposes shift-scoped additions to
   `.claude/settings.local.json` for user approval.
5. Runs `pnpm run agentic:readiness -- --objective <path>`. Aborts
   on any readiness failure with a report.
6. Enters the iteration loop, using `ScheduleWakeup` to pace between
   iterations until done, bail, or budget exhaustion.
7. On terminal state, writes a sprint result markdown at
   `.claude/shifts/<shift-id>.journal.md` and prints its path.

## See also

- Skill: `.claude/skills/agentic-developer/SKILL.md`
- Spec: `genesis/docs/superpowers/specs/2026-04-16-agentic-developer-loop-design.md`
- Templates: `genesis/docs/shifts/`, `genesis/docs/retrospectives/TEMPLATE.md`

## Loading the skill

Use the `Skill` tool with `skill: agentic-developer`.
