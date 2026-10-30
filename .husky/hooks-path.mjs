#!/usr/bin/env node
// Keep core.hooksPath absolute, run by `prepare` after husky (operator ruling 2026-10-23).
//
// husky writes the RELATIVE `.husky/_`. Git resolves a relative hooksPath against the working tree
// that runs the hook, and a linked worktree (/projects/elohim/.worktrees/<name>) has no `.husky/_`
// of its own, so a push from a worktree ran no pre-push hook at all. This points core.hooksPath at
// `<the git common dir's parent>/.husky/_` — the main checkout's — from wherever `pnpm install`
// ran. Missing/incomplete shims can be copied from this checkout after install. Before install
// (including devfile postStart), use the main checkout's tracked .husky entrypoints directly.
// Validate every governed hook chain before declaring success: a directory alone proves nothing.
// The governed entrypoints are pre-push (the push gate), pre-commit (the .epr-meta compose gate and
// the commit-time formatting ratchet) and pre-merge-commit (a clean automatic merge, delegated to
// pre-commit). A shim directory missing any of them would silently skip that hook.
//
// So a push from a worktree runs the MAIN checkout's hook script (`.husky/_/h` runs
// `<main>/.husky/pre-push`), with the worktree as its work tree: the hook's logic is whatever the
// main checkout has checked out, and the gate inputs are the worktree's. That version skew is
// expected — the hook on dev governs pushes to dev.
//
// Idempotent. A no-op under HUSKY=0 (the same switch husky honours), outside a git checkout, and
// when the common dir is not a `.git` directory (a separate git dir or a submodule), whose parent
// is not the checkout.
import { execFileSync } from 'node:child_process';
import { accessSync, constants, cpSync, statSync } from 'node:fs';
import { basename, dirname, join, resolve } from 'node:path';

function git(args, cwd) {
  return execFileSync('git', args, { cwd, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] })
    .trim();
}

function usable(path, mode = constants.R_OK) {
  try {
    accessSync(path, mode);
    return statSync(path).isFile();
  } catch {
    return false;
  }
}

// Hooks whose tracked entrypoint in .husky/ must exist and be executable, and whose husky shim
// must exist in .husky/_ before that directory may serve as core.hooksPath.
const GOVERNED_HOOKS = ['pre-push', 'pre-commit', 'pre-merge-commit'];

function shimsReady(dir) {
  return GOVERNED_HOOKS.every((hook) => usable(join(dir, hook), constants.R_OK | constants.X_OK)) &&
    usable(join(dir, 'h'));
}

function main(cwd) {
  if (process.env.HUSKY === '0') return 'HUSKY=0 — hooksPath left as it is';
  let common;
  let top;
  try {
    common = resolve(cwd, git(['rev-parse', '--git-common-dir'], cwd));
    top = git(['rev-parse', '--show-toplevel'], cwd);
  } catch (error) {
    if (String(error.stderr).includes('not a git repository')) return 'not a git checkout — nothing to do';
    throw error;
  }
  if (basename(common) !== '.git') return `${common} is not a .git common dir — hooksPath left as it is`;
  const hooks = join(dirname(common), '.husky');
  let target = join(hooks, '_');
  const local = join(top, '.husky', '_');
  const broken = GOVERNED_HOOKS.filter((hook) => !usable(join(hooks, hook), constants.R_OK | constants.X_OK));
  if (!usable(join(hooks, 'pre-push.bash'))) broken.push('pre-push.bash');
  if (broken.length) {
    throw new Error(`missing or non-executable ${broken.join(', ')} entrypoint in ${hooks}; restore the tracked .husky/${broken.length > 1 ? '{' + broken.join(',') + '}' : broken[0]}, then run pnpm run prepare`);
  }
  if (!shimsReady(target) && shimsReady(local) && local !== target) {
    cpSync(local, target, { recursive: true });
  }
  if (!shimsReady(target)) target = hooks;
  let current = '';
  try {
    current = git(['config', '--get', 'core.hooksPath'], cwd);
  } catch {
    current = '';
  }
  if (current === target) return `core.hooksPath already ${target}`;
  execFileSync('git', ['config', 'core.hooksPath', target], { cwd, stdio: 'ignore' });
  return `core.hooksPath ${current || '(unset)'} -> ${target}`;
}

try {
  console.log(`[hooks-path] ${main(process.cwd())}`);
} catch (error) {
  console.error(`[hooks-path] REFUSED: ${error.stderr?.toString().trim() || error.message}`);
  process.exitCode = 1;
}
