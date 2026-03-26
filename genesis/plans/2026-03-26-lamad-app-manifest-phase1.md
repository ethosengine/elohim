# Lamad App Manifest — Phase 1 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create the first app manifest EPR (Lamad) by extracting vocabulary from scattered codebase locations into a validated, codegen-ready manifest document.

**Architecture:** The manifest JSON schema defines what ANY app manifest looks like (protocol-level). The Lamad manifest is the first instance — it declares lamad's content types, formats, renderers, relationship patterns, value flows, and signals. A codegen script reads the manifest and generates TypeScript types that run alongside (not replacing) the existing schema-enums approach.

**Tech Stack:** JSON Schema, TypeScript, Vitest, Node.js codegen scripts

**Design doc:** `genesis/plans/2026-03-26-app-manifest-sdk-boundary-design.md`

---

## Task 1: Define the App Manifest JSON Schema

The manifest schema is protocol-level — it defines the SHAPE of any app manifest. Lives alongside the other protocol schemas.

**Files:**
- Create: `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json`

**Step 1: Write the manifest schema**

This schema validates the structure of any app manifest. It does NOT enumerate content types — it validates that the manifest declares them with proper three-leg coupling.

```json
{
  "$id": "epr:schema:manifest:app-manifest",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AppManifest",
  "description": "Application manifest declaring vocabulary, coupling, and rendering for an app built on the Elohim Protocol. The manifest itself is an EPR — content-addressed, stewarded, governed.",
  "type": "object",
  "required": ["id", "name", "version", "vocabulary"],
  "additionalProperties": false,
  "properties": {
    "id": {
      "type": "string",
      "description": "EPR content ID for this manifest"
    },
    "name": {
      "type": "string",
      "description": "App name (lowercase, kebab-case)"
    },
    "version": {
      "type": "string",
      "description": "Manifest version (semver)"
    },
    "description": {
      "type": "string"
    },
    "vocabulary": {
      "type": "object",
      "required": ["contentTypes"],
      "additionalProperties": false,
      "properties": {
        "contentTypes": {
          "type": "object",
          "description": "Map of content type name to its declaration",
          "additionalProperties": { "$ref": "#/$defs/ContentTypeDeclaration" }
        },
        "contentFormats": {
          "type": "object",
          "description": "Map of content format name to its declaration",
          "additionalProperties": { "$ref": "#/$defs/ContentFormatDeclaration" }
        },
        "relationships": {
          "type": "object",
          "description": "Map of relationship type name to its declaration",
          "additionalProperties": { "$ref": "#/$defs/RelationshipDeclaration" }
        },
        "signals": {
          "type": "object",
          "description": "Map of signal name to its declaration",
          "additionalProperties": { "$ref": "#/$defs/SignalDeclaration" }
        }
      }
    },
    "rendering": {
      "type": "object",
      "description": "Map of renderer name to its registration",
      "additionalProperties": { "$ref": "#/$defs/RendererRegistration" }
    }
  },
  "$defs": {
    "ContentTypeDeclaration": {
      "type": "object",
      "required": ["description", "coupling"],
      "additionalProperties": false,
      "properties": {
        "description": { "type": "string" },
        "bodySchema": {
          "description": "JSON Schema reference for this type's content body. Omit for freeform.",
          "type": "string"
        },
        "coupling": { "$ref": "#/$defs/ThreeLegCoupling" }
      }
    },
    "ThreeLegCoupling": {
      "type": "object",
      "required": ["value", "governance"],
      "additionalProperties": false,
      "properties": {
        "knowledge": {
          "type": "object",
          "additionalProperties": false,
          "properties": {
            "relationships": {
              "type": "object",
              "description": "Map of relationship type to target content type(s)",
              "additionalProperties": {
                "oneOf": [
                  { "type": "string" },
                  { "type": "array", "items": { "type": "string" } }
                ]
              }
            }
          }
        },
        "value": {
          "type": "object",
          "additionalProperties": false,
          "properties": {
            "onConsume": { "$ref": "#/$defs/ValueFlowEvent" },
            "onComplete": { "$ref": "#/$defs/ValueFlowEvent" },
            "onContribute": { "$ref": "#/$defs/ValueFlowEvent" }
          }
        },
        "governance": {
          "type": "object",
          "additionalProperties": false,
          "properties": {
            "defaultReach": { "type": "string" },
            "minimumReach": { "type": "string" },
            "governanceModel": { "type": "string" },
            "signalTypes": {
              "type": "array",
              "items": { "type": "string" }
            }
          }
        }
      }
    },
    "ValueFlowEvent": {
      "type": "object",
      "required": ["action"],
      "additionalProperties": false,
      "properties": {
        "action": {
          "type": "string",
          "description": "REA action type (use, produce, consume, transfer, etc.)"
        },
        "resourceConformsTo": {
          "type": "string",
          "description": "What kind of value this represents"
        },
        "recognition": {
          "type": "string",
          "description": "How recognition flows (steward-weighted, author+steward, etc.)"
        }
      }
    },
    "ContentFormatDeclaration": {
      "type": "object",
      "required": ["description"],
      "additionalProperties": false,
      "properties": {
        "description": { "type": "string" },
        "renderer": {
          "type": "string",
          "description": "Renderer name from the rendering section"
        },
        "mimeTypes": {
          "type": "array",
          "items": { "type": "string" }
        },
        "extensions": {
          "type": "array",
          "items": { "type": "string" }
        },
        "aliases": {
          "type": "array",
          "items": { "type": "string" },
          "description": "Legacy format names that resolve to this format"
        }
      }
    },
    "RelationshipDeclaration": {
      "type": "object",
      "required": ["description"],
      "additionalProperties": false,
      "properties": {
        "description": { "type": "string" },
        "source": {
          "type": "array",
          "items": { "type": "string" },
          "description": "Content types that can be the source"
        },
        "target": {
          "type": "array",
          "items": { "type": "string" },
          "description": "Content types that can be the target"
        },
        "inverse": {
          "type": "string",
          "description": "Name of the inverse relationship (if bidirectional)"
        }
      }
    },
    "SignalDeclaration": {
      "type": "object",
      "required": ["description", "substrateSignal"],
      "additionalProperties": false,
      "properties": {
        "description": { "type": "string" },
        "substrateSignal": {
          "type": "string",
          "enum": ["attention", "compute", "storage", "bandwidth", "resource"],
          "description": "Which protocol substrate signal this maps to"
        },
        "economicAction": {
          "type": "string",
          "description": "REA action type this generates"
        },
        "resourceType": {
          "type": "string",
          "description": "What kind of value resource this signal represents"
        }
      }
    },
    "RendererRegistration": {
      "type": "object",
      "required": ["formats"],
      "additionalProperties": false,
      "properties": {
        "component": {
          "type": "string",
          "description": "Angular component class name"
        },
        "formats": {
          "type": "array",
          "items": { "type": "string" },
          "description": "Content formats this renderer handles"
        },
        "platform": {
          "type": "string",
          "description": "Rendering platform (angular, web-component, etc.)"
        }
      }
    }
  }
}
```

