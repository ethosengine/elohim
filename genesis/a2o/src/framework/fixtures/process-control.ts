/**
 * Process control for the household mesh's LOCAL processes — the one place the
 * drills find, capture, signal and re-exec a storage peer or doorway.
 *
 * Two traps this module holds so no step file has to rediscover them:
 *
 * 1. procfs files report `st_size == 0`, so `fs.copyFile('/proc/<pid>/environ')`
 *    writes an EMPTY capture. 2026-08-22: a chaos drill captured jessica's environ
 *    as 0 bytes, `hc-mesh.sh storage-restart` answered "no captured environment"
 *    (exit 0), the peer stayed down, and ~21 later scenarios ECONNREFUSED. Read to
 *    EOF, assert non-empty, then write — and record the exe path beside it so the
 *    restart never depends on a default binary path that may not exist.
 * 2. A pid is resolved by EXACT argv match on the listen port, refusing on zero
 *    OR more than one match: a drill that guesses which of two candidates to kill
 *    is a drill that eventually kills the wrong thing.
 *
 * Everything here is Linux-only by design (the mesh is loopback on this host).
 */

import { strict as assert } from 'node:assert';
import { execFileSync, spawn } from 'node:child_process';
import {
  mkdirSync,
  openSync,
  readFileSync,
  readlinkSync,
  realpathSync,
  writeFileSync,
} from 'node:fs';
import path from 'node:path';

/** Absolute path: the pid this lists is about to be signalled, so the binary
 *  that produced the listing must not resolve through a writable PATH. */
const PS_BIN = '/usr/bin/ps';

/** Read a NUL-separated /proc file (cmdline, environ) into its parts. */
export function readNulSeparatedProcFile(procPath: string): string[] {
  return readFileSync(procPath, 'utf8').split('\0').filter(Boolean);
}

/** `/proc/<pid>/environ` as a map. */
export function readProcEnvironment(pid: number): Record<string, string> {
  const env: Record<string, string> = {};
  for (const entry of readNulSeparatedProcFile(`/proc/${pid}/environ`)) {
    const eq = entry.indexOf('=');
    if (eq > 0) env[entry.slice(0, eq)] = entry.slice(eq + 1);
  }
  return env;
}

/**
 * Read one of a live process's /proc files to EOF. Asserts non-empty: an empty
 * environ/cmdline means the process is already a zombie (its mm is gone) and a
 * capture taken now could never bring it back.
 */
export function captureProcFile(pid: number, file: 'environ' | 'cmdline'): Buffer {
  const bytes = readFileSync(`/proc/${pid}/${file}`);
  assert.ok(
    bytes.length > 0,
    `/proc/${pid}/${file} read as 0 bytes — the process is exiting or already a zombie; ` +
      'a capture taken now would not restore it'
  );
  return bytes;
}

/** The executable a live process is running, with procfs's " (deleted)" suffix stripped. */
export function readProcExe(pid: number): string {
  const raw = readlinkSync(`/proc/${pid}/exe`);
  return raw.endsWith(' (deleted)') ? raw.slice(0, -' (deleted)'.length) : raw;
}

export interface RestartCapture {
  /** `<workdir>/<name>.environ` — raw NUL-separated bytes, read to EOF. */
  environPath: string;
  /** `<workdir>/<name>.exe` — the binary path, one line. */
  exePath: string;
  /** The binary the process was running. */
  exe: string;
}

/**
 * Persist what `hc-mesh.sh storage-restart` needs to bring a peer back after
 * it is killed: its exact environment AND its binary path. Written BEFORE the
 * signal, while /proc still exists. Both files land where that script looks
 * (`$MESH_DIR/storage-restart/<name>.{environ,exe}`).
 */
export function writeRestartCapture(workdir: string, name: string, pid: number): RestartCapture {
  mkdirSync(workdir, { recursive: true });
  const environ = captureProcFile(pid, 'environ');
  const exe = readProcExe(pid);
  const environPath = path.join(workdir, `${name}.environ`);
  const exePath = path.join(workdir, `${name}.exe`);
  writeFileSync(environPath, environ, { mode: 0o600 });
  writeFileSync(exePath, `${exe}\n`, { mode: 0o600 });
  return { environPath, exePath, exe };
}

export interface ArgvMatch {
  /** What the caller is looking for, for error messages ("doorway on port 8888"). */
  what: string;
  /** Does this argv belong to the kind of process we want? */
  matches: (argv: readonly string[]) => boolean;
}

/**
 * Every pid (other than this test run and its parent) whose argv satisfies
 * `matches`. Never pid 0/1.
 */
