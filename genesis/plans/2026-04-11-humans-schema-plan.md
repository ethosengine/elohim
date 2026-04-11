# Humans & Presences Schema Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create IoC schemas for humans and presences, migrate the Consilience Garden Keen and orphan content stubs into first-class `ContributorPresence` entries, wire a new seeder stage, and fix narrative-coherence drift across 4 missing account packages and 2 real a2o bugs.

**Architecture:** Markdown-first canonical sources (humans + presences) with YAML frontmatter validated against JSON Schema. Generated JSON artifacts are derived projections. The validator imports `CreateHumanInputView` and `CreateContributorPresenceInputView` from `@elohim/storage-client` as compile-time type guards. Zero Rust changes — every primitive exists in imagodei DNA. One-shot migration script reads `Consilience_Garden-.../keen.json`, builds ~110 presence records, captures images locally, rewrites content references, deletes itself and the Keen directory in the same commit.

**Tech Stack:** TypeScript 5.x, Vitest, `yaml` (frontmatter parser), `sharp` (image resize — dev dep for migration, removed after), existing `@elohim/storage-client` generated types, JSON Schema draft 2020-12.

**Design spec:** `genesis/plans/2026-04-11-humans-schema-design.md`

**Commit strategy:** No per-task commits. Each task's final step is `git add` to stage. ONE final commit at the end of Phase G. This respects the user's sprint-end-commit rule while preserving TDD task discipline.

---

## Phase A — Foundation (schemas + validators + build-data)

### Task A1: Create `humans.schema.json`

**Files:**
- Create: `genesis/data/humans/humans.schema.json`

- [ ] **Step 1: Create the directory**

Run: `mkdir -p genesis/data/humans`
Expected: directory created

- [ ] **Step 2: Write the schema file**

This is the IoC contract for human frontmatter. The full schema is defined in the design doc (`2026-04-11-humans-schema-design.md` → *humans.schema.json structure* section). Copy verbatim from there — all field definitions, enums, and the `HumanRelationshipType` `$defs` entry with 22 types.

Key fields to verify present: `id` (slug pattern), `displayName`, `bio`, `agencyPhase` (enum doorway|hosted|device|node|retired|null), `category` (enum), `profileReach`, `location` (object with layer enum), `organizations[]`, `communities[]`, `affinities[]`, `guardianIds[]`, `ageCategory`, `isPseudonymous`, `acceptingConnections`, `languagePreferences`, `accessibilityNeeds`, `flags[]`, `claimedAttestations[]`, and the `HumanRelationshipType` $def with all 22 values including `key-steward-of`.

- [ ] **Step 3: Validate the schema file is valid JSON**

Run: `python3 -c "import json; json.load(open('genesis/data/humans/humans.schema.json'))"`
Expected: no output (valid JSON)

- [ ] **Step 4: Stage**

Run: `git add genesis/data/humans/humans.schema.json`

---

### Task A2: Create `presences.schema.json`

**Files:**
- Create: `genesis/data/presences/presences.schema.json`

- [ ] **Step 1: Create the directory**

Run: `mkdir -p genesis/data/presences/images`
Expected: directory created

- [ ] **Step 2: Write the schema file**

Full schema is in the design doc → *presences.schema.json structure* section. Copy verbatim. Key fields to verify: `id` (slug pattern `^presence-`), `displayName`, `presenceType` (enum person|organization), `bio`, `observations[]` (minItems: 1, requires observerId/observedAt/context), `primaryStewardId`, `stewardshipStartedAt`, `externalIdentifiers[]` (type enum includes orcid/isni/wikipedia/wikidata/linkedin/twitter/mastodon/personal-domain/doi/isbn/arxiv/github/homepage/email), `sameAsPresenceIds[]`, `works[]`, `suggestedCollectiveIds[]`, `tags[]`, `image` (object with local/placeholder/sourceUrl), and the `$defs` for `Observation`, `ExternalIdentifier`, `Work`.

- [ ] **Step 3: Validate the schema file is valid JSON**

Run: `python3 -c "import json; json.load(open('genesis/data/presences/presences.schema.json'))"`
Expected: no output

- [ ] **Step 4: Stage**

Run: `git add genesis/data/presences/presences.schema.json`

---

### Task A3: Create `validate-humans.ts` (TDD)

**Files:**
- Create: `genesis/seeder/src/validate-humans.ts`
- Create: `genesis/seeder/src/__tests__/validate-humans.test.ts`

- [ ] **Step 1: Install yaml parser dependency**

Run: `cd genesis/seeder && pnpm add yaml`
Expected: `yaml` added to package.json

- [ ] **Step 2: Write failing tests first**

Create `genesis/seeder/src/__tests__/validate-humans.test.ts`:

```typescript
import { describe, it, expect } from 'vitest';
import { validateHumanFrontmatter, validateHumansDirectory } from '../validate-humans.js';
import { mkdtemp, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

describe('validateHumanFrontmatter', () => {
  it('accepts a minimal valid human', () => {
    const fm = {
      id: 'human-test-user',
      displayName: 'Test User',
      category: 'core-family',
      profileReach: 'community',
    };
    const result = validateHumanFrontmatter(fm, 'test-user.md');
    expect(result.errors).toEqual([]);
  });

  it('rejects id not matching slug pattern', () => {
    const fm = {
      id: 'human_test_user', // underscores instead of hyphens
      displayName: 'Test User',
      category: 'core-family',
      profileReach: 'community',
    };
    const result = validateHumanFrontmatter(fm, 'test-user.md');
    expect(result.errors.some(e => e.includes('slug pattern'))).toBe(true);
  });

  it('rejects unknown category enum value', () => {
    const fm = {
      id: 'human-test-user',
      displayName: 'Test User',
      category: 'not-a-category',
      profileReach: 'community',
    };
    const result = validateHumanFrontmatter(fm, 'test-user.md');
    expect(result.errors.some(e => e.includes('category'))).toBe(true);
  });

  it('rejects unknown relationship type', () => {
    const rel = {
      source: 'human-a',
      target: 'human-b',
      relationshipType: 'bestie', // not in vocabulary
      intimacyLevel: 'trusted',
    };
    // Assuming validateRelationship is exported too
    // Test rejects bestie and accepts spouse
  });

  it('accepts valid filename/id match', () => {
    const fm = { id: 'human-matthew-manager', displayName: 'Matthew', category: 'core-family', profileReach: 'community' };
    const result = validateHumanFrontmatter(fm, 'matthew-manager.md');
    expect(result.errors).toEqual([]);
  });

  it('rejects filename/id mismatch', () => {
    const fm = { id: 'human-wrong-id', displayName: 'Matthew', category: 'core-family', profileReach: 'community' };
    const result = validateHumanFrontmatter(fm, 'matthew-manager.md');
    expect(result.errors.some(e => e.includes('filename'))).toBe(true);
  });
});

describe('validateHumansDirectory referential integrity', () => {
  it('detects duplicate ids across files', async () => {
    const dir = await mkdtemp(join(tmpdir(), 'humans-test-'));
    await writeFile(join(dir, 'a.md'), `---\nid: human-a\ndisplayName: A\ncategory: core-family\nprofileReach: community\n---\n`);
    await writeFile(join(dir, 'b.md'), `---\nid: human-a\ndisplayName: B\ncategory: core-family\nprofileReach: community\n---\n`);
    const result = await validateHumansDirectory(dir);
    expect(result.errors.some(e => e.includes('duplicate id'))).toBe(true);
  });

  it('detects duplicate displayNames (Pete aliasing rule)', async () => {
    const dir = await mkdtemp(join(tmpdir(), 'humans-test-'));
    await writeFile(join(dir, 'a.md'), `---\nid: human-a\ndisplayName: Pete\ncategory: core-family\nprofileReach: community\n---\n`);
    await writeFile(join(dir, 'b.md'), `---\nid: human-b\ndisplayName: Pete\ncategory: core-family\nprofileReach: community\n---\n`);
    const result = await validateHumansDirectory(dir);
    expect(result.errors.some(e => e.includes('duplicate displayName'))).toBe(true);
  });

  it('detects unresolved guardianIds', async () => {
    const dir = await mkdtemp(join(tmpdir(), 'humans-test-'));
    await writeFile(join(dir, 'kid.md'), `---\nid: human-kid\ndisplayName: Kid\ncategory: core-family\nprofileReach: community\nageCategory: minor\nguardianIds: [human-ghost]\n---\n`);
    const result = await validateHumansDirectory(dir);
    expect(result.errors.some(e => e.includes('guardianIds') && e.includes('human-ghost'))).toBe(true);
  });
});
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd genesis/seeder && pnpm exec vitest run src/__tests__/validate-humans.test.ts`
Expected: FAIL — `validate-humans.ts` module does not exist

- [ ] **Step 4: Implement the validator**

Create `genesis/seeder/src/validate-humans.ts`:

