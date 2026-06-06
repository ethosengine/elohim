/* eslint-disable @typescript-eslint/require-await -- Cucumber step handlers are async by convention; some pending-guard returns and sync assertions don't await. */

/**
 * Deep-link delivery step definitions.
 *
 * Drives genesis/a2o/features/lamad/deep-link-delivery.feature — the §12 URL &
 * Routing Contract (Slice 1, "alpha green"). A learner handed a URL cold lands
 * on the actual rendered surface that URL names, because the doorway SPA
 * deep-link fallback serves the bundle's entry file for ROUTE sub-paths (final
 * segment has no '.') while ASSET misses stay honest 404s.
 *
 * These steps guard the live alpha failure of 2026-06-04, where
 * GET /lamad/path/foundations-christian-technology 404'd with a storage
 * app-file error even though the content was seeded, public, and reachable.
 *
 * Two transports, by scenario shape:
 *   - Render scenarios (@browser-only) drive an anonymous Playwright visitor and
 *     assert by RENDERED data-testid (render-verified discipline — never the
 *     HTML <title> or a raw response shell). The visitor opens the link cold
 *     with no auth, mirroring "someone shared me a link to public content".
 *   - The asset-miss scenario is a pure HTTP status check against the doorway
 *     (shared delivery fetchApp helper) — a browser navigation to a 404 JSON
 *     would render nothing to assert against; the honest-404 contract lives at
 *     the transport layer.
 *
 * DOM contract (Angular light-DOM, data-testid — added in this slice where a
 * root container lacked one, testid-only, no behaviour change):
 *   path-overview    — path overview root (a learner opened a path cold)
 *   path-navigator   — step navigator root (a learner deep-linked to a step)
 *   lamad-not-found  — the DESIGNED not-found page root (legacy doubled URL)
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { Human } from '../../src/framework/human.js';
import {
  CONTENT_VIEWER,
  NOT_FOUND,
  PATH_NAV,
  PATH_OVERVIEW,
} from '../../src/framework/pages/selectors.js';
import { doorwayToAppUrl } from '../../src/framework/utils/url.js';
import { E2EWorld } from '../../src/framework/world.js';
import { AppResponse, fetchApp, responseStore } from '../delivery.steps.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const PENDING = 'pending';
const VISITOR_NAME = '_deep-link-visitor';

/** A render-verify timeout generous enough for a cold bundle load + route match. */
const RENDER_TIMEOUT_MS = 20_000;

/**
 * Resolve (or lazily create) the anonymous deep-link visitor's Playwright
 * device. The visitor carries no auth and no fixture human — it opens shared
 * links cold, exactly as a recipient of a shared URL would. The device's
 * appUrl is the doorway's paired app origin (doorwayToAppUrl), so a '/lamad/…'
 * path resolves against the mounted bundle.
 *
 * Returns null when not in Playwright mode so the @browser-only render steps
 * degrade to 'pending' on API runs, mirroring the other browser step files.
 */
async function ensureVisitor(world: E2EWorld): Promise<PlaywrightDevice | null> {
  if (world.deviceMode !== 'playwright') {
    return null;
  }

  let human: Human;
  try {
    human = world.getHuman(VISITOR_NAME);
  } catch {
    human = new Human(VISITOR_NAME, { identifier: '', password: '', displayName: VISITOR_NAME });
    world.addHuman(VISITOR_NAME, human);
  }

  const existing = human.devices.find(d => d.type === 'playwright');
  if (existing) return existing as PlaywrightDevice;

  const [, doorway] = [...world.doorways][0] ?? [];
  assert.ok(doorway, 'No doorway registered. Did the Background step run?');

  const appUrl = doorwayToAppUrl(doorway.url);
  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
  const browser = await world.getBrowser();
  // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
  const device = new PlaywrightDevice(`${VISITOR_NAME}-pw`, appUrl, doorway.url, browser);
  await device.init();
  human.addDevice(device);
  world.onCleanup(async () => device.close());
  return device;
}

/** Wait for a root container testid to become visible; return true/false. */
async function rootRendered(device: PlaywrightDevice, testId: string): Promise<boolean> {
  return device.page
    .locator(`[data-testid="${testId}"]`)
    .first()
    .waitFor({ state: 'visible', timeout: RENDER_TIMEOUT_MS })
    .then(() => true)
    .catch(() => false);
}

