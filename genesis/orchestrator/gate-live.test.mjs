import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fs, {
  appendFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  utimesSync,
  writeFileSync,
} from 'node:fs';
import { syncBuiltinESMExports } from 'node:module';
import { tmpdir } from 'node:os';
import { basename, isAbsolute, join, relative } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

import * as live from './gate-live.mjs';

const {
  liveRoot,
  resolveSession,
  openRun,
  appendEvent,
  closeRun,
  readRun,
  findRun,
  formatEvent,
  follow,
} = live;

const SHA = '0123456789abcdef0123456789abcdef01234567';
const AT = '2026-10-26T12:00:00.000Z';
const now = () => new Date(AT);
const SCRIPT = fileURLToPath(new URL('./gate-live.mjs', import.meta.url));

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), 'gate-live-test-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  return root;
}

function open(root, options = {}) {
  return openRun({ root, sha: SHA, target: 'dev', now, ...options });
}

function cli(root, args) {
  const result = spawnSync(process.execPath, [SCRIPT, ...args], {
    cwd: root,
    encoding: 'utf8',
    timeout: 5000,
    env: {
      ...process.env,
      EPR_BIN: join(root, 'missing-epr'),
      ELOHIM_SESSION_ID: '',
    },
  });
  assert.ifError(result.error);
  return result;
}

test('exports only the public run-log functions', () => {
  assert.deepEqual(Object.keys(live).sort(), [
    'appendEvent',
    'closeRun',
    'findRun',
    'follow',
    'formatEvent',
    'liveRoot',
    'openRun',
    'readRun',
    'resolveSession',
  ]);
});

test('liveRoot resolves the main checkout through git-common-dir', (t) => {
  const root = fixture(t);
  const worktree = join(root, '.worktrees', 'slice');
  const calls = [];
  const result = liveRoot(worktree, {
    git: (...args) => {
      calls.push(args);
      return `${join(root, '.git')}\n`;
    },
  });

  assert.equal(result, join(root, '.eprfs', 'status', 'gate-live'));
  assert.equal(calls.length, 1);
  assert.equal(calls[0][0], 'git');
  assert.deepEqual(calls[0][1], [
    'rev-parse',
    '--path-format=absolute',
    '--git-common-dir',
  ]);
  assert.equal(calls[0][2].cwd, worktree);
  assert.equal(calls[0][2].encoding, 'utf8');
  assert.equal(calls[0][2].stdio, 'pipe');
  assert.equal(calls[0][2].timeout, 2000);
});

test('liveRoot falls back to an absolute root when git fails', (t) => {
  const root = fixture(t);
  const result = liveRoot(root, {
    git: () => {
      throw new Error('not a git repository');
    },
  });
  assert.equal(result, join(root, '.eprfs', 'status', 'gate-live'));
  assert.equal(isAbsolute(result), true);
});

test('resolveSession prefers the epr session and bounds the subprocess', () => {
  const env = { EPR_BIN: '/custom/epr', ELOHIM_SESSION_ID: 'fallback' };
  const calls = [];
  const session = resolveSession({
    env,
    spawn: (...args) => {
      calls.push(args);
      return {
        status: 0,
        stdout: JSON.stringify({ session: 'current-session', source: 'ancestry' }),
      };
    },
  });

  assert.equal(session, 'current-session');
  assert.deepEqual(calls, [[
    '/custom/epr',
    ['session', 'id', '--json'],
    { env, encoding: 'utf8', stdio: 'pipe', timeout: 2000 },
  ]]);
});

test('resolveSession falls back on none, bad JSON, failures, and missing executables', () => {
  const env = { ELOHIM_SESSION_ID: 'fallback' };
  const spawns = [
    () => ({ status: 0, stdout: '{"session":"ignored","source":"none"}' }),
    () => ({ status: 0, stdout: '{"source":"ancestry"}' }),
    () => ({ status: 0, stdout: 'not JSON' }),
    () => ({ status: 1, stdout: '{"session":"ignored","source":"ancestry"}' }),
    () => ({ status: null, error: new Error('timed out') }),
    () => {
      throw new Error('ENOENT');
    },
  ];

  for (const spawn of spawns) {
    assert.equal(resolveSession({ env, spawn }), 'fallback');
    assert.equal(resolveSession({ env: {}, spawn }), null);
  }
});

