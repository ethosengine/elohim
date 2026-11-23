// The pre-push hook's carry wiring: the ACTUAL hook text from the gate loop's start to its verdict,
// sliced by exact comment lines as prepush-gate-live.test.mjs does, run in a scratch repository whose
// gate-runner is a fake that records start and end marks, whose berth answers `room` from the test,
// and whose gate-carry.mjs is this checkout's real module. Timing is proven from the marks, never
// from a margin: each overlapping fixture gate waits until it has seen the other start.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { chmodSync, copyFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawn, spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { groupMembers } from './gate-carry.mjs';

const ROOT = fileURLToPath(new URL('../../', import.meta.url));
const HOOK = readFileSync(join(ROOT, '.husky/pre-push.bash'), 'utf8');

function slice(startMarker, endMarker) {
  const start = HOOK.indexOf(startMarker);
  const end = HOOK.indexOf(endMarker, start);
  assert.ok(start >= 0 && end > start, `hook has ${startMarker} .. ${endMarker}`);
  return HOOK.slice(start, end);
}
const PRELUDE = slice('HOOK_ON_EXIT=""\n', 'BERTH_BIN="$HOOK_WORK_TREE');
const OPEN_BLOCK = slice('# Live gate events:', '# ── Live run open');
const RED_BLOCK = slice('# A red from a check is recorded', '# ── Red recorder end');
const GATES = slice('# ── Run Gates', 'if [ -n "$FAILED" ]; then\n  echo "====');

const cleanEnv = Object.fromEntries(
  Object.entries(process.env).filter(([k]) => !k.startsWith('GIT_') || k.startsWith('GIT_CONFIG_')),
);

// Records `start|end <gate> <ms>`; FAKE_GATES[<gate>] = { waitFor, until, linger, hold, exit }.
// `linger` keeps a gate running that many ms after its waits, so an overlap is longer than the
// millisecond clock's resolution: without it, a gate that finds its partner already started ends
// in the same ms it began, and an overlap that did happen measures 0 ms.
const FAKE_RUNNER = `
import { appendFileSync, existsSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { spawn } from 'node:child_process';
const name = process.argv[process.argv.indexOf('--target') + 1];
const cfg = JSON.parse(process.env.FAKE_GATES || '{}')[name] || {};
const mark = what => appendFileSync('marks.log', what + ' ' + name + ' ' + Date.now() + '\\n');
const seen = line => existsSync('marks.log') && readFileSync('marks.log', 'utf8').split('\\n').some(l => l.startsWith(line + ' '));
const nap = ms => new Promise(r => setTimeout(r, ms));
mark('start');
console.log('gate ' + name + ' output');
if (cfg.hold) {
  const kid = spawn('sleep', ['120'], { stdio: 'ignore' });
  writeFileSync('pids-' + name, process.pid + ' ' + kid.pid);
  await new Promise(() => {});
}
// untilCarried: the named gate's carried record says it ended (what \`gate-carry.mjs peek\` reads).
const carriedEnded = gate => {
  const runs = '.eprfs/status/gate-live/runs';
  return existsSync(runs) && readdirSync(runs).some(n => n.endsWith('.carry') && existsSync(runs + '/' + n + '/' + gate + '/exit.json'));
};
for (const deadline = Date.now() + 20000; cfg.untilCarried && !carriedEnded(cfg.untilCarried);) {
  if (Date.now() > deadline) { mark('timeout'); break; }
  await nap(20);
}
for (const [line, file] of [[cfg.waitFor && 'start ' + cfg.waitFor, null], [null, cfg.until]]) {
  const deadline = Date.now() + 20000;
  while ((line && !seen(line)) || (file && !existsSync(file))) {
    if (Date.now() > deadline) { mark('timeout'); break; }
    await nap(20);
  }
}
if (cfg.linger) await nap(cfg.linger);
mark('end');
process.exit(cfg.exit ?? 0);
`;

