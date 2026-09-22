/**
 * Internal lifecycle primitive for a disposable heap-profiling canary.
 *
 * This deliberately is not a CLI or a generic executable wrapper. The production
 * launcher must first prove that it builds an offline Holochain configuration
 * rooted in the fresh paths supplied here. Tests inject a helper-process launcher
 * and native-trigger stand-in without weakening that future boundary.
 */

import { spawn, spawnSync, type ChildProcess } from 'node:child_process';
import {
  chmodSync,
  closeSync,
  constants,
  fstatSync,
  lstatSync,
  mkdirSync,
  openSync,
  readFileSync,
  readSync,
  realpathSync,
  statSync,
  unlinkSync,
  writeSync,
} from 'node:fs';
import { dirname, isAbsolute, join, normalize } from 'node:path';
import { performance } from 'node:perf_hooks';

import { MAX_HEAP_DUMP_BYTES, TOOL_CLEANUP_TIMEOUT_MS } from './performance-heap.js';

export const MAX_CANARY_LIFETIME_MS = 900_000;
export const MAX_CANARY_DUMP_TIMEOUT_MS = 30_000;
export const MAX_CANARY_OBSERVATION_MS = 600_000;
export const MAX_CANARY_STDERR_BYTES = 64 * 1024;
export const CANARY_STDERR_BASENAME = 'canary.stderr.log';
const FIFO_READ_BYTES = 64 * 1024;
const FIFO_IDLE_POLL_MS = 10;
const DESCENDANT_POLL_MS = 100;
const MKFIFO = '/usr/bin/mkfifo';
const TIMEOUT = '/usr/bin/timeout';
const WATCHDOG_REAP_MS = 1_000;
const PARSER_REAP_MS = TOOL_CLEANUP_TIMEOUT_MS + 250;
const STDERR_TRUNCATION_MARKER = Buffer.from('\n[stderr truncated]\n');
const FIFO_ERROR = 'fifo-error' as const;
const PARSER_INVALID = 'parser-invalid' as const;
const CHILD_EXITED = 'child-exited' as const;

export type CanaryPhase = 'before' | 'after';
export type CanaryFailureReason =
  | 'child-exited'
  | 'child-spawn-failed'
  | 'descendant-check-failed'
  | 'descendant-observed'
  | 'deadline'
  | 'empty-fifo'
  | 'fifo-error'
  | 'native-completion-invalid'
  | 'native-trigger-failed'
  | 'overflow'
  | 'parser-invalid'
  | 'parser-reap-unresolved'
  | 'preparation-failed'
  | 'preparation-reap-unresolved'
  | 'watchdog-reap-unresolved';

export interface CanaryProcessIdentity {
  pid: number;
  startTicks: string;
  executableDevice: string;
  executableInode: string;
}

export interface CanaryLaunchContext {
  /** Fresh mode-0700 root. The launcher must not use state outside it. */
  isolatedRoot: string;
  stateRoot: string;
  heapRoot: string;
  network: {
    reuseExistingState: false;
    bootstrapUrl: null;
    signalUrl: null;
    listen: 'loopback-ephemeral-only';
  };
}

export interface CanaryTriggerContext {
  phase: CanaryPhase;
  nonce: string;
  fifoPath: string;
  identity: CanaryProcessIdentity;
  signal: AbortSignal;
}

export interface NativeHeapCompletion {
  kind: 'holochain-heap-capture/v1';
  completed: true;
  phase: CanaryPhase;
  nonce: string;
  identity: CanaryProcessIdentity;
}

export interface CanaryParserResult {
  valid: boolean;
  reason: 'valid' | 'empty-sampled-profile' | 'malformed' | 'truncated';
  cleanup: CanaryParserStopEvidence;
}

export interface CanaryParserStopEvidence {
  /** The owned GNU timeout direct child emitted close and was reaped by libuv. */
  directWatchdogReaped: true;
  /** A bounded post-close census found no executing member of its PGID. */
  executingGroupMembers: 0;
}

export interface CanaryParserTask {
  result: Promise<CanaryParserResult>;
  /** Cancel and resolve only with the same safe-stop evidence as a result. */
  abortAndConfirmStopped(): Promise<CanaryParserStopEvidence>;
}

export interface CanaryLaunchPlan {
  /** Exact executable passed to GNU timeout without a shell. */
  executable: string;
  args: readonly string[];
  /** Must be the fresh isolated root or a child of it. */
  cwd: string;
  env: NodeJS.ProcessEnv;
}

export interface CanaryLifecycleDependencies {
  /** Production owns construction and validation of the Holochain-only plan. */
  launchPlan(context: CanaryLaunchContext): CanaryLaunchPlan;
  /** Bounded readiness/provisioning before either heap phase begins. */
  prepare?(context: {
    launch: CanaryLaunchContext;
    identity: CanaryProcessIdentity;
    signal: AbortSignal;
  }): Promise<void>;
  /** Stand-in for the future private Admin capture call. */
  trigger(context: CanaryTriggerContext): Promise<NativeHeapCompletion>;
  validateArtifact(context: {
    phase: CanaryPhase;
    artifactPath: string;
    bytes: number;
    signal: AbortSignal;
  }): CanaryParserTask;
  /** Focused-test seam for refusing a zero-progress artifact write. */
  writeArtifact?: typeof writeSync;
  /** Focused-test observation only; not projected into the result. */
  onFifoIdlePoll?: () => void;
  /** Focused-test clock/timer seam for deterministic early-wakeup coverage. */
  observationTimer?: {
    now(): number;
    schedule(callback: () => void, delayMs: number): NodeJS.Timeout;
    cancel(timer: NodeJS.Timeout): void;
  };
}

