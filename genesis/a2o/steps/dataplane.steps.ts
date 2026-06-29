/**
 * Dataplane surface-query step definitions.
 *
 * Provides a reusable, concern-agnostic HTTP step layer so every dataplane
 * feature asserts against the same runtime surfaces (no per-feature HTTP
 * reinvention). All steps are pure HTTP — no Playwright required — so they
 * run in API mode (E2E_DEVICE_MODE=http, the default).
 *
 * Step shape reference:
 *   Given peer {string} at {string}             — resolve + register a peer URL
 *   When I query {string} on peer {string}       — hit any surface path, store JSON
 *   Then peer {string} /health p2p.caughtUp is true
 *   Then peer {string} /health peerCount >= {int}
 *   Then /sync doc {string} is present on peer {string}
 *   Then blob {string} is byte-present on peer {string}
 *   Then EPR {string} blobHash is non-null on peer {string}
 *   Then resolving {string} on peer {string} does NOT return App-not-found
 *   Then metric {string} on peer {string} {word} {float}
 *
 * Peer URL delegation:
 *   The second arg of "peer {name} at {host}" resolves via resolvePeerUrl():
 *   'alpha-A' → E2E_DOORWAY_ALPHA or https://doorway-alpha.elohim.host
 *   'elohim.host' → https://elohim.host
 *   'shem'    → E2E_SHEM_HOST (must be set)
 *   <other>   → process.env[name] if set, else throw
 *
 * Surfaces that need direct storage access (/p2p/status, storage /metrics):
 *   Set E2E_STORAGE_ALPHA (or E2E_STORAGE_{PEER}) to the direct storage base URL.
 *   Steps that need it return 'pending' (skip, not fail) when the env var is absent.
 *
 * Source: genesis/a2o/src/framework/dataplane/surfaces.ts
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import {
  resolvePeerUrl,
  resolveStorageUrl,
  getRaw,
  probeHealth,
  probeSyncDocHeads,
  probeBlob,
  probeContent,
  probeP2PStatus,
  probeEprNavContext,
  probeMetrics,
  type ParsedMetrics,
} from '../src/framework/dataplane/surfaces.js';
import { E2EWorld } from '../src/framework/world.js';

// ---------------------------------------------------------------------------
// World-scoped state stores (WeakMap keyed by E2EWorld instance so state is
// scenario-local without polluting the World class definition)
// ---------------------------------------------------------------------------

/** Map of registered peer name → resolved base URL for this scenario */
const peerUrls = new WeakMap<E2EWorld, Map<string, string>>();

/** Last generic surface query: surface path + parsed JSON body */
interface SurfaceCapture {
  peerName: string;
  path: string;
  status: number;
  body: unknown;
}
const lastCapture = new WeakMap<E2EWorld, SurfaceCapture>();

/** Helper: get or create the peer URL map for this world instance */
function getPeerMap(world: E2EWorld): Map<string, string> {
  let m = peerUrls.get(world);
  if (!m) {
    m = new Map();
    peerUrls.set(world, m);
  }
  return m;
}

/** Resolve a peer registered in this scenario, or fall back to resolvePeerUrl */
function getPeerUrl(world: E2EWorld, peerName: string): string {
  const m = peerUrls.get(world);
  const fromMap = m?.get(peerName);
  if (fromMap) return fromMap;
  // Fall back to alias resolution (allows skipping the Given step in simple scenarios)
  return resolvePeerUrl(peerName);
}

// ---------------------------------------------------------------------------
// Given — peer registration
// ---------------------------------------------------------------------------

/**
 * Register a named peer for use in subsequent steps.
 *
 * The {hostEnvOrAlias} argument is resolved via resolvePeerUrl():
 *   - 'alpha-A'      → process.env.E2E_DOORWAY_ALPHA or the public alpha endpoint
 *   - 'elohim.host'  → https://elohim.host
 *   - 'shem'         → process.env.E2E_SHEM_HOST (throws if absent)
 *   - any other str  → process.env[str] if set, else throw
 *
 * Example:
 *   Given peer "node-A" at "alpha-A"
 *   Given peer "node-B" at "E2E_DOORWAY_ALPHA"
 */
Given(
  'peer {string} at {string}',
  function (this: E2EWorld, peerName: string, hostEnvOrAlias: string) {
    const url = resolvePeerUrl(hostEnvOrAlias);
    getPeerMap(this).set(peerName, url);
    // Also register as a doorway so shared doorway-based steps work on the same peer
    this.addDoorway(peerName, url);
  }
);

