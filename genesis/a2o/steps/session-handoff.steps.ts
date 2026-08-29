/**
 * Session handoff step definitions — cross-app session transfer between
 * elohim-app and doorway-app.
 *
 * Matthew logs in via elohim-app, gets a session transfer token, and uses
 * it to authenticate in doorway-app without re-entering credentials.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { DoorwayClient } from '../src/framework/api/doorway-client.js';
import { BrowserDevice } from '../src/framework/devices/browser-device.js';
import { namesHuman, expectedIdentifiersFor } from '../src/framework/doorway-identity.js';
import { E2EWorld } from '../src/framework/world.js';

// ---------------------------------------------------------------------------
// Session transfer token
// ---------------------------------------------------------------------------

When(
  '{word} requests a session transfer token',
  async function (this: E2EWorld, humanName: string) {
    const human = this.getHuman(humanName);
    const device = human.devices[0] as BrowserDevice;
    assert.ok(device, `${humanName} has no device`);

    const response = await device.client.sessionToken();
    this.contentIds.set('lastSessionToken', response.sessionToken);
    this.contentIds.set('lastSessionTokenExpiresAt', String(response.expiresAt));
  }
);

Given('{word} has a session transfer token', async function (this: E2EWorld, humanName: string) {
  const human = this.getHuman(humanName);
  const device = human.devices[0] as BrowserDevice;
  assert.ok(device, `${humanName} has no device`);

  const response = await device.client.sessionToken();
  this.contentIds.set('lastSessionToken', response.sessionToken);
  this.contentIds.set('lastSessionTokenExpiresAt', String(response.expiresAt));
});

Given(
  '{word} has an expired session transfer token',
  function (this: E2EWorld, _humanName: string) {
    // Use a fake token that would be expired on the server
    this.contentIds.set('lastSessionToken', 'expired-fake-token-000');
    this.contentIds.set('lastSessionTokenExpiresAt', '0');
  }
);

Then('the transfer token should be present', function (this: E2EWorld) {
  const token = this.contentIds.get('lastSessionToken');
  assert.ok(token, 'Session transfer token is missing');
  assert.ok(token.length > 0, 'Session transfer token is empty');
});

Then(
  'the transfer token should expire within {int} seconds',
  function (this: E2EWorld, maxSeconds: number) {
    const expiresAt = Number(this.contentIds.get('lastSessionTokenExpiresAt'));
    assert.ok(expiresAt > 0, 'Transfer token expiresAt is missing or zero');
    const now = Math.floor(Date.now() / 1000);
    const diff = expiresAt - now;
    // expiresAt is stamped by the DOORWAY's clock; `now` is the runner's. A
    // ≥1s cross-host skew reads as maxSeconds+1 (observed: "61s exceeds 60s"
    // on genesis#1092). Allow a small bounded skew — the scenario's intent is
    // "short-lived token", not sub-second cross-host clock agreement.
    const CLOCK_SKEW_ALLOWANCE_S = 2;
    assert.ok(diff > 0, `Transfer token already expired (expiresAt=${expiresAt}, now=${now})`);
    assert.ok(
      diff <= maxSeconds + CLOCK_SKEW_ALLOWANCE_S,
      `Transfer token expiry ${diff}s exceeds maximum ${maxSeconds}s (+${CLOCK_SKEW_ALLOWANCE_S}s skew allowance)`
    );
  }
);

// ---------------------------------------------------------------------------
// Session exchange
// ---------------------------------------------------------------------------

When(
  '{word} exchanges the session transfer token',
  async function (this: E2EWorld, humanName: string) {
    const human = this.getHuman(humanName);
    const device = human.devices[0] as BrowserDevice;
    const sessionToken = this.contentIds.get('lastSessionToken');
    assert.ok(sessionToken, 'No session transfer token available');

    const response = await device.client.exchangeSession(sessionToken);
    this.contentIds.set('exchangedJwt', response.token);
    this.contentIds.set('exchangedIdentifier', response.identifier);
    this.contentIds.set('lastExchangeSuccess', 'true');
  }
);

Then('the exchange should return a valid JWT', function (this: E2EWorld) {
  assert.strictEqual(this.contentIds.get('lastExchangeSuccess'), 'true');
  const jwt = this.contentIds.get('exchangedJwt');
  assert.ok(jwt, 'Exchanged JWT is missing');
  assert.ok(jwt.length > 0, 'Exchanged JWT is empty');
});

Then(
  '{word} should be able to verify identity with the new JWT',
  async function (this: E2EWorld, humanName: string) {
    const human = this.getHuman(humanName);
    const device = human.devices[0] as BrowserDevice;
    const jwt = this.contentIds.get('exchangedJwt');
    assert.ok(jwt, 'No exchanged JWT available');

    // Create a fresh client with the exchanged JWT
    const freshClient = new DoorwayClient(device.client.url);
    freshClient.setToken(jwt);
    const me = await freshClient.me();
    assert.ok(
      namesHuman(me.identifier, human),
      `the exchanged session names "${me.identifier}", which is not ${humanName} on this ` +
        `doorway (expected ${expectedIdentifiersFor(human)})`
    );
  }
);

// ---------------------------------------------------------------------------
// Single-use enforcement
// ---------------------------------------------------------------------------

When(
  '{word} attempts to exchange the same transfer token again',
  async function (this: E2EWorld, humanName: string) {
    const human = this.getHuman(humanName);
    const device = human.devices[0] as BrowserDevice;
    const sessionToken = this.contentIds.get('lastSessionToken');
    assert.ok(sessionToken, 'No session transfer token available');

    try {
      await device.client.exchangeSession(sessionToken);
      this.contentIds.set('secondExchangeFailed', 'false');
    } catch {
      this.contentIds.set('secondExchangeFailed', 'true');
    }
  }
);

Then('the second exchange should fail', function (this: E2EWorld) {
  assert.strictEqual(
    this.contentIds.get('secondExchangeFailed'),
    'true',
    'Expected second exchange to fail but it succeeded'
  );
});

// ---------------------------------------------------------------------------
// Expired token
// ---------------------------------------------------------------------------

When(
  '{word} attempts to exchange the expired transfer token',
  async function (this: E2EWorld, humanName: string) {
    const human = this.getHuman(humanName);
    const device = human.devices[0] as BrowserDevice;
    const sessionToken = this.contentIds.get('lastSessionToken');
    assert.ok(sessionToken, 'No session transfer token available');

    try {
      await device.client.exchangeSession(sessionToken);
      this.contentIds.set('expiredExchangeFailed', 'false');
    } catch {
      this.contentIds.set('expiredExchangeFailed', 'true');
    }
  }
);

Then('the exchange should fail with unauthorized', function (this: E2EWorld) {
  assert.strictEqual(
    this.contentIds.get('expiredExchangeFailed'),
    'true',
    'Expected expired token exchange to fail'
  );
});

// ---------------------------------------------------------------------------
// Full handoff flow
// ---------------------------------------------------------------------------

When(
  '{word} opens the doorway-app with the transfer token',
  async function (this: E2EWorld, humanName: string) {
    const sessionToken = this.contentIds.get('lastSessionToken');
    assert.ok(sessionToken, 'No session transfer token available');

    const human = this.getHuman(humanName);
    const device = human.devices[0] as BrowserDevice;

    // Simulate doorway-app exchanging the session token on load
    const response = await device.client.exchangeSession(sessionToken);

    // Store the new JWT as the doorway-app would in localStorage
    this.contentIds.set('doorwayAppJwt', response.token);
    this.contentIds.set('doorwayAppIdentifier', response.identifier);
  }
);

Then(
  "the doorway-app account endpoint should return {word}'s account",
  async function (this: E2EWorld, humanName: string) {
    const jwt = this.contentIds.get('doorwayAppJwt');
    assert.ok(jwt, 'No doorway-app JWT available');

    const human = this.getHuman(humanName);
    const device = human.devices[0] as BrowserDevice;

    // Create a client simulating doorway-app with the exchanged token
    const appClient = new DoorwayClient(device.client.url);
    appClient.setToken(jwt);
    const account = await appClient.account();

    assert.ok(account, 'Account endpoint returned nothing');
    assert.ok(
      namesHuman(account.identifier, human),
      `the doorway-app account is "${account.identifier}", which is not ${humanName} on this ` +
        `doorway (expected ${expectedIdentifiersFor(human)})`
    );
    this.contentIds.set('doorwayAppAccountIdentifier', account.identifier);
  }
);

/**
 * The handoff must not RENAME the human.
 *
 * This step used to read `the account identifier should be
 * "matthew.dowell@alpha.elohim.host"` — a fleet hostname hardcoded into a
 * scenario that also runs against the household mesh, where the same human is
 * `matthew.dowell@localhost`. It could therefore only ever be green in one
 * deployment, and what it was really reaching for is deployment-independent:
 * the account the doorway-app sees is named EXACTLY as the doorway named it when
 * it issued the session, so nothing in the exchange re-qualifies or downgrades
 * the identifier. That is a strictly stronger claim than the previous step's
 * (which only asks that it be one of the names for this human) and it needs no
 * knowledge of the doorway's domain convention.
 */
Then(
  'the account identifier is exactly the one the doorway issued at sign-in',
  function (this: E2EWorld) {
    const identifier = this.contentIds.get('doorwayAppAccountIdentifier');
    const issued = this.contentIds.get('doorwayAppIdentifier');
    assert.ok(issued, 'no identifier was recorded when the doorway issued the session');
    assert.strictEqual(
      identifier,
      issued,
      `the handoff renamed the human: the doorway issued "${issued}" but the doorway-app ` +
        `account is "${identifier}"`
    );
  }
);

Then(
  "{word}'s identifier should match across both apps",
  function (this: E2EWorld, humanName: string) {
    const human = this.getHuman(humanName);
    const doorwayIdentifier = this.contentIds.get('doorwayAppIdentifier');
    assert.ok(
      namesHuman(doorwayIdentifier, human),
      `doorway-app names the handed-off human "${doorwayIdentifier}", which is not ` +
        `${humanName} on this doorway (expected ${expectedIdentifiersFor(human)})`
    );
  }
);