test('resolveSession uses epr by default and returns null without a session', () => {
  const session = resolveSession({
    env: {},
    spawn: (command, args) => {
      assert.equal(command, 'epr');
      assert.deepEqual(args, ['session', 'id', '--json']);
      return { status: 0, stdout: '{"source":"none"}' };
    },
  });
  assert.equal(session, null);
});

test('openRun writes run-open and a sanitized absolute session pointer', (t) => {
  const root = fixture(t);
  const session = 'session/alice:desktop';
  const result = open(root, { session });

  assert.match(result.runId, /^20261026T120000Z-0123456789ab-[0-9a-f]{6}$/);
  assert.equal(isAbsolute(result.path), true);
  assert.equal(
    result.path,
    join(root, '.eprfs', 'status', 'gate-live', 'runs', `${result.runId}.jsonl`),
  );
  assert.deepEqual(readRun(result.path).events, [{
    v: 1,
    run: result.runId,
    seq: 0,
    at: AT,
    kind: 'run-open',
    candidate: { sha: SHA, target: 'dev' },
    session,
  }]);
  assert.equal(
    readFileSync(
      join(root, '.eprfs', 'status', 'gate-live', 'by-session', 'session_alice_desktop'),
      'utf8',
    ),
    result.path,
  );
  assert.equal(readFileSync(result.path, 'utf8').endsWith('\n'), true);
});

test('openRun overwrites the session pointer for a later run', (t) => {
  const root = fixture(t);
  const first = open(root, { session: 'same.session-1' });
  const second = open(root, {
    session: 'same.session-1',
    now: () => new Date('2026-10-26T12:00:01.000Z'),
  });

  assert.notEqual(first.path, second.path);
  assert.equal(findRun({ root, session: 'same.session-1' }), second.path);
  assert.equal(existsSync(first.path), true);
});

test('openRun uses nosha and creates no pointer for a null session', (t) => {
  const root = fixture(t);
  const result = open(root, { sha: null, session: null });

  assert.match(result.runId, /^20261026T120000Z-nosha-[0-9a-f]{6}$/);
  assert.equal(
    existsSync(join(root, '.eprfs', 'status', 'gate-live', 'by-session')),
    false,
  );
});

test('openRun prunes to the newest N files by name, ignoring unrelated entries', (t) => {
  const root = fixture(t);
  const newest = open(root, {
    keep: 2,
    now: () => new Date('2026-10-26T12:00:02.000Z'),
  });
  const oldest = open(root, { keep: 2 });
  closeRun(oldest.path, 'passed', { now });
  const runs = join(root, '.eprfs', 'status', 'gate-live', 'runs');
  writeFileSync(join(runs, 'notes.txt'), 'leave alone');
  mkdirSync(join(runs, 'directory.jsonl'));

  const middle = open(root, {
    keep: 2,
    now: () => new Date('2026-10-26T12:00:01.000Z'),
  });

  assert.equal(existsSync(oldest.path), false);
  assert.equal(existsSync(newest.path), true);
  assert.equal(existsSync(middle.path), true);
  assert.equal(readFileSync(join(runs, 'notes.txt'), 'utf8'), 'leave alone');
  assert.equal(existsSync(join(runs, 'directory.jsonl')), true);
  assert.deepEqual(
    readdirSync(runs, { withFileTypes: true })
      .filter((entry) => entry.isFile() && entry.name.endsWith('.jsonl'))
      .map((entry) => entry.name)
      .sort(),
    [basename(middle.path), basename(newest.path)].sort(),
  );
});

