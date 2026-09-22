/* eslint-disable @typescript-eslint/no-floating-promises -- node:test owns returned promises. */
/* eslint-disable @typescript-eslint/promise-function-async -- fetch test doubles are already promises. */
/* eslint-disable sonarjs/no-duplicate-string -- CLI mode and limit literals are intentionally repeated. */
import { strict as assert } from 'node:assert';
import { chmodSync, fstatSync, mkdtempSync, openSync, writeFileSync } from 'node:fs';
import { createServer, type Socket } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, it } from 'node:test';

import { AdminWebsocket } from '@holochain/client';
// eslint-disable-next-line import/no-extraneous-dependencies -- exercise the client's installed wire codec.
import { decode, encode } from '@msgpack/msgpack';
// eslint-disable-next-line import/no-extraneous-dependencies -- exercise the client's installed Node transport.
import { WebSocketServer } from 'ws';

import {
  buildPerfRecordArgs,
  adminNetworkOptions,
  armSqlTimingSources,
  armWorkflowSources,
  buildPerfTraceArgs,
  captureMetricsEndpoint,
  closeAdminBounded,
  connectAdminBounded,
  classifyIoTraceExit,
  MAX_METRICS_BODY_BYTES,
  MAX_PERF_BYTES_PER_CONDUCTOR,
  listenerOwnedByPid,
  networkPorts,
  MAX_IO_BYTES_PER_CONDUCTOR,
  parseResourceProfileArgs,
  PerfChildGuard,
  processIdentityMatches,
  shouldPlanIoStop,
  validatePerf,
} from '../resource-profile.js';

import type { ResourceSnapshot } from '../../src/framework/fixtures/process-resources.js';
import type { SqlTimingSource } from '../lib/performance-binding.js';
import type { SqlTimingArmDependencies, WorkflowArmDependencies } from '../resource-profile.js';

const MESH = '/tmp/mesh';
const MESH_FLAG = '--mesh-dir';
const METRICS_URL = 'http://127.0.0.1/metrics';
const METRICS_NAME = 'test';
const PROFILE_OUTPUT = '/tmp/out';
const METRICS_FLAG = '--metrics-url';
const SQL_TIMING_FLAG = '--sql-timing';
const ARM_SQL_TIMING_FLAG = '--arm-sql-timing';
const ARM_WORKFLOW_FLAG = '--arm-workflow';
const PERF_PATH = '/usr/bin/perf';
const MAX_BYTES_FLAG = '--max-bytes';
const DWARF_STACK_FLAG = '--dwarf-stack-bytes';
const CALL_GRAPH_FLAG = '--call-graph';
const CPU_ARGS = [
  MESH_FLAG,
  MESH,
  '--seconds',
  '5',
  '--output',
  PROFILE_OUTPUT,
  '--mode',
  'cpu',
  '--perf',
  PERF_PATH,
  MAX_BYTES_FLAG,
  '1024',
];

