/** Hosted-human lifecycle: portal registration, account inspection, and clean closure. */
import { strict as assert } from 'node:assert';
import { randomUUID } from 'node:crypto';

import { Given, Then, When } from '@cucumber/cucumber';

import { request } from 'undici';

import { BrowserDevice } from '../../src/framework/devices/browser-device.js';
import { PlaywrightDevice, type PWPage } from '../../src/framework/devices/playwright-device.js';
import { Human } from '../../src/framework/human.js';
import { ACCOUNT, LANDING, THRESHOLD_REGISTER, TOOLBAR } from '../../src/framework/pages/selectors.js';
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
} as const;

type Json = Record<string, unknown>;
type BrowserResponse = { url(): string; status(): number; text(): Promise<string>; request(): { method(): string } };
type ResponsePage = PWPage & { waitForResponse(predicate: (response: BrowserResponse) => boolean, options?: Json): Promise<BrowserResponse> };
type PortalBridge = { localPart?: string; canonicalIdentifier?: string; password?: string; device?: PlaywrightDevice; doorwayUrl?: string };
type Attempt = { status: number; body: Json };
type HostedHuman = {
  human: Human;
  localPart: string;
  doorwayUrl: string;
  token: string;
  agentPubKey: string;
  registrationDisplayName?: string;
  preClosureToken?: string;
  attempt?: Attempt;
};
type PipelineStep = { label: string; classes: string[] };

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

// eslint-disable-next-line sonarjs/function-return-type -- null is the documented HTTP-mode guard
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
  return (page as ResponsePage).waitForResponse(response => new URL(response.url()).pathname === path && response.request().method() === 'POST', { timeout: 30_000 });
}

function remember(world: E2EWorld, displayName: string, localPart: string, doorwayId: string, doorwayUrl: string, auth: AuthResponse, device: BrowserDevice | PlaywrightDevice): void {
  assert.equal(auth.identifier.split('@')[0], localPart, 'The doorway registered a different human.');
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
    assert.fail(`Registration returned ${response.status()}: ${(await error.count()) ? await error.innerText() : text}`);
  }
  await device.page.waitForFunction((key: string) => globalThis.localStorage.getItem(key) !== null, AUTH_TOKEN_KEY, { timeout: 20_000 });
  const doorwayId = [...world.doorways].find(([, d]) => withoutTrailingSlash(d.url) === doorwayUrl)?.[0];
  assert.ok(doorwayId, 'The portal origin is not a registered doorway.');
  remember(world, displayName, localPart, doorwayId, doorwayUrl, json(text) as unknown as AuthResponse, device);
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
  const listings = await Promise.all(conductors.map(async c => admin.adminConductorAgents(c.conductorId)));
  return listings.flatMap(c => c.agents.filter(agent => agent.agentPubKey === person.agentPubKey).map(() => c.conductorId));
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

function browserWhen(pattern: string | RegExp, body: BrowserBody): void {
  When(pattern, async function (this: E2EWorld, ...args: string[]) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';
    await body(this, device, ...args);
    return undefined;
  });
}

function browserThen(pattern: string | RegExp, body: BrowserBody): void {
  Then(pattern, async function (this: E2EWorld, ...args: string[]) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';
    await body(this, device, ...args);
    return undefined;
  });
}

browserWhen('the browser opens the doorway registration portal', async (world, device) => {
  const doorwayUrl = portal(world).doorwayUrl;
  assert.ok(doorwayUrl, 'The browser has no doorway origin.');
  await device.navigate(`${doorwayUrl}${REGISTER_PATH}`);
  await device.page.getByTestId(THRESHOLD_REGISTER.DISPLAY_NAME).waitFor({ state: 'visible' });
});

browserThen('the portal renders its registration form', async (_world, device) => {
  await Promise.all([THRESHOLD_REGISTER.DISPLAY_NAME, THRESHOLD_REGISTER.EMAIL, THRESHOLD_REGISTER.PASSWORD, THRESHOLD_REGISTER.CONFIRM_PASSWORD, THRESHOLD_REGISTER.SUBMIT].map(async id => device.page.getByTestId(id).waitFor({ state: 'visible' })));
});

browserWhen(/^(?:the|that browser's) newcomer creates an account through the portal with the display name "([^"]+)"$/, async (world, _device, name) => {
  await registerThroughPortal(world, name);
});

Given('a hosted human {string} is registered on doorway {string}', async function (this: E2EWorld, name: string, doorwayId: string) {
  const doorway = this.getDoorway(doorwayId);
  const localPart = `hh-${randomUUID()}`;
  const device = new BrowserDevice(`${name}-api`, doorway.url);
  const auth = await device.register({
    identifier: localPart,
    password: PASSWORD,
    displayName: name,
  });
  remember(this, name, localPart, doorwayId, withoutTrailingSlash(doorway.url), auth, device);
});

Then(/^the doorway names (that|the first|the second) human "([^"]+)"$/, async function (this: E2EWorld, which: string, name: string) {
  let person = human(this);
  if (which === 'the first') person = human(this, 0);
  if (which === 'the second') person = human(this, 1);
  await assertDisplayName(person, name);
});