export interface DisposableCanaryRequest {
  /** Absolute, normalized, nonexistent path explicitly selected for this run. */
  isolatedRoot: string;
  nonce: string;
  lifetimeMs: number;
  observationMs: number;
  dumpTimeoutMs?: number;
  maxBytes?: number;
}

export interface CanaryPhaseWitness {
  phase: CanaryPhase;
  artifact: `${CanaryPhase}.heap`;
  bytes: number;
  overflowSentinelBytes: 0 | 1;
  eofObserved: boolean;
  nativeCompletionObserved: boolean;
  nativeIdentityMatched: boolean;
  parser: CanaryParserResult | null;
  boundedLifecycleComplete: boolean;
  reason: CanaryFailureReason | null;
}

export interface DisposableCanaryWitness {
  checkId: 'runtime-performance';
  outcome: 'passed' | 'skipped';
  summary: string;
  observed: {
    kind: 'disposable-heap-canary-helper-lifecycle/v1';
    coverageEligible: false;
    coverageReason: string;
    launchBoundary: {
      freshPrivateEvidenceRoot: true;
      stateAndNetworkIsolationVerified: false;
      declaration: 'new-state/no-bootstrap/no-signal/loopback-ephemeral-only';
      /** Node exposes no pidfd/group-fd; verify-to-signal remains a narrow race. */
      groupSignalRaceResidual: true;
    };
    observation: {
      requestedMs: number;
      actualMs: number | null;
      completed: boolean;
    };
    child: CanaryProcessIdentity & {
      watchdogPid: number;
      /** Wrapper close was observed and the pinned native identity is gone. */
      watchdogReaped: boolean;
      nativeExitConfirmed: boolean;
      watchdogExitCode: number | null;
      watchdogSignal: NodeJS.Signals | null;
      reapUnresolvedReason: 'watchdog-reap-unresolved' | null;
    };
    phases: CanaryPhaseWitness[];
  };
}

interface ChildClose {
  exitCode: number | null;
  signal: NodeJS.Signals | null;
}

interface WatchdogReap extends ChildClose {
  reaped: boolean;
  nativeExitConfirmed: boolean;
  unresolvedReason: 'watchdog-reap-unresolved' | null;
}

interface ExecutableIdentity {
  device: string;
  inode: string;
}

interface FailureSignal {
  promise: Promise<CanaryFailureReason>;
  fail(reason: CanaryFailureReason): void;
}

interface FifoPump {
  completion: Promise<{ bytes: number; eofObserved: boolean; overflow: boolean }>;
  failure: Promise<CanaryFailureReason>;
  overflow: Promise<CanaryFailureReason>;
  closeAnchor(): void;
  closeAll(): void;
}

function failureSignal(): FailureSignal {
  let resolveFailure!: (reason: CanaryFailureReason) => void;
  let failed = false;
  return {
    promise: new Promise(resolve => {
      resolveFailure = resolve;
    }),
    fail: reason => {
      if (failed) return;
      failed = true;
      resolveFailure(reason);
    },
  };
}

function validateRequest(request: DisposableCanaryRequest): {
  dumpTimeoutMs: number;
  maxBytes: number;
} {
  if (
    !isAbsolute(request.isolatedRoot) ||
    normalize(request.isolatedRoot) !== request.isolatedRoot ||
    request.isolatedRoot === '/'
  )
    throw new Error('isolatedRoot must be an absolute normalized non-root path');
  if (!/^[A-Za-z0-9_-]{1,64}$/.test(request.nonce))
    throw new Error('nonce must contain 1..64 safe ASCII characters');
  if (
    !Number.isInteger(request.lifetimeMs) ||
    request.lifetimeMs < 1 ||
    request.lifetimeMs > MAX_CANARY_LIFETIME_MS
  )
    throw new Error(`lifetimeMs must be in 1..${MAX_CANARY_LIFETIME_MS}`);
  const dumpTimeoutMs = request.dumpTimeoutMs ?? MAX_CANARY_DUMP_TIMEOUT_MS;
  if (
    !Number.isInteger(dumpTimeoutMs) ||
    dumpTimeoutMs < 1 ||
    dumpTimeoutMs > MAX_CANARY_DUMP_TIMEOUT_MS
  )
    throw new Error(`dumpTimeoutMs must be in 1..${MAX_CANARY_DUMP_TIMEOUT_MS}`);
  if (
    !Number.isInteger(request.observationMs) ||
    request.observationMs < 1 ||
    request.observationMs > MAX_CANARY_OBSERVATION_MS
  )
    throw new Error(`observationMs must be in 1..${MAX_CANARY_OBSERVATION_MS}`);
  if (request.observationMs + 2 * dumpTimeoutMs > request.lifetimeMs)
    throw new Error('observationMs plus both dump deadlines must fit lifetimeMs');
  const maxBytes = request.maxBytes ?? MAX_HEAP_DUMP_BYTES;
  if (!Number.isInteger(maxBytes) || maxBytes < 1 || maxBytes > MAX_HEAP_DUMP_BYTES)
    throw new Error(`maxBytes must be in 1..${MAX_HEAP_DUMP_BYTES}`);
  const parent = dirname(request.isolatedRoot);
  if (realpathSync(parent) !== parent)
    throw new Error('isolatedRoot parent must not traverse a symlink');
  return { dumpTimeoutMs, maxBytes };
}