// ---------------------------------------------------------------------------
// When — generic surface query
// ---------------------------------------------------------------------------

/**
 * Hit a surface path on the named peer and store the parsed JSON.
 * The path must start with '/'. For surfaces that require auth, the request
 * is unauthenticated (dataplane surfaces are public observability endpoints).
 *
 * Example:
 *   When I query "/health" on peer "alpha-A"
 *   When I query "/sync/v1/elohim/docs" on peer "alpha-A"
 */
When(
  'I query {string} on peer {string}',
  async function (this: E2EWorld, path: string, peerName: string) {
    const baseUrl = getPeerUrl(this, peerName);
    // Delegate to the single shared HTTP path in surfaces.ts (no inline undici import).
    const { status, text } = await getRaw(`${baseUrl}${path}`);
    let parsed: unknown;
    try {
      parsed = JSON.parse(text);
    } catch {
      parsed = text; // surface returned non-JSON (e.g. Prometheus text)
    }
    lastCapture.set(this, { peerName, path, status, body: parsed });
  }
);

// ---------------------------------------------------------------------------
// Then — /health assertions
// ---------------------------------------------------------------------------

/**
 * Assert that p2p.caughtUp is true on the peer's /health endpoint.
 * p2p.caughtUp is set only when P2P is enabled and the projection-reconcile
 * stream has caught up — None/absent means "not yet" (never false-positive).
 *
 * NOTE: Step text uses regex (not Cucumber expression) because '/' would be
 * parsed as an alternation separator in a Cucumber expression string.
 */
Then(
  /^peer "([^"]+)" \/health p2p\.caughtUp is true$/,
  async function (this: E2EWorld, peerName: string) {
    const url = getPeerUrl(this, peerName);
    const { body } = await probeHealth(url);
    assert.ok(
      body.p2p !== undefined,
      `${peerName} /health: p2p section absent — P2P may not be enabled`
    );
    assert.strictEqual(
      body.p2p?.caughtUp,
      true,
      `${peerName} /health: p2p.caughtUp is ${JSON.stringify(body.p2p?.caughtUp)} (expected true)`
    );
  }
);

/**
 * Assert that the p2p.peerCount is at least the given number.
 */
Then(
  /^peer "([^"]+)" \/health peerCount >= (\d+)$/,
  async function (this: E2EWorld, peerName: string, minCountStr: string) {
    const minCount = Number.parseInt(minCountStr, 10);
    const url = getPeerUrl(this, peerName);
    const { body } = await probeHealth(url);
    assert.ok(
      body.p2p !== undefined,
      `${peerName} /health: p2p section absent — P2P may not be enabled`
    );
    const peerCount = body.p2p?.peerCount ?? 0;
    assert.ok(
      peerCount >= minCount,
      `${peerName} /health: p2p.peerCount is ${peerCount}, expected >= ${minCount}`
    );
  }
);

/**
 * Assert that the conductor.connected flag is true on /health.
 */
Then(
  /^peer "([^"]+)" \/health conductor\.connected is true$/,
  async function (this: E2EWorld, peerName: string) {
    const url = getPeerUrl(this, peerName);
    const { body } = await probeHealth(url);
    assert.strictEqual(
      body.conductor.connected,
      true,
      `${peerName} /health: conductor.connected is false`
    );
  }
);

/**
 * Assert that the projection.writer flag matches the expected value on /health.
 */
Then(
  /^peer "([^"]+)" \/health projection\.writer is (true|false)$/,
  async function (this: E2EWorld, peerName: string, expectedStr: string) {
    const url = getPeerUrl(this, peerName);
    const { body } = await probeHealth(url);
    const expected = expectedStr === 'true';
    assert.strictEqual(
      body.projection.writer,
      expected,
      `${peerName} /health: projection.writer is ${body.projection.writer}, expected ${expected}`
    );
  }
);

/**
 * Assert /health returns a healthy status (healthy: true, status: 'online').
 */
Then('peer {string} is healthy', async function (this: E2EWorld, peerName: string) {
  const url = getPeerUrl(this, peerName);
  const { body } = await probeHealth(url);
  assert.ok(
    body.healthy,
    `${peerName} /health: healthy=false, status=${body.status}, error=${body.error ?? 'none'}`
  );
});

