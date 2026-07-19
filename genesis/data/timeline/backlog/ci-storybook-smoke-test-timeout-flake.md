---
id: "backlog-ci-storybook-smoke-test-timeout-flake"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-storybook Smoke-Test Stories red = build-agent pod eviction mid-run (InterruptedException) layered on the marginal designed-story timeout budget — infra flake, NOT a push regression"
slug: "ci-storybook-smoke-test-timeout-flake"
written: "2026-07-19"
author: "ci-failure-triage"
status: "backlog"
priority: "low"
ci_status: blocked
fingerprints: [14f926653a27]
jobs: [elohim-storybook]
relatedNodeIds: []
tags: [ci, infra, jenkins-kubernetes-plugin, agent-pod-eviction, InterruptedException, storybook, test-runner, smoke-test-story-budget, timeout, flake, retrigger-recovers, host-green-not-ci-green, operator-owned]
cites:
  - https://jenkins.ethosengine.com/job/elohim-storybook/job/dev/218/
  - https://jenkins.ethosengine.com/job/elohim-storybook/job/dev/217/
  - app/elohim-library/projects/graphos/src/designed/core/__docs__/elohim-reaction-bar.designed.stories.ts
  - app/elohim-library/projects/graphos/src/designed/core/__docs__/elohim-navigator.designed.stories.ts
  - app/elohim-library/projects/graphos/src/designed/core/__docs__/elohim-epr-relationships-panel.designed.stories.ts
  - app/elohim-library/projects/graphos/src/designed/core/__docs__/elohim-graduated-feedback.designed.stories.ts
  - app/elohim-library/scripts/test-storybook-ci.sh
  - genesis/data/timeline/backlog/ci-jenkins-k8s-pod-exec-websocket-transient.md
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
---

# elohim-storybook Smoke-Test Stories — agent eviction mid-run over a marginal story-timeout budget

## The failure

```
fp 14f926653a27  elohim-storybook — "FAILED at stage: Declarative: Post Actions"
                 (elohim-storybook/dev #218, FAILURE, seen 10, builds 143..218)
```

Occurrence evidence (harvester-owned): **seen 10** across a wide **143..218**
build span — a long-lived *intermittent*, not a fresh break. Prior build
**#217 was SUCCESS**. The ledger `line` is the generic post-actions
catch-all; the real failure is in the **Smoke-Test Stories** stage
(`test-storybook:ci` → `@storybook/test-runner` over 73 suites).

Build #218's stage summary:

```
Test Suites: 4 failed, 69 passed, 73 total
Tests:       7 failed, 389 passed, 396 total
Time:        483.88 s
 ELIFECYCLE  Command failed with exit code 1.
```

The four failing suites, **in run order**, with the decisive infra line spliced
where it appeared in the log:

```
FAIL designed/core/elohim-reaction-bar.designed.stories.ts   (81.567 s)   ← pre-eviction
       ● Light › smoke-test  — thrown: "Exceeded timeout of 30000 ms for a test."
FAIL designed/core/elohim-navigator.designed.stories.ts      (83.626 s)   ← pre-eviction
       ● Light › smoke-test  — "…error … most likely because of a navigation … Retrying…"
--- elohim-storybook-dev-218-w7f7j-pxvqk-tc9k5 seems to be removed or offline
    (java.lang.InterruptedException); will wait for 5 min 0 sec for it to come back online
--- elohim-storybook-dev-218-w7f7j-pxvqk-tc9k5 is back online
FAIL designed/core/elohim-epr-relationships-panel.designed.stories.ts (249.308 s) ← post-eviction
       ● Light › smoke-test / ● Dark › smoke-test  — Exceeded timeout of 30000 ms
FAIL designed/core/elohim-graduated-feedback.designed.stories.ts      (253.972 s) ← post-eviction
       ● Light / ● Dark  — Exceeded timeout of 30000 ms
       ● ProposalWithAggregates › smoke-test — page.evaluate: ReferenceError: __test is not defined
```

## Verdict — FLAKE (infra-eviction dominant, marginal story-budget secondary)

Two intertwined causes; **neither is a code regression from the #218 push**:

1. **Dominant — Jenkins build-agent pod eviction mid-run (infra).** The agent
   pod went offline with `java.lang.InterruptedException` and the controller
   waited 5 min for it to return. The two suites that failed **after** the
   eviction inflated to **249 s / 254 s** — far past the 30 s per-test
   timeout — because the test-runner's wall clock kept ticking while the pod
   was frozen/gone. The `__test is not defined` in graduated-feedback is the
   page-crash artifact of the same disruption (the injected test global lost
   on reload), not a story bug. This is the same **agent-pod-churn** family
   already canonicalized in
   `ci-jenkins-k8s-pod-exec-websocket-transient.md` — a `host-green ≠
   CI-green` inversion (build/stories fine, CI *transport/agent* died).

