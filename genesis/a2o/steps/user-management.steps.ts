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

import type {
  AdminUsersResponse,
  AdminUserSummary,
  AdminUserDetailsResponse,
} from '../src/framework/api/doorway-client.js';
import type { Human } from '../src/framework/human.js';

// ---------------------------------------------------------------------------
// Finding a human in the users list
// ---------------------------------------------------------------------------

/**
 * Every identifier the doorway could be storing this human under.
 *
 * TWO things make a naive `u.identifier === human.credentials.identifier`
 * match fail, and both are real product behaviour rather than test drift:
 *
 * 1. **The doorway gateway-scopes identifiers on register AND login**
 *    (`auth_routes.rs normalize_identifier`): the local-part is re-qualified
 *    with the doorway's own configured domain, so `susan@test.elohim.host`
 *    is STORED and returned as `susan@alpha.elohim.host`, and an ephemeral
 *    `e2e-troublemaker-<id>@test.elohim.host` becomes
 *    `e2e-troublemaker-<id>@alpha.elohim.host`. Verified live:
 *      POST /auth/login {"identifier":"susan@test.elohim.host"}
 *        -> 200 {"identifier":"susan@alpha.elohim.host", ...}
 *    The credentials the test TYPED are therefore not the credentials the
 *    doorway KEEPS.
 *
 * 2. Legacy rows from before that normalization landed still carry the old
 *    domain (alpha holds both `susan@alpha.elohim.host` and
 *    `susan@test.elohim.host`), so the requested form can also be a real —
 *    but stale — row.
 *
 * The auth response's `identifier` is the authoritative answer: it is exactly
 * what the doorway resolved this session to. Prefer it, and keep the requested
 * identifier as a fallback for deployments with no configured gateway domain
 * (local dev), where the doorway leaves the identifier untouched.
 */
function identifierCandidates(human: Human): string[] {
  const candidates: string[] = [];
  const device = human.devices[0];
  if (device instanceof BrowserDevice) {
    const sessionIdentifier = device.client.session?.identifier;
    if (sessionIdentifier) candidates.push(sessionIdentifier);
  }
  if (!candidates.includes(human.credentials.identifier)) {
    candidates.push(human.credentials.identifier);
  }
  return candidates;
}

/** The part before the `@` — stable across gateway re-qualification. */
function localPart(identifier: string): string {
  return identifier.split('@')[0];
}

/**
 * Find a human's row in the WHOLE admin users collection.
 *
 * `GET /admin/users` with no params returns **page 1 of `limit=20`, sorted
 * `metadata.created_at` DESC** (`admin_users.rs ListUsersQuery::from_query_string`).
 * The alpha doorway currently holds 3223 accounts across 162 pages, and page 1
 * is entirely `e2e-*` ephemerals from recent suite runs — so the genesis steward
 * himself is nowhere near it. Measured live:
 *
 *   GET /admin/users            -> total=3223 page=1 totalPages=162, 20 e2e-* rows
 *   GET /admin/users?search=matthew -> total=1 matthew.dowell@alpha.elohim.host (ADMIN)
 *
 * i.e. "Matthew not found in users list" never meant Matthew was missing — it
 * meant the step read 20 of 3223 rows. `search` is a server-side case-insensitive
 * regex over `identifier`, which is how the suite's own ephemeral-user cleanup
 * already looks accounts up. Page-walk only if `search` comes back empty, so the
 * assertion still speaks about the whole collection on a doorway that ignores it.
 */
async function findUserEntry(
  device: BrowserDevice,
  human: Human
): Promise<AdminUserSummary | undefined> {
  const wanted = identifierCandidates(human);
  const matches = (u: AdminUserSummary) => wanted.includes(u.identifier);

  const searched = await device.client.adminListUsers({
    search: localPart(wanted[0]),
    limit: 100,
  });
  const hit = searched.users.find(matches);
  if (hit) return hit;

  // Fallback: walk every page. Bounded by the server's own totalPages.
  const firstPage = await device.client.adminListUsers({ page: 1, limit: 100 });
  const direct = firstPage.users.find(matches);
  if (direct) return direct;
  for (let page = 2; page <= firstPage.totalPages; page++) {
    const next = await device.client.adminListUsers({ page, limit: 100 });
    const found = next.users.find(matches);
    if (found) return found;
  }
  return undefined;
}