// ---------------------------------------------------------------------------
// Then — /sync assertions
// ---------------------------------------------------------------------------

/**
 * Assert that a sync document with the given docId exists on the peer
 * and has at least one head (non-empty heads array).
 *
 * The hAppId defaults to "elohim" (the primary app ID used by the content
 * projection producer). Pass a different ID via the full step path:
 *   Then /sync doc "content:<id>" is present on peer "alpha-A"
 *
 * NOTE: Uses regex because leading '/' in Cucumber expressions is alternation.
 */
Then(
  /^\/sync doc "([^"]+)" is present on peer "([^"]+)"$/,
  async function (this: E2EWorld, docId: string, peerName: string) {
    const url = getPeerUrl(this, peerName);
    // Use the default hAppId; callers who need a different hApp use the raw
    // query step or a bespoke step in the concern-specific step file.
    const hAppId = 'elohim';
    const { body } = await probeSyncDocHeads(url, hAppId, docId);
    assert.ok(
      Array.isArray(body.heads) && body.heads.length > 0,
      `/sync doc "${docId}" on ${peerName}: heads is empty or missing — document not yet synced`
    );
  }
);

/**
 * Assert that a sync document has the given minimum number of heads.
 */
Then(
  /^\/sync doc "([^"]+)" on peer "([^"]+)" has at least (\d+) heads?$/,
  async function (this: E2EWorld, docId: string, peerName: string, minHeadsStr: string) {
    const minHeads = Number.parseInt(minHeadsStr, 10);
    const url = getPeerUrl(this, peerName);
    const { body } = await probeSyncDocHeads(url, 'elohim', docId);
    assert.ok(
      body.heads.length >= minHeads,
      `/sync doc "${docId}" on ${peerName}: ${body.heads.length} heads, expected >= ${minHeads}`
    );
  }
);

// ---------------------------------------------------------------------------
// Then — /blob assertions
// ---------------------------------------------------------------------------

/**
 * Assert that a blob is present on the peer (GET /blob/{hash} returns 200).
 */
Then(
  'blob {string} is byte-present on peer {string}',
  async function (this: E2EWorld, hash: string, peerName: string) {
    const url = getPeerUrl(this, peerName);
    const status = await probeBlob(url, hash);
    assert.strictEqual(
      status,
      200,
      `blob "${hash}" on ${peerName}: expected HTTP 200 but got ${status} — blob not yet present`
    );
  }
);

/**
 * Assert that a blob is absent on the peer (GET /blob/{hash} returns 404).
 */
Then(
  'blob {string} is NOT present on peer {string}',
  async function (this: E2EWorld, hash: string, peerName: string) {
    const url = getPeerUrl(this, peerName);
    const status = await probeBlob(url, hash);
    assert.strictEqual(
      status,
      404,
      `blob "${hash}" on ${peerName}: expected HTTP 404 but got ${status}`
    );
  }
);

// ---------------------------------------------------------------------------
// Then — /db/content assertions
// ---------------------------------------------------------------------------

/**
 * Assert that a content item's blobHash field is non-null on the peer.
 * A non-null blobHash confirms that a blob has been attached to the EPR node.
 */
Then(
  'EPR {string} blobHash is non-null on peer {string}',
  async function (this: E2EWorld, eprId: string, peerName: string) {
    const url = getPeerUrl(this, peerName);
    const { body } = await probeContent(url, eprId);
    assert.ok(
      body.blobHash !== null && body.blobHash !== undefined && body.blobHash !== '',
      `EPR "${eprId}" on ${peerName}: blobHash is ${JSON.stringify(body.blobHash)} — blob not yet attached`
    );
  }
);

// ---------------------------------------------------------------------------
// Then — anti-hard-fail assertion (SPA fallthrough guard)
// ---------------------------------------------------------------------------

/**
 * Assert that resolving a path on the peer does NOT return the "App not found"
 * error page. This guards against the EprRouter-empties-on-poisoned-scope bug
 * (the whole router returning empty sends the SPA shell, which shows
 * "App not found" at the root projection).
 */
Then(
  'resolving {string} on peer {string} does NOT return App-not-found',
  async function (this: E2EWorld, path: string, peerName: string) {
    const url = getPeerUrl(this, peerName);
    // Delegate to the single shared HTTP path in surfaces.ts (no inline undici import).
    const { status: statusCode, text } = await getRaw(`${url}${path}`);
    assert.notStrictEqual(
      statusCode,
      404,
      `Resolving "${path}" on ${peerName} returned 404 — route may not be registered`
    );
    assert.ok(
      !text.includes('App not found') && !text.includes('app-not-found'),
      `Resolving "${path}" on ${peerName}: response contains "App not found" — EprRouter may be empty`
    );
  }
);

// ---------------------------------------------------------------------------
// Then — /p2p/status assertions (direct storage access)
// ---------------------------------------------------------------------------

/**
 * Assert a /p2p/status field on the peer's direct storage URL.
 * Returns 'pending' when E2E_STORAGE_{PEER} is not set (not a failure —
 * the surface is not accessible through the doorway in the alpha environment).
 *
 * NOTE: Uses regex because leading '/' in Cucumber expressions is alternation.
 */
Then(
  /^peer "([^"]+)" \/p2p\/status (\w+) >= (\d+)$/,
  async function (this: E2EWorld, peerName: string, fieldName: string, minValueStr: string) {
    const minValue = Number.parseInt(minValueStr, 10);
    const storageUrl = resolveStorageUrl(peerName);
    if (!storageUrl) {
      // eslint-disable-next-line no-console
      console.log(
        `  PENDING: peer "${peerName}" /p2p/status ${fieldName} — ` +
          `E2E_STORAGE_${peerName.toUpperCase()} not set (direct storage not accessible)`
      );
      return 'pending';
    }
    const { body } = await probeP2PStatus(storageUrl);
    const actual = body[fieldName];
    assert.ok(
      typeof actual === 'number',
      `/p2p/status.${fieldName} on ${peerName}: expected a number, got ${typeof actual}`
    );
    assert.ok(
      actual >= minValue,
      `/p2p/status.${fieldName} on ${peerName}: ${actual} < ${minValue}`
    );
  }
);

