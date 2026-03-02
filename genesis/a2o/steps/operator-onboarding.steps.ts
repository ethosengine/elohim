/**
 * Operator onboarding step definitions — bootstrap registration,
 * federation peer management, and agency pipeline funnel.
 *
 * Matthew bootstraps the alpha doorway with the admin key,
 * configures federation peers, and monitors the user lifecycle pipeline.
 */

import { strict as assert } from 'node:assert';
import { randomUUID } from 'node:crypto';

import { When, Then } from '@cucumber/cucumber';

import { BrowserDevice } from '../src/framework/devices/browser-device.js';
import { Human } from '../src/framework/human.js';
import { E2EWorld } from '../src/framework/world.js';

// ---------------------------------------------------------------------------
// Types (mirrors doorway Rust responses)
// ---------------------------------------------------------------------------

interface FederationPeer {
  url: string;
  reachable: boolean;
  doorwayId: string | null;
  region: string | null;
  capabilities: string[];
}

interface FederationPeersResponse {
  peers: FederationPeer[];
  total: number;
  selfId: string | null;
}

interface MutationResponse {
  success: boolean;
  message: string;
}

interface PipelineResponse {
  registeredTotal: number;
  registeredActive: number;
  hostedTotal: number;
  graduatingCount: number;
  stewardCount: number;
}

// ---------------------------------------------------------------------------
// Admin HTTP helpers
// ---------------------------------------------------------------------------

async function adminGet<T>(device: BrowserDevice, path: string): Promise<T> {
  return device.client['get'](path);
}

async function adminPost<T>(device: BrowserDevice, path: string, body: unknown): Promise<T> {
  return device.client['post'](path, body);
}

async function adminDelete<T>(device: BrowserDevice, path: string, body: unknown): Promise<T> {
  const { request } = await import('undici');
  const { statusCode, body: resBody } = await request(`${device.client.url}${path}`, {
    method: 'DELETE',
    headers: {
      'content-type': 'application/json',
      authorization: `Bearer ${device.token}`,
    },
    body: JSON.stringify(body),
  });
  const text = await resBody.text();
  if (statusCode < 200 || statusCode >= 300) {
    throw new Error(`DELETE ${path} returned ${statusCode}: ${text}`);
  }
  return JSON.parse(text) as T;
}

// ---------------------------------------------------------------------------
// Bootstrap registration (genesis moment)
// ---------------------------------------------------------------------------

When(
  'a new operator {string} registers on doorway {string} with the bootstrap key',
  async function (this: E2EWorld, humanName: string, doorwayId: string) {
    const doorway = this.getDoorway(doorwayId);
    const runId = randomUUID().slice(0, 8);
    const creds = {
      identifier: `e2e-${humanName.toLowerCase()}-${runId}@test.elohim.host`,
      password: `E2ePass!${runId}`,
      displayName: `${humanName} (E2E ${runId})`,
    };

    const human = new Human(humanName, creds);
    const device = new BrowserDevice(`${humanName}-browser`, doorway.url);
    human.addDevice(device);

    // Register with the bootstrap key from environment
    const bootstrapKey = process.env.E2E_ADMIN_BOOTSTRAP_KEY ?? 'test-admin-key';
    const auth = await device.register({
      identifier: creds.identifier,
      password: creds.password,
      displayName: creds.displayName,
      adminBootstrapKey: bootstrapKey,
    });

    human.agentPubKey = auth.agentPubKey;
    human.humanId = auth.humanId;
    human.setToken(doorwayId, auth.token);

    this.addHuman(humanName, human);
    this.contentIds.set('lastAuthResponse', JSON.stringify(auth));
  }
);

Then(
  '{word} should have admin permission level',
  async function (this: E2EWorld, humanName: string) {
    const human = this.getHuman(humanName);
    const device = human.devices[0] as BrowserDevice;
    assert.ok(device, `${humanName} has no device`);

    const me = await device.client.me();
    assert.strictEqual(
      me.permissionLevel,
      'Admin',
      `Expected Admin permission but got ${me.permissionLevel}`
    );
  }
);

