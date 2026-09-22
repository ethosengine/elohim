import { strict as assert } from 'node:assert';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readlinkSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, it } from 'node:test';

import {
  CANARY_PREPARATION_LOG_BASENAME,
  CANARY_REJECTED_WORKER_EVIDENCE_BASENAME,
  heapCanaryLauncherTestApi,
  parseHeapCanaryArgs,
  runIsolatedHeapCanary,
  type IsolatedHeapCanaryRequest,
  type IsolatedHeapCanaryResult,
} from '../lib/performance-canary-launcher.js';

const BEFORE_HEAP = 'before.heap';
const UNUSED_HAPP = 'unused.happ';

function sha256(bytes: string): string {
  return createHash('sha256').update(bytes).digest('hex');
}

function fixture(): { parent: string; request: IsolatedHeapCanaryRequest; argv: string[] } {
  const parent = mkdtempSync(join(tmpdir(), 'performance-canary-launcher-'));
  const make = (name: string, bytes: string, executable = false) => {
    const path = join(parent, name);
    writeFileSync(path, bytes, { mode: executable ? 0o700 : 0o600 });
    if (executable) chmodSync(path, 0o700);
    return { path, sha256: sha256(bytes) };
  };
  const request: IsolatedHeapCanaryRequest = {
    isolatedRoot: join(parent, 'fresh-root'),
    nonce: 'offline_1',
    lifetimeMs: 35_000,
    observationMs: 1_000,
    dumpTimeoutMs: 2_000,
    holochain: make('holochain', 'ELF helper', true),
    happ: make('canary.happ', 'bundle'),
    jeprof: make('jeprof', 'perl'),
  };
  return {
    parent,
    request,
    argv: [
      '--root',
      request.isolatedRoot,
      '--nonce',
      request.nonce,
      '--lifetime-ms',
      String(request.lifetimeMs),
      '--observation-ms',
      String(request.observationMs),
      '--dump-timeout-ms',
      String(request.dumpTimeoutMs),
      '--holochain',
      request.holochain.path,
      '--holochain-sha256',
      request.holochain.sha256,
      '--happ',
      request.happ.path,
      '--happ-sha256',
      request.happ.sha256,
      '--jeprof',
      request.jeprof.path,
      '--jeprof-sha256',
      request.jeprof.sha256,
    ],
  };
}

function redactedResult(): IsolatedHeapCanaryResult {
  return {
    checkId: 'runtime-performance',
    outcome: 'skipped',
    summary: 'offline canary/pipeline smoke',
    observed: {
      kind: 'disposable-heap-canary-helper-lifecycle/v1',
      coverageEligible: false,
      coverageReason: 'not scaling evidence',
      launchBoundary: {
        freshPrivateEvidenceRoot: true,
        stateAndNetworkIsolationVerified: false,
        declaration: 'new-state/no-bootstrap/no-signal/loopback-ephemeral-only',
        groupSignalRaceResidual: true,
      },
      observation: { requestedMs: 1_000, actualMs: null, completed: false },
      child: {
        pid: 42,
        startTicks: '100',
        executableDevice: '1',
        executableInode: '2',
        watchdogPid: 41,
        watchdogReaped: true,
        nativeExitConfirmed: true,
        watchdogExitCode: null,
        watchdogSignal: 'SIGKILL',
        reapUnresolvedReason: null,
      },
      phases: [
        {
          phase: 'before',
          artifact: BEFORE_HEAP,
          bytes: 0,
          overflowSentinelBytes: 0,
          eofObserved: false,
          nativeCompletionObserved: false,
          nativeIdentityMatched: false,
          parser: null,
          boundedLifecycleComplete: false,
          reason: 'deadline',
        },
      ],
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
    },
  };
}

function passedResult(): IsolatedHeapCanaryResult {
  const skipped = redactedResult();
  const phase = (name: 'before' | 'after') => ({
    phase: name,
    artifact: `${name}.heap` as const,
    bytes: 16,
    overflowSentinelBytes: 0 as const,
    eofObserved: true,
    nativeCompletionObserved: true,
    nativeIdentityMatched: true,
    parser: {
      valid: true,
      reason: 'valid' as const,
      cleanup: { directWatchdogReaped: true as const, executingGroupMembers: 0 as const },
    },
    boundedLifecycleComplete: true,
    reason: null,
  });
  return {
    ...skipped,
    outcome: 'passed',
    summary: 'bounded offline canary completed',
    observed: {
      ...skipped.observed,
      observation: { requestedMs: 1_000, actualMs: 1_001, completed: true },
      child: {
        ...skipped.observed.child,
        watchdogExitCode: 0,
        watchdogSignal: null,
      },
      phases: [phase('before'), phase('after')],
    },
  };
}

function controlVerification() {
  const row = (family: 'sqlTiming' | 'workflow', generation: 1 | 2) => ({
    family,
    generation,
    status: 'bound' as const,
    admission: 'exact' as const,
    nativeIdentity: 'exact' as const,
    interval: 'encloses' as const,
    terminalParser: 'valid' as const,
  });
  return {
    requested: true as const,
    outcome: 'passed' as const,
    requestedSeconds: 2 as const,
    sequence: [row('sqlTiming', 1), row('sqlTiming', 2), row('workflow', 1), row('workflow', 2)],
    coverageEligible: false as const,
    coverageReason:
      'quiet private-control lifecycle only; not workload or attribution coverage' as const,
  };
}

