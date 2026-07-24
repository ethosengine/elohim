/**
 * Guide-star convergence judge: two doorways must eventually testify the same
 * commitment-backed holder footprint for one commons EPR.
 */

import { strict as assert } from 'node:assert';

import { Given, Then } from '@cucumber/cucumber';

import {
  loadHouseholdMeshFixture,
  normalizeHouseholdFootprint,
  requireFixtureDoorwayUrl,
} from '../src/framework/fixtures/household-mesh.js';
import { retry } from '../src/framework/utils/retry.js';
import { E2EWorld } from '../src/framework/world.js';

const COMMONS_EPR_KEY = 'convergence:commons-epr';

async function householdSnapshot(
  doorwayUrl: string,
  contentId: string
): Promise<Record<string, unknown>> {
  const path = `/api/v1/resilience/${encodeURIComponent(contentId)}/household`;
  const response = await fetch(`${doorwayUrl}${path}`, {
    signal: AbortSignal.timeout(10_000),
  });
  const body = await response.text();
  assert.ok(response.ok, `GET ${doorwayUrl}${path} failed: ${response.status} ${body}`);
  try {
    return JSON.parse(body) as Record<string, unknown>;
  } catch {
    assert.fail(`GET ${doorwayUrl}${path} did not return JSON: ${body.slice(0, 240)}`);
  }
}

Given('the household fixture names a commitment-backed commons EPR', function (this: E2EWorld) {
  const fixture = loadHouseholdMeshFixture();
  const contentId = fixture.commonsEprId;
  assert.ok(contentId, 'set E2E_COMMONS_EPR_ID or commonsEprId in the household fixture');
  this.contentIds.set(COMMONS_EPR_KEY, contentId);
});

Then(
  'within {int} seconds doorways {string} and {string} testify the same household footprint',
  { timeout: 70_000 },
  async function (
    this: E2EWorld,
    seconds: number,
    firstDoorwayId: string,
    secondDoorwayId: string
  ) {
    const contentId = this.contentIds.get(COMMONS_EPR_KEY);
    assert.ok(contentId, 'the household fixture did not name a commons EPR');

    const fixture = loadHouseholdMeshFixture();
    const firstUrl =
      this.doorways.get(firstDoorwayId)?.url ?? requireFixtureDoorwayUrl(fixture, firstDoorwayId);
    const secondUrl =
      this.doorways.get(secondDoorwayId)?.url ?? requireFixtureDoorwayUrl(fixture, secondDoorwayId);

    await retry(
      async () => {
        const [firstSnapshot, secondSnapshot] = await Promise.all([
          householdSnapshot(firstUrl, contentId),
          householdSnapshot(secondUrl, contentId),
        ]);
        const first = normalizeHouseholdFootprint(firstSnapshot, contentId);
        const second = normalizeHouseholdFootprint(secondSnapshot, contentId);
        assert.deepEqual(
          second,
          first,
          `${secondDoorwayId} and ${firstDoorwayId} disagree for ${contentId}\n` +
            `${firstDoorwayId}: ${JSON.stringify(first)}\n` +
            `${secondDoorwayId}: ${JSON.stringify(second)}`
        );
      },
      {
        maxAttempts: 120,
        initialDelayMs: 500,
        backoffFactor: 1.2,
        maxDelayMs: 3_000,
        timeoutMs: seconds * 1000,
      }
    );
  }
);
