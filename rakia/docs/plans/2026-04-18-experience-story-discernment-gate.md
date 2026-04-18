# Experience-Story Discernment Gate — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the v1 mechanical discernment gate as a pure, fixture-mockable TypeScript function in `elohim-service`, alongside the lamad manifest entries for `experience-story` / `experience-moment` contentTypes, so that later plans (a2o integration, storage projection, Holochain attestation) have a tested interface to call into.

**Architecture:** Pure function `discernMechanical(input, momentEntryHash) → StoryPointTag | null` implementing the 7 rules from spec §7.3. Zero dependencies on Holochain, storage, or HTTP. Tested with Vitest using explicit fixtures for each of the seven valences and the steady-state case. Schema foundations (two metadata JSON schemas, two contentTypes, one signal definition) land in the lamad manifest; `pnpm run lamad:codegen` regenerates TypeScript types consumed by both elohim-app and genesis/seeder.

**Tech Stack:** TypeScript, Vitest, JSON Schema 2020-12, pnpm workspaces. No Rust, no Holochain, no HTTP in this plan.

**Spec reference:** `genesis/docs/superpowers/specs/2026-04-18-experience-story-epr-design.md` (commit `09563f63`).

**Source-of-truth declaration (for P2P audit):** This plan introduces **zero** new DHT entry types and **zero** new storage tables / migrations. The two JSON Schemas in Tasks 1–2 are app-schema *validation* schemas attached to the existing notarized `ContentNode` entry type via the lamad manifest — the manifest is the source of truth, projection lives in the existing storage layer untouched. All Tier 1 (notarized), Tier 2 (derived, plus the existing notarized `EconomicEvent`), and Tier 3 (agent-scoped, persona source-chain) classifications were completed in the spec §3 and the §10 anti-pattern check; this plan ships only TypeScript types and a pure-function discernment gate — no Rust, no Holochain, no SQL, no operational tables.

**Why this plan lives in `rakia/docs/plans/`:** The discernment gate is the seam between a2o pipeline runs (today's Jenkins-driven CI) and the elohim economic substrate. As we close the gap toward rakia replacing Jenkins, plans that touch the build/test/attestation surface live in rakia so they travel with that submodule's evolution. A reference to this plan lives in `genesis/docs/superpowers/plans/` so the meta/automation index still surfaces it.

---

## Preflight

- [ ] **Step P.1: Confirm branch and worktree**

Run:
```bash
cd /projects/elohim && git status -sb && git log --oneline -3
```
Expected: on branch `dev` with recent spec commits (`09563f63`, `ee55de55`, `56ed4392`) visible. If not on `dev` or uncommitted work, stop and confer with user.

- [ ] **Step P.2: Confirm elohim-service tests pass baseline**

Run:
```bash
cd /projects/elohim/app/elohim-library/projects/elohim-service && pnpm install && pnpm test
```
Expected: all existing tests pass before we add new ones. Note the baseline count.

---

## Task 1: Add `experience-story` metadata schema

**Files:**
- Create: `elohim/sdk/domains/lamad/schemas/experience-story-metadata.schema.json`

- [ ] **Step 1.1: Create the schema**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://elohim.host/lamad/schemas/experience-story-metadata.schema.json",
  "title": "ExperienceStoryMetadata",
  "description": "App-schema metadata for the experience-story contentType. The subject/role/feature identity is realized in the link graph; this object carries discoverability and index hints.",
  "type": "object",
  "additionalProperties": false,
  "required": ["subjectRef", "roleRef", "featureRef"],
  "properties": {
    "subjectRef": {
      "type": "string",
      "description": "CID or slug reference to the subject ContentNode (human or collective).",
      "pattern": "^(human|collective):[a-z0-9-]+$|^baf[a-z0-9]+$"
    },
    "roleRef": {
      "type": "string",
      "description": "CID or slug reference to the role ContentNode.",
      "pattern": "^role:[a-z0-9-]+$|^baf[a-z0-9]+$"
    },
    "featureRef": {
      "type": "string",
      "description": "CID or slug reference to the feature ContentNode (gherkin feature canonical entry).",
      "pattern": "^feature:[a-z0-9-]+$|^baf[a-z0-9]+$"
    },
    "aliasEpr": {
      "type": "string",
      "description": "Optional human-readable EPR alias, e.g. epr:experience-story/matthew-manager/as-entrepreneur/learning-journey.",
      "pattern": "^epr:experience-story/[a-z0-9-]+/[a-z0-9-]+/[a-z0-9-]+$"
    }
  }
}
```

- [ ] **Step 1.2: Commit**

```bash
cd /projects/elohim && git add elohim/sdk/domains/lamad/schemas/experience-story-metadata.schema.json
git commit -m "feat(lamad): add experience-story metadata schema"
```

---

## Task 2: Add `experience-moment` metadata schema

**Files:**
- Create: `elohim/sdk/domains/lamad/schemas/experience-moment-metadata.schema.json`

- [ ] **Step 2.1: Create the schema**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://elohim.host/lamad/schemas/experience-moment-metadata.schema.json",
  "title": "ExperienceMomentMetadata",
  "description": "App-schema metadata for the experience-moment contentType — one persona's first-person recording of one scenario at one moment in time.",
  "type": "object",
  "additionalProperties": false,
  "required": [
    "subjectRef", "roleRef", "featureRef",
    "scenarioName", "scenarioUri", "status",
    "durationMs", "commit", "runId", "computeFingerprint"
  ],
  "properties": {
    "subjectRef": { "type": "string" },
    "roleRef": { "type": "string" },
    "featureRef": { "type": "string" },
    "scenarioName": { "type": "string" },
    "scenarioUri": { "type": "string" },
    "scenarioLine": { "type": "integer", "minimum": 1 },
    "scenarioTags": { "type": "array", "items": { "type": "string" } },
    "status": { "type": "string", "enum": ["passed", "failed", "pending", "skipped"] },
    "durationMs": { "type": "integer", "minimum": 0 },
    "commit": { "type": "string", "pattern": "^[a-f0-9]{7,40}$" },
    "runId": { "type": "string" },
    "computeFingerprint": {
      "type": "string",
      "description": "Format: {pod}:{deviceArchetype}:{archetypeRevisionHash}",
      "pattern": "^[a-z0-9-]+:[a-z0-9-]+:[a-f0-9]+$"
    },
    "errorClass": {
      "type": "string",
      "description": "Present when status=failed; classifies the error (e.g., 'AssertionError/timeout', 'NetworkError/503')."
    },
    "sidecarArtifacts": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "cucumber": { "type": "string" },
        "observation": { "type": "string" },
        "screenshot": { "type": "string" },
        "trace": { "type": "string" },
        "console": { "type": "string" }
      }
    },
    "relatedExperienceStory": { "type": "string" }
  }
}
```

