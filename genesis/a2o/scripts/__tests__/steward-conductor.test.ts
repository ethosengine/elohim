/* eslint-disable @typescript-eslint/no-floating-promises -- node:test owns test promises. */
import { strict as assert } from 'node:assert';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { it } from 'node:test';

import { connectConductor, loadSigningCredentials } from '../lib/steward-conductor.js';

import type { CellId } from '@holochain/client';

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
