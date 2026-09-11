/**
 * Step glue for `features/dataplane/doorway-humans-served.feature`
 * (`@concern:humans-served`) — S1 plan Task 4 of
 * `genesis/docs/superpowers/plans/2026-09-10-doorway-federation-three-reds-to-green-plan.md`.
 *
 * The count this story specifies (D3): the number of live `hosted-cell`
 * delegates-compute commitments THIS doorway's own pool has made, read from
 * `GET {doorwayUrl}/status.json` → `humansServed` (S2 Task 14 derives it;
 * until then this fails honestly naming that gap, never a fabricated pass).
 *
 * The people this story counts come from the Prologue's own caster
 * (`genesis/seeder/src/seed-hosted-humans.ts`, S1 Task 5), which registers
 * three hosted humans through the doorway's own `POST /auth/register` and
 * writes their roster to `${MESH_DIR}/prologue-hosted-humans.json`. This file
 * reads that roster rather than re-deriving names — it is fine for that path
 * not to exist yet; the read fails with a clear message naming it (Task 5
 * has not landed in this slice).
 *
 * Independent verification of "that count equals the number of live
 * hosted-cell commitments" reads each roster entry's commitment back from
 * jessica's storage — a household peer that is not this doorway's pool — via
 * `src/framework/fixtures/hosted-cell.ts`, the same Decision-1 notary-read
 * primitive `steps/ui/hosted-human.steps.ts` uses for station 7. Step files
 * are loaded by glob and do not import each other (Task 4 Interfaces note),
 * so the small amount of local state/helpers below is this file's own.
 */

import { strict as assert } from 'node:assert';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import {
  commitmentField,
  commitmentIsLive,
  readHostedCellCommitment,
} from '../../src/framework/fixtures/hosted-cell.js';

import type { StatusResponse } from '../../src/framework/api/doorway-client.js';
import type { E2EWorld } from '../../src/framework/world.js';

type Json = Record<string, unknown>;

/** UI is being built in parallel (S2 Task 14) — agreed here, used verbatim on both sides. */
const LANDING_HUMANS_SERVED_TESTID = 'landing-humans-served';
const THRESHOLD_LANDING_PATH = '/threshold';

const REPO_ROOT = resolve(process.cwd(), '..', '..');
const CASTER_ENTRY = resolve(REPO_ROOT, 'genesis/seeder/src/seed-hosted-humans.ts');

// ---------------------------------------------------------------------------
// Roster (Task 5's caster output)
// ---------------------------------------------------------------------------

interface HostedRosterEntry {
  identifier: string;
  doorwayId?: string;
  agentPubKey?: string;
  conductorId?: string;
  hostedCellGrantCid?: string;
  /**
   * Not in Task 5's own printed-line list (identifier/agentPubKey/conductorId/
   * hostedCellGrantCid) but needed for the LAST scenario here, which signs
   * back in as a roster human to close their own account through the
   * product path — a self-service close needs that human's own bearer, never
   * an admin's. The caster registers each human itself, so it is the only
   * place this password can come from; if it is absent from the roster file
   * this step fails naming exactly that gap rather than guessing one.
   */
  password?: string;
}

function rosterPath(): string {
  const meshDir = process.env['MESH_DIR'] ?? '/tmp/elohim-local-mesh';
  return `${meshDir}/prologue-hosted-humans.json`;
}

function loadRoster(): HostedRosterEntry[] {
  const path = rosterPath();
  let raw: string;
  try {
    raw = readFileSync(path, 'utf8');
  } catch {
    throw new Error(
      `no Prologue hosted-human roster at ${path} — run \`just mesh prologue\` first ` +
        "(it writes this file once S1 Task 5's seed-hosted-humans caster lands)."
    );
  }
  const parsed: unknown = JSON.parse(raw);
  const list = Array.isArray(parsed)
    ? parsed
    : Array.isArray((parsed as Json)['registrants'])
      ? ((parsed as Json)['registrants'] as unknown[])
      : Array.isArray((parsed as Json)['humans'])
        ? ((parsed as Json)['humans'] as unknown[])
        : undefined;
  assert.ok(
    list,
    `roster at ${path} is not an array (or {registrants:[...]} / {humans:[...]}): ` +
      raw.slice(0, 200)
  );
  return list as HostedRosterEntry[];
}

