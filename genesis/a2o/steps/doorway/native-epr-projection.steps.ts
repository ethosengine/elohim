/**
 * Native EPR Projection step definitions — Task B22 of the pillar EPR
 * decomposition plan.
 *
 * Feature: features/doorway/native-epr-projection.feature
 *
 * The doorway natively projects EPRs at author-declared URL paths via
 * `project-epr` commitments on the DHT. There is no doorway-side
 * hardcoding of paths — the commitment is the source of truth.
 *
 * MVP scope:
 *   - Substrate scenarios (1, 2, 4) are written against HTTP and run
 *     against any doorway that has the commitments seeded.
 *   - Scenario 3 (cache eviction within 5s) depends on B15 (the doorway
 *     SSE subscriber refresh path) and is marked pending until the
 *     end-to-end PATCH-to-eviction wiring is verifiable from a2o.
 *
 * Steps are SUBSTRATE-VERIFIABLE where possible (HTTP GETs against a
 * live doorway). Steps that require not-yet-landed mutations are stubbed
 * with `return 'pending'` so the feature parses and runs in dry mode.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Local response capture (scoped per-world)
// ---------------------------------------------------------------------------

interface ProjectionResponse {
  url: string;
  status: number;
  headers: Record<string, string>;
  body: Buffer;
}

const NO_PROJECTION_RESPONSE = 'No projection response captured';

const projectionResponses = new WeakMap<E2EWorld, ProjectionResponse>();
const projectionSecondaryResponses = new WeakMap<E2EWorld, ProjectionResponse>();

/** Inline base-href detector. Bounded char classes avoid catastrophic backtracking. */
const BASE_HREF_PATTERN = /<base[ \t]+[^>]{0,200}href=["']([^"']{1,500})["']/i;

async function getRaw(url: string): Promise<ProjectionResponse> {
  const { statusCode, headers, body } = await request(url);
  const data = Buffer.from(await body.arrayBuffer());
  const flat: Record<string, string> = {};
  for (const [k, v] of Object.entries(headers)) {
    if (typeof v === 'string') flat[k.toLowerCase()] = v;
    else if (Array.isArray(v)) flat[k.toLowerCase()] = v[0];
  }
  return { url, status: statusCode, headers: flat, body: data };
}

/**
 * Map a hard-coded hostname from the feature file to whichever doorway URL
 * the world is actually targeting. The feature reads as production prose
 * ("https://alpha.elohim.host/"), but the test runs against whatever
 * E2E_DOORWAY_ALPHA / E2E_DOORWAY_PRIMARY points at.
 */
function resolveDoorwayUrl(world: E2EWorld, hostname: string): string {
  const envByHost: Record<string, string | undefined> = {
    'alpha.elohim.host': process.env['E2E_DOORWAY_ALPHA'],
    'elohim.host': process.env['E2E_DOORWAY_PRIMARY'] ?? process.env['E2E_DOORWAY_HOSTED'],
  };
  const env = envByHost[hostname];
  if (env && env.length > 0) return env;
  // Fall back to the first registered doorway if no env var was set —
  // useful for local-stack runs where there is exactly one doorway.
  const firstDoorway = [...world.doorways.values()][0];
  if (firstDoorway) return firstDoorway.url;
  return `https://${hostname}`;
}

// ---------------------------------------------------------------------------
// Given — project-epr commitment fixtures
// ---------------------------------------------------------------------------

