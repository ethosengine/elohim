/**
 * Contributor Presences Step Definitions — Playwright-powered contributor visibility flows.
 *
 * These steps verify the "Contributors" section of the content viewer:
 * that learners can see who inspired or contributed to a piece of content
 * (recognition before registration — a presence is established the moment
 * a contributor's work enters the content graph).
 *
 * The section renders when `contributorPresences.length > 0` on the content artifact.
 * `<elohim-contributor-card>` is a Lit web component; card internals live in shadow DOM.
 * Per-card testids (`viewer-contributor-card-{id}`) are on the host (light DOM) element.
 *
 * Requires E2E_DEVICE_MODE=playwright. In HTTP mode, browser-only assertions return 'pending'.
 *
 * ENV CAVEAT (Sprint 1): the seeding Givens are documented intent — a direct
 * "seed content with contributor presences" API does not yet exist in the test fixture
 * layer (ContributorPresence is recognition-derived, not directly creatable). A full green
 * run requires a healthy stack pre-loaded with the expected seed data. The structural
 * contract (testids, page object, step wiring) is complete and verified via dry-run.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { ContributorsPage } from '../../src/framework/pages/contributors.page.js';
import { CONTRIBUTORS } from '../../src/framework/pages/selectors.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Return the Playwright device from the first logged-in human, or null in HTTP mode. */
function requirePlaywright(world: E2EWorld): PlaywrightDevice | null {
  if (world.deviceMode !== 'playwright') return null;
  for (const [, human] of world.humans) {
    const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
    if (device) return device;
  }
  return null;
}

// ---------------------------------------------------------------------------
// Fixture Givens
// ---------------------------------------------------------------------------

/**
 * Seed intent: content with named contributor presences.
 *
 * ENV CAVEAT: ContributorPresence is recognition-derived from economic events —
 * there is no direct fixture-layer "create contributor presence" call yet.
 * This step documents the precondition; it returns 'pending' when the seeding
 * infrastructure is unavailable so the scenario is clearly deferred, not failed.
 */
Given(
  'content {string} has been seeded with contributor presences',
  function (this: E2EWorld, _contentId: string, _table: unknown) {
    // Seeding via fixture is a Sprint-2 gap (contributor presences are recognition-derived).
    // Return pending so the scenario defers cleanly rather than failing on the Given.
    return 'pending';
  }
);

/**
 * Seed intent: content with no contributor presences (verifies absent-section behavior).
 */
Given(
  'content {string} has been seeded with no contributor presences',
  function (this: E2EWorld, _contentId: string) {
    // Same caveat as above — returns pending until fixture seeding is wired.
    return 'pending';
  }
);

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

When('the learner opens the content {string}', async function (this: E2EWorld, contentId: string) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  // Content is reachable at the universal EPR viewer route.
  await device.navigate(`/epr/${contentId}`);
  await device.page.waitForLoadState('networkidle');
});

// ---------------------------------------------------------------------------
// Contributors section visibility
// ---------------------------------------------------------------------------

Then('the contributors section is visible', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const page = new ContributorsPage(device.page);
  const visible = await page.isSectionVisible();
  assert.ok(
    visible,
    `Expected [data-testid="${CONTRIBUTORS.SECTION}"] to be visible, but it was not`
  );
});

Then('the contributors section is absent', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const page = new ContributorsPage(device.page);
  const absent = await page.isSectionAbsent();
  assert.ok(
    absent,
    `Expected [data-testid="${CONTRIBUTORS.SECTION}"] to be absent, but it was visible`
  );
});

// ---------------------------------------------------------------------------
// Card count
// ---------------------------------------------------------------------------

Then(
  'the contributors list shows {int} contributor cards',
  async function (this: E2EWorld, expectedCount: number) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const page = new ContributorsPage(device.page);
    const actual = await page.getCardCount();
    assert.strictEqual(
      actual,
      expectedCount,
      `Expected ${expectedCount} contributor cards, found ${actual}`
    );
  }
);

// ---------------------------------------------------------------------------
// Individual card assertions
// ---------------------------------------------------------------------------

Then(
  'there is a contributor card for {string}',
  async function (this: E2EWorld, displayName: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const page = new ContributorsPage(device.page);
    // Shadow-DOM-piercing assertion via accessible name (Playwright getByRole).
    const found = await page.hasContributorNamed(displayName);
    assert.ok(
      found,
      `Expected a contributor card for "${displayName}" (aria-label "Contributor: ${displayName}") but none was found`
    );
  }
);
