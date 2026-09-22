import { createHash } from 'node:crypto';
import {
  closeSync,
  constants,
  fstatSync,
  lstatSync,
  openSync,
  readSync,
  writeSync,
  type BigIntStats,
} from 'node:fs';
import { basename, join } from 'node:path';
import { performance } from 'node:perf_hooks';

import { inspectWorkflowArtifact } from './performance-evidence.js';

import type {
  ProcessResourceSample,
  ResourceSnapshot,
} from '../../src/framework/fixtures/process-resources.js';

export const MAX_SQL_TIMING_SOURCES = 8;
export const MAX_SQL_TIMING_BYTES = 2 * 1024 * 1024;
export const SQL_TIMING_WAIT_MS = 5000;
export const SQL_TIMING_POLL_MS = 250;
export const MAX_WORKFLOW_SOURCES = MAX_SQL_TIMING_SOURCES;
export const MAX_WORKFLOW_BYTES = MAX_SQL_TIMING_BYTES;
export const WORKFLOW_WAIT_MS = SQL_TIMING_WAIT_MS;
export const WORKFLOW_POLL_MS = SQL_TIMING_POLL_MS;
const PATH_REPLACED = 'private diagnostic source path replaced';
const MAX_BINDING_ISSUES = 32;
const SQL_TIMING_LABEL = 'SQL timing';
const WORKFLOW_LABEL = 'workflow';
const NOT_ENCLOSED = 'not-enclosed';

type BindingStatus = 'bound' | 'refused' | 'incomplete';
type NativeIdentityBinding = 'exact' | 'unavailable' | 'mismatch';
type IntervalBinding = 'encloses' | typeof NOT_ENCLOSED | 'unavailable';
type AdmissionBinding = 'exact' | 'mismatch' | 'unavailable';

export interface SqlTimingTarget {
  name: string;
  sourcePath: string;
  admission?: DiagnosticAdmissionBinding;
}

export interface SqlTimingSource {
  target: SqlTimingTarget;
  fd: number;
  opened: FileIdentity;
  openedBytes: number;
}

export interface DiagnosticAdmissionBinding {
  nonce: string;
  producerId: string;
  generation: number;
  outputBasename: string;
  requestedSeconds: number;
}

export type WorkflowAdmissionBinding = DiagnosticAdmissionBinding;

export interface WorkflowTarget extends SqlTimingTarget {
  admission: DiagnosticAdmissionBinding;
}

export interface WorkflowSource {
  target: WorkflowTarget;
  fd: number;
  opened: FileIdentity;
  openedBytes: number;
}

interface FileIdentity {
  dev: string;
  ino: string;
}

interface FileSnapshot extends FileIdentity {
  bytes: number;
  mtimeNs: string;
  content: Buffer;
}

interface NativeProcessIdentity {
  schema: 1;
  processId: number;
  processStartTicks: number;
  bootId: string;
  executable: string;
  clock: 'CLOCK_MONOTONIC';
}

interface TimingRecord {
  position: number;
  diagnosticSchema: number | null;
  diagnosticKind: string;
  processId: number | null;
  processIdMalformed: boolean;
  nativeProcess: NativeProcessIdentity | null;
  nativeProcessMalformed: boolean;
  startedMonotonicMs: number | null;
  startedMalformed: boolean;
  endedMonotonicMs: number | null;
  endedMalformed: boolean;
  requestedSeconds: number | null;
  requestedMalformed: boolean;
  eventLimit: number | null;
  eventLimitMalformed: boolean;
  generation: number | null;
  generationMalformed: boolean;
  nonce: string | null;
  outputBasename: string | null;
  producerId: string | null;
  closureReason: string | null;
  coverageComplete: boolean | null;
}

export interface SqlTimingDiagnostic {
  name: string;
  sourcePath: string;
  artifactPath: string | null;
  sourceSha256: string | null;
  artifactSha256: string | null;
  bytes: number | null;
  status: BindingStatus;
  nativeIdentity: NativeIdentityBinding;
  interval: IntervalBinding;
  issues: string[];
  warnings: string[];
  admission?: AdmissionBinding;
  nonce?: string;
  producerId?: string;
  generation?: number;
  outputBasename?: string;
  requestedSeconds?: number;
}

export interface WorkflowDiagnostic extends SqlTimingDiagnostic {
  admission: AdmissionBinding;
  nonce: string;
  producerId: string;
  generation: number;
  outputBasename: string;
  requestedSeconds: number;
  coverageEligible: false;
}

export interface SqlTimingBindingEvaluation {
  status: 'bound' | 'refused' | 'incomplete';
  nativeIdentity: SqlTimingDiagnostic['nativeIdentity'];
  interval: SqlTimingDiagnostic['interval'];
  issues: string[];
  warnings: string[];
  admission?: AdmissionBinding;
}

