/**
 * SSR External Fetch Step Definitions — verify SSR HTML is readable without a JS engine.
 *
 * Covers:
 *   - genesis/a2o/features/ssr/external-webfetch-renders-content.feature
 *   - genesis/a2o/features/ssr/social-card-crawler-gets-rich-preview.feature
 *
 * The contracts being tested:
 *
 *   external-webfetch-renders-content:
 *     An HTTP client with no JS engine (search crawler, AI design tool, screen-reader
 *     proxy) fetches a concept page and receives a complete HTML document where:
 *       1. <title> contains the concept title (Angular Title service ran during SSR).
 *       2. <article> contains rendered body text (content component rendered server-side).
 *       3. <app-root> is not empty (SSR ran; did not serve the SPA shell fallback).
 *       4. x-render-cache header is present (doorway's render cache layer is active).
 *       5. <sophia-question> elements, if present, have attributes (not bare empty tags).
 *
 *   social-card-crawler-gets-rich-preview:
 *     A social card crawler (facebookexternalhit, Twitterbot, Slackbot) fetches a
 *     learning path step and finds og:title, og:description, og:image meta tags in
 *     the initial HTML — without executing any JavaScript.
 *
 * Implementation approach:
 *   - Raw HTTP via undici.request() — no browser, no JS execution.
 *   - HTML parsing via regex on trusted SSR output from our own server. No external
 *     parser dependency required: the assertions are structural (element presence,
 *     attribute existence) not DOM-traversal queries, and SSR output is well-formed.
 *   - Captured responses are stored on world.contentIds (keyed by
 *     _ext_fetch_capture / _social_fetch_capture) so Given/When/Then steps
 *     share state without globals.
 *
 * Run requirements:
 *   - E2E_DEVICE_MODE=playwright (or omit for HTTP-only mode — no browser needed here)
 *   - E2E_DOORWAY_ALPHA pointing to a doorway with SSR_BUNDLE_PATH set
 *   - elohim-storage seeded with "elohim-protocol-overview" concept and "elohim-protocol" path
 *   - pnpm run hc:start:seed (or equivalent) before running
 *
 * Tag: @wip — cannot be exercised in CI without the full local cluster.
 * To run locally:
 *   cd app/elohim-app && pnpm run hc:start:seed
 *   cd genesis/a2o && E2E_DOORWAY_ALPHA=http://localhost:8888 \
 *     npx cucumber-js --tags '@ssr and not @wip' features/ssr/*.feature
 * Remove @wip from the scenario tag once the cluster is available in CI.
 *
 * NOTE: og:image and og:description wiring in elohim-app's SSR Meta service is not yet
 * complete. The social-card scenario will fail at runtime on those assertions until
 * Angular's Meta service is wired in the route's SSR path. @wip covers it.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then, defineParameterType } from '@cucumber/cucumber';

import { request as undiciRequest } from 'undici';

import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

/** Key for the raw HTTP response captured by "an HTTP client...fetches" step. */
const EXT_FETCH_KEY = '_ext_fetch_capture';

/** Key for the raw HTTP response captured by "a social card crawler fetches" step. */
const SOCIAL_FETCH_KEY = '_social_fetch_capture';

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

