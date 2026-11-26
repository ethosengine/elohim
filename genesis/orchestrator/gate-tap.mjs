#!/usr/bin/env node
// Runs one gate command and reads its output as it prints, without changing it.
//
//   node gate-tap.mjs --project <name> [--run <gate-live run path>] [--dir <project dir>] -- <command> [args…]
//
// Bytes reach this process's stdout/stderr unchanged and the command's exit status (or
// signal) is this process's. Failure-level lines (gate-failure-lines.mjs) are appended to
// the live run as they appear, so the session that started a push can begin on the first
// red while later gates still run. A notice (a withheld offer on a green gate) is appended
// too, but only a failure starts the outlook or a sibling red. When fd 3 is open, the classifyFailure signals and the
// first-red time are written there as one JSON object for the caller's cycle row.
// Nothing here decides a verdict. The tap is the emitter adapter a CI stage or
// rakia-executor can also wrap: genesis/docs/superpowers/specs/2026-10-26-gate-live-failure-channel-design.md
//
// The command runs as the leader of its own session and process group, so a stop can end the whole
// gate: when the live run carries a stop-now request (`gate-live.mjs stop --now`), the tap ends that
// group (gate-stop.mjs: TERM, then KILL after GATE_STOP_GRACE_MS, default 10 s), drops every
// failure line printed while it ends, and reports `stopped` on fd 3 so gate-runner records nothing
// for the gate. A signal the tap itself receives is forwarded to the group, not only the leader.
//
// A Cargo-slot holder follows only the gate's original Linux session. After the recipe leader
// closes it drains that session for at most 60 s, then TERM for 10 s, then KILL. A double-forked
// `setsid` escapee is outside this custody and outside the slot lifetime; it can only be reported by
// the separately bounded inherited-output check. Non-slot gates never start this session scanner.
// The ownership token proves an open descriptor, not flock ownership, so an ancestor could forge
// it; this lock exists to stop accidents between honest gates, not a process deliberately lying to
// its own device.
// Stated limits: the holder's non-zero exit when members survive KILL is not propagated, so a recipe
// that passed keeps its green (the verdict is true; the survivor line is printed; the risk is to the
// next gate of that slot, as before this lock). If the holder is killed after the launcher verified
// it, the recipe runs on without the lock: an external kill gives today's unlocked behaviour, not worse.

import { spawn } from 'child_process';
import {
  existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, renameSync, rmSync, statSync,
  writeFileSync, writeSync,
} from 'fs';
import { tmpdir } from 'os';
import { dirname, isAbsolute, join, resolve } from 'path';
import { createRecognizer } from './gate-failure-lines.mjs';
import { GATE_ATTRIBUTION_WINDOW_MS, attribute, readEdits } from './gate-attribution.mjs';
import { setTimeout as delay } from 'timers/promises';
import { appendEvent, emitRedDetached, liveRoot, noteUnsentReds, outlook, stopNowRequest } from './gate-live.mjs';
import { endTree, gateTree, outputTargets, processesHolding, readStat, signalTree, snapshot, stopGraceMs, stoppedBy } from './gate-stop.mjs';

// After the gate's tree is ended, how long the tap still waits for its output pipes to close.
const ESCAPED_WAIT_MS = 2000;
const SLOT_BUSY_EXIT = 73;
const SLOT_CONFLICT_EXIT = 200;
const SLOT_OWNER_ENV = 'GATE_SLOT_OWNER_TOKEN';
const SLOT_DRAIN_MS = 60_000;
const SLOT_DRAIN_TERM_MS = 10_000;
const SLOT_DRAIN_KILL_MS = 10_000;
const SLOT_OWNER_READY_MS = positiveMs(process.env.GATE_SLOT_OWNER_READY_MS, 60_000);
const SLOT_EMPTY_CONFIRM_MS = positiveMs(process.env.GATE_SLOT_EMPTY_CONFIRM_MS, 100);
const SLOT_SUPERVISOR_GONE_MS = positiveMs(process.env.GATE_SLOT_SUPERVISOR_GONE_MS, 1_000);

function oneLine(value) {
  return String(value ?? '').replace(/[\r\n]+/g, ' ').trim();
}

function currentHolder(path) {
  try { return oneLine(readFileSync(path, 'utf8')) || 'holder details unavailable'; }
  catch { return 'holder details unavailable'; }
}

function slotWaitLimit(env) {
  const value = Number(env.GATE_SLOT_LOCK_WAIT_S ?? 3600);
  return Number.isFinite(value) && value >= 0 ? value : 3600;
}

function slotHeartbeatMs(env) {
  const value = Number(env.GATE_SLOT_WAIT_HEARTBEAT_S ?? 180);
  return Number.isFinite(value) && value > 0 ? Math.max(10, value * 1000) : 180_000;
}

function writeSlotWait(path, seconds, holder) {
  if (!path) return;
  try { writeFileSync(path, `${seconds.toFixed(6)}\t${holder}\n`, 'utf8'); } catch { /* telemetry */ }
}

