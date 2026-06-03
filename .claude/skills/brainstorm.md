---
name: brainstorm
description: Brainstorm (coherence-wrapped)
---

# Brainstorm Skill

Wraps `superpowers:brainstorming` with the deterministic pre/post seam defined in
`.claude/commands/brainstorm.md`. That command is the authoritative step-by-step reference;
this file carries the project-level overrides and scope rules.

## Scope judgment (run FIRST, before any ceremony step)

**Judge scope before loading the full ceremony.** If the change is one file, one short
script, or a config edit with no architectural implications — present the design inline,
get verbal approval, implement, done. Skip Steps 6–9 (spec doc, self-review, user reviews,
writing-plans). Mark any spec/plan brainstorming tasks as `deleted`, not `completed`.

Full ceremony (spec doc → `genesis/docs/superpowers/specs/`, writing-plans handoff) is
reserved for substantive features: new service, new pillar surface, multi-component
refactor, or anything requiring cross-session coordination.

The `superpowers:brainstorming` skill itself notes "The design can be short (a few
sentences for truly simple projects)" — this rule extends that latitude to skipping
the artifact entirely for genuinely minor changes.