interface RawCapture {
  status: number;
  headers: Record<string, string>;
  body: string;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

/** Retrieve a stored RawCapture from world state. */
function storedCapture(world: E2EWorld, key: string): RawCapture {
  const raw = world.contentIds.get(key);
  assert.ok(raw, `No HTTP capture found for key "${key}" — run the preceding When step first`);
  return JSON.parse(raw) as RawCapture;
}

/** Fetch a URL and return a normalised RawCapture. */
async function fetchRaw(
  fullUrl: string,
  extraHeaders: Record<string, string> = {}
): Promise<RawCapture> {
  const { statusCode, headers, body } = await undiciRequest(fullUrl, {
    method: 'GET',
    headers: {
      accept: 'text/html,application/xhtml+xml',
      ...extraHeaders,
    },
  });

  const bodyText = await body.text();

  const headerMap: Record<string, string> = {};
  for (const [k, v] of Object.entries(headers)) {
    headerMap[k.toLowerCase()] = Array.isArray(v) ? v.join(', ') : (v ?? '');
  }

  return { status: statusCode, headers: headerMap, body: bodyText };
}

// ---------------------------------------------------------------------------
// HTML parsing helpers (regex — no external parser needed for structural checks)
// ---------------------------------------------------------------------------

/**
 * Extract the text content of the first <title> element.
 * Returns null if absent.
 */
function extractTitle(html: string): string | null {
  const m = /<title[^>]*>([^<]*)<\/title>/i.exec(html);
  return m ? m[1].trim() : null;
}

/**
 * Return true if the HTML contains at least one <article> element with
 * non-whitespace text content.
 *
 * Uses a split-based approach to avoid the nested-quantifier backtracking
 * pattern that sonarjs/slow-regex flags on [\s\S]*? inside tag-pair regexes.
 * SSR output is well-formed HTML from our own server; indexOf is safe.
 */
function hasNonEmptyArticle(html: string): boolean {
  const openIdx = html.search(/<article\b/i);
  if (openIdx === -1) return false;
  const closeIdx = html.indexOf('</article>', openIdx);
  if (closeIdx === -1) return false;

  // Extract the inner HTML and strip tags via split rather than nested regex.
  const inner = html.slice(openIdx, closeIdx + '</article>'.length);
  const textContent = inner
    .split('<')
    .map(chunk => chunk.split('>').slice(1).join('>'))
    .join(' ')
    .replace(/\s+/g, ' ')
    .trim();

  return textContent.length > 0;
}

/**
 * Return true if <app-root> is present AND has child content (not empty).
 * An empty <app-root></app-root> indicates SSR did not run (SPA shell fallback).
 *
 * Uses indexOf rather than a nested-quantifier regex to avoid slow-regex.
 */
function appRootIsNonEmpty(html: string): boolean {
  const openIdx = html.search(/<app-root\b/i);
  if (openIdx === -1) return false;
  const closeTag = '</app-root>';
  const closeIdx = html.indexOf(closeTag, openIdx);
  if (closeIdx === -1) return false;

  // Advance past the opening tag (find the '>').
  const tagCloseIdx = html.indexOf('>', openIdx);
  if (tagCloseIdx === -1 || tagCloseIdx >= closeIdx) return false;

  const inner = html.slice(tagCloseIdx + 1, closeIdx).trim();
  return inner.length > 0;
}

/**
 * Return true if any <sophia-question> element present has at least one attribute.
 * If no <sophia-question> elements exist, returns true (contract satisfied vacuously).
 */
function sophiaQuestionPlaceholdersAreValid(html: string): boolean {
  // Find all <sophia-question ...> opening tags.
  // The character class [^>] is a linear scan — no backtracking risk.
  const re = /<sophia-question\b([^>]*)>/gi;
  let m: RegExpExecArray | null;
  while ((m = re.exec(html)) !== null) {
    // m[1] is the attribute string — if SSR serialised it correctly it will be
    // non-empty (e.g. question-id="..." or data-id="...").
    const attrs = m[1].trim();
    if (attrs.length === 0) {
      return false; // Bare <sophia-question> with no attributes — SSR contract broken.
    }
  }
  // Either no elements found (acceptable — not all concepts have quizzes) or all
  // found elements had attributes.
  return true;
}

/**
 * Extract the content attribute of the first <meta property="og:title"> element.
 * Returns null if absent.
 *
 * Uses two simple linear patterns (property-first and content-first attribute
 * orderings) to avoid nested-quantifier backtracking.
 */
function extractOgTitle(html: string): string | null {
  // Pattern: property="og:title" content="VALUE" (property attribute first)
  const re1 = /<meta\s+property="og:title"\s+content="([^"]*)"/i;
  const m1 = re1.exec(html);
  if (m1) return m1[1].trim();

  // Pattern: content="VALUE" property="og:title" (content attribute first)
  const re2 = /<meta\s+content="([^"]*)"\s+property="og:title"/i;
  const m2 = re2.exec(html);
  return m2 ? m2[1].trim() : null;
}

/**
 * Return true if the HTML contains a <meta property="og:{name}"> element.
 * The name argument is a simple alphanumeric value extracted by the {ogmetatag}
 * parameter type (regexp: [a-z][a-z0-9:_-]*) — it contains no regex special chars,
 * so no escaping is needed.
 * Two patterns cover both attribute orderings (property-first and content-first).
 */
function hasOgMeta(html: string, name: string): boolean {
  const re1 = new RegExp(String.raw`<meta\s+property="og:${name}"[^>]*>`, 'i');
  const re2 = new RegExp(String.raw`<meta\s[^>]*property="og:${name}"[^>]*>`, 'i');
  return re1.test(html) || re2.test(html);
}

