/** Fixed-purpose, offline launcher for the disposable heap canary. */

import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  chmodSync,
  closeSync,
  constants,
  fstatSync,
  fchmodSync,
  mkdirSync,
  openSync,
  readFileSync,
  readlinkSync,
  readSync,
  realpathSync,
  statSync,
  writeFileSync,
  writeSync,
} from 'node:fs';
import { createRequire } from 'node:module';
import { basename, dirname, isAbsolute, join, normalize } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { captureHouseholdResources } from '../../src/framework/fixtures/process-resources.js';
import { closeAdminBounded, connectAdminBounded } from '../resource-profile.js';

import {
  closeSqlTimingSources,
  closeWorkflowSources,
  finishSqlTimingSources,
  finishWorkflowSources,
  openSqlTimingSources,
  openWorkflowSources,
  readSqlTimingArtifact,
  readWorkflowArtifact,
  type DiagnosticAdmissionBinding,
  type SqlTimingDiagnostic,
  type WorkflowDiagnostic,
} from './performance-binding.js';
import {
  MAX_CANARY_DUMP_TIMEOUT_MS,
  MAX_CANARY_LIFETIME_MS,
  MAX_CANARY_OBSERVATION_MS,
  runDisposableCanaryLifecycle,
  type CanaryLaunchContext,
  type CanaryParserResult,
  type CanaryParserStopEvidence,
  type DisposableCanaryRequest,
  type DisposableCanaryWitness,
} from './performance-canary.js';
import {
  armDiagnostic,
  captureHeapPhase,
  type DiagnosticFamily,
  type HeapExpectedProcess,
  type HeapNativeCompletionEvidence,
} from './performance-control.js';
import { inspectWorkflowArtifact, summarizeSqlTimingArtifact } from './performance-evidence.js';
import {
  JEPROF_TIMEOUT_MS,
  MAX_HEAP_DUMP_BYTES,
  MAX_JEPROF_OUTPUT_BYTES,
  parseJeprofText,
  runBoundedTool,
  snapshotBoundedFile,
  ToolStoppedError,
  type ToolCleanupEvidence,
} from './performance-heap.js';

const ADMIN_PORT = 4444;
const PREPARATION_TIMEOUT_MS = 30_000;
const ARTIFACT_MAX_BYTES = 512 * 1024 * 1024;
const WORKER_OUTPUT_BYTES = 1024 * 1024;
const MAX_REJECTED_WORKER_EVIDENCE_BYTES = WORKER_OUTPUT_BYTES + 512;
const UNSHARE = '/usr/bin/unshare';
const IP = '/usr/sbin/ip';
const PERL = '/usr/bin/perl';
const WORKER_FLAG = '--internal-heap-canary-worker';
const REQUEST_ENV = 'ELOHIM_INTERNAL_HEAP_CANARY_REQUEST';
const PARENT_NET_ENV = 'ELOHIM_INTERNAL_HEAP_CANARY_PARENT_NET';
const PARENT_PID_ENV = 'ELOHIM_INTERNAL_HEAP_CANARY_PARENT_PID';
const APP_ID = 'elohim-performance-heap-canary';
const ADMIN_ORIGIN = 'elohim-runtime-performance';
const MINIMAL_PATH = '/usr/bin:/bin';
const PROFILER_CONFIG =
  'prof:true,prof_active:false,prof_gdump:false,prof_final:false,lg_prof_interval:-1';
const CONTROL_SECONDS = 2;
const CONTROL_OBSERVATION_MS = 1_000;
const CONTROL_EVIDENCE_BASENAME = 'control-evidence';
const SQL_CONTROL_BASENAME = 'sql-diagnostics';
const WORKFLOW_CONTROL_BASENAME = 'workflow-diagnostics';
const WORKFLOW_TRACE_FILTER = 'holochain::diagnostics::workflow=trace';
const CONTROL_BINDING_INCOMPLETE = 'control-binding-incomplete';
const CONTROL_ARTIFACT_INVALID = 'control-artifact-invalid';
const EMPTY_SAMPLED_PROFILE = 'empty-sampled-profile';
export const CANARY_PREPARATION_LOG_BASENAME = 'canary.preparation.log';
export const CANARY_REJECTED_WORKER_EVIDENCE_BASENAME = 'canary.rejected-worker-evidence.log';
const MAX_PREPARATION_LOG_BYTES = 4 * 1024;
const MAX_NATIVE_HEAP_EVIDENCE_BYTES = 16 * 1024;
const NATIVE_HEAP_EVIDENCE_KIND = 'offline-disposable-heap-canary-native-evidence/v1';
const REJECTED_WORKER_EVIDENCE_KIND = 'offline-disposable-heap-canary-rejected-worker-evidence/v1';
const WORKER_FAILURE_KIND = 'offline-disposable-heap-canary-failure/v1';
const PREPARATION_FAILURE_KIND = 'offline-disposable-heap-canary-preparation-failure/v1';
const ARTIFACT_IDENTITY_CHANGED = 'artifact-identity-changed';
const WATCHDOG_NATIVE_CHILD_MISSING = 'watchdog-native-child-missing';
const PARSER_STOP_UNAVAILABLE = 'parser-stop-unavailable';
const WORKER_CLEANUP_STAGE = 'worker-cleanup';
const SHA256 = /^[a-f0-9]{64}$/;
const SAFE_NONCE = /^[A-Za-z0-9_-]{1,64}$/;
const MODULE_REQUIRE = createRequire(import.meta.url);
const FIXED_TSX_LOADER = pathToFileURL(MODULE_REQUIRE.resolve('tsx')).href;

const WORKER_FAILURE_STAGES = [
  'request-admission',
  'artifact-admission',
  'namespace-admission',
  'loopback-setup',
  'lifecycle',
  WORKER_CLEANUP_STAGE,
  'worker-envelope',
] as const;
type WorkerFailureStage = (typeof WORKER_FAILURE_STAGES)[number];

const SAFE_FAILURE_REASONS = [
  'invalid-internal-request',
  'artifact-unavailable',
  ARTIFACT_IDENTITY_CHANGED,
  'artifact-over-cap',
  'artifact-hash-mismatch',
  'network-namespace-unavailable',
  'pid-namespace-unavailable',
  'loopback-unavailable',
  'launch-plan-refused',
  'watchdog-spawn-failed',
  WATCHDOG_NATIVE_CHILD_MISSING,
  'native-identity-unavailable',
  'admin-preparation-failed',
  'native-capture-refused',
  PARSER_STOP_UNAVAILABLE,
  'permission-denied',
  'path-already-exists',
  'path-unavailable',
  'unclassified-failure',
  'diagnostic-unavailable',
] as const;
type SafeFailureReason = (typeof SAFE_FAILURE_REASONS)[number];

interface WorkerFailureDiagnostic {
  kind: typeof WORKER_FAILURE_KIND;
  stage: WorkerFailureStage;
  reason: SafeFailureReason;
}

export class HeapCanaryWorkerError extends Error {
  constructor(
    readonly stage: WorkerFailureStage,
    readonly reason: SafeFailureReason
  ) {
    super(`offline canary worker failed (${stage}: ${reason})`);
    this.name = 'HeapCanaryWorkerError';
  }
}

class InternalWorkerFailure extends Error {
  constructor(readonly diagnostic: WorkerFailureDiagnostic) {
    super(diagnostic.reason);
    this.name = 'InternalWorkerFailure';
  }
}

const SAFE_MESSAGE_REASONS = new Map<string, SafeFailureReason>([
  ['approved canary artifact is unavailable', 'artifact-unavailable'],
  ['approved canary artifact identity changed', ARTIFACT_IDENTITY_CHANGED],
  ['approved artifact exceeds cap', 'artifact-over-cap'],
  ['approved canary artifact changed while hashing', ARTIFACT_IDENTITY_CHANGED],
  ['approved canary artifact hash mismatch', 'artifact-hash-mismatch'],
  ['fresh network namespace not established', 'network-namespace-unavailable'],
  ['fresh PID namespace init not established', 'pid-namespace-unavailable'],
  ['private loopback setup failed', 'loopback-unavailable'],
  ['canary launch plan was refused', 'launch-plan-refused'],
  ['watchdog spawn failed before assigning a direct-child pid', 'watchdog-spawn-failed'],
  ['watchdog exited before native identity was pinned', WATCHDOG_NATIVE_CHILD_MISSING],
  ['watchdog did not expose the native child before its deadline', WATCHDOG_NATIVE_CHILD_MISSING],
  ['owned native canary identity was unavailable', 'native-identity-unavailable'],
  ['offline canary admin preparation failed', 'admin-preparation-failed'],
  ['native heap capture was refused', 'native-capture-refused'],
  ['parser safe-stop evidence unavailable', PARSER_STOP_UNAVAILABLE],
  ['parser safe-stop evidence is incomplete', PARSER_STOP_UNAVAILABLE],
]);

function failureReason(error: unknown): SafeFailureReason {
  if (error instanceof InternalWorkerFailure) return error.diagnostic.reason;
  if (error instanceof Error) {
    const known = SAFE_MESSAGE_REASONS.get(error.message);
    if (known) return known;
    const code = (error as NodeJS.ErrnoException).code;
    if (code === 'EACCES' || code === 'EPERM') return 'permission-denied';
    if (code === 'EEXIST') return 'path-already-exists';
    if (code === 'ENOENT' || code === 'ENOTDIR') return 'path-unavailable';
  }
  return 'unclassified-failure';
}

function workerFailure(stage: WorkerFailureStage, error: unknown): InternalWorkerFailure {
  if (error instanceof InternalWorkerFailure) return error;
  return new InternalWorkerFailure({
    kind: WORKER_FAILURE_KIND,
    stage,
    reason: failureReason(error),
  });
}

