/**
 * Agency step definitions — the agency badge (elohim-app) and the agency
 * pipeline (doorway/account), plus the source-level coherence contract
 * between doorway's `agency_phase` values and elohim-app's `AGENCY_STAGES`.
 *
 * Backs:
 *   - features/auth/agency-context-labels.feature
 *   - features/auth/agency-pipeline-coherence.feature
 *
 * Where the truth lives (these steps assert on the real symbols, never on
 * paraphrases):
 *   - app/elohim-app/src/app/imagodei/models/agency.model.ts
 *       `AGENCY_STAGES` (label / tagline / order / isRecoveryMode) and
 *       `getNextStage()`. Imported directly so the progression assertions read
 *       the shipped values, not a copy of them.
 *   - app/elohim-app/src/app/imagodei/components/agency-badge/
 *       `data-testid="agency-badge" | "agency-stage-badge" | "agency-stage-tagline"`.
 *       The badge renders inside the navigator's profile tray, so surfacing it
 *       means opening the tray on a route that mounts `app-elohim-navigator`
 *       (e.g. `/community`) — the lamad bundle uses the Lit navigator, which
 *       carries no Angular agency badge.
 *   - doorway/doorway-app/src/app/components/account/doorway-account.component.ts
 *       `AGENCY_STEPS` + `isStepCompleted()/isCurrentStep()` rendered as
 *       `.pipeline-stepper .step.completed/.current`, the
 *       `.context-banner` steward affordance, and the `.graduation-cta` card.
 *       doorway-app is served at `${doorwayUrl}/threshold/*` (see
 *       doorway/doorway-service/src/routes/threshold.rs), so doorway/account is
 *       `${doorwayUrl}/threshold/account` — NOT elohim-app's own `/account`.
 *   - doorway/doorway-service/src/routes/auth_routes.rs
 *       the `agency_phase` values, read out of the source for the drift check.
 *
 * The drift check reads the CHECKOUT, and a2o's job is to verify the DEPLOYED
 * artifact — so it does not stop at the checkout. Reading two repo files and
 * comparing them is a source-coherence claim that would stay green against a
 * doorway running a different build, which inverts what this suite is for.
 * `readAgencyPhases()` therefore hands its result to `assertDeployedAgencyPhaseContract()`,
 * which makes the RUNNING doorway answer for the same contract before any
 * comparison is allowed to pass. See that function for what the live probe can
 * and cannot establish.
 *
 * Navigation deliberately settles on `domcontentloaded` + an explicit element
 * wait rather than `networkidle`: one scenario holds `GET /auth/account` open
 * on purpose (to observe the badge BEFORE the hosting account resolves), and a
 * held request never lets the network go idle.
 */

import { strict as assert } from 'node:assert';
import { randomUUID } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import {
  AGENCY_STAGES,
  getNextStage,
  type AgencyStage,
} from '../../../../app/elohim-app/src/app/imagodei/models/agency.model.js';
import { DoorwayClient } from '../../src/framework/api/doorway-client.js';
import { BrowserDevice } from '../../src/framework/devices/browser-device.js';
import {
  PlaywrightDevice,
  type PWLocator,
  type PWRoute,
} from '../../src/framework/devices/playwright-device.js';
import { getFixture } from '../../src/framework/fixtures/humans.js';
import { Human } from '../../src/framework/human.js';
import { SHELL, ThresholdLoginPage } from '../../src/framework/pages/index.js';
import { doorwayToAppUrl } from '../../src/framework/utils/url.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const VISIBLE = 'visible';
const WAIT_MS = 20_000;
const LOGIN_PATH = '/identity/login';
const THRESHOLD_PATH = '/threshold/login';
/** A route that mounts `app-elohim-navigator`, whose profile tray hosts the badge. */
const BADGE_HOST_ROUTE = '/community';
/** doorway-app is proxied under /threshold/ by doorway-service. */
const DOORWAY_ACCOUNT_PATH = '/threshold/account';
/** doorway_auth_token — doorway-app's localStorage key (doorway-session-token.store.ts). */
const DOORWAY_TOKEN_KEY = 'doorway_auth_token';

const BADGE = '[data-testid="agency-badge"]';
const STAGE_BADGE = '[data-testid="agency-stage-badge"]';
const STAGE_TAGLINE = '[data-testid="agency-stage-tagline"]';
const PIPELINE_STEP = '.pipeline-stepper .step';
const STEP_LABEL = '.step-label';
const CONTEXT_BANNER = '.context-banner';
const GRADUATION_CTA = '.graduation-cta';
const USAGE_SECTION = '.usage-section';
const KEY_EXPORT_REQUIREMENT = 'export cryptographic keys';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../../../..');
const AUTH_ROUTES_RS = 'doorway/doorway-service/src/routes/auth_routes.rs';