/**
 * Fetch a URL WITHOUT following redirects, capturing status + headers + body.
 *
 * The shared `fetchApp` (steps/delivery.steps.ts) uses undici's `request`, which
 * does NOT auto-follow redirects (following requires the redirect interceptor /
 * a dispatcher option — deliberately not used here). That no-follow contract is
 * load-bearing for the Slice-3 doorway-302 assertions, where a followed
 * redirect would erase the 302 status and Location header we need to observe.
 * Same undici transport as `fetchApp` — the AppResponse shape (lower-cased
 * headers, Buffer body) matches so the SHARED responseStore Then assertions
 * resolve it unchanged.
 */
async function fetchAppNoRedirect(baseUrl: string, path: string): Promise<AppResponse> {
  const { statusCode, headers, body } = await request(`${baseUrl}${path}`);
  const data = Buffer.from(await body.arrayBuffer());
  const flatHeaders: Record<string, string> = {};
  for (const [key, value] of Object.entries(headers)) {
    if (typeof value === 'string') flatHeaders[key.toLowerCase()] = value;
    else if (Array.isArray(value)) flatHeaders[key.toLowerCase()] = value[0];
  }
  return { status: statusCode, headers: flatHeaders, body: data };
}

// ---------------------------------------------------------------------------
// Given — open a deep link cold
// ---------------------------------------------------------------------------

/**
 * "Given a learner opens the deep link {string} cold" — the visitor navigates
 * to the path with no prior session, as a recipient of a shared link would.
 * The subsequent Then steps verify what actually rendered.
 *
 * Example:
 *   Given a learner opens the deep link "/lamad/path/foundations-christian-technology" cold
 */
Given(
  'a learner opens the deep link {string} cold',
  async function (this: E2EWorld, deepLink: string) {
    const device = await ensureVisitor(this);
    if (!device) {
      return PENDING;
    }
    await device.navigate(deepLink);
  }
);

// ---------------------------------------------------------------------------
// When — request a missing asset (transport-layer, no browser)
// ---------------------------------------------------------------------------

/**
 * "When the missing asset {string} is requested from doorway {string}" — a raw
 * HTTP GET against the doorway (no browser). The path's final segment carries a
 * dot, so it classifies as an ASSET: the SPA fallback must NOT mask the miss
 * with index.html.
 *
 * The capture is stored in the SHARED delivery responseStore so the existing
 * `Then the response status is {int}` assertion (steps/delivery.steps.ts)
 * resolves it — full reuse, no ambiguous redefinition. Only the body-shape
 * assertion below is unique to this feature.
 *
 * Example:
 *   When the missing asset "/lamad/main-DOESNOTEXIST00000000.js" is requested from doorway "alpha"
 */
When(
  'the missing asset {string} is requested from doorway {string}',
  async function (this: E2EWorld, assetPath: string, doorwayId: string) {
    const doorway = this.getDoorway(doorwayId);
    const resp = await fetchApp(doorway.url, assetPath);
    responseStore.set(this, resp);
  }
);

// ---------------------------------------------------------------------------
// Then — render-verified assertions
// ---------------------------------------------------------------------------

/**
 * "Then the lamad path overview renders" — the cold canonical URL resolved to
 * the path overview surface (the ROUTE fallback served entry_file and Angular
 * matched path/:pathId). Render-verified by the overview's root testid.
 *
 * Example: Then the lamad path overview renders
 */
Then('the lamad path overview renders', async function (this: E2EWorld) {
  const device = await ensureVisitor(this);
  if (!device) {
    return PENDING;
  }
  const rendered = await rootRendered(device, PATH_OVERVIEW.ROOT);
  assert.ok(
    rendered,
    `Expected the lamad path overview to render (data-testid="${PATH_OVERVIEW.ROOT}"); ` +
      `URL is "${device.page.url()}"`
  );
});

/**
 * "Then the lamad step navigator renders" — the deeper step ROUTE resolved to
 * the step navigator (Angular matched path/:pathId/step/:stepIndex). The step
 * index was a route segment, not a ZIP filename. Render-verified by the
 * navigator's root testid.
 *
 * Example: Then the lamad step navigator renders
 */