function publishHolder(path, project, lockPid, run) {
  if (!path) return;
  try {
    const holder = readStat(lockPid);
    const supervisor = readStat(process.pid);
    if (!holder || !supervisor) return;
    mkdirSync(dirname(path), { recursive: true });
    const temporary = `${path}.${process.pid}`;
    writeFileSync(temporary,
      `project=${project} worktree=${process.cwd()} pid=${lockPid} start=${holder.start} supervisor=${supervisor.pid} supervisor_start=${supervisor.start} `
      + `run=${run || 'unset'} started=${new Date().toISOString().replace(/\.\d{3}Z$/, 'Z')}\n`);
    renameSync(temporary, path);
  } catch { /* a missing holder line never weakens the lock */ }
}

function signalExit(signal) {
  return { SIGHUP: 129, SIGINT: 130, SIGTERM: 143 }[signal] ?? 1;
}

function holderToken(lock, holder, supervisor) {
  return JSON.stringify({ v: 1, lock: resolve(lock), holder, supervisor });
}

function processAncestors(pid) {
  const ancestors = new Set();
  for (let current = readStat(pid), depth = 0; current && current.ppid > 1 && depth < 128; depth += 1) {
    const parent = readStat(current.ppid);
    if (!parent) break;
    ancestors.add(`${parent.pid}:${parent.start}`);
    current = parent;
  }
  return ancestors;
}

function processHoldsFile(pid, path) {
  let expected;
  try { expected = statSync(path); } catch { return false; }
  let fds;
  try { fds = readdirSync(`/proc/${pid}/fd`); } catch { return false; }
  return fds.some(fd => {
    try {
      const actual = statSync(`/proc/${pid}/fd/${fd}`);
      return actual.dev === expected.dev && actual.ino === expected.ino;
    } catch { return false; }
  });
}

function tokenFromHolder(path, lock) {
  try {
    const text = readFileSync(path, 'utf8');
    const field = name => text.match(new RegExp(`(?:^| )${name}=([^ ]+)(?: |$)`))?.[1];
    return holderToken(lock,
      { pid: Number(field('pid')), start: field('start') },
      { pid: Number(field('supervisor')), start: field('supervisor_start') });
  } catch { return null; }
}

/** A nested gate may reuse only the live holder supervised by one of its verified ancestors. */
export function verifySlotOwnerToken(lock, token = process.env[SLOT_OWNER_ENV], pid = process.pid,
  holderPath = null, report = () => {}) {
  const refuse = reason => { report(reason); return false; };
  let parsed;
  try { parsed = JSON.parse(token || tokenFromHolder(holderPath, lock)); }
  catch { return refuse('token is absent or malformed'); }
  if (parsed?.v !== 1 || resolve(parsed.lock ?? '') !== resolve(lock)) return refuse('token names a different lock');
  const holder = readStat(Number(parsed.holder?.pid));
  const supervisor = readStat(Number(parsed.supervisor?.pid));
  if (!holder || holder.start !== parsed.holder?.start) return refuse('holder pid/start is not live');
  if (!supervisor || supervisor.start !== parsed.supervisor?.start) return refuse('supervisor pid/start is not live');
  if (holder.ppid !== supervisor.pid) return refuse('holder is not parented to its supervisor');
  if (!processHoldsFile(holder.pid, lock)) return refuse('holder descriptor is not visible');
  const ancestors = processAncestors(pid);
  if (!ancestors.has(`${holder.pid}:${holder.start}`) && !ancestors.has(`${supervisor.pid}:${supervisor.start}`)) {
    return refuse('holder and supervisor are not ancestors of the gate');
  }
  return true;
}

function positiveMs(value, fallback) {
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed >= 0 ? parsed : fallback;
}

function writeHandoff(path, content) {
  const temporary = `${path}.${process.pid}.tmp`;
  writeFileSync(temporary, content);
  renameSync(temporary, path);
}

export async function drainSlotSession(sid, leaderStart, {
  proc = process.env.GATE_SLOT_PROC_ROOT || '/proc',
  scan = () => snapshot({ proc }),
  now = () => performance.now(),
  sleep = delay,
  send = signalTree,
  write = line => writeSync(2, line),
  drainMs = positiveMs(process.env.GATE_SLOT_DRAIN_MS, SLOT_DRAIN_MS),
  termMs = positiveMs(process.env.GATE_SLOT_DRAIN_TERM_MS, SLOT_DRAIN_TERM_MS),
  killMs = positiveMs(process.env.GATE_SLOT_DRAIN_KILL_MS, SLOT_DRAIN_KILL_MS),
  heartbeatMs = Math.max(10, positiveMs(process.env.GATE_SLOT_DRAIN_HEARTBEAT_MS, 5000)),
  leaderObserved = false,
  emptyConfirmMs = SLOT_EMPTY_CONFIRM_MS,
} = {}) {
  let drainAt = null;
  let termAt = null;
  let killAt = null;
  let lastHeartbeat = 0;
  let sawLeader = leaderObserved;
  let emptyAt = null;
  for (;;) {
    const table = scan();
    const members = [...table.values()].filter(stat => stat.sid === sid && stat.state !== 'Z' && stat.state !== 'X');
    const currentLeader = table.get(sid);
    const leaderLive = currentLeader && currentLeader.state !== 'Z' && currentLeader.state !== 'X';
    // A leader entry whose start time is not the gate's, in any state (live, zombie or exited but
    // still listed), means the sid was reused: the gate session is gone. Release, signal nothing.
    if (currentLeader && currentLeader.start !== leaderStart) return 0;
    if (leaderLive) {
      sawLeader = true;
      emptyAt = null;
      drainAt = null;
      await sleep(50);
      continue;
    }
    if (!sawLeader) {
      await sleep(50);
      continue;
    }
    if (members.length === 0) {
      const at = now();
      if (emptyAt === null) emptyAt = at;
      if (at - emptyAt < emptyConfirmMs) {
        await sleep(50);
        continue;
      }
      if (termAt !== null && killAt === null) write(`[gate-slot] session ${sid} exceeded the ${drainMs} ms drain; sent SIGTERM\n`);
      return 0;
    }
    emptyAt = null;
    const at = now();
    if (drainAt === null) drainAt = at;
    if (at - lastHeartbeat >= heartbeatMs) {
      write(`[gate-slot] draining session ${sid} after its recipe closed; members: ${members.map(p => p.pid).join(' ')}\n`);
      lastHeartbeat = at;
    }
    if (termAt === null && at - drainAt >= drainMs) {
      send(sid, members, 'SIGTERM');
      termAt = at;
    } else if (termAt !== null && killAt === null && at - termAt >= termMs) {
      send(sid, members, 'SIGKILL');
      killAt = at;
      write(`[gate-slot] session ${sid} exceeded the ${drainMs} ms drain and ${termMs} ms TERM grace; sent SIGKILL\n`);
    } else if (killAt !== null && at - killAt >= killMs) {
      write(`[gate-slot] session ${sid} still alive ${killMs} ms after SIGKILL; releasing slot with surviving pids: ${members.map(p => p.pid).join(' ')}\n`);
      return 1;
    }
    await sleep(50);
  }
}

