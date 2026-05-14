---
id: "backlog-story-memory-team-came-online"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Write canonical story: memory-team-came-online (collective-memory-team / as-protocol-stewards / living-memory)"
slug: "story-memory-team-came-online"
written: "2026-05-14"
author: "cartographer"
status: "proposed"
priority: "high"
relatedNodeIds:
  - "memory:project_signal_driven_audit_ceremonies"
  - "memory:project_memory_in_repo_two_tier"
  - "memory:project_historian_pattern_surface_agent"
  - "memory:project_memory_lifecycle_comet_shape"
  - "memory:project_three_temporal_perspectives"
  - "memory:project_wisdom_resolves_into_epics"
  - "memory:reference_mempalace"
  - "epic:living_memory"
tags: [story, living-memory, founding-moment, graduation-anchor]
shift_objective: |
  Author a canonical story under genesis/data/stories/ with the triple
  (collective-memory-team, role-as-protocol-stewards, living-memory). The story dramatizes
  this ceremony — the moment four agents (librarian, historian, storyteller, cartographer)
  came online and ran their first coordinated audit. The narrative weight is: the substrate
  noticed itself getting heavy (MEMORY.md breached its budget) AT the moment the team built
  to manage that weight became operational. That's the founding moment; it's why the team
  exists. Use the storyteller agent. The story is a graduation anchor for ~6 memory entries
  (signal_driven_audit_ceremonies, memory_in_repo_two_tier, historian_pattern_surface_agent,
  memory_lifecycle_comet_shape, three_temporal_perspectives, wisdom_resolves_into_epics) —
  list each in the story's frontmatter under graduated_memory[]. Resolve the role-as-protocol-
  stewards role record OR document why we punt and reference role-as-stewardee adjacency.
  Done when (a) story exists at status:draft; (b) operator confirms canonical; (c) the
  6+ memory entries are flagged for librarian graduate-during-next-cleanup-pass; (d) Stories
  INDEX.md updated.
---

# Write the canonical memory-team-came-online story

## Why this matters

The living_memory epic exists in the spec but has no human-readable anchor. The memory
team itself just landed — librarian, historian, storyteller, cartographer agents
defined; signal-driven ceremonies; _lib pattern; timeline scaffold. This very ceremony
is the founding moment. If it doesn't get memorialized as story, it becomes "obvious in
hindsight" and the entries that explain *why* it had to be built get harder to retire
without losing the reasoning.

This story is also the single highest-leverage graduation target. ~6 memory entries
become safe-to-archive once the story carries the lesson. That's working-memory headroom
restored directly.

## What's blocking

- `role-as-protocol-stewards` role record doesn't exist; closest is `role-as-stewardee`
  (also missing per Stories INDEX). Operator decision needed: create the role, or punt
  to an adjacent role for now?
- The collective subject `collective-memory-team` is new; precedent is
  `collective-maintainers` referenced in INDEX coverage gaps.

Both blockers are operator-decisions, not technical work. The story can begin in draft
while they're resolved.

## What's ready

- All carrier memory entries are stable and recent (none under active revision)
- The ceremony itself is live — storyteller can witness Waves 1-6 as primary source
- Storyteller agent is the right author per LIFECYCLE.md
- Stories INDEX has a section for living-memory under "themes with no story"

## Who knows the area

Storyteller. Cartographer dispatches; storyteller composes; operator confirms canonical.

## Convergence

- Storyteller Wave 2: HIGHEST LEVERAGE NEEDS-NEW-STORY pick (4 carry entries + living_memory epic)
- Historian Wave 1: flagged the founding-moment precedent
- Librarian Wave 1: 6 graduation candidates is the biggest single MEMORY.md compression event available

## Definition of done

1. Story file at genesis/data/stories/<slug>.md, status:draft
2. Frontmatter includes graduated_memory[] listing all 6+ entries
3. Stories INDEX updated under By theme:living-memory, By subject:collective-memory-team, By role:as-protocol-stewards
4. Operator review → status:canonical
5. Librarian flag-list: entries safe-to-graduate at next cleanup pass
