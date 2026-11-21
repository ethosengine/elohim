// doc-cut-guard.mjs: a push to dev or main that leaves a long document under half its length (and
// at least 100 lines fewer) at the destination is REFUSED unless a pushed commit that touches it
// carries `[doc:cut]`. Pure cases cover the arithmetic and the attribute matcher; push cases use
// real git repositories. Origin: the five 2026-11-21 landings (backlog "Row 68"). Habit
// `careless-reds-cost-seconds`.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { checkDocCut, docCutMarkHonoured, generatedMatcher, isCut, parseNumstat, parseRefs } from './doc-cut-guard.mjs';

const cleanEnv = Object.fromEntries(Object.entries(process.env).filter(([k]) => !k.startsWith('GIT_') || k.startsWith('GIT_CONFIG_')));
const spawnIn = (cmd, args, opts) => spawnSync(cmd, args, { ...opts, env: cleanEnv });
const doc = (n, name = 'doc') => Array.from({ length: n }, (_, i) => `${name} line ${i} of a long document, distinct so that renames pair\n`).join('');

function repo(t) {
  const dir = mkdtempSync(join(tmpdir(), 'doc-cut-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const git = (...args) => {
    const r = spawnSync('git', ['-C', dir, '-c', 'user.name=t', '-c', 'user.email=t@t', '-c', 'commit.gpgsign=false', '-c', 'core.hooksPath=/dev/null', ...args], { encoding: 'utf8', env: cleanEnv });
    assert.equal(r.status, 0, `git ${args.join(' ')}: ${r.stderr}`);
    return r.stdout.trim();
  };
  const commit = (files, message) => {
    for (const [path, text] of Object.entries(files)) {
      if (text === null) { rmSync(join(dir, path), { force: true }); continue; }
      mkdirSync(join(dir, path, '..'), { recursive: true });
      writeFileSync(join(dir, path), text);
    }
    git('add', '-A');
    const msg = `${dir}.msg`;
    writeFileSync(msg, message);
    t.after(() => rmSync(msg, { force: true }));
    git('commit', '--quiet', '--allow-empty', '-F', msg);
    return git('rev-parse', 'HEAD');
  };
  git('init', '--quiet', '-b', 'dev');
  return { dir, git, commit };
}
const push = (dir, ref, tip, base, extra = {}) => checkDocCut({ root: dir, spawn: spawnIn, env: { GATE_PUSH_TARGETS: ref, GATE_PUSH_REFS: `${ref} ${tip} ${base}`, ...extra } });
const dev = (dir, tip, base) => push(dir, 'refs/heads/dev', tip, base);
const BACKLOG = 'genesis/data/timeline/backlog/steward-device-arrival-open-items.md';

test('pure: a cut is under half AND at least 100 lines fewer', () => {
  assert.equal(isCut(1531, 39), true);
  assert.equal(isCut(150, 80), false, 'not under half');
  assert.equal(isCut(90, 0), false, 'under 100 removed');
  assert.equal(isCut(200, 99), true);
  assert.equal(isCut(200, 100), false, 'exactly half is not under half');
});

test('pure: numstat -z parsing pairs renames and drops binaries', () => {
  const text = '3\t200\ta.md\0-\t-\timg.png\0' + '0\t150\t\0old.md\0new.md\0';
  assert.deepEqual(parseNumstat(text), [{ removed: 200, path: 'a.md', oldPath: 'a.md' }, { removed: 150, path: 'new.md', oldPath: 'old.md' }]);
});

test('pure: .gitattributes linguist-generated, last match wins, basename or anchored', () => {
  const m = generatedMatcher('# c\n*.lock linguist-generated\ngen/** linguist-generated=true\ngen/keep.md -linguist-generated\ndocs/x.md linguist-vendored\n');
  assert.equal(m('a/b.lock'), true);
  assert.equal(m('gen/deep/x.md'), true);
  assert.equal(m('gen/keep.md'), false);
  assert.equal(m('docs/x.md'), false);
  assert.equal(generatedMatcher('')('x.md'), false);
});

test('pure: ref lists and the mark', (t) => {
  assert.deepEqual(parseRefs('refs/heads/dev a b\n'), [{ ref: 'refs/heads/dev', local: 'a', remote: 'b', branch: 'dev' }]);
  const dir = mkdtempSync(join(tmpdir(), 'doc-cut-mark-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  writeFileSync(join(dir, 'm'), 'refs/heads/dev a b');
  assert.equal(docCutMarkHonoured({ GATE_DOC_CUT_MARK: join(dir, 'm'), GATE_PUSH_REFS: 'refs/heads/dev a b' }), true);
  assert.equal(docCutMarkHonoured({ GATE_DOC_CUT_MARK: join(dir, 'm'), GATE_PUSH_REFS: 'refs/heads/dev a c' }), false);
  assert.equal(docCutMarkHonoured({}), false);
});

test('a 1,500-line document rewritten to 40 lines in a push to dev: REFUSED, naming the file and both counts', (t) => {
  const { dir, commit } = repo(t);
  const base = commit({ 'docs/plan.md': doc(1500) }, 'base');
  const tip = commit({ 'docs/plan.md': doc(40) }, 'docs: append a section');
  const rows = dev(dir, tip, base);
  assert.equal(rows.length, 1, JSON.stringify(rows));
  assert.equal(rows[0].level, 'REFUSED');
  assert.equal(rows[0].check, 'doc-cut');
  assert.match(rows[0].why, /push to dev: docs\/plan\.md falls from 1,500 to 40 lines/);
  assert.match(rows[0].cure, /put \[doc:cut\] and why in the message of the commit that makes it; if not, the file's last whole version is git show [0-9a-f]{12}:docs\/plan\.md/);
});

test('regression: the real numbers, 1,531 to 39 and 553 to 38, name both documents', (t) => {
  const { dir, commit } = repo(t);
  const journal = 'genesis/docs/superpowers/sprints/journal.md';
  const base = commit({ [BACKLOG]: doc(1531), [journal]: doc(553) }, 'base');
  const tip = commit({ [BACKLOG]: doc(39), [journal]: doc(38) }, 'docs(wall-clock): append row 68');
  const rows = dev(dir, tip, base);
  assert.equal(rows.length, 2, JSON.stringify(rows));
  assert.ok(rows.every(r => r.level === 'REFUSED'));
  assert.ok(rows.some(r => r.why.includes(`${BACKLOG} falls from 1,531 to 39 lines`)), JSON.stringify(rows));
  assert.ok(rows.some(r => r.why.includes(`${journal} falls from 553 to 38 lines`)), JSON.stringify(rows));
});

test('[doc:cut] in the cutting commit clears it; on a commit that does not touch the file it does not', (t) => {
  const { dir, commit } = repo(t);
  const base = commit({ 'docs/plan.md': doc(1500), 'other.md': 'x\n' }, 'base');
  const cut = commit({ 'docs/plan.md': doc(40) }, 'docs: shrink the plan [doc:cut] superseded by the new plan');
  const ok = dev(dir, cut, base);
  assert.equal(ok.length, 1);
  assert.equal(ok[0].level, 'ok');
  assert.match(ok[0].why, /declared by \[doc:cut\] in [0-9a-f]{12}/);
  const { dir: d2, commit: c2 } = repo(t);
  const b2 = c2({ 'docs/plan.md': doc(1500), 'other.md': 'x\n' }, 'base');
  c2({ 'other.md': 'y\n' }, 'chore: unrelated [doc:cut] not about the plan');
  const tip = c2({ 'docs/plan.md': doc(40) }, 'docs: append');
  const rows = dev(d2, tip, b2);
  assert.equal(rows.length, 1);
  assert.equal(rows[0].level, 'REFUSED');
});

test('a cut restored by a later commit in the same push is clear (the ends agree)', (t) => {
  const { dir, commit } = repo(t);
  const base = commit({ 'docs/plan.md': doc(1500) }, 'base');
  commit({ 'docs/plan.md': doc(40) }, 'oops');
  const tip = commit({ 'docs/plan.md': doc(1500) }, 'restore');
  assert.deepEqual(dev(dir, tip, base), []);
});

test('under half is required, and at least 100 lines fewer', (t) => {
  const { dir, commit } = repo(t);
  const base = commit({ 'a.md': doc(150), 'b.md': doc(90) }, 'base');
  const tip = commit({ 'a.md': doc(80), 'b.md': '' }, 'shrink');
  assert.deepEqual(dev(dir, tip, base), []);
});

test('a deleted file and a renamed-and-unchanged file are clear; a renamed-and-cut file is REFUSED', (t) => {
  const { dir, commit, git } = repo(t);
  const base = commit({ 'gone.md': doc(500, 'gone'), 'move.md': doc(500, 'move'), 'shrunk.md': doc(1500, 'shrunk') }, 'base');
  git('mv', 'move.md', 'moved.md');
  const tip = commit({ 'gone.md': null }, 'delete and rename');
  assert.deepEqual(dev(dir, tip, base), []);
  git('mv', 'shrunk.md', 'docs-shrunk.md');
  const tip2 = commit({ 'docs-shrunk.md': doc(1500, 'shrunk').split('\n').slice(0, 500).join('\n') + '\n' }, 'move and cut');
  const rows = dev(dir, tip2, base);
  assert.equal(rows.length, 1, JSON.stringify(rows));
  assert.equal(rows[0].level, 'REFUSED');
  assert.match(rows[0].why, /docs-shrunk\.md \(was shrunk\.md\) falls from 1,500 to 500 lines/);
});

test('a .rs file, habits.yaml, and a linguist-generated path cut the same way are clear', (t) => {
  const { dir, commit } = repo(t);
  const files = { 'src/lib.rs': doc(500), 'genesis/manifests/habits.yaml': doc(500), 'gen/out.md': doc(500), '.gitattributes': 'gen/** linguist-generated\n' };
  const base = commit(files, 'base');
  const tip = commit({ 'src/lib.rs': 'x\n', 'genesis/manifests/habits.yaml': 'x\n', 'gen/out.md': 'x\n' }, 'cut');
  assert.deepEqual(dev(dir, tip, base), []);
});

test('a push to a feature branch is a WARN, not REFUSED; main is REFUSED', (t) => {
  const { dir, commit } = repo(t);
  const base = commit({ 'docs/plan.md': doc(500) }, 'base');
  const tip = commit({ 'docs/plan.md': doc(10) }, 'cut');
  const warn = push(dir, 'refs/heads/wallclock/x', tip, base);
  assert.equal(warn.length, 1);
  assert.equal(warn[0].level, 'WARN');
  assert.equal(push(dir, 'refs/heads/main', tip, base)[0].level, 'REFUSED');
});

test('a git failure: REFUSED on a dev push naming it, WARN elsewhere', (t) => {
  const { dir, commit } = repo(t);
  const base = commit({ 'a.md': 'x\n' }, 'base');
  const missing = 'deadbeef'.repeat(5);
  const red = dev(dir, missing, base);
  assert.equal(red.length, 1);
  assert.equal(red[0].level, 'REFUSED');
  assert.match(red[0].why, /could not decide: git diff --numstat exited \d+/);
  assert.equal(push(dir, 'refs/heads/f', missing, base)[0].level, 'WARN');
  const unknownBase = dev(dir, base, missing);
  assert.equal(unknownBase[0].level, 'REFUSED');
  assert.match(unknownBase[0].why, /destination tip .* is not in this clone/);
});

test('outside a push: origin/dev against HEAD warns; without origin/dev it says it did not run', (t) => {
  const { dir, commit, git } = repo(t);
  const base = commit({ 'docs/plan.md': doc(500) }, 'base');
  const quiet = checkDocCut({ root: dir, spawn: spawnIn, env: {} });
  assert.equal(quiet.length, 1);
  assert.equal(quiet[0].level, 'WARN');
  assert.match(quiet[0].why, /did not run/);
  git('update-ref', 'refs/remotes/origin/dev', base);
  commit({ 'docs/plan.md': doc(10) }, 'cut');
  const rows = checkDocCut({ root: dir, spawn: spawnIn, env: {} });
  assert.equal(rows.length, 1);
  assert.equal(rows[0].level, 'WARN');
  assert.match(rows[0].why, /falls from 500 to 10 lines/);
});

test('cost: a push of 400 files is judged from one numstat, in seconds', (t) => {
  const { dir, commit } = repo(t);
  const many = n => Object.fromEntries(Array.from({ length: 400 }, (_, i) => [`d/f${i}.md`, doc(n)]));
  const base = commit({ ...many(30), 'big.md': doc(1000) }, 'base');
  const tip = commit({ ...many(5), 'big.md': doc(10) }, 'shrink all');
  const started = Date.now();
  const rows = dev(dir, tip, base);
  assert.equal(rows.length, 1, 'only the file with 100+ removed lines is counted');
  assert.ok(Date.now() - started < 5000);
});
