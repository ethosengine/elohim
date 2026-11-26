import { describe, test } from 'node:test';
import { strict as assert } from 'node:assert';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, utimesSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

import { runEnrichSample } from './enrich-sample.mjs';
import { gateEvidence } from './gate-evidence.mjs';
import { loadGateRegistry } from './pipeline-registry.mjs';

const COMMIT = '0123456789abcdef0123456789abcdef01234567';
const VERDICT = `bafyrei${'a'.repeat(52)}`;
const JUST_OID = '1'.repeat(40);
const RUNNER_OID = '2'.repeat(40);

function fixture(t, {
  gateStatus = 0,
  enrichStatus = 0,
  fromDevice = null,
  actualClosure = 'closure-a',
  actualRecordable = true,
  gateReports = true,
  dirty = '',
  pnpmStatus = 0,
  gateTimeout = false,
  gateThrows = false,
} = {}) {
  const root = mkdtempSync(resolve(tmpdir(), 'enrich-sample-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const calls = [];
  let now = Date.now();
  const spawn = (command, args, options = {}) => {
    const call = { command, args, options };
    calls.push(call);
    if (command === 'pgrep') return { status: 1 };
    if (command === 'just') {
      if (gateThrows) throw new TypeError('gate exploded\nwith a second line');
      now += 4_500;
      if (gateTimeout) return { status: null, error: { code: 'ETIMEDOUT', message: 'timed out' } };
      if (gateReports) {
        writeFileSync(options.env.GATE_SAMPLE_RESULT, JSON.stringify({
          v: 1,
          project: 'orchestrator',
          commit: COMMIT,
          closure: actualClosure,
          executed: true,
          status: gateStatus,
          signal: null,
          recordable: actualRecordable,
          reason: actualRecordable ? null : 'working tree was dirty or unreadable before execution',
        }));
      }
      return { status: gateStatus };
    }
    if (command === 'fixture-epr' && args.includes('--resolve-only')) {
      return { status: 0, stdout: JSON.stringify({
        verdict: VERDICT,
        project: 'orchestrator',
        closure: 'closure-a',
        commit: COMMIT,
        device: 'did:key:fixture',
      }) };
    }
    if (command === 'fixture-epr') return { status: enrichStatus };
    if (command === 'pnpm') return { status: pnpmStatus };
    if (command === 'git' && args.includes('rev-parse')) {
      return { status: 0, stdout: `${args.at(-1).endsWith(':justfile') ? JUST_OID : RUNNER_OID}\n` };
    }
    if (command === 'git' && args.includes('status')) return { status: 0, stdout: dirty };
    if (command === 'git') return { status: 0 };
    throw new Error(`unexpected command ${command}`);
  };
  return {
    root,
    calls,
    run: (extraEnv = {}) => runEnrichSample({
      project: 'orchestrator', commit: COMMIT, closure: 'closure-a',
      ...(fromDevice ? { from_device: fromDevice } : {}), root,
    }, {
      env: { EPR_BIN: 'fixture-epr', ...extraEnv }, spawn, clock: () => now, sleep: async ms => { now += ms; },
    }),
  };
}

function resolveCall(calls) {
  return calls.find(call => call.command === 'fixture-epr' && call.args.includes('--resolve-only'));
}

function enrichCall(calls) {
  return calls.find(call => call.command === 'fixture-epr' && !call.args.includes('--resolve-only'));
}

describe('detached enrichment sample', () => {
  for (const [gateStatus, outcome] of [[0, 'green'], [3, 'red']]) {
    test(`gate exit ${gateStatus} records ${outcome} only after naming the verdict and matching closure`, async t => {
      const sample = fixture(t, { gateStatus });
      const result = await sample.run();
      assert.equal(result.outcome, outcome);
      assert.equal(result.seconds, '4.5');
      const resolved = resolveCall(sample.calls);
      const gateIndex = sample.calls.findIndex(call => call.command === 'just');
      assert.ok(sample.calls.indexOf(resolved) < gateIndex, 'the immutable verdict is named before execution');
      const call = enrichCall(sample.calls);
      assert.ok(call, 'the native enrichment writer is called');
      assert.deepEqual(call.args.slice(0, 5), ['flow', 'enrich', '--verdict', VERDICT, '--kind']);
      assert.equal(call.args[call.args.indexOf('--outcome') + 1], outcome);
      assert.equal(call.args[call.args.indexOf('--method') + 1],
        `justfile:${JUST_OID},gate-runner:${RUNNER_OID}`);
      assert.equal(call.args[call.args.indexOf('--seconds') + 1], '4.5');
      assert.ok(!call.args.includes('--reason'));
      const just = sample.calls[gateIndex];
      assert.equal(just.options.env.GATE_RECORD, 'off');
      assert.equal(just.options.env.GATE_EVIDENCE, 'off');
      assert.equal(just.options.env.GATE_TASK, 'off');
      assert.equal(just.options.env.GATE_READING, 'off');
    });
  }

  test('a sibling source is used only to resolve the immutable verdict', async t => {
    const sibling = 'did:key:sibling';
    const sample = fixture(t, { fromDevice: sibling });
    await sample.run();
    const resolved = resolveCall(sample.calls);
    assert.equal(resolved.args[resolved.args.indexOf('--from-device') + 1], sibling);
    assert.ok(!enrichCall(sample.calls).args.includes('--from-device'));
  });

  for (const [name, options, reason] of [
    ['closure drift', { actualClosure: 'closure-b' }, 'closure differs: verdict closure-a, execution closure-b'],
    ['dirty checkout', { dirty: ' M tracked-file\n' }, 'dirty tree after execution'],
    ['unrecordable execution', { actualRecordable: false }, 'working tree was dirty or unreadable before execution'],
    ['setup failure', { pnpmStatus: 7 }, 'pnpm install failed (7)'],
  ]) {
    test(`${name} records not-comparable with its one-line reason`, async t => {
      const sample = fixture(t, options);
      const result = await sample.run();
      assert.equal(result.outcome, 'not-comparable');
      assert.equal(result.reason, reason);
      const call = enrichCall(sample.calls);
      assert.equal(call.args[call.args.indexOf('--outcome') + 1], 'not-comparable');
      assert.equal(call.args[call.args.indexOf('--reason') + 1], reason);
    });
  }

  test('an unexpected exception is recorded with its kind and one-line message', async t => {
    const sample = fixture(t, { gateThrows: true });
    const result = await sample.run();
    assert.equal(result.outcome, 'not-comparable');
    assert.equal(result.reason, 'unexpected TypeError: gate exploded with a second line');
    const call = enrichCall(sample.calls);
    assert.equal(call.args[call.args.indexOf('--outcome') + 1], 'not-comparable');
    assert.equal(call.args[call.args.indexOf('--reason') + 1], result.reason);
  });

  test('a missing gate result is recorded as an unexpected error, not an ordinary refusal', async t => {
    const sample = fixture(t, { gateReports: false });
    const result = await sample.run();
    assert.equal(result.outcome, 'not-comparable');
    assert.match(result.reason,
      /^unexpected Error: gate did not report a completed execution \(Error: ENOENT:/);
    const call = enrichCall(sample.calls);
    assert.equal(call.args[call.args.indexOf('--reason') + 1], result.reason);
  });

  test('a sampled reused green is comparable through the real gate runner and stub recipe', async t => {
    const root = mkdtempSync(resolve(tmpdir(), 'enrich-sample-e2e-'));
    t.after(() => rmSync(root, { recursive: true, force: true }));
    const put = (path, text, options = {}) => {
      const absolute = resolve(root, path);
      mkdirSync(resolve(absolute, '..'), { recursive: true });
      writeFileSync(absolute, text, options);
    };
    const git = (...args) => {
      const result = spawnSync('git', [
        '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.test', ...args,
      ], { cwd: root, encoding: 'utf8' });
      assert.equal(result.status, 0, result.stderr);
      return result.stdout.trim();
    };
    const gateRunner = resolve(import.meta.dirname, 'gate-runner.mjs');
    put('justfile', `gate target:
    @GATE_ROOT="{{justfile_directory()}}" node ${JSON.stringify(gateRunner)} --target "{{target}}"

_fixture-gate:
    @true
`);
    put('build-manifest.json', `${JSON.stringify({
      manifestVersion: '1.0',
      pipeline: 'fixture-pipeline',
      steps: { build: { inputs: { sources: ['fixture.txt'], buildProcess: [] } } },
      gate: { projects: { fixture: {
        dir: '.', steps: ['build'],
        run: { kind: 'root-just', recipe: '_fixture-gate' },
      } } },
    }, null, 2)}\n`);
    put('fixture.txt', 'fixture input\n');
    put('genesis/orchestrator/gate-runner.mjs', readFileSync(gateRunner, 'utf8'));
    put('genesis/orchestrator/run-local-gate.sh',
      readFileSync(resolve(import.meta.dirname, 'run-local-gate.sh'), 'utf8'));
    put('bin/brit', `#!/bin/sh
printf '%s\n' '{"fingerprints":[{"step":"build","fingerprint":"fixture-fingerprint"}]}'
`, { mode: 0o755 });
    put('bin/brit-build-ref', '#!/bin/sh\nprintf \'%s\\n\' \'[]\'\n', { mode: 0o755 });
    git('init', '-q');
    git('add', '.');
    git('commit', '-qm', 'fixture');
    const commit = git('rev-parse', 'HEAD');
    const gateEnv = {
      ...process.env,
      BRIT_BIN: resolve(root, 'bin/brit'),
      BRIT_BUILD_REF_BIN: resolve(root, 'bin/brit-build-ref'),
      GATE_RECORD: 'off',
      GATE_EVIDENCE: 'off',
      GATE_TASK: 'off',
      GATE_READING: 'off',
      GATE_PREFLIGHT: 'off',
      GATE_READ_SHADOW: 'off',
      GATE_GRADE: 'off',
      GATE_INPUTS: 'off',
      GATE_RECIPE_KEY: 'off',
      GATE_ORACLE: 'path',
      GATE_TAP: 'off',
    };
    const project = loadGateRegistry(root).get('fixture');
    const evidence = gateEvidence(project, {
      env: gateEnv, root, commit, cargoEnv: {}, spawn: spawnSync, warn: () => {},
    });
    assert.ok(evidence.key, evidence.line);
    assert.equal(evidence.dirty, false, evidence.line);

    mkdirSync(resolve(root, '.worktrees'), { recursive: true });
    const reusedGreen = { verdict: VERDICT, outcome: 'green', closure: evidence.key };
    const eprCalls = [];
    const spawn = (command, args, options = {}) => {
      if (command === 'fixture-epr') {
        eprCalls.push(args);
        if (args.includes('--resolve-only')) {
          return { status: 0, stdout: JSON.stringify({
            verdict: reusedGreen.verdict,
            project: 'fixture',
            closure: reusedGreen.closure,
            commit,
            device: 'did:key:fixture',
          }) };
        }
        return { status: 0 };
      }
      if (command === 'pgrep') return { status: 1 };
      if (command === 'pnpm') return { status: 0 };
      if (command === 'git' && args.includes('submodule')) return { status: 0 };
      return spawnSync(command, args, options);
    };
    const sampled = await runEnrichSample({
      project: 'fixture', commit, closure: reusedGreen.closure, root,
    }, { env: { ...gateEnv, EPR_BIN: 'fixture-epr' }, spawn });

    assert.equal(sampled.outcome, reusedGreen.outcome,
      `${sampled.reason}\n${readFileSync(sampled.logPath, 'utf8')}`);
    assert.equal(sampled.reason, null);
    const sampledResult = resolve(sampled.logPath, '../gate-result.json');
    assert.equal(existsSync(sampledResult), true, 'the sampled gate wrote gate-result.json');
    const executed = JSON.parse(readFileSync(sampledResult, 'utf8'));
    assert.equal(executed.closure, reusedGreen.closure);
    assert.equal(executed.status, 0);
    assert.equal(executed.recordable, true);
    const enrichment = eprCalls.find(args => !args.includes('--resolve-only'));
    assert.ok(enrichment, 'the comparable enrichment is recorded');
    assert.equal(enrichment[enrichment.indexOf('--verdict') + 1], reusedGreen.verdict);
    assert.equal(enrichment[enrichment.indexOf('--outcome') + 1], reusedGreen.outcome);
    assert.ok(!enrichment.includes('--reason'));
  });

  test('one deadline kills the gate and records not-comparable(timeout)', async t => {
    const sample = fixture(t, { gateTimeout: true });
    const result = await sample.run({ GATE_ENRICH_SAMPLE_TIMEOUT_SECONDS: '30' });
    assert.equal(result.outcome, 'not-comparable');
    assert.equal(result.reason, 'timeout');
    const gate = sample.calls.find(call => call.command === 'just');
    assert.equal(gate.options.killSignal, 'SIGKILL');
    assert.ok(gate.options.timeout <= 27_000, 'the final slice remains for record and cleanup');
    const call = enrichCall(sample.calls);
    assert.equal(call.args[call.args.indexOf('--reason') + 1], 'timeout');
  });

  test('stale sampler worktree and scratch are swept by prefix before execution', async t => {
    const sample = fixture(t);
    const staleWorktree = join(sample.root, '.worktrees', 'enrich-sample-stale-worktree');
    const staleScratch = join(sample.root, '.eprfs', 'status', 'enrich-sample', 'enrich-sample-stale-scratch');
    mkdirSync(staleWorktree, { recursive: true });
    mkdirSync(staleScratch, { recursive: true });
    const old = new Date(Date.now() - 2 * 3600 * 1000);
    utimesSync(staleWorktree, old, old);
    utimesSync(staleScratch, old, old);
    await sample.run();
    const sweep = sample.calls.find(call => call.command === 'git'
      && call.args.includes('worktree') && call.args.at(-1) === staleWorktree);
    assert.ok(sweep, 'the stale registered worktree is removed through git first');
    assert.equal(existsSync(staleWorktree), false);
    assert.equal(existsSync(staleScratch), false);
  });

  test('a refused final enrichment does not affect cleanup or the sampled outcome', async t => {
    const sample = fixture(t, { enrichStatus: 3 });
    const result = await sample.run();
    assert.equal(result.outcome, 'green', 'the sampled gate still ran');
    assert.equal(result.enriched, false);
    assert.match(readFileSync(result.logPath, 'utf8'), /enrichment refused \(3\)/);
    assert.ok(sample.calls.some(call => call.command === 'git'
      && call.args.includes('worktree') && call.args.includes('remove')));
  });

  test('the sample worktree is removed even when the gate is red', async t => {
    const sample = fixture(t, { gateStatus: 3 });
    await sample.run();
    const remove = sample.calls.find(call => call.command === 'git'
      && call.args.includes('worktree') && call.args.includes('remove'));
    assert.ok(remove);
    assert.ok(remove.args.includes('--force'));
    assert.match(remove.args.at(-1), /\.worktrees\/enrich-sample-0123456789ab-orchestrator-\d+$/);
  });
});

test('GATE_RECORD=off skips the eprfs recipe ingest and crossing producer put', () => {
  // A sampled re-execution runs `just gate eprfs` with GATE_RECORD=off. The recipe's own tail would
  // otherwise ingest into the device bin store and cross an `epr artifact put`, both authoritative.
  const justfile = readFileSync(resolve(import.meta.dirname, '../../justfile'), 'utf8');
  const start = justfile.indexOf('\n_eprfs-ingest-around mode:');
  assert.ok(start >= 0, 'the _eprfs-ingest-around recipe exists');
  const recipe = justfile.slice(start, justfile.indexOf('\n\n', start + 1));
  const guard = recipe.indexOf('"${GATE_RECORD:-on}" == off');
  assert.ok(guard >= 0, 'the recipe checks GATE_RECORD');
  for (const write of ['bin --tool epr ingest', 'eprfs-produce.sh']) {
    const at = recipe.indexOf(write);
    assert.ok(at > guard, `${write} runs only after the GATE_RECORD=off branch`);
  }
});