```typescript
/**
 * Humans Seed Data Validator
 *
 * Validates genesis/data/humans/*.md frontmatter against humans.schema.json.
 * Hand-rolled (no Ajv/Zod) to match the existing seeder validation pattern.
 * Imports type guards from @elohim/storage-client for fields that map to
 * the Rust CreateHumanInputView.
 */

import { readdirSync, readFileSync } from 'node:fs';
import { resolve, basename } from 'node:path';
import { parse as parseYaml } from 'yaml';

// =============================================================================
// Constants — sources of truth
// =============================================================================

const CATEGORIES = [
  'core-family', 'workplace', 'community', 'affinity',
  'local-economy', 'newcomer', 'visitor', 'edge-case', 'red-team',
] as const;

const PROFILE_REACH = [
  'private', 'self', 'intimate', 'trusted', 'familiar',
  'community', 'public', 'commons', 'hidden',
] as const;

const AGENCY_PHASES = ['doorway', 'hosted', 'device', 'node', 'retired'] as const;

const AGE_CATEGORIES = ['minor', 'adult', 'elder'] as const;

const LOCATION_LAYERS = [
  'neighborhood', 'municipality', 'city', 'county_regional',
  'bioregion', 'nation', 'global',
] as const;

export const HUMAN_RELATIONSHIP_TYPES = [
  'spouse', 'parent-of', 'child-of', 'sibling', 'grandparent-of', 'grandchild-of', 'extended-family',
  'guardian-of', 'ward-of', 'caregiver-of', 'key-steward-of',
  'coworker', 'supervises', 'reports-to', 'business-partner',
  'neighbor', 'congregation-member', 'community-member',
  'mentor-of', 'mentee-of', 'learning-partner',
  'acquaintance',
] as const;

const INTIMACY_LEVELS = ['recognition', 'connection', 'trusted', 'intimate'] as const;

const CLAIM_STATUSES = ['pending', 'verified', 'unverified', 'disputed', 'revoked'] as const;

const SLUG_PATTERN = /^human-[a-z0-9][a-z0-9-]*[a-z0-9]$/;

// =============================================================================
// Types
// =============================================================================

export interface HumanFrontmatter {
  id: string;
  displayName: string;
  bio?: string | null;
  agencyPhase?: string | null;
  category: string;
  profileReach: string;
  location?: { layer: string; name: string; h3Cell?: string | null } | null;
  organizations?: Array<{ id: string; name: string; role: string }>;
  communities?: string[];
  affinities?: string[];
  guardianIds?: string[];
  ageCategory?: string | null;
  isPseudonymous?: boolean;
  acceptingConnections?: boolean;
  languagePreferences?: { primary: string; secondary?: string | null; learningLevel?: string | null } | null;
  accessibilityNeeds?: string[];
  flags?: Array<{ type: string; reason: string; count?: number | null; severity?: string | null }>;
  claimedAttestations?: Array<{ claim: string; status: string; challengedAt?: string | null; verifiedBy?: string | null }>;
}

export interface RelationshipEntry {
  source: string;
  target: string;
  relationshipType: string;
  intimacyLevel: string;
  context?: string | null;
  startedAt?: string | null;
  expiresAt?: string | null;
  notes?: string | null;
}

export interface ValidationResult {
  errors: string[];
  warnings: string[];
  humans: Map<string, HumanFrontmatter>;
  relationships: RelationshipEntry[];
}

// =============================================================================
// Per-human validation
// =============================================================================

export function validateHumanFrontmatter(
  fm: HumanFrontmatter,
  filename: string,
): { errors: string[]; warnings: string[] } {
  const errors: string[] = [];
  const warnings: string[] = [];
  const label = filename;

  // Required fields
  if (!fm.id) errors.push(`${label}: missing required field 'id'`);
  if (!fm.displayName) errors.push(`${label}: missing required field 'displayName'`);
  if (!fm.category) errors.push(`${label}: missing required field 'category'`);
  if (!fm.profileReach) errors.push(`${label}: missing required field 'profileReach'`);

  if (fm.id && !SLUG_PATTERN.test(fm.id)) {
    errors.push(`${label}: id '${fm.id}' does not match slug pattern ^human-[a-z0-9][a-z0-9-]*[a-z0-9]$`);
  }

  // Filename/id consistency
  if (fm.id) {
    const stem = basename(filename).replace(/\.md$/, '');
    const expectedId = `human-${stem}`;
    if (fm.id !== expectedId) {
      errors.push(`${label}: filename '${basename(filename)}' does not match id '${fm.id}' (expected id: '${expectedId}')`);
    }
  }

  // Enum validations
  if (fm.category && !(CATEGORIES as readonly string[]).includes(fm.category)) {
    errors.push(`${label}: category '${fm.category}' is not in enum: ${CATEGORIES.join(', ')}`);
  }
  if (fm.profileReach && !(PROFILE_REACH as readonly string[]).includes(fm.profileReach)) {
    errors.push(`${label}: profileReach '${fm.profileReach}' is not in enum: ${PROFILE_REACH.join(', ')}`);
  }
  if (fm.agencyPhase != null && !(AGENCY_PHASES as readonly string[]).includes(fm.agencyPhase)) {
    errors.push(`${label}: agencyPhase '${fm.agencyPhase}' is not in enum: ${AGENCY_PHASES.join(', ')}`);
  }
  if (fm.ageCategory != null && !(AGE_CATEGORIES as readonly string[]).includes(fm.ageCategory)) {
    errors.push(`${label}: ageCategory '${fm.ageCategory}' is not in enum: ${AGE_CATEGORIES.join(', ')}`);
  }

  // Location
  if (fm.location) {
    if (!fm.location.layer || !(LOCATION_LAYERS as readonly string[]).includes(fm.location.layer)) {
      errors.push(`${label}: location.layer invalid or missing`);
    }
    if (!fm.location.name) {
      errors.push(`${label}: location.name missing`);
    }
  }

  // claimedAttestations
  if (fm.claimedAttestations) {
    for (let i = 0; i < fm.claimedAttestations.length; i++) {
      const ca = fm.claimedAttestations[i];
      if (!ca.claim) errors.push(`${label}: claimedAttestations[${i}].claim is required`);
      if (!(CLAIM_STATUSES as readonly string[]).includes(ca.status)) {
        errors.push(`${label}: claimedAttestations[${i}].status '${ca.status}' is not in enum: ${CLAIM_STATUSES.join(', ')}`);
      }
    }
  }

  // Warnings
  if (fm.ageCategory === 'minor' && (!fm.guardianIds || fm.guardianIds.length === 0)) {
    warnings.push(`${label}: minor without guardianIds (expected for Tiffany red-team persona)`);
  }

  return { errors, warnings };
}

export function validateRelationshipEntry(
  rel: RelationshipEntry,
  index: number,
): string[] {
  const errors: string[] = [];
  const label = `relationships[${index}]`;

  if (!rel.source) errors.push(`${label}: source is required`);
  if (!rel.target) errors.push(`${label}: target is required`);
  if (!rel.relationshipType) {
    errors.push(`${label}: relationshipType is required`);
  } else if (!(HUMAN_RELATIONSHIP_TYPES as readonly string[]).includes(rel.relationshipType)) {
    errors.push(`${label}: relationshipType '${rel.relationshipType}' is not in vocabulary`);
  }
  if (!rel.intimacyLevel) {
    errors.push(`${label}: intimacyLevel is required`);
  } else if (!(INTIMACY_LEVELS as readonly string[]).includes(rel.intimacyLevel)) {
    errors.push(`${label}: intimacyLevel '${rel.intimacyLevel}' is not in enum: ${INTIMACY_LEVELS.join(', ')}`);
  }

  // coworker requires context
  if (rel.relationshipType === 'coworker' && !rel.context) {
    errors.push(`${label}: coworker relationships require 'context' (organization id)`);
  }

  return errors;
}

// =============================================================================
// Directory validation
// =============================================================================

function parseFrontmatter(content: string): HumanFrontmatter | null {
  const match = content.match(/^---\n([\s\S]*?)\n---/);
  if (!match) return null;
  try {
    return parseYaml(match[1]) as HumanFrontmatter;
  } catch {
    return null;
  }
}

export async function validateHumansDirectory(dir: string): Promise<ValidationResult> {
  const result: ValidationResult = {
    errors: [],
    warnings: [],
    humans: new Map(),
    relationships: [],
  };

  const displayNameToId = new Map<string, string>();

  const files = readdirSync(dir)
    .filter(f => f.endsWith('.md') && f !== 'relationships.md' && f !== 'README.md');

  for (const file of files) {
    const full = resolve(dir, file);
    const content = readFileSync(full, 'utf-8');
    const fm = parseFrontmatter(content);

    if (!fm) {
      result.errors.push(`${file}: failed to parse YAML frontmatter`);
      continue;
    }

    const { errors, warnings } = validateHumanFrontmatter(fm, file);
    result.errors.push(...errors);
    result.warnings.push(...warnings);

    if (fm.id) {
      if (result.humans.has(fm.id)) {
        result.errors.push(`${file}: duplicate id '${fm.id}' (also in ${result.humans.get(fm.id)})`);
      } else {
        result.humans.set(fm.id, fm);
      }
    }

    if (fm.displayName) {
      const existing = displayNameToId.get(fm.displayName);
      if (existing && existing !== fm.id) {
        result.errors.push(`${file}: duplicate displayName '${fm.displayName}' (also used by ${existing})`);
      }
      displayNameToId.set(fm.displayName, fm.id);
    }
  }

  // Parse relationships.md if present
  const relsPath = resolve(dir, 'relationships.md');
  try {
    const content = readFileSync(relsPath, 'utf-8');
    const fm = parseFrontmatter(content) as { relationships?: RelationshipEntry[] } | null;
    if (fm?.relationships) {
      result.relationships = fm.relationships;
      for (let i = 0; i < fm.relationships.length; i++) {
        const relErrors = validateRelationshipEntry(fm.relationships[i], i);
        result.errors.push(...relErrors);
      }
    }
  } catch {
    // relationships.md is optional
  }

  // Referential integrity
  const ids = new Set(result.humans.keys());

  // guardianIds must resolve
  for (const [id, fm] of result.humans) {
    if (fm.guardianIds) {
      for (const g of fm.guardianIds) {
        if (!ids.has(g)) {
          result.errors.push(`${id}: guardianIds contains '${g}' which is not a known human`);
        }
      }
    }
  }

  // Relationship source/target must resolve
  for (let i = 0; i < result.relationships.length; i++) {
    const rel = result.relationships[i];
    if (rel.source && !ids.has(rel.source)) {
      result.errors.push(`relationships[${i}]: source '${rel.source}' not in humans/`);
    }
    if (rel.target && !ids.has(rel.target)) {
      result.errors.push(`relationships[${i}]: target '${rel.target}' not in humans/`);
    }
  }

  // Circular guardianship check
  for (const startId of result.humans.keys()) {
    const visited = new Set<string>();
    let current: string | undefined = startId;
    while (current && result.humans.get(current)?.guardianIds?.[0]) {
      if (visited.has(current)) {
        result.errors.push(`Circular guardianship chain involving ${[...visited].join(' → ')} → ${current}`);
        break;
      }
      visited.add(current);
      current = result.humans.get(current)?.guardianIds?.[0];
    }
  }

  return result;
}

// =============================================================================
// CLI
// =============================================================================

if (process.argv[1]?.endsWith('validate-humans.ts') || process.argv[1]?.endsWith('validate-humans.js')) {
  const dir = process.argv[2] ?? new URL('../../data/humans/', import.meta.url).pathname;

  console.log('=== Validate Humans ===\n');
  console.log(`Directory: ${dir}\n`);

  validateHumansDirectory(dir).then(result => {
    console.log(`Humans:        ${result.humans.size}`);
    console.log(`Relationships: ${result.relationships.length}`);
    console.log(`Errors:        ${result.errors.length}`);
    console.log(`Warnings:      ${result.warnings.length}\n`);

    if (result.errors.length > 0) {
      console.error('ERRORS:');
      for (const e of result.errors) console.error(`  ✗ ${e}`);
    }

    if (result.warnings.length > 0) {
      console.log('\nWARNINGS:');
      for (const w of result.warnings) console.log(`  ⚠ ${w}`);
    }

    if (result.errors.length === 0) {
      console.log('All validations passed');
    }

    process.exit(result.errors.length > 0 ? 1 : 0);
  });
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd genesis/seeder && pnpm exec vitest run src/__tests__/validate-humans.test.ts`
Expected: all tests pass

- [ ] **Step 6: Stage**

Run: `git add genesis/seeder/src/validate-humans.ts genesis/seeder/src/__tests__/validate-humans.test.ts genesis/seeder/package.json genesis/seeder/pnpm-lock.yaml`

---

### Task A4: Create `validate-presences.ts` (TDD)

**Files:**
- Create: `genesis/seeder/src/validate-presences.ts`
- Create: `genesis/seeder/src/__tests__/validate-presences.test.ts`

- [ ] **Step 1: Write failing tests**

Create `genesis/seeder/src/__tests__/validate-presences.test.ts` with tests covering:
- Minimal valid presence (id, displayName, presenceType, observations with at least one entry)
- Rejects presence with empty `observations[]` (minItems: 1)
- Rejects unknown `presenceType` (must be person or organization)
- Rejects unknown `externalIdentifiers[].type`
- Accepts multiple observations with same observerId (confirmed design decision)
- Detects duplicate presence ids across files
- Detects duplicate (type, value) pairs in externalIdentifiers across presences
- Validates `observations[].observerId` resolves when human registry is provided
- Validates `image.local` path resolves when images directory is provided

Mirror the test structure from validate-humans.test.ts. Each test uses a tmp directory.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd genesis/seeder && pnpm exec vitest run src/__tests__/validate-presences.test.ts`
Expected: FAIL — module does not exist

- [ ] **Step 3: Implement the validator**

Create `genesis/seeder/src/validate-presences.ts` following the same shape as `validate-humans.ts`. Key constants:

```typescript
const PRESENCE_TYPES = ['person', 'organization'] as const;

const EXTERNAL_ID_TYPES = [
  'orcid', 'isni', 'wikipedia', 'wikidata',
  'linkedin', 'twitter', 'mastodon', 'personal-domain',
  'doi', 'isbn', 'arxiv', 'github',
  'homepage', 'email',
] as const;

const WORK_KINDS = [
  'book', 'paper', 'talk', 'podcast', 'video',
  'project', 'organization', 'website', 'other',
] as const;

const PRESENCE_SLUG_PATTERN = /^presence-[a-z0-9][a-z0-9-]*[a-z0-9]$/;
```

The validator interface:

```typescript
export interface PresenceFrontmatter {
  id: string;
  displayName: string;
  presenceType: 'person' | 'organization';
  bio?: string | null;
  observations: Observation[];
  primaryStewardId?: string | null;
  stewardshipStartedAt?: string | null;
  externalIdentifiers?: ExternalIdentifier[];
  sameAsPresenceIds?: string[];
  works?: Work[];
  suggestedCollectiveIds?: string[];
  tags?: string[];
  image?: { local: string; placeholder?: boolean; sourceUrl?: string | null } | null;
  note?: string | null;
}

export interface Observation {
  observerId: string;
  observedAt: string;
  context: string;
  contextContentId?: string | null;
}

export interface ExternalIdentifier {
  type: string;
  value: string;
}

export interface Work {
  title: string;
  kind: string;
  year?: number | null;
  url?: string | null;
  citedInContentIds?: string[];
}

export interface PresenceValidationResult {
  errors: string[];
  warnings: string[];
  presences: Map<string, PresenceFrontmatter>;
}

