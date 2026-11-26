// A requested stop ends the gate that is RUNNING (`gate-live.mjs stop --now`) and records nothing
// for it; a plain stop still lets the running gate finish. Habit gate-failures-arrive-live.

import { describe, test } from 'node:test';
import { strict as assert } from 'node:assert';
import { spawn, spawnSync } from 'node:child_process';
import { appendFileSync, chmodSync, closeSync, existsSync, mkdirSync, mkdtempSync, openSync, readFileSync, rmSync, utimesSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { setTimeout as delay } from 'node:timers/promises';

import { appendEvent, closeRun, formatEvent, openRun, readRun, requestStop, stopRequested } from './gate-live.mjs';
import { GATE_STOPPED_EXIT, runProject } from './gate-runner.mjs';
import { endTree, gateTree, readStat, snapshot, stopGraceMs, stoppedBy } from './gate-stop.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const TAP = resolve(HERE, 'gate-tap.mjs');
const LIVE = resolve(HERE, 'gate-live.mjs');
delete process.env.GATE_LIVE_REF;
delete process.env.GATE_LIVE_RUN;
// No berth: a closed fixture run must not send a sibling final.
process.env.GATE_LIVE_SIBLING = 'off';

function nodeCliEnv(extra = {}) {
  const env = { ...process.env, ...extra };
  delete env.NODE_TEST_CONTEXT;
  delete env.NODE_TEST_WORKER_ID;
  return env;
}

function outputCapture(prefix) {
  const dir = mkdtempSync(resolve(tmpdir(), prefix));
  const path = resolve(dir, 'output');
  const fd = openSync(path, 'w');
  let closed = false;
  return {
    fd,
    close() {
      if (!closed) { closeSync(fd); closed = true; }
    },
    read() {
      this.close();
      const output = readFileSync(path, 'utf8');
      rmSync(dir, { recursive: true, force: true });
      return output;
    },
  };
}

function spawnNodeSync(args, options = {}) {
  const capture = outputCapture('gate-stop-cli-');
  const result = spawnSync(process.execPath, args, { ...options, encoding: undefined, stdio: ['ignore', capture.fd, capture.fd] });
  const output = capture.read();
  return { ...result, stdout: output, stderr: '' };
}

const alive = pid => {
  const stat = readStat(pid);
  return Boolean(stat) && stat.state !== 'Z' && stat.state !== 'X';
};
const readPid = path => Number(readFileSync(path, 'utf8').trim());

async function waitFor(predicate, ms = 30_000) {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    if (predicate()) return true;
    await delay(50);
  }
  return predicate();
}

// A long gate recipe: the leader starts a child subshell, which starts a grandchild sleep; each
// writes its pid. `stubborn` makes the grandchild ignore TERM, so only the KILL after the grace ends it.
function fakeGate(dir, { stubborn = false, seconds = 300 } = {}) {
  const script = resolve(dir, 'gate.sh');
  const grandchild = stubborn ? `bash -c "trap '' TERM; exec sleep ${seconds}"` : `sleep ${seconds}`;
  writeFileSync(script, [
    '#!/usr/bin/env bash',
    `echo $$ > "${dir}/leader"`,
    `( ${grandchild} & echo $! > "${dir}/grandchild"; wait ) &`,
    `echo $! > "${dir}/child"`,
    'echo "test long_gate ... started"',
    'wait',
  ].join('\n'), { mode: 0o755 });
  return script;
}

function scratch(t, prefix) {
  const dir = mkdtempSync(resolve(tmpdir(), prefix));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  return dir;
}

// The tap, started asynchronously so the test can stop it mid-gate; fd 3 carries its summary.
function startTap(run, script, env = {}) {
  const capture = outputCapture('gate-tap-stderr-');
  const child = spawn(process.execPath, [TAP, '--project', 'fixture', '--run', run, '--', 'bash', script], {
    stdio: ['ignore', 'ignore', capture.fd, 'pipe'],
    env: nodeCliEnv({ GATE_STOP_GRACE_MS: '1500', ...env }),
  });
  let fd3 = '';
  capture.close();
  child.stdio[3].on('data', chunk => { fd3 += chunk; });
  const done = new Promise(res => child.on('close', (code, signal) => res({
    code, signal, stderr: capture.read(), summary: fd3.trim() ? JSON.parse(fd3.trim().split('\n').pop()) : null,
  })));
  return { child, done };
}

describe('stop --now ends the running gate', () => {
  test('TERM ends the leader, its child and its grandchild within the grace', { timeout: 60_000 }, async t => {
    const dir = scratch(t, 'gate-stop-term-');
    const { path: run } = openRun({ root: dir, sha: 'a'.repeat(40), target: 'test' });
    const { done } = startTap(run, fakeGate(dir));
    assert.ok(await waitFor(() => existsSync(resolve(dir, 'grandchild'))), 'the fake gate started its tree');
    const pids = ['leader', 'child', 'grandchild'].map(name => readPid(resolve(dir, name)));
    assert.ok(pids.every(alive), 'all three run before the stop');

    const asked = Date.now();
    const request = requestStop(run, { immediate: true, by: 'session-stopper', reason: 'pre-gate red' });
    const result = await done;
    const tookMs = Date.now() - asked;

    assert.ok(tookMs < 16_000, `ended in ${tookMs} ms, within the loaded-host margin around the 3500 ms bound`);
    for (const pid of pids) assert.equal(alive(pid), false, `pid ${pid} is gone`);
    assert.deepEqual(result.summary.stopped, { by: 'session-stopper', reason: 'pre-gate red', request: request.id, signal: 'SIGTERM' });
    assert.ok(result.code !== 0 || result.signal !== null, 'the tap does not exit 0');
    assert.match(result.stderr, /stopped on request by session-stopper: its leader died of SIGTERM and its process group was ended \(SIGTERM\); no verdict is recorded for it/);
    // The gate's own death throes are the stop, not reds: nothing was appended as a failure.
    assert.equal(readRun(run).events.filter(event => event.kind === 'failure').length, 0);
  });

  test('a grandchild that ignores TERM is KILLed once the grace is over', { timeout: 60_000 }, async t => {
    const dir = scratch(t, 'gate-stop-kill-');
    const { path: run } = openRun({ root: dir, sha: 'b'.repeat(40), target: 'test' });
    const { done } = startTap(run, fakeGate(dir, { stubborn: true }), { GATE_STOP_GRACE_MS: '600' });
    assert.ok(await waitFor(() => existsSync(resolve(dir, 'grandchild'))));
    const grandchild = readPid(resolve(dir, 'grandchild'));
    await waitFor(() => { try { return readFileSync(`/proc/${grandchild}/cmdline`, 'utf8').startsWith('sleep'); } catch { return false; } });
    requestStop(run, { immediate: true, by: 'session-stopper' });
    const result = await done;
    assert.equal(alive(grandchild), false);
    assert.equal(result.summary.stopped.signal, 'SIGKILL');
  });

  test('a TERM-ignoring gate keeps its cargo slot until the stopped group is gone', { timeout: 60_000 }, async t => {
    const dir = scratch(t, 'gate-stop-slot-lock-');
    const { path: run } = openRun({ root: dir, sha: 'd'.repeat(40), target: 'test' });
    const lock = resolve(dir, 'slot.gate.lock');
    const holder = resolve(dir, 'slot/.gate.holder');
    const first = startTap(run, fakeGate(dir, { stubborn: true }), {
      GATE_STOP_GRACE_MS: '600', GATE_SLOT_LOCK_PATH: lock, GATE_SLOT_HOLDER_PATH: holder,
      GATE_SLOT_WAIT_RESULT: resolve(dir, 'first-wait.tsv'),
    });
    assert.ok(await waitFor(() => existsSync(resolve(dir, 'grandchild'))));
    const firstGrandchild = readPid(resolve(dir, 'grandchild'));

    const entered = resolve(dir, 'second-entered');
    const secondScript = resolve(dir, 'second.sh');
    writeFileSync(secondScript, `#!/usr/bin/env bash
state=$(awk '{ print $3 }' /proc/${firstGrandchild}/stat 2>/dev/null || true)
if [[ -n "$state" && "$state" != Z && "$state" != X ]]; then exit 97; fi
printf entered > ${JSON.stringify(entered)}
`, { mode: 0o755 });
    const { path: secondRun } = openRun({ root: dir, sha: 'e'.repeat(40), target: 'second-slot-test' });
    const second = startTap(secondRun, secondScript, {
      GATE_SLOT_LOCK_PATH: lock, GATE_SLOT_HOLDER_PATH: holder,
      GATE_SLOT_WAIT_RESULT: resolve(dir, 'second-wait.tsv'), GATE_SLOT_LOCK_WAIT_S: '5',
    });
    assert.ok(await waitFor(() => readRun(secondRun).events.some(event => event.kind === 'wait')),
      'the second gate never waited on the first holder');

    requestStop(run, { immediate: true, by: 'session-stopper' });
    const stopped = await first.done;
    assert.equal(stopped.summary.stopped.signal, 'SIGKILL');
    assert.equal((await second.done).code, 0, 'the second recipe entered only after the first group was gone');
    assert.ok(existsSync(entered), 'the second gate enters after the first group is gone');
  });

  test('a plain stop leaves the running gate to finish', { timeout: 60_000 }, async t => {
    const dir = scratch(t, 'gate-stop-plain-');
    const { path: run } = openRun({ root: dir, sha: 'c'.repeat(40), target: 'test' });
    const { done } = startTap(run, fakeGate(dir, { seconds: 1 }));
    assert.ok(await waitFor(() => existsSync(resolve(dir, 'grandchild'))));
    requestStop(run, { reason: 'after this one', by: 'session-stopper' });
    const result = await done;
    assert.equal(result.code, 0, 'the gate ran to its own end');
    assert.equal(result.summary.stopped, undefined);
  });
});

