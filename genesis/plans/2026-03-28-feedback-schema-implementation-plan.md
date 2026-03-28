# Feedback Information Flow — Schema Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add feedback primitives (claims, observations, instrument archetypes) to the protocol schema so every app manifest must declare what its content types claim to produce and how those claims are observed.

**Architecture:** Schema-first — all changes are to JSON Schema definitions, the lamad manifest, and tests. No Rust, no Angular, no runtime code. This establishes the contract; instrument implementations and REA observation events are future sprints.

**Tech Stack:** JSON Schema (draft 2020-12), AJV validation, json-schema-to-typescript codegen, Vitest (lamad manifest tests)

**Design Doc:** `genesis/plans/2026-03-28-feedback-information-flows-design.md`

---

### Task 1: Add instrument-archetype enum schema

**Files:**
- Create: `elohim/sdk/schemas/v1/enums/instrument-archetype.schema.json`

**Step 1: Create the enum schema**

Follow the existing enum pattern (see `substrate-signal.schema.json` for reference). The 6 archetypes from the design:

```json
{
  "$id": "epr:schema:enum:instrument-archetype",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "InstrumentArchetype",
  "description": "Protocol-level instrument archetypes for feedback observation. Each archetype is a named pattern for how observations are produced — apps reference these in their observation declarations.",
  "type": "string",
  "enum": [
    "retention-check",
    "outcome-correlation",
    "distribution-health",
    "cost-accumulation",
    "outcome-divergence",
    "community-report"
  ],
  "_tiers": {
    "core": {
      "values": [
        "retention-check",
        "outcome-correlation",
        "distribution-health",
        "cost-accumulation",
        "outcome-divergence",
        "community-report"
      ],
      "rationale": "All six archetypes are protocol primitives — they define the categories of questions a system must ask about itself."
    }
  },
  "_dna": {
    "constant": "INSTRUMENT_ARCHETYPES",
    "zome": "content_store_integrity",
    "file": "elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs",
    "tier": "core"
  }
}
```

**Step 2: Verify it parses**

Run: `node -e "JSON.parse(require('fs').readFileSync('elohim/sdk/schemas/v1/enums/instrument-archetype.schema.json', 'utf8')); console.log('OK')"`
Expected: `OK`

**Step 3: Commit**

```bash
git add elohim/sdk/schemas/v1/enums/instrument-archetype.schema.json
git commit -m "schema(protocol): add instrument-archetype enum — 6 feedback observation archetypes"
```

---

### Task 2: Add observation-polarity enum schema

**Files:**
- Create: `elohim/sdk/schemas/v1/enums/observation-polarity.schema.json`

**Step 1: Create the enum schema**

```json
{
  "$id": "epr:schema:enum:observation-polarity",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "ObservationPolarity",
  "description": "Polarity of a feedback observation. Positive observations extend claim validity horizons. Negative observations shorten them. The protocol requires at least one negative-polarity observation per manifest — systems cannot ship positive-only feedback.",
  "type": "string",
  "enum": ["positive", "negative"],
  "_tiers": {
    "core": {
      "values": ["positive", "negative"],
      "rationale": "Binary polarity is a protocol invariant — every observation either supports or strains a claim."
    }
  },
  "_dna": {
    "constant": "OBSERVATION_POLARITIES",
    "zome": "content_store_integrity",
    "file": "elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs",
    "tier": "core"
  }
}
```

**Step 2: Verify it parses**

Run: `node -e "JSON.parse(require('fs').readFileSync('elohim/sdk/schemas/v1/enums/observation-polarity.schema.json', 'utf8')); console.log('OK')"`
Expected: `OK`

**Step 3: Commit**

```bash
git add elohim/sdk/schemas/v1/enums/observation-polarity.schema.json
git commit -m "schema(protocol): add observation-polarity enum — positive/negative feedback polarity"
```

---

### Task 3: Add ClaimDeclaration and ObservationDeclaration to app-manifest schema

**Files:**
- Modify: `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json`

This is the core schema change. Three modifications:

**Step 1: Write failing manifest schema test**

Add to `elohim/sdk/schemas/scripts/test-manifest-schema.mjs` — append these test cases BEFORE the final summary output:

```javascript
// --- Feedback: Claims required ---
{
  const noClaims = {
    id: 'test', name: 'test', version: '1.0.0',
    vocabulary: {
      contentTypes: {
        thing: {
          description: 'test',
          coupling: {
            value: { onConsume: { action: 'use' } },
            governance: { defaultReach: 'commons', minimumReach: 'community', governanceModel: 'steward-consent' }
            // no claims
          }
        }
      },
      observations: {
        'good-thing': { description: 'test', instrument: 'retention-check', polarity: 'positive' },
        'bad-thing': { description: 'test', instrument: 'retention-check', polarity: 'negative' }
      }
    }
  };
  const valid = validate(noClaims);
  assert(!valid, 'Should reject content type without claims');
  passed++;
}

// --- Feedback: Claims must reference vocabulary observations ---
{
  const validClaims = {
    id: 'test', name: 'test', version: '1.0.0',
    vocabulary: {
      contentTypes: {
        thing: {
          description: 'test',
          coupling: {
            value: { onConsume: { action: 'use' } },
            governance: { defaultReach: 'commons', minimumReach: 'community', governanceModel: 'steward-consent' },
            claims: [{ asserts: 'good-thing', contradictedBy: 'bad-thing', validityHorizon: 'P30D' }]
          }
        }
      },
      observations: {
        'good-thing': { description: 'test', instrument: 'retention-check', polarity: 'positive' },
        'bad-thing': { description: 'test', instrument: 'retention-check', polarity: 'negative' }
      }
    }
  };
  const valid = validate(validClaims);
  assert(valid, 'Should accept content type with valid claims + observations');
  passed++;
}

// --- Feedback: Observations required in vocabulary ---
{
  const noObservations = {
    id: 'test', name: 'test', version: '1.0.0',
    vocabulary: {
      contentTypes: {
        thing: {
          description: 'test',
          coupling: {
            value: { onConsume: { action: 'use' } },
            governance: { defaultReach: 'commons', minimumReach: 'community', governanceModel: 'steward-consent' },
            claims: [{ asserts: 'good', contradictedBy: 'bad', validityHorizon: 'P30D' }]
          }
        }
      }
      // no observations
    }
  };
  const valid = validate(noObservations);
  assert(!valid, 'Should reject manifest without observations');
  passed++;
}
```

**Step 2: Run tests to verify they fail**

