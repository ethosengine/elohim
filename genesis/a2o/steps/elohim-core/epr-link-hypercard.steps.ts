/**
 * EPR-link HyperCard step definitions — Task B23 of the pillar EPR
 * decomposition plan.
 *
 * Feature: features/elohim-core/epr-link-hypercard.feature
 *
 * EPR-links flip in place (HyperCard semantics): clicking does NOT
 * trigger a browser navigation; the chip resolves inline; right-click
 * opens a keyboard-navigable context menu; offline EPRs fall back to a
 * declared previewEprRef.
 *
 * All three B23 scenarios are BROWSER-VERIFIABLE — they assert against
 * DOM state inside a mounted pillar bundle. The a2o framework drives the
 * browser via Playwright (E2E_DEVICE_MODE=playwright). These steps pierce
 * the nested shadow roots (elohim-epr-link → elohim-context-menu) and
 * assert by tag + ARIA role + label (the blank-slate primitives expose no
 * data-testids by design — see selectors.ts EPR_LINK / CONTEXT_MENU).
 *
 * The implementation follows steps/ui/epr-content.steps.ts: a Playwright
 * device is required; without one (non-browser mode, or no device wired
 * yet) the steps return 'pending' so the feature parses and runs cleanly.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import {
  CONTENT_VIEWER,
  CONTEXT_MENU,
  EPR_LINK,
  MARKDOWN,
} from '../../src/framework/pages/selectors.js';
import { doorwayToAppUrl } from '../../src/framework/utils/url.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** URL captured before a resolve/right-click, for the "no navigation" assertion. */
let urlBefore: string | undefined;

/**
 * Return the first registered human's Playwright device, or null if none
 * (non-Playwright mode, or no browser human wired into this scenario yet).
 * Steps that get null return 'pending' — the scenario is skipped, not failed.
 */
function pwDevice(world: E2EWorld): PlaywrightDevice | null {
  for (const [, human] of world.humans) {
    const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
    if (device) return device;
  }
  return null;
}

/**
 * The light-DOM locator for the host element. Playwright pierces the host's
 * own shadow root for descendant `>>>` queries, but the host tag itself is
 * in light DOM.
 */
function hostLocator(device: PlaywrightDevice) {
  return device.page.locator(EPR_LINK.HOST).first();
}

// ---------------------------------------------------------------------------
// Given — fixture / starting-state steps
// ---------------------------------------------------------------------------

Given(
  'the user is viewing {string} in a mounted pillar bundle',
  async function (this: E2EWorld, path: string) {
    const device = pwDevice(this);
    if (!device) return 'pending';
    const appUrl = doorwayToAppUrl(device.client.url);
    await device.page.goto(`${appUrl}${path}`, { waitUntil: 'networkidle' });
    await hostLocator(device).waitFor({ state: 'attached', timeout: 15_000 });
    return undefined;
  }
);

Given(
  'the view contains <elohim-epr-link epr={string} display={string}>',
  async function (this: E2EWorld, eprUri: string, _display: string) {
    const device = pwDevice(this);
    if (!device) return 'pending';
    const count = await device.page.locator(`${EPR_LINK.HOST}[epr="${eprUri}"]`).count();
    assert.ok(count > 0, `Expected an <elohim-epr-link epr="${eprUri}"> on the page`);
    return undefined;
  }
);

// Sentinel EPR ref for the unreachable scenario: syntactically valid, but the
// fabricated hash resolves to nothing — the seeded DHT has no such content, so
// the injected element's resolver returns null and the host transitions to L4.
const UNREACHABLE_EPR =
  'epr:sha256-deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef';
// Title carried by the previewEprRef the unreachable EPR declares — asserted by
// "the chip renders the preview EPR's content".
const PREVIEW_TITLE = 'Manifesto (cached preview)';
// Marker so the injected host is uniquely locatable across the two scenarios.
const UNREACHABLE_MARKER_ATTR = 'data-a2o-unreachable';

