import { describe, test } from 'node:test';
import { strict as assert } from 'node:assert';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { attributor, createCostSampler, lineSplitter, tapChildEnv } from './gate-tap.mjs';

const TAP = resolve(dirname(fileURLToPath(import.meta.url)), 'gate-tap.mjs');

// Two failing cargo tests, a panic for each, and the binary's verdict, the way cargo prints them.
const CARGO_RED = [
  'running 3 tests',
  'test ok_one ... ok',
  'test parses_head ... FAILED',
  'test folds_twice ... FAILED',
  '',
  'failures:',
  '',
  '---- parses_head stdout ----',
  "thread 'parses_head' panicked at src/head.rs:41:9:",
  'assertion `left == right` failed',
  '',
  '---- folds_twice stdout ----',
  "thread 'folds_twice' panicked at src/fold.rs:88:5:",
  'already folded',
  '',
  'failures:',
  '    folds_twice',
  '    parses_head',
  '',
  'test result: FAILED. 1 passed; 2 failed; 0 ignored',
].join('\n');

function tap(t, script, { run = true } = {}) {
  const dir = mkdtempSync(resolve(tmpdir(), 'gate-tap-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const runPath = resolve(dir, 'run.jsonl');
  if (run) writeFileSync(runPath, '');
  // The pre-push hook and gate-runner export GATE_REMAINING, which makes the first red append an
  // outlook event. A fixture tap is not a gate in that run: inheriting it made this suite count
  // events differently inside a push (2 !== 1) than when run alone.
  const env = { ...process.env };
  delete env.GATE_REMAINING;
  env.GATE_LIVE_SIBLING = 'off';
  const result = spawnSync(process.execPath, [TAP, '--project', 'fixture', ...(run ? ['--run', runPath] : []), '--', 'bash', '-c', script],
    { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe', 'pipe'], timeout: 15000, env });
  const events = run ? readFileSync(runPath, 'utf8').split('\n').filter(Boolean).map(line => JSON.parse(line)) : [];
  const summary = JSON.parse(String(result.output[3]).trim());
  return { result, events, summary };
}

describe('gate-tap', () => {
  test('forwards bytes unchanged and keeps the exit status as the verdict', t => {
    const script = `cat <<'EOF'\n${CARGO_RED}\nEOF\necho "warning: on stderr" >&2\nexit 101`;
    const { result, events, summary } = tap(t, script);
    assert.equal(result.status, 101);
    assert.equal(result.stdout, `${CARGO_RED}\n`);
    assert.equal(result.stderr, 'warning: on stderr\n');
    const failures = events.filter(e => e.kind === 'failure');
    assert.deepEqual(failures.map(e => e.failure.locus.test), ['parses_head', 'folds_twice']);
    assert.equal(summary.failures, 2);
    assert.equal(summary.signals.assertionFailed, true);
    assert.equal(summary.signals.testOutput, true);
  });

  test('a green exit stays green whatever the output looked like', t => {
    const { result, summary } = tap(t, 'echo "test looks_bad ... FAILED"; exit 0');
    assert.equal(result.status, 0);
    assert.equal(summary.failures, 1);
  });

  test('a red exit with nothing recognized stays red', t => {
    const { result, events, summary } = tap(t, 'echo "something odd"; exit 3');
    assert.equal(result.status, 3);
    assert.equal(events.length, 0);
    assert.equal(summary.failures, 0);
    assert.equal(summary.firstRedSeconds, null);
  });

  test('runs with no live run open and still reports on fd 3', t => {
    const { result, summary } = tap(t, 'echo "test x ... FAILED"; exit 1', { run: false });
    assert.equal(result.status, 1);
    assert.equal(summary.failures, 1);
  });

  test('carries a signal death through', t => {
    const { result } = tap(t, 'kill -TERM $$');
    assert.equal(result.signal, 'SIGTERM');
  });

  test('refuses a missing command with usage', () => {
    const result = spawnSync(process.execPath, [TAP, '--project', 'x'], { encoding: 'utf8' });
    assert.equal(result.status, 2);
    assert.match(result.stderr, /usage: gate-tap/);
  });

  test('splits lines across chunk boundaries and keeps a trailing partial', () => {
    const lines = [];
    const split = lineSplitter(line => lines.push(line));
    split.write(Buffer.from('test a ... FA'));
    split.write(Buffer.from('ILED\nsecond\nthi'));
    split.end();
    split.write(Buffer.from('rd'));
    split.end();
    assert.deepEqual(lines, ['test a ... FAILED', 'second', 'thi', 'rd']);
  });

  test('keeps colour only for a terminal, and never over a declared value', () => {
    assert.deepEqual(tapChildEnv({ A: '1' }, false), { A: '1' });
    assert.equal(tapChildEnv({}, true).CARGO_TERM_COLOR, 'always');
    assert.equal(tapChildEnv({ CARGO_TERM_COLOR: 'never' }, true).CARGO_TERM_COLOR, 'never');
  });

  test('caps an unterminated line at 64 KiB and drops its tail through the next newline', () => {
    const lines = [];
    const split = lineSplitter(line => lines.push(line));
    const prefix = 'x'.repeat(64 * 1024);
    split.write(Buffer.from(prefix));
    assert.deepEqual(lines, []);
    split.write(Buffer.from('overflow'));
    assert.deepEqual(lines, [prefix]);
    split.write(Buffer.from('test dropped ... FAILED'));
    split.write(Buffer.from('\nnext\npartial'));
    split.end();
    assert.deepEqual(lines, [prefix, 'next', 'partial']);
  });

  test('caps partials in bytes across UTF-8 chunks and resets dropping at end', () => {
    const lines = [];
    const split = lineSplitter(line => lines.push(line));
    const prefix = 'é'.repeat(32 * 1024);
    const bytes = Buffer.from(prefix);
    split.write(bytes.subarray(0, 3));
    split.write(bytes.subarray(3));
    split.write(Buffer.from('discard'));
    split.end();
    split.write(Buffer.from('fresh\n'));
    split.end();
    assert.deepEqual(lines, [prefix, 'fresh']);
  });

  test('truncates only the recognizer input while forwarding oversized lines unchanged', t => {
    const { result, events } = tap(t,
      `node -e 'const line = "not ok 1 - " + "x".repeat(70000) + "\\n"; process.stdout.write(line); process.stderr.write(line);'`);
    const line = 'not ok 1 - ' + 'x'.repeat(70000) + '\n';
    assert.equal(result.status, 0);
    assert.equal(result.stdout.length, line.length);
    assert.equal(result.stderr.length, line.length);
    assert.ok(result.stdout === line, 'stdout bytes are unchanged');
    assert.ok(result.stderr === line, 'stderr bytes are unchanged');
    assert.equal(events.length, 1);
    assert.equal(Buffer.byteLength(events[0].failure.excerpt), 64 * 1024);
  });
});

describe('gate-tap attribution', () => {
  function journal(t) {
    const root = mkdtempSync(resolve(tmpdir(), 'gate-tap-attr-'));
    t.after(() => rmSync(root, { recursive: true, force: true }));
    mkdirSync(resolve(root, '.git')); // root discovery needs only the marker, not git
    mkdirSync(resolve(root, 'crate/src'), { recursive: true });
    writeFileSync(resolve(root, 'crate/src/head.rs'), 'fn f() {}\n');
    mkdirSync(resolve(root, '.eprfs/status/lsp'), { recursive: true });
    mkdirSync(resolve(root, '.eprfs/status/gate-live/runs'), { recursive: true });
    const run = resolve(root, '.eprfs/status/gate-live/runs/r.jsonl');
    const openedAt = new Date(Date.now() - 60_000).toISOString();
    writeFileSync(run, `${JSON.stringify({ v: 1, run: 'r', seq: 0, at: openedAt, kind: 'run-open', session: 'me' })}\n`);
    const edit = (session, path) => JSON.stringify({ v: 1, at: new Date(Date.now() - 30_000).toISOString(),
      session, ownerPid: 1, ownerStartTime: '1', path, tool: 'Edit' });
    return { root, run, edit };
  }

  test('a failure in a file this session edited is labelled mine; another session\'s is other', t => {
    const { root, run, edit } = journal(t);
    writeFileSync(resolve(root, '.eprfs/status/lsp/edits.jsonl'), `${edit('me', resolve(root, 'crate/src/head.rs'))}\n`);
    const label = attributor({ run, dir: 'crate', root });
    const failure = { kind: 'failure', project: 'p', failure: { class: 'test', locus: { file: 'src/head.rs', line: 3 } } };
    assert.deepEqual(label(failure).failure.attribution, { label: 'mine', sessions: ['me'] });
    writeFileSync(resolve(root, '.eprfs/status/lsp/edits.jsonl'), `${edit('peer', resolve(root, 'crate/src/head.rs'))}\n`);
    assert.deepEqual(attributor({ run, dir: 'crate', root })(failure).failure.attribution, { label: 'other', sessions: ['peer'] });
  });

  test('a locus that resolves nowhere, or a run with no run-open, passes through unlabelled', t => {
    const { root, run } = journal(t);
    const failure = { kind: 'failure', project: 'p', failure: { class: 'test', locus: { file: 'src/missing.rs' } } };
    assert.equal(attributor({ run, dir: 'crate', root })(failure).failure.attribution, undefined);
    writeFileSync(run, '');
    const head = { kind: 'failure', project: 'p', failure: { class: 'test', locus: { file: 'src/head.rs' } } };
    assert.equal(attributor({ run, dir: 'crate', root })(head).failure.attribution, undefined);
    assert.equal(attributor({ run: null })(head), head);
  });
});


test('cost samples sum only descendants, retain peaks and take cgroup counter deltas after exit', () => {
  const files = new Map([
    ['/proc/20/cgroup', '0::/gate\n'],
    ['/proc/20/status', 'PPid:\t1\nVmRSS:\t10 kB\n'],
    ['/proc/21/status', 'PPid:\t20\nVmRSS:\t30 kB\n'],
    ['/proc/22/status', 'PPid:\t21\nVmRSS:\t60 kB\n'],
    ['/proc/90/status', 'PPid:\t1\nVmRSS:\t99999 kB\n'],
    ['/sys/fs/cgroup/gate/cpu.stat', 'usage_usec 1000000\nthrottled_usec 200000\n'],
    ['/sys/fs/cgroup/gate/memory.events', 'low 0\nhigh 7\n'],
    ['/proc/pressure/io', 'some avg10=99.0\nfull avg10=1.50 avg60=2.00 total=123\n'],
  ]);
  const sampler = createCostSampler(20, {
    readFile: path => { if (!files.has(path)) throw new Error('gone'); return files.get(path); },
    listDir: () => ['90', '22', '21', '20', 'pressure', '999'],
  });
  assert.deepEqual(sampler.sample(), { cpu_s: 0, throttled_s: 0, rss_peak_bytes: 102400,
    memory_high_events: 0, io_full_pct_max: 1.5 });
  files.set('/sys/fs/cgroup/gate/cpu.stat', 'usage_usec 4500000\nthrottled_usec 700000\n');
  files.set('/sys/fs/cgroup/gate/memory.events', 'high 10\n');
  files.set('/proc/pressure/io', 'full avg10=4.25 avg60=1.00\n');
  files.delete('/proc/20/status');
  files.delete('/proc/20/cgroup');
  files.delete('/proc/21/status');
  files.delete('/proc/22/status');
  assert.deepEqual(sampler.sample(), { cpu_s: 3.5, throttled_s: 0.5, rss_peak_bytes: 102400,
    memory_high_events: 3, io_full_pct_max: 4.25 });
  files.set('/proc/pressure/io', 'full avg10=0.01\n');
  assert.equal(sampler.sample().io_full_pct_max, 4.25);
});

test('missing kernel files are unknown, and disappearing processes do not break the tap', () => {
  const sampler = createCostSampler(7, {
    readFile: () => { throw new Error('unavailable'); },
    listDir: () => { throw new Error('unavailable'); },
  });
  assert.deepEqual(sampler.sample(), { cpu_s: null, throttled_s: null, rss_peak_bytes: null,
    memory_high_events: null, io_full_pct_max: null });
});

test('fd 3 carries all five cost axes beside the failure signals', t => {
  const { summary, result } = tap(t, 'sleep 0.05');
  assert.equal(result.status, 0);
  for (const key of ['cpu_s', 'throttled_s', 'rss_peak_bytes', 'memory_high_events', 'io_full_pct_max']) {
    assert.ok(Object.hasOwn(summary, key), key);
    assert.ok(summary[key] === null || Number.isFinite(summary[key]), key);
  }
});

describe('gate-tap sibling notes', () => {
  const GATE_LIVE = resolve(dirname(TAP), 'gate-live.mjs');
  const HERE = 'did:key:z6MkHEREdeviceAAAA';
  const SUB = { cid: 'sub1', at: Date.now() / 1000, from_device: 'did:key:z6MkOTHER', session: 'shem', to: null,
    text: JSON.stringify({ kind: 'gate-subscription@1', subscriber: 'shem', device: HERE, ref: '*',
      intent: 'default', until: '2999-01-01T00:00:00.000Z' }) };
  const THREE_RED = 'echo "test a ... FAILED"; echo "test b ... FAILED"; echo "test c ... FAILED"; exit 101';

  // sleepS stalls every berth call (inbox and say); saySleepS stalls only `say` (E4 finding 8:
  // a stall in inbox discovery means say is never reached, so it tests nothing about say).
  function setup(t, { sleepS = 0, saySleepS = 0, exit = 0, notes = [SUB], withBerth = true } = {}) {
    const dir = mkdtempSync(resolve(tmpdir(), 'gate-tap-sib-'));
    t.after(() => rmSync(dir, { recursive: true, force: true }));
    writeFileSync(resolve(dir, 'notes.json'), JSON.stringify(notes));
    const bin = resolve(dir, 'berth-stub');
    writeFileSync(bin, `#!/bin/bash\n${sleepS ? `sleep ${sleepS}\n` : ''}if [ "$1" = say ]; then ${saySleepS ? `sleep ${saySleepS}; ` : ''}printf '%s\\n' "$*" >> "${dir}/says.log"; else cat "${dir}/notes.json"; fi\nexit ${exit}\n`, { mode: 0o755 });
    const runPath = resolve(dir, 'run.jsonl');
    writeFileSync(runPath, `${JSON.stringify({ v: 1, run: 'run', seq: 0, at: new Date().toISOString(), kind: 'run-open', candidate: { sha: 'x', target: 't' }, session: 'gate-session', ref: 'refs/heads/dev' })}\n`);
    const env = { ...process.env, GATE_LIVE_BERTH_BIN: withBerth ? bin : resolve(dir, 'no-berth'),
      GATE_LIVE_DEVICE: HERE, EPR_BIN: resolve(dir, 'missing-epr'), GATE_REMAINING: 'app other' };
    delete env.GATE_LIVE_SIBLING;
    return { dir, runPath, env };
  }

  const says = dir => { try { return readFileSync(resolve(dir, 'says.log'), 'utf8').split('\n').filter(Boolean); } catch { return []; } };
  const wait = async (dir, n) => {
    for (let i = 0; i < 80 && says(dir).length < n; i++) await new Promise(r => setTimeout(r, 50));
    await new Promise(r => setTimeout(r, 250));
  };
  const runTap = (env, runPath, script) => {
    const started = Date.now();
    const result = spawnSync(process.execPath, [TAP, '--project', 'fixture', '--run', runPath, '--', 'bash', '-c', script],
      { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe', 'pipe'], timeout: 20000, env });
    return { result, ms: Date.now() - started };
  };

  test('three reds then close send exactly the first red and the final, facts only', async t => {
    const { dir, runPath, env } = setup(t);
    const { result } = runTap(env, runPath, THREE_RED);
    assert.equal(result.status, 101);
    const close = spawnSync(process.execPath, [GATE_LIVE, 'close', '--run', runPath, '--outcome', 'failed'], { encoding: 'utf8', env, timeout: 10000 });
    assert.equal(close.status, 0, close.stderr);
    await wait(dir, 2);
    const sent = says(dir);
    assert.equal(sent.length, 2, sent.join('\n'));
    const bodies = sent.map(line => JSON.parse(line.slice(line.indexOf('{'))));
    const red = bodies.find(b => b.kind === 'gate-red@1');
    const final = bodies.find(b => b.kind === 'gate-final@1');
    assert.equal(red.gate, 'fixture');
    assert.equal(red.class, 'test');
    assert.equal(red.ordinal, 1);
    assert.equal(red.ref, 'refs/heads/dev');
    for (const line of sent) assert.ok(!line.includes('FAILED') && !line.includes(dir) && !line.includes('stop'), line);
    assert.equal(final.verdict, 'failed');
    assert.equal(final.failures, 3);
    for (const line of sent) assert.match(line, /^say --session gate-session --to shem \{/);
  });

  test('a berth that fails or is missing never changes the gate status or its output', t => {
    for (const options of [{ exit: 1 }, { withBerth: false }]) {
      const { runPath, env } = setup(t, options);
      const { result } = runTap(env, runPath, THREE_RED);
      assert.equal(result.status, 101);
      assert.equal(result.stdout, 'test a ... FAILED\ntest b ... FAILED\ntest c ... FAILED\n');
    }
  });

  test('a hung berth say leaves the gate fast and its status intact, and the sender is reaped and named', async t => {
    const baseline = setup(t, { notes: [] });
    const off = { ...baseline.env, GATE_LIVE_SIBLING: 'off' };
    const base = runTap(off, baseline.runPath, THREE_RED);
    const hung = setup(t, { saySleepS: 30 });
    hung.env.GATE_LIVE_DELIVER_MS = '700';
    const measured = runTap(hung.env, hung.runPath, THREE_RED);
    assert.equal(base.result.status, 101);
    assert.equal(measured.result.status, 101, 'the gate status is the child\'s');
    assert.equal(measured.result.stdout, base.result.stdout);
    const added = measured.ms - base.ms;
    console.log(`hung-say overhead: ${added} ms (baseline ${base.ms} ms, with a hung say ${measured.ms} ms)`);
    assert.ok(added < 2000, `added ${added} ms`);
    // The detached sender hits its deadline, kills berth's process group and records why.
    let deliveries = [];
    for (let i = 0; i < 80 && !deliveries.length; i++) {
      await new Promise(r => setTimeout(r, 100));
      deliveries = readFileSync(hung.runPath, 'utf8').split('\n').filter(Boolean).map(l => JSON.parse(l))
        .filter(e => e.kind === 'sibling-delivery');
    }
    assert.equal(deliveries.length, 1, 'one red reaches one subscriber');
    assert.equal(deliveries[0].ok, false);
    assert.match(deliveries[0].why, /exceeded 700 ms/);
    const left = spawnSync('pgrep', ['-f', resolve(hung.dir, 'berth-stub')], { encoding: 'utf8' });
    assert.equal(left.stdout.trim(), '', 'no berth process outlives its deadline');
    assert.deepEqual(says(hung.dir), [], 'the hung say never wrote');
  });
});