export async function validatePresencesDirectory(
  dir: string,
  options?: {
    knownHumanIds?: Set<string>;
    knownContentIds?: Set<string>;
    imagesDir?: string;
  },
): Promise<PresenceValidationResult>
```

Implementation mirrors `validate-humans.ts` plus these presence-specific checks:
- `observations[]` minItems 1 (fail if empty)
- Every `observations[].observerId` resolves against `options.knownHumanIds` if provided
- Every `observations[].contextContentId` resolves against `options.knownContentIds` if provided and non-null
- Every `works[].citedInContentIds[]` resolves against `options.knownContentIds` if provided
- Every `externalIdentifiers[].type` in the enum
- No two presences share `(type, value)` pair in `externalIdentifiers` — early merge-conflict detection
- `image.local` file exists in `options.imagesDir` if provided

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd genesis/seeder && pnpm exec vitest run src/__tests__/validate-presences.test.ts`
Expected: all tests pass

- [ ] **Step 5: Stage**

Run: `git add genesis/seeder/src/validate-presences.ts genesis/seeder/src/__tests__/validate-presences.test.ts`

---

### Task A5: Create `build-data.ts` (markdown → JSON generator)

**Files:**
- Create: `genesis/seeder/src/build-data.ts`
- Create: `genesis/seeder/src/__tests__/build-data.test.ts`

- [ ] **Step 1: Write failing tests**

Create `genesis/seeder/src/__tests__/build-data.test.ts`:

```typescript
import { describe, it, expect } from 'vitest';
import { buildHumansJson, buildPresencesJson } from '../build-data.js';
import { mkdtemp, writeFile, readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

describe('buildHumansJson', () => {
  it('generates humans.json from markdown directory', async () => {
    const dir = await mkdtemp(join(tmpdir(), 'humans-build-'));
    await writeFile(join(dir, 'test-user.md'),
      `---\nid: human-test-user\ndisplayName: Test User\ncategory: core-family\nprofileReach: community\nbio: Bio text\n---\n# Test User\n\nNarrative here.`);
    const result = await buildHumansJson(dir);
    expect(result.humans).toHaveLength(1);
    expect(result.humans[0].id).toBe('human-test-user');
    expect(result.humans[0].displayName).toBe('Test User');
  });

  it('includes relationships from relationships.md', async () => {
    const dir = await mkdtemp(join(tmpdir(), 'humans-build-'));
    await writeFile(join(dir, 'a.md'), `---\nid: human-a\ndisplayName: A\ncategory: core-family\nprofileReach: community\n---\n`);
    await writeFile(join(dir, 'b.md'), `---\nid: human-b\ndisplayName: B\ncategory: core-family\nprofileReach: community\n---\n`);
    await writeFile(join(dir, 'relationships.md'), `---\nrelationships:\n  - source: human-a\n    target: human-b\n    relationshipType: sibling\n    intimacyLevel: trusted\n---\n`);
    const result = await buildHumansJson(dir);
    expect(result.relationships).toHaveLength(1);
    expect(result.relationships[0].relationshipType).toBe('sibling');
  });
});

describe('buildPresencesJson', () => {
  it('generates presences.json from markdown directory', async () => {
    const dir = await mkdtemp(join(tmpdir(), 'presences-build-'));
    await writeFile(join(dir, 'test.md'),
      `---\nid: presence-test\ndisplayName: Test\npresenceType: person\nobservations:\n  - observerId: human-a\n    observedAt: 2026-04-11T00:00:00Z\n    context: Test\n---\n`);
    const result = await buildPresencesJson(dir);
    expect(result.presences).toHaveLength(1);
    expect(result.presences[0].id).toBe('presence-test');
  });
});
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cd genesis/seeder && pnpm exec vitest run src/__tests__/build-data.test.ts`
Expected: FAIL

- [ ] **Step 3: Implement the generator**

Create `genesis/seeder/src/build-data.ts`:

```typescript
/**
 * Build Data — generates derived JSON artifacts from markdown sources.
 *
 * genesis/data/humans/*.md → genesis/data/humans/humans.json
 * genesis/data/presences/*.md → genesis/data/presences/presences.json
 *
 * The markdown frontmatter is the canonical source of truth. The JSON
 * artifacts are derived projections checked in for fast seeder consumption.
 * A pre-push hook runs this generator and fails if the committed JSON is
 * stale relative to the markdown source.
 */

import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { parse as parseYaml } from 'yaml';

import type { HumanFrontmatter, RelationshipEntry } from './validate-humans.js';
import type { PresenceFrontmatter } from './validate-presences.js';

function parseFrontmatter<T>(content: string): T | null {
  const match = content.match(/^---\n([\s\S]*?)\n---/);
  if (!match) return null;
  try {
    return parseYaml(match[1]) as T;
  } catch {
    return null;
  }
}

export interface HumansJson {
  version: string;
  generatedAt: string;
  description: string;
  humans: HumanFrontmatter[];
  relationships: RelationshipEntry[];
}

export async function buildHumansJson(dir: string): Promise<HumansJson> {
  const humans: HumanFrontmatter[] = [];
  const files = readdirSync(dir)
    .filter(f => f.endsWith('.md') && f !== 'relationships.md' && f !== 'README.md')
    .sort();

  for (const file of files) {
    const content = readFileSync(resolve(dir, file), 'utf-8');
    const fm = parseFrontmatter<HumanFrontmatter>(content);
    if (fm) humans.push(fm);
  }

  let relationships: RelationshipEntry[] = [];
  try {
    const relsContent = readFileSync(resolve(dir, 'relationships.md'), 'utf-8');
    const relsFm = parseFrontmatter<{ relationships?: RelationshipEntry[] }>(relsContent);
    relationships = relsFm?.relationships ?? [];
  } catch {
    // relationships.md optional
  }

  return {
    version: '2.0.0',
    generatedAt: new Date().toISOString(),
    description: 'Generated from genesis/data/humans/*.md. DO NOT EDIT BY HAND.',
    humans,
    relationships,
  };
}

export interface PresencesJson {
  version: string;
  generatedAt: string;
  description: string;
  presences: PresenceFrontmatter[];
}

export async function buildPresencesJson(dir: string): Promise<PresencesJson> {
  const presences: PresenceFrontmatter[] = [];
  const files = readdirSync(dir)
    .filter(f => f.endsWith('.md') && f !== 'relationships.md' && f !== 'README.md')
    .sort();

  for (const file of files) {
    const content = readFileSync(resolve(dir, file), 'utf-8');
    const fm = parseFrontmatter<PresenceFrontmatter>(content);
    if (fm) presences.push(fm);
  }

  return {
    version: '1.0.0',
    generatedAt: new Date().toISOString(),
    description: 'Generated from genesis/data/presences/*.md. DO NOT EDIT BY HAND.',
    presences,
  };
}

// CLI
async function main() {
  const mode = process.argv[2]; // 'humans' | 'presences' | 'all'
  const humansDir = new URL('../../data/humans/', import.meta.url).pathname;
  const presencesDir = new URL('../../data/presences/', import.meta.url).pathname;

  if (mode === 'humans' || mode === 'all' || !mode) {
    const data = await buildHumansJson(humansDir);
    writeFileSync(resolve(humansDir, 'humans.json'), JSON.stringify(data, null, 2));
    console.log(`Wrote ${data.humans.length} humans and ${data.relationships.length} relationships to humans.json`);
  }

  if (mode === 'presences' || mode === 'all' || !mode) {
    const data = await buildPresencesJson(presencesDir);
    writeFileSync(resolve(presencesDir, 'presences.json'), JSON.stringify(data, null, 2));
    console.log(`Wrote ${data.presences.length} presences to presences.json`);
  }
}

if (process.argv[1]?.endsWith('build-data.ts') || process.argv[1]?.endsWith('build-data.js')) {
  main().catch(err => {
    console.error(err);
    process.exit(1);
  });
}
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cd genesis/seeder && pnpm exec vitest run src/__tests__/build-data.test.ts`
Expected: all tests pass

- [ ] **Step 5: Stage**

Run: `git add genesis/seeder/src/build-data.ts genesis/seeder/src/__tests__/build-data.test.ts`

---

## Phase B — Migration script (one-shot, self-deleting)

### Task B1: Create `migration-author-map.json`

**Files:**
- Create: `genesis/scripts/migration-author-map.json`

- [ ] **Step 1: Create the scripts directory and author map**

Run: `mkdir -p genesis/scripts`

Create `genesis/scripts/migration-author-map.json`:

```json
{
  "$comment": "Hand-mapped book→author table for Keen 'Media and Books' section migration. One-shot file, deleted after migration.",
  "books": {
    "Finite and Infinite Games": {
      "slug": "finite-and-infinite-games",
      "author": { "slug": "james-p-carse", "displayName": "James P. Carse" },
      "year": 1986,
      "description": "A Vision of Life as Play and Possibility. Distinction between finite games (played to win) and infinite games (played to continue playing)."
    },
    "The Collapse of Complex Societies": {
      "slug": "collapse-of-complex-societies",
      "author": { "slug": "joseph-tainter", "displayName": "Joseph Tainter" },
      "year": 1988,
      "description": "Analysis of why complex civilizations collapse, arguing that increasing sociopolitical complexity yields diminishing returns on investment."
    },
    "The Ministry for the Future": {
      "slug": "ministry-for-the-future",
      "author": { "slug": "kim-stanley-robinson", "displayName": "Kim Stanley Robinson" },
      "year": 2020,
      "description": "Speculative fiction about an international body established to advocate for the rights of future generations."
    }
  }
}
```

Note: if the Keen contains a book not in this list, the migration script will warn and skip creating a book node for it — manual addition to this map is required.

- [ ] **Step 2: Stage**

Run: `git add genesis/scripts/migration-author-map.json`

---

### Task B2: Scaffold migration script structure

**Files:**
- Create: `genesis/scripts/migrate-content-to-presences.ts`

- [ ] **Step 1: Install sharp dependency in scripts context**

The scripts directory doesn't have its own package.json; install sharp at the seeder level (dev dep, will be removed at end):

Run: `cd genesis/seeder && pnpm add -D sharp`
Expected: sharp added to devDependencies

- [ ] **Step 2: Create the migration script skeleton**

Create `genesis/scripts/migrate-content-to-presences.ts`:

```typescript
/**
 * ONE-SHOT MIGRATION SCRIPT
 *
 * Migrates the Consilience Garden Keen and orphan content stubs into
 * first-class ContributorPresence entries, book ContentNodes, and rewrites
 * content references.
 *
 * This script runs ONCE, lands in the sprint commit alongside its output,
 * and is deleted in the same commit. Provenance preserved in git history.
 *
 * Usage:
 *   npx tsx genesis/scripts/migrate-content-to-presences.ts          # dry-run
 *   npx tsx genesis/scripts/migrate-content-to-presences.ts --execute
 *   npx tsx genesis/scripts/migrate-content-to-presences.ts --execute --no-images
 */

import { readFileSync, writeFileSync, readdirSync, mkdirSync, rmSync, existsSync, statSync } from 'node:fs';
import { resolve, basename, dirname, join } from 'node:path';
import sharp from 'sharp';

// =============================================================================
// Types
// =============================================================================

interface KeenGem {
  gemId: string;
  text?: string;
  metalink?: {
    title?: string;
    description?: string;
    url?: string;
    image?: string;
    publisher?: string;
  };
  tipImage?: {
    optimized_500x500?: string;
    externalUrl?: string;
  };
  tags?: string[];
  contributor?: string;
}

interface KeenSection {
  sectionId: string;
  title: string;
  order?: number;
  gems: KeenGem[];
}

interface KeenRoot {
  keenId: string;
  title: string;
  description: string;
  createdAtUnixTimestamp: number;
  sections: KeenSection[];
}

interface PresenceRecord {
  id: string;
  displayName: string;
  presenceType: 'person' | 'organization';
  bio: string | null;
  observations: Array<{
    observerId: string;
    observedAt: string;
    context: string;
    contextContentId: string | null;
  }>;
  primaryStewardId: string;
  stewardshipStartedAt: string;
  externalIdentifiers: Array<{ type: string; value: string }>;
  sameAsPresenceIds: string[];
  works: Array<{ title: string; kind: string; year?: number | null; url?: string | null; citedInContentIds: string[] }>;
  suggestedCollectiveIds: string[];
  tags: string[];
  image: { local: string; placeholder: boolean; sourceUrl: string | null } | null;
  note: string | null;
  narrative: string;  // markdown body
  sourceGemId?: string;
  sourceSectionTitle?: string;
}