Run: `pnpm run manifest:test`
Expected: 3 failures (schema doesn't know about claims or observations yet)

**Step 3: Add ClaimDeclaration $def to app-manifest.schema.json**

Add to `$defs` section (after `GovernanceLeg`):

```json
"ClaimDeclaration": {
  "title": "ClaimDeclaration",
  "description": "A claim about what outcome a content type produces. Every claim declares what it asserts, what would contradict it, and how long it's presumed valid. Accumulated positive observations extend the validity horizon; accumulated negative observations shorten it.",
  "type": "object",
  "required": ["asserts", "contradictedBy", "validityHorizon"],
  "properties": {
    "asserts": {
      "type": "string",
      "description": "Observation term this content type claims to produce. References an observation name in vocabulary.observations."
    },
    "contradictedBy": {
      "type": "string",
      "description": "Observation term that would undermine this claim. References an observation name in vocabulary.observations. Should have negative polarity."
    },
    "validityHorizon": {
      "type": "string",
      "description": "ISO 8601 duration. How long the claim is presumed valid without fresh evidence.",
      "pattern": "^P"
    },
    "leg": {
      "type": "string",
      "enum": ["knowledge", "value", "governance"],
      "description": "Which coupling leg this claim relates to. Optional — for documentation and query filtering."
    }
  },
  "additionalProperties": false
}
```

**Step 4: Add ObservationDeclaration $def**

Add to `$defs` (after `ClaimDeclaration`):

```json
"ObservationDeclaration": {
  "title": "ObservationDeclaration",
  "description": "Declaration of an observation term — something the system watches for. Observations have polarity: positive observations extend claim validity horizons, negative observations shorten them. The protocol requires at least one negative-polarity observation per manifest.",
  "type": "object",
  "required": ["description", "instrument", "polarity"],
  "properties": {
    "description": {
      "type": "string",
      "description": "Human-readable description of what this observation means."
    },
    "instrument": {
      "$ref": "../enums/instrument-archetype.schema.json",
      "description": "Instrument archetype that produces this observation. References a protocol-defined archetype."
    },
    "polarity": {
      "$ref": "../enums/observation-polarity.schema.json",
      "description": "Positive observations extend claim validity. Negative observations shorten it."
    }
  },
  "additionalProperties": false
}
```

**Step 5: Add `claims` to ThreeLegCoupling**

Modify the `ThreeLegCoupling` definition:
- Add `"claims"` to the `"required"` array (becomes `["value", "governance", "claims"]`)
- Add `claims` property:

```json
"claims": {
  "type": "array",
  "description": "Claims about what outcomes this content type produces. Every content type must assert at least one outcome and declare what would contradict it.",
  "items": {
    "$ref": "#/$defs/ClaimDeclaration"
  },
  "minItems": 1
}
```

- Update the ThreeLegCoupling description to mention claims:
  `"The three-leg coupling contract for a content type, with required feedback claims. Every EPR carries lamad (knowledge) + shefa (value) + qahal (governance) dimensions, and must declare what outcomes it claims to produce. No value-blind content. No governance-free content. No unobserved claims."`

**Step 6: Add `observations` to Vocabulary**

Modify the `Vocabulary` definition:
- Add `"observations"` to the `"required"` array (becomes `["contentTypes", "observations"]`)
- Add observations property (after signals):

```json
"observations": {
  "type": "object",
  "description": "Map of observation name to ObservationDeclaration. Declares the feedback evidence vocabulary. Must include at least one negative-polarity observation — systems cannot ship positive-only feedback.",
  "additionalProperties": {
    "$ref": "#/$defs/ObservationDeclaration"
  },
  "minProperties": 1
}
```

**Step 7: Run tests to verify they pass**

Run: `pnpm run manifest:test`
Expected: All tests pass (including the 3 new ones)

**Step 8: Commit**

```bash
git add elohim/sdk/schemas/v1/manifest/app-manifest.schema.json elohim/sdk/schemas/scripts/test-manifest-schema.mjs
git commit -m "schema(protocol): add claims + observations to manifest coupling

ThreeLegCoupling now requires claims (minItems: 1). Vocabulary now
requires observations (minProperties: 1). Every content type must
declare what it asserts and what would contradict it. Protocol
rejects manifests without feedback declarations."
```

---

### Task 4: Add observations to lamad manifest vocabulary

**Files:**
- Modify: `app/lamad/manifest.json`

**Step 1: Write failing lamad manifest test**

Add to `genesis/seeder/src/__tests__/manifest-lamad.test.ts`:

```typescript
describe('observations vocabulary', () => {
  it('should declare observations', () => {
    expect(manifest.vocabulary.observations).toBeDefined();
    expect(Object.keys(manifest.vocabulary.observations).length).toBeGreaterThan(0);
  });

  it('should include at least one negative-polarity observation', () => {
    const observations = manifest.vocabulary.observations;
    const hasNegative = Object.values(observations).some(
      (obs: any) => obs.polarity === 'negative'
    );
    expect(hasNegative).toBe(true);
  });

  it('should reference valid instrument archetypes', () => {
    const validArchetypes = [
      'retention-check', 'outcome-correlation', 'distribution-health',
      'cost-accumulation', 'outcome-divergence', 'community-report'
    ];
    const observations = manifest.vocabulary.observations;
    for (const [name, obs] of Object.entries(observations)) {
      expect(validArchetypes).toContain((obs as any).instrument);
    }
  });
});
```

**Step 2: Run test to verify it fails**

Run: `cd genesis/seeder && pnpm exec vitest run src/__tests__/manifest-lamad.test.ts`
Expected: FAIL — `manifest.vocabulary.observations` is undefined

**Step 3: Add observations to manifest.json**

Add the `observations` section to `vocabulary` in `app/lamad/manifest.json` (after `signals`):

```json
"observations": {
  "knowledge-retention": {
    "description": "Learner can recall and apply concept after interval",
    "instrument": "retention-check",
    "polarity": "positive"
  },
  "retention-failure": {
    "description": "Learner cannot recall or apply concept after interval",
    "instrument": "retention-check",
    "polarity": "negative"
  },
  "mastery-attestation-meaningful": {
    "description": "Mastery-attested learners succeed in downstream prerequisites",
    "instrument": "outcome-correlation",
    "polarity": "positive"
  },
  "downstream-prerequisite-failure": {
    "description": "Mastery-attested learners fail in downstream content they should be prepared for",
    "instrument": "outcome-correlation",
    "polarity": "negative"
  },
  "stewardship-distributed": {
    "description": "Content domain has diverse steward participation",
    "instrument": "distribution-health",
    "polarity": "positive"
  },
  "stewardship-concentrated": {
    "description": "Single steward controls disproportionate share of domain content",
    "instrument": "distribution-health",
    "polarity": "negative"
  },
  "content-relevant": {
    "description": "Content is current and referenced by active learning paths",
    "instrument": "outcome-correlation",
    "polarity": "positive"
  },
  "content-outdated": {
    "description": "Content no longer reflects current understanding (human-reported or instrument-detected)",
    "instrument": "community-report",
    "polarity": "negative"
  },
  "content-misleading": {
    "description": "Content framing does not match substance (human-reported)",
    "instrument": "community-report",
    "polarity": "negative"
  },
  "learning-outcome-achieved": {
    "description": "Learner demonstrates competence in the domain a path covers after completion",
    "instrument": "outcome-correlation",
    "polarity": "positive"
  },
  "path-completion-without-retention": {
    "description": "Learner completed path but cannot demonstrate competence in its domain",
    "instrument": "outcome-correlation",
    "polarity": "negative"
  },
  "community-value-confirmed": {
    "description": "Community contributions produce engagement and downstream learning",
    "instrument": "outcome-correlation",
    "polarity": "positive"
  },
  "engagement-decline": {
    "description": "Community contributions are not producing engagement or downstream learning",
    "instrument": "outcome-correlation",
    "polarity": "negative"
  }
}
```

**Step 4: Run test to verify it passes**

Run: `cd genesis/seeder && pnpm exec vitest run src/__tests__/manifest-lamad.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add app/lamad/manifest.json genesis/seeder/src/__tests__/manifest-lamad.test.ts
git commit -m "schema(lamad): add 13 observation terms to lamad vocabulary

Paired positive/negative observations covering retention, downstream
mastery correlation, stewardship distribution, content relevance,
path outcomes, and community reports."
```

---

### Task 5: Add claims to all lamad content types

**Files:**
- Modify: `app/lamad/manifest.json`

**Step 1: Write failing test**

Add to `genesis/seeder/src/__tests__/manifest-lamad.test.ts`:

```typescript
describe('claims on content types', () => {
  it('every content type should have at least one claim', () => {
    const types = manifest.vocabulary.contentTypes;
    for (const [name, decl] of Object.entries(types)) {
      expect((decl as any).coupling.claims, `${name} missing claims`).toBeDefined();
      expect((decl as any).coupling.claims.length, `${name} has empty claims`).toBeGreaterThan(0);
    }
  });

  it('every claim should reference a declared observation', () => {
    const types = manifest.vocabulary.contentTypes;
    const observations = Object.keys(manifest.vocabulary.observations);
    for (const [name, decl] of Object.entries(types)) {
      const claims = (decl as any).coupling.claims || [];
      for (const claim of claims) {
        expect(observations, `${name} claim asserts unknown observation: ${claim.asserts}`).toContain(claim.asserts);
        expect(observations, `${name} claim contradictedBy unknown observation: ${claim.contradictedBy}`).toContain(claim.contradictedBy);
      }
    }
  });

  it('every claim should have a valid ISO 8601 duration', () => {
    const types = manifest.vocabulary.contentTypes;
    for (const [name, decl] of Object.entries(types)) {
      const claims = (decl as any).coupling.claims || [];
      for (const claim of claims) {
        expect(claim.validityHorizon, `${name} claim missing validityHorizon`).toBeDefined();
        expect(claim.validityHorizon, `${name} validityHorizon not ISO 8601`).toMatch(/^P/);
      }
    }
  });
});
```

**Step 2: Run test to verify it fails**

Run: `cd genesis/seeder && pnpm exec vitest run src/__tests__/manifest-lamad.test.ts`
Expected: FAIL — content types don't have claims yet

**Step 3: Add claims to each content type**

Add `claims` array to each content type's `coupling` object in `app/lamad/manifest.json`.

**Claims by content type group:**

**Assessment types** (quiz, assessment, practice):
```json
"claims": [
  { "asserts": "knowledge-retention", "contradictedBy": "retention-failure", "validityHorizon": "P30D", "leg": "knowledge" },
  { "asserts": "mastery-attestation-meaningful", "contradictedBy": "downstream-prerequisite-failure", "validityHorizon": "P90D", "leg": "value" }
]
```

**Simulation:**
```json
"claims": [
  { "asserts": "knowledge-retention", "contradictedBy": "retention-failure", "validityHorizon": "P30D", "leg": "knowledge" }
]
```

**Core content** (concept, lesson):
```json
"claims": [
  { "asserts": "knowledge-retention", "contradictedBy": "retention-failure", "validityHorizon": "P30D", "leg": "knowledge" },
  { "asserts": "content-relevant", "contradictedBy": "content-outdated", "validityHorizon": "P180D", "leg": "knowledge" }
]
```

**Article:**
```json
"claims": [
  { "asserts": "content-relevant", "contradictedBy": "content-outdated", "validityHorizon": "P180D", "leg": "knowledge" },
  { "asserts": "stewardship-distributed", "contradictedBy": "stewardship-concentrated", "validityHorizon": "P180D", "leg": "governance" }
]
```

**Path:**
```json
"claims": [
  { "asserts": "learning-outcome-achieved", "contradictedBy": "path-completion-without-retention", "validityHorizon": "P90D", "leg": "knowledge" },
  { "asserts": "content-relevant", "contradictedBy": "content-outdated", "validityHorizon": "P180D", "leg": "knowledge" },
  { "asserts": "stewardship-distributed", "contradictedBy": "stewardship-concentrated", "validityHorizon": "P180D", "leg": "governance" }
]
```

**Course-module, module:**
```json
"claims": [
  { "asserts": "learning-outcome-achieved", "contradictedBy": "path-completion-without-retention", "validityHorizon": "P90D", "leg": "knowledge" },
  { "asserts": "content-relevant", "contradictedBy": "content-outdated", "validityHorizon": "P180D", "leg": "knowledge" }
]
```

**Discovery-assessment:**
```json
"claims": [
  { "asserts": "content-relevant", "contradictedBy": "content-outdated", "validityHorizon": "P365D", "leg": "knowledge" }
]
```

**Instrument:**
```json
"claims": [
  { "asserts": "content-relevant", "contradictedBy": "content-outdated", "validityHorizon": "P365D", "leg": "knowledge" }
]
```

**Discussion, community:**
```json
"claims": [
  { "asserts": "community-value-confirmed", "contradictedBy": "engagement-decline", "validityHorizon": "P90D", "leg": "value" }
]
```

**Reflection:**
```json
"claims": [
  { "asserts": "content-relevant", "contradictedBy": "content-outdated", "validityHorizon": "P180D", "leg": "knowledge" }
]
```

**Exercise:**
```json
"claims": [
  { "asserts": "knowledge-retention", "contradictedBy": "retention-failure", "validityHorizon": "P30D", "leg": "knowledge" }
]
```

**Epic, feature, scenario, tool:**
```json
"claims": [
  { "asserts": "content-relevant", "contradictedBy": "content-outdated", "validityHorizon": "P365D", "leg": "knowledge" }
]
```

**Placeholder:**
```json
"claims": [
  { "asserts": "content-relevant", "contradictedBy": "content-outdated", "validityHorizon": "P1D", "leg": "knowledge" }
]
```
Note: Placeholder gets a 1-day horizon — it claims to be temporary, and if it persists past 1 day that claim is strained.

**Step 4: Run test to verify it passes**

Run: `cd genesis/seeder && pnpm exec vitest run src/__tests__/manifest-lamad.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add app/lamad/manifest.json genesis/seeder/src/__tests__/manifest-lamad.test.ts
git commit -m "schema(lamad): add feedback claims to all 21 content types

Every content type now declares what outcomes it claims to produce
and what observations would contradict those claims. Claims reference
the 13 observation terms in the vocabulary. Validity horizons range
from P1D (placeholder — should resolve immediately) to P365D
(structural types — slow-changing)."
```

---

### Task 6: Run codegen and full validation

**Files:**
- Generated: `elohim/sdk/schemas/generated-ts/enums/instrument-archetype.ts`
- Generated: `elohim/sdk/schemas/generated-ts/enums/observation-polarity.ts`
- Generated: `app/elohim-app/src/app/generated/schema-enums.ts` (updated)
- Generated: `genesis/seeder/src/generated/schema-enums.ts` (updated)
- Generated: `app/elohim-library/projects/elohim-service/src/generated/schema-enums.ts` (updated)

**Step 1: Run schema self-tests**

Run: `pnpm run schema:test`
Expected: All 24 assertions pass

**Step 2: Run manifest schema tests**

Run: `pnpm run manifest:test`
Expected: All assertions pass (including new feedback tests)

**Step 3: Run codegen**

Run: `pnpm run schema:codegen:ts`
Expected: Generates new enum files and updates schema-enums.ts in all 3 distribution targets

**Step 4: Verify codegen output includes new types**

Run: `grep -l "INSTRUMENT_ARCHETYPES\|OBSERVATION_POLARITIES" app/elohim-app/src/app/generated/schema-enums.ts`
Expected: File found with both constants

**Step 5: Run schema validation against seed data**

Run: `pnpm run schema:validate`
Expected: All seed JSON files pass (seed data doesn't include manifest claims — this validates the input schemas still work)

**Step 6: Run lamad manifest tests**

Run: `cd genesis/seeder && pnpm exec vitest run src/__tests__/manifest-lamad.test.ts`
Expected: All tests pass

**Step 7: Commit generated files**

```bash
git add elohim/sdk/schemas/generated-ts/ app/elohim-app/src/app/generated/ genesis/seeder/src/generated/ app/elohim-library/projects/elohim-service/src/generated/
git commit -m "codegen: regenerate TypeScript from feedback schema additions

Adds InstrumentArchetype and ObservationPolarity enum types to all
three distribution targets."
```

---

### Task 7: Run full pre-push validation

**Step 1: Run the full test suite for affected projects**

Run: `pnpm run schema:test && pnpm run manifest:test && pnpm run schema:validate && pnpm run schema:check-dna`
Expected: All pass. The `schema:check-dna` may warn about new constants not yet in DNA — that's expected (DNA update is a future task).

**Step 2: Run lamad manifest tests**

Run: `cd genesis/seeder && pnpm exec vitest run src/__tests__/manifest-lamad.test.ts`
Expected: All pass

**Step 3: Verify codegen is not stale**

Run: `pnpm run schema:codegen:ts -- --verify`
Expected: Exit 0 (not stale)

---

## Summary

| Task | What | Files |
|------|------|-------|
| 1 | instrument-archetype enum | 1 new schema |
| 2 | observation-polarity enum | 1 new schema |
| 3 | ClaimDeclaration + ObservationDeclaration in manifest schema | 1 modified schema + 1 modified test |
| 4 | Lamad observation vocabulary (13 terms) | 1 modified manifest + 1 modified test |
| 5 | Claims on all 21 content types | 1 modified manifest + 1 modified test |
| 6 | Codegen + validation | Generated files |
| 7 | Full pre-push validation | No files — verification only |

**Total new files:** 2 enum schemas
**Total modified files:** 3 (app-manifest.schema.json, manifest.json, 2 test files)
**Total generated files:** ~5 (codegen outputs)

## What This Does NOT Include (future sprints)

- Rust types for ClaimDeclaration/ObservationDeclaration (storage layer)
- DNA constant updates for new enums (requires DNA pipeline build)
- Runtime observation instruments (the actual retention-check, outcome-correlation implementations)
- REA EconomicEvent with `observe` action (the obligation accumulation layer)
- FeedbackTrace types (the elohim narrative layer)
- Angular services for feedback display
- Manifest-level cross-validation (claims reference observations that exist — currently structural only, not referential)
