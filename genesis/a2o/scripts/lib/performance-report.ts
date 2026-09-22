/* eslint-disable sonarjs/no-duplicate-string -- canonical check and axis identifiers intentionally repeat. */
import { execFileSync } from 'node:child_process';
import { readFileSync, statSync } from 'node:fs';
import { basename, isAbsolute, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import * as AjvNs from 'ajv/dist/2020.js';

import {
  resourceWitness,
  type ResourceSnapshot,
} from '../../src/framework/fixtures/process-resources.js';

import {
  evaluateSqlTimingBinding,
  evaluateWorkflowBinding,
  MAX_SQL_TIMING_SOURCES,
  readSqlTimingArtifact,
  readWorkflowArtifact,
} from './performance-binding.js';
import { summarizePrometheusWindow } from './performance-prometheus.js';

import type { DiagnosticAdmissionBinding } from './performance-binding.js';
import type { ExternalEvidence } from './performance-evidence.js';

const AjvCtor: new (options: { allErrors: boolean; strict: boolean }) => AjvNs.default =
  (
    AjvNs as unknown as {
      default: new (options: { allErrors: boolean; strict: boolean }) => AjvNs.default;
    }
  ).default ??
  (AjvNs as unknown as new (options: { allErrors: boolean; strict: boolean }) => AjvNs.default);

const MAX_JSON_BYTES = 128 * 1024 * 1024;
const MAX_PERF_OUTPUT = 16 * 1024 * 1024;

export interface CheckWitness {
  checkId: string;
  outcome: 'passed' | 'failed' | 'skipped';
  summary: string;
  observed?: unknown;
}

export interface Verdict {
  axis: 'runtime-performance';
  subject: null;
  decision: {
    type: 'permit' | 'refuse' | 'refer';
    layer?: string | null;
    reason?: string | null;
    note?: string | null;
  };
  witness: { checks: CheckWitness[] };
  policyRef: string | null;
}

interface ProcessDelta {
  peer: string;
  cpuSeconds: number;
  rssKiBBefore: number;
  rssKiBAfter: number;
  io: Record<string, number>;
}

interface Capture {
  diagnostics?: { sqlTiming?: unknown; workflow?: unknown };
  mode?: string;
  requestedSeconds?: number;
  runId?: string;
  env?: unknown;
  issues?: unknown[];
  binaryFingerprints?: Record<string, { sha256?: string | null; identityValid?: boolean }>;
  resources?: {
    before?: {
      atUnixMs?: number;
      bootId?: string;
      clockTicksPerSecond?: number;
      samples?: Record<string, unknown>;
    };
    after?: {
      atUnixMs?: number;
      bootId?: string;
      clockTicksPerSecond?: number;
      samples?: Record<string, unknown>;
    };
    elapsedMs?: number;
    deltas?: Record<string, ProcessDelta>;
    issues?: unknown[];
  };
  profiles?: {
    peer?: string;
    identity?: {
      peer?: string;
      pid?: number;
      startTicks?: number;
      executable?: string;
      configPath?: string;
    };
    artifact?: string;
    exitCode?: number | null;
    truncated?: boolean;
    attached?: boolean;
    identityValid?: boolean;
    stderr?: string;
    errors?: unknown[];
    profileKind?: 'cpu' | 'io';
    maxEvents?: number;
  }[];
  telemetry?: {
    cohort?: string | null;
    metrics?: {
      name: string;
      url?: string;
      before?: { monotonicMs?: number; text?: string; status?: number; error?: string | null };
      after?: { monotonicMs?: number; text?: string; status?: number; error?: string | null };
    }[];
  };
  networkStats?: {
    before?: NetworkRoundCapture;
    after?: NetworkRoundCapture;
  } | null;
}
interface NetworkRoundCapture {
  atUnixMs?: number;
  monotonicMs?: number;
  issues?: unknown[];
  peers?: Record<
    string,
    {
      peer?: string;
      backend?: string;
      valid?: boolean;
      error?: string | null;
      identity?: NonNullable<NonNullable<Capture['profiles']>[number]['identity']>;
      atUnixMs?: number;
      monotonicMs?: number;
      blockedIncoming?: number;
      blockedOutgoing?: number;
      connections?: {
        membership?: string;
        sendMessageCount?: number;
        sendBytes?: number;
        recvMessageCount?: number;
        recvBytes?: number;
        direct?: boolean;
      }[];
    }
  >;
}

export interface NormalizedRun {
  source: string;
  runId?: string;
  env?: unknown;
  cohort: string | null;
  elapsedSeconds: number;
  window: { startUnixMs: number; endUnixMs: number } | null;
  peers: string[];
  binarySha256ByPeer: Record<string, string>;
  comparable: boolean;
  comparabilityIssues: string[];
  totals: {
    cpuCores: number;
    rssEndKiB: number;
    rssGrowthKiB: number;
    ioBytesPerSecond: number;
  };
  peersDetail: (ProcessDelta & { cpuCores: number; ioBytesPerSecond: number })[];
  methods: unknown[];
  methodLatency: unknown[];
  databaseLatency: unknown[];
  atomLatency: unknown[];
  failures: unknown[];
  cpuProfiles: unknown[];
  ioProfiles: unknown[];
  networkWatermarks: unknown[];
  heapSummary?: unknown;
  sqlTimingBindings?: {
    peer: string;
    status: string;
    nativeIdentity: string;
    interval: string;
    issues: string[];
    admission?: string;
    coverageEligible: false;
  }[];
  workflowBindings?: {
    peer: string;
    status: string;
    admission: string;
    nativeIdentity: string;
    interval: string;
    issues: string[];
    warnings: string[];
    coverageEligible: false;
  }[];
  coverageGaps: string[];
  issues: string[];
  coverageEvidence: Record<CoverageCategory, CoverageResult>;
  externalEvidence?: ExternalEvidence;
}

export const COVERAGE_CATEGORIES = [
  'cpu-leaf',
  'cpu-caller',
  'method-counts',
  'method-durations',
  'method-outcomes',
  'method-timeouts',
  'workflow-runs',
  'workflow-triggers',
  'sql-timing',
  'network-watermarks',
  'heap-attribution',
  'io-attribution',
] as const;
export type CoverageCategory = (typeof COVERAGE_CATEGORIES)[number];
export interface CoverageResult {
  status: 'present' | 'missing' | 'insufficient';
  summary: string;
  observed?: unknown;
}

function finite(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value);
}

function validSnapshot(value: unknown): value is ResourceSnapshot {
  if (!value || typeof value !== 'object') return false;
  const snapshot = value as ResourceSnapshot;
  if (
    !finite(snapshot.atUnixMs) ||
    !finite(snapshot.monotonicMs) ||
    typeof snapshot.bootId !== 'string' ||
    !snapshot.bootId ||
    !Number.isSafeInteger(snapshot.clockTicksPerSecond) ||
    snapshot.clockTicksPerSecond <= 0 ||
    !snapshot.samples ||
    typeof snapshot.samples !== 'object' ||
    !Array.isArray(snapshot.issues)
  )
    return false;
  return Object.entries(snapshot.samples).every(([peer, sample]) => {
    const io = sample?.io as ResourceSnapshot['samples'][string]['io'] | undefined;
    return (
      sample?.peer === peer &&
      Number.isSafeInteger(sample.pid) &&
      sample.pid > 0 &&
      Number.isSafeInteger(sample.startTicks) &&
      sample.startTicks >= 0 &&
      typeof sample.executable === 'string' &&
      Boolean(sample.executable) &&
      typeof sample.configPath === 'string' &&
      Boolean(sample.configPath) &&
      Number.isSafeInteger(sample.cpuTicks) &&
      sample.cpuTicks >= 0 &&
      Number.isSafeInteger(sample.rssKiB) &&
      sample.rssKiB >= 0 &&
      io !== undefined &&
      (['rchar', 'wchar', 'readBytes', 'writeBytes'] as const).every(
        field => Number.isSafeInteger(io[field]) && io[field] >= 0
      ) &&
      Number.isSafeInteger(io.cancelledWriteBytes)
    );
  });
}

export function resolveCapturePath(input: string): string {
  const absolute = resolve(input);
  return statSync(absolute).isDirectory() ? join(absolute, 'resource-profile.json') : absolute;
}

export function readCapture(input: string): { path: string; capture: Capture } {
  const path = resolveCapturePath(input);
  const stat = statSync(path);
  if (!stat.isFile()) throw new Error(`capture is not a regular file: ${path}`);
  const size = stat.size;
  if (size <= 0 || size > MAX_JSON_BYTES)
    throw new Error(`capture must be 1..${MAX_JSON_BYTES} bytes: ${path}`);
  const bytes = readFileSync(path);
  if (bytes.length > MAX_JSON_BYTES)
    throw new Error(`capture grew beyond ${MAX_JSON_BYTES} bytes while reading: ${path}`);
  const parsed: unknown = JSON.parse(bytes.toString('utf8'));
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed))
    throw new Error(`capture is not a JSON object: ${path}`);
  return { path, capture: parsed as Capture };
}

function profileSummary(capture: Capture, perf: string | undefined): unknown[] {
  if (!perf) return [];
  if (!isAbsolute(perf) || basename(perf) !== 'perf')
    throw new Error('--perf must be an absolute path ending in perf');
  return (capture.profiles ?? [])
    .filter(profile => profile.profileKind !== 'io')
    .map(profile => {
      const before = profile.peer ? capture.resources?.before?.samples?.[profile.peer] : undefined;
      const identityMatches = profileIdentityMatches(profile, before);
      const unusable =
        profile.attached === false ||
        profile.identityValid === false ||
        !identityMatches ||
        profile.truncated === true ||
        profile.exitCode !== 0 ||
        (profile.errors?.length ?? 0) > 0;
      if (unusable || !profile.artifact)
        return {
          peer: profile.peer,
          usable: false,
          reason: 'truncated, failed, or identity-invalid profile',
        };
      try {
        const text = execFileSync(
          perf,
          [
            'script',
            '-G',
            '-i',
            profile.artifact,
            '--show-lost-events',
            '--fields',
            'comm,pid,tid,time,period,event,ip,sym,dso',
          ],
          {
            encoding: 'utf8',
            timeout: 15_000,
            maxBuffer: MAX_PERF_OUTPUT,
            stdio: ['ignore', 'pipe', 'pipe'],
          }
        );
        let callchainText: string | undefined;
        let callerAnalysisIssue: string | null = null;
        try {
          callchainText = execFileSync(
            perf,
            [
              'script',
              '-i',
              profile.artifact,
              '--show-lost-events',
              '--fields',
              'comm,pid,tid,time,period,event,ip,sym,dso',
            ],
            {
              encoding: 'utf8',
              timeout: 15_000,
              maxBuffer: MAX_PERF_OUTPUT,
              stdio: ['ignore', 'pipe', 'pipe'],
            }
          );
        } catch (error) {
          callerAnalysisIssue = `caller projection unavailable: ${String(error)}`;
        }
        return {
          ...(parsePerfScript(
            profile.peer ?? 'unknown',
            text,
            profile.stderr ?? '',
            callchainText
          ) as Record<string, unknown>),
          callerAnalysisIssue,
        };
      } catch (error) {
        return {
          peer: profile.peer,
          usable: false,
          reason: `perf script failed: ${String(error)}`,
        };
      }
    });
}

