/**
 * The input-class policy: which of a manifest step's two input classes a device gate selects and
 * keys on, read by selection (graph-walker), propagation (gate-oracle) and the key (gate-evidence)
 * from one policy per run, and its shadow (GATE_INPUTS=shadow|off; there is no `on` yet).
 *
 * Every brit here is a double that understands `--inputs`; git is real, over temporary repositories
 * this file removes. No test starts a process that outlives it: every subprocess is spawnSync.
 *
 * Run:
 *   node --test genesis/orchestrator/gate-inputs.test.mjs
 */
import { test, describe, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, utimesSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  INPUTS_STATE_ENV, LEGACY_ALL, SHADOW_BUDGET, SOURCES, UNAVAILABLE_LINE, clearGateInputsCache, endShadow, inShadowBudget,
  inputsMode, matchInputs, probeBrit, refusesInputsFlag, resolveGateInputs, sourcesPatterns, stripFunctionSuffix,
} from './gate-inputs.mjs';
import { staleSteps, walkGraph } from './graph-walker.mjs';
import { britAffected } from './gate-oracle.mjs';
import { stepFingerprints } from './gate-evidence.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const GIT = spawnSync('bash', ['-c', 'command -v git'], { encoding: 'utf8' }).stdout.trim();
const PATH = ['/usr/local/bin', '/usr/bin', '/bin'].join(':');
const CAPABLE_HELP = 'Usage: brit build affected [OPTIONS]\n      --inputs <INPUTS>\n          [possible values: sources, all]\n';
const LEGACY_HELP = 'Usage: brit build affected [OPTIONS]\n      --files <FILES>\n';
const CLAP_REFUSAL = "error: unexpected argument '--inputs' found\n\nUsage: brit build fingerprint [OPTIONS] <MANIFEST>\n";

beforeEach(() => clearGateInputsCache());

function scratch(t, prefix) {
  const dir = mkdtempSync(join(tmpdir(), prefix));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  return dir;
}

/** An executable file whose identity (realpath, inode, size, mtime) a policy binds to. */
function fakeBrit(t) {
  const bins = scratch(t, 'gate-inputs-bin-');
  for (const name of ['brit', 'brit-build-ref']) {
    writeFileSync(join(bins, name), '#!/bin/sh\nexit 1\n');
    chmodSync(join(bins, name), 0o755);
  }
  return { bins, brit: join(bins, 'brit'), buildRef: join(bins, 'brit-build-ref') };
}

const helpSpawn = (help, calls = []) => (bin, args) => {
  calls.push([bin, ...args].join(' '));
  if (args[2] === '--help') return { status: 0, stdout: help, stderr: '' };
  return { status: 1, stdout: '', stderr: 'unexpected' };
};

// ---------------------------------------------------------------- the switch and the rule

test('GATE_INPUTS defaults to shadow and reads anything else, on included, as shadow', function inputs_mode_switch() {
  assert.equal(inputsMode({}), 'shadow');
  assert.equal(inputsMode({ GATE_INPUTS: 'off' }), 'off');
  assert.equal(inputsMode({ GATE_INPUTS: ' off ' }), 'off');
  assert.equal(inputsMode({ GATE_INPUTS: 'on' }), 'shadow', 'there is deliberately no on yet');
  assert.equal(inputsMode({ GATE_INPUTS: 'sources' }), 'shadow');
});

test('the suffix strip is rakia-core\'s: only a final @identifier after a non-empty path', () => {
  // rakia-core inputs.rs tests, case for case.
  assert.equal(stripFunctionSuffix('Jenkinsfile@buildApp'), 'Jenkinsfile');
  assert.equal(stripFunctionSuffix('Jenkinsfile'), 'Jenkinsfile');
  assert.equal(stripFunctionSuffix('ci/Jenkinsfile@build_app2'), 'ci/Jenkinsfile');
  assert.equal(stripFunctionSuffix('node_modules/@scope/pkg'), 'node_modules/@scope/pkg');
  assert.equal(stripFunctionSuffix('node_modules/@scope/pkg/**'), 'node_modules/@scope/pkg/**');
  assert.equal(stripFunctionSuffix('@scope/pkg/Jenkinsfile@build'), '@scope/pkg/Jenkinsfile');
  assert.equal(stripFunctionSuffix('ci/file@1x'), 'ci/file@1x');
  assert.equal(stripFunctionSuffix('@build'), '@build');
  assert.deepEqual(sourcesPatterns({ sources: ['project/**'], buildProcess: ['project/Jenkinsfile@buildApp'] }),
    { include: ['project/**'], exclude: ['project/Jenkinsfile'] });
});

