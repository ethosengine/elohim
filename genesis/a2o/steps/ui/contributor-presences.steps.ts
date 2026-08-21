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
 * SEEDING ASYMMETRY: a ContributorPresence is recognition-derived — it exists
 * because an economic event named someone — so there is no "create a
 * contributor presence" call to make, and the WITH-contributors Given still
 * documents intent and defers (its scenario stays @wip until the recognition
 * fixture exists). The WITHOUT-contributors Given has no such problem: on a
 * substrate this suite owns it simply authors a node nobody has been
 * recognized on, and verifies that before the UI is asked anything.
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
 * Seed a content node that nobody has been recognized on yet.
 *
 * This one IS seedable, and the asymmetry is the point: a contributor presence
 * is recognition-DERIVED (it appears because an economic event named someone),
 * so it cannot be conjured — but a node with NO recognition on it is just a
 * node, and on a substrate this suite owns (`processControl: true`, Act I) we
 * may author one. So this step really establishes its precondition instead of
 * asserting it: it creates the node if it is absent, and either way it proves
 * the node carries no contributor presences before the scenario asks the UI
 * whether the section is hidden. Without that proof, a green "section absent"
 * could equally mean "the section never renders at all".
 */
Given(
  'content {string} has been seeded with no contributor presences',
  async function (this: E2EWorld, contentId: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    let item: Record<string, unknown> | null = null;
    try {
      item = await device.client.getContent(contentId);
    } catch {
      item = null;
    }

    if (!item) {
      await device.client.createContent({
        id: contentId,
        contentType: 'article',
        title: 'Content without contributors',
        description: 'a2o fixture: a node no one has been recognized on yet',
        contentBody: 'Nobody has been recognized on this node. The viewer should say nothing.',
        contentFormat: 'markdown',
        tags: ['e2e', 'a2o-fixture', 'no-contributors'],
      });
      item = await device.client.getContent(contentId);
    }

    const presences = item['contributorPresences'];
    assert.ok(
      presences === undefined ||
        presences === null ||
        (Array.isArray(presences) && presences.length === 0),
      `content "${contentId}" was supposed to have no contributor presences, but carries ` +
        `${JSON.stringify(presences)} — the absent-section assertion below would be vacuous`
    );
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