interface IoProfileSummary {
  peer: string;
  usable: boolean;
  reason?: string;
  observedSyscalls?: number;
  successfulSyscalls?: number;
  failedSyscalls?: number;
  successfulUserSyscallBytes?: number;
  resolvedLeafCoveragePercent?: number;
  callerStackCoveragePercent?: number;
  topLeafMethods?: unknown[];
  topCallerClusters?: unknown[];
  syscallErrors?: unknown[];
  note?: string;
  unsupported?: string[];
}

export function parsePerfTrace(peer: string, text: string, maxEvents?: number): IoProfileSummary {
  const lostPrefix = /lost(?:\s+\d+)?\s+(?:samples|events|records|chunks)/i;
  const lostSuffix = /(?:samples|events|records|chunks)\s+lost/i;
  if (lostPrefix.test(text) || lostSuffix.test(text))
    return { peer, usable: false, reason: 'perf trace reported lost data' };
  const leaf = new Map<string, { count: number; bytes: number; latencyMs: number }>();
  const callers = new Map<string, { count: number; bytes: number }>();
  const failures = new Map<string, { count: number; latencyMs: number }>();
  let observed = 0;
  let successful = 0;
  let failed = 0;
  let totalBytes = 0;
  let resolved = 0;
  let callerResolved = 0;
  const blocks: string[][] = [];
  for (const line of text.split('\n')) {
    if (/^\s*\d/.test(line) && line.includes(' ms):')) blocks.push([line]);
    else if (blocks.length) blocks.at(-1)!.push(line);
  }
  for (const block of blocks) {
    const lines = block;
    const first = lines[0] ?? '';
    const duration = /\(\s*([\d.]+)\s+ms\):/.exec(first);
    const syscallMatch = /:\s+\S+\/\d+\s+(\w+)\(/.exec(first);
    const returnedMatch = /\)\s+=\s+(\S+)/.exec(first);
    if (!duration || !syscallMatch || !returnedMatch) continue;
    observed += 1;
    const latencyMs = Number(duration[1]);
    const syscall = syscallMatch[1];
    const returned = Number(returnedMatch[1]);
    if (!Number.isFinite(latencyMs)) continue;
    const isFailure = returnedMatch[1].startsWith('-') && returned !== 0;
    if (!isFailure && !Number.isSafeInteger(returned)) continue;
    const bytes =
      !isFailure && /^(?:read|write|pread64|pwrite64|readv|writev)$/.test(syscall) ? returned : 0;
    const frames = lines
      .slice(1)
      .map(line => {
        const open = line.lastIndexOf(' (');
        if (open < 0) return '';
        return line
          .slice(0, open)
          .trim()
          .replace(/^\S+\s+/, '')
          .replace(/\+0x[\da-f]+$/i, '');
      })
      .filter(Boolean);
    const method = frames[0] ?? '[unknown]';
    if (!/^\[?unknown\]?$/i.test(method)) resolved += 1;
    if (isFailure) {
      failed += 1;
      const failure = failures.get(syscall) ?? { count: 0, latencyMs: 0 };
      failure.count += 1;
      failure.latencyMs += latencyMs;
      failures.set(syscall, failure);
    } else {
      successful += 1;
      totalBytes += bytes;
    }
    const row = leaf.get(`${syscall}:${method}`) ?? { count: 0, bytes: 0, latencyMs: 0 };
    row.count += 1;
    row.bytes += bytes;
    row.latencyMs += latencyMs;
    leaf.set(`${syscall}:${method}`, row);
    const knownCallers = frames.slice(1, 4).filter(frame => !/^\[?unknown\]?$/i.test(frame));
    if (knownCallers.length) {
      callerResolved += 1;
      knownCallers.reverse();
      const key = `${syscall}:${knownCallers.join(' → ')}`;
      const caller = callers.get(key) ?? { count: 0, bytes: 0 };
      caller.count += 1;
      caller.bytes += bytes;
      callers.set(key, caller);
    }
  }
  const rank = <T extends { bytes: number; count: number }>(entries: [string, T][]) => {
    const sorted = [...entries];
    sorted.sort((a, b) => b[1].bytes - a[1].bytes || b[1].count - a[1].count);
    return sorted.slice(0, 20).map(([name, values]) => ({ name, ...values }));
  };
  return {
    peer,
    observedSyscalls: observed,
    successfulSyscalls: successful,
    failedSyscalls: failed,
    successfulUserSyscallBytes: totalBytes,
    usable: observed > 0 && !(maxEvents !== undefined && observed >= maxEvents),
    ...(maxEvents !== undefined && observed >= maxEvents
      ? { reason: `observed event count reached max-events ${maxEvents}` }
      : {}),
    resolvedLeafCoveragePercent: observed ? (100 * resolved) / observed : 0,
    callerStackCoveragePercent: observed ? (100 * callerResolved) / observed : 0,
    topLeafMethods: rank([...leaf.entries()]),
    topCallerClusters: rank([...callers.entries()]),
    syscallErrors: [...failures.entries()].map(([syscall, values]) => ({ syscall, ...values })),
    note: 'bytes are successful user syscall return bytes (including files, sockets, and logging), not physical/device I/O; counts are syscall observations, not method invocations',
    unsupported: ['mmap I/O', 'io_uring', 'kernel writeback', 'physical device attribution'],
  };
}

function ioProfileSummary(capture: Capture): IoProfileSummary[] {
  return (capture.profiles ?? [])
    .filter(profile => profile.profileKind === 'io')
    .map(profile => {
      const before = profile.peer ? capture.resources?.before?.samples?.[profile.peer] : undefined;
      if (
        !profile.peer ||
        !profile.artifact ||
        !profileIdentityMatches(profile, before) ||
        profile.attached === false ||
        profile.identityValid === false ||
        profile.truncated ||
        (profile.errors?.length ?? 0) > 0
      )
        return {
          peer: profile.peer ?? 'unknown',
          usable: false,
          reason: 'truncated, failed, or identity-invalid I/O profile',
        };
      try {
        const stat = statSync(profile.artifact);
        if (!stat.isFile() || stat.size <= 0 || stat.size > 64 * 1024 * 1024)
          return {
            peer: profile.peer,
            usable: false,
            reason: 'I/O artifact is not a bounded regular file',
          };
        return parsePerfTrace(
          profile.peer,
          readFileSync(profile.artifact, 'utf8'),
          profile.maxEvents
        );
      } catch (error) {
        return {
          peer: profile.peer,
          usable: false,
          reason: `I/O artifact unreadable: ${String(error)}`,
        };
      }
    });
}