test('openRun preserves active runs outside keep until their mtime is stale', (t) => {
  const root = fixture(t);
  const active = open(root);
  const reopened = open(root);
  closeRun(reopened.path, 'passed', { now });
  appendEvent(reopened.path, { kind: 'gate-verdict' }, { now });
  const stale = open(root);
  const closed = open(root);
  closeRun(closed.path, 'passed', { now });
  const pruningAt = new Date('2026-10-27T12:00:00.000Z');
  const boundary = new Date(pruningAt.getTime() - 6 * 60 * 60 * 1000);
  utimesSync(active.path, boundary, boundary);
  utimesSync(reopened.path, boundary, boundary);
  const past = new Date(boundary.getTime() - 1);
  utimesSync(stale.path, past, past);

  const newest = open(root, {
    keep: 1,
    now: () => pruningAt,
  });
  assert.equal(existsSync(newest.path), true);
  assert.equal(existsSync(active.path), true);
  assert.equal(existsSync(reopened.path), true);
  assert.equal(existsSync(stale.path), false);
  assert.equal(existsSync(closed.path), false);
});

test('openRun with keep zero retains the run it just opened', (t) => {
  const root = fixture(t);
  const { path } = open(root, { keep: 0 });
  assert.equal(readRun(path).outcome, 'running');
});

test('appendEvent increments seq and owns the envelope fields', (t) => {
  const root = fixture(t);
  const { path, runId } = open(root);
  const failure = appendEvent(path, {
    kind: 'failure',
    project: 'storage',
    class: 'test',
    test: 'adds',
    excerpt: 'test adds ... FAILED',
    v: 99,
    run: 'wrong',
    seq: 99,
    at: 'wrong',
  }, { now });
  const verdict = appendEvent(path, {
    kind: 'gate-verdict',
    project: 'storage',
    outcome: 'failed',
    seconds: 2,
    failureClass: 'code',
  }, { now });

  assert.equal(failure.v, 1);
  assert.equal(failure.run, runId);
  assert.equal(failure.seq, 1);
  assert.equal(failure.at, AT);
  assert.equal(verdict.seq, 2);
  assert.deepEqual(readRun(path).events.map((event) => event.seq), [0, 1, 2]);
});

test('appendEvent counts existing physical lines, including malformed lines', (t) => {
  const root = fixture(t);
  const { path } = open(root);
  appendFileSync(path, '{broken JSON}\n\n', 'utf8');

  const event = appendEvent(path, { kind: 'detail', excerpt: 'detail' }, { now });
  assert.equal(event.seq, 3);
  assert.deepEqual(readRun(path).events.map((entry) => entry.seq), [0, 3]);
});

test('appendEvent fails open on IO and serialization errors', (t) => {
  const root = fixture(t);
  const missing = join(root, 'missing-directory', 'run.jsonl');
  assert.equal(appendEvent(missing, { kind: 'failure' }), null);

  const { path } = open(root);
  const circular = { kind: 'failure' };
  circular.self = circular;
  assert.equal(appendEvent(path, circular), null);
  assert.equal(readRun(path).events.length, 1);
  assert.equal(appendEvent(path, { kind: 'detail' }).seq, 1);
});

test('appendEvent closes off a torn last line so every event is its own line', (t) => {
  const root = fixture(t);
  const path = join(root, 'partial.jsonl');
  writeFileSync(path, '{broken');
  assert.equal(appendEvent(path, { kind: 'detail' }).seq, 1);
  assert.equal(appendEvent(path, { kind: 'detail' }).seq, 2);
  assert.equal(appendEvent(path, { kind: 'detail' }).seq, 3);
  const lines = readFileSync(path, 'utf8').split('\n');
  assert.equal(lines[0], '{broken');
  assert.deepEqual(lines.slice(1, 4).map(line => JSON.parse(line).seq), [1, 2, 3]);
});

