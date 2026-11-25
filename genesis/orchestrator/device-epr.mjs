#!/usr/bin/env node
/**
 * The `epr` a DEVICE act runs: this device's current head, whatever release the invoking agent
 * session is pinned to.
 *
 * A session keeps the epr it started with while fixes land. An act the device performs for its
 * other devices (a landing intent and its misses, a gate verdict crossing, a read of sibling
 * verdicts or carries, the household head landing, the steward lease view) is not the session's:
 * a sibling reads its result, so it runs the release the device has landed. Session acts
 * (readings, cycle timing, session identity, envelopes, stasis) keep `env.EPR_BIN || 'epr'` and
 * are not routed through this module.
 *
 * This is the ONE resolver. JS call sites import `deviceEpr`; shell and Python call sites run
 * this file as a CLI. gate-preflight's `epr-bin` check (and so device-arrive) reads the head and
 * judges "older" with `deviceHead` and `compareToHead` below; there is no second copy.
 *
 * What it promises, and how:
 * - Never older than the session's own epr. An `EPR_BIN` the store ingested before the head is
 *   ignored with one line. An `EPR_BIN` that is a candidate (outside the store, undated, or
 *   ingested after the head) is used as it is. With no `EPR_BIN`, the launcher is asked once what
 *   this session resolves to (`epr bin resolve --tool epr --json`, bin store only); when that is
 *   not this head and not older than it, or cannot be read, the act stays on the PATH launcher.
 * - No silent fallback. When the head cannot be read the act uses `env.EPR_BIN || 'epr'` exactly
 *   as before and one line says so. Nothing here fails a gate or a push.
 * - Cheap and bounded. The head is read from the bin store's files (no subprocess, no lock, never
 *   the flow store); the one session question is a subprocess bounded at SESSION_TIMEOUT_MS. The
 *   answer is cached for the process, per environment.
 *
 * CLI: `node device-epr.mjs` prints the absolute path of the device epr and exits 0. When the
 * act stays on the session's launcher (the fallback, or a session epr that is not older) it
 * prints nothing on stdout, so the caller runs its own PATH resolution unchanged; the line that
 * says why is on stderr. `--json` prints `{command, source, why}`.
 */