describe('the sources class of a borrowed step, against the native journey cases', () => {
  // brit tests/cli-journey/tests/input_classes.rs and rakia-core inputs.rs, as JS cases:
  // [case, inputs, changed file, legacy reasons, sources reasons]
  const cases = [
    ['a pipeline-only change selects legacy and not sources',
      { sources: ['src/**'], buildProcess: ['ci/pipeline.yml'] }, 'ci/pipeline.yml', ['buildProcess: ci/pipeline.yml'], []],
    ['a source-only change selects both',
      { sources: ['src/**'], buildProcess: ['ci/pipeline.yml'] }, 'src/input.txt', ['source: src/input.txt'], ['source: src/input.txt']],
    ['a broad source glob subtracts its own pipeline file',
      { sources: ['project/**'], buildProcess: ['project/Jenkinsfile'] }, 'project/Jenkinsfile', ['source: project/Jenkinsfile', 'buildProcess: project/Jenkinsfile'], []],
    ['the subtraction strips a function suffix',
      { sources: ['project/**'], buildProcess: ['project/Jenkinsfile@buildApp'] }, 'project/Jenkinsfile', ['source: project/Jenkinsfile', 'buildProcess: project/Jenkinsfile@buildApp'], []],
    ['the broad glob still selects its other files',
      { sources: ['project/**'], buildProcess: ['project/Jenkinsfile@buildApp'] }, 'project/src/lib.rs', ['source: project/src/lib.rs'], ['source: project/src/lib.rs']],
    // Today's matcher cuts at the first `@` (the advisory on the native branch), so it gives no
    // buildProcess reason here; LEGACY_ALL keeps that byte for byte, SOURCES strips as rakia does.
    ['a path with @ that is no suffix is subtracted whole',
      { sources: ['ci/**'], buildProcess: ['ci/file@1x'] }, 'ci/file@1x', ['source: ci/file@1x'], []],
    ['a scoped package path is not cut at its @',
      { sources: ['node_modules/@scope/pkg/**'], buildProcess: [] }, 'node_modules/@scope/pkg/index.js', ['source: node_modules/@scope/pkg/index.js'], ['source: node_modules/@scope/pkg/index.js']],
    ['a governance path selects neither',
      { sources: ['src/**'], buildProcess: [] }, 'src/.epr-meta/x.habit.md', [], []],
  ];
  for (const [name, inputs, file, legacy, sources] of cases) {
    test(name, function sources_rule_native_case() {
      assert.deepEqual(LEGACY_ALL.stepReasons(inputs, [file]), legacy);
      assert.deepEqual(SOURCES.stepReasons(inputs, [file]), sources);
    });
  }
  test('LEGACY_ALL is today\'s matcher, byte for byte', function legacy_all_is_todays_matcher() {
    for (const [, inputs, file] of cases) assert.deepEqual(LEGACY_ALL.stepReasons(inputs, [file]), matchInputs(inputs, [file]));
  });
  test('one path is classified per step: a source of one step stays one when it is another\'s buildProcess', () => {
    const manifests = [{ path: 'm.json', content: { pipeline: 'fixture', steps: {
      consumer: { inputs: { sources: ['shared.txt'], buildProcess: [] }, depends: [] },
      executor: { inputs: { sources: ['other/**'], buildProcess: ['shared.txt'] }, depends: [] },
    } } }];
    assert.deepEqual([...staleSteps(manifests, ['shared.txt'], LEGACY_ALL).keys()], ['fixture:consumer', 'fixture:executor']);
    assert.deepEqual([...staleSteps(manifests, ['shared.txt'], SOURCES).keys()], ['fixture:consumer']);
  });
});

// ---------------------------------------------------------------- selection

const selectionManifests = () => [{
  path: './p/build-manifest.json',
  content: {
    pipeline: 'fixture',
    steps: {
      build: { inputs: { sources: ['p/**'], buildProcess: ['p/Jenkinsfile@build'] }, depends: [] },
      image: { inputs: { sources: ['img/**'], buildProcess: ['scripts/ci/image.sh'] }, depends: ['build'] },
    },
    gate: { projects: {
      borrowed: { dir: 'p', steps: ['build'], run: { kind: 'just', recipe: 'gate' } },
      owned: { dir: 'q', inputs: { sources: ['q/**'], buildProcess: ['justfile', 'q/check.sh'] }, run: { kind: 'root-just', recipe: '_gate-q' } },
    } },
  },
}];

test('a project-owned inputs block is never narrowed: both its classes select under every class', function project_owned_inputs_never_narrowed() {
  for (const cls of [LEGACY_ALL, SOURCES]) {
    for (const file of ['justfile', 'q/check.sh', 'q/src/a.ts']) {
      const names = walkGraph(selectionManifests(), [file], { gateInputs: cls }).projects.map(p => p.name);
      assert.deepEqual(names, ['owned'], `${file} under ${cls.name}`);
    }
  }
});

test('the local gate path drops a borrowed step\'s own pipeline file only under sources', () => {
  assert.deepEqual(walkGraph(selectionManifests(), ['p/Jenkinsfile']).projects.map(p => p.name), ['borrowed']);
  assert.deepEqual(walkGraph(selectionManifests(), ['p/Jenkinsfile'], { gateInputs: SOURCES }).projects, []);
  assert.deepEqual(walkGraph(selectionManifests(), ['p/src/a.rs'], { gateInputs: SOURCES }).projects.map(p => p.name), ['borrowed']);
});

test('CI prediction is the same with the gate class in every position', function ci_prediction_unchanged_by_class() {
  for (const files of [['p/Jenkinsfile'], ['scripts/ci/image.sh'], ['p/src/a.rs', 'justfile']]) {
    const ci = JSON.stringify(walkGraph(selectionManifests(), files, { ci: true }));
    const local = JSON.stringify(walkGraph(selectionManifests(), files).pipelines);
    for (const cls of [LEGACY_ALL, SOURCES]) {
      assert.equal(JSON.stringify(walkGraph(selectionManifests(), files, { ci: true, gateInputs: cls })), ci);
      assert.equal(JSON.stringify(walkGraph(selectionManifests(), files, { gateInputs: cls }).pipelines), local);
    }
  }
});

