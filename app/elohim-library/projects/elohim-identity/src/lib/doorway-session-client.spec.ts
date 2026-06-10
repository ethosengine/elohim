import { describe, it, expect, vi, beforeEach } from 'vitest';

/**
 * DoorwaySessionClient tests (arc plan 1.1)
 *
 * Framework-free session client for the doorway auth surface. The wire
 * contract under test mirrors doorway/doorway-service/src/routes/auth_routes.rs:
 * camelCase arrives on the wire and MUST pass through untouched — no case
 * conversion, no toWire/fromWire, no JSON.parse of nested fields.
 */

import {
  DoorwaySessionClient,
  DoorwaySessionError,
  InMemorySessionTokenStore,
  type AuthResponse,
  type ExchangeSessionResponse,
  type MeResponse,
  type SessionTokenResponse,
  type StoredSession,
  type SuccessResponse,
} from './doorway-session-client';

const BASE_URL = 'http://doorway.test:8888';

/** Unix-seconds timestamp `offsetSeconds` from now (wire uses seconds, not ms). */
function unixSeconds(offsetSeconds: number): number {
  return Math.floor(Date.now() / 1000) + offsetSeconds;
}

/** Minimal Response stand-in so Node tests need no real fetch/Response. */
function jsonResponse(status: number, body: unknown): Response {
  const text = JSON.stringify(body);
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => JSON.parse(text),
    text: async () => text,
  } as unknown as Response;
}

/** Non-JSON Response stand-in (proxy error pages, etc.). */
function textResponse(status: number, text: string): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => JSON.parse(text),
    text: async () => text,
  } as unknown as Response;
}

function makeAuthResponse(overrides: Partial<AuthResponse> = {}): AuthResponse {
  return {
    token: 'jwt-token-abc',
    humanId: 'human-123',
    agentPubKey: 'uhCAk-agent-key',
    identifier: 'terrance@example.com',
    expiresAt: unixSeconds(3600),
    doorwayId: 'alpha',
    doorwayUrl: 'https://alpha.elohim.host',
    installedAppId: 'elohim-app-terrance',
    ...overrides,
  };
}

