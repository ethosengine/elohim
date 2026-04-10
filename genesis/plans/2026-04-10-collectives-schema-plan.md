# Collectives Schema Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create `collectives.schema.json` that enforces the holonic constitutional architecture, enrich seed data with intimate-scale collectives and relationship graph, update the seeder to validate before POSTing, and ensure a2o scenario coherence.

**Architecture:** JSON Schema + hand-rolled TypeScript validator (matching existing seeder pattern in `schema-validation.ts`). New fields (`governanceModel`, `domain`, `place`, `coupling`) flow through `metadata_json` — zero Rust changes. Seed data gains 5 new intimate-scale collectives, a `relationships[]` array, corrected reach values, and `constitutionalParentId` hierarchy. Account packages updated to reference renamed collective IDs.

**Tech Stack:** JSON Schema (draft 2020-12), TypeScript/Vitest (seeder validation), existing `CreateCollectiveInputView` from `@elohim/storage-client`

**Design spec:** `genesis/plans/2026-04-10-collectives-schema-design.md`

---

### Task 1: Create `collectives.schema.json`

**Files:**
- Create: `genesis/data/collectives/collectives.schema.json`

- [ ] **Step 1: Write the schema file**

This is the IoC contract. It enforces enum values from protocol sources and structural rules for the holonic lattice.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "elohim:protocol:collectives",
  "title": "Collectives Seed Data",
  "description": "Holonic collective definitions with constitutional relationships. Source of truth for governance layer, reach, and relationship type enums. Collectives are protocol-core entities (Category A — DHT-notarized in Mishpat DNA). Relationships are derived (Category A2 — Links on Mishpat DNA). See design: genesis/plans/2026-04-10-collectives-schema-design.md",
  "type": "object",
  "required": ["version", "collectives"],
  "properties": {
    "version": {
      "type": "string",
      "const": "2.0.0"
    },
    "description": { "type": "string" },
    "collectives": {
      "type": "array",
      "items": { "$ref": "#/$defs/Collective" },
      "minItems": 1
    },
    "relationships": {
      "type": "array",
      "items": { "$ref": "#/$defs/CollectiveRelationship" }
    },
    "reachConstraints": {
      "description": "Dunbar-aware fuzzy ranges per reach level — protocol configuration, not stored data (Category C operational)",
      "$ref": "#/$defs/ReachConstraints"
    },
    "relationshipTypes": {
      "description": "Protocol-core relationship type vocabulary with provenance flow semantics",
      "type": "object",
      "additionalProperties": { "$ref": "#/$defs/RelationshipTypeDefinition" }
    }
  },
  "additionalProperties": false,
  "$defs": {
    "Collective": {
      "type": "object",
      "required": ["id", "name", "governanceLayer", "reach"],
      "properties": {
        "id": {
          "type": "string",
          "pattern": "^[a-z0-9][a-z0-9-]*[a-z0-9]$",
          "description": "Unique slug identifier. Naming convention: {type}-{identifier} where type is couple-, family-, bible-study-, neighborhood-, community-, org-"
        },
        "name": { "type": "string", "minLength": 1 },
        "governanceLayer": {
          "type": "string",
          "enum": ["family", "neighborhood", "faith", "education", "interest", "geographic", "workplace", "economic", "community"],
          "description": "Source: Rust governance_layers::ALL in elohim-storage/src/db/models.rs"
        },
        "reach": {
          "type": "string",
          "enum": ["private", "self", "intimate", "trusted", "familiar", "community", "public", "commons"],
          "description": "Source: Protocol ReachLevels in elohim/sdk/src/types.ts. See reachConstraints for Dunbar-aware ranges."
        },
        "constitutionalParentId": {
          "type": ["string", "null"],
          "description": "Primary constitutional appeal chain. Must reference another collective's id. Validated by companion script (JSON Schema cannot enforce cross-references)."
        },
        "description": { "type": ["string", "null"] },
        "governanceModel": {
          "type": ["string", "null"],
          "enum": ["consent", "steward-consent", "community-vote", "constitutional", "consensus", null],
          "description": "Source: qahal collective-metadata.schema.json. Flows through metadata_json in storage."
        },
        "domain": {
          "type": ["string", "null"],
          "enum": ["household", "curriculum", "worship", "infrastructure", "trade", "land-use", "economy", "defense", null],
          "description": "What this collective governs. Domain-bounded authority. Flows through metadata_json."
        },
        "place": {
          "type": ["string", "null"],
          "description": "H3 cell ID for geographic grounding. Null for non-geographic collectives. Flows through metadata_json."
        },
        "coupling": {
          "type": ["object", "null"],
          "description": "Three-leg EPR coupling declaration. Makes this collective a protocol entity, not just a group.",
          "properties": {
            "lamad": { "type": "string", "description": "Knowledge this collective stewards" },
            "shefa": { "type": "string", "description": "Economic flows through this collective" },
            "qahal": { "type": "string", "description": "Governance model for this collective" }
          },
          "additionalProperties": false
        }
      },
      "additionalProperties": false
    },
    "CollectiveRelationship": {
      "type": "object",
      "required": ["type", "from", "to"],
      "properties": {
        "type": {
          "type": "string",
          "enum": ["contains", "participates-in", "delegates-to", "peers-with", "succeeds"],
          "description": "Protocol-core relationship type. See relationshipTypes for provenance semantics."
        },
        "from": {
          "type": "string",
          "description": "Source collective id. Validated by companion script."
        },
        "to": {
          "type": "string",
          "description": "Target collective id. Validated by companion script."
        },
        "description": { "type": ["string", "null"] },
        "domainOverlap": {
          "type": ["string", "null"],
          "description": "Required for peers-with relationships. Declares the shared governance concern."
        }
      },
      "additionalProperties": false,
      "if": { "properties": { "type": { "const": "peers-with" } } },
      "then": { "required": ["type", "from", "to", "domainOverlap"] }
    },
    "ReachConstraints": {
      "type": "object",
      "description": "Fuzzy Dunbar-aware ranges per reach level. suggestedRange is [min, typical_max]. cautionAbove triggers elohim observation signals. directParticipants:false means governance is through sub-collective delegates only.",
      "properties": {
        "private":   { "$ref": "#/$defs/ReachConstraint" },
        "self":      { "$ref": "#/$defs/ReachConstraint" },
        "intimate":  { "$ref": "#/$defs/ReachConstraint" },
        "trusted":   { "$ref": "#/$defs/ReachConstraint" },
        "familiar":  { "$ref": "#/$defs/ReachConstraint" },
        "community": { "$ref": "#/$defs/ReachConstraint" },
        "public":    { "$ref": "#/$defs/DelegateOnlyConstraint" },
        "commons":   { "$ref": "#/$defs/DelegateOnlyConstraint" }
      },
      "additionalProperties": false
    },
    "ReachConstraint": {
      "type": "object",
      "properties": {
        "suggestedRange": {
          "type": "array",
          "items": { "type": "integer" },
          "minItems": 2,
          "maxItems": 2
        },
        "cautionAbove": { "type": "integer" },
        "description": { "type": "string" }
      },
      "required": ["suggestedRange", "cautionAbove"]
    },
    "DelegateOnlyConstraint": {
      "type": "object",
      "properties": {
        "directParticipants": { "type": "boolean", "const": false },
        "description": { "type": "string" }
      },
      "required": ["directParticipants"]
    },
    "RelationshipTypeDefinition": {
      "type": "object",
      "properties": {
        "description": { "type": "string" },
        "provenanceFlow": {
          "type": "string",
          "enum": ["bidirectional-filtered", "directional-read", "directional-governance", "bidirectional-scoped", "directional-historical"]
        },
        "agentSemantics": { "type": "string" },
        "humanSemantics": { "type": "string" },
        "constraint": { "type": ["string", "null"] }
      },
      "required": ["description", "provenanceFlow", "agentSemantics", "humanSemantics"]
    }
  }
}
```

- [ ] **Step 2: Validate the schema is valid JSON Schema**

Run: `cd /projects/elohim && python3 -c "import json; json.load(open('genesis/data/collectives/collectives.schema.json'))"`
Expected: No output (valid JSON)

- [ ] **Step 3: Commit**

```bash
git add genesis/data/collectives/collectives.schema.json
git commit -m "feat(qahal): add collectives.schema.json — holonic constitutional IoC contract

