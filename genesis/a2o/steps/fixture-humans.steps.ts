/**
 * Fixture human step definitions — login as pre-seeded humans from humans.json.
 *
 * Naming convention:
 *   "is logged in"  = fixture human (pre-seeded by genesis/seeder/src/seed-humans.ts)
 *   "is on doorway"  = ephemeral human (registered on-the-fly, see mode-aware.steps.ts)
 *
 * Credentials are derived deterministically from humans.json by
 * genesis/a2o/src/framework/fixtures/humans.ts (same algorithm as the seeder).
 */

import { strict as assert } from 'node:assert';

import { DataTable, Given, Then } from '@cucumber/cucumber';

import { ensureFixtureAdmin } from '../src/framework/api/admin-bootstrap.js';
import { BrowserDevice } from '../src/framework/devices/browser-device.js';
import { PlaywrightDevice } from '../src/framework/devices/playwright-device.js';
import { resolveDoorwayUrl } from '../src/framework/fixtures/household-mesh.js';
import {
  declaredStanding,
  getFixture,
  isHumanDeployed,
  signInStanding,
} from '../src/framework/fixtures/humans.js';
import { Human } from '../src/framework/human.js';
import { DoorwayDashboardPage } from '../src/framework/pages/index.js';
import { doorwayToAppUrl } from '../src/framework/utils/url.js';
import { DoorwayEntry, E2EWorld } from '../src/framework/world.js';

/**
 * Can this human log in on THIS lane?
 *
 * First, standing (`signInStanding`, ruled 2026-10-14): only a human with their
 * own peer runtime or one declared `hosted` may sign in at all. A scenario that
 * signs in as anyone else is misdeclared, so this throws rather than skipping;
 * a scenario about such a human asserts the refusal instead
 * (`human {string} is refused sign-in on doorway {string}`).
 *
 * Then, this run: deployments.json is the k8s roster of which humans have a
 * conductor POD, and on which pool. `isHumanDeployed` reads it two ways — a
 * hard `suspended` and a soft pool-skip (the human's only pool, shem, is
 * unavailable to this run). Either way the human has no runtime here, so the
 * login is skipped. The household lane gets no exemption: it registers only its
 * own peers' humans and the declared-hosted cast, so susan's or gertrude's
 * remote pod being down means she has nowhere to sign in on this run.
 */
function loginableOnThisLane(humanName: string): boolean {
  const standing = signInStanding(humanName);
  if (!standing.allowed) {
    throw new Error(
      `${humanName} has no sign-in standing (${standing.reason}). Sign in as a human with ` +
        `their own peer runtime or one declared hosted, or assert the refusal with ` +
        `'human "${humanName}" is refused sign-in on doorway "<id>"'.`
    );
  }
  return isHumanDeployed(humanName);
}

/**
 * A human with no sign-in standing is turned away by the doorway.
 *
 * The precondition is checked first: if the fixture HAS standing, the scenario
 * is misdeclared and this throws instead of passing on a doorway quirk. Then a
 * real login is attempted with the fixture's derived credentials; it must fail.
 *
 * Example:
 *   Then human "Charlie" is refused sign-in on doorway "alpha"
 */
Then(
  'human {string} is refused sign-in on doorway {string}',
  async function (this: E2EWorld, humanName: string, doorwayId: string) {
    const standing = signInStanding(humanName);
    if (standing.allowed) {
      throw new Error(
        `${humanName} HAS sign-in standing (${standing.reason}); a refusal scenario must name a human without it.`
      );
    }
    const fixture = getFixture(humanName);
    const doorway = resolveDoorwayFor(this, doorwayId);
    const device = new BrowserDevice(`${humanName}-browser`, doorway.url);
    let signedIn = false;
    try {
      await device.login({
        identifier: fixture.credentials.identifier,
        password: fixture.credentials.password,
      });
      signedIn = true;
    } catch {
      // Refused, which is what this step asserts.
    }
    if (signedIn) {
      throw new Error(
        `doorway "${doorwayId}" signed in ${humanName}, who has no standing (${standing.reason}). ` +
          `Something registered an account the seeder never casts.`
      );
    }
  }
);

/**
 * Login a pre-seeded fixture human via HTTP (BrowserDevice).
 *
 * Example:
 *   Given human "Matthew" is logged in on doorway "alpha"
 */