// Every fixture hook is started in its own process group, and a carried step leads another (its
// supervisor is detached). Teardown, on pass, fail and timeout alike: TERM then KILL each group, and
// assert nothing of it is left, so this file cannot leave a gate-runner behind to hold a cargo seat.
const HOOK_GROUPS = new Map();
const trackHook = (dir, pid) => { if (pid) HOOK_GROUPS.set(dir, [...(HOOK_GROUPS.get(dir) || []), pid]); };
const NAP = new Int32Array(new SharedArrayBuffer(4));

function reapFixture(dir) {
  const groups = new Set(HOOK_GROUPS.get(dir) || []);
  try {
    for (const rel of readdirSync(dir, { recursive: true })) {
      if (!String(rel).endsWith('handle.json')) continue;
      try { const pid = JSON.parse(readFileSync(join(dir, String(rel)), 'utf8')).pid; if (Number.isInteger(pid)) groups.add(pid); } catch { /* not a record */ }
    }
  } catch { /* directory gone */ }
  const left = () => [...groups].flatMap(group => groupMembers(group));
  for (const [signal, ms] of [['SIGTERM', 2000], ['SIGKILL', 3000]]) {
    if (!left().length) break;
    for (const group of groups) { try { process.kill(-group, signal); } catch { /* gone */ } }
    for (const end = Date.now() + ms; left().length && Date.now() < end;) Atomics.wait(NAP, 0, 0, 25);
  }
  HOOK_GROUPS.delete(dir);
  assert.deepEqual(left(), [], 'the fixture hook and every carried step are gone');
}

function fixture(t) {
  const dir = mkdtempSync(join(tmpdir(), 'prepush-carry-'));
  t.after(() => {
    try { reapFixture(dir); } finally {
      // Any process a failed assertion left running.
    for (const pidFile of ['pids-storage', 'pids-app']) {
      try { for (const pid of readFileSync(join(dir, pidFile), 'utf8').trim().split(' ')) process.kill(Number(pid), 'SIGKILL'); } catch { /* gone */ }
    }
      rmSync(dir, { recursive: true, force: true });
    }
  });
  assert.equal(spawnSync('git', ['-C', dir, 'init', '--quiet'], { env: cleanEnv }).status, 0);
  const orch = join(dir, 'genesis/orchestrator');
  mkdirSync(orch, { recursive: true });
  mkdirSync(join(dir, 'genesis/agentic/bin'), { recursive: true });
  copyFileSync(join(ROOT, 'genesis/orchestrator/gate-live.mjs'), join(orch, 'gate-live.mjs'));
  copyFileSync(join(ROOT, 'genesis/orchestrator/file-lock.mjs'), join(orch, 'file-lock.mjs'));
  writeFileSync(join(orch, 'gate-preflight.mjs'), "process.stdout.write((process.env.FAKE_ON_RED || 'continue') + '\\n');\n");
  writeFileSync(join(orch, 'gate-runner.mjs'), FAKE_RUNNER);
  // The real module, driven from the fixture: GATE_ROOT points its manifests, ledger and berth here.
  writeFileSync(join(orch, 'gate-carry.mjs'),
    `import { main } from ${JSON.stringify(join(ROOT, 'genesis/orchestrator/gate-carry.mjs'))};\nprocess.exitCode = await main(process.argv.slice(2));\n`);
  writeFileSync(join(dir, 'genesis/agentic/bin/berth'), '#!/usr/bin/env bash\necho "$FAKE_ROOM"\n');
  chmodSync(join(dir, 'genesis/agentic/bin/berth'), 0o755);
  writeFileSync(join(dir, 'genesis/agentic/pool-policy.json'), '{}');
  writeFileSync(join(dir, 'build-manifest.json'), JSON.stringify({ pipeline: 'fx', steps: { build: {} }, gate: { projects: {
    storage: { dir: 'storage', run: { kind: 'just', recipe: 'gate', cargo: { workspace: 'storage', env: { CARGO_BUILD_JOBS: '1' } } } },
    app: { dir: 'app', run: { kind: 'just', recipe: 'gate' } },
    docs: { dir: 'docs', run: { kind: 'just', recipe: 'gate' } },
  } } }));
  mkdirSync(join(dir, '.eprfs/status'), { recursive: true });
  writeFileSync(join(dir, '.eprfs/status/flows.jsonl'), [['storage', 3535], ['app', 30]].map(([project, value], i) =>
    JSON.stringify({ cid: `c${i}`, record: { occurredAt: '2026-11-20T00:00:00Z', classifiedAs: ['run:observation', project,
      'measure:gate-cycle-seconds@1', `value:${value}`, 'env:run-kind=pre-push', 'env:exit=ok', `env:project=${project}`] } })).join('\n'));
  return dir;
}

