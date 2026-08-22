---
name: feedback_agent_fleet_and_harness
title: Agent fleet, delegation & harness traps (umbrella)
description: "Keep 3 agents max; delegate narrow tasks to cheaper tiers; pin reader agents to an explicit model ID; subagent disjointness = read-set ∩ write-set; orphan cargo locks, StructuredOutput hangs, skew."
metadata:
  node_type: memory
  type: feedback
---

# Agent fleet, delegation & harness traps (umbrella)

Folds the multi-agent fleet-sizing, delegation and workflow-harness trap cluster. Members:

- [[feedback_three_agent_fleet_ceiling]] — Keep exactly 3 agents active at all times — more risks crashing the dev workspace; orchestrator watches returns, accelerates, and makes design calls.
- [[feedback_delegate_narrow_tasks_to_cheaper_tiers]] — Operator directive 2026-07-02: top-tier agent fleets burn the session limit — delegate narrow, crisply-defined tasks to opus/sonnet; keep the top tier for orchestration and judgment.
- [[feedback_pin_reader_agents_to_older_opus]] — Opus 5 handles complexity well but writes less accessibly — pin blind-reader to a full model ID, not the floating `opus` alias.
- [[feedback-subagent-disjointness-read-write]] — Parallel subagents are disjoint only if neither's read-set intersects the other's write-set; a porter reading source another task deletes is NOT disjoint.
- [[feedback_codex_side_delegation_queue]] — Well-specified disjoint tasks belong in genesis/data/timeline/backlog (not session lists) so ANY agent — Claude, Codex, Gemini — can claim them; offer during CI waits
- [[feedback_workflow_long_cargo_orphan_lock]] — Bash timeout orphans cargo still holding .cargo-lock; let it finish (work lands on disk), keep one profile per gate phase, run_in_background for >10min cargo.
- [[feedback_workflow_structuredoutput_hang]] — schema'd workflow agents retry empty {} StructuredOutput forever (48→481 calls, no completion notify) and hang the run — go schemaless prose + stall-watcher
- [[feedback_subagent_liveness_clock_skew]] — Container clocks skew hours apart — never infer agent death from transcript mtime vs date; check writer-relative freshness and TaskStop live racers first.
- [[feedback_overnight_permission_stalls]] — An idle overnight session may be blocked on a permission prompt (auth paths), not done; check the transcript tail and never race a blocked session.