async function observeInterval(options: {
  durationMs: number;
  fatal: Promise<CanaryFailureReason>;
  childClose: Promise<ChildClose>;
  timer?: NonNullable<CanaryLifecycleDependencies['observationTimer']>;
}): Promise<{ actualMs: number; failure: CanaryFailureReason | null }> {
  const observationTimer =
    options.timer ??
    ({
      now: () => performance.now(),
      schedule: (callback, delayMs) => setTimeout(callback, delayMs),
      cancel: timer => clearTimeout(timer),
    } satisfies NonNullable<CanaryLifecycleDependencies['observationTimer']>);
  const startedAt = observationTimer.now();
  const childClosed = options.childClose.then(() => CHILD_EXITED);
  let pendingTimer: NodeJS.Timeout | undefined;
  try {
    for (;;) {
      const actualMs = observationTimer.now() - startedAt;
      if (actualMs >= options.durationMs) return { actualMs, failure: null };

      let firedSynchronously = false;
      const elapsed = new Promise<'elapsed'>(resolve => {
        const scheduled = observationTimer.schedule(
          () => {
            firedSynchronously = true;
            pendingTimer = undefined;
            resolve('elapsed');
          },
          Math.max(1, Math.ceil(options.durationMs - actualMs))
        );
        if (!firedSynchronously) pendingTimer = scheduled;
      });
      const result = await Promise.race([elapsed, options.fatal, childClosed]);
      if (result !== 'elapsed') {
        return {
          actualMs: Math.max(0, observationTimer.now() - startedAt),
          failure: result,
        };
      }
    }
  } finally {
    if (pendingTimer) observationTimer.cancel(pendingTimer);
  }
}

async function boundedAcknowledgement(work: Promise<void>, timeoutMs: number): Promise<boolean> {
  let timer: NodeJS.Timeout | undefined;
  try {
    return await Promise.race([
      work.then(
        () => true,
        () => false
      ),
      new Promise<boolean>(resolve => {
        timer = setTimeout(() => resolve(false), timeoutMs);
      }),
    ]);
  } finally {
    if (timer) clearTimeout(timer);
  }
}

async function abortParserAndAcknowledge(task: CanaryParserTask): Promise<boolean> {
  try {
    return await boundedAcknowledgement(
      task.abortAndConfirmStopped().then(cleanup => {
        if (cleanup.directWatchdogReaped !== true || cleanup.executingGroupMembers !== 0)
          throw new Error('parser safe-stop evidence is incomplete');
      }),
      PARSER_REAP_MS
    );
  } catch {
    return false;
  }
}

function createPrivateRoots(isolatedRoot: string): CanaryLaunchContext {
  mkdirSync(isolatedRoot, { mode: 0o700 });
  chmodSync(isolatedRoot, 0o700);
  const stateRoot = join(isolatedRoot, 'state');
  const heapRoot = join(isolatedRoot, 'heap');
  mkdirSync(stateRoot, { mode: 0o700 });
  mkdirSync(heapRoot, { mode: 0o700 });
  chmodSync(stateRoot, 0o700);
  chmodSync(heapRoot, 0o700);
  return {
    isolatedRoot,
    stateRoot,
    heapRoot,
    network: {
      reuseExistingState: false,
      bootstrapUrl: null,
      signalUrl: null,
      listen: 'loopback-ephemeral-only',
    },
  };
}

function processStat(pid: number): { startTicks: string; processGroup: number } {
  const stat = readFileSync(`/proc/${pid}/stat`, 'utf8');
  const commandEnd = stat.lastIndexOf(')');
  if (commandEnd < 0) throw new Error('child process stat is malformed');
  const fieldsAfterCommand = stat
    .slice(commandEnd + 1)
    .trim()
    .split(/\s+/);
  const startTicks = fieldsAfterCommand[19];
  if (!/^\d+$/.test(startTicks ?? '')) throw new Error('child start ticks are unavailable');
  const processGroup = Number(fieldsAfterCommand[2]);
  if (!Number.isSafeInteger(processGroup) || processGroup <= 1)
    throw new Error('child process group is unavailable');
  return { startTicks, processGroup };
}

function processIdentity(pid: number): CanaryProcessIdentity {
  const before = processStat(pid);
  const executable = statSync(`/proc/${pid}/exe`, { bigint: true });
  const after = processStat(pid);
  if (before.startTicks !== after.startTicks || before.processGroup !== after.processGroup)
    throw new Error('child identity changed while reading executable');
  return {
    pid,
    startTicks: before.startTicks,
    executableDevice: executable.dev.toString(),
    executableInode: executable.ino.toString(),
  };
}

function executableIdentity(path: string): ExecutableIdentity {
  if (!isAbsolute(path)) throw new Error('canary executable must be absolute');
  const executable = statSync(realpathSync(path), { bigint: true });
  if (!executable.isFile()) throw new Error('canary executable must be a regular file');
  return { device: executable.dev.toString(), inode: executable.ino.toString() };
}

function launchedExecutableIdentity(path: string): ExecutableIdentity {
  const requested = executableIdentity(path);
  let descriptor = -1;
  try {
    descriptor = openSync(path, constants.O_RDONLY | constants.O_NOFOLLOW);
    const header = Buffer.alloc(256);
    const bytes = readSync(descriptor, header, 0, header.length, 0);
    const firstLine = header.subarray(0, bytes).toString('utf8').split(/\r?\n/, 1)[0];
    if (!firstLine.startsWith('#!')) return requested;
    const interpreter = firstLine.slice(2).trim().split(/\s+/, 1)[0];
    if (!isAbsolute(interpreter)) throw new Error('watchdog shebang interpreter must be absolute');
    return executableIdentity(interpreter);
  } finally {
    if (descriptor >= 0) closeSync(descriptor);
  }
}