function parseWorkerFailure(stderr: string): WorkerFailureDiagnostic | null {
  if (Buffer.byteLength(stderr) > 512) return null;
  try {
    const diagnostic = record(JSON.parse(stderr.trim()));
    if (
      !diagnostic ||
      !exactKeys(diagnostic, ['kind', 'stage', 'reason']) ||
      diagnostic.kind !== WORKER_FAILURE_KIND ||
      !WORKER_FAILURE_STAGES.includes(diagnostic.stage as WorkerFailureStage) ||
      !SAFE_FAILURE_REASONS.includes(diagnostic.reason as SafeFailureReason)
    )
      return null;
    return diagnostic as unknown as WorkerFailureDiagnostic;
  } catch {
    return null;
  }
}

function publicWorkerFailure(stderr: string): HeapCanaryWorkerError {
  const diagnostic = parseWorkerFailure(stderr) ?? {
    kind: WORKER_FAILURE_KIND,
    stage: 'worker-envelope' as const,
    reason: 'diagnostic-unavailable' as const,
  };
  return new HeapCanaryWorkerError(diagnostic.stage, diagnostic.reason);
}

export interface ApprovedCanaryArtifact {
  path: string;
  sha256: string;
}

export interface IsolatedHeapCanaryRequest extends DisposableCanaryRequest {
  holochain: ApprovedCanaryArtifact;
  happ: ApprovedCanaryArtifact;
  jeprof: ApprovedCanaryArtifact;
  verifyControls?: true;
}

type ControlBindingStatus = 'exact' | 'encloses' | 'bound' | 'valid';

interface ControlGenerationWitness {
  family: DiagnosticFamily;
  generation: 1 | 2;
  status: Extract<ControlBindingStatus, 'bound'>;
  admission: Extract<ControlBindingStatus, 'exact'>;
  nativeIdentity: Extract<ControlBindingStatus, 'exact'>;
  interval: Extract<ControlBindingStatus, 'encloses'>;
  terminalParser: Extract<ControlBindingStatus, 'valid'>;
}

interface ControlVerificationWitness {
  requested: true;
  outcome: 'passed' | 'incomplete';
  requestedSeconds: typeof CONTROL_SECONDS;
  sequence: ControlGenerationWitness[];
  coverageEligible: false;
  coverageReason: 'quiet private-control lifecycle only; not workload or attribution coverage';
}

export type IsolatedHeapCanaryResult = Omit<DisposableCanaryWitness, 'observed'> & {
  observed: DisposableCanaryWitness['observed'] & {
    launcher: {
      kind: 'offline-disposable-heap-canary/v1';
      networkNamespaceVerified: true;
      pidNamespaceVerified: true;
      namespaceInitPidOne: true;
      artifactsFingerprintVerified: true;
      freshStateOnly: true;
      appProvisioningMeasured: false;
      workload: 'offline-pipeline-smoke';
      retainedScope: 'allocations-sampled-after-profiler-activation';
      coverageEligible: false;
    };
    controlVerification?: ControlVerificationWitness;
  };
};

type WorkerRunner = (request: IsolatedHeapCanaryRequest) => Promise<IsolatedHeapCanaryResult>;

interface PinnedArtifacts {
  holochain: string;
  happ: string;
  jeprof: string;
}

interface PrivateNativeHeapEvidence {
  kind: typeof NATIVE_HEAP_EVIDENCE_KIND;
  schemaVersion: 1;
  phase: 'before' | 'after';
  approvedBinarySha256: string;
  artifact: {
    basename: string;
    bytes: number;
    sha256: string;
  };
  control: HeapNativeCompletionEvidence;
}

const flagNames = [
  '--root',
  '--nonce',
  '--lifetime-ms',
  '--observation-ms',
  '--dump-timeout-ms',
  '--holochain',
  '--holochain-sha256',
  '--happ',
  '--happ-sha256',
  '--jeprof',
  '--jeprof-sha256',
] as const;
const VERIFY_CONTROLS_FLAG = '--verify-controls';

function integer(
  value: string | undefined,
  label: string,
  minimum: number,
  maximum: number
): number {
  if (!value || !/^\d+$/.test(value)) throw new Error(`invalid ${label}`);
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < minimum || parsed > maximum)
    throw new Error(`invalid ${label}`);
  return parsed;
}

function artifact(path: string | undefined, sha256: string | undefined): ApprovedCanaryArtifact {
  if (!path || !isAbsolute(path) || normalize(path) !== path || !sha256 || !SHA256.test(sha256))
    throw new Error('invalid approved canary artifact');
  return { path, sha256 };
}

function validateRequest(request: IsolatedHeapCanaryRequest): void {
  if (
    !isAbsolute(request.isolatedRoot) ||
    normalize(request.isolatedRoot) !== request.isolatedRoot ||
    request.isolatedRoot === '/' ||
    !SAFE_NONCE.test(request.nonce) ||
    !Number.isSafeInteger(request.lifetimeMs) ||
    request.lifetimeMs < 1 ||
    request.lifetimeMs > MAX_CANARY_LIFETIME_MS ||
    !Number.isSafeInteger(request.observationMs) ||
    request.observationMs < 1 ||
    request.observationMs > MAX_CANARY_OBSERVATION_MS ||
    !Number.isSafeInteger(request.dumpTimeoutMs) ||
    request.dumpTimeoutMs! < 1_000 ||
    request.dumpTimeoutMs! > MAX_CANARY_DUMP_TIMEOUT_MS ||
    request.dumpTimeoutMs! % 1_000 !== 0 ||
    (request.verifyControls !== undefined && request.verifyControls !== true) ||
    PREPARATION_TIMEOUT_MS + request.observationMs + 2 * request.dumpTimeoutMs! > request.lifetimeMs
  )
    throw new Error('invalid isolated heap-canary request');
  for (const approved of [request.holochain, request.happ, request.jeprof])
    artifact(approved.path, approved.sha256);
}

export function parseHeapCanaryArgs(argv: string[]): IsolatedHeapCanaryRequest {
  if (argv.length !== flagNames.length * 2 && argv.length !== flagNames.length * 2 + 2)
    throw new Error('heap-canary requires exact arguments');
  const values = new Map<string, string>();
  for (let index = 0; index < argv.length; index += 2) {
    const flag = argv[index];
    const value = argv[index + 1];
    if (
      ![...flagNames, VERIFY_CONTROLS_FLAG].includes(flag as (typeof flagNames)[number]) ||
      values.has(flag) ||
      !value
    )
      throw new Error('heap-canary requires exact arguments');
    values.set(flag, value);
  }
  const verifyControls = values.get(VERIFY_CONTROLS_FLAG);
  if (verifyControls !== undefined && verifyControls !== 'true')
    throw new Error('--verify-controls accepts only true');
  const nonce = values.get('--nonce')!;
  if (!SAFE_NONCE.test(nonce)) throw new Error('invalid heap-canary nonce');
  const lifetimeMs = integer(
    values.get('--lifetime-ms'),
    'heap-canary lifetime',
    1,
    MAX_CANARY_LIFETIME_MS
  );
  const observationMs = integer(
    values.get('--observation-ms'),
    'heap-canary observation',
    1,
    MAX_CANARY_OBSERVATION_MS
  );
  const dumpTimeoutMs = integer(
    values.get('--dump-timeout-ms'),
    'heap-canary dump timeout',
    1_000,
    MAX_CANARY_DUMP_TIMEOUT_MS
  );
  if (dumpTimeoutMs % 1_000 !== 0)
    throw new Error('heap-canary dump timeout must be whole seconds');
  if (PREPARATION_TIMEOUT_MS + observationMs + 2 * dumpTimeoutMs > lifetimeMs)
    throw new Error('heap-canary preparation and phases do not fit its lifetime');
  const isolatedRoot = values.get('--root')!;
  if (!isAbsolute(isolatedRoot) || normalize(isolatedRoot) !== isolatedRoot || isolatedRoot === '/')
    throw new Error('invalid heap-canary root');
  const request = {
    isolatedRoot,
    nonce,
    lifetimeMs,
    observationMs,
    dumpTimeoutMs,
    holochain: artifact(values.get('--holochain'), values.get('--holochain-sha256')),
    happ: artifact(values.get('--happ'), values.get('--happ-sha256')),
    jeprof: artifact(values.get('--jeprof'), values.get('--jeprof-sha256')),
    ...(verifyControls === 'true' ? { verifyControls: true as const } : {}),
  };
  validateRequest(request);
  return request;
}

function hashRegularFile(approved: ApprovedCanaryArtifact, executable: boolean): string {
  const inspected = statSync(approved.path, { bigint: true });
  if (
    !inspected.isFile() ||
    inspected.size < 1n ||
    inspected.size > BigInt(ARTIFACT_MAX_BYTES) ||
    (executable && (inspected.mode & 0o111n) === 0n)
  )
    throw new Error('approved canary artifact is unavailable');
  const descriptor = openSync(
    approved.path,
    constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK
  );
  try {
    const opened = fstatSync(descriptor, { bigint: true });
    if (
      opened.dev !== inspected.dev ||
      opened.ino !== inspected.ino ||
      opened.size !== inspected.size
    )
      throw new Error('approved canary artifact identity changed');
    const digest = createHash('sha256');
    const buffer = Buffer.alloc(64 * 1024);
    let total = 0n;
    for (;;) {
      const count = readSync(descriptor, buffer, 0, buffer.length, null);
      if (count === 0) break;
      total += BigInt(count);
      if (total > BigInt(ARTIFACT_MAX_BYTES)) throw new Error('approved artifact exceeds cap');
      digest.update(buffer.subarray(0, count));
    }
    const completed = fstatSync(descriptor, { bigint: true });
    if (
      total !== opened.size ||
      completed.dev !== opened.dev ||
      completed.ino !== opened.ino ||
      completed.size !== opened.size ||
      completed.mtimeNs !== opened.mtimeNs
    )
      throw new Error('approved canary artifact changed while hashing');
    const actual = digest.digest('hex');
    if (actual !== approved.sha256) throw new Error('approved canary artifact hash mismatch');
    return actual;
  } finally {
    closeSync(descriptor);
  }
}

