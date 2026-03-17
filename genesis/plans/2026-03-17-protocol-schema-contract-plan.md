# Protocol Schema Contract — Phase 1 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Establish JSON Schema as the single IoC contract between DNA, storage, and genesis — delivering dev-time validation of seed files against the protocol schema.

**Architecture:** JSON Schema files in `elohim/sdk/schemas/v1/` define the protocol's data vocabulary (enums, input types, view types). Seed files reference their schema via `$schema` for instant VS Code validation. AJV validates on `pnpm run schema:validate`. Husky enforces on pre-push. TypeScript codegen from schemas is verified against existing ts-rs output but does NOT replace it yet (that's Phase 2).

**Tech Stack:** JSON Schema (Draft 2020-12), AJV (v8+ CLI), json-schema-to-typescript (v15+), Node.js scripts

**Design Doc:** `genesis/plans/2026-03-17-protocol-schema-contract-design.md`

---

## Critical Discovery: Content Type Divergence

Seed files use **20** content types. The DNA allows **12**. This is the exact problem the schema solves.

| In seed files (3,526 files) | In DNA? | Resolution needed |
|---|---|---|
| scenario (2,690), feature (256), concept (99), epic (13), assessment (12) | Yes | Valid |
| organization (132), bible-verse (111), book (35), contributor (31), human (27) | **No** | Must classify: DNA expansion or storage-only? |
| course-module (15), practice (14), narrative (14), activity (13), video (12) | **No** | Must classify |
| role (8), documentary (5), book-chapter (4), podcast (3), simulation (1) | **No** | Must classify |
| *(not in seeds)* lesson, reflection, discussion, exercise, example, reference, article | Yes (DNA) | Valid but unused |

**Task 0 (user decision):** Before writing the schema, the user must classify each non-DNA content type as:
- (a) Map to existing DNA type (e.g., `bible-verse` → `reference`)
- (b) Add to DNA (uses entry type headroom, Lamad at ~73/~100)
- (c) Storage-only type (operational, not notarized)

The enum schema will include ALL approved types — both DNA-notarized and storage-only — with a `_dna` annotation distinguishing which are notarized.

---

### Task 1: Create Schema Directory Structure

**Files:**
- Create: `elohim/sdk/schemas/v1/_protocol.json`
- Create: `elohim/sdk/schemas/v1/enums/content-type.schema.json`
- Create: `elohim/sdk/schemas/current` (symlink → `v1`)

**Step 1: Create directory structure**

```bash
mkdir -p elohim/sdk/schemas/v1/enums
mkdir -p elohim/sdk/schemas/v1/entries
mkdir -p elohim/sdk/schemas/v1/views
mkdir -p elohim/sdk/schemas/v1/inputs
mkdir -p elohim/sdk/schemas/v1/migrations
```

**Step 2: Write protocol metadata**

Create `elohim/sdk/schemas/v1/_protocol.json`:

```json
{
  "version": 1,
  "parent": null,
  "canRead": [],
  "compatibility": "genesis",
  "breaking": false,
  "created": "2026-03-17T00:00:00Z",
  "migrationFrom": {}
}
```

**Step 3: Write content-type enum schema**

Create `elohim/sdk/schemas/v1/enums/content-type.schema.json`:

```json
{
  "$id": "epr:schema:enum:content-type",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "ContentType",
  "description": "Content type vocabulary. Values marked [DNA] are notarized in the Holochain DHT. Values marked [storage] are storage-only projections.",
  "type": "string",
  "enum": [
    "epic", "concept", "lesson", "scenario", "assessment",
    "resource", "reflection", "discussion", "exercise",
    "example", "reference", "article"
  ],
  "_dna": {
    "constant": "CONTENT_TYPES",
    "zome": "content_store_integrity",
    "file": "elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs"
  }
}
```

> **Note:** The enum starts with DNA-only values. After Task 0 resolution, storage-only types will be added with `_storageOnly` annotation. This schema will be updated before Task 6 (seed validation).

**Step 4: Create current symlink**

```bash
cd elohim/sdk/schemas && ln -s v1 current
```

**Step 5: Commit**

```bash
git add elohim/sdk/schemas/
git commit -m "feat(schema): create protocol schema directory structure with content-type enum"
```

---

### Task 2: Write All Enum Schemas

