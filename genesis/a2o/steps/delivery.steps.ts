/**
 * Delivery API step definitions — content-addressing, cache headers, concurrent load.
 *
 * API-level steps that verify HTTP delivery behavior without a browser:
 * cache headers, content addressing, slug vs CID resolution, coalescing.
 *
 * NOTE: 'doorway {string} at {string}' is already registered in mode-aware.steps.ts
 * and is not duplicated here.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import { E2EWorld } from '../src/framework/world.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const NO_DOORWAY = 'No doorway registered';
const NO_RESPONSE = 'No response captured';

export interface AppResponse {
  status: number;
  headers: Record<string, string>;
  body: Buffer;
}

/** Fetch a URL and capture status + headers for assertions. */
export async function fetchApp(baseUrl: string, path: string): Promise<AppResponse> {
  const { statusCode, headers, body } = await request(`${baseUrl}${path}`);
  const data = Buffer.from(await body.arrayBuffer());
  const flatHeaders: Record<string, string> = {};
  for (const [key, value] of Object.entries(headers)) {
    if (typeof value === 'string') flatHeaders[key.toLowerCase()] = value;
    else if (Array.isArray(value)) flatHeaders[key.toLowerCase()] = value[0];
  }
  return { status: statusCode, headers: flatHeaders, body: data };
}

// Store last response for Then assertions.
// Exported so sibling step files (e.g. steps/delivery/*) can write
// captures that the shared Then assertions in this file resolve.
export const responseStore = new WeakMap<E2EWorld, AppResponse>();

/** Sentinel value used in feature files when the real hash isn't known yet. */
const PLACEHOLDER_HASH = 'sha256-abc123';

/** Pattern that a real sha256 content-address must match. */
const CONTENT_ADDRESS_PATTERN = /^sha256-[a-f0-9]{64}$/;

/** World-local storage key for the captured X-Content-Address value. */
const CAPTURED_CONTENT_ADDRESS_KEY = 'capturedContentAddress';
/** Module-level capture: survives across scenarios (each gets a fresh World). */
let capturedContentAddress: string | undefined;

// ---------------------------------------------------------------------------
// Background: doorway and seeded-content context
// ---------------------------------------------------------------------------

/**
 * Assert that the first registered doorway is reachable and connected to storage.
 * The doorway URL comes from the scenario background's earlier "doorway at" step.
 */
Given('the doorway is connected to elohim-storage', async function (this: E2EWorld) {
  const doorway = [...this.doorways.values()][0];
  assert.ok(doorway, 'No doorway registered — run "doorway ... at ..." step first');
  const resp = await fetchApp(doorway.url, '/health');
  assert.ok(
    resp.status === 200,
    `Doorway health check failed with status ${resp.status} at ${doorway.url}/health`
  );
});

/**
 * Verify that an HTML5 app with the given slug exists in storage via the doorway API.
 */
Given('an HTML5 app with slug {string} is seeded', async function (this: E2EWorld, slug: string) {
  const doorway = [...this.doorways.values()][0];
  assert.ok(doorway, NO_DOORWAY);
  const resp = await fetchApp(doorway.url, `/db/content/${slug}`);
  assert.equal(
    resp.status,
    200,
    `HTML5 app "${slug}" not found in storage (status ${resp.status}). Seed it before running this scenario.`
  );
  // Store slug for later use in body-match assertions
  this.contentIds.set('currentAppSlug', slug);
});

/**
 * Record the expected blob hash for a seeded app so response header assertions can
 * reference it. No network call — this is a scenario-level precondition fixture.
 */
Given("the app's blob hash is {string}", function (this: E2EWorld, blobHash: string) {
  this.contentIds.set('currentAppBlobHash', blobHash);
});

/**
 * Verify that a content node exists via the doorway API (used in web2-absorption.feature).
 */