async function readHandoff(path, supervisorPid, supervisorStart) {
  let missingAt = null;
  for (;;) {
    try { return readFileSync(path, 'utf8'); } catch { /* handoff not written yet */ }
    const supervisor = readStat(Number(supervisorPid));
    if (supervisor?.start === supervisorStart) missingAt = null;
    else {
      missingAt ??= performance.now();
      if (performance.now() - missingAt >= SLOT_SUPERVISOR_GONE_MS) return null;
    }
    await delay(10);
  }
}

async function slotHolderMain(lock, acquired, controlPath, admissionPath, supervisorPid, supervisorStart) {
  const holder = readStat(process.ppid);
  const supervisor = readStat(Number(supervisorPid));
  if (!holder || holder.ppid !== Number(supervisorPid) || !supervisor
      || supervisor.start !== supervisorStart || !processHoldsFile(holder.pid, lock)) {
    throw new Error(`slot holder ${process.ppid} cannot prove its descriptor for ${lock}`);
  }
  // flock starts this helper only after acquisition. Publish the exact parent pid/start only after
  // its descriptor and supervisor ancestry are visible to the same /proc checks the gate will use.
  writeHandoff(acquired, `${holder.pid}\t${holder.start}\n`);
  const control = await readHandoff(controlPath, supervisorPid, supervisorStart);
  if (control === null) return 0;
  const match = control.match(/^session\t(\d+)\t(\d+)\n/);
  if (!match) return 0; // The tap died before any recipe was placed in the slot.
  const sid = Number(match[1]);
  const leaderStart = match[2];
  const admission = await readHandoff(admissionPath, supervisorPid, supervisorStart);
  if (admission === null) return 0;
  if (admission !== 'admitted\n') return 0;
  const proc = process.env.GATE_SLOT_PROC_ROOT || '/proc';
  const trace = process.env.GATE_SLOT_PROC_TRACE;
  return drainSlotSession(sid, leaderStart, {
    // The session leader itself wrote the admission marker after proving this holder. That is a
    // stronger observation than a racy /proc snapshot and closes holder-ready -> gate-start.
    leaderObserved: true,
    proc,
    scan: () => {
      if (trace) { try { writeFileSync(trace, `${Date.now()}\n`, { flag: 'a' }); } catch { /* test trace */ } }
      return snapshot({ proc });
    },
  });
}

async function slotLaunchMain(startPath, admissionPath, supervisorPid, supervisorStart, command, args) {
  while (!existsSync(startPath)) {
    const supervisor = readStat(Number(supervisorPid));
    if (!supervisor || supervisor.start !== supervisorStart) process.exit(125);
    await delay(10);
  }
  const lock = process.env.GATE_SLOT_LOCK_PATH;
  const ownershipDeadline = performance.now() + SLOT_OWNER_READY_MS;
  let ownershipFailure = 'ownership was not checked';
  while (lock && !verifySlotOwnerToken(lock, process.env[SLOT_OWNER_ENV], process.pid, null,
    reason => { ownershipFailure = reason; })) {
    const supervisor = readStat(Number(supervisorPid));
    if (!supervisor || supervisor.start !== supervisorStart) process.exit(125);
    if (performance.now() >= ownershipDeadline) {
      writeSync(2, `gate-slot-launch: holder ownership was not verifiable after ${SLOT_OWNER_READY_MS} ms: ${ownershipFailure}\n`);
      process.exit(125);
    }
    await delay(10);
  }
  // The detached holder does not begin session drain until this exact leader has verified the
  // token. It therefore cannot release the descriptor between token export and gate spawn.
  writeHandoff(admissionPath, 'admitted\n');
  const child = spawn(command, args, { stdio: 'inherit', env: process.env });
  child.once('error', () => process.exit(127));
  child.once('close', (code, signal) => {
    if (signal) process.kill(process.pid, signal);
    else process.exit(code ?? 1);
  });
}