Given(
  'an EPR-link points to an EPR that is currently unreachable',
  async function (this: E2EWorld) {
    const device = pwDevice(this);
    if (!device) return 'pending';
    // Land on the home surface — it imports `elohim-core/register`, so the
    // <elohim-epr-link> custom element is upgraded and available to construct.
    const appUrl = doorwayToAppUrl(device.client.url);
    await device.page.goto(`${appUrl}/`, { waitUntil: 'networkidle' });
    await hostLocator(device).waitFor({ state: 'attached', timeout: 15_000 });

    // Inject a SECOND <elohim-epr-link> whose EPR is a fabricated (syntactically
    // valid) hash. We give it NO resolver so its own resolve() parks at L2 and
    // never clobbers our forced state; we then drive it to L4/unreachable via
    // the element's public setResolution() test seam. This exercises the REAL
    // L4 render path (the <elohim-mention-base> fallback), not a mock.
    await device.page.evaluate(
      (args: { epr: string; marker: string }) => {
        const existing = document.querySelector(`[${args.marker}]`);
        if (existing) existing.remove();
        const el = document.createElement('elohim-epr-link');
        el.setAttribute('epr', args.epr);
        el.setAttribute('display', 'chip');
        el.setAttribute(args.marker, '');
        document.body.appendChild(el);
      },
      { epr: UNREACHABLE_EPR, marker: UNREACHABLE_MARKER_ATTR }
    );
    return undefined;
  }
);

Given('the EPR has a previewEprRef declared', async function (this: E2EWorld) {
  const device = pwDevice(this);
  if (!device) return 'pending';
  // Drive the injected host to L4 with a preview declared. setResolution() is
  // the element's documented test seam; preview.title is what the L4 render
  // surfaces as the <elohim-mention-base> label (the "previewEprRef content").
  // We wait for Lit's updateComplete so the fallback DOM is materialised before
  // the When/Then steps query it.
  await device.page.evaluate(
    async (args: { marker: string; previewTitle: string }) => {
      interface ResolvableLink extends Element {
        setResolution?: (
          level: number,
          resolution: { unreachable?: boolean; preview?: { title?: string } }
        ) => void;
        updateComplete?: Promise<unknown>;
      }
      const el: ResolvableLink | null = document.querySelector(`[${args.marker}]`);
      if (!el?.setResolution) return;
      el.setResolution(4, { unreachable: true, preview: { title: args.previewTitle } });
      if (el.updateComplete) await el.updateComplete;
    },
    { marker: UNREACHABLE_MARKER_ATTR, previewTitle: PREVIEW_TITLE }
  );
  return undefined;
});

Given(
  'the user is viewing a page containing an <elohim-epr-link>',
  async function (this: E2EWorld) {
    const device = pwDevice(this);
    if (!device) return 'pending';
    await hostLocator(device).waitFor({ state: 'attached', timeout: 15_000 });
    return undefined;
  }
);

// ---------------------------------------------------------------------------
// When — interaction steps
// ---------------------------------------------------------------------------

When('the chip resolves', async function (this: E2EWorld) {
  const device = pwDevice(this);
  if (!device) return 'pending';
  // Resolved chip renders the anchor button (L2/L3) inside the host shadow root.
  await device.page
    .locator(`${EPR_LINK.HOST} ${EPR_LINK.ANCHOR}`)
    .first()
    .waitFor({ state: 'visible', timeout: 15_000 });
  // Record the URL so a later "no navigation" assertion can compare.
  urlBefore = device.page.url();
  return undefined;
});

When('the link resolves', async function (this: E2EWorld) {
  const device = pwDevice(this);
  if (!device) return 'pending';
  // For the unreachable scenario the injected host carries the marker; for the
  // generic case fall back to the first host on the page.
  const injected = device.page.locator(`${EPR_LINK.HOST}[${UNREACHABLE_MARKER_ATTR}]`);
  const target = (await injected.count()) > 0 ? injected.first() : hostLocator(device);
  await target.waitFor({ state: 'attached', timeout: 15_000 });
  urlBefore = device.page.url();
  return undefined;
});

