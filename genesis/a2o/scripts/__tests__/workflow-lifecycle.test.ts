/* eslint-disable @typescript-eslint/no-floating-promises -- node:test owns returned promises. */
import { strict as assert } from 'node:assert';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, it } from 'node:test';

import { loadExternalEvidence, summarizeSqlTimingArtifact } from '../lib/performance-evidence.js';

const SCOPE = 'queue_consumer_workflow_tracing_attempts';
const LIMITATIONS = 'not_persisted_proof;not_cpu_coverage;not_full_workflow_coverage';
const WINDOW = { startUnixMs: 5_000, endUnixMs: 6_000 };
const common = {
  diagnostic_schema: 1,
  telemetry_version: 1,
  process_id: 42,
  producer_id: 'workflow-2a-3e8',
  generation: 1,
  nonce: 'lifecycle',
  output_basename: 'workflow-g01-lifecycle.jsonl',
  started_unix_ms: 1_000,
  requested_seconds: 10,
  event_limit: 10,
  scope: SCOPE,
  limitations: LIMITATIONS,
};
const start = { event: 'workflow.diagnostics.window_started', ...common };
const nativeIdentity = {
  native_process_schema: 1,
  process_start_ticks: 123,
  boot_id: 'private-boot-id',
  executable: '/private/holochain',
};
const monotonicWitness = {
  monotonic_clock: 'CLOCK_MONOTONIC',
  started_monotonic_ms: 20_000.25,
};

function close(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    event: 'workflow.diagnostics.window_closed',
    ...common,
    ended_unix_ms: 11_000,
    observed_seconds: 10,
    closure_reason: 'expiry',
    reserved_detail_events: 0,
    emitted_detail_events: 0,
    remaining_detail_events: 10,
    rejected_detail_events_at_close: 0,
    admitted_runs: 0,
    finished_runs_before_close: 0,
    in_flight_runs_at_close: 0,
    censored_in_flight_runs: 0,
    ...overrides,
  };
}

function runStart(): Record<string, unknown> {
  return {
    event: 'workflow.run.start',
    telemetry_version: 1,
    process_id: 42,
    producer_id: 'workflow-2a-3e8',
    run_id: 7,
    workflow: 'integrate_dht_ops_consumer',
    dna_hash: 'dna',
    cell_token: 3,
    trigger_class: 'loop',
    wake_source: 'loop',
  };
}

function load(rows: Record<string, unknown>[], window = WINDOW) {
  const directory = mkdtempSync(join(tmpdir(), 'workflow-lifecycle-'));
  const path = join(directory, 'events.jsonl');
  writeFileSync(
    path,
    rows
      .map((fields, index) =>
        JSON.stringify({ time: new Date(1_000 + index * 1_000).toISOString(), fields })
      )
      .join('\n')
  );
  return loadExternalEvidence({ sqlxLogs: [], eventLogs: [path] }, window);
}