async function startSlotHolder(lock, mode, waitSeconds, scratch, interrupt) {
  const acquired = join(scratch, `acquired-${mode}`);
  const control = join(scratch, `session-${mode}`);
  const admission = join(scratch, `admission-${mode}`);
  const supervisor = readStat(process.pid);
  const args = ['--close', '--conflict-exit-code', String(SLOT_CONFLICT_EXIT)];
  if (mode === 'try') args.push('-n');
  else args.push('-w', String(waitSeconds));
  args.push(lock, process.execPath, resolve(process.argv[1]), '--slot-holder', lock, acquired, control, admission,
    String(supervisor.pid), supervisor.start);
  const child = spawn('flock', args, { detached: true, stdio: ['ignore', 'ignore', 'inherit'] });
  let closed = null;
  const done = new Promise(resolveDone => {
    const finish = result => {
      if (closed) return;
      closed = result;
      resolveDone(closed);
    };
    child.once('error', error => finish({ code: 127, signal: null, error }));
    child.once('close', (code, signal) => finish({ code, signal }));
  });
  let holder = null;
  while (!holder && !closed && !interrupt.signal) {
    // The helper's marker proves flock returned from acquisition. Admission additionally waits
    // until /proc exposes the descriptor on the exact flock pid named by the ownership token.
    if (existsSync(acquired) && Number.isInteger(child.pid)) {
      try {
        const [pidText, start] = readFileSync(acquired, 'utf8').trim().split('\t');
        const pid = Number(pidText);
        const stat = readStat(pid);
        if (pid === child.pid && stat?.start === start && stat.ppid === supervisor.pid
            && processHoldsFile(pid, lock)) holder = stat;
      } catch { /* holder has not published verifiable readiness yet */ }
    }
    if (!holder) await delay(10);
  }
  if (interrupt.signal && !closed) {
    try { process.kill(-child.pid, 'SIGKILL'); } catch { /* already gone */ }
    await done;
  }
  return {
    child, done, acquired: Boolean(holder) && !interrupt.signal, closed, holder, admission,
    interrupted: interrupt.signal,
    holdSession(pid, start) { writeHandoff(control, `session\t${pid}\t${start}\n`); },
    cancelGate() {
      if (!existsSync(admission)) writeHandoff(admission, 'cancelled\n');
    },
    async release() {
      await done;
    },
  };
}

async function acquireSlot(project, run, env, interrupt) {
  const lock = env.GATE_SLOT_LOCK_PATH;
  if (!lock) return { busy: false, holdSession() {}, startGate() {}, cancelGate() {}, release: async () => {} };
  if (verifySlotOwnerToken(lock, env[SLOT_OWNER_ENV])) {
    return { busy: false, nested: true, ownerToken: env[SLOT_OWNER_ENV], holdSession() {}, startGate() {}, cancelGate() {}, release: async () => {} };
  }
  mkdirSync(dirname(lock), { recursive: true });
  const scratch = mkdtempSync(join(tmpdir(), 'gate-slot-holder-'));
  const cleanup = () => { try { rmSync(scratch, { recursive: true, force: true }); } catch { /* scratch */ } };
  const first = await startSlotHolder(lock, 'try', 0, scratch, interrupt);
  if (first.interrupted) { cleanup(); return { interrupted: first.interrupted }; }
  if (first.acquired) {
    publishHolder(env.GATE_SLOT_HOLDER_PATH, project, first.child.pid, run);
    const supervisor = readStat(process.pid);
    const startPath = join(scratch, 'gate-start');
    return { busy: false, holdSession: first.holdSession, startPath, admissionPath: first.admission,
      startGate() { writeHandoff(startPath, 'start\n'); },
      cancelGate: first.cancelGate,
      ownerToken: holderToken(lock, { pid: first.holder.pid, start: first.holder.start }, { pid: supervisor.pid, start: supervisor.start }),
      release: async () => { await first.release(); cleanup(); } };
  }
  await first.done;
  const limit = slotWaitLimit(env);
  const started = performance.now();
  const heartbeat = () => {
    const holder = currentHolder(env.GATE_SLOT_HOLDER_PATH);
    const waited = (performance.now() - started) / 1000;
    writeSync(2, `[gate-tap] ${project}: cargo slot still busy after ${waited.toFixed(1)}s; holder: ${holder}\n`);
    if (run) appendEvent(run, { kind: 'wait', project, resource: 'cargo-slot', holder });
  };
  heartbeat();
  const timer = setInterval(heartbeat, slotHeartbeatMs(env));
  const blocked = await startSlotHolder(lock, 'wait', limit, scratch, interrupt);
  clearInterval(timer);
  if (blocked.interrupted) { cleanup(); return { interrupted: blocked.interrupted }; }
  const waited = (performance.now() - started) / 1000;
  const holder = currentHolder(env.GATE_SLOT_HOLDER_PATH);
  writeSlotWait(env.GATE_SLOT_WAIT_RESULT, waited, holder);
  if (!blocked.acquired) {
    await blocked.done;
    cleanup();
    process.stderr.write(`gate ${project}: REFUSED cargo slot stayed busy for ${limit}s; holder: ${holder}\n`);
    return { busy: true, release: async () => {} };
  }
  publishHolder(env.GATE_SLOT_HOLDER_PATH, project, blocked.child.pid, run);
  const supervisor = readStat(process.pid);
  const startPath = join(scratch, 'gate-start');
  return { busy: false, holdSession: blocked.holdSession, startPath, admissionPath: blocked.admission,
    startGate() { writeHandoff(startPath, 'start\n'); },
    cancelGate: blocked.cancelGate,
    ownerToken: holderToken(lock, { pid: blocked.holder.pid, start: blocked.holder.start }, { pid: supervisor.pid, start: supervisor.start }),
    release: async () => { await blocked.release(); cleanup(); } };
}