Given(
  'content {string} has been seeded as html5-app',
  async function (this: E2EWorld, contentId: string) {
    const doorway = [...this.doorways.values()][0];
    assert.ok(doorway, NO_DOORWAY);
    const resp = await fetchApp(doorway.url, `/db/content/${contentId}`);
    assert.equal(
      resp.status,
      200,
      `Content "${contentId}" not found (status ${resp.status}). Seed it before running this scenario.`
    );
    this.contentIds.set('currentAppSlug', contentId);
  }
);

// ---------------------------------------------------------------------------
// Request steps
// ---------------------------------------------------------------------------

When('I request {string}', async function (this: E2EWorld, path: string) {
  const doorway = [...this.doorways.values()][0];
  assert.ok(doorway, NO_DOORWAY);
  // If the path contains the placeholder hash, substitute the real captured hash.
  // This allows the feature file to remain readable with the literal placeholder
  // while the step uses whichever hash the server actually returned.
  let resolvedPath = path;
  if (path.includes(PLACEHOLDER_HASH)) {
    // Cucumber gives every scenario a FRESH World, so a capture made in the
    // slug-URL scenario is invisible here (caused genesis #1080's "no content
    // address was captured yet"). Prefer the module-level capture; if absent,
    // self-capture by fetching the slug URL and reading its X-Content-Address.
    let captured = capturedContentAddress ?? this.contentIds.get(CAPTURED_CONTENT_ADDRESS_KEY);
    if (!captured) {
      const slug = this.contentIds.get('currentAppSlug') ?? 'evolution-of-trust';
      const slugResp = await fetchApp(doorway.url, `/apps/${slug}/index.html`);
      captured = slugResp.headers['x-content-address'];
      assert.ok(
        captured,
        `Cannot resolve "${PLACEHOLDER_HASH}": /apps/${slug}/index.html exposed no X-Content-Address header`
      );
      assert.match(
        captured,
        CONTENT_ADDRESS_PATTERN,
        `Captured "${captured}" does not look like a sha256 content address`
      );
    }
    capturedContentAddress = captured;
    resolvedPath = path.replace(PLACEHOLDER_HASH, captured);
  }
  const resp = await fetchApp(doorway.url, resolvedPath);
  responseStore.set(this, resp);
});

When(
  '{word} requests the root path {string}',
  async function (this: E2EWorld, _humanName: string, path: string) {
    const doorway = [...this.doorways.values()][0];
    assert.ok(doorway, NO_DOORWAY);
    const resp = await fetchApp(doorway.url, path);
    responseStore.set(this, resp);
  }
);

// ---------------------------------------------------------------------------
// Response assertion steps
// ---------------------------------------------------------------------------

Then('the response status is {int}', function (this: E2EWorld, expectedStatus: number) {
  const resp = responseStore.get(this);
  assert.ok(resp, 'No response captured — run a "When I request" step first');
  assert.equal(resp.status, expectedStatus);
});

Then(
  'the response includes header {string} with value {string}',
  function (this: E2EWorld, headerName: string, expectedValue: string) {
    const resp = responseStore.get(this);
    assert.ok(resp, NO_RESPONSE);
    const actual = resp.headers[headerName.toLowerCase()];
    assert.ok(
      actual,
      `Header "${headerName}" not present. Present headers: ${JSON.stringify(resp.headers)}`
    );
    // When the expected value is the well-known placeholder hash, treat it as
    // "capture whatever hash the server returns" rather than a literal equality
    // check.  Validate the pattern and store the real hash for later steps that
    // need to build a CID URL.
    if (expectedValue === PLACEHOLDER_HASH) {
      assert.match(
        actual,
        CONTENT_ADDRESS_PATTERN,
        `Header "${headerName}" value "${actual}" does not look like a sha256 content address`
      );
      this.contentIds.set(CAPTURED_CONTENT_ADDRESS_KEY, actual);
      capturedContentAddress = actual;
    } else {
      assert.equal(actual, expectedValue);
    }
  }
);