describe('only the gate is ever signalled', () => {
  test('a recorded pid in another process group is not signalled', { timeout: 60_000 }, async t => {
    // A foreign process in its own session and group, recorded as if it were known: it is not in
    // the gate's session and descends from nothing of the gate's, so it is never a target.
    const foreign = spawn('sleep', ['30'], { detached: true, stdio: 'ignore' });
    t.after(() => { try { process.kill(foreign.pid, 'SIGKILL'); } catch { /* gone */ } });
    const gate = spawn('sleep', ['30'], { detached: true, stdio: 'ignore' });
    t.after(() => { try { process.kill(gate.pid, 'SIGKILL'); } catch { /* gone */ } });
    await waitFor(() => readStat(foreign.pid) && readStat(gate.pid));
    const foreignStat = readStat(foreign.pid);
    const gateStat = readStat(gate.pid);
    assert.notEqual(foreignStat.pgid, gateStat.pgid);

    const known = new Set([`${gateStat.pid}:${gateStat.start}`, `${foreignStat.pid}:${foreignStat.start}`]);
    const tree = gateTree(gate.pid, known, snapshot());
    assert.equal(tree.custody, true);
    assert.deepEqual(tree.members.map(member => member.pid), [gate.pid]);

    const sent = [];
    const kill = (pid, signal) => { sent.push([pid, signal]); process.kill(pid, signal); };
    const result = await endTree(gateStat, { graceMs: 500, kill });
    assert.equal(result.signal, 'SIGTERM');
    assert.ok(sent.every(([pid]) => pid === -gate.pid || pid === gate.pid), JSON.stringify(sent));
    assert.equal(alive(foreign.pid), true, 'the foreign process still runs');
  });

  test('a leader whose pid was reused (another start time) gives no custody and no signal', { timeout: 60_000 }, async t => {
    const stranger = spawn('sleep', ['30'], { detached: true, stdio: 'ignore' });
    t.after(() => { try { process.kill(stranger.pid, 'SIGKILL'); } catch { /* gone */ } });
    await waitFor(() => readStat(stranger.pid));
    const now = readStat(stranger.pid);
    // The number is the recorded leader's, the start time is not: the recorded gate is gone.
    const recorded = { pid: now.pid, start: String(Number(now.start) - 1) };
    const sent = [];
    const result = await endTree(recorded, { graceMs: 100, kill: (pid, signal) => sent.push([pid, signal]) });
    assert.equal(result.custody, false);
    assert.deepEqual(sent, []);
    assert.equal(alive(stranger.pid), true);
  });

  test('the grace is bounded', { timeout: 60_000 }, () => {
    assert.equal(stopGraceMs({}), 10_000);
    assert.equal(stopGraceMs({ GATE_STOP_GRACE_MS: '250' }), 250);
    assert.equal(stopGraceMs({ GATE_STOP_GRACE_MS: '999999' }), 60_000);
    assert.equal(stopGraceMs({ GATE_STOP_GRACE_MS: 'soon' }), 10_000);
  });
});

// runProject with its evidence path live: a fake brit answers the closure lookups and counts every
// put, so a stopped gate that recorded anything would show here.
function evidenceFixture(t, prefix, { seconds, recipe = null }) {
  const root = scratch(t, prefix);
  const pids = resolve(root, 'pids');
  mkdirSync(pids);
  mkdirSync(resolve(root, 'genesis/orchestrator'), { recursive: true });
  // The recipe the tap runs is this fixture's run-local-gate.sh: by default a long gate with a child
  // and a grandchild; `recipe(pids)` supplies another.
  writeFileSync(resolve(root, 'genesis/orchestrator/run-local-gate.sh'),
    recipe ? recipe(pids) : readFileSync(fakeGate(pids, { seconds }), 'utf8'));
  const bin = resolve(root, 'brit');
  writeFileSync(bin, '', { mode: 0o755 });
  const { path: run } = openRun({ root, sha: 'd'.repeat(40), target: 'test' });
  const project = { name: 'fixture', dir: 'fixture', steps: ['build'],
    manifestPath: 'build-manifest.json', run: { kind: 'just', recipe: 'gate' } };
  const calls = { puts: 0, epr: [], gate: 0 };
  const spawnFake = (command, args, options) => {
    const ok = stdout => ({ status: 0, stdout, stderr: '' });
    if (command === process.execPath && args[0] === TAP) { calls.gate += 1; return spawnSync(command, args, options); }
    if (command === 'epr' || command === process.env.EPR_BIN) { calls.epr.push(args.join(' ')); return ok(''); }
    if (args[0] === 'rev-parse') return ok('e'.repeat(40));
    if (args[0] === 'show') return ok(JSON.stringify({ steps: { build: { inputs: { sources: ['fixture/**'] } } } }));
    if (args[0] === 'status' || args[0] === 'ls-tree') return ok('');
    if (args[1] === 'fingerprint') return ok(JSON.stringify({ fingerprints: [{ step: 'build', fingerprint: 'fp' }] }));
    if (args[3] === 'find') return ok('[]');
    if (args[2] === 'signer') return ok(JSON.stringify({ agentId: 'ab'.repeat(32) }));
    if (args[3] === 'put') { calls.puts += 1; return ok('bafyreistored'); }
    return { status: 7, stdout: '', stderr: '' };
  };
  const env = { PATH: process.env.PATH, BRIT_BIN: bin, BRIT_BUILD_REF_BIN: bin, EPR_BIN: 'epr', GATE_TASK: 'off',
    GATE_EVIDENCE: 'off', GATE_INPUTS: 'off', GATE_RECIPE_KEY: 'off', GATE_LIVE_RUN: run,
    GATE_STOP_GRACE_MS: '1500', GATE_LIVE_SIBLING: 'off' };
  return { root, pids, run, project, calls, spawnFake, env };
}

