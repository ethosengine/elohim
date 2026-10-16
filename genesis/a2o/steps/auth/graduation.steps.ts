/** Graduation stations. Share hosted-human state so existing cell/pipeline steps stay authoritative. */
import { strict as assert } from 'node:assert';
import { generateKeyPairSync, sign } from 'node:crypto';
import { After, Given, Then, When } from '@cucumber/cucumber';
import {
  AdminWebsocket,
  HoloHashType,
  encodeHashToBase64,
  hashFrom32AndType,
} from '@holochain/client';
import type { Page } from 'playwright';
import type { E2EWorld } from '../../src/framework/world.js';
import { loadHouseholdMeshFixture } from '../../src/framework/fixtures/household-mesh.js';
import { human, registerNewcomer } from '../ui/hosted-human.steps.js';

type Json = Record<string, unknown>;
type Reply = { status: number; body: Json };
interface State {
  reply?: Reply;
  challenge?: Json;
  requestedAt?: number;
  answer?: Json;
  freshKey?: string;
  nodeUrl?: string;
  nodeKey?: string;
}
const states = new WeakMap<E2EWorld, State>();
function state(world: E2EWorld): State {
  let value = states.get(world);
  if (!value) {
    value = {};
    states.set(world, value);
  }
  return value;
}
function text(value: unknown): string {
  assert.equal(typeof value, 'string');
  assert.ok(value);
  return value as string;
}
function page(world: E2EWorld): Page {
  const device = (world as unknown as { __portal?: { device?: { page: unknown } } }).__portal
    ?.device;
  assert.ok(device, 'Graduation requires the shared Playwright portal browser');
  return device.page as Page;
}
async function api(
  world: E2EWorld,
  path: string,
  body?: Json,
  token = human(world).token
): Promise<Reply> {
  const response = await fetch(`${human(world).doorwayUrl}${path}`, {
    method: body ? 'POST' : 'GET',
    headers: { authorization: `Bearer ${token}`, 'content-type': 'application/json' },
    ...(body ? { body: JSON.stringify(body) } : {}),
  });
  return { status: response.status, body: (await response.json()) as Json };
}
async function account(world: E2EWorld): Promise<Json> {
  const result = await api(world, '/auth/account');
  assert.equal(result.status, 200, JSON.stringify(result.body));
  return result.body;
}
async function challenge(world: E2EWorld, password: string): Promise<void> {
  const s = state(world);
  s.requestedAt = Date.now();
  s.reply = await api(world, '/auth/node-key/challenge', { password });
  s.challenge = s.reply.status === 200 ? s.reply.body : undefined;
}
function answer(world: E2EWorld, otherSigner = false): Json {
  const s = state(world);
  assert.ok(s.challenge, 'No successful challenge has been obtained');
  const key = generateKeyPairSync('ed25519');
  const raw = key.publicKey.export({ format: 'der', type: 'spki' }).subarray(-32);
  s.freshKey = encodeHashToBase64(hashFrom32AndType(raw, HoloHashType.Agent));
  const signer = otherSigner ? generateKeyPairSync('ed25519').privateKey : key.privateKey;
  return {
    challengeId: text(s.challenge.challengeId),
    agentPubKey: s.freshKey,
    signature: sign(null, Buffer.from(text(s.challenge.challenge), 'utf8'), signer).toString(
      'base64'
    ),
  };
}
async function bind(world: E2EWorld, body: Json, token?: string): Promise<void> {
  const s = state(world);
  s.answer = body;
  s.reply = await api(world, '/auth/node-key/bind', body, token);
  if (s.reply.status === 200) human(world).token = text(s.reply.body.token);
}
Given(
  'a newcomer registered at doorway {string} as a hosted human with the display name {string}',
  async function (this: E2EWorld, doorway: string, name: string) {
    await registerNewcomer(this, name, doorway);
    assert.equal((await account(this)).agencyPhase, 'hosted');
  }
);
After({ tags: '@graduation' }, async function (this: E2EWorld) {
  for (const entry of this.scenarioCreatedHumans.splice(0)) await entry.close();
});
When(
  'the newcomer asks the doorway for a key-move challenge with a wrong password',
  async function (this: E2EWorld) {
    await challenge(this, `${human(this).human.credentials.password}-wrong`);
  }
);
When(
  'the newcomer asks the doorway for a key-move challenge with their password',
  async function (this: E2EWorld) {
    await challenge(this, human(this).human.credentials.password);
  }
);
Given('the newcomer holds a key-move challenge from the doorway', async function (this: E2EWorld) {
  await challenge(this, human(this).human.credentials.password);
  assert.equal(state(this).reply?.status, 200);
});
Then('the doorway refuses with {string}', function (this: E2EWorld, code: string) {
  const status: Record<string, number> = {
    WRONG_PASSWORD: 403,
    SIGNATURE_INVALID: 403,
    CHALLENGE_NOT_YOURS: 403,
    CHALLENGE_UNKNOWN: 404,
  };
  assert.ok(status[code], `No status assertion declared for ${code}`);
  assert.equal(state(this).reply?.status, status[code]);
  assert.equal(state(this).reply?.body.code, code);
});
Then(
  'the doorway answers with a challenge naming this doorway, the account and its current key',
  async function (this: E2EWorld) {
    const s = state(this);
    const person = human(this);
    assert.equal(s.reply?.status, 200);
    assert.ok(s.challenge);
    const lines = text(s.challenge.challenge).split('\n');
    assert.equal(lines.length, 7);
    assert.equal(lines[0], 'elohim:doorway-account-bind:v1');
    // The issuer id is carried by the authenticated JWT, independently of the challenge.
    const claims = JSON.parse(
      Buffer.from(person.token.split('.')[1], 'base64url').toString()
    ) as Json;
    const audience = text(
      claims.doorway_id ?? claims.doorwayId ?? new URL(person.doorwayUrl).hostname
    );
    assert.deepEqual(lines.slice(1), [
      `audience=${audience}`,
      `identifier=${person.human.credentials.identifier}`,
      `human=${person.human.humanId}`,
      `from=${person.agentPubKey}`,
      `nonce=${text(s.challenge.challengeId)}`,
      `expires=${text(s.challenge.expiresAt)}`,
    ]);
    assert.match(text(s.challenge.challengeId), /^[a-f0-9]{64}$/);
  }
);
Then('the challenge expires within five minutes', function (this: E2EWorld) {
  const s = state(this);
  assert.ok(s.challenge && s.requestedAt);
  const expiry = Date.parse(text(s.challenge.expiresAt));
  assert.ok(
    expiry > Date.now() && expiry <= s.requestedAt + 300_000,
    'Challenge must expire within five minutes of request'
  );
});
Then(
  "the doorway still names the key it made as the account's key",
  async function (this: E2EWorld) {
    assert.equal((await account(this)).agentPubKey, human(this).agentPubKey);
  }
);
When(
  'the newcomer answers it naming one fresh key but signed by another',
  async function (this: E2EWorld) {
    await bind(this, answer(this, true));
  }
);
When(
  'the newcomer answers it with a fresh key they hold, signed by that key',
  async function (this: E2EWorld) {
    await bind(this, answer(this));
  }
);
Then("the doorway names that fresh key as the account's key", async function (this: E2EWorld) {
  assert.equal(state(this).reply?.status, 200);
  assert.ok(state(this).freshKey);
  assert.equal((await account(this)).agentPubKey, state(this).freshKey);
});
When('the same answer is sent to the doorway again', async function (this: E2EWorld) {
  const body = state(this).answer;
  assert.ok(body);
  await bind(this, body);
});
When(
  "the doorway's operator answers that challenge signed in as themselves, with a key they hold",
  async function (this: E2EWorld) {
    const operator = await this.getAdminClient(human(this).doorwayUrl);
    const token = operator.session?.token;
    assert.ok(token);
    assert.notEqual(token, human(this).token);
    await bind(this, answer(this), token);
  }
);

