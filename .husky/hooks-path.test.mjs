// `.husky/hooks-path.mjs` keeps core.hooksPath absolute (operator ruling 2026-10-23): husky writes
// the relative `.husky/_`, which a linked worktree resolves against its own root, where no `_` is
// installed, so a push from a worktree ran no hook at all. Every case runs in a scratch repository
// with a linked worktree; this repository's own config is never read or written.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT = fileURLToPath(new URL('./hooks-path.mjs', import.meta.url));
const env = {
  PATH: process.env.PATH,
  HOME: tmpdir(),
  GIT_CONFIG_NOSYSTEM: '1',
  GIT_CONFIG_GLOBAL: '/dev/null',
  GIT_CONFIG_COUNT: '1',
  GIT_CONFIG_KEY_0: 'safe.directory',
  GIT_CONFIG_VALUE_0: '*',
};

function git(cwd, ...args) {
  const r = spawnSync('git', args, { cwd, env, encoding: 'utf8' });
  assert.equal(r.status, 0, `git ${args.join(' ')}: ${r.stderr}`);
  return r.stdout.trim();
}

function scratch(t) {
  const dir = realpathSync(mkdtempSync(join(tmpdir(), 'hooks-path-')));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const main = join(dir, 'main');
  mkdirSync(main);
  git(main, 'init', '-q');
  writeFileSync(join(main, 'README.md'), 'scratch\n');
  git(main, 'add', '-A');
  git(main, '-c', 'user.name=F', '-c', 'user.email=f@example.test', 'commit', '-q', '-m', 'init');
  // What husky leaves behind: its shim dir, and the relative path.
  mkdirSync(join(main, '.husky/_'), { recursive: true });
  writeFileSync(join(main, '.husky/_/h'), '#!/usr/bin/env sh\n');
  git(main, 'config', 'core.hooksPath', '.husky/_');
  const worktree = join(dir, 'wt');
  git(main, 'worktree', 'add', '-q', worktree);
  return { dir, main, worktree };
}

function runScript(cwd, extra = {}) {
  return spawnSync(process.execPath, [SCRIPT], { cwd, env: { ...env, ...extra }, encoding: 'utf8' });
}

test('from a linked worktree, core.hooksPath becomes the main checkout absolute .husky/_', (t) => {
  const { main, worktree } = scratch(t);
  const r = runScript(worktree);
  assert.equal(r.status, 0, r.stderr);
  const want = join(main, '.husky/_');
  assert.equal(git(main, 'config', '--get', 'core.hooksPath'), want);
  // The worktree reads the same shared config.
  assert.equal(git(worktree, 'config', '--get', 'core.hooksPath'), want);
  // Idempotent: a second run changes nothing and says so.
  const again = runScript(worktree);
  assert.equal(again.status, 0, again.stderr);
  assert.match(again.stdout, /already/);
  assert.equal(git(main, 'config', '--get', 'core.hooksPath'), want);
});

test('from the main checkout too, and a missing shim dir is installed from this checkout', (t) => {
  const { main, worktree } = scratch(t);
  rmSync(join(main, '.husky/_'), { recursive: true, force: true });
  mkdirSync(join(worktree, '.husky/_'), { recursive: true });
  writeFileSync(join(worktree, '.husky/_/h'), '#!/usr/bin/env sh\n');
  const r = runScript(worktree);
  assert.equal(r.status, 0, r.stderr);
  assert.ok(existsSync(join(main, '.husky/_/h')), 'the shims exist where the path points');
  const fromMain = runScript(main);
  assert.equal(fromMain.status, 0, fromMain.stderr);
  assert.equal(git(main, 'config', '--get', 'core.hooksPath'), join(main, '.husky/_'));
});

test('HUSKY=0 and a directory outside any git checkout are no-ops', (t) => {
  const { main, worktree } = scratch(t);
  const skipped = runScript(worktree, { HUSKY: '0' });
  assert.equal(skipped.status, 0, skipped.stderr);
  assert.equal(git(main, 'config', '--get', 'core.hooksPath'), '.husky/_');
  const outside = realpathSync(mkdtempSync(join(tmpdir(), 'hooks-path-none-')));
  t.after(() => rmSync(outside, { recursive: true, force: true }));
  const none = runScript(outside, { GIT_CEILING_DIRECTORIES: tmpdir() });
  assert.equal(none.status, 0, none.stderr);
});

test('a common dir not named .git (separate-git-dir, submodule) is left alone', (t) => {
  const dir = realpathSync(mkdtempSync(join(tmpdir(), 'hooks-path-sep-')));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const work = join(dir, 'work');
  mkdirSync(work);
  git(dir, 'init', '-q', '--separate-git-dir', join(dir, 'store.git'), work);
  git(work, 'config', 'core.hooksPath', '.husky/_');
  const r = runScript(work);
  assert.equal(r.status, 0, r.stderr);
  assert.match(r.stdout, /not a \.git common dir/);
  assert.equal(git(work, 'config', '--get', 'core.hooksPath'), '.husky/_');
});
