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

interface AppResponse {
  status: number;
  headers: Record<string, string>;
  body: Buffer;
}

/** Fetch a URL and capture status + headers for assertions. */
async function fetchApp(baseUrl: string, path: string): Promise<AppResponse> {
  const { statusCode, headers, body } = await request(`${baseUrl}${path}`);
  const data = Buffer.from(await body.arrayBuffer());
  const flatHeaders: Record<string, string> = {};
  for (const [key, value] of Object.entries(headers)) {
    if (typeof value === 'string') flatHeaders[key.toLowerCase()] = value;
    else if (Array.isArray(value)) flatHeaders[key.toLowerCase()] = value[0];
  }
  return { status: statusCode, headers: flatHeaders, body: data };
}

// Store last response for Then assertions
const responseStore = new WeakMap<E2EWorld, AppResponse>();

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
Given(
  'an HTML5 app with slug {string} is seeded',
  async function (this: E2EWorld, slug: string) {
    const doorway = [...this.doorways.values()][0];
    assert.ok(doorway, 'No doorway registered');
    const resp = await fetchApp(doorway.url, `/db/content/${slug}`);
    assert.equal(
      resp.status,
      200,
      `HTML5 app "${slug}" not found in storage (status ${resp.status}). Seed it before running this scenario.`
    );
    // Store slug for later use in body-match assertions
    this.contentIds.set('currentAppSlug', slug);
  }
);

/**
 * Record the expected blob hash for a seeded app so response header assertions can
 * reference it. No network call — this is a scenario-level precondition fixture.
 */
Given('the app\'s blob hash is {string}', function (this: E2EWorld, blobHash: string) {
  this.contentIds.set('currentAppBlobHash', blobHash);
});

/**
 * Verify that a content node exists via the doorway API (used in web2-absorption.feature).
 */
Given(
  'content {string} has been seeded as html5-app',
  async function (this: E2EWorld, contentId: string) {
    const doorway = [...this.doorways.values()][0];
    assert.ok(doorway, 'No doorway registered');
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
  assert.ok(doorway, 'No doorway registered');
  const resp = await fetchApp(doorway.url, path);
  responseStore.set(this, resp);
});

When(
  '{word} requests the root path {string}',
  async function (this: E2EWorld, _humanName: string, path: string) {
    const doorway = [...this.doorways.values()][0];
    assert.ok(doorway, 'No doorway registered');
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
    assert.ok(resp, 'No response captured');
    const actual = resp.headers[headerName.toLowerCase()];
    assert.ok(
      actual,
      `Header "${headerName}" not present. Present headers: ${JSON.stringify(resp.headers)}`
    );
    assert.equal(actual, expectedValue);
  }
);

Then(
  'the response body matches the slug URL response',
  async function (this: E2EWorld) {
    const doorway = [...this.doorways.values()][0];
    assert.ok(doorway, 'No doorway registered');
    const slug = this.contentIds.get('currentAppSlug') ?? 'evolution-of-trust';
    const slugResp = await fetchApp(doorway.url, `/apps/${slug}/index.html`);
    const cidResp = responseStore.get(this);
    assert.ok(cidResp, 'No CID response captured');
    assert.deepEqual(cidResp.body, slugResp.body, 'CID and slug responses differ');
  }
);

Then(
  'the response is HTML containing {string}',
  function (this: E2EWorld, expectedContent: string) {
    const resp = responseStore.get(this);
    assert.ok(resp, 'No response captured');
    const html = resp.body.toString('utf-8');
    assert.ok(
      html.includes(expectedContent),
      `Response body does not contain "${expectedContent}". First 200 chars: ${html.slice(0, 200)}`
    );
  }
);

Then(
  'the response Cache-Control is {string}',
  function (this: E2EWorld, expected: string) {
    const resp = responseStore.get(this);
    assert.ok(resp, 'No response captured');
    const actual = resp.headers['cache-control'] ?? '';
    assert.ok(
      actual.includes(expected),
      `Expected Cache-Control containing "${expected}" but got "${actual}"`
    );
  }
);

Then(
  'the response is a redirect to {string}',
  function (this: E2EWorld, expectedLocation: string) {
    const resp = responseStore.get(this);
    assert.ok(resp, 'No response captured');
    assert.ok(
      resp.status >= 300 && resp.status < 400,
      `Expected redirect status (3xx) but got ${resp.status}`
    );
    const location = resp.headers['location'] ?? '';
    assert.equal(location, expectedLocation);
  }
);

// ---------------------------------------------------------------------------
// Cache layer observation — X-Cache header
// ---------------------------------------------------------------------------

Then(
  'the response includes a header indicating the serving layer',
  function (this: E2EWorld) {
    const resp = responseStore.get(this);
    assert.ok(resp, 'No response captured');
    const cacheHeader = resp.headers['x-cache'];
    assert.ok(
      cacheHeader,
      `X-Cache header not present. Present headers: ${JSON.stringify(resp.headers)}`
    );
  }
);

Then('the serving layer is {string}', function (this: E2EWorld, expectedLayer: string) {
  const resp = responseStore.get(this);
  assert.ok(resp, 'No response captured');
  const cacheHeader = resp.headers['x-cache'];
  // Map friendly layer names to acceptable X-Cache header values
  const layerMap: Record<string, string[]> = {
    'projection-cache': ['HIT'],
    'storage-proxy': ['MISS', 'BYPASS'],
    'storage-extraction': ['MISS'],
  };
  const acceptable = layerMap[expectedLayer] ?? [expectedLayer];
  assert.ok(
    acceptable.includes(cacheHeader ?? ''),
    `Expected serving layer "${expectedLayer}" (${acceptable.join('|')}) but got X-Cache: "${cacheHeader}"`
  );
});

// ---------------------------------------------------------------------------
// Concurrent load — coalescing verification
// ---------------------------------------------------------------------------

When(
  '{int} browsers simultaneously request {string} from {string}',
  async function (this: E2EWorld, count: number, file: string, appSlug: string) {
    const doorway = [...this.doorways.values()][0];
    assert.ok(doorway, 'No doorway registered');
    const path = `/apps/${appSlug}/${file}`;

    // Fire all requests concurrently
    const promises = Array.from({ length: count }, () => fetchApp(doorway.url, path));
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

Then(
  'all {int} browsers receive the same response',
  function (this: E2EWorld, count: number) {
    const responses = (this as unknown as Record<string, unknown>)[
      '__concurrentResponses'
    ] as AppResponse[];
    assert.ok(responses, 'No concurrent responses captured');
    assert.equal(responses.length, count);
    const firstBody = responses[0].body;
    for (let i = 1; i < responses.length; i++) {
      assert.deepEqual(responses[i].body, firstBody, `Response ${i} differs from response 0`);
    }
  }
);

// ---------------------------------------------------------------------------
// Health / startup endpoint
// ---------------------------------------------------------------------------

Then(
  'the startup status shows {string} as ready',
  async function (this: E2EWorld, section: string) {
    const doorway = [...this.doorways.values()][0];
    assert.ok(doorway, 'No doorway registered');
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
    assert.ok(doorway, 'No doorway registered');
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
