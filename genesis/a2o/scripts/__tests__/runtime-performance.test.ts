/* eslint-disable @typescript-eslint/no-floating-promises -- node:test owns returned promises. */
/* eslint-disable sonarjs/no-duplicate-string -- repeated CLI flags make tuple tests legible. */
import { strict as assert } from 'node:assert';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, truncateSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, it } from 'node:test';

import { loadExternalEvidence } from '../lib/performance-evidence.js';
import {
  compareVerdict,
  coverageVerdict,
  COVERAGE_CATEGORIES,
  deriveCoverageEvidence,
  normalizeCapture,
  parsePerfScript,
  parsePerfTrace,
  pearson,
  profileIdentityMatches,
  renderMarkdown,
  reportVerdict,
  trendVerdict,
  validateVerdict,
} from '../lib/performance-report.js';
import { heapTuple, parseArgs, parseCoverage } from '../runtime-performance.js';

import type { ResourceSnapshot } from '../../src/framework/fixtures/process-resources.js';

const TEMP_PREFIX = 'runtime-performance-';
const EXECUTABLE = '/bin/holochain';
const CONFIG_PATH = '/mesh/one.yaml';
const SQLX_LOG_FLAG = '--sqlx-log';
const EVENT_TIME = '1970-01-01T00:00:06.000Z';
const STORAGE_PRODUCER = 'storage-30-1234';

function capture(
  directory: string,
  name: string,
  cpuSeconds: number,
  cohort: string | null = 'same',
  cancelledWriteBytes = 0
): string {
  const path = join(directory, name);
  const beforeSample = {
    peer: 'one',
    pid: 1,
    startTicks: 2,
    executable: EXECUTABLE,
    configPath: CONFIG_PATH,
    cpuTicks: 1,
    rssKiB: 100,
    io: { rchar: 0, wchar: 0, readBytes: 0, writeBytes: 0, cancelledWriteBytes: 0 },
  };
  const afterSample = {
    ...beforeSample,
    cpuTicks: beforeSample.cpuTicks + cpuSeconds * 100,
    rssKiB: 110,
    io: { ...beforeSample.io, readBytes: 1000, cancelledWriteBytes },
  };
  const beforeMetrics = [
    'process_start_time_seconds 1',
    'elohim_conductor_calls_total{zome="z",fn="get",class="read"} 10',
    'elohim_conductor_call_dropped_total{zome="z",fn="get",class="read"} 0',
    'elohim_conductor_call_duration_ms_bucket{zome="z",fn="get",class="read",outcome="success",le="100"} 0',
    'elohim_conductor_call_duration_ms_bucket{zome="z",fn="get",class="read",outcome="success",le="+Inf"} 0',
    'elohim_conductor_call_duration_ms_count{zome="z",fn="get",class="read",outcome="success"} 0',
    'elohim_conductor_call_duration_ms_sum{zome="z",fn="get",class="read",outcome="success"} 0',
    'elohim_conductor_call_duration_ms_bucket{zome="z",fn="get",class="read",outcome="error",le="100"} 0',
    'elohim_conductor_call_duration_ms_bucket{zome="z",fn="get",class="read",outcome="error",le="+Inf"} 0',
    'elohim_conductor_call_duration_ms_count{zome="z",fn="get",class="read",outcome="error"} 0',
    'elohim_conductor_call_duration_ms_sum{zome="z",fn="get",class="read",outcome="error"} 0',
    'elohim_db_diagnostic_query_duration_ms_bucket{operation="capacity_report",statement_site="capacity_measure",outcome="success",le="50"} 0',
    'elohim_db_diagnostic_query_duration_ms_bucket{operation="capacity_report",statement_site="capacity_measure",outcome="success",le="+Inf"} 0',
    'elohim_db_diagnostic_query_duration_ms_count{operation="capacity_report",statement_site="capacity_measure",outcome="success"} 0',
    'elohim_db_diagnostic_query_duration_ms_sum{operation="capacity_report",statement_site="capacity_measure",outcome="success"} 0',
  ].join('\n');
  const afterMetrics = [
    'process_start_time_seconds 1',
    'elohim_conductor_calls_total{zome="z",fn="get",class="read"} 20',
    'elohim_conductor_call_dropped_total{zome="z",fn="get",class="read"} 2',
    'elohim_conductor_call_duration_ms_bucket{zome="z",fn="get",class="read",outcome="success",le="100"} 10',
    'elohim_conductor_call_duration_ms_bucket{zome="z",fn="get",class="read",outcome="success",le="+Inf"} 10',
    'elohim_conductor_call_duration_ms_count{zome="z",fn="get",class="read",outcome="success"} 10',
    'elohim_conductor_call_duration_ms_sum{zome="z",fn="get",class="read",outcome="success"} 500',
    'elohim_conductor_call_duration_ms_bucket{zome="z",fn="get",class="read",outcome="error",le="100"} 1',
    'elohim_conductor_call_duration_ms_bucket{zome="z",fn="get",class="read",outcome="error",le="+Inf"} 1',
    'elohim_conductor_call_duration_ms_count{zome="z",fn="get",class="read",outcome="error"} 1',
    'elohim_conductor_call_duration_ms_sum{zome="z",fn="get",class="read",outcome="error"} 25',
    'elohim_db_diagnostic_query_duration_ms_bucket{operation="capacity_report",statement_site="capacity_measure",outcome="success",le="50"} 2',
    'elohim_db_diagnostic_query_duration_ms_bucket{operation="capacity_report",statement_site="capacity_measure",outcome="success",le="+Inf"} 2',
    'elohim_db_diagnostic_query_duration_ms_count{operation="capacity_report",statement_site="capacity_measure",outcome="success"} 2',
    'elohim_db_diagnostic_query_duration_ms_sum{operation="capacity_report",statement_site="capacity_measure",outcome="success"} 50',
  ].join('\n');
  writeFileSync(
    path,
    JSON.stringify({
      mode: 'metrics',
      issues: [],
      binaryFingerprints: { one: { sha256: name, identityValid: true } },
      telemetry: {
        cohort,
        metrics: [
          {
            name: 'one',
            before: {
              monotonicMs: 100,
              text: `${beforeMetrics}\n`,
            },
            after: {
              monotonicMs: 10_100,
              text: `${afterMetrics}\n`,
            },
          },
        ],
      },
      resources: {
        before: {
          atUnixMs: 1000,
          monotonicMs: 100,
          bootId: 'boot',
          clockTicksPerSecond: 100,
          samples: { one: beforeSample },
          issues: [],
        },
        after: {
          atUnixMs: 11000,
          monotonicMs: 10_100,
          bootId: 'boot',
          clockTicksPerSecond: 100,
          samples: { one: afterSample },
          issues: [],
        },
        elapsedMs: 10_000,
        issues: [],
        deltas: {
          one: {
            peer: 'one',
            pid: 1,
            startTicks: 2,
            executable: EXECUTABLE,
            configPath: CONFIG_PATH,
            cpuSeconds,
            rssKiBBefore: 100,
            rssKiBAfter: 110,
            io: { rchar: 0, wchar: 0, readBytes: 1000, writeBytes: 0, cancelledWriteBytes },
          },
        },
      },
      profiles: [],
    })
  );
  return path;
}

describe('runtime-performance arguments', () => {
  it('bounds trend inputs and requires explicit comparison threshold', () => {
    assert.equal(
      parseArgs(['trend', '--input', '/a', '--input', '/b', '--json']).values.get('--input')
        ?.length,
      2
    );
    assert.throws(() => parseArgs(['report', '--wat', 'x']), /Unknown flag/);
    const isolated = parseArgs(['heap-canary', '--root', '/private/new-canary']);
    assert.equal(isolated.command, 'heap-canary');
    assert.deepEqual(isolated.forwarded, ['--root', '/private/new-canary']);
    assert.equal(
      parseArgs([
        'report',
        '--input',
        '/a',
        SQLX_LOG_FLAG,
        '/one',
        SQLX_LOG_FLAG,
        '/two',
      ]).values.get(SQLX_LOG_FLAG)?.length,
      2
    );
  });

  it('requires a complete distinct trusted-local heap tuple', () => {
    assert.throws(
      () => heapTuple(parseArgs(['report', '--input', '/tmp/capture', '--heap-before', '/tmp/a'])),
      /requires each/
    );
    assert.throws(
      () =>
        heapTuple(
          parseArgs([
            'report',
            '--input',
            '/tmp/capture',
            '--heap-before',
            '/tmp/a',
            '--heap-after',
            '/tmp/a',
            '--heap-binary',
            '/tmp/bin',
            '--jeprof',
            '/tmp/jeprof',
          ])
        ),
      /must be different/
    );
    assert.deepEqual(
      heapTuple(
        parseArgs([
          'check',
          '--input',
          '/tmp/capture',
          '--require-coverage',
          'heap-attribution',
          '--heap-before',
          '/tmp/a',
          '--heap-after',
          '/tmp/b',
          '--heap-binary',
          '/tmp/bin',
          '--jeprof',
          '/tmp/jeprof',
        ])
      ),
      {
        beforeDump: '/tmp/a',
        afterDump: '/tmp/b',
        binaryPath: '/tmp/bin',
        jeprofPath: '/tmp/jeprof',
      }
    );
  });

  it('parses the finite coverage vocabulary and rejects unknowns or duplicates', () => {
    assert.deepEqual(parseCoverage('all'), [...COVERAGE_CATEGORIES]);
    assert.deepEqual(parseCoverage('cpu-leaf,method-counts'), ['cpu-leaf', 'method-counts']);
    assert.throws(() => parseCoverage('cpu-leaf,cpu-leaf'), /must not be duplicated/);
    assert.throws(() => parseCoverage('cpu-leaf,imaginary'), /Unknown coverage category/);
  });
});