Enforces governance layer, reach, relationship type enums from protocol sources.
Declares reach constraints (Dunbar-aware fuzzy ranges), EPR coupling, and
relationship types with provenance flow semantics for agent contexts."
```

---

### Task 2: Enrich `collectives.json` with holonic seed data

**Files:**
- Modify: `genesis/data/collectives/collectives.json`

This is the largest task — rewrite the seed data to match the schema. Add 5 new intimate-scale collectives, wire `constitutionalParentId` hierarchy, correct reach values, add relationship graph, and include the vocabulary declarations.

- [ ] **Step 1: Rewrite collectives.json**

The full file has the following structure:
- `version: "2.0.0"` (breaking change — new structure)
- `reachConstraints` — Dunbar-aware fuzzy ranges
- `relationshipTypes` — protocol vocabulary with provenance semantics
- `collectives[]` — all 47 collectives (42 existing + 5 new), with corrected reach, added `constitutionalParentId`, `governanceModel`, `domain`, `coupling`
- `relationships[]` — the holonic lattice

Key changes to existing collectives:

| Old ID | New ID | Reason |
|--------|--------|--------|
| `household-dowell` | `family-dowell` | Family-layer, not generic household |
| `household-eden` | `family-eden` | Same |
| `household-valley-economy` | `neighborhood-valley-economy` | Was neighborhood layer |
| `household-extended` | `neighborhood-extended` | Was neighborhood layer |

Reach corrections for ALL existing collectives:

| Old reach | New reach | Affected IDs |
|-----------|-----------|-------------|
| `private` | `trusted` | `family-dowell`, `family-eden` (families, not individuals) |
| `local` | `familiar` | `neighborhood-valley-economy`, `neighborhood-extended` |
| `municipal` | `community` | all `org-*` with old `municipal` reach |

New collectives:

```json
{
  "id": "couple-adam-eve",
  "name": "Adam & Eve",
  "governanceLayer": "family",
  "reach": "intimate",
  "constitutionalParentId": "family-eden",
  "description": "Genesis couple — the original partnership",
  "governanceModel": "consent",
  "domain": "household",
  "place": null,
  "coupling": {
    "lamad": "household-wisdom",
    "shefa": "household-economy",
    "qahal": "consent"
  }
},
{
  "id": "couple-matthew-jessica",
  "name": "Matthew & Jessica",
  "governanceLayer": "family",
  "reach": "intimate",
  "constitutionalParentId": "family-dowell",
  "description": "Founding couple — co-stewards of household decisions",
  "governanceModel": "consent",
  "domain": "household",
  "place": null,
  "coupling": {
    "lamad": "household-wisdom",
    "shefa": "household-economy",
    "qahal": "consent"
  }
},
{
  "id": "family-dowell",
  "name": "Dowell Family",
  "governanceLayer": "family",
  "reach": "trusted",
  "constitutionalParentId": "neighborhood-valley",
  "description": "Nuclear family — Matthew, Jessica, Timothy. Co-stewards of household, education, and faith formation.",
  "governanceModel": "steward-consent",
  "domain": "family",
  "place": null,
  "coupling": {
    "lamad": "household-wisdom",
    "shefa": "household-economy",
    "qahal": "steward-consent"
  }
},
{
  "id": "bible-study-valley",
  "name": "Valley Bible Study",
  "governanceLayer": "faith",
  "reach": "familiar",
  "constitutionalParentId": "community-local-church",
  "description": "Weekly bible study — Matthew, Jessica, Timothy, Pete",
  "governanceModel": "consent",
  "domain": "worship",
  "place": null,
  "coupling": {
    "lamad": "scriptural-study",
    "shefa": "shared-materials",
    "qahal": "consent"
  }
},
{
  "id": "neighborhood-valley",
  "name": "Valley Neighborhood",
  "governanceLayer": "geographic",
  "reach": "familiar",
  "constitutionalParentId": "community-neighborhood-association",
  "description": "Immediate neighborhood — Matthew, Jessica, Timothy, Nancy",
  "governanceModel": "community-vote",
  "domain": "land-use",
  "place": null,
  "coupling": {
    "lamad": "local-knowledge",
    "shefa": "shared-infrastructure",
    "qahal": "community-vote"
  }
}
```

Relationships array (the holonic lattice):

```json
"relationships": [
  { "type": "contains", "from": "family-dowell", "to": "couple-matthew-jessica", "description": "Nuclear family contains founding couple" },
  { "type": "contains", "from": "family-eden", "to": "couple-adam-eve", "description": "Eden family contains genesis couple" },
  { "type": "succeeds", "from": "couple-adam-eve", "to": "couple-matthew-jessica", "description": "Generational succession — genesis couple to founding couple" },
  { "type": "participates-in", "from": "family-dowell", "to": "community-local-church", "description": "Dowell family are church members" },
  { "type": "participates-in", "from": "family-dowell", "to": "neighborhood-valley", "description": "Dowell family in the neighborhood" },
  { "type": "participates-in", "from": "family-dowell", "to": "community-homeschool-coop", "description": "Dowell family homeschools through co-op" },
  { "type": "participates-in", "from": "family-eden", "to": "community-local-church", "description": "Eden family are church members" },
  { "type": "participates-in", "from": "family-eden", "to": "neighborhood-valley", "description": "Eden family in the neighborhood" },
  { "type": "contains", "from": "community-local-church", "to": "bible-study-valley", "description": "Church contains bible study small group" },
  { "type": "contains", "from": "community-neighborhood-association", "to": "neighborhood-valley", "description": "Association contains the neighborhood" },
  { "type": "delegates-to", "from": "bible-study-valley", "to": "community-local-church", "description": "Bible study sends representative to church governance" },
  { "type": "delegates-to", "from": "neighborhood-valley", "to": "community-neighborhood-association", "description": "Neighborhood sends delegate to association" },
  { "type": "peers-with", "from": "community-local-church", "to": "community-homeschool-coop", "domainOverlap": "child-development", "description": "Church and co-op coordinate on children's holistic development" },
  { "type": "peers-with", "from": "community-local-church", "to": "community-pastoral-network", "domainOverlap": "pastoral-care", "description": "Church and pastoral network coordinate on care" },
  { "type": "participates-in", "from": "org-ethosengine", "to": "community-holochain-devs", "description": "EthosEngine participates in Holochain dev community" },
  { "type": "participates-in", "from": "org-franks-farm", "to": "community-regen-ag-network", "description": "Frank's Farm in the regenerative agriculture network" },
  { "type": "participates-in", "from": "org-franks-farm", "to": "community-farmers-market", "description": "Frank's Farm sells at the market" },
  { "type": "peers-with", "from": "community-farmers-market", "to": "community-local-business", "domainOverlap": "local-commerce", "description": "Market and business alliance coordinate on local economy" },
  { "type": "participates-in", "from": "org-consolidated-mining", "to": "community-mining-town", "description": "Mining corp operates in mining town" },
  { "type": "participates-in", "from": "org-miners-union-local-347", "to": "community-mining-town", "description": "Miners union represents workers in mining town" },
  { "type": "peers-with", "from": "org-consolidated-mining", "to": "org-miners-union-local-347", "domainOverlap": "labor-conditions", "description": "Management and union negotiate labor conditions" },
  { "type": "participates-in", "from": "org-valley-power-electric", "to": "community-utility-workers", "description": "Power company workers in utility network" },
  { "type": "participates-in", "from": "org-ibew-local-123", "to": "community-utility-workers", "description": "IBEW represents electrical workers" },
  { "type": "participates-in", "from": "community-immigrant-support", "to": "community-esl-program", "description": "Immigrant support feeds into ESL program" },
  { "type": "participates-in", "from": "community-refugee-resettlement", "to": "community-immigrant-support", "description": "Refugee resettlement feeds into immigrant support" }
]
```

- [ ] **Step 2: Validate JSON is well-formed**

Run: `cd /projects/elohim && python3 -c "import json; d=json.load(open('genesis/data/collectives/collectives.json')); print(f'Collectives: {len(d[\"collectives\"])}, Relationships: {len(d.get(\"relationships\", []))}')"`
Expected: `Collectives: 47, Relationships: 25`

- [ ] **Step 3: Commit**

```bash
git add genesis/data/collectives/collectives.json
git commit -m "feat(qahal): enrich collectives seed data with holonic constitutional structure