export interface WorkflowBindingEvaluation extends SqlTimingBindingEvaluation {
  admission: WorkflowDiagnostic['admission'];
}

export interface SqlTimingArtifact {
  bytes: Buffer;
  sha256: string;
}

export interface SqlTimingBindingOptions {
  now?: () => number;
  sleep?: (milliseconds: number) => Promise<void>;
  waitMs?: number;
  pollMs?: number;
  signal?: AbortSignal;
}

function parseTargetName(name: string): string {
  if (!/^[a-zA-Z0-9][a-zA-Z0-9_.-]*$/.test(name))
    throw new Error(`invalid SQL timing source name: ${name}`);
  return name;
}

export function parseSqlTimingTarget(raw: string): SqlTimingTarget {
  const separator = raw.indexOf('=');
  if (separator < 1 || separator === raw.length - 1)
    throw new Error('--sql-timing expects NAME=/absolute/private-native-jsonl');
  const name = parseTargetName(raw.slice(0, separator));
  const sourcePath = raw.slice(separator + 1);
  if (!sourcePath.startsWith('/'))
    throw new Error(`SQL timing source for ${name} must be an absolute path`);
  return { name, sourcePath };
}

function statIdentity(stat: BigIntStats): FileSnapshot {
  if (!stat.isFile()) throw new Error('private diagnostic source is not a regular file');
  const bytes = Number(stat.size);
  if (!Number.isSafeInteger(bytes) || bytes < 0 || bytes > MAX_SQL_TIMING_BYTES)
    throw new Error(`private diagnostic source exceeds ${MAX_SQL_TIMING_BYTES}-byte bound`);
  return {
    dev: stat.dev.toString(),
    ino: stat.ino.toString(),
    bytes,
    mtimeNs: stat.mtimeNs.toString(),
    content: Buffer.alloc(0),
  };
}

function sameFile(left: FileIdentity, right: FileIdentity): boolean {
  return left.dev === right.dev && left.ino === right.ino;
}

function readSource(source: SqlTimingSource | WorkflowSource): FileSnapshot {
  const before = statIdentity(fstatSync(source.fd, { bigint: true }));
  if (!sameFile(source.opened, before)) throw new Error('private diagnostic source inode changed');
  if (before.bytes < source.openedBytes) throw new Error('private diagnostic source truncated');
  const pathStat = lstatSync(source.target.sourcePath, { bigint: true });
  const pathIdentity = statIdentity(pathStat);
  if (!sameFile(source.opened, pathIdentity)) throw new Error(PATH_REPLACED);
  if (pathIdentity.bytes !== before.bytes || pathIdentity.mtimeNs !== before.mtimeNs)
    throw new Error('private diagnostic source changed before read');

  const chunks: Buffer[] = [];
  let total = 0;
  while (total < before.bytes) {
    const chunk = Buffer.alloc(Math.min(64 * 1024, before.bytes - total));
    const count = readSync(source.fd, chunk, 0, chunk.length, total);
    if (count <= 0) throw new Error('private diagnostic source truncated while reading');
    total += count;
    chunks.push(chunk.subarray(0, count));
  }
  const after = statIdentity(fstatSync(source.fd, { bigint: true }));
  if (!sameFile(source.opened, after)) throw new Error('private diagnostic source inode changed');
  if (after.bytes !== before.bytes || after.mtimeNs !== before.mtimeNs)
    throw new Error('private diagnostic source changed while reading');
  const finalPath = statIdentity(lstatSync(source.target.sourcePath, { bigint: true }));
  if (!sameFile(source.opened, finalPath)) throw new Error(PATH_REPLACED);
  if (finalPath.bytes !== after.bytes || finalPath.mtimeNs !== after.mtimeNs)
    throw new Error('private diagnostic source changed after read');
  return { ...after, content: Buffer.concat(chunks, total) };
}

export function readSqlTimingArtifact(path: string): SqlTimingArtifact {
  return readPrivateDiagnosticArtifact(path, SQL_TIMING_LABEL);
}

export function readWorkflowArtifact(path: string): SqlTimingArtifact {
  return readPrivateDiagnosticArtifact(path, WORKFLOW_LABEL);
}

function readPrivateDiagnosticArtifact(path: string, label: string): SqlTimingArtifact {
  if (!path.startsWith('/')) throw new Error(`${label} artifact path must be absolute`);
  const fd = openSync(path, constants.O_RDONLY | constants.O_NONBLOCK | constants.O_NOFOLLOW);
  const source: SqlTimingSource = {
    target: { name: 'artifact', sourcePath: path },
    fd,
    opened: { dev: '', ino: '' },
    openedBytes: 0,
  };
  try {
    const opened = statIdentity(fstatSync(fd, { bigint: true }));
    const pathIdentity = statIdentity(lstatSync(path, { bigint: true }));
    if (!sameFile(opened, pathIdentity)) throw new Error(PATH_REPLACED);
    source.opened = opened;
    source.openedBytes = opened.bytes;
    const snapshot = readSource(source);
    return {
      bytes: snapshot.content,
      sha256: createHash('sha256').update(snapshot.content).digest('hex'),
    };
  } finally {
    closeSync(fd);
  }
}