function parseArgs(argv) {
  const split = argv.indexOf('--');
  if (split < 0 || split === argv.length - 1) return null;
  const own = argv.slice(0, split);
  const opt = name => {
    const i = own.indexOf(name);
    return i >= 0 ? own[i + 1] : undefined;
  };
  return {
    project: opt('--project') || 'gate',
    run: opt('--run') || null,
    dir: opt('--dir') || null,
    command: argv[split + 1],
    args: argv.slice(split + 2),
  };
}

/** Split a byte stream into lines for the recognizer; the bytes themselves are forwarded elsewhere. */
export function lineSplitter(onLine) {
  const partial = Buffer.alloc(64 * 1024);
  let length = 0;
  let dropping = false;
  return {
    write(chunk) {
      let start = 0;
      while (start < chunk.length) {
        const newline = chunk.indexOf(10, start);
        const end = newline < 0 ? chunk.length : newline;
        if (!dropping) {
          const take = Math.min(end - start, partial.length - length);
          chunk.copy(partial, length, start, start + take);
          length += take;
          if (start + take < end) {
            onLine(partial.toString('utf8', 0, length));
            length = 0;
            dropping = true;
          }
        }
        if (newline < 0) break;
        if (!dropping) onLine(partial.toString('utf8', 0, length));
        length = 0;
        dropping = false;
        start = newline + 1;
      }
    },
    end() {
      if (length) onLine(partial.toString('utf8', 0, length));
      length = 0;
      dropping = false;
    },
  };
}

// Piping the child takes its terminal away; keep the colour a person would have seen.
// The recognizer strips ANSI, so this never changes what is recognized.
export function tapChildEnv(env, isTTY) {
  if (!isTTY) return env;
  return { CARGO_TERM_COLOR: 'always', FORCE_COLOR: '1', ...env };
}

/**
 * Labels a failure with the sessions whose edits to its file fall inside the run's window
 * (gate-attribution.mjs). A label is a hint for the reader and decides nothing; with no run,
 * no run-open, or no edit journal, failures pass through unlabelled.
 */
export function attributor({ run, dir, root = process.cwd(), now = () => Date.now() }) {
  if (!run) return event => event;
  let opened = null;
  try {
    opened = JSON.parse(readFileSync(run, 'utf8').split('\n')[0]);
  } catch { /* no run-open to anchor the window */ }
  const openedAtMs = Date.parse(opened?.at ?? '');
  if (!opened || opened.kind !== 'run-open' || !Number.isFinite(openedAtMs)) return event => event;
  let edits = null;
  return event => {
    const file = event.failure?.locus?.file;
    if (event.kind !== 'failure' || typeof file !== 'string') return event;
    // A compiler names a file relative to the crate or workspace it built; try the gate's
    // own directory, then the repository root, and label nothing that resolves nowhere.
    const absolute = isAbsolute(file) ? file
      : [dir, root].filter(Boolean).map(base => resolve(root, base, file)).find(path => existsSync(path));
    if (!absolute) return event;
    edits ??= readEdits(join(dirname(liveRoot(root)), 'lsp', 'edits.jsonl'));
    const attribution = attribute({ failure: { ...event.failure, locus: { ...event.failure.locus, file: absolute } } }, edits, {
      session: opened.session, windowStartMs: openedAtMs - GATE_ATTRIBUTION_WINDOW_MS, atMs: now(),
    });
    return { ...event, failure: { ...event.failure, attribution } };
  };
}

/** Sample RSS for the child's tree; CPU and pressure describe its enclosing cgroup/host.
 * Missing kernel counters remain null, never a fabricated zero. Cgroup paths are retained
 * so the final counters can still be read after the child has exited.
 */
