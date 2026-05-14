---
id: "backlog-memory-md-size-discipline"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Tighten MEMORY.md to budget + add size-discipline as first-class ceremony"
slug: "memory-md-size-discipline"
written: "2026-05-14"
author: "cartographer"
status: "proposed"
priority: "high"
relatedNodeIds:
  - "memory:project_memory_lifecycle_comet_shape"
  - "memory:project_signal_driven_audit_ceremonies"
  - "memory:project_wisdom_resolves_into_epics"
tags: [memkit, MEMORY.md, ceremony-design, comet-head]
shift_objective: |
  Two-part work. Part A (tactical, librarian-led, can run today): tighten the ~10 longest
  MEMORY.md index entries to <=200 chars each so the file fits its 24.4 KB budget. Preserve
  load-bearing one-line summaries; move detail into the topic files they link to. Part B
  (structural, cartographer-scoped): add MEMORY.md byte-size as a first-class drift signal
  in .claude/memory-kit/claude-md-drift.json (or a new memory-md-drift.json), with threshold
  at 90% of budget, and wire it into the existing accumulator-+-ceremony pattern from
  project_signal_driven_audit_ceremonies. The ceremony should surface candidate entries to
  tighten (ranked by char-overage), not auto-edit. Done when (a) MEMORY.md loads without
  truncation warning; (b) memory-review.py reports byte-budget as one of its tracked metrics;
  (c) the next ceremony will surface size pressure before it breaches.
---

# MEMORY.md size discipline

## Why this matters

Wave-1's most pointed finding: working-memory index breached its own loading budget at
the exact moment the memory team became operational. This is the substrate failing the
discipline it was built to enforce. The fix is twofold — tighten now, then add the signal
so the breach can't happen silently again.

The deeper resonance (historian Wave 1): MEMORY.md tripping its size warning at the
moment the team that manages it came online is a precedent worth memorializing. It's
*why* we built the team. The structural fix lets this be the last time it happens
without notice.

## What's blocking

Nothing for Part A. Part B benefits from the audit-script-discovery fix landing first
so the accumulator's path resolution is correct, but is otherwise self-contained.

## What's ready

- Librarian has tiny-correction authority over MEMORY.md per LIFECYCLE.md
- Accumulator pattern + threshold pattern exist (claude-md-drift.json)
- memory-review.py already reports MEMORY.md size; just needs to gain budget-comparison
- The ~10 longest entries are surfaced by Wave 1 report directly

## Who knows the area

Librarian (primary owner of MEMORY.md curation). Cartographer scopes Part B.

## Convergence

- Librarian Wave 1: top finding 1, byte budget
- Historian Wave 1: precedent moment, system-breaches-its-own-discipline
- Storyteller Wave 2: JOINT ACTION — librarian acts immediately + cartographer elevates structural

## Definition of done

1. MEMORY.md <= 24.4 KB (loadable in full)
2. memory-review.py reports byte budget with traffic-light status
3. Signal accumulator tracks size-pressure; threshold pushes a ceremony invitation
4. Memory entry: project_signal_driven_audit_ceremonies updated to reference the new signal
