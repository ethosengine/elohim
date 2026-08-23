import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import {
  agentKeyMatchesDiagnosticAgent,
  classifyStorageTransportStatus,
  describeCatchUpRide,
  getRaw,
  getRawRidingCatchUp,
  parsePrometheusMetrics,
  shedCause,
  CATCHUP_RIDE_MAX_INTERVAL_MS,
  CATCHUP_RIDE_TIMEOUT_MS,
} from '../surfaces.js';

void describe('classifyStorageTransportStatus', () => {
  void it('distinguishes libp2p, iroh, dual, and unknown live status shapes', () => {
    assert.equal(classifyStorageTransportStatus({ peerId: '12D3KooWpeer' }), 'libp2p');
    assert.equal(classifyStorageTransportStatus({ peerId: 'a'.repeat(64) }), 'iroh');
    assert.equal(
      classifyStorageTransportStatus({ peerId: '12D3KooWpeer', irohNodeId: 'b'.repeat(64) }),
      'dual'
    );
    assert.equal(classifyStorageTransportStatus({}), 'unknown');
  });
});

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

void describe('shedCause / describeCatchUpRide — routing a shed to the right plane', () => {
  const rode = (text: string) => ({ status: 503, text, rodeCatchUpMs: 42_000 });

  void it('reads the upstream cause the doorway now names', () => {
    const named = shedCause('{"status":"catching-up","cause":"upstream","circuit":"open"}');
    assert.deepEqual(named, { cause: 'upstream', circuit: 'open' });
  });

  void it('reads the admission cause', () => {
    assert.deepEqual(shedCause('{"status":"catching-up","cause":"admission"}'), {
      cause: 'admission',
    });
  });

  void it('returns null for a pre-2026-08-20 doorway that names no cause', () => {
    assert.equal(shedCause('{"status":"catching-up","retryAfter":30}'), null);
    assert.equal(shedCause('not json at all'), null);
  });

  void it('routes an upstream shed AWAY from the admission runbook', () => {
    const msg = describeCatchUpRide(
      rode('{"status":"catching-up","cause":"upstream","circuit":"open"}')
    );
    assert.ok(msg.includes('cause=UPSTREAM'), msg);
    assert.ok(msg.includes('circuit=open'), msg);
    assert.ok(msg.includes('availability failure'), msg);
    // The misroute this whole change exists to stop: triage must be pointed at
    // the substrate-trust-contract runbook, NOT at the admission runbook. The
    // discriminator is the admission runbook's own identifying phrase — the
    // message may (and does) name the admission runbook in order to rule it out.
    assert.ok(msg.includes('substrate-trust-contract'), msg);
    assert.ok(!msg.includes('Content view sheds 503 catching-up'), msg);
  });

  void it('routes a genuine admission shed TO the admission runbook', () => {
    const msg = describeCatchUpRide(rode('{"status":"catching-up","cause":"admission"}'));
    assert.ok(msg.includes('cause=ADMISSION'), msg);
    assert.ok(msg.includes('Content view sheds 503 catching-up'), msg);
  });

  void it('says plainly that an old doorway did not report a cause', () => {
    const msg = describeCatchUpRide(rode('{"status":"catching-up","retryAfter":30}'));
    assert.ok(msg.includes('cause unreported'), msg);
    // Must not assert a plane it cannot see.
    assert.ok(!msg.includes('cause=UPSTREAM'), msg);
    assert.ok(!msg.includes('cause=ADMISSION'), msg);
  });

  void it('stays silent when no shed was ridden at all', () => {
    assert.equal(describeCatchUpRide({ status: 200, text: '{}' }), '');
  });

  void it('reports a transient shed that ended in a non-shed answer', () => {
    const msg = describeCatchUpRide({ status: 404, text: 'gone', rodeCatchUpMs: 5_000 });
    assert.ok(msg.includes('transient catching-up shed'), msg);
  });
});

void describe('getRaw — timeoutMs is a TOTAL bound, connect included', () => {
  /*
   * REGRESSION GUARD (build elohim-genesis/dev #1489 sweep, 2026-08-20).
   *
   * `bodyTimeout`/`headersTimeout` only start once a socket exists; neither
   * bounds the TCP connect, which undici defaults to 10s. So a caller asking
   * for a 4s bound waited 10.5s — MEASURED here before the fix:
   *
   *   getRaw('http://10.255.255.1:8080/health', { timeoutMs: 4000 })
   *     -> threw "Connect Timeout Error (attempted address: 10.255.255.1:8080,
   *        timeout: 10000ms)" after 10492 ms
   *
   * That is not a cosmetic overrun: steps/common.steps.ts sizes a browser
   * step's whole failure diagnosis off REACH_PROBE_TIMEOUT_MS, so a probe that
   * silently ran 2.6x its declared bound pushed the step past
   * CUCUMBER_STEP_DEADLINE_MS and cucumber killed it with the bare
   * "function timed out" line that browserStep exists to abolish.
   *
   * 10.255.255.1 is in RFC-1918 space with no route out of this container, so
   * the connect hangs rather than being refused. Where a network DOES answer
   * or refuse it fast, the assertion still holds (it only ever tightens) —
   * the test asserts the CEILING, never that a failure occurred.
   */
  // Deliberately plain http: the point is a TCP connect that never completes.
  // TLS would add a second unbounded phase and tell us nothing about the bound
  // under test.
  // eslint-disable-next-line sonarjs/no-clear-text-protocols
  const UNROUTABLE = 'http://10.255.255.1:8080/health';
  const BOUND_MS = 1_500;
  /* Loose enough not to flake on a slow runner; far below undici's 10s
   * connect default, so the pre-fix behaviour could never pass it. */
  const CEILING_MS = 5_000;

  void it("settles an unroutable address within its own bound, not undici's connect default", async () => {
    const started = Date.now();
    try {
      await getRaw(UNROUTABLE, { timeoutMs: BOUND_MS });
    } catch {
      // Either outcome is fine; the contract under test is WHEN it settles.
    }
    const elapsed = Date.now() - started;
    assert.ok(
      elapsed < CEILING_MS,
      `getRaw ignored its ${BOUND_MS}ms bound: settled after ${elapsed}ms ` +
        `(the pre-fix connect-unbounded shape took ~10500ms)`
    );
  });
});