export function openSqlTimingSources(
  targets: SqlTimingTarget[],
  expectedPeers: Record<string, string>
): { sources: SqlTimingSource[]; diagnostics: SqlTimingDiagnostic[] } {
  for (const target of targets) {
    if (target.admission && basename(target.sourcePath) !== target.admission.outputBasename)
      throw new Error(`${target.name}: SQL timing source does not match admitted basename`);
  }
  return {
    sources: openDiagnosticSources(targets, expectedPeers, SQL_TIMING_LABEL),
    diagnostics: [],
  };
}

function openDiagnosticSources<
  T extends SqlTimingTarget,
  S extends { target: T; fd: number; opened: FileIdentity; openedBytes: number },
>(targets: T[], expectedPeers: Record<string, string>, label: string): S[] {
  if (targets.length > MAX_SQL_TIMING_SOURCES)
    throw new Error(`at most ${MAX_SQL_TIMING_SOURCES} ${label} values are allowed`);
  const names = new Set<string>();
  const sources: S[] = [];
  try {
    for (const target of targets) {
      parseTargetName(target.name);
      if (!target.sourcePath.startsWith('/'))
        throw new Error(`${label} source for ${target.name} must be an absolute path`);
      if (names.has(target.name)) throw new Error(`${label} names must be unique`);
      names.add(target.name);
      if (!Object.hasOwn(expectedPeers, target.name))
        throw new Error(`${label} name does not match a conductor config: ${target.name}`);
      const fd = openSync(
        target.sourcePath,
        constants.O_RDONLY | constants.O_NONBLOCK | constants.O_NOFOLLOW
      );
      try {
        const inspected = statIdentity(fstatSync(fd, { bigint: true }));
        const pathStat = statIdentity(lstatSync(target.sourcePath, { bigint: true }));
        if (!sameFile(inspected, pathStat)) throw new Error(PATH_REPLACED);
        sources.push({
          target,
          fd,
          opened: inspected,
          openedBytes: inspected.bytes,
        } as unknown as S);
      } catch (error) {
        closeSync(fd);
        throw error;
      }
    }
  } catch (error) {
    closeDiagnosticSources(sources);
    throw error;
  }
  return sources;
}

export function openWorkflowSources(
  targets: WorkflowTarget[],
  expectedPeers: Record<string, string>
): { sources: WorkflowSource[]; diagnostics: WorkflowDiagnostic[] } {
  for (const target of targets) {
    if (basename(target.sourcePath) !== target.admission.outputBasename)
      throw new Error(`${target.name}: workflow source does not match admitted basename`);
  }
  return {
    sources: openDiagnosticSources<WorkflowTarget, WorkflowSource>(
      targets,
      expectedPeers,
      WORKFLOW_LABEL
    ),
    diagnostics: [],
  };
}

export function closeSqlTimingSources(sources: SqlTimingSource[]): void {
  closeDiagnosticSources(sources);
}

export function closeWorkflowSources(sources: WorkflowSource[]): void {
  closeDiagnosticSources(sources);
}

function closeDiagnosticSources(sources: { fd: number }[]): void {
  for (const source of sources) {
    const descriptor = source.fd;
    source.fd = -1;
    if (descriptor < 0) continue;
    try {
      closeSync(descriptor);
    } catch {
      // The descriptor is private collector state; retain the binding issue instead.
    }
  }
}

function finiteNonNegative(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) && value >= 0 ? value : null;
}

function pushIssue(issues: string[], issue: string): void {
  if (!issues.includes(issue) && issues.length < MAX_BINDING_ISSUES) issues.push(issue);
}

function optionalFinite(value: unknown): { value: number | null; malformed: boolean } {
  if (value === null || value === undefined) return { value: null, malformed: false };
  const parsed = finiteNonNegative(value);
  return { value: parsed, malformed: parsed === null };
}

function hasControl(value: string): boolean {
  return [...value].some(character => {
    const code = character.codePointAt(0) ?? 0;
    return code < 0x20 || code === 0x7f;
  });
}