// ---------------------------------------------------------------------------
// Per-scenario state
// ---------------------------------------------------------------------------

interface HumansServedState {
  roster: HostedRosterEntry[];
  closed?: HostedRosterEntry;
  byDoorway: Map<string, StatusResponse>;
  lastRead?: string;
  recorded: Map<string, number>;
  landingDevice?: PlaywrightDevice;
  lastLandingDoorwayId?: string;
}

const stateByWorld = new WeakMap<E2EWorld, HumansServedState>();

function state(world: E2EWorld): HumansServedState {
  let value = stateByWorld.get(world);
  if (!value) {
    value = { roster: [], byDoorway: new Map(), recorded: new Map() };
    stateByWorld.set(world, value);
  }
  return value;
}

function statusFor(world: E2EWorld, doorwayId: string): StatusResponse {
  const status = state(world).byDoorway.get(doorwayId);
  assert.ok(status, `doorway "${doorwayId}"'s status has not been read yet.`);
  return status;
}

function humansServedOf(status: StatusResponse): unknown {
  return status['humansServed'];
}

async function readStatus(world: E2EWorld, doorwayId: string): Promise<StatusResponse> {
  const doorway = world.getDoorway(doorwayId);
  const status = await doorway.client.status();
  const s = state(world);
  s.byDoorway.set(doorwayId, status);
  s.lastRead = doorwayId;
  return status;
}

// ---------------------------------------------------------------------------
// Minimal authenticated HTTP, local to this file by convention (step files
// are loaded by glob and do not import each other's helpers).
// ---------------------------------------------------------------------------

async function authedPost(
  url: string,
  token: string,
  body: Json
): Promise<{ status: number; body: Json }> {
  const response = await request(url, {
    method: 'POST',
    headers: { authorization: `Bearer ${token}`, 'content-type': 'application/json' },
    body: JSON.stringify(body),
  });
  const text = await response.body.text();
  let parsed: Json = {};
  if (text) {
    try {
      parsed = JSON.parse(text) as Json;
    } catch {
      // leave empty — caller asserts on status first
    }
  }
  return { status: response.statusCode, body: parsed };
}

// ---------------------------------------------------------------------------
// Steps
// ---------------------------------------------------------------------------

Given(
  "the household mesh has cast its hosted humans through the doorway's own registration",
  function (this: E2EWorld) {
    const roster = loadRoster();
    assert.ok(roster.length > 0, `roster at ${rosterPath()} is empty — the Prologue cast nobody.`);
    state(this).roster = roster;
  }
);

Given(
  'doorway {string} is checked to be hosting no humans of its own',
  async function (this: E2EWorld, doorwayId: string) {
    const doorway = this.getDoorway(doorwayId);
    const status = await doorway.client.status();
    const served = humansServedOf(status);
    if (typeof served === 'number') {
      assert.equal(
        served,
        0,
        `doorway "${doorwayId}" reports humansServed=${served} — this scenario's premise is that ` +
          'it hosts nobody, checked rather than assumed.'
      );
      return;
    }
    // S2 Task 14 has not landed humansServed on this doorway yet — fall back to
    // the admin pipeline's own hosted-account tally rather than assume the
    // premise silently.
    const admin = await this.getAdminClient(doorway.url);
    const pipeline = await admin.adminPipeline();
    assert.equal(
      pipeline.hostedTotal,
      0,
      `doorway "${doorwayId}" admin pipeline reports hostedTotal=${pipeline.hostedTotal} — not ` +
        'hosting nobody.'
    );
  }
);

When('the status of doorway {string} is read', async function (this: E2EWorld, id: string) {
  await readStatus(this, id);
});

When('the status of doorway {string} is read again', async function (this: E2EWorld, id: string) {
  await readStatus(this, id);
});

Then('it names a humans-served count', function (this: E2EWorld) {
  const { lastRead } = state(this);
  assert.ok(lastRead, 'No doorway status has been read yet.');
  const status = statusFor(this, lastRead);
  assert.equal(
    typeof humansServedOf(status),
    'number',
    `status.json for "${lastRead}" carries no numeric humansServed: ${JSON.stringify(status)} — ` +
      'S2 Task 14 (derive humansServed) has not landed on this doorway yet.'
  );
});