test('appendEvent never rereads the whole file and appends once per event', (t) => {
  const root = fixture(t);
  const paths = [join(root, 'first.jsonl'), join(root, 'second.jsonl')];
  writeFileSync(paths[0], '{broken JSON}\n\n');
  writeFileSync(paths[1], '{"kind":"run-open"}\n');
  const originalAppend = fs.appendFileSync;
  const appends = [];
  const readMock = t.mock.method(fs, 'readFileSync', () => {
    throw new Error('appendEvent must not reread the run');
  });
  const appendMock = t.mock.method(fs, 'appendFileSync', (...args) => {
    appends.push(args);
    return originalAppend(...args);
  });
  syncBuiltinESMExports();
  try {
    for (let i = 0; i < 3; i += 1) {
      assert.equal(appendEvent(paths[0], { kind: 'detail' }).seq, i + 2);
      assert.equal(appendEvent(paths[1], { kind: 'detail' }).seq, i + 1);
    }
    assert.equal(appends.length, 6);
    assert.ok(appends.every(([, text]) => text.split('\n').length === 2));
  } finally {
    readMock.mock.restore();
    appendMock.mock.restore();
    syncBuiltinESMExports();
  }
});

test('appendEvent keeps seq unique when another process appends to the same run', (t) => {
  const root = fixture(t);
  const path = join(root, 'shared.jsonl');
  writeFileSync(path, '');
  assert.equal(appendEvent(path, { kind: 'gate-start' }).seq, 0);
  // A gate's tap is a separate process writing the same run between this process's appends.
  appendFileSync(path, `${JSON.stringify({ kind: 'failure', seq: 1 })}\n`);
  appendFileSync(path, `${JSON.stringify({ kind: 'failure', seq: 2 })}\n`);
  assert.equal(appendEvent(path, { kind: 'gate-verdict' }).seq, 3);
  assert.equal(appendEvent(path, { kind: 'run-close' }).seq, 4);
  const seqs = readFileSync(path, 'utf8').trim().split('\n').map(line => JSON.parse(line).seq);
  assert.deepEqual(seqs, [0, 1, 2, 3, 4]);
});

test('closeRun accepts gates-passed with a deferred count', (t) => {
  const root = fixture(t);
  const { path } = open(root);
  const closed = closeRun(path, 'gates-passed', { now, deferred: 2 });
  assert.equal(closed?.outcome, 'gates-passed');
  assert.equal(closed.deferred, 2);
  assert.equal(readRun(path).outcome, 'gates-passed');
});

test('closeRun is idempotent and preserves the first closing outcome', (t) => {
  const root = fixture(t);
  const { path } = open(root);
  appendEvent(path, { kind: 'failure', excerpt: 'failed' }, { now });
  const closed = closeRun(path, 'failed', { now });
  const before = readFileSync(path, 'utf8');

  assert.equal(closed.seq, 2);
  assert.equal(closed.kind, 'run-close');
  assert.equal(closed.outcome, 'failed');

  closeRun(path, 'passed', {
    now: () => {
      throw new Error('an idempotent close must not need another timestamp');
    },
  });

  assert.equal(readFileSync(path, 'utf8'), before);
  assert.equal(readRun(path).outcome, 'failed');
  assert.equal(
    readRun(path).events.filter((event) => event.kind === 'run-close').length,
    1,
  );
});

test('readRun distinguishes fresh, stale, and explicitly closed runs', (t) => {
  const root = fixture(t);
  const { path } = open(root);
  const current = new Date(AT);
  const fresh = new Date(current.getTime() - 1000);
  utimesSync(path, fresh, fresh);
  assert.equal(readRun(path, { now }).outcome, 'running');

  const stale = new Date(current.getTime() - 6 * 60 * 60 * 1000 - 1000);
  utimesSync(path, stale, stale);
  assert.equal(readRun(path, { now }).outcome, 'interrupted');

  closeRun(path, 'passed', { now });
  utimesSync(path, stale, stale);
  assert.equal(readRun(path, { now }).outcome, 'passed');
});

test('readRun accepts an injected stale threshold and skips malformed events', (t) => {
  const root = fixture(t);
  const { path } = open(root);
  appendFileSync(path, '{bad}\nnull\n[]\n{"not":"an event"}\n', 'utf8');
  const old = new Date(new Date(AT).getTime() - 2000);
  utimesSync(path, old, old);

  const state = readRun(path, { now, staleMs: 1000 });
  assert.equal(state.outcome, 'interrupted');
  assert.equal(state.events.length, 1);
  assert.equal(state.events[0].kind, 'run-open');
});