function nativeProcess(value: unknown): {
  value: NativeProcessIdentity | null;
  malformed: boolean;
} {
  if (value === null || value === undefined) return { value: null, malformed: false };
  if (!value || typeof value !== 'object' || Array.isArray(value))
    return { value: null, malformed: true };
  const row = value as Record<string, unknown>;
  const processId = row.process_id;
  const processStartTicks = row.process_start_ticks;
  const bootId = row.boot_id;
  const executable = row.executable;
  if (
    row.schema !== 1 ||
    !Number.isSafeInteger(processId) ||
    Number(processId) < 1 ||
    !Number.isSafeInteger(processStartTicks) ||
    Number(processStartTicks) < 0 ||
    typeof bootId !== 'string' ||
    !/^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(bootId) ||
    typeof executable !== 'string' ||
    !executable.startsWith('/') ||
    executable.length > 4096 ||
    hasControl(executable) ||
    row.clock !== 'CLOCK_MONOTONIC'
  )
    return { value: null, malformed: true };
  return {
    value: {
      schema: 1,
      processId: Number(processId),
      processStartTicks: Number(processStartTicks),
      bootId,
      executable,
      clock: 'CLOCK_MONOTONIC',
    },
    malformed: false,
  };
}

function timingRecord(value: unknown, position: number): TimingRecord | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
  const row = value as Record<string, unknown>;
  const kind = row.diagnostic_kind;
  if (kind !== 'sql_timing_window_started' && kind !== 'sql_timing_window_closed') return null;
  const processId = row.process_id;
  const parsedStarted = optionalFinite(row.started_monotonic_ms);
  const parsedEnded = optionalFinite(row.ended_monotonic_ms);
  const parsedRequested = optionalFinite(row.requested_seconds);
  const parsedEventLimit = optionalFinite(row.event_limit);
  const parsedGeneration = optionalFinite(row.generation);
  const native = nativeProcess(row.native_process);
  return {
    position,
    diagnosticSchema: typeof row.diagnostic_schema === 'number' ? row.diagnostic_schema : null,
    diagnosticKind: kind,
    processId:
      typeof processId === 'number' && Number.isSafeInteger(processId) && processId >= 1
        ? processId
        : null,
    processIdMalformed:
      processId !== undefined &&
      processId !== null &&
      !(typeof processId === 'number' && Number.isSafeInteger(processId) && processId >= 1),
    nativeProcess: native.value,
    nativeProcessMalformed: native.malformed,
    startedMonotonicMs: parsedStarted.value,
    startedMalformed: parsedStarted.malformed,
    endedMonotonicMs: parsedEnded.value,
    endedMalformed: parsedEnded.malformed,
    requestedSeconds: parsedRequested.value,
    requestedMalformed: parsedRequested.malformed,
    eventLimit: parsedEventLimit.value,
    eventLimitMalformed: parsedEventLimit.malformed,
    generation: parsedGeneration.value,
    generationMalformed: parsedGeneration.malformed,
    nonce:
      typeof row.nonce === 'string' && /^[A-Za-z0-9_-]{1,64}$/.test(row.nonce) ? row.nonce : null,
    outputBasename:
      typeof row.output_basename === 'string' &&
      /^[A-Za-z0-9_-]{1,160}\.jsonl$/.test(row.output_basename)
        ? row.output_basename
        : null,
    producerId:
      typeof row.producer_id === 'string' &&
      row.producer_id.length > 0 &&
      row.producer_id.length <= 128 &&
      row.producer_id === row.producer_id.trim() &&
      !hasControl(row.producer_id)
        ? row.producer_id
        : null,
    closureReason: typeof row.closure_reason === 'string' ? row.closure_reason : null,
    coverageComplete: typeof row.coverage_complete === 'boolean' ? row.coverage_complete : null,
  };
}

function records(content: Buffer, issues: string[]): TimingRecord[] {
  const output: TimingRecord[] = [];
  content
    .toString('utf8')
    .split('\n')
    .forEach((line, position) => {
      if (!line.trim()) return;
      try {
        const value = JSON.parse(line) as unknown;
        const record = timingRecord(value, position);
        if (record) output.push(record);
        else if (
          value &&
          typeof value === 'object' &&
          (value as Record<string, unknown>).diagnostic_kind !== undefined
        )
          pushIssue(issues, 'SQL timing source contains a malformed timing witness');
      } catch {
        pushIssue(issues, 'SQL timing source contains malformed JSON');
      }
    });
  return output;
}

function identityMatches(
  native: NativeProcessIdentity,
  sample: ProcessResourceSample | undefined,
  bootId: string
): boolean {
  if (!sample) return false;
  return (
    native.processId === sample.pid &&
    native.processStartTicks === sample.startTicks &&
    native.executable === sample.executable &&
    native.bootId === bootId
  );
}

function artifactPath(artifactsDir: string, name: string, suffix: string): string {
  return join(artifactsDir, `${basename(name)}.${suffix}.jsonl`);
}

