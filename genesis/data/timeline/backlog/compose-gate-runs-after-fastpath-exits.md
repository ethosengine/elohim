---
id: "backlog-compose-gate-runs-after-fastpath-exits"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "pre-push .epr-meta compose-gate sits AFTER the early fast-path exits (ci-ignore skip, elohim-agent manifest) — a narrow push skips the governance backstop"
slug: "compose-gate-runs-after-fastpath-exits"
written: "2026-07-06"
author: "eprfs/elohim-agent integration (husky reshape follow-up)"
status: "backlog"
priority: "low"
jobs: [elohim]
---

## What

In `.husky/pre-push.bash` the `.epr-meta compose-gate` stage (the harness-agnostic
governance backstop) is positioned mid-file, AFTER two early `exit 0` fast-paths:

1. the `.ci-ignore` skip ("all changes are docs/agents/CI-only — skipping gates"), and
2. the `elohim-agent` manifest fast-path (added on `feat/eprfs`) which runs only
   `pnpm run elohim-agent:test` then `exit 0` for manifest/schema-only pushes.

Either early exit fires before the compose-gate is reached, so a push whose entire
changeset falls into one of those narrow buckets bypasses the compose-gate backstop.

## Why it matters

The pre-commit gate is the PRIMARY (it runs at commit time on the staged set); the
pre-push compose-gate is the BACKSTOP for commits made with `--no-verify` at commit
time. A `--no-verify` docs-only or manifest-only commit therefore reaches neither the
primary (bypassed) nor the backstop (skipped by the early exit). Low severity today —
the `.ci-ignore` bucket is docs/agents (the compose-gate's frontmatter rules mostly
concern those, but the pre-commit gate is the intended catch), and the elohim-agent
range is currently compose-gate-clean (verified: gate exit 0 over `a51f2738a..4f14a7c6a`).

## Proposed work

The compose-gate is pure-python, ~ms, PVC-exempt by omission — it is cheap enough to
run UNCONDITIONALLY. Move it to run BEFORE all fast-path early-exits (right after the
`CHANGED` set is computed), so governance is truly universal and the fast-paths skip
only the HEAVY project gates, never the cheap universal governance check. This is the
coherent completion of "governance fires for EVERY push regardless of harness."

## Provenance

Surfaced during the `feat/eprfs` + `elohim-agent` manifest integration onto dev
(husky bash-shim split reshape). Captured rather than folded in to keep that
integration scoped to behavior-preservation vs dev. See the reshape commit
`fix(husky): carry the .epr-meta compose-gate into pre-push.bash`.