- [ ] **Step 2.2: Commit**

```bash
cd /projects/elohim && git add elohim/sdk/domains/lamad/schemas/experience-moment-metadata.schema.json
git commit -m "feat(lamad): add experience-moment metadata schema"
```

---

## Task 3: Register both contentTypes in the lamad manifest

**Files:**
- Modify: `elohim/sdk/domains/lamad/manifest.json` (contentTypes section)

- [ ] **Step 3.1: Read the existing contentTypes section**

Run:
```bash
cd /projects/elohim && grep -n '"contentTypes"' elohim/sdk/domains/lamad/manifest.json | head -3
```
Note the opening line number. Open the file at that line with the Read tool and locate the insertion point alphabetically between existing entries (experience-* sits between 'exercise' and 'feature').

- [ ] **Step 3.2: Insert the two new contentType entries**

Insert after the `"exercise"` entry's closing `},` and before the `"feature"` entry:

```json
    "experience-story": {
      "description": "A durable narrative junction representing (subject, role, feature) — the persona's experience with a feature in a given role. Accumulates value via :story-point attestations over time.",
      "metadataSchema": { "$ref": "./schemas/experience-story-metadata.schema.json" },
      "coupling": {
        "knowledge": {
          "relationships": {
            "HAS_SUBJECT": ["human", "collective"],
            "IN_ROLE": ["role"],
            "EXERCISES": ["feature"]
          }
        },
        "value": {
          "onAttest": { "action": "produce", "resourceConformsTo": "experience-evidence" }
        },
        "governance": { "defaultReach": "self", "governanceModel": "steward-consent" },
        "claims": [{ "asserts": "witnessed-evidence", "validityHorizon": "P90D" }]
      }
    },
    "experience-moment": {
      "description": "One persona's first-person recording of one scenario at one moment. Agent-scoped (private source-chain entry). Projected locally per persona; never gossipped.",
      "metadataSchema": { "$ref": "./schemas/experience-moment-metadata.schema.json" },
      "coupling": {
        "knowledge": { "relationships": { "ATTESTS_TO": ["experience-story"] } },
        "value": {},
        "governance": { "defaultReach": "self", "governanceModel": "agent-scoped" },
        "claims": []
      }
    },
```

- [ ] **Step 3.3: Validate the manifest parses**

Run:
```bash
cd /projects/elohim && node -e "JSON.parse(require('node:fs').readFileSync('elohim/sdk/domains/lamad/manifest.json', 'utf-8')); console.log('OK')"
```
Expected: `OK`. If parse error, fix the JSON syntax (likely a trailing comma).

- [ ] **Step 3.4: Commit**

```bash
git add elohim/sdk/domains/lamad/manifest.json
git commit -m "feat(lamad): register experience-story and experience-moment contentTypes"
```

---

## Task 4: Register the `experience-attestation` signal

**Files:**
- Modify: `elohim/sdk/domains/lamad/manifest.json` (signals section)

- [ ] **Step 4.1: Locate the signals section and add the new entry**

Find the `"signals": {` object (per exploration, around line 769). Add this entry alphabetically:

```json
    "experience-attestation": {
      "substrateSignal": "attention",
      "economicAction": "produce",
      "description": "A :story-point attestation minted by the discernment gate when meaningful evidence is recognized. Emitted alongside the Holochain Link; dual-encoded as an EconomicEvent in shefa."
    },
```

- [ ] **Step 4.2: Validate JSON**

```bash
cd /projects/elohim && node -e "JSON.parse(require('node:fs').readFileSync('elohim/sdk/domains/lamad/manifest.json', 'utf-8')); console.log('OK')"
```
Expected: `OK`.

- [ ] **Step 4.3: Commit**

```bash
git add elohim/sdk/domains/lamad/manifest.json
git commit -m "feat(lamad): register experience-attestation signal for discernment dual-emission"
```

---

## Task 5: Regenerate manifest types and verify

**Files:**
- Regenerated: `app/elohim-app/src/app/lamad/generated/manifest-types.ts`
- Regenerated: `genesis/seeder/src/generated/manifest-types.ts`

- [ ] **Step 5.1: Run codegen**

Run:
```bash
cd /projects/elohim && pnpm run lamad:codegen
```
Expected: completes without error; emits 5 files × 2 locations.

- [ ] **Step 5.2: Verify new contentTypes appear in generated types**

```bash
grep -E "experience-story|experience-moment" app/elohim-app/src/app/lamad/generated/manifest-types.ts
```
Expected: both strings appear in the `LAMAD_CONTENT_TYPES` const array.

- [ ] **Step 5.3: Commit generated files**

```bash
git add app/elohim-app/src/app/lamad/generated/ genesis/seeder/src/generated/
git commit -m "chore(lamad): regenerate manifest types for experience-story and experience-moment"
```

