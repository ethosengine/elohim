---
name: agent-agnostic-backlog-delegation
title: Disjoint side-tasks go to the shared backlog
description: "Well-specified disjoint tasks belong in genesis/data/timeline/backlog (not session lists) so ANY agent — Claude, Codex, Gemini — can claim them; offer during CI waits"
metadata: 
  node_type: memory
  title: "Disjoint side-tasks go to the canonical backlog, agent-agnostic"
  type: feedback
  originSessionId: d9dfc541-741f-46d8-b268-23888610876f
  modified: 2026-07-29T13:01:35.736Z
---

Operator direction (2026-07-29): maintaining delegable side-tasks is general backlog discipline, not a per-runtime queue. The ideal pattern is a repo-canonical backlog any arriving agent can claim from — expanding sprint capacity across Claude, Codex, Gemini, etc.

**Why:** A session-local "Codex list" dies with the session and is invisible to other runtimes; the canonical backlog (genesis/data/timeline/backlog/, timeline-CONVENTIONS-conformant) is the shared surface all agents and triage loops already read.

**How to apply:** When a disjoint, crisply-specified, self-verifiable task surfaces mid-sprint, write it as a conformant backlog entry (scope + DoD + verification command) instead of parking it in memory or chat. During pipeline/test waits, point the operator at claimable entries for side-delegation.