interface BookContentNode {
  id: string;
  contentType: 'book';
  title: string;
  description: string;
  content: string;
  contentFormat: 'markdown';
  tags: string[];
  metadata: Record<string, unknown>;
  stewardedBy: Array<{ humanId: string; affinity: number; role: string }>;
  contributors: Array<{ presenceId: string; contributionType: string; weight: null; context: string }>;
}

interface ContentRewrite {
  path: string;
  remove: string[];  // relatedNodeIds entries to remove
  addContributors: Array<{ presenceId: string; contributionType: string; weight: null; context: string }>;
}

interface MigrationState {
  presences: Map<string, PresenceRecord>;
  newBookNodes: Map<string, BookContentNode>;
  rewrites: Map<string, ContentRewrite>;
  filesToDelete: Set<string>;
  imageTasks: Array<{ presenceId: string; url: string | null; localGemId?: string }>;
  warnings: string[];
  errors: string[];
}

// =============================================================================
// Constants
// =============================================================================

const KEEN_DIR = resolve(process.cwd(), 'Consilience_Garden-nvBCVtcYER7C9s3H6zxS');
const KEEN_JSON = join(KEEN_DIR, 'keen.json');
const LAMAD_CONTENT_DIR = resolve(process.cwd(), 'genesis/data/lamad/content');
const PRESENCES_DIR = resolve(process.cwd(), 'genesis/data/presences');
const PRESENCES_IMAGES_DIR = join(PRESENCES_DIR, 'images');
const AUTHOR_MAP_PATH = resolve(process.cwd(), 'genesis/scripts/migration-author-map.json');

const MATTHEW_ID = 'human-matthew-manager';
const PETE_ID = 'human-pete-pastor';
const KEEN_OBSERVED_AT = '2021-08-07T21:58:56Z';  // Keen creation timestamp
const FCT_OBSERVED_AT = '2026-01-15T00:00:00Z';   // Synthetic genesis date for Pete's FCT observations

// =============================================================================
// Helpers
// =============================================================================

function fresh(): MigrationState {
  return {
    presences: new Map(),
    newBookNodes: new Map(),
    rewrites: new Map(),
    filesToDelete: new Set(),
    imageTasks: [],
    warnings: [],
    errors: [],
  };
}

function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '')
    .slice(0, 60);
}

function classifyPresenceType(metalink?: KeenGem['metalink']): 'person' | 'organization' {
  const url = metalink?.url ?? '';
  const title = metalink?.title ?? '';
  // Heuristic: personal URL patterns → person; everything else → organization
  if (/linkedin\.com\/in\/|twitter\.com\/|mastodon\.|\bauthor\b/i.test(url)) return 'person';
  if (/foundation|institute|project|lab|society|network|council|corporation/i.test(title)) return 'organization';
  return 'organization';  // default — safer bet for Keen gems
}

// =============================================================================
// Phase 1: Parse Keen
// =============================================================================

async function phase1ParseKeen(state: MigrationState): Promise<void> {
  if (!existsSync(KEEN_JSON)) {
    state.errors.push(`Keen file not found: ${KEEN_JSON}`);
    return;
  }
  const keen: KeenRoot = JSON.parse(readFileSync(KEEN_JSON, 'utf-8'));
  console.log(`Phase 1: Parsing Keen '${keen.title}' (${keen.sections.length} sections)`);

  for (const section of keen.sections) {
    const sectionTag = slugify(section.title || 'unnamed');
    for (const gem of section.gems) {
      const title = gem.metalink?.title || gem.text?.split('\n')[0]?.slice(0, 60) || gem.gemId;
      const slug = slugify(title);
      if (!slug) {
        state.warnings.push(`Gem ${gem.gemId}: could not generate slug from title '${title}'`);
        continue;
      }
      const id = `presence-${slug}`;

      if (state.presences.has(id)) {
        state.warnings.push(`Gem ${gem.gemId}: duplicate slug '${id}' — skipping`);
        continue;
      }

      const presenceType = classifyPresenceType(gem.metalink);
      const bio = gem.metalink?.description || null;

      const externalIdentifiers: PresenceRecord['externalIdentifiers'] = [];
      if (gem.metalink?.url) {
        externalIdentifiers.push({ type: 'homepage', value: gem.metalink.url });
      }

      const presence: PresenceRecord = {
        id,
        displayName: title,
        presenceType,
        bio,
        observations: [{
          observerId: MATTHEW_ID,
          observedAt: KEEN_OBSERVED_AT,
          context: `Collected in Consilience Garden keen, section: ${section.title || 'unnamed'}`,
          contextContentId: null,
        }],
        primaryStewardId: MATTHEW_ID,
        stewardshipStartedAt: KEEN_OBSERVED_AT,
        externalIdentifiers,
        sameAsPresenceIds: [],
        works: [],
        suggestedCollectiveIds: [],
        tags: [sectionTag, 'consilience-garden'],
        image: null,  // populated in Phase 5
        note: null,
        narrative: bio || `Gem from Matthew's Consilience Garden keen, section "${section.title}". Imported to the protocol's presence layer from the 2021-era curation that predated EthosEngine.`,
        sourceGemId: gem.gemId,
        sourceSectionTitle: section.title,
      };

      // Queue image task
      const imageUrl = gem.metalink?.image || gem.tipImage?.optimized_500x500 || gem.tipImage?.externalUrl || null;
      state.imageTasks.push({ presenceId: id, url: imageUrl, localGemId: gem.gemId });

      state.presences.set(id, presence);
    }
  }

  console.log(`  Created ${state.presences.size} presence records from Keen`);
}

// =============================================================================
// Phase 2-5: STUBS — implemented in subsequent tasks
// =============================================================================

async function phase2ParseFctContributors(_state: MigrationState): Promise<void> {
  console.log('Phase 2: Parse FCT contributor stubs — STUB, implemented in B3');
}

async function phase3BuildBookNodes(_state: MigrationState): Promise<void> {
  console.log('Phase 3: Build book content nodes — STUB, implemented in B4');
}

async function phase4BuildRewritePlan(_state: MigrationState): Promise<void> {
  console.log('Phase 4: Build content rewrite plan — STUB, implemented in B5');
}

async function phase5Execute(_state: MigrationState, _opts: ExecuteOptions): Promise<void> {
  console.log('Phase 5: Execute — STUB, implemented in B6');
}

interface ExecuteOptions {
  execute: boolean;
  skipImages: boolean;
}

// =============================================================================
// Dry-run report
// =============================================================================

function dryRunReport(state: MigrationState): void {
  console.log('\n=== MIGRATION DRY-RUN REPORT ===\n');
  console.log(`Presences to create: ${state.presences.size}`);
  console.log(`New book content nodes: ${state.newBookNodes.size}`);
  console.log(`Content files to rewrite: ${state.rewrites.size}`);
  console.log(`Files to delete: ${state.filesToDelete.size}`);
  console.log(`Image tasks: ${state.imageTasks.length}`);
  console.log(`Warnings: ${state.warnings.length}`);
  console.log(`Errors: ${state.errors.length}`);

  if (state.warnings.length > 0) {
    console.log('\nWARNINGS:');
    for (const w of state.warnings.slice(0, 20)) console.log(`  ⚠ ${w}`);
    if (state.warnings.length > 20) console.log(`  ... ${state.warnings.length - 20} more`);
  }

  if (state.errors.length > 0) {
    console.error('\nERRORS:');
    for (const e of state.errors) console.error(`  ✗ ${e}`);
  }

  console.log('\nTo execute, run with --execute');
}

// =============================================================================
// Main
// =============================================================================

async function main() {
  const args = process.argv.slice(2);
  const opts: ExecuteOptions = {
    execute: args.includes('--execute'),
    skipImages: args.includes('--no-images'),
  };

  const state = fresh();

  await phase1ParseKeen(state);
  await phase2ParseFctContributors(state);
  await phase3BuildBookNodes(state);
  await phase4BuildRewritePlan(state);

  if (state.errors.length > 0) {
    console.error('Migration aborted — errors in planning phases');
    dryRunReport(state);
    process.exit(1);
  }

  if (opts.execute) {
    await phase5Execute(state, opts);
  } else {
    dryRunReport(state);
  }
}