function writePrivateArtifact(path: string, content: Buffer): string {
  if (content.byteLength > MAX_SQL_TIMING_BYTES)
    throw new Error(`private diagnostic artifact exceeds ${MAX_SQL_TIMING_BYTES}-byte bound`);
  const fd = openSync(path, constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL, 0o600);
  try {
    let offset = 0;
    while (offset < content.byteLength) {
      const count = writeSync(fd, content, offset, content.byteLength - offset);
      if (count <= 0) throw new Error('private diagnostic artifact write made no progress');
      offset += count;
    }
  } finally {
    closeSync(fd);
  }
  return createHash('sha256').update(content).digest('hex');
}

function sameNative(left: NativeProcessIdentity, right: NativeProcessIdentity): boolean {
  return (
    left.schema === right.schema &&
    left.processId === right.processId &&
    left.processStartTicks === right.processStartTicks &&
    left.bootId === right.bootId &&
    left.executable === right.executable &&
    left.clock === right.clock
  );
}

function requestedSecondsValid(record: TimingRecord): boolean {
  return (
    !record.requestedMalformed &&
    record.requestedSeconds !== null &&
    Number.isSafeInteger(record.requestedSeconds) &&
    record.requestedSeconds >= 1 &&
    record.requestedSeconds <= 900
  );
}

function completeSqlPair(content: Buffer): boolean {
  const parseIssues: string[] = [];
  const parsed = records(content, parseIssues);
  return (
    parsed.filter(record => record.diagnosticKind === 'sql_timing_window_started').length === 1 &&
    parsed.filter(record => record.diagnosticKind === 'sql_timing_window_closed').length === 1
  );
}

function completeWorkflowPair(content: Buffer): boolean {
  try {
    inspectWorkflowArtifact(content);
    return true;
  } catch {
    return false;
  }
}

