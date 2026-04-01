# Build Graph Orchestrator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace pattern-based changeset analysis with a declarative build graph that determines the minimal rebuild set with zero guessing.

**Architecture:** Per-pipeline `build-manifest.json` files declare build steps with explicit inputs, outputs, and dependencies. The orchestrator discovers all manifests, composes them into a unified DAG, and walks it against the changeset to determine what's stale. Shadow mode runs alongside the existing PIPELINES logic first, then graduates to primary.

**Tech Stack:** Jenkins Pipeline (Groovy), JSON Schema, Node.js (validation), Git

**Design Spec:** `genesis/plans/2026-03-31-build-graph-orchestrator-design.md`

---

## Design Refinements (discovered during planning)

1. **6 manifests, not 7** — doorway-service and elohim-storage don't have their own Jenkinsfiles. Both are built by the edge pipeline (`elohim/holochain/Jenkinsfile`). The edge manifest includes cargo-build-doorway and cargo-build-storage steps.

2. **Whole-file hashing support** — The `buildProcess` field supports two reference patterns:
   - `Jenkinsfile@buildSophiaPlugin` — hash specific function body (for extracted helpers)
   - `Jenkinsfile` — hash entire file content (for inline stages)
   Angular/service-worker builds are currently inline in the root Jenkinsfile. Whole-file hash is less precise (any Jenkinsfile change triggers all steps) but correct and safe. Extracting inline stages to named functions is a future improvement, not part of this plan.

3. **Root cause confirmed** — Jenkinsfiles are in `ciOnlyPatterns` in the current orchestrator, so Jenkinsfile changes are explicitly skipped. The build graph model fixes this by treating Jenkinsfiles as build process inputs.

---

## File Structure

### New Files
| File | Purpose |
|------|---------|
| `genesis/orchestrator/manifest.schema.json` | JSON Schema for `build-manifest.json` format |
| `genesis/orchestrator/validate-manifests.mjs` | Node.js script to validate all manifests against schema |
| `genesis/orchestrator/build-graph.groovy` | Graph walker library (loaded by orchestrator Jenkinsfile) |
| `sophia/build-manifest.json` | Sophia pipeline manifest (1 step) |
| `elohim/holochain/dna/build-manifest.json` | DNA pipeline manifest (2 steps) |
| `app/elohim-app/build-manifest.json` | App pipeline manifest (3 steps) |
| `elohim/holochain/build-manifest.json` | Edge pipeline manifest (4 steps — doorway, storage, happ, image) |
| `genesis/build-manifest.json` | Genesis pipeline manifest (2 steps) |
| `steward/device/build-manifest.json` | Steward pipeline manifest (2 steps, manual-only) |

### Modified Files
| File | Change |
|------|--------|
| `genesis/orchestrator/Jenkinsfile` | Add shadow mode: load build-graph.groovy, run alongside PIPELINES, log comparison |
| `Jenkinsfile` (root) | Add `STEPS` parameter and `shouldRunStep()` helper, add `when` expressions to build stages |
| `elohim/holochain/Jenkinsfile` | Add `STEPS` parameter and `shouldRunStep()`, gate build stages |
| `elohim/holochain/dna/Jenkinsfile` | Add `STEPS` parameter and `shouldRunStep()`, gate build stages |
| `genesis/Jenkinsfile` | Add `STEPS` parameter and `shouldRunStep()`, gate build stages |
| `sophia.Jenkinsfile` | Add `STEPS` parameter and `shouldRunStep()`, gate build stages |
| `package.json` (root) | Add `validate:manifests` script |

---

### Task 1: Manifest JSON Schema

**Files:**
- Create: `genesis/orchestrator/manifest.schema.json`

