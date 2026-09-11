/** Hosted-human lifecycle: portal registration, account inspection, and clean closure. */
import { strict as assert } from 'node:assert';
import { randomUUID } from 'node:crypto';

import { Given, Then, When } from '@cucumber/cucumber';

import { request } from 'undici';

import { BrowserDevice } from '../../src/framework/devices/browser-device.js';
import { PlaywrightDevice, type PWPage } from '../../src/framework/devices/playwright-device.js';
import {
  NON_POOL_PEER_NAME,
  commitmentField,
  commitmentIsLive,
  doorwayServiceIdentityAgentPubKey,
  readHostedCellCommitment,
  readHostedCellCommitmentConverged,
  stewardAgentPubKeyForConductorOrigin,
  type CommitmentBody,
} from '../../src/framework/fixtures/hosted-cell.js';
import { Human } from '../../src/framework/human.js';
import {
  ACCOUNT,
  LANDING,
  THRESHOLD_REGISTER,
  TOOLBAR,
} from '../../src/framework/pages/selectors.js';
import { ThresholdLoginPage } from '../../src/framework/pages/threshold-login.page.js';

import type { AuthResponse } from '../../src/framework/api/doorway-client.js';
import type { E2EWorld } from '../../src/framework/world.js';

const AUTH_TOKEN_KEY = 'doorway_auth_token';
const REGISTER_PATH = '/threshold/register';
const ACCOUNT_PATH = '/threshold/account';
// eslint-disable-next-line sonarjs/no-hardcoded-passwords -- required ephemeral E2E credential
const PASSWORD = 'Test2026!';

/** IDs absent from the shared selector registry because their UI is being built in parallel. */
const TEST_ID = {
  registerError: 'threshold-register-error',
  accountIdentifier: 'account-identifier',
  closeBegin: 'account-close-begin',
  closeInput: 'account-close-confirm-input',
  closeConfirm: 'account-close-confirm',
  closeError: 'account-close-error',
  /** S2 Task 11 adds these to the account page; agreed here per S1 plan Task 4 Step 2. */
  hostedByHousehold: 'account-hosted-by-household',
  hostedUntil: 'account-hosted-until',
} as const;

type Json = Record<string, unknown>;
interface BrowserResponse {
  url(): string;
  status(): number;
  text(): Promise<string>;
  request(): { method(): string };
}
type ResponsePage = PWPage & {
  waitForResponse(
    predicate: (response: BrowserResponse) => boolean,
    options?: Json
  ): Promise<BrowserResponse>;
};
interface PortalBridge {
  localPart?: string;
  canonicalIdentifier?: string;
  password?: string;
  device?: PlaywrightDevice;
  doorwayUrl?: string;
}
interface Attempt {
  status: number;
  body: Json;
}
interface HostedHuman {
  human: Human;
  localPart: string;
  doorwayUrl: string;
  token: string;
  agentPubKey: string;
  registrationDisplayName?: string;
  preClosureToken?: string;
  attempt?: Attempt;
  /** `hostedCellGrantCid` from `/auth/account` (S2 Task 13) — the notary's read key. */
  grantCid?: string;
  /** Last commitment body read back from a household peer that is not the doorway's pool. */
  commitment?: CommitmentBody;
  /** Cached Decision-2 steward resolution, once found, for this person's pool conductor. */
  stewardAgentPubKey?: string;
}
interface PipelineStep {
  label: string;
  classes: string[];
}

const peopleByWorld = new WeakMap<E2EWorld, HostedHuman[]>();

function people(world: E2EWorld): HostedHuman[] {
  let value = peopleByWorld.get(world);
  if (!value) {
    value = [];
    peopleByWorld.set(world, value);
  }
  return value;
}

function portal(world: E2EWorld): PortalBridge {
  const extended = world as unknown as { __portal?: PortalBridge };
  extended.__portal ??= {};
  return extended.__portal;
}

function requirePlaywright(world: E2EWorld): PlaywrightDevice | null {
  if (world.deviceMode !== 'playwright') return null;
  const device = portal(world).device;
  if (!device) throw new Error('No Playwright doorway portal is open.');
  return device;
}

function human(world: E2EWorld, index = people(world).length - 1): HostedHuman {
  const value = people(world)[index];
  assert.ok(value, `Hosted human ${index + 1} has not been registered.`);
  return value;
}

function json(text: string): Json {
  const value: unknown = JSON.parse(text);
  assert.ok(value && typeof value === 'object' && !Array.isArray(value), 'Expected a JSON object.');
  return value as Json;
}

function withoutTrailingSlash(value: string): string {
  while (value.endsWith('/')) value = value.slice(0, -1);
  return value;
}