export function pidsMatchingArgv(match: ArgvMatch): number[] {
  const listing = execFileSync(PS_BIN, ['-eo', 'pid=,args='], { encoding: 'utf8' });
  const pids: number[] = [];
  for (const line of listing.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    const split = trimmed.indexOf(' ');
    if (split < 0) continue;
    const pid = Number(trimmed.slice(0, split));
    const argv = trimmed.slice(split + 1).split(/\s+/);
    if (pid <= 1 || pid === process.pid || pid === process.ppid) continue;
    if (match.matches(argv)) pids.push(pid);
  }
  return pids;
}

/** The single pid matching `match`, or undefined on zero OR several candidates. */
export function findSinglePidByArgv(match: ArgvMatch): number | undefined {
  const pids = pidsMatchingArgv(match);
  return pids.length === 1 ? pids[0] : undefined;
}

/** The single pid matching `match`; asserts on zero or several (never guesses). */
export function requireSinglePidByArgv(match: ArgvMatch): number {
  const pids = pidsMatchingArgv(match);
  assert.ok(
    pids.length > 0,
    `no ${match.what} process is running — nothing to signal. This needs the mesh running as ` +
      'local processes (`just mesh start`); against a deployed fleet it is a remote pod and ' +
      'there is no PID here.'
  );
  assert.equal(
    pids.length,
    1,
    `${pids.length} ${match.what} processes found (pids ${pids.join(', ')}) — refusing to guess ` +
      'which one the mesh means'
  );
  return pids[0];
}

/** Argv match for a doorway binary listening (`--listen host:<port>`) on `port`. */
export function doorwayArgvMatch(port: string): ArgvMatch {
  return {
    what: `doorway listening on port ${port}`,
    matches: argv => {
      const isDoorway = argv[0]?.endsWith('/doorway') || argv[0] === 'doorway';
      if (!isDoorway) return false;
      const listenIdx = argv.indexOf('--listen');
      if (listenIdx < 0) return false;
      return (argv[listenIdx + 1] ?? '').endsWith(`:${port}`);
    },
  };
}

/** Argv match for an elohim-storage peer serving `--http-port <port>`. */
export function storagePeerArgvMatch(port: string): ArgvMatch {
  return {
    what: `elohim-storage peer on --http-port ${port}`,
    matches: argv =>
      argv.some(arg => arg.includes('elohim-storage')) &&
      argv.some((arg, index) => arg === '--http-port' && argv[index + 1] === port),
  };
}

/** The one doorway pid listening on `port`; asserts on zero or several. */
export function doorwayPidForPort(port: string): number {
  return requireSinglePidByArgv(doorwayArgvMatch(port));
}

/** The one storage-peer pid on `port`, or undefined (stale-fixture fallback). */
export function discoverStoragePeerPid(port: string): number | undefined {
  return findSinglePidByArgv(storagePeerArgvMatch(port));
}

async function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

/** Is `pid` still present in the process table? */
export function processAlive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

/**
 * SIGTERM, wait up to `graceMs` for the exit, then SIGKILL whatever is left.
 * Resolves once the pid is gone.
 */
export async function stopProcess(pid: number, graceMs: number): Promise<void> {
  process.kill(pid, 'SIGTERM');
  const deadline = Date.now() + graceMs;
  while (Date.now() < deadline && processAlive(pid)) await sleep(250);
  if (processAlive(pid)) {
    process.kill(pid, 'SIGKILL');
    while (processAlive(pid)) await sleep(100);
  }
}

export interface ReexecOptions {
  /** Grace between SIGTERM and SIGKILL. */
  graceMs: number;
  /** Append the replacement's stdout+stderr here (the fixture's logPath); else discard. */
  logPath?: string;
}

/**
 * Re-exec a live process as ITSELF: same argv, environment and cwd, read from
 * /proc BEFORE the stop, spawned detached afterwards. This is how a drill
 * "restarts the pod" for a local doorway — the replacement is the same
 * doorway, not a differently-configured one. Returns the replacement's pid.
 */
export async function reexecProcess(pid: number, options: ReexecOptions): Promise<number> {
  const argv = readNulSeparatedProcFile(`/proc/${pid}/cmdline`);
  const env = readProcEnvironment(pid);
  const cwd = realpathSync(`/proc/${pid}/cwd`);
  const [command, ...args] = argv;
  assert.ok(command, `could not read argv for pid ${pid}`);

  await stopProcess(pid, options.graceMs);

  const out = options.logPath ? openSync(options.logPath, 'a') : 'ignore';
  const child = spawn(command, args, { cwd, env, detached: true, stdio: ['ignore', out, out] });
  child.unref();
  assert.ok(child.pid, `re-exec of "${command}" produced no pid`);
  return child.pid;
}
