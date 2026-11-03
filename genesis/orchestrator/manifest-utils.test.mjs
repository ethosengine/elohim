import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { execFileSync } from 'node:child_process';
import { discoverManifests } from './manifest-utils.mjs';
import { loadGateRegistry, loadPipelineRegistry } from './pipeline-registry.mjs';

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), 'manifest-discovery-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  return root;
}

function manifest(root, path, name = 'app') {
  const target = join(root, path, 'build-manifest.json');
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, JSON.stringify({
    pipeline: name,
    steps: {},
    gate: { projects: { [name]: { dir: path, run: { kind: 'root-just', recipe: '_gate-app' } } } },
  }));
  return target;
}

test('retained local-dev copies do not contribute pipeline or gate declarations', t => {
  const root = fixture(t);
  manifest(root, 'app/elohim-app');
  manifest(root, 'elohim/local-development', 'local-development');
  const retained = manifest(root, 'genesis/local-dev/gate-task-proof/repo/app/elohim-app');
  const bytes = readFileSync(retained);
  // The reservation is basename-wide, matching **/local-dev/ in the root ignore contract.
  manifest(root, 'local-dev/another-repo/app');
  assert.deepEqual(discoverManifests(root), [
    './app/elohim-app/build-manifest.json',
    './elohim/local-development/build-manifest.json',
  ]);
  assert.deepEqual([...loadPipelineRegistry(root).keys()], ['app', 'local-development']);
  assert.deepEqual([...loadGateRegistry(root).keys()], ['app', 'local-development']);
  assert.deepEqual(readFileSync(retained), bytes, 'discovery preserves retained proof bytes');
});

test('duplicate declarations outside retained runtime state still refuse', t => {
  const root = fixture(t);
  manifest(root, 'app/first');
  manifest(root, 'app/second');
  assert.throws(() => loadGateRegistry(root), /Duplicate gate project 'app'/);
  assert.throws(() => loadPipelineRegistry(root), /Duplicate pipeline name 'app'/);
});

test('Jenkins discovery applies the same local-dev boundary', t => {
  const root = fixture(t);
  manifest(root, 'app/elohim-app');
  manifest(root, 'genesis/local-dev/gate-task-proof/repo/app/elohim-app');
  const groovy = readFileSync(new URL('./build-graph.groovy', import.meta.url), 'utf8');
  const command = groovy.match(/script: "(find \. -name 'build-manifest\.json'[^"\n]*)"/)?.[1];
  assert.ok(command, 'locate the actual Jenkins manifest discovery command');
  const discovered = execFileSync('sh', ['-c', command], { cwd: root, encoding: 'utf8' })
    .trim().split('\n');
  assert.deepEqual(discovered, ['./app/elohim-app/build-manifest.json']);
  assert.deepEqual(discovered, discoverManifests(root));
});