async function api(url: string, token: string, method = 'GET', body?: Json): Promise<Attempt> {
  const response = await request(url, {
    method,
    headers: {
      authorization: `Bearer ${token}`,
      ...(body ? { 'content-type': 'application/json' } : {}),
    },
    ...(body ? { body: JSON.stringify(body) } : {}),
  });
  return { status: response.statusCode, body: json(await response.body.text()) };
}

async function closeAccount(person: HostedHuman, token: string): Promise<Attempt> {
  return api(`${person.doorwayUrl}/auth/close-account`, token, 'POST', {
    confirmIdentifier: person.human.credentials.identifier,
  });
}

async function watch(page: PWPage, path: string): Promise<BrowserResponse> {
  return (page as ResponsePage).waitForResponse(
    response => new URL(response.url()).pathname === path && response.request().method() === 'POST',
    { timeout: 30_000 }
  );
}

function remember(
  world: E2EWorld,
  displayName: string,
  localPart: string,
  doorwayId: string,
  doorwayUrl: string,
  auth: AuthResponse,
  device: BrowserDevice | PlaywrightDevice
): void {
  assert.equal(
    auth.identifier.split('@')[0],
    localPart,
    'The doorway registered a different human.'
  );
  const ordinal = people(world).length === 0 ? 'first human' : 'second human';
  const model = new Human(ordinal, {
    identifier: auth.identifier,
    password: PASSWORD,
    displayName,
  });
  model.agentPubKey = auth.agentPubKey;
  model.humanId = auth.humanId;
  model.setToken(doorwayId, auth.token);
  model.addDevice(device);
  world.addHuman(ordinal, model);
  const person: HostedHuman = {
    human: model,
    localPart,
    doorwayUrl,
    token: auth.token,
    agentPubKey: auth.agentPubKey,
    registrationDisplayName: auth.profile?.displayName,
  };
  people(world).push(person);
  Object.assign(portal(world), {
    localPart,
    canonicalIdentifier: auth.identifier,
    password: PASSWORD,
    doorwayUrl,
    ...(device instanceof PlaywrightDevice ? { device } : {}),
  });
  world.onCleanup(async () => {
    try {
      await closeAccount(person, person.token);
    } catch {
      // Best-effort cleanup through the same self-service contract under test.
    }
  });
}

async function registerThroughPortal(world: E2EWorld, displayName: string): Promise<void> {
  const device = requirePlaywright(world);
  if (!device) return;
  const doorwayUrl = portal(world).doorwayUrl;
  assert.ok(doorwayUrl, 'The portal has no doorway origin.');
  const localPart = `hh-${randomUUID()}`;
  await device.page.getByTestId(THRESHOLD_REGISTER.DISPLAY_NAME).fill(displayName);
  await device.page.getByTestId(THRESHOLD_REGISTER.EMAIL).fill(localPart);
  await device.page.getByTestId(THRESHOLD_REGISTER.PASSWORD).fill(PASSWORD);
  await device.page.getByTestId(THRESHOLD_REGISTER.CONFIRM_PASSWORD).fill(PASSWORD);
  const responsePromise = watch(device.page, '/auth/register');
  await device.page.getByTestId(THRESHOLD_REGISTER.SUBMIT).click();
  const response = await responsePromise;
  const text = await response.text();
  if (response.status() >= 300) {
    const error = device.page.getByTestId(TEST_ID.registerError);
    assert.fail(
      `Registration returned ${response.status()}: ${(await error.count()) ? await error.innerText() : text}`
    );
  }
  await device.page.waitForFunction(
    (key: string) => globalThis.localStorage.getItem(key) !== null,
    AUTH_TOKEN_KEY,
    { timeout: 20_000 }
  );
  const doorwayId = [...world.doorways].find(
    ([, d]) => withoutTrailingSlash(d.url) === doorwayUrl
  )?.[0];
  assert.ok(doorwayId, 'The portal origin is not a registered doorway.');
  remember(
    world,
    displayName,
    localPart,
    doorwayId,
    doorwayUrl,
    json(text) as unknown as AuthResponse,
    device
  );
}

async function assertDisplayName(person: HostedHuman, expected: string): Promise<void> {
  const result = await api(`${person.doorwayUrl}/auth/account`, person.token);
  assert.equal(result.status, 200, `GET /auth/account returned ${result.status}.`);
  if (typeof result.body['displayName'] === 'string') {
    assert.equal(result.body['displayName'], expected);
    return;
  }
  // eslint-disable-next-line sonarjs/todo-tag -- names the required backend contract gap
  // TODO(hosted-human-display-name-wire): add displayName to /auth/account or /auth/me.
  assert.equal(person.registrationDisplayName, expected);
}

async function cellsFor(world: E2EWorld, person: HostedHuman): Promise<string[]> {
  const admin = await world.getAdminClient(person.doorwayUrl);
  const conductors = (await admin.adminConductors()).conductors;
  const listings = await Promise.all(
    conductors.map(async c => admin.adminConductorAgents(c.conductorId))
  );
  return listings.flatMap(c =>
    c.agents.filter(agent => agent.agentPubKey === person.agentPubKey).map(() => c.conductorId)
  );
}

