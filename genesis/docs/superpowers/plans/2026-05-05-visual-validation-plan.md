# Visual Validation for a2o Scenarios — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the perceptual layer first-class in a2o by capturing screenshots of every Playwright-mode scenario, surfacing a 2×2 validation matrix in the sprint-report, and emitting a `visual-regression` finding when an `@elohim-visually-validated` scenario fails.

**Architecture:** Six small additive changes:
1. Cucumber `After` hook captures every Playwright scenario (not just failures) into per-feature subdirs
2. `loadCucumber` retains `tags: string[]` from the cucumber JSON
3. `aggregate` computes `summary.visualValidation` (4 buckets) when run in Playwright mode
4. `aggregate` emits a new `visual-regression` finding source for tagged-and-failed scenarios
5. `render-markdown` adds a `## Visual Validation` section and screenshot links on findings
6. Schema additions (additive) keep `build-sprint-report.ts`'s AJV gate green

**Tech Stack:** TypeScript (strict), Cucumber.js 11, Playwright 1.50, AJV 8 (draft 2020-12), `node:test` runner via `tsx --test`. All work lives in `genesis/a2o/`.

**Spec:** `genesis/docs/superpowers/specs/2026-05-05-visual-validation-design.md`

**Test command (run from `genesis/a2o/`):** `pnpm test:unit`

---

## File Map

| Path | Change |
|---|---|
| `genesis/a2o/scripts/__tests__/fixtures/cucumber-mixed.json` | Add `tags` arrays to existing scenario elements |
| `genesis/a2o/scripts/lib/load-cucumber.ts` | `ScenarioResult` gains required `tags: string[]`; `loadCucumber` maps `element.tags[].name` |
| `genesis/a2o/scripts/__tests__/load-cucumber.test.ts` | Add tag-extraction assertions |
| `genesis/a2o/scripts/lib/aggregate.ts` | New `visualValidation` summary, new `visual-regression` source, optional `screenshotPath` on `Finding`, profile allowlist |
| `genesis/a2o/scripts/__tests__/aggregate.test.ts` | Add `tags: []` to existing literals; new tests for 4-bucket counts, profile gating, regression source, screenshot path |
| `genesis/a2o/scripts/lib/render-markdown.ts` | New `## Visual Validation` section; screenshot links in finding rows |
| `genesis/a2o/scripts/__tests__/render-markdown.test.ts` | New tests for the section and links |
| `genesis/a2o/schemas/sprint-report.schema.json` | Optional `summary.visualValidation`; `visual-regression` in `Finding.source` enum; optional `Finding.screenshotPath` |
| `genesis/a2o/src/framework/world.ts` | Optional `featureSlug?: string`, `scenarioSlug?: string` properties on `E2EWorld` |
| `genesis/a2o/steps/common.steps.ts` | New `captureVisualEvidence` helper; `Before` stashes slugs on world; `After` captures every Playwright scenario; failure writes sibling `.error.json` |

No feature-file edits. No Jenkinsfile edits. No package.json edits.

---

## Task 1: Add `tags` to `ScenarioResult` and extract them in `loadCucumber`

**Files:**
- Modify: `genesis/a2o/scripts/__tests__/fixtures/cucumber-mixed.json`
- Modify: `genesis/a2o/scripts/__tests__/load-cucumber.test.ts`
- Modify: `genesis/a2o/scripts/lib/load-cucumber.ts`
- Modify: `genesis/a2o/scripts/__tests__/aggregate.test.ts` (add `tags: []` to literals so TS still compiles)

- [ ] **Step 1: Update fixture to carry tags on every scenario element**

Replace the entire contents of `genesis/a2o/scripts/__tests__/fixtures/cucumber-mixed.json` with:

```json
[
  {
    "uri": "features/lamad/learning-journey.feature",
    "name": "Learning Journey",
    "elements": [
      {
        "name": "Terrance completes path",
        "type": "scenario",
        "tags": [
          { "name": "@e2e" },
          { "name": "@lamad" },
          { "name": "@browser-only" },
          { "name": "@elohim-visually-validated" }
        ],
        "steps": [
          { "name": "doorway is reachable", "result": { "status": "passed", "duration": 100 } }
        ]
      },
      {
        "name": "Mary fails on assessment",
        "type": "scenario",
        "tags": [
          { "name": "@e2e" },
          { "name": "@lamad" },
          { "name": "@browser-only" },
          { "name": "@elohim-visually-validated" }
        ],
        "steps": [
          { "name": "doorway is reachable", "result": { "status": "passed", "duration": 50 } },
          {
            "name": "answer is scored",
            "result": {
              "status": "failed",
              "duration": 200,
              "error_message": "AssertionError: expected 500 to be 200\n    at Object.<anonymous>"
            }
          }
        ]
      }
    ]
  },
  {
    "uri": "features/auth/fixture-humans.feature",
    "name": "Fixture Humans",
    "elements": [
      {
        "name": "Stub step not implemented",
        "type": "scenario",
        "tags": [
          { "name": "@e2e" },
          { "name": "@auth" }
        ],
        "steps": [
          { "name": "a new human wanders in", "result": { "status": "pending", "duration": 0 } }
        ]
      }
    ]
  }
]
```

The first scenario is now a tagged-and-passed (validated-passing). The second is tagged-and-failed (validated-regressed). The third is untagged-and-pending. This single fixture covers the four-bucket aggregator tests in Task 3 as well as the tag-extraction tests here.

- [ ] **Step 2: Write the failing tag-extraction test**

Append these `it` blocks at the end of the existing `describe` in `genesis/a2o/scripts/__tests__/load-cucumber.test.ts` (just before the final `});`):

```typescript
  void it('extracts scenario tags', () => {
    const results = loadCucumber(fixture);
    const terrance = results.find(r => r.name === 'Terrance completes path')!;
    assert.ok(terrance);
    assert.deepEqual(terrance.tags, ['@e2e', '@lamad', '@browser-only', '@elohim-visually-validated']);
  });

  void it('returns empty tags array when scenario has no tags', () => {
    const noTagsJson = JSON.stringify([
      {
        uri: 'features/x.feature',
        name: 'X',
        elements: [
          {
            name: 'untagged scenario',
            type: 'scenario',
            steps: [{ name: 's', result: { status: 'passed' } }],
          },
        ],
      },
    ]);
    const [scenario] = loadCucumber(noTagsJson);
    assert.deepEqual(scenario.tags, []);
  });
```

- [ ] **Step 3: Run tests to verify failure**

Run from `/projects/elohim/genesis/a2o`:
```bash
pnpm test:unit -- --test-name-pattern='loadCucumber'
```

Expected: FAIL — TypeScript will complain that `tags` does not exist on `ScenarioResult`, or assertion fails saying `terrance.tags` is undefined.

