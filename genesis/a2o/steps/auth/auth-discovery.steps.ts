/**
 * Auth discovery steps — `GET /.well-known/elohim-auth`.
 *
 * Backs:
 *   - features/auth/auth-discovery.feature
 *
 * Where the truth lives:
 *   - doorway/doorway-service/src/routes/auth_discovery.rs — the handler, and
 *     the reasoning for `.well-known/` over `/auth/config`.
 *   - doorway/doorway-service/src/server/http.rs — `is_service_path` reserves
 *     `/.well-known/`, which is why an unknown path there 404s instead of being
 *     answered with the SPA shell.
 */

import { strict as assert } from 'node:assert';

import { When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import type { E2EWorld } from '../../src/framework/world.js';

interface DiscoveryState {
  status: number;
  body: string;
  doc: Record<string, unknown> | null;
}

function state(world: E2EWorld): DiscoveryState {
  const w = world as unknown as { __discovery?: DiscoveryState };
  if (!w.__discovery) {
    throw new Error('Nothing has been fetched yet — a When step must run first.');
  }
  return w.__discovery;
}

function doorwayBase(world: E2EWorld, id: string): string {
  let base = world.getDoorway(id).url;
  while (base.endsWith('/')) base = base.slice(0, -1);
  return base;
}

async function fetchPath(world: E2EWorld, doorwayId: string, path: string): Promise<void> {
  const res = await request(`${doorwayBase(world, doorwayId)}${path}`, { method: 'GET' });
  const body = await res.body.text();
  let doc: Record<string, unknown> | null = null;
  try {
    doc = JSON.parse(body) as Record<string, unknown>;
  } catch {
    doc = null;
  }
  (world as unknown as { __discovery: DiscoveryState }).__discovery = {
    status: res.statusCode,
    body,
    doc,
  };
}

When(
  'the auth discovery document is fetched from doorway {string}',
  async function (this: E2EWorld, doorwayId: string) {
    await fetchPath(this, doorwayId, '/.well-known/elohim-auth');
  }
);

When(
  '{string} is fetched from doorway {string}',
  async function (this: E2EWorld, path: string, doorwayId: string) {
    await fetchPath(this, doorwayId, path);
  }
);

Then('the discovery document names the sign-in portal', function (this: E2EWorld) {
  const s = state(this);
  assert.equal(s.status, 200, `discovery did not answer 200: ${s.status} ${s.body}`);
  assert.ok(
    s.doc,
    `discovery answered ${s.status} with a body that is not JSON — an app probing here would ` +
      `get a parse error instead of a usable document: ${s.body.slice(0, 120)}`
  );
  assert.equal(
    s.doc['portal'],
    '/threshold/login',
    `discovery did not name the doorway-hosted sign-in portal: ${JSON.stringify(s.doc)}`
  );
});

Then(
  'the discovery document names the endpoints for signing in, reading the session, and handing it to a sibling app',
  function (this: E2EWorld) {
    const s = state(this);
    const endpoints = (s.doc?.['endpoints'] ?? {}) as Record<string, unknown>;
    // The set an app would otherwise hardcode. sessionToken/exchangeSession are
    // the cross-app handoff pair — the SSO primitive a client cannot invent.
    for (const key of [
      'register',
      'login',
      'logout',
      'refresh',
      'me',
      'authorize',
      'token',
      'sessionToken',
      'exchangeSession',
    ]) {
      assert.ok(
        typeof endpoints[key] === 'string',
        `discovery is missing endpoint "${key}" — an app would have to hardcode it: ` +
          JSON.stringify(s.doc)
      );
    }
  }
);

Then('every location in the discovery document is origin-relative', function (this: E2EWorld) {
  const s = state(this);
  assert.ok(s.doc, 'no discovery document to inspect');

  const offenders: string[] = [];
  const walk = (value: unknown, path: string): void => {
    if (typeof value === 'string') {
      // Location-shaped values only; `doorwayId` is a name, not a location.
      const looksLikeLocation = value.startsWith('/') || value.includes('://');
      // `//host/x` is protocol-relative — it names ANOTHER origin while passing
      // a naive "starts with /" check, so it is called out explicitly.
      if (looksLikeLocation && (!value.startsWith('/') || value.startsWith('//'))) {
        offenders.push(`${path} = ${value}`);
      }
    } else if (value && typeof value === 'object') {
      for (const [k, v] of Object.entries(value as Record<string, unknown>)) {
        walk(v, `${path}.${k}`);
      }
    }
  };
  walk(s.doc, '$');

  assert.deepEqual(
    offenders,
    [],
    'the discovery document names a location outside its own origin, which makes it an ' +
      `open-redirect primitive: ${offenders.join(', ')}`
  );
});

Then('the doorway refuses it as not found', function (this: E2EWorld) {
  const s = state(this);
  assert.equal(
    s.status,
    404,
    `expected an honest 404, got ${s.status} — a path answered with the app shell hands a ` +
      `client a parse error instead of an absence signal: ${s.body.slice(0, 120)}`
  );
});

/**
 * The advertise/serve symmetry guard (concern C7 on this route's seam-registry row).
 *
 * The document's endpoint list and `AUTH_OWNED_PATHS` in the doorway are two
 * hand-maintained lists that must agree. When they diverge, the advertised path
 * is not owned, falls through to the EPR router, and answers 200 text/html — so
 * a client following the document receives a web page where it expected an
 * endpoint. Status codes are deliberately NOT asserted (405/401/400 are all
 * healthy answers from an owned route); the content type is what separates
 * "the doorway answered" from "the app shell answered".
 */
Then(
  'every advertised endpoint answers as an auth route, not the app shell',
  async function (this: E2EWorld) {
    const s = state(this);
    const endpoints = (s.doc?.['endpoints'] ?? {}) as Record<string, unknown>;
    const paths = Object.entries(endpoints).filter(
      (e): e is [string, string] => typeof e[1] === 'string'
    );
    assert.ok(paths.length > 0, 'the document advertised no endpoints to check');

    const base = doorwayBase(this, 'alpha');
    const shells: string[] = [];
    for (const [name, path] of paths) {
      const res = await request(`${base}${path}`, { method: 'GET' });
      const contentType = String(res.headers['content-type'] ?? '');
      await res.body.text();
      if (contentType.includes('text/html')) {
        shells.push(`${name} (${path}) -> ${res.statusCode} ${contentType}`);
      }
    }
    assert.deepEqual(
      shells,
      [],
      'the document advertises endpoints the doorway does not own — these answered with the ' +
        `app shell instead of the auth layer: ${shells.join(', ')}`
    );
  }
);

Then(
  'the advertised portal is the page built for that path, not the app shell',
  async function (this: E2EWorld) {
    const s = state(this);
    const portal = s.doc?.['portal'];
    assert.equal(typeof portal, 'string', 'the document advertised no portal');
    const portalPath = String(portal);

    const res = await request(`${doorwayBase(this, 'alpha')}${portalPath}`, { method: 'GET' });
    const html = await res.body.text();
    assert.equal(
      res.statusCode,
      200,
      `the advertised portal did not serve: ${res.statusCode} — a human sent there sees nothing`
    );

    // "It answered with HTML" proves nothing here: the app-shell catch-all also
    // answers 200 HTML, and unlike the JSON endpoints the portal is SUPPOSED to
    // be a page — so content type cannot separate the real portal from the
    // fallback. What can: a bundle built to be served under a path declares that
    // path as its <base href>. The app shell's is "/". Deriving the expected
    // value from the ADVERTISED path also means this cannot drift apart from the
    // document it is checking.
    // Everything up to and including the last '/' — a string op, not a regex:
    // sonarjs/slow-regex refuses `/[^/]*$/` (backtracking on a long path).
    const baseHref = portalPath.slice(0, portalPath.lastIndexOf('/') + 1);
    const declared = /<base\s+href="([^"]*)"/i.exec(html)?.[1];
    assert.equal(
      declared,
      baseHref,
      `the portal path served a bundle whose <base href> is ${declared ?? '(absent)'}, not ` +
        `"${baseHref}" — that is the app shell answering for an unclaimed path, not the portal`
    );
  }
);
