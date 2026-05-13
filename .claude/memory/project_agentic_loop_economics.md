---
name: Agentic loop economics — iterations are the cost, not tokens
description: In this project's agentic developer loops, optimize for iteration count (wall-clock, real pipeline runs), not per-iteration token cost. Use top-tier models for orchestration and attempts.
type: project
originSessionId: 9a934a92-144d-4415-9d43-14fcb046e2db
---
For agentic developer loops in this project (CI/pipeline work, overnight shifts, anything where each iteration triggers real infrastructure like a Jenkins build):

**The scarce resource is iterations, not tokens.** One iteration = one real pipeline run = ~1 hour wall-clock + real compute cost + potential deployment side effects. A cheap model that needs 5 iterations to converge is far more expensive than an expensive model that converges in 2.

**Why:** User explicitly rejected the "Sonnet orchestrates, Opus judges" tier split during brainstorming because loops are expensive — "really understanding the deep causes why and making decisions as to how to really achieve the objective is what I want Opus to do." Shallow fixes that need re-iteration dominate the cost curve.

**How to apply:**
- When designing agentic loops: Opus orchestrates every iteration and attempts the work.
- Haiku's role is strictly data reduction (log compression, structured summaries) — never decisions.
- Sonnet's role, if any, is as Opus's delegate for specific sub-tasks Opus assigns — not autonomous orchestration.
- Tier graduation inside an iteration (Haiku reduces → Opus decides/acts) is fine; tier graduation across iterations (Sonnet handles "easy" iterations) is not.
- Do not propose "periodic Opus safety cadence" designs — they only make sense when a lower tier orchestrates between safety checks, which this project rejects.