- [ ] **Step 4: Implement tag extraction in `load-cucumber.ts`**

Replace the entire contents of `genesis/a2o/scripts/lib/load-cucumber.ts` with:

```typescript
export type ScenarioStatus = 'passed' | 'failed' | 'skipped' | 'pending' | 'undefined';

export interface ScenarioResult {
  name: string;
  feature: string;
  status: ScenarioStatus;
  failureMessage?: string;
  tags: string[];
}

interface CucumberStep {
  name: string;
  result?: { status: string; duration?: number; error_message?: string };
}

interface CucumberTag {
  name: string;
}

interface CucumberElement {
  name: string;
  type: string;
  steps?: CucumberStep[];
  tags?: CucumberTag[];
}

interface CucumberFeature {
  uri: string;
  name: string;
  elements?: CucumberElement[];
}

const STATUS_PRIORITY: ScenarioStatus[] = ['failed', 'undefined', 'pending', 'skipped', 'passed'];

function aggregateStatus(steps: CucumberStep[]): ScenarioStatus {
  const seen = new Set(steps.map(s => (s.result?.status ?? 'undefined') as ScenarioStatus));
  for (const s of STATUS_PRIORITY) if (seen.has(s)) return s;
  return 'passed';
}

export function loadCucumber(json: string): ScenarioResult[] {
  const features = JSON.parse(json) as CucumberFeature[];
  const results: ScenarioResult[] = [];
  for (const feature of features) {
    for (const el of feature.elements ?? []) {
      if (el.type !== 'scenario') continue;
      const steps = el.steps ?? [];
      const status = aggregateStatus(steps);
      const failed = steps.find(s => s.result?.status === 'failed');
      const tags = (el.tags ?? []).map(t => t.name);
      results.push({
        name: el.name,
        feature: feature.uri,
        status,
        failureMessage: failed?.result?.error_message,
        tags,
      });
    }
  }
  return results;
}
```

- [ ] **Step 5: Add `tags: []` to existing `ScenarioResult` literals in aggregate tests**

In `genesis/a2o/scripts/__tests__/aggregate.test.ts`, locate the `input()` function (around line 10). Update the three scenario literals so each includes `tags: []`. The function should look like:

```typescript
function input() {
  const scenarios: ScenarioResult[] = [
    {
      name: 'Terrance completes path',
      feature: 'features/lamad/learning-journey.feature',
      status: 'passed',
      tags: [],
    },
    {
      name: 'Mary fails on assessment',
      feature: 'features/lamad/learning-journey.feature',
      status: 'failed',
      failureMessage: 'AssertionError: expected 500 to be 200',
      tags: [],
    },
    {
      name: 'Stub not implemented',
      feature: 'features/auth/fixture-humans.feature',
      status: 'pending',
      tags: [],
    },
  ];
  // ... rest unchanged
```

- [ ] **Step 6: Run unit tests and typecheck to verify green**

Run from `/projects/elohim/genesis/a2o`:
```bash
pnpm test:unit
pnpm typecheck
```

Expected: All tests pass. Typecheck succeeds with no errors.

- [ ] **Step 7: Commit**

```bash
git add genesis/a2o/scripts/lib/load-cucumber.ts \
        genesis/a2o/scripts/__tests__/load-cucumber.test.ts \
        genesis/a2o/scripts/__tests__/aggregate.test.ts \
        genesis/a2o/scripts/__tests__/fixtures/cucumber-mixed.json
git -c commit.gpgsign=false commit -m "feat(a2o): retain scenario tags in loadCucumber

ScenarioResult gains a required tags string[] field populated from
cucumber JSON element.tags[].name. Foundation for visual validation
(@elohim-visually-validated) detection in the aggregator.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: Schema additions for `visualValidation`, `visual-regression`, and `screenshotPath`

**Files:**
- Modify: `genesis/a2o/schemas/sprint-report.schema.json`

There's no isolated unit test for the schema; it's exercised by `build-sprint-report.ts` and by Task 3/4 aggregator tests that emit reports validating against this schema. We update the schema first so subsequent tests can validate end-to-end against it.

- [ ] **Step 1: Add `visualValidation` to `summary` properties**

In `genesis/a2o/schemas/sprint-report.schema.json`, find the `summary` object definition (around line 17). It currently has `properties: { scenarios, findings }` and `required: ["scenarios", "findings"]`. Add `visualValidation` as an optional property by extending `properties` only — leave `required` unchanged.

Replace the `summary` block (lines 17–50) with:

```json
    "summary": {
      "type": "object",
      "required": ["scenarios", "findings"],
      "additionalProperties": false,
      "properties": {
        "scenarios": {
          "type": "object",
          "required": ["total", "passed", "failed", "skipped", "pending"],
          "additionalProperties": false,
          "properties": {
            "total": { "type": "integer", "minimum": 0 },
            "passed": { "type": "integer", "minimum": 0 },
            "failed": { "type": "integer", "minimum": 0 },
            "skipped": { "type": "integer", "minimum": 0 },
            "pending": { "type": "integer", "minimum": 0 }
          }
        },
        "findings": {
          "type": "object",
          "required": ["total", "bySource", "byPillar"],
          "additionalProperties": false,
          "properties": {
            "total": { "type": "integer", "minimum": 0 },
            "bySource": {
              "type": "object",
              "additionalProperties": { "type": "integer", "minimum": 0 }
            },
            "byPillar": {
              "type": "object",
              "additionalProperties": { "type": "integer", "minimum": 0 }
            }
          }
        },
        "visualValidation": {
          "type": "object",
          "description": "Present only when the run was in Playwright/browser mode. Counts the four buckets formed by { has @elohim-visually-validated tag } x { passed | failed }.",
          "required": ["validatedPassing", "validatedRegressed", "pendingPassing", "pendingFailing"],
          "additionalProperties": false,
          "properties": {
            "validatedPassing": { "type": "integer", "minimum": 0 },
            "validatedRegressed": { "type": "integer", "minimum": 0 },
            "pendingPassing": { "type": "integer", "minimum": 0 },
            "pendingFailing": { "type": "integer", "minimum": 0 }
          }
        }
      }
    },
