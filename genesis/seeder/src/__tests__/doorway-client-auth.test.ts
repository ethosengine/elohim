/**
 * Tests for the seeder's Stage-A bearer authentication
 * (jenkins-seed-bearer-gate plan, Task 2).
 *
 * Covers:
 *  - readSeederCredentials: present-with-both / absent-skip
 *  - login(): POSTs /auth/login, stores the JWT, throws an actionable error
 *    naming the env vars on bad credentials
 *  - bearer-attach: after login(), a gated PUT /admin/seed/blob carries
 *    Authorization: Bearer <jwt>
 *  - no-creds-skip: without login(), no JWT bearer is attached (dev-mode path)
 *  - actionable 401/403 surfacing from the gated route
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { DoorwayClient, readSeederCredentials } from '../doorway-client.js';
import type { BlobMetadata } from '../blob-manager.js';

const META: BlobMetadata = { hash: 'sha256-abc', sizeBytes: 4, mimeType: 'text/plain' };

/** A fetch mock that records every call and returns scripted responses by path. */
function mockFetch(handlers: Record<string, () => Response>) {
  const calls: Array<{ url: string; method?: string; headers: Record<string, string> }> = [];
  const impl = vi.fn(async (url: string | URL, init?: RequestInit) => {
    const u = typeof url === 'string' ? url : url.toString();
    calls.push({
      url: u,
      method: init?.method,
      headers: (init?.headers as Record<string, string>) ?? {},
    });
    const path = new URL(u).pathname;
    const handler = handlers[path];
    if (!handler) {
      return new Response('not found', { status: 404 });
    }
    return handler();
  });
  return { impl, calls };
}

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

describe('readSeederCredentials', () => {
  it('returns creds when both env vars are set', () => {
    const creds = readSeederCredentials({
      SEED_DOORWAY_IDENTIFIER: 'jenkins-ci@alpha.elohim.host',
      SEED_DOORWAY_PASSWORD: 's3cret',
    } as NodeJS.ProcessEnv);
    expect(creds).toEqual({
      identifier: 'jenkins-ci@alpha.elohim.host',
      password: 's3cret',
    });
  });

  it('returns null when the password is missing (dev/local-stack path)', () => {
    expect(
      readSeederCredentials({ SEED_DOORWAY_IDENTIFIER: 'x' } as NodeJS.ProcessEnv)
    ).toBeNull();
  });

  it('returns null when the identifier is missing', () => {
    expect(
      readSeederCredentials({ SEED_DOORWAY_PASSWORD: 'x' } as NodeJS.ProcessEnv)
    ).toBeNull();
  });

  it('returns null when neither var is set', () => {
    expect(readSeederCredentials({} as NodeJS.ProcessEnv)).toBeNull();
  });
});