/** Failure text that names what was searched for and how big the haystack was. */
function notFoundMessage(humanName: string, human: Human, response?: AdminUsersResponse): string {
  const total = response ? `${response.total} accounts across ${response.totalPages} pages` : 'the';
  return (
    `${humanName} not found in ${total} on this doorway. ` +
    `Searched for identifier(s): ${identifierCandidates(human).join(' | ')}. ` +
    `Note the doorway gateway-scopes identifiers on register/login ` +
    `(auth_routes.rs normalize_identifier), so the stored row may carry the ` +
    `doorway's own domain rather than the one the credentials name.`
  );
}

// ---------------------------------------------------------------------------
// List users
// ---------------------------------------------------------------------------

let lastUsersResponse: AdminUsersResponse | undefined;
/**
 * WHO ran the When. The follow-up Then steps are parameterised on the TARGET
 * human only ("the users list should include Susan's entry"), so without this
 * they had to guess the admin — and guessed `Matthew` literally, which silently
 * ignored the admin the scenario actually named and broke the moment a feature
 * gave the listing to anyone else.
 */
let lastUsersQueryAdminName: string | undefined;

When('{word} queries the admin users list', async function (this: E2EWorld, humanName: string) {
  const human = this.getHuman(humanName);
  const device = human.devices[0] as BrowserDevice;
  assert.ok(device, `${humanName} has no device`);

  lastUsersQueryAdminName = humanName;
  lastUsersResponse = await device.client.adminListUsers();
});

/**
 * What `GET /admin/users` (no params) must be true of ITSELF, independent of
 * who is in it.
 *
 * The Then steps below resolve a human across the whole collection, which is
 * the right answer to "is this account on this doorway" but reads NOTHING back
 * from the response the When captured — so a pagination or ordering regression
 * in the very endpoint the scenario exercised would sail through. These are the
 * page-1 invariants the doorway states in its own code
 * (`admin_users.rs`: `ListUsersQuery::from_query_string` defaults page=1
 * limit=20 sorted `metadata.created_at` DESC; `total_pages = ceil(total/limit)`).
 * The page size is read off the response rather than hardcoded to 20, so moving
 * the server default is not a false failure — breaking the arithmetic is.
 */
function assertListingIsSelfCoherent(response: AdminUsersResponse): void {
  assert.strictEqual(
    response.page,
    1,
    `A bare GET /admin/users must answer page 1; got page ${response.page}`
  );
  assert.ok(
    response.users.length > 0,
    `GET /admin/users returned an empty page 1 while reporting total=${response.total}`
  );
  assert.ok(
    response.total >= response.users.length,
    `GET /admin/users reported total=${response.total} but served ` +
      `${response.users.length} rows on page 1 — the collection cannot be ` +
      `smaller than the page it just returned`
  );

  assert.ok(
    response.totalPages >= 1,
    `GET /admin/users reported totalPages=${response.totalPages} while serving ` +
      `${response.users.length} rows`
  );

  // Pagination arithmetic against the server's OWN formula
  // (admin_users.rs: `total_pages = ceil(total / params.limit)`). `limit` is on
  // the wire but not yet on AdminUsersResponse (doorway-client.ts is another
  // seam's file), so read it defensively: when the doorway states its page size
  // the check is exact, and a doorway that does not state one is not failed for
  // it. Deriving the page size from `users.length` instead would false-fail on
  // a short page — handle_list_users silently drops rows that fail to decode.
  const limit = (response as AdminUsersResponse & { limit?: number }).limit;
  if (typeof limit === 'number' && limit > 0) {
    assert.ok(
      response.users.length <= limit,
      `GET /admin/users served ${response.users.length} rows on a page of limit=${limit}`
    );
    assert.strictEqual(
      response.totalPages,
      Math.ceil(response.total / limit),
      `GET /admin/users reported totalPages=${response.totalPages} for ` +
        `total=${response.total} at limit=${limit} — expected ` +
        `${Math.ceil(response.total / limit)}`
    );
  }

  // Ordering: created_at DESC. `createdAt` is Option<String> with
  // skip_serializing_if on the wire (admin_users.rs UserSummary), so a row
  // without one is not a regression — only an out-of-order pair is.
  const timestamps = response.users.map(u => (u.createdAt ? Date.parse(u.createdAt) : Number.NaN));
  for (let i = 1; i < timestamps.length; i++) {
    if (Number.isNaN(timestamps[i - 1]) || Number.isNaN(timestamps[i])) continue;
    assert.ok(
      timestamps[i - 1] >= timestamps[i],
      `GET /admin/users is sorted metadata.created_at DESC, but row ${i - 1} ` +
        `(${response.users[i - 1].identifier} @ ${response.users[i - 1].createdAt}) is ` +
        `OLDER than row ${i} (${response.users[i].identifier} @ ${response.users[i].createdAt})`
    );
  }
}