Given(
  'human {string} is logged in on doorway {string}',
  async function (this: E2EWorld, humanName: string, doorwayId: string) {
    if (!loginableOnThisLane(humanName)) {
      // Conductor suspended in deployments.json (e.g. shem outage). The
      // human's account may not be seeded; skip the scenario rather than
      // fail. Toggle "suspended": false in deployments.json to re-enable.
      return 'pending';
    }
    const fixture = getFixture(humanName);
    const doorway = resolveDoorwayFor(this, doorwayId);

    const human = new Human(humanName, fixture.credentials);
    const device = new BrowserDevice(`${humanName}-browser`, doorway.url);
    human.addDevice(device);

    // A steward is only an admin if the deployment was SEEDED with the admin
    // key; a mesh seeded without it holds a valid Matthew at AUTHENTICATED and
    // every admin scenario reds with a 403 that looks like product. Env-gated
    // no-op when this run holds no admin key. See api/admin-bootstrap.ts.
    if (fixture.isAdmin) await ensureFixtureAdmin(doorway.url, fixture.credentials.identifier);

    const auth = await device.login({
      identifier: fixture.credentials.identifier,
      password: fixture.credentials.password,
    });

    human.agentPubKey = auth.agentPubKey;
    human.humanId = auth.humanId;
    human.setToken(doorwayId, auth.token);

    this.addHuman(humanName, human);
  }
);

/**
 * The doorway this scenario named, even when its Background registered PEERS.
 *
 * A dataplane feature's Background often registers `peer "alpha-A"` rather than
 * `doorway "alpha"` — they are the same deployment seen from two sides, which
 * is exactly what content-sync's own comment says. A human still logs in at the
 * DOORWAY, so this step used to die with `Unknown doorway: "alpha"` on a
 * scenario whose substrate was sitting right there in the household manifest.
 *
 * Order matters and matches the `doorway {string} at {string}` step: what the
 * run was GIVEN wins, and the manifest is consulted only when nothing else
 * resolved — a manifest that DECLARES a doorway absent must not shadow one the
 * scenario already registered. When nothing resolves at all, the original
 * "Unknown doorway, known: …" error stands, so a typo still reads as a typo.
 */
function resolveDoorwayFor(world: E2EWorld, doorwayId: string): DoorwayEntry {
  const registered = world.doorways.get(doorwayId);
  if (registered) return registered;
  const url = resolveDoorwayUrl(doorwayId, []);
  if (!url) return world.getDoorway(doorwayId);
  return world.addDoorway(doorwayId, url);
}

/**
 * Login a pre-seeded fixture human with a mode-aware device (HTTP or Playwright).
 *
 * Example:
 *   Given human "Matthew" is logged in on doorway "alpha" with device
 */
Given(
  'human {string} is logged in on doorway {string} with device',
  async function (this: E2EWorld, humanName: string, doorwayId: string) {
    if (!loginableOnThisLane(humanName)) {
      return 'pending';
    }
    const fixture = getFixture(humanName);
    const doorway = resolveDoorwayFor(this, doorwayId);

    const human = new Human(humanName, fixture.credentials);

    if (this.deviceMode === 'playwright') {
      const browser = await this.getBrowser();
      const appUrl = doorwayToAppUrl(doorway.url);
      const device = new PlaywrightDevice(`${humanName}-pw`, appUrl, doorway.url, browser);
      await device.init();
      human.addDevice(device);

      const auth = await device.login({
        identifier: fixture.credentials.identifier,
        password: fixture.credentials.password,
      });

      human.agentPubKey = auth.agentPubKey;
      human.humanId = auth.humanId;
      human.setToken(doorwayId, auth.token);

      this.onCleanup(async () => device.close());
    } else {
      const device = new BrowserDevice(`${humanName}-browser`, doorway.url);
      human.addDevice(device);

      const auth = await device.login({
        identifier: fixture.credentials.identifier,
        password: fixture.credentials.password,
      });

      human.agentPubKey = auth.agentPubKey;
      human.humanId = auth.humanId;
      human.setToken(doorwayId, auth.token);
    }

    this.addHuman(humanName, human);
  }
);

/**
 * Login a pre-seeded fixture human on doorway-app with Playwright.
 * Injects auth via API and navigates to /threshold/dashboard instead of the elohim-app.
 *
 * Example:
 *   Given human "Matthew" is logged in on doorway-app "alpha" with device
 */
