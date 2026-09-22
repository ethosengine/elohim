/* eslint-disable @typescript-eslint/no-floating-promises -- node:test owns returned promises. */
/* eslint-disable sonarjs/no-duplicate-string -- witness fixtures intentionally repeat protocol fields. */
import { strict as assert } from 'node:assert';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  closeSync,
  fstatSync,
  lstatSync,
  mkdtempSync,
  openSync,
  readFileSync,
  renameSync,
  symlinkSync,
  truncateSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, it } from 'node:test';

import {
  closeSqlTimingSources,
  closeWorkflowSources,
  evaluateSqlTimingBinding,
  evaluateWorkflowBinding,
  finishSqlTimingSources,
  finishWorkflowSources,
  MAX_SQL_TIMING_BYTES,
  MAX_WORKFLOW_BYTES,
  openSqlTimingSources,
  openWorkflowSources,
  readSqlTimingArtifact,
} from '../lib/performance-binding.js';

import type { ResourceSnapshot } from '../../src/framework/fixtures/process-resources.js';

const PEER = 'one';
const BOOT = '11111111-1111-4111-8111-111111111111';
const EXECUTABLE = '/opt/holochain';
const SQL_ADMISSION = {
  nonce: 'sql_nonce',
  producerId: 'conductor-42',
  generation: 2,
  outputBasename: 'sql-timing-g02-sql_nonce.jsonl',
  requestedSeconds: 1,
};

function snapshot(kernelMonotonicMs: number | undefined): ResourceSnapshot {
  return {
    atUnixMs: 1,
    monotonicMs: 1,
    ...(kernelMonotonicMs === undefined
      ? {}
      : { kernelMonotonicStartMs: kernelMonotonicMs - 10, kernelMonotonicMs }),
    bootId: BOOT,
    clockTicksPerSecond: 100,
    samples: {
      [PEER]: {
        peer: PEER,
        pid: 42,
        startTicks: 7,
        executable: EXECUTABLE,
        configPath: '/mesh/one.yaml',
        cpuTicks: 1,
        rssKiB: 1,
        io: { rchar: 0, wchar: 0, readBytes: 0, writeBytes: 0, cancelledWriteBytes: 0 },
      },
    },
    issues: [],
  };
}

function witness(
  kind: 'sql_timing_window_started' | 'sql_timing_window_closed',
  overrides: Record<string, unknown> = {}
): Record<string, unknown> {
  return {
    diagnostic_schema: 1,
    diagnostic_kind: kind,
    process_id: 42,
    producer_id: 'conductor-42',
    requested_seconds: 1,
    started_monotonic_ms: 50,
    native_process: {
      schema: 1,
      process_id: 42,
      process_start_ticks: 7,
      boot_id: BOOT,
      executable: EXECUTABLE,
      clock: 'CLOCK_MONOTONIC',
    },
    ...(kind === 'sql_timing_window_closed'
      ? { ended_monotonic_ms: 400, closure_reason: 'expiry', coverage_complete: true }
      : {}),
    ...overrides,
  };
}

function bytes(rows: Record<string, unknown>[]): Buffer {
  return Buffer.from(`${rows.map(row => JSON.stringify(row)).join('\n')}\n`);
}

const WORKFLOW_ADMISSION = {
  nonce: 'nonce_1',
  producerId: 'workflow-2a-abcd',
  generation: 1,
  outputBasename: 'workflow-g01-nonce_1.jsonl',
  requestedSeconds: 1,
};