function validateLaunchPlan(
  context: CanaryLaunchContext,
  plan: CanaryLaunchPlan
): ExecutableIdentity {
  if (
    !isAbsolute(plan.cwd) ||
    (plan.cwd !== context.isolatedRoot && !plan.cwd.startsWith(`${context.isolatedRoot}/`))
  )
    throw new Error('canary cwd must remain inside isolatedRoot');
  if (plan.args.length > 256 || plan.args.some(arg => Buffer.byteLength(arg) > 4096))
    throw new Error('canary argv exceeds its bounded launch envelope');
  return executableIdentity(plan.executable);
}

function timeoutDuration(milliseconds: number): string {
  return `${(milliseconds / 1000).toFixed(3)}s`;
}

function boundedStderrCapture(path: string): {
  attach(child: ChildProcess): void;
  close(): void;
} {
  const descriptor = openSync(
    path,
    constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL | constants.O_NOFOLLOW,
    0o600
  );
  chmodSync(path, 0o600);
  let bytes = 0;
  let closed = false;
  let discard = false;
  const close = (): void => {
    if (closed) return;
    closed = true;
    closeSync(descriptor);
  };
  const writeAll = (chunk: Buffer, position: number | null = null): void => {
    let offset = 0;
    while (offset < chunk.length) {
      const progress = writeSync(
        descriptor,
        chunk,
        offset,
        chunk.length - offset,
        position === null ? null : position + offset
      );
      if (progress <= 0) throw new Error('stderr artifact write made no progress');
      offset += progress;
    }
  };
  const truncate = (): void => {
    writeAll(STDERR_TRUNCATION_MARKER, MAX_CANARY_STDERR_BYTES - STDERR_TRUNCATION_MARKER.length);
    discard = true;
  };
  return {
    attach: child => {
      if (!child.stderr) {
        close();
        throw new Error('canary stderr pipe is unavailable');
      }
      child.stderr.on('data', (chunk: Buffer) => {
        if (discard || closed) return;
        try {
          const retained = Math.min(chunk.length, MAX_CANARY_STDERR_BYTES - bytes);
          if (retained > 0) {
            writeAll(chunk.subarray(0, retained));
            bytes += retained;
          }
          if (retained < chunk.length) truncate();
        } catch {
          // Continue draining the pipe even when the private diagnostic artifact
          // becomes unwritable; never backpressure the owned native child.
          discard = true;
        }
      });
      child.stderr.once('error', () => {
        discard = true;
      });
      child.stderr.once('close', close);
      child.once('error', close);
    },
    close,
  };
}

function spawnWatchdog(
  plan: CanaryLaunchPlan,
  lifetimeMs: number,
  stderrPath: string
): ChildProcess {
  const stderrCapture = boundedStderrCapture(stderrPath);
  let child: ChildProcess;
  try {
    child = spawn(
      TIMEOUT,
      ['--signal=KILL', timeoutDuration(lifetimeMs), plan.executable, ...plan.args],
      {
        cwd: plan.cwd,
        env: plan.env,
        detached: true,
        shell: false,
        stdio: ['ignore', 'ignore', 'pipe'],
      }
    );
    stderrCapture.attach(child);
    return child;
  } catch (error) {
    stderrCapture.close();
    throw error;
  }
}

function identitiesMatch(left: CanaryProcessIdentity, right: CanaryProcessIdentity): boolean {
  return (
    left.pid === right.pid &&
    left.startTicks === right.startTicks &&
    left.executableDevice === right.executableDevice &&
    left.executableInode === right.executableInode
  );
}

function mainThreadChildren(pid: number): number[] {
  try {
    return readFileSync(`/proc/${pid}/task/${pid}/children`, 'utf8')
      .trim()
      .split(/\s+/)
      .filter(Boolean)
      .map(Number)
      .filter(childPid => Number.isSafeInteger(childPid) && childPid > 1);
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') return [];
    throw error;
  }
}

function observedExit(child: ChildProcess): boolean {
  return child.exitCode !== null || child.signalCode !== null;
}

async function waitForOwnedNativeChild(options: {
  watchdog: ChildProcess;
  watchdogClose: Promise<ChildClose>;
  watchdogExecutable: ExecutableIdentity;
  nativeExecutable: ExecutableIdentity;
  deadlineAt: number;
}): Promise<{ watchdogIdentity: CanaryProcessIdentity; nativeIdentity: CanaryProcessIdentity }> {
  const watchdogPid = options.watchdog.pid;
  if (!watchdogPid) throw new Error('watchdog has no pid');
  while (performance.now() < options.deadlineAt) {
    try {
      const watchdogIdentity = processIdentity(watchdogPid);
      const watchdogStat = processStat(watchdogPid);
      const watchdogMatches =
        watchdogIdentity.executableDevice === options.watchdogExecutable.device &&
        watchdogIdentity.executableInode === options.watchdogExecutable.inode &&
        watchdogStat.processGroup === watchdogPid;
      if (watchdogMatches) {
        const children = mainThreadChildren(watchdogPid);
        if (children.length > 1) throw new Error('watchdog has multiple direct children');
        if (children.length === 1) {
          const nativeIdentity = processIdentity(children[0]);
          const nativeStat = processStat(children[0]);
          if (
            nativeIdentity.executableDevice !== options.nativeExecutable.device ||
            nativeIdentity.executableInode !== options.nativeExecutable.inode
          ) {
            // `timeout` forks before exec; observe again while the child still
            // has the wrapper image. A persistently wrong image reaches the
            // deadline and is never accepted as the canary.
            await new Promise<void>(resolve => setTimeout(resolve, FIFO_IDLE_POLL_MS));
            continue;
          }
          if (nativeStat.processGroup !== watchdogPid)
            throw new Error('watchdog child escaped the owned process group');
          return { watchdogIdentity, nativeIdentity };
        }
      }
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
    }
    const state = await Promise.race([
      new Promise<'poll'>(resolve => setTimeout(() => resolve('poll'), FIFO_IDLE_POLL_MS)),
      options.watchdogClose.then(() => 'closed' as const),
    ]);
    if (state === 'closed') throw new Error('watchdog exited before native identity was pinned');
  }
  throw new Error('watchdog did not expose the native child before its deadline');
}

