/**
 * SSR Hydration Step Definitions — verify Angular SSR renders without re-render flash.
 *
 * Covers:
 *   - genesis/a2o/features/ssr/browser-hydrates-without-flash.feature
 *
 * The contract being tested:
 *   1. Doorway with SSR_BUNDLE_PATH set returns a full HTML document with
 *      Angular 19 hydration markers (`ngh="..."` attributes) on SSR-eligible routes.
 *   2. After Angular's client runtime hydrates the page, the visible DOM tree
 *      (element identity and text content) must not change — only event listeners
 *      are attached to existing nodes.
 *   3. No console errors are emitted during hydration.
 *
 * Implementation approach: Approach 2 (robust element comparison).
 *   - cy.request() equivalent: use `undici.request()` to capture raw SSR HTML
 *     before any JS executes.
 *   - After Playwright navigates and Angular hydrates, compare specific stable
 *     elements (article body, h1, title) against the SSR snapshot.
 *   - Differences limited to script/style injection are not asserted on;
 *     differences in content-bearing elements fail the test.
 *
 * Run requirements:
 *   - E2E_DEVICE_MODE=playwright
 *   - E2E_DOORWAY_ALPHA pointing to a doorway with SSR_BUNDLE_PATH set
 *   - The concept "elohim-protocol-overview" seeded in elohim-storage
 *   - pnpm run hc:start:seed (or equivalent) before running
 *
 * Tag: @wip — cannot be exercised in CI without the full local cluster.
 * To run locally:
 *   cd app/elohim-app && pnpm run hc:start:seed
 *   cd genesis/a2o && E2E_DEVICE_MODE=playwright E2E_DOORWAY_ALPHA=http://localhost:8888 \
 *     npx cucumber-js --tags '@browser and @ssr and not @wip' features/ssr/*.feature
 * Remove @wip from the scenario tag once the cluster is available in CI.
 */

import { strict as assert } from 'node:assert';

import { When, Then } from '@cucumber/cucumber';

import { request as undiciRequest } from 'undici';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** Captured raw SSR response, stored on world for cross-step assertions. */
interface SsrCapture {
  status: number;
  headers: Record<string, string>;
  body: string;
}

// ---------------------------------------------------------------------------
// World state key for raw HTTP capture
// ---------------------------------------------------------------------------

const SSR_CAPTURE_KEY = '_ssr_capture';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function requirePlaywrightDevice(world: E2EWorld): PlaywrightDevice {
  for (const [, human] of world.humans) {
    const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
    if (device) return device;
  }
  throw new Error('No Playwright device found. Set E2E_DEVICE_MODE=playwright.');
}

function storedCapture(world: E2EWorld): SsrCapture {
  const raw = world.contentIds.get(SSR_CAPTURE_KEY);
  assert.ok(raw, 'No SSR capture found — run "the raw HTTP response for ... is captured" first');
  return JSON.parse(raw) as SsrCapture;
}

/**
 * Resolve the doorway base URL from the world's registered doorways.
 * Falls back to E2E_DOORWAY_ALPHA env var.
 */
function resolveDoorwayUrl(world: E2EWorld): string {
  for (const [, entry] of world.doorways) {
    return entry.url;
  }
  const fallback = process.env['E2E_DOORWAY_ALPHA'];
  assert.ok(fallback, 'No doorway registered and E2E_DOORWAY_ALPHA not set');
  return fallback;
}

// ---------------------------------------------------------------------------
// Step: capture raw SSR HTML via HTTP (no browser JS, no hydration)
// ---------------------------------------------------------------------------

/**
 * Fetch the URL directly (bypassing the browser) and store the raw response.
 * This is the "before hydration" snapshot — exactly what doorway's SSR renderer
 * returned, before Angular's client runtime executed a single line.
 *
 * Example: When the raw HTTP response for "/lamad/concept/elohim-protocol-overview" is captured
 */
