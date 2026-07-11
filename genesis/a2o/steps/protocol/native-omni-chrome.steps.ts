/**
 * Native omni chrome-asset — browser-tier resilience-contract step definitions.
 *
 * Drives the "Native omni chrome resilience segment speaks the real snapshot
 * contract" scenario in
 * genesis/a2o/features/resilience/observable-distribution.feature.
 *
 * Distinct target from resilience.steps.ts's PROTOCOL_OMNI steps: those drive
 * the ANGULAR `<app-protocol-omni>` component (data-testid selectors,
 * elohim-app bundle). This file drives the hand-written, framework-free
 * chrome element the doorway SSR-splices onto EVERY served page
 * (elohim/elohim-chrome-asset/src/omni-element.js, id="elohim-omni") —
 * protocol-omnibar-chrome.feature already pins the SERVED-HTML half of that
 * contract (context island + content-addressed loader script, HTTP-only, "no
 * Playwright / browser tier — deliberately lean"); this file exercises the
 * rendered/behavioral half that feature explicitly does not cover.
 *
 * Selectors mirror the DOM the element itself renders — the native id plus
 * the data-omni-* hooks documented at the top of omni-element.js — there is
 * no data-testid convention on this element, so this file does not draw from
 * src/framework/pages/selectors.ts (that registry is explicitly scoped to
 * Angular data-testid contracts; see its file header).
 *
 * Device acquisition mirrors the anonymous `_landing-visitor` idiom in
 * steps/ui/browser-elements.steps.ts (and `_portal-visitor` in
 * steps/peer-oauth-portal/*): a synthetic human carries a Playwright device
 * pointed straight at the doorway origin, so the scenario runs as an
 * anonymous visitor with no login. Requires E2E_DEVICE_MODE=playwright.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { Human } from '../../src/framework/human.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Selectors — mirror omni-element.js's own markup, not a testid registry.
// ---------------------------------------------------------------------------

const OMNI = '#elohim-omni';
const TOGGLE = `${OMNI} [data-omni-action="toggle"]`;
const RESILIENCE_BUTTON = `${OMNI} [data-omni-action="resilience-toggle"]`;
const RESILIENCE_GLYPH = `${OMNI} [data-omni-resilience-glyph]`;
const RESILIENCE_CARD = `${OMNI} [data-omni-resilience-card]`;
const RESILIENCE_HEADLINE = `${RESILIENCE_CARD} .omni-resilience-headline`;

/// omni-element.js's applyResilience() renders the three-plane resilience
/// DIAL (an inline <svg>) into the glyph mark and testifies each plane's
/// state via data-omni-dial-* attributes (reach/lit/floor/sealed/measured).
/// The neutral ◉ text placeholder (no svg, no dial attrs) is the un-loaded
/// state the element ships with — and what the phantom-contract regression
/// left behind forever.
const DIAL_SVG = `${OMNI} [data-omni-resilience-glyph] svg`;
const DIAL_REACHES = ['local', 'hub', 'remote'];
const PHANTOM_TITLE = 'Resilience snapshot unavailable';

const NO_PW_DEVICE_PENDING = 'pending';
const LANDING_VISITOR = '_native-omni-visitor';

// ---------------------------------------------------------------------------
// Device acquisition — anonymous visitor pointed at the doorway origin.
// ---------------------------------------------------------------------------

/**
 * Resolve (or lazily create) an anonymous browser device pointed at the first
 * registered doorway. Mirrors `ensureLandingVisitor` in
 * steps/ui/browser-elements.steps.ts — kept as a local, file-owned helper
 * (this codebase's established idiom: each steps file owns its own small
 * device-acquisition helper rather than sharing one across files).
 */
async function ensureNativeOmniVisitor(world: E2EWorld): Promise<PlaywrightDevice | null> {
  let human: Human;
  try {
    human = world.getHuman(LANDING_VISITOR);
  } catch {
    human = new Human(LANDING_VISITOR, {
      identifier: '',
      password: '',
      displayName: LANDING_VISITOR,
    });
    world.addHuman(LANDING_VISITOR, human);
  }

  const existing = human.devices.find(d => d.type === 'playwright');
  if (existing) return existing as PlaywrightDevice;

  const [, doorway] = [...world.doorways][0] ?? [];
  const doorwayUrl = doorway?.url ?? process.env['E2E_DOORWAY_ALPHA'];
  if (!doorwayUrl) return null;

  if (world.deviceMode !== 'playwright') return null;

  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
  const browser = await world.getBrowser();
  // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
  const device = new PlaywrightDevice(`${LANDING_VISITOR}-pw`, doorwayUrl, doorwayUrl, browser);
  await device.init();
  human.addDevice(device);
  world.onCleanup(async () => device.close());
  return device;
}

// ---------------------------------------------------------------------------
// Navigation + interaction
// ---------------------------------------------------------------------------

/**
 * Visit the doorway landing page (slug "elohim-host-landing" — the front
 * door "/", the same EPR-router serving path protocol-omnibar-chrome.feature
 * pins) and wait for the native chrome element to mount.
 *
 * Example:
 *   Given I open the doorway landing page in the browser
 */
Given('I open the doorway landing page in the browser', async function (this: E2EWorld) {
  const device = await ensureNativeOmniVisitor(this);
  if (!device) return NO_PW_DEVICE_PENDING;
  await device.navigate('/');
  await device.page.locator(OMNI).waitFor({ state: 'attached', timeout: 15_000 });
  return undefined;
});

