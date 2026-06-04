/* eslint-disable @typescript-eslint/require-await -- Cucumber step handlers are async by convention; some sync assertions and pending-guard returns don't await. */

/**
 * EPR relationship navigation step definitions.
 *
 * Drives genesis/a2o/features/lamad/epr-link-navigation.feature: a learner
 * opens a concept and sees its typed EPR-Head relationships rendered as
 * navigable cards with trust signals (reach glyph + stewardship-resilience
 * glyph + a subtle distinct-peers count). The relationship cards are the
 * canonical surface for the resilience badge that Lane B evolves, so these
 * steps are the a2o guard for that badge.
 *
 * Browser-tier: every step degrades to 'pending' when no Playwright device is
 * registered (API / non-browser cucumber runs), mirroring the resilience and
 * topology step files. The feature carries no named persona — "the learner" —
 * so we resolve the first PlaywrightDevice across all registered humans.
 *
 * DOM contract (Angular light-DOM, data-testid):
 *   epr-relationships-panel  — the panel wrapping all groups
 *   epr-rel-group            — one per relationship type; data-type attr carries
 *                              PREREQUISITE / TEACHES / REFERENCES; the <h3>
 *                              heading reads "Prerequisites" / "This teaches" /
 *                              "References"
 *   epr-relationship-card    — a single navigable card (an <a> to /resource/:id)
 *   epr-rel-card-reach       — reach badge
 *   epr-rel-card-resilience  — stewardship-resilience glyph
 *   epr-rel-card-peers       — distinct-peers count (present only when > 0)
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { RELATIONSHIP_CARD, RELATIONSHIP_PANEL } from '../../src/framework/pages/selectors.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const PENDING = 'pending';

/** First PlaywrightDevice across all registered humans, or null in API mode. */
function findPwDevice(world: E2EWorld): PlaywrightDevice | null {
  for (const [, human] of world.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        return device;
      }
    }
  }
  return null;
}

/**
 * Locate the per-type relationship group section by its heading text.
 * The panel renders each group as `[data-testid="epr-rel-group"]` with an
 * `<h3>` heading; we match on heading text (case-insensitive) rather than the
 * data-type attribute so the assertion reads the way the learner sees it.
 *
 * Playwright's `:has-text()` is case-insensitive and substring-matching, so the
 * literal heading ("Prerequisites", "This teaches", "References") selects its
 * own group via a single CSS-engine selector — the wrapped PWLocator exposes
 * only `locator(string)`, not the `.filter({ has })` builder.
 */
async function groupSectionVisible(
  device: PlaywrightDevice,
  headingText: string
): Promise<boolean> {
  const group = device.page
    .locator(`[data-testid="${RELATIONSHIP_PANEL.GROUP}"]:has(h3:has-text("${headingText}"))`)
    .first();
  return group
    .waitFor({ state: 'visible', timeout: 15_000 })
    .then(() => true)
    .catch(() => false);
}

// ---------------------------------------------------------------------------
// Background / Given
// ---------------------------------------------------------------------------

/**
 * "Given a learner has signed in" — fixture-state annotation. The doorway /
 * auth steps own the actual session; in browser mode the seeded fixture human
 * is already authenticated by the device bootstrap. We only assert a device
 * exists (or skip in API mode).
 *
 * Example: Given a learner has signed in
 */
Given('a learner has signed in', async function (this: E2EWorld) {
  if (!findPwDevice(this)) {
    return PENDING;
  }
});

/**
 * "Given the concept {string} has been seeded with an EPR Head" — informational
 * precondition. The EPR Head (and its typed relationships) is owned by the
 * genesis seeder; the rows in the data table document the expected shape. The
 * downstream assertions verify what actually rendered, so this step does not
 * re-seed.
 *
 * Example:
 *   And the concept "systems-thinking" has been seeded with an EPR Head
 *     | type         | target          |
 *     | PREREQUISITE | feedback-loops  |
 */
Given(
  'the concept {string} has been seeded with an EPR Head',
  async function (this: E2EWorld, _conceptId: string) {
    // Informational — seeded fixture state. Browser assertions below verify the
    // rendered relationship cards; no setup action here.
  }
);

/**
 * "Given the learner is viewing the {string} concept" — navigate to the concept
 * resource page so the click scenario starts from the right surface.
 *
 * Example: Given the learner is viewing the "systems-thinking" concept
 */
Given(
  'the learner is viewing the {string} concept',
  async function (this: E2EWorld, conceptId: string) {
    const device = findPwDevice(this);
    if (!device) {
      return PENDING;
    }
    await device.navigate(`/resource/${conceptId}`);
  }
);

// ---------------------------------------------------------------------------
// When
// ---------------------------------------------------------------------------

/**
 * "When the learner opens the {string} concept" — navigate to the concept's
 * cross-pillar resource viewer (route /resource/:resourceId).
 *
 * Example: When the learner opens the "systems-thinking" concept
 */