describe('performance reports', () => {
  it('rederives SQL capture binding from immutable bytes without granting SQL coverage', () => {
    const directory = mkdtempSync(join(tmpdir(), TEMP_PREFIX));
    const path = capture(directory, 'bound.json', 1);
    const raw = JSON.parse(readFileSync(path, 'utf8')) as {
      resources: { before: ResourceSnapshot; after: ResourceSnapshot };
      diagnostics: {
        sqlTiming: {
          name: string;
          artifactPath: string;
          sourceSha256: string;
          artifactSha256: string;
          bytes: number;
          status: string;
        }[];
      };
    };
    const boot = '11111111-1111-4111-8111-111111111111';
    raw.resources.before.bootId = boot;
    raw.resources.after.bootId = boot;
    raw.resources.before.kernelMonotonicMs = 1000;
    raw.resources.after.kernelMonotonicMs = 11000;
    Object.assign(raw.resources.before, { kernelMonotonicStartMs: 900 });
    Object.assign(raw.resources.after, { kernelMonotonicStartMs: 10900 });
    const admission = {
      nonce: 'sql_nonce',
      producerId: '1-0',
      generation: 1,
      outputBasename: 'sql-timing-g01-sql_nonce.jsonl',
      requestedSeconds: 12,
    };
    const common = {
      diagnostic_schema: 1,
      process_id: 1,
      producer_id: admission.producerId,
      generation: admission.generation,
      nonce: admission.nonce,
      output_basename: admission.outputBasename,
      event_limit: 10_000,
      started_unix_ms: 0,
      requested_seconds: 12,
      started_monotonic_ms: 0,
      native_process: {
        schema: 1,
        process_id: 1,
        process_start_ticks: 2,
        boot_id: boot,
        executable: EXECUTABLE,
        clock: 'CLOCK_MONOTONIC',
      },
    };
    const bytes = Buffer.from(
      [
        { ...common, diagnostic_kind: 'sql_timing_window_started' },
        {
          ...common,
          diagnostic_kind: 'sql_timing_window_closed',
          ended_monotonic_ms: 12000,
          closure_reason: 'expiry',
          coverage_complete: true,
        },
      ]
        .map(row => JSON.stringify(row))
        .join('\n') + '\n'
    );
    const artifactPath = join(directory, 'one.sql-timing.jsonl');
    writeFileSync(artifactPath, bytes, { mode: 0o600 });
    const sha = createHash('sha256').update(bytes).digest('hex');
    raw.diagnostics = {
      sqlTiming: [
        {
          name: 'one',
          artifactPath,
          sourceSha256: sha,
          artifactSha256: sha,
          bytes: bytes.length,
          status: 'refused',
          ...admission,
        },
      ],
    };
    writeFileSync(path, JSON.stringify(raw));
    const run = normalizeCapture(path);
    assert.equal(run.sqlTimingBindings?.[0].status, 'bound');
    assert.equal(run.sqlTimingBindings?.[0].admission, 'exact');
    assert.equal(run.sqlTimingBindings?.[0].coverageEligible, false);
    assert.equal(run.coverageEvidence['sql-timing'].status, 'missing');
    const published = JSON.stringify(reportVerdict(run));
    assert.ok(!published.includes(artifactPath));
    assert.ok(!published.includes(boot));
    assert.ok(!published.includes(admission.nonce));
    assert.ok(!published.includes(admission.outputBasename));

    // Forged convenience flags do not replace the raw native process witness.
    raw.resources.after.samples.one.startTicks = 3;
    raw.diagnostics.sqlTiming[0].status = 'bound';
    writeFileSync(path, JSON.stringify(raw));
    assert.throws(() => normalizeCapture(path), /resource witness/);

    raw.resources.after.samples.one.startTicks = 2;
    raw.diagnostics.sqlTiming[0].artifactSha256 = '0'.repeat(64);
    writeFileSync(path, JSON.stringify(raw));
    const tampered = normalizeCapture(path);
    assert.equal(tampered.sqlTimingBindings?.[0].status, 'refused');
    assert.match(tampered.sqlTimingBindings?.[0].issues.join(' ') ?? '', /bytes do not match/);

    const wrongProcess = Buffer.from(
      bytes.toString().replaceAll('"process_id":1', '"process_id":2')
    );
    writeFileSync(artifactPath, wrongProcess);
    const wrongSha = createHash('sha256').update(wrongProcess).digest('hex');
    Object.assign(raw.diagnostics.sqlTiming[0], {
      sourceSha256: wrongSha,
      artifactSha256: wrongSha,
      bytes: wrongProcess.length,
      status: 'bound',
    });
    writeFileSync(path, JSON.stringify(raw));
    const mismatched = normalizeCapture(path);
    assert.notEqual(mismatched.sqlTimingBindings?.[0].status, 'bound');
    assert.equal(mismatched.sqlTimingBindings?.[0].nativeIdentity, 'mismatch');
  });

  it('rederives workflow admission, native identity, and window without claiming coverage', () => {
    const directory = mkdtempSync(join(tmpdir(), TEMP_PREFIX));
    const path = capture(directory, 'workflow-bound.json', 1);
    const raw = JSON.parse(readFileSync(path, 'utf8')) as {
      resources: { before: ResourceSnapshot; after: ResourceSnapshot };
      diagnostics?: Record<string, unknown>;
    };
    const boot = '11111111-1111-4111-8111-111111111111';
    raw.resources.before.bootId = boot;
    raw.resources.after.bootId = boot;
    Object.assign(raw.resources.before, {
      kernelMonotonicStartMs: 900,
      kernelMonotonicMs: 1000,
    });
    Object.assign(raw.resources.after, {
      kernelMonotonicStartMs: 10900,
      kernelMonotonicMs: 11000,
    });
    const admission = {
      nonce: 'workflow_nonce',
      producerId: 'workflow-1-abcd',
      generation: 1,
      outputBasename: 'workflow-g01-workflow_nonce.jsonl',
      requestedSeconds: 12,
    };
    const common = {
      telemetry_version: 1,
      diagnostic_schema: 1,
      process_id: 1,
      producer_id: admission.producerId,
      generation: admission.generation,
      nonce: admission.nonce,
      output_basename: admission.outputBasename,
      native_process_schema: 1,
      process_start_ticks: 2,
      boot_id: boot,
      executable: EXECUTABLE,
      monotonic_clock: 'CLOCK_MONOTONIC',
      started_unix_ms: 1000,
      started_monotonic_ms: 800,
      requested_seconds: 12,
      event_limit: 10_000,
      scope: 'queue_consumer_workflow_tracing_attempts',
      limitations: 'not_persisted_proof;not_cpu_coverage;not_full_workflow_coverage',
    };
    const artifactBytes = Buffer.from(
      [
        {
          time: '1970-01-01T00:00:01.000Z',
          fields: { ...common, event: 'workflow.diagnostics.window_started' },
        },
        {
          time: '1970-01-01T00:00:13.000Z',
          fields: {
            ...common,
            event: 'workflow.diagnostics.window_closed',
            ended_unix_ms: 13_000,
            ended_monotonic_ms: 12_800,
            observed_seconds: 12,
            closure_reason: 'expiry',
            reserved_detail_events: 0,
            emitted_detail_events: 0,
            remaining_detail_events: 10_000,
            rejected_detail_events_at_close: 0,
            admitted_runs: 0,
            finished_runs_before_close: 0,
            in_flight_runs_at_close: 0,
            censored_in_flight_runs: 0,
          },
        },
      ]
        .map(row => JSON.stringify(row))
        .join('\n') + '\n'
    );
    const artifactPath = join(directory, 'one.workflow.jsonl');
    writeFileSync(artifactPath, artifactBytes, { mode: 0o600 });
    const sha = createHash('sha256').update(artifactBytes).digest('hex');
    raw.diagnostics = {
      workflow: [
        {
          name: 'one',
          artifactPath,
          sourceSha256: sha,
          artifactSha256: sha,
          bytes: artifactBytes.length,
          ...admission,
          status: 'refused',
          admission: 'mismatch',
        },
      ],
    };
    writeFileSync(path, JSON.stringify(raw));
    const run = normalizeCapture(path);
    assert.equal(run.workflowBindings?.[0].status, 'bound');
    assert.equal(run.workflowBindings?.[0].admission, 'exact');
    assert.equal(run.workflowBindings?.[0].nativeIdentity, 'exact');
    assert.equal(run.workflowBindings?.[0].interval, 'encloses');
    assert.equal(run.workflowBindings?.[0].coverageEligible, false);
    assert.match(run.workflowBindings?.[0].warnings.join(' ') ?? '', /provisional/);
    assert.equal(run.coverageEvidence['workflow-runs'].status, 'missing');
    const published = JSON.stringify(reportVerdict(run));
    assert.ok(!published.includes(artifactPath));
    assert.ok(!published.includes(boot));
    assert.ok(!published.includes(admission.nonce));
    assert.ok(!published.includes(admission.outputBasename));
    assert.match(renderMarkdown(reportVerdict(run)), /Workflow capture binding[\s\S]*provisional/i);
    validateVerdict(reportVerdict(run));

    const row = (raw.diagnostics.workflow as Record<string, unknown>[])[0];
    row.nonce = 'other_nonce';
    writeFileSync(path, JSON.stringify(raw));
    const mismatched = normalizeCapture(path);
    assert.equal(mismatched.workflowBindings?.[0].admission, 'mismatch');
    assert.notEqual(mismatched.workflowBindings?.[0].status, 'bound');
  });

  it('normalizes CPU and I/O by actual elapsed time and validates the Verdict schema', () => {
    const dir = mkdtempSync(join(tmpdir(), TEMP_PREFIX));
    const run = normalizeCapture(capture(dir, 'run.json', 5));
    assert.equal(run.totals.cpuCores, 0.5);
    assert.equal(run.totals.ioBytesPerSecond, 100);
    assert.deepEqual(run.binarySha256ByPeer, { one: 'run.json' });
    assert.equal(run.methodLatency.length, 2);
    assert.equal(run.databaseLatency.length, 1);
    assert.equal((run.databaseLatency[0] as { mean: number }).mean, 25);
    assert.equal((run.databaseLatency[0] as { p95: number }).p95, 50);
    assert.equal(run.coverageEvidence['sql-timing'].status, 'missing');
    assert.equal(run.failures.length, 1);
    const verdict = reportVerdict(run);
    assert.equal(verdict.decision.type, 'refer');
    assert.equal(JSON.stringify(verdict).includes(dir), false);
    assert.doesNotMatch(JSON.stringify(verdict), /\/bin\/holochain|\/mesh\/one\.yaml/);
    validateVerdict(verdict);
  });

  it('accepts only paired identity-matched network watermarks with stable membership', () => {
    const dir = mkdtempSync(join(tmpdir(), TEMP_PREFIX));
    const path = capture(dir, 'network.json', 1);
    const raw = JSON.parse(readFileSync(path, 'utf8')) as {
      resources: { before: { samples: { one: Record<string, unknown> } } };
      networkStats?: unknown;
    };
    const identity = raw.resources.before.samples.one;
    const peer = (sendBytes: number, recvBytes: number, atUnixMs: number, monotonicMs: number) => ({
      peer: 'one',
      backend: 'iroh',
      valid: true,
      error: null,
      identity,
      atUnixMs,
      monotonicMs,
      blockedIncoming: 0,
      blockedOutgoing: 0,
      connections: [
        {
          membership: 'a'.repeat(64),
          sendMessageCount: sendBytes / 10,
          sendBytes,
          recvMessageCount: recvBytes / 10,
          recvBytes,
          direct: true,
        },
      ],
    });
    const networkStats = {
      before: {
        atUnixMs: 1000,
        monotonicMs: 1,
        issues: [],
        peers: { one: peer(100, 200, 1100, 10) },
      },
      after: {
        atUnixMs: 2000,
        monotonicMs: 1010,
        issues: [],
        peers: { one: peer(130, 250, 2000, 1010) },
      },
    };
    raw.networkStats = networkStats;
    writeFileSync(path, JSON.stringify(raw));
    const run = normalizeCapture(path);
    assert.equal(run.coverageEvidence['network-watermarks'].status, 'present');
    assert.deepEqual(run.networkWatermarks, [
      {
        peer: 'one',
        backend: 'iroh',
        connectionCount: 1,
        sendBytes: 30,
        recvBytes: 50,
        sendMessages: 3,
        recvMessages: 5,
        blockedIncomingDelta: 0,
        blockedOutgoingDelta: 0,
        elapsedSeconds: 1,
        note: 'matched-connection transport watermarks; no peer keys or URLs retained',
      },
    ]);
    assert.doesNotMatch(JSON.stringify(reportVerdict(run)), /private-hash/);

    const emptyNetwork = structuredClone(networkStats);
    emptyNetwork.before.peers.one.connections = [];
    emptyNetwork.after.peers.one.connections = [];
    writeFileSync(path, JSON.stringify({ ...raw, networkStats: emptyNetwork }));
    const emptyRun = normalizeCapture(path);
    assert.equal(emptyRun.coverageEvidence['network-watermarks'].status, 'present');
    assert.equal((emptyRun.networkWatermarks[0] as { connectionCount: number }).connectionCount, 0);

    interface MutableNetwork {
      before: { peers: { one: { connections?: unknown; valid?: unknown } } };
      after: { peers: { one: { connections?: unknown; valid?: unknown } } };
    }
    const assertInvalidShape = (mutate: (network: MutableNetwork) => void): void => {
      const brokenNetwork = structuredClone(networkStats) as unknown as MutableNetwork;
      mutate(brokenNetwork);
      writeFileSync(path, JSON.stringify({ ...raw, networkStats: brokenNetwork }));
      assert.equal(normalizeCapture(path).coverageEvidence['network-watermarks'].status, 'missing');
    };
    assertInvalidShape(network => delete network.before.peers.one.connections);
    assertInvalidShape(network => (network.before.peers.one.connections = null));
    assertInvalidShape(network => (network.before.peers.one.connections = {}));
    assertInvalidShape(
      network =>
        (network.before.peers.one.connections = Array.from({ length: 10_001 }, () => ({
          membership: 'b'.repeat(64),
          direct: true,
        })))
    );
    assertInvalidShape(network => (network.before.peers.one.valid = 'true'));

    for (const invalid of [undefined, 'not-a-digest']) {
      const brokenNetwork = structuredClone(networkStats);
      // @ts-expect-error Deliberately violate the wire type with missing/invalid identity.
      brokenNetwork.before.peers.one.connections[0].membership = invalid;
      const broken = { ...raw, networkStats: brokenNetwork };
      writeFileSync(path, JSON.stringify(broken));
      assert.equal(normalizeCapture(path).coverageEvidence['network-watermarks'].status, 'missing');
    }
    const brokenDirectNetwork = structuredClone(networkStats);
    // @ts-expect-error Deliberately violate the wire type.
    brokenDirectNetwork.before.peers.one.connections[0].direct = 'true';
    const brokenDirect = { ...raw, networkStats: brokenDirectNetwork };
    writeFileSync(path, JSON.stringify(brokenDirect));
    assert.equal(normalizeCapture(path).coverageEvidence['network-watermarks'].status, 'missing');
  });

  it('refuses only an actual threshold regression and treats missing cohort as insufficient', () => {
    const dir = mkdtempSync(join(tmpdir(), TEMP_PREFIX));
    const base = normalizeCapture(capture(dir, 'base.json', 5));
    const candidate = normalizeCapture(capture(dir, 'candidate.json', 6));
    assert.equal(compareVerdict(base, candidate, 10).decision.type, 'refuse');
    const unscoped = normalizeCapture(capture(dir, 'unscoped.json', 4, null));
    assert.equal(compareVerdict(base, unscoped, 10).decision.type, 'refer');
  });

  it('reports weighted sampled self CPU without calling samples invocations', () => {
    const profile = parsePerfScript(
      'one',
      `holochain 1/1 1.0: 100 cpu-clock:u:\n  abc hot_symbol (/bin/holochain)\n\nholochain 1/1 1.1: 300 cpu-clock:u:\n  def cold_symbol (/bin/holochain)\n`
    ) as {
      sampledCount: number;
      sampledPeriodNs: number;
      topSelf: { name: string; percent: number }[];
    };
    assert.equal(profile.sampledCount, 2);
    assert.equal(profile.sampledPeriodNs, 400);
    assert.equal(profile.topSelf[0].name, 'cold_symbol');
    assert.equal(profile.topSelf[0].percent, 75);
  });

  it('merges instruction offsets into one method self-CPU row', () => {
    const profile = parsePerfScript(
      'one',
      `holochain 1/1 1.0: 100 cpu-clock:u:\n  abc sha512_block_data_order_avx2+0x40c (/lib.so)\n\nholochain 1/1 1.1: 300 cpu-clock:u:\n  def sha512_block_data_order_avx2+0x694 (/lib.so)\n`
    ) as { topSelf: { name: string; periodNs: number; percent: number }[] };
    assert.deepEqual(profile.topSelf, [
      { name: 'sha512_block_data_order_avx2', periodNs: 400, percent: 100 },
    ]);
  });

  it('recovers a resolved header leaf when no callchain frames were recorded', () => {
    const profile = parsePerfScript(
      'one',
      'sqlx-sqlite-wor  1/2  10.0: 200 cpu-clock:u: abc sha512_block_data_order_avx2+0x40c (/bin/holochain)\n'
    ) as { unknownFramePercent: number; topSelf: { name: string; periodNs: number }[] };
    assert.equal(profile.unknownFramePercent, 0);
    assert.deepEqual(profile.topSelf, [
      { name: 'sha512_block_data_order_avx2', periodNs: 200, percent: 100 },
    ]);
  });

  it('keeps caller clusters when a separate callchain projection has reliable stacks', () => {
    const self = Array.from(
      { length: 4 },
      (_, index) => `worker 1/2 ${index}.0: 100 cpu-clock:u: abc hot_leaf (/bin/holochain)`
    ).join('\n');
    const callers = Array.from(
      { length: 4 },
      (_, index) =>
        `worker 1/2 ${index}.0: 100 cpu-clock:u:\n  abc hot_leaf (/bin/holochain)\n  def caller_method (/bin/holochain)\n`
    ).join('\n');
    const profile = parsePerfScript('one', self, '', callers) as {
      callerStackCoveragePercent: number;
      callerKnownPrefixCoveragePercent: number;
      callerClusters: { name: string; percent: number }[];
      topCallerPrefixes: { name: string; percent: number; truncatedByUnknown: boolean }[];
    };
    assert.equal(profile.callerStackCoveragePercent, 100);
    assert.equal(profile.callerKnownPrefixCoveragePercent, 100);
    assert.equal(profile.callerClusters[0].name, 'caller_method');
    assert.equal(profile.callerClusters[0].percent, 100);
    assert.deepEqual(profile.topCallerPrefixes, [
      { name: 'caller_method', periodNs: 400, percent: 100, truncatedByUnknown: false },
    ]);
  });

  it('ranks only the contiguous known caller prefix before an unknown frame', () => {
    const self = 'worker 1/2 0.0: 100 cpu-clock:u: abc leaf (/bin/holochain)';
    const callers = `worker 1/2 0.0: 100 cpu-clock:u:
  abc leaf (/bin/holochain)
  def immediate_caller (/bin/holochain)
  123 [unknown] ([unknown])
  456 higher_known_caller (/bin/holochain)
`;
    const profile = parsePerfScript('one', self, '', callers) as {
      callerStackCoveragePercent: number;
      callerKnownPrefixCoveragePercent: number;
      callerUnknownTailAfterKnownPrefixPercent: number;
      callerImmediateUnavailablePercent: number;
      topCallerPrefixes: { name: string; truncatedByUnknown: boolean }[];
    };
    assert.equal(profile.callerStackCoveragePercent, 0);
    assert.equal(profile.callerKnownPrefixCoveragePercent, 100);
    assert.equal(profile.callerUnknownTailAfterKnownPrefixPercent, 100);
    assert.equal(profile.callerImmediateUnavailablePercent, 0);
    assert.deepEqual(profile.topCallerPrefixes, [
      {
        name: 'immediate_caller',
        periodNs: 100,
        percent: 100,
        truncatedByUnknown: true,
      },
    ]);
    assert.doesNotMatch(JSON.stringify(profile.topCallerPrefixes), /higher_known_caller/);
  });

  it('does not bridge an unknown immediate caller to a known higher ancestor', () => {
    const self = 'worker 1/2 0.0: 100 cpu-clock:u: abc leaf (/bin/holochain)';
    const callers = `worker 1/2 0.0: 100 cpu-clock:u:
  abc leaf (/bin/holochain)
  123 [unknown] ([unknown])
  456 higher_known_caller (/bin/holochain)
`;
    const profile = parsePerfScript('one', self, '', callers) as {
      callerKnownPrefixCoveragePercent: number;
      callerImmediateUnavailablePercent: number;
      topCallerPrefixes: unknown[];
    };
    assert.equal(profile.callerKnownPrefixCoveragePercent, 0);
    assert.equal(profile.callerImmediateUnavailablePercent, 100);
    assert.deepEqual(profile.topCallerPrefixes, []);
  });

  it('labels a leaf-only callchain as an unavailable immediate caller', () => {
    const self = 'worker 1/2 0.0: 100 cpu-clock:u: abc leaf (/bin/holochain)';
    const callers = `worker 1/2 0.0: 100 cpu-clock:u:
  abc leaf (/bin/holochain)
`;
    const profile = parsePerfScript('one', self, '', callers) as {
      callerKnownPrefixCoveragePercent: number;
      callerImmediateUnavailablePercent: number;
      topCallerPrefixes: unknown[];
    };
    assert.equal(profile.callerKnownPrefixCoveragePercent, 0);
    assert.equal(profile.callerImmediateUnavailablePercent, 100);
    assert.deepEqual(profile.topCallerPrefixes, []);
  });

  it('withholds callers when the callchain denominator is partial or duplicated', () => {
    const self = [0, 1]
      .map(index => `worker 1/2 ${index}.0: 100 cpu-clock:u: abc leaf (/bin/holochain)`)
      .join('\n');
    const block = (index: number) =>
      `worker 1/2 ${index}.0: 100 cpu-clock:u:\n  abc leaf (/bin/holochain)\n  def caller (/bin/holochain)\n`;
    for (const callers of [block(0), `${block(0)}\n${block(1)}\n${block(2)}`]) {
      const profile = parsePerfScript('one', self, '', callers) as {
        callerStackCoveragePercent: number | null;
        callerKnownPrefixCoveragePercent: number | null;
        callerUnknownTailAfterKnownPrefixPercent: number | null;
        callerImmediateUnavailablePercent: number | null;
        topCallerPrefixes: unknown[];
        callerClusters: unknown[];
        callerProjectionIssue: string;
      };
      assert.equal(profile.callerStackCoveragePercent, null);
      assert.equal(profile.callerKnownPrefixCoveragePercent, null);
      assert.equal(profile.callerUnknownTailAfterKnownPrefixPercent, null);
      assert.equal(profile.callerImmediateUnavailablePercent, null);
      assert.deepEqual(profile.topCallerPrefixes, []);
      assert.deepEqual(profile.callerClusters, []);
      assert.match(profile.callerProjectionIssue, /denominator mismatch/);
    }
  });

  it('withholds callers when only the callchain projection reports lost data', () => {
    const self = 'worker 1/2 0.0: 100 cpu-clock:u: abc leaf (/bin/holochain)';
    const callers = `${self.replace(' abc leaf (/bin/holochain)', '')}\n  abc leaf (/bin/holochain)\n  def caller (/bin/holochain)\n\nlost 3 events`;
    const profile = parsePerfScript('one', self, '', callers) as {
      usable: boolean;
      callerStackCoveragePercent: number | null;
      callerKnownPrefixCoveragePercent: number | null;
      topCallerPrefixes: unknown[];
      callerProjectionIssue: string;
    };
    assert.equal(profile.usable, true);
    assert.equal(profile.callerStackCoveragePercent, null);
    assert.equal(profile.callerKnownPrefixCoveragePercent, null);
    assert.deepEqual(profile.topCallerPrefixes, []);
    assert.match(profile.callerProjectionIssue, /lost sampling data/);
  });

  it('rejects lost perf events, not only the exact lost-samples spelling', () => {
    const profile = parsePerfScript('one', '', 'Warning: 3 events lost') as {
      usable: boolean;
      reason: string;
    };
    assert.equal(profile.usable, false);
    assert.match(profile.reason, /lost sampling data/);
  });

  it('ranks resolved syscall leaves while redacting arguments and preserving failures', () => {
    const raw = `     0.000 ( 0.016 ms): dd/304210 write(fd: 1, buf: secret, count: 4096) = 4096
                                       7f00 __GI___libc_write+0x10 (/usr/lib64/libc.so.6)
                                       4000 app_flush (/private/path)
     0.100 ( 0.020 ms): dd/304210 read(fd: 3, buf: secret, count: 8) = -5
`;
    const profile = parsePerfTrace('one', raw);
    assert.equal(profile.usable, true);
    assert.equal(profile.successfulUserSyscallBytes, 4096);
    assert.equal(profile.failedSyscalls, 1);
    assert.equal(profile.resolvedLeafCoveragePercent, 50);
    assert.equal(profile.callerStackCoveragePercent, 50);
    assert.doesNotMatch(JSON.stringify(profile), /secret|private\/path|fd:/);
  });

  it('attributes symbolic and numeric failed-syscall formats to their stacks', () => {
    const raw = `0.0 ( 0.010 ms): p/1 read(fd: 99) = -1 (unknown) (Bad file descriptor)
  aa failed_leaf (/bin/private)
  bb failed_caller (/bin/private)
0.1 ( 0.020 ms): p/1 read(fd: 99) = -EBADF (Bad file descriptor)
  aa failed_leaf (/bin/private)
  bb failed_caller (/bin/private)
`;
    const profile = parsePerfTrace('one', raw);
    assert.equal(profile.failedSyscalls, 2);
    assert.equal(profile.resolvedLeafCoveragePercent, 100);
    assert.equal(profile.callerStackCoveragePercent, 100);
    assert.deepEqual(profile.syscallErrors, [{ syscall: 'read', count: 2, latencyMs: 0.03 }]);
    assert.doesNotMatch(JSON.stringify(profile), /bin\/private|fd:/);
  });

  it('rejects lost syscall trace records', () => {
    const profile = parsePerfTrace('one', 'lost 3 events');
    assert.equal(profile.usable, false);
    assert.match(profile.reason ?? '', /lost data/);
  });

  it('uses an explicit absolute guard for zero-to-positive changes while zero-to-zero passes', () => {
    const dir = mkdtempSync(join(tmpdir(), TEMP_PREFIX));
    const zero = normalizeCapture(capture(dir, 'zero.json', 0));
    const positive = normalizeCapture(capture(dir, 'positive.json', 1));
    assert.match(
      compareVerdict(zero, positive, 100).witness.checks[0].summary,
      /zero-baseline guard/
    );
    assert.equal(compareVerdict(zero, zero, 0).decision.type, 'permit');
  });

  it('exposes executable compare exit codes 0, 1, and 2', () => {
    const dir = mkdtempSync(join(tmpdir(), 'runtime-performance-cli-'));
    const base = capture(dir, 'base.json', 5);
    const same = capture(dir, 'same.json', 5);
    const slow = capture(dir, 'slow.json', 10);
    const unscoped = capture(dir, 'unscoped.json', 5, null);
    const cli = join(process.cwd(), 'scripts/runtime-performance.ts');
    const run = (candidate: string) =>
      spawnSync(
        process.execPath,
        [
          '--import',
          'tsx',
          cli,
          'compare',
          '--baseline',
          base,
          '--candidate',
          candidate,
          '--max-regression-percent',
          '10',
          '--json',
        ],
        { encoding: 'utf8' }
      );
    assert.equal(run(same).status, 0);
    assert.equal(run(slow).status, 1);
    assert.equal(run(unscoped).status, 2);
  });

  it('does not invent correlations for short or constant series', () => {
    assert.equal(
      pearson([
        [1, 2],
        [2, 3],
      ]),
      null
    );
    assert.equal(
      pearson([
        [1, 1],
        [1, 2],
        [1, 3],
        [1, 4],
        [1, 5],
      ]),
      null
    );
  });

  it('deduplicates trend captures before correlation', () => {
    const dir = mkdtempSync(join(tmpdir(), TEMP_PREFIX));
    const run = normalizeCapture(capture(dir, 'trend.json', 5));
    const verdict = trendVerdict([run, run, run, run, run]);
    const observed = verdict.witness.checks[0].observed as {
      correlations: { pairCount: number; pearson: number | null }[];
    };
    assert.equal(observed.correlations[0].pairCount, 1);
    assert.equal(observed.correlations[0].pearson, null);
  });

  it('rejects negative CPU and RSS observations', () => {
    const dir = mkdtempSync(join(tmpdir(), TEMP_PREFIX));
    const path = capture(dir, 'negative.json', -1);
    assert.throws(() => normalizeCapture(path), /invalid raw resource snapshots/);
  });

  it('accepts a signed cancelled-write delta while monotonic I/O remains nonnegative', () => {
    const dir = mkdtempSync(join(tmpdir(), TEMP_PREFIX));
    const run = normalizeCapture(capture(dir, 'cancelled.json', 1, 'same', -5));
    assert.equal(run.peersDetail[0].io.cancelledWriteBytes, -5);
  });

  it('requires complete matching perf identity rather than trusting a boolean', () => {
    const sample = {
      peer: 'one',
      pid: 1,
      startTicks: 2,
      executable: EXECUTABLE,
      configPath: CONFIG_PATH,
    };
    const matching = { peer: 'one', identityValid: true, identity: { ...sample } };
    assert.equal(profileIdentityMatches(matching, sample), true);
    assert.equal(profileIdentityMatches({ peer: 'one', identityValid: true }, sample), false);
    assert.equal(
      profileIdentityMatches({ ...matching, identity: { ...sample, pid: 9 } }, sample),
      false
    );
  });

  it('permits categories directly proven by the capture fixture', () => {
    const dir = mkdtempSync(join(tmpdir(), TEMP_PREFIX));
    const run = normalizeCapture(capture(dir, 'coverage-positive.json', 1));
    assert.equal(
      coverageVerdict(run, ['method-counts', 'method-durations', 'method-outcomes']).decision.type,
      'permit'
    );
  });

  it('refers rather than passing when every requested category is missing', () => {
    const dir = mkdtempSync(join(tmpdir(), TEMP_PREFIX));
    const run = normalizeCapture(capture(dir, 'coverage-missing.json', 1));
    const verdict = coverageVerdict(run, [
      'workflow-runs',
      'workflow-triggers',
      'sql-timing',
      'network-watermarks',
      'heap-attribution',
      'io-attribution',
    ]);
    assert.equal(verdict.decision.type, 'refer');
    assert.equal(verdict.witness.checks[0].outcome, 'skipped');
    const markdown = renderMarkdown(verdict);
    assert.match(markdown, /Identity ready: \*\*yes\*\*/);
    assert.match(markdown, /\| heap-attribution \| missing \|/);
    assert.match(markdown, /\| workflow-runs \| missing \|/);
  });

  it('refuses measured caller coverage below the 75 percent readiness floor', () => {
    const dir = mkdtempSync(join(tmpdir(), TEMP_PREFIX));
    const run = normalizeCapture(capture(dir, 'coverage-insufficient.json', 1));
    run.coverageEvidence['cpu-caller'] = {
      status: 'insufficient',
      summary: 'one peer has 42% caller-stack coverage',
      observed: [{ peer: 'one', callerStackCoveragePercent: 42 }],
    };
    const verdict = coverageVerdict(run, ['cpu-caller']);
    assert.equal(verdict.decision.type, 'refuse');
    assert.equal(verdict.witness.checks[0].outcome, 'failed');
  });

  it('keeps known-prefix diagnostics separate from full-caller acceptance', () => {
    const self = Array.from(
      { length: 5 },
      (_, index) => `worker 1/2 ${index}.0: 100 cpu-clock:u: abc leaf (/bin/holochain)`
    ).join('\n');
    const callers = Array.from({ length: 5 }, (_, index) => {
      const tail =
        index === 4
          ? '  def immediate (/bin/holochain)\n'
          : '  def immediate (/bin/holochain)\n  123 [unknown] ([unknown])\n';
      return `worker 1/2 ${index}.0: 100 cpu-clock:u:\n  abc leaf (/bin/holochain)\n${tail}`;
    }).join('\n');
    const profile = parsePerfScript('one', self, '', callers) as {
      peer: string;
      usable: boolean;
      unknownFramePercent: number;
      callerStackCoveragePercent: number;
      callerKnownPrefixCoveragePercent: number;
    };
    assert.equal(profile.callerKnownPrefixCoveragePercent, 100);
    assert.equal(profile.callerStackCoveragePercent, 20);

    const evidence = deriveCoverageEvidence(['one'], [], [profile], [], []);
    assert.equal(evidence['cpu-caller'].status, 'insufficient');

    const dir = mkdtempSync(join(tmpdir(), TEMP_PREFIX));
    const run = normalizeCapture(capture(dir, 'known-prefix-not-acceptance.json', 1));
    run.cpuProfiles = [profile];
    run.coverageEvidence = evidence;
    const verdict = coverageVerdict(run, ['cpu-caller']);
    assert.equal(verdict.decision.type, 'refuse');
    assert.equal(verdict.witness.checks[0].outcome, 'failed');

    const markdown = renderMarkdown(reportVerdict(run));
    assert.match(markdown, /Known caller prefixes \(descriptive; not acceptance\)/);
    assert.match(markdown, /does not contribute to the 75% full-caller coverage requirement/);
  });

  it('does not combine method outcomes from different endpoints', () => {
    const evidence = deriveCoverageEvidence(
      [],
      ['a', 'b'],
      [],
      [{ endpoint: 'a' }, { endpoint: 'b' }],
      [
        { endpoint: 'a', labels: { outcome: 'success' } },
        { endpoint: 'b', labels: { outcome: 'error' } },
      ]
    );
    assert.equal(evidence['method-outcomes'].status, 'missing');
  });

  it('does not infer per-method timeout coverage from errors or dropped callers', () => {
    const evidence = deriveCoverageEvidence(
      [],
      ['a'],
      [],
      [],
      [],
      [],
      [
        { endpoint: 'a', name: 'elohim_conductor_call_dropped_total', labels: { fn: 'read' } },
        { endpoint: 'a', name: 'elohim_conductor_errors_total', labels: { fn: 'read' } },
      ]
    );
    assert.equal(evidence['method-timeouts'].status, 'missing');
  });

  it('requires timeout evidence for every observed method key', () => {
    const methods = [
      { endpoint: 'a', labels: { zome: 'z', fn: 'one', class: 'interactive' } },
      { endpoint: 'a', labels: { zome: 'z', fn: 'two', class: 'interactive' } },
    ];
    const oneTimeout = [
      {
        endpoint: 'a',
        name: 'elohim_conductor_call_timeouts_total',
        labels: { ...methods[0].labels, source: 'websocket' },
      },
    ];
    assert.equal(
      deriveCoverageEvidence([], ['a'], [], methods, [], [], oneTimeout)['method-timeouts'].status,
      'missing'
    );
    const both = [
      ...oneTimeout,
      {
        endpoint: 'a',
        name: 'elohim_conductor_call_timeouts_total',
        labels: { ...methods[1].labels, source: 'websocket' },
      },
    ];
    assert.equal(
      deriveCoverageEvidence([], ['a'], [], methods, [], [], both)['method-timeouts'].status,
      'present'
    );
  });

  it('marks an I/O trace incomplete when observed events reach the configured cap', () => {
    const raw = `0.0 ( 0.1 ms): p/1 write(fd: 1) = 1\n  aa leaf (/bin/x)\n`;
    const profile = parsePerfTrace('one', raw, 1);
    assert.equal(profile.usable, false);
    assert.match(profile.reason ?? '', /max-events/);
  });

  it('does not select a convenient duplicate peer profile', () => {
    const evidence = deriveCoverageEvidence(
      ['one'],
      [],
      [
        { peer: 'one', usable: false },
        { peer: 'one', usable: true, unknownFramePercent: 0, callerStackCoveragePercent: 100 },
      ],
      [],
      []
    );
    assert.equal(evidence['cpu-leaf'].status, 'missing');
  });

  it('imports bounded redacted SQL and exact diagnostic events without granting coverage', () => {
    const dir = mkdtempSync(join(tmpdir(), TEMP_PREFIX));
    const sql = join(dir, 'sql.log');
    const events = join(dir, 'events.jsonl');
    writeFileSync(
      sql,
      '1970-01-01T00:00:05.000Z WARN sqlx::query: slow statement: execution time exceeded alert threshold db.statement="SELECT secret FROM private_table WHERE id = 42" rows_affected=0 rows_returned=1 elapsed_secs=1.25\n'
    );
    writeFileSync(
      events,
      `${JSON.stringify({
        time: EVENT_TIME,
        fields: {
          event: 'workflow.run.finish',
          telemetry_version: '1',
          process_id: '10',
          producer_id: 'workflow-a-1bc',
          run_id: '9',
          workflow: 'publish_dht_ops_consumer',
          dna_hash: 'dna',
          trigger_class: 'external',
          wake_source: 'notification',
          outcome: 'complete',
          duration_seconds: '0.123',
        },
      })}\n${JSON.stringify({
        timestamp: '1970-01-01T00:00:07.000Z',
        fields: {
          diagnostic_schema: '1',
          diagnostic_kind: 'db_query_finish',
          producer_pid: '30',
          producer_id: STORAGE_PRODUCER,
          operation: 'capacity_report',
          correlation: '4',
          query_ordinal: '1',
          statement_site: 'capacity_measure',
          outcome: 'success',
          elapsed_ms: '12',
        },
      })}\n`
    );
    const external = loadExternalEvidence(
      { sqlxLogs: [sql], eventLogs: [events] },
      { startUnixMs: 1000, endUnixMs: 11_000 }
    );
    assert.equal(external.sqlxSlow.length, 1);
    assert.equal(external.diagnosticEvents.length, 2);
    assert.equal(external.workflowSummary.length, 1);
    assert.equal(
      (external.workflowSummary[0] as { observedDurationSeconds: { mean: number } })
        .observedDurationSeconds.mean,
      0.123
    );
    const workflowRate = external.workflowSummary[0] as {
      notificationsPerMinute: number;
      wakeupsPerMinute: number;
      runStartsPerMinute: number;
      runFinishesPerMinute: number;
      observedEventsPerMinute: number;
    };
    assert.equal(workflowRate.notificationsPerMinute, 0);
    assert.equal(workflowRate.wakeupsPerMinute, 0);
    assert.equal(workflowRate.runStartsPerMinute, 0);
    assert.equal(workflowRate.runFinishesPerMinute, 6);
    assert.equal(workflowRate.observedEventsPerMinute, 6);
    const serialized = JSON.stringify(external);
    assert.doesNotMatch(serialized, /private_table|SELECT secret/);

    const run = normalizeCapture(capture(dir, 'evidence.json', 1));
    run.externalEvidence = external;
    assert.equal(run.coverageEvidence['sql-timing'].status, 'missing');
    assert.equal(run.coverageEvidence['workflow-runs'].status, 'missing');
  });

  it('fails loudly on malformed diagnostic JSONL and unpaired network evidence', () => {
    const dir = mkdtempSync(join(tmpdir(), TEMP_PREFIX));
    const broken = join(dir, 'broken.jsonl');
    const network = join(dir, 'network.json');
    writeFileSync(broken, '{not-json}\n');
    writeFileSync(network, '{}');
    const window = { startUnixMs: 1000, endUnixMs: 11_000 };
    assert.throws(
      () => loadExternalEvidence({ sqlxLogs: [], eventLogs: [broken] }, window),
      /invalid diagnostic JSONL/
    );
    assert.throws(
      () => loadExternalEvidence({ sqlxLogs: [], eventLogs: [], networkBefore: network }, window),
      /must be supplied together/
    );
  });

  it('surfaces external evidence through report output', () => {
    const dir = mkdtempSync(join(tmpdir(), TEMP_PREFIX));
    const input = capture(dir, 'cli-evidence.json', 1);
    const events = join(dir, 'workflow.jsonl');
    writeFileSync(
      events,
      `${JSON.stringify({
        time: EVENT_TIME,
        fields: {
          event: 'workflow.trigger.notification',
          telemetry_version: '1',
          process_id: '10',
          producer_id: 'workflow-a-1bc',
          workflow: 'publish_dht_ops_consumer',
          dna_hash: 'dna',
          trigger_class: 'external',
          delivered: 'true',
        },
      })}\n`
    );
    const run = normalizeCapture(input);
    run.externalEvidence = loadExternalEvidence(
      { sqlxLogs: [], eventLogs: [events] },
      { startUnixMs: 1000, endUnixMs: 11_000 }
    );
    const verdict = reportVerdict(run) as unknown as {
      witness: { checks: { observed: { externalEvidence: { diagnosticEvents: unknown[] } } }[] };
    };
    assert.equal(verdict.witness.checks[0].observed.externalEvidence.diagnosticEvents.length, 1);
  });

  it('strips ANSI from SQLx lines and preserves absent row counts as null', () => {
    const dir = mkdtempSync(join(tmpdir(), TEMP_PREFIX));
    const sql = join(dir, 'ansi.log');
    writeFileSync(
      sql,
      '\u001b[33m1970-01-01T00:00:05.000Z WARN sqlx::query: slow statement: execution time exceeded alert threshold db.statement="SELECT private FROM secret_table" elapsed_secs=1.5\u001b[0m\n'
    );
    const evidence = loadExternalEvidence(
      { sqlxLogs: [sql], eventLogs: [] },
      { startUnixMs: 1000, endUnixMs: 11_000 }
    );
    const row = evidence.sqlxSlow[0] as {
      rowsAffected: number | null;
      rowsReturned: number | null;
    };
    assert.equal(row.rowsAffected, null);
    assert.equal(row.rowsReturned, null);
    assert.doesNotMatch(JSON.stringify(evidence), /secret_table|SELECT private|\\u001b/);
  });

  it('accepts strict JsonTimed SQLx fields without emitting statement bytes', () => {
    const dir = mkdtempSync(join(tmpdir(), TEMP_PREFIX));
    const sql = join(dir, 'sql.jsonl');
    writeFileSync(
      sql,
      `${JSON.stringify({
        timestamp: '1970-01-01T00:00:05.000Z',
        fields: {
          message: 'slow statement: execution time exceeded alert threshold',
          'db.statement': 'SELECT hidden FROM private_table WHERE id = 7',
          elapsed_secs: '2.5',
          rows_returned: '3',
        },
      })}\n`
    );
    const evidence = loadExternalEvidence(
      { sqlxLogs: [sql], eventLogs: [] },
      { startUnixMs: 1000, endUnixMs: 11_000 }
    );
    assert.equal((evidence.sqlxSlow[0] as { rowsReturned: number }).rowsReturned, 3);
    assert.doesNotMatch(JSON.stringify(evidence), /private_table|SELECT hidden/);
  });

  it('reads a paired native SQL timing window as bounded, de-identified provisional evidence', () => {
    const dir = mkdtempSync(join(tmpdir(), TEMP_PREFIX));
    const sql = join(dir, 'sql-timing.jsonl');
    const started = {
      diagnostic_schema: 1,
      diagnostic_kind: 'sql_timing_window_started',
      process_id: 42,
      producer_id: '2a-3e8',
      started_unix_ms: 1000,
      requested_seconds: 10,
      event_limit: 10_000,
      identity_limit: 4096,
      statement_byte_limit: 65_536,
      source_site_limit: 4,
      scope: 'completed sqlx::query logger events observed by this layer during the bounded window',
      limitations: ['private native detail'],
    };
    const closed = {
      diagnostic_schema: 1,
      diagnostic_kind: 'sql_timing_window_closed',
      process_id: 42,
      producer_id: '2a-3e8',
      started_unix_ms: 1000,
      ended_unix_ms: 11_000,
      requested_seconds: 10,
      observed_seconds: 10,
      closure_reason: 'expiry',
      coverage_complete: true,
      coverage_scope:
        'only completed sqlx::query logger events delivered to this layer while its gate was active',
      observed_events: 3,
      aggregated_events: 3,
      dropped_events: 0,
      missing_statement_events: 0,
      oversize_statement_events: 0,
      identity_overflow_events: 0,
      malformed_elapsed_events: 0,
      malformed_statement_events: 0,
      arithmetic_overflow_events: 0,
      source_site_limit: 4,
      unattributed_events: 0,
      unmapped_source_site_events: 1,
      source_site_overflow_events: 0,
      identities: [
        {
          statement_hmac_sha256: 'a'.repeat(64),
          count: 2,
          total_elapsed_seconds: 3,
          max_elapsed_seconds: 2,
          rows_affected: null,
          rows_returned: 4,
          source_site_ids: ['dht.publish.select_ready'],
          unattributed_events: 0,
          unmapped_source_site_events: 1,
          source_site_overflow_events: 0,
        },
        {
          statement_hmac_sha256: 'b'.repeat(64),
          count: 1,
          total_elapsed_seconds: 0.5,
          max_elapsed_seconds: 0.5,
          rows_affected: 1,
          rows_returned: null,
          source_site_ids: ['dht.validation.app.pending_ops'],
          unattributed_events: 0,
          unmapped_source_site_events: 0,
          source_site_overflow_events: 0,
        },
      ],
      limitations: ['private native detail'],
    };
    writeFileSync(sql, `${JSON.stringify(started)}\n${JSON.stringify(closed)}\n`);
    const evidence = loadExternalEvidence(
      { sqlxLogs: [sql], eventLogs: [] },
      { startUnixMs: 1000, endUnixMs: 11_000 }
    );
    assert.equal(evidence.sqlTiming.length, 1);
    const timing = evidence.sqlTiming[0] as {
      completeness: string;
      terminalState: string;
      statements: {
        statementRef: string;
        count: number;
        totalElapsedSeconds: number;
        meanElapsedSeconds: number;
        sourceSiteIds: string[];
        mappingCounts: Record<string, number>;
      }[];
      siteMapping: string;
    };
    assert.equal(timing.completeness, 'provisional');
    assert.equal(timing.terminalState, 'expiry-no-loss');
    assert.equal(timing.siteMapping, 'available');
    assert.deepEqual(timing.statements, [
      {
        statementRef: `capture-local-hmac-sha256:${'a'.repeat(64)}`,
        count: 2,
        totalElapsedSeconds: 3,
        meanElapsedSeconds: 1.5,
        maxElapsedSeconds: 2,
        sourceSiteIds: ['dht.publish.select_ready'],
        mappingCounts: {
          unattributedEvents: 0,
          unmappedSourceSiteEvents: 1,
          sourceSiteOverflowEvents: 0,
        },
      },
      {
        statementRef: `capture-local-hmac-sha256:${'b'.repeat(64)}`,
        count: 1,
        totalElapsedSeconds: 0.5,
        meanElapsedSeconds: 0.5,
        maxElapsedSeconds: 0.5,
        sourceSiteIds: ['dht.validation.app.pending_ops'],
        mappingCounts: {
          unattributedEvents: 0,
          unmappedSourceSiteEvents: 0,
          sourceSiteOverflowEvents: 0,
        },
      },
    ]);
    const serialized = JSON.stringify(evidence);
    assert.doesNotMatch(
      serialized,
      /2a-3e8|process_id|statement_hmac|private native detail|SELECT|private_table/
    );
    const run = normalizeCapture(capture(dir, 'sql-timing-evidence.json', 1));
    run.externalEvidence = evidence;
    assert.equal(run.coverageEvidence['sql-timing'].status, 'missing');
    assert.doesNotMatch(JSON.stringify(reportVerdict(run)), /2a-3e8|process_id|statement_hmac/);
    const markdown = renderMarkdown(reportVerdict(run));
    assert.match(markdown, /SQL source-site drilldown \(partial and capture-local\)/);
    assert.match(markdown, /capture-local-hmac-sha256:a{64}/);
    assert.match(markdown, /dht\.publish\.select_ready/);
    assert.match(markdown, /\| 2 \| 3 \| 1\.5 \| 2 \| 0 \| 1 \| 0 \|/);
    assert.match(markdown, /static labels, not per-site latency attribution/);
    assert.match(markdown, /visible rows do not sum to a whole-capture denominator/);
    assert.doesNotMatch(markdown, /2a-3e8|statement_hmac|private native detail/);
  });

  it('rejects incomplete, duplicate, mismatched, malformed, and over-limit native SQL timing records', () => {
    const dir = mkdtempSync(join(tmpdir(), TEMP_PREFIX));
    const sql = join(dir, 'invalid-sql-timing.jsonl');
    const start = {
      diagnostic_schema: 1,
      diagnostic_kind: 'sql_timing_window_started',
      process_id: 42,
      producer_id: '2a-3e8',
      started_unix_ms: 1000,
      requested_seconds: 10,
      event_limit: 10_000,
      identity_limit: 4096,
      statement_byte_limit: 65_536,
      scope: 'logger-only',
      limitations: [],
    };
    const close = {
      ...start,
      diagnostic_kind: 'sql_timing_window_closed',
      ended_unix_ms: 11_000,
      observed_seconds: 10,
      closure_reason: 'expiry',
      coverage_complete: true,
      coverage_scope: 'logger-only',
      observed_events: 0,
      aggregated_events: 0,
      dropped_events: 0,
      missing_statement_events: 0,
      oversize_statement_events: 0,
      identity_overflow_events: 0,
      malformed_elapsed_events: 0,
      malformed_statement_events: 0,
      arithmetic_overflow_events: 0,
      identities: [],
      limitations: [],
    };
    const window = { startUnixMs: 1000, endUnixMs: 11_000 };
    writeFileSync(sql, `${JSON.stringify(start)}\n`);
    assert.throws(
      () => loadExternalEvidence({ sqlxLogs: [sql], eventLogs: [] }, window),
      /unpaired/
    );
    writeFileSync(
      sql,
      `${JSON.stringify(start)}\n${JSON.stringify(close)}\n${JSON.stringify(close)}\n`
    );
    assert.throws(
      () => loadExternalEvidence({ sqlxLogs: [sql], eventLogs: [] }, window),
      /duplicate/
    );
    writeFileSync(
      sql,
      `${JSON.stringify(start)}\n${JSON.stringify({ ...close, requested_seconds: 9 })}\n`
    );
    assert.throws(
      () => loadExternalEvidence({ sqlxLogs: [sql], eventLogs: [] }, window),
      /do not match/
    );
    writeFileSync(
      sql,
      `${JSON.stringify(start)}\n${JSON.stringify({ ...close, identities: [{ statement_hmac_sha256: 'A'.repeat(64), count: 1, total_elapsed_seconds: 1, max_elapsed_seconds: 1 }], observed_events: 1, aggregated_events: 1 })}\n`
    );
    assert.throws(
      () => loadExternalEvidence({ sqlxLogs: [sql], eventLogs: [] }, window),
      /statement identity/
    );
    writeFileSync(
      sql,
      `${JSON.stringify({ ...start, identity_limit: 4097 })}\n${JSON.stringify(close)}\n`
    );
    assert.throws(
      () => loadExternalEvidence({ sqlxLogs: [sql], eventLogs: [] }, window),
      /bounds exceed/
    );
    writeFileSync(
      sql,
      `${JSON.stringify(start)}\n${JSON.stringify({ ...close, closure_reason: 'other' })}\n`
    );
    assert.throws(
      () => loadExternalEvidence({ sqlxLogs: [sql], eventLogs: [] }, window),
      /closure_reason/
    );
    writeFileSync(sql, `${JSON.stringify(start)}\n${JSON.stringify(close)}\n`);
    assert.equal(
      (
        loadExternalEvidence({ sqlxLogs: [sql], eventLogs: [] }, window).sqlTiming[0] as {
          terminalState: string;
          siteMapping: string;
        }
      ).terminalState,
      'expiry-no-loss'
    );
    const legacyEvidence = loadExternalEvidence({ sqlxLogs: [sql], eventLogs: [] }, window);
    assert.equal(
      (legacyEvidence.sqlTiming[0] as { siteMapping: string }).siteMapping,
      'unavailable'
    );
    const legacyRun = normalizeCapture(capture(dir, 'legacy-sql-timing.json', 1));
    legacyRun.externalEvidence = legacyEvidence;
    assert.match(
      renderMarkdown(reportVerdict(legacyRun)),
      /Legacy source-site mapping is unavailable for this window\./
    );
    writeFileSync(
      sql,
      `${JSON.stringify(start)}\n${JSON.stringify({
        ...close,
        closure_reason: 'arithmetic_overflow',
        coverage_complete: false,
        observed_events: 1,
        aggregated_events: 0,
        arithmetic_overflow_events: 1,
      })}\n`
    );
    const incomplete = loadExternalEvidence({ sqlxLogs: [sql], eventLogs: [] }, window)
      .sqlTiming[0] as { completeness: string; terminalState: string };
    assert.equal(incomplete.completeness, 'provisional');
    assert.equal(incomplete.terminalState, 'incomplete');
    writeFileSync(
      sql,
      `${JSON.stringify(start)}\n${JSON.stringify({ ...close, diagnostic_schema: 2 })}\n`
    );
    assert.throws(() => loadExternalEvidence({ sqlxLogs: [sql], eventLogs: [] }, window), /schema/);
    writeFileSync(
      sql,
      `${JSON.stringify(start)}\n${JSON.stringify({ ...close, observed_seconds: 0 })}\n`
    );
    assert.throws(
      () => loadExternalEvidence({ sqlxLogs: [sql], eventLogs: [] }, window),
      /clock witness/
    );
    writeFileSync(
      sql,
      `${JSON.stringify({ ...start, requested_seconds: 60 })}\n${JSON.stringify({ ...close, ended_unix_ms: 59_000, observed_seconds: 59, requested_seconds: 60 })}\n`
    );
    assert.throws(
      () => loadExternalEvidence({ sqlxLogs: [sql], eventLogs: [] }, window),
      /before requested/
    );
    writeFileSync(
      sql,
      `${JSON.stringify(start)}\n${JSON.stringify({ ...close, identities: [{ statement_hmac_sha256: 'a'.repeat(64), count: 2, total_elapsed_seconds: 2, max_elapsed_seconds: 0.5, rows_affected: null, rows_returned: null }], observed_events: 2, aggregated_events: 2 })}\n`
    );
    assert.throws(
      () => loadExternalEvidence({ sqlxLogs: [sql], eventLogs: [] }, window),
      /max aggregate/
    );
    writeFileSync(
      sql,
      `${JSON.stringify({ ...start, identity_limit: 1 })}\n${JSON.stringify({
        ...close,
        identities: [
          {
            statement_hmac_sha256: 'a'.repeat(64),
            count: 1,
            total_elapsed_seconds: 1,
            max_elapsed_seconds: 1,
            rows_affected: null,
            rows_returned: null,
          },
          {
            statement_hmac_sha256: 'b'.repeat(64),
            count: 1,
            total_elapsed_seconds: 1,
            max_elapsed_seconds: 1,
            rows_affected: null,
            rows_returned: null,
          },
        ],
        observed_events: 2,
        aggregated_events: 2,
      })}\n`
    );
    assert.throws(
      () => loadExternalEvidence({ sqlxLogs: [sql], eventLogs: [] }, window),
      /declared identity limit/
    );
    const mappedStart = { ...start, source_site_limit: 4 };
    const mappedIdentity = {
      statement_hmac_sha256: 'c'.repeat(64),
      count: 2,
      total_elapsed_seconds: 2,
      max_elapsed_seconds: 1,
      rows_affected: null,
      rows_returned: null,
      source_site_ids: ['dht.publish.pending_count'],
      unattributed_events: 1,
      unmapped_source_site_events: 0,
      source_site_overflow_events: 0,
    };
    const mappedClose = {
      ...close,
      source_site_limit: 4,
      observed_events: 2,
      aggregated_events: 2,
      identities: [mappedIdentity],
      unattributed_events: 1,
      unmapped_source_site_events: 0,
      source_site_overflow_events: 0,
    };
    writeFileSync(sql, `${JSON.stringify(mappedStart)}\n${JSON.stringify(mappedClose)}\n`);
    assert.equal(
      (
        loadExternalEvidence({ sqlxLogs: [sql], eventLogs: [] }, window).sqlTiming[0] as {
          siteMapping: string;
        }
      ).siteMapping,
      'available'
    );
    writeFileSync(
      sql,
      `${JSON.stringify(mappedStart)}\n${JSON.stringify({
        ...mappedClose,
        identities: [{ ...mappedIdentity, source_site_overflow_events: undefined }],
      })}\n`
    );
    assert.throws(
      () => loadExternalEvidence({ sqlxLogs: [sql], eventLogs: [] }, window),
      /incomplete SQL timing statement source-site mapping/
    );
    writeFileSync(
      sql,
      `${JSON.stringify(mappedStart)}\n${JSON.stringify({
        ...mappedClose,
        identities: [{ ...mappedIdentity, source_site_ids: ['unknown.source'] }],
      })}\n`
    );
    assert.throws(
      () => loadExternalEvidence({ sqlxLogs: [sql], eventLogs: [] }, window),
      /source_site_ids/
    );
    writeFileSync(
      sql,
      `${JSON.stringify(mappedStart)}\n${JSON.stringify({
        ...mappedClose,
        unattributed_events: 3,
        identities: [{ ...mappedIdentity, unattributed_events: 3 }],
      })}\n`
    );
    assert.throws(
      () => loadExternalEvidence({ sqlxLogs: [sql], eventLogs: [] }, window),
      /source-site counters exceed count/
    );
    writeFileSync(
      sql,
      `${JSON.stringify(mappedStart)}\n${JSON.stringify({
        ...mappedClose,
        source_site_overflow_events: 1,
        identities: [{ ...mappedIdentity, source_site_overflow_events: 1 }],
      })}\n`
    );
    assert.throws(
      () => loadExternalEvidence({ sqlxLogs: [sql], eventLogs: [] }, window),
      /source-site counters exceed count/
    );
    writeFileSync(sql, `${JSON.stringify(close)}\n${JSON.stringify(start)}\n`);
    assert.throws(
      () => loadExternalEvidence({ sqlxLogs: [sql], eventLogs: [] }, window),
      /close precedes start/
    );
  });

  it('rejects unsupported versions, nested values, duplicates, and pre-read budget abuse', () => {
    const dir = mkdtempSync(join(tmpdir(), TEMP_PREFIX));
    const invalid = join(dir, 'invalid.jsonl');
    writeFileSync(
      invalid,
      `${JSON.stringify({
        time: EVENT_TIME,
        fields: {
          event: 'workflow.run.start',
          telemetry_version: '2',
          process_id: '10',
          producer_id: 'workflow-a-1bc',
          run_id: '1',
          workflow: { nested: true },
          dna_hash: 'dna',
          cell_token: null,
          trigger_class: 'external',
          wake_source: 'notification',
        },
      })}\n`
    );
    const window = { startUnixMs: 1000, endUnixMs: 11_000 };
    assert.throws(
      () => loadExternalEvidence({ sqlxLogs: [], eventLogs: [invalid] }, window),
      /unsupported workflow telemetry_version/
    );
    writeFileSync(
      invalid,
      `${JSON.stringify({
        time: EVENT_TIME,
        fields: {
          event: 'workflow.run.start',
          telemetry_version: '1',
          process_id: '10',
          producer_id: 'workflow-a-1bc',
          run_id: '1',
          workflow: { nested: true },
          dna_hash: 'dna',
          cell_token: null,
          trigger_class: 'external',
          wake_source: 'notification',
        },
      })}\n`
    );
    assert.throws(
      () => loadExternalEvidence({ sqlxLogs: [], eventLogs: [invalid] }, window),
      /invalid diagnostic workflow/
    );
    assert.throws(
      () =>
        loadExternalEvidence({ sqlxLogs: Array<string>(17).fill(invalid), eventLogs: [] }, window),
      /at most 16/
    );
    assert.throws(
      () => loadExternalEvidence({ sqlxLogs: [invalid, invalid], eventLogs: [] }, window),
      /duplicate evidence file/
    );
    const large = Array.from({ length: 5 }, (_, index) => join(dir, `large-${index}`));
    large.forEach(path => {
      writeFileSync(path, 'x');
      truncateSync(path, 14 * 1024 * 1024);
    });
    assert.throws(
      () => loadExternalEvidence({ sqlxLogs: large, eventLogs: [] }, window),
      /aggregate 67108864 byte limit/
    );
  });

  it('extracts aggregate network counters without private peers, time, or identity claims', () => {
    const dir = mkdtempSync(join(tmpdir(), TEMP_PREFIX));
    const before = join(dir, 'before.json');
    const after = join(dir, 'after.json');
    const dump = (sent: number) => ({
      transport_stats: {
        backend: 'iroh',
        peer_urls: ['private'],
        connections: [
          {
            pub_key: 'private-peer',
            send_message_count: sent,
            send_bytes: sent * 10,
            recv_message_count: 2,
            recv_bytes: 20,
            opened_at_s: 100,
          },
        ],
      },
      blocked_message_counts: {},
    });
    writeFileSync(before, JSON.stringify(dump(1)));
    writeFileSync(after, JSON.stringify(dump(3)));
    const evidence = loadExternalEvidence(
      { sqlxLogs: [], eventLogs: [], networkBefore: before, networkAfter: after },
      { startUnixMs: 1000, endUnixMs: 11_000 }
    );
    const serialized = JSON.stringify(evidence.networkPair);
    assert.match(serialized, /"sendMessages":2/);
    assert.doesNotMatch(serialized, /private-peer|peer_urls/);
    assert.match(serialized, /"contemporaneityVerified":false/);
    assert.match(serialized, /"coverageEligible":false/);
    const changed = dump(4);
    changed.transport_stats.connections[0].opened_at_s = 101;
    writeFileSync(after, JSON.stringify(changed));
    const changedEvidence = loadExternalEvidence(
      { sqlxLogs: [], eventLogs: [], networkBefore: before, networkAfter: after },
      { startUnixMs: 1000, endUnixMs: 11_000 }
    );
    assert.match(JSON.stringify(changedEvidence.networkPair), /"sendMessages":null/);
    assert.match(
      JSON.stringify(changedEvidence.networkPair),
      /"connectionMembershipMatched":false/
    );
  });

  it('reports truncation controls and explicit bounded-output omissions', () => {
    const dir = mkdtempSync(join(tmpdir(), TEMP_PREFIX));
    const events = join(dir, 'bounded.jsonl');
    const notification = (index: number) =>
      JSON.stringify({
        time: EVENT_TIME,
        fields: {
          event: 'workflow.trigger.notification',
          telemetry_version: '1',
          process_id: '10',
          producer_id: 'workflow-a-1bc',
          workflow: `workflow_${index}`,
          dna_hash: 'dna',
          cell_token: null,
          trigger_class: 'external',
          delivered: 'true',
        },
      });
    const control = JSON.stringify({
      timestamp: '1970-01-01T00:00:07.000Z',
      fields: {
        diagnostic_schema: '1',
        diagnostic_kind: 'window_closed',
        producer_pid: '30',
        producer_id: STORAGE_PRODUCER,
        suppressed_terminals: '1',
      },
    });
    writeFileSync(
      events,
      `${Array.from({ length: 101 }, (_, index) => notification(index)).join('\n')}\n${control}\n`
    );
    const evidence = loadExternalEvidence(
      { sqlxLogs: [], eventLogs: [events] },
      { startUnixMs: 1000, endUnixMs: 11_000 }
    );
    assert.equal(evidence.diagnosticEvents.length, 100);
    assert.equal(evidence.omitted.diagnosticEvents, 2);
    assert.equal(evidence.workflowSummary.length, 100);
    assert.equal(evidence.omitted.workflowGroups, 1);
    assert.match(evidence.issues.join(' '), /truncation.*incomplete/);
  });

  it('does not cancel duplicate lifecycle pairs or cross-match storage families', () => {
    const dir = mkdtempSync(join(tmpdir(), TEMP_PREFIX));
    const events = join(dir, 'lifecycle.jsonl');
    const common = {
      diagnostic_schema: '1',
      producer_pid: '30',
      producer_id: STORAGE_PRODUCER,
      correlation: '7',
    };
    const db = (kind: 'db_query_start' | 'db_query_finish') => ({
      ...common,
      diagnostic_kind: kind,
      operation: 'capacity_report',
      statement_site: 'capacity_measure',
      query_ordinal: '1',
      ...(kind.endsWith('_finish') ? { outcome: 'success', elapsed_ms: '1' } : {}),
    });
    writeFileSync(
      events,
      [db('db_query_start'), db('db_query_start'), db('db_query_finish'), db('db_query_finish')]
        .map(fields => JSON.stringify({ timestamp: EVENT_TIME, fields }))
        .join('\n')
    );
    const window = { startUnixMs: 1000, endUnixMs: 11_000 };
    let evidence = loadExternalEvidence({ sqlxLogs: [], eventLogs: [events] }, window);
    assert.match(evidence.issues.join(' '), /1 storage attempt.*duplicate or unmatched/);

    const conductorFinish = {
      ...common,
      diagnostic_kind: 'conductor_attempt_finish',
      operation: 'conductor_call',
      attempt: '1',
      zome: 'zome',
      function: 'fn',
      class: 'read',
      outcome: 'success',
      elapsed_ms: '1',
    };
    writeFileSync(
      events,
      [db('db_query_start'), conductorFinish]
        .map(fields => JSON.stringify({ timestamp: EVENT_TIME, fields }))
        .join('\n')
    );
    evidence = loadExternalEvidence({ sqlxLogs: [], eventLogs: [events] }, window);
    assert.match(evidence.issues.join(' '), /2 storage attempt.*duplicate or unmatched/);
  });
});