**Files:**
- Create: `elohim/sdk/schemas/v1/enums/content-format.schema.json`
- Create: `elohim/sdk/schemas/v1/enums/reach.schema.json`
- Create: `elohim/sdk/schemas/v1/enums/mastery-level.schema.json`
- Create: `elohim/sdk/schemas/v1/enums/constitutional-layer.schema.json`
- Create: `elohim/sdk/schemas/v1/enums/relationship-type.schema.json`
- Create: `elohim/sdk/schemas/v1/enums/validation-status.schema.json`

**Step 1: Write content-format enum**

Create `elohim/sdk/schemas/v1/enums/content-format.schema.json`:

```json
{
  "$id": "epr:schema:enum:content-format",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "ContentFormat",
  "description": "Content format for rendering. Matches DNA CONTENT_FORMATS constant.",
  "type": "string",
  "enum": [
    "markdown", "html", "video", "audio", "interactive", "external"
  ],
  "_dna": {
    "constant": "CONTENT_FORMATS",
    "zome": "content_store_integrity",
    "file": "elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs"
  }
}
```

**Step 2: Write reach enum**

Create `elohim/sdk/schemas/v1/enums/reach.schema.json`:

```json
{
  "$id": "epr:schema:enum:reach",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Reach",
  "description": "Content reach/visibility level. Ordered from most restrictive to most open. Matches DNA REACH_LEVELS constant.",
  "type": "string",
  "enum": [
    "private", "self", "intimate", "trusted", "familiar",
    "community", "public", "commons"
  ],
  "_dna": {
    "constant": "REACH_LEVELS",
    "zome": "content_store_integrity",
    "file": "elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs"
  }
}
```

**Step 3: Write mastery-level enum**

Create `elohim/sdk/schemas/v1/enums/mastery-level.schema.json`:

```json
{
  "$id": "epr:schema:enum:mastery-level",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "MasteryLevel",
  "description": "Bloom's taxonomy-aligned mastery progression. Matches DNA MASTERY_LEVELS constant.",
  "type": "string",
  "enum": [
    "not_started", "seen", "remember", "understand", "apply",
    "analyze", "evaluate", "create"
  ],
  "_dna": {
    "constant": "MASTERY_LEVELS",
    "zome": "content_store_integrity",
    "file": "elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs"
  }
}
```

**Step 4: Write constitutional-layer enum**

Create `elohim/sdk/schemas/v1/enums/constitutional-layer.schema.json`:

```json
{
  "$id": "epr:schema:enum:constitutional-layer",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "ConstitutionalLayer",
  "description": "Governance authority hierarchy (1=most flexible, 7=most immutable). Defined in elohim/constitution/src/types.rs.",
  "type": "string",
  "enum": [
    "individual", "family", "community", "provincial",
    "nation-state", "bioregional", "global"
  ],
  "_source": {
    "file": "elohim/constitution/src/types.rs",
    "type": "ConstitutionalLayer"
  }
}
```

**Step 5: Write relationship-type enum**

