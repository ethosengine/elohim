---
name: feedback_shift_drive_cure_not_stage
title: Shift = drive the cure end-to-end, not stage it
description: In a /shift you own the cure end-to-end: stabilize with the obvious lever, then implement the confirmed fix; "staged/gated" past diagnosis is timidity.
metadata:
  type: feedback
---

2026-06-14 (overnight integration shift): the operator called out timidity. I had (a) staged the warm_stream fix as "deploy-gated / operator-gated" and waited instead of implementing it, and (b) let my own H3 framing ("orthogonal to CPU") talk me OUT of proposing the obvious CPU-bump stabilizer — so the operator had to step in, diagnose, and apply it himself.

**Why:** In a `/shift` I am a first-class developer who OWNS the work end-to-end (agentic-developer principle 1 + principle 8: drive the surface to stasis, leave no orphan). Staging proposals for "later/someone" while the system crashloops is timidity, not discipline. systematic-debugging's "no fix before confirmed root cause" and the advisor's "don't pre-write the fix" were correct BEFORE confirmation — but once the root cause is confirmed, they stop gating; keeping the "staged" label past that point is an excuse for not acting.

**How to apply:**
1. **Stabilize first with the simplest lever** (more CPU / headroom / a timeout), THEN refine the theory. Never let a precise hypothesis dismiss an obvious stabilizer — "orthogonal to X" is a reason to ALSO try X for fast relief, not to skip it.
2. **Once root cause is confirmed + the fix is in scope + budget exists, IMPLEMENT it** (TDD → deploy → verify). Don't stage it.
3. **"Operator wants resolution / keep observing" ≠ "don't fix."** It means fix while keeping observability (e.g. keep debug logging on) — unless they explicitly forbid the change.
4. **Driving ≠ reckless.** Still verify (tests, CI gate, deploy-and-watch) — but own the end-to-end cure, don't hand the last mile to the operator.

A `/shift` with explicit operator authorization OVERRIDES the default [[feedback_commit_only_integrator_pushes]] stance — drive dev. Related: [[feedback_skip_brainstorm_gates_self_answer]], [[project_doorway_wedge_unbounded_mongo_await]].