describe('resource-profile arguments', () => {
  it('accepts repeatable private SQL timing witnesses only for bounded names and paths', () => {
    const options = parseResourceProfileArgs([
      ...CPU_ARGS,
      SQL_TIMING_FLAG,
      'one=/tmp/one.jsonl',
      SQL_TIMING_FLAG,
      'two=/tmp/two.jsonl',
    ]);
    assert.deepEqual(options.sqlTiming, [
      { name: 'one', sourcePath: '/tmp/one.jsonl' },
      { name: 'two', sourcePath: '/tmp/two.jsonl' },
    ]);
    assert.throws(
      () => parseResourceProfileArgs([...CPU_ARGS, SQL_TIMING_FLAG, 'one=relative.jsonl']),
      /absolute/
    );
    assert.throws(
      () =>
        parseResourceProfileArgs([
          ...CPU_ARGS,
          SQL_TIMING_FLAG,
          'one=/tmp/one.jsonl',
          SQL_TIMING_FLAG,
          'one=/tmp/other.jsonl',
        ]),
      /unique/
    );
  });

  it('accepts only explicit bounded SQL timing re-arm directories and excludes same-peer files', () => {
    const options = parseResourceProfileArgs([
      ...CPU_ARGS,
      ARM_SQL_TIMING_FLAG,
      'one=/tmp/private-one',
      ARM_SQL_TIMING_FLAG,
      'two=/tmp/private-two',
    ]);
    assert.deepEqual(options.armSqlTiming, [
      { name: 'one', directory: '/tmp/private-one' },
      { name: 'two', directory: '/tmp/private-two' },
    ]);
    assert.throws(
      () => parseResourceProfileArgs([...CPU_ARGS, ARM_SQL_TIMING_FLAG, 'one=relative']),
      /absolute canonical/
    );
    assert.throws(
      () =>
        parseResourceProfileArgs([
          ...CPU_ARGS,
          SQL_TIMING_FLAG,
          'one=/tmp/one.jsonl',
          ARM_SQL_TIMING_FLAG,
          'one=/tmp/private-one',
        ]),
      /mutually exclusive/
    );
    assert.throws(
      () =>
        parseResourceProfileArgs([
          ...CPU_ARGS,
          ...Array.from({ length: 9 }, (_, index) => [
            ARM_SQL_TIMING_FLAG,
            `p${index}=/tmp/private-${index}`,
          ]).flat(),
        ]),
      /at most 8/
    );
  });

  it('accepts repeatable bounded workflow re-arm directories with unique peer names', () => {
    const options = parseResourceProfileArgs([
      ...CPU_ARGS,
      ARM_WORKFLOW_FLAG,
      'one=/tmp/workflow-one',
      ARM_WORKFLOW_FLAG,
      'two=/tmp/workflow-two',
    ]);
    assert.deepEqual(options.armWorkflow, [
      { name: 'one', directory: '/tmp/workflow-one' },
      { name: 'two', directory: '/tmp/workflow-two' },
    ]);
    assert.throws(
      () => parseResourceProfileArgs([...CPU_ARGS, ARM_WORKFLOW_FLAG, 'one=relative']),
      /absolute canonical/
    );
    assert.throws(
      () =>
        parseResourceProfileArgs([
          ...CPU_ARGS,
          ARM_WORKFLOW_FLAG,
          'one=/tmp/a',
          ARM_WORKFLOW_FLAG,
          'one=/tmp/b',
        ]),
      /unique/
    );
  });

  it('accepts bounded I/O and combined windows and rejects incompatible bounds', () => {
    for (const mode of ['io', 'cpu-io']) {
      const options = parseResourceProfileArgs([
        ...CPU_ARGS.slice(0, 7),
        mode,
        ...CPU_ARGS.slice(8),
        '--max-events',
        '10000',
      ]);
      assert.equal(options.mode, mode);
      assert.equal(options.maxEvents, 10000);
    }
    assert.throws(
      () => parseResourceProfileArgs([...CPU_ARGS, '--max-events', '10000']),
      /only valid/
    );
    const combined = parseResourceProfileArgs([
      ...CPU_ARGS.slice(0, 7),
      'cpu-io',
      ...CPU_ARGS.slice(8),
      '--max-events',
      '4321',
    ]);
    assert.equal(combined.maxEvents, 4321);
    assert.throws(
      () =>
        parseResourceProfileArgs([
          ...CPU_ARGS.slice(0, 3),
          '61',
          ...CPU_ARGS.slice(4, 7),
          'io',
          ...CPU_ARGS.slice(8),
        ]),
      /1\.\.60/
    );
    assert.throws(
      () =>
        parseResourceProfileArgs([
          ...CPU_ARGS.slice(0, 7),
          'io',
          ...CPU_ARGS.slice(8, -1),
          String(MAX_IO_BYTES_PER_CONDUCTOR + 1),
        ]),
      /at most/
    );
  });

  it('constructs bounded syscall trace arguments without a shell', () => {
    const capability = {
      executable: PERF_PATH,
      version: 'perf version test',
      kind: 'successful-user-syscalls' as const,
      syscalls: ['read', 'write'],
      callGraph: 'dwarf,8192' as const,
      dwarfStackBytes: 8192 as const,
      maxEvents: 1234,
    };
    const args = buildPerfTraceArgs(capability, 42);
    assert.deepEqual(args, [
      'trace',
      '--max-events',
      '1234',
      '--call-graph',
      'dwarf,8192',
      '-e',
      'read,write',
      '-p',
      '42',
    ]);
  });

  it('distinguishes collector byte termination from an inferred early event-limit exit', () => {
    assert.deepEqual(classifyIoTraceExit(null, 'SIGINT', 'window', false), {
      exitCode: 0,
      error: null,
    });
    assert.equal(classifyIoTraceExit(0, null, 'byte-cap', true).error, null);
    assert.match(classifyIoTraceExit(0, null, 'none', false).error ?? '', /may have been reached/);
    assert.match(classifyIoTraceExit(null, 'SIGKILL', 'window', true).error ?? '', /exited/);
    assert.equal(shouldPlanIoStop(false), true);
    assert.equal(shouldPlanIoStop(true), false);
  });
  it('requires explicit bounded inputs', () => {
    const options = parseResourceProfileArgs([
      MESH_FLAG,
      MESH,
      '--seconds',
      '60',
      '--output',
      '/tmp/new-output',
      '--mode',
      'metrics',
    ]);
    assert.equal(options.seconds, 60);
    assert.equal(options.mode, 'metrics');
    assert.deepEqual(options.metricsUrls, []);
    assert.equal(options.cohort, null);
    assert.equal(options.dwarfStackBytes, 8192);
    assert.equal(options.callGraph, 'dwarf');
  });

  it('accepts explicit native network capture without changing the default', () => {
    const base = [
      MESH_FLAG,
      MESH,
      '--seconds',
      '1',
      '--output',
      PROFILE_OUTPUT,
      '--mode',
      'metrics',
    ];
    assert.equal(parseResourceProfileArgs(base).networkStats, false);
    assert.equal(parseResourceProfileArgs([...base, '--network-stats']).networkStats, true);
  });

  it('accepts only bounded CPU DWARF stack sizes', () => {
    for (const bytes of [8192, 16384, 32768]) {
      const options = parseResourceProfileArgs([...CPU_ARGS, DWARF_STACK_FLAG, String(bytes)]);
      assert.equal(options.dwarfStackBytes, bytes);
    }
    assert.throws(
      () => parseResourceProfileArgs([...CPU_ARGS, DWARF_STACK_FLAG, '65536']),
      /expects 8192, 16384, or 32768/
    );
  });

  it('accepts bounded named metrics URLs and an operator cohort', () => {
    const options = parseResourceProfileArgs([
      MESH_FLAG,
      MESH,
      '--seconds',
      '1',
      '--output',
      PROFILE_OUTPUT,
      '--mode',
      'metrics',
      METRICS_FLAG,
      'storage=http://127.0.0.1:8090/metrics',
      '--cohort',
      'idle-household-v1',
    ]);
    assert.deepEqual(options.metricsUrls, [
      {
        name: 'storage',
        url: 'http://127.0.0.1:8090/[redacted]',
        requestUrl: 'http://127.0.0.1:8090/metrics',
      },
    ]);
    assert.equal(options.cohort, 'idle-household-v1');
  });

  it('rejects unsafe or excessive metrics URLs', () => {
    const base = [
      MESH_FLAG,
      MESH,
      '--seconds',
      '1',
      '--output',
      PROFILE_OUTPUT,
      '--mode',
      'metrics',
    ];
    assert.throws(
      () => parseResourceProfileArgs([...base, METRICS_FLAG, 'x=http://user:secret@host/metrics']),
      /embedded credentials/
    );
    assert.throws(
      () => parseResourceProfileArgs([...base, METRICS_FLAG, 'x=file:///tmp/metrics']),
      /HTTP\(S\)/
    );
    assert.throws(
      () =>
        parseResourceProfileArgs([
          ...base,
          ...Array.from({ length: 9 }, (_, index) => [METRICS_FLAG, `m${index}=http://x/`]).flat(),
        ]),
      /at most 8/
    );
  });

  it('rejects an unbounded duration and implicit perf', () => {
    assert.throws(
      () =>
        parseResourceProfileArgs([
          MESH_FLAG,
          MESH,
          '--seconds',
          '601',
          '--output',
          PROFILE_OUTPUT,
          '--mode',
          'metrics',
        ]),
      /1\.\.600/
    );
    assert.throws(
      () =>
        parseResourceProfileArgs([
          MESH_FLAG,
          MESH,
          '--seconds',
          '5',
          '--output',
          PROFILE_OUTPUT,
          '--mode',
          'cpu',
        ]),
      /requires --perf/
    );
    assert.throws(
      () =>
        parseResourceProfileArgs([
          MESH_FLAG,
          MESH,
          '--seconds',
          '5',
          '--output',
          PROFILE_OUTPUT,
          '--mode',
          'cpu',
          '--perf',
          PERF_PATH,
        ]),
      /requires --max-bytes per conductor/
    );
    assert.throws(
      () =>
        parseResourceProfileArgs([
          MESH_FLAG,
          MESH,
          '--seconds',
          '5',
          '--output',
          PROFILE_OUTPUT,
          '--mode',
          'cpu',
          '--perf',
          PERF_PATH,
          MAX_BYTES_FLAG,
          String(MAX_PERF_BYTES_PER_CONDUCTOR + 1),
        ]),
      /at most 1073741824 per conductor/
    );
  });

  it('rejects CPU-only flags in metrics mode', () => {
    assert.throws(
      () =>
        parseResourceProfileArgs([
          MESH_FLAG,
          MESH,
          '--seconds',
          '5',
          '--output',
          PROFILE_OUTPUT,
          '--mode',
          'metrics',
          MAX_BYTES_FLAG,
          '1024',
        ]),
      /only valid with --mode cpu/
    );
    assert.throws(
      () =>
        parseResourceProfileArgs([
          MESH_FLAG,
          MESH,
          '--seconds',
          '5',
          '--output',
          PROFILE_OUTPUT,
          '--mode',
          'metrics',
          DWARF_STACK_FLAG,
          '16384',
        ]),
      /only valid with --mode cpu/
    );
  });
});