function networkSummary(
  capture: Capture,
  peers: string[],
  identityEligible: boolean
): { rows: unknown[]; valid: boolean; gaps: string[] } {
  const before = capture.networkStats?.before;
  const after = capture.networkStats?.after;
  const gaps: string[] = [];
  if (
    !before ||
    !after ||
    !finite(before.monotonicMs) ||
    !finite(after.monotonicMs) ||
    after.monotonicMs <= before.monotonicMs
  )
    return { rows: [], valid: false, gaps: ['paired native network stats unavailable'] };
  if (!identityEligible) gaps.push('network capture lacks stable resource/binary identity');
  if (
    !finite(before.atUnixMs) ||
    !finite(after.atUnixMs) ||
    !finite(capture.resources?.before?.atUnixMs) ||
    !finite(capture.resources?.after?.atUnixMs) ||
    before.atUnixMs < capture.resources.before.atUnixMs ||
    after.atUnixMs > capture.resources.after.atUnixMs
  )
    gaps.push('network observations are not bracketed by the resource window');
  if ((before.issues?.length ?? 0) || (after.issues?.length ?? 0))
    gaps.push('network capture reported an issue');
  const rows: unknown[] = [];
  for (const peer of peers) {
    const left = before.peers?.[peer];
    const right = after.peers?.[peer];
    const resource = capture.resources?.before?.samples?.[peer];
    if (
      left?.valid !== true ||
      right?.valid !== true ||
      !profileIdentityMatches({ peer, identity: left.identity }, resource) ||
      !profileIdentityMatches({ peer, identity: right.identity }, resource)
    ) {
      gaps.push(`${peer}: network identity invalid`);
      continue;
    }
    if (
      !finite(left.atUnixMs) ||
      !finite(right.atUnixMs) ||
      !finite(left.monotonicMs) ||
      !finite(right.monotonicMs) ||
      right.monotonicMs <= left.monotonicMs ||
      left.atUnixMs < capture.resources!.before!.atUnixMs! ||
      right.atUnixMs > capture.resources!.after!.atUnixMs!
    ) {
      gaps.push(`${peer}: network peer timestamps invalid`);
      continue;
    }
    if (
      !Array.isArray(left.connections) ||
      !Array.isArray(right.connections) ||
      left.connections.length > 10_000 ||
      right.connections.length > 10_000
    ) {
      gaps.push(`${peer}: network connections missing or exceed the 10000 row bound`);
      continue;
    }
    const validConnection = (row: unknown): row is NonNullable<typeof left.connections>[number] =>
      row !== null &&
      typeof row === 'object' &&
      typeof (row as { membership?: unknown }).membership === 'string' &&
      /^[0-9a-f]{64}$/.test((row as { membership: string }).membership) &&
      typeof (row as { direct?: unknown }).direct === 'boolean';
    if (!left.connections.every(validConnection) || !right.connections.every(validConnection)) {
      gaps.push(`${peer}: network connection identity or direct flag invalid`);
      continue;
    }
    const leftConnections = new Map(left.connections.map(row => [row.membership!, row]));
    const rightConnections = new Map(right.connections.map(row => [row.membership!, row]));
    if (
      leftConnections.size !== left.connections.length ||
      rightConnections.size !== right.connections.length
    ) {
      gaps.push(`${peer}: duplicate network connection identity`);
      continue;
    }
    if (left.backend !== 'iroh' || left.backend !== right.backend) {
      gaps.push(`${peer}: network backend unsupported or changed`);
      continue;
    }
    if (
      [...leftConnections.keys()].sort().join('\0') !==
      [...rightConnections.keys()].sort().join('\0')
    ) {
      gaps.push(`${peer}: network connection membership changed`);
      continue;
    }
    const totals = { sendBytes: 0, recvBytes: 0, sendMessages: 0, recvMessages: 0 };
    let reset = false;
    for (const [key, a] of leftConnections) {
      const b = rightConnections.get(key)!;
      for (const [source, target] of [
        ['sendBytes', 'sendBytes'],
        ['recvBytes', 'recvBytes'],
        ['sendMessageCount', 'sendMessages'],
        ['recvMessageCount', 'recvMessages'],
      ] as const) {
        const av = a[source];
        const bv = b[source];
        if (
          typeof av !== 'number' ||
          typeof bv !== 'number' ||
          !Number.isSafeInteger(av) ||
          !Number.isSafeInteger(bv) ||
          av < 0 ||
          bv < av ||
          !Number.isSafeInteger(totals[target] + bv - av)
        )
          reset = true;
        else totals[target] += bv - av;
      }
    }
    if (reset) {
      gaps.push(`${peer}: network counter reset`);
      continue;
    }
    const leftBlockedIncoming = left.blockedIncoming;
    const rightBlockedIncoming = right.blockedIncoming;
    const leftBlockedOutgoing = left.blockedOutgoing;
    const rightBlockedOutgoing = right.blockedOutgoing;
    if (
      typeof leftBlockedIncoming !== 'number' ||
      typeof rightBlockedIncoming !== 'number' ||
      typeof leftBlockedOutgoing !== 'number' ||
      typeof rightBlockedOutgoing !== 'number'
    ) {
      gaps.push(`${peer}: blocked-message counter invalid`);
      continue;
    }
    const blockedIncomingDelta = rightBlockedIncoming - leftBlockedIncoming;
    const blockedOutgoingDelta = rightBlockedOutgoing - leftBlockedOutgoing;
    if (
      !Number.isSafeInteger(leftBlockedIncoming) ||
      !Number.isSafeInteger(rightBlockedIncoming) ||
      leftBlockedIncoming < 0 ||
      blockedIncomingDelta < 0 ||
      !Number.isSafeInteger(leftBlockedOutgoing) ||
      !Number.isSafeInteger(rightBlockedOutgoing) ||
      leftBlockedOutgoing < 0 ||
      blockedOutgoingDelta < 0
    ) {
      gaps.push(`${peer}: blocked-message counter reset`);
      continue;
    }
    rows.push({
      peer,
      backend: left.backend,
      connectionCount: leftConnections.size,
      ...totals,
      blockedIncomingDelta,
      blockedOutgoingDelta,
      elapsedSeconds: (right.monotonicMs - left.monotonicMs) / 1000,
      note: 'matched-connection transport watermarks; no peer keys or URLs retained',
    });
  }
  return {
    rows,
    valid: gaps.length === 0 && rows.length === peers.length && peers.length > 0,
    gaps,
  };
}

export function profileIdentityMatches(
  profile: NonNullable<Capture['profiles']>[number],
  before: unknown
): boolean {
  if (!before || typeof before !== 'object') return false;
  const sample = before as {
    peer?: string;
    pid?: number;
    startTicks?: number;
    executable?: string;
    configPath?: string;
  };
  return Boolean(
    profile.peer &&
    profile.identity?.peer === profile.peer &&
    sample.peer === profile.peer &&
    profile.identity.pid === sample.pid &&
    profile.identity.startTicks === sample.startTicks &&
    profile.identity.executable === sample.executable &&
    profile.identity.configPath === sample.configPath
  );
}

export function parsePerfScript(
  peer: string,
  text: string,
  recordStderr = '',
  callchainText?: string
): unknown {
  const lostPrefix = /lost(?:\s+\d+)?\s+(?:samples|events|records|chunks)/i;
  const lostSuffix = /(?:samples|events|records|chunks)\s+lost/i;
  if (
    lostPrefix.exec(text) ||
    lostPrefix.exec(recordStderr) ||
    lostSuffix.exec(text) ||
    lostSuffix.exec(recordStderr)
  )
    return { peer, usable: false, reason: 'perf reported lost sampling data' };
  const callchainLost = Boolean(
    callchainText && (lostPrefix.exec(callchainText) ?? lostSuffix.exec(callchainText))
  );
  const symbols = new Map<string, number>();
  const callers = new Map<string, number>();
  const callerPrefixes = new Map<string, number>();
  let totalPeriod = 0;
  let samples = 0;
  let unknownPeriod = 0;
  let callerStackPeriod = 0;
  let callerKnownPrefixPeriod = 0;
  let callerUnknownTailAfterKnownPrefixPeriod = 0;
  let callerImmediateUnavailablePeriod = 0;
  const projected = text
    .split('\n')
    .map(line => {
      const header = /:\s+(\d+)\s+cpu-clock(?::u)?:/.exec(line);
      if (!header) return null;
      const eventEnd = (header.index ?? 0) + header[0].length;
      const tail = eventEnd > 0 ? line.slice(eventEnd).trim() : '';
      const address = /^[\da-f]+\s+/i.exec(tail)?.[0];
      const dsoAt = tail.lastIndexOf(' (');
      if (!address || dsoAt <= address.length) return null;
      return {
        period: Number(header[1]),
        leaf: tail
          .slice(address.length, dsoAt)
          .trim()
          .replace(/\+0x[\da-f]+$/i, ''),
      };
    })
    .filter((row): row is { period: number; leaf: string } => row !== null);
  for (const row of projected) {
    totalPeriod += row.period;
    samples += 1;
    symbols.set(row.leaf, (symbols.get(row.leaf) ?? 0) + row.period);
    if (row.leaf === '[unknown]' || row.leaf === 'unknown') unknownPeriod += row.period;
  }
  const hasSelfProjection = projected.length > 0;
  const callerSource = callchainText ?? (hasSelfProjection ? '' : text);
  const callerHeaders = callerSource
    ? callerSource
        .split('\n')
        .map(line => /:\s+(\d+)\s+cpu-clock(?::u)?:/.exec(line))
        .filter((header): header is RegExpExecArray => header !== null)
    : [];
  const callerSampleCount = callerHeaders.length;
  const callerTotalPeriod = callerHeaders.reduce((sum, header) => sum + Number(header[1]), 0);
  for (const block of callerSource ? callerSource.split(/\n\s*\n/) : []) {
    const lines = block.split('\n').filter(Boolean);
    const header = /:\s+(\d+)\s+cpu-clock(?::u)?:/.exec(lines[0] ?? '');
    if (!header) continue;
    const period = Number(header[1]);
    const frames = lines
      .slice(1)
      .map(line => {
        const open = line.lastIndexOf(' (');
        if (open < 0) return undefined;
        const address = /^\s*[0-9a-f]+\s+/.exec(line)?.[0];
        return address ? line.slice(address.length, open).trim() : undefined;
      })
      .map(frame => frame?.replace(/\+0x[\da-f]+$/i, ''))
      .filter((frame): frame is string => Boolean(frame));
    const leaf = (frames[0] ?? '[unknown]').replace(/\+0x[\da-f]+$/i, '');
    if (!hasSelfProjection) {
      totalPeriod += period;
      samples += 1;
      symbols.set(leaf, (symbols.get(leaf) ?? 0) + period);
      if (leaf === '[unknown]' || leaf === 'unknown') unknownPeriod += period;
    }
    const callerFrames = frames.slice(1);
    const firstUnknownCaller = callerFrames.findIndex(frame => /^\[?unknown\]?$/i.test(frame));
    const knownPrefix =
      firstUnknownCaller < 0 ? callerFrames : callerFrames.slice(0, firstUnknownCaller);
    if (knownPrefix.length > 0) {
      callerKnownPrefixPeriod += period;
      const truncatedByUnknown = firstUnknownCaller >= 0;
      if (truncatedByUnknown) callerUnknownTailAfterKnownPrefixPeriod += period;
      const cluster = knownPrefix.slice(0, 3).reverse().join(' → ');
      const key = `${truncatedByUnknown ? '1' : '0'}\u0000${cluster}`;
      callerPrefixes.set(key, (callerPrefixes.get(key) ?? 0) + period);
    } else if (callerFrames.length === 0 || firstUnknownCaller === 0) {
      callerImmediateUnavailablePeriod += period;
    }
    if (frames.length > 1 && firstUnknownCaller < 0) {
      callerStackPeriod += period;
      const cluster = callerFrames.slice(0, 3).reverse().join(' → ');
      callers.set(cluster, (callers.get(cluster) ?? 0) + period);
    }
  }
  const unknownFraction = totalPeriod ? unknownPeriod / totalPeriod : 1;
  const callerProjectionIssue = callchainLost
    ? 'caller projection reported lost sampling data'
    : callerSource && (callerSampleCount !== samples || callerTotalPeriod !== totalPeriod)
      ? `caller projection denominator mismatch: self=${samples}/${totalPeriod}, caller=${callerSampleCount}/${callerTotalPeriod}`
      : null;
  const callerProjectionValid = callerProjectionIssue === null;
  const top = (values: Map<string, number>) =>
    [...values.entries()]
      .sort((a, b) => b[1] - a[1])
      .slice(0, 12)
      .map(([name, periodNs]) => ({
        name,
        periodNs,
        percent: totalPeriod ? (100 * periodNs) / totalPeriod : 0,
      }));
  const topPrefixes = [...callerPrefixes.entries()]
    .sort((a, b) => b[1] - a[1])
    .slice(0, 12)
    .map(([key, periodNs]) => {
      const [truncated, name] = key.split('\u0000', 2);
      return {
        name,
        periodNs,
        percent: totalPeriod ? (100 * periodNs) / totalPeriod : 0,
        truncatedByUnknown: truncated === '1',
      };
    });
  return {
    peer,
    usable: samples > 0,
    sampledPeriodNs: totalPeriod,
    sampledCount: samples,
    note: 'sample count is not a call count; percentages are sampled self CPU',
    unknownFramePercent: unknownFraction * 100,
    callerStackCoveragePercent:
      callerProjectionValid && totalPeriod ? (100 * callerStackPeriod) / totalPeriod : null,
    callerKnownPrefixCoveragePercent:
      callerProjectionValid && totalPeriod ? (100 * callerKnownPrefixPeriod) / totalPeriod : null,
    callerUnknownTailAfterKnownPrefixPercent:
      callerProjectionValid && totalPeriod
        ? (100 * callerUnknownTailAfterKnownPrefixPeriod) / totalPeriod
        : null,
    callerImmediateUnavailablePercent:
      callerProjectionValid && totalPeriod
        ? (100 * callerImmediateUnavailablePeriod) / totalPeriod
        : null,
    topSelf: top(symbols),
    topCallerPrefixes: callerProjectionValid ? topPrefixes : [],
    callerClusters:
      callerProjectionValid && unknownFraction <= 0.25 && callerStackPeriod / totalPeriod >= 0.75
        ? top(callers)
        : [],
    callerClustersReason: callerProjectionValid
      ? unknownFraction <= 0.25 && callerStackPeriod / totalPeriod >= 0.75
        ? null
        : 'withheld because leaf or caller-stack coverage is below 75%'
      : callerProjectionIssue,
    callerProjectionIssue,
  };
}