function outsideSignalFixture(t, { recipe = null } = {}) {
  const scratchDir = scratch(t, 'gate-outside-signal-');
  const root = resolve(scratchDir, 'repo');
  const ready = resolve(scratchDir, 'ready');
  const calls = resolve(scratchDir, 'calls');
  const eprCalls = resolve(scratchDir, 'epr-calls');
  mkdirSync(resolve(root, 'fixture'), { recursive: true });
  mkdirSync(resolve(root, 'genesis/orchestrator'), { recursive: true });
  mkdirSync(resolve(root, 'genesis/agentic/bin'), { recursive: true });
  writeFileSync(resolve(root, '.gitignore'), '.eprfs/\n');
  writeFileSync(resolve(root, 'fixture/build-manifest.json'), JSON.stringify({
    pipeline: 'fixture',
    steps: { build: { inputs: { sources: ['fixture/**'], buildProcess: ['justfile', 'genesis/orchestrator/run-local-gate.sh'] } } },
    gate: { projects: { fixture: { dir: 'fixture', steps: ['build'], run: { kind: 'root-just', recipe: '_gate-fixture' } } } },
  }));
  writeFileSync(resolve(root, 'justfile'), '_gate-fixture:\n    true\n');
  writeFileSync(resolve(root, 'genesis/orchestrator/run-local-gate.sh'), recipe ? recipe({ scratchDir, ready }) : [
    '#!/usr/bin/env bash',
    'printf ready > "$OUTSIDE_READY"',
    'exec sleep 300',
  ].join('\n'), { mode: 0o755 });
  for (const path of ['genesis/orchestrator/cargo-lint.mjs', 'genesis/orchestrator/gate-steps.sh', 'genesis/agentic/bin/pool-lib.sh']) {
    writeFileSync(resolve(root, path), '\n');
  }
  const gitEnv = { ...process.env, GIT_AUTHOR_NAME: 'Fixture', GIT_AUTHOR_EMAIL: 'fixture@example.test',
    GIT_COMMITTER_NAME: 'Fixture', GIT_COMMITTER_EMAIL: 'fixture@example.test' };
  const git = args => spawnSync('git', args, { cwd: root, env: gitEnv, encoding: 'utf8' });
  assert.equal(git(['init', '-q']).status, 0);
  assert.equal(git(['add', '.']).status, 0);
  assert.equal(git(['commit', '-qm', 'fixture']).status, 0);

  const brit = resolve(scratchDir, 'brit');
  writeFileSync(brit, `#!/usr/bin/env node
const fs = require('node:fs');
const args = process.argv.slice(2);
fs.appendFileSync(process.env.OUTSIDE_CALLS, args.join(' ') + '\\n');
const at = name => args.indexOf(name);
if (args[0] === 'build' && args[1] === 'fingerprint') {
  const step = args[at('--step') + 1];
  fs.writeFileSync(1, JSON.stringify({ fingerprints: [{ step, fingerprint: 'fixture-fingerprint' }] }) + '\\n');
} else if (at('find') >= 0) fs.writeFileSync(1, '[]\\n');
else if (at('put') >= 0) fs.writeFileSync(1, 'bafyfixture\\n');
else if (at('signer') >= 0) fs.writeFileSync(1, JSON.stringify({ agentId: 'ab'.repeat(32) }) + '\\n');
`, { mode: 0o755 });
  chmodSync(brit, 0o755);
  const epr = resolve(scratchDir, 'epr');
  writeFileSync(epr, '#!/usr/bin/env bash\nprintf "%s\\n" "$*" >> "$OUTSIDE_EPR_CALLS"\n', { mode: 0o755 });
  chmodSync(epr, 0o755);
  const env = {
    ...nodeCliEnv(),
    GATE_ROOT: root,
    GATE_PREFLIGHT: 'off',
    GATE_TASK: 'off',
    GATE_INPUTS: 'off',
    GATE_RECIPE_KEY: 'off',
    GATE_READ_SHADOW: 'off',
    GATE_GRADE: 'off',
    GATE_READING: 'off',
    GATE_EVIDENCE: 'off',
    GATE_LIVE_SIBLING: 'off',
    BRIT_BIN: brit,
    BRIT_BUILD_REF_BIN: brit,
    EPR_BIN: epr,
    OUTSIDE_READY: ready,
    OUTSIDE_CALLS: calls,
    OUTSIDE_EPR_CALLS: eprCalls,
  };
  return { root, scratchDir, ready, calls, eprCalls, env };
}

// The stop comes from another process, as the integrator's does: runProject blocks this one.
function stopperFor(run, pids, { immediate, marker = 'grandchild', afterMs = 0 }) {
  const code = `
    import { existsSync } from 'node:fs';
    import { requestStop } from ${JSON.stringify(LIVE)};
    const until = Date.now() + 15000;
    while (!existsSync(${JSON.stringify(resolve(pids, marker))}) && Date.now() < until) await new Promise(r => setTimeout(r, 50));
    await new Promise(r => setTimeout(r, ${afterMs}));
    requestStop(${JSON.stringify(run)}, { immediate: ${immediate}, by: 'session-integrator' });`;
  return spawn(process.execPath, ['--input-type=module', '-e', code], { stdio: 'ignore' });
}