Verify DNA relationship types first:
- Read: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`
- Search for link type constants or relationship type definitions

Create `elohim/sdk/schemas/v1/enums/relationship-type.schema.json`:

```json
{
  "$id": "epr:schema:enum:relationship-type",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "RelationshipType",
  "description": "Content graph edge types.",
  "type": "string",
  "enum": [
    "contains", "belongs_to", "describes", "implements",
    "validates", "relates_to", "references", "depends_on",
    "requires", "follows", "derived_from", "source_of"
  ]
}
```

**Step 6: Write validation-status enum**

Create `elohim/sdk/schemas/v1/enums/validation-status.schema.json`:

```json
{
  "$id": "epr:schema:enum:validation-status",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "ValidationStatus",
  "description": "Schema migration status for records. Defined in views.rs.",
  "type": "string",
  "enum": ["valid", "migrated", "degraded", "healing"]
}
```

**Step 7: Commit**

```bash
git add elohim/sdk/schemas/v1/enums/
git commit -m "feat(schema): add all protocol enum schemas (content-format, reach, mastery, constitution, relationships, validation)"
```

---

### Task 3: Write CreateContentInput Schema

**Files:**
- Create: `elohim/sdk/schemas/v1/inputs/create-content-input.schema.json`
- Reference: `elohim/elohim-storage/src/views.rs:1180-1208` (CreateContentInputView)

**Step 1: Write input schema**

Create `elohim/sdk/schemas/v1/inputs/create-content-input.schema.json`:

```json
{
  "$id": "epr:schema:input:create-content",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "CreateContentInput",
  "description": "Input for creating content records. Must match Rust CreateContentInputView in views.rs.",
  "type": "object",
  "required": ["id", "title"],
  "properties": {
    "id": {
      "type": "string",
      "minLength": 1,
      "description": "Unique content identifier"
    },
    "title": {
      "type": "string",
      "minLength": 1,
      "description": "Display title"
    },
    "schemaVersion": {
      "type": "integer",
      "default": 1,
      "description": "Schema version for migration tracking"
    },
    "description": {
      "type": "string",
      "description": "Brief summary"
    },
    "contentType": {
      "$ref": "../enums/content-type.schema.json",
      "description": "Semantic content category"
    },
    "contentFormat": {
      "$ref": "../enums/content-format.schema.json",
      "description": "Rendering format hint"
    },
    "contentBody": {
      "type": "string",
      "description": "Full content body (markdown, HTML, etc.)"
    },
    "blobHash": {
      "type": "string",
      "description": "SHA-256 hash of associated blob"
    },
    "blobCid": {
      "type": "string",
      "description": "CIDv1 content address of associated blob"
    },
    "contentSizeBytes": {
      "type": "integer",
      "description": "Size of content/blob in bytes"
    },
    "metadata": {
      "type": "object",
      "description": "Domain-specific metadata (flexible schema)"
    },
    "reach": {
      "$ref": "../enums/reach.schema.json",
      "description": "Visibility/access level"
    },
    "createdBy": {
      "type": "string",
      "description": "Creator agent ID"
    },
    "tags": {
      "type": "array",
      "items": { "type": "string" },
      "default": [],
      "description": "Categorization tags"
    }
  },
  "additionalProperties": false
}
```

**Step 2: Verify schema matches Rust struct**

Manually compare the schema properties against `CreateContentInputView` fields in `views.rs:1180-1208`. Every field in the Rust struct must be in the schema, and vice versa. Field names must match after camelCase conversion.

**Step 3: Commit**

```bash
git add elohim/sdk/schemas/v1/inputs/
git commit -m "feat(schema): add CreateContentInput schema matching views.rs"
```

---

### Task 4: Write ContentView Schema

**Files:**
- Create: `elohim/sdk/schemas/v1/views/content-view.schema.json`
- Reference: `elohim/elohim-storage/src/views.rs:135-154` (ContentView)

**Step 1: Write view schema**

Create `elohim/sdk/schemas/v1/views/content-view.schema.json`:

```json
{
  "$id": "epr:schema:view:content",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "ContentView",
  "description": "Content record as returned by the storage API. Must match Rust ContentView in views.rs. Source of truth: DHT (Notarized, Category A).",
  "type": "object",
  "required": ["id", "appId", "title", "contentType", "contentFormat", "reach", "validationStatus", "createdAt", "updatedAt"],
  "properties": {
    "id": { "type": "string" },
    "appId": { "type": "string" },
    "title": { "type": "string" },
    "description": { "type": ["string", "null"] },
    "contentType": { "$ref": "../enums/content-type.schema.json" },
    "contentFormat": { "$ref": "../enums/content-format.schema.json" },
    "blobHash": { "type": ["string", "null"] },
    "blobCid": { "type": ["string", "null"] },
    "contentSizeBytes": { "type": ["integer", "null"] },
    "metadata": {},
    "reach": { "$ref": "../enums/reach.schema.json" },
    "validationStatus": { "$ref": "../enums/validation-status.schema.json" },
    "createdBy": { "type": ["string", "null"] },
    "createdAt": { "type": "string" },
    "updatedAt": { "type": "string" },
    "contentBody": { "type": ["string", "null"] },
    "dhtAnchorHash": { "type": ["string", "null"] }
  },
  "additionalProperties": false
}
```

**Step 2: Commit**

```bash
git add elohim/sdk/schemas/v1/views/
git commit -m "feat(schema): add ContentView schema matching views.rs"
```

---

### Task 5: Install Validation Tooling and Write Test Script

**Files:**
- Modify: `package.json` (root) — add devDependencies and scripts
- Create: `elohim/sdk/schemas/scripts/validate-seeds.mjs`
- Create: `elohim/sdk/schemas/scripts/test-schema.mjs`

**Step 1: Install AJV CLI**

```bash
pnpm add -Dw ajv-cli ajv-formats
```

**Step 2: Write seed validation script**

Create `elohim/sdk/schemas/scripts/validate-seeds.mjs`:

```javascript
#!/usr/bin/env node
/**
 * Validates genesis seed JSON files against the protocol schema.
 * Runs as: node elohim/sdk/schemas/scripts/validate-seeds.mjs
 * Exit code 0 = all valid, 1 = validation errors found.
 */