When('the user follows the card', async function (this: E2EWorld) {
  const device = pwDevice(this);
  if (!device) return 'pending';
  urlBefore = device.page.url();
  // Left-click the resolved anchor inside the host's shadow root: the Lit
  // element emits `navigate`, the Angular wrapper resolves the EPR and routes
  // to its universal /epr/{id} address (same bundle → router navigation).
  await device.page.locator(`${EPR_LINK.HOST} ${EPR_LINK.ANCHOR}`).first().click();
  return undefined;
});

When('the user right-clicks the link', async function (this: E2EWorld) {
  const device = pwDevice(this);
  if (!device) return 'pending';
  urlBefore = device.page.url();
  await device.page
    .locator(`${EPR_LINK.HOST} ${EPR_LINK.ANCHOR}`)
    .first()
    .click({ button: 'right' });
  await device.page
    .locator(`${EPR_LINK.HOST} ${EPR_LINK.CONTEXT_MENU}[open]`)
    .first()
    .waitFor({ state: 'visible', timeout: 5_000 });
  return undefined;
});

// ---------------------------------------------------------------------------
// Then — outcome assertions
// ---------------------------------------------------------------------------

Then(
  "the chip renders with the resolved EPR's title and metadata",
  async function (this: E2EWorld) {
    const device = pwDevice(this);
    if (!device) return 'pending';
    const text =
      (await device.page.locator(`${EPR_LINK.HOST} ${EPR_LINK.ANCHOR}`).first().textContent()) ??
      '';
    assert.ok(text.trim().length > 0, 'Expected the resolved chip to render a title');
    return undefined;
  }
);

Then('no browser navigation occurs', function (this: E2EWorld) {
  const device = pwDevice(this);
  if (!device) return 'pending';
  assert.equal(device.page.url(), urlBefore, 'Expected no browser navigation (HyperCard flip)');
  return undefined;
});

Then('the universal address {string} is showing', async function (this: E2EWorld, path: string) {
  const device = pwDevice(this);
  if (!device) return 'pending';
  await device.page.waitForURL(url => url.pathname === path, { timeout: 15_000 });
  assert.equal(new URL(device.page.url()).pathname, path, `Expected to land on ${path}`);
  return undefined;
});

Then("the resolved EPR's body is rendered, not a loading state", async function (this: E2EWorld) {
  const device = pwDevice(this);
  if (!device) return 'pending';
  const viewer = device.page.locator(`[data-testid="${CONTENT_VIEWER.ROOT}"]`).first();
  await viewer.waitFor({ state: 'attached', timeout: 15_000 });
  // The loaded node's body (.content-body on the standalone /epr/{id} viewer).
  // This is the assertion that catches the change-detection freeze: the node
  // arrives, `isLoading` flips, and nothing re-renders — the body never mounts.
  await device.page
    .locator(MARKDOWN.CONTENT)
    .first()
    .waitFor({ state: 'visible', timeout: 15_000 });
  // The viewer's own top-level loading gate must be gone (tab-level loading
  // states inside the rendered content are a different, narrower gate).
  const topLevelSpinners = await viewer.locator('> .loading-state').count();
  assert.equal(topLevelSpinners, 0, 'Expected the content viewer to leave its loading state');
  const text = (await viewer.textContent()) ?? '';
  assert.ok(
    !text.includes('Loading content...'),
    'Expected the loading placeholder to be replaced by content'
  );
  return undefined;
});

Then('the Angular app remains mounted', async function (this: E2EWorld) {
  const device = pwDevice(this);
  if (!device) return 'pending';
  const mounted = await device.page.locator('app-root').count();
  assert.ok(mounted > 0, 'Expected the app-root to remain mounted');
  return undefined;
});