Then('the response body matches the slug URL response', async function (this: E2EWorld) {
  const doorway = [...this.doorways.values()][0];
  assert.ok(doorway, NO_DOORWAY);
  const slug = this.contentIds.get('currentAppSlug') ?? 'evolution-of-trust';
  const slugResp = await fetchApp(doorway.url, `/apps/${slug}/index.html`);
  const cidResp = responseStore.get(this);
  assert.ok(cidResp, 'No CID response captured');
  assert.deepEqual(cidResp.body, slugResp.body, 'CID and slug responses differ');
});

Then(
  'the response is HTML containing {string}',
  function (this: E2EWorld, expectedContent: string) {
    const resp = responseStore.get(this);
    assert.ok(resp, NO_RESPONSE);
    const html = resp.body.toString('utf-8');
    assert.ok(
      html.includes(expectedContent),
      `Response body does not contain "${expectedContent}". First 200 chars: ${html.slice(0, 200)}`
    );
  }
);

Then('the response Cache-Control is {string}', function (this: E2EWorld, expected: string) {
  const resp = responseStore.get(this);
  assert.ok(resp, NO_RESPONSE);
  const actual = resp.headers['cache-control'] ?? '';
  assert.ok(
    actual.includes(expected),
    `Expected Cache-Control containing "${expected}" but got "${actual}"`
  );
});

Then('the response is a redirect to {string}', function (this: E2EWorld, expectedLocation: string) {
  const resp = responseStore.get(this);
  assert.ok(resp, NO_RESPONSE);
  assert.ok(
    resp.status >= 300 && resp.status < 400,
    `Expected redirect status (3xx) but got ${resp.status}`
  );
  const location = resp.headers['location'] ?? '';
  assert.equal(location, expectedLocation);
});

// ---------------------------------------------------------------------------
// Cache layer observation — X-Cache header
// ---------------------------------------------------------------------------

Then('the response includes a header indicating the serving layer', function (this: E2EWorld) {
  const resp = responseStore.get(this);
  assert.ok(resp, NO_RESPONSE);
  const cacheHeader = resp.headers['x-cache'];
  assert.ok(
    cacheHeader,
    `X-Cache header not present. Present headers: ${JSON.stringify(resp.headers)}`
  );
});

/**
 * Friendly layer names mapped to the X-Cache values the doorway emits
 * (doorway/doorway-service/src/routes/apps.rs).
 *
 * The sets are DISJOINT on purpose. An earlier revision gave BYPASS-ADMIN to
 * both "storage-proxy" and "storage-extraction", so one response satisfied both
 * names and the assertion stopped distinguishing the two layers it exists to
 * distinguish.
 */
const SERVING_LAYER_MAP: Record<string, string[]> = {
  // The doorway answered out of its own app-file cache. HIT-COALESCED is the
  // same answer for a request that arrived while an identical fetch was already
  // in flight (apps.rs, the coalescing branch): still a cache hit, still zero
  // storage fan-out — so a genuinely coalesced hit must not read as a miss.
  'projection-cache': ['HIT', 'HIT-COALESCED'],
  // The doorway went to elohim-storage for THIS request while its cache layer
  // was still in the path: either the app-file cache was consulted and cold
  // (MISS), or no html5-app/spa-bundle projection carries a blobHash to cache
  // at all (BYPASS).
  'storage-proxy': ['MISS', 'BYPASS'],
  // The doorway's cache layer is deliberately out of the path — POST
  // /admin/cache/disable makes every answer BYPASS-ADMIN (apps.rs) — so the
  // layer actually serving is storage's own extraction cache. This is the only
  // value that means that, which is what keeps the two names apart.
  'storage-extraction': ['BYPASS-ADMIN'],
  // 'blob-decompress' is deliberately UNMAPPED. Once the doorway cache is
  // admin-disabled, every answer reads BYPASS-ADMIN whether storage served from
  // its extraction cache or decompressed the blob; the doorway's X-Cache header
  // cannot express that difference, and storage's own headers would have to.
  // An unmapped name fails naming the X-Cache it saw, which is the honest
  // report — do not map it to BYPASS-ADMIN to buy a green.
};

