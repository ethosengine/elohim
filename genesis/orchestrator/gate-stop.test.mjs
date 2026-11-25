// A requested stop ends the gate that is RUNNING (`gate-live.mjs stop --now`) and records nothing
// for it; a plain stop still lets the running gate finish. Habit gate-failures-arrive-live.

import { describe, test } from 'node:test';
import { strict as assert } from 'node:assert';
import { spawn, spawnSync } from 'node:child_process';
import { appendFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { setTimeout as delay } from 'node:timers/promises';

import { appendEvent, awaitStopAnswer, closeRun, formatEvent, openRun, readRun, requestStop, stopCounts, stopRequested } from './gate-live.mjs';
import { GATE_STOPPED_EXIT, runProject } from './gate-runner.mjs';
import { endTree, gateTree, readStat, snapshot, stopGraceMs, stoppedBy } from './gate-stop.mjs';

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
    assert.match(result.stderr, /stopped on request by session-stopper: its leader died of SIGTERM and its process group was ended \(SIGTERM\); no verdict is recorded for it/);
    // The gate's own death throes are the stop, not reds: nothing was appended as a failure.
    assert.equal(readRun(run).events.filter(event => event.kind === 'failure').length, 0);
  });

  test('a grandchild that ignores TERM is KILLed once the grace is over', async t => {
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
  test('a gate that exits 1 from its own TERM handler keeps its red', async t => {
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

  test('endTree on a leader that already exited sends the leader nothing', async () => {
    const gone = spawn('sleep', ['0.1'], { detached: true, stdio: 'ignore' });
    await waitFor(() => readStat(gone.pid));
    const recorded = readStat(gone.pid);
    await new Promise(res => gone.on('exit', res));
    const sent = [];
    const result = await endTree(recorded, { graceMs: 100, kill: (pid, signal) => sent.push([pid, signal]) });
    assert.deepEqual(result.toLeader, []);
    assert.deepEqual(sent.filter(([pid]) => pid === recorded.pid), [], 'never the leader pid');
  });

  test('stoppedBy matches the signal sent to the leader, and only when it came before the exit', () => {
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
  test('the matrix has fifteen cells', () => assert.equal(MATRIX.length, 15));
});

describe('an escaped child does not hang the run', () => {
  test('a green whose output was cut after its exit is recorded, and says its output was cut', async t => {
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

  test('the leader ended on its own and a stray holds its output: the verdict stands and the wait is bounded', async t => {
    const dir = scratch(t, 'gate-stop-stray-');
    const script = resolve(dir, 'gate.sh');
    writeFileSync(script, [
      '#!/usr/bin/env bash',
      `( setsid bash -c 'echo $$ > "${dir}/escaped"; exec sleep 30' & )`,
      `while [ ! -s "${dir}/escaped" ]; do sleep 0.05; done`,
      `touch "${dir}/leader-done"`,
      'exit 1',
    ].join('\n'), { mode: 0o755 });
    t.after(() => { try { process.kill(readPid(resolve(dir, 'escaped')), 'SIGKILL'); } catch { /* gone */ } });
    const { path: run } = openRun({ root: dir, sha: '4'.repeat(40), target: 'test' });
    const { done } = startTap(run, script);
    assert.ok(await waitFor(() => existsSync(resolve(dir, 'leader-done'))));
    await delay(300);
    const escaped = readPid(resolve(dir, 'escaped'));
    const asked = Date.now();
    requestStop(run, { immediate: true, by: 'session-stopper' });
    const result = await done;
    assert.ok(Date.now() - asked < 2000 + 1500, 'the wait for the stray is bounded');
    assert.equal(result.code, 1, 'the leader\'s own red is the tap\'s exit');
    assert.equal(result.summary.stopped, undefined, 'not stopped: the recipe ended on its own');
    assert.match(result.stderr, new RegExp(`holds its output: pid ${escaped} \\(sleep 30\\)`));
    assert.match(result.stderr, /the recipe ended on its own \(exit 1\); its verdict stands, and the push is refused/);
    assert.equal(alive(escaped), true, 'never signalled');
  });

  test('a double-forked holder of the output is named, and the tap stops waiting for it', async t => {
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
    assert.ok(tookMs < 500 + 2000 + 2000 + 1500, `the tap ended in ${tookMs} ms although pid ${escaped} still holds its output`);
    assert.equal(alive(escaped), true, 'the escaped process is outside the tree and never signalled');
    assert.match(result.stderr, new RegExp(`a process outside the gate's tree may still be running and holds its output: pid ${escaped} \\(sleep 30\\)`));
    assert.equal(result.summary.stopped.signal, 'SIGTERM');
    assert.deepEqual(result.summary.stopped.escaped.map(holder => holder.pid), [escaped]);
  });
});

describe('Ctrl-C still reaches the detached gate', () => {
  test('SIGINT to the tap ends the gate group, and no stop is reported', async t => {
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
    assert.equal(result.summary?.stopped, undefined);
  });
});

describe('the stop CLI', () => {
  test('a stop for a push run that already passed says the push is proceeding, and exits 1', t => {
    const root = scratch(t, 'gate-stop-passed-');
    const { path: run } = openRun({ root, sha: 'e'.repeat(40), target: 'refs/heads/dev' });
    closeRun(run, 'gates-passed');
    const before = readFileSync(run, 'utf8');
    const result = spawnSync(process.execPath, [LIVE, 'stop', '--now', '--run', run], { encoding: 'utf8', env: { ...process.env, EPR_BIN: '/nonexistent/epr' } });
    assert.equal(result.status, 1);
    assert.match(result.stdout, /this run has passed its last stop point; the push is proceeding/);
    assert.equal(readFileSync(run, 'utf8'), before, 'nothing appended');
  });

  test('a stop for a run that already ended is a no-op with a clear line', t => {
    const root = scratch(t, 'gate-stop-ended-');
    const { path: run } = openRun({ root, sha: 'f'.repeat(40), target: 'test' });
    closeRun(run, 'failed');
    const before = readFileSync(run, 'utf8');
    for (const args of [['stop', '--now', '--run', run], ['stop', '--run', run]]) {
      const result = spawnSync(process.execPath, [LIVE, ...args], { encoding: 'utf8', env: { ...process.env, EPR_BIN: '/nonexistent/epr' } });
      assert.equal(result.status, 0, result.stderr);
      assert.match(result.stdout, /already ended \(failed\); nothing was stopped, nothing appended/);
    }
    assert.equal(readFileSync(run, 'utf8'), before, 'nothing was appended to the closed run');
  });

  test('a stop the run never answers is unconfirmed: exit 2, and the push may be proceeding', t => {
    const root = scratch(t, 'gate-stop-idle-');
    const { path: run } = openRun({ root, sha: '1'.repeat(40), target: 'test' });
    const result = spawnSync(process.execPath, [LIVE, 'stop', '--now', '--wait', '1', '--run', run],
      { encoding: 'utf8', env: { ...process.env, EPR_BIN: '/nonexistent/epr', ELOHIM_SESSION_ID: 'session-cli' }, timeout: 10_000 });
    assert.equal(result.status, 2, result.stdout + result.stderr);
    assert.match(result.stdout, /stop requested but not confirmed by the run within 1 s; the push may be proceeding/);
    const request = readRun(run).events.find(event => event.kind === 'stop-request');
    assert.equal(request.immediate, true);
    assert.equal(request.by, 'session-cli');
    assert.match(formatEvent(request), /stop requested now by session-cli — a gate still running is ended and records no verdict; .* the push is refused/);
  });

  test('stop --now on a running gate exits 0 only once the run\'s stopped close exists', async t => {
    const fx = evidenceFixture(t, 'gate-stop-cli-now-', { seconds: 300 });
    const exitFile = resolve(fx.pids, 'cli-exit');
    const cli = spawn('bash', ['-c', `until [ -f "${resolve(fx.pids, 'grandchild')}" ]; do sleep 0.05; done; `
      + `"${process.execPath}" "${LIVE}" stop --now --wait 20 --run "${fx.run}" > "${resolve(fx.pids, 'cli-out')}"; echo $? > "${exitFile}"`],
      { stdio: 'ignore', env: { ...process.env, EPR_BIN: '/nonexistent/epr', ELOHIM_SESSION_ID: 'session-cli' } });
    t.after(() => cli.kill());
    const status = runProject(fx.project, false, false, { root: fx.root, spawn: fx.spawnFake, tap: true, env: fx.env,
      readingCollector: [], write: () => {}, warn: () => {} });
    assert.equal(status, GATE_STOPPED_EXIT);
    assert.ok(readRun(fx.run).events.some(event => event.kind === 'gate-stopped'));
    await delay(600);
    assert.equal(existsSync(exitFile), false, 'no answer yet: the gate is stopped, but the run has not closed');
    closeRun(fx.run, 'failed');
    assert.ok(await waitFor(() => existsSync(exitFile) && readFileSync(exitFile, 'utf8').trim() !== ''));
    assert.equal(readFileSync(exitFile, 'utf8').trim(), '0');
    assert.match(readFileSync(resolve(fx.pids, 'cli-out'), 'utf8'), /fixture stopped \(SIGTERM\); no verdict recorded; the run closed interrupted and the push is refused/);
  });

  test('a stop that ties the stop-point mark is after it, for the reader and the stop verb alike', async t => {
    const root = scratch(t, 'gate-stop-tie-');
    const { path: run } = openRun({ root, sha: '5'.repeat(40), target: 'refs/heads/dev' });
    const mark = appendEvent(run, { kind: 'stop-boundary' });
    const tie = { kind: 'stop-request', immediate: true, v: 1, run: 'x', seq: mark.seq, at: mark.at };
    appendFileSync(run, `${JSON.stringify(tie)}\n`);
    assert.equal(stopCounts(tie, mark), false);
    assert.equal(stopRequested(run), false, 'the hook\'s reader does not count it');
    const answer = await awaitStopAnswer(run, tie, { timeoutMs: 500 });
    assert.equal(answer.state, 'proceeding', 'the stop verb reports the push proceeding');
  });
});
