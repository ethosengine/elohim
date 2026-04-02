# Husky Graph Walker Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the pre-push hook's grep-based project detection with a manifest-driven graph walker that reads `build-manifest.json` files, achieving change-detection parity with the Jenkins orchestrator.

**Architecture:** A shared `manifest-utils.mjs` provides manifest discovery/loading. A `graph-walker.mjs` module takes changed files, matches them against manifest source globs, propagates staleness through dependency edges, and maps stale steps to gate project names via a new `gate` field in each manifest. The pre-push hook calls the walker CLI and falls back to current grep logic if it fails.

**Tech Stack:** Node.js (ESM), picomatch (glob matching), node:test (testing), JSON Schema

**Design Spec:** `genesis/plans/2026-04-01-husky-graph-walker-design.md`

**Branch:** `feature/build-graph-orchestrator` (where existing manifests live)

---

## File Structure

### New Files
| File | Purpose |
|------|---------|
| `genesis/orchestrator/manifest-utils.mjs` | Shared manifest discovery, loading, and step resolution |
| `genesis/orchestrator/graph-walker.mjs` | Graph walker: changed files -> affected gate projects |
| `genesis/orchestrator/graph-walker.test.mjs` | Unit tests for graph walker |
| `sophia/build-manifest.json` | Sophia pipeline manifest (stub) |
| `doorway/doorway-app/build-manifest.json` | Doorway app manifest (stub) |
| `elohim/elohim-compute/build-manifest.json` | Elohim compute manifest (stub) |
| `genesis/orchestrator/build-manifest.json` | Orchestrator manifest (stub) |

### Modified Files
| File | Change |
|------|--------|
| `genesis/orchestrator/manifest.schema.json` | Add optional `gate` field |
| `genesis/orchestrator/validate-manifests.mjs` | Refactor to import from manifest-utils.mjs |
| `app/elohim-app/build-manifest.json` | Add `lint-library` step + gate mapping |
| `elohim/holochain/build-manifest.json` | Add gate mapping (doorway + storage) |
| `elohim/holochain/dna/build-manifest.json` | Add `schema-dna` step + gate mapping |
| `genesis/build-manifest.json` | Add quality steps + gate mapping |
| `steward/device/build-manifest.json` | Add gate mapping |
| `.husky/pre-push` | Replace project detection with walker call + fallback |
| `package.json` | Add picomatch devDependency |

---

### Task 1: Add `gate` Field to Manifest Schema

**Files:**
- Modify: `genesis/orchestrator/manifest.schema.json`

- [ ] **Step 1: Add the gate property to the schema**

Add the `gate` property inside `properties`, after `deployment`:

```json
    "gate": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "projects": {
          "type": "object",
          "additionalProperties": {
            "type": "object",
            "required": ["dir"],
            "additionalProperties": false,
            "properties": {
              "dir": {
                "type": "string",
                "description": "Working directory for running the quality gate"
              },
              "steps": {
                "type": "array",
                "items": { "type": "string" },
                "description": "Which manifest steps trigger this gate project (default: all steps)"
              }
            }
          },
          "description": "Map of hook project name to gate config"
        }
      }
    }
```

- [ ] **Step 2: Verify schema is valid JSON**

Run: `node -e "JSON.parse(require('fs').readFileSync('genesis/orchestrator/manifest.schema.json', 'utf8')); console.log('OK')"`

Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add genesis/orchestrator/manifest.schema.json
git commit -m "feat(ci): add gate field to build manifest schema

Supports mapping manifest steps to pre-push hook quality gate projects."
```

---

### Task 2: Create `manifest-utils.mjs`

**Files:**
- Create: `genesis/orchestrator/manifest-utils.mjs`

- [ ] **Step 1: Create the shared module**

```js
#!/usr/bin/env node
// Shared utilities for build manifest discovery, loading, and resolution.
// Used by both validate-manifests.mjs and graph-walker.mjs.

import { readFileSync } from 'fs';
import { resolve } from 'path';
import { execSync } from 'child_process';

/**
 * Discover all build-manifest.json files under rootDir.
 * Returns relative paths (e.g., './app/elohim-app/build-manifest.json').
 */
export function discoverManifests(rootDir) {
  const output = execSync(
    "find . -name 'build-manifest.json' -not -path '*/node_modules/*' -not -path '*/.superpowers/*'",
    { cwd: rootDir, encoding: 'utf8' }
  );
  return output.trim().split('\n').filter(Boolean);
}

/**
 * Discover and parse all build-manifest.json files.
 * Returns [{ path, content }].
 */
export function loadManifests(rootDir) {
  const paths = discoverManifests(rootDir);
  return paths.map(relPath => {
    const absPath = resolve(rootDir, relPath);
    const content = JSON.parse(readFileSync(absPath, 'utf8'));
    return { path: relPath, content };
  });
}

/**
 * Normalize a dependency reference to qualified form.
 * 'build-angular' + 'elohim' -> 'elohim:build-angular'
 * 'elohim-sophia:build-sophia-umd' -> 'elohim-sophia:build-sophia-umd' (unchanged)
 */
export function resolveStep(dep, currentPipeline) {
  return dep.includes(':') ? dep : `${currentPipeline}:${dep}`;
}
```

- [ ] **Step 2: Verify it loads**

Run: `node -e "import('./genesis/orchestrator/manifest-utils.mjs').then(m => console.log(Object.keys(m)))"`

Expected: `[ 'discoverManifests', 'loadManifests', 'resolveStep' ]`

- [ ] **Step 3: Commit**

```bash
git add genesis/orchestrator/manifest-utils.mjs
git commit -m "feat(ci): extract manifest-utils.mjs from validate-manifests