/**
 * What an unexpected X-Cache value means, so a red names the mechanism instead of
 * two opaque strings. BYPASS in particular is not "the cache was cold" — it is
 * "the app-file cache was never consulted".
 */
function servingLayerHint(actual: string): string {
  if (actual === 'BYPASS') {
    return (
      ' — BYPASS means the doorway never engaged its app-file cache at all: ' +
      'resolve_blob_hash() found no html5-app/spa-bundle projection carrying a non-empty ' +
      'blobHash for this slug (doorway/doorway-service/src/cache/app_file_cache.rs), so the ' +
      'request was proxied straight through. Warming cannot fix that; the projection has to ' +
      'exist first.'
    );
  }
  if (actual === 'BYPASS-ADMIN') {
    return ' — BYPASS-ADMIN means the projection cache is still admin-disabled (POST /admin/cache/enable).';
  }
  if (actual === 'HIT-COALESCED') {
    return (
      ' — HIT-COALESCED is a cache HIT served to a request that arrived while an identical ' +
      'fetch was already in flight; storage saw no extra load.'
    );
  }
  if (actual === 'MISS') {
    return ' — MISS means the cache was consulted but cold for this file; it is warm only after one request per file.';
  }
  if (!actual) {
    return ' — no X-Cache header at all: this response did not come from the doorway /apps/ route.';
  }
  return '';
}

/** Assert a captured response was answered by the named delivery layer. */
export function assertServingLayer(resp: AppResponse, expectedLayer: string): void {
  const cacheHeader = resp.headers['x-cache'] ?? '';
  const acceptable = SERVING_LAYER_MAP[expectedLayer] ?? [expectedLayer];
  assert.ok(
    acceptable.includes(cacheHeader),
    `Expected serving layer "${expectedLayer}" (${acceptable.join('|')}) but got X-Cache: ` +
      `"${cacheHeader}"${servingLayerHint(cacheHeader)}`
  );
}

Then('the serving layer is {string}', function (this: E2EWorld, expectedLayer: string) {
  const resp = responseStore.get(this);
  assert.ok(resp, NO_RESPONSE);
  assertServingLayer(resp, expectedLayer);
});

// ---------------------------------------------------------------------------
// Concurrent load — coalescing verification
// ---------------------------------------------------------------------------

When(
  '{int} browsers simultaneously request {string} from {string}',
  async function (this: E2EWorld, count: number, file: string, appSlug: string) {
    const doorway = [...this.doorways.values()][0];
    assert.ok(doorway, NO_DOORWAY);
    const path = `/apps/${appSlug}/${file}`;

    // Fire all requests concurrently
    const promises = Array.from({ length: count }, async () => fetchApp(doorway.url, path));
    const responses = await Promise.all(promises);

    // Store results for subsequent assertions
    (this as unknown as Record<string, unknown>)['__concurrentResponses'] = responses;
  }
);

Then(
  'only {int} request is proxied to elohim-storage',
  function (this: E2EWorld, expectedCount: number) {
    const responses = (this as unknown as Record<string, unknown>)[
      '__concurrentResponses'
    ] as AppResponse[];
    assert.ok(responses, 'No concurrent responses captured — run a concurrent request step first');
    // MISS responses = proxied to storage; HIT / HIT-COALESCED were served from cache
    const missCount = responses.filter(r => r.headers['x-cache'] === 'MISS').length;
    assert.ok(
      missCount <= expectedCount,
      `Expected at most ${expectedCount} storage proxy request(s), but ${missCount} were MISS`
    );
  }
);

Then('all {int} browsers receive the same response', function (this: E2EWorld, count: number) {
  const responses = (this as unknown as Record<string, unknown>)[
    '__concurrentResponses'
  ] as AppResponse[];
  assert.ok(responses, 'No concurrent responses captured');
  assert.equal(responses.length, count);
  const firstBody = responses[0].body;
  for (let i = 1; i < responses.length; i++) {
    assert.deepEqual(responses[i].body, firstBody, `Response ${i} differs from response 0`);
  }
});

// ---------------------------------------------------------------------------
// Health / startup endpoint
// ---------------------------------------------------------------------------