const ADMIT = JSON.stringify({ verdict: 'admit', reason: 'fixture room' });
const REFUSE = JSON.stringify({ verdict: 'refuse', axis: 'host-floor:memory', owner: 'self', reason: 'ram-guard level high at 82% committed' });
const SHA = 'a'.repeat(40);

const script = (dir, projects = 'storage app') => [
  'set -o pipefail', `HOOK_WORK_TREE='${dir}'`, `LIVE_SHA=${SHA}`, 'LIVE_TARGETS=refs/heads/dev', PRELUDE, OPEN_BLOCK, RED_BLOCK,
  `PROJECTS="${projects}"`, GATES,
  'if [ -n "$FAILED" ]; then echo "REFUSED:$FAILED"; exit 1; fi', 'echo ALL-CLEAR',
].join('\n');

function env(dir, extra) {
  return { ...cleanEnv, GATE_ROOT: dir, EPR_BIN: '/nonexistent/epr', ELOHIM_SESSION_ID: 'carry-test', ...extra };
}

// detached: the hook leads its own process group, which reapFixture ends whatever the test did.
function runHookScript(dir, source, extra) {
  const r = spawnSync('bash', ['-c', source], { cwd: dir, env: env(dir, extra), encoding: 'utf8', timeout: 60_000,
    killSignal: 'SIGKILL', detached: true });
  trackHook(dir, r.pid);
  reapFixture(dir);
  return r;
}
const runHook = (dir, extra) => runHookScript(dir, script(dir), extra);

function marks(dir) {
  const rows = readFileSync(join(dir, 'marks.log'), 'utf8').trim().split('\n').map(l => l.split(' '));
  const at = (what, gate) => Number(rows.find(([w, g]) => w === what && g === gate)?.[2]);
  assert.ok(!rows.some(([w]) => w === 'timeout'), `no fixture gate timed out waiting: ${rows.join(' | ')}`);
  return { at, rows };
}

const serial = m => m.at('end', 'storage') <= m.at('start', 'app');

test('off: no carry line, the gates run one after another', function carry_off_runs_serially(t) {
  const dir = fixture(t);
  const r = runHook(dir, { GATE_CARRY: 'off', FAKE_ROOM: ADMIT });
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.ok(!(r.stdout + r.stderr).includes('[carry]'), r.stderr);
  assert.ok(serial(marks(dir)));
  assert.ok(r.stdout.includes('storage: PASSED') && r.stdout.includes('app: PASSED'));
  assert.ok(!r.stdout.includes('carry:'), r.stdout);
});

test('off and nothing carried print the same push as each other', function carry_off_equals_nothing_carried(t) {
  const normalize = text => text.replace(/\(\d+s\)/g, '(Ns)').replace(/\S*\.jsonl/g, 'RUN').replace(/\d{4}-\d\d-\d\dT[\d:.]+Z/g, 'T');
  const off = fixture(t);
  const a = runHook(off, { GATE_CARRY: 'off', FAKE_ROOM: ADMIT });
  const refused = fixture(t);
  const b = runHook(refused, { GATE_CARRY: 'on', FAKE_ROOM: REFUSE });
  assert.equal(a.status, 0, a.stderr);
  assert.equal(b.status, 0, b.stderr);
  assert.equal(normalize(b.stdout), normalize(a.stdout));
  assert.ok(serial(marks(refused)));
  assert.match(b.stderr, /\[carry\] not carried: storage: host-floor:memory: ram-guard level high at 82% committed \(self\)/);
});