function nativeEvidenceBasename(phase: 'before' | 'after'): string {
  return `${phase}.heap.native-evidence.json`;
}

function hashFinalHeapArtifact(path: string, expectedBytes: number): string {
  if (
    !Number.isSafeInteger(expectedBytes) ||
    expectedBytes < 1 ||
    expectedBytes > MAX_HEAP_DUMP_BYTES
  )
    throw new Error('invalid final heap artifact bound');
  const descriptor = openSync(
    path,
    constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK
  );
  try {
    const before = fstatSync(descriptor, { bigint: true });
    if (!before.isFile() || before.size !== BigInt(expectedBytes))
      throw new Error('final heap artifact identity mismatch');
    const digest = createHash('sha256');
    const buffer = Buffer.alloc(64 * 1024);
    let total = 0;
    while (total < expectedBytes) {
      const count = readSync(
        descriptor,
        buffer,
        0,
        Math.min(buffer.length, expectedBytes - total),
        null
      );
      if (count <= 0) throw new Error('final heap artifact ended before its bound');
      digest.update(buffer.subarray(0, count));
      total += count;
    }
    const after = fstatSync(descriptor, { bigint: true });
    if (
      after.dev !== before.dev ||
      after.ino !== before.ino ||
      after.size !== before.size ||
      after.mtimeNs !== before.mtimeNs ||
      after.ctimeNs !== before.ctimeNs
    )
      throw new Error('final heap artifact changed while hashing');
    return digest.digest('hex');
  } finally {
    closeSync(descriptor);
  }
}

function validatePrivateNativeHeapEvidence(
  value: unknown,
  expected: {
    phase: 'before' | 'after';
    nonce: string;
    process: HeapExpectedProcess;
    approvedBinarySha256: string;
    artifactSha256: string;
    artifactBytes: number;
    dumpTimeoutMs: number;
    producerId?: string;
  }
): PrivateNativeHeapEvidence {
  const evidence = record(value);
  const artifact = record(evidence?.artifact);
  const control = record(evidence?.control);
  const expectedProcess = record(control?.expectedProcess);
  const response = record(control?.response);
  const receipt = record(response?.value);
  const nativeProcess = record(receipt?.nativeProcess);
  if (
    !evidence ||
    !artifact ||
    !control ||
    !expectedProcess ||
    !response ||
    !receipt ||
    !nativeProcess ||
    !exactKeys(evidence, [
      'kind',
      'schemaVersion',
      'phase',
      'approvedBinarySha256',
      'artifact',
      'control',
    ]) ||
    !exactKeys(artifact, ['basename', 'bytes', 'sha256']) ||
    !exactKeys(control, [
      'requestStartedMonotonicMs',
      'requestFinishedMonotonicMs',
      'expectedProcess',
      'response',
    ]) ||
    !exactKeys(expectedProcess, ['processId', 'processStartTicks', 'bootId', 'executable']) ||
    !exactKeys(response, ['outcome', 'value']) ||
    !exactKeys(receipt, [
      'schemaVersion',
      'nonce',
      'phase',
      'producerId',
      'generation',
      'artifactBasename',
      'nativeProcess',
      'startedMonotonicMs',
      'finishedMonotonicMs',
      'dumpTimeoutMs',
      'deadlineMonotonicMs',
      'nativeCompleted',
    ]) ||
    !exactKeys(nativeProcess, [
      'schema',
      'processId',
      'processStartTicks',
      'bootId',
      'executable',
      'clock',
    ]) ||
    evidence.kind !== NATIVE_HEAP_EVIDENCE_KIND ||
    evidence.schemaVersion !== 1 ||
    evidence.phase !== expected.phase ||
    !SHA256.test(expected.approvedBinarySha256) ||
    !SHA256.test(expected.artifactSha256) ||
    !SAFE_NONCE.test(expected.nonce) ||
    !Number.isSafeInteger(expected.artifactBytes) ||
    expected.artifactBytes < 1 ||
    expected.artifactBytes > MAX_HEAP_DUMP_BYTES ||
    !Number.isSafeInteger(expected.process.processId) ||
    expected.process.processId < 1 ||
    !Number.isSafeInteger(expected.process.processStartTicks) ||
    expected.process.processStartTicks < 1 ||
    !expected.process.bootId ||
    !expected.process.executable.startsWith('/') ||
    evidence.approvedBinarySha256 !== expected.approvedBinarySha256 ||
    artifact.basename !== `${expected.phase}.heap` ||
    artifact.bytes !== expected.artifactBytes ||
    artifact.sha256 !== expected.artifactSha256 ||
    response.outcome !== 'completed' ||
    receipt.schemaVersion !== 1 ||
    receipt.nonce !== expected.nonce ||
    receipt.phase !== expected.phase ||
    receipt.generation !== 1 ||
    typeof receipt.producerId !== 'string' ||
    !/^[A-Za-z0-9_-]{1,128}$/.test(receipt.producerId) ||
    (expected.producerId !== undefined && receipt.producerId !== expected.producerId) ||
    receipt.artifactBasename !== `${expected.phase}.heap.fifo` ||
    receipt.nativeCompleted !== true ||
    nativeProcess.schema !== 1 ||
    nativeProcess.clock !== 'CLOCK_MONOTONIC' ||
    !sameExpectedProcess(expectedProcess, expected.process) ||
    !sameExpectedProcess(nativeProcess, expected.process) ||
    !Number.isSafeInteger(control.requestStartedMonotonicMs) ||
    !Number.isSafeInteger(control.requestFinishedMonotonicMs) ||
    Number(control.requestStartedMonotonicMs) < 0 ||
    Number(control.requestFinishedMonotonicMs) < Number(control.requestStartedMonotonicMs) ||
    !Number.isSafeInteger(receipt.startedMonotonicMs) ||
    !Number.isSafeInteger(receipt.finishedMonotonicMs) ||
    !Number.isSafeInteger(receipt.deadlineMonotonicMs) ||
    !Number.isSafeInteger(receipt.dumpTimeoutMs) ||
    Number(receipt.dumpTimeoutMs) < 1_000 ||
    Number(receipt.dumpTimeoutMs) > 30_000 ||
    Number(receipt.dumpTimeoutMs) % 1_000 !== 0 ||
    receipt.dumpTimeoutMs !== expected.dumpTimeoutMs ||
    Number(receipt.startedMonotonicMs) < Math.floor(Number(control.requestStartedMonotonicMs)) ||
    Number(receipt.finishedMonotonicMs) > Math.floor(Number(control.requestFinishedMonotonicMs)) ||
    Number(receipt.finishedMonotonicMs) < Number(receipt.startedMonotonicMs) ||
    Number(receipt.deadlineMonotonicMs) - Number(receipt.startedMonotonicMs) !==
      Number(receipt.dumpTimeoutMs) ||
    Number(receipt.finishedMonotonicMs) > Number(receipt.deadlineMonotonicMs)
  )
    throw new Error('private native heap evidence binding mismatch');
  return value as PrivateNativeHeapEvidence;
}

function sameExpectedProcess(
  value: Record<string, unknown>,
  expected: HeapExpectedProcess
): boolean {
  return (
    value.processId === expected.processId &&
    value.processStartTicks === expected.processStartTicks &&
    value.bootId === expected.bootId &&
    value.executable === expected.executable
  );
}

function readPrivateNativeHeapEvidence(
  path: string,
  expected: Parameters<typeof validatePrivateNativeHeapEvidence>[1]
): PrivateNativeHeapEvidence {
  const descriptor = openSync(
    path,
    constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK
  );
  try {
    const before = fstatSync(descriptor, { bigint: true });
    if (
      !before.isFile() ||
      before.size < 1n ||
      before.size > BigInt(MAX_NATIVE_HEAP_EVIDENCE_BYTES)
    )
      throw new Error('private native heap evidence exceeds its bound');
    const bytes = Buffer.alloc(Number(before.size));
    let offset = 0;
    while (offset < bytes.length) {
      const count = readSync(descriptor, bytes, offset, bytes.length - offset, null);
      if (count <= 0) throw new Error('private native heap evidence ended early');
      offset += count;
    }
    const after = fstatSync(descriptor, { bigint: true });
    if (
      after.dev !== before.dev ||
      after.ino !== before.ino ||
      after.size !== before.size ||
      after.mtimeNs !== before.mtimeNs ||
      after.ctimeNs !== before.ctimeNs
    )
      throw new Error('private native heap evidence changed while reading');
    let parsed: unknown;
    try {
      parsed = JSON.parse(bytes.toString('utf8')) as unknown;
    } catch {
      throw new Error('private native heap evidence is malformed');
    }
    return validatePrivateNativeHeapEvidence(parsed, expected);
  } finally {
    closeSync(descriptor);
  }
}

