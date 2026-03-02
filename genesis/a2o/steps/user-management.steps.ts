/**
 * User management step definitions — admin CRUD operations for hosted users.
 *
 * Matthew (admin) manages hosted users: list, view details, suspend,
 * update quotas, and verify permission enforcement.
 */

import { strict as assert } from 'node:assert';

import { When, Then } from '@cucumber/cucumber';

import { BrowserDevice } from '../src/framework/devices/browser-device.js';
import { E2EWorld } from '../src/framework/world.js';

// ---------------------------------------------------------------------------
// Types for admin endpoints (mirrors doorway Rust responses)
// ---------------------------------------------------------------------------

interface UserSummary {
  id: string;
  identifier: string;
  permissionLevel: string;
  isActive: boolean;
  storagePercent?: number;
  createdAt?: string;
}

interface UsersResponse {
  users: UserSummary[];
  total: number;
  page: number;
  totalPages: number;
}

interface UserDetailsResponse {
  id: string;
  identifier: string;
  permissionLevel: string;
  isActive: boolean;
  usage?: {
    storageBytes: number;
    projectionQueries: number;
    bandwidthBytes: number;
  };
  quota?: {
    storageLimit: number;
    dailyQueryLimit: number;
    dailyBandwidthLimit: number;
  };
}

interface MutationResponse {
  success: boolean;
  message?: string;
}

const ADMIN_USERS_PATH = '/admin/users';

// ---------------------------------------------------------------------------
// Admin HTTP helpers (extends DoorwayClient pattern)
// ---------------------------------------------------------------------------

async function adminGet<T>(device: BrowserDevice, path: string): Promise<T> {
  return device.client['get'](path);
}

async function adminPut<T>(device: BrowserDevice, path: string, body: unknown): Promise<T> {
  // Use the client's internal post method but with PUT semantics
  // For now, we use fetch directly since DoorwayClient only has get/post
  const { request } = await import('undici');
  const { statusCode, body: resBody } = await request(`${device.client.url}${path}`, {
    method: 'PUT',
    headers: {
      'content-type': 'application/json',
      authorization: `Bearer ${device.token}`,
    },
    body: JSON.stringify(body),
  });
  const text = await resBody.text();
  if (statusCode < 200 || statusCode >= 300) {
    throw new Error(`PUT ${path} returned ${statusCode}: ${text}`);
  }
  return JSON.parse(text) as T;
}

// ---------------------------------------------------------------------------
// List users
// ---------------------------------------------------------------------------

let lastUsersResponse: UsersResponse | undefined;

When('{word} queries the admin users list', async function (this: E2EWorld, humanName: string) {
  const human = this.getHuman(humanName);
  const device = human.devices[0] as BrowserDevice;
  assert.ok(device, `${humanName} has no device`);

  lastUsersResponse = await adminGet<UsersResponse>(device, ADMIN_USERS_PATH);
});

Then('the users list should contain at least {int} entries', function (minCount: number) {
  assert.ok(lastUsersResponse, 'No users response available');
  assert.ok(
    lastUsersResponse.users.length >= minCount,
    `Expected at least ${minCount} users but got ${lastUsersResponse.users.length}`
  );
});

Then("the users list should include {word}'s entry", function (this: E2EWorld, humanName: string) {
  assert.ok(lastUsersResponse, 'No users response available');
  const human = this.getHuman(humanName);
  const match = lastUsersResponse.users.find(u => u.identifier === human.credentials.identifier);
  assert.ok(match, `${humanName} (${human.credentials.identifier}) not found in users list`);
});

// ---------------------------------------------------------------------------
// View user details
// ---------------------------------------------------------------------------

let lastUserDetails: UserDetailsResponse | undefined;

When(
  '{word} views user details for {word}',
  async function (this: E2EWorld, adminName: string, targetName: string) {
    const admin = this.getHuman(adminName);
    const device = admin.devices[0] as BrowserDevice;
    assert.ok(device, `${adminName} has no device`);

    // First get the user ID from the list
    const listRes = await adminGet<UsersResponse>(device, ADMIN_USERS_PATH);
    const target = this.getHuman(targetName);
    const userEntry = listRes.users.find(u => u.identifier === target.credentials.identifier);
    assert.ok(userEntry, `${targetName} not found in users list`);

    lastUserDetails = await adminGet<UserDetailsResponse>(
      device,
      `/admin/users/${encodeURIComponent(userEntry.id)}`
    );
  }
);