test('the graph-walker CLI, which CI dispatch and preview read, prints the same bytes whatever GATE_INPUTS says', function graph_walker_cli_bytes_unchanged() {
  const cli = join(ROOT, 'genesis/orchestrator/graph-walker.mjs');
  const input = 'elohim/eprfs/Jenkinsfile\nelohim/holochain/Jenkinsfile\nscripts/ci/build-storage-image.sh\nelohim/eprfs/eprfs-core/src/lib.rs\n';
  const outputs = [undefined, 'off', 'shadow', 'on'].map(mode => {
    const env = { ...process.env };
    if (mode) env.GATE_INPUTS = mode; else delete env.GATE_INPUTS;
    const plain = spawnSync(process.execPath, [cli], { cwd: ROOT, input, encoding: 'utf8', env });
    const lines = spawnSync(process.execPath, [cli, '--shell-lines'], { cwd: ROOT, input, encoding: 'utf8', env });
    assert.equal(plain.status, 0, plain.stderr);
    assert.equal(lines.status, 0, lines.stderr);
    return `${plain.stdout}\n${lines.stdout}`;
  });
  for (const out of outputs) assert.equal(out, outputs[0]);
});

test('the oracle passes --inputs sources only for SOURCES, and upstream propagation is unchanged', function oracle_passes_sources_flag() {
  const seen = [];
  const affected = [
    { qualified_name: 'fixture:build', affected_by: [{ kind: 'changedFile', inputClass: 'sources', path: 'p/a.rs' }] },
    { qualified_name: 'fixture:image', affected_by: [{ kind: 'upstreamNode', inputClass: 'upstream', upstream: 'fixture:build' }] },
  ];
  const spawn = (bin, args) => { seen.push(args); return { status: 0, stdout: JSON.stringify({ changed_paths: ['p/a.rs'], affected }), stderr: '' }; };
  for (const cls of [null, LEGACY_ALL, SOURCES]) {
    const stale = britAffected('/repo', ['p/a.rs'], { britBin: '/bin/brit', spawn, inputs: cls });
    assert.deepEqual([...stale], [['fixture:build', ['source: p/a.rs']], ['fixture:image', ['upstream: fixture:build']]]);
  }
  assert.deepEqual(seen.map(args => args.slice(6)), [[], [], ['--inputs', 'sources']]);
});

test('a refusal of the flag is told apart from any other oracle failure', () => {
  const run = (cls, stderr) => {
    const seen = [];
    britAffected('/repo', ['x'], { britBin: '/bin/brit', inputs: cls, spawn: () => ({ status: 2, stdout: '', stderr }), onUnavailable: (r, info) => seen.push(info) });
    return seen[0].refusedFlag;
  };
  assert.equal(run(SOURCES, CLAP_REFUSAL), true);
  assert.equal(run(SOURCES, "error: invalid value 'sources' for '--inputs <INPUTS>'\n"), true);
  assert.equal(run(SOURCES, 'manifest error: bad step\n'), false);
  assert.equal(run(LEGACY_ALL, CLAP_REFUSAL), false, 'a call that passed no flag cannot have been refused it');
  assert.equal(refusesInputsFlag({ status: 0, stderr: CLAP_REFUSAL }), false);
});

test('fingerprints carry the class\'s arguments, none for LEGACY_ALL, and name a refusal', () => {
  const seen = [];
  const spawn = (bin, args) => {
    seen.push(args.slice(7));
    return args.includes('--inputs') && args.at(-1) === 'sources' && seen.length > 2
      ? { status: 2, stdout: '', stderr: CLAP_REFUSAL }
      : { status: 0, stdout: JSON.stringify({ fingerprints: [{ step: 's', fingerprint: 'bafy' }] }), stderr: '' };
  };
  const project = { steps: ['s'], manifestPath: 'm.json' };
  assert.deepEqual(stepFingerprints(project, 'c', { britBin: 'b', root: '/r', spawn }), { fingerprints: [{ step: 's', fingerprint: 'bafy' }] });
  assert.deepEqual(stepFingerprints(project, 'c', { britBin: 'b', root: '/r', spawn, inputs: SOURCES }), { fingerprints: [{ step: 's', fingerprint: 'bafy' }] });
  assert.equal(stepFingerprints(project, 'c', { britBin: 'b', root: '/r', spawn, inputs: SOURCES }).refusedFlag, true);
  assert.deepEqual(seen, [[], ['--inputs', 'sources'], ['--inputs', 'sources']]);
});

// ---------------------------------------------------------------- the policy