describe('SQL timing auto-rearm', () => {
  const sample = (peer: string, pid: number) => ({
    peer,
    pid,
    startTicks: pid * 10,
    executable: '/proc/test/conductor',
    configPath: `/mesh/${peer}/conductor-config.yaml`,
    cpuTicks: 1,
    rssKiB: 1,
    io: { rchar: 0, wchar: 0, readBytes: 0, writeBytes: 0, cancelledWriteBytes: 0 },
  });
  const snapshot = (bootId = 'boot-one', onePid = 101): ResourceSnapshot => ({
    atUnixMs: 1,
    monotonicMs: 1,
    bootId,
    clockTicksPerSecond: 100,
    samples: { one: sample('one', onePid), two: sample('two', 202) },
    issues: [],
  });
  const admitted = (nonce: string) =>
    ({
      admitted: true,
      outcome: 'admitted',
      family: 'sqlTiming',
      nonce,
      producerId: 'producer_1',
      generation: 1,
      outputBasename: 'sql-timing-1.jsonl',
      requestedSeconds: 7,
      coverageEligible: false,
    }) as const;
  const privateDirectory = (): string => {
    const directory = mkdtempSync(join(tmpdir(), 'sql-arm-'));
    chmodSync(directory, 0o700);
    return directory;
  };
  const dependencies = (
    events: string[],
    overrides: Partial<SqlTimingArmDependencies> = {}
  ): SqlTimingArmDependencies => ({
    capture: () => {
      events.push('capture');
      return snapshot();
    },
    listenerOwned: () => {
      events.push('listener');
      return true;
    },
    connect: url => {
      events.push(`connect:${url.href}`);
      return Promise.resolve({} as AdminWebsocket);
    },
    close: () => {
      events.push('close');
      return Promise.resolve();
    },
    admit: (_admin, nonce, seconds) => {
      events.push(`admit:${nonce}:${seconds}`);
      return Promise.resolve(admitted(nonce));
    },
    open: targets => {
      events.push(`open:${targets[0].sourcePath}`);
      return { sources: [] };
    },
    nonce: () => {
      events.push('nonce');
      return 'nonce_1';
    },
    ...overrides,
  });

  it('orders identity and listener checks around one-shot admission and opens only its basename', async () => {
    const events: string[] = [];
    const directory = privateDirectory();
    await armSqlTimingSources(
      [{ name: 'one', directory }],
      5,
      { one: '/config/one' },
      { one: 4444 },
      snapshot(),
      dependencies(events)
    );
    assert.deepEqual(events, [
      'capture',
      'listener',
      'connect:ws://127.0.0.1:4444/',
      'capture',
      'listener',
      'nonce',
      'admit:nonce_1:7',
      `open:${directory}/sql-timing-1.jsonl`,
      'capture',
      'listener',
      'close',
    ]);
  });

  it('refuses changed process identity before sending admission and closes the admin', async () => {
    const events: string[] = [];
    let captures = 0;
    const deps = dependencies(events, {
      capture: () => {
        events.push('capture');
        captures += 1;
        return captures === 1 ? snapshot() : snapshot('boot-one', 999);
      },
    });
    await assert.rejects(
      armSqlTimingSources(
        [{ name: 'one', directory: privateDirectory() }],
        5,
        { one: '/config/one' },
        { one: 4444 },
        snapshot(),
        deps
      ),
      /admitted windows will expire without cancellation/
    );
    assert.equal(
      events.some(event => event.startsWith('admit:')),
      false
    );
    assert.equal(events.includes('close'), true);
  });

  it('does not open a file for refused admission', async () => {
    const events: string[] = [];
    const deps = dependencies(events, {
      admit: (_admin, nonce, seconds) => {
        events.push(`admit:${nonce}:${seconds}`);
        return Promise.resolve({
          admitted: false as const,
          outcome: 'refusedBusy' as const,
          family: 'sqlTiming' as const,
          nonce,
          coverageEligible: false as const,
        });
      },
    });
    await assert.rejects(
      armSqlTimingSources(
        [{ name: 'one', directory: privateDirectory() }],
        5,
        { one: '/config/one' },
        { one: 4444 },
        snapshot(),
        deps
      ),
      /admitted windows will expire without cancellation/
    );
    assert.equal(
      events.some(event => event.startsWith('open:')),
      false
    );
    assert.equal(events.filter(event => event === 'close').length, 1);
  });

  it('closes every descriptor and admin after partial multi-peer admission failure', async () => {
    const events: string[] = [];
    const directoryOne = privateDirectory();
    const directoryTwo = privateDirectory();
    const sourcePath = join(directoryOne, 'sql-timing-1.jsonl');
    writeFileSync(sourcePath, '');
    let openedFd: number | undefined;
    const deps = dependencies(events, {
      connect: url => {
        const peer = url.port === '4444' ? 'one' : 'two';
        events.push(`connect:${peer}`);
        return Promise.resolve({ peer } as unknown as AdminWebsocket);
      },
      admit: (admin, nonce) => {
        const peer = (admin as unknown as { peer: string }).peer;
        events.push(`admit:${peer}`);
        return Promise.resolve(
          peer === 'one'
            ? admitted(nonce)
            : ({
                admitted: false,
                outcome: 'refusedBusy',
                family: 'sqlTiming',
                nonce,
                coverageEligible: false,
              } as const)
        );
      },
      open: targets => {
        openedFd = openSync(targets[0].sourcePath, 'r');
        const source: SqlTimingSource = {
          target: targets[0],
          fd: openedFd,
          opened: { dev: 'test', ino: 'test' },
          openedBytes: 0,
        };
        return { sources: [source] };
      },
    });
    await assert.rejects(
      armSqlTimingSources(
        [
          { name: 'one', directory: directoryOne },
          { name: 'two', directory: directoryTwo },
        ],
        5,
        { one: '/config/one', two: '/config/two' },
        { one: 4444, two: 5555 },
        snapshot(),
        deps
      ),
      /one|two/
    );
    assert.equal(events.filter(event => event === 'close').length, 2);
    assert.ok(openedFd !== undefined);
    assert.throws(() => fstatSync(openedFd!), /EBADF/);
  });

  it('closes an admitted descriptor when bounded admin close rejects', async () => {
    const events: string[] = [];
    const directory = privateDirectory();
    const sourcePath = join(directory, 'sql-timing-1.jsonl');
    writeFileSync(sourcePath, '');
    let openedFd: number | undefined;
    const deps = dependencies(events, {
      open: targets => {
        openedFd = openSync(targets[0].sourcePath, 'r');
        return {
          sources: [
            {
              target: targets[0],
              fd: openedFd,
              opened: { dev: 'test', ino: 'test' },
              openedBytes: 0,
            },
          ],
        };
      },
      close: () => Promise.reject(new Error('injected close failure')),
    });
    await assert.rejects(
      armSqlTimingSources(
        [{ name: 'one', directory }],
        5,
        { one: '/config/one' },
        { one: 4444 },
        snapshot(),
        deps
      ),
      /admitted windows will expire without cancellation/
    );
    assert.ok(openedFd !== undefined);
    assert.throws(() => fstatSync(openedFd!), /EBADF/);
  });
});

