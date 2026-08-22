/**
 * Step definitions for the delivery-capability and cache-invalidation stories:
 *   features/delivery/delivery-diagnostics.feature  (peer capability introspection)
 *   features/delivery/web2-absorption.feature       (re-seed invalidation, load)
 *
 * ## Where "delivery capabilities" actually live
 *
 * There is no `/api/v1/delivery/capabilities` route. A peer's delivery
 * capabilities reach other peers two ways, and only one of them is readable
 * over HTTP:
 *
 *   * `EprRequest::QueryDelivery` → `EprResponse::DeliveryInfo { serves_extracted,
 *     serves_compressed, cache_tier, warm }` — a libp2p request/response on
 *     /elohim/epr/1.0.0. No HTTP route triggers it, so a2o cannot ask a peer
 *     this question directly.
 *   * `GET /api/v1/peers/delivery` on elohim-storage → the neighbour list the
 *     node built FROM gossiped capacity announcements. Each row is
 *     {peerId, multiaddrs, network, capabilities, lastSeen, httpPort,
 *     householdId, commitments}, where `capabilities` is a flat string array
 *     ("serves_compressed", "serves_extracted", "cache_tier:extraction") rather
 *     than named fields.
 *
 * These steps read the HTTP route and assert against its REAL shape. Where the
 * story asks for something the wire does not carry, the assertion says so in
 * the failure message rather than being softened — the gap is the finding.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then, DataTable } from '@cucumber/cucumber';

import { request } from 'undici';

import {
  destructiveAllowed,
  DESTRUCTIVE_HELD_HINT,
} from '../../src/framework/fixtures/substrate-scope.js';

import type { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Destructive gate — shared convention with steps/mesh/*
// ---------------------------------------------------------------------------

const DESTRUCTIVE_ENABLED = destructiveAllowed();

function destructiveGate(whatItWouldDo: string): boolean {
  if (DESTRUCTIVE_ENABLED) return true;
  // eslint-disable-next-line no-console
  console.log(
    `  ⏭️  SKIPPED (destructive, gated): would ${whatItWouldDo}. ${DESTRUCTIVE_HELD_HINT}`
  );
  return false;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function storageUrl(): string {
  return process.env['E2E_STORAGE_URL'] ?? 'http://localhost:8090';
}

function doorwayUrl(world: E2EWorld): string {
  const url = world.doorways.get('alpha')?.url ?? process.env['E2E_DOORWAY_ALPHA'];
  assert.ok(url, 'No doorway "alpha" registered and E2E_DOORWAY_ALPHA is not set');
  return url.replace(/\/$/, '');
}

async function getJson(url: string): Promise<{ status: number; body: unknown }> {
  const { statusCode, body } = await request(url);
  const text = await body.text();
  let parsed: unknown = null;
  try {
    parsed = JSON.parse(text);
  } catch {
    parsed = text;
  }
  return { status: statusCode, body: parsed };
}

interface DeliveryPeer {
  peerId?: string;
  multiaddrs?: string[];
  network?: string;
  capabilities?: string[];
  lastSeen?: number;
  httpPort?: number;
  householdId?: string | null;
  commitments?: unknown[];
}

const deliveryPeersStore = new WeakMap<E2EWorld, DeliveryPeer[]>();

function storedPeers(world: E2EWorld): DeliveryPeer[] {
  const peers = deliveryPeersStore.get(world);
  assert.ok(
    peers,
    'no delivery-peer capture — a "queries delivery capabilities" step must run first'
  );
  return peers;
}

/** Every capability string advertised by any peer in the capture. */
function allCapabilities(peers: DeliveryPeer[]): string[] {
  return peers.flatMap(p => p.capabilities ?? []);
}

// ---------------------------------------------------------------------------
// Peer capability introspection
// ---------------------------------------------------------------------------

/**
 * The extraction cache is an in-process tier of elohim-storage, not a separate
 * service: the check that it is running is the check that the node answers its
 * own delivery-peer route at all.
 */
Given('elohim-storage is running with ExtractionCache', async function (this: E2EWorld) {
  const { status } = await getJson(`${storageUrl()}/health`);
  assert.ok(status < 500, `elohim-storage at ${storageUrl()} answered /health with ${status}`);
});

When(
  '{word} queries delivery capabilities for the storage peer',
  async function (this: E2EWorld, _actor: string) {
    const { status, body } = await getJson(`${storageUrl()}/api/v1/peers/delivery`);
    assert.equal(status, 200, `GET /api/v1/peers/delivery returned ${status}`);
    assert.ok(
      Array.isArray(body),
      `expected an array of peers; got ${JSON.stringify(body).slice(0, 200)}`
    );
    deliveryPeersStore.set(this, body as DeliveryPeer[]);
  }
);

