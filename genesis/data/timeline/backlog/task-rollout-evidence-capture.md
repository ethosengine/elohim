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

## Live receipt — supplied by elohim-edge/dev #1413, NOT #1414 (ci-failure-triage, 2026-09-02)

The first natural failed rollouts arrived on 2026-09-02. **#1413 satisfies the
DoD's build-page archive receipt**: its artifact list carries five complete
bundles — `rollout-evidence/elohim-alpha--statefulset--elohim-{jessica,gertrude,susan,eve}-alpha-conductor/`
and `rollout-evidence/elohim-alpha--deployment--elohim-doorway-alpha-b/` — each
with `capture.meta`, `summary.txt`, `pods.yaml`, `pods-wide.txt`,
`pod-state.tsv`, `selector.txt`, `node-pressure.txt`, per-pod `describe.txt` /
`events.txt`, and current+previous tails for every init and regular container
(`happ-fetcher`, `elohim-conductor`, `ws-proxy`). Scope item 2 is verified live
too — the summary names actual container states, not rollout testcase counts:
`… 0/1 pods Ready — elohim-jessica-alpha-conductor-0=Running/NotReady[node=ethosengine;containers=happ-fetcher=true/Completed;elohim-conductor=false/CrashLoopBackOff;ws-proxy=true/;]`.
#1413 ended **ABORTED** (superseded) and still archived, which is the stronger
receipt.

**#1414 is not a counter-example** (correcting an earlier revision of this note,
same day, before it was acted on). #1414 logged the collector's readiness
summary and the `expected artifact:` line but its build page carried only
`build.env` — that is **not** an archive defect: `archiveArtifacts` lives in the
pipeline-level `post { always { … } }` (`elohim/holochain/Jenkinsfile`), and
#1414 was still `IN_PROGRESS` inside "Deploy Edge Node - Alpha", so `post` had
not run. A `/artifact/rollout-evidence/…` URL 404s until the build finishes.
The `expected artifact:` phrasing in `waitForRolloutWithEvidence` is a
forward-pointer for a human reading the console, not a claim of publication —
worth knowing, because it reads like a receipt when it is a promise.

Still true and worth the DoD's attention: the collector **always** exits 0 by
design (`scripts/ci/capture-rollout-evidence.sh:10-12`), and the archive uses
`allowEmptyArchive: true`. So a genuine future miss — collector wrote nothing,
or wrote outside the glob's base — would be silent at both ends. Neither this
build nor #1413 exhibits one; the pairing is just structurally unobservable, and
worth a thought before this atom closes.

Both builds' context:
`genesis/data/timeline/backlog/ci-edge-conductor-roll-no-halt-walks-the-fleet.md`.

Story-graph interstitial: this atom already is the missing station between
`rollout testcase failed` and `fleet receipt trusted` — **the named workload's
actual pod conditions, termination state, events, and bounded logs are archived
before control leaves the failed rollout**. No second atom is needed.
