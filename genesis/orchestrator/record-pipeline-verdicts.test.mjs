import assert from 'node:assert/strict';
import { test } from 'node:test';

import { buildsUrl, dispatchOnlyTargets, recordVerdicts, verdictForSha } from './record-pipeline-verdicts.mjs';

const SHA = 'a'.repeat(40);
const OTHER = 'b'.repeat(40);
const rev = sha => [{}, { lastBuiltRevision: { SHA1: sha } }];
const target = { name: 'elohim-holochain', sha: SHA, job: 'elohim-holochain/dev' };

async function run(fetchJson) {
  const lines = [];
  const rows = await recordVerdicts({
    targets: [target],
    fetchJson,
    jenkinsUrl: 'https://jenkins.example/',
    log: l => lines.push(l),
  });
  return { rows, lines };
}

test('a finished SUCCESS build of the baseline sha writes an entry', async () => {
  const { rows } = await run(async () => ({
    builds: [{ number: 12, result: 'SUCCESS', building: false, actions: rev(SHA) }],
  }));
  assert.deepEqual(rows, { 'elohim-holochain': { sha: SHA, result: 'SUCCESS', build: 12 } });
});

test('a build of the baseline sha still running writes no entry', async () => {
  const { rows } = await run(async () => ({
    builds: [
      { number: 13, result: null, building: true, actions: rev(SHA) },
      { number: 12, result: 'SUCCESS', building: false, actions: rev(SHA) },
    ],
  }));
  assert.deepEqual(rows, {});
});

test('no build whose revision matches the baseline sha writes no entry', async () => {
  const { rows } = await run(async () => ({
    builds: [{ number: 12, result: 'SUCCESS', building: false, actions: rev(OTHER) }],
  }));
  assert.deepEqual(rows, {});
});

test('a fetch error writes no entry and does not throw', async () => {
  const { rows, lines } = await run(async () => {
    throw new Error('HTTP 503');
  });
  assert.deepEqual(rows, {});
  assert.equal(lines.length, 1);
  assert.match(lines[0], /read failed \(HTTP 503\)/);
});

test('verdictForSha tolerates absent/malformed builds', () => {
  assert.equal(verdictForSha(undefined, SHA), null);
  assert.equal(verdictForSha([{ number: 1 }], SHA), null);
});

test('only longRunning pipelines with a sha baseline are targeted, on their dispatch job path', () => {
  const manifests = [
    { content: { pipeline: 'elohim-holochain', longRunning: true } },
    { content: { pipeline: 'elohim-edge' } },
    { content: { pipeline: 'elohim-steward', longRunning: true } },
    { content: { pipeline: 'conductor', longRunning: true, jenkinsJob: 'elohim-edgenode', jenkinsBranch: 'main' } },
  ];
  const targets = dispatchOnlyTargets(manifests, { 'elohim-holochain': SHA, 'elohim-edge': SHA, conductor: OTHER, __global__: SHA }, 'dev');
  assert.deepEqual(targets, [
    { name: 'elohim-holochain', sha: SHA, job: 'elohim-holochain/dev' },
    { name: 'conductor', sha: OTHER, job: 'elohim-edgenode/main' },
  ]);
  assert.equal(
    buildsUrl('https://j.example', 'elohim-holochain/dev').split('?')[0],
    'https://j.example/job/elohim-holochain/job/dev/api/json',
  );
});
