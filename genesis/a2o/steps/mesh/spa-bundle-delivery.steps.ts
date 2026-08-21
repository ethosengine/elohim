/**
 * SPA Bundle Delivery step definitions — the @wip scenarios of
 * features/delivery/spa-bundle-delivery.feature that are NOT covered by
 * steps/delivery/substrate-rea-replication.steps.ts.
 *
 * The two @requires:owned-substrate priority scenarios ("/lamad serves the
 * SPA shell once project-epr commitments propagate via DHT" and "/lamad
 * survives a doorway pod restart regardless of which storage peer the new
 * pod reads") are ALREADY fully defined by that sibling file — every Given/
 * When/Then they use (including the shared Background steps and the
 * `responseStore`-backed assertions in steps/delivery.steps.ts) resolves
 * without this file adding anything. This file exists solely to bring the
 * REST of the feature file's still-@wip scenarios to "defined" so the whole
 * file dry-runs clean.
 *
 * Real-vs-pending split:
 *   - Every step below that can hit a live HTTP surface on the configured
 *     mesh doorway does so for real (via the shared `fetchApp`/`responseStore`
 *     from steps/delivery.steps.ts, matching the convention already used by
 *     steps/delivery/substrate-rea-replication.steps.ts). A red here is a
 *     genuine substrate answer, not a stub.
 *   - Steps whose feature text depends on a surface or fixture this mesh does
 *     not stage — forcing a live doorway to run with NO root-app config,
 *     restarting elohim-storage and losing its in-memory state, pushing a
 *     server-side content-update signal, or re-seeding a ZIP blob with a new
 *     hash — `return 'pending'` with a one-line comment naming the gap, per
 *     genesis/a2o/CLAUDE.md's "Guard" convention. None of the two priority
 *     scenarios reach any step in this file, so none of the pending steps
 *     below touch them.
 *
 * DISCOVERY: the doorway source (doorway/doorway-service/src/routes/health.rs,
 * server/http.rs) shows ROOT_APP_SLUG was removed post-B14 — the router now
 * resolves the root purely from whichever EPR commitment declares
 * urlPath="/", and GET /health/startup reports that as `rootProjection`, not
 * the `rootApp` field several @wip scenarios in this file were written
 * against. Steps that touch that drift say so inline rather than silently
 * patching the mismatch into a pass.
 */

import { strict as assert } from 'node:assert';

import { DataTable, Given, When, Then } from '@cucumber/cucumber';

import { E2EWorld } from '../../src/framework/world.js';
import { AppResponse, fetchApp, responseStore } from '../delivery.steps.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const NO_DOORWAY = 'No doorway registered — run the doorway Background step first';
const NO_RESPONSE = 'No response captured — run a When-request step first';

/** First registered doorway's base URL (fixture-backed via the Background step). */
function firstDoorwayUrl(world: E2EWorld): string {
  const doorway = [...world.doorways.values()][0];
  assert.ok(doorway, NO_DOORWAY);
  return doorway.url.replace(/\/$/, '');
}

/** GET a path off the configured doorway and capture it in the shared responseStore. */
async function captureGet(world: E2EWorld, path: string): Promise<AppResponse> {
  const base = firstDoorwayUrl(world);
  const resp = await fetchApp(base, path);
  responseStore.set(world, resp);
  return resp;
}

/**
 * Same Angular-shell heuristic steps/delivery/substrate-rea-replication.steps.ts
 * uses for "the response body is the SPA index.html" — kept identical so the two
 * files agree on what "looks like the lamad SPA shell" means.
 */
function looksLikeSpaIndexHtml(resp: AppResponse): boolean {
  const ct = resp.headers['content-type'] ?? '';
  if (!ct.includes('text/html')) return false;
  const html = resp.body.toString('utf-8');
  return html.includes('<app-root') || html.includes('ng-version') || html.includes('<base ');
}

/** HTTP-body analog of the loading/starting text heuristic steps/ui/delivery.steps.ts uses via Playwright. */
function looksLikeBootstrapPage(resp: AppResponse): boolean {
  const html = resp.body.toString('utf-8').toLowerCase();
  return html.includes('loading') || html.includes('starting');
}

// ---------------------------------------------------------------------------
// Given — preconditions
// ---------------------------------------------------------------------------

