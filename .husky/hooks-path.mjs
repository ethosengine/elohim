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
// the commit-time formatting ratchet), pre-merge-commit (a clean automatic merge, delegated to
// pre-commit), prepare-commit-msg (passive native session attribution) and commit-msg (the
// Change-Id shim). A shim directory missing any of them would silently skip that hook.
//
// So a push from a worktree runs the MAIN checkout's hook script (`.husky/_/h` runs
// `<main>/.husky/pre-push`), with the worktree as its work tree: the hook's logic is whatever the
// main checkout has checked out, and the gate inputs are the worktree's. That version skew is
// expected — the hook on dev governs pushes to dev.
//
// When the main checkout's pre-push.bash differs from this worktree's (the main checkout sits on an
// older branch: the laptop, 2026-11-25, backlog row 106), a second line says so and prints the push
// form that runs this worktree's own hook for that one push. The setting itself is not moved:
// core.hooksPath lives in the config every checkout shares, so pointing it at one worktree would
// change which hook every other worktree and the main checkout run, and removing that worktree
// would leave a hooks directory that does not exist, which Git treats as no hook at all.
//
// Idempotent. A no-op under HUSKY=0 (the same switch husky honours), outside a git checkout, and
// when the common dir is not a `.git` directory (a separate git dir or a submodule), whose parent
// is not the checkout.
import { execFileSync } from 'node:child_process';
import { accessSync, constants, cpSync, readFileSync, statSync } from 'node:fs';
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
const GOVERNED_HOOKS = ['pre-push', 'pre-commit', 'pre-merge-commit', 'prepare-commit-msg', 'commit-msg'];

function shimsReady(dir) {
  return GOVERNED_HOOKS.every((hook) => usable(join(dir, hook), constants.R_OK | constants.X_OK)) &&
    usable(join(dir, 'h'));
}

/**
 * One line when a plain push from the linked worktree `top` would run a pre-push.bash that differs
 * from the worktree's own; '' when they agree, when `top` is the main checkout, or when either
 * file cannot be read (the entrypoint check has already refused a missing main one).
 */
function skewNote(top, hooks) {
  const own = join(top, '.husky');
  if (resolve(own) === resolve(hooks)) return '';
  let differs;
  try { differs = !readFileSync(join(hooks, 'pre-push.bash')).equals(readFileSync(join(own, 'pre-push.bash'))); }
  catch { return ''; }
  if (!differs) return '';
  const said = `NOTE: ${join(hooks, 'pre-push.bash')} differs from this worktree's, and a plain push from here runs the main checkout's`;
  if (!shimsReady(join(own, '_'))) {
    return `${said}; this worktree has no complete ${join(own, '_')} to run its own from: run pnpm install here first`;
  }
  return `${said}; to run this worktree's own hook: brit -c core.hooksPath=${join(own, '_')} push origin HEAD:<ref>`;
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
  const note = skewNote(top, hooks);
  const lines = said => (note ? [said, note] : [said]);
  if (current === target) return lines(`core.hooksPath already ${target}`);
  execFileSync('git', ['config', 'core.hooksPath', target], { cwd, stdio: 'ignore' });
  return lines(`core.hooksPath ${current || '(unset)'} -> ${target}`);
}

try {
  for (const line of [].concat(main(process.cwd()))) console.log(`[hooks-path] ${line}`);
} catch (error) {
  console.error(`[hooks-path] REFUSED: ${error.stderr?.toString().trim() || error.message}`);
  process.exitCode = 1;
}
