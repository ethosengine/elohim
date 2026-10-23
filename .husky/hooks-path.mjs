#!/usr/bin/env node
// Keep core.hooksPath absolute, run by `prepare` after husky (operator ruling 2026-10-23).
//
// husky writes the RELATIVE `.husky/_`. Git resolves a relative hooksPath against the working tree
// that runs the hook, and a linked worktree (/projects/elohim/.worktrees/<name>) has no `.husky/_`
// of its own, so a push from a worktree ran no pre-push hook at all. This points core.hooksPath at
// `<the git common dir's parent>/.husky/_` — the main checkout's — from wherever `pnpm install`
// ran. When that shim dir is missing (an install that ran only in a worktree), it is installed
// from this checkout's freshly generated one, so the path never names an empty directory.
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
import { cpSync, existsSync } from 'node:fs';
import { basename, dirname, join, resolve } from 'node:path';

function git(args, cwd) {
  return execFileSync('git', args, { cwd, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] })
    .trim();
}

function main(cwd) {
  if (process.env.HUSKY === '0') return 'HUSKY=0 — hooksPath left as it is';
  let common;
  let top;
  try {
    common = resolve(cwd, git(['rev-parse', '--git-common-dir'], cwd));
    top = git(['rev-parse', '--show-toplevel'], cwd);
  } catch {
    return 'not a git checkout — nothing to do';
  }
  if (basename(common) !== '.git') return `${common} is not a .git common dir — hooksPath left as it is`;
  const target = join(dirname(common), '.husky', '_');
  const local = join(top, '.husky', '_');
  if (!existsSync(target) && existsSync(local) && local !== target) {
    cpSync(local, target, { recursive: true });
  }
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

console.log(`[hooks-path] ${main(process.cwd())}`);
