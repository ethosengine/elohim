import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { DataplaneFleet } from '../src';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

/** Exercises a peer's client with a mocked fetch and returns the requested URL. */
async function calledUrl(client: { getHeads: (docId: string) => Promise<unknown> }): Promise<string> {
  const originalFetch = globalThis.fetch;
  const fetchMock = vi.fn<typeof fetch>().mockResolvedValueOnce(
    jsonResponse({ hAppId: 'elohim', doc_id: 'x', heads: [] }),
  );
  globalThis.fetch = fetchMock;
  try {
    await client.getHeads('x');
  } finally {
    globalThis.fetch = originalFetch;
  }
  return fetchMock.mock.calls[0][0] as string;
}

describe('DataplaneFleet.fromCsv', () => {
  const originalFetch = globalThis.fetch;
  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it('parses a normal multi-peer CSV, in declared order', () => {
    const fleet = DataplaneFleet.fromCsv(
      'matthew=storage-a.example:8090,adam=storage-b.example:8090',
    );

    expect(fleet.names()).toEqual(['matthew', 'adam']);
    expect(fleet.peers().map((p) => p.name)).toEqual(['matthew', 'adam']);
    expect(fleet.peers()).toHaveLength(2);
    expect(fleet.peer('matthew')).toBeDefined();
    expect(fleet.peer('adam')).toBeDefined();
  });

  it('prepends http:// to a scheme-less host:port', async () => {
    const fleet = DataplaneFleet.fromCsv('one=storage-a.example:8090');
    const url = await calledUrl(fleet.peer('one')!);
    expect(url).toBe('http://storage-a.example:8090/sync/v1/elohim/docs/x/heads');
  });

  it('leaves an already-schemed entry untouched', async () => {
    const fleet = DataplaneFleet.fromCsv('secure=https://storage-c.example:8443');
    const url = await calledUrl(fleet.peer('secure')!);
    expect(url).toBe('https://storage-c.example:8443/sync/v1/elohim/docs/x/heads');
  });

  it('skips empty segments (stray commas / whitespace)', () => {
    const fleet = DataplaneFleet.fromCsv(
      'matthew=storage-a.example:8090,,  ,adam=storage-b.example:8090',
    );

    expect(fleet.names()).toEqual(['matthew', 'adam']);
  });

  it('skips a segment missing "="', () => {
    const fleet = DataplaneFleet.fromCsv(
      'matthew=storage-a.example:8090,not-a-kv-pair,adam=storage-b.example:8090',
    );

    expect(fleet.names()).toEqual(['matthew', 'adam']);
  });

  it('skips a segment with an empty name or empty host', () => {
    const fleet = DataplaneFleet.fromCsv(
      '=storage-a.example:8090,matthew=,adam=storage-b.example:8090',
    );

    expect(fleet.names()).toEqual(['adam']);
  });

  it('returns an empty fleet for an empty string', () => {
    const fleet = DataplaneFleet.fromCsv('');

    expect(fleet.names()).toEqual([]);
    expect(fleet.peers()).toEqual([]);
    expect(fleet.peer('anyone')).toBeUndefined();
  });

  it('defaults hAppId to "elohim" and honors an explicit override', async () => {
    const defaulted = DataplaneFleet.fromCsv('matthew=storage-a.example:8090');
    const defaultedUrl = await calledUrl(defaulted.peer('matthew')!);
    expect(defaultedUrl).toContain('/sync/v1/elohim/docs/');

    const overridden = DataplaneFleet.fromCsv('matthew=storage-a.example:8090', {
      hAppId: 'lamad',
    });
    const overriddenUrl = await calledUrl(overridden.peer('matthew')!);
    expect(overriddenUrl).toContain('/sync/v1/lamad/docs/');
  });

  it('preserves insertion order across peers() and names()', () => {
    const fleet = DataplaneFleet.fromCsv('c=host-c:1,a=host-a:2,b=host-b:3');

    expect(fleet.names()).toEqual(['c', 'a', 'b']);
    expect(fleet.peers().map((p) => p.name)).toEqual(['c', 'a', 'b']);
  });
});