/**
 * `agency_phase` values whose name differs from the AGENCY_STAGES key they
 * reach. Declared here so a NEW doorway phase cannot quietly claim to be a
 * stage — it will land in neither map and the drift check goes red.
 */
const PHASE_STAGE_ALIASES = new Map<string, AgencyStage>([
  ['device', 'app-steward'],
  ['node', 'node-steward'],
]);

/**
 * `agency_phase` values that are deliberately NOT learner agency stages —
 * the "explicit non-stage state" escape the feature names.
 */
const NON_STAGE_PHASES = new Map<string, string>([
  [
    'doorway',
    'operator bootstrap — provisions the doorway operator identity itself, not a learner stage',
  ],
]);

// ---------------------------------------------------------------------------
// Per-scenario state (values that cannot live in world.contentIds: Map<string,string>)
// ---------------------------------------------------------------------------

interface AgencyScenarioState {
  /** Releases a deliberately-held `GET /auth/account`. */
  releaseHostingAccount?: () => void;
  /** The real `/auth/account` body, replayed by the held route once released. */
  hostingAccountBody?: string;
  /** `agency_phase` values read out of auth_routes.rs. */
  agencyPhases?: string[];
}

const scenarioState = new WeakMap<E2EWorld, AgencyScenarioState>();

function state(world: E2EWorld): AgencyScenarioState {
  let s = scenarioState.get(world);
  if (!s) {
    s = {};
    scenarioState.set(world, s);
  }
  return s;
}

// ---------------------------------------------------------------------------
// Helpers — doorway / fixtures
// ---------------------------------------------------------------------------

/** Resolve the first registered doorway URL. Throws if the Background missed. */
function requireDoorwayUrl(world: E2EWorld): string {
  for (const [, doorway] of world.doorways) {
    return doorway.url;
  }
  throw new Error('No doorway registered. Did the Background step run?');
}

/** First bearer token held by any of this human's devices. */
function bearerToken(world: E2EWorld, humanName: string): string {
  const human = world.getHuman(humanName);
  for (const device of human.devices) {
    const anyDevice = device as { token?: string; client?: { token?: string } };
    const token = anyDevice.token ?? anyDevice.client?.token;
    if (token) return token;
  }
  throw new Error(`${humanName} holds no bearer token — did the Given step log them in?`);
}

async function rawPut(
  baseUrl: string,
  path: string,
  token: string
): Promise<{ statusCode: number; body: string }> {
  const { statusCode, body } = await request(`${baseUrl}${path}`, {
    method: 'PUT',
    headers: { 'content-type': 'application/json', authorization: `Bearer ${token}` },
    body: '{}',
  });
  return { statusCode, body: await body.text() };
}

/**
 * Grant graduated-steward status through the dev/fixture admin surface.
 *
 * Mirrors the helper in steps/ui/steward-login-portal-handoff.steps.ts (step
 * files export nothing, so the call cannot be shared). Keep the two in step:
 * both key on the Mongo `_id` resolved from the admin users list.
 */
async function grantStewardFixture(
  doorway: string,
  adminToken: string,
  identifier: string
): Promise<void> {
  const adminClient = new DoorwayClient(doorway);
  adminClient.setToken(adminToken);
  const list = await adminClient.adminListUsers({ search: identifier, limit: 50 });
  const entry = list.users.find(u => u.identifier === identifier);
  assert.ok(
    entry,
    `Cannot grant stewardship: user "${identifier}" not found in the admin users list. ` +
      'Is the fixture human seeded on this doorway?'
  );

  const { statusCode, body } = await rawPut(
    doorway,
    `/admin/users/${encodeURIComponent(entry.id)}/steward`,
    adminToken
  );
  assert.ok(
    statusCode >= 200 && statusCode < 300,
    `Steward grant for "${identifier}" (id=${entry.id}) failed: ` +
      `PUT /admin/users/${entry.id}/steward → ${statusCode}: ${body}. ` +
      'A 403 with code "FIXTURE_ONLY" means the doorway is NOT in dev mode — this ' +
      'fixture-only grant surface is hard-gated off unless dev_mode is set.'
  );
}

/**
 * Log the fixture human in over HTTP (for the bearer token), then grant
 * graduated-steward status so `/auth/account` really carries isSteward:true.
 */
async function establishGraduatedSteward(world: E2EWorld, humanName: string): Promise<void> {
  const fixture = getFixture(humanName);
  const doorway = requireDoorwayUrl(world);

  const human = new Human(humanName, fixture.credentials);
  const device = new BrowserDevice(`${humanName}-browser`, doorway);
  human.addDevice(device);
  const auth = await device.login({
    identifier: fixture.credentials.identifier,
    password: fixture.credentials.password,
  });
  human.agentPubKey = auth.agentPubKey;
  human.humanId = auth.humanId;
  world.addHuman(humanName, human);

  const token = device.token;
  assert.ok(token, `${humanName} has no bearer token after login — cannot grant stewardship.`);
  await grantStewardFixture(doorway, token, fixture.credentials.identifier);
}