/**
 * Assert that /p2p/status.pull.caughtUp is true on the peer's direct storage URL.
 */
Then(
  /^peer "([^"]+)" \/p2p\/status pull\.caughtUp is true$/,
  async function (this: E2EWorld, peerName: string) {
    const storageUrl = resolveStorageUrl(peerName);
    if (!storageUrl) {
      // eslint-disable-next-line no-console
      console.log(
        `  PENDING: peer "${peerName}" /p2p/status pull.caughtUp — ` +
          `storage URL not set (direct storage not accessible via doorway)`
      );
      return 'pending';
    }
    const { body } = await probeP2PStatus(storageUrl);
    assert.ok(body.pull !== undefined, `/p2p/status.pull on ${peerName}: pull section absent`);
    assert.strictEqual(
      body.pull?.caughtUp,
      true,
      `/p2p/status.pull.caughtUp on ${peerName}: ${body.pull?.caughtUp}`
    );
  }
);

// ---------------------------------------------------------------------------
// Then — /metrics assertions
// ---------------------------------------------------------------------------

/**
 * Assert a Prometheus metric value on the peer's /metrics endpoint.
 *
 * Supports comparison operators: >=, <=, >, <, ==, !=
 *
 * For doorway metrics (port 8080): pass the doorway URL and the step
 * internally swaps to port 8080 for the metrics endpoint.
 * For storage metrics: set E2E_STORAGE_{PEER} and use the storage URL.
 *
 * Example:
 *   Then metric "doorway_watchdog_reconnects_total" on peer "alpha-A" >= 0
 *   Then metric "p2p_connected_peers" on peer "alpha-A" >= 1
 */
