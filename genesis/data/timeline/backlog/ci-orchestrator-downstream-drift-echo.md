---
id: "backlog-ci-orchestrator-downstream-drift-echo"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Orchestrator Reconcile-Build-Graph UNSTABLE — downstream-drift echo of genesis/elohim/edge UNSTABLE+FAILURE (museum trap #1 working as designed)"
slug: "ci-orchestrator-downstream-drift-echo"
written: "2026-06-06"
author: "ci-failure-triage"
status: "backlog"
priority: "low"
ci_status: blocked
fingerprints: [4508f1172d15, b169c3b9034c, ddd8ed2cbdc7, a90e18c0cf94, c9624ee1d1fe]
jobs: [elohim-orchestrator]
relatedNodeIds: []
tags: [ci, elohim-orchestrator, reconcile-build-graph, post-flight-health-check, post-actual-build-graph, level-n-child-failure, downstream-echo, museum-trap-1, not-a-root-cause, coarse-fingerprint]
cites:
  - https://jenkins.ethosengine.com/job/elohim-orchestrator/job/dev/1167/
  - https://jenkins.ethosengine.com/job/elohim-orchestrator/job/dev/1168/
  - https://jenkins.ethosengine.com/job/elohim-orchestrator/job/dev/1192/
  - https://jenkins.ethosengine.com/job/elohim-orchestrator/job/dev/1199/
  - https://jenkins.ethosengine.com/job/elohim-orchestrator/job/dev/1205/
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1525/
  - genesis/orchestrator/README.md
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
---

# Orchestrator UNSTABLE is a downstream echo, not an independent concern

## The failure

```
4508f1172d15  elohim-orchestrator — red build, stage:Reconcile Build Graph   (1164–1167)
b169c3b9034c  elohim-orchestrator — red build, stage:elohim-genesis          (1168)
ddd8ed2cbdc7  elohim-orchestrator — red build, stage:Post-flight Health Check (1188–1192, seen 5)
```

All builds are **UNSTABLE** (the harvester's "red build" label is its
classifier token; the actual result is UNSTABLE — confirmed via the build API).

- #1167, **Reconcile Build Graph**: `📊 BUILD GRAPH RECONCILIATION — verdict:
  DRIFT / drift: 3 unstable, 1 unknown-result`, with
  `elohim=UNSTABLE, elohim-storybook=DISPATCHED, elohim-edge=UNSTABLE,
  elohim-genesis=UNSTABLE`. Stage warns `Build graph drift detected — see
  build-graph-reconciliation.json`.
- #1168, **elohim-genesis** stage: `Build elohim-genesis » dev #1101 completed:
  FAILURE`; `Results: elohim=UNSTABLE, …, elohim-genesis=FAILURE`;
  `Investigation pointers: • elohim-genesis FAILURE — …/elohim-genesis/job/dev/
  1101/`. Orchestrator's own status UNSTABLE: "Genesis failed - seeding or
  tests may have issues."
- #1192, **Post-flight Health Check** (fp `ddd8ed2cbdc7`): the stage is
  `catchError`-wrapped (orchestrator log #1192 line 225) and emits `unstable()`
  (line 260) with `WARNING: 1 service(s) unhealthy after deployment`. The one
  unhealthy service is `elohim-edge: HTTP 000` — `curl https://alpha-edge.elohim.host/health`
  returns `000` (unreachable). `doorway-staging.elohim.host/health` returns a
  `503 Service Temporarily Unavailable`. **Crucially `doorway-alpha.elohim.host/health`
  returns HTTP 200 with `conductor.connected:true, connected_workers:4, peerCount:2`**
  (line 286) — the alpha doorway itself is healthy; only the edge-node `/health`
  and staging are down. Summary: `SERVICES: 2/3 healthy`, result `Finished:
  UNSTABLE` (never FAILURE — the boundary held).

## Verdict

**Not a root cause — a downstream echo, surfacing correctly.** This is **museum
trap #1** in its intended-behavior form: the orchestrator's Reconcile Build
Graph / per-child stages exist precisely TO surface downstream UNSTABLE/FAILURE
as orchestrator UNSTABLE (never FAILURE — the fail-regime boundary in the museum
record, `2026-06-02-…-museum.md` lines 86–91: post-dispatch stages must
`catchError(…UNSTABLE…)`, which they do here). The orchestrator is reporting the
truth about its children, not failing itself.

## Root cause

The children's states, already canonicalized:

- `elohim-genesis` UNSTABLE (1100) / FAILURE (1101) → the TS2739 concern
  (`ci-genesis-projectionspec-ts2739.md`, already fixed) + the degraded-substrate
  concern (`ci-alpha-cluster-degraded-substrate.md`).
- `elohim` UNSTABLE → Upload SPA Blob against degraded alpha
  (`ci-alpha-cluster-degraded-substrate.md`) + the lamad bundle build
  (`ci-lamad-attention-flow-null-contentid.md`, already fixed).
- `elohim-edge` UNSTABLE → alpha-doorway deploy against degraded cluster
  (`ci-alpha-cluster-degraded-substrate.md`) + the doorway image fixture gate
  (`ci-doorway-dockerfile-fixture-context.md`, already fixed).

The orchestrator carries **no independent defect**. There is nothing to fix in
the orchestrator Jenkinsfile or graph-walker.

**Breadcrumb — orchestrator FAILURE (not UNSTABLE) at a build-LEVEL stage is a
different shape (also an echo, also not a root cause):** the fingerprints above
are all *post-dispatch* UNSTABLE echoes. A separate one-shot was
`c9624ee1d1fe` (elohim-orchestrator **#1200**, `stage:Level 0: elohim`,
build result **FAILURE**). Unlike the post-dispatch stages, the build-graph
*Level N* stages run children and `error` out on a child FAILURE (not
catchError-UNSTABLE) — so a Level-0 FAILURE means a Level-0 child *hard-failed*,
not the degraded-substrate UNSTABLE echo. Here the child was `elohim-app` #1521
(FAILURE) throwing `MethodTooLargeException: WorkflowScript.___cps___7636` — the
CPS Jenkinsfile-method-size breach (museum: Jenkinsfile 64KB CPS limit). That
breach was fixed in-tree (`ec581d5ea` "CPS breach cut 2 — the killer was the
Upload-SPA-Blob script block"; `b3755bf9` was an incomplete cut 1) and
**parse-verified by elohim #1522 running every stage end-to-end**. So
`c9624ee1d1fe` was triaged `decompose_on_confirm` (transient, resolved, no
lasting lesson — closes by orchestrator green streak), NOT folded into this
persistent-echo concern. Recorded here only as the breadcrumb that an
orchestrator *Level-N FAILURE* points you at the child build, not at this doc.

## Current decision

**BLOCKED-as-echo — resolves automatically when the upstream concerns resolve.**
No tree change: per the museum record, an agent reading a red/UNSTABLE
orchestrator as its own bug and "fixing" the orchestrator is the canonical
anti-pattern (the deepest trap). The correct move is to let the four real
concerns close (one substrate-blocked, three already-fixed-await-confirm); the
orchestrator's drift verdict clears on the same green streak.

All three fingerprints set `status: blocked` in the ledger (blocker: upstream
children — see the cited backlog entries). No `triaged_at_build` (nothing landed
in the orchestrator). Recurrence tracks the children and is expected until they
clear — not an orchestrator re-fire bug.

For the new fingerprint `ddd8ed2cbdc7` (Post-flight Health Check, builds
1188–1192), the unhealthy service is the **alpha-edge node** (`alpha-edge.elohim.host/health`
= HTTP 000) plus **doorway-staging** (503). Both are operator-owned substrate:
the edge/staging health is the same degraded-alpha condition canonicalized in
`ci-alpha-cluster-degraded-substrate.md` (operator-owned, never `kubectl` from
dev — the repo manifests in `genesis/manifests/` are the only cleanup surface).
The Post-flight stage is doing exactly its job: surfacing edge-down as
orchestrator-UNSTABLE while the alpha doorway (the seeded backend) is verified
healthy (200, conductor-connected). Blocker: alpha-edge/staging availability
(operator). It clears when those endpoints come back; the harvester confirms by
green streak.

## 2026-06-10 — `a90e18c0cf94` "stage:Post Actual Build Graph" folded in (skip-echo, the breadcrumb confirmed)

Fingerprint `a90e18c0cf94` ("red build, stage:Post Actual Build Graph", seen 5,
builds 1171–1199) is the **same Level-N-child-FAILURE echo** the breadcrumb
above already anticipated — confirmed, not a new shape. The dispatch hypothesis
("its own failure shape on some builds, a skip-echo on others") resolves to:
**it is ALWAYS a skip-echo; `Post Actual Build Graph` never fails on its own.**

Mechanism (verified by reading + #1199 log):

- `Post Actual Build Graph` is an OBSERVATIONAL post-dispatch stage,
  `catchError(buildResult: 'UNSTABLE', stageResult: 'UNSTABLE')`-wrapped
  (`genesis/orchestrator/Jenkinsfile:1896`, with the explicit fail-regime
  comment at 1887-1895). It therefore **cannot turn the build FAILURE** — the
  worst it can do to its own exceptions is UNSTABLE. So a FAILURE'd orchestrator
  was never this stage failing.
- When the orchestrator goes **FAILURE** (not UNSTABLE), a build-LEVEL stage
  (`Level N: <pipeline>`) `error`'d out on a hard child FAILURE BEFORE the
  observational stages ran. Declarative-pipeline then marks every later stage
  `skipped due to earlier failure(s)` — including `Post Actual Build Graph` and
  `Reconcile Build Graph`. The harvester's coarse line-match attributes the
  build's FAILURE to the first such named stage it positions on, yielding the
  misleading "stage:Post Actual Build Graph" line for what is really a
  child-FAILURE-then-skip.

Evidence (#1199, FAILURE):
- `Build elohim-app » dev #1520 completed: FAILURE` → orchestrator echoes
  `❌ elohim: FAILURE` (the Level-0 child hard-failed).
- `Stage "Post Actual Build Graph" skipped due to earlier failure(s)` and
  `Stage "Reconcile Build Graph" skipped due to earlier failure(s)`.
- `[baseline:post] currentBuild.result=FAILURE — NOT advancing __global__`,
  then `Finished: FAILURE`.
- #1171 (first occurrence) is the same shape: result FAILURE, description
  `auto: elohim, elohim-storybook, elohim-genesis` (a child failed).

So `a90e18c0cf94` points at the **child build** (here elohim-app #1520 — the
elohim-app failure is owned by other concerns, e.g. the lamad-shell-routing /
steward-portal-handoff regressions), exactly as the breadcrumb said an
orchestrator Level-N FAILURE does. There is nothing to fix in the orchestrator
for this fingerprint; it clears when the elohim-app child goes green.

Ledger: `a90e18c0cf94` → `status: blocked` (blocker: the upstream elohim-app
child FAILURE). No `triaged_at_build` (nothing landed in the orchestrator).
Recurrence tracks the child and is expected until it clears.

Contrast with the SIBLING orchestrator concern opened the same day,
`ci-orchestrator-graph-only-pipeline-dispatch-leak.md` (`97d7fb9c085c`,
"stage:elohim-doorway-app"): that one is a **real orchestrator code bug**
(JSONNull defeats the jenkinsPath dispatch filter), NOT an echo. Two
orchestrator fingerprints, two different verdicts — this one is the echo; that
one is the bug. Keep them distinct.

## 2026-06-10 — `c9624ee1d1fe` "stage:Level 0: elohim" RECURRED at #1205 — folded in (the breadcrumb was right; `decompose_on_confirm` was wrong)

`c9624ee1d1fe` ("red build, stage:Level 0: elohim", orchestrator **FAILURE**)
was originally recorded ONLY as the breadcrumb above (the #1200 instance: its
Level-0 child was elohim-app #1521 throwing `MethodTooLargeException` — the CPS
breach, since fixed by `ec581d5ea`). It was stamped `triaged_at_build: 1202` +
`decompose_on_confirm: true` (transient, resolved, no lasting lesson). **The
harvester reopened it at #1205 (seen→4, last_build 1205)** — and the dispatcher
asked whether the JSONNull `jenkinsPath` fix (`bcbae389a`) failed.

**It did not. The JSONNull bug is a DIFFERENT fingerprint.** The JSONNull
dispatch-leak is `97d7fb9c085c` / "stage:elohim-doorway-app"
(`ci-orchestrator-graph-only-pipeline-dispatch-leak.md`), and that fix landed
and is unrelated here. `c9624ee1d1fe`'s `line` is "stage:Level 0: elohim" — a
**Level-0 child hard-FAILURE skip-echo**, the exact shape this concern's
breadcrumb (and the `a90e18c0cf94` section) already names. Verified against
#1205 (FAILURE):

- `Build elohim-app » dev #1525 completed: FAILURE` (orch log line 550) →
  `❌ elohim: FAILURE` (line 552) → the Level-0 stage `error`'d.
- Every later observational stage shows `skipped due to earlier failure(s)`
  (Post Actual Build Graph, Reconcile Build Graph, Verify Deployment,
  Post-flight Health Check, the four Advisory stages) → `Finished: FAILURE`
  (line 698). The harvester's coarse match positions on the first named
  Level-0 stage → the "stage:Level 0: elohim" line.
- **No `MethodTooLargeException` anywhere in #1525** — the CPS breach is
  genuinely gone; this recurrence is NOT the #1200 CPS shape coming back.

**Why elohim-app #1525 hard-failed: the degraded-alpha substrate** (verified in
the #1525 log) — NOT an orchestrator bug, NOT a CPS breach:
- Upload SPA Blob → `exit code 22` → UNSTABLE (line 5732), the degraded-alpha
  PUT/PATCH facet;
- `EPR mount https://alpha.elohim.host/ does not serve after blob staging`
  → 404, `App ZIP blob not found` (line 5742);
- the post-deploy E2E health gate → `exit code 124` → **FAILURE** (line 6752) —
  the alpha edge never served 200/30x post-deploy.
These are exactly facets #4/#6 of `ci-alpha-cluster-degraded-substrate.md`
(Upload-SPA-Blob + the E2E availability gate). So #1205's Level-0 echo points
at the elohim-app child, whose hard FAILURE is owned by the degraded-substrate
concern.

**The coarseness lesson (why `decompose_on_confirm` was wrong, corrected here).**
"stage:Level 0: elohim" is a **coarse fingerprint** — it carries no child/cause
identity, so it re-pins to *whatever* child hard-fails at Level 0: #1200 it was
the CPS breach; #1205 it is the degraded-alpha child. Marking it
`decompose_on_confirm` treated it as a one-shot tied to the CPS fix — but a
coarse Level-N-echo fingerprint is **persistent**, not transient: it recurs
every time any Level-0 child hard-FAILs (just as `a90e18c0cf94` recurs for
Post-Actual-Build-Graph). It belongs here, folded into this echo concern, as a
documented persistent echo — `status: blocked` (blocker: whichever Level-0
child hard-failed; currently the degraded-alpha elohim-app), `triaged_at_build`
and `decompose_on_confirm` **cleared** (it is not a fix awaiting disappearance;
it is an echo that disappears when the children go green). This mirrors exactly
how `9f60eb44561d` (the generic "PIPELINE FAILED" banner) was re-homed off its
stale `triaged_at_build: 1101` stamp in the alpha-cluster concern — same
coarse-banner-re-pins-to-the-last-holder mechanism. Classifier lesson
(harvester-side, not sentinel; already in the alpha-cluster concern's Fix
trail): prefer fingerprinting the **failing-stage CAUSE line** (the child name +
its hard-fail signature) over a generic "Level N: <pipeline>" stage banner.

## Fix trail

- No change (intentional — orchestrator is reporting correctly; the fix surface
  is the four upstream concerns, plus, for `a90e18c0cf94` and now
  `c9624ee1d1fe`, the elohim-app child build owned by its own concern —
  currently `ci-alpha-cluster-degraded-substrate.md`).
- This entry exists so the next sentinel run does not re-investigate the
  orchestrator as a novel concern: it is, and will remain, an echo until the
  children are green.