function readPrivateNativeHeapEvidencePair(
  root: string,
  expected: {
    nonce: string;
    process: HeapExpectedProcess;
    approvedBinarySha256: string;
    dumpTimeoutMs: number;
    before: { artifactSha256: string; artifactBytes: number };
    after: { artifactSha256: string; artifactBytes: number };
  }
): { before: PrivateNativeHeapEvidence; after: PrivateNativeHeapEvidence } {
  const common = {
    nonce: expected.nonce,
    process: expected.process,
    approvedBinarySha256: expected.approvedBinarySha256,
    dumpTimeoutMs: expected.dumpTimeoutMs,
  };
  const before = readPrivateNativeHeapEvidence(join(root, nativeEvidenceBasename('before')), {
    ...common,
    phase: 'before',
    ...expected.before,
  });
  const after = readPrivateNativeHeapEvidence(join(root, nativeEvidenceBasename('after')), {
    ...common,
    phase: 'after',
    producerId: before.control.response.value.producerId,
    ...expected.after,
  });
  return { before, after };
}

function writePrivateNativeHeapEvidence(
  root: string,
  evidence: PrivateNativeHeapEvidence,
  expected: Parameters<typeof validatePrivateNativeHeapEvidence>[1]
): void {
  validatePrivateNativeHeapEvidence(evidence, expected);
  const content = Buffer.from(`${JSON.stringify(evidence)}\n`);
  if (content.length > MAX_NATIVE_HEAP_EVIDENCE_BYTES)
    throw new Error('private native heap evidence exceeds its bound');
  const path = join(root, nativeEvidenceBasename(expected.phase));
  const descriptor = openSync(
    path,
    constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL | constants.O_NOFOLLOW,
    0o600
  );
  try {
    fchmodSync(descriptor, 0o600);
    let offset = 0;
    while (offset < content.length) {
      const count = writeSync(descriptor, content, offset, content.length - offset);
      if (count <= 0) throw new Error('private native heap evidence write made no progress');
      offset += count;
    }
  } finally {
    closeSync(descriptor);
  }
}

function verifyArtifacts(request: IsolatedHeapCanaryRequest): void {
  hashRegularFile(request.holochain, true);
  hashRegularFile(request.happ, false);
  hashRegularFile(request.jeprof, false);
}

function pinArtifacts(
  context: CanaryLaunchContext,
  request: IsolatedHeapCanaryRequest
): PinnedArtifacts {
  const root = join(context.isolatedRoot, 'approved-artifacts');
  mkdirSync(root, { mode: 0o700 });
  chmodSync(root, 0o700);
  const pinned = {
    holochain: join(root, 'holochain'),
    happ: join(root, 'canary.happ'),
    jeprof: join(root, 'jeprof'),
  };
  snapshotBoundedFile(request.holochain.path, pinned.holochain, ARTIFACT_MAX_BYTES, 'holochain');
  chmodSync(pinned.holochain, 0o500);
  snapshotBoundedFile(request.happ.path, pinned.happ, ARTIFACT_MAX_BYTES, 'happ');
  snapshotBoundedFile(request.jeprof.path, pinned.jeprof, ARTIFACT_MAX_BYTES, 'jeprof');
  hashRegularFile({ path: pinned.holochain, sha256: request.holochain.sha256 }, true);
  hashRegularFile({ path: pinned.happ, sha256: request.happ.sha256 }, false);
  hashRegularFile({ path: pinned.jeprof, sha256: request.jeprof.sha256 }, false);
  return pinned;
}

function renderConfig(context: CanaryLaunchContext): string {
  return [
    '---',
    `data_root_path: ${JSON.stringify(context.stateRoot)}`,
    'keystore:',
    '  type: danger_test_keystore',
    'admin_interfaces:',
    '  - driver:',
    '      type: websocket',
    `      port: ${ADMIN_PORT}`,
    `      allowed_origins: ${JSON.stringify(ADMIN_ORIGIN)}`,
    'network:',
    '  bootstrap_url: "http://127.0.0.1:9"',
    '  relay_url: "https://127.0.0.1:9"',
    '',
  ].join('\n');
}

function conductorEnvironment(
  context: CanaryLaunchContext,
  request: IsolatedHeapCanaryRequest
): NodeJS.ProcessEnv {
  return {
    PATH: MINIMAL_PATH,
    MALLOC_CONF: PROFILER_CONFIG,
    _RJEM_MALLOC_CONF: PROFILER_CONFIG,
    HOLOCHAIN_HEAP_CANARY_DIR: context.heapRoot,
    HOLOCHAIN_HEAP_CANARY_WINDOW_SECONDS: String(Math.ceil(request.lifetimeMs / 1000)),
    HOLOCHAIN_HEAP_CANARY_DUMP_TIMEOUT_SECONDS: String(request.dumpTimeoutMs! / 1000),
    ...(request.verifyControls
      ? {
          HOLOCHAIN_SQL_DIAGNOSTICS_DIR: join(context.isolatedRoot, SQL_CONTROL_BASENAME),
          HOLOCHAIN_SQL_DIAGNOSTICS_SECONDS: '0',
          HOLOCHAIN_WORKFLOW_DIAGNOSTICS_DIR: join(context.isolatedRoot, WORKFLOW_CONTROL_BASENAME),
          HOLOCHAIN_WORKFLOW_DIAGNOSTICS_SECONDS: '0',
          RUST_LOG: WORKFLOW_TRACE_FILTER,
        }
      : {}),
  };
}

function monotonicMs(): number {
  return Number(process.hrtime.bigint() / 1_000_000n);
}

function stopEvidence(cleanup: ToolCleanupEvidence | undefined): CanaryParserStopEvidence {
  if (!cleanup?.watchdogReaped || !cleanup.groupWorkStopped)
    throw new Error('parser safe-stop evidence unavailable');
  return { directWatchdogReaped: true, executingGroupMembers: 0 };
}

function hasEmptySampledHeapPrefix(path: string): boolean {
  let descriptor = -1;
  try {
    descriptor = openSync(path, constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK);
    const inspected = fstatSync(descriptor);
    if (!inspected.isFile() || inspected.size < 1) return false;
    const prefix = Buffer.alloc(256);
    const bytes = readSync(descriptor, prefix, 0, prefix.length, 0);
    return /^heap_v2\/\d+\n[ \t]*t\*: 0: 0 \[0: 0\](?:\n|$)/.test(
      prefix.subarray(0, bytes).toString('ascii')
    );
  } catch {
    return false;
  } finally {
    if (descriptor >= 0) closeSync(descriptor);
  }
}

function parserTask(options: {
  artifactPath: string;
  artifacts: PinnedArtifacts;
  signal: AbortSignal;
}) {
  const controller = new AbortController();
  options.signal.addEventListener('abort', () => controller.abort(), { once: true });
  const artifactDir = dirname(options.artifactPath);
  const tool = runBoundedTool({
    file: PERL,
    args: [
      options.artifacts.jeprof,
      '--text',
      '--functions',
      '--inuse_space',
      '--show_bytes',
      '--cum',
      options.artifacts.holochain,
      options.artifactPath,
    ],
    cwd: artifactDir,
    env: { PATH: MINIMAL_PATH, JEPROF_TMPDIR: artifactDir },
    timeoutMs: JEPROF_TIMEOUT_MS,
    maxOutputBytes: MAX_JEPROF_OUTPUT_BYTES,
    signal: controller.signal,
  });
  return {
    result: tool.then<CanaryParserResult>(result => {
      const cleanup = stopEvidence(result.cleanup);
      if (result.exitCode !== 0 || result.signal !== null)
        return { valid: false as const, reason: 'malformed' as const, cleanup };
      if (result.stdout.trim() === '' && hasEmptySampledHeapPrefix(options.artifactPath))
        return {
          valid: true as const,
          reason: EMPTY_SAMPLED_PROFILE,
          cleanup,
        };
      try {
        parseJeprofText(result.stdout);
        return { valid: true as const, reason: 'valid' as const, cleanup };
      } catch {
        return { valid: false as const, reason: 'malformed' as const, cleanup };
      }
    }),
    abortAndConfirmStopped: async (): Promise<CanaryParserStopEvidence> => {
      controller.abort();
      try {
        return stopEvidence((await tool).cleanup);
      } catch (error) {
        if (error instanceof ToolStoppedError) return stopEvidence(error.cleanup);
        throw error;
      }
    },
  };
}

type Admin = Awaited<ReturnType<typeof connectAdminBounded>>;

type PreparationStage = 'connect' | 'control-verification' | 'agent-key' | 'install' | 'enable';
type ControlVerificationFailureReason =
  | 'control-admission-refused'
  | 'control-admission-invalid'
  | 'control-transport-unknown'
  | 'control-generation-mismatch'
  | typeof CONTROL_BINDING_INCOMPLETE
  | typeof CONTROL_ARTIFACT_INVALID
  | 'control-producer-changed';
type PreparationFailureReason =
  | 'timeout'
  | 'refused'
  | 'cancelled'
  | ControlVerificationFailureReason;
interface PreparationFailureDiagnostic {
  kind: typeof PREPARATION_FAILURE_KIND;
  stage: PreparationStage;
  reason: PreparationFailureReason;
}

class ControlVerificationError extends Error {
  constructor(
    readonly reason: ControlVerificationFailureReason,
    message: string
  ) {
    super(message);
    this.name = 'ControlVerificationError';
  }
}

function preparationFailureReason(error: unknown, signal: AbortSignal): PreparationFailureReason {
  if (signal.aborted) return 'cancelled';
  if (error instanceof ControlVerificationError) return error.reason;
  const message = error instanceof Error ? error.message : '';
  return /timeout|timed out/i.test(message) ? 'timeout' : 'refused';
}

