#!/usr/bin/env node
/* eslint-disable sonarjs/no-duplicate-string -- stable CLI flags and mode names intentionally recur. */
/** Bounded, read-only resource observation for an already-running household mesh. */

import { execFileSync, spawn, spawnSync, type ChildProcess } from 'node:child_process';
import { createHash, randomBytes } from 'node:crypto';
import {
  accessSync,
  closeSync,
  constants,
  createReadStream,
  existsSync,
  fstatSync,
  mkdirSync,
  openSync,
  opendirSync,
  readSync,
  readlinkSync,
  readdirSync,
  realpathSync,
  statSync,
  writeFileSync,
  writeSync,
} from 'node:fs';
import { basename, dirname, extname, isAbsolute, join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

import { AdminWebsocket } from '@holochain/client';

import {
  captureHouseholdResources,
  resourceWitness,
  type ProcessIdentity,
  type ResourceSnapshot,
} from '../src/framework/fixtures/process-resources.js';

import {
  closeSqlTimingSources,
  closeWorkflowSources,
  finishSqlTimingSources,
  finishWorkflowSources,
  MAX_SQL_TIMING_SOURCES,
  openSqlTimingSources,
  openWorkflowSources,
  parseSqlTimingTarget,
  type SqlTimingSource,
  type SqlTimingTarget,
  type WorkflowSource,
  type WorkflowTarget,
} from './lib/performance-binding.js';
import { armDiagnostic } from './lib/performance-control.js';

export type ProfileMode = 'metrics' | 'cpu' | 'io' | 'cpu-io';
export type DwarfStackBytes = 8192 | 16384 | 32768;

export interface ResourceProfileOptions {
  meshDir: string;
  seconds: number;
  output: string;
  mode: ProfileMode;
  perf?: string;
  maxBytes?: number;
  maxEvents: number;
  dwarfStackBytes: DwarfStackBytes;
  callGraph?: 'dwarf' | 'fp';
  metricsUrls: MetricsTarget[];
  cohort: string | null;
  networkStats: boolean;
  sqlTiming: SqlTimingTarget[];
  armSqlTiming: SqlTimingArmTarget[];
  armWorkflow: WorkflowArmTarget[];
}

export interface SqlTimingArmTarget {
  name: string;
  directory: string;
}

export type WorkflowArmTarget = SqlTimingArmTarget;

export interface MetricsTarget {
  name: string;
  /** Safe for persistence: paths, queries, and fragments are deliberately removed. */
  url: string;
  requestUrl: string;
}

export interface MetricsCapture {
  atUnixMs: number;
  monotonicMs: number;
  text: string;
  status: number | null;
  error: string | null;
  observedProcessStartTimeSeconds: number | null;
}

export interface PerfCapability {
  executable: string;
  version: string;
  event: 'cpu-clock:u';
  frequencyHz: 49;
  callGraph: `dwarf,${DwarfStackBytes}` | 'fp';
  dwarfStackBytes: DwarfStackBytes | null;
  maxSize: 'native' | 'watchdog';
}
export interface IoPerfCapability {
  executable: string;
  version: string;
  kind: 'successful-user-syscalls';
  syscalls: readonly string[];
  callGraph: `dwarf,${DwarfStackBytes}` | 'fp';
  dwarfStackBytes: DwarfStackBytes | null;
  maxEvents: number;
}

export const MAX_PERF_BYTES_PER_CONDUCTOR = 1024 * 1024 * 1024;
export const MAX_IO_BYTES_PER_CONDUCTOR = 64 * 1024 * 1024;
export const MAX_IO_EVENTS = 100_000;
export const MAX_METRICS_ENDPOINTS = 8;
export const MAX_METRICS_BODY_BYTES = 4 * 1024 * 1024;
export const METRICS_TIMEOUT_MS = 5000;

const USAGE =
  'Usage: resource-profile --mesh-dir <path> --seconds <1..600> --output <new-dir|new.json> ' +
  '--mode metrics|cpu|io|cpu-io [--perf <absolute-perf> --max-bytes <bounded-per-conductor>] [--max-events <1..100000>] ' +
  '[--call-graph dwarf|fp] [--dwarf-stack-bytes 8192|16384|32768] ' +
  '[--metrics-url NAME=http://127.0.0.1:8090/metrics]... [--network-stats] [--cohort <label>] ' +
  '[--sql-timing NAME=/absolute/private-native-jsonl]... ' +
  '[--arm-sql-timing NAME=/absolute/startup-authorized-private-dir]... ' +
  '[--arm-workflow NAME=/absolute/startup-authorized-private-dir]...';
const CALL_GRAPH_FLAG = '--call-graph';
const ADMIN_ORIGIN = 'elohim-runtime-performance';

function metricsTarget(raw: string): MetricsTarget {
  const separator = raw.indexOf('=');
  if (separator < 1 || separator === raw.length - 1)
    throw new Error('--metrics-url expects NAME=http(s)://host/path');
  const name = raw.slice(0, separator);
  if (!/^[a-zA-Z0-9][a-zA-Z0-9_.-]*$/.test(name))
    throw new Error(`invalid metrics endpoint name: ${name}`);
  let parsed: URL;
  try {
    parsed = new URL(raw.slice(separator + 1));
  } catch {
    throw new Error(`invalid metrics URL for ${name}`);
  }
  if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:')
    throw new Error(`metrics URL for ${name} must use HTTP(S)`);
  if (parsed.username || parsed.password)
    throw new Error(`metrics URL for ${name} must not contain embedded credentials`);
  return { name, url: `${parsed.origin}/[redacted]`, requestUrl: parsed.href };
}

function value(argv: string[], index: number): string {
  const result = argv[index + 1];
  if (!result || result.startsWith('--')) throw new Error(`${argv[index]} requires a value`);
  return result;
}

function positiveInteger(raw: string, flag: string): number {
  const parsed = Number(raw);
  if (!Number.isSafeInteger(parsed) || parsed < 1)
    throw new Error(`${flag} expects a positive integer`);
  return parsed;
}

export function parseSqlTimingArmTarget(raw: string): SqlTimingArmTarget {
  return parseArmTarget(raw, '--arm-sql-timing', 'SQL timing');
}

export function parseWorkflowArmTarget(raw: string): WorkflowArmTarget {
  return parseArmTarget(raw, '--arm-workflow', 'workflow');
}

function parseArmTarget(raw: string, flag: string, label: string): SqlTimingArmTarget {
  const separator = raw.indexOf('=');
  if (separator < 1 || separator === raw.length - 1)
    throw new Error(`${flag} expects NAME=/absolute/startup-authorized-private-dir`);
  const name = raw.slice(0, separator);
  if (!/^[a-zA-Z0-9][a-zA-Z0-9_.-]*$/.test(name))
    throw new Error(`invalid ${label} arm name: ${name}`);
  const directory = raw.slice(separator + 1);
  if (!isAbsolute(directory) || resolve(directory) !== directory)
    throw new Error(`${label} arm directory for ${name} must be an absolute canonical path`);
  return { name, directory };
}

export function parseResourceProfileArgs(argv: string[]): ResourceProfileOptions {
  const out: Partial<ResourceProfileOptions> = {
    metricsUrls: [],
    cohort: null,
    dwarfStackBytes: 8192,
    callGraph: 'dwarf',
    maxEvents: 10_000,
    networkStats: false,
    sqlTiming: [],
    armSqlTiming: [],
    armWorkflow: [],
  };
  let dwarfStackBytesSpecified = false;
  let callGraphSpecified = false;
  let maxEventsSpecified = false;
  for (let i = 0; i < argv.length; i += 1) {
    const flag = argv[i];
    switch (flag) {
      case '--mesh-dir':
        out.meshDir = resolve(value(argv, i));
        i += 1;
        break;
      case '--seconds':
        out.seconds = positiveInteger(value(argv, i), flag);
        i += 1;
        break;
      case '--output':
        out.output = resolve(value(argv, i));
        i += 1;
        break;
      case '--mode': {
        const mode = value(argv, i);
        if (mode !== 'metrics' && mode !== 'cpu' && mode !== 'io' && mode !== 'cpu-io')
          throw new Error('--mode expects metrics, cpu, io, or cpu-io');
        out.mode = mode;
        i += 1;
        break;
      }
      case '--perf':
        out.perf = value(argv, i);
        i += 1;
        break;
      case '--max-bytes':
        out.maxBytes = positiveInteger(value(argv, i), flag);
        i += 1;
        break;
      case '--max-events':
        out.maxEvents = positiveInteger(value(argv, i), flag);
        maxEventsSpecified = true;
        i += 1;
        break;
      case CALL_GRAPH_FLAG: {
        if (callGraphSpecified) throw new Error(`${CALL_GRAPH_FLAG} may be specified only once`);
        const mode = value(argv, i);
        if (mode !== 'dwarf' && mode !== 'fp')
          throw new Error(`${CALL_GRAPH_FLAG} expects dwarf or fp`);
        out.callGraph = mode;
        callGraphSpecified = true;
        i += 1;
        break;
      }
      case '--dwarf-stack-bytes': {
        const parsed = positiveInteger(value(argv, i), flag);
        if (parsed !== 8192 && parsed !== 16384 && parsed !== 32768)
          throw new Error('--dwarf-stack-bytes expects 8192, 16384, or 32768');
        out.dwarfStackBytes = parsed;
        dwarfStackBytesSpecified = true;
        i += 1;
        break;
      }
      case '--metrics-url':
        out.metricsUrls!.push(metricsTarget(value(argv, i)));
        i += 1;
        break;
      case '--sql-timing':
        out.sqlTiming!.push(parseSqlTimingTarget(value(argv, i)));
        i += 1;
        break;
      case '--arm-sql-timing':
        out.armSqlTiming!.push(parseSqlTimingArmTarget(value(argv, i)));
        i += 1;
        break;
      case '--arm-workflow':
        out.armWorkflow!.push(parseWorkflowArmTarget(value(argv, i)));
        i += 1;
        break;
      case '--cohort':
        if (out.cohort !== null) throw new Error('--cohort may be specified only once');
        out.cohort = value(argv, i);
        i += 1;
        break;
      case '--network-stats':
        out.networkStats = true;
        break;
      default:
        throw new Error(`Unknown flag: ${flag}\n${USAGE}`);
    }
  }
  if (!out.meshDir || !out.seconds || !out.output || !out.mode) throw new Error(USAGE);
  if (out.seconds > 600) throw new Error('--seconds must be in 1..600');
  if (out.mode !== 'metrics' && !out.perf) throw new Error(`--mode ${out.mode} requires --perf`);
  if (out.mode !== 'metrics' && !out.maxBytes)
    throw new Error(`--mode ${out.mode} requires --max-bytes per conductor`);
  if (out.maxBytes && out.maxBytes > MAX_PERF_BYTES_PER_CONDUCTOR)
    throw new Error(`--max-bytes must be at most ${MAX_PERF_BYTES_PER_CONDUCTOR} per conductor`);
  if (out.mode === 'io' && out.maxBytes! > MAX_IO_BYTES_PER_CONDUCTOR)
    throw new Error(`--mode io --max-bytes must be at most ${MAX_IO_BYTES_PER_CONDUCTOR}`);
  if (out.maxEvents! > MAX_IO_EVENTS)
    throw new Error(`--max-events must be at most ${MAX_IO_EVENTS}`);
  if (out.mode !== 'io' && out.mode !== 'cpu-io' && maxEventsSpecified)
    throw new Error('--max-events is only valid with --mode io');
  if ((out.mode === 'io' || out.mode === 'cpu-io') && out.seconds > 60)
    throw new Error('--mode io and cpu-io --seconds must be in 1..60');
  if (out.mode === 'metrics' && out.perf)
    throw new Error('--perf is only valid with --mode cpu or io');
  if (out.mode === 'metrics' && out.maxBytes)
    throw new Error('--max-bytes is only valid with --mode cpu or io');
  if (out.mode === 'metrics' && dwarfStackBytesSpecified)
    throw new Error('--dwarf-stack-bytes is only valid with --mode cpu or io');
  if (out.mode === 'metrics' && callGraphSpecified)
    throw new Error(`${CALL_GRAPH_FLAG} is only valid with --mode cpu or io`);
  if (out.callGraph === 'fp' && dwarfStackBytesSpecified)
    throw new Error('--dwarf-stack-bytes cannot be combined with --call-graph fp');
  if (out.metricsUrls!.length > MAX_METRICS_ENDPOINTS)
    throw new Error(`at most ${MAX_METRICS_ENDPOINTS} --metrics-url values are allowed`);
  const names = out.metricsUrls!.map(target => target.name);
  if (new Set(names).size !== names.length) throw new Error('--metrics-url names must be unique');
  if (out.sqlTiming!.length > MAX_SQL_TIMING_SOURCES)
    throw new Error(`at most ${MAX_SQL_TIMING_SOURCES} --sql-timing values are allowed`);
  if (out.armSqlTiming!.length > MAX_SQL_TIMING_SOURCES)
    throw new Error(`at most ${MAX_SQL_TIMING_SOURCES} --arm-sql-timing values are allowed`);
  if (out.armWorkflow!.length > MAX_SQL_TIMING_SOURCES)
    throw new Error(`at most ${MAX_SQL_TIMING_SOURCES} --arm-workflow values are allowed`);
  const sqlNames = out.sqlTiming!.map(target => target.name);
  if (new Set(sqlNames).size !== sqlNames.length)
    throw new Error('--sql-timing names must be unique');
  const armSqlNames = out.armSqlTiming!.map(target => target.name);
  if (new Set(armSqlNames).size !== armSqlNames.length)
    throw new Error('--arm-sql-timing names must be unique');
  if (armSqlNames.some(name => sqlNames.includes(name)))
    throw new Error('--sql-timing and --arm-sql-timing are mutually exclusive for the same peer');
  const armWorkflowNames = out.armWorkflow!.map(target => target.name);
  if (new Set(armWorkflowNames).size !== armWorkflowNames.length)
    throw new Error('--arm-workflow names must be unique');
  return out as ResourceProfileOptions;
}

const NETWORK_MAX_PAYLOAD = 4 * 1024 * 1024;
const NETWORK_TIMEOUT_MS = 5000;
const MAX_PROC_NET_BYTES = 2 * 1024 * 1024;
const MAX_PROCESS_FDS = 4096;

interface NetworkConnection {
  membership: string;
  sendMessageCount: number;
  sendBytes: number;
  recvMessageCount: number;
  recvBytes: number;
  direct: boolean;
}
interface NetworkPeerSnapshot {
  peer: string;
  backend: string;
  connections: NetworkConnection[];
  blockedIncoming: number;
  blockedOutgoing: number;
  identity: ProcessIdentity;
  valid: boolean;
  error: string | null;
  atUnixMs: number;
  monotonicMs: number;
}
interface NetworkRound {
  atUnixMs: number;
  monotonicMs: number;
  peers: Record<string, NetworkPeerSnapshot>;
  issues: string[];
}

/** Node-only contract: AdminWebsocket forwards this object unchanged to isomorphic-ws/ws. */
export function adminNetworkOptions(url: URL): Parameters<typeof AdminWebsocket.connect>[0] {
  if (typeof process === 'undefined' || !process.versions?.node)
    throw new Error('network stats require the Node.js websocket implementation');
  return {
    url,
    defaultTimeout: NETWORK_TIMEOUT_MS,
    // @holochain/client 0.21 forwards this object unchanged to Node ws. The pinned conductor
    // expects an Origin header during its websocket handshake, including with allowed_origins `*`.
    // The integration test also pins the version-bound pre-decode byte ceiling.
    wsClientOptions: {
      origin: ADMIN_ORIGIN,
      maxPayload: NETWORK_MAX_PAYLOAD,
      handshakeTimeout: NETWORK_TIMEOUT_MS,
    } as unknown as { origin?: string },
  };
}

function boundedText(path: string, maxBytes: number): string {
  const inspected = statSync(path, { bigint: true });
  if (!inspected.isFile() || inspected.size > BigInt(maxBytes))
    throw new Error('bounded local file invalid');
  const fd = openSync(path, constants.O_RDONLY | constants.O_NONBLOCK | constants.O_NOFOLLOW);
  try {
    const opened = fstatSync(fd, { bigint: true });
    if (
      opened.dev !== inspected.dev ||
      opened.ino !== inspected.ino ||
      opened.size !== inspected.size
    )
      throw new Error('bounded local file identity changed');
    const chunks: Buffer[] = [];
    let total = 0;
    for (;;) {
      const bytes = Buffer.alloc(Math.min(64 * 1024, maxBytes + 1 - total));
      const count = readSync(fd, bytes, 0, bytes.length, null);
      if (!count) break;
      total += count;
      if (total > maxBytes) throw new Error('bounded local file exceeds byte ceiling');
      chunks.push(bytes.subarray(0, count));
    }
    const completed = fstatSync(fd, { bigint: true });
    if (
      opened.size > 0n &&
      (BigInt(total) !== opened.size ||
        completed.size !== opened.size ||
        completed.mtimeNs !== opened.mtimeNs)
    )
      throw new Error('bounded local file changed while reading');
    return Buffer.concat(chunks, total).toString('utf8');
  } finally {
    closeSync(fd);
  }
}

export async function closeAdminBounded(
  admin: Awaited<ReturnType<typeof AdminWebsocket.connect>>,
  timeoutMs = 1000
): Promise<void> {
  let timer: NodeJS.Timeout | undefined;
  try {
    await Promise.race([
      admin.client.close().catch(() => undefined),
      new Promise<void>(resolveClose => {
        timer = setTimeout(() => {
          const socket = (admin.client as unknown as { socket?: { terminate?(): void } }).socket;
          socket?.terminate?.();
          resolveClose();
        }, timeoutMs);
      }),
    ]);
  } finally {
    if (timer) clearTimeout(timer);
  }
}

export async function connectAdminBounded(
  url: URL,
  timeoutMs = NETWORK_TIMEOUT_MS
): Promise<Awaited<ReturnType<typeof AdminWebsocket.connect>>> {
  const pending = AdminWebsocket.connect(adminNetworkOptions(url));
  let timer: NodeJS.Timeout | undefined;
  try {
    return await Promise.race([
      pending,
      new Promise<never>((_resolve, reject) => {
        timer = setTimeout(() => reject(new Error('admin connect timeout')), timeoutMs);
      }),
    ]);
  } catch (error) {
    void pending.then(closeAdminBounded, () => undefined);
    throw error;
  } finally {
    if (timer) clearTimeout(timer);
  }
}

export function listenerOwnedByPid(pid: number, port: number): boolean {
  const inodes = new Set<string>();
  for (const name of ['tcp', 'tcp6']) {
    const path = `/proc/net/${name}`;
    const text = boundedText(path, MAX_PROC_NET_BYTES);
    for (const line of text.split('\n').slice(1)) {
      const fields = line.trim().split(/\s+/);
      const local = fields[1]?.split(':');
      if (
        local?.length === 2 &&
        Number.parseInt(local[1], 16) === port &&
        fields[3] === '0A' &&
        fields[9]
      )
        inodes.add(fields[9]);
    }
  }
  const fdDir = `/proc/${pid}/fd`;
  const directory = opendirSync(fdDir);
  try {
    for (let inspected = 0; inspected <= MAX_PROCESS_FDS; inspected += 1) {
      const entry = directory.readSync();
      if (!entry) return false;
      if (inspected === MAX_PROCESS_FDS) throw new Error(`pid ${pid} exceeds fd inspection bound`);
      try {
        const match = /^socket:\[(\d+)]$/.exec(readlinkSync(join(fdDir, entry.name)));
        if (match && inodes.has(match[1])) return true;
      } catch {
        // A descriptor can disappear between directory iteration and readlink.
      }
    }
    return false;
  } finally {
    directory.closeSync();
  }
}

export function networkPorts(
  meshDir: string,
  configs: Record<string, string>
): Record<string, number> {
  const fixture = JSON.parse(boundedText(join(meshDir, 'household-fixture.json'), 1024 * 1024)) as {
    storagePeers?: Record<string, { conductorAdminPort?: unknown }>;
  };
  const output: Record<string, number> = {};
  for (const [peer, config] of Object.entries(configs)) {
    const port = fixture.storagePeers?.[peer]?.conductorAdminPort;
    if (!Number.isSafeInteger(port) || Number(port) < 1 || Number(port) > 65535)
      throw new Error(`${peer}: fixture admin port invalid`);
    const yaml = boundedText(config, 1024 * 1024);
    const configured = /admin_interfaces:[\s\S]{0,2048}?port:\s*(\d+)/.exec(yaml)?.[1];
    if (Number(configured) !== port) throw new Error(`${peer}: fixture/YAML admin port mismatch`);
    output[peer] = Number(port);
  }
  return output;
}

type AdminConnection = Awaited<ReturnType<typeof AdminWebsocket.connect>>;
type SqlTimingAdmission = Awaited<ReturnType<typeof armDiagnostic>>;

export interface SqlTimingArmDependencies {
  capture(configs: Record<string, string>): ResourceSnapshot;
  listenerOwned(pid: number, port: number): boolean;
  connect(url: URL): Promise<AdminConnection>;
  close(admin: AdminConnection): Promise<void>;
  admit(admin: AdminConnection, nonce: string, seconds: number): Promise<SqlTimingAdmission>;
  open(targets: SqlTimingTarget[], configs: Record<string, string>): { sources: SqlTimingSource[] };
  nonce(): string;
}

export interface WorkflowArmDependencies {
  capture(configs: Record<string, string>): ResourceSnapshot;
  listenerOwned(pid: number, port: number): boolean;
  connect(url: URL): Promise<AdminConnection>;
  close(admin: AdminConnection): Promise<void>;
  admit(admin: AdminConnection, nonce: string, seconds: number): Promise<SqlTimingAdmission>;
  open(targets: WorkflowTarget[], configs: Record<string, string>): { sources: WorkflowSource[] };
  nonce(): string;
}

const defaultSqlTimingArmDependencies: SqlTimingArmDependencies = {
  capture: captureHouseholdResources,
  listenerOwned: listenerOwnedByPid,
  connect: connectAdminBounded,
  close: closeAdminBounded,
  admit: async (admin, nonce, seconds) =>
    await armDiagnostic(
      async (operation, payload, timeout) => admin._requester(operation)(payload, timeout),
      'sqlTiming',
      nonce,
      seconds
    ),
  open: (targets, configs) => openSqlTimingSources(targets, configs),
  nonce: () => `rp_${randomBytes(16).toString('hex')}`,
};

const defaultWorkflowArmDependencies: WorkflowArmDependencies = {
  capture: captureHouseholdResources,
  listenerOwned: listenerOwnedByPid,
  connect: connectAdminBounded,
  close: closeAdminBounded,
  admit: async (admin, nonce, seconds) =>
    await armDiagnostic(
      async (operation, payload, timeout) => admin._requester(operation)(payload, timeout),
      'workflow',
      nonce,
      seconds
    ),
  open: (targets, configs) => openWorkflowSources(targets, configs),
  nonce: () => `rp_${randomBytes(16).toString('hex')}`,
};

function validateArmDirectory(target: SqlTimingArmTarget, label: string): void {
  let canonical: string;
  try {
    canonical = realpathSync(target.directory);
    const inspected = statSync(canonical, { bigint: true });
    const uid = process.getuid?.();
    if (
      canonical !== target.directory ||
      !inspected.isDirectory() ||
      uid === undefined ||
      inspected.uid !== BigInt(uid) ||
      Number(inspected.mode & 0o777n) !== 0o700
    )
      throw new Error('invalid private directory');
  } catch {
    throw new Error(`${target.name}: ${label} arm directory is not canonical private mode 0700`);
  }
}

function requireArmInstance(
  expected: ResourceSnapshot,
  observed: ResourceSnapshot,
  peer: string,
  label: string
): ProcessIdentity {
  const identity = expected.samples[peer];
  if (
    !identity ||
    !expected.bootId ||
    expected.bootId !== observed.bootId ||
    !processIdentityMatches(identity, observed.samples[peer])
  )
    throw new Error(`${peer}: conductor identity changed during ${label} admission`);
  return identity;
}

/**
 * Arms explicit startup-authorized SQL timing peers exactly once and pins their returned files.
 *
 * The native window is the requested measurement plus a two-second bookkeeping margin. A
 * transport failure has an unknown native outcome and is never retried. If any peer fails after
 * another was admitted, all opened descriptors are closed and the bounded native window is left
 * to expire; this collector makes no rollback or cancellation claim.
 */
export async function armSqlTimingSources(
  targets: SqlTimingArmTarget[],
  seconds: number,
  configs: Record<string, string>,
  ports: Record<string, number>,
  preArm: ResourceSnapshot,
  dependencies: SqlTimingArmDependencies = defaultSqlTimingArmDependencies
): Promise<SqlTimingSource[]> {
  return await armPrivateDiagnosticSources(
    targets,
    seconds,
    configs,
    ports,
    preArm,
    { family: 'sqlTiming', label: 'SQL timing' },
    {
      ...dependencies,
      open: (target, admission) => dependencies.open([{ ...target, admission }], configs).sources,
      closeSources: closeSqlTimingSources,
    }
  );
}

export async function armWorkflowSources(
  targets: WorkflowArmTarget[],
  seconds: number,
  configs: Record<string, string>,
  ports: Record<string, number>,
  preArm: ResourceSnapshot,
  dependencies: WorkflowArmDependencies = defaultWorkflowArmDependencies
): Promise<WorkflowSource[]> {
  return await armPrivateDiagnosticSources(
    targets,
    seconds,
    configs,
    ports,
    preArm,
    { family: 'workflow', label: 'workflow' },
    {
      ...dependencies,
      open: (target, admission) => dependencies.open([{ ...target, admission }], configs).sources,
      closeSources: closeWorkflowSources,
    }
  );
}

interface PrivateDiagnosticArmDependencies<S> {
  capture(configs: Record<string, string>): ResourceSnapshot;
  listenerOwned(pid: number, port: number): boolean;
  connect(url: URL): Promise<AdminConnection>;
  close(admin: AdminConnection): Promise<void>;
  admit(admin: AdminConnection, nonce: string, seconds: number): Promise<SqlTimingAdmission>;
  open(target: SqlTimingTarget, admission: WorkflowTarget['admission']): S[];
  closeSources(sources: S[]): void;
  nonce(): string;
}

async function armPrivateDiagnosticSources<S>(
  targets: SqlTimingArmTarget[],
  seconds: number,
  configs: Record<string, string>,
  ports: Record<string, number>,
  preArm: ResourceSnapshot,
  control: { family: 'sqlTiming' | 'workflow'; label: string },
  dependencies: PrivateDiagnosticArmDependencies<S>
): Promise<S[]> {
  const { family, label } = control;
  if (!targets.length) return [];
  if (targets.length > MAX_SQL_TIMING_SOURCES)
    throw new Error(`at most ${MAX_SQL_TIMING_SOURCES} ${label} peers may be armed`);
  const armSeconds = seconds + 2;
  if (!Number.isSafeInteger(armSeconds) || armSeconds < 3 || armSeconds > 602)
    throw new Error(`${label} arm window must be in 3..602 seconds`);

  for (const target of targets) {
    if (!Object.hasOwn(configs, target.name) || !Object.hasOwn(ports, target.name))
      throw new Error(`${target.name}: ${label} arm target does not match a conductor config`);
    validateArmDirectory(target, label);
    const identity = requireArmInstance(preArm, dependencies.capture(configs), target.name, label);
    if (!dependencies.listenerOwned(identity.pid, ports[target.name]))
      throw new Error(`${target.name}: admin listener is not owned by expected conductor PID`);
  }

  const attempts = await Promise.allSettled(
    targets.map(async target => {
      const expected = preArm.samples[target.name];
      const port = ports[target.name];
      let admin: AdminConnection | undefined;
      let sources: S[] = [];
      try {
        admin = await dependencies.connect(new URL(`ws://127.0.0.1:${port}`));
        // Close the check/connect replacement gap before sending the one-shot request.
        const immediatelyBefore = dependencies.capture(configs);
        requireArmInstance(preArm, immediatelyBefore, target.name, label);
        if (!dependencies.listenerOwned(expected.pid, port))
          throw new Error(`admin listener ownership changed before ${label} admission`);

        const admission = await dependencies.admit(admin, dependencies.nonce(), armSeconds);
        if (!admission.admitted || admission.family !== family)
          throw new Error(`${label} admission refused or mismatched: ${admission.outcome}`);
        const sourcePath = join(target.directory, admission.outputBasename);
        if (dirname(sourcePath) !== target.directory)
          throw new Error(`${label} admission returned an invalid artifact basename`);
        sources = dependencies.open(
          { name: target.name, sourcePath },
          {
            nonce: admission.nonce,
            producerId: admission.producerId,
            generation: admission.generation,
            outputBasename: admission.outputBasename,
            requestedSeconds: admission.requestedSeconds,
          }
        );

        const afterOpen = dependencies.capture(configs);
        requireArmInstance(preArm, afterOpen, target.name, label);
        if (!dependencies.listenerOwned(expected.pid, port))
          throw new Error(`admin listener ownership changed after ${label} admission`);
        const closing = admin;
        admin = undefined;
        await dependencies.close(closing);
        return sources;
      } catch (error) {
        dependencies.closeSources(sources);
        throw new Error(`${target.name}: ${label} auto-rearm failed`, { cause: error });
      } finally {
        if (admin) await dependencies.close(admin).catch(() => undefined);
      }
    })
  );

  const sources = attempts.flatMap(result => (result.status === 'fulfilled' ? result.value : []));
  const failedPeers = attempts.flatMap((result, index) =>
    result.status === 'rejected' ? [targets[index].name] : []
  );
  if (failedPeers.length) {
    dependencies.closeSources(sources);
    throw new Error(
      `${label} auto-rearm failed for ${failedPeers.join(', ')}; admitted windows will expire without cancellation`
    );
  }
  return sources;
}

function safeCounter(value: unknown, label: string): number {
  if (!Number.isSafeInteger(value) || Number(value) < 0) throw new Error(`${label} invalid`);
  return Number(value);
}
function safeCounterSum(left: number, right: number, label: string): number {
  const value = left + right;
  if (!Number.isSafeInteger(value)) throw new Error(`${label} aggregate overflow`);
  return value;
}

async function captureNetworkRound(
  configs: Record<string, string>,
  ports: Record<string, number>,
  expected: Record<string, ProcessIdentity>,
  membershipSalt: Buffer
): Promise<NetworkRound> {
  const pre = captureHouseholdResources(configs);
  const rows = await Promise.all(
    Object.keys(configs).map(async peer => {
      const identity = expected[peer];
      const fail = (error: unknown): NetworkPeerSnapshot => ({
        peer,
        backend: '',
        connections: [],
        blockedIncoming: 0,
        blockedOutgoing: 0,
        identity,
        valid: false,
        error: error instanceof Error ? error.name : 'unknown error',
        atUnixMs: Date.now(),
        monotonicMs: performance.now(),
      });
      if (!identity || !processIdentityMatches(identity, pre.samples[peer]))
        return fail('identity mismatch before admin request');
      if (!listenerOwnedByPid(identity.pid, ports[peer]))
        return fail('admin listener is not owned by expected conductor PID');
      let admin: Awaited<ReturnType<typeof AdminWebsocket.connect>> | undefined;
      try {
        admin = await connectAdminBounded(new URL(`ws://127.0.0.1:${ports[peer]}`));
        const raw = await admin.dumpNetworkStats(undefined, NETWORK_TIMEOUT_MS);
        // The pinned conductor fork's transport implementation emits this exact literal
        // (patches/kitsune2_transport_iroh/src/lib.rs). Do not guess from a fixture label.
        if (raw.transport_stats.backend !== 'iroh') throw new Error('unsupported network backend');
        if (raw.transport_stats.connections.length > 10_000)
          throw new Error('network connection count exceeds bound');
        const connections = raw.transport_stats.connections.map(connection => ({
          membership: createHash('sha256')
            .update(membershipSalt)
            .update(`${connection.pub_key}\0${connection.opened_at_s}`)
            .digest('hex'),
          sendMessageCount: safeCounter(connection.send_message_count, 'send_message_count'),
          sendBytes: safeCounter(connection.send_bytes, 'send_bytes'),
          recvMessageCount: safeCounter(connection.recv_message_count, 'recv_message_count'),
          recvBytes: safeCounter(connection.recv_bytes, 'recv_bytes'),
          direct: connection.is_direct === true,
        }));
        if (
          new Set(connections.map(connection => connection.membership)).size !== connections.length
        )
          throw new Error('duplicate network connection identity');
        let blockedIncoming = 0;
        let blockedOutgoing = 0;
        let blockedSeries = 0;
        for (const spaces of Object.values(raw.blocked_message_counts))
          for (const count of Object.values(spaces)) {
            blockedSeries += 1;
            if (blockedSeries > 10_000) throw new Error('blocked-message series exceed bound');
            blockedIncoming = safeCounterSum(
              blockedIncoming,
              safeCounter(count.incoming, 'blocked incoming'),
              'blocked incoming'
            );
            blockedOutgoing = safeCounterSum(
              blockedOutgoing,
              safeCounter(count.outgoing, 'blocked outgoing'),
              'blocked outgoing'
            );
          }
        return {
          peer,
          backend: String(raw.transport_stats.backend),
          connections,
          blockedIncoming,
          blockedOutgoing,
          identity,
          valid: true,
          error: null,
          atUnixMs: Date.now(),
          monotonicMs: performance.now(),
        };
      } catch (error) {
        return fail(error);
      } finally {
        if (admin) await closeAdminBounded(admin);
      }
    })
  );
  const post = captureHouseholdResources(configs);
  const peers = Object.fromEntries(
    rows.map(row => {
      if (!processIdentityMatches(row.identity, post.samples[row.peer])) {
        row.valid = false;
        row.error = 'identity mismatch after admin request';
      }
      return [row.peer, row];
    })
  );
  const issues = [
    ...pre.issues,
    ...post.issues,
    ...rows.filter(row => !row.valid).map(row => `${row.peer}: ${row.error}`),
  ];
  return { atUnixMs: Date.now(), monotonicMs: performance.now(), peers, issues };
}

function processStart(text: string): number | null {
  const decimal = String.raw`[-+]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][-+]?\d+)?`;
  const match = new RegExp(
    String.raw`^process_start_time_seconds(?:\{[^}]*\})?\s+(${decimal})(?:\s+-?\d+)?\s*$`,
    'm'
  ).exec(text);
  if (!match) return null;
  const observed = Number(match[1]);
  return Number.isFinite(observed) ? observed : null;
}

async function readWithAbort(
  reader: ReadableStreamDefaultReader<Uint8Array>,
  signal: AbortSignal
): Promise<ReadableStreamReadResult<Uint8Array>> {
  if (signal.aborted) throw signal.reason;
  return await new Promise((accept, reject) => {
    const abort = (): void => reject(signal.reason);
    signal.addEventListener('abort', abort, { once: true });
    void reader.read().then(
      value => {
        signal.removeEventListener('abort', abort);
        accept(value);
      },
      caught => {
        signal.removeEventListener('abort', abort);
        reject(caught);
      }
    );
  });
}

export async function captureMetricsEndpoint(
  target: MetricsTarget,
  fetcher: typeof fetch = fetch,
  timeoutMs = METRICS_TIMEOUT_MS
): Promise<MetricsCapture> {
  const atUnixMs = Date.now();
  const monotonicMs = performance.now();
  let status: number | null = null;
  let text = '';
  let error: string | null = null;
  let reader: ReadableStreamDefaultReader<Uint8Array> | null = null;
  const signal = AbortSignal.timeout(timeoutMs);
  try {
    const response = await fetcher(target.requestUrl, {
      signal,
      redirect: 'manual',
      headers: { accept: 'text/plain' },
    });
    status = response.status;
    if (!response.ok) {
      await response.body?.cancel();
      throw new Error(`HTTP ${response.status}`);
    }
    const declared = Number(response.headers.get('content-length'));
    if (Number.isFinite(declared) && declared > MAX_METRICS_BODY_BYTES) {
      await response.body?.cancel();
      throw new Error(`body exceeds ${MAX_METRICS_BODY_BYTES} bytes`);
    }
    if (!response.body) throw new Error('response has no body');
    reader = response.body.getReader();
    const decoder = new TextDecoder();
    let bytes = 0;
    while (true) {
      const chunk = await readWithAbort(reader, signal);
      if (chunk.done) break;
      bytes += chunk.value.byteLength;
      if (bytes > MAX_METRICS_BODY_BYTES) {
        await reader.cancel();
        throw new Error(`body exceeds ${MAX_METRICS_BODY_BYTES} bytes`);
      }
      text += decoder.decode(chunk.value, { stream: true });
    }
    text += decoder.decode();
  } catch (caught) {
    if (reader) {
      try {
        await reader.cancel();
      } catch {
        // Preserve the primary collection error; cancellation is best-effort cleanup.
      }
    }
    const caughtName = caught instanceof Error ? caught.name : 'Error';
    const safeKnownMessage =
      caught instanceof Error &&
      /^(?:HTTP \d{3}|body exceeds \d+ bytes|response has no body)$/.test(caught.message)
        ? caught.message
        : null;
    error = safeKnownMessage ?? `request failed (${caughtName})`;
  }
  return {
    atUnixMs,
    monotonicMs,
    text,
    status,
    error,
    observedProcessStartTimeSeconds: processStart(text),
  };
}

export function validatePerf(
  executable: string,
  dwarfStackBytes: DwarfStackBytes = 8192,
  callGraph: 'dwarf' | 'fp' = 'dwarf'
): PerfCapability {
  if (!isAbsolute(executable)) throw new Error('--perf must be an absolute path');
  accessSync(executable, constants.X_OK);
  const resolved = realpathSync(executable);
  if (basename(resolved) !== 'perf')
    throw new Error(`--perf does not resolve to a perf executable: ${resolved}`);
  const version = execFileSync(resolved, ['--version'], { encoding: 'utf8', timeout: 5000 }).trim();
  const helpResult = spawnSync(resolved, ['record', '-h'], {
    encoding: 'utf8',
    timeout: 5000,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  if (helpResult.error) throw helpResult.error;
  const help = `${helpResult.stdout ?? ''}\n${helpResult.stderr ?? ''}`;
  return {
    executable: resolved,
    version,
    event: 'cpu-clock:u',
    frequencyHz: 49,
    callGraph: callGraph === 'fp' ? 'fp' : `dwarf,${dwarfStackBytes}`,
    dwarfStackBytes: callGraph === 'fp' ? null : dwarfStackBytes,
    maxSize: help.includes('--max-size') ? 'native' : 'watchdog',
  };
}

interface OutputLayout {
  json: string;
  artifacts: string;
}

function reserveOutput(path: string): OutputLayout {
  if (existsSync(path)) throw new Error(`--output already exists: ${path}`);
  if (extname(path) === '.json') {
    accessSync(dirname(path), constants.W_OK);
    const fd = openSync(path, 'wx');
    // The reserved empty file makes concurrent reuse fail before observation starts.
    writeFileSync(fd, '');
    closeSync(fd);
    const artifacts = `${path}.artifacts`;
    mkdirSync(artifacts);
    return { json: path, artifacts };
  }
  mkdirSync(path);
  return { json: join(path, 'resource-profile.json'), artifacts: path };
}

export function expectedConductorConfigs(meshDir: string): Record<string, string> {
  const conductors = join(meshDir, 'conductors');
  const expected: Record<string, string> = {};
  for (const peer of readdirSync(conductors, { withFileTypes: true })) {
    if (!peer.isDirectory()) continue;
    const config = join(conductors, peer.name, 'conductor-config.yaml');
    if (existsSync(config)) expected[peer.name] = realpathSync(config);
  }
  if (!Object.keys(expected).length) throw new Error(`No conductor configs under ${conductors}`);
  return expected;
}

interface PerfResult {
  peer: string;
  identity: ProcessIdentity;
  artifact: string;
  exitCode: number | null;
  signal: NodeJS.Signals | null;
  bytes: number | null;
  truncated: boolean;
  attached: boolean;
  identityValid: boolean;
  stderr: string;
  errors: string[];
  profileKind: 'cpu' | 'io';
  maxEvents?: number;
  stopCause?: IoTraceStopCause;
}

export type IoTraceStopCause = 'none' | 'window' | 'byte-cap' | 'artifact-write-error';

interface BinaryFingerprint {
  peer: string;
  pid: number;
  executable: string;
  device: string | null;
  inode: string | null;
  sha256: string | null;
  identityValid: boolean;
  error: string | null;
  purpose: 'verification-fingerprint';
}

type KillProcessGroup = (pid: number, signal: NodeJS.Signals) => void;
type ScheduleForce = (callback: () => void, delayMs: number) => void;

function killProcessGroup(pid: number, signal: NodeJS.Signals): void {
  try {
    process.kill(-pid, signal);
  } catch (error) {
    const code = (error as NodeJS.ErrnoException).code;
    if (code !== 'ESRCH') throw error;
  }
}

/** Tracks only process groups created by this collector and bounds shutdown cleanup. */
export class PerfChildGuard {
  private readonly pids = new Set<number>();

  public constructor(
    private readonly killGroup: KillProcessGroup = killProcessGroup,
    private readonly scheduleForce: ScheduleForce = (callback, delayMs) => {
      setTimeout(callback, delayMs);
    }
  ) {}

  public track(child: { pid?: number; once(event: 'close', listener: () => void): unknown }): void {
    if (!child.pid) throw new Error('spawned perf has no pid');
    const pid = child.pid;
    this.pids.add(pid);
    child.once('close', () => this.pids.delete(pid));
  }

  public signal(pid: number, signal: NodeJS.Signals): void {
    if (!this.pids.has(pid)) return;
    try {
      this.killGroup(pid, signal);
    } catch {
      // The process group may have exited between membership check and signal.
      this.pids.delete(pid);
    }
  }

  public terminateAll(forceAfterMs = 2000): void {
    for (const pid of this.pids) this.signal(pid, 'SIGINT');
    this.scheduleForce(() => {
      for (const pid of this.pids) this.signal(pid, 'SIGKILL');
    }, forceAfterMs);
  }
}

const activePerfChildren = new PerfChildGuard();

export function processIdentityMatches(
  expected: ProcessIdentity,
  observed: ProcessIdentity | undefined
): boolean {
  return Boolean(
    expected.pid === observed?.pid &&
    expected.startTicks === observed?.startTicks &&
    expected.executable === observed?.executable &&
    expected.configPath === observed?.configPath
  );
}

async function fingerprintBinaries(
  identities: ProcessIdentity[]
): Promise<Record<string, BinaryFingerprint>> {
  const cache = new Map<string, Promise<string>>();
  const output: Record<string, BinaryFingerprint> = {};
  await Promise.all(
    identities.map(async identity => {
      const procExe = `/proc/${identity.pid}/exe`;
      try {
        const stat = statSync(procExe, { bigint: true });
        const device = stat.dev.toString();
        const inode = stat.ino.toString();
        const cacheKey = `${device}:${inode}`;
        let digest = cache.get(cacheKey);
        if (!digest) {
          digest = (async () => {
            const hash = createHash('sha256');
            for await (const chunk of createReadStream(procExe)) hash.update(chunk as Buffer);
            return hash.digest('hex');
          })();
          cache.set(cacheKey, digest);
        }
        output[identity.peer] = {
          peer: identity.peer,
          pid: identity.pid,
          executable: identity.executable,
          device,
          inode,
          sha256: await digest,
          identityValid: true,
          error: null,
          purpose: 'verification-fingerprint',
        };
      } catch (error) {
        output[identity.peer] = {
          peer: identity.peer,
          pid: identity.pid,
          executable: identity.executable,
          device: null,
          inode: null,
          sha256: null,
          identityValid: false,
          error: String(error),
          purpose: 'verification-fingerprint',
        };
      }
    })
  );
  return output;
}

function unattachedResult(
  identity: ProcessIdentity,
  artifact: string,
  error: string,
  profileKind: 'cpu' | 'io' = 'cpu'
): PerfResult {
  return {
    peer: identity.peer,
    identity,
    artifact,
    exitCode: null,
    signal: null,
    bytes: null,
    truncated: false,
    attached: false,
    identityValid: false,
    stderr: '',
    errors: [error],
    profileKind,
  };
}

async function waitFor(
  child: ChildProcess
): Promise<{ code: number | null; signal: NodeJS.Signals | null }> {
  return new Promise((accept, reject) => {
    child.once('error', reject);
    child.once('close', (code, signal) => accept({ code, signal }));
  });
}

export function buildPerfRecordArgs(
  capability: PerfCapability,
  pid: number,
  seconds: number,
  artifact: string,
  maxBytes?: number
): string[] {
  const args = [
    'record',
    '-e',
    capability.event,
    '-F',
    String(capability.frequencyHz),
    '--call-graph',
    capability.callGraph,
    '-p',
    String(pid),
    '--output',
    artifact,
  ];
  if (maxBytes && capability.maxSize === 'native') args.push('--max-size', `${maxBytes}B`);
  args.push('--', '/usr/bin/sleep', String(seconds));
  return args;
}

const IO_SYSCALLS = [
  'read',
  'write',
  'pread64',
  'pwrite64',
  'readv',
  'writev',
  'fsync',
  'fdatasync',
] as const;

export function buildPerfTraceArgs(capability: IoPerfCapability, pid: number): string[] {
  return [
    'trace',
    '--max-events',
    String(capability.maxEvents),
    '--call-graph',
    capability.callGraph,
    '-e',
    capability.syscalls.join(','),
    '-p',
    String(pid),
  ];
}

export function validateIoPerf(
  executable: string,
  maxEvents: number,
  dwarfStackBytes: DwarfStackBytes = 8192,
  callGraph: 'dwarf' | 'fp' = 'dwarf'
): IoPerfCapability {
  const base = validatePerf(executable, dwarfStackBytes, callGraph);
  return {
    executable: base.executable,
    version: base.version,
    kind: 'successful-user-syscalls',
    syscalls: IO_SYSCALLS,
    callGraph: base.callGraph,
    dwarfStackBytes: base.dwarfStackBytes,
    maxEvents,
  };
}

export function classifyIoTraceExit(
  code: number | null,
  signal: NodeJS.Signals | null,
  stopCause: IoTraceStopCause,
  truncated: boolean
): { exitCode: number | null; error: string | null } {
  if (stopCause === 'window' && signal === 'SIGINT' && !truncated)
    return { exitCode: 0, error: null };
  // A collector-caused stop is not evidence that perf exhausted its own
  // event budget. The persisted cause lets a later reader distinguish it.
  if (stopCause === 'byte-cap' || stopCause === 'artifact-write-error')
    return { exitCode: code, error: null };
  if (stopCause === 'none' && code === 0)
    return {
      exitCode: code,
      error: 'perf trace ended before window; --max-events may have been reached',
    };
  const exit = code === null ? `by ${String(signal)}` : String(code);
  return {
    exitCode: code,
    error:
      stopCause !== 'window' || (code !== 0 && signal !== 'SIGINT')
        ? `perf trace exited ${exit}`
        : null,
  };
}

export function shouldPlanIoStop(exitObserved: boolean): boolean {
  return !exitObserved;
}

async function perfForPeer(
  capability: PerfCapability,
  identity: ProcessIdentity,
  seconds: number,
  artifact: string,
  maxBytes?: number
): Promise<PerfResult> {
  const args = buildPerfRecordArgs(capability, identity.pid, seconds, artifact, maxBytes);
  let child: ChildProcess;
  try {
    child = spawn(capability.executable, args, {
      detached: true,
      stdio: ['ignore', 'ignore', 'pipe'],
    });
    child.once('error', () => undefined);
    activePerfChildren.track(child);
  } catch (error) {
    return unattachedResult(identity, artifact, `perf launch failed: ${String(error)}`);
  }
  const perfPid = child.pid!;
  let stderr = '';
  child.stderr?.setEncoding('utf8');
  child.stderr?.on('data', chunk => {
    stderr = (stderr + String(chunk)).slice(-16_384);
  });
  let truncated = false;
  const watchdog =
    maxBytes && capability.maxSize === 'watchdog'
      ? setInterval(() => {
          try {
            if (statSync(artifact).size >= maxBytes) {
              truncated = true;
              activePerfChildren.signal(perfPid, 'SIGINT');
            }
          } catch {
            // perf may not have opened its output yet.
          }
        }, 100)
      : undefined;
  const deadline = setTimeout(
    () => activePerfChildren.signal(perfPid, 'SIGKILL'),
    (seconds + 10) * 1000
  );
  return waitFor(child)
    .then(({ code, signal }) => {
      const errors: string[] = [];
      let bytes: number | null = null;
      try {
        bytes = statSync(artifact).size;
      } catch (error) {
        errors.push(`perf artifact unavailable: ${String(error)}`);
      }
      if (maxBytes && bytes !== null && bytes >= maxBytes) truncated = true;
      const exit = code === null ? `by ${String(signal)}` : String(code);
      if (code !== 0) errors.push(`perf exited ${exit}`);
      if (truncated) errors.push(`perf artifact reached --max-bytes ${maxBytes}`);
      return {
        peer: identity.peer,
        identity,
        artifact,
        exitCode: code,
        signal,
        bytes,
        truncated,
        attached: true,
        identityValid: true,
        stderr,
        errors,
        profileKind: 'cpu' as const,
      };
    })
    .catch(error => ({
      peer: identity.peer,
      identity,
      artifact,
      exitCode: null,
      signal: null,
      bytes: null,
      truncated,
      attached: true,
      identityValid: true,
      stderr,
      errors: [`perf launch failed: ${String(error)}`],
      profileKind: 'cpu' as const,
    }))
    .finally(() => {
      clearTimeout(deadline);
      if (watchdog) clearInterval(watchdog);
    });
}

async function ioForPeer(
  capability: IoPerfCapability,
  identity: ProcessIdentity,
  seconds: number,
  artifact: string,
  maxBytes: number
): Promise<PerfResult> {
  const fd = openSync(artifact, 'wx');
  let child: ChildProcess;
  try {
    child = spawn(capability.executable, buildPerfTraceArgs(capability, identity.pid), {
      detached: true,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    child.once('error', () => undefined);
    activePerfChildren.track(child);
  } catch (error) {
    closeSync(fd);
    return unattachedResult(identity, artifact, `perf trace launch failed: ${String(error)}`, 'io');
  }
  const perfPid = child.pid!;
  let bytes = 0;
  let truncated = false;
  let stopCause: IoTraceStopCause = 'none';
  let forced = false;
  let exitObserved = false;
  child.once('exit', () => {
    exitObserved = true;
  });
  let rawStderrBytes = 0;
  let writeError: string | null = null;
  const collect = (chunk: Buffer | string, isStderr: boolean): void => {
    const value = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
    if (isStderr) rawStderrBytes += value.length;
    const remaining = maxBytes - bytes;
    if (remaining <= 0) {
      truncated = true;
      if (stopCause === 'none') stopCause = 'byte-cap';
      activePerfChildren.signal(perfPid, 'SIGINT');
      return;
    }
    const written = value.subarray(0, remaining);
    try {
      writeSync(fd, written);
    } catch (error) {
      writeError = String(error);
      truncated = true;
      if (stopCause === 'none') stopCause = 'artifact-write-error';
      activePerfChildren.signal(perfPid, 'SIGINT');
      return;
    }
    bytes += written.length;
    if (written.length !== value.length) {
      truncated = true;
      if (stopCause === 'none') stopCause = 'byte-cap';
      activePerfChildren.signal(perfPid, 'SIGINT');
    }
  };
  child.stdout?.on('data', chunk => collect(chunk as Buffer, false));
  child.stderr?.on('data', chunk => collect(chunk as Buffer, true));
  const planned = setTimeout(() => {
    if (!shouldPlanIoStop(exitObserved)) return;
    if (stopCause === 'none') stopCause = 'window';
    activePerfChildren.signal(perfPid, 'SIGINT');
  }, seconds * 1000);
  const deadline = setTimeout(
    () => {
      forced = true;
      activePerfChildren.signal(perfPid, 'SIGKILL');
    },
    (seconds + 5) * 1000
  );
  try {
    const { code, signal } = await waitFor(child);
    const errors: string[] = [];
    const exit = classifyIoTraceExit(code, signal, stopCause, truncated);
    if (stopCause === 'none' && code === 0) {
      truncated = true;
    }
    if (exit.error)
      errors.push(exit.error.replace('--max-events', `--max-events ${capability.maxEvents}`));
    if (forced) {
      truncated = true;
      errors.push('perf trace required forced termination');
    }
    if (writeError) errors.push('perf trace artifact write failed');
    if (truncated && bytes >= maxBytes) errors.push(`perf trace reached --max-bytes ${maxBytes}`);
    return {
      peer: identity.peer,
      identity,
      artifact,
      exitCode: exit.exitCode,
      signal,
      bytes,
      truncated,
      attached: true,
      identityValid: true,
      stderr: rawStderrBytes ? 'perf trace diagnostics retained only in the private artifact' : '',
      errors,
      profileKind: 'io',
      maxEvents: capability.maxEvents,
      stopCause,
    };
  } catch (error) {
    return {
      peer: identity.peer,
      identity,
      artifact,
      exitCode: null,
      signal: null,
      bytes,
      truncated: true,
      attached: true,
      identityValid: true,
      stderr: rawStderrBytes ? 'perf trace diagnostics retained only in the private artifact' : '',
      errors: [`perf trace launch failed: ${String(error)}`],
      profileKind: 'io',
      maxEvents: capability.maxEvents,
    };
  } finally {
    clearTimeout(planned);
    clearTimeout(deadline);
    closeSync(fd);
  }
}

export async function runResourceProfile(options: ResourceProfileOptions): Promise<number> {
  const configs = expectedConductorConfigs(options.meshDir);
  const cpuCapability =
    options.mode === 'cpu' || options.mode === 'cpu-io'
      ? validatePerf(options.perf!, options.dwarfStackBytes, options.callGraph)
      : null;
  const ioCapability =
    options.mode === 'io' || options.mode === 'cpu-io'
      ? validateIoPerf(options.perf!, options.maxEvents, options.dwarfStackBytes, options.callGraph)
      : null;
  const layout = reserveOutput(options.output);
  const sqlTimingSources: SqlTimingSource[] = options.sqlTiming.length
    ? openSqlTimingSources(options.sqlTiming, configs).sources
    : [];
  const workflowSources: WorkflowSource[] = [];
  try {
    const discovery = captureHouseholdResources(configs);
    const binaryFingerprints = await fingerprintBinaries(Object.values(discovery.samples));
    const metricsBefore = await Promise.all(
      options.metricsUrls.map(async target => ({
        target,
        capture: await captureMetricsEndpoint(target),
      }))
    );
    const preArm = captureHouseholdResources(configs);
    const ports =
      options.networkStats || options.armSqlTiming.length || options.armWorkflow.length
        ? networkPorts(options.meshDir, configs)
        : null;
    const networkMembershipSalt = options.networkStats ? randomBytes(32) : null;
    const networkBefore = options.networkStats
      ? await captureNetworkRound(configs, ports!, preArm.samples, networkMembershipSalt!)
      : null;
    const issues = [
      ...discovery.issues.map(issue => `discovery: ${issue}`),
      ...preArm.issues.map(
        issue =>
          `${options.armSqlTiming.length || options.armWorkflow.length ? 'pre-arm' : 'before'}: ${issue}`
      ),
    ];
    for (const entry of metricsBefore) {
      if (entry.capture.error)
        issues.push(`metrics ${entry.target.name} before: ${entry.capture.error}`);
    }
    for (const [peer, fingerprint] of Object.entries(binaryFingerprints)) {
      if (!processIdentityMatches(discovery.samples[peer], preArm.samples[peer])) {
        fingerprint.identityValid = false;
        fingerprint.error = 'conductor identity changed while fingerprinting executable';
      }
      if (fingerprint.error) issues.push(`${peer}: binary fingerprint: ${fingerprint.error}`);
    }
    if (
      (options.armSqlTiming.length || options.armWorkflow.length) &&
      Object.values(binaryFingerprints).some(
        fingerprint => !fingerprint.identityValid || fingerprint.error !== null
      )
    )
      throw new Error(
        'private diagnostic auto-rearm requires valid pre-arm executable fingerprints'
      );

    if (options.armSqlTiming.length || options.armWorkflow.length) {
      // Families are independent native gates. Admit them concurrently so neither family spends
      // the other's two-second bookkeeping margin before the shared resource window begins.
      const [sqlArm, workflowArm] = await Promise.allSettled([
        options.armSqlTiming.length
          ? armSqlTimingSources(options.armSqlTiming, options.seconds, configs, ports!, preArm)
          : Promise.resolve([]),
        options.armWorkflow.length
          ? armWorkflowSources(options.armWorkflow, options.seconds, configs, ports!, preArm)
          : Promise.resolve([]),
      ]);
      if (sqlArm.status === 'fulfilled') sqlTimingSources.push(...sqlArm.value);
      if (workflowArm.status === 'fulfilled') workflowSources.push(...workflowArm.value);
      if (sqlArm.status === 'rejected' || workflowArm.status === 'rejected') {
        closeSqlTimingSources(sqlTimingSources);
        closeWorkflowSources(workflowSources);
        throw new Error(
          'private diagnostic auto-rearm failed; admitted windows will expire without cancellation'
        );
      }
    }
    const armed = options.armSqlTiming.length || options.armWorkflow.length;
    const before = armed ? captureHouseholdResources(configs) : preArm;
    if (armed) {
      issues.push(...before.issues.map(issue => `before: ${issue}`));
      for (const target of [...options.armSqlTiming, ...options.armWorkflow]) {
        if (
          preArm.bootId !== before.bootId ||
          !processIdentityMatches(preArm.samples[target.name], before.samples[target.name])
        )
          throw new Error(
            `${target.name}: conductor identity changed before diagnostic measurement began`
          );
      }
    }
    const profiles: PerfResult[] = [];

    if (cpuCapability || ioCapability) {
      const attach = captureHouseholdResources(configs);
      issues.push(...attach.issues.map(issue => `pre-attach: ${issue}`));
      const jobs = Object.values(before.samples).flatMap(sample => {
        const kinds = [cpuCapability ? 'cpu' : null, ioCapability ? 'io' : null].filter(
          (kind): kind is 'cpu' | 'io' => kind !== null
        );
        return kinds.map(async kind => {
          const artifact = join(
            layout.artifacts,
            `${sample.peer}.${kind === 'cpu' ? 'perf.data' : 'perf-trace.txt'}`
          );
          if (!processIdentityMatches(sample, attach.samples[sample.peer])) {
            return unattachedResult(
              sample,
              artifact,
              'conductor identity changed immediately before perf attach',
              kind
            );
          }
          return kind === 'io'
            ? await ioForPeer(
                ioCapability!,
                sample,
                options.seconds,
                artifact,
                Math.min(options.maxBytes!, MAX_IO_BYTES_PER_CONDUCTOR)
              )
            : await perfForPeer(
                cpuCapability!,
                sample,
                options.seconds,
                artifact,
                options.maxBytes
              );
        });
      });
      profiles.push(...(await Promise.all(jobs)));
    } else {
      await new Promise(resolveDelay => setTimeout(resolveDelay, options.seconds * 1000));
    }

    const networkAfter = options.networkStats
      ? await captureNetworkRound(configs, ports!, before.samples, networkMembershipSalt!)
      : null;
    const after = captureHouseholdResources(configs);
    const metricsAfter = await Promise.all(
      options.metricsUrls.map(async target => ({
        target,
        capture: await captureMetricsEndpoint(target),
      }))
    );
    const resources = resourceWitness(before, after);
    // Finalizing a native terminal may wait up to five seconds. Do not extend
    // the metric scrape interval merely to wait for a diagnostic artifact.
    const sqlTimingDiagnostics = sqlTimingSources.length
      ? await finishSqlTimingSources(sqlTimingSources, before, after, layout.artifacts)
      : [];
    const workflowDiagnostics = workflowSources.length
      ? await finishWorkflowSources(workflowSources, before, after, layout.artifacts)
      : [];
    issues.push(
      ...sqlTimingDiagnostics.flatMap(diagnostic =>
        diagnostic.issues.map(issue => `sql timing ${diagnostic.name}: ${issue}`)
      )
    );
    issues.push(
      ...workflowDiagnostics.flatMap(diagnostic =>
        diagnostic.issues.map(issue => `workflow ${diagnostic.name}: ${issue}`)
      )
    );
    issues.push(...resources.issues);
    for (const entry of metricsAfter) {
      if (entry.capture.error)
        issues.push(`metrics ${entry.target.name} after: ${entry.capture.error}`);
    }
    if (networkBefore)
      issues.push(...networkBefore.issues.map(issue => `network before: ${issue}`));
    if (networkAfter) issues.push(...networkAfter.issues.map(issue => `network after: ${issue}`));
    for (const profile of profiles) {
      if (
        profile.attached &&
        !processIdentityMatches(profile.identity, after.samples[profile.peer])
      ) {
        profile.identityValid = false;
        profile.errors.push('conductor identity changed after perf attach; profile is invalid');
      }
      issues.push(...profile.errors.map(error => `${profile.peer}: ${error}`));
    }
    const collection = {
      mode: options.mode,
      requestedSeconds: options.seconds,
      meshDir: options.meshDir,
      limitations: {
        cpu: options.mode === 'cpu' || options.mode === 'cpu-io' ? null : 'not requested',
        heap: 'unavailable: this collector does not perform heap profiling',
        ioStacks:
          options.mode === 'io' || options.mode === 'cpu-io'
            ? 'successful user syscall bytes; includes files, sockets and logging; excludes mmap, io_uring and kernel writeback'
            : 'unavailable: proc I/O counters have no stack attribution',
        wallTime: 'elapsed observation time is not sampled CPU time',
      },
      perfCapability: { cpu: cpuCapability, io: ioCapability },
      binaryFingerprints,
      resources,
      telemetry: {
        cohort: options.cohort,
        metrics: options.metricsUrls.map((target, index) => ({
          name: target.name,
          url: target.url,
          before: metricsBefore[index].capture,
          after: metricsAfter[index].capture,
        })),
      },
      networkStats:
        networkBefore && networkAfter
          ? {
              before: networkBefore,
              after: networkAfter,
              limitations: [
                'connection membership must remain identical; counters are transport watermarks, not physical traffic',
                'membership tokens use an ephemeral capture-local salt and cannot be joined across captures',
              ],
            }
          : null,
      profiles,
      ...(sqlTimingSources.length || workflowSources.length
        ? {
            diagnostics: {
              ...(sqlTimingSources.length ? { sqlTiming: sqlTimingDiagnostics } : {}),
              ...(workflowSources.length ? { workflow: workflowDiagnostics } : {}),
            },
          }
        : {}),
      issues: [...new Set(issues)],
    };
    writeFileSync(layout.json, `${JSON.stringify(collection, null, 2)}\n`);
    process.stdout.write(`${layout.json}\n`);
    return collection.issues.length ? 1 : 0;
  } finally {
    closeSqlTimingSources(sqlTimingSources);
    closeWorkflowSources(workflowSources);
  }
}

const isMain = process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href;
if (isMain) {
  let shuttingDown = false;
  const shutdown = (reason: string, code: number): void => {
    if (shuttingDown) return;
    shuttingDown = true;
    process.stderr.write(`resource-profile interrupted: ${reason}; stopping perf children\n`);
    activePerfChildren.terminateAll();
    setTimeout(() => process.exit(code), 2100);
  };
  process.once('SIGINT', () => shutdown('SIGINT', 130));
  process.once('SIGTERM', () => shutdown('SIGTERM', 143));
  process.once('uncaughtException', error => shutdown(`uncaught exception: ${String(error)}`, 2));
  process.once('unhandledRejection', error => shutdown(`unhandled rejection: ${String(error)}`, 2));
  runResourceProfile(parseResourceProfileArgs(process.argv.slice(2)))
    .then(code => {
      process.exitCode = code;
    })
    .catch(error => {
      shutdown(`collection failed: ${String(error)}`, 2);
    });
}
