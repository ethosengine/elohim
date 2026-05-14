---
id: "backlog-skill-catalog-hygiene"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Routing-layer hygiene — skill descriptions + agent definitions + trigger overlap (both axes)"
slug: "skill-catalog-hygiene"
written: "2026-05-14"
author: "cartographer"
status: "proposed"
priority: "medium"
relatedNodeIds:
  - "memory:project_three_temporal_perspectives"
  - "memory:project_signal_driven_audit_ceremonies"
tags: [memkit, skills, agents, routing-layer, always-loaded]
shift_objective: |
  Two-axis hygiene of the always-loaded routing layer — skills AND agents. Wave 1 surfaced
  3 skill-trigger-overlap pairs + 6 vague skill descriptions. The audit-script fix at Wave 4
  unmasked 29 agent-trigger-overlap pairs + 12 vague agent descriptions + 18 over-imperative
  + 18 tools-mismatch (frontmatter tools list out of sync with what's actually used) + 6
  drifted-factual agents. Both axes share the same failure mode: dispatcher behavior becomes
  non-deterministic when triggers overlap, and invisible-to-dispatcher when descriptions are
  empty/vague. The fix is word-level edits to .claude/skills/*/SKILL.md and .claude/agents/*.md
  frontmatter, anchored on the temporal-perspective discipline where applicable.

  Done when: skill-audit.py reports 0 trigger-overlap pairs and 0 vague descriptions; agent-audit.py
  reports the same; tools-mismatch findings resolved (either update frontmatter to match usage
  or remove tools from list); the temporal-triad routing rules in .claude/scripts/memory-kit/CLAUDE.md
  still hold.
---

# Routing-layer hygiene (skills + agents)

## Why this matters

Always-loaded skill descriptions AND agent descriptions are the routing layer. They're
the surface that decides "which capability handles this request." When they overlap on
keywords, the dispatcher picks one essentially by hash — non-deterministic at the
dispatch layer. Empty/vague descriptions are even worse: invisible to the dispatcher.

Two axes, same failure mode:

**Skill axis** (from Wave 1 skill-audit, single source):
- `quality-orchestrator` + `holochain-import` have empty descriptions
- `ci-triage` description is 52 chars — communicates nothing about when to pick
- `converge` ↔ `memory-kit` share 5 keywords (could split-fire); the temporal-triad
  in `project_three_temporal_perspectives` gives the disambiguation axis (converge =
  future-tense synthesis; memory-kit = present-tense hygiene)
- 3 more vague descriptions across the catalog

**Agent axis** (from post-fix agent-audit, Wave 4 cascade-unmask — 19 agents total):
- 29 trigger-overlap pairs across the agent catalog (vs 3 for skills — agent space is
  more crowded and overlap-prone)
- 12 vague descriptions in agent frontmatter
- 18 over-imperative (directive density too high, likely formatting noise)
- 18 tools-mismatch (frontmatter `tools:` list out of sync with what the agent actually
  uses — false claims about capability)
- 6 drifted-factual (agent definition references things that have moved)

The agent axis is bigger and more impactful than the skill axis: trigger overlap there
shapes which agent gets dispatched when the operator says "do X." The skill axis is
about which skill loads on operator command.

## What's blocking

Nothing. The fixes are word-level edits to two file classes. Tools-mismatch may require
short audits of what each agent actually invokes — but the agent-audit report
enumerates the gaps.

## What's ready

- Wave 1 skill-audit report at `.claude/memory-kit/2026-05-14/skill-audit.md`
- Wave 4 agent-audit report at `.claude/memory-kit/2026-05-14/agent-audit.md`
  (post-cascade-fix version; the doubled-path version was removed)
- Temporal-triad disambiguation axis in `project_three_temporal_perspectives`
- Both audit scripts now functional (post-Wave-4 fix) and re-runnable for validation

## Convergence

- Librarian Wave 1: skill-audit output (3 overlap, 6 vague)
- Cascade unmask in Wave 4: agent-audit output (29 overlap, 12 vague, 18 imperative,
  18 mismatch, 6 drift) — only visible because the audit-script-discovery fix landed
- Cartographer Wave 3 + retro: convergence-bias caveat — fix the routing layer so
  the temporal-perspective discipline holds at dispatch time

## Definition of done

1. `quality-orchestrator` + `holochain-import` have non-empty SKILL.md descriptions
2. `ci-triage` SKILL.md description ≥ 100 chars and communicates when-to-pick
3. `converge` SKILL.md description leads with "future-tense synthesis" framing
4. `memory-kit` SKILL.md description leads with "present-tense hygiene" framing
5. Remaining 3 vague skill descriptions tightened
6. All 12 vague agent descriptions tightened (frontmatter `description:` field)
7. All 18 tools-mismatch agents reconciled (update frontmatter to match usage)
8. 29 agent trigger-overlap pairs disambiguated — collapse where redundant, split
   where genuinely different
9. 6 drifted-factual agent definitions corrected
10. 18 over-imperative agents toned down (target ≤ X imperative-density per
    `_lib/drift_score.py` thresholds)
11. `skill-audit.py` post-fix: 0 overlap, 0 vague
12. `agent-audit.py` post-fix: 0 overlap, 0 vague, 0 tools-mismatch, 0 drifted-factual,
    no over-imperative flags

## Suggested approach

Do this as a single shift, but in stages within it (each stage gates on its own
audit re-run):
1. Skills first (smaller surface, faster validation loop)
2. Agent descriptions + tools-mismatch (frontmatter-only edits)
3. Agent body content (over-imperative + drifted-factual)
4. Agent trigger-overlap (the 29 pairs — requires judgment per pair)

Re-run the relevant audit after each stage; budget cascade-hidden surfacing (per
`feedback_cascade_hidden_test_surface`).