describe('the run\'s policy', () => {
  test('off probes nothing and prints nothing', t => {
    const { brit } = fakeBrit(t);
    const calls = [];
    const lines = [];
    const policy = resolveGateInputs({ env: { GATE_INPUTS: 'off' }, britBin: brit, spawn: helpSpawn(CAPABLE_HELP, calls), log: l => lines.push(l) });
    assert.equal(policy.cls, LEGACY_ALL);
    assert.equal(policy.shadow, null);
    assert.deepEqual(calls, []);
    assert.deepEqual(lines, []);
    assert.ok(Object.isFrozen(policy));
  });

  test('a capable brit: shadow keeps the legacy class and compares with sources, on included', t => {
    const { brit } = fakeBrit(t);
    for (const env of [{}, { GATE_INPUTS: 'on' }]) {
      clearGateInputsCache();
      const lines = [];
      const policy = resolveGateInputs({ env, britBin: brit, spawn: helpSpawn(CAPABLE_HELP), log: l => lines.push(l) });
      assert.equal(policy.mode, 'shadow');
      assert.equal(policy.cls, LEGACY_ALL, 'selection and keys stay legacy');
      assert.equal(policy.shadow, SOURCES);
      assert.deepEqual(lines, []);
    }
  });

  for (const [why, spawn, britBin] of [
    ['a brit without the flag', helpSpawn(LEGACY_HELP), true],
    ['a probe that cannot run', () => { throw new Error('EACCES'); }, true],
    ['a probe that times out', () => ({ status: null, signal: 'SIGTERM', error: { code: 'ETIMEDOUT' } }), true],
    ['no brit at all', helpSpawn(CAPABLE_HELP), false],
  ]) {
    test(`${why} is legacy with no shadow, said once`, function incapable_brit_is_legacy(t) {
      const { brit } = fakeBrit(t);
      const lines = [];
      const policy = resolveGateInputs({ env: {}, britBin: britBin ? brit : null, spawn, log: l => lines.push(l) });
      resolveGateInputs({ env: {}, britBin: britBin ? brit : null, spawn, log: l => lines.push(l) });
      assert.equal(policy.cls, LEGACY_ALL);
      assert.equal(policy.shadow, null);
      assert.deepEqual(lines, [UNAVAILABLE_LINE]);
    });
  }

  test('the probe runs once per brit executable, and asks both verbs', function probe_once_per_brit(t) {
    const { brit } = fakeBrit(t);
    const calls = [];
    for (let i = 0; i < 3; i += 1) resolveGateInputs({ env: {}, britBin: brit, spawn: helpSpawn(CAPABLE_HELP, calls) });
    assert.deepEqual(calls.map(c => c.split(' ').slice(1).join(' ')), ['build affected --help', 'build fingerprint --help']);
    // Both verbs must show the flag.
    const half = (bin, args) => ({ status: 0, stdout: args[1] === 'affected' ? CAPABLE_HELP : LEGACY_HELP, stderr: '' });
    assert.equal(probeBrit(brit, { spawn: half }).capable, false);
  });

  test('each probe is killed at its 2 s timeout and runs inside shadow\'s deadline; a probe the budget cut is not handed on', function probe_is_bounded(t) {
    const { brit } = fakeBrit(t);
    const seen = [];
    resolveGateInputs({ env: {}, britBin: brit, spawn: (bin, args, opts) => { seen.push([opts.timeout, opts.killSignal]); return { status: 0, stdout: CAPABLE_HELP, stderr: '' }; }, log: () => {} });
    assert.deepEqual(seen, [[2_000, 'SIGKILL'], [2_000, 'SIGKILL']]);
    // A process whose shadow budget is already spent probes nothing, says why, and writes no state.
    clearGateInputsCache({ totalMs: 0 });
    const state = join(scratch(t, 'gate-inputs-state-'), 'state.json');
    const calls = [];
    const lines = [];
    const policy = resolveGateInputs({ env: { [INPUTS_STATE_ENV]: state }, britBin: brit, spawn: helpSpawn(CAPABLE_HELP, calls), log: l => lines.push(l) });
    assert.deepEqual(calls, []);
    assert.equal(policy.shadow, null);
    assert.match(policy.why, /shadow budget was spent/);
    assert.equal(lines.at(-1), UNAVAILABLE_LINE);
    assert.throws(() => readFileSync(state), /ENOENT/);
  });

  test('the push\'s answer reaches a child through the state file, written whole, and is refused for another brit', function state_file_handoff(t) {
    const { brit } = fakeBrit(t);
    const dir = scratch(t, 'gate-inputs-state-');
    const state = join(dir, 'state.json');
    const env = { [INPUTS_STATE_ENV]: state };
    const parentCalls = [];
    resolveGateInputs({ env, britBin: brit, spawn: helpSpawn(CAPABLE_HELP, parentCalls), log: () => {} });
    assert.equal(parentCalls.length, 2);
    // Only what a child validates, and no temporary file left beside it.
    assert.deepEqual(Object.keys(JSON.parse(readFileSync(state, 'utf8'))).sort(), ['brit', 'capable', 'mode', 'v']);
    assert.deepEqual(readdirSync(dir), ['state.json']);
    clearGateInputsCache(); // a new gate process
    const childCalls = [];
    const childLines = [];
    const child = resolveGateInputs({ env, britBin: brit, spawn: helpSpawn(CAPABLE_HELP, childCalls), log: l => childLines.push(l) });
    assert.equal(child.shadow, SOURCES);
    assert.deepEqual(childCalls, [], 'a child pays no probe');
    assert.deepEqual(childLines, [], 'the line was said once, by the push');
    // Another mode is not this state's: off neither probes nor reads it.
    clearGateInputsCache();
    assert.equal(resolveGateInputs({ env: { ...env, GATE_INPUTS: 'off' }, britBin: brit, spawn: helpSpawn(CAPABLE_HELP), log: () => {} }).shadow, null);
    // The brit at that path changed: the state no longer describes it, so the child probes.
    writeFileSync(brit, '#!/bin/sh\n# another brit\nexit 1\n');
    utimesSync(brit, new Date(), new Date(Date.now() + 5000));
    clearGateInputsCache();
    const again = [];
    const reprobed = resolveGateInputs({ env, britBin: brit, spawn: helpSpawn(LEGACY_HELP, again), log: () => {} });
    assert.equal(again.length, 1);
    assert.equal(reprobed.shadow, null);
    // A file that is not a state is ignored, never trusted: the child probes for itself.
    writeFileSync(state, 'not json');
    clearGateInputsCache();
    const fresh = [];
    assert.equal(resolveGateInputs({ env, britBin: brit, spawn: helpSpawn(CAPABLE_HELP, fresh), log: () => {} }).shadow, SOURCES);
    assert.equal(fresh.length, 2);
    // An unwritable state path costs only a probe per child.
    clearGateInputsCache();
    const unwritable = { [INPUTS_STATE_ENV]: join(dir, 'missing-dir', 'state.json') };
    assert.equal(resolveGateInputs({ env: unwritable, britBin: brit, spawn: helpSpawn(CAPABLE_HELP), log: () => {} }).shadow, SOURCES);
  });
});