test('findRun resolves session, latest, id, and explicit paths', (t) => {
  const root = fixture(t);
  const first = open(root, { session: 'alice/device' });
  const second = open(root, {
    session: 'bob',
    now: () => new Date('2026-10-26T12:00:01.000Z'),
  });

  assert.equal(findRun({ root, session: 'alice/device' }), first.path);
  assert.equal(findRun({ root, latest: true }), second.path);
  assert.equal(findRun({ root, run: first.runId }), first.path);
  assert.equal(findRun({ root, run: basename(first.path) }), first.path);
  assert.equal(findRun({ root, run: first.path }), first.path);
  assert.equal(findRun({ root, run: relative(root, first.path) }), first.path);
  assert.equal(findRun({ root, session: 'missing' }), null);
  assert.equal(findRun({ root, run: 'missing' }), null);
});

test('findRun returns null for an empty store and a stale pointer', (t) => {
  const root = fixture(t);
  assert.equal(findRun({ root, latest: true }), null);

  const pointers = join(root, '.eprfs', 'status', 'gate-live', 'by-session');
  mkdirSync(pointers, { recursive: true });
  writeFileSync(join(pointers, 'gone'), join(root, 'gone.jsonl'), 'utf8');
  assert.equal(findRun({ root, session: 'gone' }), null);
});

test('follow prints existing and newly appended events once, in order', async (t) => {
  const root = fixture(t);
  const { path } = open(root);
  appendEvent(path, {
    kind: 'failure',
    project: 'storage',
    class: 'test',
    test: 'adds',
    excerpt: 'test adds ... FAILED',
  });
  const lines = [];
  let polls = 0;

  const outcome = await follow({
    root,
    run: path,
    write: (line) => lines.push(line),
    pollMs: 1,
    sleep: async () => {
      polls += 1;
      assert.ok(polls <= 3, 'follow should finish after the close');
      if (polls === 2) {
        appendEvent(path, {
          kind: 'gate-verdict',
          project: 'storage',
          outcome: 'failed',
          seconds: 2,
          failureClass: 'code',
        });
      }
      if (polls === 3) closeRun(path, 'failed');
    },
  });

  assert.equal(outcome, 'failed');
  assert.equal(polls, 3);
  assert.deepEqual(lines, [
    '[gate-live] RED storage test adds — test adds ... FAILED',
    '[gate-live] storage FAILED 2s failure_class=code',
    '[gate-live] run failed',
  ]);
});

for (const selector of [{ session: 'waiting-session' }, { latest: true }]) {
  test(`follow waits for a run selected by ${Object.keys(selector)[0]}`, async (t) => {
    const root = fixture(t);
    const lines = [];
    let polls = 0;

    const outcome = await follow({
      root,
      ...selector,
      pollMs: 1,
      write: (line) => lines.push(line),
      sleep: async () => {
        polls += 1;
        assert.equal(polls, 1);
        const { path } = open(root, { session: 'waiting-session' });
        appendEvent(path, {
          kind: 'gate-verdict',
          project: 'app',
          outcome: 'passed',
          seconds: 1,
          failureClass: 'none',
        });
        closeRun(path, 'passed');
      },
    });

    assert.equal(outcome, 'passed');
    assert.equal(polls, 1);
    assert.deepEqual(lines, [
      '[gate-live] app PASSED 1s failure_class=none',
      '[gate-live] run passed',
    ]);
  });
}

test('follow by session skips the previous push and waits for the next run', async (t) => {
  const root = fixture(t);
  const previous = open(root, { session: 'pusher' });
  appendEvent(previous.path, { kind: 'gate-verdict', project: 'old', verdict: { exit: 1, seconds: 9 } });
  closeRun(previous.path, 'failed');
  const past = new Date(Date.now() - 60_000);
  utimesSync(previous.path, past, past);
  const lines = [];
  let polls = 0;

  const outcome = await follow({
    root,
    session: 'pusher',
    pollMs: 1,
    write: (line) => lines.push(line),
    sleep: async () => {
      polls += 1;
      if (polls === 2) {
        const { path } = open(root, { session: 'pusher' });
        appendEvent(path, { kind: 'gate-verdict', project: 'new', verdict: { exit: 0, seconds: 3 } });
        closeRun(path, 'passed');
      }
    },
  });

  assert.equal(outcome, 'passed');
  assert.deepEqual(lines, ['[gate-live] new PASSED 3s failure_class=none', '[gate-live] run passed']);
});

