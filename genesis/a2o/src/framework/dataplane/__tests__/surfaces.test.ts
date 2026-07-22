import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import { agentKeyMatchesDiagnosticAgent, parsePrometheusMetrics } from '../surfaces.js';

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

void describe('agentKeyMatchesDiagnosticAgent', () => {
  // REAL public keys observed on alpha (adam): the humans view carries the
  // multibase 'u' + 39-byte payload (3-byte holo type-prefix 0x84 0x20 0x24 +
  // 32-byte core + 4-byte DHT location); conductor-diagnostics carries the
  // bare base64url of the 32-byte core. Note the divergent tails (…GZG2bt69n
  // vs …GZG0): string containment is FALSE for this genuinely-matching pair —
  // the regression this suite pins (the original containment matcher NEVER
  // matched, making the fossil scenario vacuously green).
  const humanKey = 'uhCAkQte6fxZXuJtHlLBb8L87RjsVdKimUsQhdYVAMMLGZG2bt69n';
  const diagAgent = 'Qte6fxZXuJtHlLBb8L87RjsVdKimUsQhdYVAMMLGZG0';

  void it('matches a real humans key against its real diagnostics core (byte-exact)', () => {
    assert.equal(agentKeyMatchesDiagnosticAgent(humanKey, diagAgent), true);
  });

  void it('pins that string containment would NOT have matched this pair', () => {
    // Guard the guard: if the encodings ever change so containment holds, the
    // byte comparison is still correct — but this documents why it exists.
    const humanTail = humanKey.slice(5);
    assert.equal(humanTail.includes(diagAgent) || diagAgent.includes(humanTail), false);
  });

  void it('rejects a diagnostics core that differs from the humans key', () => {
    // Same length, different final core bytes ('GZG0' → 'GZF0').
    const otherAgent = 'Qte6fxZXuJtHlLBb8L87RjsVdKimUsQhdYVAMMLGZF0';
    assert.equal(agentKeyMatchesDiagnosticAgent(humanKey, otherAgent), false);
  });

  void it('tolerates a padded / standard-alphabet diagnostics encoding of the same core', () => {
    const padded = `${diagAgent}=`;
    assert.equal(agentKeyMatchesDiagnosticAgent(humanKey, padded), true);
  });

  void it('rejects empty and malformed inputs', () => {
    assert.equal(agentKeyMatchesDiagnosticAgent('', diagAgent), false);
    assert.equal(agentKeyMatchesDiagnosticAgent(humanKey, ''), false);
    assert.equal(agentKeyMatchesDiagnosticAgent(humanKey, 'not base64!!'), false);
    // A transport id (libp2p peer id) is the wrong shape — never matches.
    assert.equal(agentKeyMatchesDiagnosticAgent(humanKey, '12D3KooWtransport'), false);
    // A too-short humans value can't carry prefix+core.
    assert.equal(agentKeyMatchesDiagnosticAgent('uhCAkshort', diagAgent), false);
  });
});
