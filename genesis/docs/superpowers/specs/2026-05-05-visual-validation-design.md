# Visual Validation for a2o Scenarios — Design

**Date:** 2026-05-05
**Status:** Draft (awaiting operator review)
**Scope:** `genesis/a2o/` — capture, tag vocabulary, sprint-report additions
**Out of scope:** Agent-assisted review loop (handed off to a future `/shift` skill update); reference-image storage; pixel-level drift detection.

## Why this exists

Today the a2o suite reports `failed: N` from `cucumber-report.json`. Logical assertions pass via DOM-text matching, status codes, etc. The gap: a scenario can pass logically while the visible delivery is broken, ugly, off-vision, hidden behind a flag, or never-rendered.

"Logs aren't complaining" ≠ "I can see this is delivering the experience the manifesto describes."

Visual validation makes the perceptual layer first-class alongside the logical one: every Playwright-mode scenario produces a screenshot, and a durable `@elohim-visually-validated` tag in git records that a human has reviewed the image and confirmed it matches the design and vision.

This pairs with the load-bearing principle that features must visibly deliver the human experience, not just log-pass against an internal contract.

## Six design decisions

| # | Question | Decision |
|---|---|---|
| 1 | Capture trigger | Every scenario in Playwright mode (passed *or* failed) |
| 2 | Naming and bundle layout | Per-feature subdirs: `reports/screenshots/{featureSlug}/{scenarioSlug}--{human}.png` |
| 3 | Tag vocabulary and lifecycle | Single tag `@elohim-visually-validated`; implicit pending; no CI write-back |
| 4 | Sprint-report additions | `summary.visualValidation` object **and** `visual-regression` finding source |
| 5 | Agent-assisted review | Deferred — handled by a future `/shift` update once artifacts are in place |
| 6 | Profile scope | Playwright-mode profiles only; HTTP-mode profiles unaffected |

## State model

A scenario is in one of four buckets per run, derived from { has tag } × { run outcome }:

| | passed | failed |
|---|---|---|
| **has `@elohim-visually-validated`** | validated-passing | **validated-regressed** ← highest-priority |
| **no tag** | pending-passing | pending-failing |

- **validated-passing** — the experience continues to work as a human confirmed it does.
- **validated-regressed** — a previously-confirmed experience broke. This is the load-bearing signal; surfaces as its own finding source so an operator/agent can drill in.
- **pending-passing** — backlog candidate for human review. Sprint backlog burndown picks from here.
- **pending-failing** — already represented in existing `scenario-failure` findings; visual-validation overlay just adds a screenshot link.

## Capture mechanism

### Playwright hook integration

The existing `After` hook in `genesis/a2o/steps/common.steps.ts` already captures `FAIL-{name}-{human}.png` on failure. Visual validation expands this to **every** scenario when `device instanceof PlaywrightDevice`, regardless of pass/fail.

New helper:

```typescript
async function captureVisualEvidence(
  device: PlaywrightDevice,
  scenarioId: string,   // featureSlug--scenarioSlug
  humanName: string,
  featureSlug: string,
  scenarioSlug: string,
): Promise<void> {
  const dir = `reports/screenshots/${featureSlug}`;
  mkdirSync(dir, { recursive: true });
  await device.screenshot(`${featureSlug}/${scenarioSlug}--${humanName}`);
}
```

`PlaywrightDevice.screenshot(name)` already supports relative paths under `reports/screenshots/`; the implementation in `src/framework/devices/playwright-device.ts` does `await this.page.screenshot({ path: 'reports/screenshots/' + name + '.png', fullPage: true })`. The signature is unchanged; only the caller passes a slash-containing relative path.

`featureSlug` and `scenarioSlug` are computed identically to the existing observation-report convention in the `Before` hook (already present in `common.steps.ts`):

```typescript
const featureSlug = scenario.pickle.uri.replace(/^.*features\//, '').replace(/\.feature$/, '').replace(/\//g, '-');
const scenarioSlug = scenario.pickle.name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
```

The slugs are stored on the `World` during `Before` so the `After` hook reuses them rather than recomputing.

### When capture runs

| Scenario outcome | Existing artifacts | New artifact |
|---|---|---|
| Passed | (none) | `reports/screenshots/{featureSlug}/{scenarioSlug}--{human}.png` |
| Failed | console errors JSON, trace, `FAIL-…` screenshot | `reports/screenshots/{featureSlug}/{scenarioSlug}--{human}.png` (replaces `FAIL-…` naming) + sibling `.error.json` |
| Skipped/pending | (none) | (none) |