// ---------------------------------------------------------------- shadow's deadline and its end

test('shadow\'s budget is one elapsed deadline: each subprocess gets min(remaining, cap), and none starts after it', function shadow_budget_is_elapsed(t) {
  clearGateInputsCache({ totalMs: 10_000, callMs: 5_000 });
  let clock = 0;
  const now = () => clock;
  const timeouts = [];
  const slow = (bin, args, opts) => { timeouts.push([opts.timeout, opts.killSignal]); clock += 4_000; return { status: 0, stdout: '', stderr: '' }; };
  const lines = [];
  // One callback whose steps alone sum past the budget: 4 s each, a 10 s deadline.
  inShadowBudget(l => lines.push(l), bound => {
    const run = bound(slow);
    for (let i = 0; i < 5; i += 1) run('brit', ['build', 'fingerprint'], { timeout: 60_000 });
  }, { now });
  assert.deepEqual(timeouts, [[5_000, 'SIGKILL'], [5_000, 'SIGKILL'], [2_000, 'SIGKILL']], 'the third gets what remains, and the fourth never starts');
  assert.equal(clock, 12_000);
  // A later gate in the same process starts nothing, and the stop is said once.
  let ran = false;
  inShadowBudget(l => lines.push(l), () => { ran = true; }, { now });
  assert.equal(ran, false);
  assert.deepEqual(lines, [`[gate] inputs (shadow): stopped: the shadow's ${SHADOW_BUDGET.totalMs / 1000} s budget for this run is spent; no further inputs lines`]);
});

test('a refusal of the flag in shadow ends its printing with one line, and nothing else', function shadow_refusal_ends_printing() {
  const lines = [];
  endShadow('brit build affected exited 2', l => lines.push(l));
  endShadow('again', l => lines.push(l));
  let ran = false;
  assert.equal(inShadowBudget(l => lines.push(l), () => { ran = true; }), null);
  assert.equal(ran, false);
  assert.deepEqual(lines, ['[gate] inputs (shadow): brit refused --inputs (brit build affected exited 2); no further inputs lines this run']);
});

// ---------------------------------------------------------------- selection through the policy

function selectionRoot(t) {
  const root = scratch(t, 'gate-inputs-select-');
  mkdirSync(join(root, 'p'), { recursive: true });
  writeFileSync(join(root, 'p/build-manifest.json'), JSON.stringify(selectionManifests()[0].content));
  return root;
}

/** A brit double: `--help` as given; `build affected` from the legacy or the classified planner. */
function plannerSpawn(help, { refuseAffected = false } = {}, calls = []) {
  return (bin, args) => {
    calls.push(args.join(' '));
    if (args[2] === '--help') return { status: 0, stdout: help, stderr: '' };
    if (args[0] === 'build' && args[1] === 'affected') {
      const sources = args.includes('--inputs');
      if (sources && refuseAffected) return { status: 2, stdout: '', stderr: CLAP_REFUSAL };
      const files = args[args.indexOf('--files') + 1].split(',');
      const manifests = selectionManifests();
      const stale = staleSteps(manifests, files, sources ? SOURCES : LEGACY_ALL);
      const affected = [...stale.keys()].map(q => ({ qualified_name: q, affected_by: [{ kind: 'changedFile', path: files[0] }] }));
      return { status: 0, stdout: JSON.stringify({ changed_paths: files, affected }), stderr: '' };
    }
    return { status: 1, stdout: '', stderr: 'unexpected' };
  };
}

test('selection in shadow decides as off, asks the oracle for sources only for its line, and says what sources would drop', async function selection_follows_the_policy(t) {
  const { projectsForChanges } = await import('./gate-runner.mjs');
  const root = selectionRoot(t);
  const { brit } = fakeBrit(t);
  const run = (mode, options = {}) => {
    clearGateInputsCache();
    const calls = [];
    const inputsLines = [];
    const names = projectsForChanges(root, ['p/Jenkinsfile'], {
      env: { BRIT_BIN: brit, PATH, ...(mode ? { GATE_INPUTS: mode } : {}) }, spawn: plannerSpawn(CAPABLE_HELP, options, calls),
      log: () => {}, inputsLog: l => inputsLines.push(l),
    }).map(p => p.name);
    return { names, calls: calls.filter(c => c.startsWith('build affected --repo')), inputsLines };
  };
  const off = run('off');
  const shadow = run(undefined);
  const on = run('on');
  const refused = run(undefined, { refuseAffected: true });
  assert.deepEqual(off.names, ['borrowed']);
  for (const other of [shadow, on, refused]) assert.deepEqual(other.names, off.names, 'every mode decides as off');
  assert.deepEqual(off.inputsLines, []);
  const line = '[gate] inputs (shadow): all selects and sources would not: fixture:build; gates sources would not run: borrowed';
  assert.deepEqual(shadow.inputsLines, [line]);
  assert.deepEqual(on.inputsLines, [line], 'on reads as shadow');
  assert.deepEqual(refused.inputsLines, [`[gate] inputs (shadow): brit refused --inputs (brit build affected exited 2: error: unexpected argument '--inputs' found); no further inputs lines this run`]);
  assert.deepEqual(off.calls, ['build affected --repo ' + root + ' --files p/Jenkinsfile']);
  assert.deepEqual(shadow.calls, [...off.calls, `${off.calls[0]} --inputs sources`], 'the decision\'s call is today\'s; the shadow asks once more');
});

// ---------------------------------------------------------------- runProject: shadow decides as off

