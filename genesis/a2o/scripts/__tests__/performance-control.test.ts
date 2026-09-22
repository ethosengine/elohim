/* eslint-disable @typescript-eslint/no-floating-promises -- node:test owns returned promises. */
/* eslint-disable @typescript-eslint/require-await -- async transport doubles preserve rejection semantics. */
import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

// eslint-disable-next-line import/no-extraneous-dependencies -- exercise the installed admin wire codec.
import { decode, encode } from '@msgpack/msgpack';
// eslint-disable-next-line import/no-extraneous-dependencies -- exercise the installed Node transport.
import { WebSocketServer } from 'ws';

import {
  armDiagnostic,
  captureHeapPhase,
  parseDiagnosticControlArgs,
  type DiagnosticRequester,
} from '../lib/performance-control.js';
import { closeAdminBounded, connectAdminBounded } from '../resource-profile.js';

const PRIVATE_ERROR = '/private/secret';

describe('owned heap capture wire binding', () => {
  const expected = {
    processId: 42,
    processStartTicks: 100,
    bootId: 'boot',
    executable: '/bin/holochain',
  };
  const request = {
    nonce: 'heap_1',
    phase: 'before' as const,
    timeoutMs: 1000,
    dumpTimeoutMs: 1000,
  };
  const reply = (patch: Record<string, unknown> = {}) => ({
    outcome: 'completed',
    value: {
      schemaVersion: 1,
      nonce: request.nonce,
      phase: 'before',
      producerId: 'producer-1',
      generation: 1,
      artifactBasename: 'before.heap.fifo',
      nativeCompleted: true,
      nativeProcess: { ...expected, schema: 1, clock: 'CLOCK_MONOTONIC' },
      startedMonotonicMs: 10,
      finishedMonotonicMs: 11,
      dumpTimeoutMs: 1000,
      deadlineMonotonicMs: 1010,
      ...patch,
    },
  });
  const clock = () => {
    const readings = [10.8, 11.9];
    return () => readings.shift()!;
  };
  it('uses the installed client codec for heap completion and refusal', async () => {
    const server = new WebSocketServer({ host: '127.0.0.1', port: 0 });
    await new Promise<void>(resolve => server.once('listening', resolve));
    const observed: unknown[] = [];
    server.on('connection', socket =>
      socket.on('message', bytes => {
        const outer = decode(Buffer.from(bytes as Buffer)) as { id: number; data: Uint8Array };
        observed.push(decode(outer.data));
        const response =
          observed.length === 1
            ? reply()
            : { outcome: 'refused', value: { refusal: 'busy', message: 'busy' } };
        socket.send(
          encode({
            id: outer.id,
            type: 'response',
            data: encode({ type: 'heap_capture', value: response }),
          })
        );
      })
    );
    const address = server.address();
    assert.ok(address && typeof address !== 'string');
    let admin: Awaited<ReturnType<typeof connectAdminBounded>> | undefined;
    try {
      admin = await connectAdminBounded(new URL(`ws://127.0.0.1:${address.port}`));
      const requester = async (operation: 'capture_heap', payload: unknown, timeout: number) =>
        admin!._requester(operation)(payload, timeout);
      assert.equal((await captureHeapPhase(requester, request, expected, clock())).completed, true);
      assert.equal(
        (await captureHeapPhase(requester, request, expected, clock())).completed,
        false
      );
      for (const wire of observed)
        assert.deepEqual(wire, {
          type: 'capture_heap',
          value: { request: { schemaVersion: 1, nonce: request.nonce, phase: 'before' } },
        });
    } finally {
      if (admin) await closeAdminBounded(admin);
      for (const socket of server.clients) socket.terminate();
      await new Promise<void>(resolve => server.close(() => resolve()));
    }
  });
  it('binds the native receipt without claiming collector completeness', async () => {
    const result = await captureHeapPhase(
      async (operation, payload, timeout) => {
        assert.equal(operation, 'capture_heap');
        assert.deepEqual(payload, {
          request: { schemaVersion: 1, nonce: 'heap_1', phase: 'before' },
        });
        assert.equal(timeout, 1000);
        return reply();
      },
      request,
      expected,
      clock()
    );
    assert.equal(result.completed, true);
    assert.equal(result.coverageEligible, false);
    assert.deepEqual(result.nativeEvidence, {
      requestStartedMonotonicMs: 10.8,
      requestFinishedMonotonicMs: 11.9,
      expectedProcess: expected,
      response: reply(),
    });
  });
  it('refuses foreign process, nonce, phase, generation and clock witnesses', async () => {
    for (const patch of [
      { nonce: 'other' },
      { phase: 'after' },
      { generation: 2 },
      { artifactBasename: '/tmp/file' },
      { nativeCompleted: false },
      { startedMonotonicMs: 9 },
      { finishedMonotonicMs: 12 },
      { finishedMonotonicMs: 9 },
      { producerId: '' },
      { dumpTimeoutMs: undefined },
      { dumpTimeoutMs: 2000 },
      { deadlineMonotonicMs: undefined },
      { deadlineMonotonicMs: 1009 },
      { unexpected: true },
      ...['processId', 'processStartTicks', 'bootId', 'executable', 'clock', 'schema'].map(key => ({
        nativeProcess: { ...expected, schema: 1, clock: 'CLOCK_MONOTONIC', [key]: 'wrong' },
      })),
    ])
      await assert.rejects(captureHeapPhase(async () => reply(patch), request, expected, clock()));
    await assert.rejects(
      captureHeapPhase(async () => ({ ...reply(), unexpected: true }), request, expected, clock())
    );
  });
  it('requires and matches the before producer for the after phase', async () => {
    const after = { ...request, phase: 'after' as const };
    await assert.rejects(captureHeapPhase(async () => reply(), after, expected, clock()));
    await assert.rejects(
      captureHeapPhase(
        async () => reply({ phase: 'after', artifactBasename: 'after.heap.fifo' }),
        after,
        expected,
        clock(),
        'foreign'
      )
    );
    const result = await captureHeapPhase(
      async () => reply({ phase: 'after', artifactBasename: 'after.heap.fifo' }),
      after,
      expected,
      clock(),
      'producer-1'
    );
    assert.equal(result.completed, true);
  });
  it('redacts native refusal messages and ambiguous transport errors', async () => {
    const result = await captureHeapPhase(
      async () => ({ outcome: 'refused', value: { refusal: 'busy', message: PRIVATE_ERROR } }),
      request,
      expected,
      clock()
    );
    assert.deepEqual(result, { completed: false, refusal: 'busy', coverageEligible: false });
    await assert.rejects(
      captureHeapPhase(
        async () => {
          throw new Error(PRIVATE_ERROR);
        },
        request,
        expected,
        clock()
      ),
      /native outcome is unknown; do not retry/
    );
  });
});