/** GET /auth/account with this human's bearer token. Returns the raw body text. */
async function fetchHostingAccount(doorway: string, token: string): Promise<string> {
  const { statusCode, body } = await request(`${doorway}/auth/account`, {
    headers: { authorization: `Bearer ${token}` },
  });
  const text = await body.text();
  assert.equal(statusCode, 200, `GET /auth/account → ${statusCode}: ${text}`);
  return text;
}

// ---------------------------------------------------------------------------
// Helpers — Playwright
// ---------------------------------------------------------------------------

/**
 * Resolve (creating if needed) this human's Playwright device.
 * Returns null in HTTP mode so browser steps can report 'pending'.
 */
async function ensurePlaywrightDevice(
  world: E2EWorld,
  humanName: string
): Promise<PlaywrightDevice | null> {
  if (world.deviceMode !== 'playwright') return null;

  const human = world.getHuman(humanName);
  const existing = human.devices.find(d => d.type === 'playwright');
  if (existing) return existing as PlaywrightDevice;

  const doorway = requireDoorwayUrl(world);
  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
  const browser = await world.getBrowser();

  const device = new PlaywrightDevice(
    `${humanName}-pw`,
    doorwayToAppUrl(doorway),
    doorway,
    // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
    browser
  );
  await device.init();
  human.addDevice(device);
  world.onCleanup(async () => device.close());
  return device;
}

async function waitVisible(locator: PWLocator, timeout = WAIT_MS): Promise<void> {
  await locator.waitFor({ state: VISIBLE, timeout });
}

async function trimmedText(locator: PWLocator): Promise<string> {
  const raw = (await locator.textContent()) ?? '';
  return raw.replaceAll(/\s+/g, ' ').trim();
}

/**
 * Drive the real doorway OAuth login UI: elohim-app /identity/login redirects
 * to the doorway's /threshold/login form, which redirects back through
 * /auth/callback. Same flow as steps/ui/auth.steps.ts; settled on
 * `domcontentloaded` so a deliberately-held request cannot stall the wait.
 */
async function loginViaDoorwayOAuth(
  device: PlaywrightDevice,
  human: Human,
  appUrl: string
): Promise<void> {
  await device.page.goto(`${appUrl}${LOGIN_PATH}`, { waitUntil: 'domcontentloaded' });

  const loginPage = new ThresholdLoginPage(device.page);
  await loginPage.login(human.credentials.identifier, human.credentials.password);

  await device.page.waitForURL(
    (url: URL) =>
      !url.pathname.includes(THRESHOLD_PATH) && !url.pathname.includes('/auth/callback'),
    { timeout: 30_000 }
  );
  await device.page.waitForLoadState('domcontentloaded');
}

/** Open the navigator profile tray and wait for the agency badge to render. */
async function surfaceAgencyBadge(device: PlaywrightDevice, appUrl: string): Promise<void> {
  await device.page.goto(`${appUrl}${BADGE_HOST_ROUTE}`, { waitUntil: 'domcontentloaded' });

  const bubble = device.page.locator(`[data-testid="${SHELL.PROFILE_BUBBLE}"]`);
  await waitVisible(bubble);
  await bubble.click();

  await waitVisible(device.page.locator(BADGE));
}

async function assertBadgeReads(
  device: PlaywrightDevice,
  selector: string,
  expected: string,
  what: string
): Promise<void> {
  const el = device.page.locator(selector);
  await waitVisible(el);
  const actual = await trimmedText(el);
  assert.equal(actual, expected, `${what}: expected "${expected}", got "${actual}"`);
}

/** Poll the stage badge until it reads `expected`, or fail naming the last read. */
async function waitForBadgeText(device: PlaywrightDevice, expected: string): Promise<void> {
  const el = device.page.locator(STAGE_BADGE);
  const deadline = Date.now() + WAIT_MS;
  let last = '';
  while (Date.now() < deadline) {
    last = await trimmedText(el);
    if (last === expected) return;
    await device.page.waitForTimeout(250);
  }
  assert.fail(`Agency stage badge never updated to "${expected}" (last read: "${last}")`);
}

/** Navigate to doorway/account (doorway-app under /threshold), authenticated. */
async function openDoorwayAccount(
  world: E2EWorld,
  device: PlaywrightDevice,
  humanName: string
): Promise<void> {
  const doorway = requireDoorwayUrl(world);
  await device.preloadLocalStorage(DOORWAY_TOKEN_KEY, bearerToken(world, humanName));
  await device.page.goto(`${doorway}${DOORWAY_ACCOUNT_PATH}`, { waitUntil: 'domcontentloaded' });
  await waitVisible(device.page.locator(PIPELINE_STEP).first());
}

