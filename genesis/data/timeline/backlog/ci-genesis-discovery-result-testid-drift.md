---
id: "backlog-ci-genesis-discovery-result-testid-drift"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Discovery-assessment result coverage drifted against dead testids — know-thyself scenarios assert discovery-subscale-score/discovery-profile, superseded by the shipped assessment-completion-summary component (completion-* testids)"
slug: "ci-genesis-discovery-result-testid-drift"
written: "2026-07-02"
author: "ci-failure-triage"
status: "wip"
priority: "medium"
ci_status: in-progress
fingerprints: [529bba700060]
jobs: [elohim-genesis]
relatedNodeIds: []
tags: [ci, genesis, lamad, discovery, browser-only, testid-drift, requires-shem, scope-return, host-green-not-ci-green]
cites:
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1234/
  - genesis/a2o/features/lamad/know-thyself-discovery.feature
  - genesis/a2o/features/lamad/assessment-completion-feedback.feature
  - genesis/a2o/steps/ui/discovery-assessment.steps.ts
  - genesis/a2o/steps/ui/completion-feedback.steps.ts
  - genesis/a2o/src/framework/pages/selectors.ts
  - app/lamad/src/app/quiz-engine/components/assessment-completion-summary/assessment-completion-summary.component.ts
  - app/lamad/src/app/content-io/plugins/sophia/sophia-renderer.component.ts
---

# Discovery-assessment result coverage drifted against dead testids

## The failure

```
529bba700060  AssertionError [ERR_ASSERTION]: Expected at least one subscale score displayed   (genesis 1234)
```

Occurrence evidence: seen 1, first_build 1234, last_build 1234 (UNSTABLE).
The failing step is `discovery-assessment.steps.ts:215` — "the resonance
result should have non-zero subscale scores" — in scenario **"Susan completes
the Attachment Style assessment"** (`know-thyself-discovery.feature:23`). The
assertion diff is `-false / +true`: the count of
`[data-testid="discovery-subscale-score"]` was **0**.

Critically, the preceding step in the same scenario — "the assessment should
complete without console errors" (`:203`) — **PASSED (✔)**. The assessment page
loaded, the likert questions rendered and were answered, and no console errors
fired. This is NOT the page/backend-down shape.

Sibling facet in the same run (same root concern, different — and correct —
testid; not separately fingerprinted, rides the generic red):

```
assessment-completion-feedback.feature:26  "Discovery completion shows subscale breakdown bars"
   ✖ Then the completion summary should be visible   → locator('[data-testid="completion-summary"]') waitFor timeout 15000ms
```

## Verdict — real (test-drift), NOT flake, NOT infra/alpha-down

