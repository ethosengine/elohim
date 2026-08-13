---
id: "backlog-ci-jenkins-controller-restart-orphans-wave"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A Jenkins controller restart orphans the whole dispatched wave — ABORTED downstream, FAILURE orchestrator, and a FAKE Groovy syntax error from the resume-time reparse (2026-08-13, #1669/#1343/#1664)"
slug: "ci-jenkins-controller-restart-orphans-wave"
written: "2026-08-13"
author: "ci-failure-triage"
status: "wip"
priority: "medium"
ci_status: in-progress
fingerprints: [2ec906730fe7]
jobs: [elohim-orchestrator, elohim-edge, elohim]
relatedNodeIds: []
tags: [ci, infra, jenkins-controller-restart, durable-task, cps-resume, aborted-not-failed, lossy-measure, orphaned-agent-pod, retrigger-recovers, operator-owned, museum-graduated]
cites:
  - https://jenkins.ethosengine.com/job/elohim-orchestrator/job/dev/1669/
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1343/
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1664/
  - .claude/scripts/ci-harvest.py
  - .claude/data/failure-taxonomy.json
  - .claude/scripts/_lib/__tests__/ci_harvest_controller_restart_test.py
  - genesis/data/timeline/backlog/ci-jenkins-k8s-pod-exec-websocket-transient.md
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
---

# A controller restart orphans the wave — three symptoms, one cause, zero code fault

## The failure

```
fp 2ec906730fe7  elohim-orchestrator — red build, stage:elohim-edge
                 (elohim-orchestrator/dev #1669, FAILURE, seen 1, build 1669..1669)
```

Occurrence evidence: seen once, first/last build 1669. The ledger `line` —
`red build, stage:elohim-edge` — is the harvester's UNCLASSIFIED fallback
(`collect_build_findings` step 4, the first wfapi stage in a FAILED state). It
is **positionally true and causally misleading**: the orchestrator never
evaluated a downstream verdict at all.

The Jenkins controller restarted at roughly **19:05 UTC on 2026-08-13**, about
four minutes into a wave that orchestrator #1669 had just fully dispatched. It
came back and resumed the durable pipelines at **21:34–21:35 UTC** — a ~2h30m
outage. Three jobs, three different-looking deaths, one cause:

**1. `elohim-edge/dev` #1343 — ABORTED (not FAILURE).**

```
#33 128.4 Compiling fixt v0.7.0-dev.1                 # last live output, ~19:03
Resuming build at Thu Aug 13 21:35:01 UTC 2026 after Jenkins restart
Waiting for reconnection of elohim-edge-dev-1343-2xj5w-f2tpg-w93g9 before proceeding with build
Timeout expired 33 min ago
Cancelling nested steps due to timeout
Could not connect to elohim-edge-dev-1343-2xj5w-f2tpg-w93g9 to send interrupt signal to process
```

The build's outer `timeout` was set to 2h at 19:02 and expired at ~21:02 —
**during the outage, while nothing was running**. On resume it fired
immediately. `Quality Gate: Doorway` was cancelled (`Timeout has been
exceeded`, caught as non-blocking → `unstable()`); `Build Doorway` started
fresh at `BUILD_TIMESTAMP=2026-08-13T21:35:04Z`, got ~60s into `cargo build
--release`, and was killed with `Body did not finish within grace period;
terminating with extreme prejudice`. All eleven remaining stages read `skipped
due to earlier failure(s)`. Final result: **ABORTED**.

**2. `elohim/dev` #1664 — FAILURE.** Same restart, same signature:
`Resuming build at Thu Aug 13 21:34:56 UTC 2026 after Jenkins restart` /
`Waiting for reconnection of elohim-dev-1664-p3hd7-87jz3-s2tk7`.

**3. `elohim-orchestrator/dev` #1669 — FAILURE, and this is the trap.** Its
persisted CPS program could not be reparsed on resume:

```
Starting building: elohim-edge » dev #1343
Starting building: elohim-app » dev #1664
org.codehaus.groovy.control.MultipleCompilationErrorsException: startup failed:
WorkflowScript: 1627: expecting ')', found '(' @ line 1627, column 47.
               if (commitMsg =~ /(?i)\[depl
                                 ^
	at ...CpsFlowExecution.parseScript(CpsFlowExecution.java:669)
	at ...CpsFlowExecution.loadProgramAsync(CpsFlowExecution.java:927)
	at ...FlowExecutionList.resume(FlowExecutionList.java:127)
ERROR: Failed to load program
```

**That syntax error is not real.** The named statement —
`if (commitMsg =~ /(?i)\[deploy[-_ ]only\]/)` at `genesis/orchestrator/Jenkinsfile`
— is valid Groovy that #1669 *had already compiled and executed* (it reached
full changeset analysis and dispatched six pipelines), and that the very next
build, **#1670, compiled again without complaint**. Two independent tells that
the stored script was truncated rather than wrong: the frame is
`FlowExecutionList.resume → onLoad → loadProgramAsync → parseScript` (a
resume-time reparse, not the build-time parse), and the echoed source line
**stops mid-token** at `\[depl`. Column 47 is the `(` of `(?i)`; with the
literal truncated, Groovy stops seeing a slashy-string and reads `/` as
division. The controller went down hard enough not to flush the build's
persisted script.