Then('the lamad step navigator renders', async function (this: E2EWorld) {
  const device = await ensureVisitor(this);
  if (!device) {
    return PENDING;
  }
  const rendered = await rootRendered(device, PATH_NAV.ROOT);
  assert.ok(
    rendered,
    `Expected the lamad step navigator to render (data-testid="${PATH_NAV.ROOT}"); ` +
      `URL is "${device.page.url()}"`
  );
});

/**
 * "Then the lamad designed not-found page renders" — the legacy doubled URL
 * served the bundle, the router fell through to ** and rendered the DESIGNED
 * not-found page (not raw JSON). Render-verified by the not-found root testid.
 *
 * Example: Then the lamad designed not-found page renders
 */
Then('the lamad designed not-found page renders', async function (this: E2EWorld) {
  const device = await ensureVisitor(this);
  if (!device) {
    return PENDING;
  }
  const rendered = await rootRendered(device, NOT_FOUND.ROOT);
  assert.ok(
    rendered,
    `Expected the DESIGNED lamad not-found page to render (data-testid="${NOT_FOUND.ROOT}"); ` +
      `URL is "${device.page.url()}"`
  );
});

/**
 * "Then the rendered surface is not a raw error response" — the cross-cutting
 * render-verified guard: the page is a mounted Angular surface, not the bare
 * JSON error shell the live alpha failure served. We assert the doorway's
 * file-not-found error string is NOT the document body. (The positive render
 * assertion above already proves the right surface mounted; this catches a
 * regression where the body is JSON yet a stale testid lingers in cache.)
 *
 * Example: And the rendered surface is not a raw error response
 */
Then('the rendered surface is not a raw error response', async function (this: E2EWorld) {
  const device = await ensureVisitor(this);
  if (!device) {
    return PENDING;
  }
  const bodyText = ((await device.page.locator('body').first().textContent()) ?? '').trim();
  assert.ok(
    !bodyText.includes('File not found in app'),
    `Rendered surface is the raw storage error shell, not a designed page: "${bodyText.slice(0, 200)}"`
  );
});

// ---------------------------------------------------------------------------
// Then — transport-layer assertion (asset miss)
//
// `Then the response status is {int}` is the SHARED delivery assertion
// (steps/delivery.steps.ts) — reused, not redefined. Only the body-shape guard
// below is unique to the deep-link contract.
// ---------------------------------------------------------------------------

/**
 * "Given a learner opens the deep link {string}" — the visitor navigates to
 * the path with no prior session. Variant without "cold" suffix; used by
 * Slice-2 scenarios that don't carry the cold-start semantic (the link may
 * be a warm navigate or a cross-bundle handoff target).
 *
 * Example:
 *   Given a learner opens the deep link "/epr/foundations-christian-technology"
 */
Given('a learner opens the deep link {string}', async function (this: E2EWorld, deepLink: string) {
  const device = await ensureVisitor(this);
  if (!device) {
    return PENDING;
  }
  await device.navigate(deepLink);
});

/**
 * "Then the cross-pillar resource viewer renders" — the universal /epr/{id}
 * address resolved and the shell's epr/:resourceId route rendered the
 * cross-pillar resource viewer. Render-verified by the viewer's root testid.
 *
 * Example: Then the cross-pillar resource viewer renders
 */
Then('the cross-pillar resource viewer renders', async function (this: E2EWorld) {
  const device = await ensureVisitor(this);
  if (!device) {
    return PENDING;
  }
  const rendered = await rootRendered(device, CONTENT_VIEWER.ROOT);
  assert.ok(
    rendered,
    `Expected the cross-pillar resource viewer to render (data-testid="${CONTENT_VIEWER.ROOT}"); ` +
      `URL is "${device.page.url()}"`
  );
});

/**
 * "When the learner follows the View Resource Details link" — the step
 * navigator's "View Resource Details" affordance is a plain href to the
 * universal /epr/{id} address. Clicking it triggers a full doorway load
 * (captured by the epr-link interceptor) and lands on the shell viewer.
 * Regression anchor for the resource self-loop redirect killed in Slice 2.
 *
 * Example: When the learner follows the View Resource Details link
 */
When('the learner follows the View Resource Details link', async function (this: E2EWorld) {
  const device = await ensureVisitor(this);
  if (!device) {
    return PENDING;
  }
  await device.page.locator(`[data-testid="${PATH_NAV.VIEW_RESOURCE}"]`).first().click();
  await device.page.waitForLoadState('domcontentloaded');
});

