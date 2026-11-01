// gate-preflight.mjs: each seconds check refuses the careless miss it names, says its cure, and
// stays silent when the selected gates cannot be affected. Habit `careless-reds-cost-seconds`.
import { spawnSync } from 'node:child_process';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  checkElementDists, checkEvidenceRecordable, checkNodeModules, checkReadSets, checkSophiaUmd,
  checkCargoDeny, checkStorageClientDist, checkSubmodules, emitLive, NAMED_READ_SET_PROJECTS, onRedPolicy,
  runPreflight,
} from './gate-preflight.mjs';
import { orderByCost } from './gate-runner.mjs';
import { appendEvent, closeRun, formatEvent, formatHarvest, harvest, openRun, outlook } from './gate-live.mjs';

function scratch(t) {
  const dir = mkdtempSync(join(tmpdir(), 'gate-preflight-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  return dir;
}
const put = (root, path, text = '') => {
  mkdirSync(join(root, path, '..'), { recursive: true });
  writeFileSync(join(root, path), text);
};
const app = { name: 'elohim-app', dir: 'app/elohim-app', run: { kind: 'root-just' } };
const storage = { name: 'elohim-storage', dir: 'elohim/elohim-storage', run: { kind: 'just', cargo: {} } };

test('submodules: an uninitialised attested submodule refuses with the init command', () => {
  const spawn = () => ({ status: 0, stdout: '-abc elohim/brit\n abc elohim/rakia (v1)\n abc sophia\n' });
  const r = checkSubmodules({ root: '/x', spawn });
  assert.equal(r.level, 'REFUSED');
  assert.match(r.cure, /git submodule update --init elohim\/brit/);
});

test('submodules: a checked-out commit off the pin warns, never refuses', () => {
  const spawn = () => ({ status: 0, stdout: ' abc elohim/brit\n+def sophia\n' });
  assert.equal(checkSubmodules({ root: '/x', spawn }).level, 'WARN');
});

test('node_modules: required only by a non-cargo gate', (t) => {
  const root = scratch(t);
  assert.equal(checkNodeModules({ root, projects: [storage] }), null);
  assert.equal(checkNodeModules({ root, projects: [app] }).level, 'REFUSED');
  put(root, 'node_modules/.modules.yaml');
  assert.equal(checkNodeModules({ root, projects: [app] }).level, 'ok');
});

test('element dists: only the element packages a selected project imports, unless its prebuild builds them', (t) => {
  const root = scratch(t);
  for (const name of ['elohim-core', 'unimported-element']) {
    put(root, `app/elohim-elements/${name}/package.json`, JSON.stringify({
      exports: { './register': { import: './dist/register.js' }, './tokens.scss': './src/tokens.scss' },
    }));
  }
  put(root, 'app/elohim-app/package.json', JSON.stringify({ dependencies: { 'elohim-core': 'workspace:*' } }));
  put(root, 'doorway/doorway-app/package.json', JSON.stringify({ dependencies: { 'elohim-core': 'workspace:*' }, scripts: { prebuild: 'pnpm run elements' } }));
  const doorwayApp = { name: 'doorway-app', dir: 'doorway/doorway-app', run: { kind: 'just' } };
  assert.equal(checkElementDists({ root, projects: [storage] }), null, 'no element importer selected');
  assert.equal(checkElementDists({ root, projects: [doorwayApp] }), null, 'its prebuild builds its elements');
  const red = checkElementDists({ root, projects: [app] });
  assert.equal(red.level, 'REFUSED');
  assert.match(red.why, /elohim-core/);
  assert.doesNotMatch(red.why, /unimported-element/, 'an element no selected gate imports is never required');
  assert.match(red.cure, /--filter elohim-core/);
  put(root, 'app/elohim-elements/elohim-core/dist/register.js');
  assert.equal(checkElementDists({ root, projects: [app] }).level, 'ok');
});

test('storage-client dist refuses for genesis-a2o, warns for app gates; a missing sophia UMD only warns', (t) => {
  const root = scratch(t);
  const a2o = { name: 'genesis-a2o', dir: 'genesis/a2o', run: { kind: 'just' } };
  assert.equal(checkStorageClientDist({ root, projects: [storage] }), null);
  assert.equal(checkStorageClientDist({ root, projects: [app] }).level, 'WARN', 'elohim-app prebuild builds it');
  assert.equal(checkStorageClientDist({ root, projects: [a2o] }).level, 'REFUSED');
  assert.equal(checkSophiaUmd({ root, projects: [{ ...app, name: 'elohim-library' }] }), null);
  assert.equal(checkSophiaUmd({ root, projects: [app] }).level, 'WARN', 'check-sophia.sh can still fetch it');
  put(root, 'sophia/packages/sophia-element/dist/sophia-element.umd.js');
  assert.equal(checkSophiaUmd({ root, projects: [app] }).level, 'ok');
});

test('cargo-deny keeps its old scope: any selection, cargo or not', () => {
  const calls = [];
  const spawn = (cmd, args) => { calls.push([cmd, ...args].join(' ')); return { status: cmd === 'bash' ? 0 : 1, stdout: '', stderr: 'error: banned crate' }; };
  assert.equal(checkCargoDeny({ root: '/x', projects: [], spawn }), null);
  const red = checkCargoDeny({ root: '/x', projects: [app], spawn });
  assert.equal(red.level, 'REFUSED');
  assert.ok(calls.some(call => call.includes('deny --manifest-path elohim/Cargo.toml check bans')));
});

test('rows stream as each check completes', (t) => {
  const root = scratch(t);
  const seen = [];
  const spawn = () => ({ status: 0, stdout: '' });
  runPreflight({ root, projects: [app], env: {}, spawn, registry: new Map(), changed: [], onRow: r => seen.push(r.check) });
  assert.deepEqual(seen.slice(0, 2), ['submodules', 'node_modules']);
});

function readSetFixture(t) {
  const root = scratch(t);
  const registry = new Map();
  for (const name of NAMED_READ_SET_PROJECTS) {
    put(root, `${name}/build-manifest.json`, JSON.stringify({ steps: { s: { inputs: { sources: [`${name}/**`] } } } }));
    registry.set(name, { name, dir: name, manifestPath: `${name}/build-manifest.json`, steps: ['s'] });
  }
  return { root, registry };
}

test('read sets: an unnamed read in a changed crate refuses with file:line, unchanged crates are not audited', (t) => {
  const { root, registry } = readSetFixture(t);
  const audited = [];
  const read = ({ project }) => {
    audited.push(project.name);
    return project.name === 'doorway'
      ? { paths: [], unresolved: [{ file: 'doorway/build.rs', line: 4, expr: 'CARGO_MANIFEST_DIR/../x' }] }
      : { paths: [], unresolved: [] };
  };
  const red = checkReadSets({ root, registry, projects: [{ name: 'orchestrator' }], changed: ['doorway/build.rs'], read });
  assert.deepEqual(audited, ['doorway']);
  assert.equal(red.level, 'REFUSED');
  assert.match(red.why, /doorway\/build\.rs:4/);
  assert.match(red.cure, /gate-reads:/);
  const quiet = checkReadSets({ root, registry, projects: [{ name: 'orchestrator' }], changed: ['README.md'], read });
  assert.equal(quiet.level, 'ok');
  assert.equal(checkReadSets({ root, registry, projects: [{ name: 'elohim-app' }], changed: null, read }), null);
});

test('evidence: a dirty closure under a reuse-enabled gate warns before it runs', () => {
  const rows = checkEvidenceRecordable({
    root: '/x', projects: [{ name: 'eprfs' }, { name: 'elohim-app' }], reuse: ['eprfs'], env: {},
    read: () => { throw new Error('tree-scope projects are not read'); },
    dirty: () => ['.claude/data/ci-findings.jsonl'],
  });
  assert.equal(rows.length, 1);
  assert.equal(rows[0].level, 'WARN');
  assert.match(rows[0].why, /records no evidence/);
});

test('every REFUSED reaches the live run as a failure; ok and WARN do not', () => {
  const appended = [];
  emitLive([
    { check: 'element-dists', level: 'REFUSED', why: 'unbuilt', cure: 'build' },
    { check: 'cargo-deny-bans', level: 'WARN', why: 'absent' },
    { check: 'submodules', level: 'ok' },
  ], { GATE_LIVE_RUN: '/run.jsonl' }, (path, event) => appended.push([path, event]));
  assert.equal(appended.length, 1);
  assert.equal(appended[0][1].project, 'preflight');
  assert.equal(appended[0][1].failure.class, 'element-dists');
});

test('on-red policy: env, then pool-policy.json, then continue', (t) => {
  const root = scratch(t);
  assert.deepEqual(onRedPolicy({ env: {}, root }), { mode: 'continue', from: 'default' });
  put(root, 'genesis/agentic/pool-policy.json', JSON.stringify({ gate_on_red: { mode: 'stop' } }));
  assert.equal(onRedPolicy({ env: {}, root }).mode, 'stop');
  assert.equal(onRedPolicy({ env: { GATE_ON_RED: 'continue' }, root }).mode, 'continue');
});

test('gates run cheapest first by recorded p90; attested first, unknown as two minutes, ties stable', () => {
  const row = (project, value) => ({ project, kind: 'pre-push', exit: 'ok', value });
  const rows = [row('eprfs', 5200), row('orchestrator', 900), row('rust-style', 20)];
  const p = (name, kind = 'just') => ({ name, run: { kind } });
  const ordered = orderByCost([p('eprfs'), p('orchestrator'), p('new-a'), p('rust-style'), p('brit', 'attested'), p('new-b')], { rows });
  assert.deepEqual(ordered.map(x => x.name), ['brit', 'rust-style', 'new-a', 'new-b', 'orchestrator', 'eprfs']);
});

test('harvest names recurring reds, late cheap reds and time spent after the first red', (t) => {
  const root = scratch(t);
  // liveRoot needs a git checkout.
  const init = spawnSync('git', ['-C', root, 'init', '--quiet']);
  assert.equal(init.status, 0);
  let clock = Date.parse('2026-11-01T10:00:00Z');
  const now = () => new Date(clock);
  for (const i of [1, 2]) {
    const { path } = openRun({ root, sha: `${i}`.repeat(40), target: 'refs/heads/dev', now });
    clock += 40 * 60 * 1000; // a long gate first
    appendEvent(path, { kind: 'failure', project: 'preflight', failure: { class: 'element-dists', excerpt: ['unbuilt'] } }, { now });
    appendEvent(path, { kind: 'gate-verdict', project: 'preflight', verdict: { exit: 1, seconds: 2 } }, { now });
    clock += 30 * 60 * 1000;
    closeRun(path, 'failed', { now });
    clock += 60 * 1000;
  }
  const result = harvest({ root, last: 10 });
  assert.equal(result.runs, 2);
  const kinds = result.slices.map(s => s.kind);
  assert.ok(kinds.includes('recurring'), kinds.join(','));
  assert.ok(kinds.includes('late-cheap-red'), kinds.join(','));
  assert.ok(kinds.includes('spent-after-red'), kinds.join(','));
  const lines = formatHarvest(result);
  assert.match(lines[0], /QoL slice/);
  assert.ok(lines.some(line => line.includes('preflight/element-dists red in 2 of the last 2 runs')), lines.join('\n'));
});

test('the first red carries what continuing costs and the stop command, once per run; a red shows its cure', async (t) => {
  const root = scratch(t);
  assert.equal(spawnSync('git', ['-C', root, 'init', '--quiet']).status, 0);
  const { path } = openRun({ root, sha: 'a'.repeat(40), target: 'refs/heads/dev' });
  const rows = [{ project: 'eprfs', kind: 'pre-push', exit: 'ok', value: 3000 }];
  const first = await outlook(path, ['eprfs', 'brand-new'], { root, rows });
  assert.equal(first.kind, 'outlook');
  assert.equal(first.total_s, 3120, 'an unknown gate counts as two minutes');
  const line = formatEvent(first);
  assert.match(line, /2 gate\(s\) still to run, ~52 min \(eprfs ~50m, brand-new ~\?\)/);
  assert.ok(line.includes(`gate-live.mjs stop --run ${path}`), line);
  assert.equal(await outlook(path, ['eprfs'], { root, rows }), undefined, 'one decision per run');
  assert.match(formatEvent({ kind: 'outlook', remaining: [] }), /no gate left to run/);
  const red = formatEvent({ kind: 'failure', project: 'preflight', failure: { class: 'element-dists', excerpt: ['unbuilt: elohim-core', 'cure: pnpm --filter elohim-core run build'] } });
  assert.match(red, /RED preflight element-dists — unbuilt: elohim-core · cure: pnpm --filter elohim-core run build/);
});

test('harvest ignores failure-shaped lines from a gate whose verdict was green', (t) => {
  const root = scratch(t);
  assert.equal(spawnSync('git', ['-C', root, 'init', '--quiet']).status, 0);
  let clock = Date.parse('2026-11-01T10:00:00Z');
  const now = () => new Date(clock);
  const { path } = openRun({ root, sha: 'c'.repeat(40), target: 'refs/heads/x', now });
  clock += 20 * 60 * 1000;
  appendEvent(path, { kind: 'failure', project: 'orchestrator', failure: { class: 'test', excerpt: ['mesh: REFUSED — expected by the test'] } }, { now });
  appendEvent(path, { kind: 'gate-verdict', project: 'orchestrator', verdict: { exit: 0, seconds: 1100 } }, { now });
  clock += 60 * 60 * 1000;
  closeRun(path, 'failed', { now });
  assert.deepEqual(harvest({ root, last: 10 }).slices, []);
});
