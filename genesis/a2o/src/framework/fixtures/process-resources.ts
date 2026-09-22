/** Failure-loud Linux process resource observations for the local household mesh. */

import { execFileSync } from 'node:child_process';
import { readdirSync, readFileSync, readlinkSync } from 'node:fs';
import { resolve } from 'node:path';
import { performance } from 'node:perf_hooks';

export interface ProcReader {
  listPids(): number[];
  readText(path: string): string;
  readLink(path: string): string;
}

const linuxProc: ProcReader = {
  listPids: () =>
    readdirSync('/proc')
      .filter(v => /^\d+$/.test(v))
      .map(Number),
  readText: path => readFileSync(path, 'utf8'),
  readLink: path => readlinkSync(path),
};

export interface ProcessIdentity {
  peer: string;
  pid: number;
  startTicks: number;
  executable: string;
  configPath: string;
}

export interface ProcessResourceSample extends ProcessIdentity {
  cpuTicks: number;
  rssKiB: number;
  io: {
    rchar: number;
    wchar: number;
    readBytes: number;
    writeBytes: number;
    cancelledWriteBytes: number;
  };
}

export interface ResourceSnapshot {
  /** Wall clock for audit and correlation only; never used for rate math. */
  atUnixMs: number;
  /** Host-monotonic clock used for elapsed-window calculations. */
  monotonicMs: number;
  /**
   * Optional Linux CLOCK_MONOTONIC witness for cross-process correlation.
   * This is not the rate-math clock; existing snapshots may omit it.
   */
  kernelMonotonicStartMs?: number;
  kernelMonotonicMs?: number;
  bootId: string;
  clockTicksPerSecond: number;
  samples: Record<string, ProcessResourceSample>;
  issues: string[];
}

export interface ProcessResourceDelta extends ProcessIdentity {
  cpuSeconds: number;
  rssKiBBefore: number;
  rssKiBAfter: number;
  io: ProcessResourceSample['io'];
}

export interface ResourceWitness {
  kind: 'household-process-resources/v1';
  before: ResourceSnapshot;
  after: ResourceSnapshot;
  elapsedMs: number;
  deltas: Record<string, ProcessResourceDelta>;
  issues: string[];
}

/** Sum a present, finite Prometheus family; optional timestamps are ignored. */
export function requiredPrometheusSeriesSum(
  exposition: string,
  family: string
): { value: number | null; issue?: string } {
  let total = 0;
  let count = 0;
  for (const line of exposition.split('\n')) {
    if (!line.startsWith(family)) continue;
    const next = line.charAt(family.length);
    if (next !== '{' && next !== ' ') continue;
    const value = Number(line.trim().split(/\s+/)[1]);
    if (!Number.isFinite(value)) return { value: null, issue: `${family} has a non-finite sample` };
    total += value;
    count += 1;
  }
  return count > 0
    ? { value: total }
    : { value: null, issue: `${family} is absent from the metrics exposition` };
}

/** Rate a cumulative-counter delta against its own monotonic observation endpoints. */
export function counterRatePerMinute(
  delta: number,
  beforeMonotonicMs: number,
  afterMonotonicMs: number
): { value: number | null; issue?: string } {
  const elapsedMs = afterMonotonicMs - beforeMonotonicMs;
  if (!Number.isFinite(elapsedMs) || elapsedMs <= 0) {
    return { value: null, issue: `invalid metrics elapsed time ${elapsedMs}ms` };
  }
  return { value: delta / (elapsedMs / 60_000) };
}

interface ParsedStat {
  cpuTicks: number;
  startTicks: number;
}

