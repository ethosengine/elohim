// gate-preflight.mjs: each seconds check refuses the careless miss it names, says its cure, and
// stays silent when the selected gates cannot be affected. Habit `careless-reds-cost-seconds`.
import { spawnSync } from 'node:child_process';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  checkElementDists, checkEvidenceRecordable, checkNodeModules, checkReadSets, checkSophiaUmd,
  main, checkEprBin, checkArk, checkCargoDeny, checkCargoLocks, checkCBaselineOffer, checkStorageClientDist, checkWorkspaceDists, checkSubmodules, checkTmpdir, emitLive, NAMED_READ_SET_PROJECTS, PREFLIGHT_READ_SET_PROJECTS, onRedPolicy,
  fleetRollRows, docCutRows, runPreflight, checkSubmodulePins,
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
  // Row 25: a fresh worktree is told the one command that readies it, and where to run it.
  assert.match(r.cure, /^just dev prep, run in \/x \(or by hand: /);
});

test('tmpdir: a temp root inside a git repository refuses in the first second, naming both', () => {
  const root = mkdtempSync(join(tmpdir(), 'preflight-tmpdir-'));
  try {
    mkdirSync(join(root, '.git'));
    mkdirSync(join(root, 'scratch/deeper'), { recursive: true });
    const red = checkTmpdir({ projects: [{ name: 'eprfs' }], env: { TMPDIR: join(root, 'scratch/deeper') } });
    assert.equal(red.level, 'REFUSED');
    assert.match(red.why, /scratch\/deeper is inside the git repository at /);
    assert.match(red.cure, /export TMPDIR=/);
    const clean = mkdtempSync(join(tmpdir(), 'preflight-tmpdir-clean-'));
    try {
      // Injected: a sandbox whose real /tmp sits in a repository must not red this case.
      assert.equal(checkTmpdir({ projects: [{ name: 'eprfs' }], env: { TMPDIR: clean }, exists: () => false }).level, 'ok');
    } finally { rmSync(clean, { recursive: true, force: true }); }
    assert.equal(checkTmpdir({ projects: [], env: { TMPDIR: root } }), null);
  } finally { rmSync(root, { recursive: true, force: true }); }
});

test('submodules: a checked-out commit off the pin warns, never refuses', () => {
  const spawn = () => ({ status: 0, stdout: ' abc elohim/brit\n+def sophia\n' });
  assert.equal(checkSubmodules({ root: '/x', spawn }).level, 'WARN');
});

test('node_modules: required only by a non-cargo gate', (t) => {
  const root = scratch(t);
  assert.equal(checkNodeModules({ root, projects: [storage] }), null);
  const red = checkNodeModules({ root, projects: [app] });
  assert.equal(red.level, 'REFUSED');
  assert.equal(red.cure, `just dev prep, run in ${root} (or by hand: pnpm install --frozen-lockfile --prefer-offline)`);
  assert.deepEqual(red.commands, [['pnpm', 'install', '--frozen-lockfile', '--prefer-offline']], 'executable argv is unchanged');
  put(root, 'node_modules/.modules.yaml');
  assert.equal(checkNodeModules({ root, projects: [app] }).level, 'ok');
});