function metricSummaries(capture: Capture): {
  methods: unknown[];
  methodLatency: unknown[];
  databaseLatency: unknown[];
  atomLatency: unknown[];
  failures: unknown[];
  gaps: string[];
  issues: string[];
  producerIdentityValid: boolean;
} {
  const methods: unknown[] = [];
  const methodLatency: unknown[] = [];
  const databaseLatency: unknown[] = [];
  const atomLatency: unknown[] = [];
  const failures: unknown[] = [];
  const gaps: string[] = [];
  const issues: string[] = [];
  let producerIdentityValid = true;
  const metrics = capture.telemetry?.metrics ?? [];
  if (!metrics.length) gaps.push('Prometheus telemetry/method call counts and latency unavailable');
  for (const endpoint of metrics) {
    if (
      !endpoint.before?.text ||
      !endpoint.after?.text ||
      !finite(endpoint.before.monotonicMs) ||
      !finite(endpoint.after.monotonicMs) ||
      endpoint.after.monotonicMs <= endpoint.before.monotonicMs ||
      endpoint.before.error ||
      endpoint.after.error
    ) {
      issues.push(`${endpoint.name}: metrics window unreadable`);
      producerIdentityValid = false;
      continue;
    }
    const summary = summarizePrometheusWindow(
      endpoint.before.text,
      endpoint.after.text,
      (endpoint.after.monotonicMs - endpoint.before.monotonicMs) / 1000
    );
    issues.push(...summary.issues.map(issue => `${endpoint.name}: ${issue}`));
    if (!summary.valid || !summary.producerIdentity.verified || summary.producerIdentity.changed) {
      gaps.push(
        `${endpoint.name}: ${summary.producerIdentity.reason ?? 'metrics parser integrity failed'}`
      );
      producerIdentityValid = false;
    }
    for (const counter of summary.counters) {
      const row = { endpoint: endpoint.name, ...counter };
      if (counter.name === 'elohim_conductor_calls_total') methods.push(row);
      if (
        /timeout/i.test(counter.name) ||
        /shed/i.test(counter.name) ||
        counter.name === 'elohim_conductor_call_dropped_total' ||
        /http.*(status|request)/i.test(counter.name)
      )
        failures.push(row);
    }
    for (const histogram of summary.histograms) {
      const row = { endpoint: endpoint.name, ...histogram };
      if (histogram.name === 'elohim_atom_duration_ms') atomLatency.push(row);
      if (histogram.name === 'elohim_conductor_call_duration_ms') methodLatency.push(row);
      if (histogram.name === 'elohim_db_diagnostic_query_duration_ms') databaseLatency.push(row);
    }
  }
  methods.sort(
    (a, b) =>
      Number((b as { perMinute?: number }).perMinute ?? 0) -
      Number((a as { perMinute?: number }).perMinute ?? 0)
  );
  atomLatency.sort(
    (a, b) => Number((b as { p95?: number }).p95 ?? 0) - Number((a as { p95?: number }).p95 ?? 0)
  );
  methodLatency.sort(
    (a, b) => Number((b as { p95?: number }).p95 ?? 0) - Number((a as { p95?: number }).p95 ?? 0)
  );
  databaseLatency.sort(
    (a, b) => Number((b as { p95?: number }).p95 ?? 0) - Number((a as { p95?: number }).p95 ?? 0)
  );
  return {
    methods,
    methodLatency,
    databaseLatency,
    atomLatency,
    failures,
    gaps,
    issues,
    producerIdentityValid,
  };
}

export function normalizeCapture(input: string, perf?: string): NormalizedRun {
  const { path, capture } = readCapture(input);
  if (!validSnapshot(capture.resources?.before) || !validSnapshot(capture.resources?.after))
    throw new Error(`${path}: invalid raw resource snapshots`);
  if (capture.resources.after.atUnixMs <= capture.resources.before.atUnixMs)
    throw new Error(`${path}: invalid wall-clock resource window`);
  const recomputed = resourceWitness(capture.resources.before, capture.resources.after);
  if (recomputed.issues.length)
    throw new Error(`${path}: invalid resource witness: ${recomputed.issues.join('; ')}`);
  if (
    capture.resources?.elapsedMs !== recomputed.elapsedMs ||
    JSON.stringify(capture.resources?.deltas) !== JSON.stringify(recomputed.deltas)
  )
    throw new Error(`${path}: serialized resource witness does not match raw snapshots`);
  const elapsedSeconds = recomputed.elapsedMs / 1000;
  const deltas = recomputed.deltas;
  if (!Object.keys(deltas).length) throw new Error(`${path}: no process resource deltas`);
  const peersDetail = Object.values(deltas).map(delta => {
    if (
      !finite(delta.cpuSeconds) ||
      !finite(delta.rssKiBBefore) ||
      !finite(delta.rssKiBAfter) ||
      delta.cpuSeconds < 0 ||
      delta.rssKiBBefore < 0 ||
      delta.rssKiBAfter < 0
    )
      throw new Error(`${path}: invalid resource delta for ${delta.peer}`);
    const ioBytes = Object.entries(delta.io ?? {})
      .filter(([name]) => name === 'readBytes' || name === 'writeBytes')
      .reduce((sum, [, value]) => sum + (finite(value) ? value : 0), 0);
    if (
      (['rchar', 'wchar', 'readBytes', 'writeBytes'] as const).some(
        field => !finite(delta.io?.[field]) || delta.io[field] < 0
      ) ||
      !finite(delta.io?.cancelledWriteBytes)
    )
      throw new Error(`${path}: invalid or reset I/O counter delta for ${delta.peer}`);
    return {
      ...delta,
      cpuCores: delta.cpuSeconds / elapsedSeconds,
      ioBytesPerSecond: ioBytes / elapsedSeconds,
    };
  });
  const peers = peersDetail.map(row => row.peer).sort();
  const resourceIssues = [
    ...(capture.resources?.issues ?? []),
    ...(capture.issues ?? []).filter(issue => /^(discovery|before|after):/.test(String(issue))),
  ].map(String);
  const clocksValid =
    capture.resources?.before?.bootId &&
    capture.resources.before.bootId === capture.resources?.after?.bootId &&
    capture.resources.before.clockTicksPerSecond === capture.resources?.after?.clockTicksPerSecond;
  const fingerprints = capture.binaryFingerprints ?? {};
  const fingerprintPeers = Object.keys(fingerprints).sort();
  const identitiesValid =
    fingerprintPeers.length === peers.length &&
    fingerprintPeers.every(
      peer =>
        peers.includes(peer) && fingerprints[peer]?.identityValid && fingerprints[peer]?.sha256
    );
  const comparabilityIssues: string[] = [];
  const cohort = capture.telemetry?.cohort;
  if (
    typeof cohort !== 'string' ||
    !cohort?.length ||
    cohort !== cohort.trim() ||
    [...cohort].some(character => {
      const code = character.codePointAt(0) ?? 0;
      return code <= 31 || code === 127;
    })
  )
    comparabilityIssues.push('telemetry.cohort is absent or not canonical exact bytes');
  if (!clocksValid) comparabilityIssues.push('resource clocks/boot identity are not stable');
  if (!identitiesValid) comparabilityIssues.push('binary/resource identity witness is incomplete');
  if (resourceIssues.length) comparabilityIssues.push('resource or discovery issue was reported');
  const telemetry = metricSummaries(capture);
  if ((capture.telemetry?.metrics?.length ?? 0) > 0 && !telemetry.producerIdentityValid)
    comparabilityIssues.push('metrics producer restart identity is unavailable');
  const binarySha256ByPeer: Record<string, string> = {};
  for (const peer of fingerprintPeers) {
    const sha256 = fingerprints[peer]?.sha256;
    if (sha256) binarySha256ByPeer[peer] = sha256;
  }
  const cpuProfiles = profileSummary(capture, perf);
  const ioProfiles = ioProfileSummary(capture);
  const network = networkSummary(
    capture,
    peers,
    Boolean(identitiesValid && clocksValid && fingerprintPeers.join('\0') === peers.join('\0'))
  );
  const coverageEvidence = deriveCoverageEvidence(
    peers,
    (capture.telemetry?.metrics ?? []).map(metric => metric.name),
    cpuProfiles,
    telemetry.methods,
    telemetry.methodLatency,
    ioProfiles,
    telemetry.failures
  );
  const sqlTimingBindings = readSqlTimingBindings(capture);
  const workflowBindings = readWorkflowBindings(capture);
  if (network.valid)
    coverageEvidence['network-watermarks'] = {
      status: 'present',
      summary: 'every peer has identity-matched paired native network transport watermarks',
      observed: network.rows,
    };
  return {
    source: path,
    runId: capture.runId,
    env: capture.env,
    cohort: typeof cohort === 'string' ? cohort : null,
    elapsedSeconds,
    window:
      finite(capture.resources?.before?.atUnixMs) && finite(capture.resources?.after?.atUnixMs)
        ? {
            startUnixMs: capture.resources.before.atUnixMs,
            endUnixMs: capture.resources.after.atUnixMs,
          }
        : null,
    peers,
    binarySha256ByPeer,
    comparable: comparabilityIssues.length === 0,
    comparabilityIssues,
    totals: {
      cpuCores: peersDetail.reduce((sum, row) => sum + row.cpuCores, 0),
      rssEndKiB: peersDetail.reduce((sum, row) => sum + row.rssKiBAfter, 0),
      rssGrowthKiB: peersDetail.reduce((sum, row) => sum + row.rssKiBAfter - row.rssKiBBefore, 0),
      ioBytesPerSecond: peersDetail.reduce((sum, row) => sum + row.ioBytesPerSecond, 0),
    },
    peersDetail,
    methods: telemetry.methods,
    methodLatency: telemetry.methodLatency,
    databaseLatency: telemetry.databaseLatency,
    atomLatency: telemetry.atomLatency,
    failures: telemetry.failures,
    cpuProfiles,
    ioProfiles,
    networkWatermarks: network.rows,
    sqlTimingBindings,
    workflowBindings,
    coverageGaps: [
      ...telemetry.gaps,
      'heap attribution unavailable',
      ...(ioProfiles.length ? [] : ['I/O stack attribution unavailable']),
      ...network.gaps,
      'per-method timeouts unavailable unless exported as a labeled counter',
      'workflow trigger counts unavailable unless exported by telemetry',
    ],
    issues: [
      ...resourceIssues,
      ...telemetry.issues,
      ...sqlTimingBindings.flatMap(row => row.issues.map(issue => `${row.peer}: ${issue}`)),
      ...workflowBindings.flatMap(row => row.issues.map(issue => `${row.peer}: ${issue}`)),
    ],
    coverageEvidence,
  };
}

