/**
 * OAuth authorization-code step definitions — the API-level half of the
 * doorway's RFC-6749 server.
 *
 * Backs:
 *   - features/auth/oauth-authorization-code.feature
 *
 * Where the truth lives (these steps assert on the real wire contract, measured
 * against a running doorway on the local mesh 2026-08-28, never on a paraphrase):
 *   - doorway/doorway-service/src/routes/auth_routes.rs
 *       `handle_authorize` — validates `client_id` against the registered client
 *       list, then `redirect_uri` against that client's patterns, THEN checks the
 *       bearer. An unauthenticated caller is 302'd to `/threshold/login` with every
 *       request param preserved.
 *   - doorway/doorway-service/src/db/schemas/oauth_session.rs
 *       `matches_uri_pattern` — the redirect_uri boundary this feature regresses,
 *       and `OAuthSessionDoc` (the 5-minute single-use code record).
 *
 * THE ONE SHARP EDGE: `handle_authorize` branches on the Authorization header.
 * With a bearer present it treats the caller as a SPA that cannot follow a
 * cross-origin 302 and answers `200 {"redirect_uri": "<uri>?code=…&state=…"}`.
 * Without one it answers a real `302`. So the authenticated happy path is read
 * out of a JSON BODY, not a Location header — asserting on `302` for the
 * authenticated case silently tests nothing.
 */

import { strict as assert } from 'node:assert';
import { randomUUID } from 'node:crypto';

import { Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import type { E2EWorld } from '../../src/framework/world.js';

/** Per-scenario OAuth state. The cucumber World is constructed per scenario, so
 *  hanging this off it keeps parallel workers from sharing a code. */
interface OAuthState {
  /** What the DOORWAY calls this human, read back from its own login answer. */
  identifier: string;
  humanId: string;
  token: string;
  state: string;
  /** Raw body of the last /auth/authorize answer (JSON for the ajax branch). */
  body: string;
  status: number;
  /** Location header, present only on the unauthenticated 302 branch. */
  location: string | null;
  /** Code parsed out of the issued redirect_uri, when one was issued. */
  code: string | null;
  /** Raw body of the last /auth/token answer. */
  tokenBody: string;
  tokenStatus: number;
}

function oauth(world: E2EWorld): OAuthState {
  const w = world as unknown as { __oauth?: OAuthState };
  if (!w.__oauth) {
    throw new Error(
      'No OAuth state on this scenario — the Background step ' +
        '\'"Miriam" holds an open session on doorway ...\' must run first.'
    );
  }
  return w.__oauth;
}

function doorwayBase(world: E2EWorld, id: string): string {
  // Trailing slashes stripped without a regex, matching `probeReach` in
  // common.steps.ts (a backtracking `/\/+$/` trips sonarjs/slow-regex).
  let base = world.getDoorway(id).url;
  while (base.endsWith('/')) base = base.slice(0, -1);
  return base;
}

/** Parse `?code=…` out of an issued redirect target. Returns null when absent.
 *  Parsed rather than regexed: the code is a security-relevant value and the
 *  redirect target is attacker-influenced in the refusal scenarios. */
function codeFrom(redirectUri: string | null): string | null {
  if (!redirectUri) return null;
  try {
    return new URL(redirectUri).searchParams.get('code');
  } catch {
    return null;
  }
}

/**
 * Makes the registered-application prior state VISIBLE and CHECKED rather than
 * assumed. `handle_authorize` validates in a fixed order — client lookup, then
 * redirect_uri, then the bearer — so an UNAUTHENTICATED request carrying a
 * callback inside the bound proves both that the application is registered and
 * that the bound admits it, without minting a code as a side effect.
 */
Given(
  'application {string} is registered on doorway {string} with callbacks bounded to {string}',
  async function (this: E2EWorld, clientId: string, doorwayId: string, bound: string) {
    const base = doorwayBase(this, doorwayId);
    // A concrete callback inside the declared bound: `https://*.elohim.host/*`
    // -> `https://a2o-precondition.elohim.host/cb`.
    const probe = bound.replace('*.', 'a2o-precondition.').replace(/\/\*$/, '/cb');
    const res = await request(
      `${base}/auth/authorize?client_id=${encodeURIComponent(clientId)}` +
        `&redirect_uri=${encodeURIComponent(probe)}&response_type=code&state=precondition`,
      { method: 'GET' }
    );
    const body = await res.body.text();
    assert.notEqual(
      res.statusCode,
      400,
      `precondition failed: doorway "${doorwayId}" refused application "${clientId}" with a ` +
        `callback inside its declared bound "${bound}" (probed "${probe}"): ${body}`
    );
    assert.equal(
      res.statusCode,
      302,
      `expected a signed-out authorize inside the bound to redirect to login, got ` +
        `${res.statusCode}: ${body}`
    );
  }
);

Given(
  '{string} holds an open session on doorway {string}',
  async function (this: E2EWorld, _persona: string, doorwayId: string) {
    const base = doorwayBase(this, doorwayId);
    // A fresh identity per scenario: the flow mints codes against the logged-in
    // human, so a shared fixture would let one scenario redeem another's code.
    const identifier = `oauth-a2o-${randomUUID()}@local.mesh`;
    const password = `oauth-a2o-${randomUUID()}`;

    const reg = await request(`${base}/auth/register`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ identifier, password, displayName: 'OAuth A2O' }),
    });
    const regBody = await reg.body.text();
    assert.ok(
      reg.statusCode < 300,
      `register failed for the OAuth test human: ${reg.statusCode} ${regBody}`
    );

    const login = await request(`${base}/auth/login`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ identifier, password }),
    });
    const loginBody = await login.body.text();
    assert.ok(login.statusCode < 300, `login failed: ${login.statusCode} ${loginBody}`);

    const parsed = JSON.parse(loginBody) as {
      token?: string;
      humanId?: string;
      identifier?: string;
    };
    assert.ok(parsed.token, `login returned no token: ${loginBody}`);
    // The doorway names the human it resolved this session to, and on a
    // gateway-scoping doorway that is NOT the string typed above: the local part
    // is re-qualified with the doorway's own domain
    // (auth_routes.rs normalize_identifier), so `x@local.mesh` is kept as
    // `x@localhost` on the mesh and `x@alpha.elohim.host` on the fleet. Carrying
    // the requested spelling forward is how a scenario ends up asserting a name
    // no deployment actually uses -- see src/framework/doorway-identity.ts.
    assert.ok(
      parsed.identifier,
      `login named no human, so nothing downstream can check whose token this is: ${loginBody}`
    );

    (this as unknown as { __oauth: OAuthState }).__oauth = {
      identifier: parsed.identifier,
      humanId: parsed.humanId ?? '',
      token: parsed.token,
      state: `a2o-${randomUUID().slice(0, 8)}`,
      body: '',
      status: 0,
      location: null,
      code: null,
      tokenBody: '',
      tokenStatus: 0,
    };
  }
);

