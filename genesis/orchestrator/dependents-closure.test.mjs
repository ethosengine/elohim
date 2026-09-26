// Backlog row 19: a failed pipeline in an orchestrator wave dooms only its
// transitive consumers, never every later level.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dependentsClosure } from './graph-walker.mjs';

const meta = {
  'elohim-eprfs': { dependsOn: [] },
  'elohim-holochain': { dependsOn: [] },
  'elohim-edge': { dependsOn: ['elohim-holochain'] },
  elohim: { dependsOn: ['elohim-edge'] },
  'elohim-genesis': { dependsOn: ['elohim'] },
};

test('a leaf with no consumers dooms nothing', () =>
  assert.deepEqual(dependentsClosure(meta, ['elohim-eprfs']), []));

test('a producer dooms its transitive consumers only', () =>
  assert.deepEqual(dependentsClosure(meta, ['elohim-holochain']), ['elohim', 'elohim-edge', 'elohim-genesis']));

test('failed names are never listed as doomed, even when one consumes another', () =>
  assert.deepEqual(dependentsClosure(meta, ['elohim-edge', 'elohim-holochain']), ['elohim', 'elohim-genesis']));

test('an unknown failed name dooms nothing and a missing dependsOn is tolerated', () =>
  assert.deepEqual(dependentsClosure({ a: {}, b: { dependsOn: ['a'] } }, ['zzz']), []));

test('CLI --dependents prints one real consumer per line from the repo manifests', () => {
  const walker = fileURLToPath(new URL('./graph-walker.mjs', import.meta.url));
  const out = execFileSync('node', [walker, '--dependents', 'elohim-holochain'], { encoding: 'utf8', input: '' });
  const lines = out.split('\n').filter(Boolean);
  assert.ok(lines.includes('elohim-edge'), `expected elohim-edge in ${JSON.stringify(lines)}`);
  assert.ok(!lines.includes('elohim-holochain'));
  assert.deepEqual(lines, [...lines].sort());
});
