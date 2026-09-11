/**
 * Auth lifecycle step definitions — register, login, verify session, multi-human.
 *
 * These steps validate the Hosted Human agency phase: the basics of identity
 * on someone else's doorway. Matthew is the primary human but the steps are
 * generic so any fixture human can be used.
 */

import { strict as assert } from 'node:assert';
import { randomUUID } from 'node:crypto';

import { When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import { BrowserDevice } from '../src/framework/devices/browser-device.js';
import { namesHuman, expectedIdentifiersFor } from '../src/framework/doorway-identity.js';
import { getFixture } from '../src/framework/fixtures/humans.js';
import { Human } from '../src/framework/human.js';
import { E2EWorld } from '../src/framework/world.js';

import type { AuthResponse, HealthResponse } from '../src/framework/api/doorway-client.js';

// ---------------------------------------------------------------------------
// Ephemeral-human cleanup — product path first, admin soft-delete fallback
// ---------------------------------------------------------------------------

/** Shape of the `POST /auth/close-account` response (doorway-federation Task 9). */
interface CloseAccountResponse {
  closed?: boolean;
  cellUninstalled?: boolean;
  alreadyClosed?: boolean;
}

/**
 * Admin soft-delete — the pre-Task-9 cleanup path, kept as a fallback for a
 * build where `POST /auth/close-account` hasn't landed yet (404/405) or a
 * genuine request-level failure reaching it. Never throws.
 */
async function adminSoftDeleteFallback(
  world: E2EWorld,
  doorwayUrl: string,
  identifier: string,
  reason: 'route-absent' | 'request-error'
): Promise<void> {
  try {
    const admin = await world.getAdminClient(doorwayUrl);
    const list = await admin.adminListUsers({ search: identifier, limit: 1 });
    const match = list.users.find(u => u.identifier === identifier);
    if (!match) {
      console.warn(
        `[auth-lifecycle cleanup] admin fallback (${reason}) found no user matching ${identifier}`
      );
      return;
    }
    await admin.adminDeleteUser(match.id);
    console.warn(
      `[auth-lifecycle cleanup] ${identifier} closed via admin soft-delete fallback (${reason})`
    );
  } catch (err) {
    // Best-effort: cleanup must never throw out of the hook.
    console.warn(
      `[auth-lifecycle cleanup] admin fallback for ${identifier} failed: ${String(err)}`
    );
  }
}

/**
 * Close an ephemeral human's account through the product path:
 * `POST /auth/close-account` with the human's own bearer, falling back to
 * the admin soft-delete only when the route itself is absent (404/405).
 * A second close is expected to be harmless (idempotent per 05-leaving) —
 * the route answers `200 { alreadyClosed: true }`, never a rethrow here.
 *
 * Exported: shared with `steps/ui/doorway-portal-login.steps.ts`, whose
 * portal-registration Background needs the same product-path cleanup rather
 * than a second hand-maintained copy of it.
 */
export async function closeAccountCleanup(
  world: E2EWorld,
  doorwayUrl: string,
  identifier: string,
  humanToken: string
): Promise<void> {
  try {
    const { statusCode, body } = await request(`${doorwayUrl}/auth/close-account`, {
      method: 'POST',
      headers: {
        authorization: `Bearer ${humanToken}`,
        'content-type': 'application/json',
      },
      body: JSON.stringify({ confirmIdentifier: identifier }),
    });

    if (statusCode === 404 || statusCode === 405) {
      await adminSoftDeleteFallback(world, doorwayUrl, identifier, 'route-absent');
      return;
    }

    const text = await body.text();
    if (statusCode < 200 || statusCode >= 300) {
      // A genuine close-account failure (not route-absence) is left as
      // evidence rather than silently papered over by the fallback.
      console.warn(
        `[auth-lifecycle cleanup] POST /auth/close-account for ${identifier} returned ` +
          `${statusCode}, leaving the row as-is: ${text}`
      );
      return;
    }

    const result = JSON.parse(text) as CloseAccountResponse;
    console.warn(
      `[auth-lifecycle cleanup] ${identifier} closed via POST /auth/close-account ` +
        `(closed=${String(result.closed)} alreadyClosed=${String(result.alreadyClosed)} ` +
        `cellUninstalled=${String(result.cellUninstalled)})`
    );
  } catch (err) {
    // Network-level failure reaching the route at all (not a route-absent
    // 404/405) — still fall back so the household user count converges.
    console.warn(
      `[auth-lifecycle cleanup] close-account request for ${identifier} failed ` +
        `(${String(err)}); falling back to admin soft-delete`
    );
    await adminSoftDeleteFallback(world, doorwayUrl, identifier, 'request-error');
  }
}

// ---------------------------------------------------------------------------
// Registration (ephemeral human)
// ---------------------------------------------------------------------------

When(
  'a new human {string} registers on doorway {string}',
  async function (this: E2EWorld, humanName: string, doorwayId: string) {
    const doorway = this.getDoorway(doorwayId);
    const runId = randomUUID().slice(0, 8);
    const creds = {
      identifier: `e2e-${humanName.toLowerCase()}-${runId}@test.elohim.host`,
      password: `E2ePass!${runId}`,
      displayName: `${humanName} (E2E ${runId})`,
    };

    const human = new Human(humanName, creds);
    const device = new BrowserDevice(`${humanName}-browser`, doorway.url);
    human.addDevice(device);

    const auth = await device.register({
      identifier: creds.identifier,
      password: creds.password,
      displayName: creds.displayName,
    });

    human.agentPubKey = auth.agentPubKey;
    human.humanId = auth.humanId;
    human.setToken(doorwayId, auth.token);

    this.addHuman(humanName, human);

    // Store auth response for subsequent assertions
    this.contentIds.set('lastAuthResponse', JSON.stringify(auth));

    // Register cleanup to close the ephemeral human's account after the
    // scenario, through the product path (see closeAccountCleanup above).
    const identifier = creds.identifier;
    const doorwayUrl = doorway.url;
    const humanToken = auth.token;
    this.onCleanup(async () => {
      await closeAccountCleanup(this, doorwayUrl, identifier, humanToken);
    });
  }
);

// ---------------------------------------------------------------------------
// Auth response assertions
// ---------------------------------------------------------------------------

Then('the auth response should include a token', function (this: E2EWorld) {
  const auth = JSON.parse(this.contentIds.get('lastAuthResponse')!) as AuthResponse;
  assert.ok(auth.token, 'Auth response missing token');
  assert.ok(auth.token.length > 0, 'Auth token is empty');
});

Then('the auth response should include a humanId', function (this: E2EWorld) {
  const auth = JSON.parse(this.contentIds.get('lastAuthResponse')!) as AuthResponse;
  assert.ok(auth.humanId, 'Auth response missing humanId');
});

Then('the auth response should include an agentPubKey', function (this: E2EWorld) {
  const auth = JSON.parse(this.contentIds.get('lastAuthResponse')!) as AuthResponse;
  assert.ok(auth.agentPubKey, 'Auth response missing agentPubKey');
});

// ---------------------------------------------------------------------------
// Identity plumbing (Sprint 7 — JWT carries Holochain identity)
// ---------------------------------------------------------------------------

Then('the agentPubKey should not be empty', function (this: E2EWorld) {
  const auth = JSON.parse(this.contentIds.get('lastAuthResponse')!) as AuthResponse;
  assert.ok(auth.agentPubKey, 'Auth response missing agentPubKey');
  assert.ok(
    auth.agentPubKey.length > 10,
    `agentPubKey looks invalid (too short): "${auth.agentPubKey}"`
  );
});

Then('the identity should include an agent public key', function (this: E2EWorld) {
  assert.strictEqual(this.contentIds.get('lastMeSuccess'), 'true', '/auth/me failed');
  const me = JSON.parse(this.contentIds.get('lastMeResponse')!) as Record<string, unknown>;
  assert.ok(me.agentPubKey, 'Identity response missing agentPubKey');
});

Then(
  "the identity agent key should match {word}'s auth agentPubKey",
  function (this: E2EWorld, humanName: string) {
    const human = this.getHuman(humanName);
    assert.ok(human.agentPubKey, `${humanName} has no stored agentPubKey from auth`);

    assert.strictEqual(this.contentIds.get('lastMeSuccess'), 'true', '/auth/me failed');
    const me = JSON.parse(this.contentIds.get('lastMeResponse')!) as Record<string, unknown>;
    assert.strictEqual(
      me.agentPubKey,
      human.agentPubKey,
      `Identity agentPubKey doesn't match auth agentPubKey`
    );
  }
);

// ---------------------------------------------------------------------------
// Token and identifier verification
// ---------------------------------------------------------------------------

Then('{word} should have a valid token', function (this: E2EWorld, humanName: string) {
  const human = this.getHuman(humanName);
  // Check tokens map — at least one doorway token exists
  assert.ok(human.tokens.size > 0, `${humanName} has no tokens`);
  const token = [...human.tokens.values()][0];
  assert.ok(token && token.length > 0, `${humanName}'s token is empty`);
});

Then(
  "{word}'s identifier should be {string}",
  function (this: E2EWorld, humanName: string, expectedIdentifier: string) {
    const human = this.getHuman(humanName);
    assert.strictEqual(
      human.credentials.identifier,
      expectedIdentifier,
      `${humanName}'s identifier mismatch`
    );
  }
);

// ---------------------------------------------------------------------------
// Wrong password
// ---------------------------------------------------------------------------

When(
  'human {string} attempts login on doorway {string} with password {string}',
  async function (this: E2EWorld, humanName: string, doorwayId: string, password: string) {
    const doorway = this.getDoorway(doorwayId);
    const fixture = getFixture(humanName);

    const device = new BrowserDevice(`${humanName}-fail`, doorway.url);
    try {
      await device.login({
        identifier: fixture.credentials.identifier,
        password,
      });
      // If we get here, the login unexpectedly succeeded
      this.contentIds.set('loginFailed', 'false');
    } catch {
      this.contentIds.set('loginFailed', 'true');
    }
  }
);

Then('the login should fail', function (this: E2EWorld) {
  assert.strictEqual(
    this.contentIds.get('loginFailed'),
    'true',
    'Expected login to fail but it succeeded'
  );
});

// ---------------------------------------------------------------------------
// Identity verification (/auth/me)
// ---------------------------------------------------------------------------

When('{word} checks their identity', async function (this: E2EWorld, humanName: string) {
  const human = this.getHuman(humanName);
  const device = human.devices[0] as BrowserDevice;
  assert.ok(device, `${humanName} has no device`);

  try {
    const me = await device.client.me();
    this.contentIds.set('lastMeResponse', JSON.stringify(me));
    this.contentIds.set('lastMeSuccess', 'true');
  } catch {
    this.contentIds.set('lastMeSuccess', 'false');
  }
});

Then(
  "the identity should match {word}'s credentials",
  function (this: E2EWorld, humanName: string) {
    assert.strictEqual(this.contentIds.get('lastMeSuccess'), 'true', '/auth/me failed');
    const me = JSON.parse(this.contentIds.get('lastMeResponse')!) as Record<string, unknown>;
    const human = this.getHuman(humanName);
    // The doorway gateway-scopes identifiers, so what it ANSWERS is not always
    // what the scenario typed — see src/framework/doorway-identity.ts.
    assert.ok(
      namesHuman(me.identifier as string | undefined, human),
      `/auth/me named "${String(me.identifier)}", which is not ${humanName} on this ` +
        `doorway (expected ${expectedIdentifiersFor(human)})`
    );
  }
);

Then('the identity check should succeed', function (this: E2EWorld) {
  assert.strictEqual(
    this.contentIds.get('lastMeSuccess'),
    'true',
    '/auth/me should have succeeded'
  );
});

Then('the identity check should fail with unauthorized', function (this: E2EWorld) {
  assert.strictEqual(
    this.contentIds.get('lastMeSuccess'),
    'false',
    '/auth/me should have failed after logout'
  );
});

// ---------------------------------------------------------------------------
// Logout
// ---------------------------------------------------------------------------

When(
  '{word} logs out of doorway {string}',
  async function (this: E2EWorld, humanName: string, doorwayId: string) {
    const human = this.getHuman(humanName);
    const device = human.devices[0] as BrowserDevice;
    assert.ok(device, `${humanName} has no device`);

    await device.logout();
    human.tokens.delete(doorwayId);
  }
);

Then("{word}'s session should be cleared", function (this: E2EWorld, humanName: string) {
  const human = this.getHuman(humanName);
  const device = human.devices[0] as BrowserDevice;
  assert.ok(!device.isAuthenticated, `${humanName} should not be authenticated after logout`);
  assert.strictEqual(human.tokens.size, 0, `${humanName} should have no tokens after logout`);
});

// ---------------------------------------------------------------------------
// Re-login (after logout)
// ---------------------------------------------------------------------------

When(
  '{word} logs back in on doorway {string}',
  async function (this: E2EWorld, humanName: string, doorwayId: string) {
    const human = this.getHuman(humanName);
    const device = human.devices[0] as BrowserDevice;
    assert.ok(device, `${humanName} has no device`);

    const auth = await device.login({
      identifier: human.credentials.identifier,
      password: human.credentials.password,
    });

    human.agentPubKey = auth.agentPubKey;
    human.humanId = auth.humanId;
    human.setToken(doorwayId, auth.token);
  }
);

// ---------------------------------------------------------------------------
// Health check
// ---------------------------------------------------------------------------

let lastHealth: HealthResponse | undefined;

When('{word} checks doorway health', async function (this: E2EWorld, humanName: string) {
  const human = this.getHuman(humanName);
  const device = human.devices[0] as BrowserDevice;
  assert.ok(device, `${humanName} has no device`);

  lastHealth = await device.client.health();
});

Then('the doorway should be healthy', function () {
  assert.ok(lastHealth, 'No health check was performed');
  assert.ok(lastHealth.healthy, `Doorway is not healthy: status=${lastHealth.status}`);
});

// ---------------------------------------------------------------------------
// Multi-human distinct assertions
// ---------------------------------------------------------------------------

Then(
  'all {int} humans should have distinct tokens',
  function (this: E2EWorld, expectedCount: number) {
    assert.strictEqual(
      this.humans.size,
      expectedCount,
      `Expected ${expectedCount} humans but have ${this.humans.size}`
    );

    const tokens = new Set<string>();
    for (const [name, human] of this.humans) {
      const token = [...human.tokens.values()][0];
      assert.ok(token, `${name} has no token`);
      tokens.add(token);
    }

    assert.strictEqual(
      tokens.size,
      expectedCount,
      `Expected ${expectedCount} distinct tokens but got ${tokens.size}`
    );
  }
);

Then(
  'all {int} humans should have distinct humanIds',
  function (this: E2EWorld, expectedCount: number) {
    assert.strictEqual(this.humans.size, expectedCount);

    const ids = new Set<string>();
    for (const [name, human] of this.humans) {
      const id = human.humanId;
      assert.ok(id, `${name} has no humanId`);
      ids.add(id);
    }

    assert.strictEqual(
      ids.size,
      expectedCount,
      `Expected ${expectedCount} distinct humanIds but got ${ids.size}`
    );
  }
);
