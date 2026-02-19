/**
 * Federation step definitions — registration, content CRUD, cross-doorway sync.
 */

import { Given, When, Then } from '@cucumber/cucumber';
import { strict as assert } from 'node:assert';
import { randomUUID } from 'node:crypto';
import { E2EWorld } from '../src/framework/world.js';
import { Human } from '../src/framework/human.js';
import { BrowserDevice } from '../src/framework/devices/browser-device.js';
import { waitForContentByTags } from '../src/framework/assertions/content-sync.js';

/**
 * Generate unique test credentials so parallel runs don't collide.
 */
function testCredentials(name: string) {
  const runId = randomUUID().slice(0, 8);
  return {
    identifier: `e2e-${name.toLowerCase()}-${runId}@test.elohim.host`,
    password: `E2ePass!${runId}`,
    displayName: `${name} (E2E ${runId})`,
  };
}

/**
 * Register a human on a specific doorway.
 */
Given(
  'human {string} is registered on doorway {string}',
  async function (this: E2EWorld, humanName: string, doorwayId: string) {
    const doorway = this.getDoorway(doorwayId);
    const creds = testCredentials(humanName);
    const human = new Human(humanName, creds);

    const device = new BrowserDevice(`${humanName}-browser`, doorway.url);
    human.addDevice(device);

    const auth = await device.register({
      identifier: creds.identifier,
      password: creds.password,
      displayName: creds.displayName,
    });

    human.agentPubKey = auth.agentPubKey;
    human.humanId = auth.humanId;
    human.setToken(doorwayId, auth.token);

    this.addHuman(humanName, human);
  },
);

/**
 * Create content on a doorway. The content alias is stored for later assertions.
 * Tags the content with a unique E2E marker for cross-doorway discovery.
 */
When(
  '{word} creates content {string} on doorway {string}',
  async function (this: E2EWorld, humanName: string, contentAlias: string, doorwayId: string) {
    const human = this.getHuman(humanName);
    const doorway = this.getDoorway(doorwayId);

    const device = human.devices[0] as BrowserDevice;
    assert.ok(device, `${humanName} has no device`);

    const runTag = `e2e-run-${randomUUID().slice(0, 8)}`;
    this.contentIds.set(`${contentAlias}:tag`, runTag);

    const content = await device.client.createContent({
      contentType: 'article',
      title: contentAlias,
      description: `E2E test content created by ${humanName} on ${doorwayId}`,
      content: `This is automated test content for federation validation.`,
      contentFormat: 'text',
      tags: ['e2e', 'federation', runTag],
    });

    const id = (content as Record<string, unknown>).id as string;
    this.contentIds.set(contentAlias, id);
  },
);

/**
 * Assert that content is visible on another doorway within a timeout.
 * Uses tag-based search since content IDs may differ across federated instances.
 */
Then(
  '{word} should see content {string} on doorway {string} within {int} seconds',
  async function (
    this: E2EWorld,
    humanName: string,
    contentAlias: string,
    doorwayId: string,
    timeoutSeconds: number,
  ) {
    const human = this.getHuman(humanName);
    const doorway = this.getDoorway(doorwayId);
    const runTag = this.contentIds.get(`${contentAlias}:tag`);
    assert.ok(runTag, `No run tag found for content "${contentAlias}"`);

    // Use the human's token if they have one for this doorway, otherwise use an
    // unauthenticated client (cache endpoints may be public).
    const token = human.getToken(doorwayId);
    if (token) doorway.client.setToken(token);

    const results = await waitForContentByTags(doorway.client, ['e2e', 'federation', runTag], {
      timeoutMs: timeoutSeconds * 1000,
      initialDelayMs: 2000,
      maxDelayMs: 10_000,
    });

    assert.ok(results.length > 0, `Content "${contentAlias}" not found on doorway "${doorwayId}" within ${timeoutSeconds}s`);

    const match = results.find(
      (r) => (r as Record<string, unknown>).title === contentAlias,
    );
    assert.ok(match, `Content with title "${contentAlias}" not found in results on doorway "${doorwayId}"`);
  },
);
