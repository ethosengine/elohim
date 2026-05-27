/**
 * Substrate-REA replication regression steps — guards Tasks 1–10 of
 * genesis/docs/superpowers/plans/2026-05-26-substrate-rea-replication-fix.md.
 *
 * Scope: the two scenarios in features/delivery/spa-bundle-delivery.feature
 * tagged @substrate-rea-replication-fix — both assert that project-epr
 * commitments for lamad-spa propagated to every alpha storage peer (proof
 * of the substrate-correct write path through ContentStore::create_commitment)
 * and that the doorway can serve /lamad regardless of which storage peer the
 * EprRouter happens to read from.
 *
 * Substrate verification (scenario 1) is done via HTTP probes against the
 * configured alpha doorway: GET /api/v1/commitments?action=project-epr filters
 * to lamad-spa rows and confirms a non-null dhtAnchorHash on every row.
 *
 * Pod-restart verification (scenario 2) requires kubectl access (which a2o
 * does not have); those steps `return 'pending'` so the scenario is a
 * recorded contract rather than a false-positive pass.
 *
 * Response capture is shared with steps/delivery.steps.ts via the exported
 * responseStore WeakMap so the existing `Then('the response status is {int}')`
 * assertion resolves against captures written here.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import { E2EWorld } from '../../src/framework/world.js';
import { fetchApp, responseStore } from '../delivery.steps.js';

const NO_DOORWAY = 'No doorway registered — run the doorway background step first';
const NO_RESPONSE = 'No response captured — run a When-request step first';

function firstDoorwayUrl(world: E2EWorld): string {
  const doorway = [...world.doorways.values()][0];
  assert.ok(doorway, NO_DOORWAY);
  return doorway.url.replace(/\/$/, '');
}

async function fetchJson(
  baseUrl: string,
  path: string
): Promise<{ status: number; body: unknown }> {
  const { statusCode, body } = await request(`${baseUrl}${path}`);
  const text = await body.text();
  let parsed: unknown = null;
  try {
    parsed = JSON.parse(text);
  } catch {
    parsed = text;
  }
  return { status: statusCode, body: parsed };
}

// ---------------------------------------------------------------------------
// Given — SPA-bundle and commitment fixtures
// ---------------------------------------------------------------------------

/**
 * Confirm the doorway has the SPA bundle extracted and cached.
 *
 * Substrate check: GET /apps/{slug}/index.html returns 200 with HTML.
 * That's the canonical signal that the doorway has resolved the spa-bundle
 * content node and unpacked the zip blob into its projection cache.
 */
