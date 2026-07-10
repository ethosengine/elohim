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
import { DoorwayClient } from '../doorway-client.js';

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
