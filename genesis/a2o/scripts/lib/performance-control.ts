/** Private conductor diagnostic admission; admission never proves capture completion. */

export type DiagnosticFamily = 'sqlTiming' | 'workflow';

/** Adapter to the installed Holochain admin client's existing request mechanism. */
export type DiagnosticRequester = (
  operation: 'capture_sql_timing' | 'capture_workflow_diagnostics',
  payload: { request: { schemaVersion: 1; nonce: string; requestedSeconds: number } },
  timeoutMs: number
) => Promise<unknown>;

export type HeapRequester = (
  operation: 'capture_heap',
  payload: { request: { schemaVersion: 1; nonce: string; phase: 'before' | 'after' } },
  timeoutMs: number
) => Promise<unknown>;

/** Observed by the owning launcher, never copied from the admin reply. */
export interface HeapExpectedProcess {
  processId: number;
  processStartTicks: number;
  bootId: string;
  executable: string;
}

export interface HeapNativeCompletionEvidence {
  requestStartedMonotonicMs: number;
  requestFinishedMonotonicMs: number;
  expectedProcess: HeapExpectedProcess;
  response: {
    outcome: 'completed';
    value: {
      schemaVersion: 1;
      nonce: string;
      phase: 'before' | 'after';
      producerId: string;
      generation: 1;
      artifactBasename: string;
      nativeProcess: HeapExpectedProcess & { schema: 1; clock: 'CLOCK_MONOTONIC' };
      startedMonotonicMs: number;
      finishedMonotonicMs: number;
      dumpTimeoutMs: number;
      deadlineMonotonicMs: number;
      nativeCompleted: true;
    };
  };
}

const HEAP_REFUSALS = [
  'disabled',
  'busy',
  'duplicateNonce',
  'generationBudget',
  'invalidRequest',
  'unsupported',
  'profilerUnavailable',
  'invalidSequence',
  'deadline',
  'byteLimit',
  'io',
  'incomplete',
] as const;

/**
 * Reuse the private admin transport. The owning canary still supplies the OS
 * deadline, pinned FIFO collector and parser: mallctl success is not completeness.
 * CLOCK_MONOTONIC must be read in the same host clock namespace as the child.
 */