export function createCostSampler(pid, { readFile = readFileSync, listDir = readdirSync,
  proc = '/proc', cgroup = '/sys/fs/cgroup' } = {}) {
  const cost = { cpu_s: null, throttled_s: null, rss_peak_bytes: null,
    memory_high_events: null, io_full_pct_max: null };
  const read = path => { try { return readFile(path, 'utf8'); } catch { return ''; } };
  const counter = (text, key) => {
    const found = text.match(new RegExp(`^${key}\\s+(\\d+)$`, 'm'));
    return found ? Number(found[1]) : null;
  };
  const group = read(join(proc, String(pid), 'cgroup')).match(/^0::(.*)$/m)?.[1];
  // A cgroup namespace reports '/' for its root. Never walk outside its mounted tree.
  const groupPath = group !== undefined && !group.split('/').includes('..') ? join(cgroup, group) : null;
  const counters = () => {
    const cpu = groupPath ? read(join(groupPath, 'cpu.stat')) : '';
    const memory = groupPath ? read(join(groupPath, 'memory.events')) : '';
    return { cpu_s: counter(cpu, 'usage_usec'), throttled_s: counter(cpu, 'throttled_usec'),
      memory_high_events: counter(memory, 'high') };
  };
  const initial = counters();
  const sample = () => {
    let entries = [];
    try { entries = listDir(proc); } catch { /* proc unavailable */ }
    const processes = new Map();
    for (const entry of entries) {
      if (!/^\d+$/.test(entry)) continue;
      const status = read(join(proc, entry, 'status'));
      const parent = counter(status, 'PPid:');
      const rss = status.match(/^VmRSS:\s+(\d+)\s+kB$/m);
      if (parent !== null) processes.set(Number(entry), { parent, rss: rss ? Number(rss[1]) * 1024 : null });
    }
    const tree = new Set([pid]);
    let changed = true;
    while (changed) {
      changed = false;
      for (const [id, p] of processes) {
        if (!tree.has(id) && tree.has(p.parent)) { tree.add(id); changed = true; }
      }
    }
    const readings = [...tree].map(id => processes.get(id)?.rss).filter(value => value != null);
    if (readings.length) cost.rss_peak_bytes = Math.max(cost.rss_peak_bytes ?? 0, readings.reduce((a, b) => a + b, 0));
    const latest = counters();
    for (const key of Object.keys(latest)) {
      if (initial[key] !== null && latest[key] !== null && latest[key] >= initial[key]) {
        cost[key] = (latest[key] - initial[key]) / (key.endsWith('_s') ? 1e6 : 1);
      }
    }
    const io = read(join(proc, 'pressure/io')).match(/^full\s+avg10=(\d+(?:\.\d+)?)/m);
    if (io) cost.io_full_pct_max = Math.max(cost.io_full_pct_max ?? 0, Number(io[1]));
    return { ...cost };
  };
  return { sample };
}

