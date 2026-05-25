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