function readSqlTimingBindings(capture: Capture): NonNullable<NormalizedRun['sqlTimingBindings']> {
  const supplied = capture.diagnostics?.sqlTiming;
  if (supplied === undefined) return [];
  if (!Array.isArray(supplied) || supplied.length > MAX_SQL_TIMING_SOURCES)
    throw new Error('invalid or over-limit captured SQL timing artifact list');
  const names = new Set<string>();
  return supplied.map((value: unknown) => {
    if (!value || typeof value !== 'object' || Array.isArray(value))
      throw new Error('invalid captured SQL timing artifact');
    const row = value as Record<string, unknown>;
    if (
      typeof row.name !== 'string' ||
      !/^[a-zA-Z0-9][a-zA-Z0-9_.-]*$/.test(row.name) ||
      names.has(row.name) ||
      !Object.hasOwn(capture.resources?.before?.samples ?? {}, row.name)
    )
      throw new Error('unknown or duplicate captured SQL timing peer');
    names.add(row.name);
    try {
      if (
        typeof row.artifactPath !== 'string' ||
        !isAbsolute(row.artifactPath) ||
        typeof row.artifactSha256 !== 'string' ||
        !/^[a-f0-9]{64}$/.test(row.artifactSha256)
      )
        throw new Error('captured SQL timing artifact or digest unavailable');
      const expectedAdmission = capturedAdmission(row, 'SQL timing');
      const artifact = readSqlTimingArtifact(row.artifactPath);
      if (
        artifact.sha256 !== row.artifactSha256 ||
        artifact.sha256 !== row.sourceSha256 ||
        artifact.bytes.length !== row.bytes
      )
        throw new Error('captured SQL timing artifact bytes do not match its witness');
      if (!validSnapshot(capture.resources?.before) || !validSnapshot(capture.resources?.after))
        throw new Error('SQL timing resource snapshots unavailable');
      const binding = evaluateSqlTimingBinding(
        artifact.bytes,
        capture.resources.before,
        capture.resources.after,
        row.name,
        expectedAdmission
      );
      return { peer: row.name, ...binding, coverageEligible: false as const };
    } catch (error) {
      return {
        peer: row.name,
        status: 'refused',
        nativeIdentity: 'unavailable',
        interval: 'unavailable',
        issues: [error instanceof Error ? error.message : 'SQL timing artifact read failed'],
        coverageEligible: false as const,
      };
    }
  });
}

function capturedAdmission(
  row: Record<string, unknown>,
  label: string
): DiagnosticAdmissionBinding | undefined {
  const fields = [
    'nonce',
    'producerId',
    'generation',
    'outputBasename',
    'requestedSeconds',
  ] as const;
  const present = fields.map(field => row[field] !== undefined);
  if (!present.some(Boolean)) return undefined;
  if (
    !present.every(Boolean) ||
    typeof row.nonce !== 'string' ||
    !/^[A-Za-z0-9_-]{1,64}$/.test(row.nonce) ||
    typeof row.producerId !== 'string' ||
    !/^[A-Za-z0-9_-]{1,128}$/.test(row.producerId) ||
    !Number.isSafeInteger(row.generation) ||
    Number(row.generation) < 1 ||
    Number(row.generation) > 16 ||
    typeof row.outputBasename !== 'string' ||
    !/^[A-Za-z0-9_-]{1,160}\.jsonl$/.test(row.outputBasename) ||
    !Number.isSafeInteger(row.requestedSeconds) ||
    Number(row.requestedSeconds) < 1 ||
    Number(row.requestedSeconds) > 900
  )
    throw new Error(`captured ${label} admission witness unavailable`);
  return {
    nonce: row.nonce,
    producerId: row.producerId,
    generation: Number(row.generation),
    outputBasename: row.outputBasename,
    requestedSeconds: Number(row.requestedSeconds),
  };
}

function readWorkflowBindings(capture: Capture): NonNullable<NormalizedRun['workflowBindings']> {
  const supplied = capture.diagnostics?.workflow;
  if (supplied === undefined) return [];
  if (!Array.isArray(supplied) || supplied.length > MAX_SQL_TIMING_SOURCES)
    throw new Error('invalid or over-limit captured workflow artifact list');
  const names = new Set<string>();
  return supplied.map((value: unknown) => {
    if (!value || typeof value !== 'object' || Array.isArray(value))
      throw new Error('invalid captured workflow artifact');
    const row = value as Record<string, unknown>;
    if (
      typeof row.name !== 'string' ||
      !/^[a-zA-Z0-9][a-zA-Z0-9_.-]*$/.test(row.name) ||
      names.has(row.name) ||
      !Object.hasOwn(capture.resources?.before?.samples ?? {}, row.name)
    )
      throw new Error('unknown or duplicate captured workflow peer');
    names.add(row.name);
    try {
      const expectedAdmission = capturedAdmission(row, 'workflow');
      if (
        typeof row.artifactPath !== 'string' ||
        !isAbsolute(row.artifactPath) ||
        typeof row.artifactSha256 !== 'string' ||
        !/^[a-f0-9]{64}$/.test(row.artifactSha256) ||
        expectedAdmission === undefined
      )
        throw new Error('captured workflow admission or artifact witness unavailable');
      const artifact = readWorkflowArtifact(row.artifactPath);
      if (
        artifact.sha256 !== row.artifactSha256 ||
        artifact.sha256 !== row.sourceSha256 ||
        artifact.bytes.length !== row.bytes
      )
        throw new Error('captured workflow artifact bytes do not match its witness');
      if (!validSnapshot(capture.resources?.before) || !validSnapshot(capture.resources?.after))
        throw new Error('workflow resource snapshots unavailable');
      const binding = evaluateWorkflowBinding(
        artifact.bytes,
        capture.resources.before,
        capture.resources.after,
        row.name,
        expectedAdmission
      );
      return { peer: row.name, ...binding, coverageEligible: false as const };
    } catch (error) {
      return {
        peer: row.name,
        status: 'refused',
        admission: 'unavailable',
        nativeIdentity: 'unavailable',
        interval: 'unavailable',
        issues: [error instanceof Error ? error.message : 'workflow artifact read failed'],
        warnings: [
          'Workflow lifecycle coverage remains provisional; a quiet expiry is not complete workflow or cell proof',
        ],
        coverageEligible: false as const,
      };
    }
  });
}