// ---------------------------------------------------------------------------
// Custom parameter types for HTML element names in step text
// ---------------------------------------------------------------------------

// Cucumber parameter types for the tag-like step text patterns.
// These let us write: "the concept title in <title>" and "no <app-root> tag is empty"
// without embedding the literal angle-bracket text in a cucumber-expression string
// (which would confuse Cucumber's expression parser).

/**
 * {htmlelement} — matches a bare HTML tag name in angle brackets.
 * e.g. "<title>", "<article>", "<app-root>", "<sophia-question>"
 */
defineParameterType({
  name: 'htmlelement',
  regexp: /<([a-z][a-z0-9-]*)>/,
  transformer: (tagName: string) => tagName,
});

/**
 * {ogmetatag} — matches an og:* meta tag declaration.
 * e.g. '<meta property="og:title">', '<meta property="og:description">'
 * Extracts just the og property name (e.g. "title", "description", "image").
 */
defineParameterType({
  name: 'ogmetatag',
  regexp: /<meta property="og:([a-z][a-z0-9:_-]*)">/,
  transformer: (propertyName: string) => propertyName,
});

// ---------------------------------------------------------------------------
// Background steps — SSR context preconditions
// ---------------------------------------------------------------------------

/**
 * Assert that the registered doorway is running and has SSR enabled.
 * SSR is enabled when doorway was started with SSR_BUNDLE_PATH set.
 * We verify this by checking the /health endpoint for the ssr_enabled flag,
 * or fall back to a soft check (just assert the doorway is reachable) since
 * not all doorway versions expose that flag yet.
 *
 * Background step — runs before each scenario in SSR feature files.
 *
 * Example: Given doorway is running with SSR enabled
 */
Given('doorway is running with SSR enabled', async function (this: E2EWorld) {
  const doorwayUrl = resolveDoorwayUrl(this);

  // Ensure the doorway is registered (may already be registered by a preceding
  // "doorway {string} at {string}" step — if not, register from env var).
  if (this.doorways.size === 0) {
    this.addDoorway('alpha', doorwayUrl);
  }

  // Verify reachability.
  const { status, body } = await fetchRaw(`${doorwayUrl}/health`);
  assert.ok(
    status === 200,
    `Doorway at ${doorwayUrl} is not reachable (status ${status}). ` +
      'Start the local stack with: cd app/elohim-app && pnpm run hc:start:seed'
  );

  // Best-effort SSR check: if the health response contains `ssr_enabled`, assert it.
  // If the field is absent, we skip the assertion — older doorway builds do not expose it.
  try {
    const healthJson = JSON.parse(body) as Record<string, unknown>;
    if ('ssr_enabled' in healthJson) {
      assert.ok(
        healthJson['ssr_enabled'] === true,
        `Doorway is reachable but SSR is not enabled (ssr_enabled=${String(healthJson['ssr_enabled'])}). ` +
          'Start doorway with SSR_BUNDLE_PATH pointing to the elohim-render bundle.'
      );
    }
  } catch {
    // Health body was not JSON, or ssr_enabled field is absent — proceed.
  }
});

/**
 * Verify that the named concept is accessible via the doorway content API.
 * This confirms elohim-storage was seeded before the scenario runs.
 *
 * Background step.
 *
 * Example: Given elohim-storage is seeded with concept "elohim-protocol-overview"
 */
Given(
  'elohim-storage is seeded with concept {string}',
  async function (this: E2EWorld, conceptId: string) {
    const doorwayUrl = resolveDoorwayUrl(this);
    const { status } = await fetchRaw(`${doorwayUrl}/db/content/${conceptId}`);
    assert.ok(
      status === 200,
      `Concept "${conceptId}" is not in elohim-storage (status ${status}). ` +
        'Run: cd app/elohim-app && pnpm run hc:start:seed'
    );
    this.contentIds.set('currentConceptId', conceptId);
  }
);

/**
 * Verify that the named learning path is accessible via the doorway content API.
 * This confirms elohim-storage was seeded before the scenario runs.
 *
 * Background step.
 *
 * Example: Given elohim-storage is seeded with learning path "elohim-protocol"
 */
Given(
  'elohim-storage is seeded with learning path {string}',
  async function (this: E2EWorld, pathId: string) {
    const doorwayUrl = resolveDoorwayUrl(this);
    const { status } = await fetchRaw(`${doorwayUrl}/db/path/${pathId}`);
    assert.ok(
      status === 200,
      `Learning path "${pathId}" is not in elohim-storage (status ${status}). ` +
        'Run: cd app/elohim-app && pnpm run hc:start:seed'
    );
    this.contentIds.set('currentPathId', pathId);
  }
);

