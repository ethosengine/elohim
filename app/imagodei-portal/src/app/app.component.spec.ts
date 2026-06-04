/**
 * AppComponent ngOnInit orchestration tests.
 *
 * Drives the component directly (no TestBed — it has no injected deps; it
 * instantiates StandaloneResolver internally) and stubs the network via
 * global.fetch plus window.location/window.history. Asserts the doorway→steward
 * handoff behaviour against the four required cases:
 *   (a) params present  → /session/exchange called with both values
 *   (b) 201             → params stripped + /auth/me consulted + authenticated
 *   (c) 401             → failure state with return-to-doorway action
 *   (d) no params       → existing flow untouched (regression)
 */
// Enable Angular's JIT compiler facade so partially-compiled library
// injectables (CommonModule → PlatformLocation) resolve when the component
// module is imported in the node test env. Must precede the component import.
import '@angular/compiler';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { AppComponent } from './app.component.js';

interface FetchCall {
  url: string;
  init?: RequestInit;
}

function installLocation(search: string): void {
  // jsdom/node: redefine window.location with a writable search.
  const loc = {
    pathname: '/account',
    search,
    hash: '',
    origin: 'https://matthew.steward.example',
    href: `https://matthew.steward.example/account${search}`,
  };
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: {
      location: loc,
      history: {
        state: null,
        replaceState: vi.fn(),
      },
    },
  });
}

function meResponse(): Response {
  return new Response(
    JSON.stringify({
      humanId: 'matthew',
      authenticated: true,
      trustMode: 'peer-conductor',
      authority: { label: 'your conductor on this device', id: 'peer-abc' },
      conductorEndpoint: 'peer-abc',
    }),
    { status: 200, headers: { 'content-type': 'application/json' } },
  );
}

