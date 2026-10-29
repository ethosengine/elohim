#!/usr/bin/env node
// Runs one gate command and reads its output as it prints, without changing it.
//
//   node gate-tap.mjs --project <name> [--run <gate-live run path>] [--dir <project dir>] -- <command> [args…]
//
// Bytes reach this process's stdout/stderr unchanged and the command's exit status (or
// signal) is this process's. Failure-level lines (gate-failure-lines.mjs) are appended to
// the live run as they appear, so the session that started a push can begin on the first
// red while later gates still run. When fd 3 is open, the classifyFailure signals and the
// first-red time are written there as one JSON object for the caller's cycle row.
// Nothing here decides a verdict. The tap is the emitter adapter a CI stage or
// rakia-executor can also wrap: genesis/docs/superpowers/specs/2026-10-26-gate-live-failure-channel-design.md

import { spawn } from 'child_process';
import { existsSync, readFileSync, readdirSync, writeSync } from 'fs';
import { dirname, isAbsolute, join, resolve } from 'path';
import { createRecognizer } from './gate-failure-lines.mjs';
import { GATE_ATTRIBUTION_WINDOW_MS, attribute, readEdits } from './gate-attribution.mjs';
import { appendEvent, liveRoot } from './gate-live.mjs';

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
  const recognizer = createRecognizer({
    project,
    emit: event => { if (run) appendEvent(run, label(event)); },
  });
  const child = spawn(command, args, {
    stdio: ['inherit', 'pipe', 'pipe'],
    env: tapChildEnv(process.env, Boolean(process.stdout.isTTY)),
  });
  const sampler = createCostSampler(child.pid);
  sampler.sample();
  const sampling = setInterval(() => sampler.sample(), 5000);
  // One recognizer reads both streams: rustc diagnostics go to stderr, test output to stdout.
  const out = lineSplitter(line => recognizer.push(line));
  const err = lineSplitter(line => recognizer.push(line));
  child.stdout.on('data', chunk => { process.stdout.write(chunk); out.write(chunk); });
  child.stderr.on('data', chunk => { process.stderr.write(chunk); err.write(chunk); });

  const forward = signal => () => { try { child.kill(signal); } catch { /* already gone */ } };
  for (const signal of ['SIGINT', 'SIGTERM', 'SIGHUP']) process.on(signal, forward(signal));

  child.on('error', error => {
    process.stderr.write(`gate-tap: ${command}: ${error.message}\n`);
    process.exitCode = 127;
  });
  child.on('close', (code, signal) => {
    clearInterval(sampling);
    const cost = sampler.sample();
    out.end();
    err.end();
    recognizer.end();
    try {
      writeSync(3, `${JSON.stringify({ signals: recognizer.signals(), ...recognizer.summary(), ...cost })}\n`);
    } catch { /* no fd 3: run by hand */ }
    // Exit only after forwarded bytes drain, then carry the child's status or signal.
    const finish = () => {
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