async function authorize(
  world: E2EWorld,
  doorwayId: string,
  clientId: string,
  redirectUri: string,
  withBearer: boolean
): Promise<void> {
  const s = oauth(world);
  const base = doorwayBase(world, doorwayId);
  const url =
    `${base}/auth/authorize?client_id=${encodeURIComponent(clientId)}` +
    `&redirect_uri=${encodeURIComponent(redirectUri)}` +
    `&response_type=code&state=${encodeURIComponent(s.state)}&scope=openid`;

  // undici's `request` does not follow redirects unless asked, and the 302 branch
  // IS the assertion here — following it would silently test the login page.
  const res = await request(url, {
    method: 'GET',
    headers: withBearer ? { authorization: `Bearer ${s.token}` } : {},
  });

  s.status = res.statusCode;
  s.body = await res.body.text();
  const loc = res.headers['location'];
  s.location = typeof loc === 'string' ? loc : null;

  // The authenticated branch answers 200 with the redirect target in the body;
  // the unauthenticated branch answers 302 with it in the Location header.
  let issuedTo: string | null = null;
  if (s.status === 200) {
    try {
      issuedTo = (JSON.parse(s.body) as { redirect_uri?: string }).redirect_uri ?? null;
    } catch {
      issuedTo = null;
    }
  }
  s.code = codeFrom(issuedTo);
}

When(
  "Miriam's doorway is asked to authorize {string} with callback {string}",
  async function (this: E2EWorld, clientId: string, redirectUri: string) {
    await authorize(this, 'alpha', clientId, redirectUri, true);
  }
);

When(
  'a signed-out browser asks doorway {string} to authorize {string} with callback {string}',
  async function (this: E2EWorld, doorwayId: string, clientId: string, redirectUri: string) {
    await authorize(this, doorwayId, clientId, redirectUri, false);
  }
);

Then(
  'the authorization response is refused with error {string}',
  function (this: E2EWorld, expected: string) {
    const s = oauth(this);
    assert.equal(
      s.status,
      400,
      `expected the authorization request to be refused with 400, got ${s.status}: ${s.body}`
    );
    const parsed = JSON.parse(s.body) as { error?: string };
    assert.equal(
      parsed.error,
      expected,
      `expected error "${expected}", got "${parsed.error}": ${s.body}`
    );
  }
);

Then('the authorization error preserves the state parameter', function (this: E2EWorld) {
  const s = oauth(this);
  const parsed = JSON.parse(s.body) as { state?: string };
  assert.equal(
    parsed.state,
    s.state,
    `the refusal dropped the state parameter (RFC-6749 §4.1.2.1): ${s.body}`
  );
});

