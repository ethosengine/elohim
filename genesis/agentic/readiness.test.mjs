import { test } from 'node:test';
import assert from 'node:assert/strict';
import { checkGitClean, checkMeasureRuns, checkPaletteGaps } from './readiness.mjs';

test('checkGitClean passes on clean tree', async () => {
  // Uses `git status --porcelain`; in a clean worktree produces empty output.
  // We run against an isolated fixture dir to avoid false positives.
  const { execSync } = await import('node:child_process');
  const { mkdtempSync } = await import('node:fs');
  const { tmpdir } = await import('node:os');
  const { join } = await import('node:path');
  const dir = mkdtempSync(join(tmpdir(), 'git-clean-test-'));
  execSync('git init -q', { cwd: dir });
  execSync('git commit --allow-empty -m init -q', {
    cwd: dir,
    env: { ...process.env, GIT_AUTHOR_NAME: 't', GIT_AUTHOR_EMAIL: 't@t', GIT_COMMITTER_NAME: 't', GIT_COMMITTER_EMAIL: 't@t' },
  });
  const result = await checkGitClean({ cwd: dir });
  assert.equal(result.ok, true, result.reason);
});

test('checkGitClean fails with untracked files', async () => {
  const { execSync } = await import('node:child_process');
  const { mkdtempSync, writeFileSync } = await import('node:fs');
  const { tmpdir } = await import('node:os');
  const { join } = await import('node:path');
  const dir = mkdtempSync(join(tmpdir(), 'git-dirty-test-'));
  execSync('git init -q', { cwd: dir });
  execSync('git commit --allow-empty -m init -q', {
    cwd: dir,
    env: { ...process.env, GIT_AUTHOR_NAME: 't', GIT_AUTHOR_EMAIL: 't@t', GIT_COMMITTER_NAME: 't', GIT_COMMITTER_EMAIL: 't@t' },
  });
  writeFileSync(join(dir, 'dirty.txt'), 'x');
  const result = await checkGitClean({ cwd: dir });
  assert.equal(result.ok, false);
  assert.match(result.reason, /untracked|uncommitted/i);
});

test('checkMeasureRuns succeeds on numeric output', async () => {
  const result = await checkMeasureRuns({ cmd: 'echo 0.42' });
  assert.equal(result.ok, true);
  assert.equal(result.baseline, 0.42);
});

test('checkMeasureRuns fails on non-numeric output', async () => {
  const result = await checkMeasureRuns({ cmd: 'echo not-a-number' });
  assert.equal(result.ok, false);
  assert.match(result.reason, /numeric|parse/i);
});

test('checkMeasureRuns fails on nonzero exit', async () => {
  const result = await checkMeasureRuns({ cmd: 'false' });
  assert.equal(result.ok, false);
});

test('checkPaletteGaps reports missing commands', () => {
  const palette = ['Bash(git status)', 'Bash(pnpm run test)'];
  const planned = ['git status', 'pnpm run test', 'kubectl get pods'];
  const result = checkPaletteGaps({ palette, planned });
  assert.equal(result.ok, false);
  assert.deepEqual(result.missing, ['kubectl get pods']);
});

test('checkPaletteGaps passes when all covered', () => {
  const palette = ['Bash(git status)', 'Bash(pnpm run test)'];
  const planned = ['git status', 'pnpm run test'];
  const result = checkPaletteGaps({ palette, planned });
  assert.equal(result.ok, true);
  assert.deepEqual(result.missing, []);
});