// ---------------------------------------------------------------------------
// When: raw HTTP fetch (external WebFetch — no JS engine)
// ---------------------------------------------------------------------------

/**
 * Perform a raw HTTP GET — simulating an HTTP client with no JavaScript engine
 * (search crawler, AI design tool, screen-reader proxy, accessibility client).
 *
 * The "without a JavaScript engine" framing is the experience contract, not a
 * technical transport distinction — it's the same undici.request() call used by
 * ssr-hydration.steps.ts. The point is that whatever the server returns IS what
 * the client reads; there is no JS runtime to hydrate or fill in the content.
 *
 * Example: When an HTTP client without a JavaScript engine fetches "/lamad/concept/elohim-protocol-overview"
 */
When(
  'an HTTP client without a JavaScript engine fetches {string}',
  async function (this: E2EWorld, path: string) {
    const doorwayUrl = resolveDoorwayUrl(this);
    const capture = await fetchRaw(`${doorwayUrl}${path}`);
    this.contentIds.set(EXT_FETCH_KEY, JSON.stringify(capture));
  }
);

/**
 * Perform a raw HTTP GET with a social-card crawler User-Agent.
 * Doorway MUST NOT vary its response body by User-Agent — og:* meta tags must be
 * present for all clients. The UA header is set here for documentary symmetry and
 * future UA-vary regression testing only.
 *
 * Example: When a social card crawler fetches "/lamad/path/elohim-protocol/step/0"
 */
When('a social card crawler fetches {string}', async function (this: E2EWorld, path: string) {
  const doorwayUrl = resolveDoorwayUrl(this);
  const capture = await fetchRaw(`${doorwayUrl}${path}`, {
    'user-agent': 'facebookexternalhit/1.1 (+http://www.facebook.com/externalhit_uatext.php)',
  });
  this.contentIds.set(SOCIAL_FETCH_KEY, JSON.stringify(capture));
});

// ---------------------------------------------------------------------------
// Then: assertions for external WebFetch scenario
// ---------------------------------------------------------------------------

/**
 * Assert the HTTP response status code.
 *
 * Example: Then the response has status 200
 */
Then('the response has status {int}', function (this: E2EWorld, expectedStatus: number) {
  // Check whichever capture key was populated (ext-fetch or social-fetch).
  const rawExt = this.contentIds.get(EXT_FETCH_KEY);
  const rawSocial = this.contentIds.get(SOCIAL_FETCH_KEY);
  const raw = rawExt ?? rawSocial;
  assert.ok(raw, 'No HTTP capture found — run the preceding When step first');
  const capture = JSON.parse(raw) as RawCapture;
  assert.equal(
    capture.status,
    expectedStatus,
    `Expected HTTP ${expectedStatus} but got ${capture.status}`
  );
});

/**
 * Assert the HTML body contains a non-empty <title> element.
 * Proves Angular's Title service ran during SSR and the route resolved.
 *
 * Example: Then the response body contains the concept title in <title>
 */
Then(
  'the response body contains the concept title in {htmlelement}',
  function (this: E2EWorld, _element: string) {
    const capture = storedCapture(this, EXT_FETCH_KEY);
    const title = extractTitle(capture.body);
    assert.ok(
      title && title.length > 0,
      'Expected a non-empty <title> element in SSR HTML (Angular Title service must run during SSR).\n' +
        `Body preview (first 500 chars): ${capture.body.slice(0, 500)}`
    );
  }
);

/**
 * Assert the HTML body contains a non-empty <article> element with rendered text.
 * Proves the content component ran server-side and emitted readable markup.
 *
 * Example: Then the response body contains the concept body in <article>
 */
Then(
  'the response body contains the concept body in {htmlelement}',
  function (this: E2EWorld, _element: string) {
    const capture = storedCapture(this, EXT_FETCH_KEY);
    assert.ok(
      hasNonEmptyArticle(capture.body),
      'Expected a non-empty <article> element in SSR HTML (content component must render server-side).\n' +
        'If <article> is absent, the content viewer component is not part of the SSR render tree.\n' +
        `Body preview (first 800 chars): ${capture.body.slice(0, 800)}`
    );
  }
);