function preparationLog(root: string): {
  record(diagnostic: PreparationFailureDiagnostic): void;
  close(): void;
} {
  const path = join(root, CANARY_PREPARATION_LOG_BASENAME);
  const descriptor = openSync(
    path,
    constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL | constants.O_NOFOLLOW,
    0o600
  );
  chmodSync(path, 0o600);
  let bytes = 0;
  let closed = false;
  return {
    record: diagnostic => {
      if (closed) return;
      const line = Buffer.from(`${JSON.stringify(diagnostic)}\n`);
      if (bytes + line.length > MAX_PREPARATION_LOG_BYTES) return;
      let offset = 0;
      while (offset < line.length) {
        const progress = writeSync(descriptor, line, offset, line.length - offset);
        if (progress <= 0) return;
        offset += progress;
        bytes += progress;
      }
    },
    close: () => {
      if (closed) return;
      closed = true;
      closeSync(descriptor);
    },
  };
}

async function within<T>(
  work: Promise<T>,
  timeoutMs: number,
  label: string,
  signal?: AbortSignal
): Promise<T> {
  let timer: NodeJS.Timeout | undefined;
  let abort: (() => void) | undefined;
  try {
    return await Promise.race([
      work,
      new Promise<never>((_resolve, reject) => {
        timer = setTimeout(() => reject(new Error(`${label} timeout`)), timeoutMs);
      }),
      ...(signal
        ? [
            new Promise<never>((_resolve, reject) => {
              abort = () => reject(new Error(`${label} cancelled`));
              signal.addEventListener('abort', abort, { once: true });
              if (signal.aborted) abort();
            }),
          ]
        : []),
    ]);
  } finally {
    if (timer) clearTimeout(timer);
    if (abort) signal?.removeEventListener('abort', abort);
  }
}

async function provisionPreparedAdmin(
  admin: Admin,
  happPath: string,
  signal: AbortSignal,
  deadlineAt: number,
  recordFailure: (diagnostic: PreparationFailureDiagnostic) => void,
  readMonotonicMs: () => number = monotonicMs
): Promise<void> {
  const request = async <T>(
    stage: Exclude<PreparationStage, 'connect'>,
    label: string,
    invoke: (timeoutMs: number) => Promise<T>
  ): Promise<T> => {
    const timeoutMs = Math.max(1, Math.floor(deadlineAt - readMonotonicMs()));
    try {
      return await within(invoke(timeoutMs), timeoutMs, label, signal);
    } catch (error) {
      recordFailure({
        kind: PREPARATION_FAILURE_KIND,
        stage,
        reason: preparationFailureReason(error, signal),
      });
      throw error;
    }
  };
  const agent = await request('agent-key', 'agent preparation', async timeoutMs =>
    admin.generateAgentPubKey(undefined, timeoutMs)
  );
  await request('install', 'app preparation', async timeoutMs =>
    admin.installApp(
      {
        source: { type: 'path', value: happPath },
        installed_app_id: APP_ID,
        agent_key: agent,
      },
      timeoutMs
    )
  );
  await request('enable', 'app enable', async timeoutMs =>
    admin.enableApp({ installed_app_id: APP_ID }, timeoutMs)
  );
}

async function connectPreparationAdmin(
  signal: AbortSignal,
  deadlineAt: number,
  recordFailure: (diagnostic: PreparationFailureDiagnostic) => void,
  readMonotonicMs: () => number = monotonicMs
): Promise<Admin> {
  let admin: Admin | undefined;
  let lastConnectError: unknown;
  while (readMonotonicMs() < deadlineAt && !admin && !signal.aborted) {
    const pending = connectAdminBounded(new URL(`ws://127.0.0.1:${ADMIN_PORT}`), 500);
    admin = await within(pending, 500, 'admin connect', signal).catch(error => {
      lastConnectError = error;
      void pending.then(closeAdminBounded, () => undefined);
      return undefined;
    });
    if (!admin) await new Promise<void>(resolve => setTimeout(resolve, 50));
  }
  if (!admin) {
    recordFailure({
      kind: PREPARATION_FAILURE_KIND,
      stage: 'connect',
      reason: preparationFailureReason(lastConnectError, signal),
    });
    throw new Error('offline canary admin preparation failed');
  }
  return admin;
}

function controlAdmission(
  admitted: Awaited<ReturnType<typeof armDiagnostic>>
): DiagnosticAdmissionBinding {
  if (!admitted.admitted)
    throw new ControlVerificationError(
      'control-admission-refused',
      'private control admission was refused'
    );
  return {
    nonce: admitted.nonce,
    producerId: admitted.producerId,
    generation: admitted.generation,
    outputBasename: admitted.outputBasename,
    requestedSeconds: admitted.requestedSeconds,
  };
}

function controlNonce(base: string, family: DiagnosticFamily, generation: 1 | 2): string {
  return createHash('sha256')
    .update(base)
    .update('\0')
    .update(family)
    .update('\0')
    .update(String(generation))
    .digest('hex')
    .slice(0, 32);
}

function requireBoundControl(
  diagnostic: SqlTimingDiagnostic | WorkflowDiagnostic,
  family: DiagnosticFamily,
  generation: 1 | 2,
  dependencies: {
    read(family: DiagnosticFamily, path: string): ReturnType<typeof readSqlTimingArtifact>;
    summarizeSql(bytes: Buffer): unknown[];
    inspectWorkflow(bytes: Buffer): ReturnType<typeof inspectWorkflowArtifact>;
  } = {
    read: (selectedFamily, path) =>
      selectedFamily === 'sqlTiming' ? readSqlTimingArtifact(path) : readWorkflowArtifact(path),
    summarizeSql: summarizeSqlTimingArtifact,
    inspectWorkflow: inspectWorkflowArtifact,
  }
): ControlGenerationWitness {
  if (
    diagnostic.status !== 'bound' ||
    diagnostic.admission !== 'exact' ||
    diagnostic.nativeIdentity !== 'exact' ||
    diagnostic.interval !== 'encloses' ||
    diagnostic.issues.length !== 0 ||
    diagnostic.generation !== generation ||
    !diagnostic.artifactPath ||
    !diagnostic.artifactSha256
  )
    throw new ControlVerificationError(
      CONTROL_BINDING_INCOMPLETE,
      'private control artifact binding was incomplete'
    );
  let artifact: ReturnType<typeof readSqlTimingArtifact>;
  try {
    artifact = dependencies.read(family, diagnostic.artifactPath);
  } catch {
    throw new ControlVerificationError(
      CONTROL_ARTIFACT_INVALID,
      'private control artifact was unavailable for terminal validation'
    );
  }
  if (artifact.sha256 !== diagnostic.artifactSha256)
    throw new ControlVerificationError(
      CONTROL_ARTIFACT_INVALID,
      'private control artifact changed before terminal validation'
    );
  try {
    if (family === 'sqlTiming') {
      const summaries = dependencies.summarizeSql(artifact.bytes);
      const summary = summaries.length === 1 ? record(summaries[0]) : null;
      if (summary?.closureReason !== 'expiry')
        throw new Error('private SQL control artifact was invalid');
    } else {
      const parsed = dependencies.inspectWorkflow(artifact.bytes);
      if (parsed.generation !== generation || parsed.closureReason !== 'expiry')
        throw new Error('private workflow control artifact was invalid');
    }
  } catch {
    throw new ControlVerificationError(
      CONTROL_ARTIFACT_INVALID,
      `private ${family === 'sqlTiming' ? 'SQL' : 'workflow'} control artifact was invalid`
    );
  }
  return {
    family,
    generation,
    status: 'bound',
    admission: 'exact',
    nativeIdentity: 'exact',
    interval: 'encloses',
    terminalParser: 'valid',
  };
}

interface ControlGenerationOptions {
  admin: Admin;
  family: DiagnosticFamily;
  generation: 1 | 2;
  nonce: string;
  directory: string;
  configPath: string;
  evidenceRoot: string;
  signal: AbortSignal;
  deadlineAt: number;
}

async function runControlGeneration(
  options: ControlGenerationOptions
): Promise<{ witness: ControlGenerationWitness; producerId: string }> {
  const remaining = Math.max(1, Math.floor(options.deadlineAt - monotonicMs()));
  let admitted: Awaited<ReturnType<typeof armDiagnostic>>;
  try {
    admitted = await within(
      armDiagnostic(
        async (operation, payload, timeoutMs) =>
          options.admin._requester(operation)(payload, timeoutMs),
        options.family,
        options.nonce,
        CONTROL_SECONDS
      ),
      remaining,
      'private control admission',
      options.signal
    );
  } catch (error) {
    if (
      options.signal.aborted ||
      (error instanceof Error && /timeout|cancelled/i.test(error.message))
    )
      throw error;
    if (error instanceof Error && /transport failed/i.test(error.message))
      throw new ControlVerificationError(
        'control-transport-unknown',
        'private control transport outcome is unknown'
      );
    throw new ControlVerificationError(
      'control-admission-invalid',
      'private control admission response was invalid'
    );
  }
  const admission = controlAdmission(admitted);
  if (admission.generation !== options.generation)
    throw new ControlVerificationError(
      'control-generation-mismatch',
      'private control generation was not sequential'
    );
  const sourcePath = join(options.directory, admission.outputBasename);
  if (dirname(sourcePath) !== options.directory)
    throw new Error('private control admission returned an invalid artifact basename');
  const name = `${options.family === 'sqlTiming' ? 'sql' : 'workflow'}-g${options.generation}`;
  const target = { name, sourcePath, admission };
  const configs = { [name]: options.configPath };
  const snapshots = async () => {
    const before = captureHouseholdResources(configs);
    const observationRemaining = Math.floor(options.deadlineAt - monotonicMs());
    if (observationRemaining <= CONTROL_OBSERVATION_MS)
      throw new Error('private control observation exceeded preparation budget');
    await within(
      new Promise<void>(resolve => setTimeout(resolve, CONTROL_OBSERVATION_MS)),
      observationRemaining,
      'private control observation',
      options.signal
    );
    const after = captureHouseholdResources(configs);
    return { before, after };
  };
  if (options.family === 'sqlTiming') {
    const sources = openSqlTimingSources([target], configs).sources;
    try {
      const { before, after } = await snapshots();
      const waitMs = Math.max(0, Math.min(5_000, Math.floor(options.deadlineAt - monotonicMs())));
      const diagnostics = await finishSqlTimingSources(
        sources,
        before,
        after,
        options.evidenceRoot,
        { waitMs, signal: options.signal }
      );
      if (diagnostics.length !== 1)
        throw new ControlVerificationError(
          CONTROL_BINDING_INCOMPLETE,
          'private control artifact binding was unavailable'
        );
      return {
        witness: requireBoundControl(diagnostics[0], options.family, options.generation),
        producerId: admission.producerId,
      };
    } finally {
      closeSqlTimingSources(sources);
    }
  }
  const sources = openWorkflowSources([target], configs).sources;
  try {
    const { before, after } = await snapshots();
    const waitMs = Math.max(0, Math.min(5_000, Math.floor(options.deadlineAt - monotonicMs())));
    const diagnostics = await finishWorkflowSources(sources, before, after, options.evidenceRoot, {
      waitMs,
      signal: options.signal,
    });
    if (diagnostics.length !== 1)
      throw new ControlVerificationError(
        CONTROL_BINDING_INCOMPLETE,
        'private control artifact binding was unavailable'
      );
    return {
      witness: requireBoundControl(diagnostics[0], options.family, options.generation),
      producerId: admission.producerId,
    };
  } finally {
    closeWorkflowSources(sources);
  }
}

