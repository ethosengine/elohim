---
id: "backlog-iroh-direct-connection-ratio-meter"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Meter the Stage-2 iroh direct-connection ratio per conductor peer"
slug: "iroh-direct-connection-ratio-meter"
written: "2026-08-06"
author: "codex"
status: "wip"
priority: "high"
relatedNodeIds: []
tags: [iroh, wave-2, stage-2, loki, grafana, observability, qad]
shift_objective: |
  Provision the sole Phase-B QAD decision input before the alpha transport soak:
  direct connection percentage and observation volume per conductor pod.
---

# Iroh direct-connection ratio meter

Claimed by Codex on 2026-08-06 from relay-sovereignty U4/U5 and §5.3.

## Claim fence

- `genesis/orchestrator/manifests/infra/alpha-iroh-connection-ratio-grafana-dashboard.yaml`
- alpha's explicit `infraManifests` entry in `elohim/holochain/Jenkinsfile`
- this backlog claim

No conductor, storage, doorway, alerting, or QAD implementation code is in scope.

## Prepared result and remaining gate

The sidecar-provisioned Grafana dashboard uses Loki to compute the event-weighted
`direct=true` percentage per local conductor pod over the selected window. A
second panel exposes the exact denominator so a tiny sample is not mistaken for
a soak. The query tolerates ANSI escapes in the piped conductor log and treats
no series as unmeasured rather than zero. It is a report, not an alert: U5 proves
there is no info-level QAD-failure signal, and the Phase-B verdict is an operator
decision.

The manifest is wired into alpha's explicit infra apply list. Before using it for
the Stage-2 verdict, validate the LogQL against the first live post-flip Loki
event and set the dashboard window to the exact soak interval. That live proof is
why the claim remains `wip`.