import { spawnSync } from 'node:child_process';
import { accessSync, constants, existsSync, readFileSync, realpathSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const SESSION_TIMEOUT_MS = 2000;

/**
 * This device's current head epr, read from the bin store without executing anything:
 * `{ path, cid, store }`. Throws when it cannot be read. Mirrors Store::from_environment and
 * bin_store head(): explicit store overrides win, else the installed launcher names the store;
 * current.json wins over the symlink.
 */
export function deviceHead({ env = process.env } = {}) {
  let store = env.EPR_STORE || (env.BIN_STORE && join(env.BIN_STORE, 'epr'));
  if (!store) {
    const launcher = realpathSync(join(env.HOME, '.local/bin/epr'));
    if (!launcher.endsWith('/bin/bin-launch')) throw new Error('unknown launcher');
    store = join(dirname(dirname(launcher)), 'epr');
  }
  const head = join(store, 'current.json');
  let path;
  if (existsSync(head)) {
    const { cid } = JSON.parse(readFileSync(head, 'utf8'));
    if (typeof cid !== 'string' || !/^b[a-z2-7]+$/.test(cid)) throw new Error('invalid head');
    path = realpathSync(join(store, cid, 'epr'));
  } else {
    path = realpathSync(join(store, 'current/epr'));
  }
  return { path, cid: storeCid(path) ?? null, store };
}

const storeCid = path => path?.match(/\/epr\/(b[a-z2-7]+)\/epr$/)?.[1];

/**
 * How `candidate` (a path, as an EPR_BIN names it) stands to the head at `current` (a real
 * path): 'same', 'older' or 'candidate'. Older means a release this device's store ingested
 * before its head, and only that: a build outside the store, an undated one, or one ingested
 * after the head is a candidate someone set on purpose, never proved obsolete by being different.
 */
export function compareToHead(candidate, current) {
  let inherited;
  try { inherited = realpathSync(candidate); } catch { /* A missing candidate differs too. */ }
  if (inherited === current || (storeCid(inherited) && storeCid(inherited) === storeCid(current))) return 'same';
  const ingested = cid => {
    try {
      const at = JSON.parse(readFileSync(join(dirname(dirname(current)), cid, 'record.json'), 'utf8')).ingestedAt;
      const ms = Date.parse(at);
      return Number.isFinite(ms) ? ms : null;
    } catch { return null; }
  };
  const inheritedAt = storeCid(inherited) ? ingested(storeCid(inherited)) : null;
  const currentAt = storeCid(current) ? ingested(storeCid(current)) : null;
  const older = inherited !== undefined && dirname(dirname(inherited)) === dirname(dirname(current))
    && inheritedAt !== null && currentAt !== null && inheritedAt < currentAt;
  return older ? 'older' : 'candidate';
}

function executable(path) {
  try { accessSync(path, constants.X_OK); return true; } catch { return false; }
}

/** What the session's own launcher resolves to, or a reason it could not be read. */
function sessionResolution(env, spawn) {
  // The launcher's own resolution, bin store only. EPR_BIN is not set on this branch.
  let r;
  try {
    r = spawn('epr', ['bin', 'resolve', '--tool', 'epr', '--json'], {
      env, encoding: 'utf8', timeout: SESSION_TIMEOUT_MS, stdio: ['ignore', 'pipe', 'ignore'],
    });
  } catch (error) {
    return { why: String(error?.message || error).split('\n')[0] };
  }
  if (r?.error) return { why: r.error.code || String(r.error.message || r.error).split('\n')[0] };
  if (r?.status !== 0) return { why: `epr bin resolve exited ${r?.status ?? r?.signal ?? 'unknown'}` };
  try {
    const value = JSON.parse(r.stdout);
    if (typeof value?.path !== 'string' || !value.path) return { why: 'epr bin resolve named no path' };
    return { path: value.path, cid: typeof value.cid === 'string' ? value.cid : null };
  } catch {
    return { why: 'epr bin resolve printed no JSON' };
  }
}

/**
 * One uncached resolution: `{ command, source, why }`. `source` is 'head' (the device head),
 * 'override' (an EPR_BIN candidate that is not older than the head), 'session' (the session's
 * own epr is not older than the head) or 'path' (the head could not be established). For
 * 'session' and 'path' the command is `env.EPR_BIN || 'epr'`, the expression every call site
 * used before, and `why` is the one line to print.
 */
export function resolveDeviceEpr({ env = process.env, spawn = spawnSync } = {}) {
  const asBefore = env.EPR_BIN || 'epr';
  let head;
  try {
    head = deviceHead({ env });
    if (!executable(head.path)) throw new Error('head is not executable');
  } catch (error) {
    return {
      command: asBefore, source: 'path',
      why: `[device-epr] this device's head epr could not be read (${String(error?.message || error).split('\n')[0]}); device acts use ${env.EPR_BIN ? `EPR_BIN=${env.EPR_BIN}` : 'the PATH launcher (the session pin applies)'}`,
    };
  }
  if (env.EPR_BIN) {
    const standing = compareToHead(env.EPR_BIN, head.path);
    if (standing === 'same') return { command: head.path, source: 'head', why: null };
    if (standing === 'older') {
      return {
        command: head.path, source: 'head',
        why: `[device-epr] EPR_BIN=${env.EPR_BIN} is older than this device's head epr; ignored for device acts, which run ${head.path}`,
      };
    }
    return { command: env.EPR_BIN, source: 'override', why: null };
  }
  const session = sessionResolution(env, spawn);
  if (!session.path) {
    return {
      command: asBefore, source: 'path',
      why: `[device-epr] this session's own epr could not be read (${session.why}), so the head is not known to be at least as new; device acts use the PATH launcher (the session pin applies)`,
    };
  }
  if (compareToHead(session.path, head.path) === 'candidate') {
    return {
      command: asBefore, source: 'session',
      why: `[device-epr] this session's epr (${session.cid || session.path}) is not older than this device's head (${head.cid || head.path}); device acts stay on the PATH launcher`,
    };
  }
  return { command: head.path, source: 'head', why: null };
}

const cache = new Map();
const KEYED = ['EPR_BIN', 'EPR_STORE', 'BIN_STORE', 'HOME', 'PATH', 'EPR_PIN_CID'];

/** The resolution for this process and environment, computed once; its line is printed once. */
export function deviceEprResolution({
  env = process.env, spawn = spawnSync, warn = line => process.stderr.write(`${line}\n`),
} = {}) {
  const key = JSON.stringify(KEYED.map(name => env[name] ?? null));
  let resolution = cache.get(key);
  if (!resolution) {
    resolution = resolveDeviceEpr({ env, spawn });
    cache.set(key, resolution);
    if (resolution.why) {
      try { warn(resolution.why); } catch { /* a line that cannot print changes nothing */ }
    }
  }
  return resolution;
}

/** The command a device act spawns. Never throws. */
export function deviceEpr(options = {}) {
  return deviceEprResolution(options).command;
}

/** Tests only: forget what this process resolved. */
export function resetDeviceEprCache() {
  cache.clear();
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const resolution = deviceEprResolution();
  if (process.argv.includes('--json')) {
    process.stdout.write(`${JSON.stringify(resolution)}\n`);
  } else if (resolution.source === 'head' || resolution.source === 'override') {
    process.stdout.write(`${resolution.command}\n`);
  }
}
