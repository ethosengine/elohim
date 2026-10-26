// Device-local gate events: never imported or projected; pruned at open, never on a timer.
// No decision is ever read from this log; the verdict is the exit status / brit attestation.
// See genesis/docs/superpowers/specs/2026-10-26-gate-live-failure-channel-design.md.

import { execFileSync, spawnSync } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import {
  appendFileSync,
  closeSync,
  mkdirSync,
  openSync,
  readdirSync,
  readFileSync,
  readSync,
  statSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs';
import { basename, dirname, isAbsolute, join, resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';

const STALE_MS = 6 * 60 * 60 * 1000;
const OUTCOMES = new Set(['passed', 'gates-passed', 'failed', 'interrupted']);
const nextSeq = new Map();

function sessionFilename(session) {
  return String(session).replace(/[^A-Za-z0-9._-]/g, '_');
}

function runFiles(directory) {
  return readdirSync(directory, { withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith('.jsonl'))
    .map((entry) => entry.name)
    .sort()
    .reverse();
}

function parseEvents(text) {
  const events = [];
  for (const line of text.split(/\r?\n/)) {
    if (!line.trim()) continue;
    try {
      const event = JSON.parse(line);
      if (
        event !== null &&
        typeof event === 'object' &&
        !Array.isArray(event) &&
        typeof event.kind === 'string'
      ) {
        events.push(event);
      }
    } catch {
      // A damaged line must not hide the remaining evidence.
    }
  }
  return events;
}

function isFile(path) {
  try {
    return statSync(path).isFile();
  } catch {
    return false;
  }
}

function findAt(root, directory, { session, run, latest }) {
  try {
    let path;
    if (run) {
      if (isAbsolute(run) || run.includes('/') || run.includes('\\')) {
        path = resolve(root, run);
      } else if (run.endsWith('.jsonl') && isFile(resolve(root, run))) {
        path = resolve(root, run);
      } else {
        path = join(directory, 'runs', run.endsWith('.jsonl') ? run : `${run}.jsonl`);
      }
    } else if (session != null) {
      path = readFileSync(
        join(directory, 'by-session', sessionFilename(session)),
        'utf8',
      ).trim();
      if (!isAbsolute(path)) return null;
    } else if (latest) {
      const newest = runFiles(join(directory, 'runs'))[0];
      if (!newest) return null;
      path = join(directory, 'runs', newest);
    } else {
      return null;
    }
    return isFile(path) ? path : null;
  } catch {
    return null;
  }
}

export function liveRoot(root = process.cwd(), { git = execFileSync } = {}) {
  const checkout = resolve(root);
  let main = checkout;
  try {
    const common = String(
      git('git', ['rev-parse', '--path-format=absolute', '--git-common-dir'], {
        cwd: checkout,
        encoding: 'utf8',
        stdio: 'pipe',
        timeout: 2000,
      }),
    ).trim();
    if (common) main = dirname(resolve(checkout, common));
  } catch {
    // Non-git directories can still carry a local run log.
  }
  return join(main, '.eprfs', 'status', 'gate-live');
}

export function resolveSession({ env = process.env, spawn = spawnSync } = {}) {
  try {
    const result = spawn(env?.EPR_BIN || 'epr', ['session', 'id', '--json'], {
      env,
      encoding: 'utf8',
      stdio: 'pipe',
      timeout: 2000,
    });
    if (!result.error && result.status === 0) {
      const resultJson = JSON.parse(String(result.stdout));
      if (
        resultJson.source !== 'none' &&
        typeof resultJson.session === 'string' &&
        resultJson.session.length > 0
      ) {
        return resultJson.session;
      }
    }
  } catch {
    // Session discovery must never prevent a gate from starting.
  }
  return typeof env?.ELOHIM_SESSION_ID === 'string' && env.ELOHIM_SESSION_ID.length > 0
    ? env.ELOHIM_SESSION_ID
    : null;
}

export function openRun({
  root = process.cwd(),
  sha = null,
  target = null,
  session = null,
  now = () => new Date(),
  keep = 20,
} = {}) {
  if (!Number.isInteger(keep) || keep < 0) {
    throw new RangeError('keep must be a non-negative integer');
  }

  const directory = liveRoot(root);
  const runs = join(directory, 'runs');
  mkdirSync(runs, { recursive: true });

  const at = new Date(now()).toISOString();
  const timestamp = at.replace(/[-:]/g, '').replace(/\.\d{3}Z$/, 'Z');
  const shortSha = sha
    ? String(sha).slice(0, 12).replace(/[^A-Za-z0-9._-]/g, '_')
    : 'nosha';
  const runId = `${timestamp}-${shortSha}-${randomBytes(3).toString('hex')}`;
  const path = join(runs, `${runId}.jsonl`);
  const event = {
    v: 1,
    run: runId,
    seq: 0,
    at,
    kind: 'run-open',
    candidate: { sha, target },
    session,
  };

  appendFileSync(path, `${JSON.stringify(event)}\n`, { encoding: 'utf8', flag: 'ax' });

  if (session !== null) {
    const pointers = join(directory, 'by-session');
    mkdirSync(pointers, { recursive: true });
    writeFileSync(join(pointers, sessionFilename(session)), path, 'utf8');
  }

  for (const name of runFiles(runs).slice(keep)) {
    try {
      const oldPath = join(runs, name);
      const events = parseEvents(readFileSync(oldPath, 'utf8'));
      if (
        events.at(-1)?.kind !== 'run-close'
        && new Date(at).getTime() - statSync(oldPath).mtimeMs <= STALE_MS
      ) continue;
      unlinkSync(oldPath);
      nextSeq.delete(oldPath);
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
    }
  }

  return { runId, path };
}

export function appendEvent(path, event, { now = () => new Date() } = {}) {
  try {
    path = resolve(path);
    // Several processes append to one run (gate-runner and each gate's tap), so a cached seq
    // is only good while the file is the size this process last left it; otherwise count the
    // lines in the bytes appended since, never the whole file again.
    const cached = nextSeq.get(path);
    let size = 0;
    try {
      size = statSync(path).size;
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
    }
    let seq;
    let torn = false;
    if (cached && cached.size === size) {
      seq = cached.seq;
    } else {
      const from = cached && cached.size < size ? cached.size : 0;
      const base = cached && cached.size < size ? cached.seq : 0;
      seq = base + countLines(path, from, size);
      torn = size > 0 && lastByte(path, size) !== 0x0a;
    }

    const written = {
      ...event,
      v: 1,
      run: basename(path, '.jsonl'),
      seq,
      at: new Date(now()).toISOString(),
    };
    // A torn last line (a writer killed mid-line) is closed off, so this event is its own line.
    const line = `${torn ? '\n' : ''}${JSON.stringify(written)}\n`;
    appendFileSync(path, line, 'utf8');
    nextSeq.set(path, { seq: seq + 1, size: size + Buffer.byteLength(line) });
    return written;
  } catch {
    return null;
  }
}

function lastByte(path, size) {
  const fd = openSync(path, 'r');
  try {
    const byte = Buffer.alloc(1);
    readSync(fd, byte, 0, 1, size - 1);
    return byte[0];
  } finally {
    closeSync(fd);
  }
}

// Lines in bytes [from, to) of a file; a trailing line with no newline still counts as one.
function countLines(path, from, to) {
  if (to <= from) return 0;
  const fd = openSync(path, 'r');
  try {
    const buffer = Buffer.alloc(Math.min(to - from, 1 << 20));
    let lines = 0;
    let last = 0x0a;
    for (let at = from; at < to;) {
      const read = readSync(fd, buffer, 0, Math.min(buffer.length, to - at), at);
      if (read <= 0) break;
      for (let i = 0; i < read; i += 1) if (buffer[i] === 0x0a) lines += 1;
      last = buffer[read - 1];
      at += read;
    }
    return last === 0x0a ? lines : lines + 1;
  } finally {
    closeSync(fd);
  }
}

export function closeRun(path, outcome, {
  now = () => new Date(),
  deferred,
  deferredFromLog = false,
} = {}) {
  try {
    if (!OUTCOMES.has(outcome)) return null;
    const events = parseEvents(readFileSync(path, 'utf8'));
    if (events.at(-1)?.kind === 'run-close') return undefined;
    if (deferredFromLog) {
      deferred = events.filter((event) =>
        event.kind === 'gate-verdict' && event.verdict?.deferred === true).length;
    }
    if (deferred !== undefined && (!Number.isSafeInteger(deferred) || deferred < 0)) return null;
    return appendEvent(path, {
      kind: 'run-close',
      outcome,
      ...(deferred === undefined ? {} : { deferred }),
    }, { now });
  } catch {
    return null;
  }
}

export function readRun(path, { staleMs = STALE_MS, now = () => new Date() } = {}) {
  const events = parseEvents(readFileSync(path, 'utf8'));
  const closed = [...events]
    .reverse()
    .find((event) => event.kind === 'run-close' && OUTCOMES.has(event.outcome));

  if (closed) return { events, outcome: closed.outcome };

  const age = new Date(now()).getTime() - statSync(path).mtimeMs;
  return { events, outcome: age > staleMs ? 'interrupted' : 'running' };
}

export function findRun({ root = process.cwd(), session, run, latest = false } = {}) {
  return findAt(resolve(root), liveRoot(root), { session, run, latest });
}

function excerptLines(excerpt) {
  return (Array.isArray(excerpt) ? excerpt.join('\n') : String(excerpt ?? '')).split(/\r?\n/);
}

export function formatEvent(raw) {
  if (!raw || typeof raw !== 'object') return null;
  // v1 nests a failure's fields under `failure` and a gate's under `verdict`
  // (gate-failure-lines.mjs, gate-runner.mjs); read both that and the flat form.
  const nested = raw.verdict && typeof raw.verdict === 'object' ? raw.verdict : null;
  const event = {
    ...raw,
    ...(raw.failure && typeof raw.failure === 'object' ? raw.failure : {}),
    ...(nested ? {
      verdict: nested.reused ? 'reused' : nested.deferred ? 'deferred' : nested.exit === 0 ? 'passed' : 'failed',
      seconds: nested.seconds,
      failure_class: nested.failure_class ?? undefined,
    } : {}),
  };

  const locus = event.locus && typeof event.locus === 'object' ? event.locus : event;
  const test = locus.test ?? event.test;
  const lines = excerptLines(event.excerpt);

  switch (event.kind) {
    case 'failure': {
      const file = locus.file
        ? `${locus.file}${locus.line != null ? `:${locus.line}` : ''}`
        : '';
      const failureClass = event.class ?? event.failure_class ?? event.failureClass ?? 'unknown';
      const prefix = ['[gate-live] RED', event.project, failureClass, file, test]
        .filter((part) => part !== undefined && part !== null && part !== '')
        .join(' ');
      return `${prefix} — ${(lines[0] ?? '').slice(0, 160)}`;
    }
    case 'detail': {
      const detail = lines.find((line) => line.includes('panicked at'))
        ?? lines.find((line) => line.trim())
        ?? '';
      return ['[gate-live]   ↳', event.project, test, detail.trim()]
        .filter((part) => part !== undefined && part !== null && part !== '')
        .join(' ');
    }
    case 'step-failed':
      return `[gate-live] step ${event.project}/${event.step} exit ${event.exit}`;
    case 'gate-verdict': {
      const verdict = String(event.verdict ?? event.outcome ?? '').toUpperCase();
      const failureClass = event.failure_class ?? event.failureClass ?? 'none';
      return `[gate-live] ${event.project} ${verdict} ${event.seconds}s failure_class=${failureClass}`;
    }
    case 'run-close':
      return `[gate-live] run ${event.outcome}${event.deferred > 0 ? ` (${event.deferred} deferred)` : ''}`;
    default:
      return null;
  }
}

export async function follow({
  root = process.cwd(),
  session,
  run,
  latest = false,
  write = (line) => process.stdout.write(`${line}\n`),
  pollMs = 500,
  timeoutMs,
  sleep = delay,
} = {}) {
  const deadline = timeoutMs === undefined ? Infinity : Date.now() + timeoutMs;
  // A session pointer or `latest` can still name the previous push's closed run; wait for the
  // next one instead of replaying it. An explicit `run` is always read, closed or not.
  const startedAt = Date.now();
  const waitForNext = !run && (session != null || latest);
  const checkout = resolve(root);
  const directory = liveRoot(checkout);
  let path = null;
  let size = -1;
  let seen = 0;

  for (;;) {
    if (Date.now() >= deadline) return 'unknown';

    if (!path) {
      path = findAt(checkout, directory, { session, run, latest });
      if (!path && (run || (session == null && !latest))) return 'unknown';
      if (path && waitForNext && closedBefore(path, startedAt)) path = null;
    }

    if (path) {
      let stat;
      try {
        stat = statSync(path);
      } catch (error) {
        if (error.code === 'ENOENT') return 'interrupted';
        throw error;
      }

      if (stat.size !== size) {
        let state;
        try {
          state = readRun(path);
        } catch (error) {
          if (error.code === 'ENOENT') return 'interrupted';
          throw error;
        }

        if (state.events.length < seen) seen = 0;
        for (const event of state.events.slice(seen)) {
          const line = formatEvent(event);
          if (line !== null) write(line);
          if (event.kind === 'run-close' && OUTCOMES.has(event.outcome)) {
            return event.outcome;
          }
        }
        seen = state.events.length;
        size = stat.size;
        if (state.outcome === 'interrupted') return 'interrupted';
      }

      if (Date.now() - stat.mtimeMs > STALE_MS) return 'interrupted';
    }

    const remaining = deadline - Date.now();
    if (remaining <= 0) return 'unknown';
    await sleep(Math.min(pollMs, remaining));
  }
}

function closedBefore(path, instant) {
  try {
    const { events } = readRun(path);
    return events.at(-1)?.kind === 'run-close' && statSync(path).mtimeMs < instant;
  } catch {
    return false;
  }
}

const USAGE = `Usage:
  node gate-live.mjs open --sha S --target T [--session S]
  node gate-live.mjs event --run PATH --json '<event json>'
  node gate-live.mjs close --run PATH --outcome passed|gates-passed|failed|interrupted [--deferred N | --deferred-from-log]
  node gate-live.mjs follow [--session S | --run R | --latest] [--timeout-ms N]
  node gate-live.mjs show (--run R | --session S | --latest) [--seq N]
`;

function parseArguments(argv) {
  const [verb, ...args] = argv;
  const allowed = {
    open: ['sha', 'target', 'session'],
    event: ['run', 'json'],
    close: ['run', 'outcome', 'deferred', 'deferred-from-log'],
    follow: ['session', 'run', 'latest', 'timeout-ms'],
    show: ['session', 'run', 'latest', 'seq'],
  };
  if (!Object.hasOwn(allowed, verb)) throw new Error('unknown verb');

  const options = Object.create(null);
  for (let index = 0; index < args.length; index += 1) {
    const flag = args[index];
    const name = flag.startsWith('--') ? flag.slice(2) : '';
    if (!allowed[verb].includes(name) || Object.hasOwn(options, name)) {
      throw new Error('unknown or repeated option');
    }
    if (name === 'latest' || name === 'deferred-from-log') {
      options[name] = true;
    } else {
      const value = args[++index];
      if (!value || value.startsWith('--')) throw new Error('missing option value');
      options[name] = value;
    }
  }

  const required = {
    open: ['sha', 'target'],
    event: ['run', 'json'],
    close: ['run', 'outcome'],
    follow: [],
    show: [],
  };
  if (required[verb].some((name) => !Object.hasOwn(options, name))) {
    throw new Error('missing required option');
  }

  const selectors = ['session', 'run', 'latest']
    .filter((name) => Object.hasOwn(options, name)).length;
  if (
    ((verb === 'follow' || verb === 'show') && selectors > 1)
    || (verb === 'show' && selectors === 0)
  ) {
    throw new Error('choose one run selector');
  }

  for (const name of ['seq', 'timeout-ms', 'deferred']) {
    if (Object.hasOwn(options, name)) {
      if (!/^\d+$/.test(options[name]) || !Number.isSafeInteger(Number(options[name]))) {
        throw new Error('expected a non-negative integer');
      }
      options[name] = Number(options[name]);
    }
  }

  if (verb === 'close' && !OUTCOMES.has(options.outcome)) {
    throw new Error('invalid outcome');
  }
  if (options.deferred !== undefined && options['deferred-from-log']) {
    throw new Error('choose one deferred count source');
  }
  if (verb === 'event') {
    options.event = JSON.parse(options.json);
    if (
      options.event === null
      || typeof options.event !== 'object'
      || Array.isArray(options.event)
    ) {
      throw new Error('event must be a JSON object');
    }
  }

  return { verb, options };
}

async function main(argv) {
  let command;
  try {
    command = parseArguments(argv);
  } catch {
    process.stderr.write(USAGE);
    return 2;
  }

  const { verb, options } = command;
  const root = process.cwd();

  try {
    switch (verb) {
      case 'open': {
        const result = openRun({
          root,
          sha: options.sha,
          target: options.target,
          session: options.session ?? resolveSession(),
        });
        process.stdout.write(`${result.path}\n`);
        return 0;
      }
      case 'event':
        if (appendEvent(resolve(root, options.run), options.event) === null) {
          throw new Error(`could not append event to ${options.run}`);
        }
        return 0;
      case 'close':
        if (closeRun(resolve(root, options.run), options.outcome, {
          deferred: options.deferred,
          deferredFromLog: options['deferred-from-log'],
        }) === null) {
          throw new Error(`could not close ${options.run}`);
        }
        return 0;
      case 'follow': {
        let session = options.session;
        let latest = options.latest ?? false;
        if (!options.run && session === undefined && !latest) {
          session = resolveSession();
          latest = session === null;
        }
        const outcome = await follow({
          root,
          session,
          run: options.run,
          latest,
          timeoutMs: options['timeout-ms'],
        });
        return outcome === 'passed' || outcome === 'gates-passed' ? 0 : outcome === 'failed' ? 1 : 3;
      }
      case 'show': {
        const path = findRun({ root, ...options });
        if (!path) throw new Error('run not found');
        const { events } = readRun(path);
        for (const event of events) {
          if (options.seq === undefined || event.seq === options.seq) {
            process.stdout.write(`${JSON.stringify(event)}\n`);
          }
        }
        return 0;
      }
      default:
        return 2;
    }
  } catch (error) {
    const message = String(error?.message ?? error).replace(/[\r\n]+/g, ' ');
    process.stderr.write(`[gate-live] ${message}\n`);
    if (verb === 'open' || verb === 'event' || verb === 'close') return 0;
    return verb === 'follow' ? 3 : 1;
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = await main(process.argv.slice(2));
}