describe('workflow auto-rearm', () => {
  const sample = {
    peer: 'one',
    pid: 101,
    startTicks: 1010,
    executable: '/proc/test/conductor',
    configPath: '/mesh/one/conductor-config.yaml',
    cpuTicks: 1,
    rssKiB: 1,
    io: { rchar: 0, wchar: 0, readBytes: 0, writeBytes: 0, cancelledWriteBytes: 0 },
  };
  const snapshot = (): ResourceSnapshot => ({
    atUnixMs: 1,
    monotonicMs: 1,
    bootId: 'boot-one',
    clockTicksPerSecond: 100,
    samples: { one: sample },
    issues: [],
  });

  it('uses the workflow family and binds the exact returned basename and receipt tuple', async () => {
    const events: string[] = [];
    const directory = mkdtempSync(join(tmpdir(), 'workflow-arm-'));
    chmodSync(directory, 0o700);
    const dependencies: WorkflowArmDependencies = {
      capture: () => snapshot(),
      listenerOwned: () => true,
      connect: () => Promise.resolve({} as AdminWebsocket),
      close: () => Promise.resolve(),
      nonce: () => 'workflow_nonce',
      admit: (_admin, nonce, seconds) => {
        events.push(`admit:${nonce}:${seconds}`);
        return Promise.resolve({
          admitted: true,
          outcome: 'admitted',
          family: 'workflow',
          nonce,
          producerId: 'workflow-65-abcd',
          generation: 2,
          outputBasename: 'workflow-g02-workflow_nonce.jsonl',
          requestedSeconds: seconds,
          coverageEligible: false,
        });
      },
      open: targets => {
        const target = targets[0];
        events.push(
          `open:${target.sourcePath}:${target.admission.nonce}:${target.admission.generation}`
        );
        return { sources: [] };
      },
    };
    await armWorkflowSources(
      [{ name: 'one', directory }],
      5,
      { one: '/config/one' },
      { one: 4444 },
      snapshot(),
      dependencies
    );
    assert.deepEqual(events, [
      'admit:workflow_nonce:7',
      `open:${directory}/workflow-g02-workflow_nonce.jsonl:workflow_nonce:2`,
    ]);
  });

  it('does not retry or cancel an unknown workflow admission outcome', async () => {
    const directory = mkdtempSync(join(tmpdir(), 'workflow-arm-'));
    chmodSync(directory, 0o700);
    let admits = 0;
    let closes = 0;
    const dependencies: WorkflowArmDependencies = {
      capture: () => snapshot(),
      listenerOwned: () => true,
      connect: () => Promise.resolve({} as AdminWebsocket),
      close: () => {
        closes += 1;
        return Promise.resolve();
      },
      nonce: () => 'workflow_nonce',
      admit: () => {
        admits += 1;
        return Promise.reject(new Error('transport timeout'));
      },
      open: () => {
        throw new Error('must not open');
      },
    };
    await assert.rejects(
      armWorkflowSources(
        [{ name: 'one', directory }],
        5,
        { one: '/config/one' },
        { one: 4444 },
        snapshot(),
        dependencies
      ),
      /expire without cancellation/
    );
    assert.equal(admits, 1);
    assert.equal(closes, 1);
  });
});