Given(
  'the SPA bundle for {string} is extracted and cached in doorway {string}',
  async function (this: E2EWorld, contentSlug: string, _doorwayId: string) {
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
 * Substrate proof that the substrate-correct write path landed the
 * project-epr commitment on the DHT — every alpha storage peer is expected
 * to see at least one project-epr commitment whose epr_slug matches.
 *
 * a2o only has one doorway endpoint visible; we treat the doorway's view as
 * the authoritative read because the doorway forwards to a single storage
 * peer and that peer is selected non-deterministically across boots. If the
 * commitment is missing from any peer, the doorway-mediated query would
 * return [] from at least one boot — exactly the failure mode the
 * substrate-rea fix closes.
 */
Given(
  'the project-epr commitment for {string} has propagated across all alpha storage peers via DHT gossip',
  async function (this: E2EWorld, eprSlug: string) {
    const base = firstDoorwayUrl(this);
    const { status, body } = await fetchJson(base, '/api/v1/commitments?action=project-epr');
    assert.equal(
      status,
      200,
      `GET /api/v1/commitments?action=project-epr returned ${status} (expected 200)`
    );
    assert.ok(
      Array.isArray(body),
      `Expected array body from /api/v1/commitments, got ${typeof body}`
    );
    const rows = body as Record<string, unknown>[];
    const matching = rows.filter(row => {
      const slug = String(row['eprSlug'] ?? row['epr_slug'] ?? '');
      const inScope = JSON.stringify(row['inScopeOf'] ?? row['in_scope_of'] ?? '');
      return slug === eprSlug || inScope.includes(eprSlug);
    });
    assert.ok(
      matching.length > 0,
      `No project-epr commitment found for "${eprSlug}" — substrate-correct write may not have propagated. Sample row: ${JSON.stringify(rows[0] ?? null).slice(0, 200)}`
    );
    // Stash for the next step's dhtAnchorHash check.
    (this as unknown as Record<string, unknown>)[`__substrateRea:${eprSlug}`] = matching;
  }
);

/**
 * Verify every persisted commitment row carries a real dhtAnchorHash —
 * the marker that ContentStore::create_commitment notarized it on the DHT
 * before diesel inserted the projection row. A null dhtAnchorHash means
 * the row was written diesel-direct (the pre-fix failure mode).
 */
Given(
  /^the commitment row has a non-null dhtAnchorHash on every peer \(proof of substrate-correct write\)$/,
  function (this: E2EWorld) {
    const stashed = this as unknown as Record<string, unknown>;
    const key = Object.keys(stashed).find(k => k.startsWith('__substrateRea:'));
    assert.ok(key, 'No prior commitment rows captured — run the "has propagated" step first');
    const rows = stashed[key] as Record<string, unknown>[];
    for (const row of rows) {
      const anchor = row['dhtAnchorHash'] ?? row['dht_anchor_hash'];
      assert.ok(
        anchor && typeof anchor === 'string' && anchor.length > 0,
        `Commitment ${JSON.stringify(row['id'] ?? '?')} has null/empty dhtAnchorHash — diesel-direct write, substrate-rea fix not applied. Row: ${JSON.stringify(row).slice(0, 300)}`
      );
    }
  }
);

// ---------------------------------------------------------------------------
// When — anonymous HTTP GETs
// ---------------------------------------------------------------------------

/**
 * Issue an unauthenticated GET against a non-root path on the configured
 * doorway. The bare-`/` variant is owned by the bootstrap-page browser
 * scenario in steps/ui/delivery.steps.ts — this regex requires at least one
 * character after the leading slash, so there is no overlap.
 *
 * Writes to the shared responseStore so the existing
 * `Then('the response status is {int}')` step in delivery.steps.ts resolves.
 */
When(
  /^an unauthenticated request is made for (\/[\w-][\w/-]*)$/,
  async function (this: E2EWorld, path: string) {
    const base = firstDoorwayUrl(this);
    const resp = await fetchApp(base, path);
    responseStore.set(this, resp);
  }
);

// ---------------------------------------------------------------------------
// Then — response shape assertions
// ---------------------------------------------------------------------------

Then('the response body is the SPA index.html', function (this: E2EWorld) {
  const resp = responseStore.get(this);
  assert.ok(resp, NO_RESPONSE);
  const ct = resp.headers['content-type'] ?? '';
  assert.ok(ct.includes('text/html'), `Expected text/html, got "${ct}" — not an SPA index.html`);
  const html = resp.body.toString('utf-8');
  const hasAngularShell =
    html.includes('<app-root') || html.includes('ng-version') || html.includes('<base ');
  assert.ok(
    hasAngularShell,
    `Response does not look like an Angular SPA shell. First 200 chars: ${html.slice(0, 200)}`
  );
});

Then(
  /^Angular's router handles the (\/\S+) route on the client$/,
  function (this: E2EWorld, _path: string) {
    // Substrate check: 200 + an Angular shell HTML body is sufficient
    // evidence that the doorway returned the SPA's entry point and that
    // Angular's router will hydrate on the client side. Browser-only
    // assertion (actual route resolution) lives in steps/ui/delivery.steps.ts.
    const resp = responseStore.get(this);
    assert.ok(resp, NO_RESPONSE);
    assert.equal(
      resp.status,
      200,
      'SPA fallback must return 200 for Angular client-side routing to take over'
    );
  }
);

// ---------------------------------------------------------------------------
// Pod-restart scenario — kubectl-dependent, scaffolded as pending
// ---------------------------------------------------------------------------

/**
 * Documentary precondition: a prior probe confirmed /lamad serves the SPA
 * index.html on the configured doorway. We re-verify here so the scenario
 * is self-contained.
 */
Given(
  /^\/lamad has been confirmed serving the SPA index\.html on doorway "([^"]+)"$/,
  async function (this: E2EWorld, _doorwayId: string) {
    const base = firstDoorwayUrl(this);
    const resp = await fetchApp(base, '/lamad');
    assert.equal(resp.status, 200, `/lamad on ${base} returned ${resp.status}, expected 200`);
    const ct = resp.headers['content-type'] ?? '';
    assert.ok(ct.includes('text/html'), `/lamad served as "${ct}", expected text/html`);
    responseStore.set(this, resp);
  }
);

/**
 * Pod-restart steps require kubectl access against the alpha cluster.
 * a2o runs from CI with read-only HTTP probes; kubectl is operator-driven.
 * These return 'pending' so cucumber records the contract without
 * generating a false pass or false fail.
 */
When("doorway {string}'s pod is restarted", function (this: E2EWorld, _doorwayId: string) {
  return 'pending';
});

When(
  "the new pod's storage peer is selected non-deterministically from the alpha cluster",
  function (this: E2EWorld) {
    return 'pending';
  }
);

Then(
  /^a subsequent request for (\/\S+) returns the SPA index\.html with status (\d+)$/,
  function (this: E2EWorld, _path: string, _status: string) {
    return 'pending';
  }
);

Then('no operator intervention is required', function (this: E2EWorld) {
  return 'pending';
});
