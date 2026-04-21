/**
 * Conductor admin reachability regression guard.
 *
 * Watches for the fingerprint family that was active during the
 * 2026-04-21 consolidated-pod migration:
 *   - 75ca9eb6e5dc  "Admin WebSocket connect to … :4444 … Connection refused"
 *   - 68407ba1be5a  legacy-human admin refused via old port derivation
 *
 * Root cause: forwarder bind conflicted with conductor's localhost bind.
 * Fixed by routing forwarder through :8444/:8445 externally. If this
 * scenario fails with "Connection refused" in the register body, the
 * forwarder regressed — check elohim-storage's peer-policy bind addrs
 * and Adam's manifest/Service targetPort mappings.
 */

import { strict as assert } from 'node:assert';

import { When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import { E2EWorld } from '../src/framework/world.js';

// eslint-disable-next-line sonarjs/no-hardcoded-passwords -- a2o fixture credential, matches genesis/seeder/src/seed-humans.ts
const FIXTURE_PASSWORD = 'Test2026!';

When(
  'a fresh human is registered on doorway {string}',
  async function (this: E2EWorld, doorwayId: string) {
    const doorway = this.getDoorway(doorwayId);
    const unique = `e2e-admin-reach-${Date.now()}`;
    const { statusCode, body } = await request(`${doorway.url}/auth/register`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({
        identifier: `${unique}@test.elohim.host`,
        password: FIXTURE_PASSWORD,
        displayName: unique,
      }),
    });
    const text = await body.text();
    this.contentIds.set('adminReach:status', String(statusCode));
    this.contentIds.set('adminReach:body', text);
  }
);

Then('the register response is 200 with a valid agent identifier', function (this: E2EWorld) {
  const status = this.contentIds.get('adminReach:status');
  const body = this.contentIds.get('adminReach:body') ?? '';
  assert.equal(status, '200', `expected 200, got ${status}: ${body}`);
  const parsed = JSON.parse(body) as Record<string, unknown>;
  assert.ok(
    typeof parsed['agentPubKey'] === 'string' && parsed['agentPubKey'].length > 10,
    `expected agentPubKey in register response, got: ${body}`
  );
});

Then('the register response does not mention a refused admin socket', function (this: E2EWorld) {
  const body = this.contentIds.get('adminReach:body') ?? '';
  assert.ok(
    !/connect refused|Connection refused|os error 111/i.test(body),
    `register response mentions a refused admin socket (forwarder regression?): ${body}`
  );
});
