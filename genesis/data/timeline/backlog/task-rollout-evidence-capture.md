---
id: "backlog-task-rollout-evidence-capture"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Task: capture pod conditions/events/logs on failed rollouts — cure the replacement-pod evidence blindness that made edge #1410 undiagnosable"
slug: "task-rollout-evidence-capture"
written: "2026-09-01"
author: "session-2026-09-01-rung5-design (from the #1410 diagnosis)"
status: "in-progress"
priority: "high"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
claimedBy: "codex"
relatedNodeIds:
  - "backlog-task-iroh-user-agent-observed-receipt"
tags: [ci, rollout, evidence, observability, delegable]
---

**Claimable by any implementation agent. Born from the 2026-09-01 edge #1410
diagnosis (recorded in `task-iroh-user-agent-observed-receipt.md:142`): the
"0/2 doorways Ready" red was a counting artifact (failed rollout testcases
read as pod state), and the REAL question — why replacement pods since #1405
don't come Ready — was undiagnosable because Jenkins retained no pod
conditions, events, states, or logs.**

## Why

Rollout failures that leave no evidence force speculative fixes (the
anti-pattern the museum documents). The conductor split (rung 2) makes pod
scheduling/termination pressure a live hypothesis for #1405+ — but only
captured evidence can confirm or kill it. Rung 5's fleet receipts all queue
behind trustworthy rollout verdicts.

## Scope

1. In the edge deploy path (`elohim/holochain/Jenkinsfile` →
   `deployEdgeWithManifest` / the rollout-status leg, bash bodies in
   `scripts/ci/` per the heredoc rule): on a rollout timeout/failure, capture
   BEFORE tearing down or moving on — `kubectl get pods -o wide` for the
   affected selector, `kubectl describe` (conditions + events) for
   non-Ready pods, last ~200 log lines per non-Ready container, and node
   pressure summary — into an archived artifact with a stable name.
2. Fix the summary so pod readiness is counted from pod state, never from
   rollout testcase counts — the #1410 artifact class.
3. Respect the cluster-ops boundary: reads only (`kubectl get/describe/logs`
   from the BUILD container is the CI's existing read path); never mutate.

## DoD

- A deliberately-failed rollout (or the next natural one) archives the
  evidence bundle; the build page links it.
- The doorway-readiness summary line on an UNSTABLE build names actual pod
  states.
- MUST NOT change deploy ordering, budgets, or manifests.

## Implementation evidence (2026-09-01)

The repo-side capture path is wired. Every edge rollout wait (human storage,
split conductor, and doorway Deployment; plus the legacy generic helper) now
routes through one wrapper that preserves the original rollout exit, invokes a
read-only bash/coreutils collector synchronously on failure, and only then
propagates the failure. The collector resolves the workload's real
`.spec.selector.matchLabels` instead of the invalid historical
`app.kubernetes.io/name=<resource>` guess, so it captures both old and
replacement pods. Its stable
`rollout-evidence/<namespace>--<kind>--<name>/` bundle contains the workload,
wide/YAML pod state, a Ready-condition summary that excludes terminating pods,
deep describe/events for every non-Ready or terminating pod, current and
previous 200-line tails for all init and regular containers, and a best-effort
node-pressure table. Failed diagnostic reads (including node RBAC denial) are
recorded with their exit code and never replace the rollout verdict.

The Jenkins summary now says `rollout testcases passed`; actual pod readiness
comes only from pod `Ready` conditions plus deletion state. A hermetic mocked
failure proved the #1410 shape explicitly: one healthy pod, one non-Ready
replacement, and one Ready-but-terminating old pod produced `1/3 pods Ready`,
captured only the two attention pods, tailed every container, retained a
simulated node-RBAC denial, and exited zero. The Jenkinsfile method-size gate
remained below its hard ceiling, and `post.always` archives
`rollout-evidence/**` so a failed/UNSTABLE build exposes the bundle on its build
page.

This atom remains `in-progress` until the next natural or deliberately failed
Jenkins rollout supplies the final live build-page archive receipt required by
the DoD. No deploy ordering, timeout, restart budget, or manifest changed.

Story-graph interstitial: this atom already is the missing station between
`rollout testcase failed` and `fleet receipt trusted` — **the named workload's
actual pod conditions, termination state, events, and bounded logs are archived
before control leaves the failed rollout**. No second atom is needed.