- Add 5 intimate-scale collectives: couple-adam-eve, couple-matthew-jessica,
  family-dowell, bible-study-valley, neighborhood-valley
- Rename household-* to family-*/neighborhood-* for governance layer consistency
- Correct reach values: local→familiar, municipal→community, private→trusted (families)
- Wire constitutionalParentId hierarchy across all collectives
- Add relationships[] array: contains, participates-in, delegates-to, peers-with, succeeds
- Add governanceModel, domain, coupling declarations per collective
- Add reachConstraints (Dunbar-aware fuzzy ranges) and relationshipTypes vocabulary
- Version bump to 2.0.0 (breaking: new structure)"
```

---

### Task 3: Update account packages for renamed collective IDs

**Files:**
- Modify: `genesis/data/account-packages/*.json` (29 files with `collectiveId` references)
- Modify: `genesis/seeder/src/account-package.ts` (if it has hardcoded IDs)

The 4 renamed collectives (`household-dowell` → `family-dowell`, etc.) are referenced in account packages via `collectiveId`. All references must update.

- [ ] **Step 1: Bulk rename collective IDs in account packages**

Use sed for the mechanical replacement across all account package files:

```bash
cd /projects/elohim/genesis/data/account-packages

# Verify current references
grep -l "household-dowell\|household-eden\|household-valley-economy\|household-extended" *.json | wc -l
# Expected: 29

# Apply renames
sed -i 's/"household-dowell"/"family-dowell"/g' *.json
sed -i 's/"household-eden"/"family-eden"/g' *.json
sed -i 's/"household-valley-economy"/"neighborhood-valley-economy"/g' *.json
sed -i 's/"household-extended"/"neighborhood-extended"/g' *.json

# Verify no old references remain
grep -r "household-dowell\|household-eden\|household-valley-economy\|household-extended" *.json | wc -l
# Expected: 0
```

- [ ] **Step 2: Add new collective memberships for genesis humans**

Update account packages for the humans who participate in the new collectives:

**matthew-manager.json** — add entries for `couple-matthew-jessica`, `bible-study-valley`, `neighborhood-valley`:
```json
{
  "collectiveId": "couple-matthew-jessica",
  "roleContext": "partner",
  "intimacyLevel": "intimate"
},
{
  "collectiveId": "bible-study-valley",
  "roleContext": null,
  "intimacyLevel": "connection"
},
{
  "collectiveId": "neighborhood-valley",
  "roleContext": null,
  "intimacyLevel": "connection"
}
```

**jessica-spouse.json** — add same three entries (couple, bible study, neighborhood)

**timothy-tutor.json** — add entries for `bible-study-valley`, `neighborhood-valley` (not the couple)

**pete-pastor.json** — add entry for `bible-study-valley`

**nancy-neighbor.json** — add entry for `neighborhood-valley`

**adam-firstman.json** — add entry for `couple-adam-eve`

**eve-firstwoman.json** — add entry for `couple-adam-eve`

- [ ] **Step 3: Verify all account packages are valid JSON**

Run: `cd /projects/elohim && for f in genesis/data/account-packages/*.json; do python3 -c "import json; json.load(open('$f'))" 2>&1 && echo "OK: $f" || echo "FAIL: $f"; done | grep FAIL`
Expected: No output (all valid)

- [ ] **Step 4: Commit**

```bash
git add genesis/data/account-packages/
git commit -m "refactor(qahal): update account packages for renamed collective IDs

Rename household-dowell→family-dowell, household-eden→family-eden,
household-valley-economy→neighborhood-valley-economy, household-extended→neighborhood-extended.
Add new collective memberships: couple-adam-eve, couple-matthew-jessica,
bible-study-valley, neighborhood-valley for genesis humans."
```

---

### Task 4: Write collectives validator

**Files:**
- Create: `genesis/seeder/src/validate-collectives.ts`
- Create: `genesis/seeder/src/__tests__/validate-collectives.test.ts`

Follows the existing pattern in `schema-validation.ts` — hand-rolled TypeScript validators checking against constants, not Ajv/Zod.

- [ ] **Step 1: Write the failing test**

```typescript
// genesis/seeder/src/__tests__/validate-collectives.test.ts
import { describe, it, expect } from 'vitest';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  validateCollectivesFile,
  validateReferentialIntegrity,
  type CollectiveValidationResult,
} from '../validate-collectives.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const COLLECTIVES_FILE = resolve(__dirname, '../../../data/collectives/collectives.json');

describe('validateCollectivesFile()', () => {
  it('should validate the actual collectives.json without errors', () => {
    const result = validateCollectivesFile(COLLECTIVES_FILE);
    if (result.errors.length > 0) {
      console.error('Validation errors:', result.errors);
    }
    expect(result.errors).toEqual([]);
    expect(result.collectiveCount).toBeGreaterThanOrEqual(47);
  });

  it('should validate all reach values are protocol-canonical', () => {
    const result = validateCollectivesFile(COLLECTIVES_FILE);
    expect(result.errors.filter(e => e.includes('reach'))).toEqual([]);
  });

  it('should validate all governanceLayer values are canonical', () => {
    const result = validateCollectivesFile(COLLECTIVES_FILE);
    expect(result.errors.filter(e => e.includes('governanceLayer'))).toEqual([]);
  });
});

describe('validateReferentialIntegrity()', () => {
  it('should have no dangling constitutionalParentId references', () => {
    const result = validateCollectivesFile(COLLECTIVES_FILE);
    const integrityErrors = validateReferentialIntegrity(result);
    const parentErrors = integrityErrors.filter(e => e.includes('constitutionalParentId'));
    if (parentErrors.length > 0) {
      console.error('Dangling parent refs:', parentErrors);
    }
    expect(parentErrors).toEqual([]);
  });

  it('should have no dangling relationship from/to references', () => {
    const result = validateCollectivesFile(COLLECTIVES_FILE);
    const integrityErrors = validateReferentialIntegrity(result);
    const relErrors = integrityErrors.filter(e => e.includes('relationship'));
    if (relErrors.length > 0) {
      console.error('Dangling relationship refs:', relErrors);
    }
    expect(relErrors).toEqual([]);
  });

  it('should have no circular constitutionalParentId chains', () => {
    const result = validateCollectivesFile(COLLECTIVES_FILE);
    const integrityErrors = validateReferentialIntegrity(result);
    const circularErrors = integrityErrors.filter(e => e.includes('circular'));
    expect(circularErrors).toEqual([]);
  });

  it('should require domainOverlap on peers-with relationships', () => {
    const result = validateCollectivesFile(COLLECTIVES_FILE);
    const integrityErrors = validateReferentialIntegrity(result);
    const peerErrors = integrityErrors.filter(e => e.includes('domainOverlap'));
    expect(peerErrors).toEqual([]);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /projects/elohim/genesis/seeder && pnpm exec vitest run src/__tests__/validate-collectives.test.ts`
Expected: FAIL — module `../validate-collectives.js` not found

- [ ] **Step 3: Write the validator**

```typescript
// genesis/seeder/src/validate-collectives.ts
/**
 * Collectives Seed Data Validator
 *
 * Validates genesis/data/collectives/collectives.json against the
 * collectives.schema.json IoC contract. Hand-rolled to match the
 * existing seeder validation pattern (no Ajv/Zod).
 *
 * Two levels of validation:
 * 1. Per-collective field validation (enums, required fields, types)
 * 2. Cross-collective referential integrity (parent refs, relationship refs, cycles)
 */

