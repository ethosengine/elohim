import { test, describe } from 'node:test';
import { strict as assert } from 'node:assert';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { parseCommitTags, parseSkipCi } from './commit-tag-parser.mjs';
import { loadPipelineRegistry } from './pipeline-registry.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '../..');
const registry = loadPipelineRegistry(ROOT);

describe('parseCommitTags', () => {
  test('[build:app] returns elohim', () => {
    assert.deepStrictEqual(parseCommitTags('ci: retrigger [build:app]', registry), ['elohim']);
  });

  test('[build:edge,app] returns both', () => {
    assert.deepStrictEqual(
      parseCommitTags('[build:edge,app] fix things', registry).sort(),
      ['elohim', 'elohim-edge'].sort()
    );
  });

  test('[build:all] returns all non-manual pipelines', () => {
    const result = parseCommitTags('[build:all]', registry);
    assert.ok(!result.includes('elohim-steward'), 'should exclude steward (manualOnly)');
    assert.ok(result.includes('elohim'), 'should include elohim');
    assert.ok(result.length >= 6, `expected ≥6 pipelines, got ${result.length}`);
  });

  test('unknown tag silently dropped', () => {
    assert.deepStrictEqual(parseCommitTags('[build:nonsense] foo', registry), []);
  });

  test('no tag returns empty', () => {
    assert.deepStrictEqual(parseCommitTags('regular commit', registry), []);
  });
});

describe('parseSkipCi', () => {
  test('[skip ci] returns true', () => {
    assert.strictEqual(parseSkipCi('chore: docs [skip ci]'), true);
  });

  test('[ci skip] returns true', () => {
    assert.strictEqual(parseSkipCi('chore: docs [ci skip]'), true);
  });

  test('no tag returns false', () => {
    assert.strictEqual(parseSkipCi('regular commit'), false);
  });
});