describe('network admin transport bounds', () => {
  const closeAdmin = async (admin: AdminWebsocket): Promise<void> => {
    await Promise.race([
      admin.client.close().catch(() => undefined),
      new Promise<void>(resolve => setTimeout(resolve, 1000)),
    ]);
  };
  const server = async (handler: (socket: import('ws').WebSocket, bytes: Buffer) => void) => {
    const wss = new WebSocketServer({ host: '127.0.0.1', port: 0 });
    const origins: (string | undefined)[] = [];
    await new Promise<void>(resolve => wss.once('listening', resolve));
    wss.on('connection', (socket, request) => {
      origins.push(request.headers.origin);
      socket.on('message', bytes => handler(socket, Buffer.from(bytes as Buffer)));
    });
    const address = wss.address();
    if (typeof address === 'string' || address === null) throw new Error('test server has no port');
    const close = async (): Promise<void> => {
      for (const client of wss.clients) client.terminate();
      await new Promise<void>(resolve => wss.close(() => resolve()));
    };
    return { wss, close, origins, url: new URL(`ws://127.0.0.1:${address.port}`) };
  };

  it('passes a normal dump through the installed AdminWebsocket stack', async () => {
    const fixture = await server((socket, bytes) => {
      const request = decode(bytes) as { id: number };
      socket.send(
        encode({
          id: request.id,
          type: 'response',
          data: encode({
            type: 'dump_network_stats',
            value: {
              transport_stats: { backend: 'kitsune2', peer_urls: [], connections: [] },
              blocked_message_counts: {},
            },
          }),
        })
      );
    });
    const admin = await AdminWebsocket.connect(adminNetworkOptions(fixture.url));
    try {
      assert.equal(listenerOwnedByPid(process.pid, Number(fixture.url.port)), true);
      assert.equal(
        (await admin.dumpNetworkStats(undefined, 5000)).transport_stats.backend,
        'kitsune2'
      );
      assert.deepEqual(fixture.origins, ['elohim-runtime-performance']);
    } finally {
      await closeAdmin(admin);
      await fixture.close();
    }
  });

  it('requires fixture and YAML admin ports to agree', () => {
    const directory = mkdtempSync(join(tmpdir(), 'network-ports-'));
    const config = join(directory, 'one.yaml');
    writeFileSync(
      config,
      'admin_interfaces:\n  - driver:\n      type: websocket\n      port: 4444\n'
    );
    writeFileSync(
      join(directory, 'household-fixture.json'),
      JSON.stringify({ storagePeers: { one: { conductorAdminPort: 4444 } } })
    );
    assert.deepEqual(networkPorts(directory, { one: config }), { one: 4444 });
  });

  it('rejects an oversized websocket frame before msgpack decoding', async () => {
    const fixture = await server(socket => socket.send(Buffer.alloc(4 * 1024 * 1024 + 1)));
    const admin = await AdminWebsocket.connect(adminNetworkOptions(fixture.url));
    try {
      await assert.rejects(admin.dumpNetworkStats(undefined, 5000), /closed|pending|1009/i);
    } finally {
      await closeAdmin(admin);
      await fixture.close();
    }
  });

  it('times out and closes a hung admin request', async () => {
    const fixture = await server(() => undefined);
    const options = adminNetworkOptions(fixture.url);
    options!.defaultTimeout = 25;
    const admin = await AdminWebsocket.connect(options);
    try {
      await assert.rejects(admin.dumpNetworkStats(undefined, 25), /timed out/i);
    } finally {
      await closeAdmin(admin);
      await fixture.close();
    }
  });

  it('bounds a TCP handshake that never becomes a websocket', async () => {
    const sockets = new Set<Socket>();
    const tcp = createServer(socket => {
      sockets.add(socket);
      socket.once('close', () => sockets.delete(socket));
    });
    await new Promise<void>(resolve => tcp.listen(0, '127.0.0.1', resolve));
    const address = tcp.address();
    if (typeof address === 'string' || address === null) throw new Error('no TCP port');
    await assert.rejects(
      connectAdminBounded(new URL(`ws://127.0.0.1:${address.port}`), 25),
      /admin connect timeout/
    );
    for (const socket of sockets) socket.destroy();
    await new Promise<void>(resolve => tcp.close(() => resolve()));
  });

  it('force-terminates a stubborn admin close and clears the timer on a fast close', async () => {
    let terminated = 0;
    const stubborn = {
      client: {
        close: () => new Promise<void>(() => undefined),
        socket: { terminate: () => (terminated += 1) },
      },
    } as unknown as AdminWebsocket;
    await closeAdminBounded(stubborn, 10);
    assert.equal(terminated, 1);

    const fast = {
      client: {
        close: () => Promise.resolve(),
        socket: { terminate: () => (terminated += 1) },
      },
    } as unknown as AdminWebsocket;
    await closeAdminBounded(fast, 10);
    await new Promise(resolve => setTimeout(resolve, 25));
    assert.equal(terminated, 1);
  });
});

