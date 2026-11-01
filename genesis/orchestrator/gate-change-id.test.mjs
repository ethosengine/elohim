import { test } from 'node:test';
import assert from 'node:assert/strict';
import { chmodSync, mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { changeIdFor, stamp, stampDecision, parseChangeId } from './gate-change-id.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const MODULE = join(HERE, 'gate-change-id.mjs');

function cleanEnv(scratch) {
  const env = { ...process.env };
  delete env.NODE_TEST_CONTEXT;
  for (const key of [
    'GIT_DIR',
    'GIT_WORK_TREE',
    'GIT_INDEX_FILE',
    'GIT_PREFIX',
    'GIT_COMMON_DIR',
    'GATE_COMMIT',
  ])
    delete env[key];
  return {
    ...env,
    GIT_CEILING_DIRECTORIES: dirname(scratch),
    GIT_CONFIG_NOSYSTEM: '1',
    GIT_CONFIG_GLOBAL: '/dev/null',
    HOME: scratch,
    GIT_AUTHOR_NAME: 'Test Author',
    GIT_AUTHOR_EMAIL: 'author@example.test',
    GIT_COMMITTER_NAME: 'Test Committer',
    GIT_COMMITTER_EMAIL: 'committer@example.test',
    GIT_AUTHOR_DATE: '2026-10-31T12:00:00Z',
    GIT_COMMITTER_DATE: '2026-10-31T12:00:00Z',
  };
}

function fixture(t) {
  const dir = mkdtempSync(join(tmpdir(), 'gate-change-id-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const env = cleanEnv(dir);
  const hooks = join(dir, 'hooks');
  mkdirSync(hooks);
  const hook = join(hooks, 'commit-msg');
  writeFileSync(
    hook,
    `#!/bin/sh\ncp "$1" "$HOOK_BEFORE"\n"${process.execPath}" "${MODULE}" stamp "$1"\ncp "$1" "$HOOK_AFTER"\n`,
  );
  chmodSync(hook, 0o755);
  const git = (...args) =>
    spawnSync('git', ['-c', `core.hooksPath=${hooks}`, ...args], {
      cwd: dir,
      env,
      encoding: 'utf8',
    });
  assert.equal(git('init', '-b', 'main').status, 0);
  env.HOOK_BEFORE = join(dir, 'before');
  env.HOOK_AFTER = join(dir, 'after');
  return { dir, env, git };
}

function messageOf(git) {
  return git('log', '-1', '--format=%B').stdout;
}
function ids(message) {
  return message.match(/^Change-Id: (I[0-9a-f]{40})$/gm) || [];
}

test('stamp mints once over final message', async function stamp_mints_once_over_final_message(t) {
  const { dir, env, git } = fixture(t);
  writeFileSync(join(dir, 'a'), 'a');
  assert.equal(git('add', 'a').status, 0);
  assert.equal(git('commit', '-m', 'subject', '-m', 'body').status, 0);
  const message = messageOf(git);
  assert.equal(ids(message).length, 1);
  const parsed = parseChangeId(message);
  assert.match(parsed.id, /^I[0-9a-f]{40}$/);
  const stripped = spawnSync('git', ['stripspace', '--strip-comments'], {
    cwd: dir,
    env,
    encoding: 'utf8',
    input: readFileSync(env.HOOK_BEFORE, 'utf8'),
  });
  const expected = changeIdFor({
    tree: git('rev-parse', 'HEAD^{tree}').stdout.trim(),
    parent: git('hash-object', '-t', 'tree', '/dev/null').stdout.trim(),
    author: `Test Author <author@example.test> ${Date.parse('2026-10-31T12:00:00Z') / 1000} +0000`,
    committer: `Test Committer <committer@example.test> ${Date.parse('2026-10-31T12:00:00Z') / 1000} +0000`,
    message: stripped.stdout.trimEnd(),
  });
  assert.equal(parsed.id, expected);

  const file = join(dir, 'unit-message');
  writeFileSync(file, 'unit subject\n');
  const result = stamp(file, { cwd: dir, env });
  assert.equal(result.why, null);
  assert.equal(result.stamped, parseChangeId(readFileSync(file, 'utf8')).id);
});

test('amend and rebase keep one trailer', async function amend_and_rebase_keep_one_trailer(t) {
  const { dir, git } = fixture(t);
  writeFileSync(join(dir, 'a'), 'a');
  git('add', 'a');
  assert.equal(git('commit', '-m', 'one').status, 0);
  const id = parseChangeId(messageOf(git)).id;
  assert.equal(git('commit', '--amend', '--no-edit').status, 0);
  assert.equal(parseChangeId(messageOf(git)).id, id);
  assert.equal(ids(messageOf(git)).length, 1);
  assert.equal(git('branch', 'topic').status, 0);
  writeFileSync(join(dir, 'up'), 'up');
  git('add', 'up');
  assert.equal(git('commit', '-m', 'upstream').status, 0);
  assert.equal(git('checkout', 'topic').status, 0);
  writeFileSync(join(dir, 'topic'), 'topic');
  git('add', 'topic');
  assert.equal(git('commit', '-m', 'topic').status, 0);
  const topicId = parseChangeId(messageOf(git)).id;
  assert.equal(git('rebase', 'main').status, 0);
  assert.equal(parseChangeId(messageOf(git)).id, topicId);
  assert.equal(ids(messageOf(git)).length, 1);
});

test('existing trailer untouched', async function existing_trailer_untouched(t) {
  const { dir, env, git } = fixture(t);
  const id = `I${'1'.repeat(40)}`;
  const file = join(dir, 'msg');
  const bytes = `subject\n\nbody\n\nChange-Id: ${id}\nCo-Authored-By: X <x@y>\n`;
  writeFileSync(file, bytes);
  assert.deepEqual(stamp(file, { cwd: dir, env }), {
    stamped: null,
    why: 'Change-Id present',
  });
  assert.equal(readFileSync(file, 'utf8'), bytes);
  writeFileSync(join(dir, 'a'), 'a');
  git('add', 'a');
  assert.equal(git('commit', '-F', file).status, 0);
  assert.deepEqual(readFileSync(env.HOOK_BEFORE), readFileSync(env.HOOK_AFTER));
});

test('stamp reports malformed trailer', async function stamp_reports_malformed_trailer(t) {
  const { dir, env } = fixture(t);
  const file = join(dir, 'msg');
  for (const [message, why] of [
    ['s\n\nChange-Id: foo\n', 'malformed Change-Id: foo'],
    [`s\n\nChange-Id: I${'a'.repeat(40)}\nChange-Id: I${'b'.repeat(40)}\n`, '2 Change-Id trailers'],
  ]) {
    writeFileSync(file, message);
    assert.deepEqual(stamp(file, { cwd: dir, env }), { stamped: null, why });
    assert.equal(readFileSync(file, 'utf8'), message);
    const cli = spawnSync(process.execPath, [MODULE, 'stamp', file], {
      cwd: dir,
      env,
      encoding: 'utf8',
    });
    assert.equal(cli.status, 0);
    assert.equal(cli.stderr, `[change-id] not stamped: ${why} (cure: git commit --amend)\n`);
  }
});

test('fixup and empty messages not stamped', async function fixup_and_empty_messages_not_stamped(t) {
  assert.equal(stampDecision('fixup! x', parseChangeId('fixup! x')).stamp, false);
  assert.deepEqual(stampDecision('', parseChangeId('')), {
    stamp: false,
    why: 'empty message',
  });
});

test('stamp never refuses a commit', async function stamp_never_refuses_a_commit(t) {
  const { dir, env } = fixture(t);
  const file = join(dir, 'msg');
  writeFileSync(file, 'subject\n');
  const real = spawnSync;
  const spawn = (bin, args, opts) =>
    args[0] === 'var' ? { status: 128, stdout: '', stderr: 'fatal: var' } : real(bin, args, opts);
  assert.match(stamp(file, { cwd: dir, env, spawn }).why, /^git var failed/);
  const outside = mkdtempSync(join(tmpdir(), 'change-id-outside-'));
  t.after(() => rmSync(outside, { recursive: true, force: true }));
  const result = spawnSync(process.execPath, [MODULE, 'stamp', file], {
    cwd: outside,
    env: cleanEnv(outside),
    encoding: 'utf8',
  });
  assert.equal(result.status, 0);
  assert.match(result.stderr, /^\[change-id\] not stamped:/);
});

test('trailer write confirmed', async function trailer_write_confirmed(t) {
  const { dir, env } = fixture(t);
  const file = join(dir, 'msg');
  writeFileSync(file, 'subject\n');
  const spawn = (bin, args, opts) =>
    args[0] === 'interpret-trailers'
      ? { status: 0, stdout: '', stderr: '' }
      : spawnSync(bin, args, opts);
  assert.match(stamp(file, { cwd: dir, env, spawn }).why, /^trailer write did not take/);
});

test('partial commit hashes the commit index', async function partial_commit_hashes_the_commit_index(t) {
  const { dir, git } = fixture(t);
  writeFileSync(join(dir, 'p'), 'p0');
  writeFileSync(join(dir, 'q'), 'q0');
  git('add', 'p', 'q');
  assert.equal(git('commit', '-m', 'root').status, 0);
  writeFileSync(join(dir, 'p'), 'p1');
  git('add', 'p');
  writeFileSync(join(dir, 'q'), 'q1');
  assert.equal(git('commit', 'q', '-m', 'partial').status, 0);
  assert.match(parseChangeId(messageOf(git)).id, /^I[0-9a-f]{40}$/);
  assert.equal(git('show', 'HEAD:p').stdout, 'p0');
  assert.equal(git('show', 'HEAD:q').stdout, 'q1');
});

test('amend with new message mints new id', async function amend_with_new_message_mints_new_id(t) {
  const { dir, git } = fixture(t);
  writeFileSync(join(dir, 'a'), 'a');
  git('add', 'a');
  assert.equal(git('commit', '-m', 'old').status, 0);
  const old = parseChangeId(messageOf(git)).id;
  assert.equal(git('commit', '--amend', '-m', 'new').status, 0);
  assert.notEqual(parseChangeId(messageOf(git)).id, old);
});