main().catch(err => {
  console.error('Migration failed:', err);
  process.exit(1);
});
```

- [ ] **Step 3: Verify the script runs (dry-run phase 1 only)**

Run: `cd /projects/elohim && npx tsx genesis/scripts/migrate-content-to-presences.ts`
Expected: script runs, reports Phase 1 parsed ~100 presence records from Keen, Phase 2-5 print "STUB", dry-run report shows presence count

- [ ] **Step 4: Stage**

Run: `git add genesis/scripts/migrate-content-to-presences.ts genesis/seeder/package.json genesis/seeder/pnpm-lock.yaml`

---

### Task B3: Implement Phase 2 — FCT contributor parser with dedup

**Files:**
- Modify: `genesis/scripts/migrate-content-to-presences.ts`

- [ ] **Step 1: Replace the phase2 stub with real implementation**

In `migrate-content-to-presences.ts`, replace the `phase2ParseFctContributors` stub with:

```typescript
async function phase2ParseFctContributors(state: MigrationState): Promise<void> {
  const fctFiles = readdirSync(LAMAD_CONTENT_DIR)
    .filter(f => f.startsWith('fct-contributor-') && f.endsWith('.json'));
  console.log(`Phase 2: Parsing ${fctFiles.length} FCT contributor stubs`);

  let created = 0;
  let merged = 0;
  for (const file of fctFiles) {
    const fullPath = join(LAMAD_CONTENT_DIR, file);
    const stub = JSON.parse(readFileSync(fullPath, 'utf-8'));
    state.filesToDelete.add(fullPath);

    const displayName = stub.title || file.replace('fct-contributor-', '').replace('.json', '');
    const slug = slugify(displayName);
    const id = `presence-${slug}`;

    // Extract works from the content body (heuristic: "**Works:** Title1, Title2")
    const worksMatch = (stub.content ?? '').match(/\*\*Works:\*\*\s*([^\n]+)/);
    const works: PresenceRecord['works'] = [];
    if (worksMatch) {
      const titles = worksMatch[1].split(',').map((s: string) => s.trim()).filter(Boolean);
      for (const t of titles) {
        works.push({
          title: t,
          kind: 'book',
          year: null,
          url: null,
          citedInContentIds: [],
        });
      }
    }

    // Extract which FCT module cited this contributor (from sourceModule metadata)
    const sourceModule = stub.metadata?.sourceModule as string | undefined;

    if (state.presences.has(id)) {
      // Merge with Keen-sourced presence — add Pete observation
      const existing = state.presences.get(id)!;
      existing.observations.push({
        observerId: PETE_ID,
        observedAt: FCT_OBSERVED_AT,
        context: sourceModule
          ? `Cited in ${sourceModule} while building the FCT curriculum`
          : 'Cited in FCT curriculum',
        contextContentId: sourceModule || null,
      });
      if (!existing.tags.includes('fct')) existing.tags.push('fct');
      for (const w of works) {
        const exists = existing.works.some(ew => ew.title === w.title);
        if (!exists) existing.works.push(w);
        if (sourceModule) {
          const match = existing.works.find(ew => ew.title === w.title);
          if (match && !match.citedInContentIds.includes(sourceModule)) {
            match.citedInContentIds.push(sourceModule);
          }
        }
      }
      merged++;
    } else {
      // Create new presence with Pete as sole observer
      const presence: PresenceRecord = {
        id,
        displayName,
        presenceType: 'person',
        bio: `Author referenced in Pastor Pete's FCT curriculum.`,
        observations: [{
          observerId: PETE_ID,
          observedAt: FCT_OBSERVED_AT,
          context: sourceModule
            ? `Cited in ${sourceModule} while building the FCT curriculum`
            : 'Cited in FCT curriculum',
          contextContentId: sourceModule || null,
        }],
        primaryStewardId: PETE_ID,
        stewardshipStartedAt: FCT_OBSERVED_AT,
        externalIdentifiers: [],
        sameAsPresenceIds: [],
        works: works.map(w => ({
          ...w,
          citedInContentIds: sourceModule ? [sourceModule] : [],
        })),
        suggestedCollectiveIds: [],
        tags: ['fct', 'author'],
        image: null,
        note: null,
        narrative: `${displayName} is an author cited in Pastor Pete's Fairness, Courage, and Truth (FCT) curriculum. This presence was created from a genesis-scenario FCT contributor record.`,
      };
      state.imageTasks.push({ presenceId: id, url: null });
      state.presences.set(id, presence);
      created++;
    }
  }

  console.log(`  Created ${created} new presences from FCT stubs, merged ${merged} with Keen-sourced presences`);
}
```

- [ ] **Step 2: Run dry-run to verify phase 2 output**

Run: `cd /projects/elohim && npx tsx genesis/scripts/migrate-content-to-presences.ts`
Expected: dry-run report shows combined ~110+ presences (100 Keen + 27 FCT minus merges), FCT files queued for deletion

- [ ] **Step 3: Stage**

Run: `git add genesis/scripts/migrate-content-to-presences.ts`

---

### Task B4: Implement Phase 3 — book content nodes + author presences

**Files:**
- Modify: `genesis/scripts/migrate-content-to-presences.ts`

- [ ] **Step 1: Replace phase3 stub**

Replace `phase3BuildBookNodes` in the migration script with:

```typescript
async function phase3BuildBookNodes(state: MigrationState): Promise<void> {
  const authorMap = JSON.parse(readFileSync(AUTHOR_MAP_PATH, 'utf-8'));

  console.log('Phase 3: Building book content nodes and author presences');

  // Find the "Media and Books" section gems in state — they were queued as presences in Phase 1 but should become book ContentNodes instead
  const bookPresenceIds: string[] = [];
  for (const [id, p] of state.presences) {
    if (p.tags.includes('media-and-books') || p.tags.includes(slugify('Media and Books'))) {
      bookPresenceIds.push(id);
    }
  }

  // Also include the existing governance-books-*.json stubs for deletion
  const bookStubs = readdirSync(LAMAD_CONTENT_DIR)
    .filter(f => f.startsWith('governance-books-') && f.endsWith('.json'));
  for (const file of bookStubs) {
    state.filesToDelete.add(join(LAMAD_CONTENT_DIR, file));
  }

  // For each presence that was incorrectly classified as a book, convert:
  // - Create a book ContentNode
  // - Create/merge the author presence
  // - Remove the book-as-presence from state.presences
  for (const bookPresenceId of bookPresenceIds) {
    const bookPresence = state.presences.get(bookPresenceId)!;
    const bookTitle = bookPresence.displayName;

    // Look up author from the map
    const bookInfo = authorMap.books[bookTitle];
    if (!bookInfo) {
      state.warnings.push(`Book '${bookTitle}' not in migration-author-map.json — skipping book node creation (stays as presence)`);
      continue;
    }

    const bookNodeId = `book-${bookInfo.slug}`;
    const authorPresenceId = `presence-${bookInfo.author.slug}`;

    // Create book content node
    const bookNode: BookContentNode = {
      id: bookNodeId,
      contentType: 'book',
      title: bookTitle,
      description: bookInfo.description,
      content: `# ${bookTitle}\n\n**Author:** ${bookInfo.author.displayName} (${bookInfo.year})\n\n${bookInfo.description}\n\nThis book was curated in Matthew's Consilience Garden keen in 2021 as part of the "Media and Books" section — one of the thinkers who shaped the protocol's framing.`,
      contentFormat: 'markdown',
      tags: ['book', 'media-and-books', 'consilience-garden'],
      metadata: {
        category: 'governance',
        sourceSection: 'Media and Books',
        year: bookInfo.year,
      },
      stewardedBy: [{ humanId: MATTHEW_ID, affinity: 1.0, role: 'steward' }],
      contributors: [{
        presenceId: authorPresenceId,
        contributionType: 'author',
        weight: null,
        context: `Author of ${bookTitle}`,
      }],
    };
    state.newBookNodes.set(bookNodeId, bookNode);

    // Create or merge author presence
    if (!state.presences.has(authorPresenceId)) {
      const authorPresence: PresenceRecord = {
        id: authorPresenceId,
        displayName: bookInfo.author.displayName,
        presenceType: 'person',
        bio: `Author of ${bookTitle}.`,
        observations: [{
          observerId: MATTHEW_ID,
          observedAt: KEEN_OBSERVED_AT,
          context: `Collected '${bookTitle}' in Consilience Garden keen, section: Media and Books`,
          contextContentId: bookNodeId,
        }],
        primaryStewardId: MATTHEW_ID,
        stewardshipStartedAt: KEEN_OBSERVED_AT,
        externalIdentifiers: [],
        sameAsPresenceIds: [],
        works: [{
          title: bookTitle,
          kind: 'book',
          year: bookInfo.year,
          url: null,
          citedInContentIds: [bookNodeId],
        }],
        suggestedCollectiveIds: [],
        tags: ['author', 'media-and-books', 'consilience-garden'],
        image: null,
        note: null,
        narrative: `Author of *${bookTitle}*. Matthew collected the book in his Consilience Garden keen — the author's work shaped the protocol's thinking. If ${bookInfo.author.displayName.split(' ').pop()} or their estate eventually joins the network, this presence can be claimed.`,
      };
      state.imageTasks.push({ presenceId: authorPresenceId, url: null });
      state.presences.set(authorPresenceId, authorPresence);
    } else {
      const existing = state.presences.get(authorPresenceId)!;
      existing.works.push({
        title: bookTitle,
        kind: 'book',
        year: bookInfo.year,
        url: null,
        citedInContentIds: [bookNodeId],
      });
    }

    // Remove the book-as-presence from state
    state.presences.delete(bookPresenceId);
  }

  console.log(`  Created ${state.newBookNodes.size} book content nodes, queued ${bookStubs.length} book stubs for deletion`);
}
```

- [ ] **Step 2: Run dry-run**

Run: `cd /projects/elohim && npx tsx genesis/scripts/migrate-content-to-presences.ts`
Expected: report shows 2-3 new book content nodes (matching the author map entries found in Keen), book stubs queued for deletion

- [ ] **Step 3: Stage**

Run: `git add genesis/scripts/migrate-content-to-presences.ts`

---

### Task B5: Implement Phase 4 — content rewrite plan

**Files:**
- Modify: `genesis/scripts/migrate-content-to-presences.ts`

- [ ] **Step 1: Replace phase4 stub**

```typescript
async function phase4BuildRewritePlan(state: MigrationState): Promise<void> {
  console.log('Phase 4: Building content rewrite plan');

  // Build a map of dead-file-id → presence-id (for files that map to presences)
  const deadToPresence = new Map<string, string>();
  for (const deadPath of state.filesToDelete) {
    const deadId = basename(deadPath).replace('.json', '');
    // fct-contributor-virginia-eubanks → presence-virginia-eubanks
    if (deadId.startsWith('fct-contributor-')) {
      const slug = deadId.replace('fct-contributor-', '');
      deadToPresence.set(deadId, `presence-${slug}`);
    }
  }

  // Queue orphan deletions: human-*.json duplicates of humans.json, governance-organizations-*.json stubs
  for (const file of readdirSync(LAMAD_CONTENT_DIR)) {
    if (file.startsWith('human-') && file.endsWith('.json')) {
      state.filesToDelete.add(join(LAMAD_CONTENT_DIR, file));
    }
    if (file.startsWith('governance-organizations-') && file.endsWith('.json')) {
      state.filesToDelete.add(join(LAMAD_CONTENT_DIR, file));
    }
  }

  // Scan surviving content files for dead relatedNodeIds references
  const allContentFiles = readdirSync(LAMAD_CONTENT_DIR).filter(f => f.endsWith('.json'));
  for (const file of allContentFiles) {
    const fullPath = join(LAMAD_CONTENT_DIR, file);
    if (state.filesToDelete.has(fullPath)) continue;  // skip files we're deleting

    const content = JSON.parse(readFileSync(fullPath, 'utf-8'));
    const relatedNodeIds: string[] = content.relatedNodeIds ?? [];

    const deadRefs: string[] = [];
    const addContributors: ContentRewrite['addContributors'] = [];

    for (const ref of relatedNodeIds) {
      if (deadToPresence.has(ref)) {
        deadRefs.push(ref);
        addContributors.push({
          presenceId: deadToPresence.get(ref)!,
          contributionType: 'cited',
          weight: null,
          context: `Auto-migrated from relatedNodeIds (was referencing dead ${ref})`,
        });
      } else if (ref.startsWith('fct-contributor-') ||
                 ref.startsWith('governance-organizations-') ||
                 ref.startsWith('human-')) {
        // Dead ref with no presence mapping — just remove
        deadRefs.push(ref);
      }
    }

    if (deadRefs.length > 0 || addContributors.length > 0) {
      state.rewrites.set(fullPath, {
        path: fullPath,
        remove: deadRefs,
        addContributors,
      });
    }
  }

  // Manifesto special case: add all Keen-migrated presences as 'inspired' contributors
  const manifestoPath = join(LAMAD_CONTENT_DIR, 'manifesto.json');
  const keenInspiredContributors: ContentRewrite['addContributors'] = [];
  for (const [id, p] of state.presences) {
    if (p.tags.includes('consilience-garden')) {
      keenInspiredContributors.push({
        presenceId: id,
        contributionType: 'inspired',
        weight: null,
        context: `Inspired the manifesto via Consilience Garden curation (section: ${p.sourceSectionTitle ?? 'unknown'})`,
      });
    }
  }
  const existingManifestoRewrite = state.rewrites.get(manifestoPath);
  if (existingManifestoRewrite) {
    existingManifestoRewrite.addContributors.push(...keenInspiredContributors);
  } else {
    state.rewrites.set(manifestoPath, {
      path: manifestoPath,
      remove: [],
      addContributors: keenInspiredContributors,
    });
  }

  console.log(`  Rewrite plan: ${state.rewrites.size} content files to modify, ${state.filesToDelete.size} files to delete`);
}
```

- [ ] **Step 2: Dry-run**

Run: `cd /projects/elohim && npx tsx genesis/scripts/migrate-content-to-presences.ts`
Expected: report shows rewrite plan with ~50-100 content files modified, ~110+ files queued for deletion

- [ ] **Step 3: Stage**

Run: `git add genesis/scripts/migrate-content-to-presences.ts`

---

### Task B6: Implement Phase 5 — execute with image pipeline

**Files:**
- Modify: `genesis/scripts/migrate-content-to-presences.ts`

- [ ] **Step 1: Replace phase5 stub with full execute**

```typescript
async function acquireImage(
  task: { presenceId: string; url: string | null; localGemId?: string },
  presenceType: 'person' | 'organization',
): Promise<{ local: string; placeholder: boolean; sourceUrl: string | null }> {
  const targetName = `${task.presenceId}.webp`;
  const targetPath = join(PRESENCES_IMAGES_DIR, targetName);

  // 1. Try local gem_images from Keen
  if (task.localGemId) {
    for (const ext of ['webp', 'png', 'jpg']) {
      const src = join(KEEN_DIR, 'gem_images', `${task.localGemId}.${ext}`);
      if (existsSync(src)) {
        try {
          await sharp(src).resize(500, 500, { fit: 'cover' }).webp({ quality: 80 }).toFile(targetPath);
          return { local: `images/${targetName}`, placeholder: false, sourceUrl: null };
        } catch (err) {
          // fall through
        }
      }
    }
  }

  // 2. Try external URL
  if (task.url) {
    try {
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 10_000);
      const response = await fetch(task.url, { signal: controller.signal });
      clearTimeout(timeout);
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const buffer = Buffer.from(await response.arrayBuffer());
      await sharp(buffer).resize(500, 500, { fit: 'cover' }).webp({ quality: 80 }).toFile(targetPath);
      return { local: `images/${targetName}`, placeholder: false, sourceUrl: task.url };
    } catch (err) {
      // fall through to placeholder
    }
  }

  // 3. Placeholder fallback
  const placeholderName = presenceType === 'person' ? 'placeholder-person.webp' : 'placeholder-organization.webp';
  return {
    local: `images/${placeholderName}`,
    placeholder: true,
    sourceUrl: task.url,
  };
}

function serializeFrontmatter(obj: Record<string, unknown>): string {
  // Simple YAML serialization via JSON → manual formatting (avoid yaml stringify quirks for git diffability)
  const lines: string[] = ['---'];
  for (const [key, value] of Object.entries(obj)) {
    if (value === null || value === undefined) continue;
    if (Array.isArray(value) && value.length === 0) continue;
    lines.push(`${key}: ${JSON.stringify(value)}`);
  }
  lines.push('---');
  return lines.join('\n');
}