describe('perf record arguments', () => {
  it('accepts explicit frame-pointer capture without a misleading DWARF stack budget', () => {
    const options = parseResourceProfileArgs([...CPU_ARGS, CALL_GRAPH_FLAG, 'fp']);
    assert.equal(options.callGraph, 'fp');
    assert.throws(
      () =>
        parseResourceProfileArgs([
          MESH_FLAG,
          MESH,
          '--seconds',
          '5',
          '--output',
          PROFILE_OUTPUT,
          '--mode',
          'metrics',
          CALL_GRAPH_FLAG,
          'fp',
        ]),
      /only valid with --mode cpu/
    );
    assert.throws(
      () =>
        parseResourceProfileArgs([...CPU_ARGS, CALL_GRAPH_FLAG, 'fp', DWARF_STACK_FLAG, '8192']),
      /cannot be combined/
    );
    assert.throws(
      () => parseResourceProfileArgs([...CPU_ARGS, CALL_GRAPH_FLAG, 'lbr']),
      /expects dwarf or fp/
    );
    assert.throws(
      () => parseResourceProfileArgs([...CPU_ARGS, CALL_GRAPH_FLAG, 'fp', CALL_GRAPH_FLAG, 'fp']),
      /only once/
    );
    const args = buildPerfRecordArgs(
      {
        executable: PERF_PATH,
        version: 'perf version test',
        event: 'cpu-clock:u',
        frequencyHz: 49,
        callGraph: 'fp',
        dwarfStackBytes: null,
        maxSize: 'watchdog',
      },
      42,
      5,
      '/tmp/profile.perf.data',
      1024
    );
    assert.equal(args[args.indexOf(CALL_GRAPH_FLAG) + 1], 'fp');
  });

  it('passes the selected DWARF size as one no-shell call-graph value', () => {
    const args = buildPerfRecordArgs(
      {
        executable: PERF_PATH,
        version: 'perf version test',
        event: 'cpu-clock:u',
        frequencyHz: 49,
        callGraph: 'dwarf,32768',
        dwarfStackBytes: 32768,
        maxSize: 'native',
      },
      42,
      60,
      '/tmp/profile.perf.data',
      1024
    );
    assert.deepEqual(args.slice(args.indexOf(CALL_GRAPH_FLAG), args.indexOf(CALL_GRAPH_FLAG) + 2), [
      CALL_GRAPH_FLAG,
      'dwarf,32768',
    ]);
    assert.deepEqual(args.slice(-3), ['--', '/usr/bin/sleep', '60']);
  });
});

