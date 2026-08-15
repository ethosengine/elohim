import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { StorageClient, StorageError, streamSyncState } from '../src';
import type { StreamName } from '../src';

const baseUrl = 'https://storage.example.test';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

describe('DataplaneApi', () => {
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
      name: 'p2p status',
      invoke: () => client.dataplane.getP2pStatus(),
      path: '/p2p/status',
      payload: { peerId: 'abc', listenAddresses: [], connectedPeers: 0 },
    },
    {
      name: 'version',
      invoke: () => client.dataplane.getVersion(),
      path: '/version',
      payload: { commit: 'deadbeef', version: '1.2.3' },
    },
    {
      name: 'commitments (no query)',
      invoke: () => client.dataplane.listCommitments(),
      path: '/api/v1/commitments',
      payload: [],
    },
    {
      name: 'commitments (action + limit)',
      invoke: () => client.dataplane.listCommitments({ action: 'transfer', limit: 5 }),
      path: '/api/v1/commitments?action=transfer&limit=5',
      payload: [],
    },
    {
      name: 'economic events (no query)',
      invoke: () => client.dataplane.listEconomicEvents(),
      path: '/api/v1/economic-events',
      payload: [],
    },
    {
      name: 'economic events (action + after + limit)',
      invoke: () =>
        client.dataplane.listEconomicEvents({
          action: 'produce',
          after: '2026-08-01T00:00:00Z',
          limit: 10,
        }),
      path: '/api/v1/economic-events?action=produce&after=2026-08-01T00%3A00%3A00Z&limit=10',
      payload: [],
    },
    {
      name: 'projector status',
      invoke: () => client.dataplane.getProjectorStatus(),
      path: '/api/v1/status/projector',
      payload: { cursors: [], lag: [] },
    },
    {
      name: 'connected peers (delegated)',
      invoke: () => client.dataplane.listConnectedPeers(),
      path: '/p2p/peers',
      payload: { peers: [], total: 0 },
    },
    {
      name: 'peer statuses (delegated)',
      invoke: () => client.dataplane.listPeerStatuses(),
      path: '/api/v1/peer-statuses',
      payload: [],
    },
    {
      name: 'network posture (delegated)',
      invoke: () => client.dataplane.getNetworkPosture(),
      path: '/api/v1/network/posture',
      payload: { totalPeers: 1, activePeers: 1, stalePeers: 0, alwaysOnPeers: 1 },
    },
  ])('GETs $name from the configured base URL', async ({ invoke, path, payload }) => {
    fetchMock.mockResolvedValueOnce(jsonResponse(payload));

    await expect(invoke()).resolves.toEqual(payload);
    expect(fetchMock).toHaveBeenCalledOnce();

    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe(`${baseUrl}${path}`);
    expect(init).toMatchObject({ method: 'GET' });
    expect(init?.body).toBeUndefined();
  });

  it.each([
    { wire: { filesystem_count: 42 }, expected: 42 },
    { wire: { filesystemCount: 7 }, expected: 7 },
    { wire: {}, expected: null },
    { wire: { filesystem_count: 'not-a-number' }, expected: null },
  ])('normalizes inventory-parity filesystem count ($wire)', async ({ wire, expected }) => {
    fetchMock.mockResolvedValueOnce(jsonResponse(wire));

    await expect(client.dataplane.getInventoryParity()).resolves.toEqual({
      filesystemCount: expected,
    });

    const [url] = fetchMock.mock.calls[0];
    expect(url).toBe(`${baseUrl}/api/v1/diagnostics/inventory-parity`);
  });

  it('preserves StorageError details from a failed dataplane read', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse({ error: 'projection unavailable' }, 503));

    const request = client.dataplane.getProjectorStatus();

    await expect(request).rejects.toMatchObject({
      name: 'StorageError',
      statusCode: 503,
    });
    await expect(request).rejects.toBeInstanceOf(StorageError);
  });

  describe('getBlobStatus', () => {
    it.each([200, 404, 410, 500])(
      'issues a real GET (never HEAD) against /blob/{hash} and returns status %i without throwing',
      async (status) => {
        fetchMock.mockResolvedValueOnce(
          new Response('blob-bytes-or-error-body', { status }),
        );

        await expect(client.dataplane.getBlobStatus('sha256-deadbeef')).resolves.toBe(status);

        const [url, init] = fetchMock.mock.calls[0];
        expect(url).toBe(`${baseUrl}/blob/sha256-deadbeef`);
        expect(init?.method).toBe('GET');
        expect(init?.method).not.toBe('HEAD');
      },
    );

    it('URL-encodes the hash/CID', async () => {
      fetchMock.mockResolvedValueOnce(new Response('', { status: 200 }));

      await client.dataplane.getBlobStatus('bafkrei/with slash');

      const [url] = fetchMock.mock.calls[0];
      expect(url).toBe(`${baseUrl}/blob/${encodeURIComponent('bafkrei/with slash')}`);
    });

    it('throws on a network-level failure (not a status code)', async () => {
      fetchMock.mockRejectedValueOnce(new TypeError('network down'));

      await expect(client.dataplane.getBlobStatus('sha256-deadbeef')).rejects.toBeInstanceOf(
        StorageError,
      );
    });
  });
});

