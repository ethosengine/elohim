/* eslint-disable @typescript-eslint/no-floating-promises -- node:test owns test promises. */
import { strict as assert } from 'node:assert';
import { createHash, createPrivateKey, createPublicKey, verify } from 'node:crypto';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { it } from 'node:test';

import { AppWebsocket, encodeHashToBase64, setSigningCredentials } from '@holochain/client';

import {
  connectConductor,
  callZomeWithCredentials,
  conductorSocketOptions,
  closeConductorSocket,
  loadSigningCredentials,
} from '../lib/steward-conductor.js';

import type { AppClientTransport, AppInfo, CellId, SigningCredentials } from '@holochain/client';

it('finishes socket cleanup when already closed, rejected or missing a close event', async () => {
  let calls = 0;
  await closeConductorSocket({
    socket: { readyState: 3 },
    async close() {
      calls += 1;
      return new Promise(() => undefined);
    },
  });
  assert.equal(calls, 0);
  let normallyClosed = false;
  await closeConductorSocket({
    async close() {
      await Promise.resolve();
      normallyClosed = true;
    },
  });
  assert.equal(normallyClosed, true);
  await closeConductorSocket({
    async close() {
      await Promise.resolve();
      throw new Error('remote socket failure');
    },
  });
  const started = Date.now();
  await closeConductorSocket({ close: async () => new Promise(() => undefined) });
  assert.ok(Date.now() - started < 5000, 'missing close event must not stall cleanup');
});

function testSigningCredentials(seedHex: string, cell: CellId, capSecretByte: number) {
  const seed = Buffer.from(seedHex, 'hex');
  const privateKey = createPrivateKey({
    key: Buffer.concat([Buffer.from('302e020100300506032b657004220420', 'hex'), seed]),
    format: 'der',
    type: 'pkcs8',
  });
  const publicKey = new Uint8Array(
    createPublicKey(privateKey).export({ format: 'der', type: 'spki' }).subarray(-32)
  );
  const signingKey = new Uint8Array(
    Buffer.concat([
      Buffer.from([0x84, 0x20, 0x24]),
      Buffer.from(publicKey),
      Buffer.from(cell[1].slice(35)),
    ])
  );
  return {
    seed,
    credentials: {
      signingKey,
      keyPair: {
        publicKey,
        privateKey: new Uint8Array(Buffer.concat([seed, Buffer.from(publicKey)])),
        keyType: 'ed25519' as const,
      },
      capSecret: new Uint8Array(64).fill(capSecretByte),
    } satisfies SigningCredentials,
  };
}

function signedCallVerifies(request: unknown, seed: Buffer): boolean {
  const signed = (request as { value: { bytes: Uint8Array; signature: Uint8Array } }).value;
  const privateKey = createPrivateKey({
    key: Buffer.concat([Buffer.from('302e020100300506032b657004220420', 'hex'), seed]),
    format: 'der',
    type: 'pkcs8',
  });
  return verify(
    null,
    createHash('sha512').update(signed.bytes).digest(),
    createPublicKey(privateKey),
    signed.signature
  );
}

function fakeAppWebsocket(calls: unknown[]): AppWebsocket {
  const transport = {
    on: () => () => undefined,
    request(request: unknown) {
      calls.push(request);
      return Promise.resolve({ type: 'success', value: new Uint8Array([0xc0]) });
    },
  };
  // AppWebsocket's constructor is private in the public declaration but is the
  // real SDK implementation used after connect() obtains its transport/appInfo.
  const TestAppWebsocket = AppWebsocket as unknown as new (
    client: AppClientTransport,
    appInfo: AppInfo
  ) => AppWebsocket;
  return new TestAppWebsocket(
    transport as never,
    { installed_app_id: 'same-app', agent_pub_key: new Uint8Array(39).fill(2) } as AppInfo
  );
}

