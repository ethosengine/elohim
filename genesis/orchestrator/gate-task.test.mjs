import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { createHash } from 'node:crypto';
import { chmodSync, existsSync, mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, utimesSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { closureKey, closureMaterial } from './gate-evidence.mjs';
import { closeRun, appendEvent, openRun, readRun } from './gate-live.mjs';
import { spawnSync } from 'node:child_process';
import {
  EXIT_NOT_CARRIED, SOURCE_VIEW_BUDGET_MS, SOURCE_VIEW_SWEEP_MS, TASK_ENV, prepareTask, readShadowState, readTaskStatus,
  resolveArk, sourceViewBase, sourceViewCandidate, sourceViewComparison, sourceViewFiles, sourceViewMode, sourceViewShadow,
  sweepSourceView, taskAnswer, taskEnabled, taskEnv, taskRequest, writeShadowState,
} from './gate-task.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const orchestrator = { name: 'orchestrator', dir: 'genesis/orchestrator', steps: ['lint-jenkinsfiles'], run: { kind: 'just', recipe: 'gate' } };

function scratch(t) {
  const dir = mkdtempSync(join(tmpdir(), 'gate-task-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  return dir;
}

test('a Task runs by default for the proving gate only, and GATE_TASK overrides it', () => {
  const other = { name: 'elohim-ark', run: { kind: 'root-just' } };
  assert.equal(taskEnabled(orchestrator, {}), true);
  assert.equal(taskEnabled(other, {}), false);
  assert.equal(taskEnabled(orchestrator, { GATE_TASK: 'off' }), false);
  assert.equal(taskEnabled(other, { GATE_TASK: 'on' }), true);
  assert.equal(taskEnabled({ name: 'brit', run: { kind: 'attested' } }, { GATE_TASK: 'on' }), false);
});

test('the closure material is the bytes closureKey hashes', () => {
  const closure = { project: orchestrator, fingerprints: [{ step: 'lint-jenkinsfiles', fingerprint: 'bafy-a' }], manifestBlob: 'blob', cargoEnv: { B: '2', A: '1' } };
  const material = closureMaterial(closure);
  assert.equal(createHash('sha256').update(material).digest('hex'), closureKey(closure));
  assert.equal(material, JSON.stringify(JSON.parse(material)), 'no whitespace');
  assert.ok(material.indexOf('"A"') < material.indexOf('"B"'), 'sorted keys');
});

test('the Task child declares only the cargo pairs and the named environment, never a brace or a secret', () => {
  const { env, dropped } = taskEnv({
    GATE_CARGO_ENV: JSON.stringify({ RUSTUP_TOOLCHAIN: '1.96.1' }),
    EPR_BIN: '/bin/epr', JENKINS_TOKEN: 'secret', JENKINS_URL: 'https://j', PATH: '/bin',
    LANG: '{weird}',
  });
  assert.deepEqual(env, { RUSTUP_TOOLCHAIN: '1.96.1', EPR_BIN: '/bin/epr', JENKINS_URL: 'https://j' });
  assert.deepEqual(dropped, ['LANG']);
  assert.ok(!TASK_ENV.some(key => /TOKEN|SECRET|PASSWORD/.test(key)));
});

test('the request pins run-local-gate.sh by digest and names the gate method family', () => {
  const request = taskRequest(orchestrator, ['/repo', 'orchestrator'], { root: ROOT, env: {} });
  const sha = createHash('sha256').update(readFileSync(join(ROOT, 'genesis/orchestrator/run-local-gate.sh'))).digest('hex');
  assert.equal(request.child.artifact.pinned.sha256, sha);
  assert.deepEqual(request.child.argv, ['run-local-gate.sh', '/repo', 'orchestrator']);
  assert.equal(request.method.name, 'brit-gate:orchestrator');
  assert.equal(request.method.step, 'gate:orchestrator');
});

test('ARK_BIN wins; a pool ark older than its sources is a reason, never a build', t => {
  const dir = scratch(t);
  const ark = join(dir, 'ark');
  writeFileSync(ark, '#!/bin/sh\n');
  assert.match(resolveArk({ root: dir, env: { ARK_BIN: ark } }).why, /not executable/);
  chmodSync(ark, 0o755);
  assert.equal(resolveArk({ root: dir, env: { ARK_BIN: ark } }).bin, ark);

  const slot = join(dir, 'slot');
  mkdirSync(join(slot, 'debug'), { recursive: true });
  const pooled = join(slot, 'debug', 'ark');
  writeFileSync(pooled, '#!/bin/sh\n');
  chmodSync(pooled, 0o755);
  mkdirSync(join(dir, 'elohim/ark/cli/src'), { recursive: true });
  writeFileSync(join(dir, 'elohim/ark/cli/src/main.rs'), 'fn main() {}\n');
  const spawn = (command, args) => command === 'bash'
    ? { status: 0, stdout: `${slot}\n` }
    : { status: 0, stdout: 'elohim/ark/cli/src/main.rs\0' };
  utimesSync(pooled, new Date(2026, 0, 1), new Date(2026, 0, 1));
  assert.match(resolveArk({ root: dir, env: {}, spawn }).why, /older than elohim\/ark\/cli\/src\/main.rs/);
  utimesSync(pooled, new Date(), new Date(Date.now() + 60_000));
  assert.equal(resolveArk({ root: dir, env: {}, spawn }).bin, pooled);
  assert.match(resolveArk({ root: dir, env: {}, spawn: () => ({ status: 0, stdout: `${join(dir, 'none')}\n` }) }).why, /no ark at/);
});

test('the last status decides: answered, interrupted, or a fall back to the direct run', () => {
  const unanswered = { why: { reason: 'not-carried' }, comeBack: { when: 'on-event', event: 'rerun-requested' }, meanwhile: { local: 'run-here' }, escalation: null };
  assert.deepEqual(taskAnswer([], 66), { fallback: 'ark answered nothing (exit 66)' });
  assert.ok(taskAnswer([{ status: 'queued' }], 2).fallback);
  assert.deepEqual(taskAnswer([{ status: 'queued' }, { status: 'answered', value: { hit: true } }], 0), { answered: { hit: true } });
  assert.deepEqual(taskAnswer([{ status: 'queued' }, { status: 'unanswered', value: unanswered }], 143), { unanswered });
  assert.match(taskAnswer([{ status: 'queued' }, { status: 'unanswered', value: unanswered }], EXIT_NOT_CARRIED).fallback, /not carried here/);
});

test('prepareTask writes the material ark names and passes ark spec through', t => {
  const dir = scratch(t);
  const ark = join(dir, 'ark');
  // A fake ark: `task spec` prints a spec and two CIDs; argument order is ark's.
  writeFileSync(ark, `#!/bin/sh\necho '{"spec":{"s":1},"derivation":{"method":"bafyreim","input":"bafyreii"}}'\n`);
  chmodSync(ark, 0o755);
  mkdirSync(join(dir, 'genesis/orchestrator'), { recursive: true });
  writeFileSync(join(dir, 'genesis/orchestrator/run-local-gate.sh'), '#!/bin/sh\n');
  const material = closureMaterial({ project: orchestrator, fingerprints: [], manifestBlob: 'b', cargoEnv: {} });
  const task = prepareTask(orchestrator, {
    root: dir, env: { ARK_BIN: ark }, childEnv: { EPR_BIN: '/bin/epr' }, args: ['/r'],
    evidence: { material }, reuse: { mode: null, enabled: true, selfId: null, dirty: null, commit: 'HEAD' }, device: 'me',
    write: () => {},
  });
  assert.equal(task.why, undefined);
  assert.deepEqual(task.derivation, { method: 'bafyreim', input: 'bafyreii' });
  const at = name => task.argv[task.argv.indexOf(name) + 1];
  assert.equal(readFileSync(at('--material'), 'utf8'), material);
  assert.deepEqual(JSON.parse(readFileSync(at('--spec'), 'utf8')), { s: 1 });
  assert.equal(at('--device'), 'me');
  assert.deepEqual(readTaskStatus(task.status), []);
  assert.match(prepareTask(orchestrator, { root: dir, env: { ARK_BIN: ark }, evidence: { line: 'miss' } }).why, /no closure material/);
});

test('an interrupted Task closes its run interrupted with the three parts, never passed or failed', t => {
  const dir = scratch(t);
  const { path } = openRun({ root: dir, sha: 'abc', target: 'dev' });
  const unanswered = { why: { reason: 'not-carried' }, comeBack: { when: 'on-event', event: 'rerun-requested' }, meanwhile: { local: 'nothing', because: 'no verdict yet' }, escalation: null };
  appendEvent(path, { kind: 'gate-verdict', project: 'orchestrator', verdict: { exit: 143, seconds: 3 }, status: 'Unanswered', unanswered, derivation: { method: `bafyrei${'a'.repeat(52)}`, input: `bafyrei${'b'.repeat(52)}`, hit: false } });
  closeRun(path, 'failed');
  const close = readRun(path).events.at(-1);
  assert.equal(close.outcome, 'interrupted');
  assert.deepEqual(close.unanswered, unanswered);

  const { path: answered } = openRun({ root: dir, sha: 'abd', target: 'dev' });
  appendEvent(answered, { kind: 'gate-verdict', project: 'orchestrator', verdict: { exit: 1, seconds: 3 }, status: 'Answered' });
  closeRun(answered, 'failed');
  assert.equal(readRun(answered).events.at(-1).outcome, 'failed');
});

// Track A: the source-view shadow. Nothing it prints or fails at reaches the Task.
function shadowFixture(t) {
  const dir = scratch(t);
  const ark = join(dir, 'ark');
  writeFileSync(ark, '#!/bin/sh\nexit 0\n');
  chmodSync(ark, 0o755);
  mkdirSync(join(dir, 'genesis/orchestrator'), { recursive: true });
  writeFileSync(join(dir, 'genesis/orchestrator/run-local-gate.sh'), '#!/bin/sh\n');
  const material = closureMaterial({ project: orchestrator, fingerprints: [], manifestBlob: 'b', cargoEnv: {} });
  const specReply = { status: 0, stdout: '{"spec":{"s":1},"derivation":{"method":"bafyreim","input":"bafyreii"}}', stderr: '' };
  const prepare = (env, shadowReply) => {
    const calls = [];
    const lines = [];
    const spawn = (bin, args, opts) => {
      calls.push({ bin, args, opts });
      if (bin === ark && args[1] === 'spec') return specReply;
      if (bin === process.execPath) {
        if (typeof shadowReply === 'function') return shadowReply(bin, args, opts);
        return shadowReply;
      }
      return spawnSync(bin, args, opts);
    };
    const task = prepareTask(orchestrator, {
      root: dir, env: { ARK_BIN: ark, ...env }, childEnv: {}, args: ['/r'], evidence: { material, key: 'a'.repeat(64) },
      reuse: { mode: null, enabled: false, selfId: null, dirty: null, commit: 'c0ffee' }, device: 'me',
      spawn, now: () => 1_700_000_000_000, write: line => lines.push(line),
    });
    return { task, calls, lines };
  };
  return { dir, ark, prepare };
}

test('GATE_SOURCE_VIEW is off unless set to shadow, and off calls nothing', t => {
  assert.equal(sourceViewMode({}), 'off');
  assert.equal(sourceViewMode({ GATE_SOURCE_VIEW: 'shadow' }), 'shadow');
  assert.equal(sourceViewMode({ GATE_SOURCE_VIEW: 'off' }), 'off');
  assert.equal(sourceViewMode({ GATE_SOURCE_VIEW: 'on' }), 'off');
  const f = shadowFixture(t);
  for (const env of [{}, { GATE_SOURCE_VIEW: 'off' }]) {
    const { task, calls, lines } = f.prepare(env, () => { throw new Error('must not be called'); });
    assert.equal(task.why, undefined);
    assert.deepEqual(calls.map(c => c.args.slice(0, 2)), [['task', 'spec']], 'only ark task spec ran');
    assert.deepEqual(lines, []);
  }
});

test('a shadow failure, a throw or a timeout leaves the prepared Task exactly as off does', t => {
  const f = shadowFixture(t);
  const off = f.prepare({ GATE_SOURCE_VIEW: 'off' }, null).task;
  const cases = [
    [{ status: 1, stdout: '', stderr: 'suite-run: git show failed\n' }, /unmeasured \(suite-run: git show failed\)$/],
    [() => { throw new Error('spawn blew up'); }, /unmeasured \(spawn blew up\)$/],
    [{ status: null, signal: 'SIGKILL', error: Object.assign(new Error('spawnSync node ETIMEDOUT'), { code: 'ETIMEDOUT' }) }, /unmeasured \(timed out after 20 s\)$/],
    [{ status: 0, stdout: 'not json', stderr: '' }, /unmeasured \(the shadow printed no JSON\)$/],
  ];
  for (const [reply, line] of cases) {
    const { task, calls, lines } = f.prepare({ GATE_SOURCE_VIEW: 'shadow' }, reply);
    assert.deepEqual(task, off, 'the Task, its argv, spec and derivation are unchanged');
    assert.deepEqual(JSON.parse(readFileSync(task.argv[task.argv.indexOf('--spec') + 1], 'utf8')), { s: 1 });
    assert.equal(lines.length, 1);
    assert.match(lines[0], /^\[gate-task\] source-view shadow: /);
    assert.match(lines[0].trimEnd(), line);
    const shadow = calls.find(c => c.bin === process.execPath);
    assert.equal(shadow.opts.timeout, SOURCE_VIEW_BUDGET_MS, 'the whole shadow is capped');
    assert.equal(shadow.opts.killSignal, 'SIGKILL');
    assert.equal(shadow.opts.detached, true, 'the child leads its own process group');
    assert.deepEqual(readdirSync(sourceViewBase(f.dir)), [], 'the parent removed the run\'s scratch');
  }
});

test('the shadow line names project, commit, legacy, candidate, files and ms, then what moved', t => {
  const dir = scratch(t);
  let clock = 1000;
  const reply = candidate => () => ({ status: 0, stdout: `${JSON.stringify({ commit: 'c0ffee'.repeat(7).slice(0, 40), candidate, files: 412 })}\n`, stderr: '' });
  const run = (candidate, key) => sourceViewShadow(orchestrator, {
    root: dir, ark: '/bin/ark', commit: 'HEAD', evidence: { key }, spawn: reply(candidate), now: () => (clock += 37),
  });
  const cid = `bafyrei${'c'.repeat(52)}`;
  const first = run(cid, 'a'.repeat(64));
  assert.equal(first, `[gate-task] source-view shadow: project=orchestrator commit=c0ffeec0ffee legacy=aaaaaaaaaaaa candidate=${cid} files=412 ms=37 vs=none`);
  assert.match(first, /^\[gate-task\] source-view shadow: project=\S+ commit=[0-9a-f]{12} legacy=[0-9a-f]{12} candidate=\S+ files=\d+ ms=\d+ /);
  assert.match(run(cid, 'b'.repeat(64)), / vs=c0ffeec0ffee legacy:changed candidate:same DISAGREE$/);
  assert.match(run(`${cid.slice(0, -1)}d`, 'c'.repeat(64)), / legacy:changed candidate:changed agree$/);
  assert.match(sourceViewShadow({ name: 'elohim-ark' }, { root: dir, evidence: { key: 'k' } }), /unmeasured \(no concrete source list for elohim-ark\)$/);
  assert.equal(sourceViewComparison(null, {}), 'vs=none');
});

test('the candidate captures the inventory\'s files materialised at the gated commit, not the working tree', t => {
  const dir = scratch(t);
  const g = (...args) => {
    const r = spawnSync('git', args, { cwd: dir, encoding: 'utf8', env: { ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_CONFIG_NOSYSTEM: '1' } });
    assert.equal(r.status, 0, r.stderr);
    return r.stdout.trim();
  };
  g('init', '-q');
  mkdirSync(join(dir, 'genesis/orchestrator'), { recursive: true });
  mkdirSync(join(dir, 'lib'), { recursive: true });
  mkdirSync(join(dir, 'other'), { recursive: true });
  writeFileSync(join(dir, 'genesis/orchestrator/suite-reads.json'), JSON.stringify({ groups: [{ files: [{ file: 'a.test.mjs', reads: ['lib/x.txt'] }] }] }));
  writeFileSync(join(dir, 'genesis/orchestrator/a.test.mjs'), 'committed\n');
  writeFileSync(join(dir, 'lib/x.txt'), 'x\n');
  writeFileSync(join(dir, 'other/y.txt'), 'not read\n');
  writeFileSync(join(dir, 'package.json'), '{}\n');
  g('add', '-A');
  g('-c', 'user.name=t', '-c', 'user.email=t@t', 'commit', '-q', '-m', 'c');
  const head = g('rev-parse', 'HEAD');
  writeFileSync(join(dir, 'genesis/orchestrator/a.test.mjs'), 'dirty working tree\n');
  let seen = null;
  const spawn = (bin, args) => {
    const at = name => args[args.indexOf(name) + 1];
    const files = JSON.parse(readFileSync(at('--files'), 'utf8'));
    seen = { bin, files, bytes: readFileSync(join(at('--checkout'), 'genesis/orchestrator/a.test.mjs'), 'utf8') };
    return { status: 0, stdout: '{"projected":"bafyreiprojected","captured":"bafyreiprojected","files":4,"bytes":9}\n', stderr: '' };
  };
  const got = sourceViewCandidate({ root: dir, commit: 'HEAD', ark: '/bin/ark', spawn, scratchBase: scratch(t) });
  assert.deepEqual(got, { commit: head, candidate: 'bafyreiprojected', files: 4 });
  assert.equal(seen.bin, '/bin/ark');
  assert.deepEqual(seen.files, ['genesis/orchestrator/a.test.mjs', 'genesis/orchestrator/suite-reads.json', 'lib/x.txt', 'package.json']);
  assert.equal(seen.bytes, 'committed\n', 'the gated commit\'s bytes, never the working tree\'s');
  assert.throws(() => sourceViewCandidate({ root: dir, ark: '/bin/ark', scratchBase: scratch(t), spawn: () => ({ status: 65, stdout: '', stderr: 'ark: capture: refused\n' }) }), /ark task source-root: ark: capture: refused/);
});

test('a shadow child past its cap is killed with its whole group, and its scratch is removed', t => {
  const dir = scratch(t);
  const pids = join(dir, 'pids');
  // Stands in for node: leads its group, leaves a grandchild that would outlive it, and sleeps past the cap.
  const child = join(dir, 'slow-child');
  writeFileSync(child, `#!/bin/sh\nsleep 30 &\necho "$$ $!" > '${pids}'\nsleep 30\n`);
  chmodSync(child, 0o755);
  const started = Date.now();
  const line = sourceViewShadow(orchestrator, {
    root: dir, ark: '/bin/false', commit: 'HEAD', evidence: { key: 'a'.repeat(64) }, execPath: child, budgetMs: 400,
  });
  assert.match(line, /unmeasured \(timed out after 0\.4 s\)$/);
  assert.ok(Date.now() - started < 4000, 'the cap held');
  const [parent, grandchild] = readFileSync(pids, 'utf8').trim().split(' ').map(Number);
  const gone = pid => {
    try { process.kill(pid, 0); } catch { return true; }
    try { return /^\d+ \(.*\) Z/.test(readFileSync(`/proc/${pid}/stat`, 'utf8')); } catch { return true; }
  };
  const until = Date.now() + 1500;
  while (!(gone(parent) && gone(grandchild)) && Date.now() < until) spawnSync('sleep', ['0.05']);
  assert.ok(gone(parent), `child ${parent} is gone`);
  assert.ok(gone(grandchild), `grandchild ${grandchild} is gone`);
  assert.deepEqual(readdirSync(sourceViewBase(dir)), [], 'the run\'s scratch directory is removed');
});

test('leftover scratch runs older than ten minutes are swept; the state file is replaced whole', t => {
  const dir = scratch(t);
  const base = sourceViewBase(dir);
  mkdirSync(join(base, 'run-old/checkout'), { recursive: true });
  mkdirSync(join(base, 'run-new'), { recursive: true });
  mkdirSync(join(base, 'kept'), { recursive: true });
  const old = new Date(Date.now() - SOURCE_VIEW_SWEEP_MS - 60_000);
  utimesSync(join(base, 'run-old'), old, old);
  utimesSync(join(base, 'kept'), old, old);
  assert.equal(sweepSourceView(base), 1);
  assert.deepEqual(readdirSync(base).sort(), ['kept', 'run-new']);

  const state = join(dir, 'state.json');
  assert.deepEqual(readShadowState(state), {}, 'missing is no previous run');
  for (const bad of ['{not json', '[1]', 'null', '"x"']) {
    writeFileSync(state, bad);
    assert.deepEqual(readShadowState(state), {}, `malformed ${bad} is no previous run`);
  }
  writeShadowState(state, { orchestrator: { commit: 'c', legacy: 'l', candidate: 'k' } });
  assert.deepEqual(readShadowState(state), { orchestrator: { commit: 'c', legacy: 'l', candidate: 'k' } });
  assert.deepEqual(readdirSync(dir).filter(name => name.endsWith('.tmp')), [], 'no temporary file is left');
});

test('materialised modes come from git, not the umask: the same candidate under 0o177 and 0o022', t => {
  const dir = scratch(t);
  const g = (...args) => {
    const r = spawnSync('git', args, { cwd: dir, encoding: 'utf8', env: { ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_CONFIG_NOSYSTEM: '1' } });
    assert.equal(r.status, 0, r.stderr);
    return r.stdout.trim();
  };
  g('init', '-q');
  mkdirSync(join(dir, 'genesis/orchestrator'), { recursive: true });
  writeFileSync(join(dir, 'genesis/orchestrator/suite-reads.json'), JSON.stringify({ groups: [{ files: [{ file: 'a.test.mjs', reads: [] }] }] }));
  writeFileSync(join(dir, 'genesis/orchestrator/a.test.mjs'), 'plain\n');
  writeFileSync(join(dir, 'genesis/orchestrator/run.sh'), '#!/bin/sh\n');
  chmodSync(join(dir, 'genesis/orchestrator/run.sh'), 0o755);
  writeFileSync(join(dir, 'package.json'), '{}\n');
  g('add', '-A');
  g('-c', 'user.name=t', '-c', 'user.email=t@t', 'commit', '-q', '-m', 'c');
  // Stands in for capture's address: every file's path, executable bit and bytes.
  const spawn = (_bin, args) => {
    const at = name => args[args.indexOf(name) + 1];
    const checkout = at('--checkout');
    const hash = createHash('sha256');
    for (const file of JSON.parse(readFileSync(at('--files'), 'utf8'))) {
      const mode = statSync(join(checkout, file)).mode;
      hash.update(`${file}\0${mode & 0o111 ? 'x' : '-'}${mode & 0o777}\0`).update(readFileSync(join(checkout, file)));
    }
    return { status: 0, stdout: JSON.stringify({ projected: hash.digest('hex') }), stderr: '' };
  };
  const candidate = mask => {
    const previous = process.umask(mask);
    try {
      return sourceViewCandidate({ root: dir, ark: '/bin/ark', spawn, scratchBase: scratch(t) }).candidate;
    } finally {
      process.umask(previous);
    }
  };
  assert.equal(candidate(0o177), candidate(0o022));
  const dest = scratch(t);
  const previous = process.umask(0o177);
  try { sourceViewFiles({ root: dir, commit: 'HEAD', dest }); } finally { process.umask(previous); }
  assert.equal(statSync(join(dest, 'genesis/orchestrator/run.sh')).mode & 0o777, 0o755);
  assert.equal(statSync(join(dest, 'genesis/orchestrator/a.test.mjs')).mode & 0o777, 0o644);
});
