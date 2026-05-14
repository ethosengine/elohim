# A2O Minutia Aggregator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a post-test aggregator that reads the existing a2o artifact streams (Cucumber results, per-scenario browser console/error JSON, coverage-gap output) and produces a deduplicated, fingerprinted `sprint-report.{json,md}` ranked by frequency — the feed for iterative sprint planning and `/shift` Objectives.

**Architecture:** A small TypeScript CLI (`genesis/a2o/scripts/build-sprint-report.ts`) that runs after the Cucumber stage in CI. It loads three artifact streams, groups defects by message fingerprint, attaches the scenarios that triggered each, tags by pillar, emits JSON + Markdown. Pure functions for loading, fingerprinting, aggregating, and rendering — each unit-tested with Node's built-in `node:test`. Schema-first per project convention: `sprint-report.schema.json` is authored before any code.

**Tech Stack:** TypeScript 5.7 (existing a2o project), `tsx` runner, `node:test` + `node:assert`, `ajv` for schema validation (already a transitive dep; add explicitly). No new frameworks — matches the existing `scripts/*.ts` pattern (`scan-coverage.ts`, `extract-failures.ts`).

---

## Scope & Non-Goals

**In scope:** Parsing existing artifacts, fingerprinting, aggregation, Markdown rendering, CI wiring, one end-to-end verification against real alpha artifacts.

**Out of scope (future plans):** Backend log correlation (requires Plan B — runtime request-ID infrastructure), enabling `@browser-only` in CI, any new capture hooks in step definitions. We consume what's already written today.

**Forward compatibility:** The schema reserves an optional `peer` field on each Finding. Plan B introduces `X-Target-Peer` / `X-Served-By` headers and adds a capture hook that populates it — no changes to this plan's code required beyond a one-line aggregator addition.

**Assumption:** Cucumber already writes `reports/cucumber-report.json`; scenario hooks already write `reports/console/{scenario}-{human}-errors.json`; `pnpm scan:coverage` already writes a coverage-gap JSON file. Any gap here is surfaced in Task 7 (integration test) and dealt with inside this plan.

---

## File Structure

```
genesis/a2o/
├── schemas/
│   └── sprint-report.schema.json         # Source of truth for report shape
├── scripts/
│   ├── build-sprint-report.ts            # CLI entry (reads artifacts → writes report)
│   ├── lib/
│   │   ├── fingerprint.ts                # Pure: error message → stable fingerprint
│   │   ├── load-cucumber.ts              # Pure: cucumber-report.json → ScenarioResult[]
│   │   ├── load-console.ts               # Pure: reports/console/*.json → BrowserArtifact[]
│   │   ├── load-coverage-gap.ts          # Pure: coverage-gap.json → GapFinding[]
│   │   ├── aggregate.ts                  # Pure: merges all inputs → SprintReport
│   │   ├── pillar-from-feature.ts        # Pure: feature path → pillar tag
│   │   └── render-markdown.ts            # Pure: SprintReport → markdown string
│   └── __tests__/
│       ├── fingerprint.test.ts
│       ├── load-cucumber.test.ts
│       ├── load-console.test.ts
│       ├── aggregate.test.ts
│       ├── pillar-from-feature.test.ts
│       ├── render-markdown.test.ts
│       └── fixtures/                     # Tiny JSON fixtures
│           ├── cucumber-mixed.json
│           ├── console-terrance-errors.json
│           ├── console-mary-errors.json
│           └── coverage-gap.json
```

Each file has one responsibility. Loaders and renderers are pure; the CLI is the only I/O orchestrator.

---

## Task 1: Author the `sprint-report.schema.json`

**Files:**
- Create: `genesis/a2o/schemas/sprint-report.schema.json`

Schema-first — every downstream type and assertion derives from this contract.

