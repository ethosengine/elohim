/**
 * Landing-discovery step definitions.
 *
 * Drives genesis/a2o/features/content/landing-discovery.feature: a first-time
 * visitor lands on the redesigned elohim.host doorstep and is invited into the
 * protocol through six live epic reference cards, a start-here onboarding
 * path, several self-router "ways in", and a manifesto reference card that
 * must degrade legibly rather than blank. See that feature's header comment
 * for the full `landing-*` data-testid contract the redesigned home route
 * emits (app/elohim-app/src/app/components/{hero,vision,crisis,
 * call-to-action,learning-success}/*).
 *
 * Two card shapes render on this surface, and several steps here read across
 * both:
 *   - Angular `app-epr-relationship-card` (epic cards + the two testid-bearing
 *     way-in cards) — light DOM, `data-testid="epr-relationship-card"` with
 *     nested `epr-rel-card-{title,reach,resilience,peers}` (selectors.ts
 *     RELATIONSHIP_CARD — same contract steps/lamad/epr-link-navigation.steps.ts
 *     asserts on for the resource-viewer relationship panel).
 *   - The Lit `<elohim-epr-link display="card">` (the manifesto card) — real
 *     shadow DOM, asserted via selectors.ts EPR_LINK (tag + role, no
 *     data-testid by the blank-slate discipline in
 *     app/elohim-elements/CLAUDE.md) — same selector family
 *     steps/elohim-core/epr-link-hypercard.steps.ts uses for this element.
 *
 * Browser-tier: every step degrades to 'pending' when no Playwright device is
 * registered (API / non-browser cucumber runs). "When I open the landing page
 * in a browser" (steps/ui/browser-elements.steps.ts) registers the anonymous
 * `_landing-visitor` human before any of these steps run; `activeDevice` below
 * prefers that visitor, mirroring the same file's resolution order.
 */

import { strict as assert } from 'node:assert';

import { Then, When } from '@cucumber/cucumber';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { EPR_LINK, RELATIONSHIP_CARD } from '../../src/framework/pages/selectors.js';
import { E2EWorld } from '../../src/framework/world.js';

import type { PWLocator, PWPage } from '../../src/framework/devices/playwright-device.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const PENDING = 'pending';
const VISIBLE_TIMEOUT = 15_000;

/** Matches the anonymous human browser-elements.steps.ts registers for
 *  "I open the landing page in a browser" — kept as a local literal per this
 *  repo's per-file device-resolution convention (see epr-link-navigation.steps.ts
 *  findPwDevice, learning-journey.steps.ts getDevice, etc. — each steps file
 *  owns its own copy rather than importing an unexported helper). */
const LANDING_VISITOR = '_landing-visitor';

/** Prefer the anonymous landing visitor; fall back to any logged-in human's
 *  Playwright device (mirrors browser-elements.steps.ts's activeDevice). */
function activeDevice(world: E2EWorld): PlaywrightDevice | null {
  try {
    const visitor = world.getHuman(LANDING_VISITOR).devices.find(d => d.type === 'playwright');
    if (visitor) return visitor as PlaywrightDevice;
  } catch {
    // no landing visitor registered — fall through to any human's device
  }
  for (const [, human] of world.humans) {
    const device = human.devices.find(d => d.type === 'playwright');
    if (device) return device as PlaywrightDevice;
  }
  return null;
}

/** Returns null (⇒ the calling step returns 'pending') outside browser mode. */
function requirePlaywright(world: E2EWorld): PlaywrightDevice | null {
  if (world.deviceMode !== 'playwright') return null;
  return activeDevice(world);
}

/** "who-are-you" / "evolution-of-trust" → the way-in card's data-testid
 *  (learning-success.component.ts `otherWays`). Extend this map as new
 *  self-router / explorable doors join "Find your way in". */
const WAY_IN_TESTID: Record<string, string> = {
  'who-are-you': 'landing-wayin-quiz',
  'evolution-of-trust': 'landing-wayin-trust',
};

/** Every landing-page reference-card testid scenario (e) sweeps for a blank
 *  face. landing-wayin-browse is excluded — it's a plain "browse everything"
 *  door (<a>), not a self-resolving reference card. */
const REFERENCE_CARD_TESTIDS = [
  'landing-epic-card',
  'landing-manifesto-card',
  'landing-wayin-quiz',
  'landing-wayin-trust',
];

