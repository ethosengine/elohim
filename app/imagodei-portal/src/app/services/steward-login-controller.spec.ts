import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  readStewardHandoff,
  runStewardLogin,
  type StewardLoginEffects,
} from './steward-login-controller.js';
import type { StandaloneResolver } from './standalone-resolver.js';

function makeResolver(over: Partial<StandaloneResolver>): StandaloneResolver {
  return over as unknown as StandaloneResolver;
}

function makeEffects(): StewardLoginEffects & {
  calls: { stripped: string[]; consent: boolean[]; authority: number };
} {
  const calls = { stripped: [] as string[], consent: [] as boolean[], authority: 0 };
  return {
    calls,
    stripHandoffParams: (search: string) => {
      calls.stripped.push(search);
    },
    refreshAuthority: async () => {
      calls.authority += 1;
    },
    enterConsentIfRequested: async (search: string) => {
      calls.consent.push(new URLSearchParams(search).has('client_id'));
    },
  };
}

describe('readStewardHandoff', () => {
  it('detects a handoff when both session_token and doorway_url are present', () => {
    const h = readStewardHandoff(
      '?session_token=opaque&doorway_url=https://alpha.elohim.host'
    );
    expect(h).not.toBeNull();
    expect(h?.sessionToken).toBe('opaque');
    expect(h?.doorwayUrl).toBe('https://alpha.elohim.host');
  });

  it('returns null when only session_token is present', () => {
    expect(readStewardHandoff('?session_token=opaque')).toBeNull();
  });

  it('returns null when neither param is present (normal flow untouched)', () => {
    expect(readStewardHandoff('?client_id=graphos&claims=name')).toBeNull();
  });
});

describe('runStewardLogin', () => {
  let effects: ReturnType<typeof makeEffects>;

  beforeEach(() => {
    effects = makeEffects();
  });

  it('on 201: strips handoff params, re-runs /auth/me discovery, then proceeds (no error)', async () => {
    const loginWithSessionToken = vi.fn().mockResolvedValue({
      ok: true,
      status: 201,
      session: { humanId: 'matthew' },
    });
    const resolver = makeResolver({ loginWithSessionToken });

    const outcome = await runStewardLogin(
      resolver,
      { sessionToken: 'opaque', doorwayUrl: 'https://alpha.elohim.host' },
      '?session_token=opaque&doorway_url=https://alpha.elohim.host',
      effects
    );

    expect(loginWithSessionToken).toHaveBeenCalledWith(
      'opaque',
      'https://alpha.elohim.host'
    );
    expect(outcome.status).toBe('authenticated');
    // Params stripped from the address bar (the code never lingers in the URL).
    expect(effects.calls.stripped.length).toBe(1);
    // /auth/me discovery re-run so the shell proceeds as a local session.
    expect(effects.calls.authority).toBe(1);
  });

  it('on 201 with OAuth params: preserves them so the consent flow can pick them up', async () => {
    const loginWithSessionToken = vi
      .fn()
      .mockResolvedValue({ ok: true, status: 201, session: { humanId: 'm' } });
    const resolver = makeResolver({ loginWithSessionToken });

    const search =
      '?session_token=opaque&doorway_url=https://alpha.elohim.host' +
      '&client_id=graphos-designer&redirect_uri=https://app/cb&response_type=code&state=xyz';

    await runStewardLogin(
      resolver,
      { sessionToken: 'opaque', doorwayUrl: 'https://alpha.elohim.host' },
      search,
      effects
    );

    // The consent re-check saw the OAuth client_id survive the hop.
    expect(effects.calls.consent).toEqual([true]);
  });

  it('on 401: surfaces a non-scary failure with a return-to-doorway action', async () => {
    const loginWithSessionToken = vi
      .fn()
      .mockResolvedValue({ ok: false, status: 401 });
    const resolver = makeResolver({ loginWithSessionToken });

    const outcome = await runStewardLogin(
      resolver,
      { sessionToken: 'stale', doorwayUrl: 'https://alpha.elohim.host' },
      '?session_token=stale&doorway_url=https://alpha.elohim.host',
      effects
    );

    expect(outcome.status).toBe('failed');
    expect(outcome.returnToDoorwayUrl).toBe('https://alpha.elohim.host');
    expect(outcome.message).toMatch(/couldn.t complete the hand-off|sign in again|safe/i);
    // We did NOT strip params or advance to an authenticated session on failure.
    expect(effects.calls.authority).toBe(0);
  });

  it('on 403: same return-to-doorway action (issuer not trusted)', async () => {
    const loginWithSessionToken = vi
      .fn()
      .mockResolvedValue({ ok: false, status: 403 });
    const resolver = makeResolver({ loginWithSessionToken });

    const outcome = await runStewardLogin(
      resolver,
      { sessionToken: 'code', doorwayUrl: 'https://alpha.elohim.host' },
      '?session_token=code&doorway_url=https://alpha.elohim.host',
      effects
    );

    expect(outcome.status).toBe('failed');
    expect(outcome.returnToDoorwayUrl).toBe('https://alpha.elohim.host');
  });

  it('on network error (no status): conductor-unreachable copy, account still safe', async () => {
    const loginWithSessionToken = vi.fn().mockResolvedValue({ ok: false });
    const resolver = makeResolver({ loginWithSessionToken });

    const outcome = await runStewardLogin(
      resolver,
      { sessionToken: 'code', doorwayUrl: 'https://alpha.elohim.host' },
      '?session_token=code&doorway_url=https://alpha.elohim.host',
      effects
    );

    expect(outcome.status).toBe('unreachable');
    expect(outcome.message).toMatch(/conductor isn.t reachable|account is still safe/i);
    expect(outcome.returnToDoorwayUrl).toBe('https://alpha.elohim.host');
  });
});
