---
id: "backlog-jenkins-checkout-reliability"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Jenkins git-fetch intermittently broken (checkout-retry exhaustion, SIGTERM mid-clone) — add gitcache reference repo"
slug: "jenkins-checkout-reliability"
written: "2026-06-02"
author: "cartographer"
status: "proposed"
priority: "high"
area: "CI/infra"
recurrence: 3
source_shifts:
  - "2026-04-30"
  - "2026-05-06"
  - "2026-05-07"
domain: "operator"
relatedNodeIds:
  - "memory:feedback_jenkinsfile_safe_directory_required"
  - "memory:project_ci_storage_topology"
tags: [ci, infra, git, checkout, operator-domain, recurring]
shift_objective: |
  Jenkins git checkout is intermittently broken: builds hit "Maximum checkout retry attempts
  reached" and SIGTERM mid-clone (~78% of a fetch), recurring across 2026-04-30, 05-06, and
  05-07. Two correlated causes appear: the full clone is expensive enough to be killed
  (kubelet eviction on node-type:edge under pressure), and there is no shared reference repo
  to make the fetch cheap and resumable.
  Resolve the reliability gap: add a gitcache / reference repository (shallow + reference-repo
  clone) so each build's fetch is small and resumable rather than a full re-clone, and review
  the kubelet eviction / node-pinning that lets the checkout pod get SIGTERM'd mid-fetch
  (relate to the hostpath node-pinning correction in project_ci_storage_topology). This is
  operator-domain CI/infra. Done when checkout no longer exhausts retries under normal load
  and a build is not killed mid-clone by node pressure.
---

# Jenkins git-fetch reliability — gitcache reference repo + eviction review

## Why this matters

Operator-domain (gitcache topology + kubelet/eviction config are cluster changes). Three
shifts lost time to the same checkout failure; a reference repo is the standard fix and the
recurrence justifies prioritizing it.

## The failure shape

- "Maximum checkout retry attempts reached" — the fetch is retried and still fails.
- SIGTERM at ~78% of a clone — the checkout pod is killed mid-fetch, consistent with kubelet
  eviction on a pressured `node-type:edge` node.
- Each build does a full clone (no reference repo), so the fetch is large and not resumable.

## Shape of the fix (operator-owned)

1. Provision a gitcache / reference repository; configure checkout to use shallow +
   `--reference` so per-build fetches are small and resumable.
2. Review node pinning / eviction so the checkout pod isn't SIGTERM'd under pressure (ties to
   the hostpath node-pinning correction in `project_ci_storage_topology`).

## Acceptance

Checkout no longer exhausts retries under normal load; builds are not killed mid-clone by
node pressure.