---

## Task 6: Create discernment module scaffolding

**Files:**
- Create: `app/elohim-library/projects/elohim-service/src/discernment/types.ts`
- Create: `app/elohim-library/projects/elohim-service/src/discernment/index.ts`

- [ ] **Step 6.1: Create the types module**

`app/elohim-library/projects/elohim-service/src/discernment/types.ts`:

```typescript
/**
 * Types for the v1 mechanical discernment gate.
 *
 * The gate is a pure function; these types define its input and output.
 * Backed by spec 2026-04-18-experience-story-epr-design.md §5–§7.
 */

export type Valence =
  | 'progress'
  | 'discovery'
  | 'regression'
  | 'validation'
  | 'witness'
  | 'refinement'
  | 'confirmation';

export type Magnitude = 'small' | 'meaningful' | 'significant';

export type EvidenceType =
  | 'first-pass-green'
  | 'novel-failure-class'
  | 'known-cause-recurrence'
  | 'failure-mode-confirmed'
  | 'recovery'
  | 'cross-fingerprint-attestation'
  | 'evidence-enriched';

export type ScenarioStatus = 'passed' | 'failed' | 'pending' | 'skipped';

export type SidecarName = 'cucumber' | 'observation' | 'screenshot' | 'trace' | 'console';

/** Format: {pod}:{deviceArchetype}:{archetypeRevisionHash}. */
export type ComputeFingerprint = string;

/**
 * A moment as passed to the discerner. Mirrors the experience-moment
 * frontmatter (spec §6.2). Sidecar values are blob references; their
 * presence (keys) is what refinement rule checks.
 */
export interface ExperienceMomentPayload {
  recordedAt: string;
  subjectRef: string;
  roleRef: string;
  featureRef: string;
  scenarioName: string;
  scenarioUri: string;
  scenarioLine?: number;
  scenarioTags: readonly string[];
  status: ScenarioStatus;
  durationMs: number;
  commit: string;
  runId: string;
  computeFingerprint: ComputeFingerprint;
  errorClass?: string;
  sidecarArtifacts: Partial<Record<SidecarName, string>>;
}

/** A prior attestation, as queried from storage projection. */
export interface PriorAttestation {
  momentEntryHash: string;
  status: ScenarioStatus;
  valence: Valence;
  magnitude: Magnitude;
  evidenceType: EvidenceType;
  computeFingerprint: ComputeFingerprint;
  errorClass?: string;
  durationMs: number;
  sidecarArtifactNames: readonly SidecarName[];
}

/** The input bundle the gate receives. */
export interface DiscernmentInput {
  moment: ExperienceMomentPayload;
  priors: {
    /** Most recent attestation on this experience-story from ANY compute fingerprint. */
    latestAny?: PriorAttestation;
    /** Most recent attestation from THIS moment's compute fingerprint. */
    latestSameFingerprint?: PriorAttestation;
    /** Every error class ever attested on this experience-story. */
    knownErrorClasses: ReadonlySet<string>;
  };
}

/** The gate's output — attached to the Holochain Link and mirrored on the EconomicEvent. */
export interface StoryPointTag {
  v: 1;
  valence: Valence;
  magnitude: Magnitude;
  evidenceType: EvidenceType;
  computeFingerprint: ComputeFingerprint;
  runId: string;
  commit: string;
  momentEntryHash: string;
  discernerId: 'discernment-service-v1-mechanical';
}
```

- [ ] **Step 6.2: Create the barrel export**

`app/elohim-library/projects/elohim-service/src/discernment/index.ts`:

```typescript
export * from './types.js';
export { discernMechanical } from './mechanical-discerner.js';
```