function localSession201(): Response {
  return new Response(
    JSON.stringify({
      id: 'sess-1',
      humanId: 'matthew',
      agentPubKey: 'uhCAk',
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
  );
}

describe('AppComponent — doorway→steward handoff', () => {
  let calls: FetchCall[];

  beforeEach(() => {
    calls = [];
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  function routeFetch(handlers: Record<string, () => Response>): void {
    global.fetch = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      calls.push({ url, init });
      const key = Object.keys(handlers).find(k => url.includes(k));
      if (key) return Promise.resolve(handlers[key]());
      return Promise.resolve(new Response('not found', { status: 404 }));
    }) as unknown as typeof fetch;
  }

  it('(a) params present → POST /session/exchange called with sessionToken + doorwayUrl', async () => {
    installLocation('?session_token=opaque&doorway_url=https://alpha.elohim.host');
    routeFetch({
      '/session/exchange': localSession201,
      '/auth/me': meResponse,
    });

    const cmp = new AppComponent();
    await cmp.ngOnInit();

    const exchange = calls.find(c => c.url.includes('/session/exchange'));
    expect(exchange).toBeDefined();
    expect(exchange!.url).toBe('/session/exchange');
    expect(exchange!.init?.method).toBe('POST');
    expect(JSON.parse(exchange!.init!.body as string)).toEqual({
      sessionToken: 'opaque',
      doorwayUrl: 'https://alpha.elohim.host',
    });
  });

  it('(b) 201 → handoff params stripped from URL, /auth/me consulted, proceeds authenticated', async () => {
    installLocation('?session_token=opaque&doorway_url=https://alpha.elohim.host');
    routeFetch({
      '/session/exchange': localSession201,
      '/auth/me': meResponse,
    });

    const cmp = new AppComponent();
    await cmp.ngOnInit();

    // address bar rewritten with the code removed
    const replace = (window.history.replaceState as unknown as ReturnType<typeof vi.fn>);
    expect(replace).toHaveBeenCalled();
    const newUrl = replace.mock.calls[0][2] as string;
    expect(newUrl).not.toContain('session_token');
    expect(newUrl).not.toContain('doorway_url');

    // /auth/me was consulted (at least once: prefetch and/or refresh after exchange)
    expect(calls.some(c => c.url.includes('/auth/me'))).toBe(true);

    // authority discovered as peer-conductor (NOT configured)
    expect(cmp.authority()?.trustMode).toBe('peer-conductor');

    // no longer in the transient steward-login mode; not in a failure phase
    expect(cmp.mode()).not.toBe('steward-login');
    expect(cmp.stewardMessage()).toBe('');
  });

  it('(b2) 201 with OAuth params → params survive the hop into the consent flow', async () => {
    installLocation(
      '?session_token=opaque&doorway_url=https://alpha.elohim.host' +
        '&client_id=graphos-designer&claims=imagodei.displayName' +
        '&redirect_uri=https://app/cb&response_type=code&state=xyz',
    );
    routeFetch({
      '/session/exchange': localSession201,
      '/auth/me': meResponse,
      '/auth/authorize/prepare': () =>
        new Response(
          JSON.stringify({
            requestingClient: { id: 'graphos-designer', displayName: 'Graphos Designer' },
            requestedClaims: [{ id: 'imagodei.displayName', label: 'Display name' }],
            requiredClaims: ['imagodei.displayName'],
          }),
          { status: 200, headers: { 'content-type': 'application/json' } },
        ),
    });

    const cmp = new AppComponent();
    await cmp.ngOnInit();

    // OAuth consent picked up after the session landed
    expect(cmp.mode()).toBe('consent');
    expect(cmp.consentCtx()?.requestingClient.id).toBe('graphos-designer');
    // prepare was POSTed with the surviving client_id
    const prepare = calls.find(c => c.url.includes('/auth/authorize/prepare'));
    expect(prepare).toBeDefined();
    expect(JSON.parse(prepare!.init!.body as string).clientId).toBe('graphos-designer');
  });

  it('(c) 401 → failure state with a non-scary message + return-to-doorway action', async () => {
    installLocation('?session_token=stale&doorway_url=https://alpha.elohim.host');
    routeFetch({
      '/session/exchange': () =>
        new Response(JSON.stringify({ error: 'redeem failed' }), {
          status: 401,
          headers: { 'content-type': 'application/json' },
        }),
      '/auth/me': () =>
        new Response(JSON.stringify({ error: 'no active session' }), {
          status: 401,
          headers: { 'content-type': 'application/json' },
        }),
    });

    const cmp = new AppComponent();
    await cmp.ngOnInit();

    expect(cmp.mode()).toBe('steward-login');
    expect(cmp.stewardPhase()).toBe('failed');
    expect(cmp.stewardReturnUrl()).toBe('https://alpha.elohim.host');
    expect(cmp.stewardMessage()).toMatch(/safe/i);
    // No crypto-bro language; should NOT contain "wallet"/"keys"/"crypto".
    expect(cmp.stewardMessage().toLowerCase()).not.toMatch(/wallet|crypto|seed phrase/);
  });

  it('(d) no params → existing flow untouched (no /session/exchange, stays login)', async () => {
    installLocation('');
    routeFetch({ '/auth/me': meResponse });

    const cmp = new AppComponent();
    await cmp.ngOnInit();

    expect(calls.some(c => c.url.includes('/session/exchange'))).toBe(false);
    expect(cmp.mode()).toBe('login');
    expect(cmp.step()).toBe('resolve');
  });

  it('(d2) consent params only (no handoff) → consent flow as before (regression)', async () => {
    installLocation('?client_id=graphos-designer&claims=imagodei.displayName');
    routeFetch({
      '/auth/me': meResponse,
      '/auth/authorize/prepare': () =>
        new Response(
          JSON.stringify({
            requestingClient: { id: 'graphos-designer', displayName: 'Graphos Designer' },
            requestedClaims: [{ id: 'imagodei.displayName', label: 'Display name' }],
            requiredClaims: ['imagodei.displayName'],
          }),
          { status: 200, headers: { 'content-type': 'application/json' } },
        ),
    });

    const cmp = new AppComponent();
    await cmp.ngOnInit();

    expect(calls.some(c => c.url.includes('/session/exchange'))).toBe(false);
    expect(cmp.mode()).toBe('consent');
  });
});