test('follow timeout returns unknown while waiting for a run', async (t) => {
  const root = fixture(t);
  let clock = 0;
  t.mock.method(Date, 'now', () => clock);
  const sleeps = [];
  const lines = [];

  const outcome = await follow({
    root,
    latest: true,
    timeoutMs: 25,
    pollMs: 10,
    write: (line) => lines.push(line),
    sleep: async (milliseconds) => {
      sleeps.push(milliseconds);
      clock += milliseconds;
    },
  });

  assert.equal(outcome, 'unknown');
  assert.deepEqual(sleeps, [10, 10, 5]);
  assert.deepEqual(lines, []);
});

test('follow timeout also bounds an existing unclosed run', async (t) => {
  const root = fixture(t);
  const { path } = open(root);
  let clock = 0;
  t.mock.method(Date, 'now', () => clock);

  const outcome = await follow({
    root,
    run: path,
    timeoutMs: 20,
    pollMs: 10,
    write: () => assert.fail('run-open is not a formatted event'),
    sleep: async (milliseconds) => {
      clock += milliseconds;
    },
  });

  assert.equal(outcome, 'unknown');
  assert.equal(readRun(path).outcome, 'running');
});

test('follow returns interrupted for a stale unclosed run', async (t) => {
  const root = fixture(t);
  const { path } = open(root);
  utimesSync(path, new Date(0), new Date(0));

  assert.equal(await follow({
    root,
    run: path,
    write: () => assert.fail('run-open is not a formatted event'),
    sleep: async () => assert.fail('a stale run should not keep polling'),
  }), 'interrupted');
});

test('formatEvent renders compact failure, detail, step, verdict, and close lines', () => {
  const cases = [
    [{
      kind: 'failure',
      project: 'storage',
      class: 'compile',
      file: 'src/lib.rs',
      line: 3,
      test: 'adds',
      excerpt: 'error[E0308]: mismatched types\n  --> src/lib.rs:3:5',
    }, '[gate-live] RED storage compile src/lib.rs:3 adds — error[E0308]: mismatched types'],
    [{
      kind: 'failure',
      project: 'storage',
      class: 'test',
      locus: { file: 'src/lib.rs', line: 10, test: 'adds' },
      excerpt: 'test adds ... FAILED',
    }, '[gate-live] RED storage test src/lib.rs:10 adds — test adds ... FAILED'],
    [{
      kind: 'failure',
      project: 'app',
      class: 'test',
      test: 'adds',
      excerpt: 'x'.repeat(200) + '\nsecond line',
    }, `[gate-live] RED app test adds — ${'x'.repeat(160)}`],
    [{
      kind: 'detail',
      project: 'storage',
      test: 'adds',
      excerpt: "\n---- adds stdout ----\nthread 'adds' panicked at src/lib.rs:10:5:\nassertion failed",
    }, "[gate-live]   ↳ storage adds thread 'adds' panicked at src/lib.rs:10:5:"],
    [{
      kind: 'detail',
      project: 'app',
      test: 'adds',
      excerpt: '\n  AssertionError: expected 2 to equal 3\nother detail',
    }, '[gate-live]   ↳ app adds AssertionError: expected 2 to equal 3'],
    [{
      kind: 'failure',
      project: 'storage',
      failure: { class: 'test', fingerprint: 'abc', locus: { file: 'src/head.rs', line: 41, test: 'parses_head' }, nth: 1, excerpt: 'test parses_head ... FAILED' },
    }, '[gate-live] RED storage test src/head.rs:41 parses_head — test parses_head ... FAILED'],
    [{
      kind: 'failure',
      project: 'storage',
      failure: { class: 'test', locus: { file: 'src/head.rs', line: 41 }, excerpt: 'boom',
        attribution: { label: 'other', sessions: ['0123456789abcdef'] } },
    }, '[gate-live] RED storage test src/head.rs:41 [other: 01234567] — boom'],
    [{
      kind: 'failure',
      project: 'storage',
      failure: { class: 'test', locus: { file: 'src/head.rs', line: 41 }, excerpt: 'boom',
        attribution: { label: 'mine', sessions: ['me'] } },
    }, '[gate-live] RED storage test src/head.rs:41 [mine] — boom'],
    [{
      kind: 'gate-verdict',
      project: 'storage',
      verdict: { exit: 1, seconds: 738, failure_class: 'code', failures: 2 },
    }, '[gate-live] storage FAILED 738s failure_class=code'],
    [{
      kind: 'gate-verdict',
      project: 'eprfs',
      verdict: { exit: 0, seconds: 0, reused: true },
    }, '[gate-live] eprfs REUSED 0s failure_class=none'],
    [{
      kind: 'step-failed',
      project: 'storage',
      step: 'test',
      exit: 101,
    }, '[gate-live] step storage/test exit 101'],
    [{
      kind: 'gate-verdict',
      project: 'storage',
      outcome: 'failed',
      seconds: 12.5,
      failureClass: 'code',
    }, '[gate-live] storage FAILED 12.5s failure_class=code'],
    [{
      kind: 'gate-verdict',
      project: 'app',
      verdict: 'passed',
      seconds: 2,
      failure_class: 'none',
    }, '[gate-live] app PASSED 2s failure_class=none'],
    [{ kind: 'run-close', outcome: 'interrupted' }, '[gate-live] run interrupted'],
    [{ kind: 'run-open' }, null],
    [{ kind: 'unrecognized' }, null],
    [null, null],
  ];

  for (const [event, expected] of cases) {
    assert.equal(formatEvent(event), expected);
  }
});