Shared discovery, loading, and step resolution for manifest consumers."
```

---

### Task 3: Create `graph-walker.mjs` with Tests

**Files:**
- Create: `genesis/orchestrator/graph-walker.mjs`
- Create: `genesis/orchestrator/graph-walker.test.mjs`
- Modify: `package.json` (add picomatch)

- [ ] **Step 1: Add picomatch as explicit devDependency**

Run: `pnpm add -wD picomatch`

- [ ] **Step 2: Write the test file**

```js
import { describe, it } from 'node:test';
import { strict as assert } from 'node:assert';
import { walkGraph, topoSort } from './graph-walker.mjs';
import { resolveStep } from './manifest-utils.mjs';

// ── Helpers ──────────────────────────────────────────────────────

function makeManifest(pipeline, steps, gate) {
  return {
    path: `${pipeline}/build-manifest.json`,
    content: {
      manifestVersion: '1.0',
      pipeline,
      description: `Test manifest for ${pipeline}`,
      steps,
      ...(gate ? { gate } : {}),
    },
  };
}

function makeStep(sources = [], depends = [], buildProcess = []) {
  return {
    description: 'test step',
    inputs: { sources, buildProcess },
    outputs: { artifacts: ['test-artifact'], verify: null },
    depends,
    executor: { stage: 'Test', function: null },
  };
}

// ── resolveStep ──────────────────────────────────────────────────

describe('resolveStep', () => {
  it('qualifies bare step names', () => {
    assert.equal(resolveStep('build-angular', 'elohim'), 'elohim:build-angular');
  });

  it('passes through already-qualified names', () => {
    assert.equal(resolveStep('elohim-sophia:build-sophia-umd', 'elohim'), 'elohim-sophia:build-sophia-umd');
  });
});

// ── Source glob matching ─────────────────────────────────────────

describe('source glob matching', () => {
  it('matches a file against a glob pattern', () => {
    const manifests = [
      makeManifest('app', {
        build: makeStep(['app/src/**']),
      }, { projects: { 'my-app': { dir: 'app' } } }),
    ];
    const result = walkGraph(manifests, ['app/src/main.ts']);
    assert.equal(result.projects.length, 1);
    assert.equal(result.projects[0].name, 'my-app');
    assert.ok(result.projects[0].reasons.some(r => r.startsWith('source:')));
  });

  it('does not match unrelated files', () => {
    const manifests = [
      makeManifest('app', {
        build: makeStep(['app/src/**']),
      }, { projects: { 'my-app': { dir: 'app' } } }),
    ];
    const result = walkGraph(manifests, ['other/file.txt']);
    assert.equal(result.projects.length, 0);
  });

  it('matches tsconfig glob patterns', () => {
    const manifests = [
      makeManifest('app', {
        build: makeStep(['app/tsconfig*.json']),
      }, { projects: { 'my-app': { dir: 'app' } } }),
    ];
    const result = walkGraph(manifests, ['app/tsconfig.app.json']);
    assert.equal(result.projects.length, 1);
  });
});

// ── BuildProcess file matching ───────────────────────────────────

describe('buildProcess matching', () => {
  it('matches whole-file references', () => {
    const manifests = [
      makeManifest('app', {
        build: makeStep([], [], ['Jenkinsfile']),
      }, { projects: { 'my-app': { dir: 'app' } } }),
    ];
    const result = walkGraph(manifests, ['Jenkinsfile']);
    assert.equal(result.projects.length, 1);
    assert.ok(result.projects[0].reasons.some(r => r.includes('buildProcess: Jenkinsfile')));
  });

  it('matches @function references by file', () => {
    const manifests = [
      makeManifest('app', {
        build: makeStep([], [], ['Jenkinsfile@buildAngularApp']),
      }, { projects: { 'my-app': { dir: 'app' } } }),
    ];
    const result = walkGraph(manifests, ['Jenkinsfile']);
    assert.equal(result.projects.length, 1);
    assert.ok(result.projects[0].reasons.some(r => r.includes('buildProcess: Jenkinsfile@buildAngularApp')));
  });

  it('does not match when referenced file is not changed', () => {
    const manifests = [
      makeManifest('app', {
        build: makeStep([], [], ['Jenkinsfile']),
      }, { projects: { 'my-app': { dir: 'app' } } }),
    ];
    const result = walkGraph(manifests, ['src/main.ts']);
    assert.equal(result.projects.length, 0);
  });
});

// ── Dependency propagation ───────────────────────────────────────

describe('dependency propagation', () => {
  it('marks dependent steps stale when dependency is stale', () => {
    const manifests = [
      makeManifest('lib', {
        build: makeStep(['lib/src/**']),
      }, { projects: { lib: { dir: 'lib' } } }),
      makeManifest('app', {
        build: makeStep([], ['lib:build']),
      }, { projects: { app: { dir: 'app' } } }),
    ];
    const result = walkGraph(manifests, ['lib/src/index.ts']);
    assert.equal(result.projects.length, 2);
    const appProject = result.projects.find(p => p.name === 'app');
    assert.ok(appProject);
    assert.ok(appProject.reasons.some(r => r.includes('depends:')));
  });

  it('propagates staleness transitively (A -> B -> C)', () => {
    const manifests = [
      makeManifest('a', {
        build: makeStep(['a/**']),
      }, { projects: { a: { dir: 'a' } } }),
      makeManifest('b', {
        build: makeStep([], ['a:build']),
      }, { projects: { b: { dir: 'b' } } }),
      makeManifest('c', {
        build: makeStep([], ['b:build']),
      }, { projects: { c: { dir: 'c' } } }),
    ];
    const result = walkGraph(manifests, ['a/file.rs']);
    assert.equal(result.projects.length, 3);
    assert.ok(result.projects.find(p => p.name === 'c'));
  });

  it('propagates within same manifest (bare dep names)', () => {
    const manifests = [
      makeManifest('app', {
        compile: makeStep(['app/src/**']),
        bundle: makeStep([], ['compile']),
      }, { projects: { app: { dir: 'app' } } }),
    ];
    const result = walkGraph(manifests, ['app/src/main.ts']);
    assert.equal(result.projects.length, 1);
    // Both steps stale, but maps to one gate project
    assert.equal(result.projects[0].name, 'app');
  });
});

