/**
 * EPR content-perspective facing — /raw inspector steps.
 *
 * Exercises GET /api/v1/epr/{cid}/raw (the EPR-content facing Slice 2 inspector)
 * against the configured alpha doorway. Household-floor: reads the seeded
 * landing atom on matthew's storage through the doorway projection — no
 * cross-node discovery, so no shem dependency.
 *
 * Charter: genesis/docs/superpowers/specs/2026-06-19-epr-content-perspective-facing-lens-design.md §6 Slice 2.
 *
 * Capture is shared with steps/delivery.steps.ts via the exported `fetchApp` +
 * `responseStore` so the existing `Then('the response status is {int}')`
 * assertion resolves against the capture written here.
 */

import { strict as assert } from 'node:assert';

import { When, Then } from '@cucumber/cucumber';

import { E2EWorld } from '../../src/framework/world.js';
import { fetchApp, responseStore } from '../delivery.steps.js';

const NO_DOORWAY = 'No doorway registered — run the doorway background step first';
const NO_RESPONSE = 'No response captured — run the fetch step first';

function firstDoorwayUrl(world: E2EWorld): string {
  const doorway = [...world.doorways.values()][0];
  assert.ok(doorway, NO_DOORWAY);
  return doorway.url.replace(/\/$/, '');
}

function capturedBody(world: E2EWorld): Record<string, unknown> {
  const resp = responseStore.get(world);
  assert.ok(resp, NO_RESPONSE);
  const text = resp.body.toString('utf-8');
  let parsed: unknown = null;
  try {
    parsed = JSON.parse(text);
  } catch {
    parsed = null;
  }
  assert.ok(
    parsed && typeof parsed === 'object',
    `Expected a JSON object body, got: ${text.slice(0, 160)}`
  );
  return parsed as Record<string, unknown>;
}

When(
  'I fetch the EPR raw inspector for atom {string}',
  async function (this: E2EWorld, atom: string) {
    const base = firstDoorwayUrl(this);
    const resp = await fetchApp(base, `/api/v1/epr/${encodeURIComponent(atom)}/raw`);
    responseStore.set(this, resp);
  }
);

Then('the raw inspector echoes a content-addressed cid', function (this: E2EWorld) {
  const cid = capturedBody(this).cid;
  assert.ok(
    typeof cid === 'string' && cid.length > 0,
    `Expected a non-empty content-addressed cid, got ${JSON.stringify(cid)}`
  );
});

Then('the raw inspector reports a governance coupling leg', function (this: E2EWorld) {
  const coupling = capturedBody(this).coupling as Record<string, unknown> | undefined;
  assert.ok(coupling && typeof coupling === 'object', 'Expected a coupling object on the inspector');
  const governance = coupling.governance;
  assert.ok(
    typeof governance === 'string' && governance.length > 0,
    `Expected a populated governance coupling leg, got ${JSON.stringify(governance)}`
  );
});

Then(
  'the raw inspector reports a neighborhood degree of at least {int}',
  function (this: E2EWorld, min: number) {
    const degree = capturedBody(this).neighborhoodDegree;
    assert.ok(
      typeof degree === 'number' && Number.isInteger(degree) && degree >= min,
      `Expected an integer neighborhoodDegree >= ${min}, got ${JSON.stringify(degree)}`
    );
  }
);

Then('the raw inspector lists the atoms that couple back to it', function (this: E2EWorld) {
  const reversePartOf = capturedBody(this).reversePartOf;
  assert.ok(
    Array.isArray(reversePartOf),
    `Expected reversePartOf to be an array, got ${typeof reversePartOf}`
  );
});