it('keeps same-cell concurrent zome calls bound to each connection signing profile', async () => {
  const cell: CellId = [new Uint8Array(39).fill(1), new Uint8Array(39).fill(2)];
  const signerA = testSigningCredentials(
    '9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60',
    cell,
    0x11
  );
  const signerB = testSigningCredentials(
    '4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb',
    cell,
    0x22
  );
  const callsA: unknown[] = [];
  const callsB: unknown[] = [];
  const appA = fakeAppWebsocket(callsA);
  const appB = fakeAppWebsocket(callsB);

  // Model two established profiles for the same cell. The latest global write
  // would make appA sign with B unless each connection restores its own profile.
  setSigningCredentials(cell, signerA.credentials);
  setSigningCredentials(cell, signerB.credentials);
  await Promise.all([
    callZomeWithCredentials(appA, cell, signerA.credentials, 'content_store', 'read_a', {}),
    callZomeWithCredentials(appB, cell, signerB.credentials, 'content_store', 'read_b', {}),
  ]);

  assert.equal(callsA.length, 1);
  assert.equal(callsB.length, 1);
  assert.equal(signedCallVerifies(callsA[0], signerA.seed), true);
  assert.equal(signedCallVerifies(callsA[0], signerB.seed), false);
  assert.equal(signedCallVerifies(callsB[0], signerB.seed), true);
  assert.equal(signedCallVerifies(callsB[0], signerA.seed), false);
});

it('sends a gateway session only to its issuing origin and refuses unsafe receipts without secrets', async () => {
  const dir = mkdtempSync(join(tmpdir(), 'gateway-session-'));
  const gatewayOrigin = 'https://doorway.example';
  const options = {
    adminWs: 'wss://doorway.example/hc/admin?conductor_id=conductor-0',
    appWs: 'wss://doorway.example/hc/app/4445?conductor_id=conductor-0',
    appId: 'elohim',
    role: 'lamad',
    gatewayAuth: { doorway: gatewayOrigin, authFile: join(dir, 'auth.json') },
  };
  const saved = {
    doorwayUrl: gatewayOrigin,
    token: 'private-fixture-token',
    expiresAt: Math.floor(Date.now() / 1000) + 3600,
  };
  const save = (receipt = saved) =>
    writeFileSync(options.gatewayAuth.authFile, JSON.stringify(receipt), { mode: 0o600 });
  try {
    save();
    for (const endpoint of [options.adminWs, options.appWs]) {
      assert.deepEqual(await conductorSocketOptions(options, endpoint), {
        origin: gatewayOrigin,
        headers: { Authorization: 'Bearer private-fixture-token' },
      });
    }
    for (const endpoint of [
      'wss://other.example/hc/admin',
      'ws://doorway.example/hc/admin',
      'wss://user:password@doorway.example/hc/admin',
      `${options.adminWs}&token=private-url-token`,
    ]) {
      await assert.rejects(conductorSocketOptions(options, endpoint), {
        message: 'gateway credential origin or receipt is invalid',
      });
    }
    for (const receipt of [
      { ...saved, doorwayUrl: 'https://other.example' },
      { ...saved, expiresAt: 0 },
      { ...saved, token: 'private-token\r\ninjected-header' },
    ]) {
      save(receipt);
      await assert.rejects(conductorSocketOptions(options, options.adminWs), {
        message: 'gateway credential origin or receipt is invalid',
      });
    }
    assert.deepEqual(
      await conductorSocketOptions({ ...options, gatewayAuth: undefined }, 'ws://localhost:4444'),
      { origin: 'elohim' }
    );
  } finally {
    rmSync(dir, { recursive: true });
  }
});