// ── Gate mapping ─────────────────────────────────────────────────

describe('gate mapping', () => {
  it('maps steps to specific gate projects via steps field', () => {
    const manifests = [
      makeManifest('edge', {
        'build-doorway': makeStep(['doorway/**']),
        'build-storage': makeStep(['storage/**']),
      }, {
        projects: {
          doorway: { dir: 'doorway/service', steps: ['build-doorway'] },
          storage: { dir: 'elohim/storage', steps: ['build-storage'] },
        },
      }),
    ];
    // Only doorway files changed
    const result = walkGraph(manifests, ['doorway/src/main.rs']);
    assert.equal(result.projects.length, 1);
    assert.equal(result.projects[0].name, 'doorway');
    assert.equal(result.projects[0].dir, 'doorway/service');
  });

  it('triggers all gate projects when steps is omitted', () => {
    const manifests = [
      makeManifest('app', {
        build: makeStep(['app/**']),
      }, { projects: { 'my-app': { dir: 'app' } } }),
    ];
    const result = walkGraph(manifests, ['app/src/main.ts']);
    assert.equal(result.projects.length, 1);
    assert.equal(result.projects[0].name, 'my-app');
  });

  it('returns empty when no gate field exists', () => {
    const manifests = [
      makeManifest('app', { build: makeStep(['app/**']) }),
    ];
    const result = walkGraph(manifests, ['app/src/main.ts']);
    assert.equal(result.projects.length, 0);
  });

  it('returns empty when no manifests provided', () => {
    const result = walkGraph([], ['app/src/main.ts']);
    assert.equal(result.projects.length, 0);
  });
});

// ── Topological output ordering ──────────────────────────────────

describe('output ordering', () => {
  it('orders dependencies before dependents', () => {
    const manifests = [
      makeManifest('sophia', {
        build: makeStep(['sophia/**']),
      }, { projects: { sophia: { dir: 'sophia' } } }),
      makeManifest('app', {
        build: makeStep(['app/**'], ['sophia:build']),
      }, { projects: { app: { dir: 'app' } } }),
    ];
    // Both changed
    const result = walkGraph(manifests, ['sophia/src/x.ts', 'app/src/y.ts']);
    assert.equal(result.projects.length, 2);
    assert.equal(result.projects[0].name, 'sophia');
    assert.equal(result.projects[1].name, 'app');
  });

  it('stable order for independent projects', () => {
    const manifests = [
      makeManifest('aaa', {
        build: makeStep(['aaa/**']),
      }, { projects: { aaa: { dir: 'aaa' } } }),
      makeManifest('bbb', {
        build: makeStep(['bbb/**']),
      }, { projects: { bbb: { dir: 'bbb' } } }),
    ];
    const result = walkGraph(manifests, ['aaa/x.ts', 'bbb/y.ts']);
    assert.equal(result.projects.length, 2);
    // Independent — should appear in some stable order
  });
});
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `node --test genesis/orchestrator/graph-walker.test.mjs`

Expected: Failures — `graph-walker.mjs` doesn't exist yet.

- [ ] **Step 4: Write the graph walker module**