async function beginClosure(device: PlaywrightDevice): Promise<void> {
  await device.page.getByTestId(TEST_ID.closeBegin).click();
  await device.page.getByTestId(TEST_ID.closeInput).waitFor({ state: 'visible' });
}

async function submitClosure(device: PlaywrightDevice, identifier: string): Promise<Attempt> {
  const responsePromise = watch(device.page, '/auth/close-account');
  await device.page.getByTestId(TEST_ID.closeInput).fill(identifier);
  await device.page.getByTestId(TEST_ID.closeConfirm).click();
  const response = await responsePromise;
  return { status: response.status(), body: json(await response.text()) };
}

async function pipeline(device: PlaywrightDevice): Promise<PipelineStep[]> {
  return (await device.page.evaluate(() =>
    [...document.querySelectorAll<HTMLElement>('.step')].map(step => ({
      label: step.querySelector<HTMLElement>('.step-label')?.innerText.trim() ?? '',
      classes: [...step.classList],
    }))
  )) as PipelineStep[];
}

type BrowserBody = (world: E2EWorld, device: PlaywrightDevice, ...args: string[]) => Promise<void>;

/**
 * cucumber-js validates a step function's DECLARED parameter count against the
 * expression's capture count, and a rest-parameter wrapper declares zero — which
 * is why a `{string}` step wrapped that way fails at run time with "function has 0
 * arguments, should have 1". So the wrapper is minted at the arity the pattern
 * needs (0–3 captures cover every step in this file).
 */
function arityOf(pattern: string | RegExp): number {
  if (typeof pattern === 'string') return (pattern.match(/\{[a-z]+\}/g) ?? []).length;
  const probe = new RegExp(`${pattern.source}|`).exec('');
  return probe ? probe.length - 1 : 0;
}

type StepFn = (this: E2EWorld, ...args: string[]) => Promise<void | 'pending'>;

function withDevice(body: BrowserBody, arity: number): StepFn {
  const run = async (world: E2EWorld, args: string[]): Promise<void | 'pending'> => {
    const device = requirePlaywright(world);
    if (!device) return 'pending';
    await body(world, device, ...args);
    return undefined;
  };
  switch (arity) {
    case 0:
      return async function (this: E2EWorld) {
        return run(this, []);
      };
    case 1:
      return async function (this: E2EWorld, a: string) {
        return run(this, [a]);
      };
    case 2:
      return async function (this: E2EWorld, a: string, b: string) {
        return run(this, [a, b]);
      };
    default:
      return async function (this: E2EWorld, a: string, b: string, c: string) {
        return run(this, [a, b, c]);
      };
  }
}

function browserWhen(pattern: string | RegExp, body: BrowserBody): void {
  When(pattern, withDevice(body, arityOf(pattern)));
}

function browserThen(pattern: string | RegExp, body: BrowserBody): void {
  Then(pattern, withDevice(body, arityOf(pattern)));
}

browserWhen('the browser opens the doorway registration portal', async (world, device) => {
  const doorwayUrl = portal(world).doorwayUrl;
  assert.ok(doorwayUrl, 'The browser has no doorway origin.');
  await device.navigate(`${doorwayUrl}${REGISTER_PATH}`);
  await device.page.getByTestId(THRESHOLD_REGISTER.DISPLAY_NAME).waitFor({ state: 'visible' });
});

browserThen('the portal renders its registration form', async (_world, device) => {
  await Promise.all(
    [
      THRESHOLD_REGISTER.DISPLAY_NAME,
      THRESHOLD_REGISTER.EMAIL,
      THRESHOLD_REGISTER.PASSWORD,
      THRESHOLD_REGISTER.CONFIRM_PASSWORD,
      THRESHOLD_REGISTER.SUBMIT,
    ].map(async id => device.page.getByTestId(id).waitFor({ state: 'visible' }))
  );
});

browserWhen(
  /^(?:the|that browser's) newcomer creates an account through the portal with the display name "([^"]+)"$/,
  async (world, _device, name) => {
    await registerThroughPortal(world, name);
  }
);

Given(
  'a hosted human {string} is registered on doorway {string}',
  async function (this: E2EWorld, name: string, doorwayId: string) {
    const doorway = this.getDoorway(doorwayId);
    const localPart = `hh-${randomUUID()}`;
    const device = new BrowserDevice(`${name}-api`, doorway.url);
    const auth = await device.register({
      identifier: localPart,
      password: PASSWORD,
      displayName: name,
    });
    remember(this, name, localPart, doorwayId, withoutTrailingSlash(doorway.url), auth, device);
  }
);

Then(
  /^the doorway names (that|the first|the second) human "([^"]+)"$/,
  async function (this: E2EWorld, which: string, name: string) {
    let person = human(this);
    if (which === 'the first') person = human(this, 0);
    if (which === 'the second') person = human(this, 1);
    await assertDisplayName(person, name);
  }
);

