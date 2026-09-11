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
 * (`genesis/seeder/src/seed-hosted-humans.ts`, S1 Task 5, landed 2026-09-11),
 * which registers three hosted humans through the doorway's own
 * `POST /auth/register` and writes their roster to
 * `${MESH_DIR}/prologue-hosted-humans.json` as `{ doorwayUrl, generatedAt,
 * registrants: RosterEntry[] }`. This file reads that roster rather than
 * re-deriving names, and mirrors `RosterEntry`'s shape locally below (a2o step
 * files don't import the seeder). A registrant whose `result` is not
 * `registered`/`exists` (a soft-failed or no-pool row) cast nobody —
 * `liveRosterEntries()` is the one place that filter lives. If the roster
 * file itself is absent (`just mesh prologue` has not run), the read fails
 * with a clear message naming that path.
 *
 * Independent verification of "that count equals the number of live
 * hosted-cell commitments" reads each registrant's commitment back from
 * jessica's storage — a household peer that is not this doorway's pool — via
 * `src/framework/fixtures/hosted-cell.ts`, the same Decision-1 notary-read
 * primitive `steps/ui/hosted-human.steps.ts` uses for station 7. Step files
 * are loaded by glob and do not import each other (Task 4 Interfaces note),
 * so the small amount of local state/helpers below is this file's own.
 *
 * CORRECTION (2026-09-11, household run 20260911T041510Z-faca0d95): the
 * assertions below were written before the household lane's Prologue
 * actually provisioned real cells for the household mesh's whole hosted
 * cast. Registering through `/auth/register` now PROVISIONS a cell, so the
 * lane's Prologue casts TWO populations at doorway A: the 3
 * `prologue-hosted-*` registrants this file's roster tracks, AND the
 * standing household cast (`HOUSEHOLD_HOSTED_CAST` in
 * `genesis/seeder/src/seed-humans.ts`, 14 names as of 2026-09-11) that
 * `just mesh prologue` also registers there. A doorway's `humansServed`
 * legitimately counts both — the earlier `== 3` / `== live.length` (roster
 * only) equalities assumed a world where only the roster caster provisioned
 * anything, which stopped being true when `seed-humans.ts` started
 * provisioning real cells too.
 *
 * `seed-humans.ts` writes NO roster/manifest file of its own (grepped: no
 * `MESH_DIR` read, no `writeFileSync` anywhere in that script), and the
 * household fixture manifest `hc-mesh-prologue.sh` writes to
 * `${MESH_DIR}/household-fixture.json` carries no `humans` field either (it
 * names storage peers and doorway URLs only — see
 * `app/elohim-app/scripts/hc-mesh-prologue.sh` §6). The only source that
 * exists on disk after a Prologue naming this cast is the hardcoded
 * `HOUSEHOLD_HOSTED_CAST` name list itself in `seed-humans.ts` (not
 * exported, and a2o step files don't import the seeder — same reason the
 * roster's `RosterEntry` shape is mirrored below rather than imported), so
 * that list is mirrored here too. Each name's live hosted-cell grant cid is
 * not recorded anywhere on disk (that script never captures one), so it is
 * read the way any authenticated caller reads it: log in as that persona
 * (credentials from the a2o framework's own `fixtureCredentials()`, which
 * already derives identically to `seed-humans.ts::deriveCredentials()`) and
 * read `GET /auth/account`'s `hostedCellGrantCid`.
 */

import { strict as assert } from 'node:assert';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import { DoorwayClient } from '../../src/framework/api/doorway-client.js';
import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import {
  commitmentIsLive,
  readHostedCellCommitment,
} from '../../src/framework/fixtures/hosted-cell.js';
import { fixtureCredentials } from '../../src/framework/fixtures/humans.js';

import type { AccountResponse, StatusResponse } from '../../src/framework/api/doorway-client.js';
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

/**
 * Byte-for-byte the wire shape `genesis/seeder/src/seed-hosted-humans.ts`
 * writes (S1 Task 5, landed 2026-09-11) — `RosterEntry` there, mirrored here
 * rather than imported because a2o step files are loaded by glob and the
 * seeder is a standalone script, not a package this workspace resolves.
 * `'-'` is that script's own sentinel for "no live value" (a soft-failed or
 * no-pool registrant), not `undefined` — every reader here must check for it.
 */
type RosterOutcome = 'registered' | 'exists' | 'no-pool' | 'unreachable' | 'failed';

interface HostedRosterEntry {
  identifier: string;
  displayName: string;
  doorwayUrl: string;
  intendedPool: string;
  result: RosterOutcome;
  agentPubKey: string;
  conductorId: string;
  hostedCellGrantCid: string;
  error?: string;
}

/** The seeder's own fixed credential for every roster registrant (DEFAULT_PASSWORD there). */
// eslint-disable-next-line sonarjs/no-hardcoded-passwords -- test fixture credential, not production
const ROSTER_PASSWORD = 'Prologue2026!';

function rosterPath(): string {
  // eslint-disable-next-line sonarjs/publicly-writable-directories -- local dev mesh scratch dir
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
        '(seed-hosted-humans.ts writes this file).'
    );
  }
  const parsed: unknown = JSON.parse(raw);
  /* eslint-disable sonarjs/no-nested-conditional -- {registrants:[...]} vs bare-array fallback */
  const list = Array.isArray((parsed as Json)['registrants'])
    ? ((parsed as Json)['registrants'] as unknown[])
    : Array.isArray(parsed)
      ? parsed
      : undefined;
  /* eslint-enable sonarjs/no-nested-conditional */
  assert.ok(
    list,
    `roster at ${path} is not {registrants:[...]} (or a bare array): ${raw.slice(0, 200)}`
  );
  return list as HostedRosterEntry[];
}