/**
 * Click the collapsed chip — this is also what triggers omni-element.js's
 * maybeLoadResilience() (the lazy /api/v1/resilience/{slug} fetch), mirroring
 * the Angular toolbar's equivalent expand-triggers-fetch behavior.
 *
 * Example:
 *   When I expand the native omni chrome
 */
When('I expand the native omni chrome', async function (this: E2EWorld) {
  const device = await ensureNativeOmniVisitor(this);
  if (!device) return NO_PW_DEVICE_PENDING;
  await device.page.locator(TOGGLE).first().click();
  await device.page.locator(RESILIENCE_BUTTON).waitFor({ state: 'visible', timeout: 5_000 });
  return undefined;
});

/**
 * Click the resilience glyph (progressive disclosure — folds down the
 * drilldown card, mirroring the Angular hypercard's click-to-open).
 *
 * Example:
 *   When I click the native omni resilience glyph
 */
When('I click the native omni resilience glyph', async function (this: E2EWorld) {
  const device = await ensureNativeOmniVisitor(this);
  if (!device) return NO_PW_DEVICE_PENDING;
  await device.page.locator(RESILIENCE_BUTTON).first().click();
  return undefined;
});

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

/**
 * Poll data-omni-resilience-loaded until it leaves the transient "loading"
 * state (or is never set at all — the omni-element never reached the fetch),
 * then assert the settled value. "unmatched" is precisely the phantom-wire-
 * contract regression state (fields read that never existed on
 * ResilienceSnapshotView, or a network failure — see b13d4d04e); "applied"
 * is the cure.
 *
 * Example:
 *   Then the native omni resilience-loaded state settles to "applied"
 */
Then(
  'the native omni resilience-loaded state settles to {string}',
  async function (this: E2EWorld, expected: string) {
    const device = await ensureNativeOmniVisitor(this);
    if (!device) return NO_PW_DEVICE_PENDING;
    await device.page.waitForFunction(
      (sel: string) => {
        const el = document.querySelector(sel);
        const state = el?.getAttribute('data-omni-resilience-loaded');
        return state === 'applied' || state === 'unmatched';
      },
      OMNI,
      { timeout: 15_000 }
    );
    const settled = await device.page.locator(OMNI).getAttribute('data-omni-resilience-loaded');
    assert.equal(
      settled,
      expected,
      `Expected data-omni-resilience-loaded to settle to "${expected}", got "${String(settled)}" ` +
        '("unmatched" is the phantom-wire-contract regression state this scenario guards against — ' +
        'see cf7679688 / b13d4d04e)'
    );
    return undefined;
  }
);

/**
 * The glyph must be the live three-plane resilience DIAL — an inline <svg>
 * plus the data-omni-dial-* state attributes — never the neutral ◉ text
 * placeholder the element ships with before a snapshot applies (or forever,
 * under the phantom-contract regression).
 *
 * Example:
 *   Then the native omni resilience glyph shows a live protection state
 */
Then(
  'the native omni resilience glyph shows a live protection state',
  async function (this: E2EWorld) {
    const device = await ensureNativeOmniVisitor(this);
    if (!device) return NO_PW_DEVICE_PENDING;
    const dialCount = await device.page.locator(DIAL_SVG).count();
    assert.ok(
      dialCount > 0,
      'Expected the resilience mark to contain the dial <svg> (never the neutral ◉ text placeholder)'
    );
    const mark = device.page.locator(RESILIENCE_GLYPH).first();
    const reach = await mark.getAttribute('data-omni-dial-reach');
    assert.ok(
      reach !== null && DIAL_REACHES.includes(reach),
      `Expected data-omni-dial-reach in {${DIAL_REACHES.join(', ')}}, got "${String(reach)}"`
    );
    const lit = await mark.getAttribute('data-omni-dial-lit');
    assert.ok(
      lit !== null && ['0', '1', '2', '3'].includes(lit),
      `Expected data-omni-dial-lit in 0..3, got "${String(lit)}"`
    );
    return undefined;
  }
);

/**
 * The glyph's title attribute must have moved on from the hard-coded
 * unavailable placeholder — the second half of the phantom-contract symptom
 * (applyResilience() never ran, so the SSR-served neutral title survived).
 *
 * Example:
 *   Then the native omni resilience title is no longer the phantom placeholder
 */
Then(
  'the native omni resilience title is no longer the phantom placeholder',
  async function (this: E2EWorld) {
    const device = await ensureNativeOmniVisitor(this);
    if (!device) return NO_PW_DEVICE_PENDING;
    const title = await device.page.locator(RESILIENCE_GLYPH).first().getAttribute('title');
    assert.notEqual(
      title,
      PHANTOM_TITLE,
      `Glyph title is still the phantom-contract placeholder ("${PHANTOM_TITLE}")`
    );
    return undefined;
  }
);

/**
 * The drilldown card (progressive disclosure, click-to-open) must become
 * visible with a non-empty headline once the glyph is clicked.
 *
 * Example:
 *   Then the native omni resilience drilldown card is visible with a headline
 */
Then(
  'the native omni resilience drilldown card is visible with a headline',
  async function (this: E2EWorld) {
    const device = await ensureNativeOmniVisitor(this);
    if (!device) return NO_PW_DEVICE_PENDING;
    const card = device.page.locator(RESILIENCE_CARD).first();
    await card.waitFor({ state: 'visible', timeout: 5_000 });
    assert.ok(
      await card.isVisible(),
      'Expected the resilience drilldown card to be visible after clicking the glyph'
    );
    const headline = await device.page.locator(RESILIENCE_HEADLINE).first().textContent();
    assert.ok(
      headline && headline.trim().length > 0,
      'Expected the drilldown card to render a non-empty headline'
    );
    return undefined;
  }
);