/** Re-evaluate identity and interval binding without opening or writing files. */
export function evaluateSqlTimingBinding(
  bytes: Buffer,
  before: ResourceSnapshot,
  after: ResourceSnapshot,
  peer: string,
  expectedAdmission?: DiagnosticAdmissionBinding
): SqlTimingBindingEvaluation {
  const issues: string[] = [];
  const warnings: string[] = [];
  if (bytes.byteLength > MAX_SQL_TIMING_BYTES) {
    return {
      status: 'refused',
      nativeIdentity: 'unavailable',
      interval: 'unavailable',
      issues: [`SQL timing artifact exceeds ${MAX_SQL_TIMING_BYTES}-byte bound`],
      warnings,
      ...(expectedAdmission ? { admission: 'unavailable' as const } : {}),
    };
  }
  const parsed = records(bytes, issues);
  const starts = parsed.filter(record => record.diagnosticKind === 'sql_timing_window_started');
  const closes = parsed.filter(record => record.diagnosticKind === 'sql_timing_window_closed');
  if (starts.length !== 1 || closes.length !== 1) {
    issues.push('SQL timing source must contain exactly one start and one close record');
    return {
      status: 'refused',
      nativeIdentity: 'unavailable',
      interval: 'unavailable',
      issues,
      warnings,
      ...(expectedAdmission ? { admission: 'unavailable' as const } : {}),
    };
  }
  const start = starts[0];
  const close = closes[0];
  if (close.position <= start.position)
    issues.push('SQL timing close record is not ordered after start');
  if (start.diagnosticSchema !== 1 || close.diagnosticSchema !== 1)
    issues.push('SQL timing diagnostic_schema must be 1');
  if (start.processId === null || close.processId === null || start.processId !== close.processId)
    issues.push('SQL timing top-level process_id is missing or changed');
  if (start.processIdMalformed || close.processIdMalformed)
    issues.push('SQL timing top-level process_id is malformed');
  if (start.nativeProcessMalformed || close.nativeProcessMalformed)
    issues.push('SQL timing native process identity is malformed');
  if (!start.nativeProcess || !close.nativeProcess) {
    issues.push('SQL timing native process identity unavailable');
  } else if (!sameNative(start.nativeProcess, close.nativeProcess)) {
    issues.push('SQL timing native process identity changed between start and close');
  }
  if (
    start.nativeProcess &&
    close.nativeProcess &&
    start.processId !== null &&
    close.processId !== null &&
    start.processId === start.nativeProcess.processId &&
    close.processId === close.nativeProcess.processId
  ) {
    const matchesBefore = identityMatches(start.nativeProcess, before.samples[peer], before.bootId);
    const matchesAfter = identityMatches(start.nativeProcess, after.samples[peer], after.bootId);
    if (!matchesBefore || !matchesAfter)
      issues.push('SQL timing native identity does not exactly match resource snapshots');
  }
  if (
    start.nativeProcess &&
    close.nativeProcess &&
    (start.processId !== start.nativeProcess.processId ||
      close.processId !== close.nativeProcess.processId)
  )
    issues.push('SQL timing top-level process_id does not match native process identity');
  if (
    start.startedMalformed ||
    close.startedMalformed ||
    start.startedMonotonicMs === null ||
    close.startedMonotonicMs === null ||
    start.startedMonotonicMs !== close.startedMonotonicMs
  )
    issues.push('SQL timing started_monotonic_ms is missing, malformed, or changed');
  if (close.endedMalformed || close.endedMonotonicMs === null)
    issues.push('SQL timing ended_monotonic_ms is missing or malformed');
  if (!requestedSecondsValid(start) || !requestedSecondsValid(close))
    issues.push('SQL timing requested_seconds must be an integer in 1..900');
  else if (start.requestedSeconds !== close.requestedSeconds)
    issues.push('SQL timing requested_seconds changed between start and close');
  if (
    start.startedMonotonicMs !== null &&
    close.endedMonotonicMs !== null &&
    close.endedMonotonicMs < start.startedMonotonicMs
  )
    issues.push('SQL timing monotonic interval reversed');
  if (
    start.startedMonotonicMs !== null &&
    close.endedMonotonicMs !== null &&
    requestedSecondsValid(start) &&
    close.endedMonotonicMs > start.startedMonotonicMs + start.requestedSeconds! * 1000
  )
    issues.push('SQL timing end exceeds the requested active window');

  let nativeIdentity: SqlTimingDiagnostic['nativeIdentity'] = 'unavailable';
  if (start.nativeProcess && close.nativeProcess) {
    const topLevelIdsMatch =
      start.processId !== null &&
      close.processId !== null &&
      start.processId === start.nativeProcess.processId &&
      close.processId === close.nativeProcess.processId;
    nativeIdentity =
      sameNative(start.nativeProcess, close.nativeProcess) && topLevelIdsMatch
        ? 'exact'
        : 'mismatch';
    if (nativeIdentity === 'exact') {
      const matchesBefore = identityMatches(
        start.nativeProcess,
        before.samples[peer],
        before.bootId
      );
      const matchesAfter = identityMatches(start.nativeProcess, after.samples[peer], after.bootId);
      if (!matchesBefore || !matchesAfter) nativeIdentity = 'mismatch';
    }
  }

  let interval: SqlTimingDiagnostic['interval'] = 'unavailable';
  const beforeStartClock = finiteNonNegative(before.kernelMonotonicStartMs);
  const beforeEndClock = finiteNonNegative(before.kernelMonotonicMs);
  const afterStartClock = finiteNonNegative(after.kernelMonotonicStartMs);
  const afterEndClock = finiteNonNegative(after.kernelMonotonicMs);
  if (
    beforeStartClock === null ||
    beforeEndClock === null ||
    afterStartClock === null ||
    afterEndClock === null
  ) {
    issues.push('resource kernel monotonic interval unavailable for SQL timing enclosure');
  } else if (
    beforeEndClock < beforeStartClock ||
    afterEndClock < afterStartClock ||
    afterStartClock < beforeEndClock
  ) {
    issues.push('resource kernel monotonic brackets are reversed or overlap');
    interval = NOT_ENCLOSED;
  } else if (start.startedMonotonicMs !== null && close.endedMonotonicMs !== null) {
    if (start.startedMonotonicMs > beforeStartClock || close.endedMonotonicMs < afterEndClock) {
      interval = NOT_ENCLOSED;
      issues.push('SQL timing native interval does not enclose resource snapshots');
    } else interval = 'encloses';
  }
  if (close.closureReason && close.closureReason !== 'expiry')
    warnings.push(`SQL timing native window closed with ${close.closureReason}`);
  if (close.coverageComplete === false)
    warnings.push('SQL timing native window reports incomplete capture');
  if (
    start.producerId === null ||
    close.producerId === null ||
    start.producerId !== close.producerId
  )
    issues.push('SQL timing producer identity changed between start and close');

  let admission: AdmissionBinding | undefined;
  if (expectedAdmission) {
    const exactTuple = (record: TimingRecord): boolean =>
      !record.generationMalformed &&
      Number.isSafeInteger(record.generation) &&
      record.generation === expectedAdmission.generation &&
      record.nonce === expectedAdmission.nonce &&
      record.outputBasename === expectedAdmission.outputBasename &&
      record.producerId === expectedAdmission.producerId &&
      record.requestedSeconds === expectedAdmission.requestedSeconds;
    admission =
      exactTuple(start) &&
      exactTuple(close) &&
      !start.eventLimitMalformed &&
      start.eventLimit === 10_000
        ? 'exact'
        : 'mismatch';
    if (admission !== 'exact')
      issues.push(
        'SQL timing start/close do not match the admitted nonce, generation, producer, basename, duration, or event cap'
      );
  }

  return {
    status: issues.length ? 'incomplete' : 'bound',
    nativeIdentity,
    interval,
    issues,
    warnings,
    ...(admission ? { admission } : {}),
  };
}