test('element dists: only the element packages a selected project imports, unless its prebuild builds them', (t) => {
  const root = scratch(t);
  for (const name of ['elohim-core', 'unimported-element']) {
    put(root, `app/elohim-elements/${name}/package.json`, JSON.stringify({
      exports: { './register': { import: './dist/register.js' }, './tokens.scss': './src/tokens.scss', './styles.css': './dist/styles.css' },
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
  assert.equal(checkElementDists({ root, projects: [app] }).level, 'ok', 'a declared CSS asset with no build step is never required');
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

function cargoTree(t) {
  const root = scratch(t);
  put(root, 'ws/Cargo.toml', '[workspace]\nmembers = ["a", "b"]\n');
  put(root, 'ws/Cargo.lock', '# lock\n');
  put(root, 'ws/a/Cargo.toml', '[package]\nname = "a"\n');
  put(root, 'ws/b/Cargo.toml', '[package]\nname = "b"\n');
  return root;
}
const pa = { name: 'a', dir: 'ws/a', run: { kind: 'just', cargo: {} } };
const pb = { name: 'b', dir: 'ws/b', run: { kind: 'just', cargo: {} } };
const STALE = 'error: the lock file /x/ws/Cargo.lock needs to be updated but --locked was passed to prevent this\nIf you want to try to generate the lock file without accessing the network, remove the --locked flag and use --offline instead.';

test('cargo-locks: a lock its manifests agree with is ok, checked --locked --offline, never writing', (t) => {
  const root = cargoTree(t);
  const before = readFileSync(join(root, 'ws/Cargo.lock'));
  const calls = [];
  const spawn = (cmd, args, opts) => { calls.push({ cmd, args, opts }); return { status: 0, stderr: '' }; };
  const rows = checkCargoLocks({ root, projects: [pa], spawn, env: { RUSTFLAGS: '--cfg x' } });
  assert.deepEqual(rows.map(r => [r.check, r.level]), [['cargo-lock:ws', 'ok']]);
  assert.deepEqual(calls[0].args.slice(0, 4), ['metadata', '--locked', '--offline', '--format-version']);
  assert.equal(calls[0].opts.env.RUSTFLAGS, '');
  assert.equal('RUSTUP_TOOLCHAIN' in calls[0].opts.env, false, 'no policy pins nothing');
  assert.deepEqual(readFileSync(join(root, 'ws/Cargo.lock')), before, 'the check must not modify the lockfile');
  put(root, 'genesis/agentic/pool-policy.json', JSON.stringify({ cargo_env_overrides: { '*': { RUSTUP_TOOLCHAIN: '9.9.9' } } }));
  checkCargoLocks({ root, projects: [pa], spawn, env: {} });
  assert.equal(calls[1].opts.env.RUSTUP_TOOLCHAIN, '9.9.9', 'the policy pin is the only toolchain source');
});

test('cargo-locks: a stale lock is REFUSED naming the lockfile with the cure', (t) => {
  const root = cargoTree(t);
  const before = readFileSync(join(root, 'ws/Cargo.lock'));
  const [r] = checkCargoLocks({ root, projects: [pa], spawn: () => ({ status: 101, stderr: STALE }) });
  assert.equal(r.level, 'REFUSED');
  assert.match(r.why, /ws\/Cargo\.lock/);
  assert.match(r.cure, /cargo update --workspace --offline --manifest-path ws\/Cargo\.toml/);
  assert.match(r.cure, /commit the lockfile with the manifest change/);
  assert.deepEqual(readFileSync(join(root, 'ws/Cargo.lock')), before);
});

test('cargo-locks: a crate absent from the offline cache is a WARN, not a lock mismatch', (t) => {
  const root = cargoTree(t);
  const stderr = 'error: failed to download `foo v1.0.0`\nCaused by: attempting to make an HTTP request, but --offline was specified';
  const [r] = checkCargoLocks({ root, projects: [pa], spawn: () => ({ status: 101, stderr }) });
  assert.equal(r.level, 'WARN');
  assert.match(r.why, /not a lock mismatch/);
});

test('cargo-locks: two projects in one workspace run cargo once', (t) => {
  const root = cargoTree(t);
  let n = 0;
  const rows = checkCargoLocks({ root, projects: [pa, pb], spawn: () => { n += 1; return { status: 0 }; } });
  assert.equal(n, 1);
  assert.equal(rows.length, 1);
});

test('cargo-locks: a non-cargo project, and a DNA workspace, are not checked', (t) => {
  const root = cargoTree(t);
  put(root, 'elohim/holochain/dna/Cargo.toml', '[workspace]\n');
  const dna = { name: 'dna', dir: 'elohim/holochain/dna', run: { kind: 'just', cargo: {} } };
  const spawn = () => { throw new Error('cargo must not run'); };
  assert.equal(checkCargoLocks({ root, projects: [app, dna], spawn }), null);
});

test('rows stream as each check completes', (t) => {
  const root = scratch(t);
  const seen = [];
  const spawn = () => ({ status: 0, stdout: '' });
  runPreflight({ root, projects: [app], env: {}, spawn, registry: new Map(), changed: [], onRow: r => seen.push(r.check) });
  assert.deepEqual(seen.slice(0, 4), ['epr-bin', 'submodules', 'tmpdir', 'node_modules']);
});

function readSetFixture(t) {
  const root = scratch(t);
  const registry = new Map();
  for (const name of PREFLIGHT_READ_SET_PROJECTS) {
    put(root, `${name}/build-manifest.json`, JSON.stringify({ steps: { s: { inputs: { sources: [`${name}/**`] } } } }));
    registry.set(name, { name, dir: name, manifestPath: `${name}/build-manifest.json`, steps: ['s'] });
  }
  return { root, registry };
}

test('read sets: eprfs is audited, so a new unnamed test-file read refuses before the gates', (t) => {
  const { root, registry } = readSetFixture(t);
  const read = ({ project }) => (project.name === 'eprfs'
    ? { paths: [], unresolved: [{ file: 'eprfs/epr-cli/tests/session_envelope.rs', line: 269, expr: 'CARGO_MANIFEST_DIR/../../..' }] }
    : { paths: [], unresolved: [] });
  const red = checkReadSets({ root, registry, projects: [{ name: 'eprfs' }], changed: ['eprfs/epr-cli/tests/session_envelope.rs'], read });
  assert.equal(red.level, 'REFUSED');
  assert.match(red.why, /session_envelope\.rs:269/);
});

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
    // orchestrator is a JS gate: its reads cannot be named, so it keys on the commit tree.
    root: '/x', projects: [{ name: 'orchestrator' }, { name: 'elohim-app' }], reuse: ['orchestrator'], env: {},
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

// Row 29.2: the C-baseline row warns and withholds the producer's offer; it never refuses the gate.
const DECLARED = { 'x86_64-unknown-linux-gnu': { isa: 'x86-64', cflags: '-march=x86-64', workspaces: ['elohim/eprfs'] } };
const eprfs = { name: 'eprfs', dir: 'elohim/eprfs', run: { kind: 'root-just', cargo: { workspace: 'elohim/eprfs' } } };
const orchestrator = { name: 'orchestrator', dir: 'genesis/orchestrator', run: { kind: 'just', cargo: { workspace: 'elohim/eprfs' } } };
const storageWs = { ...storage, run: { kind: 'just', cargo: { workspace: 'elohim/elohim-storage' } } };

test('c-baseline: a project on a declared workspace with a C finding WARNs, names the cure and withholds the offer', () => {
  const finding = { triple: 'x86_64-unknown-linux-gnu', check: 'env', variable: 'CFLAGS', why: 'CFLAGS is set (-O3)', cure: 'unset CFLAGS' };
  const seen = [];
  const r = checkCBaselineOffer({ root: '/x', projects: [orchestrator, app], env: { CFLAGS: '-O3' }, record: DECLARED, hostTriple: 'x86_64-unknown-linux-gnu',
    check: options => { seen.push(options.workspaces); return [finding]; } });
  assert.deepEqual(seen, [['elohim/eprfs']]);
  assert.equal(r.check, 'c-baseline');
  assert.equal(r.level, 'WARN', 'never REFUSED: the device\'s own gate outcome does not change');
  assert.match(r.why, /CFLAGS is set/);
  assert.match(r.why, /offer .*withheld/);
  assert.match(r.cure, /unset CFLAGS/);
  assert.deepEqual(r.withholds, { offer: ['orchestrator'] });
});

test('c-baseline: clean is ok; no declared workspace, another host triple, or no record is silent', () => {
  const clean = checkCBaselineOffer({ root: '/x', projects: [eprfs], env: {}, record: DECLARED, hostTriple: 'x86_64-unknown-linux-gnu', check: () => [] });
  assert.equal(clean.level, 'ok');
  assert.equal(clean.withholds, undefined);
  const never = () => { throw new Error('must not check'); };
  assert.equal(checkCBaselineOffer({ root: '/x', projects: [storageWs, app], env: {}, record: DECLARED, hostTriple: 'x86_64-unknown-linux-gnu', check: never }), null);
  assert.equal(checkCBaselineOffer({ root: '/x', projects: [eprfs], env: {}, record: DECLARED, hostTriple: 'aarch64-unknown-linux-gnu', check: never }), null);
  assert.equal(checkCBaselineOffer({ root: '/x', projects: [eprfs], env: {}, record: null, hostTriple: 'x86_64-unknown-linux-gnu', check: never }), null);
});

test('c-baseline: runPreflight reads the record from pool-policy.json and the row never refuses', (t) => {
  const root = scratch(t);
  put(root, 'genesis/agentic/pool-policy.json', JSON.stringify({ declared_targets: DECLARED }));
  const spawn = (cmd) => (cmd === 'git' || cmd === 'bash' || cmd === 'cargo' ? { status: 0, stdout: '' } : { status: 0, stdout: '#define __AVX2__ 1\n' });
  const rows = runPreflight({ root, projects: [eprfs], env: { CFLAGS: '-O3' }, spawn, registry: new Map(), changed: [], hostTriple: 'x86_64-unknown-linux-gnu' });
  const row = rows.find(r => r.check === 'c-baseline');
  assert.equal(row?.level, 'WARN', JSON.stringify(rows.map(r => [r.check, r.level])));
  assert.match(row.why, /__AVX2__/);
  assert.match(row.why, /CFLAGS/);
});

// The env judged is the one the gate builds with: the caller's, with gateChildEnv's
// GATE_CARGO_ENV map (manifest run.cargo.env, pool-policy cargo_env_overrides, declared
// targets) exported over it as run-local-gate.sh does. A clean caller proves nothing.
const TRIPLE = 'x86_64-unknown-linux-gnu';
const cleanCc = (calls = []) => (cmd, args, opts) => { calls.push({ cmd, args, env: opts?.env }); return { status: 0, stdout: '#define __SSE2__ 1\n', stderr: '' }; };

test('c-baseline: a C-family key the gate itself sets (pool-policy cargo_env_overrides) withholds, from a clean caller', (t) => {
  const root = scratch(t);
  put(root, 'genesis/agentic/pool-policy.json', JSON.stringify({ declared_targets: DECLARED, cargo_env_overrides: { eprfs: { CFLAGS: '-O3 -mavx2' } } }));
  const r = checkCBaselineOffer({ root, projects: [eprfs], env: {}, record: DECLARED, hostTriple: TRIPLE, spawn: cleanCc() });
  assert.equal(r?.level, 'WARN', JSON.stringify(r));
  assert.match(r.why, /CFLAGS="-O3 -mavx2" is set/);
  assert.deepEqual(r.withholds, { offer: ['eprfs'] });
});

test('c-baseline: a manifest run.cargo.env C-family key withholds that project only', (t) => {
  const root = scratch(t);
  put(root, 'genesis/agentic/pool-policy.json', JSON.stringify({ declared_targets: DECLARED }));
  const tuned = { ...orchestrator, run: { ...orchestrator.run, cargo: { workspace: 'elohim/eprfs', env: { CXXFLAGS: '-march=native' } } } };
  const r = checkCBaselineOffer({ root, projects: [eprfs, tuned], env: {}, record: DECLARED, hostTriple: TRIPLE, spawn: cleanCc() });
  assert.equal(r?.level, 'WARN', JSON.stringify(r));
  assert.match(r.why, /CXXFLAGS="-march=native" is set/);
  assert.deepEqual(r.withholds, { offer: ['orchestrator'] });
});

test('c-baseline: the gate\'s own declared flags pass, and the probe runs under the build env', (t) => {
  const root = scratch(t);
  put(root, 'genesis/agentic/pool-policy.json', JSON.stringify({ declared_targets: DECLARED, cargo_env_overrides: { eprfs: { HOST_CC: 'gcc-pinned' } } }));
  const calls = [];
  // A stale declared variable in the caller is overwritten by the record's value in the build.
  const r = checkCBaselineOffer({ root, projects: [eprfs], env: { CFLAGS_x86_64_unknown_linux_gnu: '-march=x86-64-v3' }, record: DECLARED, hostTriple: TRIPLE, spawn: cleanCc(calls) });
  assert.equal(r?.level, 'ok', JSON.stringify(r));
  assert.equal(calls[0].cmd, 'gcc-pinned', 'the compiler the gate env names');
  assert.equal(calls[0].env.CFLAGS_x86_64_unknown_linux_gnu, '-march=x86-64');
  assert.equal(calls[0].env.GATE_CARGO_ENV, undefined, 'run-local-gate.sh exports the map and unsets it');
});

test('a withholding WARN reaches the live run as a notice, never a failure; an ordinary WARN does not', () => {
  const appended = [];
  emitLive([
    { check: 'c-baseline', level: 'WARN', why: 'CFLAGS is set\nsecond line', cure: 'unset CFLAGS', withholds: { offer: ['eprfs'] } },
    { check: 'sophia-umd', level: 'WARN', why: 'absent', cure: 'build' },
    { check: 'c-baseline', level: 'ok', withholds: { offer: ['eprfs'] } },
    { check: 'element-dists', level: 'REFUSED', why: 'unbuilt', cure: 'build' },
  ], { GATE_LIVE_RUN: '/run.jsonl' }, (path, event) => appended.push(event));
  assert.equal(appended.length, 2);
  // The notice contract: { kind: 'notice', project, notice: { class: 'offer-withheld', text } }.
  const [notice, red] = appended;
  assert.deepEqual(Object.keys(notice).sort(), ['kind', 'notice', 'project']);
  assert.equal(notice.kind, 'notice');
  assert.equal(notice.project, 'preflight');
  assert.equal(notice.failure, undefined, 'never a red');
  assert.deepEqual(Object.keys(notice.notice).sort(), ['class', 'text']);
  assert.equal(notice.notice.class, 'offer-withheld');
  assert.equal(notice.notice.text, 'c-baseline: offer of eprfs withheld, the gate still runs: CFLAGS is set second line — cure: unset CFLAGS');
  assert.equal(red.kind, 'failure', 'a REFUSED is still a red');
  assert.equal(red.failure.class, 'element-dists');
});

// A tiny repository whose committed manifests make sdk/** an Edge build input.
function fleetRepo(t) {
  const dir = scratch(t);
  const env = Object.fromEntries(Object.entries(process.env).filter(([k]) => !k.startsWith('GIT_') || k.startsWith('GIT_CONFIG_')));
  const git = (...args) => {
    const r = spawnSync('git', ['-C', dir, '-c', 'user.name=t', '-c', 'user.email=t@t', '-c', 'commit.gpgsign=false', '-c', 'core.hooksPath=/dev/null', ...args], { encoding: 'utf8', env });
    assert.equal(r.status, 0, r.stderr);
    return r.stdout.trim();
  };
  put(dir, 'e/build-manifest.json', JSON.stringify({ pipeline: 'elohim-edge', jenkinsPath: 'e/Jenkinsfile', steps: { 'cargo-build-storage': { inputs: { sources: ['sdk/**'] }, depends: [] } } }));
  put(dir, 'a/build-manifest.json', JSON.stringify({ pipeline: 'elohim', jenkinsPath: 'Jenkinsfile', steps: { 'build-app': { inputs: { sources: ['app/**'] }, depends: [] } } }));
  git('init', '--quiet', '-b', 'dev');
  git('add', '-A');
  git('commit', '--quiet', '-m', 'base');
  const base = git('rev-parse', 'HEAD');
  const commit = (path, message) => { put(dir, path, 'x\n'); git('add', '-A'); git('commit', '--quiet', '-m', message); return git('rev-parse', 'HEAD'); };
  const spawn = (cmd, args, opts) => spawnSync(cmd, args, { ...opts, env });
  return { dir, base, commit, spawn };
}

test('fleet-roll: an undeclared Edge selection on a dev push is a preflight row, and its REFUSED reaches the live run', (t) => {
  const { dir, base, commit, spawn } = fleetRepo(t);
  const tip = commit('sdk/schema.json', 'feat(sdk): one field');
  const env = { GATE_PUSH_TARGETS: 'refs/heads/dev', GATE_PUSH_REFS: `refs/heads/dev ${tip} ${base}` };
  const rows = runPreflight({ root: dir, projects: [], env, spawn, registry: new Map(), changed: null, fleet: { root: dir, spawn } });
  const fleet = rows.find(r => r.check === 'fleet-roll');
  assert.equal(fleet?.level, 'REFUSED', JSON.stringify(rows));
  assert.match(fleet.why, /CI would select an Edge build and its deploy step for this push to dev: elohim-edge stale steps cargo-build-storage/);
  const appended = [];
  emitLive(rows, { GATE_LIVE_RUN: '/run.jsonl' }, (path, event) => appended.push(event));
  const red = appended.find(e => e.failure.class === 'fleet-roll');
  assert.equal(red.project, 'preflight');
  assert.match(red.failure.excerpt.at(-1), /^cure: .*\[edge:roll\]/);
  // The early block's mark file, holding exactly this ref list, is the only proof that skips it here.
  const mark = join(dir, 'mark');
  const run = extra => runPreflight({ root: dir, projects: [], env: { ...env, ...extra }, spawn, registry: new Map(), changed: null, fleet: { root: dir, spawn } });
  writeFileSync(mark, env.GATE_PUSH_REFS);
  assert.equal(run({ GATE_FLEET_ROLL_MARK: mark }).some(r => r.check === 'fleet-roll'), false);
  writeFileSync(mark, `refs/heads/dev ${base} ${base}`);
  assert.equal(run({ GATE_FLEET_ROLL_MARK: mark }).some(r => r.check === 'fleet-roll'), true, 'a mark for other refs proves nothing');
  assert.equal(run({ GATE_FLEET_ROLL_DONE: String(process.ppid) }).some(r => r.check === 'fleet-roll'), true, 'a parent pid proves nothing');
});

test('fleet-roll under an older hook (no GATE_PUSH_REFS): silent with no Edge build, refused with one', (t) => {
  const { dir, commit, spawn } = fleetRepo(t);
  commit('app/x.ts', 'feat: app');
  const env = { GATE_PUSH_TARGETS: 'refs/heads/dev' };
  const run = changed => runPreflight({ root: dir, projects: [], env, spawn, registry: new Map(), changed, fleet: { root: dir, spawn } });
  const quiet = run(['app/x.ts']);
  assert.equal(quiet.some(r => r.check === 'fleet-roll'), false);
  assert.equal(quiet.some(r => r.level === 'REFUSED'), false, 'the preflight exits 0');
  const red = run(['sdk/schema.json']).find(r => r.check === 'fleet-roll');
  assert.equal(red.level, 'REFUSED');
  assert.match(red.cure, /^check the launcher first: test -x <worktree>\/\.husky\/_\/pre-push .*-c core\.hooksPath=<worktree>\/\.husky\/_/);
});

test('doc-cut: a document cut on a dev push is a preflight row, its REFUSED reaches the live run, and the mark skips it', (t) => {
  const { dir, spawn } = fleetRepo(t);
  const long = Array.from({ length: 300 }, (_, i) => `row ${i}\n`).join('');
  put(dir, 'docs/plan.md', long);
  const git = (...args) => { const r = spawn('git', ['-C', dir, '-c', 'user.name=t', '-c', 'user.email=t@t', '-c', 'commit.gpgsign=false', '-c', 'core.hooksPath=/dev/null', ...args], { encoding: 'utf8' }); assert.equal(r.status, 0, r.stderr); return r.stdout.trim(); };
  git('add', '-A'); git('commit', '--quiet', '-m', 'add plan');
  const base = git('rev-parse', 'HEAD');
  put(dir, 'docs/plan.md', 'row 0\n');
  git('add', '-A'); git('commit', '--quiet', '-m', 'docs: append a section');
  const tip = git('rev-parse', 'HEAD');
  const env = { GATE_PUSH_TARGETS: 'refs/heads/dev', GATE_PUSH_REFS: `refs/heads/dev ${tip} ${base}` };
  const run = extra => runPreflight({ root: dir, projects: [], env: { ...env, ...extra }, spawn, registry: new Map(), changed: null, fleet: { root: dir, spawn }, docCut: { root: dir, spawn } });
  const rows = run({});
  const cut = rows.find(r => r.check === 'doc-cut');
  assert.equal(cut?.level, 'REFUSED', JSON.stringify(rows));
  assert.match(cut.why, /push to dev: docs\/plan\.md falls from 300 to 1 lines/);
  const appended = [];
  emitLive(rows, { GATE_LIVE_RUN: '/run.jsonl' }, (path, event) => appended.push(event));
  const red = appended.find(e => e.failure?.class === 'doc-cut');
  assert.equal(red.project, 'preflight');
  assert.match(red.failure.excerpt.at(-1), /^cure: .*\[doc:cut\]/);
  const mark = join(dir, 'dc-mark');
  writeFileSync(mark, env.GATE_PUSH_REFS);
  assert.equal(run({ GATE_DOC_CUT_MARK: mark }).some(r => r.check === 'doc-cut'), false);
  writeFileSync(mark, `refs/heads/dev ${base} ${base}`);
  assert.equal(run({ GATE_DOC_CUT_MARK: mark }).some(r => r.check === 'doc-cut'), true, 'a mark for other refs proves nothing');
});

test('doc-cut: a guard module that cannot load still gives a row, red on a protected push', () => {
  const error = new Error('Cannot find module x');
  const red = docCutRows({ GATE_PUSH_TARGETS: 'refs/heads/dev' }, {}, null, error);
  assert.equal(red.level, 'REFUSED');
  assert.match(red.why, /^the doc-cut check could not load: Cannot find module x$/);
  assert.equal(docCutRows({ GATE_PUSH_TARGETS: 'refs/heads/self/a' }, {}, null, error).level, 'WARN');
});

test('fleet-roll: a guard module that cannot load still gives a row, red on a protected push in every mode', () => {
  const error = new Error("Cannot find package 'picomatch'");
  const red = fleetRollRows({ GATE_PUSH_TARGETS: 'refs/heads/dev' }, null, {}, null, error);
  assert.equal(red.level, 'REFUSED');
  assert.match(red.why, /^the fleet-roll check could not load: Cannot find package 'picomatch'$/);
  assert.equal(fleetRollRows({ GATE_PUSH_TARGETS: 'refs/heads/self/a' }, null, {}, null, error).level, 'WARN');
  assert.equal(fleetRollRows({}, ['x'], {}, null, error).level, 'WARN');
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

test('ark: required only by agentic-shift, satisfied by ARK_BIN, PATH or the dev slot', () => {
  const shift = { name: 'agentic-shift', dir: 'genesis/agentic' };
  const other = { name: 'orchestrator', dir: 'genesis/orchestrator' };
  const none = () => false;
  assert.equal(checkArk({ projects: [other], env: {}, executable: none }), null);
  const refused = checkArk({ projects: [shift], env: { PATH: '/usr/bin' }, executable: none });
  assert.equal(refused.level, 'REFUSED');
  assert.equal(refused.cure, 'just gate elohim-ark');
  assert.equal(checkArk({ projects: [shift], env: { ARK_BIN: '/x/ark' }, executable: p => p === '/x/ark' }).level, 'ok');
  assert.equal(checkArk({ projects: [shift], env: { PATH: '/a:/b' }, executable: p => p === '/b/ark' }).level, 'ok');
  assert.equal(checkArk({ projects: [shift], env: {}, executable: p => p.endsWith('/dev/debug/ark') }).level, 'ok');
});

test('workspace dists: a gate whose package.json names @elohim/epr needs its dist', (t) => {
  const root = mkdtempSync(join(tmpdir(), 'preflight-wsdist-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, 'app/x'), { recursive: true });
  writeFileSync(join(root, 'app/x/package.json'), JSON.stringify({ dependencies: { '@elohim/epr': 'workspace:*' } }));
  mkdirSync(join(root, 'tools/y'), { recursive: true });
  writeFileSync(join(root, 'tools/y/package.json'), JSON.stringify({ dependencies: {} }));
  const app = { name: 'x', dir: 'app/x' };
  const other = { name: 'y', dir: 'tools/y' };
  assert.equal(checkWorkspaceDists({ root, projects: [other] }), null);
  const refused = checkWorkspaceDists({ root, projects: [app] });
  assert.equal(refused.level, 'REFUSED');
  assert.equal(refused.cure, 'pnpm --filter @elohim/epr run build');
  mkdirSync(join(root, 'elohim/sdk/epr-ts/dist'), { recursive: true });
  writeFileSync(join(root, 'elohim/sdk/epr-ts/dist/index.js'), '');
  assert.equal(checkWorkspaceDists({ root, projects: [app] }).level, 'ok');
});

// A fixture superproject whose suite list declares one submodule, pinned to a commit made in a second repository.
function pinFixture(t, { present, none = false }) {
  const env = {
    ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_CONFIG_NOSYSTEM: '1',
    GIT_AUTHOR_NAME: 't', GIT_AUTHOR_EMAIL: 't@example.invalid', GIT_COMMITTER_NAME: 't', GIT_COMMITTER_EMAIL: 't@example.invalid',
  };
  const run = (cwd, ...args) => {
    const r = spawnSync('git', args, { cwd, env, encoding: 'utf8' });
    assert.equal(r.status, 0, `git ${args.join(' ')}: ${r.stderr}`);
    return r.stdout.trim();
  };
  const base = scratch(t);
  const upstream = join(base, 'upstream');
  const root = join(base, 'root');
  mkdirSync(upstream); mkdirSync(root);
  run(upstream, 'init', '-q');
  put(upstream, 'f.txt', 'x');
  run(upstream, 'add', '.');
  run(upstream, 'commit', '-q', '-m', 'pin');
  const oid = run(upstream, 'rev-parse', 'HEAD');
  run(root, 'init', '-q');
  put(root, 'genesis/orchestrator/suite-reads.json', JSON.stringify({
    groups: [{ files: [{ file: 'x.test.mjs', reads: ['**/build-manifest.json', 'vendor/thing'] }] }],
  }));
  if (none) put(root, '.gitmodules', '[submodule "vendor/thing"]\n\tpath = vendor/thing\n\turl = ./x\n\tupdate = none\n');
  run(root, 'add', '.');
  run(root, 'update-index', '--add', '--cacheinfo', `160000,${oid},vendor/thing`);
  run(root, 'commit', '-q', '-m', 'root');
  if (present) run(base, 'clone', '-q', upstream, join(root, 'vendor/thing'));
  return { root, oid };
}
const pinGate = { name: 'orchestrator', dir: 'genesis/orchestrator', run: { kind: 'just' } };

test('submodule pins: a declared submodule whose pin is in no store refuses, naming it and the init cure', (t) => {
  const { root, oid } = pinFixture(t, { present: false });
  const r = checkSubmodulePins({ root, projects: [pinGate] });
  assert.equal(r.level, 'REFUSED');
  assert.match(r.why, new RegExp(`vendor/thing@${oid.slice(0, 12)}`));
  assert.equal(r.cure, 'git submodule update --init vendor/thing');
});

test('submodule pins: the same fixture with the pinned object present passes', (t) => {
  const { root } = pinFixture(t, { present: true });
  assert.equal(checkSubmodulePins({ root, projects: [pinGate] }).level, 'ok');
});

test('submodule pins: no row unless the orchestrator project is selected', (t) => {
  const { root } = pinFixture(t, { present: false });
  assert.equal(checkSubmodulePins({ root, projects: [app] }), null);
});

test('submodule pins: one declared update = none is never named, since init would skip it', (t) => {
  const { root } = pinFixture(t, { present: false, none: true });
  assert.equal(checkSubmodulePins({ root, projects: [pinGate] }).level, 'ok');
});

function eprFixture(t) {
  const root = scratch(t);
  const cid = 'bafyreicurrent';
  const current = join(root, 'store/epr', cid, 'epr');
  put(root, `store/epr/${cid}/epr`);
  put(root, 'store/bin/bin-launch');
  mkdirSync(join(root, '.local/bin'), { recursive: true });
  symlinkSync(join(root, 'store/bin/bin-launch'), join(root, '.local/bin/epr'));
  symlinkSync(cid, join(root, 'store/epr/current'));
  return { root, current, env: { HOME: root } };
}

test('epr-bin: unset is ok without reading a device', () => {
  assert.equal(checkEprBin({ env: {} }).level, 'ok');
});

test('epr-bin: current path, symlink and same store cid are ok', t => {
  const { root, current, env } = eprFixture(t);
  for (const path of [current, join(root, 'store/epr/current/epr')]) {
    assert.equal(checkEprBin({ env: { ...env, EPR_BIN: path } }).level, 'ok');
  }
  const copy = join(root, 'copy/epr/bafyreicurrent/epr');
  put(root, 'copy/epr/bafyreicurrent/epr');
  assert.equal(checkEprBin({ env: { ...env, EPR_BIN: copy } }).level, 'ok');
});

const release = (root, cid, ingestedAt) => {
  put(root, `store/epr/${cid}/epr`);
  put(root, `store/epr/${cid}/record.json`, JSON.stringify({ cid, ingestedAt }));
  return join(root, 'store/epr', cid, 'epr');
};

test('epr-bin: a store release older than the head refuses with both paths, cost and cure; override warns', t => {
  const { root, current, env } = eprFixture(t);
  put(root, 'store/epr/bafyreicurrent/record.json', JSON.stringify({ cid: 'bafyreicurrent', ingestedAt: '2026-11-21T20:18:26Z' }));
  const inherited = release(root, 'bafyreiolder', '2026-11-02T10:00:00Z');
  for (const override of [undefined, '1', '0']) {
    const r = checkEprBin({ env: { ...env, EPR_BIN: inherited, GATE_ALLOW_EPR_BIN: override } });
    assert.equal(r.level, override === '1' ? 'WARN' : 'REFUSED');
    assert.ok(r.why.includes(inherited));
    assert.ok(r.why.includes(current));
    assert.match(r.why, /ingested before its head/);
    assert.match(r.why, /native evaluator; an older one makes evidence unavailable, so nothing is reused/);
    assert.match(r.why, /a new Claude session unsets it at start \(device-arrive\)$/);
    assert.equal(r.cure, 'unset EPR_BIN');
  }
});

test('epr-bin: a candidate outside the store, a newer release, or an undated one warns and is left set', t => {
  const { root, env } = eprFixture(t);
  put(root, 'store/epr/bafyreicurrent/record.json', JSON.stringify({ cid: 'bafyreicurrent', ingestedAt: '2026-11-21T20:18:26Z' }));
  const outside = join(root, 'target/debug/epr');
  put(root, 'target/debug/epr');
  const newer = release(root, 'bafyreinewer', '2026-11-23T05:00:00Z');
  put(root, 'store/epr/bafyreiundated/epr');
  const undated = join(root, 'store/epr/bafyreiundated/epr');
  for (const candidate of [outside, newer, undated]) {
    const r = checkEprBin({ env: { ...env, EPR_BIN: candidate } });
    assert.equal(r.level, 'WARN', candidate);
    assert.match(r.why, /is a candidate, not this device's head epr/);
    assert.match(r.why, /it is left set/);
  }
});

test('epr-bin: unreadable current warns, including a malformed authoritative head', t => {
  const { root, env } = eprFixture(t);
  const candidate = { ...env, EPR_BIN: '/candidate/epr' };
  put(root, 'store/epr/current.json', '{broken');
  const r = checkEprBin({ env: candidate });
  assert.equal(r.level, 'WARN');
  assert.equal(r.why, "EPR_BIN is set (/candidate/epr) and this device's current epr could not be read, so it is left set");
  assert.equal(checkEprBin({ env: { ...candidate, HOME: join(root, 'absent') } }).level, 'WARN');
});

test('epr-bin: current.json takes precedence and explicit store overrides locate it', t => {
  const { root, env } = eprFixture(t);
  put(root, 'store/epr/bafyreinew/epr');
  put(root, 'store/epr/current.json', JSON.stringify({ cid: 'bafyreinew' }));
  for (const location of [{}, { BIN_STORE: join(root, 'store') }, { EPR_STORE: join(root, 'store/epr') }]) {
    assert.equal(checkEprBin({ env: { ...env, ...location, EPR_BIN: join(root, 'store/epr/bafyreinew/epr') } }).level, 'ok');
  }
});

test('epr-bin: first preflight row refuses and the single-check CLI exits 3, its own code', t => {
  const { root, env } = eprFixture(t);
  put(root, 'store/epr/bafyreicurrent/record.json', JSON.stringify({ cid: 'bafyreicurrent', ingestedAt: '2026-11-21T20:18:26Z' }));
  const candidate = { ...env, EPR_BIN: release(root, 'bafyreiolder', '2026-11-02T10:00:00Z') };
  const rows = runPreflight({ root, projects: [], env: candidate, registry: new Map(), changed: [],
    spawn: () => ({ status: 0, stdout: '' }) });
  assert.equal(rows[0].check, 'epr-bin');
  assert.equal(rows[0].level, 'REFUSED');
  let output = '';
  t.mock.method(process.stdout, 'write', chunk => { output += chunk; return true; });
  for (const override of ['0', '1']) {
    output = '';
    const status = main(['--epr-bin'], { ...candidate, GATE_ALLOW_EPR_BIN: override });
    assert.equal(status, override === '1' ? 0 : 3);
    assert.match(output, /\[preflight\] (REFUSED|WARN) epr-bin:/);
    assert.match(output, /cure: unset EPR_BIN/);
  }
});
