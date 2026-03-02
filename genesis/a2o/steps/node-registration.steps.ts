/**
 * Node registration step definitions — doorway self-registration and admin node queries.
 *
 * Matthew's doorway IS his node. These steps verify that the doorway registers
 * itself with the orchestrator on startup so the admin dashboard has something
 * to display.
 */

import { strict as assert } from 'node:assert';

import { When, Then } from '@cucumber/cucumber';

import { BrowserDevice } from '../src/framework/devices/browser-device.js';
import { E2EWorld } from '../src/framework/world.js';

import type { StatusResponse, AdminNodesResponse } from '../src/framework/api/doorway-client.js';

// ---------------------------------------------------------------------------
// Status check
// ---------------------------------------------------------------------------

let lastStatus: StatusResponse | undefined;

When('{word} checks doorway status', async function (this: E2EWorld, humanName: string) {
  const human = this.getHuman(humanName);
  const device = human.devices[0] as BrowserDevice;
  assert.ok(device, `${humanName} has no device`);

  lastStatus = await device.client.status();
});

Then('the status should include an orchestrator section', function () {
  assert.ok(lastStatus, 'No status response available');
  assert.ok(
    'orchestrator' in lastStatus,
    'Status response does not include an orchestrator section'
  );
});

// ---------------------------------------------------------------------------
// Admin nodes queries
// ---------------------------------------------------------------------------

let lastNodes: AdminNodesResponse | undefined;

When('{word} queries the admin nodes endpoint', async function (this: E2EWorld, humanName: string) {
  const human = this.getHuman(humanName);
  const device = human.devices[0] as BrowserDevice;
  assert.ok(device, `${humanName} has no device`);

  lastNodes = await device.client.adminNodes();
});

Then(
  'the response should include at least {int} node(s)',
  function (expectedMin: number) {
    assert.ok(lastNodes, 'No admin nodes response available');
    assert.ok(
      lastNodes.nodes.length >= expectedMin,
      `Expected at least ${expectedMin} node(s) but got ${lastNodes.nodes.length}`
    );
  }
);

Then(
  'at least one node should have status {string}',
  function (expectedStatus: string) {
    assert.ok(lastNodes, 'No admin nodes response available');
    const match = lastNodes.nodes.find(
      (n) => n.status.toLowerCase() === expectedStatus.toLowerCase()
    );
    assert.ok(
      match,
      `No node with status "${expectedStatus}". Statuses: ${lastNodes.nodes.map((n) => n.status).join(', ')}`
    );
  }
);

Then(
  'at least one node should report cpu cores greater than {int}',
  function (minCores: number) {
    assert.ok(lastNodes, 'No admin nodes response available');
    const match = lastNodes.nodes.find(
      (n) => n.inventory && n.inventory.cpuCores > minCores
    );
    assert.ok(
      match,
      `No node with cpu cores > ${minCores}. Values: ${lastNodes.nodes.map((n) => n.inventory?.cpuCores ?? 'N/A').join(', ')}`
    );
  }
);

Then(
  'at least one node should report memory greater than {int}',
  function (minMemory: number) {
    assert.ok(lastNodes, 'No admin nodes response available');
    const match = lastNodes.nodes.find(
      (n) => n.inventory && n.inventory.memoryGb > minMemory
    );
    assert.ok(
      match,
      `No node with memory > ${minMemory}. Values: ${lastNodes.nodes.map((n) => n.inventory?.memoryGb ?? 'N/A').join(', ')}`
    );
  }
);