Then(
  'the startup status shows {string} as ready',
  async function (this: E2EWorld, section: string) {
    const doorway = [...this.doorways.values()][0];
    assert.ok(doorway, NO_DOORWAY);
    const resp = await fetchApp(doorway.url, '/health/startup');
    assert.equal(resp.status, 200, `/health/startup returned ${resp.status}`);
    const data = JSON.parse(resp.body.toString('utf-8')) as Record<
      string,
      { ready?: boolean } | undefined
    >;
    assert.ok(
      data[section]?.ready,
      `Startup section "${section}" is not ready: ${JSON.stringify(data[section])}`
    );
  }
);

Then(
  'the startup status includes rootApp.slug {string}',
  async function (this: E2EWorld, expectedSlug: string) {
    const doorway = [...this.doorways.values()][0];
    assert.ok(doorway, NO_DOORWAY);
    const resp = await fetchApp(doorway.url, '/health/startup');
    assert.equal(resp.status, 200, `/health/startup returned ${resp.status}`);
    const data = JSON.parse(resp.body.toString('utf-8')) as Record<
      string,
      { slug?: string } | undefined
    >;
    assert.equal(
      data['rootApp']?.slug,
      expectedSlug,
      `rootApp.slug mismatch: expected "${expectedSlug}" but got "${data['rootApp']?.slug}"`
    );
  }
);

// ---------------------------------------------------------------------------
// App-wide load: "N browsers simultaneously load <app>"
//
// A browser loading an HTML5 app is not one request — it is the entry document
// plus every relative asset the document references (30+ for evolution-of-trust).
// That fan-out is the whole point of the projection cache, so the load steps
// below reproduce it honestly instead of fetching index.html alone.
//
// Layer evidence comes from the doorway's X-Cache header, whose vocabulary is
// fixed by doorway/doorway-service/src/routes/apps.rs:
//   HIT / HIT-COALESCED — the doorway answered; storage was not touched
//   MISS / BYPASS / BYPASS-ADMIN — the request was proxied to elohim-storage
//
// REQUEST BUDGET — /apps/{slug}/{file} is NOT static-asset-exempt from the
// doorway membrane (doorway/doorway-service/src/server/membrane.rs), which
// shapes at 300 requests / 60s per client IP, challenges (429) at 600 and BANS
// for 900s at 1200. A whole suite shares one source IP. A full-bundle pass here
// is ~40 requests; a 10-browser fan-out is ~400 and is therefore fenced behind
// @wip in the feature file, to be run deliberately against a dedicated doorway.
// ---------------------------------------------------------------------------

/** Entry document every HTML5 app bundle exposes. */
const APP_ENTRY_FILE = 'index.html';

/** Default app slug when a scenario asserts before naming one. */
const DEFAULT_APP_SLUG = 'evolution-of-trust';

/**
 * X-Cache values that mean the doorway answered without touching storage.
 * Derived from SERVING_LAYER_MAP rather than restated, so the two cannot
 * disagree about what a cache hit is (they did: this set knew HIT-COALESCED
 * was a hit while the layer map did not).
 */
const CACHE_SERVED = new Set(SERVING_LAYER_MAP['projection-cache']);

/**
 * X-Cache values that mean the request was proxied through to elohim-storage.
 * Both proxying layers count here — this set answers "did storage take the
 * load", not "which layer served it", so admin-bypassed traffic belongs in it.
 */
const STORAGE_PROXIED = new Set([
  ...SERVING_LAYER_MAP['storage-proxy'],
  ...SERVING_LAYER_MAP['storage-extraction'],
]);

/** One file request and the response it got, kept paired so failures can name the file. */
interface AppFileResponse {
  file: string;
  resp: AppResponse;
}

/** Capture of one "N browsers load <app>" fan-out. */
interface AppLoadCapture {
  slug: string;
  /** Files each simulated browser requested (entry document first). */
  files: string[];
  /** One entry per simulated browser. */
  browsers: AppFileResponse[][];
}

