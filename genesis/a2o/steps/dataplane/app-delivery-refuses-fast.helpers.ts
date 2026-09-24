/**
 * Fixtures for features/dataplane/app-delivery-refuses-fast.feature.
 *
 * Four jobs, and nothing else lives here:
 *   1. ask the fleet write-readiness probe (scripts/ci/fleet-write-readiness.sh) exactly the
 *      way a pipeline asks it, and bound how long the ASKER waits on it, so a probe that
 *      hangs is reported as hanging instead of hanging the story with it;
 *   2. read its NOT-READY lines and match the host each one names to a doorway;
 *   3. read back the timing lines scripts/ci/stage-spa-blob.sh prints while it waits, and
 *      judge them against the budgets the story told it;
 *   4. make one household doorway shed on purpose, and stop it again.
 *
 * Publishing reuses the deliverability story's own fixtures (buildFixtureBundle, stageBundle,
 * pollUntil, meshControl in ./epr-app-deliverability.helpers.ts) — the deploy path itself,
 * never a re-implementation of it.
 *
 * The probe's contract (Lane A of the 2026-09-24 native-delivery sprint plan):
 *   fleet-write-readiness.sh <doorway-url>...
 *   exit 0  every doorway can take a write now
 *   exit 3  one line per doorway that cannot:
 *           FLEET-NOT-READY <host> face=<cell-not-running|catching-up|storage-forward-timeout> retryAfter=<s>
 *   exit 2  usage
 *   It asks once and never sleeps.
 */

import { spawn } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

import { REPO_ROOT } from './epr-app-deliverability.helpers.js';

/** The probe a deploy asks before it writes — owned by scripts/ci, never re-implemented here. */
export const FLEET_WRITE_READINESS = join(REPO_ROOT, 'scripts', 'ci', 'fleet-write-readiness.sh');

export const READINESS_EXIT = { ready: 0, usage: 2, notReady: 3 } as const;

/** The three causes a not-ready answer may name (stage-spa-blob.sh `not_ready_face`). */
export const NOT_READY_FACES = ['cell-not-running', 'catching-up', 'storage-forward-timeout'];

/** Seconds allowed for an attempt already on the wire when a deadline passes (the feature's bound). */
export const IN_FLIGHT_SLACK_SECS = 30;

/** Admin key the household doorways accept on their fixture-only control surface. */
const ADMIN_KEY =
  process.env['API_KEY_ADMIN'] ?? process.env['MESH_API_KEY_ADMIN'] ?? 'mesh-admin-dev-key';
const REQUEST_TIMEOUT_MS = 15_000;

export function readinessProbePresent(script = FLEET_WRITE_READINESS): boolean {
  return existsSync(script);
}

export interface NotReadyLine {
  host: string;
  face: string;
  /** Seconds, or null when the line carried no usable number. */
  retryAfter: number | null;
  raw: string;
}

export function parseNotReadyLines(output: string): NotReadyLine[] {
  const lines: NotReadyLine[] = [];
  for (const raw of output.split('\n')) {
    const match = /FLEET-NOT-READY\s+(\S+)\s+face=(\S+)\s+retryAfter=(\S*)/.exec(raw);
    if (!match) continue;
    const seconds = Number(match[3]);
    lines.push({
      host: match[1],
      face: match[2],
      retryAfter: match[3] !== '' && Number.isFinite(seconds) ? seconds : null,
      raw: raw.trim(),
    });
  }
  return lines;
}

/** `http://x/` → `http://x`, without a backtracking regex. */
export function trimTrailingSlashes(value: string): string {
  let end = value.length;
  while (end > 0 && value[end - 1] === '/') end -= 1;
  return value.slice(0, end);
}

/**
 * Does the host a NOT-READY line names mean this doorway? A URL or origin must match the
 * doorway's origin; anything else must equal its host INCLUDING the port. A bare hostname
 * never matches: both household doorways share one machine, so "localhost" names neither.
 */
export function namesDoorway(host: string, doorwayUrl: string): boolean {
  const doorway = new URL(doorwayUrl);
  const token = trimTrailingSlashes(host);
  if (/^[a-z][a-z0-9+.-]*:\/\//i.test(token)) {
    try {
      return new URL(token).origin === doorway.origin;
    } catch {
      return false;
    }
  }
  return token === doorway.host;
}