import { readFileSync } from 'node:fs';
import { REACH_LEVELS } from './validation-constants.js';

// =============================================================================
// Constants — sources of truth
// =============================================================================

/** Source: Rust governance_layers::ALL in elohim-storage/src/db/models.rs */
const GOVERNANCE_LAYERS = [
  'family', 'neighborhood', 'faith', 'education', 'interest',
  'geographic', 'workplace', 'economic', 'community',
] as const;

/** Source: qahal collective-metadata.schema.json */
const GOVERNANCE_MODELS = [
  'consent', 'steward-consent', 'community-vote', 'constitutional', 'consensus',
] as const;

/** Source: collectives.schema.json design spec */
const DOMAINS = [
  'household', 'curriculum', 'worship', 'infrastructure',
  'trade', 'land-use', 'economy', 'defense',
] as const;

/** Source: collectives.schema.json design spec */
const RELATIONSHIP_TYPES = [
  'contains', 'participates-in', 'delegates-to', 'peers-with', 'succeeds',
] as const;

const SLUG_PATTERN = /^[a-z0-9][a-z0-9-]*[a-z0-9]$/;

// =============================================================================
// Types
// =============================================================================

interface CollectiveEntry {
  id: string;
  name: string;
  governanceLayer: string;
  reach: string;
  constitutionalParentId?: string | null;
  description?: string | null;
  governanceModel?: string | null;
  domain?: string | null;
  place?: string | null;
  coupling?: {
    lamad?: string;
    shefa?: string;
    qahal?: string;
  } | null;
}