(The `mechanical-discerner.js` import will resolve once Task 7 lands; the file does not compile standalone until then — that's fine, it's checked in Task 12's final type-check step.)

- [ ] **Step 6.3: Commit**

```bash
git add app/elohim-library/projects/elohim-service/src/discernment/
git commit -m "feat(elohim-service): scaffold discernment module types"
```

---

## Task 7: Rule 1 — first-pass-green (progress, no prior)

**Files:**
- Create: `app/elohim-library/projects/elohim-service/src/discernment/mechanical-discerner.ts`
- Create: `app/elohim-library/projects/elohim-service/src/discernment/mechanical-discerner.spec.ts`
- Create: `app/elohim-library/projects/elohim-service/src/discernment/fixtures.ts`

- [ ] **Step 7.1: Write the fixture helper**

`app/elohim-library/projects/elohim-service/src/discernment/fixtures.ts`:

```typescript
import type {
  ExperienceMomentPayload,
  PriorAttestation,
  SidecarName,
} from './types.js';

export function momentFixture(
  overrides: Partial<ExperienceMomentPayload> = {},
): ExperienceMomentPayload {
  return {
    recordedAt: '2026-04-18T14:32:11Z',
    subjectRef: 'human:matthew-manager',
    roleRef: 'role:as-entrepreneur',
    featureRef: 'feature:learning-journey',
    scenarioName: 'Welcome flow loads in under 2s',
    scenarioUri: 'features/lamad/learning-journey.feature',
    scenarioLine: 47,
    scenarioTags: ['@e2e', '@lamad'],
    status: 'passed',
    durationMs: 1842,
    commit: 'abc123d',
    runId: 'pipeline-42',
    computeFingerprint: 'matthew-alpha:device-family-node-base:abc123',
    sidecarArtifacts: { cucumber: 'blob:bafkrei-cucumber/xyz.json' },
    ...overrides,
  };
}

export function priorFixture(
  overrides: Partial<PriorAttestation> = {},
): PriorAttestation {
  return {
    momentEntryHash: 'uhCEk-prior-moment-hash',
    status: 'passed',
    valence: 'progress',
    magnitude: 'meaningful',
    evidenceType: 'first-pass-green',
    computeFingerprint: 'matthew-alpha:device-family-node-base:abc123',
    durationMs: 2100,
    sidecarArtifactNames: ['cucumber'] as readonly SidecarName[],
    ...overrides,
  };
}
```

- [ ] **Step 7.2: Write the failing test for rule 1**

`app/elohim-library/projects/elohim-service/src/discernment/mechanical-discerner.spec.ts`:

```typescript
import { describe, expect, it } from 'vitest';

import { discernMechanical } from './mechanical-discerner.js';
import { momentFixture } from './fixtures.js';

describe('discernMechanical — rule 1 (first-pass-green)', () => {
  it('mints progress/meaningful/first-pass-green for a passing moment with no prior', () => {
    const moment = momentFixture({ status: 'passed' });

    const tag = discernMechanical(
      {
        moment,
        priors: { knownErrorClasses: new Set<string>() },
      },
      'uhCEk-moment-hash',
    );

    expect(tag).not.toBeNull();
    expect(tag).toMatchObject({
      v: 1,
      valence: 'progress',
      magnitude: 'meaningful',
      evidenceType: 'first-pass-green',
      computeFingerprint: moment.computeFingerprint,
      runId: moment.runId,
      commit: moment.commit,
      momentEntryHash: 'uhCEk-moment-hash',
      discernerId: 'discernment-service-v1-mechanical',
    });
  });
});
```

- [ ] **Step 7.3: Run the test to verify it fails**

```bash
cd /projects/elohim/app/elohim-library/projects/elohim-service && pnpm exec vitest run src/discernment/mechanical-discerner.spec.ts
```
Expected: FAIL with module-not-found or discernMechanical-undefined.

- [ ] **Step 7.4: Write minimal implementation for rule 1**

`app/elohim-library/projects/elohim-service/src/discernment/mechanical-discerner.ts`:

```typescript
import type {
  DiscernmentInput,
  EvidenceType,
  Magnitude,
  StoryPointTag,
  Valence,
} from './types.js';

function mkTag(
  input: DiscernmentInput,
  momentEntryHash: string,
  valence: Valence,
  magnitude: Magnitude,
  evidenceType: EvidenceType,
): StoryPointTag {
  return {
    v: 1,
    valence,
    magnitude,
    evidenceType,
    computeFingerprint: input.moment.computeFingerprint,
    runId: input.moment.runId,
    commit: input.moment.commit,
    momentEntryHash,
    discernerId: 'discernment-service-v1-mechanical',
  };
}

/**
 * v1 mechanical discernment gate. Pure function.
 * See spec §7.3 for rule ordering and rationale.
 */
export function discernMechanical(
  input: DiscernmentInput,
  momentEntryHash: string,
): StoryPointTag | null {
  const { moment, priors } = input;

  // Rule 1 — first-pass-green
  if (moment.status === 'passed' && !priors.latestAny) {
    return mkTag(input, momentEntryHash, 'progress', 'meaningful', 'first-pass-green');
  }

  return null;
}
```

- [ ] **Step 7.5: Run the test to verify it passes**

```bash
pnpm exec vitest run src/discernment/mechanical-discerner.spec.ts
```
Expected: 1 passed.

- [ ] **Step 7.6: Commit**

```bash
cd /projects/elohim && git add app/elohim-library/projects/elohim-service/src/discernment/
git commit -m "feat(elohim-service): discernment rule 1 — first-pass-green"
```

---

## Task 8: Rule 2 — discovery vs regression (failed after prior-passed)

**Files:**
- Modify: `app/elohim-library/projects/elohim-service/src/discernment/mechanical-discerner.ts`
- Modify: `app/elohim-library/projects/elohim-service/src/discernment/mechanical-discerner.spec.ts`

- [ ] **Step 8.1: Add two failing tests**

Append to the spec file inside a new `describe` block:

```typescript
import { priorFixture } from './fixtures.js';

describe('discernMechanical — rule 2 (failed after prior-passed)', () => {
  it('mints discovery/meaningful/novel-failure-class when error class is new', () => {
    const moment = momentFixture({
      status: 'failed',
      errorClass: 'AssertionError/timeout',
    });

    const tag = discernMechanical(
      {
        moment,
        priors: {
          latestAny: priorFixture({ status: 'passed' }),
          knownErrorClasses: new Set<string>(),
        },
      },
      'uhCEk-moment-hash',
    );

    expect(tag).toMatchObject({
      valence: 'discovery',
      magnitude: 'meaningful',
      evidenceType: 'novel-failure-class',
    });
  });

  it('mints regression/meaningful/known-cause-recurrence when error class was seen before', () => {
    const moment = momentFixture({
      status: 'failed',
      errorClass: 'NetworkError/503',
    });

    const tag = discernMechanical(
      {
        moment,
        priors: {
          latestAny: priorFixture({ status: 'passed' }),
          knownErrorClasses: new Set(['NetworkError/503']),
        },
      },
      'uhCEk-moment-hash',
    );

    expect(tag).toMatchObject({
      valence: 'regression',
      magnitude: 'meaningful',
      evidenceType: 'known-cause-recurrence',
    });
  });
});
```

- [ ] **Step 8.2: Run tests — verify two new tests fail**

```bash
pnpm exec vitest run src/discernment/mechanical-discerner.spec.ts
```
Expected: 1 passed (rule 1), 2 failed (rule 2a/2b).

- [ ] **Step 8.3: Add rule 2 to the discerner**

In `mechanical-discerner.ts`, insert BEFORE `return null;`:

```typescript
  // Rule 2 — failed after prior-passed → discovery (novel) or regression (known)
  if (moment.status === 'failed' && priors.latestAny?.status === 'passed') {
    const isNovel =
      !moment.errorClass || !priors.knownErrorClasses.has(moment.errorClass);
    return mkTag(
      input,
      momentEntryHash,
      isNovel ? 'discovery' : 'regression',
      'meaningful',
      isNovel ? 'novel-failure-class' : 'known-cause-recurrence',
    );
  }
```

- [ ] **Step 8.4: Run tests to verify they pass**

```bash
pnpm exec vitest run src/discernment/mechanical-discerner.spec.ts
```
Expected: 3 passed.

- [ ] **Step 8.5: Commit**

```bash
cd /projects/elohim && git add app/elohim-library/projects/elohim-service/src/discernment/
git commit -m "feat(elohim-service): discernment rule 2 — discovery vs regression"
```

---

## Task 9: Rule 3 — validation (@validates-failure-mode, failed)

**Files:**
- Modify: `app/elohim-library/projects/elohim-service/src/discernment/mechanical-discerner.ts`
- Modify: `app/elohim-library/projects/elohim-service/src/discernment/mechanical-discerner.spec.ts`

- [ ] **Step 9.1: Add failing test**

Append new describe block:

```typescript
describe('discernMechanical — rule 3 (validation)', () => {
  it('mints validation/meaningful/failure-mode-confirmed when a @validates-failure-mode scenario fails and there is no prior-passed attestation', () => {
    const moment = momentFixture({
      status: 'failed',
      scenarioTags: ['@e2e', '@lamad', '@validates-failure-mode'],
      errorClass: 'ExpectedFailure/unauthorized',
    });

    const tag = discernMechanical(
      {
        moment,
        priors: { knownErrorClasses: new Set(['ExpectedFailure/unauthorized']) },
      },
      'uhCEk-moment-hash',
    );

    expect(tag).toMatchObject({
      valence: 'validation',
      magnitude: 'meaningful',
      evidenceType: 'failure-mode-confirmed',
    });
  });

  it('yields to rule 2 when a @validates-failure-mode scenario fails AFTER a prior-passed attestation (the validation itself regressed)', () => {
    // Per spec §7.3, rule 2 is listed before rule 3: a prior-passed attestation
    // followed by a failure is classified as discovery/regression even when the
    // scenario is tagged @validates-failure-mode, because the validation lane
    // has itself moved. This is a deliberate edge case.
    const moment = momentFixture({
      status: 'failed',
      scenarioTags: ['@e2e', '@validates-failure-mode'],
      errorClass: 'AssertionError/timeout',
    });

    const tag = discernMechanical(
      {
        moment,
        priors: {
          latestAny: priorFixture({ status: 'passed' }),
          knownErrorClasses: new Set<string>(),
        },
      },
      'uhCEk-moment-hash',
    );

    expect(tag).toMatchObject({ valence: 'discovery' });
  });
});
```

- [ ] **Step 9.2: Run — verify first test fails, second passes (rule 2 handles it)**

```bash
pnpm exec vitest run src/discernment/mechanical-discerner.spec.ts
```
Expected: 4 passed, 1 failed (the validation test).

- [ ] **Step 9.3: Add rule 3 to the discerner**

In `mechanical-discerner.ts`, insert AFTER rule 2's block (order matters — rule 2's prior-passed check runs first, this catches the no-prior-passed case):