export interface ReadinessAnswer {
  /** Exit status, or null when the asker stopped a probe that outlived its bound. */
  code: number | null;
  timedOut: boolean;
  output: string;
  startedAt: number;
  elapsedMs: number;
  notReady: NotReadyLine[];
}

/**
 * Ask the probe once. The probe runs in its own process group so that a probe which
 * outlives `boundMs` is stopped together with anything it started — a hung child holding
 * the output pipe open must not make the asker wait with it.
 */
export async function askFleetWriteReadiness(
  doorwayUrls: string[],
  boundMs: number,
  script = FLEET_WRITE_READINESS
): Promise<ReadinessAnswer> {
  const startedAt = Date.now();
  return new Promise(resolve => {
    // eslint-disable-next-line sonarjs/no-os-command-from-path -- dev-only a2o harness, fixed script arg
    const child = spawn('bash', [script, ...doorwayUrls], {
      cwd: REPO_ROOT,
      env: process.env,
      detached: true,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let output = '';
    let timedOut = false;
    child.stdout.on('data', chunk => (output += String(chunk)));
    child.stderr.on('data', chunk => (output += String(chunk)));
    const timer = setTimeout(() => {
      timedOut = true;
      try {
        if (child.pid) process.kill(-child.pid, 'SIGKILL');
      } catch {
        child.kill('SIGKILL');
      }
    }, boundMs);
    const finish = (code: number | null) => {
      clearTimeout(timer);
      resolve({
        code: timedOut ? null : code,
        timedOut,
        output,
        startedAt,
        elapsedMs: Date.now() - startedAt,
        notReady: parseNotReadyLines(output),
      });
    };
    child.on('error', error => {
      output += `\n${error.message}`;
      finish(null);
    });
    child.on('close', code => finish(code));
  });
}

export type TimingKind =
  | 'readiness-wait'
  | 'readiness-exhausted'
  | 'readiness-cleared'
  | 'transport-retry'
  | 'transport-exhausted';

export interface TimingLine {
  kind: TimingKind;
  elapsedSecs: number;
  /** The budget the deploy says it is working to, or null when the line names none. */
  budgetSecs: number | null;
  face: string | null;
  raw: string;
}

/** Each pattern mirrors one echo template in scripts/ci/stage-spa-blob.sh. */
const TIMING_PATTERNS: {
  kind: TimingKind;
  re: RegExp;
  read: (m: RegExpExecArray) => Omit<TimingLine, 'kind' | 'raw'>;
}[] = [
  {
    kind: 'readiness-wait',
    // eslint-disable-next-line sonarjs/slow-regex -- one bounded log line; negated classes cannot overlap
    re: /still not ready on \S+ \(face=([^,)]+)[^)]*\) — (\d+)s waited, (-?\d+)s left of (\d+)s;/,
    read: m => ({ face: m[1], elapsedSecs: Number(m[2]), budgetSecs: Number(m[4]) }),
  },
  {
    kind: 'readiness-exhausted',
    re: /readiness deadline reached; the holder was still not ready after (\d+)s \(last face=([^,]+), budget (\d+)s/,
    read: m => ({ face: m[2], elapsedSecs: Number(m[1]), budgetSecs: Number(m[3]) }),
  },
  {
    kind: 'readiness-cleared',
    re: /took the write after (\d+)s of not-ready answers \(last face=([^)]+)\)/,
    read: m => ({ face: m[2], elapsedSecs: Number(m[1]), budgetSecs: null }),
  },
  {
    kind: 'transport-retry',
    re: /\(elapsed (\d+)s, (-?\d+)s left of (\d+)s budget\)/,
    read: m => ({ face: null, elapsedSecs: Number(m[1]), budgetSecs: Number(m[3]) }),
  },
  {
    kind: 'transport-exhausted',
    re: /stage failed after \d+ attempt\(s\) \/ (\d+)s against \S+ \(budget (\d+)s/,
    read: m => ({ face: null, elapsedSecs: Number(m[1]), budgetSecs: Number(m[2]) }),
  },
];

export function parseTimingLines(output: string): TimingLine[] {
  const lines: TimingLine[] = [];
  for (const raw of output.split('\n')) {
    for (const pattern of TIMING_PATTERNS) {
      const match = pattern.re.exec(raw);
      if (!match) continue;
      lines.push({ kind: pattern.kind, raw: raw.trim(), ...pattern.read(match) });
      break;
    }
  }
  return lines;
}

export interface ToldBudgets {
  readinessSecs: number;
  transportSecs: number;
  inFlightSlackSecs: number;
}

/**
 * Every way a deploy's own timing lines can show it waited longer than it was told.
 * A re-offer may only be SCHEDULED while time is left; a stop may be reported up to one
 * in-flight attempt past the deadline; and every line must name the budget the story
 * actually told it (a line naming another number means the deploy never read ours).
 */
export function timingViolations(lines: TimingLine[], told: ToldBudgets): string[] {
  return lines.flatMap(line => {
    const verdict = judgeTimingLine(line, told);
    return verdict ? [verdict] : [];
  });
}

/** One line's verdict, or null when it obeyed what the deploy was told. */
function judgeTimingLine(line: TimingLine, told: ToldBudgets): string | null {
  const isReadiness = line.kind.startsWith('readiness');
  const toldBudget = isReadiness ? told.readinessSecs : told.transportSecs;
  if (line.budgetSecs !== null && line.budgetSecs !== toldBudget) {
    const which = isReadiness ? 'readiness' : 'transport';
    return `the deploy worked to a ${line.budgetSecs}s ${which} budget, but was told ${toldBudget}s: ${line.raw}`;
  }
  if (line.kind === 'readiness-wait') {
    return line.elapsedSecs >= toldBudget
      ? `a re-offer was scheduled ${line.elapsedSecs}s into a ${toldBudget}s readiness budget: ${line.raw}`
      : null;
  }
  if (line.kind === 'transport-retry') {
    return line.elapsedSecs > toldBudget
      ? `a retry was scheduled ${line.elapsedSecs}s into a ${toldBudget}s transport budget: ${line.raw}`
      : null;
  }
  const limit = toldBudget + told.inFlightSlackSecs;
  return line.elapsedSecs > limit
    ? `${line.elapsedSecs}s is past the ${toldBudget}s budget plus ${told.inFlightSlackSecs}s for an attempt in flight: ${line.raw}`
    : null;
}

/** `first_not_ready=<epoch>` from the one run-deadline record stage-spa-blob.sh keeps. */
export function readFirstNotReady(stateDir: string): number | null {
  try {
    const match = /^first_not_ready=(\d+)$/m.exec(
      readFileSync(join(stateDir, 'run-deadline'), 'utf8')
    );
    return match ? Number(match[1]) : null;
  } catch {
    return null;
  }
}

/** Run `fn` with extra environment variables set, restoring the previous values after. */
export async function withEnv<T>(vars: Record<string, string>, fn: () => Promise<T>): Promise<T> {
  const previous = Object.fromEntries(Object.keys(vars).map(key => [key, process.env[key]]));
  Object.assign(process.env, vars);
  try {
    return await fn();
  } finally {
    for (const [key, value] of Object.entries(previous)) {
      if (value === undefined) delete process.env[key];
      else process.env[key] = value;
    }
  }
}

export interface ShedAnswer {
  status: number;
  text: string;
  retryAfter?: string;
}

/**
 * `PUT {doorway}/admin/dev/shed {"retryAfterSecs": N}` — the household's fixture-only
 * control (doorway-service routes/admin_dev.rs, the same surface name-routing.feature
 * uses). `0` clears a standing shed. It answers only on a doorway that declared the
 * household stage at boot, and only to a loopback caller.
 */
export async function putShed(doorwayUrl: string, retryAfterSecs: number): Promise<ShedAnswer> {
  const response = await fetch(`${doorwayUrl}/admin/dev/shed`, {
    method: 'PUT',
    headers: { Authorization: `Bearer ${ADMIN_KEY}`, 'Content-Type': 'application/json' },
    body: JSON.stringify({ retryAfterSecs }),
    signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
  });
  return { status: response.status, text: await response.text() };
}

/**
 * Confirm the doorway really is turning writes away: a write-route read forced local-only
 * (x-federation-hop: 1, so no sibling answers for it) must come back 503 with a Retry-After.
 */
export async function probeShedding(doorwayUrl: string): Promise<ShedAnswer> {
  const response = await fetch(`${doorwayUrl}/db/content/app-delivery-refuses-fast-shed-probe`, {
    headers: { 'x-federation-hop': '1', Accept: 'application/json' },
    signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
  });
  return {
    status: response.status,
    text: await response.text(),
    retryAfter: response.headers.get('retry-after') ?? undefined,
  };
}