interface RelationshipEntry {
  type: string;
  from: string;
  to: string;
  description?: string | null;
  domainOverlap?: string | null;
}

interface CollectivesData {
  version: string;
  collectives: CollectiveEntry[];
  relationships?: RelationshipEntry[];
}

export interface CollectiveValidationResult {
  errors: string[];
  warnings: string[];
  collectiveCount: number;
  relationshipCount: number;
  collectiveIds: Set<string>;
  data: CollectivesData | null;
}

// =============================================================================
// Validation
// =============================================================================

function validateEnum(
  value: string | undefined | null,
  field: string,
  allowed: readonly string[],
  entityId: string,
  required: boolean,
): string | null {
  if (value == null) {
    return required ? `${entityId}: ${field} is required` : null;
  }
  if (!allowed.includes(value)) {
    return `${entityId}: ${field} "${value}" is not valid. Allowed: ${allowed.join(', ')}`;
  }
  return null;
}

export function validateCollectivesFile(filePath: string): CollectiveValidationResult {
  const result: CollectiveValidationResult = {
    errors: [],
    warnings: [],
    collectiveCount: 0,
    relationshipCount: 0,
    collectiveIds: new Set(),
    data: null,
  };

  let data: CollectivesData;
  try {
    data = JSON.parse(readFileSync(filePath, 'utf-8'));
    result.data = data;
  } catch (e) {
    result.errors.push(`Failed to parse JSON: ${e instanceof Error ? e.message : String(e)}`);
    return result;
  }

  if (!data.version) {
    result.errors.push('Missing required field: version');
  }

  if (!Array.isArray(data.collectives)) {
    result.errors.push('Missing or invalid field: collectives (must be array)');
    return result;
  }

  result.collectiveCount = data.collectives.length;
  result.relationshipCount = data.relationships?.length ?? 0;

  // Validate each collective
  for (const coll of data.collectives) {
    const id = coll.id ?? '(missing id)';

    // Required fields
    if (!coll.id) {
      result.errors.push(`${id}: missing required field 'id'`);
      continue;
    }
    if (!SLUG_PATTERN.test(coll.id)) {
      result.errors.push(`${id}: id must match slug pattern [a-z0-9-]`);
    }
    if (result.collectiveIds.has(coll.id)) {
      result.errors.push(`${id}: duplicate id`);
    }
    result.collectiveIds.add(coll.id);

    if (!coll.name) {
      result.errors.push(`${id}: missing required field 'name'`);
    }

    // Enum validations
    const layerErr = validateEnum(coll.governanceLayer, 'governanceLayer', GOVERNANCE_LAYERS as unknown as string[], id, true);
    if (layerErr) result.errors.push(layerErr);

    const reachErr = validateEnum(coll.reach, 'reach', REACH_LEVELS as unknown as string[], id, true);
    if (reachErr) result.errors.push(reachErr);

    const modelErr = validateEnum(coll.governanceModel, 'governanceModel', GOVERNANCE_MODELS as unknown as string[], id, false);
    if (modelErr) result.errors.push(modelErr);

    const domainErr = validateEnum(coll.domain, 'domain', DOMAINS as unknown as string[], id, false);
    if (domainErr) result.errors.push(domainErr);

    // Coupling validation
    if (coll.coupling != null && typeof coll.coupling === 'object') {
      const validKeys = new Set(['lamad', 'shefa', 'qahal']);
      for (const key of Object.keys(coll.coupling)) {
        if (!validKeys.has(key)) {
          result.errors.push(`${id}: coupling has unknown key '${key}'. Allowed: lamad, shefa, qahal`);
        }
      }
    }
  }

  // Validate relationships
  if (data.relationships) {
    for (let i = 0; i < data.relationships.length; i++) {
      const rel = data.relationships[i];
      const label = `relationship[${i}] (${rel.from} → ${rel.to})`;

      const typeErr = validateEnum(rel.type, 'type', RELATIONSHIP_TYPES as unknown as string[], label, true);
      if (typeErr) result.errors.push(typeErr);

      if (!rel.from) result.errors.push(`${label}: missing required field 'from'`);
      if (!rel.to) result.errors.push(`${label}: missing required field 'to'`);

      // peers-with requires domainOverlap
      if (rel.type === 'peers-with' && !rel.domainOverlap) {
        result.errors.push(`${label}: peers-with relationships require 'domainOverlap'`);
      }
    }
  }

  return result;
}