Then(/^(the doorway holds a cell for that human on one of its pool conductors|no pool conductor holds a cell for that human)$/, async function (this: E2EWorld, phrase: string) {
  assert.equal((await cellsFor(this, human(this))).length, phrase.startsWith('no ') ? 0 : 1);
});

Then('that cell belongs to no other account on the doorway', async function (this: E2EWorld) {
  const person = human(this);
  const admin = await this.getAdminClient(person.doorwayUrl);
  const token = admin.session?.token;
  assert.ok(token, 'The admin client has no session.');
  const result = await api(`${person.doorwayUrl}/admin/hosted-users`, token);
  assert.equal(result.status, 200);
  assert.ok(Array.isArray(result.body['users']), 'The hosted-user listing omitted users.');
  const owners = (result.body['users'] as Json[]).filter(row => row['agentPubKey'] === person.agentPubKey);
  assert.deepEqual(
    owners.map(row => row['identifier']),
    [person.human.credentials.identifier]
  );
});

browserWhen('the human opens their doorway account page', async (world, device) => {
  await device.navigate(`${human(world).doorwayUrl}${ACCOUNT_PATH}`);
  await device.page.getByTestId(ACCOUNT.BACK).waitFor({ state: 'visible' });
});

browserThen('the account page shows the identifier the doorway issued at registration', async (world, device) => {
  const identifier = human(world).human.credentials.identifier;
  const testId = device.page.getByTestId(TEST_ID.accountIdentifier);
  const shown = (await testId.count()) ? testId : device.page.getByText(identifier, { exact: false });
  await shown.first().waitFor({ state: 'visible' });
});

browserThen('the agency pipeline marks {string} as the current step', async (_world, device, label) => {
  const step = (await pipeline(device)).find(item => item.label === label);
  assert.ok(step, `Agency pipeline has no "${label}" step.`);
  assert.ok(step.classes.includes('current'), `${label} is not the current agency step.`);
});

browserThen('the agency pipeline marks no later step as completed', async (_world, device) => {
  const steps = await pipeline(device);
  const current = steps.findIndex(step => step.classes.includes('current'));
  assert.ok(current >= 0, 'Agency pipeline has no current step.');
  assert.ok(steps.slice(current + 1).every(step => !step.classes.includes('completed')));
});

browserWhen('the human begins closing their account', async (_world, device) => {
  await beginClosure(device);
});

browserThen('the portal asks the human to confirm by typing their identifier', async (_world, device) => {
  await device.page.getByTestId(TEST_ID.closeInput).waitFor({ state: 'visible' });
});

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

browserWhen(/^the human (confirms with their own identifier|closes their account through the portal)$/, async (world, device, action) => {
  const person = human(world);
  person.preClosureToken ??= person.token;
  if (action.startsWith('closes')) await beginClosure(device);
  person.attempt = await submitClosure(device, person.human.credentials.identifier);
  assert.equal(person.attempt.status, 200);
  assert.equal(person.attempt.body['closed'], true);
});

browserThen('the portal returns to the signed-out doorway landing', async (_world, device) => {
  await device.page.waitForURL(url => url.pathname === '/threshold/' || url.pathname === '/threshold');
  await device.page.getByTestId(LANDING.SIGN_IN).waitFor({ state: 'visible' });
  const profile = device.page.getByTestId(TOOLBAR.PROFILE_BUBBLE);
  assert.ok((await profile.count()) === 0 || !(await profile.first().isVisible()));
});

browserWhen('the human attempts to sign in through the portal with the password they registered with', async (world, device) => {
  const person = human(world);
  world.expectBrowserError(/status of 401/);
  const login = new ThresholdLoginPage(device.page);
  await login.login(person.localPart, person.human.credentials.password);
  await login.waitForError(20_000);
});

Then("the doorway's account store holds no active account for that identifier", async function (this: E2EWorld) {
  const person = human(this);
  const admin = await this.getAdminClient(person.doorwayUrl);
  const result = await admin.adminListUsers({
    search: person.human.credentials.identifier,
    limit: 50,
  });
  assert.equal(result.users.filter(row => row.identifier === person.human.credentials.identifier && row.isActive).length, 0);
});

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

Then("the two humans hold different cells on the doorway's pool conductors", async function (this: E2EWorld) {
  const first = human(this, 0);
  const second = human(this, 1);
  assert.notEqual(first.agentPubKey, second.agentPubKey);
  assert.equal((await cellsFor(this, first)).length, 1);
  assert.equal((await cellsFor(this, second)).length, 1);
});

Given('the human has closed their account', async function (this: E2EWorld) {
  const person = human(this);
  person.preClosureToken = person.token;
  person.attempt = await closeAccount(person, person.token);
  assert.equal(person.attempt.status, 200);
  assert.equal(person.attempt.body['closed'], true);
});

When('the closure is requested again with the session the human held before closing', async function (this: E2EWorld) {
  const person = human(this);
  assert.ok(person.preClosureToken, 'The pre-closure session was not retained.');
  person.attempt = await closeAccount(person, person.preClosureToken);
});

Then('the doorway answers that the account is already closed', function (this: E2EWorld) {
  const attempt = human(this).attempt;
  assert.ok(attempt, 'No repeated close-account response was observed.');
  assert.equal(attempt.status, 200);
  assert.equal(attempt.body['alreadyClosed'], true);
});