Then(
  /^(the doorway holds a cell for that human on one of its pool conductors|no pool conductor holds a cell for that human)$/,
  async function (this: E2EWorld, phrase: string) {
    assert.equal((await cellsFor(this, human(this))).length, phrase.startsWith('no ') ? 0 : 1);
  }
);

Then('that cell belongs to no other account on the doorway', async function (this: E2EWorld) {
  const person = human(this);
  const admin = await this.getAdminClient(person.doorwayUrl);
  const token = admin.session?.token;
  assert.ok(token, 'The admin client has no session.');
  const result = await api(`${person.doorwayUrl}/admin/hosted-users`, token);
  assert.equal(result.status, 200);
  assert.ok(Array.isArray(result.body['users']), 'The hosted-user listing omitted users.');
  const owners = (result.body['users'] as Json[]).filter(
    row => row['agentPubKey'] === person.agentPubKey
  );
  assert.deepEqual(
    owners.map(row => row['identifier']),
    [person.human.credentials.identifier]
  );
});

browserWhen('the human opens their doorway account page', async (world, device) => {
  await device.navigate(`${human(world).doorwayUrl}${ACCOUNT_PATH}`);
  await device.page.getByTestId(ACCOUNT.BACK).waitFor({ state: 'visible' });
});

browserThen(
  'the account page shows the identifier the doorway issued at registration',
  async (world, device) => {
    const identifier = human(world).human.credentials.identifier;
    const testId = device.page.getByTestId(TEST_ID.accountIdentifier);
    const shown = (await testId.count())
      ? testId
      : device.page.getByText(identifier, { exact: false });
    await shown.first().waitFor({ state: 'visible' });
  }
);

browserThen(
  'the agency pipeline marks {string} as the current step',
  async (_world, device, label) => {
    const step = (await pipeline(device)).find(item => item.label === label);
    assert.ok(step, `Agency pipeline has no "${label}" step.`);
    assert.ok(step.classes.includes('current'), `${label} is not the current agency step.`);
  }
);

browserThen('the agency pipeline marks no later step as completed', async (_world, device) => {
  const steps = await pipeline(device);
  const current = steps.findIndex(step => step.classes.includes('current'));
  assert.ok(current >= 0, 'Agency pipeline has no current step.');
  assert.ok(steps.slice(current + 1).every(step => !step.classes.includes('completed')));
});

browserWhen('the human begins closing their account', async (_world, device) => {
  await beginClosure(device);
});

browserThen(
  'the portal asks the human to confirm by typing their identifier',
  async (_world, device) => {
    await device.page.getByTestId(TEST_ID.closeInput).waitFor({ state: 'visible' });
  }
);

browserWhen('the human confirms with the wrong identifier', async (world, device) => {
  world.expectBrowserError(/status of 400/);
  human(world).attempt = await submitClosure(device, `wrong-${randomUUID()}`);
});

browserThen('the account is not closed', async (world, device) => {
  const attempt = human(world).attempt;
  assert.ok(attempt, 'No close-account response was observed.');
  assert.equal(attempt.status, 400);
  assert.equal(attempt.body['code'], 'CONFIRMATION_MISMATCH');
  await device.page.getByTestId(TEST_ID.closeError).waitFor({ state: 'visible' });
});

Then('the doorway still confirms a session for that human', async function (this: E2EWorld) {
  const person = human(this);
  const result = await api(`${person.doorwayUrl}/auth/me`, person.token);
  assert.equal(result.status, 200);
  assert.equal(result.body['identifier'], person.human.credentials.identifier);
});

browserWhen(
  /^the human (confirms with their own identifier|closes their account through the portal)$/,
  async (world, device, action) => {
    const person = human(world);
    person.preClosureToken ??= person.token;
    if (action.startsWith('closes')) await beginClosure(device);
    person.attempt = await submitClosure(device, person.human.credentials.identifier);
    assert.equal(person.attempt.status, 200);
    assert.equal(person.attempt.body['closed'], true);
  }
);

browserThen('the portal returns to the signed-out doorway landing', async (_world, device) => {
  await device.page.waitForURL(
    url => url.pathname === '/threshold/' || url.pathname === '/threshold'
  );
  await device.page.getByTestId(LANDING.SIGN_IN).waitFor({ state: 'visible' });
  const profile = device.page.getByTestId(TOOLBAR.PROFILE_BUBBLE);
  assert.ok((await profile.count()) === 0 || !(await profile.first().isVisible()));
});

