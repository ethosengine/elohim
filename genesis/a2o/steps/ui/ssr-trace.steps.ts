/**
 * SSR Render-Trace Step Definitions — verify the render trace distinguishes a
 * stalled upstream from a legitimately-empty render.
 *
 * Covers:
 *   - genesis/a2o/features/ssr/render-trace-distinguishes-stall-from-empty.feature
 *
 * The contract being tested (HTTP-level projection of elohim-render's trace):
 *   - x-ssr-terminal carries the terminal classification: rendered | rendered-empty
 *     | stalled | timed-out | errored.
 *   - A truthful empty (upstream 404 / []) → `rendered-empty`, never `stalled`.
 *   - A stalled upstream → `stalled`, with the CSR shell served FAST (the per-fetch
 *     soft-deadline falls back well before the hard wall-time) rather than hanging.
 *
 * Reuses the raw-capture mechanism from ssr-hydration.steps.ts via the shared
 * Cucumber World key `_ssr_capture` (step files share one World instance).
 *
 * Fault injection (so the stall scenario is exercisable on a test deployment):
 *   - Empty route: E2E_SSR_EMPTY_ROUTE (default a guaranteed-missing concept slug).
 *     No server hook needed — a missing concept's upstream fetch 404s naturally.
 *   - Stall route: E2E_SSR_STALL_ROUTE, served by a doorway started with
 *     DOORWAY_SSR_FAULT_STALL_PATH set to the same path (env-gated, off by default).
 *
 * Tag: @wip — needs a doorway with SSR_BUNDLE_PATH set (and, for the stall
 * scenario, DOORWAY_SSR_FAULT_STALL_PATH). The deterministic mechanism is proven
 * by elohim-render's Rust integration tests; this feature guards the HTTP contract.
 */

import { strict as assert } from 'node:assert';

import { When, Then } from '@cucumber/cucumber';

import { request as undiciRequest } from 'undici';

import { E2EWorld } from '../../src/framework/world.js';

// Shared with ssr-hydration.steps.ts — same World instance, same key.
const SSR_CAPTURE_KEY = '_ssr_capture';

/** Captured raw SSR response (+ optional client-observed duration). */
interface SsrCapture {
  status: number;
  headers: Record<string, string>;
  body: string;
  durationMs?: number;
}

/** Conservative ceiling proving a fast fallback rather than a wall-time hang.
 * Doorway's SSR wall-time is 60s; the soft-deadline fallback returns in ~1-2s,
 * so anything comfortably under this bound proves the stall did not hang. */
const FAST_FALLBACK_CEILING_MS = 15_000;

function resolveDoorwayUrl(world: E2EWorld): string {
  for (const [, entry] of world.doorways) {
    return entry.url;
  }
  const fallback = process.env['E2E_DOORWAY_ALPHA'];
  assert.ok(fallback, 'No doorway registered and E2E_DOORWAY_ALPHA not set');
  return fallback;
}

function storedCapture(world: E2EWorld): SsrCapture {
  const raw = world.contentIds.get(SSR_CAPTURE_KEY);
  assert.ok(raw, 'No SSR capture found — capture a raw HTTP response first');
  return JSON.parse(raw) as SsrCapture;
}

async function captureRaw(world: E2EWorld, path: string): Promise<void> {
  const fullUrl = `${resolveDoorwayUrl(world)}${path}`;
  const startedAt = Date.now();
  const { statusCode, headers, body } = await undiciRequest(fullUrl, {
    method: 'GET',
    headers: { accept: 'text/html,application/xhtml+xml' },
  });
  const bodyText = await body.text();
  const durationMs = Date.now() - startedAt;

  const headerMap: Record<string, string> = {};
  for (const [k, v] of Object.entries(headers)) {
    headerMap[k] = Array.isArray(v) ? v.join(', ') : (v ?? '');
  }

  const capture: SsrCapture = {
    status: statusCode,
    headers: headerMap,
    body: bodyText,
    durationMs,
  };
  world.contentIds.set(SSR_CAPTURE_KEY, JSON.stringify(capture));
}

function headerValue(capture: SsrCapture, headerName: string): string | undefined {
  const lower = headerName.toLowerCase();
  return Object.prototype.hasOwnProperty.call(capture.headers, lower)
    ? capture.headers[lower]
    : undefined;
}