**Step 2: Commit**

```bash
git add elohim/sdk/schemas/v1/manifest/app-manifest.schema.json
git commit -m "feat(schema): add app manifest JSON schema — protocol-level manifest structure"
```

---

## Task 2: Write Manifest Schema Validation Tests

**Files:**
- Create: `elohim/sdk/schemas/scripts/test-manifest-schema.mjs`

**Step 1: Write validation tests**

```javascript
import { readFile } from 'fs/promises';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import Ajv from 'ajv';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SCHEMA_DIR = resolve(__dirname, '../v1/manifest');

const ajv = new Ajv({ strict: false, allErrors: true });

let passed = 0;
let failed = 0;

function test(name, fn) {
  try {
    fn();
    console.log(`PASS: ${name}`);
    passed++;
  } catch (e) {
    console.log(`FAIL: ${name} — ${e.message}`);
    failed++;
  }
}

const schemaText = await readFile(resolve(SCHEMA_DIR, 'app-manifest.schema.json'), 'utf8');
const schema = JSON.parse(schemaText);
const validate = ajv.compile(schema);

// Minimal valid manifest
test('accepts minimal valid manifest', () => {
  const valid = validate({
    id: 'manifest-test',
    name: 'test-app',
    version: '1.0.0',
    vocabulary: {
      contentTypes: {
        'test-type': {
          description: 'A test content type',
          coupling: {
            value: { onConsume: { action: 'use' } },
            governance: { defaultReach: 'community' }
          }
        }
      }
    }
  });
  if (!valid) throw new Error(JSON.stringify(validate.errors, null, 2));
});

// Full manifest with all sections
test('accepts full manifest with rendering and signals', () => {
  const valid = validate({
    id: 'manifest-full',
    name: 'full-app',
    version: '2.0.0',
    description: 'A fully populated manifest',
    vocabulary: {
      contentTypes: {
        'lesson': {
          description: 'A learning lesson',
          bodySchema: 'schemas/lesson.schema.json',
          coupling: {
            knowledge: { relationships: { TEACHES: 'concept' } },
            value: {
              onConsume: { action: 'use', resourceConformsTo: 'learning', recognition: 'steward-weighted' },
              onComplete: { action: 'produce', resourceConformsTo: 'mastery', recognition: 'author+steward' }
            },
            governance: {
              defaultReach: 'community',
              minimumReach: 'intimate',
              governanceModel: 'steward-consent',
              signalTypes: ['lesson-completed']
            }
          }
        }
      },
      contentFormats: {
        'markdown': { description: 'Markdown text', renderer: 'md-renderer', mimeTypes: ['text/markdown'], extensions: ['.md'] }
      },
      relationships: {
        'TEACHES': { description: 'Lesson teaches a concept', source: ['lesson'], target: ['concept'] }
      },
      signals: {
        'lesson-completed': { description: 'Learner finished lesson', substrateSignal: 'attention', economicAction: 'use', resourceType: 'learning' }
      }
    },
    rendering: {
      'md-renderer': { component: 'MarkdownRendererComponent', formats: ['markdown'], platform: 'angular' }
    }
  });
  if (!valid) throw new Error(JSON.stringify(validate.errors, null, 2));
});

// Missing required fields
test('rejects manifest without id', () => {
  const valid = validate({ name: 'test', version: '1.0.0', vocabulary: { contentTypes: {} } });
  if (valid) throw new Error('Should have rejected');
});

test('rejects manifest without vocabulary', () => {
  const valid = validate({ id: 'test', name: 'test', version: '1.0.0' });
  if (valid) throw new Error('Should have rejected');
});

// Coupling validation
test('rejects content type without coupling', () => {
  const valid = validate({
    id: 'test', name: 'test', version: '1.0.0',
    vocabulary: { contentTypes: { 'bad': { description: 'no coupling' } } }
  });
  if (valid) throw new Error('Should have rejected — missing coupling');
});

test('rejects coupling without value leg', () => {
  const valid = validate({
    id: 'test', name: 'test', version: '1.0.0',
    vocabulary: {
      contentTypes: {
        'bad': { description: 'missing value', coupling: { governance: { defaultReach: 'public' } } }
      }
    }
  });
  if (valid) throw new Error('Should have rejected — missing value leg');
});

test('rejects coupling without governance leg', () => {
  const valid = validate({
    id: 'test', name: 'test', version: '1.0.0',
    vocabulary: {
      contentTypes: {
        'bad': { description: 'missing governance', coupling: { value: { onConsume: { action: 'use' } } } }
      }
    }
  });
  if (valid) throw new Error('Should have rejected — missing governance leg');
});

// Signal substrate validation
test('rejects signal with invalid substrate', () => {
  const valid = validate({
    id: 'test', name: 'test', version: '1.0.0',
    vocabulary: {
      contentTypes: { 'x': { description: 'x', coupling: { value: { onConsume: { action: 'use' } }, governance: {} } } },
      signals: { 's': { description: 's', substrateSignal: 'invalid-substrate' } }
    }
  });
  if (valid) throw new Error('Should have rejected — invalid substrate signal');
});

console.log(`\n${passed} passed, ${failed} failed`);
process.exit(failed > 0 ? 1 : 0);
```