/**
 * Doorway-agnostic sibling of substrate-rea-replication.steps.ts's
 * "...is extracted and cached in doorway {string}" — same substrate check
 * (GET /apps/{slug}/index.html resolves), used by scenarios that don't name
 * a doorway id in this phrasing.
 */
Given(
  'the SPA bundle for {string} is extracted and cached',
  async function (this: E2EWorld, contentSlug: string) {
    const base = firstDoorwayUrl(this);
    const resp = await fetchApp(base, `/apps/${contentSlug}/index.html`);
    assert.equal(
      resp.status,
      200,
      `SPA bundle "${contentSlug}" not extracted on ${base} (status ${resp.status})`
    );
    const ct = resp.headers['content-type'] ?? '';
    assert.ok(
      ct.includes('text/html'),
      `SPA bundle "${contentSlug}" served as "${ct}" — expected text/html`
    );
  }
);

/**
 * Narrowed real precondition: a2o cannot force a genuine cold boot of the
 * doorway process from here (that needs an actual restart — out of scope on
 * a mesh another agent is mid-measuring). What IS falsifiable is that the
 * doorway process answers at all, which is what makes the /health/startup
 * poll this precedes meaningful; 503 (still warming) counts as reachable.
 */
Given('doorway {string} is starting up', async function (this: E2EWorld, doorwayId: string) {
  const base = firstDoorwayUrl(this);
  const resp = await fetchApp(base, '/health');
  assert.ok(
    resp.status === 200 || resp.status === 503,
    `doorway "${doorwayId}" at ${base}/health is unreachable (status ${resp.status})`
  );
});

// Gap: ROOT_APP_SLUG was removed post-B14 (doorway/doorway-service/src/
// server/http.rs, routes/health.rs) — root resolution now reads live EPR
// commitments, so there is no running-doorway config toggle left to flip to
// simulate "no root app configured" over HTTP.
Given(
  'doorway {string} has no ROOT_APP_SLUG configured',
  function (this: E2EWorld, _doorwayId: string) {
    return 'pending';
  }
);

/** Same substrate check as "...is extracted and cached" — different phrasing, same probe. */
Given(
  'the SPA for {string} was previously extracted and serving correctly',
  async function (this: E2EWorld, contentSlug: string) {
    const base = firstDoorwayUrl(this);
    const resp = await fetchApp(base, `/apps/${contentSlug}/index.html`);
    assert.equal(
      resp.status,
      200,
      `SPA "${contentSlug}" is not serving on ${base} (status ${resp.status})`
    );
  }
);

// Gap: simulating this needs an actual restart of a storage peer process.
// This mesh's processControl only exposes pause/resume (SIGSTOP/SIGCONT via
// requireFixturePeerPid), which does not drop in-memory state the way a real
// restart does, and there is no supervisor-relaunch mechanism this step can
// call without invoking `just mesh` tooling (forbidden) or corrupting the
// concurrent measurement this shared mesh is running.
Given(
  'the elohim-storage pod has restarted and lost in-memory extraction state',
  function (this: E2EWorld) {
    return 'pending';
  }
);

/**
 * Real check against the doorway's own /health/startup `warmup` block
 * (routes/health.rs startup_check — `{inProgress, attempts, maxAttempts,
 * completed, lastError, upstreams}`).
 */
Given(
  'doorway {string} has just completed warmup retry for {string}',
  async function (this: E2EWorld, doorwayId: string, slug: string) {
    const base = firstDoorwayUrl(this);
    const resp = await fetchApp(base, '/health/startup');
    assert.equal(resp.status, 200, `GET ${base}/health/startup returned ${resp.status}`);
    const body = JSON.parse(resp.body.toString('utf-8')) as Record<string, unknown>;
    const warmup = body['warmup'] as Record<string, unknown> | null | undefined;
    assert.ok(
      warmup,
      `doorway "${doorwayId}" /health/startup carries no warmup block — cannot confirm retry for "${slug}"`
    );
    assert.equal(
      warmup['completed'],
      true,
      `doorway "${doorwayId}" warmup.completed is not yet true (real value: ${JSON.stringify(warmup['completed'])})`
    );
  }
);

// Gap: the literal placeholder hashes ("sha256-old"/"sha256-new") are not
// realizable content addresses on this mesh — real blob hashes are bafkrei…
// CIDv1 (see steps/delivery.steps.ts CONTENT_ADDRESS_PATTERN), and there is
// no a2o-callable path to pin a running doorway to an arbitrary literal hash.
Given(
  'doorway {string} is serving the SPA from blobHash {string}',
  function (this: E2EWorld, _doorwayId: string, _blobHash: string) {
    return 'pending';
  }
);