function ownedGroupStillMatches(options: {
  watchdog: ChildProcess;
  watchdogIdentity: CanaryProcessIdentity;
  nativeIdentity: CanaryProcessIdentity;
}): boolean {
  if (!options.watchdog.pid || observedExit(options.watchdog)) return false;
  try {
    const currentWatchdog = processIdentity(options.watchdog.pid);
    const currentNative = processIdentity(options.nativeIdentity.pid);
    return (
      identitiesMatch(currentWatchdog, options.watchdogIdentity) &&
      identitiesMatch(currentNative, options.nativeIdentity) &&
      processStat(options.watchdog.pid).processGroup === options.watchdog.pid &&
      processStat(options.nativeIdentity.pid).processGroup === options.watchdog.pid
    );
  } catch {
    return false;
  }
}

function observeChild(child: ChildProcess): {
  close: Promise<ChildClose>;
  spawnFailure: Promise<CanaryFailureReason>;
} {
  const spawnFailure = failureSignal();
  const close = new Promise<ChildClose>(resolve => {
    child.once('error', () => spawnFailure.fail('child-spawn-failed'));
    child.once('close', (exitCode, signal) => resolve({ exitCode, signal }));
  });
  return { close, spawnFailure: spawnFailure.promise };
}

function mkfifo(path: string): void {
  const made = spawnSync(MKFIFO, ['-m', '600', path], {
    encoding: 'utf8',
    timeout: 5_000,
  });
  if (made.status !== 0 || made.error) throw new Error('failed to create private capture FIFO');
  const fifo = lstatSync(path);
  if (!fifo.isFIFO() || (fifo.mode & 0o777) !== 0o600)
    throw new Error('capture FIFO is not a private FIFO');
}

function prepareCaptureFifos(heapRoot: string): Record<CanaryPhase, string> {
  const paths = {
    before: join(heapRoot, 'before.heap.fifo'),
    after: join(heapRoot, 'after.heap.fifo'),
  };
  try {
    mkfifo(paths.before);
    mkfifo(paths.after);
    return paths;
  } catch (error) {
    for (const path of Object.values(paths)) {
      try {
        unlinkSync(path);
      } catch {
        // Preserve the bounded FIFO preparation failure.
      }
    }
    throw error;
  }
}

function openFifoPump(
  fifoPath: string,
  artifactPath: string,
  maxBytes: number,
  writeArtifact: typeof writeSync,
  onIdlePoll?: () => void
): FifoPump {
  let reader = -1;
  let anchor = -1;
  let output = -1;
  let closed = false;
  const overflow = failureSignal();
  const failure = failureSignal();
  try {
    reader = openSync(fifoPath, constants.O_RDONLY | constants.O_NONBLOCK | constants.O_NOFOLLOW);
    if (!fstatSync(reader).isFIFO()) throw new Error('opened capture source is not a FIFO');
    // Holding one writer prevents a pre-native read from being misread as EOF.
    anchor = openSync(fifoPath, constants.O_WRONLY | constants.O_NONBLOCK | constants.O_NOFOLLOW);
    output = openSync(
      artifactPath,
      constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL | constants.O_NOFOLLOW,
      0o600
    );
    chmodSync(artifactPath, 0o600);
  } catch (error) {
    if (reader >= 0) closeSync(reader);
    if (anchor >= 0) closeSync(anchor);
    if (output >= 0) closeSync(output);
    throw error;
  }

  const closeDescriptor = (descriptor: number): number => {
    if (descriptor >= 0) closeSync(descriptor);
    return -1;
  };
  const closeAnchor = (): void => {
    anchor = closeDescriptor(anchor);
  };
  const closeAll = (): void => {
    if (closed) return;
    closed = true;
    reader = closeDescriptor(reader);
    anchor = closeDescriptor(anchor);
    output = closeDescriptor(output);
  };
  const completion = (async (): Promise<{
    bytes: number;
    eofObserved: boolean;
    overflow: boolean;
  }> => {
    let bytes = 0;
    const buffer = Buffer.alloc(FIFO_READ_BYTES);
    try {
      while (!closed) {
        const remainingWithSentinel = Math.min(buffer.length, maxBytes - bytes + 1);
        let count: number;
        try {
          count = readSync(reader, buffer, 0, remainingWithSentinel, null);
        } catch (error) {
          if ((error as NodeJS.ErrnoException).code === 'EAGAIN') {
            onIdlePoll?.();
            await new Promise<void>(resolve => setTimeout(resolve, FIFO_IDLE_POLL_MS));
            continue;
          }
          throw error;
        }
        if (count === 0) return { bytes, eofObserved: true, overflow: false };
        const persist = Math.min(count, maxBytes - bytes);
        let written = 0;
        while (written < persist) {
          const progress = writeArtifact(output, buffer, written, persist - written, null);
          if (progress <= 0) throw new Error('artifact write made no progress');
          written += progress;
        }
        bytes += persist;
        if (count > persist) {
          overflow.fail('overflow');
          return { bytes, eofObserved: false, overflow: true };
        }
      }
      return { bytes, eofObserved: false, overflow: false };
    } catch (error) {
      failure.fail(FIFO_ERROR);
      throw error;
    } finally {
      closeAll();
    }
  })();
  return {
    completion,
    failure: failure.promise,
    overflow: overflow.promise,
    closeAnchor,
    closeAll,
  };
}

