/**
 * Reach gate step definitions — earned-reach floor + anonymous access verification.
 *
 * Feature: features/auth/reach-commons.feature
 *
 * These steps verify that content holding an authored commons/public reach grade
 * is accessible to anonymous clients without authentication. The gate in
 * elohim-storage's http.rs allows anonymous access when reach == "commons" ||
 * reach == "public"; these steps exercise that path end-to-end through the doorway.
 *
 * Response capture is shared with steps/delivery.steps.ts via the exported
 * responseStore WeakMap, so the existing `Then the response status is {int}`
 * step resolves assertions written here.
 */

import { strict as assert } from 'node:assert';

import { When, Then } from '@cucumber/cucumber';

import { E2EWorld } from '../../src/framework/world.js';
import { fetchApp, responseStore } from '../delivery.steps.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const NO_DOORWAY = 'No doorway registered — run "Given doorway ... at ..." step first';
const NO_RESPONSE = 'No response captured — run a "When an anonymous client GETs" step first';

function firstDoorwayUrl(world: E2EWorld): string {
  const doorway = [...world.doorways.values()][0];
  assert.ok(doorway, NO_DOORWAY);
  return doorway.url;
}

// ---------------------------------------------------------------------------
// When — anonymous client fetches content by id
// ---------------------------------------------------------------------------

/**
 * Issue an unauthenticated GET against /db/content/{id} on the first
 * registered doorway. No Authorization header is sent.
 *
 * Writes to the shared responseStore so the existing
 * `Then('the response status is {int}')` step in delivery.steps.ts resolves.
 */
When(
  'an anonymous client GETs content {string}',
  async function (this: E2EWorld, contentId: string) {
    const base = firstDoorwayUrl(this);
    const resp = await fetchApp(base, `/db/content/${contentId}`);
    responseStore.set(this, resp);
  }
);

// ---------------------------------------------------------------------------
// Then — 403 reach body assertions
// ---------------------------------------------------------------------------

/**
 * Parse the JSON 403 body and assert the requiredReach field.
 * Expected shape: { "error": "Authentication required", "requiredReach": "<level>" }
 */
Then('the 403 body requiredReach is {string}', function (this: E2EWorld, expectedReach: string) {
  const resp = responseStore.get(this);
  assert.ok(resp, NO_RESPONSE);
  assert.equal(
    resp.status,
    403,
    `Expected a 403 response to inspect requiredReach, got ${resp.status}`
  );
  let body: Record<string, unknown>;
  try {
    body = JSON.parse(resp.body.toString('utf-8')) as Record<string, unknown>;
  } catch {
    assert.fail(`Response body is not valid JSON: ${resp.body.toString('utf-8').slice(0, 200)}`);
  }
  assert.equal(
    body['requiredReach'],
    expectedReach,
    `Expected requiredReach "${expectedReach}", got "${String(body['requiredReach'])}". Full body: ${JSON.stringify(body)}`
  );
});