describe('bounded workflow lifecycle evidence', () => {
  it('bounds the private in-memory SQL timing artifact adapter', () => {
    assert.deepEqual(summarizeSqlTimingArtifact(Buffer.from('{}\n')), []);
    assert.throws(() => summarizeSqlTimingArtifact(Buffer.alloc(0)), /1\.\.2097152 bytes/);
    assert.throws(
      () => summarizeSqlTimingArtifact(Buffer.alloc(2 * 1024 * 1024 + 1)),
      /1\.\.2097152 bytes/
    );
  });

  it('accepts the live private re-arm SQL producer shape without weakening PID pairing', () => {
    const native = {
      schema: 1,
      process_id: 18,
      process_start_ticks: 1_640_908,
      boot_id: 'private-boot-id',
      executable: '/private/holochain',
      clock: 'CLOCK_MONOTONIC',
    };
    const started = {
      diagnostic_schema: 1,
      diagnostic_kind: 'sql_timing_window_started',
      generation: 1,
      nonce: '4cc48f52e1e99ade2891cf9662d6f019',
      output_basename: 'sql-timing-g01-4cc48f52e1e99ade2891cf9662d6f019.jsonl',
      process_id: 18,
      producer_id: 'sql-12-1a0c67b6d9f',
      started_unix_ms: 1_790_036_373_872,
      native_process: native,
      started_monotonic_ms: 16_410_071.395966,
      requested_seconds: 2,
      event_limit: 10_000,
      identity_limit: 4_096,
      source_site_limit: 4,
      statement_byte_limit: 65_536,
      scope: 'completed sqlx::query logger events observed by this layer during the bounded window',
      limitations: ['quiet native control lifecycle'],
    };
    const closed = {
      ...started,
      diagnostic_kind: 'sql_timing_window_closed',
      ended_monotonic_ms: 16_412_071.395966,
      ended_unix_ms: 1_790_036_375_872,
      observed_seconds: 2.000143949,
      closure_reason: 'expiry',
      coverage_complete: true,
      coverage_scope:
        'only completed sqlx::query logger events delivered to this layer while its gate was active',
      observed_events: 0,
      aggregated_events: 0,
      dropped_events: 0,
      missing_statement_events: 0,
      oversize_statement_events: 0,
      identity_overflow_events: 0,
      malformed_elapsed_events: 0,
      malformed_statement_events: 0,
      arithmetic_overflow_events: 0,
      unattributed_events: 0,
      unmapped_source_site_events: 0,
      source_site_overflow_events: 0,
      identities: [],
    };
    const encode = (rows: Record<string, unknown>[]) =>
      Buffer.from(rows.map(row => JSON.stringify(row)).join('\n'));
    const summary = summarizeSqlTimingArtifact(encode([started, closed]));
    assert.equal(summary.length, 1);
    assert.equal((summary[0] as { closureReason: string }).closureReason, 'expiry');
    assert.throws(
      () =>
        summarizeSqlTimingArtifact(
          encode([
            { ...started, producer_id: 'sql-13-1a0c67b6d9f' },
            { ...closed, producer_id: 'sql-13-1a0c67b6d9f' },
          ])
        ),
      /producer identity/
    );
  });

  it('retains a quiet expiry read before report-window filtering', () => {
    const evidence = load([
      { ...start, ...nativeIdentity, ...monotonicWitness },
      close({ ...nativeIdentity, ...monotonicWitness }),
    ]);
    assert.equal(evidence.diagnosticEvents.length, 2);
    assert.equal(evidence.workflowSummary.length, 0);
    assert.deepEqual(evidence.issues, []);
    assert.doesNotMatch(
      JSON.stringify(evidence),
      /private-boot-id|private\/holochain|process_start_ticks|native_process_schema|started_monotonic_ms|monotonic_clock|"nonce"|"output_basename"|workflow-g01-lifecycle/
    );
  });

  it('accepts independently available native identity and monotonic witness groups', () => {
    assert.doesNotThrow(() => load([{ ...start, ...nativeIdentity }, close(nativeIdentity)]));
    assert.doesNotThrow(() => load([{ ...start, ...monotonicWitness }, close(monotonicWitness)]));
  });

  it('accepts declared right-censoring and uses diagnostic gate wall time for rates', () => {
    const evidence = load([
      start,
      runStart(),
      close({
        reserved_detail_events: 2,
        emitted_detail_events: 1,
        remaining_detail_events: 8,
        admitted_runs: 1,
        in_flight_runs_at_close: 1,
        censored_in_flight_runs: 1,
      }),
    ]);
    const summary = evidence.workflowSummary[0] as {
      runStarts: number;
      runFinishes: number;
      censoredRuns: number;
      observedRateWindowSeconds: number;
      rateBasis: string;
    };
    assert.equal(summary.runStarts, 1);
    assert.equal(summary.runFinishes, 0);
    assert.equal(summary.censoredRuns, 1);
    assert.equal(summary.observedRateWindowSeconds, 10);
    assert.equal(summary.rateBasis, 'diagnostic-active-window');
    assert.match(JSON.stringify(summary), /capture-local-sha256:/);
    assert.doesNotMatch(JSON.stringify(evidence), /"dna_hash"|"cell_token"|"dna"/);
    assert.doesNotMatch(evidence.issues.join(' '), /duplicate or unmatched/);
    assert.match(evidence.issues.join(' '), /right-censored/);
  });

  it('rejects missing, duplicate, reordered, post-close, and unbalanced lifecycles', () => {
    const invalid = [
      [start],
      [start, start, close()],
      [close(), start],
      [start, close(), runStart()],
      [start, close({ emitted_detail_events: 1 })],
      [{ ...start, ...nativeIdentity }, close()],
    ];
    for (const rows of invalid) assert.throws(() => load(rows), /workflow lifecycle/);
  });

  it('preserves legacy provisional unmatched-run behavior', () => {
    const evidence = load([runStart()], { startUnixMs: 0, endUnixMs: 2_000 });
    assert.equal(evidence.workflowSummary.length, 1);
    assert.match(evidence.issues.join(' '), /duplicate or unmatched/);
  });

  it('does not dilute active-window rates when terminal emission is delayed', () => {
    const evidence = load([
      start,
      runStart(),
      close({
        ended_unix_ms: 31_000,
        observed_seconds: 30,
        reserved_detail_events: 2,
        emitted_detail_events: 1,
        remaining_detail_events: 8,
        admitted_runs: 1,
        in_flight_runs_at_close: 1,
        censored_in_flight_runs: 1,
      }),
    ]);
    const summary = evidence.workflowSummary[0] as {
      observedRateWindowSeconds: number;
      runStartsPerMinute: number;
    };
    assert.equal(summary.observedRateWindowSeconds, 10);
    assert.equal(summary.runStartsPerMinute, 6);
  });

  it('bounds lifecycle producers per file and rejects cross-file duplicates', () => {
    const rows = Array.from({ length: 17 }, (_, index) => {
      const producer_id = `workflow-2a-${(1_000 + index).toString(16)}`;
      return [{ ...start, producer_id }, close({ producer_id })];
    }).flat();
    assert.throws(() => load(rows), /at most 16 workflow lifecycle producers/);

    const directory = mkdtempSync(join(tmpdir(), 'workflow-lifecycle-duplicate-'));
    const serialized = [start, close()]
      .map((fields, index) =>
        JSON.stringify({ time: new Date(1_000 + index * 10_000).toISOString(), fields })
      )
      .join('\n');
    const first = join(directory, 'first.jsonl');
    const second = join(directory, 'second.jsonl');
    writeFileSync(first, serialized);
    writeFileSync(second, serialized);
    assert.throws(
      () =>
        loadExternalEvidence(
          { sqlxLogs: [], eventLogs: [first, second] },
          { startUnixMs: 0, endUnixMs: 20_000 }
        ),
      /duplicate workflow lifecycle producer across evidence files/
    );
  });
});