When('the learner opens the {string} concept', async function (this: E2EWorld, conceptId: string) {
  const device = findPwDevice(this);
  if (!device) {
    return PENDING;
  }
  await device.navigate(`/resource/${conceptId}`);
});

/**
 * "When the learner clicks the {string} prerequisite card" — click the card
 * linking to the named target inside the Prerequisites group.
 *
 * Example: When the learner clicks the "feedback-loops" prerequisite card
 */
When(
  'the learner clicks the {string} prerequisite card',
  async function (this: E2EWorld, target: string) {
    const device = findPwDevice(this);
    if (!device) {
      return PENDING;
    }
    // The card is an <a> whose href routes to /resource/{target}. Click by href
    // so we hit the specific prerequisite rather than the first card.
    const card = device.page
      .locator(`[data-testid="${RELATIONSHIP_CARD.CARD}"][href*="${target}"]`)
      .first();
    await card.waitFor({ state: 'visible', timeout: 15_000 });
    await card.click();
    await device.page.waitForLoadState('networkidle');
  }
);

// ---------------------------------------------------------------------------
// Then
// ---------------------------------------------------------------------------

/**
 * "Then a {string} section is visible" — assert a relationship group section
 * with the given heading rendered (Prerequisites / This teaches / References).
 *
 * Example: Then a "Prerequisites" section is visible
 */
Then('a {string} section is visible', async function (this: E2EWorld, heading: string) {
  const device = findPwDevice(this);
  if (!device) {
    return PENDING;
  }
  const visible = await groupSectionVisible(device, heading);
  assert.ok(visible, `Expected a relationship group section headed "${heading}" to be visible`);
});

/**
 * "Then it contains a card linking to {string}" — assert a relationship card
 * inside the panel links to the named target concept.
 *
 * Example: And it contains a card linking to "feedback-loops"
 */
Then('it contains a card linking to {string}', async function (this: E2EWorld, target: string) {
  const device = findPwDevice(this);
  if (!device) {
    return PENDING;
  }
  const card = device.page
    .locator(`[data-testid="${RELATIONSHIP_CARD.CARD}"][href*="${target}"]`)
    .first();
  await card.waitFor({ state: 'visible', timeout: 15_000 });
  const href = (await card.getAttribute('href')) ?? '';
  assert.ok(href.includes(target), `Expected a card linking to "${target}"; got href "${href}"`);
});

/**
 * "Then the card shows a reach badge and a resilience badge" — assert the trust
 * signals render on the first relationship card. The distinct-peers badge is
 * household-honest: present only when the wire reports distinctPeers > 0, so we
 * assert it WHEN present rather than requiring a multi-peer count.
 *
 * Example: And the card shows a reach badge and a resilience badge
 */
Then('the card shows a reach badge and a resilience badge', async function (this: E2EWorld) {
  const device = findPwDevice(this);
  if (!device) {
    return PENDING;
  }
  const reach = device.page.locator(`[data-testid="${RELATIONSHIP_CARD.REACH}"]`).first();
  const resilience = device.page.locator(`[data-testid="${RELATIONSHIP_CARD.RESILIENCE}"]`).first();
  await reach.waitFor({ state: 'visible', timeout: 15_000 });
  await resilience.waitFor({ state: 'visible', timeout: 15_000 });

  // Distinct-peers badge: assert structure only when it rendered. At single-
  // household scale distinctPeers is often 0 and the badge is absent — that is
  // a valid, honest state, NOT a failure.
  const peers = device.page.locator(`[data-testid="${RELATIONSHIP_CARD.PEERS}"]`).first();
  if ((await peers.count()) > 0) {
    const text = ((await peers.textContent()) ?? '').trim();
    assert.match(text, /\d/, `Distinct-peers badge rendered but carried no count; got "${text}"`);
  }
});

/**
 * "Then the {string} concept is displayed" — after clicking a card, assert the
 * target concept's resource viewer rendered.
 *
 * Example: Then the "feedback-loops" concept is displayed
 */
Then('the {string} concept is displayed', async function (this: E2EWorld, conceptId: string) {
  const device = findPwDevice(this);
  if (!device) {
    return PENDING;
  }
  const url = device.page.url();
  assert.ok(
    url.includes(`/resource/${conceptId}`),
    `Expected the "${conceptId}" concept to be displayed; URL is "${url}"`
  );
});

/**
 * "Then the URL reflects the {string} concept" — assert the browser URL routed
 * to the target concept's resource path.
 *
 * Example: And the URL reflects the "feedback-loops" concept
 */
Then('the URL reflects the {string} concept', async function (this: E2EWorld, conceptId: string) {
  const device = findPwDevice(this);
  if (!device) {
    return PENDING;
  }
  const url = device.page.url();
  assert.ok(
    url.includes(`/resource/${conceptId}`),
    `Expected URL to reflect the "${conceptId}" concept; got "${url}"`
  );
});