## Verdict — INFRA (env-red). Not code-red, and not an unrelated stage.

The dispatch question was whether #1343's red implicates commit `f125282a8`
(*fix(doorway): one pooled client per storage upstream — retire
ssr_http_client*), which rode in with `7501eb57b` under a `[build:edge]` tag.
It does not, and the evidence is exhaustive rather than inferential: a regex
sweep of all **2771** log lines of #1343 for `error[E\d+]`, `^error`, `FAILED`,
`panicked`, `ERROR`, `clippy` and `Diff in` returns **five** hits, and every
one is benign — three are the string `clippy` inside `RUN rustup component add
rustfmt clippy`, and two are pre-existing `warning:` lines (`unused import:
std::sync::Arc` at `elohim/constitution/src/stack.rs:7`, `field cached_at is
never read` at `elohim/constitution/src/verification.rs:186`) in a **different
crate** from the one the commit touched. There is no compile error, no clippy
denial, and no failed test anywhere in the build.

The precise status of the doorway change is **unmeasured, not exonerated**:
`Quality Gate: Doorway` was ~128 seconds into a `cargo` compile inside its
Dockerfile build when the controller went down, and its post-resume retry was
killed ~60s in. CI never rendered a verdict on `f125282a8` either way. The
next wave measures it.

Ruled out explicitly:

- **Superseded / `abortPrevious`** (museum trap #1's usual mechanism) — #1343
  is the newest build of `elohim-edge/dev`; nothing preempted it. The ABORTED
  here is *restart-orphaned*, not superseded, and the two demand opposite
  responses (see the museum note below).
- **The pod-exec websocket transient**
  (`ci-jenkins-k8s-pod-exec-websocket-transient.md`) — that is a
  `ContainerExecDecorator` 404 killing one `sh` mid-stage on an otherwise-live
  controller. Here the controller itself restarted, three jobs died together,
  and the tell is `Resuming build … after Jenkins restart`. Sibling family
  (agent-pod channel loss), different mechanism.
- **A real orchestrator Jenkinsfile regression** — #1670 builds the same file
  clean. The parse error is a resume artifact.
- **The alpha-cluster degraded substrate** — nothing reached a deploy or
  validation stage; every one of them was skipped.

## Root cause

A hard Jenkins controller restart at ~19:05 UTC mid-wave. Three consequences,
in the order an investigator meets them:

1. Every in-flight k8s agent pod was orphaned; durable-task resume waits for a
   reconnection that can never happen (`Could not connect to … to send
   interrupt signal`).
2. Wall-clock `timeout` blocks keep counting **through the outage**, so a 2h
   budget can be fully spent on an idle build and fire the instant it resumes
   — the work is destroyed by a timeout that measured downtime, not work.
3. A build whose CPS script was not flushed cleanly fails to reload with a
   **fabricated Groovy compile error** pointing at valid source.

## Current decision

`ci_status: in-progress`. The concern has two halves and they resolve
differently.

**The cause is operator-owned and needs no in-tree change.** The remedy for an
orphaned wave is a retrigger — already in motion: orchestrator **#1670** is
building. Nothing in `genesis/orchestrator/Jenkinsfile` or
`elohim/holochain/Jenkinsfile` should be edited in response, and the museum's
standing rule applies (the root `Jenkinsfile` is near the 64KB CPS limit;
CI watch-outs route to docs, never to inline pipeline logic). If controller
restarts become frequent rather than incidental, the operator move is Jenkins
controller stability (memory headroom / eviction pressure on the controller
pod) — not pipeline surgery. A `timeout` that excludes downtime is not
expressible in declarative Pipeline, so "raise the timeout" is not the fix
either; it would only lengthen the window in which an outage is silently
absorbed.

**The measure was blind, and that half is fixed this run** (see Fix trail).
Before it, this class landed as `UNCLASSIFIED / red build, stage:<downstream>`
— a line that names the wrong culprit and costs a background triage dispatch
every occurrence, with a *fresh fingerprint each time* because agent-pod names
carry per-build random suffixes. Now it self-classifies as `CONTROLLER_RESTART`
with a stable fingerprint and a remediation hint the deterministic layer can
answer with.

What unblocks full closure: the retriggered wave (#1670 →
`elohim-edge/dev` #1344+) rendering an actual verdict on `f125282a8`, and the
harvester confirming disappearance via an `elohim-orchestrator` green streak
≥3 with no recurrence.

## Fix trail

Measure-layer fix, locally verified, commit-only (integrator pushes):

- `.claude/data/failure-taxonomy.json` — new **`CONTROLLER_RESTART`** category,
  placed **first** so the decisive fact is not crowded out of
  `MAX_CONSOLE_FINDINGS_PER_BUILD = 4` (museum trap #14's *displacement*
  lesson), applying to all eight harvested jobs because a controller restart
  is not per-pipeline. Its `search` requires an actual breakage marker —
  `Failed to load program`, `Waiting for reconnection of … before proceeding
  with build`, `Could not connect to … to send interrupt signal to process`.
  It deliberately does **not** match the benign `Resuming build … after Jenkins
  restart` (a build that resumes cleanly and then fails for a real reason must
  keep its real cause) nor the generic `Body did not finish within grace
  period` (a genuine hang is a different concern). `blocks: []` — a restart is
  not a dependency fault.
- `.claude/scripts/ci-harvest.py` `normalize()` — scrub Jenkins agent-pod names
  (`<job>-<build>-<5>-<5>-<5>`). The 5-char suffixes are neither 7+-hex nor
  duration-shaped, so the existing hash scrub missed them entirely and every
  restart would have minted a new fingerprint for the same concern. Placed
  before the hash scrub so the whole name is consumed intact; narrow enough
  that ordinary hyphenated identifiers (`elohim-storage-client`) survive.
- `.claude/scripts/_lib/__tests__/ci_harvest_controller_restart_test.py` — new,
  26 checks: category presence and scan-order primacy, all-job applicability,
  the three decisive lines classifying, the four non-capture rails, and
  fingerprint stability across two different pods in two different jobs.

Verification (in-container, plain `python3`, exit codes echoed):

```
ci_harvest_controller_restart_test: 26 checks passed   EXIT=0
ci_harvest_echo_test:               26 checks passed   EXIT=0
ci_harvest_no_measure_test:         23 checks passed   EXIT=0
runtime_harvest_test:               38 assertions      EXIT=0
```

End-to-end through `_scan_console` on the real #1669 and #1343 tails: the
orchestrator tail yields one `CONTROLLER_RESTART` finding
(`ERROR: Failed to load program`) instead of the misleading
`stage:elohim-edge`; the edge tail yields two, both pod-name-scrubbed.

Ledger `2ec906730fe7`: `status → triaged`, `triaged_at_build: 1669`. **Not**
stamped `decompose_on_confirm` — this entry is cited by the museum row it
graduated (below), so it must survive as that row's detailed reference;
on confirmed disappearance the harvester reports it for graduate-then-decompose
rather than deleting it silently.

## Museum-graduation note

Graduated this run as **row #15** of
`genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md`
(extended in place — never a second lessons doc). It earns a row on first
occurrence, on the same grounds #11 did: it is a **new structural class**, not
a recurrence of trap #1.

Trap #1 teaches *ABORTED is not a failure — a superseded build is not a
regression, so do nothing*. That advice is **actively wrong here**: this
ABORTED build's work was destroyed and never redone, so the correct response
is precisely the retrigger #1 tells you to withhold. Same symptom, opposite
remedy — the discriminator is the string `Resuming build … after Jenkins
restart` (restart-orphaned → retrigger) versus an `abortPrevious` preemption by
a newer build number (superseded → ignore).

And the highest-cost failure mode is the third symptom: a `MultipleCompilation
ErrorsException` naming a valid line will send a future agent to "fix" a regex
that was never broken, in a file that is near the CPS size limit and expensive
to churn. The lesson shape: *`Failed to load program` under a
`FlowExecutionList.resume` frame means the persisted CPS script was truncated
by a hard controller restart — check whether the job's NEXT build compiles the
same file before editing a single character.*