Then(
  'that count equals the number of live hosted-cell commitments its pool has made',
  async function (this: E2EWorld) {
    const { lastRead, roster } = state(this);
    assert.ok(lastRead, 'No doorway status has been read yet.');
    const status = statusFor(this, lastRead);
    const served = humansServedOf(status);
    assert.equal(typeof served, 'number');
    assert.ok(roster.length > 0, 'No Prologue roster loaded — run the cast Given first.');
    let live = 0;
    for (const entry of roster) {
      if (!entry.hostedCellGrantCid) continue;
      const { status: httpStatus, body } = await readHostedCellCommitment(
        entry.hostedCellGrantCid
      );
      if (httpStatus === 200 && commitmentIsLive(body)) live += 1;
    }
    assert.equal(
      served,
      live,
      `doorway "${lastRead}" reports humansServed=${String(served)} but ${live} roster entries ` +
        'have a live hosted-cell commitment, read independently from a household peer that is ' +
        "not the doorway's pool."
    );
  }
);

Then('it names a humans-served count of {int}', function (this: E2EWorld, expected: number) {
  const { lastRead } = state(this);
  assert.ok(lastRead, 'No doorway status has been read yet.');
  assert.equal(humansServedOf(statusFor(this, lastRead)), expected);
});

Then('it does not answer with a dash', function (this: E2EWorld) {
  const { lastRead } = state(this);
  assert.ok(lastRead, 'No doorway status has been read yet.');
  const served = humansServedOf(statusFor(this, lastRead));
  assert.notEqual(served, null);
  assert.notEqual(served, undefined);
  assert.notEqual(String(served), '—');
  assert.equal(
    typeof served,
    'number',
    `humansServed is ${JSON.stringify(served)} — a dash/null means "cannot tell you", not a count.`
  );
});

Then("neither doorway's count includes a human hosted only by the other", function (this: E2EWorld) {
  const { roster } = state(this);
  const alpha = statusFor(this, 'alpha');
  const beta = statusFor(this, 'beta');
  assert.equal(
    humansServedOf(alpha),
    roster.length,
    `doorway "alpha" reports humansServed=${String(humansServedOf(alpha))} but the Prologue cast ` +
      `${roster.length} humans at alpha only.`
  );
  assert.equal(
    humansServedOf(beta),
    0,
    `doorway "beta" reports humansServed=${String(humansServedOf(beta))} — it must not count ` +
      "alpha's registrants."
  );
});

Then(
  'doorway {string} counts every human the mesh cast at doorway {string}',
  function (this: E2EWorld, hostId: string, castId: string) {
    assert.equal(
      hostId,
      castId,
      'This story only casts humans at one doorway; the two names must name the same one.'
    );
    const { roster } = state(this);
    const status = statusFor(this, hostId);
    assert.equal(
      humansServedOf(status),
      roster.length,
      `doorway "${hostId}" reports humansServed=${String(humansServedOf(status))} but the ` +
        `Prologue cast ${roster.length}.`
    );
  }
);

Then('doorway {string} counts none of them', function (this: E2EWorld, id: string) {
  const status = statusFor(this, id);
  assert.equal(
    humansServedOf(status),
    0,
    `doorway "${id}" reports humansServed=${String(humansServedOf(status))}, expected 0 (it never ` +
      'hosted the mesh\'s cast).'
  );
});

When(
  'a visitor opens the threshold landing of doorway {string}',
  async function (this: E2EWorld, id: string) {
    if (this.deviceMode !== 'playwright') return 'pending';
    const doorway = this.getDoorway(id);
    const base = doorway.url.replace(/\/+$/, '');
    // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
    const browser = await this.getBrowser();
    // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
    const device = new PlaywrightDevice(`humans-served-${id}`, base, base, browser);
    await device.init();
    this.onCleanup(async () => device.close());
    await device.navigate(`${base}${THRESHOLD_LANDING_PATH}`);
    await device.page.getByTestId(LANDING_HUMANS_SERVED_TESTID).waitFor({ state: 'visible' });
    const s = state(this);
    s.landingDevice = device;
    s.lastLandingDoorwayId = id;
    return undefined;
  }
);