/** The `class` attribute of the pipeline step whose label matches. Fails if absent. */
async function pipelineStepClasses(device: PlaywrightDevice, label: string): Promise<string> {
  const steps = device.page.locator(PIPELINE_STEP);
  await waitVisible(steps.first());

  const count = await steps.count();
  const seen: string[] = [];
  for (let i = 0; i < count; i++) {
    const step = steps.nth(i);
    const text = await trimmedText(step.locator(STEP_LABEL));
    seen.push(text);
    if (text === label) return (await step.getAttribute('class')) ?? '';
  }
  return assert.fail(
    `No agency pipeline step labelled "${label}" on doorway/account. Rendered steps: ${seen.join(', ')}`
  );
}

// ---------------------------------------------------------------------------
// Helpers — source-level contract reads
// ---------------------------------------------------------------------------

function readRepoFile(relPath: string): string {
  return readFileSync(resolve(REPO_ROOT, relPath), 'utf8');
}

/**
 * Read the `agency_phase` values doorway actually accepts, out of auth_routes.rs:
 * the `unwrap_or("…")` default, the explicit `== "visitor"` rejection, and every
 * string arm of `match agency_phase { … }` up to its `_ =>` fallback.
 */
function readAgencyPhases(): string[] {
  const source = readRepoFile(AUTH_ROUTES_RS);
  const phases = new Set<string>();

  const fallback =
    /agency_phase\s*=\s*body\.agency_phase\.as_deref\(\)\.unwrap_or\("([\w-]+)"\)/.exec(source);
  assert.ok(
    fallback,
    `${AUTH_ROUTES_RS}: could not find the agency_phase default (unwrap_or). ` +
      'The registration handler changed shape — update this contract reader.'
  );
  phases.add(fallback[1]);

  for (const m of source.matchAll(/agency_phase\s*==\s*"([\w-]+)"/g)) phases.add(m[1]);

  const matchIdx = source.indexOf('match agency_phase {');
  assert.ok(
    matchIdx !== -1,
    `${AUTH_ROUTES_RS}: no "match agency_phase {" block found — update this contract reader.`
  );
  for (const line of source.slice(matchIdx).split('\n').slice(1)) {
    if (/^\s*_\s*=>/.test(line)) break;
    const arm = /^\s*("[\w-]+"(?:\s*\|\s*"[\w-]+")*)\s*=>/.exec(line);
    if (arm) for (const lit of arm[1].matchAll(/"([\w-]+)"/g)) phases.add(lit[1]);
  }

  assert.ok(
    phases.size >= 2,
    `${AUTH_ROUTES_RS}: read only ${phases.size} agency_phase value(s) — the reader is stale.`
  );
  return [...phases].sort((a, b) => a.localeCompare(b));
}

/** The AGENCY_STAGES stage an agency_phase reaches, or null when it is a non-stage. */
function stageForPhase(phase: string): AgencyStage | null {
  if (NON_STAGE_PHASES.has(phase)) return null;
  const alias = PHASE_STAGE_ALIASES.get(phase);
  if (alias) return alias;
  return phase in AGENCY_STAGES ? (phase as AgencyStage) : null;
}

function requireAgencyPhases(world: E2EWorld): string[] {
  const phases = state(world).agencyPhases;
  assert.ok(phases, 'agency_phase values were never read — did the When step run?');
  return phases;
}

// ═══════════════════════════════════════════════════════════════════════════
// agency-context-labels.feature — the badge in elohim-app
// ═══════════════════════════════════════════════════════════════════════════

/**
 * A human the doorway records as a graduated steward: logged in over HTTP for
 * the bearer token, then granted stewardship through the fixture admin surface
 * so `/auth/account` carries isSteward:true. No browser login happens here —
 * the OAuth-flow When step drives the real UI.
 *
 * Example: Given human "Matthew" is a graduated steward
 */
Given('human {string} is a graduated steward', async function (this: E2EWorld, humanName: string) {
  await establishGraduatedSteward(this, humanName);
});

/**
 * Drive the real doorway OAuth login from elohim-app.
 *
 * Example: When Matthew logs in at elohim-app via the doorway OAuth flow
 */
When(
  '{word} logs in at elohim-app via the doorway OAuth flow',
  async function (this: E2EWorld, humanName: string) {
    const device = await ensurePlaywrightDevice(this, humanName);
    if (!device) return 'pending';
    await loginViaDoorwayOAuth(
      device,
      this.getHuman(humanName),
      doorwayToAppUrl(requireDoorwayUrl(this))
    );
    return undefined;
  }
);

/**
 * Surface the agency badge — it lives in the navigator's profile tray.
 *
 * Example: And elohim-app loads the agency badge
 */
When('elohim-app loads the agency badge', async function (this: E2EWorld) {
  for (const [, human] of this.humans) {
    const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
    if (!device) continue;
    await surfaceAgencyBadge(device, doorwayToAppUrl(requireDoorwayUrl(this)));
    return undefined;
  }
  return 'pending';
});