type ControlGenerationRunner = (
  options: ControlGenerationOptions
) => Promise<{ witness: ControlGenerationWitness; producerId: string }>;

async function verifyPrivateControls(
  options: {
    admin: Admin;
    baseNonce: string;
    root: string;
    configPath: string;
    signal: AbortSignal;
    deadlineAt: number;
    witness: ControlVerificationWitness;
  },
  runGeneration: ControlGenerationRunner = runControlGeneration
): Promise<void> {
  const directories = {
    sqlTiming: join(options.root, SQL_CONTROL_BASENAME),
    workflow: join(options.root, WORKFLOW_CONTROL_BASENAME),
  } as const;
  const evidenceRoot = join(options.root, CONTROL_EVIDENCE_BASENAME);
  const producerIds = new Map<DiagnosticFamily, string>();
  for (const family of ['sqlTiming', 'workflow'] as const) {
    for (const generation of [1, 2] as const) {
      const result = await runGeneration({
        admin: options.admin,
        family,
        generation,
        nonce: controlNonce(options.baseNonce, family, generation),
        directory: directories[family],
        configPath: options.configPath,
        evidenceRoot,
        signal: options.signal,
        deadlineAt: options.deadlineAt,
      });
      const expectedProducer = producerIds.get(family);
      if (expectedProducer && expectedProducer !== result.producerId)
        throw new ControlVerificationError(
          'control-producer-changed',
          'private control producer changed between generations'
        );
      producerIds.set(family, result.producerId);
      options.witness.sequence.push(result.witness);
    }
  }
  options.witness.outcome = 'passed';
}

function nativeExpected(
  identity: {
    pid: number;
    startTicks: string;
  },
  executable: string
): HeapExpectedProcess {
  const processStartTicks = Number(identity.startTicks);
  if (!Number.isSafeInteger(processStartTicks))
    throw new Error('native start ticks exceed JS range');
  return {
    processId: identity.pid,
    processStartTicks,
    bootId: readFileSync('/proc/sys/kernel/random/boot_id', 'utf8').trim(),
    executable,
  };
}

function requireParserTask(
  artifacts: PinnedArtifacts | undefined,
  artifactPath: string,
  signal: AbortSignal,
  phase: 'before' | 'after',
  bytes: number,
  retained:
    | {
        control: HeapNativeCompletionEvidence;
        process: HeapExpectedProcess;
        approvedBinarySha256: string;
        dumpTimeoutMs: number;
      }
    | undefined
) {
  if (!artifacts || !retained) throw new Error('native heap evidence is unavailable');
  const artifactSha256 = hashFinalHeapArtifact(artifactPath, bytes);
  const expected = {
    phase,
    nonce: retained.control.response.value.nonce,
    process: retained.process,
    approvedBinarySha256: retained.approvedBinarySha256,
    artifactSha256,
    artifactBytes: bytes,
    dumpTimeoutMs: retained.dumpTimeoutMs,
  };
  writePrivateNativeHeapEvidence(
    dirname(artifactPath),
    {
      kind: NATIVE_HEAP_EVIDENCE_KIND,
      schemaVersion: 1,
      phase,
      approvedBinarySha256: retained.approvedBinarySha256,
      artifact: { basename: basename(artifactPath), bytes, sha256: artifactSha256 },
      control: retained.control,
    },
    expected
  );
  return parserTask({ artifactPath, artifacts, signal });
}

async function runWorker(request: IsolatedHeapCanaryRequest): Promise<IsolatedHeapCanaryResult> {
  try {
    verifyArtifacts(request);
  } catch (error) {
    throw workerFailure('artifact-admission', error);
  }
  try {
    const parentNet = process.env[PARENT_NET_ENV];
    const parentPid = process.env[PARENT_PID_ENV];
    const ownNet = readlinkSync('/proc/self/ns/net');
    const ownPid = readlinkSync('/proc/self/ns/pid');
    if (!parentNet || ownNet === parentNet)
      throw new Error('fresh network namespace not established');
    if (!parentPid || ownPid === parentPid || process.pid !== 1)
      throw new Error('fresh PID namespace init not established');
  } catch (error) {
    throw workerFailure('namespace-admission', error);
  }
  try {
    const loopback = spawnSync(IP, ['link', 'set', 'lo', 'up'], {
      timeout: 5_000,
      stdio: 'ignore',
    });
    if (loopback.status !== 0 || loopback.error) throw new Error('private loopback setup failed');
  } catch (error) {
    throw workerFailure('loopback-setup', error);
  }

  let admin: Admin | undefined;
  let producerId: string | undefined;
  let artifacts: PinnedArtifacts | undefined;
  let configPath: string | undefined;
  const controlVerification: ControlVerificationWitness | undefined = request.verifyControls
    ? {
        requested: true,
        outcome: 'incomplete',
        requestedSeconds: CONTROL_SECONDS,
        sequence: [],
        coverageEligible: false,
        coverageReason:
          'quiet private-control lifecycle only; not workload or attribution coverage',
      }
    : undefined;
  const nativeEvidence = new Map<
    'before' | 'after',
    {
      control: HeapNativeCompletionEvidence;
      process: HeapExpectedProcess;
      approvedBinarySha256: string;
      dumpTimeoutMs: number;
    }
  >();
  let preparationDiagnostics: ReturnType<typeof preparationLog> | undefined;
  let workerResult: IsolatedHeapCanaryResult | undefined;
  let failure: InternalWorkerFailure | undefined;
  try {
    const witness = await runDisposableCanaryLifecycle(request, {
      launchPlan: context => {
        artifacts = pinArtifacts(context, request);
        preparationDiagnostics = preparationLog(context.isolatedRoot);
        configPath = join(context.isolatedRoot, 'conductor-config.yaml');
        writeFileSync(configPath, renderConfig(context), { mode: 0o600, flag: 'wx' });
        chmodSync(configPath, 0o600);
        if (request.verifyControls) {
          for (const path of [
            join(context.isolatedRoot, SQL_CONTROL_BASENAME),
            join(context.isolatedRoot, WORKFLOW_CONTROL_BASENAME),
            join(context.isolatedRoot, CONTROL_EVIDENCE_BASENAME),
          ]) {
            mkdirSync(path, { mode: 0o700 });
            chmodSync(path, 0o700);
          }
        }
        return {
          executable: artifacts.holochain,
          args: ['--piped', '--structured=Log', '--config-path', configPath],
          cwd: context.isolatedRoot,
          env: conductorEnvironment(context, request),
        };
      },
      prepare: async ({ launch, signal }) => {
        if (!artifacts || !preparationDiagnostics || !configPath)
          throw new Error('approved artifacts were not pinned');
        const deadlineAt = monotonicMs() + PREPARATION_TIMEOUT_MS;
        admin = await connectPreparationAdmin(signal, deadlineAt, preparationDiagnostics.record);
        try {
          if (request.verifyControls && controlVerification) {
            try {
              await verifyPrivateControls({
                admin,
                baseNonce: request.nonce,
                root: launch.isolatedRoot,
                configPath,
                signal,
                deadlineAt,
                witness: controlVerification,
              });
            } catch (error) {
              preparationDiagnostics.record({
                kind: PREPARATION_FAILURE_KIND,
                stage: 'control-verification',
                reason: preparationFailureReason(error, signal),
              });
              throw error;
            }
          }
          await provisionPreparedAdmin(
            admin,
            artifacts.happ,
            signal,
            deadlineAt,
            preparationDiagnostics.record
          );
        } catch (error) {
          const closing = admin;
          admin = undefined;
          await closeAdminBounded(closing);
          throw error;
        }
      },
      trigger: async context => {
        if (!admin || !artifacts) throw new Error('offline canary admin was not prepared');
        const executable = realpathSync(artifacts.holochain);
        const expectedProcess = nativeExpected(context.identity, executable);
        const receipt = await captureHeapPhase(
          async (operation, payload, timeoutMs) => admin!._requester(operation)(payload, timeoutMs),
          {
            nonce: context.nonce,
            phase: context.phase,
            timeoutMs: request.dumpTimeoutMs!,
            dumpTimeoutMs: request.dumpTimeoutMs!,
          },
          expectedProcess,
          monotonicMs,
          context.phase === 'after' ? producerId : undefined
        );
        if (!receipt.completed) throw new Error('native heap capture was refused');
        nativeEvidence.set(context.phase, {
          control: receipt.nativeEvidence,
          process: expectedProcess,
          approvedBinarySha256: request.holochain.sha256,
          dumpTimeoutMs: request.dumpTimeoutMs!,
        });
        if (context.phase === 'before') producerId = receipt.producerId;
        return {
          kind: 'holochain-heap-capture/v1',
          completed: true,
          phase: context.phase,
          nonce: context.nonce,
          identity: context.identity,
        };
      },
      validateArtifact: ({ phase, artifactPath, bytes, signal }) =>
        requireParserTask(artifacts, artifactPath, signal, phase, bytes, nativeEvidence.get(phase)),
    });
    workerResult = {
      ...witness,
      summary: `${witness.summary}; offline canary/pipeline smoke only, preparatory app provisioning excluded`,
      observed: {
        ...witness.observed,
        launcher: {
          kind: 'offline-disposable-heap-canary/v1',
          networkNamespaceVerified: true,
          pidNamespaceVerified: true,
          namespaceInitPidOne: true,
          artifactsFingerprintVerified: true,
          freshStateOnly: true,
          appProvisioningMeasured: false,
          workload: 'offline-pipeline-smoke',
          retainedScope: 'allocations-sampled-after-profiler-activation',
          coverageEligible: false,
        },
        ...(controlVerification ? { controlVerification } : {}),
      },
    };
  } catch (error) {
    failure = workerFailure('lifecycle', error);
  }
  if (admin) {
    try {
      await closeAdminBounded(admin);
    } catch (error) {
      failure ??= workerFailure(WORKER_CLEANUP_STAGE, error);
    }
  }
  if (preparationDiagnostics) {
    try {
      preparationDiagnostics.close();
    } catch (error) {
      failure ??= workerFailure(WORKER_CLEANUP_STAGE, error);
    }
  }
  if (failure) throw failure;
  if (!workerResult) throw workerFailure('lifecycle', undefined);
  return workerResult;
}