const loadStore = new WeakMap<E2EWorld, AppLoadCapture>();

const NO_LOAD_CAPTURED =
  'No app load captured — run a "N browsers simultaneously load ..." step first';

/** The X-Cache header of a response, or '' when the doorway sent none. */
function cacheHeaderOf(entry: AppFileResponse): string {
  return entry.resp.headers['x-cache'] ?? '';
}

/** Flatten a load capture back into one list of file/response pairs. */
function allEntries(capture: AppLoadCapture): AppFileResponse[] {
  return capture.browsers.flat();
}

/** Compact "<file> -> <status> X-Cache:<value>" lines for assertion messages. */
function describeEntries(entries: AppFileResponse[], limit = 5): string {
  return entries
    .slice(0, limit)
    .map(e => `  ${e.file} -> ${e.resp.status} X-Cache:${cacheHeaderOf(e) || '<none>'}`)
    .join('\n');
}

/** Fetch every named file of an app once, keeping the file name with the response. */
async function fetchAppFiles(
  baseUrl: string,
  slug: string,
  files: string[]
): Promise<AppFileResponse[]> {
  return Promise.all(
    files.map(async file => ({ file, resp: await fetchApp(baseUrl, `/apps/${slug}/${file}`) }))
  );
}

/** The app slug the scenario is currently working with. */
function currentSlug(world: E2EWorld): string {
  return world.contentIds.get('currentAppSlug') ?? DEFAULT_APP_SLUG;
}

/**
 * The relative asset paths an app's entry document references — the file set a
 * real browser would fetch after parsing index.html. Absolute URLs, fragments
 * and data: URIs are third-party or in-document and never hit the doorway.
 */
export async function appAssetPaths(baseUrl: string, slug: string): Promise<string[]> {
  const entry = await fetchApp(baseUrl, `/apps/${slug}/${APP_ENTRY_FILE}`);
  assert.equal(
    entry.status,
    200,
    `Cannot enumerate "${slug}" assets: GET /apps/${slug}/${APP_ENTRY_FILE} returned ${entry.status}`
  );
  const html = entry.body.toString('utf-8');
  const refs = new Set<string>();
  for (const match of html.matchAll(/(?:src|href)="([^"]+)"/g)) {
    const ref = match[1];
    if (!ref || ref.startsWith('#') || ref.startsWith('/') || ref.includes('://')) continue;
    if (ref.startsWith('data:') || ref.startsWith('mailto:')) continue;
    // Cache-busting query strings ("css/slides.css?v11") are not part of the
    // stored file path — the bundle keys on the path alone.
    refs.add(ref.split('?')[0]);
  }
  refs.delete(APP_ENTRY_FILE);
  return [...refs];
}

/** Entry document + every relative asset it references. */
export async function appFileSet(baseUrl: string, slug: string): Promise<string[]> {
  return [APP_ENTRY_FILE, ...(await appAssetPaths(baseUrl, slug))];
}

When(
  '{int} browsers simultaneously load {string}',
  { timeout: 180_000 },
  async function (this: E2EWorld, count: number, appSlug: string) {
    const doorway = [...this.doorways.values()][0];
    assert.ok(doorway, NO_DOORWAY);

    const files = await appFileSet(doorway.url, appSlug);
    const browsers = await Promise.all(
      Array.from({ length: count }, async () => fetchAppFiles(doorway.url, appSlug, files))
    );

    this.contentIds.set('currentAppSlug', appSlug);
    loadStore.set(this, { slug: appSlug, files, browsers });
  }
);

Then('all requests proxy directly to elohim-storage', function (this: E2EWorld) {
  const capture = loadStore.get(this);
  assert.ok(capture, NO_LOAD_CAPTURED);
  const entries = allEntries(capture);
  assert.ok(entries.length > 0, 'The load captured zero requests');
  const cached = entries.filter(e => CACHE_SERVED.has(cacheHeaderOf(e)));
  assert.equal(
    cached.length,
    0,
    `${cached.length}/${entries.length} requests were answered from the doorway cache; ` +
      `expected every request to proxy to elohim-storage.\n${describeEntries(cached)}`
  );
});