test('formatEvent shows the deferred count only when positive', () => {
  assert.equal(formatEvent({ kind: 'run-close', outcome: 'gates-passed', deferred: 2 }),
    '[gate-live] run gates-passed (2 deferred)');
  assert.equal(formatEvent({ kind: 'run-close', outcome: 'gates-passed', deferred: 0 }),
    '[gate-live] run gates-passed');
});

test('CLI close accepts an explicit deferred count', (t) => {
  const root = fixture(t);
  const { path } = open(root);
  const result = cli(root, ['close', '--run', path, '--outcome', 'passed', '--deferred', '3']);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(readRun(path).events.at(-1).deferred, 3);
});

test('CLI close counts only deferred gate verdicts from the log', (t) => {
  const root = fixture(t);
  const { path } = open(root);
  for (const event of [
    { kind: 'gate-verdict', project: 'one', verdict: { deferred: true } },
    { kind: 'gate-verdict', project: 'two', verdict: { deferred: true } },
    { kind: 'gate-verdict', project: 'three', verdict: { deferred: false } },
    { kind: 'gate-verdict', project: 'four', verdict: { deferred: 'true' } },
    { kind: 'gate-verdict', project: 'five', verdict: { exit: 0 } },
    { kind: 'detail', verdict: { deferred: true } },
  ]) appendEvent(path, event);
  appendFileSync(path, '{damaged}\n');
  const result = cli(root, ['close', '--run', path, '--outcome', 'gates-passed', '--deferred-from-log']);
  assert.equal(result.status, 0, result.stderr);
  const closed = readRun(path).events.at(-1);
  assert.equal(closed.outcome, 'gates-passed');
  assert.equal(closed.deferred, 2);
});

test('CLI follow exits zero for gates-passed', (t) => {
  const root = fixture(t);
  const { path } = open(root);
  appendEvent(path, { kind: 'run-close', outcome: 'gates-passed', deferred: 2 });
  const result = cli(root, ['follow', '--run', path, '--timeout-ms', '100']);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout, '[gate-live] run gates-passed (2 deferred)\n');
});