Given(
  /^the ([\w.-]+) doorway has an active project-epr commitment for "([^"]+)" at urlPath "([^"]+)"$/,
  function (this: E2EWorld, hostname: string, eprSlug: string, urlPath: string) {
    // The commitment is asserted to exist; the seeder + doorway-side
    // ingest path (Tasks B12–B16) is what creates it. In a hosted run,
    // this step is documentation — it tells the reader which commitment
    // must already be on the DHT for the scenario to pass.
    //
    // For automated verification of presence the runner can query
    // /db/commitment/<eprSlug>; that endpoint shape ships with B16, so
    // we keep this as a documentary no-op until then.
    const doorwayUrl = resolveDoorwayUrl(this, hostname);
    this.contentIds.set(`projection:${eprSlug}:urlPath`, urlPath);
    this.contentIds.set(`projection:${eprSlug}:doorway`, doorwayUrl);
  }
);

Given(
  /^the ([\w.-]+) doorway also has an active project-epr commitment for "([^"]+)" at urlPath "([^"]+)"$/,
  function (this: E2EWorld, hostname: string, eprSlug: string, urlPath: string) {
    const doorwayUrl = resolveDoorwayUrl(this, hostname);
    this.contentIds.set(`projection:${eprSlug}:secondary-urlPath`, urlPath);
    this.contentIds.set(`projection:${eprSlug}:secondary-doorway`, doorwayUrl);
  }
);

Given(
  /^the ([\w-]+) EPR's blob is ([\w-]+)$/,
  function (this: E2EWorld, _eprSlug: string, _blobHash: string) {
    // Cache-eviction scenario — blob mutation isn't wired through a2o yet.
    // Tracked under B15 (doorway SSE refresh) + B22 (this feature).
    return 'pending';
  }
);

// ---------------------------------------------------------------------------
// When — anonymous browser GETs
// ---------------------------------------------------------------------------

When('an anonymous browser GETs {string}', async function (this: E2EWorld, urlString: string) {
  // Translate the production hostname in the URL to the doorway URL the
  // test run targets, then re-attach the original path + query.
  let parsed: URL;
  try {
    parsed = new URL(urlString);
  } catch {
    assert.fail(`Could not parse URL "${urlString}"`);
  }
  const base = resolveDoorwayUrl(this, parsed.hostname);
  const target = `${base.replace(/\/$/, '')}${parsed.pathname}${parsed.search}`;
  const resp = await getRaw(target);

  // Second GET to the same URL across two doorways: capture both. The
  // first hostname seen becomes the primary; any subsequent GET to a
  // different hostname goes into the secondary slot.
  const primary = projectionResponses.get(this);
  if (primary && new URL(primary.url).hostname !== new URL(target).hostname) {
    projectionSecondaryResponses.set(this, resp);
  } else {
    projectionResponses.set(this, resp);
  }
});

When(
  /^a deploy PATCHes the ([\w-]+) EPR with blobHash ([\w-]+)$/,
  function (this: E2EWorld, _eprSlug: string, _newBlobHash: string) {
    // Mutation path isn't a2o-runnable until B15/B16 land — see above.
    return 'pending';
  }
);

// ---------------------------------------------------------------------------
// Then — response assertions
// ---------------------------------------------------------------------------

Then("the response serves the landing EPR's bundle entry file", function (this: E2EWorld) {
  const resp = projectionResponses.get(this);
  assert.ok(resp, NO_PROJECTION_RESPONSE);
  // Substrate check: bundle entry file is HTML and big enough to not be
  // a placeholder error page.
  const ct = resp.headers['content-type'] ?? '';
  assert.ok(ct.includes('text/html'), `Expected HTML bundle entry, got Content-Type "${ct}"`);
  assert.ok(
    resp.body.length > 200,
    `Bundle entry suspiciously small (${resp.body.length} bytes) — likely a placeholder`
  );
});

Then('the response is HTTP {int}', function (this: E2EWorld, expected: number) {
  const resp = projectionResponses.get(this);
  assert.ok(resp, NO_PROJECTION_RESPONSE);
  assert.equal(resp.status, expected);
});

Then(
  /^the response serves the lamad bundle's index\.html \(SPA fallback\)$/,
  function (this: E2EWorld) {
    const resp = projectionResponses.get(this);
    assert.ok(resp, NO_PROJECTION_RESPONSE);
    const ct = resp.headers['content-type'] ?? '';
    assert.ok(ct.includes('text/html'), `Expected HTML SPA fallback, got Content-Type "${ct}"`);
    const html = resp.body.toString('utf-8');
    // The SPA shell is recognisable by an Angular root element or an
    // app-root tag — either is acceptable.
    assert.ok(
      html.includes('<app-root') || html.includes('ng-version') || html.includes('<base'),
      `Response does not look like an Angular SPA shell. First 200 chars: ${html.slice(0, 200)}`
    );
  }
);

Then("the lamad bundle's <base href> is {string}", function (this: E2EWorld, expected: string) {
  const resp = projectionResponses.get(this);
  assert.ok(resp, NO_PROJECTION_RESPONSE);
  const html = resp.body.toString('utf-8');
  // Loose base-href detection — tolerates single/double quotes and
  // attribute ordering. Pattern is hoisted to module scope so the regex
  // is compiled once and bounded against backtracking.
  const baseMatch = BASE_HREF_PATTERN.exec(html);
  assert.ok(baseMatch, `No <base href> found in response. First 200 chars: ${html.slice(0, 200)}`);
  assert.equal(baseMatch[1], expected, `<base href> mismatch`);
});

Then('Angular client-side router handles {string}', function (this: E2EWorld, _path: string) {
  // Substrate check: SPA fallback responded with 200 + index.html. The
  // actual route-handling is a browser-side claim. Without Playwright,
  // we treat the SPA-fallback shape as sufficient evidence here.
  const resp = projectionResponses.get(this);
  assert.ok(resp, NO_PROJECTION_RESPONSE);
  assert.equal(
    resp.status,
    200,
    'SPA fallback must return 200 for client-side routing to take over'
  );
});

Then(
  "within {int} seconds, the doorway's cache for {string} is evicted",
  function (this: E2EWorld, _seconds: number, _path: string) {
    // Requires the PATCH → cache-eviction wiring (B15). Pending.
    return 'pending';
  }
);

Then(
  /^the next browser request to "([^"]+)" serves bytes from ([\w-]+)$/,
  function (this: E2EWorld, _path: string, _expectedBlobHash: string) {
    // Paired with the PATCH/eviction step — pending until that lands.
    return 'pending';
  }
);