```js
#!/usr/bin/env node
// Graph walker: matches changed files against build manifest source globs,
// propagates staleness through dependency edges, and maps to gate projects.
//
// Library usage: import { walkGraph } from './graph-walker.mjs'
// CLI usage: echo "file1\nfile2" | node graph-walker.mjs

import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import picomatch from 'picomatch';
import { loadManifests, resolveStep } from './manifest-utils.mjs';

/**
 * Topologically sort steps using Kahn's algorithm.
 * Returns qualified step names in dependency order (dependencies first).
 */
export function topoSort(stepIndex) {
  const inDegree = new Map();
  const adj = new Map();

  for (const qualified of stepIndex.keys()) {
    inDegree.set(qualified, 0);
    adj.set(qualified, []);
  }

  for (const [qualified, { step, pipeline }] of stepIndex) {
    for (const dep of step.depends) {
      const qualDep = resolveStep(dep, pipeline);
      if (!stepIndex.has(qualDep)) continue;
      adj.get(qualDep).push(qualified);
      inDegree.set(qualified, inDegree.get(qualified) + 1);
    }
  }

  const queue = [];
  for (const [node, deg] of inDegree) {
    if (deg === 0) queue.push(node);
  }

  const order = [];
  while (queue.length > 0) {
    const node = queue.shift();
    order.push(node);
    for (const neighbor of adj.get(node)) {
      const newDeg = inDegree.get(neighbor) - 1;
      inDegree.set(neighbor, newDeg);
      if (newDeg === 0) queue.push(neighbor);
    }
  }

  return order;
}

/**
 * Walk the build graph to determine which gate projects are affected by changed files.
 *
 * @param {Array<{path: string, content: object}>} manifests - Loaded manifests
 * @param {string[]} changedFiles - List of changed file paths (relative to repo root)
 * @returns {{ projects: Array<{name: string, dir: string, reasons: string[]}> }}
 */
export function walkGraph(manifests, changedFiles) {
  if (manifests.length === 0) return { projects: [] };

  // Phase 1: Build index
  const stepIndex = new Map();
  for (const { content } of manifests) {
    for (const [name, step] of Object.entries(content.steps)) {
      const qualified = `${content.pipeline}:${name}`;
      stepIndex.set(qualified, { step, pipeline: content.pipeline, manifest: content });
    }
  }

  // Phase 2: Mark stale (source globs + buildProcess files)
  const stale = new Map();

  for (const [qualified, { step }] of stepIndex) {
    const reasons = [];

    for (const pattern of step.inputs.sources) {
      const matcher = picomatch(pattern);
      for (const file of changedFiles) {
        if (matcher(file)) {
          reasons.push(`source: ${file}`);
          break;
        }
      }
    }

    for (const ref of step.inputs.buildProcess) {
      const fileName = ref.split('@')[0];
      if (changedFiles.includes(fileName)) {
        reasons.push(`buildProcess: ${ref}`);
      }
    }

    if (reasons.length > 0) {
      stale.set(qualified, reasons);
    }
  }

  // Phase 3: Propagate staleness in dependency order
  const order = topoSort(stepIndex);

  for (const qualified of order) {
    const { step, pipeline } = stepIndex.get(qualified);
    for (const dep of step.depends) {
      const qualDep = resolveStep(dep, pipeline);
      if (stale.has(qualDep)) {
        if (!stale.has(qualified)) {
          stale.set(qualified, [`depends: ${qualDep}`]);
        } else {
          stale.get(qualified).push(`depends: ${qualDep}`);
        }
      }
    }
  }

  // Phase 4: Map stale steps to gate projects
  const projectMap = new Map();

  for (const { content } of manifests) {
    if (!content.gate?.projects) continue;

    for (const [projectName, config] of Object.entries(content.gate.projects)) {
      const triggerSteps = config.steps || Object.keys(content.steps);
      const reasons = [];
      let minOrder = Infinity;

      for (const stepName of triggerSteps) {
        const qualified = `${content.pipeline}:${stepName}`;
        if (stale.has(qualified)) {
          reasons.push(...stale.get(qualified));
          const idx = order.indexOf(qualified);
          if (idx >= 0 && idx < minOrder) minOrder = idx;
        }
      }

      if (reasons.length > 0) {
        projectMap.set(projectName, { dir: config.dir, reasons, minOrder });
      }
    }
  }

  // Sort by dependency order (lowest topo-sort index first)
  const projects = [...projectMap.entries()]
    .sort((a, b) => a[1].minOrder - b[1].minOrder)
    .map(([name, { dir, reasons }]) => ({ name, dir, reasons }));

  return { projects };
}

// ── CLI mode ─────────────────────────────────────────────────────

const isMain = import.meta.url === `file://${process.argv[1]}` ||
               import.meta.url === `file://${resolve(process.argv[1])}`;

if (isMain) {
  const ROOT = resolve(dirname(new URL(import.meta.url).pathname), '../..');
  const input = readFileSync('/dev/stdin', 'utf8');
  const changedFiles = input.split('\n').map(f => f.trim()).filter(Boolean);
  const manifests = loadManifests(ROOT);
  const result = walkGraph(manifests, changedFiles);
  process.stdout.write(JSON.stringify(result) + '\n');
}
```

- [ ] **Step 5: Run the tests**

Run: `node --test genesis/orchestrator/graph-walker.test.mjs`

Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add genesis/orchestrator/graph-walker.mjs genesis/orchestrator/graph-walker.test.mjs package.json pnpm-lock.yaml
git commit -m "feat(ci): add graph walker for manifest-driven change detection

Matches changed files against build manifest source globs, propagates
staleness through dependency edges, maps to gate projects for the
pre-push hook. Tested with node:test."
```

---

### Task 4: Refactor `validate-manifests.mjs` to Use Shared Utils

**Files:**
- Modify: `genesis/orchestrator/validate-manifests.mjs`

- [ ] **Step 1: Replace inline discovery with manifest-utils imports**

Replace the top imports and discovery block:

```js
#!/usr/bin/env node
// Validates all build-manifest.json files against the manifest schema.
// Also performs cross-manifest validation (dependency references, pipeline uniqueness).

import { readFileSync, existsSync } from 'fs';
import { resolve, dirname } from 'path';
import Ajv from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';
import { discoverManifests, resolveStep } from './manifest-utils.mjs';

const ROOT = resolve(dirname(new URL(import.meta.url).pathname), '../..');
const SCHEMA_PATH = resolve(ROOT, 'genesis/orchestrator/manifest.schema.json');

// Discover all build-manifest.json files
const manifestPaths = discoverManifests(ROOT);

if (manifestPaths.length === 0) {
  console.error('ERROR: No build-manifest.json files found');
  process.exit(1);
}
```

Remove the old `execSync` import since discovery is now delegated.

- [ ] **Step 2: Replace inline `dep.includes(':')` with `resolveStep` calls**

In the dependency validation section, replace:

```js
      const qualified = dep.includes(':') ? dep : `${content.pipeline}:${dep}`;
```

with:

```js
      const qualified = resolveStep(dep, content.pipeline);
```

And in the cycle detection `dfs` function, replace:

```js
  const qualified = node.includes(':') ? node : `${pipeline}:${node}`;
```

with:

```js
  const qualified = resolveStep(node, pipeline);
```

And:

```js
    const qualDep = dep.includes(':') ? dep : `${stepPipeline}:${dep}`;
```

with:

```js
    const qualDep = resolveStep(dep, stepPipeline);
```

- [ ] **Step 3: Verify the refactored script still works**

Run: `node genesis/orchestrator/validate-manifests.mjs`

Expected: Same output as before — all manifests valid, no errors.

- [ ] **Step 4: Commit**

```bash
git add genesis/orchestrator/validate-manifests.mjs
git commit -m "refactor(ci): validate-manifests uses shared manifest-utils

Replaces inline discovery and step resolution with imports from
manifest-utils.mjs. No behavior change."
```

---

### Task 5: Add Gate Mappings and Quality Steps to Existing Manifests

**Files:**
- Modify: `app/elohim-app/build-manifest.json`
- Modify: `elohim/holochain/build-manifest.json`
- Modify: `elohim/holochain/dna/build-manifest.json`
- Modify: `genesis/build-manifest.json`
- Modify: `steward/device/build-manifest.json`

- [ ] **Step 1: Update `app/elohim-app/build-manifest.json`**

Add `lint-library` step and `gate` field. The full file becomes:

```json
{
  "manifestVersion": "1.0",
  "pipeline": "elohim",
  "description": "Elohim Angular app — build, service worker, site image",
  "steps": {
    "build-angular": {
      "description": "Build Angular production bundle (includes service worker compilation via pnpm run build:sw)",
      "inputs": {
        "sources": [
          "app/elohim-app/src/**",
          "app/elohim-app/angular.json",
          "app/elohim-app/tsconfig*.json",
          "app/elohim-app/package.json",
          "app/elohim-app/vite.config.ts",
          "app/elohim-library/**"
        ],
        "buildProcess": ["Jenkinsfile"]
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
    "build-site-image": {
      "description": "Package app into container image",
      "inputs": {
        "sources": [
          "app/elohim-app/images/Dockerfile",
          "app/elohim-app/images/nginx.conf"
        ],
        "buildProcess": ["Jenkinsfile"]
      },
      "outputs": {
        "artifacts": ["site-image"],
        "verify": null
      },
      "depends": ["build-angular"],
      "executor": {
        "stage": "Build Image",
        "function": null
      }
    },
    "lint-library": {
      "description": "Quality gate for elohim-library (lint + typecheck + tests)",
      "inputs": {
        "sources": ["app/elohim-library/**"],
        "buildProcess": []
      },
      "outputs": {
        "artifacts": [],
        "verify": null
      },
      "depends": [],
      "executor": {
        "stage": "Lint Library",
        "function": null
      }
    }
  },
  "gate": {
    "projects": {
      "elohim-app": { "dir": "app/elohim-app", "steps": ["build-angular", "build-site-image"] },
      "elohim-library": { "dir": "app/elohim-library", "steps": ["lint-library"] }
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

- [ ] **Step 2: Update `elohim/holochain/build-manifest.json` (edge)**

Add gate field to the existing manifest. Insert after the `steps` block, before `deployment`:

```json
  "gate": {
    "projects": {
      "doorway": { "dir": "doorway/doorway-service", "steps": ["cargo-build-doorway"] },
      "elohim-storage": { "dir": "elohim/elohim-storage", "steps": ["cargo-build-storage"] }
    }
  },
```

- [ ] **Step 3: Update `elohim/holochain/dna/build-manifest.json`**

Add `schema-dna` step and gate field. Insert `schema-dna` into `steps`:

```json
    "schema-dna": {
      "description": "Verify DNA constants match protocol schema enums",
      "inputs": {
        "sources": [
          "elohim/holochain/**",
          "elohim/sdk/schemas/**"
        ],
        "buildProcess": []
      },
      "outputs": {
        "artifacts": [],
        "verify": null
      },
      "depends": [],
      "executor": {
        "stage": "Schema DNA Check",
        "function": null
      }
    }
```

And add gate field:

```json
  "gate": {
    "projects": {
      "schema-dna": { "dir": ".", "steps": ["schema-dna"] }
    }
  }
```

- [ ] **Step 4: Update `genesis/build-manifest.json`**

Add quality steps and gate field. Insert these steps:

```json
    "schema-validate": {
      "description": "Validate seed data against protocol schemas",
      "inputs": {
        "sources": [
          "genesis/seeds/**",
          "elohim/sdk/schemas/**"
        ],
        "buildProcess": []
      },
      "outputs": { "artifacts": [], "verify": null },
      "depends": [],
      "executor": { "stage": "Schema Validate", "function": null }
    },
    "schema-codegen": {
      "description": "Verify schema codegen is fresh",
      "inputs": {
        "sources": ["elohim/sdk/schemas/**"],
        "buildProcess": []
      },
      "outputs": { "artifacts": [], "verify": null },
      "depends": [],
      "executor": { "stage": "Schema Codegen", "function": null }
    },
    "constants-sync": {
      "description": "Verify constants sync between schema enums, generated code, and seed data",
      "inputs": {
        "sources": [
          "genesis/data/**",
          "elohim/sdk/schemas/**/*enum*",
          "**/generated/schema-enums*"
        ],
        "buildProcess": []
      },
      "outputs": { "artifacts": [], "verify": null },
      "depends": [],
      "executor": { "stage": "Constants Sync", "function": null }
    },
    "lint-a2o": {
      "description": "Lint and typecheck a2o E2E framework",
      "inputs": {
        "sources": ["genesis/a2o/**"],
        "buildProcess": []
      },
      "outputs": { "artifacts": [], "verify": null },
      "depends": [],
      "executor": { "stage": "Lint A2O", "function": null }
    }
