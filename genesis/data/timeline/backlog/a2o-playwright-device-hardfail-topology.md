---
id: "backlog-a2o-playwright-device-hardfail-topology"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "steps/ui/topology.steps.ts hard-fails (not skips) when no Playwright device — 5 blob-durability scenarios red in non-browser CI runs"
slug: "a2o-playwright-device-hardfail-topology"
written: "2026-07-03"
author: "blob-durability-suite-green shift"
status: "wip"
priority: "medium"
ci_status: in-progress
resolved: "2026-07-03"
fingerprints: [5dba80d982e1]
jobs: [elohim-edge, elohim-genesis]
relatedNodeIds: [blob-durability]
tags: [ci, a2o, playwright, device-mode, resilience, topology]
cites:
  - genesis/a2o/steps/ui/topology.steps.ts
  - genesis/a2o/steps/resilience.steps.ts
  - genesis/a2o/features/resilience/observable-distribution.feature
  - scripts/ci/run-dataplane-validation.sh
  - genesis/scripts/ci/e2e-verify-browser.sh
  - genesis/a2o/CLAUDE.md
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1246/
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1253/
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1255/
---

**Scope note (2026-07-05):** this entry covers the whole `assert.fail(NO_PW_DEVICE)`
hard-fail-vs-graceful-pending inconsistency across the a2o browser step files — not only
`topology.steps.ts`. The 2026-07-03 fix below closed it for `elohim-edge`'s non-browser
dataplane-validation job (a job-scoped tag-filter workaround); the 2026-07-05 extension
closes the actual code site in `resilience.steps.ts` that recurred in `elohim-genesis`'s
own **browser-mode** E2E stage, where the tag-filter workaround does not apply (that stage
correctly runs `@browser-only` scenarios). Reopened rather than forked — same concern,
same assertion text, a site this doc already named on 2026-07-03.

# `requirePwDevice`/`requirePwDevice`-style helpers in `steps/ui/topology.steps.ts` `assert.fail` instead of returning `'pending'`

## The failure

