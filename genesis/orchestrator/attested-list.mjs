#!/usr/bin/env node
// The one resolver for the attested-submodule list (attested-submodules.txt), read by the pre-push
// hook (CLI below), gate-preflight.mjs and worktree-prep.mjs, so none can drift from another.
//
// A rule about a push is read from the destination as well as the pushed tip: a submodule is
// required when it is in EITHER list. Adding one takes effect at once; removing one takes effect
// only after the removal has landed through a push that still checked it.
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const LIST_PATH = 'genesis/orchestrator/attested-submodules.txt';
const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const ZERO = /^0+$/;

const git = (root, args) => execFileSync('git', ['-C', root, ...args], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });

/** The submodule paths a .gitmodules text declares. */
export function modulePaths(text) {
  return new Set(String(text).split('\n').map(line => /^\s*path\s*=\s*(.+?)\s*$/.exec(line)?.[1]).filter(Boolean));
}

/** Parse and validate one list text against the .gitmodules paths. Throws one line naming the cause. */
export function parseList(text, declared, source) {
  const seen = new Set();
  for (const line of String(text).split('\n')) {
    if (line === '' || line.startsWith('#')) continue;
    if (line.includes('\r')) throw new Error(`${source}: a line has a carriage return`);
    if (line !== line.trim()) throw new Error(`${source}: line "${line.trim()}" has leading or trailing whitespace`);
    if (!declared.has(line)) throw new Error(`${source}: "${line}" is not a path in .gitmodules`);
    if (seen.has(line)) throw new Error(`${source}: "${line}" is listed twice`);
    seen.add(line);
  }
  if (!seen.size) throw new Error(`${source}: the list is empty`);
  return [...seen];
}

const blob = (root, commit, path) => git(root, ['show', `${commit}:${path}`]);
const resolves = (root, ref) => { try { git(root, ['rev-parse', '--verify', '--quiet', `${ref}^{commit}`]); return true; } catch { return false; } };

function fromCommit(root, commit, source) {
  return parseList(blob(root, commit, LIST_PATH), modulePaths(blob(root, commit, '.gitmodules')), source);
}

/**
 * The required list. `pushLines` are the hook's stdin lines (`local_ref local_sha remote_ref
 * remote_sha`); with none, the working tree stands for the pushed tip and origin/dev for the destination.
 */
export function attestedSubmodules({ root = ROOT, pushLines = [], remote = 'origin' } = {}) {
  const required = new Set();
  const add = list => list.forEach(path => required.add(path));
  const pushes = pushLines.map(line => line.trim().split(/\s+/)).filter(f => f.length >= 3);
  if (!pushes.length) {
    const text = readFileSync(join(root, LIST_PATH), 'utf8');
    add(parseList(text, modulePaths(readFileSync(join(root, '.gitmodules'), 'utf8')), `${LIST_PATH} (working tree)`));
  }
  const destinations = new Set();
  for (const [, localSha, remoteRef] of pushes) {
    if (localSha && !ZERO.test(localSha)) add(fromCommit(root, localSha, `${LIST_PATH} at pushed ${localSha.slice(0, 12)}`));
    const branch = String(remoteRef).replace(/^refs\/heads\//, '');
    const tracking = `refs/remotes/${remote}/${branch}`;
    destinations.add(resolves(root, tracking) ? tracking : `refs/remotes/${remote}/dev`);
  }
  if (!pushes.length) destinations.add(`refs/remotes/${remote}/dev`);
  for (const ref of destinations) {
    if (!resolves(root, ref)) continue; // nothing has landed there: no earlier version to keep
    let present = true;
    try { git(root, ['cat-file', '-e', `${ref}:${LIST_PATH}`]); } catch { present = false; }
    if (present) add(fromCommit(root, ref, `${LIST_PATH} at destination ${ref}`)); // a malformed landed list refuses
  }
  return [...required];
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const stdin = process.argv.includes('--push-stdin') ? readFileSync(0, 'utf8').split('\n').filter(Boolean) : [];
    const rootArg = process.argv.indexOf('--root');
    process.stdout.write(`${attestedSubmodules({ root: rootArg > 0 ? process.argv[rootArg + 1] : ROOT, pushLines: stdin }).join('\n')}\n`);
  } catch (error) {
    process.stderr.write(`attested-list: ${String(error.stderr || error.message).trim().split('\n')[0]}\n`);
    process.exitCode = 1;
  }
}
