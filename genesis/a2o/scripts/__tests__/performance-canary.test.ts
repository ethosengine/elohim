import { strict as assert } from 'node:assert';
import { spawn } from 'node:child_process';
import {
  chmodSync,
  existsSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { performance } from 'node:perf_hooks';
import { describe, it } from 'node:test';

import {
  CANARY_STDERR_BASENAME,
  MAX_CANARY_DUMP_TIMEOUT_MS,
  MAX_CANARY_LIFETIME_MS,
  MAX_CANARY_OBSERVATION_MS,
  MAX_CANARY_STDERR_BYTES,
  runDisposableCanaryLifecycle,
  type CanaryLaunchContext,
  type CanaryLaunchPlan,
  type CanaryLifecycleDependencies,
  type CanaryParserResult,
  type CanaryParserStopEvidence,
  type CanaryParserTask,
  type CanaryTriggerContext,
  type NativeHeapCompletion,
} from '../lib/performance-canary.js';
import { MAX_HEAP_DUMP_BYTES } from '../lib/performance-heap.js';

const PARSER_STOPPED = {
  directWatchdogReaped: true,
  executingGroupMembers: 0,
} as const;
const EMPTY_SAMPLED_REASON = 'empty-sampled-profile';

function freshRoot(label: string): { parent: string; root: string } {
  const parent = mkdtempSync(join(tmpdir(), `performance-canary-${label}-`));
  return { parent, root: join(parent, 'new-private-root') };
}

function launchIdle(context: CanaryLaunchContext): CanaryLaunchPlan {
  return {
    executable: process.execPath,
    args: ['-e', 'setInterval(() => {}, 1000)'],
    cwd: context.isolatedRoot,
    env: process.env,
  };
}

function launchTermMaskedAndBlocked(context: CanaryLaunchContext): CanaryLaunchPlan {
  return {
    executable: process.execPath,
    args: [
      '-e',
      `require('node:fs').writeFileSync('term-masked.ready','ready',{mode:0o600});process.on('SIGTERM',()=>{});Atomics.wait(new Int32Array(new SharedArrayBuffer(4)),0,0,10000)`,
    ],
    cwd: context.isolatedRoot,
    env: process.env,
  };
}

async function waitForFile(path: string): Promise<void> {
  const deadlineAt = performance.now() + 500;
  while (!existsSync(path)) {
    if (performance.now() >= deadlineAt) throw new Error('helper readiness timeout');
    await new Promise<void>(resolve => setTimeout(resolve, 5));
  }
}

function completion(context: CanaryTriggerContext): NativeHeapCompletion {
  return {
    kind: 'holochain-heap-capture/v1',
    completed: true,
    phase: context.phase,
    nonce: context.nonce,
    identity: context.identity,
  };
}

async function writer(fifoPath: string, chunks: string[]): Promise<void> {
  const script = `
    const fs = require('node:fs');
    const fd = fs.openSync(process.argv[1], 'w');
    for (const encoded of JSON.parse(process.argv[2])) {
      const bytes = Buffer.from(encoded, 'base64');
      let offset = 0;
      while (offset < bytes.length) offset += fs.writeSync(fd, bytes, offset, bytes.length - offset);
    }
    fs.closeSync(fd);
  `;
  const child = spawn(
    process.execPath,
    [
      '-e',
      script,
      fifoPath,
      JSON.stringify(chunks.map(value => Buffer.from(value).toString('base64'))),
    ],
    { stdio: 'ignore' }
  );
  await new Promise<void>((resolve, reject) => {
    child.once('error', reject);
    child.once('close', code =>
      code === 0 ? resolve() : reject(new Error(`writer exit ${code}`))
    );
  });
}

function dependencies(options?: {
  launchPlan?: (context: CanaryLaunchContext) => CanaryLaunchPlan;
  trigger?: (context: CanaryTriggerContext) => Promise<NativeHeapCompletion>;
  validate?: (bytes: Buffer) => CanaryParserResult;
  parserTask?: (artifactPath: string) => CanaryParserTask;
}): CanaryLifecycleDependencies {
  return {
    launchPlan: options?.launchPlan ?? launchIdle,
    trigger:
      options?.trigger ??
      (async context => {
        await writer(context.fifoPath, [`${context.phase}-`, 'valid-profile']);
        return completion(context);
      }),
    validateArtifact: ({ artifactPath }) =>
      options?.parserTask?.(artifactPath) ?? {
        result: Promise.resolve(
          options?.validate?.(readFileSync(artifactPath)) ?? {
            valid: true,
            reason: 'valid',
            cleanup: PARSER_STOPPED,
          }
        ),
        abortAndConfirmStopped: async () => {
          await Promise.resolve();
          return PARSER_STOPPED;
        },
      },
  };
}

void describe('disposable performance canary lifecycle', () => {
  void it('captures short writes, EOF, native completion, parser validity, and reaps its child', async () => {
    const scratch = freshRoot('complete');
    try {
      const result = await runDisposableCanaryLifecycle(
        {
          isolatedRoot: scratch.root,
          nonce: 'pair_1',
          lifetimeMs: 2_500,
          observationMs: 20,
          dumpTimeoutMs: 500,
          maxBytes: 1024,
        },
        dependencies()
      );
      assert.equal(result.outcome, 'passed');
      assert.equal(result.checkId, 'runtime-performance');
      assert.equal(result.observed.coverageEligible, false);
      assert.equal(result.observed.launchBoundary.stateAndNetworkIsolationVerified, false);
      assert.equal(result.observed.observation.completed, true);
      assert.ok(result.observed.observation.actualMs! >= 20);
      assert.equal(result.observed.child.watchdogReaped, true);
      assert.deepEqual(
        result.observed.phases.map(phase => ({
          phase: phase.phase,
          bytes: phase.bytes,
          eof: phase.eofObserved,
          native: phase.nativeCompletionObserved,
          parser: phase.parser?.valid,
          complete: phase.boundedLifecycleComplete,
        })),
        [
          { phase: 'before', bytes: 20, eof: true, native: true, parser: true, complete: true },
          { phase: 'after', bytes: 19, eof: true, native: true, parser: true, complete: true },
        ]
      );
      assert.equal(statSync(scratch.root).mode & 0o777, 0o700);
      assert.equal(statSync(join(scratch.root, 'heap')).mode & 0o777, 0o700);
      assert.equal(statSync(join(scratch.root, 'heap', 'before.heap')).mode & 0o777, 0o600);
      assert.equal(existsSync(join(scratch.root, 'heap', 'before.heap.fifo')), false);
      assert.doesNotMatch(JSON.stringify(result), /new-private-root|valid-profile|pair_1/);
    } finally {
      rmSync(scratch.parent, { recursive: true, force: true });
    }
  });

  void it('finishes bounded preparation before starting either heap phase', async () => {
    const scratch = freshRoot('prepare');
    let prepared = false;
    try {
      const base = dependencies();
      const result = await runDisposableCanaryLifecycle(
        {
          isolatedRoot: scratch.root,
          nonce: 'prepared',
          lifetimeMs: 1_000,
          observationMs: 10,
          dumpTimeoutMs: 200,
        },
        {
          ...base,
          prepare: async context => {
            assert.equal(context.launch.isolatedRoot, scratch.root);
            assert.ok(context.identity.pid > 1);
            await Promise.resolve();
            prepared = true;
          },
          trigger: async context => {
            assert.equal(prepared, true);
            await writer(context.fifoPath, [context.phase]);
            return completion(context);
          },
        }
      );
      assert.equal(result.outcome, 'passed');
      assert.equal(result.observed.phases.length, 2);
    } finally {
      rmSync(scratch.parent, { recursive: true, force: true });
    }
  });

  void it('awaits preparation cancellation acknowledgement before returning', async () => {
    const scratch = freshRoot('prepare-cancel');
    let cancellationAcknowledged = false;
    try {
      const result = await runDisposableCanaryLifecycle(
        {
          isolatedRoot: scratch.root,
          nonce: 'prepare_cancel',
          lifetimeMs: 100,
          observationMs: 10,
          dumpTimeoutMs: 20,
        },
        {
          ...dependencies(),
          prepare: async ({ signal }) => {
            await new Promise<void>(resolve => signal.addEventListener('abort', () => resolve()));
            await new Promise<void>(resolve => setTimeout(resolve, 10));
            cancellationAcknowledged = true;
          },
        }
      );
      assert.equal(result.outcome, 'skipped');
      assert.equal(result.observed.phases[0].reason, 'deadline');
      assert.equal(cancellationAcknowledged, true);
      assert.equal(result.observed.child.watchdogReaped, true);
    } finally {
      rmSync(scratch.parent, { recursive: true, force: true });
    }
  });

  void it('persists exactly the cap plus one unpersisted sentinel, then kills and reaps', async () => {
    const scratch = freshRoot('overflow');
    const startedAt = performance.now();
    try {
      const result = await runDisposableCanaryLifecycle(
        {
          isolatedRoot: scratch.root,
          nonce: 'overflow',
          lifetimeMs: 2_100,
          observationMs: 10,
          dumpTimeoutMs: 1_000,
          maxBytes: 32,
        },
        dependencies({
          launchPlan: launchTermMaskedAndBlocked,
          trigger: async context => {
            await waitForFile(join(scratch.root, 'term-masked.ready'));
            await writer(context.fifoPath, ['x'.repeat(4096)]).catch(() => undefined);
            return completion(context);
          },
        })
      );
      assert.equal(result.outcome, 'skipped');
      assert.equal(result.observed.phases[0].reason, 'overflow');
      assert.equal(result.observed.phases[0].bytes, 32);
      assert.equal(result.observed.phases[0].overflowSentinelBytes, 1);
      assert.equal(readFileSync(join(scratch.root, 'heap', 'before.heap')).length, 32);
      assert.equal(result.observed.child.watchdogReaped, true);
      assert.equal(result.observed.child.watchdogSignal, 'SIGKILL');
      assert.ok(performance.now() - startedAt < 1_000, 'overflow must stop before full lifetime');
    } finally {
      rmSync(scratch.parent, { recursive: true, force: true });
    }
  });

  void it('distinguishes native completion from an unopened/empty FIFO', async () => {
    const scratch = freshRoot('empty');
    try {
      const result = await runDisposableCanaryLifecycle(
        {
          isolatedRoot: scratch.root,
          nonce: 'empty',
          lifetimeMs: 1_200,
          observationMs: 10,
          dumpTimeoutMs: 250,
        },
        dependencies({
          trigger: async context => {
            await Promise.resolve();
            return completion(context);
          },
        })
      );
      assert.equal(result.outcome, 'skipped');
      assert.equal(result.observed.phases[0].nativeCompletionObserved, true);
      assert.equal(result.observed.phases[0].eofObserved, true);
      assert.equal(result.observed.phases[0].reason, 'empty-fifo');
    } finally {
      rmSync(scratch.parent, { recursive: true, force: true });
    }
  });

  void it('enforces the dump deadline when the native writer never opens', async () => {
    const scratch = freshRoot('deadline');
    const started = Date.now();
    try {
      const result = await runDisposableCanaryLifecycle(
        {
          isolatedRoot: scratch.root,
          nonce: 'never_open',
          lifetimeMs: 1_000,
          observationMs: 10,
          dumpTimeoutMs: 100,
        },
        dependencies({
          launchPlan: launchTermMaskedAndBlocked,
          trigger: async () => {
            await waitForFile(join(scratch.root, 'term-masked.ready'));
            return new Promise<NativeHeapCompletion>(() => undefined);
          },
        })
      );
      assert.equal(result.outcome, 'skipped');
      assert.equal(result.observed.phases[0].reason, 'deadline');
      assert.equal(result.observed.child.watchdogReaped, true);
      assert.equal(result.observed.child.watchdogSignal, 'SIGKILL');
      assert.ok(Date.now() - started < 800, 'deadline and reap must remain bounded');
    } finally {
      rmSync(scratch.parent, { recursive: true, force: true });
    }
  });

  void it('paces quiet FIFO polling instead of spinning the event loop', async () => {
    const scratch = freshRoot('paced-idle');
    let idlePolls = 0;
    try {
      const result = await runDisposableCanaryLifecycle(
        {
          isolatedRoot: scratch.root,
          nonce: 'paced_idle',
          lifetimeMs: 1_000,
          observationMs: 10,
          dumpTimeoutMs: 60,
        },
        {
          ...dependencies({
            trigger: async () => new Promise<NativeHeapCompletion>(() => undefined),
          }),
          onFifoIdlePoll: () => {
            idlePolls += 1;
          },
        }
      );
      assert.equal(result.observed.phases[0].reason, 'deadline');
      assert.ok(idlePolls >= 2, 'the idle reader should remain live');
      assert.ok(idlePolls <= 10, `idle reader polled too often: ${idlePolls}`);
    } finally {
      rmSync(scratch.parent, { recursive: true, force: true });
    }
  });

  void it('refuses a zero-progress artifact write without blocking its kill timer', async () => {
    const scratch = freshRoot('zero-write');
    try {
      const result = await runDisposableCanaryLifecycle(
        {
          isolatedRoot: scratch.root,
          nonce: 'zero_write',
          lifetimeMs: 1_000,
          observationMs: 10,
          dumpTimeoutMs: 200,
        },
        { ...dependencies(), writeArtifact: () => 0 }
      );
      assert.equal(result.outcome, 'skipped');
      assert.equal(result.observed.phases[0].reason, 'fifo-error');
      assert.equal(result.observed.child.watchdogReaped, true);
    } finally {
      rmSync(scratch.parent, { recursive: true, force: true });
    }
  });

  void it('handles a native exec failure without an unhandled child error', async () => {
    const scratch = freshRoot('spawn-error');
    const notExecutable = join(scratch.parent, 'not-executable');
    writeFileSync(notExecutable, 'not an executable');
    chmodSync(notExecutable, 0o600);
    try {
      await assert.rejects(
        runDisposableCanaryLifecycle(
          {
            isolatedRoot: scratch.root,
            nonce: 'spawn_error',
            lifetimeMs: 1_000,
            observationMs: 10,
            dumpTimeoutMs: 200,
          },
          dependencies({
            launchPlan: context => ({
              executable: notExecutable,
              args: [],
              cwd: context.isolatedRoot,
              env: process.env,
            }),
          })
        ),
        /native canary identity was unavailable/
      );
      assert.equal(existsSync(join(scratch.root, 'heap', 'before.heap.fifo')), false);
      assert.equal(existsSync(join(scratch.root, 'heap', 'after.heap.fifo')), false);
      const stderrPath = join(scratch.root, CANARY_STDERR_BASENAME);
      assert.equal(existsSync(stderrPath), true);
      assert.equal(statSync(stderrPath).mode & 0o777, 0o600);
      assert.ok(statSync(stderrPath).size <= MAX_CANARY_STDERR_BYTES);
    } finally {
      rmSync(scratch.parent, { recursive: true, force: true });
    }
  });

  void it('retains bounded private native stderr without projecting it', async () => {
    const scratch = freshRoot('stderr-private');
    const privateMessage = 'private-native-startup-detail';
    try {
      const result = await runDisposableCanaryLifecycle(
        {
          isolatedRoot: scratch.root,
          nonce: 'stderr_private',
          lifetimeMs: 1_000,
          observationMs: 10,
          dumpTimeoutMs: 200,
        },
        dependencies({
          launchPlan: context => ({
            executable: process.execPath,
            args: [
              '-e',
              String.raw`process.stderr.write('${privateMessage}\n');setInterval(()=>{},1000)`,
            ],
            cwd: context.isolatedRoot,
            env: process.env,
          }),
        })
      );
      assert.equal(result.outcome, 'passed');
      const stderrPath = join(scratch.root, CANARY_STDERR_BASENAME);
      assert.equal(readFileSync(stderrPath, 'utf8'), `${privateMessage}\n`);
      assert.equal(statSync(stderrPath).mode & 0o777, 0o600);
      assert.doesNotMatch(JSON.stringify(result), new RegExp(privateMessage));
      assert.doesNotMatch(JSON.stringify(result), new RegExp(CANARY_STDERR_BASENAME));
    } finally {
      rmSync(scratch.parent, { recursive: true, force: true });
    }
  });

  void it('drains excess native stderr while retaining one capped truncation record', async () => {
    const scratch = freshRoot('stderr-cap');
    try {
      const base = dependencies({
        launchPlan: context => ({
          executable: process.execPath,
          args: [
            '-e',
            `const fs=require('node:fs');process.stderr.write(Buffer.alloc(${MAX_CANARY_STDERR_BYTES * 4},120),()=>{fs.writeFileSync('stderr.ready','ready',{mode:0o600});setInterval(()=>{},1000)})`,
          ],
          cwd: context.isolatedRoot,
          env: process.env,
        }),
      });
      const result = await runDisposableCanaryLifecycle(
        {
          isolatedRoot: scratch.root,
          nonce: 'stderr_cap',
          lifetimeMs: 2_000,
          observationMs: 10,
          dumpTimeoutMs: 300,
        },
        {
          ...base,
          prepare: async () => waitForFile(join(scratch.root, 'stderr.ready')),
        }
      );
      assert.equal(result.outcome, 'passed');
      const stderrPath = join(scratch.root, CANARY_STDERR_BASENAME);
      const retained = readFileSync(stderrPath);
      assert.equal(retained.length, MAX_CANARY_STDERR_BYTES);
      assert.equal(retained.subarray(0, 32).equals(Buffer.alloc(32, 120)), true);
      assert.equal(retained.toString('utf8').endsWith('\n[stderr truncated]\n'), true);
      assert.equal(statSync(stderrPath).mode & 0o777, 0o600);
    } finally {
      rmSync(scratch.parent, { recursive: true, force: true });
    }
  });

  void it('preserves an early child exit as incomplete and reaped', async () => {
    const scratch = freshRoot('early-exit');
    try {
      const result = await runDisposableCanaryLifecycle(
        {
          isolatedRoot: scratch.root,
          nonce: 'early_exit',
          lifetimeMs: 1_200,
          observationMs: 10,
          dumpTimeoutMs: 500,
        },
        dependencies({
          launchPlan: context => ({
            executable: process.execPath,
            args: ['-e', 'setTimeout(() => {}, 5)'],
            cwd: context.isolatedRoot,
            env: process.env,
          }),
          trigger: async () => new Promise<NativeHeapCompletion>(() => undefined),
        })
      );
      assert.equal(result.outcome, 'skipped');
      assert.equal(result.observed.phases[0].reason, 'child-exited');
      assert.equal(result.observed.child.watchdogReaped, true);
      assert.equal(result.observed.child.watchdogExitCode, 0);
    } finally {
      rmSync(scratch.parent, { recursive: true, force: true });
    }
  });

  void it('keeps a truncated parser result incomplete', async () => {
    const scratch = freshRoot('truncated');
    try {
      const result = await runDisposableCanaryLifecycle(
        {
          isolatedRoot: scratch.root,
          nonce: 'truncated',
          lifetimeMs: 1_000,
          observationMs: 10,
          dumpTimeoutMs: 250,
        },
        dependencies({
          validate: () => ({ valid: false, reason: 'truncated', cleanup: PARSER_STOPPED }),
        })
      );
      assert.equal(result.outcome, 'skipped');
      assert.equal(result.observed.phases[0].reason, 'parser-invalid');
      assert.deepEqual(result.observed.phases[0].parser, {
        valid: false,
        reason: 'truncated',
        cleanup: PARSER_STOPPED,
      });
      assert.equal(result.observed.phases[0].boundedLifecycleComplete, false);
    } finally {
      rmSync(scratch.parent, { recursive: true, force: true });
    }
  });

  void it('keeps empty sampled before/after profiles lifecycle-valid but coverage-ineligible', async () => {
    const scratch = freshRoot('empty-sampled');
    try {
      const result = await runDisposableCanaryLifecycle(
        {
          isolatedRoot: scratch.root,
          nonce: 'empty_sampled',
          lifetimeMs: 1_000,
          observationMs: 10,
          dumpTimeoutMs: 250,
        },
        dependencies({
          validate: () => ({
            valid: true,
            reason: EMPTY_SAMPLED_REASON,
            cleanup: PARSER_STOPPED,
          }),
        })
      );
      assert.equal(result.outcome, 'passed');
      assert.equal(result.observed.coverageEligible, false);
      assert.deepEqual(
        result.observed.phases.map(phase => phase.parser?.reason),
        [EMPTY_SAMPLED_REASON, EMPTY_SAMPLED_REASON]
      );
    } finally {
      rmSync(scratch.parent, { recursive: true, force: true });
    }
  });

  void it('reports unresolved parser reap acknowledgement', async () => {
    const scratch = freshRoot('parser-unbounded');
    try {
      const result = await runDisposableCanaryLifecycle(
        {
          isolatedRoot: scratch.root,
          nonce: 'parser_unbounded',
          lifetimeMs: 1_000,
          observationMs: 10,
          dumpTimeoutMs: 200,
        },
        dependencies({
          parserTask: () => ({
            result: new Promise<CanaryParserResult>(() => undefined),
            abortAndConfirmStopped: async () =>
              new Promise<CanaryParserStopEvidence>(() => undefined),
          }),
        })
      );
      assert.equal(result.outcome, 'skipped');
      assert.equal(result.observed.phases[0].reason, 'parser-reap-unresolved');
      assert.equal(result.observed.coverageEligible, false);
    } finally {
      rmSync(scratch.parent, { recursive: true, force: true });
    }
  });

  void it('keeps parser validation inside the same dump deadline', async () => {
    const scratch = freshRoot('parser-deadline');
    try {
      const result = await runDisposableCanaryLifecycle(
        {
          isolatedRoot: scratch.root,
          nonce: 'parser_deadline',
          lifetimeMs: 1_000,
          observationMs: 10,
          dumpTimeoutMs: 100,
        },
        {
          ...dependencies(),
          validateArtifact: () => ({
            result: new Promise<CanaryParserResult>(() => {
              // Deliberately never settles; the supervisor deadline owns it.
            }),
            abortAndConfirmStopped: async () => {
              await Promise.resolve();
              return PARSER_STOPPED;
            },
          }),
        }
      );
      assert.equal(result.outcome, 'skipped');
      assert.equal(result.observed.phases[0].reason, 'deadline');
      assert.equal(result.observed.child.watchdogReaped, true);
    } finally {
      rmSync(scratch.parent, { recursive: true, force: true });
    }
  });

  void it('uses a monotonic clock when the wall clock jumps', async () => {
    const scratch = freshRoot('monotonic');
    const originalDateNow = Date.now;
    let wallClock = 10_000;
    Date.now = () => {
      wallClock = wallClock === 10_000 ? -9_000_000_000 : 9_000_000_000;
      return wallClock;
    };
    try {
      const result = await runDisposableCanaryLifecycle(
        {
          isolatedRoot: scratch.root,
          nonce: 'monotonic',
          lifetimeMs: 1_000,
          observationMs: 20,
          dumpTimeoutMs: 200,
        },
        dependencies()
      );
      assert.equal(result.outcome, 'passed');
      assert.equal(result.observed.observation.completed, true);
      assert.ok(result.observed.observation.actualMs! >= 20);
      assert.ok(result.observed.observation.actualMs! < 500);
    } finally {
      Date.now = originalDateNow;
      rmSync(scratch.parent, { recursive: true, force: true });
    }
  });

  void it('re-arms an early observation timer until the monotonic duration has elapsed', async () => {
    const scratch = freshRoot('early-observation-wakeup');
    const clockReadings = [0, 0, 5, 19, 20];
    const requestedDelays: number[] = [];
    const activeTimers = new Set<NodeJS.Timeout>();
    try {
      const result = await runDisposableCanaryLifecycle(
        {
          isolatedRoot: scratch.root,
          nonce: 'early_observation_wakeup',
          lifetimeMs: 1_000,
          observationMs: 20,
          dumpTimeoutMs: 100,
        },
        {
          ...dependencies(),
          observationTimer: {
            now: () => clockReadings.shift() ?? 20,
            schedule: (callback, delayMs) => {
              requestedDelays.push(delayMs);
              const timer = setTimeout(() => {
                activeTimers.delete(timer);
                callback();
              }, 0);
              activeTimers.add(timer);
              return timer;
            },
            cancel: timer => {
              clearTimeout(timer);
              activeTimers.delete(timer);
            },
          },
        }
      );
      assert.equal(result.outcome, 'passed');
      assert.deepEqual(requestedDelays, [20, 15, 1]);
      assert.deepEqual(result.observed.observation, {
        requestedMs: 20,
        actualMs: 20,
        completed: true,
      });
      assert.equal(activeTimers.size, 0);
    } finally {
      for (const timer of activeTimers) clearTimeout(timer);
      rmSync(scratch.parent, { recursive: true, force: true });
    }
  });

  void it('lets the OS watchdog kill the exact native child while the parent event loop is blocked', async () => {
    const scratch = freshRoot('os-watchdog');
    let nativePid = 0;
    const startedAt = performance.now();
    try {
      const result = await runDisposableCanaryLifecycle(
        {
          isolatedRoot: scratch.root,
          nonce: 'os_watchdog',
          lifetimeMs: 120,
          observationMs: 20,
          dumpTimeoutMs: 40,
        },
        dependencies({
          trigger: async context => {
            await Promise.resolve();
            nativePid = context.identity.pid;
            Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 250);
            return completion(context);
          },
        })
      );
      const elapsedMs = performance.now() - startedAt;
      assert.equal(result.outcome, 'skipped');
      assert.equal(result.observed.child.watchdogReaped, true);
      assert.equal(result.observed.child.nativeExitConfirmed, true);
      assert.equal(result.observed.child.watchdogSignal, 'SIGKILL');
      assert.ok(nativePid > 1);
      assert.equal(existsSync(`/proc/${nativePid}`), false);
      assert.ok(elapsedMs >= 240, 'the injected parent block must have occurred');
      assert.ok(elapsedMs < 1_000, 'watchdog observation and reap must remain bounded');
    } finally {
      rmSync(scratch.parent, { recursive: true, force: true });
    }
  });

  void it('refuses reusable roots and bounds before spawning a child', async () => {
    const scratch = freshRoot('validation');
    let launches = 0;
    const deps = dependencies({
      launchPlan: context => {
        launches += 1;
        return launchIdle(context);
      },
    });
    try {
      await assert.rejects(
        runDisposableCanaryLifecycle(
          {
            isolatedRoot: scratch.root,
            nonce: 'bad',
            lifetimeMs: MAX_CANARY_LIFETIME_MS + 1,
            observationMs: 10,
          },
          deps
        ),
        /lifetimeMs/
      );
      await assert.rejects(
        runDisposableCanaryLifecycle(
          {
            isolatedRoot: scratch.root,
            nonce: 'bad',
            lifetimeMs: 100,
            observationMs: 10,
            dumpTimeoutMs: MAX_CANARY_DUMP_TIMEOUT_MS + 1,
          },
          deps
        ),
        /dumpTimeoutMs/
      );
      await assert.rejects(
        runDisposableCanaryLifecycle(
          {
            isolatedRoot: scratch.root,
            nonce: 'bad',
            lifetimeMs: MAX_CANARY_LIFETIME_MS,
            observationMs: MAX_CANARY_OBSERVATION_MS + 1,
            dumpTimeoutMs: 10,
          },
          deps
        ),
        /observationMs/
      );
      await assert.rejects(
        runDisposableCanaryLifecycle(
          {
            isolatedRoot: scratch.root,
            nonce: 'bad',
            lifetimeMs: 100,
            observationMs: 81,
            dumpTimeoutMs: 10,
          },
          deps
        ),
        /must fit lifetimeMs/
      );
      await assert.rejects(
        runDisposableCanaryLifecycle(
          {
            isolatedRoot: scratch.root,
            nonce: 'bad',
            lifetimeMs: 100,
            observationMs: 10,
            dumpTimeoutMs: 10,
            maxBytes: MAX_HEAP_DUMP_BYTES + 1,
          },
          deps
        ),
        /maxBytes/
      );
      assert.equal(launches, 0);
      await runDisposableCanaryLifecycle(
        {
          isolatedRoot: scratch.root,
          nonce: 'first',
          lifetimeMs: 1_000,
          observationMs: 10,
          dumpTimeoutMs: 100,
        },
        dependencies()
      );
      await assert.rejects(
        runDisposableCanaryLifecycle(
          {
            isolatedRoot: scratch.root,
            nonce: 'reuse',
            lifetimeMs: 200,
            observationMs: 10,
            dumpTimeoutMs: 50,
          },
          deps
        ),
        /EEXIST/
      );
    } finally {
      rmSync(scratch.parent, { recursive: true, force: true });
    }
  });
});