function phaseWitness(phase: CanaryPhase): CanaryPhaseWitness {
  return {
    phase,
    artifact: `${phase}.heap`,
    bytes: 0,
    overflowSentinelBytes: 0,
    eofObserved: false,
    nativeCompletionObserved: false,
    nativeIdentityMatched: false,
    parser: null,
    boundedLifecycleComplete: false,
    reason: null,
  };
}

async function capturePhase(options: {
  phase: CanaryPhase;
  context: CanaryLaunchContext;
  identity: CanaryProcessIdentity;
  request: DisposableCanaryRequest;
  dependencies: CanaryLifecycleDependencies;
  fifoPath: string;
  maxBytes: number;
  deadlineAt: number;
  childClose: Promise<ChildClose>;
  fatal: Promise<CanaryFailureReason>;
}): Promise<CanaryPhaseWitness> {
  const witness = phaseWitness(options.phase);
  const fifoPath = options.fifoPath;
  const artifactPath = join(options.context.heapRoot, witness.artifact);
  const timeoutMs = Math.min(
    options.request.dumpTimeoutMs ?? MAX_CANARY_DUMP_TIMEOUT_MS,
    Math.max(1, options.deadlineAt - performance.now())
  );
  let pump: FifoPump | undefined;
  let timer: NodeJS.Timeout | undefined;
  const controller = new AbortController();
  try {
    pump = openFifoPump(
      fifoPath,
      artifactPath,
      options.maxBytes,
      options.dependencies.writeArtifact ?? writeSync,
      options.dependencies.onFifoIdlePoll
    );
    const pumpResult = pump.completion.then(
      value => ({ value }),
      () => ({ failure: FIFO_ERROR })
    );
    const timeout = new Promise<CanaryFailureReason>(resolve => {
      timer = setTimeout(() => resolve('deadline'), timeoutMs);
    });
    const childExited = options.childClose.then(() => CHILD_EXITED);
    const trigger = options.dependencies
      .trigger({
        phase: options.phase,
        nonce: options.request.nonce,
        fifoPath,
        identity: options.identity,
        signal: controller.signal,
      })
      .then(
        completion => ({ completion }),
        () => ({ failure: 'native-trigger-failed' as const })
      );
    const native = await Promise.race([
      trigger,
      pump.failure.then(failure => ({ failure })),
      pump.overflow.then(failure => ({ failure })),
      timeout.then(failure => ({ failure })),
      childExited.then(failure => ({ failure })),
      options.fatal.then(failure => ({ failure })),
    ]);
    if ('failure' in native) {
      witness.reason = native.failure;
      witness.overflowSentinelBytes = native.failure === 'overflow' ? 1 : 0;
      controller.abort();
      pump.closeAll();
      const streamed = await pumpResult;
      if ('value' in streamed) {
        witness.bytes = streamed.value.bytes;
        witness.eofObserved = streamed.value.eofObserved;
      }
      return witness;
    }
    witness.nativeCompletionObserved = true;
    witness.nativeIdentityMatched =
      native.completion.kind === 'holochain-heap-capture/v1' &&
      native.completion.completed &&
      native.completion.phase === options.phase &&
      native.completion.nonce === options.request.nonce &&
      identitiesMatch(native.completion.identity, options.identity);
    if (!witness.nativeIdentityMatched) {
      witness.reason = 'native-completion-invalid';
      controller.abort();
      pump.closeAll();
      const streamed = await pumpResult;
      if ('value' in streamed) {
        witness.bytes = streamed.value.bytes;
        witness.eofObserved = streamed.value.eofObserved;
      }
      return witness;
    }
    pump.closeAnchor();
    const streamed = await Promise.race([
      pumpResult,
      timeout.then(failure => ({ failure })),
      childExited.then(failure => ({ failure })),
      options.fatal.then(failure => ({ failure })),
    ]);
    if ('failure' in streamed) {
      witness.reason = streamed.failure;
      controller.abort();
      pump.closeAll();
      return witness;
    }
    witness.bytes = streamed.value.bytes;
    witness.eofObserved = streamed.value.eofObserved;
    witness.overflowSentinelBytes = streamed.value.overflow ? 1 : 0;
    if (streamed.value.overflow) {
      witness.reason = 'overflow';
      return witness;
    }
    if (!streamed.value.eofObserved || streamed.value.bytes === 0) {
      witness.reason = 'empty-fifo';
      return witness;
    }
    let parserTask: CanaryParserTask;
    try {
      parserTask = options.dependencies.validateArtifact({
        phase: options.phase,
        artifactPath,
        bytes: streamed.value.bytes,
        signal: controller.signal,
      });
    } catch {
      witness.reason = PARSER_INVALID;
      return witness;
    }
    const parser = parserTask.result.then(
      value => ({ value }),
      () => ({ parserFailure: true as const })
    );
    const parsed = await Promise.race([
      parser,
      timeout.then(failure => ({ failure })),
      childExited.then(failure => ({ failure })),
      options.fatal.then(failure => ({ failure })),
    ]);
    if ('failure' in parsed) {
      controller.abort();
      const reaped = await abortParserAndAcknowledge(parserTask);
      witness.reason = reaped ? parsed.failure : 'parser-reap-unresolved';
      return witness;
    }
    if ('parserFailure' in parsed) {
      controller.abort();
      const reaped = await abortParserAndAcknowledge(parserTask);
      witness.reason = reaped ? PARSER_INVALID : 'parser-reap-unresolved';
      return witness;
    }
    witness.parser = parsed.value;
    if (
      !witness.parser.valid ||
      witness.parser.cleanup.directWatchdogReaped !== true ||
      witness.parser.cleanup.executingGroupMembers !== 0
    ) {
      witness.reason = PARSER_INVALID;
      return witness;
    }
    witness.boundedLifecycleComplete = true;
    return witness;
  } catch {
    witness.reason = FIFO_ERROR;
    return witness;
  } finally {
    if (timer) clearTimeout(timer);
    controller.abort();
    pump?.closeAll();
  }
}