function parseStat(text: string, label: string): ParsedStat {
  const close = text.lastIndexOf(')');
  if (close < 0) throw new Error(`${label}: malformed stat (missing command terminator)`);
  const fields = text
    .slice(close + 2)
    .trim()
    .split(/\s+/);
  const cpuTicks = Number(fields[11]) + Number(fields[12]);
  const startTicks = Number(fields[19]);
  if (
    !Number.isSafeInteger(cpuTicks) ||
    cpuTicks < 0 ||
    !Number.isSafeInteger(startTicks) ||
    startTicks < 0
  ) {
    throw new Error(`${label}: malformed stat counters`);
  }
  return { cpuTicks, startTicks };
}

function parseNamedInteger(text: string, name: string, label: string, signed = false): number {
  const line = text.split('\n').find(candidate => candidate.startsWith(`${name}:`));
  const raw =
    line
      ?.slice(name.length + 1)
      .trim()
      .split(/\s+/)[0] ?? '';
  const value = Number(raw);
  const allowed = signed ? /^-?\d+$/.test(raw) : /^\d+$/.test(raw);
  if (!allowed || !Number.isSafeInteger(value)) {
    throw new Error(`${label}: missing or unsafe ${name}`);
  }
  return value;
}

function readBootId(reader: ProcReader, issues: string[]): string {
  try {
    const bootId = reader.readText('/proc/sys/kernel/random/boot_id').trim();
    if (!bootId) throw new Error('empty boot id');
    return bootId;
  } catch (error) {
    issues.push(`host: cannot read boot identity: ${String(error)}`);
    return '';
  }
}

function readKernelMonotonicMs(provider: () => number, issues: string[]): number | undefined {
  try {
    const value = provider();
    if (!Number.isFinite(value) || value < 0) {
      issues.push(`host: invalid kernel monotonic clock ${value}ms`);
      return undefined;
    }
    return value;
  } catch (error) {
    issues.push(`host: cannot read kernel monotonic clock: ${String(error)}`);
    return undefined;
  }
}

/** Linux CLOCK_MONOTONIC in milliseconds; this witness is not the rate clock. */
function processKernelMonotonicMs(): number {
  if (process.platform !== 'linux') throw new Error('native diagnostic clock requires Linux');
  return Number(process.hrtime.bigint()) / 1_000_000;
}

function discoverCandidates(
  reader: ProcReader,
  expectedConfigs: Record<string, string>
): Map<string, number[]> {
  const byConfig = new Map(
    Object.entries(expectedConfigs).map(([peer, config]) => [resolve(config), peer])
  );
  const candidates = new Map(Object.keys(expectedConfigs).map(peer => [peer, [] as number[]]));
  for (const pid of reader.listPids()) {
    try {
      const argv = reader.readText(`/proc/${pid}/cmdline`).split('\0').filter(Boolean);
      const exe = normalizedExecutable(reader.readLink(`/proc/${pid}/exe`));
      if (!/(^|\/)holochain$/.test(exe) || !/(^|\/)holochain$/.test(argv[0] ?? '')) continue;
      const config = configPath(argv, reader.readLink(`/proc/${pid}/cwd`));
      const peer = config ? byConfig.get(config) : undefined;
      if (peer) candidates.get(peer)?.push(pid);
    } catch {
      // Unrelated processes may exit while enumerating. Named peers are checked below.
    }
  }
  return candidates;
}

function captureCandidate(
  reader: ProcReader,
  peer: string,
  pids: number[],
  expectedConfig: string,
  samples: Record<string, ProcessResourceSample>,
  issues: string[]
): void {
  if (pids.length !== 1) {
    issues.push(`${peer}: expected exactly one owned conductor, observed ${pids.length}`);
    return;
  }
  try {
    samples[peer] = readSample(reader, peer, pids[0], expectedConfig);
  } catch (error) {
    issues.push(`${peer}: resource snapshot failed: ${String(error)}`);
  }
}

function configPath(argv: string[], cwd: string): string | undefined {
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--config-path' || arg === '-c') {
      const value = argv[i + 1];
      if (value) return resolve(cwd, value);
    }
    if (arg.startsWith('--config-path=')) return resolve(cwd, arg.slice('--config-path='.length));
  }
  return undefined;
}

