/**
 * Step definitions for features/ssr/compose-serves-the-projected-app.feature.
 *
 * The compose story is HTTP-only: every assertion reads a raw response the
 * doorway produced, so nothing here drives a browser. The capture itself is
 * taken by the shared `the raw HTTP response for {string} is captured` step
 * (steps/ui/ssr-hydration.steps.ts), which stores a JSON blob under the
 * `_ssr_capture` key of `world.contentIds`. That key is the contract between
 * the capture step and every assertion step in the suite; this file reads it
 * the same way steps/ui/ssr-trace.steps.ts does.
 *
 * Two surfaces on the serving peer make the "did it render?" question
 * falsifiable from outside:
 *
 *   GET /health/startup   → `servedBundleHeads[]`, the slugs this peer is
 *                           actually serving bundles for. A route projected to
 *                           an app NOT in that list is exactly the
 *                           renderer-app-mismatch case.
 *   GET /admin/render-stats → the in-process RenderTraceSnapshot
 *                           ({total, rendered, ...}). A render that happened
 *                           increments `total`; a render-free skip does not.
 *                           Bracketing a repeat request between two snapshots
 *                           is how "no SSR render trace is recorded" becomes an
 *                           observation rather than an assumption.
 *
 * Route selection is env-overridable, matching the convention already used by
 * steps/ui/ssr-trace.steps.ts (E2E_SSR_EMPTY_ROUTE / E2E_SSR_STALL_ROUTE):
 *   E2E_SSR_MISMATCH_ROUTE — a route projected to an app this peer holds no
 *                            server bundle for (default /lamad/path/elohim-protocol)
 *   E2E_SSR_PROJECTED_APP  — the slug that route projects to (default lamad-spa)
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { request as undiciRequest } from 'undici';

import type { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Shared capture contract (see steps/ui/ssr-hydration.steps.ts)
// ---------------------------------------------------------------------------

const SSR_CAPTURE_KEY = '_ssr_capture';

interface SsrCapture {
  status: number;
  headers: Record<string, string>;
  body: string;
}

function storedCapture(world: E2EWorld): SsrCapture {
  const raw = world.contentIds.get(SSR_CAPTURE_KEY);
  assert.ok(
    raw,
    'No raw HTTP capture in the world — a "the raw HTTP response ... is captured" step must run first'
  );
  return JSON.parse(raw) as SsrCapture;
}

function doorwayUrl(world: E2EWorld): string {
  const registered = world.doorways.get('alpha');
  const url = registered?.url ?? process.env['E2E_DOORWAY_ALPHA'];
  assert.ok(url, 'No doorway "alpha" registered and E2E_DOORWAY_ALPHA is not set');
  return url.replace(/\/$/, '');
}

function mismatchRoute(): string {
  return process.env['E2E_SSR_MISMATCH_ROUTE'] ?? '/lamad/path/elohim-protocol';
}

function projectedApp(): string {
  return process.env['E2E_SSR_PROJECTED_APP'] ?? 'lamad-spa';
}

async function getJson(url: string): Promise<{ status: number; body: unknown }> {
  const { statusCode, body } = await undiciRequest(url, { method: 'GET' });
  const text = await body.text();
  let parsed: unknown = null;
  try {
    parsed = JSON.parse(text);
  } catch {
    parsed = text;
  }
  return { status: statusCode, body: parsed };
}

/** The slugs this peer reports it is actually serving a bundle for. */
async function servedBundleSlugs(base: string): Promise<string[]> {
  const { status, body } = await getJson(`${base}/health/startup`);
  assert.equal(status, 200, `GET /health/startup returned ${status}`);
  const heads = (body as Record<string, unknown>)['servedBundleHeads'];
  assert.ok(
    Array.isArray(heads),
    `/health/startup carries no servedBundleHeads array — got ${JSON.stringify(body).slice(0, 300)}`
  );
  return (heads as Record<string, unknown>[]).map(h => String(h['slug'] ?? ''));
}

/** The peer's cumulative render-trace total (renders it has actually run). */
async function renderTotal(base: string): Promise<number> {
  const { status, body } = await getJson(`${base}/admin/render-stats`);
  assert.equal(status, 200, `GET /admin/render-stats returned ${status}`);
  const total = (body as Record<string, unknown>)['total'];
  assert.ok(
    typeof total === 'number',
    `/admin/render-stats carries no numeric "total" — got ${JSON.stringify(body).slice(0, 300)}`
  );
  return total;
}

// ---------------------------------------------------------------------------
// Scenario: a route projected to an app the renderer does not serve
// ---------------------------------------------------------------------------

/**
 * The precondition the mismatch case rests on, read from the peer itself: the
 * bundles it serves do NOT include the app this route projects to. Asserting
 * this separately from the `x-ssr-skipped` header is the point — otherwise the
 * scenario could pass for the wrong reason (a peer serving no bundles at all
 * takes the renderer-absent path, which is a different seam).
 */
When(
  "the loaded SSR bundle serves a different app than the route's projection",
  async function (this: E2EWorld) {
    const base = doorwayUrl(this);
    const slugs = await servedBundleSlugs(base);
    assert.ok(
      slugs.length > 0,
      'this peer serves NO bundles at all — with nothing loaded the dispatch takes the ' +
        'renderer-absent path, not renderer-app-mismatch, so this scenario cannot be ' +
        'constructed here. Stage a server bundle (the mesh Prologue does this for the ' +
        'landing app) and re-run.'
    );
    assert.ok(
      !slugs.includes(projectedApp()),
      `this peer already serves a bundle for "${projectedApp()}" (served: ${slugs.join(', ')}), ` +
        'so the route is NOT a cross-app projection and no mismatch can occur. Set ' +
        'E2E_SSR_PROJECTED_APP/E2E_SSR_MISMATCH_ROUTE to a route this peer really cannot render.'
    );
  }
);