/**
 * "Then the response body is JSON, not an index.html shell" — the honest-404
 * contract: an ASSET miss must NOT be masked by the SPA fallback serving
 * index.html. The body is a JSON error, not an HTML document. Reads the capture
 * the existing status assertion also reads (the shared delivery responseStore).
 *
 * Example: And the response body is JSON, not an index.html shell
 */
Then('the response body is JSON, not an index.html shell', async function (this: E2EWorld) {
  const resp = responseStore.get(this);
  assert.ok(resp, 'No response captured — run the missing-asset request step first');
  const bodyText = resp.body.toString('utf8');

  const looksHtml =
    /^\s*<!doctype html/i.test(bodyText) ||
    /<html[\s>]/i.test(bodyText) ||
    /<app-root[\s/>]/i.test(bodyText);
  assert.ok(
    !looksHtml,
    `Asset miss was masked by an index.html shell (SPA fallback leaked to an ASSET): "${bodyText.slice(0, 200)}"`
  );

  // Positive shape: a JSON content-type and/or a parseable JSON error body.
  const contentType = resp.headers['content-type'] ?? '';
  const jsonContentType = contentType.includes('application/json');
  let jsonParsable = false;
  try {
    JSON.parse(bodyText);
    jsonParsable = true;
  } catch {
    jsonParsable = false;
  }
  assert.ok(
    jsonContentType || jsonParsable,
    `Expected a JSON error body for the asset miss; content-type="${contentType}", body="${bodyText.slice(0, 200)}"`
  );
});

// ---------------------------------------------------------------------------
// Slice 3 — doorway-side routing (HTTP transport, no browser)
//
// The doorway honors notarized alias promises (redirectTemplates / redirects_from)
// and claimed-type 302s BEFORE any bundle boots, and materializes a /sitemap.xml
// projection of the routing table. These steps observe that substrate behavior at
// the transport layer with redirects unfollowed, so the 302 status + Location are
// directly assertable. `Then the response status is {int}` is the SHARED delivery
// assertion (steps/delivery.steps.ts) — reused, not redefined.
// ---------------------------------------------------------------------------

/**
 * "When the path {string} is requested without following redirects from doorway
 * {string}" — a raw HTTP GET against the doorway with redirects unfollowed, so a
 * doorway-side 302 (alias promise or claimed-type redirect) surfaces as a 302
 * status + Location header rather than being chased to its target.
 *
 * The capture is stored in the SHARED delivery responseStore so the existing
 * `Then the response status is {int}` assertion resolves it.
 *
 * Example:
 *   When the path "/lamad/resource/abc" is requested without following redirects from doorway "alpha"
 */
When(
  'the path {string} is requested without following redirects from doorway {string}',
  async function (this: E2EWorld, path: string, doorwayId: string) {
    const doorway = this.getDoorway(doorwayId);
    const resp = await fetchAppNoRedirect(doorway.url, path);
    responseStore.set(this, resp);
  }
);

/**
 * "Then the response Location header is {string}" — the doorway's 302 Location
 * is the expected canonical target (spec §4 alias law / §5.1 claimed-type
 * redirect). Reads the shared delivery responseStore.
 *
 * Example: And the response Location header is "/epr/abc"
 */
Then('the response Location header is {string}', function (this: E2EWorld, expected: string) {
  const resp = responseStore.get(this);
  assert.ok(resp, 'No response captured — run the request step first');
  assert.equal(resp.headers['location'], expected);
});

/**
 * "Then the response body contains {string}" — the response body includes the
 * given substring (spec §7.5: the sitemap enumerates the claimed static plane).
 * Reads the shared delivery responseStore.
 *
 * Example: And the response body contains "/lamad/path/foundations-christian-technology"
 */
Then('the response body contains {string}', function (this: E2EWorld, needle: string) {
  const resp = responseStore.get(this);
  assert.ok(resp, 'No response captured — run the request step first');
  assert.ok(resp.body.toString('utf8').includes(needle), `Expected body to contain "${needle}"`);
});

When('the learner follows the View as Content link', async function (this: E2EWorld) {
  const device = await ensureVisitor(this);
  if (!device) {
    return PENDING;
  }
  await device.page.locator(`[data-testid="${PATH_OVERVIEW.VIEW_AS_CONTENT}"]`).first().click();
  await device.page.waitForLoadState('domcontentloaded');
});