Then(
  "the user details should include {word}'s identifier",
  function (this: E2EWorld, humanName: string) {
    assert.ok(lastUserDetails, 'No user details available');
    const human = this.getHuman(humanName);
    assert.strictEqual(lastUserDetails.identifier, human.credentials.identifier);
  }
);

Then('the user details should include usage stats', function () {
  assert.ok(lastUserDetails, 'No user details available');
  assert.ok(lastUserDetails.usage !== undefined, 'User details missing usage stats');
});

Then('the user details should include quota limits', function () {
  assert.ok(lastUserDetails, 'No user details available');
  assert.ok(lastUserDetails.quota !== undefined, 'User details missing quota limits');
});

// ---------------------------------------------------------------------------
// Suspend user
// ---------------------------------------------------------------------------

When(
  '{word} suspends user {string}',
  async function (this: E2EWorld, adminName: string, targetName: string) {
    const admin = this.getHuman(adminName);
    const device = admin.devices[0] as BrowserDevice;

    // Find user ID
    const listRes = await adminGet<UsersResponse>(device, ADMIN_USERS_PATH);
    const target = this.getHuman(targetName);
    const userEntry = listRes.users.find(u => u.identifier === target.credentials.identifier);
    assert.ok(userEntry, `${targetName} not found in users list`);

    const result = await adminPut<MutationResponse>(
      device,
      `/admin/users/${encodeURIComponent(userEntry.id)}/status`,
      { isActive: false }
    );
    this.contentIds.set('lastMutationSuccess', result.success ? 'true' : 'false');
  }
);

Then('the suspension should succeed', function (this: E2EWorld) {
  assert.strictEqual(this.contentIds.get('lastMutationSuccess'), 'true', 'User suspension failed');
});

// ---------------------------------------------------------------------------
// Update quota
// ---------------------------------------------------------------------------

When(
  "{word} updates {word}'s storage quota to {int} MB",
  async function (this: E2EWorld, adminName: string, targetName: string, limitMb: number) {
    const admin = this.getHuman(adminName);
    const device = admin.devices[0] as BrowserDevice;

    // Find user ID
    const listRes = await adminGet<UsersResponse>(device, ADMIN_USERS_PATH);
    const target = this.getHuman(targetName);
    const userEntry = listRes.users.find(u => u.identifier === target.credentials.identifier);
    assert.ok(userEntry, `${targetName} not found in users list`);

    const result = await adminPut<MutationResponse>(
      device,
      `/admin/users/${encodeURIComponent(userEntry.id)}/quota`,
      { storageLimit: limitMb * 1024 * 1024 }
    );
    this.contentIds.set('lastMutationSuccess', result.success ? 'true' : 'false');
  }
);

Then('the quota update should succeed', function (this: E2EWorld) {
  assert.strictEqual(this.contentIds.get('lastMutationSuccess'), 'true', 'Quota update failed');
});

Then(
  "{word}'s storage quota should be {int} MB",
  function (_humanName: string, expectedMb: number) {
    assert.ok(lastUserDetails, 'No user details available');
    assert.ok(lastUserDetails.quota, 'User details missing quota');
    const expectedBytes = expectedMb * 1024 * 1024;
    assert.strictEqual(
      lastUserDetails.quota.storageLimit,
      expectedBytes,
      `Expected quota ${expectedBytes} but got ${lastUserDetails.quota.storageLimit}`
    );
  }
);

// ---------------------------------------------------------------------------
// Permission enforcement
// ---------------------------------------------------------------------------

When(
  '{word} attempts to access the admin users endpoint',
  async function (this: E2EWorld, humanName: string) {
    const human = this.getHuman(humanName);
    const device = human.devices[0] as BrowserDevice;

    try {
      await adminGet<UsersResponse>(device, ADMIN_USERS_PATH);
      this.contentIds.set('adminAccessDenied', 'false');
    } catch {
      this.contentIds.set('adminAccessDenied', 'true');
    }
  }
);

Then('the request should be forbidden', function (this: E2EWorld) {
  assert.strictEqual(
    this.contentIds.get('adminAccessDenied'),
    'true',
    'Expected admin access to be denied but it succeeded'
  );
});