When(
  '{word} requests the delivery capability summary',
  async function (this: E2EWorld, _actor: string) {
    const { status, body } = await getJson(`${storageUrl()}/api/v1/peers/delivery`);
    assert.equal(status, 200, `GET /api/v1/peers/delivery returned ${status}`);
    assert.ok(
      Array.isArray(body),
      `expected an array of peers; got ${JSON.stringify(body).slice(0, 200)}`
    );
    deliveryPeersStore.set(this, body as DeliveryPeer[]);
  }
);

Given(
  'doorway {string} is connected to {int} peers',
  async function (this: E2EWorld, _doorway: string, expected: number) {
    const { status, body } = await getJson(`${storageUrl()}/api/v1/peers/delivery`);
    assert.equal(status, 200, `GET /api/v1/peers/delivery returned ${status}`);
    const peers = body as DeliveryPeer[];
    assert.equal(
      peers.length,
      expected,
      `this mesh advertises ${peers.length} delivery peer(s) from ${storageUrl()}, not ${expected}. ` +
        'The household triad gives any one member two connected peers; a scenario that names a ' +
        'different fabric size needs that fabric, not a rewritten expectation.'
    );
    deliveryPeersStore.set(this, peers);
  }
);

Then('the response shows serves_extracted and serves_compressed status', function (this: E2EWorld) {
  const caps = allCapabilities(storedPeers(this));
  assert.ok(
    caps.includes('serves_compressed'),
    `no peer advertises "serves_compressed"; advertised: [${caps.join(', ')}]`
  );
  assert.ok(
    caps.includes('serves_extracted'),
    'no peer advertises "serves_extracted" — the delivery-peer row carries only ' +
      `[${caps.join(', ')}]. A client choosing a peer to fetch individual files from cannot ` +
      'tell whether ANY peer can serve them file-by-file, so it must fall back to whole-blob ' +
      'download every time.'
  );
});

Then('the response shows cache_tier', function (this: E2EWorld) {
  const caps = allCapabilities(storedPeers(this));
  const tier = caps.find(c => c.startsWith('cache_tier:'));
  assert.ok(
    tier,
    'no peer advertises a "cache_tier:<tier>" capability — advertised: ' +
      `[${caps.join(', ')}]. Tier is what tells a client whether a peer answers from a ` +
      'projection, an extraction cache, or only raw blobs; without it every peer looks alike.'
  );
});

Then('the response lists ready_content hashes', function (this: E2EWorld) {
  const peers = storedPeers(this);
  const withReady = peers.filter(
    p => (p.capabilities ?? []).some(c => c.startsWith('ready_content')) || 'readyContent' in p
  );
  assert.ok(
    withReady.length > 0,
    'no peer carries ready_content on the HTTP delivery-peer row. The content-hash list a peer ' +
      'holds warm exists on the libp2p side (EprResponse::DeliveryInfo) but is not projected onto ' +
      `GET /api/v1/peers/delivery. Rows seen: ${JSON.stringify(peers).slice(0, 400)}`
  );
});

/**
 * The story names the per-peer fields an operator needs. This checks them
 * against the row the route actually returns, naming each field the wire does
 * not carry — an operator cannot read a posture from fields that are not there.
 */
Then('the summary lists each peer with:', function (this: E2EWorld, table: DataTable) {
  const peers = storedPeers(this);
  assert.ok(peers.length > 0, 'the delivery capability summary is empty — no peers to describe');
  const wanted = table.hashes().map(row => String(row['field']));
  const camel = (snake: string): string =>
    snake.replace(/_([a-z])/g, (_m, c: string) => c.toUpperCase());

  const missing: string[] = [];
  for (const field of wanted) {
    const key = camel(field);
    const present = peers.every(p => key in (p as Record<string, unknown>));
    if (!present) missing.push(field);
  }
  assert.deepEqual(
    missing,
    [],
    `the delivery-peer row does not carry ${missing.join(', ')}. Present keys: ` +
      `[${Object.keys(peers[0] as Record<string, unknown>).join(', ')}]. serves_extracted, ` +
      'serves_compressed and cache_tier are flattened into the untyped `capabilities` string ' +
      'array, and ready_content is not projected onto HTTP at all — so an operator reading this ' +
      'summary cannot answer "which peer can serve me this file" without parsing strings.'
  );
});

// ---------------------------------------------------------------------------
// Concurrent browser load
// ---------------------------------------------------------------------------