Then(
  '{word} should not have admin permission level',
  async function (this: E2EWorld, humanName: string) {
    const human = this.getHuman(humanName);
    const device = human.devices[0] as BrowserDevice;
    assert.ok(device, `${humanName} has no device`);

    const me = await device.client.me();
    assert.notStrictEqual(
      me.permissionLevel,
      'Admin',
      `Expected non-Admin permission but got Admin`
    );
  }
);

// ---------------------------------------------------------------------------
// Federation peer management
// ---------------------------------------------------------------------------

let lastFederationPeers: FederationPeersResponse | undefined;
let lastPeerMutation: MutationResponse | undefined;

When('{word} lists federation peers', async function (this: E2EWorld, humanName: string) {
  const human = this.getHuman(humanName);
  const device = human.devices[0] as BrowserDevice;
  assert.ok(device, `${humanName} has no device`);

  lastFederationPeers = await adminGet<FederationPeersResponse>(device, '/admin/federation/peers');
});

Then('the federation peers response should succeed', function () {
  assert.ok(lastFederationPeers, 'No federation peers response available');
  assert.ok(
    typeof lastFederationPeers.total === 'number',
    'Missing total in federation peers response'
  );
});

When(
  '{word} adds a federation peer {string}',
  async function (this: E2EWorld, humanName: string, peerUrl: string) {
    const human = this.getHuman(humanName);
    const device = human.devices[0] as BrowserDevice;
    assert.ok(device, `${humanName} has no device`);

    lastPeerMutation = await adminPost<MutationResponse>(device, '/admin/federation/peers', {
      url: peerUrl,
    });
  }
);

Then('the peer mutation should succeed', function () {
  assert.ok(lastPeerMutation, 'No peer mutation response available');
  assert.ok(lastPeerMutation.success, `Peer mutation failed: ${lastPeerMutation.message}`);
});

Then('the federation peers list should include {string}', function (peerUrl: string) {
  assert.ok(lastFederationPeers, 'No federation peers response available');
  const match = lastFederationPeers.peers.find(p => p.url === peerUrl);
  assert.ok(match, `Peer ${peerUrl} not found in federation peers list`);
});

When(
  '{word} removes the federation peer {string}',
  async function (this: E2EWorld, humanName: string, peerUrl: string) {
    const human = this.getHuman(humanName);
    const device = human.devices[0] as BrowserDevice;
    assert.ok(device, `${humanName} has no device`);

    lastPeerMutation = await adminDelete<MutationResponse>(device, '/admin/federation/peers', {
      url: peerUrl,
    });
  }
);

When('{word} refreshes federation peers', async function (this: E2EWorld, humanName: string) {
  const human = this.getHuman(humanName);
  const device = human.devices[0] as BrowserDevice;
  assert.ok(device, `${humanName} has no device`);

  lastPeerMutation = await adminPost<MutationResponse>(
    device,
    '/admin/federation/peers/refresh',
    {}
  );
});

// ---------------------------------------------------------------------------
// Agency pipeline funnel
// ---------------------------------------------------------------------------

let lastPipeline: PipelineResponse | undefined;

When('{word} queries the agency pipeline', async function (this: E2EWorld, humanName: string) {
  const human = this.getHuman(humanName);
  const device = human.devices[0] as BrowserDevice;
  assert.ok(device, `${humanName} has no device`);

  lastPipeline = await adminGet<PipelineResponse>(device, '/admin/pipeline');
});

Then('the pipeline should show at least {int} registered users', function (minCount: number) {
  assert.ok(lastPipeline, 'No pipeline response available');
  assert.ok(
    lastPipeline.registeredTotal >= minCount,
    `Expected at least ${minCount} registered users but got ${lastPipeline.registeredTotal}`
  );
});

Then('the pipeline response should include all funnel stages', function () {
  assert.ok(lastPipeline, 'No pipeline response available');
  assert.ok(typeof lastPipeline.registeredTotal === 'number', 'Missing registeredTotal');
  assert.ok(typeof lastPipeline.registeredActive === 'number', 'Missing registeredActive');
  assert.ok(typeof lastPipeline.hostedTotal === 'number', 'Missing hostedTotal');
  assert.ok(typeof lastPipeline.graduatingCount === 'number', 'Missing graduatingCount');
  assert.ok(typeof lastPipeline.stewardCount === 'number', 'Missing stewardCount');
});