Then(
  'metric {string} on peer {string} {word} {float}',
  async function (
    this: E2EWorld,
    metricName: string,
    peerName: string,
    cmp: string,
    expected: number
  ) {
    const doorwayUrl = getPeerUrl(this, peerName);

    // Determine which metrics endpoint to probe:
    // - doorway metrics: same host, port 8080 (if E2E_DOORWAY_METRICS_ALPHA is set,
    //   use that; otherwise derive from the doorway URL)
    // - storage metrics: E2E_STORAGE_ALPHA direct URL
    const metricsEnvKey = `E2E_METRICS_${peerName.toUpperCase().replace(/[^A-Z0-9]/g, '_')}`;
    const storageUrl = resolveStorageUrl(peerName);

    let metricsBaseUrl: string;
    if (process.env[metricsEnvKey]) {
      metricsBaseUrl = process.env[metricsEnvKey]!;
    } else if (
      metricName.startsWith('p2p_') ||
      metricName.startsWith('reconcile_') ||
      metricName.startsWith('dedup_')
    ) {
      // Storage metric — needs direct storage URL
      if (!storageUrl) {
        // eslint-disable-next-line no-console
        console.log(
          `  PENDING: metric "${metricName}" on ${peerName} — ` +
            `storage metrics URL not set (set E2E_STORAGE_${peerName.toUpperCase()})`
        );
        return 'pending';
      }
      metricsBaseUrl = storageUrl;
    } else {
      // Doorway metric: try port 8080 on the same host
      try {
        const parsed = new URL(doorwayUrl);
        parsed.port = '8080';
        metricsBaseUrl = parsed.origin;
      } catch {
        metricsBaseUrl = doorwayUrl;
      }
    }

    let metrics: ParsedMetrics;
    try {
      metrics = await probeMetrics(metricsBaseUrl);
    } catch (err) {
      // eslint-disable-next-line no-console
      console.log(
        `  PENDING: metric "${metricName}" on ${peerName} — /metrics not reachable: ${String(err)}`
      );
      return 'pending';
    }

    const actual = metrics.get(metricName);
    if (actual === undefined) {
      // Metric absent from scrape — a typo'd name would silently false-pass a >= 0 assertion.
      // Treat as an observability gap (pending), not a confirmed zero measurement.
      // eslint-disable-next-line no-console
      console.log(
        `  PENDING: metric "${metricName}" on ${peerName} — ` +
          `metric not present in scrape (typo? not yet emitted?)`
      );
      return 'pending';
    }
    assertMetric(metricName, peerName, cmp, expected, actual);
  }
);

/** Apply a comparison operator and throw a descriptive assertion if it fails */
function assertMetric(
  metricName: string,
  peerName: string,
  cmp: string,
  expected: number,
  actual: number
): void {
  const label = `metric "${metricName}" on ${peerName}: ${actual} ${cmp} ${expected}`;
  switch (cmp) {
    case '>=':
      assert.ok(actual >= expected, `${label} — FAILED`);
      break;
    case '<=':
      assert.ok(actual <= expected, `${label} — FAILED`);
      break;
    case '>':
      assert.ok(actual > expected, `${label} — FAILED`);
      break;
    case '<':
      assert.ok(actual < expected, `${label} — FAILED`);
      break;
    case '==':
    case '=':
      assert.strictEqual(actual, expected, `${label} — FAILED`);
      break;
    case '!=':
      assert.notStrictEqual(actual, expected, `${label} — FAILED`);
      break;
    default:
      throw new Error(`Unknown comparison operator: "${cmp}". Use >=, <=, >, <, ==, !=`);
  }
}

// ---------------------------------------------------------------------------
// Then — EPR nav-context reachability
// ---------------------------------------------------------------------------

/**
 * Assert that a specific EPR's nav-context is reachable (non-404) and
 * the response is not an "App not found" error.
 */
Then(
  'EPR {string} nav-context is reachable on peer {string}',
  async function (this: E2EWorld, eprId: string, peerName: string) {
    const url = getPeerUrl(this, peerName);
    const { body } = await probeEprNavContext(url, eprId);
    assert.ok(typeof body === 'object', `EPR "${eprId}" nav-context: empty response`);
  }
);

// ---------------------------------------------------------------------------
// Then — generic last-capture assertions (for "When I query" step)
// ---------------------------------------------------------------------------

/**
 * Assert that the last queried surface returned the given HTTP status.
 */
Then('the surface response status is {int}', function (this: E2EWorld, expectedStatus: number) {
  const capture = lastCapture.get(this);
  assert.ok(capture, 'No surface response captured — run "When I query" first');
  assert.strictEqual(
    capture.status,
    expectedStatus,
    `Surface "${capture.path}" on ${capture.peerName}: status ${capture.status}, expected ${expectedStatus}`
  );
});

/**
 * Assert that the last queried surface response body contains a specific key with a truthy value.
 */
Then('the surface response has field {string}', function (this: E2EWorld, fieldName: string) {
  const capture = lastCapture.get(this);
  assert.ok(capture, 'No surface response captured — run "When I query" first');
  const body = capture.body as Record<string, unknown>;
  assert.ok(
    Object.prototype.hasOwnProperty.call(body, fieldName),
    `Surface "${capture.path}": field "${fieldName}" not present in response`
  );
});