/**
 * The concurrent-load scenarios fan one "browser" out to every file in the app
 * bundle (~40 requests each), and /apps/{slug}/{file} is NOT exempt from the
 * doorway membrane: 300 requests / 60s per client IP, a 429 challenge at 600,
 * and a 900-second BAN at 1200. A 30-browser fan-out is ~1200 requests from one
 * source IP — enough to ban the whole suite, and on a mesh shared with other
 * runs, to ban them too. So the load is held behind the same explicit go-ahead
 * as any other act that changes the mesh for everyone.
 */
Given("this run owns the doorway's request budget", function (this: E2EWorld) {
  if (
    !destructiveGate(
      'fan ~1200 requests at one doorway from a single source IP — past the membrane BAN ' +
        'threshold (300/60s shape, 429 at 600, 900s ban at 1200)'
    )
  ) {
    return 'skipped';
  }
  return undefined;
});

// ---------------------------------------------------------------------------
// Re-seed invalidation
// ---------------------------------------------------------------------------

interface ReseedState {
  slug: string;
  /** The content address the doorway SERVED before the re-seed (X-Content-Address). */
  originalHash: string | null;
  /** The storage row's blobHash before the re-seed — what cleanup restores. */
  storedBlobHash: string | null;
}
const reseedStore = new WeakMap<E2EWorld, ReseedState>();

/** The doorway reports the served bundle's address as X-Content-Address on every app
 *  file response (doorway-service/src/routes/apps.rs `build_app_response`); X-Blob-Hash
 *  exists only on the `_capability` probe. Reading the wrong header made both sides
 *  `null` and the eviction unobservable (run 20260822T201747Z-3bd326d6). */
function servedContentAddress(headers: Record<string, unknown>): string | null {
  return (headers['x-content-address'] as string | undefined) ?? null;
}

/** Authorization the mesh's admin write routes accept; absent on substrates without one. */
function adminHeaders(): Record<string, string> {
  const key = process.env['API_KEY_ADMIN'];
  return key ? { authorization: `Bearer ${key}` } : {};
}

/** PATCH one row's blobHash on the primary storage peer; returns the status and body. */
async function patchBlobHash(
  slug: string,
  blobHash: string
): Promise<{ status: number; text: string }> {
  const { statusCode, body } = await request(`${storageUrl()}/db/content/${slug}`, {
    method: 'PATCH',
    headers: { 'content-type': 'application/json', ...adminHeaders() },
    body: JSON.stringify({ blobHash }),
  });
  return { status: statusCode, text: await body.text() };
}

Given(
  'the projection cache for {string} is warm with blob_hash {string}',
  { timeout: 120_000 },
  async function (this: E2EWorld, slug: string, _namedHash: string) {
    const base = doorwayUrl(this);
    // Warm by serving the entry document; the doorway caches what it proxies.
    const { statusCode, headers } = await request(`${base}/apps/${slug}/index.html`);
    assert.ok(
      statusCode >= 200 && statusCode < 400,
      `warming "${slug}" through ${base} returned ${statusCode}`
    );
    const observed = servedContentAddress(headers);
    // The storage row is the thing the re-seed moves and cleanup must put back.
    const { body: row } = await getJson(`${storageUrl()}/db/content/${slug}`);
    const storedBlobHash = (row as Record<string, unknown> | null)?.['blobHash'];
    // The scenario's literal "sha256-old" is a stand-in for "whatever hash is
    // cached right now": the contract is that the CACHED hash is superseded,
    // not that it equals a particular string.
    reseedStore.set(this, {
      slug,
      originalHash: observed,
      storedBlobHash: typeof storedBlobHash === 'string' ? storedBlobHash : null,
    });
  }
);

When('{string} is re-seeded with a new ZIP blob', async function (this: E2EWorld, slug: string) {
  if (!destructiveGate(`re-seed "${slug}" on ${storageUrl()} with a new blob`)) {
    return 'skipped';
  }
  const state = reseedStore.get(this);
  assert.ok(state, 'no warm-cache baseline — the "is warm with blob_hash" Given must run first');
  assert.ok(
    state.storedBlobHash,
    `storage holds no blobHash for "${slug}" — nothing to supersede or restore`
  );
  // "A new ZIP blob" must be a REAL bundle the peer holds, so the app keeps
  // serving during the scenario and the new address is observable end to end.
  // The previous shape PATCHed a fabricated `sha256-a2o-reseed-<ts>` and never
  // restored it: the row pointed at bytes nobody had, doorway A served 404 for
  // the app from then on, and four delivery-diagnostics scenarios in other
  // runs went red on a fixture this step had poisoned (2026-08-22). The
  // landing bundle is the stand-in: seeded on every household peer by the
  // Prologue, a ZIP with its own index.html.
  const { status: donorStatus, body: donor } = await getJson(
    `${storageUrl()}/db/content/elohim-host-landing`
  );
  assert.equal(donorStatus, 200, `GET /db/content/elohim-host-landing returned ${donorStatus}`);
  const newHash = (donor as Record<string, unknown>)['blobHash'];
  assert.ok(
    typeof newHash === 'string' && newHash !== state.storedBlobHash,
    `no distinct donor bundle to re-seed "${slug}" with (landing blobHash ${String(newHash)})`
  );
  // Restore the row when the scenario ends, whatever happens in between — a
  // moved head on a shared mesh is exactly the kind of change that outlives
  // the run that made it. Same gate as the move itself.
  const restoreTo = state.storedBlobHash;
  this.onCleanup(async () => {
    const { status, text } = await patchBlobHash(slug, restoreTo);
    if (status < 200 || status >= 300) {
      console.warn(
        `  ⚠️  could not restore "${slug}" blobHash to ${restoreTo} (${status}): ${text.slice(0, 200)}`
      );
    }
  });
  const { status, text } = await patchBlobHash(slug, newHash);
  assert.ok(
    status >= 200 && status < 300,
    `re-seeding "${slug}" returned ${status}: ${text.slice(0, 300)}`
  );
  return undefined;
});