```

- [ ] **Step 2: Add `visual-regression` to the `Finding.source` enum and `screenshotPath` to `Finding`**

In the same file, find `$defs.Finding.properties.source` (around line 67). Add `"visual-regression"` to the enum array. Then add an optional `screenshotPath` property to `Finding.properties`.

Replace the `Finding` block (lines 58–109) with:

```json
    "Finding": {
      "type": "object",
      "required": ["fingerprint", "source", "pillar", "message", "occurrences", "scenarios"],
      "additionalProperties": false,
      "properties": {
        "fingerprint": {
          "type": "string",
          "description": "Stable hash over the normalized message"
        },
        "source": {
          "type": "string",
          "enum": [
            "console-error",
            "page-error",
            "failed-request",
            "scenario-failure",
            "pending-step",
            "coverage-gap",
            "visual-regression"
          ]
        },
        "pillar": {
          "type": "string",
          "description": "lamad | imagodei | elohim | federation | delivery | browser | content | deployment | qahal | shefa | unknown"
        },
        "peer": {
          "type": "string",
          "description": "Target-peer slug (e.g., 'terrance-household', 'shem'). Populated by Plan B once request-ID/peer routing is live; absent for local-only runs."
        },
        "severity": { "type": "string", "enum": ["error", "warning", "info"], "default": "error" },
        "message": { "type": "string" },
        "firstSeenUrl": { "type": "string", "format": "uri" },
        "screenshotPath": {
          "type": "string",
          "description": "Relative path to the captured screenshot (e.g., reports/screenshots/lamad-learning-journey/starting-a-journey--Matthew.png). Populated for visual-regression and for scenario-failure when an image was captured."
        },
        "occurrences": { "type": "integer", "minimum": 1 },
        "scenarios": {
          "type": "array",
          "minItems": 1,
          "items": {
            "type": "object",
            "required": ["name", "feature"],
            "additionalProperties": false,
            "properties": {
              "name": { "type": "string" },
              "feature": { "type": "string" },
              "human": { "type": "string" }
            }
          }
        },
        "suggestedObjective": {
          "type": "string",
          "description": "One-line headline for /shift Objective seeding"
        }
      }
    }
```

- [ ] **Step 3: Verify schema parses**

Run from `/projects/elohim/genesis/a2o`:
```bash
node -e "const s = require('./schemas/sprint-report.schema.json'); console.log('ok', Object.keys(s.\$defs));"
```

Expected: `ok [ 'Finding' ]`

- [ ] **Step 4: Verify existing aggregate tests still produce schema-valid reports**

```bash
pnpm test:unit
```

Expected: All tests pass. (Schema is additive — old reports still validate.)

- [ ] **Step 5: Commit**

```bash
git add genesis/a2o/schemas/sprint-report.schema.json
git -c commit.gpgsign=false commit -m "feat(a2o): schema additions for visual validation

- summary.visualValidation (optional 4-bucket counts)
- visual-regression added to Finding.source enum
- Finding.screenshotPath optional field

Additive only; existing reports remain valid.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: Aggregator computes `summary.visualValidation` (4 buckets) gated on Playwright profile

**Files:**
- Modify: `genesis/a2o/scripts/lib/aggregate.ts`
- Modify: `genesis/a2o/scripts/__tests__/aggregate.test.ts`

- [ ] **Step 1: Write failing test for visualValidation gating and counts**

Append these tests at the end of the existing `describe('aggregate', () => { ... })` block in `aggregate.test.ts`, just before the closing `});`:

```typescript
  function visualScenarios(): ScenarioResult[] {
    return [
      // validated-passing
      {
        name: 'Validated and passing',
        feature: 'features/lamad/a.feature',
        status: 'passed',
        tags: ['@e2e', '@elohim-visually-validated'],
      },
      // validated-regressed
      {
        name: 'Validated but failed',
        feature: 'features/lamad/b.feature',
        status: 'failed',
        failureMessage: 'AssertionError: visual element missing',
        tags: ['@e2e', '@elohim-visually-validated'],
      },
      // pending-passing
      {
        name: 'Untagged passing',
        feature: 'features/lamad/c.feature',
        status: 'passed',
        tags: ['@e2e'],
      },
      // pending-failing
      {
        name: 'Untagged failed',
        feature: 'features/lamad/d.feature',
        status: 'failed',
        failureMessage: 'AssertionError: nope',
        tags: ['@e2e'],
      },
    ];
  }

  void it('emits summary.visualValidation when profile is browser', () => {
    const r = aggregate({
      scenarios: visualScenarios(),
      consoleArtifacts: [],
      gaps: [],
      runId: 'r1',
      profile: 'browser',
    });
    assert.ok(r.summary.visualValidation, 'visualValidation should be present');
    assert.equal(r.summary.visualValidation!.validatedPassing, 1);
    assert.equal(r.summary.visualValidation!.validatedRegressed, 1);
    assert.equal(r.summary.visualValidation!.pendingPassing, 1);
    assert.equal(r.summary.visualValidation!.pendingFailing, 1);
  });

  void it('emits summary.visualValidation when profile is delivery-browser', () => {
    const r = aggregate({
      scenarios: visualScenarios(),
      consoleArtifacts: [],
      gaps: [],
      runId: 'r1',
      profile: 'delivery-browser',
    });
    assert.ok(r.summary.visualValidation);
  });

  void it('omits summary.visualValidation when profile is alpha', () => {
    const r = aggregate({
      scenarios: visualScenarios(),
      consoleArtifacts: [],
      gaps: [],
      runId: 'r1',
      profile: 'alpha',
    });
    assert.equal(r.summary.visualValidation, undefined);
  });

  void it('omits summary.visualValidation when profile is local', () => {
    const r = aggregate({
      scenarios: visualScenarios(),
      consoleArtifacts: [],
      gaps: [],
      runId: 'r1',
      profile: 'local',
    });
    assert.equal(r.summary.visualValidation, undefined);
  });
```

- [ ] **Step 2: Run tests to verify failure**

Run from `/projects/elohim/genesis/a2o`:
```bash
pnpm test:unit -- --test-name-pattern='visualValidation'
```

Expected: FAIL — `r.summary.visualValidation` is undefined when it should be defined; properties don't exist on the type.

- [ ] **Step 3: Implement profile-gated `visualValidation` summary**

In `genesis/a2o/scripts/lib/aggregate.ts`:

(a) Add the constant and helper at the top of the file (after the imports, before `export type FindingSource`):

```typescript
const VISUAL_VALIDATION_TAG = '@elohim-visually-validated';

const PLAYWRIGHT_PROFILES = new Set(['browser', 'delivery-browser']);

function isPlaywrightProfile(profile: string): boolean {
  return PLAYWRIGHT_PROFILES.has(profile);
}
```

(b) Update the `SprintReport.summary` type to include `visualValidation`. Replace the existing `summary:` block in the `SprintReport` interface (around line 39) with:

```typescript
  summary: {
    scenarios: { total: number; passed: number; failed: number; skipped: number; pending: number };
    findings: { total: number; bySource: Record<string, number>; byPillar: Record<string, number> };
    visualValidation?: {
      validatedPassing: number;
      validatedRegressed: number;
      pendingPassing: number;
      pendingFailing: number;
    };
  };
```

