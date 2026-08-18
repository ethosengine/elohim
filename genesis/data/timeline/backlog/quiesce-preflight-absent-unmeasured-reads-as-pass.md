---
id: "backlog-quiesce-preflight-absent-unmeasured-reads-as-pass"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Quiesce-gate preflight: an ABSENT unmeasured series reads as a pass — the doc-level leg lacks the fail-closed rule the CI gate already has"
slug: "quiesce-preflight-absent-unmeasured-reads-as-pass"
written: "2026-08-18"
author: "integrator-session"
status: "backlog"
priority: "medium"
tags: [fleet-quiesce, probes, unmeasured-vs-zero, gospel-drift, dataplane, gate-semantics]
relatedNodeIds:
  - "backlog-fleet-quiesce-pass-not-convergence"
  - "backlog-resilience-unmeasured-vs-zero-honest-denominators"
  - "backlog-doorway-substrate-stats-unmeasured-not-zero"
cites:
  - scripts/ci/fleet-quiesce-gate.sh
  - CLAUDE.md
  - genesis/data/timeline/backlog/fleet-quiesce-pass-not-convergence.md
  - genesis/data/timeline/backlog/resilience-unmeasured-vs-zero-honest-denominators.md
---

# The leg that cannot fail

`CLAUDE.md` / `AGENTS.md` (§CI/CD, "Measuring without deploying") tell the next
reader to preflight the quiesce gate's four legs by hand before firing
`[build:edge] [edge:validate-only]` — among them `unmeasured=0` **via per-pod
Prometheus**. Measured 2026-08-18 against the live alpha Prometheus, that leg
cannot fail through the channel the gospel names, because **PromQL absence is
not zero** and the doc states no rule for absence.

## What was measured (alpha Prometheus, 2026-08-18)

| Query | Result |
|---|---|
| `count_over_time((count(elohim_projection_reconcile_converged_blocked_by{term="unmeasured"}))[10d:5m])` | **2385** of an expected 2880 — the series is **absent fleet-wide for ~495 samples (17%)** of the last 10 days |
| `count by (term)(elohim_projection_reconcile_converged_blocked_by)` at `now-37h` (inside one absence window, ≈2026-08-17 06:22 UTC) | **empty** — and `count(up{job="elohim-alpha/elohim-edgenode"} == 1)` is **empty at the same instant** |
| `count_over_time((count(...{term="unmeasured",pod="elohim-matthew-alpha-0"} == 0))[10d:5m])` | **2301** / 2880 — present-and-zero 80% of the window |
| `max_over_time(...{term="unmeasured",pod="elohim-matthew-alpha-0"}[30d])` | **0** — the term has **never lit on storage-A inside retention** |
| `max_over_time(...{term="unmeasured"}[30d]) > 0` | returns rows for **eve / susan / gertrude** instances — the gauge is alive; only A has no observed positive |

Two facts, kept separate:

1. **The absence is real and recurrent** — roughly one sample in six over the
   last 10 days, in multi-hour blocks. It is *not* a term-specific vanish: the
   whole `elohim-alpha/elohim-edgenode` scrape target set is missing from `up`
   in those windows (pod/instance churn — the `instance` label rotates through
   dozens of pod IPs per human in 30d). So the honest name for it is
   **"we had no measurement"**, which is exactly the state the `unmeasured`
   term exists to make visible, arriving as the *absence of that term*.
2. **The A-side leg has never discriminated.** On `elohim-matthew-alpha-0` —
   the only pod the gate's predicate reads — the term is 0 across the full
   30-day retention. Record that; do not "fix" it. It means the leg's live
   evidential value is untested on the pod that matters, while the same gauge
   demonstrably lights on shem peers.

## Why it is unfalsifiable in the preflight channel

A reader following the gospel runs a per-pod query and reads "no rows" as "no
blocker" — the same failure mode this project has already named twice
(`resilience-unmeasured-vs-zero-honest-denominators`,
`doorway-substrate-stats-unmeasured-not-zero`): **unmeasured is being rendered
as zero, one layer up, in the human procedure instead of in a UI.** With a ~17%
absence rate, a preflight run at an arbitrary moment has about a one-in-six
chance of passing this leg *because nothing was measured at all*.

## What is NOT broken

`scripts/ci/fleet-quiesce-gate.sh` is correct and needs no change. It parses
storage-A's `/metrics` text directly (never Prometheus), and
`labeled_metric_value()` returns `None` on an absent series, which makes
`quiesced_ok` False — its own header says it: *"If the blocked_by series is
ABSENT (pre-honesty-floor image), the gate fails closed and keeps waiting —
absence of evidence is not quiescence."* The drift is **doc-level only**: the
human/agent preflight was written as a value comparison and inherited none of
the gate's absence discipline.

## Remediation (small, two parts)

1. **Gospel (managed surface — goes through the cite tooling, not a hand edit):**
   the preflight line gains an absence rule, e.g. *"an empty result is a FAIL,
   not a pass — require `count(...{term="unmeasured"}) == 7` and the value 0;
   `absent(...)` returning a row means you have no measurement, so you have not
   preflighted."* Same sentence covers the `divergent_actionable<=2` leg, which
   has the identical hole.
2. **Optional, and the better fix if it stays cheap:** a preflight script beside
   `fleet-quiesce-gate.sh` that reuses the gate's own fail-closed parser against
   the pods, so the human path and the CI path cannot diverge again. The
   substrate trust contract's rule applies — the probes are the authority, so
   the durable cure is a probe, not a prose warning.

## Reproduction

All five queries above are instant/range PromQL against the `prometheus`
datasource; nothing needs cluster access. Re-run them before acting — the
absence rate is a live number and the A-side "never lit" claim is bounded by
30-day retention, not by history.