describe('metrics capture', () => {
  const target = { name: METRICS_NAME, url: `${METRICS_URL}/[redacted]`, requestUrl: METRICS_URL };

  it('preserves raw text and observes process start time', async () => {
    const capture = await captureMetricsEndpoint(target, () =>
      Promise.resolve(new Response('process_start_time_seconds 123\nforeign_metric 7\n'))
    );
    assert.equal(capture.status, 200);
    assert.equal(capture.error, null);
    assert.equal(capture.observedProcessStartTimeSeconds, 123);
    assert.match(capture.text, /foreign_metric 7/);
  });

  it('does not accept hex or trailing comments as process identity', async () => {
    for (const text of [
      'process_start_time_seconds 0x10\n',
      'process_start_time_seconds 123 # spoof\n',
    ]) {
      const capture = await captureMetricsEndpoint(target, () =>
        Promise.resolve(new Response(text))
      );
      assert.equal(capture.observedProcessStartTimeSeconds, null);
    }
  });

  it('reports HTTP and body-size collection failures', async () => {
    const failed = await captureMetricsEndpoint(target, () =>
      Promise.resolve(new Response('nope', { status: 503 }))
    );
    assert.equal(failed.status, 503);
    assert.match(failed.error ?? '', /HTTP 503/);

    const oversized = await captureMetricsEndpoint(target, () =>
      Promise.resolve(
        new Response('small', {
          headers: { 'content-length': String(MAX_METRICS_BODY_BYTES + 1) },
        })
      )
    );
    assert.match(oversized.error ?? '', /exceeds/);
  });

  it('redacts secret paths and queries from persisted target and errors', async () => {
    const options = parseResourceProfileArgs([
      MESH_FLAG,
      MESH,
      '--seconds',
      '1',
      '--output',
      PROFILE_OUTPUT,
      '--mode',
      'metrics',
      METRICS_FLAG,
      'secret=https://metrics.example/private/token-123?api_key=hunter2',
    ]);
    assert.equal(options.metricsUrls[0].url, 'https://metrics.example/[redacted]');
    const capture = await captureMetricsEndpoint(options.metricsUrls[0], () =>
      Promise.reject(new Error('failed https://metrics.example/private/token-123?api_key=hunter2'))
    );
    assert.doesNotMatch(
      JSON.stringify({ url: options.metricsUrls[0].url, capture }),
      /hunter2|token-123/
    );
  });

  it('cancels oversized streams and times out bounded requests', async () => {
    let cancelled = false;
    const stream = new ReadableStream<Uint8Array>({
      pull(controller) {
        controller.enqueue(new Uint8Array(MAX_METRICS_BODY_BYTES + 1));
      },
      cancel() {
        cancelled = true;
      },
    });
    const oversized = await captureMetricsEndpoint(target, () =>
      Promise.resolve(new Response(stream))
    );
    assert.match(oversized.error ?? '', /exceeds/);
    assert.equal(cancelled, true);

    const timedOut = await captureMetricsEndpoint(
      target,
      (_input, init) =>
        new Promise((_accept, reject) => {
          init?.signal?.addEventListener('abort', () => reject(init.signal?.reason), {
            once: true,
          });
        }),
      5
    );
    assert.match(timedOut.error ?? '', /request failed/);
  });

  it('cancels an acquired reader after body errors and body stalls', async () => {
    for (const shape of ['error', 'stall'] as const) {
      let cancelled = false;
      const reader = {
        read: () =>
          shape === 'error'
            ? Promise.reject(new Error('secret body failure'))
            : new Promise<ReadableStreamReadResult<Uint8Array>>(() => undefined),
        cancel: () => {
          cancelled = true;
          return Promise.resolve();
        },
      } as unknown as ReadableStreamDefaultReader<Uint8Array>;
      const response = {
        status: 200,
        ok: true,
        headers: new Headers(),
        body: { getReader: () => reader },
      } as unknown as Response;
      const capture = await captureMetricsEndpoint(target, () => Promise.resolve(response), 5);
      assert.match(capture.error ?? '', /request failed/);
      assert.equal(cancelled, true);
    }
  });
});