test('shadow (the default) names what it would carry and runs the gates one after another', function carry_shadow_prints_and_runs_serially(t) {
  const dir = fixture(t);
  const r = runHook(dir, { FAKE_ROOM: ADMIT });
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.match(r.stderr, /\[carry\] shadow: would carry storage on second-seat \(p90 59 min, one build job\); GATE_CARRY=on carries it/);
  assert.ok(serial(marks(dir)));
  assert.ok(!r.stdout.includes('carry: the overlap saved'));
});

test('on: the carried gate runs beside the loop and both verdicts are reported', function carry_on_overlaps(t) {
  const dir = fixture(t);
  const r = runHook(dir, { GATE_CARRY: 'on', FAKE_ROOM: ADMIT,
    FAKE_GATES: JSON.stringify({ storage: { waitFor: 'app', linger: 30 }, app: { waitFor: 'storage', linger: 30 } }) });
  assert.equal(r.status, 0, r.stdout + r.stderr);
  const m = marks(dir);
  // Each waited to see the other start, then lingered, so the two ran at once by construction
  // and the overlap is at least the linger.
  const starts = [m.at('start', 'storage'), m.at('start', 'app')];
  const ends = [m.at('end', 'storage'), m.at('end', 'app')];
  assert.ok(Math.max(...starts) <= Math.min(...ends), m.rows.join(' | '));
  const sum = ends[0] - starts[0] + ends[1] - starts[1];
  const wall = Math.max(...ends) - Math.min(...starts);
  assert.ok(wall < sum, `wall ${wall} ms < sum ${sum} ms`);
  assert.match(r.stderr, /\[carry\] carried: storage on second-seat \(p90 59 min, one build job\) beside the loop/);
  assert.match(r.stdout, /──── carried: storage on second-seat .*\n[\s\S]*gate storage output[\s\S]*──── end of storage: exit 0 ────\nstorage: PASSED \(\d+s\)/);
  assert.ok(r.stdout.indexOf('app: PASSED') < r.stdout.indexOf('storage: PASSED'), 'the loop reports first, the carried step after it');
  assert.match(r.stdout, /\[pre-push\] carry: the overlap saved -?\d+s \(gates \d+s, wall \d+s\)/);
  assert.ok(r.stdout.includes('ALL-CLEAR'));
});

test('on: a red in the carried gate refuses the push', function carry_red_refuses(t) {
  const dir = fixture(t);
  const r = runHook(dir, { GATE_CARRY: 'on', FAKE_ROOM: ADMIT, FAKE_GATES: JSON.stringify({ storage: { exit: 1 } }) });
  assert.equal(r.status, 1, r.stdout + r.stderr);
  assert.match(r.stdout, /storage: FAILED \(\d+s\)/);
  assert.match(r.stdout, /REFUSED:storage/);
  assert.ok(r.stdout.includes('app: PASSED'));
});

test('on: a red in a loop gate still waits for and reports the carried gate', async function carry_loop_red_waits_for_carried(t) {
  const dir = fixture(t);
  const child = spawn('bash', ['-c', script(dir)], { cwd: dir, env: env(dir, { GATE_CARRY: 'on', FAKE_ROOM: ADMIT,
    FAKE_GATES: JSON.stringify({ storage: { waitFor: 'app', until: 'release-storage' }, app: { exit: 1 } }) }), detached: true });
  trackHook(dir, child.pid); // reaped by the fixture's teardown
  let out = '';
  child.stdout.on('data', d => { out += d; });
  child.stderr.on('data', d => { out += d; });
  const exited = new Promise(done => child.on('exit', code => done(code)));
  const deadline = Date.now() + 20_000;
  while (Date.now() < deadline && !out.includes('app: FAILED')) await new Promise(r => setTimeout(r, 20));
  assert.ok(out.includes('app: FAILED'), out);
  assert.ok(!out.includes('storage: PASSED'), 'the carried gate is still running when the loop red is reported');
  writeFileSync(join(dir, 'release-storage'), '');
  assert.equal(await exited, 1, out);
  assert.match(out, /storage: PASSED/);
  assert.match(out, /REFUSED:app/);
  assert.ok(out.indexOf('app: FAILED') < out.indexOf('storage: PASSED'));
});