2. **Secondary — marginal designed-story timeout budget.** The two suites
   that failed **before** the eviction (reaction-bar 81.5 s, navigator
   83.6 s = the 30 s test retried 2–3×) are the known **Smoke-Test
   Stories story-budget** trap (memory `project_storybook_smoke_test_story_budget`):
   the stage runs near its cumulative ceiling and heavy `designed` (Library B)
   stories intermittently tip a previously-green run over. The navigator
   "…because of a navigation … Retrying…" lines are the runner re-attempting a
   stalling render.

**Why it is NOT a fresh push regression** (the dispatch framing corrected by
evidence):

- The #218 changeset (`2e7f8c8..35b0cb62`: epr-rea crate/docs, epr-cli flow,
  deprecation triage, SSR compose + multi-app, gate-debt repair) touches
  **zero** files under `app/elohim-library/projects/graphos/src/designed/` —
  none of the four failing stories were modified.
- #217 ran the same story set and passed.
- The fingerprint's **seen=10 / 143..218** span is the harvester's own
  signature of a recurring intermittent, not a step change at #218.

## Museum gate

Checked against the trap list
(`…/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md`): the
eviction cause is the **agent-pod-churn / host-green ≠ CI-green** family —
a museum-*candidate* already tracked via the pod-exec-websocket transient
entry, not (yet) a ranked museum trap. The story-budget cause is a
project-local gate quirk documented in memory, not a ≥3-shift orchestrator
anti-pattern. **No new museum trap graduates from this run.** If the
agent-eviction-during-storybook class earns a frequency rank alongside the
pod-exec-404 transient, the two graduate together as one "Jenkins build-agent
pod churn kills a stage mid-run — retrigger first" trap **into** the existing
museum doc (never fork a second lessons file).

## Root cause

- **#1 (this build's red):** the `elohim-storybook-dev-218-…` build-agent pod
  was evicted / went `InterruptedException`-offline for ~5 min during the
  test-runner stage — apiserver/kubelet/node pressure on the CI build node
  (the same infra surface as the pod-exec-404 transient). Un-fixable in-tree.
- **#2 (the marginal amplifier):** the `designed` core stories
  (reaction-bar, navigator, and the two post-eviction suites) sit heavy enough
  that the shared Smoke-Test budget has little headroom; a slow node makes the
  30 s per-test timeout fire and retry, converting a slow run into a red one.

## Current decision

`ci_status: blocked` — **no clean bounded in-tree fix would have made #218
green.** The dominant cause is an operator-owned build-agent eviction, and the
failing stories were untouched by the push, so there is no code regression to
revert. The unblock is twofold:

1. **Immediate (integrator/operator):** the next `dev` push re-runs the
   storybook pipeline; a clean node clears the eviction (as #217 was green,
   the transient is non-deterministic). Recurrence reopens the fingerprint
   automatically; the harvester closes it by **disappearance** (green-streak
   ≥3, no recurrence).
2. **If the *pre-eviction* story-stall recurs without an eviction (≥2–3×):**
   `graphos-designer` owns the mitigation — trim the heavy `designed`
   core stories (start with `elohim-reaction-bar.designed`,
   `elohim-navigator.designed`) toward the ~3–4-story / trimmed-fixture budget
   per `project_storybook_smoke_test_story_budget`, verified eyes-on against a
   local `pnpm storybook` render (CI-green ≠ binding-correct). That is a
   graphos surface + needs render verification a background CI-triage agent
   cannot do blind, so it is **not** landed here.
3. **If agent eviction recurs during storybook:** operator investigates
   CI-build-node stability (eviction churn / memory pressure), same surface as
   `ci-jenkins-k8s-pod-exec-websocket-transient.md`; the repo-side lever, if
   justified, is a `retry {}` wrapper around the test-runner `sh` in the
   storybook Jenkinsfile.

Held at `low` priority: intermittent, retrigger-clears, no product-code fault.

## Fix trail

- No code change (background CI-triage cannot cure an agent-pod eviction, and
  the failing stories were not touched by the push).
- Canonicalized here from Jenkins evidence (build #218 log: eviction line +
  per-suite timings + 4-failed/69-passed summary; #218 changeset; #217 green).
- Ledger `14f926653a27`: `status: open → blocked`, `backlog` pointer set. NOT
  stamped `triaged`/`decompose_on_confirm` — the lesson is museum-candidate
  and closure is disappearance-observed, not asserted.