function admission(overrides: Record<string, unknown> = {}) {
  return {
    schemaVersion: 1,
    family: 'sqlTiming',
    nonce: 'capture_1',
    outcome: 'admitted',
    producerId: 'process-1',
    generation: 1,
    outputBasename: 'sql-1-capture_1.jsonl',
    window: { requestedSeconds: 10, eventLimit: 10_000, state: 'active' },
    ...overrides,
  };
}

describe('private diagnostic admission', () => {
  it('uses the installed Holochain client wire without a second transport implementation', async () => {
    const server = new WebSocketServer({ host: '127.0.0.1', port: 0 });
    await new Promise<void>(resolve => server.once('listening', resolve));
    let observed: unknown;
    server.on('connection', socket =>
      socket.once('message', bytes => {
        const outer = decode(Buffer.from(bytes as Buffer)) as { id: number; data: Uint8Array };
        observed = decode(outer.data);
        socket.send(
          encode({
            id: outer.id,
            type: 'response',
            data: encode({ type: 'diagnostic_capture', value: admission() }),
          })
        );
      })
    );
    const address = server.address();
    assert.ok(address && typeof address !== 'string');
    const admin = await connectAdminBounded(new URL(`ws://127.0.0.1:${address.port}`));
    try {
      const receipt = await armDiagnostic(
        async (operation, payload, timeout) => admin._requester(operation)(payload, timeout),
        'sqlTiming',
        'capture_1',
        10
      );
      assert.equal(receipt.admitted, true);
      assert.deepEqual(observed, {
        type: 'capture_sql_timing',
        value: { request: { schemaVersion: 1, nonce: 'capture_1', requestedSeconds: 10 } },
      });
    } finally {
      await closeAdminBounded(admin);
      for (const socket of server.clients) socket.terminate();
      await new Promise<void>(resolve => server.close(() => resolve()));
    }
  });
  it('requires explicit loopback control arguments and refuses remote or ambiguous targets', () => {
    const args = [
      '--admin-url',
      'ws://127.0.0.1:4444',
      '--family',
      'workflow',
      '--nonce',
      'one',
      '--seconds',
      '30',
    ];
    assert.equal(parseDiagnosticControlArgs(args).seconds, 30);
    for (const url of [
      'ws://example.com:4444',
      'ws://localhost:4444',
      'ws://127.0.0.1',
      'ws://user@127.0.0.1:4444',
      'ws://127.0.0.1:4444/path',
    ])
      assert.throws(() => parseDiagnosticControlArgs([args[0], url, ...args.slice(2)]));
    assert.throws(() => parseDiagnosticControlArgs([...args, '--seconds', '40']));
    assert.throws(() => parseDiagnosticControlArgs(args.slice(0, -2)));
  });
  it('sends the bounded native request and keeps coverage ineligible', async () => {
    const requester: DiagnosticRequester = async (operation, payload, timeout) => {
      assert.equal(operation, 'capture_sql_timing');
      assert.deepEqual(payload, {
        request: { schemaVersion: 1, nonce: 'capture_1', requestedSeconds: 10 },
      });
      assert.equal(timeout, 5000);
      return admission();
    };
    const result = await armDiagnostic(requester, 'sqlTiming', 'capture_1', 10);
    assert.equal(result.admitted, true);
    assert.equal(result.coverageEligible, false);
  });

  it('selects only the workflow operation when explicitly requested', async () => {
    const result = await armDiagnostic(
      async operation => {
        assert.equal(operation, 'capture_workflow_diagnostics');
        return admission({ family: 'workflow' });
      },
      'workflow',
      'capture_1',
      10
    );
    assert.equal(result.family, 'workflow');
  });

  it('refuses unsafe input before making any request', async () => {
    let calls = 0;
    const requester = async () => {
      calls += 1;
      return admission();
    };
    for (const nonce of ['', '../capture', 'x'.repeat(65), 'with space'])
      await assert.rejects(armDiagnostic(requester, 'sqlTiming', nonce, 10));
    for (const seconds of [0, 901, 1.5, Number.NaN, Infinity])
      await assert.rejects(armDiagnostic(requester, 'sqlTiming', 'capture_1', seconds));
    assert.equal(calls, 0);
  });

  it('retains refusal and never interprets it as an empty successful capture', async () => {
    const result = await armDiagnostic(
      async () =>
        admission({
          outcome: 'refusedBusy',
          producerId: null,
          generation: null,
          outputBasename: null,
          window: null,
        }),
      'sqlTiming',
      'capture_1',
      10
    );
    assert.equal(result.admitted, false);
    assert.equal(result.outcome, 'refusedBusy');
  });

  it('rejects mismatched, unsafe, future and contradictory receipts', async () => {
    for (const overrides of [
      { schemaVersion: 2 },
      { family: 'workflow' },
      { nonce: 'other' },
      { outcome: 'newOutcome' },
      { producerId: '/private/path' },
      { outputBasename: '../capture.jsonl' },
      { generation: 17 },
      { window: { requestedSeconds: 11, eventLimit: 10_000, state: 'active' } },
      { outcome: 'refusedBusy' },
    ])
      await assert.rejects(
        armDiagnostic(async () => admission(overrides), 'sqlTiming', 'capture_1', 10)
      );
  });

  it('does not retry or leak a transport failure', async () => {
    let calls = 0;
    await assert.rejects(
      armDiagnostic(
        async () => {
          calls += 1;
          throw new Error('/private/secret');
        },
        'sqlTiming',
        'capture_1',
        10
      ),
      /native outcome is unknown; do not retry$/
    );
    assert.equal(calls, 1);
  });
});
