// A requested stop ends the gate that is RUNNING (`gate-live.mjs stop --now`) and records nothing
// for it; a plain stop still lets the running gate finish. Habit gate-failures-arrive-live.

import { describe, test } from 'node:test';
import { strict as assert } from 'node:assert';
import { spawn, spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { setTimeout as delay } from 'node:timers/promises';

import { closeRun, formatEvent, openRun, readRun, requestStop } from './gate-live.mjs';
import { GATE_STOPPED_EXIT, runProject } from './gate-runner.mjs';
import { endTree, gateTree, readStat, snapshot, stopGraceMs } from './gate-stop.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const TAP = resolve(HERE, 'gate-tap.mjs');
const LIVE = resolve(HERE, 'gate-live.mjs');
delete process.env.GATE_LIVE_REF;
delete process.env.GATE_LIVE_RUN;
// No berth: a closed fixture run must not send a sibling final.
process.env.GATE_LIVE_SIBLING = 'off';

const alive = pid => {
  const stat = readStat(pid);
  return Boolean(stat) && stat.state !== 'Z' && stat.state !== 'X';
};
const readPid = path => Number(readFileSync(path, 'utf8').trim());

async function waitFor(predicate, ms = 10_000) {
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
  const child = spawn(process.execPath, [TAP, '--project', 'fixture', '--run', run, '--', 'bash', script], {
    stdio: ['ignore', 'pipe', 'pipe', 'pipe'],
    env: { ...process.env, GATE_STOP_GRACE_MS: '1500', ...env },
  });
  let fd3 = '';
  let stderr = '';
  child.stdio[3].on('data', chunk => { fd3 += chunk; });
  child.stderr.on('data', chunk => { stderr += chunk; });
  child.stdout.resume();
  const done = new Promise(res => child.on('close', (code, signal) => res({
    code, signal, stderr, summary: fd3.trim() ? JSON.parse(fd3.trim().split('\n').pop()) : null,
  })));
  return { child, done };
}

describe('stop --now ends the running gate', () => {
  test('TERM ends the leader, its child and its grandchild within the grace', async t => {
    const dir = scratch(t, 'gate-stop-term-');
    const { path: run } = openRun({ root: dir, sha: 'a'.repeat(40), target: 'test' });
    const { done } = startTap(run, fakeGate(dir));
    assert.ok(await waitFor(() => existsSync(resolve(dir, 'grandchild'))), 'the fake gate started its tree');
    const pids = ['leader', 'child', 'grandchild'].map(name => readPid(resolve(dir, name)));
    assert.ok(pids.every(alive), 'all three run before the stop');

    const asked = Date.now();
    requestStop(run, { immediate: true, by: 'session-stopper', reason: 'pre-gate red' });
    const result = await done;
    const tookMs = Date.now() - asked;

    assert.ok(tookMs < 1500 + 2000, `ended in ${tookMs} ms, inside the grace`);
    for (const pid of pids) assert.equal(alive(pid), false, `pid ${pid} is gone`);
    assert.deepEqual(result.summary.stopped, { by: 'session-stopper', reason: 'pre-gate red', signal: 'SIGTERM' });
    assert.ok(result.code !== 0 || result.signal !== null, 'the tap does not exit 0');
    assert.match(result.stderr, /stop requested now by session-stopper; ending the gate's process group/);
    // The gate's own death throes are the stop, not reds: nothing was appended as a failure.
    assert.equal(readRun(run).events.filter(event => event.kind === 'failure').length, 0);
  });

  test('a grandchild that ignores TERM is KILLed once the grace is over', async t => {
    const dir = scratch(t, 'gate-stop-kill-');
    const { path: run } = openRun({ root: dir, sha: 'b'.repeat(40), target: 'test' });
    const { done } = startTap(run, fakeGate(dir, { stubborn: true }), { GATE_STOP_GRACE_MS: '600' });
    assert.ok(await waitFor(() => existsSync(resolve(dir, 'grandchild'))));
    const grandchild = readPid(resolve(dir, 'grandchild'));
    await waitFor(() => readFileSync(`/proc/${grandchild}/cmdline`, 'utf8').startsWith('sleep'));
    requestStop(run, { immediate: true, by: 'session-stopper' });
    const result = await done;
    assert.equal(alive(grandchild), false);
    assert.equal(result.summary.stopped.signal, 'SIGKILL');
  });

  test('a plain stop leaves the running gate to finish', async t => {
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
  test('a recorded pid in another process group is not signalled', async t => {
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

  test('a leader whose pid was reused (another start time) gives no custody and no signal', async t => {
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

  test('the grace is bounded', () => {
    assert.equal(stopGraceMs({}), 10_000);
    assert.equal(stopGraceMs({ GATE_STOP_GRACE_MS: '250' }), 250);
    assert.equal(stopGraceMs({ GATE_STOP_GRACE_MS: '999999' }), 60_000);
    assert.equal(stopGraceMs({ GATE_STOP_GRACE_MS: 'soon' }), 10_000);
  });
});

// runProject with its evidence path live: a fake brit answers the closure lookups and counts every
// put, so a stopped gate that recorded anything would show here.
function evidenceFixture(t, prefix, { seconds }) {
  const root = scratch(t, prefix);
  const pids = resolve(root, 'pids');
  mkdirSync(pids);
  mkdirSync(resolve(root, 'genesis/orchestrator'), { recursive: true });
  // The recipe the tap runs is this fixture's run-local-gate.sh: a long gate with a child and a grandchild.
  writeFileSync(resolve(root, 'genesis/orchestrator/run-local-gate.sh'),
    readFileSync(fakeGate(pids, { seconds }), 'utf8'));
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

// The stop comes from another process, as the integrator's does: runProject blocks this one.
function stopperFor(run, pids, { immediate }) {
  const code = `
    import { existsSync } from 'node:fs';
    import { requestStop } from ${JSON.stringify(LIVE)};
    const until = Date.now() + 15000;
    while (!existsSync(${JSON.stringify(resolve(pids, 'grandchild'))}) && Date.now() < until) await new Promise(r => setTimeout(r, 50));
    requestStop(${JSON.stringify(run)}, { immediate: ${immediate}, by: 'session-integrator' });`;
  return spawn(process.execPath, ['--input-type=module', '-e', code], { stdio: 'ignore' });
}

describe('a stopped gate records no evidence', () => {
  test('stop --now: no put, no verdict note, no cycle observation, no reading; the close is the stopped line', async t => {
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
    assert.ok(tookMs < 15_000, `the gate ended in ${tookMs} ms, not after its 300 s`);
    assert.equal(fx.calls.gate, 1, 'the gate ran once and was not retried');
    for (const name of ['leader', 'child', 'grandchild']) assert.equal(alive(readPid(resolve(fx.pids, name))), false, name);
    assert.equal(fx.calls.puts, 0, 'no closure evidence put');
    assert.deepEqual(fx.calls.epr, [], 'no gate-verdict@1 note and no cycle observation');
    assert.deepEqual(readings, [], 'no gate or step reading row');
    assert.match(output, /\[gate\] fixture stopped on request by session-integrator: its process group was ended \(SIGTERM\); no evidence recorded/);
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
      '[gate-live] run interrupted: fixture was stopped on request by session-integrator; no evidence was recorded for it, and the push is refused');
    const cli = spawnSync(process.execPath, [LIVE, 'stopped', '--run', fx.run, '--gate', 'fixture'], { encoding: 'utf8' });
    assert.equal(cli.status, 0, 'the hook reads the gate as stopped');
  });

  test('a plain stop waits for the running gate, whose verdict is recorded as before', async t => {
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

describe('the stop CLI', () => {
  test('a stop for a run that already ended is a no-op with a clear line', t => {
    const root = scratch(t, 'gate-stop-ended-');
    const { path: run } = openRun({ root, sha: 'f'.repeat(40), target: 'test' });
    closeRun(run, 'passed');
    const before = readFileSync(run, 'utf8');
    for (const args of [['stop', '--now', '--run', run], ['stop', '--run', run]]) {
      const result = spawnSync(process.execPath, [LIVE, ...args], { encoding: 'utf8', env: { ...process.env, EPR_BIN: '/nonexistent/epr' } });
      assert.equal(result.status, 0, result.stderr);
      assert.match(result.stdout, /already ended \(passed\); nothing to stop, nothing appended/);
    }
    assert.equal(readFileSync(run, 'utf8'), before, 'nothing was appended to the closed run');
  });

  test('stop --now with no gate running says so at once and leaves the plain stop in force', t => {
    const root = scratch(t, 'gate-stop-idle-');
    const { path: run } = openRun({ root, sha: '1'.repeat(40), target: 'test' });
    const result = spawnSync(process.execPath, [LIVE, 'stop', '--now', '--run', run],
      { encoding: 'utf8', env: { ...process.env, EPR_BIN: '/nonexistent/epr', ELOHIM_SESSION_ID: 'session-cli' }, timeout: 10_000 });
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /no gate is running now: no gate after this starts/);
    const request = readRun(run).events.find(event => event.kind === 'stop-request');
    assert.equal(request.immediate, true);
    assert.equal(request.by, 'session-cli');
    assert.match(formatEvent(request), /stop requested now by session-cli — the running gate is being ended and records no evidence/);
  });
});