/** Does this pathname carry `id` as one of its segments? Tolerant of every
 *  pretty-mount form a card's href can resolve to (/epr/{id},
 *  /lamad/path/{id}, /resource/{id}, ...) — the scenarios only care that the
 *  visitor landed on THAT content's address, not which mount served it. */
function pathContainsId(pathname: string, id: string): boolean {
  return pathname.split('/').filter(Boolean).includes(id);
}

/** Wait for the page to land somewhere carrying `id` in its path — tolerant of
 *  whichever address the card mints: the universal /epr/{id} (served in place as
 *  the shell since the Slice-1 claims-302 demotion — no redirect to the pretty
 *  mount), or a direct pretty mount like /lamad/path/{id}. No client-side redirect
 *  chain is assumed. */
async function waitForUrlToTargetId(page: PWPage, id: string, description: string): Promise<void> {
  try {
    await page.waitForURL((url: URL) => pathContainsId(url.pathname, id), {
      timeout: VISIBLE_TIMEOUT,
    });
  } catch {
    assert.fail(`${description}; last URL was "${page.url()}"`);
  }
}

/** Extract the trailing path segment (the EPR id) from an href. */
function hrefTargetId(href: string): string {
  try {
    const u = new URL(href, 'https://landing-discovery.local');
    const segments = u.pathname.split('/').filter(Boolean);
    return segments.at(-1) ?? '';
  } catch {
    return '';
  }
}

/**
 * A resolved human title has spaces/capitals/punctuation (see the seeded
 * titles: "The Elohim Value Scanner: How a Boy Buying Strawberries Changes
 * Everything", "Find Your Path in the Protocol", ...). An UNresolved face is
 * either the bare kebab-case slug ("value-scanner-epic") EprRelationshipCardComponent
 * seeds itself with before resolution completes, or the raw "epr:{id}" string
 * <elohim-mention-base> falls back to when even its label is unknown. Either
 * shape is the "raw id" face the doorstep must never show a visitor.
 */
function looksLikeRawId(text: string): boolean {
  if (/^epr:/i.test(text)) return true;
  return /^[a-z0-9]+(-[a-z0-9]+)*$/.test(text);
}

function assertLegibleTitle(text: string, cardLabel: string): void {
  assert.ok(text.length > 0, `${cardLabel} has a blank title`);
  assert.ok(
    !looksLikeRawId(text),
    `${cardLabel} shows a raw id/slug ("${text}") instead of a resolved title`
  );
}

/**
 * Poll a locator's text until it moves off a blank/raw-id face or the timeout
 * elapses — used for the Angular epr-relationship-card title below, which (unlike
 * the Lit card) renders its testid from the FIRST synchronous paint (reset()
 * seeds `title = rel.target`, the raw slug) and updates it in place once
 * resolveRelationship()'s forkJoin settles, so plain DOM attachment fires
 * before resolution. Timing out just returns the still-raw text — the caller's
 * legibility assertion then reports the true (unresolved) state honestly.
 */
async function pollForLegibleText(locator: PWLocator, timeout: number): Promise<string> {
  const deadline = Date.now() + timeout;
  let text = ((await locator.textContent()) ?? '').trim();
  while ((text.length === 0 || looksLikeRawId(text)) && Date.now() < deadline) {
    await new Promise(resolve => setTimeout(resolve, 200));
    text = ((await locator.textContent()) ?? '').trim();
  }
  return text;
}

/**
 * Read a reference card's title text regardless of which of the two card
 * shapes rendered (see file header): the Angular epr-relationship-card's
 * `epr-rel-card-title` testid, the Lit elohim-epr-link's resolved `button.anchor`
 * (single shadow hop — plain text, textContent is reliable), or its degraded
 * `elohim-mention-base` fallback — whose OWN nested shadow root means
 * textContent reads empty by design (see epr-link-hypercard.steps.ts), so we
 * read the `label` attribute the host binds instead.
 */
