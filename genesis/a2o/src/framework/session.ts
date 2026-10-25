/** Dependency-free adapter to the native, ephemeral session identity primitive. */
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, resolve as absolute } from 'node:path';

import type { SessionIdentity } from '../../../../elohim/sdk/storage-client-ts/src/generated/session-identity.js';
export type { SessionIdentity } from '../../../../elohim/sdk/storage-client-ts/src/generated/session-identity.js';
export interface Registry {
  runtimes: { id: string; comm: string[]; sessionEnv: string[] }[];
}
type Env = Readonly<Record<string, string | undefined>>;
interface Options {
  env?: Env;
  procRoot?: string;
  startPid?: number;
  registry?: Registry;
}

export function loadRegistry(env: Env, cwd = process.cwd()): Registry {
  let path = env.EPR_AGENT_RUNTIMES?.trim();
  if (!path) {
    let dir = absolute(cwd);
    for (;;) {
      if (existsSync(join(dir, 'genesis/agentic'))) {
        path = join(dir, 'genesis/agentic/agent-runtimes.json');
        break;
      }
      const parent = dirname(dir);
      if (parent === dir) break;
      dir = parent;
    }
  }
  try {
    const registry = JSON.parse(readFileSync(path ?? '', 'utf8')) as Registry;
    if (
      !Array.isArray(registry.runtimes) ||
      registry.runtimes.some(
        r =>
          !r ||
          typeof r.id !== 'string' ||
          [r.comm, r.sessionEnv].some(
            values => !Array.isArray(values) || values.some(v => typeof v !== 'string')
          )
      )
    )
      return { runtimes: [] };
    return registry;
  } catch {
    return { runtimes: [] };
  }
}

/** @returns Parsed process fields, or null when the process is absent or malformed. */
function readProcess(
  root: string,
  pid: number
): { comm: string; parent: number; leader: number; tty: number; start: string } | null {
  if (!Number.isInteger(pid) || pid <= 0 || pid > 0xffffffff) return null;
  try {
    const stat = readFileSync(join(root, String(pid), 'stat'), 'utf8');
    const first = stat.indexOf('(');
    const last = stat.lastIndexOf(')');
    if (first < 0 || last <= first) return null;
    const fields = stat
      .slice(last + 1)
      .trim()
      .split(/\s+/);
    const raw = [fields[1], fields[3], fields[4], fields[19]];
    if (raw.some(v => !v || !/^[+-]?\d+$/.test(v))) return null;
    const [parent, leader, tty] = raw.slice(0, 3).map(Number);
    const start = BigInt(raw[3]);
    if (
      ![parent, leader].every(v => v >= 0 && v <= 0xffffffff) ||
      start < 0n ||
      start > 0xffffffffffffffffn ||
      BigInt(raw[2]) < -(2n ** 63n) ||
      BigInt(raw[2]) >= 2n ** 63n
    )
      return null;
    return { comm: stat.slice(first + 1, last), parent, leader, tty, start: String(start) };
  } catch {
    return null;
  }
}
function identity(
  runtime = 'none',
  session: string | null = null,
  owner: [number, string] | null = null,
  source: SessionIdentity['source'] = 'none'
): SessionIdentity {
  return {
    runtime,
    session,
    ownerPid: owner?.[0] ?? null,
    ownerStartTime: owner?.[1] ?? null,
    source,
  };
}

export function resolveFallback(options: Options = {}): SessionIdentity {
  const env = options.env ?? process.env;
  const root = options.procRoot ?? nonblank(env.EPR_PROC_ROOT) ?? '/proc';
  const startPid = options.startPid ?? process.pid;
  const registry = options.registry ?? loadRegistry(env);
  const declared = env.ELOHIM_SESSION_ID?.trim();
  if (declared) {
    const raw = env.ELOHIM_SESSION_OWNER_PID?.trim() ?? '';
    const pid = /^\+?\d+$/.test(raw) ? Number(raw) : 0;
    const owner = readProcess(root, pid);
    return identity('declared', declared, owner ? [pid, owner.start] : null, 'declared');
  }
  const current = readProcess(root, startPid);
  let pid = current?.parent ?? 0;
  const seen = new Set([startPid]);
  for (let hop = 0; hop < 256; hop++) {
    if (pid === 0 || seen.has(pid)) break;
    seen.add(pid);
    const ancestor = readProcess(root, pid);
    if (!ancestor) break;
    const runtime = registry.runtimes.find(r => r.comm.includes(ancestor.comm));
    if (runtime) {
      const session = ownedSessionEnv(root, pid, runtime.sessionEnv, env);
      return identity(
        runtime.id,
        session ?? `${runtime.id}-${pid}-${ancestor.start}`,
        [pid, ancestor.start],
        'runtimeAncestor'
      );
    }
    pid = ancestor.parent;
  }
  return ttyIdentity(root, current) ?? identity();
}

/**
 * @returns The first session variable this process holds that the runtime ancestor did not
 * itself inherit, or undefined when none is owned (or the ancestry is unreadable).
 */
function ownedSessionEnv(
  root: string,
  pid: number,
  names: string[],
  env: NodeJS.ProcessEnv
): string | undefined {
  let inherited: string[];
  try {
    inherited = readFileSync(join(root, String(pid), 'environ'), 'utf8').split('\0');
  } catch {
    // Unreadable ancestry cannot establish ownership of a native variable.
    return undefined;
  }
  return names
    .filter(k => !inherited.includes(`${k}=${env[k]}`))
    .map(k => env[k]?.trim())
    .find(Boolean);
}

/** @returns The TTY session leader's identity, or null when there is no readable leader. */
function ttyIdentity(
  root: string,
  current: ReturnType<typeof readProcess>
): SessionIdentity | null {
  if (!current?.tty) return null;
  const leader = readProcess(root, current.leader);
  if (!leader) return null;
  return identity(
    'tty',
    `tty-${current.leader}-${leader.start}`,
    [current.leader, leader.start],
    'ttyLeader'
  );
}

export function resolve(options: Options = {}): SessionIdentity {
  const env = options.env ?? process.env;
  if (
    options.startPid === undefined &&
    options.procRoot === undefined &&
    options.registry === undefined
  ) {
    const result = spawnSync(nonblank(env.EPR_BIN) ?? 'epr', ['session', 'id', '--json'], {
      env: { ...env, EPR_SESSION_RESOLVING: '1' },
      encoding: 'utf8',
      timeout: 2000,
    });
    if (!result.error && result.status === 0) {
      try {
        const value = JSON.parse(result.stdout) as SessionIdentity;
        if (
          value &&
          Object.keys(identity()).every(k => Object.hasOwn(value, k)) &&
          ['declared', 'runtimeAncestor', 'ttyLeader', 'none'].includes(value.source)
        )
          return value;
      } catch {
        /* A stale or malformed binary uses the same local fallback. */
      }
    }
  }
  return resolveFallback(options);
}

/** @returns Trimmed text, or undefined for absent and blank values. */
function nonblank(value: string | undefined): string | undefined {
  const trimmed = value?.trim();
  return trimmed === '' ? undefined : trimmed;
}
