// One advisory file lock for the orchestrator's read-modify-write files (a gate run's event log,
// the cycle ledger). Synchronous, because both callers are synchronous.
//
// The lock is `<path>.lock`, created exclusively and holding the holder's { pid, start } (start =
// field 22 of /proc/<pid>/stat, so a reused pid is not mistaken for the holder). A lock whose holder
// is gone is taken over; an unreadable lock older than 2 s is a writer that died between create and
// write, and is taken over. A lock held by a live process is NEVER broken, whatever its age. On
// timeout `fn` runs without the lock and one stderr line says so: the gate's exit status is the
// verdict, so an event is never lost to the lock, and the lock never throws.

import { closeSync, openSync, readFileSync, statSync, unlinkSync, writeSync } from 'fs';

const NAP = new Int32Array(new SharedArrayBuffer(4));
const UNREADABLE_STALE_MS = 2000;

/** Start time (clock ticks since boot) of `pid`, or null when it is gone. */
export function processStart(pid, procDir = '/proc') {
  try {
    const text = readFileSync(`${procDir}/${pid}/stat`, 'utf8');
    const fields = text.slice(text.lastIndexOf(')') + 2).split(' ');
    return fields[19] ?? null;
  } catch { return null; }
}

function readHolder(lock) {
  try {
    const holder = JSON.parse(readFileSync(lock, 'utf8'));
    return holder && Number.isInteger(holder.pid) ? holder : null;
  } catch { return null; }
}

export function withFileLock(path, fn, { waitMs = 10000 } = {}) {
  const lock = `${path}.lock`;
  const me = { pid: process.pid, start: processStart(process.pid) };
  const deadline = Date.now() + waitMs;
  let held = false;
  let blocker = null;
  try {
    for (;;) {
      try {
        const fd = openSync(lock, 'wx');
        try { writeSync(fd, JSON.stringify(me)); } finally { closeSync(fd); }
        held = true;
        break;
      } catch (error) {
        if (error.code !== 'EEXIST') break; // unwritable directory and the like: run unlocked
      }
      const holder = readHolder(lock);
      let breakIt = false;
      if (holder) {
        const start = processStart(holder.pid);
        breakIt = start === null || (holder.start != null && start !== holder.start);
        blocker = `pid ${holder.pid}`;
      } else {
        try { breakIt = Date.now() - statSync(lock).mtimeMs > UNREADABLE_STALE_MS; } catch { breakIt = false; }
        blocker = 'an unreadable lock';
      }
      if (breakIt) {
        try { unlinkSync(lock); } catch { /* another waiter took it first */ }
        continue;
      }
      if (Date.now() >= deadline) {
        process.stderr.write(`[file-lock] ${lock} still held by ${blocker} after ${waitMs} ms; proceeding unlocked\n`);
        break;
      }
      Atomics.wait(NAP, 0, 0, 10);
    }
  } catch { held = held && true; }
  try {
    return fn();
  } finally {
    if (held) {
      try {
        const holder = readHolder(lock);
        if (holder && holder.pid === me.pid && holder.start === me.start) unlinkSync(lock);
      } catch { /* already gone */ }
    }
  }
}