Then('no authorization code is present in the response', function (this: E2EWorld) {
  const s = oauth(this);
  assert.equal(s.code, null, `a refused authorization still issued a code: ${s.body}`);
  assert.ok(
    !/[?&]code=/.test(s.body),
    `a refused authorization leaked a code into its body: ${s.body}`
  );
});

Then('the authorization response redirects to the login surface', function (this: E2EWorld) {
  const s = oauth(this);
  assert.equal(
    s.status,
    302,
    `expected an unauthenticated authorization request to 302 to the login surface, ` +
      `got ${s.status}: ${s.body}`
  );
  assert.ok(s.location, 'the 302 carried no Location header');
  assert.ok(
    s.location.includes('/threshold/login'),
    `expected the login surface in the redirect, got: ${s.location}`
  );
});

Then(
  'the login redirect preserves the client_id, redirect_uri, response_type and state',
  function (this: E2EWorld) {
    const s = oauth(this);
    assert.ok(s.location, 'no Location header to inspect');
    // A login bounce that drops these cannot resume the authorization afterwards.
    for (const param of ['client_id', 'redirect_uri', 'response_type', 'state']) {
      assert.ok(
        s.location.includes(`${param}=`),
        `the login redirect dropped "${param}" — the authorization cannot resume: ${s.location}`
      );
    }
    assert.ok(
      s.location.includes(encodeURIComponent(s.state)) || s.location.includes(s.state),
      `the login redirect dropped the state VALUE: ${s.location}`
    );
  }
);

Then(
  'an authorization code is issued to {string}',
  function (this: E2EWorld, expectedRedirect: string) {
    const s = oauth(this);
    assert.equal(
      s.status,
      200,
      `expected the authenticated ajax branch to answer 200 with a redirect_uri body, ` +
        `got ${s.status}: ${s.body}`
    );
    const issued = (JSON.parse(s.body) as { redirect_uri?: string }).redirect_uri;
    assert.ok(issued, `no redirect_uri in the authorization response: ${s.body}`);
    assert.ok(
      issued.startsWith(expectedRedirect),
      `the code was issued to "${issued}", not to the requested "${expectedRedirect}"`
    );
    assert.ok(s.code, `no code present in the issued redirect: ${issued}`);
  }
);

Then('the authorization response preserves the state parameter', function (this: E2EWorld) {
  const s = oauth(this);
  const issued = (JSON.parse(s.body) as { redirect_uri?: string }).redirect_uri ?? '';
  assert.ok(
    issued.includes(`state=${encodeURIComponent(s.state)}`) || issued.includes(`state=${s.state}`),
    `the issued redirect dropped the state parameter: ${issued}`
  );
});

async function exchange(world: E2EWorld, doorwayId: string, clientId: string): Promise<void> {
  const s = oauth(world);
  assert.ok(s.code, 'no authorization code to exchange — the authorize step issued none');
  const base = doorwayBase(world, doorwayId);
  const res = await request(`${base}/auth/token`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      grant_type: 'authorization_code',
      code: s.code,
      client_id: clientId,
      redirect_uri: 'https://app.elohim.host/callback',
    }),
  });
  s.tokenStatus = res.statusCode;
  s.tokenBody = await res.body.text();
}

When(
  'the authorization code is exchanged at doorway {string} for client {string}',
  async function (this: E2EWorld, doorwayId: string, clientId: string) {
    await exchange(this, doorwayId, clientId);
  }
);

When(
  'the same authorization code is exchanged again at doorway {string} for client {string}',
  async function (this: E2EWorld, doorwayId: string, clientId: string) {
    await exchange(this, doorwayId, clientId);
  }
);

Then('the token response carries an access token belonging to Miriam', function (this: E2EWorld) {
  const s = oauth(this);
  assert.ok(s.tokenStatus < 300, `token exchange failed: ${s.tokenStatus} ${s.tokenBody}`);
  const parsed = JSON.parse(s.tokenBody) as {
    access_token?: string;
    identifier?: string;
  };
  assert.ok(parsed.access_token, `no access_token in the token response: ${s.tokenBody}`);
  // The token must belong to the human who authorized, not merely be well-formed.
  assert.equal(
    parsed.identifier,
    s.identifier,
    `the exchanged token names "${parsed.identifier}", not Miriam, the human who ` +
      `authorized it ("${s.identifier}")`
  );
});

Then(
  'the token response is refused with error {string}',
  function (this: E2EWorld, expected: string) {
    const s = oauth(this);
    const parsed = JSON.parse(s.tokenBody) as { error?: string };
    assert.equal(
      parsed.error,
      expected,
      `expected the replayed code to be refused with "${expected}", got ` +
        `${s.tokenStatus} ${s.tokenBody}`
    );
  }
);