import { readdir, readFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import Ajv from 'ajv';

const SCHEMA_DIR = resolve(import.meta.dirname, '../current/inputs');
const SEED_DIR = resolve(import.meta.dirname, '../../../../genesis/data/lamad/content');
const SCHEMA_FILE = 'create-content-input.schema.json';

async function loadSchema(schemaDir, filename) {
  const schemaPath = join(schemaDir, filename);
  const raw = await readFile(schemaPath, 'utf8');
  return JSON.parse(raw);
}

async function loadEnumSchemas(enumDir) {
  const files = await readdir(enumDir);
  const schemas = [];
  for (const file of files) {
    if (file.endsWith('.schema.json')) {
      const raw = await readFile(join(enumDir, file), 'utf8');
      schemas.push(JSON.parse(raw));
    }
  }
  return schemas;
}

async function main() {
  const enumDir = resolve(import.meta.dirname, '../current/enums');
  const enumSchemas = await loadEnumSchemas(enumDir);
  const inputSchema = await loadSchema(SCHEMA_DIR, SCHEMA_FILE);

  const ajv = new Ajv({ allErrors: true, strict: false });

  // Register enum schemas so $ref can resolve them
  for (const schema of enumSchemas) {
    ajv.addSchema(schema);
  }

  const validate = ajv.compile(inputSchema);

  const files = (await readdir(SEED_DIR)).filter(f => f.endsWith('.json'));
  let errors = 0;
  let valid = 0;

  for (const file of files) {
    const raw = await readFile(join(SEED_DIR, file), 'utf8');
    let data;
    try {
      data = JSON.parse(raw);
    } catch {
      console.error(`PARSE ERROR: ${file}`);
      errors++;
      continue;
    }

    // Seed files may have extra fields not in CreateContentInput
    // (e.g., content array for assessments). For Phase 1, validate
    // only the fields the schema knows about (skip additionalProperties).
    if (!validate(data)) {
      const enumErrors = validate.errors.filter(e =>
        e.keyword === 'enum' || e.keyword === 'type' || e.keyword === 'required'
      );
      if (enumErrors.length > 0) {
        console.error(`INVALID: ${file}`);
        for (const err of enumErrors) {
          console.error(`  ${err.instancePath} ${err.message} (${JSON.stringify(err.params)})`);
        }
        errors++;
      } else {
        valid++;
      }
    } else {
      valid++;
    }
  }

  console.log(`\nSchema validation: ${valid} valid, ${errors} errors, ${files.length} total`);
  process.exit(errors > 0 ? 1 : 0);
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
```

**Step 3: Write schema self-test script**

Create `elohim/sdk/schemas/scripts/test-schema.mjs`:

```javascript
#!/usr/bin/env node
/**
 * Tests that protocol schemas accept valid data and reject invalid data.
 * This is the "unit test" for the schemas themselves.
 */
import { readFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import Ajv from 'ajv';

const SCHEMA_DIR = resolve(import.meta.dirname, '../current');
let failures = 0;

function assert(condition, message) {
  if (!condition) {
    console.error(`FAIL: ${message}`);
    failures++;
  } else {
    console.log(`PASS: ${message}`);
  }
}

async function loadSchemaWithRefs(ajv, schemaDir, filename) {
  const raw = await readFile(join(schemaDir, filename), 'utf8');
  const schema = JSON.parse(raw);
  return ajv.compile(schema);
}

async function main() {
  const ajv = new Ajv({ allErrors: true, strict: false });

  // Load all enum schemas
  const enumDir = join(SCHEMA_DIR, 'enums');
  for (const file of ['content-type', 'content-format', 'reach', 'mastery-level',
                       'constitutional-layer', 'relationship-type', 'validation-status']) {
    const raw = await readFile(join(enumDir, `${file}.schema.json`), 'utf8');
    ajv.addSchema(JSON.parse(raw));
  }

  // Test content-type enum
  const ctSchema = ajv.getSchema('epr:schema:enum:content-type');
  assert(ctSchema('epic'), 'content-type accepts "epic"');
  assert(ctSchema('scenario'), 'content-type accepts "scenario"');
  assert(!ctSchema('invalid-type'), 'content-type rejects "invalid-type"');
  assert(!ctSchema(''), 'content-type rejects empty string');
  assert(!ctSchema(42), 'content-type rejects number');

  // Test reach enum
  const reachSchema = ajv.getSchema('epr:schema:enum:reach');
  assert(reachSchema('commons'), 'reach accepts "commons"');
  assert(reachSchema('private'), 'reach accepts "private"');
  assert(!reachSchema('public-all'), 'reach rejects "public-all"');

  // Test CreateContentInput
  const validate = await loadSchemaWithRefs(ajv, join(SCHEMA_DIR, 'inputs'), 'create-content-input.schema.json');

  // Valid minimal input
  assert(validate({ id: 'test-1', title: 'Test' }),
    'CreateContentInput accepts minimal valid input');

  // Valid full input
  assert(validate({
    id: 'test-2', title: 'Test', contentType: 'concept',
    contentFormat: 'markdown', reach: 'commons', tags: ['test']
  }), 'CreateContentInput accepts full valid input');

  // Missing required field: id
  assert(!validate({ title: 'Test' }),
    'CreateContentInput rejects missing id');

  // Missing required field: title
  assert(!validate({ id: 'test' }),
    'CreateContentInput rejects missing title');

  // Invalid content type
  assert(!validate({ id: 'test', title: 'Test', contentType: 'bible-verse' }),
    'CreateContentInput rejects non-DNA content type "bible-verse"');

  // Invalid reach
  assert(!validate({ id: 'test', title: 'Test', reach: 'invited' }),
    'CreateContentInput rejects non-DNA reach "invited"');

  console.log(`\n${failures === 0 ? 'ALL TESTS PASSED' : `${failures} TESTS FAILED`}`);
  process.exit(failures > 0 ? 1 : 0);
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
```

**Step 4: Add scripts to root package.json**

Add to `scripts` in root `package.json`:

```json
{
  "schema:test": "node elohim/sdk/schemas/scripts/test-schema.mjs",
  "schema:validate": "node elohim/sdk/schemas/scripts/validate-seeds.mjs"
}
```

**Step 5: Run schema self-test**

```bash
pnpm run schema:test
```

Expected: All PASS except the tests that validate non-DNA content types are rejected (confirming the schema catches real mismatches).

**Step 6: Run seed validation**

```bash
pnpm run schema:validate
```

Expected: Many errors — this is correct! It proves the schema catches the content type divergence. The exact count depends on how many seed files use non-DNA content types (~495 files based on the content type analysis).

**Step 7: Commit**

```bash
git add elohim/sdk/schemas/scripts/ package.json pnpm-lock.yaml
git commit -m "feat(schema): add validation scripts and schema self-tests"
```

---

### Task 6: Resolve Content Type Divergence (User Decision)

**This task requires user input.** Cannot be automated.

**Step 1: Present the divergence report**

Show the user the content type analysis from the plan header. For each non-DNA content type in the seed files, the user must decide:

| Seed Type | Count | Decision Options |
|---|---|---|
| `organization` | 132 | (a) Map to `resource` / (b) Add to DNA / (c) Storage-only |
| `bible-verse` | 111 | (a) Map to `reference` / (b) Add to DNA / (c) Storage-only |
| `book` | 35 | (a) Map to `resource` / (b) Add to DNA / (c) Storage-only |
| `contributor` | 31 | (a) Map to `resource` / (b) Add to DNA / (c) Storage-only |
| `human` | 27 | (a) Already a DNA entry type in imagodei, not content_store / (c) Storage-only |
| `course-module` | 15 | (a) Map to `lesson` / (b) Add to DNA / (c) Storage-only |
| `practice` | 14 | (a) Map to `exercise` / (b) Add to DNA / (c) Storage-only |
| `narrative` | 14 | (a) Map to `epic` / (b) Add to DNA / (c) Storage-only |
| `activity` | 13 | (a) Map to `exercise` / (b) Add to DNA / (c) Storage-only |
| `video` | 12 | Already a content FORMAT, not type. Map to `resource`? |
| `role` | 8 | (a) Map to `resource` / (b) Add to DNA / (c) Storage-only |
| `documentary` | 5 | (a) Map to `resource` / (c) Storage-only |
| `book-chapter` | 4 | (a) Map to `resource` / (c) Storage-only |
| `podcast` | 3 | (a) Map to `resource` / (c) Storage-only |
| `simulation` | 1 | Already in EPR spec appendix. (b) Add to DNA? |

**Step 2: Update content-type enum schema**

Based on user decisions, update `content-type.schema.json` to include approved additional types. Types classified as storage-only get added to the enum with a `_storageOnly` annotation in a separate metadata field.

**Step 3: If any seed files need content type remapping**

Write a migration script to update the `contentType` field in affected seed JSON files:

```bash
# Example: remap bible-verse → reference
find genesis/data/lamad/content -name '*.json' -exec \
  sed -i 's/"contentType": "bible-verse"/"contentType": "reference"/g' {} +
```

**Step 4: Re-run validation**

```bash
pnpm run schema:validate
```

Expected: Error count should drop significantly. Remaining errors are structural issues (not content type mismatches).

**Step 5: Commit**

```bash
git add elohim/sdk/schemas/v1/enums/content-type.schema.json genesis/data/lamad/content/
git commit -m "fix(schema): resolve content type divergence between seed files and DNA"
```

---

### Task 7: Set Up TypeScript Code Generation (Verification Only)

**Files:**
- Create: `elohim/sdk/schemas/scripts/codegen-ts.mjs`
- Create: `elohim/sdk/schemas/generated-ts/` (output directory, gitignored initially)

**Step 1: Install json-schema-to-typescript**

```bash
pnpm add -Dw json-schema-to-typescript
```

**Step 2: Write codegen script**

Create `elohim/sdk/schemas/scripts/codegen-ts.mjs`:

```javascript
#!/usr/bin/env node
/**
 * Generates TypeScript interfaces from protocol schemas.
 * Phase 1: generates to a separate directory for comparison with ts-rs output.
 * Phase 2: will replace ts-rs output in storage-client-ts/src/generated/.
 */
import { readdir, readFile, writeFile, mkdir } from 'node:fs/promises';
import { join, resolve, basename } from 'node:path';
import { compile } from 'json-schema-to-typescript';

const SCHEMA_DIR = resolve(import.meta.dirname, '../current');
const OUTPUT_DIR = resolve(import.meta.dirname, '../generated-ts');

async function generateFromDir(subdir) {
  const dir = join(SCHEMA_DIR, subdir);
  let files;
  try {
    files = (await readdir(dir)).filter(f => f.endsWith('.schema.json'));
  } catch {
    return; // Directory doesn't exist yet
  }

  const outDir = join(OUTPUT_DIR, subdir);
  await mkdir(outDir, { recursive: true });

  for (const file of files) {
    const raw = await readFile(join(dir, file), 'utf8');
    const schema = JSON.parse(raw);
    const name = basename(file, '.schema.json');

    const ts = await compile(schema, schema.title || name, {
      bannerComment: `/* Generated from protocol schema: ${subdir}/${file} — DO NOT EDIT */`,
      additionalProperties: false,
      style: { singleQuote: true, trailingComma: 'all' },
      cwd: dir, // Resolve $ref relative to schema directory
    });

    await writeFile(join(outDir, `${name}.ts`), ts);
    console.log(`Generated: ${subdir}/${name}.ts`);
  }
}

async function main() {
  await mkdir(OUTPUT_DIR, { recursive: true });

  await generateFromDir('enums');
  await generateFromDir('inputs');
  await generateFromDir('views');

  // Generate barrel export
  const dirs = ['enums', 'inputs', 'views'];
  const exports = [];
  for (const dir of dirs) {
    const outDir = join(OUTPUT_DIR, dir);
    try {
      const files = (await readdir(outDir)).filter(f => f.endsWith('.ts'));
      for (const file of files) {
        exports.push(`export * from './${dir}/${basename(file, '.ts')}';`);
      }
    } catch { /* dir doesn't exist */ }
  }
  await writeFile(join(OUTPUT_DIR, 'index.ts'), exports.join('\n') + '\n');

  console.log('\nTypeScript generation complete.');
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
```

**Step 3: Add script to root package.json**

```json
{
  "schema:codegen:ts": "node elohim/sdk/schemas/scripts/codegen-ts.mjs"
}
```

**Step 4: Run codegen**

```bash
pnpm run schema:codegen:ts
```

Expected: TypeScript files generated in `elohim/sdk/schemas/generated-ts/`.

**Step 5: Compare with existing ts-rs output**

Manually compare `elohim/sdk/schemas/generated-ts/views/content-view.ts` against `elohim/sdk/storage-client-ts/src/generated/ContentView.ts`. They should have the same fields and types. Document any differences — these will need resolution in Phase 2.

**Step 6: Commit**

```bash
echo "generated-ts/" >> elohim/sdk/schemas/.gitignore
git add elohim/sdk/schemas/scripts/codegen-ts.mjs elohim/sdk/schemas/.gitignore package.json pnpm-lock.yaml
git commit -m "feat(schema): add TypeScript codegen from protocol schemas (verification mode)"
```

---

### Task 8: Write DNA Conformance Check

**Files:**
- Create: `elohim/sdk/schemas/scripts/check-dna.mjs`

**Step 1: Write DNA check script**

Create `elohim/sdk/schemas/scripts/check-dna.mjs`:

```javascript
#!/usr/bin/env node
/**
 * Verifies that DNA constants match protocol schema enum definitions.
 * Parses Rust source to extract const arrays and compares against schema enums.
 */
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const DNA_FILE = resolve(import.meta.dirname,
  '../../../../elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs');
const SCHEMA_DIR = resolve(import.meta.dirname, '../current/enums');

// Map schema enum files to DNA constant names
const CHECKS = [
  { schema: 'content-type.schema.json', dnaConstant: 'CONTENT_TYPES' },
  { schema: 'content-format.schema.json', dnaConstant: 'CONTENT_FORMATS' },
  { schema: 'reach.schema.json', dnaConstant: 'REACH_LEVELS' },
  { schema: 'mastery-level.schema.json', dnaConstant: 'MASTERY_LEVELS' },
];

function extractRustConstArray(source, constName) {
  // Match: pub const NAME: [&str; N] = [ "val1", "val2", ... ];
  const pattern = new RegExp(
    `pub\\s+const\\s+${constName}\\s*:\\s*\\[&str;\\s*\\d+\\]\\s*=\\s*\\[([^\\]]+)\\]`,
    's'
  );
  const match = source.match(pattern);
  if (!match) return null;
  return match[1]
    .split(',')
    .map(s => s.trim().replace(/^"/, '').replace(/"$/, ''))
    .filter(s => s.length > 0);
}

async function main() {
  const dnaSource = await readFile(DNA_FILE, 'utf8');
  let failures = 0;

  for (const { schema, dnaConstant } of CHECKS) {
    const schemaRaw = await readFile(resolve(SCHEMA_DIR, schema), 'utf8');
    const schemaEnum = JSON.parse(schemaRaw).enum;
    const dnaValues = extractRustConstArray(dnaSource, dnaConstant);

    if (!dnaValues) {
      console.error(`FAIL: Could not find ${dnaConstant} in DNA source`);
      failures++;
      continue;
    }

    // Check that every DNA value is in the schema
    for (const val of dnaValues) {
      if (!schemaEnum.includes(val)) {
        console.error(`FAIL: DNA ${dnaConstant} has "${val}" but schema ${schema} does not`);
        failures++;
      }
    }

    // Check that every schema value marked as DNA is in the DNA
    // (storage-only types are allowed to be absent from DNA)
    const schemaJson = JSON.parse(schemaRaw);
    if (schemaJson._dna) {
      for (const val of schemaEnum) {
        if (!dnaValues.includes(val)) {
          // Only warn if schema doesn't have _storageOnly annotation for this value
          const storageOnly = schemaJson._storageOnly || [];
          if (!storageOnly.includes(val)) {
            console.error(`FAIL: Schema ${schema} has "${val}" but DNA ${dnaConstant} does not (and not marked _storageOnly)`);
            failures++;
          }
        }
      }
    }

    if (failures === 0) {
      console.log(`PASS: ${schema} ↔ ${dnaConstant} (${dnaValues.length} values)`);
    }
  }

  console.log(`\n${failures === 0 ? 'ALL DNA CHECKS PASSED' : `${failures} DNA CHECKS FAILED`}`);
  process.exit(failures > 0 ? 1 : 0);
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
```

**Step 2: Add script to root package.json**

```json
{
  "schema:check-dna": "node elohim/sdk/schemas/scripts/check-dna.mjs"
}
```

**Step 3: Run DNA check**

```bash
pnpm run schema:check-dna
```

Expected: All PASS (assuming the enum schemas were written correctly from the DNA constants).

**Step 4: Commit**

```bash
git add elohim/sdk/schemas/scripts/check-dna.mjs package.json
git commit -m "feat(schema): add DNA conformance check script"
```

---

### Task 9: Wire Husky Pre-Push Hook

**Files:**
- Modify: `.husky/pre-push` — add schema validation for genesis changes

**Step 1: Read current pre-push hook**

Read: `.husky/pre-push` — understand the existing project detection pattern.

**Step 2: Add schema validation to genesis section**

Find the section that handles `genesis/` changes and add schema validation. Add after the existing genesis gate logic:

```bash
# Schema validation for seed data
if echo "$CHANGED_PROJECTS" | grep -q "genesis"; then
  echo "🔍 Validating seed data against protocol schemas..."
  pnpm run schema:validate || {
    echo "❌ Seed data does not conform to protocol schema."
    echo "   Run 'pnpm run schema:validate' to see details."
    exit 1
  }
fi
```

**Step 3: Add schema check for DNA changes**

Find the section that handles `elohim/holochain/` changes and add DNA conformance check:

```bash
# DNA conformance check
if echo "$CHANGED_PROJECTS" | grep -q "elohim/holochain"; then
  echo "🔍 Checking DNA constants against protocol schemas..."
  pnpm run schema:check-dna || {
    echo "❌ DNA constants diverged from protocol schema."
    echo "   Run 'pnpm run schema:check-dna' to see details."
    exit 1
  }
fi
```

**Step 4: Test the hook**

```bash
# Verify hook runs without error on current state
.husky/pre-push
```

**Step 5: Commit**

```bash
git add .husky/pre-push
git commit -m "feat(schema): wire schema validation into husky pre-push hook"
```

---

### Task 10: Add VS Code Schema Association

**Files:**
- Modify or create: `.vscode/settings.json` — add JSON schema associations

**Step 1: Add schema association for seed files**

Add to `.vscode/settings.json`:

```json
{
  "json.schemas": [
    {
      "fileMatch": ["genesis/data/lamad/content/*.json"],
      "url": "./elohim/sdk/schemas/current/inputs/create-content-input.schema.json"
    }
  ]
}
```

**Step 2: Verify in VS Code**

Open any seed file in `genesis/data/lamad/content/`. VS Code should show validation errors for invalid content types, missing required fields, etc.

**Step 3: Commit**

```bash
git add .vscode/settings.json
git commit -m "feat(schema): add VS Code schema association for seed files"
```

---

### Task 11: Final Integration Verification

**Step 1: Run all schema commands**

```bash
pnpm run schema:test         # Schema self-tests pass
pnpm run schema:check-dna    # DNA conformance passes
pnpm run schema:validate     # Seed validation (expect results based on Task 6 resolution)
pnpm run schema:codegen:ts   # TypeScript generation works
```

**Step 2: Verify VS Code feedback**

1. Open `genesis/data/lamad/content/` and pick any seed file
2. Change `contentType` to `"invalid-garbage"`
3. Confirm red squiggle appears
4. Revert the change

**Step 3: Verify husky hook**

```bash
# Stage a deliberately invalid seed file and attempt push
echo '{"id":"test","title":"Test","contentType":"invalid"}' > /tmp/test-invalid.json
cp /tmp/test-invalid.json genesis/data/lamad/content/test-invalid.json
git add genesis/data/lamad/content/test-invalid.json
git stash  # Don't actually commit this
```

**Step 4: Final commit with updated CLAUDE.md reference**

Add a note to the root `CLAUDE.md` under the Build & Test Commands section:

```markdown
### Protocol Schema Validation
```bash
pnpm run schema:test        # Schema self-tests
pnpm run schema:validate    # Validate seed JSON against schemas
pnpm run schema:check-dna   # Verify DNA constants match schema enums
pnpm run schema:codegen:ts  # Generate TypeScript from schemas (verification mode)
```
```

```bash
git add CLAUDE.md
git commit -m "docs: add protocol schema commands to CLAUDE.md"
```

---

## Phase 2 Preview (Next Sprint)

After Phase 1 is validated, Phase 2 replaces generated code:

1. **Replace ts-rs with json-schema-to-typescript** — move generated-ts/ output to storage-client-ts/src/generated/, remove ts-rs from Cargo.toml
2. **Set up typify for Rust codegen** — generate view/input structs from schemas, replace hand-written structs in views.rs
3. **Replace ContentNode** — elohim-service imports from schema-generated types instead of hand-written models
4. **Update seeder** — remove hand-written validators, use AJV against schemas
5. **Add to CI pipeline** — schema validation as explicit stage in genesis/Jenkinsfile

## Phase 3 Preview (Future Sprint)

1. **CID computation** — content-address schema versions
2. **Migration chain** — implement `From<V1> for V2` pattern with migration manifests
3. **`schema_version` migration** — change from u32 to CID string in storage
4. **Multiple schema version support** — storage layer routes by schema CID

## Phase 4 Preview (Future Sprint)

1. **Schemas as DHT content** — publish schemas via existing Content entry type
2. **P2P schema resolution** — fetch unknown schemas by CID from DHT
3. **Schema negotiation** — peers declare supported schema versions