**Step 2: Add script to root package.json**

Add to `package.json` scripts:
```json
"manifest:test": "node elohim/sdk/schemas/scripts/test-manifest-schema.mjs"
```

**Step 3: Run tests**

Run: `pnpm -w run manifest:test`
Expected: All 8 tests pass.

**Step 4: Commit**

```bash
git add elohim/sdk/schemas/scripts/test-manifest-schema.mjs package.json
git commit -m "test(schema): add app manifest schema validation tests"
```

---

## Task 3: Create the Lamad App Manifest

Extract lamad vocabulary from the codebase into the first manifest EPR. This is the largest task — it requires reading existing model files and distilling them into manifest declarations.

**Files:**
- Create: `genesis/data/manifests/manifest-lamad.json`

**Step 1: Write the Lamad manifest**

This extracts vocabulary from these source files (read but don't modify):
- `app/elohim-app/src/app/lamad/models/content-node.model.ts` (content types, formats, relationships, stewardship roles)
- `app/elohim-app/src/app/lamad/models/learning-path.model.ts` (path structure)
- `app/elohim-app/src/app/lamad/models/learning-points.model.ts` (point triggers, recognition flows)
- `app/elohim-app/src/app/lamad/models/steward-economy.model.ts` (steward tiers, pricing)
- `app/elohim-app/src/app/lamad/models/feedback-profile.model.ts` (feedback mechanisms)
- `app/elohim-app/src/app/lamad/services/mastery.service.ts` (mastery levels, engagement types)
- `app/elohim-app/src/app/lamad/renderers/renderer-initializer.service.ts` (renderer registrations)
- `app/elohim-app/src/app/lamad/content-io/plugins/` (format plugins)

The manifest should contain:
- All lamad-specific content types with three-leg coupling
- All content formats with renderer mappings
- All relationship types
- All signal declarations mapping to substrate signals
- All renderer registrations

Note: Content types already in the protocol `core` tier (epic, concept, lesson, etc.) are included in the manifest because lamad assigns coupling to them. The manifest doesn't redefine them — it declares how lamad couples them to value and governance.

**Step 2: Validate manifest against schema**

Run: `pnpm -w run manifest:test` (should still pass — schema validates structure)

Then validate the actual Lamad manifest:
```bash
node -e "
import Ajv from 'ajv';
import { readFileSync } from 'fs';
const schema = JSON.parse(readFileSync('elohim/sdk/schemas/v1/manifest/app-manifest.schema.json', 'utf8'));
const manifest = JSON.parse(readFileSync('genesis/data/manifests/manifest-lamad.json', 'utf8'));
const ajv = new Ajv({ strict: false, allErrors: true });
const valid = ajv.validate(schema, manifest);
if (valid) console.log('✅ Lamad manifest valid');
else { console.error('❌', ajv.errors); process.exit(1); }
"
```

**Step 3: Commit**

```bash
git add genesis/data/manifests/manifest-lamad.json
git commit -m "feat(manifest): create Lamad app manifest — first EPR app manifest"
```

---

## Task 4: Write Lamad Manifest Content Tests

Tests that validate the Lamad manifest's content is complete and consistent — not just structurally valid (Task 2 covers that) but semantically correct.

**Files:**
- Create: `genesis/seeder/src/__tests__/manifest-lamad.test.ts`

**Step 1: Write content validation tests**

```typescript
import { describe, it, expect } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, '../../../..');
const MANIFEST_PATH = path.join(REPO_ROOT, 'genesis/data/manifests/manifest-lamad.json');

interface AppManifest {
  id: string;
  name: string;
  version: string;
  vocabulary: {
    contentTypes: Record<string, { description: string; coupling: unknown }>;
    contentFormats?: Record<string, { description: string; renderer?: string }>;
    relationships?: Record<string, { description: string }>;
    signals?: Record<string, { description: string; substrateSignal: string }>;
  };
  rendering?: Record<string, { formats: string[]; component?: string }>;
}

const manifest: AppManifest = JSON.parse(fs.readFileSync(MANIFEST_PATH, 'utf8'));

describe('Lamad App Manifest — Content Validation', () => {
  it('should have the correct app identity', () => {
    expect(manifest.id).toBe('manifest-lamad');
    expect(manifest.name).toBe('lamad');
    expect(manifest.version).toMatch(/^\d+\.\d+\.\d+$/);
  });

  it('should declare all lamad content types', () => {
    const types = Object.keys(manifest.vocabulary.contentTypes);
    // Core learning types that lamad couples
    expect(types).toContain('lesson');
    expect(types).toContain('assessment');
    expect(types).toContain('path');
    expect(types).toContain('concept');
    expect(types).toContain('exercise');
    expect(types).toContain('reflection');
    // App-layer extensions
    expect(types).toContain('discovery-assessment');
    expect(types).toContain('instrument');
  });

  it('every content type should have three-leg coupling', () => {
    for (const [name, decl] of Object.entries(manifest.vocabulary.contentTypes)) {
      const coupling = decl.coupling as Record<string, unknown>;
      expect(coupling, `${name} missing coupling`).toBeDefined();
      expect(coupling.value, `${name} missing value leg`).toBeDefined();
      expect(coupling.governance, `${name} missing governance leg`).toBeDefined();
    }
  });

  it('should declare content formats with renderers', () => {
    const formats = manifest.vocabulary.contentFormats ?? {};
    expect(Object.keys(formats).length).toBeGreaterThan(0);
    // Key lamad formats
    expect(formats['sophia-quiz-json']).toBeDefined();
    expect(formats['markdown']).toBeDefined();
  });

  it('every format with a renderer should reference a valid renderer', () => {
    const formats = manifest.vocabulary.contentFormats ?? {};
    const renderers = manifest.rendering ?? {};
    for (const [name, decl] of Object.entries(formats)) {
      if (decl.renderer) {
        expect(
          renderers[decl.renderer],
          `Format '${name}' references renderer '${decl.renderer}' which is not registered`
        ).toBeDefined();
      }
    }
  });

  it('every renderer should list formats it handles', () => {
    const renderers = manifest.rendering ?? {};
    for (const [name, reg] of Object.entries(renderers)) {
      expect(reg.formats.length, `Renderer '${name}' has no formats`).toBeGreaterThan(0);
    }
  });

  it('should declare signals that map to valid substrate signals', () => {
    const validSubstrate = ['attention', 'compute', 'storage', 'bandwidth', 'resource'];
    const signals = manifest.vocabulary.signals ?? {};
    for (const [name, decl] of Object.entries(signals)) {
      expect(
        validSubstrate,
        `Signal '${name}' has invalid substrate '${decl.substrateSignal}'`
      ).toContain(decl.substrateSignal);
    }
  });

  it('should declare relationship types', () => {
    const rels = manifest.vocabulary.relationships ?? {};
    expect(Object.keys(rels).length).toBeGreaterThan(0);
    // Key lamad relationships
    expect(rels['CONTAINS']).toBeDefined();
    expect(rels['REFERENCES']).toBeDefined();
  });
});
```

**Step 2: Run tests**

Run: `cd genesis/seeder && npx vitest run src/__tests__/manifest-lamad.test.ts`
Expected: All tests pass.

**Step 3: Commit**

```bash
git add genesis/seeder/src/__tests__/manifest-lamad.test.ts
git commit -m "test(manifest): add Lamad manifest content validation tests"
```

---

## Task 5: Build Manifest Codegen Script

Generates TypeScript types from an app manifest — the app-level equivalent of `schema:codegen:ts`.

**Files:**
- Create: `elohim/sdk/schemas/scripts/codegen-manifest.mjs`

**Step 1: Write the codegen script**

The script reads a manifest and generates:
- Content type union: `type LamadContentType = 'lesson' | 'assessment' | ...`
- Content format union: `type LamadContentFormat = 'sophia-quiz-json' | 'markdown' | ...`
- Signal type union: `type LamadSignal = 'learning-signal' | 'mastery-achieved' | ...`
- Relationship type union: `type LamadRelationship = 'CONTAINS' | 'REFERENCES' | ...`
- Runtime arrays: `const LAMAD_CONTENT_TYPES = [...] as const`

```javascript
#!/usr/bin/env node
/**
 * Manifest Codegen — generate TypeScript types from app manifests.
 *
 * Usage:
 *   node codegen-manifest.mjs <manifest-path> <output-path>
 *
 * Example:
 *   node codegen-manifest.mjs genesis/data/manifests/manifest-lamad.json \
 *     app/elohim-app/src/app/lamad/generated/manifest-types.ts
 */

import { readFile, writeFile, mkdir } from 'fs/promises';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '../../../..');

const VERIFY = process.argv.includes('--verify');
const manifestPath = process.argv[2];
const outputPath = process.argv[3];

if (!manifestPath || !outputPath) {
  console.error('Usage: codegen-manifest.mjs <manifest-path> <output-path>');
  process.exit(1);
}

const manifest = JSON.parse(await readFile(resolve(REPO_ROOT, manifestPath), 'utf8'));
const appName = manifest.name;
const pascalName = appName.charAt(0).toUpperCase() + appName.slice(1);

// Generate content
const lines = [
  `// AUTO-GENERATED from app manifest: ${manifestPath}`,
  `// DO NOT EDIT — regenerate with: pnpm run manifest:codegen`,
  `//`,
  `// App: ${manifest.name} v${manifest.version}`,
  `// ${manifest.description || ''}`,
  '',
];