export function deriveCoverageEvidence(
  peers: string[],
  endpoints: string[],
  cpuProfiles: unknown[],
  methods: unknown[],
  methodLatency: unknown[],
  ioProfiles: unknown[] = [],
  failures: unknown[] = []
): Record<CoverageCategory, CoverageResult> {
  const missing = (summary: string): CoverageResult => ({ status: 'missing', summary });
  const result = Object.fromEntries(
    COVERAGE_CATEGORIES.map(category => [category, missing(`${category} has no direct evidence`)])
  ) as Record<CoverageCategory, CoverageResult>;
  const profiles = cpuProfiles as {
    peer?: string;
    usable?: boolean;
    unknownFramePercent?: number;
    callerStackCoveragePercent?: number | null;
  }[];
  const uniqueProfiles = peers.every(
    peer => profiles.filter(profile => profile.peer === peer).length === 1
  );
  if (
    peers.length &&
    uniqueProfiles &&
    peers.every(peer => profiles.some(profile => profile.peer === peer && profile.usable))
  ) {
    const rows = peers.map(peer => profiles.find(profile => profile.peer === peer)!);
    result['cpu-leaf'] = rows.every(
      row => finite(row.unknownFramePercent) && row.unknownFramePercent <= 25
    )
      ? {
          status: 'present',
          summary: 'every peer has at least 75% resolved sampled self CPU',
          observed: rows,
        }
      : {
          status: 'insufficient',
          summary: 'one or more peers have less than 75% resolved sampled self CPU',
          observed: rows,
        };
    result['cpu-caller'] = rows.every(row => finite(row.callerStackCoveragePercent))
      ? rows.every(row => row.callerStackCoveragePercent! >= 75)
        ? {
            status: 'present',
            summary: 'every peer has at least 75% caller-stack coverage',
            observed: rows,
          }
        : {
            status: 'insufficient',
            summary: 'one or more peers have less than 75% caller-stack coverage',
            observed: rows,
          }
      : missing('caller-stack coverage is absent for one or more peers');
  }
  const endpointCovered = (rows: unknown[]): boolean =>
    endpoints.length > 0 &&
    endpoints.every(endpoint =>
      (rows as { endpoint?: string }[]).some(row => row.endpoint === endpoint)
    );
  if (endpointCovered(methods))
    result['method-counts'] = {
      status: 'present',
      summary: 'every endpoint has conductor-call counters',
    };
  if (endpointCovered(methodLatency))
    result['method-durations'] = {
      status: 'present',
      summary: 'every endpoint has conductor-call duration histograms',
    };
  const latencyRows = methodLatency as {
    endpoint?: string;
    labels?: Record<string, string>;
  }[];
  const outcomesCovered =
    endpoints.length > 0 &&
    endpoints.every(endpoint => {
      const outcomes = new Set(
        latencyRows.filter(row => row.endpoint === endpoint).map(row => row.labels?.outcome)
      );
      return outcomes.has('success') && outcomes.has('error');
    });
  if (outcomesCovered)
    result['method-outcomes'] = {
      status: 'present',
      summary:
        'every endpoint directly represents success and error completion outcomes; absent zero-initialized label series cannot prove capability',
    };
  const timeoutRows = failures as {
    endpoint?: string;
    name?: string;
    labels?: Record<string, string>;
  }[];
  const methodRows = methods as { endpoint?: string; labels?: Record<string, string> }[];
  const methodKey = (row: { endpoint?: string; labels?: Record<string, string> }): string =>
    [row.endpoint, row.labels?.zome, row.labels?.fn ?? row.labels?.method, row.labels?.class].join(
      '\u0000'
    );
  const timeoutKeys = new Set(
    timeoutRows
      .filter(
        row =>
          row.name === 'elohim_conductor_call_timeouts_total' &&
          row.labels?.source === 'websocket' &&
          Boolean(row.labels?.fn)
      )
      .map(methodKey)
  );
  if (methodRows.length > 0 && methodRows.every(row => timeoutKeys.has(methodKey(row))))
    result['method-timeouts'] = {
      status: 'present',
      summary: 'every endpoint has directly labeled per-method timeout counters',
    };
  const ioRows = ioProfiles as IoProfileSummary[];
  const uniqueIo =
    peers.length > 0 && peers.every(peer => ioRows.filter(row => row.peer === peer).length === 1);
  if (uniqueIo && peers.every(peer => ioRows.some(row => row.peer === peer && row.usable))) {
    const rows = peers.map(peer => ioRows.find(row => row.peer === peer)!);
    result['io-attribution'] = rows.every(
      row =>
        finite(row.resolvedLeafCoveragePercent) &&
        row.resolvedLeafCoveragePercent >= 75 &&
        finite(row.callerStackCoveragePercent) &&
        row.callerStackCoveragePercent >= 75
    )
      ? {
          status: 'present',
          summary: 'every peer has at least 75% resolved I/O leaf and caller-stack coverage',
          observed: rows,
        }
      : {
          status: 'insufficient',
          summary:
            'one or more peers have less than 75% resolved I/O leaf or caller-stack coverage',
          observed: rows,
        };
  }
  return result;
}

export function reportVerdict(run: NormalizedRun): Verdict {
  const bounded = {
    ...publicRun(run),
    methods: run.methods.slice(0, 20),
    methodLatency: run.methodLatency.slice(0, 20),
    databaseLatency: run.databaseLatency.slice(0, 20),
    atomLatency: run.atomLatency.slice(0, 20),
    failures: run.failures.slice(0, 20),
    ioProfiles: run.ioProfiles.slice(0, 20).map(profile => sanitizeProfile(profile)),
    outputOmitted: {
      methods: Math.max(0, run.methods.length - 20),
      methodLatency: Math.max(0, run.methodLatency.length - 20),
      databaseLatency: Math.max(0, run.databaseLatency.length - 20),
      atomLatency: Math.max(0, run.atomLatency.length - 20),
      failures: Math.max(0, run.failures.length - 20),
    },
  };
  return {
    axis: 'runtime-performance',
    subject: null,
    decision: {
      type: 'refer',
      layer: 'operator',
      reason: 'insufficient-authority',
      note: 'Descriptive report only; no acceptance threshold was supplied.',
    },
    witness: {
      checks: [
        {
          checkId: 'runtime-performance',
          outcome: 'skipped',
          summary: 'Descriptive observation; no numeric acceptance verdict.',
          observed: bounded,
        },
      ],
    },
    policyRef: null,
  };
}

function sanitizeEvidence(evidence: ExternalEvidence | undefined): ExternalEvidence | undefined {
  return evidence
    ? {
        ...evidence,
        issues: evidence.issues.length
          ? [`${evidence.issues.length} issue(s); inspect private evidence inputs`]
          : [],
      }
    : undefined;
}

function publicRun(run: NormalizedRun): NormalizedRun {
  return {
    ...run,
    source: '[private capture]',
    env: undefined,
    peersDetail: run.peersDetail.map(row => ({
      peer: row.peer,
      cpuSeconds: row.cpuSeconds,
      rssKiBBefore: row.rssKiBBefore,
      rssKiBAfter: row.rssKiBAfter,
      io: row.io,
      cpuCores: row.cpuCores,
      ioBytesPerSecond: row.ioBytesPerSecond,
    })),
    issues: run.issues.length ? [`${run.issues.length} issue(s); inspect private capture`] : [],
    cpuProfiles: run.cpuProfiles.map(profile => sanitizeProfile(profile)),
    ioProfiles: run.ioProfiles.map(profile => sanitizeProfile(profile)),
    networkWatermarks: run.networkWatermarks,
    sqlTimingBindings: run.sqlTimingBindings?.map(row => ({
      peer: row.peer,
      status: row.status,
      admission: row.admission,
      nativeIdentity: row.nativeIdentity,
      interval: row.interval,
      coverageEligible: false,
      issues: row.issues.length
        ? [`${row.issues.length} binding issue(s); inspect private capture`]
        : [],
    })),
    workflowBindings: run.workflowBindings?.map(row => ({
      peer: row.peer,
      status: row.status,
      admission: row.admission,
      nativeIdentity: row.nativeIdentity,
      interval: row.interval,
      coverageEligible: false,
      issues: row.issues.length
        ? [`${row.issues.length} binding issue(s); inspect private capture`]
        : [],
      warnings: [
        'Provisional only: a bounded quiet expiry is not complete workflow or cell coverage',
      ],
    })),
    coverageEvidence: Object.fromEntries(
      Object.entries(run.coverageEvidence).map(([category, evidence]) => [
        category,
        {
          ...evidence,
          ...(Array.isArray(evidence.observed)
            ? { observed: evidence.observed.map(value => sanitizeProfile(value)) }
            : {}),
        },
      ])
    ) as Record<CoverageCategory, CoverageResult>,
    externalEvidence: sanitizeEvidence(run.externalEvidence),
  };
}

function sanitizeProfile(profile: unknown): unknown {
  if (!profile || typeof profile !== 'object') return profile;
  const copy = { ...(profile as Record<string, unknown>) };
  for (const key of ['artifact', 'source', 'executable', 'configPath', 'identity'])
    delete copy[key];
  for (const key of ['reason', 'callerAnalysisIssue']) {
    const value = copy[key];
    if (typeof value === 'string' && /[/\\]/.test(value))
      copy[key] = 'details retained in private capture';
  }
  return copy;
}

export function coverageVerdict(run: NormalizedRun, required: CoverageCategory[]): Verdict {
  const identityReady =
    run.comparable &&
    run.cohort !== null &&
    run.peers.length === Object.keys(run.binarySha256ByPeer).length;
  const results = Object.fromEntries(
    required.map(category => {
      const evidence = run.coverageEvidence[category];
      return [
        category,
        {
          ...evidence,
          ...(Array.isArray(evidence.observed)
            ? { observed: evidence.observed.map(value => sanitizeProfile(value)) }
            : {}),
        },
      ];
    })
  );
  const missing = required.filter(category => run.coverageEvidence[category].status === 'missing');
  const insufficient = required.filter(
    category => run.coverageEvidence[category].status === 'insufficient'
  );
  if (!identityReady || missing.length) {
    const reasons = [
      ...(identityReady ? [] : ['exact cohort/fingerprint/comparability evidence is absent']),
      ...missing.map(category => `${category}: ${run.coverageEvidence[category].summary}`),
    ];
    return {
      axis: 'runtime-performance',
      subject: null,
      decision: {
        type: 'refer',
        layer: 'operator',
        reason: 'contested-evidence',
        note: reasons.join('; '),
      },
      witness: {
        checks: [
          {
            checkId: 'runtime-performance',
            outcome: 'skipped',
            summary: 'Required performance coverage is absent.',
            observed: {
              required,
              results,
              identityReady,
              externalEvidence: sanitizeEvidence(run.externalEvidence),
            },
          },
        ],
      },
      policyRef: `local-required-coverage:${required.join(',')}`,
    };
  }
  const failed = insufficient.length > 0;
  return {
    axis: 'runtime-performance',
    subject: null,
    decision: { type: failed ? 'refuse' : 'permit' },
    witness: {
      checks: [
        {
          checkId: 'runtime-performance',
          outcome: failed ? 'failed' : 'passed',
          summary: failed
            ? `Coverage evidence is insufficient: ${insufficient.join(', ')}`
            : 'All required performance coverage is directly evidenced.',
          observed: {
            required,
            results,
            identityReady,
            externalEvidence: sanitizeEvidence(run.externalEvidence),
          },
        },
      ],
    },
    policyRef: `local-required-coverage:${required.join(',')}`,
  };
}

function percentChange(baseline: number, candidate: number): number | null {
  return baseline === 0
    ? candidate === 0
      ? 0
      : null
    : ((candidate - baseline) / Math.abs(baseline)) * 100;
}

function seriesKey(row: unknown): string {
  const value = row as {
    endpoint?: string;
    name?: string;
    labels?: Record<string, string>;
    unit?: string;
  };
  return `${value.endpoint ?? ''}|${value.name ?? ''}|${Object.entries(value.labels ?? {})
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([key, label]) => `${key}=${JSON.stringify(label)}`)
    .join(',')}|${value.unit ?? ''}`;
}

