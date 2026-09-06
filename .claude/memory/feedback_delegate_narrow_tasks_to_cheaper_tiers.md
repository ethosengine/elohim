---
index: false
name: feedback_delegate_narrow_tasks_to_cheaper_tiers
title: Delegate narrow tasks to Opus/Sonnet; top-tier fleets burn the limit
description: "Delegate narrow, crisply-defined tasks to Opus/Sonnet; keep top-tier fleets for orchestration and judgment to avoid session limit burn."
metadata:
  node_type: memory
  type: feedback
---

**Why:** a 100-agent reconcile + a 6-dimension review running verify fan-outs on the session's
top-tier model hit the usage limit mid-arc (2026-07-02; workflow verify stages died with
"session limit" errors and had to be re-run on `model: 'opus'`). The work itself was
tier-insensitive — mechanical per-file edits, adversarial code-verification against disk.

**How to apply:** when a task is narrow enough to define crisply (one file, one finding, one
disposition table), dispatch it with an explicit `model: 'opus'` / `'sonnet'` override (Agent
tool or workflow `opts.model`) and reserve the top tier for orchestration, synthesis, and
judgment calls. Also pace parallel fleets after any 429/limit failure — resume sequentially
(SendMessage to the same agent keeps its context) instead of relaunching the burst. Sibling:
[[feedback_workflow_structuredoutput_hang]] (schemaless prose returns for workflow agents).
