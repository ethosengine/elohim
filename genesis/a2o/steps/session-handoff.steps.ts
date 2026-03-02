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
    assert.ok(diff > 0, `Transfer token already expired (expiresAt=${expiresAt}, now=${now})`);
    assert.ok(diff <= maxSeconds, `Transfer token expiry ${diff}s exceeds maximum ${maxSeconds}s`);
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
    assert.strictEqual(me.identifier, human.credentials.identifier);
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
    assert.strictEqual(account.identifier, human.credentials.identifier);
    this.contentIds.set('doorwayAppAccountIdentifier', account.identifier);
  }
);

Then(
  'the account identifier should be {string}',
  function (this: E2EWorld, expectedIdentifier: string) {
    const identifier = this.contentIds.get('doorwayAppAccountIdentifier');
    assert.strictEqual(identifier, expectedIdentifier);
  }
);

Then(
  "{word}'s identifier should match across both apps",
  function (this: E2EWorld, humanName: string) {
    const human = this.getHuman(humanName);
    const doorwayIdentifier = this.contentIds.get('doorwayAppIdentifier');
    assert.strictEqual(
      doorwayIdentifier,
      human.credentials.identifier,
      'Identifiers do not match across elohim-app and doorway-app'
    );
  }
);
