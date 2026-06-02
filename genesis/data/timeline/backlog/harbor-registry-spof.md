---
id: "backlog-harbor-registry-spof"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Harbor registry is a single point of failure for all CI — add multi-replica or cached-image fail-over"
slug: "harbor-registry-spof"
written: "2026-06-02"
author: "cartographer"
status: "proposed"
priority: "high"
area: "CI/infra"
recurrence: 2
source_shifts:
  - "2026-04-28"
  - "2026-05-30"
domain: "operator"
relatedNodeIds:
  - "memory:feedback_check_helm_chart_status_before_runbooks"
  - "memory:project_ci_storage_topology"
tags: [ci, infra, harbor, registry, spof, operator-domain, recurring]
shift_objective: |
  The Harbor registry is a single point of failure for the entire CI substrate: when Harbor
  is unhealthy (ImagePullBackOff, storage EIO) every pipeline that pulls a builder or runtime
  image halts, and there is no self-heal — builds simply wedge until the operator intervenes.
  This bit twice (2026-04-28 ImagePullBackOff cascade; 2026-05-30 handoff recorded the SSR
  alpha deploy BLOCKED on Harbor registry storage EIO), and the second time it blocked a
  delivery, not just a build.
  Resolve the SPOF: multi-replica Harbor (HA registry) and/or a cached-image fail-over so a
  pull can fall back to a mirror / local cache when the primary is down, plus a health signal
  that surfaces Harbor degradation BEFORE a pipeline wedges on it. This is operator-domain
  cluster/infra work — surface the failure mode and the design options; the operator owns the
  topology change. Done when a single Harbor instance going unhealthy no longer halts all CI,
  and the degradation is observable rather than discovered via ImagePullBackOff.
---

# Harbor registry single-point-of-failure — multi-replica or cached-image fail-over

## Why this matters

Operator-domain (registry topology is a cluster change the agent cannot make). The
recurrence — and the fact that the 2026-05-30 instance blocked an actual delivery (SSR alpha
deploy on storage EIO, `cf53a76c2`) rather than just a build — is what promotes this from a
one-off incident to a standing risk worth a backlog slot.

## The failure shape

- Harbor goes unhealthy: backing storage EIO, or the registry pod itself unschedulable.
- Every pipeline that pulls an image from Harbor (builders, runtime base images) enters
  ImagePullBackOff and wedges. No retry path recovers it; no replica absorbs the load.
- There is no early signal — the first symptom is a stalled pipeline, not a degraded-health
  alert, so the operator finds out by triaging a wedged build.

## Shape of the fix (operator-owned topology)

1. HA Harbor (multi-replica registry) and/or a pull-through mirror / local image cache so a
   pull can fail over when the primary is degraded.
2. A health signal that surfaces Harbor degradation (storage EIO, pod NotReady) ahead of the
   pipeline wedge, so the operator acts before CI halts.
3. Check upstream chart status before reaching for the Bitnami Harbor chart
   (`feedback_check_helm_chart_status_before_runbooks`).

## Acceptance

A single Harbor instance going unhealthy no longer halts all CI; degradation is observable
ahead of an ImagePullBackOff cascade.