/**
 * Render-free means render-free: bracket a repeat of the same request between
 * two /admin/render-stats reads and require the cumulative render total not to
 * move. The wasted-render defect this guards is invisible in the response — the
 * body is the same CSR shell either way — so the only witness is the counter.
 */
Then('no SSR render trace is recorded for the request', async function (this: E2EWorld) {
  const base = doorwayUrl(this);
  const before = await renderTotal(base);
  const { statusCode } = await undiciRequest(`${base}${mismatchRoute()}`, {
    method: 'GET',
    headers: { accept: 'text/html,application/xhtml+xml' },
  });
  assert.equal(statusCode, 200, `repeat GET ${mismatchRoute()} returned ${statusCode}`);
  const after = await renderTotal(base);
  assert.equal(
    after,
    before,
    `the peer ran ${after - before} render(s) for a route it cannot serve ` +
      `(${mismatchRoute()}): /admin/render-stats total went ${before} → ${after}. ` +
      'That is the wrong-app render waste — the app-identity gate must shed BEFORE spending a render.'
  );
});

// ---------------------------------------------------------------------------
// Scenario: a projected app with its own server bundle composes its own selector
// ---------------------------------------------------------------------------

/**
 * The capability precondition for selector-agnosticism. Held (not failed) when
 * the peer serves no bundle for the projected app: multi-bundle renderer
 * selection is a staging fact, and a run whose Prologue staged only the landing
 * server bundle has nothing to prove here.
 */
Given("a server bundle for the route's projected app is loaded", async function (this: E2EWorld) {
  const base = doorwayUrl(this);
  const slugs = await servedBundleSlugs(base);
  if (!slugs.includes(projectedApp())) {
    // eslint-disable-next-line no-console
    console.log(
      `  ⏭️  SKIPPED (capability): this peer serves no server bundle for "${projectedApp()}" ` +
        `(served: ${slugs.join(', ') || '<none>'}). Selector-agnosticism cannot be observed until ` +
        "multi-bundle renderer selection stages that app's server bundle."
    );
    return 'skipped';
  }
  return undefined;
});

/**
 * The compose primitive derives the root tag from the rendered document, so the
 * proof is that the served body carries the projected app's own root element —
 * not <app-root>.
 */
Then(
  "the raw HTTP response body contains server-rendered markup for the projected app's root element",
  function (this: E2EWorld) {
    const capture = storedCapture(this);
    const app = projectedApp();
    // lamad-spa → <lamad-root>; the slug's leading segment is the element name.
    const rootTag = `${app.replace(/-spa$/, '')}-root`;
    assert.ok(
      new RegExp(String.raw`<${rootTag}[\s>]`, 'i').test(capture.body),
      `Expected the projected app's own root element <${rootTag}> in the composed body. ` +
        "Its absence means the compose step spliced a different app's shell (the cross-app " +
        `splice this story forbids).\nBody preview: ${capture.body.slice(0, 400)}`
    );
  }
);

// ---------------------------------------------------------------------------
// Scenario: a compose shed names the seam that failed
// ---------------------------------------------------------------------------

/**
 * Capture a route that is SSR-eligible (the EPR router dispatches it) but which
 * this peer must shed. The mismatch route is the one shed every mesh can
 * construct; E2E_SSR_SHED_ROUTE overrides it where a different seam is staged.
 */
When(
  'the raw HTTP response for an SSR-eligible route that sheds to CSR is captured',
  async function (this: E2EWorld) {
    const base = doorwayUrl(this);
    const path = process.env['E2E_SSR_SHED_ROUTE'] ?? mismatchRoute();
    const { statusCode, headers, body } = await undiciRequest(`${base}${path}`, {
      method: 'GET',
      headers: { accept: 'text/html,application/xhtml+xml' },
    });
    const bodyText = await body.text();
    const headerMap: Record<string, string> = {};
    for (const [k, v] of Object.entries(headers)) {
      headerMap[k] = Array.isArray(v) ? v.join(', ') : (v ?? '');
    }
    this.contentIds.set(
      SSR_CAPTURE_KEY,
      JSON.stringify({ status: statusCode, headers: headerMap, body: bodyText })
    );
  }
);

/**
 * "Present and not the old opaque value" is not the same as "names a seam": a
 * shed tagged `unknown` or `error` would satisfy both and still tell an
 * operator nothing. The shed vocabulary is closed — every value the compose
 * layer can emit is one of these — so the assertion checks membership, which is
 * what the scenario's title actually promises.
 */
const KNOWN_SHED_SEAMS = [
  'renderer-app-mismatch',
  'shell-no-projection',
  'shell-breaker-open',
  'shell-fetch-failed',
  'shell-root-missing',
  'render-root-missing',
  'render-root-unclosed',
  'auth-mode-not-supported',
  'error-backoff',
  'overflow',
];

Then(
  'the raw HTTP response header "x-ssr-skipped" names a known compose seam',
  function (this: E2EWorld) {
    const capture = storedCapture(this);
    const value = capture.headers['x-ssr-skipped'] ?? '';
    assert.ok(
      KNOWN_SHED_SEAMS.includes(value),
      `x-ssr-skipped is "${value}", which is not one of the typed compose seams ` +
        `[${KNOWN_SHED_SEAMS.join(', ')}]. A shed that answers with a value outside the ` +
        'vocabulary is the opacity defect again under a new string — the header must name a ' +
        'seam an operator can act on, not merely differ from the old one.'
    );
  }
);