/**
 * Log in with `GET /auth/account` deliberately held open, so the badge can be
 * read in the window where the human is authenticated but the hosting account
 * has not resolved yet. AgencyService falls back to `hosted` in that window.
 *
 * Example: When Matthew logs in at elohim-app and the hosting account has not loaded yet
 */
When(
  '{word} logs in at elohim-app and the hosting account has not loaded yet',
  async function (this: E2EWorld, humanName: string) {
    const device = await ensurePlaywrightDevice(this, humanName);
    if (!device) return 'pending';

    const doorway = requireDoorwayUrl(this);
    const appUrl = doorwayToAppUrl(doorway);
    const s = state(this);

    // Capture the REAL account body first — the held route replays it verbatim
    // once released, so nothing about the steward's shape is invented here.
    s.hostingAccountBody = await fetchHostingAccount(doorway, bearerToken(this, humanName));

    const gate = new Promise<void>(release => {
      s.releaseHostingAccount = release;
    });
    // Never leave a browser blocked on an unreleased gate.
    this.onCleanup(async () => {
      s.releaseHostingAccount?.();
      return Promise.resolve();
    });

    await device.page.route('**/auth/account*', async (route: PWRoute) => {
      await gate;
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: s.hostingAccountBody ?? '{}',
      });
    });

    await loginViaDoorwayOAuth(device, this.getHuman(humanName), appUrl);
    await surfaceAgencyBadge(device, appUrl);
    return undefined;
  }
);

/**
 * Release the held `/auth/account`, serving the real steward body.
 *
 * Example: When the hosting account loads with isSteward=true
 */
When('the hosting account loads with isSteward=true', function (this: E2EWorld) {
  const s = state(this);
  assert.ok(
    s.releaseHostingAccount,
    'No held /auth/account request — the "has not loaded yet" step must run first.'
  );
  const account = JSON.parse(s.hostingAccountBody ?? '{}') as { isSteward?: boolean };
  assert.equal(
    account.isSteward,
    true,
    'The captured /auth/account body does not carry isSteward:true — the fixture ' +
      'steward grant did not take, so this scenario cannot observe the upgrade.'
  );
  s.releaseHostingAccount();
  return undefined;
});

Then('the agency stage badge reads {string}', async function (this: E2EWorld, expected: string) {
  for (const [, human] of this.humans) {
    const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
    if (!device) continue;
    await assertBadgeReads(device, STAGE_BADGE, expected, 'Agency stage badge');
    return undefined;
  }
  return 'pending';
});

Then(
  'the agency stage badge initially reads {string}',
  async function (this: E2EWorld, expected: string) {
    for (const [, human] of this.humans) {
      const device = human.devices.find(d => d.type === 'playwright') as
        | PlaywrightDevice
        | undefined;
      if (!device) continue;
      await assertBadgeReads(device, STAGE_BADGE, expected, 'Agency stage badge (pre-account)');
      return undefined;
    }
    return 'pending';
  }
);

Then(
  'the agency stage badge updates to {string}',
  async function (this: E2EWorld, expected: string) {
    for (const [, human] of this.humans) {
      const device = human.devices.find(d => d.type === 'playwright') as
        | PlaywrightDevice
        | undefined;
      if (!device) continue;
      await waitForBadgeText(device, expected);
      return undefined;
    }
    return 'pending';
  }
);

Then('the tagline reads {string}', async function (this: E2EWorld, expected: string) {
  for (const [, human] of this.humans) {
    const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
    if (!device) continue;
    await assertBadgeReads(device, STAGE_TAGLINE, expected, 'Agency stage tagline');
    return undefined;
  }
  return 'pending';
});

// ---------------------------------------------------------------------------
// AGENCY_STAGES progression — read from the shipped model, no browser needed
// ---------------------------------------------------------------------------

When('I read the AGENCY_STAGES progression', function () {
  const stages = Object.keys(AGENCY_STAGES);
  assert.ok(
    stages.length > 0,
    'AGENCY_STAGES is empty — the imported elohim-app model did not resolve.'
  );
});

Then('{string} is flagged as a recovery mode, not a progression step', function (stage: string) {
  assert.ok(stage in AGENCY_STAGES, `"${stage}" is not a stage in AGENCY_STAGES`);
  assert.equal(
    AGENCY_STAGES[stage as AgencyStage].isRecoveryMode,
    true,
    `AGENCY_STAGES["${stage}"].isRecoveryMode is not true — it now reads as a rung ` +
      'on the progression ladder, which contradicts the recovery-mode contract.'
  );
});

