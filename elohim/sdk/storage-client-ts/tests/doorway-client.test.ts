import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { DoorwayClient, StorageError } from '../src';

const baseUrl = 'https://doorway.example.test';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

describe('DoorwayClient', () => {
  const originalFetch = globalThis.fetch;
  const fetchMock = vi.fn<typeof fetch>();
  const doorway = new DoorwayClient({ baseUrl });

  beforeEach(() => {
    fetchMock.mockReset();
    globalThis.fetch = fetchMock;
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it('GETs /health', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse({ status: 'ok' }));

    await expect(doorway.getHealth()).resolves.toEqual({ status: 'ok' });

    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe(`${baseUrl}/health`);
    expect(init).toMatchObject({ method: 'GET' });
  });

  it('GETs /api/v1/federation/doorways', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse({ doorways: [{ status: 'online' }] }));

    await expect(doorway.listFederationDoorways()).resolves.toEqual({
      doorways: [{ status: 'online' }],
    });

    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe(`${baseUrl}/api/v1/federation/doorways`);
    expect(init).toMatchObject({ method: 'GET' });
  });

  it('GETs /api/v1/federation/p2p-peers', async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse({ total: 1, peers: [{ capabilities: [] }] }),
    );

    await expect(doorway.listFederationP2pPeers()).resolves.toEqual({
      total: 1,
      peers: [{ capabilities: [] }],
    });

    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe(`${baseUrl}/api/v1/federation/p2p-peers`);
    expect(init).toMatchObject({ method: 'GET' });
  });

  it('throws StorageError on a non-2xx federation read', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse({ error: 'unavailable' }, 502));

    await expect(doorway.listFederationDoorways()).rejects.toMatchObject({
      name: 'StorageError',
      statusCode: 502,
    });
    await expect(doorway.listFederationDoorways()).rejects.toBeInstanceOf(StorageError);
  });

  describe('seedBlob', () => {
    const data = new Uint8Array([1, 2, 3]);

    it.each([
      { status: 200, expected: { ok: true, alreadyPresent: false } },
      { status: 201, expected: { ok: true, alreadyPresent: false } },
      { status: 409, expected: { ok: true, alreadyPresent: true } },
      { status: 500, expected: { ok: false } },
    ])('treats HTTP $status as $expected', async ({ status, expected }) => {
      fetchMock.mockResolvedValueOnce(new Response('server said no', { status }));

      const result = await doorway.seedBlob({ hash: 'sha256-abc', data });

      expect(result).toMatchObject({ status, ...expected });
      if (status === 500) {
        expect(result.errorBody).toBe('server said no');
      } else {
        expect(result.errorBody).toBeUndefined();
      }
    });

    it('truncates errorBody to ~300 chars', async () => {
      const longBody = 'x'.repeat(1000);
      fetchMock.mockResolvedValueOnce(new Response(longBody, { status: 500 }));

      const result = await doorway.seedBlob({ hash: 'sha256-abc', data });

      expect(result.ok).toBe(false);
      expect(result.errorBody).toHaveLength(300);
    });

    it('sends X-Blob-Hash and Content-Type, and OMITS Authorization when no token is given', async () => {
      fetchMock.mockResolvedValueOnce(new Response('', { status: 200 }));

      await doorway.seedBlob({ hash: 'sha256-abc', data });

      const [url, init] = fetchMock.mock.calls[0];
      expect(url).toBe(`${baseUrl}/admin/seed/blob`);
      expect(init?.method).toBe('PUT');
      const headers = init?.headers as Record<string, string>;
      expect(headers['X-Blob-Hash']).toBe('sha256-abc');
      expect(headers['Content-Type']).toBe('application/octet-stream');
      expect(headers['Authorization']).toBeUndefined();
      expect('Authorization' in headers).toBe(false);
    });

    it('sends Authorization: Bearer <token> when a token is given', async () => {
      fetchMock.mockResolvedValueOnce(new Response('', { status: 200 }));

      await doorway.seedBlob({ hash: 'sha256-abc', data, token: 'tok-123' });

      const [, init] = fetchMock.mock.calls[0];
      const headers = init?.headers as Record<string, string>;
      expect(headers['Authorization']).toBe('Bearer tok-123');
    });

    it('never sends Authorization: Bearer  with an empty value', async () => {
      fetchMock.mockResolvedValueOnce(new Response('', { status: 200 }));

      await doorway.seedBlob({ hash: 'sha256-abc', data, token: '' });

      const [, init] = fetchMock.mock.calls[0];
      const headers = init?.headers as Record<string, string>;
      expect(headers['Authorization']).toBeUndefined();
    });
  });
});