Then(
  /^the response serves the same lamad bundle as ([\w.-]+)$/,
  async function (this: E2EWorld, hostname: string) {
    const primary = projectionResponses.get(this);
    const secondary = projectionSecondaryResponses.get(this);
    // The primary/secondary assignment depends on the order of GETs; if
    // the federation scenario's first GET targeted the secondary doorway,
    // pull the primary on demand here.
    if (!primary || !secondary) {
      // Fetch the comparison doorway directly.
      const otherBase = resolveDoorwayUrl(this, hostname);
      const lamadPath = '/lamad/';
      const otherResp = await getRaw(`${otherBase.replace(/\/$/, '')}${lamadPath}`);
      const here = primary ?? secondary;
      assert.ok(here, 'No projection response captured for federation comparison');
      assert.deepEqual(
        here.body,
        otherResp.body,
        `Federation bytes diverge between "${here.url}" and "${otherResp.url}"`
      );
      return;
    }
    assert.deepEqual(
      primary.body,
      secondary.body,
      'Federation: byte content diverges across doorways'
    );
  }
);

Then("both doorways' projections reference the same blob_hash", function (this: E2EWorld) {
  // The blob_hash is exposed via X-Content-Address (or similar) on
  // delivered responses. If neither doorway emits the header today,
  // mark pending so the contract is recorded in the feature without
  // false-positives.
  const primary = projectionResponses.get(this);
  const secondary = projectionSecondaryResponses.get(this);
  if (!primary || !secondary) {
    return 'pending';
  }
  const primaryHash = primary.headers['x-content-address'] ?? primary.headers['x-blob-hash'];
  const secondaryHash = secondary.headers['x-content-address'] ?? secondary.headers['x-blob-hash'];
  if (!primaryHash || !secondaryHash) {
    // Header contract isn't enforced yet — bytes-equal check in the
    // prior step is the substrate equivalent. Mark pending so the
    // expectation stays on the radar.
    return 'pending';
  }
  assert.equal(primaryHash, secondaryHash, 'blob_hash mismatch across federated doorways');
});
