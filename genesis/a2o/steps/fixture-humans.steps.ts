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

import { Given } from '@cucumber/cucumber';

import { ensureFixtureAdmin } from '../src/framework/api/admin-bootstrap.js';
import { BrowserDevice } from '../src/framework/devices/browser-device.js';
import { PlaywrightDevice } from '../src/framework/devices/playwright-device.js';
import {
  loadHouseholdMeshFixture,
  resolveDoorwayUrl,
} from '../src/framework/fixtures/household-mesh.js';
import {
  autoSkippedHumans,
  getFixture,
  isHumanDeployed,
} from '../src/framework/fixtures/humans.js';
import { Human } from '../src/framework/human.js';
import { DoorwayDashboardPage } from '../src/framework/pages/index.js';
import { doorwayToAppUrl } from '../src/framework/utils/url.js';
import { DoorwayEntry, E2EWorld } from '../src/framework/world.js';

/**
 * Can this human log in on THIS lane?
 *
 * deployments.json is the k8s roster: which humans have a conductor POD, and
 * on which pool. `isHumanDeployed` reads it two ways — a hard `suspended`
 * (the operator retired the pod AND the seeder skips the human, so no account
 * exists anywhere) and a soft pool-skip (the human's only pool, shem, is
 * unavailable to this run). The soft skip is a statement about pods, not about
 * accounts: a household lane that OWNS its substrate (`processControl: true`
 * in the fixture manifest — `just mesh prologue`) seeds every fixture human as
 * a hosted registration on doorway A's pool, matthew's conductor, so susan's
 * or gertrude's remote pod being down on shem says nothing about whether she
 * can log in here. On that lane the soft skip is lifted and the login is
 * attempted for real: it passes on its own evidence or reds honestly. A hard
 * suspension still holds everywhere — that human was never seeded.
 */
function loginableOnThisLane(humanName: string): boolean {
  if (isHumanDeployed(humanName)) return true;
  if (!autoSkippedHumans().includes(humanName.toLowerCase())) return false;
  try {
    return loadHouseholdMeshFixture().processControl === true;
  } catch {
    return false;
  }
}

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
