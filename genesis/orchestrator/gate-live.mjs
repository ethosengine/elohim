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
    // An interrupted Task gave no verdict, so the run is interrupted whatever its loop concluded,
    // and carries that Task's three-part answer.
    const interrupted = events.find((event) => event.kind === 'gate-verdict'
      && event.status === 'Unanswered' && event.verdict?.deferred !== true && event.unanswered);
    return appendEvent(path, {
      kind: 'run-close',
      outcome: interrupted ? 'interrupted' : outcome,
      ...(deferred === undefined ? {} : { deferred }),
      ...(interrupted ? { unanswered: interrupted.unanswered } : {}),
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
      const by = event.attribution && event.attribution.label !== 'unattributed'
        ? `[${event.attribution.label}${event.attribution.label === 'mine' ? '' : `: ${event.attribution.sessions.map(s => s.slice(0, 8)).join(',')}`}]`
        : '';
      const prefix = ['[gate-live] RED', event.project, failureClass, file, test, by]
        .filter((part) => part !== undefined && part !== null && part !== '')
        .join(' ');
      const cure = lines.find((line) => /^cure: /.test(line.trim()));
      return `${prefix} — ${(lines[0] ?? '').slice(0, 160)}${cure ? ` · ${cure.trim().slice(0, 160)}` : ''}`;
    }
    case 'outlook': {
      const remaining = Array.isArray(event.remaining) ? event.remaining : [];
      const stop = event.stop ? ` — to stop and fix now: ${event.stop}` : '';
      if (!remaining.length) return `[gate-live] no gate left to run after this red; the verdict is close`;
      const parts = remaining.slice(0, 4)
        .map((gate) => `${gate.project} ${gate.p90_s == null ? '~?' : `~${Math.max(1, Math.round(gate.p90_s / 60))}m`}`);
      const more = remaining.length > 4 ? ` +${remaining.length - 4}` : '';
      return `[gate-live] the push keeps reporting: ${remaining.length} gate(s) still to run, ~${Math.max(1, Math.round((event.total_s ?? 0) / 60))} min (${parts.join(', ')}${more})${stop}`;
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
    case 'stop-request':
      return `[gate-live] stop requested${event.reason ? ` (${event.reason})` : ''} — gates not yet started will not run`;
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

// What an agent hears without asking: every red, a failed verdict, and the close.
function noteworthy(event) {
  if (event.kind !== 'gate-verdict') {
    return ['failure', 'detail', 'step-failed', 'outlook', 'stop-request', 'run-close'].includes(event.kind);
  }
  const verdict = event.verdict && typeof event.verdict === 'object' ? event.verdict : null;
  return verdict
    ? verdict.exit !== 0 && !verdict.reused && !verdict.deferred
    : String(event.verdict ?? event.outcome ?? '') === 'failed';
}

// The lines this reader has not yet been shown from its session's run. Every agent sharing a
// session (a main loop and its subagents) keeps its own cursor, so one hearing a red never
// consumes it for another. A run already over at first contact is history, not news: the cursor
// starts at its end. The cursor records the size stat'd BEFORE reading, so a write racing this
// read leaves the size stale and the next look re-reads instead of skipping it.
export function unseen({ root = process.cwd(), session, reader = 'main' } = {}) {
  if (session == null || session === '') return [];
  const checkout = resolve(root);
  const directory = liveRoot(checkout);
  const path = findAt(checkout, directory, { session });
  if (!path) return [];
  const { size } = statSync(path);
  const { events, outcome } = readRun(path);

  const cursors = join(directory, 'cursors');
  const cursorPath = join(cursors, `${sessionFilename(session)}--${sessionFilename(reader)}.json`);
  let cursor = null;
  try {
    cursor = JSON.parse(readFileSync(cursorPath, 'utf8'));
  } catch {
    // No cursor yet: this reader's first look at its session.
  }
  const fresh = cursor?.run === path
    ? events.filter((event) => Number.isInteger(event.seq) && event.seq > cursor.seq)
    : outcome === 'running' ? events : [];
  const seq = events.reduce(
    (last, event) => (Number.isInteger(event.seq) && event.seq > last ? event.seq : last),
    cursor?.run === path ? cursor.seq : -1,
  );
  mkdirSync(cursors, { recursive: true });
  writeFileSync(cursorPath, JSON.stringify({ run: path, seq, size }), 'utf8');
  return fresh.filter(noteworthy).map(formatEvent).filter((line) => line !== null);
}

function closedBefore(path, instant) {
  try {
    const { events } = readRun(path);
    return events.at(-1)?.kind === 'run-close' && statSync(path).mtimeMs < instant;
  } catch {
    return false;
  }
}

/**
 * The developer's or integrator's stop: the hook and gate-runner read it before each gate, so the
 * gate already running finishes and nothing after it starts. Kill the push for an immediate stop.
 */
export function requestStop(path, { reason = '', now } = {}) {
  return appendEvent(path, { kind: 'stop-request', ...(reason ? { reason } : {}) }, now ? { now } : {});
}

export function stopRequested(path) {
  try {
    return parseEvents(readFileSync(path, 'utf8')).some((event) => event.kind === 'stop-request');
  } catch {
    return false;
  }
}

/**
 * After a run's first red, what continuing costs: the gates still to run with their recorded p90,
 * and the exact stop command. Once per run, so the agent gets one decision, not a stream of them.
 * The push itself continues (operator ruling 2026-11-01): the agent weighs signal against cost.
 */
export async function outlook(path, remaining, { root = process.cwd(), rows } = {}) {
  try {
    if (parseEvents(readFileSync(path, 'utf8')).some((event) => event.kind === 'outlook')) return undefined;
    // Loaded on demand: the live channel's other verbs never depend on the cost ledger's reader.
    const { costOf, readCycleUnion } = await import('./gate-cost.mjs');
    let ledger = rows;
    if (ledger === undefined) {
      try {
        const union = readCycleUnion(root);
        ledger = union.readableLedgers ? union.rows : [];
      } catch { ledger = []; }
    }
    const gates = remaining.map((project) => ({
      project, p90_s: costOf(ledger, { project: { name: project }, subjectFallback: null }).p90,
    }));
    return appendEvent(path, {
      kind: 'outlook',
      remaining: gates,
      total_s: gates.reduce((sum, gate) => sum + (gate.p90_s ?? 120), 0),
      stop: `node genesis/orchestrator/gate-live.mjs stop --run ${path}`,
    });
  } catch {
    return null;
  }
}

// A red the preflight or a pre-gate leg raised, or one from a gate whose own verdict took under
// two minutes, is cheap: arriving later than ten minutes into the run means it waited behind
// something long.
const CHEAP_GATE_S = 120;
const LATE_S = 600;

/**
 * Read this device's last `last` closed runs and name the slices worth an integrator's hour:
 * reds that recur across pushes, cheap reds that arrived late, and the minutes spent after a run's
 * first red. Reads only the device-local run logs; writes nothing.
 */
export function harvest({ root = process.cwd(), last = 10 } = {}) {
  const directory = liveRoot(resolve(root));
  const runsDir = join(directory, 'runs');
  let names = [];
  try { names = runFiles(runsDir); } catch { return { runs: 0, slices: [] }; }
  const runs = [];
  for (const name of names) {
    if (runs.length >= last) break;
    let events;
    try { events = parseEvents(readFileSync(join(runsDir, name), 'utf8')); } catch { continue; }
    const open = events.find((event) => event.kind === 'run-open');
    const close = events.findLast((event) => event.kind === 'run-close');
    if (!open || !close) continue;
    const t0 = Date.parse(open.at);
    const at = (event) => (Date.parse(event.at) - t0) / 1000;
    const seconds = new Map();
    for (const event of events.filter((e) => e.kind === 'gate-verdict' && e.verdict)) {
      seconds.set(event.project, Number(event.verdict.seconds) || 0);
    }
    // A failure-shaped line inside a gate whose verdict was green (a test named "…REFUSED…", a
    // logged refusal the test expects) is noise, not a red: only reds from red or unfinished gates
    // and from pre-gate checks count.
    const green = new Set(events.filter((e) => e.kind === 'gate-verdict' && e.verdict && e.verdict.exit === 0)
      .map((e) => e.project));
    const reds = events.filter((event) => event.kind === 'failure' && !green.has(event.project)).map((event) => {
      const failure = event.failure && typeof event.failure === 'object' ? event.failure : event;
      return { project: event.project ?? 'unknown', cls: failure.class ?? 'unknown', at: at(event), first: String(excerptLines(failure.excerpt)[0] ?? '').slice(0, 120) };
    });
    runs.push({ name, sha: open.candidate?.sha ?? null, outcome: close.outcome, total: at(close), reds, seconds });
  }

  const slices = [];
  const byKey = new Map();
  for (const run of runs) {
    for (const key of new Set(run.reds.map((red) => `${red.project}/${red.cls}`))) {
      const red = run.reds.find((r) => `${r.project}/${r.cls}` === key);
      if (!byKey.has(key)) byKey.set(key, []);
      byKey.get(key).push({ run, red });
    }
  }
  for (const [key, hits] of byKey) {
    if (hits.length < 2) continue;
    slices.push({
      kind: 'recurring', key, count: hits.length,
      cost_s: hits.reduce((sum, { run }) => sum + run.total, 0),
      line: `${key} red in ${hits.length} of the last ${runs.length} runs (latest: ${hits[0].red.first || 'no excerpt'}) — move its cheapest detector left: gate-preflight.mjs, a pre-commit leg, or an edit-time hook`,
    });
  }
  for (const run of runs) {
    for (const red of run.reds) {
      const cheap = red.project === 'preflight' || red.cls === 'pre-gate'
        || (run.seconds.has(red.project) && run.seconds.get(red.project) < CHEAP_GATE_S);
      if (!cheap || red.at <= LATE_S) continue;
      slices.push({
        kind: 'late-cheap-red', key: `${red.project}/${red.cls}`, cost_s: red.at,
        line: `${red.project}/${red.cls} is a cheap red that arrived ${Math.round(red.at / 60)} min into run ${run.name.slice(0, 16)} — order it ahead of the long gates`,
      });
      break;
    }
  }
  const failed = runs.filter((run) => run.reds.length > 0 && run.outcome !== 'passed' && run.outcome !== 'gates-passed');
  const after = failed.reduce((sum, run) => sum + Math.max(0, run.total - run.reds[0].at), 0);
  if (failed.length && after > LATE_S) {
    slices.push({
      kind: 'spent-after-red', key: 'policy', cost_s: after,
      line: `${Math.round(after / 60)} min ran after the first red across ${failed.length} failed run(s) — the reds were live by then; stop with \`gate-live.mjs stop --run <run>\` or GATE_ON_RED=stop when the rest of the report is not worth that`,
    });
  }
  slices.sort((a, b) => b.cost_s - a.cost_s);
  return { runs: runs.length, slices };
}

export function formatHarvest({ runs, slices }) {
  if (!runs) return ['[harvest] no closed gate runs on this device yet'];
  if (!slices.length) return [`[harvest] ${runs} recent run(s): no recurring or late cheap reds`];
  return [`[harvest] ${slices.length} QoL slice(s) from the last ${runs} run(s), costliest first:`,
    ...slices.slice(0, 6).map((slice) => `[harvest]   ${slice.kind}: ${slice.line}`)];
}

const USAGE = `Usage:
  node gate-live.mjs open --sha S --target T [--session S]
  node gate-live.mjs event --run PATH --json '<event json>'
  node gate-live.mjs close --run PATH --outcome passed|gates-passed|failed|interrupted [--deferred N | --deferred-from-log]
  node gate-live.mjs follow [--session S | --run R | --latest] [--timeout-ms N]
  node gate-live.mjs show (--run R | --session S | --latest) [--seq N]
  node gate-live.mjs unseen --session S [--reader R]
  node gate-live.mjs red --run PATH --project P --message M     (a pre-gate leg's red, live at once)
  node gate-live.mjs stop [--run PATH | --session S | --latest] [--reason R]
  node gate-live.mjs stopped --run PATH                         (exit 0 when a stop was requested)
  node gate-live.mjs outlook --run PATH --remaining "a b c"     (once per run: what continuing costs)
  node gate-live.mjs harvest [--last N]
`;

function parseArguments(argv) {
  const [verb, ...args] = argv;
  const allowed = {
    open: ['sha', 'target', 'session'],
    event: ['run', 'json'],
    close: ['run', 'outcome', 'deferred', 'deferred-from-log'],
    follow: ['session', 'run', 'latest', 'timeout-ms'],
    show: ['session', 'run', 'latest', 'seq'],
    unseen: ['session', 'reader'],
    red: ['run', 'project', 'message'],
    stop: ['run', 'session', 'latest', 'reason'],
    stopped: ['run'],
    harvest: ['last'],
    outlook: ['run', 'remaining'],
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
    unseen: ['session'],
    red: ['run', 'project', 'message'],
    stop: [],
    stopped: ['run'],
    harvest: [],
    outlook: ['run'],
  };
  if (required[verb].some((name) => !Object.hasOwn(options, name))) {
    throw new Error('missing required option');
  }

  const selectors = ['session', 'run', 'latest']
    .filter((name) => Object.hasOwn(options, name)).length;
  if (
    ((verb === 'follow' || verb === 'show' || verb === 'stop') && selectors > 1)
    || (verb === 'show' && selectors === 0)
  ) {
    throw new Error('choose one run selector');
  }

  for (const name of ['seq', 'timeout-ms', 'deferred', 'last']) {
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
      case 'red':
        if (appendEvent(resolve(root, options.run), {
          kind: 'failure', project: options.project,
          failure: { class: 'pre-gate', excerpt: [options.message] },
        }) === null) {
          throw new Error(`could not append red to ${options.run}`);
        }
        return 0;
      case 'stop': {
        let session = options.session;
        let latest = options.latest ?? false;
        if (!options.run && session === undefined && !latest) {
          session = resolveSession();
          latest = session === null;
        }
        const path = options.run ? resolve(root, options.run) : findRun({ root, session, latest });
        if (!path || readRun(path).outcome !== 'running') throw new Error('no running gate run to stop');
        if (requestStop(path, { reason: options.reason }) === null) throw new Error(`could not request a stop on ${path}`);
        process.stdout.write(`stop requested: ${path}\n`);
        return 0;
      }
      case 'outlook':
        if (await outlook(resolve(root, options.run), String(options.remaining ?? '').split(/\s+/).filter(Boolean), { root }) === null) {
          throw new Error(`could not append an outlook to ${options.run}`);
        }
        return 0;
      case 'stopped':
        return stopRequested(resolve(root, options.run)) ? 0 : 1;
      case 'harvest':
        for (const line of formatHarvest(harvest({ root, last: options.last ?? 10 }))) {
          process.stdout.write(`${line}\n`);
        }
        return 0;
      case 'unseen':
        for (const line of unseen({ root, session: options.session, reader: options.reader })) {
          process.stdout.write(`${line}\n`);
        }
        return 0;
      default:
        return 2;
    }
  } catch (error) {
    const message = String(error?.message ?? error).replace(/[\r\n]+/g, ' ');
    process.stderr.write(`[gate-live] ${message}\n`);
    if (['open', 'event', 'close', 'unseen', 'red', 'harvest', 'outlook'].includes(verb)) return 0;
    return verb === 'follow' ? 3 : 1;
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = await main(process.argv.slice(2));
}