function workerExecArgv(): string[] {
  return ['--import', FIXED_TSX_LOADER];
}

type RejectedWorkerEvidenceReason = 'invalid-json' | 'invalid-witness';

function writeRejectedWorkerEvidence(
  root: string,
  reason: RejectedWorkerEvidenceReason,
  stdout: string
): void {
  const stdoutBytes = Buffer.from(stdout, 'utf8');
  if (stdoutBytes.length > WORKER_OUTPUT_BYTES)
    throw new Error('rejected worker stdout exceeds its bound');
  const header = Buffer.from(
    `${JSON.stringify({
      kind: REJECTED_WORKER_EVIDENCE_KIND,
      reason,
      stdoutBytes: stdoutBytes.length,
    })}\n`,
    'utf8'
  );
  if (header.length + stdoutBytes.length > MAX_REJECTED_WORKER_EVIDENCE_BYTES)
    throw new Error('rejected worker evidence exceeds its bound');

  const descriptor = openSync(
    join(root, CANARY_REJECTED_WORKER_EVIDENCE_BASENAME),
    constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL | constants.O_NOFOLLOW,
    0o600
  );
  try {
    fchmodSync(descriptor, 0o600);
    for (const bytes of [header, stdoutBytes]) {
      let offset = 0;
      while (offset < bytes.length) {
        const progress = writeSync(descriptor, bytes, offset, bytes.length - offset);
        if (progress <= 0) throw new Error('rejected worker evidence write made no progress');
        offset += progress;
      }
    }
  } finally {
    closeSync(descriptor);
  }
}

function rejectWorkerOutput(
  root: string,
  reason: RejectedWorkerEvidenceReason,
  stdout: string
): never {
  try {
    writeRejectedWorkerEvidence(root, reason, stdout);
  } catch {
    throw new Error(
      'offline canary worker returned invalid evidence; private evidence retention failed'
    );
  }
  throw new Error('offline canary worker returned invalid evidence');
}

function parseWorkerOutput(
  root: string,
  stdout: string,
  request: IsolatedHeapCanaryRequest
): IsolatedHeapCanaryResult {
  let parsed: unknown;
  try {
    parsed = JSON.parse(stdout);
  } catch {
    return rejectWorkerOutput(root, 'invalid-json', stdout);
  }
  if (!validWorkerResult(parsed, request))
    return rejectWorkerOutput(root, 'invalid-witness', stdout);
  return parsed;
}

async function spawnNamespaceWorker(
  request: IsolatedHeapCanaryRequest
): Promise<IsolatedHeapCanaryResult> {
  const modulePath = fileURLToPath(import.meta.url);
  const parentNet = readlinkSync('/proc/self/ns/net');
  const parentPid = readlinkSync('/proc/self/ns/pid');
  const workerMs = request.lifetimeMs + 2_000;
  const result = await runBoundedTool({
    purpose: 'heap-canary-worker',
    file: UNSHARE,
    args: [
      '--net',
      '--map-root-user',
      '--mount',
      '--propagation',
      'private',
      '--pid',
      '--fork',
      '--kill-child',
      '--mount-proc',
      '--',
      process.execPath,
      ...workerExecArgv(),
      modulePath,
      WORKER_FLAG,
    ],
    cwd: dirname(modulePath),
    env: {
      PATH: MINIMAL_PATH,
      [REQUEST_ENV]: Buffer.from(JSON.stringify(request)).toString('base64url'),
      [PARENT_NET_ENV]: parentNet,
      [PARENT_PID_ENV]: parentPid,
    },
    timeoutMs: workerMs,
    maxOutputBytes: WORKER_OUTPUT_BYTES,
  });
  stopEvidence(result.cleanup);
  if (result.exitCode !== 0 || result.signal !== null) throw publicWorkerFailure(result.stderr);
  return parseWorkerOutput(request.isolatedRoot, result.stdout, request);
}

