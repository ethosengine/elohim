import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

import { attribute, GATE_ATTRIBUTION_WINDOW_MS, readEdits } from './gate-attribution.mjs';

const root = '/checkout';
const openedAtMs = Date.parse('2026-10-26T12:00:00.000Z');
const atMs = Date.parse('2026-10-26T13:00:00.000Z');
const windowStartMs = openedAtMs - 6 * 60 * 60 * 1000;
const options = { root, session: 'mine', windowStartMs, atMs };
const failure = { locus: { file: 'src/answer.rs', line: 14 } };
const edit = (session = 'mine', overrides = {}) => ({
  v: 1,
  at: '2026-10-26T08:00:00.000Z',
  session,
  ownerPid: 123,
  ownerStartTime: '987654',
  path: '/checkout/src/answer.rs',
  tool: 'Edit',
  ...overrides,
});

function journal(t, contents) {
  const directory = mkdtempSync(join(tmpdir(), 'gate-attribution-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const path = join(directory, 'edits.jsonl');
  writeFileSync(path, contents);
  return path;
}

test('reads C2 tools, ignores torn/unknown rows, and preserves anonymous sessions', t => {
  const valid = ['Edit', 'Write', 'MultiEdit', 'NotebookEdit'].map((tool, index) =>
    edit(index === 3 ? 'anon-123' : `session-${index}`, { tool }));
  const invalid = [
    null, [], { v: 2 }, edit('x', { v: 2 }), edit('x', { tool: 'Bash' }),
    edit('x', { at: 'not-a-time' }), edit('x', { at: '2026-02-30T12:00:00.000Z' }),
    edit('x', { at: '2026-10-26T08:00:00+00:00' }), edit(''),
    edit('x', { ownerPid: 1.5 }), edit('x', { ownerStartTime: null }),
    edit('x', { path: 'src/answer.rs' }),
  ];
  const contents = [valid[0], ...invalid, ...valid.slice(1)].map(JSON.stringify).join('\r\n');
  const path = journal(t, `not json\n${contents}\n{"v":1,"at":`);
  assert.deepEqual(readEdits(path), valid);
});

test('maxLines selects the physical tail, not the first matches', t => {
  const rows = [edit('old'), edit('recent'), edit('newest')];
  const path = journal(t, `${rows.map(JSON.stringify).join('\n')}\n`);
  assert.deepEqual(readEdits(path, { maxLines: 2 }), rows.slice(1));
  assert.deepEqual(readEdits(path, { maxLines: 0 }), []);
  writeFileSync(path, `${rows.map(JSON.stringify).join('\n')}\n{"torn":`);
  assert.deepEqual(readEdits(path, { maxLines: 2 }), [rows[2]]);
  assert.deepEqual(readEdits(path, { maxLines: 1 }), []);
  for (const maxLines of [-1, 1.5, Infinity, '2']) {
    assert.throws(() => readEdits(path, { maxLines }), RangeError);
  }
});

test('missing, empty, and unreadable journals supply no evidence', t => {
  const path = journal(t, '');
  assert.deepEqual(readEdits(path), []);
  assert.deepEqual(readEdits(`${path}.missing`), []);
  assert.deepEqual(readEdits(join(path, '..')), []);
});

test('attributes one distinct session as mine or other without counting duplicate edits', () => {
  assert.deepEqual(attribute(failure, [edit(), edit()], options), {
    label: 'mine', sessions: ['mine'],
  });
  assert.deepEqual(attribute(failure, [edit('colleague')], options), {
    label: 'other', sessions: ['colleague'],
  });
});

test('multiple sessions are ambiguous and sorted, even when one is mine', () => {
  assert.deepEqual(attribute(failure, [edit('z'), edit(), edit('a'), edit('z')], options), {
    label: 'ambiguous', sessions: ['a', 'mine', 'z'],
  });
  assert.deepEqual(attribute(failure, [edit('z'), edit('a')], options), {
    label: 'ambiguous', sessions: ['a', 'z'],
  });
});

test('anon-PID is an independent session, even when another row has the same owner PID', () => {
  assert.deepEqual(attribute(failure, [edit('anon-123')], options), {
    label: 'other', sessions: ['anon-123'],
  });
  assert.deepEqual(attribute(failure, [edit('anon-123')], { ...options, session: 'anon-123' }), {
    label: 'mine', sessions: ['anon-123'],
  });
  assert.deepEqual(attribute(failure, [edit('anon-123'), edit()], options), {
    label: 'ambiguous', sessions: ['anon-123', 'mine'],
  });
});

test('window includes both endpoints and excludes old/future edits', () => {
  const at = ms => new Date(ms).toISOString();
  const rows = [
    edit('old', { at: at(windowStartMs - 1) }),
    edit('start', { at: at(windowStartMs) }),
    edit('end', { at: at(atMs) }),
    edit('future', { at: at(atMs + 1) }),
  ];
  assert.deepEqual(attribute(failure, rows, options), {
    label: 'ambiguous', sessions: ['end', 'start'],
  });
  assert.deepEqual(attribute(failure, [rows[0], rows[3]], options), {
    label: 'unattributed', sessions: [],
  });
  // A long-running gate keeps the lower bound at run-open minus the window.
  assert.deepEqual(attribute(failure, [rows[1]], { ...options, atMs: atMs + 12 * 60 * 60 * 1000 }), {
    label: 'other', sessions: ['start'],
  });
});

test('resolves relative loci against root and accepts absolute loci or enclosing events', () => {
  for (const locus of ['src/answer.rs', './src/../src/answer.rs', '/checkout/src/answer.rs']) {
    assert.deepEqual(attribute({ locus: { file: locus } }, [edit()], options), {
      label: 'mine', sessions: ['mine'],
    });
  }
  assert.deepEqual(attribute({ kind: 'failure', failure }, [edit()], options), {
    label: 'mine', sessions: ['mine'],
  });
  assert.deepEqual(attribute({ locus: { file: edit().path } }, [edit()], { ...options, root: undefined }), {
    label: 'mine', sessions: ['mine'],
  });
});

test('never guesses from test names, basenames, another worktree, or invalid bounds', () => {
  const none = { label: 'unattributed', sessions: [] };
  for (const value of [undefined, {}, { locus: { test: 'src/answer.rs' } }, { locus: { file: '' } }]) {
    assert.deepEqual(attribute(value, [edit()], options), none);
  }
  assert.deepEqual(attribute(failure, [edit('x', { path: '/other/src/answer.rs' })], options), none);
  assert.deepEqual(attribute(failure, [edit('x', { v: 2 })], options), none);
  assert.deepEqual(attribute(failure, [], options), none);
  for (const changed of [
    { root: undefined }, { root: '.' }, { windowStartMs: NaN }, { atMs: undefined },
    { windowStartMs: atMs + 1 },
  ]) assert.deepEqual(attribute(failure, [edit()], { ...options, ...changed }), none);
  assert.deepEqual(attribute(failure, [edit()]), none);
});

test('window defaults to six hours and accepts a non-negative millisecond environment override', async () => {
  assert.ok(Number.isSafeInteger(GATE_ATTRIBUTION_WINDOW_MS));
  const original = process.env.GATE_ATTRIBUTION_WINDOW_MS;
  const settings = [
    [undefined, 21_600_000], ['120000', 120_000], ['0', 0],
    ['', 21_600_000], ['-1', 21_600_000], ['NaN', 21_600_000], ['1.5', 21_600_000],
    ['9007199254740992', 21_600_000],
  ];
  try {
    for (const [index, [setting, expected]] of settings.entries()) {
      if (setting === undefined) delete process.env.GATE_ATTRIBUTION_WINDOW_MS;
      else process.env.GATE_ATTRIBUTION_WINDOW_MS = setting;
      const moduleUrl = new URL(`./gate-attribution.mjs?window-test=${index}`, import.meta.url);
      const loaded = await import(moduleUrl.href);
      assert.equal(loaded.GATE_ATTRIBUTION_WINDOW_MS, expected);
    }
  } finally {
    if (original === undefined) delete process.env.GATE_ATTRIBUTION_WINDOW_MS;
    else process.env.GATE_ATTRIBUTION_WINDOW_MS = original;
  }
});