void describe('offline heap canary production launcher', () => {
  void it('parses one strict fixed-purpose request and rejects command/env expansion', () => {
    const item = fixture();
    try {
      assert.deepEqual(parseHeapCanaryArgs(item.argv), item.request);
      assert.deepEqual(parseHeapCanaryArgs([...item.argv, '--verify-controls', 'true']), {
        ...item.request,
        verifyControls: true,
      });
      assert.throws(
        () => parseHeapCanaryArgs([...item.argv, '--verify-controls', 'false']),
        /accepts only true/
      );
      assert.throws(() => parseHeapCanaryArgs([...item.argv, '--shell', '/bin/sh']), /exact/);
      assert.throws(
        () => parseHeapCanaryArgs(item.argv.map(value => (value === '2000' ? '2500' : value))),
        /whole seconds/
      );
      const relative = [...item.argv];
      relative[item.argv.indexOf('--root') + 1] = 'relative';
      assert.throws(() => parseHeapCanaryArgs(relative), /root/);
    } finally {
      rmSync(item.parent, { recursive: true, force: true });
    }
  });

  void it('derives one fixed installed loader under pnpm tsx and bare node --import tsx', async () => {
    const expected = heapCanaryLauncherTestApi.workerExecArgv();
    assert.equal(expected[0], '--import');
    assert.match(expected[1], /^file:\/\/.+\/tsx@[^/]+\/node_modules\/tsx\/dist\/loader\.mjs$/);
    const moduleUrl = new URL('../lib/performance-canary-launcher.ts', import.meta.url).href;
    const script = `const launcher=await import(${JSON.stringify(moduleUrl)});process.stdout.write(JSON.stringify(launcher.heapCanaryLauncherTestApi.workerExecArgv()))`;
    const child = spawn(
      process.execPath,
      ['--import', 'tsx', '--input-type=module', '--eval', script],
      { cwd: process.cwd(), stdio: ['ignore', 'pipe', 'ignore'] }
    );
    let output = '';
    child.stdout.on('data', chunk => {
      output += String(chunk);
    });
    const code = await new Promise<number | null>((resolve, reject) => {
      child.once('error', reject);
      child.once('close', resolve);
    });
    assert.equal(code, 0);
    assert.deepEqual(JSON.parse(output), expected);
  });

  void it('verifies every approved fingerprint before invoking the namespace worker', async () => {
    const item = fixture();
    let runs = 0;
    try {
      const result = await runIsolatedHeapCanary(item.request, async request => {
        await Promise.resolve();
        runs += 1;
        assert.equal(request.holochain.path, item.request.holochain.path);
        return redactedResult();
      });
      assert.equal(result.checkId, 'runtime-performance');
      assert.equal(result.observed.launcher.coverageEligible, false);
      assert.equal(runs, 1);
      await assert.rejects(
        runIsolatedHeapCanary(
          { ...item.request, happ: { ...item.request.happ, sha256: '0'.repeat(64) } },
          async () => {
            await Promise.resolve();
            runs += 1;
            return redactedResult();
          }
        ),
        /hash mismatch/
      );
      assert.equal(runs, 1);
      assert.doesNotMatch(JSON.stringify(result), /fresh-root|canary\.happ|offline_1/);
    } finally {
      rmSync(item.parent, { recursive: true, force: true });
    }
  });

  void it('accepts only the exact bounded redacted worker witness', () => {
    const item = fixture();
    try {
      const skipped = redactedResult();
      const passed = passedResult();
      assert.equal(heapCanaryLauncherTestApi.validWorkerResult(skipped, item.request), true);
      assert.equal(heapCanaryLauncherTestApi.validWorkerResult(passed, item.request), true);
      const controlRequest = { ...item.request, verifyControls: true as const };
      const controlsPassed = {
        ...passed,
        observed: { ...passed.observed, controlVerification: controlVerification() },
      };
      assert.equal(
        heapCanaryLauncherTestApi.validWorkerResult(controlsPassed, controlRequest),
        true
      );
      assert.equal(heapCanaryLauncherTestApi.validWorkerResult(passed, controlRequest), false);
      assert.equal(
        heapCanaryLauncherTestApi.validWorkerResult(controlsPassed, item.request),
        false
      );
      const controlsIncomplete = {
        ...skipped,
        observed: {
          ...skipped.observed,
          controlVerification: {
            ...controlVerification(),
            outcome: 'incomplete' as const,
            sequence: controlVerification().sequence.slice(0, 1),
          },
        },
      };
      assert.equal(
        heapCanaryLauncherTestApi.validWorkerResult(controlsIncomplete, controlRequest),
        true
      );
      for (const malformed of [
        {
          ...controlsPassed,
          observed: {
            ...controlsPassed.observed,
            controlVerification: {
              ...controlVerification(),
              sequence: [...controlVerification().sequence].reverse(),
            },
          },
        },
        {
          ...controlsPassed,
          observed: {
            ...controlsPassed.observed,
            controlVerification: {
              ...controlVerification(),
              sequence: controlVerification().sequence.map((row, index) =>
                index === 0 ? { ...row, admission: 'mismatch' } : row
              ),
            },
          },
        },
        {
          ...controlsPassed,
          observed: {
            ...controlsPassed.observed,
            controlVerification: { ...controlVerification(), outcome: 'incomplete' },
          },
        },
      ])
        assert.equal(heapCanaryLauncherTestApi.validWorkerResult(malformed, controlRequest), false);
      const emptySampled = {
        ...passed,
        observed: {
          ...passed.observed,
          phases: passed.observed.phases.map(phase => ({
            ...phase,
            parser: { ...phase.parser!, reason: 'empty-sampled-profile' as const },
          })),
        },
      };
      assert.equal(heapCanaryLauncherTestApi.validWorkerResult(emptySampled, item.request), true);
      assert.equal(
        heapCanaryLauncherTestApi.validWorkerResult(
          { ...skipped, summary: `leaked ${item.request.happ.path}` },
          item.request
        ),
        false
      );
      assert.equal(
        heapCanaryLauncherTestApi.validWorkerResult(
          {
            ...emptySampled,
            observed: {
              ...emptySampled.observed,
              phases: [
                {
                  ...emptySampled.observed.phases[0],
                  parser: { ...emptySampled.observed.phases[0].parser, valid: false },
                },
                emptySampled.observed.phases[1],
              ],
            },
          },
          item.request
        ),
        false
      );
      assert.equal(
        heapCanaryLauncherTestApi.validWorkerResult({ ...skipped, extra: true }, item.request),
        false
      );
      assert.equal(
        heapCanaryLauncherTestApi.validWorkerResult(
          { ...skipped, outcome: 'passed' },
          item.request
        ),
        false
      );
      assert.equal(
        heapCanaryLauncherTestApi.validWorkerResult(
          { ...passed, outcome: 'skipped' },
          item.request
        ),
        false
      );

      const invalidPassed = [
        { ...passed, observed: { ...passed.observed, phases: [] } },
        { ...passed, observed: { ...passed.observed, phases: passed.observed.phases.slice(0, 1) } },
        {
          ...passed,
          observed: { ...passed.observed, phases: [...passed.observed.phases].reverse() },
        },
        {
          ...passed,
          observed: {
            ...passed.observed,
            phases: [{ ...passed.observed.phases[0], parser: null }, passed.observed.phases[1]],
          },
        },
        {
          ...passed,
          observed: {
            ...passed.observed,
            phases: [
              {
                ...passed.observed.phases[0],
                parser: {
                  valid: false,
                  reason: 'malformed' as const,
                  cleanup: {
                    directWatchdogReaped: true as const,
                    executingGroupMembers: 0 as const,
                  },
                },
              },
              passed.observed.phases[1],
            ],
          },
        },
        {
          ...passed,
          observed: {
            ...passed.observed,
            phases: [
              { ...passed.observed.phases[0], eofObserved: false },
              passed.observed.phases[1],
            ],
          },
        },
        {
          ...passed,
          observed: {
            ...passed.observed,
            phases: [
              { ...passed.observed.phases[0], nativeIdentityMatched: false },
              passed.observed.phases[1],
            ],
          },
        },
        {
          ...passed,
          observed: {
            ...passed.observed,
            observation: { ...passed.observed.observation, completed: false },
          },
        },
        {
          ...passed,
          observed: {
            ...passed.observed,
            observation: { ...passed.observed.observation, actualMs: 999 },
          },
        },
        {
          ...passed,
          observed: {
            ...passed.observed,
            child: { ...passed.observed.child, watchdogReaped: false },
          },
        },
        {
          ...passed,
          observed: {
            ...passed.observed,
            launchBoundary: {
              ...passed.observed.launchBoundary,
              stateAndNetworkIsolationVerified: true,
            },
          },
        },
      ];
      for (const candidate of invalidPassed)
        assert.equal(heapCanaryLauncherTestApi.validWorkerResult(candidate, item.request), false);
    } finally {
      rmSync(item.parent, { recursive: true, force: true });
    }
  });

  void it('retains rejected worker stdout privately without weakening the witness predicate', () => {
    const item = fixture();
    const invalidJsonRoot = join(item.parent, 'invalid-json-root');
    const invalidWitnessRoot = join(item.parent, 'invalid-witness-root');
    mkdirSync(invalidJsonRoot, { mode: 0o700 });
    mkdirSync(invalidWitnessRoot, { mode: 0o700 });
    const inspect = (
      root: string,
      expectedReason: 'invalid-json' | 'invalid-witness',
      expectedStdout: string
    ) => {
      const path = join(root, CANARY_REJECTED_WORKER_EVIDENCE_BASENAME);
      const retained = readFileSync(path);
      const newline = retained.indexOf(0x0a);
      assert.ok(newline > 0);
      assert.deepEqual(JSON.parse(retained.subarray(0, newline).toString('utf8')), {
        kind: 'offline-disposable-heap-canary-rejected-worker-evidence/v1',
        reason: expectedReason,
        stdoutBytes: Buffer.byteLength(expectedStdout),
      });
      assert.equal(retained.subarray(newline + 1).toString('utf8'), expectedStdout);
      assert.equal(statSync(path).mode & 0o777, 0o600);
      assert.ok(retained.length <= 1024 * 1024 + 512);
    };
    try {
      const invalidJson = `{private:${JSON.stringify(item.request.happ.path)}}`;
      assert.throws(
        () =>
          heapCanaryLauncherTestApi.parseWorkerOutput(invalidJsonRoot, invalidJson, item.request),
        error =>
          error instanceof Error &&
          error.message === 'offline canary worker returned invalid evidence' &&
          !error.message.includes(item.request.happ.path)
      );
      inspect(invalidJsonRoot, 'invalid-json', invalidJson);

      const invalidWitness = JSON.stringify({ outcome: 'passed', private: item.request.nonce });
      assert.throws(
        () =>
          heapCanaryLauncherTestApi.parseWorkerOutput(
            invalidWitnessRoot,
            invalidWitness,
            item.request
          ),
        /offline canary worker returned invalid evidence$/
      );
      inspect(invalidWitnessRoot, 'invalid-witness', invalidWitness);

      assert.throws(() =>
        heapCanaryLauncherTestApi.writeRejectedWorkerEvidence(
          invalidWitnessRoot,
          'invalid-witness',
          'replacement'
        )
      );
      const overBoundRoot = join(item.parent, 'over-bound-root');
      mkdirSync(overBoundRoot, { mode: 0o700 });
      assert.throws(
        () =>
          heapCanaryLauncherTestApi.writeRejectedWorkerEvidence(
            overBoundRoot,
            'invalid-json',
            'x'.repeat(1024 * 1024 + 1)
          ),
        /exceeds its bound/
      );
      assert.equal(
        existsSync(join(overBoundRoot, CANARY_REJECTED_WORKER_EVIDENCE_BASENAME)),
        false
      );
    } finally {
      rmSync(item.parent, { recursive: true, force: true });
    }
  });

  void it('renders only fresh state, private admin and dead-loopback network endpoints', () => {
    const context = {
      isolatedRoot: '/private/canary',
      stateRoot: '/private/canary/state',
      heapRoot: '/private/canary/heap',
      network: {
        reuseExistingState: false,
        bootstrapUrl: null,
        signalUrl: null,
        listen: 'loopback-ephemeral-only',
      },
    } as const;
    const config = heapCanaryLauncherTestApi.renderConfig(context);
    assert.match(config, /data_root_path: "\/private\/canary\/state"/);
    assert.match(config, /port: 4444/);
    assert.match(config, /^ {6}allowed_origins: "elohim-runtime-performance"$/m);
    assert.equal(config.includes('allowed_origins:\n'), false);
    assert.match(config, /danger_test_keystore/);
    assert.equal((config.match(/127\.0\.0\.1:9/g) ?? []).length, 2);
    assert.match(config, /^ {2}bootstrap_url: "http:\/\/127\.0\.0\.1:9"$/m);
    assert.match(config, /^ {2}relay_url: "https:\/\/127\.0\.0\.1:9"$/m);
    assert.doesNotMatch(config, /elohim\.host|holostrap|local-dev|household/);

    const item = fixture();
    try {
      const env = heapCanaryLauncherTestApi.conductorEnvironment(context, item.request);
      const profiler =
        'prof:true,prof_active:false,prof_gdump:false,prof_final:false,lg_prof_interval:-1';
      assert.deepEqual(Object.keys(env).sort(), [
        'HOLOCHAIN_HEAP_CANARY_DIR',
        'HOLOCHAIN_HEAP_CANARY_DUMP_TIMEOUT_SECONDS',
        'HOLOCHAIN_HEAP_CANARY_WINDOW_SECONDS',
        'MALLOC_CONF',
        'PATH',
        '_RJEM_MALLOC_CONF',
      ]);
      assert.equal(env.MALLOC_CONF, profiler);
      assert.equal(env._RJEM_MALLOC_CONF, profiler);
      assert.equal(env.HOLOCHAIN_HEAP_CANARY_DUMP_TIMEOUT_SECONDS, '2');
      assert.equal(env.HOLOCHAIN_HEAP_CANARY_WINDOW_SECONDS, '35');

      const controls = heapCanaryLauncherTestApi.conductorEnvironment(context, {
        ...item.request,
        verifyControls: true,
      });
      assert.deepEqual(Object.keys(controls).sort(), [
        'HOLOCHAIN_HEAP_CANARY_DIR',
        'HOLOCHAIN_HEAP_CANARY_DUMP_TIMEOUT_SECONDS',
        'HOLOCHAIN_HEAP_CANARY_WINDOW_SECONDS',
        'HOLOCHAIN_SQL_DIAGNOSTICS_DIR',
        'HOLOCHAIN_SQL_DIAGNOSTICS_SECONDS',
        'HOLOCHAIN_WORKFLOW_DIAGNOSTICS_DIR',
        'HOLOCHAIN_WORKFLOW_DIAGNOSTICS_SECONDS',
        'MALLOC_CONF',
        'PATH',
        'RUST_LOG',
        '_RJEM_MALLOC_CONF',
      ]);
      assert.equal(controls.HOLOCHAIN_SQL_DIAGNOSTICS_DIR, '/private/canary/sql-diagnostics');
      assert.equal(controls.HOLOCHAIN_SQL_DIAGNOSTICS_SECONDS, '0');
      assert.equal(
        controls.HOLOCHAIN_WORKFLOW_DIAGNOSTICS_DIR,
        '/private/canary/workflow-diagnostics'
      );
      assert.equal(controls.HOLOCHAIN_WORKFLOW_DIAGNOSTICS_SECONDS, '0');
      assert.equal(controls.RUST_LOG, 'holochain::diagnostics::workflow=trace');
    } finally {
      rmSync(item.parent, { recursive: true, force: true });
    }
  });

  void it('projects only bounded allow-listed worker failure diagnostics', () => {
    const secret = '/private/canary/fresh-root offline_1 native payload';
    const unknown = heapCanaryLauncherTestApi.failureDiagnostic(
      'lifecycle',
      new Error(`unexpected ${secret}`)
    );
    assert.deepEqual(unknown, {
      kind: 'offline-disposable-heap-canary-failure/v1',
      stage: 'lifecycle',
      reason: 'unclassified-failure',
    });
    const encoded = JSON.stringify(unknown);
    assert.equal(encoded.includes(secret), false);
    assert.deepEqual(heapCanaryLauncherTestApi.parseWorkerFailure(`${encoded}\n`), unknown);

    const known = heapCanaryLauncherTestApi.failureDiagnostic(
      'lifecycle',
      new Error('watchdog exited before native identity was pinned')
    );
    assert.equal(known.reason, 'watchdog-native-child-missing');
    const projected = heapCanaryLauncherTestApi.publicWorkerFailure(JSON.stringify(known));
    assert.equal(projected.stage, 'lifecycle');
    assert.equal(projected.reason, 'watchdog-native-child-missing');
    assert.equal(
      projected.message,
      'offline canary worker failed (lifecycle: watchdog-native-child-missing)'
    );

    assert.equal(
      heapCanaryLauncherTestApi.parseWorkerFailure(JSON.stringify({ ...known, raw: secret })),
      null
    );
    const unavailable = heapCanaryLauncherTestApi.publicWorkerFailure(secret);
    assert.equal(unavailable.stage, 'worker-envelope');
    assert.equal(unavailable.reason, 'diagnostic-unavailable');
    assert.equal(unavailable.message.includes(secret), false);
  });

  void it('passes one shrinking preparation budget to both outer and native requests', async () => {
    const timeouts: { operation: string; timeoutMs: number | undefined }[] = [];
    const admin = {
      generateAgentPubKey: async (_request: void, timeoutMs?: number) => {
        timeouts.push({ operation: 'agent-key', timeoutMs });
        await Promise.resolve();
        return new Uint8Array(39);
      },
      installApp: async (_request: unknown, timeoutMs?: number) => {
        timeouts.push({ operation: 'install', timeoutMs });
        await Promise.resolve();
        return {};
      },
      enableApp: async (_request: unknown, timeoutMs?: number) => {
        timeouts.push({ operation: 'enable', timeoutMs });
        await Promise.resolve();
        return {};
      },
    } as unknown as Parameters<typeof heapCanaryLauncherTestApi.provisionPreparedAdmin>[0];
    const monotonic = [1_000, 2_000, 3_000];
    const failures: unknown[] = [];
    await heapCanaryLauncherTestApi.provisionPreparedAdmin(
      admin,
      '/private/canary/canary.happ',
      new AbortController().signal,
      30_000,
      diagnostic => failures.push(diagnostic),
      () => monotonic.shift()!
    );
    assert.deepEqual(timeouts, [
      { operation: 'agent-key', timeoutMs: 29_000 },
      { operation: 'install', timeoutMs: 28_000 },
      { operation: 'enable', timeoutMs: 27_000 },
    ]);
    assert.deepEqual(failures, []);
  });

  void it('runs the opted private controls once in strict sequential order without retry', async () => {
    const makeWitness = () => ({
      requested: true as const,
      outcome: 'incomplete' as const,
      requestedSeconds: 2 as const,
      sequence: [] as ReturnType<typeof controlVerification>['sequence'],
      coverageEligible: false as const,
      coverageReason:
        'quiet private-control lifecycle only; not workload or attribution coverage' as const,
    });
    const options = {
      admin: {} as Parameters<typeof heapCanaryLauncherTestApi.verifyPrivateControls>[0]['admin'],
      baseNonce: 'private-secret-nonce',
      root: '/private/canary',
      configPath: '/private/canary/conductor-config.yaml',
      signal: new AbortController().signal,
      deadlineAt: Number.MAX_SAFE_INTEGER,
      witness: makeWitness(),
    };
    const calls: { family: string; generation: number; nonce: string }[] = [];
    await heapCanaryLauncherTestApi.verifyPrivateControls(options, async request => {
      await Promise.resolve();
      calls.push({
        family: request.family,
        generation: request.generation,
        nonce: request.nonce,
      });
      const witness = controlVerification().sequence.find(
        row => row.family === request.family && row.generation === request.generation
      )!;
      return { witness, producerId: `${request.family}-producer` };
    });
    assert.deepEqual(
      calls.map(({ family, generation }) => ({ family, generation })),
      [
        { family: 'sqlTiming', generation: 1 },
        { family: 'sqlTiming', generation: 2 },
        { family: 'workflow', generation: 1 },
        { family: 'workflow', generation: 2 },
      ]
    );
    assert.equal(new Set(calls.map(call => call.nonce)).size, 4);
    assert.equal(
      calls.some(call => call.nonce.includes(options.baseNonce)),
      false
    );
    assert.equal(options.witness.outcome, 'passed');
    assert.deepEqual(options.witness.sequence, controlVerification().sequence);

    const incomplete = makeWitness();
    let attempts = 0;
    await assert.rejects(
      heapCanaryLauncherTestApi.verifyPrivateControls(
        { ...options, witness: incomplete },
        async request => {
          await Promise.resolve();
          attempts += 1;
          if (attempts === 2) throw new Error('terminal refusal');
          return {
            witness: controlVerification().sequence[0],
            producerId: `${request.family}-producer`,
          };
        }
      ),
      /terminal refusal/
    );
    assert.equal(attempts, 2);
    assert.equal(incomplete.outcome, 'incomplete');
    assert.equal(incomplete.sequence.length, 1);
  });

  void it('requires expiry and a stable retained hash before projecting a valid control terminal', () => {
    const sha256 = 'a'.repeat(64);
    const diagnostic = {
      name: 'sql-g1',
      sourcePath: '/private/canary/sql/source.jsonl',
      artifactPath: '/private/canary/control-evidence/sql-g1.sql-timing.jsonl',
      sourceSha256: sha256,
      artifactSha256: sha256,
      bytes: 10,
      status: 'bound' as const,
      nativeIdentity: 'exact' as const,
      interval: 'encloses' as const,
      issues: [],
      warnings: [],
      admission: 'exact' as const,
      nonce: 'private',
      producerId: 'sql-producer',
      generation: 1,
      outputBasename: 'sql-timing-g01-private.jsonl',
      requestedSeconds: 2,
    };
    const dependencies = {
      read: () => ({ bytes: Buffer.from('private'), sha256 }),
      summarizeSql: () => [{ closureReason: 'expiry' }],
      inspectWorkflow: () => {
        throw new Error('unexpected workflow parser');
      },
    };
    assert.equal(
      heapCanaryLauncherTestApi.requireBoundControl(diagnostic, 'sqlTiming', 1, dependencies)
        .terminalParser,
      'valid'
    );
    assert.throws(
      () =>
        heapCanaryLauncherTestApi.requireBoundControl(diagnostic, 'sqlTiming', 1, {
          ...dependencies,
          summarizeSql: () => [{ closureReason: 'event_limit' }],
        }),
      error => {
        assert.match(String(error), /SQL control artifact was invalid/);
        assert.equal(
          heapCanaryLauncherTestApi.preparationFailureReason(error, new AbortController().signal),
          'control-artifact-invalid'
        );
        return true;
      }
    );
    assert.throws(
      () =>
        heapCanaryLauncherTestApi.requireBoundControl(diagnostic, 'sqlTiming', 1, {
          ...dependencies,
          read: () => ({ bytes: Buffer.from('replacement'), sha256: 'b'.repeat(64) }),
        }),
      /changed before terminal validation/
    );
    assert.throws(
      () =>
        heapCanaryLauncherTestApi.requireBoundControl(
          {
            ...diagnostic,
            name: 'workflow-g1',
            sourcePath: '/private/canary/workflow/source.jsonl',
            artifactPath: '/private/canary/control-evidence/workflow-g1.workflow.jsonl',
            producerId: 'workflow-producer',
            outputBasename: 'workflow-g01-private.jsonl',
            coverageEligible: false as const,
          },
          'workflow',
          1,
          {
            ...dependencies,
            read: () => ({ bytes: Buffer.from('replacement'), sha256: 'b'.repeat(64) }),
          }
        ),
      /changed before terminal validation/
    );
  });

  void it('records a private bounded preparation stage and timeout/refusal class', async () => {
    const item = fixture();
    mkdirSync(item.request.isolatedRoot, { mode: 0o700 });
    const log = heapCanaryLauncherTestApi.preparationLog(item.request.isolatedRoot);
    const failures: { stage: string; reason: string }[] = [];
    try {
      const admin = {
        generateAgentPubKey: async (_request: void, _timeoutMs?: number) => {
          await Promise.resolve();
          throw new Error(`request timed out with private path ${item.request.happ.path}`);
        },
      } as unknown as Parameters<typeof heapCanaryLauncherTestApi.provisionPreparedAdmin>[0];
      await assert.rejects(
        heapCanaryLauncherTestApi.provisionPreparedAdmin(
          admin,
          item.request.happ.path,
          new AbortController().signal,
          30_000,
          diagnostic => {
            failures.push({ stage: diagnostic.stage, reason: diagnostic.reason });
            log.record(diagnostic);
          },
          () => 1_000
        )
      );
      assert.deepEqual(failures, [{ stage: 'agent-key', reason: 'timeout' }]);
      log.close();
      const path = join(item.request.isolatedRoot, CANARY_PREPARATION_LOG_BASENAME);
      const saved = readFileSync(path, 'utf8');
      assert.equal(statSync(path).mode & 0o777, 0o600);
      assert.equal(Buffer.byteLength(saved) < 4 * 1024, true);
      assert.match(saved, /"stage":"agent-key","reason":"timeout"/);
      assert.equal(saved.includes(item.request.happ.path), false);

      const refusalReasons: string[] = [];
      const refused = {
        generateAgentPubKey: async (_request: void, _timeoutMs?: number) => {
          await Promise.resolve();
          throw new Error('native request refused with private detail');
        },
      } as unknown as Parameters<typeof heapCanaryLauncherTestApi.provisionPreparedAdmin>[0];
      await assert.rejects(
        heapCanaryLauncherTestApi.provisionPreparedAdmin(
          refused,
          item.request.happ.path,
          new AbortController().signal,
          30_000,
          diagnostic => refusalReasons.push(diagnostic.reason),
          () => 1_000
        )
      );
      assert.deepEqual(refusalReasons, ['refused']);
    } finally {
      log.close();
      rmSync(item.parent, { recursive: true, force: true });
    }
  });

  void it('retains and strictly rebinds private native receipts to final heap bytes', () => {
    const parent = mkdtempSync(join(tmpdir(), 'performance-canary-native-evidence-'));
    const approvedBinarySha256 = 'a'.repeat(64);
    const process = {
      processId: 42,
      processStartTicks: 100,
      bootId: 'boot',
      executable: '/private/canary/holochain',
    };
    const nonce = 'native_pair_1';
    const dumpTimeoutMs = 1_000;
    const heapBytes = Buffer.from('heap_v2/524288\n  t*: 0: 0 [0: 0]\n');
    const artifactSha256 = createHash('sha256').update(heapBytes).digest('hex');
    const evidence = (phase: 'before' | 'after', producerId = 'producer-1') => ({
      kind: 'offline-disposable-heap-canary-native-evidence/v1' as const,
      schemaVersion: 1 as const,
      phase,
      approvedBinarySha256,
      artifact: { basename: `${phase}.heap`, bytes: heapBytes.length, sha256: artifactSha256 },
      control: {
        requestStartedMonotonicMs: 100,
        requestFinishedMonotonicMs: 102,
        expectedProcess: process,
        response: {
          outcome: 'completed' as const,
          value: {
            schemaVersion: 1 as const,
            nonce,
            phase,
            producerId,
            generation: 1 as const,
            artifactBasename: `${phase}.heap.fifo`,
            nativeProcess: { ...process, schema: 1 as const, clock: 'CLOCK_MONOTONIC' as const },
            startedMonotonicMs: 100,
            finishedMonotonicMs: 101,
            dumpTimeoutMs,
            deadlineMonotonicMs: 1_100,
            nativeCompleted: true as const,
          },
        },
      },
    });
    const expected = (phase: 'before' | 'after') => ({
      phase,
      nonce,
      process,
      approvedBinarySha256,
      artifactSha256,
      artifactBytes: heapBytes.length,
      dumpTimeoutMs,
    });
    try {
      for (const phase of ['before', 'after'] as const) {
        writeFileSync(join(parent, `${phase}.heap`), heapBytes, { mode: 0o600 });
        assert.equal(
          heapCanaryLauncherTestApi.hashFinalHeapArtifact(
            join(parent, `${phase}.heap`),
            heapBytes.length
          ),
          artifactSha256
        );
        heapCanaryLauncherTestApi.writePrivateNativeHeapEvidence(
          parent,
          evidence(phase),
          expected(phase)
        );
        const path = join(parent, heapCanaryLauncherTestApi.nativeEvidenceBasename(phase));
        assert.equal(statSync(path).mode & 0o777, 0o600);
        assert.ok(statSync(path).size <= 16 * 1024);
      }
      assert.equal(
        heapCanaryLauncherTestApi.readPrivateNativeHeapEvidencePair(parent, {
          nonce,
          process,
          approvedBinarySha256,
          dumpTimeoutMs,
          before: { artifactSha256, artifactBytes: heapBytes.length },
          after: { artifactSha256, artifactBytes: heapBytes.length },
        }).after.control.response.value.producerId,
        'producer-1'
      );

      const beforePath = join(parent, heapCanaryLauncherTestApi.nativeEvidenceBasename('before'));
      for (const mismatch of [
        { ...expected('before'), nonce: 'foreign' },
        { ...expected('before'), process: { ...process, processStartTicks: 101 } },
        { ...expected('before'), artifactSha256: 'b'.repeat(64) },
        { ...expected('before'), dumpTimeoutMs: 2_000 },
      ])
        assert.throws(() =>
          heapCanaryLauncherTestApi.readPrivateNativeHeapEvidence(beforePath, mismatch)
        );

      const original = JSON.parse(readFileSync(beforePath, 'utf8')) as Record<string, unknown>;
      const altered = structuredClone(original) as {
        control: { requestFinishedMonotonicMs: number };
      };
      altered.control.requestFinishedMonotonicMs = 100;
      writeFileSync(beforePath, `${JSON.stringify(altered)}\n`, { mode: 0o600 });
      assert.throws(() =>
        heapCanaryLauncherTestApi.readPrivateNativeHeapEvidence(beforePath, expected('before'))
      );
      writeFileSync(beforePath, `${JSON.stringify(original)}\n`, { mode: 0o600 });

      writeFileSync(beforePath, `${JSON.stringify(evidence('before', 'unsafe producer'))}\n`, {
        mode: 0o600,
      });
      assert.throws(() =>
        heapCanaryLauncherTestApi.readPrivateNativeHeapEvidence(beforePath, expected('before'))
      );
      writeFileSync(beforePath, `${JSON.stringify(original)}\n`, { mode: 0o600 });

      const afterPath = join(parent, heapCanaryLauncherTestApi.nativeEvidenceBasename('after'));
      writeFileSync(afterPath, `${JSON.stringify(evidence('after', 'producer-2'))}\n`, {
        mode: 0o600,
      });
      assert.throws(() =>
        heapCanaryLauncherTestApi.readPrivateNativeHeapEvidencePair(parent, {
          nonce,
          process,
          approvedBinarySha256,
          dumpTimeoutMs,
          before: { artifactSha256, artifactBytes: heapBytes.length },
          after: { artifactSha256, artifactBytes: heapBytes.length },
        })
      );
    } finally {
      rmSync(parent, { recursive: true, force: true });
    }
  });

  void it('executes, installs and parses only fingerprinted snapshots under the fresh root', () => {
    const item = fixture();
    try {
      mkdirSync(item.request.isolatedRoot, { mode: 0o700 });
      const pinned = heapCanaryLauncherTestApi.pinArtifacts(
        {
          isolatedRoot: item.request.isolatedRoot,
          stateRoot: join(item.request.isolatedRoot, 'state'),
          heapRoot: join(item.request.isolatedRoot, 'heap'),
          network: {
            reuseExistingState: false,
            bootstrapUrl: null,
            signalUrl: null,
            listen: 'loopback-ephemeral-only',
          },
        },
        item.request
      );
      writeFileSync(item.request.holochain.path, 'replacement');
      writeFileSync(item.request.happ.path, 'replacement');
      writeFileSync(item.request.jeprof.path, 'replacement');
      assert.equal(readFileSync(pinned.holochain, 'utf8'), 'ELF helper');
      assert.equal(readFileSync(pinned.happ, 'utf8'), 'bundle');
      assert.equal(readFileSync(pinned.jeprof, 'utf8'), 'perl');
      assert.equal(statSync(pinned.holochain).mode & 0o777, 0o500);
      assert.ok(
        [pinned.holochain, pinned.happ, pinned.jeprof].every(path =>
          path.startsWith(item.request.isolatedRoot)
        )
      );
    } finally {
      rmSync(item.parent, { recursive: true, force: true });
    }
  });

  void it('refuses parseable jeprof output from a nonzero tool exit', async () => {
    const parent = mkdtempSync(join(tmpdir(), 'performance-canary-parser-exit-'));
    try {
      const jeprof = join(parent, 'jeprof');
      const holochain = join(parent, 'holochain');
      const heap = join(parent, BEFORE_HEAP);
      writeFileSync(
        jeprof,
        String.raw`print "Total: 1 B\n1 100.0% 100.0% 1 100.0% canary::row\n"; exit 2;`
      );
      writeFileSync(holochain, 'binary');
      writeFileSync(heap, 'profile');
      const task = heapCanaryLauncherTestApi.parserTask({
        artifactPath: heap,
        artifacts: { jeprof, holochain, happ: join(parent, UNUSED_HAPP) },
        signal: new AbortController().signal,
      });
      const result = await task.result;
      assert.equal(result.valid, false);
      assert.equal(result.reason, 'malformed');
      assert.equal(result.cleanup.directWatchdogReaped, true);
    } finally {
      rmSync(parent, { recursive: true, force: true });
    }
  });

  void it('accepts empty sampled jeprof output only with a heap_v2 zero aggregate', async () => {
    const parent = mkdtempSync(join(tmpdir(), 'performance-canary-empty-sampled-'));
    try {
      const jeprof = join(parent, 'jeprof');
      const holochain = join(parent, 'holochain');
      const heap = join(parent, BEFORE_HEAP);
      writeFileSync(jeprof, String.raw`print STDERR "Using local file\n"; exit 0;`);
      writeFileSync(holochain, 'binary');
      writeFileSync(heap, 'heap_v2/524288\n  t*: 0: 0 [0: 0]\n  t0: 0: 0 [0: 0]\n');
      const valid = heapCanaryLauncherTestApi.parserTask({
        artifactPath: heap,
        artifacts: { jeprof, holochain, happ: join(parent, UNUSED_HAPP) },
        signal: new AbortController().signal,
      });
      assert.deepEqual(await valid.result, {
        valid: true,
        reason: 'empty-sampled-profile',
        cleanup: { directWatchdogReaped: true, executingGroupMembers: 0 },
      });

      writeFileSync(heap, 'heap_v2/524288\n  t*: 1: 1 [0: 0]\n');
      const nonzero = heapCanaryLauncherTestApi.parserTask({
        artifactPath: heap,
        artifacts: { jeprof, holochain, happ: join(parent, UNUSED_HAPP) },
        signal: new AbortController().signal,
      });
      const nonzeroResult = await nonzero.result;
      assert.equal(nonzeroResult.valid, false);
      assert.equal(nonzeroResult.reason, 'malformed');

      writeFileSync(heap, 'not-a-native-heap-profile\n');
      const invalid = heapCanaryLauncherTestApi.parserTask({
        artifactPath: heap,
        artifacts: { jeprof, holochain, happ: join(parent, UNUSED_HAPP) },
        signal: new AbortController().signal,
      });
      const invalidResult = await invalid.result;
      assert.equal(invalidResult.valid, false);
      assert.equal(invalidResult.reason, 'malformed');
    } finally {
      rmSync(parent, { recursive: true, force: true });
    }
  });

  void it('can establish a distinct network namespace and bring up only its loopback helper', async () => {
    const parentNet = readlinkSync('/proc/self/ns/net');
    const parentPid = readlinkSync('/proc/self/ns/pid');
    const script = `
      const {spawnSync}=require('node:child_process');
      const fs=require('node:fs');
      const up=spawnSync('/usr/sbin/ip',['link','set','lo','up']);
      if(up.status!==0) process.exit(2);
      process.stdout.write(JSON.stringify({net:fs.readlinkSync('/proc/self/ns/net'),pid:fs.readlinkSync('/proc/self/ns/pid'),processId:process.pid}));
    `;
    const child = spawn(
      '/usr/bin/unshare',
      [
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
        '-e',
        script,
      ],
      { stdio: ['ignore', 'pipe', 'ignore'] }
    );
    let output = '';
    child.stdout.on('data', chunk => {
      output += String(chunk);
    });
    const code = await new Promise<number | null>((resolve, reject) => {
      child.once('error', reject);
      child.once('close', resolve);
    });
    assert.equal(code, 0);
    const namespaces = JSON.parse(output) as { net: string; pid: string; processId: number };
    assert.notEqual(namespaces.net, parentNet);
    assert.notEqual(namespaces.pid, parentPid);
    assert.equal(namespaces.processId, 1);
  });

  void it('kernel PID-namespace teardown stops detached worker descendants', async () => {
    const parent = mkdtempSync(join(tmpdir(), 'performance-canary-pidns-'));
    const marker = join(parent, 'detached-survived');
    const helper = `setTimeout(()=>require('node:fs').writeFileSync(${JSON.stringify(marker)},'survived'),500);setInterval(()=>{},1000)`;
    const init = `
      const {spawn}=require('node:child_process');
      const child=spawn(process.execPath,['-e',${JSON.stringify(helper)}],{detached:true,stdio:'ignore'});
      child.unref();
      process.stdout.write('ready');
      setInterval(()=>{},1000);
    `;
    const child = spawn(
      '/usr/bin/unshare',
      [
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
        '-e',
        init,
      ],
      { stdio: ['ignore', 'pipe', 'ignore'] }
    );
    try {
      await new Promise<void>((resolve, reject) => {
        child.once('error', reject);
        child.stdout.once('data', () => resolve());
      });
      assert.equal(child.kill('SIGKILL'), true);
      await new Promise<void>(resolve => child.once('close', () => resolve()));
      await new Promise<void>(resolve => setTimeout(resolve, 700));
      assert.equal(existsSync(marker), false);
    } finally {
      if (child.exitCode === null && child.signalCode === null) child.kill('SIGKILL');
      rmSync(parent, { recursive: true, force: true });
    }
  });
});