test('an interrupted hook leaves no carried gate running', async function carry_interrupt_withdraws(t) {
  const dir = fixture(t);
  const child = spawn('bash', ['-c', script(dir)], { cwd: dir, env: env(dir, { GATE_CARRY: 'on', FAKE_ROOM: ADMIT,
    FAKE_GATES: JSON.stringify({ storage: { hold: true }, app: { waitFor: 'storage', until: 'release-app' } }) }), detached: true });
  trackHook(dir, child.pid); // reaped by the fixture's teardown
  let out = '';
  child.stdout.on('data', d => { out += d; });
  child.stderr.on('data', d => { out += d; });
  const exited = new Promise(done => child.on('exit', (code, signal) => done({ code, signal })));
  const deadline = Date.now() + 20_000;
  while (Date.now() < deadline && !existsSync(join(dir, 'pids-storage'))) await new Promise(r => setTimeout(r, 20));
  const pids = readFileSync(join(dir, 'pids-storage'), 'utf8').trim().split(' ').map(Number);
  const alive = pid => { try { const s = readFileSync(`/proc/${pid}/stat`, 'utf8'); return s.slice(s.lastIndexOf(')') + 2)[0] !== 'Z'; } catch { return false; } };
  assert.ok(pids.every(alive), 'the carried gate and its child run');
  // Only bash is signalled: its TERM trap runs once the loop's foreground gate returns.
  child.kill('SIGTERM');
  writeFileSync(join(dir, 'release-app'), '');
  const { code } = await exited;
  assert.equal(code, 143, out);
  assert.ok(pids.every(pid => !alive(pid)), `no carried process left: ${pids.filter(alive)}`);
  assert.match(out, /\[carry\] withdrawn: storage from second-seat/);
});

test('GATE_ON_RED=stop: a carried red stops the loop before its next gate', async function carry_red_under_stop_stops_the_loop(t) {
  const dir = fixture(t);
  const r = runHookScript(dir, script(dir, 'storage app docs'), { GATE_CARRY: 'on', FAKE_ROOM: ADMIT, FAKE_ON_RED: 'stop',
    FAKE_GATES: JSON.stringify({ storage: { waitFor: 'app', exit: 1 }, app: { untilCarried: 'storage' } }) });
  assert.equal(r.status, 1, r.stdout + r.stderr);
  assert.ok(r.stdout.includes('app: PASSED'), r.stdout);
  assert.ok(!r.stdout.includes('docs: PASSED'), 'nothing starts after the carried red');
  assert.match(r.stdout, /stopped: GATE_ON_RED=stop after a red; not run: docs/);
  assert.match(r.stdout, /storage: FAILED/);
});

// A stand-in for the helper, for the hook's own defences: it says what it is told on `offer` and
// records every verb it is called with.
function fakeHelper(dir, { offer = '', collectExit = 1 } = {}) {
  writeFileSync(join(dir, 'genesis/orchestrator/gate-carry.mjs'), `
import { appendFileSync } from 'node:fs';
const [verb, ...rest] = process.argv.slice(2);
appendFileSync('helper-calls.log', verb + ' ' + rest.join(' ') + '\\n');
if (verb === 'offer') process.stdout.write(${JSON.stringify(offer)});
if (verb === 'peek') process.stdout.write('running\\n');
if (verb === 'collect') process.exit(${collectExit});
process.exit(0);
`);
}