/** Re-derive a private workflow admission, process identity, and active window from raw bytes. */
export function evaluateWorkflowBinding(
  bytes: Buffer,
  before: ResourceSnapshot,
  after: ResourceSnapshot,
  peer: string,
  expected: WorkflowAdmissionBinding
): WorkflowBindingEvaluation {
  const issues: string[] = [];
  const warnings: string[] = [
    'Workflow lifecycle coverage remains provisional; a quiet expiry is not complete workflow or cell proof',
  ];
  if (bytes.byteLength > MAX_WORKFLOW_BYTES) {
    return {
      status: 'refused',
      admission: 'unavailable',
      nativeIdentity: 'unavailable',
      interval: 'unavailable',
      issues: [`workflow artifact exceeds ${MAX_WORKFLOW_BYTES}-byte bound`],
      warnings,
    };
  }
  let witness: ReturnType<typeof inspectWorkflowArtifact>;
  try {
    witness = inspectWorkflowArtifact(bytes);
  } catch (error) {
    return {
      status: 'refused',
      admission: 'unavailable',
      nativeIdentity: 'unavailable',
      interval: 'unavailable',
      issues: [error instanceof Error ? error.message : 'workflow artifact parsing failed'],
      warnings,
    };
  }
  const admission: WorkflowDiagnostic['admission'] =
    witness.nonce === expected.nonce &&
    witness.generation === expected.generation &&
    witness.producerId === expected.producerId &&
    witness.outputBasename === expected.outputBasename &&
    witness.requestedSeconds === expected.requestedSeconds &&
    witness.eventLimit === 10_000
      ? 'exact'
      : 'mismatch';
  if (admission !== 'exact')
    issues.push(
      'workflow artifact does not match the admitted nonce, generation, producer, or basename'
    );

  const matchesBefore = identityMatches(witness.nativeProcess, before.samples[peer], before.bootId);
  const matchesAfter = identityMatches(witness.nativeProcess, after.samples[peer], after.bootId);
  const nativeIdentity: WorkflowDiagnostic['nativeIdentity'] =
    matchesBefore && matchesAfter ? 'exact' : 'mismatch';
  if (nativeIdentity !== 'exact')
    issues.push('workflow native identity does not exactly match resource snapshots');

  let interval: WorkflowDiagnostic['interval'] = 'unavailable';
  const beforeStartClock = finiteNonNegative(before.kernelMonotonicStartMs);
  const beforeEndClock = finiteNonNegative(before.kernelMonotonicMs);
  const afterStartClock = finiteNonNegative(after.kernelMonotonicStartMs);
  const afterEndClock = finiteNonNegative(after.kernelMonotonicMs);
  if (
    beforeStartClock === null ||
    beforeEndClock === null ||
    afterStartClock === null ||
    afterEndClock === null
  ) {
    issues.push('resource kernel monotonic interval unavailable for workflow enclosure');
  } else if (
    beforeEndClock < beforeStartClock ||
    afterEndClock < afterStartClock ||
    afterStartClock < beforeEndClock
  ) {
    interval = NOT_ENCLOSED;
    issues.push('resource kernel monotonic brackets are reversed or overlap');
  } else if (
    witness.startedMonotonicMs > beforeStartClock ||
    witness.endedMonotonicMs < afterEndClock
  ) {
    interval = NOT_ENCLOSED;
    issues.push('workflow native interval does not enclose resource snapshots');
  } else {
    interval = 'encloses';
  }
  if (witness.closureReason !== 'expiry')
    warnings.push(`workflow native window closed with ${witness.closureReason}`);
  return {
    status: issues.length ? 'incomplete' : 'bound',
    admission,
    nativeIdentity,
    interval,
    issues,
    warnings,
  };
}