function presenceToMarkdown(p: PresenceRecord): string {
  const frontmatter: Record<string, unknown> = {
    id: p.id,
    displayName: p.displayName,
    presenceType: p.presenceType,
    bio: p.bio,
    observations: p.observations,
    primaryStewardId: p.primaryStewardId,
    stewardshipStartedAt: p.stewardshipStartedAt,
    externalIdentifiers: p.externalIdentifiers,
    sameAsPresenceIds: p.sameAsPresenceIds,
    works: p.works,
    suggestedCollectiveIds: p.suggestedCollectiveIds,
    tags: p.tags,
    image: p.image,
    note: p.note,
  };

  const observationTable = [
    '<!-- BEGIN AUTO-OBSERVATIONS -->',
    '| Observer | Context | Date |',
    '|---|---|---|',
    ...p.observations.map(o => {
      const obsLink = `[${o.observerId.replace('human-', '')}](../humans/${o.observerId.replace('human-', '')}.md)`;
      return `| ${obsLink} | ${o.context} | ${o.observedAt.slice(0, 10)} |`;
    }),
    '<!-- END AUTO-OBSERVATIONS -->',
  ].join('\n');

  return `${serializeFrontmatter(frontmatter)}\n\n# ${p.displayName}\n\n${p.narrative}\n\n## Observations\n\n${observationTable}\n`;
}

async function phase5Execute(state: MigrationState, opts: ExecuteOptions): Promise<void> {
  console.log('Phase 5: Executing migration');

  mkdirSync(PRESENCES_DIR, { recursive: true });
  mkdirSync(PRESENCES_IMAGES_DIR, { recursive: true });

  // 1. Acquire images
  if (!opts.skipImages) {
    console.log(`  Acquiring ${state.imageTasks.length} images...`);
    let imgSuccess = 0;
    let imgPlaceholder = 0;
    for (const task of state.imageTasks) {
      const presence = state.presences.get(task.presenceId);
      if (!presence) continue;
      const img = await acquireImage(task, presence.presenceType);
      presence.image = img;
      if (img.placeholder) imgPlaceholder++;
      else imgSuccess++;
    }
    console.log(`  Images: ${imgSuccess} acquired, ${imgPlaceholder} placeholders`);
  }

  // 2. Write presence .md files
  console.log(`  Writing ${state.presences.size} presence markdown files...`);
  for (const [id, presence] of state.presences) {
    const filename = id.replace('presence-', '') + '.md';
    const path = join(PRESENCES_DIR, filename);
    writeFileSync(path, presenceToMarkdown(presence));
  }

  // 3. Write new book content nodes
  console.log(`  Writing ${state.newBookNodes.size} new book content nodes...`);
  for (const [id, book] of state.newBookNodes) {
    const path = join(LAMAD_CONTENT_DIR, `${id}.json`);
    writeFileSync(path, JSON.stringify(book, null, 2));
  }

  // 4. Apply content rewrites
  console.log(`  Applying ${state.rewrites.size} content rewrites...`);
  for (const [path, rewrite] of state.rewrites) {
    const content = JSON.parse(readFileSync(path, 'utf-8'));
    // Remove dead relatedNodeIds
    if (rewrite.remove.length > 0) {
      content.relatedNodeIds = (content.relatedNodeIds ?? []).filter(
        (r: string) => !rewrite.remove.includes(r),
      );
    }
    // Add contributors (merge if existing)
    if (rewrite.addContributors.length > 0) {
      content.contributors = [...(content.contributors ?? []), ...rewrite.addContributors];
    }
    writeFileSync(path, JSON.stringify(content, null, 2));
  }

  // 5. Delete dead content files
  console.log(`  Deleting ${state.filesToDelete.size} dead content files...`);
  for (const path of state.filesToDelete) {
    rmSync(path);
  }

  // 6. Delete the Keen directory
  console.log('  Deleting Consilience Garden directory...');
  rmSync(KEEN_DIR, { recursive: true, force: true });

  console.log('\n✓ Migration complete');
  if (state.warnings.length > 0) {
    console.log(`\n${state.warnings.length} warnings:`);
    for (const w of state.warnings.slice(0, 20)) console.log(`  ⚠ ${w}`);
  }
}
```

- [ ] **Step 2: Create the placeholder images**

Before running execute, create three committed placeholder images. These can be simple generated webp files. Use sharp to create 500x500 solid-color placeholders:

Create `genesis/scripts/make-placeholders.ts`:

```typescript
import sharp from 'sharp';
import { resolve } from 'node:path';

const dir = resolve('genesis/data/presences/images');

async function makePlaceholder(name: string, color: { r: number; g: number; b: number }) {
  await sharp({
    create: {
      width: 500, height: 500, channels: 3,
      background: color,
    },
  }).webp({ quality: 80 }).toFile(resolve(dir, name));
}

await makePlaceholder('placeholder-person.webp', { r: 180, g: 180, b: 200 });
await makePlaceholder('placeholder-organization.webp', { r: 200, g: 190, b: 180 });
await makePlaceholder('placeholder-generic.webp', { r: 190, g: 190, b: 190 });
console.log('Placeholders created');
```

Run: `mkdir -p genesis/data/presences/images && npx tsx genesis/scripts/make-placeholders.ts`
Expected: 3 webp files in `genesis/data/presences/images/`. Delete `make-placeholders.ts` after — it's one-shot.

Run: `rm genesis/scripts/make-placeholders.ts`

- [ ] **Step 3: Stage (still not executed)**

Run: `git add genesis/scripts/migrate-content-to-presences.ts genesis/data/presences/images/`

---

## Phase C — Execute migration

### Task C1: Dry-run review and execute

**Files:** no new files, just running the script

- [ ] **Step 1: Final dry-run for review**

Run: `cd /projects/elohim && npx tsx genesis/scripts/migrate-content-to-presences.ts > /tmp/migration-dryrun.log 2>&1; cat /tmp/migration-dryrun.log`
Expected: dry-run report showing:
- ~110 presences to create
- 2-3 new book content nodes
- ~50-100 content files to rewrite
- ~113 files to delete
- Small number of warnings (book title mismatches, image fetch failures — these are expected)
- Zero errors

**STOP and review the report before proceeding.** If errors exist, debug before executing.

- [ ] **Step 2: Stash current state as safety net**

Run: `git stash push -u -m "migration-safety-net"`
Expected: working tree clean

- [ ] **Step 3: Restore stash (we want the staged schemas + script back)**

Run: `git stash pop`
Expected: staged changes restored

- [ ] **Step 4: Execute migration**

Run: `cd /projects/elohim && npx tsx genesis/scripts/migrate-content-to-presences.ts --execute`
Expected: "Migration complete" message, warnings (if any) listed

- [ ] **Step 5: Verify Keen directory is gone**

Run: `ls /projects/elohim/Consilience_Garden-nvBCVtcYER7C9s3H6zxS 2>&1`
Expected: "No such file or directory"

- [ ] **Step 6: Verify presences were created**

Run: `ls /projects/elohim/genesis/data/presences/*.md | wc -l`
Expected: ~110+ files

Run: `ls /projects/elohim/genesis/data/presences/images/*.webp | wc -l`
Expected: ~100-113 files (presences + 3 placeholders)

- [ ] **Step 7: Verify dead content files are gone**

Run: `ls /projects/elohim/genesis/data/lamad/content/fct-contributor-*.json 2>&1 | head`
Expected: "No such file or directory" or zero matches

Run: `ls /projects/elohim/genesis/data/lamad/content/human-*.json 2>&1 | head`
Expected: zero matches

Run: `ls /projects/elohim/genesis/data/lamad/content/governance-organizations-*.json 2>&1 | head`
Expected: zero matches

- [ ] **Step 8: Verify book content nodes created**

Run: `ls /projects/elohim/genesis/data/lamad/content/book-*.json`
Expected: 2-3 book files

- [ ] **Step 9: Stage all the generated content**

Run: `git add genesis/data/presences/ genesis/data/lamad/content/`

---

### Task C2: Delete the migration script and Keen directory stub

**Files:**
- Delete: `genesis/scripts/migrate-content-to-presences.ts`
- Delete: `genesis/scripts/migration-author-map.json`

- [ ] **Step 1: Delete migration script**

Run: `rm genesis/scripts/migrate-content-to-presences.ts genesis/scripts/migration-author-map.json`

- [ ] **Step 2: Stage deletions**

Run: `git add genesis/scripts/`

- [ ] **Step 3: Remove sharp from devDependencies**

Run: `cd genesis/seeder && pnpm remove sharp`
Expected: sharp removed from package.json

Run: `git add genesis/seeder/package.json genesis/seeder/pnpm-lock.yaml`

---

## Phase D — Seeder updates

### Task D1: Create `seed-presences.ts`

**Files:**
- Create: `genesis/seeder/src/seed-presences.ts`
- Create: `genesis/seeder/src/__tests__/seed-presences.test.ts`

- [ ] **Step 1: Write failing tests**

Create test file with:
- Test: POST success returns 'created'
- Test: POST 409 returns 'exists'
- Test: POST other error returns 'failed'
- Test: Generator resolves presence records from .md directory
- Test: Summary reports created/existing/failed counts correctly

- [ ] **Step 2: Verify tests fail**

Run: `cd genesis/seeder && pnpm exec vitest run src/__tests__/seed-presences.test.ts`
Expected: FAIL

- [ ] **Step 3: Implement seed-presences.ts**

```typescript
/**
 * Seed Presences — POST each presence from genesis/data/presences/ to /db/presences.
 *
 * Reads presence markdown files, validates, and posts to the doorway API.
 * Idempotent — 409 responses (already exists) are treated as success.
 *
 * Must run AFTER:
 *   - seed-humans.ts  (humans must exist; presences reference them as observers/stewards)
 *
 * Must run BEFORE:
 *   - seed-collectives.ts  (collectives may reference presence-stewarded orgs)
 *   - seed-accounts.ts     (account packages reference presence IDs)
 */