// Gap: same placeholder-hash limitation as above — nothing in this mesh's
// projection cache is addressed by a literal "sha256-old" value to assert on.
Given(
  'the projection cache contains SPA assets from blobHash {string}',
  function (this: E2EWorld, _blobHash: string) {
    return 'pending';
  }
);

// ---------------------------------------------------------------------------
// When — requests
// ---------------------------------------------------------------------------

When(
  String.raw`a request is made for a content-hashed asset \(e.g. \/main.abc123.js)`,
  async function (this: E2EWorld) {
    await captureGet(this, '/main.abc123.js');
  }
);

When(String.raw`a request is made for \/index.html`, async function (this: E2EWorld) {
  await captureGet(this, '/index.html');
});

When(String.raw`a request is made for \/content\/manifesto`, async function (this: E2EWorld) {
  await captureGet(this, '/content/manifesto');
});

When(
  String.raw`a request is made for \/db\/content\/evolution-of-trust`,
  async function (this: E2EWorld) {
    await captureGet(this, '/db/content/evolution-of-trust');
  }
);

When(String.raw`Matthew navigates to \/threshold`, async function (this: E2EWorld) {
  await captureGet(this, '/threshold');
});

When(
  String.raw`Terrance navigates to \/apps\/evolution-of-trust\/index.html`,
  async function (this: E2EWorld) {
    await captureGet(this, '/apps/evolution-of-trust/index.html');
  }
);

When(String.raw`a request is made for \/health\/startup`, async function (this: E2EWorld) {
  await captureGet(this, '/health/startup');
});

When(String.raw`a request is made for \/some\/unknown\/path`, async function (this: E2EWorld) {
  await captureGet(this, '/some/unknown/path');
});

When(String.raw`a request is made for \/`, async function (this: E2EWorld) {
  await captureGet(this, '/');
});

// Gap: no re-seed/import pathway is callable from a2o step code — the
// elohim-import CLI that mints a new ZIP blob is out-of-band, and even if it
// were reachable the literal "sha256-new" is not a real content address.
When(
  String.raw`CI re-seeds {string} with a new ZIP blob \(blobHash {string})`,
  function (this: E2EWorld, _slug: string, _blobHash: string) {
    return 'pending';
  }
);

// Gap: no HTTP/admin surface was found that pushes a server-side content-
// update signal for a spa-bundle row (routes/admin*.rs); the only content-
// update signal wired in this codebase is the client-side Service Worker
// postMessage simulated in steps/ui/delivery.steps.ts, not a doorway-side push.
When(
  'doorway receives the content update signal for {string}',
  function (this: E2EWorld, _contentSlug: string) {
    return 'pending';
  }
);

// Gap: same placeholder-hash / no-mutation-path limitation as the Given steps above.
When(
  'the blobHash for {string} changes to {string}',
  function (this: E2EWorld, _contentSlug: string, _newBlobHash: string) {
    return 'pending';
  }
);

// Gap: no admin endpoint was found to force warmup-sentinel staleness
// detection; the only observable signal is the passive `warmup` block read
// via /health/startup (used by the "has just completed warmup retry" Given above).
When(
  'doorway {string} detects that the warmup sentinel is stale',
  function (this: E2EWorld, _doorwayId: string) {
    return 'pending';
  }
);

// ---------------------------------------------------------------------------
// Then — response assertions
// ---------------------------------------------------------------------------

/**
 * Dual-mode: the Background's browser-only "an unauthenticated request is
 * made for /" step (steps/ui/delivery.steps.ts) never populates responseStore
 * in HTTP mode (it returns 'pending' there), so this step falls back to its
 * own fresh GET of "/" when nothing was captured — a genuinely independent
 * probe rather than a claim about state nothing actually produced.
 */
Then('the response is the SPA index.html', async function (this: E2EWorld) {
  const resp = responseStore.get(this) ?? (await captureGet(this, '/'));
  const ct = resp.headers['content-type'] ?? '';
  assert.ok(ct.includes('text/html'), `Expected text/html, got "${ct}" — not an SPA index.html`);
  assert.ok(
    looksLikeSpaIndexHtml(resp),
    `Response does not look like an Angular SPA shell. First 200 chars: ` +
      `${resp.body.toString('utf-8').slice(0, 200)}`
  );
});