export function validateReferentialIntegrity(
  result: CollectiveValidationResult,
): string[] {
  const errors: string[] = [];
  const ids = result.collectiveIds;
  const data = result.data;
  if (!data) return errors;

  // Check constitutionalParentId references
  for (const coll of data.collectives) {
    if (coll.constitutionalParentId && !ids.has(coll.constitutionalParentId)) {
      errors.push(
        `${coll.id}: constitutionalParentId "${coll.constitutionalParentId}" does not reference a known collective`,
      );
    }
  }

  // Check relationship from/to references
  if (data.relationships) {
    for (let i = 0; i < data.relationships.length; i++) {
      const rel = data.relationships[i];
      if (rel.from && !ids.has(rel.from)) {
        errors.push(`relationship[${i}]: 'from' "${rel.from}" does not reference a known collective`);
      }
      if (rel.to && !ids.has(rel.to)) {
        errors.push(`relationship[${i}]: 'to' "${rel.to}" does not reference a known collective`);
      }
    }
  }

  // Check for circular constitutionalParentId chains
  const parentMap = new Map<string, string>();
  for (const coll of data.collectives) {
    if (coll.constitutionalParentId) {
      parentMap.set(coll.id, coll.constitutionalParentId);
    }
  }

  for (const startId of parentMap.keys()) {
    const visited = new Set<string>();
    let current: string | undefined = startId;
    while (current && parentMap.has(current)) {
      if (visited.has(current)) {
        errors.push(`circular constitutionalParentId chain detected involving: ${[...visited].join(' → ')} → ${current}`);
        break;
      }
      visited.add(current);
      current = parentMap.get(current);
    }
  }

  return errors;
}

// =============================================================================
// CLI
// =============================================================================

