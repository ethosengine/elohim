/**
 * Conductor pool visibility step definitions — admin views of the conductor pool.
 *
 * Matthew (admin) inspects conductors: list pool, view agents per conductor,
 * look up which conductor hosts a specific user.
 */

import { strict as assert } from 'node:assert';

import { When, Then } from '@cucumber/cucumber';

import { BrowserDevice } from '../src/framework/devices/browser-device.js';
import { E2EWorld } from '../src/framework/world.js';

// ---------------------------------------------------------------------------
// Types (mirrors doorway Rust responses from admin_conductors.rs)
// ---------------------------------------------------------------------------

interface ConductorSummary {
  conductorId: string;
  conductorUrl: string;
  adminUrl: string;
  capacityUsed: number;
  capacityMax: number;
  capacityAvailable: number;
  agentCount: number;
}

interface ConductorsResponse {
  total: number;
  totalAgents: number;
  totalCapacity: number;
  conductors: ConductorSummary[];
}

interface AgentSummary {
  agentPubKey: string;
  appId: string;
  assignedAt: string;
}

interface ConductorAgentsResponse {
  conductorId: string;
  total: number;
  agents: AgentSummary[];
}

interface AgentConductorResponse {
  agentPubKey: string;
  conductorId: string;
  conductorUrl: string;
  appId: string;
  assignedAt: string;
}

// ---------------------------------------------------------------------------
// Admin HTTP helper (reuse pattern from other admin steps)
// ---------------------------------------------------------------------------

async function adminGet<T>(device: BrowserDevice, path: string): Promise<T> {
  return device.client['get'](path);
}

// ---------------------------------------------------------------------------
// List conductors
// ---------------------------------------------------------------------------

let lastConductorsResponse: ConductorsResponse | undefined;

When('{word} queries the conductor pool', async function (this: E2EWorld, humanName: string) {
  const human = this.getHuman(humanName);
  const device = human.devices[0] as BrowserDevice;
  assert.ok(device, `${humanName} has no device`);

  lastConductorsResponse = await adminGet<ConductorsResponse>(device, '/admin/conductors');
});

Then('the conductor pool response should include a total count', function () {
  assert.ok(lastConductorsResponse, 'No conductors response available');
  assert.ok(
    typeof lastConductorsResponse.total === 'number',
    `Expected total to be a number, got ${typeof lastConductorsResponse.total}`
  );
});

Then('the conductor pool should include total capacity metrics', function () {
  assert.ok(lastConductorsResponse, 'No conductors response available');
  assert.ok(typeof lastConductorsResponse.totalCapacity === 'number', 'Missing totalCapacity');
  assert.ok(typeof lastConductorsResponse.totalAgents === 'number', 'Missing totalAgents');
});

// ---------------------------------------------------------------------------
// View agents on a conductor
// ---------------------------------------------------------------------------

let lastConductorAgentsResponse: ConductorAgentsResponse | undefined;

When(
  '{word} views agents on the first conductor',
  async function (this: E2EWorld, humanName: string) {
    assert.ok(lastConductorsResponse, 'Must query conductor pool first');
    assert.ok(lastConductorsResponse.conductors.length > 0, 'No conductors in pool to inspect');

    const human = this.getHuman(humanName);
    const device = human.devices[0] as BrowserDevice;
    const firstConductorId = lastConductorsResponse.conductors[0].conductorId;

    lastConductorAgentsResponse = await adminGet<ConductorAgentsResponse>(
      device,
      `/admin/conductors/${encodeURIComponent(firstConductorId)}/agents`
    );
  }
);

Then('the conductor agents response should include the conductor ID', function () {
  assert.ok(lastConductorAgentsResponse, 'No conductor agents response available');
  assert.ok(lastConductorAgentsResponse.conductorId, 'Missing conductorId in agents response');
});

Then('the agents list should be an array', function () {
  assert.ok(lastConductorAgentsResponse, 'No conductor agents response available');
  assert.ok(Array.isArray(lastConductorAgentsResponse.agents), 'agents should be an array');
});

// ---------------------------------------------------------------------------
// Agent → conductor lookup
// ---------------------------------------------------------------------------

let lastAgentConductorResponse: AgentConductorResponse | undefined;

When(
  '{word} looks up which conductor hosts {word}',
  async function (this: E2EWorld, adminName: string, targetName: string) {
    const admin = this.getHuman(adminName);
    const device = admin.devices[0] as BrowserDevice;
    assert.ok(device, `${adminName} has no device`);

    const target = this.getHuman(targetName);
    assert.ok(target.agentPubKey, `${targetName} has no agentPubKey`);

    lastAgentConductorResponse = await adminGet<AgentConductorResponse>(
      device,
      `/admin/agents/${encodeURIComponent(target.agentPubKey)}/conductor`
    );
  }
);

Then('the agent conductor lookup should return a conductor ID', function () {
  assert.ok(lastAgentConductorResponse, 'No agent conductor response available');
  assert.ok(lastAgentConductorResponse.conductorId, 'Missing conductorId in agent lookup response');
});

Then('the lookup should include the conductor URL', function () {
  assert.ok(lastAgentConductorResponse, 'No agent conductor response available');
  assert.ok(
    lastAgentConductorResponse.conductorUrl,
    'Missing conductorUrl in agent lookup response'
  );
});

// ---------------------------------------------------------------------------
// Permission enforcement (reuses step from user-management)
// ---------------------------------------------------------------------------

When(
  '{word} attempts to access the conductor pool',
  async function (this: E2EWorld, humanName: string) {
    const human = this.getHuman(humanName);
    const device = human.devices[0] as BrowserDevice;

    try {
      await adminGet<ConductorsResponse>(device, '/admin/conductors');
      this.contentIds.set('adminAccessDenied', 'false');
    } catch {
      this.contentIds.set('adminAccessDenied', 'true');
    }
  }
);