// Content types
const contentTypes = Object.keys(manifest.vocabulary.contentTypes);
if (contentTypes.length > 0) {
  lines.push(`export const ${pascalName.toUpperCase()}_CONTENT_TYPES = [`);
  for (const t of contentTypes) {
    lines.push(`  '${t}',`);
  }
  lines.push(`] as const;`);
  lines.push(`export type ${pascalName}ContentType = (typeof ${pascalName.toUpperCase()}_CONTENT_TYPES)[number];`);
  lines.push('');
}

// Content formats
const contentFormats = Object.keys(manifest.vocabulary.contentFormats || {});
if (contentFormats.length > 0) {
  lines.push(`export const ${pascalName.toUpperCase()}_CONTENT_FORMATS = [`);
  for (const f of contentFormats) {
    lines.push(`  '${f}',`);
  }
  lines.push(`] as const;`);
  lines.push(`export type ${pascalName}ContentFormat = (typeof ${pascalName.toUpperCase()}_CONTENT_FORMATS)[number];`);
  lines.push('');
}

// Relationships
const relationships = Object.keys(manifest.vocabulary.relationships || {});
if (relationships.length > 0) {
  lines.push(`export const ${pascalName.toUpperCase()}_RELATIONSHIPS = [`);
  for (const r of relationships) {
    lines.push(`  '${r}',`);
  }
  lines.push(`] as const;`);
  lines.push(`export type ${pascalName}Relationship = (typeof ${pascalName.toUpperCase()}_RELATIONSHIPS)[number];`);
  lines.push('');
}

