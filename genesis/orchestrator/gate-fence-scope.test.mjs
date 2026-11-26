import { readFileSync } from 'node:fs';
import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { decideReuse, pushRange } from './gate-runner.mjs';
import {
  FENCE_SCOPE_TABLE,
  classifyFencePath,
  evaluateFenceScope,
  fenceScopeLine,
  fenceScopeShadow,
  parseFenceScopeTable,
  readDestinationFenceScope,
  resetFenceScopeCache,
} from './gate-fence-scope.mjs';

const HERE = fileURLToPath(new URL('.', import.meta.url));
const TABLE = parseFenceScopeTable(readFileSync(resolve(HERE, 'gate-fence-scope.json'), 'utf8'));
const destination = { state: 'read', table: TABLE, imports: new Set(TABLE.judge.importRoots) };
const project = 'fixture';
const key = 'closure';
const green = { source: 'sibling', project, closure: key, success: true };

test('classifies unknown gate modules and the concrete R1 verdict reader as J', () => {
  assert.equal(classifyFencePath('genesis/orchestrator/gate-future-reader.mjs', destination), 'J');
  assert.equal(classifyFencePath('elohim/eprfs/epr-cli/src/peer.rs', destination), 'J');
});

test('classifies a plain peer substrate file as S and compiler input cargo-lint as K', () => {
  assert.equal(classifyFencePath('elohim/eprfs/eprfs-peer/src/fetch.rs', destination), 'S');
  assert.equal(classifyFencePath('genesis/orchestrator/cargo-lint.mjs', destination), 'K');
});

test('an unclassified changed path is J and refuses with that path named', () => {
  const path = 'docs/unclassified-fence-input.txt';
  assert.equal(classifyFencePath(path, destination), 'J');
  const reading = evaluateFenceScope({
    project, key, paths: [path], candidates: [green], destination, readerAdmitted: true,
  });
  assert.deepEqual(reading, { answer: 'would-refuse', paths: [path], reason: 'judge' });
  assert.equal(fenceScopeLine(project, reading),
    `[gate] fixture fence-scope (shadow): would refuse: J path ${path}`);
});

test('an S change becomes a J refusal unless the reader is an admitted store release', () => {
  const scene = {
    project, key, paths: ['elohim/eprfs/eprfs-peer/src/fetch.rs'], candidates: [green], destination,
  };
  assert.deepEqual(evaluateFenceScope({ ...scene, readerAdmitted: false }), {
    answer: 'would-refuse',
    paths: ['elohim/eprfs/eprfs-peer/src/fetch.rs'],
    reason: 'substrate-reader-not-admitted',
  });
  assert.deepEqual(evaluateFenceScope({ ...scene, readerAdmitted: true }), {
    answer: 'would-admit',
    paths: ['elohim/eprfs/eprfs-peer/src/fetch.rs'],
    reason: 'key-unchanged',
  });
});

test('reads the table and transitive import closure only from the destination ref', () => {
  resetFenceScopeCache();
  const table = JSON.stringify({
    version: 1,
    judge: { importRoots: ['genesis/orchestrator/judge.mjs'], exact: [FENCE_SCOPE_TABLE] },
    keyed: { exact: ['keyed'] },
    substrate: { prefixes: ['peer/'], exact: [] },
  });
  const asked = [];
  const blobs = new Map([
    [`origin/dev:${FENCE_SCOPE_TABLE}`, table],
    ['origin/dev:genesis/orchestrator/judge.mjs', "import './child.mjs';\n"],
    ['origin/dev:genesis/orchestrator/child.mjs', 'export const child = true;\n'],
  ]);
  const spawn = (_command, args) => {
    asked.push(args[1]);
    const text = blobs.get(args[1]);
    return text === undefined ? { status: 128, stdout: '' } : { status: 0, stdout: text };
  };
  const read = readDestinationFenceScope({ root: '/fixture-destination', spawn });
  assert.deepEqual(asked, [...blobs.keys()]);
  assert.ok(read.imports.has('genesis/orchestrator/child.mjs'));
  assert.equal(classifyFencePath('genesis/orchestrator/child.mjs', read), 'J');
  assert.ok(asked.every(spec => spec.startsWith('origin/dev:')));
});

test('an absent destination table holds the fence and says so', () => {
  resetFenceScopeCache();
  const reading = fenceScopeShadow({
    project, key, range: { base: 'base', files: ['elohim/eprfs/eprfs-peer/src/fetch.rs'] },
    candidates: [green], root: '/fixture-absent', env: {},
    spawn: () => ({ status: 128, stdout: '' }),
  });
  assert.deepEqual(reading, {
    answer: 'would-refuse', paths: [FENCE_SCOPE_TABLE], reason: 'table-absent',
  });
  assert.equal(fenceScopeLine(project, reading),
    '[gate] fixture fence-scope (shadow): table absent at destination; the fence holds');
});

test('a changed-path diff that fails is unavailable, not empty, and the shadow refuses on it', () => {
  // pushRange's merge base answers, its diff fails: the range says unavailable rather than "no files".
  const failedDiff = (_command, args) => (args[0] === 'merge-base'
    ? { status: 0, stdout: 'base\n' }
    : { status: 128, stdout: '', stderr: 'fatal: bad object' });
  const range = pushRange({ root: '/fixture-failed-diff', gateCommit: 'head', spawn: failedDiff });
  assert.deepEqual(range, { base: 'base', files: [], unavailable: true });

  resetFenceScopeCache();
  const table = JSON.stringify({
    version: 1,
    judge: { importRoots: [], exact: [FENCE_SCOPE_TABLE] },
    keyed: { exact: [] },
    substrate: { prefixes: [], exact: [] },
  });
  const spawn = (_command, args) => (args[1] === `origin/dev:${FENCE_SCOPE_TABLE}`
    ? { status: 0, stdout: table } : { status: 128, stdout: '' });
  const reading = fenceScopeShadow({
    project, key, range, candidates: [green], root: '/fixture-unavailable-range', env: {}, spawn,
  });
  assert.equal(reading.answer, 'would-refuse');
  assert.equal(reading.reason, 'range-unavailable');
  assert.equal(fenceScopeLine(project, reading),
    '[gate] fixture fence-scope (shadow): would refuse: governance range unavailable; the fence holds');
});

test('GATE_FENCE_SCOPE=off adds no spawn, reader resolution or result', () => {
  let spawns = 0;
  let resolutions = 0;
  const reading = fenceScopeShadow({
    project, key, range: { base: 'base', files: ['elohim/eprfs/eprfs-peer/src/fetch.rs'] },
    candidates: [green], root: '/fixture-off', env: { GATE_FENCE_SCOPE: 'off' },
    spawn: () => { spawns++; return { status: 1 }; },
    resolveReader: () => { resolutions++; return { source: 'head' }; },
  });
  assert.equal(reading, null);
  assert.equal(spawns, 0);
  assert.equal(resolutions, 0);
});

test('the shadow leaves decideReuse decision objects unchanged', () => {
  const evidence = { key, project, rows: [], unverifiable: [], dirty: false };
  const context = { mode: 'off', enabled: true, selfId: 'self', selectorCheck: () => ({ ok: true }) };
  const before = decideReuse({ name: project }, evidence, context);
  const snapshot = structuredClone(before);
  const reading = evaluateFenceScope({
    project, key,
    paths: ['genesis/orchestrator/cargo-lint.mjs'],
    candidates: [green], destination, readerAdmitted: true,
  });
  assert.equal(reading.answer, 'would-admit');
  assert.deepEqual(before, snapshot);
  assert.deepEqual(decideReuse({ name: project }, evidence, context), snapshot);
});
