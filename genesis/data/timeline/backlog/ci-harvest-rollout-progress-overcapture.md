---
id: "backlog-ci-harvest-rollout-progress-overcapture"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "ci-harvest over-captures successful kubectl rollout progress as DEPLOYMENT failures — the taxonomy's bare `rollout` token matches the narration of a rollout that SUCCEEDED"
slug: "ci-harvest-rollout-progress-overcapture"
written: "2026-08-02"
author: "ci-failure-triage"
status: "wip"
priority: "medium"
ci_status: in-progress
fingerprints: [3efa4f507399, 2e71d043c742, ca397410678e, ffb17d09045a]
jobs: [elohim-edge]
relatedNodeIds: []
tags: [ci, ci-harvest, taxonomy, false-positive, classifier-precision, kubectl, rollout, deployment, elohim-edge, tooling]
cites:
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1291/
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1293/
  - .claude/scripts/ci-harvest.py
  - .claude/data/failure-taxonomy.json
  - .claude/scripts/_lib/__tests__/ci_harvest_echo_test.py
  - genesis/data/timeline/backlog/ci-harvest-nerdctl-cleanup-echo-overcapture.md
  - genesis/data/timeline/backlog/ci-rbac-jenkins-deployer.md
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
---

# ci-harvest over-captures successful rollout progress as DEPLOYMENT failures

ONE concern behind four fingerprints. The harvester filed four `elohim-edge`
DEPLOYMENT "failures" that are not failures at all — they are the progress
narration `kubectl rollout status` prints on its way to **success**. Second
instance of the over-capture shape already canonicalized for `nerdctl`
(`ci-harvest-nerdctl-cleanup-echo-overcapture`), now in a second taxonomy
category — which is the graduation criterion that entry pre-committed to.

## The failure (what was fingerprinted)

Four ledger lines, all `elohim-edge`, category DEPLOYMENT:

```
3efa4f507399  Waiting for deployment "elohim-doorway-alpha-b" rollout to finish: 0 out of 1 new replicas have been updated...   seen 2, 1291→1293
2e71d043c742  Waiting for deployment "elohim-doorway-alpha"   rollout to finish: 0 out of 1 new replicas have been updated...   seen 2, 1195→1293
ca397410678e  Waiting for deployment "elohim-doorway-alpha"   rollout to finish: 1 old replicas are pending termination...      seen 4, 1195→1293
ffb17d09045a  Waiting for deployment "elohim-doorway-alpha-b" rollout to finish: 1 old replicas are pending termination...      seen 5, 1195→1293
```

In the #1293 console these lines are immediately followed by their own success
confirmation (log lines 20863–20869):

```
+ kubectl rollout status deployment/elohim-doorway-alpha-b -n elohim-alpha --timeout=300s
Waiting for deployment "elohim-doorway-alpha-b" rollout to finish: 0 out of 1 new replicas have been updated...
Waiting for deployment "elohim-doorway-alpha-b" rollout to finish: 1 old replicas are pending termination...
deployment "elohim-doorway-alpha-b" successfully rolled out
```

#1291 is identical in shape (lines 20171–20174, and 20124–20126 for alpha-A):
every one of the four captured signatures sits two lines above
`deployment "…" successfully rolled out`.

## Verdict — false positive (harvester over-capture), NOT a CI failure

- Both rollouts **succeeded** in both builds. `kubectl rollout status` exits 0;
  the `--timeout=300s` was never hit; no `exceeded its progress deadline`, no
  `error: timed out waiting for the condition` anywhere in either console.
- The captured strings are the *healthy* rollout's own progress narration. "0
  out of 1 new replicas have been updated" is what a rollout prints in its first
  seconds; "1 old replicas are pending termination" is the normal drain step.
- The builds' real UNSTABLE cause is elsewhere and already canonicalized:
  #1291 went UNSTABLE at a stage-level `unstable()` for **RBAC drift** —
  `⚠️ RBAC DRIFT: could not scale 10 orphaned StatefulSet(s) to 0 … jenkins-deployer
  lacks statefulsets/scale` (console 20242–20244), tracked in
  `ci-rbac-jenkins-deployer.md`. Neither build's UNSTABLE has anything to do
  with a doorway rollout.