browserWhen(
  'the human attempts to sign in through the portal with the password they registered with',
  async (world, device) => {
    const person = human(world);
    world.expectBrowserError(/status of 401/);
    const login = new ThresholdLoginPage(device.page);
    await login.login(person.localPart, person.human.credentials.password);
    await login.waitForError(20_000);
  }
);

Then(
  "the doorway's account store holds no active account for that identifier",
  async function (this: E2EWorld) {
    const person = human(this);
    const admin = await this.getAdminClient(person.doorwayUrl);
    const result = await admin.adminListUsers({
      search: person.human.credentials.identifier,
      limit: 50,
    });
    assert.equal(
      result.users.filter(
        row => row.identifier === person.human.credentials.identifier && row.isActive
      ).length,
      0
    );
  }
);

browserWhen('a second browser opens the doorway registration portal', async (world, _first) => {
  const doorwayUrl = human(world).doorwayUrl;
  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
  const browser = await world.getBrowser();
  // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
  const device = new PlaywrightDevice('second-hosted-human-pw', doorwayUrl, doorwayUrl, browser);
  await device.init();
  portal(world).device = device;
  world.onCleanup(async () => device.close());
  await device.navigate(`${doorwayUrl}${REGISTER_PATH}`);
  await device.page.getByTestId(THRESHOLD_REGISTER.DISPLAY_NAME).waitFor({ state: 'visible' });
});

Then(
  "the two humans hold different cells on the doorway's pool conductors",
  async function (this: E2EWorld) {
    const first = human(this, 0);
    const second = human(this, 1);
    assert.notEqual(first.agentPubKey, second.agentPubKey);
    assert.equal((await cellsFor(this, first)).length, 1);
    assert.equal((await cellsFor(this, second)).length, 1);
  }
);

Given('the human has closed their account', async function (this: E2EWorld) {
  const person = human(this);
  person.preClosureToken = person.token;
  person.attempt = await closeAccount(person, person.token);
  assert.equal(person.attempt.status, 200);
  assert.equal(person.attempt.body['closed'], true);
});

When(
  'the closure is requested again with the session the human held before closing',
  async function (this: E2EWorld) {
    const person = human(this);
    assert.ok(person.preClosureToken, 'The pre-closure session was not retained.');
    person.attempt = await closeAccount(person, person.preClosureToken);
  }
);

Then('the doorway answers that the account is already closed', function (this: E2EWorld) {
  const attempt = human(this).attempt;
  assert.ok(attempt, 'No repeated close-account response was observed.');
  assert.equal(attempt.status, 200);
  assert.equal(attempt.body['alreadyClosed'], true);
});

// ---------------------------------------------------------------------------
// Station 7 — "Hosted by a household" (07-hosted-by-a-household.feature).
//
// Reuses every primitive above (remember, registerThroughPortal, cellsFor,
// browserWhen/browserThen, api, the close-account helpers) rather than
// re-minting them. The notary-read and steward-key primitives this needs live
// in src/framework/fixtures/hosted-cell.ts (Decisions 1 and 2, S1 plan Task 4
// "Chief decisions on the S1 blind-reader findings", 2026-09-11): a
// `hosted-cell` commitment is read back BY CID from jessica's storage — a
// household peer that is not this doorway's pool — never through the doorway;
// and a pool conductor's steward key resolves from that peer's OWN storage
// self-identity, never from the doorway's opinion about who it is.
//
// This runs RED until S2 Task 13 issues the grant and S3 Task 15/16 land its
// scope + provider-side revoke — the correct state for a story-first check.
// ---------------------------------------------------------------------------

/** `alpha` is the only doorway this station's Background ever declares. */
const STATION_7_DOORWAY_ID = 'alpha';

/** Guard message for every step that assumes `readBackCommitment` already ran. */
const NO_COMMITMENT_READ_YET = 'No commitment has been read back yet.';

async function ensurePlaywrightPortalOpen(
  world: E2EWorld,
  doorwayId: string
): Promise<PlaywrightDevice> {
  const existing = portal(world).device;
  if (existing) return existing;
  const doorway = world.getDoorway(doorwayId);
  const base = withoutTrailingSlash(doorway.url);
  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
  const browser = await world.getBrowser();
  // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
  const device = new PlaywrightDevice('newcomer-pw', base, base, browser);
  await device.init();
  Object.assign(portal(world), { device, doorwayUrl: base });
  world.onCleanup(async () => device.close());
  return device;
}

/**
 * A newcomer's registration, mode-aware: through the real portal in Playwright
 * mode (so a later browser step can navigate the SAME authenticated session),
 * through the HTTP API otherwise. Mirrors `05-leaving`'s own split between
 * `registerThroughPortal` and the plain `BrowserDevice` register — this just
 * enters it from one combined phrase instead of two separate ones.
 */
