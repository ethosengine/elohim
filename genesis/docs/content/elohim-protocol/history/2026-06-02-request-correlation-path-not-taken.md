---
title: "History/ADR: Cross-runtime request correlation — the path not taken"
type: history-gotcha
status: Accepted
tier: history
created: 2026-06-02
topic: [observability, request-tracing, a2o, che-feedback, path-not-taken]
# DISTILLS an ABANDONED design (never committed — elohim/sdk/schemas/v1/correlation/
# has zero git history). The need it targeted was met more cheaply by the sprint-report
# aggregator + the Che headless capture loop. Raw body retires to git.
distills:
  - .claude/archive/2026-05-15/genesis/docs/superpowers/plans/2026-04-19-runtime-request-correlation.md
# Bidirectional: the memory entry documenting the mechanism that REPLACED this.
canonical:
  - ../../../../../.claude/memory/project_che_browser_feedback_loop.md
memory_anchors:
  - project_che_browser_feedback_loop
  - feedback_haiku_observe_only_no_specifics
  - feedback_cascade_hidden_test_surface
---

# Cross-runtime request correlation — the path not taken (2026-04-19)

> **Hot-context pointer (the one sentence to remember):**
> When a cheap *observational* loop covers the need, do not build the distributed-tracing rig.
> End-to-end header-tracing + per-peer ring buffers across two Rust runtimes was proposed and
> **never built**; "look at the rendered surface and read the deduped failure feed" answered the
> same need.

A six-phase plan proposed making every HTTP hop correlatable end-to-end via three propagated headers —
`X-Request-ID` (stable per-request UUID), `X-Target-Peer` (intended peer, drives federation routing),
`X-Served-By` (peer that actually handled it) — plus a bounded in-process ring-buffer log layer and an
`/admin/logs?request_id=X` endpoint, so an a2o failure could fetch backend logs per peer. It was framed
as "the prerequisite for closing the dev-to-acceptance loop against a federated mesh."

## It was never implemented

`elohim/sdk/schemas/v1/correlation/` has zero commits; no `X-Served-By` exists in doorway or storage.
The need it targeted — an agent being able to SEE why a federated surface failed — was instead met by
two cheaper, independently-valuable mechanisms that *did* land:

- the **sprint-report aggregator** (`genesis/a2o/scripts/build-sprint-report.ts`), which
  dedupes/fingerprints Cucumber + console-error + coverage-gap artifacts into a ranked feed for
  `/shift` Objectives; and
- the **Che browser feedback loop** (`pnpm look <url>`, landed 2026-05-30), which renders any surface
  headless and lets the agent multimodally read the screenshot.

## Why we turned

End-to-end header-tracing + ring buffers across two Rust runtimes is a lot of standing infrastructure
to maintain for what turned out to be answerable by observation. When the cheap observational loop
covers the need, don't build the distributed-tracing rig. (If a genuine multi-hop federation-routing
debug need returns, this plan is the starting point — recoverable in git.)

## Watch-out for future planners

Before standing up cross-runtime tracing, ask whether the actual need is "debug a federated routing
failure" or "see why a surface looks wrong." The latter is satisfied by `pnpm look` + the sprint-report
feed. Only the former justifies the header/ring-buffer rig — and even then, scope it to the routing hop
that's failing, not every HTTP hop.

## Bidirectional links

- **This record → canonical:** the [`project_che_browser_feedback_loop`](../../../../../.claude/memory/project_che_browser_feedback_loop.md) memory entry (the observational loop that replaced this).
- **Distilled-from (raw body in git history):** the runtime-request-correlation plan (linked in frontmatter; never committed at its schema path).