if (process.argv[1]?.endsWith('validate-collectives.ts') || process.argv[1]?.endsWith('validate-collectives.js')) {
  const filePath = process.argv[2] ?? new URL('../../data/collectives/collectives.json', import.meta.url).pathname;

  console.log('=== Validate Collectives ===\n');
  console.log(`File: ${filePath}\n`);

  const result = validateCollectivesFile(filePath);
  const integrityErrors = validateReferentialIntegrity(result);

  console.log(`Collectives:    ${result.collectiveCount}`);
  console.log(`Relationships:  ${result.relationshipCount}`);
  console.log(`Errors:         ${result.errors.length}`);
  console.log(`Integrity:      ${integrityErrors.length}`);
  console.log(`Warnings:       ${result.warnings.length}\n`);

  if (result.errors.length > 0) {
    console.error('ERRORS:');
    for (const e of result.errors) console.error(`  ✗ ${e}`);
  }

  if (integrityErrors.length > 0) {
    console.error('\nREFERENTIAL INTEGRITY:');
    for (const e of integrityErrors) console.error(`  ✗ ${e}`);
  }

  if (result.warnings.length > 0) {
    console.log('\nWARNINGS:');
    for (const w of result.warnings) console.log(`  ⚠ ${w}`);
  }

  if (result.errors.length === 0 && integrityErrors.length === 0) {
    console.log('✓ All validations passed');
  }

  process.exit(result.errors.length + integrityErrors.length > 0 ? 1 : 0);
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /projects/elohim/genesis/seeder && pnpm exec vitest run src/__tests__/validate-collectives.test.ts`
Expected: All 6 tests PASS

- [ ] **Step 5: Commit**

```bash
git add genesis/seeder/src/validate-collectives.ts genesis/seeder/src/__tests__/validate-collectives.test.ts
git commit -m "feat(qahal): add collectives seed data validator

Hand-rolled TypeScript validator matching existing seeder pattern.
Validates governance layer, reach, governance model enums against
protocol sources. Checks referential integrity: constitutionalParentId,
relationship from/to, circular chains, peers-with domainOverlap."
```

---

### Task 5: Update seeder to validate and map metadata before POSTing

**Files:**
- Modify: `genesis/seeder/src/seed-collectives.ts`

The seeder needs three changes:
1. Run validation before POSTing (fail fast on schema errors)
2. Map new fields (`governanceModel`, `domain`, `place`, `coupling`) into the `metadata` bag
3. Topological sort on `constitutionalParentId` so parents are created before children

- [ ] **Step 1: Update the seeder**

```typescript
// genesis/seeder/src/seed-collectives.ts
/**
 * Seed Collectives — create collective definitions via doorway /db/collectives.
 *
 * Reads collective definitions from genesis/data/collectives/collectives.json,
 * validates against collectives.schema.json IoC contract, maps extended fields
 * into metadata, and POSTs each to the storage API in dependency order.
 *
 * Must run BEFORE seed-accounts.ts so that collectives exist before participations.
 *
 * Usage:
 *   npx tsx src/seed-collectives.ts                             # Seed all collectives
 *   npx tsx src/seed-collectives.ts --dry-run                   # Preview without seeding
 *   npx tsx src/seed-collectives.ts --doorway-url http://...    # Override doorway URL
 *   npx tsx src/seed-collectives.ts --validate-only             # Validate without seeding
 */

import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import type { CreateCollectiveInputView } from '@elohim/storage-client';
import {
  validateCollectivesFile,
  validateReferentialIntegrity,
} from './validate-collectives.js';

// =============================================================================
// Types
// =============================================================================

interface CollectiveEntry {
  id: string;
  name: string;
  governanceLayer: string;
  reach: string;
  constitutionalParentId?: string | null;
  description?: string | null;
  governanceModel?: string | null;
  domain?: string | null;
  place?: string | null;
  coupling?: Record<string, string> | null;
}

interface CollectivesData {
  version: string;
  collectives: CollectiveEntry[];
  relationships?: unknown[];
}

// =============================================================================
// Helpers
// =============================================================================

/**
 * Map extended fields into the metadata bag for CreateCollectiveInputView.
 * Fields not in the Rust schema (governanceModel, domain, place, coupling)
 * ride through metadata_json. Zero Rust changes.
 */
function toInputView(entry: CollectiveEntry): CreateCollectiveInputView {
  const metadata: Record<string, unknown> = {};
  if (entry.governanceModel) metadata.governanceModel = entry.governanceModel;
  if (entry.domain) metadata.domain = entry.domain;
  if (entry.place) metadata.place = entry.place;
  if (entry.coupling) metadata.coupling = entry.coupling;

  return {
    id: entry.id,
    name: entry.name,
    description: entry.description ?? null,
    governanceLayer: entry.governanceLayer,
    constitutionalParentId: entry.constitutionalParentId ?? null,
    reach: entry.reach ?? null,
    metadata: Object.keys(metadata).length > 0 ? metadata : null,
    createdBy: null,
  };
}

/**
 * Topological sort on constitutionalParentId — parents before children.
 * Handles cycles by appending remaining entries at the end (validator catches cycles).
 */
function topologicalSort(collectives: CollectiveEntry[]): CollectiveEntry[] {
  const byId = new Map(collectives.map(c => [c.id, c]));
  const sorted: CollectiveEntry[] = [];
  const visited = new Set<string>();

  function visit(c: CollectiveEntry): void {
    if (visited.has(c.id)) return;
    visited.add(c.id);
    // Visit parent first
    if (c.constitutionalParentId) {
      const parent = byId.get(c.constitutionalParentId);
      if (parent) visit(parent);
    }
    sorted.push(c);
  }

  for (const c of collectives) {
    visit(c);
  }

  return sorted;
}

// =============================================================================
// Main
// =============================================================================

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  const DRY_RUN = args.includes('--dry-run');
  const VALIDATE_ONLY = args.includes('--validate-only');
  const doorwayUrlArg = args.find(a => a.startsWith('--doorway-url='))?.split('=')[1]
    ?? (args.includes('--doorway-url') ? args[args.indexOf('--doorway-url') + 1] : undefined);

  const doorwayUrl = (doorwayUrlArg ?? process.env.DOORWAY_URL ?? 'https://doorway-alpha.elohim.host').replace(
    /\/$/,
    ''
  );

  const __dirname = dirname(fileURLToPath(import.meta.url));
  const collectivesFile = resolve(__dirname, '../../data/collectives/collectives.json');

  console.log('=== Seed Collectives ===\n');
  console.log(`Doorway:      ${doorwayUrl}`);
  console.log(`Collectives:  ${collectivesFile}`);
  if (DRY_RUN) console.log('Mode:         DRY RUN');
  if (VALIDATE_ONLY) console.log('Mode:         VALIDATE ONLY');
  console.log('');

  // ── Validate ──────────────────────────────────────────────────────────────
  console.log('Validating...');
  const validation = validateCollectivesFile(collectivesFile);
  const integrityErrors = validateReferentialIntegrity(validation);

  if (validation.errors.length > 0) {
    console.error('\nValidation errors:');
    for (const e of validation.errors) console.error(`  ✗ ${e}`);
    process.exit(1);
  }

  if (integrityErrors.length > 0) {
    console.error('\nReferential integrity errors:');
    for (const e of integrityErrors) console.error(`  ✗ ${e}`);
    process.exit(1);
  }

  console.log(`  ✓ ${validation.collectiveCount} collectives, ${validation.relationshipCount} relationships validated\n`);

  if (VALIDATE_ONLY) {
    console.log('Validation complete.');
    process.exit(0);
  }

  // ── Load and sort ─────────────────────────────────────────────────────────
  const data: CollectivesData = JSON.parse(readFileSync(collectivesFile, 'utf-8'));
  const sorted = topologicalSort(data.collectives);

  console.log(`Seeding ${sorted.length} collectives (topologically sorted)...\n`);

  if (DRY_RUN) {
    for (const coll of sorted) {
      const parent = coll.constitutionalParentId ? ` ← ${coll.constitutionalParentId}` : '';
      console.log(`  [DRY] ${coll.id.padEnd(40)} ${coll.governanceLayer.padEnd(12)} ${coll.reach.padEnd(10)} ${coll.name}${parent}`);
    }
    console.log('\nDry run complete. No collectives seeded.');
    return;
  }

  // ── Seed ──────────────────────────────────────────────────────────────────
  let created = 0;
  let failed = 0;

  for (const coll of sorted) {
    const inputView = toInputView(coll);

    try {
      const res = await fetch(`${doorwayUrl}/db/collectives`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(inputView),
      });

      if (res.ok) {
        console.log(`  [+] ${coll.id.padEnd(40)} ${coll.governanceLayer.padEnd(12)} ${coll.name}`);
        created++;
      } else {
        const errorText = await res.text();
        console.log(`  [X] ${coll.id.padEnd(40)} HTTP ${res.status}: ${errorText}`);
        failed++;
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.log(`  [X] ${coll.id.padEnd(40)} ${msg}`);
      failed++;
    }
  }

  console.log(`\n=== Results: ${created} created, ${failed} failed ===`);

  if (failed > 0) {
    console.error(`\n${failed} collective(s) failed to seed.`);
    process.exit(1);
  }

  process.exit(0);
}