function atomicChanges(baseline: NormalizedRun, candidate: NormalizedRun) {
  const changes: {
    series: string;
    measure: string;
    baseline: number;
    candidate: number;
    changePercent: number | null;
  }[] = [];
  const coverage: string[] = [];
  const compareRows = (
    family: string,
    beforeRows: unknown[],
    afterRows: unknown[],
    measures: string[]
  ) => {
    const before = new Map(beforeRows.map(row => [seriesKey(row), row as Record<string, unknown>]));
    const after = new Map(afterRows.map(row => [seriesKey(row), row as Record<string, unknown>]));
    for (const key of new Set([...before.keys(), ...after.keys()])) {
      const left = before.get(key);
      const right = after.get(key);
      if (!left || !right) {
        coverage.push(`${family} series ${left ? 'disappeared' : 'appeared'}: ${key}`);
        continue;
      }
      for (const measure of measures) {
        const baselineValue = left[measure];
        const candidateValue = right[measure];
        if (finite(baselineValue) && finite(candidateValue))
          changes.push({
            series: key,
            measure,
            baseline: baselineValue,
            candidate: candidateValue,
            changePercent: percentChange(baselineValue, candidateValue),
          });
      }
    }
  };
  compareRows('method', baseline.methods, candidate.methods, ['perMinute']);
  compareRows('method-latency', baseline.methodLatency, candidate.methodLatency, ['mean', 'p95']);
  compareRows('database-source-site-latency', baseline.databaseLatency, candidate.databaseLatency, [
    'mean',
    'p95',
  ]);
  compareRows('atom-latency', baseline.atomLatency, candidate.atomLatency, ['mean', 'p95']);
  compareRows('failure', baseline.failures, candidate.failures, ['perMinute']);
  return { changes, coverage };
}

export function compareVerdict(
  baseline: NormalizedRun,
  candidate: NormalizedRun,
  threshold: number
): Verdict {
  const blockers = [
    ...baseline.comparabilityIssues.map(value => `baseline: ${value}`),
    ...candidate.comparabilityIssues.map(value => `candidate: ${value}`),
  ];
  if (baseline.cohort !== candidate.cohort) blockers.push('telemetry cohorts differ');
  if (baseline.peers.join('\0') !== candidate.peers.join('\0')) blockers.push('peer sets differ');
  const atomic = atomicChanges(baseline, candidate);
  if (!baseline.methods.length && !baseline.atomLatency.length)
    blockers.push('baseline has no method-call or atom-latency telemetry');
  if (!candidate.methods.length && !candidate.atomLatency.length)
    blockers.push('candidate has no method-call or atom-latency telemetry');
  blockers.push(...atomic.coverage);
  const changes = {
    cpuCores: percentChange(baseline.totals.cpuCores, candidate.totals.cpuCores),
    rssEndKiB: percentChange(baseline.totals.rssEndKiB, candidate.totals.rssEndKiB),
    ioBytesPerSecond: percentChange(
      baseline.totals.ioBytesPerSecond,
      candidate.totals.ioBytesPerSecond
    ),
  };
  const observed = {
    baseline: publicRun(baseline),
    candidate: publicRun(candidate),
    changesPercent: changes,
    atomicChanges: atomic.changes,
    binaryChanged:
      JSON.stringify(baseline.binarySha256ByPeer) !== JSON.stringify(candidate.binarySha256ByPeer),
    comparabilityIssues: blockers,
  };
  if (blockers.length) {
    return {
      axis: 'runtime-performance',
      subject: null,
      decision: {
        type: 'refer',
        layer: 'operator',
        reason: 'contested-evidence',
        note: blockers.join('; '),
      },
      witness: {
        checks: [
          {
            checkId: 'runtime-performance',
            outcome: 'skipped',
            summary: 'Captures are not comparable.',
            observed,
          },
        ],
      },
      policyRef: `local-threshold:max-regression-percent=${threshold}`,
    };
  }
  const evaluated = [
    ...Object.entries(changes).map(([name, changePercent]) => ({ name, changePercent })),
    ...atomic.changes.map(change => ({
      name: `${change.series}:${change.measure}`,
      changePercent: change.changePercent,
    })),
  ];
  const zeroBaselineIncreases = evaluated.filter(value => value.changePercent === null);
  const regressions = evaluated.filter(
    value => value.changePercent !== null && value.changePercent > threshold
  );
  const failed = regressions.length > 0 || zeroBaselineIncreases.length > 0;
  return {
    axis: 'runtime-performance',
    subject: null,
    decision: { type: failed ? 'refuse' : 'permit' },
    witness: {
      checks: [
        {
          checkId: 'runtime-performance',
          outcome: failed ? 'failed' : 'passed',
          summary: failed
            ? [
                regressions.length
                  ? `Regression exceeded ${threshold}%: ${regressions.map(value => value.name).join(', ')}`
                  : null,
                zeroBaselineIncreases.length
                  ? `Absolute zero-baseline guard refused 0→positive: ${zeroBaselineIncreases.map(value => value.name).join(', ')}`
                  : null,
              ]
                .filter(Boolean)
                .join('; ')
            : `All measured resource rates/endpoints are within ${threshold}%.`,
          observed,
        },
      ],
    },
    policyRef: `local-threshold:max-regression-percent=${threshold}`,
  };
}

export function pearson(pairs: [number, number][]): number | null {
  if (pairs.length < 5) return null;
  const meanX = pairs.reduce((sum, pair) => sum + pair[0], 0) / pairs.length;
  const meanY = pairs.reduce((sum, pair) => sum + pair[1], 0) / pairs.length;
  let xy = 0,
    xx = 0,
    yy = 0;
  for (const [x, y] of pairs) {
    const dx = x - meanX;
    const dy = y - meanY;
    xy += dx * dy;
    xx += dx * dx;
    yy += dy * dy;
  }
  return xx === 0 || yy === 0 ? null : xy / Math.sqrt(xx * yy);
}

