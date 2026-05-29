#!/usr/bin/env node
/**
 * Generates genesis/orchestrator/pipeline-list.json from build-manifest.json
 * files via pipeline-registry.mjs. Shell scripts (count-pipeline-failures.sh,
 * jenkins-client.sh) consume the JSON instead of hardcoding their own lists.
 *
 * Run by:
 *   - just ci-pipeline-list      (manual / pre-commit)
 *   - .husky/pre-push            (when build-manifest.json files change)
 *   - genesis/orchestrator/Jenkinsfile  (as a sanity check stage)
 */

import { writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadPipelineRegistry } from '../pipeline-registry.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '../../..');
const out = resolve(__dirname, '..', 'pipeline-list.json');

const registry = loadPipelineRegistry(ROOT);
const pipelines = [...registry.values()]
  .filter(p => p.jenkinsPath)  // only dispatchable pipelines
  .map(p => ({
    name: p.pipeline,
    manualOnly: p.manualOnly,
    triggersGenesis: p.triggersGenesis,
    cascades: p.cascades,
  }));

const payload = {
  generatedFrom: 'build-manifest.json files via pipeline-registry.mjs',
  pipelines,
};

writeFileSync(out, JSON.stringify(payload, null, 2) + '\n');
console.log(`Wrote ${pipelines.length} pipelines to ${out}`);