Then(
  'elohim-storage receives {int}+ concurrent requests',
  function (this: E2EWorld, minimum: number) {
    const capture = loadStore.get(this);
    assert.ok(capture, NO_LOAD_CAPTURED);
    const proxied = allEntries(capture).filter(e => STORAGE_PROXIED.has(cacheHeaderOf(e)));
    assert.ok(
      proxied.length >= minimum,
      `Only ${proxied.length} of ${capture.browsers.length} browsers x ${capture.files.length} ` +
        `files reached elohim-storage; the scenario needs at least ${minimum}`
    );
  }
);

Then('zero requests reach elohim-storage', function (this: E2EWorld) {
  const capture = loadStore.get(this);
  assert.ok(capture, NO_LOAD_CAPTURED);
  const entries = allEntries(capture);
  assert.ok(entries.length > 0, 'The load captured zero requests');
  const proxied = entries.filter(e => !CACHE_SERVED.has(cacheHeaderOf(e)));
  assert.equal(
    proxied.length,
    0,
    `${proxied.length}/${entries.length} requests reached elohim-storage instead of being ` +
      `absorbed by the projection cache.\n${describeEntries(proxied)}`
  );
});

Then('all browsers receive complete responses', function (this: E2EWorld) {
  const capture = loadStore.get(this);
  assert.ok(capture, NO_LOAD_CAPTURED);
  capture.browsers.forEach((entries, browser) => {
    assert.equal(
      entries.length,
      capture.files.length,
      `Browser ${browser} received ${entries.length} of ${capture.files.length} files`
    );
    for (const entry of entries) {
      assert.ok(
        entry.resp.status >= 200 && entry.resp.status < 300,
        `Browser ${browser}: ${entry.file} returned ${entry.resp.status}`
      );
      assert.ok(entry.resp.body.length > 0, `Browser ${browser}: ${entry.file} was empty`);
    }
  });
});

// ---------------------------------------------------------------------------
// Single-file requests and per-layer assertions
// ---------------------------------------------------------------------------

/**
 * Request one file from an app so the serving-layer assertions have a response.
 * The entry document is used because it is the one file every app bundle has.
 *
 * Example: When Matthew requests a file from "evolution-of-trust"
 */
When(
  '{word} requests a file from {string}',
  async function (this: E2EWorld, _humanName: string, appSlug: string) {
    const doorway = [...this.doorways.values()][0];
    assert.ok(doorway, NO_DOORWAY);
    this.contentIds.set('currentAppSlug', appSlug);
    responseStore.set(this, await fetchApp(doorway.url, `/apps/${appSlug}/${APP_ENTRY_FILE}`));
  }
);

/**
 * Re-request the app entry document and assert which layer answered it.
 * "returns to" is the same claim as "is", made after the layers were restored,
 * so it takes a fresh measurement rather than re-reading a stale response.
 *
 * Example: And the serving layer returns to "projection-cache"
 */
Then(
  'the serving layer returns to {string}',
  async function (this: E2EWorld, expectedLayer: string) {
    const doorway = [...this.doorways.values()][0];
    assert.ok(doorway, NO_DOORWAY);
    const slug = currentSlug(this);
    const resp = await fetchApp(doorway.url, `/apps/${slug}/${APP_ENTRY_FILE}`);
    responseStore.set(this, resp);
    assertServingLayer(resp, expectedLayer);
  }
);

// ---------------------------------------------------------------------------
// Projection-cache assertions over the whole app file set (web2-absorption)
// ---------------------------------------------------------------------------

/**
 * Every file of the bundle must come back from the doorway cache. The evidence
 * is per-file X-Cache, which only the HTTP layer carries — a Playwright load
 * captures console and failed requests, not response headers — so this step
 * makes its own file-by-file pass over the same bundle the scenario just loaded.
 *
 * Example: Then all app files are served from the projection cache
 */