// Signals
const signals = Object.keys(manifest.vocabulary.signals || {});
if (signals.length > 0) {
  lines.push(`export const ${pascalName.toUpperCase()}_SIGNALS = [`);
  for (const s of signals) {
    lines.push(`  '${s}',`);
  }
  lines.push(`] as const;`);
  lines.push(`export type ${pascalName}Signal = (typeof ${pascalName.toUpperCase()}_SIGNALS)[number];`);
  lines.push('');
}

// Renderer map (format -> component)
const rendering = manifest.rendering || {};
if (Object.keys(rendering).length > 0) {
  lines.push(`/** Format-to-renderer mapping */`);
  lines.push(`export const ${pascalName.toUpperCase()}_RENDERER_MAP: Record<string, { component: string; platform: string }> = {`);
  for (const [name, reg] of Object.entries(rendering)) {
    for (const fmt of reg.formats) {
      lines.push(`  '${fmt}': { component: '${reg.component || name}', platform: '${reg.platform || 'angular'}' },`);
    }
  }
  lines.push(`};`);
  lines.push('');
}

const content = lines.join('\n') + '\n';
const absOutput = resolve(REPO_ROOT, outputPath);

if (VERIFY) {
  try {
    const existing = await readFile(absOutput, 'utf8');
    if (existing !== content) {
      console.error(`VERIFY FAILED: ${outputPath} is out of date. Run: pnpm run manifest:codegen`);
      process.exit(1);
    }
    console.log(`VERIFY OK: ${outputPath}`);
  } catch {
    console.error(`VERIFY FAILED: ${outputPath} does not exist. Run: pnpm run manifest:codegen`);
    process.exit(1);
  }
} else {
  await mkdir(dirname(absOutput), { recursive: true });
  await writeFile(absOutput, content, 'utf8');
  console.log(`Generated: ${outputPath}`);
}
```

**Step 2: Add scripts to root package.json**

```json
"manifest:codegen": "node elohim/sdk/schemas/scripts/codegen-manifest.mjs genesis/data/manifests/manifest-lamad.json app/elohim-app/src/app/lamad/generated/manifest-types.ts",
"manifest:codegen:verify": "node elohim/sdk/schemas/scripts/codegen-manifest.mjs --verify genesis/data/manifests/manifest-lamad.json app/elohim-app/src/app/lamad/generated/manifest-types.ts"
```

**Step 3: Run codegen**

Run: `pnpm -w run manifest:codegen`
Expected: `Generated: app/elohim-app/src/app/lamad/generated/manifest-types.ts`

**Step 4: Verify the generated file**

Read `app/elohim-app/src/app/lamad/generated/manifest-types.ts` and confirm it contains:
- `LAMAD_CONTENT_TYPES` array
- `LamadContentType` union type
- `LAMAD_CONTENT_FORMATS` array
- `LamadContentFormat` union type
- `LAMAD_RENDERER_MAP` object
- Signal and relationship types

**Step 5: Commit**

```bash
git add elohim/sdk/schemas/scripts/codegen-manifest.mjs \
        app/elohim-app/src/app/lamad/generated/manifest-types.ts \
        package.json