(c) Add a helper function below `computeSummary`:

```typescript
function computeVisualValidation(
  scenarios: ScenarioResult[]
): NonNullable<SprintReport['summary']['visualValidation']> {
  const counts = {
    validatedPassing: 0,
    validatedRegressed: 0,
    pendingPassing: 0,
    pendingFailing: 0,
  };
  for (const s of scenarios) {
    const validated = s.tags.includes(VISUAL_VALIDATION_TAG);
    if (validated && s.status === 'passed') counts.validatedPassing += 1;
    else if (validated && s.status === 'failed') counts.validatedRegressed += 1;
    else if (!validated && s.status === 'passed') counts.pendingPassing += 1;
    else if (!validated && s.status === 'failed') counts.pendingFailing += 1;
  }
  return counts;
}
```

(d) Update the bottom `aggregate` function to attach the field conditionally. Replace the final `return` block (around line 207) with:

```typescript
export function aggregate(input: AggregateInput): SprintReport {
  const raws = [
    ...buildScenarioRaws(input.scenarios),
    ...buildConsoleRaws(input.consoleArtifacts),
    ...buildGapRaws(input.gaps),
  ];

  const findings = groupIntoFindings(raws);
  const summary = computeSummary(input.scenarios, findings);

  if (isPlaywrightProfile(input.profile)) {
    summary.visualValidation = computeVisualValidation(input.scenarios);
  }

  return {
    generatedAt: new Date().toISOString(),
    runId: input.runId,
    profile: input.profile,
    doorway: input.doorway,
    summary,
    findings,
  };
}
```

- [ ] **Step 4: Run tests to verify pass**

Run from `/projects/elohim/genesis/a2o`:
```bash
pnpm test:unit
pnpm typecheck
```

Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add genesis/a2o/scripts/lib/aggregate.ts \
        genesis/a2o/scripts/__tests__/aggregate.test.ts
git -c commit.gpgsign=false commit -m "feat(a2o): compute visualValidation 2x2 buckets in aggregator

Gated on Playwright profile (browser, delivery-browser). Counts
{validated, pending} x {passing, regressed} from the
@elohim-visually-validated tag joined with scenario status.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: Aggregator emits `visual-regression` findings and `screenshotPath`

**Files:**
- Modify: `genesis/a2o/scripts/lib/aggregate.ts`
- Modify: `genesis/a2o/scripts/__tests__/aggregate.test.ts`

- [ ] **Step 1: Write failing tests for visual-regression source and screenshotPath**

Append at the end of the existing `describe('aggregate', () => { ... })` block in `aggregate.test.ts`, just before the closing `});`:

```typescript
  void it('emits a visual-regression finding for tagged-and-failed scenarios in playwright mode', () => {
    const r = aggregate({
      scenarios: visualScenarios(),
      consoleArtifacts: [],
      gaps: [],
      runId: 'r1',
      profile: 'browser',
    });
    const regression = r.findings.find(f => f.source === 'visual-regression');
    assert.ok(regression, 'expected a visual-regression finding');
    assert.equal(regression.scenarios.length, 1);
    assert.equal(regression.scenarios[0].name, 'Validated but failed');
    assert.equal(regression.severity, 'error');
    assert.match(regression.message, /Validated but failed/);
    assert.match(regression.suggestedObjective, /Restore visual delivery/);
  });

  void it('does not emit visual-regression for tagged-and-failed scenarios outside playwright mode', () => {
    const r = aggregate({
      scenarios: visualScenarios(),
      consoleArtifacts: [],
      gaps: [],
      runId: 'r1',
      profile: 'alpha',
    });
    const regression = r.findings.find(f => f.source === 'visual-regression');
    assert.equal(regression, undefined);
  });

  void it('populates screenshotPath on visual-regression findings', () => {
    const r = aggregate({
      scenarios: visualScenarios(),
      consoleArtifacts: [],
      gaps: [],
      runId: 'r1',
      profile: 'browser',
    });
    const regression = r.findings.find(f => f.source === 'visual-regression')!;
    assert.ok(regression.screenshotPath);
    assert.match(regression.screenshotPath!, /^reports\/screenshots\/lamad-b\//);
    assert.match(regression.screenshotPath!, /\.png$/);
  });

  void it('populates screenshotPath on scenario-failure findings in playwright mode', () => {
    const r = aggregate({
      scenarios: visualScenarios(),
      consoleArtifacts: [],
      gaps: [],
      runId: 'r1',
      profile: 'browser',
    });
    const failure = r.findings.find(
      f => f.source === 'scenario-failure' && f.message.includes('nope')
    );
    assert.ok(failure);
    assert.ok(failure.screenshotPath);
    assert.match(failure.screenshotPath!, /^reports\/screenshots\/lamad-d\//);
  });

  void it('omits screenshotPath on scenario-failure findings outside playwright mode', () => {
    const r = aggregate({
      scenarios: visualScenarios(),
      consoleArtifacts: [],
      gaps: [],
      runId: 'r1',
      profile: 'alpha',
    });
    const failure = r.findings.find(f => f.source === 'scenario-failure')!;
    assert.equal(failure.screenshotPath, undefined);
  });
```

- [ ] **Step 2: Run tests to verify failure**

```bash
pnpm test:unit -- --test-name-pattern='visual-regression|screenshotPath'
```

Expected: FAIL — finding source `visual-regression` doesn't exist; type doesn't have `screenshotPath`.

- [ ] **Step 3: Add `visual-regression` to FindingSource and `screenshotPath` to Finding**

In `genesis/a2o/scripts/lib/aggregate.ts`:

(a) Replace the `FindingSource` type (around line 8) with:

```typescript
export type FindingSource =
  | 'console-error'
  | 'page-error'
  | 'failed-request'
  | 'scenario-failure'
  | 'pending-step'
  | 'coverage-gap'
  | 'visual-regression';
```

(b) Add `screenshotPath?: string;` to the `Finding` interface. Replace the existing `Finding` interface (around line 22) with:

```typescript
export interface Finding {
  fingerprint: string;
  source: FindingSource;
  pillar: string;
  severity: 'error' | 'warning' | 'info';
  message: string;
  firstSeenUrl?: string;
  screenshotPath?: string;
  occurrences: number;
  scenarios: FindingScenario[];
  suggestedObjective: string;
}
```

(c) Update `RawFinding` to carry an optional screenshot path. Replace the `RawFinding` interface (around line 55) with:

```typescript
interface RawFinding {
  source: FindingSource;
  pillar: string;
  severity: 'error' | 'warning' | 'info';
  rawMessage: string;
  url?: string;
  screenshotPath?: string;
  scenario: FindingScenario;
}
```

(d) Add a `case 'visual-regression':` arm to `suggestObjective`. Replace the `suggestObjective` function (around line 64) with:

```typescript
function suggestObjective(source: FindingSource, message: string): string {
  const head = message.slice(0, 120);
  switch (source) {
    case 'console-error':
      return `Fix browser console error: ${head}`;
    case 'page-error':
      return `Fix unhandled page exception: ${head}`;
    case 'failed-request':
      return `Fix failing network request: ${head}`;
    case 'scenario-failure':
      return `Fix scenario failure: ${head}`;
    case 'pending-step':
      return `Implement pending step definition: ${head}`;
    case 'coverage-gap':
      return `Author missing scenario: ${head}`;
    case 'visual-regression':
      return `Restore visual delivery of: ${head}`;
  }
}
```

(e) Add a screenshot-path helper near the top (after `isPlaywrightProfile`):

```typescript
function screenshotPathFor(scenario: ScenarioResult, human?: string): string {
  const featureSlug = scenario.feature
    .replace(/^.*features\//, '')
    .replace(/\.feature$/, '')
    .replace(/\//g, '-');
  const scenarioSlug = scenario.name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');
  const suffix = human ? `--${human}` : '';
  return `reports/screenshots/${featureSlug}/${scenarioSlug}${suffix}.png`;
}
```

(f) Update `buildScenarioRaws` to populate `screenshotPath` and emit visual-regression raws. Replace the existing `buildScenarioRaws` function (around line 82) with:

```typescript
function buildScenarioRaws(
  scenarios: ScenarioResult[],
  emitVisual: boolean
): RawFinding[] {
  const raws: RawFinding[] = [];
  for (const s of scenarios) {
    const validated = s.tags.includes(VISUAL_VALIDATION_TAG);
    if (s.status === 'failed' && s.failureMessage) {
      raws.push({
        source: 'scenario-failure',
        pillar: pillarFromFeature(s.feature),
        severity: 'error',
        rawMessage: s.failureMessage.split('\n')[0],
        screenshotPath: emitVisual ? screenshotPathFor(s) : undefined,
        scenario: { name: s.name, feature: s.feature },
      });
      if (emitVisual && validated) {
        raws.push({
          source: 'visual-regression',
          pillar: pillarFromFeature(s.feature),
          severity: 'error',
          rawMessage: `Validated visual regressed: ${s.name}`,
          screenshotPath: screenshotPathFor(s),
          scenario: { name: s.name, feature: s.feature },
        });
      }
    } else if (s.status === 'pending') {
      raws.push({
        source: 'pending-step',
        pillar: pillarFromFeature(s.feature),
        severity: 'warning',
        rawMessage: `Pending scenario: ${s.name}`,
        scenario: { name: s.name, feature: s.feature },
      });
    }
  }
  return raws;
}
```

(g) Update the call site in `aggregate` (around line 199) — and propagate `screenshotPath` through `groupIntoFindings`. Replace `groupIntoFindings` (around line 143) with:

```typescript
function groupIntoFindings(raws: RawFinding[]): Finding[] {
  const groups = new Map<string, Finding>();
  for (const r of raws) {
    const fp = fingerprint(r.rawMessage);
    const key = `${r.source}::${fp}`;
    const scenarioKey = (s: FindingScenario) => `${s.feature}::${s.name}::${s.human ?? ''}`;
    const existing = groups.get(key);
    if (existing) {
      existing.occurrences += 1;
      if (!existing.scenarios.some(s => scenarioKey(s) === scenarioKey(r.scenario))) {
        existing.scenarios.push(r.scenario);
      }
      // First non-empty screenshotPath wins; downstream renderers link to one image per finding.
      if (!existing.screenshotPath && r.screenshotPath) {
        existing.screenshotPath = r.screenshotPath;
      }
    } else {
      const message = normalizeMessage(r.rawMessage);
      groups.set(key, {
        fingerprint: fp,
        source: r.source,
        pillar: r.pillar,
        severity: r.severity,
        message,
        firstSeenUrl: r.url,
        screenshotPath: r.screenshotPath,
        occurrences: 1,
        scenarios: [r.scenario],
        suggestedObjective: suggestObjective(r.source, message),
      });
    }
  }
  return [...groups.values()].sort((a, b) => {
    if (b.occurrences !== a.occurrences) return b.occurrences - a.occurrences;
    return a.fingerprint.localeCompare(b.fingerprint);
  });
}
```

(h) Replace the bottom of `aggregate` (around line 197) so `buildScenarioRaws` receives the visual flag:

```typescript
export function aggregate(input: AggregateInput): SprintReport {
  const emitVisual = isPlaywrightProfile(input.profile);
  const raws = [
    ...buildScenarioRaws(input.scenarios, emitVisual),
    ...buildConsoleRaws(input.consoleArtifacts),
    ...buildGapRaws(input.gaps),
  ];

  const findings = groupIntoFindings(raws);
  const summary = computeSummary(input.scenarios, findings);

  if (emitVisual) {
    summary.visualValidation = computeVisualValidation(input.scenarios);
  }

  return {
    generatedAt: new Date().toISOString(),
    runId: input.runId,
    profile: input.profile,
    doorway: input.doorway,
    summary,
    findings,
  };
}
```

- [ ] **Step 4: Run tests to verify pass**

```bash
pnpm test:unit
pnpm typecheck
```

Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add genesis/a2o/scripts/lib/aggregate.ts \
        genesis/a2o/scripts/__tests__/aggregate.test.ts
git -c commit.gpgsign=false commit -m "feat(a2o): visual-regression findings + screenshotPath on findings

Tagged-and-failed scenarios in Playwright mode produce a dedicated
visual-regression finding (top-priority signal: a previously-confirmed
experience broke). scenario-failure findings also carry the screenshot
path when in Playwright mode, so reviewers click through from any
failure row to what the user actually saw.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: Render `## Visual Validation` section and screenshot links in markdown

**Files:**
- Modify: `genesis/a2o/scripts/lib/render-markdown.ts`
- Modify: `genesis/a2o/scripts/__tests__/render-markdown.test.ts`

- [ ] **Step 1: Write failing render tests**

Append the following at the end of `render-markdown.test.ts`, just before the final `});`:

```typescript
  void it('renders the Visual Validation section when summary.visualValidation is present', () => {
    const reportWithVisual: SprintReport = {
      ...report,
      profile: 'browser',
      summary: {
        ...report.summary,
        visualValidation: {
          validatedPassing: 12,
          validatedRegressed: 3,
          pendingPassing: 47,
          pendingFailing: 18,
        },
      },
    };
    const md = renderMarkdown(reportWithVisual);
    assert.match(md, /## Visual Validation/);
    assert.match(md, /validatedPassing.*12|12.*validated/i);
    assert.match(md, /validatedRegressed.*3|3.*regressed/i);
    assert.match(md, /pendingPassing.*47|47.*pending/i);
    assert.match(md, /pendingFailing.*18|18.*failing/i);
  });

  void it('omits the Visual Validation section when summary.visualValidation is absent', () => {
    const md = renderMarkdown(report);
    assert.doesNotMatch(md, /## Visual Validation/);
  });

  void it('renders screenshotPath as a Screenshot bullet on findings that have one', () => {
    const reportWithRegression: SprintReport = {
      ...report,
      findings: [
        {
          fingerprint: 'reg123',
          source: 'visual-regression',
          pillar: 'lamad',
          severity: 'error',
          message: 'Validated visual regressed: Starting a Journey',
          screenshotPath: 'reports/screenshots/lamad-learning-journey/starting-a-journey--Matthew.png',
          occurrences: 1,
          scenarios: [
            {
              name: 'Starting a Journey',
              feature: 'features/lamad/learning-journey.feature',
              human: 'Matthew',
            },
          ],
          suggestedObjective: 'Restore visual delivery of: Starting a Journey',
        },
      ],
    };
    const md = renderMarkdown(reportWithRegression);
    assert.match(md, /Screenshot.*lamad-learning-journey\/starting-a-journey--Matthew\.png/);
  });
```

- [ ] **Step 2: Run tests to verify failure**

```bash
pnpm test:unit -- --test-name-pattern='Visual Validation|screenshotPath'
```

Expected: FAIL — section not rendered, screenshot bullet absent.

- [ ] **Step 3: Implement render changes**

Replace the entire contents of `genesis/a2o/scripts/lib/render-markdown.ts` with:

```typescript
import type { SprintReport, Finding } from './aggregate.js';

function renderVisualValidation(report: SprintReport): string[] {
  const v = report.summary.visualValidation;
  if (!v) return [];
  const lines: string[] = [];
  lines.push(`## Visual Validation`);
  lines.push('');
  lines.push(`|  | passed | failed |`);
  lines.push(`|---|---|---|`);
  lines.push(`| has \`@elohim-visually-validated\` | ${v.validatedPassing} | **${v.validatedRegressed}** |`);
  lines.push(`| no tag (pending review) | ${v.pendingPassing} | ${v.pendingFailing} |`);
  lines.push('');
  lines.push(`- **${v.validatedPassing}** validatedPassing — confirmed delivering as designed`);
  lines.push(`- **${v.validatedRegressed}** validatedRegressed — see \`visual-regression\` findings below`);
  lines.push(`- **${v.pendingPassing}** pendingPassing — candidates for review`);
  lines.push(`- **${v.pendingFailing}** pendingFailing — see \`scenario-failure\` findings below`);
  lines.push('');
  return lines;
}

export function renderMarkdown(report: SprintReport): string {
  const lines: string[] = [];
  lines.push(`# A2O Sprint Report`);
  lines.push('');
  lines.push(`- **Run**: \`${report.runId}\``);
  lines.push(`- **Profile**: \`${report.profile}\``);
  if (report.doorway) lines.push(`- **Doorway**: ${report.doorway}`);
  lines.push(`- **Generated**: ${report.generatedAt}`);
  lines.push('');
  lines.push(`## Summary`);
  lines.push('');
  lines.push(`| scenarios | passed | failed | skipped | pending |`);
  lines.push(`|---|---|---|---|---|`);
  lines.push(
    `| ${report.summary.scenarios.total} | ${report.summary.scenarios.passed} | ${report.summary.scenarios.failed} | ${report.summary.scenarios.skipped} | ${report.summary.scenarios.pending} |`
  );
  lines.push('');
  lines.push(
    `Passed: **${report.summary.scenarios.passed}** | Failed: **${report.summary.scenarios.failed}** | Findings total: **${report.summary.findings.total}**`
  );
  lines.push('');

  lines.push(...renderVisualValidation(report));

  const byPillar = new Map<string, Finding[]>();
  for (const f of report.findings) {
    const arr = byPillar.get(f.pillar) ?? [];
    arr.push(f);
    byPillar.set(f.pillar, arr);
  }

  for (const [pillar, findings] of [...byPillar.entries()].sort(([a], [b]) => a.localeCompare(b))) {
    lines.push(`## ${pillar}`);
    lines.push('');
    for (const f of findings) {
      lines.push(`### [${f.source}] \`${f.fingerprint}\` (occurrences: ${f.occurrences})`);
      lines.push('');
      lines.push(`> ${f.message}`);
      lines.push('');
      if (f.firstSeenUrl) lines.push(`- URL: ${f.firstSeenUrl}`);
      if (f.screenshotPath) lines.push(`- **Screenshot**: \`${f.screenshotPath}\``);
      lines.push(`- **Objective**: ${f.suggestedObjective}`);
      lines.push('');
      lines.push(`<details><summary>Scenarios (${f.scenarios.length})</summary>`);
      lines.push('');
      for (const s of f.scenarios) {
        const who = s.human ? ` — ${s.human}` : '';
        lines.push(`- \`${s.feature}\` · ${s.name}${who}`);
      }
      lines.push('');
      lines.push(`</details>`);
      lines.push('');
    }
  }

  return lines.join('\n');
}
```

- [ ] **Step 4: Run tests to verify pass**

```bash
pnpm test:unit
pnpm typecheck
```

Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add genesis/a2o/scripts/lib/render-markdown.ts \
        genesis/a2o/scripts/__tests__/render-markdown.test.ts
git -c commit.gpgsign=false commit -m "feat(a2o): render Visual Validation section + screenshot links

When summary.visualValidation is present, prepend a 2x2 buckets table
under ## Visual Validation. Findings with screenshotPath render a
Screenshot bullet so reviewers click through from the markdown report.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 6: Add `featureSlug` and `scenarioSlug` properties to `E2EWorld`

**Files:**
- Modify: `genesis/a2o/src/framework/world.ts`

The cucumber `Before` hook already computes feature and scenario slugs to identify the observation report. The `After` hook needs the same slugs to compute the screenshot path. Stash them on the world.

- [ ] **Step 1: Add the two optional properties**

In `genesis/a2o/src/framework/world.ts`, find the `E2EWorld` class. Just below the existing `humans = new Map<string, Human>();` line (around line 39), add:

```typescript
  /** Slug of the current scenario's feature file, set by the Before hook (e.g. "lamad-learning-journey") */
  featureSlug?: string;

  /** Slug of the current scenario's name, set by the Before hook (e.g. "starting-a-journey") */
  scenarioSlug?: string;
```

- [ ] **Step 2: Verify typecheck still passes**

```bash
pnpm typecheck
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add genesis/a2o/src/framework/world.ts
git -c commit.gpgsign=false commit -m "feat(a2o): add featureSlug/scenarioSlug to E2EWorld