```typescript
  // Rule 3 — @validates-failure-mode scenario failing with no prior-passed attestation
  // (the failure-mode is confirmed as currently active and expected)
  if (moment.status === 'failed' && moment.scenarioTags.includes('@validates-failure-mode')) {
    return mkTag(input, momentEntryHash, 'validation', 'meaningful', 'failure-mode-confirmed');
  }
```

- [ ] **Step 9.4: Run tests to verify all pass**

```bash
pnpm exec vitest run src/discernment/mechanical-discerner.spec.ts
```
Expected: 5 passed.

- [ ] **Step 9.5: Commit**

```bash
cd /projects/elohim && git add app/elohim-library/projects/elohim-service/src/discernment/
git commit -m "feat(elohim-service): discernment rule 3 — validation of expected failure mode"
```

---

## Task 10: Rule 4 — recovery (passed after prior-failed)

**Files:**
- Modify: `app/elohim-library/projects/elohim-service/src/discernment/mechanical-discerner.ts`
- Modify: `app/elohim-library/projects/elohim-service/src/discernment/mechanical-discerner.spec.ts`

- [ ] **Step 10.1: Add failing test**

```typescript
describe('discernMechanical — rule 4 (recovery)', () => {
  it('mints progress/meaningful/recovery when a scenario passes after a prior-failed attestation', () => {
    const moment = momentFixture({ status: 'passed' });

    const tag = discernMechanical(
      {
        moment,
        priors: {
          latestAny: priorFixture({
            status: 'failed',
            valence: 'regression',
            evidenceType: 'known-cause-recurrence',
            errorClass: 'NetworkError/503',
          }),
          knownErrorClasses: new Set(['NetworkError/503']),
        },
      },
      'uhCEk-moment-hash',
    );

    expect(tag).toMatchObject({
      valence: 'progress',
      magnitude: 'meaningful',
      evidenceType: 'recovery',
    });
  });
});
```

- [ ] **Step 10.2: Run — verify 1 new failure**

```bash
pnpm exec vitest run src/discernment/mechanical-discerner.spec.ts
```
Expected: 5 passed, 1 failed.

- [ ] **Step 10.3: Add rule 4 to the discerner**

```typescript
  // Rule 4 — passed after prior-failed → recovery
  if (moment.status === 'passed' && priors.latestAny?.status === 'failed') {
    return mkTag(input, momentEntryHash, 'progress', 'meaningful', 'recovery');
  }
```

- [ ] **Step 10.4: Run — verify all pass**

```bash
pnpm exec vitest run src/discernment/mechanical-discerner.spec.ts
```
Expected: 6 passed.

- [ ] **Step 10.5: Commit**