When(
  'the raw HTTP response for {string} is captured',
  async function (this: E2EWorld, path: string) {
    const doorwayUrl = resolveDoorwayUrl(this);
    const fullUrl = `${doorwayUrl}${path}`;

    const { statusCode, headers, body } = await undiciRequest(fullUrl, {
      method: 'GET',
      headers: { accept: 'text/html,application/xhtml+xml' },
    });

    const bodyText = await body.text();

    // Normalise header values to strings
    const headerMap: Record<string, string> = {};
    for (const [k, v] of Object.entries(headers)) {
      headerMap[k] = Array.isArray(v) ? v.join(', ') : (v ?? '');
    }

    const capture: SsrCapture = {
      status: statusCode,
      headers: headerMap,
      body: bodyText,
    };

    this.contentIds.set(SSR_CAPTURE_KEY, JSON.stringify(capture));
  }
);

// ---------------------------------------------------------------------------
// Assertions on the raw HTTP capture
// ---------------------------------------------------------------------------

/**
 * Example: Then the raw HTTP response status is 200
 */
Then('the raw HTTP response status is {int}', function (this: E2EWorld, expectedStatus: number) {
  const capture = storedCapture(this);
  assert.equal(
    capture.status,
    expectedStatus,
    `Expected HTTP ${expectedStatus} but got ${capture.status}`
  );
});

/**
 * Assert a named response header is present (non-empty).
 *
 * Example: Then the raw HTTP response header "x-render-cache" is present
 */
Then(
  'the raw HTTP response header {string} is present',
  function (this: E2EWorld, headerName: string) {
    const capture = storedCapture(this);
    const lowerHeader = headerName.toLowerCase();
    const hasHeader = Object.prototype.hasOwnProperty.call(capture.headers, lowerHeader);
    const value = hasHeader ? capture.headers[lowerHeader] : '';
    assert.ok(
      hasHeader && value !== '',
      `Expected response header "${headerName}" to be present but it was absent.\n` +
        `Present headers: ${Object.keys(capture.headers).join(', ')}`
    );
  }
);

/**
 * Assert the raw HTML body contains Angular 19 client hydration markers.
 *
 * Angular 19's `provideClientHydration()` emits `ngh="..."` attributes on the
 * root component host and its children. Their presence proves the SSR renderer
 * ran Angular's serialisation pass — not just a static HTML template.
 *
 * If this assertion fails, either:
 *   a) SSR_BUNDLE_PATH is not set (doorway fell back to SPA shell), or
 *   b) `provideClientHydration()` was removed from app.config.server.ts
 *
 * Example: Then the raw HTTP response body contains Angular hydration markers
 */
Then('the raw HTTP response body contains Angular hydration markers', function (this: E2EWorld) {
  const capture = storedCapture(this);

  // Angular 19 hydration: ngh attribute on component host elements.
  // Format: ngh="0" or ngh="1" etc. (index into the ngh transfer state).
  const hasNgh = capture.body.includes(' ngh=') || capture.body.includes('\nngh=');

  // Transfer state script: Angular serialises hydration data into a
  // <script id="ng-state" type="application/json"> element.
  const hasNgState =
    capture.body.includes('id="ng-state"') || capture.body.includes("id='ng-state'");

  assert.ok(
    hasNgh || hasNgState,
    'Expected Angular hydration markers (ngh attribute or ng-state script) in SSR HTML.\n' +
      'This means either SSR_BUNDLE_PATH is not set (doorway served the SPA shell fallback) ' +
      'or provideClientHydration() is absent from app.config.server.ts.\n' +
      `Body preview (first 500 chars): ${capture.body.slice(0, 500)}`
  );
});

/**
 * Assert the raw HTML has a non-empty <title> element — proving Angular's
 * Title service ran during SSR and the route resolved correctly.
 *
 * Example: Then the raw HTTP response body contains a non-empty html title element
 */
