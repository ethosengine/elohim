import { describe, it, expect, beforeEach, vi } from 'vitest';
import { StandaloneResolver } from './standalone-resolver.js';

describe('StandaloneResolver', () => {
  let resolver: StandaloneResolver;

  beforeEach(() => {
    resolver = new StandaloneResolver();
  });

  describe('resolveIdentifier', () => {
    it('returns ok=false when identifier is malformed (no @)', async () => {
      const out = await resolver.resolveIdentifier('not-a-federated-id');
      expect(out.ok).toBe(false);
      expect(out.reason).toMatch(/format/);
    });

    it('returns ok=false when gateway host is empty', async () => {
      const out = await resolver.resolveIdentifier('matthew@');
      expect(out.ok).toBe(false);
    });

    it('returns ok=false when fetch fails', async () => {
      global.fetch = vi.fn().mockRejectedValueOnce(new Error('network'));
      const out = await resolver.resolveIdentifier('matthew@nowhere.host');
      expect(out.ok).toBe(false);
    });

    it('returns ok=true with doorwayUrl when /healthz is reachable', async () => {
      global.fetch = vi.fn().mockResolvedValueOnce(new Response('ok', {
        status: 200,
      }));
      const out = await resolver.resolveIdentifier('matthew@alpha.elohim.host');
      expect(out.ok).toBe(true);
      expect(out.doorwayUrl).toBe('https://alpha.elohim.host');
    });
  });

  describe('loginWithPassword', () => {
    it('POSTs to /auth/login and returns redirect path', async () => {
      global.fetch = vi.fn().mockResolvedValueOnce(new Response(
        JSON.stringify({ redirect: '/lamad' }),
        { status: 200, headers: { 'content-type': 'application/json' } },
      ));
      const out = await resolver.loginWithPassword({
        identifier: 'matthew@alpha.elohim.host',
        password: 'shibboleth',
        remember: false,
      });
      expect(out.redirect).toBe('/lamad');
    });

    it('returns error message when login fails (4xx)', async () => {
      global.fetch = vi.fn().mockResolvedValueOnce(new Response(
        JSON.stringify({ error: 'invalid credentials' }),
        { status: 401, headers: { 'content-type': 'application/json' } },
      ));
      const out = await resolver.loginWithPassword({
        identifier: 'matthew@alpha.elohim.host',
        password: 'wrong',
        remember: false,
      });
      expect(out.error).toBeDefined();
    });
  });

  describe('exchangeCode', () => {
    it('POSTs to /auth/token with code + state, returns session', async () => {
      global.fetch = vi.fn().mockResolvedValueOnce(new Response(
        JSON.stringify({ session: { humanId: 'matthew' } }),
        { status: 200, headers: { 'content-type': 'application/json' } },
      ));
      const out = await resolver.exchangeCode('abc', 'xyz');
      expect(out.session).toEqual({ humanId: 'matthew' });
    });

    it('throws when exchange fails', async () => {
      global.fetch = vi.fn().mockResolvedValueOnce(new Response('bad', { status: 400 }));
      await expect(resolver.exchangeCode('bad', 'xyz')).rejects.toThrow();
    });
  });

  describe('loginWithSessionToken', () => {
    it('POSTs to /session/exchange with sessionToken + doorwayUrl, returns ok + session on 201', async () => {
      const fetchSpy = vi.fn().mockResolvedValueOnce(new Response(
        JSON.stringify({
          id: 'sess-1',
          humanId: 'matthew',
          agentPubKey: 'uhCAk...',
          doorwayUrl: 'https://alpha.elohim.host',
          doorwayId: null,
          identifier: 'matthew@alpha.elohim.host',
          displayName: 'Matthew',
          profileImageHash: null,
          isActive: true,
          createdAt: '2026-06-04T00:00:00Z',
          updatedAt: '2026-06-04T00:00:00Z',
          lastSyncedAt: null,
          bootstrapUrl: null,
        }),
        { status: 201, headers: { 'content-type': 'application/json' } },
      ));
      global.fetch = fetchSpy;

      const out = await resolver.loginWithSessionToken('opaque-code', 'https://alpha.elohim.host');

      expect(out.ok).toBe(true);
      expect(out.session?.humanId).toBe('matthew');

      // Same-origin relative URL — never the issuer origin.
      const [url, init] = fetchSpy.mock.calls[0];
      expect(url).toBe('/session/exchange');
      expect((init as RequestInit).method).toBe('POST');
      expect((init as RequestInit).credentials).toBe('include');
      expect(JSON.parse((init as RequestInit).body as string)).toEqual({
        sessionToken: 'opaque-code',
        doorwayUrl: 'https://alpha.elohim.host',
      });
    });

    it('returns ok=false with status 401 when the token is rejected (expired/replayed)', async () => {
      global.fetch = vi.fn().mockResolvedValueOnce(new Response(
        JSON.stringify({ error: 'redeem failed' }),
        { status: 401, headers: { 'content-type': 'application/json' } },
      ));
      const out = await resolver.loginWithSessionToken('stale', 'https://alpha.elohim.host');
      expect(out.ok).toBe(false);
      expect(out.status).toBe(401);
    });

    it('returns ok=false with status 403 when the issuer is not trusted', async () => {
      global.fetch = vi.fn().mockResolvedValueOnce(new Response(
        JSON.stringify({ error: 'issuer not trusted' }),
        { status: 403, headers: { 'content-type': 'application/json' } },
      ));
      const out = await resolver.loginWithSessionToken('code', 'https://stranger.example');
      expect(out.ok).toBe(false);
      expect(out.status).toBe(403);
    });

    it('returns ok=false with no status when the network call throws', async () => {
      global.fetch = vi.fn().mockRejectedValueOnce(new Error('offline'));
      const out = await resolver.loginWithSessionToken('code', 'https://alpha.elohim.host');
      expect(out.ok).toBe(false);
      expect(out.status).toBeUndefined();
    });
  });

  describe('fetchAuthority', () => {
    it('GETs /auth/me and surfaces the discovered trustMode (peer-conductor) — never configured', async () => {
      global.fetch = vi.fn().mockResolvedValueOnce(new Response(
        JSON.stringify({
          humanId: 'matthew',
          authenticated: true,
          trustMode: 'peer-conductor',
          authority: { label: 'your conductor on this device', id: 'peer-abc' },
          conductorEndpoint: 'peer-abc',
        }),
        { status: 200, headers: { 'content-type': 'application/json' } },
      ));
      const authority = await resolver.fetchAuthority();
      expect(authority).not.toBeNull();
      expect(authority?.trustMode).toBe('peer-conductor');
      expect(authority?.authority.label).toBe('your conductor on this device');
    });

    it('returns null when /auth/me is unauthenticated (401)', async () => {
      global.fetch = vi.fn().mockResolvedValueOnce(new Response(
        JSON.stringify({ error: 'no active session' }),
        { status: 401, headers: { 'content-type': 'application/json' } },
      ));
      const authority = await resolver.fetchAuthority();
      expect(authority).toBeNull();
    });
  });

  describe('prepareConsent', () => {
    it('POSTs to /auth/authorize/prepare and returns context', async () => {
      global.fetch = vi.fn().mockResolvedValueOnce(new Response(
        JSON.stringify({
          requestingClient: { id: 'graphos-designer', displayName: 'Graphos Designer' },
          requestedClaims: [{ id: 'imagodei.displayName', label: 'Display name' }],
          requiredClaims: ['imagodei.displayName'],
        }),
        { status: 200, headers: { 'content-type': 'application/json' } },
      ));
      const out = await resolver.prepareConsent({
        clientId: 'graphos-designer',
        claims: ['imagodei.displayName'],
        redirectUri: 'https://graphos-designer.example/callback',
        state: 'abc',
      });
      expect(out.requestingClient.id).toBe('graphos-designer');
      expect(out.requiredClaims).toContain('imagodei.displayName');
    });
  });
});