Then('the stage after {string} is {string}', function (current: string, expected: string) {
  assert.ok(current in AGENCY_STAGES, `"${current}" is not a stage in AGENCY_STAGES`);
  const next = getNextStage(current as AgencyStage);
  assert.equal(next, expected, `getNextStage("${current}") returned ${String(next)}`);
});

Then(
  'getNextStage from {string} is null — a recovery mode has no next rung',
  function (stage: string) {
    assert.ok(stage in AGENCY_STAGES, `"${stage}" is not a stage in AGENCY_STAGES`);
    const next = getNextStage(stage as AgencyStage);
    assert.equal(
      next,
      null,
      `getNextStage("${stage}") returned "${String(next)}" — a recovery mode must not ` +
        'offer a next rung; its resolution is restoring peer-native authority.'
    );
  }
);

Then(
  'the order ranking is visitor < hosted < hosted-steward < app-steward < node-steward',
  function () {
    const expected: AgencyStage[] = [
      'visitor',
      'hosted',
      'hosted-steward',
      'app-steward',
      'node-steward',
    ];
    const actual = Object.values(AGENCY_STAGES)
      .slice()
      .sort((a, b) => a.order - b.order)
      .map(s => s.stage);
    assert.deepEqual(
      actual,
      expected,
      `AGENCY_STAGES order ranking is ${actual.join(' < ')}, expected ${expected.join(' < ')}`
    );
  }
);

// ═══════════════════════════════════════════════════════════════════════════
// agency-pipeline-coherence.feature — the pipeline on doorway/account
// ═══════════════════════════════════════════════════════════════════════════

/**
 * A graduated steward whose own conductor is unreachable, so the doorway is
 * still hosting their cell. The wire signal for "doorway-hosted" is a non-null
 * `conductorId` on /auth/account (doorway-account.component.ts reads exactly
 * that for `stewardAccessingThroughDoorway`).
 *
 * Example: Given human "Matthew" is a graduated steward whose conductor is offline
 */
Given(
  'human {string} is a graduated steward whose conductor is offline',
  async function (this: E2EWorld, humanName: string) {
    await establishGraduatedSteward(this, humanName);

    const doorway = requireDoorwayUrl(this);
    const body = await fetchHostingAccount(doorway, bearerToken(this, humanName));
    const account = JSON.parse(body) as { isSteward?: boolean; conductorId?: string | null };

    assert.equal(account.isSteward, true, `${humanName} is not recorded as a steward: ${body}`);
    assert.ok(
      account.conductorId,
      `${humanName}'s /auth/account carries no conductorId, so the doorway is NOT hosting ` +
        'their cell — the "conductor offline" precondition does not hold on this doorway.'
    );
  }
);

/**
 * Example: When Matthew opens doorway/account
 */
When(String.raw`{word} opens doorway\/account`, async function (this: E2EWorld, humanName: string) {
  const device = await ensurePlaywrightDevice(this, humanName);
  if (!device) return 'pending';
  await openDoorwayAccount(this, device, humanName);
  return undefined;
});

function requireAccountDevice(world: E2EWorld): PlaywrightDevice | null {
  for (const [, human] of world.humans) {
    const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
    if (device) return device;
  }
  return null;
}

Then(
  'the agency pipeline step {string} is marked completed',
  async function (this: E2EWorld, label: string) {
    const device = requireAccountDevice(this);
    if (!device) return 'pending';
    const classes = await pipelineStepClasses(device, label);
    assert.ok(
      classes.split(/\s+/).includes('completed'),
      `Pipeline step "${label}" is not marked completed (class="${classes}")`
    );
    return undefined;
  }
);

Then(
  'the pipeline step {string} is NOT marked completed',
  async function (this: E2EWorld, label: string) {
    const device = requireAccountDevice(this);
    if (!device) return 'pending';
    const classes = await pipelineStepClasses(device, label);
    assert.ok(
      !classes.split(/\s+/).includes('completed'),
      `Pipeline step "${label}" is marked completed (class="${classes}") — doorway/account ` +
        'claims this human is further along than the elohim-app agency badge does.'
    );
    return undefined;
  }
);

Then(
  'the agency pipeline step {string} is marked current',
  async function (this: E2EWorld, label: string) {
    const device = requireAccountDevice(this);
    if (!device) return 'pending';
    const classes = await pipelineStepClasses(device, label);
    assert.ok(
      classes.split(/\s+/).includes('current'),
      `Pipeline step "${label}" is not marked current (class="${classes}")`
    );
    return undefined;
  }
);

Then(
  'the pipeline shows the in-between {string} affordance',
  async function (this: E2EWorld, affordance: string) {
    const device = requireAccountDevice(this);
    if (!device) return 'pending';

    const banner = device.page.locator(CONTEXT_BANNER);
    const present = await banner.isVisible().catch(() => false);
    assert.ok(
      present,
      `doorway/account shows no in-between "${affordance}" affordance — the steward-context ` +
        'banner is absent, so the page reads as a plain hosted account.'
    );

    const text = await trimmedText(banner);
    assert.ok(
      text.toLowerCase().includes(affordance.toLowerCase()),
      `The in-between affordance does not name "${affordance}" — it reads "${text}".`
    );
    return undefined;
  }
);