async function waitForClose(
  close: Promise<ChildClose>,
  timeoutMs: number
): Promise<ChildClose | null> {
  let timer: NodeJS.Timeout | undefined;
  try {
    return await Promise.race([
      close,
      new Promise<null>(resolve => {
        timer = setTimeout(() => resolve(null), timeoutMs);
      }),
    ]);
  } finally {
    if (timer) clearTimeout(timer);
  }
}

function identityPresence(identity: CanaryProcessIdentity): 'exact' | 'gone' | 'unverified' {
  try {
    return identitiesMatch(processIdentity(identity.pid), identity) ? 'exact' : 'gone';
  } catch (error) {
    return (error as NodeJS.ErrnoException).code === 'ENOENT' ? 'gone' : 'unverified';
  }
}

async function waitForIdentityGone(
  identity: CanaryProcessIdentity,
  deadlineAt: number
): Promise<boolean> {
  while (performance.now() < deadlineAt) {
    if (identityPresence(identity) === 'gone') return true;
    await new Promise<void>(resolve => setTimeout(resolve, FIFO_IDLE_POLL_MS));
  }
  return identityPresence(identity) === 'gone';
}

async function terminateOwnedWatchdog(options: {
  watchdog: ChildProcess;
  close: Promise<ChildClose>;
  watchdogIdentity: CanaryProcessIdentity;
  nativeIdentity: CanaryProcessIdentity;
}): Promise<WatchdogReap> {
  const reapDeadlineAt = performance.now() + WATCHDOG_REAP_MS;
  if (!observedExit(options.watchdog) && ownedGroupStillMatches(options)) {
    try {
      // Revalidated immediately above, but Node exposes no pidfd/group-fd. A
      // narrow verify-to-signal race remains and is disclosed in the witness.
      process.kill(-options.watchdogIdentity.pid, 'SIGKILL');
    } catch {
      // GNU timeout remains the independent final KILL authority at lifetime.
    }
  }
  const close = await waitForClose(options.close, Math.max(1, reapDeadlineAt - performance.now()));
  const nativeExitConfirmed = close
    ? await waitForIdentityGone(options.nativeIdentity, reapDeadlineAt)
    : false;
  return close && nativeExitConfirmed
    ? { ...close, reaped: true, nativeExitConfirmed: true, unresolvedReason: null }
    : {
        exitCode: close?.exitCode ?? null,
        signal: close?.signal ?? null,
        reaped: false,
        nativeExitConfirmed,
        unresolvedReason: 'watchdog-reap-unresolved',
      };
}

/**
 * Exercise the bounded lifecycle. Production code must wrap this with a
 * reviewed Holochain-only launcher; this seam alone does not authorize a
 * command, network, state directory, PID attachment, or Admin API endpoint.
 */