```bash
cd /projects/elohim && git add app/elohim-library/projects/elohim-service/src/discernment/
git commit -m "feat(elohim-service): discernment rule 4 — recovery"
```

---

## Task 11: Rule 5 — witness (cross-fingerprint attestation)

**Files:**
- Modify: `app/elohim-library/projects/elohim-service/src/discernment/mechanical-discerner.ts`
- Modify: `app/elohim-library/projects/elohim-service/src/discernment/mechanical-discerner.spec.ts`

- [ ] **Step 11.1: Add two failing tests (passing witness + failing witness)**

```typescript
describe('discernMechanical — rule 5 (witness)', () => {
  it('mints witness/meaningful/cross-fingerprint-attestation when a passing scenario is validated by a NEW compute fingerprint', () => {
    const moment = momentFixture({
      status: 'passed',
      computeFingerprint: 'adam-alpha:device-family-laptop-small:def456',
    });

    const tag = discernMechanical(
      {
        moment,
        priors: {
          latestAny: priorFixture({
            status: 'passed',
            computeFingerprint: 'matthew-alpha:device-family-node-base:abc123',
          }),
          knownErrorClasses: new Set<string>(),
        },
      },
      'uhCEk-moment-hash',
    );

    expect(tag).toMatchObject({
      valence: 'witness',
      magnitude: 'meaningful',
      evidenceType: 'cross-fingerprint-attestation',
    });
  });

  it('mints witness for a FAILING scenario confirmed by a new fingerprint (structural failure, not flake)', () => {
    const moment = momentFixture({
      status: 'failed',
      errorClass: 'AssertionError/timeout',
      computeFingerprint: 'jessica-alpha:device-family-mobile:ghi789',
    });

    const tag = discernMechanical(
      {
        moment,
        priors: {
          latestAny: priorFixture({
            status: 'failed',
            valence: 'discovery',
            evidenceType: 'novel-failure-class',
            errorClass: 'AssertionError/timeout',
            computeFingerprint: 'matthew-alpha:device-family-node-base:abc123',
          }),
          knownErrorClasses: new Set(['AssertionError/timeout']),
        },
      },
      'uhCEk-moment-hash',
    );

    expect(tag).toMatchObject({
      valence: 'witness',
      evidenceType: 'cross-fingerprint-attestation',
    });
  });
});
```

- [ ] **Step 11.2: Run — verify 2 new failures**

```bash
pnpm exec vitest run src/discernment/mechanical-discerner.spec.ts
```
Expected: 6 passed, 2 failed.

- [ ] **Step 11.3: Add rule 5 to the discerner**

Insert AFTER rule 4 (rules 1–4 take precedence when status changes; rule 5 fires only when status matches prior):

```typescript
  // Rule 5 — witness (same status, DIFFERENT fingerprint)
  if (
    priors.latestAny &&
    priors.latestAny.status === moment.status &&
    priors.latestAny.computeFingerprint !== moment.computeFingerprint
  ) {
    return mkTag(
      input,
      momentEntryHash,
      'witness',
      'meaningful',
      'cross-fingerprint-attestation',
    );
  }
```

- [ ] **Step 11.4: Run — verify all pass**

```bash
pnpm exec vitest run src/discernment/mechanical-discerner.spec.ts
```
Expected: 8 passed.

- [ ] **Step 11.5: Commit**

```bash
cd /projects/elohim && git add app/elohim-library/projects/elohim-service/src/discernment/
git commit -m "feat(elohim-service): discernment rule 5 — witness (cross-fingerprint)"
```

---

## Task 12: Rule 6 — refinement (richer evidence, same fingerprint)

**Files:**
- Modify: `app/elohim-library/projects/elohim-service/src/discernment/mechanical-discerner.ts`
- Modify: `app/elohim-library/projects/elohim-service/src/discernment/mechanical-discerner.spec.ts`

- [ ] **Step 12.1: Add three failing tests**

```typescript
describe('discernMechanical — rule 6 (refinement)', () => {
  it('mints refinement/small/evidence-enriched when a new sidecar artifact is present that was not on the prior', () => {
    const moment = momentFixture({
      status: 'passed',
      sidecarArtifacts: {
        cucumber: 'blob:bafkrei-cucumber/xyz.json',
        trace: 'blob:bafkrei-trace/abc.zip',  // new — not in prior
      },
    });

    const tag = discernMechanical(
      {
        moment,
        priors: {
          latestAny: priorFixture({
            status: 'passed',
            sidecarArtifactNames: ['cucumber'],
            computeFingerprint: moment.computeFingerprint,
          }),
          latestSameFingerprint: priorFixture({
            status: 'passed',
            sidecarArtifactNames: ['cucumber'],
            computeFingerprint: moment.computeFingerprint,
          }),
          knownErrorClasses: new Set<string>(),
        },
      },
      'uhCEk-moment-hash',
    );

    expect(tag).toMatchObject({
      valence: 'refinement',
      magnitude: 'small',
      evidenceType: 'evidence-enriched',
    });
  });

  it('mints refinement when duration improves by more than 20%', () => {
    const moment = momentFixture({ status: 'passed', durationMs: 1500 });

    const tag = discernMechanical(
      {
        moment,
        priors: {
          latestAny: priorFixture({
            status: 'passed',
            durationMs: 2100,  // 1500 is a 28.6% improvement
            computeFingerprint: moment.computeFingerprint,
          }),
          latestSameFingerprint: priorFixture({
            status: 'passed',
            durationMs: 2100,
            computeFingerprint: moment.computeFingerprint,
          }),
          knownErrorClasses: new Set<string>(),
        },
      },
      'uhCEk-moment-hash',
    );

    expect(tag).toMatchObject({ valence: 'refinement', evidenceType: 'evidence-enriched' });
  });

  it('does NOT mint refinement when duration improves by less than 20%', () => {
    const moment = momentFixture({ status: 'passed', durationMs: 1900 });

    const tag = discernMechanical(
      {
        moment,
        priors: {
          latestAny: priorFixture({
            status: 'passed',
            durationMs: 2100,  // 1900 is a 9.5% improvement — below threshold
            computeFingerprint: moment.computeFingerprint,
          }),
          latestSameFingerprint: priorFixture({
            status: 'passed',
            durationMs: 2100,
            computeFingerprint: moment.computeFingerprint,
          }),
          knownErrorClasses: new Set<string>(),
        },
      },
      'uhCEk-moment-hash',
    );

    expect(tag).toBeNull();
  });
});
```