Then(
  'the pipeline does NOT show a {string} affordance',
  async function (this: E2EWorld, affordance: string) {
    const device = requireAccountDevice(this);
    if (!device) return 'pending';

    const banner = device.page.locator(CONTEXT_BANNER);
    const present = await banner.isVisible().catch(() => false);
    assert.ok(
      !present,
      `doorway/account shows a "${affordance}" affordance for a human who has not graduated.`
    );
    return undefined;
  }
);

Then('the page reads {string} near the header', async function (this: E2EWorld, phrase: string) {
  const device = requireAccountDevice(this);
  if (!device) return 'pending';

  const match = device.page.getByText(phrase, { exact: false }).first();
  const present = await match.isVisible().catch(() => false);
  assert.ok(present, `doorway/account never renders "${phrase}".`);

  // "near the header" = above the Usage card, i.e. in the page's header band.
  const phraseBox = await match.boundingBox();
  const usageBox = await device.page.locator(USAGE_SECTION).first().boundingBox();
  if (phraseBox && usageBox) {
    assert.ok(
      phraseBox.y < usageBox.y,
      `"${phrase}" renders below the Usage card (y=${phraseBox.y} vs ${usageBox.y}), not near the header.`
    );
  }
  return undefined;
});

Then(
  'the Graduate-to-Steward CTA is visible with key_export listed as the next gate',
  async function (this: E2EWorld) {
    const device = requireAccountDevice(this);
    if (!device) return 'pending';

    const cta = device.page.locator(GRADUATION_CTA);
    await waitVisible(cta);

    const requirements = (await cta.locator('.requirements li').allTextContents()).map(t =>
      t.replaceAll(/\s+/g, ' ').trim()
    );
    assert.ok(
      requirements.some(t => t.toLowerCase().includes(KEY_EXPORT_REQUIREMENT)),
      `The Graduate-to-Steward CTA does not list key export. Requirements: ${requirements.join(' | ')}`
    );

    const met = (await cta.locator('.requirements li.met').allTextContents()).map(t =>
      t.toLowerCase()
    );
    assert.ok(
      !met.some(t => t.includes(KEY_EXPORT_REQUIREMENT)),
      'Key export is already marked met, so it is not the NEXT gate.'
    );
    return undefined;
  }
);

// ---------------------------------------------------------------------------
// agency_phase ↔ AGENCY_STAGES drift — both sides read at source level
// ---------------------------------------------------------------------------

/** A phase token no build has ever accepted, used to probe the closed-set boundary. */
const UNKNOWN_PHASE_PROBE = 'a2o-not-a-real-agency-phase';

/** The non-registering phase both surfaces name; rejected before any registration work. */
const VISITOR_PHASE = 'visitor';

/** POST /auth/register with one agency phase, returning the status and error code. */
async function probeAgencyPhase(
  doorwayUrl: string,
  phase: string
): Promise<{ statusCode: number; code: string; body: string }> {
  const { statusCode, body } = await request(`${doorwayUrl}/auth/register`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      // Non-empty identifier and password only because handle_register rejects
      // empty ones BEFORE it looks at agency_phase; the address is unroutable
      // and the phase never reaches a registration branch.
      identifier: `a2o-agency-contract-probe-${randomUUID()}@invalid.test`,
      // Freshly generated, never reused, and never stored: both probed phases
      // are refused before handle_register ever looks at the password.
      password: `a2o-agency-contract-probe-${randomUUID()}`,
      agencyPhase: phase,
    }),
  });
  const text = await body.text();
  let code = '';
  try {
    code = String((JSON.parse(text) as { code?: unknown }).code ?? '');
  } catch {
    code = '';
  }
  return { statusCode, code, body: text };
}

/**
 * Make the DEPLOYED doorway answer for the agency-phase contract the checkout
 * claims, so this scenario cannot be green against a doorway that disagrees.
 *
 * Both probes are side-effect free by construction: each names a phase that
 * `handle_register` rejects BEFORE the `match agency_phase` block does any
 * registration work, so no Human is created, no agent provisioned, no row
 * written. (That is also why the positive direction is NOT probed here —
 * "hosted"/"node"/"device" reach real create_human / provision_agent calls, and
 * a test may not mint identities on a shared fleet to take a measurement.)
 *
 * What this establishes about the running binary:
 *   - it enforces a CLOSED set of phases (the `_ =>` INVALID_AGENCY_PHASE arm
 *     the source reader parsed is really there, in the build that is serving);
 *   - it shares the checkout's `visitor` semantics — a named phase that is
 *     deliberately not a registration path.
 * What it does not: a deployed doorway that ADDED a phase the checkout has
 * never heard of is still invisible from outside, because a phase set is not
 * enumerable over HTTP. Closing that gap needs the doorway to publish its own
 * phase list; until it does, this is the boundary a live probe can reach.
 */
