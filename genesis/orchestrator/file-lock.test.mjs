// withFileLock: the one advisory lock the gate's read-modify-write files take.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, utimesSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawn } from 'node:child_process';
import { processStart, withFileLock } from './file-lock.mjs';

const scratch = t => {
  const dir = mkdtempSync(join(tmpdir(), 'file-lock-'));
  t.after(() => { try { chmodSync(dir, 0o755); } catch { /* gone */ } rmSync(dir, { recursive: true, force: true }); });
  return dir;
};

function captureStderr(fn) {
  const lines = [];
  const real = process.stderr.write;
  process.stderr.write = chunk => { lines.push(String(chunk)); return true; };
  try { return { result: fn(), lines }; } finally { process.stderr.write = real; }
}

test('runs fn under the lock and releases it', t => {
  const path = join(scratch(t), 'ledger');
  const seen = withFileLock(path, () => existsSync(`${path}.lock`));
  assert.equal(seen, true);
  assert.equal(existsSync(`${path}.lock`), false);
});

test('a stale lock of a dead pid is taken over', t => {
  const path = join(scratch(t), 'ledger');
  const dead = spawn(process.execPath, ['-e', '0']);
  return new Promise(done => dead.on('exit', () => {
    writeFileSync(`${path}.lock`, JSON.stringify({ pid: dead.pid, start: '1' }));
    const { lines } = captureStderr(() => assert.equal(withFileLock(path, () => 'ran', { waitMs: 2000 }), 'ran'));
    assert.deepEqual(lines, [], 'taken over silently');
    assert.equal(existsSync(`${path}.lock`), false);
    done();
  }));
});

test('a lock whose pid was reused by a process with another start time is taken over', t => {
  const path = join(scratch(t), 'ledger');
  writeFileSync(`${path}.lock`, JSON.stringify({ pid: process.pid, start: 'not-my-start' }));
  assert.equal(withFileLock(path, () => 'ran', { waitMs: 2000 }), 'ran');
});

test('an unparsable lock is taken over only once it is older than 2 s', t => {
  const path = join(scratch(t), 'ledger');
  writeFileSync(`${path}.lock`, 'not json');
  const young = captureStderr(() => withFileLock(path, () => 'ran', { waitMs: 150 }));
  assert.equal(young.result, 'ran');
  assert.match(young.lines.join(''), /still held by an unreadable lock/, 'a young unreadable lock is waited for');
  assert.equal(existsSync(`${path}.lock`), true, 'and left alone');
  const old = new Date(Date.now() - 5000);
  utimesSync(`${path}.lock`, old, old);
  const aged = captureStderr(() => withFileLock(path, () => 'ran', { waitMs: 2000 }));
  assert.equal(aged.result, 'ran');
  assert.deepEqual(aged.lines, []);
  assert.equal(existsSync(`${path}.lock`), false);
});

test('a live holder is never broken, however old; the waiter proceeds unlocked after waitMs and says so', t => {
  const path = join(scratch(t), 'ledger');
  const holder = spawn('sleep', ['30']);
  t.after(() => holder.kill('SIGKILL'));
  const start = processStart(holder.pid);
  assert.ok(start, 'the holder is alive');
  writeFileSync(`${path}.lock`, JSON.stringify({ pid: holder.pid, start }));
  const long = new Date(Date.now() - 3_600_000);
  utimesSync(`${path}.lock`, long, long);
  const began = Date.now();
  const { result, lines } = captureStderr(() => withFileLock(path, () => 'ran', { waitMs: 300 }));
  assert.equal(result, 'ran');
  assert.ok(Date.now() - began >= 290, 'it waited');
  assert.match(lines.join(''), new RegExp(`\\[file-lock\\] .*ledger\\.lock still held by pid ${holder.pid} after 300 ms`));
  assert.deepEqual(JSON.parse(readFileSync(`${path}.lock`, 'utf8')), { pid: holder.pid, start }, 'the holder keeps its lock');
});

test('release does not remove another holder\'s lock', t => {
  const path = join(scratch(t), 'ledger');
  withFileLock(path, () => {
    // Someone broke our lock and took it while fn ran.
    writeFileSync(`${path}.lock`, JSON.stringify({ pid: 1, start: 'other' }));
  });
  assert.deepEqual(JSON.parse(readFileSync(`${path}.lock`, 'utf8')), { pid: 1, start: 'other' });
});

test('an unwritable lock directory still runs fn', t => {
  const dir = scratch(t);
  const missing = join(dir, 'no-such-dir', 'ledger');
  assert.equal(withFileLock(missing, () => 'ran'), 'ran', 'a missing directory');
  if (process.getuid?.() !== 0) {
    const ro = join(dir, 'ro');
    mkdirSync(ro);
    chmodSync(ro, 0o555);
    assert.equal(withFileLock(join(ro, 'ledger'), () => 'ran'), 'ran', 'a read-only directory');
  }
});

test('fn\'s own error propagates and still releases the lock', t => {
  const path = join(scratch(t), 'ledger');
  assert.throws(() => withFileLock(path, () => { throw new Error('boom'); }), /boom/);
  assert.equal(existsSync(`${path}.lock`), false);
});
