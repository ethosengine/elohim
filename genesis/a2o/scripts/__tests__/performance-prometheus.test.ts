/* eslint-disable @typescript-eslint/no-floating-promises -- node:test owns returned promises. */
/* eslint-disable no-useless-escape -- the escaped quotes are Prometheus exposition bytes. */
import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import { summarizePrometheusWindow } from '../lib/performance-prometheus.js';

const STABLE_START = 'process_start_time_seconds 100';

describe('Prometheus performance window', () => {
  it('parses labels, escapes, optional timestamps, counters, and known gauges', () => {
    const before = [
      STABLE_START,
      'requests_total{route="a b",note="line\\n\\\"quoted\\\""} 10 1000',
      'worker_in_flight{pool="one"} 2',
    ].join('\n');
    const after = [
      STABLE_START,
      'requests_total{note="line\\n\\\"quoted\\\"",route="a b"} 16 2000',
      'worker_in_flight{pool="one"} 4',
    ].join('\n');
    const summary = summarizePrometheusWindow(before, after, 30);
    assert.deepEqual(summary.issues, []);
    assert.equal(summary.valid, true);
    assert.deepEqual(summary.producerIdentity, { verified: true, changed: false, reason: null });
    assert.equal(summary.counters[0].delta, 6);
    assert.equal(summary.counters[0].perMinute, 12);
    assert.equal(summary.counters[0].labels.note, 'line\n"quoted"');
    assert.deepEqual(summary.gauges[0], {
      name: 'worker_in_flight',
      labels: { pool: 'one' },
      before: 2,
      after: 4,
    });
  });

  it('reports reset, missing, new, and non-finite series instead of zeroing them', () => {
    const summary = summarizePrometheusWindow(
      'reset_total 9\nmissing_total 2\nbad_total NaN',
      'reset_total 1\nnew_total 3\nbad_total +Inf',
      10
    );
    assert.equal(summary.counters.length, 0);
    assert.ok(summary.issues.some(issue => issue.includes('counter reset')));
    assert.ok(summary.issues.some(issue => issue.includes('missing from after')));
    assert.ok(summary.issues.some(issue => issue.includes('new series')));
    assert.ok(summary.issues.some(issue => issue.includes('not finite')));
  });

  it('honors declared gauge type even when its name ends in total', () => {
    const before = [
      STABLE_START,
      '# TYPE rotating_window_total gauge',
      'rotating_window_total{stream="content"} 100',
    ].join('\n');
    const after = [
      STABLE_START,
      '# TYPE rotating_window_total gauge',
      'rotating_window_total{stream="content"} 40',
    ].join('\n');
    const summary = summarizePrometheusWindow(before, after, 10);
    assert.deepEqual(summary.issues, []);
    assert.deepEqual(summary.counters, []);
    assert.deepEqual(summary.gauges, [
      {
        name: 'rotating_window_total',
        labels: { stream: 'content' },
        before: 100,
        after: 40,
      },
    ]);
  });

  it('rejects malformed, conflicting, and changed TYPE metadata', () => {
    const malformed = summarizePrometheusWindow(
      `${STABLE_START}\n# TYPE bad_total nonsense\nbad_total 1`,
      `${STABLE_START}\n# TYPE bad_total nonsense\nbad_total 2`,
      1
    );
    assert.ok(malformed.issues.some(issue => issue.includes('invalid Prometheus TYPE')));

    const conflict = summarizePrometheusWindow(
      `${STABLE_START}\n# TYPE value gauge\n# TYPE value counter\nvalue 1`,
      `${STABLE_START}\n# TYPE value gauge\nvalue 2`,
      1
    );
    assert.ok(conflict.issues.some(issue => issue.includes('conflicting TYPE')));

    const changed = summarizePrometheusWindow(
      `${STABLE_START}\n# TYPE value gauge\nvalue 1`,
      `${STABLE_START}\n# TYPE value counter\nvalue 2`,
      1
    );
    assert.ok(changed.issues.some(issue => issue.includes('TYPE metadata changed')));
  });

  it('keeps typed histogram components out of gauge and counter projections', () => {
    const scrape = (count: number) =>
      [
        STABLE_START,
        '# TYPE work_ms histogram',
        `work_ms_bucket{le="+Inf"} ${count}`,
        `work_ms_sum ${count}`,
        `work_ms_count ${count}`,
      ].join('\n');
    const summary = summarizePrometheusWindow(scrape(1), scrape(2), 1);
    assert.equal(summary.histograms.length, 1);
    assert.deepEqual(summary.counters, []);
    assert.deepEqual(summary.gauges, []);
  });

  it('does not infer histograms from bucket-suffixed non-histogram families', () => {
    const scrape = (value: number) =>
      [
        STABLE_START,
        '# TYPE gauge_bucket gauge',
        `gauge_bucket ${value}`,
        '# TYPE counter_bucket counter',
        `counter_bucket{le="1"} ${value}`,
        '# TYPE summarized summary',
        `summarized_bucket{le="1"} ${value}`,
        `summarized_count ${value}`,
        `summarized_sum ${value}`,
      ].join('\n');
    const summary = summarizePrometheusWindow(scrape(1), scrape(2), 1);
    assert.deepEqual(summary.histograms, []);
    assert.ok(!summary.issues.some(issue => issue.includes('histogram')));
    assert.deepEqual(summary.gauges, [{ name: 'gauge_bucket', labels: {}, before: 1, after: 2 }]);
    assert.equal(summary.counters.length, 1);
    assert.equal(summary.counters[0].name, 'counter_bucket');
  });

  it('keeps useful deltas provisional when producer identity is unavailable', () => {
    const summary = summarizePrometheusWindow('requests_total 1', 'requests_total 4', 60);
    assert.equal(summary.counters[0].delta, 3);
    assert.equal(summary.producerIdentity.verified, false);
    assert.equal(summary.valid, false);
    assert.equal(summary.producerIdentity.changed, false);
    assert.match(summary.producerIdentity.reason ?? '', /provisional/);
  });

  it('computes histogram deltas without joining label sets', () => {
    const scrape = (count: number, sum: number, first: number) =>
      [
        STABLE_START,
        `latency_seconds_bucket{peer="a",le="0.1"} ${first}`,
        `latency_seconds_bucket{peer="a",le="0.5"} ${count}`,
        `latency_seconds_bucket{peer="a",le="+Inf"} ${count}`,
        `latency_seconds_sum{peer="a"} ${sum}`,
        `latency_seconds_count{peer="a"} ${count}`,
      ].join('\n');
    const summary = summarizePrometheusWindow(scrape(10, 2, 4), scrape(14, 3, 5), 60);
    assert.deepEqual(summary.issues, []);
    assert.deepEqual(summary.histograms[0], {
      name: 'latency_seconds',
      labels: { peer: 'a' },
      count: 4,
      totalMs: 1000,
      mean: 0.25,
      p50: 0.5,
      p95: 0.5,
      p99: 0.5,
      unit: 'seconds',
      quantilesApproximate: true,
      quantileMethod: 'bucket-upper-bound',
    });
  });

  it('returns null when a quantile lands only in the infinite bucket', () => {
    const before =
      'process_start_time_seconds 100\nwork_ms_bucket{le="1"} 0\nwork_ms_bucket{le="+Inf"} 0\nwork_ms_sum 0\nwork_ms_count 0';
    const after =
      'process_start_time_seconds 100\nwork_ms_bucket{le="1"} 1\nwork_ms_bucket{le="+Inf"} 10\nwork_ms_sum 50\nwork_ms_count 10';
    const histogram = summarizePrometheusWindow(before, after, 1).histograms[0];
    assert.equal(histogram.p50, null);
    assert.equal(histogram.p95, null);
    assert.equal(histogram.p99, null);
  });

  it('suppresses all counter and histogram deltas across a process restart', () => {
    const before =
      'process_start_time_seconds 100\nrequests_total 1\nx_bucket{le="+Inf"} 1\nx_count 1';
    const after =
      'process_start_time_seconds 200\nrequests_total 9\nx_bucket{le="+Inf"} 9\nx_count 9';
    const summary = summarizePrometheusWindow(before, after, 2);
    assert.equal(summary.counters.length, 0);
    assert.equal(summary.histograms.length, 0);
    assert.ok(summary.issues.some(issue => issue.includes('deltas suppressed')));
    assert.equal(summary.valid, false);
    assert.deepEqual(summary.producerIdentity, {
      verified: true,
      changed: true,
      reason: 'process_start_time_seconds changed',
    });
  });

  it('invalidates histograms with changed, duplicate, or aliased bucket membership', () => {
    const before =
      'process_start_time_seconds 1\nh_ms_bucket{le="1"} 1\nh_ms_bucket{le="1.0"} 1\nh_ms_bucket{le="+Inf"} 1\nh_ms_sum 1\nh_ms_count 1';
    const after =
      'process_start_time_seconds 1\nh_ms_bucket{le="1"} 2\nh_ms_bucket{le="1.0"} 2\nh_ms_bucket{le="+Inf"} 2\nh_ms_sum 2\nh_ms_count 2';
    const aliased = summarizePrometheusWindow(before, after, 1);
    assert.equal(aliased.histograms.length, 0);
    assert.ok(aliased.issues.some(issue => issue.includes('invalid histogram bucket')));

    const duplicateLabels = summarizePrometheusWindow(
      'process_start_time_seconds 1\nh_ms_bucket{le="1",le="2"} 1',
      'process_start_time_seconds 1\nh_ms_bucket{le="1",le="2"} 2',
      1
    );
    assert.ok(duplicateLabels.issues.some(issue => issue.includes('duplicate label')));
    assert.equal(duplicateLabels.valid, false);
  });

  it('rejects unknown label escapes and non-integer timestamps', () => {
    const summary = summarizePrometheusWindow(
      'process_start_time_seconds 1\nrequests_total{bad="\\q"} 1\nok_total 1 1.5',
      'process_start_time_seconds 1\nrequests_total{bad="\\q"} 2\nok_total 2 1.5',
      1
    );
    assert.equal(summary.valid, false);
    assert.ok(summary.issues.some(issue => issue.includes('invalid label set')));
    assert.ok(summary.issues.some(issue => issue.includes('invalid Prometheus sample')));
  });
});