const SEED = '9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60';
const PUBLIC_KEY = 'd75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a';
const OUTPUT_CID = [0x12, 0x20, ...Array(32).fill(7)];
const DATE = { GIT_AUTHOR_DATE: '2026-11-21T00:00:00Z', GIT_COMMITTER_DATE: '2026-11-21T00:00:00Z' };

function git(repo, args) {
  const r = spawnSync(GIT, args, {
    cwd: repo, encoding: 'utf8',
    env: { ...process.env, ...DATE, GIT_AUTHOR_NAME: 'f', GIT_AUTHOR_EMAIL: 'f@x', GIT_COMMITTER_NAME: 'f', GIT_COMMITTER_EMAIL: 'f@x' },
  });
  assert.equal(r.status, 0, `git ${args.join(' ')}: ${r.stderr}`);
  return r.stdout.trim();
}
function commitFiles(repo, files, message) {
  for (const [path, text] of Object.entries(files)) {
    mkdirSync(dirname(join(repo, path)), { recursive: true });
    writeFileSync(join(repo, path), text);
  }
  git(repo, ['add', '-A']);
  git(repo, ['commit', '-q', '--no-verify', '-m', message]);
  return git(repo, ['rev-parse', 'HEAD']);
}

// Two borrowed steps: `s` lists p/Jenkinsfile as its own buildProcess, `t` lists p/t.Jenkinsfile.
const gateProject = {
  name: 'fixture-gate', dir: 'p', pipeline: 'fixture', steps: ['s', 't'], manifestPath: 'p/build-manifest.json',
  run: { kind: 'just', recipe: 'gate' },
};
const manifestText = extra => JSON.stringify({
  pipeline: 'fixture',
  steps: {
    s: { inputs: { sources: ['p/**'], buildProcess: ['p/Jenkinsfile'] }, depends: [], ...extra },
    t: { inputs: { sources: ['p/t/**'], buildProcess: ['p/t.Jenkinsfile'] }, depends: [] },
  },
  gate: { projects: { 'fixture-gate': { dir: 'p', steps: ['s', 't'], run: { kind: 'just', recipe: 'gate' } } } },
});
const JENKINSFILE_CHANGE = { 'p/Jenkinsfile': 'pipeline two\n' };

/**
 * dev at a base commit, HEAD on a feature branch one commit later: by default an empty commit (a
 * rebase's shape: a new commit, the same tree), else one that writes `headFiles`. A non-cargo gate
 * keys on the whole commit tree, so only an equal tree can carry a green across commits.
 */
function gateRepo(t, { reuse = true, headFiles = null } = {}) {
  const repo = scratch(t, 'gate-inputs-repo-');
  git(repo, ['init', '-q', '-b', 'dev']);
  const base = commitFiles(repo, {
    '.gitignore': '.claude/\n',
    justfile: 'gate:\n    echo gate\n',
    'p/build-manifest.json': manifestText(),
    'p/x.txt': 'x\n',
    'p/Jenkinsfile': 'pipeline\n',
    'outside.txt': 'one\n',
    'genesis/orchestrator/run-local-gate.sh': '#!/bin/sh\n',
    'genesis/agentic/pool-policy.json': JSON.stringify({ gate_reuse: { projects: reuse ? ['fixture-gate'] : [] } }),
  }, 'base');
  mkdirSync(join(repo, '.git/brit'), { recursive: true });
  writeFileSync(join(repo, '.git/brit/agent-key'), Buffer.from(SEED, 'hex'));
  git(repo, ['switch', '-q', '-c', 'feature']);
  let head;
  if (headFiles) head = commitFiles(repo, headFiles, 'head');
  else {
    git(repo, ['commit', '-q', '--no-verify', '--allow-empty', '-m', 'head']);
    head = git(repo, ['rev-parse', 'HEAD']);
  }
  return { repo, base, head };
}

/**
 * A world for runProject: brit answers `--help` as `help` and fingerprints by class (flag omitted,
 * `--inputs sources`, `--inputs all` each give a different CID, so a test can tell which one keyed);
 * brit-build-ref stores rows per inputs hash in `rows`; the clock advances one second per subprocess.
 */
