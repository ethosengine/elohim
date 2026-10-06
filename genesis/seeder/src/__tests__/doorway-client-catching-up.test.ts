/**
 * Tests for the seeder's central catching-up (projector-backpressure) retry.
 *
 * Under seed load the projector re-enters the `catching-up` admission state and
 * sheds writes with 503 {"status":"catching-up","retryAfter":N} (genesis
 * #1079/#1182/#1191). A shed write was REJECTED, not lost — DoorwayClient.fetch()
 * retries the whole request until the projector drains. This retry lives in the
 * central fetch() so EVERY seed path (stewardship / projections / commitments /
 * operator-bindings) is resilient, not just /import.
 *
 * The mock returns Retry-After: 0 so the retry delay collapses to ~0ms and the
 * test runs without fake timers.
 */
import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  resetShedRetryProcessBudget,
  SHED_RETRY_PROCESS_BOUND_MS,
  DoorwayClient,
  SHED_RETRY_DELAY_CAP_MS,
  SHED_RETRY_MAX_RETRIES,
  SHED_RETRY_TOTAL_BOUND_MS,
  sendRetryingShed,
  shedRetryDelayMs,
} from '../doorway-client.js';

/** Exposes the protected fetch() so we can exercise the retry directly. */
class TestClient extends DoorwayClient {
  public callFetch(path: string, options: Parameters<DoorwayClient['fetch']>[1] = {}) {
    return this.fetch(path, options);
  }
}

const catchingUp = () =>
  new Response(JSON.stringify({ status: 'catching-up', retryAfter: 0 }), {
    status: 503,
    headers: { 'Content-Type': 'application/json', 'Retry-After': '0' },
  });

/** A fetch mock whose Nth-call behavior is scripted per path via a queue. */
function scriptedFetch(script: Record<string, Array<() => Response>>) {
  const counts: Record<string, number> = {};
  const impl = vi.fn(async (url: string | URL) => {
    const path = new URL(typeof url === 'string' ? url : url.toString()).pathname;
    const queue = script[path];
    if (!queue) return new Response('not found', { status: 404 });
    const i = counts[path] ?? 0;
    counts[path] = i + 1;
    // Last scripted response repeats once the queue is exhausted.
    return (queue[i] ?? queue[queue.length - 1])();
  });
  return { impl, counts };
}

describe('DoorwayClient catching-up (503) retry', () => {
  const originalFetch = globalThis.fetch;

  afterEach(() => {
    globalThis.fetch = originalFetch;
    vi.restoreAllMocks();
  });

  it('retries a catching-up 503 and succeeds once the projector drains', async () => {
    const { impl, counts } = scriptedFetch({
      '/db/allocations/bulk': [
        catchingUp,
        catchingUp,
        () => new Response(JSON.stringify({ created: 3 }), { status: 200 }),
      ],
    });
    globalThis.fetch = impl as unknown as typeof fetch;

    const client = new TestClient({ baseUrl: 'https://doorway-alpha.elohim.host' });
    const res = await client.callFetch('/db/allocations/bulk', { method: 'POST' });

    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ created: 3 });
    expect(counts['/db/allocations/bulk']).toBe(3); // two sheds + the success
  });

  it('does NOT retry a 503 that is not a catching-up shed; body stays readable', async () => {
    const { impl, counts } = scriptedFetch({
      '/api/v1/commitments': [
        () => new Response(JSON.stringify({ error: 'boom' }), { status: 503 }),
      ],
    });
    globalThis.fetch = impl as unknown as typeof fetch;

    const client = new TestClient({ baseUrl: 'https://doorway-alpha.elohim.host' });
    const res = await client.callFetch('/api/v1/commitments', { method: 'POST' });

    expect(res.status).toBe(503);
    expect(await res.json()).toEqual({ error: 'boom' }); // reconstructed, still readable
    expect(counts['/api/v1/commitments']).toBe(1); // no retry
  });

  it('gives up after the bounded budget and returns a readable 503', async () => {
    const { impl, counts } = scriptedFetch({
      '/db/presences': [catchingUp], // always shedding
    });
    globalThis.fetch = impl as unknown as typeof fetch;

    const client = new TestClient({ baseUrl: 'https://doorway-alpha.elohim.host' });
    const res = await client.callFetch('/db/presences', { method: 'POST' });

    expect(res.status).toBe(503);
    expect(await res.json()).toEqual({ status: 'catching-up', retryAfter: 0 });
    // 1 initial attempt + 12 retries (catchingUpMax) = 13 calls
    expect(counts['/db/presences']).toBe(13);
  });
});