it('reuses per-cell credentials and refuses a different cell or corrupt key without exposing secrets', async () => {
  const dir = mkdtempSync(join(tmpdir(), 'steward-credentials-'));
  // RFC 8032 test vector 1: public fixture, never an operator credential.
  const publicKey = Buffer.from(
    'd75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a',
    'hex'
  );
  const signingKey = new Uint8Array(
    Buffer.concat([Buffer.from('842024', 'hex'), publicKey, Buffer.alloc(4)])
  );
  const cell: CellId = [new Uint8Array(39).fill(1), new Uint8Array(39).fill(2)];
  const name = cell.map(hash => Buffer.from(hash).toString('hex')).join('-');
  const seed = '9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60';
  const stored = {
    keypair: seed,
    signingAgentKey: Buffer.from(signingKey).toString('hex'),
    capSecret: '03'.repeat(64),
  };
  const save = () =>
    writeFileSync(join(dir, `${name}.json`), JSON.stringify(stored), { mode: 0o600 });
  try {
    save();
    const loaded = await loadSigningCredentials(dir, cell);
    assert.deepEqual(loaded.keyPair.publicKey, new Uint8Array(publicKey));
    assert.deepEqual(loaded.signingKey, signingKey);
    assert.equal(loaded.capSecret.length, 64);
    await assert.rejects(
      loadSigningCredentials(dir, [cell[0], new Uint8Array(39).fill(4)]),
      /explicit enrollment required/
    );
    stored.signingAgentKey = '00'.repeat(39);
    save();
    await assert.rejects(loadSigningCredentials(dir, cell), /public key does not match/);
    stored.keypair = 'secret-invalid-seed';
    save();
    await assert.rejects(loadSigningCredentials(dir, cell), error => {
      assert.equal((error as Error).message, 'invalid signing credential field: keypair');
      return true;
    });
  } finally {
    rmSync(dir, { recursive: true });
  }
});

it('refuses absent credentials before contacting the admin socket', async () => {
  const previous = process.env.STEWARD_SIGNING_CREDENTIALS_DIR;
  delete process.env.STEWARD_SIGNING_CREDENTIALS_DIR;
  try {
    await assert.rejects(
      connectConductor({
        adminWs: 'not-a-url',
        appWs: 'not-a-url',
        appId: 'elohim',
        role: 'lamad',
      }),
      /existing signing credentials required/
    );
  } finally {
    if (previous !== undefined) process.env.STEWARD_SIGNING_CREDENTIALS_DIR = previous;
  }
});

it('refuses expired or mismatched hosted receipts before sockets without exposing tokens', async () => {
  const dir = mkdtempSync(join(tmpdir(), 'hosted-receipt-'));
  const agent = encodeHashToBase64(new Uint8Array(39).fill(2));
  const dna = encodeHashToBase64(new Uint8Array(39).fill(1));
  const receipt = {
    doorway: 'not-a-url',
    appToken: 'private-app-token',
    appPort: 1,
    agentPubKey: agent,
    installedAppId: 'operator',
    cellIds: { lamad: [dna, agent] },
    token: 'private-jwt',
    expiresAt: Date.now() + 60000,
  };
  const options = {
    adminWs: 'not-a-url',
    appWs: 'not-a-url',
    appId: 'operator',
    role: 'lamad',
    expectedAgent: agent,
    expectedDna: dna,
    signingCredentialsDir: dir,
    hosted: {
      doorway: receipt.doorway,
      receipt: join(dir, 'receipt.json'),
      authFile: '',
      signingFile: '',
    },
  };
  try {
    for (const patch of [
      { expiresAt: 0 },
      { installedAppId: 'wrong' },
      { agentPubKey: 'wrong' },
      { cellIds: { lamad: [encodeHashToBase64(new Uint8Array(39).fill(3)), agent] } },
    ]) {
      writeFileSync(options.hosted.receipt, JSON.stringify({ ...receipt, ...patch }), {
        mode: 0o600,
      });
      await assert.rejects(connectConductor(options), error => {
        const message = (error as Error).message;
        assert.match(message, /expired|mismatch/);
        assert.ok(!message.includes(receipt.token) && !message.includes(receipt.appToken));
        assert.ok(!message.includes(receipt.doorway));
        return true;
      });
    }
  } finally {
    rmSync(dir, { recursive: true });
  }
});