git commit -m "feat(manifest): add manifest codegen script and generate Lamad types"
```

---

## Task 6: Wire Generated Types Into Lamad (Parallel)

Replace `AppContentTypeExtension` in `content-node.model.ts` with the manifest-generated types. This runs ALONGSIDE existing schema-enums — both are valid, neither replaces the other yet.

**Files:**
- Modify: `app/elohim-app/src/app/lamad/models/content-node.model.ts`

**Step 1: Import manifest-generated types**

Add import:
```typescript
import {
  type LamadContentType,
  type LamadContentFormat,
} from '../generated/manifest-types';
```

**Step 2: Replace AppContentTypeExtension with manifest types**

Change:
```typescript
type AppContentTypeExtension =
  | 'community'
  | 'discovery-assessment'
  | 'instrument'
  | 'tool'
  | 'placeholder';

export type ContentType = WireContentType | AppContentTypeExtension;
```

To:
```typescript
/** Frontend-only types not yet in any manifest */
type AppContentTypeExtension = 'placeholder';

export type ContentType = WireContentType | LamadContentType | AppContentTypeExtension;
```

This works because `LamadContentType` includes all the types that were in `AppContentTypeExtension` (community, discovery-assessment, instrument, tool) PLUS the protocol wire types that lamad couples (lesson, assessment, etc.). The union deduplicates automatically.

**Step 3: Run typecheck**

Run: `cd app/elohim-app && npx tsc --noEmit 2>&1 | grep "error TS" | grep -v "spec\.ts\|test\.ts\|scripts/\|vite\.config\|quiz-migration\|quiz-engine/index\|stewardship-allocation" | head -10`
Expected: No new errors.

**Step 4: Commit**

```bash
git add app/elohim-app/src/app/lamad/models/content-node.model.ts
git commit -m "refactor(lamad): wire manifest-generated types into content model (parallel)"
```

---

## Task 7: Add Manifest Codegen to CI

Ensure manifest codegen stays fresh — add verification to the Genesis pipeline.

**Files:**
- Modify: `genesis/Jenkinsfile`

**Step 1: Add verify step to Validate Constants stage**

In the existing `dir('genesis/seeder')` block in the Validate Constants stage, add BEFORE the vitest run:

```groovy
sh '''#!/bin/bash
    set -euo pipefail

    echo "Verifying manifest codegen is up to date..."
    cd "$WORKSPACE"
    pnpm -w run manifest:codegen:verify
    echo "✅ Manifest codegen verified"
'''
```

**Step 2: Commit**

```bash
git add genesis/Jenkinsfile
git commit -m "ci(genesis): add manifest codegen verification to Validate Constants stage"
```

---

## Verification

After all tasks complete:

1. **Manifest schema tests**: `pnpm -w run manifest:test` → all pass
2. **Manifest codegen**: `pnpm -w run manifest:codegen` → generates types
3. **Codegen verify**: `pnpm -w run manifest:codegen:verify` → matches
4. **Lamad manifest tests**: `cd genesis/seeder && npx vitest run src/__tests__/manifest-lamad.test.ts` → all pass
5. **Seeder typecheck**: `cd genesis/seeder && npx tsc --noEmit` → 0 errors
6. **Seeder tests**: `cd genesis/seeder && npx vitest run` → all pass
7. **App typecheck**: `cd app/elohim-app && npx tsc --noEmit 2>&1 | grep "error TS" | grep -v "spec\|test\|scripts\|vite\|quiz-migration\|quiz-engine/index\|stewardship-allocation"` → 0 new errors

## What This Does NOT Do (Phase 2+)

- **Replace protocol schema extensible tier** — both exist in parallel
- **Validate content bodies against manifest schemas** — manifest declares `bodySchema` refs but nothing enforces them yet
- **Generate coupling validators** — the three-leg coupling is declared but not enforced at runtime yet
- **Connect to elohim agent** — agent doesn't read manifests yet
- **Cross-manifest composition** — no mechanism for one app to reference another's types
- **Manifest governance** — no stewardship/reach on the manifest EPR itself yet