// join-peer starts a real conductor + storage, but exposes no symmetric removal verb.
// Do not mutate the shared household until a cleanup fixture can restore its roster and processes.
Given("the newcomer's own node has joined the household", function () {
  throw new Error(
    'UNBOUND: scenario-owned just mesh join-peer fixture with teardown of the fresh conductor, storage and roster is missing'
  );
});
Given(/^the newcomer is signed in to their node(?:'s portal in the browser)?$/, function () {
  throw new Error(
    'UNBOUND: fresh-node account enrollment and browser device-key-bound session fixture (session-key prover / DPoP) is missing'
  );
});
When(
  "the challenge is sent to the newcomer's node with a session copied to another browser, which has no device key",
  function () {
    throw new Error(
      'UNBOUND: node session fixture and second browser cookie-copy without its device key are missing'
    );
  }
);
When('the newcomer asks their node to sign the text {string}', function (_text: string) {
  throw new Error(
    'UNBOUND: authenticated node signing client with the browser session-key DPoP prover is missing'
  );
});
When(
  "the challenge is sent to the newcomer's node with no node session",
  async function (this: E2EWorld) {
    const s = state(this);
    assert.ok(s.nodeUrl && s.challenge, 'Fresh node and challenge required');
    const result = await fetch(`${s.nodeUrl}/api/v1/identity/account-bind/sign`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ challenge: s.challenge.challenge }),
    });
    s.reply = { status: result.status, body: (await result.json()) as Json };
  }
);
Then('the node refuses with {string}', function (this: E2EWorld, code: string) {
  const codes: Record<string, number> = {
    NO_SESSION: 401,
    SESSION_NOT_BOUND: 403,
    NOT_A_BIND_CHALLENGE: 400,
  };
  assert.ok(codes[code]);
  assert.equal(state(this).reply?.status, codes[code]);
  assert.equal(state(this).reply?.body.code, code);
});
Given(
  'the newcomer is signed in to the doorway portal in the same browser',
  async function (this: E2EWorld) {
    await page(this).goto(`${human(this).doorwayUrl}/threshold/account`);
    await page(this).getByTestId('account-identifier').waitFor();
    assert.equal(
      (await page(this).getByTestId('account-identifier').innerText()).trim(),
      human(this).human.credentials.identifier
    );
  }
);
When(/^the newcomer opens their account settings(?: again)?$/, async function (this: E2EWorld) {
  await page(this).goto(`${human(this).doorwayUrl}/imagodei/profile`);
  await page(this).getByRole('tab', { name: 'Network' }).click();
  await page(this).locator('.step.current').waitFor();
});
Then(
  "the account settings offer to move the doorway account onto the newcomer's node",
  async function (this: E2EWorld) {
    const link = page(this).getByTestId('move-to-node-entry');
    await link.waitFor();
    assert.equal(
      await link.getAttribute('href'),
      `${human(this).doorwayUrl}/threshold/account#move-to-node`
    );
  }
);
When('the newcomer chooses to move the account onto their node', async function (this: E2EWorld) {
  await page(this).getByTestId('move-to-node-entry').click();
});
Then(
  "the doorway portal's account page asks for their node's address and their doorway password",
  async function (this: E2EWorld) {
    for (const id of ['move-to-node-address', 'move-to-node-password', 'move-to-node-submit'])
      await page(this).getByTestId(id).waitFor();
  }
);
When(
  "the newcomer gives their node's address and their doorway password",
  async function (this: E2EWorld) {
    await page(this)
      .getByTestId('move-to-node-address')
      .fill(text(state(this).nodeUrl));
    await page(this)
      .getByTestId('move-to-node-password')
      .fill(human(this).human.credentials.password);
    await page(this).getByTestId('move-to-node-submit').click();
  }
);
Then(
  "the browser is sent to the newcomer's node portal, carrying the doorway's challenge",
  async function (this: E2EWorld) {
    const p = page(this);
    await p.getByTestId('account-bind-view').waitFor();
    const url = new URL(p.url());
    assert.equal(url.origin, text(state(this).nodeUrl));
    const challengeText = Buffer.from(
      text(url.searchParams.get('challenge')),
      'base64url'
    ).toString('utf8');
    assert.ok(challengeText.includes(`\nidentifier=${human(this).human.credentials.identifier}\n`));
    assert.ok(challengeText.includes(`\nfrom=${human(this).agentPubKey}\n`));
  }
);
Then(
  'the node portal shows which doorway and which account are asking, and asks the newcomer to approve',
  async function (this: E2EWorld) {
    const p = page(this);
    const lines = Buffer.from(text(new URL(p.url()).searchParams.get('challenge')), 'base64url')
      .toString('utf8')
      .split('\n');
    assert.equal(
      (await p.getByTestId('account-bind-audience').innerText()).trim(),
      lines[1].slice('audience='.length)
    );
    assert.equal(
      (await p.getByTestId('account-bind-identifier').innerText()).trim(),
      human(this).human.credentials.identifier
    );
    assert.ok(await p.getByTestId('account-bind-approve').isEnabled());
  }
);
When('the newcomer approves on the node portal', async function (this: E2EWorld) {
  const p = page(this);
  const response = p.waitForResponse(
    r => r.url().endsWith('/auth/node-key/bind') && r.request().method() === 'POST'
  );
  await p.getByTestId('account-bind-approve').click();
  const result = await response;
  state(this).answer = result.request().postDataJSON() as Json;
  state(this).reply = { status: result.status(), body: (await result.json()) as Json };
  if (result.ok()) human(this).token = text(state(this).reply?.body.token);
});
Then(
  "the browser is sent back to the doorway portal's account page, carrying the node's signature",
  function (this: E2EWorld) {
    const url = new URL(page(this).url());
    assert.equal(url.origin, human(this).doorwayUrl);
    assert.equal(url.pathname, '/threshold/account');
    const body = state(this).answer;
    assert.ok(body);
    assert.equal(Buffer.from(text(body.signature), 'base64').length, 64);
    assert.equal(body.agentPubKey, text(state(this).nodeKey));
  }
);
Then(
  "the doorway portal's account page says the account now answers to the newcomer's node's key",
  async function (this: E2EWorld) {
    assert.equal(state(this).reply?.status, 200);
    await page(this).getByTestId('move-to-node-result').waitFor();
    assert.ok(
      (await page(this).getByTestId('move-to-node-result').innerText()).includes(
        text(state(this).nodeKey).slice(0, 12)
      )
    );
  }
);
Then(
  "asked directly, the doorway names the newcomer's node's key as the account's key",
  async function (this: E2EWorld) {
    assert.equal((await account(this)).agentPubKey, text(state(this).nodeKey));
  }
);
Then(
  "asked directly, the doorway keeps the key the doorway made as the account's previous key",
  async function (this: E2EWorld) {
    assert.equal((await account(this)).previousAgentPubKey, human(this).agentPubKey);
  }
);
async function noCells(world: E2EWorld): Promise<void> {
  const admin = await world.getAdminClient(human(world).doorwayUrl);
  const conductors = (await admin.adminConductors()).conductors;
  assert.ok(conductors.length, 'Pool must be observable');
  for (const c of conductors) {
    const peer = Object.values(loadHouseholdMeshFixture().storagePeers ?? {}).find(
      p => p.conductorAppUrl && new URL(p.conductorAppUrl).port === new URL(c.conductorUrl).port
    );
    assert.ok(
      peer?.conductorAdminPort,
      `No household admin endpoint for pool conductor ${c.conductorId}`
    );
    const conductor = await AdminWebsocket.connect({
      url: new URL(`ws://127.0.0.1:${peer.conductorAdminPort}`),
      wsClientOptions: { origin: 'elohim' },
    });
    try {
      const apps = await conductor.listApps({});
      for (const app of apps) {
        const key = encodeHashToBase64(app.agent_pub_key);
        assert.notEqual(
          key,
          human(world).agentPubKey,
          `Old hosted app still installed: ${app.installed_app_id}`
        );
        assert.notEqual(
          key,
          state(world).nodeKey,
          `Login provisioned a new hosted app: ${app.installed_app_id}`
        );
      }
    } finally {
      await conductor.client.close();
    }
  }
}
Then(
  "the pool conductors no longer hold the newcomer's old hosted cell",
  async function (this: E2EWorld) {
    await noCells(this);
  }
);
When('the newcomer signs in at the doorway again', async function (this: E2EWorld) {
  const person = human(this);
  const result = await api(this, '/auth/login', {
    identifier: person.human.credentials.identifier,
    password: person.human.credentials.password,
  });
  assert.equal(result.status, 200);
  person.token = text(result.body.token);
  assert.equal(result.body.agentPubKey, text(state(this).nodeKey));
});
Then('no pool conductor starts a cell for them', async function (this: E2EWorld) {
  await noCells(this);
});
for (const step of [
  "the newcomer has moved their doorway account onto their node's key",
  "the newcomer's identity on the network records a key rotation from the key the doorway made to the node's key",
  "what the old key wrote is reachable from the newcomer's identity",
])
  Given(step, function () {
    throw new Error(
      'UNBOUND: witnessed network key rotation and old-key content linkage are not implemented (@wip)'
    );
  });