const ranInline = (dir, r) => {
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.ok(serial(marks(dir)), 'every gate ran inline, one after another');
  assert.ok(r.stdout.includes('storage: PASSED') && r.stdout.includes('app: PASSED') && r.stdout.includes('ALL-CLEAR'));
  assert.ok(!r.stdout.includes('carry: the overlap saved'));
};

test('shadow: a helper that prints a garbage row changes nothing', function carry_shadow_ignores_helper_stdout(t) {
  for (const mode of [{}, { GATE_CARRY: 'shadow' }, { GATE_CARRY: 'garbage' }]) {
    const dir = fixture(t);
    fakeHelper(dir, { offer: 'storage\t/missing\n' });
    const r = runHook(dir, { ...mode, FAKE_ROOM: ADMIT });
    ranInline(dir, r);
    assert.match(readFileSync(join(dir, 'helper-calls.log'), 'utf8'), /^offer /);
    assert.ok(!readFileSync(join(dir, 'helper-calls.log'), 'utf8').includes('withdraw'), 'nothing to withdraw');
  }
});

test('off: the helper is not called at all', function carry_off_never_calls_helper(t) {
  const dir = fixture(t);
  fakeHelper(dir, { offer: 'storage\t/missing\n' });
  const r = runHook(dir, { GATE_CARRY: 'off', FAKE_ROOM: ADMIT });
  ranInline(dir, r);
  assert.ok(!existsSync(join(dir, 'helper-calls.log')), 'no verb was run');
});

test('on: a row for an unknown project or a missing handle is dropped, saying so', function carry_on_drops_bad_rows(t) {
  const dir = fixture(t);
  mkdirSync(join(dir, 'handle-ok'));
  writeFileSync(join(dir, 'handle-ok/handle.json'), '{}');
  fakeHelper(dir, { offer: `nosuch\t${join(dir, 'handle-ok')}\nstorage\t${join(dir, 'no-such-handle')}\ngarbage line\n` });
  const r = runHook(dir, { GATE_CARRY: 'on', FAKE_ROOM: ADMIT });
  ranInline(dir, r);
  assert.match(r.stderr, /\[pre-push\] carry: dropped a row .*nosuch/);
  assert.match(r.stderr, /\[pre-push\] carry: dropped a row .*storage/);
  assert.ok(!readFileSync(join(dir, 'helper-calls.log'), 'utf8').includes('withdraw'));
});

test('on: a helper that fails or is missing leaves the loop as it was', function carry_on_helper_failure(t) {
  const dir = fixture(t);
  rmSync(join(dir, 'genesis/orchestrator/gate-carry.mjs'));
  ranInline(dir, runHook(dir, { GATE_CARRY: 'on', FAKE_ROOM: ADMIT }));
});

test('on: a collect that prints no verdict fails the step and leaves its handle to the exit trap', function carry_failed_collect_is_withdrawn(t) {
  const dir = fixture(t);
  const handle = join(dir, 'handle-storage');
  mkdirSync(handle);
  writeFileSync(join(handle, 'handle.json'), '{}');
  fakeHelper(dir, { offer: `storage\t${handle}\n`, collectExit: 1 });
  const r = runHook(dir, { GATE_CARRY: 'on', FAKE_ROOM: ADMIT });
  assert.equal(r.status, 1, r.stdout + r.stderr);
  assert.match(r.stdout, /storage: FAILED/);
  assert.match(r.stdout, /REFUSED:storage/);
  assert.match(r.stderr, /collect of storage printed no verdict/);
  // The step was carried, so the loop did not run it; only app ran inline.
  assert.ok(r.stdout.includes('app: PASSED'));
  // The hook's exit ran the trap: the uncollected handle was withdrawn.
  assert.match(readFileSync(join(dir, 'helper-calls.log'), 'utf8'), new RegExp(`withdraw ${handle}`));
});
