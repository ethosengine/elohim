import { test, describe } from 'node:test';
import { strict as assert } from 'node:assert';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import {
  loadPipelineRegistry,
  nonManualPipelines,
  dispatchablePipelines,
  pipelinesThatTriggerGenesis,
  pipelineDependencyMap,
} from './pipeline-registry.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '../..');

describe('pipeline-registry', () => {
  const registry = loadPipelineRegistry(ROOT);

  test('loads every build-manifest.json with a pipeline field', () => {
    assert.ok(registry.size >= 8, `expected ≥8 pipelines, got ${registry.size}`);
    for (const known of ['elohim', 'elohim-edge', 'elohim-holochain', 'elohim-genesis', 'elohim-sophia']) {
      assert.ok(registry.has(known), `missing pipeline: ${known}`);
    }
  });

  test('nonManualPipelines excludes manualOnly entries', () => {
    const names = nonManualPipelines(registry);
    assert.ok(!names.includes('elohim-steward'), 'elohim-steward should be excluded');
    assert.ok(names.includes('elohim'), 'elohim should be included');
  });

  test('dispatchablePipelines returns only entries with jenkinsPath', () => {
    const names = dispatchablePipelines(registry);
    assert.ok(!names.includes('elohim-doorway-app'), 'graph-only pipeline should be excluded');
    assert.ok(!names.includes('elohim-compute'), 'graph-only pipeline should be excluded');
    assert.ok(names.includes('elohim'), 'elohim has jenkinsPath');
  });

  test('pipelinesThatTriggerGenesis returns the marked set', () => {
    const names = pipelinesThatTriggerGenesis(registry);
    assert.deepStrictEqual(
      [...names].sort(),
      ['elohim', 'elohim-edge', 'elohim-holochain'].sort()
    );
  });

  test('pipelineDependencyMap produces a name → deps map', () => {
    const deps = pipelineDependencyMap(registry);
    assert.deepStrictEqual(deps.get('elohim-edge'), ['elohim-holochain']);
    assert.deepStrictEqual(deps.get('elohim-genesis'), ['elohim-edge', 'elohim']);
    assert.deepStrictEqual(deps.get('elohim-sophia'), []);
  });
});