async function registerNewcomer(
  world: E2EWorld,
  displayName: string,
  doorwayId: string = STATION_7_DOORWAY_ID
): Promise<HostedHuman> {
  if (world.deviceMode === 'playwright') {
    const device = await ensurePlaywrightPortalOpen(world, doorwayId);
    await device.navigate(`${portal(world).doorwayUrl}${REGISTER_PATH}`);
    await device.page.getByTestId(THRESHOLD_REGISTER.DISPLAY_NAME).waitFor({ state: 'visible' });
    await registerThroughPortal(world, displayName);
    return human(world);
  }
  const doorway = world.getDoorway(doorwayId);
  const localPart = `hh-${randomUUID()}`;
  const device = new BrowserDevice(`${localPart}-api`, doorway.url);
  const auth = await device.register({ identifier: localPart, password: PASSWORD, displayName });
  remember(
    world,
    displayName,
    localPart,
    doorwayId,
    withoutTrailingSlash(doorway.url),
    auth,
    device
  );
  return human(world);
}

/** Read `hostedCellGrantCid` off `/auth/account` and cache it on the person. */
async function captureGrantCid(person: HostedHuman): Promise<string> {
  const result = await api(`${person.doorwayUrl}/auth/account`, person.token);
  assert.equal(
    result.status,
    200,
    `GET /auth/account for "${person.human.credentials.identifier}" returned ${result.status}.`
  );
  const cid = result.body['hostedCellGrantCid'];
  assert.ok(
    typeof cid === 'string' && cid.length > 0,
    'GET /auth/account carries no hostedCellGrantCid — S2 Task 13 (issue the hosted-cell grant ' +
      'on register) has not landed on this doorway yet.'
  );
  person.grantCid = cid;
  return cid;
}

async function grantCidFor(person: HostedHuman): Promise<string> {
  return person.grantCid ?? captureGrantCid(person);
}

/**
 * Decision 1: read the commitment back by cid from a peer that is not the
 * doorway's pool — convergence-bounded (the off-pool read-back gap, S1 plan
 * 2026-09-11): a freshly-minted grant's cid can 404 on the non-authoring
 * peer for a while before its DHT view catches up, so this polls the same
 * GET within the household fixture's declared `convergenceWindowMs` rather
 * than asserting 200 on a single attempt.
 */
async function readBackCommitment(person: HostedHuman): Promise<CommitmentBody> {
  const cid = await grantCidFor(person);
  const { body } = await readHostedCellCommitmentConverged(cid);
  person.commitment = body;
  return body;
}

/** Decision 2: the steward key the pool conductor's own peer names as itself. */
async function resolveStewardAgentPubKey(world: E2EWorld, person: HostedHuman): Promise<string> {
  if (person.stewardAgentPubKey) return person.stewardAgentPubKey;
  const admin = await world.getAdminClient(person.doorwayUrl);
  const agentConductor = await admin.adminAgentConductor(person.agentPubKey);
  const steward = stewardAgentPubKeyForConductorOrigin(agentConductor.conductorUrl);
  assert.ok(
    steward,
    "no household fixture storage peer matches the pool conductor's origin " +
      `(${agentConductor.conductorUrl}) — set agentPubKey on that peer in ` +
      'E2E_HOUSEHOLD_FIXTURE_PATH (stamped by hc-mesh.sh refresh_fixture_pids).'
  );
  person.stewardAgentPubKey = steward;
  return person.stewardAgentPubKey;
}

Given('a newcomer who has never registered at this doorway', function (this: E2EWorld) {
  // Sentinel only — the account itself is created by the paired "When ...
  // creates an account" step below. Recorded so the scenario reads naturally;
  // there is nothing to arrange yet for someone who does not exist.
});

When(
  'they create an account at this doorway with the display name {string}',
  async function (this: E2EWorld, displayName: string) {
    await registerNewcomer(this, displayName);
  }
);

Given('a newcomer who has created an account at this doorway', async function (this: E2EWorld) {
  await registerNewcomer(this, `Newcomer ${randomUUID().slice(0, 8)}`);
});

Given(
  'a second newcomer who has created an account at this doorway',
  async function (this: E2EWorld) {
    await registerNewcomer(this, `Second newcomer ${randomUUID().slice(0, 8)}`);
  }
);

Then(
  'the doorway hosts a cell for them on one of its pool conductors',
  async function (this: E2EWorld) {
    assert.equal((await cellsFor(this, human(this))).length, 1);
  }
);

Then(
  "no other account at this doorway shares that cell's agent key",
  async function (this: E2EWorld) {
    const person = human(this);
    const admin = await this.getAdminClient(person.doorwayUrl);
    const token = admin.session?.token;
    assert.ok(token, 'The admin client has no session.');
    const result = await api(`${person.doorwayUrl}/admin/hosted-users`, token);
    assert.equal(result.status, 200);
    assert.ok(Array.isArray(result.body['users']), 'The hosted-user listing omitted users.');
    const owners = (result.body['users'] as Json[]).filter(
      row => row['agentPubKey'] === person.agentPubKey
    );
    assert.deepEqual(
      owners.map(row => row['identifier']),
      [person.human.credentials.identifier]
    );
  }
);