describe('a stopped gate records no evidence', () => {
  test('TERM delivered to the gate tap records no verdict or evidence', { timeout: 60_000 }, async t => {
    const fx = outsideSignalFixture(t);
    const cli = resolve(HERE, 'gate-runner.mjs');
    const stdoutCapture = outputCapture('gate-outside-stdout-');
    const stderrCapture = outputCapture('gate-outside-stderr-');
    const child = spawn(process.execPath, [cli, '--target', 'fixture'], {
      cwd: fx.root,
      env: fx.env,
      stdio: ['ignore', stdoutCapture.fd, stderrCapture.fd],
    });
    stdoutCapture.close();
    stderrCapture.close();
    t.after(() => { try { child.kill('SIGTERM'); } catch { /* gone */ } });
    assert.ok(await waitFor(() => existsSync(fx.ready)), 'the fixture gate did not start');
    const tap = [...snapshot().values()].find(stat => stat.ppid === child.pid);
    assert.ok(tap, 'the runner had no gate-tap child');
    process.kill(tap.pid, 'SIGTERM');
    const result = await new Promise(resolveDone => child.once('close', (code, signal) => resolveDone({ code, signal })));
    const stdout = stdoutCapture.read();
    const stderr = stderrCapture.read();

    assert.deepEqual(result, { code: GATE_STOPPED_EXIT, signal: null }, stderr);
    assert.match(stdout, /\[gate\] fixture: stopped from outside \(signal 15\); no verdict recorded/, `${stdout}\n${stderr}`);
    assert.doesNotMatch(stdout, /evidence: recorded/);
    const evidenceCalls = readFileSync(fx.calls, 'utf8');
    assert.match(evidenceCalls, /build find/, 'the fixture had a recordable closure before it was stopped');
    assert.doesNotMatch(evidenceCalls, /build put/);
    const nativeCalls = existsSync(fx.eprCalls) ? readFileSync(fx.eprCalls, 'utf8') : '';
    assert.doesNotMatch(nativeCalls, /flow (?:note|observe)/, nativeCalls);
    const runPath = /live failures: .* --run (.+)\n/.exec(stdout)?.[1];
    assert.ok(runPath, stdout);
    const events = readRun(runPath).events;
    assert.equal(events.filter(event => event.kind === 'gate-verdict').length, 0);
    assert.deepEqual(events.filter(event => event.kind === 'gate-stopped').map(event => ({
      project: event.project, by: event.by, signal: event.signal, evidence: event.evidence,
    })), [{ project: 'fixture', by: null, signal: 'SIGTERM', evidence: 'none' }]);
    assert.equal(events.at(-1).kind, 'run-close');
    assert.equal(events.at(-1).outcome, 'interrupted');
  });

  test('outside TERM after a failed leader exit keeps the red verdict and evidence', { timeout: 60_000 }, async t => {
    const fx = outsideSignalFixture(t, { recipe: ({ scratchDir }) => [
      '#!/usr/bin/env bash',
      `echo $$ > "${scratchDir}/leader"`,
      `touch "${scratchDir}/hold-output"`,
      `while [ -e "${scratchDir}/hold-output" ]; do sleep 0.05; done &`,
      `echo $! > "${scratchDir}/leaked"`,
      'echo "test late_failure ... FAILED"',
      'printf ready > "$OUTSIDE_READY"',
      `printf exiting > "${scratchDir}/leader-exiting"`,
      'exit 1',
    ].join('\n') });
    t.after(() => rmSync(resolve(fx.scratchDir, 'hold-output'), { force: true }));
    const cli = resolve(HERE, 'gate-runner.mjs');
    const stdoutCapture = outputCapture('gate-outside-late-stdout-');
    const stderrCapture = outputCapture('gate-outside-late-stderr-');
    const child = spawn(process.execPath, [cli, '--target', 'fixture'], {
      cwd: fx.root,
      env: fx.env,
      stdio: ['ignore', stdoutCapture.fd, stderrCapture.fd],
    });
    stdoutCapture.close();
    stderrCapture.close();
    t.after(() => { try { child.kill('SIGTERM'); } catch { /* gone */ } });
    assert.ok(await waitFor(() => existsSync(resolve(fx.scratchDir, 'leader-exiting'))),
      'the fixture leader did not mark its own exit');
    const leader = readPid(resolve(fx.scratchDir, 'leader'));
    assert.ok(await waitFor(() => !alive(leader)), 'the recipe leader did not exit before the outside signal');
    assert.equal(alive(readPid(resolve(fx.scratchDir, 'leaked'))), true, 'the leaked child still holds the output open');
    const tap = [...snapshot().values()].find(stat => stat.ppid === child.pid);
    assert.ok(tap, 'the runner had no gate-tap child while the leaked child held stdout open');
    process.kill(tap.pid, 'SIGTERM');
    rmSync(resolve(fx.scratchDir, 'hold-output'), { force: true });
    const result = await new Promise(resolveDone => child.once('close', (code, signal) => resolveDone({ code, signal })));
    const stdout = stdoutCapture.read();
    const stderr = stderrCapture.read();

    assert.deepEqual(result, { code: 1, signal: null }, stderr);
    assert.match(stdout, /evidence: recorded/);
    assert.doesNotMatch(stdout + stderr, /stopped from outside .*no verdict/);
    assert.match(stderr, /an outside SIGTERM arrived after the recipe exited with code 1; its verdict stands/);
    assert.match(readFileSync(fx.calls, 'utf8'), /build put/, 'the failed closure evidence is recorded');
    const runPath = /live failures: .* --run (.+)\n/.exec(stdout)?.[1];
    assert.ok(runPath, stdout);
    const events = readRun(runPath).events;
    assert.equal(events.filter(event => event.kind === 'gate-stopped').length, 0);
    assert.equal(events.filter(event => event.kind === 'gate-verdict').at(-1).verdict.exit, 1);
  });

  test('outside TERM after a signal-killed leader keeps the red verdict and evidence', { timeout: 60_000 }, async t => {
    const fx = outsideSignalFixture(t, { recipe: ({ scratchDir }) => [
      '#!/usr/bin/env bash',
      `echo $$ > "${scratchDir}/leader"`,
      `touch "${scratchDir}/hold-output"`,
      `while [ -e "${scratchDir}/hold-output" ]; do sleep 0.05; done &`,
      `echo $! > "${scratchDir}/leaked"`,
      'echo "test shed_failure ... FAILED"',
      'printf ready > "$OUTSIDE_READY"',
      'wait',
    ].join('\n') });
    t.after(() => rmSync(resolve(fx.scratchDir, 'hold-output'), { force: true }));
    const cli = resolve(HERE, 'gate-runner.mjs');
    const stdoutCapture = outputCapture('gate-outside-shed-stdout-');
    const stderrCapture = outputCapture('gate-outside-shed-stderr-');
    const child = spawn(process.execPath, [cli, '--target', 'fixture'], {
      cwd: fx.root,
      env: fx.env,
      stdio: ['ignore', stdoutCapture.fd, stderrCapture.fd],
    });
    stdoutCapture.close();
    stderrCapture.close();
    t.after(() => { try { child.kill('SIGTERM'); } catch { /* gone */ } });
    assert.ok(await waitFor(() => existsSync(fx.ready)), 'the fixture gate did not reach its failure');
    const leader = readPid(resolve(fx.scratchDir, 'leader'));
    process.kill(leader, 'SIGTERM');
    assert.ok(await waitFor(() => !alive(leader)), 'the recipe leader did not die before the outside signal');
    assert.equal(alive(readPid(resolve(fx.scratchDir, 'leaked'))), true, 'the leaked child still holds the output open');
    const tap = [...snapshot().values()].find(stat => stat.ppid === child.pid);
    assert.ok(tap, 'the runner had no gate-tap child while the leaked child held stdout open');
    process.kill(tap.pid, 'SIGTERM');
    rmSync(resolve(fx.scratchDir, 'hold-output'), { force: true });
    const result = await new Promise(resolveDone => child.once('close', (code, signal) => resolveDone({ code, signal })));
    const stdout = stdoutCapture.read();
    const stderr = stderrCapture.read();

    assert.deepEqual(result, { code: 1, signal: null }, stderr);
    assert.match(stdout, /evidence: recorded/);
    assert.doesNotMatch(stdout + stderr, /stopped from outside .*no verdict/);
    assert.match(stderr, /an outside SIGTERM arrived after the recipe died of SIGTERM; its verdict stands/);
    assert.match(readFileSync(fx.calls, 'utf8'), /build put/, 'the shed closure evidence is recorded');
    const runPath = /live failures: .* --run (.+)\n/.exec(stdout)?.[1];
    assert.ok(runPath, stdout);
    const events = readRun(runPath).events;
    assert.equal(events.filter(event => event.kind === 'gate-stopped').length, 0);
    assert.equal(events.filter(event => event.kind === 'gate-verdict').at(-1).verdict.exit, 1);
  });

  test('TERM delivered only to the recipe leader keeps the red verdict and evidence', { timeout: 60_000 }, t => {
    const fx = evidenceFixture(t, 'gate-child-term-evidence-', {
      recipe: () => ['#!/usr/bin/env bash', 'kill -TERM $$'].join('\n'),
    });
    let output = '';
    const status = runProject(fx.project, false, false, { root: fx.root, spawn: fx.spawnFake, tap: true, env: fx.env,
      readingCollector: [], write: line => { output += line; }, warn: line => { output += line; } });

    assert.equal(status, 1, output);
    assert.equal(fx.calls.puts, 1, 'the recipe-only TERM records the red it reached');
    const events = readRun(fx.run).events;
    assert.equal(events.filter(event => event.kind === 'gate-stopped').length, 0);
    assert.equal(events.filter(event => event.kind === 'gate-verdict').at(-1).verdict.exit, 1);
    assert.match(output, /evidence: recorded/);
  });

  test('stop --now: no put, no verdict note, no cycle observation, no reading; the close is the stopped line', { timeout: 60_000 }, async t => {
    const fx = evidenceFixture(t, 'gate-stop-evidence-', { seconds: 300 });
    const stopper = stopperFor(fx.run, fx.pids, { immediate: true });
    t.after(() => stopper.kill());
    const readings = [];
    let output = '';
    const started = Date.now();
    const status = runProject(fx.project, false, false, { root: fx.root, spawn: fx.spawnFake, tap: true, env: fx.env,
      readingCollector: readings, write: line => { output += line; }, warn: line => { output += line; } });
    const tookMs = Date.now() - started;

    assert.equal(status, GATE_STOPPED_EXIT);
    assert.notEqual(status, 0);
    assert.ok(tookMs < 30_000, `the gate ended in ${tookMs} ms, not after its 300 s`);
    assert.equal(fx.calls.gate, 1, 'the gate ran once and was not retried');
    for (const name of ['leader', 'child', 'grandchild']) assert.equal(alive(readPid(resolve(fx.pids, name))), false, name);
    assert.equal(fx.calls.puts, 0, 'no closure evidence put');
    assert.deepEqual(fx.calls.epr, [], 'no gate-verdict@1 note and no cycle observation');
    assert.deepEqual(readings, [], 'no gate or step reading row');
    assert.match(output, /\[gate\] fixture stopped on request by session-integrator: its process group was ended \(SIGTERM\); no verdict recorded \(what it already wrote or put is not undone\)/);
    assert.doesNotMatch(output, /evidence: recorded/);

    const events = readRun(fx.run).events;
    assert.equal(events.filter(event => event.kind === 'gate-verdict').length, 0, 'no live verdict');
    assert.equal(events.filter(event => event.kind === 'failure').length, 0, 'no red from the stop');
    const stopped = events.find(event => event.kind === 'gate-stopped');
    assert.deepEqual({ project: stopped.project, by: stopped.by, signal: stopped.signal, evidence: stopped.evidence },
      { project: 'fixture', by: 'session-integrator', signal: 'SIGTERM', evidence: 'none' });

    // The hook (or just gate) closes the run as failed; a stopped gate makes it interrupted, and
    // the close is the run's last line: who stopped which gate, and that nothing was recorded.
    closeRun(fx.run, 'failed');
    const last = readRun(fx.run).events.at(-1);
    assert.equal(last.kind, 'run-close');
    assert.equal(last.outcome, 'interrupted');
    assert.equal(formatEvent(last),
      '[gate-live] run interrupted: fixture was stopped on request by session-integrator; no verdict was recorded for it, and the push is refused');
    const cli = spawnSync(process.execPath, [LIVE, 'stopped', '--run', fx.run, '--gate', 'fixture'], { encoding: 'utf8' });
    assert.equal(cli.status, 0, 'the hook reads the gate as stopped');
  });

  test('a plain stop waits for the running gate, whose verdict is recorded as before', { timeout: 60_000 }, async t => {
    const fx = evidenceFixture(t, 'gate-stop-plain-evidence-', { seconds: 1 });
    const stopper = stopperFor(fx.run, fx.pids, { immediate: false });
    t.after(() => stopper.kill());
    let output = '';
    const status = runProject(fx.project, false, false, { root: fx.root, spawn: fx.spawnFake, tap: true, env: fx.env,
      readingCollector: [], write: line => { output += line; }, warn: line => { output += line; } });
    assert.equal(status, 0, output);
    assert.equal(fx.calls.puts, 1, 'the finished gate records its evidence');
    assert.equal(readRun(fx.run).events.filter(event => event.kind === 'gate-stopped').length, 0);
    assert.ok(readRun(fx.run).events.some(event => event.kind === 'stop-request'), 'the stop was asked mid-gate');
  });
});