Then(
  'the projection cache entries for {string} with hash {string} are evicted',
  { timeout: 120_000 },
  async function (this: E2EWorld, slug: string, _staleHash: string) {
    const base = doorwayUrl(this);
    const state = reseedStore.get(this);
    assert.ok(state, 'no warm-cache baseline');
    // The content-event subscriber clears the slug on content.updated; give the
    // SSE hop a bounded window, then read the serving hash back.
    const deadline = Date.now() + 60_000;
    let observed: string | null = null;
    while (Date.now() < deadline) {
      const { headers } = await request(`${base}/apps/${slug}/index.html`);
      observed = servedContentAddress(headers);
      if (observed !== state.originalHash) return;
      await new Promise(r => setTimeout(r, 3_000));
    }
    assert.fail(
      `the doorway is still serving "${slug}" at blob hash ${String(observed)} 60s after the ` +
        'content row was re-seeded. The projection cache did not evict on content.updated, so ' +
        'every visitor keeps receiving the superseded bundle.'
    );
  }
);

Then(
  'the next request for {string} proxies to storage',
  async function (this: E2EWorld, slug: string) {
    const base = doorwayUrl(this);
    const { headers } = await request(`${base}/apps/${slug}/index.html`);
    const cache = (headers['x-cache'] as string | undefined) ?? '';
    assert.ok(
      ['MISS', 'BYPASS', 'BYPASS-ADMIN'].includes(cache),
      `expected the post-eviction request to reach storage (X-Cache MISS/BYPASS); got "${cache}". ` +
        'A HIT here means the doorway answered from an entry it should have dropped.'
    );
  }
);

Then('the new response is cached with the new blob_hash', async function (this: E2EWorld) {
  const base = doorwayUrl(this);
  const state = reseedStore.get(this);
  assert.ok(state, 'no warm-cache baseline');
  const { headers } = await request(`${base}/apps/${state.slug}/index.html`);
  const hash = servedContentAddress(headers);
  assert.ok(
    hash !== null && hash !== state.originalHash,
    `expected the re-warmed cache to carry a NEW blob hash; got ${String(hash)} ` +
      `(baseline ${String(state.originalHash)})`
  );
});

/**
 * Name the propagation trigger the invalidation actually rests on, so the
 * causal chain is visible in the Gherkin rather than implied: elohim-storage
 * publishes an SSE event stream, the doorway's storage-events subscriber
 * consumes it, and a content.{created,updated,deleted} event for a slug clears
 * every cached file for that slug. Nothing polls; nothing compares hashes at
 * request time. This step asserts the stream the whole mechanism hangs off is
 * actually being consumed — a doorway with a dead subscription would evict
 * nothing and the next step's failure would look like a cache bug.
 */
When(
  'storage publishes the content-changed event for {string}',
  async function (this: E2EWorld, _slug: string) {
    const base = doorwayUrl(this);
    const { status, body } = await getJson(`${base}/admin/self-healing`);
    assert.equal(status, 200, `GET /admin/self-healing returned ${status}`);
    const warmup = (body as Record<string, unknown>)['warmup'] as
      | Record<string, unknown>
      | undefined;
    assert.ok(
      warmup?.['lastError'] === null,
      'the doorway is not consuming storage events cleanly ' +
        `(warmup.lastError=${JSON.stringify(warmup?.['lastError'])}). Eviction is PUSHED over ` +
        "that stream, so with it broken no re-seed can reach the cache and the next step's " +
        'failure would read as a cache bug rather than a dead subscription.'
    );
  }
);