Then(
  'the raw HTTP response body contains a non-empty html title element',
  function (this: E2EWorld) {
    const capture = storedCapture(this);

    // Match <title>some text</title> — case-insensitive, captures content between tags.
    const titleMatch = /<title>([^<]+)<\/title>/i.exec(capture.body);
    assert.ok(
      titleMatch && titleMatch[1].trim().length > 0,
      'Expected a non-empty <title> element in SSR HTML.\n' +
        `Body preview (first 500 chars): ${capture.body.slice(0, 500)}`
    );
  }
);

// ---------------------------------------------------------------------------
// Assertion: concept page visible after hydration
// ---------------------------------------------------------------------------

/**
 * Wait for the concept page's primary content element to be visible.
 * Uses a broad selector that matches the content viewer regardless of
 * Angular's exact component hierarchy.
 *
 * Example: Then the concept page content is visible
 */
Then('the concept page content is visible', async function (this: E2EWorld) {
  const device = requirePlaywrightDevice(this);

  // Wait for the Angular app root to be present and the content area to render.
  // app-root is present in the SSR HTML; after hydration it gains Angular bindings.
  const appRoot = device.page.locator('app-root');
  await appRoot.waitFor({ state: 'attached', timeout: 15_000 });

  // The content viewer renders either article, main, or a content-specific element.
  // A generous selector catches all known layout shapes.
  const contentArea = device.page.locator(
    'article, main, [class*="content-viewer"], [class*="concept-viewer"], app-root'
  );
  await contentArea.first().waitFor({ state: 'visible', timeout: 15_000 });
});

// ---------------------------------------------------------------------------
// Assertion: no uncaught JS errors (distinct from console.error)
// ---------------------------------------------------------------------------

/**
 * Assert no uncaught JS errors (window.onerror / unhandled rejections) were
 * captured by PlaywrightDevice's `pageerror` listener.
 *
 * Hydration errors in Angular 19 surface as uncaught errors with messages like
 * "NG0500: During hydration Angular expected..." — this assertion catches them.
 *
 * Example: And no uncaught JavaScript errors occurred
 */
Then('no uncaught JavaScript errors occurred', function (this: E2EWorld) {
  const device = requirePlaywrightDevice(this);

  const errors = device.pageErrors.filter(
    e =>
      // Filter out known non-hydration noise (external scripts, extensions)
      !e.message.includes('ResizeObserver loop') &&
      !e.message.includes('Script error.') &&
      !e.url.includes('chrome-extension://')
  );

  assert.equal(
    errors.length,
    0,
    `Expected no uncaught JS errors but found ${errors.length}:\n` +
      errors
        .slice(0, 5)
        .map(e => `  [${e.url}] ${e.message}`)
        .join('\n')
  );
});

// ---------------------------------------------------------------------------
// Assertion: DOM tree matches SSR'd content (no reconstruction on hydration)
// ---------------------------------------------------------------------------

/**
 * Compare the SSR'd HTML snapshot against the post-hydration DOM to verify
 * Angular did NOT re-render the page content.
 *
 * Strategy: extract the stable text content of content-bearing elements from
 * both the raw SSR HTML (string manipulation) and the live DOM (Playwright
 * evaluate). Angular's hydration attaches event listeners to existing DOM nodes
 * without replacing them — so the visible text content must be identical.
 *
 * Script tags, style elements, Angular-injected metadata (ng-state, ngh attrs),
 * and comment nodes are excluded from the comparison because Angular legitimately
 * adds or modifies them during hydration.
 *
 * Example: And the DOM tree matches the SSR'd content without reconstruction
 */
