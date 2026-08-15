import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { StorageClient, StorageError } from '../src';

const baseUrl = 'https://storage.example.test';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

describe('StorageClient dataplane read routes', () => {
  const originalFetch = globalThis.fetch;
  const fetchMock = vi.fn<typeof fetch>();
  const client = new StorageClient({ baseUrl, hAppId: 'learning' });

  beforeEach(() => {
    fetchMock.mockReset();
    globalThis.fetch = fetchMock;
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it.each([
    {
      name: 'connected peers',
      invoke: () => client.listConnectedPeers(),
      path: '/p2p/peers',
      payload: { peers: [], total: 0 },
    },
    {
      name: 'projected peer statuses',
      invoke: () => client.listPeerStatuses(),
      path: '/api/v1/peer-statuses',
      payload: [],
    },
    {
      name: 'network posture',
      invoke: () => client.getNetworkPosture(),
      path: '/api/v1/network/posture',
      payload: {
        totalPeers: 4,
        activePeers: 3,
        stalePeers: 1,
        alwaysOnPeers: 2,
        householdsReciprocating: 2,
        computeAvailable: true,
        storagePressure: 0.42,
        computedAt: '2026-08-15T12:00:00Z',
      },
    },
  ])(
    'GETs $name from the existing base URL and returns JSON unchanged',
    async ({ invoke, path, payload }) => {
      fetchMock.mockResolvedValueOnce(jsonResponse(payload));

      await expect(invoke()).resolves.toEqual(payload);
      expect(fetchMock).toHaveBeenCalledOnce();

      const [url, init] = fetchMock.mock.calls[0];
      expect(url).toBe(`${baseUrl}${path}`);
      expect(init).toMatchObject({ method: 'GET' });
      expect(init?.body).toBeUndefined();
    },
  );

  it('preserves StorageError details from a failed dataplane read', async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse({ error: 'projection unavailable' }, 503),
    );

    const request = client.getNetworkPosture();

    await expect(request).rejects.toMatchObject({
      name: 'StorageError',
      statusCode: 503,
      responseBody: JSON.stringify({ error: 'projection unavailable' }),
    });
    await expect(request).rejects.toBeInstanceOf(StorageError);
  });
});
