import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { createHash } from 'node:crypto';
import { chmodSync, mkdtempSync, mkdirSync, readFileSync, rmSync, utimesSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { closureKey, closureMaterial } from './gate-evidence.mjs';
import { closeRun, appendEvent, openRun, readRun } from './gate-live.mjs';
import {
  EXIT_NOT_CARRIED, TASK_ENV, prepareTask, readTaskStatus, resolveArk, taskAnswer, taskEnabled, taskEnv, taskRequest,
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
