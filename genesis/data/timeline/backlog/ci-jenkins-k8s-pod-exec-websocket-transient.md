---
id: "backlog-ci-jenkins-k8s-pod-exec-websocket-transient"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Jenkins kubernetes-plugin pod-exec websocket 404 transient kills a stage mid-`sh` (Build App, elohim #1530) — infra flake, retrigger-recovers, NOT a code/substrate fault"
slug: "ci-jenkins-k8s-pod-exec-websocket-transient"
written: "2026-06-11"
author: "ci-failure-triage"
status: "backlog"
priority: "low"
ci_status: blocked
fingerprints: [9131306b0242]
jobs: [elohim]
relatedNodeIds: []
tags: [ci, infra, jenkins-kubernetes-plugin, pod-exec, websocket-404, agent-pod-channel, flake, retrigger-recovers, host-green-not-ci-green, operator-owned, museum-candidate]
cites:
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1530/
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1531/
  - Jenkinsfile
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
---

# Jenkins K8s pod-exec websocket transient — a stage dies mid-`sh`, not the build

## The failure

```
fp 9131306b0242  elohim — red build, stage:Build App  (elohim/dev #1530, FAILURE, seen 1, build 1530..1530)
```

Occurrence evidence: seen once, first/last build 1530. The retrigger build
#1531 completed UNSTABLE (not FAILURE) — the Build-App-stage death did NOT
recur (and ran ~2.1x longer: 930s vs 435s, consistent with a full run vs an
early stage abort).

The signature, in build #1530's order:

```
L1644  [Pipeline] { (Build App)
L1702  + pnpm exec ng build --configuration=alpha        # Angular build PROCEEDS
        ...                                                # (build output, ESM warnings — normal)
L2075  io.fabric8.kubernetes.client.http.WebSocketHandshakeException
L2173  Caused by: java.net.ProtocolException: Expected HTTP 101 response but was '404 Not Found'
L2359  at ...ExecWebSocketListener.onError
        at ...PodOperationsImpl.lambda$setupConnectionToPod$2
L2374  Retrying in 2s ...
L2675  Retrying in 4s ...                                  # exponential backoff, multiple attempts
```

The decisive frames (build #1530, ~L2790):

```
PodOperationsImpl.setupConnectionToPod(PodOperationsImpl.java:387)
PodOperationsImpl.exec(PodOperationsImpl.java:293)
ContainerExecDecorator$1.doLaunch(ContainerExecDecorator.java:520)   # kubernetes-plugin
Launcher$ProcStarter.start(...)
BourneShellScript.launchWithCookie(...)                              # durable-task `sh` step
DurableTaskStep$Execution.start(...)
```

## Verdict — FLAKE (infra / operator-owned)

This is the Jenkins **kubernetes-plugin**'s `ContainerExecDecorator` opening a
`kubectl exec`-equivalent **websocket** into the *build-agent pod's* container
to launch a `sh` step's process channel. The apiserver returned `404 Not
Found` on the pod `exec` subresource handshake (the SPDY/websocket upgrade to
101 never happened). That is an **agent-pod channel** transient — apiserver
proxy / kubelet-not-ready / pod-evicted-or-churned during the stage — fully on
the Jenkins-controller↔k8s-infra surface. It is NOT:

- a code/build fault — `ng build --configuration=alpha` had already proceeded
  past invocation; the green Angular compile is upstream of the exec failure.
- the **alpha-cluster-degraded substrate** concern
  (`ci-alpha-cluster-degraded-substrate.md`) — that is the *deployed* app's
  peers crashlooping / shem-down → deploy/upload/E2E-health-gate failures in
  LATER stages. This died in **Build App**, before any deploy/E2E stage, on
  the *build agent's own* pod channel — a different surface.
- any museum trap (NOT_BUILT-lossy-measure, `#[ignore]`-no-op, webhook
  double-fire, baseline-rollback over-build, sccache poisoning). Checked — no
  match.

It is a clean instance of the museum's **host-green ≠ CI-green** family,
inverted: here the *build* is green and the *CI pod transport* is what failed.

## Root cause

apiserver/kubelet returned 404 on the build-agent pod's `exec` websocket
subresource at `sh`-launch time — pod churn (eviction/restart/not-yet-ready)
or apiserver exec-proxy transient. The kubernetes-plugin retried with backoff
(2s, 4s, …) and ultimately failed the step, which failed the stage, which
failed the build. Single occurrence; non-deterministic; cleared on retrigger.

## Current decision

`ci_status: blocked` — there is no in-tree code fix; the unblock is
**operator-owned** and twofold:

1. **Immediate (already done):** empty-commit retrigger `[build:app,edge]`
   (push `a44a89b6a`) → #1531 UNSTABLE, transient gone. This is the correct
   response to a single pod-exec flake.
2. **If it recurs (≥2–3×):** operator investigates Jenkins agent-pod
   stability / apiserver exec-proxy health (kubelet pressure, pod eviction
   churn on the build node, controller↔apiserver network). The repo-side
   mitigation, if recurrence justifies it, is a `retry { … }` wrapper around
   the Build-App `sh` invocation in `Jenkinsfile` so a single exec-channel
   404 self-heals without a human retrigger — but a one-off does NOT justify
   editing the size-pressured root Jenkinsfile (near the 64KB CPS limit).

Held at `low` priority precisely because it is a single, retrigger-cleared
infra transient. The ledger entry closes by **disappearance** (the harvester
confirms via job green-streak ≥3 with no recurrence) — not asserted here.

## Fix trail

- No code change. Remedy = empty-commit retrigger (push `a44a89b6a`,
  `[build:app,edge]`); #1531 completed UNSTABLE, the Build-App pod-exec
  transient did not recur.
- Ledger `9131306b0242`: status → `triaged` (triage complete: classified,
  verdict flake/infra, recovery confirmed in #1531). NOT stamped
  `decompose_on_confirm` — this transient class is **museum-candidate** (see
  below); on confirmed disappearance the harvester reports it for
  graduate-then-decompose rather than silently dropping the lesson.

## Museum-graduation note

If this Jenkins-K8s-plugin pod-exec websocket-404 transient recurs and earns a
frequency rank, it graduates as a NEW recurring trap **into** the existing
museum record
(`genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md`)
— extend that doc, never fork a second lessons file. The lesson shape:
*"Expected HTTP 101 but was 404 + ExecWebSocketListener/ContainerExecDecorator
in the trace = the Jenkins build-agent pod's exec channel died, NOT your code
and NOT the deployed substrate; retrigger first, instrument agent-pod
stability if it repeats."*
