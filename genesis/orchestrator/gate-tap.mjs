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

import { spawn } from 'child_process';
import { existsSync, readFileSync, readdirSync, statSync, writeSync } from 'fs';
import { dirname, isAbsolute, join, resolve } from 'path';
import { createRecognizer } from './gate-failure-lines.mjs';
import { GATE_ATTRIBUTION_WINDOW_MS, attribute, readEdits } from './gate-attribution.mjs';
import { setTimeout as delay } from 'timers/promises';
import { appendEvent, emitRedDetached, liveRoot, noteUnsentReds, outlook, stopNowRequest } from './gate-live.mjs';
import { endTree, gateTree, outputTargets, processesHolding, readStat, signalTree, snapshot, stopGraceMs, stoppedBy } from './gate-stop.mjs';

// After the gate's tree is ended, how long the tap still waits for its output pipes to close.
const ESCAPED_WAIT_MS = 2000;

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

function main() {
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
      if (stopping) { held.push(event); return; }
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
  // Its own session and group (detached), so the gate's whole tree can be signalled as one and no
  // signal meant for it reaches the tap, gate-runner or the hook. The leader is read now, while
  // this process holds it unreaped: that pid and start time are the first link of gate-stop's custody.
  const child = spawn(command, args, {
    stdio: ['inherit', 'pipe', 'pipe'],
    env: tapChildEnv(process.env, Boolean(process.stdout.isTTY)),
    detached: true,
  });
  const leader = Number.isInteger(child.pid) ? readStat(child.pid) : null;
  // What the gate's side of its output points at, so a process that keeps the output open after the
  // leader ended can be named (gate-stop.mjs processesHolding); nothing is added to its environment.
  const targets = Number.isInteger(child.pid) ? outputTargets(child.pid) : [];
  const leaderHeld = () => child.exitCode === null && child.signalCode === null;
  const signalGate = signal => {
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

  const forward = signal => () => signalGate(signal);
  for (const signal of ['SIGINT', 'SIGTERM', 'SIGHUP']) process.on(signal, forward(signal));

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
  const nothing = { custody: false, delivered: [], signal: null, survivors: [] };
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
      child.stdout.destroy();
      child.stderr.destroy();
    }, ESCAPED_WAIT_MS);
  };
  let stray = null;
  const probeStop = () => {
    if (!run || stopping) return;
    let size;
    try { size = statSync(run).size; } catch { return; }
    if (size === probedSize) return;
    probedSize = size;
    const request = stopNowRequest(run);
    if (!request) return;
    stopping = { by: request.by ?? null, reason: request.reason ?? null };
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
    if (stopping) {
      const asked = stopping;
      const ended = await asked.ended;
      if (escapeTimer) clearTimeout(escapeTimer);
      const by = stoppedBy({ code, signal }, ended.delivered);
      if (by) {
        stopped = { by: asked.by, ...(asked.reason ? { reason: asked.reason } : {}), signal: ended.signal,
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
    try {
      writeSync(3, `${JSON.stringify({ signals: recognizer.signals(), ...recognizer.summary(), ...cost, ...(stopped ? { stopped } : {}) })}\n`);
    } catch { /* no fd 3: run by hand */ }
    // Exit only after forwarded bytes drain, then carry the child's status or signal.
    const finish = () => {
      // Only the outlook write is awaited, capped; berth is never awaited here.
      Promise.race([Promise.allSettled([...pending]), delay(1500)]).then(exit, exit);
    };
    const exit = () => {
      if (run) noteUnsentReds(run);
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
if (isMain) main();