Then(
  "the humans-served card shows the same number the doorway's status names",
  async function (this: E2EWorld) {
    const s = state(this);
    assert.ok(
      s.landingDevice && s.lastLandingDoorwayId,
      'No threshold landing has been opened yet.'
    );
    const text = (
      await s.landingDevice.page.getByTestId(LANDING_HUMANS_SERVED_TESTID).innerText()
    ).trim();
    const shown = Number(text);
    assert.ok(Number.isFinite(shown), `humans-served card shows "${text}", not a number.`);
    const status = statusFor(this, s.lastLandingDoorwayId);
    assert.equal(shown, humansServedOf(status));
  }
);

Then(
  'the humans-served card shows {int} rather than a dash',
  async function (this: E2EWorld, expected: number) {
    const s = state(this);
    assert.ok(s.landingDevice, 'No threshold landing has been opened yet.');
    const text = (
      await s.landingDevice.page.getByTestId(LANDING_HUMANS_SERVED_TESTID).innerText()
    ).trim();
    assert.notEqual(text, '—');
    assert.equal(Number(text), expected);
  }
);

Given(
  'the humans-served count of doorway {string} is recorded',
  async function (this: E2EWorld, id: string) {
    const status = await readStatus(this, id);
    const served = humansServedOf(status);
    assert.equal(typeof served, 'number');
    state(this).recorded.set(id, served as number);
  }
);

When(
  "one of those humans closes their account through the doorway's own close path",
  async function (this: E2EWorld) {
    const s = state(this);
    assert.ok(s.roster.length > 0, 'No Prologue roster loaded — run the cast Given first.');
    const entry = s.roster[0];
    assert.ok(
      entry.password,
      `roster entry "${entry.identifier}" carries no password — the Prologue caster (S1 Task 5) ` +
        'must record one so this scenario can sign back in as the human it created and close its ' +
        'own account through the product path.'
    );
    const doorwayId = entry.doorwayId ?? 'alpha'; // Task 5 casts every roster human at doorway A.
    const doorway = this.getDoorway(doorwayId);
    const base = doorway.url.replace(/\/+$/, '');
    const login = await doorway.client.login({
      identifier: entry.identifier,
      password: entry.password,
    });
    const result = await authedPost(`${base}/auth/close-account`, login.token, {
      confirmIdentifier: entry.identifier,
    });
    assert.equal(
      result.status,
      200,
      `POST /auth/close-account for "${entry.identifier}" returned ${result.status}: ` +
        JSON.stringify(result.body)
    );
    assert.equal(result.body['closed'], true);
    s.closed = entry;
  }
);

Then('the humans-served count is one lower than the recorded count', function (this: E2EWorld) {
  const s = state(this);
  const recorded = s.recorded.get('alpha');
  assert.ok(typeof recorded === 'number', 'No recorded humans-served baseline for doorway "alpha".');
  const current = humansServedOf(statusFor(this, 'alpha'));
  assert.equal(current, recorded - 1);
});

When('the household mesh casts that human again', function (this: E2EWorld) {
  const s = state(this);
  assert.ok(s.closed, 'No human has been closed yet in this scenario.');
  const doorway = this.getDoorway(s.closed.doorwayId ?? 'alpha');
  const result = spawnSync('npx', ['tsx', CASTER_ENTRY], {
    cwd: resolve(REPO_ROOT, 'genesis/seeder'),
    encoding: 'utf8',
    env: {
      ...process.env,
      DOORWAY_URL: doorway.url,
      MESH_DIR: process.env['MESH_DIR'] ?? '/tmp/elohim-local-mesh',
    },
  });
  assert.equal(
    result.status,
    0,
    "re-running the Prologue's hosted-human caster (S1 Task 5, seed-hosted-humans.ts) to restore " +
      `its cast failed: ${result.stderr || result.stdout}`
  );
});

Then('the humans-served count is what it was recorded as', function (this: E2EWorld) {
  const s = state(this);
  const recorded = s.recorded.get('alpha');
  assert.ok(typeof recorded === 'number', 'No recorded humans-served baseline for doorway "alpha".');
  const current = humansServedOf(statusFor(this, 'alpha'));
  assert.equal(current, recorded);
});
