/**
 * Doorway sign-in portal steps — the chaperone portal, driven through the doorway.
 *
 * Backs:
 *   - features/browser/doorway-portal-login.feature
 *
 * Where the truth lives:
 *   - doorway/doorway-app/src/app/components/login/threshold-login.component.ts
 *       the portal itself, and the testids asserted here (threshold-identifier,
 *       threshold-password, threshold-submit, threshold-error).
 *   - doorway/doorway-app/src/environments/environment.ts — `doorwayUrl: ''`,
 *       i.e. SAME ORIGIN. The portal calls the doorway that serves it, so these
 *       steps navigate to the DOORWAY origin, never to the SPA's dev server.
 *   - doorway/doorway-app/src/app/services/doorway-session-token.store.ts
 *       `doorway_auth_token` — the localStorage key the portal writes on success
 *       and the auth interceptor reads.
 *   - doorway/doorway-service/src/routes/threshold.rs — forwards /threshold/*
 *       to THRESHOLD_URL with the path INTACT, which is why the portal is served
 *       under /threshold/ and not at the origin root.
 *
 * WHY THE SESSION ASSERTION GOES BACK TO THE SERVER: a portal can render a
 * flawless form, store a string, and have authenticated nobody. So the token the
 * portal stored is replayed to `GET /auth/me` and the DOORWAY has to name the same
 * human. DOM state alone is not evidence that a session exists.
 */

import { strict as assert } from 'node:assert';
import { randomUUID } from 'node:crypto';

import { Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { ThresholdLoginPage } from '../../src/framework/pages/threshold-login.page.js';

import type { E2EWorld } from '../../src/framework/world.js';

/** localStorage key shared with doorway-app's auth interceptor. */
const AUTH_TOKEN_KEY = 'doorway_auth_token';

/** The portal path the doorway serves the SPA under. */
const PORTAL_PATH = '/threshold/login';

interface PortalState {
  /** What the human types into the portal, and the whole identifier on the wire. */
  localPart: string;
  /**
   * What the DOORWAY says it stored for that human, read back from the register
   * response. Bare on a doorway with no configured URL, `<local>@<gatewayDomain>`
   * on a fleet doorway — either way it is this deployment's own answer, not ours.
   */
  canonicalIdentifier: string;
  password: string;
  device: PlaywrightDevice;
  doorwayUrl: string;
}

function portal(world: E2EWorld): PortalState {
  const w = world as unknown as { __portal?: PortalState };
  if (!w.__portal?.device) {
    throw new Error(
      'No portal browser on this scenario — the Background steps "a hosted human is ' +
        'registered on doorway ..." and "a browser is open on doorway ..." must both run first.'
    );
  }
  return w.__portal;
}

function stash(world: E2EWorld): Partial<PortalState> {
  const w = world as unknown as { __portal?: Partial<PortalState> };
  w.__portal ??= {};
  return w.__portal;
}

function doorwayBase(world: E2EWorld, id: string): string {
  let base = world.getDoorway(id).url;
  while (base.endsWith('/')) base = base.slice(0, -1);
  return base;
}

/*
 * WHY THIS ASKS THE DOORWAY WHAT IT CALLS THE HUMAN, INSTEAD OF WORKING IT OUT.
 *
 * The portal RENDERS a domain suffix beside the identifier field
 * (`threshold-domain-suffix`, from `gatewayDomain()` =
 * `window.location.hostname` minus a `doorway-` prefix), so a human reads
 * "you are signing in as name@<doorway>". It does NOT send that domain: the
 * submitted body is `{"identifier":"<exactly what was typed>"}`, measured on the
 * local mesh 2026-08-28. So these steps type the bare local part — exactly the
 * string the portal will submit.
 *
 * What the doorway STORES is a different question, and it depends on how that
 * doorway is deployed (`doorway-service/src/routes/auth_routes.rs`,
 * `gateway_domain` + `normalize_identifier`): with `--doorway-url` configured it
 * re-qualifies the local part to `<local>@<gateway domain>` on register AND on
 * login, so `GET /auth/me` names the human `portal-a2o-<uuid>@alpha.elohim.host`
 * (genesis #1519, 2026-08-29); with no URL configured it stores the identifier
 * verbatim. Asserting the bare name is what failed that build — on an identifier
 * convention, not on sign-in. The session was real.
 *
 * The cure is NOT to re-derive the convention here. Re-implementing
 * `gateway_domain` in TypeScript would make a second hand-maintained home for a
 * rule that already has one, guarded by nothing but a comment asking the two to
 * agree — and it would still be wrong wherever a test reaches a doorway at an
 * address other than the one that doorway was configured with.
 *
 * Instead: registration ASKS. `POST /auth/register` answers with the canonical
 * identifier it stored (`AuthResponse.identifier`, auth_routes.rs:168), so the
 * doorway states its own convention once, in the deployment under test, and the
 * scenario holds it to that answer afterwards. The anchor that keeps this from
 * being circular is the uuid: the local part of what the doorway returns must be
 * the local part this scenario minted, so a doorway cannot satisfy it by naming
 * some other human. Then the portal's token must name that canonical identifier
 * EXACTLY — not one of several tolerated spellings.
 *
 * The rendered-but-not-submitted suffix is filed as backlog
 * `security-doorway-portal-domain-suffix-not-submitted`; this identifier
 * convention as `portal-login-step-domain-scoped-identifier`.
 */

Given(
  'a hosted human is registered on doorway {string}',
  async function (this: E2EWorld, doorwayId: string) {
    const base = doorwayBase(this, doorwayId);
    // Registered through the API rather than drawn from the fixture cast, so this
    // feature runs against ANY doorway — local mesh, hybrid, or deployed — without
    // depending on that doorway having been seeded with the household personas.
    // Registered under exactly what the portal submits — see the note above.
    const localPart = `portal-a2o-${randomUUID()}`;
    const password = `portal-a2o-${randomUUID()}`;

    const res = await request(`${base}/auth/register`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      // The bare local part — see the note above: it is the whole identifier the
      // portal will submit, so it is what this human is registered under.
      body: JSON.stringify({ identifier: localPart, password, displayName: 'Portal A2O' }),
    });
    const body = await res.body.text();
    assert.ok(
      res.statusCode < 300,
      `could not register a hosted human on "${doorwayId}": ${res.statusCode} ${body}`
    );

    // The doorway names the human it just stored. Which spelling that is depends
    // on how THIS doorway is deployed, so it is read rather than derived.
    const registered = JSON.parse(body) as { identifier?: string };
    const canonicalIdentifier = registered.identifier;
    assert.ok(
      typeof canonicalIdentifier === 'string' && canonicalIdentifier.length > 0,
      `the doorway registered a human without naming it, so nothing downstream can check ` +
        `whose session the portal mints: ${body}`
    );
    // The anchor that stops this being circular: whatever domain the doorway may
    // qualify with, the LOCAL PART has to be the one-off name minted here. A
    // doorway cannot satisfy the scenario by answering for a different human.
    assert.equal(
      canonicalIdentifier.split('@')[0],
      localPart,
      `registration asked for "${localPart}" and the doorway stored ` +
        `"${canonicalIdentifier}" — that is a different human, not a domain convention`
    );

    const s = stash(this);
    s.localPart = localPart;
    s.canonicalIdentifier = canonicalIdentifier;
    s.password = password;
    s.doorwayUrl = base;
  }
);

Given('a browser is open on doorway {string}', async function (this: E2EWorld, doorwayId: string) {
  const base = doorwayBase(this, doorwayId);
  // Both URLs are the DOORWAY: the portal is served by the doorway and calls it
  // same-origin, so there is no separate app origin in this flow.
  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
  const browser = await this.getBrowser();
  // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
  const device = new PlaywrightDevice('portal-pw', base, base, browser);
  await device.init();

  const s = stash(this);
  s.device = device;
  s.doorwayUrl = base;
  this.onCleanup(async () => device.close());
});