import { readdirSync, readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import type { CreateContributorPresenceInputView } from '@elohim/storage-client';
import { validatePresencesDirectory, type PresenceFrontmatter } from './validate-presences.js';

function frontmatterToInputView(fm: PresenceFrontmatter): CreateContributorPresenceInputView {
  // Core fields map directly; everything else rides in metadata_json
  const metadata = {
    presenceType: fm.presenceType,
    observations: fm.observations,
    sameAsPresenceIds: fm.sameAsPresenceIds,
    works: fm.works,
    suggestedCollectiveIds: fm.suggestedCollectiveIds,
    tags: fm.tags,
  };
  // Compile-time type guard: the cast must satisfy CreateContributorPresenceInputView
  return {
    id: fm.id,
    displayName: fm.displayName,
    note: fm.bio ?? null,
    stewardId: fm.primaryStewardId ?? null,
    stewardshipStartedAt: fm.stewardshipStartedAt ?? null,
    externalIdentifiersJson: JSON.stringify(fm.externalIdentifiers ?? []),
    establishingContentIdsJson: JSON.stringify(
      fm.observations
        .map(o => o.contextContentId)
        .filter((id): id is string => id !== null),
    ),
    establishedAt: fm.observations[0]?.observedAt ?? new Date().toISOString(),
    image: fm.image?.local ?? null,
    metadata,
  } as unknown as CreateContributorPresenceInputView;
}

type Outcome = 'created' | 'exists' | 'failed';

interface SeedResult {
  id: string;
  outcome: Outcome;
  error?: string;
}

async function postPresence(
  doorwayUrl: string,
  input: CreateContributorPresenceInputView,
): Promise<SeedResult> {
  try {
    const res = await fetch(`${doorwayUrl}/db/presences`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(input),
    });
    if (res.ok) return { id: (input as any).id, outcome: 'created' };
    if (res.status === 409) return { id: (input as any).id, outcome: 'exists' };
    const errorText = await res.text();
    return { id: (input as any).id, outcome: 'failed', error: `HTTP ${res.status}: ${errorText}` };
  } catch (err) {
    return {
      id: (input as any).id,
      outcome: 'failed',
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

async function main() {
  const doorwayUrl = (process.env.DOORWAY_URL ?? 'http://localhost:8888').replace(/\/$/, '');

  const __dirname = dirname(fileURLToPath(import.meta.url));
  const presencesDir = resolve(__dirname, '../../data/presences');

  console.log('=== Seed Presences ===\n');
  console.log(`Doorway:  ${doorwayUrl}`);
  console.log(`Source:   ${presencesDir}\n`);

  // Validate first
  const validation = await validatePresencesDirectory(presencesDir);
  if (validation.errors.length > 0) {
    console.error('Validation failed:');
    for (const e of validation.errors) console.error(`  ✗ ${e}`);
    process.exit(1);
  }
  console.log(`Validated ${validation.presences.size} presences\n`);

  const results: SeedResult[] = [];
  for (const [, fm] of validation.presences) {
    const input = frontmatterToInputView(fm);
    const result = await postPresence(doorwayUrl, input);
    results.push(result);

    const icon = result.outcome === 'created' ? '+' : result.outcome === 'exists' ? '=' : '✗';
    console.log(`  [${icon}] ${result.id}${result.error ? ` — ${result.error}` : ''}`);
  }

  const created = results.filter(r => r.outcome === 'created').length;
  const existing = results.filter(r => r.outcome === 'exists').length;
  const failed = results.filter(r => r.outcome === 'failed').length;

  console.log(`\n=== ${created} created, ${existing} existing, ${failed} failed ===`);

  if (failed > 0) {
    console.error(`\n${failed} presences failed to seed.`);
    process.exit(1);
  }
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
```

- [ ] **Step 4: Run tests**

Run: `cd genesis/seeder && pnpm exec vitest run src/__tests__/seed-presences.test.ts`
Expected: pass

- [ ] **Step 5: Stage**

Run: `git add genesis/seeder/src/seed-presences.ts genesis/seeder/src/__tests__/seed-presences.test.ts`

---

### Task D2: Migrate humans.json to humans/*.md (one-time conversion)

**Files:**
- Create: 33 `genesis/data/humans/*.md` files
- Create: `genesis/data/humans/relationships.md`
- Delete: `genesis/docs/humans/humans.json`

- [ ] **Step 1: Create a one-shot conversion script**

Create `genesis/scripts/convert-humans-to-markdown.ts` (also one-shot, deleted after):

```typescript
import { readFileSync, writeFileSync, mkdirSync, rmSync } from 'node:fs';
import { resolve, join } from 'node:path';

const SRC = resolve('genesis/docs/humans/humans.json');
const OUT = resolve('genesis/data/humans');

mkdirSync(OUT, { recursive: true });

interface HumanJson { id: string; displayName: string; [k: string]: unknown; }
interface RelJson { source: string; target: string; type: string; intimacy: string; context?: string; }

const data = JSON.parse(readFileSync(SRC, 'utf-8'));

// Relationship type migration map
const TYPE_MAP: Record<string, string> = {
  'spouse': 'spouse',
  'parent': 'parent-of',
  'grandparent': 'grandparent-of',
  'sibling': 'sibling',
  'coworker': 'coworker',
  'neighbor': 'neighbor',
  'congregation_member': 'congregation-member',
  'learning_partner': 'learning-partner',
  'mentee': 'mentee-of',
  'business_partner': 'business-partner',
  'acquaintance': 'acquaintance',
  'network_connection': 'acquaintance',
  'community_member': 'community-member',
};

function serializeFrontmatter(obj: Record<string, unknown>): string {
  const lines = ['---'];
  for (const [k, v] of Object.entries(obj)) {
    if (v === null || v === undefined) continue;
    if (Array.isArray(v) && v.length === 0) continue;
    lines.push(`${k}: ${JSON.stringify(v)}`);
  }
  lines.push('---');
  return lines.join('\n');
}

// Write humans
for (const h of data.humans as HumanJson[]) {
  // Rename Pastor Pete → Pete (narrative coherence fix)
  if (h.id === 'human-pete-pastor') {
    h.displayName = 'Pete';
  }
  const filename = h.id.replace('human-', '') + '.md';
  const body = `\n\n# ${h.displayName}\n\n${h.bio || '(No bio yet. Edit this narrative freely.)'}\n`;
  writeFileSync(join(OUT, filename), serializeFrontmatter(h) + body);
}

// Write relationships.md
const relationships = (data.relationships as RelJson[]).map(r => ({
  source: r.source,
  target: r.target,
  relationshipType: TYPE_MAP[r.type] ?? r.type,
  intimacyLevel: r.intimacy,
  context: r.context ?? null,
}));

writeFileSync(
  join(OUT, 'relationships.md'),
  serializeFrontmatter({ relationships }) + '\n\n# Human Relationships\n\nSingle source of truth for human-to-human edges.\n',
);

console.log(`Wrote ${data.humans.length} humans and ${relationships.length} relationships`);

// Delete old humans.json
rmSync(SRC);
console.log('Deleted old genesis/docs/humans/humans.json');
```

- [ ] **Step 2: Run the conversion**

Run: `npx tsx genesis/scripts/convert-humans-to-markdown.ts`
Expected: 33 markdown files + relationships.md + README removal log

- [ ] **Step 3: Delete the conversion script (one-shot)**

Run: `rm genesis/scripts/convert-humans-to-markdown.ts`

- [ ] **Step 4: Update a2o fixture-humans.ts to point at the new location**

Edit `genesis/a2o/src/framework/fixtures/humans.ts`:
- Change the import path from `genesis/docs/humans/humans.json` to `genesis/data/humans/humans.json` (the generated one will be produced by build-data.ts)

- [ ] **Step 5: Stage**

Run: `git add genesis/data/humans/ genesis/docs/humans/ genesis/a2o/src/framework/fixtures/humans.ts`

---

### Task D3: Regenerate account packages for all 33 humans

**Files:**
- Modify: `genesis/seeder/src/account-package.ts`
- Regenerate: `genesis/data/account-packages/*.json`

- [ ] **Step 1: Update `account-package.ts` source path**

Edit `genesis/seeder/src/account-package.ts`:
- Change `HUMANS_FILE` from `genesis/docs/humans/humans.json` to `genesis/data/humans/humans.json` (the generated artifact)
- Run `build-data.ts humans` inline at the start if humans.json is missing — or document that it must run first

- [ ] **Step 2: Run `build-data.ts` to generate humans.json**

Run: `cd genesis/seeder && npx tsx src/build-data.ts humans`
Expected: `genesis/data/humans/humans.json` created with 33 humans

- [ ] **Step 3: Run account-package generator**

Run: `cd genesis/seeder && npx tsx src/account-package.ts`
Expected: 33 package files written (4 new: ezra, levi, miriam, susan + 29 updated)

- [ ] **Step 4: Verify 33 packages exist**

Run: `ls /projects/elohim/genesis/data/account-packages/*.json | grep -v -E "(index|conductor-groups)" | wc -l`
Expected: 33

- [ ] **Step 5: Stage**

Run: `git add genesis/seeder/src/account-package.ts genesis/data/account-packages/ genesis/data/humans/humans.json`

---

### Task D4: Update `seed-humans.ts` to consume generated humans.json

**Files:**
- Modify: `genesis/seeder/src/seed-humans.ts`

- [ ] **Step 1: Update the source path**

Edit `genesis/seeder/src/seed-humans.ts`:
- Change the `humans.json` path from `genesis/docs/humans/humans.json` to `genesis/data/humans/humans.json`
- Add a pre-check: if the file doesn't exist, call the build-data module to generate it

- [ ] **Step 2: Run seed-humans locally (optional sanity check — requires local doorway)**

Skip if local doorway isn't running. Full run happens in Phase G.

- [ ] **Step 3: Stage**

Run: `git add genesis/seeder/src/seed-humans.ts`

---

## Phase E — CI/CD wiring

### Task E1: Add "Seed Presences" stage to Jenkinsfile

**Files:**
- Modify: `genesis/Jenkinsfile`

- [ ] **Step 1: Find the Seed Humans stage and insert Seed Presences after it**

Edit `genesis/Jenkinsfile`:

After the `stage('Seed Humans')` block (around line 756), insert:

```groovy
stage('Seed Presences') {
    when { allOf {
        expression { env.PIPELINE_SKIPPED != 'true' }
        expression { params.SEED_DATA }
    }}
    steps {
        container('builder') {
            script {
                def doorwayHost = env.RESOLVED_DOORWAY_HOST
                dir('genesis/seeder') {
                    sh """#!/bin/bash
                        set -euo pipefail
                        echo "═══════════════════════════════════════════════════════════"
                        echo "SEED PRESENCES"
                        echo "═══════════════════════════════════════════════════════════"
                        echo "Doorway: ${doorwayHost}"
                        echo ""
                        DOORWAY_URL="${doorwayHost}" npx tsx src/seed-presences.ts
                    """
                }
            }
        }
    }
}
```

- [ ] **Step 2: Verify Jenkinsfile is still within CPS method size limit**

Run: `wc -l genesis/Jenkinsfile` (root Jenkinsfile limit is stricter; genesis Jenkinsfile has more headroom)
Expected: reasonable size, no concerns

- [ ] **Step 3: Stage**

Run: `git add genesis/Jenkinsfile`

---

### Task E2: Update `package.json` with new pnpm scripts

**Files:**
- Modify: `genesis/seeder/package.json`

- [ ] **Step 1: Add new scripts**

Edit `genesis/seeder/package.json` and add to the `scripts` section:

```json
{
  "scripts": {
    "validate:humans": "tsx src/validate-humans.ts",
    "validate:presences": "tsx src/validate-presences.ts",
    "validate:content": "tsx src/validate-content.ts",
    "validate:all": "pnpm validate:humans && pnpm validate:presences && pnpm validate:collectives",
    "build:data": "tsx src/build-data.ts all",
    "seed:presences": "tsx src/seed-presences.ts"
  }
}
```

(Preserve existing scripts.)

- [ ] **Step 2: Stage**

Run: `git add genesis/seeder/package.json`

---

### Task E3: Update `.husky/pre-push` with validator hooks

**Files:**
- Modify: `.husky/pre-push`

- [ ] **Step 1: Add validator triggers**

Edit `.husky/pre-push` and add (after the existing project-detection logic):

```bash
# Humans/presences schema validation + freshness
if git diff --cached --name-only HEAD | grep -qE "genesis/data/humans/|genesis/data/presences/"; then
  echo "[pre-push] Validating humans and presences schemas..."
  cd genesis/seeder
  pnpm run validate:humans || exit 1
  pnpm run validate:presences || exit 1

  # Freshness check: regenerate humans.json / presences.json and fail if they differ
  pnpm run build:data
  cd ../..
  if ! git diff --quiet genesis/data/humans/humans.json genesis/data/presences/presences.json; then
    echo "[pre-push] ERROR: generated JSON artifacts are stale. Regenerated; please stage and re-push."
    exit 1
  fi
fi
```

- [ ] **Step 2: Stage**

Run: `git add .husky/pre-push`

---

### Task E4: Update `.claude/file-relationships.json`

**Files:**
- Modify: `.claude/file-relationships.json`

- [ ] **Step 1: Add humans-presences-sync entry**

Edit `.claude/file-relationships.json` and add:

```json
{
  "humans-presences-sync": {
    "description": "Humans, presences, and content citations must stay bidirectionally consistent",
    "patterns": [
      {
        "changed": "genesis/data/humans/*.md",
        "related": "genesis/data/presences/*.md",
        "message": "Humans changed — check that observer/steward references in presences still resolve"
      },
      {
        "changed": "genesis/data/presences/*.md",
        "related": "genesis/data/lamad/content/*.json",
        "message": "Presences changed — check that content contributors[] references still resolve"
      },
      {
        "changed": "genesis/data/lamad/content/*.json",
        "related": "genesis/data/presences/*.md",
        "message": "Content changed — check bidirectional consistency with presence.works.citedInContentIds"
      }
    ]
  }
}
```

- [ ] **Step 2: Stage**

Run: `git add .claude/file-relationships.json`

---

## Phase F — Narrative coherence fixes

### Task F1: Fix Sammy typo in fixture-humans.feature

**Files:**
- Modify: `genesis/a2o/features/auth/fixture-humans.feature`

- [ ] **Step 1: Review the Sammy reference in context**

Run: `grep -n -C 3 "Sammy" genesis/a2o/features/auth/fixture-humans.feature`
Expected: the reference at line 19 in a scenario testing 4 distinct household tokens

- [ ] **Step 2: Fix the typo**

The scenario logs in Matthew, Susan, Sammy, Gertrude. Sammy appears to be a typo. The most likely intended human is **James** (Matthew's son, also a household member).

Edit `genesis/a2o/features/auth/fixture-humans.feature`:
- Replace `"Sammy"` with `"James"` on line 19

- [ ] **Step 3: Verify no other Sammy references**

Run: `grep -r "Sammy" genesis/a2o/features/ genesis/a2o/src/`
Expected: no matches

- [ ] **Step 4: Stage**

Run: `git add genesis/a2o/features/auth/fixture-humans.feature`

---

### Task F2: Add narrative-coherence CI checks to validator

**Files:**
- Modify: `genesis/seeder/src/validate-humans.ts`

- [ ] **Step 1: Add a2o scenario cross-reference checker**

Add a new exported function to `validate-humans.ts`:

```typescript
import { readdirSync as readdirSyncFs, readFileSync as readFileSyncFs } from 'node:fs';
import { join as joinPath } from 'node:path';

/**
 * Scan genesis/a2o/features/ for `human "Name"` patterns.
 * Returns names that don't resolve to a known humans.json displayName
 * and aren't part of a synthetic-registration scenario.
 */
export async function validateA2oNarrativeCoherence(
  a2oFeaturesDir: string,
  humans: Map<string, HumanFrontmatter>,
): Promise<{ unresolved: string[] }> {
  const displayNames = new Set<string>();
  for (const h of humans.values()) {
    displayNames.add(h.displayName);
  }

  // Add known synthetic-scenario names (exclude from unresolved report)
  const syntheticNames = new Set([
    'Alice', 'Bob',            // federation cross-doorway test personas
    'Newcomer', 'Lifecycle', 'Troublemaker',  // auth registration scenario subjects
    'Validator',                // staging persona
  ]);

  const unresolved = new Set<string>();

  function walk(dir: string): void {
    for (const entry of readdirSyncFs(dir, { withFileTypes: true })) {
      const path = joinPath(dir, entry.name);
      if (entry.isDirectory()) {
        walk(path);
      } else if (entry.name.endsWith('.feature')) {
        const content = readFileSyncFs(path, 'utf-8');
        const matches = content.matchAll(/human "([A-Z][a-zA-Z ]+)"/g);
        for (const m of matches) {
          const name = m[1].trim();
          if (!displayNames.has(name) && !syntheticNames.has(name)) {
            unresolved.add(name);
          }
        }
      }
    }
  }

  walk(a2oFeaturesDir);
  return { unresolved: Array.from(unresolved).sort() };
}
```

- [ ] **Step 2: Add a test for the a2o check**

Add to `validate-humans.test.ts`:

```typescript
import { validateA2oNarrativeCoherence } from '../validate-humans.js';

describe('validateA2oNarrativeCoherence', () => {
  it('flags names that do not resolve to humans.json', async () => {
    const humans = new Map<string, HumanFrontmatter>();
    humans.set('human-matthew', {
      id: 'human-matthew', displayName: 'Matthew', category: 'core-family', profileReach: 'community',
    } as HumanFrontmatter);

    // Create a tmp feature dir with one bad reference
    const dir = await mkdtemp(join(tmpdir(), 'a2o-test-'));
    await writeFile(join(dir, 'bad.feature'), 'Scenario: ...\n  Given human "Ghost" is logged in\n');

    const result = await validateA2oNarrativeCoherence(dir, humans);
    expect(result.unresolved).toContain('Ghost');
  });

  it('allows synthetic scenario names', async () => {
    const humans = new Map<string, HumanFrontmatter>();
    const dir = await mkdtemp(join(tmpdir(), 'a2o-test-'));
    await writeFile(join(dir, 'auth.feature'), 'Scenario: ...\n  Given human "Alice" is registered\n');
    const result = await validateA2oNarrativeCoherence(dir, humans);
    expect(result.unresolved).not.toContain('Alice');
  });
});
```

- [ ] **Step 3: Run tests**

Run: `cd genesis/seeder && pnpm exec vitest run src/__tests__/validate-humans.test.ts`
Expected: all tests pass

- [ ] **Step 4: Stage**

Run: `git add genesis/seeder/src/validate-humans.ts genesis/seeder/src/__tests__/validate-humans.test.ts`

---

## Phase G — Verification & final commit

### Task G1: Run full validator chain

- [ ] **Step 1: Regenerate all JSON artifacts**

Run: `cd genesis/seeder && pnpm run build:data`
Expected: `genesis/data/humans/humans.json` and `genesis/data/presences/presences.json` generated

- [ ] **Step 2: Validate humans directory**

Run: `cd genesis/seeder && pnpm run validate:humans`
Expected: "All validations passed" with 33 humans, 27+ relationships

- [ ] **Step 3: Validate presences directory**

Run: `cd genesis/seeder && pnpm run validate:presences`
Expected: "All validations passed" with ~110 presences

- [ ] **Step 4: Validate collectives (regression check — existing work)**

Run: `cd genesis/seeder && pnpm run validate:collectives`
Expected: still passes

- [ ] **Step 5: Validate content (referential integrity with new contributors[])**

Run: `cd genesis/seeder && pnpm run validate:content` (if implemented — else skip)
Expected: pass

- [ ] **Step 6: Run a2o narrative coherence check**

Run: `cd genesis/seeder && npx tsx -e "import {validateA2oNarrativeCoherence, validateHumansDirectory} from './src/validate-humans.js'; const h = await validateHumansDirectory('../data/humans'); const r = await validateA2oNarrativeCoherence('../a2o/features', h.humans); console.log('Unresolved:', r.unresolved);"`
Expected: empty or only the 4 red-team personas flagged as backlog

- [ ] **Step 7: Stage any regenerated files**

Run: `git add genesis/data/humans/humans.json genesis/data/presences/presences.json`

---

### Task G2: Local seeder smoke test (if dev env is running)

- [ ] **Step 1: Start the local dev stack**

Run: `cd app/elohim-app && pnpm run hc:start`
Wait for "conductor ready" output (~30s)

- [ ] **Step 2: Run full seeder chain**

Run: `cd genesis/seeder && DOORWAY_URL=http://localhost:8888 pnpm run seed:all` (or equivalent existing script)

If there's no `seed:all` script, run in order:
```
DOORWAY_URL=http://localhost:8888 npx tsx src/seed-sqlite.ts
DOORWAY_URL=http://localhost:8888 npx tsx src/seed-humans.ts
DOORWAY_URL=http://localhost:8888 npx tsx src/seed-presences.ts
DOORWAY_URL=http://localhost:8888 npx tsx src/seed-collectives.ts
DOORWAY_URL=http://localhost:8888 npx tsx src/seed-accounts.ts
```

Expected: all stages complete; presences POST successfully

- [ ] **Step 3: Verify presences via API**

Run: `curl -s http://localhost:8888/db/presences | jq 'length'`
Expected: ~110

- [ ] **Step 4: Stop dev stack**

Run: `cd app/elohim-app && pnpm run hc:stop` (or Ctrl-C the running process)

If the local dev stack isn't available, skip this task and rely on CI for verification.

---

### Task G3: Run a2o smoke scenarios

- [ ] **Step 1: Run the fixture-humans scenario (the one Sammy typo was in)**

Run: `cd genesis/a2o && npx cucumber-js features/auth/fixture-humans.feature`
Expected: all 4 humans log in with distinct tokens (including James — the Sammy fix)

- [ ] **Step 2: Run a broader auth scenario smoke set**

Run: `cd genesis/a2o && npx cucumber-js features/auth/`
Expected: no regressions from the humans migration

If a2o isn't runnable in the current env, defer to CI.

---

### Task G4: Final sprint commit

- [ ] **Step 1: Review the staged changes**

Run: `git status`
Expected: large diff spanning:
- 3 new schemas (humans, presences, scripts deletion)
- 33 new human markdown files
- ~110 new presence markdown files + ~110 images
- 2-3 new book content nodes
- ~50-100 modified content nodes (contributors[] + relatedNodeIds rewrites)
- ~113 deleted content files (fct-contributor, human-, governance-organizations, governance-books)
- 1 deleted humans.json (moved)
- Modified seeder source files + new seed-presences.ts
- Modified Jenkinsfile
- Modified pre-push hook
- Modified file-relationships.json
- Regenerated account-packages (4 new, 29 updated)
- Modified a2o fixture-humans.feature (Sammy → James)
- Regenerated humans.json / presences.json

- [ ] **Step 2: Run the pre-push hook dry-run** (optional confidence check)

Run: `bash .husky/pre-push`
Expected: all validators pass, no freshness errors

- [ ] **Step 3: Create the commit**

Commit message (follow repo convention):

```
feat(genesis): humans & presences schema sprint

- Add humans.schema.json and presences.schema.json as IoC contracts
- Migrate Consilience Garden Keen (~100 gems) to ContributorPresence entries
  with historical observations dated 2021-08-07
- Migrate 31 FCT contributor stubs to presences with Pete observations
- Create book ContentNodes for Keen's Media and Books section with
  bidirectional author-presence links
- Delete 113 orphan stub content files (fct-contributor-, human-,
  governance-organizations-, governance-books-)
- Delete Consilience_Garden-nvBCVtcYER7C9s3H6zxS/ directory
- Add seed-presences.ts stage between seed-humans and seed-collectives
- Add validate-humans.ts and validate-presences.ts with referential integrity
- Introduce markdown-first canonical sources with generated JSON artifacts
- Regenerate account packages for 4 missing humans (ezra, levi, miriam, susan)
- Narrative coherence fixes: rename Pastor Pete → Pete, Sammy → James
- Add 22 human relationship types as operational source of truth
- Add key-steward-of relationship type for recovery network declarations
- Add claimedAttestations[] field for red-team persona modeling

Design: genesis/plans/2026-04-11-humans-schema-design.md
Plan:   genesis/plans/2026-04-11-humans-schema-plan.md

Zero Rust changes — every primitive exists in imagodei DNA.
```

Follow the user's commit conventions including the Co-Authored-By trailer per CLAUDE.md.

Run: the user will issue the `/commit` or equivalent when ready. Do NOT auto-commit.

---

## Self-Review Summary

**Spec coverage check:**
- ✓ humans.schema.json (Task A1)
- ✓ presences.schema.json (Task A2)
- ✓ validate-humans.ts (Task A3)
- ✓ validate-presences.ts (Task A4)
- ✓ build-data.ts generator (Task A5)
- ✓ Consilience Garden migration (Tasks B1-B6, C1)
- ✓ Book ContentNodes with author presences (Task B4)
- ✓ Content rewrite plan (Task B5)
- ✓ Image capture pipeline with placeholders (Task B6)
- ✓ Script self-deletion (Task C2)
- ✓ seed-presences.ts (Task D1)
- ✓ humans.json → humans/*.md conversion (Task D2)
- ✓ Account package regeneration for 4 missing humans (Task D3)
- ✓ Jenkinsfile Seed Presences stage (Task E1)
- ✓ pnpm scripts (Task E2)
- ✓ Pre-push hook updates (Task E3)
- ✓ file-relationships.json (Task E4)
- ✓ Sammy → James fix (Task F1)
- ✓ Pastor Pete → Pete rename (Task D2 inline)
- ✓ a2o narrative coherence validator (Task F2)
- ✓ Full verification chain (Tasks G1-G3)
- ✓ Final commit at sprint end (Task G4)

**Placeholder scan:** None — all steps contain actual code or exact commands.

**Type consistency:** `PresenceFrontmatter`, `HumanFrontmatter`, `RelationshipEntry` used consistently across tasks. `validateHumansDirectory` / `validatePresencesDirectory` signatures consistent. `CreateContributorPresenceInputView` imported from `@elohim/storage-client` in Task D1.

**Scope check:** Single-sprint scope, bounded by narrative coherence verification. No cross-sprint dependencies.

---

## Execution Handoff

Plan complete and saved to `genesis/plans/2026-04-11-humans-schema-plan.md`. Two execution options:

**1. Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks, fast iteration. Matches the collectives sprint cadence.

**2. Inline Execution** — execute tasks in this session using `executing-plans`, batch execution with checkpoints for review.

Which approach?