Then('the Cache-Control header is {string}', function (this: E2EWorld, expected: string) {
  const resp = responseStore.get(this);
  assert.ok(resp, NO_RESPONSE);
  const actual = resp.headers['cache-control'] ?? '';
  assert.equal(actual, expected, `Expected Cache-Control "${expected}" but got "${actual}"`);
});

Then('the request is proxied to elohim-storage', function (this: E2EWorld) {
  const resp = responseStore.get(this);
  assert.ok(resp, NO_RESPONSE);
  assert.ok(
    !looksLikeSpaIndexHtml(resp),
    'Response looks like the Angular SPA shell, not a proxied elohim-storage answer'
  );
  const ct = resp.headers['content-type'] ?? '';
  assert.ok(
    ct.includes('json'),
    `Expected an elohim-storage API answer (JSON content-type), got "${ct}"`
  );
});

Then('the response is not the SPA index.html', function (this: E2EWorld) {
  const resp = responseStore.get(this);
  assert.ok(resp, NO_RESPONSE);
  assert.ok(
    !looksLikeSpaIndexHtml(resp),
    `Response looks like the SPA index.html (content-type "${resp.headers['content-type'] ?? '<none>'}")`
  );
});

/**
 * Narrowed real check: /threshold proxies to the doorway-app operator bundle
 * (doorway/doorway-service/src/routes/threshold.rs). At the HTTP layer a 2xx
 * HTML response is the falsifiable signal available; it cannot distinguish
 * the operator dashboard's content from any other Angular shell by body
 * alone — the "does not intercept" step below carries the more specific claim.
 */
Then('the response is the operator dashboard', function (this: E2EWorld) {
  const resp = responseStore.get(this);
  assert.ok(resp, NO_RESPONSE);
  assert.ok(
    resp.status >= 200 && resp.status < 300,
    `/threshold returned ${resp.status}, expected 2xx`
  );
  const ct = resp.headers['content-type'] ?? '';
  assert.ok(ct.includes('text/html'), `/threshold served "${ct}", expected text/html`);
});

Then(
  String.raw`the SPA catch-all does not intercept the \/threshold route`,
  function (this: E2EWorld) {
    const resp = responseStore.get(this);
    assert.ok(resp, NO_RESPONSE);
    assert.ok(
      !looksLikeSpaIndexHtml(resp),
      '/threshold response looks like the lamad SPA shell — the root-app catch-all intercepted it'
    );
  }
);

Then('the response is the evolution-of-trust html5-app entry point', function (this: E2EWorld) {
  const resp = responseStore.get(this);
  assert.ok(resp, NO_RESPONSE);
  assert.equal(resp.status, 200, `/apps/evolution-of-trust/index.html returned ${resp.status}`);
  const ct = resp.headers['content-type'] ?? '';
  assert.ok(ct.includes('text/html'), `Expected text/html, got "${ct}"`);
  assert.ok(
    !looksLikeSpaIndexHtml(resp),
    'Response looks like the lamad Angular SPA shell, not the evolution-of-trust html5-app bundle'
  );
});

Then(
  String.raw`the SPA catch-all does not intercept the \/apps\/ route`,
  function (this: E2EWorld) {
    const resp = responseStore.get(this);
    assert.ok(resp, NO_RESPONSE);
    assert.ok(
      !looksLikeSpaIndexHtml(resp),
      '/apps/ response looks like the lamad SPA shell — the root-app catch-all intercepted it'
    );
  }
);

/**
 * Real check against the actual /health/startup shape (routes/health.rs
 * startup_check): each named section nests a boolean `ready`. "ready or
 * pending" in the feature's table is that two-state boolean.
 */
Then('the response is JSON with status fields:', function (this: E2EWorld, table: DataTable) {
  const resp = responseStore.get(this);
  assert.ok(resp, NO_RESPONSE);
  assert.equal(resp.status, 200, `/health/startup returned ${resp.status}`);
  const body = JSON.parse(resp.body.toString('utf-8')) as Record<string, unknown>;
  for (const row of table.hashes()) {
    const field = row['field'];
    assert.ok(field, 'DataTable row missing a "field" column');
    const section = body[field] as Record<string, unknown> | undefined;
    assert.ok(
      section && typeof section === 'object',
      `/health/startup has no "${field}" section — full body: ${JSON.stringify(body).slice(0, 300)}`
    );
    assert.equal(
      typeof section['ready'],
      'boolean',
      `/health/startup "${field}".ready is not a boolean: ${JSON.stringify(section)}`
    );
  }
});