/** The portal's page object — selectors live in framework/pages/selectors.ts. */
function loginPage(s: PortalState): ThresholdLoginPage {
  return new ThresholdLoginPage(s.device.page);
}

When('the browser opens the doorway sign-in portal', async function (this: E2EWorld) {
  const s = portal(this);
  // Absolute URL: PlaywrightDevice.navigate uses the app base for relative paths,
  // and the portal lives on the doorway origin under /threshold/.
  await s.device.navigate(`${s.doorwayUrl}${PORTAL_PATH}`);
  await loginPage(s).waitForReady();
});

Then('the portal renders its sign-in form', async function (this: E2EWorld) {
  const s = portal(this);
  assert.ok(
    await loginPage(s).formIsRendered(),
    `the portal did not render a complete sign-in form at ${s.device.page.url()} — ` +
      'the doorway served something, but not the form a human could sign in with'
  );
});

When('the human signs in through the portal', async function (this: E2EWorld) {
  const s = portal(this);
  // Local part ONLY — the portal renders a domain beside the field but does not
  // submit it (see the note above), so this is the whole identifier on the wire.
  await loginPage(s).login(s.localPart, s.password);
  // The portal stores the token before it leaves the sign-in step; wait on the
  // token rather than on navigation, so this does not depend on where it routes to.
  await s.device.page.waitForFunction(
    (key: string) => globalThis.localStorage.getItem(key) !== null,
    AUTH_TOKEN_KEY,
    { timeout: 20_000 }
  );
});

When('the human submits a wrong password through the portal', async function (this: E2EWorld) {
  const s = portal(this);
  await loginPage(s).login(s.localPart, `definitely-not-${s.password}`);
  await loginPage(s).waitForError(20_000);
});

Then('the portal shows a sign-in error', async function (this: E2EWorld) {
  const s = portal(this);
  assert.ok(
    await loginPage(s).hasError(),
    'the portal did not show its error banner after a wrong password'
  );
});

/** Read whatever token the portal stored, if any. */
async function storedToken(s: PortalState): Promise<string | null> {
  const raw = await s.device.page.evaluate(
    (key: string) => globalThis.localStorage.getItem(key),
    AUTH_TOKEN_KEY
  );
  return typeof raw === 'string' ? raw : null;
}

Then('the doorway confirms a session for that human', async function (this: E2EWorld) {
  const s = portal(this);
  const token = await storedToken(s);
  assert.ok(token, `the portal stored no "${AUTH_TOKEN_KEY}" — nothing was minted`);

  // The load-bearing assertion: the DOORWAY answers for this token, and names the
  // human who signed in. A stored string is not a session.
  const res = await request(`${s.doorwayUrl}/auth/me`, {
    method: 'GET',
    headers: { authorization: `Bearer ${token}` },
  });
  const body = await res.body.text();
  assert.equal(
    res.statusCode,
    200,
    `the doorway refused the token the portal minted: ${res.statusCode} ${body}`
  );
  const me = JSON.parse(body) as { identifier?: string };
  // Exact: the token the PORTAL minted must resolve to the same account
  // REGISTRATION created, under the very name this doorway said it stored.
  assert.equal(
    me.identifier,
    s.canonicalIdentifier,
    `the doorway named "${me.identifier}" for the portal's token, but registration ` +
      `stored "${s.canonicalIdentifier}" — the portal signed somebody else in`
  );
});

Then('the doorway confirms no session for that human', async function (this: E2EWorld) {
  const s = portal(this);
  const token = await storedToken(s);
  assert.equal(
    token,
    null,
    'a refused sign-in still stored a session token — the portal minted something it should not have'
  );
});
