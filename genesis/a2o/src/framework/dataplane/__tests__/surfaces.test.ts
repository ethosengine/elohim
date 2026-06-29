import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import { parsePrometheusMetrics } from '../surfaces.js';

void describe('parsePrometheusMetrics', () => {
  void it('parses a plain metric line', () => {
    const m = parsePrometheusMetrics('http_requests_total 42\n');
    assert.equal(m.get('http_requests_total'), 42);
  });

  void it('parses a metric with labels', () => {
    const m = parsePrometheusMetrics('http_requests_total{method="GET",code="200"} 17\n');
    assert.equal(m.get('http_requests_total'), 17);
  });

  void it('ignores the timestamp token — does not mistake it for the value', () => {
    // lastIndexOf(' ') bug: picks 1234567890000 as the value. Correct: pick 99.
    const m = parsePrometheusMetrics('process_resident_memory_bytes 99 1234567890000\n');
    assert.equal(m.get('process_resident_memory_bytes'), 99);
  });

  void it('ignores the timestamp when labels are present', () => {
    const m = parsePrometheusMetrics(
      'doorway_watchdog_reconnects_total{peer="alpha"} 3 1700000000000\n'
    );
    assert.equal(m.get('doorway_watchdog_reconnects_total'), 3);
  });

  void it('skips comment lines', () => {
    const input = [
      '# HELP http_requests_total Total requests',
      '# TYPE http_requests_total counter',
      'http_requests_total 5',
    ].join('\n');
    const m = parsePrometheusMetrics(input);
    assert.equal(m.get('http_requests_total'), 5);
    assert.equal(m.size, 1);
  });

  void it('keeps only the first series for a repeated metric name (label variants)', () => {
    const input = [
      'http_requests_total{method="GET"} 10',
      'http_requests_total{method="POST"} 20',
    ].join('\n');
    const m = parsePrometheusMetrics(input);
    assert.equal(m.get('http_requests_total'), 10);
  });

  void it('returns an empty map for blank input', () => {
    assert.equal(parsePrometheusMetrics('').size, 0);
  });
});