Build #1234 substrate signal was **shem AVAILABLE**
(`ELOHIM_REMOTE_COMPUTE_STATUS=available`, "SUBSTRATE PROBE — remote pool (shem):
AVAILABLE"). The `@requires:shem` `know-thyself-discovery.feature` therefore
ran (not held). The assessment surface rendered fully (console-error step
passed). So this is neither the degraded-substrate condition
(`ci-alpha-cluster-degraded-substrate` — that is alpha-DOWN, page never loads)
nor a flake (a superseded selector fails deterministically on every run).

## Root cause — a component refactor superseded the testids the scenario asserts

The know-thyself result assertions were authored in `fec6357cb`
(likert-scale widget + expanded discovery assessments) against a
discovery-result rendering contract of the form
`discovery-subscale-score` / `discovery-profile` / `discovery-primary-type` /
`discovery-attestation-badge` (the `DISCOVERY.*` constants in
`selectors.ts`). Shortly after, `ade643ce4` (personalized assessment
completion feedback) shipped an **`assessment-completion-summary` component**
(687 lines) that became the actual result view — it emits `completion-*`
testids (`completion-summary`, `completion-subscales` with `.subscale-fill`
bars, `completion-headline`, `completion-hex-badge`, …), rendered by the
sophia-renderer as `<app-assessment-completion-summary data-testid="sophia-completion-summary">`
when `showResults` is true (`sophia-renderer.component.ts:105`). The component
was then relocated to `app/lamad/` by the frontend-consolidation refactors
(`96097e81b`, `6195cdfe9`).

The legacy `discovery-subscale-score` / `discovery-profile` /
`discovery-primary-type` / `discovery-attestation-badge` testids exist in **no
shipped component anywhere in the repo** (grep-verified across app/elohim-app
AND app/lamad). They were never reconciled after the completion-summary
component superseded them. So the know-thyself result assertions target a
rendering contract that exists in **no deployed version** — they cannot pass on
any alpha state.

**Why it surfaced now (the mechanism worth remembering):** the feature is
feature-level `@requires:shem`. While shem was offline it was `git mv`'d to
`held/` (out of the cucumber glob) by the scope-reconciler and could not fail.
`41c565f3e` (shem back online — expand the plate) moved it back onto the live
plate; the dormant drift surfaced as red on the first post-return run (#1234).
Held features rot silently against the evolving app; scope-return is when the
bill comes due. (First clear instance of this pattern — below the museum's
≥3-shift bar, so noted here rather than graduated. Adjacent to the existing
"host-green ≠ CI-green" / test-drift-vs-shipped-components repair `80c959d8c`.)

## Current decision

Bounded fix landed: **`@wip` the three discovery-RESULT scenarios** in
`know-thyself-discovery.feature` (Values Hierarchy, Attachment Style [the
fingerprint], First-discovery attestation) — the CI browser run filters
`@e2e and @browser-only and not @wip` (`genesis/scripts/ci/e2e-verify-browser.sh`),
so the drifted-testid failure leaves the suite. The two `@regression`
scenarios (single-likert-selection, no-console-errors-navigation) stay LIVE —
they guard the shipped likert widget + navigation and do not touch the result
view. No coverage is lost: the correct-testid completion coverage already lives
in `assessment-completion-feedback.feature`.

Residual (the true resolution — story-owner / lamad work, not triage now-work):
1. **Consolidate**: the know-thyself result assertions are redundant stale
   duplicates of `assessment-completion-feedback.feature`'s discovery
   scenarios. Either retire them or reconcile `discovery-assessment.steps.ts`
   (`:215`, `:256`, `:281`) + the `DISCOVERY.*` selectors onto the shipped
   `completion-*` / `sophia-completion-summary` contract, then drop `@wip`.
2. **Facet 2 — verify the deployed completion flow** (live trajectory, NOT
   yet actioned): `assessment-completion-feedback.feature:26` asserts the
   CORRECT `completion-summary` testid yet timed out at runtime (15s) on
   #1234. That is a distinct question — does the deployed alpha bundle wire the
   `app/lamad` quiz-engine completion flow so `<app-assessment-completion-summary>`
   renders after a discovery likert run, or is this post-consolidation
   deploy-lag? Verify with `pnpm look --as <FixtureHuman>` against a completed
   discovery assessment on alpha before deciding whether facet 2 is a real
   integration gap or self-resolving deploy-lag. Left live (not `@wip`) so a
   genuine integration regression stays visible.

## Fix trail

- `genesis/a2o/features/lamad/know-thyself-discovery.feature` — `@wip` +
  explanatory comments on the 3 result scenarios (this commit).
- Local verification: `npx cucumber-js --dry-run --tags '@e2e and @browser-only
  and not @wip' features/lamad/know-thyself-discovery.feature` — parses clean
  (no gherkin abort), 3 result scenarios excluded, 2 `@regression` retained.
- Ledger `529bba700060`: `status: triaged`, `triaged_at_build: 1234`. NO
  `decompose_on_confirm` — this backlog graduates (consolidation + facet-2
  verification are real residual work), not auto-decompose. The sweep confirms
  the `@wip` fix by disappearance (genesis green-streak ≥3, no recurrence).