Discovered 2026-07-03 (edge #1144, first live run of the blob-durability suite past the
`E2E_STORAGE_URL` harness gate). `scripts/ci/run-dataplane-validation.sh` runs plain
`cucumber-js` — no `E2E_DEVICE_MODE=playwright` — so any scenario that opens a browser
page via `Matthew opens "/shefa/cluster"` etc. hits `requirePwDevice` in
`genesis/a2o/steps/ui/topology.steps.ts:46`, which `assert.fail`s
(`"Matthew has no Playwright device. Is E2E_DEVICE_MODE=playwright?"`) instead of
returning `'pending'` the way the rest of the resilience-suite's browser-gated steps do
(e.g. `requirePlaywright` elsewhere per `genesis/a2o/CLAUDE.md` → "Guard:
`requirePlaywright(this)` returns null in non-Playwright mode").

Confirmed via edge #1145 (2026-07-03, first live run past the `E2E_STORAGE_URL` +
`ALLOW_SEED_SHARD_MANIFEST` gates): **5** `@concern:blob-durability` scenarios (not 3 as
originally estimated) go red for this reason every non-browser CI run — 4 in
`observable-distribution.feature` ("Operator can see their household device cluster",
"Peer-topology page aggregates by household, not by peer", "Reciprocity page shows
inflow, outflow, and net hosting", "Doorway operator dashboard topology tab is
reachable") plus 1 in `resilience.steps.ts`'s viewport-archetype step ("Resilience
hypercard stays fully inside a phone viewport" — a distinct `assert.fail('No Playwright
device found')` site, not `requirePwDevice` itself, but the same root cause).

## Why not fixed in the same shift

Different root-cause class from the seeding/data bugs fixed alongside this entry
(commit `f0c295a58`) — `requirePwDevice`/similar helpers in `topology.steps.ts` are
shared across other feature suites beyond blob-durability, so the right fix (skip
gracefully vs. tag these scenarios `@browser-only` vs. run the dataplane validation
stage in playwright mode) is a scoped design decision, not a one-line mechanical fix.

## Proposed next step

Either (a) tag these 5 scenarios `@browser-only` (they genuinely need a rendered page)
so they cleanly HELD-skip on the non-browser `@dataplane` runner, or (b) make
`topology.steps.ts`'s device-requirement helper return `'pending'` like the rest of the
suite. (a) is safer/narrower; (b) is more broadly correct but needs auditing every
other consumer of that helper first.

**2026-07-03 follow-up:** (a) alone does NOT resolve this for the dataplane-validation
job — confirmed `scripts/ci/run-dataplane-validation.sh`'s cucumber invocation uses
`--tags '@dataplane and not @wip'`, which does not exclude `@browser-only`. The complete
(a)-shaped fix is two parts together: tag the 5 scenarios `@browser-only` in the feature
files, AND extend the `--tags` filter in `run-dataplane-validation.sh` to
`'@dataplane and not @wip and not @browser-only'`. Deliberately left undone by the
`blob-durability-suite-green` shift (2026-07-03) — changing the tag filter changes which
scenarios the measure command's own report counts, which reads closer to a measure-scope
decision than a mechanical fix; left for operator/next-shift judgment rather than made
unilaterally overnight.

## Resolution (2026-07-03, operator-authorized)

Operator authorized the recommended option (a). Part 1 (tagging) was already in
place — all 5 scenarios already carry `@browser-only` in
`observable-distribution.feature` (the 4 topology scenarios + "Resilience hypercard
stays fully inside a phone viewport"). Part 2 applied: extended the `--tags` filter in
`scripts/ci/run-dataplane-validation.sh` to
`'@dataplane and not @wip and not @browser-only'`, with an inline comment anchoring the
rationale.

**Verified via cucumber `--dry-run` (no substrate needed):** the blob-durability set
drops 20 → 10 scenarios; the full `@dataplane` suite drops 37 → 27. The entire
10-scenario delta is blob-durability's `@browser-only` scenarios — no other concern and
no non-browser scenario is affected. Coverage is preserved: `@browser-only` scenarios
still run under the playwright-mode jobs (`test:browser:e2e`, delivery-browser profile).

**Expected effect on the spine's blob-durability node:** `passed=3 failed=5` (RED) →
`passed=3 failed=0` (GREEN — the 5 structural hard-fails become clean HELD-skips). This
clears the failures and greens the concern; it does not by itself lift `passed` to ≥5
(the remaining non-browser pending scenarios flip to passed only when their live-substrate
preconditions are met — a substrate-timing condition, not a code gap). The closing CI
measurement lands when this reaches `dev` and edge re-runs the Dataplane Validation stage.

## Reopened 2026-07-05 — the same site recurred in elohim-genesis's browser-mode E2E stage

**Fingerprint `5dba80d982e1`** (`AssertionError [ERR_ASSERTION]: No Playwright device
found`, elohim-genesis #1246–#1255, seen 7×) is this same class recurring on a job the
2026-07-03 fix doesn't reach. Confirmed identical across both ends of the build range
(#1246 and #1255 produce byte-identical stack traces):

```
✔ Given "content-alpha" has been distributed to at least 2 households  # steps/resilience.steps.ts:356
✖ And the browser viewport is the "phone" archetype                   # steps/resilience.steps.ts:1059
AssertionError [ERR_ASSERTION]: No Playwright device found
  at E2EWorld.<anonymous> (…/genesis/a2o/steps/resilience.steps.ts:1064:14)
- When I open the EPR resource page for "content-alpha"               # steps/resilience.steps.ts:658
```

**Why the 2026-07-03 fix didn't cover this:** that fix only changed `scripts/ci/
run-dataplane-validation.sh`'s `--tags` filter for `elohim-edge`'s non-browser dataplane
stage. `elohim-genesis` runs its own script, `genesis/scripts/ci/e2e-verify-browser.sh`,
which correctly sets `E2E_DEVICE_MODE=playwright` and **includes** `@browser-only`
(`--tags '@e2e and @browser-only and not @wip'`) — as it must, since these scenarios are
tagged specifically to run there. The scenario itself never registers a Playwright device
(no `Given human "<X>" is logged in on doorway "alpha" with device` step anywhere in
`observable-distribution.feature`'s resilience-hypercard trio), so `findPwDevice(this)`
legitimately returns `null` even in playwright mode. Confirmed via the two sibling
scenarios in the same feature (`Protocol omni toolbar surfaces the live resilience
snapshot`, `Omni resilience tooltip folds down into the viewport…`) which hit the exact
same "no device" condition at their `When I open the EPR resource page for …` step
(`resilience.steps.ts:658`) and correctly report **Pending** (that call site already
`return`s `'pending'`, matching the convention `genesis/a2o/CLAUDE.md` documents: "Guard:
`requirePlaywright(this)` returns null in non-Playwright mode (return `'pending'` to
skip)"). Only the viewport-archetype step at line 1064 broke that convention with
`assert.fail(NO_PW_DEVICE)` instead — the one inconsistent call site among ~20 in this
file, and the one this doc already named on 2026-07-03 as "a distinct
`assert.fail('No Playwright device found')` site, not `requirePwDevice` itself, but the
same root cause."

**Fix landed** (`genesis/a2o/steps/resilience.steps.ts:1059-1067`, commit below): changed

```ts
const device = findPwDevice(this);
if (!device) {
  assert.fail(NO_PW_DEVICE);
}
```
to
```ts
const device = findPwDevice(this);
if (!device) return 'pending';
```

— matching the identical guard five lines above in the same function family
(`resilience.steps.ts:662`) and the file's/suite's established convention. This is a
narrower, more durable fix than the 2026-07-03 tag-filter workaround: it patches the
actual inconsistent call site rather than routing around it for one job, so it also
protects any future non-browser run of this scenario without needing a second tag-filter
carve-out.

**Verified locally** (no live alpha/substrate available in this environment, so this is
static + structural verification, not a live run):
- `pnpm exec tsc --noEmit -p .` (genesis/a2o) — clean, no errors.
- `pnpm exec eslint steps/resilience.steps.ts` — clean, no errors/warnings.
- `pnpm exec prettier --check steps/resilience.steps.ts` — passes.
- `npx cucumber-js features/resilience/observable-distribution.feature --dry-run --name
  "Resilience hypercard stays fully inside a phone viewport"` — all 10 steps resolve
  (no undefined-step or parse errors); confirms the edit didn't break step-definition
  wiring or Gherkin matching.
- Runtime behavior is proven by the sibling call site's field evidence above (identical
  guard shape, already observed going cleanly to `Pending` in the same build's own log for
  scenarios #21/#22) rather than a fresh live run.

**Residual gap, not fixed here (named for the record, not sentinel scope):** even after
this fix, the "Resilience hypercard stays fully inside a phone viewport" scenario (and its
two siblings above) will go quietly **Pending** in every `elohim-genesis` browser run
forever — no step in this feature ever creates a Playwright device for any persona, so
none of the three actually exercises the phone-viewport/tooltip/hypercard rendering they
claim to guard. This matches this steps file's own header comment ("Browser-layer
assertions are @wip and exercised in Task 19 / Plan 5 chaos runs") — these three were
seemingly meant to be `@wip`-gated pending a persona-login wire-up that never landed, but
carry no `@wip` tag today. Two options, both a2o-authoring judgment calls (Opus scenario
work, not a mechanical fix): (a) tag the trio `@wip` to make the theater explicit, or
(b) author the missing `Given human "<persona>" is logged in on doorway "alpha" with
device` step so they actually run. Left for operator/next-shift.

**Ledger:** `5dba80d982e1` set `status: triaged`, `triaged_at_build: 1255` (the
fingerprint's `last_build` at triage time). No `decompose_on_confirm` stamp — this entry
carries an open residual (the persona-login gap above) that should stay visible in the
backlog even after the fingerprint itself disappears on a green streak, so the harvester
should NOT auto-decompose this backlog file on confirmation.