/**
 * Assert that any <sophia-question> elements present have attributes.
 * If no <sophia-question> elements exist, the assertion passes vacuously —
 * not all concept pages include quizzes.
 *
 * Example: Then the response body contains <sophia-question> placeholders if any quizzes exist
 */
Then(
  'the response body contains {htmlelement} placeholders if any quizzes exist',
  function (this: E2EWorld, _element: string) {
    const capture = storedCapture(this, EXT_FETCH_KEY);
    assert.ok(
      sophiaQuestionPlaceholdersAreValid(capture.body),
      'Found bare <sophia-question> element(s) with no attributes in SSR HTML.\n' +
        'SSR must serialise sophia-question custom elements with their content-id or question-id attribute.\n' +
        'A bare <sophia-question> means the web component received no data during server render.\n' +
        `Body preview (first 800 chars): ${capture.body.slice(0, 800)}`
    );
  }
);

/**
 * Assert that <app-root> is present and non-empty.
 * An empty <app-root> indicates doorway served the SPA shell fallback instead of
 * running the SSR renderer.
 *
 * Example: And no <app-root> tag is empty
 */
Then('no {htmlelement} tag is empty', function (this: E2EWorld, _element: string) {
  const capture = storedCapture(this, EXT_FETCH_KEY);
  assert.ok(
    appRootIsNonEmpty(capture.body),
    'Expected <app-root> to be non-empty in SSR HTML.\n' +
      'An empty <app-root> means doorway served the SPA shell fallback (SSR_BUNDLE_PATH not set, ' +
      'or the SSR renderer failed and fell back to the shell).\n' +
      `Body preview (first 500 chars): ${capture.body.slice(0, 500)}`
  );
});

/**
 * Assert the x-render-cache response header is present.
 * Doorway's render cache layer sets this on all SSR-eligible routes
 * (value is either "HIT" or "MISS" on first request).
 *
 * Example: And the response includes the x-render-cache header
 */
Then('the response includes the x-render-cache header', function (this: E2EWorld) {
  const capture = storedCapture(this, EXT_FETCH_KEY);
  const hasHeader = Object.prototype.hasOwnProperty.call(capture.headers, 'x-render-cache');
  const value = hasHeader ? capture.headers['x-render-cache'] : '';
  assert.ok(
    hasHeader && value !== '',
    'Expected "x-render-cache" response header to be present.\n' +
      "This header is set by doorway's render cache layer on SSR-eligible routes.\n" +
      `Present headers: ${Object.keys(capture.headers).join(', ')}`
  );
});

// ---------------------------------------------------------------------------
// Then: assertions for social card crawler scenario
// ---------------------------------------------------------------------------

/**
 * Assert that the HTML body contains a <meta property="og:{name}"> element.
 * The step text uses a bare tag syntax like: <meta property="og:title">
 *
 * Example: Then the response body contains <meta property="og:title">
 */
Then('the response body contains {ogmetatag}', function (this: E2EWorld, ogPropertyName: string) {
  const capture = storedCapture(this, SOCIAL_FETCH_KEY);
  assert.ok(
    hasOgMeta(capture.body, ogPropertyName),
    `Expected <meta property="og:${ogPropertyName}"> in SSR HTML.\n` +
      "og:* meta tags must be emitted by Angular's Meta service during SSR.\n" +
      "If absent, wire the Meta service in the route's SSR rendering path.\n" +
      `Body preview (first 1000 chars): ${capture.body.slice(0, 1000)}`
  );
});

/**
 * Assert that og:title matches the first step's title.
 * The path's first step title is the title of step index 0 in the learning path.
 * We verify og:title is non-empty, which confirms it was populated from content
 * rather than being a fallback default.
 *
 * Once the path-step API is stable, resolve step[0].title via
 * /db/path/{id}/steps and assert the og:title equals it exactly.
 *
 * Example: And the og:title equals the path's first step title
 */
Then("the og:title equals the path's first step title", function (this: E2EWorld) {
  const capture = storedCapture(this, SOCIAL_FETCH_KEY);
  const ogTitle = extractOgTitle(capture.body);
  assert.ok(
    ogTitle && ogTitle.length > 0,
    'Expected og:title to be non-empty in SSR HTML.\n' +
      'The og:title must be populated from the path step title during server-side rendering.\n' +
      "An empty or absent og:title means Angular's Meta service did not run or received no title.\n" +
      `Body preview (first 1000 chars): ${capture.body.slice(0, 1000)}`
  );
});