Before hook stashes; After hook reads. No behavior change yet — wired
in next task with universal capture.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 7: Universal Playwright capture in `Before`/`After` hooks

**Files:**
- Modify: `genesis/a2o/steps/common.steps.ts`

This task changes runtime behaviour: every Playwright-mode scenario now writes a screenshot to `reports/screenshots/{featureSlug}/{scenarioSlug}--{human}.png`, and failures additionally write a sibling `.error.json`. The legacy `FAIL-{name}-{human}` screenshot naming is retired (failure context lives in the new `.error.json`); console-error and trace artifacts keep their existing `FAIL-…` filenames.

There is no clean unit-test seam here (these are cucumber lifecycle hooks). Verification is by directory inspection after a Playwright run (Step 4 below).

- [ ] **Step 1: Refactor common.steps.ts**

Open `genesis/a2o/steps/common.steps.ts`. Apply three changes — no other code in the file should move.

(a) Add `mkdirSync` is already imported. After the imports and `setDefaultTimeout`, add a new helper near the top of the file (right above `function captureFailureArtifacts`):

```typescript
/**
 * Compute slugs for a cucumber scenario the same way they're used in
 * observation reports. Idempotent and pure.
 */
function computeSlugs(scenarioUri: string, scenarioName: string): {
  featureSlug: string;
  scenarioSlug: string;
} {
  const featureSlug = scenarioUri
    .replace(/^.*features\//, '')
    .replace(/\.feature$/, '')
    .replace(/\//g, '-');
  const scenarioSlug = scenarioName
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');
  return { featureSlug, scenarioSlug };
}

/**
 * Capture a full-page screenshot for a single device into the per-feature
 * subdir. Called for every Playwright scenario regardless of pass/fail.
 */
async function captureVisualEvidence(
  device: PlaywrightDevice,
  featureSlug: string,
  scenarioSlug: string,
  humanName: string
): Promise<void> {
  const dir = `reports/screenshots/${featureSlug}`;
  mkdirSync(dir, { recursive: true });
  try {
    await device.screenshot(`${featureSlug}/${scenarioSlug}--${humanName}`);
  } catch {
    // best-effort — page may have crashed
  }
}

/**
 * Write a sibling .error.json next to the screenshot for failed scenarios so
 * a reviewer can read the failure context without consulting the cucumber JSON.
 */
function writeErrorSidecar(
  featureSlug: string,
  scenarioSlug: string,
  humanName: string,
  failureMessage: string
): void {
  try {
    const path = `reports/screenshots/${featureSlug}/${scenarioSlug}--${humanName}.error.json`;
    writeFileSync(
      path,
      JSON.stringify(
        {
          status: 'failed',
          failureMessage: failureMessage.split('\n')[0],
        },
        null,
        2
      )
    );
  } catch {
    // best-effort
  }
}
```

(b) Update the `Before` hook (around line 139). Replace the existing `Before` function body — keeping the existing observation logic, but adding slug stashing — with:

```typescript
Before(async function (this: E2EWorld, scenario) {
  for (const [, human] of this.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        device.clearCapture();
      }
    }
  }

  const { featureSlug, scenarioSlug } = computeSlugs(scenario.pickle.uri, scenario.pickle.name);
  this.featureSlug = featureSlug;
  this.scenarioSlug = scenarioSlug;

  // Start observation session if a doorway is registered
  for (const [, doorway] of this.doorways) {
    try {
      const scenarioId = `${featureSlug}--${scenarioSlug}`;
      await doorway.client.beginObservation({
        scenario: scenario.pickle.name,
        scenarioId,
        tags: scenario.pickle.tags.map(t => t.name),
        feature: scenario.pickle.uri,
      });
    } catch {
      // Observation is best-effort — don't block scenarios if storage is down
    }
    break; // Only observe on the first doorway
  }
});
```

(c) Update the `After` hook (around line 227). Replace it with:

```typescript
After(async function (this: E2EWorld, scenario) {
  const featureSlug = this.featureSlug ?? 'unknown-feature';
  const scenarioSlug = this.scenarioSlug ?? 'unknown-scenario';

  // Universal capture: every Playwright device gets a screenshot regardless
  // of pass/fail outcome. Failures additionally get a sibling .error.json,
  // and the existing console-errors / trace artifacts (failure-only).
  for (const [name, human] of this.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        await captureVisualEvidence(device, featureSlug, scenarioSlug, name);
      }
    }
  }

  if (scenario.result?.status === Status.FAILED) {
    const safeName = scenario.pickle.name.replace(/[^a-zA-Z0-9]/g, '-');
    const failureMessage = scenario.result.message ?? 'unknown failure';

    for (const [name, human] of this.humans) {
      for (const device of human.devices) {
        if (device instanceof PlaywrightDevice) {
          // Write console-errors JSON and trace under reports/console and reports/traces.
          // We no longer write a FAIL- prefixed screenshot — the universal capture above
          // covered it and a sidecar .error.json carries the failure context.
          await captureFailureArtifactsExceptScreenshot(device, safeName, name);
          writeErrorSidecar(featureSlug, scenarioSlug, name, failureMessage);
        }
      }
    }
  }

  // For passing scenarios, assert that no real console errors were logged.
  // This makes console cleanliness an automatic test contract for all browser scenarios.
  if (scenario.result?.status === Status.PASSED) {
    const errorReport = collectBrowserErrors(this, scenario.pickle.name);
    if (errorReport.length) {
      throw new Error(
        `Scenario passed but had ${errorReport.length} browser error(s):\n` +
          errorReport.map(e => `  ${e}`).join('\n')
      );
    }
  }

  await collectObservationReport(this, scenario);
  await this.runCleanup();
});
```

(d) Replace the existing `captureFailureArtifacts` helper (around line 42) with one that no longer takes the screenshot — renamed for clarity:

```typescript
/**
 * Capture failure-only artifacts (console errors JSON, Playwright trace) for a single
 * device. The screenshot is captured separately by captureVisualEvidence in the
 * universal-capture path of the After hook.
 */
async function captureFailureArtifactsExceptScreenshot(
  device: PlaywrightDevice,
  safeName: string,
  humanName: string
): Promise<void> {
  const errors = device.getErrors();
  const hasArtifacts = errors.console.length || errors.page.length || errors.network.length;
  if (hasArtifacts) {
    try {
      writeFileSync(
        `reports/console/${safeName}-${humanName}.json`,
        JSON.stringify(errors, null, 2)
      );
    } catch {
      // best-effort
    }
  }

  try {
    await device.saveTrace(`FAIL-${safeName}-${humanName}`);
  } catch {
    // best-effort
  }
}
```

