import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import {
  agentKeyMatchesDiagnosticAgent,
  getRawRidingCatchUp,
  parsePrometheusMetrics,
  CATCHUP_RIDE_MAX_INTERVAL_MS,
  CATCHUP_RIDE_TIMEOUT_MS,
} from '../surfaces.js';

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

/* eslint-disable @typescript-eslint/require-await -- the injected fetch/sleep stubs carry async signatures by contract but resolve synchronously under the fake clock */
void describe('getRawRidingCatchUp', () => {
  const CATCHING_UP = 'catching-up';
  const CONTENT_URL = 'https://peer.test/db/content/x';
  const catchingUp = (retryAfter?: number) => ({
    status: 503,
    text: JSON.stringify(
      retryAfter === undefined ? { status: CATCHING_UP } : { status: CATCHING_UP, retryAfter }
    ),
  });
  const ok = { status: 200, text: '{"id":"x"}' };

  /** Fake clock: sleeps advance time instantly and are recorded. */
  function fakeClock() {
    let now = 0;
    const sleeps: number[] = [];
    return {
      nowFn: () => now,
      sleepFn: async (ms: number) => {
        sleeps.push(ms);
        now += ms;
      },
      sleeps,
    };
  }

  void it('returns a 200 immediately without sleeping', async () => {
    const clock = fakeClock();
    const res = await getRawRidingCatchUp(CONTENT_URL, {
      fetchFn: async () => ok,
      ...clock,
    });
    assert.equal(res.status, 200);
    assert.deepEqual(clock.sleeps, []);
  });

  void it('returns a 404 immediately — only the catching-up shed is ridden', async () => {
    const clock = fakeClock();
    const res = await getRawRidingCatchUp(CONTENT_URL, {
      fetchFn: async () => ({ status: 404, text: 'not found' }),
      ...clock,
    });
    assert.equal(res.status, 404);
    assert.deepEqual(clock.sleeps, []);
  });

  void it('returns a non-catching-up 503 immediately — a plain outage is not ridden', async () => {
    const clock = fakeClock();
    const res = await getRawRidingCatchUp(CONTENT_URL, {
      fetchFn: async () => ({ status: 503, text: 'upstream unavailable' }),
      ...clock,
    });
    assert.equal(res.status, 503);
    assert.deepEqual(clock.sleeps, []);
  });

  void it('rides a catching-up shed honoring retryAfter, then returns the 200', async () => {
    const clock = fakeClock();
    const responses = [catchingUp(30), ok];
    const res = await getRawRidingCatchUp(CONTENT_URL, {
      fetchFn: async () => responses.shift()!,
      ...clock,
    });
    assert.equal(res.status, 200);
    assert.deepEqual(clock.sleeps, [15_000]); // retryAfter=30s clamped to the interval cap
    assert.equal(res.rodeCatchUpMs, 15_000);
  });

  void it('clamps an absent retryAfter to the interval cap', async () => {
    const clock = fakeClock();
    const responses = [catchingUp(), ok];
    const res = await getRawRidingCatchUp(CONTENT_URL, {
      fetchFn: async () => responses.shift()!,
      ...clock,
    });
    assert.equal(res.status, 200);
    assert.deepEqual(clock.sleeps, [CATCHUP_RIDE_MAX_INTERVAL_MS]);
  });

  void it('honors a small retryAfter as-is', async () => {
    const clock = fakeClock();
    const responses = [catchingUp(3), ok];
    const res = await getRawRidingCatchUp(CONTENT_URL, {
      fetchFn: async () => responses.shift()!,
      ...clock,
    });
    assert.equal(res.status, 200);
    assert.deepEqual(clock.sleeps, [3_000]);
  });

  void it('gives up at the deadline and returns the last catching-up 503 honestly', async () => {
    const clock = fakeClock();
    let calls = 0;
    const res = await getRawRidingCatchUp(CONTENT_URL, {
      fetchFn: async () => {
        calls += 1;
        return catchingUp(30);
      },
      ...clock,
    });
    assert.equal(res.status, 503);
    assert.ok(res.text.includes('catching-up'));
    // Bounded: deadline / clamped interval, +1 for the initial probe.
    assert.equal(calls, Math.ceil(CATCHUP_RIDE_TIMEOUT_MS / CATCHUP_RIDE_MAX_INTERVAL_MS) + 1);
    assert.ok((res.rodeCatchUpMs ?? 0) >= CATCHUP_RIDE_TIMEOUT_MS);
  });
});