function normalizedExecutable(value: string): string {
  return value.endsWith(' (deleted)') ? value.slice(0, -' (deleted)'.length) : value;
}

function readSample(
  reader: ProcReader,
  peer: string,
  pid: number,
  expectedConfig: string
): ProcessResourceSample {
  const root = `/proc/${pid}`;
  const first = parseStat(reader.readText(`${root}/stat`), `${peer} pid ${pid}`);
  const argv = reader.readText(`${root}/cmdline`).split('\0').filter(Boolean);
  const executable = normalizedExecutable(reader.readLink(`${root}/exe`));
  const cwd = reader.readLink(`${root}/cwd`);
  const observedConfig = configPath(argv, cwd);
  if (!/(^|\/)holochain$/.test(executable) || !/(^|\/)holochain$/.test(argv[0] ?? '')) {
    throw new Error(`${peer}: pid ${pid} is not a holochain executable`);
  }
  if (observedConfig !== resolve(expectedConfig)) {
    throw new Error(
      `${peer}: pid ${pid} config is ${observedConfig ?? 'missing'}, expected ${resolve(expectedConfig)}`
    );
  }
  const status = reader.readText(`${root}/status`);
  const io = reader.readText(`${root}/io`);
  const last = parseStat(reader.readText(`${root}/stat`), `${peer} pid ${pid}`);
  if (first.startTicks !== last.startTicks) {
    throw new Error(`${peer}: pid ${pid} changed start ticks during snapshot`);
  }
  return {
    peer,
    pid,
    startTicks: last.startTicks,
    executable,
    configPath: observedConfig,
    cpuTicks: last.cpuTicks,
    rssKiB: parseNamedInteger(status, 'VmRSS', `${peer} pid ${pid}`),
    io: {
      rchar: parseNamedInteger(io, 'rchar', `${peer} pid ${pid}`),
      wchar: parseNamedInteger(io, 'wchar', `${peer} pid ${pid}`),
      readBytes: parseNamedInteger(io, 'read_bytes', `${peer} pid ${pid}`),
      writeBytes: parseNamedInteger(io, 'write_bytes', `${peer} pid ${pid}`),
      // Linux documents this as potentially negative when truncation undoes
      // another process's accounted write. It is a signed delta, not a counter.
      cancelledWriteBytes: parseNamedInteger(
        io,
        'cancelled_write_bytes',
        `${peer} pid ${pid}`,
        true
      ),
    },
  };
}

export function captureHouseholdResources(
  expectedConfigs: Record<string, string>,
  options: {
    reader?: ProcReader;
    now?: () => number;
    monotonicNow?: () => number;
    kernelMonotonicNow?: () => number;
    clockTicksPerSecond?: number;
  } = {}
): ResourceSnapshot {
  const reader = options.reader ?? linuxProc;
  const issues: string[] = [];
  const samples: Record<string, ProcessResourceSample> = {};
  const kernelMonotonicNow = options.kernelMonotonicNow ?? processKernelMonotonicMs;
  const kernelMonotonicStartMs = readKernelMonotonicMs(kernelMonotonicNow, issues);
  const bootId = readBootId(reader, issues);
  const hz =
    options.clockTicksPerSecond ??
    Number(execFileSync('/usr/bin/getconf', ['CLK_TCK'], { encoding: 'utf8' }).trim());
  if (!Number.isSafeInteger(hz) || hz <= 0) issues.push(`host: invalid CLK_TCK ${hz}`);

  const candidates = discoverCandidates(reader, expectedConfigs);
  for (const [peer, pids] of candidates) {
    captureCandidate(reader, peer, pids, expectedConfigs[peer], samples, issues);
  }
  return {
    atUnixMs: (options.now ?? Date.now)(),
    monotonicMs: (options.monotonicNow ?? performance.now.bind(performance))(),
    kernelMonotonicStartMs,
    kernelMonotonicMs: readKernelMonotonicMs(kernelMonotonicNow, issues),
    bootId,
    clockTicksPerSecond: hz,
    samples,
    issues,
  };
}

