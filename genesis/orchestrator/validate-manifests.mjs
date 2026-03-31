#!/usr/bin/env node
// Validates all build-manifest.json files against the manifest schema.
// Also performs cross-manifest validation (dependency references, pipeline uniqueness).

import { readFileSync, existsSync } from 'fs';
import { resolve, dirname } from 'path';
import { execSync } from 'child_process';
import Ajv from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

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
addFormats(ajv);
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

// Phase 3: Check buildProcess file references
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