/** Entries the doorway actually registered — a soft-failed/no-pool row casts nobody. */
function liveRosterEntries(roster: HostedRosterEntry[]): HostedRosterEntry[] {
  return roster.filter(entry => entry.result === 'registered' || entry.result === 'exists');
}

/** A live entry's grant cid, or `undefined` when this doorway has no pool-compute wiring yet. */
function grantCidOf(entry: HostedRosterEntry): string | undefined {
  return entry.hostedCellGrantCid && entry.hostedCellGrantCid !== '-'
    ? entry.hostedCellGrantCid
    : undefined;
}

// ---------------------------------------------------------------------------
// Household cast — the OTHER population doorway A's Prologue casts (see the
// module doc's CORRECTION note). Mirrored from `HOUSEHOLD_HOSTED_CAST` in
// `genesis/seeder/src/seed-humans.ts` (14 names as of 2026-09-11) — the only
// source on disk naming this cast after a Prologue run (neither
// `seed-humans.ts` nor `${MESH_DIR}/household-fixture.json` write a
// humans/roster record of it). Cast only at doorway A, same as the roster.
// ---------------------------------------------------------------------------

const HOUSEHOLD_HOSTED_CAST: readonly string[] = [
  'Matthew',
  'Susan',
  'James',
  'Gertrude',
  'Maria',
  'Ronald',
  'Charlie',
  'Sam',
  'Dr. Dolittle',
  'Jessica',
  'Terrance',
  'Miriam',
  'Ezra',
  'Levi',
];

/**
 * This household-cast member's live hosted-cell grant cid, read the only way
 * it is obtainable off disk: log in as them (credentials from the a2o
 * framework's own `fixtureCredentials()`, which already matches
 * `seed-humans.ts::deriveCredentials()`) and read `GET /auth/account`. A
 * fresh `DoorwayClient` is used per name so this never touches the world's
 * shared per-doorway client's session. A login or account read that fails
 * (the persona isn't actually registered on this doorway, or the doorway
 * doesn't recognize it) answers `undefined` — a missing grant, not a thrown
 * error, exactly like a roster entry's `'-'` sentinel.
 */
async function castMemberGrantCid(doorwayUrl: string, name: string): Promise<string | undefined> {
  let identifier: string;
  let password: string;
  try {
    ({ identifier, password } = fixtureCredentials(name));
  } catch {
    return undefined;
  }
  const client = new DoorwayClient(doorwayUrl);
  try {
    await client.login({ identifier, password });
    const account: AccountResponse = await client.account();
    const cid = account.hostedCellGrantCid;
    if (!cid) return undefined;
    return cid;
  } catch {
    return undefined;
  }
}

/**
 * The full population this scenario's Given actually casts at `doorwayId`:
 * the roster's live entries (already loaded) PLUS the household cast (looked
 * up live, since it carries no on-disk roster). For each candidate with a
 * grant cid, reads that commitment back from jessica's storage — the
 * non-pool peer (Decision 1) — never trusting the doorway's own claim.
 *
 * `minted` counts every candidate that carries a grant cid at all (the
 * doorway believes it made a promise); `live` counts only those whose
 * commitment reads back LIVE off-pool. The gap between the two is the named
 * storage defect (a promise minted but not yet readable off-pool), not a
 * count mismatch — callers report the two numbers separately so the two
 * failure classes are never conflated in one message.
 */
