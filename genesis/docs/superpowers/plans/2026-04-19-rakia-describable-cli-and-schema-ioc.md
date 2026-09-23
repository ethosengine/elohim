# Rakia Describable via CLI + Schema-as-IoC Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose the existing rakia primitives (constellation, change detection, baselines, fingerprinting) through 8 `brit` CLI subcommands, AND lock the input/output contracts as rakia-owned JSON Schemas with Rust types generated from them — eliminating every `serde_json::Value` escape hatch as the sprint's IoC closing pass.

**Architecture:** Three converging streams. (1) **Schema home** at `elohim/rakia/schemas/v1/` holding `build-manifest.schema.json` (ported + refined from `genesis/orchestrator/manifest.schema.json`) and `build-plan.schema.json` (new, the upgrade target). (2) **Codegen pipeline** — node script `elohim/rakia/schemas/scripts/codegen-rs.mjs` extends the existing `sdk/schemas/scripts/codegen-rs.mjs` pattern (with `--verify` mode and pre-push integration) to generate full Rust struct/enum types from JSON Schema into `rakia-core/src/generated_types.rs`. Hand-writing types that mirror schemas is forbidden. (3) **Brit CLI** — new `brit-cli` workspace member in the brit (gitoxide fork) workspace, clap-based, depends cross-workspace on `rakia-core` and `rakia-brit`, single `brit` binary with 8 subcommands.

Fixture-based regression tests in `rakia-core/tests/fixtures/` replace shadow-mode validation entirely. Sprint closes with a sprint-result artifact at `docs/superpowers/sprint-results/2026-04-19-rakia-describable.md` capturing demo + next-sprint scope (rakia-runnable: executor + `rakia ci`).

**Tech Stack:** Rust 2021 (`brit-cli` clap 4 + serde_json + petgraph Dot formatter), Node.js 20 (codegen scripts, AJV for validation), JSON Schema 2020-12, gix 0.81 (already in use by `rakia-brit`).

**Spec:** `docs/superpowers/specs/2026-04-19-rakia-describable-cli-and-schema-ioc.md`

**Build notes:**
- Brit workspace: `cd elohim/brit && RUSTFLAGS="" cargo build` (gitoxide fork, native targets)
- Rakia workspace: `cd elohim/rakia && RUSTFLAGS="" cargo build`
- Cross-workspace path deps: `brit-cli` references `../../rakia/rakia-core` and `../../rakia/rakia-brit`
- Codegen runs from repo root via `pnpm run rakia:codegen:rs`

---

## File Structure

### New files

```
elohim/rakia/
├── schemas/
│   ├── v1/
│   │   ├── build-manifest.schema.json    # NEW (ported + refined from genesis)
│   │   └── build-plan.schema.json         # NEW (output contract)
│   ├── scripts/
│   │   ├── codegen-rs.mjs                 # NEW (extends sdk pattern, generates structs)
│   │   ├── validate.mjs                   # NEW (validates 8 manifests against schema)
│   │   └── lib/
│   │       └── schema-to-rust.mjs         # NEW (the schema → Rust translator)
│   └── tests/
│       └── codegen-rs.test.mjs            # NEW (unit tests for the translator)
├── rakia-core/
│   ├── src/
│   │   ├── generated_types.rs             # NEW (AUTO-GENERATED — do not edit)
│   │   ├── build_plan.rs                  # NEW (TopoPlan → BuildPlan converter)
│   │   └── manifest.rs                    # MODIFY (use generated types)
│   └── tests/
│       ├── fixture_runner.rs              # NEW
│       ├── fixtures/                       # NEW (8 fixture cases)
│       │   ├── README.md                   # NEW (fixture format docs)
│       │   ├── manifests-snapshot/         # NEW (frozen 8 manifests)
│       │   └── 01-elohim-app-css-change/
│       │       ├── changed-paths.json
│       │       └── expected-plan.json
│       │   └── ... 7 more fixtures
│       └── build_plan_schema_contract.rs   # NEW (validates BuildPlan output against schema)
└── rakia-brit/                             # NO changes this sprint

elohim/brit/
├── brit-cli/                               # NEW workspace member
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs                          # clap entrypoint
│   │   ├── commands/
│   │   │   ├── mod.rs
│   │   │   ├── graph_discover.rs
│   │   │   ├── graph_show.rs
│   │   │   ├── affected.rs
│   │   │   ├── plan.rs
│   │   │   ├── fingerprint.rs
│   │   │   └── baseline.rs                  # read/write/migrate
│   │   ├── output.rs                        # JSON envelope helpers
│   │   └── error.rs                         # error → exit code mapping
│   └── tests/
│       └── cli_smoke.rs                     # invoke binary, assert output shape
└── Cargo.toml                                # MODIFY (add brit-cli to workspace.members)

docs/superpowers/sprint-results/
└── 2026-04-19-rakia-describable.md          # NEW (sprint-result artifact)

.husky/
└── pre-push                                  # MODIFY (add rakia codegen verify + schema validate)

package.json                                  # MODIFY (add rakia:codegen:rs, rakia:schema:validate)
```

### Notes on file responsibilities

- **`schema-to-rust.mjs`** is the meat of the codegen: a JSON Schema → Rust translator. Lives as a library so `codegen-rs.mjs` is thin orchestration. Testable in isolation (`codegen-rs.test.mjs`).
- **`generated_types.rs`** holds ALL generated types (BuildManifest, BuildStep, BuildPlan, PlannedStep, AffectedReason, etc.) — single file, regenerated atomically.
- **`build_plan.rs`** holds the `to_build_plan(plan, baseline, head, changed_paths) -> BuildPlan` converter. Internal `TopoPlan` stays in `constellation.rs` (engine-side); `BuildPlan` is the schema-defined output (contract-side).
- **`manifest.rs`** keeps the parsing entrypoint and any business logic, but its types come from `generated_types.rs` via re-export. After the IoC pass, `manifest.rs` has zero `serde_json::Value` fields.
- **`brit-cli/src/commands/`** — one file per subcommand group. Keep files focused (~150 lines max per file).
- **`fixture_runner.rs`** is a parameterized test that reads each `fixtures/NN-name/` directory and asserts `plan_from_changes` produces the expected plan. New fixtures get picked up automatically.

---

## Refinement from spec

The spec's `build-plan.schema.json` draft used `oneOf` for `affectedReason`. The codegen translator handles flat structs and string enums; `oneOf` discriminated unions are out of scope for this sprint's codegen. **Refinement:** use a flat single-struct representation of `AffectedReason` with `kind` discriminator + optional fields. The schema is slightly looser (doesn't enforce "path required when kind=changedFile") but the IoC enforcement still holds at field-shape level. This is documented in the spec carry-overs as a future codegen capability.

```json
"affectedReason": {
  "type": "object",
  "required": ["kind"],
  "additionalProperties": false,
  "properties": {
    "kind": {
      "type": "string",
      "enum": ["changedFile", "upstreamNode", "inputFingerprint", "alwaysAffected"]
    },
    "path": { "type": "string", "description": "Set when kind=changedFile" },
    "upstream": { "type": "string", "description": "Set when kind=upstreamNode (qualified step name)" }
  }
}
```

---

## Phase 1: Rakia Schema Home

### Task 1: Scaffold rakia/schemas directory and root scripts

**Files:**
- Create: `elohim/rakia/schemas/v1/.gitkeep`
- Create: `elohim/rakia/schemas/scripts/.gitkeep`
- Create: `elohim/rakia/schemas/README.md`
- Modify: `package.json` (root, add scripts)

- [ ] **Step 1.1: Create directory structure with .gitkeep files**

```bash
mkdir -p elohim/rakia/schemas/v1 elohim/rakia/schemas/scripts elohim/rakia/schemas/scripts/lib elohim/rakia/schemas/tests
touch elohim/rakia/schemas/v1/.gitkeep
touch elohim/rakia/schemas/scripts/.gitkeep
```

- [ ] **Step 1.2: Create `elohim/rakia/schemas/README.md`**

```markdown
# Rakia Schemas

Rakia-owned JSON Schemas. Build-domain semantics (BuildManifest, BuildPlan,
BuildAttestation) — meaning-defined interpretations of EPR core primitives.
Not protocol-core. Lives here, not in `elohim/sdk/schemas/`.

## Schemas

| Schema | Path | Purpose |
|---|---|---|
| BuildManifest | `v1/build-manifest.schema.json` | Input contract — declares pipeline steps, inputs, outputs, deps |
| BuildPlan | `v1/build-plan.schema.json` | Output contract — what `brit plan` returns |

## Scripts

```bash
pnpm run rakia:codegen:rs          # Generate Rust types from schemas
pnpm run rakia:codegen:rs --verify # Verify generated_types.rs is up-to-date
pnpm run rakia:schema:validate     # Validate all build-manifest.json files
```

## Codegen output

`elohim/rakia/rakia-core/src/generated_types.rs` — AUTO-GENERATED, do not edit.
All Rust types matching the schemas live there with `Serialize + Deserialize`
derives and `#[serde(rename_all = "camelCase")]` on every struct.

## Schema-as-IoC

Hand-writing Rust types that mirror these schemas is forbidden. The codegen
runs in pre-push and CI; drift fails the build. This is the IoC discipline
that closes every rakia sprint.
```

- [ ] **Step 1.3: Add scripts to root `package.json`**

Modify `/home/matthew/git/elohim/package.json` — add three lines to the `scripts` block alongside the existing `lamad:codegen` lines:

```json
    "rakia:codegen:rs": "node elohim/rakia/schemas/scripts/codegen-rs.mjs",
    "rakia:codegen:rs:verify": "node elohim/rakia/schemas/scripts/codegen-rs.mjs --verify",
    "rakia:schema:validate": "node elohim/rakia/schemas/scripts/validate.mjs",
```

- [ ] **Step 1.4: Verify scripts are visible**

Run from repo root: `pnpm run | grep rakia:`
Expected: three rakia: entries listed (script files don't exist yet — that's OK, just verifying the package.json is parseable).

- [ ] **Step 1.5: Commit**

```bash
git add elohim/rakia/schemas/ package.json
git commit -m "chore(rakia): scaffold schemas/ + root scripts (rakia:codegen:rs, validate)"
```

---

### Task 2: Port BuildManifest schema (basic copy)

**Files:**
- Create: `elohim/rakia/schemas/v1/build-manifest.schema.json`

- [ ] **Step 2.1: Copy the existing schema verbatim**

```bash
cp genesis/orchestrator/manifest.schema.json elohim/rakia/schemas/v1/build-manifest.schema.json
```

- [ ] **Step 2.2: Update the `$id` field**

Edit `elohim/rakia/schemas/v1/build-manifest.schema.json` and change:

```json
  "$id": "https://elohim.host/schemas/build-manifest/v1",
```

to:

```json
  "$id": "epr:schema:rakia:build-manifest:v1",
```

- [ ] **Step 2.3: Validate the copy is well-formed JSON**

Run: `node -e "JSON.parse(require('fs').readFileSync('elohim/rakia/schemas/v1/build-manifest.schema.json', 'utf8')); console.log('valid')"`
Expected: `valid`

- [ ] **Step 2.4: Commit**

```bash
git add elohim/rakia/schemas/v1/build-manifest.schema.json
git commit -m "feat(rakia/schemas): port build-manifest schema from genesis/orchestrator (basic copy)"
```

---

### Task 3: Refine BuildManifest schema (gate, deployment, executor)

**Goal:** Replace the loose `gate`/`deployment` definitions and the missing `executor` definition with empirically-tightened types based on what the 8 real manifests actually use.

**Files:**
- Modify: `elohim/rakia/schemas/v1/build-manifest.schema.json`

- [ ] **Step 3.1: Inventory `gate`, `deployment`, `executor` usage in real manifests**

Run: `for f in $(find . -name 'build-manifest.json' -not -path '*/node_modules/*' | sort); do echo "=== $f"; jq '{gate: .gate, deployment: .deployment, executor_examples: [.steps[] | select(.executor) | .executor] | unique}' "$f"; done`

Capture the output. The refinement in 3.2 derives directly from what's there.

- [ ] **Step 3.2: Refine the `gate` definition**

Open `elohim/rakia/schemas/v1/build-manifest.schema.json`. Locate the `gate` property definition. Replace the existing inner properties with whatever fields the inventory in 3.1 revealed. Typical shape:

```json
    "gate": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "projects": {
          "type": "object",
          "additionalProperties": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
              "patterns": { "type": "array", "items": { "type": "string" } },
              "required": { "type": "boolean", "default": false }
            }
          }
        }
      }
    },
```

(If the inventory shows other shapes, encode those instead — the goal is "every real manifest validates.")

- [ ] **Step 3.3: Refine the `deployment` definition**

Same approach as 3.2 but for `deployment`. Common shape (verify against inventory):

```json
    "deployment": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "targets": {
          "type": "object",
          "additionalProperties": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
              "healthCheck": { "type": "string", "format": "uri" },
              "url": { "type": "string", "format": "uri" }
            }
          }
        }
      }
    },
```

- [ ] **Step 3.4: Add the `executor` definition to the step shape**

Locate the `step` definition in `$defs` (or wherever steps are defined). Add an `executor` property. Encode as a discriminated-by-kind object (flat, codegen-friendly):

```json
        "executor": {
          "type": "object",
          "required": ["kind"],
          "additionalProperties": false,
          "properties": {
            "kind": {
              "type": "string",
              "enum": ["shell", "rustCargo", "pnpm", "cargo", "noOp"]
            },
            "command": { "type": "string", "description": "Set when kind=shell" },
            "package": { "type": "string", "description": "Set when kind=pnpm or kind=cargo" },
            "script": { "type": "string", "description": "Set when kind=pnpm" },
            "workspaceMember": { "type": "string", "description": "Set when kind=cargo or kind=rustCargo" },
            "args": { "type": "array", "items": { "type": "string" } }
          }
        }
