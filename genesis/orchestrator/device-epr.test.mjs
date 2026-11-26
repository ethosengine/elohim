// Device acts run the device's head epr; session acts keep the session's pin (device-epr.mjs).
//
// A fake bin store holds an old release (the session's pin) and a newer head, each a stub that
// appends "<which> <args>" to one log, and a PATH launcher that answers `bin resolve` with the
// pin and execs the pin for everything else, as bin-launch does for a pinned session.
import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmodSync, copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  deviceEpr, deviceEprResolution, deviceHead, resetDeviceEprCache, resolveDeviceEpr,
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

test('a hand-over raises its intent and posts its misses through the head, not the pin', t => {
  resetDeviceEprCache();
  const fx = fixture(t);
  const [line, crossing, ...rest] = handOver(fx).split('\n');
  assert.match(line, /^hand-over: posted 1 misses on intent bafyintent: orchestrator$/);
  // The crossing read is a device act too; the fixture's epr prints no status, which is one line.
  assert.match(crossing, /^\[pre-push\] hand-over: posted locally; `epr peer status` could not be read within 5 s /);
  assert.deepEqual(rest, []);
  assert.equal(ran(fx, 'head', 'peer status').length, 1);
  assert.equal(ran(fx, 'pin', 'peer status').length, 0);
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

test('a reading append is a session act and runs the pin, beside a device act on the head', t => {
  resetDeviceEprCache();
  const fx = fixture(t);
  assert.equal(emit(fx), true);
  appendGateReadings([{ run_id: 'run-1', step: 's' }], { root: fx.root, env: fx.env, spawn: spawnIn(fx), warn: () => {}, write: () => {} });
  assert.equal(ran(fx, 'pin', 'flow reading append').length, 1);
  assert.equal(ran(fx, 'pin', 'flow report reading').length, 1);
  assert.deepEqual(ran(fx, 'head', 'reading'), []);
});

test('the session-act call sites still name the session epr and import no resolver', () => {
  const runner = readFileSync(join(HERE, 'gate-runner.mjs'), 'utf8');
  // reading append, reading report, the attested pin-check observation, the cycle observation.
  assert.equal(runner.split("env.EPR_BIN || 'epr'").length - 1, 4);
  assert.equal(runner.split('deviceEpr(').length - 1, 1);
  const live = readFileSync(join(HERE, 'gate-live.mjs'), 'utf8');
  assert.ok(!live.includes('device-epr'), 'gate-live resolves session identity and is left alone');
  for (const file of ['gate-evidence.mjs', 'handover-record.mjs', 'gate-runner.mjs', 'gate-preflight.mjs']) {
    assert.ok(!/function\s+(deviceHead|resolveDeviceEpr)\b/.test(readFileSync(join(HERE, file), 'utf8')), `${file} holds no second resolver`);
  }
});

const promote = (fx, cid) => writeFileSync(join(fx.store, 'epr', 'current.json'), JSON.stringify({ cid }));

test('ruling: EPR_BIN is not consulted: /bin/true, an older release and a later one all leave the head in use', t => {
  resetDeviceEprCache();
  const fx = fixture(t);
  for (const candidate of ['/bin/true', fx.pin, fx.newer]) {
    const lines = [];
    const resolution = deviceEprResolution({ env: { ...fx.env, EPR_BIN: candidate }, warn: line => lines.push(line) });
    assert.deepEqual(resolution, { command: fx.head, source: 'head', why: null }, candidate);
    assert.deepEqual(lines, []);
  }
  fx.env.EPR_BIN = fx.pin;
  assert.equal(emit(fx), true);
  assert.equal(ran(fx, 'head', 'flow note').length, 1);
  assert.deepEqual(fx.log().filter(line => line.startsWith('pin ')), []);
  // The CLI the shell and Python call sites run prints the head, whatever EPR_BIN names.
  const cli = spawnSync(process.execPath, [join(HERE, 'device-epr.mjs')], { env: { ...fx.env, EPR_BIN: '/bin/true' }, encoding: 'utf8' });
  assert.deepEqual([cli.status, cli.stdout, cli.stderr], [0, `${fx.head}\n`, '']);
});

test('ruling: no downgrade judgment: a re-ingested older head is used though the session epr is newer, and the session is never asked', t => {
  resetDeviceEprCache();
  // The device rolled back to OLD and re-ingested it, so its record is dated after the session's.
  const fx = fixture(t, { current: OLD, sessionSays: 'newer' });
  writeFileSync(join(fx.store, 'epr', OLD, 'record.json'), JSON.stringify({ cid: OLD, ingestedAt: '2026-11-27T00:00:00Z' }));
  assert.equal(deviceEpr({ env: fx.env }), fx.pin);
  assert.equal(emit(fx), true);
  // The OLD release's stub logs as "pin": it is the head here, and it ran the crossing.
  assert.equal(ran(fx, 'pin', 'flow note').length, 1);
  assert.deepEqual(fx.log().filter(line => line.startsWith('newer ') || line.startsWith('launcher ')), []);
  // And the other direction: the same store with the record dated before the session's.
  writeFileSync(join(fx.store, 'epr', OLD, 'record.json'), JSON.stringify({ cid: OLD, ingestedAt: '2026-11-01T00:00:00Z' }));
  assert.equal(deviceEpr({ env: fx.env }), fx.pin);
});

test('containment: a head that resolves outside the bin store, or is no regular executable file, is an unreadable head', t => {
  const fx = fixture(t);
  const outside = join(fx.root, 'outside', 'epr');
  script(outside, 'exit 0');
  const refused = (cid, make) => {
    mkdirSync(join(fx.store, 'epr', cid), { recursive: true });
    make(join(fx.store, 'epr', cid, 'epr'));
    promote(fx, cid);
    const resolution = resolveDeviceEpr({ env: fx.env });
    assert.deepEqual([resolution.command, resolution.source], ['epr', 'path'], cid);
    assert.throws(() => deviceHead({ env: fx.env }));
    return resolution.why;
  };
  assert.match(refused('bafylinkout', path => symlinkSync(outside, path)), /resolves outside the bin store/);
  assert.match(refused('bafylinktrue', path => symlinkSync('/bin/true', path)), /resolves outside the bin store/);
  assert.match(refused('bafydir', path => mkdirSync(path)), /is not a regular file/);
  refused('bafynoexec', path => writeFileSync(path, '#!/bin/sh\n', { mode: 0o644 }));
  refused('bafyabsent', () => {});
  // A cid that climbs out of the store is no cid.
  writeFileSync(join(fx.store, 'epr', 'current.json'), JSON.stringify({ cid: '../../outside' }));
  assert.match(resolveDeviceEpr({ env: fx.env }).why, /invalid head/);
  // A symlink that stays inside the store is the store's own business and is followed.
  mkdirSync(join(fx.store, 'epr', 'bafyalias'), { recursive: true });
  symlinkSync(fx.head, join(fx.store, 'epr', 'bafyalias', 'epr'));
  promote(fx, 'bafyalias');
  assert.deepEqual(resolveDeviceEpr({ env: fx.env }), { command: fx.head, source: 'head', why: null });
  // With no current.json the `current` symlink names the head, under the same containment.
  rmSync(join(fx.store, 'epr', 'current.json'));
  symlinkSync(join(fx.store, 'epr', NEWER), join(fx.store, 'epr', 'current'));
  assert.equal(resolveDeviceEpr({ env: fx.env }).command, fx.newer);
  rmSync(join(fx.store, 'epr', 'current'));
  symlinkSync(dirname(outside), join(fx.store, 'epr', 'current'));
  assert.equal(resolveDeviceEpr({ env: fx.env }).source, 'path');
});

test('an unreadable head falls back with one line, and the acts answer as before', t => {
  resetDeviceEprCache();
  const good = fixture(t);
  const withHead = { verdict: emit(good), line: handOver(good) };

  resetDeviceEprCache();
  const fx = fixture(t, { current: null });
  assert.throws(() => deviceHead({ env: fx.env }));
  const lines = [];
  const warn = line => lines.push(line);
  const resolution = deviceEprResolution({ env: fx.env, warn });
  assert.deepEqual([resolution.command, resolution.source], ['epr', 'path']);
  deviceEprResolution({ env: fx.env, warn });
  assert.equal(lines.length, 1, 'the line is printed once per process');
  assert.match(lines[0], /^\[device-epr\] this device's head epr could not be read \(.*\); device acts use the PATH launcher \(the session pin applies\)$/);
  // Same answers as with a head: nothing is failed for this reason alone.
  assert.deepEqual({ verdict: emit(fx), line: handOver(fx) }, withHead);
  assert.equal(ran(fx, 'pin', 'flow note').length, 1);
  assert.equal(ran(fx, 'pin', 'flow intend --misses').length, 1);
  assert.deepEqual(fx.log().filter(line => line.startsWith('head ')), []);
  // The CLI exits 0 and prints nothing, so a shell caller keeps its own resolution.
  const cli = spawnSync(process.execPath, [join(HERE, 'device-epr.mjs')], { env: fx.env, encoding: 'utf8' });
  assert.deepEqual([cli.status, cli.stdout], [0, '']);
  assert.equal(cli.stderr.trim().split('\n').length, 1);
  // The fallback is the expression the call sites used before, EPR_BIN included, and says so.
  const before = resolveDeviceEpr({ env: { ...fx.env, EPR_BIN: fx.pin } });
  assert.equal(before.command, fx.pin);
  assert.match(before.why, /device acts use EPR_BIN=/);
});

test('a head promoted mid-process is used by the next device act', t => {
  resetDeviceEprCache();
  const fx = fixture(t);
  assert.equal(emit(fx), true);
  assert.equal(ran(fx, 'head', 'flow note').length, 1);
  promote(fx, NEWER);
  assert.equal(deviceEpr({ env: fx.env }), fx.newer);
  assert.equal(emit(fx), true);
  assert.equal(ran(fx, 'newer', 'flow note').length, 1);
  assert.equal(ran(fx, 'head', 'flow note').length, 1, 'the previous head ran nothing more');
  // Same bytes under a new name: the pointer, not the file, is what moved.
  mkdirSync(join(fx.store, 'epr', 'bafycopy'), { recursive: true });
  copyFileSync(fx.newer, join(fx.store, 'epr', 'bafycopy', 'epr'));
  promote(fx, 'bafycopy');
  assert.equal(deviceEpr({ env: fx.env }), join(fx.store, 'epr', 'bafycopy', 'epr'));
});

test('cost: a resolution runs no subprocess and many of them stay cheap', t => {
  const fx = fixture(t);
  const started = process.hrtime.bigint();
  for (let i = 0; i < 200; i += 1) assert.equal(deviceEpr({ env: fx.env }), fx.head);
  const each = Number(process.hrtime.bigint() - started) / 1e6 / 200;
  assert.ok(each < 20, `${each.toFixed(2)} ms per resolution`);
  assert.deepEqual(fx.log(), [], 'no launcher and no release was executed to resolve');
});

test('a verdict crossing the bound killed says so, and that the append is unconfirmed', t => {
  resetDeviceEprCache();
  const fx = fixture(t);
  const lines = [];
  const killed = (command, args, options) => (args.includes('gate-verdict@1')
    ? { status: null, signal: 'SIGTERM', error: Object.assign(new Error('spawnSync epr ETIMEDOUT'), { code: 'ETIMEDOUT' }), stdout: '', stderr: '' }
    : spawnIn(fx)(command, args, options));
  assert.equal(emitGateVerdict(project, 'closure', {
    success: true, seconds: 1, commit: SHA, put: {}, root: fx.root, env: fx.env, spawn: killed, warn: line => lines.push(line),
  }), false);
  assert.deepEqual(lines, ['[gate] orchestrator verdict crossing: not emitted (killed by the 5 s bound (SIGTERM); the append is unconfirmed)\n']);
});

// The store is found beside the launcher a shell would run, wherever it was installed. Every test
// above sets BIN_STORE, so this lookup had no test: on 2026-11-25 a device whose launcher is
// ~/.cargo/bin/epr was read as having no head, and its hand-over's misses were dropped.
function launcherStore(t) {
  const root = mkdtempSync(join(tmpdir(), 'device-epr-launcher-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const binStore = join(root, 'share/elohim/bin-store');
  script(join(binStore, 'bin/bin-launch'), 'exit 0');
  script(join(binStore, 'epr', HEAD, 'epr'), 'exit 0');
  writeFileSync(join(binStore, 'epr/current.json'), JSON.stringify({ cid: HEAD }));
  return { root, binStore, head: join(binStore, 'epr', HEAD, 'epr') };
}

test('launcher: the store is found beside an epr launcher on PATH that is not at ~/.local/bin', t => {
  const fx = launcherStore(t);
  mkdirSync(join(fx.root, '.cargo/bin'), { recursive: true });
  symlinkSync(join(fx.binStore, 'bin/bin-launch'), join(fx.root, '.cargo/bin/epr'));
  const env = { PATH: `${join(fx.root, 'empty')}:${join(fx.root, '.cargo/bin')}:/usr/bin`, HOME: fx.root };
  assert.equal(deviceHead({ env }).path, fx.head);
  assert.equal(resolveDeviceEpr({ env }).source, 'head');
});

test('launcher: an epr on PATH that is not a bin-store launcher is passed over for ~/.local/bin/epr', t => {
  const fx = launcherStore(t);
  script(join(fx.root, 'plain/epr'), 'exit 0');
  mkdirSync(join(fx.root, '.local/bin'), { recursive: true });
  symlinkSync(join(fx.binStore, 'bin/bin-launch'), join(fx.root, '.local/bin/epr'));
  const env = { PATH: `${join(fx.root, 'plain')}:/usr/bin`, HOME: fx.root };
  assert.equal(deviceHead({ env }).path, fx.head);
});

test('launcher: no launcher anywhere is an unreadable head, said in one line, and falls back', t => {
  const fx = launcherStore(t);
  script(join(fx.root, 'plain/epr'), 'exit 0');
  const env = { PATH: `${join(fx.root, 'plain')}:/usr/bin`, HOME: fx.root };
  assert.throws(() => deviceHead({ env }), /no epr launcher/);
  const r = resolveDeviceEpr({ env });
  assert.equal(r.source, 'path');
  assert.equal(r.command, 'epr');
  assert.match(r.why, /no epr launcher/);
});
