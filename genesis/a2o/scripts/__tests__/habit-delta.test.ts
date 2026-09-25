/* eslint-disable @typescript-eslint/no-floating-promises -- node:test owns returned promises. */
/* eslint-disable sonarjs/no-duplicate-string -- fixture paths and concern tags are intentionally repeated. */
import assert from 'node:assert/strict';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';

import {
  appendDelta,
  checksTextOf,
  DELTA_DATE_RE,
  findHabitByConcern,
  frontmatterOf,
  walkHabitFiles,
} from '../lib/habit-delta.js';

const HABIT_BODY = (concern: string) => `---
epr-habit-version: 1
id: ${concern}-habit
invariant: >
  A fixture invariant for ${concern}.
status: red
active: false
checks:
  - "a2o @concern:${concern} (some/feature.feature)"
  - "python3 -m unittest some_test"
refs:
  - "fixture"
retire-when: >
  never, this is a test fixture.
---
DELTA 2026-01-01: born RED.
`;

async function makeRepo(t: { after: (fn: () => unknown) => void }) {
  const root = await mkdtemp(join(tmpdir(), 'habit-delta-repo-'));
  t.after(async () => rm(root, { recursive: true, force: true }));
  return root;
}

async function writeHabit(root: string, dir: string, id: string, concern: string) {
  const target = join(root, dir, '.epr-meta');
  await mkdir(target, { recursive: true });
  const path = join(target, `${id}.habit.md`);
  await writeFile(path, HABIT_BODY(concern));
  return path;
}

test('findHabitByConcern resolves the one habit atom naming the tag', async t => {
  const root = await makeRepo(t);
  const path = await writeHabit(
    root,
    'elohim/elohim-storage',
    'operator-runtime-surface',
    'operator-runtime-surface'
  );
  // A sibling habit with a DIFFERENT concern must not be picked up.
  await writeHabit(root, 'doorway/doorway-service', 'doorway-failover', 'doorway-failover');
  await mkdir(join(root, 'node_modules', '.epr-meta'), { recursive: true });
  await writeFile(
    join(root, 'node_modules', '.epr-meta', 'decoy.habit.md'),
    HABIT_BODY('operator-runtime-surface')
  );

  const found = findHabitByConcern(root, 'operator-runtime-surface');
  assert.equal(found.ok, true);
  assert.equal(found.ok && found.path, path);
});

test('findHabitByConcern skips a nested git worktree checkout (dir with a .git FILE)', async t => {
  const root = await makeRepo(t);
  const real = await writeHabit(
    root,
    'elohim/elohim-storage',
    'operator-runtime-surface',
    'operator-runtime-surface'
  );

  // A nested worktree checkout under an unlisted parent name (.claude/worktrees/<slice>, not
  // the bare .worktrees SKIP_DIRS already knows) — the shape `git worktree add` produces: a
  // `.git` REGULAR FILE (a gitdir pointer), never a directory. It carries a duplicate atom for
  // the SAME concern; before the fix this made findHabitByConcern report "ambiguous".
  const worktreeDir = join(root, '.claude', 'worktrees', 'fake-slice');
  await mkdir(worktreeDir, { recursive: true });
  await writeFile(
    join(worktreeDir, '.git'),
    'gitdir: /projects/elohim/.git/worktrees/fake-slice\n'
  );
  await writeHabit(
    worktreeDir,
    'elohim/elohim-storage',
    'operator-runtime-surface',
    'operator-runtime-surface'
  );

  const found = findHabitByConcern(root, 'operator-runtime-surface');
  assert.equal(found.ok, true);
  assert.equal(found.ok && found.path, real);
});

test('findHabitByConcern refuses when zero atoms name the tag', async t => {
  const root = await makeRepo(t);
  await writeHabit(root, 'elohim/elohim-storage', 'other-habit', 'some-other-concern');

  const found = findHabitByConcern(root, 'missing-concern');
  assert.equal(found.ok, false);
  assert.ok(!found.ok && found.reason.includes('no habit atom'));
  assert.ok(!found.ok && found.matches.length === 0);
});