- [ ] **Step 12.2: Run — verify 3 new failures**

```bash
pnpm exec vitest run src/discernment/mechanical-discerner.spec.ts
```
Expected: 8 passed, 3 failed.

- [ ] **Step 12.3: Add rule 6 plus the materially-richer helper**

In `mechanical-discerner.ts`, add helper after `mkTag`:

```typescript
import type {
  ExperienceMomentPayload,
  PriorAttestation,
  SidecarName,
} from './types.js';

const DURATION_REFINEMENT_THRESHOLD = 0.2;

function isMateriallyRicher(
  moment: ExperienceMomentPayload,
  prior: PriorAttestation,
): boolean {
  const priorSidecars = new Set<SidecarName>(prior.sidecarArtifactNames);
  const momentSidecarNames = Object.keys(moment.sidecarArtifacts) as SidecarName[];
  for (const s of momentSidecarNames) {
    if (!priorSidecars.has(s)) return true;
  }

  if (prior.durationMs > 0) {
    const improvement = (prior.durationMs - moment.durationMs) / prior.durationMs;
    if (improvement > DURATION_REFINEMENT_THRESHOLD) return true;
  }

  return false;
}
```

Then insert rule 6 AFTER rule 5 and BEFORE `return null`:

```typescript
  // Rule 6 — refinement (same status, same fingerprint, richer evidence)
  if (
    priors.latestSameFingerprint &&
    priors.latestSameFingerprint.status === moment.status &&
    priors.latestSameFingerprint.computeFingerprint === moment.computeFingerprint &&
    isMateriallyRicher(moment, priors.latestSameFingerprint)
  ) {
    return mkTag(input, momentEntryHash, 'refinement', 'small', 'evidence-enriched');
  }
```

- [ ] **Step 12.4: Run — verify all pass**

```bash
pnpm exec vitest run src/discernment/mechanical-discerner.spec.ts
```
Expected: 11 passed.

- [ ] **Step 12.5: Commit**

```bash
cd /projects/elohim && git add app/elohim-library/projects/elohim-service/src/discernment/
git commit -m "feat(elohim-service): discernment rule 6 — refinement (evidence-enriched)"
```

---

## Task 13: Rule 7 — steady-state (no mint)

**Files:**
- Modify: `app/elohim-library/projects/elohim-service/src/discernment/mechanical-discerner.spec.ts`

- [ ] **Step 13.1: Add steady-state tests**

```typescript
describe('discernMechanical — rule 7 (steady-state: no mint)', () => {
  it('returns null for identical status on same fingerprint with no enriched evidence', () => {
    const moment = momentFixture({
      status: 'passed',
      durationMs: 2050,  // barely different
      sidecarArtifacts: { cucumber: 'blob:bafkrei-cucumber/xyz.json' },
    });

    const tag = discernMechanical(
      {
        moment,
        priors: {
          latestAny: priorFixture({
            status: 'passed',
            durationMs: 2100,
            sidecarArtifactNames: ['cucumber'],
            computeFingerprint: moment.computeFingerprint,
          }),
          latestSameFingerprint: priorFixture({
            status: 'passed',
            durationMs: 2100,
            sidecarArtifactNames: ['cucumber'],
            computeFingerprint: moment.computeFingerprint,
          }),
          knownErrorClasses: new Set<string>(),
        },
      },
      'uhCEk-moment-hash',
    );

    expect(tag).toBeNull();
  });
});
```

- [ ] **Step 13.2: Run — verify test passes (no implementation change needed, null is default)**

```bash
pnpm exec vitest run src/discernment/mechanical-discerner.spec.ts
```
Expected: 12 passed.

- [ ] **Step 13.3: Commit**

```bash
cd /projects/elohim && git add app/elohim-library/projects/elohim-service/src/discernment/
git commit -m "test(elohim-service): discernment rule 7 — steady-state no-mint"
```

---

## Task 14: Edge cases — skipped/pending statuses

**Files:**
- Modify: `app/elohim-library/projects/elohim-service/src/discernment/mechanical-discerner.spec.ts`

- [ ] **Step 14.1: Add skipped/pending tests**

```typescript
describe('discernMechanical — non-terminal statuses', () => {
  it('returns null for a skipped scenario', () => {
    const moment = momentFixture({ status: 'skipped' });
    const tag = discernMechanical(
      { moment, priors: { knownErrorClasses: new Set<string>() } },
      'uhCEk',
    );
    expect(tag).toBeNull();
  });

  it('returns null for a pending scenario', () => {
    const moment = momentFixture({ status: 'pending' });
    const tag = discernMechanical(
      { moment, priors: { knownErrorClasses: new Set<string>() } },
      'uhCEk',
    );
    expect(tag).toBeNull();
  });
});
```

- [ ] **Step 14.2: Run tests**

```bash
pnpm exec vitest run src/discernment/mechanical-discerner.spec.ts
```
Expected: 14 passed. (These should pass already — rule 1 requires `passed`, rule 2 requires `failed`, etc. skipped/pending fall through to `return null`.)

- [ ] **Step 14.3: Commit**