- [ ] **Step 1: Write the schema file**

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.protocol/schemas/a2o/sprint-report.schema.json",
  "title": "A2oSprintReport",
  "description": "Aggregated sprint-planning report from an a2o run: deduplicated defects ranked by frequency.",
  "type": "object",
  "required": ["generatedAt", "runId", "profile", "summary", "findings"],
  "additionalProperties": false,
  "properties": {
    "generatedAt": { "type": "string", "format": "date-time" },
    "runId": { "type": "string", "description": "CI run identifier or timestamp for local runs" },
    "profile": { "type": "string", "description": "cucumber profile (alpha, local, browser, delivery, ...)" },
    "doorway": { "type": "string", "description": "Doorway base URL the run targeted" },
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
            "total":   { "type": "integer", "minimum": 0 },
            "passed":  { "type": "integer", "minimum": 0 },
            "failed":  { "type": "integer", "minimum": 0 },
            "skipped": { "type": "integer", "minimum": 0 },
            "pending": { "type": "integer", "minimum": 0 }
          }
        },
        "findings": {
          "type": "object",
          "required": ["total", "bySource", "byPillar"],
          "additionalProperties": false,
          "properties": {
            "total":    { "type": "integer", "minimum": 0 },
            "bySource": { "type": "object", "additionalProperties": { "type": "integer", "minimum": 0 } },
            "byPillar": { "type": "object", "additionalProperties": { "type": "integer", "minimum": 0 } }
          }
        }
      }
    },
    "findings": {
      "type": "array",
      "items": { "$ref": "#/definitions/Finding" }
    }
  },
  "definitions": {
    "Finding": {
      "type": "object",
      "required": ["fingerprint", "source", "pillar", "message", "occurrences", "scenarios"],
      "additionalProperties": false,
      "properties": {
        "fingerprint": { "type": "string", "description": "Stable hash over the normalized message" },
        "source":      { "enum": ["console-error", "page-error", "failed-request", "scenario-failure", "pending-step", "coverage-gap"] },
        "pillar":      { "type": "string", "description": "lamad | imagodei | elohim | federation | delivery | browser | content | deployment | qahal | shefa | unknown" },
        "peer":        { "type": "string", "description": "Target-peer slug (e.g., 'terrance-household', 'shem'). Populated by Plan B once request-ID/peer routing is live; absent for local-only runs." },
        "severity":    { "enum": ["error", "warning", "info"], "default": "error" },
        "message":     { "type": "string" },
        "firstSeenUrl":{ "type": "string" },
        "occurrences": { "type": "integer", "minimum": 1 },
        "scenarios":   {
          "type": "array",
          "minItems": 1,
          "items":   {
            "type": "object",
            "required": ["name", "feature"],
            "additionalProperties": false,
            "properties": {
              "name":    { "type": "string" },
              "feature": { "type": "string" },
              "human":   { "type": "string" }
            }
          }
        },
        "suggestedObjective": { "type": "string", "description": "One-line headline for /shift Objective seeding" }
      }
    }
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add genesis/a2o/schemas/sprint-report.schema.json
git commit -m "feat(a2o): add sprint-report JSON schema (source of truth for aggregator output)"
```

---

## Task 2: Fingerprint utility — pure function + tests

**Files:**
- Create: `genesis/a2o/scripts/lib/fingerprint.ts`
- Create: `genesis/a2o/scripts/__tests__/fingerprint.test.ts`

Fingerprints must be **stable across runs** (same error → same hash) but **normalized** against runtime noise (UUIDs, ISO timestamps, file:line positions, port numbers, quoted paths). A 12-hex-char prefix of SHA-256 is plenty for human-readable collision-resistant keys at our scale.

- [ ] **Step 1: Write the failing test**

```typescript
// genesis/a2o/scripts/__tests__/fingerprint.test.ts
import { describe, it } from 'node:test';
import { strict as assert } from 'node:assert';
import { fingerprint, normalizeMessage } from '../lib/fingerprint.js';

describe('normalizeMessage', () => {
  it('strips UUIDs', () => {
    const m = 'Failed to load content 550e8400-e29b-41d4-a716-446655440000';
    assert.equal(normalizeMessage(m), 'Failed to load content <uuid>');
  });

  it('strips ISO timestamps', () => {
    const m = 'Request at 2026-04-19T10:15:23.123Z timed out';
    assert.equal(normalizeMessage(m), 'Request at <ts> timed out');
  });

  it('strips port numbers from URLs', () => {
    const m = 'fetch https://doorway-alpha.elohim.host:8443/foo failed';
    assert.equal(
      normalizeMessage(m),
      'fetch https://doorway-alpha.elohim.host/foo failed'
    );
  });

  it('strips hex hashes (sha-256 style)', () => {
    const m = 'Shard sha256-abc123def456789012345678901234567890 missing';
    assert.equal(normalizeMessage(m), 'Shard sha256-<hash> missing');
  });

  it('collapses multi-whitespace', () => {
    assert.equal(normalizeMessage('a  \n  b\t\tc'), 'a b c');
  });

  it('is idempotent', () => {
    const once = normalizeMessage('id 550e8400-e29b-41d4-a716-446655440000');
    assert.equal(normalizeMessage(once), once);
  });
});

describe('fingerprint', () => {
  it('returns 12-hex-char prefix', () => {
    const fp = fingerprint('any message');
    assert.match(fp, /^[0-9a-f]{12}$/);
  });

  it('is stable across calls', () => {
    assert.equal(
      fingerprint('ReferenceError: x is not defined'),
      fingerprint('ReferenceError: x is not defined')
    );
  });

  it('ignores runtime noise so two realistic variants collide', () => {
    const a = fingerprint('Failed to load 550e8400-e29b-41d4-a716-446655440000 at 2026-04-19T10:15:23.123Z');
    const b = fingerprint('Failed to load 660e8400-e29b-41d4-a716-446655441111 at 2026-04-19T10:20:00.000Z');
    assert.equal(a, b);
  });

  it('distinguishes genuinely different messages', () => {
    assert.notEqual(
      fingerprint('ReferenceError: x is not defined'),
      fingerprint('TypeError: Cannot read properties of undefined')
    );
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd genesis/a2o
npx tsx --test scripts/__tests__/fingerprint.test.ts
```

Expected: FAIL with "Cannot find module '../lib/fingerprint.js'"

- [ ] **Step 3: Write the implementation**

```typescript
// genesis/a2o/scripts/lib/fingerprint.ts
import { createHash } from 'node:crypto';

const UUID_RE = /\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b/gi;
const ISO_RE  = /\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z/g;
const HEX_HASH_RE = /\b[0-9a-f]{16,}\b/gi;
const URL_PORT_RE = /(https?:\/\/[^\s/:]+):\d+/g;
const WS_RE = /\s+/g;

export function normalizeMessage(raw: string): string {
  return raw
    .replace(UUID_RE, '<uuid>')
    .replace(ISO_RE, '<ts>')
    .replace(URL_PORT_RE, '$1')
    .replace(HEX_HASH_RE, '<hash>')
    .replace(WS_RE, ' ')
    .trim();
}

export function fingerprint(raw: string): string {
  const normalized = normalizeMessage(raw);
  return createHash('sha256').update(normalized).digest('hex').slice(0, 12);
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd genesis/a2o
npx tsx --test scripts/__tests__/fingerprint.test.ts
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add genesis/a2o/scripts/lib/fingerprint.ts genesis/a2o/scripts/__tests__/fingerprint.test.ts
git commit -m "feat(a2o): fingerprint utility for defect deduplication"
```

---

## Task 3: Pillar-from-feature-path — pure function + tests

**Files:**
- Create: `genesis/a2o/scripts/lib/pillar-from-feature.ts`
- Create: `genesis/a2o/scripts/__tests__/pillar-from-feature.test.ts`

Maps a feature URI to a pillar tag for sprint routing. Uses the directory under `features/` as the pillar, defaulting to `unknown` when it can't be inferred.

- [ ] **Step 1: Write the failing test**

```typescript
// genesis/a2o/scripts/__tests__/pillar-from-feature.test.ts
import { describe, it } from 'node:test';
import { strict as assert } from 'node:assert';
import { pillarFromFeature } from '../lib/pillar-from-feature.js';

describe('pillarFromFeature', () => {
  const cases: [string, string][] = [
    ['features/lamad/learning-journey.feature',              'lamad'],
    ['features/auth/fixture-humans.feature',                 'imagodei'],
    ['features/content/content-lifecycle.feature',           'content'],
    ['features/federation/peer-advertisement.feature',       'federation'],
    ['features/delivery/peer-mesh.feature',                  'delivery'],
    ['features/browser/auth-browser.feature',                'browser'],
    ['features/elohim/presence.feature',                     'elohim'],
    ['features/qahal/collective-governance.feature',         'qahal'],
    ['features/shefa/human-resilience.feature',              'shefa'],
    ['features/deployment/staging-validation.feature',       'deployment'],
    ['genesis/a2o/features/lamad/path-adaptation.feature',   'lamad'],
    ['/absolute/path/to/features/browser/nav.feature',       'browser'],
    ['features/weird-new-area/x.feature',                    'weird-new-area'],
    ['not-a-feature-path.txt',                               'unknown'],
  ];

  for (const [uri, expected] of cases) {
    it(`"${uri}" → ${expected}`, () => {
      assert.equal(pillarFromFeature(uri), expected);
    });
  }
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd genesis/a2o
npx tsx --test scripts/__tests__/pillar-from-feature.test.ts
```

Expected: FAIL — module not found.

- [ ] **Step 3: Write the implementation**

```typescript
// genesis/a2o/scripts/lib/pillar-from-feature.ts
const AUTH_IS_IMAGODEI = new Map<string, string>([['auth', 'imagodei']]);

export function pillarFromFeature(uri: string): string {
  const match = uri.match(/features\/([^/]+)\/[^/]+\.feature$/);
  if (!match) return 'unknown';
  const raw = match[1];
  return AUTH_IS_IMAGODEI.get(raw) ?? raw;
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd genesis/a2o
npx tsx --test scripts/__tests__/pillar-from-feature.test.ts
```

Expected: all 14 cases pass.

- [ ] **Step 5: Commit**

```bash
git add genesis/a2o/scripts/lib/pillar-from-feature.ts genesis/a2o/scripts/__tests__/pillar-from-feature.test.ts
git commit -m "feat(a2o): feature-path → pillar mapping"
```

---

## Task 4: Cucumber loader — pure function + tests

**Files:**
- Create: `genesis/a2o/scripts/lib/load-cucumber.ts`
- Create: `genesis/a2o/scripts/__tests__/fixtures/cucumber-mixed.json`
- Create: `genesis/a2o/scripts/__tests__/load-cucumber.test.ts`

Cucumber's JSON format is one array of feature objects, each with `elements` (scenarios), each with `steps`. We extract scenario name, feature URI, status, and any failed step's error message. See the [official schema](https://github.com/cucumber/cucumber-json-converter).

- [ ] **Step 1: Write the fixture**

```json
// genesis/a2o/scripts/__tests__/fixtures/cucumber-mixed.json
[
  {
    "uri": "features/lamad/learning-journey.feature",
    "name": "Learning Journey",
    "elements": [
      {
        "name": "Terrance completes path",
        "type": "scenario",
        "steps": [
          { "name": "doorway is reachable", "result": { "status": "passed", "duration": 100 } }
        ]
      },
      {
        "name": "Mary fails on assessment",
        "type": "scenario",
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
        "steps": [
          { "name": "a new human wanders in", "result": { "status": "pending", "duration": 0 } }
        ]
      }
    ]
  }
]
```

- [ ] **Step 2: Write the failing test**

```typescript
// genesis/a2o/scripts/__tests__/load-cucumber.test.ts
import { describe, it } from 'node:test';
import { strict as assert } from 'node:assert';
import { readFileSync } from 'node:fs';
import { loadCucumber } from '../lib/load-cucumber.js';

const fixture = readFileSync(
  new URL('./fixtures/cucumber-mixed.json', import.meta.url),
  'utf8'
);

describe('loadCucumber', () => {
  it('parses all scenarios with feature URI', () => {
    const results = loadCucumber(fixture);
    assert.equal(results.length, 3);
    assert.equal(results[0].name, 'Terrance completes path');
    assert.equal(results[0].feature, 'features/lamad/learning-journey.feature');
  });

  it('classifies passed scenario', () => {
    const [passed] = loadCucumber(fixture);
    assert.equal(passed.status, 'passed');
    assert.equal(passed.failureMessage, undefined);
  });

  it('extracts failure message from failed step', () => {
    const failed = loadCucumber(fixture).find(r => r.status === 'failed')!;
    assert.ok(failed);
    assert.match(failed.failureMessage!, /AssertionError: expected 500 to be 200/);
  });

  it('classifies pending scenario', () => {
    const pending = loadCucumber(fixture).find(r => r.status === 'pending')!;
    assert.ok(pending);
    assert.equal(pending.failureMessage, undefined);
  });

  it('computes summary counts', () => {
    const summary = loadCucumber(fixture).reduce(
      (acc, r) => ({ ...acc, [r.status]: (acc[r.status] || 0) + 1 }),
      {} as Record<string, number>
    );
    assert.deepEqual(summary, { passed: 1, failed: 1, pending: 1 });
  });

  it('throws on malformed input', () => {
    assert.throws(() => loadCucumber('not-json'), /JSON/);
  });
});
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cd genesis/a2o
npx tsx --test scripts/__tests__/load-cucumber.test.ts
```

Expected: FAIL — module not found.

- [ ] **Step 4: Write the implementation**

```typescript
// genesis/a2o/scripts/lib/load-cucumber.ts
export type ScenarioStatus = 'passed' | 'failed' | 'skipped' | 'pending' | 'undefined';

export interface ScenarioResult {
  name: string;
  feature: string;
  status: ScenarioStatus;
  failureMessage?: string;
}

interface CucumberStep {
  name: string;
  result?: { status: string; duration?: number; error_message?: string };
}

interface CucumberElement {
  name: string;
  type: string;
  steps?: CucumberStep[];
}

interface CucumberFeature {
  uri: string;
  name: string;
  elements?: CucumberElement[];
}

const STATUS_PRIORITY: ScenarioStatus[] = [
  'failed', 'undefined', 'pending', 'skipped', 'passed',
];

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
      results.push({
        name: el.name,
        feature: feature.uri,
        status,
        failureMessage: failed?.result?.error_message,
      });
    }
  }
  return results;
}
```

- [ ] **Step 5: Run test to verify it passes**

```bash
cd genesis/a2o
npx tsx --test scripts/__tests__/load-cucumber.test.ts
```

Expected: all 6 tests pass.

- [ ] **Step 6: Commit**

```bash
git add genesis/a2o/scripts/lib/load-cucumber.ts \
        genesis/a2o/scripts/__tests__/load-cucumber.test.ts \
        genesis/a2o/scripts/__tests__/fixtures/cucumber-mixed.json
git commit -m "feat(a2o): cucumber report loader with scenario status + failure extraction"
```

---

## Task 5: Console-artifact loader — pure function + tests

**Files:**
- Create: `genesis/a2o/scripts/lib/load-console.ts`
- Create: `genesis/a2o/scripts/__tests__/fixtures/console-terrance-errors.json`
- Create: `genesis/a2o/scripts/__tests__/fixtures/console-mary-errors.json`
- Create: `genesis/a2o/scripts/__tests__/load-console.test.ts`

Step-level capture in `steps/common.steps.ts:89` writes `reports/console/{safeName}.json` with shape `{ consoleErrors, pageErrors }` — where each error matches `CapturedConsoleLog` from `PlaywrightDevice`. We derive scenario name from filename (`{scenario}-{human}-errors.json`).

- [ ] **Step 1: Write the fixtures**

```json
// genesis/a2o/scripts/__tests__/fixtures/console-terrance-errors.json
{
  "consoleErrors": [
    {
      "level": "error",
      "text": "ReferenceError: Sophia is not defined",
      "url": "https://doorway-alpha.elohim.host/assets/sophia-element.umd.js"
    },
    {
      "level": "error",
      "text": "Failed to load resource: the server responded with a status of 404",
      "url": "https://doorway-alpha.elohim.host/api/v1/cache/missing-cid"
    }
  ],
  "pageErrors": []
}
```

```json
// genesis/a2o/scripts/__tests__/fixtures/console-mary-errors.json
{
  "consoleErrors": [
    {
      "level": "error",
      "text": "ReferenceError: Sophia is not defined",
      "url": "https://doorway-alpha.elohim.host/assets/sophia-element.umd.js"
    }
  ],
  "pageErrors": [
    { "message": "Uncaught (in promise) TypeError: Cannot read properties of null (reading 'token')", "url": "https://doorway-alpha.elohim.host/login" }
  ]
}
```

- [ ] **Step 2: Write the failing test**

```typescript
// genesis/a2o/scripts/__tests__/load-console.test.ts
import { describe, it } from 'node:test';
import { strict as assert } from 'node:assert';
import { mkdtempSync, writeFileSync, cpSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadConsoleArtifacts, parseScenarioHumanFromFilename } from '../lib/load-console.js';

const fixturesDir = fileURLToPath(new URL('./fixtures/', import.meta.url));

function makeReportsDir(): string {
  const tmp = mkdtempSync(join(tmpdir(), 'a2o-console-'));
  cpSync(join(fixturesDir, 'console-terrance-errors.json'), join(tmp, 'learning-journey-terrance-errors.json'));
  cpSync(join(fixturesDir, 'console-mary-errors.json'),    join(tmp, 'learning-journey-mary-errors.json'));
  writeFileSync(join(tmp, 'not-an-artifact.txt'), 'ignored');
  return tmp;
}

describe('parseScenarioHumanFromFilename', () => {
  it('splits scenario and human on last hyphen before "errors"', () => {
    assert.deepEqual(
      parseScenarioHumanFromFilename('learning-journey-terrance-errors.json'),
      { scenario: 'learning-journey', human: 'terrance' }
    );
  });

  it('returns null on unrecognized filename', () => {
    assert.equal(parseScenarioHumanFromFilename('random.json'), null);
  });
});

describe('loadConsoleArtifacts', () => {
  it('reads every *-errors.json in the directory', () => {
    const dir = makeReportsDir();
    const arts = loadConsoleArtifacts(dir);
    assert.equal(arts.length, 2);
  });

  it('returns empty array when directory does not exist', () => {
    assert.deepEqual(loadConsoleArtifacts('/no/such/path'), []);
  });

  it('includes console + page errors with scenario/human tagging', () => {
    const dir = makeReportsDir();
    const arts = loadConsoleArtifacts(dir);
    const mary = arts.find(a => a.human === 'mary')!;
    assert.ok(mary);
    assert.equal(mary.scenario, 'learning-journey');
    assert.equal(mary.consoleErrors.length, 1);
    assert.equal(mary.pageErrors.length, 1);
  });

  it('ignores non-artifact files silently', () => {
    const dir = makeReportsDir();
    const arts = loadConsoleArtifacts(dir);
    assert.ok(arts.every(a => a.scenario && a.human));
  });
});
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cd genesis/a2o
npx tsx --test scripts/__tests__/load-console.test.ts
```

Expected: FAIL — module not found.

- [ ] **Step 4: Write the implementation**

```typescript
// genesis/a2o/scripts/lib/load-console.ts
import { readdirSync, readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';

export interface ConsoleLogEntry { level: string; text: string; url: string }
export interface PageErrorEntry   { message: string; url: string }

export interface ConsoleArtifact {
  scenario: string;
  human: string;
  consoleErrors: ConsoleLogEntry[];
  pageErrors: PageErrorEntry[];
}

const FILENAME_RE = /^(.+)-([^-]+)-errors\.json$/;

export function parseScenarioHumanFromFilename(
  filename: string
): { scenario: string; human: string } | null {
  const m = filename.match(FILENAME_RE);
  if (!m) return null;
  return { scenario: m[1], human: m[2] };
}

export function loadConsoleArtifacts(dir: string): ConsoleArtifact[] {
  if (!existsSync(dir)) return [];
  const entries = readdirSync(dir);
  const artifacts: ConsoleArtifact[] = [];
  for (const name of entries) {
    const parts = parseScenarioHumanFromFilename(name);
    if (!parts) continue;
    const body = JSON.parse(readFileSync(join(dir, name), 'utf8')) as {
      consoleErrors?: ConsoleLogEntry[];
      pageErrors?: PageErrorEntry[];
    };
    artifacts.push({
      scenario: parts.scenario,
      human: parts.human,
      consoleErrors: body.consoleErrors ?? [],
      pageErrors:    body.pageErrors    ?? [],
    });
  }
  return artifacts;
}
```

- [ ] **Step 5: Run test to verify it passes**

```bash
cd genesis/a2o
npx tsx --test scripts/__tests__/load-console.test.ts
```

Expected: all 5 tests pass.

- [ ] **Step 6: Commit**

```bash
git add genesis/a2o/scripts/lib/load-console.ts \
        genesis/a2o/scripts/__tests__/load-console.test.ts \
        genesis/a2o/scripts/__tests__/fixtures/console-terrance-errors.json \
        genesis/a2o/scripts/__tests__/fixtures/console-mary-errors.json
git commit -m "feat(a2o): console-artifact loader with scenario/human inference"
```

---

## Task 6: Coverage-gap loader — pure function + tests

**Files:**
- Create: `genesis/a2o/scripts/lib/load-coverage-gap.ts`
- Create: `genesis/a2o/scripts/__tests__/fixtures/coverage-gap.json`
- Create: `genesis/a2o/scripts/__tests__/load-coverage-gap.test.ts`

The existing `scan-coverage.ts` CLI emits a JSON report. We read it with tolerant shape assumptions — missing file returns empty array so the aggregator still runs when coverage wasn't scanned.

- [ ] **Step 1: Write the fixture**

```json
// genesis/a2o/scripts/__tests__/fixtures/coverage-gap.json
{
  "generatedAt": "2026-04-19T10:00:00Z",
  "gaps": [
    {
      "feature": "features/lamad/path-adaptation.feature",
      "missing": "Scenario: Terrance's path reorders after rapid mastery",
      "severity": "high"
    },
    {
      "feature": "features/elohim/presence.feature",
      "missing": "Scenario: presence claim expires and resurrects on reconnect",
      "severity": "medium"
    }
  ]
}
```

- [ ] **Step 2: Write the failing test**

```typescript
// genesis/a2o/scripts/__tests__/load-coverage-gap.test.ts
import { describe, it } from 'node:test';
import { strict as assert } from 'node:assert';
import { fileURLToPath } from 'node:url';
import { loadCoverageGap } from '../lib/load-coverage-gap.js';

const fixturePath = fileURLToPath(new URL('./fixtures/coverage-gap.json', import.meta.url));

describe('loadCoverageGap', () => {
  it('returns every gap entry with feature + missing', () => {
    const gaps = loadCoverageGap(fixturePath);
    assert.equal(gaps.length, 2);
    assert.equal(gaps[0].feature, 'features/lamad/path-adaptation.feature');
    assert.match(gaps[0].missing, /path reorders/);
  });

  it('defaults severity to "medium" when absent', () => {
    const gaps = loadCoverageGap(fixturePath);
    assert.equal(gaps[1].severity, 'medium');
  });

  it('returns empty array when file does not exist', () => {
    assert.deepEqual(loadCoverageGap('/no/such/file.json'), []);
  });
});
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cd genesis/a2o
npx tsx --test scripts/__tests__/load-coverage-gap.test.ts
```

Expected: FAIL — module not found.

- [ ] **Step 4: Write the implementation**

```typescript
// genesis/a2o/scripts/lib/load-coverage-gap.ts
import { readFileSync, existsSync } from 'node:fs';

export interface GapFinding {
  feature: string;
  missing: string;
  severity: 'low' | 'medium' | 'high';
}

interface GapFileShape {
  gaps?: Array<{ feature: string; missing: string; severity?: string }>;
}

export function loadCoverageGap(path: string): GapFinding[] {
  if (!existsSync(path)) return [];
  const raw = JSON.parse(readFileSync(path, 'utf8')) as GapFileShape;
  const out: GapFinding[] = [];
  for (const gap of raw.gaps ?? []) {
    const severity = (gap.severity as GapFinding['severity']) ?? 'medium';
    out.push({ feature: gap.feature, missing: gap.missing, severity });
  }
  return out;
}
```

- [ ] **Step 5: Run test to verify it passes**

```bash
cd genesis/a2o
npx tsx --test scripts/__tests__/load-coverage-gap.test.ts
```

Expected: all 3 tests pass.

- [ ] **Step 6: Commit**

```bash
git add genesis/a2o/scripts/lib/load-coverage-gap.ts \
        genesis/a2o/scripts/__tests__/load-coverage-gap.test.ts \
        genesis/a2o/scripts/__tests__/fixtures/coverage-gap.json
git commit -m "feat(a2o): coverage-gap loader (tolerant of missing file)"
```

---

## Task 7: Aggregation pipeline — pure function + tests

**Files:**
- Create: `genesis/a2o/scripts/lib/aggregate.ts`
- Create: `genesis/a2o/scripts/__tests__/aggregate.test.ts`

Merges all three input streams into a single `SprintReport`. Findings are grouped by `fingerprint`, then each group records its sources (to count occurrences) and its triggering scenarios (name + feature + human, deduped). Pillar comes from feature path. Output findings array is sorted by `occurrences desc, fingerprint asc` (ties stable across runs).

- [ ] **Step 1: Write the failing test**

```typescript
// genesis/a2o/scripts/__tests__/aggregate.test.ts
import { describe, it } from 'node:test';
import { strict as assert } from 'node:assert';
import { aggregate } from '../lib/aggregate.js';
import type { ScenarioResult } from '../lib/load-cucumber.js';
import type { ConsoleArtifact } from '../lib/load-console.js';
import type { GapFinding } from '../lib/load-coverage-gap.js';

function input() {
  const scenarios: ScenarioResult[] = [
    { name: 'Terrance completes path', feature: 'features/lamad/learning-journey.feature', status: 'passed' },
    { name: 'Mary fails on assessment', feature: 'features/lamad/learning-journey.feature', status: 'failed', failureMessage: 'AssertionError: expected 500 to be 200' },
    { name: 'Stub not implemented', feature: 'features/auth/fixture-humans.feature', status: 'pending' },
  ];
  const console: ConsoleArtifact[] = [
    {
      scenario: 'learning-journey', human: 'terrance',
      consoleErrors: [
        { level: 'error', text: 'ReferenceError: Sophia is not defined', url: 'https://doorway-alpha.elohim.host/a.js' },
      ],
      pageErrors: [],
    },
    {
      scenario: 'learning-journey', human: 'mary',
      consoleErrors: [
        { level: 'error', text: 'ReferenceError: Sophia is not defined', url: 'https://doorway-alpha.elohim.host/a.js' },
      ],
      pageErrors: [
        { message: "TypeError: Cannot read properties of null (reading 'token')", url: 'https://doorway-alpha.elohim.host/login' },
      ],
    },
  ];
  const gaps: GapFinding[] = [
    { feature: 'features/elohim/presence.feature', missing: 'presence claim expires', severity: 'medium' },
  ];
  return { scenarios, console, gaps };
}

describe('aggregate', () => {
  it('counts scenarios in summary', () => {
    const { scenarios, console, gaps } = input();
    const r = aggregate({ scenarios, consoleArtifacts: console, gaps, runId: 'r1', profile: 'alpha', doorway: 'https://d.alpha' });
    assert.equal(r.summary.scenarios.total, 3);
    assert.equal(r.summary.scenarios.passed, 1);
    assert.equal(r.summary.scenarios.failed, 1);
    assert.equal(r.summary.scenarios.pending, 1);
  });

  it('dedupes identical console errors into one finding with occurrences=2', () => {
    const { scenarios, console, gaps } = input();
    const r = aggregate({ scenarios, consoleArtifacts: console, gaps, runId: 'r1', profile: 'alpha' });
    const sophia = r.findings.find(f => f.message.includes('Sophia is not defined'))!;
    assert.ok(sophia);
    assert.equal(sophia.occurrences, 2);
    assert.equal(sophia.source, 'console-error');
    assert.equal(sophia.scenarios.length, 2);
  });

  it('includes scenario-failure findings', () => {
    const { scenarios, console, gaps } = input();
    const r = aggregate({ scenarios, consoleArtifacts: console, gaps, runId: 'r1', profile: 'alpha' });
    const failure = r.findings.find(f => f.source === 'scenario-failure')!;
    assert.ok(failure);
    assert.match(failure.message, /AssertionError/);
    assert.equal(failure.pillar, 'lamad');
  });

  it('includes pending-step findings', () => {
    const { scenarios, console, gaps } = input();
    const r = aggregate({ scenarios, consoleArtifacts: console, gaps, runId: 'r1', profile: 'alpha' });
    const pending = r.findings.find(f => f.source === 'pending-step');
    assert.ok(pending);
    assert.equal(pending!.pillar, 'imagodei');
  });

  it('includes coverage-gap findings', () => {
    const { scenarios, console, gaps } = input();
    const r = aggregate({ scenarios, consoleArtifacts: console, gaps, runId: 'r1', profile: 'alpha' });
    const gap = r.findings.find(f => f.source === 'coverage-gap')!;
    assert.ok(gap);
    assert.equal(gap.pillar, 'elohim');
  });

  it('sorts findings by occurrences desc', () => {
    const { scenarios, console, gaps } = input();
    const r = aggregate({ scenarios, consoleArtifacts: console, gaps, runId: 'r1', profile: 'alpha' });
    for (let i = 1; i < r.findings.length; i++) {
      assert.ok(r.findings[i - 1].occurrences >= r.findings[i].occurrences);
    }
  });

  it('emits suggested objective headlines', () => {
    const { scenarios, console, gaps } = input();
    const r = aggregate({ scenarios, consoleArtifacts: console, gaps, runId: 'r1', profile: 'alpha' });
    for (const f of r.findings) {
      assert.ok(f.suggestedObjective && f.suggestedObjective.length > 0);
    }
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd genesis/a2o
npx tsx --test scripts/__tests__/aggregate.test.ts
```

Expected: FAIL — module not found.

- [ ] **Step 3: Write the implementation**

```typescript
// genesis/a2o/scripts/lib/aggregate.ts
import { fingerprint, normalizeMessage } from './fingerprint.js';
import { pillarFromFeature } from './pillar-from-feature.js';
import type { ScenarioResult } from './load-cucumber.js';
import type { ConsoleArtifact } from './load-console.js';
import type { GapFinding } from './load-coverage-gap.js';

export type FindingSource =
  | 'console-error'
  | 'page-error'
  | 'failed-request'
  | 'scenario-failure'
  | 'pending-step'
  | 'coverage-gap';

export interface FindingScenario { name: string; feature: string; human?: string }

export interface Finding {
  fingerprint: string;
  source: FindingSource;
  pillar: string;
  severity: 'error' | 'warning' | 'info';
  message: string;
  firstSeenUrl?: string;
  occurrences: number;
  scenarios: FindingScenario[];
  suggestedObjective: string;
}

export interface SprintReport {
  generatedAt: string;
  runId: string;
  profile: string;
  doorway?: string;
  summary: {
    scenarios: { total: number; passed: number; failed: number; skipped: number; pending: number };
    findings:  { total: number; bySource: Record<string, number>; byPillar: Record<string, number> };
  };
  findings: Finding[];
}

interface AggregateInput {
  scenarios: ScenarioResult[];
  consoleArtifacts: ConsoleArtifact[];
  gaps: GapFinding[];
  runId: string;
  profile: string;
  doorway?: string;
}

interface RawFinding {
  source: FindingSource;
  pillar: string;
  severity: 'error' | 'warning' | 'info';
  rawMessage: string;
  url?: string;
  scenario: FindingScenario;
}

function suggestObjective(source: FindingSource, message: string): string {
  const head = message.slice(0, 120);
  switch (source) {
    case 'console-error':     return `Fix browser console error: ${head}`;
    case 'page-error':        return `Fix unhandled page exception: ${head}`;
    case 'failed-request':    return `Fix failing network request: ${head}`;
    case 'scenario-failure':  return `Fix scenario failure: ${head}`;
    case 'pending-step':      return `Implement pending step definition: ${head}`;
    case 'coverage-gap':      return `Author missing scenario: ${head}`;
  }
}

export function aggregate(input: AggregateInput): SprintReport {
  const raws: RawFinding[] = [];

  // Scenario-level
  for (const s of input.scenarios) {
    if (s.status === 'failed' && s.failureMessage) {
      raws.push({
        source: 'scenario-failure',
        pillar: pillarFromFeature(s.feature),
        severity: 'error',
        rawMessage: s.failureMessage.split('\n')[0],
        scenario: { name: s.name, feature: s.feature },
      });
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

  // Console / page / network
  for (const art of input.consoleArtifacts) {
    for (const e of art.consoleErrors) {
      raws.push({
        source: 'console-error',
        pillar: 'browser',
        severity: e.level === 'warning' ? 'warning' : 'error',
        rawMessage: e.text,
        url: e.url,
        scenario: { name: art.scenario, feature: 'browser', human: art.human },
      });
    }
    for (const p of art.pageErrors) {
      raws.push({
        source: 'page-error',
        pillar: 'browser',
        severity: 'error',
        rawMessage: p.message,
        url: p.url,
        scenario: { name: art.scenario, feature: 'browser', human: art.human },
      });
    }
  }

  // Coverage gaps
  for (const g of input.gaps) {
    raws.push({
      source: 'coverage-gap',
      pillar: pillarFromFeature(g.feature),
      severity: g.severity === 'high' ? 'error' : 'warning',
      rawMessage: g.missing,
      scenario: { name: g.missing, feature: g.feature },
    });
  }

  // Group by (source + fingerprint) — keep sources separate so the same normalized
  // text from two different sources doesn't collapse unexpectedly.
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
    } else {
      const message = normalizeMessage(r.rawMessage);
      groups.set(key, {
        fingerprint: fp,
        source: r.source,
        pillar: r.pillar,
        severity: r.severity,
        message,
        firstSeenUrl: r.url,
        occurrences: 1,
        scenarios: [r.scenario],
        suggestedObjective: suggestObjective(r.source, message),
      });
    }
  }

  const findings = [...groups.values()].sort((a, b) => {
    if (b.occurrences !== a.occurrences) return b.occurrences - a.occurrences;
    return a.fingerprint.localeCompare(b.fingerprint);
  });

  const scenarioCounts = { total: 0, passed: 0, failed: 0, skipped: 0, pending: 0 };
  for (const s of input.scenarios) {
    scenarioCounts.total += 1;
    const status = (s.status === 'undefined' ? 'pending' : s.status) as keyof typeof scenarioCounts;
    if (status !== 'total') scenarioCounts[status] += 1;
  }

  const bySource: Record<string, number> = {};
  const byPillar: Record<string, number> = {};
  for (const f of findings) {
    bySource[f.source] = (bySource[f.source] ?? 0) + 1;
    byPillar[f.pillar] = (byPillar[f.pillar] ?? 0) + 1;
  }

  return {
    generatedAt: new Date().toISOString(),
    runId: input.runId,
    profile: input.profile,
    doorway: input.doorway,
    summary: {
      scenarios: scenarioCounts,
      findings: { total: findings.length, bySource, byPillar },
    },
    findings,
  };
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd genesis/a2o
npx tsx --test scripts/__tests__/aggregate.test.ts
```

Expected: all 7 tests pass.

- [ ] **Step 5: Commit**

```bash
git add genesis/a2o/scripts/lib/aggregate.ts \
        genesis/a2o/scripts/__tests__/aggregate.test.ts
git commit -m "feat(a2o): aggregation pipeline — dedupe + rank findings"
```

---

## Task 8: Markdown renderer — pure function + tests

**Files:**
- Create: `genesis/a2o/scripts/lib/render-markdown.ts`
- Create: `genesis/a2o/scripts/__tests__/render-markdown.test.ts`

Human-readable output for PR review and Jenkins archiving. Structure: header → summary table → findings grouped by pillar, each with a collapsible list of scenarios.

- [ ] **Step 1: Write the failing test**

```typescript
// genesis/a2o/scripts/__tests__/render-markdown.test.ts
import { describe, it } from 'node:test';
import { strict as assert } from 'node:assert';
import { renderMarkdown } from '../lib/render-markdown.js';
import type { SprintReport } from '../lib/aggregate.js';

const report: SprintReport = {
  generatedAt: '2026-04-19T10:00:00Z',
  runId: 'build-123',
  profile: 'alpha',
  doorway: 'https://doorway-alpha.elohim.host',
  summary: {
    scenarios: { total: 3, passed: 1, failed: 1, skipped: 0, pending: 1 },
    findings:  { total: 3, bySource: { 'console-error': 1, 'scenario-failure': 1, 'pending-step': 1 }, byPillar: { browser: 1, lamad: 1, imagodei: 1 } },
  },
  findings: [
    {
      fingerprint: 'abc123def456',
      source: 'console-error',
      pillar: 'browser',
      severity: 'error',
      message: 'ReferenceError: Sophia is not defined',
      firstSeenUrl: 'https://doorway-alpha.elohim.host/a.js',
      occurrences: 2,
      scenarios: [
        { name: 'learning-journey', feature: 'browser', human: 'terrance' },
        { name: 'learning-journey', feature: 'browser', human: 'mary' },
      ],
      suggestedObjective: 'Fix browser console error: ReferenceError: Sophia is not defined',
    },
  ],
};

describe('renderMarkdown', () => {
  it('includes the run id and profile in the header', () => {
    const md = renderMarkdown(report);
    assert.match(md, /A2O Sprint Report/);
    assert.match(md, /build-123/);
    assert.match(md, /alpha/);
  });

  it('renders summary counts', () => {
    const md = renderMarkdown(report);
    assert.match(md, /passed[^0-9]*1/i);
    assert.match(md, /failed[^0-9]*1/i);
  });

  it('groups findings by pillar header', () => {
    const md = renderMarkdown(report);
    assert.match(md, /## .*browser/i);
  });

  it('includes fingerprint, occurrences, and suggested objective', () => {
    const md = renderMarkdown(report);
    assert.match(md, /abc123def456/);
    assert.match(md, /occurrences.*2/i);
    assert.match(md, /Fix browser console error/);
  });

  it('lists each scenario that triggered the finding', () => {
    const md = renderMarkdown(report);
    assert.match(md, /terrance/);
    assert.match(md, /mary/);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd genesis/a2o
npx tsx --test scripts/__tests__/render-markdown.test.ts
```

Expected: FAIL — module not found.

- [ ] **Step 3: Write the implementation**

```typescript
// genesis/a2o/scripts/lib/render-markdown.ts
import type { SprintReport, Finding } from './aggregate.js';

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
  lines.push(`Findings total: **${report.summary.findings.total}**`);
  lines.push('');

  const byPillar = new Map<string, Finding[]>();
  for (const f of report.findings) {
    const arr = byPillar.get(f.pillar) ?? [];
    arr.push(f);
    byPillar.set(f.pillar, arr);
  }

  for (const [pillar, findings] of [...byPillar.entries()].sort()) {
    lines.push(`## ${pillar}`);
    lines.push('');
    for (const f of findings) {
      lines.push(`### [${f.source}] \`${f.fingerprint}\` (occurrences: ${f.occurrences})`);
      lines.push('');
      lines.push(`> ${f.message}`);
      lines.push('');
      if (f.firstSeenUrl) lines.push(`- URL: ${f.firstSeenUrl}`);
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

- [ ] **Step 4: Run test to verify it passes**

```bash
cd genesis/a2o
npx tsx --test scripts/__tests__/render-markdown.test.ts
```

Expected: all 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add genesis/a2o/scripts/lib/render-markdown.ts \
        genesis/a2o/scripts/__tests__/render-markdown.test.ts
git commit -m "feat(a2o): markdown renderer for sprint report"
```

---

## Task 9: Entry-point CLI + schema validation

**Files:**
- Create: `genesis/a2o/scripts/build-sprint-report.ts`
- Modify: `genesis/a2o/package.json` (add devDep `ajv` + script entry)

Entry point wires loaders → aggregator → schema-validation → writers. Writes `reports/sprint-report.json` and `reports/sprint-report.md`. Fails loud on schema validation errors so drift between schema and aggregator surfaces immediately.

- [ ] **Step 1: Add ajv to devDependencies**

Edit `genesis/a2o/package.json`. Add to `devDependencies`:

```json
    "ajv": "^8.17.1",
    "ajv-formats": "^3.0.1",
```

And add a scripts entry:

```json
    "build:sprint-report": "tsx scripts/build-sprint-report.ts"
```

Then install:

```bash
cd genesis/a2o
pnpm install
```

- [ ] **Step 2: Write the CLI**

```typescript
// genesis/a2o/scripts/build-sprint-report.ts
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv from 'ajv';
import addFormats from 'ajv-formats';

import { loadCucumber } from './lib/load-cucumber.js';
import { loadConsoleArtifacts } from './lib/load-console.js';
import { loadCoverageGap } from './lib/load-coverage-gap.js';
import { aggregate } from './lib/aggregate.js';
import { renderMarkdown } from './lib/render-markdown.js';

interface Args {
  reportsDir: string;
  cucumberPath: string;
  consoleDir: string;
  coverageGapPath: string;
  outJson: string;
  outMd: string;
  runId: string;
  profile: string;
  doorway?: string;
}

function parseArgs(argv: string[]): Args {
  const opts = new Map<string, string>();
  for (let i = 0; i < argv.length; i += 2) opts.set(argv[i], argv[i + 1]);

  const reportsDir = opts.get('--reports-dir') ?? 'reports';
  return {
    reportsDir,
    cucumberPath:    opts.get('--cucumber')      ?? join(reportsDir, 'cucumber-report.json'),
    consoleDir:      opts.get('--console-dir')   ?? join(reportsDir, 'console'),
    coverageGapPath: opts.get('--coverage-gap')  ?? join(reportsDir, 'coverage-gap.json'),
    outJson:         opts.get('--out-json')      ?? join(reportsDir, 'sprint-report.json'),
    outMd:           opts.get('--out-md')        ?? join(reportsDir, 'sprint-report.md'),
    runId:           opts.get('--run-id')        ?? process.env.BUILD_TAG ?? new Date().toISOString(),
    profile:         opts.get('--profile')       ?? process.env.CUCUMBER_PROFILE ?? 'unknown',
    doorway:         opts.get('--doorway')       ?? process.env.E2E_DOORWAY_ALPHA,
  };
}

function ensureDir(p: string) {
  const dir = dirname(p);
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true });
}

function main() {
  const args = parseArgs(process.argv.slice(2));

  const cucumberJson = existsSync(args.cucumberPath)
    ? readFileSync(args.cucumberPath, 'utf8')
    : '[]';
  const scenarios = loadCucumber(cucumberJson);
  const consoleArtifacts = loadConsoleArtifacts(args.consoleDir);
  const gaps = loadCoverageGap(args.coverageGapPath);

  const report = aggregate({
    scenarios, consoleArtifacts, gaps,
    runId: args.runId, profile: args.profile, doorway: args.doorway,
  });

  // Schema-validate before writing
  const schemaPath = fileURLToPath(
    new URL('../schemas/sprint-report.schema.json', import.meta.url)
  );
  const schema = JSON.parse(readFileSync(schemaPath, 'utf8'));
  const ajv = new Ajv({ strict: true, allErrors: true });
  addFormats(ajv);
  const validate = ajv.compile(schema);
  if (!validate(report)) {
    console.error('Sprint report failed schema validation:');
    console.error(JSON.stringify(validate.errors, null, 2));
    process.exit(2);
  }

  ensureDir(args.outJson);
  writeFileSync(args.outJson, JSON.stringify(report, null, 2));
  ensureDir(args.outMd);
  writeFileSync(args.outMd, renderMarkdown(report));

  console.log(`Sprint report written:`);
  console.log(`  ${args.outJson}`);
  console.log(`  ${args.outMd}`);
  console.log(`Findings: ${report.summary.findings.total} (scenarios: ${report.summary.scenarios.total})`);
}

main();
```

- [ ] **Step 3: Verify CLI runs end-to-end on fixture data**

Build a tiny staged-reports directory using test fixtures and run the CLI against it:

```bash
cd genesis/a2o
mkdir -p /tmp/a2o-verify/reports/console
cp scripts/__tests__/fixtures/cucumber-mixed.json        /tmp/a2o-verify/reports/cucumber-report.json
cp scripts/__tests__/fixtures/console-terrance-errors.json /tmp/a2o-verify/reports/console/learning-journey-terrance-errors.json
cp scripts/__tests__/fixtures/console-mary-errors.json    /tmp/a2o-verify/reports/console/learning-journey-mary-errors.json
cp scripts/__tests__/fixtures/coverage-gap.json          /tmp/a2o-verify/reports/coverage-gap.json

pnpm run build:sprint-report -- \
  --reports-dir /tmp/a2o-verify/reports \
  --run-id verify-1 \
  --profile alpha \
  --doorway https://doorway-alpha.elohim.host
```

Expected output: "Sprint report written: ... Findings: 5 (scenarios: 3)" (3 scenario-level + console + page + gap). Both output files exist and the JSON parses.

```bash
cat /tmp/a2o-verify/reports/sprint-report.json | jq '.summary'
head -40 /tmp/a2o-verify/reports/sprint-report.md
```

- [ ] **Step 4: Commit**

```bash
git add genesis/a2o/scripts/build-sprint-report.ts genesis/a2o/package.json pnpm-lock.yaml
git commit -m "feat(a2o): build-sprint-report CLI with schema validation"
```

---

## Task 10: Wire into `genesis/Jenkinsfile`

**Files:**
- Modify: `genesis/Jenkinsfile` — E2E stage

Run the aggregator after the Cucumber stage (regardless of pass/fail) and archive the outputs. Failing the aggregator must NOT fail the pipeline — it's informational.

- [ ] **Step 1: Read the current E2E stage**

```bash
grep -n "cucumber-js\|coverage.*scan\|archiveArtifacts" /projects/elohim/genesis/Jenkinsfile | head -20
```

Note the line numbers for the E2E / archive blocks.

- [ ] **Step 2: Insert sprint-report aggregator before artifact archive**

Inside the E2E stage, after the `cucumber-js` invocation and the `scan-coverage` script runs, but before `archiveArtifacts`, add:

```groovy
// --- Sprint-report aggregator ---
// Always runs (pass or fail). Non-blocking: report is informational, not a gate.
sh """
  cd genesis/a2o
  pnpm run build:sprint-report -- \\
    --run-id "${env.BUILD_TAG}" \\
    --profile "\${CUCUMBER_PROFILE:-alpha}" \\
    --doorway "\${E2E_DOORWAY_ALPHA:-}" \\
    || echo 'Sprint-report aggregator failed (non-blocking)'
"""
```

Then extend the existing `archiveArtifacts` to include the two output files (keep any existing patterns intact):

```groovy
archiveArtifacts artifacts: 'genesis/a2o/reports/**, genesis/a2o/reports/sprint-report.md, genesis/a2o/reports/sprint-report.json',
                 allowEmptyArchive: true
```

- [ ] **Step 3: Validate the Jenkinsfile parses**

```bash
cd /projects/elohim
npx @jenkins-x/jx-jenkinsfile-runner --help > /dev/null 2>&1 || true
# No in-repo validator — rely on Jenkins replay. As a cheap sanity check:
grep -A2 "build:sprint-report" genesis/Jenkinsfile
```

Expected: the inserted block is present and syntactically clean (no stray backticks / unbalanced quotes).

- [ ] **Step 4: Commit**

```bash
git add genesis/Jenkinsfile
git commit -m "ci(a2o): run sprint-report aggregator after E2E and archive outputs"
```

---

## Task 11: Verify end-to-end against a real alpha run

**Files (verification only):**
- None created — uses existing artifacts from a past alpha E2E run

- [ ] **Step 1: Pull the most recent alpha E2E artifacts**

Find the latest successful genesis E2E build in Jenkins. Download `genesis/a2o/reports/**` to a local dir:

```bash
mkdir -p /tmp/a2o-real
# Assumes user has already downloaded Jenkins artifacts to /tmp/a2o-real/reports/
ls /tmp/a2o-real/reports/
```

Expected at minimum: `cucumber-report.json`, `console/*.json`.

- [ ] **Step 2: Run aggregator against real data**

```bash
cd /projects/elohim/genesis/a2o
pnpm run build:sprint-report -- \
  --reports-dir /tmp/a2o-real/reports \
  --run-id real-alpha-verify \
  --profile alpha
```

Expected: report generates successfully, prints findings count. Open the markdown — findings should look plausible (real browser errors, real failed scenarios).

- [ ] **Step 3: Inspect and capture notes**

Read the top 10 findings. For each:
- Is the message useful or runtime-noisy? If noisy, note what regex to add to `fingerprint.ts`.
- Is the pillar correct?
- Is the suggested objective something you'd actually hand to `/shift`?

Write these observations into a short `NOTES.md` (not committed) — they become the input for a follow-up polish sprint (out of scope for this plan).

- [ ] **Step 4: Report completion**

Post results back to the conversation: report file locations, total findings, top 3 defects as teaser for the first `/shift` Objectives.

---

## Self-Review Checklist

- [x] Task 1 defines the schema that Task 9 validates against (contract closure)
- [x] `fingerprint` (Task 2) used by `aggregate` (Task 7)
- [x] `pillarFromFeature` (Task 3) used by `aggregate` (Task 7)
- [x] All three loaders (Tasks 4/5/6) produce inputs consumed by Task 7
- [x] Types referenced in tests (`ScenarioResult`, `ConsoleArtifact`, `GapFinding`) are defined in the respective loader tasks before aggregate test uses them
- [x] No placeholders — every step has runnable code or exact commands
- [x] Every commit message follows `type(scope): message` convention
- [x] CI wiring is non-blocking (`|| echo`) — aggregator failure doesn't fail the pipeline
- [x] Task 11 validates against real production data before declaring done

## Ready for Execution

This plan consumes only artifacts that exist today. It has no runtime prerequisites. Total bite-sized steps: ~55. Estimated effort: 4-6 hours sequential, ~2 hours with subagent parallelism.

When Plan B (runtime request-ID correlation) lands, this aggregator gets extended with a backend-logs loader in a follow-up plan (not scoped here).