async function main() {
  const parsed = parseArgs(process.argv.slice(2));
  if (!parsed) {
    process.stderr.write('usage: gate-tap.mjs --project <name> [--run <path>] [--dir <project dir>] -- <command> [args…]\n');
    process.exit(2);
  }
  const { project, run, dir, command, args } = parsed;
  const label = attributor({ run, dir });
  // The first red inside a gate carries the decision with it, mid-gate: the rest of this gate and
  // every gate after it (GATE_REMAINING, set by the hook and gate-runner) with their recorded cost.
  // outlook() is once per run, so a run whose first red came earlier adds nothing here.
  let outlooked = false;
  let firstOutlook;
  // Set once a stop-now request is seen while the leader runs: { by, reason, ended } where `ended`
  // resolves to endTree's answer. `held` keeps the lines printed meanwhile: when the stop turns out
  // not to apply (the leader ended on its own first), they are appended after all.
  let stopping = null;
  // A signal delivered to the tap is an outside stop of the gate boundary. This is distinct from a
  // signal delivered only to the detached recipe tree (OOM / RAM-guard shed), which never reaches
  // this process and therefore keeps the recipe's resulting verdict.
  let outsideSignal = null;
  let held = [];
  // The first red's outlook is written to the run log before its sibling note is sent, so the
  // detached sender reads it there; the tap waits for that write only (bounded, below).
  const pending = new Set();
  const track = promise => { pending.add(promise); promise.finally(() => pending.delete(promise)); };
  const recognizer = createRecognizer({
    project,
    emit: event => {
      // A gate being stopped prints its own death (cargo's `signal: 15`, a test runner's abort):
      // those lines are the stop, not reds, and none is appended or sent while it is ended.
      if (!run) return;
      if (stopping || outsideSignal) { held.push(event); return; }
      deliver(event);
    },
  });
  function deliver(event) {
    const written = appendEvent(run, label(event));
    if (!outlooked && event.kind === 'failure' && process.env.GATE_REMAINING !== undefined) {
      outlooked = true;
      const remaining = [project, ...process.env.GATE_REMAINING.split(/\s+/).filter(Boolean)];
      firstOutlook = outlook(run, remaining, { root: process.cwd() }).catch(() => null);
      track(firstOutlook);
    }
    // E4 finding 3: a detached, deadline-bound worker sends the note; the tap never waits on
    // berth, and an unsendable note is a sibling-delivery line in the run log.
    if (written && event.kind === 'failure') emitRedDetached(run, written);
  }
  // An interrupt to the tap is the gate's: its handlers are installed BEFORE the gate starts. The
  // gate runs in its own session, so nothing else carries a Ctrl-C to it; a SIGINT arriving between
  // the spawn and a later registration took the tap's default action, killed the tap and orphaned
  // the whole gate (pid 1 adopted it; 30 of 30 runs when signalled at the gate's first line). A
  // signal JavaScript sees before the gate exists (none can, spawn below is in the same tick) is
  // held and forwarded once it does.
  let child = null;
  let signalGate = null;
  const slotInterrupt = { signal: null };
  const earlySignals = [];
  const forward = signal => () => {
    outsideSignal ??= signal;
    if (signalGate) signalGate(signal);
    else { slotInterrupt.signal ??= signal; earlySignals.push(signal); }
  };
  for (const signal of ['SIGINT', 'SIGTERM', 'SIGHUP']) process.on(signal, forward(signal));

  // A detached holder process owns the slot outside the gate's group. Once told the gate's session,
  // it ignores tap-pipe EOF and holds the lock until that session is empty. Before that message,
  // EOF releases it because no recipe has run. This is why SIGKILL of the tap cannot free a live
  // recipe's lock, while an interrupt during admission releases the waiter and records no verdict.
  const slot = await acquireSlot(project, run, process.env, slotInterrupt);
  if (slot.interrupted) {
    try {
      writeSync(3, `${JSON.stringify({ signals: recognizer.signals(), ...recognizer.summary(),
        stopped: { by: null, signal: slot.interrupted, outside: true } })}\n`);
    } catch { /* no fd 3: run by hand */ }
    process.exit(signalExit(slot.interrupted));
  }
  if (slot.busy) {
    try { writeSync(3, `${JSON.stringify({ signals: recognizer.signals(), ...recognizer.summary(), slotBusy: true })}\n`); }
    catch { /* no fd 3: run by hand */ }
    process.exit(SLOT_BUSY_EXIT);
  }

  // Its own session and group (detached), so the gate's whole tree can be signalled as one and no
  // signal meant for it reaches the tap, gate-runner or the hook. The leader is read now, while
  // this process holds it unreaped: that pid and start time are the first link of gate-stop's custody.
  const gateEnv = tapChildEnv(process.env, Boolean(process.stdout.isTTY));
  if (slot.ownerToken) gateEnv[SLOT_OWNER_ENV] = slot.ownerToken;
  const launched = slot.startPath
    ? { command: process.execPath, args: [resolve(process.argv[1]), '--slot-launch', slot.startPath,
      slot.admissionPath, String(process.pid), readStat(process.pid).start, '--', command, ...args] }
    : { command, args };
  child = spawn(launched.command, launched.args, {
    stdio: ['inherit', 'pipe', 'pipe'],
    env: gateEnv,
    detached: true,
  });
  const leader = Number.isInteger(child.pid) ? readStat(child.pid) : null;
  if (leader) {
    slot.holdSession(child.pid, leader.start);
    slot.startGate();
  }
  // What the gate's side of its output points at, so a process that keeps the output open after the
  // leader ended can be named (gate-stop.mjs processesHolding); nothing is added to its environment.
  const targets = Number.isInteger(child.pid) ? outputTargets(child.pid) : [];
  const leaderHeld = () => child.exitCode === null && child.signalCode === null;
  signalGate = signal => {
    if (!Number.isInteger(child.pid)) return;
    // While the leader is unreaped its pid, and so its group, cannot be anyone else's.
    if (leaderHeld()) {
      try { process.kill(-child.pid, signal); } catch { try { child.kill(signal); } catch { /* gone */ } }
      return;
    }
    if (!leader) return;
    const tree = gateTree(child.pid, new Set([`${leader.pid}:${leader.start}`]), snapshot());
    if (tree.custody) signalTree(child.pid, tree.members, signal);
  };
  const sampler = createCostSampler(child.pid);
  sampler.sample();
  const sampling = setInterval(() => sampler.sample(), 5000);
  // One recognizer reads both streams: rustc diagnostics go to stderr, test output to stdout.
  const out = lineSplitter(line => recognizer.push(line));
  const err = lineSplitter(line => recognizer.push(line));
  child.stdout.on('data', chunk => { process.stdout.write(chunk); out.write(chunk); });
  child.stderr.on('data', chunk => { process.stderr.write(chunk); err.write(chunk); });

  for (const signal of earlySignals.splice(0)) signalGate(signal);

  // The stop-now probe: the run log is re-read only when it grew, so an idle check is one stat.
  // A stop-now request ends the gate's tree while its leader runs (gate-stop.mjs endTree). Whether
  // the gate WAS stopped is decided only at close, from the leader's own exit (stoppedBy): death
  // by a signal the stop delivered. A leader that exited with a code, before or at the instant of
  // the signal, ran to its own verdict, and that red or green is recorded as without a stop; the
  // stop then only refuses the push. Either way, once a stop is asked the tap waits for the gate's
  // output to close for a bounded time only, then stops waiting and names what still holds it.
  let probedSize = -1;
  let closed = false;
  let escapeTimer = null;
  const nothing = { custody: false, toLeader: [], signal: null, survivors: [] };
  const stopWaitingOnOutput = (survivors = []) => {
    if (closed || escapeTimer) return;
    escapeTimer = setTimeout(() => {
      if (closed) return;
      const holders = processesHolding(targets);
      stray = holders;
      process.stderr.write(`[gate-tap] ${project}: a process outside the gate's tree may still be running`
        + (holders.length ? ` and holds its output: ${holders.map(h => `pid ${h.pid}${h.cmd ? ` (${h.cmd})` : ''}`).join(', ')}`
          : ' (none found holding its output)')
        + `${survivors.length ? `; still alive in its tree after KILL: ${survivors.join(' ')}` : ''}`
        + '; the tap stops waiting for it\n');
      truncated = true;
      child.stdout.destroy();
      child.stderr.destroy();
    }, ESCAPED_WAIT_MS);
  };
  let stray = null;
  // Set when the tap stopped reading the gate's output before its writers closed it.
  let truncated = false;
  const probeStop = () => {
    if (!run || stopping) return;
    let size;
    try { size = statSync(run).size; } catch { return; }
    if (size === probedSize) return;
    probedSize = size;
    const request = stopNowRequest(run);
    if (!request) return;
    stopping = { by: request.by ?? null, reason: request.reason ?? null, id: request.id ?? null };
    if (!leaderHeld()) {
      // The recipe already ended on its own; only its output can still be open.
      stopping.ended = Promise.resolve(nothing);
      stopWaitingOnOutput();
      return;
    }
    stopping.ended = endTree(leader, { graceMs: stopGraceMs(process.env) })
      .catch(() => nothing)
      .then(result => { stopWaitingOnOutput(result.survivors); return result; });
  };
  const stopProbe = run ? setInterval(probeStop, 250) : null;
  probeStop();

  // When the leader's exit was observed, on the clock endTree stamps its deliveries with.
  let exitAt = null;
  child.on('exit', () => { exitAt = performance.now(); });
  child.on('error', error => {
    process.stderr.write(`gate-tap: ${command}: ${error.message}\n`);
    process.exitCode = 127;
  });
  child.on('close', async (code, signal) => {
    closed = true;
    clearInterval(sampling);
    if (stopProbe) clearInterval(stopProbe);
    const cost = sampler.sample();
    let stopped;
    if (outsideSignal) {
      stopped = { by: null, signal: outsideSignal, outside: true };
      stopping = null;
      held = [];
      process.stderr.write(`[gate-tap] ${project}: stopped from outside (${outsideSignal}); no verdict is recorded for it\n`);
    } else if (stopping) {
      const asked = stopping;
      const ended = await asked.ended;
      if (escapeTimer) clearTimeout(escapeTimer);
      const by = stoppedBy({ code, signal, exitAt }, ended.toLeader);
      if (by) {
        stopped = { by: asked.by, ...(asked.reason ? { reason: asked.reason } : {}), ...(asked.id ? { request: asked.id } : {}), signal: ended.signal,
          ...(ended.survivors.length ? { survivors: ended.survivors } : {}),
          ...(stray ? { escaped: stray } : {}) };
        process.stderr.write(`[gate-tap] ${project}: stopped on request${asked.by ? ` by ${asked.by}` : ''}: `
          + `its leader died of ${by} and its process group was ended (${ended.signal}); no verdict is recorded for it\n`);
      } else {
        // It ran to its own verdict: the lines held while the stop was pending are its own.
        stopping = null;
        const keep = held;
        held = [];
        for (const event of keep) deliver(event);
        if (asked) process.stderr.write(`[gate-tap] ${project}: a stop was requested, but the recipe ended on its own `
          + `(${signal ?? `exit ${code}`}); its verdict stands, and the push is refused\n`);
      }
    }
    out.end();
    err.end();
    // The child's status decides whether an unrecognized red step stands in as a failure (E6 #2).
    // A stopped gate has no such step: its end is the stop (the emit above holds it, then drops it).
    recognizer.end(signal ? 1 : code);
    // For a slot gate the detached holder owns the bounded session drain. For every other gate this
    // close path performs no session scan.
    slot.cancelGate();
    await slot.release();
    try {
      writeSync(3, `${JSON.stringify({ signals: recognizer.signals(), ...recognizer.summary(), ...cost,
        ...(stopped ? { stopped } : {}), ...(truncated ? { outputTruncated: true } : {}) })}\n`);
    } catch { /* no fd 3: run by hand */ }
    // Exit only after forwarded bytes drain, then carry the child's status or signal.
    const finish = () => {
      // Only the outlook write is awaited, capped; berth is never awaited here.
      Promise.race([Promise.allSettled([...pending]), delay(1500)]).then(exit, exit);
    };
    const exit = () => {
      if (run) noteUnsentReds(run);
      if (outsideSignal) {
        process.exit(signalExit(outsideSignal));
        return;
      }
      if (signal) {
        for (const s of ['SIGINT', 'SIGTERM', 'SIGHUP']) process.removeAllListeners(s);
        process.kill(process.pid, signal);
        return;
      }
      process.exit(code ?? 1);
    };
    process.stdout.write('', () => process.stderr.write('', finish));
  });
}

const isMain = import.meta.url === `file://${process.argv[1]}` ||
  import.meta.url === `file://${resolve(process.argv[1])}`;
if (isMain && process.argv[2] === '--slot-holder') {
  slotHolderMain(...process.argv.slice(3, 9)).then(status => { process.exitCode = status; }).catch(error => {
    process.stderr.write(`gate-slot-holder: ${error?.stack || error}\n`);
    process.exitCode = 2;
  });
}
else if (isMain && process.argv[2] === '--slot-launch') {
  const split = process.argv.indexOf('--', 3);
  slotLaunchMain(process.argv[3], process.argv[4], process.argv[5], process.argv[6], process.argv[split + 1], process.argv.slice(split + 2))
    .catch(() => process.exit(127));
}
else if (isMain && process.argv[2] === '--verify-slot-owner') {
  (async () => {
    for (let attempt = 0; attempt < 20; attempt += 1) {
      if (verifySlotOwnerToken(process.argv[3], process.env[SLOT_OWNER_ENV], process.pid, process.argv[4])) process.exit(0);
      await delay(10);
    }
    process.exit(1);
  })();
} else if (isMain) main().catch(error => {
  process.stderr.write(`gate-tap: ${error?.stack || error}\n`);
  process.exit(2);
});