Then(String.raw`the response is a redirect to \/threshold`, function (this: E2EWorld) {
  const resp = responseStore.get(this);
  assert.ok(resp, NO_RESPONSE);
  assert.ok(resp.status >= 300 && resp.status < 400, `Expected redirect (3xx), got ${resp.status}`);
  const location = resp.headers['location'] ?? '';
  assert.ok(
    location.includes('/threshold'),
    `Location header "${location}" does not point at /threshold`
  );
});

Then('the response is not the bootstrap page', function (this: E2EWorld) {
  const resp = responseStore.get(this);
  assert.ok(resp, NO_RESPONSE);
  assert.ok(
    !looksLikeBootstrapPage(resp),
    'Response looks like the bootstrap loading page, not a genuine 404'
  );
});

// Gap: no metrics/admin surface exposes a discrete "warmup retry initiated"
// event for this scenario to observe — only the passive completed/attempts
// counters in /health/startup's `warmup` block (see the Given step above),
// which cannot distinguish "just initiated" from "already in progress".
Then(
  'doorway initiates a warmup retry for ROOT_APP_SLUG {string}',
  function (this: E2EWorld, _slug: string) {
    return 'pending';
  }
);

// Gap: same observability limit as above — re-extraction is an internal
// storage-side effect with no HTTP signal this scenario can assert on
// without actually forcing the storage pod restart it depends on.
Then('elohim-storage re-extracts the SPA blob', function (this: E2EWorld) {
  return 'pending';
});

// Gap: depends on the same unobservable restart/re-extraction chain.
Then(
  'the root app becomes available again without operator intervention',
  function (this: E2EWorld) {
    return 'pending';
  }
);

/**
 * Real check, with the field-name drift documented up top: the feature was
 * written against the pre-B14 `rootApp` field; the live doorway reports
 * `rootProjection`. Checks the real field name first, falling back to the
 * historical one only so an older doorway build still matches — never the
 * reverse (a live-but-differently-named field must not read as absent).
 */
Then(String.raw`\/health\/startup shows rootApp: ready`, async function (this: E2EWorld) {
  const base = firstDoorwayUrl(this);
  const resp = await fetchApp(base, '/health/startup');
  assert.equal(resp.status, 200, `GET ${base}/health/startup returned ${resp.status}`);
  const body = JSON.parse(resp.body.toString('utf-8')) as Record<string, unknown>;
  const rootSection =
    (body['rootProjection'] as Record<string, unknown> | null | undefined) ??
    (body['rootApp'] as Record<string, unknown> | null | undefined);
  assert.ok(
    rootSection,
    `/health/startup carries neither "rootProjection" nor "rootApp": ${JSON.stringify(body).slice(0, 300)}`
  );
  assert.equal(
    rootSection['ready'],
    true,
    `root app is not ready yet: ${JSON.stringify(rootSection)}`
  );
});

// Gap: depends on the CI re-seed pathway the "CI re-seeds ..." When step
// above cannot exercise (no a2o-callable re-seed mechanism; placeholder hash).
Then(String.raw`subsequent requests for \/ serve the updated SPA`, function (this: E2EWorld) {
  return 'pending';
});

// Gap: same re-seed/placeholder-hash limitation.
Then(
  'doorway evicts the extracted SPA cache for blobHash {string}',
  function (this: E2EWorld, _blobHash: string) {
    return 'pending';
  }
);

// Gap: same re-seed/placeholder-hash limitation.
Then(
  'doorway re-extracts the SPA bundle from blobHash {string}',
  function (this: E2EWorld, _blobHash: string) {
    return 'pending';
  }
);

// Gap: same placeholder-hash limitation as the Given steps above — nothing
// in this mesh is actually tagged with a literal "sha256-old" blobHash.
Then(
  'all projection cache entries tagged with blobHash {string} are evicted',
  function (this: E2EWorld, _blobHash: string) {
    return 'pending';
  }
);

// Gap: depends on the same unstaged blobHash-change precondition.
Then('stale asset requests do not return old bundle files', function (this: E2EWorld) {
  return 'pending';
});