// A recipe that ends on its own with `status` while a background sleep keeps its output open for
// a while: the stop arrives in that teardown, after the leader exited and before the tap closes.
const endsOnItsOwn = status => pids => [
  '#!/usr/bin/env bash',
  'sleep 1.5 &',
  'echo "test long_gate ... started"',
  `touch "${pids}/leader-done"`,
  `exit ${status}`,
].join('\n');

describe('a verdict the gate reached on its own survives a stop', () => {
  for (const [status, name] of [[1, 'red'], [0, 'green']]) {
    test(`the gate exits ${status} on its own, a stop arrives in its teardown: the ${name} is recorded`, async t => {
      const fx = evidenceFixture(t, `gate-stop-own-${name}-`, { recipe: endsOnItsOwn(status) });
      const stopper = stopperFor(fx.run, fx.pids, { immediate: true, marker: 'leader-done', afterMs: 300 });
      t.after(() => stopper.kill());
      let output = '';
      const result = runProject(fx.project, false, false, { root: fx.root, spawn: fx.spawnFake, tap: true, env: fx.env,
        readingCollector: [], write: line => { output += line; }, warn: line => { output += line; } });
      assert.equal(result, status, output);
      assert.equal(fx.calls.puts, 1, `the ${name} verdict is recorded`);
      const events = readRun(fx.run).events;
      assert.ok(events.some(event => event.kind === 'stop-request' && event.immediate), 'the stop was asked in the teardown');
      assert.equal(events.filter(event => event.kind === 'gate-stopped').length, 0, 'nothing reads as stopped');
      assert.equal(events.filter(event => event.kind === 'gate-verdict').at(-1).verdict.exit, status);
      assert.doesNotMatch(output, /stopped on request/);
      // The stop still refuses the push: the hook's last check reads it (prepush-gate-live.test.mjs).
      const cli = spawnSync(process.execPath, [LIVE, 'stopped', '--run', fx.run], { encoding: 'utf8' });
      assert.equal(cli.status, 0);
    });
  }
});