test('findHabitByConcern refuses when more than one atom names the tag', async t => {
  const root = await makeRepo(t);
  await writeHabit(root, 'a', 'atom-a', 'shared-concern');
  await writeHabit(root, 'b', 'atom-b', 'shared-concern');

  const found = findHabitByConcern(root, 'shared-concern');
  assert.equal(found.ok, false);
  assert.ok(!found.ok && found.reason.includes('ambiguous'));
  assert.ok(!found.ok && found.matches.length === 2);
});

test('walkHabitFiles skips node_modules, target, .worktrees and .git', async t => {
  const root = await makeRepo(t);
  const real = await writeHabit(root, 'keep', 'keep-habit', 'keep-concern');
  for (const skipped of ['node_modules', 'target', '.worktrees', '.git']) {
    await mkdir(join(root, skipped, '.epr-meta'), { recursive: true });
    await writeFile(join(root, skipped, '.epr-meta', 'decoy.habit.md'), HABIT_BODY('keep-concern'));
  }
  const files = walkHabitFiles(root);
  assert.deepEqual(files, [real]);
});

test('checksTextOf isolates the checks: list from refs: and other frontmatter keys', () => {
  const front = frontmatterOf(HABIT_BODY('x'))!;
  const checks = checksTextOf(front);
  assert.match(checks, /@concern:x/);
  assert.doesNotMatch(checks, /fixture/); // that string lives under refs:, not checks:
});

test('appendDelta inserts newest-first directly after the closing frontmatter fence', async t => {
  const root = await makeRepo(t);
  const path = await writeHabit(root, 'x', 'x-habit', 'x-concern');

  const result = appendDelta(path, {
    date: '2026-09-25',
    label: 'peer-stage rung H, provider ab12cd34',
    text: 'x-concern passed=1 failed=0 — report reports/peer-stage/2026-09-25/abc/stage.json',
    onceKey: 'completion-hash-1',
  });
  assert.deepEqual(result, { ok: true, appended: true });

  const content = await readFile(path, 'utf8');
  const lines = content.split('\n');
  const fenceEndIndex = lines.findIndex((l, i) => i > 0 && l === '---');
  assert.equal(
    lines[fenceEndIndex + 1],
    'DELTA 2026-09-25 (peer-stage rung H, provider ab12cd34): x-concern passed=1 failed=0 — report reports/peer-stage/2026-09-25/abc/stage.json'
  );
  // The old newest DELTA (2026-01-01) is still present, now second.
  assert.ok(content.includes('DELTA 2026-01-01: born RED.'));
  assert.ok(content.indexOf('DELTA 2026-09-25') < content.indexOf('DELTA 2026-01-01'));
});

test('appendDelta is idempotent: a repeated onceKey writes nothing a second time', async t => {
  const root = await makeRepo(t);
  const path = await writeHabit(root, 'x', 'x-habit', 'x-concern');
  const input = {
    date: '2026-09-25',
    text: 'x-concern passed=1 failed=0 (completion-hash-2)',
    onceKey: 'completion-hash-2',
  };

  const first = appendDelta(path, input);
  assert.deepEqual(first, { ok: true, appended: true });
  const afterFirst = await readFile(path, 'utf8');

  const second = appendDelta(path, input);
  assert.equal(second.ok, true);
  assert.equal(second.appended, false);
  const afterSecond = await readFile(path, 'utf8');
  assert.equal(afterFirst, afterSecond);
});

test('appendDelta writes a line the habits-status.py _DELTA_DATE grammar can parse', async t => {
  const root = await makeRepo(t);
  const path = await writeHabit(root, 'x', 'x-habit', 'x-concern');
  appendDelta(path, { date: '2026-09-25', text: 'hello', onceKey: 'k3' });
  const content = await readFile(path, 'utf8');
  const match = DELTA_DATE_RE.exec(content);
  assert.ok(match);
  assert.equal(match?.[1], '2026-09-25');
});

test('appendDelta refuses a target with no frontmatter to anchor after', async t => {
  const root = await makeRepo(t);
  const path = join(root, 'no-frontmatter.habit.md');
  await writeFile(path, 'not a habit atom at all');
  const result = appendDelta(path, { date: '2026-09-25', text: 'x', onceKey: 'k4' });
  assert.equal(result.ok, false);
  assert.match(result.reason ?? '', /frontmatter/);
});