Given(
  'human {string} is logged in on doorway-app {string} with device',
  async function (this: E2EWorld, humanName: string, doorwayId: string) {
    if (!loginableOnThisLane(humanName)) {
      return 'pending';
    }
    const fixture = getFixture(humanName);
    const doorway = resolveDoorwayFor(this, doorwayId);

    const human = new Human(humanName, fixture.credentials);

    if (this.deviceMode === 'playwright') {
      // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
      const browser = await this.getBrowser();
      // doorway-app is served from the doorway URL itself (not the elohim-app URL)
      // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
      const device = new PlaywrightDevice(`${humanName}-pw`, doorway.url, doorway.url, browser);

      // Login via API first to get a valid JWT
      const auth = await device.client.login({
        identifier: fixture.credentials.identifier,
        password: fixture.credentials.password,
      });
      device.client.setToken(auth.token);

      // Init with extra HTTP headers — injects Authorization on every request.
      // This works even if the Angular auth interceptor isn't deployed yet.
      await device.init({
        extraHTTPHeaders: { Authorization: `Bearer ${auth.token}` },
      });
      human.addDevice(device);

      // Also set localStorage so Angular reads the token when the interceptor IS deployed
      await device.preloadLocalStorage('doorway_auth_token', auth.token);

      human.agentPubKey = auth.agentPubKey;
      human.humanId = auth.humanId;
      human.setToken(doorwayId, auth.token);

      // Navigate — auth is injected at the network level
      await device.page.goto(`${doorway.url}/threshold/dashboard`, { waitUntil: 'networkidle' });
      const dashboard = new DoorwayDashboardPage(device.page);
      await dashboard.waitForReady();

      this.onCleanup(async () => device.close());
    } else {
      const device = new BrowserDevice(`${humanName}-browser`, doorway.url);
      human.addDevice(device);

      const auth = await device.login({
        identifier: fixture.credentials.identifier,
        password: fixture.credentials.password,
      });

      human.agentPubKey = auth.agentPubKey;
      human.humanId = auth.humanId;
      human.setToken(doorwayId, auth.token);
    }

    this.addHuman(humanName, human);
  }
);

/**
 * Whether the doorway runs a cell for a signed-in human, read from their own
 * `GET /auth/account`: `conductorId` is set only when the doorway provisioned a
 * cell for them (the hosted arm). A human with their own runtime (device, node)
 * or the doorway's own operator holds an account with no conductor here.
 */
async function doorwayRunsCellFor(
  world: E2EWorld,
  doorwayId: string,
  humanName: string
): Promise<string | null> {
  const human = world.humans.get(humanName);
  const token = human?.getToken(doorwayId);
  assert.ok(token, `${humanName} is not signed in on doorway "${doorwayId}"`);
  const doorway = resolveDoorwayFor(world, doorwayId);
  const res = await fetch(`${doorway.url.replace(/\/$/, '')}/auth/account`, {
    headers: { authorization: `Bearer ${token}` },
  });
  const body = await res.text();
  assert.equal(res.status, 200, `GET /auth/account for ${humanName}: ${res.status} ${body}`);
  return (JSON.parse(body) as { conductorId?: string | null }).conductorId ?? null;
}

Then(
  'doorway {string} runs a cell for {string}',
  async function (this: E2EWorld, doorwayId: string, humanName: string) {
    const conductor = await doorwayRunsCellFor(this, doorwayId, humanName);
    assert.ok(conductor, `doorway "${doorwayId}" runs no cell for ${humanName}`);
  }
);

Then(
  'doorway {string} runs no cell for {string}',
  async function (this: E2EWorld, doorwayId: string, humanName: string) {
    const conductor = await doorwayRunsCellFor(this, doorwayId, humanName);
    assert.equal(conductor, null, `doorway "${doorwayId}" runs a cell for ${humanName} on ${conductor}`);
  }
);

/**
 * Shows, and checks against the fixture data, what each human in a scenario
 * declares: their stage, their household and whether they have a runtime of
 * their own. A mismatch fails here, before any sign-in, so a scenario's cause
 * is visible in the Gherkin and cannot silently drift from humans.json or
 * deployments.json.
 */
Given('these fixture humans declare:', function (table: DataTable) {
  const wrong: string[] = [];
  for (const row of table.hashes()) {
    const actual = declaredStanding(row['human']);
    for (const [column, key] of [
      ['stage', 'stage'],
      ['household', 'household'],
      ['own runtime', 'ownRuntime'],
    ] as const) {
      if (row[column] !== undefined && row[column] !== actual[key]) {
        wrong.push(`${row['human']}: ${column} is "${actual[key]}", not "${row[column]}"`);
      }
    }
  }
  assert.deepEqual(wrong, [], 'the fixture data says otherwise');
});
