#!/usr/bin/env node
// Runs one gate command and reads its output as it prints, without changing it.
//
//   node gate-tap.mjs --project <name> [--run <gate-live run path>] -- <command> [args…]
//
// Bytes reach this process's stdout/stderr unchanged and the command's exit status (or
// signal) is this process's. Failure-level lines (gate-failure-lines.mjs) are appended to
// the live run as they appear, so the session that started a push can begin on the first
// red while later gates still run. When fd 3 is open, the classifyFailure signals and the
// first-red time are written there as one JSON object for the caller's cycle row.
// Nothing here decides a verdict. The tap is the emitter adapter a CI stage or
// rakia-executor can also wrap: genesis/docs/superpowers/specs/2026-10-26-gate-live-failure-channel-design.md

import { spawn } from 'child_process';
import { writeSync } from 'fs';
import { resolve } from 'path';
import { createRecognizer } from './gate-failure-lines.mjs';
import { appendEvent } from './gate-live.mjs';

function parseArgs(argv) {
  const split = argv.indexOf('--');
  if (split < 0 || split === argv.length - 1) return null;
  const own = argv.slice(0, split);
  const opt = name => {
    const i = own.indexOf(name);
    return i >= 0 ? own[i + 1] : undefined;
  };
  return { project: opt('--project') || 'gate', run: opt('--run') || null, command: argv[split + 1], args: argv.slice(split + 2) };
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

function main() {
  const parsed = parseArgs(process.argv.slice(2));
  if (!parsed) {
    process.stderr.write('usage: gate-tap.mjs --project <name> [--run <path>] -- <command> [args…]\n');
    process.exit(2);
  }
  const { project, run, command, args } = parsed;
  const recognizer = createRecognizer({
    project,
    emit: event => { if (run) appendEvent(run, event); },
  });
  const child = spawn(command, args, {
    stdio: ['inherit', 'pipe', 'pipe'],
    env: tapChildEnv(process.env, Boolean(process.stdout.isTTY)),
  });
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
    out.end();
    err.end();
    recognizer.end();
    try {
      writeSync(3, `${JSON.stringify({ signals: recognizer.signals(), ...recognizer.summary() })}\n`);
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
