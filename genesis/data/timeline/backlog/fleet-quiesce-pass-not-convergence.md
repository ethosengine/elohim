---
id: "backlog-fleet-quiesce-pass-not-convergence"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "fleet-quiesce gate PASS is not convergence — A-quiesced only; B-caughtUp=False at PASS while validation 503s minutes later"
slug: "fleet-quiesce-pass-not-convergence"
written: "2026-08-09"
author: "integrator-session"
status: "backlog"
priority: "high"
tags: [edge-pipeline, fleet-quiesce, probes, dataplane-validation, gate-semantics]

relatedNodeIds:
  - "backlog-edge-deploy-ready-gate-liveness-only"
---

# Two probes in one build disagreed about the same fleet

Edge #1327 (2026-08-09): the fleet-quiesce gate ran a genuine measurement
(35 poll lines, 60s cadence) and declared FLEET QUIESCENT at 1999s — while
**every poll including the terminal PASS carried B-caughtUp=False**. The
gate's pass condition tracks A-quiesced (actionable-count convergence on
probe A, content=elohim-host-landing) only. Minutes later, the SAME
build's Dataplane Validation found alpha-A serving HTTP 503
{"status":"catching-up"} on /db/content and p2p.caughtUp=false — 0/3
notary-authority scenarios, saga-06 heads-converge 3/4 failed on it.

Consequences:
1. "F2 PASS" timing numbers (33m19s this run) measure a NARROWER signal
   than fleet convergence — any decision criterion (e.g. the conductor
   call-deadline spike memo §6.4 fork-deploy go/no-go) that reads the
   quiesce gate as a convergence green must not.
2. Sibling of the Ready≠mesh gap (edge-deploy-ready-gate-liveness-only):
   both are gate-semantics findings where a green surface certifies less
   than its name implies. Per the substrate trust contract, the probes are
   the authority — so the fix is probe semantics (gate B-caughtUp, or
   rename/annotate the gate's claim), not prose.

Also recorded from the same measurement pass: PTxnGuard emits NO
Prometheus metric under any name (81-metric elohim_* catalog checked) —
flatness claims about it are unverifiable until instrumented; and
elohim_head_batch_queue_wait_ms splits by node class (shem peers flat
~0.5/0.9ms p50/p90; household matthew/james/jessica elevated, matthew p90
peaking 900ms during validation traffic).