async function finishSource(
  source: SqlTimingSource | WorkflowSource,
  before: ResourceSnapshot,
  after: ResourceSnapshot,
  artifactsDir: string,
  options: SqlTimingBindingOptions,
  config: {
    label: string;
    suffix: string;
    complete(content: Buffer): boolean;
    evaluate(content: Buffer): SqlTimingBindingEvaluation;
  }
): Promise<SqlTimingDiagnostic> {
  const issues: string[] = [];
  let final: FileSnapshot | null = null;
  const now = options.now ?? (() => performance.now());
  const sleep =
    options.sleep ??
    (async (milliseconds: number) =>
      await new Promise(resolve => setTimeout(resolve, milliseconds)));
  const waitMs = Math.min(Math.max(options.waitMs ?? SQL_TIMING_WAIT_MS, 0), SQL_TIMING_WAIT_MS);
  const pollMs = Math.max(options.pollMs ?? SQL_TIMING_POLL_MS, 1);
  const deadline = now() + waitMs;
  const maxIterations = Math.ceil(waitMs / pollMs) + 2;
  let iterations = 0;
  while (iterations < maxIterations) {
    if (options.signal?.aborted) {
      issues.push(`${config.label} source wait aborted`);
      break;
    }
    iterations += 1;
    try {
      final = readSource(source);
      if (config.complete(final.content)) break;
    } catch (error) {
      issues.push(error instanceof Error ? error.message : `${config.label} source read failed`);
      break;
    }
    if (now() >= deadline) break;
    const delay = Math.min(pollMs, Math.max(0, deadline - now()));
    if (await abortableSleep(delay, sleep, options.signal)) {
      issues.push(`${config.label} source wait aborted`);
      break;
    }
  }
  if (!final) {
    if (!issues.length) issues.push('SQL timing source did not provide a stable final read');
  } else if (!config.complete(final.content)) {
    issues.push(`${config.label} source did not provide a complete terminal before timeout`);
  }
  if (iterations >= maxIterations && final && !config.complete(final.content))
    issues.push(`${config.label} source wait reached its hard iteration cap`);

  let artifactPathValue: string | null = null;
  let artifactSha256: string | null = null;
  if (final) {
    artifactPathValue = artifactPath(artifactsDir, source.target.name, config.suffix);
    try {
      artifactSha256 = writePrivateArtifact(artifactPathValue, final.content);
    } catch (error) {
      issues.push(error instanceof Error ? error.message : `${config.label} artifact write failed`);
      artifactPathValue = null;
    }
  }
  const evaluation = final
    ? config.evaluate(final.content)
    : {
        status: 'refused' as const,
        nativeIdentity: 'unavailable' as const,
        interval: 'unavailable' as const,
        issues: [],
        warnings: [],
      };
  const sourceSha256 = final ? createHash('sha256').update(final.content).digest('hex') : null;
  const admission = (evaluation as Partial<WorkflowBindingEvaluation>).admission;
  return {
    name: source.target.name,
    sourcePath: source.target.sourcePath,
    artifactPath: artifactPathValue,
    sourceSha256,
    artifactSha256,
    bytes: final?.bytes ?? null,
    status: issues.length
      ? evaluation.status === 'refused'
        ? 'refused'
        : 'incomplete'
      : evaluation.status,
    nativeIdentity: evaluation.nativeIdentity,
    interval: evaluation.interval,
    issues: [...issues, ...evaluation.issues],
    warnings: evaluation.warnings,
    ...(admission ? { admission } : {}),
    ...(source.target.admission ?? {}),
  };
}

async function abortableSleep(
  milliseconds: number,
  sleep: (milliseconds: number) => Promise<void>,
  signal?: AbortSignal
): Promise<boolean> {
  if (!signal) {
    await sleep(milliseconds);
    return false;
  }
  if (signal.aborted) return true;
  let aborted = false;
  let resolveAbort: (() => void) | undefined;
  const abort = new Promise<void>(resolve => {
    resolveAbort = resolve;
  });
  const onAbort = (): void => {
    aborted = true;
    resolveAbort?.();
  };
  signal.addEventListener('abort', onAbort, { once: true });
  try {
    await Promise.race([sleep(milliseconds), abort]);
    return aborted || signal.aborted;
  } finally {
    signal.removeEventListener('abort', onAbort);
  }
}

export async function finishSqlTimingSources(
  sources: SqlTimingSource[],
  before: ResourceSnapshot,
  after: ResourceSnapshot,
  artifactsDir: string,
  options: SqlTimingBindingOptions = {}
): Promise<SqlTimingDiagnostic[]> {
  return await Promise.all(
    sources.map(
      async source =>
        await finishSource(source, before, after, artifactsDir, options, {
          label: SQL_TIMING_LABEL,
          suffix: 'sql-timing',
          complete: completeSqlPair,
          evaluate: content =>
            evaluateSqlTimingBinding(
              content,
              before,
              after,
              source.target.name,
              source.target.admission
            ),
        })
    )
  );
}

export async function finishWorkflowSources(
  sources: WorkflowSource[],
  before: ResourceSnapshot,
  after: ResourceSnapshot,
  artifactsDir: string,
  options: SqlTimingBindingOptions = {}
): Promise<WorkflowDiagnostic[]> {
  return await Promise.all(
    sources.map(async source => {
      const base = await finishSource(source, before, after, artifactsDir, options, {
        label: WORKFLOW_LABEL,
        suffix: 'workflow',
        complete: completeWorkflowPair,
        evaluate: content =>
          evaluateWorkflowBinding(
            content,
            before,
            after,
            source.target.name,
            source.target.admission
          ),
      });
      return {
        ...base,
        admission: base.admission ?? 'unavailable',
        ...source.target.admission,
        coverageEligible: false,
      };
    })
  );
}
