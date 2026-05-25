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
 * DOM state inside a mounted pillar bundle. The a2o framework drives
 * the browser via Playwright (E2E_DEVICE_MODE=playwright).
 *
 * MVP scope:
 *   - The <elohim-epr-link> web component, the inline-resolution
 *     pipeline, and the context menu are deliverables of other tasks
 *     in the pillar EPR decomposition plan (B17–B19).
 *   - Until those land, the steps are stubbed `pending` so the feature
 *     parses cleanly. The contract is captured; implementation hooks
 *     into the page objects once the component is shipping.
 *
 * When B17–B19 land, the implementation pattern follows
 * steps/ui/epr-content.steps.ts — drive a PlaywrightDevice through an
 * EprLinkPage page object that owns selectors. A page-object stub is
 * referenced in comments rather than introduced now to avoid stranding
 * an unused file.
 */

import { Given, When, Then } from '@cucumber/cucumber';

import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Given — fixture / starting-state steps
// ---------------------------------------------------------------------------

Given(
  'the user is viewing {string} in a mounted lamad bundle',
  function (this: E2EWorld, _path: string) {
    // Will navigate device.page to `${appUrl}${_path}` and wait for the
    // lamad bundle to mount (window.__lamadMounted === true). Pending
    // until the <elohim-epr-link> web component ships (B17).
    return 'pending';
  }
);

Given(
  'the concept view contains <elohim-epr-link epr={string} display={string}>',
  function (this: E2EWorld, _eprUri: string, _display: string) {
    // Will assert `await device.page.locator('elohim-epr-link[epr="..."]').count() > 0`.
    return 'pending';
  }
);

Given('an EPR-link points to an EPR that is currently unreachable', function (this: E2EWorld) {
  // Will set up a fixture EPR with a sentinel hash that the storage
  // layer is configured to deny — exercising the offline path.
  return 'pending';
});

Given('the EPR has a previewEprRef declared', function (this: E2EWorld) {
  // Will load the EPR Head and assert previewEprRef is set.
  return 'pending';
});

Given('the user is viewing a page containing an <elohim-epr-link>', function (this: E2EWorld) {
  return 'pending';
});

// ---------------------------------------------------------------------------
// When — interaction steps
// ---------------------------------------------------------------------------

When('the chip resolves', function (this: E2EWorld) {
  // Will await an `elohim-epr-link[data-state="resolved"]` selector.
  return 'pending';
});

When('the link resolves', function (this: E2EWorld) {
  return 'pending';
});

When('the user right-clicks the link', function (this: E2EWorld) {
  // Will use `device.page.locator('elohim-epr-link').click({ button: 'right' })`.
  return 'pending';
});

// ---------------------------------------------------------------------------
// Then — outcome assertions
// ---------------------------------------------------------------------------

Then("the chip renders with the landing EPR's title and metadata", function (this: E2EWorld) {
  return 'pending';
});

Then('no browser navigation occurs', function (this: E2EWorld) {
  // Will compare device.page.url() before vs after the resolve step.
  return 'pending';
});

Then('the lamad Angular app remains mounted', function (this: E2EWorld) {
  // Will assert window.__lamadMounted is still true and the same
  // app-root instance id is present.
  return 'pending';
});

Then("the chip renders the preview EPR's content", function (this: E2EWorld) {
  return 'pending';
});

Then(/^the chip displays the offline\/unreachable marker$/, function (this: E2EWorld) {
  // Will assert an `[data-state="offline"]` attribute on the chip.
  return 'pending';
});

Then(
  'a context menu opens with Open, About this EPR, and Copy EPR link',
  function (this: E2EWorld) {
    // Will assert role=menu with three menuitem children matching
    // the expected labels.
    return 'pending';
  }
);

Then(
  /^the menu can be navigated by keyboard \(arrows, Enter, Escape\)$/,
  function (this: E2EWorld) {
    // Will press ArrowDown/ArrowUp and assert :focus moves, then
    // Escape and assert the menu closes.
    return 'pending';
  }
);