describe('DoorwaySessionClient', () => {
  let fetchMock: ReturnType<typeof vi.fn>;
  let client: DoorwaySessionClient;

  beforeEach(() => {
    fetchMock = vi.fn();
    client = new DoorwaySessionClient({
      baseUrl: BASE_URL,
      fetchImpl: fetchMock as unknown as typeof fetch,
    });
  });

  function lastRequest(): { url: string; init: RequestInit } {
    const call = fetchMock.mock.calls[fetchMock.mock.calls.length - 1];
    return { url: call[0] as string, init: (call[1] ?? {}) as RequestInit };
  }

  // ===========================================================================
  // login
  // ===========================================================================

  describe('login', () => {
    it('POSTs identifier/password to /auth/login and returns the wire response', async () => {
      const wire = makeAuthResponse();
      fetchMock.mockResolvedValueOnce(jsonResponse(200, wire));

      const result = await client.login({ identifier: 'terrance@example.com', password: 'pw' });

      const { url, init } = lastRequest();
      expect(url).toBe(`${BASE_URL}/auth/login`);
      expect(init.method).toBe('POST');
      expect((init.headers as Record<string, string>)['Content-Type']).toBe('application/json');
      expect(JSON.parse(init.body as string)).toEqual({
        identifier: 'terrance@example.com',
        password: 'pw',
      });
      expect(result).toEqual(wire);
    });

    it('does not send an Authorization header on login', async () => {
      fetchMock.mockResolvedValueOnce(jsonResponse(200, makeAuthResponse()));

      await client.login({ identifier: 'a@b', password: 'pw' });

      const { init } = lastRequest();
      expect((init.headers as Record<string, string>)['Authorization']).toBeUndefined();
    });

    it('stores the session so restoreSession() returns it', async () => {
      const wire = makeAuthResponse();
      fetchMock.mockResolvedValueOnce(jsonResponse(200, wire));

      await client.login({ identifier: 'terrance@example.com', password: 'pw' });

      const stored = client.restoreSession();
      expect(stored).not.toBeNull();
      expect(stored?.token).toBe(wire.token);
      expect(stored?.humanId).toBe(wire.humanId);
      expect(stored?.agentPubKey).toBe(wire.agentPubKey);
      expect(stored?.identifier).toBe(wire.identifier);
      expect(stored?.expiresAt).toBe(wire.expiresAt);
      expect(stored?.installedAppId).toBe(wire.installedAppId);
    });
  });

  // ===========================================================================
  // register
  // ===========================================================================

  describe('register', () => {
    it('POSTs the camelCase body to /auth/register verbatim (no case conversion)', async () => {
      const wire = makeAuthResponse({ profile: undefined });
      fetchMock.mockResolvedValueOnce(jsonResponse(201, wire));

      const req = {
        identifier: 'new@example.com',
        password: 'pw',
        displayName: 'New Human',
        agencyPhase: 'hosted',
        affinities: ['trust', 'learning'],
        profileReach: 'public',
      };
      const result = await client.register(req);

      const { url, init } = lastRequest();
      expect(url).toBe(`${BASE_URL}/auth/register`);
      expect(init.method).toBe('POST');
      // Request body keys are exactly what the caller passed — camelCase through.
      expect(JSON.parse(init.body as string)).toEqual(req);
      expect(result).toEqual(wire);
    });

    it('accepts a 201 Created and stores the session', async () => {
      const wire = makeAuthResponse();
      fetchMock.mockResolvedValueOnce(jsonResponse(201, wire));

      await client.register({ identifier: 'new@example.com', password: 'pw' });

      expect(client.restoreSession()?.token).toBe(wire.token);
    });
  });

  // ===========================================================================
  // logout
  // ===========================================================================

  describe('logout', () => {
    it('POSTs to /auth/logout with the bearer token and clears the stored session', async () => {
      fetchMock.mockResolvedValueOnce(jsonResponse(200, makeAuthResponse()));
      await client.login({ identifier: 'a@b', password: 'pw' });

      const success: SuccessResponse = { success: true, message: 'Logged out successfully' };
      fetchMock.mockResolvedValueOnce(jsonResponse(200, success));

      const result = await client.logout();

      const { url, init } = lastRequest();
      expect(url).toBe(`${BASE_URL}/auth/logout`);
      expect(init.method).toBe('POST');
      expect((init.headers as Record<string, string>)['Authorization']).toBe(
        'Bearer jwt-token-abc'
      );
      expect(result).toEqual(success);
      expect(client.restoreSession()).toBeNull();
    });

    it('clears the stored session even when the server errors', async () => {
      fetchMock.mockResolvedValueOnce(jsonResponse(200, makeAuthResponse()));
      await client.login({ identifier: 'a@b', password: 'pw' });

      fetchMock.mockResolvedValueOnce(jsonResponse(500, { error: 'boom' }));

      await expect(client.logout()).rejects.toBeInstanceOf(DoorwaySessionError);
      expect(client.restoreSession()).toBeNull();
    });

    it('clears the stored session even when fetchImpl network fails', async () => {
      fetchMock.mockResolvedValueOnce(jsonResponse(200, makeAuthResponse()));
      await client.login({ identifier: 'a@b', password: 'pw' });

      fetchMock.mockRejectedValueOnce(new TypeError('fetch failed'));

      await expect(client.logout()).rejects.toThrow(TypeError);
      expect(client.restoreSession()).toBeNull();
    });
  });

  // ===========================================================================
  // me
  // ===========================================================================

  describe('me', () => {
    it('GETs /auth/me with the bearer token and returns the wire response untouched', async () => {
      fetchMock.mockResolvedValueOnce(jsonResponse(200, makeAuthResponse()));
      await client.login({ identifier: 'a@b', password: 'pw' });

      const wire: MeResponse = {
        humanId: 'human-123',
        agentPubKey: 'uhCAk-agent-key',
        identifier: 'terrance@example.com',
        permissionLevel: 'member',
        doorwayId: 'alpha',
        doorwayUrl: 'https://alpha.elohim.host',
        authenticated: true,
        trustMode: 'doorway-host',
        authority: { label: 'alpha.elohim.host', id: 'alpha' },
      };
      fetchMock.mockResolvedValueOnce(jsonResponse(200, wire));

      const result = await client.me();

      const { url, init } = lastRequest();
      expect(url).toBe(`${BASE_URL}/auth/me`);
      expect(init.method).toBe('GET');
      expect((init.headers as Record<string, string>)['Authorization']).toBe(
        'Bearer jwt-token-abc'
      );
      expect(result).toEqual(wire);
      // Nested authority object arrives pre-parsed and untouched.
      expect(result.authority.label).toBe('alpha.elohim.host');
    });

    it('omits the Authorization header when no session is stored (server owns the 401)', async () => {
      fetchMock.mockResolvedValueOnce(jsonResponse(401, { error: 'No token provided' }));

      await expect(client.me()).rejects.toBeInstanceOf(DoorwaySessionError);

      const { init } = lastRequest();
      expect((init.headers as Record<string, string>)['Authorization']).toBeUndefined();
    });
  });

  // ===========================================================================
  // refresh
  // ===========================================================================

  describe('refresh', () => {
    it('POSTs /auth/refresh with the bearer token and replaces the stored session', async () => {
      fetchMock.mockResolvedValueOnce(jsonResponse(200, makeAuthResponse()));
      await client.login({ identifier: 'a@b', password: 'pw' });

      const refreshed = makeAuthResponse({
        token: 'jwt-token-refreshed',
        expiresAt: unixSeconds(7200),
      });
      fetchMock.mockResolvedValueOnce(jsonResponse(200, refreshed));

      const result = await client.refresh();

      const { url, init } = lastRequest();
      expect(url).toBe(`${BASE_URL}/auth/refresh`);
      expect(init.method).toBe('POST');
      expect((init.headers as Record<string, string>)['Authorization']).toBe(
        'Bearer jwt-token-abc'
      );
      expect(result).toEqual(refreshed);
      expect(client.restoreSession()?.token).toBe('jwt-token-refreshed');
      expect(client.restoreSession()?.expiresAt).toBe(refreshed.expiresAt);
    });
  });

  // ===========================================================================
  // session token issue + exchange (cross-app handoff)
  // ===========================================================================

  describe('requestSessionToken', () => {
    it('GETs /auth/session-token with the bearer token', async () => {
      fetchMock.mockResolvedValueOnce(jsonResponse(200, makeAuthResponse()));
      await client.login({ identifier: 'a@b', password: 'pw' });

      const wire: SessionTokenResponse = {
        sessionToken: 'transfer-uuid-1',
        expiresAt: unixSeconds(60),
      };
      fetchMock.mockResolvedValueOnce(jsonResponse(200, wire));

      const result = await client.requestSessionToken();

      const { url, init } = lastRequest();
      expect(url).toBe(`${BASE_URL}/auth/session-token`);
      expect(init.method).toBe('GET');
      expect((init.headers as Record<string, string>)['Authorization']).toBe(
        'Bearer jwt-token-abc'
      );
      expect(result).toEqual(wire);
    });
  });

  describe('exchangeSessionToken', () => {
    it('GETs /auth/exchange-session with the session_token query param and stores the session', async () => {
      const wire: ExchangeSessionResponse = {
        token: 'jwt-from-exchange',
        humanId: 'human-123',
        agentPubKey: 'uhCAk-agent-key',
        identifier: 'terrance@example.com',
        expiresAt: unixSeconds(3600),
        doorwayId: 'alpha',
        doorwayUrl: 'https://alpha.elohim.host',
      };
      fetchMock.mockResolvedValueOnce(jsonResponse(200, wire));

      const result = await client.exchangeSessionToken('transfer uuid+1');

      const { url, init } = lastRequest();
      // Query param name is snake_case on the wire (serde_urlencoded, auth_routes.rs:4100-4102)
      // and the value is URL-encoded.
      expect(url).toBe(`${BASE_URL}/auth/exchange-session?session_token=transfer%20uuid%2B1`);
      expect(init.method).toBe('GET');
      expect(result).toEqual(wire);
      expect(client.restoreSession()?.token).toBe('jwt-from-exchange');
    });

    it('surfaces a consumed-token 401 as a typed error', async () => {
      fetchMock.mockResolvedValueOnce(
        jsonResponse(401, {
          error: 'Session token has already been used',
          code: 'TOKEN_CONSUMED',
        })
      );

      const err = await client.exchangeSessionToken('used-token').catch((e: unknown) => e);

      expect(err).toBeInstanceOf(DoorwaySessionError);
      expect((err as DoorwaySessionError).status).toBe(401);
      expect((err as DoorwaySessionError).code).toBe('TOKEN_CONSUMED');
      expect((err as DoorwaySessionError).message).toBe('Session token has already been used');
    });
  });

  // ===========================================================================
  // restoreSession
  // ===========================================================================

  describe('restoreSession', () => {
    it('returns null when no session is stored', () => {
      expect(client.restoreSession()).toBeNull();
    });

    it('returns the stored session when unexpired', () => {
      const session: StoredSession = {
        token: 'jwt-stored',
        humanId: 'human-9',
        agentPubKey: 'uhCAk-9',
        identifier: 'stored@example.com',
        expiresAt: unixSeconds(600),
      };
      const store = new InMemorySessionTokenStore();
      store.set(session);
      const restoring = new DoorwaySessionClient({
        baseUrl: BASE_URL,
        fetchImpl: fetchMock as unknown as typeof fetch,
        tokenStore: store,
      });

      expect(restoring.restoreSession()).toEqual(session);
    });

    it('returns null and clears the store when the session is expired', () => {
      const store = new InMemorySessionTokenStore();
      store.set({
        token: 'jwt-expired',
        humanId: 'human-9',
        agentPubKey: 'uhCAk-9',
        identifier: 'stored@example.com',
        expiresAt: unixSeconds(-10),
      });
      const restoring = new DoorwaySessionClient({
        baseUrl: BASE_URL,
        fetchImpl: fetchMock as unknown as typeof fetch,
        tokenStore: store,
      });

      expect(restoring.restoreSession()).toBeNull();
      expect(store.get()).toBeNull();
    });
  });

  // ===========================================================================
  // injected token store
  // ===========================================================================

  describe('tokenStore injection', () => {
    it('writes sessions through an injected store', async () => {
      const store = new InMemorySessionTokenStore();
      const injected = new DoorwaySessionClient({
        baseUrl: BASE_URL,
        fetchImpl: fetchMock as unknown as typeof fetch,
        tokenStore: store,
      });
      fetchMock.mockResolvedValueOnce(jsonResponse(200, makeAuthResponse()));

      await injected.login({ identifier: 'a@b', password: 'pw' });

      expect(store.get()?.token).toBe('jwt-token-abc');
    });

    it('defaults to an isolated in-memory store per client instance', async () => {
      fetchMock.mockResolvedValueOnce(jsonResponse(200, makeAuthResponse()));
      await client.login({ identifier: 'a@b', password: 'pw' });

      const sibling = new DoorwaySessionClient({
        baseUrl: BASE_URL,
        fetchImpl: fetchMock as unknown as typeof fetch,
      });
      expect(sibling.restoreSession()).toBeNull();
      expect(client.restoreSession()).not.toBeNull();
    });
  });

  // ===========================================================================
  // error mapping
  // ===========================================================================

  describe('error mapping', () => {
    it('maps a JSON error envelope to DoorwaySessionError with status, message, and code', async () => {
      fetchMock.mockResolvedValueOnce(
        jsonResponse(401, { error: 'Invalid credentials', code: 'INVALID_CREDENTIALS' })
      );

      const err = await client
        .login({ identifier: 'a@b', password: 'wrong' })
        .catch((e: unknown) => e);

      expect(err).toBeInstanceOf(DoorwaySessionError);
      expect((err as DoorwaySessionError).status).toBe(401);
      expect((err as DoorwaySessionError).message).toBe('Invalid credentials');
      expect((err as DoorwaySessionError).code).toBe('INVALID_CREDENTIALS');
    });

    it('falls back to the raw body text for non-JSON error responses', async () => {
      fetchMock.mockResolvedValueOnce(textResponse(502, 'Bad Gateway'));

      const err = await client.me().catch((e: unknown) => e);

      expect(err).toBeInstanceOf(DoorwaySessionError);
      expect((err as DoorwaySessionError).status).toBe(502);
      expect((err as DoorwaySessionError).message).toBe('Bad Gateway');
      expect((err as DoorwaySessionError).code).toBeUndefined();
    });

    it('falls back to HTTP status when body is whitespace-only', async () => {
      fetchMock.mockResolvedValueOnce(textResponse(502, '\n  '));

      const err = await client.me().catch((e: unknown) => e);

      expect(err).toBeInstanceOf(DoorwaySessionError);
      expect((err as DoorwaySessionError).status).toBe(502);
      expect((err as DoorwaySessionError).message).toBe('HTTP 502');
      expect((err as DoorwaySessionError).code).toBeUndefined();
    });
  });

  // ===========================================================================
  // wire-format passthrough (KEY RULE: camelCase in = camelCase out)
  // ===========================================================================

  describe('wire-format passthrough', () => {
    it('returns the parsed wire payload with zero key transformation', async () => {
      const wire = makeAuthResponse({
        isSteward: true,
        portalHostUrl: 'https://portal.matthew.host',
        profile: {
          id: 'profile-1',
          displayName: 'Terrance',
          bio: 'Walking the path',
          affinities: ['trust', 'governance'],
          profileReach: 'public',
          location: 'porch',
          createdAt: '2026-06-01T00:00:00Z',
          updatedAt: '2026-06-02T00:00:00Z',
        },
      });
      fetchMock.mockResolvedValueOnce(jsonResponse(200, wire));

      const result = await client.login({ identifier: 'a@b', password: 'pw' });

      // Deep-equal to the wire shape: nothing renamed, nothing added.
      expect(result).toEqual(wire);
      // Key sets are byte-identical at top level and nested.
      expect(Object.keys(result).sort()).toEqual(Object.keys(wire).sort());
      expect(Object.keys(result.profile as object).sort()).toEqual(
        Object.keys(wire.profile as object).sort()
      );
      // No snake_case leaked into the result.
      expect(JSON.stringify(result)).not.toMatch(/[a-z]+_[a-z]+":/);
    });
  });

  // ===========================================================================
  // baseUrl normalization
  // ===========================================================================

  describe('baseUrl handling', () => {
    it('tolerates a trailing slash on baseUrl', async () => {
      const slashed = new DoorwaySessionClient({
        baseUrl: `${BASE_URL}/`,
        fetchImpl: fetchMock as unknown as typeof fetch,
      });
      fetchMock.mockResolvedValueOnce(jsonResponse(200, makeAuthResponse()));

      await slashed.login({ identifier: 'a@b', password: 'pw' });

      expect(lastRequest().url).toBe(`${BASE_URL}/auth/login`);
    });

    it('preserves path segments in baseUrl', async () => {
      const withPath = new DoorwaySessionClient({
        baseUrl: 'http://localhost:8888/proxy',
        fetchImpl: fetchMock as unknown as typeof fetch,
      });
      const store = new InMemorySessionTokenStore();
      store.set({
        token: 'stored-token',
        humanId: 'human-1',
        agentPubKey: 'key-1',
        identifier: 'user@example.com',
        expiresAt: unixSeconds(3600),
      });
      const client2 = new DoorwaySessionClient({
        baseUrl: 'http://localhost:8888/proxy',
        fetchImpl: fetchMock as unknown as typeof fetch,
        tokenStore: store,
      });
      fetchMock.mockResolvedValueOnce(
        jsonResponse(200, {
          humanId: 'human-1',
          agentPubKey: 'key-1',
          identifier: 'user@example.com',
          permissionLevel: 'member',
          authenticated: true,
          trustMode: 'doorway-host',
          authority: { label: 'localhost' },
        })
      );

      await client2.me();

      expect(lastRequest().url).toBe('http://localhost:8888/proxy/auth/me');
    });
  });
});