function record(value: unknown): Record<string, unknown> | null {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function exactKeys(value: Record<string, unknown>, expected: readonly string[]): boolean {
  return Object.keys(value).sort().join(',') === [...expected].sort().join(',');
}

const PHASE_FAILURE_REASONS = new Set([
  'child-exited',
  'child-spawn-failed',
  'descendant-check-failed',
  'descendant-observed',
  'deadline',
  'empty-fifo',
  'fifo-error',
  'native-completion-invalid',
  'native-trigger-failed',
  'overflow',
  'parser-invalid',
  'parser-reap-unresolved',
  'preparation-failed',
  'preparation-reap-unresolved',
  'watchdog-reap-unresolved',
]);

function validParser(value: unknown): boolean {
  const parser = record(value);
  const cleanup = record(parser?.cleanup);
  if (
    !parser ||
    !cleanup ||
    !exactKeys(parser, ['valid', 'reason', 'cleanup']) ||
    !exactKeys(cleanup, ['directWatchdogReaped', 'executingGroupMembers']) ||
    cleanup.directWatchdogReaped !== true ||
    cleanup.executingGroupMembers !== 0
  )
    return false;
  return (
    (parser.valid === true && ['valid', EMPTY_SAMPLED_PROFILE].includes(String(parser.reason))) ||
    (parser.valid === false && ['malformed', 'truncated'].includes(String(parser.reason)))
  );
}

function validPhaseShape(
  value: unknown,
  expectedPhase: 'before' | 'after',
  maxBytes: number
): boolean {
  const phase = record(value);
  if (
    !phase ||
    !exactKeys(phase, [
      'phase',
      'artifact',
      'bytes',
      'overflowSentinelBytes',
      'eofObserved',
      'nativeCompletionObserved',
      'nativeIdentityMatched',
      'parser',
      'boundedLifecycleComplete',
      'reason',
    ]) ||
    phase.phase !== expectedPhase ||
    phase.artifact !== `${expectedPhase}.heap` ||
    !Number.isSafeInteger(phase.bytes) ||
    Number(phase.bytes) < 0 ||
    Number(phase.bytes) > maxBytes ||
    ![0, 1].includes(Number(phase.overflowSentinelBytes)) ||
    typeof phase.eofObserved !== 'boolean' ||
    typeof phase.nativeCompletionObserved !== 'boolean' ||
    typeof phase.nativeIdentityMatched !== 'boolean' ||
    (phase.parser !== null && !validParser(phase.parser)) ||
    typeof phase.boundedLifecycleComplete !== 'boolean' ||
    (phase.reason !== null && !PHASE_FAILURE_REASONS.has(String(phase.reason))) ||
    (phase.boundedLifecycleComplete === false && phase.reason === null)
  )
    return false;
  return phase.boundedLifecycleComplete !== true || phaseComplete(phase, expectedPhase, maxBytes);
}

function phaseComplete(
  phase: Record<string, unknown>,
  expectedPhase: 'before' | 'after',
  maxBytes: number
): boolean {
  const parser = record(phase.parser);
  return (
    phase.phase === expectedPhase &&
    phase.artifact === `${expectedPhase}.heap` &&
    Number.isSafeInteger(phase.bytes) &&
    Number(phase.bytes) >= 1 &&
    Number(phase.bytes) <= maxBytes &&
    phase.overflowSentinelBytes === 0 &&
    phase.eofObserved === true &&
    phase.nativeCompletionObserved === true &&
    phase.nativeIdentityMatched === true &&
    parser?.valid === true &&
    ['valid', EMPTY_SAMPLED_PROFILE].includes(String(parser.reason)) &&
    validParser(parser) &&
    phase.boundedLifecycleComplete === true &&
    phase.reason === null
  );
}

function validControlVerification(
  value: unknown,
  requested: boolean
): value is ControlVerificationWitness {
  if (!requested) return value === undefined;
  const verification = record(value);
  if (
    !verification ||
    !exactKeys(verification, [
      'requested',
      'outcome',
      'requestedSeconds',
      'sequence',
      'coverageEligible',
      'coverageReason',
    ]) ||
    verification.requested !== true ||
    !['passed', 'incomplete'].includes(String(verification.outcome)) ||
    verification.requestedSeconds !== CONTROL_SECONDS ||
    verification.coverageEligible !== false ||
    verification.coverageReason !==
      'quiet private-control lifecycle only; not workload or attribution coverage' ||
    !Array.isArray(verification.sequence) ||
    verification.sequence.length > 4
  )
    return false;
  const expected = [
    ['sqlTiming', 1],
    ['sqlTiming', 2],
    ['workflow', 1],
    ['workflow', 2],
  ] as const;
  for (const [index, value] of verification.sequence.entries()) {
    const generation = record(value);
    if (
      !generation ||
      !exactKeys(generation, [
        'family',
        'generation',
        'status',
        'admission',
        'nativeIdentity',
        'interval',
        'terminalParser',
      ]) ||
      generation.family !== expected[index][0] ||
      generation.generation !== expected[index][1] ||
      generation.status !== 'bound' ||
      generation.admission !== 'exact' ||
      generation.nativeIdentity !== 'exact' ||
      generation.interval !== 'encloses' ||
      generation.terminalParser !== 'valid'
    )
      return false;
  }
  return (
    verification.outcome ===
    (verification.sequence.length === expected.length ? 'passed' : 'incomplete')
  );
}

function validWorkerResult(
  value: unknown,
  request: IsolatedHeapCanaryRequest
): value is IsolatedHeapCanaryResult {
  const result = record(value);
  const observed = record(result?.observed);
  const launcher = record(observed?.launcher);
  const launchBoundary = record(observed?.launchBoundary);
  const observation = record(observed?.observation);
  const child = record(observed?.child);
  const controlVerification = observed?.controlVerification;
  const maxBytes = request.maxBytes ?? MAX_HEAP_DUMP_BYTES;
  const serialized = JSON.stringify(value);
  if (
    !Number.isSafeInteger(maxBytes) ||
    maxBytes < 1 ||
    maxBytes > MAX_HEAP_DUMP_BYTES ||
    !result ||
    !observed ||
    !launcher ||
    !launchBoundary ||
    !observation ||
    !child ||
    !exactKeys(result, ['checkId', 'observed', 'outcome', 'summary']) ||
    !exactKeys(observed, [
      'kind',
      'coverageEligible',
      'coverageReason',
      'launchBoundary',
      'observation',
      'child',
      'phases',
      'launcher',
      ...(request.verifyControls ? ['controlVerification'] : []),
    ]) ||
    !exactKeys(launchBoundary, [
      'freshPrivateEvidenceRoot',
      'stateAndNetworkIsolationVerified',
      'declaration',
      'groupSignalRaceResidual',
    ]) ||
    !exactKeys(observation, ['requestedMs', 'actualMs', 'completed']) ||
    !exactKeys(child, [
      'pid',
      'startTicks',
      'executableDevice',
      'executableInode',
      'watchdogPid',
      'watchdogReaped',
      'nativeExitConfirmed',
      'watchdogExitCode',
      'watchdogSignal',
      'reapUnresolvedReason',
    ]) ||
    !exactKeys(launcher, [
      'kind',
      'networkNamespaceVerified',
      'pidNamespaceVerified',
      'namespaceInitPidOne',
      'artifactsFingerprintVerified',
      'freshStateOnly',
      'appProvisioningMeasured',
      'workload',
      'retainedScope',
      'coverageEligible',
    ]) ||
    result.checkId !== 'runtime-performance' ||
    typeof result.summary !== 'string' ||
    result.summary.length < 1 ||
    result.summary.length > 2_000 ||
    observed.kind !== 'disposable-heap-canary-helper-lifecycle/v1' ||
    observed.coverageEligible !== false ||
    typeof observed.coverageReason !== 'string' ||
    observed.coverageReason.length < 1 ||
    observed.coverageReason.length > 2_000 ||
    launchBoundary.freshPrivateEvidenceRoot !== true ||
    launchBoundary.stateAndNetworkIsolationVerified !== false ||
    launchBoundary.declaration !== 'new-state/no-bootstrap/no-signal/loopback-ephemeral-only' ||
    launchBoundary.groupSignalRaceResidual !== true ||
    observation.requestedMs !== request.observationMs ||
    typeof observation.completed !== 'boolean' ||
    !(
      observation.actualMs === null ||
      (typeof observation.actualMs === 'number' &&
        Number.isFinite(observation.actualMs) &&
        observation.actualMs >= 0)
    ) ||
    !Number.isSafeInteger(child.pid) ||
    Number(child.pid) < 1 ||
    !Number.isSafeInteger(child.watchdogPid) ||
    Number(child.watchdogPid) < 1 ||
    ![child.startTicks, child.executableDevice, child.executableInode].every(
      field => typeof field === 'string' && /^\d+$/.test(field)
    ) ||
    typeof child.watchdogReaped !== 'boolean' ||
    typeof child.nativeExitConfirmed !== 'boolean' ||
    !(
      child.watchdogExitCode === null ||
      (Number.isSafeInteger(child.watchdogExitCode) &&
        Number(child.watchdogExitCode) >= 0 &&
        Number(child.watchdogExitCode) <= 255)
    ) ||
    ![null, 'SIGKILL'].includes(child.watchdogSignal as null | string) ||
    ![null, 'watchdog-reap-unresolved'].includes(child.reapUnresolvedReason as null | string) ||
    !Array.isArray(observed.phases) ||
    observed.phases.length < 1 ||
    observed.phases.length > 2 ||
    !validPhaseShape(observed.phases[0], 'before', maxBytes) ||
    (observed.phases.length === 2 && !validPhaseShape(observed.phases[1], 'after', maxBytes)) ||
    launcher.kind !== 'offline-disposable-heap-canary/v1' ||
    launcher.networkNamespaceVerified !== true ||
    launcher.pidNamespaceVerified !== true ||
    launcher.namespaceInitPidOne !== true ||
    launcher.artifactsFingerprintVerified !== true ||
    launcher.freshStateOnly !== true ||
    launcher.appProvisioningMeasured !== false ||
    launcher.workload !== 'offline-pipeline-smoke' ||
    launcher.retainedScope !== 'allocations-sampled-after-profiler-activation' ||
    launcher.coverageEligible !== false ||
    !validControlVerification(controlVerification, request.verifyControls === true) ||
    Buffer.byteLength(serialized) > WORKER_OUTPUT_BYTES ||
    [
      request.isolatedRoot,
      request.nonce,
      request.holochain.path,
      request.happ.path,
      request.jeprof.path,
    ].some(secret => serialized.includes(secret))
  )
    return false;

  const phases = observed.phases.map(record) as Record<string, unknown>[];
  const controlsPassed =
    request.verifyControls !== true || record(controlVerification)?.outcome === 'passed';
  const passed =
    phases.length === 2 &&
    phaseComplete(phases[0], 'before', maxBytes) &&
    phaseComplete(phases[1], 'after', maxBytes) &&
    observation.completed === true &&
    typeof observation.actualMs === 'number' &&
    observation.actualMs >= request.observationMs &&
    controlsPassed &&
    child.watchdogReaped === true &&
    child.nativeExitConfirmed === true &&
    child.reapUnresolvedReason === null;
  return result.outcome === (passed ? 'passed' : 'skipped');
}

export async function runIsolatedHeapCanary(
  request: IsolatedHeapCanaryRequest,
  worker: WorkerRunner = spawnNamespaceWorker
): Promise<IsolatedHeapCanaryResult> {
  validateRequest(request);
  verifyArtifacts(request);
  return worker(request);
}

export const heapCanaryLauncherTestApi = {
  conductorEnvironment,
  failureDiagnostic: (stage: WorkerFailureStage, error: unknown) =>
    workerFailure(stage, error).diagnostic,
  parseWorkerFailure,
  parseWorkerOutput,
  parserTask,
  pinArtifacts,
  preparationLog,
  preparationFailureReason,
  provisionPreparedAdmin,
  requireBoundControl,
  verifyPrivateControls,
  publicWorkerFailure,
  hashFinalHeapArtifact,
  nativeEvidenceBasename,
  readPrivateNativeHeapEvidence,
  readPrivateNativeHeapEvidencePair,
  writePrivateNativeHeapEvidence,
  renderConfig,
  validWorkerResult,
  writeRejectedWorkerEvidence,
  workerExecArgv,
};

function isInternalWorkerEntry(): boolean {
  if (process.argv[2] !== WORKER_FLAG || !process.argv[1]) return false;
  try {
    return realpathSync(process.argv[1]) === realpathSync(fileURLToPath(import.meta.url));
  } catch {
    return false;
  }
}

if (isInternalWorkerEntry()) {
  void (async () => {
    try {
      const encoded = process.env[REQUEST_ENV];
      if (!encoded) throw new Error('missing internal request');
      const request = JSON.parse(
        Buffer.from(encoded, 'base64url').toString('utf8')
      ) as IsolatedHeapCanaryRequest;
      const result = await runWorker(request);
      process.stdout.write(JSON.stringify(result));
    } catch (error) {
      const failure = workerFailure('request-admission', error);
      process.stderr.write(`${JSON.stringify(failure.diagnostic)}\n`);
      process.exitCode = 1;
    }
  })();
}
