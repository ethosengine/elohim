---
id: "backlog-a2o-loki-step-primitive"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "resiliency-saga chapters only assert against HTTP/metrics endpoints — a Loki log-line step primitive would let later chapters prove log-only invariants"
slug: "a2o-loki-step-primitive"
written: "2026-07-25"
author: "claude (saga-status.py implementation session)"
status: "envisioned"
priority: "low"
relatedNodeIds:
  - "genesis/a2o/features/dataplane/resiliency-saga/README.md"
  - "genesis/a2o/steps/dataplane.steps.ts"
  - "genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md"
tags: [a2o, observability, loki, resiliency-saga, cucumber-steps]
---

Read while grounding `saga-status.py` in the resiliency-saga's own chapter table (T6): every one
of the ten chapters' proof signals is an HTTP endpoint or a Prometheus metric —
`dataplane.steps.ts`, `resilience.steps.ts`, and the saga-specific
`dataplane/resiliency-saga.steps.ts` cover `/health`, `/metrics`, `/db/content`,
`/api/v1/resilience/*`, commitment polls, and served-head compares. Nothing in the a2o step
library today can assert against a Loki LOG LINE. The substrate-trust-contract-runbook names
invariants that are naturally log-shaped rather than endpoint-shaped — "heal fills-never-moves",
"restart churn ≈20min", "fresh actions need publish time" — the kind of claim that's easiest to
verify by finding the actual Rust log line that reports a heal decision or a restart event, not by
polling a snapshot endpoint that may not expose the distinction at all.

This is a forward-looking idea, not a gap blocking anything today (chapters 1-10 all have working
non-Loki proof signals, several intentionally born-red per the README's "RED-FIRST is correct"
section). Candidate shape: a `Then Loki shows a log line matching "<pattern>" within <n>s` step in
a new `steps/observability.steps.ts`, using the `mcp__observability__query_loki_logs`/
`find_error_pattern_logs` surface already available to agents in this environment as the reference
implementation to port into the CI-runnable step (CI's cucumber runner would need its own Loki
client, not the MCP tool). Would let a future saga chapter (or an eleventh, "the mesh heals without
moving data") assert against log evidence instead of only structural HTTP responses.