describe('perf capability', () => {
  it('requires an absolute executable named perf', () => {
    assert.throws(() => validatePerf('perf'), /absolute path/);
    const dir = mkdtempSync(join(tmpdir(), 'resource-profile-'));
    const fake = join(dir, 'not-perf');
    writeFileSync(fake, '#!/bin/sh\nexit 0\n');
    chmodSync(fake, 0o700);
    assert.throws(() => validatePerf(fake), /perf executable/);
  });

  it('interrupts tracked process groups and force-kills only survivors', () => {
    const signals: [number, NodeJS.Signals][] = [];
    let force: (() => void) | undefined;
    const guard = new PerfChildGuard(
      (pid, signal) => signals.push([pid, signal]),
      callback => {
        force = callback;
      }
    );
    let firstClosed: (() => void) | undefined;
    const first = {
      pid: 101,
      once: (_event: 'close', listener: () => void) => {
        firstClosed = listener;
      },
    };
    const second = { pid: 202, once: () => undefined };
    guard.track(first);
    guard.track(second);

    guard.terminateAll(25);
    assert.deepEqual(signals, [
      [101, 'SIGINT'],
      [202, 'SIGINT'],
    ]);
    firstClosed?.();
    force?.();
    assert.deepEqual(signals, [
      [101, 'SIGINT'],
      [202, 'SIGINT'],
      [202, 'SIGKILL'],
    ]);
  });

  it('rejects a reused pid before or after perf attachment', () => {
    const identity = {
      peer: 'matthew',
      pid: 42,
      startTicks: 100,
      executable: '/bin/holochain',
      configPath: '/mesh/matthew/conductor-config.yaml',
    };
    assert.equal(processIdentityMatches(identity, { ...identity }), true);
    assert.equal(processIdentityMatches(identity, { ...identity, startTicks: 101 }), false);
    assert.equal(processIdentityMatches(identity, undefined), false);
  });
});
