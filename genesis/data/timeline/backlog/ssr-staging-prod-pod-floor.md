---
id: "backlog-ssr-staging-prod-pod-floor"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "SSR staging.yaml + prod.yaml carry 256Mi (will OOM on SSR roll) — apply the memory bump + startupProbe"
slug: "ssr-staging-prod-pod-floor"
written: "2026-06-02"
author: "cartographer"
status: "proposed"
priority: "high"
area: "SSR/doorway"
recurrence: 1
source_shifts:
  - "2026-05-07"
domain: "code"
relatedNodeIds:
  - "memory:project_storage_as_pod_operator_sets_virtual_limits"
  - "memory:project_ci_storage_topology"
tags: [ssr, doorway, k8s, memory, oom, startupprobe, code-domain]
shift_objective: |
  The doorway SSR runtime embeds V8 (deno_core), whose cold-start wall-time and resident
  memory are far higher than a CSR-only doorway — but the SSR staging.yaml and prod.yaml
  manifests still carry the old 256Mi memory floor and no startupProbe. The first SSR roll on
  those environments will OOM on V8 cold-start (observed 2026-05-07; the SSR-runtime design
  flags V8 cold-start at 2s→15s→60s with a required pod-memory bump + startupProbe).
  Resolve it by applying the memory bump + startupProbe to the SSR staging.yaml and prod.yaml
  manifests so V8 cold-start fits the pod budget and the probe waits out the cold-start instead
  of crash-looping. This is code-domain (committed k8s manifests in the repo; the operator's
  pipeline reconciles them — per project_storage_as_pod_operator_sets_virtual_limits, the
  manifest declares the limits, the operator's reconcile applies them). Done when SSR
  staging.yaml and prod.yaml carry the bumped memory floor + a startupProbe sized for V8
  cold-start.
---

# SSR staging/prod manifests need the memory bump + startupProbe

## Why this matters

Code-domain (the k8s manifests live in the repo and the operator's pipeline reconciles them).
This is a known-OOM waiting to happen — the SSR-runtime design already documented the V8
cold-start memory/wall-time, but the staging/prod manifests were never updated off the
CSR-era 256Mi floor. The first SSR roll on those environments crash-loops.

## The failure shape

- Doorway SSR embeds V8 (deno_core); cold-start is memory- and time-heavy (2s→15s→60s).
- staging.yaml / prod.yaml still carry 256Mi and no startupProbe.
- First SSR roll OOMs on cold-start and/or the readiness probe kills the pod before V8 warms.

## Shape of the fix (code-domain)

Apply the memory bump + a startupProbe sized for V8 cold-start to the SSR `staging.yaml` and
`prod.yaml` manifests. The manifest declares the limits; the operator's reconcile applies them
(`project_storage_as_pod_operator_sets_virtual_limits`).

## Acceptance

SSR `staging.yaml` and `prod.yaml` carry the bumped memory floor + a startupProbe sized for V8
cold-start.