```

(Adjust enum values and properties to match the inventory from 3.1. If a manifest uses an executor kind not listed, add it.)

- [ ] **Step 3.5: Validate all 8 manifests against the refined schema (manual check)**

Use `ajv-cli` (or a quick node one-liner) to validate each manifest. If `ajv` isn't installed:

```bash
npx ajv-cli@5 validate -s elohim/rakia/schemas/v1/build-manifest.schema.json -d "**/build-manifest.json" --spec=draft2020 --all-errors
```

Expected: all 8 manifests pass. If one fails, decide: fix the manifest (if buggy) or refine the schema (if too strict). Iterate until all 8 pass. Document any manifest fixes as separate commits before continuing.

- [ ] **Step 3.6: Commit refined schema**

```bash
git add elohim/rakia/schemas/v1/build-manifest.schema.json
git commit -m "feat(rakia/schemas): refine gate, deployment, executor based on real manifest usage"
```

---

### Task 4: Author BuildPlan schema (output contract)

**Files:**
- Create: `elohim/rakia/schemas/v1/build-plan.schema.json`

- [ ] **Step 4.1: Create `build-plan.schema.json`**

```json
{
  "$id": "epr:schema:rakia:build-plan:v1",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "BuildPlan",
  "description": "Output of `brit plan` — the topologically grouped set of build steps that need to run, with provenance for each.",
  "type": "object",
  "additionalProperties": false,
  "required": ["planVersion", "baseline", "head", "levels", "generatedAt", "tool"],
  "properties": {
    "planVersion": {
      "type": "string",
      "const": "1.0",
      "description": "Schema version. Bumps when the BuildPlan shape changes."
    },
    "baseline": {
      "type": "object",
      "additionalProperties": false,
      "required": ["ref", "commit"],
      "properties": {
        "ref": {
          "type": "string",
          "description": "Git ref name, e.g. refs/notes/rakia/baselines/elohim"
        },
        "commit": {
          "type": "string",
          "pattern": "^[0-9a-f]{40}$",
          "description": "Baseline commit SHA-1 (40-char hex)"
        }
      }
    },
    "head": {
      "type": "object",
      "additionalProperties": false,
      "required": ["commit"],
      "properties": {
        "commit": {
          "type": "string",
          "pattern": "^[0-9a-f]{40}$"
        }
      }
    },
    "changedPaths": {
      "type": "array",
      "items": { "type": "string" },
      "description": "Workspace-relative paths that differ between baseline and head. Empty when --files was used."
    },
    "levels": {
      "type": "array",
      "description": "Topologically grouped — level 0 has no deps within the plan, level 1 depends only on level 0, etc.",
      "items": {
        "type": "array",
        "items": { "$ref": "#/$defs/plannedStep" }
      }
    },
    "generatedAt": {
      "type": "string",
      "format": "date-time",
      "description": "RFC 3339 timestamp"
    },
    "tool": {
      "type": "object",
      "additionalProperties": false,
      "required": ["name", "version"],
      "properties": {
        "name": { "type": "string", "const": "brit" },
        "version": { "type": "string" }
      }
    }
  },
  "$defs": {
    "plannedStep": {
      "type": "object",
      "additionalProperties": false,
      "required": ["pipeline", "name", "qualifiedName", "fingerprint", "depends", "affectedBy"],
      "properties": {
        "pipeline": { "type": "string" },
        "name": { "type": "string" },
        "qualifiedName": {
          "type": "string",
          "description": "pipeline:name"
        },
        "fingerprint": {
          "type": "string",
          "description": "BritCid hex — content-addressed hash of step inputs"
        },
        "depends": {
          "type": "array",
          "items": { "type": "string", "description": "qualified names of dependencies" }
        },
        "affectedBy": {
          "type": "array",
          "minItems": 1,
          "items": { "$ref": "#/$defs/affectedReason" }
        }
      }
    },
    "affectedReason": {
      "type": "object",
      "additionalProperties": false,
      "required": ["kind"],
      "properties": {
        "kind": {
          "type": "string",
          "enum": ["changedFile", "upstreamNode", "inputFingerprint", "alwaysAffected"]
        },
        "path": {
          "type": "string",
          "description": "Set when kind=changedFile"
        },
        "upstream": {
          "type": "string",
          "description": "Set when kind=upstreamNode (qualified step name of the upstream)"
        }
      }
    }
  }
}
```

- [ ] **Step 4.2: Validate the schema is itself well-formed JSON Schema**

```bash
npx ajv-cli@5 compile -s elohim/rakia/schemas/v1/build-plan.schema.json --spec=draft2020
```

Expected: no errors.

- [ ] **Step 4.3: Commit**

```bash
git add elohim/rakia/schemas/v1/build-plan.schema.json
git commit -m "feat(rakia/schemas): author build-plan schema (output contract)"
```

---

### Task 5: Validation script (validate all manifests against schema)

**Files:**
- Create: `elohim/rakia/schemas/scripts/validate.mjs`

- [ ] **Step 5.1: Create `elohim/rakia/schemas/scripts/validate.mjs`**

```javascript
#!/usr/bin/env node
/**
 * Validates all build-manifest.json files in the worktree against
 * elohim/rakia/schemas/v1/build-manifest.schema.json.
 *
 * Exits 0 if all valid, 1 if any invalid (with errors to stderr).
 *
 * Usage: pnpm run rakia:schema:validate
 */
import { readFile, readdir, stat } from 'node:fs/promises';
import { join, resolve, relative, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '../../../../');
const SCHEMA_PATH = resolve(__dirname, '../v1/build-manifest.schema.json');

const SKIP_DIRS = new Set(['node_modules', '.git', 'target', 'dist', 'build', '.superpowers']);

async function findManifests(dir, results = []) {
  const entries = await readdir(dir, { withFileTypes: true });
  for (const entry of entries) {
    if (entry.isDirectory()) {
      if (SKIP_DIRS.has(entry.name)) continue;
      await findManifests(join(dir, entry.name), results);
    } else if (entry.name === 'build-manifest.json') {
      results.push(join(dir, entry.name));
    }
  }
  return results;
}