Then(
  'the notary records a live {string} delegates-compute commitment naming that agent key as recipient',
  async function (this: E2EWorld, scope: string) {
    const person = human(this);
    const body = await readBackCommitment(person);
    assert.ok(
      commitmentIsLive(body),
      `commitment ${String(person.grantCid)} is not live: ${JSON.stringify(body)}`
    );
    assert.equal(commitmentField(body, 'scope'), scope);
    assert.equal(commitmentField(body, 'recipient', 'receiver'), person.agentPubKey);
  }
);

Then(
  'that commitment names the steward of that pool conductor as provider',
  async function (this: E2EWorld) {
    const person = human(this);
    assert.ok(person.commitment, NO_COMMITMENT_READ_YET);
    const steward = await resolveStewardAgentPubKey(this, person);
    assert.equal(commitmentField(person.commitment, 'provider'), steward);
  }
);

Then('that commitment carries an end date in the future', function (this: E2EWorld) {
  const person = human(this);
  assert.ok(person.commitment, NO_COMMITMENT_READ_YET);
  const raw = commitmentField(person.commitment, 'validUntil', 'valid_until', 'hasEnd', 'has_end');
  assert.equal(
    typeof raw,
    'string',
    `commitment carries no end-date field: ${JSON.stringify(person.commitment)}`
  );
  const until = Date.parse(raw as string);
  assert.ok(
    !Number.isNaN(until) && until > Date.now(),
    `end date "${String(raw)}" is not in the future`
  );
});

browserWhen('they open their doorway account page', async (world, device) => {
  await device.navigate(`${human(world).doorwayUrl}${ACCOUNT_PATH}`);
  await device.page.getByTestId(ACCOUNT.BACK).waitFor({ state: 'visible' });
});

browserThen('the page names the household hosting them', async (_world, device) => {
  const el = device.page.getByTestId(TEST_ID.hostedByHousehold);
  await el.waitFor({ state: 'visible' });
  assert.ok((await el.innerText()).trim().length > 0, 'The hosted-by-household element is empty.');
});

browserThen('the page names the date their hosting is promised until', async (_world, device) => {
  const el = device.page.getByTestId(TEST_ID.hostedUntil);
  await el.waitFor({ state: 'visible' });
  assert.ok((await el.innerText()).trim().length > 0, 'The hosted-until element is empty.');
});

browserThen(
  "the page never shows the raw commitment identifier as the household's name",
  async (world, device) => {
    const person = human(world);
    const el = device.page.getByTestId(TEST_ID.hostedByHousehold);
    const text = (await el.innerText()).trim();
    let grantCid: string | undefined;
    try {
      grantCid = await grantCidFor(person);
    } catch {
      grantCid = undefined; // S2 Task 13 not landed yet — the shape check below still applies.
    }
    if (grantCid) {
      assert.notEqual(text, grantCid, 'The household name shows the raw commitment cid.');
    }
    assert.ok(
      !/^u[A-Za-z0-9_-]{40,}$/.test(text),
      `"${text}" looks like a raw notarized hash, not a household's name.`
    );
  }
);

When("the doorway names that human's hosting commitment", async function (this: E2EWorld) {
  await captureGrantCid(human(this));
});

When(
  "that commitment is read back, by its identifier, from a household peer that is not the doorway's pool",
  async function (this: E2EWorld) {
    await readBackCommitment(human(this));
  }
);

Then("the read-back commitment's scope is {string}", function (this: E2EWorld, scope: string) {
  const person = human(this);
  assert.ok(person.commitment, NO_COMMITMENT_READ_YET);
  assert.equal(commitmentField(person.commitment, 'scope'), scope);
});

Then(
  "its provider is the agent key that the pool conductor's own peer names as its steward",
  async function (this: E2EWorld) {
    const person = human(this);
    assert.ok(person.commitment, NO_COMMITMENT_READ_YET);
    const steward = await resolveStewardAgentPubKey(this, person);
    assert.equal(commitmentField(person.commitment, 'provider'), steward);
  }
);

Then("its provider is not the doorway's own service identity", function (this: E2EWorld) {
  const person = human(this);
  assert.ok(person.commitment, NO_COMMITMENT_READ_YET);
  const provider = commitmentField(person.commitment, 'provider');
  assert.ok(
    typeof provider === 'string' && provider.length > 0,
    `commitment carries no provider field: ${JSON.stringify(person.commitment)}`
  );
  const serviceIdentity = doorwayServiceIdentityAgentPubKey(STATION_7_DOORWAY_ID);
  if (serviceIdentity) {
    assert.notEqual(provider, serviceIdentity);
  }
});