describe('the stop is decided from the leader\'s own exit', () => {
  test('a gate that exits 1 from its own TERM handler keeps its red', { timeout: 60_000 }, async t => {
    const fx = evidenceFixture(t, 'gate-stop-race-trap-', { recipe: pids => [
      '#!/usr/bin/env bash',
      "trap 'exit 1' TERM",
      `touch "${pids}/ready"`,
      'sleep 300 &',
      'wait',
    ].join('\n') });
    const stopper = stopperFor(fx.run, fx.pids, { immediate: true, marker: 'ready', afterMs: 200 });
    t.after(() => stopper.kill());
    let output = '';
    const status = runProject(fx.project, false, false, { root: fx.root, spawn: fx.spawnFake, tap: true, env: fx.env,
      readingCollector: [], write: line => { output += line; }, warn: line => { output += line; } });
    assert.equal(status, 1, output);
    assert.equal(fx.calls.puts, 1, 'the red the gate reached is recorded');
    const events = readRun(fx.run).events;
    assert.ok(events.some(event => event.kind === 'stop-request' && event.immediate));
    assert.equal(events.filter(event => event.kind === 'gate-stopped').length, 0);
    assert.equal(events.filter(event => event.kind === 'gate-verdict').at(-1).verdict.exit, 1);
  });

  test('endTree on a leader that already exited sends the leader nothing', { timeout: 60_000 }, async () => {
    const gone = spawn('sleep', ['0.1'], { detached: true, stdio: 'ignore' });
    await waitFor(() => readStat(gone.pid));
    const recorded = readStat(gone.pid);
    await new Promise(res => gone.on('exit', res));
    const sent = [];
    const result = await endTree(recorded, { graceMs: 100, kill: (pid, signal) => sent.push([pid, signal]) });
    assert.deepEqual(result.toLeader, []);
    assert.deepEqual(sent.filter(([pid]) => pid === recorded.pid), [], 'never the leader pid');
  });

  test('stoppedBy matches the signal sent to the leader, and only when it came before the exit', { timeout: 60_000 }, () => {
    const term = [{ signal: 'SIGTERM', at: 10 }];
    assert.equal(stoppedBy({ code: null, signal: 'SIGTERM', exitAt: 11 }, term), 'SIGTERM');
    assert.equal(stoppedBy({ code: 143, signal: null, exitAt: 11 }, term), 'SIGTERM', 'just reports a TERM death as 143');
    assert.equal(stoppedBy({ code: null, signal: 'SIGKILL', exitAt: 11 }, term), null, 'SIGKILL when only TERM was sent');
    assert.equal(stoppedBy({ code: 137, signal: null, exitAt: 11 }, term), null, '137 when only TERM was sent');
    assert.equal(stoppedBy({ code: 143, signal: null, exitAt: 9 }, term), null, '143 before the delivery');
    assert.equal(stoppedBy({ code: null, signal: 'SIGTERM', exitAt: 9 }, term), null, 'a TERM death before the delivery');
    assert.equal(stoppedBy({ code: 1, signal: null, exitAt: 11 }, term), null, 'an exit code is the gate\'s own verdict');
    assert.equal(stoppedBy({ code: 0, signal: null, exitAt: 11 }, term), null);
    assert.equal(stoppedBy({ code: 137, signal: null, exitAt: 30 }, [...term, { signal: 'SIGKILL', at: 20 }]), 'SIGKILL');
    assert.equal(stoppedBy({ code: null, signal: 'SIGTERM', exitAt: 11 }, []), null, 'nothing sent to the leader, nothing stopped');
  });
});

// The runner matrix: no stop / a stop while the recipe runs / a stop after it ended on its own,
// times exit 0, 1, 143, SIGTERM and SIGKILL. Evidence is withheld only where the recipe's leader
// died of the signal the stop sent it (a TERM death, or 143 after the TERM); everything else
// records what an unstopped run records.
const OUTCOMES = {
  'exit 0': { end: 'exit 0', status: 0 },
  'exit 1': { end: 'exit 1', status: 1 },
  'exit 143': { end: 'exit 143', status: 143 },
  SIGTERM: { end: 'kill -TERM $$', status: 1 },
  SIGKILL: { end: 'kill -KILL $$', status: 1 },
};
const MATRIX = [];
for (const when of ['no stop', 'stop before', 'stop after']) {
  for (const [outcome, { end, status }] of Object.entries(OUTCOMES)) {
    const stopped = when === 'stop before' && (outcome === 'exit 143' || outcome === 'SIGTERM');
    MATRIX.push({ when, outcome, end, status: stopped ? GATE_STOPPED_EXIT : status, evidence: !stopped });
  }
}
const matrixRecipe = ({ when, outcome, end }) => pids => {
  if (when === 'no stop') return ['#!/usr/bin/env bash', end].join('\n');
  if (when === 'stop after') return ['#!/usr/bin/env bash', 'sleep 1.5 &', `touch "${pids}/marker"`, end].join('\n');
  // A stop while it runs: the recipe meets the stop's TERM with the outcome (SIGTERM: it simply dies of it).
  return ['#!/usr/bin/env bash', ...(outcome === 'SIGTERM' ? [] : [`trap '${end}' TERM`]),
    `touch "${pids}/marker"`, 'sleep 300 &', 'wait'].join('\n');
};

describe('runner matrix: a verdict is withheld only for a leader the stop itself ended', () => {
  for (const cell of MATRIX) {
    test(`${cell.when} × ${cell.outcome}: evidence ${cell.evidence ? 'written' : 'withheld'}`, async t => {
      const fx = evidenceFixture(t, 'gate-stop-matrix-', { recipe: matrixRecipe(cell) });
      if (cell.when !== 'no stop') {
        const stopper = stopperFor(fx.run, fx.pids, { immediate: true, marker: 'marker', afterMs: cell.when === 'stop after' ? 300 : 100 });
        t.after(() => stopper.kill());
      }
      let output = '';
      const status = runProject(fx.project, false, false, { root: fx.root, spawn: fx.spawnFake, tap: true, env: fx.env,
        readingCollector: [], write: line => { output += line; }, warn: line => { output += line; } });
      assert.equal(fx.calls.puts, cell.evidence ? 1 : 0, output);
      assert.equal(status, cell.status, output);
      const stoppedEvents = readRun(fx.run).events.filter(event => event.kind === 'gate-stopped');
      assert.equal(stoppedEvents.length, cell.evidence ? 0 : 1);
    });
  }
  test('the matrix has fifteen cells', { timeout: 60_000 }, () => assert.equal(MATRIX.length, 15));
});

