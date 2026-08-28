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