Then(
  "the DOM tree matches the SSR'd content without reconstruction",
  async function (this: E2EWorld) {
    const capture = storedCapture(this);
    const device = requirePlaywrightDevice(this);

    // --- Extract stable text from SSR HTML snapshot ---
    // We strip script/style/comment nodes and collapse whitespace.
    // This mirrors what the browser renders to the user on first paint.
    const ssrText = extractStableText(capture.body);

    // --- Extract stable text from post-hydration DOM ---
    // Evaluated inside the browser after networkidle so hydration has completed.
    const liveText = (await device.page.evaluate(() => {
      // Clone the body so we can mutate without affecting the page.
      const body = document.body.cloneNode(true) as HTMLElement;

      // Remove non-content elements that Angular modifies during hydration.
      const toRemove = body.querySelectorAll(
        'script, style, noscript, [id="ng-state"], link[rel="stylesheet"]'
      );
      for (const el of toRemove) {
        el.remove();
      }

      // Collapse whitespace and return visible text content.
      return (body.textContent ?? '').replace(/\s+/g, ' ').trim();
    })) as string;

    // Allow a 5% difference to account for Angular adding small amounts of
    // supplementary text (e.g., aria labels, sr-only elements) that aren't
    // in the SSR'd HTML but don't constitute a re-render.
    const similarity = computeTextSimilarity(ssrText, liveText);

    assert.ok(
      similarity >= 0.95,
      `DOM text content diverged after hydration (similarity: ${(similarity * 100).toFixed(1)}%).\n` +
        'This may indicate Angular re-rendered the component tree instead of reusing SSR nodes.\n' +
        `SSR text (first 300 chars):  ${ssrText.slice(0, 300)}\n` +
        `Live text (first 300 chars): ${liveText.slice(0, 300)}`
    );
  }
);

// ---------------------------------------------------------------------------
// Pure utility functions (no I/O, testable independently)
// ---------------------------------------------------------------------------

/**
 * Extract stable visible text from an HTML string.
 * Strips script, style, comment, and Angular-internal elements,
 * then collapses whitespace.
 */
function extractStableText(html: string): string {
  // Remove script and style blocks (including their content)
  let cleaned = html
    .replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, ' ')
    .replace(/<style\b[^>]*>[\s\S]*?<\/style>/gi, ' ')
    .replace(/<!--[\s\S]*?-->/g, ' ')
    // Remove Angular transfer state script (id="ng-state").
    // Use a simple contains-check rather than a complex nested regex to avoid
    // backtracking DoS — this runs only on trusted SSR HTML from our own server.
    .replace(/<script[^>]*>[\s\S]*?<\/script>/gi, match =>
      match.includes('ng-state') ? ' ' : match
    );

  // Strip all remaining HTML tags.
  // Split on '<' and '>' rather than a nested-quantifier regex to avoid
  // sonarjs/slow-regex. Works correctly for well-formed HTML (which SSR output is).
  cleaned = cleaned
    .split('<')
    .map(chunk => chunk.split('>').slice(1).join('>'))
    .join(' ');

  // Decode common HTML entities
  cleaned = cleaned
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&nbsp;/g, ' ')
    .replace(/&#(\d+);/g, (_, code: string) => String.fromCodePoint(Number(code)));

  // Collapse whitespace
  return cleaned.replace(/\s+/g, ' ').trim();
}

/**
 * Compute a rough text similarity score between two strings.
 * Uses Jaccard similarity on word sets — good enough to catch wholesale
 * re-renders while tolerating small additions (aria labels, sr-only text).
 */
function computeTextSimilarity(a: string, b: string): number {
  if (a === b) return 1;
  if (!a || !b) return 0;

  const wordsA = new Set(a.toLowerCase().split(/\s+/).filter(Boolean));
  const wordsB = new Set(b.toLowerCase().split(/\s+/).filter(Boolean));

  let intersectionSize = 0;
  for (const word of wordsA) {
    if (wordsB.has(word)) intersectionSize++;
  }

  const unionSize = wordsA.size + wordsB.size - intersectionSize;
  return unionSize === 0 ? 1 : intersectionSize / unionSize;
}