**The harm is not just noise.** A stage-level `unstable()` produces no JUnit
FAILED cases, so `collect_build_findings` falls through to the console-tail scan,
which caps at `MAX_CONSOLE_FINDINGS_PER_BUILD = 4`. Four benign rollout-progress
lines are exactly enough to spend the entire per-build budget — so the real
signal in the same tail is crowded out, and each new benign permutation costs a
background triage dispatch. This is the identical harm shape recorded for the
`nerdctl` echoes at #1137.

## Root cause

`failure-taxonomy.json` → `DEPLOYMENT.search` was
`"kubectl.*failed|rollout|CrashLoopBackOff"`. The bare `rollout` alternative
matches **every** line containing the string `rollout` — and `kubectl rollout
status` emits one such line per poll interval on every healthy deploy, on every
build. There is no error context in the token, so a succeeding step's output is
indistinguishable from a failing one's.

The existing `_CMD_ECHO` guard (landed for the `nerdctl` instance) cannot help
here: these are step **output**, not `set -x` echoes, so they carry no `+ `
prefix. The echo guard fixed one surface of the class; this is a second surface.

## Museum gate — graduates as trap #14

The `nerdctl` entry set the bar explicitly: *"If the 'tool-name token matches a
command echo' shape recurs across ≥3 shifts or bites another taxonomy category,
it earns a museum row then."* It has now bitten a second category (DEPLOYMENT),
in a second shift, a month later, through a mechanism the first fix could not
cover. Graduated this run into
`2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md` as row **#14**
(*the measure over-reads*) — the opposite-polarity sibling of trap #1's lossy
measure, and a **meta-trap** like #13: it corrupts the apparatus the agent uses
to see CI at all. Per the museum's own rule, the lesson EXTENDS that record; no
second lessons doc was forked.

## Fix trail — landed, locally verified

Three layers, mirroring the precedent fix's shape (data + code + test):

1. **Data — `.claude/data/failure-taxonomy.json`.** `DEPLOYMENT.search` now
   requires error context and names the real failure shapes:
   `kubectl.*failed|rollout.*(?:failed|timed out|exceeded)|exceeded its progress deadline|error: timed out waiting|CrashLoopBackOff|ImagePullBackOff|ErrImagePull`.
   All four captured signatures stop matching; genuine rollout stalls still do.
2. **Code — `.claude/scripts/ci-harvest.py`.** Added `_BENIGN_PROGRESS`, a
   class-level guard applied beside `_CMD_ECHO` in the step-2 console scan, that
   skips rollout/statefulset progress narration and success confirmations
   regardless of which category's token matched. *Soundness:* skipping progress
   chatter cannot hide a real stall, because a genuinely failing rollout emits a
   SEPARATE non-progress error line (`error: timed out waiting for the
   condition` / `exceeded its progress deadline`) which still classifies — this
   is asserted in the test, not merely argued.
3. **Test — `.claude/scripts/_lib/__tests__/ci_harvest_echo_test.py`** extended
   (not forked — same concern, one test surface) with the rollout instance:
   verbatim #1293 progress lines assert non-capture through both the guard and
   the taxonomy regex, and the three real-failure shapes assert still-captured.
   `python3 .claude/scripts/_lib/__tests__/ci_harvest_echo_test.py` →
   **26 checks passed, exit 0** (was 8 before this run).

Ledger: all four fingerprints → `status: triaged`, `triaged_at_build: 1293`,
`decompose_on_confirm: true` (the durable lesson now lives in museum row #14, so
this entry is lesson-free residue once disappearance confirms).

## Current decision

`in-progress` — the fix is landed and locally verified; closure is by
disappearance, which the harvester confirms deterministically (elohim-edge green
streak ≥3 with no recurrence past #1293). Note that `elohim-edge` is currently
red for unrelated, separately-canonicalized reasons (P2P sim compose gap, RBAC
drift), so confirmation will lag the fix — that lag is expected and is not
evidence the fix failed. Recurrence with `last_build > 1293` would reopen
automatically and would mean a benign permutation the guard does not yet cover.