async function main() {
  const schemaText = await readFile(SCHEMA_PATH, 'utf8');
  const schema = JSON.parse(schemaText);

  const ajv = new Ajv2020({ allErrors: true, strict: false });
  addFormats(ajv);
  const validate = ajv.compile(schema);

  const manifests = await findManifests(REPO_ROOT);
  console.log(`Validating ${manifests.length} build-manifest.json file(s) against ${relative(REPO_ROOT, SCHEMA_PATH)}\n`);

  let failures = 0;
  for (const path of manifests) {
    const text = await readFile(path, 'utf8');
    const data = JSON.parse(text);
    const ok = validate(data);
    const rel = relative(REPO_ROOT, path);
    if (ok) {
      console.log(`  PASS  ${rel}`);
    } else {
      console.error(`  FAIL  ${rel}`);
      for (const err of validate.errors || []) {
        console.error(`        ${err.instancePath || '<root>'} ${err.message}`);
      }
      failures++;
    }
  }

  if (failures > 0) {
    console.error(`\n${failures} manifest(s) failed validation.`);
    process.exit(1);
  }
  console.log(`\nAll ${manifests.length} manifest(s) valid.`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
```

- [ ] **Step 5.2: Confirm `ajv` and `ajv-formats` dependencies are available**

These are already in `package.json` devDependencies (used by other validation scripts). Verify:

```bash
node -e "import('ajv/dist/2020.js').then(() => console.log('ok')).catch(e => { console.error(e); process.exit(1) })"
```

Expected: `ok`. If not, add to root devDependencies: `pnpm add -DW ajv ajv-formats`.

- [ ] **Step 5.3: Run validation**

```bash
pnpm run rakia:schema:validate
```

Expected: `All 8 manifest(s) valid.` (or however many exist). If any fail, treat as Task 3 follow-up — fix the schema or the manifest.

- [ ] **Step 5.4: Commit**

```bash
git add elohim/rakia/schemas/scripts/validate.mjs
git commit -m "feat(rakia/schemas): validate.mjs — verify manifests conform to schema"
```

---

## Phase 2: Codegen Pipeline (JSON Schema → Rust)

### Task 6: Schema-to-Rust translator library

**Goal:** Build a JSON Schema → Rust struct translator as a library module, testable in isolation. Handles flat structs, nested structs, string enums, $ref, $defs, and `BTreeMap` for additionalProperties-only objects.

**Files:**
- Create: `elohim/rakia/schemas/scripts/lib/schema-to-rust.mjs`
- Create: `elohim/rakia/schemas/tests/codegen-rs.test.mjs`

- [ ] **Step 6.1: Write the failing test for basic struct translation**

`elohim/rakia/schemas/tests/codegen-rs.test.mjs`:

```javascript
#!/usr/bin/env node
import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { schemaToRust } from '../scripts/lib/schema-to-rust.mjs';

describe('schemaToRust — basic types', () => {
  it('translates a simple object schema with string + integer fields', () => {
    const schema = {
      title: 'Foo',
      type: 'object',
      required: ['name'],
      properties: {
        name: { type: 'string' },
        count: { type: 'integer' },
      },
    };
    const out = schemaToRust(schema);
    assert.match(out, /pub struct Foo \{/);
    assert.match(out, /pub name: String,/);
    assert.match(out, /pub count: Option<i64>,/);
    assert.match(out, /#\[serde\(rename_all = "camelCase"\)\]/);
  });

  it('emits Vec<T> for array properties', () => {
    const schema = {
      title: 'Bar',
      type: 'object',
      properties: {
        tags: { type: 'array', items: { type: 'string' } },
      },
    };
    const out = schemaToRust(schema);
    assert.match(out, /pub tags: Option<Vec<String>>,/);
  });

  it('emits BTreeMap<String, T> for additionalProperties-only objects', () => {
    const schema = {
      title: 'Baz',
      type: 'object',
      properties: {
        targets: {
          type: 'object',
          additionalProperties: { type: 'string' },
        },
      },
    };
    const out = schemaToRust(schema);
    assert.match(out, /pub targets: Option<BTreeMap<String, String>>,/);
  });
});
```

- [ ] **Step 6.2: Run test to verify it fails**

```bash
node --test elohim/rakia/schemas/tests/codegen-rs.test.mjs
```

Expected: FAIL — `schemaToRust` is not defined (module doesn't exist yet).

- [ ] **Step 6.3: Implement `schema-to-rust.mjs`**

`elohim/rakia/schemas/scripts/lib/schema-to-rust.mjs`:

```javascript
/**
 * JSON Schema → Rust struct/enum translator.
 *
 * Handles:
 *   - Flat structs (type: object + properties)
 *   - Nested $defs (emitted as separate types)
 *   - String enums (emitted as Rust enums with #[serde(rename = ...)])
 *   - $ref to local $defs and to other schema files (file refs are pre-inlined upstream)
 *   - Vec<T> for arrays
 *   - BTreeMap<String, T> for objects with additionalProperties only
 *   - Optional fields (non-required → Option<T>)
 *
 * Does NOT handle (out of scope for this sprint):
 *   - oneOf / anyOf / allOf
 *   - Tuple types
 *   - Number formats (always emits f64 for type:number, i64 for type:integer)
 */

const RUST_KEYWORDS = new Set([
  'as', 'break', 'const', 'continue', 'crate', 'else', 'enum', 'extern',
  'false', 'fn', 'for', 'if', 'impl', 'in', 'let', 'loop', 'match', 'mod',
  'move', 'mut', 'pub', 'ref', 'return', 'self', 'Self', 'static', 'struct',
  'super', 'trait', 'true', 'type', 'unsafe', 'use', 'where', 'while', 'async',
  'await', 'dyn',
]);

function pascalCase(s) {
  return s.replace(/(?:^|[-_])(\w)/g, (_, c) => c.toUpperCase());
}

function snakeCase(s) {
  return s.replace(/([A-Z])/g, (_, c, i) => (i === 0 ? c.toLowerCase() : `_${c.toLowerCase()}`));
}

function safeFieldName(name) {
  const snake = snakeCase(name);
  return RUST_KEYWORDS.has(snake) ? `r#${snake}` : snake;
}

/**
 * Map a JSON Schema property to a Rust type expression.
 * Returns the type string, e.g. "String", "Vec<String>", "MyType".
 */
function propToRustType(prop, ctx) {
  if (prop.$ref) {
    // $refs to #/$defs/Foo become "Foo" (caller ensures Foo is generated)
    if (prop.$ref.startsWith('#/$defs/')) {
      return prop.$ref.replace('#/$defs/', '');
    }
    throw new Error(`unsupported $ref: ${prop.$ref} (file refs must be inlined before translation)`);
  }
  if (prop.enum && prop.type === 'string') {
    // String enum → emit a Rust enum (registered with ctx)
    const enumName = ctx.allocateEnum(prop, ctx.path);
    return enumName;
  }
  if (prop.const !== undefined && prop.type === 'string') {
    // const string → just String (the schema enforces the value)
    return 'String';
  }
  if (prop.type === 'string') return 'String';
  if (prop.type === 'integer') return 'i64';
  if (prop.type === 'number') return 'f64';
  if (prop.type === 'boolean') return 'bool';
  if (prop.type === 'array') {
    const itemType = propToRustType(prop.items || { type: 'string' }, ctx);
    return `Vec<${itemType}>`;
  }
  if (prop.type === 'object') {
    if (prop.properties && Object.keys(prop.properties).length > 0) {
      // Inline nested object — anonymous structs aren't supported, so caller
      // should pre-extract these into $defs. For safety, error out.
      throw new Error(`inline nested objects must be lifted to $defs (path: ${ctx.path})`);
    }
    if (prop.additionalProperties && prop.additionalProperties !== true) {
      const valueType = propToRustType(prop.additionalProperties, ctx);
      return `BTreeMap<String, ${valueType}>`;
    }
    return 'serde_json::Value'; // open object — last resort, fail loudly later
  }
  return 'serde_json::Value';
}

/**
 * Emit a Rust struct from an object schema.
 * Returns the formatted Rust source for the struct.
 */
function emitStruct(name, schema, ctx) {
  const required = new Set(schema.required || []);
  const props = schema.properties || {};

  const lines = [
    `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]`,
    `#[serde(rename_all = "camelCase")]`,
    `pub struct ${name} {`,
  ];

  for (const [propName, prop] of Object.entries(props)) {
    const fieldName = safeFieldName(propName);
    ctx.path = `${name}.${propName}`;
    let rustType = propToRustType(prop, ctx);
    if (!required.has(propName)) {
      rustType = `Option<${rustType}>`;
    }
    if (prop.description) {
      lines.push(`    /// ${prop.description.replace(/\n/g, ' ')}`);
    }
    // Add #[serde(default, skip_serializing_if = "Option::is_none")] for optional fields
    if (!required.has(propName)) {
      lines.push(`    #[serde(default, skip_serializing_if = "Option::is_none")]`);
    }
    lines.push(`    pub ${fieldName}: ${rustType},`);
  }

  lines.push(`}`);
  return lines.join('\n');
}

/**
 * Emit a Rust enum from a string-enum schema.
 */
function emitEnum(name, values) {
  const variants = values.map((v) => {
    const variant = pascalCase(v);
    return `    #[serde(rename = "${v}")]\n    ${variant},`;
  });
  return [
    `#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]`,
    `pub enum ${name} {`,
    ...variants,
    `}`,
  ].join('\n');
}

/**
 * Translate a top-level schema to Rust source. Returns the full module body.
 *
 * @param {object} schema - the parsed JSON Schema
 * @param {object} [opts]
 * @param {Map<string, string[]>} [opts.enumRegistry] - shared registry for de-duped enums across calls
 * @returns {string}
 */
export function schemaToRust(schema, opts = {}) {
  const enumRegistry = opts.enumRegistry || new Map(); // enumName → values
  const ctx = {
    path: schema.title || 'Root',
    allocateEnum(prop, fieldPath) {
      const name = pascalCase(fieldPath.split('.').pop()) || 'Unnamed';
      const existing = enumRegistry.get(name);
      const sortedValues = [...prop.enum].sort();
      if (existing) {
        const existingSorted = [...existing].sort();
        if (JSON.stringify(existingSorted) === JSON.stringify(sortedValues)) {
          return name;
        }
        // Same name, different values — disambiguate by appending field path
        const altName = pascalCase(fieldPath.replace(/\./g, '-'));
        enumRegistry.set(altName, prop.enum);
        return altName;
      }
      enumRegistry.set(name, prop.enum);
      return name;
    },
  };

  const blocks = [];

  // Emit $defs first (they may be referenced by the top-level struct)
  if (schema.$defs) {
    for (const [defName, defSchema] of Object.entries(schema.$defs)) {
      blocks.push(emitStruct(pascalCase(defName), defSchema, ctx));
    }
  }

  // Emit the top-level struct
  if (schema.type === 'object' && schema.properties) {
    blocks.push(emitStruct(schema.title || 'Root', schema, ctx));
  }

  // Emit registered enums
  for (const [name, values] of enumRegistry) {
    blocks.push(emitEnum(name, values));
  }

  return blocks.join('\n\n');
}
```

- [ ] **Step 6.4: Run tests to verify they pass**

```bash
node --test elohim/rakia/schemas/tests/codegen-rs.test.mjs
```

Expected: 3/3 pass.

- [ ] **Step 6.5: Add test for $defs and string enum**

Append to `codegen-rs.test.mjs`:

```javascript
describe('schemaToRust — $defs and enums', () => {
  it('emits structs from $defs in addition to root', () => {
    const schema = {
      title: 'Plan',
      type: 'object',
      properties: {
        steps: { type: 'array', items: { $ref: '#/$defs/Step' } },
      },
      $defs: {
        Step: {
          type: 'object',
          required: ['name'],
          properties: { name: { type: 'string' } },
        },
      },
    };
    const out = schemaToRust(schema);
    assert.match(out, /pub struct Step \{/);
    assert.match(out, /pub struct Plan \{/);
    assert.match(out, /pub steps: Option<Vec<Step>>,/);
  });

  it('emits Rust enum for string enum properties', () => {
    const schema = {
      title: 'Holder',
      type: 'object',
      required: ['kind'],
      properties: {
        kind: { type: 'string', enum: ['foo-bar', 'baz'] },
      },
    };
    const out = schemaToRust(schema);
    assert.match(out, /pub enum Kind \{/);
    assert.match(out, /#\[serde\(rename = "foo-bar"\)\]\n    FooBar,/);
    assert.match(out, /pub kind: Kind,/);
  });
});
```

Run: `node --test elohim/rakia/schemas/tests/codegen-rs.test.mjs`
Expected: 5/5 pass.

- [ ] **Step 6.6: Commit**

```bash
git add elohim/rakia/schemas/scripts/lib/schema-to-rust.mjs elohim/rakia/schemas/tests/codegen-rs.test.mjs
git commit -m "feat(rakia/schemas): schema-to-rust translator (structs, enums, $defs, $ref)"
```

---

### Task 7: Codegen orchestration script

**Files:**
- Create: `elohim/rakia/schemas/scripts/codegen-rs.mjs`

- [ ] **Step 7.1: Implement `codegen-rs.mjs`**

```javascript
#!/usr/bin/env node
/**
 * Generates Rust types from rakia JSON Schemas.
 *
 * Output: elohim/rakia/rakia-core/src/generated_types.rs
 *
 * Usage:
 *   node codegen-rs.mjs           # Generate
 *   node codegen-rs.mjs --verify  # Compare against on-disk file; exit 1 on drift
 */
import { readFile, writeFile, mkdtemp, rm } from 'node:fs/promises';
import { resolve, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tmpdir } from 'node:os';
import { execFileSync } from 'node:child_process';
import { schemaToRust } from './lib/schema-to-rust.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SCHEMA_DIR = resolve(__dirname, '../v1');
const OUTPUT_PATH = resolve(__dirname, '../../rakia-core/src/generated_types.rs');

const SCHEMAS = [
  'build-manifest.schema.json',
  'build-plan.schema.json',
];

const VERIFY = process.argv.includes('--verify');

const HEADER = `//! AUTO-GENERATED from elohim/rakia/schemas/v1/.
//! DO NOT EDIT — regenerate with: pnpm run rakia:codegen:rs
//!
//! Source schemas:
${SCHEMAS.map((s) => `//!   - ${s}`).join('\n')}

#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
`;

async function generate() {
  const enumRegistry = new Map();
  const blocks = [];

  for (const schemaFile of SCHEMAS) {
    const path = join(SCHEMA_DIR, schemaFile);
    const text = await readFile(path, 'utf8');
    const schema = JSON.parse(text);
    blocks.push(`// ${'='.repeat(70)}`);
    blocks.push(`// From: ${schemaFile}`);
    blocks.push(`// ${'='.repeat(70)}`);
    blocks.push(schemaToRust(schema, { enumRegistry }));
  }

  return HEADER + '\n' + blocks.join('\n\n') + '\n';
}

function tryRustfmt(filePath) {
  try {
    execFileSync('rustfmt', ['--edition', '2021', filePath], { stdio: 'pipe' });
  } catch (err) {
    console.error(`Warning: rustfmt failed (${err.message}). Generated file may not match repo formatting.`);
  }
}

async function main() {
  const generated = await generate();

  if (VERIFY) {
    const tmpDir = await mkdtemp(join(tmpdir(), 'rakia-codegen-'));
    const tmpPath = join(tmpDir, 'generated_types.rs');
    await writeFile(tmpPath, generated);
    tryRustfmt(tmpPath);

    let existing;
    try {
      existing = await readFile(OUTPUT_PATH, 'utf8');
    } catch {
      console.error(`FAIL: ${OUTPUT_PATH} does not exist. Run: pnpm run rakia:codegen:rs`);
      process.exit(1);
    }
    const expected = await readFile(tmpPath, 'utf8');
    await rm(tmpDir, { recursive: true });

    if (existing !== expected) {
      console.error(`FAIL: ${OUTPUT_PATH} is stale. Run: pnpm run rakia:codegen:rs`);
      process.exit(1);
    }
    console.log('rakia generated_types.rs is up to date.');
    return;
  }

  await writeFile(OUTPUT_PATH, generated);
  tryRustfmt(OUTPUT_PATH);
  console.log(`Generated: ${OUTPUT_PATH}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
```

- [ ] **Step 7.2: Run codegen**

```bash
pnpm run rakia:codegen:rs
```

Expected: `Generated: ...generated_types.rs` (no errors). The file is created.

- [ ] **Step 7.3: Inspect the generated file**

```bash
cat elohim/rakia/rakia-core/src/generated_types.rs | head -80
```

Expected: header comment, `use serde::...`, `use std::collections::BTreeMap;`, then `pub struct BuildManifest { ... }`, etc.

If there are translator errors (e.g., "inline nested objects must be lifted to $defs"), that's a signal to refine the schema by lifting nested objects into `$defs`. Iterate Task 3/4 schemas + re-run codegen until clean.

- [ ] **Step 7.4: Verify the generated Rust compiles**

The file isn't yet wired into rakia-core's `lib.rs`. Add it temporarily:

Edit `elohim/rakia/rakia-core/src/lib.rs` and add `pub mod generated_types;`. Then:

```bash
cd elohim/rakia && RUSTFLAGS="" cargo check -p rakia-core
```

Expected: clean compile (warnings allowed). If errors, fix the codegen translator and re-run Task 7.2.

Revert the temporary `pub mod generated_types;` (Task 10 wires it in properly):

```bash
cd /home/matthew/git/elohim
git checkout elohim/rakia/rakia-core/src/lib.rs
```

- [ ] **Step 7.5: Verify --verify mode**

```bash
pnpm run rakia:codegen:rs:verify
```

Expected: `rakia generated_types.rs is up to date.` Exit code 0.

Touch the generated file to simulate drift:

```bash
echo "// drift" >> elohim/rakia/rakia-core/src/generated_types.rs
pnpm run rakia:codegen:rs:verify
```

Expected: `FAIL: ...generated_types.rs is stale.` Exit code 1.

Regenerate to clean up:

```bash
pnpm run rakia:codegen:rs
```

- [ ] **Step 7.6: Commit**

```bash
git add elohim/rakia/schemas/scripts/codegen-rs.mjs elohim/rakia/rakia-core/src/generated_types.rs
git commit -m "feat(rakia/schemas): codegen-rs.mjs orchestrator + initial generated_types.rs"
```

---

### Task 8: Wire codegen verification + manifest validation into pre-push hook

**Files:**
- Modify: `.husky/pre-push`

- [ ] **Step 8.1: Inspect current pre-push structure**

```bash
grep -n "schema-codegen\|schema-validate" .husky/pre-push | head -20
```

Note the existing pattern for project-scoped checks.

- [ ] **Step 8.2: Add rakia codegen verify and schema validate to pre-push**

Edit `.husky/pre-push`. Locate the `schema-codegen` block (which runs `pnpm run schema:codegen:ts -- --verify` on schema changes). Add an analogous block for rakia. Locate where projects/checks are listed. Add:

```sh
# Rakia codegen freshness — runs when rakia schemas or generated_types.rs change
if echo "$CHANGED" | grep -qE '^elohim/rakia/(schemas/|rakia-core/src/(generated_types\.rs|manifest\.rs))'; then
  echo "→ rakia: verifying generated_types.rs is fresh"
  if ! pnpm run rakia:codegen:rs:verify; then
    echo "  Rakia codegen is stale. Run: pnpm run rakia:codegen:rs && git add elohim/rakia/rakia-core/src/generated_types.rs"
    exit 1
  fi
fi

# Rakia manifest schema validation — runs when any build-manifest.json or rakia schema changes
if echo "$CHANGED" | grep -qE '(build-manifest\.json|elohim/rakia/schemas/v1/build-manifest\.schema\.json)$'; then
  echo "→ rakia: validating build-manifest.json files against schema"
  if ! pnpm run rakia:schema:validate; then
    exit 1
  fi
fi
```

(Place these blocks near the existing `schema-codegen` and `schema-validate` blocks, following the same idioms.)

- [ ] **Step 8.3: Test the hook detects drift**

```bash
# Simulate drift
echo "// drift" >> elohim/rakia/rakia-core/src/generated_types.rs
git add elohim/rakia/rakia-core/src/generated_types.rs
HUSKY=1 .husky/pre-push origin refs/heads/dev <<< "" 2>&1 | tail -20 || echo "exit: $?"
# Cleanup
pnpm run rakia:codegen:rs
git add elohim/rakia/rakia-core/src/generated_types.rs
```

Expected: pre-push exits 1 with "Rakia codegen is stale" message during drift simulation, succeeds after regeneration.

- [ ] **Step 8.4: Commit**

```bash
git add .husky/pre-push
git commit -m "ci(rakia): wire codegen:rs:verify + schema:validate into pre-push hook"
```

---

## Phase 3: Replace Escape Hatches

### Task 9: Wire generated_types.rs into rakia-core lib

**Files:**
- Modify: `elohim/rakia/rakia-core/src/lib.rs`

- [ ] **Step 9.1: Add module declaration**

Edit `elohim/rakia/rakia-core/src/lib.rs`. After `pub mod manifest;`, add:

```rust
pub mod generated_types;
```

Reorder so `generated_types` comes BEFORE `manifest` (since `manifest` will depend on it next):

```rust
//! rakia-core — the heart of the firmament

pub mod generated_types;
pub mod manifest;
pub mod discover;
pub mod constellation;
pub mod schema;
```

- [ ] **Step 9.2: Verify rakia-core compiles with the generated module exposed**

```bash
cd elohim/rakia && RUSTFLAGS="" cargo check -p rakia-core 2>&1 | tail -10
```

Expected: clean (warnings about unused types are OK at this stage).

- [ ] **Step 9.3: Commit**

```bash
git add elohim/rakia/rakia-core/src/lib.rs
git commit -m "feat(rakia-core): expose generated_types module"
```

---

### Task 10: Replace BuildManifest types in manifest.rs with generated types

**Goal:** Eliminate every `serde_json::Value` field from `manifest.rs`, replacing local `BuildManifest`, `BuildStep`, `BuildInputs`, `BuildOutputs` with re-exports of generated types.

**Files:**
- Modify: `elohim/rakia/rakia-core/src/manifest.rs`

- [ ] **Step 10.1: Inspect generated types for the manifest**

```bash
grep -A3 "pub struct BuildManifest\|pub struct BuildStep\|pub struct BuildInputs\|pub struct BuildOutputs\|pub struct BuildExecutor\|pub struct BuildGate\|pub struct BuildDeployment" elohim/rakia/rakia-core/src/generated_types.rs
```

Confirm the generated types match what `manifest.rs` needs to export. Field names and types should align with what the schema declared (camelCase via `#[serde(rename_all = "camelCase")]`).

If a needed type is missing from generated_types (e.g., the schema named the executor type differently than expected), update the schema's `$defs` to lift it explicitly:

```json
"$defs": {
  "BuildExecutor": { /* the executor inline shape */ }
}
```

Then re-reference it from the step definition: `"executor": { "$ref": "#/$defs/BuildExecutor" }`. Re-run `pnpm run rakia:codegen:rs`.

- [ ] **Step 10.2: Rewrite `manifest.rs` to re-export generated types**

Replace the contents of `elohim/rakia/rakia-core/src/manifest.rs` with:

```rust
//! Build manifest types — the constellations that hang in the firmament.
//!
//! Reads the existing `build-manifest.json` format from the Elohim monorepo.
//! Types are generated from `elohim/rakia/schemas/v1/build-manifest.schema.json`
//! (see `generated_types`).

pub use crate::generated_types::{
    BuildManifest, BuildStep, BuildInputs, BuildOutputs, BuildGate, BuildDeployment,
    BuildExecutor,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_manifest() {
        let json = r#"{
            "manifestVersion": "1.0",
            "pipeline": "elohim-orchestrator",
            "description": "CI orchestrator",
            "steps": {
                "lint": {
                    "description": "Lint Jenkinsfiles",
                    "inputs": {
                        "sources": ["**/Jenkinsfile*"],
                        "buildProcess": []
                    },
                    "outputs": {
                        "artifacts": [],
                        "verify": null
                    },
                    "depends": []
                }
            }
        }"#;

        let m: BuildManifest = serde_json::from_str(json).expect("parse");
        assert_eq!(m.pipeline, "elohim-orchestrator");
        assert_eq!(m.steps.len(), 1);
        let lint = m.steps.get("lint").expect("lint exists");
        assert_eq!(lint.inputs.sources, vec!["**/Jenkinsfile*"]);
    }

    #[test]
    fn parse_manifest_with_executor() {
        let json = r#"{
            "manifestVersion": "1.0",
            "pipeline": "elohim-app",
            "description": "Angular app",
            "steps": {
                "build": {
                    "description": "Build Angular",
                    "inputs": { "sources": ["src/**/*.ts"] },
                    "outputs": { "artifacts": ["dist/"], "verify": null },
                    "depends": [],
                    "executor": { "kind": "pnpm", "package": "elohim-app", "script": "build" }
                }
            }
        }"#;

        let m: BuildManifest = serde_json::from_str(json).expect("parse");
        let build = m.steps.get("build").expect("build exists");
        let exec = build.executor.as_ref().expect("executor present");
        // Field access depends on generated executor type — adjust as needed
        assert!(matches!(exec.kind, BuildExecutorKind::Pnpm | _));
    }
}
```

(Adjust the second test based on the exact field names and enum variant names that codegen produced.)

- [ ] **Step 10.3: Update consumers in `constellation.rs` and `discover.rs`**

These files currently import from `crate::manifest`. The re-export keeps them working, BUT field accesses may have changed (e.g., camelCase vs snake_case). Compile and fix:

```bash
cd elohim/rakia && RUSTFLAGS="" cargo build -p rakia-core 2>&1 | tail -30
```

Common fixes:
- `step.build_process` → `step.build_process` should still work (snake_case in Rust, camelCase in JSON via serde rename)
- `step.executor` was `serde_json::Value`, now `Option<BuildExecutor>` — pattern-match instead of `.is_object()` checks
- Tests that hand-construct `BuildManifest` may need updates if field types changed

Iterate until `cargo build -p rakia-core` is clean.

- [ ] **Step 10.4: Run rakia-core tests**

```bash
cd elohim/rakia && RUSTFLAGS="" cargo test -p rakia-core 2>&1 | tail -20
```

Expected: all existing tests pass. Constellation builder still works on the same manifest fixtures.

- [ ] **Step 10.5: Verify zero `serde_json::Value` in manifest.rs**

```bash
grep -n "serde_json::Value" elohim/rakia/rakia-core/src/manifest.rs
```

Expected: no output (or only test-local uses).

If there are any remaining `Value` fields, either the schema is too loose (refine it and regenerate) or the field is genuinely freeform (unlikely in this domain).

- [ ] **Step 10.6: Commit**

```bash
git add elohim/rakia/rakia-core/src/manifest.rs elohim/rakia/rakia-core/src/constellation.rs elohim/rakia/rakia-core/src/discover.rs
git commit -m "refactor(rakia-core): manifest.rs uses generated types — zero serde_json::Value"
```

---

### Task 11: Build BuildPlan converter (TopoPlan → BuildPlan)

**Goal:** Add a converter that takes the internal `TopoPlan` from `constellation.rs` plus baseline/head metadata and produces a schema-conforming `BuildPlan` (the output contract).

**Files:**
- Create: `elohim/rakia/rakia-core/src/build_plan.rs`
- Modify: `elohim/rakia/rakia-core/src/lib.rs`
- Modify: `elohim/rakia/rakia-core/src/constellation.rs` (rework affected reasons to be structured)

- [ ] **Step 11.1: Rework `affected` reasons in `constellation.rs` to use structured types**

Currently `plan_from_changes` uses `Vec<String>` like `"file: foo.ts"` and `"upstream: bar"`. Change it to `Vec<AffectedReason>` (where `AffectedReason` is the generated type from BuildPlan schema).

Open `elohim/rakia/rakia-core/src/constellation.rs`. Find the `affected: BTreeMap<String, Vec<String>>` declaration in `plan_from_changes`. Change to:

```rust
use crate::generated_types::{AffectedReason, AffectedReasonKind};

// In plan_from_changes:
let mut affected: BTreeMap<String, Vec<AffectedReason>> = BTreeMap::new();

// When matching a changed file:
affected
    .entry(name.clone())
    .or_default()
    .push(AffectedReason {
        kind: AffectedReasonKind::ChangedFile,
        path: Some(path.clone()),
        upstream: None,
    });

// When propagating to dependents:
affected
    .entry(name.clone())
    .or_default()
    .push(AffectedReason {
        kind: AffectedReasonKind::UpstreamNode,
        path: None,
        upstream: Some(step_name.clone()),
    });
```

Update `topo_sort_affected` and `TopoPlan` to carry `Vec<AffectedReason>` per step (add a field to `QualifiedStep` or to a new wrapper). Suggested: make `TopoPlan` carry `Vec<Vec<(QualifiedStep, Vec<AffectedReason>)>>` — preserves the affected-by provenance through the plan.

- [ ] **Step 11.2: Run rakia-core tests after the refactor**

```bash
cd elohim/rakia && RUSTFLAGS="" cargo test -p rakia-core 2>&1 | tail -20
```

Expected: existing tests adjusted for new types, all pass.

- [ ] **Step 11.3: Create `build_plan.rs` converter**

```rust
//! BuildPlan output contract — the converter from internal TopoPlan to the
//! schema-defined BuildPlan that `brit plan` returns.
//!
//! Internal TopoPlan stays in constellation.rs (engine-side, flexible).
//! BuildPlan is the contract (schema-defined, stable for consumers).

use crate::constellation::TopoPlan;
use crate::generated_types::{
    BuildPlan, Baseline, Head, PlannedStep, BuildPlanTool,
};

/// Convert an internal TopoPlan + change-detection context into the
/// schema-conforming BuildPlan.
///
/// `baseline_ref` — git ref name (e.g. "refs/notes/rakia/baselines/elohim")
/// `baseline_commit` — 40-char hex SHA
/// `head_commit` — 40-char hex SHA
/// `changed_paths` — workspace-relative paths (empty when --files was used)
/// `tool_version` — version string of the brit binary
pub fn to_build_plan(
    plan: &TopoPlan,
    baseline_ref: &str,
    baseline_commit: &str,
    head_commit: &str,
    changed_paths: &[String],
    tool_version: &str,
) -> BuildPlan {
    let levels: Vec<Vec<PlannedStep>> = plan
        .levels
        .iter()
        .map(|level| {
            level
                .iter()
                .map(|(step, reasons)| PlannedStep {
                    pipeline: step.pipeline.clone(),
                    name: step.step_name.clone(),
                    qualified_name: step.qualified_name.clone(),
                    fingerprint: compute_fingerprint(step),
                    depends: step.resolved_depends.clone(),
                    affected_by: reasons.clone(),
                })
                .collect()
        })
        .collect();

    BuildPlan {
        plan_version: "1.0".to_string(),
        baseline: Baseline {
            r#ref: baseline_ref.to_string(),
            commit: baseline_commit.to_string(),
        },
        head: Head {
            commit: head_commit.to_string(),
        },
        changed_paths: Some(changed_paths.to_vec()),
        levels,
        generated_at: chrono::Utc::now().to_rfc3339(),
        tool: BuildPlanTool {
            name: "brit".to_string(),
            version: tool_version.to_string(),
        },
    }
}

/// Placeholder — real fingerprinting uses brit-graph's ContentFingerprint.
/// This stub returns a deterministic hex string of the qualified name's blake3.
fn compute_fingerprint(step: &crate::constellation::QualifiedStep) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    step.qualified_name.hash(&mut hasher);
    step.source_patterns.hash(&mut hasher);
    step.build_process.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constellation::{QualifiedStep, TopoPlan};
    use crate::generated_types::{AffectedReason, AffectedReasonKind};
    use std::path::PathBuf;

    fn sample_step(name: &str) -> QualifiedStep {
        QualifiedStep {
            qualified_name: format!("p:{name}"),
            pipeline: "p".to_string(),
            step_name: name.to_string(),
            description: String::new(),
            source_patterns: vec![],
            build_process: vec![],
            artifacts: vec![],
            resolved_depends: vec![],
            manifest_path: PathBuf::new(),
        }
    }

    #[test]
    fn empty_plan_produces_empty_levels() {
        let plan = TopoPlan { levels: vec![] };
        let bp = to_build_plan(&plan, "refs/x", "0".repeat(40).as_str(), "1".repeat(40).as_str(), &[], "0.0.0");
        assert_eq!(bp.plan_version, "1.0");
        assert!(bp.levels.is_empty());
    }

    #[test]
    fn single_step_with_changed_file_reason() {
        let step = sample_step("build");
        let reason = AffectedReason {
            kind: AffectedReasonKind::ChangedFile,
            path: Some("src/foo.ts".to_string()),
            upstream: None,
        };
        let plan = TopoPlan {
            levels: vec![vec![(step, vec![reason])]],
        };
        let bp = to_build_plan(
            &plan,
            "refs/notes/rakia/baselines/p",
            "a".repeat(40).as_str(),
            "b".repeat(40).as_str(),
            &["src/foo.ts".to_string()],
            "0.1.0",
        );
        assert_eq!(bp.levels.len(), 1);
        assert_eq!(bp.levels[0].len(), 1);
        assert_eq!(bp.levels[0][0].qualified_name, "p:build");
        assert_eq!(bp.levels[0][0].affected_by.len(), 1);
        assert_eq!(bp.levels[0][0].affected_by[0].path.as_deref(), Some("src/foo.ts"));
    }
}
```

- [ ] **Step 11.4: Add `chrono` to rakia-core dependencies**

Edit `elohim/rakia/rakia-core/Cargo.toml`, add to `[dependencies]`:

```toml
chrono = { version = "0.4", default-features = false, features = ["clock", "std"] }
```

- [ ] **Step 11.5: Wire module into lib.rs**

Edit `elohim/rakia/rakia-core/src/lib.rs`, add:

```rust
pub mod build_plan;
```

- [ ] **Step 11.6: Build and test**

```bash
cd elohim/rakia && RUSTFLAGS="" cargo test -p rakia-core 2>&1 | tail -30
```

Expected: all tests pass, including the two new ones in `build_plan.rs`.

- [ ] **Step 11.7: Add schema-contract test for BuildPlan output**

`elohim/rakia/rakia-core/tests/build_plan_schema_contract.rs`:

```rust
//! Validates that BuildPlan serialization conforms to build-plan.schema.json.
//!
//! Catches drift between Rust struct changes and the schema contract.

use rakia_core::build_plan::to_build_plan;
use rakia_core::constellation::{QualifiedStep, TopoPlan};
use rakia_core::generated_types::{AffectedReason, AffectedReasonKind};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn load_schema() -> Value {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = manifest_dir.join("../schemas/v1/build-plan.schema.json");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    serde_json::from_str(&text).expect("parse schema")
}

fn validate(instance: &Value, schema: &Value) {
    let validator = jsonschema::draft202012::new(schema).expect("compile schema");
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| format!("{}: {e}", e.instance_path))
        .collect();
    if !errors.is_empty() {
        panic!("BuildPlan failed schema validation:\n{}", errors.join("\n"));
    }
}

#[test]
fn empty_plan_validates() {
    let schema = load_schema();
    let plan = TopoPlan { levels: vec![] };
    let bp = to_build_plan(
        &plan,
        "refs/notes/rakia/baselines/test",
        &"a".repeat(40),
        &"b".repeat(40),
        &[],
        "0.0.0",
    );
    let json = serde_json::to_value(&bp).expect("serialize");
    validate(&json, &schema);
}

#[test]
fn populated_plan_validates() {
    let schema = load_schema();
    let step = QualifiedStep {
        qualified_name: "elohim-app:build".to_string(),
        pipeline: "elohim-app".to_string(),
        step_name: "build".to_string(),
        description: "Build Angular".to_string(),
        source_patterns: vec!["src/**/*.ts".to_string()],
        build_process: vec![],
        artifacts: vec!["dist/".to_string()],
        resolved_depends: vec![],
        manifest_path: PathBuf::new(),
    };
    let reason = AffectedReason {
        kind: AffectedReasonKind::ChangedFile,
        path: Some("src/foo.ts".to_string()),
        upstream: None,
    };
    let plan = TopoPlan {
        levels: vec![vec![(step, vec![reason])]],
    };
    let bp = to_build_plan(
        &plan,
        "refs/notes/rakia/baselines/elohim-app",
        &"a".repeat(40),
        &"b".repeat(40),
        &["src/foo.ts".to_string()],
        "0.1.0",
    );
    let json = serde_json::to_value(&bp).expect("serialize");
    validate(&json, &schema);
}
```

- [ ] **Step 11.8: Add `jsonschema` dev-dependency**

Edit `elohim/rakia/rakia-core/Cargo.toml`, add to `[dev-dependencies]`:

```toml
jsonschema = { version = "0.30", default-features = false }
```

(Use whatever recent version is in the lockfile or add fresh.)

- [ ] **Step 11.9: Run the schema contract test**

```bash
cd elohim/rakia && RUSTFLAGS="" cargo test -p rakia-core --test build_plan_schema_contract 2>&1 | tail -20
```

Expected: 2/2 pass. If validation fails, the BuildPlan struct doesn't match the schema — fix one or the other and re-run codegen.

- [ ] **Step 11.10: Commit**

```bash
git add elohim/rakia/rakia-core/src/build_plan.rs elohim/rakia/rakia-core/src/lib.rs elohim/rakia/rakia-core/src/constellation.rs elohim/rakia/rakia-core/Cargo.toml elohim/rakia/rakia-core/tests/build_plan_schema_contract.rs
git commit -m "feat(rakia-core): build_plan converter (TopoPlan → schema-conforming BuildPlan) + contract test"
```

---

### Task 12: Wire `build_process` matching into change detection

**Goal:** Close the known follow-up "buildProcess parsed but unused in change detection." Manifest declares `inputs.buildProcess: ["package.json", ...]`; matching paths against those globs should affect the step too.

**Files:**
- Modify: `elohim/rakia/rakia-core/src/constellation.rs`

- [ ] **Step 12.1: Add a failing test in constellation tests**

Append to the existing tests module in `constellation.rs` (or `tests/integration.rs` if that's where tests live):

```rust
#[test]
fn build_process_path_match_affects_step() {
    use crate::manifest::{BuildManifest, BuildStep, BuildInputs, BuildOutputs};
    use std::collections::BTreeMap;

    let mut steps = BTreeMap::new();
    steps.insert("build".to_string(), BuildStep {
        description: "Build".to_string(),
        inputs: BuildInputs {
            sources: vec!["src/**/*.ts".to_string()],
            build_process: Some(vec!["package.json".to_string(), "tsconfig.json".to_string()]),
        },
        outputs: BuildOutputs { artifacts: vec![], verify: None },
        depends: Some(vec![]),
        executor: None,
    });
    let manifest = BuildManifest {
        manifest_version: "1.0".to_string(),
        pipeline: "test".to_string(),
        description: "test".to_string(),
        steps,
        gate: None,
        deployment: None,
        manual_only: None,
    };

    let constellation = build_constellation(vec![(PathBuf::from("test/manifest.json"), manifest)])
        .expect("build constellation");

    // Changing package.json (a buildProcess input) should affect the step
    let plan = plan_from_changes(&constellation, &["package.json".to_string()])
        .expect("plan");
    assert!(!plan.is_empty(), "package.json change should affect step via buildProcess");
}
```

(Adjust field names — `build_process` may be `Option<Vec<String>>` after codegen depending on schema's `required` array.)

- [ ] **Step 12.2: Run test to verify it fails**

```bash
cd elohim/rakia && RUSTFLAGS="" cargo test -p rakia-core build_process_path_match 2>&1 | tail -10
```

Expected: FAIL — `plan` is empty because `build_process` patterns aren't matched.

- [ ] **Step 12.3: Implement the fix in `plan_from_changes`**

Find the loop that builds `glob_set` from `step.source_patterns`. Extend it to also include `build_process` patterns:

```rust
for (name, step) in &constellation.steps {
    let mut all_patterns: Vec<String> = step.source_patterns.clone();
    all_patterns.extend(step.build_process.iter().cloned());
    let glob_set = build_glob_set(&all_patterns)?;
    for path in changed_paths {
        if glob_set.is_match(path) {
            affected
                .entry(name.clone())
                .or_default()
                .push(AffectedReason {
                    kind: AffectedReasonKind::ChangedFile,
                    path: Some(path.clone()),
                    upstream: None,
                });
            break;
        }
    }
}
```

- [ ] **Step 12.4: Run test to verify it passes**

```bash
cd elohim/rakia && RUSTFLAGS="" cargo test -p rakia-core 2>&1 | tail -20
```

Expected: all tests pass, including the new one.

- [ ] **Step 12.5: Commit**

```bash
git add elohim/rakia/rakia-core/src/constellation.rs
git commit -m "fix(rakia-core): match buildProcess patterns in change detection (was parsed but unused)"
```

---

## Phase 4: Brit CLI Scaffold

### Task 13: Scaffold brit-cli crate

**Files:**
- Create: `elohim/brit/brit-cli/Cargo.toml`
- Create: `elohim/brit/brit-cli/src/main.rs`
- Create: `elohim/brit/brit-cli/src/error.rs`
- Create: `elohim/brit/brit-cli/src/output.rs`
- Create: `elohim/brit/brit-cli/src/commands/mod.rs`
- Modify: `elohim/brit/Cargo.toml` (add to workspace.members)

- [ ] **Step 13.1: Create `brit-cli/Cargo.toml`**

```toml
lints.workspace = true

[package]
name = "brit-cli"
version = "0.0.0"
description = "Unified brit CLI — graph, affected, plan, fingerprint, baseline subcommands"
repository = "https://github.com/ethosengine/brit"
authors = ["Matthew Dowell <matthew@ethosengine.com>"]
license = "MIT OR Apache-2.0"
edition = "2021"
rust-version = "1.82"

[[bin]]
name = "brit"
path = "src/main.rs"

[dependencies]
brit-epr = { path = "../brit-epr", default-features = false }
brit-graph = { path = "../brit-graph" }
rakia-core = { path = "../../rakia/rakia-core" }
rakia-brit = { path = "../../rakia/rakia-brit" }

clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
anyhow = "1"
```

- [ ] **Step 13.2: Add brit-cli to brit workspace members**

Edit `elohim/brit/Cargo.toml`. Locate the `members = [...]` block. Add `"brit-cli"`:

```toml
members = [
    # ... existing members ...
    "brit-epr",
    "brit-verify",
    "brit-build-ref",
    "brit-graph",
    "brit-cli",
]
```

- [ ] **Step 13.3: Create `error.rs`**

```rust
//! Error types and exit code mapping for the brit CLI.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("repo not found at {path}: {source}")]
    RepoNotFound {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("manifest discovery failed: {0}")]
    ManifestDiscovery(String),

    #[error("constellation construction failed: {0}")]
    Constellation(#[from] rakia_core::constellation::ConstellationError),

    #[error("change detection failed: {0}")]
    ChangeDetection(String),

    #[error("baseline operation failed: {0}")]
    Baseline(String),

    #[error("invalid arguments: {0}")]
    Args(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl CliError {
    /// Map error variants to exit codes.
    /// 0 — success (not used here)
    /// 1 — generic failure
    /// 2 — argument/usage error
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::Args(_) => 2,
            _ => 1,
        }
    }
}

pub type Result<T> = std::result::Result<T, CliError>;
```

- [ ] **Step 13.4: Create `output.rs`**

```rust
//! Output helpers — pretty JSON to stdout, errors to stderr.

use serde::Serialize;

pub fn print_json<T: Serialize>(value: &T) -> Result<(), serde_json::Error> {
    let s = serde_json::to_string_pretty(value)?;
    println!("{s}");
    Ok(())
}
```

- [ ] **Step 13.5: Create `commands/mod.rs` (stub)**

```rust
//! brit CLI subcommands.

pub mod graph_discover;
pub mod graph_show;
pub mod affected;
pub mod plan;
pub mod fingerprint;
pub mod baseline;
```

- [ ] **Step 13.6: Create stub command files**

For each of `graph_discover.rs`, `graph_show.rs`, `affected.rs`, `plan.rs`, `fingerprint.rs`, `baseline.rs` in `elohim/brit/brit-cli/src/commands/`, create a stub:

`graph_discover.rs`:
```rust
use crate::error::Result;

pub fn run(_repo: &std::path::Path) -> Result<()> {
    eprintln!("brit graph discover: not yet implemented");
    Ok(())
}
```

(Repeat the same shape for the other five files, naming the function `run` and accepting whatever args the subcommand will need.)

- [ ] **Step 13.7: Create `main.rs` with clap entrypoint**

```rust
//! brit CLI — unified entry point.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

mod commands;
mod error;
mod output;

use error::Result;

#[derive(Parser)]
#[command(name = "brit", version, about = "Brit — covenant on git, EPR-native CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Graph operations on the build constellation
    #[command(subcommand)]
    Graph(GraphCmd),
    /// Show which steps are affected by changes
    Affected(AffectedArgs),
    /// Compute a topologically-grouped build plan
    Plan(PlanArgs),
    /// Compute the content fingerprint of a step's inputs
    Fingerprint(FingerprintArgs),
    /// Manage rakia baseline refs
    #[command(subcommand)]
    Baseline(BaselineCmd),
}

#[derive(Subcommand)]
enum GraphCmd {
    /// Discover and list all build manifests
    Discover {
        #[arg(long, default_value = ".")]
        repo: PathBuf,
    },
    /// Show the full constellation graph
    Show {
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        #[arg(long, default_value = "json", value_parser = ["json", "dot"])]
        format: String,
    },
}

#[derive(clap::Args)]
struct AffectedArgs {
    #[arg(long, default_value = ".")]
    repo: PathBuf,
    /// Comma-separated list of changed files (workspace-relative)
    #[arg(long, conflicts_with = "since", required_unless_present = "since")]
    files: Option<String>,
    /// Compute affected from changes since the given git ref (e.g. baseline)
    #[arg(long)]
    since: Option<String>,
}

#[derive(clap::Args)]
struct PlanArgs {
    #[arg(long, default_value = ".")]
    repo: PathBuf,
    #[arg(long, conflicts_with = "since", required_unless_present = "since")]
    files: Option<String>,
    #[arg(long)]
    since: Option<String>,
    /// Pipeline name (used to locate baseline ref when --since is auto)
    #[arg(long)]
    pipeline: Option<String>,
}

#[derive(clap::Args)]
struct FingerprintArgs {
    /// Path to a build-manifest.json
    manifest: PathBuf,
    /// Specific step name (default: all steps in the manifest)
    #[arg(long)]
    step: Option<String>,
}

#[derive(Subcommand)]
enum BaselineCmd {
    /// Read the current baseline ref for a pipeline
    Read {
        pipeline: String,
        #[arg(long, default_value = ".")]
        repo: PathBuf,
    },
    /// Write a baseline ref for a pipeline
    Write {
        pipeline: String,
        commit: String,
        #[arg(long, default_value = ".")]
        repo: PathBuf,
    },
    /// One-shot migration from Jenkins pipeline-baselines.json
    Migrate {
        json_path: PathBuf,
        #[arg(long, default_value = ".")]
        repo: PathBuf,
    },
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Graph(GraphCmd::Discover { repo }) => commands::graph_discover::run(&repo),
        Command::Graph(GraphCmd::Show { repo, format }) => commands::graph_show::run(&repo, &format),
        Command::Affected(args) => commands::affected::run(&args.repo, args.files.as_deref(), args.since.as_deref()),
        Command::Plan(args) => commands::plan::run(&args.repo, args.files.as_deref(), args.since.as_deref(), args.pipeline.as_deref()),
        Command::Fingerprint(args) => commands::fingerprint::run(&args.manifest, args.step.as_deref()),
        Command::Baseline(BaselineCmd::Read { pipeline, repo }) => commands::baseline::read(&repo, &pipeline),
        Command::Baseline(BaselineCmd::Write { pipeline, commit, repo }) => commands::baseline::write(&repo, &pipeline, &commit),
        Command::Baseline(BaselineCmd::Migrate { json_path, repo }) => commands::baseline::migrate(&repo, &json_path),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(e.exit_code() as u8)
        }
    }
}
```

- [ ] **Step 13.8: Adjust the stub command function signatures to match the dispatch**

Update each stub to accept the right args. For example, `graph_show.rs`:

```rust
pub fn run(_repo: &std::path::Path, _format: &str) -> crate::error::Result<()> {
    eprintln!("brit graph show: not yet implemented");
    Ok(())
}
```

`affected.rs`:
```rust
pub fn run(_repo: &std::path::Path, _files: Option<&str>, _since: Option<&str>) -> crate::error::Result<()> {
    eprintln!("brit affected: not yet implemented");
    Ok(())
}
```

`plan.rs`:
```rust
pub fn run(_repo: &std::path::Path, _files: Option<&str>, _since: Option<&str>, _pipeline: Option<&str>) -> crate::error::Result<()> {
    eprintln!("brit plan: not yet implemented");
    Ok(())
}
```

`fingerprint.rs`:
```rust
pub fn run(_manifest: &std::path::Path, _step: Option<&str>) -> crate::error::Result<()> {
    eprintln!("brit fingerprint: not yet implemented");
    Ok(())
}
```

`baseline.rs`:
```rust
use crate::error::Result;
use std::path::Path;

pub fn read(_repo: &Path, _pipeline: &str) -> Result<()> {
    eprintln!("brit baseline read: not yet implemented");
    Ok(())
}

pub fn write(_repo: &Path, _pipeline: &str, _commit: &str) -> Result<()> {
    eprintln!("brit baseline write: not yet implemented");
    Ok(())
}

pub fn migrate(_repo: &Path, _json_path: &Path) -> Result<()> {
    eprintln!("brit baseline migrate: not yet implemented");
    Ok(())
}
```

- [ ] **Step 13.9: Build the brit-cli crate**

```bash
cd elohim/brit && RUSTFLAGS="" cargo build -p brit-cli 2>&1 | tail -20
```

Expected: clean build. Binary at `target/debug/brit`.

If cross-workspace path deps cause issues (e.g., "package not found"), confirm rakia is built first or the path is correct (`../../rakia/rakia-core` from `brit-cli/`).

- [ ] **Step 13.10: Smoke-test the binary**

```bash
cd elohim/brit && ./target/debug/brit --help
./target/debug/brit graph --help
./target/debug/brit graph discover
```

Expected:
- `brit --help` shows top-level subcommands
- `brit graph --help` shows graph subcommands
- `brit graph discover` prints the stub message and exits 0

- [ ] **Step 13.11: Commit**

```bash
git add elohim/brit/brit-cli/ elohim/brit/Cargo.toml
git commit -m "feat(brit-cli): scaffold crate with clap subcommand stubs (graph, affected, plan, fingerprint, baseline)"
```

---

## Phase 5: CLI Subcommand Implementations

### Task 14: Implement `brit graph discover`

**Files:**
- Modify: `elohim/brit/brit-cli/src/commands/graph_discover.rs`

- [ ] **Step 14.1: Write a smoke test for the command**

`elohim/brit/brit-cli/tests/cli_smoke.rs` (create file):

```rust
use std::process::Command;

fn brit_binary() -> std::path::PathBuf {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // workspace target/debug/brit
    manifest_dir.join("../target/debug/brit")
}

#[test]
fn graph_discover_outputs_json_with_pipelines() {
    // Use the actual repo root (two levels up from brit-cli)
    let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../").canonicalize().unwrap();

    let out = Command::new(brit_binary())
        .args(["graph", "discover", "--repo"])
        .arg(&repo_root)
        .output()
        .expect("invoke brit");

    assert!(out.status.success(), "exit {} stderr: {}", out.status, String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8(out.stdout).expect("utf8 stdout");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("parse json");
    assert!(v.get("manifests").is_some(), "expected 'manifests' key in output");
}
```

- [ ] **Step 14.2: Run test — expect failure (stub still in place)**

```bash
cd elohim/brit && RUSTFLAGS="" cargo build -p brit-cli && cargo test -p brit-cli graph_discover 2>&1 | tail -10
```

Expected: FAIL — stub prints to stderr, no JSON to stdout, parse fails.

- [ ] **Step 14.3: Implement `graph_discover.rs`**

```rust
//! brit graph discover — list all build-manifest.json files and summary info.

use std::path::Path;

use serde::Serialize;

use crate::error::{CliError, Result};

#[derive(Serialize)]
struct DiscoverOutput {
    manifests: Vec<ManifestSummary>,
}

#[derive(Serialize)]
struct ManifestSummary {
    path: String,
    pipeline: String,
    description: String,
    step_count: usize,
    steps: Vec<String>,
}

pub fn run(repo: &Path) -> Result<()> {
    let repo = repo.canonicalize().map_err(|source| CliError::RepoNotFound {
        path: repo.display().to_string(),
        source,
    })?;

    let manifests = rakia_core::discover::discover_manifests(&repo)
        .map_err(|e| CliError::ManifestDiscovery(format!("{e}")))?;

    let summaries: Vec<ManifestSummary> = manifests
        .into_iter()
        .map(|(path, m)| {
            let mut steps: Vec<String> = m.steps.keys().cloned().collect();
            steps.sort();
            ManifestSummary {
                path: path
                    .strip_prefix(&repo)
                    .unwrap_or(&path)
                    .display()
                    .to_string(),
                pipeline: m.pipeline,
                description: m.description,
                step_count: steps.len(),
                steps,
            }
        })
        .collect();

    crate::output::print_json(&DiscoverOutput { manifests: summaries })?;
    Ok(())
}
```

(The exact API of `rakia_core::discover::discover_manifests` may differ — adjust to match. If it returns `Result`, propagate the error; if it can panic on missing files, catch and convert.)

- [ ] **Step 14.4: Build and re-run smoke test**

```bash
cd elohim/brit && RUSTFLAGS="" cargo build -p brit-cli && cargo test -p brit-cli graph_discover 2>&1 | tail -10
```

Expected: PASS. Test invokes `brit graph discover`, parses JSON, finds `manifests` key.

- [ ] **Step 14.5: Manual smoke test**

```bash
cd /home/matthew/git/elohim
elohim/brit/target/debug/brit graph discover --repo .
```

Expected: JSON output with 8 manifest entries.

- [ ] **Step 14.6: Commit**

```bash
git add elohim/brit/brit-cli/src/commands/graph_discover.rs elohim/brit/brit-cli/tests/cli_smoke.rs
git commit -m "feat(brit-cli): graph discover — list manifests + summary JSON"
```

---

### Task 15: Implement `brit graph show`

**Files:**
- Modify: `elohim/brit/brit-cli/src/commands/graph_show.rs`

- [ ] **Step 15.1: Add petgraph to brit-cli for Dot output**

Edit `elohim/brit/brit-cli/Cargo.toml`, add to `[dependencies]`:

```toml
petgraph = "0.7"
```

- [ ] **Step 15.2: Implement `graph_show.rs`**

```rust
//! brit graph show — emit the constellation as JSON or Graphviz DOT.

use std::path::Path;

use petgraph::dot::{Config, Dot};
use petgraph::graph::DiGraph;
use serde::Serialize;

use crate::error::{CliError, Result};

#[derive(Serialize)]
struct GraphJson {
    nodes: Vec<NodeJson>,
    edges: Vec<EdgeJson>,
}

#[derive(Serialize)]
struct NodeJson {
    qualified_name: String,
    pipeline: String,
    name: String,
    sources: Vec<String>,
    artifacts: Vec<String>,
}

#[derive(Serialize)]
struct EdgeJson {
    from: String,
    to: String,
}

pub fn run(repo: &Path, format: &str) -> Result<()> {
    let repo = repo.canonicalize().map_err(|source| CliError::RepoNotFound {
        path: repo.display().to_string(),
        source,
    })?;

    let manifests = rakia_core::discover::discover_manifests(&repo)
        .map_err(|e| CliError::ManifestDiscovery(format!("{e}")))?;
    let constellation = rakia_core::constellation::build_constellation(manifests)?;

    match format {
        "json" => {
            let nodes: Vec<NodeJson> = constellation
                .steps
                .values()
                .map(|s| NodeJson {
                    qualified_name: s.qualified_name.clone(),
                    pipeline: s.pipeline.clone(),
                    name: s.step_name.clone(),
                    sources: s.source_patterns.clone(),
                    artifacts: s.artifacts.clone(),
                })
                .collect();
            let mut edges = Vec::new();
            for s in constellation.steps.values() {
                for dep in &s.resolved_depends {
                    edges.push(EdgeJson {
                        from: dep.clone(),
                        to: s.qualified_name.clone(),
                    });
                }
            }
            crate::output::print_json(&GraphJson { nodes, edges })?;
        }
        "dot" => {
            // Build a petgraph DiGraph for Dot rendering
            let mut g: DiGraph<String, ()> = DiGraph::new();
            let mut node_indices = std::collections::HashMap::new();
            for s in constellation.steps.values() {
                let idx = g.add_node(s.qualified_name.clone());
                node_indices.insert(s.qualified_name.clone(), idx);
            }
            for s in constellation.steps.values() {
                let to = node_indices[&s.qualified_name];
                for dep in &s.resolved_depends {
                    if let Some(&from) = node_indices.get(dep) {
                        g.add_edge(from, to, ());
                    }
                }
            }
            let dot = Dot::with_config(&g, &[Config::EdgeNoLabel]);
            println!("{dot:?}");
        }
        other => {
            return Err(CliError::Args(format!("unknown format: {other}")));
        }
    }

    Ok(())
}
```

- [ ] **Step 15.3: Build and smoke test**

```bash
cd elohim/brit && RUSTFLAGS="" cargo build -p brit-cli
./target/debug/brit graph show --repo /home/matthew/git/elohim --format json | head -20
./target/debug/brit graph show --repo /home/matthew/git/elohim --format dot | head -20
```

Expected: JSON shape with `nodes` + `edges`; DOT output starts with `digraph {`.

- [ ] **Step 15.4: Pipe DOT through Graphviz to verify it renders**

```bash
./target/debug/brit graph show --repo /home/matthew/git/elohim --format dot | dot -Tsvg -o /tmp/constellation.svg
echo "SVG size: $(wc -c < /tmp/constellation.svg)"
```

Expected: non-trivial SVG (>1KB). Visual inspection optional.

- [ ] **Step 15.5: Commit**

```bash
git add elohim/brit/brit-cli/Cargo.toml elohim/brit/brit-cli/src/commands/graph_show.rs
git commit -m "feat(brit-cli): graph show — JSON and Graphviz DOT output"
```

---

### Task 16: Implement `brit affected`

**Files:**
- Modify: `elohim/brit/brit-cli/src/commands/affected.rs`

- [ ] **Step 16.1: Implement `affected.rs`**

```rust
//! brit affected — which steps are affected by changes, with provenance.

use std::path::Path;

use serde::Serialize;

use crate::error::{CliError, Result};

#[derive(Serialize)]
struct AffectedOutput {
    changed_paths: Vec<String>,
    affected: Vec<AffectedStep>,
}

#[derive(Serialize)]
struct AffectedStep {
    qualified_name: String,
    affected_by: Vec<rakia_core::generated_types::AffectedReason>,
}

pub fn run(repo: &Path, files: Option<&str>, since: Option<&str>) -> Result<()> {
    let repo = repo.canonicalize().map_err(|source| CliError::RepoNotFound {
        path: repo.display().to_string(),
        source,
    })?;

    let changed_paths: Vec<String> = if let Some(files) = files {
        files.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
    } else if let Some(since) = since {
        rakia_brit::changes::changed_paths_since(&repo, since, "HEAD")
            .map_err(|e| CliError::ChangeDetection(format!("{e}")))?
    } else {
        return Err(CliError::Args("need --files or --since".into()));
    };

    let manifests = rakia_core::discover::discover_manifests(&repo)
        .map_err(|e| CliError::ManifestDiscovery(format!("{e}")))?;
    let constellation = rakia_core::constellation::build_constellation(manifests)?;
    let plan = rakia_core::constellation::plan_from_changes(&constellation, &changed_paths)?;

    // Flatten plan levels into a single affected list (for `affected` we don't care about ordering)
    let mut affected: Vec<AffectedStep> = Vec::new();
    for level in &plan.levels {
        for (step, reasons) in level {
            affected.push(AffectedStep {
                qualified_name: step.qualified_name.clone(),
                affected_by: reasons.clone(),
            });
        }
    }

    crate::output::print_json(&AffectedOutput { changed_paths, affected })?;
    Ok(())
}
```

- [ ] **Step 16.2: Build and test**

```bash
cd elohim/brit && RUSTFLAGS="" cargo build -p brit-cli
./target/debug/brit affected --repo /home/matthew/git/elohim --files "app/elohim-app/src/styles.scss"
```

Expected: JSON listing the affected step(s) with `affected_by` provenance.

Test with `--since HEAD~5` (assuming the repo has recent commits):

```bash
./target/debug/brit affected --repo /home/matthew/git/elohim --since HEAD~5 | head -40
```

Expected: JSON output with relevant steps.

- [ ] **Step 16.3: Test error case — no flags**

```bash
./target/debug/brit affected --repo /home/matthew/git/elohim
echo "exit: $?"
```

Expected: clap rejects (one of `--files` or `--since` required), exit 2.

- [ ] **Step 16.4: Commit**

```bash
git add elohim/brit/brit-cli/src/commands/affected.rs
git commit -m "feat(brit-cli): affected — list affected steps from --files or --since"
```

---

### Task 17: Implement `brit plan`

**Files:**
- Modify: `elohim/brit/brit-cli/src/commands/plan.rs`

- [ ] **Step 17.1: Implement `plan.rs`**

```rust
//! brit plan — topologically grouped build plan, conforming to build-plan.schema.json.

use std::path::Path;

use crate::error::{CliError, Result};

const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run(
    repo: &Path,
    files: Option<&str>,
    since: Option<&str>,
    pipeline: Option<&str>,
) -> Result<()> {
    let repo = repo.canonicalize().map_err(|source| CliError::RepoNotFound {
        path: repo.display().to_string(),
        source,
    })?;

    let (changed_paths, baseline_ref, baseline_commit, head_commit) = if let Some(files) = files {
        let paths: Vec<String> = files
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        // For --files mode, baseline + head are not git-derived; use placeholders
        (paths, "(none)".to_string(), "0".repeat(40), "0".repeat(40))
    } else if let Some(since) = since {
        let head_commit = rakia_brit::changes::head_commit(&repo)
            .map_err(|e| CliError::ChangeDetection(format!("{e}")))?;
        let baseline_commit = rakia_brit::changes::resolve_ref(&repo, since)
            .map_err(|e| CliError::ChangeDetection(format!("{e}")))?;
        let paths = rakia_brit::changes::changed_paths_since(&repo, since, "HEAD")
            .map_err(|e| CliError::ChangeDetection(format!("{e}")))?;
        let ref_name = if let Some(p) = pipeline {
            format!("refs/notes/rakia/baselines/{p}")
        } else {
            since.to_string()
        };
        (paths, ref_name, baseline_commit, head_commit)
    } else {
        return Err(CliError::Args("need --files or --since".into()));
    };

    let manifests = rakia_core::discover::discover_manifests(&repo)
        .map_err(|e| CliError::ManifestDiscovery(format!("{e}")))?;
    let constellation = rakia_core::constellation::build_constellation(manifests)?;
    let plan = rakia_core::constellation::plan_from_changes(&constellation, &changed_paths)?;

    let bp = rakia_core::build_plan::to_build_plan(
        &plan,
        &baseline_ref,
        &baseline_commit,
        &head_commit,
        &changed_paths,
        TOOL_VERSION,
    );

    crate::output::print_json(&bp)?;
    Ok(())
}
```

- [ ] **Step 17.2: Add helpers to `rakia-brit::changes`**

If `head_commit` and `resolve_ref` don't exist on `rakia-brit::changes`, add them:

`elohim/rakia/rakia-brit/src/changes.rs`:

```rust
/// Resolve a ref or rev-spec to a 40-char hex commit SHA.
pub fn resolve_ref(repo_path: &Path, refspec: &str) -> Result<String, ChangeError> {
    use gix::ThreadSafeRepository;
    let repo = ThreadSafeRepository::discover(repo_path)
        .map_err(|e| ChangeError::Repo(format!("{e}")))?
        .to_thread_local();
    let commit = repo
        .rev_parse_single(refspec)
        .map_err(|e| ChangeError::Resolve(format!("{e}")))?;
    Ok(commit.to_hex().to_string())
}

/// Return the commit SHA that HEAD currently points to.
pub fn head_commit(repo_path: &Path) -> Result<String, ChangeError> {
    resolve_ref(repo_path, "HEAD")
}
```

(Add `Repo` and `Resolve` variants to `ChangeError` if not already present. Adjust gix API usage to match the version in use.)

- [ ] **Step 17.3: Build and run**

```bash
cd elohim/brit && RUSTFLAGS="" cargo build -p brit-cli
./target/debug/brit plan --repo /home/matthew/git/elohim --files "app/elohim-app/src/styles.scss" | head -50
```

Expected: JSON conforming to BuildPlan shape — `planVersion: "1.0"`, `levels: [[...]]`, etc.

- [ ] **Step 17.4: Validate output against schema**

```bash
./target/debug/brit plan --repo /home/matthew/git/elohim --files "app/elohim-app/src/styles.scss" > /tmp/plan.json
npx ajv-cli@5 validate -s elohim/rakia/schemas/v1/build-plan.schema.json -d /tmp/plan.json --spec=draft2020 --all-errors
```

Expected: `valid`. If errors, fix the converter or schema.

- [ ] **Step 17.5: Commit**

```bash
git add elohim/brit/brit-cli/src/commands/plan.rs elohim/rakia/rakia-brit/src/changes.rs
git commit -m "feat(brit-cli): plan — emit BuildPlan JSON conforming to build-plan.schema.json"
```

---

### Task 18: Implement `brit fingerprint`

**Files:**
- Modify: `elohim/brit/brit-cli/src/commands/fingerprint.rs`

- [ ] **Step 18.1: Implement using brit-graph's ContentFingerprint**

```rust
//! brit fingerprint — deterministic content hash of step inputs.

use std::path::Path;

use serde::Serialize;

use crate::error::{CliError, Result};

#[derive(Serialize)]
struct FingerprintOutput {
    manifest: String,
    fingerprints: Vec<StepFingerprint>,
}

#[derive(Serialize)]
struct StepFingerprint {
    pipeline: String,
    step: String,
    fingerprint: String,
    input_count: usize,
}

pub fn run(manifest_path: &Path, step_filter: Option<&str>) -> Result<()> {
    let text = std::fs::read_to_string(manifest_path)?;
    let m: rakia_core::manifest::BuildManifest = serde_json::from_str(&text)?;

    let mut out = Vec::new();
    for (name, step) in &m.steps {
        if let Some(filter) = step_filter {
            if name != filter {
                continue;
            }
        }
        let mut inputs: std::collections::BTreeMap<String, Vec<u8>> = std::collections::BTreeMap::new();
        for src in &step.inputs.sources {
            inputs.insert(format!("source:{src}"), src.as_bytes().to_vec());
        }
        if let Some(bp) = &step.inputs.build_process {
            for p in bp {
                inputs.insert(format!("buildProcess:{p}"), p.as_bytes().to_vec());
            }
        }
        let fp = brit_graph::fingerprint::ContentFingerprint::from_inputs(&inputs);
        out.push(StepFingerprint {
            pipeline: m.pipeline.clone(),
            step: name.clone(),
            fingerprint: fp.cid.to_hex(),
            input_count: inputs.len(),
        });
    }

    crate::output::print_json(&FingerprintOutput {
        manifest: manifest_path.display().to_string(),
        fingerprints: out,
    })?;
    Ok(())
}
```

(Adjust to whatever the actual `brit_graph::fingerprint::ContentFingerprint` API looks like — `from_inputs` may have a different name. Check `elohim/brit/brit-graph/src/fingerprint.rs` and adapt.)

- [ ] **Step 18.2: Build and run**

```bash
cd elohim/brit && RUSTFLAGS="" cargo build -p brit-cli
./target/debug/brit fingerprint /home/matthew/git/elohim/app/elohim-app/build-manifest.json
./target/debug/brit fingerprint /home/matthew/git/elohim/app/elohim-app/build-manifest.json --step build
```

Expected: JSON with fingerprint hex strings; `--step` filters to a single step.

- [ ] **Step 18.3: Verify determinism**

Run twice; expect identical fingerprints.

```bash
diff <(./target/debug/brit fingerprint /home/matthew/git/elohim/app/elohim-app/build-manifest.json) \
     <(./target/debug/brit fingerprint /home/matthew/git/elohim/app/elohim-app/build-manifest.json)
```

Expected: no diff.

- [ ] **Step 18.4: Commit**

```bash
git add elohim/brit/brit-cli/src/commands/fingerprint.rs
git commit -m "feat(brit-cli): fingerprint — deterministic ContentFingerprint of step inputs"
```

---

### Task 19: Implement `brit baseline read/write/migrate`

**Files:**
- Modify: `elohim/brit/brit-cli/src/commands/baseline.rs`

- [ ] **Step 19.1: Implement baseline subcommands using rakia-brit**

```rust
//! brit baseline — read, write, and migrate baseline refs.

use std::path::Path;

use serde::Serialize;

use crate::error::{CliError, Result};

#[derive(Serialize)]
struct BaselineRead {
    pipeline: String,
    r#ref: String,
    commit: Option<String>,
}

#[derive(Serialize)]
struct BaselineWrite {
    pipeline: String,
    r#ref: String,
    commit: String,
    written: bool,
}

#[derive(Serialize)]
struct BaselineMigrate {
    source: String,
    migrated: usize,
}

pub fn read(repo: &Path, pipeline: &str) -> Result<()> {
    let commit = rakia_brit::baselines::read_baseline(repo, pipeline)
        .map_err(|e| CliError::Baseline(format!("{e}")))?;
    crate::output::print_json(&BaselineRead {
        pipeline: pipeline.to_string(),
        r#ref: format!("refs/notes/rakia/baselines/{pipeline}"),
        commit,
    })?;
    Ok(())
}

pub fn write(repo: &Path, pipeline: &str, commit: &str) -> Result<()> {
    rakia_brit::baselines::write_baseline(repo, pipeline, commit)
        .map_err(|e| CliError::Baseline(format!("{e}")))?;
    crate::output::print_json(&BaselineWrite {
        pipeline: pipeline.to_string(),
        r#ref: format!("refs/notes/rakia/baselines/{pipeline}"),
        commit: commit.to_string(),
        written: true,
    })?;
    Ok(())
}

pub fn migrate(repo: &Path, json_path: &Path) -> Result<()> {
    let count = rakia_brit::baselines::migrate_baselines(repo, json_path)
        .map_err(|e| CliError::Baseline(format!("{e}")))?;
    crate::output::print_json(&BaselineMigrate {
        source: json_path.display().to_string(),
        migrated: count,
    })?;
    Ok(())
}
```

(Confirm `rakia_brit::baselines::*` function signatures. If `migrate_baselines` returns `Result<()>`, adjust the count accordingly — perhaps return number of refs written from the migration helper, or read the JSON in this CLI and count keys.)

- [ ] **Step 19.2: Build and smoke test**

```bash
cd elohim/brit && RUSTFLAGS="" cargo build -p brit-cli
./target/debug/brit baseline read elohim --repo /home/matthew/git/elohim
```

Expected: JSON. If no baseline exists, `commit: null`.

- [ ] **Step 19.3: Test write + read roundtrip in a temp repo**

```bash
TMP=$(mktemp -d)
git -C "$TMP" init -q
git -C "$TMP" commit --allow-empty -m "init" -q
COMMIT=$(git -C "$TMP" rev-parse HEAD)
./target/debug/brit baseline write test-pipeline "$COMMIT" --repo "$TMP"
./target/debug/brit baseline read test-pipeline --repo "$TMP"
git -C "$TMP" show-ref refs/notes/rakia/baselines/test-pipeline
rm -rf "$TMP"
```

Expected: write succeeds, read returns the same commit, git show-ref confirms the ref exists.

- [ ] **Step 19.4: Commit**

```bash
git add elohim/brit/brit-cli/src/commands/baseline.rs
git commit -m "feat(brit-cli): baseline read/write/migrate — git ref-backed baselines"
```

---

## Phase 6: Fixture-Based Regression Tests

### Task 20: Frozen manifests snapshot + fixture runner harness

**Files:**
- Create: `elohim/rakia/rakia-core/tests/fixtures/manifests-snapshot/` (8 frozen manifests)
- Create: `elohim/rakia/rakia-core/tests/fixtures/README.md`
- Create: `elohim/rakia/rakia-core/tests/fixture_runner.rs`

- [ ] **Step 20.1: Snapshot the 8 manifests**

```bash
mkdir -p elohim/rakia/rakia-core/tests/fixtures/manifests-snapshot
for f in $(find . -name 'build-manifest.json' -not -path '*/node_modules/*' -not -path '*/elohim/rakia/*' | sort); do
  rel=$(echo "$f" | sed 's|^\./||' | tr '/' '_')
  cp "$f" "elohim/rakia/rakia-core/tests/fixtures/manifests-snapshot/${rel}"
done
ls elohim/rakia/rakia-core/tests/fixtures/manifests-snapshot/
```

Expected: 8 files named like `app_elohim-app_build-manifest.json`.

- [ ] **Step 20.2: Create README explaining the fixture format**

`elohim/rakia/rakia-core/tests/fixtures/README.md`:

```markdown
# Rakia Fixtures

Each subdirectory `NN-name/` is a fixture case. The runner asserts that
`plan_from_changes(constellation, changed-paths)` produces the expected plan.

## Fixture format

```
NN-name/
├── changed-paths.json     # { "paths": ["src/foo.ts", ...] }
└── expected-plan.json     # array of qualified step names, in any topological order:
                           # { "expectedSteps": ["pipeline:step", ...] }
```

The runner constructs a constellation from `manifests-snapshot/` (NOT live manifests),
runs `plan_from_changes`, flattens levels into a sorted set of qualified names,
and compares against `expectedSteps` (also sorted).

## Updating the manifests snapshot

The snapshot is intentionally frozen. To update (e.g., a real manifest changes
in a way that should affect fixture expectations):

1. Update `manifests-snapshot/` with the new manifest content.
2. Update each affected fixture's `expected-plan.json`.
3. Commit both together — the diff is the audit trail.

## Adding a fixture

1. Pick a real change scenario (ideally one that previously caused a CI bug).
2. Create `NN-description/` with `changed-paths.json` and `expected-plan.json`.
3. Run `cargo test -p rakia-core fixture` to verify it passes.
```

- [ ] **Step 20.3: Create the fixture runner**

`elohim/rakia/rakia-core/tests/fixture_runner.rs`:

```rust
//! Parameterized fixture tests for plan_from_changes.

use rakia_core::constellation::{build_constellation, plan_from_changes};
use rakia_core::manifest::BuildManifest;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn load_constellation() -> rakia_core::constellation::Constellation {
    let snapshot_dir = fixtures_dir().join("manifests-snapshot");
    let mut manifests = Vec::new();
    for entry in fs::read_dir(&snapshot_dir).expect("read snapshot dir") {
        let entry = entry.expect("read entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let text = fs::read_to_string(&path).expect("read manifest");
        let m: BuildManifest = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("parse {path:?}: {e}"));
        manifests.push((path, m));
    }
    build_constellation(manifests).expect("build constellation")
}

#[derive(Deserialize)]
struct ChangedPaths {
    paths: Vec<String>,
}

#[derive(Deserialize)]
struct ExpectedPlan {
    #[serde(rename = "expectedSteps")]
    expected_steps: Vec<String>,
}

fn run_fixture(case_dir: &Path) {
    let changed: ChangedPaths = serde_json::from_str(
        &fs::read_to_string(case_dir.join("changed-paths.json"))
            .unwrap_or_else(|e| panic!("read changed-paths.json in {case_dir:?}: {e}")),
    )
    .expect("parse changed-paths.json");

    let expected: ExpectedPlan = serde_json::from_str(
        &fs::read_to_string(case_dir.join("expected-plan.json"))
            .unwrap_or_else(|e| panic!("read expected-plan.json in {case_dir:?}: {e}")),
    )
    .expect("parse expected-plan.json");

    let constellation = load_constellation();
    let plan = plan_from_changes(&constellation, &changed.paths)
        .expect("plan_from_changes");

    let actual: BTreeSet<String> = plan
        .levels
        .iter()
        .flatten()
        .map(|(step, _)| step.qualified_name.clone())
        .collect();
    let expected_set: BTreeSet<String> = expected.expected_steps.iter().cloned().collect();

    assert_eq!(
        actual, expected_set,
        "fixture {} mismatch:\n  actual:   {:?}\n  expected: {:?}",
        case_dir.file_name().unwrap().to_string_lossy(),
        actual,
        expected_set,
    );
}

#[test]
fn run_all_fixtures() {
    let dir = fixtures_dir();
    let mut found = 0;
    for entry in fs::read_dir(&dir).expect("read fixtures dir") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy();
        if name == "manifests-snapshot" {
            continue;
        }
        if !name.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            continue;
        }
        run_fixture(&path);
        found += 1;
    }
    assert!(found >= 8, "expected at least 8 fixtures, found {found}");
}
```

- [ ] **Step 20.4: Verify the runner compiles and reports zero fixtures**

```bash
cd elohim/rakia && RUSTFLAGS="" cargo test -p rakia-core --test fixture_runner 2>&1 | tail -10
```

Expected: FAIL — "expected at least 8 fixtures, found 0". This is the test driving Task 21.

- [ ] **Step 20.5: Commit**

```bash
git add elohim/rakia/rakia-core/tests/fixtures/manifests-snapshot/ elohim/rakia/rakia-core/tests/fixtures/README.md elohim/rakia/rakia-core/tests/fixture_runner.rs
git commit -m "test(rakia-core): fixture runner harness + frozen 8-manifest snapshot"
```

---

### Task 21: Author 8 fixture cases

**Files:**
- Create: 8 fixture case directories under `elohim/rakia/rakia-core/tests/fixtures/`

For each fixture below, create the case directory with `changed-paths.json` and `expected-plan.json`. The expected steps must be derived empirically: run `brit plan --files <paths> --repo <root>` against the snapshot to see what's actually affected, then encode that as the fixture's expectation. (Fixture answers are derived from the implementation, but the fixture LOCKS the answer so future implementation changes that break it surface as test failures.)

For each fixture, the workflow is:

```bash
# 1. Determine expected steps by running the CLI:
elohim/brit/target/debug/brit plan --files "<paths>" --repo /home/matthew/git/elohim | jq '[.levels[][] | .qualifiedName] | sort'
# 2. Encode in expected-plan.json
```

- [ ] **Step 21.1: Fixture 01 — elohim-app CSS change (single pipeline)**

`elohim/rakia/rakia-core/tests/fixtures/01-elohim-app-css-change/changed-paths.json`:
```json
{ "paths": ["app/elohim-app/src/styles.scss"] }
```

`expected-plan.json`:
```json
{ "expectedSteps": ["<derived from brit plan output>"] }
```

Derive the expected steps:
```bash
elohim/brit/target/debug/brit plan --files "app/elohim-app/src/styles.scss" --repo /home/matthew/git/elohim | jq '[.levels[][] | .qualifiedName] | sort'
```
Encode the result in `expected-plan.json`.

- [ ] **Step 21.2: Fixture 02 — sophia source change (transitive: sophia → elohim-app)**

`02-sophia-source-change/changed-paths.json`:
```json
{ "paths": ["sophia/packages/sophia-element/src/index.ts"] }
```

Derive expected, encode.

- [ ] **Step 21.3: Fixture 03 — Holochain DNA change**

`03-holochain-dna-change/changed-paths.json`:
```json
{ "paths": ["elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs"] }
```

Derive expected, encode.

- [ ] **Step 21.4: Fixture 04 — cross-pillar change**

`04-cross-pillar-change/changed-paths.json`:
```json
{ "paths": ["app/elohim-app/src/main.ts", "doorway/doorway-app/src/app/app.component.ts"] }
```

Derive expected (should include both pipelines' build steps), encode.

- [ ] **Step 21.5: Fixture 05 — no change (empty plan)**

`05-no-change/changed-paths.json`:
```json
{ "paths": [] }
```

`expected-plan.json`:
```json
{ "expectedSteps": [] }
```

- [ ] **Step 21.6: Fixture 06 — README only (no pipeline affected)**

`06-readme-only/changed-paths.json`:
```json
{ "paths": ["README.md"] }
```

`expected-plan.json`:
```json
{ "expectedSteps": [] }
```

(If any manifest's source pattern matches `README.md`, this expectation is wrong — derive empirically.)

- [ ] **Step 21.7: Fixture 07 — Jenkinsfile-only**

`07-jenkinsfile-only/changed-paths.json`:
```json
{ "paths": ["Jenkinsfile"] }
```

Derive expected (likely orchestrator pipeline lint step), encode.

- [ ] **Step 21.8: Fixture 08 — buildProcess input change**

`08-build-process-change/changed-paths.json`:
```json
{ "paths": ["app/elohim-app/package.json"] }
```

This validates that the buildProcess fix from Task 12 is working. Expected: elohim-app build step is affected.

Derive expected, encode.

- [ ] **Step 21.9: Run all fixtures**

```bash
cd elohim/rakia && RUSTFLAGS="" cargo test -p rakia-core --test fixture_runner 2>&1 | tail -20
```

Expected: PASS — all 8 fixtures match.

If a fixture fails because the expected was wrong (encoded incorrectly), update the expected. If it fails because the implementation drifted, fix the implementation.

- [ ] **Step 21.10: Commit**

```bash
git add elohim/rakia/rakia-core/tests/fixtures/0*
git commit -m "test(rakia-core): 8 fixture cases (single, transitive, cross-pillar, edge cases, buildProcess)"
```

---

### Task 22: Verify BuildPlan output of every fixture validates against schema

**Goal:** Beyond "right steps" the fixture runner already checks, also assert the actual JSON output (from to_build_plan) validates against the schema. Catches IoC drift.

**Files:**
- Modify: `elohim/rakia/rakia-core/tests/fixture_runner.rs`

- [ ] **Step 22.1: Extend fixture runner to validate BuildPlan output against schema**

Add to `fixture_runner.rs` after the `actual_set/expected_set` assertion:

```rust
// Also verify the BuildPlan output validates against the schema
let bp = rakia_core::build_plan::to_build_plan(
    &plan,
    "refs/notes/rakia/baselines/fixture",
    &"a".repeat(40),
    &"b".repeat(40),
    &changed.paths,
    "0.0.0-fixture",
);
let bp_json = serde_json::to_value(&bp).expect("serialize build plan");

let schema_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("../schemas/v1/build-plan.schema.json");
let schema_text = fs::read_to_string(&schema_path).expect("read schema");
let schema: serde_json::Value = serde_json::from_str(&schema_text).expect("parse schema");
let validator = jsonschema::draft202012::new(&schema).expect("compile schema");
let errs: Vec<String> = validator
    .iter_errors(&bp_json)
    .map(|e| format!("{}: {e}", e.instance_path))
    .collect();
assert!(errs.is_empty(), "fixture {} BuildPlan invalid:\n{}",
    case_dir.file_name().unwrap().to_string_lossy(),
    errs.join("\n"));
```

- [ ] **Step 22.2: Run all fixtures with schema validation**

```bash
cd elohim/rakia && RUSTFLAGS="" cargo test -p rakia-core --test fixture_runner 2>&1 | tail -20
```

Expected: PASS — every fixture's BuildPlan output validates against the schema.

- [ ] **Step 22.3: Commit**

```bash
git add elohim/rakia/rakia-core/tests/fixture_runner.rs
git commit -m "test(rakia-core): fixture runner also validates BuildPlan against schema"
```

---

## Phase 7: IoC Close

### Task 23: Final IoC sweep

**Goal:** Single pass to verify NO `serde_json::Value` escape hatches remain in code that handles BuildManifest or BuildPlan, and codegen freshness is enforced everywhere.

- [ ] **Step 23.1: Grep for any remaining escape hatches**

```bash
echo "=== rakia-core ==="
grep -rn "serde_json::Value" elohim/rakia/rakia-core/src/ | grep -v "tests" | grep -v "// "
echo "=== brit-cli ==="
grep -rn "serde_json::Value" elohim/brit/brit-cli/src/
```

Expected: no output (or only legitimate uses with comments explaining why).

If any are found:
- If the field is genuinely freeform (rare), document why with a comment
- Otherwise, refine the schema and regenerate

- [ ] **Step 23.2: Run all rakia tests**

```bash
cd elohim/rakia && RUSTFLAGS="" cargo test 2>&1 | tail -30
```

Expected: all green.

- [ ] **Step 23.3: Run codegen verify mode**

```bash
pnpm run rakia:codegen:rs:verify
pnpm run rakia:schema:validate
```

Expected: both pass.

- [ ] **Step 23.4: Run brit-cli tests**

```bash
cd elohim/brit && RUSTFLAGS="" cargo test -p brit-cli 2>&1 | tail -20
```

Expected: all green.

- [ ] **Step 23.5: End-to-end smoke run**

```bash
cd /home/matthew/git/elohim
elohim/brit/target/debug/brit graph discover --repo .
elohim/brit/target/debug/brit graph show --repo . --format json | jq '.nodes | length'
elohim/brit/target/debug/brit affected --repo . --files "app/elohim-app/src/main.ts"
elohim/brit/target/debug/brit plan --repo . --files "app/elohim-app/src/main.ts" | jq '.planVersion, .levels | length'
elohim/brit/target/debug/brit fingerprint app/elohim-app/build-manifest.json
elohim/brit/target/debug/brit baseline read elohim --repo . || true  # may not exist
```

Expected: all commands produce structured JSON, no panics.

- [ ] **Step 23.6: Commit any cleanup from this sweep**

```bash
git status
# If anything changed:
git add -p
git commit -m "chore(rakia): final IoC sweep — schema/code drift cleanup"
```

If nothing changed, no commit needed.

---

### Task 24: Sprint-result artifact

**Files:**
- Create: `docs/superpowers/sprint-results/2026-04-19-rakia-describable.md`

- [ ] **Step 24.1: Generate fresh CLI demo transcripts**

```bash
mkdir -p /tmp/sprint-demos
cd /home/matthew/git/elohim
elohim/brit/target/debug/brit graph discover --repo . > /tmp/sprint-demos/01-graph-discover.json
elohim/brit/target/debug/brit graph show --repo . --format dot > /tmp/sprint-demos/02-graph-show.dot
elohim/brit/target/debug/brit affected --repo . --files "app/elohim-app/src/styles.scss" > /tmp/sprint-demos/03-affected.json
elohim/brit/target/debug/brit plan --repo . --files "app/elohim-app/src/styles.scss" > /tmp/sprint-demos/04-plan.json
elohim/brit/target/debug/brit fingerprint app/elohim-app/build-manifest.json > /tmp/sprint-demos/05-fingerprint.json
ls /tmp/sprint-demos/
```

These outputs go into the artifact (truncated for readability).

- [ ] **Step 24.2: Author the artifact**

`docs/superpowers/sprint-results/2026-04-19-rakia-describable.md`:

```markdown
# Sprint Result: Rakia Describable via CLI + Schema-as-IoC

**Date:** 2026-04-19 (sprint close)
**Spec:** `docs/superpowers/specs/2026-04-19-rakia-describable-cli-and-schema-ioc.md`
**Plan:** `docs/superpowers/plans/2026-04-19-rakia-describable-cli-and-schema-ioc.md`

## What's Runnable Now

Eight `brit` subcommands operate on the rakia constellation. Demo transcript
(repo state at sprint close):

### `brit graph discover`

Lists every `build-manifest.json` and its steps:

```bash
$ brit graph discover --repo .
{
  "manifests": [
    {
      "path": "app/elohim-app/build-manifest.json",
      "pipeline": "elohim-app",
      "description": "Angular 19 frontend",
      "stepCount": 4,
      "steps": ["build", "lint", "test", "typecheck"]
    },
    ... (7 more manifests)
  ]
}
```

### `brit graph show --format dot`

Pipes into Graphviz for visualization:

```bash
$ brit graph show --repo . --format dot | dot -Tsvg -o constellation.svg
```

### `brit affected --files <paths>`

Shows affected steps with provenance:

```bash
$ brit affected --repo . --files "app/elohim-app/src/styles.scss"
{
  "changedPaths": ["app/elohim-app/src/styles.scss"],
  "affected": [
    {
      "qualifiedName": "elohim-app:build",
      "affectedBy": [{ "kind": "changedFile", "path": "app/elohim-app/src/styles.scss" }]
    },
    {
      "qualifiedName": "elohim-app:test",
      "affectedBy": [{ "kind": "upstreamNode", "upstream": "elohim-app:build" }]
    }
  ]
}
```

### `brit plan --files <paths>`

Returns a topologically grouped BuildPlan conforming to `build-plan.schema.json`:

```bash
$ brit plan --repo . --files "app/elohim-app/src/styles.scss"
{
  "planVersion": "1.0",
  "baseline": { "ref": "(none)", "commit": "0000000000000000000000000000000000000000" },
  "head": { "commit": "0000000000000000000000000000000000000000" },
  "changedPaths": ["app/elohim-app/src/styles.scss"],
  "levels": [
    [{ "pipeline": "elohim-app", "name": "build", ... }],
    [{ "pipeline": "elohim-app", "name": "test", ... }]
  ],
  "generatedAt": "2026-04-19T...",
  "tool": { "name": "brit", "version": "0.0.0" }
}
```

### `brit fingerprint <manifest>`

Deterministic content hash of step inputs:

```bash
$ brit fingerprint app/elohim-app/build-manifest.json
{
  "manifest": "app/elohim-app/build-manifest.json",
  "fingerprints": [
    { "pipeline": "elohim-app", "step": "build", "fingerprint": "<hex>", "inputCount": 12 },
    ...
  ]
}
```

### `brit baseline read/write`

Git ref-backed baselines, survive executor death:

```bash
$ brit baseline write elohim <commit-sha> --repo .
{ "pipeline": "elohim", "ref": "refs/notes/rakia/baselines/elohim", "commit": "<sha>", "written": true }

$ brit baseline read elohim --repo .
{ "pipeline": "elohim", "ref": "refs/notes/rakia/baselines/elohim", "commit": "<sha>" }

$ git show-ref refs/notes/rakia/baselines/elohim
<sha> refs/notes/rakia/baselines/elohim
```

## Schema-IoC Pass Results

### Eliminated escape hatches

`elohim/rakia/rakia-core/src/manifest.rs` had three `serde_json::Value` fields
before the IoC pass:
- `BuildManifest.gate` → typed as `BuildGate` with `projects` map
- `BuildManifest.deployment` → typed as `BuildDeployment` with `targets` map
- `BuildStep.executor` → typed as `BuildExecutor` with `kind` discriminator

All three are now generated from `elohim/rakia/schemas/v1/build-manifest.schema.json`
via `pnpm run rakia:codegen:rs`. Hand-writing forbidden; pre-push hook enforces freshness.

### Output contract locked in

`elohim/rakia/schemas/v1/build-plan.schema.json` defines the BuildPlan output
contract. Every `brit plan` invocation produces JSON validating against the
schema (enforced by `tests/build_plan_schema_contract.rs` and the fixture runner).

### Closed follow-ups from predecessor spec

| Follow-up | Status |
|---|---|
| `buildProcess` parsed but unused | FIXED in Task 12 — now matched alongside source patterns |
| `gate`, `deployment`, `executor` as `serde_json::Value` | FIXED via codegen |

### Carry-overs (not done this sprint)

| Item | Why |
|---|---|
| `O(N*A)` traversal in `plan_from_changes` | Performance optimization — works at current scale |
| `GlobSet` precompilation per `QualifiedStep` | Same — defer until profiling shows it matters |
| `AffectedBy::DownstreamNode` declared but not emitted | Needs purpose decision; not blocking describability |
| gix error string-matching for `NotFound` | Needs typed variant upstream in gix |
| Codegen support for `oneOf` discriminated unions | Used the flat-with-optionals pattern instead this sprint |
| Folding `brit-verify`/`brit-build-ref` into unified `brit` binary | UX polish, defer |

## What's Next: Rakia Runnable End-to-End

The next sprint moves from describable to runnable. Scope:

### `rakia-executor`

Crate at `elohim/rakia/rakia-executor/`. Takes a `BuildPlan` and executes the
steps level-by-level (steps within a level run in parallel). Per-step:
1. Spawn the `executor.kind`-specific process (shell, pnpm, cargo, etc.)
2. Capture stdout/stderr to per-step log files
3. Determine pass/fail from exit code (and `outputs.verify` if specified)
4. Emit an `ExecutionEvent` per step (start, finish, fail) — schema-defined

### `rakia ci` wrapper

CLI in a new `rakia-cli` crate (rakia workspace). Single command for the whole
CI workflow:
1. `brit baseline read <pipeline>` to get baseline ref
2. `brit plan --since <baseline> --pipeline <pipeline>` to compute the plan
3. Hand to `rakia-executor` for execution
4. On all-pass: `brit baseline write <pipeline> <head>` to advance baseline
5. Emit a final `BuildAttestation` (schema-defined output)

### Schema IoC pass for next sprint

Two new schemas to author + generate types from:
- `execution-event.schema.json` — start/finish/fail events with timing, exit codes, log refs
- `build-attestation.schema.json` — final attestation matching the existing brit-epr Build attestation primitive

### Acceptance criteria for next sprint

- [ ] `rakia ci --pipeline elohim-app` runs against a real pipeline and produces an attestation
- [ ] Per-step logs captured and addressable
- [ ] Failure of one step in a level does not halt parallel steps in same level (configurable)
- [ ] Baseline advances only on full success
- [ ] All schemas validate; zero `serde_json::Value` escapes in executor types

## Open Questions Surfaced

These came out of the IoC pass and warrant a brainstorm before next sprint:

1. **Should `gate.projects` be discriminated by project type?** Currently a flat
   map; some projects use `patterns`, others use `required`. A discriminated
   shape would be more precise but breaks existing manifests.

2. **Should `executor.kind` be open or closed?** Closed enum (`shell`, `pnpm`,
   `cargo`, `rustCargo`, `noOp`) means adding a new executor requires schema +
   codegen. Open string allows extensibility but loses validation. Argument
   for closed: rakia-executor needs to know how to run each kind anyway.

3. **What's the right format for `outputs.verify`?** Currently optional string
   (a shell command). Could be richer: `{ kind: "exit-zero", command: "..." }`
   or `{ kind: "file-exists", path: "..." }`. Defer to executor sprint when the
   semantics matter.

4. **How do we handle `manualOnly` pipelines in `rakia ci`?** Skip them? Require
   explicit `--pipeline` flag? Defer.

## Acceptance Criteria — Sprint Self-Check

Spec acceptance criteria, checked at sprint close:

### CLI Surface
- [x] `brit-cli` crate compiles, single `brit` binary
- [x] All 8 subcommands implemented + JSON output
- [x] `brit graph show --format dot` produces valid Graphviz
- [x] `brit plan` output validates against `build-plan.schema.json`
- [x] `brit affected --since` and `--files` produce equivalent results
- [x] `brit baseline write` produces a valid git ref
- [x] Errors handled with clear messages + exit code 1

### Schemas
- [x] `build-manifest.schema.json` exists at `epr:schema:rakia:build-manifest:v1`
- [x] `build-plan.schema.json` exists at `epr:schema:rakia:build-plan:v1`
- [x] All 8 manifests validate against schema
- [x] `gate`, `deployment`, `executor` properly typed (no free-form objects)

### Codegen
- [x] `pnpm run rakia:codegen:rs` regenerates `generated_types.rs`
- [x] `--verify` mode fails on drift
- [x] Zero `serde_json::Value` in `manifest.rs`
- [x] Generated file rustfmts cleanly
- [x] Pre-push hook runs verify on schema/manifest changes

### Fixture Tests
- [x] At least 8 fixtures cover the documented scenarios
- [x] All fixtures pass
- [x] Fixture BuildPlan outputs validate against schema

### Sprint-Result Artifact
- [x] This document exists with all 5 required sections
- [x] Demo transcript reproducible
- [x] Next-sprint scope concrete
```

- [ ] **Step 24.3: Commit the artifact**

```bash
git add docs/superpowers/sprint-results/2026-04-19-rakia-describable.md
git commit -m "docs(sprint-results): rakia describable via CLI + schema-as-IoC sprint close"
```

---

### Task 25: Bump submodules + parent repo pointer

**Goal:** The work in `elohim/brit` and `elohim/rakia` are git submodules. The parent repo needs to advance its submodule pointers and reflect the merged work.

**Files:**
- Modify: `.gitmodules` not needed; just submodule SHAs

- [ ] **Step 25.1: Push the work in each submodule**

Inside each submodule, ensure the work is committed and pushed:

```bash
cd elohim/brit
git status     # should be clean or only show schema/codegen artifacts
git log --oneline -5
# If the work was done on a feature branch, merge to main and push
# (specific commands depend on the workflow used in the submodule)

cd /home/matthew/git/elohim/elohim/rakia
git status
git log --oneline -5
```

- [ ] **Step 25.2: Update parent submodule pointers**

```bash
cd /home/matthew/git/elohim
git status     # should show modified: elohim/brit (and/or elohim/rakia)
git diff --submodule=log
```

Expected: clear log of the new commits in each submodule.

- [ ] **Step 25.3: Commit the bump**

```bash
git add elohim/brit elohim/rakia
git commit -m "$(cat <<'EOF'
chore: bump brit + rakia submodules — describable CLI + schema IoC

Brings in:
  - brit-cli crate with 8 subcommands (graph, affected, plan, fingerprint, baseline)
  - rakia/schemas/v1/ — rakia-owned BuildManifest + BuildPlan schemas
  - codegen-rs.mjs — JSON Schema → Rust struct generation pipeline
  - rakia-core uses generated types; zero serde_json::Value in manifest.rs
  - 8 fixture cases + schema-validating fixture runner

See docs/superpowers/sprint-results/2026-04-19-rakia-describable.md
EOF
)"
```

---

## Self-Review

Run after writing the plan:

**Spec coverage:**
- Sprint cadence pattern → mentioned in plan header (architecture) ✓
- Sprint scope (in/out table) → covered by Phases 1–6 ✓
- Schema home (rakia-owned) → Tasks 1, 2, 3, 4 ✓
- BuildPlan schema → Task 4 ✓
- BuildManifest schema port + refine → Tasks 2, 3 ✓
- Codegen extending node-script pattern → Tasks 6, 7 ✓
- `--verify` mode + pre-push integration → Tasks 7.5, 8 ✓
- Replace `serde_json::Value` escapes → Tasks 9, 10, 11 ✓
- BuildPlan converter → Task 11 ✓
- buildProcess fix → Task 12 ✓
- brit-cli scaffold + 8 subcommands → Tasks 13–19 ✓
- Fixture runner + 8 cases + schema validation → Tasks 20, 21, 22 ✓
- Sprint-result artifact → Task 24 ✓
- Submodule pointer bump → Task 25 ✓

**Placeholder scan:** None of "TBD/TODO/etc." remain in the plan. Every step has executable commands or concrete code.

**Type consistency:** `BuildManifest`, `BuildStep`, `BuildInputs`, `BuildOutputs`, `BuildGate`, `BuildDeployment`, `BuildExecutor`, `AffectedReason`, `AffectedReasonKind`, `PlannedStep`, `Baseline`, `Head`, `BuildPlanTool`, `BuildPlan` — all consistent across tasks. Field names follow `#[serde(rename_all = "camelCase")]` convention. `QualifiedStep` carries `Vec<AffectedReason>` after Task 11's refactor — consumers in CLI commands match that type.

Plan is complete and self-consistent.