- [ ] **Step 1: Create the JSON Schema**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://elohim.host/schemas/build-manifest/v1",
  "title": "Build Manifest",
  "description": "Declares build steps, inputs, outputs, and dependencies for a pipeline",
  "type": "object",
  "required": ["manifestVersion", "pipeline", "description", "steps"],
  "additionalProperties": false,
  "properties": {
    "manifestVersion": {
      "type": "string",
      "const": "1.0",
      "description": "Schema version for the manifest format"
    },
    "pipeline": {
      "type": "string",
      "pattern": "^[a-z][a-z0-9-]*$",
      "description": "Pipeline name — must match orchestrator's pipeline identifier"
    },
    "description": {
      "type": "string",
      "description": "Human-readable description of this pipeline's purpose"
    },
    "manualOnly": {
      "type": "boolean",
      "default": false,
      "description": "If true, pipeline is never auto-triggered by the orchestrator"
    },
    "steps": {
      "type": "object",
      "minProperties": 1,
      "additionalProperties": { "$ref": "#/$defs/step" },
      "propertyNames": {
        "pattern": "^[a-z][a-z0-9-]*$"
      },
      "description": "Map of step name to step definition"
    },
    "deployment": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "targets": {
          "type": "object",
          "additionalProperties": {
            "type": "object",
            "properties": {
              "healthCheck": { "type": "string", "format": "uri" }
            }
          }
        }
      }
    }
  },
  "$defs": {
    "step": {
      "type": "object",
      "required": ["description", "inputs", "outputs", "depends", "executor"],
      "additionalProperties": false,
      "properties": {
        "description": {
          "type": "string"
        },
        "inputs": {
          "type": "object",
          "required": ["sources", "buildProcess"],
          "additionalProperties": false,
          "properties": {
            "sources": {
              "type": "array",
              "items": { "type": "string" },
              "description": "Glob patterns for source files (relative to repo root)"
            },
            "buildProcess": {
              "type": "array",
              "items": {
                "type": "string",
                "pattern": "^[^@]+(@[a-zA-Z][a-zA-Z0-9_]*)?$"
              },
              "description": "References to build logic — 'File@functionName' for function hash, 'File' for whole-file hash"
            }
          }
        },
        "outputs": {
          "type": "object",
          "required": ["artifacts"],
          "additionalProperties": false,
          "properties": {
            "artifacts": {
              "type": "array",
              "items": { "type": "string" },
              "description": "Named output artifacts this step produces"
            },
            "verify": {
              "type": ["string", "null"],
              "description": "Shell command to confirm artifact was produced"
            }
          }
        },
        "depends": {
          "type": "array",
          "items": { "type": "string" },
          "description": "Step dependencies — bare name for same-manifest, 'pipeline:step' for cross-manifest"
        },
        "executor": {
          "type": "object",
          "required": ["stage"],
          "additionalProperties": false,
          "properties": {
            "stage": {
              "type": "string",
              "description": "Jenkins stage name"
            },
            "function": {
              "type": ["string", "null"],
              "description": "Jenkinsfile helper function name (null if inline stage)"
            }
          }
        }
      }
    }
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add genesis/orchestrator/manifest.schema.json
git commit -m "feat(orchestrator): add JSON Schema for build-manifest.json format"
```

---

### Task 2: Manifest Validation Script

**Files:**
- Create: `genesis/orchestrator/validate-manifests.mjs`
- Modify: `package.json` (root)

- [ ] **Step 1: Create validation script**

```javascript
#!/usr/bin/env node
// Validates all build-manifest.json files against the manifest schema.
// Also performs cross-manifest validation (dependency references, pipeline uniqueness).

import { readFileSync, existsSync } from 'fs';
import { resolve, dirname } from 'path';
import { execSync } from 'child_process';
import Ajv from 'ajv';

const ROOT = resolve(dirname(new URL(import.meta.url).pathname), '../..');
const SCHEMA_PATH = resolve(ROOT, 'genesis/orchestrator/manifest.schema.json');

// Discover all build-manifest.json files
const manifestPaths = execSync(
  "find . -name 'build-manifest.json' -not -path '*/node_modules/*' -not -path '*/.superpowers/*'",
  { cwd: ROOT, encoding: 'utf8' }
).trim().split('\n').filter(Boolean);

if (manifestPaths.length === 0) {
  console.error('ERROR: No build-manifest.json files found');
  process.exit(1);
}

// Load schema
const schema = JSON.parse(readFileSync(SCHEMA_PATH, 'utf8'));
const ajv = new Ajv({ allErrors: true });
const validate = ajv.compile(schema);

let errors = 0;
const manifests = [];

// Phase 1: Schema validation
console.log('=== Phase 1: Schema Validation ===\n');
for (const relPath of manifestPaths) {
  const absPath = resolve(ROOT, relPath);
  const content = JSON.parse(readFileSync(absPath, 'utf8'));

  if (validate(content)) {
    console.log(`  ✓ ${relPath}`);
    manifests.push({ path: relPath, content });
  } else {
    console.error(`  ✗ ${relPath}`);
    for (const err of validate.errors) {
      console.error(`    ${err.instancePath || '/'}: ${err.message}`);
    }
    errors++;
  }
}

// Phase 2: Cross-manifest validation
console.log('\n=== Phase 2: Cross-Manifest Validation ===\n');

// Check pipeline uniqueness
const pipelineNames = new Map();
for (const { path, content } of manifests) {
  const existing = pipelineNames.get(content.pipeline);
  if (existing) {
    console.error(`  ✗ Duplicate pipeline '${content.pipeline}' in ${path} and ${existing}`);
    errors++;
  } else {
    pipelineNames.set(content.pipeline, path);
  }
}
if (!errors) console.log('  ✓ No duplicate pipeline names');

// Collect all qualified step names
const allSteps = new Set();
for (const { content } of manifests) {
  for (const stepName of Object.keys(content.steps)) {
    allSteps.add(`${content.pipeline}:${stepName}`);
  }
}

// Validate dependency references
let depErrors = 0;
for (const { path, content } of manifests) {
  for (const [stepName, step] of Object.entries(content.steps)) {
    for (const dep of step.depends) {
      const qualified = dep.includes(':') ? dep : `${content.pipeline}:${dep}`;
      if (!allSteps.has(qualified)) {
        console.error(`  ✗ ${path}: step '${stepName}' depends on '${dep}' which does not exist`);
        depErrors++;
        errors++;
      }
    }
  }
}
if (!depErrors) console.log('  ✓ All dependency references resolve');

// Check for cycles (DFS)
const visited = new Set();
const inStack = new Set();
let hasCycle = false;

function dfs(node, pipeline) {
  const qualified = node.includes(':') ? node : `${pipeline}:${node}`;
  visited.add(qualified);
  inStack.add(qualified);

  // Find the step definition
  const [stepPipeline, stepName] = qualified.split(':');
  const manifest = manifests.find(m => m.content.pipeline === stepPipeline);
  if (!manifest) return;
  const step = manifest.content.steps[stepName];
  if (!step) return;

  for (const dep of step.depends) {
    const qualDep = dep.includes(':') ? dep : `${stepPipeline}:${dep}`;
    if (!visited.has(qualDep)) {
      dfs(qualDep, stepPipeline);
    } else if (inStack.has(qualDep)) {
      console.error(`  ✗ Cycle detected: ${qualified} -> ${qualDep}`);
      hasCycle = true;
      errors++;
    }
  }

  inStack.delete(qualified);
}

for (const step of allSteps) {
  if (!visited.has(step)) {
    const [pipeline] = step.split(':');
    dfs(step, pipeline);
  }
}
if (!hasCycle) console.log('  ✓ No dependency cycles');

// Check buildProcess file references
console.log('\n=== Phase 3: Build Process References ===\n');
let refErrors = 0;
for (const { path, content } of manifests) {
  for (const [stepName, step] of Object.entries(content.steps)) {
    for (const ref of step.inputs.buildProcess) {
      const fileName = ref.split('@')[0];
      const absFile = resolve(ROOT, fileName);
      if (!existsSync(absFile)) {
        console.error(`  ✗ ${path}: step '${stepName}' references '${fileName}' which does not exist`);
        refErrors++;
        errors++;
      } else {
        // If @functionName specified, check function exists
        if (ref.includes('@')) {
          const funcName = ref.split('@')[1];
          const fileContent = readFileSync(absFile, 'utf8');
          const funcPattern = new RegExp(`def\\s+${funcName}\\s*\\(`);
          if (!funcPattern.test(fileContent)) {
            console.error(`  ✗ ${path}: step '${stepName}' references function '${funcName}' not found in '${fileName}'`);
            refErrors++;
            errors++;
          }
        }
      }
    }
  }
}
if (!refErrors) console.log('  ✓ All buildProcess references resolve');

// Summary
console.log(`\n=== Summary: ${manifests.length} manifests, ${allSteps.size} steps, ${errors} errors ===`);
process.exit(errors > 0 ? 1 : 0);
```

- [ ] **Step 2: Add npm script to root package.json**

In the root `package.json`, add to `"scripts"`:

```json
"validate:manifests": "node genesis/orchestrator/validate-manifests.mjs"
```

- [ ] **Step 3: Verify ajv is available**

Run: `pnpm ls ajv --depth 0`

If not found, install: `pnpm add -Dw ajv`

- [ ] **Step 4: Run validation (expect failure — no manifests yet)**

Run: `pnpm run validate:manifests`

Expected: `ERROR: No build-manifest.json files found` (exit code 1)

- [ ] **Step 5: Commit**

```bash
git add genesis/orchestrator/validate-manifests.mjs package.json
git commit -m "feat(orchestrator): add build manifest validation script"
```

---

### Task 3: Build Manifests — Leaf Pipelines

Create manifests for pipelines with no cross-manifest dependencies.

**Files:**
- Create: `sophia/build-manifest.json`
- Create: `elohim/holochain/dna/build-manifest.json`

- [ ] **Step 1: Create sophia manifest**

```json
{
  "manifestVersion": "1.0",
  "pipeline": "elohim-sophia",
  "description": "Sophia assessment engine — monorepo build, UMD bundle for Angular",
  "steps": {
    "build-sophia-umd": {
      "description": "Build sophia-element UMD bundle for Angular consumption",
      "inputs": {
        "sources": [
          "sophia/packages/**",
          "sophia/package.json",
          "sophia/pnpm-lock.yaml",
          "sophia/tsconfig*.json"
        ],
        "buildProcess": [
          "sophia.Jenkinsfile"
        ]
      },
      "outputs": {
        "artifacts": ["sophia-element.umd.js"],
        "verify": "test -f sophia/packages/sophia-element/dist/sophia-element.umd.js"
      },
      "depends": [],
      "executor": {
        "stage": "Build",
        "function": null
      }
    }
  },
  "deployment": {}
}
```

- [ ] **Step 2: Create DNA manifest**

First, read `elohim/holochain/dna/Jenkinsfile` to identify the build stages:

```bash
grep -n "stage(" elohim/holochain/dna/Jenkinsfile | head -20
```

Then create the manifest. The DNA pipeline builds WASM integrity/coordinator zomes, then packages them into a .happ:

```json
{
  "manifestVersion": "1.0",
  "pipeline": "elohim-holochain",
  "description": "Holochain DNA compilation — WASM zomes and hApp packaging",
  "steps": {
    "build-dna-wasm": {
      "description": "Compile integrity and coordinator zomes to WASM",
      "inputs": {
        "sources": [
          "elohim/holochain/dna/**/*.rs",
          "elohim/holochain/dna/**/Cargo.toml",
          "elohim/holochain/dna/**/Cargo.lock",
          "elohim/holochain/dna/**/*.nix",
          "elohim/elohim-cache-core/**"
        ],
        "buildProcess": [
          "elohim/holochain/dna/Jenkinsfile"
        ]
      },
      "outputs": {
        "artifacts": ["dna-wasm"],
        "verify": null
      },
      "depends": [],
      "executor": {
        "stage": "Build DNA",
        "function": null
      }
    },
    "build-happ": {
      "description": "Package compiled zomes into .happ bundle",
      "inputs": {
        "sources": [
          "elohim/holochain/dna/**/happ.yaml",
          "elohim/holochain/dna/**/dna.yaml"
        ],
        "buildProcess": []
      },
      "outputs": {
        "artifacts": ["elohim.happ"],
        "verify": null
      },
      "depends": ["build-dna-wasm"],
      "executor": {
        "stage": "Package hApp",
        "function": null
      }
    }
  },
  "deployment": {}
}
```

Note: Verify exact stage names against `elohim/holochain/dna/Jenkinsfile` and adjust.

- [ ] **Step 3: Run validation**

Run: `pnpm run validate:manifests`

Expected: 2 manifests, all steps valid, no cross-manifest deps to check yet.

- [ ] **Step 4: Commit**

```bash
git add sophia/build-manifest.json elohim/holochain/dna/build-manifest.json
git commit -m "feat(orchestrator): add build manifests for sophia and DNA pipelines"
```

---

### Task 4: Build Manifests — Dependent Pipelines

Create manifests for pipelines that have cross-manifest dependencies.

**Files:**
- Create: `app/elohim-app/build-manifest.json`
- Create: `elohim/holochain/build-manifest.json`
- Create: `genesis/build-manifest.json`
- Create: `steward/device/build-manifest.json`

- [ ] **Step 1: Create app manifest**

First, identify the root Jenkinsfile build stages:

```bash
grep -n "stage(" Jenkinsfile | head -30
```

Create the manifest. The app pipeline builds Angular, service worker, and site image. Build logic is inline (no extracted helper functions except `buildSophiaPlugin`), so `buildProcess` references the whole Jenkinsfile:

```json
{
  "manifestVersion": "1.0",
  "pipeline": "elohim",
  "description": "Elohim Angular app — build, service worker, site image",
  "steps": {
    "build-angular": {
      "description": "Build Angular production bundle",
      "inputs": {
        "sources": [
          "app/elohim-app/src/**",
          "app/elohim-app/angular.json",
          "app/elohim-app/tsconfig*.json",
          "app/elohim-app/package.json",
          "app/elohim-app/vite.config.ts",
          "app/elohim-library/**"
        ],
        "buildProcess": [
          "Jenkinsfile"
        ]
      },
      "outputs": {
        "artifacts": ["elohim-app-dist"],
        "verify": "test -d app/elohim-app/dist/elohim-app"
      },
      "depends": ["elohim-sophia:build-sophia-umd"],
      "executor": {
        "stage": "Build App",
        "function": null
      }
    },
    "build-service-worker": {
      "description": "Compile and inject service worker into app bundle",
      "inputs": {
        "sources": [],
        "buildProcess": [
          "Jenkinsfile"
        ]
      },
      "outputs": {
        "artifacts": ["service-worker"],
        "verify": "test -f app/elohim-app/dist/elohim-app/browser/sw.js"
      },
      "depends": ["build-angular"],
      "executor": {
        "stage": "Build Service Worker",
        "function": null
      }
    },
    "build-site-image": {
      "description": "Package app into container image",
      "inputs": {
        "sources": [
          "app/elohim-app/Dockerfile",
          "app/elohim-app/nginx*.conf"
        ],
        "buildProcess": [
          "Jenkinsfile"
        ]
      },
      "outputs": {
        "artifacts": ["site-image"],
        "verify": null
      },
      "depends": ["build-service-worker"],
      "executor": {
        "stage": "Build Site Image",
        "function": null
      }
    }
  },
  "deployment": {
    "targets": {
      "alpha": { "healthCheck": "https://alpha.elohim.host/version" },
      "staging": { "healthCheck": "https://staging.elohim.host/version" }
    }
  }
}
```

Note: Verify exact stage names against the root `Jenkinsfile` and adjust. All three steps use whole-file `Jenkinsfile` reference because build logic is inline. Future improvement: extract build logic to named functions, then reference `Jenkinsfile@buildAngularApp` etc.

- [ ] **Step 2: Create edge manifest**

The edge pipeline (`elohim/holochain/Jenkinsfile`) builds doorway, storage, and packages the edge image. First identify stages:

```bash
grep -n "stage(" elohim/holochain/Jenkinsfile | head -30
```

```json
{
  "manifestVersion": "1.0",
  "pipeline": "elohim-edge",
  "description": "Edge services — doorway gateway, storage, conductor, edge image",
  "steps": {
    "cargo-build-doorway": {
      "description": "Build doorway gateway Rust binary",
      "inputs": {
        "sources": [
          "doorway/doorway-service/src/**",
          "doorway/doorway-service/Cargo.toml",
          "doorway/doorway-service/Cargo.lock",
          "doorway/doorway-client/**"
        ],
        "buildProcess": [
          "elohim/holochain/Jenkinsfile"
        ]
      },
      "outputs": {
        "artifacts": ["doorway-binary"],
        "verify": null
      },
      "depends": [],
      "executor": {
        "stage": "Build Doorway",
        "function": null
      }
    },
    "cargo-build-storage": {
      "description": "Build elohim-storage Rust binary",
      "inputs": {
        "sources": [
          "elohim/elohim-storage/src/**",
          "elohim/elohim-storage/Cargo.toml",
          "elohim/elohim-storage/Cargo.lock",
          "elohim/sdk/**"
        ],
        "buildProcess": [
          "elohim/holochain/Jenkinsfile"
        ]
      },
      "outputs": {
        "artifacts": ["storage-binary"],
        "verify": null
      },
      "depends": [],
      "executor": {
        "stage": "Build Storage",
        "function": null
      }
    },
    "build-edge-image": {
      "description": "Package doorway + storage + conductor into edge container image",
      "inputs": {
        "sources": [
          "doorway/doorway-service/Dockerfile",
          "elohim/holochain/*.yaml"
        ],
        "buildProcess": [
          "elohim/holochain/Jenkinsfile"
        ]
      },
      "outputs": {
        "artifacts": ["edge-image"],
        "verify": null
      },
      "depends": ["cargo-build-doorway", "cargo-build-storage", "elohim-holochain:build-happ"],
      "executor": {
        "stage": "Build Edge Image",
        "function": null
      }
    },
    "export-ts-bindings": {
      "description": "Generate TypeScript types from Rust view types",
      "inputs": {
        "sources": [
          "elohim/elohim-storage/src/**/views.rs"
        ],
        "buildProcess": []
      },
      "outputs": {
        "artifacts": ["storage-client-ts-types"],
        "verify": "test -d elohim/sdk/storage-client-ts/src/generated"
      },
      "depends": ["cargo-build-storage"],
      "executor": {
        "stage": "Export TS Bindings",
        "function": null
      }
    }
  },
  "deployment": {
    "targets": {
      "alpha": { "healthCheck": "https://alpha-edge.elohim.host/health" },
      "staging": { "healthCheck": "https://staging-edge.elohim.host/health" }
    }
  }
}
```

Note: Verify stage names and Dockerfile paths against `elohim/holochain/Jenkinsfile`.

- [ ] **Step 3: Create genesis manifest**

```bash
grep -n "stage(" genesis/Jenkinsfile | head -20
```

```json
{
  "manifestVersion": "1.0",
  "pipeline": "elohim-genesis",
  "description": "Content seeding and validation",
  "steps": {
    "validate-seeds": {
      "description": "Validate seed JSON against protocol schemas",
      "inputs": {
        "sources": [
          "genesis/seeds/**",
          "elohim/sdk/schemas/**",
          "elohim/sdk/domains/**"
        ],
        "buildProcess": [
          "genesis/Jenkinsfile"
        ]
      },
      "outputs": {
        "artifacts": ["validated-seeds"],
        "verify": null
      },
      "depends": [],
      "executor": {
        "stage": "Validate Seeds",
        "function": null
      }
    },
    "seed-content": {
      "description": "Seed validated content to target environment",
      "inputs": {
        "sources": [],
        "buildProcess": [
          "genesis/Jenkinsfile"
        ]
      },
      "outputs": {
        "artifacts": ["seeded-content"],
        "verify": null
      },
      "depends": [
        "validate-seeds",
        "elohim:build-site-image",
        "elohim-edge:build-edge-image",
        "elohim-edge:cargo-build-storage"
      ],
      "executor": {
        "stage": "Seed Content",
        "function": null
      }
    }
  },
  "deployment": {}
}
```

- [ ] **Step 4: Create steward manifest**

```json
{
  "manifestVersion": "1.0",
  "pipeline": "elohim-steward",
  "description": "Steward desktop app — Tauri + Holochain conductor",
  "manualOnly": true,
  "steps": {
    "cargo-build-steward": {
      "description": "Build steward Rust binary and Tauri app",
      "inputs": {
        "sources": [
          "steward/node/src/**",
          "steward/node/Cargo.toml",
          "steward/node/Cargo.lock",
          "steward/device/**",
          "crates/**"
        ],
        "buildProcess": [
          "steward/device/Jenkinsfile"
        ]
      },
      "outputs": {
        "artifacts": ["steward-binary"],
        "verify": null
      },
      "depends": [],
      "executor": {
        "stage": "Build Steward",
        "function": null
      }
    },
    "build-steward-app": {
      "description": "Package steward desktop app with embedded conductor",
      "inputs": {
        "sources": [
          "steward/device/src-tauri/**"
        ],
        "buildProcess": [
          "steward/device/Jenkinsfile"
        ]
      },
      "outputs": {
        "artifacts": ["steward-app"],
        "verify": null
      },
      "depends": ["cargo-build-steward", "elohim-holochain:build-happ"],
      "executor": {
        "stage": "Package Desktop App",
        "function": null
      }
    }
  },
  "deployment": {}
}
```

- [ ] **Step 5: Run validation**

Run: `pnpm run validate:manifests`

Expected: 6 manifests, all schema valid, all cross-manifest dependencies resolve, no cycles. Output:

```
=== Phase 1: Schema Validation ===
  ✓ sophia/build-manifest.json
  ✓ elohim/holochain/dna/build-manifest.json
  ✓ app/elohim-app/build-manifest.json
  ✓ elohim/holochain/build-manifest.json
  ✓ genesis/build-manifest.json
  ✓ steward/device/build-manifest.json

=== Phase 2: Cross-Manifest Validation ===
  ✓ No duplicate pipeline names
  ✓ All dependency references resolve
  ✓ No dependency cycles

=== Phase 3: Build Process References ===
  ✓ All buildProcess references resolve

=== Summary: 6 manifests, 15 steps, 0 errors ===
```

- [ ] **Step 6: Commit**

```bash
git add app/elohim-app/build-manifest.json elohim/holochain/build-manifest.json genesis/build-manifest.json steward/device/build-manifest.json
git commit -m "feat(orchestrator): add build manifests for app, edge, genesis, and steward pipelines"
```

---

### Task 5: Graph Walker — Discovery, Parsing, Composition

Create the core graph walker library that the orchestrator will load.

**Files:**
- Create: `genesis/orchestrator/build-graph.groovy`

- [ ] **Step 1: Create build-graph.groovy with discovery and parsing**

```groovy
// build-graph.groovy
// Build Graph Walker for Jenkins Orchestrator
//
// Discovers per-pipeline build-manifest.json files, composes them into
// a unified DAG, and walks it against a changeset to determine the
// minimal set of build steps needed. Zero guessing.
//
// Usage (from Jenkinsfile):
//   def buildGraph = load('genesis/orchestrator/build-graph.groovy')
//   def result = buildGraph.walkBuildGraph(changedFiles)

import groovy.json.JsonSlurper
import groovy.json.JsonOutput
import java.security.MessageDigest

// ============================================================
// DISCOVERY & PARSING
// ============================================================

/**
 * Discover and parse all build-manifest.json files in the workspace.
 * CPS-compatible (uses pipeline steps: sh, readFile).
 */
def discoverAndParseManifests() {
    def paths = sh(
        script: "find . -name 'build-manifest.json' -not -path '*/node_modules/*' -not -path '*/.superpowers/*' | sort",
        returnStdout: true
    ).trim().split('\n').findAll { it }

    echo "Found ${paths.size()} build manifests: ${paths.join(', ')}"

    def manifests = []
    for (def path : paths) {
        def content = readFile(file: path)
        def manifest = parseManifest(content, path)
        manifests.add(manifest)
    }
    return manifests
}

@NonCPS
def parseManifest(String content, String filePath) {
    def manifest = new JsonSlurper().parseText(content)
    manifest._filePath = filePath
    return manifest
}

// ============================================================
// COMPOSITION
// ============================================================

/**
 * Compose all manifests into a unified build graph.
 * Returns [steps: [:], pipelines: [:]] where each step has qualified name.
 */
@NonCPS
def composeGraph(List manifests) {
    def graph = [steps: [:], pipelines: [:]]

    for (def manifest : manifests) {
        def pipeline = manifest.pipeline
        if (graph.pipelines.containsKey(pipeline)) {
            throw new RuntimeException(
                "Duplicate pipeline name '${pipeline}' in ${manifest._filePath} " +
                "and ${graph.pipelines[pipeline]._filePath}"
            )
        }
        graph.pipelines[pipeline] = manifest

        manifest.steps.each { stepName, stepDef ->
            def qualifiedName = "${pipeline}:${stepName}"
            graph.steps[qualifiedName] = [
                pipeline: pipeline,
                localName: stepName,
                description: stepDef.description,
                inputs: stepDef.inputs,
                outputs: stepDef.outputs,
                depends: (stepDef.depends ?: []).collect { dep ->
                    // Qualify local references with pipeline name
                    dep.contains(':') ? dep : "${pipeline}:${dep}"
                },
                executor: stepDef.executor,
                manualOnly: manifest.manualOnly ?: false
            ]
        }
    }

    // Validate: every dependency target exists
    graph.steps.each { name, step ->
        step.depends.each { dep ->
            if (!graph.steps.containsKey(dep)) {
                throw new RuntimeException(
                    "Step '${name}' depends on '${dep}' which does not exist. " +
                    "Available steps: ${graph.steps.keySet().sort().join(', ')}"
                )
            }
        }
    }

    // Detect cycles
    detectCycles(graph)

    return graph
}

@NonCPS
def detectCycles(Map graph) {
    def visited = new HashSet()
    def inStack = new HashSet()

    for (def stepName : graph.steps.keySet()) {
        if (!visited.contains(stepName)) {
            dfsDetectCycle(graph, stepName, visited, inStack, [])
        }
    }
}

@NonCPS
def dfsDetectCycle(Map graph, String node, Set visited, Set inStack, List path) {
    visited.add(node)
    inStack.add(node)
    path = path + [node]

    def step = graph.steps[node]
    for (def dep : step.depends) {
        if (!visited.contains(dep)) {
            dfsDetectCycle(graph, dep, visited, inStack, path)
        } else if (inStack.contains(dep)) {
            def cycleStart = path.indexOf(dep)
            def cycle = path.subList(cycleStart, path.size()) + [dep]
            throw new RuntimeException("Dependency cycle detected: ${cycle.join(' -> ')}")
        }
    }

    inStack.remove(node)
}

return this
```

- [ ] **Step 2: Verify syntax**

Run: `groovy -e "evaluate(new File('genesis/orchestrator/build-graph.groovy'))" 2>&1 || echo "Syntax check (errors expected without Jenkins env)"`

If `groovy` is not installed, verify by reading the file and checking for obvious issues. The real validation happens in Jenkins.

- [ ] **Step 3: Commit**

```bash
git add genesis/orchestrator/build-graph.groovy
git commit -m "feat(orchestrator): add graph walker — discovery, parsing, composition"
```

---

### Task 6: Graph Walker — Change Detection & Staleness

Add change detection logic: source matching, build process hashing, staleness propagation.

**Files:**
- Modify: `genesis/orchestrator/build-graph.groovy`

- [ ] **Step 1: Add glob matching and source change detection**

Append before `return this`:

```groovy
// ============================================================
// CHANGE DETECTION
// ============================================================

/**
 * Test if a file path matches a glob pattern.
 * Supports **, *, and ? wildcards.
 */
@NonCPS
def matchesGlob(String filePath, String pattern) {
    // Normalize: remove leading ./
    def normalizedFile = filePath.startsWith('./') ? filePath.substring(2) : filePath
    def normalizedPattern = pattern.startsWith('./') ? pattern.substring(2) : pattern

    // Convert glob to regex
    def regex = normalizedPattern
        .replace('.', '\\.')
        .replace('**/','(.+/)?')
        .replace('**', '.*')
        .replace('*', '[^/]*')
        .replace('?', '[^/]')

    return normalizedFile.matches(regex)
}

/**
 * Check if any changed files match a step's source patterns.
 * Returns [stale: bool, reason: string]
 */
@NonCPS
def checkSourceChanges(List changedFiles, Map step) {
    def sources = step.inputs?.sources ?: []
    if (sources.isEmpty()) return [stale: false]

    for (def file : changedFiles) {
        for (def pattern : sources) {
            if (matchesGlob(file, pattern)) {
                return [stale: true, reason: "source: ${file} matches ${pattern}"]
            }
        }
    }
    return [stale: false]
}
```

- [ ] **Step 2: Add build process hashing**

Append before `return this`:

```groovy
/**
 * Extract a function body from file content by matching 'def funcName(...) {'
 * and counting braces to find the closing '}'.
 * Returns null if function not found.
 */
@NonCPS
def extractFunctionBody(String fileContent, String functionName) {
    def pattern = ~/def\s+${functionName}\s*\([^)]*\)\s*\{/
    def matcher = pattern.matcher(fileContent)
    if (!matcher.find()) return null

    int start = matcher.end()
    int depth = 1
    int pos = start
    while (pos < fileContent.length() && depth > 0) {
        char c = fileContent.charAt(pos)
        if (c == '{' as char) depth++
        else if (c == '}' as char) depth--
        pos++
    }
    return fileContent.substring(start, pos - 1).trim()
}

/**
 * SHA-256 hash a string, return hex digest.
 */
@NonCPS
def sha256(String content) {
    def digest = MessageDigest.getInstance('SHA-256')
    def hash = digest.digest(content.getBytes('UTF-8'))
    return hash.collect { String.format('%02x', it) }.join()
}

/**
 * Check if any build process references have changed for a step.
 * CPS-compatible (uses readFile for file content).
 * Returns [stale: bool, reason: string, hashes: [:]]
 */
def checkBuildProcessChanges(Map step, Map buildState, String qualifiedName) {
    def refs = step.inputs?.buildProcess ?: []
    if (refs.isEmpty()) return [stale: false, hashes: [:]]

    def currentHashes = [:]

    for (def ref : refs) {
        def parts = ref.split('@', 2)
        def fileName = parts[0]
        def funcName = parts.length > 1 ? parts[1] : null

        def fileContent
        try {
            fileContent = readFile(file: fileName)
        } catch (Exception e) {
            echo "WARNING: Cannot read '${fileName}' referenced by '${qualifiedName}': ${e.message}"
            return [stale: true, reason: "buildProcess: cannot read ${fileName}", hashes: [:]]
        }

        String contentToHash
        if (funcName) {
            // Hash specific function body
            contentToHash = extractFunctionBody(fileContent, funcName)
            if (contentToHash == null) {
                echo "WARNING: Function '${funcName}' not found in '${fileName}' (referenced by '${qualifiedName}')"
                return [stale: true, reason: "buildProcess: function ${funcName} not found in ${fileName}", hashes: [:]]
            }
        } else {
            // Hash entire file content
            contentToHash = fileContent
        }

        def currentHash = sha256(contentToHash)
        currentHashes[ref] = currentHash

        def previousHash = buildState?.stepStates?.get(qualifiedName)?.buildProcessHashes?.get(ref)
        if (previousHash == null || currentHash != previousHash) {
            def label = funcName ? "${fileName}@${funcName}" : fileName
            return [stale: true, reason: "buildProcess: ${label} hash changed", hashes: currentHashes]
        }
    }

    return [stale: false, hashes: currentHashes]
}
```

- [ ] **Step 3: Add staleness propagation**

Append before `return this`:

```groovy
/**
 * Propagate staleness through dependency edges.
 * If any dependency is stale, the dependent step is also stale.
 * Fixed-point iteration handles transitive deps.
 */
@NonCPS
def propagateStaleness(Map graph, Map staleMap) {
    def changed = true
    while (changed) {
        changed = false
        graph.steps.each { name, step ->
            if (staleMap[name]?.stale) return // already stale

            def staleDep = step.depends.find { dep -> staleMap[dep]?.stale }
            if (staleDep) {
                staleMap[name] = [stale: true, reason: "depends: ${staleDep}"]
                changed = true
            }
        }
    }
    return staleMap
}

/**
 * Run full change detection for all steps in the graph.
 * CPS-compatible (calls checkBuildProcessChanges which uses readFile).
 * Returns staleMap: [qualifiedName: [stale: bool, reason: string, hashes: [:]]]
 */
def detectAllStaleness(Map graph, List changedFiles, Map buildState) {
    def staleMap = [:]
    def allHashes = [:]

    // Phase 1: Check sources and build process for each step
    for (def entry : graph.steps.entrySet()) {
        def name = entry.key
        def step = entry.value

        // Skip manual-only pipelines from auto-trigger analysis
        // (they're still in the graph for dependency validation)

        // Check sources first
        def sourceResult = checkSourceChanges(changedFiles, step)
        if (sourceResult.stale) {
            staleMap[name] = sourceResult
            continue
        }

        // Check build process
        def processResult = checkBuildProcessChanges(step, buildState, name)
        allHashes[name] = processResult.hashes
        if (processResult.stale) {
            staleMap[name] = [stale: true, reason: processResult.reason]
            continue
        }

        staleMap[name] = [stale: false]
    }

    // Phase 2: Propagate through dependencies
    staleMap = propagateStaleness(graph, staleMap)

    return [staleMap: staleMap, buildProcessHashes: allHashes]
}
```

- [ ] **Step 4: Commit**

```bash
git add genesis/orchestrator/build-graph.groovy
git commit -m "feat(orchestrator): add change detection — source matching, build process hashing, staleness propagation"
```

---

### Task 7: Graph Walker — Execution Planning & Decision Matrix

Add topological sort, parallel level grouping, pipeline grouping, and decision matrix formatting.

**Files:**
- Modify: `genesis/orchestrator/build-graph.groovy`

- [ ] **Step 1: Add execution planning functions**

Append before `return this`:

```groovy
// ============================================================
// EXECUTION PLANNING
// ============================================================

/**
 * Topologically sort stale steps and group into parallel execution levels.
 * Steps in the same level have no inter-dependencies and can run in parallel.
 */
@NonCPS
def topoSortAndLevel(Map graph, Set staleSteps) {
    if (staleSteps.isEmpty()) return []

    def levels = []
    def placed = new HashSet()
    def remaining = new HashSet(staleSteps)

    while (!remaining.isEmpty()) {
        def level = remaining.findAll { name ->
            def step = graph.steps[name]
            // All dependencies either: not stale (already built), or already placed in earlier level
            step.depends.every { dep ->
                !remaining.contains(dep) || placed.contains(dep)
            }
        } as Set

        if (level.isEmpty()) {
            // Should not happen (cycles detected earlier), but safety valve
            echo "WARNING: Cannot resolve remaining steps, adding all: ${remaining}"
            levels.add(remaining.sort() as List)
            break
        }

        levels.add(level.sort() as List)
        placed.addAll(level)
        remaining.removeAll(level)
    }

    return levels
}

/**
 * Group stale steps by pipeline for triggering.
 * Returns [pipelineName: [stepLocalName, ...]]
 */
@NonCPS
def groupByPipeline(Set staleSteps, Map graph) {
    def pipelineSteps = [:]
    staleSteps.each { name ->
        def step = graph.steps[name]
        def pipeline = step.pipeline
        if (!pipelineSteps.containsKey(pipeline)) {
            pipelineSteps[pipeline] = []
        }
        pipelineSteps[pipeline].add(step.localName)
    }
    return pipelineSteps
}
```

- [ ] **Step 2: Add decision matrix formatting**

Append before `return this`:

```groovy
// ============================================================
// DECISION MATRIX
// ============================================================

/**
 * Format the build graph decision matrix for console output.
 * Shows every step, its status (BUILD/SKIP/MANUAL), and the reason.
 */
@NonCPS
def formatDecisionMatrix(Map graph, Map staleMap) {
    def lines = []
    lines.add('╔══════════════════════════════════════════════════════════════════════════╗')
    lines.add('║                       BUILD GRAPH DECISION MATRIX                        ║')
    lines.add('╠══════════════════════════════════════════════════════════════════════════╣')

    // Sort by pipeline then step name for consistent output
    def sortedSteps = graph.steps.entrySet().sort { a, b ->
        def cmp = a.value.pipeline <=> b.value.pipeline
        cmp != 0 ? cmp : a.key <=> b.key
    }

    def currentPipeline = ''
    for (def entry : sortedSteps) {
        def name = entry.key
        def step = entry.value
        def info = staleMap[name] ?: [stale: false]

        // Pipeline separator
        if (step.pipeline != currentPipeline) {
            currentPipeline = step.pipeline
            def pipelineHeader = "║ [${currentPipeline}]"
            lines.add(pipelineHeader.padRight(76) + '║')
        }

        def status
        def icon
        if (step.manualOnly) {
            status = 'MANUAL'
            icon = '🔒'
        } else if (info.stale) {
            status = 'BUILD '
            icon = '🔨'
        } else {
            status = 'SKIP  '
            icon = '⏭️ '
        }

        def reason = info.stale ? info.reason : 'no changes'
        if (step.manualOnly && info.stale) {
            reason = "would build (${info.reason}) but manual-only"
        }

        def displayName = step.localName.padRight(26)
        def line = "║ ${icon} ${displayName}│ ${status} │ ${reason}"
        // Truncate reason if too long
        if (line.length() > 75) {
            line = line.substring(0, 72) + '...'
        }
        lines.add(line.padRight(76) + '║')
    }

    lines.add('╚══════════════════════════════════════════════════════════════════════════╝')
    return lines.join('\n')
}

/**
 * Format a comparison matrix showing PIPELINES vs Build Graph results.
 * Used during shadow mode to surface divergences.
 */
@NonCPS
def formatComparisonMatrix(Map pipelinesAnalysis, Map graphStaleMap, Map graph) {
    def lines = []
    lines.add('╔══════════════════════════════════════════════════════════════════════════╗')
    lines.add('║                    CHANGESET ANALYSIS COMPARISON                         ║')
    lines.add('╠══════════════════════════════════════════════════════════════════════════╣')
    lines.add('║ Pipeline               │ PIPELINES │ Build Graph │ Match? │ Detail       ║')
    lines.add('╟────────────────────────┼───────────┼─────────────┼────────┼──────────────╢')

    // Aggregate graph results per pipeline
    def graphPipelines = [:]
    graph.steps.each { name, step ->
        def pipeline = step.pipeline
        if (!graphPipelines.containsKey(pipeline)) {
            graphPipelines[pipeline] = [shouldBuild: false, reasons: []]
        }
        def info = graphStaleMap[name]
        if (info?.stale && !step.manualOnly) {
            graphPipelines[pipeline].shouldBuild = true
            graphPipelines[pipeline].reasons.add(info.reason)
        }
    }

    // Compare
    def divergences = 0
    def allPipelines = (pipelinesAnalysis.keySet() + graphPipelines.keySet()).sort().unique()

    for (def pipeline : allPipelines) {
        def pResult = pipelinesAnalysis[pipeline]?.shouldRun ?: false
        def gResult = graphPipelines[pipeline]?.shouldBuild ?: false
        def match = (pResult == gResult)
        if (!match) divergences++

        def pStatus = pResult ? 'BUILD' : 'SKIP '
        def gStatus = gResult ? 'BUILD' : 'SKIP '
        def matchIcon = match ? '  ✓   ' : '  ✗   '
        def detail = ''
        if (!match && gResult) {
            detail = graphPipelines[pipeline].reasons.take(1).join()
        }
        if (!match && !gResult && pResult) {
            detail = 'PIPELINES false positive?'
        }

        def name = pipeline.padRight(23)
        def line = "║ ${name}│ ${pStatus}    │ ${gStatus}       │${matchIcon}│ ${detail}"
        if (line.length() > 75) line = line.substring(0, 72) + '...'
        lines.add(line.padRight(76) + '║')
    }

    lines.add('╚══════════════════════════════════════════════════════════════════════════╝')

    if (divergences > 0) {
        lines.add("⚠️  ${divergences} DIVERGENCE(S) detected between PIPELINES and Build Graph")
    } else {
        lines.add("✓ PIPELINES and Build Graph agree on all pipelines")
    }

    return lines.join('\n')
}
```

- [ ] **Step 3: Commit**

```bash
git add genesis/orchestrator/build-graph.groovy
git commit -m "feat(orchestrator): add execution planning — topo sort, leveling, decision matrix"
```

---

### Task 8: Graph Walker — Build State Persistence & Main Entry Point

Add build state load/save and the main `walkBuildGraph()` function.

**Files:**
- Modify: `genesis/orchestrator/build-graph.groovy`

- [ ] **Step 1: Add build state persistence**

Append before `return this`:

```groovy
// ============================================================
// BUILD STATE PERSISTENCE
// ============================================================

/**
 * Load the previous build state from Jenkins artifacts.
 * CPS-compatible (uses copyArtifacts, readFile).
 */
def loadBuildState() {
    try {
        copyArtifacts(
            projectName: env.JOB_NAME,
            selector: lastSuccessful(),
            filter: 'build-state.json',
            optional: true,
            fingerprintArtifacts: false
        )
        if (fileExists('build-state.json')) {
            def content = readFile(file: 'build-state.json')
            return parseJson(content)
        }
    } catch (Exception e) {
        echo "No previous build state found: ${e.message}"
    }
    return [version: '1.0', lastSuccessfulCommit: null, stepStates: [:]]
}

@NonCPS
def parseJson(String content) {
    return new JsonSlurper().parseText(content)
}

/**
 * Save build state as a Jenkins artifact.
 * Updates step hashes for all steps that were analyzed.
 */
def saveBuildState(Map graph, Map staleMap, Map buildProcessHashes, String commitHash) {
    def stepStates = [:]

    graph.steps.each { name, step ->
        def info = staleMap[name]
        stepStates[name] = [
            lastBuiltCommit: info?.stale ? commitHash : (info?.lastBuiltCommit ?: null),
            buildProcessHashes: buildProcessHashes[name] ?: [:],
            outputVerified: false
        ]
    }

    def state = [
        version: '1.0',
        lastSuccessfulCommit: commitHash,
        stepStates: stepStates
    ]

    writeFile(file: 'build-state.json', text: serializeJson(state))
    archiveArtifacts(artifacts: 'build-state.json', fingerprint: true)
}

@NonCPS
def serializeJson(Object data) {
    return JsonOutput.prettyPrint(JsonOutput.toJson(data))
}
```

- [ ] **Step 2: Add main entry point**

Append before `return this`:

```groovy
// ============================================================
// MAIN ENTRY POINT
// ============================================================

/**
 * Walk the build graph to determine which steps need rebuilding.
 *
 * @param changedFiles List of changed file paths (relative to repo root)
 * @return Map with keys: graph, staleMap, staleSteps, levels, pipelineSteps
 */
def walkBuildGraph(List changedFiles) {
    echo '=== Build Graph Walker ==='
    echo "Analyzing ${changedFiles.size()} changed files"

    // 1. Discover & parse manifests
    def manifests = discoverAndParseManifests()
    echo "Parsed ${manifests.size()} build manifests"

    // 2. Compose unified graph
    def graph = composeGraph(manifests)
    echo "Composed graph: ${graph.steps.size()} steps across ${graph.pipelines.size()} pipelines"

    // 3. Load previous build state
    def buildState = loadBuildState()
    if (buildState.lastSuccessfulCommit) {
        echo "Previous build state: commit ${buildState.lastSuccessfulCommit}, ${buildState.stepStates.size()} step hashes"
    } else {
        echo "No previous build state — all steps with buildProcess references will be marked stale"
    }

    // 4. Detect staleness (sources + buildProcess + propagation)
    def detection = detectAllStaleness(graph, changedFiles, buildState)
    def staleMap = detection.staleMap
    def buildProcessHashes = detection.buildProcessHashes

    // 5. Print decision matrix
    echo formatDecisionMatrix(graph, staleMap)

    // 6. Compute execution plan
    def staleSteps = staleMap.findAll { k, v -> v.stale && !graph.steps[k].manualOnly }.keySet() as Set
    def manualStaleSteps = staleMap.findAll { k, v -> v.stale && graph.steps[k].manualOnly }.keySet() as Set

    def levels = topoSortAndLevel(graph, staleSteps)
    def pipelineSteps = groupByPipeline(staleSteps, graph)

    echo "Rebuild set: ${staleSteps.size()} steps across ${pipelineSteps.size()} pipelines"
    if (levels) {
        levels.eachWithIndex { level, i ->
            echo "  Level ${i}: ${level.join(', ')}"
        }
    }
    if (manualStaleSteps) {
        echo "Manual-only steps with changes (not auto-triggered): ${manualStaleSteps.join(', ')}"
    }

    return [
        graph: graph,
        staleMap: staleMap,
        staleSteps: staleSteps,
        levels: levels,
        pipelineSteps: pipelineSteps,
        buildProcessHashes: buildProcessHashes
    ]
}
```

- [ ] **Step 3: Verify the complete file**

Read the entire `genesis/orchestrator/build-graph.groovy` file end-to-end and verify:
- All sections are present: Discovery, Composition, Change Detection, Execution Planning, Decision Matrix, Build State, Main Entry
- File ends with `return this`
- No syntax errors (matching braces, proper Groovy)
- `@NonCPS` on all methods that use regex, JsonSlurper, MessageDigest, or Set/Map operations
- No `@NonCPS` on methods that use `readFile`, `sh`, `writeFile`, `copyArtifacts`, `echo`, `fileExists`

- [ ] **Step 4: Commit**

```bash
git add genesis/orchestrator/build-graph.groovy
git commit -m "feat(orchestrator): add build state persistence and walkBuildGraph entry point"
```

---

### Task 9: Shadow Mode Integration in Orchestrator

Wire the build graph walker into the orchestrator Jenkinsfile to run alongside the existing PIPELINES analysis, compare results, and log divergences.

**Files:**
- Modify: `genesis/orchestrator/Jenkinsfile`

- [ ] **Step 1: Read the orchestrator Jenkinsfile**

Read `genesis/orchestrator/Jenkinsfile` to find:
1. Where `analyzeChangeset()` is called (to get the changedFiles list)
2. Where `analyzePipelineRequirements()` is called (to get the PIPELINES result)
3. Where `printDecisionMatrix()` is called (to add the comparison after it)
4. The stage structure — which stage to add the shadow mode to

- [ ] **Step 2: Add shadow mode function to orchestrator**

Add this helper function in the HELPER METHODS section of `genesis/orchestrator/Jenkinsfile`:

```groovy
/**
 * Run build graph walker in shadow mode alongside PIPELINES analysis.
 * Compares results and logs divergences without affecting build decisions.
 */
def runBuildGraphShadow(List changedFiles, Map pipelinesAnalysis) {
    try {
        def buildGraph = load('genesis/orchestrator/build-graph.groovy')
        def result = buildGraph.walkBuildGraph(changedFiles)

        // Print comparison matrix
        echo buildGraph.formatComparisonMatrix(pipelinesAnalysis, result.staleMap, result.graph)

        // Save build state for next run (even in shadow mode, to build up hash baseline)
        def commitHash = getGitCommitHash()
        buildGraph.saveBuildState(result.graph, result.staleMap, result.buildProcessHashes, commitHash)

    } catch (Exception e) {
        echo "⚠️ Build Graph shadow mode failed (non-blocking): ${e.message}"
        echo "Stack trace: ${e.stackTrace.take(10).collect { it.toString() }.join('\n')}"
    }
}
```

- [ ] **Step 3: Wire shadow mode into the orchestrator pipeline**

Find the stage that calls `printDecisionMatrix()` and add the shadow mode call after it. The exact location depends on the orchestrator's stage structure. Look for the pattern:

```groovy
// After the existing decision matrix:
printDecisionMatrix(pipelineAnalysis, ...)

// Add shadow mode:
echo '\n=== Build Graph Shadow Mode ==='
runBuildGraphShadow(changedFiles, pipelineAnalysis)
```

The `changedFiles` variable is the list returned by `analyzeChangeset()`. The `pipelineAnalysis` is the map returned by `analyzePipelineRequirements()`. Wire into the existing variables — don't re-compute.

- [ ] **Step 4: Commit**

```bash
git add genesis/orchestrator/Jenkinsfile
git commit -m "feat(orchestrator): wire build graph walker in shadow mode alongside PIPELINES"
```

---

### Task 10: Pipeline Step Gating — Root Jenkinsfile

Add `STEPS` parameter and `shouldRunStep()` to the root Jenkinsfile so individual build stages can be skipped.

**Files:**
- Modify: `Jenkinsfile` (root)

- [ ] **Step 1: Read the root Jenkinsfile parameters section**

Read the root `Jenkinsfile` to find:
1. The `parameters { }` block (to add `STEPS`)
2. The build stages (`Build App`, `Build Sophia Plugin`, service worker, site image) to add `when` expressions

- [ ] **Step 2: Add STEPS parameter**

In the `parameters { }` block, add:

```groovy
string(name: 'STEPS', defaultValue: 'all', description: 'Comma-separated list of build steps to run (from build-manifest.json). "all" runs everything.')
```

- [ ] **Step 3: Add shouldRunStep helper**

In the STAGE HELPER METHODS section, add:

```groovy
/**
 * Check if a build step should run based on the STEPS parameter.
 * Returns true if STEPS is 'all' (default) or contains the step name.
 * Used by build stages to skip work the orchestrator determined is unnecessary.
 */
def shouldRunStep(String stepName) {
    def steps = (params.STEPS ?: 'all').split(',').collect { it.trim() }
    return steps.contains('all') || steps.contains(stepName)
}
```

- [ ] **Step 4: Add when expressions to build stages**

For each build stage, add a `when` block. Find the stage names by reading the Jenkinsfile. The pattern for each stage:

```groovy
stage('Build Sophia Plugin') {
    when { expression { shouldRunStep('build-sophia-umd') } }
    // ... existing steps unchanged
}

stage('Build App') {  // or whatever the Angular build stage is named
    when { expression { shouldRunStep('build-angular') } }
    // ... existing steps unchanged
}

stage('Build Service Worker') {
    when { expression { shouldRunStep('build-service-worker') } }
    // ... existing steps unchanged
}

stage('Build Site Image') {
    when { expression { shouldRunStep('build-site-image') } }
    // ... existing steps unchanged
}
```

Match the step names in `when` to the step names in `app/elohim-app/build-manifest.json`. The step names in the manifest are the canonical identifiers.

**Important:** Do NOT add `when` to test stages, deploy stages, or utility stages — only build stages. The `STEPS` parameter controls what gets BUILT, not what gets tested or deployed.

- [ ] **Step 5: Test fallback behavior**

Verify that `STEPS=all` (default) runs all stages — identical to current behavior. This is the safety property: without the orchestrator passing specific steps, everything runs.

Read through the modified Jenkinsfile and confirm:
1. `shouldRunStep('any-step-name')` returns `true` when `STEPS` is null (first build, null params)
2. `shouldRunStep('any-step-name')` returns `true` when `STEPS` is 'all'
3. `shouldRunStep('build-angular')` returns `true` when `STEPS` is 'build-angular,build-site-image'
4. `shouldRunStep('build-angular')` returns `false` when `STEPS` is 'build-service-worker'

- [ ] **Step 6: Commit**

```bash
git add Jenkinsfile
git commit -m "feat(ci): add STEPS parameter and shouldRunStep gating to app pipeline"
```

---

### Task 11: Pipeline Step Gating — Remaining Pipelines

Add the same `STEPS` parameter and `shouldRunStep()` pattern to all other pipeline Jenkinsfiles.

**Files:**
- Modify: `elohim/holochain/Jenkinsfile` (edge)
- Modify: `elohim/holochain/dna/Jenkinsfile` (DNA)
- Modify: `genesis/Jenkinsfile` (genesis)
- Modify: `sophia.Jenkinsfile` (sophia)

- [ ] **Step 1: Read each Jenkinsfile to identify build stages**

For each file, find the `parameters { }` block and the build stages:

```bash
grep -n "parameters\|stage(" elohim/holochain/Jenkinsfile | head -30
grep -n "parameters\|stage(" elohim/holochain/dna/Jenkinsfile | head -30
grep -n "parameters\|stage(" genesis/Jenkinsfile | head -30
grep -n "parameters\|stage(" sophia.Jenkinsfile | head -30
```

- [ ] **Step 2: Add STEPS parameter and shouldRunStep to each Jenkinsfile**

For each file, apply the same pattern as Task 10:

1. Add `string(name: 'STEPS', defaultValue: 'all', ...)` to `parameters { }`
2. Add `shouldRunStep()` function
3. Add `when { expression { shouldRunStep('step-name') } }` to build stages

Step names must match the corresponding `build-manifest.json`:

**elohim/holochain/Jenkinsfile** (edge):
- `shouldRunStep('cargo-build-doorway')` on the doorway build stage
- `shouldRunStep('cargo-build-storage')` on the storage build stage
- `shouldRunStep('build-edge-image')` on the image build stage
- `shouldRunStep('export-ts-bindings')` on the TS export stage

**elohim/holochain/dna/Jenkinsfile** (DNA):
- `shouldRunStep('build-dna-wasm')` on the WASM build stage
- `shouldRunStep('build-happ')` on the hApp packaging stage

**genesis/Jenkinsfile** (genesis):
- `shouldRunStep('validate-seeds')` on the validation stage
- `shouldRunStep('seed-content')` on the seeding stage

**sophia.Jenkinsfile** (sophia):
- `shouldRunStep('build-sophia-umd')` on the build stage

- [ ] **Step 3: Commit**

```bash
git add elohim/holochain/Jenkinsfile elohim/holochain/dna/Jenkinsfile genesis/Jenkinsfile sophia.Jenkinsfile
git commit -m "feat(ci): add STEPS parameter and shouldRunStep gating to all pipelines"
```

---

### Task 12: End-to-End Validation Plan

Document how to validate the full system before transitioning from shadow to primary.

**Files:**
- No files changed — this is a validation checklist

- [ ] **Step 1: Validate manifests locally**

Run: `pnpm run validate:manifests`

Expected: 6 manifests, 15 steps, 0 errors.

- [ ] **Step 2: Push and trigger orchestrator**

Push the branch. The orchestrator should:
1. Run the existing PIPELINES analysis (unchanged)
2. Run the build graph shadow mode
3. Print both the PIPELINES decision matrix AND the build graph decision matrix
4. Print the comparison matrix showing any divergences

Check the build log for:
- `=== Build Graph Walker ===` section
- `=== BUILD GRAPH DECISION MATRIX ===`
- `=== CHANGESET ANALYSIS COMPARISON ===`
- Whether divergences are detected

- [ ] **Step 3: Test three scenarios**

After the initial shadow mode is working, test these scenarios by making targeted changes:

**Scenario A — Jenkinsfile build process change:**
Change the root `Jenkinsfile` (e.g., add a comment to `buildSophiaPlugin`). Push.
- PIPELINES expected: SKIP (Jenkinsfiles are in ciOnlyPatterns)
- Build Graph expected: BUILD (Jenkinsfile hash changed)
- This divergence proves the build graph catches what PIPELINES misses.

**Scenario B — Source-only change:**
Change a file in `app/elohim-app/src/`. Push.
- PIPELINES expected: BUILD elohim
- Build Graph expected: BUILD build-angular → build-service-worker → build-site-image
- Both should agree on the pipeline level.

**Scenario C — Cross-manifest dependency:**
Change a file in `sophia/packages/`. Push.
- PIPELINES expected: BUILD elohim-sophia, BUILD elohim (cascades)
- Build Graph expected: BUILD build-sophia-umd, BUILD build-angular (depends), → build-service-worker → build-site-image
- Both should agree, but build graph provides step-level granularity.

- [ ] **Step 4: Monitor shadow mode over N builds**

Watch for:
- False negatives (build graph says SKIP but PIPELINES says BUILD and it was needed)
- False positives (build graph says BUILD when nothing actually changed)
- Errors in manifest parsing or graph composition

Track divergences. When shadow mode shows zero false negatives for 10+ builds, the build graph is ready to become primary (Phase 2 of migration plan).

- [ ] **Step 5: Commit validation notes**

Add observations to the design doc or create a follow-up tracking issue.

```bash
# If any manifest adjustments were needed during validation:
git add **/build-manifest.json
git commit -m "fix(orchestrator): adjust build manifests based on shadow mode validation"
```