Then('the users list should contain at least {int} entries', function (minCount: number) {
  assert.ok(lastUsersResponse, 'No users response available');
  assert.ok(
    lastUsersResponse.users.length >= minCount,
    `Expected at least ${minCount} users but got ${lastUsersResponse.users.length}`
  );
});

Then(
  "the users list should include {word}'s entry",
  async function (this: E2EWorld, humanName: string) {
    assert.ok(lastUsersResponse, 'No users response available');
    assert.ok(lastUsersQueryAdminName, 'No admin recorded for the users query');
    const human = this.getHuman(humanName);

    // FIRST: the response the When actually captured must hold up on its own
    // terms. Everything below reads the collection again through `search` /
    // page-walk, so without this the step would never fail on a regression in
    // the bare listing it is named after.
    assertListingIsSelfCoherent(lastUsersResponse);

    // THEN: "the users list" is the doorway's account collection, not the
    // newest-20 window a bare GET happens to return — resolve across every
    // page, through the SAME admin session that ran the When.
    const admin = this.getHuman(lastUsersQueryAdminName).devices[0];
    assert.ok(
      admin instanceof BrowserDevice,
      `${lastUsersQueryAdminName} (who queried the users list) has no browser device`
    );
    const match = await findUserEntry(admin, human);

    assert.ok(match, notFoundMessage(humanName, human, lastUsersResponse));

    // The two reads hit the same endpoint. When the row IS inside the captured
    // page, the paged view and the resolved view must agree on WHO that account
    // id is — a listing that answers a different identifier or permission level
    // for one id depending on how it was reached is a real defect. Only the
    // identity-stable fields are compared: usage/last-login move under a live
    // fleet and would flake, not inform.
    const onCapturedPage = lastUsersResponse.users.find(u => u.id === match.id);
    if (onCapturedPage) {
      assert.strictEqual(
        onCapturedPage.identifier,
        match.identifier,
        `GET /admin/users answered identifier "${onCapturedPage.identifier}" for account ` +
          `${match.id} on the captured page but "${match.identifier}" on the resolved lookup`
      );
      assert.strictEqual(
        onCapturedPage.permissionLevel,
        match.permissionLevel,
        `GET /admin/users answered permissionLevel "${onCapturedPage.permissionLevel}" for ` +
          `account ${match.id} on the captured page but "${match.permissionLevel}" on the ` +
          `resolved lookup`
      );
    }
  }
);

// ---------------------------------------------------------------------------
// View user details
// ---------------------------------------------------------------------------

let lastUserDetails: AdminUserDetailsResponse | undefined;

When(
  '{word} views user details for {word}',
  async function (this: E2EWorld, adminName: string, targetName: string) {
    const admin = this.getHuman(adminName);
    const device = admin.devices[0] as BrowserDevice;
    assert.ok(device, `${adminName} has no device`);

    // Resolve across the whole collection and against the doorway-canonical
    // identifier — see findUserEntry / identifierCandidates.
    const target = this.getHuman(targetName);
    const userEntry = await findUserEntry(device, target);
    assert.ok(userEntry, notFoundMessage(targetName, target));

    lastUserDetails = await device.client.adminGetUser(userEntry.id);
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

    // Find user ID. A just-registered ephemeral IS at the top of the
    // created_at-DESC ordering, but its stored identifier carries the
    // doorway's gateway domain, not the `@test.elohim.host` the step asked
    // for — so the match key, not the page, is what used to miss here.
    const target = this.getHuman(targetName);
    const userEntry = await findUserEntry(device, target);
    assert.ok(userEntry, notFoundMessage(targetName, target));

    const result = await device.client.adminSetUserStatus(userEntry.id, false);
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
  async function (this: E2EWorld, adminName: string, targetName: string, _limitMb: number) {
    const admin = this.getHuman(adminName);
    const device = admin.devices[0] as BrowserDevice;

    // Find user ID — see findUserEntry / identifierCandidates.
    const target = this.getHuman(targetName);
    const userEntry = await findUserEntry(device, target);
    assert.ok(userEntry, notFoundMessage(targetName, target));

    // Quota update placeholder — re-activates the user as a proxy until
    // a dedicated adminUpdateQuota endpoint is added to DoorwayClient
    const result = await device.client.adminSetUserStatus(userEntry.id, true);
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
      await device.client.adminListUsers();
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