export function trendVerdict(runs: NormalizedRun[]): Verdict {
  const anchor = runs[0];
  const eligible = runs.filter(
    run =>
      run.comparable &&
      anchor?.comparable &&
      run.cohort === anchor.cohort &&
      run.peers.join('\0') === anchor.peers.join('\0') &&
      run.window !== null
  );
  const seen = new Set<string>();
  const unique = eligible.filter(run => {
    const key = run.runId
      ? `run:${run.runId}`
      : `window:${run.window?.startUnixMs}|${run.window?.endUnixMs}|${run.cohort}|${run.peers.join(',')}|${JSON.stringify(run.binarySha256ByPeer)}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
  unique.sort((a, b) => a.window!.startUnixMs - b.window!.startUnixMs);
  const nonOverlapping = unique.every(
    (run, index) => index === 0 || unique[index - 1].window!.endUnixMs <= run.window!.startUnixMs
  );
  const correlationRuns = nonOverlapping ? unique : [];
  const pairs = correlationRuns.map(
    run => [run.totals.cpuCores, run.totals.rssGrowthKiB] as [number, number]
  );
  const observed = {
    runs: runs.map((run, index) => ({
      source: run.runId ?? `capture-${index + 1}`,
      runId: run.runId,
      cohort: run.cohort,
      peers: run.peers,
      binarySha256ByPeer: run.binarySha256ByPeer,
      comparable: run.comparable,
      comparabilityIssues: run.comparabilityIssues,
      elapsedSeconds: run.elapsedSeconds,
      totals: run.totals,
      coverageGaps: run.coverageGaps,
    })),
    correlations: [
      {
        axes: ['cpuCores', 'rssGrowthKiB'],
        pairCount: pairs.length,
        pearson: pearson(pairs),
        eligibility:
          pairs.length < 5
            ? 'requires at least five unique, proven non-overlapping comparable windows'
            : 'unique, proven non-overlapping comparable windows',
        interpretation: 'descriptive association only; never evidence of causation',
      },
    ],
  };
  return {
    axis: 'runtime-performance',
    subject: null,
    decision: {
      type: 'refer',
      layer: 'operator',
      reason: 'insufficient-authority',
      note: 'Trend is descriptive and has no ratified acceptance policy.',
    },
    witness: {
      checks: [
        {
          checkId: 'runtime-performance',
          outcome: 'skipped',
          summary: `Descriptive trend across ${runs.length} capture(s); ${pairs.length} mutually comparable pair(s).`,
          observed,
        },
      ],
    },
    policyRef: null,
  };
}

function markdownCell(value: unknown): string {
  return String(value ?? '')
    .replaceAll('|', String.raw`\|`)
    .split('')
    .map(character => {
      const code = character.codePointAt(0) ?? 0;
      return code < 32 || code === 127 ? ' ' : character;
    })
    .join('')
    .replace(/\s+/g, ' ')
    .slice(0, 240);
}

function sqlTimingMarkdown(evidence: ExternalEvidence): string[] {
  const windows = evidence.sqlTiming as {
    completeness?: unknown;
    terminalState?: unknown;
    siteMapping?: unknown;
    omittedStatements?: unknown;
    statements?: {
      statementRef?: unknown;
      count?: unknown;
      totalElapsedSeconds?: unknown;
      meanElapsedSeconds?: unknown;
      maxElapsedSeconds?: unknown;
      sourceSiteIds?: unknown;
      mappingCounts?: {
        unattributedEvents?: unknown;
        unmappedSourceSiteEvents?: unknown;
        sourceSiteOverflowEvents?: unknown;
      };
    }[];
  }[];
  if (!windows.length) return [];
  const lines = [
    '### SQL source-site drilldown (partial and capture-local)',
    '',
    'Statement references are capture-local and cannot be compared across windows. Source-site IDs are static labels, not per-site latency attribution. Rows are ranked statement aggregates from the bounded logger window; they are not complete database coverage, and visible rows do not sum to a whole-capture denominator.',
    '',
  ];
  let remaining = 20;
  const visibleWindows = windows.slice(0, 20);
  visibleWindows.forEach((window, index) => {
    const statements = Array.isArray(window.statements) ? window.statements : [];
    const visible = statements.slice(0, remaining);
    remaining -= visible.length;
    lines.push(
      `Window ${index + 1}: completeness **${markdownCell(window.completeness ?? 'unknown')}**; terminal **${markdownCell(window.terminalState ?? 'unknown')}**; source-site mapping **${markdownCell(window.siteMapping ?? 'unavailable')}**.`,
      ''
    );
    if (window.siteMapping !== 'available')
      lines.push('Legacy source-site mapping is unavailable for this window.', '');
    if (visible.length)
      lines.push(
        '| Statement ref | Static source-site IDs | Count | Total s | Mean s | Max s | Unattributed/missing | Unmapped | Mapping overflow |',
        '| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |',
        ...visible.map(statement => {
          const ids = Array.isArray(statement.sourceSiteIds)
            ? statement.sourceSiteIds.map(markdownCell).join(', ')
            : 'unavailable';
          const counts = statement.mappingCounts;
          return `| ${markdownCell(statement.statementRef)} | ${ids || 'none'} | ${markdownCell(statement.count)} | ${markdownCell(statement.totalElapsedSeconds)} | ${markdownCell(statement.meanElapsedSeconds)} | ${markdownCell(statement.maxElapsedSeconds)} | ${markdownCell(counts?.unattributedEvents ?? 'unavailable')} | ${markdownCell(counts?.unmappedSourceSiteEvents ?? 'unavailable')} | ${markdownCell(counts?.sourceSiteOverflowEvents ?? 'unavailable')} |`;
        }),
        ''
      );
    const omitted =
      Math.max(0, statements.length - visible.length) +
      (typeof window.omittedStatements === 'number' &&
      Number.isSafeInteger(window.omittedStatements)
        ? Math.max(0, window.omittedStatements)
        : 0);
    if (omitted) lines.push(`${omitted} additional statement row(s) omitted.`, '');
  });
  if (windows.length > visibleWindows.length)
    lines.push(
      `${windows.length - visibleWindows.length} additional SQL timing window(s) omitted.`,
      ''
    );
  return lines;
}

export function renderMarkdown(verdict: Verdict): string {
  const check = verdict.witness.checks[0];
  const observed = check?.observed as
    | {
        totals?: NormalizedRun['totals'];
        peersDetail?: NormalizedRun['peersDetail'];
        changesPercent?: Record<string, number | null>;
        runs?: { source: string; totals: NormalizedRun['totals'] }[];
        correlations?: unknown[];
        coverageGaps?: string[];
        methods?: unknown[];
        methodLatency?: unknown[];
        databaseLatency?: unknown[];
        atomLatency?: unknown[];
        failures?: unknown[];
        cpuProfiles?: unknown[];
        ioProfiles?: unknown[];
        heapSummary?: unknown;
        sqlTimingBindings?: NormalizedRun['sqlTimingBindings'];
        workflowBindings?: NormalizedRun['workflowBindings'];
        externalEvidence?: ExternalEvidence;
        required?: string[];
        results?: Record<string, { status?: string; summary?: string }>;
        identityReady?: boolean;
      }
    | undefined;
  const lines = [
    '# Runtime performance',
    '',
    `Decision: **${verdict.decision.type}**`,
    '',
    check?.summary ?? 'No check.',
    '',
  ];
  if (observed?.required && observed.results) {
    lines.push(
      '## Required coverage',
      '',
      `Identity ready: **${observed.identityReady === true ? 'yes' : 'no'}**`,
      '',
      '| Category | Status | Evidence |',
      '| --- | --- | --- |',
      ...observed.required.map(category => {
        const result = observed.results?.[category];
        const safe = (value: string): string =>
          value.replaceAll('|', String.raw`\|`).replace(/[\r\n]+/g, ' ');
        return `| ${safe(category)} | ${safe(result?.status ?? 'missing')} | ${safe(result?.summary ?? 'No evidence summary.')} |`;
      }),
      ''
    );
  }
  if (observed?.totals) {
    lines.push(
      '## Resources',
      '',
      '| CPU cores | RSS end KiB | RSS growth KiB | I/O bytes/s |',
      '| ---: | ---: | ---: | ---: |',
      `| ${observed.totals.cpuCores.toFixed(3)} | ${observed.totals.rssEndKiB} | ${observed.totals.rssGrowthKiB} | ${observed.totals.ioBytesPerSecond.toFixed(1)} |`,
      ''
    );
  }
  if (observed?.peersDetail?.length) {
    lines.push(
      '## Resources by peer',
      '',
      '| Peer | CPU cores | RSS end KiB | RSS growth KiB | I/O bytes/s |',
      '| --- | ---: | ---: | ---: | ---: |',
      ...observed.peersDetail.map(peer => {
        const escapedPeer = peer.peer.replaceAll('|', String.raw`\|`);
        return `| ${escapedPeer} | ${peer.cpuCores.toFixed(3)} | ${peer.rssKiBAfter} | ${peer.rssKiBAfter - peer.rssKiBBefore} | ${peer.ioBytesPerSecond.toFixed(1)} |`;
      }),
      ''
    );
  }
  if (observed?.changesPercent)
    lines.push(
      '## Change from baseline (%)',
      '',
      '```json',
      JSON.stringify(observed.changesPercent, null, 2),
      '```',
      ''
    );
  if (observed?.runs)
    lines.push(
      '## Windows',
      '',
      ...observed.runs.map(
        (run, index) =>
          `${index + 1}. ${run.source}: CPU ${run.totals.cpuCores.toFixed(3)} cores; RSS end ${run.totals.rssEndKiB} KiB; I/O ${run.totals.ioBytesPerSecond.toFixed(1)} B/s`
      ),
      '',
      '## Correlations',
      '',
      'Descriptive only; correlation never establishes causation.',
      '',
      '```json',
      JSON.stringify(observed.correlations, null, 2),
      '```',
      ''
    );
  if (observed?.methods?.length)
    lines.push(
      '## Methods by calls/minute',
      '',
      '```json',
      JSON.stringify(observed.methods, null, 2),
      '```',
      ''
    );
  if (observed?.methodLatency?.length)
    lines.push(
      '## Completed conductor-call latency',
      '',
      'Completion outcomes are kept distinct by labels. Dropped awaits are censored observations and appear only in failure counters, never in this histogram.',
      '',
      '```json',
      JSON.stringify(observed.methodLatency, null, 2),
      '```',
      ''
    );
  if (observed?.databaseLatency?.length)
    lines.push(
      '## Database source-site latency',
      '',
      'These rows describe completed queries grouped by source call-site and outcome. A source site may issue multiple SQL statements; this is not atomic SQL identity or a complete SQL distribution.',
      '',
      '```json',
      JSON.stringify(observed.databaseLatency, null, 2),
      '```',
      ''
    );
  if (observed?.sqlTimingBindings?.length)
    lines.push(
      '## SQL capture binding',
      '',
      'Recomputed from bounded artifact bytes and native process/monotonic-clock witnesses. A bound window does not establish complete statement coverage or CPU attribution.',
      '',
      '| Peer | Binding | Admission | Process identity | Window |',
      '| --- | --- | --- | --- | --- |',
      ...observed.sqlTimingBindings.map(
        row =>
          `| ${markdownCell(row.peer)} | ${markdownCell(row.status)} | ${markdownCell(row.admission ?? 'not supplied')} | ${markdownCell(row.nativeIdentity)} | ${markdownCell(row.interval)} |`
      ),
      ''
    );
  if (observed?.workflowBindings?.length)
    lines.push(
      '## Workflow capture binding',
      '',
      'Recomputed from the bounded raw artifact, admitted producer tuple, and native process/monotonic-clock witnesses. Binding is provisional: a quiet expiry is not complete workflow or cell coverage.',
      '',
      '| Peer | Binding | Admission | Process identity | Window |',
      '| --- | --- | --- | --- | --- |',
      ...observed.workflowBindings.map(
        row =>
          `| ${markdownCell(row.peer)} | ${markdownCell(row.status)} | ${markdownCell(row.admission)} | ${markdownCell(row.nativeIdentity)} | ${markdownCell(row.interval)} |`
      ),
      ''
    );
  if (observed?.externalEvidence)
    lines.push(
      '## External evidence',
      '',
      'Bounded descriptive observations only. Slow-only SQL, native SQL timing windows (logger-scoped and provisional), unknown-source logs, and identity-unmatched network snapshots do not satisfy required coverage.',
      '',
      '```json',
      JSON.stringify(observed.externalEvidence, null, 2),
      '```',
      '',
      ...sqlTimingMarkdown(observed.externalEvidence)
    );
  if (observed?.atomLatency?.length)
    lines.push(
      '## Atom latency (separate from method calls)',
      '',
      '```json',
      JSON.stringify(observed.atomLatency, null, 2),
      '```',
      ''
    );
  if (observed?.failures?.length)
    lines.push(
      '## HTTP status, shed, and timeout counters',
      '',
      '```json',
      JSON.stringify(observed.failures, null, 2),
      '```',
      ''
    );
  if (observed?.cpuProfiles?.length)
    lines.push(
      '## Sampled CPU self-rank and stack coverage',
      '',
      'Sample counts are not invocations. Unusable profiles and withheld caller clusters retain their exclusion reason.',
      '',
      '### Known caller prefixes (descriptive; not acceptance)',
      '',
      'A known prefix stops at the first unknown caller frame. It never bridges that gap to a known higher ancestor, is not a complete caller stack, and does not contribute to the 75% full-caller coverage requirement.',
      '',
      '```json',
      JSON.stringify(observed.cpuProfiles, null, 2),
      '```',
      ''
    );
  if (observed?.ioProfiles?.length)
    lines.push(
      '## User-syscall I/O attribution',
      '',
      'Bytes are successful user syscall return bytes, including files, sockets, and logging; they are not physical/device I/O. Counts are syscall observations, not method invocations. Raw arguments, descriptor paths, and payloads are never included.',
      '',
      '```json',
      JSON.stringify(observed.ioProfiles, null, 2),
      '```',
      ''
    );
  if (observed?.heapSummary)
    lines.push(
      '## Retained heap diff (descriptive only)',
      '',
      'Trusted-local jemalloc dumps do not establish native process/binary identity. This summary cannot satisfy heap-attribution coverage without a matched capture envelope.',
      '',
      '```json',
      JSON.stringify(observed.heapSummary, null, 2),
      '```',
      ''
    );
  const gaps = observed?.coverageGaps;
  if (gaps?.length) lines.push('## Coverage gaps', '', ...gaps.map(gap => `- ${gap}`), '');
  return `${lines.join('\n')}\n`;
}

export function validateVerdict(verdict: Verdict): void {
  const schemaPath = resolve(
    fileURLToPath(
      new URL('../../../../elohim/sdk/schemas/v1/views/verdict-view.schema.json', import.meta.url)
    )
  );
  const enumPath = resolve(
    fileURLToPath(
      new URL('../../../../elohim/sdk/schemas/v1/enums/decision.schema.json', import.meta.url)
    )
  );
  const schema = JSON.parse(readFileSync(schemaPath, 'utf8')) as object;
  const decision = JSON.parse(readFileSync(enumPath, 'utf8')) as object;
  const ajv = new AjvCtor({ allErrors: true, strict: true });
  ajv.addSchema(decision, 'epr:enums/decision.schema.json');
  const validate = ajv.compile(schema);
  if (!validate(verdict))
    throw new Error(
      `generated Verdict failed canonical schema: ${ajv.errorsText(validate.errors)}`
    );
}