function world(t, { help = CAPABLE_HELP, refuseFingerprint = false, rows = new Map(), siblings = [], fingerprintAt = () => 'v1' } = {}) {
  const { brit, buildRef } = fakeBrit(t);
  let now = 0n;
  const recorded = [];
  const unexpected = [];
  const calls = [];
  const spawn = (bin, args, opts) => {
    now += 1_000_000_000n;
    calls.push([bin, ...args].join(' '));
    if (bin === 'git') return spawnSync(GIT, args, opts);
    if (bin === brit && args[2] === '--help') return { status: 0, stdout: help, stderr: '' };
    if (bin === brit && args[0] === 'build' && args[1] === 'fingerprint') {
      const flag = args.includes('--inputs') ? args[args.indexOf('--inputs') + 1] : null;
      if (flag && refuseFingerprint) return { status: 2, stdout: '', stderr: CLAP_REFUSAL };
      const commit = args[args.indexOf('--commit') + 1];
      const step = args[args.indexOf('--step') + 1];
      const fingerprint = `bafy-${step}-${flag || 'legacy'}-${fingerprintAt(flag, commit)}`;
      return { status: 0, stdout: JSON.stringify({ fingerprints: [{ step, fingerprint }] }), stderr: '' };
    }
    if (bin === brit && args[0] === 'build' && args[1] === 'affected') {
      // The planner over the fixture's one manifest, legacy or classified as asked.
      const files = args[args.indexOf('--files') + 1].split(',');
      const manifests = [{ path: './p/build-manifest.json', content: JSON.parse(manifestText()) }];
      const stale = staleSteps(manifests, files, args.includes('--inputs') ? SOURCES : LEGACY_ALL);
      const affected = [...stale.keys()].map(q => ({ qualified_name: q, affected_by: [{ kind: 'changedFile', path: files[0] }] }));
      return { status: 0, stdout: JSON.stringify({ changed_paths: files, affected }), stderr: '' };
    }
    if (bin === buildRef) {
      const key = args[args.indexOf('--inputs-hash') + 1];
      if (args.includes('find')) { recorded.push(['find', key]); return { status: 0, stdout: JSON.stringify(rows.get(key) || []), stderr: '' }; }
      if (args.includes('put')) {
        recorded.push(['put', key, args[args.indexOf('--output-json') + 1], args[args.indexOf('--duration-ms') + 1]]);
        return { status: 0, stdout: 'bafyrecorded\n', stderr: '' };
      }
      if (args.includes('signer')) return { status: 0, stdout: `{"agentId":"${PUBLIC_KEY}"}\n`, stderr: '' };
      if (args.includes('get')) return { status: 0, stdout: '', stderr: 'no build ref found\n' };
    }
    if (bin === 'rustc') return { status: 1, stdout: '', stderr: '' };
    if (bin === 'bash' && args[0].endsWith('run-local-gate.sh')) { recorded.push(['ran']); return { status: 0, stdout: '', stderr: '' }; }
    if (bin === 'epr' && args[0] === 'peer' && args[1] === 'verdicts') {
      const closure = args[args.indexOf('--closure') + 1];
      return { status: 0, stdout: JSON.stringify(siblings.filter(s => s.closure === closure)), stderr: '' };
    }
    if (bin === 'epr' && args[0] === 'peer' && args[1] === 'leases') return { status: 0, stdout: '{"standing":"none"}', stderr: '' };
    if (bin === 'epr' && args[0] === 'flow' && args[1] === 'note') { recorded.push(['note', ...args]); return { status: 0, stdout: '', stderr: '' }; }
    unexpected.push([bin, ...args].join(' '));
    return { status: 127, stdout: '', stderr: 'unexpected command' };
  };
  return { brit, buildRef, spawn, recorded, unexpected, calls, clock: () => now };
}

async function runGate(t, w, repo, mode, extraEnv = {}) {
  const { runProject } = await import('./gate-runner.mjs');
  clearGateInputsCache();
  const lines = [];
  const status = runProject(gateProject, false, false, {
    root: repo, spawn: w.spawn, tap: false, write: text => lines.push(text), warn: () => {}, clock: w.clock,
    env: {
      PATH, BRIT_BIN: w.brit, BRIT_BUILD_REF_BIN: w.buildRef, GATE_TASK: 'off', GATE_EVIDENCE: 'reuse',
      GATE_RUN_KIND: 'full-gate', GATE_RECIPE_KEY: 'off', ...(mode ? { GATE_INPUTS: mode } : {}), ...extraEnv,
    },
  });
  const text = lines.join('');
  return { status, text, lines: text.split('\n').filter(Boolean) };
}

const green = (commit, extra = {}) => ({ commit, attestation: { success: true, agentId: PUBLIC_KEY, builtAt: '2026-11-21T00:00:00Z', outputCid: OUTPUT_CID, ...extra } });

for (const reuse of [false, true]) {
  test(`GATE_INPUTS=shadow and off decide, record and time identically (${reuse ? 'a reused green' : 'a run that records'})`, async function shadow_equals_off(t) {
    const { repo, base, head } = gateRepo(t, { reuse, headFiles: JENKINSFILE_CHANGE });
    // The step's legacy fingerprint moved since the base and its sources one did not.
    const fingerprintAt = (flag, commit) => (flag ? 'same' : commit === base ? 'old' : 'new');
    const rows = new Map();
    const outcomes = {};
    for (const mode of ['off', 'shadow']) {
      const w = world(t, { rows, fingerprintAt });
      if (reuse) {
        // The legacy key this world computes at HEAD holds this workspace's green from the base.
        const probe = world(t, { fingerprintAt });
        await runGate(t, probe, repo, 'off');
        // At HEAD: a green from the base would be refused, rightly, since p/Jenkinsfile selects this gate.
        rows.set(probe.recorded.find(([kind]) => kind === 'find')[1], [green(head)]);
      }
      const run = await runGate(t, w, repo, mode);
      outcomes[mode] = {
        status: run.status,
        unexpected: w.unexpected,
        records: w.recorded.filter(([kind]) => kind !== 'find'),
        decisionFinds: w.recorded.filter(([kind]) => kind === 'find').slice(0, 1),
        lines: run.lines.filter(l => !l.includes('inputs (shadow)')).map(l => l.replace(/\d+[smhd] ago/, 'AGE')),
        shadow: run.lines.filter(l => l.includes('inputs (shadow)')),
      };
    }
    const { off, shadow } = outcomes;
    assert.deepEqual(shadow.unexpected, []);
    assert.deepEqual(off.unexpected, []);
    assert.equal(shadow.status, off.status);
    assert.deepEqual(shadow.decisionFinds, off.decisionFinds, 'the decision looked up the same key');
    // Every record: the verdict JSON, --duration-ms, the run itself and the cycle note.
    assert.deepEqual(shadow.records, off.records);
    assert.deepEqual(shadow.lines, off.lines);
    assert.deepEqual(off.shadow, []);
    assert.deepEqual(shadow.shadow, [`[gate] fixture-gate inputs (shadow): since ${base.slice(0, 9)} the step fingerprints moved under all (s) and are unchanged under sources`]);
    if (reuse) {
      assert.match(off.lines.join('\n'), /fixture-gate reused: green/);
    } else {
      const put = off.records.find(([kind]) => kind === 'put');
      assert.ok(Number(put[3]) > 0, 'the recorded duration is the clock\'s, nonzero, and equal in both modes');
      assert.ok(head);
    }
  });
}