Then("its recipient is that human's own agent key", function (this: E2EWorld) {
  const person = human(this);
  assert.ok(person.commitment, NO_COMMITMENT_READ_YET);
  assert.equal(commitmentField(person.commitment, 'recipient', 'receiver'), person.agentPubKey);
});

When(
  "both of their hosting commitments are read back from a household peer that is not the doorway's pool",
  async function (this: E2EWorld) {
    await readBackCommitment(human(this, 0));
    await readBackCommitment(human(this, 1));
  }
);

Then('the two humans hold different cells', async function (this: E2EWorld) {
  const first = human(this, 0);
  const second = human(this, 1);
  assert.notEqual(first.agentPubKey, second.agentPubKey);
  assert.equal((await cellsFor(this, first)).length, 1);
  assert.equal((await cellsFor(this, second)).length, 1);
});

Then('each holds their own {string} commitment', function (this: E2EWorld, scope: string) {
  for (const index of [0, 1]) {
    const person = human(this, index);
    assert.ok(person.commitment, `Human ${index + 1} has no commitment read back yet.`);
    assert.equal(commitmentField(person.commitment, 'scope'), scope);
  }
  assert.notEqual(human(this, 0).grantCid, human(this, 1).grantCid);
});

Then('neither commitment names the other human as recipient', function (this: E2EWorld) {
  const first = human(this, 0);
  const second = human(this, 1);
  const firstRecipient = commitmentField(
    first.commitment as CommitmentBody,
    'recipient',
    'receiver'
  );
  const secondRecipient = commitmentField(
    second.commitment as CommitmentBody,
    'recipient',
    'receiver'
  );
  assert.notEqual(firstRecipient, second.agentPubKey);
  assert.notEqual(secondRecipient, first.agentPubKey);
});

Given('the agent key of the cell the doorway runs for them', function (this: E2EWorld) {
  assert.ok(human(this).agentPubKey, 'The newcomer has no agent key yet.');
});

Given(
  'the identifier of their live {string} commitment',
  async function (this: E2EWorld, _scope: string) {
    await captureGrantCid(human(this));
  }
);

Given(
  'the notary records a live {string} commitment for them',
  async function (this: E2EWorld, scope: string) {
    const person = human(this);
    const body = await readBackCommitment(person);
    assert.ok(
      commitmentIsLive(body),
      `commitment ${String(person.grantCid)} is not live: ${JSON.stringify(body)}`
    );
    assert.equal(commitmentField(body, 'scope'), scope);
  }
);

browserWhen('they close their account through the portal', async (world, device) => {
  const person = human(world);
  await beginClosure(device);
  person.attempt = await submitClosure(device, person.human.credentials.identifier);
  assert.equal(person.attempt.status, 200);
  assert.equal(person.attempt.body['closed'], true);
});

Then('no pool conductor holds a cell for that agent key', async function (this: E2EWorld) {
  assert.equal((await cellsFor(this, human(this))).length, 0);
});

Then(
  'the notary records no live {string} commitment for that agent key',
  async function (this: E2EWorld, scope: string) {
    const person = human(this);
    assert.ok(person.grantCid, 'No commitment identifier was captured before closing.');
    const { status, body } = await readHostedCellCommitment(person.grantCid);
    if (status === 404) return; // no record under this cid at all is also "no live commitment".
    assert.equal(status, 200, `GET /api/v1/commitments/${person.grantCid} returned ${status}.`);
    assert.equal(commitmentField(body, 'scope'), scope);
    assert.ok(
      !commitmentIsLive(body),
      `commitment ${person.grantCid} is still live after closing: ${JSON.stringify(body)}`
    );
  }
);

Then(
  "a household peer that is not the doorway's pool still reads the commitment with that " +
    'identifier back, carrying the date it was made and the date it ended',
  async function (this: E2EWorld) {
    const person = human(this);
    assert.ok(person.grantCid, 'No commitment identifier was captured before closing.');
    const { status, body } = await readHostedCellCommitment(person.grantCid);
    assert.equal(
      status,
      200,
      `GET /api/v1/commitments/${person.grantCid} on ${NON_POOL_PEER_NAME}'s storage returned ` +
        `${status} — the notary must still answer for a withdrawn commitment.`
    );
    const made = commitmentField(
      body,
      'issuedAt',
      'issued_at',
      'createdAt',
      'created_at',
      'signedAt',
      'signed_at'
    );
    const ended = commitmentField(
      body,
      'revokedAt',
      'revoked_at',
      'endedAt',
      'ended_at',
      'validUntil',
      'valid_until'
    );
    assert.ok(
      typeof made === 'string' && made.length > 0,
      `commitment carries no "made" timestamp: ${JSON.stringify(body)}`
    );
    assert.ok(
      typeof ended === 'string' && ended.length > 0,
      `commitment carries no "ended" timestamp: ${JSON.stringify(body)}`
    );
  }
);
