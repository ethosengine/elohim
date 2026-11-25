// The attested-submodule list resolver: malformed lists refuse, and a rule about a push is read
// from the destination's landed list as well as the pushed tip (the union).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { attestedSubmodules, LIST_PATH, modulePaths, parseList } from './attested-list.mjs';

const ROOT = fileURLToPath(new URL('../../', import.meta.url));
const HOOK = readFileSync(join(ROOT, '.husky/pre-push.bash'), 'utf8');
const MODULES = ['elohim/brit', 'elohim/rakia', 'sophia', 'extra/mod'];
const gitmodules = MODULES.map(path => `[submodule "${path}"]\n\tpath = ${path}\n\turl = x\n`).join('');

function repo(t) {
  const dir = mkdtempSync(join(tmpdir(), 'attested-list-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const git = (...args) => {
    const r = spawnSync('git', ['-C', dir, '-c', 'user.name=t', '-c', 'user.email=t@t', ...args], { encoding: 'utf8' });
    assert.equal(r.status, 0, r.stderr);
    return r.stdout.trim();
  };
  git('init', '-q', '-b', 'main');
  const commit = (list, message = 'c') => {
    mkdirSync(join(dir, 'genesis/orchestrator'), { recursive: true });
    writeFileSync(join(dir, '.gitmodules'), gitmodules);
    writeFileSync(join(dir, LIST_PATH), list);
    git('add', '-A');
    git('commit', '-q', '-m', message);
    return git('rev-parse', 'HEAD');
  };
  return { dir, git, commit };
}
const ZERO = '0'.repeat(40);
const push = (sha, ref) => `refs/heads/local ${sha} ${ref} ${ZERO}`;
const LIST = 'elohim/brit\nelohim/rakia\nsophia\n';

test('parseList: a path absent from .gitmodules, a CR, padding, a duplicate and an empty list each refuse', () => {
  const declared = modulePaths(gitmodules);
  assert.deepEqual(parseList(LIST, declared, 'l'), ['elohim/brit', 'elohim/rakia', 'sophia']);
  assert.throws(() => parseList('elohim/brit\nnot/a/module\n', declared, 'l'), /"not\/a\/module" is not a path in \.gitmodules/);
  assert.throws(() => parseList('elohim/brit\r\nsophia\n', declared, 'l'), /carriage return/);
  assert.throws(() => parseList(' sophia\n', declared, 'l'), /whitespace/);
  assert.throws(() => parseList('sophia \n', declared, 'l'), /whitespace/);
  assert.throws(() => parseList('sophia\nsophia\n', declared, 'l'), /listed twice/);
  assert.throws(() => parseList('# only a comment\n\n', declared, 'l'), /empty/);
  assert.throws(() => parseList('   \n', declared, 'l'), /whitespace/);
});

test('a push that removes sophia from the list is still required to check it out (union with the destination)', t => {
  const r = repo(t);
  const landed = r.commit(LIST);
  r.git('update-ref', 'refs/remotes/origin/dev', landed);
  const pushed = r.commit('elohim/brit\nelohim/rakia\n', 'drop sophia');
  const required = attestedSubmodules({ root: r.dir, pushLines: [push(pushed, 'refs/heads/dev')] });
  assert.deepEqual([...required].sort(), ['elohim/brit', 'elohim/rakia', 'sophia']);
  // After the removal has landed, the next push no longer requires it.
  r.git('update-ref', 'refs/remotes/origin/dev', pushed);
  assert.deepEqual([...attestedSubmodules({ root: r.dir, pushLines: [push(pushed, 'refs/heads/dev')] })].sort(), ['elohim/brit', 'elohim/rakia']);
});

test('a push that adds a submodule requires it at once', t => {
  const r = repo(t);
  r.git('update-ref', 'refs/remotes/origin/dev', r.commit(LIST));
  const pushed = r.commit(`${LIST}extra/mod\n`, 'add');
  assert.ok(attestedSubmodules({ root: r.dir, pushLines: [push(pushed, 'refs/heads/dev')] }).includes('extra/mod'));
});

test('a new ref uses origin/dev as its destination list', t => {
  const r = repo(t);
  r.git('update-ref', 'refs/remotes/origin/dev', r.commit(LIST));
  const pushed = r.commit('elohim/brit\n', 'narrow');
  const required = attestedSubmodules({ root: r.dir, pushLines: [push(pushed, 'refs/heads/self/new')] });
  assert.deepEqual([...required].sort(), ['elohim/brit', 'elohim/rakia', 'sophia']);
});

test('a malformed landed list at the destination refuses too', t => {
  const r = repo(t);
  r.git('update-ref', 'refs/remotes/origin/dev', r.commit('elohim/brit\nnot/a/module\n'));
  const pushed = r.commit(LIST, 'fix');
  assert.throws(() => attestedSubmodules({ root: r.dir, pushLines: [push(pushed, 'refs/heads/dev')] }), /destination/);
});

// The hook end to end: its own two functions, with real git, against the fixture repository.
function hookRun(r, pushLines) {
  const fn = name => { const s = HOOK.indexOf(`${name}() {\n`); return HOOK.slice(s, HOOK.indexOf('\n}\n', s) + 3); };
  const script = [
    `HOOK_WORK_TREE=${JSON.stringify(r.dir)}`,
    `PUSH_LINES=(${pushLines.map(l => JSON.stringify(l)).join(' ')})`,
    `export HOOK_WORK_TREE`,
    // the resolver is the repository's own, not the fixture's
    fn('attested_submodules_list').replace('"${HOOK_WORK_TREE:-.}/genesis/orchestrator/attested-list.mjs"', JSON.stringify(join(ROOT, 'genesis/orchestrator/attested-list.mjs'))),
    fn('attested_submodules_missing'),
    'cd "$HOOK_WORK_TREE"',
    'list=$(attested_submodules_list) || { echo LISTFAIL; exit 3; }',
    'mapfile -t L <<<"$list"',
    'attested_submodules_missing "${L[@]}"; echo "rc=$?"',
  ].join('\n');
  return spawnSync('bash', ['-c', script], { encoding: 'utf8', env: { ...process.env, GIT_DIR: undefined, GIT_WORK_TREE: undefined } });
}

test('hook: a list naming a path git cannot map fails closed, never reads as success', t => {
  const r = repo(t);
  const sha = r.commit('elohim/brit\nnot/a/module\n');
  const out = hookRun(r, [push(sha, 'refs/heads/dev')]);
  assert.equal(out.status, 3, out.stdout + out.stderr);
  assert.match(out.stderr, /attested-list: .*not\/a\/module/);
});

test('hook: a CR in the list fails closed', t => {
  const r = repo(t);
  const sha = r.commit('elohim/brit\r\nsophia\n');
  const out = hookRun(r, [push(sha, 'refs/heads/dev')]);
  assert.equal(out.status, 3, out.stdout + out.stderr);
  assert.match(out.stderr, /carriage return/);
});

test('hook: a git failure while reading submodule state is a refusal with one line naming it', t => {
  const r = repo(t);
  const sha = r.commit(LIST);
  const out = hookRun(r, [push(sha, 'refs/heads/dev')]);
  // The fixture has no submodule checkouts: the declared paths are not gitlinks, so git status fails or lists them missing.
  assert.match(out.stdout + out.stderr, /git submodule status failed|^-|rc=[12]/m);
  assert.doesNotMatch(out.stdout, /^rc=0$/m);
});

test('hook: reads the resolver from the one shared module, not a hard-coded list', () => {
  assert.match(HOOK, /attested-list\.mjs/);
  assert.doesNotMatch(HOOK, /submodule status -- elohim\/brit/);
});