```bash
cd /projects/elohim && git add app/elohim-library/projects/elohim-service/src/discernment/
git commit -m "test(elohim-service): discernment non-terminal statuses fall to no-mint"
```

---

## Task 15: Expose the module through elohim-service public API

**Files:**
- Modify: `app/elohim-library/projects/elohim-service/src/public-api.ts`

- [ ] **Step 15.1: Read the current public-api file**

Run:
```bash
cd /projects/elohim && head -40 app/elohim-library/projects/elohim-service/src/public-api.ts
```
Note existing export pattern.

- [ ] **Step 15.2: Add discernment re-export**

Append to the end of `public-api.ts`:

```typescript
/*
 * Experience-story discernment (v1 mechanical).
 * Pure function gate; see spec 2026-04-18-experience-story-epr-design.md §5–§7.
 */
export * from './discernment/index.js';
```

- [ ] **Step 15.3: Run full elohim-service test suite + type-check**

```bash
cd /projects/elohim/app/elohim-library/projects/elohim-service && pnpm test
```
Expected: all tests pass (baseline + 14 new discernment tests).

- [ ] **Step 15.4: Commit**

```bash
cd /projects/elohim && git add app/elohim-library/projects/elohim-service/src/public-api.ts
git commit -m "feat(elohim-service): export discernment module from public API"
```

---

## Task 16: Documentation anchor — module README

**Files:**
- Create: `app/elohim-library/projects/elohim-service/src/discernment/README.md`

- [ ] **Step 16.1: Write the README**

```markdown
# Discernment — v1 Mechanical Gate

Pure-function gate that classifies an `experience-moment` into an optional `StoryPointTag`.

## Design source

- Spec: `genesis/docs/superpowers/specs/2026-04-18-experience-story-epr-design.md` §5–§7
- Plan: `genesis/docs/superpowers/plans/2026-04-18-experience-story-discernment-gate.md`

## Contract

```typescript
discernMechanical(
  input: DiscernmentInput,
  momentEntryHash: string,
): StoryPointTag | null;
```

`null` means "this moment does not warrant a DHT attestation" — the moment itself is still recorded on the persona's source-chain; only the public notarization is withheld.

## Seven valences

| Valence | Rule | When |
|---|---|---|
| `progress` | 1 | First-pass-green — passing moment, no prior. |
| `discovery` | 2a | Failed after prior-passed, **novel** error class. |
| `regression` | 2b | Failed after prior-passed, **known** error class recurrence. |
| `validation` | 3 | Failed AND `@validates-failure-mode` tag AND no prior-passed to trip rule 2. |
| `progress` (recovery) | 4 | Passed after prior-failed. |
| `witness` | 5 | Same status, **different** compute fingerprint. |
| `refinement` | 6 | Same status, same fingerprint, new sidecar OR >20% duration improvement. |
| *(no mint)* | 7 | Steady-state — silence is the signal. |

## How to extend

- Rules are evaluated top-to-bottom; insert new rules in priority order.
- All rule functions share the `mkTag(input, hash, valence, magnitude, evidenceType)` helper.
- Sophisticated discernment (sophia-mediated, steward-curated) lands in a sibling module with the same `DiscernmentInput → StoryPointTag | null` interface — the two can run in parallel during transition.
```

- [ ] **Step 16.2: Commit**

```bash
cd /projects/elohim && git add app/elohim-library/projects/elohim-service/src/discernment/README.md
git commit -m "docs(elohim-service): README for v1 mechanical discernment gate"
```

---

## Final verification

- [ ] **Step F.1: Run full discernment suite**

```bash
cd /projects/elohim/app/elohim-library/projects/elohim-service && pnpm exec vitest run src/discernment/
```
Expected: 14+ tests passed, 0 failed.

- [ ] **Step F.2: Run elohim-service full test suite**

```bash
pnpm test
```
Expected: all previous tests + 14 new discernment tests pass.

- [ ] **Step F.3: Run lamad:codegen one more time to confirm idempotent**

```bash
cd /projects/elohim && pnpm run lamad:codegen
git status
```
Expected: no diff (generated files stable after commit in Task 5).

- [ ] **Step F.4: Run elohim-app build as smoke test**

```bash
cd /projects/elohim/app/elohim-app && pnpm run build
```
Expected: build succeeds (the new generated contentTypes don't break anything).

- [ ] **Step F.5: Final commit check**

```bash
cd /projects/elohim && git log --oneline $(git merge-base HEAD origin/dev)..HEAD
```
Expected: 15+ focused commits telling the story of the implementation, each passing tests at the commit point.

---

## What this plan does NOT do (follow-on plans)

1. **A2O framework integration** — hooking `discernMechanical` into the `After` scenario hook, serializing moments to the persona's source-chain. (Plan 2)
2. **elohim-storage projections + migrations** — the `experience_stories`, `story_point_attestations`, `experience_moments`, and coupling to the existing `economic_events` table. (Plan 3)
3. **Holochain zome entry types + coordinator functions** — the `:hasSubject`, `:inRole`, `:exercises`, `:story-point` link types; the `experience_story::attest()` dual-emit coordinator. Decides α (reuse ContentNode with private visibility) vs β (new `ExperienceMoment` entry type) per spec open question #6. (Plan 4)
4. **Doorway HTTP routes** — `GET /api/v1/experience-stories/{cid}` and the list endpoint. (Plan 5)
5. **EconomicEvent co-emission wiring** — mapping `valence` to REA `action` values against the existing `lamad_event_type` vocabulary per spec open question #7. (Plan 4/5)
6. **Sub-projects A, C, D** — Matthew peer persistence, export/re-upload, inter-pipeline diff to shefa valueflow. Each has its own spec.

When Plan 2 lands, the a2o `After` hook will call `discernMechanical(input, hash)` directly with fixture-shaped data; no change to this module is required.