describe('sendRetryingShed — the shared 503 policy plain-fetch seeders use', () => {
  const shed = (body: object, headers: Record<string, string> = {}) => () =>
    Promise.resolve(new Response(JSON.stringify(body), { status: 503, headers }));

  it('honours the body retryAfter of a circuit-open catching-up shed, then succeeds', async () => {
    const sleeps: number[] = [];
    const answers = [
      shed({ status: 'catching-up', cause: 'upstream', circuit: 'open', retryAfter: 7 }),
      () => Promise.resolve(new Response('{}', { status: 201 })),
    ];
    let n = 0;
    const res = await sendRetryingShed(() => answers[n++](), 'POST /db/presences', async ms => {
      sleeps.push(ms);
    });
    expect(res.status).toBe(201);
    expect(sleeps).toEqual([7000]);
  });

  it('caps a large hint, and backs off (capped doubling) when no hint is given', () => {
    expect(shedRetryDelayMs(503, '{"retryAfter":600}', null, 0)).toBe(SHED_RETRY_DELAY_CAP_MS);
    expect(shedRetryDelayMs(503, '{"status":"catching-up"}', null, 0)).toBe(2000);
    expect(shedRetryDelayMs(503, '{"status":"catching-up"}', null, 1)).toBe(4000);
    expect(shedRetryDelayMs(503, '{"status":"catching-up"}', null, 9)).toBe(SHED_RETRY_DELAY_CAP_MS);
    expect(shedRetryDelayMs(503, 'not json', '3', 0)).toBe(3000);
  });

  it('never retries a non-503 or a 503 with no retry hint', async () => {
    expect(shedRetryDelayMs(500, '{"retryAfter":1}', '1', 0)).toBeNull();
    expect(shedRetryDelayMs(503, '{"error":"boom"}', null, 0)).toBeNull();
    let calls = 0;
    const res = await sendRetryingShed(
      () => {
        calls++;
        return Promise.resolve(new Response('{"error":"x"}', { status: 409 }));
      },
      'POST /x',
      async () => {}
    );
    expect(res.status).toBe(409);
    expect(calls).toBe(1);
  });

  it('stops at the total sleep bound and returns a readable 503', async () => {
    let calls = 0;
    let slept = 0;
    const res = await sendRetryingShed(
      () => {
        calls++;
        return shed({ status: 'catching-up', retryAfter: 15 })();
      },
      'POST /db/collectives',
      async ms => {
        slept += ms;
      }
    );
    expect(res.status).toBe(503);
    expect(await res.json()).toEqual({ status: 'catching-up', retryAfter: 15 });
    expect(slept).toBeLessThanOrEqual(SHED_RETRY_TOTAL_BOUND_MS);
    expect(calls).toBe(Math.min(SHED_RETRY_MAX_RETRIES, Math.floor(SHED_RETRY_TOTAL_BOUND_MS / 15_000)) + 1);
  });
});

describe('sendRetryingShed — process-wide shed budget', () => {
  afterEach(() => resetShedRetryProcessBudget());

  it('stops retrying once the process has spent its whole shed budget', async () => {
    resetShedRetryProcessBudget();
    const { sendRetryingShed } = await import('../doorway-client.js');
    const shed = () =>
      Promise.resolve(new Response(JSON.stringify({ status: 'catching-up', retryAfter: 15 }), { status: 503 }));
    let slept = 0;
    const sleep = async (ms: number) => {
      slept += ms;
    };
    for (let i = 0; i < 10; i++) {
      const r = await sendRetryingShed(shed, `w${i}`, sleep);
      expect(r.status).toBe(503);
    }
    expect(slept).toBeLessThanOrEqual(SHED_RETRY_PROCESS_BOUND_MS);
    const before = slept;
    await sendRetryingShed(shed, 'after-budget', sleep);
    expect(slept).toBe(before);
  });
});