test('a fingerprint call that refuses the flag in shadow ends the shadow and changes nothing', async function shadow_fingerprint_refusal(t) {
  const { repo } = gateRepo(t, { reuse: false, headFiles: JENKINSFILE_CHANGE });
  const off = world(t);
  const refused = world(t, { refuseFingerprint: true });
  const a = await runGate(t, off, repo, 'off');
  const b = await runGate(t, refused, repo, undefined);
  assert.equal(b.status, a.status);
  assert.deepEqual(refused.recorded.filter(([kind]) => kind !== 'find'), off.recorded.filter(([kind]) => kind !== 'find'));
  assert.deepEqual(b.lines.filter(l => l.includes('inputs (shadow)')), [`[gate] inputs (shadow): brit refused --inputs (fingerprint of s failed: Usage: brit build fingerprint [OPTIONS] <MANIFEST>); no further inputs lines this run`]);
});

describe('the shadow is free unless a step\'s own buildProcess file changed', () => {
  // Shadow's brit spawns: every brit call but the probe's `--help`, beyond the ones the off run makes.
  const britCalls = w => w.calls.filter(c => c.startsWith(`${w.brit} `) && !c.endsWith('--help')).map(c => c.slice(w.brit.length + 1));
  const compare = async (t, repo) => {
    const off = world(t);
    await runGate(t, off, repo, 'off');
    const shadow = world(t);
    const run = await runGate(t, shadow, repo, undefined);
    const extra = britCalls(shadow).filter(c => !britCalls(off).includes(c));
    return { extra, lines: run.lines.filter(l => l.includes('inputs (shadow)')), offCalls: britCalls(off), shadowCalls: britCalls(shadow) };
  };

  test('no brit spawn, no line, when the base is the gated commit', async function shadow_free_on_unchanged_head(t) {
    const { repo } = gateRepo(t, { headFiles: JENKINSFILE_CHANGE });
    git(repo, ['update-ref', 'refs/heads/dev', 'HEAD']);
    const { extra, lines, offCalls, shadowCalls } = await compare(t, repo);
    assert.deepEqual(extra, []);
    assert.deepEqual(shadowCalls, offCalls);
    assert.deepEqual(lines, []);
  });

  test('no brit spawn, no line, when the range changes no borrowed step\'s buildProcess file', async function shadow_free_without_process_change(t) {
    const { repo } = gateRepo(t, { headFiles: { 'p/x.txt': 'x two\n', 'outside.txt': 'two\n' } });
    const { extra, lines, offCalls, shadowCalls } = await compare(t, repo);
    assert.deepEqual(extra, []);
    assert.deepEqual(shadowCalls, offCalls);
    assert.deepEqual(lines, []);
  });

  test('when one is changed, only the touched step is fingerprinted, three times', async function shadow_fingerprints_only_touched_steps(t) {
    const { repo, base } = gateRepo(t, { headFiles: JENKINSFILE_CHANGE });
    const off = world(t, { fingerprintAt: (flag, commit) => (flag ? 'same' : commit === base ? 'old' : 'new') });
    await runGate(t, off, repo, 'off');
    const shadow = world(t, { fingerprintAt: (flag, commit) => (flag ? 'same' : commit === base ? 'old' : 'new') });
    const run = await runGate(t, shadow, repo, undefined);
    const extra = britCalls(shadow).slice(britCalls(off).length);
    assert.deepEqual(britCalls(shadow).slice(0, britCalls(off).length), britCalls(off), 'the evidence\'s own calls are today\'s');
    assert.equal(extra.length, 3);
    assert.ok(extra.every(c => c.includes('--step s ')), extra.join('\n'));
    assert.deepEqual(run.lines.filter(l => l.includes('inputs (shadow)')),
      [`[gate] fixture-gate inputs (shadow): since ${base.slice(0, 9)} the step fingerprints moved under all (s) and are unchanged under sources`]);
  });

  test('selection asks the second planner call only for a change to some step\'s buildProcess file', async function selection_free_without_process_change(t) {
    const { projectsForChanges } = await import('./gate-runner.mjs');
    const root = selectionRoot(t);
    const { brit } = fakeBrit(t);
    const ask = files => {
      clearGateInputsCache();
      const calls = [];
      const lines = [];
      projectsForChanges(root, files, { env: { BRIT_BIN: brit, PATH }, spawn: plannerSpawn(CAPABLE_HELP, {}, calls), log: () => {}, inputsLog: l => lines.push(l) });
      return { affected: calls.filter(c => c.startsWith('build affected --repo')), lines };
    };
    const source = ask(['p/src/a.rs']);
    assert.equal(source.affected.length, 1, 'only the decision\'s own call');
    assert.ok(!source.affected[0].includes('--inputs'));
    assert.deepEqual(source.lines, []);
    // A step a gate does not borrow counts too: its upstream reason could reach a borrowed dependent.
    const image = ask(['scripts/ci/image.sh']);
    assert.equal(image.affected.length, 2);
    const pipeline = ask(['p/Jenkinsfile']);
    assert.deepEqual(pipeline.lines, ['[gate] inputs (shadow): all selects and sources would not: fixture:build; gates sources would not run: borrowed']);
  });
});
