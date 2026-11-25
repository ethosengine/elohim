// Device acts run the device's head epr; session acts keep the session's pin (device-epr.mjs).
//
// A fake bin store holds an old release (the session's pin) and a newer head, each a stub that
// appends "<which> <args>" to one log, and a PATH launcher that answers `bin resolve` with the
// pin and execs the pin for everything else, as bin-launch does for a pinned session.
import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  compareToHead, deviceEpr, deviceEprResolution, deviceHead, resetDeviceEprCache, resolveDeviceEpr,
} from './device-epr.mjs';
import { emitGateVerdict, siblingVerdicts } from './gate-evidence.mjs';
import { postMisses } from './handover-record.mjs';
import { appendGateReadings } from './gate-runner.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const SHA = 'a'.repeat(40);
const TREE = 'b'.repeat(40);
const BASE = 'c'.repeat(40);
const OLD = 'bafyold';
const HEAD = 'bafyhead';
const NEWER = 'bafynewer';

function script(path, body) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `#!/bin/sh\n${body}\n`);
  chmodSync(path, 0o755);
}

/** `{ env, log(), head, pin, newer, root }`; `sessionSays` is what the launcher's `bin resolve` prints. */
function fixture(t, { current = HEAD, sessionSays = 'pin', launcher = true } = {}) {
  const root = mkdtempSync(join(tmpdir(), 'device-epr-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const log = join(root, 'ran.log');
  const store = join(root, 'store');
  const release = (cid, which, ingestedAt) => {
    const path = join(store, 'epr', cid, 'epr');
    script(path, [
      `echo "${which} $*" >> "${log}"`,
      'case "$*" in',
      '  "peer leases"*) echo \'{"standing":"device","device":"did:key:zFixture"}\' ;;',
      '  "peer verdicts"*) echo "[]" ;;',
      '  *--misses*) echo "{}" ;;',
      '  "flow intend"*) echo \'{"intent":"bafyintent"}\' ;;',
      'esac',
      'exit 0',
    ].join('\n'));
    if (ingestedAt) writeFileSync(join(dirname(path), 'record.json'), JSON.stringify({ cid, ingestedAt }));
    return path;
  };
  const pin = release(OLD, 'pin', '2026-11-02T00:00:00Z');
  const head = release(HEAD, 'head', '2026-11-25T00:00:00Z');
  const newer = release(NEWER, 'newer', '2026-11-26T00:00:00Z');
  if (current) writeFileSync(join(store, 'epr', 'current.json'), JSON.stringify({ cid: current }));
  const bin = join(root, 'bin');
  mkdirSync(bin, { recursive: true });
  const said = { pin: [pin, OLD], newer: [newer, NEWER], head: [head, HEAD] }[sessionSays];
  if (launcher) {
    script(join(bin, 'epr'), [
      'if [ "$1 $2" = "bin resolve" ]; then',
      `  echo "launcher $*" >> "${log}"`,
      said ? `  echo '{"path":"${said[0]}","cid":"${said[1]}","source":"session-pin"}'; exit 0` : '  exit 1',
      'fi',
      `exec "${said ? said[0] : pin}" "$@"`,
    ].join('\n'));
  }
  // `timeout -k 1 5 <cmd...>` without needing coreutils on the fixture PATH.
  script(join(bin, 'timeout-stub'), 'shift 3; exec "$@"');
  const env = { PATH: `${bin}:/usr/bin:/bin`, HOME: root, BIN_STORE: store, TIMEOUT_BIN: join(bin, 'timeout-stub') };
  return {
    root, env, head, pin, newer, store,
    log: () => (existsSync(log) ? readFileSync(log, 'utf8').split('\n').filter(Boolean) : []),
  };
}

/** The acts spawn without an env of their own; give them the fixture's. git is answered here. */
function spawnIn(fx) {
  return (command, args, options = {}) => {
    if (command === 'git') {
      if (args.includes('--git-common-dir')) return { status: 0, stdout: `${fx.root}/.git\n` };
      if (args[0] === 'show') return { status: 0, stdout: `fix: a thing\n\nChange-Id: I${'d'.repeat(40)}\n` };
      if (args[0] === 'rev-parse' && String(args.at(-1)).endsWith('^{tree}')) return { status: 0, stdout: `${TREE}\n` };
      if (args[0] === 'rev-parse') return { status: 0, stdout: `${SHA}\n` };
      return { status: 1, stdout: '', stderr: `unexpected git ${args.join(' ')}` };
    }
    return spawnSync(command, args, { ...options, env: fx.env });
  };
}

const project = { name: 'orchestrator' };
const record = { head: SHA, destinations: ['refs/heads/self/fixture'], verdicts: [] };
const handOver = (fx, extra = {}) => postMisses(record, ['orchestrator'], {
  root: fx.root, env: fx.env, spawn: spawnIn(fx), now: () => new Date(0), err: line => (extra.lines || []).push(line),
  lookup: () => ({ closure: 'f'.repeat(16), green: false }), p90: () => 60,
  classify: () => ({ base: BASE, class: 'low' }),
});
const emit = fx => emitGateVerdict(project, 'closure', {
  success: true, seconds: 1, commit: SHA, put: {}, root: fx.root, env: fx.env, spawn: spawnIn(fx),
});
const ran = (fx, which, text) => fx.log().filter(line => line.startsWith(`${which} `) && line.includes(text));

test('harm (i)/(v): a hand-over raises its intent and posts its misses through the head, not the pin', t => {
  resetDeviceEprCache();
  const fx = fixture(t);
  const line = handOver(fx);
  assert.match(line, /^hand-over: posted 1 misses on intent bafyintent: orchestrator$/);
  assert.equal(ran(fx, 'head', 'flow intend --change').length, 1);
  assert.equal(ran(fx, 'head', 'flow intend --misses bafyintent').length, 1);
  assert.equal(ran(fx, 'pin', 'flow intend').length, 0);
});

test('a verdict emit and the sibling verdict read run the head', t => {
  resetDeviceEprCache();
  const fx = fixture(t);
  assert.equal(emit(fx), true);
  assert.deepEqual(siblingVerdicts(project, 'closure', { root: fx.root, env: fx.env, spawn: spawnIn(fx) }), { candidates: [] });
  assert.equal(ran(fx, 'head', 'peer leases').length, 1);
  assert.equal(ran(fx, 'head', 'flow note --kind observation --measure gate-verdict@1').length, 1);
  assert.equal(ran(fx, 'head', 'peer verdicts').length, 1);
  assert.deepEqual(fx.log().filter(line => line.startsWith('pin ')), []);
});

test('harm (v): a reading append is a session act and runs the pin, beside a device act on the head', t => {
  resetDeviceEprCache();
  const fx = fixture(t);
  assert.equal(emit(fx), true);
  appendGateReadings([{ run_id: 'run-1', step: 's' }], { root: fx.root, env: fx.env, spawn: spawnIn(fx), warn: () => {}, write: () => {} });
  assert.equal(ran(fx, 'pin', 'flow reading append').length, 1);
  assert.equal(ran(fx, 'pin', 'flow report reading').length, 1);
  assert.deepEqual(ran(fx, 'head', 'reading'), []);
});

test('harm (v): the session-act call sites still name the session epr and import no resolver', () => {
  const runner = readFileSync(join(HERE, 'gate-runner.mjs'), 'utf8');
  // reading append, reading report, the attested pin-check observation, the cycle observation.
  assert.equal(runner.split("env.EPR_BIN || 'epr'").length - 1, 4);
  assert.equal(runner.split('deviceEpr(').length - 1, 1);
  const live = readFileSync(join(HERE, 'gate-live.mjs'), 'utf8');
  assert.ok(!live.includes('device-epr'), 'gate-live resolves session identity and is left alone');
  for (const file of ['gate-evidence.mjs', 'handover-record.mjs', 'gate-runner.mjs', 'gate-preflight.mjs']) {
    assert.ok(!/function\s+(deviceHead|compareToHead|resolveDeviceEpr)\b/.test(readFileSync(join(HERE, file), 'utf8')), `${file} holds no second resolver`);
  }
});

test('harm (ii): an EPR_BIN older than the head is ignored with one line, and the act runs the head', t => {
  resetDeviceEprCache();
  const fx = fixture(t);
  fx.env.EPR_BIN = fx.pin;
  const lines = [];
  const resolution = deviceEprResolution({ env: fx.env, warn: line => lines.push(line) });
  assert.equal(resolution.command, fx.head);
  assert.equal(lines.length, 1);
  assert.match(lines[0], /^\[device-epr\] EPR_BIN=.*bafyold\/epr is older than this device's head epr; ignored/);
  assert.equal(emit(fx), true);
  assert.equal(ran(fx, 'head', 'flow note').length, 1);
  assert.deepEqual(fx.log().filter(line => line.startsWith('pin ')), []);
  assert.equal(lines.length, 1, 'the line is printed once per process');
  // The CLI the shell and Python call sites run says the same and prints the head.
  const cli = spawnSync(process.execPath, [join(HERE, 'device-epr.mjs')], { env: fx.env, encoding: 'utf8' });
  assert.equal(cli.status, 0);
  assert.equal(cli.stdout, `${fx.head}\n`);
  assert.equal(cli.stderr.trim().split('\n').length, 1);
  assert.match(cli.stderr, /is older than this device's head epr; ignored/);
});

test('harm (ii): never older: a newer EPR_BIN candidate is used, and a session epr newer than the head keeps the launcher', t => {
  const fx = fixture(t);
  assert.deepEqual(resolveDeviceEpr({ env: { ...fx.env, EPR_BIN: fx.newer } }), { command: fx.newer, source: 'override', why: null });
  const outside = join(fx.root, 'built', 'epr');
  script(outside, 'exit 0');
  assert.equal(resolveDeviceEpr({ env: { ...fx.env, EPR_BIN: outside } }).source, 'override');
  assert.equal(compareToHead(fx.pin, fx.head), 'older');
  assert.equal(compareToHead(fx.newer, fx.head), 'candidate');
  assert.equal(compareToHead(fx.head, fx.head), 'same');

  const rolledBack = fixture(t, { sessionSays: 'newer' });
  const stays = resolveDeviceEpr({ env: rolledBack.env });
  assert.equal(stays.source, 'session');
  assert.equal(stays.command, 'epr');
  assert.match(stays.why, /^\[device-epr\] this session's epr \(bafynewer\) is not older than this device's head \(bafyhead\)/);

  const unreadable = fixture(t, { sessionSays: 'none' });
  const unknown = resolveDeviceEpr({ env: unreadable.env });
  assert.equal(unknown.source, 'path');
  assert.equal(unknown.command, 'epr');
  assert.match(unknown.why, /this session's own epr could not be read \(epr bin resolve exited 1\)/);
});

test('harm (iii): an unreadable head falls back to the PATH launcher with one line, and the acts answer as before', t => {
  resetDeviceEprCache();
  const good = fixture(t);
  const withHead = { verdict: emit(good), line: handOver(good) };

  resetDeviceEprCache();
  const fx = fixture(t, { current: null });
  assert.throws(() => deviceHead({ env: fx.env }));
  const lines = [];
  const resolution = deviceEprResolution({ env: fx.env, warn: line => lines.push(line) });
  assert.deepEqual([resolution.command, resolution.source], ['epr', 'path']);
  assert.equal(lines.length, 1);
  assert.match(lines[0], /^\[device-epr\] this device's head epr could not be read \(.*\); device acts use the PATH launcher \(the session pin applies\)$/);
  // Same answers as with a head: nothing is failed for this reason alone.
  assert.deepEqual({ verdict: emit(fx), line: handOver(fx) }, withHead);
  assert.equal(ran(fx, 'pin', 'flow note').length, 1);
  assert.equal(ran(fx, 'pin', 'flow intend --misses').length, 1);
  assert.deepEqual(fx.log().filter(line => line.startsWith('head ')), []);
  // The CLI exits 0 and prints nothing, so a shell caller keeps its own PATH resolution.
  const cli = spawnSync(process.execPath, [join(HERE, 'device-epr.mjs')], { env: fx.env, encoding: 'utf8' });
  assert.deepEqual([cli.status, cli.stdout], [0, '']);
  assert.match(cli.stderr, /could not be read/);
  // With EPR_BIN set and no head, EPR_BIN is used exactly as before.
  assert.equal(resolveDeviceEpr({ env: { ...fx.env, EPR_BIN: fx.pin } }).command, fx.pin);
});

test('harm (iv)/(vi): the resolver asks once per process, however many device acts run', t => {
  resetDeviceEprCache();
  const fx = fixture(t);
  emit(fx);
  siblingVerdicts(project, 'closure', { root: fx.root, env: fx.env, spawn: spawnIn(fx) });
  handOver(fx);
  assert.equal(deviceEpr({ env: fx.env }), fx.head);
  assert.equal(fx.log().filter(line => line.startsWith('launcher bin resolve')).length, 1);
  // A different environment is a different question.
  assert.equal(deviceEpr({ env: { ...fx.env, EPR_BIN: fx.newer } }), fx.newer);
  // Without the cache every act asks again: the count above can fail.
  resetDeviceEprCache();
  deviceEpr({ env: fx.env });
  assert.equal(fx.log().filter(line => line.startsWith('launcher bin resolve')).length, 2);
});

test('harm (iv): a resolution is bounded: a launcher that never answers costs the bound, then the PATH fallback', t => {
  const fx = fixture(t, { launcher: false });
  script(join(fx.root, 'bin', 'epr'), 'sleep 30');
  const started = Date.now();
  const hung = (command, args, options) => spawnSync(command, args, { ...options, timeout: 200 });
  const resolution = resolveDeviceEpr({ env: fx.env, spawn: hung });
  assert.ok(Date.now() - started < 2000, 'the session question is killed at its timeout');
  assert.deepEqual([resolution.command, resolution.source], ['epr', 'path']);
});
