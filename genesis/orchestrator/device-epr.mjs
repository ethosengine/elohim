#!/usr/bin/env node
/**
 * The `epr` a DEVICE act runs: this device's landed head, whatever release the invoking agent
 * session is pinned to.
 *
 * A session keeps the epr it started with while fixes land. An act the device performs for its
 * other devices (a landing intent and its misses, a gate verdict crossing, a read of sibling
 * verdicts or carries, the household head landing, the steward lease view) is not the session's:
 * a sibling reads its result, so it runs the release the device has landed. Session acts
 * (readings, cycle timing, session identity, envelopes, stasis) keep `env.EPR_BIN || 'epr'` and
 * are not routed through this module.
 *
 * The rule (steward ruling, 2026-11-25):
 * - A device act uses the device's landed head, full stop. Neither `EPR_BIN` nor the session's
 *   pin is consulted, and no age is compared: a rolled-back head is the device's decision and
 *   device acts follow it. (Ingestion time is no release age: a re-ingest refreshes it.)
 * - The head is admitted only as a regular, executable file whose real path is inside the
 *   device's bin store (symlinks resolved first). Anything else is an unreadable head.
 * - An unreadable head keeps `env.EPR_BIN || 'epr'`, the expression every call site used before,
 *   and one line says so. Nothing here fails a gate or a push.
 *
 * This is the ONE resolver. JS call sites import `deviceEpr`; shell and Python call sites run
 * this file as a CLI. The head pointer is re-read on every resolution (bin store files only: no
 * subprocess, no lock, never the flow store; 0.3 ms measured), so a promotion during a long gate
 * is seen by the next device act. Only the diagnostic line is remembered, to print it once.
 *
 * The store is located as bin-launch locates it (Store::from_environment): EPR_STORE, else
 * BIN_STORE/epr, else the store beside the launcher installed at ~/.local/bin/epr. There is no
 * other override.
 *
 * CLI: `node device-epr.mjs` prints the absolute path of the head epr and exits 0. With an
 * unreadable head it prints nothing on stdout, so the caller runs its own resolution unchanged;
 * the line that says why is on stderr. `--json` prints `{command, source, why}`.
 */
import { accessSync, constants, existsSync, readFileSync, realpathSync, statSync } from 'node:fs';
import { dirname, join, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

/**
 * This device's landed head epr, read from the bin store without executing anything:
 * `{ path, cid, store }`. Throws when it cannot be read or is not a regular executable file
 * inside the store. Mirrors bin_store head(): current.json wins over the symlink.
 */
export function deviceHead({ env = process.env } = {}) {
  let store = env.EPR_STORE || (env.BIN_STORE && join(env.BIN_STORE, 'epr'));
  if (!store) {
    const launcher = realpathSync(join(env.HOME, '.local/bin/epr'));
    if (!launcher.endsWith('/bin/bin-launch')) throw new Error('unknown launcher');
    store = join(dirname(dirname(launcher)), 'epr');
  }
  const root = realpathSync(store);
  const pointer = join(root, 'current.json');
  let named;
  if (existsSync(pointer)) {
    const { cid } = JSON.parse(readFileSync(pointer, 'utf8'));
    if (typeof cid !== 'string' || !/^b[a-z2-7]+$/.test(cid)) throw new Error('invalid head');
    named = join(root, cid, 'epr');
  } else {
    named = join(root, 'current/epr');
  }
  const path = realpathSync(named);
  if (!path.startsWith(root + sep)) throw new Error(`head ${named} resolves outside the bin store (${path})`);
  if (!statSync(path).isFile()) throw new Error(`head ${path} is not a regular file`);
  accessSync(path, constants.X_OK);
  return { path, cid: path.slice(root.length + 1).split(sep)[0], store: root };
}

/**
 * One resolution, read afresh: `{ command, source, why }`. `source` is 'head', or 'path' when
 * the head is unreadable; then the command is `env.EPR_BIN || 'epr'` and `why` is the one line.
 */
export function resolveDeviceEpr({ env = process.env } = {}) {
  try {
    return { command: deviceHead({ env }).path, source: 'head', why: null };
  } catch (error) {
    return {
      command: env.EPR_BIN || 'epr', source: 'path',
      why: `[device-epr] this device's head epr could not be read (${String(error?.message || error).split('\n')[0]}); device acts use ${env.EPR_BIN ? `EPR_BIN=${env.EPR_BIN}` : 'the PATH launcher (the session pin applies)'}`,
    };
  }
}

const said = new Set();

/** The resolution for this call; its diagnostic line, if any, is printed once per process. */
export function deviceEprResolution({
  env = process.env, warn = line => process.stderr.write(`${line}\n`),
} = {}) {
  const resolution = resolveDeviceEpr({ env });
  if (resolution.why && !said.has(resolution.why)) {
    said.add(resolution.why);
    try { warn(resolution.why); } catch { /* a line that cannot print changes nothing */ }
  }
  return resolution;
}

/** The command a device act spawns. Never throws. */
export function deviceEpr(options = {}) {
  return deviceEprResolution(options).command;
}

/** Tests only: forget which lines this process has printed. */
export function resetDeviceEprCache() {
  said.clear();
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const resolution = deviceEprResolution();
  if (process.argv.includes('--json')) {
    process.stdout.write(`${JSON.stringify(resolution)}\n`);
  } else if (resolution.source === 'head') {
    process.stdout.write(`${resolution.command}\n`);
  }
}
