---
index: false
name: hook-bypass-integration-shakeout
title: Hook-bypass grant for CI-pipeline integration shakeout
description: The agent working ON the CI pipeline may push --no-verify during integration shakeout only if gates already ran green; CI becomes its verification surface.
metadata: 
  node_type: memory
  type: feedback
  originSessionId: da9eca9d-9d7f-4a87-83f4-1f4197e6beba
---

Operator's general rule for `git push --no-verify` to dev: if you are the agent working on the CI/CD pipeline itself — your work already passed the hooks (gates run and documented), and the push is an **integration shakeout** — you effectively own the pipeline and may use CI as your verification surface, leaving local compute to the other agents. It is not a license for unverified code: the precondition is gates-already-green.

**Why:** the pre-push hook and CI cover the same ground; the integrating agent re-running heavy local gates duplicates compute the co-working agents need, and environmental hook breakage (pool-slot ENOENT, node_modules churn) shouldn't stall an integration whose substance is already verified.

**How to apply:** before bypassing — (1) every substantive gate for the changed projects has been run and is documented (commit messages are the receipt); (2) any REAL finding the hook surfaced is fixed first (the 2026-06-12 push: the hook's typecheck caught a missing structural-type member — fixed, THEN bypassed the environmentally-broken ESLint leg); (3) single-dispatcher verified, run-spawn confirmed after; (4) you then WATCH the wave fix-or-revert — bypassing the hook transfers the gate duty to you-on-CI. Relates to [[commit-only-integrator-pushes]] (push authority itself still comes from an operator grant).