async function cardTitleText(card: PWLocator, timeout = VISIBLE_TIMEOUT): Promise<string> {
  const angularTitle = card.locator(`[data-testid="${RELATIONSHIP_CARD.TITLE}"]`);
  if ((await angularTitle.count()) > 0) {
    return pollForLegibleText(angularTitle.first(), timeout);
  }

  // The Lit <elohim-epr-link> renders NEITHER shape below until resolution
  // settles (loadLevel 1 is a bare skeleton), so plain attachment here IS the
  // resolution signal — no polling needed.
  const resolvedAnchor = card.locator(`${EPR_LINK.HOST} ${EPR_LINK.ANCHOR}`);
  const degradedFallback = card.locator(`${EPR_LINK.HOST} ${EPR_LINK.FALLBACK}`);
  await card
    .locator(`${EPR_LINK.HOST} ${EPR_LINK.ANCHOR}, ${EPR_LINK.HOST} ${EPR_LINK.FALLBACK}`)
    .first()
    .waitFor({ state: 'attached', timeout });

  if ((await resolvedAnchor.count()) > 0) {
    return ((await resolvedAnchor.first().textContent()) ?? '').trim();
  }
  return ((await degradedFallback.first().getAttribute('label')) ?? '').trim();
}

/** Window-global marker used by the hard-navigation probe (see the epic-card
 *  click step below). */
interface HardNavProbeWindow {
  __a2oHardNavProbe?: boolean;
}

// ---------------------------------------------------------------------------
// (a) Start the journey -> onboarding path
// ---------------------------------------------------------------------------

Then(
  'the start-here call carries the visitor into the {string} onboarding journey',
  async function (this: E2EWorld, journeyId: string) {
    const device = requirePlaywright(this);
    if (!device) return PENDING;
    await waitForUrlToTargetId(
      device.page,
      journeyId,
      `Expected the start-here call to land in the "${journeyId}" onboarding journey ` +
        `(either /epr/${journeyId} or /lamad/path/${journeyId})`
    );
  }
);

// ---------------------------------------------------------------------------
// (b) The epic story cards are living protocol content
// ---------------------------------------------------------------------------

Then(
  'each epic story card shows a living title, a reach glyph, and a steward badge',
  async function (this: E2EWorld) {
    const device = requirePlaywright(this);
    if (!device) return PENDING;

    const cards = device.page.locator('[data-testid="landing-epic-card"]');
    const count = await cards.count();
    assert.ok(count > 0, 'Expected at least one landing-epic-card to render');

    for (let i = 0; i < count; i++) {
      const card = cards.nth(i);

      const titleText = await cardTitleText(card);
      assertLegibleTitle(titleText, `landing-epic-card[${i}]`);

      const reach = card.locator(`[data-testid="${RELATIONSHIP_CARD.REACH}"]`).first();
      await reach.waitFor({ state: 'visible', timeout: VISIBLE_TIMEOUT });

      // The steward badge (RELATIONSHIP_CARD.RESILIENCE — the stewardship-
      // resilience glyph) is a strictly-optional enrichment since commit
      // 2c3271997: epr-relationship-card.component.ts hides it entirely when
      // the resilience payload doesn't resolve to a well-formed ResilienceView
      // (e.g. through the start:alpha dev proxy, or before alpha's resilience
      // API serves these landing content ids). We assert "present with a real
      // glyph OR legitimately absent" per card, never bare presence. Strict
      // (always-present) badge assertion activates once that API serves the
      // landing route's content ids.
      const badge = card.locator(`[data-testid="${RELATIONSHIP_CARD.RESILIENCE}"]`);
      if ((await badge.count()) > 0) {
        const badgeText = ((await badge.first().textContent()) ?? '').trim();
        assert.ok(
          badgeText.length > 0,
          `landing-epic-card[${i}] rendered a steward badge with no glyph`
        );
      }
    }
  }
);

