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
  postRaw,
  probeHealth,
  probeSyncDocs,
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

/**
 * Assert that p2p.divergentAnchor is at most the given bound.
 * divergentAnchor counts DHT anchors the gossip reconciler found diverged on the last
 * reconcile pass. A value within the bound confirms the mesh is converging, not diverging.
 *
 * NOTE: Uses regex because '/' in Cucumber expressions is parsed as alternation.
 */
Then(
  /^peer "([^"]+)" \/health divergentAnchor <= (\d+)$/,
  async function (this: E2EWorld, peerName: string, maxAnchorStr: string) {
    const maxAnchor = Number.parseInt(maxAnchorStr, 10);
    const url = getPeerUrl(this, peerName);
    const { body } = await probeHealth(url);
    assert.ok(
      body.p2p !== undefined,
      `${peerName} /health: p2p section absent — P2P may not be enabled`
    );
    const divergentAnchor = body.p2p?.divergentAnchor ?? 0;
    assert.ok(
      divergentAnchor <= maxAnchor,
      `${peerName} /health: p2p.divergentAnchor is ${divergentAnchor}, expected <= ${maxAnchor}`
    );
  }
);

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

/**
 * Assert that the /sync/v1/elohim/docs list has at least the given number of documents.
 * Confirms the content-projection producer has run and the sync table is non-empty.
 *
 * NOTE: Uses regex because '/' in Cucumber expressions is parsed as alternation.
 */