- [ ] **Step 2: Lint and typecheck**

Run from `/projects/elohim/genesis/a2o`:
```bash
pnpm typecheck
pnpm lint
```

Expected: Both pass. (Lint may want the now-unused `captureFailureArtifacts` removed — Step 1d already removed it. If lint warns about an unused import, prune it.)

- [ ] **Step 3: Run unit tests**

```bash
pnpm test:unit
```

Expected: All tests pass. (Unit tests don't exercise hooks directly; this is a sanity check that no other file broke.)

- [ ] **Step 4: Manual smoke verification**

This step requires Playwright + a doorway. If Playwright can't run in this environment, mark this step as deferred and proceed; the integration smoke runs in CI.

If Playwright can run locally:
```bash
cd /projects/elohim/genesis/a2o
rm -rf reports/screenshots
E2E_DOORWAY_ALPHA=https://doorway-alpha.elohim.host \
  pnpm test:browser -- --tags '@e2e and @lamad and @browser-only' --fail-fast
ls -la reports/screenshots/
find reports/screenshots/ -type f
```

Expected:
- `reports/screenshots/` contains a subdir per feature touched (e.g. `lamad-learning-journey/`)
- Each subdir contains `{scenarioSlug}--{human}.png`
- For any failed scenario, a sibling `{scenarioSlug}--{human}.error.json` exists
- No `FAIL-…` files in `reports/screenshots/` (those have been retired)
- `reports/console/` and `reports/traces/` still contain `FAIL-…` files for failures

- [ ] **Step 5: Commit**

```bash
git add genesis/a2o/steps/common.steps.ts
git -c commit.gpgsign=false commit -m "feat(a2o): universal Playwright capture into per-feature subdirs

After hook now captures every Playwright-mode scenario regardless of
outcome at reports/screenshots/{featureSlug}/{scenarioSlug}--{human}.png.
Failures additionally write a sibling .error.json carrying the
failure message. Legacy FAIL-{name}-{human}.png screenshot naming
retired; console-errors and traces keep FAIL- prefix as before.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 8: Final integration check

**Files:** None (verification only)

- [ ] **Step 1: Run all gates**

Run from `/projects/elohim/genesis/a2o`:
```bash
pnpm test:unit
pnpm typecheck
pnpm lint
pnpm format:check
```

All must pass. If `format:check` fails, run `pnpm format` and amend the most recent commit with the formatting fix only:

```bash
git add -A
git -c commit.gpgsign=false commit -m "chore(a2o): prettier format pass

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

- [ ] **Step 2: Smoke-validate the schema against an emitted report**

Generate a synthetic sprint-report against the new schema by running the existing build script with a small input. From `/projects/elohim/genesis/a2o`:

```bash
mkdir -p /tmp/a2o-smoke
cat > /tmp/a2o-smoke/cucumber-report.json <<'EOF'
[
  {
    "uri": "features/lamad/learning-journey.feature",
    "name": "Learning Journey",
    "elements": [
      {
        "name": "Starting a Journey",
        "type": "scenario",
        "tags": [{"name": "@e2e"}, {"name": "@elohim-visually-validated"}],
        "steps": [{"name": "step", "result": {"status": "passed"}}]
      },
      {
        "name": "Earning Affinity",
        "type": "scenario",
        "tags": [{"name": "@e2e"}, {"name": "@elohim-visually-validated"}],
        "steps": [{"name": "step", "result": {"status": "failed", "error_message": "boom"}}]
      }
    ]
  }
]
EOF
mkdir -p /tmp/a2o-smoke/console
echo '[]' > /tmp/a2o-smoke/coverage-gap.json

pnpm exec tsx scripts/build-sprint-report.ts \
  --reports-dir /tmp/a2o-smoke \
  --profile browser \
  --run-id smoke-test \
  --out-json /tmp/a2o-smoke/sprint-report.json \
  --out-md /tmp/a2o-smoke/sprint-report.md

cat /tmp/a2o-smoke/sprint-report.md
```

Expected output includes:
- `## Visual Validation` section with the 2×2 table
- `validatedPassing` = 1, `validatedRegressed` = 1
- A finding with `[visual-regression]` and a `**Screenshot**:` bullet pointing at `reports/screenshots/lamad-learning-journey/earning-affinity.png`
- The script exits 0 (no schema validation errors)

If the script exits non-zero, the schema and the aggregator have drifted — fix and re-run.

- [ ] **Step 3: Confirm no behavior change for non-Playwright profiles**

```bash
pnpm exec tsx scripts/build-sprint-report.ts \
  --reports-dir /tmp/a2o-smoke \
  --profile alpha \
  --run-id smoke-test \
  --out-json /tmp/a2o-smoke/sprint-report.json \
  --out-md /tmp/a2o-smoke/sprint-report.md

grep -c '## Visual Validation' /tmp/a2o-smoke/sprint-report.md || echo "section absent (correct)"
grep -c 'Screenshot' /tmp/a2o-smoke/sprint-report.md || echo "no screenshot links (correct)"
```

Expected: both grep commands print `section absent (correct)` / `no screenshot links (correct)` — i.e. zero matches.

- [ ] **Step 4: Clean up smoke artifacts**

```bash
rm -rf /tmp/a2o-smoke
```

- [ ] **Step 5: Done**

No additional commit unless a formatting fix was needed in Step 1. The branch is ready for normal `dev` integration. The `/finishing-a-development-branch` flow can now run.

---

## Notes

- **P2P design gate — not applicable.** All "schema" references in this plan are to `genesis/a2o/schemas/sprint-report.schema.json`, a local CI artifact format produced and consumed by the same process (`build-sprint-report.ts`). It is not a DHT entry type, libp2p sync message, REA primitive, or HTTP route — none of the categories the p2p-design-gate skill governs. Screenshots are ephemeral local files under `reports/`, archived by Jenkins build retention. No peer-to-peer state is created or modified.
- **No flag day.** All changes are additive. Existing reports remain valid against the new schema; non-Playwright profiles behave identically to today.
- **No feature-file edits required to land.** All scenarios start in `pendingPassing` / `pendingFailing` until a human reviewer adds `@elohim-visually-validated`.
- **Jenkins archival.** The root and a2o pipelines already archive `reports/**`; per-feature screenshot subdirs are picked up automatically.
- **Out of scope (deferred to a future `/shift` skill update):** Agent-assisted review loop where a vision-capable agent proposes tag additions in PRs. The artifacts and counts produced by this plan are exactly the inputs that loop will need.
- **Multi-human scenarios** (e.g. Matthew + Terrance in the same scenario) produce one image per human under the same feature subdir; the finding's `screenshotPath` points at one image (Matthew's by default — first human iteration), with the per-scenario list still in `<details>`.