The status-prefixed `FAIL-` filename is retired **for the screenshot only**. Failure-only artifacts (console errors JSON, Playwright trace) keep their existing `FAIL-…` naming under `reports/console/` and `reports/traces/` respectively — they don't need to be uniform with the screenshot path because they only exist on failure. Failure context that pairs with the screenshot lives in a sibling `{scenarioSlug}--{human}.error.json` in the same feature subdir:

```json
{
  "status": "failed",
  "failureMessage": "...first line of step error...",
  "failedStep": "When I start the \"Elohim Protocol\" path"
}
```

This keeps the screenshot path deterministic regardless of outcome, so a sprint-report row can link to it without consulting the run status first.

### Profile scope

Capture is gated by `deviceMode === 'playwright'`. The `alpha` and `local` profiles (HTTP-only) are unaffected and produce no screenshots. The `browser`, `delivery-browser`, and any future Playwright-mode profile (defined by `worldParameters.deviceMode === 'playwright'` in `cucumber.mjs`) capture every scenario.

## Tag vocabulary and lifecycle

Single tag: `@elohim-visually-validated`.

- **Authoring** — the tag is added manually in git by a human reviewer (or by an agent in a follow-up sprint that produces a PR for human approval).
- **Effect at runtime** — none. The tag is purely metadata; capture and assertion behaviour are identical for tagged and untagged scenarios.
- **Effect in sprint-report** — see [Sprint-report additions](#sprint-report-additions).
- **Invalidation** — the tag stays in git until a human removes it. A regression (tagged scenario fails) does not auto-remove; it surfaces as a `visual-regression` finding so the human (or agent) can decide:
  - Bug → fix the bug; tag stays valid.
  - Intentional redesign → remove the tag in the same commit as the redesign; reviewer re-validates against the new image after merge.
- **Backlog burndown** — `pending-passing` scenarios are the candidates for review. The `/shift` skill (in a follow-up update) iterates these and proposes tag additions in PRs.

The tag is an additive opt-in. Existing scenarios start as pending. The migration is non-breaking: zero feature-file edits required to land this design.

## Sprint-report additions

### 1. `loadCucumber` retains tags

`genesis/a2o/scripts/lib/load-cucumber.ts` currently extracts `{ name, feature, status, failureMessage }` per scenario. Add `tags: string[]`:

```typescript
export interface ScenarioResult {
  name: string;
  feature: string;
  status: ScenarioStatus;
  failureMessage?: string;
  tags: string[];   // NEW — names of tags on the scenario, e.g. ['@e2e', '@lamad', '@browser-only', '@elohim-visually-validated']
}
```

The cucumber JSON already carries `element.tags[].name`; load-cucumber maps them through.

### 2. Aggregator computes `summary.visualValidation`

`genesis/a2o/scripts/lib/aggregate.ts` adds a `summary.visualValidation` object **only when the profile ran in Playwright mode**:

```typescript
summary: {
  scenarios: { ... },                     // unchanged
  findings: { ... },                      // unchanged
  visualValidation?: {                    // NEW — present only when deviceMode=playwright
    validatedPassing: number,
    validatedRegressed: number,
    pendingPassing: number,
    pendingFailing: number,
  },
}
```

Detection of Playwright mode: the aggregator checks the `profile` argument against an allowlist (`browser`, `delivery-browser`, plus any profile whose `cucumber.mjs` definition sets `worldParameters.deviceMode === 'playwright'`). HTTP-only profiles omit the field entirely.

Counts are computed by joining `ScenarioResult.status` and `ScenarioResult.tags.includes('@elohim-visually-validated')`.

### 3. New finding source `visual-regression`

A scenario with `@elohim-visually-validated` AND `status === 'failed'` produces a `Finding` with:

- `source: 'visual-regression'`
- `severity: 'error'`
- `pillar`: derived via existing `pillarFromFeature`
- `message`: `Visually-validated scenario regressed: {scenario name}`
- `screenshotPath`: relative path to the captured image (see below)
- `suggestedObjective`: `Restore visual delivery of: {scenario name}` — these are top-priority because a previously-confirmed experience broke.

Existing `scenario-failure` findings also gain `screenshotPath` populated when a Playwright image exists, so reviewers can click through to see what the user saw at failure.

### 4. Schema changes

`genesis/a2o/schemas/sprint-report.schema.json` updates:

- Add `summary.visualValidation` (optional object, four required integer fields, `additionalProperties: false`).
- Extend `findings.bySource` to allow `visual-regression` (it's an open `additionalProperties` map already, so no schema change needed for the count; but the enum on `Finding.source` needs the new value).
- Add `visual-regression` to the `Finding.source` enum.
- Add optional `screenshotPath: { type: 'string' }` to `Finding`.

### 5. Markdown layout

`genesis/a2o/scripts/lib/render-markdown.ts` gains a new section above the per-pillar findings (only when `summary.visualValidation` is present):

```markdown
## Visual Validation

| | passed | failed |
|---|---|---|
| has `@elohim-visually-validated` | 12 | **3** |
| no tag (pending) | 47 | 18 |

- 12 validated-passing — confirmed delivering as designed
- **3 validated-regressed** — see findings below
- 47 pending-passing — candidates for review
- 18 pending-failing — see scenario-failure findings
```

Per-finding rendering for `visual-regression` and `scenario-failure` includes the screenshot link:

```markdown
### [visual-regression] `<fingerprint>` (occurrences: 1)

> Visually-validated scenario regressed: Starting a Journey

- **Screenshot**: `reports/screenshots/lamad-learning-journey/starting-a-journey--Matthew.png`
- **Objective**: Restore visual delivery of: Starting a Journey
```

In Jenkins, `archiveArtifacts artifacts: 'reports/**'` makes the path clickable through Jenkins's archived-artifacts browser.

## Files affected

| File | Change |
|---|---|
| `genesis/a2o/steps/common.steps.ts` | New `captureVisualEvidence` helper; `Before` hook stores `featureSlug`/`scenarioSlug` on `world`; `After` hook calls capture for every Playwright scenario; failure-mode writes sibling `.error.json` instead of `FAIL-…` filename |
| `genesis/a2o/src/framework/world.ts` | Add `featureSlug?: string`, `scenarioSlug?: string` properties to `E2EWorld` for hook-to-hook handoff |
| `genesis/a2o/scripts/lib/load-cucumber.ts` | `ScenarioResult` gains `tags: string[]`; map from `element.tags[].name` |
| `genesis/a2o/scripts/lib/aggregate.ts` | Compute `summary.visualValidation`; emit `visual-regression` raw findings; populate `screenshotPath` on findings; profile allowlist for Playwright detection |
| `genesis/a2o/scripts/lib/render-markdown.ts` | New `## Visual Validation` section; screenshot links in finding rows |
| `genesis/a2o/schemas/sprint-report.schema.json` | New `summary.visualValidation` object; `visual-regression` in `Finding.source` enum; optional `Finding.screenshotPath` |

No feature-file changes required to land. No CI Jenkinsfile changes required (artifact archival already covers `reports/**`).

## Testing

| Concern | Test |
|---|---|
| `loadCucumber` extracts tags | Unit test against a fixture cucumber JSON with multiple tags per scenario |
| Aggregator computes 2×2 buckets correctly | Unit test with synthetic `ScenarioResult[]` covering all four cells |
| Aggregator omits `visualValidation` for non-Playwright profiles | Unit test with `profile: 'alpha'` |
| `visual-regression` findings produced only when tag + failed | Unit test |
| `screenshotPath` population for both `visual-regression` and `scenario-failure` | Unit test |
| Schema validation passes for both shapes (with and without `visualValidation`) | Existing schema-validate gate in `build-sprint-report.ts` |
| Capture hook runs in Playwright mode and writes to subdirs | Integration smoke against the `browser` profile |

The first five all live in `genesis/a2o/scripts/__tests__/`; the last needs a Playwright run, validated locally then in CI.

## Migration plan

1. **Land schema, aggregator, render changes** — additive; existing reports continue to validate (visualValidation is optional).
2. **Land hook capture changes** — additive; non-Playwright profiles unaffected.
3. **First Playwright run after merge** — every browser-mode scenario captures; sprint-report shows all scenarios as `pending-*`.
4. **First human review pass** — operator browses Jenkins archived screenshots, adds `@elohim-visually-validated` to feature files for scenarios where the image matches the vision, opens a PR.
5. **Steady state** — pending-passing list shrinks over time; validated-regressed becomes the fastest signal for regression backlog; future `/shift` update automates step 4 with PR proposals.

No flag day, no migration script, no feature-file rewrite.

## Open questions deferred to follow-ups

- **Agent-assisted review** — `/shift` skill update; needs its own design conversation about prompt template, reference manifesto excerpts per pillar, PR-proposal flow.
- **Multi-human scenarios** — when a scenario registers multiple humans (e.g., Matthew + Terrance), each human's device captures its own image. The 2×2 counts treat the scenario as one row; the per-finding render shows all images. No design change needed; documenting the behavior.
- **Reference-image storage** — explicitly out of scope. If pixel-level drift detection becomes desirable later, reference images can be added without breaking this design (extend `Finding.screenshotPath` to a path pair).
- **Storybook integration** — Storybook lives in the `ethosengine` namespace (per memory `project_storybook_in_ethosengine_namespace`); component-level visual validation there is a separate concern from a2o scenario-level perceptual validation.