export async function captureHeapPhase(
  requester: HeapRequester,
  request: { nonce: string; phase: 'before' | 'after'; timeoutMs: number; dumpTimeoutMs: number },
  expected: HeapExpectedProcess,
  readMonotonicMs: () => number,
  expectedProducerId?: string
) {
  validateRequest('workflow', request.nonce, 1);
  if (
    !['before', 'after'].includes(request.phase) ||
    !Number.isSafeInteger(request.timeoutMs) ||
    request.timeoutMs < 1 ||
    request.timeoutMs > 30_000 ||
    !Number.isSafeInteger(request.dumpTimeoutMs) ||
    request.dumpTimeoutMs < 1000 ||
    request.dumpTimeoutMs > 30_000 ||
    request.dumpTimeoutMs % 1000 !== 0 ||
    !Number.isSafeInteger(expected.processId) ||
    expected.processId < 1 ||
    !Number.isSafeInteger(expected.processStartTicks) ||
    expected.processStartTicks < 1 ||
    !expected.bootId ||
    !expected.executable.startsWith('/') ||
    (request.phase === 'after' && !expectedProducerId)
  )
    throw new Error('invalid heap capture binding');
  const started = readMonotonicMs();
  if (!Number.isFinite(started) || started < 0) throw new Error('invalid heap capture clock');
  let raw: unknown;
  try {
    raw = await requester(
      'capture_heap',
      {
        request: { schemaVersion: 1, nonce: request.nonce, phase: request.phase },
      },
      request.timeoutMs
    );
  } catch {
    throw new Error('heap capture transport failed; native outcome is unknown; do not retry');
  }
  const finished = readMonotonicMs();
  if (!Number.isFinite(finished) || finished < started)
    throw new Error('invalid heap capture clock');
  const response = record(raw);
  const value = record(response.value);
  if (response.outcome === 'refused') {
    const refusal = HEAP_REFUSALS.find(candidate => candidate === value.refusal);
    if (!refusal) throw new Error('unsupported heap capture refusal');
    // Never forward arbitrary server exception text or paths.
    return { completed: false as const, refusal, coverageEligible: false as const };
  }
  const native = record(value.nativeProcess);
  if (
    !exactKeys(response, ['outcome', 'value']) ||
    !exactKeys(value, [
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
    !exactKeys(native, [
      'schema',
      'processId',
      'processStartTicks',
      'bootId',
      'executable',
      'clock',
    ]) ||
    response.outcome !== 'completed' ||
    value.schemaVersion !== 1 ||
    value.nonce !== request.nonce ||
    value.phase !== request.phase ||
    value.generation !== 1 ||
    value.artifactBasename !== `${request.phase}.heap.fifo` ||
    value.nativeCompleted !== true ||
    typeof value.producerId !== 'string' ||
    !/^[A-Za-z0-9_-]{1,128}$/.test(value.producerId) ||
    (expectedProducerId !== undefined && value.producerId !== expectedProducerId) ||
    native.schema !== 1 ||
    native.clock !== 'CLOCK_MONOTONIC' ||
    native.processId !== expected.processId ||
    native.processStartTicks !== expected.processStartTicks ||
    native.bootId !== expected.bootId ||
    native.executable !== expected.executable ||
    !Number.isSafeInteger(value.startedMonotonicMs) ||
    !Number.isSafeInteger(value.finishedMonotonicMs) ||
    value.dumpTimeoutMs !== request.dumpTimeoutMs ||
    !Number.isSafeInteger(value.deadlineMonotonicMs) ||
    (value.deadlineMonotonicMs as number) - (value.startedMonotonicMs as number) !==
      request.dumpTimeoutMs ||
    (value.finishedMonotonicMs as number) > (value.deadlineMonotonicMs as number) ||
    (value.startedMonotonicMs as number) < Math.floor(started) ||
    (value.finishedMonotonicMs as number) < (value.startedMonotonicMs as number) ||
    (value.finishedMonotonicMs as number) > Math.floor(finished)
  )
    throw new Error('heap capture receipt does not match the owned process and request window');
  return {
    completed: true as const,
    producerId: value.producerId,
    phase: request.phase,
    nonce: request.nonce,
    startedMonotonicMs: value.startedMonotonicMs as number,
    finishedMonotonicMs: value.finishedMonotonicMs as number,
    deadlineMonotonicMs: value.deadlineMonotonicMs as number,
    dumpTimeoutMs: request.dumpTimeoutMs,
    coverageEligible: false as const,
    nativeEvidence: {
      requestStartedMonotonicMs: started,
      requestFinishedMonotonicMs: finished,
      expectedProcess: { ...expected },
      response: {
        outcome: 'completed' as const,
        value: {
          schemaVersion: 1 as const,
          nonce: request.nonce,
          phase: request.phase,
          producerId: value.producerId,
          generation: 1 as const,
          artifactBasename: value.artifactBasename,
          nativeProcess: {
            schema: 1 as const,
            processId: native.processId,
            processStartTicks: native.processStartTicks,
            bootId: native.bootId,
            executable: native.executable,
            clock: 'CLOCK_MONOTONIC' as const,
          },
          startedMonotonicMs: value.startedMonotonicMs as number,
          finishedMonotonicMs: value.finishedMonotonicMs as number,
          dumpTimeoutMs: request.dumpTimeoutMs,
          deadlineMonotonicMs: value.deadlineMonotonicMs as number,
          nativeCompleted: true as const,
        },
      },
    } satisfies HeapNativeCompletionEvidence,
  };
}

const OUTCOMES = [
  'admitted',
  'refusedDisabled',
  'refusedBusy',
  'refusedDuplicateNonce',
  'refusedGenerationBudget',
  'refusedInvalidRequest',
  'refusedTraceUnavailable',
  'refusedIo',
] as const;

/** Explicit local-only control arguments; no default runtime or capture nonce. */
export function parseDiagnosticControlArgs(argv: string[]) {
  const values = new Map<string, string>();
  const allowed = ['--admin-url', '--family', '--nonce', '--seconds'];
  for (let index = 0; index < argv.length; index += 2) {
    const flag = argv[index];
    const value = argv[index + 1];
    if (!allowed.includes(flag) || values.has(flag) || !value || value.startsWith('--'))
      throw new Error('arm requires exactly --admin-url --family --nonce --seconds');
    values.set(flag, value);
  }
  if (values.size !== allowed.length)
    throw new Error('arm requires exactly --admin-url --family --nonce --seconds');
  const url = new URL(values.get('--admin-url')!);
  if (
    url.protocol !== 'ws:' ||
    !['127.0.0.1', '[::1]'].includes(url.hostname) ||
    !url.port ||
    url.username ||
    url.password ||
    url.search ||
    url.hash ||
    url.pathname !== '/'
  )
    throw new Error('arm requires a numeric loopback ws:// address with an explicit port');
  const family = values.get('--family');
  if (family !== 'sqlTiming' && family !== 'workflow')
    throw new Error('arm --family must be sqlTiming or workflow');
  const nonce = values.get('--nonce')!;
  const secondsText = values.get('--seconds')!;
  if (!/^[1-9]\d{0,2}$/.test(secondsText)) throw new Error('invalid arm duration');
  const seconds = Number(secondsText);
  validateRequest(family, nonce, seconds);
  return { url, family: family as DiagnosticFamily, nonce, seconds };
}

function validateRequest(family: DiagnosticFamily, nonce: string, seconds: number): void {
  if (family !== 'sqlTiming' && family !== 'workflow')
    throw new Error('unsupported diagnostic family');
  if (typeof nonce !== 'string' || !/^[A-Za-z0-9_-]{1,64}$/.test(nonce))
    throw new Error('invalid diagnostic nonce');
  if (!Number.isSafeInteger(seconds) || seconds < 1 || seconds > 900)
    throw new Error('diagnostic duration must be 1..900 integer seconds');
}

function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new Error('invalid diagnostic admission response');
  return value as Record<string, unknown>;
}

function exactKeys(value: Record<string, unknown>, expected: readonly string[]): boolean {
  return Object.keys(value).sort().join(',') === [...expected].sort().join(',');
}

/**
 * Request one bounded generation through an already-connected private admin client.
 *
 * The caller owns socket selection/closure and must supply the client's bounded
 * request operation. A transport timeout does not cancel native work: its outcome
 * is unknown, and callers must not automatically retry with a different nonce.
 * This module never connects to, restarts, or selects a runtime on its own.
 */
export async function armDiagnostic(
  requester: DiagnosticRequester,
  family: DiagnosticFamily,
  nonce: string,
  seconds: number
) {
  validateRequest(family, nonce, seconds);

  let raw: unknown;
  try {
    raw = await requester(
      family === 'sqlTiming' ? 'capture_sql_timing' : 'capture_workflow_diagnostics',
      { request: { schemaVersion: 1, nonce, requestedSeconds: seconds } },
      5000
    );
  } catch {
    // Do not echo transport payloads, private paths or server exception strings.
    throw new Error(
      'diagnostic admission transport failed; native outcome is unknown; do not retry'
    );
  }
  const response = record(raw);
  if (response.schemaVersion !== 1 || response.family !== family || response.nonce !== nonce)
    throw new Error('diagnostic admission response does not match the request');
  const outcome = OUTCOMES.find(candidate => candidate === response.outcome);
  if (!outcome) throw new Error('unsupported diagnostic admission outcome');

  if (outcome !== 'admitted') {
    if (
      response.producerId !== null ||
      response.generation !== null ||
      response.outputBasename !== null ||
      response.window !== null
    )
      throw new Error('refused diagnostic admission carries contradictory capture evidence');
    return { admitted: false as const, outcome, family, nonce, coverageEligible: false as const };
  }

  const window = record(response.window);
  if (
    typeof response.producerId !== 'string' ||
    !/^[A-Za-z0-9_-]{1,128}$/.test(response.producerId) ||
    !Number.isSafeInteger(response.generation) ||
    (response.generation as number) < 1 ||
    (response.generation as number) > 16 ||
    typeof response.outputBasename !== 'string' ||
    !/^[A-Za-z0-9_-]{1,160}\.jsonl$/.test(response.outputBasename) ||
    window.state !== 'active' ||
    window.requestedSeconds !== seconds ||
    window.eventLimit !== 10_000
  )
    throw new Error('invalid admitted diagnostic generation');
  return {
    admitted: true as const,
    outcome,
    family,
    nonce,
    producerId: response.producerId,
    generation: response.generation as number,
    outputBasename: response.outputBasename,
    requestedSeconds: seconds,
    coverageEligible: false as const,
  };
}