Then(
  'all app files are served from the projection cache',
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const doorway = [...this.doorways.values()][0];
    assert.ok(doorway, NO_DOORWAY);
    const slug = currentSlug(this);
    const files = await appFileSet(doorway.url, slug);
    const entries = await fetchAppFiles(doorway.url, slug, files);
    loadStore.set(this, { slug, files, browsers: [entries] });

    const proxied = entries.filter(e => !CACHE_SERVED.has(cacheHeaderOf(e)));
    assert.equal(
      proxied.length,
      0,
      `${proxied.length}/${entries.length} app files were not served by the projection ` +
        `cache.\n${describeEntries(proxied)}`
    );
  }
);

/**
 * Every cached app file must carry the blob hash elohim-storage advertises for
 * the bundle. The capability probe (HEAD /apps/{slug}/_capability, X-Blob-Hash)
 * is storage's answer; each served file echoes it as X-Content-Address. Drift
 * between the two means a cache entry is pinned to a blob the bundle no longer is.
 *
 * Example: And each cache entry records the blob_hash from storage
 */
Then(
  'each cache entry records the blob_hash from storage',
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const doorway = [...this.doorways.values()][0];
    assert.ok(doorway, NO_DOORWAY);
    const slug = currentSlug(this);

    const { statusCode, headers } = await request(`${doorway.url}/apps/${slug}/_capability`, {
      method: 'HEAD',
    });
    assert.equal(statusCode, 200, `HEAD /apps/${slug}/_capability returned ${statusCode}`);
    const rawBlobHash = headers['x-blob-hash'];
    const blobHash = Array.isArray(rawBlobHash) ? rawBlobHash[0] : rawBlobHash;
    assert.ok(blobHash, `Storage advertises no X-Blob-Hash for "${slug}"`);

    const files = await appFileSet(doorway.url, slug);
    const entries = await fetchAppFiles(doorway.url, slug, files);
    for (const entry of entries) {
      assert.equal(entry.resp.status, 200, `GET ${entry.file} returned ${entry.resp.status}`);
      assert.equal(
        entry.resp.headers['x-content-address'],
        blobHash,
        `Cache entry for "${entry.file}" records X-Content-Address ` +
          `"${entry.resp.headers['x-content-address']}" but storage advertises "${blobHash}"`
      );
    }
  }
);

// ---------------------------------------------------------------------------
// Coalescing: "N browsers simultaneously request the same file"
// ---------------------------------------------------------------------------

/**
 * Resolve a file reference against an app bundle. Feature files name assets the
 * way a reader does ("pixi.min.js"); the bundle stores them under their real
 * path ("js/lib/pixi.min.js").
 *
 * Deliberately does NOT probe the target file: the coalescing scenario needs the
 * projection cache still cold for it when the burst starts, and a probe would
 * warm it and make "only 1 request is proxied" pass vacuously. Enumerating
 * assets touches the entry document only.
 */
async function resolveAppFile(baseUrl: string, slug: string, file: string): Promise<string> {
  if (file.includes('/')) return file;
  const assets = await appAssetPaths(baseUrl, slug);
  const match = assets.find(a => a.split('/').pop() === file);
  assert.ok(
    match,
    `File "${file}" is not referenced by /apps/${slug}/${APP_ENTRY_FILE}, so its bundle path ` +
      `cannot be resolved. Name it with its bundle path in the feature file.`
  );
  return match;
}

When(
  '{int} browsers simultaneously request the same file {string} from {string}',
  { timeout: 120_000 },
  async function (this: E2EWorld, count: number, file: string, appSlug: string) {
    const doorway = [...this.doorways.values()][0];
    assert.ok(doorway, NO_DOORWAY);
    const path = `/apps/${appSlug}/${await resolveAppFile(doorway.url, appSlug, file)}`;
    const responses = await Promise.all(
      Array.from({ length: count }, async () => fetchApp(doorway.url, path))
    );
    this.contentIds.set('currentAppSlug', appSlug);
    (this as unknown as Record<string, unknown>)['__concurrentResponses'] = responses;
  }
);