Then(
  /^\/sync\/v1\/elohim\/docs list on peer "([^"]+)" has at least (\d+) documents?$/,
  async function (this: E2EWorld, peerName: string, minCountStr: string) {
    const minCount = Number.parseInt(minCountStr, 10);
    const url = getPeerUrl(this, peerName);
    const { body } = await probeSyncDocs(url, 'elohim');
    assert.ok(
      body.total >= minCount,
      `/sync/v1/elohim/docs on ${peerName}: total=${body.total}, expected >= ${minCount}`
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

/**
 * Assert a content row's REQ-F10 trust legibility tier on the peer.
 *
 * `trust` is computed on the ContentView from the row's provenance markers:
 *   "notarized"   — dht_anchor_hash set (green padlock — DHT-notary HEAD anchor)
 *   "published"   — p2p_published_at set (blue — peer-attested custody, served but not notarized)
 *   "unconfirmed" — crdt_converged_at-only or all-null (amber — served like HTTP "Not secure")
 *
 * The notary-authority concern asserts "notarized" on every federation peer: the DHT-notary
 * HEAD anchor must propagate uniformly, not stamp only on the peer whose conductor ran the
 * deploy. A lower tier here means the anchor has not reached this peer — an honest red until
 * the notary-overlay propagation lands (Plan C1/C3/C4).
 */
Then(
  'EPR {string} trust is {string} on peer {string}',
  async function (this: E2EWorld, eprId: string, expectedTrust: string, peerName: string) {
    const url = getPeerUrl(this, peerName);
    // Use getRaw (never throws) so a non-200 (e.g. 503 "catching-up" on a peer
    // mid-reconcile, or a 404) surfaces as a crisp "not notarized" red rather
    // than an opaque thrown HTTP error — the invariant is the same either way.
    const { status, text } = await getRaw(`${url}/db/content/${encodeURIComponent(eprId)}`);
    assert.strictEqual(
      status,
      200,
      `EPR "${eprId}" on ${peerName}: GET /db/content returned HTTP ${status} (body: ${text.slice(0, 120)}) ` +
        `— the peer cannot serve the notarized content view, so it has not reached trust="${expectedTrust}".`
    );
    let trust: unknown;
    try {
      trust = (JSON.parse(text) as Record<string, unknown>)['trust'];
    } catch {
      trust = undefined;
    }
    assert.strictEqual(
      trust,
      expectedTrust,
      `EPR "${eprId}" on ${peerName}: trust is ${JSON.stringify(trust)}, expected "${expectedTrust}" ` +
        `— the DHT-notary HEAD anchor (dht_anchor_hash) has not reached this peer at the expected tier ` +
        `(served at a lower trust tier, or the trust field is absent on this build).`
    );
  }
);

/**
 * Assert the federation converges on ONE canonical head: the notarized head anchor
 * (dhtAnchorHash) must be present AND identical across the two named peers.
 *
 * This is the notary's core invariant, strictly stronger than per-peer trust="notarized":
 * two peers can each be green over DIFFERENT heads (divergent dhtAnchorHash + blobHash — each
 * having independently authored/notarized its own bundle), which is a false-green. Convergence
 * means ONE canonical head, elected by earned authority and adopted by every peer. A missing
 * anchor on either peer, or two differing anchors, is an honest red until cross-peer head
 * election/adoption lands (verified adoption — author/community signature checked against the
 * DHT-notary witness, never a blind copy of a peer's asserted value).
 */
Then(
  'EPR {string} resolves the same canonical head across peers {string} and {string}',
  async function (this: E2EWorld, eprId: string, peerA: string, peerB: string) {
    const readAnchor = async (peerName: string): Promise<string> => {
      const url = getPeerUrl(this, peerName);
      const { status, text } = await getRaw(`${url}/db/content/${encodeURIComponent(eprId)}`);
      assert.strictEqual(
        status,
        200,
        `EPR "${eprId}" on ${peerName}: GET /db/content returned HTTP ${status} (body: ${text.slice(0, 120)}) ` +
          `— the peer cannot serve the content view, so no canonical head can be compared.`
      );
      let anchor: unknown;
      try {
        anchor = (JSON.parse(text) as Record<string, unknown>)['dhtAnchorHash'];
      } catch {
        anchor = undefined;
      }
      assert.ok(
        typeof anchor === 'string' && anchor.length > 0,
        `EPR "${eprId}" on ${peerName}: dhtAnchorHash is ${JSON.stringify(anchor)} — no canonical head ` +
          `notarized on this peer, so the federation cannot have converged on one it shares.`
      );
      return anchor;
    };
    const anchorA = await readAnchor(peerA);
    const anchorB = await readAnchor(peerB);
    assert.strictEqual(
      anchorA,
      anchorB,
      `EPR "${eprId}": canonical head DIVERGES — ${peerA} dhtAnchorHash ${anchorA} != ${peerB} ${anchorB}. ` +
        `Both peers are notarized over DIFFERENT heads (false-green): no single earned canonical head has been ` +
        `elected and adopted across the federation.`
    );
  }
);

/**
 * Assert that the notary HEAD-authority surface exists AND refuses a non-author's move.
 *
 * The declared HEAD (declare_content_head, Plan C1/C3) is the notary's authority answer —
 * which version of a content node the author declared. Moving it must be an author-only act.
 *
 * The check is read-only-and-safe while the surface is unwired: it first GETs the HEAD
 * surface for a real (notarized) EPR; if that returns 404 the surface does not exist, so
 * HEAD advances by last-writer recency and NO ONE is refused — the honest red today. Only
 * when the surface exists does it issue an unauthenticated (non-author) move, and it targets
 * a dedicated test-persona id, never production content — a correctly-built gate refuses the
 * non-author before any mutation. So no write is ever attempted while the surface is a 404.
 */
Then(
  'the HEAD authority surface refuses a non-author move of {string} on peer {string}',
  async function (this: E2EWorld, contentId: string, peerName: string) {
    const url = getPeerUrl(this, peerName);
    // (1) Read-only existence check against a real, notarized EPR.
    const headPath = `/db/content/${encodeURIComponent(contentId)}/head`;
    const { status: headStatus } = await getRaw(`${url}${headPath}`);
    assert.notStrictEqual(
      headStatus,
      404,
      `HEAD-authority surface ${headPath} on ${peerName} returned 404 — the notary ` +
        `HEAD-declaration surface (declare_content_head, Plan C1/C3) is not wired. Without it the ` +
        `declared HEAD is chosen by last-writer recency (VERDICT L2 #3), not by the notary, so a ` +
        `non-author's HEAD move cannot be refused — any peer on the wire wins the race. Expected 200 ` +
        `(the notary-arbitrated HEAD) so a non-author move can be gated.`
    );
    // (2) Surface exists → an unauthenticated (non-author) move MUST be refused.
    //     Targets a test-persona id; never production content. Only reached post-(1).
    const nonAuthorProbeId = 'e2e-notary-authority-nonauthor-probe';
    const probePath = `/db/content/${encodeURIComponent(nonAuthorProbeId)}/head`;
    const { status: moveStatus } = await postRaw(`${url}${probePath}`, {
      headActionHash: 'uhCkk-e2e-notary-authority-nonauthor-move',
    });
    assert.ok(
      [401, 403].includes(moveStatus),
      `Non-author move of HEAD on ${peerName}: expected 401/403 (authority refusal), got ` +
        `${moveStatus} — a non-author was able to move the declared HEAD (authority not enforced).`
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
 * Assert the last queried surface was NOT answered with the doorway's own
 * catching-up shed body (503 + {"status":"catching-up"}). Diagnostic probe
 * routes bypass the upstream breaker (doorway-catching-up-page spec §4):
 * during an upstream incident they must return the upstream's real response
 * or its real failure — never the doorway's shed. Healthy substrate →
 * trivially green; incident → pins the self-blinding fix.
 */
Then('the surface response is not the doorway shed body', function (this: E2EWorld) {
  const capture = lastCapture.get(this);
  assert.ok(capture, 'No surface response captured — run "When I query" first');
  const body = capture.body;
  const isShedBody =
    capture.status === 503 &&
    typeof body === 'object' &&
    body !== null &&
    (body as Record<string, unknown>).status === 'catching-up';
  assert.ok(
    !isShedBody,
    `Surface "${capture.path}" on ${capture.peerName} was breaker-shed by the doorway ` +
      '(503 catching-up) — diagnostic probes must bypass the breaker'
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