```

And add gate field:

```json
  "gate": {
    "projects": {
      "genesis": { "dir": "genesis/seeder", "steps": ["validate-seeds", "seed-content"] },
      "schema-validate": { "dir": ".", "steps": ["schema-validate"] },
      "schema-codegen": { "dir": ".", "steps": ["schema-codegen"] },
      "constants-sync": { "dir": ".", "steps": ["constants-sync"] },
      "genesis-a2o": { "dir": "genesis/a2o", "steps": ["lint-a2o"] }
    }
  }
```

- [ ] **Step 5: Update `steward/device/build-manifest.json`**

Add gate field:

```json
  "gate": {
    "projects": {
      "steward-node": { "dir": "steward/node" }
    }
  }
```

- [ ] **Step 6: Validate all updated manifests**

Run: `node genesis/orchestrator/validate-manifests.mjs`

Expected: All manifests pass validation. If schema-dna sources overlap with build-dna-wasm sources, that's expected — different gate purposes.

- [ ] **Step 7: Commit**

```bash
git add app/elohim-app/build-manifest.json elohim/holochain/build-manifest.json elohim/holochain/dna/build-manifest.json genesis/build-manifest.json steward/device/build-manifest.json
git commit -m "feat(ci): add gate mappings and quality steps to existing manifests

Maps build steps to pre-push hook quality gate projects. Adds schema,
codegen, constants-sync, and a2o quality steps to genesis and DNA manifests."
```

---

### Task 6: Create Stub Manifests

**Files:**
- Create: `sophia/build-manifest.json`
- Create: `doorway/doorway-app/build-manifest.json`
- Create: `elohim/elohim-compute/build-manifest.json`
- Create: `genesis/orchestrator/build-manifest.json`

- [ ] **Step 1: Create `sophia/build-manifest.json`**

```json
{
  "manifestVersion": "1.0",
  "pipeline": "elohim-sophia",
  "description": "Sophia assessment engine — UMD bundle for Angular embedding",
  "steps": {
    "build-sophia-umd": {
      "description": "Build sophia-element UMD bundle",
      "inputs": {
        "sources": ["sophia/**"],
        "buildProcess": ["sophia.Jenkinsfile"]
      },
      "outputs": {
        "artifacts": ["sophia-element-umd"],
        "verify": null
      },
      "depends": [],
      "executor": {
        "stage": "Build Sophia",
        "function": null
      }
    }
  },
  "gate": {
    "projects": {
      "sophia": { "dir": "sophia" }
    }
  },
  "deployment": {}
}
```

- [ ] **Step 2: Create `doorway/doorway-app/build-manifest.json`**

```json
{
  "manifestVersion": "1.0",
  "pipeline": "elohim-doorway-app",
  "description": "Doorway admin UI — Angular app",
  "steps": {
    "build-doorway-app": {
      "description": "Build doorway admin Angular app",
      "inputs": {
        "sources": ["doorway/doorway-app/**"],
        "buildProcess": []
      },
      "outputs": {
        "artifacts": ["doorway-app-dist"],
        "verify": null
      },
      "depends": [],
      "executor": {
        "stage": "Build Doorway App",
        "function": null
      }
    }
  },
  "gate": {
    "projects": {
      "doorway-app": { "dir": "doorway/doorway-app" }
    }
  },
  "deployment": {}
}
```

- [ ] **Step 3: Create `elohim/elohim-compute/build-manifest.json`**

```json
{
  "manifestVersion": "1.0",
  "pipeline": "elohim-compute",
  "description": "Elohim compute — WASM compute module",
  "steps": {
    "build-compute": {
      "description": "Build elohim-compute WASM module",
      "inputs": {
        "sources": ["elohim/elohim-compute/**"],
        "buildProcess": []
      },
      "outputs": {
        "artifacts": ["compute-wasm"],
        "verify": null
      },
      "depends": [],
      "executor": {
        "stage": "Build Compute",
        "function": null
      }
    }
  },
  "gate": {
    "projects": {
      "elohim-compute": { "dir": "elohim/elohim-compute" }
    }
  },
  "deployment": {}
}
```

- [ ] **Step 4: Create `genesis/orchestrator/build-manifest.json`**

```json
{
  "manifestVersion": "1.0",
  "pipeline": "elohim-orchestrator",
  "description": "CI orchestrator — Jenkinsfile linting and validation",
  "steps": {
    "lint-jenkinsfiles": {
      "description": "Lint all Jenkinsfiles with npm-groovy-lint",
      "inputs": {
        "sources": [
          "**/Jenkinsfile*",
          "genesis/orchestrator/**"
        ],
        "buildProcess": []
      },
      "outputs": {
        "artifacts": [],
        "verify": null
      },
      "depends": [],
      "executor": {
        "stage": "Lint Jenkinsfiles",
        "function": null
      }
    }
  },
  "gate": {
    "projects": {
      "orchestrator": { "dir": "genesis/orchestrator" }
    }
  },
  "deployment": {}
}
```

- [ ] **Step 5: Validate all manifests (including new stubs)**

Run: `node genesis/orchestrator/validate-manifests.mjs`

Expected: All manifests pass. The `elohim-sophia:build-sophia-umd` cross-manifest dependency (from `elohim:build-angular`) now resolves.

- [ ] **Step 6: Commit**

```bash
git add sophia/build-manifest.json doorway/doorway-app/build-manifest.json elohim/elohim-compute/build-manifest.json genesis/orchestrator/build-manifest.json
git commit -m "feat(ci): add stub manifests for sophia, doorway-app, compute, orchestrator