describe('an escaped child does not hang the run', () => {
  test('a green whose output was cut after its exit is recorded, and says its output was cut', { timeout: 60_000 }, async t => {
    const fx = evidenceFixture(t, 'gate-stop-cut-', { recipe: pids => [
      '#!/usr/bin/env bash',
      `( setsid bash -c 'echo $$ > "${pids}/escaped"; exec sleep 30' & )`,
      `while [ ! -s "${pids}/escaped" ]; do sleep 0.05; done`,
      `touch "${pids}/marker"`,
      'exit 0',
    ].join('\n') });
    t.after(() => { try { process.kill(readPid(resolve(fx.pids, 'escaped')), 'SIGKILL'); } catch { /* gone */ } });
    const stopper = stopperFor(fx.run, fx.pids, { immediate: true, marker: 'marker', afterMs: 300 });
    t.after(() => stopper.kill());
    let output = '';
    const status = runProject(fx.project, false, false, { root: fx.root, spawn: fx.spawnFake, tap: true, env: fx.env,
      readingCollector: [], write: line => { output += line; }, warn: line => { output += line; } });
    assert.equal(status, 0, output);
    assert.equal(fx.calls.puts, 1, 'the green stands');
    assert.match(output, /its output was cut after the recipe exited .*the verdict is the recipe's own exit/);
  });

  test('the leader ended on its own and a stray holds its output: the verdict stands and the wait is bounded', { timeout: 60_000 }, async t => {
    const dir = scratch(t, 'gate-stop-stray-');
    const script = resolve(dir, 'gate.sh');
    writeFileSync(script, [
      '#!/usr/bin/env bash',
      `echo $$ > "${dir}/leader"`,
      `( setsid bash -c 'echo $$ > "${dir}/escaped"; exec sleep 30' & )`,
      `while [ ! -s "${dir}/escaped" ]; do sleep 0.05; done`,
      `touch "${dir}/leader-done"`,
      'exit 1',
    ].join('\n'), { mode: 0o755 });
    t.after(() => { try { process.kill(readPid(resolve(dir, 'escaped')), 'SIGKILL'); } catch { /* gone */ } });
    const { path: run } = openRun({ root: dir, sha: '4'.repeat(40), target: 'test' });
    const { done } = startTap(run, script);
    assert.ok(await waitFor(() => existsSync(resolve(dir, 'leader-done'))));
    await waitFor(() => !alive(readPid(resolve(dir, 'leader'))));
    const escaped = readPid(resolve(dir, 'escaped'));
    const asked = Date.now();
    requestStop(run, { immediate: true, by: 'session-stopper' });
    const result = await done;
    assert.ok(Date.now() - asked < 16_000, 'the wait for the stray is bounded with loaded-host margin');
    assert.equal(result.code, 1, 'the leader\'s own red is the tap\'s exit');
    assert.equal(result.summary.stopped, undefined, 'not stopped: the recipe ended on its own');
    assert.match(result.stderr, new RegExp(`holds its output: pid ${escaped} \\([^)]*sleep [^)]*30\\)`));
    assert.match(result.stderr, /the recipe ended on its own \(exit 1\); its verdict stands, and the push is refused/);
    assert.equal(alive(escaped), true, 'never signalled');
  });

  test('a double-forked holder of the output is named, and the tap stops waiting for it', { timeout: 60_000 }, async t => {
    const dir = scratch(t, 'gate-stop-escaped-');
    const script = resolve(dir, 'gate.sh');
    writeFileSync(script, [
      '#!/usr/bin/env bash',
      `( setsid bash -c 'echo $$ > "${dir}/escaped"; exec sleep 30' & )`,
      `echo $$ > "${dir}/leader"`,
      'sleep 300',
    ].join('\n'), { mode: 0o755 });
    t.after(() => { try { process.kill(readPid(resolve(dir, 'escaped')), 'SIGKILL'); } catch { /* gone */ } });
    const { path: run } = openRun({ root: dir, sha: '2'.repeat(40), target: 'test' });
    const { done } = startTap(run, script, { GATE_STOP_GRACE_MS: '500' });
    assert.ok(await waitFor(() => existsSync(resolve(dir, 'escaped')) && existsSync(resolve(dir, 'leader'))));
    const escaped = readPid(resolve(dir, 'escaped'));
    await waitFor(() => readStat(escaped)?.sid === escaped); // its own session: outside the gate's
    const asked = Date.now();
    requestStop(run, { immediate: true, by: 'session-stopper' });
    const result = await done;
    const tookMs = Date.now() - asked;
    assert.ok(tookMs < 28_000, `the tap ended in ${tookMs} ms although pid ${escaped} still holds its output`);
    assert.equal(alive(escaped), true, 'the escaped process is outside the tree and never signalled');
    assert.match(result.stderr, new RegExp(`a process outside the gate's tree may still be running and holds its output: pid ${escaped} \\([^)]*sleep [^)]*30\\)`));
    assert.equal(result.summary.stopped.signal, 'SIGTERM');
    assert.deepEqual(result.summary.stopped.escaped.map(holder => holder.pid), [escaped]);
  });
});

describe('Ctrl-C still reaches the detached gate', () => {
  test('SIGINT to the tap ends the gate group and reports an outside stop', { timeout: 60_000 }, async t => {
    const dir = scratch(t, 'gate-stop-int-');
    const script = resolve(dir, 'gate.sh');
    // Foreground generations, as a recipe runs cargo: SIGINT reaches each through the group.
    writeFileSync(script, [
      '#!/usr/bin/env bash',
      `echo $$ > "${dir}/leader"`,
      `bash -c 'echo $$ > "${dir}/child"; bash -c "echo \\$\\$ > ${dir}/grandchild; exec sleep 300"; true'`,
      'true',
    ].join('\n'), { mode: 0o755 });
    const { path: run } = openRun({ root: dir, sha: '3'.repeat(40), target: 'test' });
    const { child, done } = startTap(run, script);
    assert.ok(await waitFor(() => existsSync(resolve(dir, 'grandchild'))));
    const pids = ['leader', 'child', 'grandchild'].map(name => readPid(resolve(dir, name)));
    await waitFor(() => { try { return readFileSync(`/proc/${pids[2]}/cmdline`, 'utf8').startsWith('sleep'); } catch { return false; } });
    child.kill('SIGINT');
    const result = await done;
    for (const pid of pids) assert.equal(alive(pid), false, `pid ${pid} is gone`);
    assert.ok(result.code !== 0 || result.signal !== null);
    assert.deepEqual(result.summary?.stopped, { by: null, signal: 'SIGINT', outside: true });
  });

  test('SIGINT at the gate\'s first line, before the tap has settled, still ends the gate', { timeout: 60_000 }, async t => {
    // The tap used to install its signal handlers after spawning (and after a full /proc scan for
    // its cost sampler): an interrupt in that window killed the tap and orphaned the detached gate.
    for (let round = 0; round < 5; round += 1) {
      const dir = scratch(t, 'gate-stop-int-early-');
      const script = resolve(dir, 'gate.sh');
      writeFileSync(script, ['#!/usr/bin/env bash', `echo $$ > "${dir}/leader"`, 'sleep 300', 'true'].join('\n'), { mode: 0o755 });
      const { path: run } = openRun({ root: dir, sha: '9'.repeat(40), target: 'test' });
      const { child, done } = startTap(run, script);
      // As soon as the gate's first line ran: polled every millisecond, inside the tap's start-up.
      const until = Date.now() + 10_000;
      while (!(existsSync(resolve(dir, 'leader')) && readFileSync(resolve(dir, 'leader'), 'utf8').trim()) && Date.now() < until) await delay(1);
      child.kill('SIGINT');
      await done;
      const leader = readPid(resolve(dir, 'leader'));
      const orphaned = await waitFor(() => !alive(leader), 15_000) ? null : readStat(leader);
      if (orphaned) { try { process.kill(-leader, 'SIGKILL'); } catch { /* gone */ } }
      assert.equal(orphaned, null, `round ${round}: the gate's leader survived the tap: ${JSON.stringify(orphaned)}`);
    }
  });
});

// The real stop verb, run asynchronously so the test can act on the run while it waits.
function startCli(run, args = []) {
  const capture = outputCapture('gate-stop-live-cli-');
  const child = spawn(process.execPath, [LIVE, 'stop', ...args, '--run', run],
    { stdio: ['ignore', capture.fd, capture.fd], env: nodeCliEnv({ EPR_BIN: '/nonexistent/epr', ELOHIM_SESSION_ID: 'session-cli' }) });
  capture.close();
  return new Promise(res => child.on('close', code => res({ code, out: capture.read() })));
}
async function requestOf(run) {
  await waitFor(() => readRun(run).events.some(event => event.kind === 'stop-request'));
  return readRun(run).events.filter(event => event.kind === 'stop-request').at(-1);
}

describe('the stop CLI', () => {
  test('a stop for a push run that already passed says the push is proceeding, and exits 1', { timeout: 60_000 }, t => {
    const root = scratch(t, 'gate-stop-passed-');
    const { path: run } = openRun({ root, sha: 'e'.repeat(40), target: 'refs/heads/dev' });
    closeRun(run, 'gates-passed');
    const before = readFileSync(run, 'utf8');
    const result = spawnNodeSync([LIVE, 'stop', '--now', '--run', run], { env: nodeCliEnv({ EPR_BIN: '/nonexistent/epr' }) });
    assert.equal(result.status, 1);
    assert.match(result.stdout, /this run has passed its last stop point; the push is proceeding/);
    assert.equal(readFileSync(run, 'utf8'), before, 'nothing appended');
  });

  test('a stop against a run that looks abandoned stops nothing, says why, and exits 1', { timeout: 60_000 }, t => {
    const root = scratch(t, 'gate-stop-abandoned-');
    const { path: run } = openRun({ root, sha: '6'.repeat(40), target: 'refs/heads/dev' });
    const old = new Date(Date.now() - 7 * 3600 * 1000);
    utimesSync(run, old, old);
    const before = readFileSync(run, 'utf8');
    const result = spawnNodeSync([LIVE, 'stop', '--now', '--wait', '1', '--run', run], { env: nodeCliEnv({ EPR_BIN: '/nonexistent/epr' }) });
    assert.equal(result.status, 1);
    assert.match(result.stdout, /nothing was stopped: the run looks abandoned \(no record for 7 hours, never closed\)/);
    assert.equal(readFileSync(run, 'utf8'), before, 'nothing appended');
  });

  test('a stop for a run that already ended is a no-op with a clear line', { timeout: 60_000 }, t => {
    const root = scratch(t, 'gate-stop-ended-');
    const { path: run } = openRun({ root, sha: 'f'.repeat(40), target: 'test' });
    closeRun(run, 'failed');
    const before = readFileSync(run, 'utf8');
    for (const args of [['stop', '--now', '--run', run], ['stop', '--run', run]]) {
      const result = spawnNodeSync([LIVE, ...args], { env: nodeCliEnv({ EPR_BIN: '/nonexistent/epr' }) });
      assert.equal(result.status, 1, 'exit 0 means this request stopped this run, and nothing was requested');
      assert.match(result.stdout, /nothing was stopped: the run already ended \(failed\); nothing appended/);
    }
    assert.equal(readFileSync(run, 'utf8'), before, 'nothing was appended to the closed run');
  });

  test('a stop the run never answers is unconfirmed: exit 2, and the push may be proceeding', { timeout: 60_000 }, t => {
    const root = scratch(t, 'gate-stop-idle-');
    const { path: run } = openRun({ root, sha: '1'.repeat(40), target: 'test' });
    const result = spawnNodeSync([LIVE, 'stop', '--now', '--wait', '1', '--run', run],
      { env: nodeCliEnv({ EPR_BIN: '/nonexistent/epr', ELOHIM_SESSION_ID: 'session-cli' }), timeout: 10_000 });
    assert.equal(result.status, 2, result.stdout + result.stderr);
    assert.match(result.stdout, /stop requested but not confirmed by the run within 1 s; the push may be proceeding/);
    const request = readRun(run).events.find(event => event.kind === 'stop-request');
    assert.equal(request.immediate, true);
    assert.equal(request.by, 'session-cli');
    assert.match(formatEvent(request), /stop requested now by session-cli — a gate still running is ended and records no verdict; .* the push is refused/);
  });

  test('stop --now on a running gate exits 0 only once the run\'s stopped close exists', { timeout: 60_000 }, async t => {
    const fx = evidenceFixture(t, 'gate-stop-cli-now-', { seconds: 300 });
    const exitFile = resolve(fx.pids, 'cli-exit');
    const cli = spawn('bash', ['-c', `until [ -f "${resolve(fx.pids, 'grandchild')}" ]; do sleep 0.05; done; `
      + `"${process.execPath}" "${LIVE}" stop --now --wait 20 --run "${fx.run}" > "${resolve(fx.pids, 'cli-out')}"; echo $? > "${exitFile}"`],
      { stdio: 'ignore', env: nodeCliEnv({ EPR_BIN: '/nonexistent/epr', ELOHIM_SESSION_ID: 'session-cli' }) });
    t.after(() => cli.kill());
    const status = runProject(fx.project, false, false, { root: fx.root, spawn: fx.spawnFake, tap: true, env: fx.env,
      readingCollector: [], write: () => {}, warn: () => {} });
    assert.equal(status, GATE_STOPPED_EXIT);
    assert.ok(readRun(fx.run).events.some(event => event.kind === 'gate-stopped'));
    assert.equal(spawnSync('kill', ['-0', String(cli.pid)]).status, 0,
      'the stop CLI is still waiting for the run close');
    assert.equal(existsSync(exitFile), false, 'no answer yet: the gate is stopped, but the run has not closed');
    closeRun(fx.run, 'failed');
    assert.ok(await waitFor(() => existsSync(exitFile) && readFileSync(exitFile, 'utf8').trim() !== ''));
    assert.equal(readFileSync(exitFile, 'utf8').trim(), '0');
    assert.match(readFileSync(resolve(fx.pids, 'cli-out'), 'utf8'), /fixture stopped by this request \(SIGTERM\); no verdict recorded; the run closed interrupted and the push is refused/);
  });

  test('a stop that ties the stop-point mark is after it, through the real stop verb', { timeout: 60_000 }, async t => {
    const root = scratch(t, 'gate-stop-tie-');
    const { path: run } = openRun({ root, sha: '5'.repeat(40), target: 'refs/heads/dev' });
    const cli = startCli(run, ['--wait', '5']);
    const request = await requestOf(run);
    // An unlocked writer's mark landing with the request's own sequence number.
    appendFileSync(run, `${JSON.stringify({ kind: 'stop-boundary', v: 1, run: 'x', seq: request.seq, at: request.at })}\n`);
    const result = await cli;
    assert.equal(result.code, 1, result.out);
    assert.match(result.out, /this run has passed its last stop point; the push is proceeding/);
    assert.equal(stopRequested(run), false, 'the hook\'s reader does not count it either');
  });

  test('stop A pending, the mark, then stop B: A is confirmed by the record naming it, B is told another request stopped the run', { timeout: 60_000 }, async t => {
    const root = scratch(t, 'gate-stop-ab-');
    const { path: run } = openRun({ root, sha: '7'.repeat(40), target: 'refs/heads/dev' });
    const a = startCli(run, ['--wait', '10']);
    const requestA = await requestOf(run);
    assert.equal(spawnSync(process.execPath, [LIVE, 'boundary', '--run', run]).status, 0, 'the hook writes its mark');
    const b = startCli(run, ['--wait', '10']);
    await waitFor(() => readRun(run).events.filter(event => event.kind === 'stop-request').length === 2);
    // The hook's read of the stops before its mark, and its refusal: the record names A alone.
    assert.equal(spawnSync(process.execPath, [LIVE, 'honour', '--run', run, '--where', 'stop-point']).status, 0);
    const honoured = readRun(run).events.find(event => event.kind === 'stop-honoured');
    assert.deepEqual(honoured.requests, [requestA.id]);
    const [resultA, resultB] = await Promise.all([a, b]);
    assert.equal(resultA.code, 0, resultA.out);
    assert.match(resultA.out, /the run took this stop \(stop-point\)/);
    assert.equal(resultB.code, 1, resultB.out);
    assert.match(resultB.out, /this run has passed its last stop point; it was already stopped by another request/);
  });

  test('a stopped record that names another request never confirms this one', { timeout: 60_000 }, async t => {
    const root = scratch(t, 'gate-stop-other-');
    const { path: run } = openRun({ root, sha: '8'.repeat(40), target: 'refs/heads/dev' });
    appendEvent(run, { kind: 'stop-honoured', where: 'between-gates', requests: ['ffffffffffffffff'] });
    const result = await startCli(run, ['--wait', '1']);
    assert.notEqual(result.code, 0, result.out);
  });

});