describe('DoorwayClient bearer authentication', () => {
  const originalFetch = globalThis.fetch;

  afterEach(() => {
    globalThis.fetch = originalFetch;
    vi.restoreAllMocks();
  });

  it('login() POSTs /auth/login and holds the JWT', async () => {
    const { impl, calls } = mockFetch({
      '/auth/login': () => jsonResponse({ token: 'jwt-xyz' }),
    });
    globalThis.fetch = impl as unknown as typeof fetch;

    const client = new DoorwayClient({ baseUrl: 'https://doorway-alpha.elohim.host' });
    await client.login({ identifier: 'jenkins-ci@alpha.elohim.host', password: 'pw' });

    expect(client.bearerToken).toBe('jwt-xyz');
    const loginCall = calls.find((c) => c.url.endsWith('/auth/login'));
    expect(loginCall?.method).toBe('POST');
  });

  it('attaches Authorization: Bearer <jwt> to the gated blob PUT after login', async () => {
    const { impl, calls } = mockFetch({
      '/auth/login': () => jsonResponse({ token: 'jwt-xyz' }),
      // blobExists checks (CID + hash) → 404 so we reach the PUT
      '/blob/sha256-abc': () => new Response('', { status: 404 }),
      '/admin/seed/blob': () => jsonResponse({ blake3Hash: 'b3-1' }),
    });
    // CID-based blobExists path hits a bafkrei* path — default 404 from mock.
    globalThis.fetch = impl as unknown as typeof fetch;

    const client = new DoorwayClient({ baseUrl: 'https://doorway-alpha.elohim.host' });
    await client.login({ identifier: 'jenkins-ci@alpha.elohim.host', password: 'pw' });

    const result = await client.pushBlob('sha256-abc', Buffer.from('data'), META);
    expect(result.success).toBe(true);

    const putCall = calls.find((c) => c.url.endsWith('/admin/seed/blob'));
    expect(putCall?.method).toBe('PUT');
    expect(putCall?.headers['Authorization']).toBe('Bearer jwt-xyz');
  });

  it('attaches NO JWT bearer when not authenticated (dev-mode skip path)', async () => {
    const { impl, calls } = mockFetch({
      '/admin/seed/blob': () => jsonResponse({ blake3Hash: 'b3-1' }),
    });
    globalThis.fetch = impl as unknown as typeof fetch;

    // No apiKey, no login() — the dev-mode doorway accepts unauthenticated seeding.
    const client = new DoorwayClient({ baseUrl: 'http://localhost:8888' });
    expect(client.bearerToken).toBeNull();

    await client.pushBlob('sha256-abc', Buffer.from('data'), META);

    const putCall = calls.find((c) => c.url.endsWith('/admin/seed/blob'));
    expect(putCall?.headers['Authorization']).toBeUndefined();
  });

  it('setBearerToken() lets a sibling client adopt a pre-obtained JWT', async () => {
    const { impl, calls } = mockFetch({
      '/admin/seed/blob': () => jsonResponse({ blake3Hash: 'b3-1' }),
    });
    globalThis.fetch = impl as unknown as typeof fetch;

    const client = new DoorwayClient({ baseUrl: 'https://doorway-alpha.elohim.host' });
    client.setBearerToken('shared-jwt');
    expect(client.bearerToken).toBe('shared-jwt');

    await client.pushBlob('sha256-abc', Buffer.from('data'), META);
    const putCall = calls.find((c) => c.url.endsWith('/admin/seed/blob'));
    expect(putCall?.headers['Authorization']).toBe('Bearer shared-jwt');
  });

  it('login() throws an actionable error naming the env vars on 401', async () => {
    const { impl } = mockFetch({
      '/auth/login': () => new Response('invalid credentials', { status: 401 }),
    });
    globalThis.fetch = impl as unknown as typeof fetch;

    const client = new DoorwayClient({ baseUrl: 'https://doorway-alpha.elohim.host' });
    await expect(
      client.login({ identifier: 'jenkins-ci@alpha.elohim.host', password: 'wrong' })
    ).rejects.toThrow(/SEED_DOORWAY_IDENTIFIER.*SEED_DOORWAY_PASSWORD/s);
    expect(client.bearerToken).toBeNull();
  });

  it('surfaces an actionable 401 from the gated blob PUT naming the env vars', async () => {
    const { impl } = mockFetch({
      '/admin/seed/blob': () => new Response('unauthorized', { status: 401 }),
    });
    globalThis.fetch = impl as unknown as typeof fetch;

    // No login — simulates a non-dev doorway rejecting an unauthenticated seed.
    const client = new DoorwayClient({ baseUrl: 'https://doorway-alpha.elohim.host' });
    const result = await client.pushBlob('sha256-abc', Buffer.from('data'), META);

    expect(result.success).toBe(false);
    expect(result.error).toMatch(/SEED_DOORWAY_IDENTIFIER/);
    expect(result.error).toMatch(/SEED_DOORWAY_PASSWORD/);
  });

  it('surfaces an actionable 403 (under-privileged) from the gated blob PUT', async () => {
    const { impl } = mockFetch({
      '/admin/seed/blob': () => new Response('forbidden', { status: 403 }),
    });
    globalThis.fetch = impl as unknown as typeof fetch;

    const client = new DoorwayClient({ baseUrl: 'https://doorway-alpha.elohim.host' });
    client.setBearerToken('jwt-but-not-admin');
    const result = await client.pushBlob('sha256-abc', Buffer.from('data'), META);

    expect(result.success).toBe(false);
    expect(result.error).toMatch(/under-privileged|Admin/);
  });
});