async function assertDeployedAgencyPhaseContract(
  doorwayUrl: string,
  sourcePhases: string[]
): Promise<void> {
  assert.ok(
    !sourcePhases.includes(UNKNOWN_PHASE_PROBE),
    `${AUTH_ROUTES_RS} now accepts the probe token "${UNKNOWN_PHASE_PROBE}" — pick another.`
  );

  const unknown = await probeAgencyPhase(doorwayUrl, UNKNOWN_PHASE_PROBE);
  assert.equal(
    unknown.statusCode,
    400,
    `The doorway at ${doorwayUrl} answered ${unknown.statusCode} to an unknown agency_phase ` +
      `instead of rejecting it: ${unknown.body}. The checkout's ${AUTH_ROUTES_RS} closes the ` +
      'phase set with an INVALID_AGENCY_PHASE fallback; the running build does not, so this ' +
      'comparison would be judging source the fleet is not serving.'
  );
  assert.equal(
    unknown.code,
    'INVALID_AGENCY_PHASE',
    `The doorway at ${doorwayUrl} rejected an unknown agency_phase with code ` +
      `"${unknown.code}" rather than INVALID_AGENCY_PHASE: ${unknown.body}. Deployed and ` +
      'checked-out doorway disagree about how the phase set is closed.'
  );

  assert.ok(
    sourcePhases.includes(VISITOR_PHASE),
    `${AUTH_ROUTES_RS} no longer names "${VISITOR_PHASE}" — the reader or the contract moved.`
  );
  const visitor = await probeAgencyPhase(doorwayUrl, VISITOR_PHASE);
  assert.equal(
    visitor.statusCode,
    400,
    `The doorway at ${doorwayUrl} answered ${visitor.statusCode} to agencyPhase="visitor" ` +
      `instead of refusing it: ${visitor.body}. The checkout refuses visitors at registration; ` +
      'the running build does not.'
  );
  assert.equal(
    visitor.code,
    'VISITOR_NO_REGISTER',
    `The doorway at ${doorwayUrl} refused agencyPhase="visitor" with code "${visitor.code}" ` +
      `rather than VISITOR_NO_REGISTER: ${visitor.body}. Deployed and checked-out doorway ` +
      'disagree about what a visitor is.'
  );
}

When(
  'I compare auth_routes.rs agency_phase values to elohim-app AGENCY_STAGES stages',
  async function (this: E2EWorld) {
    const phases = readAgencyPhases();
    // Bind the checkout's claim to the fleet BEFORE any comparison is allowed
    // to pass — otherwise the two Then steps below judge only the CI checkout.
    await assertDeployedAgencyPhaseContract(requireDoorwayUrl(this), phases);
    state(this).agencyPhases = phases;
  }
);

Then(
  'every agency_phase value maps to an AGENCY_STAGES stage or an explicit non-stage state',
  function (this: E2EWorld) {
    const unmapped = requireAgencyPhases(this).filter(
      phase => !NON_STAGE_PHASES.has(phase) && stageForPhase(phase) === null
    );
    assert.deepEqual(
      unmapped,
      [],
      `auth_routes.rs accepts agency_phase value(s) ${unmapped.join(', ')} that reach no ` +
        'AGENCY_STAGES stage and are not declared non-stage states. Either map them in ' +
        'PHASE_STAGE_ALIASES or declare them in NON_STAGE_PHASES with a reason.'
    );
  }
);

Then(
  'no stage is reachable in one surface without being reachable in the other',
  function (this: E2EWorld) {
    const reachableStages = new Set(
      requireAgencyPhases(this)
        .map(phase => stageForPhase(phase))
        .filter((s): s is AgencyStage => s !== null)
    );

    const orphans = Object.values(AGENCY_STAGES)
      .filter(info => !reachableStages.has(info.stage) && !info.isRecoveryMode)
      .map(info => info.stage);
    assert.deepEqual(
      orphans,
      [],
      `AGENCY_STAGES stage(s) ${orphans.join(', ')} are reachable in elohim-app but no ` +
        'auth_routes.rs agency_phase reaches them. A progression stage with no registration ' +
        'phase is a surface claiming a rung the other surface cannot produce.'
    );

    // The converse is the previous assertion; what remains is the deliberate
    // asymmetry: recovery modes are runtime states, never registration phases.
    for (const info of Object.values(AGENCY_STAGES)) {
      if (reachableStages.has(info.stage)) continue;
      assert.equal(
        info.isRecoveryMode,
        true,
        `"${info.stage}" has no agency_phase and is not marked isRecoveryMode.`
      );
    }
  }
);
