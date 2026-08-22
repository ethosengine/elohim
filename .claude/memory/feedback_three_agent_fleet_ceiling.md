---
index: false
name: three-agent-fleet-ceiling
title: "Fleet ceiling: three concurrent agents, orchestrator does design"
description: "Keep exactly 3 agents active at all times — more risks crashing the dev workspace; orchestrator watches returns, accelerates, and makes design calls."
metadata: 
  node_type: memory
  title: "Fleet ceiling: three concurrent agents, orchestrator does design"
  type: feedback
  originSessionId: 77071821-7182-463a-ae84-0c496dd5f84e
  modified: 2026-08-22T04:12:21.457Z
---

Operator directive (2026-08-22): keep **three agents active at all times** — more than that risks crashing the dev workspace (it has restarted twice under load). The orchestrator's job while the fleet runs: watch what comes back, look for opportunities to accelerate development, improve the test suite, enhance the valueflow, pursue design and performance improvements, help anything that escalates, and supply design input when an implementing agent needs it to finish coherently.

**Why:** The Che workspace container has finite memory/CPU; oversubscription (10 agents + mesh + cargo builds) preceded both container restarts on 2026-08-21/22. Three is the operator's judged safe ceiling with the mesh running.

**Terminal goal (operator, 2026-08-22):** as agents prove features working/complete on the localdev mesh, the point is to get the **resiliency saga completed** (all 11 chapters green). Queue ordering serves saga chapters first.

**How to apply:** Treat the fleet as a fixed-size worker pool: when one agent finishes, backfill from the queue immediately; never spawn a 4th while three run. The orchestrator stays out of grunt work and does judgment: triage returns, design decisions, queue ordering. Related: [[delegate-narrow-tasks-to-cheaper-tiers]], [[subagent-disjointness-read-write]].
