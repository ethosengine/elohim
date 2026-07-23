/**
 * elohim-client multi-host failover ("logical anycast")
 *
 * Spec: genesis/docs/superpowers/specs/2026-07-16-dual-wan-utility-plane-failover-design.md §3a
 *
 * Covers the private `fetchWithFailover` helper as exercised through the
 * public read/write surface of `ElohimClient` (browser mode). Mocks
 * `global.fetch` directly rather than a network layer.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { ElohimClient } from './elohim-client';
import type { BrowserMode, ContentReadable } from './types';

interface TestContent extends ContentReadable {
  id: string;
  title?: string;
}

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  });
}

function networkError(): Promise<never> {
  return Promise.reject(new TypeError('fetch failed'));
}

describe('ElohimClient multi-host failover', () => {
  const PRIMARY = 'https://primary.example.com';
  const FALLBACK = 'https://fallback.example.com';

  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  function makeClient(fallbacks?: string[]): ElohimClient {
    const mode: BrowserMode = {
      type: 'browser',
      doorway: { url: PRIMARY, fallbacks },
    };
    return new ElohimClient({ mode });
  }

  it('(a) primary success — no fallback attempted', async () => {
    const client = makeClient([FALLBACK]);
    fetchMock.mockResolvedValueOnce(jsonResponse({ id: 'x', title: 'hi' }));

    const result = await client.get<TestContent>('content', 'x');

    expect(result).toEqual({ id: 'x', title: 'hi' });
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls[0][0]).toBe(`${PRIMARY}/db/content/x`);
  });

  it('(b) primary network-reject — fallback used and result returned', async () => {
    const client = makeClient([FALLBACK]);
    fetchMock.mockImplementationOnce(networkError);
    fetchMock.mockResolvedValueOnce(jsonResponse({ id: 'x', title: 'from-fallback' }));

    const result = await client.get<TestContent>('content', 'x');

    expect(result).toEqual({ id: 'x', title: 'from-fallback' });
    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(fetchMock.mock.calls[0][0]).toBe(`${PRIMARY}/db/content/x`);
    expect(fetchMock.mock.calls[1][0]).toBe(`${FALLBACK}/db/content/x`);
  });

  it('(c) primary HTTP 500 — returned as-is, no failover', async () => {
    const client = makeClient([FALLBACK]);
    fetchMock.mockResolvedValueOnce(new Response('boom', { status: 500 }));

    await expect(client.get<TestContent>('content', 'x')).rejects.toThrow('HTTP 500');

    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it('(d) sticky: after a failover, next call tries the fallback host first', async () => {
    const client = makeClient([FALLBACK]);

    // First call: primary fails, fallback succeeds.
    fetchMock.mockImplementationOnce(networkError);
    fetchMock.mockResolvedValueOnce(jsonResponse({ id: 'a' }));
    await client.get<TestContent>('content', 'a');

    fetchMock.mockClear();

    // Second call: should try the fallback FIRST.
    fetchMock.mockResolvedValueOnce(jsonResponse({ id: 'b' }));
    const result = await client.get<TestContent>('content', 'b');

    expect(result).toEqual({ id: 'b' });
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls[0][0]).toBe(`${FALLBACK}/db/content/b`);
  });

  it('(e) write call on network failure: error propagates, no duplicate attempt, but stickiness advances', async () => {
    const client = makeClient([FALLBACK]);
    fetchMock.mockImplementationOnce(networkError);

    await client.save('content', { id: 'w', title: 'queued' });
    await expect(client.flush()).rejects.toThrow(TypeError);

    // Only the primary was attempted — no auto-retry on a write.
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls[0][0]).toBe(`${PRIMARY}/db/content/bulk`);

    // Stickiness still advanced: the NEXT call (any call) tries the fallback first.
    fetchMock.mockClear();
    fetchMock.mockResolvedValueOnce(jsonResponse({ id: 'r' }));
    await client.get<TestContent>('content', 'r');

    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls[0][0]).toBe(`${FALLBACK}/db/content/r`);
  });

  it('(f) no fallbacks configured — identical to today: single attempt, error propagates', async () => {
    const client = makeClient(undefined);
    fetchMock.mockImplementationOnce(networkError);

    await expect(client.get<TestContent>('content', 'x')).rejects.toThrow(TypeError);
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls[0][0]).toBe(`${PRIMARY}/db/content/x`);
  });

  it('(g) caller AbortSignal aborts mid-flight — rejects immediately, no failover, stickiness unchanged', async () => {
    const client = makeClient([FALLBACK]);
    const ac = new AbortController();

    // Mimics real fetch (verified against Node's native fetch/undici in this
    // vitest env: aborting a caller signal rejects with a DOMException named
    // 'AbortError') — the promise never settles until the signal aborts.
    fetchMock.mockImplementationOnce((_url: string, init?: RequestInit) => {
      return new Promise((_resolve, reject) => {
        init?.signal?.addEventListener('abort', () => {
          reject(new DOMException('This operation was aborted', 'AbortError'));
        });
      });
    });

    const pending = client.fetch('/db/content/x', { signal: ac.signal });
    ac.abort();

    await expect(pending).rejects.toMatchObject({ name: 'AbortError' });

    // No failover: fetch was called exactly once, only against the primary.
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls[0][0]).toBe(`${PRIMARY}/db/content/x`);

    // Stickiness unchanged: the NEXT call still tries the primary first.
    fetchMock.mockClear();
    fetchMock.mockResolvedValueOnce(jsonResponse({ id: 'y' }));
    const result = await client.get<TestContent>('content', 'y');

    expect(result).toEqual({ id: 'y' });
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls[0][0]).toBe(`${PRIMARY}/db/content/y`);
  });

  it('(h) TimeoutError (our own per-attempt AbortSignal.timeout) still fails over', async () => {
    const client = makeClient([FALLBACK]);
    fetchMock.mockImplementationOnce(() =>
      Promise.reject(new DOMException('The operation timed out.', 'TimeoutError'))
    );
    fetchMock.mockResolvedValueOnce(jsonResponse({ id: 'x', title: 'from-fallback' }));

    const result = await client.get<TestContent>('content', 'x');

    expect(result).toEqual({ id: 'x', title: 'from-fallback' });
    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(fetchMock.mock.calls[0][0]).toBe(`${PRIMARY}/db/content/x`);
    expect(fetchMock.mock.calls[1][0]).toBe(`${FALLBACK}/db/content/x`);
  });

  it('(i) sticky-on-fallback then fallback network-fails — ladder still includes the primary (fallback, then primary)', async () => {
    const client = makeClient([FALLBACK]);

    // First call: primary fails, fallback succeeds — sticky becomes FALLBACK.
    fetchMock.mockImplementationOnce(networkError);
    fetchMock.mockResolvedValueOnce(jsonResponse({ id: 'a' }));
    await client.get<TestContent>('content', 'a');

    fetchMock.mockClear();

    // Second call: fallback (now sticky) fails too — the ladder must still
    // include the primary as the next candidate: [sticky, primary, ...fallbacks]
    // deduped to [FALLBACK, PRIMARY].
    fetchMock.mockImplementationOnce(networkError);
    fetchMock.mockResolvedValueOnce(jsonResponse({ id: 'b' }));
    const result = await client.get<TestContent>('content', 'b');

    expect(result).toEqual({ id: 'b' });
    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(fetchMock.mock.calls[0][0]).toBe(`${FALLBACK}/db/content/b`);
    expect(fetchMock.mock.calls[1][0]).toBe(`${PRIMARY}/db/content/b`);
  });

  it('(k) GET gets the default 8s per-attempt AbortSignal; bulk-write POST gets none', async () => {
    const client = makeClient([FALLBACK]);

    // GET: should carry an injected AbortSignal (failover-probing default).
    fetchMock.mockResolvedValueOnce(jsonResponse({ id: 'x' }));
    await client.get<TestContent>('content', 'x');

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const getInit = fetchMock.mock.calls[0][1] as RequestInit;
    expect(getInit.signal).toBeInstanceOf(AbortSignal);

    fetchMock.mockClear();

    // Bulk-write POST (flushWriteBuffer -> flushToProjection): must NOT get
    // an injected timeout — an 8s abort could kill an in-flight,
    // non-idempotent write that would otherwise have succeeded.
    fetchMock.mockResolvedValueOnce(jsonResponse({ ok: true }));
    await client.save('content', { id: 'w', title: 'queued' });
    await client.flush();

    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls[0][0]).toBe(`${PRIMARY}/db/content/bulk`);
    const postInit = fetchMock.mock.calls[0][1] as RequestInit;
    expect(postInit.signal).toBeUndefined();
  });

  it('(j) fallback configured with a trailing slash — no double slash in the request URL, dedupe still works', async () => {
    const client = makeClient([`${FALLBACK}/`]);

    // Primary fails, trailing-slash fallback succeeds — URL must not double the slash.
    fetchMock.mockImplementationOnce(networkError);
    fetchMock.mockResolvedValueOnce(jsonResponse({ id: 'x', title: 'from-fallback' }));

    const result = await client.get<TestContent>('content', 'x');

    expect(result).toEqual({ id: 'x', title: 'from-fallback' });
    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(fetchMock.mock.calls[1][0]).toBe(`${FALLBACK}/db/content/x`);
    expect(fetchMock.mock.calls[1][0]).not.toMatch(/[^:]\/\/db/);

    // Sticky is now the normalized fallback host. A subsequent call's
    // candidate ladder dedupes the (normalized) sticky host against the
    // (trailing-slash-configured, but normalized) fallback entry — only two
    // distinct hosts are ever tried, never three.
    fetchMock.mockClear();
    fetchMock.mockImplementationOnce(networkError);
    fetchMock.mockResolvedValueOnce(jsonResponse({ id: 'y' }));
    const result2 = await client.get<TestContent>('content', 'y');

    expect(result2).toEqual({ id: 'y' });
    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(fetchMock.mock.calls[0][0]).toBe(`${FALLBACK}/db/content/y`);
    expect(fetchMock.mock.calls[1][0]).toBe(`${PRIMARY}/db/content/y`);
  });
});