test('CLI rejects unknown verbs, missing arguments, and invalid selectors with exit 2', (t) => {
  const root = fixture(t);
  const cases = [
    [],
    ['unknown'],
    ['open', '--sha', SHA],
    ['event', '--run', 'run.jsonl'],
    ['close', '--run', 'run.jsonl'],
    ['close', '--run', 'run.jsonl', '--outcome', 'unknown'],
    ['close', '--run', 'run.jsonl', '--outcome', 'passed', '--deferred', '-1'],
    ['close', '--run', 'run.jsonl', '--outcome', 'passed', '--deferred', '1.5'],
    ['close', '--run', 'run.jsonl', '--outcome', 'passed', '--deferred', '9007199254740992'],
    ['close', '--run', 'run.jsonl', '--outcome', 'passed', '--deferred', '1', '--deferred-from-log'],
    ['show'],
    ['show', '--latest', '--session', 'alice'],
    ['follow', '--run', 'run.jsonl', '--latest'],
    ['follow', '--timeout-ms', '-1'],
    ['event', '--run', 'run.jsonl', '--json', '{bad}'],
  ];

  for (const args of cases) {
    const result = cli(root, args);
    assert.equal(result.status, 2, JSON.stringify(args));
    assert.match(result.stderr, /Usage:/);
    assert.equal(result.stdout, '');
  }
});

test('CLI open, event, show, and close preserve excerpts and support seq selection', (t) => {
  const root = fixture(t);
  const opened = cli(root, ['open', '--sha', SHA, '--target', 'dev', '--session', 'cli']);
  assert.equal(opened.status, 0);
  assert.equal(opened.stderr, '');
  const path = opened.stdout.trim();
  assert.equal(isAbsolute(path), true);

  const excerpt = 'test adds ... FAILED\nassertion failed';
  const added = cli(root, [
    'event',
    '--run',
    path,
    '--json',
    JSON.stringify({ kind: 'failure', project: 'app', class: 'test', test: 'adds', excerpt }),
  ]);
  assert.equal(added.status, 0);
  assert.equal(added.stdout, '');
  assert.equal(added.stderr, '');

  const shown = cli(root, ['show', '--session', 'cli']);
  assert.equal(shown.status, 0);
  const events = shown.stdout.trim().split('\n').map((line) => JSON.parse(line));
  assert.equal(events.length, 2);
  assert.equal(events[1].excerpt, excerpt);

  const selected = cli(root, ['show', '--run', path, '--seq', '1']);
  assert.equal(selected.status, 0);
  assert.deepEqual(JSON.parse(selected.stdout), events[1]);

  const closed = cli(root, ['close', '--run', path, '--outcome', 'failed']);
  assert.equal(closed.status, 0);
  assert.equal(closed.stderr, '');
  assert.equal(readRun(path).outcome, 'failed');
});

test('CLI open, event, and close fail open with one stderr line on IO errors', (t) => {
  const root = fixture(t);
  writeFileSync(join(root, '.eprfs'), 'blocks the log directory', 'utf8');
  const missing = join(root, 'missing', 'run.jsonl');
  const cases = [
    ['open', '--sha', SHA, '--target', 'dev', '--session', 'cli'],
    ['event', '--run', missing, '--json', '{"kind":"failure"}'],
    ['close', '--run', missing, '--outcome', 'failed'],
  ];

  for (const args of cases) {
    const result = cli(root, args);
    assert.equal(result.status, 0);
    assert.equal(result.stdout, '');
    assert.match(result.stderr, /^\[gate-live\] [^\r\n]+\n$/);
  }
});

test('CLI follow maps outcomes to exit codes', (t) => {
  const root = fixture(t);
  for (const [outcome, status] of [
    ['passed', 0],
    ['failed', 1],
    ['interrupted', 3],
  ]) {
    const { path } = open(root);
    closeRun(path, outcome);
    const result = cli(root, ['follow', '--run', path, '--timeout-ms', '1000']);
    assert.equal(result.status, status);
    assert.equal(result.stdout, `[gate-live] run ${outcome}\n`);
    assert.equal(result.stderr, '');
  }

  const unknown = cli(root, ['follow', '--session', 'missing', '--timeout-ms', '0']);
  assert.equal(unknown.status, 3);
  assert.equal(unknown.stdout, '');
  assert.equal(unknown.stderr, '');
});
