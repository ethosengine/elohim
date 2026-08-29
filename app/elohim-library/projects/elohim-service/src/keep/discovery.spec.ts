/**
 * The discovery reader's job is to be safe to trust on an UNSIGNED document.
 * Every test here is one way a hostile or broken response could otherwise aim a
 * Login button, or be misread as an answer it is not.
 */

import { describe, expect, it, vi } from 'vitest';

import {
  CLIENT_AUTH_PATHS,
  isOriginRelative,
  pathDrift,
  portalUrl,
  readAuthDiscovery,
  rejectionsIn,
} from './discovery.js';

import type { AuthDiscovery } from '../generated/auth-discovery.js';

const ORIGIN = 'https://doorway-alpha.elohim.host';

function goodDoc(over: Partial<AuthDiscovery> = {}): AuthDiscovery {
  return {
    version: 1,
    doorwayId: 'alpha-elohim-host',
    portal: '/threshold/login',
    endpoints: {
      register: '/auth/register',
      login: '/auth/login',
      logout: '/auth/logout',
      refresh: '/auth/refresh',
      me: '/auth/me',
      authorize: '/auth/authorize',
      token: '/auth/token',
      sessionToken: '/auth/session-token',
      exchangeSession: '/auth/exchange-session',
      portalHost: '/auth/portal-host',
    },
    ...over,
  };
}

function serving(body: unknown, init: { status?: number; json?: boolean } = {}): typeof fetch {
  const status = init.status ?? 200;
  return vi.fn(
    async () =>
      ({
        ok: status >= 200 && status < 300,
        status,
        json: async () => {
          if (init.json === false) throw new SyntaxError('Unexpected token <');
          return body;
        },
      }) as unknown as Response
  ) as unknown as typeof fetch;
}

describe('origin-relative enforcement', () => {
  it('rejects the protocol-relative bypass, not just absolute URLs', () => {
    expect(isOriginRelative('/auth/login')).toBe(true);
    // The classic: names ANOTHER origin while passing a naive leading-slash check.
    expect(isOriginRelative('//evil.tld/login')).toBe(false);
    expect(isOriginRelative('https://evil.tld/login')).toBe(false);
    expect(isOriginRelative('http://evil.tld/login')).toBe(false);
  });

  it('catches an escaping document through the real walker', () => {
    for (const hostile of ['//evil.tld/login', 'https://evil.tld/login']) {
      const bad = rejectionsIn(goodDoc({ portal: hostile }));
      expect(bad.join(' '), `did not catch ${hostile}`).toContain(hostile);
      expect(bad.join(' ')).toContain('escapes this origin');
    }
  });

  it('detector control: a clean document produces NO rejections', () => {
    // Without this, a walker that silently visited nothing would look identical
    // to a walker that checked everything and found it clean.
    expect(rejectionsIn(goodDoc())).toEqual([]);
  });
});

describe('owned-prefix enforcement', () => {
  it("refuses an origin-relative path that is NOT the auth layer's", () => {
    // The sharp one: /apps/{epr_id}/{sub_path} is a service prefix proxying to
    // storage's bundle-serving surface — third-party content. This path is
    // origin-relative and passes the view schema, and it points a password form
    // at something an attacker uploaded.
    const bad = rejectionsIn(
      goodDoc({ endpoints: { ...goodDoc().endpoints, login: '/apps/evil/login.html' } })
    );
    expect(bad.join(' ')).toContain('/apps/evil/login.html');
    expect(bad.join(' ')).toContain("outside the auth layer's own prefixes");
  });

  it('refuses a portal outside the owned prefixes', () => {
    expect(rejectionsIn(goodDoc({ portal: '/apps/evil/index.html' }))).not.toEqual([]);
    expect(portalUrl(goodDoc({ portal: '/apps/evil/index.html' }), ORIGIN)).toBeNull();
    expect(portalUrl(goodDoc({ portal: '//evil.tld/login' }), ORIGIN)).toBeNull();
  });
});