async function hostedCellPopulationCheck(
  world: E2EWorld,
  doorwayId: string
): Promise<{ attempted: number; minted: number; live: number }> {
  const doorwayUrl = world.getDoorway(doorwayId).url;
  const roster = state(world).roster;
  const rosterLive = liveRosterEntries(roster);
  const rosterCids = rosterLive.map(grantCidOf).filter((c): c is string => Boolean(c));

  const castCids = (
    await Promise.all(HOUSEHOLD_HOSTED_CAST.map(async name => castMemberGrantCid(doorwayUrl, name)))
  ).filter((c): c is string => Boolean(c));

  const allCids = [...rosterCids, ...castCids];
  const liveFlags = await Promise.all(
    allCids.map(async cid => {
      const { status, body } = await readHostedCellCommitment(cid);
      return status === 200 && commitmentIsLive(body);
    })
  );

  return {
    attempted: rosterLive.length + HOUSEHOLD_HOSTED_CAST.length,
    minted: allCids.length,
    live: liveFlags.filter(Boolean).length,
  };
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

    const { attempted, minted, live } = await hostedCellPopulationCheck(this, lastRead);

    if (live !== minted) {
      // The named storage defect (S1 plan, 2026-09-11): a hosted-cell grant
      // this doorway believes it minted does not yet read back LIVE from a
      // household peer that is not the doorway's own pool. Distinct from a
      // plain count mismatch — see the module doc's CORRECTION note.
      assert.fail(
        `doorway "${lastRead}" reports humansServed=${String(served)}; independently, ${minted} of ` +
          `${attempted} hosted registrants (the roster's ${liveRosterEntries(roster).length} plus the ` +
          `${HOUSEHOLD_HOSTED_CAST.length}-name household cast) carry a hosted-cell grant cid, but only ` +
          `${live} of those ${minted} read back LIVE from jessica's storage (the non-pool peer) — this ` +
          'is the off-pool read defect, not a count mismatch.'
      );
    }

    assert.equal(
      served,
      live,
      `doorway "${lastRead}" reports humansServed=${String(served)} but ${live} registrants (of ` +
        `${attempted} attempted: the roster's ${liveRosterEntries(roster).length} plus the ` +
        `${HOUSEHOLD_HOSTED_CAST.length}-name household cast) have a live hosted-cell commitment, read ` +
        "independently from a household peer that is not the doorway's pool."
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

Then(
  "neither doorway's count includes a human hosted only by the other",
  function (this: E2EWorld) {
    const live = liveRosterEntries(state(this).roster);
    // Everything this scenario's Given casts at alpha: the roster's live
    // entries plus the household cast (see the module doc's CORRECTION note —
    // both are real, provisioned registrants there now, not just the roster).
    const castAtAlpha = live.length + HOUSEHOLD_HOSTED_CAST.length;
    const alpha = statusFor(this, 'alpha');
    const beta = statusFor(this, 'beta');
    const alphaServed = humansServedOf(alpha);
    assert.equal(
      typeof alphaServed,
      'number',
      `doorway "alpha" carries no numeric humansServed: ${JSON.stringify(alpha)}`
    );
    assert.ok(
      (alphaServed as number) >= castAtAlpha,
      `doorway "alpha" reports humansServed=${String(alphaServed)}, fewer than the ${castAtAlpha} ` +
        `humans this scenario cast at alpha (the roster's ${live.length} plus the ` +
        `${HOUSEHOLD_HOSTED_CAST.length}-name household cast) — it cannot be leaving out humans it ` +
        'itself hosts and still be counting only its own.'
    );
    assert.equal(
      humansServedOf(beta),
      0,
      `doorway "beta" reports humansServed=${String(humansServedOf(beta))} — nothing was cast at beta ` +
        "in this scenario, so it must not be counting alpha's registrants."
    );
  }
);

Then(
  'doorway {string} counts every human the mesh cast at doorway {string}',
  function (this: E2EWorld, hostId: string, castId: string) {
    assert.equal(
      hostId,
      castId,
      'This story only casts humans at one doorway; the two names must name the same one.'
    );
    const live = liveRosterEntries(state(this).roster);
    // Everything the mesh cast at this doorway: the roster's live entries
    // plus the household cast (see the module doc's CORRECTION note — both
    // are real, provisioned registrants there now, not just the roster).
    const cast = live.length + HOUSEHOLD_HOSTED_CAST.length;
    const status = statusFor(this, hostId);
    const served = humansServedOf(status);
    assert.equal(
      typeof served,
      'number',
      `doorway "${hostId}" carries no numeric humansServed: ${JSON.stringify(status)}`
    );
    assert.ok(
      (served as number) >= cast,
      `doorway "${hostId}" reports humansServed=${String(served)}, fewer than the ${cast} humans ` +
        `the mesh cast there (the roster's ${live.length} plus the ` +
        `${HOUSEHOLD_HOSTED_CAST.length}-name household cast) — it is not counting every human it ` +
        'hosts.'
    );
  }
);

Then('doorway {string} counts none of them', function (this: E2EWorld, id: string) {
  const status = statusFor(this, id);
  assert.equal(
    humansServedOf(status),
    0,
    `doorway "${id}" reports humansServed=${String(humansServedOf(status))}, expected 0 (it never ` +
      "hosted the mesh's cast)."
  );
});

When(
  'a visitor opens the threshold landing of doorway {string}',
  async function (this: E2EWorld, id: string) {
    if (this.deviceMode !== 'playwright') return 'pending';
    const doorway = this.getDoorway(id);
    // eslint-disable-next-line sonarjs/slow-regex -- fixed trailing-slash trim, no backtracking
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
    const live = liveRosterEntries(s.roster);
    assert.ok(
      live.length > 0,
      'No live Prologue registrant to close — run the cast Given first (or every registrant is ' +
        'soft-failed/no-pool on this doorway).'
    );
    const entry = live[0];
    // Task 5 casts every roster human at doorway A with the same fixed
    // ROSTER_PASSWORD (seed-hosted-humans.ts's own DEFAULT_PASSWORD) — a
    // self-service close needs that human's own bearer, never an admin's.
    const doorway = this.getDoorway('alpha');
    // eslint-disable-next-line sonarjs/slow-regex -- fixed trailing-slash trim, no backtracking
    const base = doorway.url.replace(/\/+$/, '');
    const login = await doorway.client.login({
      identifier: entry.identifier,
      password: ROSTER_PASSWORD,
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
  assert.ok(
    typeof recorded === 'number',
    'No recorded humans-served baseline for doorway "alpha".'
  );
  const current = humansServedOf(statusFor(this, 'alpha'));
  assert.equal(current, recorded - 1);
});

/**
 * KNOWN CAVEAT (2026-09-11): re-registering a CLOSED identifier can answer
 * `exists` (the doorway's 409/503-already-exists branches verify by LOGIN
 * succeeding, which a closed-but-still-authenticatable identity satisfies)
 * without actually minting a fresh live grant. That is a separate defect
 * (another agent is making closed identifiers re-registrable with a new
 * cell/grant) from this step's own expectation, which stays exit-code-0: the
 * caster ran and reported a result for the closed identifier, full stop. If
 * it ever fails, the message below quotes that identifier's own printed
 * line so the seeder's own words — not this step's guess — say what result
 * it got.
 */
When('the household mesh casts that human again', function (this: E2EWorld) {
  const s = state(this);
  assert.ok(s.closed, 'No human has been closed yet in this scenario.');
  const closedIdentifier = s.closed.identifier;
  const doorway = this.getDoorway('alpha'); // Task 5 casts every roster human at doorway A.
  // eslint-disable-next-line sonarjs/no-os-command-from-path -- dev-only a2o harness, fixed script arg
  const result = spawnSync('npx', ['tsx', CASTER_ENTRY], {
    cwd: resolve(REPO_ROOT, 'genesis/seeder'),
    encoding: 'utf8',
    env: {
      ...process.env,
      DOORWAY_URL: doorway.url,
      // eslint-disable-next-line sonarjs/publicly-writable-directories -- local dev mesh scratch dir
      MESH_DIR: process.env['MESH_DIR'] ?? '/tmp/elohim-local-mesh',
    },
  });
  const output = `${result.stdout ?? ''}${result.stderr ?? ''}`;
  const closedLine = output.split('\n').find(line => line.includes(closedIdentifier));
  assert.equal(
    result.status,
    0,
    "re-running the Prologue's hosted-human caster (S1 Task 5, seed-hosted-humans.ts) to restore " +
      `its cast failed: ${closedLine ?? output}`
  );
});

Then('the humans-served count is what it was recorded as', function (this: E2EWorld) {
  const s = state(this);
  const recorded = s.recorded.get('alpha');
  assert.ok(
    typeof recorded === 'number',
    'No recorded humans-served baseline for doorway "alpha".'
  );
  const current = humansServedOf(statusFor(this, 'alpha'));
  assert.equal(current, recorded);
});
