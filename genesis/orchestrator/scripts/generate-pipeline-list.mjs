#!/usr/bin/env node
/**
 * Generates genesis/orchestrator/pipeline-list.json from PIPELINES in
 * orchestrator-strategy.mjs. Shell scripts (count-pipeline-failures.sh,
 * jenkins-client.sh) consume the JSON instead of hardcoding their own lists.
 *
 * Run by:
 *   - just ci-pipeline-list      (manual / pre-commit)
 *   - .husky/pre-push            (when orchestrator-strategy.mjs changes)
 *   - genesis/orchestrator/Jenkinsfile  (as a sanity check stage)
 */

import { writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { PIPELINES } from '../orchestrator-strategy.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const out = resolve(__dirname, '..', 'pipeline-list.json');

const payload = {
  generatedFrom: 'orchestrator-strategy.mjs PIPELINES',
  pipelines: Object.entries(PIPELINES).map(([name, cfg]) => ({
    name,
    manualOnly: !!cfg.manualOnly,
    triggersGenesis: !!cfg.triggersGenesis,
    cascades: cfg.cascades !== false,
  })),
};

writeFileSync(out, JSON.stringify(payload, null, 2) + '\n');
console.log(`wrote ${out} (${payload.pipelines.length} pipelines)`);