describe('streamSyncState', () => {
  it.each([
    { name: 'missing status', status: undefined, stream: 'pull' as StreamName, expected: 'not-computable' },
    { name: 'null status', status: null, stream: 'pull' as StreamName, expected: 'not-computable' },
    {
      name: 'missing stream on status',
      status: {},
      stream: 'replication' as StreamName,
      expected: 'not-computable',
    },
    {
      name: 'null stream value',
      status: { pull: null },
      stream: 'pull' as StreamName,
      expected: 'not-computable',
    },
    {
      name: 'pull total 0 (camelCase) -> idle regardless of caughtUp',
      status: { pull: { total: 0, caughtUp: false } },
      stream: 'pull' as StreamName,
      expected: 'idle',
    },
    {
      name: 'pull total 0 (missing caughtUp) -> idle',
      status: { pull: { total: 0 } },
      stream: 'pull' as StreamName,
      expected: 'idle',
    },
    {
      name: 'pull total > 0, caughtUp camelCase true -> caught-up',
      status: { pull: { total: 3, caughtUp: true } },
      stream: 'pull' as StreamName,
      expected: 'caught-up',
    },
    {
      name: 'pull total > 0, caught_up snake_case true -> caught-up',
      status: { pull: { total: 3, caught_up: true } },
      stream: 'pull' as StreamName,
      expected: 'caught-up',
    },
    {
      name: 'pull total > 0, caughtUp camelCase false -> behind',
      status: { pull: { total: 3, caughtUp: false } },
      stream: 'pull' as StreamName,
      expected: 'behind',
    },
    {
      name: 'pull total > 0, caught_up snake_case false -> behind',
      status: { pull: { total: 3, caught_up: false } },
      stream: 'pull' as StreamName,
      expected: 'behind',
    },
    {
      name: 'replication caughtUp camelCase true -> caught-up',
      status: { replication: { caughtUp: true } },
      stream: 'replication' as StreamName,
      expected: 'caught-up',
    },
    {
      name: 'replication caught_up snake_case false -> behind',
      status: { replication: { caught_up: false } },
      stream: 'replication' as StreamName,
      expected: 'behind',
    },
    {
      name: 'projectionReconcile via camelCase key, camelCase field -> caught-up',
      status: { projectionReconcile: { caughtUp: true } },
      stream: 'projectionReconcile' as StreamName,
      expected: 'caught-up',
    },
    {
      name: 'projectionReconcile via snake_case key, snake_case field -> behind',
      status: { projection_reconcile: { caught_up: false } },
      stream: 'projectionReconcile' as StreamName,
      expected: 'behind',
    },
    {
      name: 'caughtUp present but non-boolean -> not-computable',
      status: { replication: { caughtUp: 'yes' } },
      stream: 'replication' as StreamName,
      expected: 'not-computable',
    },
  ])('$name', ({ status, stream, expected }) => {
    expect(streamSyncState(status, stream)).toBe(expected);
  });
});
