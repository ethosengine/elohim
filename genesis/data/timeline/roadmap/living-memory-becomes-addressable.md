---
id: "roadmap-living-memory-becomes-addressable"
kind: "roadmap"
contentType: "roadmap-item"
contentFormat: "markdown"
title: "Living-memory becomes addressable in the protocol substrate (not just .claude/)"
slug: "living-memory-becomes-addressable"
written: "2026-05-14"
author: "cartographer"
status: "proposed"
target_window: "2026-H2"
themes: [living-memory, EPR, content-addressing, DHT-graduation]
relatedNodeIds:
  - "memory:project_memory_in_repo_two_tier"
  - "memory:project_wisdom_resolves_into_epics"
  - "memory:project_first_class_graph_pattern"
  - "memory:project_reach_earned_at_authoring"
  - "memory:project_epr_substrate_vs_vf_graphql"
  - "epic:living_memory"
tags: [roadmap, memory-graduation, DHT-attestation, EPR]
---

# Living-memory becomes addressable in the protocol substrate

## Direction

Memory entries today are markdown files in .claude/memory/. They're team-shareable
(git-tracked, PVC-recoverable) but they're not first-class protocol citizens. By H2 2026,
the graduation lifecycle should be: a memory entry that survives multiple ceremonies
and earns enough convergent reach gets attested in the DHT as an EPR-addressed lesson —
becoming a node in the project's first-class graph alongside humans, devices, and stories.

## Why this matters

- `project_wisdom_resolves_into_epics`: memory's destination is story-compaction, and
  stories are protocol-citizens. So memory's terminal state should also be a protocol-
  citizen, not a file in a config directory.
- `project_reach_earned_at_authoring`: reach is the substrate's way of deciding what
  matters; right now memory entries can't earn reach because they're not addressable.
- `project_first_class_graph_pattern`: nodes-and-edges, not tables. Today memory is
  tabular (filename = primary key); it should be a node like everything else.

## What it would feel like to have achieved this

- Operator says "graduate this entry" → cartographer issues a `memorialize` primitive →
  EPR notarized, content-addressed CID, reach-policy attached, appears as a node in
  the epic-graph next to humans and stories
- The librarian's dedupe-scan can ask the DHT "is this an attested lesson?" and route
  hits differently
- A future agent (or a future operator) can ask "what's the protocol's wisdom on X?"
  and get a content-addressed answer, not a grep over .claude/memory/

## Current state

- Memory in repo (working memory tier) is stable
- Stories catalog is first-class with EPR-style ids (epr:experience-story/...) — the
  shape is already chosen
- Timeline catalog uses ContentNode-aligned ids; the on-ramp to DHT is explicit
- lamad manifest has work-story / work-project types; chronicle-entry, roadmap-item,
  backlog-item are TBD (CONVENTIONS.md open question 1)

## Next inflection points

- CONVENTIONS.md open question 1 — declare chronicle-entry, roadmap-item, backlog-item
  in the lamad manifest, OR lean on work-story / work-project
- First chronicle entry seeded as EPR (test the path)
- First memory entry graduated to story → memorialized to deep tier (test the lifecycle)
- Reach-policy for memory entries (who can attest a graduation?)

## Out of scope for this horizon

- Reach-policy substrate beyond Stage-1 social trust
- Cross-peer memory entry replication (P2P-attested wisdom)
- Memory-entry-as-EPR validators (HDI restrictions per project_hdi_no_get_links_in_validators)