const MONOTONIC_IO_FIELDS = ['rchar', 'wchar', 'readBytes', 'writeBytes'] as const;

function deltaFor(
  peer: string,
  before: ProcessResourceSample | undefined,
  after: ProcessResourceSample | undefined,
  clockTicksPerSecond: number,
  issues: string[]
): ProcessResourceDelta | undefined {
  if (!before || !after) {
    issues.push(`${peer}: absent from ${before ? 'after' : 'before'} snapshot`);
    return undefined;
  }
  if (
    before.pid !== after.pid ||
    before.startTicks !== after.startTicks ||
    before.executable !== after.executable ||
    before.configPath !== after.configPath
  ) {
    issues.push(`${peer}: conductor identity changed during observation`);
    return undefined;
  }
  const cpuTicks = after.cpuTicks - before.cpuTicks;
  if (cpuTicks < 0) {
    issues.push(`${peer}: CPU counter rolled backwards`);
    return undefined;
  }
  const io = {} as ProcessResourceSample['io'];
  for (const field of MONOTONIC_IO_FIELDS) {
    const value = after.io[field] - before.io[field];
    if (value < 0) {
      issues.push(`${peer}: ${field} counter rolled backwards`);
      return undefined;
    }
    io[field] = value;
  }
  io.cancelledWriteBytes = after.io.cancelledWriteBytes - before.io.cancelledWriteBytes;
  return {
    peer,
    pid: before.pid,
    startTicks: before.startTicks,
    executable: before.executable,
    configPath: before.configPath,
    cpuSeconds: cpuTicks / clockTicksPerSecond,
    rssKiBBefore: before.rssKiB,
    rssKiBAfter: after.rssKiB,
    io,
  };
}

export function resourceWitness(
  before: ResourceSnapshot,
  after: ResourceSnapshot
): ResourceWitness {
  const issues = [
    ...before.issues.map(v => `before: ${v}`),
    ...after.issues.map(v => `after: ${v}`),
  ];
  const deltas: Record<string, ProcessResourceDelta> = {};
  const observedElapsedMs = after.monotonicMs - before.monotonicMs;
  const elapsedValid = Number.isFinite(observedElapsedMs) && observedElapsedMs > 0;
  // Keep the serialized witness numeric even when the observed clock is bad;
  // the issue below makes zero an invalid window, never a measured duration.
  const elapsedMs = elapsedValid ? observedElapsedMs : 0;
  const bootValid =
    before.bootId.length > 0 && after.bootId.length > 0 && before.bootId === after.bootId;
  const clocksValid =
    Number.isFinite(before.clockTicksPerSecond) &&
    before.clockTicksPerSecond > 0 &&
    Number.isFinite(after.clockTicksPerSecond) &&
    after.clockTicksPerSecond > 0 &&
    before.clockTicksPerSecond === after.clockTicksPerSecond;
  if (!elapsedValid) issues.push(`window: invalid monotonic elapsed time ${observedElapsedMs}ms`);
  if (!bootValid) issues.push('host: boot identity missing or changed during observation');
  if (!clocksValid) issues.push('host: CLK_TCK missing, invalid, or changed during observation');

  if (elapsedValid && bootValid && clocksValid) {
    for (const peer of new Set([...Object.keys(before.samples), ...Object.keys(after.samples)])) {
      const delta = deltaFor(
        peer,
        before.samples[peer],
        after.samples[peer],
        before.clockTicksPerSecond,
        issues
      );
      if (delta) deltas[peer] = delta;
    }
  }
  return { kind: 'household-process-resources/v1', before, after, elapsedMs, deltas, issues };
}