Completes manifest coverage — all pre-push hook projects now have a
manifest home. Resolves elohim-sophia:build-sophia-umd cross-manifest dep."
```

---

### Task 7: Integrate Walker into Pre-Push Hook

**Files:**
- Modify: `.husky/pre-push`

- [ ] **Step 1: Add manifest-driven project detection block**

Replace lines 75-138 (the `# ── Project Detection ──` section through `exit 0`) with:

```sh
# ── Project Detection (manifest-driven) ──────────────────────────
#
# Try graph walker first — reads build-manifest.json files and matches
# changed files against source globs. Falls back to grep patterns if
# node is unavailable or no manifests exist.

PROJECTS=""
USE_MANIFEST=false

if command -v node >/dev/null 2>&1; then
  MANIFEST_RESULT=$(echo "$CHANGED" | node genesis/orchestrator/graph-walker.mjs 2>/dev/null)
  if [ $? -eq 0 ] && [ -n "$MANIFEST_RESULT" ]; then
    MANIFEST_DIRS_RAW=$(echo "$MANIFEST_RESULT" | node -e "
      const d = JSON.parse(require('fs').readFileSync('/dev/stdin','utf8'));
      const names = [], dirs = [];
      for (const p of d.projects) { names.push(p.name); dirs.push(p.dir); }
      if (names.length > 0) {
        console.log('PROJECTS=\"' + names.join(' ') + '\"');
        console.log('MANIFEST_DIRS=\"' + dirs.join(' ') + '\"');
      }
    " 2>/dev/null)
    if [ -n "$MANIFEST_DIRS_RAW" ]; then
      eval "$MANIFEST_DIRS_RAW"
      USE_MANIFEST=true
    fi
  fi
fi

# Fallback: grep-based project detection
if [ "$USE_MANIFEST" = false ]; then
  # doorway-app must be checked BEFORE doorway (prefix overlap)
  if echo "$CHANGED" | grep -q "^doorway/doorway-app/"; then
    PROJECTS="$PROJECTS doorway-app"
  fi
  if echo "$CHANGED" | grep "^doorway/" | grep -qv "^doorway/doorway-app/"; then
    PROJECTS="$PROJECTS doorway"
  fi
  if echo "$CHANGED" | grep -q "^app/elohim-app/"; then
    PROJECTS="$PROJECTS elohim-app"
  fi
  if echo "$CHANGED" | grep -q "^sophia/"; then
    PROJECTS="$PROJECTS sophia"
  fi
  if echo "$CHANGED" | grep -q "^elohim/elohim-storage/"; then
    PROJECTS="$PROJECTS elohim-storage"
  fi
  if echo "$CHANGED" | grep -q "^elohim/elohim-compute/"; then
    PROJECTS="$PROJECTS elohim-compute"
  fi
  if echo "$CHANGED" | grep -q "^steward/node/"; then
    PROJECTS="$PROJECTS steward-node"
  fi
  if echo "$CHANGED" | grep -q "^app/elohim-library/"; then
    PROJECTS="$PROJECTS elohim-library"
  fi
  if echo "$CHANGED" | grep -q "^genesis/"; then
    PROJECTS="$PROJECTS genesis"
  fi
  if echo "$CHANGED" | grep -q "^genesis/\|^elohim/sdk/schemas/"; then
    PROJECTS="$PROJECTS schema-validate"
  fi
  if echo "$CHANGED" | grep -q "^elohim/holochain/\|^elohim/sdk/schemas/"; then
    PROJECTS="$PROJECTS schema-dna"
  fi
  if echo "$CHANGED" | grep -q "^genesis/data/\|^elohim/sdk/schemas/.*enum\|generated/schema-enums"; then
    PROJECTS="$PROJECTS constants-sync"
  fi
  if echo "$CHANGED" | grep -q "^elohim/sdk/schemas/"; then
    PROJECTS="$PROJECTS schema-codegen"
  fi
  if echo "$CHANGED" | grep -q "Jenkinsfile"; then
    PROJECTS="$PROJECTS orchestrator"
  fi
  if echo "$CHANGED" | grep -q "^genesis/a2o/"; then
    PROJECTS="$PROJECTS genesis-a2o"
  fi
fi

# Trim leading space
PROJECTS=$(echo "$PROJECTS" | sed 's/^ //')

# No project source changes — let it through
if [ -z "$PROJECTS" ]; then
  exit 0
fi
```

- [ ] **Step 2: Update the directory resolution in the gate loop**

Replace the `case "$PROJECT" in ... esac` block inside the gate loop (lines 324-341) with a dual-path resolver:

```sh
  # Map project name to directory
  if [ "$USE_MANIFEST" = true ]; then
    PROJECT_DIR="$1"
    shift
  else
    case "$PROJECT" in
      elohim-app)       PROJECT_DIR="app/elohim-app" ;;
      elohim-library)   PROJECT_DIR="app/elohim-library" ;;
      sophia)           PROJECT_DIR="sophia" ;;
      doorway)          PROJECT_DIR="doorway/doorway-service" ;;
      doorway-app)      PROJECT_DIR="doorway/doorway-app" ;;
      elohim-storage)   PROJECT_DIR="elohim/elohim-storage" ;;
      elohim-compute)   PROJECT_DIR="elohim/elohim-compute" ;;
      steward-node)     PROJECT_DIR="steward/node" ;;
      genesis)          PROJECT_DIR="genesis/seeder" ;;
      schema-validate)  PROJECT_DIR="." ;;
      schema-dna)       PROJECT_DIR="." ;;
      schema-codegen)   PROJECT_DIR="." ;;
      constants-sync)   PROJECT_DIR="." ;;
      genesis-a2o)      PROJECT_DIR="genesis/a2o" ;;
      orchestrator)     PROJECT_DIR="genesis/orchestrator" ;;
      *)                PROJECT_DIR="$PROJECT" ;;
    esac
  fi
```