export async function runDisposableCanaryLifecycle(
  request: DisposableCanaryRequest,
  dependencies: CanaryLifecycleDependencies
): Promise<DisposableCanaryWitness> {
  const { dumpTimeoutMs, maxBytes } = validateRequest(request);
  request = { ...request, dumpTimeoutMs, maxBytes };
  const context = createPrivateRoots(request.isolatedRoot);
  const fifoPaths = prepareCaptureFifos(context.heapRoot);
  let watchdog: ChildProcess | undefined;
  try {
    let plan: CanaryLaunchPlan;
    let nativeExecutable: ExecutableIdentity;
    try {
      plan = dependencies.launchPlan(context);
      nativeExecutable = validateLaunchPlan(context, plan);
    } catch {
      throw new Error('canary launch plan was refused');
    }
    // Linux exposes a shebang interpreter, rather than the script inode, via
    // `/proc/<pid>/exe`. Pin the requested timeout entrypoint above and the
    // actual interpreter image here (coreutils on this host).
    const watchdogExecutable = launchedExecutableIdentity(TIMEOUT);
    watchdog = spawnWatchdog(
      plan,
      request.lifetimeMs,
      join(context.isolatedRoot, CANARY_STDERR_BASENAME)
    );
    const deadlineAt = performance.now() + request.lifetimeMs;
    // Install error/close handlers before inspecting pid. `spawn()` reports an
    // exec failure asynchronously and has no pid in precisely that case.
    const observedWatchdog = observeChild(watchdog);
    if (!watchdog.pid) {
      await Promise.race([observedWatchdog.spawnFailure, observedWatchdog.close]);
      await observedWatchdog.close;
      throw new Error('watchdog spawn failed before assigning a direct-child pid');
    }
    let identities: {
      watchdogIdentity: CanaryProcessIdentity;
      nativeIdentity: CanaryProcessIdentity;
    };
    try {
      identities = await waitForOwnedNativeChild({
        watchdog,
        watchdogClose: observedWatchdog.close,
        watchdogExecutable,
        nativeExecutable,
        deadlineAt,
      });
    } catch {
      // Without the exact native member identity there is no safe group target.
      // Leave the already-armed GNU timeout as final authority and bound our wait.
      await waitForClose(observedWatchdog.close, WATCHDOG_REAP_MS);
      throw new Error('owned native canary identity was unavailable');
    }
    const { watchdogIdentity, nativeIdentity: identity } = identities;

    const fatal = failureSignal();
    // Advisory in-process observation only. GNU timeout owns the hard child
    // lifetime even when this event loop is blocked.
    const lifetimeTimer = setTimeout(
      () => fatal.fail('deadline'),
      Math.max(1, deadlineAt - performance.now())
    );
    // This catches children visible from the leader thread as a fail-closed
    // diagnostic. It is not proof that another thread never spawned a child;
    // the production wrapper owns the no-descendant executable constraint.
    const descendantTimer = setInterval(() => {
      try {
        if (mainThreadChildren(identity.pid).length > 0) fatal.fail('descendant-observed');
      } catch {
        fatal.fail('descendant-check-failed');
      }
    }, DESCENDANT_POLL_MS);
    const phases: CanaryPhaseWitness[] = [];
    const observation = {
      requestedMs: request.observationMs,
      actualMs: null as number | null,
      completed: false,
    };
    let close: WatchdogReap = {
      exitCode: null,
      signal: null,
      reaped: false,
      nativeExitConfirmed: false,
      unresolvedReason: 'watchdog-reap-unresolved',
    };
    try {
      const firstFatal = Promise.race([fatal.promise, observedWatchdog.spawnFailure]);
      let prepared = true;
      if (dependencies.prepare) {
        const preparationController = new AbortController();
        try {
          const preparation = dependencies
            .prepare({ launch: context, identity, signal: preparationController.signal })
            .then(
              () => 'prepared' as const,
              () => 'preparation-failed' as const
            );
          const preparationOutcome = await Promise.race([
            preparation,
            firstFatal,
            observedWatchdog.close.then(() => CHILD_EXITED),
          ]);
          if (preparationOutcome !== 'prepared') {
            preparationController.abort();
            const stopped = await boundedAcknowledgement(
              preparation.then(() => undefined),
              PARSER_REAP_MS
            );
            const witness = phaseWitness('before');
            witness.reason = stopped ? preparationOutcome : 'preparation-reap-unresolved';
            phases.push(witness);
            prepared = false;
          }
        } finally {
          preparationController.abort();
        }
      }
      for (const phase of prepared ? (['before', 'after'] as const) : []) {
        if (performance.now() >= deadlineAt) {
          const witness = phaseWitness(phase);
          witness.reason = 'deadline';
          phases.push(witness);
          break;
        }
        const witness = await capturePhase({
          phase,
          context,
          identity,
          request,
          dependencies,
          fifoPath: fifoPaths[phase],
          maxBytes,
          deadlineAt,
          childClose: observedWatchdog.close,
          fatal: firstFatal,
        });
        phases.push(witness);
        if (!witness.boundedLifecycleComplete) break;
        if (phase === 'before') {
          const interval = await observeInterval({
            durationMs: request.observationMs,
            fatal: firstFatal,
            childClose: observedWatchdog.close,
            timer: dependencies.observationTimer,
          });
          observation.actualMs = interval.actualMs;
          observation.completed = interval.failure === null;
          if (interval.failure) {
            const after = phaseWitness('after');
            after.reason = interval.failure;
            phases.push(after);
            break;
          }
        }
      }
    } finally {
      clearTimeout(lifetimeTimer);
      clearInterval(descendantTimer);
      close = await terminateOwnedWatchdog({
        watchdog,
        close: observedWatchdog.close,
        watchdogIdentity,
        nativeIdentity: identity,
      });
    }
    const phasesComplete =
      phases.length === 2 && phases.every(phase => phase.boundedLifecycleComplete);
    const complete = phasesComplete && close.reaped;
    return {
      checkId: 'runtime-performance',
      outcome: complete ? 'passed' : 'skipped',
      summary: complete
        ? 'bounded disposable-canary helper lifecycle completed; production isolation and heap coverage remain unverified'
        : `bounded disposable-canary helper lifecycle incomplete: ${close.unresolvedReason ?? phases.at(-1)?.reason ?? 'child-spawn-failed'}`,
      observed: {
        kind: 'disposable-heap-canary-helper-lifecycle/v1',
        coverageEligible: false,
        coverageReason:
          'internal lifecycle exercise only; production launcher isolation and native heap semantics are not certified',
        launchBoundary: {
          freshPrivateEvidenceRoot: true,
          stateAndNetworkIsolationVerified: false,
          declaration: 'new-state/no-bootstrap/no-signal/loopback-ephemeral-only',
          groupSignalRaceResidual: true,
        },
        observation,
        child: {
          ...identity,
          watchdogPid: watchdogIdentity.pid,
          watchdogReaped: close.reaped,
          nativeExitConfirmed: close.nativeExitConfirmed,
          watchdogExitCode: close.exitCode,
          watchdogSignal: close.signal,
          reapUnresolvedReason: close.unresolvedReason,
        },
        phases,
      },
    };
  } finally {
    for (const path of Object.values(fifoPaths)) {
      try {
        unlinkSync(path);
      } catch {
        // Cleanup must never replace a launch/capture failure or skip child reap.
      }
    }
  }
}