main();
```

- [ ] **Step 2: Add validate and seed script entries to package.json**

Add to `genesis/seeder/package.json` scripts:

```json
"validate:collectives": "npx tsx src/validate-collectives.ts",
"seed:collectives:validate": "npx tsx src/seed-collectives.ts --validate-only"
```

- [ ] **Step 3: Run validation to verify the full chain works**

Run: `cd /projects/elohim/genesis/seeder && npx tsx src/seed-collectives.ts --validate-only`
Expected: `✓ 47 collectives, 25 relationships validated`

- [ ] **Step 4: Commit**

```bash
git add genesis/seeder/src/seed-collectives.ts genesis/seeder/package.json
git commit -m "feat(qahal): seeder validates collectives against schema before POSTing

Validates enum values, referential integrity, and circular chains before
any HTTP calls. Maps governanceModel, domain, place, coupling into metadata
bag for CreateCollectiveInputView. Topological sort ensures parents created
before children. Adds --validate-only flag."
```

---

### Task 6: Update a2o scenario for narrative coherence

**Files:**
- Modify: `genesis/a2o/features/qahal/collective-governance.feature`

The Background references `"Valley Bible Study"` which now exists as `bible-study-valley` in the seed data with constitutional parent `community-local-church`.

- [ ] **Step 1: Verify the scenario references match seed data**

Check that all collective names referenced in the feature file exist in collectives.json:

```bash
cd /projects/elohim
grep -oP '"[^"]*"' genesis/a2o/features/qahal/collective-governance.feature | sort -u
# Cross-reference with:
python3 -c "import json; d=json.load(open('genesis/data/collectives/collectives.json')); [print(c['name']) for c in d['collectives']]" | sort
```

- [ ] **Step 2: Update the feature Background if needed**

The current Background:
```gherkin
Given I am "Matthew" in the "Valley Bible Study" collective
```

This should match `bible-study-valley` whose `name` is `"Valley Bible Study"`. Verify the scenario at line 41 references `"homeschool-coop"` which matches `community-homeschool-coop`. Update any mismatches.

- [ ] **Step 3: Commit**

```bash
git add genesis/a2o/features/qahal/collective-governance.feature
git commit -m "fix(a2o): align collective-governance scenario with enriched seed data

Verify Background 'Valley Bible Study' matches bible-study-valley collective.
Ensure all referenced collective names match seed data."
```

---

### Task 7: Final verification — full validation pass

**Files:** None (verification only)

- [ ] **Step 1: Run the collectives validator**

Run: `cd /projects/elohim/genesis/seeder && npx tsx src/validate-collectives.ts`
Expected: `✓ All validations passed`

- [ ] **Step 2: Run the collectives validator test suite**

Run: `cd /projects/elohim/genesis/seeder && pnpm exec vitest run src/__tests__/validate-collectives.test.ts`
Expected: All tests PASS

- [ ] **Step 3: Run the seeder in dry-run mode**

Run: `cd /projects/elohim/genesis/seeder && npx tsx src/seed-collectives.ts --dry-run`
Expected: 47 collectives listed in topological order (parents before children), no errors

- [ ] **Step 4: Verify the routing fix compiles**

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check`
Expected: Compiles clean

- [ ] **Step 5: Verify account packages are valid JSON**

Run: `cd /projects/elohim && for f in genesis/data/account-packages/*.json; do python3 -c "import json; json.load(open('$f'))" || echo "FAIL: $f"; done`
Expected: No FAIL output