function workflowBytes(overrides: Record<string, unknown> = {}): Buffer {
  const common = {
    telemetry_version: 1,
    diagnostic_schema: 1,
    process_id: 42,
    producer_id: WORKFLOW_ADMISSION.producerId,
    generation: 1,
    nonce: WORKFLOW_ADMISSION.nonce,
    output_basename: WORKFLOW_ADMISSION.outputBasename,
    native_process_schema: 1,
    process_start_ticks: 7,
    boot_id: BOOT,
    executable: EXECUTABLE,
    monotonic_clock: 'CLOCK_MONOTONIC',
    started_unix_ms: 1000,
    started_monotonic_ms: 50,
    requested_seconds: 1,
    event_limit: 10_000,
    scope: 'queue_consumer_workflow_tracing_attempts',
    limitations: 'not_persisted_proof;not_cpu_coverage;not_full_workflow_coverage',
    ...overrides,
  };
  return bytes([
    {
      time: '1970-01-01T00:00:01.000Z',
      fields: { ...common, event: 'workflow.diagnostics.window_started' },
    },
    {
      time: '1970-01-01T00:00:02.000Z',
      fields: {
        ...common,
        event: 'workflow.diagnostics.window_closed',
        ended_unix_ms: 2000,
        ended_monotonic_ms: 400,
        observed_seconds: 1,
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
  ]);
}

describe('SQL timing binding', () => {
  it('accepts an exact identity and enclosing monotonic interval', () => {
    const result = evaluateSqlTimingBinding(
      bytes([witness('sql_timing_window_started'), witness('sql_timing_window_closed')]),
      snapshot(100),
      snapshot(300),
      PEER
    );
    assert.deepEqual(result, {
      status: 'bound',
      nativeIdentity: 'exact',
      interval: 'encloses',
      issues: [],
      warnings: [],
    });
  });

  it('rejects PID reuse, boot, and executable mismatches', () => {
    for (const field of ['process_id', 'boot_id', 'executable']) {
      const native = witness('sql_timing_window_started').native_process as Record<string, unknown>;
      const changed = witness('sql_timing_window_closed', {
        native_process: {
          ...native,
          ...(field === 'process_id' ? { process_id: 43 } : {}),
          ...(field === 'boot_id' ? { boot_id: '22222222-2222-4222-8222-222222222222' } : {}),
          ...(field === 'executable' ? { executable: '/opt/other' } : {}),
        },
        ...(field === 'process_id' ? { process_id: 43 } : {}),
      });
      const result = evaluateSqlTimingBinding(
        bytes([witness('sql_timing_window_started'), changed]),
        snapshot(100),
        snapshot(300),
        PEER
      );
      assert.equal(result.nativeIdentity, 'mismatch');
      assert.notEqual(result.status, 'bound');
    }
    const topOnlyMismatch = evaluateSqlTimingBinding(
      bytes([
        witness('sql_timing_window_started'),
        witness('sql_timing_window_closed', { process_id: 43 }),
      ]),
      snapshot(100),
      snapshot(300),
      PEER
    );
    assert.equal(topOnlyMismatch.nativeIdentity, 'mismatch');
    assert.match(topOnlyMismatch.issues.join(' '), /does not match native/);
  });

  it('distinguishes unavailable native/null and legacy kernel clocks', () => {
    const result = evaluateSqlTimingBinding(
      bytes([
        witness('sql_timing_window_started', { native_process: null }),
        witness('sql_timing_window_closed', { native_process: null }),
      ]),
      snapshot(undefined),
      snapshot(undefined),
      PEER
    );
    assert.equal(result.nativeIdentity, 'unavailable');
    assert.equal(result.interval, 'unavailable');
    assert.match(result.issues.join(' '), /identity unavailable/);
  });

  it('requires one ordered pair, valid duration, and conservative end boundary', () => {
    const duplicate = evaluateSqlTimingBinding(
      bytes([
        witness('sql_timing_window_started'),
        witness('sql_timing_window_started'),
        witness('sql_timing_window_closed'),
      ]),
      snapshot(100),
      snapshot(300),
      PEER
    );
    assert.equal(duplicate.status, 'refused');
    const overrun = evaluateSqlTimingBinding(
      bytes([
        witness('sql_timing_window_started'),
        witness('sql_timing_window_closed', { ended_monotonic_ms: 1051 }),
      ]),
      snapshot(100),
      snapshot(300),
      PEER
    );
    assert.match(overrun.issues.join(' '), /exceeds/);
    const invalidDuration = evaluateSqlTimingBinding(
      bytes([
        witness('sql_timing_window_started', { requested_seconds: 0 }),
        witness('sql_timing_window_closed', { requested_seconds: 0 }),
      ]),
      snapshot(100),
      snapshot(300),
      PEER
    );
    assert.match(invalidDuration.issues.join(' '), /requested_seconds/);
    const outside = evaluateSqlTimingBinding(
      bytes([witness('sql_timing_window_started'), witness('sql_timing_window_closed')]),
      snapshot(100),
      snapshot(500),
      PEER
    );
    assert.equal(outside.interval, 'not-enclosed');
    assert.match(outside.issues.join(' '), /does not enclose/);
  });

  it('optionally binds the exact SQL admission tuple on both lifecycle records', () => {
    const tuple = {
      generation: SQL_ADMISSION.generation,
      nonce: SQL_ADMISSION.nonce,
      output_basename: SQL_ADMISSION.outputBasename,
      event_limit: 10_000,
    };
    const exact = evaluateSqlTimingBinding(
      bytes([
        witness('sql_timing_window_started', tuple),
        witness('sql_timing_window_closed', tuple),
      ]),
      snapshot(100),
      snapshot(300),
      PEER,
      SQL_ADMISSION
    );
    assert.equal(exact.status, 'bound');
    assert.equal(exact.admission, 'exact');
    const substituted = evaluateSqlTimingBinding(
      bytes([
        witness('sql_timing_window_started', tuple),
        witness('sql_timing_window_closed', { ...tuple, nonce: 'other' }),
      ]),
      snapshot(100),
      snapshot(300),
      PEER,
      SQL_ADMISSION
    );
    assert.equal(substituted.admission, 'mismatch');
    assert.notEqual(substituted.status, 'bound');
  });

  it('pins an admitted SQL basename while retaining the preexisting-file path', () => {
    const directory = mkdtempSync(join(tmpdir(), 'sql-admission-'));
    const path = join(directory, SQL_ADMISSION.outputBasename);
    writeFileSync(path, '');
    assert.throws(
      () =>
        openSqlTimingSources(
          [
            {
              name: PEER,
              sourcePath: path,
              admission: { ...SQL_ADMISSION, outputBasename: 'claimed.jsonl' },
            },
          ],
          { [PEER]: '/mesh/one.yaml' }
        ),
      /admitted basename/
    );
    const admitted = openSqlTimingSources(
      [{ name: PEER, sourcePath: path, admission: SQL_ADMISSION }],
      { [PEER]: '/mesh/one.yaml' }
    );
    closeSqlTimingSources(admitted.sources);
    const legacy = openSqlTimingSources([{ name: PEER, sourcePath: path }], {
      [PEER]: '/mesh/one.yaml',
    });
    closeSqlTimingSources(legacy.sources);
  });

  it('does not close a recycled descriptor during repeated source cleanup', () => {
    const directory = mkdtempSync(join(tmpdir(), 'diagnostic-close-'));
    const path = join(directory, 'source.jsonl');
    writeFileSync(path, '');
    const { sources } = openSqlTimingSources([{ name: PEER, sourcePath: path }], {
      [PEER]: '/mesh/one.yaml',
    });
    const original = sources[0].fd;
    closeSqlTimingSources(sources);
    assert.equal(sources[0].fd, -1);
    const replacement = openSync(path, 'r');
    try {
      assert.equal(replacement, original);
      closeSqlTimingSources(sources);
      assert.ok(fstatSync(replacement).isFile());
    } finally {
      closeSync(replacement);
    }
  });

  it('retains the SQL receipt tuple on a completed copied artifact', async () => {
    const directory = mkdtempSync(join(tmpdir(), 'sql-admission-finish-'));
    const path = join(directory, SQL_ADMISSION.outputBasename);
    const tuple = {
      generation: SQL_ADMISSION.generation,
      nonce: SQL_ADMISSION.nonce,
      output_basename: SQL_ADMISSION.outputBasename,
      event_limit: 10_000,
    };
    writeFileSync(
      path,
      bytes([
        witness('sql_timing_window_started', tuple),
        witness('sql_timing_window_closed', tuple),
      ])
    );
    const opened = openSqlTimingSources(
      [{ name: PEER, sourcePath: path, admission: SQL_ADMISSION }],
      { [PEER]: '/mesh/one.yaml' }
    );
    try {
      const diagnostics = await finishSqlTimingSources(
        opened.sources,
        snapshot(100),
        snapshot(300),
        directory,
        { waitMs: 0 }
      );
      assert.equal(diagnostics[0].status, 'bound');
      assert.equal(diagnostics[0].admission, 'exact');
      assert.equal(diagnostics[0].nonce, SQL_ADMISSION.nonce);
      assert.equal(diagnostics[0].outputBasename, SQL_ADMISSION.outputBasename);
    } finally {
      closeSqlTimingSources(opened.sources);
    }
  });

  it('keeps source descriptors bounded and reads private artifacts stably', async () => {
    const directory = mkdtempSync(join(tmpdir(), 'sql-timing-binding-'));
    const sourcePath = join(directory, 'native.jsonl');
    const artifactPath = join(directory, 'artifact.jsonl');
    const content = bytes([witness('sql_timing_window_started')]);
    writeFileSync(sourcePath, content);
    writeFileSync(artifactPath, content, { mode: 0o600 });
    const opened = openSqlTimingSources([{ name: PEER, sourcePath }], {
      [PEER]: '/mesh/one.yaml',
    });
    try {
      const diagnostics = await finishSqlTimingSources(
        opened.sources,
        snapshot(100),
        snapshot(300),
        directory,
        { now: () => 0, sleep: async () => await Promise.resolve(), waitMs: 1, pollMs: 1 }
      );
      assert.equal(diagnostics.length, 1);
      assert.match(diagnostics[0].issues.join(' '), /complete|iteration cap/);
    } finally {
      // finish writes with O_EXCL; no source mutation is needed to close this descriptor.
      opened.sources.forEach(source => {
        try {
          closeSync(source.fd);
        } catch {
          // already closed by test cleanup
        }
      });
    }
    const artifact = readSqlTimingArtifact(artifactPath);
    assert.deepEqual(artifact.bytes, Buffer.from(readFileSync(artifactPath)));
    assert.equal(artifact.sha256, createHash('sha256').update(content).digest('hex'));
    assert.equal((lstatSync(artifactPath).mode & 0o777).toString(8), '600');
    const symlink = join(directory, 'link.jsonl');
    symlinkSync(sourcePath, symlink);
    assert.throws(
      () =>
        openSqlTimingSources([{ name: PEER, sourcePath: symlink }], { [PEER]: '/mesh/one.yaml' }),
      /ELOOP|symbolic|not a regular/
    );
  });

  it('rejects FIFOs and over-limit sources, and reports replacement/truncation', async () => {
    const directory = mkdtempSync(join(tmpdir(), 'sql-timing-bounds-'));
    const fifo = join(directory, 'native.fifo');
    try {
      execFileSync('/usr/bin/mkfifo', [fifo]);
      assert.throws(
        () =>
          openSqlTimingSources([{ name: PEER, sourcePath: fifo }], { [PEER]: '/mesh/one.yaml' }),
        /regular/
      );
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== 'EPERM') throw error;
      // Restricted sandboxes may deny mkfifo; the collector's regular-file guard remains covered
      // by the symlink and over-limit cases in this same test.
    }
    const oversized = join(directory, 'oversized.jsonl');
    writeFileSync(oversized, Buffer.alloc(MAX_SQL_TIMING_BYTES + 1));
    assert.throws(
      () =>
        openSqlTimingSources([{ name: PEER, sourcePath: oversized }], {
          [PEER]: '/mesh/one.yaml',
        }),
      /exceeds/
    );

    const replacement = join(directory, 'replacement.jsonl');
    const replacementSource = bytes([witness('sql_timing_window_started')]);
    writeFileSync(replacement, replacementSource);
    const openedReplacement = openSqlTimingSources(
      [{ name: 'replacement', sourcePath: replacement }],
      { replacement: '/mesh/one.yaml' }
    );
    renameSync(replacement, `${replacement}.old`);
    writeFileSync(replacement, replacementSource);
    try {
      const result = await finishSqlTimingSources(
        openedReplacement.sources,
        snapshot(100),
        snapshot(300),
        directory,
        { waitMs: 0 }
      );
      assert.match(result[0].issues.join(' '), /replaced|inode/);
    } finally {
      closeSqlTimingSources(openedReplacement.sources);
    }

    const truncated = join(directory, 'truncated.jsonl');
    writeFileSync(truncated, replacementSource);
    const openedTruncated = openSqlTimingSources([{ name: 'truncated', sourcePath: truncated }], {
      truncated: '/mesh/one.yaml',
    });
    truncateSync(truncated, 0);
    try {
      const result = await finishSqlTimingSources(
        openedTruncated.sources,
        snapshot(100),
        snapshot(300),
        directory,
        { waitMs: 0 }
      );
      assert.match(result[0].issues.join(' '), /changed|truncated/);
    } finally {
      closeSqlTimingSources(openedTruncated.sources);
    }
  });
});

describe('workflow capture binding', () => {
  it('binds the admission tuple, process identity, and enclosing monotonic window', () => {
    const result = evaluateWorkflowBinding(
      workflowBytes(),
      snapshot(100),
      snapshot(300),
      PEER,
      WORKFLOW_ADMISSION
    );
    assert.equal(result.status, 'bound');
    assert.equal(result.admission, 'exact');
    assert.equal(result.nativeIdentity, 'exact');
    assert.equal(result.interval, 'encloses');
    assert.equal(result.issues.length, 0);
    assert.match(result.warnings.join(' '), /provisional/);
  });

  it('refuses receipt substitution and process/listener-era identity reuse', () => {
    const receiptMismatch = evaluateWorkflowBinding(
      workflowBytes(),
      snapshot(100),
      snapshot(300),
      PEER,
      { ...WORKFLOW_ADMISSION, nonce: 'other' }
    );
    assert.equal(receiptMismatch.admission, 'mismatch');
    assert.notEqual(receiptMismatch.status, 'bound');
    const processMismatch = evaluateWorkflowBinding(
      workflowBytes({ process_start_ticks: 8 }),
      snapshot(100),
      snapshot(300),
      PEER,
      WORKFLOW_ADMISSION
    );
    assert.equal(processMismatch.nativeIdentity, 'mismatch');
    assert.notEqual(processMismatch.status, 'bound');
  });

  it('pins the admitted basename and persists one bounded completed artifact', async () => {
    const directory = mkdtempSync(join(tmpdir(), 'workflow-binding-'));
    const sourcePath = join(directory, WORKFLOW_ADMISSION.outputBasename);
    const content = workflowBytes();
    writeFileSync(sourcePath, content, { mode: 0o600 });
    assert.throws(
      () =>
        openWorkflowSources(
          [
            {
              name: PEER,
              sourcePath,
              admission: { ...WORKFLOW_ADMISSION, outputBasename: 'claimed.jsonl' },
            },
          ],
          { [PEER]: '/mesh/one.yaml' }
        ),
      /admitted basename/
    );
    const opened = openWorkflowSources(
      [{ name: PEER, sourcePath, admission: WORKFLOW_ADMISSION }],
      { [PEER]: '/mesh/one.yaml' }
    );
    try {
      const diagnostics = await finishWorkflowSources(
        opened.sources,
        snapshot(100),
        snapshot(300),
        directory,
        { waitMs: 0 }
      );
      assert.equal(diagnostics[0].status, 'bound');
      assert.equal(diagnostics[0].admission, 'exact');
      assert.equal(diagnostics[0].coverageEligible, false);
      assert.equal(diagnostics[0].outputBasename, WORKFLOW_ADMISSION.outputBasename);
      assert.equal(diagnostics[0].artifactPath, join(directory, 'one.workflow.jsonl'));
      assert.equal(
        diagnostics[0].artifactSha256,
        createHash('sha256').update(content).digest('hex')
      );
    } finally {
      closeWorkflowSources(opened.sources);
    }
  });

  it('applies the shared hard byte cap and finite terminal wait to workflow sources', async () => {
    const oversizedDirectory = mkdtempSync(join(tmpdir(), 'workflow-cap-'));
    const oversizedPath = join(oversizedDirectory, WORKFLOW_ADMISSION.outputBasename);
    writeFileSync(oversizedPath, Buffer.alloc(MAX_WORKFLOW_BYTES + 1));
    assert.throws(
      () =>
        openWorkflowSources(
          [{ name: PEER, sourcePath: oversizedPath, admission: WORKFLOW_ADMISSION }],
          { [PEER]: '/mesh/one.yaml' }
        ),
      /exceeds/
    );

    const incompleteDirectory = mkdtempSync(join(tmpdir(), 'workflow-wait-'));
    const incompletePath = join(incompleteDirectory, WORKFLOW_ADMISSION.outputBasename);
    writeFileSync(incompletePath, `${workflowBytes().toString().split('\n')[0]}\n`);
    const opened = openWorkflowSources(
      [{ name: PEER, sourcePath: incompletePath, admission: WORKFLOW_ADMISSION }],
      { [PEER]: '/mesh/one.yaml' }
    );
    try {
      const diagnostics = await finishWorkflowSources(
        opened.sources,
        snapshot(100),
        snapshot(300),
        incompleteDirectory,
        { now: () => 0, sleep: async () => await Promise.resolve(), waitMs: 1, pollMs: 1 }
      );
      assert.notEqual(diagnostics[0].status, 'bound');
      assert.match(diagnostics[0].issues.join(' '), /terminal|iteration cap/);
    } finally {
      closeWorkflowSources(opened.sources);
    }
  });

  it('ends terminal polling promptly when the owning lifecycle aborts', async () => {
    const directory = mkdtempSync(join(tmpdir(), 'workflow-abort-'));
    const sourcePath = join(directory, WORKFLOW_ADMISSION.outputBasename);
    writeFileSync(sourcePath, `${workflowBytes().toString().split('\n')[0]}\n`);
    const opened = openWorkflowSources(
      [{ name: PEER, sourcePath, admission: WORKFLOW_ADMISSION }],
      { [PEER]: '/mesh/one.yaml' }
    );
    const controller = new AbortController();
    let sleeps = 0;
    try {
      const diagnostics = await finishWorkflowSources(
        opened.sources,
        snapshot(100),
        snapshot(300),
        directory,
        {
          waitMs: 5000,
          pollMs: 250,
          signal: controller.signal,
          sleep: async () => {
            sleeps += 1;
            controller.abort();
            await Promise.resolve();
          },
        }
      );
      assert.equal(sleeps, 1);
      assert.match(diagnostics[0].issues.join(' '), /wait aborted/);
    } finally {
      closeWorkflowSources(opened.sources);
    }
  });
});