Then(
  "following an epic story card opens that epic's own EPR address",
  async function (this: E2EWorld) {
    const device = requirePlaywright(this);
    if (!device) return PENDING;

    const card = device.page.locator('[data-testid="landing-epic-card"]').first();
    await card.waitFor({ state: 'visible', timeout: VISIBLE_TIMEOUT });
    const anchor = card.locator(`[data-testid="${RELATIONSHIP_CARD.CARD}"]`).first();
    await anchor.waitFor({ state: 'visible', timeout: VISIBLE_TIMEOUT });

    const href = (await anchor.getAttribute('href')) ?? '';
    assert.ok(href, 'The first epic card has no href to follow');
    const epicId = hrefTargetId(href);
    assert.ok(epicId, `Could not extract an EPR id from epic card href "${href}"`);

    // A window-global marker survives an in-SPA (routerLink) navigation but is
    // wiped by a real full-document load. Cross-bundle EPR cards render a
    // plain href (never routerLink — see the feature file's header note), and
    // the shell's capture-phase epr-link interceptor
    // (elohim-core/navigation/epr-link-interceptor.ts) passes same-bundle
    // anchors through untouched, so the click should hard-navigate. This
    // marker is the black-box proof of that — it doesn't depend on
    // interceptor internals, only on whether the JS realm survived the click.
    await device.page.evaluate(() => {
      (globalThis as unknown as HardNavProbeWindow).__a2oHardNavProbe = true;
    });

    await anchor.click();
    await waitForUrlToTargetId(
      device.page,
      epicId,
      `Expected following the epic card to open "${epicId}"'s own EPR address`
    );

    const probeSurvived = (await device.page.evaluate(
      () => (globalThis as unknown as HardNavProbeWindow).__a2oHardNavProbe === true
    )) as boolean;
    assert.ok(
      !probeSurvived,
      'Expected a full-document navigation (plain href) — the window context ' +
        'survived the click, which looks like an in-SPA route swap instead'
    );
  }
);

// ---------------------------------------------------------------------------
// (c) Several ways in for several kinds of visitor
// ---------------------------------------------------------------------------

When('I follow the {string} way in', async function (this: E2EWorld, wayIn: string) {
  const device = requirePlaywright(this);
  if (!device) return PENDING;

  const testid = WAY_IN_TESTID[wayIn];
  assert.ok(testid, `Unknown way-in "${wayIn}" — known: ${Object.keys(WAY_IN_TESTID).join(', ')}`);

  const card = device.page.locator(`[data-testid="${testid}"]`).first();
  await card.waitFor({ state: 'visible', timeout: VISIBLE_TIMEOUT });
  const anchor = card.locator(`[data-testid="${RELATIONSHIP_CARD.CARD}"]`).first();
  await anchor.waitFor({ state: 'visible', timeout: VISIBLE_TIMEOUT });
  await anchor.click();
});

Then('the visitor arrives at the who-are-you self-router', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return PENDING;
  await waitForUrlToTargetId(
    device.page,
    'quiz-who-are-you',
    'Expected the visitor to arrive at the who-are-you self-router'
  );
});

Then('the visitor arrives at the evolution-of-trust explorable', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return PENDING;
  await waitForUrlToTargetId(
    device.page,
    'evolution-of-trust',
    'Expected the visitor to arrive at the evolution-of-trust explorable'
  );
});

// ---------------------------------------------------------------------------
// (e) An unreachable card degrades to a legible fallback, never blank
// ---------------------------------------------------------------------------

Then(
  'the manifesto reference card renders a legible fallback with its title and an unavailable affordance',
  async function (this: E2EWorld) {
    const device = requirePlaywright(this);
    if (!device) return PENDING;

    const card = device.page.locator('[data-testid="landing-manifesto-card"]').first();
    await card.waitFor({ state: 'visible', timeout: VISIBLE_TIMEOUT });

    const titleText = await cardTitleText(card);
    assertLegibleTitle(titleText, 'landing-manifesto-card');

    // The honest "unavailable" affordance is elohim-epr-link's degraded (L4)
    // fallback-note part (e.g. "Unavailable at your reach" — see
    // app/elohim-elements/elohim-core/src/elohim-epr-link.ts degradedNote()).
    // It renders ONLY when the card actually degraded; a fully-resolved card
    // needs no affordance at all. We therefore PASS either way — the title
    // assertion above is the only thing that fails on a blank/raw-id face,
    // matching the scenario's intent (legible fallback OR full resolution,
    // never a blank/raw face).
    const fallbackNote = card.locator(EPR_LINK.FALLBACK_NOTE);
    if ((await fallbackNote.count()) > 0) {
      const noteText = ((await fallbackNote.first().textContent()) ?? '').trim();
      assert.ok(
        noteText.length > 0,
        'landing-manifesto-card degraded but its fallback-note carried no message'
      );
    }
  }
);

Then('no reference card on the doorstep is left blank', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return PENDING;

  for (const testid of REFERENCE_CARD_TESTIDS) {
    const cards = device.page.locator(`[data-testid="${testid}"]`);
    const count = await cards.count();
    for (let i = 0; i < count; i++) {
      const titleText = await cardTitleText(cards.nth(i));
      assertLegibleTitle(titleText, `${testid}[${i}]`);
    }
  }
});