describe('readAuthDiscovery — states that must not be confused', () => {
  it('a 404 is ABSENT: the doorway answered our question', async () => {
    const answer = await readAuthDiscovery(ORIGIN, { fetchImpl: serving(null, { status: 404 }) });
    expect(answer.state).toBe('absent');
    expect(answer.state !== 'present' && answer.reason).toBe('not-found');
  });

  it('a network failure is UNREACHABLE, never absent', async () => {
    const throwing = vi.fn(async () => {
      throw new TypeError('Failed to fetch');
    }) as unknown as typeof fetch;
    const answer = await readAuthDiscovery(ORIGIN, { fetchImpl: throwing });
    // Absence must never be inferred from silence — that is how a client
    // concludes a doorway has no document because it was briefly offline.
    expect(answer.state).toBe('unreachable');
    expect(answer.state !== 'present' && answer.reason).toBe('transport');
  });

  it('an abort is UNREACHABLE/timeout, distinct from transport', async () => {
    const aborting = vi.fn(async () => {
      const e = new Error('aborted');
      e.name = 'AbortError';
      throw e;
    }) as unknown as typeof fetch;
    const answer = await readAuthDiscovery(ORIGIN, { fetchImpl: aborting });
    expect(answer.state !== 'present' && answer.reason).toBe('timeout');
  });

  it('a 403 is REFUSED — about us, not about whether the document exists', async () => {
    const answer = await readAuthDiscovery(ORIGIN, { fetchImpl: serving(null, { status: 403 }) });
    expect(answer.state).toBe('unreachable');
    expect(answer.state !== 'present' && answer.reason).toBe('refused');
  });

  it('200-with-HTML is document-rejected, not a parse crash', async () => {
    // The app-shell-answering shape. Living under /.well-known/ is what makes
    // this rare, but a client must still branch rather than throw.
    const answer = await readAuthDiscovery(ORIGIN, {
      fetchImpl: serving(null, { json: false }),
    });
    expect(answer.state).toBe('unreachable');
    expect(answer.state !== 'present' && answer.reason).toBe('document-rejected');
    expect(answer.state !== 'present' && answer.detail).toContain('not JSON');
  });

  it('a hostile document is refused rather than returned', async () => {
    const answer = await readAuthDiscovery(ORIGIN, {
      fetchImpl: serving(goodDoc({ portal: '//evil.tld/login' })),
    });
    expect(answer.state).toBe('unreachable');
    expect(answer.state !== 'present' && answer.reason).toBe('document-rejected');
  });

  it('a JSON body that is not a discovery document is refused', async () => {
    const answer = await readAuthDiscovery(ORIGIN, { fetchImpl: serving({ hello: 'world' }) });
    expect(answer.state !== 'present' && answer.reason).toBe('document-rejected');
  });

  it('a well-formed document is PRESENT and yields a portal on this origin', async () => {
    const answer = await readAuthDiscovery(ORIGIN, { fetchImpl: serving(goodDoc()) });
    expect(answer.state).toBe('present');
    if (answer.state !== 'present') return;
    expect(portalUrl(answer.value, ORIGIN)).toBe(`${ORIGIN}/threshold/login`);
    expect(portalUrl(answer.value, `${ORIGIN}/`, '/lamad')).toBe(
      `${ORIGIN}/threshold/login?returnUrl=%2Flamad`
    );
  });
});

describe('pathDrift', () => {
  it('is silent when the document and the client agree', () => {
    expect(pathDrift(goodDoc())).toEqual([]);
  });

  it('reports a moved endpoint as data instead of throwing', () => {
    const moved = goodDoc({ endpoints: { ...goodDoc().endpoints, me: '/auth/whoami' } });
    expect(pathDrift(moved)).toEqual([
      { endpoint: 'me', advertised: '/auth/whoami', clientUses: '/auth/me' },
    ]);
  });

  it('every path the client hardcodes is one the document advertises', () => {
    // If this fails, either the session client grew an endpoint the document
    // does not name, or an endpoint was renamed — both are exactly the drift
    // this reader exists to surface.
    const advertised = goodDoc().endpoints as unknown as Record<string, string>;
    const missing = Object.keys(CLIENT_AUTH_PATHS).filter(k => typeof advertised[k] !== 'string');
    expect(missing).toEqual([]);
  });
});