- [ ] **Step 3: Initialize positional parameters for manifest dirs**

Add this line just before the `for PROJECT in $PROJECTS` loop:

```sh
if [ "$USE_MANIFEST" = true ]; then
  set -- $MANIFEST_DIRS
fi
```

- [ ] **Step 4: Smoke test — verify hook runs with no errors**

Run: `echo "app/elohim-app/src/main.ts" | node genesis/orchestrator/graph-walker.mjs`

Expected: JSON output with `elohim-app` in the projects array.

Run: `echo "" | sh .husky/pre-push <<< "refs/heads/dev 1234567890abcdef1234567890abcdef12345678 refs/heads/dev 0000000000000000000000000000000000000000"`

Expected: Hook exits 0 (no changes detected in empty input).

- [ ] **Step 5: Commit**

```bash
git add .husky/pre-push
git commit -m "feat(ci): pre-push hook uses manifest-driven graph walker

Replaces grep-based project detection with graph walker that reads
build-manifest.json files. Falls back to grep patterns if node is
unavailable or manifests don't exist. Gate execution unchanged."
```

---

### Task 8: End-to-End Validation

- [ ] **Step 1: Run manifest validation**

Run: `node genesis/orchestrator/validate-manifests.mjs`

Expected: All manifests pass schema validation, no duplicate pipelines, all dependency references resolve, no cycles, all buildProcess references resolve.

- [ ] **Step 2: Run walker tests**

Run: `node --test genesis/orchestrator/graph-walker.test.mjs`

Expected: All tests pass.

- [ ] **Step 3: Test walker CLI with realistic changed files**

Test 1 — Angular app change:
```bash
echo "app/elohim-app/src/app/lamad/lamad.component.ts" | node genesis/orchestrator/graph-walker.mjs | node -e "const d=JSON.parse(require('fs').readFileSync('/dev/stdin','utf8')); console.log(d.projects.map(p=>p.name))"
```
Expected: `[ 'elohim-app' ]`

Test 2 — Doorway change:
```bash
echo "doorway/doorway-service/src/main.rs" | node genesis/orchestrator/graph-walker.mjs | node -e "const d=JSON.parse(require('fs').readFileSync('/dev/stdin','utf8')); console.log(d.projects.map(p=>p.name))"
```
Expected: `[ 'doorway' ]`

Test 3 — Schema change (triggers multiple gates):
```bash
echo "elohim/sdk/schemas/v1/content-type.schema.json" | node genesis/orchestrator/graph-walker.mjs | node -e "const d=JSON.parse(require('fs').readFileSync('/dev/stdin','utf8')); console.log(d.projects.map(p=>p.name))"
```
Expected: Should include `schema-validate`, `schema-codegen`, and `schema-dna` (schema changes trigger all three).

Test 4 — Sophia change cascades to elohim-app:
```bash
printf "sophia/packages/sophia-core/src/index.ts" | node genesis/orchestrator/graph-walker.mjs | node -e "const d=JSON.parse(require('fs').readFileSync('/dev/stdin','utf8')); console.log(d.projects.map(p=>p.name))"
```
Expected: `[ 'sophia', 'elohim-app' ]` — sophia first (dependency), elohim-app second (dependent via `elohim-sophia:build-sophia-umd`).

Test 5 — Jenkinsfile change triggers orchestrator + all referencing pipelines:
```bash
echo "Jenkinsfile" | node genesis/orchestrator/graph-walker.mjs | node -e "const d=JSON.parse(require('fs').readFileSync('/dev/stdin','utf8')); console.log(d.projects.map(p=>p.name))"
```
Expected: Should include `orchestrator` (Jenkinsfile matches `**/Jenkinsfile*`) and `elohim-app` (root Jenkinsfile is a buildProcess for elohim:build-angular and elohim:build-site-image).

- [ ] **Step 4: Compare walker output to current grep detection for parity**

```bash
# Simulate the file list that would trigger every project in the current hook
TEST_FILES="app/elohim-app/src/x.ts
doorway/doorway-service/src/x.rs
doorway/doorway-app/src/x.ts
sophia/src/x.ts
elohim/elohim-storage/src/x.rs
elohim/elohim-compute/src/x.rs
steward/node/src/x.rs
app/elohim-library/src/x.ts
genesis/seeds/x.json
elohim/sdk/schemas/v1/x.json
elohim/holochain/dna/x.rs
genesis/a2o/src/x.ts
Jenkinsfile"

echo "$TEST_FILES" | node genesis/orchestrator/graph-walker.mjs | node -e "
  const d=JSON.parse(require('fs').readFileSync('/dev/stdin','utf8'));
  console.log('Walker detected:', d.projects.map(p=>p.name).sort().join(', '));
"
```

Expected: All 15 project names that the current grep logic would detect. If any are missing, check the manifest source globs.

- [ ] **Step 5: Commit final state**

If any fixes were needed during validation, commit them:

```bash
git add -A
git commit -m "fix(ci): address parity gaps found during e2e validation"
```