// ---------------------------------------------------------------------------
// Fault-route capture steps
// ---------------------------------------------------------------------------

/**
 * Capture an SSR route whose upstream content is genuinely absent. The render's
 * data fetch 404s, so the trace must classify `rendered-empty` (a truthful empty,
 * not a stall). No server fault hook needed — a missing concept does this naturally.
 *
 * Example: When the raw HTTP response for an SSR route whose upstream returns no content is captured
 */
When(
  'the raw HTTP response for an SSR route whose upstream returns no content is captured',
  async function (this: E2EWorld) {
    const path = process.env['E2E_SSR_EMPTY_ROUTE'] ?? '/lamad/concept/__a2o_nonexistent_concept__';
    await captureRaw(this, path);
  }
);

/**
 * Capture an SSR route whose upstream never settles. Served by a doorway started
 * with DOORWAY_SSR_FAULT_STALL_PATH matching this route; the per-fetch soft-deadline
 * converts the hang into a `stalled` terminal and a fast CSR-shell fallback.
 *
 * Example: When the raw HTTP response for an SSR route whose upstream stalls is captured
 */
When(
  'the raw HTTP response for an SSR route whose upstream stalls is captured',
  async function (this: E2EWorld) {
    const path = process.env['E2E_SSR_STALL_ROUTE'] ?? '/lamad/concept/__a2o_stall__';
    await captureRaw(this, path);
  }
);

// ---------------------------------------------------------------------------
// Header assertions (exact value, negation)
// ---------------------------------------------------------------------------

/**
 * Example: Then the raw HTTP response header "x-ssr-terminal" is "rendered-empty"
 */
Then(
  'the raw HTTP response header {string} is {string}',
  function (this: E2EWorld, headerName: string, expected: string) {
    const capture = storedCapture(this);
    const actual = headerValue(capture, headerName);
    assert.equal(
      actual,
      expected,
      `Expected header "${headerName}" to be "${expected}" but got "${actual ?? '<absent>'}".\n` +
        `Present headers: ${Object.keys(capture.headers).join(', ')}`
    );
  }
);

/**
 * Example: Then the raw HTTP response header "x-ssr-terminal" is not "stalled"
 */
Then(
  'the raw HTTP response header {string} is not {string}',
  function (this: E2EWorld, headerName: string, rejected: string) {
    const capture = storedCapture(this);
    const actual = headerValue(capture, headerName);
    assert.notEqual(
      actual,
      rejected,
      `Expected header "${headerName}" NOT to be "${rejected}" but it was.`
    );
  }
);

// ---------------------------------------------------------------------------
// Body + timing assertions
// ---------------------------------------------------------------------------

/**
 * The CSR shell fallback is the `<app-root></app-root>` shell doorway serves when
 * SSR falls back (here: after a stall) so the client can hydrate.
 *
 * Example: Then the raw HTTP response body contains the CSR shell fallback
 */
Then('the raw HTTP response body contains the CSR shell fallback', function (this: E2EWorld) {
  const capture = storedCapture(this);
  const hasEmptyAppRoot =
    capture.body.includes('<app-root></app-root>') ||
    /<app-root>\s*<\/app-root>/.test(capture.body);
  assert.ok(
    hasEmptyAppRoot,
    'Expected the CSR shell fallback (empty <app-root>) in the response body.\n' +
      `Body preview (first 300 chars): ${capture.body.slice(0, 300)}`
  );
});

/**
 * Proves the stall fell back FAST — the soft-deadline returned well before the
 * hard wall-time, rather than the visitor hanging on a blank document.
 *
 * Example: Then the SSR fallback arrived before the hard wall-time limit
 */
Then('the SSR fallback arrived before the hard wall-time limit', function (this: E2EWorld) {
  const capture = storedCapture(this);
  assert.ok(
    typeof capture.durationMs === 'number',
    'No capture duration recorded — use a fault-route capture step that records timing.'
  );
  assert.ok(
    capture.durationMs < FAST_FALLBACK_CEILING_MS,
    `Expected the fallback within ${FAST_FALLBACK_CEILING_MS}ms (fast soft-deadline) ` +
      `but it took ${capture.durationMs}ms — a stall must not ride to the hard wall-time.`
  );
});