Then("the chip renders the preview EPR's content", async function (this: E2EWorld) {
  const device = pwDevice(this);
  if (!device) return 'pending';
  // At L4 the host renders <elohim-mention-base> (the `fallback` part) labelled
  // with preview.title. Pierce the injected host's shadow root and assert the
  // preview title text surfaces.
  const fallback = device.page
    .locator(`${EPR_LINK.HOST}[${UNREACHABLE_MARKER_ATTR}] ${EPR_LINK.FALLBACK}`)
    .first();
  await fallback.waitFor({ state: 'attached', timeout: 5_000 });
  // The preview title surfaces as the mention-base's `label` attribute and is
  // rendered inside ITS shadow root — light-DOM textContent is empty by design,
  // so assert the attribute the L4 render binds.
  const label = (await fallback.getAttribute('label')) ?? '';
  assert.ok(
    label.includes(PREVIEW_TITLE),
    `Expected the L4 fallback to carry the preview title "${PREVIEW_TITLE}". Got label: "${label.trim()}"`
  );
  return undefined;
});

Then(/^the chip displays the offline\/unreachable marker$/, async function (this: E2EWorld) {
  const device = pwDevice(this);
  if (!device) return 'pending';
  // The L4 fallback IS the unreachable marker: the host swaps its interactive
  // button.anchor for the generic <elohim-mention-base> chip (the "pillar
  // element not loaded / unreachable" fallback). Assert (a) the fallback is
  // present and (b) the live anchor is NOT — i.e. the host degraded, it did not
  // resolve. The fallback's `fallback-mark` (role=presentation) is the visible
  // marker dot.
  const host = `${EPR_LINK.HOST}[${UNREACHABLE_MARKER_ATTR}]`;
  const fallbackMark = device.page.locator(`${host} ${EPR_LINK.FALLBACK_MARK}`).first();
  await fallbackMark.waitFor({ state: 'attached', timeout: 5_000 });
  const anchorCount = await device.page.locator(`${host} ${EPR_LINK.ANCHOR}`).count();
  assert.equal(anchorCount, 0, 'Expected no live anchor at L4 — the host should show the fallback');
  return undefined;
});

Then(
  'a context menu opens including Open, About this EPR, and Copy EPR link',
  async function (this: E2EWorld) {
    const device = pwDevice(this);
    if (!device) return 'pending';
    // Pierce host shadow → context-menu shadow → menuitems.
    const items = device.page.locator(
      `${EPR_LINK.HOST} ${EPR_LINK.CONTEXT_MENU}[open] ${CONTEXT_MENU.ITEM_ROLE}`
    );
    await items.first().waitFor({ state: 'visible', timeout: 5_000 });
    const labels = (await items.allTextContents()).map(t => t.trim());
    // Presence assertion (NOT exhaustive — the full Epic E set may be present).
    for (const expected of CONTEXT_MENU.MVP_LABELS) {
      assert.ok(
        labels.includes(expected),
        `Expected context menu to include "${expected}". Got: ${labels.join(', ')}`
      );
    }
    return undefined;
  }
);

Then(
  /^the menu can be navigated by keyboard \(arrows, Enter, Escape\)$/,
  async function (this: E2EWorld) {
    const device = pwDevice(this);
    if (!device) return 'pending';
    const menu = device.page.locator(`${EPR_LINK.HOST} ${EPR_LINK.CONTEXT_MENU}[open]`).first();
    await menu.waitFor({ state: 'visible', timeout: 5_000 });
    // ArrowDown/ArrowUp move focus across menuitems; Escape closes the menu.
    // The stub PWPage exposes no `keyboard`, so dispatch keydowns through the
    // menu's [role=menu] (which owns the keydown handler) via evaluate. The
    // events must cross the host → context-menu shadow boundary.
    await device.page.evaluate(() => {
      const host = document.querySelector('elohim-epr-link');
      const menuEl = host?.shadowRoot
        ?.querySelector('elohim-context-menu')
        ?.shadowRoot?.querySelector('[role="menu"]');
      if (!menuEl) return;
      for (const key of ['ArrowDown', 'ArrowUp', 'Escape']) {
        menuEl.dispatchEvent(
          new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true })
        );
      }
    });
    await menu.waitFor({ state: 'hidden', timeout: 5_000 });
    return undefined;
  }
);
